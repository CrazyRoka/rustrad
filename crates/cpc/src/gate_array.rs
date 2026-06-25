pub struct GateArray {
    lower_rom_enabled: bool,
    upper_rom_enabled: bool,
}

impl GateArray {
    pub fn new() -> Self {
        Self {
            lower_rom_enabled: true,
            // TODO: check if upper rom is enabled by default or not
            upper_rom_enabled: true,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        todo!();
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        assert_eq!(addr, 0x7F00);

        // TODO: handle other bits
        self.lower_rom_enabled = (value & (1 << 2)) == 0;
        self.upper_rom_enabled = (value & (1 << 3)) == 0;
    }

    pub fn lower_rom_enabled(&self) -> bool {
        self.lower_rom_enabled
    }

    pub fn upper_rom_enabled(&self) -> bool {
        self.upper_rom_enabled
    }
}
