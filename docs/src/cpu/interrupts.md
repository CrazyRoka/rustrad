### Z80 Interrupt Execution & Restarts (Base Model: CPC 464)

The Amstrad CPC 464 architecture utilizes standard Z80 restart (RST) instructions for system services, firmwire calls, and interrupt handlers. An emulator must handle the following vector behaviors when these instructions are executed:

| Mnemonic | Opcode | Target Address | Low-Level Operation / Firmware Function |
| :--- | :--- | :--- | :--- |
| **RST 0** | `&C7` | `&0000` | Complete system reset. Reinitializes hardware/firmware, then hands control to Upper ROM 0. |
| **RST 1** | `&CF` | `&0008` | `LOW JUMP`: Jumps to a routine in Lower ROM or low RAM. Followed by a 2-byte inline address configuration:<br> - `b0` to `b13`: Target address.<br> - `b14`: Low ROM disable flag.<br> - `b15`: Upper ROM disable flag. |
| **RST 2** | `&D7` | `&0010` | `SIDE CALL`: Inline 2-byte side address call to active foreground ROM groups.<br> - `b0` to `b13`: Address minus `&C000`.<br> - `b14` to `b15`: ROM offset relative to selected Foreground ROM. |
| **RST 3** | `&DF` | `&0018` | `FAR CALL`: Followed by a 2-byte inline pointer to a 3-byte address block:<br> - Bytes 0–1: Target memory address.<br> - Byte 2: ROM selection address. |
| **RST 4** | `&E7` | `&0020` | `RAM LAM`: Force execution of `LD A,(HL)` from RAM, regardless of whether Upper or Lower ROMs are paged in. |
| **RST 5** | `&EF` | `&0028` | `FIRM JUMP`: Followed by a 2-byte target address. Temporarily enables Lower ROM, executes the jump, and disables Lower ROM on return. |
| **RST 6** | `&F7` | `&0030` | User restart vector. Defaults to `RST 0`. |
| **RST 7** | `&FF` | `&0038` | Primary Interrupt entry vector (Keyboard scan, System timers). Triggered by the system's 1/300-second interrupt mechanism. |

#### External Hardware Interrupts
* **Vector Address:** `&003B` (`EXT INTERRUPT`)
* **Behavior:** Called directly when an external hardware interrupt is asserted via the expansion port. The Lower ROM is disabled on entry.
* **Emulator Note:** The default instruction at `&003B` is a `RET`. If external interrupts occur without a user patch at this address, the system will enter an infinite interrupt loop and hang because the interrupt request is not cleared.

