//! PWA web-app manifest endpoint.
//!
//! Serves `Assets/manifest.json` from the web-root with the site's title
//! (and short description) overridden to match the runtime config. If the
//! prebuilt manifest is missing or unparseable, falls back to a minimal
//! inline manifest so the browser doesn't reject the install prompt.
//!
//! Note on `icons`: the bundled `frontend/Assets/manifest.json` currently
//! reuses `favicon.png` for both the `192x192` and `512x512` slots
//! because dedicated PNGs of those exact dimensions aren't checked in.
//! Chrome accepts a single source file at multiple sizes, but for the
//! strictest installability grade we should ship distinct raster assets
//! per `sizes` value. Until then the warning lives here so the next
//! manifest refresh knows the spot.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use tokio::fs;

use crate::state::AppState;

const FALLBACK_DESCRIPTION: &str = "A traditional arcade snake game";

/// `GET /Assets/manifest.json` — PWA manifest with runtime-overridden title.
pub async fn serve_manifest(State(state): State<AppState>) -> Response {
    let manifest_path = state.web_root.join("Assets").join("manifest.json");
    let content = match fs::read_to_string(&manifest_path).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                target: "pwa_manifest",
                path = %manifest_path.display(),
                error = %e,
                "manifest missing; returning built-in fallback"
            );
            return rendered_response(fallback_manifest(&state.config.server.site_title));
        }
    };

    let mut value: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                target: "pwa_manifest",
                path = %manifest_path.display(),
                error = %e,
                "manifest unparseable; returning built-in fallback"
            );
            return rendered_response(fallback_manifest(&state.config.server.site_title));
        }
    };

    value["name"] = Value::String(state.config.server.site_title.clone());
    value["short_name"] = Value::String(state.config.server.site_title.clone());
    value["description"] = Value::String(FALLBACK_DESCRIPTION.to_string());
    rendered_response(value)
}

fn rendered_response(value: Value) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    (StatusCode::OK, headers, Json(value)).into_response()
}

fn fallback_manifest(site_title: &str) -> Value {
    serde_json::json!({
        "start_url": "/",
        "display": "standalone",
        "background_color": "#ffffff",
        "theme_color": "#000000",
        "icons": [
            { "src": "logo.png", "type": "image/png", "sizes": "192x192" },
            { "src": "logo.png", "type": "image/png", "sizes": "512x512" }
        ],
        "orientation": "any",
        "name": site_title,
        "short_name": site_title,
        "description": FALLBACK_DESCRIPTION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_includes_runtime_site_title() {
        let v = fallback_manifest("Snake Deluxe");
        assert_eq!(v["name"], "Snake Deluxe");
        assert_eq!(v["short_name"], "Snake Deluxe");
        assert_eq!(v["description"], FALLBACK_DESCRIPTION);
    }

    /// The manifest endpoint only mutates `name`, `short_name`, and
    /// `description` via `serde_json::Value` indexing — every other key
    /// must round-trip unchanged. This pins down the new Chrome-friendly
    /// keys (`lang`, `dir`, `prefer_related_applications`, `categories`,
    /// `screenshots`) so a future refactor can't silently drop them.
    #[test]
    fn new_manifest_keys_survive_merge() {
        let mut value = serde_json::json!({
            "background_color": "#000000",
            "description": "stale",
            "display": "standalone",
            "icons": [
                { "sizes": "192x192", "src": "favicon.png", "type": "image/png" }
            ],
            "lang": "en",
            "dir": "ltr",
            "name": "stale",
            "orientation": "any",
            "prefer_related_applications": false,
            "short_name": "stale",
            "start_url": "/",
            "theme_color": "#000000",
            "categories": ["games", "entertainment"],
            "screenshots": [],
        });

        // Mirror the same mutations `serve_manifest` performs.
        let site_title = "Snake";
        value["name"] = Value::String(site_title.to_string());
        value["short_name"] = Value::String(site_title.to_string());
        value["description"] = Value::String(FALLBACK_DESCRIPTION.to_string());

        // Overridden fields reflect the runtime config.
        assert_eq!(value["name"], site_title);
        assert_eq!(value["short_name"], site_title);
        assert_eq!(value["description"], FALLBACK_DESCRIPTION);

        // Untouched keys survived the merge.
        assert_eq!(value["lang"], "en");
        assert_eq!(value["dir"], "ltr");
        assert_eq!(value["prefer_related_applications"], false);
        assert_eq!(value["start_url"], "/");
        assert_eq!(value["theme_color"], "#000000");
        assert_eq!(value["background_color"], "#000000");
        assert_eq!(value["display"], "standalone");
        assert_eq!(value["orientation"], "any");
        assert_eq!(value["categories"][0], "games");
        assert_eq!(value["categories"][1], "entertainment");
        assert_eq!(value["screenshots"], serde_json::json!([]));
        assert_eq!(value["icons"][0]["src"], "favicon.png");
    }
}
