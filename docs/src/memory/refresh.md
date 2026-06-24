### Dynamic RAM Refresh Mechanics

Dynamic RAM (DRAM) requires periodic row accesses to prevent data loss due to leakage currents in the internal microcondensers.

#### Hardware Architecture (Base Model)
The standard CPC uses 4-bit wide, 16k-bit depth DRAM ICs configured as a square matrix of 128 x 128 cells. There are 128 rows that must be refreshed within their maximum charge retention window (typically several milliseconds).

#### Z80 Refresh (R) Register Role
During the instruction decode phase (second half of an instruction fetch cycle `M1`), the Z80 places the 7-bit contents of its internal `R` register onto the lower 7 bits of the Address Bus (`A0`–`A6`) and asserts the `/RFSH` pin low.
* **Halt Behavior:** During a `HALT` instruction, the Z80 continues to generate virtual `NOP` fetch-refresh cycles to maintain row scanning.

#### CRTC Backup Refresh Erratum
In the CPC design, the CRTC acts as a continuous backup memory refresher for the lower 64KB RAM bank. It generates active memory addresses on the bus during VDU display and during horizontal/vertical retrace phases.

##### Memory Destruction Bug (VDU Disabling)
If an emulator-relevant program disables the display by altering the CRTC registers:
```assembly
OUT &BC00, 1  ; Select CRTC R1 (Horizontal Displayed)
OUT &BD00, 0  ; Set Horizontal Displayed to 0
```
This halts the CRTC's address scanning. If the Z80's `R` register is concurrently modified or not cycling normally, rows in the upper memory banks will fail to refresh, resulting in data corruption on physical hardware.
* **Emulator Note:** Standard emulators do not emulate charge decay. However, for complete hardware verification, decay timers or warning states can be maintained for memory blocks if they are not swept by either `/RFSH` or CRTC bus cycles.