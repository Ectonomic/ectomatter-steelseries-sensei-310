//! SteelSeries' core: the Sensei 310, through the USB connection Ectomatter
//! lends it, speaking the core protocol's shared methods (docs/core-protocol.md,
//! section 5): keys, lighting and settings.
//!
//! The mouse takes settings but tells back only its sensitivity presets and
//! its lights' colors (and its firmware). So the core keeps its own record of
//! everything it has set, in its folder (/data), marked by what came from the
//! mouse and what is only assumed (the factory's, until something is set).
//! Meeting the mouse for the first time writes nothing: only a change does.
//! A change is sent at once, and kept in the mouse's memory once changes stop
//! for a moment, so a run of changes doesn't wear that memory.

mod protocol;

use std::cell::RefCell;
use std::time::{Duration, Instant};

use protocol::{Job, Stop};
use serde_json::{Map, Value, json};

wit_bindgen::generate!({ path: "../../../wit", world: "core" });

use ectomatter::core::{host, usb};

/// the timers: looking for the mouse coming and going, and keeping changes in its memory
const LOOK: u32 = 1;
const KEEP: u32 = 2;
/// the timer that listens for buttons given one of Ectomatter's host slots,
/// and every button while Ectomatter watches every press
const HEAR: u32 = 3;
/// how often it listens, in milliseconds: a press heard within a frame, and
/// every 32 once nothing has come for two seconds (as the Z10's core listens)
const HEAR_EVERY: u32 = 8;
const HEAR_QUIET: u32 = 32;
const QUIET_AFTER: Duration = Duration::from_secs(2);

/// The codes the USB standard reserves that a button given a host slot
/// sends, one a slot (as the Z10's core gives its keys): no key sends one,
/// and the system types nothing for one (Linux reports it as KEY_UNKNOWN,
/// which keymaps leave without a symbol), so the core hears them in the
/// mouse's own key reports. One slot a button.
const HOST_FIRST: u8 = 0xA5;
const HOST_SLOTS: u8 = 8;
const LOOK_EVERY: u32 = 3000;
const KEEP_AFTER: u32 = 1500;
/// rivalcfg leaves this long between commands (sending them faster can hang
/// some SteelSeries mice), and so does this core
const BETWEEN: Duration = Duration::from_millis(50);
const DEVICE: &str = "sensei310";
const RECORD: &str = "/data/sensei-310.json";

/// The buttons, in the mouse's order (its button 1 to 8, which are also the
/// numbers of their pieces in the art), with what each does from the factory.
const BUTTONS: [(&str, &str); 8] = [
    ("left", "Left button"),
    ("right", "Right button"),
    ("wheel-click", "Wheel click"),
    ("left-side-back", "Left side, back"),
    ("left-side-front", "Left side, front"),
    // (which of 6 and 7 is in front is not yet confirmed on the mouse)
    ("right-side-back", "Right side, back"),
    ("right-side-front", "Right side, front"),
    ("cpi", "CPI button"),
];

/// Each button's short name, for where room is tight (the cap beside it in a
/// list of the buttons), and the group a list puts it in, by where it sits.
const PLACES: [(&str, &str); 8] = [
    ("L", "Top buttons"),
    ("R", "Top buttons"),
    ("W", "Top buttons"),
    ("LB", "Side buttons"),
    ("LF", "Side buttons"),
    ("RB", "Side buttons"),
    ("RF", "Side buttons"),
    ("CPI", "Top buttons"),
];

/// Each button's surfaces in the drawing, by their ids there (Grant's
/// drawing names them its own way), for the frontend to pick the button out.
const PIECES: [&[&str]; 8] = [
    &["path20", "left-middle", "path21"],
    &["path22", "right-middle", "path23"],
    &["rect23"],
    &["path26"],
    &["path25"],
    &["path28"],
    &["path27"],
    &["rect24"],
];

/// Each button's callout on the mouse's page, as its mock draws it
/// (mock/sensei-310.html, a canvas 1092 by 470 with the mouse at 480, 30,
/// 240 by 410): its side, how far down its words are, and the line from
/// them to its dot on the button.
const CALLOUTS: [(&str, f32, &[(f32, f32)]); 8] = [
    ("left", 70.0, &[(424.0, 96.0), (538.0, 96.0), (552.0, 110.0)]),
    ("right", 70.0, &[(776.0, 96.0), (662.0, 96.0), (648.0, 110.0)]),
    ("left", 128.0, &[(424.0, 154.0), (565.0, 154.0), (599.0, 118.0)]),
    ("left", 245.0, &[(424.0, 271.0), (486.0, 271.0), (499.0, 260.0)]),
    ("left", 193.0, &[(424.0, 219.0), (490.0, 219.0), (501.0, 200.0)]),
    ("right", 245.0, &[(776.0, 271.0), (714.0, 271.0), (701.0, 260.0)]),
    ("right", 193.0, &[(776.0, 219.0), (710.0, 219.0), (699.0, 200.0)]),
    ("right", 128.0, &[(776.0, 154.0), (640.0, 154.0), (601.0, 186.0)]),
];

fn factory_jobs() -> [Value; 8] {
    [
        json!({ "mouse": 1 }),
        json!({ "mouse": 2 }),
        json!({ "mouse": 3 }),
        json!({ "mouse": 4 }),
        json!({ "mouse": 5 }),
        json!({ "key": "PageDown" }),
        json!({ "key": "PageUp" }),
        json!({ "sensitivity": "switch" }),
    ]
}

/// The mouse, opened, and what the core knows of it.
struct Mouse {
    dev: usb::HidDevice,
    path: String,
    firmware: Option<String>,
    last: Option<Instant>,
    /// its key reports (its third interface), read for the buttons given a
    /// host slot, and for every button while Ectomatter watches: the keys
    /// and the media keys down in the last of each
    keys: Option<usb::HidDevice>,
    down: Vec<u8>,
    media: Vec<u16>,
    /// its button reports (its second interface, the pointer's), read while
    /// Ectomatter watches every press, and the buttons down in the last one
    /// (bit 0 the mouse's button 1)
    pointer: Option<usb::HidDevice>,
    pressed: u8,
    /// when a report last came, to listen less often while none do
    heard: Instant,
}

thread_local! {
    static MOUSE: RefCell<Option<Mouse>> = const { RefCell::new(None) };
    /// a change sent and not yet kept in the mouse's memory
    static WAITING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// the lights showing colors the frontend streams (`lighting/direct`):
    /// the mouse's memory is kept from them, and nothing is kept till they go
    static DIRECT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// whether any button is given a host slot, as the record last said, and
    /// whether Ectomatter watches every press (`input/watch`)
    static HOSTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn color(v: &Value) -> Result<[u8; 3], String> {
    let s = v.as_str().ok_or("a color is written #RRGGBB")?.trim_start_matches('#');
    let n = u32::from_str_radix(s, 16).ok().filter(|_| s.len() == 6).ok_or("a color is written #RRGGBB")?;
    Ok([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

// ---------------------------------------------------------------- the record

/// What the core has set, or knows, of the mouse; `known` says which parts
/// came from the mouse itself.
fn record() -> Value {
    std::fs::read_to_string(RECORD).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null)
}

/// What's known of a light: `true` all of it (Ectomatter set it), "colors" its
/// color alone (the mouse says it), "none" nothing. A record from before
/// kept one word for both lights.
fn light_known(r: &Value, light: &str) -> Value {
    let k = &r["known"]["lights"];
    if k.is_object() {
        k[light].clone()
    } else {
        k.clone()
    }
}

fn set_light_known(r: &mut Value, light: &str, v: Value) {
    if !r["known"]["lights"].is_object() {
        let was = r["known"]["lights"].clone();
        r["known"]["lights"] = json!({ "logo": was, "wheel": was });
    }
    r["known"]["lights"][light] = v;
}

fn keep_record(r: &Value) {
    let _ = std::fs::write(RECORD, serde_json::to_string_pretty(r).unwrap_or_default());
}

/// The record when the mouse is first met: what its settings read says, and
/// the factory's for the rest.
fn first_record(s: Option<&protocol::Settings>) -> Value {
    let color = |i: usize| s.map_or("#FF1000".to_string(), |s| hex(s.colors[i]));
    json!({
        "buttons": factory_jobs(),
        "lights": {
            "logo": { "effect": "static", "params": { "color": color(0) } },
            "wheel": { "effect": "static", "params": { "color": color(1) } },
        },
        "settings": {
            "cpi1": s.map_or(800, |s| s.cpi[0]),
            "cpi2": s.map_or(1600, |s| s.cpi[1]),
            "polling": 1000,
        },
        "active preset": s.map(|s| s.active),
        "known": {
            "buttons": false,
            "lights": { "logo": if s.is_some() { "colors" } else { "none" }, "wheel": if s.is_some() { "colors" } else { "none" } },
            "cpi": s.is_some(),
            "polling": false,
        },
    })
}

// ---------------------------------------------------------------- talking to the mouse

/// Send one command, no sooner than `BETWEEN` after the last.
fn send(m: &mut Mouse, report: &[u8], feature: bool) -> Result<(), String> {
    if let Some(t) = m.last {
        let since = t.elapsed();
        if since < BETWEEN {
            std::thread::sleep(BETWEEN - since);
        }
    }
    let r = if feature { m.dev.send_feature(report) } else { m.dev.write(report).map(|_| ()) };
    m.last = Some(Instant::now());
    r.map_err(|e| format!("The mouse didn't take the change: {e}"))
}

/// Ask the mouse something and read its answer, after dropping any answer left waiting.
fn ask(m: &mut Mouse, report: &[u8]) -> Option<Vec<u8>> {
    while m.dev.read(0).is_ok_and(|r| !r.is_empty()) {}
    send(m, report, false).ok()?;
    m.dev.read(200).ok().filter(|r| !r.is_empty())
}

fn find() -> Option<usb::DeviceInfo> {
    usb::devices()
        .into_iter()
        .find(|d| d.vendor == protocol::VENDOR && d.product == protocol::PRODUCT && (d.usage_page == protocol::USAGE_PAGE || d.interface == protocol::INTERFACE))
}

/// Open the mouse and learn what it can say; tell Ectomatter it has come.
fn meet(info: usb::DeviceInfo) {
    let Ok(dev) = usb::open(&info) else { return };
    // (its key reports, a keyboard's: where a button given a host slot is heard)
    let keys = usb::devices()
        .into_iter()
        .find(|d| d.vendor == protocol::VENDOR && d.product == protocol::PRODUCT && ((d.usage_page == 1 && d.usage == 6) || d.interface == 2))
        .and_then(|d| usb::open(&d).ok());
    let pointer = if ALL.get() { open_pointer() } else { None };
    let mut m = Mouse { dev, path: info.path.clone(), firmware: None, last: None, keys, down: Vec::new(), media: Vec::new(), pointer, pressed: 0, heard: Instant::now() };
    m.firmware = ask(&mut m, &protocol::firmware()).and_then(|a| protocol::firmware_version(&a));
    let read = ask(&mut m, &protocol::read_settings()).and_then(|a| protocol::parse_settings(&a));
    let mut r = record();
    if r.is_null() {
        r = first_record(read.as_ref());
    } else if let Some(s) = &read {
        // what the mouse says over what was assumed: its presets and colors
        // may have been changed by another program since
        r["settings"]["cpi1"] = json!(s.cpi[0]);
        r["settings"]["cpi2"] = json!(s.cpi[1]);
        r["active preset"] = json!(s.active);
        r["known"]["cpi"] = json!(true);
        // and its lights' colors: a steady light showing another color than
        // the record's was changed by something else since, and a light never
        // read has its color now, so only its color is known
        for (i, light) in ["logo", "wheel"].into_iter().enumerate() {
            let said = hex(s.colors[i]);
            let l = &r["lights"][light];
            let other = l["effect"] == "static" && !l["params"]["color"].as_str().is_some_and(|c| c.eq_ignore_ascii_case(&said));
            let unread = matches!(light_known(&r, light).as_str(), Some("none")) || light_known(&r, light) == json!(false);
            if other || unread {
                r["lights"][light]["params"]["color"] = json!(said);
                set_light_known(&mut r, light, json!("colors"));
            }
        }
        host::log(&format!("the mouse says its logo is {} and its wheel {}", hex(s.colors[0]), hex(s.colors[1])));
    }
    keep_record(&r);
    let firmware = m.firmware.clone();
    MOUSE.with(|c| *c.borrow_mut() = Some(m));
    host::notify("device/added", &describe(firmware.as_deref()).to_string());
    hear_if_hosted(&r);
}

/// Listen for the buttons given a host slot while any is (`r`, the record),
/// and for every button while Ectomatter watches.
fn hear_if_hosted(r: &Value) {
    HOSTED.set((0..BUTTONS.len()).any(|i| r["buttons"][i].get("host").is_some()));
    listen();
}

/// The next listen, while there is anything to listen for and the mouse is
/// here: in 8 ms while reports come, in 32 once none have for two seconds.
fn listen() {
    if !(HOSTED.get() || ALL.get()) {
        return;
    }
    let Some(quiet) = MOUSE.with(|c| c.borrow().as_ref().map(|m| m.heard.elapsed() >= QUIET_AFTER)) else { return };
    host::after(if quiet { HEAR_QUIET } else { HEAR_EVERY }, HEAR);
}

/// What a press sent, as the mouse's reports say it.
#[derive(Clone, Copy, PartialEq)]
enum Sent {
    /// one of the mouse's own buttons, 1 to 8
    Button(u8),
    /// a keyboard key, by its usage (a host slot's reserved code among them)
    Key(u8),
    /// a media key, by its consumer usage
    Media(u16),
}

/// The mouse's button reports, the pointer's, opened: read beside the
/// computer, which still gets every one.
fn open_pointer() -> Option<usb::HidDevice> {
    usb::devices()
        .into_iter()
        .find(|d| d.vendor == protocol::VENDOR && d.product == protocol::PRODUCT && ((d.usage_page == 1 && d.usage == 2) || d.interface == 1))
        .and_then(|d| usb::open(&d).ok())
}

/// The mouse's reports since last time, read beside the computer, which
/// still gets every one: each host slot's button pressed or let go, and while
/// Ectomatter watches, every other button too, found by what its job sends.
/// Each is told to Ectomatter (`input/down`, `input/up`) by the button's id,
/// with what it sent as its job is written (`sends`), and for a host slot the
/// slot's number.
fn hear() {
    let all = ALL.get();
    let told = MOUSE.with(|c| {
        let mut c = c.borrow_mut();
        let Some(m) = c.as_mut() else { return Vec::new() };
        let mut told: Vec<(bool, Sent)> = Vec::new();
        if let Some(keys) = &m.keys {
            while let Ok(rep) = keys.read(0) {
                if rep.is_empty() {
                    break;
                }
                m.heard = Instant::now();
                // (report 1: the modifiers, a byte kept empty, then six keys;
                // report 2: two media keys)
                if rep[0] == 1 && rep.len() >= 9 {
                    let now: Vec<u8> = rep[3..9].iter().copied().filter(|k| *k != 0).collect();
                    told.extend(now.iter().filter(|k| !m.down.contains(k)).map(|k| (true, Sent::Key(*k))));
                    told.extend(m.down.iter().filter(|k| !now.contains(k)).map(|k| (false, Sent::Key(*k))));
                    m.down = now;
                } else if rep[0] == 2 && rep.len() >= 5 {
                    let now: Vec<u16> = [u16::from_le_bytes([rep[1], rep[2]]), u16::from_le_bytes([rep[3], rep[4]])].into_iter().filter(|u| *u != 0).collect();
                    told.extend(now.iter().filter(|u| !m.media.contains(u)).map(|u| (true, Sent::Media(*u))));
                    told.extend(m.media.iter().filter(|u| !now.contains(u)).map(|u| (false, Sent::Media(*u))));
                    m.media = now;
                }
            }
        }
        // (the pointer's report: its buttons, a bit each, then how far it moved)
        if let Some(pointer) = &m.pointer {
            while let Ok(rep) = pointer.read(0) {
                if rep.is_empty() {
                    break;
                }
                m.heard = Instant::now();
                for b in 0..8u8 {
                    if (rep[0] ^ m.pressed) & (1 << b) != 0 {
                        told.push((rep[0] & (1 << b) != 0, Sent::Button(b + 1)));
                    }
                }
                m.pressed = rep[0];
            }
        }
        told
    });
    let slot = |s: Sent| match s {
        Sent::Key(k) if (HOST_FIRST..HOST_FIRST + HOST_SLOTS).contains(&k) => Some(k - HOST_FIRST),
        _ => None,
    };
    let told: Vec<(bool, Sent)> = told.into_iter().filter(|(_, s)| all || slot(*s).is_some()).collect();
    if !told.is_empty() {
        let r = record();
        for (down, s) in told {
            // the button whose job sends it (the first, where two send the same)
            let i = (0..BUTTONS.len()).find(|&i| match (s, job(&r["buttons"][i])) {
                _ if slot(s).is_some() => r["buttons"][i]["host"].as_u64() == slot(s).map(u64::from),
                (Sent::Button(n), Ok(Job::Mouse(b))) => b == n,
                (Sent::Key(k), Ok(Job::Key(u))) => u == k,
                // (a key with modifiers held, by its key)
                (Sent::Key(k), Ok(Job::Chord(b))) => b.iter().any(|u| *u == k && !(0xE0..=0xE7).contains(u)),
                (Sent::Media(u), Ok(Job::Media(v))) => u16::from(v) == u,
                _ => false,
            });
            if let Some(i) = i {
                let mut p = json!({ "device": DEVICE, "control": BUTTONS[i].0, "sends": r["buttons"][i] });
                if let Some(n) = slot(s) {
                    p["host"] = json!(n);
                }
                host::notify(if down { "input/down" } else { "input/up" }, &p.to_string());
            }
        }
    }
    listen();
}

/// The mouse as the core protocol describes a device (section 4).
fn describe(firmware: Option<&str>) -> Value {
    let controls: Vec<Value> = BUTTONS
        .iter()
        .zip(PLACES)
        .enumerate()
        .map(|(i, ((id, label), (short, group)))| {
            let (side, at, line) = CALLOUTS[i];
            json!({ "id": id, "kind": "button", "label": label, "short": short, "group": group, "art": i + 1, "pieces": PIECES[i],
                "callout": { "side": side, "at": at, "line": line.iter().map(|(x, y)| [x, y]).collect::<Vec<_>>() } })
        })
        .collect();
    let speed = |default: u32| json!({ "type": "duration", "min": 500, "max": 30000, "label": "Speed", "default": default });
    let speed_in = |default: u32, min: u32, max: u32| json!({ "type": "duration", "min": min, "max": max, "label": "Speed", "default": default });
    let color = json!({ "type": "color", "label": "Color", "default": "#FF1000" });
    // (whether a moving effect plays round and round, or once at each press
    // of a button, as SteelSeries Engine's Trigger: see `cycle`)
    let plays = json!({
        "type": "choice",
        "label": "Plays",
        "default": "always",
        "options": [{ "value": "always", "label": "All the time" }, { "value": "press", "label": "Once per press" }],
    });
    // (each param's default is what the mouse plays while it has none;
    // Color shift starts as SteelSeries GG's own: red, green and blue, a
    // second around)
    let effects = json!([
        { "id": "off", "label": "Off", "note": "The light stays dark.", "params": {} },
        { "id": "static", "label": "Static", "note": "One color, steady.", "params": { "color": color } },
        { "id": "breathing", "label": "Breathing", "note": "The color fades in and out.", "params": { "color": color, "speed": speed(3000), "plays": plays } },
        {
            "id": "colorbreathing",
            "label": "Color breathing",
            "note": "Each color below fades in and out in turn.",
            "params": {
                "colors": { "type": "colors", "max": BREATH_COLORS, "label": "Colors", "default": GG_COLORS },
                "speed": speed(9000),
                "plays": plays,
            },
        },
        {
            "id": "colorshift",
            "label": "Color shift",
            "note": "The colors below play in turn, each blending into the next.",
            "params": {
                "colors": { "type": "colors", "max": MAX_COLORS, "label": "Colors", "default": ["#FF0000", "#FFFF00", "#00FF00", "#00FFFF", "#0000FF", "#FF00FF"] },
                "speed": speed(5000),
                "plays": plays,
            },
            "start": { "colors": GG_COLORS, "speed": 1000 },
            // (it looks like the keyboard's Wave, for its picture on the page)
            "like": "wave",
        },
        { "id": "heartbeat", "label": "Heartbeat", "note": "Two quick pulses, then a rest.", "params": { "color": color, "speed": speed_in(1200, 500, 3000), "plays": plays } },
        // (a cycle is one flash, though the mouse is sent three to a cycle: see `cycle`)
        { "id": "strobe", "label": "Strobe", "note": "The color flashes on and off.", "params": { "color": color, "speed": speed_in(300, 200, 2000), "plays": plays } },
        {
            "id": "flicker",
            "label": "Flicker",
            "note": "The color flickers like a flame.",
            "params": { "color": { "type": "color", "label": "Color", "default": FLAME }, "speed": speed_in(3000, 1000, 10000), "plays": plays },
        },
        // (played by the mouse itself, at a press of any of its buttons)
        {
            "id": "reactive",
            "label": "Reactive",
            "note": "The light flashes when you press a button, then fades back to its background color.",
            "reactive": true,
            "like": "trigger",
            "params": {
                "color": { "type": "color", "label": "Flash", "default": "#FF1000" },
                "background": { "type": "color", "label": "Background", "default": "#000000" },
                "fade": { "type": "duration", "min": 500, "max": 3000, "label": "Fade", "default": 800 },
            },
        },
    ]);
    json!({
        "device": DEVICE,
        "kind": "mouse",
        "name": "SteelSeries Sensei 310",
        "vendor": "SteelSeries",
        "model": "Sensei 310",
        "firmware": { "kind": "retail", "version": firmware },
        "art": "art/sensei-310.svg",
        // (the part of the drawing the mouse fills, x, y, width, height)
        "view": [28, 30, 304, 520],
        // (how its lights are made, for the frontend to light them: the Key lab tunes it)
        "light": "light.json",
        // the drawing's units to the millimeter: its mouse, 520 units long, is 125.1 mm
        "scale": 520.0 / 125.1,
        "controls": controls,
        "capabilities": {
            // (the mouse keeps one setup in its memory: Ectomatter keeps its profiles
            // and puts the one in use on it, profiles/apply)
            "profiles": { "stored": "host", "factory": true },
            "keys": {
                "stored": "device",
                "readable": false,
                "layers": [{ "id": "base", "label": "When pressed" }],
                "actions": ["mouse", "key", "media", "sensitivity", "scroll", "host", "off"],
                "host": { "slots": HOST_SLOTS },
                "factory": true,
            },
            // (every press told while Ectomatter watches, input/watch)
            "input": { "watch": true },
            "lighting": {
                "stored": "device",
                "readable": "colors",
                // (as they are on the mouse, front to back: Ectomatter lists them so)
                "lights": [
                    { "id": "wheel", "label": "Wheel", "art": [3, 203] },
                    // (the drawing's own glow round the logo on the palm, 9, drawn before
                    // the light was simulated: no part of the logo's window)
                    { "id": "logo", "label": "Logo", "art": [9, 209], "glow": 9 },
                ],
                "effects": effects,
                "preview": false,
                // (each light's color set by the frontend, as fast as the mouse takes two commands)
                "direct": { "lights": ["logo", "wheel"], "rate": 8 },
            },
            "settings": {
                "stored": "device",
                "items": [
                    { "id": "cpi1", "type": "int", "min": protocol::CPI_MIN, "max": protocol::CPI_MAX, "step": 100, "unit": "CPI", "label": "First sensitivity", "short": "First", "preset": 1 },
                    { "id": "cpi2", "type": "int", "min": protocol::CPI_MIN, "max": protocol::CPI_MAX, "step": 100, "unit": "CPI", "label": "Second sensitivity", "short": "Second", "preset": 2 },
                    { "id": "polling", "type": "choice", "options": protocol::POLLING, "unit": "Hz", "label": "Polling rate" },
                ],
            },
        },
        "layout": {
            "device": [
                { "section": "Sensitivity", "show": ["cpi1", "cpi2"], "note": "From 100 to 12,000 CPI, in steps of 100. The CPI button behind the wheel switches between the two." },
                { "section": "Polling rate", "show": ["polling"], "note": "How many times a second the mouse tells the computer where it is. 1000 Hz follows your hand most closely." },
            ],
        },
    })
}

/// Look for the mouse coming or going.
fn look() {
    let here = find();
    let had = MOUSE.with(|c| c.borrow().as_ref().map(|m| m.path.clone()));
    match (had, here) {
        (None, Some(info)) => meet(info),
        (Some(path), here) if here.as_ref().is_none_or(|i| i.path != path) => {
            MOUSE.with(|c| *c.borrow_mut() = None);
            host::notify("device/removed", &json!({ "device": DEVICE }).to_string());
            if let Some(info) = here {
                meet(info);
            }
        }
        _ => {}
    }
    host::after(LOOK_EVERY, LOOK);
}

fn with_mouse<R>(f: impl FnOnce(&mut Mouse) -> Result<R, String>) -> Result<R, String> {
    MOUSE.with(|c| match c.borrow_mut().as_mut() {
        Some(m) => f(m),
        None => Err("The mouse isn't plugged in.".into()),
    })
}

// ---------------------------------------------------------------- jobs, lights, settings

fn job(a: &Value) -> Result<Job, String> {
    if let Some(n) = a.get("mouse").and_then(Value::as_u64) {
        return (1..=8).contains(&n).then_some(Job::Mouse(n as u8)).ok_or_else(|| "A mouse button is 1 to 8.".into());
    }
    if let Some(k) = a.get("key") {
        let u = k.as_u64().map(|n| n as u8).or_else(|| k.as_str().and_then(protocol::key_usage));
        return u.map(Job::Key).ok_or_else(|| format!("The mouse can't send the key {k}."));
    }
    // (a key with modifiers held: Ctrl+T)
    if let Some(list) = a.get("shortcut").and_then(Value::as_array) {
        let codes: Option<Vec<u8>> = list.iter().map(|k| k.as_u64().map(|n| n as u8).or_else(|| k.as_str().and_then(protocol::key_usage))).collect();
        let codes = codes.ok_or_else(|| format!("The mouse can't send {a}."))?;
        let (mut mods, keys): (Vec<u8>, Vec<u8>) = codes.into_iter().partition(|u| (0xE0..=0xE7).contains(u));
        mods.sort_unstable();
        mods.dedup();
        if keys.len() != 1 || mods.len() > 3 {
            return Err("The mouse can send one key with up to three modifiers held.".into());
        }
        let mut b = [0u8; 4];
        b[..mods.len()].copy_from_slice(&mods);
        b[mods.len()] = keys[0];
        return Ok(Job::Chord(b));
    }
    if let Some(k) = a.get("media").and_then(Value::as_str) {
        return protocol::media_usage(k).map(Job::Media).ok_or_else(|| format!("The mouse can't send the media key {k}."));
    }
    if a.get("sensitivity").is_some() {
        return Ok(Job::Sensitivity);
    }
    match a.get("scroll").and_then(Value::as_str) {
        Some("up") => return Ok(Job::ScrollUp),
        Some("down") => return Ok(Job::ScrollDown),
        _ => {}
    }
    if a.get("off").is_some() {
        return Ok(Job::Off);
    }
    // (one of Ectomatter's host slots: a reserved key the core listens for)
    if let Some(n) = a.get("host").and_then(Value::as_u64) {
        return (n < HOST_SLOTS as u64).then(|| Job::Key(HOST_FIRST + n as u8)).ok_or_else(|| format!("The mouse has {HOST_SLOTS} host slots."));
    }
    Err(format!("The mouse can't do {a}."))
}

fn send_buttons(r: &Value) -> Result<(), String> {
    let jobs: Vec<Job> = (0..8).map(|i| job(&r["buttons"][i])).collect::<Result<_, _>>()?;
    let jobs: [Job; 8] = jobs.try_into().map_err(|_| "eight buttons")?;
    with_mouse(|m| send(m, &protocol::buttons(&jobs), false))
}

/// What a light plays, as it is sent to the mouse: its colors at points
/// through a cycle of so many milliseconds (ending on its first color again,
/// see protocol::closed), or one color held.
fn cycle(l: &Value) -> Result<(u16, Vec<Stop>, bool, u8), String> {
    let p = &l["params"];
    let speed_in = |d: u16, min: u64, max: u64| p["speed"].as_u64().map_or(d, |s| s.clamp(min, max) as u16);
    let speed = |d: u16| speed_in(d, 500, 30000);
    let dark = [0u8; 3];
    let stop = |at: u32, color: [u8; 3]| Stop { at: at.min(255) as u8, color };
    let (ms, stops, held) = match l["effect"].as_str().unwrap_or("static") {
        "off" => (1000, vec![Stop { at: 0, color: [0; 3] }], true),
        "static" => (1000, vec![Stop { at: 0, color: color(&p["color"])? }], true),
        // from dark to the color and back, round and round
        "breathing" => (speed(3000), vec![Stop { at: 0, color: [0; 3] }, Stop { at: 127, color: color(&p["color"])? }], false),
        // each color a breath of its own, from dark and back, in turn round the cycle
        "colorbreathing" => {
            let colors = colors_or(&p["colors"], BREATH_COLORS)?;
            let n = colors.len() as u32;
            let stops = colors.iter().enumerate().flat_map(|(i, c)| [stop(i as u32 * 255 / n, dark), stop(i as u32 * 255 / n + 255 / (2 * n), *c)]).collect();
            (speed(9000), stops, false)
        }
        // two beats, the second a little softer, then dark till the cycle ends
        "heartbeat" => {
            let c = color_or(&p["color"], [0xFF, 0x10, 0x00])?;
            let soft = c.map(|v| (v as u32 * 3 / 4) as u8);
            (speed_in(1200, 500, 3000), vec![stop(0, dark), stop(20, c), stop(50, dark), stop(75, soft), stop(120, dark)], false)
        }
        // three flashes a cycle, each lit for a quarter of its time and cut
        // sharp (its stops one apart): a cycle of one flash would be shorter
        // than any this core has sent the mouse (half a second)
        "strobe" => {
            let c = color_or(&p["color"], [0xFF, 0x10, 0x00])?;
            let every = speed_in(300, 200, 2000);
            let stops = (0..3u32).flat_map(|k| [stop(85 * k, c), stop(85 * k + 20, c), stop(85 * k + 21, dark), stop(85 * k + 84, dark)]).collect();
            (every * 3, stops, false)
        }
        // the color at uneven brightnesses at uneven points, back to full at the end
        "flicker" => {
            let c = color_or(&p["color"], FLAME_RGB)?;
            let stops = FLICKER.iter().map(|&(at, level)| stop(at, c.map(|v| (v as f32 * level).round() as u8))).collect();
            (speed_in(3000, 1000, 10000), stops, false)
        }
        // played once by the mouse at each press of any button, from the
        // flash to the background, which it rests on till the next
        "reactive" => {
            let fade = p["fade"].as_u64().map_or(800, |f| f.clamp(500, 3000)) as u16;
            (fade, vec![stop(0, color_or(&p["color"], [0xFF, 0x10, 0x00])?), stop(255, color_or(&p["background"], dark)?)], true)
        }
        // its own colors in turn, evenly round the cycle (SteelSeries GG's own:
        // red, green and blue, a second around); a light set to Color shift
        // before it had colors goes round the color wheel, as it always did
        "colorshift" => match p["colors"].as_array().filter(|a| !a.is_empty()) {
            Some(colors) => {
                let colors: Vec<[u8; 3]> = colors.iter().take(MAX_COLORS).map(color).collect::<Result<_, _>>()?;
                let n = colors.len();
                // (a cycle of 5 s when none is given, as before it had colors of its own)
                (speed(5000), colors.iter().enumerate().map(|(i, c)| Stop { at: (i * 255 / n) as u8, color: *c }).collect(), false)
            }
            // (the mouse blends straight between stops, so red to green alone
            // passes a dim olive; with yellow between them every step stays bright)
            None => (
                speed(5000),
                vec![
                    Stop { at: 0, color: [255, 0, 0] },
                    Stop { at: 43, color: [255, 255, 0] },
                    Stop { at: 85, color: [0, 255, 0] },
                    Stop { at: 128, color: [0, 255, 255] },
                    Stop { at: 170, color: [0, 0, 255] },
                    Stop { at: 213, color: [255, 0, 255] },
                ],
                false,
            ),
        },
        other => return Err(format!("The mouse has no effect called {other}.")),
    };
    // (a cycle played once at each press: sent closed, so it ends where it
    // starts and rests there, and to play once)
    let pressed = l["effect"] == "reactive" || (!held && p["plays"] == "press");
    Ok((ms, protocol::closed(&stops, held), held || pressed, if pressed { EVERY_BUTTON } else { 0 }))
}

/// The most colors a light's Color shift takes: the mouse holds 14 stops,
/// one of them the closing one, and Ectomatter's color lists hold seven.
const MAX_COLORS: usize = 7;

/// The most colors Color breathing takes: each is two stops (dark, then the
/// color), and the closing one makes thirteen.
const BREATH_COLORS: usize = 6;

/// Flicker's color while it has none: a flame's orange.
const FLAME: &str = "#FF6000";
const FLAME_RGB: [u8; 3] = [0xFF, 0x60, 0x00];

/// Flicker's points through its cycle and the brightness at each, uneven so
/// the cycle's repeat is hard to see (the closing stop, full, makes thirteen).
const FLICKER: [(u32, f32); 12] = [
    (0, 1.0),
    (17, 0.72),
    (31, 0.9),
    (52, 0.55),
    (70, 0.85),
    (96, 0.97),
    (115, 0.62),
    (139, 0.8),
    (160, 0.93),
    (184, 0.58),
    (205, 0.88),
    (230, 0.7),
];

/// A color param, or `d` where the effect was given none.
fn color_or(v: &Value, d: [u8; 3]) -> Result<[u8; 3], String> {
    if v.is_null() { Ok(d) } else { color(v) }
}

/// A colors param, at most `max`, or GG's red, green and blue where the
/// effect was given none.
fn colors_or(v: &Value, max: usize) -> Result<Vec<[u8; 3]>, String> {
    match v.as_array().filter(|a| !a.is_empty()) {
        Some(a) => a.iter().take(max).map(color).collect(),
        None => GG_COLORS.iter().map(|c| color(&json!(c))).collect(),
    }
}

/// SteelSeries GG's own Color shift for the Sensei 310 (GG 121.0.0's
/// database: type 1, 1000 ms, red at 0%, green at 33%, blue at 66%).
/// SteelSeries GG's Color shift colors: red, green and blue.
const GG_COLORS: [&str; 3] = ["#FF0000", "#00FF00", "#0000FF"];

fn gg_colorshift() -> Value {
    json!({ "effect": "colorshift", "params": { "colors": GG_COLORS, "speed": 1000 } })
}

/// The mouse as SteelSeries GG ships it, what a new profile starts as: each
/// button's own job, both lights in GG's Color shift, 800 and 1600 CPI, 1000 Hz.
fn factory_profile() -> Value {
    let base: Map<String, Value> = BUTTONS.iter().zip(factory_jobs()).map(|((id, _), j)| (id.to_string(), j)).collect();
    json!({
        "keys": { "base": base },
        "lighting": { "logo": gg_colorshift(), "wheel": gg_colorshift() },
        "settings": { "cpi1": 800, "cpi2": 1600, "polling": 1000 },
    })
}

/// A light's effect as the mouse plays it.
fn light_report(led: u8, l: &Value) -> Result<Vec<u8>, String> {
    let (ms, stops, held, triggers) = cycle(l)?;
    Ok(protocol::light(led, ms, &stops, held, triggers))
}

/// The buttons that start Reactive: all eight (which bit is which button
/// hasn't been tried, see protocol::light).
const EVERY_BUTTON: u8 = 0xFF;

/// What a light plays, for Ectomatter to show it as the mouse does: the
/// cycle's length, whether one color is held, and its colors at points
/// through the cycle (0 to 255), blended straight from each to the next.
fn plays(l: &Value) -> Value {
    let listed = |stops: &[Stop]| stops.iter().map(|s| json!({ "at": s.at, "color": hex(s.color) })).collect::<Vec<_>>();
    match cycle(l) {
        // (a cycle a press starts: its last color held between presses, and
        // the cycle played once from each)
        Ok((ms, stops, _, triggers)) if triggers != 0 => json!({
            "ms": 1000,
            "held": true,
            "stops": [{ "at": 0, "color": hex(stops[stops.len() - 1].color) }],
            "press": { "ms": ms, "stops": listed(&stops) },
        }),
        Ok((ms, stops, held, _)) => json!({ "ms": ms, "held": held, "stops": listed(&stops) }),
        Err(_) => Value::Null,
    }
}

/// A light as the record keeps it, with what it plays.
fn light_now(l: &Value) -> Value {
    let mut l = l.clone();
    if l.is_object() {
        l["plays"] = plays(&l);
    }
    l
}

/// Keep what was sent in the mouse's memory once changes stop for a moment.
fn keep_soon() {
    WAITING.set(true);
    host::after(KEEP_AFTER, KEEP);
}

fn answer(method: &str, p: &Value) -> Result<Value, String> {
    let mut r = record();
    let device = p["device"].as_str().unwrap_or(DEVICE);
    if method != "initialize" && method != "shutdown" && device != DEVICE {
        return Err(format!("This core has no device {device}."));
    }
    match method {
        "initialize" => {
            let met = ectomatter_protocol::meet(p, "SteelSeries", env!("CARGO_PKG_VERSION"))?;
            look();
            Ok(met)
        }
        "keys/get" => {
            let base: Map<String, Value> = BUTTONS.iter().enumerate().map(|(i, (id, _))| (id.to_string(), r["buttons"][i].clone())).collect();
            Ok(json!({ "layers": { "base": base }, "factory": factory_profile()["keys"], "known": r["known"]["buttons"] }))
        }
        // (what the mouse is, before it's met: as device/added says it, but its firmware)
        "device/describe" => Ok(describe(None)),
        "profiles/factory" => Ok(factory_profile()),
        // a whole profile Ectomatter keeps, put on the mouse: its buttons in one
        // report, then each light, then both sensitivities and the polling
        // rate, and kept in the mouse's memory once that's done
        "profiles/apply" => {
            if let Some(base) = p["keys"]["base"].as_object() {
                for (id, action) in base {
                    let i = BUTTONS.iter().position(|(b, _)| b == id).ok_or_else(|| format!("The mouse has no button {id}."))?;
                    job(action)?;
                    r["buttons"][i] = action.clone();
                }
                send_buttons(&r)?;
                r["known"]["buttons"] = json!(true);
            }
            let mut plays_now = Map::new();
            for (light, led) in [("logo", protocol::LOGO), ("wheel", protocol::WHEEL)] {
                let Some(l) = p["lighting"].get(light).filter(|l| l.is_object()) else { continue };
                let l = json!({ "effect": l["effect"], "params": l["params"] });
                let report = light_report(led, &l)?;
                with_mouse(|m| send(m, &report, true))?;
                plays_now.insert(light.into(), plays(&l));
                r["lights"][light] = l;
                set_light_known(&mut r, light, json!(true));
            }
            for (id, v) in p["settings"].as_object().into_iter().flatten() {
                let v = v.as_u64().ok_or("A setting's value is a number.")? as u16;
                let report = match id.as_str() {
                    "cpi1" | "cpi2" => protocol::cpi(if id == "cpi1" { 1 } else { 2 }, v.clamp(protocol::CPI_MIN, protocol::CPI_MAX)),
                    "polling" => protocol::polling(v).ok_or("The polling rate is 125, 250, 500 or 1000 Hz.")?,
                    _ => return Err(format!("The mouse has no setting {id}.")),
                };
                with_mouse(|m| send(m, &report, false))?;
                r["settings"][id] = json!(if id == "polling" { v } else { (v.clamp(protocol::CPI_MIN, protocol::CPI_MAX) + 50) / 100 * 100 });
                r["known"][if id == "polling" { "polling" } else { "cpi" }] = json!(true);
            }
            keep_record(&r);
            keep_soon();
            hear_if_hosted(&r);
            Ok(json!({ "plays": plays_now }))
        }
        // every press told from now on (`all`), or the host slots' alone
        "input/watch" => {
            let all = p["all"].as_bool().unwrap_or(false);
            ALL.set(all);
            // (the pointer's reports read only while watched, from the moment they are)
            let listening = MOUSE.with(|c| {
                let mut c = c.borrow_mut();
                let Some(m) = c.as_mut() else { return false };
                if !all {
                    m.pointer = None;
                } else if m.pointer.is_none() {
                    m.pointer = open_pointer();
                    m.pressed = 0;
                }
                m.keys.is_some() && (!all || m.pointer.is_some())
            });
            listen();
            Ok(json!({ "listening": listening }))
        }
        "keys/set" | "keys/reset" => {
            let control = p["control"].as_str().unwrap_or_default();
            let i = BUTTONS.iter().position(|(id, _)| *id == control).ok_or_else(|| format!("The mouse has no button {control}."))?;
            let action = if method == "keys/reset" { factory_jobs()[i].clone() } else { p["action"].clone() };
            job(&action)?;
            r["buttons"][i] = action;
            send_buttons(&r)?;
            r["known"]["buttons"] = json!(true);
            keep_record(&r);
            keep_soon();
            hear_if_hosted(&r);
            Ok(Value::Null)
        }
        "lighting/get" => Ok(json!({ "logo": light_now(&r["lights"]["logo"]), "wheel": light_now(&r["lights"]["wheel"]), "known": { "logo": light_known(&r, "logo"), "wheel": light_known(&r, "wheel") } })),
        "lighting/set" => {
            let light = p["light"].as_str().unwrap_or_default();
            let led = match light {
                "logo" => protocol::LOGO,
                "wheel" => protocol::WHEEL,
                _ => return Err(format!("The mouse has no light called {light}.")),
            };
            let l = json!({ "effect": p["effect"], "params": p["params"] });
            let report = light_report(led, &l)?;
            with_mouse(|m| send(m, &report, true))?;
            let now = plays(&l);
            r["lights"][light] = l;
            set_light_known(&mut r, light, json!(true));
            keep_record(&r);
            keep_soon();
            // what the light plays from now: the mouse starts its cycle as it takes the report
            Ok(json!({ "plays": now }))
        }
        // the lights in colors the frontend streams, each held, the mouse's
        // memory kept from them: a light given no color keeps what it shows
        "lighting/direct" => {
            DIRECT.set(true);
            for (light, led) in [("logo", protocol::LOGO), ("wheel", protocol::WHEEL)] {
                if let Some(c) = p["colors"].get(light).filter(|c| !c.is_null()) {
                    let report = light_report(led, &json!({ "effect": "static", "params": { "color": c } }))?;
                    with_mouse(|m| send(m, &report, true))?;
                }
            }
            Ok(Value::Null)
        }
        // the lights as the mouse keeps them again, after streamed colors;
        // what was changed meanwhile kept now
        "lighting/hand-back" => {
            if !DIRECT.replace(false) {
                return Ok(Value::Null);
            }
            let mut out = json!({});
            for (light, led) in [("logo", protocol::LOGO), ("wheel", protocol::WHEEL)] {
                let l = &r["lights"][light];
                let report = light_report(led, l)?;
                with_mouse(|m| send(m, &report, true))?;
                out[light] = plays(l);
            }
            if WAITING.get() {
                keep_soon();
            }
            Ok(json!({ "plays": out }))
        }
        "settings/get" => Ok(json!({ "values": r["settings"], "active": r["active preset"], "known": { "cpi1": r["known"]["cpi"], "cpi2": r["known"]["cpi"], "polling": r["known"]["polling"] } })),
        "settings/set" => {
            let id = p["id"].as_str().unwrap_or_default();
            let v = p["value"].as_u64().ok_or("A setting's value is a number.")? as u16;
            let report = match id {
                "cpi1" | "cpi2" => {
                    if !(protocol::CPI_MIN..=protocol::CPI_MAX).contains(&v) {
                        return Err(format!("Sensitivity goes from {} to {} CPI.", protocol::CPI_MIN, protocol::CPI_MAX));
                    }
                    protocol::cpi(if id == "cpi1" { 1 } else { 2 }, v)
                }
                "polling" => protocol::polling(v).ok_or("The polling rate is 125, 250, 500 or 1000 Hz.")?,
                _ => return Err(format!("The mouse has no setting {id}.")),
            };
            with_mouse(|m| send(m, &report, false))?;
            r["settings"][id] = json!(if id == "polling" { v } else { (v + 50) / 100 * 100 });
            r["known"][if id == "polling" { "polling" } else { "cpi" }] = json!(true);
            keep_record(&r);
            keep_soon();
            Ok(Value::Null)
        }
        "shutdown" => {
            // what is waiting to be kept, kept now; the mouse holds its own settings
            keep();
            Ok(Value::Null)
        }
        other => Err(format!("SteelSeries' core has no call named {other}.")),
    }
}

/// Keep a change in the mouse's memory, if one is waiting (never otherwise:
/// each save wears the mouse's memory a little).
fn keep() {
    // (not while the lights show streamed colors, which the mouse would keep:
    // kept once they're handed back)
    if DIRECT.get() {
        if WAITING.get() {
            host::after(KEEP_AFTER, KEEP);
        }
        return;
    }
    if WAITING.replace(false) {
        let _ = with_mouse(|m| send(m, &protocol::save(), false));
    }
}

struct Core;

impl Guest for Core {
    fn call(method: String, params: String) -> Result<String, String> {
        let p: Value = if params.is_empty() { Value::Null } else { serde_json::from_str(&params).map_err(|e| format!("{method}: {e}"))? };
        answer(&method, &p).map(|v| v.to_string())
    }

    fn timer(tag: u32) {
        match tag {
            LOOK => look(),
            KEEP => keep(),
            HEAR => hear(),
            _ => {}
        }
    }
}

export!(Core);
