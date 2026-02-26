use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use fractic_server_error::ServerError;

use crate::entities::{EndOfYearEntry, FinancialRecords, NotesToFinancialRecords};

pub(crate) struct AnnotationProcessor<'a> {
    records: &'a FinancialRecords,
}

impl<'a> AnnotationProcessor<'a> {
    pub(crate) fn new(records: &'a FinancialRecords) -> Self {
        Self { records }
    }

    pub(crate) fn process(self) -> Result<NotesToFinancialRecords, ServerError> {
        let unknown_label = "Unknown".to_string();
        let annotations_map: BTreeMap<String, BTreeSet<String>> = self
            .records
            .transactions
            .iter()
            .flat_map(|tx| {
                let label = self
                    .records
                    .label_lookup
                    .get(&tx.spec_id)
                    .map_or_else(|| &unknown_label, |label| &label.description);
                self.records
                    .annotations_lookup
                    .get(&tx.spec_id)
                    .into_iter()
                    .flatten()
                    .map(move |annotation| (annotation.to_string(), label))
            })
            .fold(BTreeMap::new(), |mut map, (annotation, label)| {
                map.entry(annotation).or_default().insert(label.to_string());
                map
            });

        let transaction_notes = annotations_map
            .into_iter()
            .map(|(annotation, labels)| {
                (
                    annotation,
                    labels.into_iter().collect::<Vec<_>>().join(", "),
                )
            })
            .collect::<Vec<_>>();

        let general_notes = {
            let mut v = Vec::new();
            v.extend(self.unreimbursed_transaction_notes()?);
            v.extend(self.manual_correction_notes());
            v
        };

        Ok(NotesToFinancialRecords {
            transaction_notes,
            general_notes,
        })
    }

    fn unreimbursed_transaction_notes(&self) -> Result<Vec<(String, String)>, ServerError> {
        let mut n = Vec::new();
        if !self.records.unreimbursed_entries.is_empty() {
            n.push((
                "Currently, unreimbursed transactions remain.".to_string(),
                self.records
                    .unreimbursed_entries
                    .iter()
                    .map(|(account, _)| account.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .map(|account| {
                        let entries = self
                            .records
                            .unreimbursed_entries
                            .iter()
                            .filter(|(a, _)| a == &account)
                            .collect::<Vec<_>>();
                        format!(
                            "{} ({} | {:.2} | {})",
                            account.0.as_ref().unwrap_or(&"Unknown".to_string()),
                            entries.len(),
                            entries
                                .iter()
                                .map(|(_, entry)| entry.total_amount)
                                .sum::<f64>(),
                            entries
                                .iter()
                                .flat_map(|(_, entry)| entry
                                    .credit_postings
                                    .iter()
                                    .map(|p| p.account.ledger())
                                    .collect::<Vec<_>>())
                                .collect::<BTreeSet<_>>()
                                .into_iter()
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
            let cashflow_tags_of_unreimbursed_entries: BTreeSet<_> = self
                .records
                .unreimbursed_entries
                .iter()
                .flat_map(|(_, entry)| {
                    entry
                        .credit_postings
                        .iter()
                        .filter_map(|p| p.account.cashflow_tag(p.amount))
                })
                .collect();
            if !cashflow_tags_of_unreimbursed_entries.is_empty() {
                n.push((
                    "WARNING: Unreimbursed transactions are associated with cashflow tracing tags."
                        .to_string(),
                    cashflow_tags_of_unreimbursed_entries
                        .into_iter()
                        .map(|tag| tag.value())
                        .collect::<Vec<_>>()
                        .join(", "),
                ));
            }
        }
        Ok(n)
    }

    fn manual_correction_notes(&self) -> Vec<(String, String)> {
        let mut examples_by_year: BTreeMap<i32, BTreeSet<String>> = BTreeMap::new();
        for entry in &self.records.eoy_entries {
            if let EndOfYearEntry::Correction {
                date, description, ..
            } = entry
            {
                examples_by_year.entry(date.year()).or_default().insert(
                    description
                        .as_ref()
                        .filter(|s| !s.trim().is_empty())
                        .cloned()
                        .unwrap_or_else(|| date.format("%F").to_string()),
                );
            }
        }
        examples_by_year
            .into_iter()
            .map(|(year, examples)| {
                (
                    format!("WARNING: {} had manual corrections.", year),
                    examples.into_iter().collect::<Vec<_>>().join(", "),
                )
            })
            .collect()
    }
}
