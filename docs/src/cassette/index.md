# Cassette Storage Overview

The Amstrad CPC features built-in support for cassette tape storage. Depending on the computer revision, tape read/write circuitry is either integrated into an internal tape deck or exposed via an external connector.

```
          +-------------------+
CPU <---> |   Intel 8255 PPI  | <---> [ Read Bit: Port B Bit 7 ]
          |  (Ports B and C)  | <---> [ Write Bit: Port C Bit 5 ]
          +-------------------+ <---> [ Motor Relay: Port C Bit 4 ]
```

## Software Storage Standards
Tape storage operates on a frequency-modulated physical signal. Emulators process tape media using the standard **.CDT** file format (digitally identical to the Sinclair ZX Spectrum `.TZX` format), which preserves the exact timing pulses, pilot tones, and data blocks of original cassette tapes.

## Chapter Directory
* [Cassette Hardware Interface](hardware.md)
* [CDT Tape Image Format](cdt_format.md)
