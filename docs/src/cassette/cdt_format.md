### CDT Tape Image Format

The Amstrad CPC **.CDT** (CPC Digital Tape) format is structurally identical to the Sinclair ZX Spectrum **.TZX** format (currently revision v1.20). It stores the compact physical timing of tape pulses rather than raw, uncompressed audio waveforms. While the format was originally designed for the ZX Spectrum, the CDT extension is used to distinguish tape images for the Amstrad CPC, though the internal structure and block rules remain exactly the same.

---

### Rules and Definitions

To correctly parse and emulate CDT files, the following standard rules apply:
1. **Endianness:** Any value requiring more than one byte is stored in little-endian format (LSB first).
2. **Text Encoding:** All ASCII texts use the ISO 8859-1 (Latin 1) encoding. Lines are separated by a single `0x0D` (13 decimal).
3. **Timing Base:** All timings are given in Z80 clock ticks (T-states) based on a **3.5 MHz clock** (the standard ZX Spectrum clock), regardless of the target machine's actual clock speed.
4. **Pulse Levels:** The format refers to 'high' and 'low' pulse levels. A 'pulse' is defined as a half-period (either `----` or `____`).

---

### Clock Timing Translation

Because the Amstrad CPC master clock runs at **4.0 MHz** and all CDT timings are strictly stored in ZX Spectrum T-states (3.5 MHz), an emulator must scale the timings during playback.

Rather than using floating-point conversions, you must scale the timings using the exact integer ratio fraction **8/7** to prevent cumulative rounding drift:

```rust
// Converts a 3.5 MHz CDT T-state duration to CPC 4.0 MHz T-states
fn cdt_to_cpc_t_states(cdt_t_states: u32) -> u64 {
    (cdt_t_states as u64 * 8) / 7
}
```

#### Mathematical Basis:
```text
1 T-state (CDT) = 1 / 3,500,000 seconds
1 T-state (CPC) = 1 / 4,000,000 seconds

CPC T-states = CDT T-states * (4,000,000 / 3,500,000)
CPC T-states = CDT T-states * (8 / 7)
```

---

### Amstrad ROM Speed and Sync Timings

Unlike the ZX Spectrum, the Amstrad CPC ROM load/save routine uses a **variable speed** for loading, typically varying from **1000 to 2000 baud**. Because of this, standard speed blocks (ID `&10`) are rarely used for Amstrad-specific standard blocks; instead, Turbo Loading (ID `&11`) or Generalized Data (ID `&19`) blocks are used to define the exact timings.

When replaying standard Amstrad ROM loading blocks, the baseline structure is as follows:

* **Pilot Tone:** The pilot tone consists of a sequence of **4096 pulses**. The length of each individual pilot pulse is equal to the duration of a **Bit-1** pulse.
* **Sync Signals:** The sync phase consists of two pulses (Sync 1 and Sync 2). The duration of both Sync 1 and Sync 2 is equal to the duration of a **Bit-0** pulse.
* **Data Bits:**
  * **Bit-0:** Always exactly **half the size** of a Bit-1 pulse.
  * **Bit-1:** Duration is read directly from the pilot tone pulse timing.
* **Checksum:** The checksum algorithm used by the Amstrad CPC ROM differs from the ZX Spectrum.

---

### CDT Block ID Implementation Rules

An Amstrad CPC emulator must handle the following TZX block IDs with these specific Amstrad rules:

#### ID 10: Standard Speed Data Block
* **Support:** Mandatory.
* **Behavior:** This block can exist in a CDT and must be supported. Emulators should use standard ZX Spectrum ROM timings for playback.

#### ID 11: Turbo Loading Data Block
* **Support:** Mandatory.
* **Behavior:** The timings for playback are stored in the block header. Replays data using these explicit pulse and pilot timings. This is commonly used for Amstrad standard loads to lock in the variable baud rate.

#### ID 12: Pure Tone
* **Support:** Mandatory.
* **Behavior:** Produces a tone with a defined pulse length and count.

#### ID 13: Sequence of Pulses of Different Lengths
* **Support:** Mandatory.
* **Behavior:** The timings for playback are stored in the block header. Replays custom pulse trains. Typically used by custom loaders and speed-lock software protections to generate non-standard sync tones.

#### ID 14: Pure Data Block
* **Support:** Mandatory.
* **Behavior:** The timings for playback are stored in the block header. Replays raw data bits immediately, omitting standard pilot and sync pulses.

#### ID 15: Direct Recording
* **Support:** Mandatory.
* **Behavior:** Replays raw digital state samples (`0` = low output, `1` = high output) at a specified sample rate. This block can be used by emulators to support **writing/recording** a new tape image to CDT, but should be avoided by sample-to-CDT converters when creating new files.

#### ID 18: CSW Recording (v1.20)
* **Support:** Optional.
* **Behavior:** Contains a sequence of raw pulses encoded in CSW (Compressed Square Wave) format v2. Useful for highly irregular tape data that cannot be easily represented by standard data blocks.

#### ID 19: Generalized Data Block (v1.20)
* **Support:** Recommended.
* **Behavior:** A highly flexible block capable of representing an extremely wide range of data encoding techniques. It allows associating different sequences of pulses (waves) to pilot tones, sync pulses, and data bits. This is exceptionally useful for Amstrad CPC custom loaders that use varying pulse counts per bit or non-standard waveforms.

#### ID 20: Pause / Stop Tape Command
* **Behavior:** 
  * If the pause value is **greater than 0**, output a low amplitude state for the specified duration (in milliseconds).
  * If the pause value is **exactly 0**: Sinclair Spectrum emulators treat this as an infinite pause ("Stop the Tape"). However, **Amstrad CPC emulators must ignore a 0-length pause**; it should be treated as "no pause," and tape execution must continue uninterrupted.

#### ID 2A: Stop Tape if in 48K Mode
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**. It must not trigger a pause, nor stop the tape motor.

#### ID 2B: Set Signal Level (v1.20)
* **Support:** Optional.
* **Behavior:** Sets the current signal level to the specified value (high or low). Used to avoid ambiguities in custom loaders which are level-sensitive.

#### ID 33: Hardware Type
* **Amstrad Rule:** Hardware types `0x01` (External Storage) through `0x0F` (EPROM programmers) are Spectrum specific and must be ignored. Hardware Type `0x00` (Computers) can be used but only as a guideline.

#### ID 34: Emulation Info (Deprecated in v1.20)
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**. This block was deprecated in TZX v1.20 and should not appear in modern CDT files.

#### ID 40: Snapshot Block (Deprecated in v1.20)
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**. This block was deprecated in TZX v1.20.

---

### Deprecated Blocks
The following blocks were present in TZX v1.13 but have been deprecated in v1.20. They should not be added to new CDT files, but emulators may still parse them for backward compatibility:
* **ID 16:** C64 ROM Type Data Block
* **ID 17:** C64 Turbo Tape Data Block
* **ID 34:** Emulation Info
* **ID 40:** Snapshot Block
* **ID 35 (Standardized Types):** The standard uses of the Custom Info block (POKEs, Instructions, Spectrum Screen, ZX-Edit document, Picture) are deprecated.