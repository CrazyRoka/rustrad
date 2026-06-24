### Z80 Interrupt Acknowledge & Wait-State Timing

In the Amstrad CPC architecture, memory accesses and I/O operations are locked to a 1 μs (4 T-states) boundary. This has a direct effect on the cycle-by-cycle execution of Z80 maskable interrupts.

#### Standard Interrupt Acknowledge Latency
The Z80 processor natively forces a minimum of 2 internal Wait states (T-states) during an interrupt acknowledge cycle (`/IORQ` and `/M1` both asserted low by the CPU). On the CPC, the hardware design typically synchronizes this acknowledge phase to a 1 μs boundary, which can introduce up to 4 T-states of delay.

#### Interrupt Acknowledge Wait-State Removal
When an interrupt is acknowledged, the standard CPC-imposed synchronization delay is bypassed or removed if the Z80 is currently executing or has just completed certain instructions. When these specific instructions are running, the CPU is already aligned with the boundary, eliminating the extra wait state.

Your emulator must adjust its instruction cycle timing during interrupt assertion if any of the following instructions are active:

* **16-Bit Register Arithmetic:**
  * `INC ss` / `DEC ss` (where `ss` $\in$ {`HL`, `BC`, `DE`, `SP`})
  * `INC IX` / `DEC IX`
  * `INC IY` / `DEC IY`
* **Conditional Control Flow:**
  * `RET cc` (only when the condition `cc` is **false / not met**)
* **Stack & Pointer Manipulation:**
  * `EX (SP), HL`
  * `EX (SP), IX`
  * `EX (SP), IY`
  * `LD SP, HL`
  * `LD SP, IX`
  * `LD SP, IY`
* **Special Registers Access:**
  * `LD A, I` / `LD I, A`
  * `LD A, R` / `LD R, A`
* **Block Instructions:**
  * `LDI` / `LDIR` (applicable to both searching and looping states)
  * `LDD` / `LDDR` (applicable to both searching and looping states)
  * `CPIR` (only during active looping states)
  * `CPDR` (only during active looping states)