//! Mega Brain V0 — SQLite Configuration
//!
//! Canonical connection configuration applied to every database handle.
//! All pragmas are validated after application; we never assume a PRAGMA
//! took effect merely because the statement executed without error.

use rusqlite::Connection;

use super::error::PersistenceError;

/// Minimum SQLite version required by the Mega Brain V0 constitution.
/// libsqlite3-sys 0.37.0 bundles SQLite 3.51.3+.
pub const MIN_SQLITE_VERSION: &str = "3.51.3";

/// Parse a SQLite version string "X.Y.Z" into a comparable tuple.
fn parse_sqlite_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Apply all mandatory pragmas to a connection and verify each one.
pub fn apply_and_verify_pragmas(conn: &Connection) -> Result<(), PersistenceError> {
    // Foreign keys must be enabled explicitly per connection.
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma>".to_string(),
            source: e,
        })?;
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma foreign_keys>".to_string(),
            source: e,
        })?;
    if fk != 1 {
        return Err(PersistenceError::Corrupt {
            detail: "PRAGMA foreign_keys did not take effect".to_string(),
        });
    }

    // WAL mode for concurrent readers + single writer.
    conn.execute_batch("PRAGMA journal_mode = WAL;")
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma journal_mode>".to_string(),
            source: e,
        })?;
    let jm: String = conn
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma journal_mode read>".to_string(),
            source: e,
        })?;
    // In-memory databases report "memory" and cannot use WAL; that is acceptable
    // for testing. File-backed databases must be "wal".
    let jm_lower = jm.to_lowercase();
    if jm_lower != "wal" && jm_lower != "memory" {
        return Err(PersistenceError::Corrupt {
            detail: format!("expected journal_mode=wal or memory, got {}", jm),
        });
    }

    // FULL synchronous for durability.
    conn.execute_batch("PRAGMA synchronous = FULL;")
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma synchronous>".to_string(),
            source: e,
        })?;
    let sync: i64 = conn
        .query_row("PRAGMA synchronous;", [], |row| row.get(0))
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma synchronous read>".to_string(),
            source: e,
        })?;
    // 2 = FULL
    if sync != 2 {
        return Err(PersistenceError::Corrupt {
            detail: format!("expected synchronous=2 (FULL), got {}", sync),
        });
    }

    // Busy timeout for WAL contention.
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma busy_timeout>".to_string(),
            source: e,
        })?;
    let bt: i64 = conn
        .query_row("PRAGMA busy_timeout;", [], |row| row.get(0))
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<pragma busy_timeout read>".to_string(),
            source: e,
        })?;
    if bt != 5000 {
        return Err(PersistenceError::Corrupt {
            detail: format!("expected busy_timeout=5000, got {}", bt),
        });
    }

    Ok(())
}

/// Verify that the bundled SQLite version meets the constitutional minimum.
pub fn verify_sqlite_version(conn: &Connection) -> Result<String, PersistenceError> {
    let version: String = conn
        .query_row("SELECT sqlite_version();", [], |row| row.get(0))
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<version check>".to_string(),
            source: e,
        })?;

    let current = parse_sqlite_version(&version).ok_or_else(|| PersistenceError::Corrupt {
        detail: format!("could not parse sqlite_version(): {}", version),
    })?;
    let minimum = parse_sqlite_version(MIN_SQLITE_VERSION).expect("MIN_SQLITE_VERSION is valid");

    if current < minimum {
        return Err(PersistenceError::SchemaTooNew {
            found: 0, // not a schema version issue, but reuse the fail-closed pattern
            supported: 0,
        });
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_version_meets_minimum() {
        let conn = Connection::open(":memory:").unwrap();
        let version = verify_sqlite_version(&conn).expect("version check must pass");
        let current = parse_sqlite_version(&version).unwrap();
        let minimum = parse_sqlite_version(MIN_SQLITE_VERSION).unwrap();
        assert!(
            current >= minimum,
            "bundled SQLite {} < required {}",
            version,
            MIN_SQLITE_VERSION
        );
    }

    #[test]
    fn pragmas_applied_and_verified() {
        let conn = Connection::open(":memory:").unwrap();
        apply_and_verify_pragmas(&conn).expect("all pragmas must apply cleanly");

        // Double-check independently
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);

        let jm: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap();
        // In-memory databases report "memory" instead of "wal"; both are acceptable.
        let jm_lower = jm.to_lowercase();
        assert!(
            jm_lower == "wal" || jm_lower == "memory",
            "expected journal_mode=wal or memory, got {}",
            jm
        );

        let sync: i64 = conn
            .query_row("PRAGMA synchronous;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sync, 2);

        let bt: i64 = conn
            .query_row("PRAGMA busy_timeout;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(bt, 5000);
    }

    #[test]
    fn parse_sqlite_version_valid() {
        assert_eq!(parse_sqlite_version("3.51.3"), Some((3, 51, 3)));
        assert_eq!(parse_sqlite_version("3.43.1"), Some((3, 43, 1)));
        assert_eq!(parse_sqlite_version("4.0.0"), Some((4, 0, 0)));
    }

    #[test]
    fn parse_sqlite_version_invalid() {
        assert_eq!(parse_sqlite_version("not.a.version"), None);
        assert_eq!(parse_sqlite_version("3.51"), None);
        assert_eq!(parse_sqlite_version(""), None);
    }
}
