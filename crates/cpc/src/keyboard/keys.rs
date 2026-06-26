#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CpcKey {
    // Row 1
    Fdot,
    Enter,
    F3,
    F6,
    F9,
    CursorDown,
    CursorRight,
    CursorUp,
    // Row 2
    F0,
    F2,
    F1,
    F5,
    F8,
    F7,
    Copy,
    CursorLeft,
    // Row 3
    Ctrl,
    Backslash,
    Shift,
    F4,
    ClosedBracket,
    Return,
    OpenBracket,
    Clr,
    // Row 4
    Comma,
    Slash,
    Colon,
    Semicolon,
    P,
    At,
    Minus,
    Caret,
    // Row 5
    Dot,
    M,
    K,
    L,
    I,
    O,
    Nine,
    Zero,
    // Row 6
    Space,
    N,
    J,
    H,
    Y,
    U,
    Seven,
    Eight,
    // Row 7
    V,
    B,
    F,
    G,
    T,
    R,
    Five,
    Six,
    // Row 8
    X,
    C,
    D,
    S,
    W,
    E,
    Three,
    Four,
    // Row 9
    Z,
    CapsLock,
    A,
    Tab,
    Q,
    Esc,
    Two,
    One,
    // Row 10
    Del,
    Unused,
    Joy0Fire1,
    Joy0Fire2,
    Joy0Right,
    Joy0Left,
    Joy0Down,
    Joy0Up,
}

impl CpcKey {
    pub const ROW1: [Self; 8] = [
        Self::Fdot,
        Self::Enter,
        Self::F3,
        Self::F6,
        Self::F9,
        Self::CursorDown,
        Self::CursorRight,
        Self::CursorUp,
    ];
    pub const ROW2: [Self; 8] = [
        Self::F0,
        Self::F2,
        Self::F1,
        Self::F5,
        Self::F8,
        Self::F7,
        Self::Copy,
        Self::CursorLeft,
    ];
    pub const ROW3: [Self; 8] = [
        Self::Ctrl,
        Self::Backslash,
        Self::Shift,
        Self::F4,
        Self::ClosedBracket,
        Self::Return,
        Self::OpenBracket,
        Self::Clr,
    ];
    pub const ROW4: [Self; 8] = [
        Self::Comma,
        Self::Slash,
        Self::Colon,
        Self::Semicolon,
        Self::P,
        Self::At,
        Self::Minus,
        Self::Caret,
    ];
    pub const ROW5: [Self; 8] = [
        Self::Dot,
        Self::M,
        Self::K,
        Self::L,
        Self::I,
        Self::O,
        Self::Nine,
        Self::Zero,
    ];
    pub const ROW6: [Self; 8] = [
        Self::Space,
        Self::N,
        Self::J,
        Self::H,
        Self::Y,
        Self::U,
        Self::Seven,
        Self::Eight,
    ];
    pub const ROW7: [Self; 8] = [
        Self::V,
        Self::B,
        Self::F,
        Self::G,
        Self::T,
        Self::R,
        Self::Five,
        Self::Six,
    ];
    pub const ROW8: [Self; 8] = [
        Self::X,
        Self::C,
        Self::D,
        Self::S,
        Self::W,
        Self::E,
        Self::Three,
        Self::Four,
    ];
    pub const ROW9: [Self; 8] = [
        Self::Z,
        Self::CapsLock,
        Self::A,
        Self::Tab,
        Self::Q,
        Self::Esc,
        Self::Two,
        Self::One,
    ];
    pub const ROW10: [Self; 8] = [
        Self::Del,
        Self::Unused,
        Self::Joy0Fire1,
        Self::Joy0Fire2,
        Self::Joy0Right,
        Self::Joy0Left,
        Self::Joy0Down,
        Self::Joy0Up,
    ];
    pub const ALL_KEYS: [Self; 80] = {
        let mut flat = [Self::Fdot; 80];

        let matrix = [
            Self::ROW1,
            Self::ROW2,
            Self::ROW3,
            Self::ROW4,
            Self::ROW5,
            Self::ROW6,
            Self::ROW7,
            Self::ROW8,
            Self::ROW9,
            Self::ROW10,
        ];

        let mut idx = 0;
        let mut row = 0;
        while row < matrix.len() {
            let mut col = 0;

            while col < matrix[row].len() {
                flat[idx] = matrix[row][col];
                idx += 1;
                col += 1;
            }

            row += 1;
        }

        flat
    };
}
