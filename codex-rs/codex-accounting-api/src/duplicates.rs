use codex_bank_ingest::NormalizedBankTransaction;
use std::collections::BTreeSet;

/// Collect unique duplicate set labels from transactions, preferring the external reference.
pub fn duplicate_set_labels(transactions: &[NormalizedBankTransaction]) -> Vec<String> {
    let mut labels = BTreeSet::new();
    for tx in transactions {
        if tx.duplicate_metadata.total_occurrences > 1 {
            let label = tx
                .source_reference
                .clone()
                .or_else(|| tx.duplicate_metadata.group_key.clone())
                .unwrap_or_else(|| tx.transaction_id.clone());
            labels.insert(label);
        }
    }
    labels.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_bank_ingest::DuplicateMetadata;
    use pretty_assertions::assert_eq;

    fn sample_tx(
        id: &str,
        reference: Option<&str>,
        group: Option<&str>,
        total: usize,
    ) -> NormalizedBankTransaction {
        NormalizedBankTransaction {
            transaction_id: id.to_string(),
            account_id: "acc".into(),
            posted_date: chrono::NaiveDate::from_ymd_opt(2024, 10, 23).expect("date"),
            amount_minor: 100,
            currency: "USD".into(),
            description: "desc".into(),
            source_reference: reference.map(std::string::ToString::to_string),
            source_checksum: None,
            is_void: false,
            duplicate_metadata: DuplicateMetadata {
                group_key: group.map(std::string::ToString::to_string),
                total_occurrences: total,
                discarded_ids: Vec::new(),
            },
            currency_validation: Default::default(),
        }
    }

    #[test]
    fn ignores_unique_transactions() {
        let txs = vec![sample_tx("t1", Some("REF-1"), None, 1)];
        assert_eq!(duplicate_set_labels(&txs), Vec::<String>::new());
    }

    #[test]
    fn prefers_source_reference() {
        let txs = vec![
            sample_tx("t1", Some("REF-1"), Some("grp-1"), 2),
            sample_tx("t2", Some("REF-1"), Some("grp-1"), 1),
        ];
        assert_eq!(duplicate_set_labels(&txs), vec!["REF-1".to_string()]);
    }

    #[test]
    fn falls_back_to_group_then_transaction() {
        let txs = vec![
            sample_tx("t1", None, Some("grp-1"), 3),
            sample_tx("t2", None, Some("grp-1"), 2),
            sample_tx("t3", None, None, 4),
        ];
        assert_eq!(
            duplicate_set_labels(&txs),
            vec!["grp-1".to_string(), "t3".to_string()]
        );
    }
}
