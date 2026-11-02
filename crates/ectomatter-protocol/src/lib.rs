//! The words of Ectomatter's core protocol (docs/core-protocol.md) that every
//! core and Ectomatter share, so a key called `F13` or a media key called
//! `Mute` means the same on every device:
//!
//! - [keys]: the USB HID keyboard usages, by the names actions use, and the
//!   words Ectomatter shows for each;
//! - [media]: the USB HID consumer usages a media key sends, the same way;
//! - [text]: text spelled in the characters a text display draws;
//! - [Action]: what a control can be given, as the protocol writes it.

pub mod device;
pub mod keys;
pub mod manifest;
pub mod media;
pub mod text;

/// The version of the protocol this crate writes (docs/core-protocol.md,
/// section 9): what Ectomatter says in `initialize`, and what each core's and
/// plugin's manifest says it speaks.
pub const PROTOCOL: u32 = 4;

/// A package's answer to `initialize` (section 3): the protocol it speaks,
/// its name and version, when Ectomatter asks for the one it speaks; a
/// message for the person when it asks for another, or says none.
pub fn meet(params: &Value, name: &str, version: &str) -> Result<Value, String> {
    match params["protocol"].as_u64() {
        Some(n) if n == PROTOCOL as u64 => Ok(json!({ "protocol": PROTOCOL, "name": name, "version": version })),
        Some(n) => Err(format!("{name} speaks version {PROTOCOL} of Ectomatter’s core protocol, and this Ectomatter speaks {n}. Update whichever is older.")),
        None => Err(format!("{name} speaks version {PROTOCOL} of Ectomatter’s core protocol, and wasn’t told which this Ectomatter speaks.")),
    }
}

use serde_json::{Value, json};

/// How a stored macro plays on a control, by the protocol's names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plays {
    /// once each press
    Once,
    /// a number of times each press
    Times(u32),
    /// over and over while the control is held
    WhileHeld,
    /// over and over until the control is pressed again
    UntilPressedAgain,
}

/// A profile a control switches to: one by its number, or the next or the
/// one before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jump {
    To(u32),
    Next,
    Previous,
}

/// What a control does, as the protocol writes it (section 5, keys).
///
/// Every core reads and writes these. A device code no action here can hold
/// travels as `Raw`, the device's own bytes, so nothing a device stores is
/// lost on its way through Ectomatter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// `{"none": true}`: the control sends nothing
    None,
    /// `{"off": true}`: the control is switched off
    Off,
    /// `{"key": "A"}`: one key, which may be a modifier alone (`LeftCtrl`)
    Key(u8),
    /// `{"shortcut": ["LeftCtrl", "C"]}`: modifiers and keys pressed together,
    /// the modifiers first, each key in the order given
    Shortcut(Vec<u8>),
    /// `{"media": "Mute"}`: a media key, by its consumer usage
    Media(u16),
    /// `{"mouse": 1}`: a mouse button (1 left, 2 right, 3 middle, 4 back, 5 forward)
    Mouse(u8),
    /// `{"scroll": "up"}` or `{"scroll": "down"}`
    Scroll { up: bool },
    /// `{"macro": 3, "plays": "times", "times": 2}`: a stored macro
    Macro { n: u32, plays: Plays },
    /// `{"profile": 2}`, `{"profile": "next"}` or `{"profile": "previous"}`
    Profile(Jump),
    /// `{"fn": true}`: the Fn layer while held
    Fn,
    /// `{"sensitivity": "switch"}`: a mouse's next sensitivity preset
    Sensitivity,
    /// `{"host": 0}`: one of the device's host slots, a job Ectomatter does
    /// when the control is pressed (section 5, input)
    Host(u32),
    /// `{"own": "game-mode"}`: one of the device's own actions, which its
    /// description lists with their labels
    Own(String),
    /// `{"raw": [3, 1, 0, 0]}`: the device's own code, kept as it is
    Raw(Vec<u8>),
}

impl Action {
    /// The action written in `v`, if it is one.
    pub fn read(v: &Value) -> Option<Action> {
        let o = v.as_object()?;
        if let Some(r) = o.get("raw") {
            let bytes: Option<Vec<u8>> = r.as_array()?.iter().map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok())).collect();
            return bytes.map(Action::Raw);
        }
        if o.contains_key("none") {
            return Some(Action::None);
        }
        if o.contains_key("off") {
            return Some(Action::Off);
        }
        if let Some(k) = o.get("key") {
            return k.as_str().and_then(keys::usage).or_else(|| k.as_u64().and_then(|n| u8::try_from(n).ok())).map(Action::Key);
        }
        if let Some(s) = o.get("shortcut") {
            let codes: Option<Vec<u8>> = s.as_array()?.iter().map(|k| k.as_str().and_then(keys::usage).or_else(|| k.as_u64().and_then(|n| u8::try_from(n).ok()))).collect();
            return codes.filter(|c| !c.is_empty()).map(Action::Shortcut);
        }
        if let Some(m) = o.get("media") {
            return m.as_str().and_then(media::code).or_else(|| m.as_u64().and_then(|n| u16::try_from(n).ok())).map(Action::Media);
        }
        if let Some(b) = o.get("mouse") {
            return b.as_u64().and_then(|n| u8::try_from(n).ok()).map(Action::Mouse);
        }
        if let Some(s) = o.get("scroll") {
            return match s.as_str()? {
                "up" => Some(Action::Scroll { up: true }),
                "down" => Some(Action::Scroll { up: false }),
                _ => None,
            };
        }
        if let Some(n) = o.get("macro") {
            let n = n.as_u64().and_then(|n| u32::try_from(n).ok())?;
            let plays = match o.get("plays").and_then(Value::as_str).unwrap_or("once") {
                "once" => Plays::Once,
                "times" => Plays::Times(o.get("times").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).unwrap_or(2)),
                "while-held" => Plays::WhileHeld,
                "until-pressed-again" => Plays::UntilPressedAgain,
                _ => return None,
            };
            return Some(Action::Macro { n, plays });
        }
        if let Some(p) = o.get("profile") {
            return Some(Action::Profile(match p {
                Value::String(s) if s == "next" => Jump::Next,
                Value::String(s) if s == "previous" => Jump::Previous,
                _ => Jump::To(p.as_u64().and_then(|n| u32::try_from(n).ok())?),
            }));
        }
        if o.contains_key("fn") {
            return Some(Action::Fn);
        }
        if o.contains_key("sensitivity") {
            return Some(Action::Sensitivity);
        }
        if let Some(h) = o.get("host") {
            return h.as_u64().and_then(|n| u32::try_from(n).ok()).map(Action::Host);
        }
        if let Some(id) = o.get("own") {
            return id.as_str().map(|s| Action::Own(s.to_string()));
        }
        None
    }

    /// The action as the protocol writes it.
    pub fn write(&self) -> Value {
        let key = |u: u8| keys::name(u).map_or(json!(u), |n| json!(n));
        match self {
            Action::None => json!({ "none": true }),
            Action::Off => json!({ "off": true }),
            Action::Key(u) => json!({ "key": key(*u) }),
            Action::Shortcut(codes) => json!({ "shortcut": codes.iter().map(|u| key(*u)).collect::<Vec<_>>() }),
            Action::Media(c) => json!({ "media": media::name(*c).map_or(json!(c), |n| json!(n)) }),
            Action::Mouse(b) => json!({ "mouse": b }),
            Action::Scroll { up } => json!({ "scroll": if *up { "up" } else { "down" } }),
            Action::Macro { n, plays } => match plays {
                Plays::Once => json!({ "macro": n, "plays": "once" }),
                Plays::Times(t) => json!({ "macro": n, "plays": "times", "times": t }),
                Plays::WhileHeld => json!({ "macro": n, "plays": "while-held" }),
                Plays::UntilPressedAgain => json!({ "macro": n, "plays": "until-pressed-again" }),
            },
            Action::Profile(Jump::To(n)) => json!({ "profile": n }),
            Action::Profile(Jump::Next) => json!({ "profile": "next" }),
            Action::Profile(Jump::Previous) => json!({ "profile": "previous" }),
            Action::Fn => json!({ "fn": true }),
            Action::Sensitivity => json!({ "sensitivity": "switch" }),
            Action::Host(n) => json!({ "host": n }),
            Action::Own(id) => json!({ "own": id }),
            Action::Raw(b) => json!({ "raw": b }),
        }
    }

    /// The protocol's word for the kind of action, as an action's tag names it.
    pub fn kind(&self) -> &'static str {
        match self {
            Action::None => "none",
            Action::Off => "off",
            Action::Key(_) | Action::Shortcut(_) => "key",
            Action::Media(_) => "media",
            Action::Mouse(_) | Action::Scroll { .. } => "mouse",
            Action::Macro { .. } => "macro",
            Action::Profile(_) => "profile",
            Action::Fn => "fn",
            Action::Sensitivity => "sensitivity",
            Action::Host(_) => "host",
            Action::Own(_) => "own",
            Action::Raw(_) => "raw",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A package meets Ectomatter speaking this version, and declines one
    /// asking for another or for none, in words for the person.
    #[test]
    fn a_package_meets_only_its_own_version() {
        assert_eq!(meet(&json!({ "protocol": PROTOCOL }), "Weather Feed", "1.0.0").unwrap(), json!({ "protocol": PROTOCOL, "name": "Weather Feed", "version": "1.0.0" }));
        assert_eq!(meet(&json!({ "protocol": 3 }), "Weather Feed", "1.0.0").unwrap_err(), "Weather Feed speaks version 4 of Ectomatter’s core protocol, and this Ectomatter speaks 3. Update whichever is older.");
        assert!(meet(&Value::Null, "Weather Feed", "1.0.0").is_err());
    }

    /// Every action reads back as itself once written, and the words a person
    /// can read are used where the protocol has them.
    #[test]
    fn every_action_reads_back_as_written() {
        let all = [
            Action::None,
            Action::Off,
            Action::Key(keys::usage("F13").unwrap()),
            Action::Key(keys::usage("LeftCtrl").unwrap()),
            Action::Key(0x32),
            Action::Shortcut(vec![keys::usage("LeftCtrl").unwrap(), keys::usage("C").unwrap()]),
            Action::Shortcut(vec![4, 1]),
            Action::Media(media::code("Mute").unwrap()),
            Action::Media(0x3F0),
            Action::Mouse(3),
            Action::Scroll { up: false },
            Action::Macro { n: 4, plays: Plays::Times(3) },
            Action::Macro { n: 9, plays: Plays::UntilPressedAgain },
            Action::Profile(Jump::To(2)),
            Action::Profile(Jump::Previous),
            Action::Fn,
            Action::Sensitivity,
            Action::Host(5),
            Action::Own("game-mode".into()),
            Action::Raw(vec![119, 179, 228, 149]),
        ];
        for a in all {
            assert_eq!(Action::read(&a.write()), Some(a.clone()), "{:?} as {}", a, a.write());
        }
        assert_eq!(Action::Key(keys::usage("F13").unwrap()).write(), json!({ "key": "F13" }));
        assert_eq!(Action::Shortcut(vec![0xE0, 0x06]).write(), json!({ "shortcut": ["LeftCtrl", "C"] }));
        assert_eq!(Action::read(&json!({ "key": "Nonsense" })), None);
    }
}
