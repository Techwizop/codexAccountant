#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::RwLock;

pub type FirmId = String;
pub type DocumentId = String;

pub type OcrResult<T> = Result<T, OcrError>;

#[derive(Debug, Error)]
pub enum OcrError {
    #[error("unsupported mime type: {0}")]
    UnsupportedMime(String),
    #[error("provider unavailable: {0}")]
    Provider(String),
    #[error("classification failure: {0}")]
    Classification(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum DocumentKind {
    Invoice,
    Receipt,
    BankStatement,
    Payroll,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct OcrRequest {
    pub firm_id: FirmId,
    pub document_id: DocumentId,
    pub mime_type: String,
    pub payload: Vec<u8>,
}

impl OcrRequest {
    pub fn validate(&self) -> OcrResult<()> {
        if self.mime_type.trim().is_empty() {
            return Err(OcrError::UnsupportedMime("".into()));
        }
        if self.payload.is_empty() {
            return Err(OcrError::Provider(
                "payload is empty; cannot perform OCR".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub kind: DocumentKind,
    pub confidence: f32,
    pub synopsis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrDocument {
    pub document_id: DocumentId,
    pub text: String,
    pub tokens: Vec<String>,
    pub confidence: f32,
    pub classifications: Vec<ClassificationResult>,
}

#[async_trait]
pub trait OcrProvider: Send + Sync {
    async fn extract(&self, request: &OcrRequest) -> OcrResult<OcrDocument>;
}

#[async_trait]
pub trait DocumentClassifier: Send + Sync {
    async fn classify(&self, document: &OcrDocument) -> OcrResult<Vec<ClassificationResult>>;
}

#[derive(Clone)]
pub struct OcrService {
    provider: Arc<dyn OcrProvider>,
    classifier: Arc<dyn DocumentClassifier>,
}

impl OcrService {
    #[must_use]
    pub fn new(provider: Arc<dyn OcrProvider>, classifier: Arc<dyn DocumentClassifier>) -> Self {
        Self {
            provider,
            classifier,
        }
    }

    pub async fn process(&self, request: OcrRequest) -> OcrResult<OcrDocument> {
        request.validate()?;
        let mut document = self.provider.extract(&request).await?;
        let classifications = self.classifier.classify(&document).await?;
        document.classifications = classifications;
        Ok(document)
    }
}

#[derive(Default)]
pub struct MockOcrProvider {
    documents: RwLock<HashMap<DocumentId, OcrDocument>>,
}

impl MockOcrProvider {
    pub async fn with_document(self, document: OcrDocument) -> Self {
        self.documents
            .write()
            .await
            .insert(document.document_id.clone(), document);
        self
    }

    pub async fn inject(&self, document: OcrDocument) {
        self.documents
            .write()
            .await
            .insert(document.document_id.clone(), document);
    }
}

#[async_trait]
impl OcrProvider for MockOcrProvider {
    async fn extract(&self, request: &OcrRequest) -> OcrResult<OcrDocument> {
        self.documents
            .read()
            .await
            .get(&request.document_id)
            .cloned()
            .with_context(|| format!("missing mock OCR for {}", request.document_id))
            .map_err(|err| OcrError::Provider(err.to_string()))
    }
}

#[derive(Default)]
pub struct KeywordClassifier {
    rules: HashMap<DocumentKind, Vec<String>>,
}

impl KeywordClassifier {
    #[must_use]
    pub fn with_rule(mut self, kind: DocumentKind, keywords: Vec<String>) -> Self {
        self.rules.insert(kind, keywords);
        self
    }
}

#[async_trait]
impl DocumentClassifier for KeywordClassifier {
    async fn classify(&self, document: &OcrDocument) -> OcrResult<Vec<ClassificationResult>> {
        let mut results = Vec::new();
        for (kind, keywords) in &self.rules {
            let hits = keywords
                .iter()
                .filter(|keyword| {
                    document
                        .text
                        .to_ascii_lowercase()
                        .contains(&keyword.to_ascii_lowercase())
                })
                .count();
            if hits > 0 {
                let confidence = (hits as f32 / keywords.len() as f32).clamp(0.0, 1.0);
                results.push(ClassificationResult {
                    kind: kind.clone(),
                    confidence,
                    synopsis: Some(format!("matched {hits} keyword(s)")),
                });
            }
        }
        if results.is_empty() {
            results.push(ClassificationResult {
                kind: DocumentKind::Unknown,
                confidence: 0.0,
                synopsis: None,
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_document(document_id: &str, text: &str) -> OcrDocument {
        OcrDocument {
            document_id: document_id.into(),
            text: text.into(),
            tokens: text
                .split_whitespace()
                .map(std::string::ToString::to_string)
                .collect(),
            confidence: 0.92,
            classifications: Vec::new(),
        }
    }

    #[tokio::test]
    async fn service_applies_classifier_rules() {
        let provider = MockOcrProvider::default()
            .with_document(sample_document("doc-1", "Invoice #1234 for ACME"))
            .await;
        let classifier = KeywordClassifier::default()
            .with_rule(
                DocumentKind::Invoice,
                vec!["invoice".into(), "total".into()],
            )
            .with_rule(DocumentKind::Receipt, vec!["receipt".into()]);

        let service = OcrService::new(Arc::new(provider), Arc::new(classifier));
        let result = service
            .process(OcrRequest {
                firm_id: "firm-1".into(),
                document_id: "doc-1".into(),
                mime_type: "application/pdf".into(),
                payload: vec![1, 2, 3],
            })
            .await
            .expect("OCR should succeed");

        assert_eq!(result.document_id, "doc-1");
        assert_eq!(result.classifications.len(), 1);
        assert_eq!(result.classifications[0].kind, DocumentKind::Invoice);
        assert!(result.classifications[0].confidence > 0.4);
    }

    #[tokio::test]
    async fn missing_document_yields_error() {
        let service = OcrService::new(
            Arc::new(MockOcrProvider::default()),
            Arc::new(KeywordClassifier::default()),
        );

        let err = service
            .process(OcrRequest {
                firm_id: "firm-1".into(),
                document_id: "missing".into(),
                mime_type: "image/png".into(),
                payload: vec![1],
            })
            .await
            .unwrap_err();

        assert!(matches!(err, OcrError::Provider(_)));
    }

    #[tokio::test]
    async fn unsupported_payloads_validate() {
        let service = OcrService::new(
            Arc::new(MockOcrProvider::default()),
            Arc::new(KeywordClassifier::default()),
        );

        let err = service
            .process(OcrRequest {
                firm_id: "firm".into(),
                document_id: "doc".into(),
                mime_type: "".into(),
                payload: vec![1],
            })
            .await
            .unwrap_err();

        assert!(matches!(err, OcrError::UnsupportedMime(_)));
    }
}
