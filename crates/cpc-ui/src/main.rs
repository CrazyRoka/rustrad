use std::time::Instant;

use cpc::{Cpc, CpcKey, CpcMemory, GateArray, Ppi, Video, WINDOW_HEIGHT, WINDOW_WIDTH};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use z80::Z80;

// TODO: Adjust these values
const CYCLES_PER_LINE: u64 = 256;
const LINES_PER_FRAME: u64 = 312;
const CYCLES_PER_FRAME: u64 = CYCLES_PER_LINE * LINES_PER_FRAME;

const ROM_BYTES_464_MODEL: &[u8] = include_bytes!("../../../roms/cpc464.rom");

fn main() {
    let memory = CpcMemory::new_64k();
    let mut bus = Cpc::new(memory, ROM_BYTES_464_MODEL);
    let mut cpu = Z80::new();
    let video = Video::new();

    let mut buffer: Vec<u32> = vec![0; WINDOW_HEIGHT * WINDOW_WIDTH];
    let mut window = match Window::new(
        "Amstrad CPC 464 Emulator",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    ) {
        Ok(win) => win,
        Err(err) => {
            panic!("Failed to create a window: {}", err);
        }
    };
    window.set_target_fps(50);
    // let mut unlimited_fps = false;
    let mut last_fps_update = Instant::now();
    let mut frame_count = 0;
    let mut cycles_counter = 0;

    while window.is_open() {
        bus.ppi_mut().keyboard_mut().reset();
        for key in window.get_keys() {
            if let Some(spectrum_key) = convert_to_cpc_key(key) {
                bus.ppi_mut().keyboard_mut().press_key(&spectrum_key);
            }
        }

        while cycles_counter < CYCLES_PER_FRAME {
            let cycles = cpu.execute(&mut bus);
            // TODO: align cycles according to Gate Array documentation
            if cycles_counter / CYCLES_PER_LINE != (cycles_counter + cycles) / CYCLES_PER_LINE {
                bus.gate_array_mut().hsync();
                if bus.gate_array_mut().interrupt_requested() {
                    cpu.request_int(0xFF);
                }
            }
            cycles_counter += cycles;
        }
        cycles_counter -= CYCLES_PER_FRAME;
        video.render(&bus, &mut buffer);

        if let Err(err) = window.update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT) {
            panic!("Failed to update window: {}", err);
        }

        frame_count += 1;
        let elapsed = last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            let fps = frame_count as f32 / elapsed.as_secs_f32();
            // let mode_str = if unlimited_fps {
            //     "Unlimited"
            // } else {
            //     "Locked (50Hz)"
            // };

            window.set_title(&format!(
                "Amstrad CPC 464 | FPS: {:.1}",
                fps,
                // "Amstrad CPC 464 | FPS: {:.1} | Mode: {} [Press F1 to Toggle]",
                // fps, mode_str
            ));

            frame_count = 0;
            last_fps_update = Instant::now();
        }
    }
}

fn convert_to_cpc_key(key: Key) -> Option<CpcKey> {
    match key {
        Key::NumPadDot => Some(CpcKey::Fdot),
        Key::NumPadEnter => Some(CpcKey::Enter),
        Key::NumPad3 | Key::F3 => Some(CpcKey::F3),
        Key::NumPad6 | Key::F6 => Some(CpcKey::F6),
        Key::NumPad9 | Key::F9 => Some(CpcKey::F9),
        Key::Down => Some(CpcKey::CursorDown),
        Key::Right => Some(CpcKey::CursorRight),
        Key::Up => Some(CpcKey::CursorUp),
        Key::NumPad0 | Key::Insert => Some(CpcKey::F0),
        Key::NumPad2 | Key::F2 => Some(CpcKey::F2),
        Key::NumPad1 | Key::F1 => Some(CpcKey::F1),
        Key::NumPad5 | Key::F5 => Some(CpcKey::F5),
        Key::NumPad8 | Key::F8 => Some(CpcKey::F8),
        Key::NumPad7 | Key::F7 => Some(CpcKey::F7),
        Key::End | Key::PageDown => Some(CpcKey::Copy),
        Key::Left => Some(CpcKey::CursorLeft),
        Key::LeftCtrl | Key::RightCtrl => Some(CpcKey::Ctrl),
        Key::Backslash => Some(CpcKey::Backslash),
        Key::LeftShift | Key::RightShift => Some(CpcKey::Shift),
        Key::NumPad4 | Key::F4 => Some(CpcKey::F4),
        Key::RightBracket => Some(CpcKey::ClosedBracket),
        Key::Enter => Some(CpcKey::Return),
        Key::LeftBracket => Some(CpcKey::OpenBracket),
        Key::Home | Key::PageUp => Some(CpcKey::Clr),
        Key::Comma => Some(CpcKey::Comma),
        Key::Slash => Some(CpcKey::Slash),
        Key::Semicolon => Some(CpcKey::Colon),
        Key::Apostrophe => Some(CpcKey::Semicolon),
        Key::P => Some(CpcKey::P),
        Key::F10 => Some(CpcKey::At),
        Key::Minus => Some(CpcKey::Minus),
        Key::Equal => Some(CpcKey::Caret),
        Key::Period => Some(CpcKey::Dot),
        Key::M => Some(CpcKey::M),
        Key::K => Some(CpcKey::K),
        Key::L => Some(CpcKey::L),
        Key::I => Some(CpcKey::I),
        Key::O => Some(CpcKey::O),
        Key::Key9 => Some(CpcKey::Nine),
        Key::Key0 => Some(CpcKey::Zero),
        Key::Space => Some(CpcKey::Space),
        Key::N => Some(CpcKey::N),
        Key::J => Some(CpcKey::J),
        Key::H => Some(CpcKey::H),
        Key::Y => Some(CpcKey::Y),
        Key::U => Some(CpcKey::U),
        Key::Key7 => Some(CpcKey::Seven),
        Key::Key8 => Some(CpcKey::Eight),
        Key::V => Some(CpcKey::V),
        Key::B => Some(CpcKey::B),
        Key::F => Some(CpcKey::F),
        Key::G => Some(CpcKey::G),
        Key::T => Some(CpcKey::T),
        Key::R => Some(CpcKey::R),
        Key::Key5 => Some(CpcKey::Five),
        Key::Key6 => Some(CpcKey::Six),
        Key::X => Some(CpcKey::X),
        Key::C => Some(CpcKey::C),
        Key::D => Some(CpcKey::D),
        Key::S => Some(CpcKey::S),
        Key::W => Some(CpcKey::W),
        Key::E => Some(CpcKey::E),
        Key::Key3 => Some(CpcKey::Three),
        Key::Key4 => Some(CpcKey::Four),
        Key::Z => Some(CpcKey::Z),
        Key::CapsLock => Some(CpcKey::CapsLock),
        Key::A => Some(CpcKey::A),
        Key::Tab => Some(CpcKey::Tab),
        Key::Q => Some(CpcKey::Q),
        Key::Escape => Some(CpcKey::Esc),
        Key::Key2 => Some(CpcKey::Two),
        Key::Key1 => Some(CpcKey::One),
        Key::Backspace => Some(CpcKey::Del),
        _ => None,
    }
}
