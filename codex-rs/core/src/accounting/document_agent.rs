use super::types::*;
use codex_ledger::Account;

// These will need proper imports when integrating with actual services:
// use codex_ocr::OcrService;
// use codex_accounting_api::LedgerFacade;
// use some_chatgpt_client::ChatGPTClient;

pub struct DocumentAgent {
    // ocr_service: Arc<dyn OcrService>,
    // ledger_facade: Arc<LedgerFacade>,
    // chatgpt_client: Arc<ChatGPTClient>,
}

impl Default for DocumentAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentAgent {
    pub fn new(// ocr_service: Arc<dyn OcrService>,
        // ledger_facade: Arc<LedgerFacade>,
        // chatgpt_client: Arc<ChatGPTClient>,
    ) -> Self {
        Self {
            // ocr_service,
            // ledger_facade,
            // chatgpt_client,
        }
    }

    pub async fn process_document(
        &self,
        _upload_id: &str,
        company_id: &str,
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // Step 1: Get OCR text
        // For now, mock until OCR service integrated:
        let mock_ocr_text =
            "INVOICE\nAcme Supplies\nInvoice: INV-001\nDate: Jan 15, 2024\nTotal: $108.00";

        // Step 2: Extract structured data
        let invoice_data = self.extract_invoice_data(mock_ocr_text, company_id).await?;

        // Step 3: Get chart of accounts
        // Mock for now:
        let mock_accounts = vec![
            // You'll need to create proper Account structs from codex_ledger
        ];

        // Step 4: Suggest journal entry
        let suggestion = self
            .suggest_journal_entry(&invoice_data, &mock_accounts)
            .await?;

        // Step 5: Final validation
        if !suggestion.is_balanced() {
            return Err("Entry validation failed: not balanced".into());
        }

        if suggestion.confidence < 0.5 {
            return Err(format!("Confidence too low: {}", suggestion.confidence).into());
        }

        Ok(suggestion)
    }

    async fn extract_invoice_data(
        &self,
        ocr_text: &str,
        _company_id: &str,
    ) -> Result<InvoiceData, Box<dyn std::error::Error>> {
        let _system_prompt = "You are an expert accountant specializing in invoice processing. \
            Extract structured data from invoice text with high accuracy.";

        let _user_prompt = format!(
            "Extract the following information from this invoice OCR text:\n\
            - Vendor name (the company providing goods/services)\n\
            - Invoice number (if present)\n\
            - Date (convert to YYYY-MM-DD format)\n\
            - Line items with descriptions and amounts\n\
            - Subtotal (before tax)\n\
            - Tax amount\n\
            - Total amount\n\
            - Your confidence level (0.0 to 1.0)\n\n\
            OCR Text:\n{ocr_text}\n\n\
            Return ONLY valid JSON matching this exact schema:\n\
            {{\n\
              \"vendor\": \"string\",\n\
              \"invoice_number\": \"string or null\",\n\
              \"date\": \"YYYY-MM-DD\",\n\
              \"line_items\": [\n\
                {{\n\
                  \"description\": \"string\",\n\
                  \"quantity\": number or null,\n\
                  \"unit_price\": number or null,\n\
                  \"amount\": number\n\
                }}\n\
              ],\n\
              \"subtotal\": number,\n\
              \"tax_amount\": number,\n\
              \"total\": number,\n\
              \"confidence\": number (0.0-1.0)\n\
            }}"
        );

        // Call ChatGPT (you'll need to find actual client method):
        // let response = self.chatgpt_client.chat(system_prompt, &user_prompt).await?;

        // For now, return mock until ChatGPT client wired up:
        let response = r#"{
            "vendor": "Mock Vendor",
            "invoice_number": "INV-001",
            "date": "2024-01-15",
            "line_items": [{"description": "Mock item", "quantity": null, "unit_price": null, "amount": 100.0}],
            "subtotal": 100.0,
            "tax_amount": 8.0,
            "total": 108.0,
            "confidence": 0.95
        }"#;

        let invoice_data: InvoiceData = serde_json::from_str(response)?;
        Ok(invoice_data)
    }

    async fn suggest_journal_entry(
        &self,
        invoice_data: &InvoiceData,
        accounts: &[Account],
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // Format accounts list
        let accounts_list = accounts
            .iter()
            .map(|a| format!("{} - {} ({:?})", a.code, a.name, a.account_type))
            .collect::<Vec<_>>()
            .join("\n");

        let _system_prompt = "You are an expert accountant. Suggest journal entries following \
            double-entry bookkeeping rules. Debits must always equal credits. \
            Amounts should be in minor currency units (cents).";

        let _user_prompt = format!(
            "Invoice details:\n\
            Vendor: {}\n\
            Date: {}\n\
            Total: ${:.2}\n\
            Tax: ${:.2}\n\
            Subtotal: ${:.2}\n\n\
            Available accounts:\n{}\n\n\
            Task: Suggest a balanced journal entry to record this expense invoice.\n\
            The entry must follow double-entry bookkeeping (debits = credits).\n\
            Use minor currency units (multiply dollars by 100 for cents).\n\n\
            Return ONLY valid JSON:\n\
            {{\n\
              \"lines\": [\n\
                {{\n\
                  \"account_code\": \"string\",\n\
                  \"account_name\": \"string\",\n\
                  \"debit_minor\": integer (cents),\n\
                  \"credit_minor\": integer (cents)\n\
                }}\n\
              ],\n\
              \"memo\": \"string describing the transaction\",\n\
              \"confidence\": number (0.0-1.0),\n\
              \"reasoning\": \"string explaining your account choices\"\n\
            }}",
            invoice_data.vendor,
            invoice_data.date,
            invoice_data.total,
            invoice_data.tax_amount,
            invoice_data.subtotal,
            accounts_list
        );

        // Call ChatGPT:
        // let response = self.chatgpt_client.chat(system_prompt, &user_prompt).await?;

        // Mock for now:
        let total_minor = (invoice_data.total * 100.0) as i64;
        let response = format!(
            r#"{{
            "lines": [
                {{"account_code": "5000", "account_name": "Expenses", "debit_minor": {}, "credit_minor": 0}},
                {{"account_code": "1000", "account_name": "Cash", "debit_minor": 0, "credit_minor": {}}}
            ],
            "memo": "Expense from {}",
            "confidence": 0.9,
            "reasoning": "Standard expense entry"
        }}"#,
            total_minor, total_minor, invoice_data.vendor
        );

        let suggestion: JournalEntrySuggestion = serde_json::from_str(&response)?;

        // Validate balance
        if !suggestion.is_balanced() {
            return Err("AI suggested unbalanced entry".into());
        }

        Ok(suggestion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_validates_balance() {
        let suggestion = JournalEntrySuggestion {
            lines: vec![
                SuggestedLine {
                    account_code: "5000".into(),
                    account_name: "Expense".into(),
                    debit_minor: 100,
                    credit_minor: 0,
                },
                SuggestedLine {
                    account_code: "1000".into(),
                    account_name: "Cash".into(),
                    debit_minor: 0,
                    credit_minor: 100,
                },
            ],
            memo: "Test".into(),
            confidence: 0.9,
            reasoning: "Test reasoning".into(),
        };

        assert!(suggestion.is_balanced());
    }

    #[test]
    fn suggestion_detects_imbalance() {
        let suggestion = JournalEntrySuggestion {
            lines: vec![
                SuggestedLine {
                    account_code: "5000".into(),
                    account_name: "Expense".into(),
                    debit_minor: 100,
                    credit_minor: 0,
                },
                SuggestedLine {
                    account_code: "1000".into(),
                    account_name: "Cash".into(),
                    debit_minor: 0,
                    credit_minor: 50,
                },
            ],
            memo: "Test".into(),
            confidence: 0.9,
            reasoning: "Test".into(),
        };

        assert!(!suggestion.is_balanced());
    }
}
