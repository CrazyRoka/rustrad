# Sound Subsystem Overview

Audio generation on the Amstrad CPC is handled by a General Instrument **AY-3-8912** Programmable Sound Generator (PSG). This chip provides three independent audio channels, a noise generator, and an 8-bit general-purpose parallel I/O port.

```
           +------------------+
CPU -----> |    PPI 8255      | (Port A / Port C control)
           +------------------+
                    |
                    v (Multiplexed Bus)
           +------------------+
           |   AY-3-8912      | ---> [ Analog Audio Out ]
           |     (PSG)        | ---> [ Keyboard Scan Matrix ] (via Port A)
           +------------------+
```

## System Integration

### Bus Gating via Intel 8255 PPI
The Z80 cannot access the AY-3-8912 directly. PSG control signals (BDIR, BC1) and data lines are multiplexed through the Intel 8255 PPI chip. Specifically, PPI Port A is used as a bidirectional data bus, and PPI Port C control lines drive the PSG bus state.

### Keyboard Scanning
The 8-bit parallel I/O port (Port A) inside the AY-3-8912 is connected directly to the keyboard matrix. The operating system polls this port to read keystroke states, scanning the matrix column-by-column.

## Chapter Directory
* [PSG Register Port I/O & I/O Ports](psg_io.md)