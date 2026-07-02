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
* **Logical vs Physical IDs:** The track, sector, and head IDs are logical IDs only. These are defined during formatting and are not required to reflect the physical track, sector, or head numbers. However, the exact same IDs must be specified when reading or writing.
* **Deleted Data:** A sector with a Deleted Data Address Mark (DAM) is not actually deleted; the DAM-flag is just another ID bit. 'Deleted' sectors can be read/written just like normal data sectors if that ID bit is specified correctly in the command.
* **N Field Definition:** `N` defines the number of data bytes in a sector: `Data Size = 2^(N+7)`. `N=2` means 512 bytes. Real uPD765 behaviour treats size codes `>= 8` as `N=8` (32768 bytes), though older implementations masked this to 3 bits (`N=8` -> `N=0` / 128 bytes). EDSK images support up to `N=8` for full 32K sector storage. See [DSK & EDSK Disk Image Format](dsk_format.md).

---

### Copy Protection Schemes

The FDC's behavior is often abused for copy protection schemes:

1. **Gaps Protection:** Commonly used by French software houses. It consists of writing specific custom values (other than the standard `&4E`) in the separation area between two consecutive sectors. The FDC can read these custom gap bytes but cannot write them, making it hard to duplicate.
2. **Weak Sectors Protection:** Uses fully or partially unmagnetized sectors. The unmagnetized data appears as random values when read. The protected program reads these sectors multiple times; if the data changes on each read, the disk is recognized as original. The FDC cannot recreate unmagnetized portions of a sector. *(Note: EDSK images store these as multiple sector data copies. See [DSK & EDSK Disk Image Format](dsk_format.md).)*

---

### FDC Command Set Reference

The µPD765A/B supports 15 operational instructions. Most commands require exactly **9 parameter bytes** written during the Command Phase, and return **7 status bytes** in the Result Phase.

#### Key Instruction Table

| Command Name | Opcode Byte 0 | Command Bytes | Result Bytes | Primary Operations |
| :--- | :---: | :---: | :---: | :--- |
| **Read Data** | `MT MF SK 00110` | 9 | 7 | Reads sectors from disk. Increments sector pointer for multi-sector reads. |
| **Write Data** | `MT MF 000101` | 9 | 7 | Writes sector data from CPU to diskette. |
| **Read Deleted Data** | `MT MF SK 01100` | 9 | 7 | Same as Read Data, but targets deleted data sectors (DDAM). |
| **Write Deleted Data** | `MT MF 001001` | 9 | 7 | Same as Write Data, but writes DDAM at sector start. |
| **Read Track** | `0 MF SK 00010` | 9 | 7 | Continuous read of all sectors on the track starting from the index hole. |
| **Read ID** | `0 MF 001010` | 2 | 7 | Returns the first valid Sector ID field encountered. |
| **Format Track** | `0 MF 001101` | 6 | 7 | Synthesizes standard track format (Gaps, IDAM, DAM). |
| **Scan Equal** | `MT MF SK 10001` | 9 | 7 | Byte-for-byte comparison of disk sectors vs. CPU-supplied stream. |
| **Scan Low or Equal** | `MT MF SK 11001` | 9 | 7 | Byte-for-byte comparison (Disk <= CPU). |
| **Scan High or Equal**| `MT MF SK 11101` | 9 | 7 | Byte-for-byte comparison (Disk >= CPU). |
| **Recalibrate** | `00000111` | 2 | 0 | Retracts the drive head to Track 0. Walks up to 77 tracks; 80-track drives may need a second recalibrate. |
| **Seek** | `00001111` | 3 | 0 | Moves drive head to a specified cylinder number (`NCN`). |
| **Specify** | `00000011` | 3 | 0 | Configures internal timers (`SRT`, `HUT`, `HLT`, and `ND`). |
| **Sense Drive Status**| `00000100` | 2 | 1 | Returns status register `ST3` for a selected drive unit. |
| **Sense Int Status** | `00001000` | 1 | 2 | Clears FDC interrupt line; returns `ST0` and `PCN` (current track). |
| **Version** | `00010000` | 1 | 1 | Returns FDC revision: `80H` (uPD765A), `90H` (uPD765B). |

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