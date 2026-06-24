# Disk Storage Subsystem Overview

Disk storage on the Amstrad CPC is controlled by a dedicated NEC uPD765 (specifically the µPD765A or µPD765B) Floppy Disk Controller (FDC). 

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
The uPD765 is an 8 MHz, active-high, non-DMA (polled interrupt) or DMA controller. In standard Amstrad CPC and DDI-1 configurations, data transfers are non-DMA, driven by high-speed CPU polling loops synchronized to the controller's hardware registers.

## Chapter Directory
* [FDC Low-Level Registers](fdc_registers.md)
* [FDC Command Phase & Instructions](fdc_commands.md)
* [Drive Set Up Timing & Parameters](parameters.md)
