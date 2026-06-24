### Delta: CPC 6128 PAL Bank Switching

The Amstrad CPC 6128 contains a second 64KB block of RAM (Bank 1), managed by a custom Programmed Array Logic (PAL) chip.

#### PAL Chip Interface Signals
```
              +-------\/-------+
    D7 AND D6 | 1           20 | VCC
           D0 | 2           19 | /CAS1 (Bank 1 Select)
       /RESET | 3           18 | /CAS0 (Bank 0 Select)
       RAMDIS | 4           17 | A15OUT
           D1 | 5           16 | A14OUT
           D2 | 6           15 | NCAS
         /CPU | 7           14 | A14
         A15  | 8           13 | /IOWR
           NC | 9           12 | NC
          GND | 10          11 | NC
              +----------------+
```

* **I/O Trigger Condition:** The PAL registers a bank-switching configuration write when the following conditions are simultaneously met on the bus:
  $$\text{Address } A15 = 0, \quad A14 = 1$$
  $$\text{Data } D7 = 1, \quad D6 = 1$$
  $$\text{Control } \overline{\text{IOWR}} = 0$$
* **Configuration Selection:** Bits `D2`, `D1`, and `D0` of the written byte determine the active memory configuration map.

#### Memory Configurations Table (Selections 0–7)
Memory is paged in 16KB sub-blocks (numbered $0..3$). Blocks with an asterisk (`*`) belong to the second 64KB bank (Bank 1).

| Range | Selection 0 | Selection 1 | Selection 2 | Selection 3 | Selection 4 | Selection 5 | Selection 6 | Selection 7 |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `&0000-&3FFF` | 0 | 0 | 0* | 0 | 0 | 0 | 0 | 0 |
| `&4000-&7FFF` | 1 | 1 | 1* | 3 | 0* | 1* | 2* | 3* |
| `&8000-&BFFF` | 2 | 2 | 2* | 2 | 2 | 2 | 2 | 2 |
| `&C000-&FFFF` | 3 | 3* | 3* | 3* | 3 | 3 | 3 | 3 |

#### Signal Truth Table

During any memory access, the PAL maps physical Z80 addresses ($A15, A14$) to bank-specific `/CAS0` (Bank 0), `/CAS1` (Bank 1) selects, and translated addresses $A15OUT, A14OUT$.

| Selection | $D[2:0]$ | $A15$ | $A14$ | $/CAS1$ | $/CAS0$ | $A15OUT$ | $A14OUT$ | Mapped Block |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **0** | `000` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 1 | 0 | 0 | 1 | 1 |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 1 | 0 | 1 | 1 | 3 |
| **1** | `001` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 1 | 0 | 0 | 1 | 1 |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 0 | 1 | 1 | 1 | 3* |
| **2** | `010` | 0 | 0 | 0 | 1 | 0 | 0 | 0* |
| | | 0 | 1 | 0 | 1 | 0 | 1 | 1* |
| | | 1 | 0 | 0 | 1 | 1 | 0 | 2* |
| | | 1 | 1 | 0 | 1 | 1 | 1 | 3* |
| **3** | `011` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 1 | 0 | 1 | 1 | 3 |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 0 | 1 | 1 | 1 | 3* |
| **4** | `100` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 0 | 1 | 0 | 0 | 0* |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 1 | 0 | 1 | 1 | 3 |
| **5** | `101` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 0 | 1 | 0 | 1 | 1* |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 1 | 0 | 1 | 1 | 3 |
| **6** | `110` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 0 | 1 | 1 | 0 | 2* |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 1 | 0 | 1 | 1 | 3 |
| **7** | `111` | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| | | 0 | 1 | 0 | 1 | 1 | 1 | 3* |
| | | 1 | 0 | 1 | 0 | 1 | 0 | 2 |
| | | 1 | 1 | 1 | 0 | 1 | 1 | 3 |

#### Functional Observations
1. `/CAS1` and `/CAS0` are mutually exclusive ($/\text{CAS1} = \neg/\text{CAS0}$), gated by physical timing signal `NCAS`.
2. Only ranges `&4000-&7FFF` (sub-block 1) and `&C000-&FFFF` (sub-block 3) are affected by PAL bank-switching selections, with the exception of Configuration 2 (which pages Bank 1 across the entire range).
3. Under Selections 4, 5, 6, 7: If the CPU addresses the range `&4000-&7FFF` ($A15=0, A14=1$), the PAL outputs $A15OUT = D1$ and $A14OUT = D0$, routing the access directly to the block designated by the selection register bits.
