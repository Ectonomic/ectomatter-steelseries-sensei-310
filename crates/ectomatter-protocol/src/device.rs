//! A device as its core describes it (docs/core-protocol.md, section 4,
//! "Describing a device", and section 5's capabilities): the description
//! `device/added` carries, or a core's `deviceFile` holds. Its top level,
//! its firmware, its controls and its screens are typed here; its art's
//! print, its light and each capability's own description are JSON yet
//! (their parts are in the sections the doc gives them).
//!
//! As section 9 has it, a field this version doesn't name is passed over;
//! `Description::unknown` names each, for a check to hold a core to the
//! protocol it says it speaks. A control may give fields of its own (section
//! 4: "A core may give a control fields of its own"), so a control's are
//! never unknown.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A device's description.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Description {
    /// its id among the core's devices (none in a `deviceFile`, which is the
    /// core's one device's)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// what sort of device it is (`keyboard`, `mouse`, `deck`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// a line under its name, for a device that stands for several
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<Firmware>,
    /// its drawing, an SVG file in the core's folder
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
    /// the drawing's units to the millimeter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default)]
    pub controls: Vec<Control>,
    #[serde(default)]
    pub screens: Vec<Screen>,
    /// how its settings are laid out on the frontend's pages (section 6)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Value>,
    /// the part of the drawing the page shows, `[x, y, width, height]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<[f64; 4]>,
    /// the order its controls' groups are listed in
    #[serde(default)]
    pub groups: Vec<String>,
    /// what's printed on its controls (The device's print)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<Value>,
    /// how its light falls (The device's light), or the file in the core's
    /// folder that holds it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<Value>,
    /// its lock lights (The device's light)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_lights: Option<Value>,
    /// each capability it offers, by its name (section 5), with its own description
    #[serde(default)]
    pub capabilities: Map<String, Value>,
}

/// The firmware the device runs, as its core says it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Firmware {
    /// `retail`, `custom` or `unknown`
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<u32>,
    /// what a custom build adds, each feature by its id
    #[serde(default)]
    pub features: Vec<Value>,
    /// what newer builds add
    #[serde(default)]
    pub later: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub none: Option<String>,
}

/// A part of the device a capability can refer to.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Control {
    /// its stable id, a word a person can read (`esc`, `f1`)
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// the pieces of the drawing that are it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<Value>,
    /// the fields of the core's own, and those a page reads where a control
    /// has them (`pos`, `box`, `lit`, `molded`, `print`, `pieces`, `callout`)
    #[serde(flatten)]
    pub own: Map<String, Value>,
}

/// A display of the device's, and the pieces of the drawing that show it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Screen {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<Value>,
}

/// The fields a description may give at its top, by the names it writes.
pub const FIELDS: &[&str] = &["device", "name", "vendor", "model", "kind", "sub", "firmware", "art", "scale", "controls", "screens", "layout", "view", "groups", "print", "light", "lock_lights", "capabilities"];

/// The fields a description's firmware may give.
pub const FIRMWARE_FIELDS: &[&str] = &["kind", "version", "build", "features", "later", "words", "tag", "chip", "note", "none"];

/// The fields a screen may give.
pub const SCREEN_FIELDS: &[&str] = &["id", "label", "art"];

/// The capabilities a device may offer (section 5).
pub const CAPABILITIES: &[&str] = &["keys", "macros", "lighting", "displays", "input", "profiles", "settings", "status", "backup"];

impl Description {
    /// A description from its JSON (its comments already out).
    pub fn of(v: &Value) -> Result<Description, String> {
        Description::deserialize(v).map_err(|e| e.to_string())
    }

    /// Each field `v` gives that this version of the protocol doesn't name,
    /// by where it is (`firmware.tagg`, `capabilities.lights`), a control's
    /// own fields aside.
    pub fn unknown(v: &Value) -> Vec<String> {
        let mut out = Vec::new();
        let keys = |o: &Value, known: &[&str], at: &str, out: &mut Vec<String>| {
            for k in o.as_object().into_iter().flat_map(|o| o.keys()) {
                if !known.contains(&k.as_str()) {
                    out.push(format!("{at}{k}"));
                }
            }
        };
        keys(v, FIELDS, "", &mut out);
        keys(&v["firmware"], FIRMWARE_FIELDS, "firmware.", &mut out);
        for (i, s) in v["screens"].as_array().into_iter().flatten().enumerate() {
            keys(s, SCREEN_FIELDS, &format!("screens[{i}]."), &mut out);
        }
        keys(&v["capabilities"], CAPABILITIES, "capabilities.", &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_description_reads_and_names_what_it_doesnt_know() {
        let v = json!({
            "device": "kb-1", "name": "A keyboard", "kind": "keyboard",
            "firmware": { "kind": "custom", "build": 3, "tagg": "x" },
            "controls": [{ "id": "esc", "label": "Esc", "art": 12, "pos": 41, "anything": true }],
            "screens": [{ "id": "lcd", "label": "Display", "size": 1 }],
            "capabilities": { "keys": {}, "lights": {} },
            "colour": "red"
        });
        let d = Description::of(&v).expect("read past what it doesn't know");
        assert_eq!(d.controls[0].own.get("pos"), Some(&json!(41)), "a control's own field kept");
        assert_eq!(d.firmware.as_ref().unwrap().build, Some(3));
        assert_eq!(Description::unknown(&v), vec!["colour", "firmware.tagg", "screens[0].size", "capabilities.lights"]);
    }

    #[test]
    fn the_field_lists_are_the_descriptions() {
        let full = Description { device: Some("d".into()), name: Some("n".into()), vendor: Some("v".into()), model: Some("m".into()), kind: Some("k".into()), sub: Some("s".into()), firmware: Some(Firmware { kind: "custom".into(), version: Some("1".into()), build: Some(1), features: vec![], later: vec![], words: Some("w".into()), tag: Some("t".into()), chip: Some("c".into()), note: Some("n".into()), none: Some("n".into()) }), art: Some("a".into()), scale: Some(1.0), controls: vec![], screens: vec![Screen { id: "s".into(), label: "l".into(), art: Some(json!(1)) }], layout: Some(json!({})), view: Some([0.0; 4]), groups: vec![], print: Some(json!({})), light: Some(json!({})), lock_lights: Some(json!({})), capabilities: Map::new() };
        let v = serde_json::to_value(&full).unwrap();
        let sorted = |mut k: Vec<String>| {
            k.sort();
            k
        };
        assert_eq!(sorted(v.as_object().unwrap().keys().cloned().collect()), sorted(FIELDS.iter().map(|s| s.to_string()).collect()));
        assert_eq!(sorted(v["firmware"].as_object().unwrap().keys().cloned().collect()), sorted(FIRMWARE_FIELDS.iter().map(|s| s.to_string()).collect()));
        assert_eq!(sorted(v["screens"][0].as_object().unwrap().keys().cloned().collect()), sorted(SCREEN_FIELDS.iter().map(|s| s.to_string()).collect()));
    }
}
