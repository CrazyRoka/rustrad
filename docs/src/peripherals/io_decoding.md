### 16-Bit I/O Port Address Decoding

Unlike standard 8-bit Z80 systems, the Amstrad CPC decodes peripheral port numbers using all **16 address lines (`A15`–`A0`)**. 

#### Hardware Register-to-Address Generation
During Z80 I/O instruction execution, address lines are driven as follows:
* **`IN r, (C)` / `OUT (C), r`:** The 16-bit port address is formed by the `BC` register pair (High byte `A15`–`A8` from `B`, Low byte `A7`–`A0` from `C`).
* **`IN A, (n)` / `OUT (n), A`:** The 16-bit port address is formed by the `A` register and the immediate operand `n` (High byte `A15`–`A8` from `A`, Low byte `A7`–`A0` from `n`).

*Emulator Note:* Standard 8-bit I/O instructions like `OUT (n), A` or `IN A, (n)` are highly restricted in practice because the CPU mirrors the `A` register onto the upper address bus half (`A15`–`A8`). If the `A` register's value does not align with the target peripheral's active-low address mask, the write or read operation will address the wrong device or fail entirely.

#### The "One Low Bit" Selection Rule & Hardware Hazards
The Amstrad CPC hardware selects its main internal and expansion peripherals by checking if specific bits in the upper half of the address bus (`A15`–`A10`) are driven **low (0)**:

* `A15` low $\rightarrow$ Gate Array Select
* `A14` low $\rightarrow$ CRTC Select
* `A13` low $\rightarrow$ ROM Select
* `A12` low $\rightarrow$ Printer Port Select
* `A11` low $\rightarrow$ Intel 8255 PPI Select
* `A10` low $\rightarrow$ Expansion Peripherals Select

##### Bus Contention Warning
A robust emulator should monitor and flag situations where **more than one** of these six selection bits (`A15`–`A10`) are simultaneously driven low. On physical hardware:
* During **output** operations, sending data to a port with multiple low bits will trigger write operations on all matching devices at the same time.
* During **input** operations, multiple matching devices will attempt to drive the Z80 data bus simultaneously. This creates physical electrical conflicts (bus fighting), potentially causing data corruption or physical hardware damage.

#### Device Address Decoding Map
* `0` = Bit must be low.
* `1` = Bit must be high.
* `-` = Bit is ignored (don't care state).
* `r1, r0` = Bit used to select internal register offsets on the peripheral.

| Hardware Device | Recommended Port | Read/Write | A15 | A14 | A13 | A12 | A11 | A10 | A9 | A8 |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Gate Array** | `&7Fxx` | Write Only | 0 | 1 | - | - | - | - | - | - |
| **RAM Configuration** (MMR) | `&7Fxx` | Write Only | 0 | - | - | - | - | - | - | - |
| **CRTC (6845)** | `&BCxx - &BFxx`| Read/Write | - | 0 | - | - | - | - | $r_1$ | $r_0$ |
| **ROM Select** | `&DFxx` | Write Only | - | - | 0 | - | - | - | - | - |
| **Printer Port** | `&EFxx` | Write Only | - | - | - | 0 | - | - | - | - |
| **8255 PPI** | `&F4xx - &F7xx`| Read/Write | - | - | - | - | 0 | - | $r_1$ | $r_0$ |
| **Expansion Peripherals** | `&F8xx - &FBxx`| Read/Write | - | - | - | - | - | 0 | - | - |

#### Internal Registrations Decode

##### CRTC Registers (`r1`, `r0` on bits 9, 8)
* `&BCxx` (Bits: `00`): **CRTC Index Register** (Write-Only)
* `&BDxx` (Bits: `01`): **CRTC Data Out** (Write-Only)
* `&BExx` (Bits: `10`): **CRTC Status Register** (Read-Only)
* `&BFxx` (Bits: `11`): **CRTC Data In** (Read-Only)

##### PPI Registers (`r1`, `r0` on bits 9, 8)
* `&F4xx` (Bits: `00`): **PPI Port A Data** (Read/Write)
* `&F5xx` (Bits: `01`): **PPI Port B Data** (Read/Write)
* `&F6xx` (Bits: `10`): **PPI Port C Data** (Read/Write)
* `&F7xx` (Bits: `11`): **PPI Control Register** (Write-Only)

#### Expansion Port Sub-Decoding (`A10` is Low)
When `A10` is low, the expansion bus is active. The system decodes specific expansion sub-channels using address lines `A7`, `A6`, and `A5`:

* `A5` low $\rightarrow$ Communication Channel
* `A6` low $\rightarrow$ Reserved Function
* `A7` low $\rightarrow$ Disc Subsystem (FDC Interface)

##### Expansion General Reset
* **Port Address:** `&F8FF`
* **Function:** Serves as a master software-triggered reset for all connected expansion devices.

#### Safe User I/O Ranges
To avoid bus contention and collision with standard internal hardware or recognized expansion cards, custom user-designed peripherals must restrict their I/O addresses to the following blocks:
* `&F8E0` – `&F8FE`
* `&F9E0` – `&F9FF`
* `&FAE0` – `&FAFF`
* `&FBE0` – `&FBFF`
