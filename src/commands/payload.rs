//! Mega Brain V0 — Canonical Payload Hashing
//!
//! Deterministic SHA-256 hashing of command payloads for idempotency.
//! The hash must be stable across serialization runs: semantically identical
//! payloads always produce the same hash, and different payloads produce
//! different hashes.
//!
//! We canonicalize via `serde_json::Value` to guarantee sorted object keys
//! regardless of the original struct field order or map type (HashMap vs BTreeMap).
//! Serialization failures return a typed error instead of panicking.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Errors that can occur during canonical payload hashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadHashError {
    /// Serialization of the payload failed. This should never happen for
    /// well-formed domain types, but we fail closed rather than panicking.
    SerializationFailed { detail: String },
}

impl std::fmt::Display for PayloadHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed { detail } => {
                write!(f, "payload serialization failed: {}", detail)
            }
        }
    }
}

impl std::error::Error for PayloadHashError {}

/// Compute a deterministic SHA-256 hex digest of a serializable payload.
///
/// Canonicalizes through `serde_json::Value` to ensure object keys are always
/// sorted alphabetically, making the hash independent of struct field declaration
/// order or HashMap iteration order. Returns an error if serialization fails.
pub fn canonical_payload_hash<T: Serialize>(payload: &T) -> Result<String, PayloadHashError> {
    // Serialize to Value first — this normalizes all maps to BTreeMap internally,
    // guaranteeing sorted key output when re-serialized to bytes.
    let value =
        serde_json::to_value(payload).map_err(|e| PayloadHashError::SerializationFailed {
            detail: e.to_string(),
        })?;

    let canonical =
        serde_json::to_vec(&value).map_err(|e| PayloadHashError::SerializationFailed {
            detail: e.to_string(),
        })?;

    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    Ok(hex_encode(&hasher.finalize()))
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
    use std::collections::BTreeMap;

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
        assert_eq!(
            canonical_payload_hash(&a).unwrap(),
            canonical_payload_hash(&b).unwrap()
        );
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
        assert_ne!(
            canonical_payload_hash(&a).unwrap(),
            canonical_payload_hash(&b).unwrap()
        );
    }

    #[test]
    fn hash_is_64_char_lowercase_hex() {
        let p = SamplePayload {
            name: "x".to_string(),
            value: 0,
        };
        let h = canonical_payload_hash(&p).unwrap();
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
        let h1 = canonical_payload_hash(&p).unwrap();
        let h2 = canonical_payload_hash(&p).unwrap();
        let h3 = canonical_payload_hash(&p).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    /// Proves that BTreeMap key ordering does not affect the hash.
    /// Two maps with identical entries inserted in different orders
    /// must produce the same canonical hash because serde_json::Value
    /// normalizes all objects to sorted keys.
    #[test]
    fn map_key_order_does_not_affect_hash() {
        let mut map_a = BTreeMap::new();
        map_a.insert("zebra".to_string(), 1);
        map_a.insert("alpha".to_string(), 2);
        map_a.insert("middle".to_string(), 3);

        let mut map_b = BTreeMap::new();
        map_b.insert("middle".to_string(), 3);
        map_b.insert("zebra".to_string(), 1);
        map_b.insert("alpha".to_string(), 2);

        assert_eq!(
            canonical_payload_hash(&map_a).unwrap(),
            canonical_payload_hash(&map_b).unwrap(),
            "semantically identical maps must produce the same hash regardless of insertion order"
        );
    }

    /// Nested structures with maps also produce stable hashes.
    #[test]
    fn nested_map_order_does_not_affect_hash() {
        #[derive(Serialize)]
        struct Nested {
            label: String,
            data: BTreeMap<String, i64>,
        }

        let mut data_a = BTreeMap::new();
        data_a.insert("z".to_string(), 1);
        data_a.insert("a".to_string(), 2);

        let mut data_b = BTreeMap::new();
        data_b.insert("a".to_string(), 2);
        data_b.insert("z".to_string(), 1);

        let a = Nested {
            label: "test".into(),
            data: data_a,
        };
        let b = Nested {
            label: "test".into(),
            data: data_b,
        };

        assert_eq!(
            canonical_payload_hash(&a).unwrap(),
            canonical_payload_hash(&b).unwrap()
        );
    }
}
