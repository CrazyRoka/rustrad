### Gate Array Registers & Color Palette

The custom Amstrad Gate Array coordinates video generation, ROM paging, and color palette translation.

#### I/O Port Addressing
The Gate Array is selected when address bits 15 and 14 are configured as:
```text
Address A15 = 0, A14 = 1
```

Writing to any port matching this mask targets the Gate Array. The standard, collision-free register port is `&7Fxx`. The Gate Array is **write-only**.

#### Register Selection (Control Byte)
When a byte is written to the Gate Array, bits 7 and 6 determine the register function:

| Bit 7 | Bit 6 | Target Register / Function |
| :---: | :---: | :--- |
| 0 | 0 | Select Pen (Inks) |
| 0 | 1 | Select Color for the selected Pen |
| 1 | 0 | Select Screen Mode, ROM Configuration, and Interrupt Reset |
| 1 | 1 | RAM Memory Management (Bypassed to RAM PAL on CPC 6128) |

---

### Register Definitions

#### 1. Select Pen Register (Bits 7=0, 6=0)
* **Standard Pen Selection:** If bit 4 is `0`, bits 3–0 specify which ink palette index (`0` to `15`) is targeted for color reassignment.
* **Border Selection:** If bit 4 is `1`, bits 3–0 are ignored, and the screen Border is targeted.

#### 2. Select Color Register (Bits 7=0, 6=1)
Assigns a physical hardware color value to the currently selected pen. Bits 4–0 select the color index from the 27-color analog palette:

| Color Index | Hardware Color Name | R % | G % | B % |
| :---: | :--- | :---: | :---: | :---: |
| **0** | White | 50 | 50 | 50 |
| **1** | White (Duplicate) | 50 | 50 | 50 |
| **2** | Sea Green | 0 | 100 | 50 |
| **3** | Pastel Yellow | 100 | 100 | 50 |
| **4** | Blue | 0 | 0 | 50 |
| **5** | Purple | 100 | 0 | 50 |
| **6** | Cyan | 0 | 50 | 50 |
| **7** | Pink | 100 | 50 | 50 |
| **8** | Purple (Duplicate) | 100 | 0 | 50 |
| **9** | Pastel Yellow (Duplicate) | 100 | 100 | 50 |
| **10** | Bright Yellow | 100 | 100 | 0 |
| **11** | Bright White | 100 | 100 | 100 |
| **12** | Bright Red | 100 | 0 | 0 |
| **13** | Bright Magenta | 100 | 0 | 100 |
| **14** | Orange | 100 | 50 | 0 |
| **15** | Pastel Magenta | 100 | 50 | 100 |
| **16** | Blue (Duplicate) | 0 | 0 | 50 |
| **17** | Sea Green (Duplicate) | 0 | 100 | 50 |
| **18** | Bright Green | 0 | 100 | 0 |
| **19** | Bright Cyan | 0 | 100 | 100 |
| **20** | Black | 0 | 0 | 0 |
| **21** | Bright Blue | 0 | 0 | 100 |
| **22** | Green | 0 | 50 | 0 |
| **23** | Sky Blue | 0 | 50 | 100 |
| **24** | Magenta | 50 | 0 | 50 |
| **25** | Pastel Green | 50 | 100 | 50 |
| **26** | Lime | 50 | 100 | 0 |
| **27** | Pastel Cyan | 50 | 100 | 100 |
| **28** | Red | 50 | 0 | 0 |
| **29** | Mauve | 50 | 0 | 100 |
| **30** | Yellow | 50 | 50 | 0 |
| **31** | Pastel Blue | 50 | 50 | 100 |

#### 3. Mode and ROM Configuration Register (Bits 7=1, 6=0)

| Bit | Default State | Description / Function |
| :---: | :---: | :--- |
| **4** | 0 | **Interrupt Generation Control:** Writing `1` resets the Gate Array's internal 6-bit interrupt counter to `0` and clears pending interrupts. |
| **3** | 0 | **Upper ROM Mapping:** `0` = Enabled (paged in at `&C000-&FFFF`), `1` = Disabled. |
| **2** | 0 | **Lower ROM Mapping:** `0` = Enabled (paged in at `&0000-&3FFF`), `1` = Disabled. |
| **1 - 0**| 0 | **Screen Mode Selection:** Determines active resolution and color depth (see mode table below). |

##### Mode Selection Mapping (Bits 1–0)
* **Mode 0:** `00` — $160 \times 200$ Resolution, 16 Colors.
* **Mode 1:** `01` — $320 \times 200$ Resolution, 4 Colors.
* **Mode 2:** `10` — $640 \times 200$ Resolution, 2 Colors.
* **Mode 3:** `11` — $160 \times 200$ Resolution, 4 Colors (Unofficial mode; uses Mode 0 pixel dimensions but restricts active pixels to Pens 0–3).

*Note: Mode changes do not take effect immediately upon register write; they are synchronized to, and execute from, the next falling HSYNC transition.*

The custom Amstrad Gate Array translates internal Pen selections to physical colors and controls the display frame parameters.

#### Scanline and Frame Timing (Base Model: CPC 464)
* **Scanline Duration:** Exactly 64 microseconds.
* <!-- TODO: Determine the exact total number of scanlines per frame -->
* <!-- TODO: Determine the exact duration of Vertical Blanking -->
* <!-- TODO: Determine the exact duration of Horizontal Blanking -->

#### Palette Definition Lookup Table
The CPC uses 5 bits to define a color code. While this allows for 32 theoretical values, the three-state logic (0%, 50%, 100%) applied to the Red, Green, and Blue pins yields exactly **27 distinct analog colors**. 

When writing to the **INKR (Select Color)** register, you must map the active color code to the corresponding hex and RGB levels:

| Quick Ref Code | Hardware Index | Color Name | R % | G % | B % | Hex Code |
| :---: | :---: | :--- | :---: | :---: | :---: | :--- |
| `&54` | `54h` | Black | 0 | 0 | 0 | `#000000` |
| `&44` | `44h` (or `50h`) | Blue | 0 | 0 | 50 | `#000080` |
| `&55` | `55h` | Bright Blue | 0 | 0 | 100 | `#0000FF` |
| `&5C` | `5Ch` | Red | 50 | 0 | 0 | `#800000` |
| `&58` | `58h` | Magenta | 50 | 0 | 50 | `#800080` |
| `&5D` | `5Dh` | Mauve | 50 | 0 | 100 | `#8000FF` |
| `&4C` | `4Ch` | Bright Red | 100 | 0 | 0 | `#FF0000` |
| `&45` | `45h` (or `48h`) | Purple | 100 | 0 | 50 | `#FF0080` |
| `&4D` | `4Dh` | Bright Magenta| 100 | 0 | 100 | `#FF00FF` |
| `&56` | `56h` | Green | 0 | 50 | 0 | `#008000` |
| `&46` | `46h` | Cyan | 0 | 50 | 50 | `#008080` |
| `&57` | `57h` | Sky Blue | 0 | 50 | 100 | `#0080FF` |
| `&5E` | `5Eh` | Yellow | 50 | 50 | 0 | `#808000` |
| `&40` | `40h` (or `41h`) | White | 50 | 50 | 50 | `#808080` |
| `&5F` | `5Fh` | Pastel Blue | 50 | 50 | 100 | `#8080FF` |
| `&4E` | `4Eh` | Orange | 100 | 50 | 0 | `#FF8000` |
| `&47` | `47h` | Pink | 100 | 50 | 50 | `#FF8080` |
| `&4F` | `4Fh` | Pastel Magenta| 100 | 50 | 100 | `#FF80FF` |
| `&52` | `52h` | Bright Green | 0 | 100 | 0 | `#00FF00` |
| `&42` | `42h` (or `51h`) | Sea Green | 0 | 100 | 50 | `#00FF80` |
| `&53` | `53h` | Bright Cyan | 0 | 100 | 100 | `#00FFFF` |
| `&5A` | `5Ah` | Lime | 50 | 100 | 0 | `#80FF00` |
| `&59` | `59h` | Pastel Green | 50 | 100 | 50 | `#80FF80` |
| `&5B` | `5Bh` | Pastel Cyan | 50 | 100 | 100 | `#80FFFF` |
| `&4A` | `4Ah` | Bright Yellow | 100 | 100 | 0 | `#FFFF00` |
| `&43` | `43h` (or `49h`) | Pastel Yellow | 100 | 100 | 50 | `#FFFF80` |
| `&4B` | `4Bh` | Bright White | 100 | 100 | 100 | `#FFFFFF` |