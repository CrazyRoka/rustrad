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

* **Example calculation:** If byte is `&10` (binary `00010000`):
  * `p0 = (Bit 7 << 1) OR Bit 3 = (0 << 1) OR 0 = 0`
  * `p1 = (Bit 6 << 1) OR Bit 2 = (0 << 1) OR 0 = 0`
  * `p2 = (Bit 5 << 1) OR Bit 1 = (0 << 1) OR 0 = 0`
  * `p3 = (Bit 4 << 1) OR Bit 0 = (1 << 1) OR 0 = 2`

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