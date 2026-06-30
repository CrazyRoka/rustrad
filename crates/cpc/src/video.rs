use crate::{Cpc, Crtc, GateArray, ScreenMode, memory::MemoryReader};

pub const WINDOW_WIDTH: usize = 640;
pub const WINDOW_HEIGHT: usize = 200;

pub struct Video {
    buffer: Box<[u32; WINDOW_HEIGHT * WINDOW_WIDTH]>,
}

impl Video {
    pub fn new() -> Self {
        Self {
            buffer: Box::new([0; WINDOW_HEIGHT * WINDOW_WIDTH]),
        }
    }

    pub fn buffer(&self) -> &[u32] {
        self.buffer.as_ref()
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    pub fn tick<R: MemoryReader>(&mut self, crtc: &Crtc, ga: &GateArray, ram: &R) {
        let char_x = crtc.c0() as usize;
        let pixel_x = char_x * 16;

        let char_y = crtc.c4() as usize;
        let pixel_y = char_y * (crtc.register(9) as usize + 1) + crtc.c9() as usize;

        if pixel_y >= WINDOW_HEIGHT || pixel_x + 16 > WINDOW_WIDTH {
            return;
        }

        let mode = ga.mode();
        let start = pixel_y * WINDOW_WIDTH + pixel_x;
        let end = start + 16;
        let slice = &mut self.buffer[start..end];

        if !crtc.dispen() {
            let border = ga.ink_for_pen(16);
            slice.fill(border.color());
            return;
        }

        let addr = crtc.phys_address();
        for byte_idx in 0..2 {
            let byte = ram.read_byte(addr + byte_idx as u16);
            let byte_pens = mode.decode_byte(byte);

            for (idx, pen) in byte_pens.iter().enumerate() {
                slice[byte_idx * 8 + idx] = ga.ink_for_pen(*pen).color();
            }
        }
    }
}

impl ScreenMode {
    /// Decodes bytes for each ScreenMode. Always returns an array of 8 elements.
    #[inline(always)]
    pub fn decode_byte(&self, byte: u8) -> [u8; 8] {
        let mut res = [0xFF; 8];
        let pixels = match self {
            ScreenMode::Mode0 => 2,
            ScreenMode::Mode1 => 4,
            ScreenMode::Mode2 => 8,
            ScreenMode::Mode3 => 2,
        };
        let width = 8 / pixels;
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

            for j in 0..width {
                res[pixel_idx * width + j] = pen;
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Crtc, GateArray, ScreenMode, Video, WINDOW_HEIGHT, WINDOW_WIDTH, memory::MemoryReader,
    };

    #[test]
    fn mode0_decode_byte_00_yields_pen0_pen0() {
        let pixels = ScreenMode::Mode0.decode_byte(0x00);
        assert_eq!(pixels, [0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn mode0_decode_byte_ff_yields_pen15_pen15() {
        let pixels = ScreenMode::Mode0.decode_byte(0xFF);
        assert_eq!(pixels, [15, 15, 15, 15, 15, 15, 15, 15]);
    }

    #[test]
    fn mode0_decode_byte_known_pattern() {
        let pixels = ScreenMode::Mode0.decode_byte(0x77);
        assert_eq!(pixels, [12, 12, 12, 12, 15, 15, 15, 15]);
    }

    #[test]
    fn mode1_decode_byte_yields_4_pixels() {
        let pixels = ScreenMode::Mode1.decode_byte(0xF0);
        assert_eq!(pixels, [1, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn mode1_decode_byte_alternating_pattern() {
        let pixels = ScreenMode::Mode1.decode_byte(0xE1);
        assert_eq!(pixels, [1, 1, 1, 1, 1, 1, 2, 2]);
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
        assert_eq!(pixels, [3, 3, 3, 3, 3, 3, 3, 3]);
    }

    /// 64 KB scratch RAM for tests, no ROM/Cpu needed.
    struct TestRam(Box<[u8; 0x10000]>);
    impl TestRam {
        fn new() -> Self {
            Self(Box::new([0; 0x10000]))
        }
        fn set(&mut self, addr: u16, v: u8) {
            self.0[addr as usize] = v;
        }
        fn fill_range(&mut self, start: u16, end: u16, v: u8) {
            for a in start..end {
                self.0[a as usize] = v;
            }
        }
    }
    impl MemoryReader for TestRam {
        fn read_byte(&self, addr: u16) -> u8 {
            self.0[addr as usize]
        }
    }

    /// Standard CPC 464 mode-1 CRTC setup:
    /// R0=63, R1=40, R6=25, R9=7, R12/R13 = 0x3000 (phys 0xC000).
    fn setup_standard_crtc(crtc: &mut Crtc) {
        for (r, v) in [
            (0u8, 63u8),
            (1, 40),
            (4, 38),
            (6, 25),
            (9, 7),
            (12, 0x30),
            (13, 0x00),
            (8, 0),
        ] {
            crtc.write(0xBC00, r);
            crtc.write(0xBD00, v);
        }
    }

    #[test]
    fn new_video_has_blank_buffer() {
        let v = Video::new();
        assert_eq!(v.buffer().len(), WINDOW_WIDTH * WINDOW_HEIGHT);
        assert!(v.buffer().iter().all(|&p| p == 0));
    }

    #[test]
    fn tick_at_origin_writes_exactly_16_pixels() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        let ga = GateArray::new(); // mode 1, pen 0 = White
        let mut ram = TestRam::new();
        ram.set(0xC000, 0x00); // mode 1 decodes 0x00 -> [0,0,0,0] -> pen 0
        ram.set(0xC001, 0x00);

        video.tick(&crtc, &ga, &ram);

        let white = ga.ink_for_pen(0).color();
        for x in 0..16 {
            assert_eq!(video.buffer()[x], white, "x={}", x);
        }
        // Pixel 8 must be untouched (proves we wrote exactly 8 px, not more)
        assert_eq!(video.buffer()[16], 0, "x=16 should be untouched");
    }

    #[test]
    fn tick_with_dispen_false_writes_border_not_ram_data() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        crtc.write(0xBC00, 1);
        crtc.write(0xBD00, 1); // R1=1: only c0=0 has dispen

        // Make border Black; pen 0 stays White. RAM=0x00 would decode
        // to pen 0 (White) if dispen were true. So if we see Black, the
        // implementation correctly used the border color, not RAM.
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10); // select border (pen 16)
        ga.write(0x7F00, 0x40 | 20); // color 20 = Black
        let mut ram = TestRam::new();
        ram.set(0xC000, 0x00);
        ram.set(0xC001, 0x00);

        crtc.tick(); // c0: 0 -> 1; dispen now false

        video.tick(&crtc, &ga, &ram);

        let black = ga.ink_for_pen(16).color();
        for x in 16..32 {
            assert_eq!(video.buffer()[x], black, "x={} should be border", x);
        }
        for x in 0..16 {
            assert_eq!(video.buffer()[x], 0, "x={} should be untouched", x);
        }
    }

    #[test]
    fn tick_outside_buffer_width_does_not_write() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        for _ in 0..40 {
            crtc.tick();
        } // c0=40 -> pixel_x=320 (out)
        assert_eq!(crtc.c0(), 40);

        let ga = GateArray::new();
        let ram = TestRam::new();
        video.tick(&crtc, &ga, &ram);

        assert!(video.buffer().iter().all(|&p| p == 0));
    }

    #[test]
    fn tick_outside_buffer_height_does_not_write() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        // 200 scanlines * 64 chars = 12800 ticks -> pixel_y = 200 (out)
        for _ in 0..12800 {
            crtc.tick();
        }

        let ga = GateArray::new();
        let ram = TestRam::new();
        video.tick(&crtc, &ga, &ram);

        assert!(video.buffer().iter().all(|&p| p == 0));
    }

    #[test]
    fn forty_ticks_fill_first_scanline_in_mode1() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        let ga = GateArray::new();
        let ram = TestRam::new(); // all 0 -> pen 0 (White)

        for _ in 0..40 {
            video.tick(&crtc, &ga, &ram);
            crtc.tick();
        }

        let white = ga.ink_for_pen(0).color();
        for x in 0..WINDOW_WIDTH {
            assert_eq!(video.buffer()[x], white, "x={}", x);
        }
    }

    #[test]
    fn non_zero_ram_data_produces_non_white_pixels() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);

        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x03); // select pen 3
        ga.write(0x7F00, 0x40 | 20); // pen 3 = Black
        // 0xFF in mode 1 decodes to [3,3,3,3]

        let mut ram = TestRam::new();
        ram.set(0xC000, 0xFF);
        ram.set(0xC001, 0xFF);

        video.tick(&crtc, &ga, &ram);

        let black = ga.ink_for_pen(3).color();
        for x in 0..8 {
            assert_eq!(video.buffer()[x], black, "x={}", x);
        }
    }

    #[test]
    fn scanline_advances_after_full_line() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc); // R0=63 -> 64 chars per line

        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x03);
        ga.write(0x7F00, 0x40 | 20); // pen 3 = Black

        let mut ram = TestRam::new();
        // Scanline 0 phys 0xC000: 0x00 -> pen 0 (White)
        // Scanline 1 phys 0xC800: 0xFF -> pen 3 (Black)
        ram.fill_range(0xC000, 0xC050, 0x00);
        ram.fill_range(0xC800, 0xC850, 0xFF);

        // Complete scanline 0
        for _ in 0..64 {
            video.tick(&crtc, &ga, &ram);
            crtc.tick();
        }
        assert_eq!(crtc.c9(), 1);
        assert_eq!(crtc.c0(), 0);

        // First char of scanline 1
        video.tick(&crtc, &ga, &ram);

        let black = ga.ink_for_pen(3).color();
        let white = ga.ink_for_pen(0).color();
        for x in 0..8 {
            assert_eq!(
                video.buffer()[WINDOW_WIDTH + x],
                black,
                "scanline 1, x={}",
                x
            );
        }
        // Scanline 0 untouched (still White from earlier writes)
        assert_eq!(video.buffer()[0], white);
    }

    #[test]
    fn border_color_uses_pen_16_from_palette() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        crtc.write(0xBC00, 1);
        crtc.write(0xBD00, 0); // R1=0 -> dispen always false

        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10); // select border
        ga.write(0x7F00, 0x40 | 20); // Black

        let ram = TestRam::new();
        video.tick(&crtc, &ga, &ram);

        let black = ga.ink_for_pen(16).color();
        for x in 0..8 {
            assert_eq!(video.buffer()[x], black, "x={}", x);
        }
    }

    #[test]
    fn r8_border_force_renders_border_in_displayed_area() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        // R8 bits 5:4 = 11 -> crtc.dispen() returns false (force border)
        crtc.write(0xBC00, 8);
        crtc.write(0xBD00, 0x30);

        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10);
        ga.write(0x7F00, 0x40 | 20); // border = Black

        let mut ram = TestRam::new();
        ram.set(0xC000, 0x00); // would be pen 0 (White) if displayed

        video.tick(&crtc, &ga, &ram);

        let black = ga.ink_for_pen(16).color();
        for x in 0..8 {
            assert_eq!(video.buffer()[x], black, "x={}", x);
        }
    }

    #[test]
    fn full_frame_fills_visible_buffer_uniformly() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        let ga = GateArray::new();
        let ram = TestRam::new(); // 0x00 -> pen 0 (White)

        // 312 scanlines * 64 chars = 19968 ticks (one full frame)
        for _ in 0..19968 {
            video.tick(&crtc, &ga, &ram);
            crtc.tick();
        }

        let white = ga.ink_for_pen(0).color();
        for y in 0..WINDOW_HEIGHT {
            for x in 0..WINDOW_WIDTH {
                let idx = y * WINDOW_WIDTH + x;
                assert_eq!(video.buffer()[idx], white, "({}, {})", x, y);
            }
        }
    }

    #[test]
    fn clear_resets_buffer_to_zeros() {
        let mut video = Video::new();
        let mut crtc = Crtc::new();
        setup_standard_crtc(&mut crtc);
        let ga = GateArray::new();
        let ram = TestRam::new();

        video.tick(&crtc, &ga, &ram);
        assert_ne!(video.buffer()[0], 0);

        video.clear();
        assert!(video.buffer().iter().all(|&p| p == 0));
    }
}
