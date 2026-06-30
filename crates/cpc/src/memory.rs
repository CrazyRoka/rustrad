pub trait MemoryReader {
    fn read_byte(&self, addr: u16) -> u8;
}

pub enum CpcMemory {
    Model64K { ram: [u8; 0x10000] },
    Model128K { ram: [u8; 0x20000] },
}

impl CpcMemory {
    pub fn new_64k() -> Self {
        Self::Model64K {
            ram: [0xFF; 0x10000],
        }
    }

    pub fn new_128k() -> Self {
        Self::Model128K {
            ram: [0xFF; 0x20000],
        }
    }

    pub fn read(&self, addr: u16, memory_bank_selection: u8) -> u8 {
        let memory_address = self.memory_address(addr, memory_bank_selection);
        match self {
            Self::Model64K { ram } => ram[memory_address],
            Self::Model128K { ram } => ram[memory_address],
        }
    }

    pub fn write(&mut self, addr: u16, value: u8, memory_bank_selection: u8) {
        let memory_address = self.memory_address(addr, memory_bank_selection);
        match self {
            Self::Model64K { ram } => ram[memory_address] = value,
            Self::Model128K { ram } => ram[memory_address] = value,
        }
    }

    fn memory_address(&self, addr: u16, memory_bank_selection: u8) -> usize {
        match self {
            Self::Model64K { .. } => addr as usize,
            Self::Model128K { .. } => match addr {
                0x0000..=0xFFFF if memory_bank_selection == 2 => 0x10000 + addr as usize,
                0x4000..=0x7FFF if memory_bank_selection == 3 => 0x8000 + addr as usize,
                0x4000..=0x7FFF if memory_bank_selection >= 4 && memory_bank_selection <= 7 => {
                    0x10000
                        + (memory_bank_selection as usize - 4) * 0x4000
                        + (addr - 0x4000) as usize
                }
                0xC000..=0xFFFF if memory_bank_selection >= 1 && memory_bank_selection <= 3 => {
                    0x10000 + addr as usize
                }
                _ => addr as usize,
            },
        }
    }
}

impl MemoryReader for CpcMemory {
    fn read_byte(&self, addr: u16) -> u8 {
        self.read(addr, 0)
    }
}
