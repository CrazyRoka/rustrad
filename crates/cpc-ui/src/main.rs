use std::time::Instant;

use cpc::{Cpc, CpcMemory, Ppi};
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use z80::Z80;

const WINDOW_HEIGHT: usize = 100;
const WINDOW_WIDTH: usize = 100;

const ROM_BYTES_464_MODEL: &[u8] = include_bytes!("../../../roms/cpc464.rom");

fn main() {
    let memory = CpcMemory::new_64k();
    let ppi = Ppi::new();
    let mut bus = Cpc::new(memory, ROM_BYTES_464_MODEL, ppi);
    let mut cpu = Z80::new();

    let mut buffer: Vec<u32> = vec![0; WINDOW_HEIGHT * WINDOW_WIDTH];
    let mut window = match Window::new(
        "ZX Spectrum Emulator",
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
    let mut unlimited_fps = false;
    let mut last_fps_update = Instant::now();
    let mut frame_count = 0;

    while window.is_open() {
        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
            unlimited_fps = !unlimited_fps;
            if unlimited_fps {
                window.set_target_fps(0);
            } else {
                window.set_target_fps(50);
            }
        }

        loop {
            let cycles = cpu.execute(&mut bus);
        }

        if let Err(err) = window.update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT) {
            panic!("Failed to update window: {}", err);
        }

        frame_count += 1;
        let elapsed = last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            let fps = frame_count as f32 / elapsed.as_secs_f32();
            let mode_str = if unlimited_fps {
                "Unlimited"
            } else {
                "Locked (50Hz)"
            };

            window.set_title(&format!(
                "ZX Spectrum Emulator | FPS: {:.1} | Mode: {} [Press F1 to Toggle]",
                fps, mode_str
            ));

            frame_count = 0;
            last_fps_update = Instant::now();
        }
    }
}
