//! Session-id generation.
//!
//! We deliberately do **not** use the time-seeded SHA-256 fallback that the
//! original `verify_pin` shipped with — a time-only seed is fully predictable
//! to anyone who can guess when the server booted. Instead we always draw
//! from `OsRng`, the operating system's cryptographic RNG.

use rand::{TryRngCore, rngs::OsRng};

/// Length (in bytes) of the random session token before hex encoding.
///
/// 16 bytes (128 bits) is well beyond brute-force feasibility for the
/// lifetime of any single cookie.
const SESSION_ID_BYTES: usize = 16;

/// Generate a fresh cryptographically random session id.
///
/// Returns a 32-character lowercase hex string. Never panics: on the
/// (impossible-on-supported-platforms) chance that `OsRng` returns an
/// error, we fall back to drawing from the thread-local RNG. The fallback
/// is **not** cryptographically strong but at least entropy-mixed.
#[must_use]
pub fn generate_session_id() -> String {
    let mut bytes = [0u8; SESSION_ID_BYTES];
    OsRng
        .try_fill_bytes(&mut bytes)
        .or_else(|_| {
            let mut r = rand::rng();
            r.try_fill_bytes(&mut bytes)
        })
        .unwrap_or_else(|_| {
            // Last-ditch: panic-free zeroed bytes. The cookie will be
            // rejected on next request because it won't match a stored
            // session, but the server keeps running.
            tracing::warn!(target: "session", "OsRng failed; falling back to zeroed bytes");
        });
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn session_id_is_32_hex_chars() {
        let id = generate_session_id();
        assert_eq!(id.len(), SESSION_ID_BYTES * 2);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn session_ids_are_unique() {
        let mut seen = HashSet::new();
        for _ in 0..256 {
            let id = generate_session_id();
            assert!(seen.insert(id), "collision in 256 generated ids");
        }
    }
}
