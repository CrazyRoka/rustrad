# Video Subsystem Overview

The CPC video subsystem uses a dual-processor architecture: a standard **CRTC 6845** controller handles scanline timing and address generation, while a custom **Gate Array** translates memory bytes into active screen pixels.

```
       +------------------+         +----------------+
RAM -> |   Gate Array     | ------> | Palette Mixing | -> RGB Out
       | (Pixel Decoder)  |         +----------------+
       +------------------+                 ^
                ^                           |
                | (Pixel Clock / Address)   | (Inks)
       +------------------+                 |
       |    CRTC 6845     | ----------------+
       +------------------+
```

## Functional Roles

### The CRTC 6845
Responsible for generating sync signals (HSYNC, VSYNC), defining horizontal/vertical blanking periods, and driving the 14-bit memory address bus used to retrieve pixel data from RAM. It also manages hardware scrolling and screen splitting.

### The Gate Array
Acts as the bus arbitrator and pixel rendering pipeline. It multiplexes RAM access between the CPU and the display generator, decodes planar bytes into color pixels, manages the hardware palette (color mixing), and generates maskable interrupts.

## Chapter Directory
* [CRTC Port I/O & Registers](crtc_registers.md)
* [Video Memory Mapping & Offsets](video_memory.md)
* [Pixel Decoding & Video Modes](pixel_decoding.md)
* [Gate Array Interrupt Generation](gate_array_interrupts.md)

