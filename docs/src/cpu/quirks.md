### Z80 CPC-Specific Quirks

Rather than detailing the generic Z80 instruction set (which should be implemented using standard Zilog Z80 datasheets and instruction references), this page documents undocumented behaviors and hardware interactions unique to the Amstrad CPC.

#### Index Register Splitting
The 16-bit index registers `IX` and `IY` can be accessed as independent 8-bit registers (referred to as `HIX`/`LIX` and `HIY`/`LIY`). These registers behave identically to the standard `H` and `L` registers but are prefixed by `&DD` or `&FD` opcodes. 

#### Shift Left Logical (SLL)
The undocumented `SLL` instruction is fully supported on the CPC CPU. It performs a standard left-shift operation but, unlike `SLA`, it shifts a `1` into bit 0 of the target register or memory address:
$$\text{Target} \leftarrow (\text{Target} \ll 1) \mid 1$$

#### Hardware-Imposed Delay (Wait States)
Every CPU instruction cycle is rounded up to a multiple of 4 T-states (1 μs) by the Gate Array's bus arbitration. This synchronization must be executed at the machine-cycle (`M`-cycle) level of your CPU emulation loop.

