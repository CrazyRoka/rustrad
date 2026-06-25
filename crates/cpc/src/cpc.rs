use crate::{GateArray, Ppi, memory::CpcMemory};
use z80::Bus;

pub struct Cpc {
    rom: [u8; 0x8000], // 32 KB
    memory: CpcMemory,
    // Peripherals
    gate_array: GateArray,
    ppi: Ppi,
}

impl Cpc {
    pub fn new(memory: CpcMemory, rom: &[u8], gate_array: GateArray, ppi: Ppi) -> Self {
        assert_eq!(rom.len(), 0x8000, "ROM length is supposed to be 32KB");
        let mut rom_clone = [0; 0x8000];
        rom_clone.copy_from_slice(rom);

        Self {
            rom: rom_clone,
            memory,
            gate_array,
            ppi,
        }
    }

    pub fn gate_array_mut(&mut self) -> &mut GateArray {
        &mut self.gate_array
    }

    pub fn gate_array(&self) -> &GateArray {
        &self.gate_array
    }
}

impl Bus for Cpc {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF if self.gate_array.lower_rom_enabled() => self.rom[addr as usize],
            0xC000..=0xFFFF if self.gate_array.upper_rom_enabled() => {
                self.rom[addr as usize - 0x8000]
            }
            _ => self.memory.read(addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.memory.write(addr, value);
    }

    fn port_read(&self, port: u16) -> u8 {
        match port >> 8 {
            0xF4..=0xF7 => self.ppi.read(port),
            _ => todo!("Unexpected port read at address {:#04X}", port),
        }
    }

    fn port_write(&mut self, port: u16, value: u8) {
        match port >> 8 {
            // TODO: handle
            0xEF | 0xBC | 0xBD | 0xDF | 0xF8 => {}
            0xF4..=0xF7 => self.ppi.write(port, value),
            0x7F => self.gate_array.write(port, value),
            _ => todo!(
                "Unexpected port write at address {:#04X} with value {:#02x}",
                port,
                value
            ),
        }
    }

    fn acknowledge_interrupt(&mut self) {
        self.gate_array.acknowledge_interrupt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to generate a dummy 32KB ROM structure
    // 0x0000 - 0x3FFF contains 0x11 (Lower ROM representation)
    // 0x4000 - 0x7FFF contains 0x22 (Upper ROM page 0 representation)
    fn create_test_rom() -> [u8; 0x8000] {
        let mut rom = [0; 0x8000];
        for i in 0..0x4000 {
            rom[i] = 0x11;
        }
        for i in 0x4000..0x8000 {
            rom[i] = 0x22;
        }
        rom
    }

    fn create_cpc() -> Cpc {
        let memory = CpcMemory::new_64k();
        let rom = create_test_rom();
        let ppi = Ppi::new();
        let gate_array = GateArray::new();

        Cpc::new(memory, &rom, gate_array, ppi)
    }

    #[test]
    fn test_ram_read_write() {
        let mut cpc = create_cpc();

        // Middle area
        for i in 0x4000..=0xBFFF {
            cpc.write(i, (i & 0xFF) as u8);
            assert_eq!(cpc.read(i), (i & 0xFF) as u8);
        }
    }

    #[test]
    fn test_rom_default_mapping() {
        let cpc = create_cpc();

        // Lower ROM should be active by default
        assert_eq!(cpc.read(0x1000), 0x11);

        // Upper ROM should be active by default
        assert_eq!(cpc.read(0xD000), 0x22);
    }

    #[test]
    fn test_write_through_to_ram() {
        let mut cpc = create_cpc();

        // Writes to 0x1000 (Lower ROM region) should pass through to RAM
        cpc.write(0x1000, 0x99);
        // Reading should still return Lower ROM content while enabled
        assert_eq!(cpc.read(0x1000), 0x11);

        // Disable Lower ROM
        cpc.port_write(0x7F00, 0x84); // Gate Array Multi-Config: Lower ROM disabled (bit 2 set)

        // Reading should now show the written value in RAM
        assert_eq!(cpc.read(0x1000), 0x99);
    }

    #[test]
    fn test_gate_array_rom_control() {
        let mut cpc = create_cpc();

        // Write values to underlying RAM in ROM spaces
        cpc.write(0x1000, 0xAA);
        cpc.write(0xD000, 0xBB);

        // Disable Lower ROM, leave Upper ROM enabled
        cpc.port_write(0x7F00, 0x84);
        assert_eq!(cpc.read(0x1000), 0xAA); // Should see RAM
        assert_eq!(cpc.read(0xD000), 0x22); // Should see Upper ROM

        // Enable Lower ROM, disable Upper ROM
        cpc.port_write(0x7F00, 0x88);
        assert_eq!(cpc.read(0x1000), 0x11); // Should see Lower ROM
        assert_eq!(cpc.read(0xD000), 0xBB); // Should see RAM

        // Disable both ROMs
        cpc.port_write(0x7F00, 0x8C);
        assert_eq!(cpc.read(0x1000), 0xAA); // Should see RAM
        assert_eq!(cpc.read(0xD000), 0xBB); // Should see RAM

        // Enable both ROMs
        cpc.port_write(0x7F00, 0x80);
        assert_eq!(cpc.read(0x1000), 0x11); // Should see Lower ROM
        assert_eq!(cpc.read(0xD000), 0x22); // Should see Upper ROM
    }
}
