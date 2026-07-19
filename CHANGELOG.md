# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.32] - 2026-07-02

### Fixed
- **PWA install path mismatch**: `frontend/index.html` referenced `/manifest.json` but the backend serves it at `/Assets/manifest.json`. The PWA install prompt would 404 against the `<link rel="manifest">`. Updated `index.html` to point to the correct path.
- **Leaderboard race condition**: two concurrent `POST /api/leaderboard` requests could each read the file, append their entry, and write — losing one entry. Now serialised through `state.leaderboard_lock` and written atomically via temp-file + rename (`leaderboard.rs::atomic_write`).
- **No request body size limit**: a multi-MB POST to `/api/verify-pin` (or any other `/api/*` endpoint) would fully buffer before the Json extractor rejects, exhausting memory. Now capped at 64 KiB via `tower_http::limit::RequestBodyLimitLayer`; over-sized bodies get `413 Payload Too Large` before any handler runs.

### Added
- `leaderboard_lock: Arc<tokio::sync::Mutex<()>>` field on `AppStateInner`.
- `routes/leaderboard.rs::atomic_write()` helper, with two unit tests covering round-trip and over-write.
- Two new integration tests: `request_body_over_limit_returns_413` and `leaderboard_concurrent_submissions_do_not_lose_data` (the latter fires 10 parallel POSTs and asserts every name lands).

## [1.0.31] - 2026-07-02

### Added
- Frontend `<meta http-equiv="Content-Security-Policy">` tag in `frontend/index.html` for defence-in-depth (the backend already sets CSP via response headers; the meta fires only if served without headers, e.g. `file://`).
- `frontend/scripts/optimise-wasm.sh`: a post-`trunk build --release` hook that runs `wasm-opt -Oz --strip-debug --strip-producers`. Reduces the WASM bundle from 520 KB → 355 KB (-32% raw, -19% gzipped over the wire).
- README dev instructions document the local wasm-opt step.

### Changed
- `service-worker.js` rewritten for Snake (was carrying fork leftovers for a notepad app: `LOG_CACHE_*` names, references to non-existent `/js/marked/*` and `/js/@highlightjs/*` packages, `config.highlightLanguages` field lookup that returned `undefined`).
- `super_metroid_theme` cookie name replaced by `snake_theme` everywhere (in `frontend/index.html` flicker script and `frontend/src/storage.rs`).
- Favicon cache-bust query string `?v=1.0.18` → `?v=1.0.30` in `frontend/index.html`.
- Brand-name literal consolidated to `pub const APP_BRAND: &str = "Snake"` in `backend/src/config.rs` (replaces 4 hardcoded `"Snake"` strings).

### Notes
- The published Docker image still ships the unoptimised 518 KB WASM. Embedding `wasm-opt -Oz` into the container build ran into read-only-file permission issues that required a `chmod u+w` dance during the chroot-build, and the local script handles the same operation trivially. Run `frontend/scripts/optimise-wasm.sh` after `trunk build --release` for the locally-developed bundle.

## [1.0.27] - 2026-07-02

### Added
- Backend integration tests covering rate limiting, authentication, leaderboard persistence, and health check.
- Frontend tests for game-logic helpers (`generate_food`, `direction_for_key`, `apply_tick`).
- Optional `cargo deny` step in CI for supply-chain auditing.
- App version is now sourced from `/api/config` instead of being hardcoded.

### Changed
- `cookie` name renamed `PAD_PIN` → `SNAKE_PIN`.
- Date timestamps in leaderboard now produced by `chrono` (RFC 3339, infallible).
- Session IDs generated via `rand::rngs::OsRng` instead of a `/dev/urandom`-with-time-seeded-SHA-256 fallback.
- Frontend split into focused modules under `components/snake/` (state, food, tick, actions, keys) to enforce the 250-line file-size rule.
- `frontend/dist/Assets/manifest.json` PWA metadata corrected from the upstream "Log" notepad values to Snake's branding.

### Fixed
- UTF-8 byte slice panic in leaderboard name sanitizer (now truncates by `chars()`).
- Hardcoded fallback date replaced with proper error propagation.
- `Path::parent().unwrap()` (panic at filesystem root) replaced with explicit `web_root` resolution at startup.
- `/api/logout` cookie age built via unclamped `try_into` (could panic) replaced with clamped builder.
- Pre-existing typo in `frontend/Cargo.toml` (missing `}` on `web-sys` inline table) — surfaced by a clean Trunk build.

### Removed
- ~150 lines of dead "notepad" code paths carried over from the upstream shared fork.
- Tracked 2.1 GB vendored `frontend/Assets/shared-assets/shared-rust/` nested Cargo workspace.
- Unused backend dependencies: `notify`, `futures-util`.
- Stale `data/notepads.json` and `data/default.txt` artifacts.
- Stale `frontend/Assets/asset-manifest.json` referencing non-existent files.

### Security
- `redirect` URL sanitizer rejects `%2F`, `%5c`, double-encoded forms, control characters, and scheme-relative URLs in addition to the existing checks.
- Cookie maximum age clamped to `[1 minute, 30 days]` so a misconfigured `cookie_max_age_hours` cannot pin a session forever or zero it out.
- Hardcoded version strings removed from the frontend (was leaking through to JS until the `/api/config` response arrived).

## [1.0.26] - 2026-07-01

Carried over from upstream fork (pad/notepad). Pre-refactor state.

## [1.0.25]

Initial release under the studio2201 organisation.