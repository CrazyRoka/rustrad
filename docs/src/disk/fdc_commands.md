### FDC Command Phase & Instructions

The µPD765A/B operates as a strict synchronous state machine. All interactions are divided into three distinct chronological phases.

```
  +------------------+
  |  Command Phase   |  <--- CPU writes sequential parameter bytes.
  +------------------+       Checked via MSR (RQM=1, DIO=0)
           |
           v
  +------------------+
  | Execution Phase  |  <--- Real-time data transfer.
  +------------------+       Polled CPU IO loops.
           |
           v
  +------------------+
  |   Result Phase   |  <--- CPU must read all output bytes.
  +------------------+       Checked via MSR (RQM=1, DIO=1)
```

---

### Phase Control and Handshaking

An emulator must enforce strict bus handshaking using bits `RQM` (Bit 7) and `DIO` (Bit 6) of the Main Status Register (`&FB7E`):

#### 1. Command Phase
The CPU initializes an instruction by writing a multi-byte sequence into the Data Register (`&FB7F`). 
* **Handshake Condition:** Before writing each byte, the CPU must poll the MSR until `RQM = 1` and `DIO = 0`.
* **Timing Delay:** The CPU should wait **12 μs** between byte writes before re-checking the MSR.
* **Trigger:** Writing the final command-specific parameter byte terminates the Command Phase and automatically initiates the Execution Phase.

#### 2. Execution Phase
The controller executes the requested read, write, or drive movement.
* **Non-DMA Mode (CPC standard):** Because standard CPC setups do not use the interrupt or DMA lines, software must high-speed poll the MSR:
  * For **Read** operations, wait for `RQM = 1` and `DIO = 1`, then read data from `&FB7F`.
  * For **Write** operations, wait for `RQM = 1` and `DIO = 0`, then write data to `&FB7F`.
  * **Timing Constraint:** To avoid overruns, transfers must occur within **26 μs (MFM mode)** or **54 μs (FM mode)**. The CPC FDC is clocked at 4 MHz, doubling the datasheet's 8 MHz service windows of 13 μs / 27 μs. If missed, the `OR` (Overrun) bit in `ST1` is set, and the execution phase terminates immediately.

#### 3. Result Phase
After completion or termination, status bytes are placed in the internal FIFO stack for reading.
* **Handshake Condition:** The CPU must poll the MSR until `RQM = 1` and `DIO = 1` before reading each result byte.
* **Completeness Rule:** The CPU **must read all bytes** defined for the command's result phase. The FDC is locked and will ignore any new commands until the final byte of the active Result Phase has been read out of the Data Register.
* **Seek/Recalibrate Exception:** The `Seek` and `Recalibrate` commands do not have a Result Phase. Instead, the CPU must poll the MSR until the command completes, and then **must** issue a `Sense Interrupt Status` command to clear the internal interrupt state and retrieve the drive status.

#### Result Phase ID Information (C, H, R, N)
If the processor terminates a read or write operation early, the ID information returned in the Result Phase depends on the `MT` (Multi-Track) bit, `HD` (Head), and whether the final sector transferred was less than or equal to `EOT` (End of Track):

| MT | HD | Final Sector Transferred | C | H | R | N |
|:--:|:--:|:---|:---|:---|:---|:---|
| 0 | 0 | Less than EOT | - | - | R + 1 | - |
| 0 | 0 | Equal to EOT | C + 1 | - | 0 | - |
| 0 | 1 | Less than EOT | - | - | R + 1 | - |
| 0 | 1 | Equal to EOT | C + 1 | - | 0 | - |
| 1 | 0 | Less than EOT | - | - | R + 1 | - |
| 1 | 0 | Equal to EOT | - | LSB | 0 | - |
| 1 | 1 | Less than EOT | - | - | R + 1 | - |
| 1 | 1 | Equal to EOT | C + 1 | LSB | 0 | - |

*An empty cell means the value is unchanged from the beginning of command execution.*
*LSB (Least Significant Bit): The least significant bit of H is complemented.*

---

### Error Detection & Encoding

* **CRC Algorithm:** The FDC uses the CCITT-CRC16 algorithm for error detection. The CRC register is initialized to `&FFFF` and is updated byte by byte. CRC bytes are written after the ID and Data fields in big-endian format.
* **Track Format:** The FDC only supports the IBM System/34 Double Density (MFM) track format on the CPC (FM mode is unusable due to unconnected pins). An MFM track contains about 6250 raw bytes.
* **Address Marks:** ID Address Marks (IDAM) and Data Address Marks (DAM) are preceded by three `A1` bytes to help the FDC lock onto the data stream. Deleted data sectors are marked by an `F8` byte instead of an `FB` byte.
* **Logical vs Physical IDs:** The track, sector, and head IDs are logical IDs only. These are defined during formatting and are not required to reflect the physical track, sector, or head numbers. However, when performing read or write operations, the target sector's logical cylinder ID must match the parameter `C` of the command, and its logical head ID `H` must match the currently active physical head `HD` (which is updated dynamically during multi-track operations). Additionally, physical track access is always constrained by the Present Cylinder Number (`PCN`) of the drive.
* **Deleted Data:** A sector with a Deleted Data Address Mark (DAM) is not actually deleted; the DAM-flag is just another ID bit. 'Deleted' sectors can be read/written just like normal data sectors if that ID bit is specified correctly in the command.
* **N Field Definition:** `N` defines the number of data bytes in a sector: `Data Size = 2^(N+7)`. `N=2` means 512 bytes. Real uPD765 behaviour treats size codes `>= 8` as `N=8` (32768 bytes), though older implementations masked this to 3 bits (`N=8` -> `N=0` / 128 bytes). EDSK images support up to `N=8` for full 32K sector storage. See [DSK & EDSK Disk Image Format](dsk_format.md).

---

### Copy Protection Schemes

The FDC's behavior is often abused for copy protection schemes:

1. **Gaps Protection:** Commonly used by French software houses. It consists of writing specific custom values (other than the standard `&4E`) in the separation area between two consecutive sectors. The FDC can read these custom gap bytes but cannot write them, making it hard to duplicate.
2. **Weak Sectors Protection:** Uses fully or partially unmagnetized sectors. The unmagnetized data appears as random values when read. The protected program reads these sectors multiple times; if the data changes on each read, the disk is recognized as original. The FDC cannot recreate unmagnetized portions of a sector. *(Note: EDSK images store these as multiple sector data copies. See [DSK & EDSK Disk Image Format](dsk_format.md).)*

---

### FDC Command Set Reference

The µPD765A/B supports 15 operational instructions. Commands are initiated via a multi-byte sequence in the Command Phase, and most return status and ID information in the Result Phase. 

#### Command Symbol Definitions
* **MT** (Multi-Track): `0` = Single track, `1` = Read/Write both sides of a cylinder (Side 0 to Side 1).
* **MF** (FM/MFM Mode): `0` = FM (Single Density), `1` = MFM (Double Density). *Note: CPC hardware only supports MFM.*
* **SK** (Skip): `1` = Skip sectors containing a Deleted Data Address Mark (DDAM).
* **HD** (Head Address): Physical head select (`0` = Side 0, `1` = Side 1).
* **US1, US0** (Unit Select): Selects the target drive (0 to 3). *Note: CPC only connects US0, max 2 drives.*
* **C** (Cylinder): Logical track number stored in the ID field.
* **H** (Head): Logical head number stored in the ID field (must match `HD`).
* **R** (Record): Starting sector number.
* **N** (Number): Sector size code. `Data Size = 2^(N+7)`. `N=2` is 512 bytes. `N=0` uses `DTL`.
* **EOT** (End of Track): Final sector number on a track to transfer.
* **GPL** (Gap Length): Gap 3 length used between sectors.
* **DTL** (Data Length): Special data length used when `N=0` (128 bytes).
* **STP** (Scan Step): `1` for contiguous sectors, `2` for alternate sectors during Scan.
* **NCN** (New Cylinder Number): Target track for Seek operations.
* **PCN** (Present Cylinder Number): Current track reported by Sense Interrupt Status.
* **SC** (Sectors): Sectors per track (used in Format).
* **D** (Data): Filler byte for formatting (e.g., `&E5`).
* **SRT** (Step Rate Time): 4-bit value. `F`=1ms, `E`=2ms, etc. (Doubled on CPC 4MHz clock).
* **HUT** (Head Unload Time): 4-bit value. 0=240ms, 1=16ms, 2=32ms...
* **HLT** (Head Load Time): 7-bit value. 01=2ms, 02=4ms...
* **ND** (Non-DMA Mode): `1` = Non-DMA mode (CPC standard), `0` = DMA mode.

---

#### 1. Read Data
Reads sector data from the disk to the CPU. Increments sector pointer for multi-sector reads.
* **Opcode Bits:** `MT MF SK 00110` (`&06` base)
* **Command Phase (9 Bytes):**
  1. Opcode byte
  2. `X X X X X HD US1 US0`
  3. `C` (Cylinder)
  4. `H` (Head)
  5. `R` (Record/Sector)
  6. `N` (Number)
  7. `EOT` (End of Track)
  8. `GPL` (Gap Length)
  9. `DTL` (Data Length, or `&FF` if N!=0)
* **Result Phase (7 Bytes):** `ST0`, `ST1`, `ST2`, `C`, `H`, `R`, `N`

#### 2. Read Deleted Data
Same as Read Data, but specifically targets sectors marked with a Deleted Data Address Mark (DDAM).
* **Opcode Bits:** `MT MF SK 01100` (`&0C` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Read Data.

#### 3. Write Data
Writes sector data from the CPU to the diskette.
* **Opcode Bits:** `MT MF 000101` (`&05` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Read Data.

#### 4. Write Deleted Data
Same as Write Data, but writes a Deleted Data Address Mark (DDAM) at the sector start instead of a normal DAM.
* **Opcode Bits:** `MT MF 001001` (`&09` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Read Data.

#### 5. Read Diagnostic (Read Track)
Continuous read of all sectors on the track starting from the index hole. Data is transferred regardless of CRC errors.
* **Opcode Bits:** `0 MF SK 00010` (`&02` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Read Data.

#### 6. Read ID
Returns the first valid Sector ID field encountered. No data transfer occurs during execution.
* **Opcode Bits:** `0 MF 001010` (`&0A` base)
* **Command Phase (2 Bytes):**
  1. Opcode byte
  2. `X X X X X HD US1 US0`
* **Result Phase (7 Bytes):** `ST0`, `ST1`, `ST2`, `C`, `H`, `R`, `N`

#### 7. Format Track (Write ID)
Synthesizes a standard track format (Gaps, IDAM, DAM) based on parameters supplied by the CPU.
* **Opcode Bits:** `0 MF 001101` (`&0D` base)
* **Command Phase (6 Bytes):**
  1. Opcode byte
  2. `X X X X X HD US1 US0`
  3. `N` (Bytes/sector size code)
  4. `SC` (Sectors/track)
  5. `GPL` (Gap 3 length)
  6. `D` (Filler byte)
* **Result Phase (7 Bytes):** `ST0`, `ST1`, `ST2`, `C`, `H`, `R`, `N` *(ID info has no meaning here except for reporting errors).*

#### 8. Scan Equal
Byte-for-byte comparison of disk sectors vs. CPU-supplied stream. Looks for `DFdd = DProcessor`.
* **Opcode Bits:** `MT MF SK 10001` (`&11` base)
* **Command Phase (9 Bytes):**
  1. Opcode byte
  2. `X X X X X HD US1 US0`
  3. `C`, 4. `H`, 5. `R`, 6. `N`, 7. `EOT`, 8. `GPL`
  9. `STP` (Scan Step: 1 or 2)
* **Result Phase (7 Bytes):** `ST0`, `ST1`, `ST2`, `C`, `H`, `R`, `N`

#### 9. Scan Low or Equal
Byte-for-byte comparison looking for `DFdd <= DProcessor`.
* **Opcode Bits:** `MT MF SK 11001` (`&19` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Scan Equal (Byte 9 is `STP`).

#### 10. Scan High or Equal
Byte-for-byte comparison looking for `DFdd >= DProcessor`.
* **Opcode Bits:** `MT MF SK 11101` (`&1D` base)
* **Command/Result Phase:** Same 9-byte Command / 7-byte Result layout as Scan Equal (Byte 9 is `STP`).

#### 11. Recalibrate
Retracts the drive head to Track 0. The FDC issues up to 77 step pulses; 80-track drives may need a second recalibrate.
* **Opcode Bits:** `00000111` (`&07`)
* **Command Phase (2 Bytes):**
  1. `00000111` (Opcode)
  2. `X X X X X 0 US1 US0`
* **Result Phase:** None. *(Must issue `Sense Interrupt Status` to retrieve status).*

#### 12. Sense Interrupt Status
Clears the FDC interrupt line and returns the current track position (`PCN`). Essential after Seek/Recalibrate.
* **Opcode Bits:** `00001000` (`&08`)
* **Command Phase (1 Byte):** `00001000` (Opcode)
* **Result Phase (2 Bytes):** `ST0`, `PCN` (Present Cylinder Number)

#### 13. Specify
Configures internal timers for drive mechanics (`SRT`, `HUT`, `HLT`) and the DMA/Non-DMA mode.
* **Opcode Bits:** `00000011` (`&03`)
* **Command Phase (3 Bytes):**
  1. `00000011` (Opcode)
  2. `SRT` (bits 7-4) | `HUT` (bits 3-0)
  3. `HLT` (bits 7-1) | `ND` (bit 0)
* **Result Phase:** None.

#### 14. Sense Drive Status
Returns real-time status of the physical floppy drive lines directly via `ST3`.
* **Opcode Bits:** `00000100` (`&04`)
* **Command Phase (2 Bytes):**
  1. `00000100` (Opcode)
  2. `X X X X X HD US1 US0`
* **Result Phase (1 Byte):** `ST3`

#### 15. Version
Returns the FDC silicon revision. `80H` = uPD765A, `90H` = uPD765B.
* **Opcode Bits:** `00010000` (`&10`)
* **Command Phase (1 Byte):** `00010000` (Opcode)
* **Result Phase (1 Byte):** `ST0` (Contains `&80` or `&90`)

#### 16. Seek
Moves the drive head to a specified cylinder number (`NCN`).
* **Opcode Bits:** `00001111` (`&0F`)
* **Command Phase (3 Bytes):**
  1. `00001111` (Opcode)
  2. `X X X X X HD US1 US0`
  3. `NCN` (New Cylinder Number)
* **Result Phase:** None. *(Must issue `Sense Interrupt Status` to retrieve status).*

---

### µPD765A vs. µPD765B Silicon Deltas

For maximum accuracy when running test suites or highly specific timing software, you must emulate the following functional differences between the two chip variants:

#### 1. Overrun Status (OR)
* **µPD765A:** If a CPU data transfer overrun occurs on the very last byte of a sector during read/write execution, the controller fails to register the error and the status bit `OR` inside Status Register 1 is left as `0`.
* **µPD765B:** The overrun logic is corrected. The `OR` bit is reliably set in ST1 if an overrun occurs on any byte, including the final byte of a sector.

#### 2. DMA Request (DRQ) Reset on Overrun
* **µPD765A:** Requires an active low DMA Acknowledge (`/DACK`) pulse to drop its DMA request (`DRQ`) signal after an overrun occurs. If `/DACK` is not generated, `DRQ` is held active, which can cause external DMA controllers to overwrite the subsequent Result Phase bytes.
* **µPD765B:** Automatically resets its internal `DRQ` latch immediately before transitioning into the Result Phase, completely independent of the `/DACK` signal state.

#### 3. Clock Constraints
* **µPD765A:** Demands that the rising edge of the system clock (`CLK`) and the write clock (`WCLK`) are strictly synchronized on the physical PCB.
* **µPD765B:** Contains internal clock-synchronization circuitry; it has no timing phase requirements between `CLK` and `WCLK`.