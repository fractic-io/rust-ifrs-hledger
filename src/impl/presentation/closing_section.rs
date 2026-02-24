use std::collections::HashSet;

use chrono::{Datelike as _, NaiveDate};

/// Represents the 'closing' section appended to the end of the ledger,
/// including all EOY-closing transactions (retaining earnings, etc.).
#[derive(Debug, Clone, Default)]
pub(crate) struct ClosingSection {
    /// Additional account declarations encountered in the closing statements.
    /// These are deduped and sorted.
    account_declarations: HashSet<String>,
    closing_transactions: Vec<ClosingTransaction>,
}

/// Represents a single year's closing transaction.
#[derive(Debug, Clone)]
struct ClosingTransaction {
    date: NaiveDate,
    raw_statement: String,
}

// Crate-public interface.
// ----------------------------------------------------------------------------

impl ClosingSection {
    pub(crate) fn new<'a>(closing_statements: impl IntoIterator<Item = &'a str>) -> Self {
        let mut section = Self::default();
        for statement in closing_statements {
            section.ingest(statement);
        }

        section.closing_transactions.sort_by(|a, b| {
            a.date
                .cmp(&b.date)
                .then_with(|| a.raw_statement.cmp(&b.raw_statement))
        });
        section
    }

    pub(crate) fn render(&self) -> Option<String> {
        if self.closing_transactions.is_empty() {
            return None;
        }

        let title = self.title();
        let mut entries = Vec::new();

        let mut account_declarations: Vec<&str> = self
            .account_declarations
            .iter()
            .map(String::as_str)
            .collect();
        account_declarations.sort();
        entries.extend(account_declarations.into_iter().map(String::from));
        entries.extend(
            self.closing_transactions
                .iter()
                .map(|record| record.raw_statement.clone()),
        );

        Some(format!(
            "{}\n\n{}",
            comment_header(&title),
            entries.join("\n")
        ))
    }
}

// Internal.
// ----------------------------------------------------------------------------

impl ClosingSection {
    fn ingest(&mut self, source: &str) {
        let mut current_record_date: Option<NaiveDate> = None;
        let mut current_record_lines: Vec<String> = Vec::new();

        for line in source.lines() {
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with(';') {
                continue;
            }
            if trimmed_start.starts_with("account ") {
                self.account_declarations
                    .insert(line.trim_end().to_string());
                continue;
            }
            if let Some(date) = parse_record_date(trimmed_start) {
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

        self.closing_transactions.push(ClosingTransaction {
            date,
            raw_statement: statement,
        });
    }

    fn title(&self) -> String {
        let start_year = self
            .closing_transactions
            .first()
            .map(|record| record.date.year())
            .unwrap_or_default();
        let end_year = self
            .closing_transactions
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

// Helpers.
// ----------------------------------------------------------------------------

fn parse_record_date(line: &str) -> Option<NaiveDate> {
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
