### Delta: CPC 6128 PAL Bank Switching

The Amstrad CPC 6128 contains a second 64KB block of RAM (Bank 1), managed by a custom Programmed Array Logic (PAL) chip.

#### Hardware Overview (Base 464 vs 6128)

| Feature | CPC 464 (Base) | CPC 6128 |
|---------|----------------|----------|
| **CPU** | Z80A @ 4 MHz | Z80A @ 4 MHz |
| **Base RAM** | 64 KB | 64 KB (Bank 0) |
| **Extended RAM** | None | 64 KB (Bank 1) via PAL16L8 |
| **Total RAM** | 64 KB | 128 KB |
| **Physical ROM** | 32 KB | 48 KB |
| **Lower ROM** | 16 KB Firmware | 16 KB Firmware |
| **Upper ROM** | 16 KB BASIC (ROM 0) | 32 KB: BASIC (ROM 0) + AMSDOS (ROM 7) |
| **Gate Array** | 40007 (early) / 40008 / 40010 | 40010 |
| **Banking Controller** | None | PAL16L8 |
| **Mass Storage** | Built-in cassette deck | Built-in 3" Hitachi disc drive |
| **Cassette Port** | None (integrated deck) | 5-pin DIN external connector |
| **CP/M Support** | CP/M 2.2 (with DDI-1, 39K TPA) | CP/M Plus 3.1 (61K TPA) + CP/M 2.2 |
| **BASIC Version** | Locomotive BASIC 1.0 (Line Input bug) | Locomotive BASIC 1.1 (same as CPC 664) |

#### Gate Array Version

The CPC 6128 ships exclusively with the Amstrad **40010** Gate Array. The 40010 is pinout-compatible with the 40008 used in late CPC 664 models, and an improved version of the 40007 used in early CPC 464 models.

The 40010 exhibits a unique Mode 2 rasterization timing: in video mode 2 (640x200, 2 colors), the display starts exactly **1 Mode 2 pixel (0.0625 μs)** earlier than in modes 0, 1, and 3. This shifts the entire scanline 1 pixel to the left in Mode 2. The 40007, 40008, and the costdown ASIC (40226) are not affected by this quirk.

#### ROM Configuration Differences

Both 464 and 6128 use the same Gate Array RMR register (port `&7Fxx`, bits 7-6 = `10`) to page Lower and Upper ROMs. However, the physical ROM contents differ:

* **CPC 464**: 32 KB physical ROM = 16 KB Lower (Firmware) + 16 KB Upper (BASIC, ROM 0). The 464 has no built-in ROM 7.
* **CPC 6128**: 48 KB physical ROM = 16 KB Lower (Firmware) + 32 KB Upper. The Upper ROM space contains two logical ROMs:
  * **ROM 0**: Locomotive BASIC (same version as CPC 664, with Line Input bug fixed).
  * **ROM 7**: AMSDOS (the disc operating system, equivalent to the DDI-1 ROM on a 464+DDI-1 configuration).

ROM selection (which Upper ROM is mapped to `&C000-&FFFF`) is performed via a separate write to port `&DFxx` (ROM Select Register). This register is identical on both 464 and 6128, but the bare 464 only has ROM 0 physically populated. Selecting ROM 7 on a 464 without DDI-1 reads from empty socket space (typically returning `&FF`). The 464+DDI-1 expansion adds ROM 7 to the system, making its ROM layout equivalent to the 6128.

*Note:* The ROM Bank Number is not stored inside the CPC; peripherals must watch the bus for writes to `&DFxx` and match the bank number via a flip-flop (e.g., 74LS74). If no peripheral claims the bank, the internal BASIC ROM responds. On the CPC Plus, the ASIC handles `&DFxx` writes differently to support physical cartridge ROMs (0–31) and logical ROM board ROMs (0–127). See [16-Bit I/O Port Address Decoding](../peripherals/io_decoding.md#upper-rom-bank-number-selection-port-dfxx) for full hardware details.

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
  * Address: `A15 = 0`, `A14 = 1`
  * Data: `D7 = 1`, `D6 = 1`
  * Control: `/IOWR = 0`
* **Configuration Selection:** Bits `D2`, `D1`, and `D0` of the written byte determine the active memory configuration map.

#### Memory Configurations Table (Selections 0–7)
Memory is paged in 16KB sub-blocks (numbered `0..3`). Blocks with an asterisk (`*`) belong to the second 64KB bank (Bank 1).

| Range | Selection 0 | Selection 1 | Selection 2 | Selection 3 | Selection 4 | Selection 5 | Selection 6 | Selection 7 |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `&0000-&3FFF` | 0 | 0 | 0* | 0 | 0 | 0 | 0 | 0 |
| `&4000-&7FFF` | 1 | 1 | 1* | 3 | 0* | 1* | 2* | 3* |
| `&8000-&BFFF` | 2 | 2 | 2* | 2 | 2 | 2 | 2 | 2 |
| `&C000-&FFFF` | 3 | 3* | 3* | 3* | 3 | 3 | 3 | 3 |

#### Signal Truth Table

During any memory access, the PAL maps physical Z80 addresses (`A15`, `A14`) to bank-specific `/CAS0` (Bank 0), `/CAS1` (Bank 1) selects, and translated addresses `A15OUT`, `A14OUT`.

| Selection | `D[2:0]` | `A15` | `A14` | `/CAS1` | `/CAS0` | `A15OUT` | `A14OUT` | Mapped Block |
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
1. `/CAS1` and `/CAS0` are mutually exclusive (`/CAS1 = NOT /CAS0`), gated by physical timing signal `NCAS`.
2. Only ranges `&4000-&7FFF` (sub-block 1) and `&C000-&FFFF` (sub-block 3) are affected by PAL bank-switching selections, with the exception of Configuration 2 (which pages Bank 1 across the entire range).
3. Under Selections 4, 5, 6, 7: If the CPU addresses the range `&4000-&7FFF` (`A15=0, A14=1`), the PAL outputs `A15OUT = D1` and `A14OUT = D0`, routing the access directly to the block designated by the selection register bits.

#### PAL vs Gate Array I/O Decoding

The PAL16L8 and the Gate Array both respond to I/O writes at port `&7Fxx`, but they have **distinct decoding rules** and command code spaces:

| Decoding Aspect | Gate Array (all CPC models) | PAL (6128 only) |
|----------------|------------|-----------------|
| **Address decode** | `A15 = 0` AND `A14 = 1` | `A15 = 0` only |
| **I/O Read response** | Responds (returns high-impedance bus value) | Does not respond |
| **I/O Write response** | Responds | Responds |
| **Command bits 7,6 = `00`** | Processes PENR | Ignores |
| **Command bits 7,6 = `01`** | Processes INKR | Ignores |
| **Command bits 7,6 = `10`** | Processes RMR | Ignores |
| **Command bits 7,6 = `11`** | Ignores (undefined for GA) | Processes MMR |

Both chips examine the data byte written to port `&7Fxx` simultaneously. They have **mutually exclusive command codes**, so a single write never affects both chips at the same time.

**Emulator Implementation Note:**
* On **CPC 464** (no PAL): writes with bits 7,6 = `11` are silently ignored by the Gate Array. No memory configuration change occurs. No error is generated.
* On **CPC 6128** (with PAL): writes with bits 7,6 = `11` are intercepted by the PAL, which configures RAM banking. The Gate Array itself ignores these bits.
* On **all CPC models**: reading port `&7Fxx` returns an unpredictable value from the high-impedance data bus. The PAL never responds to reads.

**Detection Hazard:** Some software attempts to detect Plus/GX4000 hardware by reading port `&7F00` and inspecting the floating bus state. This is **unreliable** because the high-impedance value depends on capacitance, temperature, and time, and varies between individual machines.

#### Behavior on the CPC 464 (No Banking Support)

Since the CPC 464 has no PAL chip and no extended RAM, attempts to use 6128-style banking commands produce the following behaviors:

* **MMR Writes (bits 7,6 = `11` to port `&7Fxx`)**: Silently ignored by the Gate Array. The 464's memory map remains in the default linear 64 KB configuration (equivalent to 6128 Selection 0). No error is generated, no crash occurs.
* **ROM Selection (port `&DFxx`)**: Functional. The 464 supports selecting any ROM index 0–255 via port `&DFxx`, but only ROM 0 (BASIC) is physically present. Selecting ROM 7 without a DDI-1 expansion returns `&FF` bytes (empty ROM socket).
* **Firmware Call `&BD5B`**: On the 464 firmware, this call does not perform bank switching. Software relying on `&BD5B` for second-bank access requires a 6128 (or 664) or a 464 with a compatible external RAM expansion.
* **RSX Commands (`|BANK`, `|BANKOPEN`, `|BANKWRITE`, `|BANKREAD`, `|BANKTIND`, `|SCREENSWAP`, `|SCREENCOPY`)**: Not present in 464 firmware. These are 6128-specific RSX extensions supplied on the 6128 system disc.

**Software Compatibility Note:** Many programs written for the 464 use "illegal" firmware calls (direct memory addresses) rather than the standardized jumpblock vectors. Because the 6128 firmware is derived from the 664 (not the 464), some of these illegal call targets point to different code or no longer exist. Such software may fail on the 6128 despite both machines being CPC-compatible at the jumpblock level.

#### External RAM Expansions and 6128 Compatibility

External RAM expansions (Dk'tronics, Dobbertin, etc.) can add bank-switched RAM to the CPC 464 (and 664). Some are 6128-compatible; others use proprietary banking schemes.

* **6128-Compatible Expansions**: Embed a PAL16L8 (or equivalent logic) that responds to the same MMR command codes as the 6128's internal PAL. These allow 6128 banking software to run unmodified on a 464. The Dk'tronics RAM box connects to the floppy disc port on the CPC 464, or the expansion port on the CPC 664/6128.
* **Dk'tronics `|EMULATE` RSX**: The Dk'tronics RAM box includes an `|EMULATE` RSX command that emulates the 6128's bank switching technique, allowing CP/M software written for the 6128 (e.g., Masterfile, Tasword 128) to run on a 464.
* **Direct Access Without RSX**: For assembly language programmers, the Dk'tronics RAM box can be accessed directly via:
  ```text
  OUT &7FBB, 196 + (Bank AND 3) + (Bank AND 28) * 2
  ```
  Where `Bank` is the bank to be switched in.
* **Limitations**: Even with 6128-compatible banking, a 464+expansion is **not** a complete 6128:
  * The 464 firmware lacks the 6128's extra BASIC commands and RSX extensions.
  * CP/M Plus (3.1) is not bundled with the Dk'tronics expansion.
  * The 464 firmware checks the BASIC ROM version number to detect 128K capability; CP/M Plus will refuse to boot unless the ROM version check is patched.
* **Auto-Disable on 6128**: When an external RAM expansion is connected to a CPC 6128, the external expansion **automatically disables the 6128's internal 64K extended RAM**. The total memory becomes: 64 KB base + external expansion size (not 64 + 64 + external). Only 64 KB of the 6128's internal memory is used; the second half of internal memory is disabled and replaced by the expansion RAM.

#### Standard Memory Expansion Protocol

The de facto standard for CPC RAM expansions (originally credited to Dk'tronics and Dobbertin) extends the 6128's MMR protocol for capacities up to 512 KB:

```
Port &7Fxx write data byte:
  Bits 7-6 : Must be `11` (MMR command)
  Bits 5-3 : 64K bank number (0..7, for expansions > 64K, max = 512K)
  Bits 2-0 : RAM configuration (same as 6128 Selections 0-7)
```

For expansions larger than 512 KB (e.g., RAM7, Yarek's 4MB expansion), additional address lines (`A10`, `A9`, `A8`) are used to select the upper bits of the 64K bank number. The LSBs of the 64K bank number remain in data bits `D5-D3`; the MSBs are in address bits `A10-A8` (typically in inverted form, so the first 512K block is accessed via Port `&7Fxx`, the next via `&7Exx`, etc.).

*Caution:* 64K bank numbering does not always start at 0 across all manufacturers. For example, the Dk'tronics 256K Silicon Disc numbers its 64K banks 4 through 7.

#### Motherboard Links: LK5, LK6, LK8 (Disable 128K RAM Banking)

The CPC 6128 mainboard can be factory-configured for 64 KB operation by:
1. Populating only 64 KB of RAM.
2. Omitting the PAL16L8 chip.
3. Installing wire links **LK5, LK6, and LK8**.

These links bypass the RAM banking logic and pass the `/CAS`, `A14`, and `A15` signals directly from the Gate Array to the unbanked RAM array, making the mainboard behave electrically as a 64 KB machine.

* **Default state on 6128**: LK5, LK6, LK8 are **not installed** (128K mode active).
* **Default state on 464/664**: Not applicable (no PAL exists on these mainboards).

The CPC+ series has a similar feature implemented via resistor R128 (aka R28, 10kΩ) instead of wire links.

#### CP/M Operating System Support

The 6128 ships with **CP/M 3.1 (CP/M Plus)** on its system discs, exploiting the extended 64K bank to provide a Transient Program Area (TPA) of 61 KB.

| Feature | CPC 464 + DDI-1 | CPC 6128 |
|---------|-----------------|----------|
| **CP/M Version** | 2.2 | 3.1 (Plus) + 2.2 (for backward compat) |
| **TPA** | 39 KB | 61 KB |
| **Warm Boot on Disc Change** | Required (`Ctrl+C`) | Not required |
| **Error Messages** | Minimal | Scrollable banner at bottom of screen |
| **Bundled Software** | Limited | GSX, Dr. Logo, Disckit3, Pip (single-drive) |

The 6128 includes CP/M 2.2 on one side of the system disc for backward compatibility with 464/664 CP/M software. Earlier CP/M software is easily up-graded, making the vast range of existing CP/M software (including the "freeware" of the CP/M user's group) accessible.

#### Firmware Compatibility Notes

The 6128 firmware is derived from the CPC 664 firmware, **not** the CPC 464 firmware. The 464 firmware has known bugs (e.g., the Line Input bug) that were fixed in the 664/6128 ROMs.

* **Firmware Call `&BD5B`** (6128-specific): Enables machine-code programs to access the second 64K bank. This call does not exist on the 464.
* **BASIC Interpreter**: The 6128 uses the same BASIC version as the 664 (v1.1). The 464 uses an earlier BASIC (v1.0) with the Line Input bug. The BASIC interpreter is unaware of the additional 64K — you cannot write bigger programs because the interpreter does not manage the extended bank.
* **ROM Version Detection**: CP/M Plus "detects" the 128K capability by checking the BASIC ROM version number. Emulator authors should ensure the 6128 reports the correct BASIC version (v1.1, same as 664) to enable CP/M Plus boot. A 6128 with a 64K-only external expansion will fail this check unless patched.
* **RSX Extensions**: The 6128 system disc includes a suite of RSX routines for managing the extra RAM:
  * `|BANKOPEN,n`: Sets up the second bank for string storage (n = string length, up to 255).
  * `|BANKWRITE`, `|BANKREAD`, `|BANKTIND`: Read/write/search records in the second bank (operates like a RAMdisc).
  * `|SCREENSWAP`, `|SCREENCOPY`: Copy 16K blocks between extended RAM and the video RAM area. Allows storing 4 additional screens, but displaying them takes ~0.5 seconds. Does **not** reprogram the video chip to fetch from elsewhere — only copies data in and out of the video RAM area.

#### Video RAM Restrictions

The PAL banking logic **only** affects the CPU's view of memory. The Gate Array always fetches video data from the base 64K RAM (Bank 0), regardless of the active MMR configuration.

* **You cannot use extended RAM as video RAM.** The Gate Array reads 2 bytes from the base 64K RAM every microsecond to generate RGB video signals; it has no knowledge of the PAL's banking state.
* **You cannot run DMA lists from extended RAM** (on Plus machines, the 3 ASIC DMA channels also read from base 64K RAM only).
* Software using `|SCREENSWAP` or `|SCREENCOPY` works around this by copying 16 KB blocks between the extended RAM and the active video RAM area in Bank 0.

#### Memory Access Priority

When the CPU accesses a memory address, multiple memory types may respond. The priority (highest first) is:

1. **ROM** (Lower or Upper, if paged in for reads).
2. **ASIC I/O Page** (Plus only, when unlocked and mapped to `&4000-&7FFF`).
3. **Extended RAM banks** (Bank 1 of 6128, or external expansion banks).
4. **Base 64K RAM** (Bank 0).

Memory **writes** always target the underlying RAM (never ROM). If multiple RAM banks are mapped to the same CPU address range, the highest-priority bank receives the write. All memory mapping features (RMR, RMR2, MMR, URR) affect **only** the CPU address space; other devices (Gate Array, ASIC DMAs) are unaffected and always use the base 64K RAM.