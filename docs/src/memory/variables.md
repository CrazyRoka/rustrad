# System Variables Reference

To ensure accurate emulator state tracking, debuggers should monitor the following primary hardware-relevant system variables located in low RAM.

| Address (6128) | Address (*464*) | Size (Bytes) | Label / Technical Description |
| :--- | :--- | :---: | :--- |
| `&B7C3` | `&B1C8` | 1 | **MODE Number:** Active display configuration (Values: `0`, `1`, `2`). |
| `&B7C4` | `&B1C9` | 2 | **Screen Offset:** Word tracking the active CRTC hardware offset value (Modulo `&0800`). |
| `&B7C6` | `&B1CC` | 1 | **Screen Base:** High byte of screen address (Defaults: `&C0` or `&40`). |
| `&B7D2` | `&B1D7` | 1 | **First Flash Duration:** Interval for alternating color flash phases (In 1/50th second steps). |
| `&B7D3` | `&B1D8` | 1 | **Second Flash Duration:** Interval for alternating color flash phases (In 1/50th second steps). |
| `&B7D4` | `&B1DA` | 1 | **Border Color:** Active Hardware color number loaded to Border. |
| `&B7D5 - &B7E4`| `&B1DB - &B1E9`| 16 | **First Ink Vectors:** Primary hardware color mappings assigned to Pen 0 through 15. |
| `&B7E5` | `&B1EA` | 1 | **Border Flash Color:** Flash phase hardware color assigned to Border. |
| `&B7E6 - &B7F5`| `&B1EB - &B1FA`| 16 | **Second Ink Vectors:** Flash phase hardware color mappings assigned to Pen 0 through 15. |
| `&B8D5` | *N/A* | 1 | **RAM Bank Selection:** Active RAM layout (Only valid on CPC 6128). |
| `&B8D6` | `&B1A8` | 1 | **Upper ROM Selector:** Holds selection index of the active Upper ROM. |
| `&BE5F` | *N/A* | 1 | **Disk Motor State Flag:** Inverted register flag tracking active motor power (`&00` = On, `&01` = Off). |