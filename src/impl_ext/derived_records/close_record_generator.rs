use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use std::process::Command;

use fractic_server_error::{CriticalError, ServerError};

use crate::errors::{
    HledgerCloseInvalidResponse, HledgerCommandFailed, HledgerInvalidPath, NoAccountsToClose,
};

// Public interface.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CloseLogic {
    Retain,
}

#[derive(Debug, Clone)]
pub struct CloseRecordGenerator {
    ledger_path: PathBuf,
    year: i32,
    logic: CloseLogic,
}

impl CloseRecordGenerator {
    pub fn new<P: AsRef<Path>>(
        ledger_path: P,
        year: i32,
        logic: CloseLogic,
    ) -> Result<Self, ServerError> {
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
            logic,
        })
    }

    pub fn generate(&self) -> Result<CloseRecord, ServerError> {
        let output = self.run_hledger_close()?;
        let entries = parse_close_entries(&output)?;
        if entries.is_empty() {
            return Err(NoAccountsToClose::new(self.year));
        }

        let total_amount = entries.iter().map(|(_, amount)| *amount).sum::<f64>();
        Ok(CloseRecord {
            year: self.year,
            closing_date: format!("{}-12-31", self.year),
            entries,
            total_amount,
        })
    }
}

// Output format.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CloseRecord {
    pub year: i32,
    pub closing_date: String,
    pub entries: Vec<(String, f64)>,
    pub total_amount: f64,
}

impl Display for CloseRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = self
            .entries
            .iter()
            .map(|(account, amount)| format!(r#"["{}",{}]"#, escape_json_string(account), amount))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "[{}]", body)
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
            .arg("close");

        match self.logic {
            CloseLogic::Retain => {
                cmd.arg("--retain");
            }
        }

        cmd.arg(r#"--close-desc=Auto-Generated: Temporary Close Record"#)
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

fn output_context(line: &str) -> String {
    format!("invalid close output line: '{}'", line)
}

fn escape_json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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
    fn close_record_display_is_minified_json_vector() {
        let record = CloseRecord {
            year: 2024,
            closing_date: "2024-12-31".to_string(),
            entries: vec![
                ("Expenses:Operating:Sample".to_string(), -1200.0),
                ("Income:NonOperating:Other".to_string(), 300.0),
            ],
            total_amount: -900.0,
        };
        assert_eq!(
            record.to_string(),
            r#"[["Expenses:Operating:Sample",-1200],["Income:NonOperating:Other",300]]"#
        );
    }
}
