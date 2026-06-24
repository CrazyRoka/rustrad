### PSG Register Port I/O & Bus Operations

The Amstrad CPC, CPC+, and KC Compact systems feature an AY-3-8912 Programmable Sound Generator running at a fixed 1 MHz frequency.

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

#### PSG-PPI Control Bus Interface
The CPU cannot communicate with the PSG directly. All bus transitions are managed by writing to PPI Port C, which controls the PSG's `BDIR` and `BC1` signals:

| PPI Port C Bit 7 (BDIR) | PPI Port C Bit 6 (BC1) | Selected PSG Bus Function | Description |
| :---: | :---: | :--- | :--- |
| 0 | 0 | **Inactive Mode** | PSG disconnects its data bus. No read/write occurs. |
| 0 | 1 | **Read Register** | PSG outputs the contents of the currently selected register to PPI Port A. |
| 1 | 0 | **Write Register** | PSG writes the data byte on PPI Port A into the currently selected register. |
| 1 | 1 | **Select Register** | PSG latches the register address byte on PPI Port A to select a target register (Registers 0–15). |

#### The "Inactive" Bus Transition Rule
To prevent data bus collisions and internal state corruption, software must transition the PSG through the **Inactive state (`00`)** before initiating any function changes (such as transitioning from *Select Register* to *Write Register*).
* **ASIC Emulation Note:** While physical CPC models are somewhat forgiving of omitted inactive cycles, the CPC+ ASIC emulation layer is strictly intolerant. Failing to insert inactive phases between PSG function changes will corrupt the sound registers on CPC+ hardware.

#### Hardware Quirks

##### Low Tone Period Cutoff
If a channel's 12-bit Tone Period registers (representing the frequency divisor) are set in the range of **`0` to `4`**, the analog generation hardware is unable to resolve the clock output. The PSG channel falls silent.

##### Port B Read Default
Since the AY-3-8912 model lacks a physical Port B interface on its packaging, reading Register 15 when Port B is configured as input (Register 7, Bit 7 = 0) must always return `&FF`.