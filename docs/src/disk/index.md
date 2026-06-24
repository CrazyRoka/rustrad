# Disk Storage Subsystem Overview

Disk storage on the Amstrad CPC is controlled by a dedicated NEC uPD765 (or compatible) Floppy Disk Controller (FDC). This chip works alongside the Amstrad Microcomputer System Disk Operating System (AMSDOS).

```
          +-------------------+
CPU ----> |  Floppy Disk Ctrl | <---> [ FDD A ]
          |    (NEC uPD765)   | <---> [ FDD B ]
          +-------------------+
                    ^
                    | (Timing Control)
          +-------------------+
          | BIOS Calibration  |
          +-------------------+
```

## Functional Roles

### NEC uPD765 FDC
Manages low-level sector reading, writing, and formatting. It decodes standard sector configurations (MFM) and handles track-seeking commands. The CPC interface is non-DMA; data transfers are handled directly by the CPU through polling loops.

### Drive Calibration Timers
AMSDOS maintains a software calibration block in RAM (`&BE44`) to enforce motor spin-up delays, head settle times, and drive stepping periods. Accurate emulation of these timing parameters is necessary for software that implements custom floppy-based copy protections.

## Chapter Directory
* [Drive Set Up Timing & Parameters](parameters.md)

