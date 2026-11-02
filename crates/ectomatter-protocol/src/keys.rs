//! The USB HID keyboard usages (usage page 7) that actions name, and the
//! words Ectomatter shows for each: `F13` is the same key on every device.

/// The modifiers, in the order of their bits in a keyboard's report (bit 0
/// is usage 0xE0): their names, the words for them in full, and short.
pub const MODIFIERS: [(&str, &str, &str); 8] = [
    ("LeftCtrl", "Left Ctrl", "Ctrl"),
    ("LeftShift", "Left Shift", "Shift"),
    ("LeftAlt", "Left Alt", "Alt"),
    ("LeftMeta", "Left Super", "Super"),
    ("RightCtrl", "Right Ctrl", "Ctrl"),
    ("RightShift", "Right Shift", "Shift"),
    ("RightAlt", "Right Alt", "Alt"),
    ("RightMeta", "Right Super", "Super"),
];

/// The first modifier's usage; the others follow it.
pub const FIRST_MODIFIER: u8 = 0xE0;

/// The usages named by a table rather than a run: each with its name and the
/// words Ectomatter shows for it, where it shows any.
const NAMED: [(u8, &str, Option<&str>); 48] = [
    (40, "Enter", Some("Enter")),
    (41, "Escape", Some("Escape")),
    (42, "Backspace", Some("Backspace")),
    (43, "Tab", Some("Tab")),
    (44, "Space", Some("Space")),
    (45, "Minus", Some("-")),
    (46, "Equal", Some("=")),
    (47, "LeftBracket", Some("[")),
    (48, "RightBracket", Some("]")),
    (49, "Backslash", Some("\\")),
    (50, "NonUSHash", None),
    (51, "Semicolon", Some(";")),
    (52, "Quote", Some("'")),
    (53, "Grave", Some("`")),
    (54, "Comma", Some(",")),
    (55, "Period", Some(".")),
    (56, "Slash", Some("/")),
    (57, "CapsLock", Some("Caps Lock")),
    (70, "PrintScreen", Some("Print Screen")),
    (71, "ScrollLock", Some("Scroll Lock")),
    (72, "Pause", Some("Pause")),
    (73, "Insert", Some("Insert")),
    (74, "Home", Some("Home")),
    (75, "PageUp", Some("Page Up")),
    (76, "Delete", Some("Delete")),
    (77, "End", Some("End")),
    (78, "PageDown", Some("Page Down")),
    (79, "ArrowRight", Some("Right arrow")),
    (80, "ArrowLeft", Some("Left arrow")),
    (81, "ArrowDown", Some("Down arrow")),
    (82, "ArrowUp", Some("Up arrow")),
    (83, "NumLock", Some("Num Lock")),
    (84, "KeypadSlash", Some("Keypad /")),
    (85, "KeypadAsterisk", Some("Keypad *")),
    (86, "KeypadMinus", Some("Keypad -")),
    (87, "KeypadPlus", Some("Keypad +")),
    (88, "KeypadEnter", Some("Keypad Enter")),
    (99, "KeypadPeriod", Some("Keypad .")),
    (100, "NonUSBackslash", None),
    (101, "Menu", Some("Menu")),
    (0xE0, "LeftCtrl", Some("Left Ctrl")),
    (0xE1, "LeftShift", Some("Left Shift")),
    (0xE2, "LeftAlt", Some("Left Alt")),
    (0xE3, "LeftMeta", Some("Left Super")),
    (0xE4, "RightCtrl", Some("Right Ctrl")),
    (0xE5, "RightShift", Some("Right Shift")),
    (0xE6, "RightAlt", Some("Right Alt")),
    (0xE7, "RightMeta", Some("Right Super")),
];

/// The usage a key's name stands for.
pub fn usage(name: &str) -> Option<u8> {
    if name.len() == 1 {
        let c = name.as_bytes()[0];
        return match c {
            b'A'..=b'Z' => Some(4 + c - b'A'),
            b'1'..=b'9' => Some(30 + c - b'1'),
            b'0' => Some(39),
            _ => None,
        };
    }
    if let Some(f) = name.strip_prefix('F').and_then(|d| d.parse::<u8>().ok()) {
        return match f {
            1..=12 => Some(57 + f),
            13..=24 => Some(91 + f),
            _ => None,
        };
    }
    if let Some(d) = name.strip_prefix("Keypad").filter(|d| d.len() == 1).and_then(|d| d.parse::<u8>().ok()) {
        return Some(if d == 0 { 98 } else { 88 + d });
    }
    NAMED.iter().find(|(_, n, _)| *n == name).map(|(u, _, _)| *u)
}

/// A key's name, for its usage.
pub fn name(u: u8) -> Option<String> {
    Some(match u {
        4..=29 => ((b'A' + (u - 4)) as char).to_string(),
        30..=38 => (u - 29).to_string(),
        39 => "0".into(),
        58..=69 => format!("F{}", u - 57),
        89..=97 => format!("Keypad{}", u - 88),
        98 => "Keypad0".into(),
        104..=115 => format!("F{}", u - 91),
        _ => return NAMED.iter().find(|(c, _, _)| *c == u).map(|(_, n, _)| n.to_string()),
    })
}

/// The words Ectomatter shows for a key, by its usage.
pub fn label(u: u8) -> Option<String> {
    Some(match u {
        4..=29 => ((b'A' + (u - 4)) as char).to_string(),
        30..=38 => (u - 29).to_string(),
        39 => "0".into(),
        58..=69 => format!("F{}", u - 57),
        89..=97 => format!("Keypad {}", u - 88),
        98 => "Keypad 0".into(),
        104..=115 => format!("F{}", u - 91),
        _ => return NAMED.iter().find(|(c, _, _)| *c == u).and_then(|(_, _, l)| l.map(String::from)),
    })
}

/// Whether a usage is one of the modifiers.
pub fn is_modifier(u: u8) -> bool {
    (FIRST_MODIFIER..FIRST_MODIFIER + 8).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every named usage reads back from its name, and the names are the
    /// ones the protocol has always used (the mouse's, and the HID names).
    #[test]
    fn each_name_is_one_usage() {
        for u in 0..=255u8 {
            if let Some(n) = name(u) {
                assert_eq!(usage(&n), Some(u), "{n}");
            }
        }
        assert_eq!(usage("F13"), Some(104));
        assert_eq!(usage("PageDown"), Some(78));
        assert_eq!(usage("ArrowUp"), Some(82));
        assert_eq!(usage("Keypad0"), Some(98));
        assert_eq!(usage("LeftMeta"), Some(0xE3));
        assert_eq!(usage("RightMeta"), Some(0xE7));
        assert_eq!(label(84).as_deref(), Some("Keypad /"));
        assert_eq!(label(50), None);
        assert!(is_modifier(0xE7) && !is_modifier(0xE8));
    }
}
