#[derive(Clone, Copy, Debug)]
pub enum Palette {
    White,
    SeaGreen,
    PastelYellow,
    Blue,
    Purple,
    Cyan,
    Pink,
    BrightYellow,
    BrightWhite,
    BrightRed,
    BrightMagenta,
    Orange,
    PastelMagenta,
    BrightGreen,
    BrightCyan,
    Black,
    BrightBlue,
    Green,
    SkyBlue,
    Magenta,
    PastelGreen,
    Lime,
    PastelCyan,
    Red,
    Mauve,
    Yellow,
    PastelBlue,
}

impl From<u8> for Palette {
    fn from(idx: u8) -> Self {
        match idx {
            0 => Self::White,
            1 => Self::White, // Duplicate
            2 => Self::SeaGreen,
            3 => Self::PastelYellow,
            4 => Self::Blue,
            5 => Self::Purple,
            6 => Self::Cyan,
            7 => Self::Pink,
            8 => Self::Purple,       // Duplicate
            9 => Self::PastelYellow, // Duplicate
            10 => Self::BrightYellow,
            11 => Self::BrightWhite,
            12 => Self::BrightRed,
            13 => Self::BrightMagenta,
            14 => Self::Orange,
            15 => Self::PastelMagenta,
            16 => Self::Blue,     // Duplicate
            17 => Self::SeaGreen, // Duplicate
            18 => Self::BrightGreen,
            19 => Self::BrightCyan,
            20 => Self::Black,
            21 => Self::BrightBlue,
            22 => Self::Green,
            23 => Self::SkyBlue,
            24 => Self::Magenta,
            25 => Self::PastelGreen,
            26 => Self::Lime,
            27 => Self::PastelCyan,
            28 => Self::Red,
            29 => Self::Mauve,
            30 => Self::Yellow,
            31 => Self::PastelBlue,
            _ => panic!("Unsupported color idx {idx}"),
        }
    }
}

impl Into<u8> for Palette {
    fn into(self) -> u8 {
        match self {
            Self::White => 0,
            Self::SeaGreen => 2,
            Self::PastelYellow => 3,
            Self::Blue => 4,
            Self::Purple => 5,
            Self::Cyan => 6,
            Self::Pink => 7,
            Self::BrightYellow => 10,
            Self::BrightWhite => 11,
            Self::BrightRed => 12,
            Self::BrightMagenta => 13,
            Self::Orange => 14,
            Self::PastelMagenta => 15,
            Self::BrightGreen => 18,
            Self::BrightCyan => 19,
            Self::Black => 20,
            Self::BrightBlue => 21,
            Self::Green => 22,
            Self::SkyBlue => 23,
            Self::Magenta => 24,
            Self::PastelGreen => 25,
            Self::Lime => 26,
            Self::PastelCyan => 27,
            Self::Red => 28,
            Self::Mauve => 29,
            Self::Yellow => 30,
            Self::PastelBlue => 31,
        }
    }
}

impl Palette {
    pub fn color(&self) -> u32 {
        match self {
            Self::Black => 0x000000,
            Self::Blue => 0x000080,
            Self::BrightBlue => 0x0000FF,
            Self::Red => 0x800000,
            Self::Magenta => 0x800080,
            Self::Mauve => 0x8000FF,
            Self::BrightRed => 0xFF0000,
            Self::Purple => 0xFF0080,
            Self::BrightMagenta => 0xFF00FF,
            Self::Green => 0x008000,
            Self::Cyan => 0x008080,
            Self::SkyBlue => 0x0080FF,
            Self::Yellow => 0x808000,
            Self::White => 0x808080,
            Self::PastelBlue => 0x8080FF,
            Self::Orange => 0xFF8000,
            Self::Pink => 0xFF8080,
            Self::PastelMagenta => 0xFF80FF,
            Self::BrightGreen => 0x00FF00,
            Self::SeaGreen => 0x00FF80,
            Self::BrightCyan => 0x00FFFF,
            Self::Lime => 0x80FF00,
            Self::PastelGreen => 0x80FF80,
            Self::PastelCyan => 0x80FFFF,
            Self::BrightYellow => 0xFFFF00,
            Self::PastelYellow => 0xFFFF80,
            Self::BrightWhite => 0xFFFFFF,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ScreenMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

impl From<u8> for ScreenMode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Mode0,
            1 => Self::Mode1,
            2 => Self::Mode2,
            3 => Self::Mode3,
            _ => panic!("Unsupported screen mode value {value}"),
        }
    }
}

pub struct GateArray {
    lower_rom_enabled: bool,
    upper_rom_enabled: bool,
    mode: ScreenMode,
    selected_pen: u8,
    palette: [Palette; 17],
    interrupt_counter: u8,
    interrupt_requested: bool,
    vsync: bool,
    vsync_counter: u8,
}

impl GateArray {
    pub fn new() -> Self {
        Self {
            lower_rom_enabled: true,
            // TODO: check if upper rom is enabled by default or not
            upper_rom_enabled: true,
            mode: ScreenMode::Mode1,
            selected_pen: 0,
            palette: [Palette::White; 17],
            interrupt_counter: 0,
            interrupt_requested: false,
            vsync: false,
            vsync_counter: 0,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        assert_eq!(addr >> 8, 0x7F);

        match value >> 6 {
            0b00 => self.select_pen(value),
            0b01 => self.select_color(value),
            0b10 => self.select_mode_and_rom(value),
            0b11 => self.select_ram_memory_management(value),
            _ => panic!("Impossible"),
        }
    }

    pub fn lower_rom_enabled(&self) -> bool {
        self.lower_rom_enabled
    }

    pub fn upper_rom_enabled(&self) -> bool {
        self.upper_rom_enabled
    }

    pub fn mode(&self) -> ScreenMode {
        self.mode
    }

    pub fn selected_pen(&self) -> u8 {
        self.selected_pen
    }

    pub fn ink_for_pen(&self, pen: u8) -> Palette {
        self.palette[pen as usize]
    }

    fn select_pen(&mut self, value: u8) {
        self.selected_pen = if (value & (1 << 4)) != 0 {
            16
        } else {
            value & 0x0F
        };
    }

    fn select_color(&mut self, value: u8) {
        self.palette[self.selected_pen as usize] = Palette::from(value & 0x1F);
    }

    fn select_mode_and_rom(&mut self, value: u8) {
        self.upper_rom_enabled = (value & (1 << 3)) == 0;
        self.lower_rom_enabled = (value & (1 << 2)) == 0;
        self.mode = ScreenMode::from(value & 0b11);

        if (value & (1 << 4)) != 0 {
            self.interrupt_counter = 0;
            self.interrupt_requested = false;
        }
    }

    fn select_ram_memory_management(&mut self, value: u8) {
        todo!()
    }

    pub fn hsync(&mut self) {
        if self.interrupt_counter == 52 {
            self.interrupt_counter = 0;
        }

        self.interrupt_counter += 1;
        if self.interrupt_counter == 52 {
            self.interrupt_requested = true;
        }
        if self.vsync && self.vsync_counter < 2 {
            self.vsync_counter += 1;
            if self.vsync_counter == 2 {
                self.interrupt_requested = self.interrupt_counter < 32;
                self.interrupt_counter = 0;
            }
        }
    }

    pub fn interrupt_requested(&self) -> bool {
        self.interrupt_requested
    }

    pub fn acknowledge_interrupt(&mut self) {
        self.interrupt_counter &= !(1 << 5);
        self.interrupt_requested = false;
    }

    pub fn set_vsync(&mut self, value: bool) {
        if value {
            if !self.vsync {
                self.vsync = true;
                self.vsync_counter = 0;
            }
        } else {
            self.vsync = false;
            self.vsync_counter = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let ga = GateArray::new();
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
        assert_eq!(ga.mode(), ScreenMode::Mode1);
        assert_eq!(ga.selected_pen(), 0);
        for pen in 0..=15u8 {
            let color: u8 = ga.ink_for_pen(pen).into();
            assert_eq!(color, 0, "Pen {} should default to White", pen);
        }
    }

    #[test]
    fn test_rom_configuration_selection() {
        let mut ga = GateArray::new();

        ga.write(0x7F00, 0x8C);
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        ga.write(0x7F00, 0x80);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());

        ga.write(0x7F00, 0x88);
        assert!(ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        ga.write(0x7F00, 0x84);
        assert!(!ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_other_registers_do_not_affect_rom_state() {
        let mut ga = GateArray::new();

        ga.write(0x7F00, 0x80);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());

        // Select Pen 12 (Bit 7 = 0, Bit 6 = 0)
        // Written value: 0b0000_1100 (0x0C) -> Note that bits 2 and 3 are set here,
        // but because this targets the Pen register, the ROM state should not change.
        ga.write(0x7F00, 0x0C);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());

        // Select Color (Bit 7 = 0, Bit 6 = 1)
        // Written value: 0b0100_1100 (0x4C)
        ga.write(0x7F00, 0x4C);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_address_decoding_valid() {
        let mut ga = GateArray::new();

        // Reset to enabled
        ga.write(0x7F00, 0x80);

        // 0x7FFF is a valid address (Bit 15 = 0, Bit 14 = 1)
        // Disable both ROMs (0x8C)
        ga.write(0x7FFF, 0x8C);
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        // 0x7FAA is also a valid address (Bit 15 = 0, Bit 14 = 1)
        // Enable both ROMs (0x80)
        ga.write(0x7FAA, 0x80);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_pen_select_does_not_affect_rom_state() {
        let mut ga = GateArray::new();

        // Disable both roms
        ga.write(0x7F00, 0x8C);

        ga.write(0x7F00, 0x10); // pen select (border)
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());
    }

    #[test]
    fn test_ink_color_write_does_not_affect_rom_state() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x15); // ink color value
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    // TODO: implement
    // #[test]
    // fn test_ram_config_write_does_not_affect_rom_state() {
    //     let mut ga = GateArray::new();
    //     ga.write(0x7F00, 0xC4); // RAM banking config
    //     assert!(ga.lower_rom_enabled());
    //     assert!(ga.upper_rom_enabled());
    // }

    #[test]
    fn test_default_mode_1() {
        let ga = GateArray::new();
        assert_eq!(ga.mode(), ScreenMode::Mode1);
    }

    #[test]
    fn test_set_mode_0() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x80);
        assert_eq!(ga.mode(), ScreenMode::Mode0);
    }

    #[test]
    fn test_set_mode_1() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x81);
        assert_eq!(ga.mode(), ScreenMode::Mode1);
    }

    #[test]
    fn test_set_mode_2() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x82);
        assert_eq!(ga.mode(), ScreenMode::Mode2);
    }

    #[test]
    fn test_set_mode_3() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x83);
        assert_eq!(ga.mode(), ScreenMode::Mode3);
    }

    // TODO: Implement RAM banking
    // #[test]
    // fn test_ram_config_values() {
    //     let mut ga = GateArray::new();
    //     for cfg in 0..=7u8 {
    //         ga.write(0x7F00, 0xC0 | (cfg << 2));
    //         assert_eq!(ga.ram_config(), cfg, "RAM config should be {}", cfg);
    //     }
    // }

    #[test]
    fn test_default_selected_pen() {
        let ga = GateArray::new();
        assert_eq!(ga.selected_pen, 0);
    }

    #[test]
    fn test_select_border_pen() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10);
        assert_eq!(ga.selected_pen(), 16);
    }

    #[test]
    fn test_set_ink_colors() {
        let mut ga = GateArray::new();
        for color in 0..=31u8 {
            if color == 1 || color == 8 || color == 9 || color == 16 || color == 17 {
                continue;
            }

            ga.write(0x7F00, 0x01); // select pen 1 first
            ga.write(0x7F00, 0x40 | color); // set ink
            let ink: u8 = ga.ink_for_pen(1).into();
            assert_eq!(ink, color);
        }
    }

    #[test]
    fn test_screen_mode_selection() {
        let mut ga = GateArray::new();

        // Write value: 0b1000_0000 (0x80)
        // Bits 7,6 = 10 (ROM/Mode register)
        // Bits 1,0 = 00 (Mode 0)
        ga.write(0x7F00, 0x80);
        assert_eq!(ga.mode(), ScreenMode::Mode0);

        // Write value: 0b1000_0001 (0x81)
        // Bits 1,0 = 01 (Mode 1)
        ga.write(0x7F00, 0x81);
        assert_eq!(ga.mode(), ScreenMode::Mode1);

        // Write value: 0b1000_0010 (0x82)
        // Bits 1,0 = 10 (Mode 2)
        ga.write(0x7F00, 0x82);
        assert_eq!(ga.mode(), ScreenMode::Mode2);

        // Write value: 0b1000_0011 (0x83)
        // Bits 1,0 = 11 (Mode 3)
        ga.write(0x7F00, 0x83);
        assert_eq!(ga.mode(), ScreenMode::Mode3);
    }

    #[test]
    fn test_pen_selection() {
        let mut ga = GateArray::new();

        // Write value: 0b0000_0101 (0x05)
        // Bits 7,6 = 00 (Pen selection)
        // Bit 4 = 0 (Normal pen)
        // Bits 3-0 = 0101 (Pen 5)
        ga.write(0x7F00, 0x05);
        assert_eq!(ga.selected_pen(), 5);

        // Write value: 0b0000_1111 (0x0F)
        // Bits 3-0 = 1111 (Pen 15)
        ga.write(0x7F00, 0x0F);
        assert_eq!(ga.selected_pen(), 15);

        // Write value: 0b0001_0000 (0x10)
        // Bit 4 = 1 (Select Border).
        // Note: You may decide to represent the border as Pen 16 or via a separate field.
        // This test assumes a common design of mapping the border to index 16.
        ga.write(0x7F00, 0x10);
        assert_eq!(ga.selected_pen(), 16);
    }

    #[test]
    fn test_color_selection_updates_ink() {
        let mut ga = GateArray::new();

        // Step 1: Select Pen 3
        ga.write(0x7F00, 0x03);
        assert_eq!(ga.selected_pen(), 3);

        // Step 2: Assign Color 9 (Green) to the selected pen (Pen 3)
        // Write value: 0b0100_1001 (0x49)
        // Bits 7,6 = 01 (Color selection)
        // Bits 4-0 = 01001 (Color Index 3)
        ga.write(0x7F00, 0x43);

        // Assert that Pen 3 is now Green
        let pen_3_color: u8 = ga.ink_for_pen(3).into();
        assert_eq!(pen_3_color, 3);

        // Step 3: Select Pen 7
        ga.write(0x7F00, 0x07);

        // Assign Color 26 (Bright White) to Pen 7
        // Write value: 0b0101_1010 (0x5A) -> Bits 4-0 = 26
        ga.write(0x7F00, 0x5A);

        let pen_7_color: u8 = ga.ink_for_pen(7).into();
        assert_eq!(pen_7_color, 26);

        // Ensure Pen 3 was not overwritten and remains Green
        let pen_3_color_still: u8 = ga.ink_for_pen(3).into();
        assert_eq!(pen_3_color_still, 3);
    }

    #[test]
    fn test_palette_color_mapping() {
        // Test a few known hardware colors to ensure matching values are returned.
        assert_eq!(Palette::Black.color(), 0x000000);
        assert_eq!(Palette::BrightRed.color(), 0xFF0000);
        assert_eq!(Palette::BrightWhite.color(), 0xFFFFFF);
    }

    #[test]
    fn test_palette_from_u8_unique_indices() {
        let expected: [(u8, Palette); 27] = [
            (0, Palette::White),
            (2, Palette::SeaGreen),
            (3, Palette::PastelYellow),
            (4, Palette::Blue),
            (5, Palette::Purple),
            (6, Palette::Cyan),
            (7, Palette::Pink),
            (10, Palette::BrightYellow),
            (11, Palette::BrightWhite),
            (12, Palette::BrightRed),
            (13, Palette::BrightMagenta),
            (14, Palette::Orange),
            (15, Palette::PastelMagenta),
            (18, Palette::BrightGreen),
            (19, Palette::BrightCyan),
            (20, Palette::Black),
            (21, Palette::BrightBlue),
            (22, Palette::Green),
            (23, Palette::SkyBlue),
            (24, Palette::Magenta),
            (25, Palette::PastelGreen),
            (26, Palette::Lime),
            (27, Palette::PastelCyan),
            (28, Palette::Red),
            (29, Palette::Mauve),
            (30, Palette::Yellow),
            (31, Palette::PastelBlue),
        ];
        for (idx, expected_palette) in expected {
            assert_eq!(Palette::from(idx).color(), expected_palette.color());
            let back: u8 = Palette::from(idx).into();
            assert_eq!(
                back, idx,
                "Round-trip u8 -> Palette -> u8 failed for idx {}",
                idx
            );
        }
    }

    #[test]
    fn test_palette_from_u8_duplicate_indices() {
        // Hardware duplicates — these indices alias to existing colors
        assert_eq!(Palette::from(1).color(), Palette::White.color());
        assert_eq!(Palette::from(8).color(), Palette::Purple.color());
        assert_eq!(Palette::from(9).color(), Palette::PastelYellow.color());
        assert_eq!(Palette::from(16).color(), Palette::Blue.color());
        assert_eq!(Palette::from(17).color(), Palette::SeaGreen.color());
    }

    #[test]
    fn test_palette_all_27_unique_colors_have_distinct_rgb() {
        let all = [
            Palette::Black,
            Palette::Blue,
            Palette::BrightBlue,
            Palette::Red,
            Palette::Magenta,
            Palette::Mauve,
            Palette::BrightRed,
            Palette::Purple,
            Palette::BrightMagenta,
            Palette::Green,
            Palette::Cyan,
            Palette::SkyBlue,
            Palette::Yellow,
            Palette::White,
            Palette::PastelBlue,
            Palette::Orange,
            Palette::Pink,
            Palette::PastelMagenta,
            Palette::BrightGreen,
            Palette::SeaGreen,
            Palette::BrightCyan,
            Palette::Lime,
            Palette::PastelGreen,
            Palette::PastelCyan,
            Palette::BrightYellow,
            Palette::PastelYellow,
            Palette::BrightWhite,
        ];
        let mut seen = std::collections::HashSet::new();
        for p in all {
            assert!(seen.insert(p.color()), "Duplicate RGB for {:?}", p);
        }
        assert_eq!(seen.len(), 27);
    }

    #[test]
    fn test_palette_uses_only_cpc_3_level_rgb() {
        // CPC palette uses only 0, 0x80, 0xFF per channel
        for color in 0..=31u8 {
            let rgb = Palette::from(color).color();
            let r = (rgb >> 16) & 0xFF;
            let g = (rgb >> 8) & 0xFF;
            let b = rgb & 0xFF;
            for c in [r, g, b] {
                assert!(
                    c == 0 || c == 0x80 || c == 0xFF,
                    "Color {} has invalid channel {:#x}",
                    color,
                    c
                );
            }
        }
    }

    #[test]
    fn test_palette_known_rgb_values() {
        let expected: [(Palette, u32); 27] = [
            (Palette::Black, 0x000000),
            (Palette::Blue, 0x000080),
            (Palette::BrightBlue, 0x0000FF),
            (Palette::Red, 0x800000),
            (Palette::Magenta, 0x800080),
            (Palette::Mauve, 0x8000FF),
            (Palette::BrightRed, 0xFF0000),
            (Palette::Purple, 0xFF0080),
            (Palette::BrightMagenta, 0xFF00FF),
            (Palette::Green, 0x008000),
            (Palette::Cyan, 0x008080),
            (Palette::SkyBlue, 0x0080FF),
            (Palette::Yellow, 0x808000),
            (Palette::White, 0x808080),
            (Palette::PastelBlue, 0x8080FF),
            (Palette::Orange, 0xFF8000),
            (Palette::Pink, 0xFF8080),
            (Palette::PastelMagenta, 0xFF80FF),
            (Palette::BrightGreen, 0x00FF00),
            (Palette::SeaGreen, 0x00FF80),
            (Palette::BrightCyan, 0x00FFFF),
            (Palette::Lime, 0x80FF00),
            (Palette::PastelGreen, 0x80FF80),
            (Palette::PastelCyan, 0x80FFFF),
            (Palette::BrightYellow, 0xFFFF00),
            (Palette::PastelYellow, 0xFFFF80),
            (Palette::BrightWhite, 0xFFFFFF),
        ];
        for (p, rgb) in expected {
            assert_eq!(p.color(), rgb, "RGB mismatch for {:?}", p);
        }
    }

    #[test]
    fn test_screen_mode_from_u8_all_modes() {
        assert_eq!(ScreenMode::from(0), ScreenMode::Mode0);
        assert_eq!(ScreenMode::from(1), ScreenMode::Mode1);
        assert_eq!(ScreenMode::from(2), ScreenMode::Mode2);
        assert_eq!(ScreenMode::from(3), ScreenMode::Mode3);
    }

    #[test]
    fn test_select_each_pen_individually() {
        let mut ga = GateArray::new();
        for pen in 0..=15u8 {
            ga.write(0x7F00, pen);
            assert_eq!(ga.selected_pen(), pen);
        }
    }

    #[test]
    fn test_select_border_pen_0x10() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10);
        assert_eq!(ga.selected_pen(), 16);
    }

    #[test]
    fn test_select_border_pen_0x1f() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x1F);
        assert_eq!(ga.selected_pen(), 16);
    }

    #[test]
    fn test_pen_select_bit5_ignored_when_bit4_clear() {
        // Hardware: bit 4 = border select, bit 5-7 = unused
        // 0x20-0x2F should select pen 0-15 (bit 4 = 0)
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x20);
        assert_eq!(ga.selected_pen(), 0, "Bit 5 should be ignored");
        ga.write(0x7F00, 0x25);
        assert_eq!(ga.selected_pen(), 5, "Bit 5 should be ignored");
        ga.write(0x7F00, 0x2F);
        assert_eq!(ga.selected_pen(), 15, "Bit 5 should be ignored");
    }

    #[test]
    fn test_pen_select_border_with_bit5_set() {
        // 0x30: bit 4 = 1 (border), bit 5 = 1 (ignored)
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x30);
        assert_eq!(ga.selected_pen(), 16);
        ga.write(0x7F00, 0x3F);
        assert_eq!(ga.selected_pen(), 16);
    }

    #[test]
    fn test_set_color_all_unique_values() {
        let mut ga = GateArray::new();
        let unique: [u8; 27] = [
            0, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
            28, 29, 30, 31,
        ];
        for (i, &color) in unique.iter().enumerate() {
            let pen = (i % 16) as u8;
            ga.write(0x7F00, pen);
            ga.write(0x7F00, 0x40 | color);
            let ink: u8 = ga.ink_for_pen(pen).into();
            assert_eq!(ink, color, "Failed to set color {} on pen {}", color, pen);
        }
    }

    #[test]
    fn test_set_color_duplicate_indices() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x00);

        ga.write(0x7F00, 0x41); // idx 1 -> White
        assert_eq!(ga.ink_for_pen(0).color(), Palette::White.color());
        ga.write(0x7F00, 0x48); // idx 8 -> Purple
        assert_eq!(ga.ink_for_pen(0).color(), Palette::Purple.color());
        ga.write(0x7F00, 0x49); // idx 9 -> PastelYellow
        assert_eq!(ga.ink_for_pen(0).color(), Palette::PastelYellow.color());
        ga.write(0x7F00, 0x50); // idx 16 -> Blue
        assert_eq!(ga.ink_for_pen(0).color(), Palette::Blue.color());
        ga.write(0x7F00, 0x51); // idx 17 -> SeaGreen
        assert_eq!(ga.ink_for_pen(0).color(), Palette::SeaGreen.color());
    }

    #[test]
    fn test_color_uses_only_low_5_bits() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x00);

        // 0x60 = 0x40 | 0x20  -> color = 0x20 & 0x1F = 0
        ga.write(0x7F00, 0x60);
        let c: u8 = ga.ink_for_pen(0).into();
        assert_eq!(c, 0);

        // 0x7F = 0x40 | 0x3F  -> color = 0x3F & 0x1F = 31
        ga.write(0x7F00, 0x7F);
        let c: u8 = ga.ink_for_pen(0).into();
        assert_eq!(c, 31);
    }

    #[test]
    fn test_color_applied_only_to_currently_selected_pen() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x07); // pen 7
        ga.write(0x7F00, 0x4C); // color 12
        let c7: u8 = ga.ink_for_pen(7).into();
        assert_eq!(c7, 12);
        // Pen 0 untouched
        let c0: u8 = ga.ink_for_pen(0).into();
        assert_eq!(c0, 0);
    }

    #[test]
    fn test_color_persistence_across_pen_changes() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x02);
        ga.write(0x7F00, 0x4A); // color 10
        ga.write(0x7F00, 0x05);
        ga.write(0x7F00, 0x4F); // color 15
        let c2: u8 = ga.ink_for_pen(2).into();
        let c5: u8 = ga.ink_for_pen(5).into();
        assert_eq!(c2, 10);
        assert_eq!(c5, 15);
    }

    #[test]
    fn test_selected_pen_unchanged_by_color_write() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x05);
        ga.write(0x7F00, 0x40);
        assert_eq!(ga.selected_pen(), 5);
    }

    #[test]
    fn test_border_pen_color_setting() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x10); // select border
        ga.write(0x7F00, 0x42); // color 2 (SeaGreen)
        let color: u8 = ga.ink_for_pen(16).into();
        assert_eq!(color, 2);
    }

    #[test]
    fn test_border_color_independent_from_pen_0() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x00);
        ga.write(0x7F00, 0x4C); // pen 0 = BrightRed (12)
        ga.write(0x7F00, 0x10);
        ga.write(0x7F00, 0x44); // border = Blue (4)
        let c0: u8 = ga.ink_for_pen(0).into();
        let c16: u8 = ga.ink_for_pen(16).into();
        assert_eq!(c0, 12);
        assert_eq!(c16, 4);
    }

    #[test]
    fn test_mode_setting_persists_across_pen_writes() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x82); // mode 2
        ga.write(0x7F00, 0x00); // pen select
        assert_eq!(ga.mode(), ScreenMode::Mode2);
    }

    #[test]
    fn test_mode_with_rom_settings_combined() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x8C);
        assert_eq!(ga.mode(), ScreenMode::Mode0);
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        ga.write(0x7F00, 0x89);
        assert_eq!(ga.mode(), ScreenMode::Mode1);
        assert!(ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        ga.write(0x7F00, 0x82);
        assert_eq!(ga.mode(), ScreenMode::Mode2);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());

        ga.write(0x7F00, 0x83);
        assert_eq!(ga.mode(), ScreenMode::Mode3);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_mode_register_bit5_ignored() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0xA0); // 0x80 | 0x20
        assert_eq!(ga.mode(), ScreenMode::Mode0);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_interrupt_reset_bit_does_not_change_mode_or_rom() {
        // Bit 4 of mode register resets interrupt counter — should not affect mode/ROM
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x82); // mode 2, ROMs on
        ga.write(0x7F00, 0x92); // mode 2 + bit 4 (interrupt reset)
        assert_eq!(ga.mode(), ScreenMode::Mode2);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());

        ga.write(0x7F00, 0x9C); // mode 0 + bit 4 + both ROMs off
        assert_eq!(ga.mode(), ScreenMode::Mode0);
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());
    }

    #[test]
    fn test_mode_change_does_not_reset_palette() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x00);
        ga.write(0x7F00, 0x4C); // color 12
        ga.write(0x7F00, 0x82); // mode 2
        let c0: u8 = ga.ink_for_pen(0).into();
        assert_eq!(c0, 12);
    }

    #[test]
    fn test_mode_change_does_not_reset_selected_pen() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x05);
        ga.write(0x7F00, 0x82);
        assert_eq!(ga.selected_pen(), 5);
    }

    #[test]
    fn test_rom_change_does_not_reset_palette() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x02);
        ga.write(0x7F00, 0x4A); // color 10
        ga.write(0x7F00, 0x8C); // disable ROMs
        let c2: u8 = ga.ink_for_pen(2).into();
        assert_eq!(c2, 10);
    }

    #[test]
    fn test_color_change_does_not_affect_mode_or_rom() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x82); // mode 2
        ga.write(0x7F00, 0x00);
        ga.write(0x7F00, 0x4C);
        assert_eq!(ga.mode(), ScreenMode::Mode2);
        assert!(ga.lower_rom_enabled());
        assert!(ga.upper_rom_enabled());
    }

    #[test]
    fn test_pen_change_does_not_affect_palette_contents() {
        let mut ga = GateArray::new();
        ga.write(0x7F00, 0x03);
        ga.write(0x7F00, 0x4E); // color 14
        ga.write(0x7F00, 0x0A); // select different pen
        let c3: u8 = ga.ink_for_pen(3).into();
        assert_eq!(c3, 14);
    }

    #[test]
    fn test_all_valid_addresses_in_0x7f_range() {
        let mut ga = GateArray::new();
        for addr in [0x7F00u16, 0x7F01, 0x7F7F, 0x7F80, 0x7FFE, 0x7FFF] {
            ga.write(addr, 0x80);
        }
        assert_eq!(ga.mode(), ScreenMode::Mode0);
    }

    #[test]
    fn test_typical_initialization_sequence() {
        let mut ga = GateArray::new();

        ga.write(0x7F00, 0x80); // mode 0
        assert_eq!(ga.mode(), ScreenMode::Mode0);

        ga.write(0x7F00, 0x00);
        ga.write(0x7F00, 0x54); // pen 0 = Black (20)
        ga.write(0x7F00, 0x01);
        ga.write(0x7F00, 0x4C); // pen 1 = BrightRed (12)
        ga.write(0x7F00, 0x10);
        ga.write(0x7F00, 0x55); // border = BrightBlue (21)

        let c0: u8 = ga.ink_for_pen(0).into();
        let c1: u8 = ga.ink_for_pen(1).into();
        let c16: u8 = ga.ink_for_pen(16).into();
        assert_eq!(c0, 20);
        assert_eq!(c1, 12);
        assert_eq!(c16, 21);

        ga.write(0x7F00, 0x8C); // disable ROMs
        assert!(!ga.lower_rom_enabled());
        assert!(!ga.upper_rom_enabled());

        // Palette preserved across mode/ROM writes
        let c0: u8 = ga.ink_for_pen(0).into();
        let c1: u8 = ga.ink_for_pen(1).into();
        let c16: u8 = ga.ink_for_pen(16).into();
        assert_eq!(c0, 20);
        assert_eq!(c1, 12);
        assert_eq!(c16, 21);
    }

    #[test]
    fn test_repeated_writes_same_value_idempotent() {
        let mut ga = GateArray::new();
        for _ in 0..10 {
            ga.write(0x7F00, 0x82);
        }
        assert_eq!(ga.mode(), ScreenMode::Mode2);
    }

    #[test]
    fn test_full_palette_overwrite() {
        let mut ga = GateArray::new();
        let colors: [u8; 16] = [
            20, 12, 4, 14, // black, bright red, blue, orange
            26, 11, 19, 2, // lime, bright white, bright cyan, sea green
            28, 30, 22, 24, // red, yellow, green, magenta
            21, 23, 6, 0, // bright blue, sky blue, cyan, white
        ];
        for (pen, &color) in colors.iter().enumerate() {
            ga.write(0x7F00, pen as u8);
            ga.write(0x7F00, 0x40 | color);
        }
        for (pen, &expected) in colors.iter().enumerate() {
            let color: u8 = ga.ink_for_pen(pen as u8).into();
            assert_eq!(color, expected, "Pen {} mismatch", pen);
        }
    }

    #[test]
    fn test_border_pen_color_assignment() {
        let mut ga = GateArray::new();

        // Select border pen (index 16)
        ga.write(0x7F00, 0x10);
        assert_eq!(ga.selected_pen(), 16);

        // Assign color 20 (Black) to the border
        ga.write(0x7F00, 0x40 | 20);

        // Retrieve the color of the border pen
        let border_color: u8 = ga.ink_for_pen(16).into();
        assert_eq!(border_color, 20);
    }

    #[test]
    fn test_interrupt_counter_increment_and_trigger() {
        let mut ga = GateArray::new();

        // Simulate 51 HSYNC pulses (no interrupt yet)
        for _ in 0..51 {
            ga.hsync();
            assert!(!ga.interrupt_requested());
        }

        // The 52nd HSYNC should trigger the interrupt request
        ga.hsync();
        assert!(ga.interrupt_requested());
        assert_eq!(ga.interrupt_counter, 52);

        ga.hsync();
        assert!(ga.interrupt_requested());
        assert_eq!(ga.interrupt_counter, 1);
    }

    #[test]
    fn test_interrupt_acknowledge() {
        let mut ga = GateArray::new();

        // Bring counter to 52 to trigger interrupt
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        // Acknowledge the interrupt
        ga.acknowledge_interrupt();
        assert!(!ga.interrupt_requested());

        // After acknowledgement, the counter's 5th bit (value 32) is cleared.
        // The counter goes from 52 to 20.
        // It should now take 32 more HSYNCs (52 - 20) to trigger another interrupt.
        for _ in 0..31 {
            ga.hsync();
            assert!(!ga.interrupt_requested());
        }

        ga.hsync();
        assert!(ga.interrupt_requested());
    }

    #[test]
    fn test_interrupt_manual_reset() {
        let mut ga = GateArray::new();

        // Increment counter partially
        for _ in 0..30 {
            ga.hsync();
        }

        // Write to Screen/ROM selection register with bit 4 set to 1 (0x90)
        // This should reset the counter to 0.
        ga.write(0x7F00, 0x90);

        // Since the counter is reset to 0, it should take a full 52 HSYNCs to trigger
        for _ in 0..51 {
            ga.hsync();
            assert!(!ga.interrupt_requested());
        }

        ga.hsync();
        assert!(ga.interrupt_requested());
    }

    #[test]
    fn test_hsync_no_interrupt_initially() {
        let ga = GateArray::new();
        assert!(
            !ga.interrupt_requested(),
            "GateArray should not request interrupt on creation"
        );
    }

    #[test]
    fn test_hsync_does_not_trigger_interrupt_before_52() {
        let mut ga = GateArray::new();
        for _ in 0..51 {
            ga.hsync();
        }
        assert!(
            !ga.interrupt_requested(),
            "Interrupt should not be requested before 52 HSYNCs"
        );
    }

    #[test]
    fn test_hsync_triggers_interrupt_at_exactly_52() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(
            ga.interrupt_requested(),
            "Interrupt should be requested after exactly 52 HSYNCs"
        );
    }

    #[test]
    fn test_acknowledge_interrupt_clears_request() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        ga.acknowledge_interrupt();
        assert_eq!(ga.interrupt_counter, 20);
        assert!(
            !ga.interrupt_requested(),
            "Interrupt request should be cleared after acknowledgement"
        );
    }

    #[test]
    fn test_hsync_interrupt_stays_pending_until_acknowledged() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        // Continue calling hsync - interrupt should stay requested
        for _ in 0..10 {
            ga.hsync();
            assert!(
                ga.interrupt_requested(),
                "Interrupt should remain pending until explicitly acknowledged"
            );
        }
    }

    #[test]
    fn test_vsync_clears_pending_interrupt() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        // Pass few more hsync, to get counter closer to 30
        for _ in 0..30 {
            ga.hsync();
        }
        ga.set_vsync(true);
        assert!(
            ga.interrupt_requested(),
            "VSYNC should clear a pending interrupt request only after 2 HSYNC"
        );

        ga.hsync();
        assert!(ga.interrupt_requested(), "1 HSYNC afte VSYNC is not enough");

        ga.hsync();
        assert_eq!(ga.interrupt_counter, 0, "Counter is reset");
        assert!(
            !ga.interrupt_requested(),
            "VSYNC should clear a pending interrupt request after 2 HSYNC"
        );
    }

    #[test]
    fn test_vsync_before_32_hsync() {
        let mut ga = GateArray::new();
        for _ in 0..29 {
            ga.hsync();
        }

        ga.set_vsync(true);
        ga.hsync();
        ga.hsync();

        assert_eq!(ga.interrupt_counter, 0, "Counter is reset");
        assert!(
            ga.interrupt_requested(),
            "VSYNC should set interrupt, because counter was 31"
        );
    }

    #[test]
    fn test_mode_register_bit4_clears_pending_interrupt() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        // Write 0x90 (0x80 | 0x10) -> Mode register with bit 4 set
        ga.write(0x7F00, 0x90);
        assert!(
            !ga.interrupt_requested(),
            "Writing to mode register with bit 4 set should clear interrupt"
        );
    }

    #[test]
    fn test_mode_register_bit4_resets_hsync_counter() {
        let mut ga = GateArray::new();

        // Advance 50 hsyncs
        for _ in 0..50 {
            ga.hsync();
        }

        // Reset counter via mode register bit 4
        ga.write(0x7F00, 0x90);

        // 10 more hsyncs (would be 60 total, triggering if not reset)
        for _ in 0..10 {
            ga.hsync();
        }
        assert!(
            !ga.interrupt_requested(),
            "Counter should have been reset by mode register write"
        );

        // 41 more hsyncs (total 51 since reset) -> no interrupt
        for _ in 0..41 {
            ga.hsync();
        }
        assert!(!ga.interrupt_requested());

        // 52nd hsync since reset -> interrupt
        ga.hsync();
        assert!(ga.interrupt_requested());
    }

    #[test]
    fn test_mode_register_without_bit4_does_not_clear_interrupt() {
        let mut ga = GateArray::new();
        for _ in 0..52 {
            ga.hsync();
        }
        assert!(ga.interrupt_requested());

        // Write 0x80 (bit 4 is 0) -> should NOT clear interrupt
        ga.write(0x7F00, 0x80);
        assert!(
            ga.interrupt_requested(),
            "Writing to mode register without bit 4 should not clear interrupt"
        );
    }

    #[test]
    fn test_vsync_reset_execution() {
        let mut ga = GateArray::new();

        // Advance counter
        for _ in 0..40 {
            ga.hsync();
        }

        // VSYNC goes active
        ga.set_vsync(true);

        // First HSYNC during VSYNC
        ga.hsync();

        // Second HSYNC during VSYNC: This should trigger the reset of the counter to 0
        ga.hsync();

        // Since it reset to 0, it should now require 52 HSYNCs to trigger an interrupt,
        // rather than the 12 it originally needed.
        for _ in 0..51 {
            ga.hsync();
            assert!(!ga.interrupt_requested());
        }
        ga.hsync();
        assert!(ga.interrupt_requested());
    }

    #[test]
    fn test_vsync_clearing() {
        let mut ga = GateArray::new();
        ga.set_vsync(true);
        ga.hsync();

        assert_eq!(ga.vsync, true);
        assert_eq!(ga.vsync_counter, 1);

        ga.set_vsync(true);
        assert_eq!(ga.vsync, true);
        assert_eq!(ga.vsync_counter, 1);

        ga.set_vsync(false);
        assert_eq!(ga.vsync, false);
        assert_eq!(ga.vsync_counter, 0);
    }

    #[test]
    fn test_vsync_continuous_for_multiple_lines() {
        let mut ga = GateArray::new();
        ga.set_vsync(true);

        ga.hsync();
        ga.hsync();

        assert_eq!(ga.vsync, true);
        assert_eq!(ga.vsync_counter, 2);
        assert_eq!(ga.interrupt_requested(), true);

        for _ in 0..6 {
            ga.set_vsync(true);
            ga.hsync();
        }

        assert_eq!(ga.vsync, true);
        assert_eq!(ga.vsync_counter, 2);
        assert_eq!(ga.interrupt_requested(), true);
    }
}
