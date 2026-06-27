### Pixel Decoding & Video Modes

The Amstrad Gate Array translates RAM byte values into pixel color configurations depending on the active display mode. The mapping from RAM bits to the logical pen indices is highly non-linear.

#### Mode 2 (High Resolution)
* **Resolution:** 640 x 200
* **Color Depth:** 2 Colors (Pen 0–1)
* **Pixel Encoding:** 1 byte represents 8 pixels (`p0` to `p7`). The pixels are arranged sequentially from left (`p0`) to right (`p7`). Each bit corresponds directly to a binary Pen index.

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| p0    | p1    | p2    | p3    | p4    | p5    | p6    | p7    |

---

#### Mode 1 (Medium Resolution)
* **Resolution:** 320 x 200
* **Color Depth:** 4 Colors (Pen 0–3)
* **Pixel Encoding:** 1 byte represents 4 pixels (`p0` to `p3`). Each pixel pen index is calculated from a split pair of bits:
  * High-order bits `[7..4]` contain bit 1 of the pen values.
  * Low-order bits `[3..0]` contain bit 0 of the pen values.

```text
Pen Index for Pixel x = (Bit[x+4] << 1) OR Bit[x]
```

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| p0(1) | p1(1) | p2(1) | p3(1) | p0(0) | p1(0) | p2(0) | p3(0) |

---

#### Mode 0 (Low Resolution / Double Width)
* **Resolution:** 160 x 200
* **Color Depth:** 16 Colors (Pen 0–15)
* **Pixel Encoding:** 1 byte represents 2 pixels (`p0` and `p1`). The 4-bit Pen index for each pixel is interleaved across the byte structure:

```text
Pen Index for Pixel 0 = (Bit 7 * 1) + (Bit 3 * 2) + (Bit 5 * 4) + (Bit 1 * 8)
Pen Index for Pixel 1 = (Bit 6 * 1) + (Bit 2 * 2) + (Bit 4 * 4) + (Bit 0 * 8)
```

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| p0(0) | p1(0) | p0(2) | p1(2) | p0(1) | p1(1) | p0(3) | p1(3) |

---

#### Video Mode 3 (Unofficial Mode)
When bits 1 and 0 of the Gate Array's Mode and ROM Configuration register (RMR) are set to `11`, the display enters **Mode 3**:

* **Physical Characteristics:** Mode 3 displays at a horizontal resolution of 160 pixels (matching Mode 0 dimensions), but restricts the color selection to a maximum of **4 active pens** (limited to Pens 0–3).
* **Emulator Note:** To emulate Mode 3 correctly, map pixel bits using the physical layout of Mode 0, but mask or ignore pen selections that fall outside the Pen 0–3 boundary.

---

### Pixel Alignment & Graphics Mode Splitting

When the graphics mode is dynamically changed during a line, specific hardware quirks occur:

#### Mode 2 Early Display Offset
* **The Quirk:** On standard CPCs equipped with a **Gate Array 40007, 40008, 40010** or the **Pre-ASIC 40226 (CRTC Type 4)**, the display of pixels in **Mode 2** is processed exactly **1/16 μs (0.0625 μs, or 1 Mode 2 Pixel-M2)** earlier than for Mode 0, 1, or 3. 
* **The Exception:** The CPC+ **ASIC 40489 (CRTC Type 3)** does not exhibit this offset. Mode 2 is aligned with the other graphics modes.
* **Border Impact:** Changing to Mode 2 causes the active Border display to terminate 1 Pixel-M2 earlier at the left, and commence 1 Pixel-M2 earlier at the right side of the screen.

#### Mid-Byte Mode Switch Cooking
When a mode change is written to the Gate Array, the change takes effect instantly on the 3rd microsecond of the `OUT` instruction. Since the Gate Array may be in the middle of processing a byte from VRAM, the internal bit-shift registers and latching logic are immediately converted to the new mode parameters:
* **GA 40010:** Any raw data bits that have already been shifted out for the previous mode's pixels are assumed to be `0` when calculating the remaining pixel pen values in the new mode.
* **GA 40007 / 40008:** Any raw data bits that have already been shifted out are treated as `1` in the new mode's pen calculation.
* **Pre-ASIC 40226 (CRTC Type 4):** Exhibits its own distinct pixel cooking behavior. Display stops for exactly 32 Pixel-M2 (2 µs), and the shift register reuses bits similarly to the 40010, but with different alignment offsets based on the 2nd Pixel-M2 of the 7th VRAM byte.
* **Implications:** This "pixel cooking" distorts the remaining pixels of the transitioning byte, combining properties of both the old and new mode. Cycle-accurate emulators must track the exact sub-microsecond pixel clock boundary where the mode register is modified to correctly mix these transition pixels.