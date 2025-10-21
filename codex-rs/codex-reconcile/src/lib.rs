#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

pub type SessionId = String;
pub type CandidateId = String;
pub type ReconcileResult<T> = Result<T, ReconcileError>;

#[derive(Debug, Error)]
pub enum ReconcileError {
    #[error("session {0} not found")]
    SessionNotFound(SessionId),
    #[error("candidate {0} not found")]
    CandidateNotFound(CandidateId),
    #[error("invalid transition: {0}")]
    InvalidTransition(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchCandidate {
    pub id: CandidateId,
    pub transaction_id: String,
    pub journal_entry_id: String,
    pub proposed_at: DateTime<Utc>,
    pub score: f32,
    pub status: CandidateStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_off_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateStatus {
    Pending,
    Accepted,
    PartiallyAccepted,
    Rejected,
    WrittenOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Open,
    PendingPartial,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconciliationSession {
    pub id: SessionId,
    pub company_id: String,
    pub status: SessionStatus,
    pub opened_at: DateTime<Utc>,
    pub candidates: Vec<MatchCandidate>,
}

impl ReconciliationSession {
    fn ensure_mutable(&self) -> ReconcileResult<()> {
        if matches!(self.status, SessionStatus::Closed) {
            Err(ReconcileError::InvalidTransition(format!(
                "session {} is closed",
                self.id
            )))
        } else {
            Ok(())
        }
    }

    fn add_candidate(&mut self, candidate: MatchCandidate) -> ReconcileResult<()> {
        self.ensure_mutable()?;
        self.candidates.push(candidate);
        Ok(())
    }

    fn accept(&mut self, candidate_id: &CandidateId) -> ReconcileResult<MatchCandidate> {
        self.ensure_mutable()?;
        let mut accepted = None;
        for candidate in &mut self.candidates {
            if &candidate.id == candidate_id {
                if !matches!(
                    candidate.status,
                    CandidateStatus::Pending | CandidateStatus::PartiallyAccepted
                ) {
                    return Err(ReconcileError::InvalidTransition(format!(
                        "candidate {candidate_id} is not pending"
                    )));
                }
                candidate.status = CandidateStatus::Accepted;
                candidate.write_off_reason = None;
                accepted = Some(candidate.clone());
            } else if matches!(
                candidate.status,
                CandidateStatus::Pending | CandidateStatus::PartiallyAccepted
            ) {
                candidate.status = CandidateStatus::Rejected;
            }
        }
        let accepted =
            accepted.ok_or_else(|| ReconcileError::CandidateNotFound(candidate_id.clone()))?;
        self.status = SessionStatus::Closed;
        Ok(accepted)
    }

    fn reject(&mut self, candidate_id: &CandidateId) -> ReconcileResult<MatchCandidate> {
        self.ensure_mutable()?;
        let candidate = self
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == *candidate_id)
            .ok_or_else(|| ReconcileError::CandidateNotFound(candidate_id.clone()))?;
        if candidate.status != CandidateStatus::Pending {
            return Err(ReconcileError::InvalidTransition(format!(
                "candidate {candidate_id} is not pending"
            )));
        }
        candidate.status = CandidateStatus::Rejected;
        Ok(candidate.clone())
    }

    fn partial_accept(
        &mut self,
        group_id: &str,
        candidate_ids: &[CandidateId],
    ) -> ReconcileResult<Vec<MatchCandidate>> {
        self.ensure_mutable()?;
        if candidate_ids.is_empty() {
            return Err(ReconcileError::InvalidTransition(
                "partial accept requires at least one candidate".into(),
            ));
        }
        let mut updated = Vec::new();
        for candidate in &mut self.candidates {
            if candidate_ids.iter().any(|id| id == &candidate.id) {
                if candidate.group_id.as_deref() != Some(group_id) {
                    return Err(ReconcileError::InvalidTransition(format!(
                        "candidate {} does not belong to group {group_id}",
                        candidate.id
                    )));
                }
                if candidate.status != CandidateStatus::Pending {
                    return Err(ReconcileError::InvalidTransition(format!(
                        "candidate {} is not pending",
                        candidate.id
                    )));
                }
                candidate.status = CandidateStatus::PartiallyAccepted;
                updated.push(candidate.clone());
            }
        }
        if updated.is_empty() {
            return Err(ReconcileError::InvalidTransition(format!(
                "no pending candidates were found for group {group_id}"
            )));
        }
        self.status = SessionStatus::PendingPartial;
        Ok(updated)
    }

    fn write_off(
        &mut self,
        candidate_id: &CandidateId,
        reason: String,
    ) -> ReconcileResult<MatchCandidate> {
        self.ensure_mutable()?;
        let candidate = self
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == *candidate_id)
            .ok_or_else(|| ReconcileError::CandidateNotFound(candidate_id.clone()))?;
        if !matches!(
            candidate.status,
            CandidateStatus::Pending
                | CandidateStatus::PartiallyAccepted
                | CandidateStatus::Rejected
        ) {
            return Err(ReconcileError::InvalidTransition(format!(
                "candidate {candidate_id} cannot be written off from status {:?}",
                candidate.status
            )));
        }
        candidate.status = CandidateStatus::WrittenOff;
        candidate.write_off_reason = Some(reason);
        self.status = SessionStatus::PendingPartial;
        Ok(candidate.clone())
    }

    fn reopen(&mut self) -> ReconcileResult<()> {
        if matches!(self.status, SessionStatus::Open) {
            return Ok(());
        }
        for candidate in &mut self.candidates {
            candidate.status = CandidateStatus::Pending;
            candidate.write_off_reason = None;
        }
        self.status = SessionStatus::Open;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchProposal {
    pub transaction_id: String,
    pub journal_entry_id: String,
    pub amount_delta_minor: i64,
    pub date_delta_days: i64,
    pub transaction_description: String,
    pub journal_description: String,
    pub group_id: Option<String>,
}

pub trait ScoringStrategy: Send + Sync {
    fn score(&self, proposal: &MatchProposal) -> f32;
}

#[derive(Debug, Clone)]
pub struct WeightedScoringStrategy {
    amount_weight: f32,
    date_weight: f32,
    description_weight: f32,
    amount_tolerance_minor: i64,
    date_tolerance_days: i64,
}

impl WeightedScoringStrategy {
    pub fn new(
        amount_weight: f32,
        date_weight: f32,
        description_weight: f32,
        amount_tolerance_minor: i64,
        date_tolerance_days: i64,
    ) -> Self {
        Self {
            amount_weight,
            date_weight,
            description_weight,
            amount_tolerance_minor: amount_tolerance_minor.max(1),
            date_tolerance_days: date_tolerance_days.max(1),
        }
    }

    fn normalize_amount(&self, delta: i64) -> f32 {
        let ratio = (delta.abs() as f32) / (self.amount_tolerance_minor as f32);
        (1.0 - ratio).clamp(0.0, 1.0)
    }

    fn normalize_date(&self, delta: i64) -> f32 {
        let ratio = (delta.abs() as f32) / (self.date_tolerance_days as f32);
        (1.0 - ratio).clamp(0.0, 1.0)
    }
}

impl Default for WeightedScoringStrategy {
    fn default() -> Self {
        Self::new(0.45, 0.35, 0.20, 5_000, 7)
    }
}

impl ScoringStrategy for WeightedScoringStrategy {
    fn score(&self, proposal: &MatchProposal) -> f32 {
        let total_weight = self.amount_weight + self.date_weight + self.description_weight;
        if total_weight <= f32::EPSILON {
            return 0.0;
        }
        let amount_component = self.normalize_amount(proposal.amount_delta_minor);
        let date_component = self.normalize_date(proposal.date_delta_days);
        let description_component = description_similarity(
            &proposal.transaction_description,
            &proposal.journal_description,
        );
        let weighted = amount_component * self.amount_weight
            + date_component * self.date_weight
            + description_component * self.description_weight;
        (weighted / total_weight).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Default)]
pub struct LinearScoringStrategy {
    inner: WeightedScoringStrategy,
}

impl LinearScoringStrategy {
    pub fn new() -> Self {
        Self {
            inner: WeightedScoringStrategy::default(),
        }
    }
}

impl ScoringStrategy for LinearScoringStrategy {
    fn score(&self, proposal: &MatchProposal) -> f32 {
        self.inner.score(proposal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationAuditAction {
    SessionCreated,
    CandidateAdded,
    CandidateAccepted,
    CandidateRejected,
    CandidatePartiallyAccepted,
    CandidateWrittenOff,
    SessionReopened,
}

#[derive(Debug, Clone)]
pub struct ReconciliationAuditEvent {
    pub session_id: SessionId,
    pub candidate_id: Option<CandidateId>,
    pub action: ReconciliationAuditAction,
    pub note: Option<String>,
}

pub trait ReconciliationAuditHook: Send + Sync {
    fn record(&self, event: &ReconciliationAuditEvent);
}

#[derive(Default)]
pub struct NoopReconciliationAuditHook;

impl ReconciliationAuditHook for NoopReconciliationAuditHook {
    fn record(&self, _event: &ReconciliationAuditEvent) {}
}

pub trait ReconciliationStore: Send + Sync {
    fn create_session(
        &self,
        session: ReconciliationSession,
    ) -> ReconcileResult<ReconciliationSession>;
    fn save_session(&self, session: &ReconciliationSession) -> ReconcileResult<()>;
    fn get_session(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession>;
}

#[derive(Default)]
pub struct InMemoryReconciliationStore {
    sessions: RwLock<HashMap<SessionId, ReconciliationSession>>,
}

impl InMemoryReconciliationStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn with_write<F, T>(&self, f: F) -> ReconcileResult<T>
    where
        F: FnOnce(&mut HashMap<SessionId, ReconciliationSession>) -> ReconcileResult<T>,
    {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_| ReconcileError::Storage("session store poisoned".into()))?;
        f(&mut guard)
    }
}

impl ReconciliationStore for InMemoryReconciliationStore {
    fn create_session(
        &self,
        session: ReconciliationSession,
    ) -> ReconcileResult<ReconciliationSession> {
        self.with_write(|sessions| {
            sessions.insert(session.id.clone(), session.clone());
            Ok(session)
        })
    }

    fn save_session(&self, session: &ReconciliationSession) -> ReconcileResult<()> {
        self.with_write(|sessions| {
            if !sessions.contains_key(&session.id) {
                return Err(ReconcileError::SessionNotFound(session.id.clone()));
            }
            sessions.insert(session.id.clone(), session.clone());
            Ok(())
        })
    }

    fn get_session(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession> {
        let guard = self
            .sessions
            .read()
            .map_err(|_| ReconcileError::Storage("session store poisoned".into()))?;
        guard
            .get(session_id)
            .cloned()
            .ok_or_else(|| ReconcileError::SessionNotFound(session_id.clone()))
    }
}

#[cfg(feature = "postgres-store")]
#[derive(Clone)]
pub struct PostgresReconciliationStore {
    connection_string: String,
}

#[cfg(feature = "postgres-store")]
impl PostgresReconciliationStore {
    #[must_use]
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
        }
    }
}

#[cfg(feature = "postgres-store")]
impl ReconciliationStore for PostgresReconciliationStore {
    fn create_session(
        &self,
        session: ReconciliationSession,
    ) -> ReconcileResult<ReconciliationSession> {
        let _ = (&self.connection_string, &session);
        Err(ReconcileError::Storage(
            "postgres reconciliation store not yet implemented".into(),
        ))
    }

    fn save_session(&self, session: &ReconciliationSession) -> ReconcileResult<()> {
        let _ = (&self.connection_string, session);
        Err(ReconcileError::Storage(
            "postgres reconciliation store not yet implemented".into(),
        ))
    }

    fn get_session(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession> {
        let _ = (&self.connection_string, session_id);
        Err(ReconcileError::Storage(
            "postgres reconciliation store not yet implemented".into(),
        ))
    }
}

pub trait ReconciliationService: Send + Sync {
    fn create_session(&self, company_id: &str) -> ReconcileResult<ReconciliationSession>;
    fn add_candidate(
        &self,
        session_id: &SessionId,
        proposal: MatchProposal,
    ) -> ReconcileResult<MatchCandidate>;
    fn accept(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
    ) -> ReconcileResult<MatchCandidate>;
    fn reject(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
    ) -> ReconcileResult<MatchCandidate>;
    fn accept_partial(
        &self,
        session_id: &SessionId,
        group_id: &str,
        candidate_ids: Vec<CandidateId>,
    ) -> ReconcileResult<Vec<MatchCandidate>>;
    fn write_off(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
        reason: String,
    ) -> ReconcileResult<MatchCandidate>;
    fn reopen(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession>;
    fn session(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession>;
    fn register_audit_hook(&self, hook: Arc<dyn ReconciliationAuditHook>);
}

pub struct InMemoryReconciliationService {
    scoring: Arc<dyn ScoringStrategy>,
    store: Arc<dyn ReconciliationStore>,
    audit_hooks: RwLock<Vec<Arc<dyn ReconciliationAuditHook>>>,
}

impl InMemoryReconciliationService {
    pub fn new(scoring: Arc<dyn ScoringStrategy>) -> Self {
        Self::with_store(scoring, Arc::new(InMemoryReconciliationStore::new()))
    }

    pub fn with_store(
        scoring: Arc<dyn ScoringStrategy>,
        store: Arc<dyn ReconciliationStore>,
    ) -> Self {
        Self {
            scoring,
            store,
            audit_hooks: RwLock::new(Vec::new()),
        }
    }

    fn emit_audit(&self, event: ReconciliationAuditEvent) {
        if let Ok(hooks) = self.audit_hooks.read() {
            for hook in hooks.iter() {
                hook.record(&event);
            }
        }
    }

    fn modify_session<F, T>(
        &self,
        session_id: &SessionId,
        mutator: F,
    ) -> ReconcileResult<(ReconciliationSession, T)>
    where
        F: FnOnce(&mut ReconciliationSession) -> ReconcileResult<T>,
    {
        let mut session = self.store.get_session(session_id)?;
        let result = mutator(&mut session)?;
        self.store.save_session(&session)?;
        Ok((session, result))
    }

    fn update_session<F>(
        &self,
        session_id: &SessionId,
        mutator: F,
    ) -> ReconcileResult<ReconciliationSession>
    where
        F: FnOnce(&mut ReconciliationSession) -> ReconcileResult<()>,
    {
        self.modify_session(session_id, |session| {
            mutator(session)?;
            Ok(())
        })
        .map(|(session, _)| session)
    }
}

impl ReconciliationService for InMemoryReconciliationService {
    fn create_session(&self, company_id: &str) -> ReconcileResult<ReconciliationSession> {
        let session = ReconciliationSession {
            id: Uuid::new_v4().to_string(),
            company_id: company_id.into(),
            status: SessionStatus::Open,
            opened_at: Utc::now(),
            candidates: Vec::new(),
        };
        let stored = self.store.create_session(session)?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: stored.id.clone(),
            candidate_id: None,
            action: ReconciliationAuditAction::SessionCreated,
            note: None,
        });
        Ok(stored)
    }

    fn add_candidate(
        &self,
        session_id: &SessionId,
        proposal: MatchProposal,
    ) -> ReconcileResult<MatchCandidate> {
        let score = self.scoring.score(&proposal);
        let candidate = MatchCandidate {
            id: Uuid::new_v4().to_string(),
            transaction_id: proposal.transaction_id,
            journal_entry_id: proposal.journal_entry_id,
            proposed_at: Utc::now(),
            score,
            status: CandidateStatus::Pending,
            group_id: proposal.group_id,
            write_off_reason: None,
        };
        self.update_session(session_id, |session| {
            session.add_candidate(candidate.clone())
        })?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: Some(candidate.id.clone()),
            action: ReconciliationAuditAction::CandidateAdded,
            note: None,
        });
        Ok(candidate)
    }

    fn accept(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
    ) -> ReconcileResult<MatchCandidate> {
        let (_, accepted) =
            self.modify_session(session_id, |session| session.accept(candidate_id))?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: Some(candidate_id.clone()),
            action: ReconciliationAuditAction::CandidateAccepted,
            note: None,
        });
        Ok(accepted)
    }

    fn reject(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
    ) -> ReconcileResult<MatchCandidate> {
        let (_, rejected) =
            self.modify_session(session_id, |session| session.reject(candidate_id))?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: Some(candidate_id.clone()),
            action: ReconciliationAuditAction::CandidateRejected,
            note: None,
        });
        Ok(rejected)
    }

    fn accept_partial(
        &self,
        session_id: &SessionId,
        group_id: &str,
        candidate_ids: Vec<CandidateId>,
    ) -> ReconcileResult<Vec<MatchCandidate>> {
        let (_, updated) = self.modify_session(session_id, |session| {
            session.partial_accept(group_id, &candidate_ids)
        })?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: candidate_ids.first().cloned(),
            action: ReconciliationAuditAction::CandidatePartiallyAccepted,
            note: Some(format!("group {group_id}")),
        });
        Ok(updated)
    }

    fn write_off(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
        reason: String,
    ) -> ReconcileResult<MatchCandidate> {
        let reason_clone = reason.clone();
        let (_, written_off) = self.modify_session(session_id, |session| {
            session.write_off(candidate_id, reason_clone.clone())
        })?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: Some(candidate_id.clone()),
            action: ReconciliationAuditAction::CandidateWrittenOff,
            note: Some(reason),
        });
        Ok(written_off)
    }

    fn reopen(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession> {
        let session = self.update_session(session_id, ReconciliationSession::reopen)?;
        self.emit_audit(ReconciliationAuditEvent {
            session_id: session_id.clone(),
            candidate_id: None,
            action: ReconciliationAuditAction::SessionReopened,
            note: None,
        });
        Ok(session)
    }

    fn session(&self, session_id: &SessionId) -> ReconcileResult<ReconciliationSession> {
        self.store.get_session(session_id)
    }

    fn register_audit_hook(&self, hook: Arc<dyn ReconciliationAuditHook>) {
        if let Ok(mut hooks) = self.audit_hooks.write() {
            hooks.push(hook);
        }
    }
}

fn description_similarity(left: &str, right: &str) -> f32 {
    let tokenize = |input: &str| -> BTreeSet<String> {
        input
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let intersection = left_tokens.intersection(&right_tokens).count() as f32;
    let union = left_tokens.union(&right_tokens).count() as f32;
    if union <= f32::EPSILON {
        0.0
    } else {
        (intersection / union).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn service() -> InMemoryReconciliationService {
        InMemoryReconciliationService::new(Arc::new(LinearScoringStrategy::new()))
    }

    fn proposal(
        group_id: Option<&str>,
        amount_delta_minor: i64,
        date_delta_days: i64,
        transaction_description: &str,
        journal_description: &str,
    ) -> MatchProposal {
        MatchProposal {
            transaction_id: "txn-1".into(),
            journal_entry_id: "je-1".into(),
            amount_delta_minor,
            date_delta_days,
            transaction_description: transaction_description.into(),
            journal_description: journal_description.into(),
            group_id: group_id.map(ToString::to_string),
        }
    }

    #[test]
    fn create_session_initializes_state() {
        let service = service();
        let session = service
            .create_session("comp-1")
            .expect("session should be created");
        assert_eq!(session.company_id, "comp-1");
        assert_eq!(session.status, SessionStatus::Open);
        assert!(session.candidates.is_empty());
    }

    #[test]
    fn add_candidate_scores_and_tracks() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                proposal(Some("grp-1"), 0, 0, "Coffee", "Coffee"),
            )
            .expect("candidate added");
        assert_eq!(candidate.transaction_id, "txn-1");
        assert_eq!(candidate.status, CandidateStatus::Pending);
        assert_eq!(candidate.group_id.as_deref(), Some("grp-1"));
        let fetched = service.session(&session.id).expect("session fetch");
        assert_eq!(fetched.candidates.len(), 1);
    }

    #[test]
    fn accept_candidate_closes_session() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                proposal(None, 0, 0, "Invoice #1", "Invoice #1"),
            )
            .expect("candidate added");
        let accepted = service
            .accept(&session.id, &candidate.id)
            .expect("candidate accepted");
        assert_eq!(accepted.status, CandidateStatus::Accepted);
        let updated = service.session(&session.id).expect("session fetch");
        assert_eq!(updated.status, SessionStatus::Closed);
    }

    #[test]
    fn reject_candidate_keeps_session_open() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                proposal(None, 0, 0, "Invoice #2", "Invoice #3"),
            )
            .expect("candidate added");
        let rejected = service
            .reject(&session.id, &candidate.id)
            .expect("candidate rejected");
        assert_eq!(rejected.status, CandidateStatus::Rejected);
        let updated = service.session(&session.id).expect("session fetch");
        assert_eq!(updated.status, SessionStatus::Open);
    }

    #[test]
    fn partial_accept_transitions_session() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let first = service
            .add_candidate(
                &session.id,
                proposal(Some("grp-1"), 50, 1, "Lunch", "Team lunch"),
            )
            .expect("candidate added");
        let second = service
            .add_candidate(
                &session.id,
                proposal(Some("grp-1"), 100, 1, "Lunch receipt", "Team lunch"),
            )
            .expect("candidate added");
        let updated = service
            .accept_partial(&session.id, "grp-1", vec![first.id, second.id])
            .expect("partial accept");
        assert_eq!(updated.len(), 2);
        let session = service.session(&session.id).expect("session fetch");
        assert_eq!(session.status, SessionStatus::PendingPartial);
        assert!(
            session
                .candidates
                .iter()
                .all(|candidate| candidate.status == CandidateStatus::PartiallyAccepted)
        );
    }

    #[test]
    fn write_off_marks_candidate() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                proposal(Some("grp-2"), 10, 0, "Bank fee", "Monthly fee"),
            )
            .expect("candidate added");
        let written_off = service
            .write_off(&session.id, &candidate.id, "Immateral difference".into())
            .expect("write off");
        assert_eq!(written_off.status, CandidateStatus::WrittenOff);
        assert_eq!(
            written_off.write_off_reason.as_deref(),
            Some("Immateral difference")
        );
    }

    #[test]
    fn reopen_resets_candidate_statuses() {
        let service = service();
        let session = service.create_session("comp-1").expect("session created");
        let candidate = service
            .add_candidate(&session.id, proposal(None, 0, 0, "Rent", "Rent"))
            .expect("candidate added");
        service
            .accept(&session.id, &candidate.id)
            .expect("candidate accepted");
        let reopened = service.reopen(&session.id).expect("session reopened");
        assert_eq!(reopened.status, SessionStatus::Open);
        assert!(
            reopened
                .candidates
                .iter()
                .all(|candidate| candidate.status == CandidateStatus::Pending)
        );
    }

    #[test]
    fn weighted_strategy_penalizes_amount_delta() {
        let strategy = WeightedScoringStrategy::default();
        let mut previous = strategy.score(&proposal(None, 0, 0, "Rent", "Rent"));
        for delta in [250, 1_000, 10_000] {
            let score = strategy.score(&proposal(None, delta, 0, "Rent", "Rent"));
            assert!(score < previous);
            previous = score;
        }
    }

    #[test]
    fn weighted_strategy_penalizes_date_delta() {
        let strategy = WeightedScoringStrategy::default();
        let mut previous = strategy.score(&proposal(None, 0, 0, "Rent", "Rent"));
        let mut reductions = 0;
        for days in [2, 7, 21] {
            let score = strategy.score(&proposal(None, 0, days, "Rent", "Rent"));
            assert!(score <= previous);
            if score + f32::EPSILON < previous {
                reductions += 1;
            }
            previous = score;
        }
        assert!(reductions >= 1, "expected at least one reduction");
    }

    #[test]
    fn weighted_strategy_rewards_description_similarity() {
        let strategy = WeightedScoringStrategy::default();
        let high = strategy.score(&proposal(
            None,
            0,
            0,
            "Utilities invoice",
            "Utilities invoice",
        ));
        let medium = strategy.score(&proposal(
            None,
            0,
            0,
            "Utilities invoice",
            "Monthly utilities",
        ));
        let low = strategy.score(&proposal(None, 0, 0, "Utilities invoice", "Travel expense"));
        assert!(high > medium);
        assert!(medium > low);
    }

    #[test]
    fn audit_hook_captures_events() {
        #[derive(Default)]
        struct CollectingHook {
            events: RwLock<Vec<ReconciliationAuditAction>>,
        }

        impl ReconciliationAuditHook for CollectingHook {
            fn record(&self, event: &ReconciliationAuditEvent) {
                if let Ok(mut guard) = self.events.write() {
                    guard.push(event.action.clone());
                }
            }
        }

        let hook = Arc::new(CollectingHook::default());
        let service = service();
        service.register_audit_hook(hook.clone());
        let session = service.create_session("comp-2").expect("session created");
        let candidate = service
            .add_candidate(&session.id, proposal(None, 0, 0, "Audit", "Audit"))
            .expect("candidate added");
        service
            .reject(&session.id, &candidate.id)
            .expect("candidate rejected");
        let events = hook.events.read().expect("events lock");
        assert!(events.contains(&ReconciliationAuditAction::SessionCreated));
        assert!(events.contains(&ReconciliationAuditAction::CandidateAdded));
        assert!(events.contains(&ReconciliationAuditAction::CandidateRejected));
    }
}
