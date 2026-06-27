### Z80 Interrupt Acknowledge & Wait-State Timing

In the Amstrad CPC architecture, memory accesses and I/O operations are locked to a 1 μs (4 T-states) boundary. This synchronization is achieved by the Gate Array manipulating the Z80 `/WAIT` (pin 24) and `/READY` lines.

#### Gate Array wait-state generation
The custom Gate Array (or equivalent integrated ASIC) is clocked at 16 MHz. It generates the system's 4 MHz CPU clock and regulates bus access to the shared RAM:
* **Wait-State Pattern:** The Gate Array continuously cycles a `/WAIT` signal that is active low for exactly 3 out of every 4 cycles, allowing the Z80 unhindered access on only the remaining cycle. 
* **Instruction Stretching:** Any CPU instruction that attempts a memory read (`/MREQ` + `/RD`), memory write (`/MREQ` + `/WR`), or I/O request (`/IORQ`) during an active `/WAIT` phase is forced to insert wait states (`Tw`) until the `/WAIT` line goes high. This effectively "stretches" and aligns all instructions to the nearest 4 T-state multiple.

#### Standard Interrupt Acknowledge Latency
The Z80 processor natively forces a minimum of 2 internal Wait states during an interrupt acknowledge cycle (when `/IORQ = 0` and `/M1 = 0` are asserted simultaneously). On the CPC, the Gate Array's synchronization delay can stretch this cycle further, introducing up to 4 T-states of jitter depending on the instruction alignment when the `/INT` pin is sampled.

#### Interrupt Acknowledge Wait-State Removal
When an interrupt is acknowledged, the standard CPC-imposed synchronization delay is bypassed or removed if the Z80 is executing or has just completed certain instructions. When these specific instructions are running, the CPU is already aligned with the boundary, eliminating the extra wait state.

Your emulator must adjust its instruction cycle timing during interrupt assertion if any of the following instructions are active:

* **16-Bit Register Arithmetic:**
  * `INC ss` / `DEC ss` (where `ss` is one of `HL`, `BC`, `DE`, `SP`)
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