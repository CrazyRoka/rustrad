### CDT Tape Image Format

The Amstrad CPC **.CDT** (CPC Digital Tape) format is digitally identical to the Sinclair ZX Spectrum **.TZX** (v1.13) format. It stores the compact physical timing of tape pulses rather than raw, uncompressed audio waveforms.

---

### Clock Timing Translation

To maintain complete compatibility with the .TZX standard, **all timing values inside a CDT file are strictly stored in Sinclair ZX Spectrum T-states (3.5 MHz)**. 

Because the Amstrad CPC master clock runs at **4.0 MHz**, a Rust emulator must scale the timings during playback.

#### Exact Conversion Ratio
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

When replaying standard loading blocks (ID `&10`), the CPC operating system assumes a baud rate varying from **1000 to 2000 baud** with the following baseline structures:

* **Pulse vs. Wave:** A single pulse (half-period) is represented as `----` or `____`. A full wave (full-period) is represented as two pulses `----____`.
* **Pilot Tone:** The pilot tone consists of a sequence of pulses. The length of each individual pilot pulse is equal to the duration of a **Bit-1** pulse.
* **Sync Signals:** The sync phase consists of two pulses (Sync 1 and Sync 2). The duration of both Sync 1 and Sync 2 is equal to the duration of a **Bit-0** pulse.
* **Data Bits:**
  * **Bit-0:** Always exactly **half the size** of a Bit-1 pulse.
  * **Bit-1:** Duration is read directly from the pilot tone pulse timing.

---

### CDT Block ID Implementation Rules

An Amstrad CPC emulator must handle the following TZX block IDs specifically for CDT operations:

#### ID 10: Standard Speed Data Block
* **Support:** Mandatory.
* **Behavior:** Replays data using the standard Amstrad ROM tape-loading timing variables.

#### ID 11: Turbo Loading Data Block
* **Support:** Mandatory.
* **Behavior:** Replays data using the explicit pulse and pilot timings specified in the block's header.

#### ID 13: Sequence of Pulses of Different Lengths
* **Support:** Mandatory.
* **Behavior:** Replays custom pulse trains. Typically used by custom loaders and speed-lock software protections to generate non-standard sync tones.

#### ID 14: Pure Data Block
* **Support:** Mandatory.
* **Behavior:** Replays raw data bits immediately, omitting standard pilot and sync pulses.

#### ID 15: Direct Recording
* **Support:** Mandatory.
* **Behavior:** Replays raw digital state samples (`0` = low output, `1` = high output) at a specified sample rate (typically 22050 Hz or 44100 Hz). This block should be used by emulators when **writing/recording** a new tape image from the CPC.

#### ID 20: Pause / Stop Tape Command
* **Behavior:** 
  * If the pause value is **greater than 0**, output a low amplitude state for the specified duration (in milliseconds).
  * If the pause value is **exactly 0**: Sinclair Spectrum emulators treat this as an infinite pause ("Stop the Tape"). However, **Amstrad CPC emulators must ignore a 0-length pause**; it must be treated as "no pause," and tape execution must continue uninterrupted.

#### ID 2A: Stop Tape if in 48K Mode
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**. It must not trigger a pause, nor stop the tape motor.

#### ID 33: Hardware Type
* **Amstrad Rule:** Hardware types `0x01` (External Storage) through `0x0F` (EPROM programmers) are Spectrum specific and must be ignored. Hardware Type `0x00` (Computers) is for guideline informational purposes only.

#### ID 34: Emulation Info
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**.

#### ID 40: Snapshot Block
* **Support:** Spectrum specific.
* **Amstrad Rule:** **Must be entirely ignored**.