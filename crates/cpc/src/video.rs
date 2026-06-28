use crate::{Cpc, ScreenMode};

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
            for x in 0..80 {
                let byte = bus.read_ram(scanline_address + x);

                for (pixel_idx, pen) in bus.gate_array().mode().decode_byte(byte).iter().enumerate()
                {
                    if *pen == 0xFF {
                        break;
                    }
                    let color = bus.gate_array().ink_for_pen(*pen);
                    buffer[y as usize * WINDOW_WIDTH + (x as usize) * 4 + pixel_idx] =
                        color.color();
                }
            }
        }
    }
}

impl ScreenMode {
    /// Decodes bytes for each ScreenMode. Always returns an array of 8 elements to prevent heap allocations.
    /// Unused values equal to 0xFF
    #[inline(always)]
    pub fn decode_byte(&self, byte: u8) -> [u8; 8] {
        let mut res = [0xFF; 8];
        let pixels = match self {
            ScreenMode::Mode0 => 2,
            ScreenMode::Mode1 => 4,
            ScreenMode::Mode2 => 8,
            ScreenMode::Mode3 => 2,
        };
        for pixel_idx in 0..pixels {
            let pen = match self {
                ScreenMode::Mode0 => {
                    let highest = (byte >> (1 - pixel_idx)) & 1;
                    let high = (byte >> (5 - pixel_idx)) & 1;
                    let low = (byte >> (3 - pixel_idx)) & 1;
                    let lowest = (byte >> (7 - pixel_idx)) & 1;
                    (highest << 3) | (high << 2) | (low << 1) | lowest
                }
                ScreenMode::Mode1 => {
                    let low = (byte >> (7 - pixel_idx)) & 1;
                    let high = (byte >> (3 - pixel_idx)) & 1;
                    (high << 1) | low
                }
                ScreenMode::Mode2 => (byte >> (7 - pixel_idx)) & 1,
                ScreenMode::Mode3 => {
                    let low = (byte >> (3 - pixel_idx)) & 1;
                    let lowest = (byte >> (7 - pixel_idx)) & 1;
                    (low << 1) | lowest
                }
            };

            res[pixel_idx] = pen;
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::ScreenMode;

    #[test]
    fn mode0_decode_byte_00_yields_pen0_pen0() {
        let pixels = ScreenMode::Mode0.decode_byte(0x00);
        assert_eq!(pixels, [0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mode0_decode_byte_ff_yields_pen15_pen15() {
        let pixels = ScreenMode::Mode0.decode_byte(0xFF);
        assert_eq!(pixels, [15, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mode0_decode_byte_known_pattern() {
        let pixels = ScreenMode::Mode0.decode_byte(0x77);
        assert_eq!(pixels, [12, 15, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mode1_decode_byte_yields_4_pixels() {
        let pixels = ScreenMode::Mode1.decode_byte(0xF0);
        assert_eq!(pixels, [1, 1, 1, 1, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mode1_decode_byte_alternating_pattern() {
        let pixels = ScreenMode::Mode1.decode_byte(0xE1);
        assert_eq!(pixels, [1, 1, 1, 2, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mode2_decode_byte_yields_8_pixels() {
        let pixels = ScreenMode::Mode2.decode_byte(0xAA);
        assert_eq!(pixels, [1, 0, 1, 0, 1, 0, 1, 0]);
    }

    #[test]
    fn mode2_decode_byte_all_ones() {
        let pixels = ScreenMode::Mode2.decode_byte(0xFF);
        assert_eq!(pixels, [1, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn mode2_decode_byte_all_zeros() {
        let pixels = ScreenMode::Mode2.decode_byte(0x00);
        assert_eq!(pixels, [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn mode3_decode_uses_mode0_layout_capped_to_pen3() {
        let pixels = ScreenMode::Mode3.decode_byte(0xFF);
        assert_eq!(pixels.len(), 2);
        assert!(pixels.iter().all(|&p| p <= 3), "Mode 3 pens must be 0-3");
    }
}
