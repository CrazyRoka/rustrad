use std::cell::{Ref, RefCell, RefMut};

use crate::{
    Crtc, GateArray, Ppi, Video,
    fdc::{Controller, Variant},
    memory::{CpcMemory, MemoryReader},
};
use z80::Bus;

pub struct Cpc {
    lower_rom: Box<[u8; 0x4000]>, // 16 KB
    // TODO: handle arbitrary number of rows
    upper_roms: Box<[[u8; 0x4000]; 2]>, // 16x2 KB
    selected_rom: u8,
    memory: CpcMemory,
    memory_banking_selection: u8,
    // Peripherals
    gate_array: GateArray,
    ppi: Ppi,
    crtc: Crtc,
    video: Video,
    fdc: Option<RefCell<Controller>>,
}

impl Cpc {
    pub fn new_464(rom: &[u8]) -> Self {
        assert_eq!(rom.len(), 0x8000, "ROM length is supposed to be 32KB");
        let mut lower_rom = Box::new([0; 0x4000]);
        lower_rom.copy_from_slice(&rom[0..0x4000]);
        let mut upper_roms = Box::new([[0; 0x4000]; 2]);
        upper_roms[0].copy_from_slice(&rom[0x4000..0x8000]);

        Self {
            lower_rom,
            upper_roms,
            selected_rom: 0,
            memory: CpcMemory::new_64k(),
            memory_banking_selection: 0,
            gate_array: GateArray::new(),
            ppi: Ppi::new(),
            crtc: Crtc::new(),
            video: Video::new(),
            fdc: None,
        }
    }

    pub fn new_6128(rom: &[u8]) -> Self {
        assert_eq!(rom.len(), 0xC000, "ROM length is supposed to be 48KB");
        let mut lower_rom = Box::new([0; 0x4000]);
        lower_rom.copy_from_slice(&rom[0..0x4000]);
        let mut upper_roms = Box::new([[0xFF; 0x4000]; 2]);
        upper_roms[0].copy_from_slice(&rom[0x4000..0x8000]);
        upper_roms[1].copy_from_slice(&rom[0x8000..0xC000]);

        Self {
            lower_rom,
            upper_roms,
            selected_rom: 0,
            memory: CpcMemory::new_128k(),
            memory_banking_selection: 0,
            gate_array: GateArray::new(),
            ppi: Ppi::new(),
            crtc: Crtc::new(),
            video: Video::new(),
            fdc: Some(RefCell::new(Controller::new_with_variant(Variant::Upd765A))),
        }
    }

    pub fn gate_array_mut(&mut self) -> &mut GateArray {
        &mut self.gate_array
    }

    pub fn gate_array(&self) -> &GateArray {
        &self.gate_array
    }

    pub fn ppi(&self) -> &Ppi {
        &self.ppi
    }

    pub fn ppi_mut(&mut self) -> &mut Ppi {
        &mut self.ppi
    }

    pub fn crtc(&self) -> &Crtc {
        &self.crtc
    }

    pub fn crtc_mut(&mut self) -> &mut Crtc {
        &mut self.crtc
    }

    pub fn video(&self) -> &Video {
        &self.video
    }

    pub fn video_mut(&mut self) -> &mut Video {
        &mut self.video
    }

    pub fn fdc(&self) -> Option<Ref<'_, Controller>> {
        self.fdc.as_ref().map(|fdc| fdc.borrow())
    }

    pub fn fdc_mut(&mut self) -> Option<RefMut<'_, Controller>> {
        self.fdc.as_ref().map(|fdc| fdc.borrow_mut())
    }

    // Advances the video and other subsystems by exactly one CRTC character clock (one cycle).
    pub fn tick(&mut self) {
        self.video.tick(&self.crtc, &self.gate_array, &self.memory);
        let hsync_prev = self.crtc.hsync();
        self.crtc.tick();
        if hsync_prev && !self.crtc.hsync() {
            self.gate_array.hsync();
        }
        self.gate_array.set_vsync(self.crtc.vsync());
        self.ppi.set_vsync(self.crtc.vsync());
        self.ppi.tick_tape(4);
        if let Some(fdc) = self.fdc.as_ref() {
            fdc.borrow_mut().tick(4);
        }
    }

    /// Reads raw RAM, bypassing ROM mapping. This is how the GA accesses memory.
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.memory.read_byte(addr)
    }
}

impl Bus for Cpc {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF if self.gate_array.lower_rom_enabled() => self.lower_rom[addr as usize],
            0xC000..=0xFFFF if self.gate_array.upper_rom_enabled() => {
                let rom = match self.memory {
                    CpcMemory::Model128K { .. } if self.selected_rom == 7 => 1,
                    _ => 0,
                };
                self.upper_roms[rom][addr as usize - 0xC000]
            }
            _ => self.memory.read(addr, self.memory_banking_selection),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.memory
            .write(addr, value, self.memory_banking_selection);
    }

    fn port_read(&self, port: u16) -> u8 {
        match port {
            0xBC00..=0xBFFF => self.crtc.read(port),
            0xF400..=0xF7FF => self.ppi.read(port),
            0xFB7E => match self.fdc.as_ref() {
                Some(fdc) => fdc.borrow().read_main_status_register(),
                None => todo!("Unexpected port read at address {:#04X}", port),
            },
            0xFB7F => match self.fdc.as_ref() {
                Some(fdc) => fdc.borrow_mut().read_data_register(),
                None => todo!("Unexpected port read at address {:#04X}", port),
            },
            _ => todo!("Unexpected port read at address {:#04X}", port),
        }
    }

    fn port_write(&mut self, port: u16, value: u8) {
        match port {
            // TODO: handle
            0xEF00..=0xEFFF | 0xF800..=0xF8FF => {}
            0xBC00..=0xBFFF => self.crtc.write(port, value),
            0xDF00..=0xDFFF => self.selected_rom = value,
            0xF400..=0xF7FF => self.ppi.write(port, value),
            0x7F00..=0x7FFF => {
                self.gate_array.write(port, value);
                if (value & 0xC0) == 0xC0 && matches!(self.memory, CpcMemory::Model128K { .. }) {
                    self.memory_banking_selection = value & 0x07;
                }
            }
            0xFA7E => match self.fdc.as_ref() {
                Some(fdc) => fdc.borrow_mut().set_motor(value & 1 != 0),
                None => panic!(
                    "Unexpected port write at address {:#04X} with value {:#02x}",
                    port, value
                ),
            },
            0xFB7F => match self.fdc.as_ref() {
                Some(fdc) => fdc.borrow_mut().write_data_register(value),
                None => panic!(
                    "Unexpected port write at address {:#04X} with value {:#02x}",
                    port, value
                ),
            },
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
    use crate::ScreenMode;

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
        let rom = create_test_rom();

        Cpc::new_464(&rom)
    }

    /// Advance the CPC by exactly one full CRTC scanline worth of cycles
    /// (i.e. `R0 + 1` calls to `tick()`).
    fn tick_one_scanline(cpc: &mut Cpc) {
        for _ in 0..=cpc.crtc().register(0) {
            cpc.tick();
        }
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

    #[test]
    fn crtc_port_select_register_routed() {
        let mut cpc = create_cpc();
        cpc.port_write(0xBC00, 12);
        assert_eq!(cpc.crtc().selected_register(), 12);
        // Low byte of port is don't-care
        cpc.port_write(0xBCFF, 7);
        assert_eq!(cpc.crtc().selected_register(), 7);
    }

    #[test]
    fn crtc_port_data_write_routed() {
        let mut cpc = create_cpc();
        cpc.port_write(0xBC00, 12); // R12 is readable on Type 0
        cpc.port_write(0xBD00, 0x30);
        cpc.port_write(0xBC00, 12);
        assert_eq!(cpc.port_read(0xBF00), 0x30);
    }

    #[test]
    fn crtc_port_data_write_low_byte_dont_care() {
        let mut cpc = create_cpc();
        cpc.port_write(0xBC00, 12);
        cpc.port_write(0xBDAA, 0x21);
        cpc.port_write(0xBC00, 12);
        assert_eq!(cpc.port_read(0xBF00), 0x21);
    }

    #[test]
    fn crtc_write_does_not_leak_into_gate_array() {
        let mut cpc = create_cpc();
        cpc.port_write(0xBC00, 0);
        cpc.port_write(0xBD00, 0x82);
        assert_eq!(cpc.gate_array().mode(), ScreenMode::Mode1);
    }

    #[test]
    fn ga_write_does_not_leak_into_crtc() {
        let mut cpc = create_cpc();
        cpc.port_write(0x7F00, 0x82); // GA mode 2
        cpc.port_write(0xBC00, 12);
        assert_eq!(cpc.port_read(0xBF00), 0); // R12 untouched
        assert_eq!(cpc.gate_array().mode(), ScreenMode::Mode2);
    }

    fn setup_small_frame_via_ports(cpc: &mut Cpc) {
        // Same setup as crtc::tests::setup_small_frame, but through the bus
        let regs: [(u8, u8); 10] = [
            (0, 4),
            (1, 2),
            (2, 3),
            (3, 0x12),
            (4, 2),
            (5, 0),
            (6, 1),
            (7, 1),
            (8, 0),
            (9, 1),
        ];
        for (reg, val) in regs {
            cpc.port_write(0xBC00, reg);
            cpc.port_write(0xBD00, val);
        }
        cpc.port_write(0xBC00, 12);
        cpc.port_write(0xBD00, 0);
        cpc.port_write(0xBC00, 13);
        cpc.port_write(0xBD00, 0);
    }

    #[test]
    fn tick_advances_crtc_by_one_character() {
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc); // R0=4 → 5 chars per line
        assert_eq!(cpc.crtc().c0(), 0);
        cpc.tick();
        assert_eq!(cpc.crtc().c0(), 1);
    }

    #[test]
    fn tick_advances_crtc_one_line() {
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc); // R0=4 → 5 chars per line
        assert_eq!(cpc.crtc().c0(), 0);
        tick_one_scanline(&mut cpc);
        assert_eq!(cpc.crtc().c0(), 0); // wrapped
        assert_eq!(cpc.crtc().c9(), 1); // advanced one scanline within the char row
    }

    #[test]
    fn tick_propagates_one_hsync_to_gate_array() {
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc);
        // Run exactly 52 scanlines — GA should fire interrupt on the 52nd HSYNC.
        for _ in 0..51 {
            tick_one_scanline(&mut cpc);
            assert!(
                !cpc.gate_array().interrupt_requested(),
                "No interrupt expected before 52 HSYNCs"
            );
        }
        tick_one_scanline(&mut cpc);
        assert!(
            cpc.gate_array().interrupt_requested(),
            "GA interrupt must fire after 52 HSYNC falling edges propagated from CRTC"
        );
    }

    #[test]
    fn tick_propagates_vsync_to_gate_array() {
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc); // VSYNC at C4=1, C9=0 → after 2 scanlines
        assert_eq!(cpc.gate_array().vsync(), false);
        tick_one_scanline(&mut cpc);
        assert_eq!(cpc.gate_array().vsync(), false);
        tick_one_scanline(&mut cpc);
        assert_eq!(cpc.gate_array().vsync(), true);
    }

    #[test]
    fn tick_propagates_vsync_to_ppi_port_b_bit0() {
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc);
        // Before VSYNC: bit 0 should be 0
        assert_eq!(
            cpc.port_read(0xF500) & 0x01,
            0,
            "PPI Port B bit 0 should be 0 when CRTC VSYNC inactive"
        );

        // Run 2 scanlines to reach C4=1, C9=0 (VSYNC starts)
        tick_one_scanline(&mut cpc);
        tick_one_scanline(&mut cpc);
        assert_eq!(cpc.gate_array().vsync(), true);

        assert_ne!(
            cpc.port_read(0xF500) & 0x01,
            0,
            "PPI Port B bit 0 must follow CRTC VSYNC (high during VSYNC)"
        );

        // After VSYNC width (1 scanline in this setup), it should go low again
        tick_one_scanline(&mut cpc);
        assert_eq!(
            cpc.port_read(0xF500) & 0x01,
            0,
            "PPI Port B bit 0 should drop when CRTC VSYNC ends"
        );
    }

    #[test]
    fn tick_does_not_double_fire_ga_hsync() {
        // Each scanline must produce exactly one GA.hsync() call.
        // Verify by counting scanlines needed to fire interrupt: must be 52, not fewer.
        let mut cpc = create_cpc();
        setup_small_frame_via_ports(&mut cpc);
        for _ in 0..51 {
            tick_one_scanline(&mut cpc);
            assert!(!cpc.gate_array().interrupt_requested());
        }
    }

    #[test]
    fn read_ram_bypasses_upper_rom() {
        let mut cpc = create_cpc();
        // Upper ROM is enabled by default — bus.read(0xC000) returns ROM
        assert_eq!(cpc.read(0xC000), 0x22); // ROM byte
        // Write to RAM at 0xC000
        cpc.write(0xC000, 0x99);
        // read_ram should return the RAM value, not ROM
        assert_eq!(cpc.read_ram(0xC000), 0x99);
    }

    #[test]
    fn read_ram_bypasses_lower_rom() {
        let mut cpc = create_cpc();
        assert_eq!(cpc.read(0x1000), 0x11); // Lower ROM
        cpc.write(0x1000, 0x77);
        assert_eq!(cpc.read_ram(0x1000), 0x77);
    }

    fn create_test_6128_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0xC000];
        for i in 0..0x4000 {
            rom[i] = 0xAA; // Lower
        }
        for i in 0x4000..0x8000 {
            rom[i] = 0xBB; // Upper 0
        }
        for i in 0x8000..0xC000 {
            rom[i] = 0xCC; // Upper 7
        }
        rom
    }

    fn create_6128() -> Cpc {
        let rom = create_test_6128_rom();
        Cpc::new_6128(&rom)
    }

    #[test]
    fn test_6128_rom_default_mapping() {
        let cpc = create_6128();

        // Lower ROM should be active by default
        assert_eq!(
            cpc.read(0x0000),
            0xAA,
            "Lower ROM should be mapped to 0x0000"
        );

        // Upper ROM 0 (BASIC) should be active by default at 0xC000
        assert_eq!(
            cpc.read(0xC000),
            0xBB,
            "Upper ROM 0 should be mapped to 0xC000"
        );
    }

    #[test]
    fn test_upper_rom_selection_via_port_df() {
        let mut cpc = create_6128();

        // Default is ROM 0
        assert_eq!(cpc.read(0xC000), 0xBB);

        // Select ROM 7 (AMSDOS)
        cpc.port_write(0xDF00, 7);
        assert_eq!(
            cpc.read(0xC000),
            0xCC,
            "Upper ROM 7 should be mapped after OUT &DF00,7"
        );

        // Select ROM 0 (BASIC) back
        cpc.port_write(0xDF00, 0);
        assert_eq!(
            cpc.read(0xC000),
            0xBB,
            "Upper ROM 0 should be mapped after OUT &DF00,0"
        );

        // Select unpopulated ROM -> should return ROM 0
        for rom in 1..7 {
            cpc.port_write(0xDF00, rom);
            assert_eq!(
                cpc.read(0xC000),
                0xBB,
                "Unpopulated Upper ROM should fallback to ROM 0"
            );
        }
    }

    #[test]
    fn test_upper_rom_selection_persists_across_memory_writes() {
        let mut cpc = create_6128();

        cpc.port_write(0xDF00, 7);
        cpc.write(0xC000, 0x42); // Writes to RAM, not ROM

        // Reading should still return ROM 7 data, not the written RAM data
        assert_eq!(cpc.read(0xC000), 0xCC);
    }

    /// Helper to write MMR config
    fn write_mmr(cpc: &mut Cpc, config: u8) {
        let value = 0xC0 | (config & 0x07);
        cpc.port_write(0x7F00, value);
    }

    /// Helper to disable both ROMs so reads hit the underlying RAM
    fn disable_roms(cpc: &mut Cpc) {
        cpc.port_write(0x7F00, 0x8C); // 1000_1100 -> Mode 0, ROMs off
    }

    #[test]
    fn test_mmr_config_0_default_linear_mapping() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);
        write_mmr(&mut cpc, 0);

        cpc.write(0x0000, 0x10);
        cpc.write(0x4000, 0x11);
        cpc.write(0x8000, 0x12);
        cpc.write(0xC000, 0x13);

        assert_eq!(cpc.read(0x0000), 0x10);
        assert_eq!(cpc.read(0x4000), 0x11);
        assert_eq!(cpc.read(0x8000), 0x12);
        assert_eq!(cpc.read(0xC000), 0x13);
    }

    #[test]
    fn test_mmr_config_1_bank1_at_c000() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);

        // Config 0: Write to 0xC000 goes to Bank 0, Block 3
        write_mmr(&mut cpc, 0);
        cpc.write(0xC000, 0x33);

        // Config 1: Write to 0xC000 goes to Bank 1, Block 3 (3*)
        write_mmr(&mut cpc, 1);
        cpc.write(0xC000, 0x77);

        // Verify Config 0
        write_mmr(&mut cpc, 0);
        assert_eq!(
            cpc.read(0xC000),
            0x33,
            "Config 0 C000 should read Bank 0 Block 3"
        );

        // Verify Config 1
        write_mmr(&mut cpc, 1);
        assert_eq!(
            cpc.read(0xC000),
            0x77,
            "Config 1 C000 should read Bank 1 Block 3"
        );
    }

    #[test]
    fn test_mmr_config_2_full_bank1_mapping() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);
        write_mmr(&mut cpc, 2); // 0*, 1*, 2*, 3*

        cpc.write(0x0000, 0x44); // Block 4
        cpc.write(0x4000, 0x55); // Block 5
        cpc.write(0x8000, 0x66); // Block 6
        cpc.write(0xC000, 0x77); // Block 7

        assert_eq!(cpc.read(0x0000), 0x44);
        assert_eq!(cpc.read(0x4000), 0x55);
        assert_eq!(cpc.read(0x8000), 0x66);
        assert_eq!(cpc.read(0xC000), 0x77);

        // Ensure Bank 0 is untouched
        write_mmr(&mut cpc, 0);
        assert_ne!(cpc.read(0x0000), 0x44);
        assert_ne!(cpc.read(0x4000), 0x55);
    }

    #[test]
    fn test_mmr_config_3_mixed_mapping() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);

        // Config 0: Map standard blocks
        write_mmr(&mut cpc, 0);
        cpc.write(0x4000, 0x11); // Block 1
        cpc.write(0xC000, 0x33); // Block 3

        // Config 3: 0, 3, 2, 3*
        write_mmr(&mut cpc, 3);
        cpc.write(0x4000, 0x33); // Block 3 (Bank 0)
        cpc.write(0xC000, 0x77); // Block 3 (Bank 1)

        // Verify 0x4000 sees Block 3 (0x33), not Block 1 (0x11)
        assert_eq!(cpc.read(0x4000), 0x33);

        // Verify 0xC000 sees Bank 1 Block 3 (0x77)
        assert_eq!(cpc.read(0xC000), 0x77);

        // Switch back to Config 0 to verify Bank 0 Block 3 was overwritten by 0x4000 write
        write_mmr(&mut cpc, 0);
        assert_eq!(
            cpc.read(0x4000),
            0x11,
            "Config 0 0x4000 should still be Block 1"
        );
        assert_eq!(
            cpc.read(0xC000),
            0x33,
            "Config 0 0xC000 should be Block 3, which was overwritten"
        );
        assert_eq!(
            cpc.read(0x8000),
            0xFF,
            "Config 0 0x8000 should be Block 2, which was untouched"
        );
    }

    #[test]
    fn test_mmr_config_4_to_7_ram_expansion_protocol() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);

        // Config 4: 0, 0*, 2, 3
        write_mmr(&mut cpc, 4);
        cpc.write(0x4000, 0x44); // Block 4

        // Config 5: 0, 1*, 2, 3
        write_mmr(&mut cpc, 5);
        cpc.write(0x4000, 0x55); // Block 5

        // Config 6: 0, 2*, 2, 3
        write_mmr(&mut cpc, 6);
        cpc.write(0x4000, 0x66); // Block 6

        // Config 7: 0, 3*, 2, 3
        write_mmr(&mut cpc, 7);
        cpc.write(0x4000, 0x77); // Block 7

        // Verify isolation
        write_mmr(&mut cpc, 4);
        assert_eq!(cpc.read(0x4000), 0x44);

        write_mmr(&mut cpc, 5);
        assert_eq!(cpc.read(0x4000), 0x55);

        write_mmr(&mut cpc, 6);
        assert_eq!(cpc.read(0x4000), 0x66);

        write_mmr(&mut cpc, 7);
        assert_eq!(cpc.read(0x4000), 0x77);
    }

    #[test]
    fn test_gate_array_ignores_mmr_writes() {
        let mut cpc = create_6128();

        // Set a known state: Mode 2, ROMs enabled
        cpc.port_write(0x7F00, 0x82); // 1000_0010

        // Write MMR config 1 (1100_0001)
        write_mmr(&mut cpc, 1);

        // GA should have ignored bits 7,6=11.
        // Mode should still be 2, ROMs should still be enabled.
        assert_eq!(
            cpc.gate_array().mode(),
            ScreenMode::Mode2,
            "GA Mode must not change on MMR write"
        );
        assert!(
            cpc.gate_array().lower_rom_enabled(),
            "Lower ROM state must not change on MMR write"
        );
        assert!(
            cpc.gate_array().upper_rom_enabled(),
            "Upper ROM state must not change on MMR write"
        );
    }

    #[test]
    fn test_rom_enable_overrides_mmr_mapping() {
        let mut cpc = create_6128();

        // MMR Config 2 maps Bank 1 to 0x0000-0x3FFF
        write_mmr(&mut cpc, 2);

        // But Lower ROM is enabled by default, so 0x0000 should read Lower ROM
        assert_eq!(
            cpc.read(0x0000),
            0xAA,
            "Lower ROM should override Bank 1 mapping"
        );

        // Disable Lower ROM
        cpc.port_write(0x7F00, 0x84); // 1000_0100 (Mode 0, Lower ROM off)

        // Now it should read Bank 1 Block 0 (Block 4)
        // We haven't written anything there, so it should be 0x00 (from Vec initialization)
        assert_eq!(
            cpc.read(0x0000),
            0xFF,
            "Should read Bank 1 after disabling Lower ROM"
        );

        // Write to Bank 1 and verify
        cpc.write(0x0000, 0x99);
        assert_eq!(cpc.read(0x0000), 0x99);

        // Re-enable Lower ROM
        cpc.port_write(0x7F00, 0x80); // 1000_0000
        assert_eq!(cpc.read(0x0000), 0xAA, "Lower ROM should be back");
    }

    #[test]
    fn test_video_reads_always_bank_0() {
        let mut cpc = create_6128();
        disable_roms(&mut cpc);

        // Config 0: Write 0xAA to 0xC000 (Bank 0, Block 3)
        write_mmr(&mut cpc, 0);
        cpc.write(0xC000, 0xAA);

        // Config 1: Write 0xBB to 0xC000 (Bank 1, Block 3)
        write_mmr(&mut cpc, 1);
        cpc.write(0xC000, 0xBB);

        // If the Gate Array reads video memory, it MUST read from Bank 0,
        // regardless of the active MMR config (which is currently 1).
        let video_byte = cpc.read_ram(0xC000);
        assert_eq!(
            video_byte, 0xAA,
            "Video fetcher must read Bank 0, ignoring MMR config"
        );

        // Same for lower memory
        write_mmr(&mut cpc, 0);
        cpc.write(0x0000, 0x11); // Bank 0
        write_mmr(&mut cpc, 2);
        cpc.write(0x0000, 0x22); // Bank 1

        let video_byte_low = cpc.read_ram(0x0000);
        assert_eq!(
            video_byte_low, 0x11,
            "Video fetcher must read Bank 0 at low memory"
        );
    }

    fn create_test_rom6128() -> [u8; 0xC000] {
        let mut rom = [0; 0xC000];
        for i in 0..0x4000 {
            rom[i] = 0x11;
        }
        for i in 0x4000..0x8000 {
            rom[i] = 0x22;
        }
        for i in 0x8000..0xC000 {
            rom[i] = 0x33;
        }
        rom
    }

    fn create_cpc6128() -> Cpc {
        let rom = create_test_rom6128();

        Cpc::new_6128(&rom)
    }

    #[test]
    fn fdc_msr_readable_at_port_fb7e() {
        let cpc = create_cpc6128();
        // MSR must have RQM set when FDC is idle in Command phase
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(msr & 0x80, 0, "RQM must be set when FDC is idle");
        assert_eq!(msr & 0x40, 0, "DIO must be 0 in Command phase (CPU writes)");
    }

    // TODO
    // #[test]
    // fn fdc_msr_low_byte_is_dont_care() {
    //     let cpc = create_cpc6128();
    //     // Any address in 0xFBxx with A0=0 should return the MSR
    //     let msr_7e = cpc.port_read(0xFB7E);
    //     let msr_00 = cpc.port_read(0xFB00);
    //     let msr_ff = cpc.port_read(0xFBFE);
    //     assert_eq!(
    //         msr_7e, msr_00,
    //         "MSR should be accessible at any 0xFBxx with A0=0"
    //     );
    //     assert_eq!(
    //         msr_7e, msr_ff,
    //         "MSR should be accessible at any 0xFBxx with A0=0"
    //     );
    // }

    #[test]
    fn fdc_motor_control_at_port_fa7e() {
        let mut cpc = create_cpc6128();
        cpc.port_write(0xFA7E, 0x01);
        assert!(
            cpc.fdc().unwrap().motor_state(),
            "Motor must be on after writing 0x01 to 0xFA7E"
        );
        cpc.port_write(0xFA7E, 0x00);
        assert!(
            !cpc.fdc().unwrap().motor_state(),
            "Motor must be off after writing 0x00 to 0xFA7E"
        );
    }

    #[test]
    fn fdc_motor_write_does_not_affect_data_register() {
        let mut cpc = create_cpc6128();
        // Writing to motor port should not be interpreted as a command byte
        cpc.port_write(0xFA7E, 0x01);
        // FDC should still be in Command phase, ready for a new command
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(msr & 0x80, 0, "RQM must still be set after motor write");
        assert_eq!(
            msr & 0x10,
            0,
            "CB must be clear — motor write is not a command"
        );
    }

    #[test]
    fn fdc_version_command_through_cpc_bus() {
        let mut cpc = create_cpc6128();
        // Issue Version command (opcode 0x10) via the data register port
        cpc.port_write(0xFB7F, 0x10);
        // Tick to let the FDC process
        for _ in 0..200 {
            cpc.tick();
        }
        // Poll MSR until DIO=1 (Result phase)
        for _ in 0..10_000 {
            let msr = cpc.port_read(0xFB7E);
            if (msr & 0x40) != 0 && (msr & 0x80) != 0 {
                break;
            }
            cpc.tick();
        }
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(msr & 0x40, 0, "DIO must be 1 in Result phase");
        // Read the single result byte
        let version = cpc.port_read(0xFB7F);
        assert!(
            version == 0x80 || version == 0x90,
            "Version should be 0x80 (765A) or 0x90 (765B), got {:#x}",
            version
        );
    }

    #[test]
    fn fdc_seek_and_sense_interrupt_through_cpc_bus() {
        let mut cpc = create_cpc6128();
        // Seek command: opcode 0x0F, drive 0, NCN=5
        cpc.port_write(0xFB7F, 0x0F);
        cpc.port_write(0xFB7F, 0x00);
        cpc.port_write(0xFB7F, 0x05);
        // Tick enough cycles for the seek to complete
        for _ in 0..50_000 {
            cpc.tick();
        }
        // Sense Interrupt Status: opcode 0x08
        cpc.port_write(0xFB7F, 0x08);
        for _ in 0..200 {
            cpc.tick();
        }
        // Read 2 result bytes: ST0, PCN
        let st0 = cpc.port_read(0xFB7F);
        let pcn = cpc.port_read(0xFB7F);
        assert_ne!(st0 & 0x20, 0, "ST0 SE must be set after seek completion");
        assert_eq!(pcn, 5, "PCN must be 5 after seeking to track 5");
    }

    // TODO:
    // #[test]
    // fn fdc_write_to_msr_port_ignored() {
    //     let mut cpc = create_cpc6128();
    //     // MSR is read-only; writing to 0xFB7E should not corrupt FDC state
    //     cpc.port_write(0xFB7E, 0xFF);
    //     let msr = cpc.port_read(0xFB7E);
    //     assert_ne!(
    //         msr & 0x80,
    //         0,
    //         "RQM must still be set after writing to read-only MSR port"
    //     );
    // }

    #[test]
    fn fdc_port_does_not_interfere_with_crtc() {
        let mut cpc = create_cpc6128();
        // Writing to FDC data register should not affect CRTC registers
        cpc.port_write(0xBC00, 12); // Select CRTC R12
        cpc.port_write(0xBD00, 0x30);
        cpc.port_write(0xFB7F, 0x10); // FDC Version command
        // CRTC R12 should be unchanged
        cpc.port_write(0xBC00, 12);
        assert_eq!(
            cpc.port_read(0xBF00),
            0x30,
            "CRTC R12 must not be affected by FDC write"
        );
    }

    #[test]
    fn fdc_port_does_not_interfere_with_gate_array() {
        let mut cpc = create_cpc6128();
        // FDC port writes should not be interpreted as Gate Array commands
        cpc.port_write(0xFB7F, 0x82); // This looks like GA mode register, but goes to FDC
        assert_eq!(
            cpc.gate_array().mode(),
            ScreenMode::Mode1,
            "GA mode must not change from FDC port write"
        );
    }

    #[test]
    fn fdc_tick_advances_seek_timing() {
        let mut cpc = create_cpc6128();
        // Issue a seek command
        cpc.port_write(0xFB7F, 0x0F);
        cpc.port_write(0xFB7F, 0x00);
        cpc.port_write(0xFB7F, 0x0A); // Seek to track 10
        // Before ticking, the seek should not be complete
        // (FDC is in Execution phase, D0B should be set)
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(msr & 0x01, 0, "D0B must be set during seek on drive 0");
        // Tick enough for seek completion
        for _ in 0..50_000 {
            cpc.tick();
        }
        // After completion, D0B should be clear
        let msr = cpc.port_read(0xFB7E);
        assert_eq!(msr & 0x01, 0, "D0B must be clear after seek completes");
    }

    #[test]
    fn fdc_command_phase_locked_until_results_read() {
        let mut cpc = create_cpc6128();
        // Issue Version command
        cpc.port_write(0xFB7F, 0x10);
        for _ in 0..200 {
            cpc.tick();
        }
        // FDC should be in Result phase — DIO=1
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(msr & 0x40, 0, "DIO must be 1 in Result phase");
        // Try writing a new command without reading the result — should be ignored
        cpc.port_write(0xFB7F, 0x03); // Specify opcode
        for _ in 0..200 {
            cpc.tick();
        }
        // Still in Result phase — locked
        let msr = cpc.port_read(0xFB7E);
        assert_ne!(
            msr & 0x40,
            0,
            "FDC must remain locked in Result phase until all bytes are read"
        );
        // Now read the result byte
        let _ = cpc.port_read(0xFB7F);
        // Should be back in Command phase
        let msr = cpc.port_read(0xFB7E);
        assert_eq!(
            msr & 0x40,
            0,
            "DIO must be 0 after reading all result bytes"
        );
    }
}
