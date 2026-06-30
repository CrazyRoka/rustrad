### 16-Bit I/O Port Address Decoding

Unlike standard 8-bit Z80 systems, the Amstrad CPC decodes peripheral port numbers using all **16 address lines (`A15`–`A0`)**. 

#### Hardware Register-to-Address Generation
During Z80 I/O instruction execution, address lines are driven as follows:
* **`IN r, (C)` / `OUT (C), r`:** The 16-bit port address is formed by the `BC` register pair (High byte `A15`–`A8` from `B`, Low byte `A7`–`A0` from `C`).
* **`IN A, (n)` / `OUT (n), A`:** The 16-bit port address is formed by the `A` register and the immediate operand `n` (High byte `A15`–`A8` from `A`, Low byte `A7`–`A0` from `n`).

*Emulator Note:* Standard 8-bit I/O instructions like `OUT (n), A` or `IN A, (n)` are highly restricted in practice because the CPU mirrors the `A` register onto the upper address bus half (`A15`–`A8`). If the `A` register's value does not align with the target peripheral's active-low address mask, the write or read operation will address the wrong device or fail entirely.

#### The "One Low Bit" Selection Rule & Hardware Hazards
The Amstrad CPC hardware selects its main internal and expansion peripherals by checking if specific bits in the upper half of the address bus (`A15`–`A10`) are driven **low (0)**:

* `A15` low → Gate Array Select
* `A14` low → CRTC Select
* `A13` low → ROM Select
* `A12` low → Printer Port Select
* `A11` low → Intel 8255 PPI Select
* `A10` low → Expansion Peripherals Select

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

##### Gate Array vs PAL Decoding at Port `&7Fxx`

The **Gate Array** and the **PAL** (present only on the CPC 6128, or via external RAM expansion) both respond to writes at port `&7Fxx`, but they use **different address decoding** and **different command codes**:

| Aspect | Gate Array | PAL (6128 / expansions) |
|--------|-----------|------------------------|
| Address decode | `A15 = 0` AND `A14 = 1` | `A15 = 0` only (ignores `A14`) |
| I/O Read | Responds (returns floating bus) | Does not respond |
| I/O Write | Responds | Responds |
| Active command bits | Bits 7,6 = `00`, `01`, or `10` | Bits 7,6 = `11` only |

Because their command codes are mutually exclusive, a single write to `&7Fxx` never triggers both chips simultaneously. The PAL's broader address decode (`A15 = 0` only, ignoring `A14`) means it also responds to writes at ports where `A14 = 0` (e.g., `&3Fxx`), but in practice, software always uses `&7Fxx` for MMR commands to avoid conflicts with the CRTC (`&BCxx`–`&BFxx`, selected by `A14 = 0`).

**On the CPC 464 (no PAL)**: The MMR row in the table above is effectively a no-op. Writes with data bits 7,6 = `11` to port `&7Fxx` are silently ignored by the Gate Array. No memory configuration change occurs.

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

#### Upper ROM Bank Number Selection (Port `&DFxx`)

Writing to Port `&DFxx` selects the Upper ROM Bank Number (0–255) to be mapped to the CPU address space at `&C000`–`&FFFF`. The actual enabling of the ROM (or mapping RAM to that region) is controlled by the Gate Array's RMR register (Bit 3).

##### Hardware Implementation (CPC)
The ROM Bank Number is **not stored anywhere inside the CPC itself**. Instead, peripherals must watch the Z80 bus for writes to Port `&DFxx`. If the peripheral's onboard logic (e.g., a 74LS74 flip-flop) detects a bank number match, it sets a flip-flop.

When the CPU reads from `&C000`–`&FFFF` (A15=HIGH) and the flip-flop indicates a match, the peripheral sets its ROM's `/OE` (Output Enable) low and outputs `ROMDIS=HIGH` to the CPC, which disables the internal BASIC ROM. The CPC's `/ROMEN` signal is wired to the peripheral ROM's `/CS` (Chip Select). `A14` does not need to be decoded by peripherals since there is no ROM at `&8000`–`&BFFF`, only at `&C000`–`&FFFF`.

By default, if there are no peripherals asserting `ROMDIS=HIGH`, the internal BASIC ROM is mapped to all bank numbers (0–255).

##### Common ROM Bank Numbers

| Bank Number | Description |
|-------------|-------------|
| `00h` | BASIC (or AMSDOS, depending on LK1 on the DDI-1 board) |
| `07h` | AMSDOS (or BASIC, depending on LK1 on the DDI-1 board) |
| `00h`–`07h` | Bootable ROMs on CPC 464/664/6128 (scanned by `KL_ROM_WALK`) |
| `08h`–`0Fh` | Bootable ROMs on CPC 664/6128 (scanned by `KL_ROM_WALK`) |
| `10h`–`FFh` | Non-bootable ROMs (or secondary banks of Bootable ROMs) |
| `FCh`–`FFh` | Can be used, but aren't accessible by BIOS functions |
| `FFh` | BASIC (or ROM with similar ID); used for the crude 128K RAM-size detection in CP/M+ and BIOS key scan detection in AMSDOS+ |

*Note:* The DDI-1 link (LK1) controls whether AMSDOS is assigned to ROM 0 or ROM 7. By default, AMSDOS is ROM 7. If LK1 is changed, AMSDOS becomes ROM 0 and automatically boots CP/M from drive A on startup.

##### ASIC ROM Numbering System (CPC Plus)

On the classic CPC, Port `&DFxx` simply accepts a logical ROM number from 0 to 255.

On the Amstrad Plus, this port behaves in two modes depending on **Bit 7** of the written value:
* **Bit 7 = 0:** Selects a **logical ROM** number (0–127). ROM boards attached to the expansion port are assigned logical IDs.
* **Bit 7 = 1:** Bits 6–5 are ignored. Bits 4–0 select a **physical ROM** number (0–31) directly from the cartridge.

The Plus factory cartridge contains the following **physical ROMs**:

| Physical ROM | Content |
|--------------|---------|
| `0` | Firmware |
| `1` | BASIC |
| `2` | Unused |
| `3` | AMSDOS |
| `4` | Burnin' Rubber ROM 0 |
| `5` | Burnin' Rubber ROM 1 |
| `6` | Burnin' Rubber ROM 2 |
| `7` | Burnin' Rubber ROM 3 |

To maintain compatibility with classic CPC software, the Plus firmware also maps physical ROMs to logical IDs:
* Physical ROM 1 (BASIC) is mapped to **Logical ROM 0**.
* Physical ROM 3 (AMSDOS) is mapped to **Logical ROM 7**.

Logical ROMs can be overridden by a ROM board in the expansion port, but physical ROMs on the cartridge cannot be replaced.

**Summary of ROM Limits:**
* **CPC:** 1 Lower ROM (Firmware) + up to 256 Upper ROMs.
* **Plus:** Up to 32 physical ROMs per cartridge. The first 8 physical ROMs can also be accessed as Lower ROMs. ROM boards provide up to 128 logical ROMs, accessible only as Upper ROMs.

---

### The 7-Bit Printer Port Interface & Pin 14 Ground Hazard

The physical Centronics printer port (`&EFxx`) is managed using a **74LS273 8-bit latch** connected to the CPU data bus. However, the hardware implementation implements several strict deviations from standard 8-bit Centronics specifications:

1. **7-Bit Data Bus**: Only 7 physical data pins (`D0` through `D6`) are wired from the latch to the printer connector. Hardware or software trying to write 8-bit character data must mask off bit 7.
2. **Data Bus Bit 7 as STROBE**: Pin 14 (the printer's active-low `/STROBE` line) is driven by the data bus's **Bit 7**. Writing a byte with Bit 7 set to `1` is inverted by an on-board 74LS10 NAND gate (configured as an inverter) to pull `/STROBE` Low (`0`), signalling the printer to latch the active data bytes on `D0`–`D6`.
3. **Motherboard Pin 14 Ground Hazard**: On physical CPC mainboards, Pin 14 of the physical Centronics port is permanently hardwired to **Ground** (GND). On standard Centronics printers, Pin 14 is often used as the "Auto Line Feed" (Auto-LF) signal line. Because this pin is grounded, standard printers are forced into an auto-LF state, resulting in unwanted double-spaced printouts on carriage returns unless the physical line on the printer cable is severed or the printer's internal DIP switches are set to override it.

---

### Safe User I/O Ranges
To avoid bus contention and collision with standard internal hardware or recognized expansion cards, custom user-designed peripherals must restrict their I/O addresses to the following blocks:
* `&F8E0` – `&F8FE`
* `&F9E0` – `&F9FF`
* `&FAE0` – `&FAFF`
* `&FBE0` – `&FBFF`