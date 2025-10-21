use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceData {
    pub vendor: String,
    pub invoice_number: Option<String>,
    pub date: NaiveDate,
    pub line_items: Vec<LineItem>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub description: String,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntrySuggestion {
    pub lines: Vec<SuggestedLine>,
    pub memo: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedLine {
    pub account_code: String,
    pub account_name: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
}

impl JournalEntrySuggestion {
    pub fn is_balanced(&self) -> bool {
        let total_debits: i64 = self.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = self.lines.iter().map(|l| l.credit_minor).sum();
        total_debits == total_credits
    }
}
