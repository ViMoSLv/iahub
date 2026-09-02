//! Mega Brain V0 — Repository Layer
//!
//! Each aggregate has its own repository with explicit SQL. No generic CRUD
//! god object. Domain types are mapped to/from persistence rows here; the
//! `domain` module never imports `rusqlite`.

pub mod project;
pub mod provider_account;

pub use project::ProjectRepository;
pub use provider_account::ProviderAccountRepository;
