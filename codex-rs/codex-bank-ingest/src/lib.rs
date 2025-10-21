#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;

use chrono::NaiveDate;
use csv::StringRecord;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use thiserror::Error;

fn default_date_format() -> String {
    "%Y-%m-%d".into()
}

fn default_amount_factor() -> i64 {
    100
}

#[derive(Debug, Error)]
pub enum BankIngestError {
    #[error("parser not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("invalid payload: {0}")]
    Invalid(String),
    #[error("missing column {0}")]
    MissingColumn(String),
    #[error("csv error: {0}")]
    Csv(String),
    #[error("parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyValidation {
    pub is_iso4217: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for CurrencyValidation {
    fn default() -> Self {
        Self {
            is_iso4217: true,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(default)]
    pub total_occurrences: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discarded_ids: Vec<String>,
}

impl Default for DuplicateMetadata {
    fn default() -> Self {
        Self {
            group_key: None,
            total_occurrences: 1,
            discarded_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedBankTransaction {
    pub transaction_id: String,
    pub account_id: String,
    pub posted_date: NaiveDate,
    pub amount_minor: i64,
    pub currency: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_checksum: Option<String>,
    #[serde(default)]
    pub is_void: bool,
    #[serde(default)]
    pub duplicate_metadata: DuplicateMetadata,
    #[serde(default)]
    pub currency_validation: CurrencyValidation,
}

impl NormalizedBankTransaction {
    pub fn dedupe_key(&self) -> String {
        if let Some(reference) = &self.source_reference {
            return reference.clone();
        }
        format!(
            "{}|{}|{}",
            self.transaction_id, self.amount_minor, self.posted_date
        )
    }

    fn build_checksum_fields(&self) -> [String; 4] {
        [
            self.transaction_id.clone(),
            self.account_id.clone(),
            self.posted_date.to_string(),
            self.amount_minor.to_string(),
        ]
    }

    fn ensure_checksum(&mut self) {
        if self.source_checksum.is_some() {
            return;
        }
        let joined = self.build_checksum_fields();
        self.source_checksum = Some(compute_checksum(&joined));
    }
}

fn compute_checksum(fields: &[String; 4]) -> String {
    let mut hasher = Sha256::new();
    for field in fields {
        hasher.update(field.as_bytes());
        hasher.update(b"|");
    }
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DedupeMetrics {
    pub kept: usize,
    pub dropped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupeOutcome {
    pub transactions: Vec<NormalizedBankTransaction>,
    pub metrics: DedupeMetrics,
}

#[must_use]
pub fn dedupe_transactions(transactions: Vec<NormalizedBankTransaction>) -> DedupeOutcome {
    let mut grouped: HashMap<String, Vec<(usize, NormalizedBankTransaction)>> = HashMap::new();
    for (index, mut tx) in transactions.into_iter().enumerate() {
        let key = tx.dedupe_key();
        tx.duplicate_metadata.group_key = Some(key.clone());
        grouped.entry(key).or_default().push((index, tx));
    }

    let mut metrics = DedupeMetrics::default();
    let mut ordered: Vec<(usize, NormalizedBankTransaction)> = Vec::new();
    for (_, mut entries) in grouped {
        entries.sort_by_key(|(idx, _)| *idx);
        let mut iter = entries.into_iter();
        if let Some((first_index, mut primary)) = iter.next() {
            let mut duplicates = Vec::new();
            for (_, tx) in iter {
                duplicates.push(tx.transaction_id);
            }
            metrics.kept += 1;
            metrics.dropped += duplicates.len();
            primary.duplicate_metadata.total_occurrences = duplicates.len() + 1;
            primary.duplicate_metadata.discarded_ids = duplicates;
            primary.ensure_checksum();
            ordered.push((first_index, primary));
        }
    }

    ordered.sort_by_key(|(idx, _)| *idx);
    let deduped = ordered
        .into_iter()
        .map(|(_, tx)| tx)
        .collect::<Vec<NormalizedBankTransaction>>();

    DedupeOutcome {
        transactions: deduped,
        metrics,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CsvParserProfile {
    pub transaction_id: String,
    pub account_id: String,
    pub posted_date: String,
    pub amount: String,
    pub currency: String,
    pub description: String,
    #[serde(default)]
    pub source_reference: Option<String>,
    #[serde(default)]
    pub source_checksum: Option<String>,
    #[serde(default)]
    pub voided: Option<String>,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_amount_factor")]
    pub amount_minor_factor: i64,
}

impl Default for CsvParserProfile {
    fn default() -> Self {
        Self {
            transaction_id: "transaction_id".into(),
            account_id: "account_id".into(),
            posted_date: "posted_date".into(),
            amount: "amount".into(),
            currency: "currency".into(),
            description: "description".into(),
            source_reference: Some("source_reference".into()),
            source_checksum: Some("checksum".into()),
            voided: Some("voided".into()),
            date_format: default_date_format(),
            amount_minor_factor: default_amount_factor(),
        }
    }
}

struct CsvIndexes {
    transaction_id: usize,
    account_id: usize,
    posted_date: usize,
    amount: usize,
    currency: usize,
    description: usize,
    source_reference: Option<usize>,
    source_checksum: Option<usize>,
    voided: Option<usize>,
}

impl CsvParserProfile {
    fn indexes(&self, headers: &StringRecord) -> Result<CsvIndexes, BankIngestError> {
        Ok(CsvIndexes {
            transaction_id: find_index(headers, &self.transaction_id)?,
            account_id: find_index(headers, &self.account_id)?,
            posted_date: find_index(headers, &self.posted_date)?,
            amount: find_index(headers, &self.amount)?,
            currency: find_index(headers, &self.currency)?,
            description: find_index(headers, &self.description)?,
            source_reference: optional_index(headers, self.source_reference.as_deref())?,
            source_checksum: optional_index(headers, self.source_checksum.as_deref())?,
            voided: optional_index(headers, self.voided.as_deref())?,
        })
    }
}

fn find_index(headers: &StringRecord, column: &str) -> Result<usize, BankIngestError> {
    headers
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(column))
        .ok_or_else(|| BankIngestError::MissingColumn(column.into()))
}

fn optional_index(
    headers: &StringRecord,
    column: Option<&str>,
) -> Result<Option<usize>, BankIngestError> {
    column.map(|name| find_index(headers, name)).transpose()
}

#[derive(Clone)]
pub struct CsvBankParser {
    profile: CsvParserProfile,
}

impl CsvBankParser {
    pub fn new(profile: CsvParserProfile) -> Self {
        Self { profile }
    }

    fn build_transaction(
        &self,
        record: &StringRecord,
        indexes: &CsvIndexes,
    ) -> Result<NormalizedBankTransaction, BankIngestError> {
        let transaction_id = record
            .get(indexes.transaction_id)
            .ok_or_else(|| BankIngestError::Invalid("transaction_id missing".into()))?
            .trim()
            .to_owned();

        let account_id = record
            .get(indexes.account_id)
            .ok_or_else(|| BankIngestError::Invalid("account_id missing".into()))?
            .trim()
            .to_owned();

        let posted_date_raw = record
            .get(indexes.posted_date)
            .ok_or_else(|| BankIngestError::Invalid("posted_date missing".into()))?
            .trim();
        let posted_date = NaiveDate::parse_from_str(posted_date_raw, &self.profile.date_format)
            .map_err(|err| {
                BankIngestError::Parse(format!("invalid date {posted_date_raw}: {err}"))
            })?;

        let amount_raw = record
            .get(indexes.amount)
            .ok_or_else(|| BankIngestError::Invalid("amount missing".into()))?
            .trim();
        let amount_minor = parse_amount(amount_raw, self.profile.amount_minor_factor)?;

        let currency = record
            .get(indexes.currency)
            .ok_or_else(|| BankIngestError::Invalid("currency missing".into()))?
            .trim()
            .to_uppercase();

        let description = record
            .get(indexes.description)
            .ok_or_else(|| BankIngestError::Invalid("description missing".into()))?
            .trim()
            .to_owned();

        let source_reference = indexes
            .source_reference
            .and_then(|idx| record.get(idx))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let checksum_from_source = indexes
            .source_checksum
            .and_then(|idx| record.get(idx))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let is_void = indexes
            .voided
            .and_then(|idx| record.get(idx))
            .map(|value| is_truthy(value.trim()))
            .unwrap_or(false);

        let currency_validation = validate_currency(&currency)?;

        let mut transaction = NormalizedBankTransaction {
            transaction_id,
            account_id,
            posted_date,
            amount_minor,
            currency,
            description,
            source_reference,
            source_checksum: checksum_from_source,
            is_void,
            duplicate_metadata: DuplicateMetadata::default(),
            currency_validation,
        };
        transaction.ensure_checksum();
        Ok(transaction)
    }
}

impl Default for CsvBankParser {
    fn default() -> Self {
        Self::new(CsvParserProfile::default())
    }
}

impl BankStatementParser for CsvBankParser {
    fn parse(&self, input: &str) -> Result<Vec<NormalizedBankTransaction>, BankIngestError> {
        let mut reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(input.as_bytes());
        let headers = reader
            .headers()
            .map_err(|err| BankIngestError::Csv(err.to_string()))?
            .clone();
        let indexes = self.profile.indexes(&headers)?;
        let mut transactions = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|err| BankIngestError::Csv(err.to_string()))?;
            if record.iter().all(|field| field.trim().is_empty()) {
                continue;
            }
            let transaction = self.build_transaction(&record, &indexes)?;
            transactions.push(transaction);
        }
        Ok(transactions)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OfxParserProfile {
    pub amount_minor_factor: i64,
}

impl Default for OfxBankParser {
    fn default() -> Self {
        Self {
            profile: OfxParserProfile {
                amount_minor_factor: default_amount_factor(),
            },
        }
    }
}

#[derive(Clone)]
pub struct OfxBankParser {
    profile: OfxParserProfile,
}

impl OfxBankParser {
    pub fn new(profile: OfxParserProfile) -> Self {
        Self { profile }
    }

    fn build_transaction(
        &self,
        fields: &HashMap<String, String>,
        account_id: &str,
        currency: &str,
    ) -> Result<NormalizedBankTransaction, BankIngestError> {
        let transaction_id = fields
            .get("FITID")
            .ok_or_else(|| {
                let keys = fields.keys().cloned().collect::<Vec<String>>().join(", ");
                BankIngestError::Invalid(format!("OFX missing FITID; saw keys [{keys}]"))
            })?
            .trim()
            .to_owned();
        let amount_raw = fields
            .get("TRNAMT")
            .ok_or_else(|| BankIngestError::Invalid("OFX missing TRNAMT".into()))?
            .trim();
        let amount_minor = parse_amount(amount_raw, self.profile.amount_minor_factor)?;
        let date_raw = fields
            .get("DTPOSTED")
            .ok_or_else(|| BankIngestError::Invalid("OFX missing DTPOSTED".into()))?;
        let date = parse_ofx_date(date_raw)?;
        let description = fields
            .get("NAME")
            .or_else(|| fields.get("MEMO"))
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|| "Unspecified".into());
        let source_reference = fields
            .get("CHECKNUM")
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let is_void = fields
            .get("TRNTYPE")
            .map(|value| value.trim().eq_ignore_ascii_case("VOID"))
            .unwrap_or(false);

        let raw_currency = fields
            .get("CURRENCY")
            .map(|value| value.trim().to_uppercase())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| currency.to_uppercase());
        let currency_validation = validate_currency(&raw_currency)?;

        let mut transaction = NormalizedBankTransaction {
            transaction_id,
            account_id: account_id.to_owned(),
            posted_date: date,
            amount_minor,
            currency: raw_currency,
            description,
            source_reference,
            source_checksum: None,
            is_void,
            duplicate_metadata: DuplicateMetadata::default(),
            currency_validation,
        };
        transaction.ensure_checksum();
        Ok(transaction)
    }
}

impl BankStatementParser for OfxBankParser {
    fn parse(&self, input: &str) -> Result<Vec<NormalizedBankTransaction>, BankIngestError> {
        let mut account_id = String::new();
        let mut currency = String::new();
        let mut current: HashMap<String, String> = HashMap::new();
        let mut transactions = Vec::new();
        let mut in_transaction = false;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.eq_ignore_ascii_case("<STMTTRN>") {
                if in_transaction && !current.is_empty() && !account_id.is_empty() {
                    let tx = self.build_transaction(&current, &account_id, &currency)?;
                    transactions.push(tx);
                }
                current.clear();
                in_transaction = true;
                continue;
            }
            if trimmed.eq_ignore_ascii_case("</STMTTRN>") {
                if in_transaction && !current.is_empty() && !account_id.is_empty() {
                    let tx = self.build_transaction(&current, &account_id, &currency)?;
                    transactions.push(tx);
                }
                current.clear();
                in_transaction = false;
                continue;
            }
            if let Some(value) = extract_tag_value(trimmed, "ACCTID") {
                account_id = value.to_owned();
                continue;
            }
            if let Some(value) = extract_tag_value(trimmed, "CURDEF") {
                currency = value.to_owned();
                continue;
            }
            if in_transaction {
                if let Some((tag, value)) = split_tag(trimmed) {
                    current.insert(tag.to_owned(), value.to_owned());
                }
            } else if let Some((tag, value)) = split_tag(trimmed)
                && tag.eq_ignore_ascii_case("CURRENCY")
            {
                currency = value.to_owned();
            }
        }

        if in_transaction && !current.is_empty() && !account_id.is_empty() {
            let tx = self.build_transaction(&current, &account_id, &currency)?;
            transactions.push(tx);
        }

        if transactions.is_empty() {
            return Err(BankIngestError::Invalid(
                "OFX payload did not contain any STMTTRN blocks".into(),
            ));
        }

        Ok(transactions)
    }
}

pub trait BankStatementParser {
    fn parse(&self, input: &str) -> Result<Vec<NormalizedBankTransaction>, BankIngestError>;
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "true" | "t" | "1" | "yes" | "y"
    )
}

fn parse_amount(value: &str, factor: i64) -> Result<i64, BankIngestError> {
    if factor <= 0 {
        return Err(BankIngestError::Invalid(
            "amount_minor_factor must be positive".into(),
        ));
    }
    let mut cleaned = value.trim().replace(',', "");
    if cleaned.is_empty() {
        return Err(BankIngestError::Invalid("amount cannot be empty".into()));
    }
    let negative = cleaned.starts_with('-');
    if negative {
        cleaned.remove(0);
    }
    let parts: Vec<&str> = cleaned.split('.').collect();
    let integer = parts
        .first()
        .unwrap_or(&"0")
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    let fraction = if parts.len() > 1 {
        parts[1]
            .chars()
            .take(6)
            .filter(char::is_ascii_digit)
            .collect::<String>()
    } else {
        String::new()
    };
    let mut amount = integer.parse::<i64>().map_err(|err| {
        BankIngestError::Parse(format!(
            "failed to parse integer component {integer}: {err}"
        ))
    })?;
    amount = amount
        .checked_mul(factor)
        .ok_or_else(|| BankIngestError::Parse("amount overflow".into()))?;
    if !fraction.is_empty() {
        let fraction_scale = 10_i64.pow(fraction.len() as u32);
        let fraction_value = fraction
            .parse::<i64>()
            .map_err(|err| BankIngestError::Parse(format!("invalid fraction {fraction}: {err}")))?;
        amount = amount
            .checked_add((fraction_value * factor) / fraction_scale)
            .ok_or_else(|| BankIngestError::Parse("fraction overflow".into()))?;
    }
    if negative {
        amount = -amount;
    }
    Ok(amount)
}

fn validate_currency(code: &str) -> Result<CurrencyValidation, BankIngestError> {
    if code.len() == 3 && code.chars().all(|ch| ch.is_ascii_uppercase()) {
        Ok(CurrencyValidation::default())
    } else {
        Err(BankIngestError::Invalid(format!(
            "invalid ISO-4217 currency code {code}"
        )))
    }
}

fn extract_tag_value<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    if !line.starts_with('<') {
        return None;
    }
    let end = line.find('>')?;
    let name = &line[1..end];
    if !name.eq_ignore_ascii_case(tag) {
        return None;
    }
    Some(line[end + 1..].trim())
}

fn split_tag(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with('<') {
        return None;
    }
    let end = line.find('>')?;
    if end + 1 >= line.len() {
        return None;
    }
    let name = &line[1..end];
    let value = line[end + 1..].trim();
    Some((name, value))
}

fn parse_ofx_date(raw: &str) -> Result<NaiveDate, BankIngestError> {
    let digits: String = raw.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 8 {
        return Err(BankIngestError::Invalid(format!("invalid OFX date {raw}")));
    }
    let date_str = &digits[0..8];
    NaiveDate::parse_from_str(date_str, "%Y%m%d")
        .map_err(|err| BankIngestError::Parse(format!("invalid OFX date {raw}: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::from_str;

    #[test]
    fn csv_parser_parses_profiled_sample() {
        let profile: CsvParserProfile =
            from_str(include_str!("../tests/fixtures/csv/profile.json"))
                .expect("profile fixture must be valid");
        let parser = CsvBankParser::new(profile);
        let transactions = parser
            .parse(include_str!("../tests/fixtures/csv/sample.csv"))
            .expect("csv parser should succeed");

        assert_eq!(transactions.len(), 4);
        assert!(transactions[0].source_checksum.is_some());
        let eur_tx = transactions
            .iter()
            .find(|tx| tx.transaction_id == "txn-eur-void")
            .expect("EUR transaction should exist");
        assert_eq!(eur_tx.currency, "EUR");
        assert!(eur_tx.is_void);
        assert!(eur_tx.currency_validation.is_iso4217);
        let duplicate_count = transactions
            .iter()
            .filter(|tx| tx.source_reference.as_deref() == Some("REF-002"))
            .count();
        assert_eq!(duplicate_count, 2);
    }

    #[test]
    fn csv_parser_rejects_invalid_currency() {
        let profile: CsvParserProfile =
            from_str(include_str!("../tests/fixtures/csv/profile.json"))
                .expect("profile fixture must be valid");
        let parser = CsvBankParser::new(profile);
        let invalid_payload = "\
transaction_id,account_id,posted_date,amount,currency,description,source_reference,checksum,voided
bad-txn,acct-1,2024-10-01,10.00,US,Coffee,,,
";
        let err = parser
            .parse(invalid_payload)
            .expect_err("invalid currency values should fail");
        match err {
            BankIngestError::Invalid(message) => {
                assert!(message.contains("invalid ISO-4217"));
            }
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[test]
    fn dedupe_transactions_reports_metrics() {
        let profile: CsvParserProfile =
            from_str(include_str!("../tests/fixtures/csv/profile.json"))
                .expect("profile fixture must be valid");
        let parser = CsvBankParser::new(profile);
        let parsed = parser
            .parse(include_str!("../tests/fixtures/csv/sample.csv"))
            .expect("csv parser should succeed");
        let outcome = dedupe_transactions(parsed);
        assert_eq!(outcome.metrics.kept, 3);
        assert_eq!(outcome.metrics.dropped, 1);
        let duplicate = outcome
            .transactions
            .iter()
            .find(|tx| tx.duplicate_metadata.total_occurrences > 1)
            .expect("duplicate group should exist");
        assert_eq!(duplicate.duplicate_metadata.total_occurrences, 2);
        assert_eq!(
            duplicate.duplicate_metadata.discarded_ids,
            vec![String::from("txn-duplicate-1")]
        );
    }

    #[test]
    fn ofx_parser_extracts_transactions() {
        let parser = OfxBankParser::default();
        let transactions = parser
            .parse(include_str!("../tests/fixtures/ofx/sample.ofx"))
            .expect("ofx parser should succeed");
        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].transaction_id, "OFX-100");
        assert_eq!(transactions[0].currency, "USD");
        assert_eq!(transactions[1].currency, "EUR");
    }
}
