//! The game in the player's language (since v0.1.15, the user's list of 2026-11-23: the game
//! translated into French, the player's language detected, the system's natively and the
//! browser's on the web, with English for everyone else). See docs/ui.md#languages.
//!
//! The English text is its own key: `tr("QUICK PLAY")` gives the French when the player reads
//! French and there is one in `assets/i18n/fr.ron`, the English otherwise. So a text nobody has
//! translated yet still shows, in English, and the code reads as the screen does. Texts with
//! numbers in them go through [`trf`], `{name}` in the key standing for each value.
//!
//! The language is chosen once, before anything is drawn, and read from anywhere: menus are built
//! in many places, and none of them should have to carry it around.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A language the game speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
}

impl Lang {
    /// The language for a locale or language tag such as `fr-FR`, `fr_CA.UTF-8` or `en-US`:
    /// French for any French, English for everything else.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if primary == "fr" { Lang::Fr } else { Lang::En }
    }
}

static LANG: OnceLock<Lang> = OnceLock::new();
static FRENCH: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

/// Chooses the language: `forced` (`--lang`, `?lang=`) if given, else the system's or the
/// browser's. Called once, at startup; later calls change nothing.
pub fn init(forced: Option<&str>) -> Lang {
    let lang = match forced {
        Some(tag) => Lang::from_tag(tag),
        None => sys_locale::get_locale().map_or(Lang::En, |tag| Lang::from_tag(&tag)),
    };
    *LANG.get_or_init(|| lang)
}

/// The language chosen, English until [`init`] has run (as in tests).
pub fn lang() -> Lang {
    LANG.get().copied().unwrap_or(Lang::En)
}

/// The French texts, keyed by their English, read once from the file built into the game.
fn french() -> &'static HashMap<&'static str, &'static str> {
    FRENCH.get_or_init(|| {
        let table: HashMap<String, String> =
            ron::from_str(include_str!("../assets/i18n/fr.ron")).unwrap_or_else(|error| {
                bevy::log::error!("the French texts cannot be read: {error}");
                HashMap::new()
            });
        table
            .into_iter()
            .map(|(english, french)| -> (&'static str, &'static str) {
                (english.leak(), french.leak())
            })
            .collect()
    })
}

/// `english` in the player's language.
pub fn tr(english: &str) -> &str {
    tr_in(lang(), english)
}

/// `english` in `lang`.
pub fn tr_in(lang: Lang, english: &str) -> &str {
    match lang {
        Lang::En => english,
        Lang::Fr => french().get(english).copied().unwrap_or(english),
    }
}

/// `english` in the player's language, each `{name}` in it replaced by its value.
// Used by the screens as they are translated (v0.1.15 and after).
#[allow(dead_code)]
pub fn trf(english: &str, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    fill(tr(english), values)
}

fn fill(template: &str, values: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut text = template.to_owned();
    for (name, value) in values {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_is_any_french() {
        for tag in ["fr", "fr-FR", "fr_CA.UTF-8", "FR-be", "fr@euro"] {
            assert_eq!(Lang::from_tag(tag), Lang::Fr, "{tag}");
        }
        for tag in ["en-US", "de-DE", "", "C", "POSIX", "frr"] {
            assert_eq!(Lang::from_tag(tag), Lang::En, "{tag}");
        }
    }

    #[test]
    fn english_is_the_key_and_the_fallback() {
        assert_eq!(tr_in(Lang::En, "QUICK PLAY"), "QUICK PLAY");
        assert_eq!(tr_in(Lang::Fr, "QUICK PLAY"), "PARTIE RAPIDE");
        assert_eq!(tr_in(Lang::Fr, "a text nobody translated"), "a text nobody translated");
    }

    #[test]
    fn values_fill_their_places() {
        let text = fill("LAP {lap}/{laps}", &[("lap", &2), ("laps", &3)]);
        assert_eq!(text, "LAP 2/3");
    }

    /// Every French text keeps the places of its English one: a `{name}` lost in translation
    /// would show as is, and one added would never be filled.
    #[test]
    fn translations_keep_their_places() {
        let places = |text: &str| {
            let mut found: Vec<String> = text
                .split('{')
                .skip(1)
                .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_owned()))
                .collect();
            found.sort();
            found
        };
        for (english, french) in french() {
            assert_eq!(places(english), places(french), "{english:?} / {french:?}");
            assert!(!french.trim().is_empty(), "{english:?} has an empty translation");
        }
    }
}
