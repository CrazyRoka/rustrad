use crate::{TapePlayer, keyboard::Keyboard};

#[derive(PartialEq, Eq, Debug)]
enum PpiDirection {
    Input,
    Output,
}

#[derive(PartialEq, Eq, Debug)]
enum PsgBusFunction {
    Inactive,
    ReadRegister,
    WriteRegister,
    SelectRegister,
}

pub struct Ppi {
    vsync_active: bool,
    port_a_latch: u8,
    port_a_direction: PpiDirection,
    port_c_latch: u8,
    screen_frequency_50hz: bool,
    parallel_port_busy: bool,
    exp_present: bool,
    manufacturer_jumper: u8,
    psg_selected_register: u8,
    keyboard: Keyboard,
    tape: Option<TapePlayer>,
}

impl Ppi {
    pub fn new() -> Self {
        Self {
            vsync_active: false,
            port_a_latch: 0xFF,
            port_a_direction: PpiDirection::Input,
            port_c_latch: 0xFF,
            screen_frequency_50hz: true,
            parallel_port_busy: false,
            exp_present: true,
            manufacturer_jumper: 0b111,
            psg_selected_register: 0,
            keyboard: Keyboard::new(),
            tape: None,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr >> 8 {
            // TODO: handle
            0xF4 => self.read_port_a(),
            0xF5 => self.read_port_b(),
            0xF6 => self.read_port_c(),
            _ => todo!("Implement port {:#04X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr >> 8 {
            // TODO: handle
            0xF4 => self.write_port_a(value),
            0xF6 => self.write_port_c(value),
            0xF7 => self.write_control_register(value),
            _ => {
                todo!(
                    "Unexpected PPI write at {:#04X} with value {:#02X}",
                    addr,
                    value
                );
            }
        }
    }

    pub fn keyboard_mut(&mut self) -> &mut Keyboard {
        &mut self.keyboard
    }

    fn read_port_a(&self) -> u8 {
        match self.port_a_direction {
            PpiDirection::Output => self.port_a_latch,
            PpiDirection::Input => match self.psg_selected_register {
                14 if self.psg_bus_function() == PsgBusFunction::ReadRegister => {
                    self.keyboard.read_row(self.keyboard_row())
                }
                _ => {
                    println!(
                        "Unexpected PPI Port A read with register {}",
                        self.psg_selected_register
                    );
                    0xFF
                }
            },
        }
    }

    fn read_port_b(&self) -> u8 {
        const CASSETTE_READ: u8 = 1 << 7;
        const PARALLEL_PORT_BUSY: u8 = 1 << 6;
        const EXP: u8 = 1 << 5;
        const SCREEN_FREQUENCY: u8 = 1 << 4;
        const MANUFACTURER_JUMPER: u8 = (1 << 3) | (1 << 2) | (1 << 1);
        const VSYNC: u8 = 1 << 0;

        (if self.cassette_read_data() {
            CASSETTE_READ
        } else {
            0
        }) + if self.parallel_port_busy {
            0
        } else {
            PARALLEL_PORT_BUSY
        } + if self.exp_present { EXP } else { 0 }
            + if self.screen_frequency_50hz {
                SCREEN_FREQUENCY
            } else {
                0
            }
            + (((self.manufacturer_jumper & 0b111) << 1) & MANUFACTURER_JUMPER)
            + if self.vsync_active { VSYNC } else { 0 }
    }

    fn read_port_c(&self) -> u8 {
        self.port_c_latch
    }

    fn write_port_a(&mut self, value: u8) {
        match self.port_a_direction {
            PpiDirection::Output => {
                self.port_a_latch = value;
            }
            _ => {
                println!("Unexpected PPI Port A write at Input mode")
            }
        }
    }

    fn write_port_c(&mut self, value: u8) {
        let motor_before = self.cassette_motor();
        self.port_c_latch = value;
        let motor_after = self.cassette_motor();

        if let Some(ref mut tape) = self.tape {
            if !motor_before && motor_after {
                tape.play();
            } else if motor_before && !motor_after {
                tape.stop();
            }
        }

        if self.psg_bus_function() == PsgBusFunction::SelectRegister {
            self.psg_selected_register = self.port_a_latch;
        }
    }

    fn write_control_register(&mut self, value: u8) {
        match value {
            0x00..=0x7F => {
                let bit = (value >> 1) & 0b111;
                let state = value & 0b1;
                let new_value = (self.port_c_latch & !(1 << bit)) | (state << bit);
                self.write_port_c(new_value);
            }
            0x82 => self.port_a_direction = PpiDirection::Output,
            0x92 => self.port_a_direction = PpiDirection::Input,
            _ => panic!(
                "Unexpected PPI Control Register write with value {:#02X}",
                value
            ),
        }
    }

    fn psg_bdir(&self) -> bool {
        const PSG_BDIR: u8 = 1 << 7;
        (self.port_c_latch & PSG_BDIR) != 0
    }

    fn psg_bc1(&self) -> bool {
        const PSG_BC1: u8 = 1 << 6;
        (self.port_c_latch & PSG_BC1) != 0
    }

    fn cassette_write_data(&self) -> bool {
        const CASSETTE_WRITE: u8 = 1 << 5;
        (self.port_c_latch & CASSETTE_WRITE) != 0
    }

    fn cassette_read_data(&self) -> bool {
        if let Some(ref tape) = self.tape {
            tape.ear()
        } else {
            false
        }
    }

    fn cassette_motor(&self) -> bool {
        const CASSETTE_MOTOR: u8 = 1 << 4;
        (self.port_c_latch & CASSETTE_MOTOR) != 0
    }

    fn keyboard_row(&self) -> u8 {
        const KEYBOARD_ROW: u8 = (1 << 3) | (1 << 2) | (1 << 1) | (1 << 0);
        self.port_c_latch & KEYBOARD_ROW
    }

    fn psg_bus_function(&self) -> PsgBusFunction {
        match (self.psg_bdir(), self.psg_bc1()) {
            (false, false) => PsgBusFunction::Inactive,
            (false, true) => PsgBusFunction::ReadRegister,
            (true, false) => PsgBusFunction::WriteRegister,
            (true, true) => PsgBusFunction::SelectRegister,
        }
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        self.vsync_active = vsync;
    }

    pub fn load_tape(&mut self, tape: TapePlayer) {
        self.tape = Some(tape);
    }

    pub fn tape(&self) -> Option<&TapePlayer> {
        self.tape.as_ref()
    }

    pub fn tape_mut(&mut self) -> Option<&mut TapePlayer> {
        self.tape.as_mut()
    }

    pub fn unload_tape(&mut self) -> Option<TapePlayer> {
        self.tape.take()
    }

    pub fn tick_tape(&mut self, cycles: u64) {
        if let Some(ref mut tape) = self.tape {
            tape.advance(cycles);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TapePlayer, keyboard::CpcKey};

    use super::*;

    #[test]
    fn new_ppi_has_port_a_in_input_mode() {
        let ppi = Ppi::new();
        assert_eq!(ppi.port_a_direction, PpiDirection::Input);
    }

    #[test]
    fn new_ppi_port_a_reads_ff_when_input() {
        // Port A is bidirectional and starts as input => reads 0xFF
        let ppi = Ppi::new();
        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn new_ppi_port_c_reads_ff_when_input() {
        let ppi = Ppi::new();
        assert_eq!(ppi.read(0xF600), 0xFF);
    }

    #[test]
    fn new_ppi_port_b_reads_known_system_status() {
        // Per docs: Port B's idle pattern is:
        //   bit 7 (Cassette Read)       = 0 (no tape data)
        //   bit 6 (Parallel Port Busy)  = 1 (printer not connected => not ready)
        //   bit 5 (/EXP)                = 1 (no expansion present)
        //   bit 4 (Screen Frequency)    = 1 (50 Hz)
        //   bit 3..1 (Manufacturer)     = 1 1 1  (Amstrad)
        //   bit 0 (CRTC VSYNC)          = 0 (idle, not in VSYNC)
        // => 0b0111_1110 = 0x7E
        let ppi = Ppi::new();
        assert_eq!(ppi.read(0xF500), 0x7E);
    }

    #[test]
    fn new_ppi_vsync_inactive() {
        let ppi = Ppi::new();
        assert!(!ppi.vsync_active);
    }

    #[test]
    fn read_port_a_anywhere_in_f4xx_block() {
        let ppi = Ppi::new();
        for low in 0x00..=0xFF {
            let addr = 0xF400 | low as u16;
            assert_eq!(ppi.read(addr), 0xFF, "Port A read at {:#06X}", addr);
        }
    }

    #[test]
    fn read_port_b_anywhere_in_f5xx_block() {
        let ppi = Ppi::new();
        for low in 0x00..=0xFF {
            let addr = 0xF500 | low as u16;
            assert_eq!(ppi.read(addr), 0x7E, "Port B read at {:#06X}", addr);
        }
    }

    #[test]
    fn read_port_c_anywhere_in_f6xx_block() {
        let ppi = Ppi::new();
        for low in 0x00..=0xFF {
            let addr = 0xF600 | low as u16;
            assert_eq!(ppi.read(addr), 0xFF, "Port C read at {:#06X}", addr);
        }
    }

    #[test]
    fn config_0x82_sets_a_out() {
        // 0x82 = 1000_0010
        //  bit 7 = 1 (config mode)
        //  bits 6-5 = 00 (Group A Mode 0)
        //  bit 4   = 0 (Port A Output)
        //  bit 3   = 0 (Port C Upper Output)
        //  bit 2   = 0 (Group B Mode 0)
        //  bit 1   = 1 (Port B Input)
        //  bit 0   = 0 (Port C Lower Output)
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        assert_eq!(ppi.port_a_direction, PpiDirection::Output);
    }

    #[test]
    fn config_0x92_sets_a_in() {
        // 0x92 = 1001_0010
        //  bit 4 = 1 (Port A Input)
        let mut ppi = Ppi::new();
        ppi.write(0xF792, 0x92);
        assert_eq!(ppi.port_a_direction, PpiDirection::Input);
    }

    #[test]
    fn port_a_write_when_output_stores_value() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82); // A out
        ppi.write(0xF400, 0x42);
        assert_eq!(ppi.port_a_latch, 0x42);
    }

    #[test]
    fn port_a_write_when_input_does_not_modify_latch() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82); // A out
        ppi.write(0xF400, 0x42);
        ppi.write(0xF792, 0x92); // A in
        ppi.write(0xF400, 0xCC); // ignored (bus is input)
        assert_eq!(ppi.port_a_latch, 0x42);
    }

    #[test]
    fn port_a_read_when_output_returns_latch() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x99);
        // When Port A is configured as output, reading returns the latch
        // (real silicon behaviour for Mode 0 output reads).
        assert_eq!(ppi.read(0xF400), 0x99);
    }

    #[test]
    fn port_a_low_byte_is_dont_care_for_write() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF4FF, 0x12);
        ppi.write(0xF400, 0x34);
        assert_eq!(ppi.port_a_latch, 0x34);
        ppi.write(0xF4AA, 0x56);
        assert_eq!(ppi.port_a_latch, 0x56);
    }

    #[test]
    fn port_b_vsync_set_clears_no_other_bits() {
        let mut ppi = Ppi::new();
        let before = ppi.read(0xF500);
        assert_eq!(before, 0x7E);

        ppi.vsync_active = true;
        let after = ppi.read(0xF500);
        assert_eq!(after, 0x7F);
    }

    #[test]
    fn port_b_vsync_clear_sets_bit_0_to_zero() {
        let mut ppi = Ppi::new();
        ppi.vsync_active = true;
        ppi.vsync_active = false;
        assert_eq!(ppi.read(0xF500) & 0x01, 0x00);
    }

    #[test]
    fn port_b_parallel_port_busy_bit_reflects_input() {
        let mut ppi = Ppi::new();
        ppi.parallel_port_busy = true;
        assert_eq!(ppi.read(0xF500), 0x3E);

        ppi.parallel_port_busy = false;
        assert_eq!(ppi.read(0xF500), 0x7E);
    }

    #[test]
    fn port_b_exp_bit_reflects_input() {
        let mut ppi = Ppi::new();
        ppi.exp_present = true;
        assert_eq!(ppi.read(0xF500), 0x7E);

        ppi.exp_present = false;
        assert_eq!(ppi.read(0xF500), 0x5E);
    }

    #[test]
    fn port_b_screen_frequency_bit_reflects_input() {
        let mut ppi = Ppi::new();
        ppi.screen_frequency_50hz = true;
        assert_eq!(ppi.read(0xF500), 0x7E);

        ppi.screen_frequency_50hz = false;
        assert_eq!(ppi.read(0xF500), 0x6E);
    }

    #[test]
    fn port_b_manufacturer_jumper_all_combinations() {
        // (LK3, LK2, LK1) => expected manufacturer name (per docs table)
        let cases: [(u8, &str); 8] = [
            (0b000, "Isp"),
            (0b001, "Triumph"),
            (0b010, "Saisho"),
            (0b011, "Solavox"),
            (0b100, "Awa"),
            (0b101, "Schneider"),
            (0b110, "Orion"),
            (0b111, "Amstrad"),
        ];

        for (jumpers, _name) in cases {
            let mut ppi = Ppi::new();
            ppi.manufacturer_jumper = jumpers;
            let pb = ppi.read(0xF500);
            let decoded = ((pb >> 1) & 0b111) as u8;
            assert_eq!(decoded, jumpers, "manufacturer {:#b}", jumpers);
        }
    }

    #[test]
    fn port_b_default_manufacturer_is_amstrad() {
        let ppi = Ppi::new();
        let pb = ppi.read(0xF500);
        let decoded = ((pb >> 1) & 0b111) as u8;
        assert_eq!(decoded, 0b111, "Default manufacturer must be Amstrad");
    }

    #[test]
    fn port_c_psg_bdir_bit_writable() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x80);
        assert!(ppi.psg_bdir());
        ppi.write(0xF600, 0x00);
        assert!(!ppi.psg_bdir());
    }

    #[test]
    fn port_c_psg_bc1_bit_writable() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x40);
        assert!(ppi.psg_bc1());
        ppi.write(0xF600, 0x00);
        assert!(!ppi.psg_bc1());
    }

    #[test]
    fn port_c_cassette_write_data_writable() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x20);
        assert!(ppi.cassette_write_data());
        ppi.write(0xF600, 0x00);
        assert!(!ppi.cassette_write_data());
    }

    #[test]
    fn port_c_cassette_motor_writable() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x10);
        assert!(ppi.cassette_motor());
        ppi.write(0xF600, 0x00);
        assert!(!ppi.cassette_motor());
    }

    #[test]
    fn keyboard_row_select_each_value_0_to_9() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        for row in 0..=9u8 {
            ppi.write(0xF600, row);
            assert_eq!(
                ppi.keyboard_row(),
                row,
                "keyboard row should decode to {}",
                row
            );
        }
    }

    // TODO
    // #[test]
    // fn keyboard_row_select_10_to_15_decodes_as_no_row() {
    //     let mut ppi = Ppi::new();
    //     ppi.write(0xF782, 0x82);
    //     for row in 10..=15u8 {
    //         ppi.write(0xF600, row);
    //         assert_eq!(
    //             ppi.keyboard_row(),
    //             0xFF, // or some sentinel like None
    //             "row {} should select no keyboard row",
    //             row
    //         );
    //     }
    // }

    #[test]
    fn keyboard_row_is_isolated_in_lower_nibble() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        // Set BDIR/BC1 + row 5 = 0xC0 | 0x05
        ppi.write(0xF600, 0xC5);
        assert_eq!(ppi.keyboard_row(), 5);
        assert!(ppi.psg_bdir());
        assert!(ppi.psg_bc1());
    }

    #[test]
    fn psg_function_inactive() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b0000_0000);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
    }

    #[test]
    fn psg_function_read_register() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b0100_0000);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::ReadRegister);
    }

    #[test]
    fn psg_function_write_register() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b1000_0000);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);
    }

    #[test]
    fn psg_function_select_register() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b1100_0000);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);
    }

    #[test]
    fn psg_function_reflects_bsr_changes_to_port_c() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);

        // Use BSR to set bit 7 (BDIR) — function should become WriteRegister
        ppi.write(0xF600, 1 << 7);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);

        // Now set bit 6 (BC1) too — should become SelectRegister
        ppi.write(0xF600, (1 << 7) | (1 << 6));
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);
    }

    #[test]
    fn psg_select_register_sequence_latches_port_a() {
        // Sequence: write 0x0E (PSG Register 14) to Port A, then set
        // Port C bits 7,6 = 1,1 (Select Register), then go Inactive.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        ppi.write(0xF400, 0x0E); // PSG reg index on Port A
        ppi.write(0xF600, 0b1100_0000); // Select Register
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);

        ppi.write(0xF600, 0b0000_0000); // Inactive
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
        assert_eq!(ppi.port_a_latch, 0x0E);
    }

    #[test]
    fn psg_write_register_sequence_latches_port_a() {
        // Sequence: write data (15) to Port A, then set Port C bits 7,6
        // = 1,0 (Write Register), then go Inactive.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        ppi.write(0xF400, 15);
        ppi.write(0xF600, 0b1000_0000);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);

        ppi.write(0xF600, 0b0000_0000);
        assert_eq!(ppi.port_a_latch, 15);
    }

    #[test]
    fn psg_read_register_sequence_uses_input_port_a() {
        // Sequence: configure Port A as input, set Port C bits 7,6 = 0,1
        // (Read Register), read from Port A.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF792, 0x92); // A input
        ppi.write(0xF600, 0b0100_0000); // Read Register
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::ReadRegister);

        // Port A now in input mode -> external PSG drives it.
        // We can simulate the PSG's response by letting the test read 0xFF.
        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn full_psg_round_trip_select_then_write() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        // 1. Put register number 8 on Port A
        ppi.write(0xF400, 8);

        // 2. Select Register (BDIR=1, BC1=1)
        ppi.write(0xF600, 0xC0);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);

        // 3. Inactive (mandatory per docs)
        ppi.write(0xF600, 0x00);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);

        // 4. Put data 15 on Port A
        ppi.write(0xF400, 15);

        // 5. Write Register (BDIR=1, BC1=0)
        ppi.write(0xF600, 0x80);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);

        // 6. Inactive again
        ppi.write(0xF600, 0x00);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
        assert_eq!(ppi.port_a_latch, 15);
    }

    #[test]
    fn new_ppi_has_psg_selected_register_initialized_to_zero() {
        let ppi = Ppi::new();
        assert_eq!(ppi.psg_selected_register, 0);
    }

    #[test]
    fn psg_select_register_latches_port_a_value() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82); // Port A as output

        ppi.write(0xF400, 14); // PSG register 14 on Port A
        assert_eq!(ppi.psg_selected_register, 0);

        ppi.write(0xF600, 0xC0); // Select Register (BDIR=1, BC1=1)
        assert_eq!(ppi.psg_selected_register, 14);
    }

    #[test]
    fn psg_select_register_latches_each_valid_index() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        for reg in 0..=15u8 {
            ppi.write(0xF400, reg);
            ppi.write(0xF600, 0xC0); // Select Register
            ppi.write(0xF600, 0x00); // Inactive
            assert_eq!(
                ppi.psg_selected_register, reg,
                "PSG selected register should be {}",
                reg
            );
        }
    }

    #[test]
    fn psg_selected_register_persists_across_bus_state_changes() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        // Select register 7
        ppi.write(0xF400, 7);
        ppi.write(0xF600, 0xC0); // Select
        ppi.write(0xF600, 0x00); // Inactive

        // Go through various bus states — register should persist
        ppi.write(0xF600, 0x80); // Write Register
        assert_eq!(ppi.psg_selected_register, 7);
        ppi.write(0xF600, 0x00); // Inactive
        ppi.write(0xF600, 0x40); // Read Register
        assert_eq!(ppi.psg_selected_register, 7);
        ppi.write(0xF600, 0x00); // Inactive

        assert_eq!(ppi.psg_selected_register, 7);
    }

    #[test]
    fn psg_select_register_only_latches_in_select_mode() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        // Write 14 to Port A but don't enter Select Register mode
        ppi.write(0xF400, 14);
        ppi.write(0xF600, 0x80); // Write Register mode (not Select)
        assert_eq!(
            ppi.psg_selected_register, 0,
            "Should not latch register in Write mode"
        );

        ppi.write(0xF600, 0x00); // Inactive
        ppi.write(0xF600, 0x40); // Read Register mode (not Select)
        assert_eq!(
            ppi.psg_selected_register, 0,
            "Should not latch register in Read mode"
        );
    }

    #[test]
    fn keyboard_scan_row_0_no_keys_returns_0xff() {
        let mut ppi = Ppi::new();

        // Full keyboard scan sequence per keyboard.md
        ppi.write(0xF782, 0x82); // Port A output
        ppi.write(0xF400, 0x0E); // PSG register 14
        ppi.write(0xF600, 0xC0); // Select Register
        ppi.write(0xF600, 0x00); // Inactive (mandatory)
        ppi.write(0xF792, 0x92); // Port A input
        ppi.write(0xF600, 0x40); // Row 0, Read Register

        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn keyboard_scan_all_rows_0_to_9_no_keys_returns_0xff() {
        let mut ppi = Ppi::new();

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        for row in 0..=9u8 {
            ppi.write(0xF600, 0x40 | row); // Row N, Read Register
            assert_eq!(
                ppi.read(0xF400),
                0xFF,
                "Row {} should return 0xFF with no keys pressed",
                row
            );
        }
    }

    // #[test]
    // fn keyboard_scan_rows_10_to_15_return_0xff_regardless_of_keys() {
    //     // Per keyboard.md: 74ALS145 decoder drives all outputs high for
    //     // values 10-15, so PSG Port A pull-ups float to 1 → 0xFF
    //     let mut ppi = Ppi::new();
    //     ppi.keyboard.press_key(&CpcKey::Enter);
    //     ppi.keyboard.press_key(&CpcKey::A);
    //     ppi.keyboard.press_key(&CpcKey::Space);

    //     ppi.write(0xF782, 0x82);
    //     ppi.write(0xF400, 0x0E);
    //     ppi.write(0xF600, 0xC0);
    //     ppi.write(0xF600, 0x00);
    //     ppi.write(0xF792, 0x92);

    //     for row in 10..=15u8 {
    //         ppi.write(0xF600, 0x40 | row);
    //         assert_eq!(
    //             ppi.read(0xF400),
    //             0xFF,
    //             "Row {} should return 0xFF (decoder deselects all rows)",
    //             row
    //         );
    //     }
    // }

    #[test]
    fn keyboard_scan_row_0_cursor_up_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::CursorUp); // Row 0, bit 0

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40); // Row 0, Read Register

        assert_eq!(ppi.read(0xF400), 0xFE); // Bit 0 cleared
    }

    #[test]
    fn keyboard_scan_row_0_enter_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter); // Row 0, bit 6

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40);

        assert_eq!(ppi.read(0xF400), 0xBF); // Bit 6 cleared
    }

    #[test]
    fn keyboard_scan_row_0_fdot_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Fdot); // Row 0, bit 7

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40);

        assert_eq!(ppi.read(0xF400), 0x7F); // Bit 7 cleared
    }

    #[test]
    fn keyboard_scan_row_5_space_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Space); // Row 5, bit 7

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x45); // Row 5, Read Register

        assert_eq!(ppi.read(0xF400), 0x7F); // Bit 7 cleared
    }

    #[test]
    fn keyboard_scan_row_8_a_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::A); // Row 8, bit 5

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x48); // Row 8, Read Register

        assert_eq!(ppi.read(0xF400), 0xDF); // Bit 5 cleared
    }

    #[test]
    fn keyboard_scan_row_8_esc_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Esc); // Row 8, bit 2

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x48);

        assert_eq!(ppi.read(0xF400), 0xFB); // Bit 2 cleared
    }

    #[test]
    fn keyboard_scan_row_2_ctrl_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Ctrl); // Row 2, bit 7

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x42); // Row 2, Read Register

        assert_eq!(ppi.read(0xF400), 0x7F); // Bit 7 cleared
    }

    #[test]
    fn keyboard_scan_row_2_shift_pressed() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Shift); // Row 2, bit 5

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x42);

        assert_eq!(ppi.read(0xF400), 0xDF); // Bit 5 cleared
    }

    #[test]
    fn keyboard_scan_multiple_keys_same_row() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::CursorUp); // Row 0, bit 0
        ppi.keyboard.press_key(&CpcKey::CursorRight); // Row 0, bit 1
        ppi.keyboard.press_key(&CpcKey::Enter); // Row 0, bit 6

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40); // Row 0

        assert_eq!(ppi.read(0xF400), 0xBC); // Bits 0, 1, 6 cleared
    }

    #[test]
    fn keyboard_scan_all_keys_in_row_0_pressed() {
        let mut ppi = Ppi::new();
        for key in &CpcKey::ROW1 {
            ppi.keyboard.press_key(key);
        }

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40);

        assert_eq!(ppi.read(0xF400), 0x00); // All bits cleared
    }

    #[test]
    fn keyboard_scan_keys_in_different_rows_dont_interfere() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::A); // Row 8, bit 5
        ppi.keyboard.press_key(&CpcKey::Space); // Row 5, bit 7

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        ppi.write(0xF600, 0x45); // Row 5
        assert_eq!(ppi.read(0xF400), 0x7F); // Only Space

        ppi.write(0xF600, 0x48); // Row 8
        assert_eq!(ppi.read(0xF400), 0xDF); // Only A
    }

    #[test]
    fn keyboard_scan_row_changes_between_reads() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter); // Row 0, bit 6
        ppi.keyboard.press_key(&CpcKey::A); // Row 8, bit 5

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        // Read row 0 → Enter detected
        ppi.write(0xF600, 0x40);
        assert_eq!(ppi.read(0xF400), 0xBF);

        // Read row 8 → A detected
        ppi.write(0xF600, 0x48);
        assert_eq!(ppi.read(0xF400), 0xDF);

        // Read row 0 again → Enter still detected
        ppi.write(0xF600, 0x40);
        assert_eq!(ppi.read(0xF400), 0xBF);

        // Read row 5 → nothing
        ppi.write(0xF600, 0x45);
        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn keyboard_scan_each_row_bit_7_key() {
        // Bit 7 of each row (per docs matrix):
        // Row 0: Fdot, Row 1: F0, Row 2: Ctrl, Row 3: Comma,
        // Row 4: Dot, Row 5: Space, Row 6: V, Row 7: X,
        // Row 8: Z, Row 9: Del
        let keys = [
            CpcKey::Fdot,
            CpcKey::F0,
            CpcKey::Ctrl,
            CpcKey::Comma,
            CpcKey::Dot,
            CpcKey::Space,
            CpcKey::V,
            CpcKey::X,
            CpcKey::Z,
            CpcKey::Del,
        ];

        let mut ppi = Ppi::new();
        for key in &keys {
            ppi.keyboard.press_key(key);
        }

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        for row in 0..=9u8 {
            ppi.write(0xF600, 0x40 | row);
            assert_eq!(
                ppi.read(0xF400),
                0x7F,
                "Row {} should have bit 7 cleared",
                row
            );
        }
    }

    #[test]
    fn keyboard_scan_each_row_bit_0_key() {
        // Bit 0 of each row:
        // Row 0: CursorUp, Row 1: CursorLeft, Row 2: Clr, Row 3: Caret,
        // Row 4: Zero, Row 5: Eight, Row 6: Six, Row 7: Four,
        // Row 8: One, Row 9: Joy0Up
        let keys = [
            CpcKey::CursorUp,
            CpcKey::CursorLeft,
            CpcKey::Clr,
            CpcKey::Caret,
            CpcKey::Zero,
            CpcKey::Eight,
            CpcKey::Six,
            CpcKey::Four,
            CpcKey::One,
            CpcKey::Joy0Up,
        ];

        let mut ppi = Ppi::new();
        for key in &keys {
            ppi.keyboard.press_key(key);
        }

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        for row in 0..=9u8 {
            ppi.write(0xF600, 0x40 | row);
            assert_eq!(
                ppi.read(0xF400),
                0xFE,
                "Row {} should have bit 0 cleared",
                row
            );
        }
    }

    #[test]
    fn keyboard_scan_full_sequence_matches_firmware_algorithm() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter); // Row 0, bit 6

        // Step 1: Write 0x0E (Reg 14) to PPI Port A
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);

        // Step 2: Select PSG Reg (Bits 7-6 = 11)
        ppi.write(0xF600, 0xC0);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);
        assert_eq!(ppi.psg_selected_register, 14);

        // Step 3: Inactive Phase (Bits 7-6 = 00) — mandatory per docs
        ppi.write(0xF600, 0x00);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);

        // Step 4: Change PPI Buffer Direction — Port A to Input
        ppi.write(0xF792, 0x92);
        assert_eq!(ppi.port_a_direction, PpiDirection::Input);

        // Step 5: Assert Column Line — row 0
        ppi.write(0xF600, 0x00);
        assert_eq!(ppi.keyboard_row(), 0);

        // Step 6: Enable PSG Data Read (Bits 7-6 = 01)
        ppi.write(0xF600, 0x40);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::ReadRegister);

        // Step 7: Capture Matrix Byte
        let row_data = ppi.read(0xF400);
        assert_eq!(row_data, 0xBF); // Bit 6 cleared (Enter)

        // Step 8: Re-initialize Bus State
        ppi.write(0xF600, 0x00);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
    }

    #[test]
    fn keyboard_scan_full_sequence_no_key_pressed() {
        let mut ppi = Ppi::new();

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40);

        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn keyboard_scan_restore_port_a_to_output_after_reading() {
        let mut ppi = Ppi::new();

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92); // Input
        ppi.write(0xF600, 0x40);
        let _ = ppi.read(0xF400);

        // Restore: Port A back to Output, PSG to Inactive
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);

        assert_eq!(ppi.port_a_direction, PpiDirection::Output);
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
    }

    #[test]
    fn keyboard_scan_requires_port_a_in_input_mode() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter);

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);

        // Don't switch to input — stay in output mode
        ppi.write(0xF600, 0x40); // Read Register, row 0

        // Should return the latch value (0x0E), not keyboard data
        assert_eq!(ppi.read(0xF400), 0x0E);
        assert_ne!(ppi.read(0xF400), 0xBF); // Not keyboard data
    }

    #[test]
    fn keyboard_scan_requires_psg_in_read_register_mode() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter);

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92); // Input mode

        // PSG in Inactive mode, not Read Register
        ppi.write(0xF600, 0x00); // Inactive, row 0

        // Should return 0xFF (floating bus), not keyboard data
        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn keyboard_scan_requires_register_14_selected() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter);

        // Select register 7 instead of 14
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x07);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40); // Read Register, row 0

        // Should NOT return keyboard data since register is not 14
        let data = ppi.read(0xF400);
        assert_ne!(data, 0xBF, "Should not return keyboard data for reg 7");
    }

    #[test]
    fn keyboard_release_key_updates_scan_result() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter);

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);
        ppi.write(0xF600, 0x40);

        // Key pressed → bit 6 cleared
        assert_eq!(ppi.read(0xF400), 0xBF);

        // Release key
        ppi.keyboard.release_key(&CpcKey::Enter);

        // Key released → all bits set
        assert_eq!(ppi.read(0xF400), 0xFF);
    }

    #[test]
    fn keyboard_reset_clears_all_keys_through_ppi() {
        let mut ppi = Ppi::new();
        ppi.keyboard.press_key(&CpcKey::Enter);
        ppi.keyboard.press_key(&CpcKey::A);
        ppi.keyboard.press_key(&CpcKey::Space);

        ppi.keyboard.reset();

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        for row in 0..=9u8 {
            ppi.write(0xF600, 0x40 | row);
            assert_eq!(
                ppi.read(0xF400),
                0xFF,
                "Row {} should be clear after reset",
                row
            );
        }
    }

    #[test]
    fn keyboard_rapid_press_release_through_ppi() {
        let mut ppi = Ppi::new();

        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E);
        ppi.write(0xF600, 0xC0);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF792, 0x92);

        for _ in 0..100 {
            ppi.keyboard.press_key(&CpcKey::Enter);
            ppi.write(0xF600, 0x40);
            assert_eq!(ppi.read(0xF400), 0xBF);

            ppi.keyboard.release_key(&CpcKey::Enter);
            ppi.write(0xF600, 0x40);
            assert_eq!(ppi.read(0xF400), 0xFF);
        }
    }

    #[test]
    fn port_c_read_returns_last_written_value() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        ppi.write(0xF600, 0xC5); // BDIR=1, BC1=1, row=5
        assert_eq!(ppi.read(0xF600), 0xC5);
    }

    #[test]
    fn port_c_read_reflects_all_bits() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);

        for value in 0x00..=0xFF {
            ppi.write(0xF600, value);
            assert_eq!(
                ppi.read(0xF600),
                value,
                "Port C should return {:#02X}",
                value
            );
        }
    }

    fn make_cdt_header(major: u8, minor: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"ZXTape!\x1a");
        v.push(major);
        v.push(minor);
        v
    }

    fn make_cdt_with_block_10(pause_ms: u16, payload: &[u8]) -> Vec<u8> {
        let mut v = make_cdt_header(1, 13);
        v.push(0x10);
        v.push((pause_ms & 0xFF) as u8);
        v.push((pause_ms >> 8) as u8);
        v.push((payload.len() & 0xFF) as u8);
        v.push((payload.len() >> 8) as u8);
        v.extend_from_slice(payload);
        v
    }

    /// Port B bit 7 mask = Cassette Read Data.
    const CASSETTE_READ_BIT: u8 = 0x80;
    /// Port C bit 4 mask = Cassette Motor.
    const CASSETTE_MOTOR_BIT: u8 = 0x10;

    #[test]
    fn new_ppi_has_no_tape_loaded() {
        let ppi = Ppi::new();
        assert!(ppi.tape().is_none(), "PPI must start with no tape loaded");
    }

    #[test]
    fn load_tape_installs_player() {
        let mut ppi = Ppi::new();
        let tape = TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap();
        ppi.load_tape(tape);
        assert!(ppi.tape().is_some());
    }

    #[test]
    fn unload_tape_returns_loaded_player() {
        let mut ppi = Ppi::new();
        let tape = TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap();
        ppi.load_tape(tape);
        let unloaded = ppi.unload_tape();
        assert!(unloaded.is_some(), "unload must return the player");
        assert!(ppi.tape().is_none(), "tape must be removed after unload");
    }

    #[test]
    fn unload_tape_when_none_loaded_returns_none() {
        let mut ppi = Ppi::new();
        assert!(ppi.unload_tape().is_none());
    }

    #[test]
    fn load_tape_replaces_existing_tape() {
        // Decide your policy; this test asserts silent replacement.
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(2, &[0xFF])).unwrap());
        assert!(ppi.tape().is_some());
        // The second tape's first pulse is still 2168 (same pilot), so we can't
        // distinguish here, but we at least confirm no panic and `tape()` is Some.
    }

    #[test]
    fn tape_mut_allows_mutable_access() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        assert!(ppi.tape_mut().is_some());
        ppi.tape_mut().unwrap().play();
        // Internal check: tape is now playing (if it has data)
        assert!(ppi.tape().unwrap().is_playing());
    }

    #[test]
    fn port_b_bit7_is_zero_when_no_tape_loaded() {
        // Existing behavior: cassette_read_data defaults to false → bit 7 = 0.
        let ppi = Ppi::new();
        assert_eq!(ppi.read(0xF500) & CASSETTE_READ_BIT, 0);
    }

    #[test]
    fn port_b_bit7_reflects_initial_ear_state_when_tape_loaded() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        // Tape not playing yet → EAR is false → bit 7 = 0.
        assert_eq!(ppi.read(0xF500) & CASSETTE_READ_BIT, 0);
    }

    #[test]
    fn port_b_bit7_toggles_after_tape_advances_one_pilot_pulse() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());

        // Configure Port C as output and turn motor on.
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0);
        ppi.write(0xF600, CASSETTE_MOTOR_BIT); // motor on, no PSG bits

        let initial = ppi.read(0xF500) & CASSETTE_READ_BIT;

        // Pilot pulse = 2168 CDT T-states = 2478 CPC T-states.
        // tick_tape(N) calls tape.advance(N) which consumes N CPC T-states.
        // 2477 isn't enough; 2478 toggles EAR once.
        ppi.tick_tape(2477);
        assert_eq!(
            ppi.read(0xF500) & CASSETTE_READ_BIT,
            initial,
            "EAR must not toggle before pilot pulse boundary"
        );

        ppi.tick_tape(1);
        assert_ne!(
            ppi.read(0xF500) & CASSETTE_READ_BIT,
            initial,
            "EAR must toggle after consuming exactly 2478 CPC T-states"
        );
    }

    #[test]
    fn port_b_bit7_alternates_with_each_pilot_pulse() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0);
        ppi.write(0xF600, CASSETTE_MOTOR_BIT);

        let initial = ppi.read(0xF500) & CASSETTE_READ_BIT;

        ppi.tick_tape(2478);
        assert_ne!(ppi.read(0xF500) & CASSETTE_READ_BIT, initial);
        ppi.tick_tape(2478);
        assert_eq!(ppi.read(0xF500) & CASSETTE_READ_BIT, initial);
        ppi.tick_tape(2478);
        assert_ne!(ppi.read(0xF500) & CASSETTE_READ_BIT, initial);
    }

    #[test]
    fn port_b_bit7_does_not_change_when_motor_off() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        // Motor stays off
        ppi.write(0xF600, 0x00);

        let initial = ppi.read(0xF500) & CASSETTE_READ_BIT;
        ppi.tick_tape(50_000);
        assert_eq!(
            ppi.read(0xF500) & CASSETTE_READ_BIT,
            initial,
            "EAR must not change when motor is off"
        );
    }

    #[test]
    fn motor_bit_set_starts_tape_playing() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);

        ppi.write(0xF600, 0x00);

        ppi.write(0xF600, CASSETTE_MOTOR_BIT);
        assert!(
            ppi.tape().unwrap().is_playing(),
            "Setting motor bit must start the tape"
        );
    }

    #[test]
    fn motor_bit_cleared_stops_tape_playing() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);

        ppi.write(0xF600, 0x00);
        assert!(
            !ppi.tape().unwrap().is_playing(),
            "Clearing motor bit must stop the tape"
        );

        ppi.write(0xF600, CASSETTE_MOTOR_BIT);
        assert!(ppi.tape().unwrap().is_playing());
    }

    #[test]
    fn motor_bit_unchanged_does_not_call_play_or_stop_repeatedly() {
        // Writing the same motor state should be a no-op (idempotent).
        // We can't directly observe call count, but we can verify state stays
        // consistent and EAR doesn't unexpectedly change.
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);

        ppi.write(0xF600, 0x00);
        assert!(!ppi.tape().unwrap().is_playing());

        ppi.write(0xF600, CASSETTE_MOTOR_BIT);
        assert!(ppi.tape().unwrap().is_playing());

        // Write the same value again
        ppi.write(0xF600, CASSETTE_MOTOR_BIT);
        assert!(ppi.tape().unwrap().is_playing());

        // Write again with extra bits set (motor still on)
        ppi.write(0xF600, 0x1F);
        assert!(ppi.tape().unwrap().is_playing());
    }

    #[test]
    fn motor_on_with_other_port_c_bits_preserves_other_bits() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);

        ppi.write(0xF600, 0x00);

        // Set motor + keyboard row 5 = 0x15
        ppi.write(0xF600, 0x15);
        assert!(ppi.tape().unwrap().is_playing());
        assert_eq!(
            ppi.read(0xF600),
            0x15,
            "Other Port C bits must be preserved"
        );
    }

    #[test]
    fn motor_on_without_tape_loaded_does_not_panic() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, CASSETTE_MOTOR_BIT); // should be silent no-op
        assert!(ppi.tape().is_none());
    }

    #[test]
    fn motor_off_without_tape_loaded_does_not_panic() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        assert!(ppi.tape().is_none());
    }

    #[test]
    fn tick_tape_without_loaded_tape_is_noop() {
        let mut ppi = Ppi::new();
        ppi.tick_tape(1_000_000); // must not panic
    }

    #[test]
    fn tick_tape_advances_loaded_playing_tape() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0);
        ppi.write(0xF600, CASSETTE_MOTOR_BIT);

        let initial = ppi.read(0xF500) & CASSETTE_READ_BIT;
        ppi.tick_tape(2478);
        assert_ne!(ppi.read(0xF500) & CASSETTE_READ_BIT, initial);
    }

    #[test]
    fn tick_tape_does_not_advance_when_motor_off() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        // motor off
        ppi.write(0xF600, 0x00);

        let initial = ppi.read(0xF500) & CASSETTE_READ_BIT;
        ppi.tick_tape(100_000);
        assert_eq!(ppi.read(0xF500) & CASSETTE_READ_BIT, initial);
    }

    #[test]
    fn tick_tape_accumulates_correctly_across_calls() {
        // advance(2478) once == advance(4) called 620 times (4*620 = 2480 ≥ 2478)
        let mut ppi_a = Ppi::new();
        let mut ppi_b = Ppi::new();
        let cdt = make_cdt_with_block_10(1, &[0]);
        ppi_a.load_tape(TapePlayer::from_cdt(&cdt).unwrap());
        ppi_b.load_tape(TapePlayer::from_cdt(&cdt).unwrap());

        for ppi in [&mut ppi_a, &mut ppi_b] {
            ppi.write(0xF782, 0x82);
            ppi.write(0xF600, CASSETTE_MOTOR_BIT);
        }

        ppi_a.tick_tape(2478);
        for _ in 0..620 {
            ppi_b.tick_tape(4);
        }

        assert_eq!(
            ppi_a.read(0xF500) & CASSETTE_READ_BIT,
            ppi_b.read(0xF500) & CASSETTE_READ_BIT,
            "Chunked advance must match single advance"
        );
    }

    #[test]
    fn tick_tape_after_tape_ends_stops_toggling_ear() {
        // Build a tape with a single short pause block that ends quickly.
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // 1ms pause, then tape ends
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&v).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, CASSETTE_MOTOR_BIT);

        // Burn through the tape
        ppi.tick_tape(1_000_000);
        assert!(
            !ppi.tape().unwrap().is_playing(),
            "Tape should be exhausted"
        );

        // EAR must now be stable regardless of further ticks
        let stable = ppi.read(0xF500) & CASSETTE_READ_BIT;
        ppi.tick_tape(1_000_000);
        assert_eq!(ppi.read(0xF500) & CASSETTE_READ_BIT, stable);
    }

    #[test]
    fn bsr_set_bit_0_sets_only_bit_0() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF701, 0x01); // 0_000_000_1 = set bit 0
        assert_eq!(ppi.read(0xF600), 0x01);
    }

    #[test]
    fn bsr_clear_bit_0_clears_only_bit_0() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0xFF);
        ppi.write(0xF700, 0x00); // 0_000_000_0 = clear bit 0
        assert_eq!(ppi.read(0xF600), 0xFE);
    }

    #[test]
    fn bsr_set_each_bit_0_through_7_individually() {
        for bit in 0u8..=7 {
            let mut ppi = Ppi::new();
            ppi.write(0xF782, 0x82);
            ppi.write(0xF600, 0x00);
            let bsr = (bit << 1) | 1; // set bit `bit`
            ppi.write(0xF700 | bsr as u16, bsr);
            assert_eq!(
                ppi.read(0xF600),
                1u8 << bit,
                "BSR {:#04X} should set only bit {}",
                bsr,
                bit
            );
        }
    }

    #[test]
    fn bsr_clear_each_bit_0_through_7_individually() {
        for bit in 0u8..=7 {
            let mut ppi = Ppi::new();
            ppi.write(0xF782, 0x82);
            ppi.write(0xF600, 0xFF);
            let bsr = bit << 1; // clear bit `bit`
            ppi.write(0xF700 | bsr as u16, bsr);
            assert_eq!(
                ppi.read(0xF600),
                0xFF & !(1u8 << bit),
                "BSR {:#04X} should clear only bit {}",
                bsr,
                bit
            );
        }
    }

    #[test]
    fn bsr_set_preserves_other_bits() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b1010_1010);
        ppi.write(0xF701, 0x01); // set bit 0
        assert_eq!(ppi.read(0xF600), 0b1010_1011);
    }

    #[test]
    fn bsr_clear_preserves_other_bits() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0b1111_1111);
        ppi.write(0xF708, 0x08); // 0_000_100_0 = clear bit 4
        assert_eq!(ppi.read(0xF600), 0b1110_1111);
    }

    #[test]
    fn bsr_does_not_change_port_a_direction() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        assert_eq!(ppi.port_a_direction, PpiDirection::Output);
        ppi.write(0xF701, 0x01);
        assert_eq!(
            ppi.port_a_direction,
            PpiDirection::Output,
            "BSR must not change Port A direction"
        );

        ppi.write(0xF792, 0x92);
        assert_eq!(ppi.port_a_direction, PpiDirection::Input);
        ppi.write(0xF703, 0x03);
        assert_eq!(
            ppi.port_a_direction,
            PpiDirection::Input,
            "BSR must not change Port A direction"
        );
    }

    #[test]
    fn bsr_does_not_reset_port_a_latch() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x42);
        ppi.write(0xF701, 0x01);
        assert_eq!(ppi.port_a_latch, 0x42, "BSR must not touch Port A latch");
    }

    #[test]
    fn bsr_does_not_reset_other_port_c_bits_to_zero() {
        // Unlike group configuration, BSR must NOT clear the rest of Port C.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0xCC);
        ppi.write(0xF701, 0x01); // set bit 0 → 0xCD
        assert_eq!(ppi.read(0xF600), 0xCD);
    }

    #[test]
    fn bsr_does_not_affect_port_b_read() {
        let mut ppi = Ppi::new();
        let before = ppi.read(0xF500);
        ppi.write(0xF701, 0x01);
        ppi.write(0xF70F, 0x0F);
        ppi.write(0xF708, 0x08);
        let after = ppi.read(0xF500);
        assert_eq!(before, after, "BSR must not affect Port B");
    }

    #[test]
    fn bsr_don_t_care_bits_6_4_are_ignored() {
        // Same N2..N0 + VAL with arbitrary bits 6-4 must give identical results.
        for dc in [0x00u8, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70] {
            let mut ppi = Ppi::new();
            ppi.write(0xF782, 0x82);
            ppi.write(0xF600, 0x00);
            let bsr = dc | 0x01; // set bit 0
            ppi.write(0xF700 | bsr as u16, bsr);
            assert_eq!(
                ppi.read(0xF600),
                0x01,
                "BSR {:#04X} should behave identically (don't-care bits 6-4)",
                bsr
            );
        }
    }

    #[test]
    fn bsr_address_low_byte_is_ignored() {
        // 0xF700..=0xF7FF all hit the control register.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF7FF, 0x01); // set bit 0 via 0xF7FF
        assert_eq!(ppi.read(0xF600), 0x01);
        ppi.write(0xF7AA, 0x02); // clear bit 1 via 0xF7AA — bit 1 already 0
        assert_eq!(ppi.read(0xF600), 0x01);
        ppi.write(0xF755, 0x03); // set bit 1 via 0xF755
        assert_eq!(ppi.read(0xF600), 0x03);
    }

    #[test]
    fn bsr_set_bit_7_enables_psg_bdir() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF70F, 0x0F); // 0_000_111_1 = set bit 7
        assert!(ppi.psg_bdir());
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);
    }

    #[test]
    fn bsr_set_bit_6_enables_psg_bc1() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF70D, 0x0D); // 0_000_110_1 = set bit 6
        assert!(ppi.psg_bc1());
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::ReadRegister);
    }

    #[test]
    fn bsr_set_bits_7_then_6_enters_select_register_mode() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x0E); // PSG register 14 on Port A
        ppi.write(0xF600, 0x00);

        ppi.write(0xF70F, 0x0F); // set bit 7 → WriteRegister
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::WriteRegister);
        assert_eq!(
            ppi.psg_selected_register, 0,
            "WriteRegister must not latch the selected register"
        );

        ppi.write(0xF70D, 0x0D); // set bit 6 → SelectRegister
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::SelectRegister);
        assert_eq!(
            ppi.psg_selected_register, 0x0E,
            "SelectRegister via BSR must latch Port A into psg_selected_register"
        );
    }

    #[test]
    fn bsr_clear_bit_7_disables_psg_bdir() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x80); // BDIR on
        ppi.write(0xF70E, 0x0E); // 0_000_111_0 = clear bit 7
        assert!(!ppi.psg_bdir());
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
    }

    #[test]
    fn bsr_clear_bit_6_disables_psg_bc1() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x40); // BC1 on
        ppi.write(0xF70C, 0x0C); // 0_000_110_0 = clear bit 6
        assert!(!ppi.psg_bc1());
        assert_eq!(ppi.psg_bus_function(), PsgBusFunction::Inactive);
    }

    #[test]
    fn bsr_can_turn_cassette_motor_on() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF709, 0x09); // 0_000_100_1 = set bit 4 (motor)
        assert!(ppi.cassette_motor());
        assert!(
            ppi.tape().unwrap().is_playing(),
            "BSR setting motor bit must start the tape"
        );
    }

    #[test]
    fn bsr_can_turn_cassette_motor_off() {
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x10); // motor on
        ppi.write(0xF708, 0x08); // 0_000_100_0 = clear bit 4
        assert!(!ppi.cassette_motor());
        assert!(
            !ppi.tape().unwrap().is_playing(),
            "BSR clearing motor bit must stop the tape"
        );
    }

    #[test]
    fn bsr_set_clear_cassette_write_data_bit_5() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);
        ppi.write(0xF70B, 0x0B); // 0_000_101_1 = set bit 5
        assert!(ppi.cassette_write_data());
        ppi.write(0xF70A, 0x0A); // 0_000_101_0 = clear bit 5
        assert!(!ppi.cassette_write_data());
    }

    #[test]
    fn bsr_can_modify_keyboard_row_bits_one_at_a_time() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);

        ppi.write(0xF703, 0x03); // set bit 1
        assert_eq!(ppi.keyboard_row(), 0b0010);
        ppi.write(0xF707, 0x07); // set bit 3
        assert_eq!(ppi.keyboard_row(), 0b1010);
        ppi.write(0xF705, 0x05); // set bit 2
        assert_eq!(ppi.keyboard_row(), 0b1110);
        ppi.write(0xF702, 0x02); // clear bit 1
        assert_eq!(ppi.keyboard_row(), 0b1100);
    }

    #[test]
    fn bsr_does_not_disturb_upper_nibble_when_changing_row() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0xC0); // BDIR=1, BC1=1, row 0
        ppi.write(0xF705, 0x05); // set bit 2 → row 4
        assert_eq!(ppi.keyboard_row(), 0b0100);
        assert!(ppi.psg_bdir(), "BDIR must be preserved");
        assert!(ppi.psg_bc1(), "BC1 must be preserved");
        assert_eq!(ppi.read(0xF600), 0xC4);
    }

    #[test]
    fn bsr_accumulates_across_multiple_writes() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);

        ppi.write(0xF701, 0x01); // set bit 0
        ppi.write(0xF705, 0x05); // set bit 2
        ppi.write(0xF709, 0x09); // set bit 4
        ppi.write(0xF70D, 0x0D); // set bit 6
        assert_eq!(ppi.read(0xF600), 0b0101_0101);

        ppi.write(0xF704, 0x04); // clear bit 2
        ppi.write(0xF70C, 0x0C); // clear bit 6
        assert_eq!(ppi.read(0xF600), 0b0001_0001);
    }

    #[test]
    fn bsr_set_then_clear_returns_to_original_value() {
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0xA5);
        let original = ppi.read(0xF600);

        ppi.write(0xF707, 0x07); // set bit 3
        assert_eq!(ppi.read(0xF600), original | 0x08);
        ppi.write(0xF706, 0x06); // clear bit 3
        assert_eq!(ppi.read(0xF600), original);
    }

    #[test]
    fn bsr_after_group_config_does_not_reset_latches() {
        // Group config writes reset latches on real 8255 silicon.
        // BSR writes must NOT. Verify by setting non-zero latches,
        // issuing a BSR, and confirming only the targeted bit changes.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF400, 0x77); // Port A = 0x77
        ppi.write(0xF600, 0x55); // Port C = 0x55
        ppi.write(0xF70F, 0x0F); // BSR set bit 7
        assert_eq!(ppi.read(0xF600), 0xD5, "Only bit 7 must change");
        assert_eq!(ppi.port_a_latch, 0x77, "Port A latch must be untouched");
    }

    #[test]
    fn bsr_uses_control_register_path_not_port_c_data_path() {
        // Sanity: writing 0x01 to 0xF6xx replaces the entire latch with 0x01.
        //         writing 0x01 to 0xF7xx (BSR) sets only bit 0.
        let mut ppi_a = Ppi::new();
        ppi_a.write(0xF782, 0x82);
        ppi_a.write(0xF600, 0xAA);
        ppi_a.write(0xF600, 0x01); // full Port C write → 0x01
        assert_eq!(ppi_a.read(0xF600), 0x01);

        let mut ppi_b = Ppi::new();
        ppi_b.write(0xF782, 0x82);
        ppi_b.write(0xF600, 0xAA);
        ppi_b.write(0xF701, 0x01); // BSR set bit 0 → 0xAB
        assert_eq!(ppi_b.read(0xF600), 0xAB);
    }

    #[test]
    fn bsr_motor_toggling_via_individual_bits_drives_tape() {
        // End-to-end: starting from a Port C with motor on, use BSR to flip
        // motor bit off and on, verifying tape state each time.
        let mut ppi = Ppi::new();
        ppi.load_tape(TapePlayer::from_cdt(&make_cdt_with_block_10(1, &[0])).unwrap());
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x10); // motor on

        ppi.write(0xF708, 0x08); // BSR clear bit 4
        assert!(!ppi.tape().unwrap().is_playing());

        ppi.write(0xF709, 0x09); // BSR set bit 4
        assert!(ppi.tape().unwrap().is_playing());

        // Toggle an unrelated bit while motor stays on — tape must not stop.
        ppi.write(0xF701, 0x01); // set bit 0
        assert!(ppi.tape().unwrap().is_playing());
        ppi.write(0xF700, 0x00); // clear bit 0
        assert!(ppi.tape().unwrap().is_playing());
    }

    #[test]
    fn bsr_does_not_invoke_group_config_behavior_for_bytes_below_0x80() {
        // Any byte with bit 7 = 0 is BSR, regardless of bits 6-4.
        // Specifically, bytes that share bits with the standard group-config
        // patterns must not be misinterpreted as group config.
        let mut ppi = Ppi::new();
        ppi.write(0xF782, 0x82);
        ppi.write(0xF600, 0x00);

        // 0x02 = 0_000_001_0 = clear bit 1 (must NOT be treated as a group
        // config byte that would change Port A direction).
        ppi.write(0xF702, 0x02);
        assert_eq!(
            ppi.port_a_direction,
            PpiDirection::Output,
            "BSR byte 0x02 must not be confused with group config"
        );
        assert_eq!(ppi.read(0xF600), 0x00, "bit 1 was already 0");
    }
}
