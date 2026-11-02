//! The USB HID consumer usages (usage page 0x0C) a media key sends, by the
//! names actions use, and the words Ectomatter shows for each.

/// Each media key: its usage, its name, the words for it.
pub const MEDIA: [(u16, &str, &str); 18] = [
    (0xB5, "Next", "Next track"),
    (0xB6, "Previous", "Previous track"),
    (0xB7, "Stop", "Stop"),
    (0xCD, "PlayPause", "Play/pause"),
    (0xE2, "Mute", "Mute"),
    (0xE9, "VolumeUp", "Volume up"),
    (0xEA, "VolumeDown", "Volume down"),
    (0x183, "MediaPlayer", "Media player"),
    (0x18A, "Mail", "Mail"),
    (0x192, "Calculator", "Calculator"),
    (0x194, "Computer", "My computer"),
    (0x221, "WebSearch", "Web search"),
    (0x223, "WebHome", "Web home"),
    (0x224, "WebBack", "Web back"),
    (0x225, "WebForward", "Web forward"),
    (0x226, "WebStop", "Web stop"),
    (0x227, "WebRefresh", "Web refresh"),
    (0x22A, "WebFavorites", "Web favorites"),
];

/// The usage a media key's name stands for.
pub fn code(name: &str) -> Option<u16> {
    MEDIA.iter().find(|(_, n, _)| *n == name).map(|(c, _, _)| *c)
}

/// A media key's name, for its usage.
pub fn name(c: u16) -> Option<&'static str> {
    MEDIA.iter().find(|(u, _, _)| *u == c).map(|(_, n, _)| *n)
}

/// The words Ectomatter shows for a media key, by its usage.
pub fn label(c: u16) -> Option<&'static str> {
    MEDIA.iter().find(|(u, _, _)| *u == c).map(|(_, _, l)| *l)
}
