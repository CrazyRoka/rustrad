# CRTC Synchronization & Composite Sync (C-SYNC)

The CRTC generates HSYNC and VSYNC signals, but it does not send them directly to the monitor. The Gate Array intercepts both signals, processes them, and generates a single **Composite Sync (C-SYNC)** signal for the monitor.

Understanding the exact timing of these signals—and how they differ per CRTC type—is critical for emulating raster effects, horizontal scrolling, and mid-line mode splits.

---

## HSYNC Generation (R2, R3)

The CRTC HSYNC signal is controlled by Register 2 (position) and Register 3 (width).

* **Start:** When `C0 == R2`, HSYNC goes active (high) and the internal counter `C3l` resets to 0.
* **Duration:** `C3l` increments every µs. When `C3l == R3l`, HSYNC goes inactive (low).
* **R3l = 0:** Type 0/1 produce no HSYNC. Type 2/3/4 treat 0 as 16 µs (maximum).

### Gate Array HSYNC Delay
There is a fundamental architectural delay between the CRTC asserting HSYNC and the Gate Array reacting to it:
* **Type 0, 1, 2:** The CRTC is "ahead" of the Gate Array display. The GA reacts to HSYNC almost immediately (within 1-2 Mode 2 pixels), meaning the black border/hblank starts *before* the character `C0=R2` is fully displayed.
* **Type 3, 4 (ASIC):** The ASIC delays HSYNC by exactly 1 µs to align it with the displayed character. The black border starts exactly on the character boundary of `C0=R2`.

### JIT Techniques (Just-In-Time Updates)
Demos often update `R2` or `R3` on the exact microsecond `C0 == R2` to shrink the horizontal blank area, allowing graphics to be drawn closer to the screen edges.
* **R2.JIT:** Updating `R2` with `OUT (C),r8` exactly when `C0 == R2`. Delays the start of the HSYNC black area by ~0.25 µs (4 Mode 2 pixels). Does not work with `OUTI` (which updates 2 µs too late).
* **R3.JIT:** Updating `R3l` with the current value of `C3l` during HSYNC. Prematurely cuts off the HSYNC signal after exactly 0.25 µs. This is used for fine-pixel horizontal scrolling.

---

## VSYNC Generation (R7, R3)

The CRTC VSYNC signal is controlled by Register 7 (position) and Register 3 (width, upper nibble).

* **Start:** When `C4 == R7` (and `C9 == C0 == 0` on Type 3/4).
* **Duration:** `C3h` increments every line. When `C3h == R3h`, VSYNC goes inactive.
* **R3h = 0:** 16 lines.
* **Type 1, 2:** Ignore `R3h`; VSYNC is fixed at 16 lines.
* **Type 0, 3, 4:** Programmable 1-15 lines.

### Ghost VSYNC (Type 2 Only)
Type 2 evaluates the `C4 == R7` condition on *every* cycle of `C0`. If this condition becomes true while HSYNC is active (i.e., `C0` is between `R2` and `R2+R3`), the CRTC enters a **Ghost VSYNC** state.
* The internal line counter (`C3h`) increments as if VSYNC were occurring, preventing a real VSYNC from triggering later.
* However, the physical VSYNC pin is **not activated** because HSYNC is already active (bus conflict avoidance).
* This breaks monitor synchronization. To avoid it, Type 2 code must ensure `C4 == R7` is never evaluated during HSYNC, either by moving `R7` or shrinking `R3`.

### VSYNC Blocking (Type 0 Only)
Type 0 evaluates VSYNC authorization on `C0 = 2`. If `R0 < 2` on the line preceding `C4 == R7`, the VSYNC is permanently **blocked** for that `C4` value. The only way to unblock it is to change `R7` so `C4 != R7`, or change `R4` so `C4` increments away from `R7`.

### Type 3, 4 VSYNC Quirks
* VSYNC is only evaluated at `C4 == R7 AND C9 == 0 AND C0 == 0`. Writing `R7 = C4` mid-line **does not** trigger a VSYNC.
* **No re-entrancy protection:** If `C4 == R7 == 0` (because `R4 = 0`), an infinite VSYNC loop occurs. Unlike Type 0/1/2, the ASIC lacks the mechanism to ignore `C4 == R7` if the equality hasn't changed.
* The ASIC requires the CRTC VSYNC signal to stay active for at least 3 lines for the Gate Array to generate the monitor C-VSYNC.

---

## Gate Array C-SYNC Algorithm

The Gate Array generates C-SYNC using an **XNOR** logic gate combining internal `SIG_GA_HSYNC` and `SIG_GA_VSYNC` states. It maintains two internal counters: `H06` and `V26`.

### Gate Array Counters
* `H06`: Counts characters during an active HSYNC. Caps C-HSYNC at 4 µs.
* `V26`: Counts HSYNC endings during an active VSYNC. Manages C-VSYNC duration (4 lines) and black border enforcement (26 lines).

### Simplified C-SYNC State Machine
```text
on VSYNC_CRTC rising_edge:
    V26 = 0
    VSYNC_GA = true
    CBLACK_VSYNC = true

on HSYNC_CRTC rising_edge:
    H06 = 0
    CBLACK_HSYNC = true

on HSYNC_CRTC falling_edge:
    SIG_GA_HSYNC = low
    CBLACK_HSYNC = false
    if VSYNC_GA == true:
        V26++
        if V26 == 2: SIG_GA_VSYNC = high  # Start C-VSYNC monitor pulse
        if V26 == 6: SIG_GA_VSYNC = low   # End C-VSYNC monitor pulse
        if V26 == 26:
            CBLACK_VSYNC = false          # End VBL black border
            VSYNC_GA = false

each character_clock:
    H06++
    if H06 == 2: SIG_GA_HSYNC = high      # Start C-HSYNC monitor pulse
    if H06 == 6: SIG_GA_HSYNC = low       # End C-HSYNC monitor pulse

# Final outputs
BLACKCOLOR = CBLACK_HSYNC OR CBLACK_VSYNC
C_SYNC     = SIG_GA_HSYNC XNOR SIG_GA_VSYNC
```

### Monitor Pulse Durations
* **C-HSYNC:** Exactly 4 µs (64 Mode 2 pixels). Starts at `H06=2`, ends at `H06=6`. If `R3l < 6`, the C-HSYNC is truncated by the CRTC's early HSYNC-end signal.
* **C-VSYNC:** Exactly 4 scanlines. Starts at `V26=2` (end of 2nd HSYNC after VSYNC), ends at `V26=6`.
* **VBL Black Border:** 26 scanlines forced black by the Gate Array, regardless of CRTC R3h programming.