# Memory Subsystem Overview

The Z80 processor has a 16-bit address space, allowing it to address 64 KB of memory directly. The CPC manages physical memory through a paging system controlled by the custom Gate Array chip.

## Memory Configuration Components
The physical memory of the Base Model (CPC 464) consists of:
1. **64 KB of Dynamic RAM:** Mapped from `&0000` to `&FFFF`.
2. **16 KB Lower ROM:** Contains the operating system, mapped to `&0000-&3FFF` when enabled.
3. **16 KB Upper ROM:** Contains BASIC (and other ROM expansions), mapped to `&C000-&FFFF` when enabled.

## Gate Array ROM Selection Registers
The Z80 selects and pages ROMs by writing to the Gate Array configuration port.
* **Lower ROM Control:** Can be enabled or disabled dynamically. When enabled, read operations in the `&0000-&3FFF` range access the ROM; write operations always target the underlying RAM.
* **Upper ROM Control:** Can be paged in or out of the `&C000-&FFFF` range. Read operations access the active upper ROM; write operations always target the underlying RAM.

## Chapter Directory
* [CPC 464 Base Memory Map](map_464.md)
* [Dynamic RAM Refresh Mechanics](refresh.md)
* [System Variables Reference](variables.md)
* [Delta: CPC 6128 PAL Bank Switching](delta_6128.md)