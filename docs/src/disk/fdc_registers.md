
The NEC µPD765A/B Floppy Disk Controller exposes only two directly addressable registers to the CPU. Address decoding utilizes the 16-bit I/O map.

---

### I/O Port Address Mapping

The FDC is selected when address bus line `A10` is driven low and line `A7` is driven low. The specific register selection is determined by address line `A0`:

* **`&FA7E` (A0 = 0) - Read Only:** Main Status Register (MSR)
* **`&FA7F` (A0 = 1) - Read/Write:** FDC Data Register (FIFO/Stack)

#### Address Decoding Truth Table
```
A15 A14 A13 A12 A11 A10 A9  A8  A7  A6  A5  A4  A3  A2  A1  A0  | Selected Port
 -   -   -   -   -   0   -   -   0   -   -   -   -   -   -   0  | &FA7E (MSR Read)
 -   -   -   -   -   0   -   -   0   -   -   -   -   -   -   1  | &FA7F (Data FIFO Read/Write)
```

---

### Main Status Register (MSR)

The MSR is an 8-bit read-only register that provides the handshake signals, data transfer direction, and busy states required to synchronize CPU accesses.

| Bit | Name | Function / Description |
| :---: | :--- | :--- |
| **7** | **RQM** (Request for Master) | `1` = Data Register is ready to send or receive bytes. `0` = Not ready. |
| **6** | **DIO** (Data Input/Output) | Indicates transfer direction: `1` = Read (FDC to CPU), `0` = Write (CPU to FDC). |
| **5** | **EXM** (Execution Mode) | Set to `1` only during the execution phase of non-DMA command loops. |
| **4** | **CB** (FDC Busy) | `1` = Read, Write, or Format command is currently in progress. |
| **3** | **D3B** (Drive 3 Busy) | Drive 3 is currently executing a seek/recalibrate sequence. |
| **2** | **D2B** (Drive 2 Busy) | Drive 2 is currently executing a seek/recalibrate sequence. |
| **1** | **D1B** (Drive 1 Busy) | Drive 1 is currently executing a seek/recalibrate sequence. |
| **0** | **D0B** (Drive 0 Busy) | Drive 0 is currently executing a seek/recalibrate sequence. |

---

### Internal Status Registers (ST0, ST1, ST2, ST3)

These four registers are not accessible via standard I/O ports. They are placed in an internal register stack and are read-out sequentially through the Data Register (`&FA7F`) solely during a command's **Result Phase**.

#### Status Register 0 (ST0)
* **Bits [7..6] - Interrupt Code (IC):**
  * `00` = Normal Termination (NT). Command executed successfully.
  * `01` = Abnormal Termination (AT). Command started but failed to complete.
  * `10` = Invalid Command (IC). Command was never started.
  * `11` = Abnormal Termination (Ready Changed). Drive Ready signal changed state during execution.
* **Bit 5 - Seek End (SE):** Set to `1` when a Seek or Recalibrate command finishes.
* **Bit 4 - Equipment Check (EC):** Set if a drive fault is detected, or if Track 0 fails to assert after 77 step pulses.
* **Bit 3 - Not Ready (NR):** Set if a read/write command is issued to a drive that is offline or single-sided (Side 1 access).
* **Bit 2 - Head Address (HD):** Reports the selected physical head address during interrupt.
* **Bits [1..0] - Unit Select (US1, US0):** Identifies the active drive ID at the time of interrupt.

#### Status Register 1 (ST1)
* **Bit 7 - End of Cylinder (EN):** Set if the FDC attempts to access a sector index beyond the final sector of a track.
* **Bit 6:** Unused (always `0`).
* **Bit 5 - Data Error (DE):** CRC error detected in either the Sector ID field or the Data field.
* **Bit 4 - Overrun (OR):** Set if the Z80 CPU fails to read/write a byte within the MFM/FM timing limits (13 μs / 27 μs).
* **Bit 3:** Unused (always `0`).
* **Bit 2 - No Data (ND):** Sector specified in command parameters could not be located on the track.
* **Bit 1 - Not Writeable (NW):** Write attempt detected a physical write-protect tab on the medium.
* **Bit 0 - Missing Address Mark (MA):** FDC did not find an ID Address Mark (IDAM) before 2 index pulses, or failed to find a Data Address Mark (DAM) after IDAM.

#### Status Register 2 (ST2)
* **Bit 7:** Unused (always `0`).
* **Bit 6 - Control Mark (CM):** Read Data encountered a sector with a Deleted Data Address Mark (DDAM), or Read Deleted Data encountered a standard DAM.
* **Bit 5 - Data Error in Data Field (DD):** CRC error detected specifically within the sector's Data Field.
* **Bit 4 - Wrong Cylinder (WC):** The cylinder value (`C`) recorded on the track ID field does not match the cylinder number stored in the FDC's internal register.
* **Bit 3 - Scan Equal Hit (SH):** The Scan command criteria was met.
* **Bit 2 - Scan Not Satisfied (SN):** No sector on the track met the Scan comparison criteria.
* **Bit 1 - Bad Cylinder (BC):** Track ID cylinder byte `C` does not match FDC register, and its value on disk is `&FF`.
* **Bit 0 - Missing Address Mark in Data Field (MD):** The FDC could not locate a DAM or DDAM inside the sector.

#### Status Register 3 (ST3)
*Reports real-time status of the physical floppy drive lines directly (via Sense Drive Status).*
* **Bit 7 - Fault (FT):** Drive reports a fault.
* **Bit 6 - Write Protected (WP):** Drive write-protect sensor active.
* **Bit 5 - Ready (RY):** Drive Ready line active.
* **Bit 4 - Track 0 (T0):** Drive head is at Track 0.
* **Bit 3 - Two-Side (TS):** Drive reports dual-sided medium is inserted.
* **Bit 2 - Head Address (HD):** Reports the physical head select signal state.
* **Bits [1..0] - Unit Select (US1, US0):** Reports the selected physical drive lines.
