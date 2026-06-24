### Video Memory Mapping & Offsets

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

