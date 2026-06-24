# Memory Subsystem Overview

The Z80 processor possesses a 16-bit address space, allowing it to address 64 KB of memory directly. The CPC manages physical memory through a paging system controlled by the custom Gate Array chip.

## Memory Configuration Components
While the logical address space is restricted to 64 KB, the physical system contains a total of **96 KB** of integrated memory:
1. **64 KB of Dynamic RAM:** Mapped from `&0000` to `&FFFF`.
2. **32 KB of physical ROM:** Although implemented on the mainboard as a single 32 KB physical silicon component, internal hardware address line manipulation divides it into two distinct 16 KB logical blocks:
   * **16 KB Lower ROM:** Contains the operating system, mapped to `&0000-&3FFF` when enabled.
   * **16 KB Upper ROM:** Contains the BASIC interpreter (and other ROM expansions), mapped to `&C000-&FFFF` when enabled.

## Gate Array ROM Selection Registers
The physical switching between ROM and RAM states is driven directly by the Video Gate Array. The CPU configures these states by writing to port `&7Fxx` (Gate Array configuration port).

* **Lower ROM Control (Bit 2 of Port `&7Fxx`):**
  * `0` = Enabled. Read operations in the range `&0000-&3FFF` access the Lower ROM.
  * `1` = Disabled. Read operations in the range `&0000-&3FFF` access the underlying RAM.
* **Upper ROM Control (Bit 3 of Port `&7Fxx`):**
  * `0` = Enabled. Read operations in the range `&C000-&FFFF` access the currently active Upper ROM.
  * `1` = Disabled. Read operations in the range `&C000-&FFFF` access the underlying RAM.

*Note:* All memory **write** operations always bypass the ROM logic and target the underlying physical RAM, regardless of the active ROM enablement state.

## Bank-Switching Execution Strategy
To perform jumps and register state changes while simultaneously paged-in ROM banks are swapped, the Amstrad operating system relies on dedicated routines placed in the **central RAM** space (`&4000-&BFFF`). Because this middle 32 KB of RAM has no overlapping ROM banks, its contents are persistently accessible to the CPU across all ROM configuration states.

## Chapter Directory
* [CPC 464 Base Memory Map](map_464.md)
* [Dynamic RAM Refresh Mechanics](refresh.md)
* [System Variables Reference](variables.md)
* [Delta: CPC 6128 PAL Bank Switching](delta_6128.md)