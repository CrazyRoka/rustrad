use crate::{Crtc, GateArray, Ppi, Video, memory::CpcMemory};
use z80::Bus;

pub struct Cpc {
    rom: [u8; 0x8000], // 32 KB
    memory: CpcMemory,
    // Peripherals
    gate_array: GateArray,
    ppi: Ppi,
    crtc: Crtc,
    video: Video,
}

impl Cpc {
    pub fn new(memory: CpcMemory, rom: &[u8]) -> Self {
        assert_eq!(rom.len(), 0x8000, "ROM length is supposed to be 32KB");
        let mut rom_clone = [0; 0x8000];
        rom_clone.copy_from_slice(rom);

        Self {
            rom: rom_clone,
            memory,
            gate_array: GateArray::new(),
            ppi: Ppi::new(),
            crtc: Crtc::new(),
            video: Video::new(),
        }
    }

    pub fn gate_array_mut(&mut self) -> &mut GateArray {
        &mut self.gate_array
    }

    pub fn gate_array(&self) -> &GateArray {
        &self.gate_array
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
        self.ppi.tick_tape(4)
    }

    /// Reads raw RAM, bypassing ROM mapping. This is how the GA accesses memory.
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.memory.read(addr)
    }
}

impl Bus for Cpc {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF if self.gate_array.lower_rom_enabled() => self.rom[addr as usize],
            0xC000..=0xFFFF if self.gate_array.upper_rom_enabled() => {
                self.rom[addr as usize - 0x8000]
            }
            _ => self.memory.read(addr),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.memory.write(addr, value);
    }

    fn port_read(&self, port: u16) -> u8 {
        match port >> 8 {
            0xBC..=0xBF => self.crtc.read(port),
            0xF4..=0xF7 => self.ppi.read(port),
            _ => todo!("Unexpected port read at address {:#04X}", port),
        }
    }

    fn port_write(&mut self, port: u16, value: u8) {
        match port >> 8 {
            // TODO: handle
            0xEF | 0xDF | 0xF8 => {}
            0xBC..=0xBF => self.crtc.write(port, value),
            0xF4..=0xF7 => self.ppi.write(port, value),
            0x7F => self.gate_array.write(port, value),
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
        let memory = CpcMemory::new_64k();
        let rom = create_test_rom();

        Cpc::new(memory, &rom)
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
}
