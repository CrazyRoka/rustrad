### Drive Set Up Timing & Parameters

For emulation of disc timing loops, drive execution delays are initialized in the AMSDOS parameter structures.

#### Disc Controller Timings (`&BE44` Data Block)
The disc interface timing registers are held in memory starting at block `&BE44` (Base Model with disk drive upgrade, or CPC 6128). An emulator can reference these to simulate realistic drive delays and rotational behavior:

* **Motor On Delay (`&BE44`):** 2 bytes. Default value is `&0032` (50 units of 20ms = 1.0 second). Maximum safe value is `&0023` (35 units of 20ms = 0.7 seconds).
* **Motor Off Delay (`&BE46`):** 2 bytes. Default value is `&00FA` (250 units of 20ms = 5.0 seconds). Maximum safe value is `&00C8` (200 units of 20ms = 4.0 seconds).
* **Write Current Off Delay (`&BE48`):** 1 byte. Default value is `&AF` (measured in units of 10 microseconds).
* **Head Settle Time (`&BE49`):** 1 byte. Default value is `&0F` (15ms).
* **Step Rate Period (`&BE4A`):** 1 byte. Default value is `&0C` (12ms). Maximum safe speed is `&0A` (10ms).
* **Head Unload Delay (`&BE4B`):** 1 byte. Default value is `&01`.
* **DMA/Non-DMA Mode Configuration (`&BE4C`):** 1 byte. `b0` designates non-DMA mode setting. `b1` to `b7` represent head load delay. Default value is `&03`.

#### Drive Format Structs (Extended Disc Parameter Block - XDPB)
The memory block starting at `&A890` (Drive A) and `&A8D0` (Drive B) details the hardware profile of the disk configurations:

```
Address Block Offset:
  +&00 (2 Bytes): Number of 128-byte sectors per track.
  +&02 (1 Byte) : log2(Block size) - 7 (e.g., &03 = 1024 bytes, &04 = 2048 bytes).
  +&03 (1 Byte) : (Block size / 128) - 1 (e.g., &07 = 1024 bytes).
  +&04 (1 Byte) : (Block size / 1024) - 1.
  +&05 (2 Bytes): Total data blocks per disk side.
  +&07 (2 Bytes): Directory entries minus 1.
  +&09 (2 Bytes): Allocation mask for directory entries (&0080 = 1 block, &00C0 = 2 blocks).
  +&0B (2 Bytes): Checksum size configuration = (Block size factor / 4).
  +&0D (2 Bytes): Reserved tracks count (Data Format = 0, IBM = 1, System = 2).
  +&0F (1 Byte) : Sector ID of the first sector (IBM = &01, System = &41, Data = &C1).
  +&10 (1 Byte) : Sectors per track count (Data = 9, System = 9, IBM = 8).
  +&11 (1 Byte) : Read/Write Gap Length.
  +&12 (1 Byte) : Format Gap Length.
  +&13 (1 Byte) : Format Filler Byte (Default: &E5).
  +&14 (1 Byte) : Sector Size configuration parameter (log2(size) - 7: &02 = 512, &03 = 1024).
  +&15 (1 Byte) : Records per sector count.
```

#### Standard Floppy Disk Formats

The CPC supports 3 standard MFM formats, all single-sided, 40-track, with a nominal rotation speed of 300 rpm. The FDC has a measured tolerance of ±12% for rotational speed variations.

| Format | Sectors/Track | Sector IDs | Sector Size | Catalog Position | Capacity |
| :--- | :---: | :--- | :---: | :--- | :--- |
| **DATA** | 9 | `&C1` to `&C9` | 512 bytes | Track 0, sectors `&C1`-`&C4` | 178 KB |
| **SYSTEM** | 9 | `&41` to `&49` | 512 bytes | Track 2, sectors `&41`-`&44` | 169 KB |
| **IBM** | 8 | `&01` to `&08` | 512 bytes | Track 1, sectors `&01`-`&04` | 154 KB |

*Note: The DATA format is the most commonly used on the CPC. The SYSTEM format is a VENDOR format with an added CP/M boot sector on Track 0.*

#### Catalog Structure & CP/M Terminology

AMSDOS and CP/M share the same directory structure. The catalog supports up to 64 entries of 32 bytes. A file larger than 16KB requires multiple entries.

| Byte Offset | Size | Description |
| :--- | :--- | :--- |
| **0** | 1 Byte | **User Number:** 0-255 (USER 229 / `&E5` = deleted file). |
| **1-8** | 8 Bytes | **Filename:** Always CAPS, max 8 chars. |
| **9-11** | 3 Bytes | **Extension:** Always CAPS, max 3 chars. Bit7 of Byte 9 = Read Only, Byte 10 = Hidden, Byte 11 = Archive. |
| **12** | 1 Byte | **Current Extent:** 0 is first extent. Up to 128K bytes directly addressed. |
| **13** | 1 Byte | Reserved. |
| **14** | 1 Byte | Extent High Byte (Not used). |
| **15** | 1 Byte | **Record Count:** Number of 128-byte blocks (up to 128 / `&80`). |
| **16-31** | 16 Bytes | **Block IDs:** 1 byte per 1KB block where file data is stored. |

* **CP/M Terminology:**
  * **Record:** 128 bytes (standard data access unit).
  * **Block:** 1 KB (group of records).
  * **Extent:** 16 KB (group of blocks).
  * **Sector:** 512 bytes (smallest physically addressable unit on CPC disk).

#### Custom Floppy Disk Formats

While AMSDOS only natively supports the 3 standard formats, the FDC can control 80-track and double-sided drives. For practical purposes, 42 tracks could be used on standard drives (the limit is specific to the drive, but 42 is a safe maximum).

Alternative DOS systems, such as **ParaDOS**, support up to 796 KB of usable space and 22 different disk file formats, including the standard AMSDOS ones. ParaDOS achieves this by utilizing 80-track and/or double-sided drives with 10 sectors per track.

#### HD Floppy Disks on CPC
It is theoretically possible to use HD floppy disks (1.44MB) on CPC using Gotek drives or Amiga HD floppy drives spinning at 150rpm. With a Gotek, you can simulate fantasy floppy disks with up to 255 cylinders, and the FDC will handle them perfectly fine, though timeouts in the FDC routines may need relaxing.

#### Track Layout (MFM Encoding)

An MFM track on the CPC contains about 6250 raw bytes. The FDC relies on specific gaps and address marks to parse the track:

* **Gaps:** Gaps (`GAP1`, `GAP2`, `GAP3`) are filled with `4E` bytes and exist to accommodate variations in rotation speed and prevent overlapping sectors.
* **Address Marks:** 
  * **IAM** (Index Address Mark): Marks the beginning of a track. Preceded by `00` bytes.
  * **IDAM** (ID Address Mark): Marks the beginning of a sector's header. Preceded by three `A1` bytes. Data follows as `FE`.
  * **DAM** (Data Address Mark): Marks the beginning of actual data in a sector. Preceded by three `A1` bytes. Data follows as `FB` (normal) or `F8` (deleted).
* **Error Detection:** CCITT-CRC16 checksums (initialized to `&FFFF`) are appended after the ID and Data fields in big-endian format.