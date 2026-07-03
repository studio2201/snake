# Security Audit — v1.0.27

Audit performed: 2026-07-02

Scope: backend (`backend/src/**`) and frontend (`frontend/src/**`) Rust sources.
Reference version: v1.0.27 (`d2341b8`).

This audit documents the security posture of Snake at the v1.0.27 release. It
is informational — no runtime behaviour changes.

## Findings

### Hardcoded secrets
- **None**: No API keys, OAuth tokens, database passwords, JWT signing keys,
  or private keys were found in source.
- Notes:
  - `backend/src/routes/auth/mod.rs` and `backend/src/state.rs` reference the
    literal strings `"token-a"` and `"token-bogus"`. These are session-id
    fixtures inside `#[cfg(test)] mod tests` blocks, not real credentials.
  - `backend/src/error.rs` contains the comment
    `"db password leaked into message"`. It is a documentation string inside
    the error-sanitisation module that explicitly forbids that behaviour.
  - `frontend/src/components/pin.rs` references `type="password"`. That is
    the HTML `<input type="password">` element attribute, not a credential.

### Hardcoded URLs / hosts
- **None**: No external hosts are baked into backend or frontend source.
- Notes: The only outbound dependency surface is the `shared-backend` and
  `shared-core` git dependencies declared in `backend/Cargo.toml`. URLs in
  `frontend/src/**` are limited to internal route paths (`/api/...`,
  `/login`, etc.).

### `unwrap()` / `expect()` in non-test production code
- **Low** — 1 occurrence remaining after the v1.0.27 refactor
  (`backend/src/routes/assets.rs:40`):
  ```rust
  Regex::new(APP_VERSION_REGEX_STR)
      .expect("APP_VERSION_REGEX_STR is a compile-time constant pattern")
  ```
  The pattern is a `const &str` literal validated at write-time, and the
  surrounding `OnceLock` guarantees the cost is paid once at startup.
  Acceptable; would only panic if `APP_VERSION_REGEX_STR` is corrupted by a
  future edit, in which case fail-fast at startup is preferable to a silent
  empty regex.

  Every other `.unwrap()` / `.expect()` in the backend is inside a
  `#[cfg(test)] mod tests` block. Earlier audit (pre-refactor) reported
  multiple production-path panics; all of them have been replaced with the
  `AppError`/`Result` propagation pattern (`routes/auth/cookie.rs:63` keeps
  one `try_from(...).unwrap_or(...)` as a defensive clamp fallback).

### `panic!` / `unimplemented!` / `todo!` in non-test code
- **None**: No `panic!`, `unimplemented!`, or `todo!` macros appear in
  production code paths. The codebase relies on the `AppError` enum
  (`backend/src/error.rs`) for graceful error propagation, including at
  process startup.

### `unsafe` blocks
- **None**: No `unsafe { ... }` blocks in production code. The workspace
  also enforces `unsafe_code = "deny"` in `Cargo.toml`'s `[workspace.lints.rust]`,
  which would cause any new `unsafe` to fail `cargo clippy` (locally and on any
  reviewer's machine).

### Crypto shortcuts
- **OK**: PIN comparison uses `constant_time_eq` at both credential-check
  sites:
  - `backend/src/routes/auth/verify_pin.rs:100` — submitted vs. expected PIN.
  - `backend/src/routes/auth/mod.rs:50` — `x-pin` header vs. configured PIN.
  No `==` / `!=` comparisons of secrets were found in
  `backend/src/routes/auth/**`. Session IDs are 32-byte hex tokens generated
  via `rand::rngs::OsRng` (`backend/src/services/session.rs`); the legacy
  `/dev/urandom` + time-seeded SHA-256 fallback was removed in this release.

### Cookie flags
- **OK**: All authentication cookies are built through a single helper,
  `backend/src/routes/auth/cookie.rs::build_auth_cookie`, which sets:
  - `HttpOnly(true)` — JS cannot read the session token.
  - `Secure(secure)` — propagated from the X-Forwarded-Proto / base-URL
    HTTPS detector in `verify_pin.rs::cookie_should_be_secure`.
  - `SameSite::Strict` — cross-origin requests never carry the session
    cookie, neutralising CSRF for state-changing endpoints.
  - `path("/")` — sent on every route.
  - `max_age` clamped to `[60s, 30d]` so a misconfigured
    `cookie_max_age_hours` cannot zero-out or immortalise the cookie.
  Logout uses `build_clear_cookie` with `Duration::ZERO` and the same flags.

### CORS / CSRF
- **Low**: The shipped `docker-compose.yml` defaults `ALLOWED_ORIGINS=*`
  (line 15: `ALLOWED_ORIGINS: ${SNAKE_ALLOWED_ORIGINS:-*}`). The
  `shared-backend` middleware's `cors_layer(&config)` honours the
  `ALLOWED_ORIGINS` env var, so production deployments can restrict it
  per-environment without rebuilding.
  - Because the auth cookie is `SameSite=Strict`, cross-origin browsers
    will not send it on `POST /api/...` requests, so CORS `*` does **not**
    by itself enable cross-origin authentication.
  - Recommendation: set `SNAKE_ALLOWED_ORIGINS` (or `ALLOWED_ORIGINS`)
    explicitly to the deployed origin in `production` compose profiles.

### `println!` / `dbg!` / `eprintln!` in non-test code
- **Low** — 2 occurrences, both deliberate and documented:
  - `backend/src/main.rs:17` — `eprintln!("startup failed: {e}")` runs
    before the tracing subscriber is initialised, so the structured
    logger is unavailable at that point. Fallback is appropriate.
  - `backend/src/bin/sh.rs:11` — the `/bin/sh` stub binary prints its
    notice to stderr and exits 127. It is never invoked in normal
    operation; the message exists to make accidental invocation obvious.
  All other logging flows through `tracing` (initialised in
  `backend/src/tracing_init.rs`).

### `unwrap()` on `web-sys` / browser APIs (frontend)
- **OK (documented)**: The frontend has zero bare `.unwrap()` calls. Every
  browser-API failure uses `.expect("...")` with a descriptive invariant
  message:
  - `frontend/src/components/snake/keys.rs:60` —
    `web_sys::window().expect("renderer runs in a browser window")`.
  - `frontend/src/components/snake/keys.rs:69` —
    `.expect("keydown event is a KeyboardEvent")`.
  - `frontend/src/app.rs:100` —
    `web_sys::window().expect("renderer runs in a browser window")`.
  - `frontend/src/components/event_listener.rs:36` —
    `.expect("failed to attach event listener")`.
  - `frontend/src/components/{pin,snake_game,snake_overlay,snake_leaderboard,snake/state}.rs`
    — `use_context::<LocaleContext>().expect("LocaleContext provided ...")`
    — the `App` view always installs the context before any of these
    children mount.

  In every case, the failure mode represents a violated render-tree
  invariant (no `window` in a browser context, missing event type, missing
  provider) and panicking is more diagnostic than a silent fallback.

## Summary

Total findings: 4 (0 high, 0 medium, 4 low, 0 critical).

No action required for:
- Hardcoded secrets (None).
- Hardcoded URLs (None).
- `panic!` / `unimplemented!` / `todo!` (None).
- `unsafe` blocks (None).
- Crypto shortcuts (OK — `constant_time_eq` everywhere; OsRng session IDs).
- Cookie flags (OK — `HttpOnly` + `Secure` + `SameSite=Strict` + clamped max-age).
- Frontend `web-sys` `expect`s (OK — documented invariants).

Follow-up (all Low severity, no release blocker):
1. Consider replacing `assets.rs:40` `.expect(...)` with a `match` that
   surfaces a startup-time `AppError` if the literal pattern is ever
   edited to something invalid. Cosmetic.
2. Set `ALLOWED_ORIGINS` explicitly in production compose / Nix
   deployments instead of relying on the `*` default.
3. (Already mitigated by `SameSite=Strict`.) If the deployment ever needs
   to accept third-party frontends, revisit CSRF strategy (double-submit
   tokens or `SameSite=Lax` + per-form tokens).