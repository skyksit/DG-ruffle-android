use android_activity::input::{KeyEvent, Keycode};
use ruffle_core::events::{KeyDescriptor, KeyLocation, LogicalKey, NamedKey, PhysicalKey};

pub fn android_key_event_to_ruffle_key_descriptor(android: &KeyEvent) -> Option<KeyDescriptor> {
    // TODO: Maybe do something with `android.scan_code()`?
    let physical_key = PhysicalKey::Unknown;

    let logical_key = match android.key_code() {
        Keycode::DpadUp => LogicalKey::Named(NamedKey::ArrowUp),
        Keycode::DpadDown => LogicalKey::Named(NamedKey::ArrowDown),
        Keycode::DpadLeft => LogicalKey::Named(NamedKey::ArrowLeft),
        Keycode::DpadRight => LogicalKey::Named(NamedKey::ArrowRight),
        Keycode::Keycode0 => LogicalKey::Character('0'),
        Keycode::Keycode1 => LogicalKey::Character('1'),
        Keycode::Keycode2 => LogicalKey::Character('2'),
        Keycode::Keycode3 => LogicalKey::Character('3'),
        Keycode::Keycode4 => LogicalKey::Character('4'),
        Keycode::Keycode5 => LogicalKey::Character('5'),
        Keycode::Keycode6 => LogicalKey::Character('6'),
        Keycode::Keycode7 => LogicalKey::Character('7'),
        Keycode::Keycode8 => LogicalKey::Character('8'),
        Keycode::Keycode9 => LogicalKey::Character('9'),
        Keycode::A => LogicalKey::Character('a'),
        Keycode::B => LogicalKey::Character('b'),
        Keycode::C => LogicalKey::Character('c'),
        Keycode::D => LogicalKey::Character('d'),
        Keycode::E => LogicalKey::Character('e'),
        Keycode::F => LogicalKey::Character('f'),
        Keycode::G => LogicalKey::Character('g'),
        Keycode::H => LogicalKey::Character('h'),
        Keycode::I => LogicalKey::Character('i'),
        Keycode::J => LogicalKey::Character('j'),
        Keycode::K => LogicalKey::Character('k'),
        Keycode::L => LogicalKey::Character('l'),
        Keycode::M => LogicalKey::Character('m'),
        Keycode::N => LogicalKey::Character('n'),
        Keycode::O => LogicalKey::Character('o'),
        Keycode::P => LogicalKey::Character('p'),
        Keycode::Q => LogicalKey::Character('q'),
        Keycode::R => LogicalKey::Character('r'),
        Keycode::S => LogicalKey::Character('s'),
        Keycode::T => LogicalKey::Character('t'),
        Keycode::U => LogicalKey::Character('u'),
        Keycode::V => LogicalKey::Character('v'),
        Keycode::W => LogicalKey::Character('w'),
        Keycode::X => LogicalKey::Character('x'),
        Keycode::Y => LogicalKey::Character('y'),
        Keycode::Z => LogicalKey::Character('z'),
        Keycode::Comma => LogicalKey::Character(','),
        Keycode::Period => LogicalKey::Character('.'),
        Keycode::AltLeft => LogicalKey::Named(NamedKey::Alt),
        Keycode::AltRight => LogicalKey::Named(NamedKey::Alt),
        Keycode::ShiftLeft => LogicalKey::Named(NamedKey::Shift),
        Keycode::ShiftRight => LogicalKey::Named(NamedKey::Shift),
        Keycode::Tab => LogicalKey::Named(NamedKey::Tab),
        Keycode::Space => LogicalKey::Character(' '),
        Keycode::Enter => LogicalKey::Named(NamedKey::Enter),
        Keycode::Del => LogicalKey::Named(NamedKey::Backspace),
        Keycode::Grave => LogicalKey::Character('`'),
        Keycode::Minus => LogicalKey::Character('-'),
        Keycode::Equals => LogicalKey::Character('='),
        Keycode::LeftBracket => LogicalKey::Character('['),
        Keycode::RightBracket => LogicalKey::Character(']'),
        Keycode::Backslash => LogicalKey::Character('\\'),
        Keycode::Semicolon => LogicalKey::Character(';'),
        Keycode::Apostrophe => LogicalKey::Character('\''),
        Keycode::Slash => LogicalKey::Character('/'),
        Keycode::Plus => LogicalKey::Character('+'),
        Keycode::PageUp => LogicalKey::Named(NamedKey::PageUp),
        Keycode::PageDown => LogicalKey::Named(NamedKey::PageDown),
        Keycode::Escape => LogicalKey::Named(NamedKey::Escape),
        Keycode::ForwardDel => LogicalKey::Named(NamedKey::Delete),
        Keycode::CtrlLeft => LogicalKey::Named(NamedKey::Control),
        Keycode::CtrlRight => LogicalKey::Named(NamedKey::Control),
        Keycode::CapsLock => LogicalKey::Named(NamedKey::CapsLock),
        Keycode::ScrollLock => LogicalKey::Named(NamedKey::ScrollLock),
        Keycode::Break => LogicalKey::Named(NamedKey::Pause),
        Keycode::MoveHome => LogicalKey::Named(NamedKey::Home),
        Keycode::MoveEnd => LogicalKey::Named(NamedKey::End),
        Keycode::Insert => LogicalKey::Named(NamedKey::Insert),
        Keycode::F1 => LogicalKey::Named(NamedKey::F1),
        Keycode::F2 => LogicalKey::Named(NamedKey::F2),
        Keycode::F3 => LogicalKey::Named(NamedKey::F3),
        Keycode::F4 => LogicalKey::Named(NamedKey::F4),
        Keycode::F5 => LogicalKey::Named(NamedKey::F5),
        Keycode::F6 => LogicalKey::Named(NamedKey::F6),
        Keycode::F7 => LogicalKey::Named(NamedKey::F7),
        Keycode::F8 => LogicalKey::Named(NamedKey::F8),
        Keycode::F9 => LogicalKey::Named(NamedKey::F9),
        Keycode::F10 => LogicalKey::Named(NamedKey::F10),
        Keycode::F11 => LogicalKey::Named(NamedKey::F11),
        Keycode::F12 => LogicalKey::Named(NamedKey::F12),
        Keycode::NumLock => LogicalKey::Named(NamedKey::NumLock),
        Keycode::Numpad0 => LogicalKey::Character('0'),
        Keycode::Numpad1 => LogicalKey::Character('1'),
        Keycode::Numpad2 => LogicalKey::Character('2'),
        Keycode::Numpad3 => LogicalKey::Character('3'),
        Keycode::Numpad4 => LogicalKey::Character('4'),
        Keycode::Numpad5 => LogicalKey::Character('5'),
        Keycode::Numpad6 => LogicalKey::Character('6'),
        Keycode::Numpad7 => LogicalKey::Character('7'),
        Keycode::Numpad8 => LogicalKey::Character('8'),
        Keycode::Numpad9 => LogicalKey::Character('9'),
        Keycode::NumpadDivide => LogicalKey::Character('/'),
        Keycode::NumpadMultiply => LogicalKey::Character('*'),
        Keycode::NumpadSubtract => LogicalKey::Character('-'),
        Keycode::NumpadAdd => LogicalKey::Character('+'),
        Keycode::NumpadDot => LogicalKey::Character('.'),
        Keycode::NumpadComma => LogicalKey::Character(','),
        Keycode::NumpadEnter => LogicalKey::Named(NamedKey::Enter),
        Keycode::NumpadEquals => LogicalKey::Character('='),
        _ => return None,
    };

    let key_location = match android.key_code() {
        Keycode::AltLeft | Keycode::ShiftLeft | Keycode::CtrlLeft | Keycode::MetaLeft => {
            KeyLocation::Left
        }

        Keycode::AltRight | Keycode::ShiftRight | Keycode::CtrlRight | Keycode::MetaRight => {
            KeyLocation::Right
        }

        Keycode::NumLock
        | Keycode::Numpad0
        | Keycode::Numpad1
        | Keycode::Numpad2
        | Keycode::Numpad3
        | Keycode::Numpad4
        | Keycode::Numpad5
        | Keycode::Numpad6
        | Keycode::Numpad7
        | Keycode::Numpad8
        | Keycode::Numpad9
        | Keycode::NumpadDivide
        | Keycode::NumpadMultiply
        | Keycode::NumpadSubtract
        | Keycode::NumpadAdd
        | Keycode::NumpadDot
        | Keycode::NumpadComma
        | Keycode::NumpadEnter
        | Keycode::NumpadEquals
        | Keycode::NumpadLeftParen
        | Keycode::NumpadRightParen => KeyLocation::Numpad,

        _ => KeyLocation::Standard,
    };

    Some(KeyDescriptor {
        physical_key,
        logical_key,
        key_location,
    })
}

pub fn key_tag_to_key_descriptor(tag: &str) -> Option<KeyDescriptor> {
    match tag {
        "A" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyA,
            logical_key: LogicalKey::Character('a'),
            key_location: KeyLocation::Standard,
        }),
        "B" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyB,
            logical_key: LogicalKey::Character('b'),
            key_location: KeyLocation::Standard,
        }),
        "C" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyC,
            logical_key: LogicalKey::Character('c'),
            key_location: KeyLocation::Standard,
        }),
        "D" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyD,
            logical_key: LogicalKey::Character('d'),
            key_location: KeyLocation::Standard,
        }),
        "E" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyE,
            logical_key: LogicalKey::Character('e'),
            key_location: KeyLocation::Standard,
        }),
        "F" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyF,
            logical_key: LogicalKey::Character('f'),
            key_location: KeyLocation::Standard,
        }),
        "G" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyG,
            logical_key: LogicalKey::Character('g'),
            key_location: KeyLocation::Standard,
        }),
        "H" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyH,
            logical_key: LogicalKey::Character('h'),
            key_location: KeyLocation::Standard,
        }),
        "I" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyI,
            logical_key: LogicalKey::Character('i'),
            key_location: KeyLocation::Standard,
        }),
        "J" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyJ,
            logical_key: LogicalKey::Character('j'),
            key_location: KeyLocation::Standard,
        }),
        "K" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyK,
            logical_key: LogicalKey::Character('k'),
            key_location: KeyLocation::Standard,
        }),
        "L" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyL,
            logical_key: LogicalKey::Character('l'),
            key_location: KeyLocation::Standard,
        }),
        "M" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyM,
            logical_key: LogicalKey::Character('m'),
            key_location: KeyLocation::Standard,
        }),
        "N" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyN,
            logical_key: LogicalKey::Character('n'),
            key_location: KeyLocation::Standard,
        }),
        "O" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyO,
            logical_key: LogicalKey::Character('o'),
            key_location: KeyLocation::Standard,
        }),
        "P" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyP,
            logical_key: LogicalKey::Character('p'),
            key_location: KeyLocation::Standard,
        }),
        "Q" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyQ,
            logical_key: LogicalKey::Character('q'),
            key_location: KeyLocation::Standard,
        }),
        "R" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyR,
            logical_key: LogicalKey::Character('r'),
            key_location: KeyLocation::Standard,
        }),
        "S" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyS,
            logical_key: LogicalKey::Character('s'),
            key_location: KeyLocation::Standard,
        }),
        "T" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyT,
            logical_key: LogicalKey::Character('t'),
            key_location: KeyLocation::Standard,
        }),
        "U" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyU,
            logical_key: LogicalKey::Character('u'),
            key_location: KeyLocation::Standard,
        }),
        "V" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyV,
            logical_key: LogicalKey::Character('v'),
            key_location: KeyLocation::Standard,
        }),
        "W" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyW,
            logical_key: LogicalKey::Character('w'),
            key_location: KeyLocation::Standard,
        }),
        "X" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyX,
            logical_key: LogicalKey::Character('x'),
            key_location: KeyLocation::Standard,
        }),
        "Y" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyY,
            logical_key: LogicalKey::Character('y'),
            key_location: KeyLocation::Standard,
        }),
        "Z" => Some(KeyDescriptor {
            physical_key: PhysicalKey::KeyZ,
            logical_key: LogicalKey::Character('z'),
            key_location: KeyLocation::Standard,
        }),
        "0" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit0,
            logical_key: LogicalKey::Character('0'),
            key_location: KeyLocation::Standard,
        }),
        "1" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit1,
            logical_key: LogicalKey::Character('1'),
            key_location: KeyLocation::Standard,
        }),
        "2" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit2,
            logical_key: LogicalKey::Character('2'),
            key_location: KeyLocation::Standard,
        }),
        "3" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit3,
            logical_key: LogicalKey::Character('3'),
            key_location: KeyLocation::Standard,
        }),
        "4" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit4,
            logical_key: LogicalKey::Character('4'),
            key_location: KeyLocation::Standard,
        }),
        "5" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit5,
            logical_key: LogicalKey::Character('5'),
            key_location: KeyLocation::Standard,
        }),
        "6" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit6,
            logical_key: LogicalKey::Character('6'),
            key_location: KeyLocation::Standard,
        }),
        "7" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit7,
            logical_key: LogicalKey::Character('7'),
            key_location: KeyLocation::Standard,
        }),
        "8" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit8,
            logical_key: LogicalKey::Character('8'),
            key_location: KeyLocation::Standard,
        }),
        "9" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Digit9,
            logical_key: LogicalKey::Character('9'),
            key_location: KeyLocation::Standard,
        }),
        "UP" => Some(KeyDescriptor {
            physical_key: PhysicalKey::ArrowUp,
            logical_key: LogicalKey::Named(NamedKey::ArrowUp),
            key_location: KeyLocation::Standard,
        }),
        "DOWN" => Some(KeyDescriptor {
            physical_key: PhysicalKey::ArrowDown,
            logical_key: LogicalKey::Named(NamedKey::ArrowDown),
            key_location: KeyLocation::Standard,
        }),
        "LEFT" => Some(KeyDescriptor {
            physical_key: PhysicalKey::ArrowLeft,
            logical_key: LogicalKey::Named(NamedKey::ArrowLeft),
            key_location: KeyLocation::Standard,
        }),
        "RIGHT" => Some(KeyDescriptor {
            physical_key: PhysicalKey::ArrowRight,
            logical_key: LogicalKey::Named(NamedKey::ArrowRight),
            key_location: KeyLocation::Standard,
        }),
        "SPACE" => Some(KeyDescriptor {
            physical_key: PhysicalKey::Space,
            logical_key: LogicalKey::Character(' '),
            key_location: KeyLocation::Standard,
        }),
        "ALT" => Some(KeyDescriptor {
            physical_key: PhysicalKey::AltLeft,
            logical_key: LogicalKey::Named(NamedKey::Alt),
            key_location: KeyLocation::Left,
        }),
        "CTRL" => Some(KeyDescriptor {
            physical_key: PhysicalKey::ControlLeft,
            logical_key: LogicalKey::Named(NamedKey::Control),
            key_location: KeyLocation::Left,
        }),
        _ => None,
    }
}

/// Convert an Android keycode integer to Ruffle KeyDescriptor
/// This is a convenience function for VPAD events that come as integer keycodes
/// Uses Android KeyEvent constants directly
pub fn keycode_to_key_descriptor(keycode: i32) -> Option<KeyDescriptor> {
    // Android KeyEvent keycode constants
    // https://developer.android.com/reference/android/view/KeyEvent
    let physical_key = PhysicalKey::Unknown;

    let (logical_key, key_location) = match keycode {
        // DPAD (Direction Pad)
        19 => (LogicalKey::Named(NamedKey::ArrowUp), KeyLocation::Standard),        // KEYCODE_DPAD_UP
        20 => (LogicalKey::Named(NamedKey::ArrowDown), KeyLocation::Standard),      // KEYCODE_DPAD_DOWN
        21 => (LogicalKey::Named(NamedKey::ArrowLeft), KeyLocation::Standard),      // KEYCODE_DPAD_LEFT
        22 => (LogicalKey::Named(NamedKey::ArrowRight), KeyLocation::Standard),     // KEYCODE_DPAD_RIGHT
        
        // Numbers 0-9
        7 => (LogicalKey::Character('0'), KeyLocation::Standard),   // KEYCODE_0
        8 => (LogicalKey::Character('1'), KeyLocation::Standard),   // KEYCODE_1
        9 => (LogicalKey::Character('2'), KeyLocation::Standard),   // KEYCODE_2
        10 => (LogicalKey::Character('3'), KeyLocation::Standard),  // KEYCODE_3
        11 => (LogicalKey::Character('4'), KeyLocation::Standard),  // KEYCODE_4
        12 => (LogicalKey::Character('5'), KeyLocation::Standard),  // KEYCODE_5
        13 => (LogicalKey::Character('6'), KeyLocation::Standard),  // KEYCODE_6
        14 => (LogicalKey::Character('7'), KeyLocation::Standard),  // KEYCODE_7
        15 => (LogicalKey::Character('8'), KeyLocation::Standard),  // KEYCODE_8
        16 => (LogicalKey::Character('9'), KeyLocation::Standard),  // KEYCODE_9
        
        // Letters A-Z
        29 => (LogicalKey::Character('a'), KeyLocation::Standard),  // KEYCODE_A
        30 => (LogicalKey::Character('b'), KeyLocation::Standard),  // KEYCODE_B
        31 => (LogicalKey::Character('c'), KeyLocation::Standard),  // KEYCODE_C
        32 => (LogicalKey::Character('d'), KeyLocation::Standard),  // KEYCODE_D
        33 => (LogicalKey::Character('e'), KeyLocation::Standard),  // KEYCODE_E
        34 => (LogicalKey::Character('f'), KeyLocation::Standard),  // KEYCODE_F
        35 => (LogicalKey::Character('g'), KeyLocation::Standard),  // KEYCODE_G
        36 => (LogicalKey::Character('h'), KeyLocation::Standard),  // KEYCODE_H
        37 => (LogicalKey::Character('i'), KeyLocation::Standard),  // KEYCODE_I
        38 => (LogicalKey::Character('j'), KeyLocation::Standard),  // KEYCODE_J
        39 => (LogicalKey::Character('k'), KeyLocation::Standard),  // KEYCODE_K
        40 => (LogicalKey::Character('l'), KeyLocation::Standard),  // KEYCODE_L
        41 => (LogicalKey::Character('m'), KeyLocation::Standard),  // KEYCODE_M
        42 => (LogicalKey::Character('n'), KeyLocation::Standard),  // KEYCODE_N
        43 => (LogicalKey::Character('o'), KeyLocation::Standard),  // KEYCODE_O
        44 => (LogicalKey::Character('p'), KeyLocation::Standard),  // KEYCODE_P
        45 => (LogicalKey::Character('q'), KeyLocation::Standard),  // KEYCODE_Q
        46 => (LogicalKey::Character('r'), KeyLocation::Standard),  // KEYCODE_R
        47 => (LogicalKey::Character('s'), KeyLocation::Standard),  // KEYCODE_S
        48 => (LogicalKey::Character('t'), KeyLocation::Standard),  // KEYCODE_T
        49 => (LogicalKey::Character('u'), KeyLocation::Standard),  // KEYCODE_U
        50 => (LogicalKey::Character('v'), KeyLocation::Standard),  // KEYCODE_V
        51 => (LogicalKey::Character('w'), KeyLocation::Standard),  // KEYCODE_W
        52 => (LogicalKey::Character('x'), KeyLocation::Standard),  // KEYCODE_X
        53 => (LogicalKey::Character('y'), KeyLocation::Standard),  // KEYCODE_Y
        54 => (LogicalKey::Character('z'), KeyLocation::Standard),  // KEYCODE_Z
        
        // Special keys
        55 => (LogicalKey::Character(','), KeyLocation::Standard),  // KEYCODE_COMMA
        56 => (LogicalKey::Character('.'), KeyLocation::Standard),  // KEYCODE_PERIOD
        57 => (LogicalKey::Named(NamedKey::Alt), KeyLocation::Left),     // KEYCODE_ALT_LEFT
        58 => (LogicalKey::Named(NamedKey::Alt), KeyLocation::Right),    // KEYCODE_ALT_RIGHT
        59 => (LogicalKey::Named(NamedKey::Shift), KeyLocation::Left),   // KEYCODE_SHIFT_LEFT
        60 => (LogicalKey::Named(NamedKey::Shift), KeyLocation::Right),  // KEYCODE_SHIFT_RIGHT
        61 => (LogicalKey::Named(NamedKey::Tab), KeyLocation::Standard), // KEYCODE_TAB
        62 => (LogicalKey::Character(' '), KeyLocation::Standard),  // KEYCODE_SPACE
        66 => (LogicalKey::Named(NamedKey::Enter), KeyLocation::Standard), // KEYCODE_ENTER
        67 => (LogicalKey::Named(NamedKey::Backspace), KeyLocation::Standard), // KEYCODE_DEL
        68 => (LogicalKey::Character('`'), KeyLocation::Standard),  // KEYCODE_GRAVE
        69 => (LogicalKey::Character('-'), KeyLocation::Standard),  // KEYCODE_MINUS
        70 => (LogicalKey::Character('='), KeyLocation::Standard),  // KEYCODE_EQUALS
        71 => (LogicalKey::Character('['), KeyLocation::Standard),  // KEYCODE_LEFT_BRACKET
        72 => (LogicalKey::Character(']'), KeyLocation::Standard),  // KEYCODE_RIGHT_BRACKET
        73 => (LogicalKey::Character('\\'), KeyLocation::Standard), // KEYCODE_BACKSLASH
        74 => (LogicalKey::Character(';'), KeyLocation::Standard),  // KEYCODE_SEMICOLON
        75 => (LogicalKey::Character('\''), KeyLocation::Standard), // KEYCODE_APOSTROPHE
        76 => (LogicalKey::Character('/'), KeyLocation::Standard),  // KEYCODE_SLASH
        81 => (LogicalKey::Character('+'), KeyLocation::Standard),  // KEYCODE_PLUS
        
        // Navigation
        92 => (LogicalKey::Named(NamedKey::PageUp), KeyLocation::Standard),   // KEYCODE_PAGE_UP
        93 => (LogicalKey::Named(NamedKey::PageDown), KeyLocation::Standard), // KEYCODE_PAGE_DOWN
        111 => (LogicalKey::Named(NamedKey::Escape), KeyLocation::Standard),  // KEYCODE_ESCAPE
        112 => (LogicalKey::Named(NamedKey::Delete), KeyLocation::Standard),  // KEYCODE_FORWARD_DEL
        113 => (LogicalKey::Named(NamedKey::Control), KeyLocation::Left),     // KEYCODE_CTRL_LEFT
        114 => (LogicalKey::Named(NamedKey::Control), KeyLocation::Right),    // KEYCODE_CTRL_RIGHT
        115 => (LogicalKey::Named(NamedKey::CapsLock), KeyLocation::Standard), // KEYCODE_CAPS_LOCK
        116 => (LogicalKey::Named(NamedKey::ScrollLock), KeyLocation::Standard), // KEYCODE_SCROLL_LOCK
        121 => (LogicalKey::Named(NamedKey::Pause), KeyLocation::Standard),   // KEYCODE_BREAK
        122 => (LogicalKey::Named(NamedKey::Home), KeyLocation::Standard),    // KEYCODE_MOVE_HOME
        123 => (LogicalKey::Named(NamedKey::End), KeyLocation::Standard),     // KEYCODE_MOVE_END
        124 => (LogicalKey::Named(NamedKey::Insert), KeyLocation::Standard),  // KEYCODE_INSERT
        
        // Function keys F1-F12
        131 => (LogicalKey::Named(NamedKey::F1), KeyLocation::Standard),  // KEYCODE_F1
        132 => (LogicalKey::Named(NamedKey::F2), KeyLocation::Standard),  // KEYCODE_F2
        133 => (LogicalKey::Named(NamedKey::F3), KeyLocation::Standard),  // KEYCODE_F3
        134 => (LogicalKey::Named(NamedKey::F4), KeyLocation::Standard),  // KEYCODE_F4
        135 => (LogicalKey::Named(NamedKey::F5), KeyLocation::Standard),  // KEYCODE_F5
        136 => (LogicalKey::Named(NamedKey::F6), KeyLocation::Standard),  // KEYCODE_F6
        137 => (LogicalKey::Named(NamedKey::F7), KeyLocation::Standard),  // KEYCODE_F7
        138 => (LogicalKey::Named(NamedKey::F8), KeyLocation::Standard),  // KEYCODE_F8
        139 => (LogicalKey::Named(NamedKey::F9), KeyLocation::Standard),  // KEYCODE_F9
        140 => (LogicalKey::Named(NamedKey::F10), KeyLocation::Standard), // KEYCODE_F10
        141 => (LogicalKey::Named(NamedKey::F11), KeyLocation::Standard), // KEYCODE_F11
        142 => (LogicalKey::Named(NamedKey::F12), KeyLocation::Standard), // KEYCODE_F12
        
        // Numpad
        143 => (LogicalKey::Named(NamedKey::NumLock), KeyLocation::Numpad), // KEYCODE_NUM_LOCK
        144 => (LogicalKey::Character('0'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_0
        145 => (LogicalKey::Character('1'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_1
        146 => (LogicalKey::Character('2'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_2
        147 => (LogicalKey::Character('3'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_3
        148 => (LogicalKey::Character('4'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_4
        149 => (LogicalKey::Character('5'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_5
        150 => (LogicalKey::Character('6'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_6
        151 => (LogicalKey::Character('7'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_7
        152 => (LogicalKey::Character('8'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_8
        153 => (LogicalKey::Character('9'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_9
        154 => (LogicalKey::Character('/'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_DIVIDE
        155 => (LogicalKey::Character('*'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_MULTIPLY
        156 => (LogicalKey::Character('-'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_SUBTRACT
        157 => (LogicalKey::Character('+'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_ADD
        158 => (LogicalKey::Character('.'), KeyLocation::Numpad),  // KEYCODE_NUMPAD_DOT
        159 => (LogicalKey::Character(','), KeyLocation::Numpad),  // KEYCODE_NUMPAD_COMMA
        160 => (LogicalKey::Named(NamedKey::Enter), KeyLocation::Numpad), // KEYCODE_NUMPAD_ENTER
        161 => (LogicalKey::Character('='), KeyLocation::Numpad),  // KEYCODE_NUMPAD_EQUALS
        
        _ => return None,
    };

    Some(KeyDescriptor {
        physical_key,
        logical_key,
        key_location,
    })
}
