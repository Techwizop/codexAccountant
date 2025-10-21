//! Shared copy between CLI and TUI reconciliation views to keep guidance text in sync.

/// Prefix used when pointing operators at the CLI duplicate summary.
pub const DUPLICATE_GUIDANCE_PREFIX: &str = "See CLI summary for duplicate set ";

/// Joint helper so both surfaces render the exact duplicate guidance wording.
pub fn duplicate_guidance_message(id: &str) -> String {
    format!("{DUPLICATE_GUIDANCE_PREFIX}{id}")
}
