### Video Memory Mapping & Offsets

Display generation relies on CRTC hardware counters to determine which RAM location to access for the active pixel stream.

The default video memory is located in the upper 16KB of RAM, spanning `&C000` to `&FFFF`.

#### Line Address Formula
The screen memory is divided into character-sized cells. Each character row consists of 8 vertical scanlines (determined by `R9`). Scanlines within a character row are mapped non-contiguously, with a step offset of `&0800` bytes.

The physical RAM address of a pixel scanline is derived from the base address of the character row, calculated as:

```text
Scanline Address = Base Row Address + (S * &0800)
```

Where `S` represents the scanline index (`0 < S < 7`).

##### Baseline Memory Row Offsets (Characters 1 to 25):
```
Character Row 0:
  Line 0: &C000 | Line 1: &C800 | Line 2: &D000 | Line 3: &D800 
  Line 4: &E000 | Line 5: &E800 | Line 6: &F000 | Line 7: &F800
Character Row 1:
  Line 0: &C050 | Line 1: &C850 | Line 2: &D050 | Line 3: &D850
  Line 4: &E050 | Line 5: &E850 | Line 6: &F050 | Line 7: &F850
```

#### Screen Memory Scaling by Character Cells
To maintain consistent RAM requirements and coordinate with the CRTC, the default display buffer occupies exactly **16,000 bytes** of memory (rounded up to the active 16 KB segment) regardless of the screen mode. This sizing scales directly with character cell widths:
* **Mode 0**: $20 \times 25$ character rows, where each character cell spans 32 bytes ($20 \times 25 \times 32 = 16,000$ bytes).
* **Mode 1**: $40 \times 25$ character rows, where each character cell spans 16 bytes ($40 \times 25 \times 16 = 16,000$ bytes).
* **Mode 2**: $80 \times 25$ character rows, where each character cell spans 8 bytes ($80 \times 25 \times 8 = 16,000$ bytes).

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
    |   |   |   |       |                                  (Bits 0-9)
    |   |   \___/     |-> Video Buffer Size 
    |   |     |       |                 (Bits 10-11)
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

---

#### VMA / VMA' Update Rules by CRTC Type
The CRTC maintains two internal video pointers: `VMA` (the active display pointer) and `VMA'` (the line start pointer). The rules for loading these pointers from R12/R13 differ significantly:

* **Type 0, 3, 4:** `VMA'` and `VMA` are loaded with `R12/R13` exclusively when `C4 = 0` and `C0 = 0` (the start of the frame).
* **Type 1:** `VMA` is loaded with `R12/R13` whenever `C4 = 0` and `C0 = 0`, regardless of `C9`. This allows software to change the screen offset on *any* scanline within the first character row (`C4=0`) without requiring complex raster splits.
* **Type 2:** `VMA'` is loaded with `R12/R13` on the last line of the frame when `C0` reaches `R1`. `VMA` is then loaded from `VMA'` at the start of the next line (`C0 = 0`). If `R12/R13` is modified after `C0` exceeds `R1` on the last line, the change is ignored until the next frame.

> **Note:** The rules above are simplified. Type 1 has a unique behavior where
> `VMA ← R12/R13` directly (not via VMA') whenever `C4==0`, on every line of
> the first character row. Type 2 has a partial-logic bug when `R1=0` on the
> last line. See [CRTC Internal Counters](crtc_counters.md#vma--vma--video-memory-address-pointers)
> for full detail.


---

### Overscan Bits and Video Pointer Counter Carry

The internal 14-bit VMA address counter increments sequentially to fetch the active video data. While bits 11, 12, and 13 of the final 16-bit physical RAM address are substituted with the row index `C9`, the internal 14-bit counter still carries overflows logically:

* **Overscan Page-Switching Carry:** When the lower 10 bits of VMA (`MA0` to `MA9`) overflow past their limit, the carry updates bits 10 and 11 of the 14-bit VMA. If these carry bits both reach `1`, they report directly into bits 12 and 13 of the starting address (which map to bits 14 and 15 of the physical memory map). 
* **Implications:** This allows games and demo routines to display a continuous screen buffer that exceeds the standard 16 KB page boundary (automatically switching from `&FFFF` to `&0000` or `&4000` depending on configuration) without requiring software splits or manual mid-frame register updates.