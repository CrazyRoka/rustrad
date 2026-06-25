pub struct Ppi {}

impl Ppi {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr >> 8 {
            0xF5 => self.read_port_b(),
            0xF6 => self.read_port_c(),
            _ => todo!("Implement port {:#04X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        todo!()
    }

    fn read_port_b(&self) -> u8 {
        // TODO: implement cassette read
        // bit 7 - Cassette Read Data
        const CASSETTE_READ: u8 = 1 << 7;

        // TODO: implement Parallel Port
        // bit 6 - Parallel Port Busy
        const PARALLEL_PORT_BUSY: u8 = 1 << 6;

        // TODO: implement /EXP
        // bit 5 - /EXP
        const EXP: u8 = 1 << 5;

        // TODO: Consider implementing 50 - 60hz frequency switch
        // bit 4 - Screen Frequency
        const SCREEN_FREQUENCY: u8 = 1 << 4;

        // TODO: different values?
        // bit 3 - LK Jumper 3
        // bit 2 - LK Jumper 2
        // bit 1 - LK Jumper 1
        const MANUFACTURER_JUMPER: u8 = (1 << 3) | (1 << 2) | (1 << 1);

        // TODO: handle CRTC VSYNC
        // bit 0 - CRTC VSYNC
        const VSYNC: u8 = 1 << 0;

        CASSETTE_READ * 0
            + PARALLEL_PORT_BUSY * 1
            + EXP * 1
            + SCREEN_FREQUENCY * 1
            + MANUFACTURER_JUMPER
            + VSYNC * 1
    }

    fn read_port_c(&self) -> u8 {
        // TODO: implement PSG
        // bit 7 - PSG BDIR
        const PSG_BDIR: u8 = 1 << 7;

        // TODO: implement PSG
        // bit 6 - PSG BC1
        const PSG_BC1: u8 = 1 << 6;

        // TODO: implement Cassette
        // bit 5 - Cassette Write Data
        const CASSETTE_WRITE: u8 = 1 << 5;

        // TODO: Implement Cassette
        // bit 4 - Cassette Motor
        const CASSETTE_MOTOR: u8 = 1 << 4;

        // TODO: Implement Keyboard
        // bit 3-0 - Keyboard Row Select
        const KEYBOARD_ROW: u8 = (1 << 3) | (1 << 2) | (1 << 1) | (1 << 0);

        PSG_BDIR * 1 + PSG_BC1 * 1 + CASSETTE_WRITE * 1 + CASSETTE_MOTOR * 1 + KEYBOARD_ROW * 1
    }
}
