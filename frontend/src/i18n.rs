//! Internationalisation plumbing for the Snake UI.
//!
//! Three responsibilities:
//!
//! 1. Detect the user's preferred locale from `navigator.language`
//!    ([`detect_browser_locale`]) and persist the choice in a cookie via
//!    [`get_saved_locale`] / [`set_saved_locale`] (both re-exported from
//!    `shared_frontend::locale`).
//! 2. Expose a Yew context ([`LocaleContext`]) so any component can call
//!    `ctx.t("score")` without threading the locale code through props.
//! 3. Dispatch a translation key to the matching language table via
//!    [`translate`], falling back to the raw key when the table has no entry.

use yew::prelude::*;

pub use shared_frontend::locale::{detect_browser_locale, get_saved_locale, set_saved_locale};

mod de;
mod en;
mod es;
mod fr;
mod ja;
mod pt;
mod ru;
mod zh;

/// Shared context object provided by the root [`crate::app::App`] view.
///
/// Components deeper in the tree call `use_context::<LocaleContext>()` to
/// access the current locale and a callback for switching it from a child.
#[derive(Clone, PartialEq)]
pub struct LocaleContext {
    /// Active locale code (`"en"`, `"de"`, ...).
    pub current: String,
    /// Dispatched when the user picks a new language from the picker.
    pub on_change: Callback<String>,
}

impl LocaleContext {
    /// Convenience wrapper around [`translate`] using `self.current`.
    pub fn t(&self, key: &str) -> String {
        translate(&self.current, key)
    }
}

/// Looks up a translation key in the appropriate language table.
///
/// Falls back to the raw key when no table contains it so the UI never
/// goes blank for an untranslated string — the developer sees the key in
/// place during development.
pub fn translate(lang: &str, key: &str) -> String {
    let l = if lang.starts_with("zh") {
        "zh"
    } else if lang.starts_with("es") {
        "es"
    } else if lang.starts_with("de") {
        "de"
    } else if lang.starts_with("ja") {
        "ja"
    } else if lang.starts_with("fr") {
        "fr"
    } else if lang.starts_with("pt") {
        "pt"
    } else if lang.starts_with("ru") {
        "ru"
    } else {
        "en"
    };

    let val = match l {
        "zh" => zh::translate(key),
        "es" => es::translate(key),
        "de" => de::translate(key),
        "ja" => ja::translate(key),
        "fr" => fr::translate(key),
        "pt" => pt::translate(key),
        "ru" => ru::translate(key),
        _ => en::translate(key),
    };

    val.unwrap_or(key).to_string()
}
