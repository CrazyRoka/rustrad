### Video Memory Mapping & Offsets

Display generation relies on CRTC hardware counters to determine which RAM location to access for the active pixel stream.

The default video memory is located in the upper 16KB of RAM, spanning `&C000` to `&FFFF`.

#### Line Address Formula
The screen memory is divided into character-sized cells. Each character row consists of 8 vertical scanlines (determined by `R9`). Scanlines within a character row are mapped non-contiguously, with a step offset of `&0800` bytes.

The physical RAM address of a pixel scanline is derived from the base address of the character row, calculated as:

$$\text{Scanline Address} = \text{Base Row Address} + (S \times \&0800)$$

Where $S$ represents the scanline index ($0 \le S \le 7$).

##### Baseline Memory Row Offsets (Characters 1 to 25):
```
Character Row 0:
  Line 0: &C000 | Line 1: &C800 | Line 2: &D000 | Line 3: &D800 
  Line 4: &E000 | Line 5: &E800 | Line 6: &F000 | Line 7: &F800
Character Row 1:
  Line 0: &C050 | Line 1: &C850 | Line 2: &D050 | Line 3: &D850
  Line 4: &E050 | Line 5: &E850 | Line 6: &F050 | Line 7: &F850
```

#### Hardware Scrolling and Wrap-Around
Modifying the screen offset shifts all row address calculations. The calculation must account for the following offset parameters:
* **Scroll Left:** Add `+&02` to the offset register per unit shift.
* **Scroll Right:** Subtract `-&02` from the offset register per unit shift.
* **Scroll Up:** Add `+&50` (decimal 80) per line shift.
* **Scroll Down:** Subtract `-&50` (decimal 80) per line shift.

##### Address Wrap-Around
* **Lower Bound:** `&C000`
* **Upper Bound:** `&FFFF`
When horizontal or vertical scrolling causes the row calculation to exceed `&FFFF`, the pointer must wrap back to `&C000` within the active 16KB screen space.

#### Display Start Address registers (CRTC R12 & R13)
CRTC Registers 12 (High) and 13 (Low) form a 16-bit register pair that dictates the base address of the screen display buffer. 

Your memory mapping logic must decode the bits of this register pair as follows:

```
        CRTC REGISTER 12 (High)             CRTC REGISTER 13 (Low)
   15  14  13  12  11  10  09  08       07  06  05  04  03  02  01  00
  +---+---+---+---+---+---+---+---+    +---+---+---+---+---+---+---+---+
  | X | X | Page  |  Size |  Offset (High) |   Offset (Low)            |
  +---+---+---+---+---+---+---+---+    +---+---+---+---+---+---+---+---+
    |   |   |   |   |   |   \________________________________________/
    |   |   |   |   |   |                        |
    |   |   |   |   \___/                        +-> Base Offset
    |   |   \___/     |                                  (Bits 0-9)
    |   |     |       +-> Video Buffer Size 
    |   |     |                 (Bits 10-11)
    |   |     +---------> Video Page Selector
    |   |                       (Bits 12-13)
    \___/
      +-> Ignored bits (X)
```

##### 1. Video Page Selector (Bits 13–12)
Selects the base 16 KB RAM boundary for the display buffer:
* **`00`**: `&0000 - &3FFF`
* **`01`**: `&4000 - &7FFF`
* **`10`**: `&8000 - &BFFF`
* **`11`**: `&C000 - &FFFF` *(Default configuration)*

##### 2. Video Buffer Size (Bits 11–10)
Defines the depth of the active display buffer page:
* **`00`**: 16 KB
* **`01`**: 16 KB
* **`10`**: 16 KB
* **`11`**: 32 KB

##### 3. Screen Offset (Bits 9–0)
Specifies the starting memory offset inside the selected bank. This offset value is added directly to screen line addresses, modulo `&0800`.

