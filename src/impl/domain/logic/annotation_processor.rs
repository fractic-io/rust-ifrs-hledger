use std::collections::{HashMap, HashSet};

use fractic_server_error::ServerError;

use crate::entities::{Annotation, FinancialRecords, NotesToFinancialRecords};

pub(crate) struct AnnotationProcessor<'a> {
    records: &'a FinancialRecords,
}

impl<'a> AnnotationProcessor<'a> {
    pub(crate) fn new(records: &'a FinancialRecords) -> Self {
        Self { records }
    }

    pub(crate) fn process(self) -> Result<NotesToFinancialRecords, ServerError> {
        let unknown_label = "Unknown".to_string();
        let annotations_map: HashMap<Annotation, HashSet<String>> = self
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
                    .map(move |annotation| (annotation.clone(), label))
            })
            .fold(HashMap::new(), |mut map, (annotation, label)| {
                map.entry(annotation).or_default().insert(label.to_string());
                map
            });

        let notes = annotations_map
            .into_iter()
            .map(|(annotation, labels)| {
                (
                    annotation.to_string(),
                    labels.into_iter().collect::<Vec<_>>().join(", "),
                )
            })
            .collect();

        Ok(NotesToFinancialRecords {
            transaction_notes: notes,
        })
    }
}
