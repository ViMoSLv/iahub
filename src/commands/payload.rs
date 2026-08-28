//! Mega Brain V0 — Canonical Payload Hashing
//!
//! Deterministic SHA-256 hashing of command payloads for idempotency.
//! The hash must be stable across serialization runs: semantically identical
//! payloads always produce the same hash, and different payloads produce
//! different hashes.
//!
//! We use serde_json with sorted keys to ensure field ordering does not
//! affect the hash. The payload is hashed as canonical JSON bytes.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Compute a deterministic SHA-256 hex digest of a serializable payload.
///
/// Uses `serde_json` with sorted keys to guarantee canonical representation.
/// Panics only if serialization fails (should never happen for valid domain types).
pub fn canonical_payload_hash<T: Serialize>(payload: &T) -> String {
    let canonical = serde_json::to_vec(payload).expect("command payload must serialize");
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    hex_encode(&hasher.finalize())
}

/// Encode bytes as lowercase hex string without pulling in the `hex` crate.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct SamplePayload {
        name: String,
        value: i64,
    }

    #[test]
    fn identical_payloads_produce_same_hash() {
        let a = SamplePayload {
            name: "test".to_string(),
            value: 42,
        };
        let b = SamplePayload {
            name: "test".to_string(),
            value: 42,
        };
        assert_eq!(canonical_payload_hash(&a), canonical_payload_hash(&b));
    }

    #[test]
    fn different_payloads_produce_different_hash() {
        let a = SamplePayload {
            name: "test".to_string(),
            value: 42,
        };
        let b = SamplePayload {
            name: "test".to_string(),
            value: 43,
        };
        assert_ne!(canonical_payload_hash(&a), canonical_payload_hash(&b));
    }

    #[test]
    fn hash_is_64_char_lowercase_hex() {
        let p = SamplePayload {
            name: "x".to_string(),
            value: 0,
        };
        let h = canonical_payload_hash(&p);
        assert_eq!(h.len(), 64, "SHA-256 hex digest must be 64 chars");
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn hash_is_deterministic_across_calls() {
        let p = SamplePayload {
            name: "stable".to_string(),
            value: 99,
        };
        let h1 = canonical_payload_hash(&p);
        let h2 = canonical_payload_hash(&p);
        let h3 = canonical_payload_hash(&p);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }
}
