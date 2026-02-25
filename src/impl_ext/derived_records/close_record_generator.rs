use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use fractic_server_error::{CriticalError, ServerError};

use crate::errors::{
    HledgerCloseInvalidResponse, HledgerCommandFailed, HledgerInvalidPath, NoAccountsToClose,
};

// Public interface.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CloseRecordGenerator {
    ledger_path: PathBuf,
    year: i32,
}

impl CloseRecordGenerator {
    pub fn new<P: AsRef<Path>>(ledger_path: P, year: i32) -> Result<Self, ServerError> {
        Ok(Self {
            ledger_path: ledger_path
                .as_ref()
                .canonicalize()
                .map_err(|e| {
                    HledgerInvalidPath::with_debug(
                        &ledger_path.as_ref().to_string_lossy().to_string(),
                        &e,
                    )
                })?
                .to_path_buf(),
            year,
        })
    }

    pub fn generate(&self) -> Result<CloseRecord, ServerError> {
        let output = self.run_hledger_close()?;
        let entries = parse_close_entries(&output)?;
        if entries.is_empty() {
            return Err(NoAccountsToClose::new(self.year));
        }

        let total_amount = entries.iter().map(|(_, amount)| *amount).sum::<f64>();
        let entries_json = serde_json::to_string(&entries).map_err(|e| {
            CriticalError::with_debug("failed to serialize close entries as JSON", &e)
        })?;
        let tag = STANDARD.encode(entries_json);
        let closing_date = format!("{}-12-31", self.year);
        let clipboard = format_clipboard_row(&closing_date, &tag, total_amount);
        Ok(CloseRecord {
            year: self.year,
            closing_date,
            tag,
            total_amount,
            clipboard,
        })
    }
}

// Output format.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CloseRecord {
    pub year: i32,
    pub closing_date: String,
    pub tag: String,
    pub total_amount: f64,

    /// Pre-generated row that, if copied to clipboard, can be directly pasted
    /// into Google Sheets / Excel.
    pub clipboard: String,
}

impl Display for CloseRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BOLD: &str = "\x1b[1m";
        const RESET: &str = "\x1b[0m";
        write!(
            f,
            "{BOLD}Year:{RESET} {}\n{BOLD}Closing Date:{RESET} {}\n{BOLD}Total Amount:{RESET} {}",
            self.year, self.closing_date, self.total_amount
        )?;
        write!(f, "\n\n{BOLD}Tag:{RESET} {}", self.tag)?;
        write!(
            f,
            "\n\n{BOLD}Instructions:{RESET} In the transactions CSV, create a \
             'Close(<CLOSE_LOGIC>)' record by setting the first column to ':' (to indicate an \
             operation record), the `Accounting Logic` column to the desired close logic (for \
             example 'Close(Retain)'), the `Decorators` column to the `Tag` value, and the \
             `Amount` column to the `Total Amount` value.",
        )
    }
}

// Private.
// ----------------------------------------------------------------------------

impl CloseRecordGenerator {
    fn run_hledger_close(&self) -> Result<String, ServerError> {
        let mut cmd = Command::new("hledger");
        cmd.arg("--strict")
            .arg("-f")
            .arg(&self.ledger_path)
            .arg("-p")
            .arg(self.year.to_string())
            .arg("close")
            .arg("--retain")
            .arg(r#"--close-desc=Auto-Generated: Temporary Close Record"#)
            .arg("--close-acct=VoidOut")
            .arg("not:tag:close");

        let output = cmd.output().map_err(|e| {
            HledgerCommandFailed::with_debug(&self.ledger_path.display().to_string(), &cmd, &e)
        })?;
        if !output.status.success() {
            return Err(HledgerCommandFailed::with_debug(
                &self.ledger_path.display().to_string(),
                &cmd,
                &output,
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|e| CriticalError::with_debug("failed to parse hledger output as UTF-8", &e))
    }
}

// Helpers.
// ----------------------------------------------------------------------------

fn parse_close_entries(output: &str) -> Result<Vec<(String, f64)>, ServerError> {
    output.lines().try_fold(Vec::new(), |mut acc, line| {
        if let Some(entry) = parse_close_entry(line)? {
            acc.push(entry);
        }
        Ok(acc)
    })
}

fn parse_close_entry(raw_line: &str) -> Result<Option<(String, f64)>, ServerError> {
    let line = raw_line.trim();
    if line.is_empty() || line == "VoidOut" {
        return Ok(None);
    }
    if !raw_line.starts_with("    ") {
        // Header line: "YYYY-12-31 ... ; retain:"
        return Ok(None);
    }
    if !line.contains(" = ") {
        return Err(HledgerCloseInvalidResponse::new(output_context(raw_line)));
    }

    let left = line
        .split(" = ")
        .next()
        .ok_or_else(|| HledgerCloseInvalidResponse::new(output_context(raw_line)))?;
    let mut sections = left.split_whitespace();
    let account = sections
        .next()
        .ok_or_else(|| HledgerCloseInvalidResponse::new(output_context(raw_line)))?;
    let amount_raw = sections
        .next()
        .ok_or_else(|| HledgerCloseInvalidResponse::new(output_context(raw_line)))?;

    let amount = parse_hledger_amount(amount_raw)?;
    Ok(Some((account.to_string(), amount)))
}

fn parse_hledger_amount(value: &str) -> Result<f64, ServerError> {
    let normalized = value.replace(",", "");
    let without_trailing_dot = normalized.strip_suffix('.').unwrap_or(&normalized);
    without_trailing_dot.parse::<f64>().map_err(|e| {
        HledgerCloseInvalidResponse::with_debug(format!("invalid close amount '{}'", value), &e)
    })
}

fn format_clipboard_row(closing_date: &str, tag: &str, total_amount: f64) -> String {
    // Google Sheets expands tab-separated values across columns on paste.
    format!(
        ":\t\t{}\tClose(<CloseLogic>)\t{}\t\t\t{}",
        closing_date, tag, total_amount
    )
}

fn output_context(line: &str) -> String {
    format!("invalid close output line: '{}'", line)
}

// Tests.
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hledger_amount_handles_commas_and_dot_suffix() {
        assert_eq!(parse_hledger_amount("-4,000.").unwrap(), -4000.0);
        assert_eq!(parse_hledger_amount("9,993.").unwrap(), 9993.0);
        assert_eq!(parse_hledger_amount("-535").unwrap(), -535.0);
    }

    #[test]
    fn close_record_display_is_multiline_with_base64_tag() {
        let record = CloseRecord {
            year: 2024,
            closing_date: "2024-12-31".to_string(),
            tag: "W1siRXhwZW5zZXM6T3BlcmF0aW5nOlNhbXBsZSIsLTEyMDAuMF0sWyJJbmNvbWU6Tm9uT3BlcmF0aW5nOk90aGVyIiwzMDAuMF1d"
                .to_string(),
            total_amount: -900.0,
            clipboard: ":\t\t2024-12-31\tClose(<CloseLogic>)\ttag123\t\t\t-900".to_string(),
        };
        let display = record.to_string();
        assert!(display.contains("\u{1b}[1mTag:\u{1b}[0m"));
        assert!(display.contains(&record.tag));
    }

    #[test]
    fn format_clipboard_row_is_tab_separated_for_google_sheets() {
        let row = format_clipboard_row("2024-12-31", "abc123", -900.0);
        assert_eq!(
            row,
            ":\t\t2024-12-31\tClose(<CloseLogic>)\tabc123\t\t\t-900"
        );
    }
}
