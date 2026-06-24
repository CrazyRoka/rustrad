### Cassette Hardware Interface

Cassette read, write, and motor control lines are routed directly through the Intel 8255 PPI chip.

---

### Low-Level Register Pin Routing

The CPU does not interface with the analog tape circuitry directly. Instead, it accesses tape states via bits mapped to PPI Ports B and C:

* **Cassette Motor Power (PPI Port C, Bit 4 - Output):**
  * `1` = Relay closed (Motor **ON**).
  * `0` = Relay open (Motor **OFF**).
* **Cassette Write Data (PPI Port C, Bit 5 - Output):**
  * Drives the active voltage level (high/low) sent to the tape recorder write-head to record magnetic transitions.
* **Cassette Read Data (PPI Port B, Bit 7 - Input):**
  * Reads the serial, un-latched real-time digitized audio bitstream directly from the tape deck read-head.

---

### Hardware Configuration Deltas

Cassette hardware features differ significantly depending on the Amstrad CPC computer model:

#### 1. CPC 464 & CPC 464+
* **Configuration:** Built-in physical tape player deck integrated directly into the main plastic enclosure.
* **Control:** Contains no external cassette input/output ports. Tape motor power is entirely controlled by the PPI Port C Bit 4 output line.

#### 2. CPC 664, CPC 6128 & KC Compact
* **Configuration:** No internal tape player deck. Instead, a physical **5-pin DIN connector** is populated on the PCB rear.
* **Connection:** Requires a cassette patch lead (5-pin DIN to three 3.5mm phono plugs: Audio In, Audio Out, Motor Control).

#### 3. CPC 6128+
* **Configuration:** No internal tape player, and **no cassette connector** is present on the physical chassis. Tape cassette software cannot be loaded on this machine without hardware motherboard modifications.

---

### Operating System Tape Cataloging (Verify Command)

During development and testing of tape loading emulation, developers can utilize the firmware's built-in cassette diagnostic tool.

#### The `|TAPE` and `CAT` Sequence:
1. Initialize the tape subsystem by typing: `|TAPE`
2. Run the tape catalog command: `CAT`
3. The system will prompt: `Press PLAY then any key`
4. Once a key is pressed, the CPC starts the tape motor and reads the standard speed blocks.

As each block header is parsed and verified against its 8-bit XOR checksum, the OS outputs the following line:

```text
[FILENAME]      block [BLOCK_NUMBER] [TYPE_SYMBOL] Ok
```

##### File Type Decodes:
* **`$`** = Unprotected BASIC program.
* **`%`** = Protected BASIC program (cannot be listed).
* **`&`** = Unprotected Binary file.
* **`'`** = Protected Binary file.

*Emulator Debugging Note:* If a read error or checksum failure occurs, the OS prints `Read error a` or `Read error b` instead of `Ok`. This can be used to isolate timing or parity bugs in your cassette edge-detection logic.
