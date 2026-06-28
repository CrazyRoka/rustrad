use std::cell::Cell;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrtcType {
    /// UM6845 / HD6845S — Standard in early CPC 464 models.
    Type0,
    /// UM6845R — Common in mid-generation models. Features readable status.
    Type1,
    /// MC6845 — Motorola. Common in late-generation CPC 464/6128 mainboards.
    Type2,
    /// AMS40489 — ASIC-integrated CRTC. Standard in CPC+ series.
    Type3,
    /// AMS40226 (Pre-ASIC) — Integrated in cost-down CPC 6128 mainboards.
    Type4,
}

pub struct Crtc {
    crtc_type: CrtcType,
    registers: [u8; 32],
    selected_register: u8,
    lpstb_prev: bool,
    lpen_full: Cell<bool>,
    // Internal Counters
    c0: u8,
    c3l: u8,
    c3h: u8,
    c4: u8,
    c5: u8,
    c9: u8,
    vma: u16,
    vma_prime: u16,
    char_row_start_ma: u16,
}

impl Crtc {
    /// Creates a new CRTC defaulting to Type 0 (UM6845 / HD6845S).
    pub fn new() -> Self {
        Self::new_with_type(CrtcType::Type0)
    }

    /// Creates a new CRTC with the specified hardware type.
    pub fn new_with_type(crtc_type: CrtcType) -> Self {
        Self {
            crtc_type,
            registers: [0; 32],
            selected_register: 0,
            lpstb_prev: false,
            lpen_full: Cell::new(false),
            c0: 0,
            c3l: 0,
            c3h: 0,
            c4: 0,
            c5: 0,
            c9: 0,
            vma: 0,
            vma_prime: 0,
            char_row_start_ma: 0,
        }
    }

    /// Returns the CRTC hardware type.
    pub fn crtc_type(&self) -> CrtcType {
        self.crtc_type
    }

    /// Reads from the CRTC at the given I/O address.
    ///
    /// Address decoding (A14 = 0 selects CRTC):
    /// - `&BCxx` (A9=0, A8=0): Not meaningful for reads.
    /// - `&BDxx` (A9=0, A8=1): Not meaningful for reads.
    /// - `&BExx` (A9=1, A8=0): Status register (Type 1) or floating bus.
    /// - `&BFxx` (A9=1, A8=1): Register data read.
    pub fn read(&self, addr: u16) -> u8 {
        match addr >> 8 {
            0xBE if self.crtc_type == CrtcType::Type0 => 127,
            0xBE if self.crtc_type == CrtcType::Type1 => self.status_read(),
            0xBE if self.crtc_type == CrtcType::Type2 => 255,
            0xBF => self.register_read(),
            _ => todo!(),
        }
    }

    /// Writes to the CRTC at the given I/O address.
    ///
    /// Address decoding (A14 = 0 selects CRTC):
    /// - `&BCxx` (A9=0, A8=0): Register Select — writes to the Address Register.
    /// - `&BDxx` (A9=0, A8=1): Register Data Write — writes to the selected register.
    /// - `&BExx` (A9=1, A8=0): Not meaningful for writes.
    /// - `&BFxx` (A9=1, A8=1): Not meaningful for writes.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr >> 8 {
            0xBC => self.register_select(value),
            0xBD => self.register_write(value),
            _ => todo!(),
        }
    }

    /// Returns the currently selected register index (internal Address Register).
    /// On Type 0/1/2 this is 5 bits (0–31).
    /// On Type 3/4 this is 3 bits (0–7).
    pub fn selected_register(&self) -> u8 {
        match self.crtc_type {
            CrtcType::Type0 | CrtcType::Type1 | CrtcType::Type2 => self.selected_register & 0b11111,
            CrtcType::Type3 | CrtcType::Type4 => self.selected_register & 0b111,
        }
    }

    /// Read parameter byte from the currently selected register.
    fn register_read(&self) -> u8 {
        match self.crtc_type {
            CrtcType::Type0 => match self.selected_register() {
                12..=17 => self.registers[self.selected_register() as usize],
                _ => 0,
            },
            CrtcType::Type1 => match self.selected_register() {
                14..=15 => self.registers[self.selected_register() as usize],
                16..=17 => {
                    self.lpen_full.set(false);
                    self.registers[self.selected_register() as usize]
                }
                31 => 127,
                _ => 0,
            },
            CrtcType::Type2 => match self.selected_register() {
                16..=17 => self.registers[self.selected_register() as usize],
                _ => 0,
            },
            CrtcType::Type3 | CrtcType::Type4 => {
                let idx = self.selected_register() as usize;
                self.registers[if idx < 2 { idx + 16 } else { idx + 8 }]
            }
        }
    }

    /// Write parameter byte to the currently selected register.
    fn register_write(&mut self, value: u8) {
        if self.selected_register == 16 || self.selected_register == 17 {
            println!(
                "Unexpected register {} write with value {}",
                self.selected_register, value
            );
            return;
        }
        self.registers[self.selected_register as usize] = value;
        if self.c0 == 0
            && self.c4 == 0
            && self.c9 == 0
            && (self.selected_register == 12 || self.selected_register == 13)
        {
            self.vma_prime = ((self.register(12) as u16) << 8) | (self.register(13) as u16);
            self.vma = self.vma_prime;
            self.char_row_start_ma = self.vma_prime;
        }
    }

    /// Write a register index (0 to 31) to the selected register.
    fn register_select(&mut self, value: u8) {
        self.selected_register = value;
    }

    fn status_read(&self) -> u8 {
        assert_eq!(self.crtc_type, CrtcType::Type1);

        // TODO: implement and test St_R6
        0 + if self.lpen_full.get() { 1 << 6 } else { 0 }
    }

    /// Returns the raw value stored in the internal register at `idx` (0–31).
    /// This bypasses read-port masking / readability rules and returns the
    /// full 8-bit latch value as written.
    pub fn register(&self, idx: u8) -> u8 {
        match idx {
            0x00..=0x31 => self.registers[idx as usize],
            _ => panic!("Unexpected register {idx:#02X} selected"),
        }
    }

    /// Advances the CRTC by exactly one character clock (CCLK).
    ///
    /// After this call:
    /// - All internal counters (C0, C9, C4, C5, MA, VMA, VMA') have been
    ///   updated for the new CCLK.
    /// - All output pins (HSYNC, VSYNC, DISPEN, CURSOR) reflect the new state.
    pub fn tick(&mut self) {
        let c0_wraps = if self.crtc_type == CrtcType::Type0 && self.register(0) == 0 {
            false // Type 0 with R0=0: freeze, never wrap
        } else {
            self.c0 += 1;
            self.c0 == self.register(0) + 1
        };
        if c0_wraps {
            self.c0 = 0;

            if self.c4 < self.register(4) + 1 {
                // Normal character row period
                self.c9 += 1;
                if self.c9 == self.register(9) + 1 {
                    self.c9 = 0;
                    self.c4 += 1;
                    self.char_row_start_ma = self.vma;

                    if self.c4 == self.register(4) + 1 && self.register(5) == 0 {
                        // No vertical adjust: frame ends now
                        self.c4 = 0;
                        self.vma_prime =
                            ((self.register(12) as u16) << 8) | (self.register(13) as u16);
                        self.char_row_start_ma = self.vma_prime;
                    }
                }
            } else {
                // Vertical adjust period: C5 counts individual scanlines
                if self.c5 + 1 >= self.register(5) {
                    // Frame end
                    self.c4 = 0;
                    self.c5 = 0;
                    self.c9 = 0;
                    self.vma_prime = ((self.register(12) as u16) << 8) | (self.register(13) as u16);
                    self.char_row_start_ma = self.vma_prime;
                } else {
                    self.c5 += 1;
                }
            }
        }

        // HSYNC
        {
            if self.c0 == self.register(2) {
                self.c3l = 1;
            } else if self.c3l > 0 {
                self.c3l += 1;
            }

            let width = match self.crtc_type {
                CrtcType::Type0 | CrtcType::Type1 if (self.register(3) & 0x0F) == 0 => 0,
                CrtcType::Type2 | CrtcType::Type3 | CrtcType::Type4
                    if (self.register(3) & 0x0F) == 0 =>
                {
                    16
                }
                _ => (self.register(3) & 0x0F),
            };
            if self.c3l == width + 1 {
                self.c3l = 0;
            }
        }

        // VSYNC
        if self.c0 == 0 {
            if self.c4 == self.register(7) && self.c9 == 0 {
                self.c3h = 1;
            } else if self.c3h > 0 {
                self.c3h += 1;
            }

            let width = match self.crtc_type {
                CrtcType::Type1 | CrtcType::Type2 => 16,
                CrtcType::Type0 | CrtcType::Type3 | CrtcType::Type4 => {
                    if (self.register(3) >> 4) == 0 {
                        16
                    } else {
                        self.register(3) >> 4
                    }
                }
            };

            if self.c3h == width + 1 {
                self.c3h = 0;
            }
        }

        if self.c0 == 0 {
            self.vma = self.vma_prime;
        } else {
            self.vma += 1;
            if self.c0 == self.register(1) && self.c9 == self.register(9) {
                self.vma_prime = self.vma;
            }
        }
    }

    /// Returns the current HSYNC output pin state (active high).
    pub fn hsync(&self) -> bool {
        self.c3l != 0
    }

    /// Returns the current VSYNC output pin state (active high).
    pub fn vsync(&self) -> bool {
        self.c3h != 0
    }

    /// Returns the current Display Enable output state.
    pub fn dispen(&self) -> bool {
        if ((self.register(8) >> 4) & 0b11) == 0b11 {
            return false;
        }

        self.c0 < self.register(1) && self.c4 < self.register(6)
    }

    /// Returns the current Cursor output state.
    pub fn cursor(&self) -> bool {
        if self.register(1) == 0 {
            return false;
        }

        let mode = (self.register(10) >> 5) & 0b11;
        if mode == 0b01 {
            return false;
        }

        let cursor_start = self.register(10) & 0x0F;
        let cursor_end = self.register(11) & 0x0F;
        let cursor_position = ((self.register(14) as u16) << 8) | (self.register(15) as u16);

        self.char_row_start_ma == cursor_position
            && self.c9 >= cursor_start
            && self.c9 <= cursor_end
    }

    /// Returns the current 14-bit memory address (MA) being output.
    pub fn current_ma(&self) -> u16 {
        self.vma
    }

    /// Returns the current raster address output (RA0–RA4).
    pub fn current_raster(&self) -> u8 {
        self.c9
    }

    /// Returns the horizontal character counter (C0).
    pub fn c0(&self) -> u8 {
        self.c0
    }

    /// Returns the vertical character row counter (C4).
    pub fn c4(&self) -> u8 {
        self.c4
    }

    /// Returns the raster counter (C9).
    pub fn c9(&self) -> u8 {
        self.c9
    }

    /// Returns the vertical total adjust counter (C5).
    /// Only meaningful on Type 1/2 which have a dedicated C5 counter.
    pub fn c5(&self) -> u8 {
        self.c5
    }

    /// HSC (Horizontal Sync Counter)
    /// Counts µs inside an HSYNC pulse.
    pub fn c3l(&self) -> u8 {
        self.c3l
    }

    /// VSC (Vertical Sync Counter)
    /// Counts lines inside a VSYNC pulse.
    pub fn c3h(&self) -> u8 {
        self.c3h
    }

    /// Returns the current VMA (active display memory pointer).
    pub fn vma(&self) -> u16 {
        self.vma
    }

    /// Returns the current VMA' (line-start memory pointer).
    pub fn vma_prime(&self) -> u16 {
        self.vma_prime
    }

    /// Sets the light pen strobe (LPSTB) input.
    /// A low-to-high transition (rising edge) captures the current MA into
    /// R16 (high byte) and R17 (low byte), and sets the LPEN_FULL flag.
    fn lpstb(&mut self, value: bool) {
        if value && !self.lpstb_prev {
            let ma = self.vma;
            self.registers[16] = (ma >> 8) as u8;
            self.registers[17] = (ma & 0xFF) as u8;
            self.lpen_full.set(true);
        }
        self.lpstb_prev = value;
    }

    /// Performs a hardware reset of the CRTC.
    /// Clears all internal registers, counters, and output pins.
    fn reset(&mut self) {
        *self = Self::new_with_type(self.crtc_type)
    }

    pub fn phys_address(&self) -> u16 {
        Self::phys_address_for(self.current_ma(), self.current_raster())
    }

    pub fn phys_address_for(ma: u16, raster: u8) -> u16 {
        (ma & 0x3FF) << 1 | (((raster as u16) & 0x07) << 11) | ((ma & 0x3000) << 2)
    }
}

impl Default for Crtc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a value to a CRTC register via the standard two-write sequence.
    fn write_register(crtc: &mut Crtc, reg: u8, value: u8) {
        crtc.write(0xBC00, reg);
        crtc.write(0xBD00, value);
    }

    /// Selects a register and reads its value from port &BFxx.
    fn read_register(crtc: &mut Crtc, reg: u8) -> u8 {
        crtc.write(0xBC00, reg);
        crtc.read(0xBF00)
    }

    /// Sets up a small, easy-to-trace frame:
    ///
    /// - R0=4  → 5 CCLKs per line (C0 counts 0..4, then wraps)
    /// - R1=2  → 2 chars displayed
    /// - R2=3  → HSYNC starts at C0=3
    /// - R3=0x14 → VSYNC width = 1 scanline, HSYNC width = 4 CCLKs
    /// - R4=2  → 3 character rows (C4 counts 0..2)
    /// - R5=0  → no vertical adjust
    /// - R6=1  → 1 character row displayed
    /// - R7=1  → VSYNC at C4=1
    /// - R8=0  → non-interlace, no skew
    /// - R9=1  → 2 scanlines per char row (C9 counts 0..1)
    ///
    /// Total: 3 rows × 2 scanlines × 5 CCLKs = 30 CCLKs per frame.
    ///
    /// Scanline layout:
    ///   tick  0–4:  C4=0 C9=0  (displayed)
    ///   tick  5–9:  C4=0 C9=1  (displayed)
    ///   tick 10–14: C4=1 C9=0  (VSYNC active)
    ///   tick 15–19: C4=1 C9=1
    ///   tick 20–24: C4=2 C9=0
    ///   tick 25–29: C4=2 C9=1
    ///   tick 30:    frame wraps → C4=0 C9=0 C0=0
    fn setup_small_frame(crtc: &mut Crtc) {
        write_register(crtc, 0, 4);
        write_register(crtc, 1, 2);
        write_register(crtc, 2, 3);
        write_register(crtc, 3, 0x14);
        write_register(crtc, 4, 2);
        write_register(crtc, 5, 0);
        write_register(crtc, 6, 1);
        write_register(crtc, 7, 1);
        write_register(crtc, 8, 0);
        write_register(crtc, 9, 1);
        write_register(crtc, 12, 0);
        write_register(crtc, 13, 0);
    }

    /// Sets up a minimal single-scanline frame for HSYNC-focused tests.
    ///
    /// - R0=10 → 11 CCLKs per line
    /// - R1=5  → 5 chars displayed
    /// - R2=6  → HSYNC at C0=6
    /// - R3=0x02 → HSYNC width = 2
    /// - R4=0, R9=0 → single scanline per frame
    fn setup_hsync_frame(crtc: &mut Crtc) {
        write_register(crtc, 0, 10);
        write_register(crtc, 1, 5);
        write_register(crtc, 2, 6);
        write_register(crtc, 3, 0x02);
        write_register(crtc, 4, 0);
        write_register(crtc, 5, 0);
        write_register(crtc, 6, 1);
        write_register(crtc, 7, 0);
        write_register(crtc, 8, 0);
        write_register(crtc, 9, 0);
        write_register(crtc, 12, 0);
        write_register(crtc, 13, 0);
    }

    #[test]
    fn new_crtc_default_type_is_type0() {
        let crtc = Crtc::new();
        assert_eq!(crtc.crtc_type(), CrtcType::Type0);
    }

    #[test]
    fn new_with_type_sets_crtc_type() {
        let types = [
            CrtcType::Type0,
            CrtcType::Type1,
            CrtcType::Type2,
            CrtcType::Type3,
            CrtcType::Type4,
        ];
        for t in types {
            let crtc = Crtc::new_with_type(t);
            assert_eq!(crtc.crtc_type(), t);
        }
    }

    #[test]
    fn new_crtc_all_registers_zero() {
        let crtc = Crtc::new();
        for reg in 0..=31u8 {
            assert_eq!(crtc.register(reg), 0, "Register {} should be 0 on new", reg);
        }
    }

    #[test]
    fn new_crtc_selected_register_is_zero() {
        let crtc = Crtc::new();
        assert_eq!(crtc.selected_register(), 0);
    }

    #[test]
    fn new_crtc_all_counters_zero() {
        let crtc = Crtc::new();
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c4(), 0);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c5(), 0);
        assert_eq!(crtc.c3l(), 0);
        assert_eq!(crtc.c3h(), 0);
    }

    #[test]
    fn new_crtc_all_outputs_inactive() {
        let crtc = Crtc::new();
        assert!(!crtc.hsync());
        assert!(!crtc.vsync());
        assert!(!crtc.dispen());
        assert!(!crtc.cursor());
    }

    #[test]
    fn new_crtc_ma_is_zero() {
        let crtc = Crtc::new();
        assert_eq!(crtc.current_ma(), 0);
    }

    #[test]
    fn write_to_bcxx_selects_register() {
        let mut crtc = Crtc::new();
        crtc.write(0xBC00, 7);
        assert_eq!(crtc.selected_register(), 7);
    }

    #[test]
    fn write_to_bdxx_writes_register_data() {
        let mut crtc = Crtc::new();
        crtc.write(0xBC00, 0); // Select R0
        crtc.write(0xBD00, 42);
        assert_eq!(crtc.register(0), 42);
    }

    #[test]
    fn address_low_byte_dont_care_for_select() {
        let mut crtc = Crtc::new();
        crtc.write(0xBCFF, 5);
        assert_eq!(crtc.selected_register(), 5);
        crtc.write(0xBC01, 9);
        assert_eq!(crtc.selected_register(), 9);
        crtc.write(0xBCAA, 3);
        assert_eq!(crtc.selected_register(), 3);
    }

    #[test]
    fn address_low_byte_dont_care_for_data_write() {
        let mut crtc = Crtc::new();
        crtc.write(0xBC00, 0); // Select R0
        crtc.write(0xBDFF, 77);
        assert_eq!(crtc.register(0), 77);
        crtc.write(0xBD55, 88);
        assert_eq!(crtc.register(0), 88);
    }

    #[test]
    fn address_low_byte_dont_care_for_data_read() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type0);
        write_register(&mut crtc, 12, 0x30);
        crtc.write(0xBC00, 12);
        assert_eq!(crtc.read(0xBFFF), 0x30);
        assert_eq!(crtc.read(0xBF00), 0x30);
        assert_eq!(crtc.read(0xBFAA), 0x30);
    }

    #[test]
    fn write_to_bcxx_then_bdxx_sequence() {
        let mut crtc = Crtc::new();
        // Select R3, write 0x8E
        crtc.write(0xBC00, 3);
        crtc.write(0xBD00, 0x8E);
        assert_eq!(crtc.register(3), 0x8E);
        // Select R9, write 7
        crtc.write(0xBC00, 9);
        crtc.write(0xBD00, 7);
        assert_eq!(crtc.register(9), 7);
        // R3 should be unchanged
        assert_eq!(crtc.register(3), 0x8E);
    }

    #[test]
    fn select_register_0_to_31_type0() {
        let mut crtc = Crtc::new();
        for reg in 0..=31u8 {
            crtc.write(0xBC00, reg);
            assert_eq!(crtc.selected_register(), reg);
        }
    }

    #[test]
    fn select_register_5_bit_truncation_type0() {
        let mut crtc = Crtc::new();
        // 32 = 0x20 → truncated to 0
        crtc.write(0xBC00, 32);
        assert_eq!(crtc.selected_register(), 0);
        // 33 = 0x21 → truncated to 1
        crtc.write(0xBC00, 33);
        assert_eq!(crtc.selected_register(), 1);
        // 63 = 0x3F → truncated to 31
        crtc.write(0xBC00, 63);
        assert_eq!(crtc.selected_register(), 31);
    }

    #[test]
    fn select_register_5_bit_truncation_type1() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        crtc.write(0xBC00, 32);
        assert_eq!(crtc.selected_register(), 0);
        crtc.write(0xBC00, 63);
        assert_eq!(crtc.selected_register(), 31);
    }

    #[test]
    fn select_register_5_bit_truncation_type2() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type2);
        crtc.write(0xBC00, 63);
        assert_eq!(crtc.selected_register(), 31);
    }

    #[test]
    fn select_register_3_bit_truncation_type3() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        // 12 = 0b1100 → truncated to 0b100 = 4
        crtc.write(0xBC00, 12);
        assert_eq!(crtc.selected_register(), 4);
        // 20 = 0b10100 → truncated to 0b100 = 4
        crtc.write(0xBC00, 20);
        assert_eq!(crtc.selected_register(), 4);
        // 15 = 0b1111 → truncated to 0b111 = 7
        crtc.write(0xBC00, 15);
        assert_eq!(crtc.selected_register(), 7);
    }

    #[test]
    fn select_register_3_bit_truncation_type4() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type4);
        crtc.write(0xBC00, 15);
        assert_eq!(crtc.selected_register(), 7);
        crtc.write(0xBC00, 12);
        assert_eq!(crtc.selected_register(), 4);
    }

    #[test]
    fn write_r0_horizontal_total() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 63);
        assert_eq!(crtc.register(0), 63);
    }

    #[test]
    fn write_r1_horizontal_displayed() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 1, 40);
        assert_eq!(crtc.register(1), 40);
    }

    #[test]
    fn write_r2_horizontal_sync_position() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 2, 46);
        assert_eq!(crtc.register(2), 46);
    }

    #[test]
    fn write_r3_sync_widths() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 3, 0x8E);
        assert_eq!(crtc.register(3), 0x8E);
    }

    #[test]
    fn write_r4_vertical_total() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 4, 38);
        assert_eq!(crtc.register(4), 38);
    }

    #[test]
    fn write_r5_vertical_total_adjust() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 5, 6);
        assert_eq!(crtc.register(5), 6);
    }

    #[test]
    fn write_r6_vertical_displayed() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 6, 25);
        assert_eq!(crtc.register(6), 25);
    }

    #[test]
    fn write_r7_vertical_sync_position() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 7, 30);
        assert_eq!(crtc.register(7), 30);
    }

    #[test]
    fn write_r8_interlace_and_skew() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 8, 0x00);
        assert_eq!(crtc.register(8), 0x00);
    }

    #[test]
    fn write_r9_max_raster_address() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 9, 7);
        assert_eq!(crtc.register(9), 7);
    }

    #[test]
    fn write_r10_cursor_start_raster() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 10, 0x60); // Blink 1/16, raster 0
        assert_eq!(crtc.register(10), 0x60);
    }

    #[test]
    fn write_r11_cursor_end_raster() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 11, 7);
        assert_eq!(crtc.register(11), 7);
    }

    #[test]
    fn write_r12_r13_start_address() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x12);
        assert_eq!(crtc.register(12), 0x30);
        assert_eq!(crtc.register(13), 0x12);
    }

    #[test]
    fn write_r14_r15_cursor_position() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 14, 0x0A);
        write_register(&mut crtc, 15, 0xBC);
        assert_eq!(crtc.register(14), 0x0A);
        assert_eq!(crtc.register(15), 0xBC);
    }

    #[test]
    fn write_to_r16_r17_ignored() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 16, 0xFF);
        write_register(&mut crtc, 17, 0xFF);
        // R16/R17 are read-only (light pen registers); writes are ignored.
        assert_eq!(crtc.register(16), 0);
        assert_eq!(crtc.register(17), 0);
    }

    #[test]
    fn register_write_persists_across_select_changes() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 42);
        write_register(&mut crtc, 1, 99);
        // Select a different register and back
        crtc.write(0xBC00, 5);
        crtc.write(0xBC00, 0);
        assert_eq!(crtc.register(0), 42);
        assert_eq!(crtc.register(1), 99);
    }

    #[test]
    fn data_write_only_affects_selected_register() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 42);
        crtc.write(0xBC00, 1);
        crtc.write(0xBD00, 99);
        assert_eq!(crtc.register(0), 42);
        assert_eq!(crtc.register(1), 99);
    }

    #[test]
    fn type0_read_r12_r13_readable() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        assert_eq!(read_register(&mut crtc, 12), 0x30);
        assert_eq!(read_register(&mut crtc, 13), 0x00);
    }

    #[test]
    fn type0_read_r14_r15_readable() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 14, 0x0A);
        write_register(&mut crtc, 15, 0xBC);
        assert_eq!(read_register(&mut crtc, 14), 0x0A);
        assert_eq!(read_register(&mut crtc, 15), 0xBC);
    }

    #[test]
    fn type0_read_r0_returns_zero() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 63);
        assert_eq!(read_register(&mut crtc, 0), 0); // R0 is write-only on Type 0
    }

    #[test]
    fn type0_read_r1_through_r11_returns_zero() {
        let mut crtc = Crtc::new();
        for reg in 1..=11u8 {
            write_register(&mut crtc, reg, 0xAA);
            assert_eq!(
                read_register(&mut crtc, reg),
                0,
                "R{} should be write-only on Type 0",
                reg
            );
        }
    }

    #[test]
    fn type1_read_r14_r15_readable() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 14, 0x0A);
        write_register(&mut crtc, 15, 0xBC);
        assert_eq!(read_register(&mut crtc, 14), 0x0A);
        assert_eq!(read_register(&mut crtc, 15), 0xBC);
    }

    #[test]
    fn type1_read_r12_r13_returns_zero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        assert_eq!(read_register(&mut crtc, 12), 0);
        assert_eq!(read_register(&mut crtc, 13), 0);
    }

    #[test]
    fn type1_read_r31_returns_nonzero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        let val = read_register(&mut crtc, 31);
        assert_ne!(val, 0, "R31 should return non-zero on Type 1");
    }

    #[test]
    fn type1_read_r0_returns_zero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 63);
        assert_eq!(read_register(&mut crtc, 0), 0);
    }

    #[test]
    fn type2_read_r14_r15_returns_zero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type2);
        write_register(&mut crtc, 14, 0x0A);
        write_register(&mut crtc, 15, 0xBC);
        assert_eq!(read_register(&mut crtc, 14), 0);
        assert_eq!(read_register(&mut crtc, 15), 0);
    }

    #[test]
    fn type2_read_r12_r13_returns_zero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type2);
        write_register(&mut crtc, 12, 0x30);
        assert_eq!(read_register(&mut crtc, 12), 0);
    }

    #[test]
    fn type3_read_r12_r13_readable() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        assert_eq!(read_register(&mut crtc, 12), 0x30);
        assert_eq!(read_register(&mut crtc, 13), 0x00);
    }

    #[test]
    fn type3_read_r14_r15_readable() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 14, 0x0A);
        write_register(&mut crtc, 15, 0xBC);
        assert_eq!(read_register(&mut crtc, 14), 0x0A);
        assert_eq!(read_register(&mut crtc, 15), 0xBC);
    }

    #[test]
    fn type3_read_r0_is_write_only_aliases_r16() {
        // Type 3/4: reading AR0 doesn't return R0, it returns R16 (low 3 bits = 0)
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 0, 63); // write to R0 (has effect internally)

        let read_r0 = read_register(&mut crtc, 0);
        let read_r16 = read_register(&mut crtc, 16);

        assert_ne!(read_r0, 63, "R0 is write-only on Type3");
        assert_eq!(read_r0, read_r16, "AR0 aliases to R16 on Type3");
        assert_eq!(crtc.registers[0], 63);
    }

    #[test]
    fn type3_read_r9_is_write_only_aliases_r17() {
        // Type 3/4: AR9 & 7 = 1, so read returns R17, not R9
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 9, 7);

        let read_r9 = read_register(&mut crtc, 9);
        let read_r17 = read_register(&mut crtc, 17);

        assert_ne!(read_r9, 7, "R9 is write-only on Type3");
        assert_eq!(read_r9, read_r17, "AR9 aliases to R17 on Type3");
        assert_eq!(crtc.registers[9], 7);
    }

    #[test]
    fn type3_read_r4_yields_r12_value() {
        // Per docs: "Truncates the register selection number to 3 bits
        // (e.g., reading R4 or R20 yields the value of R12)."
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 12, 0x30);
        crtc.write(0xBC00, 4); // Select AR=4 (truncates to 3 bits → reads R12)
        let val = crtc.read(0xBF00);
        assert_eq!(val, 0x30, "Reading AR=4 on Type 3 should yield R12's value");
    }

    #[test]
    fn type1_status_register_initial_state() {
        let crtc = Crtc::new_with_type(CrtcType::Type1);
        let status = crtc.read(0xBE00);
        // Bit 6 (LPEN full) = 0, Bit 5 (St_R6) = 0, all others 0
        assert_eq!(
            status & 0x60,
            0,
            "Status bits 6 and 5 should be 0 initially"
        );
    }

    #[test]
    fn type1_status_lpen_full_set_after_strobe() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        // Pulse LPSTB (rising edge)
        crtc.lpstb(true);
        let status = crtc.read(0xBE00);
        assert_eq!(
            status, 0x40,
            "Bit 6 (LPEN full) should be set after light pen strobe"
        );
    }

    #[test]
    fn type1_status_lpen_full_resets_on_r16_read() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        // Pulse LPSTB (rising edge)
        crtc.lpstb(true);
        let status = crtc.read(0xBE00);
        assert_eq!(
            status, 0x40,
            "Bit 6 (LPEN full) should be set after light pen strobe"
        );
        read_register(&mut crtc, 16);
        let status = crtc.read(0xBE00);
        assert_eq!(
            status, 0x00,
            "Bit 6 (LPEN full) should be reset on R16 read"
        );
    }

    #[test]
    fn type1_status_lpen_full_resets_on_r17_read() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        // Pulse LPSTB (rising edge)
        crtc.lpstb(true);
        let status = crtc.read(0xBE00);
        assert_eq!(
            status, 0x40,
            "Bit 6 (LPEN full) should be set after light pen strobe"
        );
        read_register(&mut crtc, 17);
        let status = crtc.read(0xBE00);
        assert_eq!(
            status, 0x00,
            "Bit 6 (LPEN full) should be reset on R17 read"
        );
    }

    #[test]
    fn type0_read_bexx_returns_floating_bus() {
        let crtc = Crtc::new();
        let val = crtc.read(0xBE00);
        // Type 0: typically returns 127 or 255 (floating bus)
        assert!(
            val == 127 || val == 255,
            "Type 0 &BE00 should return 127 or 255, got {}",
            val
        );
    }

    #[test]
    fn type2_read_bexx_returns_ff() {
        let crtc = Crtc::new_with_type(CrtcType::Type2);
        let val = crtc.read(0xBE00);
        assert_eq!(val, 0xFF, "Type 2 &BE00 should return 0xFF");
    }

    #[test]
    fn hsync_inactive_at_c0_zero() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc);
        // C0=0, R2=6 → HSYNC should be inactive
        assert!(!crtc.hsync());
    }

    #[test]
    fn hsync_activates_at_r2() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc);
        // Tick 6 times: C0 = 6 = R2 → HSYNC on
        for _ in 0..6 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 6);
        assert!(crtc.hsync(), "HSYNC should be active at C0=R2=6");
    }

    #[test]
    fn hsync_deactivates_after_width() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc); // R2=6, HSYNC width=2
        for _ in 0..6 {
            crtc.tick();
        }
        assert!(crtc.hsync()); // C0=6
        crtc.tick();
        assert!(crtc.hsync()); // C0=7 (still within width)
        crtc.tick();
        assert!(!crtc.hsync()); // C0=8 (width expired)
    }

    #[test]
    fn hsync_width_zero_disables_on_type0() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 2, 6);
        write_register(&mut crtc, 3, 0x00); // HSYNC width = 0
        for _ in 0..6 {
            crtc.tick();
        }
        assert!(
            !crtc.hsync(),
            "HSYNC width 0 should disable HSYNC on Type 0"
        );
    }

    #[test]
    fn hsync_width_zero_disables_on_type1() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 2, 6);
        write_register(&mut crtc, 3, 0x00);
        for _ in 0..6 {
            crtc.tick();
        }
        assert!(
            !crtc.hsync(),
            "HSYNC width 0 should disable HSYNC on Type 1"
        );
    }

    #[test]
    fn hsync_width_zero_is_16_on_type2() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type2);
        write_register(&mut crtc, 0, 20);
        write_register(&mut crtc, 2, 2);
        write_register(&mut crtc, 3, 0x00); // HSYNC width = 0 → 16 on Type 2
        for _ in 0..2 {
            crtc.tick();
        }
        assert!(crtc.hsync(), "HSYNC should be active at C0=R2=2");
        // Should stay active for 16 CCLKs
        for _ in 0..15 {
            crtc.tick();
            assert!(
                crtc.hsync(),
                "HSYNC should still be active within 16 CCLK width"
            );
        }
        crtc.tick();
        assert!(!crtc.hsync(), "HSYNC should be off after 16 CCLKs");
    }

    #[test]
    fn hsync_pulses_every_line() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc); // R0=10, 11 CCLKs per line
        // Complete one full line (11 ticks: C0 0→10→wrap to 0)
        for _ in 0..11 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 0);
        assert!(!crtc.hsync());
        // Tick to R2 again
        for _ in 0..6 {
            crtc.tick();
        }
        assert!(crtc.hsync(), "HSYNC should pulse again on the second line");
    }

    #[test]
    fn vsync_inactive_initially() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        // C4=0, R7=1 → VSYNC not yet at R7
        assert!(!crtc.vsync());
    }

    #[test]
    fn vsync_activates_at_r7() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        // R7=1. VSYNC at C4=1, C9=0, C0=0 = after 10 ticks
        for _ in 0..10 {
            crtc.tick();
        }
        assert_eq!(crtc.c4(), 1);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c0(), 0);
        assert!(crtc.vsync(), "VSYNC should be active at C4=R7=1");
    }

    #[test]
    fn vsync_deactivates_after_width_type0() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R3 upper nibble = 1 → VSYNC width = 1 scanline
        for _ in 0..10 {
            crtc.tick();
        }
        assert!(crtc.vsync()); // C4=1, C9=0
        // Tick through the rest of this scanline (5 CCLKs)
        for _ in 0..5 {
            crtc.tick();
        }
        // C4=1, C9=1 → VSYNC should be off after 1 scanline
        assert!(!crtc.vsync(), "VSYNC should be off after 1 scanline width");
    }

    #[test]
    fn vsync_fixed_16_lines_type1() {
        // On Type 1, VSYNC width is fixed at 16 scanlines regardless of R3 upper nibble.
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 2, 3);
        write_register(&mut crtc, 3, 0x14); // VSYNC width = 1 (ignored on Type 1)
        write_register(&mut crtc, 4, 20); // 21 rows → enough room
        write_register(&mut crtc, 5, 0);
        write_register(&mut crtc, 6, 10);
        write_register(&mut crtc, 7, 2);
        write_register(&mut crtc, 8, 0);
        write_register(&mut crtc, 9, 1); // 2 scanlines per row
        // VSYNC at C4=2 = tick 20 (2 rows × 2 scanlines × 5 CCLKs)
        for _ in 0..20 {
            crtc.tick();
        }
        assert!(crtc.vsync());
        // After 1 scanline (5 CCLKs), VSYNC should still be active (fixed 16)
        for _ in 0..5 {
            crtc.tick();
        }
        assert!(
            crtc.vsync(),
            "Type 1 VSYNC should still be active after 1 scanline (fixed 16)"
        );
        // After 15 scanlines total
        for _ in 0..14 * 5 {
            crtc.tick();
        }
        assert!(
            crtc.vsync(),
            "Type 1 VSYNC should still be active after 15 scanlines"
        );
        // After 16 scanlines total
        for _ in 0..5 {
            crtc.tick();
        }
        assert!(
            !crtc.vsync(),
            "Type 1 VSYNC should be off after 16 scanlines"
        );
    }

    #[test]
    fn vsync_fixed_16_lines_type2() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type2);
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 2, 3);
        write_register(&mut crtc, 3, 0x14); // VSYNC width = 1 (ignored on Type 2)
        write_register(&mut crtc, 4, 20);
        write_register(&mut crtc, 5, 0);
        write_register(&mut crtc, 6, 10);
        write_register(&mut crtc, 7, 2);
        write_register(&mut crtc, 8, 0);
        write_register(&mut crtc, 9, 1);
        for _ in 0..20 {
            crtc.tick();
        }
        assert!(crtc.vsync());
        // Should be fixed at 16 scanlines on Type 2
        for _ in 0..15 * 5 {
            crtc.tick();
        }
        assert!(
            crtc.vsync(),
            "Type 2 VSYNC should still be active after 15 scanlines"
        );
        for _ in 0..5 {
            crtc.tick();
        }
        assert!(
            !crtc.vsync(),
            "Type 2 VSYNC should be off after 16 scanlines"
        );
    }

    #[test]
    fn vsync_width_zero_is_16_type0() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 2, 3);
        write_register(&mut crtc, 3, 0x04); // VSYNC width = 0 → 16 on Type 0
        write_register(&mut crtc, 4, 20);
        write_register(&mut crtc, 5, 0);
        write_register(&mut crtc, 6, 10);
        write_register(&mut crtc, 7, 2);
        write_register(&mut crtc, 8, 0);
        write_register(&mut crtc, 9, 1);
        for _ in 0..20 {
            crtc.tick();
        }
        assert!(crtc.vsync());
        for _ in 0..15 * 5 {
            crtc.tick();
        }
        assert!(crtc.vsync(), "Type 0 VSYNC width 0 should be 16 scanlines");
        for _ in 0..5 {
            crtc.tick();
        }
        assert!(!crtc.vsync());
    }

    #[test]
    fn dispen_active_at_start_of_displayed_area() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc); // R1=5, R6=1, R9=0
        // C0=0, C4=0, C9=0 → within horizontal and vertical display
        assert!(crtc.dispen());
    }

    #[test]
    fn dispen_inactive_after_r1() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc); // R1=5
        for _ in 0..5 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 5);
        assert!(!crtc.dispen(), "DISPEN should be off at C0=R1");
    }

    #[test]
    fn dispen_still_active_before_r1() {
        let mut crtc = Crtc::new();
        setup_hsync_frame(&mut crtc); // R1=5
        for _ in 0..4 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 4);
        assert!(crtc.dispen(), "DISPEN should still be on at C0=4 (R1=5)");
    }

    #[test]
    fn dispen_inactive_after_r6() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R6=1 → display only C4=0
        // C4=1 → vertical display ended
        for _ in 0..10 {
            crtc.tick();
        }
        assert_eq!(crtc.c4(), 1);
        assert!(!crtc.dispen(), "DISPEN should be off when C4 >= R6");
    }

    #[test]
    fn dispen_active_only_during_displayed_rows() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R6=1 → only C4=0 is displayed
        // C4=0 → DISPEN active (within horizontal display)
        assert!(crtc.dispen());
        // C4=2 → should be off
        for _ in 0..20 {
            crtc.tick();
        }
        assert_eq!(crtc.c4(), 2);
        assert!(!crtc.dispen());
    }

    #[test]
    fn r8_border_force_on_disables_display() {
        let mut crtc = Crtc::new(); // Type 0 supports skew
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 8, 0x30); // Bits 5:4 = 11 → Border Force ON
        write_register(&mut crtc, 9, 0);
        write_register(&mut crtc, 4, 0);
        write_register(&mut crtc, 6, 1);
        assert!(
            !crtc.dispen(),
            "R8 bits 5:4 = 11 should force border on (DISPEN off)"
        );
    }

    #[test]
    fn ma_starts_at_r12_r13() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 0);
        assert_eq!(crtc.current_ma(), 0x3000);
    }

    #[test]
    fn ma_increments_each_tick() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 0);
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);
        let initial_ma = crtc.current_ma();
        crtc.tick();
        assert_eq!(crtc.current_ma(), initial_ma + 1);
        crtc.tick();
        assert_eq!(crtc.current_ma(), initial_ma + 2);
    }

    #[test]
    fn ma_resets_at_frame_start() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        write_register(&mut crtc, 12, 0x10);
        write_register(&mut crtc, 13, 0x00);
        // Run one full frame (30 CCLKs)
        for _ in 0..30 {
            crtc.tick();
        }
        // Should be back at start address
        assert_eq!(crtc.current_ma(), 0x1000);
    }

    #[test]
    fn vma_loaded_from_r12_r13_at_frame_start() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        write_register(&mut crtc, 12, 0x20);
        write_register(&mut crtc, 13, 0x00);
        // VMA should be loaded with R12/R13 at frame start
        assert_eq!(crtc.vma(), 0x2000);
    }

    #[test]
    fn cursor_activates_at_cursor_position() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 7);
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);
        write_register(&mut crtc, 14, 0); // Cursor at MA = 0
        write_register(&mut crtc, 15, 0);
        write_register(&mut crtc, 10, 0); // Start raster 0, steady
        write_register(&mut crtc, 11, 7); // End raster 7
        assert!(
            crtc.cursor(),
            "Cursor should be active when MA matches R14/R15"
        );
    }

    #[test]
    fn cursor_not_active_when_ma_does_not_match() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 7);
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);
        write_register(&mut crtc, 14, 0);
        write_register(&mut crtc, 15, 3); // Cursor at MA = 3
        write_register(&mut crtc, 10, 0);
        write_register(&mut crtc, 11, 7);
        assert!(
            !crtc.cursor(),
            "Cursor should not be active when MA != cursor pos"
        );
    }

    #[test]
    fn cursor_hidden_mode() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 7);
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);
        write_register(&mut crtc, 14, 0);
        write_register(&mut crtc, 15, 0);
        write_register(&mut crtc, 10, 0x20); // Bits 6:5 = 01 → Hidden
        write_register(&mut crtc, 11, 7);
        assert!(
            !crtc.cursor(),
            "Cursor should be hidden when R10 bits 6:5 = 01"
        );
    }

    #[test]
    fn cursor_respects_raster_bounds() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 7); // 8 scanlines per row
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);
        write_register(&mut crtc, 14, 0);
        write_register(&mut crtc, 15, 0);
        write_register(&mut crtc, 10, 4); // Cursor start raster = 4
        write_register(&mut crtc, 11, 6); // Cursor end raster = 6
        // C9 = 0 → not within [4, 6] → cursor off
        assert!(
            !crtc.cursor(),
            "Cursor should be off at C9=0 when range is [4,6]"
        );
        // Tick to C9=4 (need to complete 4 full lines)
        for _ in 0..4 * 11 {
            crtc.tick();
        }
        assert_eq!(crtc.c9(), 4);
        assert!(crtc.cursor(), "Cursor should be on at C9=4 (within [4,6])");
    }

    #[test]
    fn lpstb_captures_ma_into_r16_r17() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 1, 5);
        write_register(&mut crtc, 9, 0);
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        assert_eq!(crtc.current_ma(), 0x0ABC);
        // Rising edge of LPSTB captures MA
        crtc.lpstb(true);
        assert_eq!(crtc.register(16), 0x0A, "R16 should capture MA high byte");
        assert_eq!(crtc.register(17), 0xBC, "R17 should capture MA low byte");
    }

    #[test]
    fn lpstb_rising_edge_only() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 10);
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        // First rising edge captures MA
        crtc.lpstb(true);
        assert_eq!(crtc.register(16), 0x0A);
        // Tick to advance MA
        crtc.tick();
        // LPSTB still high — no new rising edge → no new capture
        crtc.lpstb(true);
        assert_eq!(
            crtc.register(17),
            0xBC,
            "No new capture without rising edge"
        );
        // Go low, then high for a new capture
        crtc.lpstb(false);
        crtc.lpstb(true);
        // MA was 0x0ABC + 1 = 0x0ABD
        assert_eq!(
            crtc.register(17),
            0xBD,
            "New rising edge should capture updated MA"
        );
    }

    #[test]
    fn lpstb_does_not_capture_on_falling_edge() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x0A);
        write_register(&mut crtc, 13, 0xBC);
        // Set high first (rising edge captures)
        crtc.lpstb(true);
        assert_eq!(crtc.register(16), 0x0A);
        // Tick to change MA
        crtc.tick();
        // Falling edge should NOT capture
        crtc.lpstb(false);
        assert_eq!(crtc.register(17), 0xBC, "Falling edge should not capture");
    }

    #[test]
    fn c9_resets_at_r9() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R9=1 → C9 counts 0, 1, then resets
        assert_eq!(crtc.c9(), 0);
        // One full line (5 CCLKs): C0 wraps, C9 increments
        for _ in 0..5 {
            crtc.tick();
        }
        assert_eq!(crtc.c9(), 1);
        // Another full line: C9 wraps to 0, C4 increments
        for _ in 0..5 {
            crtc.tick();
        }
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c4(), 1);
    }

    #[test]
    fn c9_counts_within_char_row() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 9, 3); // 4 scanlines per row
        // Tick through 4 scanlines
        for scanline in 0..4u8 {
            assert_eq!(crtc.c9(), scanline);
            for _ in 0..5 {
                crtc.tick();
            }
        }
        assert_eq!(crtc.c9(), 0, "C9 should wrap to 0 after R9+1 scanlines");
    }

    #[test]
    fn type0_r0_zero_freezes_c9() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 0); // R0 = 0
        write_register(&mut crtc, 9, 7);
        // With R0 = 0 on Type 0, C9 should not increment
        for _ in 0..100 {
            crtc.tick();
        }
        assert_eq!(crtc.c9(), 0, "Type 0 with R0=0 should freeze C9");
    }

    #[test]
    fn type1_r0_zero_does_not_freeze_c9() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 0, 0);
        write_register(&mut crtc, 9, 7);
        for _ in 0..100 {
            crtc.tick();
        }
        assert_ne!(crtc.c9(), 0, "Type 1 with R0=0 should not freeze C9");
    }

    #[test]
    fn reset_clears_registers() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 63);
        write_register(&mut crtc, 1, 40);
        write_register(&mut crtc, 9, 7);
        write_register(&mut crtc, 12, 0x30);
        crtc.reset();
        assert_eq!(crtc.register(0), 0);
        assert_eq!(crtc.register(1), 0);
        assert_eq!(crtc.register(9), 0);
        assert_eq!(crtc.register(12), 0);
    }

    #[test]
    fn reset_clears_counters() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        for _ in 0..15 {
            crtc.tick();
        }
        crtc.reset();
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c4(), 0);
        assert_eq!(crtc.c9(), 0);
    }

    #[test]
    fn reset_clears_outputs() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        for _ in 0..15 {
            crtc.tick();
        }
        crtc.reset();
        assert!(!crtc.hsync());
        assert!(!crtc.vsync());
        assert!(!crtc.dispen());
        assert!(!crtc.cursor());
    }

    #[test]
    fn full_frame_returns_to_start() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        // 3 rows × 2 scanlines × 5 CCLKs = 30 CCLKs per frame
        for _ in 0..30 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c4(), 0);
    }

    #[test]
    fn full_frame_vsync_occurs_once() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        let mut vsync_rising_edges = 0;
        let mut was_vsync = false;
        for _ in 0..30 {
            crtc.tick();
            let is_vsync = crtc.vsync();
            if is_vsync && !was_vsync {
                vsync_rising_edges += 1;
            }
            was_vsync = is_vsync;
        }
        assert_eq!(
            vsync_rising_edges, 1,
            "VSYNC should rise exactly once per frame"
        );
    }

    #[test]
    fn full_frame_hsyc_count_matches_lines() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        // 6 scanlines → 6 HSYNC pulses
        let mut hsync_rising_edges = 0;
        let mut was_hsync = false;
        for _ in 0..30 {
            crtc.tick();
            let is_hsync = crtc.hsync();
            if is_hsync && !was_hsync {
                hsync_rising_edges += 1;
            }
            was_hsync = is_hsync;
        }
        assert_eq!(
            hsync_rising_edges, 6,
            "Should see 6 HSYNC pulses in 6 scanlines"
        );
    }

    #[test]
    fn multiple_frames_stable() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc);
        // Run 10 full frames
        for _ in 0..10 * 30 {
            crtc.tick();
        }
        // Should still be at frame start
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c4(), 0);
        assert_eq!(crtc.c9(), 0);
    }

    #[test]
    fn vertical_adjust_adds_extra_scanlines() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 2, 3);
        write_register(&mut crtc, 3, 0x14);
        write_register(&mut crtc, 4, 1); // R4=1 → 2 char rows
        write_register(&mut crtc, 5, 2); // R5=2 → 2 extra scanlines
        write_register(&mut crtc, 6, 1);
        write_register(&mut crtc, 7, 0);
        write_register(&mut crtc, 8, 0);
        write_register(&mut crtc, 9, 1); // 2 scanlines per row
        // Total scanlines = (R4+1) * (R9+1) + R5 = 2 * 2 + 2 = 6
        // Total CCLKs = 6 * 5 = 30
        for _ in 0..30 {
            crtc.tick();
        }
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c4(), 0);
    }

    #[test]
    fn vertical_adjust_zero_no_extra_scanlines() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R5=0
        // Total = 3 * 2 + 0 = 6 scanlines = 30 CCLKs
        for _ in 0..30 {
            crtc.tick();
        }
        assert_eq!(
            crtc.c4(),
            0,
            "Frame should complete exactly at 30 CCLKs with R5=0"
        );
    }

    #[test]
    fn type3_vsync_triggers_at_c9_c0_zero() {
        // On Type 3/4, VSYNC is only triggered if C4=R7 AND C9=0 AND C0=0.
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        setup_small_frame(&mut crtc);
        // VSYNC at C4=1, C9=0, C0=0 = tick 10
        for _ in 0..10 {
            crtc.tick();
        }
        assert_eq!(crtc.c4(), 1);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c0(), 0);
        assert!(
            crtc.vsync(),
            "Type 3 VSYNC should trigger at C4=R7, C9=0, C0=0"
        );
    }

    #[test]
    fn raster_address_matches_c9() {
        let mut crtc = Crtc::new();
        setup_small_frame(&mut crtc); // R9=1
        assert_eq!(crtc.current_raster(), 0);
        for _ in 0..5 {
            crtc.tick();
        }
        assert_eq!(crtc.current_raster(), 1);
    }

    #[test]
    fn typical_cpc464_initialization_sequence() {
        let mut crtc = Crtc::new();
        // Standard CPC 464 register values
        write_register(&mut crtc, 0, 63); // H Total = 63 (64 chars)
        write_register(&mut crtc, 1, 40); // H Displayed = 40
        write_register(&mut crtc, 2, 46); // H Sync Pos = 46
        write_register(&mut crtc, 3, 0x8E); // VSYNC 8, HSYNC 14
        write_register(&mut crtc, 4, 38); // V Total = 38 (39 rows)
        write_register(&mut crtc, 5, 0); // V Adjust = 0
        write_register(&mut crtc, 6, 25); // V Displayed = 25
        write_register(&mut crtc, 7, 30); // V Sync Pos = 30
        write_register(&mut crtc, 8, 0); // Non-interlace
        write_register(&mut crtc, 9, 7); // Max Raster = 7 (8 scanlines)
        write_register(&mut crtc, 12, 0x30); // Start addr high
        write_register(&mut crtc, 13, 0x00); // Start addr low

        assert_eq!(crtc.register(0), 63);
        assert_eq!(crtc.register(1), 40);
        assert_eq!(crtc.register(3), 0x8E);
        assert_eq!(crtc.register(9), 7);
        assert_eq!(crtc.current_ma(), 0x3000);
        assert!(crtc.dispen(), "Display should be enabled at frame start");
    }

    #[test]
    fn cpc464_frame_has_312_scanlines() {
        let mut crtc = Crtc::new();
        // Standard CPC 464 values
        write_register(&mut crtc, 0, 63); // 64 CCLKs per line
        write_register(&mut crtc, 1, 40);
        write_register(&mut crtc, 2, 46);
        write_register(&mut crtc, 3, 0x8E);
        write_register(&mut crtc, 4, 38); // 39 char rows
        write_register(&mut crtc, 5, 0);
        write_register(&mut crtc, 6, 25);
        write_register(&mut crtc, 7, 30);
        write_register(&mut crtc, 8, 0);
        write_register(&mut crtc, 9, 7); // 8 scanlines per row
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);

        // Total scanlines = 39 * 8 + 0 = 312
        // Total CCLKs = 312 * 64 = 19968
        let mut hsync_count = 0;
        let mut was_hsync = false;
        for _ in 0..19968 {
            crtc.tick();
            let is_hsync = crtc.hsync();
            if is_hsync && !was_hsync {
                hsync_count += 1;
            }
            was_hsync = is_hsync;
        }
        assert_eq!(hsync_count, 312, "CPC 464 frame should have 312 scanlines");
        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c4(), 0);
        assert_eq!(crtc.c9(), 0);
    }

    #[test]
    fn repeated_register_writes_idempotent() {
        let mut crtc = Crtc::new();
        for _ in 0..10 {
            write_register(&mut crtc, 0, 63);
        }
        assert_eq!(crtc.register(0), 63);
    }

    #[test]
    fn repeated_select_writes_dont_change_data() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 42);
        for _ in 0..10 {
            crtc.write(0xBC00, 0);
        }
        assert_eq!(crtc.register(0), 42);
    }

    #[test]
    fn type3_read_ar2_aliases_to_r10() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 10, 0x55); // Write to R10 (Cursor Start)
        crtc.write(0xBC00, 2); // Select AR=2
        let val = crtc.read(0xBF00);
        assert_eq!(val, 0x55, "AR2 on Type 3 should alias to R10");
    }

    #[test]
    fn type3_read_ar3_aliases_to_r11() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type3);
        write_register(&mut crtc, 11, 0x66); // Write to R11 (Cursor End)
        crtc.write(0xBC00, 3); // Select AR=3
        let val = crtc.read(0xBF00);
        assert_eq!(val, 0x66, "AR3 on Type 3 should alias to R11");
    }

    #[test]
    fn type0_r0_zero_freezes_c0() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 0);
        write_register(&mut crtc, 9, 7);

        for _ in 0..5 {
            crtc.tick();
        }

        assert_eq!(crtc.c0(), 0, "Type 0 with R0=0 should freeze C0 at 0");
        assert_eq!(crtc.c9(), 0, "Type 0 with R0=0 should freeze C9");
    }

    #[test]
    fn type1_read_r13_returns_zero() {
        let mut crtc = Crtc::new_with_type(CrtcType::Type1);
        write_register(&mut crtc, 13, 0xFF);
        assert_eq!(
            read_register(&mut crtc, 13),
            0,
            "R13 should be write-only on Type 1"
        );
    }

    #[test]
    fn type0_vertical_adjust_uses_c9_not_c5() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 4, 0); // V Total = 1 row
        write_register(&mut crtc, 5, 2); // V Adjust = 2 lines
        write_register(&mut crtc, 9, 0); // 1 scanline per row

        // Total frame should be 1 row (1 line) + 2 adjust lines = 3 lines.
        // 3 lines * 5 CCLKs = 15 CCLKs.
        for _ in 0..15 {
            crtc.tick();
        }

        assert_eq!(crtc.c0(), 0);
        assert_eq!(crtc.c4(), 0);
        assert_eq!(crtc.c9(), 0);
        assert_eq!(crtc.c5(), 0, "Type 0 should not use C5 counter");
    }

    #[test]
    fn address_decode_default_page_c000() {
        // R12=0x30, R13=0x00 → MA starts at 0x3000 → phys 0xC000
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        assert_eq!(crtc.phys_address(), 0xC000);
    }

    #[test]
    fn address_decode_scanline_offset_0x0800() {
        let mut crtc = Crtc::new();
        write_register(&mut crtc, 12, 0x30);
        write_register(&mut crtc, 13, 0x00);
        assert_eq!(crtc.phys_address(), 0xC000);
        crtc.c9 = 1;
        assert_eq!(crtc.phys_address(), 0xC800);
        crtc.c9 = 2;
        assert_eq!(crtc.phys_address(), 0xD000);
        crtc.c9 = 3;
        assert_eq!(crtc.phys_address(), 0xD800);
        crtc.c9 = 4;
        assert_eq!(crtc.phys_address(), 0xE000);
        crtc.c9 = 5;
        assert_eq!(crtc.phys_address(), 0xE800);
        crtc.c9 = 6;
        assert_eq!(crtc.phys_address(), 0xF000);
        crtc.c9 = 7;
        assert_eq!(crtc.phys_address(), 0xF800);
    }

    #[test]
    fn phys_address_for_arbitrary_ma_and_raster() {
        // Standard CPC layout: MA=0x3000, raster=0 → phys 0xC000
        assert_eq!(Crtc::phys_address_for(0x3000, 0), 0xC000);
        // Same MA, raster=1 → next scanline block (0x0800 step)
        assert_eq!(Crtc::phys_address_for(0x3000, 1), 0xC800);
        assert_eq!(Crtc::phys_address_for(0x3000, 7), 0xF800);
        // MA=0x3001 → next byte pair (A1 toggles)
        assert_eq!(Crtc::phys_address_for(0x3001, 0), 0xC002);
        // Char row 1 start: MA = 0x3000 + 40 = 0x3028
        assert_eq!(Crtc::phys_address_for(0x3028, 0), 0xC050);
    }

    #[test]
    fn vma_prime_only_updates_at_end_of_char_row() {
        let mut crtc = Crtc::new();
        // R0=4 (5 CCLKs/line), R1=2 (2 displayed), R9=1 (2 scanlines/row)
        write_register(&mut crtc, 0, 4);
        write_register(&mut crtc, 1, 2);
        write_register(&mut crtc, 4, 2);
        write_register(&mut crtc, 9, 1);
        write_register(&mut crtc, 12, 0);
        write_register(&mut crtc, 13, 0);

        // Tick through one full scanline (5 CCLKs → C0 wraps, C9→1)
        for _ in 0..5 {
            crtc.tick();
        }

        // Now at C4=0, C9=1, C0=0 — start of 2nd scanline, SAME char row.
        // VMA must still be 0: VMA' must not update until C9==R9.
        assert_eq!(crtc.c9(), 1);
        assert_eq!(crtc.c0(), 0);
        assert_eq!(
            crtc.current_ma(),
            0,
            "VMA at start of 2nd scanline must equal frame start; \
         VMA' must only update when C9==R9 (end of char row)"
        );
    }
}
