# codex-ocr

Mockable OCR abstraction for Phase 1, including:

- Provider trait that surfaces extracted text, confidence, and key classifications.
- In-memory/mock provider returning canned data for tests and local development.
- Classification helpers for detecting invoices vs. receipts, plus error types for unsupported formats.
- Unit tests covering extraction pipeline and classification branching.
