use iced::{
    ContentFit, Element, Length, Subscription, Task as Command,
    keyboard::{
        self,
        key::{Code, Physical},
    },
    time,
    widget::{
        Space, button, column, container,
        image::{self as iced_image, FilterMethod},
        pick_list, row, text,
    },
};
use std::{
    fmt::Display,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cpc::{Cpc, CpcKey, Disk, Drive, TapePlayer, WINDOW_HEIGHT, WINDOW_WIDTH};
use z80::Z80;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CpcModel {
    Cpc464,
    Cpc6128,
}

impl Display for CpcModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Cpc464 => "CPC 464",
            Self::Cpc6128 => "CPC 6128",
        })
    }
}

const ROM_BYTES_464_MODEL: &[u8] = include_bytes!("../../../roms/cpc464.rom");
const ROM_BYTES_6128_MODEL: &[u8] = include_bytes!("../../../roms/cpc6128.rom");

const SIDEBAR_WIDTH: f32 = 200.0;
const TICKS_PER_LINE: u64 = 64;
const LINES_PER_FRAME: u64 = 312;
const TICKS_PER_FRAME: u64 = TICKS_PER_LINE * LINES_PER_FRAME;

pub fn main() -> iced::Result {
    iced::application(EmulatorApp::new, EmulatorApp::update, EmulatorApp::view)
        .subscription(EmulatorApp::subscription)
        .window_size((
            WINDOW_WIDTH as f32 * 2.0 + SIDEBAR_WIDTH,
            WINDOW_HEIGHT as f32 * 4.0,
        ))
        .title(EmulatorApp::title)
        .run()
}

// -----------------------------------------------------------------------------
// Shared State between Emulator Thread and UI
// -----------------------------------------------------------------------------
struct SharedState {
    frame_buffer: Vec<u8>, // RGBA8 buffer for Iced Image
    current_fps: f32,
    tape_playing: bool,
}

// Commands sent from UI -> Emulator Thread
enum EmuCommand {
    LoadDisk(PathBuf),
    LoadTape(PathBuf),
    ToggleTape,
    ReloadTape,
    Restart,
    ToggleFpsLimit,
    SetModel(CpcModel),
    KeyDown(CpcKey),
    KeyUp(CpcKey),
}

// -----------------------------------------------------------------------------
// Iced UI Application
// -----------------------------------------------------------------------------
struct EmulatorApp {
    shared_state: Arc<Mutex<SharedState>>,
    command_tx: std::sync::mpsc::Sender<EmuCommand>,
    unlimited_fps: bool,
    filter_method: FilterMethod,
    selected_model: CpcModel,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    LoadDiskPressed,
    DiskLoaded(Option<PathBuf>),
    LoadTapePressed,
    TapeLoaded(Option<PathBuf>),
    ToggleTapePressed,
    ReloadTapePressed,
    ModelSelected(CpcModel),
    RestartPressed,
    ToggleFpsPressed,
    ToggleFilterMethod,
    EventProcessed(iced::Event),
}

impl EmulatorApp {
    fn new() -> (Self, Command<Message>) {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let shared_state = Arc::new(Mutex::new(SharedState {
            frame_buffer: vec![0; WINDOW_WIDTH * WINDOW_HEIGHT * 4],
            current_fps: 0.0,
            tape_playing: false,
        }));

        // Spawn Emulator Thread
        std::thread::spawn({
            let state_clone = Arc::clone(&shared_state);
            move || run_emulator_thread(command_rx, state_clone)
        });

        (
            Self {
                shared_state,
                command_tx,
                unlimited_fps: false,
                filter_method: FilterMethod::Nearest,
                selected_model: CpcModel::Cpc6128,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        let state = self.shared_state.lock().unwrap();
        format!("Amstrad CPC Emulator - FPS: {:.1}", state.current_fps)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ModelSelected(cpc_model) => {
                self.selected_model = cpc_model.clone();
                let _ = self.command_tx.send(EmuCommand::SetModel(cpc_model));
            }
            Message::LoadDiskPressed => {
                return Command::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("DSK Files", &["dsk"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::DiskLoaded,
                );
            }
            Message::DiskLoaded(Some(path)) => {
                let _ = self.command_tx.send(EmuCommand::LoadDisk(path));
            }
            Message::LoadTapePressed => {
                return Command::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Tape Files", &["cdt"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::TapeLoaded,
                );
            }
            Message::TapeLoaded(Some(path)) => {
                let _ = self.command_tx.send(EmuCommand::LoadTape(path));
            }
            Message::TapeLoaded(None) => {}
            Message::ToggleTapePressed => {
                let _ = self.command_tx.send(EmuCommand::ToggleTape);
            }
            Message::ReloadTapePressed => {
                let _ = self.command_tx.send(EmuCommand::ReloadTape);
            }
            Message::RestartPressed => {
                let _ = self.command_tx.send(EmuCommand::Restart);
            }
            Message::ToggleFpsPressed => {
                self.unlimited_fps = !self.unlimited_fps;
                let _ = self.command_tx.send(EmuCommand::ToggleFpsLimit);
            }
            Message::ToggleFilterMethod => {
                self.filter_method = match self.filter_method {
                    FilterMethod::Nearest => FilterMethod::Linear,
                    FilterMethod::Linear => FilterMethod::Nearest,
                };
            }
            Message::EventProcessed(iced::Event::Keyboard(event)) => match event {
                keyboard::Event::KeyPressed { physical_key, .. } => {
                    if let Physical::Code(code) = physical_key {
                        // F12 toggles FPS limit
                        if code == Code::F12 {
                            self.unlimited_fps = !self.unlimited_fps;
                            let _ = self.command_tx.send(EmuCommand::ToggleFpsLimit);
                        } else if let Some(c_key) = map_keycode(code) {
                            let _ = self.command_tx.send(EmuCommand::KeyDown(c_key));
                        }
                    }
                }
                keyboard::Event::KeyReleased { physical_key, .. } => {
                    if let Physical::Code(code) = physical_key {
                        if code != Code::F12 {
                            if let Some(c_key) = map_keycode(code) {
                                let _ = self.command_tx.send(EmuCommand::KeyUp(c_key));
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let (current_fps, tape_playing, frame_data) = {
            let state = self.shared_state.lock().unwrap();
            (
                state.current_fps,
                state.tape_playing,
                state.frame_buffer.clone(),
            )
        };

        // Left Sidebar Control Panel
        let sidebar = column![
            text(format!("Amstrad {}", self.selected_model)).size(24),
            Space::new().height(Length::Fixed(20.0)),
            button(text("Insert DSK"))
                .width(Length::Fill)
                .on_press(Message::LoadDiskPressed),
            Space::new().height(Length::Fixed(20.0)),
            button(text("Load CDT File..."))
                .width(Length::Fill)
                .on_press(Message::LoadTapePressed),
            button(text(if tape_playing {
                "Stop Tape"
            } else {
                "Play Tape"
            }))
            .width(Length::Fill)
            .on_press(Message::ToggleTapePressed),
            button(text("Rewind Tape"))
                .width(Length::Fill)
                .on_press(Message::ReloadTapePressed),
            Space::new().height(Length::Fixed(20.0)),
            text("Model:"),
            pick_list(
                vec![CpcModel::Cpc464, CpcModel::Cpc6128],
                Some(self.selected_model.clone()),
                Message::ModelSelected
            )
            .width(Length::Fill),
            Space::new().height(Length::Fixed(20.0)),
            button(text("Restart"))
                .width(Length::Fill)
                .on_press(Message::RestartPressed),
            Space::new().height(Length::Fixed(20.0)),
            button(text(if self.unlimited_fps {
                "Lock FPS (50Hz)"
            } else {
                "Unlock FPS"
            }))
            .width(Length::Fill)
            .on_press(Message::ToggleFpsPressed),
            Space::new().height(Length::Fixed(20.0)),
            button(text(match self.filter_method {
                FilterMethod::Nearest => "Filter: Nearest",
                FilterMethod::Linear => "Filter: Linear",
            }))
            .width(Length::Fill)
            .on_press(Message::ToggleFilterMethod),
            Space::new().height(Length::Fill),
            text(format!("FPS: {:.1}", current_fps)).size(14),
        ]
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .padding(15)
        .spacing(10);

        // Emulator Screen
        let handle =
            iced_image::Handle::from_rgba(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32, frame_data);

        let screen = container(
            iced_image::Image::new(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Fill)
                .filter_method(self.filter_method),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Alignment::Center)
        .align_y(iced::alignment::Alignment::Center);

        row![sidebar, screen].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(Duration::from_millis(16)).map(|_| Message::Tick), // ~60Hz screen refresh
            iced::event::listen().map(Message::EventProcessed),
        ])
    }
}

// -----------------------------------------------------------------------------
// Core Emulator Background Thread
// -----------------------------------------------------------------------------
fn run_emulator_thread(
    rx: std::sync::mpsc::Receiver<EmuCommand>,
    shared_state: Arc<Mutex<SharedState>>,
) {
    let mut model = CpcModel::Cpc6128;
    let mut bus = Cpc::new_6128(ROM_BYTES_6128_MODEL);
    let mut cpu = Z80::new();

    let mut disk_bytes: Vec<u8> = Vec::new();
    let mut tape_bytes: Vec<u8> = Vec::new();
    let mut unlimited_fps = false;
    let mut frame_count = 0;
    let mut last_fps_update = Instant::now();
    let mut last_frame_time = Instant::now();
    let mut ticks_count: u64 = 0;

    loop {
        // Handle UI Commands
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                EmuCommand::LoadDisk(path) => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        disk_bytes = bytes;
                        match Disk::from_bytes(&disk_bytes) {
                            Ok(disk) => {
                                if let Some(mut fdc) = bus.fdc_mut() {
                                    let two_sided = disk.side_count() == 2;
                                    fdc.insert_disk(Drive::Drive0, disk);
                                    fdc.set_drive_ready(Drive::Drive0, true);
                                    fdc.set_drive_at_track0(Drive::Drive0, true);
                                    fdc.set_drive_two_sided(Drive::Drive0, two_sided);
                                    fdc.set_drive_write_protected(Drive::Drive0, false);
                                }
                            }
                            Err(err) => println!("Error reading disk: {:?}", err),
                        }
                    }
                }
                EmuCommand::LoadTape(path) => {
                    if let Ok(bytes) = std::fs::read(&path) {
                        tape_bytes = bytes;
                        if let Ok(tape) = TapePlayer::from_cdt(&tape_bytes) {
                            bus.ppi_mut().load_tape(tape);
                        }
                    }
                }
                EmuCommand::ReloadTape => {
                    if let Ok(tape) = TapePlayer::from_cdt(&tape_bytes) {
                        bus.ppi_mut().load_tape(tape);
                    }
                }
                EmuCommand::ToggleTape => {
                    if let Some(tape) = bus.ppi_mut().tape_mut() {
                        if tape.is_playing() {
                            tape.stop();
                        } else {
                            tape.play();
                        }
                    }
                }
                EmuCommand::Restart => {
                    rebuild_machine(
                        &mut bus,
                        &mut cpu,
                        &tape_bytes,
                        &mut ticks_count,
                        model.clone(),
                    );
                }
                EmuCommand::ToggleFpsLimit => unlimited_fps = !unlimited_fps,
                EmuCommand::KeyDown(k) => {
                    bus.ppi_mut().keyboard_mut().press_key(&k);
                }
                EmuCommand::KeyUp(k) => {
                    bus.ppi_mut().keyboard_mut().release_key(&k);
                }
                EmuCommand::SetModel(cpc_model) => {
                    model = cpc_model;
                    rebuild_machine(
                        &mut bus,
                        &mut cpu,
                        &tape_bytes,
                        &mut ticks_count,
                        model.clone(),
                    );
                }
            }
        }

        // Execute 1 frame worth of cycles
        loop {
            let cycles = cpu.execute(&mut bus);
            let ticks = (cycles + 3) / 4;
            ticks_count += ticks;

            for _ in 0..ticks {
                bus.tick();
                if bus.gate_array_mut().interrupt_requested() {
                    cpu.request_int(0xFF);
                }
            }

            if ticks_count >= TICKS_PER_FRAME {
                ticks_count -= TICKS_PER_FRAME;
                break;
            }
        }

        // Update Shared Framebuffer
        {
            let mut state = shared_state.lock().unwrap();
            state.frame_buffer = bus
                .video()
                .buffer()
                .iter()
                .flat_map(|rgba| rgba.to_be_bytes())
                .collect();
            // NOTE: Adjust based on your CPC PPI/TapePlayer API.
            state.tape_playing = bus.ppi().tape().map_or(false, |tape| tape.is_playing());
        }

        frame_count += 1;
        let elapsed = last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            let fps = frame_count as f32 / elapsed.as_secs_f32();
            shared_state.lock().unwrap().current_fps = fps;
            frame_count = 0;
            last_fps_update = Instant::now();
        }

        // Limit FPS to 50Hz if requested
        if !unlimited_fps {
            let target_frame_time = Duration::from_millis(20); // 50 FPS
            let time_taken = last_frame_time.elapsed();
            if time_taken < target_frame_time {
                std::thread::sleep(target_frame_time - time_taken);
            }
            last_frame_time = Instant::now();
        } else {
            last_frame_time = Instant::now();
        }
    }
}

fn rebuild_machine(
    bus: &mut Cpc,
    cpu: &mut Z80,
    tape_bytes: &Vec<u8>,
    ticks_count: &mut u64,
    model: CpcModel,
) {
    *bus = match model {
        CpcModel::Cpc464 => Cpc::new_464(ROM_BYTES_464_MODEL),
        CpcModel::Cpc6128 => Cpc::new_6128(ROM_BYTES_6128_MODEL),
    };
    *cpu = Z80::new();
    *ticks_count = 0;
    // Reload tape if we had one
    if !tape_bytes.is_empty() {
        if let Ok(tape) = TapePlayer::from_cdt(tape_bytes) {
            bus.ppi_mut().load_tape(tape);
        }
    }
}

// -----------------------------------------------------------------------------
// Keyboard Mapping
// -----------------------------------------------------------------------------
fn map_keycode(code: Code) -> Option<CpcKey> {
    match code {
        // Function keys (CPC soft keys)
        Code::F1 => Some(CpcKey::F1),
        Code::F2 => Some(CpcKey::F2),
        Code::F3 => Some(CpcKey::F3),
        Code::F4 => Some(CpcKey::F4),
        Code::F5 => Some(CpcKey::F5),
        Code::F6 => Some(CpcKey::F6),
        Code::F7 => Some(CpcKey::F7),
        Code::F8 => Some(CpcKey::F8),
        Code::F9 => Some(CpcKey::F9),
        Code::F10 => Some(CpcKey::At),

        // Numpad keys mapping to CPC function keys
        Code::Numpad0 | Code::Insert => Some(CpcKey::F0),
        Code::Numpad1 => Some(CpcKey::F1),
        Code::Numpad2 => Some(CpcKey::F2),
        Code::Numpad3 => Some(CpcKey::F3),
        Code::Numpad4 => Some(CpcKey::F4),
        Code::Numpad5 => Some(CpcKey::F5),
        Code::Numpad6 => Some(CpcKey::F6),
        Code::Numpad7 => Some(CpcKey::F7),
        Code::Numpad8 => Some(CpcKey::F8),
        Code::Numpad9 => Some(CpcKey::F9),
        Code::NumpadDecimal => Some(CpcKey::Fdot),
        Code::NumpadEnter => Some(CpcKey::Enter),

        // Cursor keys
        Code::ArrowUp => Some(CpcKey::CursorUp),
        Code::ArrowDown => Some(CpcKey::CursorDown),
        Code::ArrowLeft => Some(CpcKey::CursorLeft),
        Code::ArrowRight => Some(CpcKey::CursorRight),

        // Special keys
        Code::End | Code::PageDown => Some(CpcKey::Copy),
        Code::Home | Code::PageUp => Some(CpcKey::Clr),
        Code::Enter => Some(CpcKey::Return),
        Code::Backspace => Some(CpcKey::Del),
        Code::Escape => Some(CpcKey::Esc),
        Code::Tab => Some(CpcKey::Tab),
        Code::CapsLock => Some(CpcKey::CapsLock),
        Code::Space => Some(CpcKey::Space),

        // Modifiers
        Code::ShiftLeft | Code::ShiftRight => Some(CpcKey::Shift),
        Code::ControlLeft | Code::ControlRight => Some(CpcKey::Ctrl),

        // Punctuation
        Code::Minus => Some(CpcKey::Minus),
        Code::Equal => Some(CpcKey::Caret),
        Code::BracketLeft => Some(CpcKey::OpenBracket),
        Code::BracketRight => Some(CpcKey::ClosedBracket),
        Code::Backslash => Some(CpcKey::Backslash),
        Code::Semicolon => Some(CpcKey::Colon),
        Code::Quote => Some(CpcKey::Semicolon),
        Code::Comma => Some(CpcKey::Comma),
        Code::Period => Some(CpcKey::Dot),
        Code::Slash => Some(CpcKey::Slash),

        // Digits
        Code::Digit0 => Some(CpcKey::Zero),
        Code::Digit1 => Some(CpcKey::One),
        Code::Digit2 => Some(CpcKey::Two),
        Code::Digit3 => Some(CpcKey::Three),
        Code::Digit4 => Some(CpcKey::Four),
        Code::Digit5 => Some(CpcKey::Five),
        Code::Digit6 => Some(CpcKey::Six),
        Code::Digit7 => Some(CpcKey::Seven),
        Code::Digit8 => Some(CpcKey::Eight),
        Code::Digit9 => Some(CpcKey::Nine),

        // Letters
        Code::KeyA => Some(CpcKey::A),
        Code::KeyB => Some(CpcKey::B),
        Code::KeyC => Some(CpcKey::C),
        Code::KeyD => Some(CpcKey::D),
        Code::KeyE => Some(CpcKey::E),
        Code::KeyF => Some(CpcKey::F),
        Code::KeyG => Some(CpcKey::G),
        Code::KeyH => Some(CpcKey::H),
        Code::KeyI => Some(CpcKey::I),
        Code::KeyJ => Some(CpcKey::J),
        Code::KeyK => Some(CpcKey::K),
        Code::KeyL => Some(CpcKey::L),
        Code::KeyM => Some(CpcKey::M),
        Code::KeyN => Some(CpcKey::N),
        Code::KeyO => Some(CpcKey::O),
        Code::KeyP => Some(CpcKey::P),
        Code::KeyQ => Some(CpcKey::Q),
        Code::KeyR => Some(CpcKey::R),
        Code::KeyS => Some(CpcKey::S),
        Code::KeyT => Some(CpcKey::T),
        Code::KeyU => Some(CpcKey::U),
        Code::KeyV => Some(CpcKey::V),
        Code::KeyW => Some(CpcKey::W),
        Code::KeyX => Some(CpcKey::X),
        Code::KeyY => Some(CpcKey::Y),
        Code::KeyZ => Some(CpcKey::Z),

        _ => None,
    }
}
