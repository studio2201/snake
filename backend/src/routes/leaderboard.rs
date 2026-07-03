//! Snake leaderboard persistence and endpoints.
//!
//! - `GET /api/leaderboard` — read the top-10 scores
//! - `POST /api/leaderboard` — submit a new score (sanitised + sorted)
//!
//! Submissions serialise through `state.leaderboard_lock` and use atomic
//! temp-file + rename so concurrent writers can't lose data via a
//! read-modify-write race or leave a half-written file on disk.
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
async fn read_leaderboard(path: &Path) -> Vec<LeaderboardEntry> {
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

/// Atomically replace `path` with `content`.
///
/// Writes to a sibling temp file then `rename`s it on top of the target.
/// `rename` is atomic on POSIX (and on Windows when the target exists),
/// so a crash or concurrent reader can never observe a half-written
/// leaderboard. The temp file lives next to `path` so the rename stays on
/// the same filesystem.
async fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        tracing::error!(
            target: "leaderboard",
            path = %path.display(),
            "leaderboard path has no parent; cannot atomic-write"
        );
        AppError::internal("leaderboard path has no parent")
    })?;
    let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        tracing::error!(
            target: "leaderboard",
            path = %path.display(),
            "leaderboard file name is not UTF-8; refusing to write"
        );
        AppError::internal("leaderboard file name is not UTF-8")
    })?;
    let tmp: PathBuf = parent.join(format!(".{file_name}.tmp"));

    fs::create_dir_all(parent).await?;
    fs::write(&tmp, content).await?;
    if let Err(e) = fs::rename(&tmp, path).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(AppError::Io(e));
    }
    Ok(())
}

/// `GET /api/leaderboard` — return the current top-10 leaderboard.
pub async fn get_leaderboard(State(state): State<AppState>) -> Response {
    let path = state.leaderboard_file.clone();
    let list = read_leaderboard(&path).await;
    (StatusCode::OK, Json(list)).into_response()
}

/// Read the leaderboard from disk and return the entry count. Used by
/// the `/metrics` endpoint to refresh the `snake_leaderboard_entries`
/// gauge without holding the leaderboard mutex.
pub async fn read_leaderboard_count(state: &AppState) -> u64 {
    read_leaderboard(&state.leaderboard_file).await.len() as u64
}

/// `POST /api/leaderboard` — accept a new entry, sanitise it, sort, truncate,
/// and persist.
///
/// The submission flow holds `state.leaderboard_lock` for the entire
/// read-modify-write critical section so two parallel POSTs cannot lose
/// data. `get_leaderboard` does NOT take the lock — a concurrent reader
/// may observe either the pre- or post-write top-10, never a half-written
/// file (atomic-rename semantics).
pub async fn submit_score(
    State(state): State<AppState>,
    Json(mut entry): Json<LeaderboardEntry>,
) -> Result<Response, AppError> {
    let path = state.leaderboard_file.clone();
    let path_for_log = path.clone();

    entry.name = sanitize_player_name(&entry.name);
    // `chrono::DateTime<Utc>::to_rfc3339` is infallible, so we don't need a
    // dedicated error variant for timestamp formatting any more — keep the
    // log site in case future format strings become fallible.
    entry.date = Utc::now().to_rfc3339();
    tracing::trace!(
        target: "leaderboard",
        date = %entry.date,
        "timestamped new entry"
    );

    // Serialise concurrent submissions so the read-modify-write can't
    // race. The lock guard drops at the end of the function, releasing
    // the lock only after `atomic_write` has renamed the temp file.
    let _guard = state.leaderboard_lock.lock().await;

    let mut list = read_leaderboard(&path).await;
    list.push(entry);
    list.sort_by_key(|e| std::cmp::Reverse(e.score));
    list.truncate(MAX_LEADERBOARD_ENTRIES);

    let content = serde_json::to_string_pretty(&list)?;
    atomic_write(&path, content.as_bytes()).await?;

    tracing::info!(
        target: "leaderboard",
        path = %path_for_log.display(),
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

    #[tokio::test]
    async fn atomic_write_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("round.json");
        atomic_write(&path, b"[{\"name\":\"a\"}]")
            .await
            .expect("write");
        let body = tokio::fs::read_to_string(&path).await.expect("read");
        assert_eq!(body, "[{\"name\":\"a\"}]");
        // Temp file should have been renamed away.
        assert!(!tmp.path().join(".round.json.tmp").exists());
    }

    #[tokio::test]
    async fn atomic_write_overwrites_existing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("over.json");
        tokio::fs::write(&path, b"OLD").await.expect("old");
        atomic_write(&path, b"NEW").await.expect("new");
        assert_eq!(tokio::fs::read_to_string(&path).await.expect("read"), "NEW");
    }
}
