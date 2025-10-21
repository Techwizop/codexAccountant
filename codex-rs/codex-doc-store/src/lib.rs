#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type FirmId = String;
pub type CompanyId = String;
pub type DocumentId = String;
pub type ObjectVersion = u64;

pub type DocStoreResult<T> = Result<T, DocStoreError>;

#[derive(Debug, Error)]
pub enum DocStoreError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("resource conflict: {0}")]
    Conflict(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("encryption failure: {0}")]
    Encryption(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionEnvelope {
    pub key_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub wrapped_key: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    AwsKmsEnvelope,
    Aes256Gcm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionContext {
    pub firm_id: FirmId,
    pub document_id: DocumentId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutObjectRequest {
    pub metadata: DocumentMetadata,
    pub payload: Vec<u8>,
    pub retention: RetentionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub metadata: DocumentMetadata,
    pub payload: Vec<u8>,
    pub envelope: EncryptionEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub document_id: DocumentId,
    pub firm_id: FirmId,
    pub company_id: Option<CompanyId>,
    pub version: ObjectVersion,
    pub content_type: String,
    pub content_length: u64,
    pub checksum: String,
    pub uploaded_at: DateTime<Utc>,
    pub uploaded_by: String,
    pub tags: Vec<String>,
    pub retention_class: String,
}

impl DocumentMetadata {
    pub fn normalize(mut self) -> DocStoreResult<Self> {
        if self.document_id.trim().is_empty() {
            return Err(DocStoreError::Validation(
                "document id cannot be empty".into(),
            ));
        }
        if self.firm_id.trim().is_empty() {
            return Err(DocStoreError::Validation("firm id cannot be empty".into()));
        }
        if self.version == 0 {
            return Err(DocStoreError::Validation("version must start at 1".into()));
        }
        if self.content_length == 0 {
            return Err(DocStoreError::Validation(
                "content length must be non-zero".into(),
            ));
        }

        self.document_id = self.document_id.trim().to_string();
        self.firm_id = self.firm_id.trim().to_string();
        self.content_type = self.content_type.trim().to_string();
        self.checksum = self.checksum.trim().to_string();
        self.retention_class = self.retention_class.trim().to_string();

        self.tags = normalize_tags(self.tags);

        Ok(self)
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut dedup = std::collections::HashSet::new();
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| dedup.insert(tag.to_ascii_lowercase()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub class: String,
    pub retention_days: u32,
    pub legal_hold: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MetadataQuery {
    pub firm_id: Option<FirmId>,
    pub company_id: Option<CompanyId>,
    pub tags: Vec<String>,
}

impl MetadataQuery {
    pub fn matches(&self, metadata: &DocumentMetadata) -> bool {
        if let Some(firm_id) = &self.firm_id
            && metadata.firm_id != *firm_id
        {
            return false;
        }
        if let Some(company_id) = &self.company_id
            && metadata.company_id.as_ref() != Some(company_id)
        {
            return false;
        }
        if !self.tags.is_empty()
            && !self.tags.iter().all(|tag| {
                metadata
                    .tags
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(tag))
            })
        {
            return false;
        }
        true
    }
}

#[async_trait]
pub trait DocumentStore: Send + Sync {
    async fn put_object(&self, request: PutObjectRequest) -> DocStoreResult<DocumentMetadata>;

    async fn get_object(&self, document_id: &DocumentId) -> DocStoreResult<StoredObject>;

    async fn delete_object(&self, document_id: &DocumentId) -> DocStoreResult<()>;

    async fn list_metadata(&self, query: MetadataQuery) -> DocStoreResult<Vec<DocumentMetadata>>;
}

#[async_trait]
pub trait EnvelopeEncryptor: Send + Sync {
    async fn wrap_key(
        &self,
        context: EncryptionContext,
    ) -> DocStoreResult<(EncryptionEnvelope, Vec<u8>)>;

    async fn unwrap_key(&self, envelope: &EncryptionEnvelope) -> DocStoreResult<Vec<u8>>;
}

#[async_trait]
pub trait RetentionScheduler: Send + Sync {
    async fn register(
        &self,
        metadata: &DocumentMetadata,
        policy: &RetentionPolicy,
    ) -> DocStoreResult<()>;

    async fn cancel(&self, metadata: &DocumentMetadata) -> DocStoreResult<()>;
}

#[derive(Clone)]
pub struct InMemoryDocumentStore {
    state: Arc<RwLock<InMemoryState>>,
    encryptor: Arc<dyn EnvelopeEncryptor>,
    scheduler: Arc<dyn RetentionScheduler>,
}

#[derive(Default)]
struct InMemoryState {
    objects: HashMap<DocumentId, StoredObject>,
}

impl InMemoryDocumentStore {
    #[must_use]
    pub fn new(
        encryptor: Arc<dyn EnvelopeEncryptor>,
        scheduler: Arc<dyn RetentionScheduler>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(InMemoryState::default())),
            encryptor,
            scheduler,
        }
    }

    fn ensure_new_version(
        state: &InMemoryState,
        metadata: &DocumentMetadata,
    ) -> DocStoreResult<()> {
        if let Some(existing) = state.objects.get(&metadata.document_id)
            && metadata.version <= existing.metadata.version
        {
            return Err(DocStoreError::Conflict(format!(
                "document {} already has version {}",
                metadata.document_id, existing.metadata.version
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl DocumentStore for InMemoryDocumentStore {
    async fn put_object(&self, request: PutObjectRequest) -> DocStoreResult<DocumentMetadata> {
        let normalized = request.metadata.clone().normalize()?;
        let (envelope, data_key) = self
            .encryptor
            .wrap_key(EncryptionContext {
                firm_id: normalized.firm_id.clone(),
                document_id: normalized.document_id.clone(),
            })
            .await?;

        let mut guard = self.state.write().await;
        Self::ensure_new_version(&guard, &normalized)?;

        let stored = StoredObject {
            metadata: normalized.clone(),
            payload: request.payload.clone(),
            envelope,
        };
        guard
            .objects
            .insert(normalized.document_id.clone(), stored.clone());

        drop(guard);

        self.scheduler
            .register(&stored.metadata, &request.retention)
            .await?;

        // Drop the derived data key to make sure tests assert on behavior rather than leaks.
        drop(data_key);

        Ok(stored.metadata)
    }

    async fn get_object(&self, document_id: &DocumentId) -> DocStoreResult<StoredObject> {
        let guard = self.state.read().await;
        guard
            .objects
            .get(document_id)
            .cloned()
            .ok_or_else(|| DocStoreError::NotFound(format!("document {document_id}")))
    }

    async fn delete_object(&self, document_id: &DocumentId) -> DocStoreResult<()> {
        let mut guard = self.state.write().await;
        let removed = guard.objects.remove(document_id);
        let removed =
            removed.ok_or_else(|| DocStoreError::NotFound(format!("document {document_id}")))?;
        drop(guard);
        self.scheduler.cancel(&removed.metadata).await
    }

    async fn list_metadata(&self, query: MetadataQuery) -> DocStoreResult<Vec<DocumentMetadata>> {
        let guard = self.state.read().await;
        let mut results = guard
            .objects
            .values()
            .filter(|stored| query.matches(&stored.metadata))
            .map(|stored| stored.metadata.clone())
            .collect::<Vec<_>>();
        results.sort_by(|left, right| left.uploaded_at.cmp(&right.uploaded_at));
        Ok(results)
    }
}

pub struct NoopRetentionScheduler {
    calls: Arc<RwLock<Vec<RetentionCall>>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
struct RetentionCall {
    document_id: DocumentId,
    _class: String,
    action: RetentionAction,
}

#[derive(Debug, Clone, Copy)]
enum RetentionAction {
    Register,
    Cancel,
}

impl Default for NoopRetentionScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl NoopRetentionScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            calls: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[cfg(test)]
    pub(crate) async fn calls(&self) -> Vec<RetentionCall> {
        self.calls.read().await.clone()
    }
}

#[async_trait]
impl RetentionScheduler for NoopRetentionScheduler {
    async fn register(
        &self,
        metadata: &DocumentMetadata,
        policy: &RetentionPolicy,
    ) -> DocStoreResult<()> {
        let mut guard = self.calls.write().await;
        guard.push(RetentionCall {
            document_id: metadata.document_id.clone(),
            _class: policy.class.clone(),
            action: RetentionAction::Register,
        });
        Ok(())
    }

    async fn cancel(&self, metadata: &DocumentMetadata) -> DocStoreResult<()> {
        let mut guard = self.calls.write().await;
        guard.push(RetentionCall {
            document_id: metadata.document_id.clone(),
            _class: metadata.retention_class.clone(),
            action: RetentionAction::Cancel,
        });
        Ok(())
    }
}

#[derive(Default)]
pub struct MockEnvelopeEncryptor;

#[async_trait]
impl EnvelopeEncryptor for MockEnvelopeEncryptor {
    async fn wrap_key(
        &self,
        context: EncryptionContext,
    ) -> DocStoreResult<(EncryptionEnvelope, Vec<u8>)> {
        let key = vec![0u8; 32];
        let envelope = EncryptionEnvelope {
            key_id: format!("mock-kms:{}", context.firm_id),
            algorithm: EncryptionAlgorithm::AwsKmsEnvelope,
            wrapped_key: key.clone(),
        };
        Ok((envelope, key))
    }

    async fn unwrap_key(&self, envelope: &EncryptionEnvelope) -> DocStoreResult<Vec<u8>> {
        Ok(envelope.wrapped_key.clone())
    }
}

pub fn generate_document_id() -> DocumentId {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_metadata() -> DocumentMetadata {
        DocumentMetadata {
            document_id: generate_document_id(),
            firm_id: "firm-123".into(),
            company_id: Some("company-456".into()),
            version: 1,
            content_type: "application/pdf".into(),
            content_length: 1024,
            checksum: "abc123".into(),
            uploaded_at: Utc::now(),
            uploaded_by: "user@example.com".into(),
            tags: vec!["invoice".into(), "Q1".into()],
            retention_class: "finance.7y".into(),
        }
    }

    fn sample_policy() -> RetentionPolicy {
        RetentionPolicy {
            class: "finance.7y".into(),
            retention_days: 365 * 7,
            legal_hold: false,
        }
    }

    #[tokio::test]
    async fn stores_and_retrieves_objects() {
        let scheduler = Arc::new(NoopRetentionScheduler::new());
        let store = InMemoryDocumentStore::new(Arc::new(MockEnvelopeEncryptor), scheduler.clone());

        let metadata = sample_metadata();
        let request = PutObjectRequest {
            metadata: metadata.clone(),
            payload: vec![42; 8],
            retention: sample_policy(),
        };

        let expected = metadata
            .clone()
            .normalize()
            .expect("metadata should normalize");

        let stored = store.put_object(request).await.expect("store object");
        assert_eq!(stored.document_id, metadata.document_id);

        let fetched = store
            .get_object(&metadata.document_id)
            .await
            .expect("fetch object");
        assert_eq!(fetched.metadata, expected);
        assert_eq!(fetched.payload, vec![42; 8]);

        let calls = scheduler.calls().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].document_id, metadata.document_id);
        assert!(matches!(calls[0].action, RetentionAction::Register));
    }

    #[tokio::test]
    async fn enforces_version_ordering() {
        let store = InMemoryDocumentStore::new(
            Arc::new(MockEnvelopeEncryptor),
            Arc::new(NoopRetentionScheduler::new()),
        );
        let mut metadata = sample_metadata();

        store
            .put_object(PutObjectRequest {
                metadata: metadata.clone(),
                payload: vec![],
                retention: sample_policy(),
            })
            .await
            .expect("first version");

        metadata.version = 1;
        let err = store
            .put_object(PutObjectRequest {
                metadata,
                payload: vec![],
                retention: sample_policy(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DocStoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn lists_metadata_with_filters() {
        let store = InMemoryDocumentStore::new(
            Arc::new(MockEnvelopeEncryptor),
            Arc::new(NoopRetentionScheduler::new()),
        );

        let mut meta_a = sample_metadata();
        meta_a.tags = vec!["invoice".into(), "2024".into()];
        meta_a.document_id = "doc-a".into();
        store
            .put_object(PutObjectRequest {
                metadata: meta_a.clone(),
                payload: vec![],
                retention: sample_policy(),
            })
            .await
            .expect("store a");

        let mut meta_b = sample_metadata();
        meta_b.tags = vec!["statement".into()];
        meta_b.company_id = Some("company-789".into());
        meta_b.document_id = "doc-b".into();
        store
            .put_object(PutObjectRequest {
                metadata: meta_b.clone(),
                payload: vec![],
                retention: sample_policy(),
            })
            .await
            .expect("store b");

        let results = store
            .list_metadata(MetadataQuery {
                firm_id: Some(meta_a.firm_id.clone()),
                company_id: meta_b.company_id.clone(),
                tags: vec!["statement".into()],
            })
            .await
            .expect("list metadata");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, meta_b.document_id);
    }

    #[tokio::test]
    async fn delete_cancels_retention() {
        let scheduler = Arc::new(NoopRetentionScheduler::new());
        let store = InMemoryDocumentStore::new(Arc::new(MockEnvelopeEncryptor), scheduler.clone());
        let metadata = sample_metadata();
        store
            .put_object(PutObjectRequest {
                metadata: metadata.clone(),
                payload: vec![],
                retention: sample_policy(),
            })
            .await
            .expect("store");

        store
            .delete_object(&metadata.document_id)
            .await
            .expect("delete");

        let calls = scheduler.calls().await;
        assert_eq!(calls.len(), 2);
        assert!(matches!(calls[1].action, RetentionAction::Cancel));
    }
}
