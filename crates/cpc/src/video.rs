use z80::Bus;

use crate::Cpc;

pub const WINDOW_WIDTH: usize = 320;
pub const WINDOW_HEIGHT: usize = 200;

pub struct Video {}

impl Video {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, bus: &Cpc, buffer: &mut [u32]) {
        for y in 0..200 {
            let scanline_address = 0xC000 + (y % 8) * 0x0800 + (y / 8) * 80;
            for x in (0..320).step_by(4) {
                let byte = bus.read(scanline_address + x / 4);

                for pixel_idx in 0..4 {
                    let low = (byte >> (7 - pixel_idx)) & 1;
                    let high = (byte >> (3 - pixel_idx)) & 1;
                    let pen = (high << 1) | low;

                    let color = bus.gate_array().ink_for_pen(pen).color();
                    buffer[y as usize * WINDOW_WIDTH + x as usize + pixel_idx] = color;
                }
            }
        }
    }
}
