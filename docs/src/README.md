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