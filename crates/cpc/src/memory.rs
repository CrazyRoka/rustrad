use crate::memory::CpcMemory::Model64K;

pub trait MemoryReader {
    fn read_byte(&self, addr: u16) -> u8;
}

pub enum CpcMemory {
    Model64K { ram: [u8; 0x10000] },
}

impl CpcMemory {
    pub fn new_64k() -> Self {
        Self::Model64K {
            ram: [0xFF; 0x10000],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match self {
            Model64K { ram } => ram[addr as usize],
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match self {
            Model64K { ram } => ram[addr as usize] = value,
        }
    }
}

impl MemoryReader for CpcMemory {
    fn read_byte(&self, addr: u16) -> u8 {
        self.read(addr)
    }
}
