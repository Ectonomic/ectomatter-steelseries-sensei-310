//! Text for a text display (section 5, displays): each row spelled in the
//! characters the display draws, which its description lists (`draws`), the
//! rest folded into those: accents dropped, Cyrillic and Greek spelled in
//! Latin letters, typographic punctuation made plain (data/fold.json), and
//! anything else a question mark. A core spells what it sends this way, and
//! Ectomatter what it shows of the display, so the two always agree.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

/// The characters a display draws: each a run from its first to its last.
#[derive(Clone, Debug, PartialEq)]
pub struct Draws(Vec<(char, char)>);

impl Draws {
    /// As a display's description gives them: `"draws": [" ~", "°", "█"]`,
    /// each a character, or two for the run between them. None given, the
    /// printable ASCII characters.
    pub fn of(v: &Value) -> Draws {
        let runs: Vec<(char, char)> = v
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| {
                let mut c = r.as_str()?.chars();
                let first = c.next()?;
                Some((first, c.next().unwrap_or(first)))
            })
            .collect();
        if runs.is_empty() { Draws(vec![(' ', '~')]) } else { Draws(runs) }
    }

    pub fn draws(&self, c: char) -> bool {
        self.0.iter().any(|(a, b)| (*a..=*b).contains(&c))
    }

    /// `s` in the characters the display draws.
    pub fn spell(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            if self.draws(c) {
                out.push(c);
            } else if let Some(r) = fold().get(&c).filter(|r| r.chars().all(|c| self.draws(c))) {
                out.push_str(r);
            } else {
                out.push(if self.draws('?') { '?' } else { ' ' });
            }
        }
        out
    }
}

/// What each character a display may not draw is spelled as.
fn fold() -> &'static HashMap<char, String> {
    static FOLD: OnceLock<HashMap<char, String>> = OnceLock::new();
    FOLD.get_or_init(|| {
        let m: serde_json::Map<String, Value> = serde_json::from_str(include_str!("../data/fold.json")).unwrap_or_default();
        m.into_iter().filter_map(|(k, v)| Some((k.chars().next()?, v.as_str()?.to_string()))).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A display spells what it doesn't draw in what it does: the Z10's
    /// screen keeps its degree sign and block, folds accents and Cyrillic, and
    /// marks the rest; a display drawing less folds the same text further.
    #[test]
    fn text_is_spelled_in_what_the_display_draws() {
        let z10 = Draws::of(&json!([" ~", "°", "█"]));
        assert_eq!(z10.spell("Café 21°C █"), "Cafe 21°C █");
        assert_eq!(z10.spell("Москва"), "Moskva");
        assert_eq!(z10.spell("“quoted” – £5"), "\"quoted\" - GBP5");
        assert_eq!(z10.spell("☃"), "?");
        let plain = Draws::of(&Value::Null);
        assert_eq!(plain.spell("21°C █"), "21?C ?");
        assert!(!plain.draws('°') && z10.draws('°'));
    }
}
