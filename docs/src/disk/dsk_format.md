### DSK & EDSK Disk Image Format

The CPC uses `.DSK` files to store floppy disk images. There are two primary formats: Standard DSK and Extended DSK (EDSK). EDSK is preferred for copy-protected software as it allows variable track and sector sizes.

#### 1. Disk Information Block (256 bytes)
Always located at offset `0x0000`. The file format is identified by the first 34 bytes:
* **Standard DSK:** `"MV - CPCEMU Disk-File\r\nDisk-Info\r\n"`
* **Extended DSK:** `"EXTENDED CPC DSK File\r\nDisk-Info\r\n"`

| Offset | Size | Description |
| :--- | :--- | :--- |
| `00-21` | 34 | Header Tag |
| `22-2F` | 14 | Creator Name |
| `30` | 1 | Number of tracks |
| `31` | 1 | Number of sides |
| `32-33` | 2 | **Standard Only:** Track size (Little-endian). Includes 256-byte TIB. Ignored in Extended. |
| `34-FF` | varies | **Extended Only:** Track Size Table. 1 byte per track/side representing the MSB of the track length (Track Length = Byte * 256). A value of `0` indicates an unformatted/missing track. |

Tracks are stored sequentially: Track 0 Side 0, Track 0 Side 1, Track 1 Side 0, etc. In Standard DSK, offset = `0x100 + (Track * Sides + Side) * Track_Size`. In Extended DSK, offset = `0x100 + sum(previous track sizes)`.

#### 2. Track Information Block (256 bytes)
Located immediately after the DIB for the first track. 

| Offset | Size | Description |
| :--- | :--- | :--- |
| `00-0C` | 13 | `"Track-Info\r\n"` |
| `0D-0F` | 3 | Unused |
| `10` | 1 | Track Number |
| `11` | 1 | Side Number |
| `12` | 1 | Data Rate (0=Unknown, 1=SD/DD, 2=HD, 3=ED) |
| `13` | 1 | Recording Mode (0=Unknown, 1=FM, 2=MFM) |
| `14` | 1 | Sector Size (Shift factor, e.g., 2=512 bytes). Used for standard uniform sector sizing. |
| `15` | 1 | Number of Sectors |
| `16` | 1 | GAP#3 Length |
| `17` | 1 | Filler Byte |
| `18-FF` | varies | Sector Information List (8 bytes per sector). |

*Note: If a track contains >29 sectors, additional sector headers overflow into the next 256-byte boundary, expanding the TIB size. The track size in the DIB must reflect this rounded size.*

#### 3. Sector Information List
Each sector entry is 8 bytes. Sector data follows immediately after the full 256-byte (or larger) TIB.

| Offset | Size | Description |
| :--- | :--- | :--- |
| `00` | 1 | Track (C) |
| `01` | 1 | Side (H) |
| `02` | 1 | Sector ID (R) |
| `03` | 1 | Sector Size (N) |
| `04` | 1 | FDC Status Register 1 (ST1) |
| `05` | 1 | FDC Status Register 2 (ST2) |
| `06-07` | 2 | Actual Data Length in bytes (Little-endian). In Standard DSK, this is 0. In EDSK, it defines the exact stored bytes. |

#### 4. EDSK Data Length & Copy Protection Extensions
To simulate protections accurately, the EDSK actual data length (Bytes 06-07) is interpreted as follows:

* **Standard Length:** `2^(7+N)`. Emulator returns exact requested size.
* **Short Sectors:** Length < `2^(7+N)`. Emulator returns only the stored bytes, then reports an FDC error (using ST1/ST2).
* **Large Sectors (N>=6):** Up to the full sector length is stored (`2^(7+N)`). Max supported is N=8 (32K). Older limits (e.g., 6144 bytes for N=6) are deprecated.
* **Weak/Random Sectors:** Data length is an exact multiple of `2^(7+N)`. The file stores multiple copies of the sector data. The emulator should return a random copy on each read to simulate unstable magnetic media.
* **Gap 3 Data:** Data length exceeds normal sector length but is not an exact multiple. The extra bytes represent CRC and GAP#3 data hidden by protection schemes, read via `READ TRACK` commands.

#### 5. Sector Offset Block (Optional)
Appended to the end of the EDSK file if sector positional data is required (e.g., for protections checking sector spacing).
    
| Offset | Size | Description |
| :--- | :--- | :--- |
| `00-0D` | 14 | `"Offset-Info\r\n"` |
| `0E` | 1 | Unused (0) |
| `0F+` | varies | For each track: 2-byte track length, followed by 2-byte offsets for each sector (Little-endian). |