#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::sync::Arc;

use async_trait::async_trait;
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::Instrument;
use tracing::info_span;
use uuid::Uuid;

pub type FirmId = String;
pub type CompanyId = String;
pub type UploadId = String;

pub type IngestResult<T> = Result<T, IngestError>;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("upstream failure: {0}")]
    Upstream(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadRequestPayload {
    pub firm_id: FirmId,
    pub company_id: Option<CompanyId>,
    pub file_name: String,
    pub content_type: String,
    pub content_length: u64,
}

impl UploadRequestPayload {
    pub fn validate(&self) -> IngestResult<()> {
        if self.firm_id.trim().is_empty() {
            return Err(IngestError::Validation("firm_id must be provided".into()));
        }
        if self.file_name.trim().is_empty() {
            return Err(IngestError::Validation("file_name must be provided".into()));
        }
        if self.content_length == 0 {
            return Err(IngestError::Validation(
                "content_length must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedUploadResponse {
    pub upload_id: UploadId,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJob {
    pub upload_id: UploadId,
    pub firm_id: FirmId,
    pub company_id: Option<CompanyId>,
    pub file_name: String,
    pub content_type: String,
    pub content_length: u64,
    pub requested_at: DateTime<Utc>,
}

#[async_trait]
pub trait UploadSigner: Send + Sync {
    async fn sign(&self, payload: &UploadRequestPayload) -> IngestResult<SignedUploadResponse>;
}

#[async_trait]
pub trait IngestionQueue: Send + Sync {
    async fn enqueue(&self, job: IngestionJob) -> IngestResult<()>;
}

#[async_trait]
pub trait IngestionService: Send + Sync {
    async fn create_upload(
        &self,
        payload: UploadRequestPayload,
    ) -> IngestResult<SignedUploadResponse>;
}

#[derive(Clone)]
pub struct IngestionFacade {
    signer: Arc<dyn UploadSigner>,
    queue: Arc<dyn IngestionQueue>,
}

impl IngestionFacade {
    #[must_use]
    pub fn new(signer: Arc<dyn UploadSigner>, queue: Arc<dyn IngestionQueue>) -> Self {
        Self { signer, queue }
    }
}

#[async_trait]
impl IngestionService for IngestionFacade {
    async fn create_upload(
        &self,
        payload: UploadRequestPayload,
    ) -> IngestResult<SignedUploadResponse> {
        payload.validate()?;
        let response = self.signer.sign(&payload).await?;
        let job = IngestionJob {
            upload_id: response.upload_id.clone(),
            firm_id: payload.firm_id.clone(),
            company_id: payload.company_id.clone(),
            file_name: payload.file_name.clone(),
            content_type: payload.content_type.clone(),
            content_length: payload.content_length,
            requested_at: Utc::now(),
        };
        self.queue.enqueue(job).await?;
        Ok(response)
    }
}

#[derive(Clone)]
pub struct ApiState {
    service: Arc<dyn IngestionService>,
}

pub fn router(service: Arc<dyn IngestionService>) -> Router<ApiState> {
    Router::new()
        .route("/upload-url", post(create_upload_handler))
        .with_state(ApiState { service })
}

async fn create_upload_handler(
    State(state): State<ApiState>,
    Json(payload): Json<UploadRequestPayload>,
) -> Result<Json<SignedUploadResponse>, ApiError> {
    let span = info_span!("create_upload_url", firm = %payload.firm_id);
    let result = state
        .service
        .create_upload(payload)
        .instrument(span)
        .await
        .map(Json);
    result.map_err(ApiError)
}

#[derive(Debug)]
pub struct ApiError(IngestError);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            IngestError::Validation(_) => StatusCode::BAD_REQUEST,
            IngestError::NotFound(_) => StatusCode::NOT_FOUND,
            IngestError::Upstream(_) => StatusCode::BAD_GATEWAY,
            IngestError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({
            "error": self.0.to_string(),
        });
        (status, Json(body)).into_response()
    }
}

#[derive(Default)]
pub struct MockUploadSigner;

#[async_trait]
impl UploadSigner for MockUploadSigner {
    async fn sign(&self, payload: &UploadRequestPayload) -> IngestResult<SignedUploadResponse> {
        let upload_id = generate_upload_id();
        Ok(SignedUploadResponse {
            upload_id,
            upload_url: format!(
                "https://mock-storage/{}/{}",
                payload.firm_id, payload.file_name
            ),
            expires_at: Utc::now() + chrono::Duration::minutes(15),
            fields: serde_json::json!({ "token": "mock-token" }),
        })
    }
}

#[derive(Default)]
pub struct InMemoryQueue {
    jobs: RwLock<Vec<IngestionJob>>,
}

impl InMemoryQueue {
    pub async fn jobs(&self) -> Vec<IngestionJob> {
        self.jobs.read().await.clone()
    }
}

#[async_trait]
impl IngestionQueue for InMemoryQueue {
    async fn enqueue(&self, job: IngestionJob) -> IngestResult<()> {
        self.jobs.write().await.push(job);
        Ok(())
    }
}

pub mod cli {
    use super::*;

    #[derive(Clone)]
    pub struct CliHarness {
        service: Arc<dyn IngestionService>,
    }

    impl CliHarness {
        #[must_use]
        pub fn new(service: Arc<dyn IngestionService>) -> Self {
            Self { service }
        }

        pub async fn simulate_signed_upload(
            &self,
            firm_id: FirmId,
            file_name: &str,
            content_length: u64,
        ) -> IngestResult<SignedUploadResponse> {
            self.service
                .create_upload(UploadRequestPayload {
                    firm_id,
                    company_id: None,
                    file_name: file_name.to_string(),
                    content_type: mime_guess::from_path(file_name)
                        .first_raw()
                        .unwrap_or("application/octet-stream")
                        .to_string(),
                    content_length,
                })
                .await
        }
    }
}

pub fn generate_upload_id() -> UploadId {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn facade_enqueues_job() {
        let signer = Arc::new(MockUploadSigner);
        let queue = Arc::new(InMemoryQueue::default());
        let service = IngestionFacade::new(signer, queue.clone());

        let payload = UploadRequestPayload {
            firm_id: "firm-1".into(),
            company_id: Some("company-2".into()),
            file_name: "invoice.pdf".into(),
            content_type: "application/pdf".into(),
            content_length: 2048,
        };

        let response = service
            .create_upload(payload.clone())
            .await
            .expect("create upload");

        assert_eq!(
            response.upload_url,
            "https://mock-storage/firm-1/invoice.pdf"
        );

        let jobs = queue.jobs().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].firm_id, payload.firm_id);
    }

    #[tokio::test]
    async fn router_returns_signed_url() {
        let service = Arc::new(IngestionFacade::new(
            Arc::new(MockUploadSigner),
            Arc::new(InMemoryQueue::default()),
        ));

        let result = create_upload_handler(
            State(ApiState { service }),
            Json(UploadRequestPayload {
                firm_id: "firm-123".into(),
                company_id: None,
                file_name: "receipt.png".into(),
                content_type: "image/png".into(),
                content_length: 5120,
            }),
        )
        .await
        .expect("handler should succeed");

        assert!(result.upload_url.contains("receipt.png"));
    }

    #[tokio::test]
    async fn cli_harness_round_trips() {
        let service = Arc::new(IngestionFacade::new(
            Arc::new(MockUploadSigner),
            Arc::new(InMemoryQueue::default()),
        ));
        let harness = cli::CliHarness::new(service);
        let response = harness
            .simulate_signed_upload("firm-987".into(), "report.csv", 4096)
            .await
            .expect("simulate upload");

        assert!(response.upload_url.contains("firm-987"));
        assert_eq!(response.fields["token"], "mock-token");
    }
}
