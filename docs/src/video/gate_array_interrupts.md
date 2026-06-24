### Gate Array Interrupt Generation

The Gate Array generates maskable interrupts without an external vector controller (typically operating in Z80 Interrupt Mode 1).

#### Interrupt Counter Circuitry
The Gate Array maintains a internal **6-bit counter** which increment on every HSYNC signal transition from the CRTC.
* **Trigger Value:** When the 6-bit counter reaches a value of **52**, the Gate Array clears the counter to 0 and asserts the Z80 `/INT` line low.
* **Assertion Duration:** `/INT` remains active until the Z80 acknowledges the interrupt.

#### Z80 Acknowledge Sense (`/IORQ` + `/M1`)
The Gate Array monitors the Z80 control bus for an Interrupt Acknowledge cycle (asserted by Z80 as `/IORQ = 0` and `/M1 = 0`).
* **Acknowledge Behavior:** 
  1. The interrupt request line (`/INT`) is cleared to high.
  2. The highest bit of the 6-bit counter (Bit 5) is cleared to `0`. 
  3. This limits the counter to a maximum range of 31, preventing a subsequent interrupt from occurring for at least **32 HSYNCs**.

#### Software Clear Command
+If the CPU performs a write to the Gate Array "Select screen mode and rom configuration" register (gated by command bits `7..6 = 10`) and sets **Bit 4 to 1**, the active interrupt request is immediately cleared and the 6-bit counter is reset to `0`.

#### VSYNC Synchronization
The Gate Array monitors the CRTC's VSYNC signal. After exactly **2 HSYNC transitions** have been registered following the rising edge of VSYNC, the Gate Array executes one of two operations:
* **Case A (Counter >= 32):** If Bit 5 of the counter is `1`, the counter is reset to `0`, and no interrupt is requested.
* **Case B (Counter < 32):** If Bit 5 of the counter is `0`, the counter is reset to `0`, and a new interrupt request is immediately generated.

This synchronization aligns subsequent interrupt steps to the vertical retrace phase.

---

### Delta: CPC+ Interrupts

CPC+ interrupts bypass standard Gate Array logic when the ASIC features are unlocked.

* **Trigger Control:** If the Programmable Raster Interrupt (PRI) register is set to a non-zero value, standard 52-line Gate Array interrupts are disabled.
* **Counter Operation:** The internal 6-bit counter continues to cycle as normal.
* **Acknowledge Interlocking:** When a PRI interrupt is acknowledged, the Gate Array's Bit 5 counter bit is cleared, ensuring a 32-line safety gap if standard interrupts are re-enabled.

---

### Delta: KC Compact Interrupts

Interrupts on the East German KC Compact clone do not use the Gate Array counting mechanism.

* **Controller Chip:** Uses a Z8536 CIO (Counter/Timer and I/O) chip, specifically Counter/Timer 3.
* **Source Signal:** Counter 3 decrement inputs are tied directly to CRTC HSYNC.
* **Triggering:** The CIO trigger is set by a circuit detecting 2 HSYNCs after VSYNC.
* **Acknowledge Difference:** The Z80 interrupt acknowledge cycle does **not** reset the count value or enforce a 32-line safety margin. Interrupts can occur closer than 32 lines.