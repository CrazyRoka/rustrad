# Disk Storage Subsystem Overview

Disk storage on the Amstrad CPC is controlled by a dedicated NEC µPD765 (specifically the µPD765A or µPD765B, or equivalents like the UMC UM8272A or Zilog Z765A) Floppy Disk Controller (FDC). 

```
          +-------------------+
CPU ----> |  Floppy Disk Ctrl | <---> [ FDD A ] (3" Drive)
          | (NEC uPD765A/B)   | <---> [ FDD B ] (External Option)
          +-------------------+
                    ^
                    | (Timing Control)
          +-------------------+
          | BIOS Calibration  |
          +-------------------+
```

## Low-Level Operations
The CPC architecture does not have a DMA controller associated with the FDC, and the FDC is clocked at **4 MHz** instead of the datasheet-standard 8 MHz. Therefore, all internal timings (such as step rates and data service windows) are doubled compared to the datasheet.

Data transfers are non-DMA, driven by high-speed CPU polling loops synchronized to the controller's hardware registers. The FDC's `INT` and `DRQ` pins are not connected on the CPC.

The FDC is internally a microcoded part with a primitive controller. It contains its own internal CPU, ROM, and RAM to sequence high-level commands.

## Chapter Directory
* [FDC Low-Level Registers](fdc_registers.md)
* [FDC Command Phase & Instructions](fdc_commands.md)
* [Drive Set Up Timing & Parameters](parameters.md)
* [DSK & EDSK Disk Image Format](dsk_format.md)