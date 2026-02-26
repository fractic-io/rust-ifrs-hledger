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
pub struct CloseEntryGenerator {
    ledger_path: PathBuf,
    year: i32,
}

impl CloseEntryGenerator {
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

    pub fn generate(&self) -> Result<CloseEntry, ServerError> {
        let output = self.run_hledger_close()?;
        let postings = parse_close_postings(&output)?;
        if postings.is_empty() {
            return Err(NoAccountsToClose::new(self.year));
        }

        let total_amount = postings.iter().map(|(_, amount)| *amount).sum::<f64>();
        let postings_json = serde_json::to_string(&postings).map_err(|e| {
            CriticalError::with_debug("failed to serialize close postings as JSON", &e)
        })?;
        let tag = STANDARD.encode(postings_json);
        let closing_date = format!("{}-12-31", self.year);
        let clipboard = format_clipboard_row(&closing_date, &tag, total_amount);
        Ok(CloseEntry {
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
pub struct CloseEntry {
    pub year: i32,
    pub closing_date: String,
    pub tag: String,
    pub total_amount: f64,

    /// Pre-generated row that, if copied to clipboard, can be directly pasted
    /// into Google Sheets / Excel.
    pub clipboard: String,
}

impl Display for CloseEntry {
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
             'Close(<CLOSE_LOGIC>)' entry by setting the first column to ':' (to indicate a \
             command), the `Accounting Logic` column to the desired close logic (for example \
             'Close(Retain)'), the `Decorators` column to the `Tag` value, and the `Amount` \
             column to the `Total Amount` value.",
        )
    }
}

// Private.
// ----------------------------------------------------------------------------

impl CloseEntryGenerator {
    fn run_hledger_close(&self) -> Result<String, ServerError> {
        let mut cmd = Command::new("hledger");
        cmd.arg("--strict")
            .arg("-f")
            .arg(&self.ledger_path)
            .arg("-p")
            .arg(self.year.to_string())
            .arg("close")
            .arg("--retain")
            .arg(r#"--close-desc=Auto-Generated: Temporary Close Entry"#)
            .arg("--close-acct=VoidOut");

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

fn parse_close_postings(output: &str) -> Result<Vec<(String, f64)>, ServerError> {
    output.lines().try_fold(Vec::new(), |mut acc, line| {
        if let Some(posting) = parse_close_posting(line)? {
            acc.push(posting);
        }
        Ok(acc)
    })
}

fn parse_close_posting(raw_line: &str) -> Result<Option<(String, f64)>, ServerError> {
    // We are looking for lines like:
    //     Expenses:NonOperating:Other:Incidental       -10,000. ₩ = 0. ₩

    if !raw_line.starts_with("    ") {
        // Header line: "YYYY-12-31 ... ; retain:"
        return Ok(None);
    }

    let line = raw_line.trim();

    if line.is_empty() || line == "VoidOut" {
        return Ok(None);
    }
    if !line.contains(" = ") {
        return Err(HledgerCloseInvalidResponse::new(output_context(raw_line)));
    }

    let (debit_expr, _assertion_expr) = line
        .split_once(" = ")
        .ok_or_else(|| HledgerCloseInvalidResponse::new(output_context(raw_line)))?;
    let (account, amount_expr) = debit_expr
        .split_once(char::is_whitespace)
        .ok_or_else(|| HledgerCloseInvalidResponse::new(output_context(raw_line)))?;
    let amount = parse_amount_expr(amount_expr)?;
    Ok(Some((account.to_string(), amount)))
}

/// Parse a formatted number like:
/// - `-4,000.`
/// - `9,993.`
/// - `-535`
/// into a floating point number.
fn parse_formatted_number(value: &str) -> Result<f64, ServerError> {
    let normalized = value.replace(",", "");
    let without_trailing_dot = normalized.strip_suffix('.').unwrap_or(&normalized);
    without_trailing_dot.parse::<f64>().map_err(|e| {
        HledgerCloseInvalidResponse::with_debug(format!("invalid close amount '{}'", value), &e)
    })
}

/// Parse an amount expression like:
/// - `-4,000. ₩`
/// - `USD -4,000.45`
/// - `$4,000.45`
/// into a floating point number.
fn parse_amount_expr(expr: &str) -> Result<f64, ServerError> {
    for token in expr.split_whitespace() {
        if let Ok(amount) = parse_formatted_number(token) {
            return Ok(amount);
        }
        let cleaned = token.trim_matches(|c: char| {
            !c.is_ascii_digit() && c != '-' && c != '+' && c != ',' && c != '.'
        });
        if !cleaned.is_empty() {
            if let Ok(amount) = parse_formatted_number(cleaned) {
                return Ok(amount);
            }
        }
    }
    Err(HledgerCloseInvalidResponse::new(format!(
        "could not parse amount from close posting expression '{}'",
        expr
    )))
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
        assert_eq!(parse_formatted_number("-4,000.").unwrap(), -4000.0);
        assert_eq!(parse_formatted_number("9,993.").unwrap(), 9993.0);
        assert_eq!(parse_formatted_number("-535").unwrap(), -535.0);
    }

    #[test]
    fn parse_close_posting_accepts_amount_before_or_after_commodity() {
        assert_eq!(
            parse_close_posting("    Expenses:Ops -4,000. USD = 0").unwrap(),
            Some(("Expenses:Ops".to_string(), -4000.0))
        );
        assert_eq!(
            parse_close_posting("    Expenses:Ops USD -4,000. = 0").unwrap(),
            Some(("Expenses:Ops".to_string(), -4000.0))
        );
    }

    #[test]
    fn parse_amount_expr_handles_currency_symbols_and_explicit_plus() {
        assert_eq!(parse_amount_expr("USD +4,000.45").unwrap(), 4000.45);
        assert_eq!(parse_amount_expr("($4,000.45)").unwrap(), 4000.45);
        assert_eq!(parse_amount_expr("JPY -535").unwrap(), -535.0);
    }

    #[test]
    fn parse_amount_expr_uses_first_parseable_numeric_token() {
        assert_eq!(parse_amount_expr("KRW 1,200. note -90").unwrap(), 1200.0);
        assert_eq!(parse_amount_expr("abc -12.5 xyz").unwrap(), -12.5);
    }

    #[test]
    fn parse_amount_expr_rejects_empty_or_missing_numeric_tokens() {
        assert!(parse_amount_expr("").is_err());
        assert!(parse_amount_expr("USD KRW").is_err());
        assert!(parse_amount_expr("...").is_err());
    }

    #[test]
    fn parse_amount_expr_rejects_malformed_number_like_double_dot() {
        assert!(parse_amount_expr("USD 1,234..56").is_err());
    }

    #[test]
    fn close_entry_display_is_multiline_with_base64_tag() {
        let entry = CloseEntry {
            year: 2024,
            closing_date: "2024-12-31".to_string(),
            tag: "W1siRXhwZW5zZXM6T3BlcmF0aW5nOlNhbXBsZSIsLTEyMDAuMF0sWyJJbmNvbWU6Tm9uT3BlcmF0aW5nOk90aGVyIiwzMDAuMF1d"
                .to_string(),
            total_amount: -900.0,
            clipboard: ":\t\t2024-12-31\tClose(<CloseLogic>)\ttag123\t\t\t-900".to_string(),
        };
        let display = entry.to_string();
        assert!(display.contains("\u{1b}[1mTag:\u{1b}[0m"));
        assert!(display.contains(&entry.tag));
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
