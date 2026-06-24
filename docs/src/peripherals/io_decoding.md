### 16-Bit I/O Port Address Decoding

Unlike standard systems that decode only the lower 8 bits of the Z80 I/O address space, the Amstrad CPC decodes port numbers using all **16 address lines (`A15`–`A0`)**. 

#### Hardware Register-to-Address Generation
During Z80 I/O instruction execution, address lines are driven as follows:
* **`IN r, (C)` / `OUT (C), r`:** The 16-bit port address is formed by the `BC` register pair (High byte `A15`–`A8` from `B`, Low byte `A7`–`A0` from `C`).
* **`IN A, (n)` / `OUT (n), A`:** The 16-bit port address is formed by the `A` register and the immediate operand `n` (High byte `A15`–`A8` from `A`, Low byte `A7`–`A0` from `n`).

#### Device Address Decoding Map
Internal hardware peripherals match specific bit patterns on the 16-bit address bus. 
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

