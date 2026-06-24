### Sprite Handling (Software vs. CPC+ Hardware)

The base Amstrad CPC 464 lacks any hardware-accelerated sprite generation, placing the entire burden of graphics scaling, masking, and collision calculation onto the software layer.

#### Software Sprite Rendering Methods
Emulators executing software routines will encounter three major software rendering techniques:

##### 1. Pixel Replacement (Opaque Rendering)
The byte data representing the sprite is directly copied to screen memory, entirely overwriting the background. This is highly efficient but does not support transparency.

##### 2. Bitwise XOR Masking
The sprite data is combined with the existing screen RAM using a bitwise `XOR` operation:
```text
Screen Byte <- Screen Byte XOR Sprite Byte
```
* **Advantages:** Avoids the need to store separate masking data; drawing the sprite a second time at the identical coordinates perfectly restores the original background.
* **Disadvantages:** Colors shift dynamically depending on the background pixels beneath the sprite, causing visual artifacts.

##### 3. Bitwise AND-OR Masking (Transparency)
To achieve true transparency without color shifts, the sprite must be stored with associated mask bytes:
1. The screen RAM byte is read.
2. The screen RAM is `AND`ed with the mask byte to "carve out" the transparent area (clearing pixels slated for replacement).
3. The result is `OR`ed with the sprite's active color data.
4. The final mixed byte is written back to screen RAM.

```text
Screen Byte <- (Screen Byte AND Mask) OR Sprite Data
```

#### Pre-Shifted Sprites
Because screen memory is byte-addressed, moving a software sprite horizontally by a single pixel requires complex bit-shifting operations on the fly. To avoid this CPU bottleneck, software developers often duplicate the sprite graphics in memory at different pixel-offset alignments (2 offsets in Mode 0, 4 offsets in Mode 1, 8 offsets in Mode 2). The program then renders the pre-shifted image that matches the coordinate's pixel remainder.

---

### Delta: CPC+ Hardware Sprites

The CPC+ ASIC introduces a true hardware sprite system, completely bypassing standard CPU software raster routines.

* **Sprite Limit:** 16 independent hardware sprites.
* **Dimensions:** Fixed at $16 \times 16$ pixels per sprite.
* **Color Depth:** 16 Colors, mapped to a dedicated sprite color palette completely independent of the main screen's Gate Array palette.
* **Magnification:** Independent horizontal and vertical scaling factors (`1x`, `2x`, `4x`) can be applied to match the main screen's mode resolution (e.g., `4x` magnification for Mode 0).
* **ASIC Mapping:** Hardware sprite configuration registers and pixel matrices are mapped directly into the CPU's memory space when the ASIC registers are unlocked.

