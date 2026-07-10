# Amstrad CPC Emulator

A work-in-progress Amstrad CPC emulator written in Rust, targeting both the CPC 464 and CPC 6128 home computer models. The project is structured as a Cargo workspace with a clean separation between the emulation core and the user interface.

## Features

### Machine Models
- **CPC 464** — 64 KB RAM, 32 KB ROM (OS + BASIC 1.0), cassette tape storage
- **CPC 6128** — 128 KB RAM, 48 KB ROM (OS + BASIC 1.1 + AMSDOS), floppy disk drive

### CPU
- Cycle-accurate Z80 emulator (`crates/z80`) driving the machine bus
- Full instruction set incl. extended (`ED`) and bit (`CB`) opcodes

### Video
- **Gate Array**-based pixel generation with all four screen modes:
  - **Mode 0** — 160×200, 16 colors
  - **Mode 1** — 320×200, 4 colors
  - **Mode 2** — 640×200, 2 colors
  - **Mode 3** — 160×200, 4 colors (rarely used)
- Full 27-color hardware palette with proper RGB triplet encoding
- Border color support (Pen 16)
- R8 skew / "border force" handling

### CRTC
Faithful emulation of the 6845 family with selectable hardware variants:

| Type | Chip             | Used in                            |
|------|------------------|------------------------------------|
| 0    | UM6845 / HD6845S | Early CPC 464                      |
| 1    | UM6845R          | Mid-generation models              |
| 2    | MC6845           | Late CPC 464/6128 mainboards       |
| 3    | AMS40489         | CPC+ series (ASIC)                 |
| 4    | AMS40226          | Cost-down CPC 6128 (Pre-ASIC)      |

- Per-type register readability/masking rules
- HSYNC/VSYNC pulse generation with proper width semantics
- Light pen strobe (`LPSTB`) capture into R16/R17
- Cursor output with raster-bounded blink modes
- 14-bit memory address (`MA`) and raster (`RA`) output
- Internal counters `C0/C3l/C3h/C4/C5/C9` and `VMA/VMA'` tracked cycle-accurately

### Memory
- 64 KB and 128 KB configurations
- Gate Array RAM banking (MMR configurations 0–7)
- Lower/Upper ROM enable with write-through to underlying RAM
- Video fetch always reads from Bank 0, regardless of MMR config (hardware-accurate)

### Peripherals
- **8255 PPI**:
  - PSG bus function decoding (`BDIR`/`BC1` → Inactive/Read/Write/Select)
  - Bit Set/Reset (BSR) mode for individual Port C bits
  - Port B status reads: Cassette data, Printer busy, /EXP, 50/60 Hz jumper, Manufacturer jumpers, VSYNC
  - Cassette motor control with automatic tape play/stop
  - Keyboard scanning via PSG Register 14
- **Keyboard**:
  - 10-row × 8-column matrix (80 keys)
  - Hardware **ghost key** simulation (rectangle pattern detection)
  - Joystick 0 row (cursor keys mapping)
- **uPD765A/B FDC** (CPC 6128):
  - All standard commands: Read/Write Data, Read/Write Deleted, Read Track, Read ID, Format Track, Scan Equal/Low/High, Recalibrate, Seek, Specify, Sense Drive/Interrupt Status, Version
  - Multi-track (MT) reads across sides
  - Weak sector support (multiple data copies)
  - Per-drive concurrent seek state with independent `D0B–D3B` busy bits
  - 4 MHz clock doubling (matches CPC timing)
  - uPD765A vs. uPD765B overrun-on-last-byte behavior
- **Disk images**:
  - Standard (`MV - CPCEMU`) and Extended (`EXTENDED CPC DSK`) formats
  - Variable track sizes, unformatted tracks, per-sector ST1/ST2, arbitrary `N` values, weak sectors, GAP3 data
- **Tape (CDT)**:
  - v1.x header validation
  - Block types `0x10` (standard data), `0x11` (custom pulse data), `0x20` (pause), `0x30` (text description)
  - CPC-specific ZX/CPC T-state scaling
  - EAR output via PPI Port B bit 7
  - Motor-gated playback

### UI (cpc-ui)
Built with [Iced](https://github.com/iced-rs/iced):

- Side panel with controls:
  - Insert DSK / Load CDT file dialogs
  - Play / Stop / Rewind tape
  - Model selector (464 / 6128)
  - Restart button
  - FPS limit toggle (50 Hz lock vs. unlimited)
  - Scaling filter toggle (Nearest / Linear)
- 640×200 emulator canvas scaled to window
- Real-time FPS counter in title bar
- Keyboard mapped to CPC matrix
- Emulator runs on a dedicated background thread; UI communicates via channels

## Getting Started

### Prerequisites
- Rust toolchain (stable, edition 2021 or later)

### Build & Run

```bash
cargo run --release -p cpc-ui
```

### Controls
- Default keyboard layout maps host keys to the CPC matrix (see `map_keycode` in `cpc-ui/src/main.rs`)
- **F12** — Toggle FPS limiter
- UI buttons handle disk/tape/model operations

## Documentation

In-tree documentation lives in `docs/` as an [mdBook](https://rust-lang.github.io/mdBook/) source, covering:

- `cpu/` — Z80 implementation notes
- `memory/` — Banking and ROM mapping
- `video/` — Modes, palette, GA behaviour
- `crtc/` — CRTC types and register semantics
- `peripherals/` — PPI, PSG, keyboard, joystick
- `disk/` — FDC commands and DSK format
- `cassette/` — CDT blocks and pulse timing
- `sound/` — (planned) PSG audio

Build the docs with:
```bash
mdbook build docs
```

## Testing

The emulator has an extensive test suite covering nearly every component:

```bash
cargo test
```

Highlights:
- **CRTC**: ~70 tests exercising register access rules per type, HSYNC/VSYNC widths, DISPEN, cursor, light pen, vertical adjust, full-frame stability, and physical address decoding
- **Gate Array**: palette round-trip, mode/ROM/pen register semantics, interrupt counter (52 HSYNC) and VSYNC reset behaviour
- **PPI**: BSR bit manipulation, PSG bus function decoding, full keyboard scan sequence, cassette motor control, manufacturer jumper readout
- **FDC**: every command opcode, result phase byte counts, ST0/ST1/ST2/ST3 flags, multi-track reads, weak sectors, uPD765A/B variant differences, overrun timing
- **Disk**: standard vs. extended DSK parsing, variable track sizes, mixed sector types, weak sectors, GAP3 data, truncation errors
- **Tape**: every CDT block type, pulse durations, polarity, multi-byte bit ordering, motor gating
- **Keyboard**: ghost key detection with rectangle formation and partial release scenarios