
# CRTC Interlace Modes & Parity

The CRTC supports two interlace modes via Register 8 (`R8`):
* **Interlace Sync (IS):** `R8 = 1`. Displays the same frame twice, offset by half a scanline, to reduce flicker.
* **Interlace Sync & Video (IVM):** `R8 = 3`. Displays even scanlines on even frames, odd scanlines on odd frames, doubling vertical resolution to 625 lines.

**Critical CPC Architecture Note:** The Gate Array suppresses the half-line VSYNC timing required for true interlace. It waits for the 2nd HSYNC after VSYNC to trigger C-VSYNC, meaning the monitor beam always returns to the exact same vertical position. To achieve true interlace on a CPC, software must manually delay the HSYNC or VSYNC.

---

## Register Programming in IVM Mode

In IVM mode, a character row spans two frames. Therefore, `R4` and `R7` must be doubled (minus 1), and `R9` programming differs by CRTC type.

| CRTC Type | `R9` Formula (N = lines per char row) | Notes |
|-----------|--------------------------------------|-------|
| **0, 3, 4** | `R9 = N - 2` | If N=8, `R9=6`. C9 bit 0 is used for parity. |
| **1** | `R9 = N - 1` | If N=8, `R9=7`. C9 bit 0 is managed via C4 parity. |
| **2** | `R9 = N - 1` | If N=8, `R9=7`. Uses a separate `C9.IVM` counter for display. |

---

## Parity States

Interlace relies on frame parity (Even/Odd) toggling every frame.

* **Type 0, 2:** Parity is anticipated using `ParityR6`. When `C4 == R6`, `ParityR6` toggles. At frame start, `ParityFrame = ParityR6`. If `R6 > R4`, parity freezes.
* **Type 1, 3, 4:** `ParityFrame` simply toggles at `C4 = C9 = C0 = 0`.

`ParityC9` determines if the current scanline is even or odd:
* Type 0, 3, 4: `ParityC9 = C4.0 XOR ParityFrame` (only if `R9` is odd).
* Type 1: `ParityC9` toggles on every C4 if `R9` is even.

---

## IVM Counting Algorithms

### Type 0, 3, 4 (The Shift Method)
When `R8 = 3`, the CRTC increments `C9` normally, but for the video address (`C9.VMA`), it shifts `C9` left by 1 and ORs the parity bit:
```text
C9.VMA = (C9 << 1) | ParityC9
```
The comparison to reset `C9` becomes: `(C9 << 1) | ParityFrame == (R9 | ParityFrame)`.

### Type 1 (The Increment Method)
Type 1 increments `C9` by 1 (if `R9` is odd) or 2 (if `R9` is even) every line:
```text
C9 = C9 + 1 + (R9 & 1)
```
If `R9` is odd (even number of lines), `ParityC9` toggles every `C4` increment.

### Type 2 (The Dual Counter Method)
Type 2 maintains a completely separate counter, `C9.IVM`, for video addressing.
* `C9` continues to count normally against `R9` to manage `C4` increments.
* `C9.IVM` counts by 2, resetting when `(C9.IVM & 0x1E) == (R9 & 0x1E)`.
* `C9.IVM` is used for the video pointer and VMA' updates.

---

## Mid-VSYNC

In interlace modes, on an **even frame**, the VSYNC is delayed to the middle of the line (`C0 = R0/2`).
* This is evaluated when `C4 == R7` and `ParityFrame == Even`.
* **Type 3, 4 Exception:** If `R7 = 0`, VSYNC priority takes over parity. If parity was odd, no Mid-VSYNC occurs. If parity was even, Mid-VSYNC occurs even though parity flips to odd.

---

## Additional Interlace Line

To interlace two 312-line frames into a 625-line image, an extra line must be added to the even frame.
* The condition is evaluated on the last line of the frame.
* **Type 0, 2:** The extra line is tied to `ParityR6`. If `R6 > R4`, the extra line is generated on *every* frame, breaking interlace.
* **Type 1, 3, 4:** The extra line is tied to `ParityFrame == Even`.
* **Type 3, 4 Quirk:** `C4` does **not** increment during the additional interlace line (unlike R5 adjustment lines). The video pointer updates without `C4` advancing.