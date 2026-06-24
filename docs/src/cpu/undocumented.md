### Undocumented Z80 Opcodes

To accurately execute commercial software and custom firmware routines, the CPU emulation layer must support undocumented behaviors concerning the Z80 Index Registers (`IX` and `IY`) and the logical shift operations.

#### Index Register Partitioning (Half-Registers)
The Z80 index registers `IX` and `IY` can be split into high and low 8-bit registers. These are defined as:
* **HIX / IXH**: High byte of `IX`
* **LIX / IXL**: Low byte of `IX`
* **HIY / IYH**: High byte of `IY`
* **LIY / IYL**: Low byte of `IY`

Operations involving these 8-bit registers copy the execution patterns of their standard 8-bit counterparts (`H` and `L`), with prefix bytes `&DD` (for `IX` variants) or `&FD` (for `IY` variants).

##### Example Half-Register Opcodes:
* `LD HIX, n` (Opcode: `DD 26 n`, 3 Bytes, 11 T-states)
* `INC LIX` (Opcode: `DD 2C`, 2 Bytes, 8 T-states)
* `CP HIY` (Opcode: `FD BC`, 2 Bytes, 8 T-states)
* `XOR LIY` (Opcode: `FD AD`, 2 Bytes, 8 T-states)

#### Shift Left Logical (SLL)
The undocumented instruction `SLL` performs a left shift on the target register or memory location. It differs from `SLA` by shifting a `1` into bit 0 instead of a `0`.

* **Logical Behavior:** 
  $$\text{Bit } 0 \leftarrow 1$$
  $$\text{Bit } [n+1] \leftarrow \text{Bit } [n]$$
  $$\text{Carry} \leftarrow \text{Bit } 7$$
* **Flag Effects:**
  * **S (Sign):** Set if bit 7 of the result is set.
  * **Z (Zero):** Set if result is 0.
  * **P/V (Parity/Overflow):** Set if parity is even.
  * **C (Carry):** Receives the original bit 7 value.

##### SLL Opcode Reference Table:
| Instruction | Opcode | Bytes | T-states | S | Z | P/V | C |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **SLL A** | `CB 37` | 2 | 8 | 7 | z | p | r7 |
| **SLL B** | `CB 30` | 2 | 8 | 7 | z | p | r7 |
| **SLL C** | `CB 31` | 2 | 8 | 7 | z | p | r7 |
| **SLL D** | `CB 32` | 2 | 8 | 7 | z | p | r7 |
| **SLL E** | `CB 33` | 2 | 8 | 7 | z | p | r7 |
| **SLL H** | `CB 34` | 2 | 8 | 7 | z | p | r7 |
| **SLL L** | `CB 35` | 2 | 8 | 7 | z | p | r7 |
| **SLL (HL)** | `CB 36` | 2 | 15 | 7 | z | p | r7 |
| **SLL (IX+d)**| `DD CB d 36` | 4 | 23 | 7 | z | p | r7 |
| **SLL (IY+d)**| `FD CB d 36` | 4 | 23 | 7 | z | p | r7 |