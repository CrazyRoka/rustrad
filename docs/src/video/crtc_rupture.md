
# CRTC Ruptures & Splitscreen Techniques

"Rupture" is the French demoscene term for dynamically modifying CRTC registers mid-frame to alter the video memory pointer (offset), create splitscreens, or achieve effects like line-to-line scrolling.

Because the CRTC types have different internal counter rules, ruptures that work flawlessly on a Type 1 often fail on a Type 0 or Type 2. This page documents the standard emulation requirements for these techniques.

---

## Line-to-Line Rupture (RLAL)

**Goal:** Force `C4 = 0` and `C9 = 0` on *every* scanline, so `R12/R13` can be reloaded every 64 µs to display a completely different screen buffer line.

### Type 1, 3, 4 (Trivial)
Simply set `R4 = 0` and `R9 = 0` when `C4 = 0` and `C9 = 0`.
* Type 1: VMA is reloaded from `R12/R13` whenever `C4 == 0`, so the offset changes instantly.
* Type 3, 4: `C9 > R9` comparison forces `C9` to 0 immediately.

### Type 0 (RVLL - Rupture Verticale Last Line)
Type 0 evaluates the "Last Line" condition (`C4 == R4 && C9 == R9`) only when `C0 < 2`.
* To get `C4 = 0` on every line, you must program `R9 = 0` on the *previous* line when `C0 < 2`.
* You must also avoid `R0 < 2`, otherwise `C9` freezes.
* Hidden 2-µs ruptures (`R0 = 1`) are created during HSYNC to advance `C9` to the desired value for the visible line.

### Type 2 (The HSYNC Trick)
Type 2 arms the "Last Line" reset, but once armed, it **cannot be cancelled**. Furthermore, it will not arm on a first line (`C4=0, C9=0`) unless a specific trick is used:
* During HSYNC, the `LastLineMgmt` state can be re-authorized if `C4 != R4` or `C9 != R9` is evaluated at the last character of HSYNC (`C0 = R2+R3-1`).
* The standard RLAL code for Type 2 modifies `R9` during HSYNC to break the `C9 == R9` equality, re-arming the LastLine management, then restores `R9 = 0` before `C0 = 0` to satisfy the LastLine condition.

---

## Rupture Verticale Invisible (RVI)

**Goal:** Change the video memory offset mid-line without visible HSYNC artifacts, by creating "hidden" 1-µs or 2-µs frames during the horizontal border.

* **Type 0:** Requires `R0 >= 2`. A 2-µs frame (`R0 = 1`) generates a mandatory additional adjustment line (`C4 = 1`), meaning it takes 4 µs to reload `R12/R13` instead of 2 µs.
* **Type 1, 3, 4:** `R0 = 0` works perfectly. 14 hidden ruptures of 1 µs can fit in the 16 µs horizontal border, allowing access to any `C9` value.
* **Type 2:** Impossible to perform a classic RVI because `C0` must reach `R1` on the last line for `R12/R13` to be considered. Setting `R1 = 1` when `R0 = 1` activates the border immediately.

---

## Rupture for Dummies (RFD) - Type 1 Only

RFD is a famous bug in the UM6845R (Type 1) CRTC that allows per-scanline offset changes without complex RLAL timing.

### Triggering the Bug
1. Ensure `R5 = 0`.
2. On a line where `C9 != R9`, write `R5 = 1` exactly at `C0 == R0`.
3. Immediately write `R5 = 0` (optional, prevents actual adjustment lines).

### Effect
This toggles an internal state (`RFD_VMA_From_R12R13 = true`). Normally, `VMA` is only loaded from `R12/R13` when `C4 == 0`. With RFD active, `VMA` is loaded from `R12/R13` at **every `C0 = 0`**, regardless of `C4`. 

### The Parity Problem
RFD also activates parity management in the `C9 == R9` test. This causes a stroboscopic effect: on odd frames, `VMA'` fails to update (lines repeat); on even frames, it works normally.

### The Fix: IVM ON/OFF
To freeze the parity to "Even" and make RFD work on every frame:
1. Execute `OUT R8, 3` (Enable Interlace Video Mode).
2. Execute `OUT R8, 0` (Disable Interlace Video Mode).
3. This must be done on an **even `C9`** line. 

Once parity is frozen, the programmer can simply update `R12/R13` on any scanline, and the CRTC will display the new address on the very next line, perfectly mimicking a Type 0 RLAL but with trivial code.