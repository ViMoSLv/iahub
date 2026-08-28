//! Mega Brain V0 — Operation Journal Service (ADR-0003)
//!
//! High-level operations for the append-only journal. All state transitions
//! go through this service; no handler may bypass it.
//!
//! Key invariant: every external side effect MUST have a PREPARED journal entry
//! persisted BEFORE execution begins (INV-004, INV-019).

use super::error::OperationError;
use super::model::{
    validate_operation_transition, OperationId, OperationRecord, OperationStatus, OperationType,
};
use super::repository;
use crate::persistence::transaction::Transaction;

/// OperationService provides all journal operations. It is stateless;
/// all state lives in SQLite via the provided Transaction.
pub struct OperationService;

impl OperationService {
    /// Create a new PREPARED journal entry. This MUST be called and committed
    /// BEFORE any external side effect begins (INV-004).
    pub fn prepare(
        tx: &Transaction,
        id: OperationId,
        operation_type: OperationType,
        now: i64,
    ) -> Result<OperationRecord, OperationError> {
        let record = OperationRecord::prepare(id, operation_type, now);
        repository::insert_operation(tx, &record)?;
        Ok(record)
    }

    /// Transition an operation to EXECUTING status.
    pub fn begin_execution(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::Executing,
            now,
            expected_version,
            None,
            None,
            None,
            None,
        )
    }

    /// Record that the external side effect was observed.
    pub fn record_observation(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
        external_reference: Option<&str>,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::SideEffectObserved,
            now,
            expected_version,
            None,
            None,
            external_reference,
            None,
        )
    }

    /// Mark operation as successfully committed.
    pub fn commit(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
        result_payload: Option<&str>,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::Committed,
            now,
            expected_version,
            None,
            result_payload,
            None,
            None,
        )
    }

    /// Roll back an operation (no residual side effects).
    pub fn rollback(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
        last_error: Option<&str>,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::RolledBack,
            now,
            expected_version,
            last_error,
            None,
            None,
            None,
        )
    }

    /// Mark operation as failed definitively.
    pub fn mark_failed(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
        last_error: Option<&str>,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::Failed,
            now,
            expected_version,
            last_error,
            None,
            None,
            None,
        )
    }

    /// Mark operation as requiring reconcile (outcome uncertain).
    pub fn mark_requires_reconcile(
        tx: &Transaction,
        operation_id: &OperationId,
        now: i64,
        expected_version: i64,
        recovery_hint: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<i64, OperationError> {
        Self::transition(
            tx,
            operation_id,
            OperationStatus::RequiresReconcile,
            now,
            expected_version,
            last_error,
            None,
            None,
            recovery_hint,
        )
    }

    /// Get an operation by ID.
    pub fn get_by_id(
        tx: &Transaction,
        operation_id: &OperationId,
    ) -> Result<Option<OperationRecord>, OperationError> {
        repository::find_by_id(tx, operation_id)
    }

    /// List all non-terminal operations for startup reconcile.
    pub fn list_non_terminal(tx: &Transaction) -> Result<Vec<OperationRecord>, OperationError> {
        repository::list_non_terminal(tx)
    }

    /// Internal: validate transition and update status atomically.
    #[allow(clippy::too_many_arguments)]
    fn transition(
        tx: &Transaction,
        operation_id: &OperationId,
        new_status: OperationStatus,
        now: i64,
        expected_version: i64,
        last_error: Option<&str>,
        result_payload: Option<&str>,
        external_reference: Option<&str>,
        recovery_hint: Option<&str>,
    ) -> Result<i64, OperationError> {
        // Read current state to validate transition
        let current =
            repository::find_by_id(tx, operation_id)?.ok_or_else(|| OperationError::NotFound {
                operation_id: operation_id.clone(),
            })?;

        if current.version != expected_version {
            return Err(OperationError::VersionConflict {
                operation_id: operation_id.clone(),
                expected_version,
                actual_version: current.version,
            });
        }

        // Validate the state machine transition
        validate_operation_transition(current.status, new_status).map_err(|_| {
            OperationError::InvalidTransition {
                operation_id: operation_id.clone(),
                from: current.status,
                to: new_status,
            }
        })?;

        repository::update_status(
            tx,
            &repository::StatusUpdate {
                operation_id: operation_id.clone(),
                new_status,
                now,
                expected_version,
                last_error: last_error.map(String::from),
                result_payload: result_payload.map(String::from),
                external_reference: external_reference.map(String::from),
                recovery_hint: recovery_hint.map(String::from),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    #[test]
    fn prepare_persists_before_execution() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationService::prepare(
            &tx,
            OperationId("op-prep".into()),
            OperationType::WorktreeCreate,
            1700000000,
        )
        .unwrap();

        assert_eq!(record.status, OperationStatus::Prepared);
        assert_eq!(record.version, 1);

        // Verify it's persisted and findable
        let found = OperationService::get_by_id(&tx, &OperationId("op-prep".into()))
            .unwrap()
            .expect("must exist after prepare");
        assert_eq!(found.status, OperationStatus::Prepared);

        tx.commit().unwrap();
    }

    #[test]
    fn valid_full_lifecycle() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let op_id = OperationId("op-lifecycle".into());

        // PREPARED
        let record =
            OperationService::prepare(&tx, op_id.clone(), OperationType::AgentSpawn, 100).unwrap();
        assert_eq!(record.version, 1);

        // EXECUTING
        let v2 = OperationService::begin_execution(&tx, &op_id, 200, 1).unwrap();
        assert_eq!(v2, 2);

        // SIDE_EFFECT_OBSERVED
        let v3 =
            OperationService::record_observation(&tx, &op_id, 300, 2, Some("pid-12345")).unwrap();
        assert_eq!(v3, 3);

        // COMMITTED
        let v4 =
            OperationService::commit(&tx, &op_id, 400, 3, Some("{\"agent_id\":\"a-1\"}")).unwrap();
        assert_eq!(v4, 4);

        let final_record = OperationService::get_by_id(&tx, &op_id).unwrap().unwrap();
        assert_eq!(final_record.status, OperationStatus::Committed);
        assert_eq!(
            final_record.external_reference.as_deref(),
            Some("pid-12345")
        );
        assert_eq!(
            final_record.result_payload.as_deref(),
            Some("{\"agent_id\":\"a-1\"}")
        );
        assert!(final_record.status.is_terminal());

        tx.commit().unwrap();
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let op_id = OperationId("op-invalid".into());
        OperationService::prepare(&tx, op_id.clone(), OperationType::MergeExecute, 100).unwrap();

        // PREPARED → COMMITTED is invalid (must go through EXECUTING → SIDE_EFFECT_OBSERVED)
        let err = OperationService::commit(&tx, &op_id, 200, 1, None).unwrap_err();
        assert!(matches!(err, OperationError::InvalidTransition { .. }));

        tx.rollback().unwrap();
    }

    #[test]
    fn failed_operation_records_error() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let op_id = OperationId("op-fail".into());
        OperationService::prepare(&tx, op_id.clone(), OperationType::GitRefCreate, 100).unwrap();

        let v2 =
            OperationService::mark_failed(&tx, &op_id, 200, 1, Some("ref already exists")).unwrap();
        assert_eq!(v2, 2);

        let record = OperationService::get_by_id(&tx, &op_id).unwrap().unwrap();
        assert_eq!(record.status, OperationStatus::Failed);
        assert_eq!(record.last_error.as_deref(), Some("ref already exists"));
        assert!(record.status.is_terminal());

        tx.commit().unwrap();
    }

    #[test]
    fn requires_reconcile_survives_with_hint() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let op_id = OperationId("op-reconcile".into());
        OperationService::prepare(&tx, op_id.clone(), OperationType::WorktreeRemove, 100).unwrap();
        OperationService::begin_execution(&tx, &op_id, 200, 1).unwrap();

        let v3 = OperationService::mark_requires_reconcile(
            &tx,
            &op_id,
            300,
            2,
            Some("check if worktree dir still exists on disk"),
            Some("process killed during cleanup"),
        )
        .unwrap();
        assert_eq!(v3, 3);

        let record = OperationService::get_by_id(&tx, &op_id).unwrap().unwrap();
        assert_eq!(record.status, OperationStatus::RequiresReconcile);
        assert_eq!(
            record.recovery_hint.as_deref(),
            Some("check if worktree dir still exists on disk")
        );
        assert!(record.status.is_terminal());

        tx.commit().unwrap();
    }

    #[test]
    fn list_non_terminal_for_startup_reconcile() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        // PREPARED (non-terminal)
        OperationService::prepare(
            &tx,
            OperationId("op-nt-1".into()),
            OperationType::LeaseAcquire,
            100,
        )
        .unwrap();

        // EXECUTING (non-terminal)
        OperationService::prepare(
            &tx,
            OperationId("op-nt-2".into()),
            OperationType::ArtifactStore,
            200,
        )
        .unwrap();
        OperationService::begin_execution(&tx, &OperationId("op-nt-2".into()), 201, 1).unwrap();

        // FAILED (terminal — should NOT appear)
        OperationService::prepare(
            &tx,
            OperationId("op-term".into()),
            OperationType::NotificationSend,
            300,
        )
        .unwrap();
        OperationService::mark_failed(
            &tx,
            &OperationId("op-term".into()),
            301,
            1,
            Some("smtp error"),
        )
        .unwrap();

        let non_terminal = OperationService::list_non_terminal(&tx).unwrap();
        assert_eq!(non_terminal.len(), 2);
        assert_eq!(non_terminal[0].id, OperationId("op-nt-1".into()));
        assert_eq!(non_terminal[1].id, OperationId("op-nt-2".into()));

        tx.commit().unwrap();
    }

    #[test]
    fn unknown_status_fails_closed() {
        let result: Result<OperationStatus, _> = serde_json::from_str("\"UNKNOWN_STATE\"");
        assert!(result.is_err(), "unknown operation status must fail closed");
    }
}
