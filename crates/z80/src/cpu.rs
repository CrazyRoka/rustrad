use crate::bus::Bus;

const FLAG_SIGN: u8 = 1 << 7;
const FLAG_ZERO: u8 = 1 << 6;
const FLAG_HALF_CARRY: u8 = 1 << 4;
const FLAG_PARITY: u8 = 1 << 2;
const FLAG_ADD_OR_SUBTRACT: u8 = 1 << 1;
const FLAG_CARRY: u8 = 1 << 0;

#[derive(Default, PartialEq, Eq, Debug)]
enum InterruptMode {
    #[default]
    IM1,
    IM2,
    IM0,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IndexReg {
    IX,
    IY,
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Z80 {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    i: u8,
    r: u8,
    p: bool,
    w: u8,
    z: u8,
    q: u8,
    a_shadow: u8,
    b_shadow: u8,
    c_shadow: u8,
    d_shadow: u8,
    e_shadow: u8,
    f_shadow: u8,
    h_shadow: u8,
    l_shadow: u8,
    w_shadow: u8,
    z_shadow: u8,
    pc: u16,
    sp: u16,
    ix: u16,
    iy: u16,
    ei: bool,
    im: InterruptMode,
    iff1: bool,
    iff2: bool,
    pending_nmi: bool,
    pending_int: Option<u8>,
    halted: bool,
}

impl Z80 {
    pub fn new() -> Self {
        Self {
            a: 0xFF,
            b: 0xFF,
            c: 0xFF,
            d: 0xFF,
            e: 0xFF,
            f: 0xFF,
            h: 0xFF,
            l: 0xFF,
            i: 0xFF,
            r: 0,
            p: true,
            w: 0xFF,
            z: 0xFF,
            q: 0xFF,
            a_shadow: 0xFF,
            b_shadow: 0xFF,
            c_shadow: 0xFF,
            d_shadow: 0xFF,
            e_shadow: 0xFF,
            f_shadow: 0xFF,
            h_shadow: 0xFF,
            l_shadow: 0xFF,
            w_shadow: 0xFF,
            z_shadow: 0xFF,
            pc: 0,
            sp: 0xFFFF,
            ix: 0xFFFF,
            iy: 0xFFFF,
            ei: false,
            im: InterruptMode::IM0,
            iff1: false,
            iff2: false,
            pending_nmi: false,
            pending_int: None,
            halted: false,
        }
    }

    pub fn execute<B: Bus>(&mut self, bus: &mut B) -> u64 {
        if self.pending_nmi {
            self.pending_nmi = false;
            self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);
            self.iff1 = false;
            self.halted = false;
            self.push_word(bus, self.pc);
            self.set_wz(0x0066);
            self.pc = 0x0066;
            return 11;
        }

        if let Some(data) = self.pending_int
            && !self.ei
            && self.iff1
        {
            self.pending_int = None;
            self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);
            self.halted = false;
            self.iff1 = false;
            self.iff2 = false;

            bus.acknowledge_interrupt();
            return match self.im {
                InterruptMode::IM0 => {
                    dbg!(
                        "WARNING: IM0 called, check if opcode is 1 byte width or not - maybe we need to refactor",
                        data
                    );
                    2 + self.step(bus, data)
                }
                InterruptMode::IM1 => 2 + self.step(bus, 0xFF),
                InterruptMode::IM2 => {
                    self.push_word(bus, self.pc);
                    let addr = ((self.i as u16) << 8) | data as u16;
                    self.set_wz(addr);
                    let low = bus.read(addr);
                    let high = bus.read(addr.wrapping_add(1));
                    self.pc = ((high as u16) << 8) | low as u16;
                    19
                }
            };
        }

        if self.halted {
            return 4;
        }

        self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);
        let opcode = self.read_byte(bus);
        let next_opcode = bus.read(self.pc);
        self.ei = false;
        self.p = false;
        let old_f = self.f;
        let old_q = self.q;
        let cycles = self.step(bus, opcode);
        self.q = if (old_f != self.f || old_q != self.q) && opcode != 0x08 && opcode != 0xF1 {
            if (opcode == 0xDD || opcode == 0xFD) && (next_opcode == 0x08 || next_opcode == 0xF1) {
                0
            } else {
                self.f
            }
        } else {
            0
        };
        cycles
    }

    fn read_byte<B: Bus>(&mut self, bus: &mut B) -> u8 {
        let byte = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn read_word<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let lo = self.read_byte(bus) as u16;
        let hi = self.read_byte(bus) as u16;
        (hi << 8) | lo
    }

    fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    fn wz(&self) -> u16 {
        ((self.w as u16) << 8) | (self.z as u16)
    }

    fn set_wz(&mut self, value: u16) {
        self.w = (value >> 8) as u8;
        self.z = (value & 0xFF) as u8;
    }

    fn get_rp(&self, idx: u8) -> u16 {
        match idx {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => panic!("Unexpected index {idx}"),
        }
    }

    fn set_rp(&mut self, idx: u8, value: u16) {
        match idx {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.sp = value,
            _ => panic!("Unexpected index {idx}"),
        }
    }

    fn get_reg<B: Bus>(&self, bus: &B, idx: u8) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => bus.read(self.hl()),
            7 => self.a,
            _ => panic!("Unexpected index {idx}"),
        }
    }

    fn set_reg<B: Bus>(&mut self, bus: &mut B, idx: u8, value: u8) {
        match idx {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            6 => bus.write(self.hl(), value),
            7 => self.a = value,
            _ => panic!("Unexpected index {idx}"),
        }
    }

    fn parity(value: u8) -> bool {
        value.count_ones() % 2 == 0
    }

    fn inc8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.f = if result == 0 { FLAG_ZERO } else { 0 }
            | if result & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | (result & 0x28) // Mirror bits 3 and 5 from the result
            | if value & 0x0F == 0x0F {
                FLAG_HALF_CARRY
            } else {
                0
            }
            | if value == 0x7F { FLAG_PARITY } else { 0 }
            | (self.f & FLAG_CARRY);
        self.q = !self.q;
        result
    }

    fn dec8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.f = if result == 0 { FLAG_ZERO } else { 0 }
            | if result & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | (result & 0x28) // Mirror bits 3 and 5 from the result
            | if value & 0x0F == 0x00 {
                FLAG_HALF_CARRY
            } else {
                0
            }
            | if value == 0x80 { FLAG_PARITY } else { 0 }
            | FLAG_ADD_OR_SUBTRACT
            | (self.f & FLAG_CARRY);
        self.q = !self.q;
        result
    }

    fn add_hl(&mut self, value: u16) {
        let hl = self.hl();
        let (result, carry) = hl.overflowing_add(value);
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if carry { FLAG_CARRY } else { 0 }
            | ((result >> 8) & 0x28) as u8;
        self.q = !self.q;
        self.set_wz(hl.wrapping_add(1));
        self.set_hl(result);
    }

    fn check_cc(&self, cc: u8) -> bool {
        match cc {
            0 => self.f & FLAG_ZERO == 0,   // NZ
            1 => self.f & FLAG_ZERO != 0,   // Z
            2 => self.f & FLAG_CARRY == 0,  // NC
            3 => self.f & FLAG_CARRY != 0,  // C
            4 => self.f & FLAG_PARITY == 0, // PO (parity odd)
            5 => self.f & FLAG_PARITY != 0, // PE (parity even)
            6 => self.f & FLAG_SIGN == 0,   // P (positive)
            7 => self.f & FLAG_SIGN != 0,   // M (minus)
            _ => panic!("Unexpected cc {cc}"),
        }
    }

    fn push_word<B: Bus>(&mut self, bus: &mut B, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xFF) as u8;
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, lo);
    }

    fn pop_word<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let lo = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn add_a(&mut self, value: u8, with_carry: bool) {
        let c = if with_carry && self.f & FLAG_CARRY != 0 {
            1
        } else {
            0
        };
        let result = self.a as u16 + value as u16 + c as u16;
        let half_carry = (self.a & 0x0F) + (value & 0x0F) + c > 0x0F;
        let result8 = result as u8;
        let overflow = ((self.a ^ result8) & (value ^ result8) & 0x80) != 0;
        self.f = if result8 == 0 { FLAG_ZERO } else { 0 }
            | if result8 & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if overflow { FLAG_PARITY } else { 0 }
            | if result > 0xFF { FLAG_CARRY } else { 0 }
            | (result8 & 0x28);
        self.q = !self.q;
        self.a = result8;
    }

    fn sub_a(&mut self, value: u8, with_carry: bool, store: bool) {
        let c = if with_carry && self.f & FLAG_CARRY != 0 {
            1
        } else {
            0
        };
        let result = self.a.wrapping_sub(value).wrapping_sub(c);
        let carry = (self.a as u16) < value as u16 + c as u16;
        let half_carry = ((self.a & 0x0F) as u16) < (value & 0x0F) as u16 + c as u16;
        let overflow = ((self.a ^ value) & 0x80) != 0 && ((self.a ^ result) & 0x80) != 0;
        self.f = if result == 0 { FLAG_ZERO } else { 0 }
            | if result & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if overflow { FLAG_PARITY } else { 0 }
            | if carry { FLAG_CARRY } else { 0 }
            | FLAG_ADD_OR_SUBTRACT
            | if store { result & 0x28 } else { value & 0x28 };
        self.q = !self.q;
        if store {
            self.a = result;
        }
    }

    fn and_a(&mut self, value: u8) {
        self.a &= value;
        self.f = if self.a == 0 { FLAG_ZERO } else { 0 }
            | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | FLAG_HALF_CARRY
            | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
            | (self.a & 0x28);
        self.q = !self.q;
    }

    fn xor_a(&mut self, value: u8) {
        self.a ^= value;
        self.f = if self.a == 0 { FLAG_ZERO } else { 0 }
            | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
            | (self.a & 0x28);
        self.q = !self.q;
    }

    fn or_a(&mut self, value: u8) {
        self.a |= value;
        self.f = if self.a == 0 { FLAG_ZERO } else { 0 }
            | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
            | (self.a & 0x28);
        self.q = !self.q;
    }

    fn daa(&mut self) {
        let a = self.a;
        let carry = self.f & FLAG_CARRY != 0;
        let half = self.f & FLAG_HALF_CARRY != 0;
        let sub = self.f & FLAG_ADD_OR_SUBTRACT != 0;
        let mut add = 0u8;
        let mut new_carry = carry;

        if half || (a & 0x0F) > 9 {
            add |= 0x06;
        }
        if carry || (a & 0xF0) > 0x90 || ((a & 0x0F) > 9 && (a & 0xF0) > 0x80) {
            add |= 0x60;
            new_carry = true;
        }

        self.a = if sub {
            a.wrapping_sub(add)
        } else {
            a.wrapping_add(add)
        };
        self.f = if new_carry { FLAG_CARRY } else { 0 }
            | if self.a == 0 { FLAG_ZERO } else { 0 }
            | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
            | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
            | if sub { FLAG_ADD_OR_SUBTRACT } else { 0 }
            | (self.a & 0x28)
            | if sub && half && (a & 0x0F) < 6 {
                FLAG_HALF_CARRY
            } else {
                0
            }
            | if !sub && (a & 0x0F) > 9 {
                FLAG_HALF_CARRY
            } else {
                0
            };
        self.q = !self.q;
    }

    pub fn request_nmi(&mut self) {
        self.pending_nmi = true;
    }

    pub fn request_int(&mut self, data: u8) {
        self.pending_int = Some(data);
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    fn step<B: Bus>(&mut self, bus: &mut B, opcode: u8) -> u64 {
        match opcode {
            // NOP
            0x00 => 4, // NOP
            0x01 => {
                let word = self.read_word(bus);
                self.set_bc(word);
                10
            } // LD BC,nn
            0x02 => {
                bus.write(self.bc(), self.a);
                self.w = self.a;
                self.z = self.c.wrapping_add(1);
                7
            } // LD (BC),A
            0x03 => {
                self.set_bc(self.bc().wrapping_add(1));
                6
            } // INC BC
            0x04 => {
                self.b = self.inc8(self.b);
                4
            } // INC B
            0x05 => {
                self.b = self.dec8(self.b);
                4
            } // DEC B
            0x06 => {
                self.b = self.read_byte(bus);
                7
            } // LD B,n
            0x07 => {
                // RLCA
                let c = self.a >> 7;
                self.a = (self.a << 1) | c;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY)) | c | (self.a & 0x28);
                self.q = !self.q;
                4
            }
            0x08 => {
                // EX AF,AF'
                std::mem::swap(&mut self.a, &mut self.a_shadow);
                std::mem::swap(&mut self.f, &mut self.f_shadow);
                4
            }
            0x09 => {
                self.add_hl(self.bc());
                11
            } // ADD HL,BC
            0x0A => {
                self.a = bus.read(self.bc());
                self.set_wz(self.bc().wrapping_add(1));
                7
            } // LD A,(BC)
            0x0B => {
                self.set_bc(self.bc().wrapping_sub(1));
                6
            } // DEC BC
            0x0C => {
                self.c = self.inc8(self.c);
                4
            } // INC C
            0x0D => {
                self.c = self.dec8(self.c);
                4
            } // DEC C
            0x0E => {
                self.c = self.read_byte(bus);
                7
            } // LD C,n
            0x0F => {
                // RRCA
                let c = self.a & 1;
                self.a = (self.a >> 1) | (c << 7);
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY)) | c | (self.a & 0x28);
                self.q = !self.q; // reset inside execute
                4
            }
            0x10 => {
                // DJNZ d
                self.b = self.b.wrapping_sub(1);
                let displacement = self.read_byte(bus) as i8 as u16;
                if self.b != 0 {
                    self.pc = self.pc.wrapping_add(displacement);
                    self.set_wz(self.pc);
                    13
                } else {
                    8
                }
            }
            0x11 => {
                let word = self.read_word(bus);
                self.set_de(word);
                10
            } // LD DE,nn
            0x12 => {
                bus.write(self.de(), self.a);
                self.w = self.a;
                self.z = self.e.wrapping_add(1);
                7
            } // LD (DE),A
            0x13 => {
                self.set_de(self.de().wrapping_add(1));
                6
            } // INC DE
            0x14 => {
                self.d = self.inc8(self.d);
                4
            } // INC D
            0x15 => {
                self.d = self.dec8(self.d);
                4
            } // DEC D
            0x16 => {
                self.d = self.read_byte(bus);
                7
            } // LD D,n
            0x17 => {
                // RLA
                let old_c = if self.f & FLAG_CARRY != 0 { 1 } else { 0 };
                let new_c = self.a >> 7;
                self.a = (self.a << 1) | old_c;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY)) | new_c | (self.a & 0x28);
                self.q = !self.q; // reset inside execute
                4
            }
            0x18 => {
                // JR d
                let displacement = self.read_byte(bus) as i8 as u16;
                self.pc = self.pc.wrapping_add(displacement);
                self.set_wz(self.pc);
                12
            }
            0x19 => {
                self.add_hl(self.de());
                11
            } // ADD HL,DE
            0x1A => {
                self.a = bus.read(self.de());
                self.set_wz(self.de().wrapping_add(1));
                7
            } // LD A,(DE)
            0x1B => {
                self.set_de(self.de().wrapping_sub(1));
                6
            } // DEC DE
            0x1C => {
                self.e = self.inc8(self.e);
                4
            } // INC E
            0x1D => {
                self.e = self.dec8(self.e);
                4
            } // DEC E
            0x1E => {
                self.e = self.read_byte(bus);
                7
            } // LD E,n
            0x1F => {
                // RRA
                let old_c = if self.f & FLAG_CARRY != 0 { 0x80 } else { 0 };
                let new_c = self.a & 1;
                self.a = (self.a >> 1) | old_c;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY)) | new_c | (self.a & 0x28);
                self.q = !self.q;
                4
            }
            0x20 => {
                // JR NZ,d
                let displacement = self.read_byte(bus) as i8 as u16;
                if self.f & FLAG_ZERO == 0 {
                    self.pc = self.pc.wrapping_add(displacement);
                    self.set_wz(self.pc);
                    12
                } else {
                    7
                }
            }
            0x21 => {
                let word = self.read_word(bus);
                self.set_hl(word);
                10
            } // LD HL,nn
            0x22 => {
                // LD (nn),HL
                let addr = self.read_word(bus);
                bus.write(addr, self.l);
                bus.write(addr.wrapping_add(1), self.h);
                self.set_wz(addr.wrapping_add(1));
                16
            }
            0x23 => {
                self.set_hl(self.hl().wrapping_add(1));
                6
            } // INC HL
            0x24 => {
                self.h = self.inc8(self.h);
                4
            } // INC H
            0x25 => {
                self.h = self.dec8(self.h);
                4
            } // DEC H
            0x26 => {
                self.h = self.read_byte(bus);
                7
            } // LD H,n
            0x27 => {
                self.daa();
                4
            } // DAA
            0x28 => {
                // JR Z,d
                let displacement = self.read_byte(bus) as i8 as u16;
                if self.f & FLAG_ZERO != 0 {
                    self.pc = self.pc.wrapping_add(displacement);
                    self.set_wz(self.pc);
                    12
                } else {
                    7
                }
            }
            0x29 => {
                self.add_hl(self.hl());
                11
            } // ADD HL,HL
            0x2A => {
                // LD HL,(nn)
                let addr = self.read_word(bus);
                self.l = bus.read(addr);
                self.h = bus.read(addr.wrapping_add(1));
                self.set_wz(addr.wrapping_add(1));
                16
            }
            0x2B => {
                self.set_hl(self.hl().wrapping_sub(1));
                6
            } // DEC HL
            0x2C => {
                self.l = self.inc8(self.l);
                4
            } // INC L
            0x2D => {
                self.l = self.dec8(self.l);
                4
            } // DEC L
            0x2E => {
                self.l = self.read_byte(bus);
                7
            } // LD L,n
            0x2F => {
                // CPL
                self.a = !self.a;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY | FLAG_CARRY))
                    | FLAG_HALF_CARRY
                    | FLAG_ADD_OR_SUBTRACT
                    | (self.a & 0x28); // Mirror bits 3 and 5 from the new A
                self.q = !self.q;
                4
            }
            0x30 => {
                // JR NC,d
                let displacement = self.read_byte(bus) as i8 as u16;
                if self.f & FLAG_CARRY == 0 {
                    self.pc = self.pc.wrapping_add(displacement);
                    self.set_wz(self.pc);
                    12
                } else {
                    7
                }
            }
            0x31 => {
                self.sp = self.read_word(bus);
                10
            } // LD SP,nn
            0x32 => {
                // LD (nn),A
                let addr = self.read_word(bus);
                bus.write(addr, self.a);
                self.w = self.a;
                self.z = (addr.wrapping_add(1) & 0xFF) as u8;
                13
            }
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                6
            } // INC SP
            0x34 => {
                // INC (HL)
                let addr = self.hl();
                let value = bus.read(addr);
                let result = self.inc8(value);
                bus.write(addr, result);
                11
            }
            0x35 => {
                // DEC (HL)
                let addr = self.hl();
                let value = bus.read(addr);
                let result = self.dec8(value);
                bus.write(addr, result);
                11
            }
            0x36 => {
                // LD (HL),n
                let value = self.read_byte(bus);
                bus.write(self.hl(), value);
                10
            }
            0x37 => {
                // SCF
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
                    | FLAG_CARRY
                    | (((self.q ^ self.f) | self.a) & 0x28);
                self.q = !self.q;
                4
            }
            0x38 => {
                // JR C,d
                let displacement = self.read_byte(bus) as i8 as u16;
                if self.f & FLAG_CARRY != 0 {
                    self.pc = self.pc.wrapping_add(displacement);
                    self.set_wz(self.pc);
                    12
                } else {
                    7
                }
            }
            0x39 => {
                self.add_hl(self.sp);
                11
            } // ADD HL,SP
            0x3A => {
                // LD A,(nn)
                let addr = self.read_word(bus);
                self.a = bus.read(addr);
                self.set_wz(addr.wrapping_add(1));
                13
            }
            0x3B => {
                self.sp = self.sp.wrapping_sub(1);
                6
            } // DEC SP
            0x3C => {
                self.a = self.inc8(self.a);
                4
            } // INC A
            0x3D => {
                self.a = self.dec8(self.a);
                4
            } // DEC A
            0x3E => {
                self.a = self.read_byte(bus);
                7
            } // LD A,n
            0x3F => {
                // CCF
                let old_carry = self.f & FLAG_CARRY;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
                    | if old_carry != 0 { FLAG_HALF_CARRY } else { 0 }
                    | if old_carry == 0 { FLAG_CARRY } else { 0 }
                    | (((self.q ^ self.f) | self.a) & 0x28);
                self.q = !self.q;
                4
            }
            0x40..=0x7F if opcode != 0x76 => {
                let dst = (opcode >> 3) & 7;
                let src = opcode & 7;
                let val = self.get_reg(bus, src);
                self.set_reg(bus, dst, val);
                if dst == 6 || src == 6 { 7 } else { 4 }
            }
            0x76 => {
                // HALT
                self.halted = true;
                4
            }
            0x80..=0xBF => {
                let op = (opcode >> 3) & 7;
                let src = opcode & 7;
                let val = self.get_reg(bus, src);
                match op {
                    0 => self.add_a(val, false),        // ADD
                    1 => self.add_a(val, true),         // ADC
                    2 => self.sub_a(val, false, true),  // SUB
                    3 => self.sub_a(val, true, true),   // SBC
                    4 => self.and_a(val),               // AND
                    5 => self.xor_a(val),               // XOR
                    6 => self.or_a(val),                // OR
                    7 => self.sub_a(val, false, false), // CP
                    _ => panic!("Unexpected OP {op}"),
                }
                if src == 6 { 7 } else { 4 }
            }
            // RET cc
            0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8 => {
                let cc = (opcode >> 3) & 7;
                if self.check_cc(cc) {
                    let pc = self.pop_word(bus);
                    self.pc = pc;
                    self.set_wz(pc);
                    11
                } else {
                    5
                }
            }
            0xC1 => {
                // POP BC
                let word = self.pop_word(bus);
                self.set_bc(word);
                10
            }
            // JP cc,nn
            0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA => {
                let cc = (opcode >> 3) & 7;
                let addr = self.read_word(bus);
                self.set_wz(addr);
                if self.check_cc(cc) {
                    self.pc = addr;
                }
                10
            }
            0xC3 => {
                // JP nn
                let addr = self.read_word(bus);
                self.pc = addr;
                self.set_wz(addr);
                10
            }
            // CALL cc,nn
            0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC => {
                let cc = (opcode >> 3) & 7;
                let addr = self.read_word(bus);
                self.set_wz(addr);
                if self.check_cc(cc) {
                    self.push_word(bus, self.pc);
                    self.pc = addr;
                    17
                } else {
                    10
                }
            }
            0xC5 => {
                // PUSH BC
                self.push_word(bus, self.bc());
                11
            }
            0xC6 => {
                // ADD A,n
                let n = self.read_byte(bus);
                self.add_a(n, false);
                7
            }
            // RST p
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                let addr = (opcode & 0x38) as u16;
                self.push_word(bus, self.pc);
                self.pc = addr;
                self.set_wz(addr);
                11
            }
            0xC9 => {
                // RET
                let pc = self.pop_word(bus);
                self.pc = pc;
                self.set_wz(pc);
                10
            }
            0xCB => self.step_cb(bus),
            0xCD => {
                // CALL nn
                let addr = self.read_word(bus);
                self.push_word(bus, self.pc);
                self.pc = addr;
                self.set_wz(addr);
                17
            }
            0xCE => {
                // ADC A,n
                let n = self.read_byte(bus);
                self.add_a(n, true);
                7
            }
            0xD1 => {
                // POP DE
                let word = self.pop_word(bus);
                self.set_de(word);
                10
            }
            0xD3 => {
                // OUT (n),A
                let n = self.read_byte(bus);
                let port = ((self.a as u16) << 8) | n as u16;
                self.set_wz(((self.a as u16) << 8) | n.wrapping_add(1) as u16);
                bus.port_write(port, self.a);
                11
            }
            0xD5 => {
                // PUSH DE
                self.push_word(bus, self.de());
                11
            }
            0xD6 => {
                // SUB n
                let n = self.read_byte(bus);
                self.sub_a(n, false, true);
                7
            }
            0xD9 => {
                // EXX
                std::mem::swap(&mut self.b, &mut self.b_shadow);
                std::mem::swap(&mut self.c, &mut self.c_shadow);
                std::mem::swap(&mut self.d, &mut self.d_shadow);
                std::mem::swap(&mut self.e, &mut self.e_shadow);
                std::mem::swap(&mut self.h, &mut self.h_shadow);
                std::mem::swap(&mut self.l, &mut self.l_shadow);
                4
            }
            0xDB => {
                // IN A,(n)
                let n = self.read_byte(bus);
                let port = ((self.a as u16) << 8) | n as u16;
                self.set_wz(port.wrapping_add(1));
                self.a = bus.port_read(port);
                11
            }
            0xDD => self.step_index(bus, IndexReg::IX),
            0xDE => {
                // SBC A,n
                let n = self.read_byte(bus);
                self.sub_a(n, true, true);
                7
            }
            0xE1 => {
                // POP HL
                let word = self.pop_word(bus);
                self.set_hl(word);
                10
            }
            0xE3 => {
                // EX (SP),HL
                let sp = self.sp;
                let old_l = self.l;
                let old_h = self.h;
                self.l = bus.read(sp);
                self.h = bus.read(sp.wrapping_add(1));
                bus.write(sp, old_l);
                bus.write(sp.wrapping_add(1), old_h);
                self.set_wz(self.hl());
                19
            }
            0xE5 => {
                // PUSH HL
                self.push_word(bus, self.hl());
                11
            }
            0xE6 => {
                // AND n
                let n = self.read_byte(bus);
                self.and_a(n);
                7
            }
            0xE9 => {
                // JP (HL)
                self.pc = self.hl();
                4
            }
            0xEB => {
                // EX DE,HL
                let de = self.de();
                let hl = self.hl();
                self.set_de(hl);
                self.set_hl(de);
                4
            }
            0xED => self.step_ed(bus),
            0xEE => {
                // XOR n
                let n = self.read_byte(bus);
                self.xor_a(n);
                7
            }
            0xF1 => {
                // POP AF
                let word = self.pop_word(bus);
                self.a = (word >> 8) as u8;
                self.f = (word & 0xFF) as u8;
                10
            }
            0xF3 => {
                // DI
                self.iff1 = false;
                self.iff2 = false;
                4
            }
            0xF5 => {
                // PUSH AF
                let word = ((self.a as u16) << 8) | (self.f as u16);
                self.push_word(bus, word);
                11
            }
            0xF6 => {
                // OR n
                let n = self.read_byte(bus);
                self.or_a(n);
                7
            }
            0xF9 => {
                // LD SP,HL
                self.sp = self.hl();
                6
            }
            0xFB => {
                // EI
                self.iff1 = true;
                self.iff2 = true;
                self.ei = true;
                4
            }
            0xFD => self.step_index(bus, IndexReg::IY),
            0xFE => {
                // CP n
                let n = self.read_byte(bus);
                self.sub_a(n, false, false);
                7
            }
            _ => panic!("Unexpected opcode {opcode:02X}"),
        }
    }

    fn get_index(&self, idx: IndexReg) -> u16 {
        match idx {
            IndexReg::IX => self.ix,
            IndexReg::IY => self.iy,
        }
    }

    fn set_index(&mut self, idx: IndexReg, value: u16) {
        match idx {
            IndexReg::IX => self.ix = value,
            IndexReg::IY => self.iy = value,
        }
    }

    fn get_index_h(&self, idx: IndexReg) -> u8 {
        (self.get_index(idx) >> 8) as u8
    }

    fn set_index_h(&mut self, idx: IndexReg, value: u8) {
        let v = self.get_index(idx);
        self.set_index(idx, ((value as u16) << 8) | (v & 0xFF));
    }

    fn get_index_l(&self, idx: IndexReg) -> u8 {
        (self.get_index(idx) & 0xFF) as u8
    }

    fn set_index_l(&mut self, idx: IndexReg, value: u8) {
        let v = self.get_index(idx);
        self.set_index(idx, (v & 0xFF00) | value as u16);
    }

    fn get_index_reg(&self, idx: IndexReg, reg: u8) -> u8 {
        match reg {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.get_index_h(idx),
            5 => self.get_index_l(idx),
            7 => self.a,
            _ => panic!("Unexpected reg {reg} for index"),
        }
    }

    fn set_index_reg(&mut self, idx: IndexReg, reg: u8, value: u8) {
        match reg {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.set_index_h(idx, value),
            5 => self.set_index_l(idx, value),
            7 => self.a = value,
            _ => panic!("Unexpected reg {reg} for index"),
        }
    }

    fn read_index_addr<B: Bus>(&mut self, bus: &mut B, idx: IndexReg) -> u16 {
        let d = self.read_byte(bus) as i8 as i16 as u16;
        let addr = self.get_index(idx).wrapping_add(d);
        self.set_wz(addr);
        addr
    }

    fn add_index(&mut self, idx: IndexReg, value: u16) {
        let ix = self.get_index(idx);
        let (result, carry) = ix.overflowing_add(value);
        let half_carry = (ix & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if carry { FLAG_CARRY } else { 0 }
            | ((result >> 8) & 0x28) as u8;
        self.q = !self.q;
        self.set_wz(ix.wrapping_add(1));
        self.set_index(idx, result);
    }

    fn step_index<B: Bus>(&mut self, bus: &mut B, idx: IndexReg) -> u64 {
        let opcode = self.read_byte(bus);
        self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);

        match opcode {
            0x08 => {
                std::mem::swap(&mut self.a, &mut self.a_shadow);
                std::mem::swap(&mut self.f, &mut self.f_shadow);
                8
            }
            0x09 => {
                self.add_index(idx, self.bc());
                15
            }
            0x19 => {
                self.add_index(idx, self.de());
                15
            }
            0x29 => {
                self.add_index(idx, self.get_index(idx));
                15
            }
            0x39 => {
                self.add_index(idx, self.sp);
                15
            }
            0x21 => {
                let value = self.read_word(bus);
                self.set_index(idx, value);
                14
            }
            0x22 => {
                let addr = self.read_word(bus);
                let val = self.get_index(idx);
                bus.write(addr, val as u8);
                bus.write(addr.wrapping_add(1), (val >> 8) as u8);
                self.set_wz(addr.wrapping_add(1));
                20
            }
            0x2A => {
                let addr = self.read_word(bus);
                let lo = bus.read(addr) as u16;
                let hi = bus.read(addr.wrapping_add(1)) as u16;
                self.set_index(idx, (hi << 8) | lo);
                self.set_wz(addr.wrapping_add(1));
                20
            }
            0x23 => {
                self.set_index(idx, self.get_index(idx).wrapping_add(1));
                10
            }
            0x2B => {
                self.set_index(idx, self.get_index(idx).wrapping_sub(1));
                10
            }
            0x24 => {
                let value = self.inc8(self.get_index_h(idx));
                self.set_index_h(idx, value);
                8
            }
            0x25 => {
                let value = self.dec8(self.get_index_h(idx));
                self.set_index_h(idx, value);
                8
            }
            0x26 => {
                let value = self.read_byte(bus);
                self.set_index_h(idx, value);
                11
            }
            0x2C => {
                let value = self.inc8(self.get_index_l(idx));
                self.set_index_l(idx, value);
                8
            }
            0x2D => {
                let value = self.dec8(self.get_index_l(idx));
                self.set_index_l(idx, value);
                8
            }
            0x2E => {
                let value = self.read_byte(bus);
                self.set_index_l(idx, value);
                11
            }
            0x34 => {
                let addr = self.read_index_addr(bus, idx);
                let value = bus.read(addr);
                let result = self.inc8(value);
                bus.write(addr, result);
                23
            }
            0x35 => {
                let addr = self.read_index_addr(bus, idx);
                let value = bus.read(addr);
                let result = self.dec8(value);
                bus.write(addr, result);
                23
            }
            0x36 => {
                let addr = self.read_index_addr(bus, idx);
                let n = self.read_byte(bus);
                bus.write(addr, n);
                19
            }
            0x37 => {
                // SCF
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
                    | FLAG_CARRY
                    | ((self.f | self.a) & 0x28);
                self.q = !self.q;
                8
            }
            0x3F => {
                // CCF
                let old_carry = self.f & FLAG_CARRY;
                self.f = (self.f & (FLAG_SIGN | FLAG_ZERO | FLAG_PARITY))
                    | if old_carry != 0 { FLAG_HALF_CARRY } else { 0 }
                    | if old_carry == 0 { FLAG_CARRY } else { 0 }
                    | ((self.f | self.a) & 0x28);
                self.q = !self.q;
                8
            }
            0x40..=0x7F if opcode != 0x76 => {
                let dst = (opcode >> 3) & 7;
                let src = opcode & 7;
                if src == 6 {
                    let addr = self.read_index_addr(bus, idx);
                    let val = bus.read(addr);
                    self.set_reg(bus, dst, val);
                    19
                } else if dst == 6 {
                    let addr = self.read_index_addr(bus, idx);
                    let val = self.get_reg(bus, src);
                    bus.write(addr, val);
                    19
                } else {
                    let val = self.get_index_reg(idx, src);
                    self.set_index_reg(idx, dst, val);
                    8
                }
            }
            0x80..=0xBF => {
                let op = (opcode >> 3) & 7;
                let src = opcode & 7;
                let val = if src == 6 {
                    let addr = self.read_index_addr(bus, idx);
                    bus.read(addr)
                } else {
                    self.get_index_reg(idx, src)
                };
                match op {
                    0 => self.add_a(val, false),
                    1 => self.add_a(val, true),
                    2 => self.sub_a(val, false, true),
                    3 => self.sub_a(val, true, true),
                    4 => self.and_a(val),
                    5 => self.xor_a(val),
                    6 => self.or_a(val),
                    7 => self.sub_a(val, false, false),
                    _ => panic!("Unexpected OP {op}"),
                }
                if src == 6 { 19 } else { 8 }
            }
            0xCB => self.step_index_cb(bus, idx),
            0xE1 => {
                let word = self.pop_word(bus);
                self.set_index(idx, word);
                14
            }
            0xE3 => {
                let sp = self.sp;
                let old_ix = self.get_index(idx);
                let old_lo = (old_ix & 0xFF) as u8;
                let old_hi = (old_ix >> 8) as u8;
                let new_lo = bus.read(sp);
                let new_hi = bus.read(sp.wrapping_add(1));
                bus.write(sp, old_lo);
                bus.write(sp.wrapping_add(1), old_hi);
                let new_ix = ((new_hi as u16) << 8) | new_lo as u16;
                self.set_index(idx, new_ix);
                self.set_wz(new_ix);
                23
            }
            0xE5 => {
                self.push_word(bus, self.get_index(idx));
                15
            }
            0xE9 => {
                self.pc = self.get_index(idx);
                8
            }
            0xF1 => {
                let word = self.pop_word(bus);
                self.a = (word >> 8) as u8;
                self.f = (word & 0xFF) as u8;
                14
            }
            0xF9 => {
                self.sp = self.get_index(idx);
                10
            }
            _ => match opcode {
                0xDD => self.step_index(bus, IndexReg::IX) + 4,
                0xFD => self.step_index(bus, IndexReg::IY) + 4,
                0xED => self.step_ed(bus) + 4,
                _ => self.step(bus, opcode) + 4,
            },
        }
    }

    fn step_index_cb<B: Bus>(&mut self, bus: &mut B, idx: IndexReg) -> u64 {
        let d = self.read_byte(bus) as i8 as i16 as u16;
        let addr = self.get_index(idx).wrapping_add(d);
        self.set_wz(addr);
        let opcode = self.read_byte(bus);
        let op = opcode >> 6;
        let sub = (opcode >> 3) & 7;
        let reg = opcode & 7;

        match op {
            0 => {
                let value = bus.read(addr);
                let (result, carry) = match sub {
                    0 => {
                        let c = value >> 7;
                        ((value << 1) | c, c)
                    }
                    1 => {
                        let c = value & 1;
                        ((value >> 1) | (c << 7), c)
                    }
                    2 => {
                        let old_c = if self.f & FLAG_CARRY != 0 { 1 } else { 0 };
                        let new_c = value >> 7;
                        ((value << 1) | old_c, new_c)
                    }
                    3 => {
                        let old_c = if self.f & FLAG_CARRY != 0 { 0x80 } else { 0 };
                        let new_c = value & 1;
                        ((value >> 1) | old_c, new_c)
                    }
                    4 => {
                        let new_c = value >> 7;
                        (value << 1, new_c)
                    }
                    5 => {
                        let new_c = value & 1;
                        ((value >> 1) | (value & 0x80), new_c)
                    }
                    6 => {
                        let new_c = value >> 7;
                        ((value << 1) | 1, new_c)
                    }
                    7 => {
                        let new_c = value & 1;
                        (value >> 1, new_c)
                    }
                    _ => panic!("Unexpected SUB {sub}"),
                };
                self.f = if result == 0 { FLAG_ZERO } else { 0 }
                    | if result & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(result) { FLAG_PARITY } else { 0 }
                    | carry
                    | (result & 0x28);
                self.q = !self.q;
                bus.write(addr, result);
                if reg != 6 {
                    self.set_reg(bus, reg, result);
                }
                23
            }
            1 => {
                let value = bus.read(addr);
                let test = value & (1 << sub);
                let xy_source = (addr >> 8) as u8;
                self.f = (self.f & FLAG_CARRY)
                    | FLAG_HALF_CARRY
                    | if test == 0 {
                        FLAG_ZERO | FLAG_PARITY
                    } else {
                        0
                    }
                    | if sub == 7 && test != 0 { FLAG_SIGN } else { 0 }
                    | (xy_source & 0x28);
                self.q = !self.q;
                20
            }
            2 => {
                let value = bus.read(addr);
                let result = value & !(1 << sub);
                bus.write(addr, result);
                if reg != 6 {
                    self.set_reg(bus, reg, result);
                }
                23
            }
            3 => {
                let value = bus.read(addr);
                let result = value | (1 << sub);
                bus.write(addr, result);
                if reg != 6 {
                    self.set_reg(bus, reg, result);
                }
                23
            }
            _ => panic!("Unexpected OP {op}"),
        }
    }

    fn step_cb<B: Bus>(&mut self, bus: &mut B) -> u64 {
        let opcode = self.read_byte(bus);
        self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);
        let op = opcode >> 6;
        let sub = (opcode >> 3) & 7;
        let reg = opcode & 7;
        let is_hl = reg == 6;

        let cycles = match op {
            0 => {
                // Rotate/shift
                let value = self.get_reg(bus, reg);
                let (result, carry) = match sub {
                    0 => {
                        // RLC
                        let c = value >> 7;
                        ((value << 1) | c, c)
                    }
                    1 => {
                        // RRC
                        let c = value & 1;
                        ((value >> 1) | (c << 7), c)
                    }
                    2 => {
                        // RL
                        let old_c = if self.f & FLAG_CARRY != 0 { 1 } else { 0 };
                        let new_c = value >> 7;
                        ((value << 1) | old_c, new_c)
                    }
                    3 => {
                        // RR
                        let old_c = if self.f & FLAG_CARRY != 0 { 0x80 } else { 0 };
                        let new_c = value & 1;
                        ((value >> 1) | old_c, new_c)
                    }
                    4 => {
                        // SLA
                        let new_c = value >> 7;
                        (value << 1, new_c)
                    }
                    5 => {
                        // SRA
                        let new_c = value & 1;
                        ((value >> 1) | (value & 0x80), new_c)
                    }
                    6 => {
                        // SLL (undocumented)
                        let new_c = value >> 7;
                        ((value << 1) | 1, new_c)
                    }
                    7 => {
                        // SRL
                        let new_c = value & 1;
                        (value >> 1, new_c)
                    }
                    _ => panic!("Unexpected sub {sub}"),
                };
                self.f = if result == 0 { FLAG_ZERO } else { 0 }
                    | if result & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(result) { FLAG_PARITY } else { 0 }
                    | carry
                    | (result & 0x28);
                self.q = !self.q;
                self.set_reg(bus, reg, result);
                if is_hl { 15 } else { 8 }
            }
            1 => {
                // BIT n, r
                let value = self.get_reg(bus, reg);
                let bit = sub;
                let test = value & (1 << bit);
                let xy_source = if is_hl { (self.wz() >> 8) as u8 } else { value };
                self.f = (self.f & FLAG_CARRY)
                    | FLAG_HALF_CARRY
                    | if test == 0 {
                        FLAG_ZERO | FLAG_PARITY
                    } else {
                        0
                    }
                    | if bit == 7 && test != 0 { FLAG_SIGN } else { 0 }
                    | (xy_source & 0x28);
                self.q = !self.q;
                if is_hl { 12 } else { 8 }
            }
            2 => {
                // RES n, r
                let value = self.get_reg(bus, reg);
                let result = value & !(1 << sub);
                self.set_reg(bus, reg, result);
                if is_hl { 15 } else { 8 }
            }
            3 => {
                // SET n, r
                let value = self.get_reg(bus, reg);
                let result = value | (1 << sub);
                self.set_reg(bus, reg, result);
                if is_hl { 15 } else { 8 }
            }
            _ => panic!("Unexpected op {op}"),
        };

        cycles
    }

    fn adc_hl(&mut self, value: u16) {
        let hl = self.hl();
        let c = if self.f & FLAG_CARRY != 0 { 1 } else { 0 };
        let result = hl as u32 + value as u32 + c as u32;
        let result16 = result as u16;
        let carry = result > 0xFFFF;
        let overflow = (hl ^ result16) & (value ^ result16) & 0x8000 != 0;
        let half_carry = (hl & 0x0FFF) + (value & 0x0FFF) + c > 0x0FFF;

        self.f = if result16 == 0 { FLAG_ZERO } else { 0 }
            | if result16 & 0x8000 != 0 { FLAG_SIGN } else { 0 }
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if overflow { FLAG_PARITY } else { 0 }
            | if carry { FLAG_CARRY } else { 0 }
            | ((result16 >> 8) & 0x28) as u8;
        self.set_wz(hl.wrapping_add(1));
        self.set_hl(result16);
        self.q = !self.q;
    }

    fn sbc_hl(&mut self, value: u16) {
        let hl = self.hl();
        let c = if self.f & FLAG_CARRY != 0 { 1 } else { 0 };
        let result = hl.wrapping_sub(value).wrapping_sub(c);
        let carry = hl < value.wrapping_add(c);
        let half_carry = (hl & 0x0FFF) < (value & 0x0FFF).wrapping_add(c);
        let overflow = ((hl ^ value) & (hl ^ result) & 0x8000) != 0;

        self.f = if result == 0 { FLAG_ZERO } else { 0 }
            | if result & 0x8000 != 0 { FLAG_SIGN } else { 0 }
            | if half_carry { FLAG_HALF_CARRY } else { 0 }
            | if overflow { FLAG_PARITY } else { 0 }
            | if carry { FLAG_CARRY } else { 0 }
            | FLAG_ADD_OR_SUBTRACT
            | ((result >> 8) & 0x28) as u8;
        self.set_wz(hl.wrapping_add(1));
        self.set_hl(result);
        self.q = !self.q;
    }

    fn execute_block_io_base<B: Bus>(
        &mut self,
        bus: &mut B,
        is_input: bool,
        is_increment: bool,
    ) -> u8 {
        let hl = self.hl();

        if is_input {
            let port = self.bc();
            let val = bus.port_read(port);
            bus.write(hl, val);

            if is_increment {
                self.set_hl(hl.wrapping_add(1));
                self.set_wz(self.bc().wrapping_add(1));
            } else {
                self.set_hl(hl.wrapping_sub(1));
                self.set_wz(self.bc().wrapping_sub(1));
            }
            self.b = self.b.wrapping_sub(1);
            val
        } else {
            let val = bus.read(hl);

            if is_increment {
                self.set_hl(hl.wrapping_add(1));
            } else {
                self.set_hl(hl.wrapping_sub(1));
            }

            self.b = self.b.wrapping_sub(1);
            let port = self.bc();

            if is_increment {
                self.set_wz(port.wrapping_add(1));
            } else {
                self.set_wz(port.wrapping_sub(1));
            }

            bus.port_write(port, val);
            val
        }
    }

    fn handle_block_repeat(&mut self, is_repeating: bool, base_f: u8) -> u64 {
        self.f = base_f;
        self.q = !self.q;

        if is_repeating && self.b != 0 {
            self.pc = self.pc.wrapping_sub(2);
            self.set_wz(self.pc.wrapping_add(1));

            let x_bit = ((self.pc >> 11) & 1) as u8;
            let y_bit = ((self.pc >> 13) & 1) as u8;
            self.f |= (x_bit << 3) | (y_bit << 5);

            21
        } else {
            self.f |= self.b & 0x28;

            16
        }
    }

    fn post_in_o_r(
        &self,
        c_flag: bool,
        value: u8,
        mut h_flag: bool,
        mut pv_flag: bool,
    ) -> (bool, bool) {
        if self.b != 0 {
            if c_flag {
                if (value & 0x80) != 0 {
                    pv_flag ^= Self::parity(self.b.wrapping_sub(1) & 7) ^ true;
                    h_flag = (self.b & 0x0F) == 0;
                } else {
                    pv_flag ^= Self::parity(self.b.wrapping_add(1) & 7) ^ true;
                    h_flag = (self.b & 0x0F) == 0x0F;
                }
            } else {
                pv_flag ^= Self::parity(self.b & 7) ^ true;
            }
        }
        (h_flag, pv_flag)
    }

    fn execute_block_cp_base<B: Bus>(&mut self, bus: &mut B, is_increment: bool) -> (u8, bool) {
        let hl = self.hl();
        let data = bus.read(hl);

        if is_increment {
            self.set_hl(hl.wrapping_add(1));
            self.set_wz(self.wz().wrapping_add(1));
        } else {
            self.set_hl(hl.wrapping_sub(1));
            self.set_wz(self.wz().wrapping_sub(1));
        }

        let n = self.a.wrapping_sub(data);
        let bc = self.bc().wrapping_sub(1);
        self.set_bc(bc);

        let h_flag = ((self.a ^ data ^ n) & 0x10) != 0;
        let h_subtractor = if h_flag { 1 } else { 0 };
        let look = n.wrapping_sub(h_subtractor);

        let sign_bit = (n & 0x80) != 0;
        let zero_bit = n == 0;
        let pv_bit = bc != 0;

        self.f = (self.f & FLAG_CARRY)
            | if sign_bit { FLAG_SIGN } else { 0 }
            | if zero_bit { FLAG_ZERO } else { 0 }
            | if h_flag { FLAG_HALF_CARRY } else { 0 }
            | if pv_bit { FLAG_PARITY } else { 0 }
            | FLAG_ADD_OR_SUBTRACT
            | (look & 0x08)
            | ((look & 0x02) << 4);

        (n, zero_bit)
    }

    fn handle_block_cp_repeat(&mut self, is_repeating: bool, zero_bit: bool) -> u64 {
        let bc = self.bc();
        self.q = !self.q;

        if is_repeating && bc != 0 && !zero_bit {
            self.pc = self.pc.wrapping_sub(2);
            self.set_wz(self.pc.wrapping_add(1));

            let x_bit = ((self.pc >> 11) & 1) as u8;
            let y_bit = ((self.pc >> 13) & 1) as u8;

            self.f = (self.f & !0x28) | (x_bit << 3) | (y_bit << 5);
            21
        } else {
            16
        }
    }

    fn step_ed<B: Bus>(&mut self, bus: &mut B) -> u64 {
        let opcode = self.read_byte(bus);
        self.r = (((self.r & 0x7F) + 1) & 0x7F) | (self.r & 0x80);

        match opcode {
            0x40 | 0x48 | 0x50 | 0x58 | 0x60 | 0x68 | 0x70 | 0x78 => {
                // IN r,(C)
                let port = self.bc();
                let value = bus.port_read(port);
                self.set_wz(port.wrapping_add(1));
                let reg = (opcode >> 3) & 7;
                if reg != 6 {
                    self.set_reg(bus, reg, value);
                }
                self.f = if value == 0 { FLAG_ZERO } else { 0 }
                    | if value & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(value) { FLAG_PARITY } else { 0 }
                    | (value & 0x28)
                    | (self.f & FLAG_CARRY);
                self.q = !self.q;
                12
            }
            0x41 | 0x49 | 0x51 | 0x59 | 0x61 | 0x69 | 0x71 | 0x79 => {
                // OUT (C),r
                let port = self.bc();
                let reg = (opcode >> 3) & 7;
                let value = if reg == 6 { 0 } else { self.get_reg(bus, reg) };
                bus.port_write(port, value);
                self.set_wz(port.wrapping_add(1));
                12
            }
            0x42 | 0x52 | 0x62 | 0x72 => {
                // SBC HL, rp
                let rp = (opcode >> 4) & 3;
                let value = self.get_rp(rp);
                self.sbc_hl(value);
                15
            }
            0x4A | 0x5A | 0x6A | 0x7A => {
                // ADC HL, rp
                let rp = (opcode >> 4) & 3;
                let value = self.get_rp(rp);
                self.adc_hl(value);
                15
            }
            0x43 | 0x53 | 0x63 | 0x73 => {
                // LD (nn), rp
                let addr = self.read_word(bus);
                let rp = (opcode >> 4) & 3;
                let value = self.get_rp(rp);
                bus.write(addr, value as u8);
                bus.write(addr.wrapping_add(1), (value >> 8) as u8);
                self.set_wz(addr.wrapping_add(1));
                20
            }
            0x4B | 0x5B | 0x6B | 0x7B => {
                // LD rp, (nn)
                let addr = self.read_word(bus);
                let lo = bus.read(addr) as u16;
                let hi = bus.read(addr.wrapping_add(1)) as u16;
                let value = (hi << 8) | lo;
                let rp = (opcode >> 4) & 3;
                self.set_rp(rp, value);
                self.set_wz(addr.wrapping_add(1));
                20
            }
            0x44 | 0x4C | 0x54 | 0x5C | 0x64 | 0x6C | 0x74 | 0x7C => {
                // NEG
                let a = self.a;
                self.a = 0;
                self.sub_a(a, false, true);
                8
            }
            0x45 | 0x55 | 0x5D | 0x65 | 0x6D | 0x75 | 0x7D => {
                // RETN
                self.iff1 = self.iff2;
                let pc = self.pop_word(bus);
                self.pc = pc;
                self.set_wz(pc);
                14
            }
            0x4D => {
                // RETI
                self.iff1 = self.iff2;
                let pc = self.pop_word(bus);
                self.pc = pc;
                self.set_wz(pc);
                14
            }
            0x46 | 0x4E | 0x66 | 0x6E => {
                // IM 0
                self.im = InterruptMode::IM0;
                8
            }
            0x56 | 0x76 => {
                // IM 1
                self.im = InterruptMode::IM1;
                8
            }
            0x5E | 0x7E => {
                // IM 2
                self.im = InterruptMode::IM2;
                8
            }
            0x47 => {
                // LD I, A
                self.i = self.a;
                9
            }
            0x4F => {
                // LD R, A
                self.r = self.a;
                9
            }
            0x57 => {
                // LD A, I
                self.a = self.i;
                self.f = (self.f & FLAG_CARRY)
                    | if self.a == 0 { FLAG_ZERO } else { 0 }
                    | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if self.iff2 { FLAG_PARITY } else { 0 }
                    | (self.a & 0x28);
                self.q = !self.q;
                self.p = true;
                9
            }
            0x5F => {
                // LD A, R
                self.a = self.r;
                self.f = (self.f & FLAG_CARRY)
                    | if self.a == 0 { FLAG_ZERO } else { 0 }
                    | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if self.iff2 { FLAG_PARITY } else { 0 }
                    | (self.a & 0x28);
                self.q = !self.q;
                self.p = true;
                9
            }
            0x67 => {
                // RRD
                let addr = self.hl();
                let value = bus.read(addr);
                let new_a = (self.a & 0xF0) | (value & 0x0F);
                let new_mem = ((value >> 4) & 0x0F) | ((self.a & 0x0F) << 4);
                self.a = new_a;
                bus.write(addr, new_mem);
                self.f = if self.a == 0 { FLAG_ZERO } else { 0 }
                    | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
                    | (self.a & 0x28)
                    | (self.f & FLAG_CARRY);
                self.set_wz(self.hl().wrapping_add(1));
                self.q = !self.q;
                18
            }
            0x6F => {
                // RLD
                let addr = self.hl();
                let value = bus.read(addr);
                let new_a = (self.a & 0xF0) | ((value >> 4) & 0x0F);
                let new_mem = ((value & 0x0F) << 4) | (self.a & 0x0F);
                self.a = new_a;
                bus.write(addr, new_mem);
                self.f = if self.a == 0 { FLAG_ZERO } else { 0 }
                    | if self.a & 0x80 != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(self.a) { FLAG_PARITY } else { 0 }
                    | (self.a & 0x28)
                    | (self.f & FLAG_CARRY);
                self.set_wz(self.hl().wrapping_add(1));
                self.q = !self.q;
                18
            }
            0x77 | 0x7F => {
                // NOP (undocumented)
                8
            }
            0xA0 => {
                // LDI
                let hl = self.hl();
                let de = self.de();
                let bc = self.bc();
                let value = bus.read(hl);
                bus.write(de, value);
                self.set_hl(hl.wrapping_add(1));
                self.set_de(de.wrapping_add(1));
                self.set_bc(bc.wrapping_sub(1));
                let n = value.wrapping_add(self.a);
                self.f = (self.f & (FLAG_CARRY | FLAG_ZERO | FLAG_SIGN))
                    | if bc.wrapping_sub(1) == 0 {
                        0
                    } else {
                        FLAG_PARITY
                    }
                    | (n & 0x08)
                    | ((n & 0x02) << 4);
                self.q = !self.q;
                16
            }
            0xA8 => {
                // LDD
                let hl = self.hl();
                let de = self.de();
                let bc = self.bc();
                let value = bus.read(hl);
                bus.write(de, value);
                self.set_hl(hl.wrapping_sub(1));
                self.set_de(de.wrapping_sub(1));
                self.set_bc(bc.wrapping_sub(1));
                let n = value.wrapping_add(self.a);
                self.f = (self.f & (FLAG_CARRY | FLAG_ZERO | FLAG_SIGN))
                    | if bc.wrapping_sub(1) == 0 {
                        0
                    } else {
                        FLAG_PARITY
                    }
                    | (n & 0x08)
                    | ((n & 0x02) << 4);
                self.q = !self.q;
                16
            }
            0xB0 => {
                // LDIR
                let hl = self.hl();
                let de = self.de();
                let bc = self.bc();
                let value = bus.read(hl);
                bus.write(de, value);
                self.set_hl(hl.wrapping_add(1));
                self.set_de(de.wrapping_add(1));
                let new_bc = bc.wrapping_sub(1);
                self.set_bc(new_bc);
                let n = value.wrapping_add(self.a);
                self.f &= FLAG_CARRY | FLAG_ZERO | FLAG_SIGN;
                self.q = !self.q;

                if new_bc != 0 {
                    self.pc = self.pc.wrapping_sub(2);
                    self.set_wz(self.pc.wrapping_add(1));
                    self.f |= (self.pc >> 8) as u8 & 0x28;
                    self.f |= FLAG_PARITY;
                    21
                } else {
                    self.set_wz(de.wrapping_add(1));
                    self.f |= (n & 0x08) | ((n & 0x02) << 4);
                    16
                }
            }
            0xB8 => {
                // LDDR
                let hl = self.hl();
                let de = self.de();
                let bc = self.bc();
                let value = bus.read(hl);
                bus.write(de, value);
                self.set_hl(hl.wrapping_sub(1));
                self.set_de(de.wrapping_sub(1));
                let new_bc = bc.wrapping_sub(1);
                self.set_bc(new_bc);
                let n = value.wrapping_add(self.a);
                self.f &= FLAG_CARRY | FLAG_ZERO | FLAG_SIGN;
                self.q = !self.q;

                if new_bc != 0 {
                    self.pc = self.pc.wrapping_sub(2);
                    self.set_wz(self.pc.wrapping_add(1));
                    self.f |= (self.pc >> 8) as u8 & 0x28;
                    self.f |= FLAG_PARITY;
                    21
                } else {
                    self.set_wz(de.wrapping_sub(1));
                    self.f |= (n & 0x08) | ((n & 0x02) << 4);
                    16
                }
            }
            0xA1 => {
                // CPI
                let (_, zero_bit) = self.execute_block_cp_base(bus, true);
                self.handle_block_cp_repeat(false, zero_bit)
            }
            0xA9 => {
                // CPD
                let (_, zero_bit) = self.execute_block_cp_base(bus, false);
                self.handle_block_cp_repeat(false, zero_bit)
            }
            0xB1 => {
                // CPIR
                let (_, zero_bit) = self.execute_block_cp_base(bus, true);
                self.handle_block_cp_repeat(true, zero_bit)
            }
            0xB9 => {
                // CPDR
                let (_, zero_bit) = self.execute_block_cp_base(bus, false);
                self.handle_block_cp_repeat(true, zero_bit)
            }
            0xA2 => {
                // INI
                let value = self.execute_block_io_base(bus, true, true);

                let modified_c = (self.c.wrapping_add(1)) as u16;
                let k = modified_c.wrapping_add(value as u16);

                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let parity_byte = (((k & 7) as u8) ^ self.b) & 0xFF;

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(parity_byte) {
                        FLAG_PARITY
                    } else {
                        0
                    }
                    | if c_flag {
                        FLAG_CARRY | FLAG_HALF_CARRY
                    } else {
                        0
                    }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 };

                self.handle_block_repeat(false, base_f)
            }
            0xAA => {
                // IND
                let value = self.execute_block_io_base(bus, true, false);

                let modified_c = (self.c.wrapping_sub(1)) as u16;
                let k = modified_c.wrapping_add(value as u16);

                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let parity_byte = (((k & 7) as u8) ^ self.b) & 0xFF;

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(parity_byte) {
                        FLAG_PARITY
                    } else {
                        0
                    }
                    | if c_flag {
                        FLAG_CARRY | FLAG_HALF_CARRY
                    } else {
                        0
                    }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 };

                self.handle_block_repeat(false, base_f)
            }
            0xA3 => {
                // OUTI
                let value = self.execute_block_io_base(bus, false, true);

                let k = (value as u16).wrapping_add(self.l as u16);
                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(((k & 0x07) as u8) ^ self.b) {
                        FLAG_PARITY
                    } else {
                        0
                    }
                    | if c_flag {
                        FLAG_CARRY | FLAG_HALF_CARRY
                    } else {
                        0
                    }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 };

                self.handle_block_repeat(false, base_f)
            }
            0xAB => {
                // OUTD
                let value = self.execute_block_io_base(bus, false, false);

                let k = (value as u16).wrapping_add(self.l as u16);
                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;

                let parity_byte = (((k & 7) as u8) ^ self.b) & 0xFF;
                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if Self::parity(parity_byte) {
                        FLAG_PARITY
                    } else {
                        0
                    }
                    | if c_flag {
                        FLAG_CARRY | FLAG_HALF_CARRY
                    } else {
                        0
                    }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 };

                self.handle_block_repeat(false, base_f)
            }
            0xB2 => {
                // INIR
                let value = self.execute_block_io_base(bus, true, true);

                let modified_c = (self.c.wrapping_add(1)) as u16;
                let k = modified_c.wrapping_add(value as u16);

                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let pf_base = Self::parity((((k & 7) as u8) ^ self.b) & 0xFF);

                let (h_flag, pv_flag) = self.post_in_o_r(c_flag, value, c_flag, pf_base);

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if h_flag { FLAG_HALF_CARRY } else { 0 }
                    | if pv_flag { FLAG_PARITY } else { 0 }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 }
                    | if c_flag { FLAG_CARRY } else { 0 };

                self.handle_block_repeat(true, base_f)
            }
            0xBA => {
                // INDR
                let value = self.execute_block_io_base(bus, true, false);

                let modified_c = (self.c.wrapping_sub(1)) as u16;
                let k = modified_c.wrapping_add(value as u16);

                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let pf_base = Self::parity((((k & 7) as u8) ^ self.b) & 0xFF);

                let (h_flag, pv_flag) = self.post_in_o_r(c_flag, value, c_flag, pf_base);

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if h_flag { FLAG_HALF_CARRY } else { 0 }
                    | if pv_flag { FLAG_PARITY } else { 0 }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 }
                    | if c_flag { FLAG_CARRY } else { 0 };

                self.handle_block_repeat(true, base_f)
            }
            0xB3 => {
                // OTIR
                let value = self.execute_block_io_base(bus, false, true);

                let k = (value as u16).wrapping_add(self.l as u16);
                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let pf_base = Self::parity(((k & 0x07) as u8) ^ self.b);

                let (h_flag, pv_flag) = self.post_in_o_r(c_flag, value, c_flag, pf_base);

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if h_flag { FLAG_HALF_CARRY } else { 0 }
                    | if pv_flag { FLAG_PARITY } else { 0 }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 }
                    | if c_flag { FLAG_CARRY } else { 0 };

                self.handle_block_repeat(true, base_f)
            }
            0xBB => {
                // OTDR
                let value = self.execute_block_io_base(bus, false, false);

                let k = (value as u16).wrapping_add(self.l as u16);
                let c_flag = (k & 0x100) != 0;
                let n_flag = (value & 0x80) != 0;
                let pf_base = Self::parity((((k & 7) as u8) ^ self.b) & 0xFF);

                let (h_flag, pv_flag) = self.post_in_o_r(c_flag, value, c_flag, pf_base);

                let base_f = if self.b == 0 { FLAG_ZERO } else { 0 }
                    | if (self.b & 0x80) != 0 { FLAG_SIGN } else { 0 }
                    | if h_flag { FLAG_HALF_CARRY } else { 0 }
                    | if pv_flag { FLAG_PARITY } else { 0 }
                    | if n_flag { FLAG_ADD_OR_SUBTRACT } else { 0 }
                    | if c_flag { FLAG_CARRY } else { 0 };

                self.handle_block_repeat(true, base_f)
            }
            _ => panic!("Unexpected ED opcode {opcode:02X}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, path::Path};

    use serde::Deserialize;

    use crate::{
        bus::Bus,
        cpu::{InterruptMode, Z80},
    };

    #[derive(Default, PartialEq, Eq, Debug)]
    struct TestBus {
        memory: HashMap<u16, u8>,
        port_inputs: HashMap<u16, u8>,
    }

    impl Bus for TestBus {
        fn read(&self, addr: u16) -> u8 {
            *self.memory.get(&addr).unwrap_or(&0xFF)
        }

        fn write(&mut self, addr: u16, value: u8) {
            self.memory.insert(addr, value);
        }

        fn port_read(&self, port: u16) -> u8 {
            *self.port_inputs.get(&port).unwrap_or(&0xFF)
        }

        fn port_write(&mut self, port: u16, value: u8) {
            self.port_inputs.insert(port, value);
        }

        fn acknowledge_interrupt(&mut self) {}
    }

    #[test]
    fn test_nmi() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        cpu.sp = 0x2000;
        cpu.iff1 = true;
        cpu.iff2 = true;

        let r_before = cpu.r;
        cpu.request_nmi();

        let cycles = cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0066);
        assert_eq!(cpu.sp, 0x1FFE);
        assert_eq!(bus.read(0x1FFF), 0x10);
        assert_eq!(bus.read(0x1FFE), 0x00);
        assert_eq!(cpu.iff1, false); // NMI disables maskable interrupts
        assert_eq!(cpu.iff2, true); // but preserves IFF2
        assert_eq!(cpu.r, r_before.wrapping_add(1)); // R is increased by 1
        assert_eq!(cycles, 11);
    }

    #[test]
    fn nmi_calls_0x0066() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        bus.write(0x0066, 0xED);
        bus.write(0x0067, 0x45); // RETN
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0066);
    }

    #[test]
    fn nmi_clears_iff1_only() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.iff2 = true;
        bus.write(0x0066, 0xED);
        bus.write(0x0067, 0x45);
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, true);
    }

    #[test]
    fn retn_restores_iff1_from_iff2() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = false;
        cpu.iff2 = true;
        cpu.sp = 0x1000;
        bus.write(0x0FFE, 0x00);
        bus.write(0x0FFF, 0x05);
        bus.write(0x0000, 0xED);
        bus.write(0x0001, 0x45);
        cpu.execute(&mut bus);
        assert_eq!(cpu.iff1, true);
    }

    #[test]
    fn nmi_pushes_pc_to_stack() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.sp = 0x1000;
        cpu.pc = 0x0500;
        bus.write(0x0066, 0xED);
        bus.write(0x0067, 0x45);
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(bus.read(0x0FFE), 0x00);
        assert_eq!(bus.read(0x0FFF), 0x05);
        assert_eq!(cpu.sp, 0x0FFE);
    }

    #[test]
    fn nmi_costs_11_t_states() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        bus.write(0x0066, 0xED);
        bus.write(0x0067, 0x45);
        cpu.request_nmi();
        let cycles = cpu.execute(&mut bus);

        assert_eq!(cycles, 11);
    }

    #[test]
    fn nmi_increases_r_register() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.r = 0x00;
        bus.write(0x0066, 0xED);
        bus.write(0x0067, 0x45);
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(cpu.r & 0x7F, 0x01);
    }

    #[test]
    fn nmi_has_priority_over_int() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0066, 0xC9);
        bus.write(0x0038, 0xC9);
        cpu.request_nmi();
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0066);
    }

    #[test]
    fn nested_nmi_does_not_overwrite_iff2() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.iff2 = true;
        bus.write(0x0066, 0xFB); // EI
        bus.write(0x0067, 0x76); // HALT
        bus.write(0x0068, 0xED); // RETN
        bus.write(0x0069, 0x45);
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, true);
        cpu.execute(&mut bus); // EI
        cpu.execute(&mut bus); // HALT
        cpu.request_nmi();
        cpu.execute(&mut bus);
        assert_eq!(cpu.iff2, true);
    }

    #[test]
    fn int_im1_calls_0x0038() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
    }

    #[test]
    fn int_im1_ignores_data_bus() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0x00);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
    }

    #[test]
    fn int_im1_clears_both_iff() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, false);
    }

    #[test]
    fn int_im1_costs_13_t_states() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        let cycles = cpu.execute(&mut bus);

        assert_eq!(cycles, 13);
    }

    #[test]
    fn int_increases_r_register() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        cpu.r = 0x00;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.r & 0x7F, 0x01);
    }

    #[test]
    fn test_interrupt_im1() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        cpu.sp = 0x2000;
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.im = InterruptMode::IM1;

        let r_before = cpu.r;
        cpu.request_int(0xFF);

        let cycles = cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
        assert_eq!(cpu.sp, 0x1FFE);
        assert_eq!(bus.read(0x1FFF), 0x10);
        assert_eq!(bus.read(0x1FFE), 0x00);
        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, false);
        assert_eq!(cpu.r, r_before.wrapping_add(1));
        assert_eq!(cycles, 13);
    }

    #[test]
    fn int_im2_uses_vector_table() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM2;
        cpu.i = 0x80;
        bus.write(0x80FF, 0x00);
        bus.write(0x8100, 0x10);
        bus.write(0x1000, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x1000);
    }

    #[test]
    fn int_im2_does_not_mask_low_bit_of_data_bus() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM2;
        cpu.i = 0x80;
        bus.write(0x8001, 0x00);
        bus.write(0x8002, 0x20);
        bus.write(0x2000, 0xC9);
        cpu.request_int(0x01);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x2000);
    }

    #[test]
    fn int_im2_costs_19_t_states() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM2;
        cpu.i = 0x80;
        bus.write(0x80FF, 0x00);
        bus.write(0x8000, 0x10);
        bus.write(0x1000, 0xC9);
        cpu.request_int(0xFF);
        let cycles = cpu.execute(&mut bus);

        assert_eq!(cycles, 19);
    }

    #[test]
    fn int_blocked_when_iff1_clear() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = false;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0x00);
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0001);
    }

    #[test]
    fn int_not_accepted_directly_after_ei() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = false;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0xFB); // EI
        bus.write(0x0001, 0x00); // NOP
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus); // EI
        assert_eq!(cpu.iff1, true);
        cpu.execute(&mut bus); // NOP - interrupt accepted after this
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
    }

    #[test]
    fn int_not_accepted_directly_after_di() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0xF3); // DI
        bus.write(0x0001, 0x00); // NOP
        bus.write(0x0038, 0xC9);
        cpu.execute(&mut bus); // DI
        assert_eq!(cpu.iff1, false);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus); // NOP

        assert_eq!(cpu.pc, 0x0002);
    }

    #[test]
    fn test_interrupt_im2() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        cpu.sp = 0x2000;
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.im = InterruptMode::IM2;
        cpu.i = 0x01;

        // Setup the vector table at 0x01AA
        bus.write(0x01AA, 0x34);
        bus.write(0x01AB, 0x12);

        let r_before = cpu.r;
        cpu.request_int(0xAA);

        let cycles = cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(cpu.sp, 0x1FFE);
        assert_eq!(bus.read(0x1FFF), 0x10);
        assert_eq!(bus.read(0x1FFE), 0x00);
        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, false);
        assert_eq!(cpu.r, r_before.wrapping_add(1));
        assert_eq!(cycles, 19);
    }

    #[test]
    fn int_im0_executes_instruction_on_bus() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM0;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
    }

    #[test]
    fn int_im0_costs_13_t_states_for_rst() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM0;
        bus.write(0x0038, 0xC9);
        cpu.request_int(0xFF);
        let cycles = cpu.execute(&mut bus);

        assert_eq!(cycles, 13);
    }

    #[test]
    fn test_interrupt_im0() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        cpu.sp = 0x2000;
        cpu.iff1 = true;
        cpu.iff2 = true;
        cpu.im = InterruptMode::IM0;

        let r_before = cpu.r;
        cpu.request_int(0xD7); // RST 10h

        let cycles = cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0010);
        assert_eq!(cpu.sp, 0x1FFE);
        assert_eq!(bus.read(0x1FFF), 0x10);
        assert_eq!(bus.read(0x1FFE), 0x00);
        assert_eq!(cpu.iff1, false);
        assert_eq!(cpu.iff2, false);
        assert_eq!(cpu.r, r_before.wrapping_add(1));
        assert_eq!(cycles, 13);
    }

    #[test]
    fn halt_does_not_advance_pc() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        bus.write(0x0000, 0x76);
        cpu.execute(&mut bus);
        assert_eq!(cpu.pc, 0x0001);
    }

    #[test]
    fn halt_sets_halt_line() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        bus.write(0x0000, 0x76);
        cpu.execute(&mut bus);
        assert!(cpu.halted);
    }

    #[test]
    fn halt_exits_on_nmi() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        bus.write(0x0000, 0x76);
        bus.write(0x0066, 0xC9);
        cpu.execute(&mut bus);
        cpu.request_nmi();
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0066);
        cpu.execute(&mut bus); // RET
        assert_eq!(cpu.pc, 0x0001);
    }

    #[test]
    fn halt_exits_on_int_when_enabled() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0x76);
        bus.write(0x0038, 0xC9);
        cpu.execute(&mut bus);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0038);
    }

    #[test]
    fn halt_does_not_exit_on_int_when_disabled() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = false;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0x76);
        bus.write(0x0038, 0xC9);
        cpu.execute(&mut bus);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert_eq!(cpu.pc, 0x0001);
    }

    #[test]
    fn halt_clears_halt_line_on_interrupt() {
        let mut bus = TestBus::default();
        let mut cpu = Z80::new();
        cpu.iff1 = true;
        cpu.im = InterruptMode::IM1;
        bus.write(0x0000, 0x76);
        bus.write(0x0038, 0xC9);
        cpu.execute(&mut bus);
        assert!(cpu.halted);
        cpu.request_int(0xFF);
        cpu.execute(&mut bus);

        assert!(!cpu.halted);
    }

    #[test]
    fn test_halt_and_interrupt() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        bus.write(0x1000, 0x76); // HALT

        let cycles = cpu.execute(&mut bus);
        assert_eq!(cycles, 4);
        assert!(cpu.is_halted());
        assert_eq!(cpu.pc, 0x1001);

        let cycles2 = cpu.execute(&mut bus);
        assert_eq!(cycles2, 4);
        assert!(cpu.is_halted());
        assert_eq!(cpu.pc, 0x1001);

        cpu.request_nmi();
        let cycles3 = cpu.execute(&mut bus);
        assert_eq!(cycles3, 11);
        assert_eq!(cpu.pc, 0x0066);
        assert!(!cpu.is_halted());
    }

    #[test]
    fn test_ei_delays_interrupt() {
        let mut cpu = Z80::new();
        let mut bus = TestBus::default();

        cpu.pc = 0x1000;
        bus.write(0x1000, 0xFB); // EI
        bus.write(0x1001, 0x00); // NOP

        cpu.iff1 = false;
        cpu.iff2 = false;

        cpu.execute(&mut bus); // Execute EI
        assert_eq!(cpu.pc, 0x1001);
        assert_eq!(cpu.iff1, true);

        cpu.request_int(0xFF);
        cpu.im = InterruptMode::IM1;

        let cycles = cpu.execute(&mut bus); // Should execute NOP, ignoring INT for 1 instruction
        assert_eq!(cycles, 4);
        assert_eq!(cpu.pc, 0x1002);

        let cycles_int = cpu.execute(&mut bus); // Now accept INT
        assert_eq!(cycles_int, 13);
        assert_eq!(cpu.pc, 0x0038);
    }

    #[derive(Deserialize)]
    struct TestCaseState {
        pc: u16,
        sp: u16,
        a: u8,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        f: u8,
        h: u8,
        l: u8,
        i: u8,
        r: u8,
        ei: u8,
        wz: u16,
        ix: u16,
        iy: u16,
        af_: u16,
        bc_: u16,
        de_: u16,
        hl_: u16,
        im: u8,
        p: u8,
        q: u8,
        iff1: u8,
        iff2: u8,
        ram: Vec<(u16, u8)>,
    }

    impl TestCaseState {
        fn create_cpu(&self) -> Z80 {
            let mut cpu = Z80::default();

            cpu.a = self.a;
            cpu.b = self.b;
            cpu.c = self.c;
            cpu.d = self.d;
            cpu.e = self.e;
            cpu.f = self.f;
            cpu.h = self.h;
            cpu.l = self.l;
            cpu.i = self.i;
            cpu.r = self.r;
            cpu.p = match self.p {
                0 => false,
                1 => true,
                _ => panic!("Unexpected P value {}", self.ei),
            };
            cpu.w = (self.wz >> 8) as u8;
            cpu.z = (self.wz & 0xFF) as u8;
            cpu.q = self.q;
            cpu.ix = self.ix;
            cpu.iy = self.iy;
            cpu.a_shadow = (self.af_ >> 8) as u8;
            cpu.f_shadow = (self.af_ & 0xFF) as u8;
            cpu.b_shadow = (self.bc_ >> 8) as u8;
            cpu.c_shadow = (self.bc_ & 0xFF) as u8;
            cpu.d_shadow = (self.de_ >> 8) as u8;
            cpu.e_shadow = (self.de_ & 0xFF) as u8;
            cpu.h_shadow = (self.hl_ >> 8) as u8;
            cpu.l_shadow = (self.hl_ & 0xFF) as u8;
            cpu.pc = self.pc;
            cpu.sp = self.sp;
            cpu.ei = match self.ei {
                0 => false,
                1 => true,
                _ => panic!("Unexpected EI value {}", self.ei),
            };
            cpu.im = match self.im {
                0 => super::InterruptMode::IM0,
                1 => super::InterruptMode::IM1,
                2 => super::InterruptMode::IM2,
                _ => panic!("Unexpected interrupt mode {}", self.im),
            };
            cpu.iff1 = match self.iff1 {
                0 => false,
                1 => true,
                _ => panic!("Unexpected IFF1 value {}", self.iff1),
            };
            cpu.iff2 = match self.iff2 {
                0 => false,
                1 => true,
                _ => panic!("Unexpected IFF2 value {}", self.iff2),
            };

            cpu
        }

        fn create_memory(&self, ports: Vec<(u16, u8, String)>) -> TestBus {
            let mut bus = TestBus::default();

            for (addr, value) in self.ram.iter() {
                bus.write(*addr, *value);
            }

            for (port, value, _) in ports.iter() {
                bus.port_write(*port, *value);
            }

            bus
        }
    }

    #[derive(Deserialize)]
    struct TestCase {
        name: String,
        #[serde(rename = "initial")]
        initial_state: TestCaseState,
        #[serde(rename = "final")]
        final_state: TestCaseState,
        cycles: Vec<(u16, Option<u8>, String)>,
        ports: Option<Vec<(u16, u8, String)>>,
    }

    macro_rules! z80_tests {
        ($($name:ident => $file:expr),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    execute_test_cases($file);
                }
            )*
        };
    }

    fn execute_test_cases(test_file_name: &str) {
        let test_path = "tests/z80/v1";
        let full_test_path = Path::new(test_path).join(test_file_name);
        let test_cases_string = fs::read_to_string(&full_test_path)
            .expect(&format!("Failed to read test file: {full_test_path:?}"));
        let test_cases: Vec<TestCase> =
            serde_json::from_str(&test_cases_string).expect("Test case file failed to deserialize");

        for case in test_cases {
            let initial_ports_state = case.ports.clone().map_or(Vec::default(), |ports| {
                ports
                    .iter()
                    .filter(|(_, _, op)| op == "r")
                    .cloned()
                    .collect()
            });
            let mut bus = case.initial_state.create_memory(initial_ports_state);
            let mut cpu = case.initial_state.create_cpu();

            let cycles = cpu.execute(&mut bus);

            let mut expected_cpu = case.final_state.create_cpu();
            expected_cpu.halted = cpu.halted; // This test doesn't care about HALTED boolean
            assert_eq!(
                case.cycles.len() as u64,
                cycles,
                "Test Case '{}': CPU cycles produced doesn't match with the test case",
                case.name
            );
            assert_eq!(
                case.final_state
                    .create_memory(case.ports.unwrap_or_default()),
                bus,
                "Test Case '{}': RAM and PORTS state doesn't match the expected final state",
                case.name
            );
            assert_eq!(
                expected_cpu, cpu,
                "Test Case '{}': CPU state doesn't match the expected final state",
                case.name
            );
        }
    }

    z80_tests! {
         test_nop      => "00.json",
         test_ld_bc_nn => "01.json",
         test_ld_bcp_a => "02.json",
         test_inc_bc   => "03.json",
         test_inc_b    => "04.json",
         test_dec_b    => "05.json",
         test_ld_b_n   => "06.json",
         test_rlca     => "07.json",
         test_ex_af_af => "08.json",
         test_add_hl_bc=> "09.json",
         test_ld_a_bcp => "0a.json",
         test_dec_bc   => "0b.json",
         test_inc_c    => "0c.json",
         test_dec_c    => "0d.json",
         test_ld_c_n   => "0e.json",
         test_rrca     => "0f.json",
         test_djnz_d   => "10.json",
         test_ld_de_nn => "11.json",
         test_ld_dep_a => "12.json",
         test_inc_de   => "13.json",
         test_inc_d    => "14.json",
         test_dec_d    => "15.json",
         test_ld_d_n   => "16.json",
         test_rla      => "17.json",
         test_jr_d     => "18.json",
         test_add_hl_de=> "19.json",
         test_ld_a_dep => "1a.json",
         test_dec_de   => "1b.json",
         test_inc_e    => "1c.json",
         test_dec_e    => "1d.json",
         test_ld_e_n   => "1e.json",
         test_rra      => "1f.json",
         test_jr_nz_d  => "20.json",
         test_ld_hl_nn => "21.json",
         test_ld_nnp_hl=> "22.json",
         test_inc_hl   => "23.json",
         test_inc_h    => "24.json",
         test_dec_h    => "25.json",
         test_ld_h_n   => "26.json",
         test_daa      => "27.json",
         test_jr_z_d   => "28.json",
         test_add_hl_hl=> "29.json",
         test_ld_hl_nnp=> "2a.json",
         test_dec_hl   => "2b.json",
         test_inc_l    => "2c.json",
         test_dec_l    => "2d.json",
         test_ld_l_n   => "2e.json",
         test_cpl      => "2f.json",
         test_jr_nc_d  => "30.json",
         test_ld_sp_nn => "31.json",
         test_ld_nnp_a => "32.json",
         test_inc_sp   => "33.json",
         test_inc_hlp  => "34.json",
         test_dec_hlp  => "35.json",
         test_ld_hlp_n => "36.json",
         test_scf      => "37.json",
         test_jr_c_d   => "38.json",
         test_add_hl_sp=> "39.json",
         test_ld_a_nnp => "3a.json",
         test_dec_sp   => "3b.json",
         test_inc_a    => "3c.json",
         test_dec_a    => "3d.json",
         test_ld_a_n   => "3e.json",
         test_ccf      => "3f.json",
         test_ld_b_b   => "40.json",
         test_ld_b_c   => "41.json",
         test_ld_b_d   => "42.json",
         test_ld_b_e   => "43.json",
         test_ld_b_h   => "44.json",
         test_ld_b_l   => "45.json",
         test_ld_b_hlp => "46.json",
         test_ld_b_a   => "47.json",
         test_ld_c_b   => "48.json",
         test_ld_c_c   => "49.json",
         test_ld_c_d   => "4a.json",
         test_ld_c_e   => "4b.json",
         test_ld_c_h   => "4c.json",
         test_ld_c_l   => "4d.json",
         test_ld_c_hlp => "4e.json",
         test_ld_c_a   => "4f.json",
         test_ld_d_b   => "50.json",
         test_ld_d_c   => "51.json",
         test_ld_d_d   => "52.json",
         test_ld_d_e   => "53.json",
         test_ld_d_h   => "54.json",
         test_ld_d_l   => "55.json",
         test_ld_d_hlp => "56.json",
         test_ld_d_a   => "57.json",
         test_ld_e_b   => "58.json",
         test_ld_e_c   => "59.json",
         test_ld_e_d   => "5a.json",
         test_ld_e_e   => "5b.json",
         test_ld_e_h   => "5c.json",
         test_ld_e_l   => "5d.json",
         test_ld_e_hlp => "5e.json",
         test_ld_e_a   => "5f.json",
         test_ld_h_b   => "60.json",
         test_ld_h_c   => "61.json",
         test_ld_h_d   => "62.json",
         test_ld_h_e   => "63.json",
         test_ld_h_h   => "64.json",
         test_ld_h_l   => "65.json",
         test_ld_h_hlp => "66.json",
         test_ld_h_a   => "67.json",
         test_ld_l_b   => "68.json",
         test_ld_l_c   => "69.json",
         test_ld_l_d   => "6a.json",
         test_ld_l_e   => "6b.json",
         test_ld_l_h   => "6c.json",
         test_ld_l_l   => "6d.json",
         test_ld_l_hlp => "6e.json",
         test_ld_l_a   => "6f.json",
         test_ld_hlp_b => "70.json",
         test_ld_hlp_c => "71.json",
         test_ld_hlp_d => "72.json",
         test_ld_hlp_e => "73.json",
         test_ld_hlp_h => "74.json",
         test_ld_hlp_l => "75.json",
         test_halt     => "76.json",
         test_ld_hlp_a => "77.json",
         test_ld_a_b   => "78.json",
         test_ld_a_c   => "79.json",
         test_ld_a_d   => "7a.json",
         test_ld_a_e   => "7b.json",
         test_ld_a_h   => "7c.json",
         test_ld_a_l   => "7d.json",
         test_ld_a_hlp => "7e.json",
         test_ld_a_a   => "7f.json",
         test_add_a_b   => "80.json",
         test_add_a_c   => "81.json",
         test_add_a_d   => "82.json",
         test_add_a_e   => "83.json",
         test_add_a_h   => "84.json",
         test_add_a_l   => "85.json",
         test_add_a_hlp => "86.json",
         test_add_a_a   => "87.json",
         test_adc_a_b   => "88.json",
         test_adc_a_c   => "89.json",
         test_adc_a_d   => "8a.json",
         test_adc_a_e   => "8b.json",
         test_adc_a_h   => "8c.json",
         test_adc_a_l   => "8d.json",
         test_adc_a_hlp => "8e.json",
         test_adc_a_a   => "8f.json",
         test_sub_b     => "90.json",
         test_sub_c     => "91.json",
         test_sub_d     => "92.json",
         test_sub_e     => "93.json",
         test_sub_h     => "94.json",
         test_sub_l     => "95.json",
         test_sub_hlp   => "96.json",
         test_sub_a     => "97.json",
         test_sbc_a_b   => "98.json",
         test_sbc_a_c   => "99.json",
         test_sbc_a_d   => "9a.json",
         test_sbc_a_e   => "9b.json",
         test_sbc_a_h   => "9c.json",
         test_sbc_a_l   => "9d.json",
         test_sbc_a_hlp => "9e.json",
         test_sbc_a_a   => "9f.json",
         test_and_b     => "a0.json",
         test_and_c     => "a1.json",
         test_and_d     => "a2.json",
         test_and_e     => "a3.json",
         test_and_h     => "a4.json",
         test_and_l     => "a5.json",
         test_and_hlp   => "a6.json",
         test_and_a     => "a7.json",
         test_xor_b     => "a8.json",
         test_xor_c     => "a9.json",
         test_xor_d     => "aa.json",
         test_xor_e     => "ab.json",
         test_xor_h     => "ac.json",
         test_xor_l     => "ad.json",
         test_xor_hlp   => "ae.json",
         test_xor_a     => "af.json",
         test_or_b      => "b0.json",
         test_or_c      => "b1.json",
         test_or_d      => "b2.json",
         test_or_e      => "b3.json",
         test_or_h      => "b4.json",
         test_or_l      => "b5.json",
         test_or_hlp    => "b6.json",
         test_or_a      => "b7.json",
         test_cp_b      => "b8.json",
         test_cp_c      => "b9.json",
         test_cp_d      => "ba.json",
         test_cp_e      => "bb.json",
         test_cp_h      => "bc.json",
         test_cp_l      => "bd.json",
         test_cp_hlp    => "be.json",
         test_cp_a      => "bf.json",
         test_ret_nz     => "c0.json",
         test_pop_bc     => "c1.json",
         test_jp_nz_nn   => "c2.json",
         test_jp_nn      => "c3.json",
         test_call_nz_nn => "c4.json",
         test_push_bc    => "c5.json",
         test_add_a_n    => "c6.json",
         test_rst_00     => "c7.json",
         test_ret_z      => "c8.json",
         test_ret        => "c9.json",
         test_jp_z_nn    => "ca.json",
         test_call_z_nn  => "cc.json",
         test_call_nn    => "cd.json",
         test_adc_a_n    => "ce.json",
         test_rst_08     => "cf.json",
         test_ret_nc     => "d0.json",
         test_pop_de     => "d1.json",
         test_jp_nc_nn   => "d2.json",
         test_out_n_a    => "d3.json",
         test_call_nc_nn => "d4.json",
         test_push_de    => "d5.json",
         test_sub_n      => "d6.json",
         test_rst_10     => "d7.json",
         test_ret_c      => "d8.json",
         test_exx        => "d9.json",
         test_jp_c_nn    => "da.json",
         test_in_a_n     => "db.json",
         test_call_c_nn  => "dc.json",
         test_sbc_a_n    => "de.json",
         test_rst_18     => "df.json",
         test_ret_po     => "e0.json",
         test_pop_hl     => "e1.json",
         test_jp_po_nn   => "e2.json",
         test_ex_spp_hl  => "e3.json",
         test_call_po_nn => "e4.json",
         test_push_hl    => "e5.json",
         test_and_n      => "e6.json",
         test_rst_20     => "e7.json",
         test_ret_pe     => "e8.json",
         test_jp_hlp     => "e9.json",
         test_jp_pe_nn   => "ea.json",
         test_ex_de_hl   => "eb.json",
         test_call_pe_nn => "ec.json",
         test_xor_n      => "ee.json",
         test_rst_28     => "ef.json",
         test_ret_p      => "f0.json",
         test_pop_af     => "f1.json",
         test_jp_p_nn    => "f2.json",
         test_di         => "f3.json",
         test_call_p_nn  => "f4.json",
         test_push_af    => "f5.json",
         test_or_n       => "f6.json",
         test_rst_30     => "f7.json",
         test_ret_m      => "f8.json",
         test_ld_sp_hl   => "f9.json",
         test_jp_m_nn    => "fa.json",
         test_ei         => "fb.json",
         test_call_m_nn  => "fc.json",
         test_cp_n       => "fe.json",
         test_rst_38     => "ff.json",
    }

    z80_tests! {
        test_cb_00 => "cb 00.json", test_cb_01 => "cb 01.json", test_cb_02 => "cb 02.json", test_cb_03 => "cb 03.json",
        test_cb_04 => "cb 04.json", test_cb_05 => "cb 05.json", test_cb_06 => "cb 06.json", test_cb_07 => "cb 07.json",
        test_cb_08 => "cb 08.json", test_cb_09 => "cb 09.json", test_cb_0a => "cb 0a.json", test_cb_0b => "cb 0b.json",
        test_cb_0c => "cb 0c.json", test_cb_0d => "cb 0d.json", test_cb_0e => "cb 0e.json", test_cb_0f => "cb 0f.json",
        test_cb_10 => "cb 10.json", test_cb_11 => "cb 11.json", test_cb_12 => "cb 12.json", test_cb_13 => "cb 13.json",
        test_cb_14 => "cb 14.json", test_cb_15 => "cb 15.json", test_cb_16 => "cb 16.json", test_cb_17 => "cb 17.json",
        test_cb_18 => "cb 18.json", test_cb_19 => "cb 19.json", test_cb_1a => "cb 1a.json", test_cb_1b => "cb 1b.json",
        test_cb_1c => "cb 1c.json", test_cb_1d => "cb 1d.json", test_cb_1e => "cb 1e.json", test_cb_1f => "cb 1f.json",
        test_cb_20 => "cb 20.json", test_cb_21 => "cb 21.json", test_cb_22 => "cb 22.json", test_cb_23 => "cb 23.json",
        test_cb_24 => "cb 24.json", test_cb_25 => "cb 25.json", test_cb_26 => "cb 26.json", test_cb_27 => "cb 27.json",
        test_cb_28 => "cb 28.json", test_cb_29 => "cb 29.json", test_cb_2a => "cb 2a.json", test_cb_2b => "cb 2b.json",
        test_cb_2c => "cb 2c.json", test_cb_2d => "cb 2d.json", test_cb_2e => "cb 2e.json", test_cb_2f => "cb 2f.json",
        test_cb_30 => "cb 30.json", test_cb_31 => "cb 31.json", test_cb_32 => "cb 32.json", test_cb_33 => "cb 33.json",
        test_cb_34 => "cb 34.json", test_cb_35 => "cb 35.json", test_cb_36 => "cb 36.json", test_cb_37 => "cb 37.json",
        test_cb_38 => "cb 38.json", test_cb_39 => "cb 39.json", test_cb_3a => "cb 3a.json", test_cb_3b => "cb 3b.json",
        test_cb_3c => "cb 3c.json", test_cb_3d => "cb 3d.json", test_cb_3e => "cb 3e.json", test_cb_3f => "cb 3f.json",
        test_cb_40 => "cb 40.json", test_cb_41 => "cb 41.json", test_cb_42 => "cb 42.json", test_cb_43 => "cb 43.json",
        test_cb_44 => "cb 44.json", test_cb_45 => "cb 45.json", test_cb_46 => "cb 46.json", test_cb_47 => "cb 47.json",
        test_cb_48 => "cb 48.json", test_cb_49 => "cb 49.json", test_cb_4a => "cb 4a.json", test_cb_4b => "cb 4b.json",
        test_cb_4c => "cb 4c.json", test_cb_4d => "cb 4d.json", test_cb_4e => "cb 4e.json", test_cb_4f => "cb 4f.json",
        test_cb_50 => "cb 50.json", test_cb_51 => "cb 51.json", test_cb_52 => "cb 52.json", test_cb_53 => "cb 53.json",
        test_cb_54 => "cb 54.json", test_cb_55 => "cb 55.json", test_cb_56 => "cb 56.json", test_cb_57 => "cb 57.json",
        test_cb_58 => "cb 58.json", test_cb_59 => "cb 59.json", test_cb_5a => "cb 5a.json", test_cb_5b => "cb 5b.json",
        test_cb_5c => "cb 5c.json", test_cb_5d => "cb 5d.json", test_cb_5e => "cb 5e.json", test_cb_5f => "cb 5f.json",
        test_cb_60 => "cb 60.json", test_cb_61 => "cb 61.json", test_cb_62 => "cb 62.json", test_cb_63 => "cb 63.json",
        test_cb_64 => "cb 64.json", test_cb_65 => "cb 65.json", test_cb_66 => "cb 66.json", test_cb_67 => "cb 67.json",
        test_cb_68 => "cb 68.json", test_cb_69 => "cb 69.json", test_cb_6a => "cb 6a.json", test_cb_6b => "cb 6b.json",
        test_cb_6c => "cb 6c.json", test_cb_6d => "cb 6d.json", test_cb_6e => "cb 6e.json", test_cb_6f => "cb 6f.json",
        test_cb_70 => "cb 70.json", test_cb_71 => "cb 71.json", test_cb_72 => "cb 72.json", test_cb_73 => "cb 73.json",
        test_cb_74 => "cb 74.json", test_cb_75 => "cb 75.json", test_cb_76 => "cb 76.json", test_cb_77 => "cb 77.json",
        test_cb_78 => "cb 78.json", test_cb_79 => "cb 79.json", test_cb_7a => "cb 7a.json", test_cb_7b => "cb 7b.json",
        test_cb_7c => "cb 7c.json", test_cb_7d => "cb 7d.json", test_cb_7e => "cb 7e.json", test_cb_7f => "cb 7f.json",
        test_cb_80 => "cb 80.json", test_cb_81 => "cb 81.json", test_cb_82 => "cb 82.json", test_cb_83 => "cb 83.json",
        test_cb_84 => "cb 84.json", test_cb_85 => "cb 85.json", test_cb_86 => "cb 86.json", test_cb_87 => "cb 87.json",
        test_cb_88 => "cb 88.json", test_cb_89 => "cb 89.json", test_cb_8a => "cb 8a.json", test_cb_8b => "cb 8b.json",
        test_cb_8c => "cb 8c.json", test_cb_8d => "cb 8d.json", test_cb_8e => "cb 8e.json", test_cb_8f => "cb 8f.json",
        test_cb_90 => "cb 90.json", test_cb_91 => "cb 91.json", test_cb_92 => "cb 92.json", test_cb_93 => "cb 93.json",
        test_cb_94 => "cb 94.json", test_cb_95 => "cb 95.json", test_cb_96 => "cb 96.json", test_cb_97 => "cb 97.json",
        test_cb_98 => "cb 98.json", test_cb_99 => "cb 99.json", test_cb_9a => "cb 9a.json", test_cb_9b => "cb 9b.json",
        test_cb_9c => "cb 9c.json", test_cb_9d => "cb 9d.json", test_cb_9e => "cb 9e.json", test_cb_9f => "cb 9f.json",
        test_cb_a0 => "cb a0.json", test_cb_a1 => "cb a1.json", test_cb_a2 => "cb a2.json", test_cb_a3 => "cb a3.json",
        test_cb_a4 => "cb a4.json", test_cb_a5 => "cb a5.json", test_cb_a6 => "cb a6.json", test_cb_a7 => "cb a7.json",
        test_cb_a8 => "cb a8.json", test_cb_a9 => "cb a9.json", test_cb_aa => "cb aa.json", test_cb_ab => "cb ab.json",
        test_cb_ac => "cb ac.json", test_cb_ad => "cb ad.json", test_cb_ae => "cb ae.json", test_cb_af => "cb af.json",
        test_cb_b0 => "cb b0.json", test_cb_b1 => "cb b1.json", test_cb_b2 => "cb b2.json", test_cb_b3 => "cb b3.json",
        test_cb_b4 => "cb b4.json", test_cb_b5 => "cb b5.json", test_cb_b6 => "cb b6.json", test_cb_b7 => "cb b7.json",
        test_cb_b8 => "cb b8.json", test_cb_b9 => "cb b9.json", test_cb_ba => "cb ba.json", test_cb_bb => "cb bb.json",
        test_cb_bc => "cb bc.json", test_cb_bd => "cb bd.json", test_cb_be => "cb be.json", test_cb_bf => "cb bf.json",
        test_cb_c0 => "cb c0.json", test_cb_c1 => "cb c1.json", test_cb_c2 => "cb c2.json", test_cb_c3 => "cb c3.json",
        test_cb_c4 => "cb c4.json", test_cb_c5 => "cb c5.json", test_cb_c6 => "cb c6.json", test_cb_c7 => "cb c7.json",
        test_cb_c8 => "cb c8.json", test_cb_c9 => "cb c9.json", test_cb_ca => "cb ca.json", test_cb_cb => "cb cb.json",
        test_cb_cc => "cb cc.json", test_cb_cd => "cb cd.json", test_cb_ce => "cb ce.json", test_cb_cf => "cb cf.json",
        test_cb_d0 => "cb d0.json", test_cb_d1 => "cb d1.json", test_cb_d2 => "cb d2.json", test_cb_d3 => "cb d3.json",
        test_cb_d4 => "cb d4.json", test_cb_d5 => "cb d5.json", test_cb_d6 => "cb d6.json", test_cb_d7 => "cb d7.json",
        test_cb_d8 => "cb d8.json", test_cb_d9 => "cb d9.json", test_cb_da => "cb da.json", test_cb_db => "cb db.json",
        test_cb_dc => "cb dc.json", test_cb_dd => "cb dd.json", test_cb_de => "cb de.json", test_cb_df => "cb df.json",
        test_cb_e0 => "cb e0.json", test_cb_e1 => "cb e1.json", test_cb_e2 => "cb e2.json", test_cb_e3 => "cb e3.json",
        test_cb_e4 => "cb e4.json", test_cb_e5 => "cb e5.json", test_cb_e6 => "cb e6.json", test_cb_e7 => "cb e7.json",
        test_cb_e8 => "cb e8.json", test_cb_e9 => "cb e9.json", test_cb_ea => "cb ea.json", test_cb_eb => "cb eb.json",
        test_cb_ec => "cb ec.json", test_cb_ed => "cb ed.json", test_cb_ee => "cb ee.json", test_cb_ef => "cb ef.json",
        test_cb_f0 => "cb f0.json", test_cb_f1 => "cb f1.json", test_cb_f2 => "cb f2.json", test_cb_f3 => "cb f3.json",
        test_cb_f4 => "cb f4.json", test_cb_f5 => "cb f5.json", test_cb_f6 => "cb f6.json", test_cb_f7 => "cb f7.json",
        test_cb_f8 => "cb f8.json", test_cb_f9 => "cb f9.json", test_cb_fa => "cb fa.json", test_cb_fb => "cb fb.json",
        test_cb_fc => "cb fc.json", test_cb_fd => "cb fd.json", test_cb_fe => "cb fe.json", test_cb_ff => "cb ff.json",
    }

    z80_tests! {
        test_ed_40 => "ed 40.json", test_ed_41 => "ed 41.json", test_ed_42 => "ed 42.json", test_ed_43 => "ed 43.json",
        test_ed_44 => "ed 44.json", test_ed_45 => "ed 45.json", test_ed_46 => "ed 46.json", test_ed_47 => "ed 47.json",
        test_ed_48 => "ed 48.json", test_ed_49 => "ed 49.json", test_ed_4a => "ed 4a.json", test_ed_4b => "ed 4b.json",
        test_ed_4c => "ed 4c.json", test_ed_4d => "ed 4d.json", test_ed_4e => "ed 4e.json", test_ed_4f => "ed 4f.json",
        test_ed_50 => "ed 50.json", test_ed_51 => "ed 51.json", test_ed_52 => "ed 52.json", test_ed_53 => "ed 53.json",
        test_ed_54 => "ed 54.json", test_ed_55 => "ed 55.json", test_ed_56 => "ed 56.json", test_ed_57 => "ed 57.json",
        test_ed_58 => "ed 58.json", test_ed_59 => "ed 59.json", test_ed_5a => "ed 5a.json", test_ed_5b => "ed 5b.json",
        test_ed_5c => "ed 5c.json", test_ed_5d => "ed 5d.json", test_ed_5e => "ed 5e.json", test_ed_5f => "ed 5f.json",
        test_ed_60 => "ed 60.json", test_ed_61 => "ed 61.json", test_ed_62 => "ed 62.json", test_ed_63 => "ed 63.json",
        test_ed_64 => "ed 64.json", test_ed_65 => "ed 65.json", test_ed_66 => "ed 66.json", test_ed_67 => "ed 67.json",
        test_ed_68 => "ed 68.json", test_ed_69 => "ed 69.json", test_ed_6a => "ed 6a.json", test_ed_6b => "ed 6b.json",
        test_ed_6c => "ed 6c.json", test_ed_6d => "ed 6d.json", test_ed_6e => "ed 6e.json", test_ed_6f => "ed 6f.json",
        test_ed_70 => "ed 70.json", test_ed_71 => "ed 71.json", test_ed_72 => "ed 72.json", test_ed_73 => "ed 73.json",
        test_ed_74 => "ed 74.json", test_ed_75 => "ed 75.json", test_ed_76 => "ed 76.json", test_ed_77 => "ed 77.json",
        test_ed_78 => "ed 78.json", test_ed_79 => "ed 79.json", test_ed_7a => "ed 7a.json", test_ed_7b => "ed 7b.json",
        test_ed_7c => "ed 7c.json", test_ed_7d => "ed 7d.json", test_ed_7e => "ed 7e.json", test_ed_7f => "ed 7f.json",
        test_ed_a0 => "ed a0.json", test_ed_a1 => "ed a1.json", test_ed_a2 => "ed a2.json", test_ed_a3 => "ed a3.json",
        test_ed_a8 => "ed a8.json", test_ed_a9 => "ed a9.json", test_ed_aa => "ed aa.json", test_ed_ab => "ed ab.json",
        test_ed_b0 => "ed b0.json", test_ed_b1 => "ed b1.json", test_ed_b2 => "ed b2.json", test_ed_b3 => "ed b3.json",
        test_ed_b8 => "ed b8.json", test_ed_b9 => "ed b9.json", test_ed_ba => "ed ba.json", test_ed_bb => "ed bb.json",
    }

    z80_tests! {
        test_dd_00 => "dd 00.json", test_dd_01 => "dd 01.json", test_dd_02 => "dd 02.json", test_dd_03 => "dd 03.json",
        test_dd_04 => "dd 04.json", test_dd_05 => "dd 05.json", test_dd_06 => "dd 06.json", test_dd_07 => "dd 07.json",
        test_dd_08 => "dd 08.json", test_dd_09 => "dd 09.json", test_dd_0a => "dd 0a.json", test_dd_0b => "dd 0b.json",
        test_dd_0c => "dd 0c.json", test_dd_0d => "dd 0d.json", test_dd_0e => "dd 0e.json", test_dd_0f => "dd 0f.json",
        test_dd_10 => "dd 10.json", test_dd_11 => "dd 11.json", test_dd_12 => "dd 12.json", test_dd_13 => "dd 13.json",
        test_dd_14 => "dd 14.json", test_dd_15 => "dd 15.json", test_dd_16 => "dd 16.json", test_dd_17 => "dd 17.json",
        test_dd_18 => "dd 18.json", test_dd_19 => "dd 19.json", test_dd_1a => "dd 1a.json", test_dd_1b => "dd 1b.json",
        test_dd_1c => "dd 1c.json", test_dd_1d => "dd 1d.json", test_dd_1e => "dd 1e.json", test_dd_1f => "dd 1f.json",
        test_dd_20 => "dd 20.json", test_dd_21 => "dd 21.json", test_dd_22 => "dd 22.json", test_dd_23 => "dd 23.json",
        test_dd_24 => "dd 24.json", test_dd_25 => "dd 25.json", test_dd_26 => "dd 26.json", test_dd_27 => "dd 27.json",
        test_dd_28 => "dd 28.json", test_dd_29 => "dd 29.json", test_dd_2a => "dd 2a.json", test_dd_2b => "dd 2b.json",
        test_dd_2c => "dd 2c.json", test_dd_2d => "dd 2d.json", test_dd_2e => "dd 2e.json", test_dd_2f => "dd 2f.json",
        test_dd_30 => "dd 30.json", test_dd_31 => "dd 31.json", test_dd_32 => "dd 32.json", test_dd_33 => "dd 33.json",
        test_dd_34 => "dd 34.json", test_dd_35 => "dd 35.json", test_dd_36 => "dd 36.json", test_dd_37 => "dd 37.json",
        test_dd_38 => "dd 38.json", test_dd_39 => "dd 39.json", test_dd_3a => "dd 3a.json", test_dd_3b => "dd 3b.json",
        test_dd_3c => "dd 3c.json", test_dd_3d => "dd 3d.json", test_dd_3e => "dd 3e.json", test_dd_3f => "dd 3f.json",
        test_dd_40 => "dd 40.json", test_dd_41 => "dd 41.json", test_dd_42 => "dd 42.json", test_dd_43 => "dd 43.json",
        test_dd_44 => "dd 44.json", test_dd_45 => "dd 45.json", test_dd_46 => "dd 46.json", test_dd_47 => "dd 47.json",
        test_dd_48 => "dd 48.json", test_dd_49 => "dd 49.json", test_dd_4a => "dd 4a.json", test_dd_4b => "dd 4b.json",
        test_dd_4c => "dd 4c.json", test_dd_4d => "dd 4d.json", test_dd_4e => "dd 4e.json", test_dd_4f => "dd 4f.json",
        test_dd_50 => "dd 50.json", test_dd_51 => "dd 51.json", test_dd_52 => "dd 52.json", test_dd_53 => "dd 53.json",
        test_dd_54 => "dd 54.json", test_dd_55 => "dd 55.json", test_dd_56 => "dd 56.json", test_dd_57 => "dd 57.json",
        test_dd_58 => "dd 58.json", test_dd_59 => "dd 59.json", test_dd_5a => "dd 5a.json", test_dd_5b => "dd 5b.json",
        test_dd_5c => "dd 5c.json", test_dd_5d => "dd 5d.json", test_dd_5e => "dd 5e.json", test_dd_5f => "dd 5f.json",
        test_dd_60 => "dd 60.json", test_dd_61 => "dd 61.json", test_dd_62 => "dd 62.json", test_dd_63 => "dd 63.json",
        test_dd_64 => "dd 64.json", test_dd_65 => "dd 65.json", test_dd_66 => "dd 66.json", test_dd_67 => "dd 67.json",
        test_dd_68 => "dd 68.json", test_dd_69 => "dd 69.json", test_dd_6a => "dd 6a.json", test_dd_6b => "dd 6b.json",
        test_dd_6c => "dd 6c.json", test_dd_6d => "dd 6d.json", test_dd_6e => "dd 6e.json", test_dd_6f => "dd 6f.json",
        test_dd_70 => "dd 70.json", test_dd_71 => "dd 71.json", test_dd_72 => "dd 72.json", test_dd_73 => "dd 73.json",
        test_dd_74 => "dd 74.json", test_dd_75 => "dd 75.json", test_dd_76 => "dd 76.json", test_dd_77 => "dd 77.json",
        test_dd_78 => "dd 78.json", test_dd_79 => "dd 79.json", test_dd_7a => "dd 7a.json", test_dd_7b => "dd 7b.json",
        test_dd_7c => "dd 7c.json", test_dd_7d => "dd 7d.json", test_dd_7e => "dd 7e.json", test_dd_7f => "dd 7f.json",
        test_dd_80 => "dd 80.json", test_dd_81 => "dd 81.json", test_dd_82 => "dd 82.json", test_dd_83 => "dd 83.json",
        test_dd_84 => "dd 84.json", test_dd_85 => "dd 85.json", test_dd_86 => "dd 86.json", test_dd_87 => "dd 87.json",
        test_dd_88 => "dd 88.json", test_dd_89 => "dd 89.json", test_dd_8a => "dd 8a.json", test_dd_8b => "dd 8b.json",
        test_dd_8c => "dd 8c.json", test_dd_8d => "dd 8d.json", test_dd_8e => "dd 8e.json", test_dd_8f => "dd 8f.json",
        test_dd_90 => "dd 90.json", test_dd_91 => "dd 91.json", test_dd_92 => "dd 92.json", test_dd_93 => "dd 93.json",
        test_dd_94 => "dd 94.json", test_dd_95 => "dd 95.json", test_dd_96 => "dd 96.json", test_dd_97 => "dd 97.json",
        test_dd_98 => "dd 98.json", test_dd_99 => "dd 99.json", test_dd_9a => "dd 9a.json", test_dd_9b => "dd 9b.json",
        test_dd_9c => "dd 9c.json", test_dd_9d => "dd 9d.json", test_dd_9e => "dd 9e.json", test_dd_9f => "dd 9f.json",
        test_dd_a0 => "dd a0.json", test_dd_a1 => "dd a1.json", test_dd_a2 => "dd a2.json", test_dd_a3 => "dd a3.json",
        test_dd_a4 => "dd a4.json", test_dd_a5 => "dd a5.json", test_dd_a6 => "dd a6.json", test_dd_a7 => "dd a7.json",
        test_dd_a8 => "dd a8.json", test_dd_a9 => "dd a9.json", test_dd_aa => "dd aa.json", test_dd_ab => "dd ab.json",
        test_dd_ac => "dd ac.json", test_dd_ad => "dd ad.json", test_dd_ae => "dd ae.json", test_dd_af => "dd af.json",
        test_dd_b0 => "dd b0.json", test_dd_b1 => "dd b1.json", test_dd_b2 => "dd b2.json", test_dd_b3 => "dd b3.json",
        test_dd_b4 => "dd b4.json", test_dd_b5 => "dd b5.json", test_dd_b6 => "dd b6.json", test_dd_b7 => "dd b7.json",
        test_dd_b8 => "dd b8.json", test_dd_b9 => "dd b9.json", test_dd_ba => "dd ba.json", test_dd_bb => "dd bb.json",
        test_dd_bc => "dd bc.json", test_dd_bd => "dd bd.json", test_dd_be => "dd be.json", test_dd_bf => "dd bf.json",
        test_dd_c0 => "dd c0.json", test_dd_c1 => "dd c1.json", test_dd_c2 => "dd c2.json", test_dd_c3 => "dd c3.json",
        test_dd_c4 => "dd c4.json", test_dd_c5 => "dd c5.json", test_dd_c6 => "dd c6.json", test_dd_c7 => "dd c7.json",
        test_dd_c8 => "dd c8.json", test_dd_c9 => "dd c9.json", test_dd_ca => "dd ca.json",
        test_dd_cc => "dd cc.json", test_dd_cd => "dd cd.json", test_dd_ce => "dd ce.json", test_dd_cf => "dd cf.json",
        test_dd_d0 => "dd d0.json", test_dd_d1 => "dd d1.json", test_dd_d2 => "dd d2.json", test_dd_d3 => "dd d3.json",
        test_dd_d4 => "dd d4.json", test_dd_d5 => "dd d5.json", test_dd_d6 => "dd d6.json", test_dd_d7 => "dd d7.json",
        test_dd_d8 => "dd d8.json", test_dd_d9 => "dd d9.json", test_dd_da => "dd da.json", test_dd_db => "dd db.json",
        test_dd_dc => "dd dc.json", test_dd_de => "dd de.json", test_dd_df => "dd df.json",
        test_dd_e0 => "dd e0.json", test_dd_e1 => "dd e1.json", test_dd_e2 => "dd e2.json", test_dd_e3 => "dd e3.json",
        test_dd_e4 => "dd e4.json", test_dd_e5 => "dd e5.json", test_dd_e6 => "dd e6.json", test_dd_e7 => "dd e7.json",
        test_dd_e8 => "dd e8.json", test_dd_e9 => "dd e9.json", test_dd_ea => "dd ea.json", test_dd_eb => "dd eb.json",
        test_dd_ec => "dd ec.json", test_dd_ee => "dd ee.json", test_dd_ef => "dd ef.json",
        test_dd_f0 => "dd f0.json", test_dd_f1 => "dd f1.json", test_dd_f2 => "dd f2.json", test_dd_f3 => "dd f3.json",
        test_dd_f4 => "dd f4.json", test_dd_f5 => "dd f5.json", test_dd_f6 => "dd f6.json", test_dd_f7 => "dd f7.json",
        test_dd_f8 => "dd f8.json", test_dd_f9 => "dd f9.json", test_dd_fa => "dd fa.json", test_dd_fb => "dd fb.json",
        test_dd_fc => "dd fc.json", test_dd_fe => "dd fe.json", test_dd_ff => "dd ff.json",
    }

    z80_tests! {
        test_dd_cb_00 => "dd cb __ 00.json", test_dd_cb_01 => "dd cb __ 01.json", test_dd_cb_02 => "dd cb __ 02.json", test_dd_cb_03 => "dd cb __ 03.json",
        test_dd_cb_04 => "dd cb __ 04.json", test_dd_cb_05 => "dd cb __ 05.json", test_dd_cb_06 => "dd cb __ 06.json", test_dd_cb_07 => "dd cb __ 07.json",
        test_dd_cb_08 => "dd cb __ 08.json", test_dd_cb_09 => "dd cb __ 09.json", test_dd_cb_0a => "dd cb __ 0a.json", test_dd_cb_0b => "dd cb __ 0b.json",
        test_dd_cb_0c => "dd cb __ 0c.json", test_dd_cb_0d => "dd cb __ 0d.json", test_dd_cb_0e => "dd cb __ 0e.json", test_dd_cb_0f => "dd cb __ 0f.json",
        test_dd_cb_10 => "dd cb __ 10.json", test_dd_cb_11 => "dd cb __ 11.json", test_dd_cb_12 => "dd cb __ 12.json", test_dd_cb_13 => "dd cb __ 13.json",
        test_dd_cb_14 => "dd cb __ 14.json", test_dd_cb_15 => "dd cb __ 15.json", test_dd_cb_16 => "dd cb __ 16.json", test_dd_cb_17 => "dd cb __ 17.json",
        test_dd_cb_18 => "dd cb __ 18.json", test_dd_cb_19 => "dd cb __ 19.json", test_dd_cb_1a => "dd cb __ 1a.json", test_dd_cb_1b => "dd cb __ 1b.json",
        test_dd_cb_1c => "dd cb __ 1c.json", test_dd_cb_1d => "dd cb __ 1d.json", test_dd_cb_1e => "dd cb __ 1e.json", test_dd_cb_1f => "dd cb __ 1f.json",
        test_dd_cb_20 => "dd cb __ 20.json", test_dd_cb_21 => "dd cb __ 21.json", test_dd_cb_22 => "dd cb __ 22.json", test_dd_cb_23 => "dd cb __ 23.json",
        test_dd_cb_24 => "dd cb __ 24.json", test_dd_cb_25 => "dd cb __ 25.json", test_dd_cb_26 => "dd cb __ 26.json", test_dd_cb_27 => "dd cb __ 27.json",
        test_dd_cb_28 => "dd cb __ 28.json", test_dd_cb_29 => "dd cb __ 29.json", test_dd_cb_2a => "dd cb __ 2a.json", test_dd_cb_2b => "dd cb __ 2b.json",
        test_dd_cb_2c => "dd cb __ 2c.json", test_dd_cb_2d => "dd cb __ 2d.json", test_dd_cb_2e => "dd cb __ 2e.json", test_dd_cb_2f => "dd cb __ 2f.json",
        test_dd_cb_30 => "dd cb __ 30.json", test_dd_cb_31 => "dd cb __ 31.json", test_dd_cb_32 => "dd cb __ 32.json", test_dd_cb_33 => "dd cb __ 33.json",
        test_dd_cb_34 => "dd cb __ 34.json", test_dd_cb_35 => "dd cb __ 35.json", test_dd_cb_36 => "dd cb __ 36.json", test_dd_cb_37 => "dd cb __ 37.json",
        test_dd_cb_38 => "dd cb __ 38.json", test_dd_cb_39 => "dd cb __ 39.json", test_dd_cb_3a => "dd cb __ 3a.json", test_dd_cb_3b => "dd cb __ 3b.json",
        test_dd_cb_3c => "dd cb __ 3c.json", test_dd_cb_3d => "dd cb __ 3d.json", test_dd_cb_3e => "dd cb __ 3e.json", test_dd_cb_3f => "dd cb __ 3f.json",
        test_dd_cb_40 => "dd cb __ 40.json", test_dd_cb_41 => "dd cb __ 41.json", test_dd_cb_42 => "dd cb __ 42.json", test_dd_cb_43 => "dd cb __ 43.json",
        test_dd_cb_44 => "dd cb __ 44.json", test_dd_cb_45 => "dd cb __ 45.json", test_dd_cb_46 => "dd cb __ 46.json", test_dd_cb_47 => "dd cb __ 47.json",
        test_dd_cb_48 => "dd cb __ 48.json", test_dd_cb_49 => "dd cb __ 49.json", test_dd_cb_4a => "dd cb __ 4a.json", test_dd_cb_4b => "dd cb __ 4b.json",
        test_dd_cb_4c => "dd cb __ 4c.json", test_dd_cb_4d => "dd cb __ 4d.json", test_dd_cb_4e => "dd cb __ 4e.json", test_dd_cb_4f => "dd cb __ 4f.json",
        test_dd_cb_50 => "dd cb __ 50.json", test_dd_cb_51 => "dd cb __ 51.json", test_dd_cb_52 => "dd cb __ 52.json", test_dd_cb_53 => "dd cb __ 53.json",
        test_dd_cb_54 => "dd cb __ 54.json", test_dd_cb_55 => "dd cb __ 55.json", test_dd_cb_56 => "dd cb __ 56.json", test_dd_cb_57 => "dd cb __ 57.json",
        test_dd_cb_58 => "dd cb __ 58.json", test_dd_cb_59 => "dd cb __ 59.json", test_dd_cb_5a => "dd cb __ 5a.json", test_dd_cb_5b => "dd cb __ 5b.json",
        test_dd_cb_5c => "dd cb __ 5c.json", test_dd_cb_5d => "dd cb __ 5d.json", test_dd_cb_5e => "dd cb __ 5e.json", test_dd_cb_5f => "dd cb __ 5f.json",
        test_dd_cb_60 => "dd cb __ 60.json", test_dd_cb_61 => "dd cb __ 61.json", test_dd_cb_62 => "dd cb __ 62.json", test_dd_cb_63 => "dd cb __ 63.json",
        test_dd_cb_64 => "dd cb __ 64.json", test_dd_cb_65 => "dd cb __ 65.json", test_dd_cb_66 => "dd cb __ 66.json", test_dd_cb_67 => "dd cb __ 67.json",
        test_dd_cb_68 => "dd cb __ 68.json", test_dd_cb_69 => "dd cb __ 69.json", test_dd_cb_6a => "dd cb __ 6a.json", test_dd_cb_6b => "dd cb __ 6b.json",
        test_dd_cb_6c => "dd cb __ 6c.json", test_dd_cb_6d => "dd cb __ 6d.json", test_dd_cb_6e => "dd cb __ 6e.json", test_dd_cb_6f => "dd cb __ 6f.json",
        test_dd_cb_70 => "dd cb __ 70.json", test_dd_cb_71 => "dd cb __ 71.json", test_dd_cb_72 => "dd cb __ 72.json", test_dd_cb_73 => "dd cb __ 73.json",
        test_dd_cb_74 => "dd cb __ 74.json", test_dd_cb_75 => "dd cb __ 75.json", test_dd_cb_76 => "dd cb __ 76.json", test_dd_cb_77 => "dd cb __ 77.json",
        test_dd_cb_78 => "dd cb __ 78.json", test_dd_cb_79 => "dd cb __ 79.json", test_dd_cb_7a => "dd cb __ 7a.json", test_dd_cb_7b => "dd cb __ 7b.json",
        test_dd_cb_7c => "dd cb __ 7c.json", test_dd_cb_7d => "dd cb __ 7d.json", test_dd_cb_7e => "dd cb __ 7e.json", test_dd_cb_7f => "dd cb __ 7f.json",
        test_dd_cb_80 => "dd cb __ 80.json", test_dd_cb_81 => "dd cb __ 81.json", test_dd_cb_82 => "dd cb __ 82.json", test_dd_cb_83 => "dd cb __ 83.json",
        test_dd_cb_84 => "dd cb __ 84.json", test_dd_cb_85 => "dd cb __ 85.json", test_dd_cb_86 => "dd cb __ 86.json", test_dd_cb_87 => "dd cb __ 87.json",
        test_dd_cb_88 => "dd cb __ 88.json", test_dd_cb_89 => "dd cb __ 89.json", test_dd_cb_8a => "dd cb __ 8a.json", test_dd_cb_8b => "dd cb __ 8b.json",
        test_dd_cb_8c => "dd cb __ 8c.json", test_dd_cb_8d => "dd cb __ 8d.json", test_dd_cb_8e => "dd cb __ 8e.json", test_dd_cb_8f => "dd cb __ 8f.json",
        test_dd_cb_90 => "dd cb __ 90.json", test_dd_cb_91 => "dd cb __ 91.json", test_dd_cb_92 => "dd cb __ 92.json", test_dd_cb_93 => "dd cb __ 93.json",
        test_dd_cb_94 => "dd cb __ 94.json", test_dd_cb_95 => "dd cb __ 95.json", test_dd_cb_96 => "dd cb __ 96.json", test_dd_cb_97 => "dd cb __ 97.json",
        test_dd_cb_98 => "dd cb __ 98.json", test_dd_cb_99 => "dd cb __ 99.json", test_dd_cb_9a => "dd cb __ 9a.json", test_dd_cb_9b => "dd cb __ 9b.json",
        test_dd_cb_9c => "dd cb __ 9c.json", test_dd_cb_9d => "dd cb __ 9d.json", test_dd_cb_9e => "dd cb __ 9e.json", test_dd_cb_9f => "dd cb __ 9f.json",
        test_dd_cb_a0 => "dd cb __ a0.json", test_dd_cb_a1 => "dd cb __ a1.json", test_dd_cb_a2 => "dd cb __ a2.json", test_dd_cb_a3 => "dd cb __ a3.json",
        test_dd_cb_a4 => "dd cb __ a4.json", test_dd_cb_a5 => "dd cb __ a5.json", test_dd_cb_a6 => "dd cb __ a6.json", test_dd_cb_a7 => "dd cb __ a7.json",
        test_dd_cb_a8 => "dd cb __ a8.json", test_dd_cb_a9 => "dd cb __ a9.json", test_dd_cb_aa => "dd cb __ aa.json", test_dd_cb_ab => "dd cb __ ab.json",
        test_dd_cb_ac => "dd cb __ ac.json", test_dd_cb_ad => "dd cb __ ad.json", test_dd_cb_ae => "dd cb __ ae.json", test_dd_cb_af => "dd cb __ af.json",
        test_dd_cb_b0 => "dd cb __ b0.json", test_dd_cb_b1 => "dd cb __ b1.json", test_dd_cb_b2 => "dd cb __ b2.json", test_dd_cb_b3 => "dd cb __ b3.json",
        test_dd_cb_b4 => "dd cb __ b4.json", test_dd_cb_b5 => "dd cb __ b5.json", test_dd_cb_b6 => "dd cb __ b6.json", test_dd_cb_b7 => "dd cb __ b7.json",
        test_dd_cb_b8 => "dd cb __ b8.json", test_dd_cb_b9 => "dd cb __ b9.json", test_dd_cb_ba => "dd cb __ ba.json", test_dd_cb_bb => "dd cb __ bb.json",
        test_dd_cb_bc => "dd cb __ bc.json", test_dd_cb_bd => "dd cb __ bd.json", test_dd_cb_be => "dd cb __ be.json", test_dd_cb_bf => "dd cb __ bf.json",
        test_dd_cb_c0 => "dd cb __ c0.json", test_dd_cb_c1 => "dd cb __ c1.json", test_dd_cb_c2 => "dd cb __ c2.json", test_dd_cb_c3 => "dd cb __ c3.json",
        test_dd_cb_c4 => "dd cb __ c4.json", test_dd_cb_c5 => "dd cb __ c5.json", test_dd_cb_c6 => "dd cb __ c6.json", test_dd_cb_c7 => "dd cb __ c7.json",
        test_dd_cb_c8 => "dd cb __ c8.json", test_dd_cb_c9 => "dd cb __ c9.json", test_dd_cb_ca => "dd cb __ ca.json", test_dd_cb_cb => "dd cb __ cb.json",
        test_dd_cb_cc => "dd cb __ cc.json", test_dd_cb_cd => "dd cb __ cd.json", test_dd_cb_ce => "dd cb __ ce.json", test_dd_cb_cf => "dd cb __ cf.json",
        test_dd_cb_d0 => "dd cb __ d0.json", test_dd_cb_d1 => "dd cb __ d1.json", test_dd_cb_d2 => "dd cb __ d2.json", test_dd_cb_d3 => "dd cb __ d3.json",
        test_dd_cb_d4 => "dd cb __ d4.json", test_dd_cb_d5 => "dd cb __ d5.json", test_dd_cb_d6 => "dd cb __ d6.json", test_dd_cb_d7 => "dd cb __ d7.json",
        test_dd_cb_d8 => "dd cb __ d8.json", test_dd_cb_d9 => "dd cb __ d9.json", test_dd_cb_da => "dd cb __ da.json", test_dd_cb_db => "dd cb __ db.json",
        test_dd_cb_dc => "dd cb __ dc.json", test_dd_cb_dd => "dd cb __ dd.json", test_dd_cb_de => "dd cb __ de.json", test_dd_cb_df => "dd cb __ df.json",
        test_dd_cb_e0 => "dd cb __ e0.json", test_dd_cb_e1 => "dd cb __ e1.json", test_dd_cb_e2 => "dd cb __ e2.json", test_dd_cb_e3 => "dd cb __ e3.json",
        test_dd_cb_e4 => "dd cb __ e4.json", test_dd_cb_e5 => "dd cb __ e5.json", test_dd_cb_e6 => "dd cb __ e6.json", test_dd_cb_e7 => "dd cb __ e7.json",
        test_dd_cb_e8 => "dd cb __ e8.json", test_dd_cb_e9 => "dd cb __ e9.json", test_dd_cb_ea => "dd cb __ ea.json", test_dd_cb_eb => "dd cb __ eb.json",
        test_dd_cb_ec => "dd cb __ ec.json", test_dd_cb_ed => "dd cb __ ed.json", test_dd_cb_ee => "dd cb __ ee.json", test_dd_cb_ef => "dd cb __ ef.json",
        test_dd_cb_f0 => "dd cb __ f0.json", test_dd_cb_f1 => "dd cb __ f1.json", test_dd_cb_f2 => "dd cb __ f2.json", test_dd_cb_f3 => "dd cb __ f3.json",
        test_dd_cb_f4 => "dd cb __ f4.json", test_dd_cb_f5 => "dd cb __ f5.json", test_dd_cb_f6 => "dd cb __ f6.json", test_dd_cb_f7 => "dd cb __ f7.json",
        test_dd_cb_f8 => "dd cb __ f8.json", test_dd_cb_f9 => "dd cb __ f9.json", test_dd_cb_fa => "dd cb __ fa.json", test_dd_cb_fb => "dd cb __ fb.json",
        test_dd_cb_fc => "dd cb __ fc.json", test_dd_cb_fd => "dd cb __ fd.json", test_dd_cb_fe => "dd cb __ fe.json", test_dd_cb_ff => "dd cb __ ff.json",
    }

    z80_tests! {
        test_fd_00 => "fd 00.json", test_fd_01 => "fd 01.json", test_fd_02 => "fd 02.json", test_fd_03 => "fd 03.json",
        test_fd_04 => "fd 04.json", test_fd_05 => "fd 05.json", test_fd_06 => "fd 06.json", test_fd_07 => "fd 07.json",
        test_fd_08 => "fd 08.json", test_fd_09 => "fd 09.json", test_fd_0a => "fd 0a.json", test_fd_0b => "fd 0b.json",
        test_fd_0c => "fd 0c.json", test_fd_0d => "fd 0d.json", test_fd_0e => "fd 0e.json", test_fd_0f => "fd 0f.json",
        test_fd_10 => "fd 10.json", test_fd_11 => "fd 11.json", test_fd_12 => "fd 12.json", test_fd_13 => "fd 13.json",
        test_fd_14 => "fd 14.json", test_fd_15 => "fd 15.json", test_fd_16 => "fd 16.json", test_fd_17 => "fd 17.json",
        test_fd_18 => "fd 18.json", test_fd_19 => "fd 19.json", test_fd_1a => "fd 1a.json", test_fd_1b => "fd 1b.json",
        test_fd_1c => "fd 1c.json", test_fd_1d => "fd 1d.json", test_fd_1e => "fd 1e.json", test_fd_1f => "fd 1f.json",
        test_fd_20 => "fd 20.json", test_fd_21 => "fd 21.json", test_fd_22 => "fd 22.json", test_fd_23 => "fd 23.json",
        test_fd_24 => "fd 24.json", test_fd_25 => "fd 25.json", test_fd_26 => "fd 26.json", test_fd_27 => "fd 27.json",
        test_fd_28 => "fd 28.json", test_fd_29 => "fd 29.json", test_fd_2a => "fd 2a.json", test_fd_2b => "fd 2b.json",
        test_fd_2c => "fd 2c.json", test_fd_2d => "fd 2d.json", test_fd_2e => "fd 2e.json", test_fd_2f => "fd 2f.json",
        test_fd_30 => "fd 30.json", test_fd_31 => "fd 31.json", test_fd_32 => "fd 32.json", test_fd_33 => "fd 33.json",
        test_fd_34 => "fd 34.json", test_fd_35 => "fd 35.json", test_fd_36 => "fd 36.json", test_fd_37 => "fd 37.json",
        test_fd_38 => "fd 38.json", test_fd_39 => "fd 39.json", test_fd_3a => "fd 3a.json", test_fd_3b => "fd 3b.json",
        test_fd_3c => "fd 3c.json", test_fd_3d => "fd 3d.json", test_fd_3e => "fd 3e.json", test_fd_3f => "fd 3f.json",
        test_fd_40 => "fd 40.json", test_fd_41 => "fd 41.json", test_fd_42 => "fd 42.json", test_fd_43 => "fd 43.json",
        test_fd_44 => "fd 44.json", test_fd_45 => "fd 45.json", test_fd_46 => "fd 46.json", test_fd_47 => "fd 47.json",
        test_fd_48 => "fd 48.json", test_fd_49 => "fd 49.json", test_fd_4a => "fd 4a.json", test_fd_4b => "fd 4b.json",
        test_fd_4c => "fd 4c.json", test_fd_4d => "fd 4d.json", test_fd_4e => "fd 4e.json", test_fd_4f => "fd 4f.json",
        test_fd_50 => "fd 50.json", test_fd_51 => "fd 51.json", test_fd_52 => "fd 52.json", test_fd_53 => "fd 53.json",
        test_fd_54 => "fd 54.json", test_fd_55 => "fd 55.json", test_fd_56 => "fd 56.json", test_fd_57 => "fd 57.json",
        test_fd_58 => "fd 58.json", test_fd_59 => "fd 59.json", test_fd_5a => "fd 5a.json", test_fd_5b => "fd 5b.json",
        test_fd_5c => "fd 5c.json", test_fd_5d => "fd 5d.json", test_fd_5e => "fd 5e.json", test_fd_5f => "fd 5f.json",
        test_fd_60 => "fd 60.json", test_fd_61 => "fd 61.json", test_fd_62 => "fd 62.json", test_fd_63 => "fd 63.json",
        test_fd_64 => "fd 64.json", test_fd_65 => "fd 65.json", test_fd_66 => "fd 66.json", test_fd_67 => "fd 67.json",
        test_fd_68 => "fd 68.json", test_fd_69 => "fd 69.json", test_fd_6a => "fd 6a.json", test_fd_6b => "fd 6b.json",
        test_fd_6c => "fd 6c.json", test_fd_6d => "fd 6d.json", test_fd_6e => "fd 6e.json", test_fd_6f => "fd 6f.json",
        test_fd_70 => "fd 70.json", test_fd_71 => "fd 71.json", test_fd_72 => "fd 72.json", test_fd_73 => "fd 73.json",
        test_fd_74 => "fd 74.json", test_fd_75 => "fd 75.json", test_fd_76 => "fd 76.json", test_fd_77 => "fd 77.json",
        test_fd_78 => "fd 78.json", test_fd_79 => "fd 79.json", test_fd_7a => "fd 7a.json", test_fd_7b => "fd 7b.json",
        test_fd_7c => "fd 7c.json", test_fd_7d => "fd 7d.json", test_fd_7e => "fd 7e.json", test_fd_7f => "fd 7f.json",
        test_fd_80 => "fd 80.json", test_fd_81 => "fd 81.json", test_fd_82 => "fd 82.json", test_fd_83 => "fd 83.json",
        test_fd_84 => "fd 84.json", test_fd_85 => "fd 85.json", test_fd_86 => "fd 86.json", test_fd_87 => "fd 87.json",
        test_fd_88 => "fd 88.json", test_fd_89 => "fd 89.json", test_fd_8a => "fd 8a.json", test_fd_8b => "fd 8b.json",
        test_fd_8c => "fd 8c.json", test_fd_8d => "fd 8d.json", test_fd_8e => "fd 8e.json", test_fd_8f => "fd 8f.json",
        test_fd_90 => "fd 90.json", test_fd_91 => "fd 91.json", test_fd_92 => "fd 92.json", test_fd_93 => "fd 93.json",
        test_fd_94 => "fd 94.json", test_fd_95 => "fd 95.json", test_fd_96 => "fd 96.json", test_fd_97 => "fd 97.json",
        test_fd_98 => "fd 98.json", test_fd_99 => "fd 99.json", test_fd_9a => "fd 9a.json", test_fd_9b => "fd 9b.json",
        test_fd_9c => "fd 9c.json", test_fd_9d => "fd 9d.json", test_fd_9e => "fd 9e.json", test_fd_9f => "fd 9f.json",
        test_fd_a0 => "fd a0.json", test_fd_a1 => "fd a1.json", test_fd_a2 => "fd a2.json", test_fd_a3 => "fd a3.json",
        test_fd_a4 => "fd a4.json", test_fd_a5 => "fd a5.json", test_fd_a6 => "fd a6.json", test_fd_a7 => "fd a7.json",
        test_fd_a8 => "fd a8.json", test_fd_a9 => "fd a9.json", test_fd_aa => "fd aa.json", test_fd_ab => "fd ab.json",
        test_fd_ac => "fd ac.json", test_fd_ad => "fd ad.json", test_fd_ae => "fd ae.json", test_fd_af => "fd af.json",
        test_fd_b0 => "fd b0.json", test_fd_b1 => "fd b1.json", test_fd_b2 => "fd b2.json", test_fd_b3 => "fd b3.json",
        test_fd_b4 => "fd b4.json", test_fd_b5 => "fd b5.json", test_fd_b6 => "fd b6.json", test_fd_b7 => "fd b7.json",
        test_fd_b8 => "fd b8.json", test_fd_b9 => "fd b9.json", test_fd_ba => "fd ba.json", test_fd_bb => "fd bb.json",
        test_fd_bc => "fd bc.json", test_fd_bd => "fd bd.json", test_fd_be => "fd be.json", test_fd_bf => "fd bf.json",
        test_fd_c0 => "fd c0.json", test_fd_c1 => "fd c1.json", test_fd_c2 => "fd c2.json", test_fd_c3 => "fd c3.json",
        test_fd_c4 => "fd c4.json", test_fd_c5 => "fd c5.json", test_fd_c6 => "fd c6.json", test_fd_c7 => "fd c7.json",
        test_fd_c8 => "fd c8.json", test_fd_c9 => "fd c9.json", test_fd_ca => "fd ca.json",
        test_fd_cc => "fd cc.json", test_fd_cd => "fd cd.json", test_fd_ce => "fd ce.json", test_fd_cf => "fd cf.json",
        test_fd_d0 => "fd d0.json", test_fd_d1 => "fd d1.json", test_fd_d2 => "fd d2.json", test_fd_d3 => "fd d3.json",
        test_fd_d4 => "fd d4.json", test_fd_d5 => "fd d5.json", test_fd_d6 => "fd d6.json", test_fd_d7 => "fd d7.json",
        test_fd_d8 => "fd d8.json", test_fd_d9 => "fd d9.json", test_fd_da => "fd da.json", test_fd_db => "fd db.json",
        test_fd_dc => "fd dc.json", test_fd_de => "fd de.json", test_fd_df => "fd df.json",
        test_fd_e0 => "fd e0.json", test_fd_e1 => "fd e1.json", test_fd_e2 => "fd e2.json", test_fd_e3 => "fd e3.json",
        test_fd_e4 => "fd e4.json", test_fd_e5 => "fd e5.json", test_fd_e6 => "fd e6.json", test_fd_e7 => "fd e7.json",
        test_fd_e8 => "fd e8.json", test_fd_e9 => "fd e9.json", test_fd_ea => "fd ea.json", test_fd_eb => "fd eb.json",
        test_fd_ec => "fd ec.json", test_fd_ee => "fd ee.json", test_fd_ef => "fd ef.json",
        test_fd_f0 => "fd f0.json", test_fd_f1 => "fd f1.json", test_fd_f2 => "fd f2.json", test_fd_f3 => "fd f3.json",
        test_fd_f4 => "fd f4.json", test_fd_f5 => "fd f5.json", test_fd_f6 => "fd f6.json", test_fd_f7 => "fd f7.json",
        test_fd_f8 => "fd f8.json", test_fd_f9 => "fd f9.json", test_fd_fa => "fd fa.json", test_fd_fb => "fd fb.json",
        test_fd_fc => "fd fc.json", test_fd_fe => "fd fe.json", test_fd_ff => "fd ff.json",
    }

    z80_tests! {
        test_fd_cb_00 => "fd cb __ 00.json", test_fd_cb_01 => "fd cb __ 01.json", test_fd_cb_02 => "fd cb __ 02.json", test_fd_cb_03 => "fd cb __ 03.json",
        test_fd_cb_04 => "fd cb __ 04.json", test_fd_cb_05 => "fd cb __ 05.json", test_fd_cb_06 => "fd cb __ 06.json", test_fd_cb_07 => "fd cb __ 07.json",
        test_fd_cb_08 => "fd cb __ 08.json", test_fd_cb_09 => "fd cb __ 09.json", test_fd_cb_0a => "fd cb __ 0a.json", test_fd_cb_0b => "fd cb __ 0b.json",
        test_fd_cb_0c => "fd cb __ 0c.json", test_fd_cb_0d => "fd cb __ 0d.json", test_fd_cb_0e => "fd cb __ 0e.json", test_fd_cb_0f => "fd cb __ 0f.json",
        test_fd_cb_10 => "fd cb __ 10.json", test_fd_cb_11 => "fd cb __ 11.json", test_fd_cb_12 => "fd cb __ 12.json", test_fd_cb_13 => "fd cb __ 13.json",
        test_fd_cb_14 => "fd cb __ 14.json", test_fd_cb_15 => "fd cb __ 15.json", test_fd_cb_16 => "fd cb __ 16.json", test_fd_cb_17 => "fd cb __ 17.json",
        test_fd_cb_18 => "fd cb __ 18.json", test_fd_cb_19 => "fd cb __ 19.json", test_fd_cb_1a => "fd cb __ 1a.json", test_fd_cb_1b => "fd cb __ 1b.json",
        test_fd_cb_1c => "fd cb __ 1c.json", test_fd_cb_1d => "fd cb __ 1d.json", test_fd_cb_1e => "fd cb __ 1e.json", test_fd_cb_1f => "fd cb __ 1f.json",
        test_fd_cb_20 => "fd cb __ 20.json", test_fd_cb_21 => "fd cb __ 21.json", test_fd_cb_22 => "fd cb __ 22.json", test_fd_cb_23 => "fd cb __ 23.json",
        test_fd_cb_24 => "fd cb __ 24.json", test_fd_cb_25 => "fd cb __ 25.json", test_fd_cb_26 => "fd cb __ 26.json", test_fd_cb_27 => "fd cb __ 27.json",
        test_fd_cb_28 => "fd cb __ 28.json", test_fd_cb_29 => "fd cb __ 29.json", test_fd_cb_2a => "fd cb __ 2a.json", test_fd_cb_2b => "fd cb __ 2b.json",
        test_fd_cb_2c => "fd cb __ 2c.json", test_fd_cb_2d => "fd cb __ 2d.json", test_fd_cb_2e => "fd cb __ 2e.json", test_fd_cb_2f => "fd cb __ 2f.json",
        test_fd_cb_30 => "fd cb __ 30.json", test_fd_cb_31 => "fd cb __ 31.json", test_fd_cb_32 => "fd cb __ 32.json", test_fd_cb_33 => "fd cb __ 33.json",
        test_fd_cb_34 => "fd cb __ 34.json", test_fd_cb_35 => "fd cb __ 35.json", test_fd_cb_36 => "fd cb __ 36.json", test_fd_cb_37 => "fd cb __ 37.json",
        test_fd_cb_38 => "fd cb __ 38.json", test_fd_cb_39 => "fd cb __ 39.json", test_fd_cb_3a => "fd cb __ 3a.json", test_fd_cb_3b => "fd cb __ 3b.json",
        test_fd_cb_3c => "fd cb __ 3c.json", test_fd_cb_3d => "fd cb __ 3d.json", test_fd_cb_3e => "fd cb __ 3e.json", test_fd_cb_3f => "fd cb __ 3f.json",
        test_fd_cb_40 => "fd cb __ 40.json", test_fd_cb_41 => "fd cb __ 41.json", test_fd_cb_42 => "fd cb __ 42.json", test_fd_cb_43 => "fd cb __ 43.json",
        test_fd_cb_44 => "fd cb __ 44.json", test_fd_cb_45 => "fd cb __ 45.json", test_fd_cb_46 => "fd cb __ 46.json", test_fd_cb_47 => "fd cb __ 47.json",
        test_fd_cb_48 => "fd cb __ 48.json", test_fd_cb_49 => "fd cb __ 49.json", test_fd_cb_4a => "fd cb __ 4a.json", test_fd_cb_4b => "fd cb __ 4b.json",
        test_fd_cb_4c => "fd cb __ 4c.json", test_fd_cb_4d => "fd cb __ 4d.json", test_fd_cb_4e => "fd cb __ 4e.json", test_fd_cb_4f => "fd cb __ 4f.json",
        test_fd_cb_50 => "fd cb __ 50.json", test_fd_cb_51 => "fd cb __ 51.json", test_fd_cb_52 => "fd cb __ 52.json", test_fd_cb_53 => "fd cb __ 53.json",
        test_fd_cb_54 => "fd cb __ 54.json", test_fd_cb_55 => "fd cb __ 55.json", test_fd_cb_56 => "fd cb __ 56.json", test_fd_cb_57 => "fd cb __ 57.json",
        test_fd_cb_58 => "fd cb __ 58.json", test_fd_cb_59 => "fd cb __ 59.json", test_fd_cb_5a => "fd cb __ 5a.json", test_fd_cb_5b => "fd cb __ 5b.json",
        test_fd_cb_5c => "fd cb __ 5c.json", test_fd_cb_5d => "fd cb __ 5d.json", test_fd_cb_5e => "fd cb __ 5e.json", test_fd_cb_5f => "fd cb __ 5f.json",
        test_fd_cb_60 => "fd cb __ 60.json", test_fd_cb_61 => "fd cb __ 61.json", test_fd_cb_62 => "fd cb __ 62.json", test_fd_cb_63 => "fd cb __ 63.json",
        test_fd_cb_64 => "fd cb __ 64.json", test_fd_cb_65 => "fd cb __ 65.json", test_fd_cb_66 => "fd cb __ 66.json", test_fd_cb_67 => "fd cb __ 67.json",
        test_fd_cb_68 => "fd cb __ 68.json", test_fd_cb_69 => "fd cb __ 69.json", test_fd_cb_6a => "fd cb __ 6a.json", test_fd_cb_6b => "fd cb __ 6b.json",
        test_fd_cb_6c => "fd cb __ 6c.json", test_fd_cb_6d => "fd cb __ 6d.json", test_fd_cb_6e => "fd cb __ 6e.json", test_fd_cb_6f => "fd cb __ 6f.json",
        test_fd_cb_70 => "fd cb __ 70.json", test_fd_cb_71 => "fd cb __ 71.json", test_fd_cb_72 => "fd cb __ 72.json", test_fd_cb_73 => "fd cb __ 73.json",
        test_fd_cb_74 => "fd cb __ 74.json", test_fd_cb_75 => "fd cb __ 75.json", test_fd_cb_76 => "fd cb __ 76.json", test_fd_cb_77 => "fd cb __ 77.json",
        test_fd_cb_78 => "fd cb __ 78.json", test_fd_cb_79 => "fd cb __ 79.json", test_fd_cb_7a => "fd cb __ 7a.json", test_fd_cb_7b => "fd cb __ 7b.json",
        test_fd_cb_7c => "fd cb __ 7c.json", test_fd_cb_7d => "fd cb __ 7d.json", test_fd_cb_7e => "fd cb __ 7e.json", test_fd_cb_7f => "fd cb __ 7f.json",
        test_fd_cb_80 => "fd cb __ 80.json", test_fd_cb_81 => "fd cb __ 81.json", test_fd_cb_82 => "fd cb __ 82.json", test_fd_cb_83 => "fd cb __ 83.json",
        test_fd_cb_84 => "fd cb __ 84.json", test_fd_cb_85 => "fd cb __ 85.json", test_fd_cb_86 => "fd cb __ 86.json", test_fd_cb_87 => "fd cb __ 87.json",
        test_fd_cb_88 => "fd cb __ 88.json", test_fd_cb_89 => "fd cb __ 89.json", test_fd_cb_8a => "fd cb __ 8a.json", test_fd_cb_8b => "fd cb __ 8b.json",
        test_fd_cb_8c => "fd cb __ 8c.json", test_fd_cb_8d => "fd cb __ 8d.json", test_fd_cb_8e => "fd cb __ 8e.json", test_fd_cb_8f => "fd cb __ 8f.json",
        test_fd_cb_90 => "fd cb __ 90.json", test_fd_cb_91 => "fd cb __ 91.json", test_fd_cb_92 => "fd cb __ 92.json", test_fd_cb_93 => "fd cb __ 93.json",
        test_fd_cb_94 => "fd cb __ 94.json", test_fd_cb_95 => "fd cb __ 95.json", test_fd_cb_96 => "fd cb __ 96.json", test_fd_cb_97 => "fd cb __ 97.json",
        test_fd_cb_98 => "fd cb __ 98.json", test_fd_cb_99 => "fd cb __ 99.json", test_fd_cb_9a => "fd cb __ 9a.json", test_fd_cb_9b => "fd cb __ 9b.json",
        test_fd_cb_9c => "fd cb __ 9c.json", test_fd_cb_9d => "fd cb __ 9d.json", test_fd_cb_9e => "fd cb __ 9e.json", test_fd_cb_9f => "fd cb __ 9f.json",
        test_fd_cb_a0 => "fd cb __ a0.json", test_fd_cb_a1 => "fd cb __ a1.json", test_fd_cb_a2 => "fd cb __ a2.json", test_fd_cb_a3 => "fd cb __ a3.json",
        test_fd_cb_a4 => "fd cb __ a4.json", test_fd_cb_a5 => "fd cb __ a5.json", test_fd_cb_a6 => "fd cb __ a6.json", test_fd_cb_a7 => "fd cb __ a7.json",
        test_fd_cb_a8 => "fd cb __ a8.json", test_fd_cb_a9 => "fd cb __ a9.json", test_fd_cb_aa => "fd cb __ aa.json", test_fd_cb_ab => "fd cb __ ab.json",
        test_fd_cb_ac => "fd cb __ ac.json", test_fd_cb_ad => "fd cb __ ad.json", test_fd_cb_ae => "fd cb __ ae.json", test_fd_cb_af => "fd cb __ af.json",
        test_fd_cb_b0 => "fd cb __ b0.json", test_fd_cb_b1 => "fd cb __ b1.json", test_fd_cb_b2 => "fd cb __ b2.json", test_fd_cb_b3 => "fd cb __ b3.json",
        test_fd_cb_b4 => "fd cb __ b4.json", test_fd_cb_b5 => "fd cb __ b5.json", test_fd_cb_b6 => "fd cb __ b6.json", test_fd_cb_b7 => "fd cb __ b7.json",
        test_fd_cb_b8 => "fd cb __ b8.json", test_fd_cb_b9 => "fd cb __ b9.json", test_fd_cb_ba => "fd cb __ ba.json", test_fd_cb_bb => "fd cb __ bb.json",
        test_fd_cb_bc => "fd cb __ bc.json", test_fd_cb_bd => "fd cb __ bd.json", test_fd_cb_be => "fd cb __ be.json", test_fd_cb_bf => "fd cb __ bf.json",
        test_fd_cb_c0 => "fd cb __ c0.json", test_fd_cb_c1 => "fd cb __ c1.json", test_fd_cb_c2 => "fd cb __ c2.json", test_fd_cb_c3 => "fd cb __ c3.json",
        test_fd_cb_c4 => "fd cb __ c4.json", test_fd_cb_c5 => "fd cb __ c5.json", test_fd_cb_c6 => "fd cb __ c6.json", test_fd_cb_c7 => "fd cb __ c7.json",
        test_fd_cb_c8 => "fd cb __ c8.json", test_fd_cb_c9 => "fd cb __ c9.json", test_fd_cb_ca => "fd cb __ ca.json", test_fd_cb_cb => "fd cb __ cb.json",
        test_fd_cb_cc => "fd cb __ cc.json", test_fd_cb_cd => "fd cb __ cd.json", test_fd_cb_ce => "fd cb __ ce.json", test_fd_cb_cf => "fd cb __ cf.json",
        test_fd_cb_d0 => "fd cb __ d0.json", test_fd_cb_d1 => "fd cb __ d1.json", test_fd_cb_d2 => "fd cb __ d2.json", test_fd_cb_d3 => "fd cb __ d3.json",
        test_fd_cb_d4 => "fd cb __ d4.json", test_fd_cb_d5 => "fd cb __ d5.json", test_fd_cb_d6 => "fd cb __ d6.json", test_fd_cb_d7 => "fd cb __ d7.json",
        test_fd_cb_d8 => "fd cb __ d8.json", test_fd_cb_d9 => "fd cb __ d9.json", test_fd_cb_da => "fd cb __ da.json", test_fd_cb_db => "fd cb __ db.json",
        test_fd_cb_dc => "fd cb __ dc.json", test_fd_cb_dd => "fd cb __ dd.json", test_fd_cb_de => "fd cb __ de.json", test_fd_cb_df => "fd cb __ df.json",
        test_fd_cb_e0 => "fd cb __ e0.json", test_fd_cb_e1 => "fd cb __ e1.json", test_fd_cb_e2 => "fd cb __ e2.json", test_fd_cb_e3 => "fd cb __ e3.json",
        test_fd_cb_e4 => "fd cb __ e4.json", test_fd_cb_e5 => "fd cb __ e5.json", test_fd_cb_e6 => "fd cb __ e6.json", test_fd_cb_e7 => "fd cb __ e7.json",
        test_fd_cb_e8 => "fd cb __ e8.json", test_fd_cb_e9 => "fd cb __ e9.json", test_fd_cb_ea => "fd cb __ ea.json", test_fd_cb_eb => "fd cb __ eb.json",
        test_fd_cb_ec => "fd cb __ ec.json", test_fd_cb_ed => "fd cb __ ed.json", test_fd_cb_ee => "fd cb __ ee.json", test_fd_cb_ef => "fd cb __ ef.json",
        test_fd_cb_f0 => "fd cb __ f0.json", test_fd_cb_f1 => "fd cb __ f1.json", test_fd_cb_f2 => "fd cb __ f2.json", test_fd_cb_f3 => "fd cb __ f3.json",
        test_fd_cb_f4 => "fd cb __ f4.json", test_fd_cb_f5 => "fd cb __ f5.json", test_fd_cb_f6 => "fd cb __ f6.json", test_fd_cb_f7 => "fd cb __ f7.json",
        test_fd_cb_f8 => "fd cb __ f8.json", test_fd_cb_f9 => "fd cb __ f9.json", test_fd_cb_fa => "fd cb __ fa.json", test_fd_cb_fb => "fd cb __ fb.json",
        test_fd_cb_fc => "fd cb __ fc.json", test_fd_cb_fd => "fd cb __ fd.json", test_fd_cb_fe => "fd cb __ fe.json", test_fd_cb_ff => "fd cb __ ff.json",
    }
}
