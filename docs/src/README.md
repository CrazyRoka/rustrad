# Amstrad CPC Architecture Reference Manual

This technical documentation repository is structured specifically to support the development of cycle-accurate Amstrad CPC hardware emulators. 

The material here focuses strictly on hardware behaviors, register-level interfaces, bus interactions, memory configurations, and signal timings.

## Target Hardware Specification

To simplify implementation and maintain clarity, this reference manual employs the **Base Model Paradigm**:

1. **The Base Model:** All core chapters describe the **Amstrad CPC 464** equipped with a **Type 0 or Type 1 CRTC (6845)**.
2. **The Delta Model:** Architectural differences present in subsequent hardware revisions (CPC 664, CPC 6128, and the Plus series) are isolated in dedicated **Delta** sub-sections. These sub-sections only detail specific deviations from the Base Model.

### Base Model Summary
* **CPU:** Zilog Z80A running at 4.0 MHz (effectively throttled to 3.2 MHz by Gate Array bus arbitration).
* **Memory:** 64 KB Dynamic RAM, 32 KB ROM (16 KB Lower ROM containing the OS, 16 KB Upper ROM containing BASIC).
* **Video:** CRTC 6845 (Type 0/1) coupled with the Amstrad Gate Array. Supporting three planar color modes.
* **Audio:** General Instrument AY-3-8912 Programmable Sound Generator (PSG).
* **I/O:** Intel 8255 Peripheral Interface (PPI) managing keyboard, cassette, and PSG control.

### Delta Model Summary

| Model | Key Differences from Base 464 |
|-------|-------------------------------|
| **CPC 664** | Replaces cassette deck with built-in 3" disc drive. Firmware bugs fixed (Line Input). Slightly extended BASIC. Gate Array 40008. |
| **CPC 6128** | 128 KB RAM (64 KB base + 64 KB extended via PAL16L8). 48 KB ROM (adds AMSDOS as ROM 7). Gate Array 40010. CP/M Plus 3.1 (61K TPA). External cassette port (5-pin DIN). See [Delta: CPC 6128 PAL Bank Switching](memory/delta_6128.md). |
| **CPC+ / GX4000** | Full ASIC integration (40489). Hardware sprites, DMA, analog ADC, RMR2 register, cartridge slot. 128K internal RAM. Different RGB output levels. See [Delta: CPC+ Hardware Sprites](video/sprites.md). |

#### CPC 6128 Specifics

Key architectural additions over the 464 base:

* **Extended RAM Banking**: A PAL16L8 chip manages a second 64 KB RAM bank, accessible via MMR commands (port `&7Fxx`, data bits 7-6 = `11`). The 464 lacks this PAL; MMR commands are silently ignored on the 464.
* **Extended ROM**: 48 KB physical ROM contains both BASIC (ROM 0) and AMSDOS (ROM 7), versus 32 KB (BASIC only) on the 464.
* **Firmware**: Derived from the CPC 664 firmware (not the 464). Includes `&BD5B` call for second-bank access. Some 464 "illegal" call targets may not work.
* **Video RAM Restriction**: The PAL banking only affects CPU addressing. The Gate Array always reads video data from the base 64K RAM; extended RAM cannot serve as video RAM.
* **External Expansion Auto-Disable**: Connecting an external RAM expansion to a 6128 automatically disables the internal 64K extended RAM.