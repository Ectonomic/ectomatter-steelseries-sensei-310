//! A package's manifest, `core.json` (docs/core-protocol.md, section 3, "A
//! core on disk", and section 11): every field it may give, typed. As section
//! 9 has it, a field this version of the protocol doesn't name (a newer
//! one's) is passed over, not an error; `Manifest::unknown` names each such
//! field, for the app to tell the package's author and for a check to hold a
//! package to the protocol it says it speaks. A use the protocol doesn't name
//! is an error: Ectomatter grants nothing it doesn't know.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A package's manifest.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    /// which the package is; a manifest without one is a core if it names the
    /// devices it reaches (`matches`), a plugin otherwise (`Manifest::kind`)
    #[serde(default, rename = "kind", skip_serializing_if = "Option::is_none")]
    pub kind_given: Option<Kind>,
    pub name: String,
    pub version: String,
    /// the core protocol it speaks
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<u32>,
    /// the oldest Ectomatter it runs on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ectomatter: Option<String>,
    /// its code, built for wasm32-wasip2, a file in its folder
    pub module: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// a sentence of what it's for, in its own words
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// an SVG file in its folder for its tile
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    /// where its releases are on GitHub, "owner/name"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// a core's device's description, a file in its folder (section 4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_file: Option<String>,
    /// the devices it works with, by name (none named: its own name's; an
    /// empty list: none of its own, as a plugin bringing in another
    /// program's has)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<String>>,
    /// the whole of what it may open
    #[serde(default)]
    pub matches: Vec<Match>,
    /// what it does for itself beyond its devices
    #[serde(default)]
    pub uses: Vec<Use>,
    /// what it adds to the frontend's pages
    #[serde(default)]
    pub adds: Vec<Adds>,
    /// the systems it works on, as `std::env::consts::OS` names them; every
    /// one where it names none
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub systems: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Core,
    Plugin,
}

/// A device a core may open: by its USB ids, and where a device shows
/// several interfaces, the one it talks to.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Match {
    pub usb: Usb,
}

/// USB ids in hex, as the system lists them ("3842").
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usb {
    pub vendor: String,
    pub product: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_page: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
}

/// One thing a package does for itself.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Use {
    /// `"monitors"`, `"nvidia"`, `"launch"`, `"sound-capture"`, `"network"`
    Word(Word),
    /// an address it may connect to, and no other
    Network {
        network: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
    /// a web host it may ask, and no other
    Web {
        web: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
    /// a file or folder it may read, whole, from /
    Read { read: String },
    /// how its games start: the programs they may run through, the
    /// variables they may set, the programs its buttons may run (section 11,
    /// A game's start; read by the frontend's launcher)
    Launch { launch: Value },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Word {
    Monitors,
    Nvidia,
    Launch,
    SoundCapture,
    Network,
}

/// What a plugin adds to the frontend's pages.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Adds {
    /// a section on each game's and app's page
    Game,
    /// a section of its own in Settings
    Settings,
    /// readings, for the screens that show them
    Readings,
}

/// The fields a manifest may give, by the names it writes them with.
pub const FIELDS: &[&str] = &["id", "kind", "name", "version", "protocol", "ectomatter", "module", "license", "description", "glyph", "repository", "deviceFile", "devices", "matches", "uses", "adds", "systems"];

/// The fields a match's `usb` may give.
pub const USB_FIELDS: &[&str] = &["vendor", "product", "interface", "usagePage", "usage"];

/// The fields a use given as an object may give, by the use they make it.
pub const USE_FIELDS: &[(&str, &[&str])] = &[("network", &["network", "to"]), ("web", &["web", "to"]), ("read", &["read"]), ("launch", &["launch"])];

/// The systems a package may say it works on, as `std::env::consts::OS`
/// names them.
pub const SYSTEMS: &[&str] = &["linux", "windows", "macos"];

impl Manifest {
    /// A manifest from its JSON (its comments already out). A field this
    /// protocol doesn't name is passed over (see `unknown`); a use it
    /// doesn't name, or a field of the wrong kind, is an error.
    pub fn of(v: &Value) -> Result<Manifest, String> {
        Manifest::deserialize(v).map_err(|e| e.to_string())
    }

    /// Each field `v` gives that this version of the protocol doesn't name,
    /// by where it is ("matches[0].usb.usage-page"), and each system it
    /// names that isn't one: what a newer protocol may add, or a misspelling.
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
        for (i, m) in v["matches"].as_array().into_iter().flatten().enumerate() {
            keys(m, &["usb"], &format!("matches[{i}]."), &mut out);
            keys(&m["usb"], USB_FIELDS, &format!("matches[{i}].usb."), &mut out);
        }
        for (i, u) in v["uses"].as_array().into_iter().flatten().enumerate() {
            if let Some(o) = u.as_object() {
                match USE_FIELDS.iter().find(|(kind, _)| o.contains_key(*kind)) {
                    Some((_, known)) => keys(u, known, &format!("uses[{i}]."), &mut out),
                    None => out.push(format!("uses[{i}]")),
                }
            }
        }
        for (i, s) in v["systems"].as_array().into_iter().flatten().enumerate() {
            if !s.as_str().is_some_and(|s| SYSTEMS.contains(&s)) {
                out.push(format!("systems[{i}] {s}"));
            }
        }
        out
    }

    /// Which the package is, given or as its manifest reads.
    pub fn kind(&self) -> Kind {
        self.kind_given.unwrap_or(if self.matches.is_empty() { Kind::Plugin } else { Kind::Core })
    }

    pub fn adds(&self, a: Adds) -> bool {
        self.adds.contains(&a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal() -> Value {
        json!({ "id": "x", "name": "X", "version": "1.0.0", "protocol": 4, "ectomatter": "0.1.0", "module": "core.wasm" })
    }

    #[test]
    fn a_manifest_reads_with_only_what_it_must_give() {
        let m = Manifest::of(&minimal()).unwrap();
        assert_eq!(m.kind(), Kind::Plugin, "no matches: a plugin");
        assert!(m.uses.is_empty() && m.adds.is_empty() && m.systems.is_none());
    }

    /// docs/core-protocol.md, section 9: "Within a version, new optional
    /// fields may appear and must be ignored by anyone who does not know
    /// them." They're read past, and named.
    #[test]
    fn a_field_the_protocol_doesnt_name_is_passed_over_and_named() {
        let mut v = minimal();
        v["deviceFiel"] = json!("device.json");
        v["matches"] = json!([{ "usb": { "vendor": "3842", "product": "2611", "usage-page": "0008" } }]);
        v["uses"] = json!([{ "web": "api.open-meteo.com", "too": "Open-Meteo" }]);
        v["systems"] = json!(["linx"]);
        let m = Manifest::of(&v).expect("read past");
        assert_eq!(m.matches.len(), 1);
        assert_eq!(Manifest::unknown(&v), vec!["deviceFiel", "matches[0].usb.usage-page", "uses[0].too", "systems[0] \"linx\""]);
        assert!(Manifest::unknown(&minimal()).is_empty());
    }

    #[test]
    fn a_use_the_protocol_doesnt_name_is_an_error() {
        let mut v = minimal();
        v["uses"] = json!(["monitor"]);
        assert!(Manifest::of(&v).is_err(), "a word");
        let mut v = minimal();
        v["uses"] = json!([{ "webb": "api.open-meteo.com" }]);
        assert!(Manifest::of(&v).is_err(), "an object");
    }

    /// The field lists name every field the manifest reads, and only them.
    #[test]
    fn the_field_lists_are_the_manifests() {
        let full = Manifest {
            id: "x".into(),
            kind_given: Some(Kind::Core),
            name: "X".into(),
            version: "1".into(),
            protocol: Some(4),
            ectomatter: Some("0.1.0".into()),
            module: "m".into(),
            license: Some("MIT".into()),
            description: Some("d".into()),
            glyph: Some("g".into()),
            repository: Some("a/b".into()),
            device_file: Some("f".into()),
            devices: Some(vec![]),
            matches: vec![Match { usb: Usb { vendor: "1".into(), product: "2".into(), interface: Some(1), usage_page: Some("3".into()), usage: Some("4".into()) } }],
            uses: vec![],
            adds: vec![],
            systems: Some(vec![]),
        };
        let v = serde_json::to_value(&full).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        let mut fields = FIELDS.to_vec();
        fields.sort();
        assert_eq!(keys, fields);
        let mut usb: Vec<&str> = v["matches"][0]["usb"].as_object().unwrap().keys().map(String::as_str).collect();
        usb.sort();
        let mut want = USB_FIELDS.to_vec();
        want.sort();
        assert_eq!(usb, want);
    }

    #[test]
    fn every_kind_of_use_reads() {
        let mut v = minimal();
        v["uses"] = json!(["monitors", "nvidia", "sound-capture", { "network": "127.0.0.1:6742", "to": "OpenRGB" }, { "web": "api.open-meteo.com", "to": "Open-Meteo" }, { "read": "/sys" }, { "launch": { "tools": ["winecfg"] } }]);
        v["adds"] = json!(["game", "settings", "readings"]);
        let m = Manifest::of(&v).unwrap();
        assert_eq!(m.uses.len(), 7);
        assert!(matches!(&m.uses[3], Use::Network { network, .. } if network == "127.0.0.1:6742"));
        assert!(m.adds(Adds::Settings));
    }
}
