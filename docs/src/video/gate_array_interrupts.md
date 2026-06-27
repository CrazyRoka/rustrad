### Gate Array Interrupt Generation

The Gate Array generates maskable interrupts without an external vector controller (typically operating in Z80 Interrupt Mode 1).

#### Interrupt Counter Circuitry
The Gate Array maintains an internal **6-bit counter** (commonly denoted as `R52`) which increments on the falling edge of every HSYNC signal transition received from the CRTC.
* **Trigger Value:** When the 6-bit counter reaches a value of **52**, the Gate Array immediately asserts the Z80 `/INT` line low.
* **Reset Conditions:** The `R52` counter is cleared to `0` under three distinct hardware events:
  1. **Overflow:** The counter naturally rolls over from `51` to `0` (triggering an interrupt request).
  2. **Software Clear:** The CPU writes to the Gate Array Mode and ROM Configuration register (RMR) with **Bit 4 set to 1**.
  3. **VSYNC Synchronization:** The counter is cleared at the end of the **2nd HSYNC** transition encountered after the rising edge of the CRTC's VSYNC signal.

#### Interrupt Acknowledge & Bit 5 Safety Margin
When the Z80 acknowledges a pending interrupt (`/IORQ = 0` and `/M1 = 0`), the Gate Array immediately clears the `/INT` line back to high. However, to prevent interrupts from occurring too close to one another, the Gate Array utilizes the highest bit of the counter (Bit 5):
* **Late-Acknowledge Correction:** If the CPU was executing a long instruction or had disabled interrupts (`DI`), the `R52` counter may have continued counting past 51. 
* **The Subtraction Rule:** When the interrupt is finally acknowledged:
  1. The highest bit of the counter (Bit 5) is cleared to `0`.
  2. If the counter had reached 32 or more (`R52 >= 32`), this bit clearance effectively subtracts 32 from the current value of the counter.
  3. This enforces a mandatory safety gap of at least **20 HSYNC lines** before another interrupt request can be generated.

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

#### HSYNC Delay on CRTC Type 3 & 4
On CPC+ and cost-down ASICs, the physical HSYNC signal received from the CRTC is delayed internally by **1 microsecond** (synchronized to the character display boundary). Consequently, with identical register programming, maskable interrupts on these machines trigger exactly **1 μs later** than on older CRTC Type 0, 1, or 2 configurations.


---

### Delta: KC Compact Interrupts

Interrupts on the East German KC Compact clone do not use the Gate Array counting mechanism.

* **Controller Chip:** Uses a Z8536 CIO (Counter/Timer and I/O) chip, specifically Counter/Timer 3.
* **Source Signal:** Counter 3 decrement inputs are tied directly to CRTC HSYNC.
* **Triggering:** The CIO trigger is set by a circuit detecting 2 HSYNCs after VSYNC.
* **Acknowledge Difference:** The Z80 interrupt acknowledge cycle does **not** reset the count value or enforce a 32-line safety margin. Interrupts can occur closer than 32 lines.