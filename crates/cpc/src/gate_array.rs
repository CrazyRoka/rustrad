#[derive(Clone, Copy)]
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
    fn get_color(&self) -> u32 {
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
    palette: [Palette; 16],
}

impl GateArray {
    pub fn new() -> Self {
        Self {
            lower_rom_enabled: true,
            // TODO: check if upper rom is enabled by default or not
            upper_rom_enabled: true,
            mode: ScreenMode::Mode1,
            selected_pen: 0,
            palette: [Palette::White; 16],
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        todo!();
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
        match value {
            0x00..=0x0F => self.selected_pen = value,
            0x10..=0xFF => self.selected_pen = 16,
        }
    }

    fn select_color(&mut self, value: u8) {
        self.palette[self.selected_pen as usize] = Palette::from(value & 0x1F);
    }

    fn select_mode_and_rom(&mut self, value: u8) {
        // TODO: handle other bits
        // TODO: handle interrupts
        self.upper_rom_enabled = (value & (1 << 3)) == 0;
        self.lower_rom_enabled = (value & (1 << 2)) == 0;
        self.mode = ScreenMode::from(value & 0b11);
    }

    fn select_ram_memory_management(&mut self, value: u8) {
        todo!()
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

    // TODO: implement
    // #[test]
    // fn test_read_returns_floating_bus() {
    //     let ga = GateArray::new();
    //     // GA has no readable registers. Reading should return 0
    //     assert_eq!(ga.read(0x7F00), 0);
    // }

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
        assert_eq!(Palette::Black.get_color(), 0x000000);
        assert_eq!(Palette::BrightRed.get_color(), 0xFF0000);
        assert_eq!(Palette::BrightWhite.get_color(), 0xFFFFFF);
    }
}
