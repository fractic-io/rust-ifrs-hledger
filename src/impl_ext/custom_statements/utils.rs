use fractic_server_error::{CriticalError, ServerError};
use regex::Regex;
use std::{collections::HashMap, path::PathBuf, process::Command};

use crate::errors::{HledgerCommandFailed, HledgerInvalidResponse, UnreplacedPlaceholdersRemain};

pub(crate) fn replace_all_placeholders_in_string(
    content: String,
    placeholders: &HashMap<String, String>,
    error_if_unreplaced_placeholders_remain: bool,
) -> Result<String, ServerError> {
    // Use a regex to find placeholders of the form {{Key}}.
    let placeholder_pattern =
        Regex::new(r"\{\{(\w+)\}\}").expect("hardcoded regex should be valid");

    // Replace all placeholders with their corresponding values.
    let mut unknown_keys = Vec::new();
    let result = placeholder_pattern.replace_all(&content, |caps: &regex::Captures| {
        let key = &caps[1]; // The content inside {{ }}.
        if let Some(value) = placeholders.get(key) {
            value.clone()
        } else {
            unknown_keys.push(key.to_string());
            caps[0].to_string() // The full '{{Key}}' string.
        }
    });

    let replaced_content = result.into_owned();

    if error_if_unreplaced_placeholders_remain && !unknown_keys.is_empty() {
        return Err(UnreplacedPlaceholdersRemain::new(&unknown_keys));
    }

    Ok(replaced_content)
}

#[derive(Debug)]
pub(crate) enum Query {
    IncomeStatement,
    ChangeInAccount {
        account: String,
    },
    ChangeInAccountReverse {
        account: String,
    },
    ChangeByTag {
        key: &'static str,
        value: &'static str,
    },
    CumulativeBalance {
        account: String,
    },
}
impl Query {
    pub(crate) fn dbg(&self) -> String {
        format!("{:?}", self)
    }
}
#[derive(Debug)]
pub(crate) enum Return {
    /// Return the value in the last row, last column.
    Total,
    /// Search for the row where the first column matches the given string, and
    /// return the value in the last column.
    SearchRowOrZero(String),
}
impl Return {
    pub(crate) fn dbg(&self) -> String {
        format!("{:?}", self)
    }
}
pub(crate) fn hledger(
    ledger_path: &PathBuf,
    period: &str,
    query: Query,
    pivot: Option<&'static str>,
    fetch: Return,
) -> Result<f64, ServerError> {
    let mut cmd = Command::new("hledger");
    cmd.arg("-f").arg(ledger_path).arg("-p").arg(period);

    match &query {
        Query::IncomeStatement => {
            cmd.arg("incomestatement");
        }
        Query::ChangeInAccount { account } => {
            let account_query = format!("^{}($|:)", account);
            cmd.arg("balance").arg(account_query);
        }
        Query::ChangeInAccountReverse { account } => {
            let account_query = format!("^{}($|:)", account);
            cmd.arg("balance").arg(account_query).arg("-r");
        }
        Query::ChangeByTag { key, value } => {
            let tag_query = format!("tag:{}={}", key, value);
            cmd.arg("balance").arg(tag_query);
        }
        Query::CumulativeBalance { account } => {
            let account_query = format!("^{}($|:)", account);
            cmd.arg("balance").arg(account_query).arg("-H");
        }
    }

    if let Some(pivot) = pivot {
        cmd.arg("--pivot").arg(pivot);
    }

    cmd.arg("--output-format=csv").arg("--layout=bare");

    let output = cmd.output().map_err(|e| {
        HledgerCommandFailed::with_debug(&ledger_path.display().to_string(), &cmd, &e)
    })?;
    if !output.status.success() {
        return Err(HledgerCommandFailed::with_debug(
            &ledger_path.display().to_string(),
            &cmd,
            &output,
        ));
    }
    let out_csv = String::from_utf8(output.stdout)
        .map_err(|e| CriticalError::with_debug("failed to parse hledger output as UTF-8", &e))?;

    let amount = match &fetch {
        Return::Total => {
            let amount_str = csv::Reader::from_reader(out_csv.as_bytes())
                .records()
                .last()
                .and_then(|r| r.ok())
                .and_then(|r| r.iter().last().map(|s| s.to_owned()))
                .ok_or_else(|| {
                    HledgerInvalidResponse::with_debug(&cmd, query.dbg(), fetch.dbg(), &out_csv)
                })?;
            amount_str.parse::<f64>().map_err(|e| {
                HledgerInvalidResponse::with_debug(&cmd, query.dbg(), fetch.dbg(), &e)
            })?
        }
        Return::SearchRowOrZero(search) => {
            let amount_str = csv::Reader::from_reader(out_csv.as_bytes())
                .records()
                .find(|r| r.as_ref().map_or(false, |r| r[0] == *search))
                .and_then(|r| r.ok())
                .and_then(|r| r.iter().last().map(|s| s.to_owned()))
                .unwrap_or("0".to_string());
            amount_str.parse::<f64>().map_err(|e| {
                HledgerInvalidResponse::with_debug(&cmd, query.dbg(), fetch.dbg(), &e)
            })?
        }
    };
    Ok(amount)
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum RegisterQuery {
    Account(String),
    Tag {
        key: &'static str,
        value: &'static str,
    },
    TagReverse {
        key: &'static str,
        value: &'static str,
    },
}
#[derive(Debug)]
pub(crate) enum RegisterOutput {
    Raw { width: i64 },
}
pub(crate) fn hledger_register(
    ledger_path: &PathBuf,
    period: &str,
    query: RegisterQuery,
    pivot: Option<&'static str>,
    format: RegisterOutput,
) -> Result<Vec<String>, ServerError> {
    let mut cmd = Command::new("hledger");
    cmd.arg("-f")
        .arg(ledger_path)
        .arg("-p")
        .arg(period)
        .arg("register");

    match &query {
        RegisterQuery::Account(account) => {
            let account_query = format!("^{}($|:)", account);
            cmd.arg(account_query);
        }
        RegisterQuery::Tag { key, value } => {
            let tag_query = format!("tag:{}={}", key, value);
            cmd.arg(tag_query);
        }
        RegisterQuery::TagReverse { key, value } => {
            let tag_query = format!("tag:{}={}", key, value);
            cmd.arg(tag_query).arg("-r");
        }
    }

    if let Some(pivot) = pivot {
        cmd.arg("--pivot").arg(pivot);
    }

    match &format {
        RegisterOutput::Raw { width } => {
            cmd.arg("-w").arg(width.to_string());

            let output = cmd.output().map_err(|e| {
                HledgerCommandFailed::with_debug(&ledger_path.display().to_string(), &cmd, &e)
            })?;
            if !output.status.success() {
                return Err(HledgerCommandFailed::with_debug(
                    &ledger_path.display().to_string(),
                    &cmd,
                    &output,
                ));
            }

            let out_raw = String::from_utf8(output.stdout).map_err(|e| {
                CriticalError::with_debug("failed to parse hledger output as UTF-8", &e)
            })?;

            Ok(out_raw.lines().map(|s| s.to_string()).collect())
        }
    }
}

pub(crate) fn split_sections(s: &str) -> Vec<&str> {
    let re = Regex::new(r"\s{4,}").unwrap();
    re.split(s).filter(|part| !part.trim().is_empty()).collect()
}
