//! The Sensei 310's commands, as the bytes it takes, and its one answer.
//!
//! Written from two references, neither copied: rivalcfg's profile of the
//! mouse (rivalcfg/devices/sensei310.py, with its handlers for ranges,
//! choices, color gradients and buttons) for the settings it writes, and
//! libratbag's SteelSeries driver (src/driver-steelseries.c, where the
//! Sensei 310 is "DeviceVersion 2") for its settings read, the one thing the
//! mouse answers besides its firmware.
//!
//! Every command goes to the mouse's first interface (vendor page 0xFFC0). Its
//! reports are unnumbered, so each begins with a 0 for the report number.

pub const VENDOR: u16 = 0x1038;
pub const PRODUCT: u16 = 0x1722;
pub const USAGE_PAGE: u16 = 0xFFC0;
pub const INTERFACE: i32 = 0;

/// Sensitivity is set in steps of 100, from 100 to 12000.
pub const CPI_MIN: u16 = 100;
pub const CPI_MAX: u16 = 12000;
pub const POLLING: [u16; 4] = [125, 250, 500, 1000];

/// Set sensitivity preset `n` (1 or 2) to `cpi`, rounded to the nearest 100.
pub fn cpi(n: u8, cpi: u16) -> Vec<u8> {
    let c = cpi.clamp(CPI_MIN, CPI_MAX);
    let step = ((c + 50) / 100).max(1) - 1;
    vec![0x00, 0x53, 0x00, n, step as u8, 0x00, 0x42]
}

/// Set how often the mouse reports, in Hz: one of `POLLING`.
pub fn polling(hz: u16) -> Option<Vec<u8>> {
    let v = match hz {
        1000 => 0x01,
        500 => 0x02,
        250 => 0x03,
        125 => 0x04,
        _ => return None,
    };
    Some(vec![0x00, 0x54, 0x00, v])
}

/// Keep the settings sent so far in the mouse's own memory, so they outlast
/// unplugging it.
pub fn save() -> Vec<u8> {
    vec![0x00, 0x59, 0x00]
}

/// Ask for the firmware's version; the mouse answers with two numbers.
pub fn firmware() -> Vec<u8> {
    vec![0x00, 0x90, 0x00]
}

/// The firmware's version as rivalcfg shows it: its two numbers in the order
/// the mouse sends them (`24 01` is 36.1). (libratbag reads them the other
/// way round and says it isn't sure which is right.)
pub fn firmware_version(answer: &[u8]) -> Option<String> {
    (answer.len() >= 2 && answer[..2] != [0, 0]).then(|| format!("{}.{}", answer[0], answer[1]))
}

/// Ask for the settings the mouse can say: a whole 64-byte report, as
/// libratbag sends it. Asking changes nothing.
pub fn read_settings() -> Vec<u8> {
    let mut v = vec![0u8; 64];
    v[1] = 0x92;
    v
}

/// What the settings read says: the preset in use (1 or 2), both presets'
/// sensitivity, and both lights' colors (the logo's, then the wheel's).
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub active: u8,
    pub cpi: [u16; 2],
    pub colors: [[u8; 3]; 2],
}

/// The answer to `read_settings`, as libratbag reads it: the preset in use at
/// byte 1, the presets' steps at 2 and 4 (each 100 times one more than its
/// step), the lights' colors from byte 6. (Byte 0 is 1 on the mouse tried; no
/// reference says what it is.)
pub fn parse_settings(b: &[u8]) -> Option<Settings> {
    if b.len() < 12 || !(1..=2).contains(&b[1]) {
        return None;
    }
    Some(Settings {
        active: b[1],
        cpi: [100 * (1 + b[2] as u16), 100 * (1 + b[4] as u16)],
        colors: [[b[6], b[7], b[8]], [b[9], b[10], b[11]]],
    })
}

/// The lights, by the number the mouse knows each by.
pub const LOGO: u8 = 0;
pub const WHEEL: u8 = 1;

/// A color at a point through an effect's cycle, 0 to 255.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    pub at: u8,
    pub color: [u8; 3],
}

/// A light's effect, as a feature report: colors at points through a cycle
/// of `duration` milliseconds, which the mouse plays round and round (`repeat`
/// false), or one color held (`repeat` true, a single stop), or played once
/// each time one of the buttons in `triggers` is pressed (`repeat` true, as
/// rivalcfg sets it with triggers), resting on its last color between. A
/// 26-byte header (the light's number at 0, the cycle's length at 1 and 2,
/// the repeat flag at 17, the buttons that start it at 21, a bit each, and
/// the number of stops at 25), then the first stop's color, then each stop's
/// color with how far past the last stop it comes. The stops are sent
/// closed (see `closed`).
///
/// The buttons' byte is rivalcfg's "triggers" (a buttons mask, which it
/// never sets). All eight bits set, a click lit the logo on Grant's mouse
/// (2026-10-30); which bit is which button hasn't been tried.
pub fn light(led: u8, duration: u16, stops: &[Stop], repeat: bool, triggers: u8) -> Vec<u8> {
    let stops = closed(stops, repeat);
    let mut header = [0u8; 26];
    header[0] = led;
    header[1..3].copy_from_slice(&duration.to_le_bytes());
    header[17] = u8::from(repeat);
    header[21] = triggers;
    header[25] = stops.len() as u8;
    let mut v = vec![0x00, 0x5B, 0x00];
    v.extend_from_slice(&header);
    if let Some(first) = stops.first() {
        v.extend_from_slice(&first.color);
    }
    let mut last = 0u8;
    for s in &stops {
        v.extend_from_slice(&s.color);
        v.push(s.at.saturating_sub(last));
        last = s.at;
    }
    v
}

/// A cycle's stops as the mouse is sent them: at most 14, and ending on the
/// first color again at 255, as rivalcfg's gradient handler ends a cycle.
/// The mouse jumps from the last stop straight to the first, so without
/// that a breath would snap dark rather than fade back down. One color held
/// is left as it is.
pub fn closed(stops: &[Stop], repeat: bool) -> Vec<Stop> {
    let mut stops = stops[..stops.len().min(14)].to_vec();
    if let (false, Some(first), Some(last)) = (repeat, stops.first().copied(), stops.last().copied()) {
        if stops.len() < 14 && last.at != 255 {
            stops.push(Stop { at: 255, color: first.color });
        }
    }
    stops
}

/// What a button does, as the mouse takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Job {
    /// one of the mouse's own buttons, 1 to 8 (1 left, 2 right, 3 middle, 4 back, 5 forward)
    Mouse(u8),
    /// a keyboard key, by its USB HID usage
    Key(u8),
    /// a keyboard key with up to three modifiers held: the modifiers' usages
    /// (0xE0 to 0xE7, in that order), then the key's, then nothing, as
    /// libratbag writes a button's macro of one key (Ctrl+T)
    Chord([u8; 4]),
    /// a media key, by its USB HID consumer usage
    Media(u8),
    /// switch between the two sensitivity presets
    Sensitivity,
    ScrollUp,
    ScrollDown,
    /// nothing at all
    Off,
}

/// Every button's job at once, in the mouse's order of its buttons (1 to 8):
/// five bytes each, the kind of job first (a mouse button's own number, 0x51
/// a key, 0x61 a media key, 0x30 the sensitivity switch, 0x31 and 0x32
/// scrolling, 0 nothing), then the key's usage where it has one.
pub fn buttons(jobs: &[Job; 8]) -> Vec<u8> {
    let mut v = vec![0x00, 0x31, 0x00];
    for j in jobs {
        let field: [u8; 5] = match *j {
            Job::Mouse(n) => [n, 0, 0, 0, 0],
            Job::Key(k) => [0x51, k, 0, 0, 0],
            Job::Chord(b) => [0x51, b[0], b[1], b[2], b[3]],
            Job::Media(k) => [0x61, k, 0, 0, 0],
            Job::Sensitivity => [0x30, 0, 0, 0, 0],
            Job::ScrollUp => [0x31, 0, 0, 0, 0],
            Job::ScrollDown => [0x32, 0, 0, 0, 0],
            Job::Off => [0, 0, 0, 0, 0],
        };
        v.extend_from_slice(&field);
    }
    v
}

/// USB HID keyboard usages by name, as the core protocol names keys (the USB
/// HID usage names, so every core means the same thing by "F13").
pub fn key_usage(name: &str) -> Option<u8> {
    let n = name;
    if n.len() == 1 {
        let c = n.as_bytes()[0];
        return match c {
            b'A'..=b'Z' => Some(4 + c - b'A'),
            b'a'..=b'z' => Some(4 + c - b'a'),
            b'1'..=b'9' => Some(30 + c - b'1'),
            b'0' => Some(39),
            _ => None,
        };
    }
    if let Some(f) = n.strip_prefix('F').and_then(|d| d.parse::<u8>().ok()) {
        return match f {
            1..=12 => Some(57 + f),
            13..=24 => Some(91 + f),
            _ => None,
        };
    }
    if let Some(d) = n.strip_prefix("Keypad").and_then(|d| d.parse::<u8>().ok()) {
        return match d {
            1..=9 => Some(88 + d),
            0 => Some(98),
            _ => None,
        };
    }
    Some(match n {
        "Enter" => 40,
        "Escape" => 41,
        "Backspace" => 42,
        "Tab" => 43,
        "Space" => 44,
        "Minus" => 45,
        "Equal" => 46,
        "LeftBracket" => 47,
        "RightBracket" => 48,
        "Backslash" => 49,
        "Semicolon" => 51,
        "Quote" => 52,
        "Grave" => 53,
        "Comma" => 54,
        "Period" => 55,
        "Slash" => 56,
        "CapsLock" => 57,
        "PrintScreen" => 70,
        "ScrollLock" => 71,
        "Pause" => 72,
        "Insert" => 73,
        "Home" => 74,
        "PageUp" => 75,
        "Delete" => 76,
        "End" => 77,
        "PageDown" => 78,
        "ArrowRight" => 79,
        "ArrowLeft" => 80,
        "ArrowDown" => 81,
        "ArrowUp" => 82,
        "NumLock" => 83,
        "KeypadSlash" => 84,
        "KeypadAsterisk" => 85,
        "KeypadMinus" => 86,
        "KeypadPlus" => 87,
        "KeypadEnter" => 88,
        "KeypadPeriod" => 99,
        "Menu" => 101,
        "LeftCtrl" => 224,
        "LeftShift" => 225,
        "LeftAlt" => 226,
        "LeftMeta" => 227,
        "RightCtrl" => 228,
        "RightShift" => 229,
        "RightAlt" => 230,
        "RightMeta" => 231,
        _ => return None,
    })
}

/// USB HID consumer usages for the media keys the core protocol names.
pub fn media_usage(name: &str) -> Option<u8> {
    Some(match name {
        "Mute" => 0xE2,
        "VolumeUp" => 0xE9,
        "VolumeDown" => 0xEA,
        "PlayPause" => 0xCD,
        "Next" => 0xB5,
        "Previous" => 0xB6,
        _ => return None,
    })
}
