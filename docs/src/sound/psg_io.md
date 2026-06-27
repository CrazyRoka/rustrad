### PSG Register Port I/O & Bus Operations

The Amstrad CPC, CPC+, and KC Compact systems feature a custom General Instrument **AY-3-8912** Programmable Sound Generator (PSG) operating at a fixed clock frequency of **1.0 MHz** (derived by dividing the system's 4.0 MHz clock).

---

### Physical Pin Configuration & Bus Control

The AY-3-8912 is packaged as a 28-pin IC. Unlike its sister chip, the AY-3-8910 (which features two 8-bit parallel I/O ports), the 8912 only exposes **I/O Port A** to physical pins.

```
                         AY-3-8912 Pinout
                          +---\/---+
       ANALOG CHANNEL C - | 1    28| - DA0 (Multiplexed Data/Addr)
                 TEST 1 - | 2    27| - DA1
                    Vcc - | 3    26| - DA2
       ANALOG CHANNEL B - | 4    25| - DA3
       ANALOG CHANNEL A - | 5    24| - DA4
                    Gnd - | 6    23| - DA5
                   IOA7 - | 7    22| - DA6
                   IOA6 - | 8    21| - DA7
                   IOA5 - | 9    20| - BC1
                   IOA4 - |10    19| - BC2 (Hardwired High in CPC)
                   IOA3 - |11    18| - BDIR
                   IOA2 - |12    17| - A8 (Hardwired High in CPC)
                   IOA1 - |13    16| - /RESET
                   IOA0 - |14    15| - CLOCK (1 MHz)
                          +--------+
```

#### The PSG-PPI Bus Protocol
The PSG multiplexes its address selection and data transfers over the same 8-bit bidirectional data lines (`DA0`–`DA7`). This bus state is driven by the logic combinations of control pins `BDIR`, `BC1`, and `BC2`.

By design, the PSG decodes active addressing using internal chip-select logic requiring the active-low address line `\A9` to be `0` (internally tied on the 28-pin AY-3-8912) and active-high address line `A8` to be `1`. In the Amstrad CPC architecture:
* **`BC2` and `A8`** are hardwired directly to the system's `+5V` line (permanently active high `1`). Tying `BC2` high takes advantage of bus control decoding redundancies, simplifying active CPU management of the PSG down to just two signal lines.
* **`BDIR`** is tied to **PPI Port C Bit 7**.
* **`BC1`** is tied to **PPI Port C Bit 6**.

This simplifies PSG bus function selection to the following truth table:

| PPI Port C Bit 7 (BDIR) | PPI Port C Bit 6 (BC1) | Selected PSG Bus Function | Description |
| :---: | :---: | :--- | :--- |
| **0** | **0** | **Inactive Mode** | PSG disconnects its data bus, entering a high-impedance state. |
| **0** | **1** | **Read Register** | PSG outputs the contents of the currently selected register to PPI Port A. |
| **1** | **0** | **Write Register** | PSG writes the data byte on PPI Port A into the currently selected register. |
| **1** | **1** | **Select Register** | PSG latches the register address byte on PPI Port A to select a target register (0–15). |

#### The "Inactive" Bus Transition Rule
To prevent data bus collisions and accidental register corruption, software must transition the PSG through the **Inactive state (`00`)** between functions (e.g., moving from *Select Register* to *Write Register*).
* **ASIC Emulation Note:** Standard CPC systems are occasionally forgiving of omitted inactive cycles, but the CPC+ ASIC integration is strictly intolerant. If the transition through state `00` is missing, subsequent write commands are ignored or corrupt the selected register index on CPC+ hardware.

#### Register Address Latch Durability
Once a register index (0–15) has been decoded and held in the internal Address Latch, it remains active indefinitely. The CPU can execute successive write or read operations to the currently selected register without performing redundant address-latching phases.

---

### Register Map & Read Masking Rules

The PSG has 16 internal registers (Registers 0–15). Because the internal hardware register latches do not implement all 8 bits on the silicon, reading back from these registers returns specific masked bit-patterns.

| Register | Function | Bit Range | Read Mask (Unused bits forced to `0`) |
| :---: | :--- | :---: | :--- |
| **R0** | Channel A Tone Period (Fine) | `[7..0]` | Returns full 8-bit value written |
| **R1** | Channel A Tone Period (Coarse) | `[3..0]` | Bits `[7..4]` are masked to `0` |
| **R2** | Channel B Tone Period (Fine) | `[7..0]` | Returns full 8-bit value written |
| **R3** | Channel B Tone Period (Coarse) | `[3..0]` | Bits `[7..4]` are masked to `0` |
| **R4** | Channel C Tone Period (Fine) | `[7..0]` | Returns full 8-bit value written |
| **R5** | Channel C Tone Period (Coarse) | `[3..0]` | Bits `[7..4]` are masked to `0` |
| **R6** | Noise Period | `[4..0]` | Bits `[7..5]` are masked to `0` |
| **R7** | Mixer & Port Configuration | `[7..0]` | Returns full 8-bit value written |
| **R8** | Channel A Amplitude / Mode | `[4..0]` | Bits `[7..5]` are masked to `0` |
| **R9** | Channel B Amplitude / Mode | `[4..0]` | Bits `[7..5]` are masked to `0` |
| **R10** | Channel C Amplitude / Mode | `[4..0]` | Bits `[7..5]` are masked to `0` |
| **R11** | Envelope Period (Fine) | `[7..0]` | Returns full 8-bit value written |
| **R12** | Envelope Period (Coarse) | `[7..0]` | Returns full 8-bit value written |
| **R13** | Envelope Shape | `[3..0]` | Bits `[7..4]` are masked to `0` |
| **R14** | Parallel I/O Port A | `[7..0]` | Read state depends on Reg 7 direction (see below) |
| **R15** | Parallel I/O Port B (Unconnected)| `[7..0]` | Read state depends on Reg 7 direction (returns `&FF` on CPC) |

---

### Low-Level Register Specifications & Calculations

#### 1. Tone Period Generators (Registers 0–5)
Channels A, B, and C output analog square-wave tones. Each channel uses a fine-tune (8-bit) and coarse-tune (4-bit) register pair to form a 12-bit divisor value (0 to 4095).

```
12-Bit Programmed Divisor:
  Coarse Register (R1, R3, R5)          Fine Register (R0, R2, R4)
  15  14  13  12  11  10  09  08       07  06  05  04  03  02  01  00
 +---+---+---+---+---+---+---+---+    +---+---+---+---+---+---+---+---+
 | 0 | 0 | 0 | 0 |     Divisor   |    |            Divisor            |
 +---+---+---+---+---+---+---+---+    +---+---+---+---+---+---+---+---+
```

* **Calculation Formula:**
  ```rust
  // Calculates target frequency based on the 1 MHz chip clock
  Tone_Frequency_Hz = 1_000_000.0 / (16.0 * Programmed_Divisor as f64);
  ```
* **Low Period Cutoff (Mute):** If the combined 12-bit divisor is set in the range **`0` to `4`**, the analog generation hardware fails to cycle and the channel falls silent.

#### 2. Noise Period Generator (Register 6)
An internal 5-bit register specifies the period divisor (0 to 31) for a pseudo-random, frequency-modulated pulse-width square wave (white noise).
* **Calculation Formula:**
  ```rust
  Noise_Frequency_Hz = 1_000_000.0 / (16.0 * Programmed_Period as f64);
  ```

#### 3. Mixer Control (Register 7)
Configures which sound generators (Tone and/or Noise) are enabled for output, and specifies the data directions for Ports A and B.
* **Active State:** Logic **`0` = Enabled**, Logic **`1` = Disabled / Muted**.

```
Bit 7: Port B I/O Direction (0 = Input, 1 = Output) - Ignored by AY-3-8912 physical pins
Bit 6: Port A I/O Direction (0 = Input, 1 = Output)
Bit 5: Channel C Noise Output Disable
Bit 4: Channel B Noise Output Disable
Bit 3: Channel A Noise Output Disable
Bit 2: Channel C Tone Output Disable
Bit 1: Channel B Tone Output Disable
Bit 0: Channel A Tone Output Disable
```

#### 4. Channel Amplitude & Mode Registers (Registers 8–10)
Configures how the channel's output volume level is determined.
* **Fixed Amplitude Mode (Bit 4 = 0):** The output level is fixed. Bits `[3..0]` define the volume index ($0$ to $15$).
* **Hardware Envelope Mode (Bit 4 = 1):** Fixed volume is ignored. The output amplitude is modulated dynamically by the hardware envelope generator (Registers 11–13).

```
Bit 4: Amplitude Mode (0 = Fixed Volume, 1 = Envelope Control)
Bits [3..0]: Fixed volume value (used only if Bit 4 is 0)
```

#### 5. Envelope Period Duration (Registers 11–12)
Specifies the 16-bit frequency divisor (0 to 65535) used to cycle the envelope generation steps.
* **Calculation Formula:**
  ```rust
  Envelope_Period_Sec = 256.0 * Programmed_Value as f64 / 1_000_000.0;
  ```

#### 6. Envelope Shape (Register 13)
The lower 4 bits of Register 13 define the waveform shape used to modulate channel volume:
* `Continue` (Bit 3)
* `Attack` (Bit 2)
* `Alternate` (Bit 1)
* `Hold` (Bit 0)

These four parameters generate 10 unique waveforms:

| Bit 3 (Continue) | Bit 2 (Attack) | Bit 1 (Alternate) | Bit 0 (Hold) | Waveform Behavior |
| :---: | :---: | :---: | :---: | :--- |
| `0` | `0` | `x` | `x` | **Decay** once, then off (volume drops to 0). |
| `0` | `1` | `x` | `x` | **Attack** once, then off (volume rises to 15, drops to 0). |
| `1` | `0` | `0` | `0` | **Repeated Decay** (sawtooth pattern `\ | \ | \ `). |
| `1` | `0` | `0` | `1` | **Decay** once, then hold at 0. |
| `1` | `0` | `1` | `0` | **Repeated Decay/Attack** (triangle pattern `\ / \ / \ `). |
| `1` | `0` | `1` | `1` | **Decay** once, then hold at 15. |
| `1` | `1` | `0` | `0` | **Repeated Attack** (sawtooth pattern `/ | / | / `). |
| `1` | `1` | `0` | `1` | **Attack** once, then hold at 15. |
| `1` | `1` | `1` | `0` | **Repeated Attack/Decay** (triangle pattern `/ \ / \ / `). |
| `1` | `1` | `1` | `1` | **Attack** once, then hold at 0. |

---

### Low-Level I/O Port A & B Logic

To correctly emulate I/O Port operations, you must model the chip's internal bidirectional latches:

#### Internal Latch Registers
Each port contains an internal, un-masked **Output Latch Register** that stores whatever data the CPU writes to Registers 14 and 15, regardless of the active I/O direction specified in Register 7.

#### 1. Input Mode Configuration (Register 7 Direction Bit = 0)
* **Write Behavior:** The written byte is saved in the internal Output Latch Register (unaltered). The physical pins remain high-impedance.
* **Read Behavior:** The internal Output Latch is bypassed. The PSG returns **only** the unlatched, real-time logic levels present on the physical port pins.
* *CPC Hardware Specifics:*
  * **On-Chip Pull-up Resistors:** All physical I/O Port A pins are provided with internal on-chip pull-up resistors. When configured as inputs, any pin that is left unconnected, open, or un-driven by external hardware will natively read back as high (`1`).
  * **Port A (Reg 14):** Connected to the keyboard matrix. Under default input configuration, reading Reg 14 yields the active column state byte. (Closed switches/pressed keys ground the input line, reading `0`, while open switches read `1` due to the internal pull-up).
  * **Port B (Reg 15):** The AY-3-8912 package has no external pins for Port B. Therefore, reading Register 15 in input mode **must always return `&FF`**.

#### 2. Output Mode Configuration (Register 7 Direction Bit = 1)
* **Write Behavior:** The written byte is saved in the internal Output Latch Register. The latch output is driven directly onto the physical pins.
* **Read Behavior:** The PSG outputs the contents of its internal Output Latch, logically **ANDed** with whatever logic levels are actively driven back on the physical pins:
  ```rust
  Read_Value = Internal_Output_Latch & Physical_Pin_States;
  ```
* *CPC Hardware Specifics:*
  * **Port A Warning:** If Port A is reprogrammed to Output mode, its internal latch states are forced onto the keyboard matrix. This conflicts with the column scanner, causing the keyboard to freeze and become entirely unresponsive.

#### Hardware `/RESET` Pin Behavior (Pin 16)
When the active-low physical `/RESET` pin of the PSG is held low (`0`) during a hardware system reset (which requires at least 500 ns during normal operation, or 50 μs on initial power-up), the chip clears all 16 internal registers to `0`. This action completely silences all audio channels, sets the output amplitudes to zero, and disables all tone and noise generation.

---

### Timing-Accurate Assembly Sequence Verification

To guarantee that your emulator's PPI-to-PSG bus arbitration is accurate, test its implementation against the following standard firmware sequences.

#### Register Read (High-Level Simulation Vector)
```assembly
; Select PSG Register 7 (Mixer) and read its current state.
; Input: None. Output: A = Register 7 Value.

ld b,&F4          ; Setup PSG register number 7 on PPI Port A
ld c,7
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 11 (Select Register Mode)
ld c,%11000000
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 00 (Transition to Inactive)
ld c,%00000000
out (c),c

; -- Reconfigure PPI Port A direction --
ld b,&F7          ; PPI Control Register Port
ld a,%10010010    ; Configure Port A as Input
out (c),a

ld b,&F6          ; PPI Port C Bits 7-6 = 01 (Enable PSG Read Mode)
ld c,%01000000
out (c),c

ld b,&F4          ; Read the returned register data from PPI Port A
in a,(c)

; -- Restore PPI Bus State --
ld b,&F7          ; PPI Control Register Port
ld a,%10000010    ; Reconfigure Port A back to Output
out (c),a

ld b,&F6          ; PPI Port C Bits 7-6 = 00 (Return to Inactive Mode)
ld c,%00000000
out (c),c
ret
```

#### Register Write (High-Level Simulation Vector)
```assembly
; Write maximum volume (15) to PSG Register 8 (Channel A Amplitude).
; Input: None. Output: Registers mutated.

ld b,&F4          ; Setup PSG register number 8 on PPI Port A
ld c,8
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 11 (Select Register Mode)
ld c,%11000000
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 00 (Transition to Inactive)
ld c,%00000000
out (c),c

ld b,&F4          ; Setup the data byte (15) on PPI Port A
ld c,15
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 10 (Enable PSG Write Mode)
ld c,%10000000
out (c),c

ld b,&F6          ; PPI Port C Bits 7-6 = 00 (Return to Inactive Mode)
ld c,%00000000
out (c),c
ret
```