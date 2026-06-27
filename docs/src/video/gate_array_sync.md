
# Gate Array Pixel Cooking & Mode Splits

The Gate Array (GA) is responsible for decoding video memory bytes into pixels. When the video mode is changed mid-line (a "mode split"), the GA is often caught halfway through processing a byte. The resulting distorted pixels are called **"cooked pixels"**.

Cycle-accurate emulators must model the GA's internal shift registers and bit-latching logic to replicate these artifacts, which are heavily used in demoscene productions to mix graphics modes on a single scanline.

---

## Mode 2 Pixel Advance

There is a fundamental timing discrepancy in how the Gate Array processes Mode 2 (640x200, 2 colors) vs Modes 0, 1, and 3.

* **GA 40007, 40008, 40010, and Pre-ASIC 40226 (Type 4):** Mode 2 pixels are processed **1 Mode 2 pixel (0.0625 µs)** earlier than Modes 0/1/3.
* **ASIC 40489 (Type 3 / CPC+):** Mode 2 is perfectly aligned with the other modes.

**Emulator Impact:** When splitting from Mode 0 to Mode 2 mid-line, the border will stop 1 pixel earlier, and the Mode 2 pixels will start 1 pixel earlier on older GAs.

---

## HSYNC Requirements for Mode Splits

To change the video mode mid-line, the GA must detect an HSYNC signal of at least 2 µs (`R3 >= 2`).
* The mode change is applied on the **3rd microsecond** of the `OUT (C),r8` instruction to the GA.
* If `R3 < 2`, the GA does not process the HSYNC, and the mode change will not take effect until the next valid HSYNC.

---

## Pixel Cooking Tables

When the display is re-enabled after an HSYNC (or R3.JIT), the GA is in the middle of shifting a byte. The bits it has already shifted out are "lost". To calculate the new pixels, the GA reuses the remaining bits, combined with either `0` or `1` for the lost bits.

* **GA 40010:** Lost bits are treated as **`0`**.
* **GA 40007 / 40008:** Lost bits are treated as **`1`**.
* **Pre-ASIC 40226 (CRTC 4):** Has a unique bit-reuse alignment, reusing bits from the 7th VRAM byte.

### Example: Mode 2 to Mode 0 (GA 40010)
If the GA was in Mode 2 and had already shifted out bits `b7, b6, b5` before the mode switched to Mode 0:
* The missing 3 bits are replaced with `0`.
* The new Mode 0 pixels are formed using the remaining bits `b4, b3, b2, b1, b0` combined with the `0`s.
* Resulting pixels use only a subset of the 16-color palette (Colors 0, 1, 4, 5).

### Example: Mode 0 to Mode 1 (All GA Types)
If the GA was in Mode 0 and had shifted out the first pixel (`b7, b3`), the remaining bits form a hybrid pixel.
* The last Mode 1 pixel's color is directly tied to the bits of the first Mode 0 pixel.
* E.g., If the Mode 0 pixel was color 4 (`0100`), the resulting Mode 1 pixel will be color 0.

---

## Emulating the Shift Register

To accurately emulate pixel cooking, the Gate Array emulation should not simply decode bytes to pixels per-cycle. Instead, it must maintain an 8-bit shift register and a bit-position counter.

1. **During Display:** Every 0.0625 µs (1 Mode 2 pixel), shift the register by 1 bit.
2. **During HSYNC Blackout:** Stop shifting, but keep the register and bit position intact.
3. **On Mode Change:** Recalculate the current pixel using the new mode's decoding matrix, substituting `0` or `1` for bits at indices lower than the current bit-position counter.
4. **Byte Boundary:** When the bit-position counter wraps to 0, fetch the next VRAM byte and resume normal decoding.