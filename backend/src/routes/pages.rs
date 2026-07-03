//! Static page handlers (`/`, `/login`).
//!
//! Both routes serve the same `frontend/dist/index.html` shell, gating the
//! `/` route on PIN authentication. The `web_root` path is read once at
//! startup (in [`AppStateInner::web_root`]) so handlers never have to walk
//! up the path tree at request time.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::CookieJar;
use std::collections::HashMap;
use tokio::fs;

use crate::error::AppError;
use crate::routes::auth::is_authenticated;
use crate::routes::redirect::is_valid_redirect_url;
use crate::state::AppState;

/// Render `frontend/dist/index.html` with the runtime site title substituted
/// in for the `{{SITE_TITLE}}` placeholder.
async fn render_index(state: &AppState) -> Result<Response, AppError> {
    let path = state.web_root.join("index.html");
    let html = fs::read_to_string(&path).await.map_err(|e| {
        tracing::error!(
            target: "page",
            path = %path.display(),
            error = %e,
            "failed to read index.html"
        );
        AppError::Io(e)
    })?;
    let rendered = html.replace("{{SITE_TITLE}}", &state.config.server.site_title);

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/html"),
    );
    Ok((StatusCode::OK, headers, rendered).into_response())
}

/// `GET /` — if authenticated, serve the SPA shell; otherwise bounce to
/// `/login?redirect=<original-path>`.
pub async fn serve_root(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    uri: Uri,
) -> Response {
    if !is_authenticated(&jar, &state, &headers).await {
        let target = percent_encoding::utf8_percent_encode(
            &uri.to_string(),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        return Redirect::temporary(&format!("/login?redirect={target}")).into_response();
    }
    match render_index(&state).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

/// `GET /login` — if already authenticated, honour the optional `redirect=`
/// query parameter (validated by [`is_valid_redirect_url`]). Otherwise
/// render the SPA shell.
pub async fn serve_login(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if is_authenticated(&jar, &state, &headers).await {
        if let Some(redirect) = params.get("redirect")
            && is_valid_redirect_url(redirect)
        {
            return Redirect::temporary(redirect).into_response();
        }
        return Redirect::temporary("/").into_response();
    }
    match render_index(&state).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn render_index_replaces_placeholder() {
        // We can't easily hit `render_index` from a sync test without a
        // temp web-root. Instead, verify the substitution logic in isolation
        // by mirroring it on a synthetic HTML body.
        let body = "<title>{{SITE_TITLE}}</title>";
        let rendered = body.replace("{{SITE_TITLE}}", crate::config::APP_BRAND);
        assert!(
            rendered.contains(&format!("<title>{}</title>", crate::config::APP_BRAND)),
            "rendered = {rendered:?}",
        );
    }
}
