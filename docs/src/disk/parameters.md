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

<!-- TODO: Verify exact controller track limits for Type 0 controllers -->
