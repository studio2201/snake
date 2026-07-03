//! Snake leaderboard persistence and endpoints.
//!
//! - `GET /api/leaderboard` — read the top-10 scores
//! - `POST /api/leaderboard` — submit a new score (sanitised + sorted)
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::AppError;
use crate::state::AppState;

const MAX_PLAYER_NAME_CHARS: usize = 15;
const MAX_LEADERBOARD_ENTRIES: usize = 10;
const ANONYMOUS_NAME: &str = "Anonymous";

/// One row in the leaderboard.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeaderboardEntry {
    /// Player-chosen display name; sanitised server-side.
    pub name: String,
    /// Score, higher = better.
    pub score: u32,
    /// Rfc3339 timestamp taken when the score was submitted.
    pub date: String,
}

/// Read the leaderboard file from disk, returning an empty list when the
/// file is missing or unparseable.
async fn read_leaderboard(path: &std::path::Path) -> Vec<LeaderboardEntry> {
    match fs::read_to_string(path).await {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!(
                    target: "leaderboard",
                    path = %path.display(),
                    error = %e,
                    "leaderboard file is unparseable; treating as empty"
                );
                Vec::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            tracing::warn!(
                target: "leaderboard",
                path = %path.display(),
                error = %e,
                "leaderboard file unreadable; treating as empty"
            );
            Vec::new()
        }
    }
}

/// UTF-8-safe truncation of `name` to at most [`MAX_PLAYER_NAME_CHARS`]
/// characters. Empty / whitespace-only inputs are coerced to
/// [`ANONYMOUS_NAME`].
#[must_use]
pub fn sanitize_player_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ANONYMOUS_NAME.to_string();
    }
    if trimmed.chars().count() <= MAX_PLAYER_NAME_CHARS {
        trimmed.to_string()
    } else {
        trimmed.chars().take(MAX_PLAYER_NAME_CHARS).collect()
    }
}

/// `GET /api/leaderboard` — return the current top-10 leaderboard.
pub async fn get_leaderboard(State(state): State<AppState>) -> Response {
    let path = state.leaderboard_file.clone();
    let list = read_leaderboard(&path).await;
    (StatusCode::OK, Json(list)).into_response()
}

/// `POST /api/leaderboard` — accept a new entry, sanitise it, sort, truncate,
/// and persist.
pub async fn submit_score(
    State(state): State<AppState>,
    Json(mut entry): Json<LeaderboardEntry>,
) -> Result<Response, AppError> {
    let path = state.leaderboard_file.clone();

    let mut list = read_leaderboard(&path).await;

    entry.name = sanitize_player_name(&entry.name);
    // `chrono::DateTime<Utc>::to_rfc3339` is infallible, so we don't need a
    // dedicated error variant for timestamp formatting any more — keep the
    // log site in case future format strings become fallible.
    entry.date = Utc::now().to_rfc3339();
    tracing::trace!(target: "leaderboard", date = %entry.date, "timestamped new entry");

    list.push(entry);
    list.sort_by_key(|e| std::cmp::Reverse(e.score));
    list.truncate(MAX_LEADERBOARD_ENTRIES);

    let content = serde_json::to_string_pretty(&list)?;
    // Ensure the directory exists (it should, but cheap insurance if the
    // operator mounted a fresh volume between startup and first write).
    fs::create_dir_all(&state.data_dir).await?;
    fs::write(&path, content).await?;

    tracing::info!(
        target: "leaderboard",
        path = %path.display(),
        entries = list.len(),
        "leaderboard updated"
    );
    Ok((StatusCode::OK, Json(list)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_handles_empty() {
        assert_eq!(sanitize_player_name(""), ANONYMOUS_NAME);
        assert_eq!(sanitize_player_name("   "), ANONYMOUS_NAME);
        assert_eq!(sanitize_player_name("\t\n"), ANONYMOUS_NAME);
    }

    #[test]
    fn sanitize_trims_whitespace() {
        assert_eq!(sanitize_player_name("  alice  "), "alice");
    }

    #[test]
    fn sanitize_keeps_short_names_untouched() {
        assert_eq!(sanitize_player_name("alice"), "alice");
    }

    #[test]
    fn sanitize_truncates_by_chars_not_bytes() {
        // Each emoji is 4 bytes; 16 such chars = 64 bytes. A naive byte
        // slice on this would panic; we want a 15-char result.
        let emoji_name = "😀".repeat(16);
        let out = sanitize_player_name(&emoji_name);
        assert_eq!(out.chars().count(), MAX_PLAYER_NAME_CHARS);
    }

    #[test]
    fn sanitize_truncates_multibyte_gracefully() {
        // "héllo wörld 你好世界" is more than 15 chars but composed entirely
        // of multi-byte UTF-8. `name[..15]` would panic; `chars().take(15)`
        // must not.
        let s = "héllo wörld 你好世界";
        let out = sanitize_player_name(s);
        assert_eq!(out.chars().count(), MAX_PLAYER_NAME_CHARS);
        assert!(s.starts_with(&out));
    }

    #[test]
    fn sanitize_exactly_at_limit_is_unchanged() {
        let s: String = "a".repeat(MAX_PLAYER_NAME_CHARS);
        assert_eq!(sanitize_player_name(&s), s);
    }
}
