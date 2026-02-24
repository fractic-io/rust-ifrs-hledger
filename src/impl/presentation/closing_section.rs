use std::collections::HashSet;

use chrono::{Datelike as _, NaiveDate};

#[derive(Debug, Clone)]
struct ClosingRecord {
    date: NaiveDate,
    statement: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClosingSection {
    account_statements: HashSet<String>,
    closing_records: Vec<ClosingRecord>,
}

impl ClosingSection {
    pub(crate) fn from_sources<'a>(sources: impl IntoIterator<Item = &'a str>) -> Self {
        let mut section = Self::default();
        for source in sources {
            section.ingest(source);
        }

        section.closing_records.sort_by(|a, b| {
            a.date
                .cmp(&b.date)
                .then_with(|| a.statement.cmp(&b.statement))
        });
        section
    }

    pub(crate) fn render(&self) -> Option<String> {
        if self.closing_records.is_empty() {
            return None;
        }

        let title = self.title();
        let mut records = Vec::new();

        // Keep account declarations stable and deduplicated before entries.
        let mut account_statements: Vec<&str> =
            self.account_statements.iter().map(String::as_str).collect();
        account_statements.sort();
        records.extend(account_statements.into_iter().map(String::from));
        records.extend(self.closing_records.iter().map(|record| record.statement.clone()));

        Some(format!(
            "{}\n\n{}",
            comment_header(&title),
            records.join("\n")
        ))
    }

    fn ingest(&mut self, source: &str) {
        let mut current_record_date: Option<NaiveDate> = None;
        let mut current_record_lines: Vec<String> = Vec::new();

        for line in source.lines() {
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with(';') {
                continue;
            }
            if trimmed_start.starts_with("account ") {
                self.account_statements.insert(line.trim_end().to_string());
                continue;
            }
            if let Some(date) = parse_statement_date(trimmed_start) {
                // A new dated line begins a new multiline closing record.
                self.flush_record(&mut current_record_date, &mut current_record_lines);
                current_record_date = Some(date);
                current_record_lines.push(line.trim_end().to_string());
                continue;
            }
            if current_record_date.is_some() {
                current_record_lines.push(line.trim_end().to_string());
            }
        }

        self.flush_record(&mut current_record_date, &mut current_record_lines);
    }

    fn flush_record(
        &mut self,
        current_record_date: &mut Option<NaiveDate>,
        current_record_lines: &mut Vec<String>,
    ) {
        let Some(date) = current_record_date.take() else {
            return;
        };
        let statement = current_record_lines.join("\n");
        current_record_lines.clear();

        self.closing_records.push(ClosingRecord { date, statement });
    }

    fn title(&self) -> String {
        let start_year = self
            .closing_records
            .first()
            .map(|record| record.date.year())
            .unwrap_or_default();
        let end_year = self
            .closing_records
            .last()
            .map(|record| record.date.year())
            .unwrap_or_default();

        if start_year == end_year {
            format!("Closing {}", start_year)
        } else {
            format!("Closing {}-{}", start_year, end_year)
        }
    }
}

fn parse_statement_date(line: &str) -> Option<NaiveDate> {
    if line.len() < 10 {
        return None;
    }
    NaiveDate::parse_from_str(&line[..10], "%Y-%m-%d").ok()
}

fn comment_header(title: &str) -> String {
    // Match the existing 100-char section header style used in ledger output.
    let mut header = format!("; --- {} ", title);
    if header.len() < 100 {
        header.push_str(&"-".repeat(100 - header.len()));
    }
    header
}
