### Keyboard & Joystick Matrix

The Amstrad CPC keyboard, alongside connected joysticks, is configured as a physical 10 x 8 switch matrix. Reading keystrokes requires coordinated control of both the Intel 8255 PPI and the AY-3-8912 PSG.

---

### Hardware Matrix Layout

The matrix is structured as 10 selectable lines (columns `0..9`), with each line returning an 8-bit status byte (bits `0..7`). 
* **Logic States:** A bit value of `0` denotes a closed switch (key/button **pressed**). A bit value of `1` denotes an open switch (key/button **not pressed**).

| Bit | Line 0 | Line 1 | Line 2 | Line 3 | Line 4 | Line 5 | Line 6 | Line 7 | Line 8 | Line 9 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **7** | `f.` | `f0` | `Ctrl` | `> ,` | `< .` | `Space` | `V` | `X` | `Z` | `Del` |
| **6** | `Enter` | `f2` | `` ` \ `` | `? /` | `M` | `N` | `B` | `C` | `Caps Lock` | *Unused* |
| **5** | `f3` | `f1` | `Shift` | `* :` | `K` | `J` | `F` / `J1 F1` | `D` | `A` | `J0 F1` |
| **4** | `f6` | `f5` | `f4` | `+ ;` | `L` | `H` | `G` / `J1 F2` | `S` | `Tab` | `J0 F2` |
| **3** | `f9` | `f8` | `} ]` | `P` | `I` | `Y` | `T` / `J1 R` | `W` | `Q` | `J0 R` |
| **2** | `Cur Dn` | `f7` | `Return` | `\| @` | `O` | `U` | `R` / `J1 L` | `E` | `Esc` | `J0 L` |
| **1** | `Cur R` | `Copy` | `{ [` | `= -` | `) 9` | `' 7` | `% 5` / `J1 Dn`| `# 3` | `" 2` | `J0 Dn` |
| **0** | `Cur Up` | `Cur L` | `Clr` | `£ ^` | `_ 0` | `( 8` | `& 6` / `J1 Up`| `$ 4` | `! 1` | `J0 Up` |

#### Key & Joystick Mapping Rules
* **Keypad vs Mainboard:** Keys prefixed with `f` (e.g., `f0`–`f9`, `f.`) are located on the dedicated numeric keypad.
* **Return vs Enter:** `RETURN` (Line 2, Bit 2) maps to the primary keyboard carriage return. `ENTER` (Line 0, Bit 6) maps to the smaller numeric keypad enter key.
* **Joystick 1 Routing (Shared):** Joystick 1 (`J1`) directions and fire buttons are hardwired in parallel with the standard keyboard keys on Line 6 (Bits 0–5). Pressing these keyboard keys generates inputs identical to Joystick 1 movements.
* **Joystick 0 Routing (Isolated):** Joystick 0 (`J0`) directions and fire buttons occupy their own dedicated column on Line 9 (Bits 0–5).
* **Third Fire Button / Mouse Logic:** Bit 6 on Line 6 and Line 9 can optionally be used by software to read a third joystick fire button or the middle button of an AMX-compatible mouse.
* **Lines 11–14 Return:** If an address selects any matrix line in the range 11 to 14, the read buffer hardware must always return `&FF`.
<!-- TODO: Verify default return for Matrix Line 10 -->
<!-- TODO: Verify default return for Matrix Line 15 -->

---

### Low-Level Register Scanning Algorithm

Because the matrix lines are connected to **PSG Port A (Register 14)** and selection is driven by **PPI Port C (Bits 3..0)**, an emulator must execute the following multi-step bus state transition sequence to read a matrix column:

```
  +------------------+       1. Write &0E (Reg 14)       +--------------------+
  |   PPI Port A     | ================================> |     PSG Bus        |
  |  (Data Buffer)   | <================================ | (Latch Register 14)|
  +------------------+       7. Read Column Data         +--------------------+
           ||                                                      ||
           || 5. Write Column Select (Bits 3-0)                    ||
           \/                                                      ||
  +------------------+                                             ||
  |   PPI Port C     | --------------------------------------------+
  |  (Select Lines)  | 2. Select PSG Reg (Bits 7-6 = 11)
  +------------------+ 3. Inactive Phase (Bits 7-6 = 00)
                       6. PSG Read Mode (Bits 7-6 = 01)
```

1. **Latch PSG Register 14:**
   * Write `14` (`&0E`) to PPI Port A (the target PSG register index for I/O Port A).
   * Set PPI Port C Bits 7–6 to `11` (Select Register function).
2. **Transition to Inactive:**
   * Set PPI Port C Bits 7–6 to `00` (Inactive state). 
   * *ASIC Verification Note:* This step is physically mandatory. Failing to cycle through the `00` inactive bus state before switching directions will cause register write failures on CPC+ hardware.
3. **Change PPI Buffer Direction:**
   * Configure PPI Port A to **Input** mode by writing to the PPI Control Register (`&F7xx`).
4. **Assert Column Line:**
   * Write the target matrix line index (0 to 15) to the lower nibble of PPI Port C (Bits 3..0). This routes the selected physical column lines to the inputs of PSG Port A.
5. **Enable PSG Data Read:**
   * Set PPI Port C Bits 7–6 to `01` (Read Register function). This instructs the PSG to output the latched register data (Register 14 matrix line byte) back onto PPI Port A.
6. **Capture Matrix Byte:**
   * Read the active key state byte from PPI Port A.
7. **Re-initialize Bus State (If complete):**
   * If no further columns are being read immediately, set PPI Port A back to **Output** mode using the PPI Control Register.
   * Set PPI Port C Bits 7–6 to `00` (Inactive state).

---

### Hardware Keyboard Clash (Ghosting)

The physical switch matrix lacks decoupling diodes at each key intersection. Consequently, if multiple keys are closed simultaneously, current can flow backward through neighboring lines, generating "ghost" keypresses.

#### Emulation Logic
Your emulation layer can simulate this current leakage path by checking for closed loops in the matrix. If we represent the keyboard states as a 2D matrix array, the ghosting behavior is defined as:

```rust
// Keyboard Ghosting Simulation Rule
// 0 = Pressed (Closed Switch), 1 = Released (Open Switch)
if matrix[line_x1][bit_y1] == 0 && 
   matrix[line_x2][bit_y1] == 0 && 
   matrix[line_x2][bit_y2] == 0 
{
    matrix[line_x1][bit_y2] = 0; // Ghost key is forced active
}
```

*Example Demonstration:* If a user presses `C` (Line 7, Bit 6), `W` (Line 7, Bit 3), and `N` (Line 5, Bit 6) at the same time:
* `line_x1 = 5`, `line_x2 = 7`
* `bit_y1 = 6`, `bit_y2 = 3`
* Because `matrix[5][6]` (`N`), `matrix[7][6]` (`C`), and `matrix[7][3]` (`W`) are all active (`0`), the logic forces `matrix[5][3]` (`Y`) to `0` as well. The system reports that the `Y` key is pressed even though it is physically untouched.

---

### Operating System Key Manager Reference

When designing High-Level Emulation (HLE) layers or tracking OS states during debugging, the firmware's keyboard supervisor operates under the following behavioral parameters:

* **Sampling Period:** The Key Manager scans the keyboard matrix during the system's vertical sync interrupt loop, executing exactly once every **1/50th of a second** (20ms).
* **Debouncing Logic:** To filter transient switch bounce, a key state transition (press or release) is only committed to the internal RAM bitmap if it retains its new state for **two consecutive scan cycles** (40ms).
* **Hardwired System Resets:** During the matrix scan, if the keyboard supervisor detects that `Ctrl` (Line 2, Bit 7), `Shift` (Line 2, Bit 5), and `Esc` (Line 8, Bit 2) are pressed simultaneously, it bypasses standard event loops and executes a hardware vector jump directly to **`RST 0`** (`&0000`).