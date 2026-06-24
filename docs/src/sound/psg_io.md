### PSG Register Port I/O & I/O Ports

The Amstrad CPC incorporates a General Instrument AY-3-8912 Programmable Sound Generator (PSG).

#### Register Read Bit-Masking
When registers are read back from the AY-3-8912, certain unused bits are forced to `0` regardless of what value was originally written:

* **Registers 1, 3, 5, 13 (Envelope/Pitch):** Bits $[7..4]$ are masked to `0`. Only bits $[3..0]$ are returned.
* **Registers 6, 8, 9, 10 (Noise/Amplitude):** Bits $[7..5]$ are masked to `0`. Only bits $[4..0]$ are returned.
* **Registers 0, 2, 4, 7, 11, 12:** Return the full 8-bit written values ($[7..0]$) unmodified.

#### I/O Ports A and B Logic
The AY-3-8912 physically possesses only **Port A** on its IC packaging. **Port B** is physically unconnected, but its internal register logic remains active in the silicon.

* **Port A:** Accessed via Register 14.
* **Port B:** Accessed via Register 15.

##### Port Direction Configuration (Register 7)
* **Port A Direction:** Bit 6 of Register 7 (0 = Input, 1 = Output).
* **Port B Direction:** Bit 7 of Register 7 (0 = Input, 1 = Output).

##### Read/Write Behavior Matrix:
* **Output Mode (Reg 7, Bit = 1):**
  * Writing to Reg 14/15 stores the data in the port's internal output register and drives the physical pins (Port A only).
  * Reading Reg 14/15 returns the current value of the internal output register logically ANDed with the signals present on the physical port pins:
    $$\text{Read Value} = \text{Output Register} \text{ AND } \text{Port Pins}$$
* **Input Mode (Reg 7, Bit = 0):**
  * Writing to Reg 14/15 still stores the written data in the internal output register.
  * Reading Reg 14/15 returns **only** the real-time, unlatched logic state of the physical port pins.

##### CPC Hardware Routing
* **Port A Pin Routing:** Connected directly to the CPC Keyboard Matrix lines. The OS assumes Port A is configured as Input. If reprogrammed to Output, the keyboard becomes entirely unresponsive.
* **Port B Emulation Note:** Since Port B has no physical pins, reading Register 15 in Input Mode (Register 7 Bit 7 = 0) must always return `&FF` to the CPU.