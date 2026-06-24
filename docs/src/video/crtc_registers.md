### CRTC Port I/O & Registers (Base Model: CPC 464 Type 0/1)

The CPC uses a Motorola 6845 (or compatible) Cathode Ray Tube Controller (CRTC) to generate screen addresses, sync timings, and raster limits. 

#### I/O Port Addressing
* **Register Select Port:** `&BCxx` (Output)
  * Writing to this port selects which internal register (`R0`–`R15`) is targeted by subsequent data writes.
* **Register Data Write Port:** `&BDxx` (Output)
  * Writing to this port updates the value stored in the currently selected register.

#### Register Reference & Reset Defaults
An emulator must initialize its CRTC registers to the following values upon cold reset:

| Register | Function | Default Value | Unit / Scale |
| :--- | :--- | :--- | :--- |
| **R0** | Horizontal Total | 63 | Character Clocks |
| **R1** | Horizontal Displayed | 40 | Character Clocks |
| **R2** | Horizontal Sync Position | 46 | Character Clocks |
| **R3** | Sync Width | 112 | Character Clocks (packed H/V) |
| **R4** | Vertical Total | 38 | Character Lines |
| **R5** | Vertical Total Adjust | 0 | Scanlines |
| **R6** | Vertical Displayed | 25 | Character Lines |
| **R7** | Vertical Sync Position | 30 | Character Lines |
| **R8** | Interlace and Skew | 0 | Flags |
| **R9** | Maximum Raster Address | 7 | Scanlines per Character Row (0 to 7) |
| **R10** | Cursor Start Raster | 0 | Scanline |
| **R11** | Cursor End Raster | 0 | Scanline |
| **R12** | Start Address (High) | 48 (`&30`) | Base Address Offset |
| **R13** | Start Address (Low) | 0 | Base Address Offset |
| **R14** | Cursor Register (High) | 192 (`&C0`) | Screen Pointer |
| **R15** | Cursor Register (Low) | 7 | Screen Pointer |
