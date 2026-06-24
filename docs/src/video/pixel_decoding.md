### Pixel Decoding & Video Modes

The Amstrad Gate Array translates RAM byte values into pixel color configurations depending on the active display mode. The mapping from RAM bits to the logical pen indices is highly non-linear.

#### Mode 2 (High Resolution)
* **Resolution:** $640 \times 200$
* **Color Depth:** 2 Colors (Pen 0–1)
* **Pixel Encoding:** 1 byte represents 8 pixels ($p0$ to $p7$). The pixels are arranged sequentially from left ($p0$) to right ($p7$). Each bit corresponds directly to a binary Pen index.

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| $p0$  | $p1$  | $p2$  | $p3$  | $p4$  | $p5$  | $p6$  | $p7$  |

---

#### Mode 1 (Medium Resolution)
* **Resolution:** $320 \times 200$
* **Color Depth:** 4 Colors (Pen 0–3)
* **Pixel Encoding:** 1 byte represents 4 pixels ($p0$ to $p3$). Each pixel pen index is calculated from a split pair of bits:
  * High-order bits $[7..4]$ contain bit 1 of the pen values.
  * Low-order bits $[3..0]$ contain bit 0 of the pen values.

$$\text{Pen Index for Pixel } x = (\text{Bit}[x+4] \ll 1) \mid \text{Bit}[x]$$

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| $p0(1)$| $p1(1)$| $p2(1)$| $p3(1)$| $p0(0)$| $p1(0)$| $p2(0)$| $p3(0)$|

* **Example calculation:** If byte is `&10` (binary `00010000`):
  * $p0 = (\text{Bit } 7 \ll 1) \mid \text{Bit } 3 = (0 \ll 1) \mid 0 = 0$
  * $p1 = (\text{Bit } 6 \ll 1) \mid \text{Bit } 2 = (0 \ll 1) \mid 0 = 0$
  * $p2 = (\text{Bit } 5 \ll 1) \mid \text{Bit } 1 = (0 \ll 1) \mid 0 = 0$
  * $p3 = (\text{Bit } 4 \ll 1) \mid \text{Bit } 0 = (1 \ll 1) \mid 0 = 2$

---

#### Mode 0 (Low Resolution / Double Width)
* **Resolution:** $160 \times 200$
* **Color Depth:** 16 Colors (Pen 0–15)
* **Pixel Encoding:** 1 byte represents 2 pixels ($p0$ and $p1$). The 4-bit Pen index for each pixel is interleaved across the byte structure:

$$\text{Pen Index for Pixel } 0 = (\text{Bit } 7 \times 1) + (\text{Bit } 3 \times 2) + (\text{Bit } 5 \times 4) + (\text{Bit } 1 \times 8)$$

$$\text{Pen Index for Pixel } 1 = (\text{Bit } 6 \times 1) + (\text{Bit } 2 \times 2) + (\text{Bit } 4 \times 4) + (\text{Bit } 0 \times 8)$$

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| $p0(0)$| $p1(0)$| $p0(2)$| $p1(2)$| $p0(1)$| $p1(1)$| $p0(3)$| $p1(3)$|

---
