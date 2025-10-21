#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type AuditLogResult<T> = Result<T, AuditLogError>;

#[derive(Debug, Error)]
pub enum AuditLogError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage failure: {0}")]
    Storage(String),
    #[error("corrupted chain: {0}")]
    Corrupted(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendRequest {
    pub entity_id: String,
    pub actor: String,
    pub action: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub entity_id: String,
    pub actor: String,
    pub action: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub previous_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuditLogFilter {
    pub entity_id: Option<String>,
    pub limit: Option<usize>,
}

#[async_trait]
pub trait AuditLog: Send + Sync {
    async fn append(&self, request: AppendRequest) -> AuditLogResult<AuditRecord>;

    async fn records(&self, filter: AuditLogFilter) -> AuditLogResult<Vec<AuditRecord>>;
}

#[derive(Default)]
pub struct InMemoryAuditLog {
    records: RwLock<Vec<AuditRecord>>,
}

impl InMemoryAuditLog {
    #[must_use]
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn compute_hash(
        previous: &str,
        entity_id: &str,
        action: &str,
        occurred_at: DateTime<Utc>,
        metadata: &serde_json::Value,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous.as_bytes());
        hasher.update(entity_id.as_bytes());
        hasher.update(action.as_bytes());
        let nanos = occurred_at
            .timestamp_nanos_opt()
            .unwrap_or_else(|| occurred_at.timestamp_micros() * 1_000);
        hasher.update(nanos.to_be_bytes());
        hasher.update(metadata.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn validate_request(request: &AppendRequest) -> AuditLogResult<()> {
        if request.entity_id.trim().is_empty() {
            return Err(AuditLogError::Validation(
                "entity_id must be provided".into(),
            ));
        }
        if request.action.trim().is_empty() {
            return Err(AuditLogError::Validation("action must be provided".into()));
        }
        if request.actor.trim().is_empty() {
            return Err(AuditLogError::Validation("actor must be provided".into()));
        }
        Ok(())
    }

    async fn verify_chain(records: &[AuditRecord]) -> AuditLogResult<()> {
        let mut previous = String::from("genesis");
        for record in records {
            if record.previous_hash != previous {
                return Err(AuditLogError::Corrupted(format!(
                    "unexpected previous hash for {}",
                    record.id
                )));
            }
            let expected = Self::compute_hash(
                &record.previous_hash,
                &record.entity_id,
                &record.action,
                record.occurred_at,
                &record.metadata,
            );
            if expected != record.hash {
                return Err(AuditLogError::Corrupted(format!(
                    "hash mismatch for {}",
                    record.id
                )));
            }
            previous = record.hash.clone();
        }
        Ok(())
    }
}

#[async_trait]
impl AuditLog for InMemoryAuditLog {
    async fn append(&self, request: AppendRequest) -> AuditLogResult<AuditRecord> {
        Self::validate_request(&request)?;

        let mut guard = self.records.write().await;
        let previous_hash = guard
            .last()
            .map(|record| record.hash.clone())
            .unwrap_or_else(|| "genesis".into());

        let occurred_at = Utc::now();
        let hash = Self::compute_hash(
            &previous_hash,
            &request.entity_id,
            &request.action,
            occurred_at,
            &request.metadata,
        );

        let record = AuditRecord {
            id: Uuid::new_v4().to_string(),
            entity_id: request.entity_id,
            actor: request.actor,
            action: request.action,
            occurred_at,
            metadata: request.metadata,
            previous_hash,
            hash,
        };

        guard.push(record.clone());
        Ok(record)
    }

    async fn records(&self, filter: AuditLogFilter) -> AuditLogResult<Vec<AuditRecord>> {
        let guard = self.records.read().await;
        Self::verify_chain(&guard).await?;
        let mut filtered = guard.clone();

        if let Some(entity_id) = filter.entity_id {
            filtered.retain(|record| record.entity_id == entity_id);
        }

        if let Some(limit) = filter.limit
            && filtered.len() > limit
        {
            filtered.truncate(limit);
        }

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn appends_records_with_hash_chain() {
        let log = InMemoryAuditLog::shared();

        let first = log
            .append(AppendRequest {
                entity_id: "company-1".into(),
                actor: "system".into(),
                action: "created".into(),
                metadata: serde_json::json!({"source": "test"}),
            })
            .await
            .expect("append record");

        assert_eq!(first.previous_hash, "genesis");
        assert!(!first.hash.is_empty());

        let second = log
            .append(AppendRequest {
                entity_id: "company-1".into(),
                actor: "user".into(),
                action: "updated".into(),
                metadata: serde_json::json!({"field": "status"}),
            })
            .await
            .expect("append record");

        assert_eq!(second.previous_hash, first.hash);

        let records = log
            .records(AuditLogFilter::default())
            .await
            .expect("records");
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn detects_tampering() {
        let log = InMemoryAuditLog::shared();

        log.append(AppendRequest {
            entity_id: "entity".into(),
            actor: "user".into(),
            action: "created".into(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("append");

        log.append(AppendRequest {
            entity_id: "entity".into(),
            actor: "user".into(),
            action: "updated".into(),
            metadata: serde_json::json!({}),
        })
        .await
        .expect("append");

        {
            let mut guard = log.records.write().await;
            guard[1].previous_hash = "tampered".into();
        }

        let err = log.records(AuditLogFilter::default()).await.unwrap_err();
        assert!(matches!(err, AuditLogError::Corrupted(_)));
    }
}
