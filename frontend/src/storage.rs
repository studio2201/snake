//! Tiny wrapper around `localStorage` and `document.cookie` used by the
//! frontend to persist the active theme and the chosen locale.
//!
//! Strings are stored wrapped in JSON quotes by some browsers; this layer
//! transparently unwraps them on read and re-normalises on write.

/// Name of the long-lived cookie used to persist the active theme.
///
/// Note: this used to be `super_metroid_theme` in the upstream fork; it was
/// renamed to `snake_theme` when the cookie key was rebranded to match this
/// project's name.
const COOKIE_NAME: &str = "snake_theme";

/// Static facade over the browser storage APIs used by the application.
pub struct StorageService;

impl StorageService {
    /// Returns the [`web_sys::Storage`] handle for the current window, if any.
    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    /// Reads the raw `document.cookie` string. Returns `None` when the cookie
    /// API is unavailable or when `document.cookie` is not a string.
    fn get_cookie_str() -> Option<String> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let val =
            js_sys::Reflect::get(&document, &wasm_bindgen::JsValue::from_str("cookie")).ok()?;
        val.as_string()
    }

    /// Writes a new cookie string to `document.cookie`. Returns `None` only
    /// when the DOM itself is unavailable; the actual `set` is fire-and-forget.
    fn set_cookie_str(cookie_value: &str) -> Option<()> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let _ = js_sys::Reflect::set(
            &document,
            &wasm_bindgen::JsValue::from_str("cookie"),
            &wasm_bindgen::JsValue::from_str(cookie_value),
        );
        Some(())
    }

    /// Reads a string value from storage. For the `"theme"` key, the cookie
    /// is consulted first so the value survives across browser sessions even
    /// when `localStorage` is cleared. Returns `default` when nothing is set.
    pub fn get_item(key: &str, default: &str) -> String {
        if key == "theme"
            && let Some(cookie_str) = Self::get_cookie_str()
        {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.split('=').map(|s| s.trim()).collect();
                if parts.len() >= 2 && parts[0] == COOKIE_NAME {
                    let val = parts[1].to_string();
                    let clean = if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                        val[1..val.len() - 1].to_string()
                    } else {
                        val
                    };
                    let _ = Self::local_storage().map(|s| s.set_item(key, &clean));
                    return clean;
                }
            }
        }
        let val = Self::local_storage().and_then(|s| s.get_item(key).ok().flatten());
        match val {
            Some(v) => {
                if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                    let clean = v[1..v.len() - 1].to_string();
                    Self::set_item(key, &clean);
                    clean
                } else {
                    v
                }
            }
            None => default.to_string(),
        }
    }

    /// Writes a string value to `localStorage`. When the key is `"theme"`,
    /// the value is also mirrored to a long-lived cookie so it survives a
    /// browser session reset.
    pub fn set_item(key: &str, value: &str) {
        if let Some(s) = Self::local_storage() {
            let _ = s.set_item(key, value);
        }
        if key == "theme" {
            let cookie_value = format!(
                "{COOKIE_NAME}={}; Path=/; Max-Age=31536000; SameSite=Lax",
                value
            );
            Self::set_cookie_str(&cookie_value);
        }
    }
}
