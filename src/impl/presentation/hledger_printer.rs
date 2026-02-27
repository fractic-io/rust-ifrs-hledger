use std::collections::{BTreeMap, BTreeSet, HashSet};

use iso_currency::Currency;

use crate::{
    entities::{
        Account, Assertion, CashflowTracingTag, CloseLogic, EndOfYearEntry, FinancialRecords,
        Transaction,
    },
    ext::standard_accounts::RETAINED_EARNINGS,
    presentation::utils::header_comment,
};

use super::utils::format_amount;

pub(crate) struct HledgerPrinter;

const POSTING_INDENT: &str = "    ";
const POSTING_TOTAL_WIDTH: usize = 100;
const POSTING_MIN_GAP: usize = 2;

impl HledgerPrinter {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn print_ledger(&self, financial_records: &FinancialRecords) -> String {
        let mut ledger_output = String::new();

        ledger_output.push_str(&header_comment("Accounts"));
        self.print_accounts(&mut ledger_output, financial_records);

        ledger_output.push_str("\n\n");
        ledger_output.push_str(&header_comment("Commodities"));
        self.print_commodities(&mut ledger_output, financial_records);

        ledger_output.push_str("\n\n");
        ledger_output.push_str(&header_comment("Payees"));
        self.print_payees(&mut ledger_output, financial_records);

        ledger_output.push_str("\n\n");
        ledger_output.push_str(&header_comment("Transactions"));
        self.print_transactions(&mut ledger_output, financial_records);

        ledger_output.push_str("\n\n");
        ledger_output.push_str(&header_comment("Assertions"));
        self.print_assertions(&mut ledger_output, financial_records);

        if !financial_records.ledger_extensions.is_empty() {
            ledger_output.push_str("\n\n");
            ledger_output.push_str(&header_comment("Custom Ledger Extensions"));
            self.print_ledger_extensions(&mut ledger_output, financial_records);
        }

        for (year, entries) in eoy_entries_by_year(&financial_records.eoy_entries) {
            ledger_output.push_str("\n\n");
            ledger_output.push_str(&header_comment(&format!("{} Corrections / Closing", year)));
            self.print_eoy_entries(&mut ledger_output, entries);
        }

        ledger_output
    }

    fn print_accounts(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let accounts: HashSet<&Account> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| &p.account))
            .chain(financial_records.assertions.iter().map(|a| &a.account))
            .collect();
        let sorted_account_declarations = {
            let mut v: Vec<String> = accounts
                .into_iter()
                .map(format_account_declaration)
                .collect();
            v.sort();
            v
        };
        for d in sorted_account_declarations {
            ledger_output.push_str(&d);
            ledger_output.push('\n');
        }
    }

    fn print_commodities(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        const SAMPLE_AMOUNT: f64 = 1000.0;
        let currencies: HashSet<Currency> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| p.currency))
            .collect();
        let sorted_commodity_declarations = {
            let mut v: Vec<String> = currencies
                .iter()
                .map(|c| format!("commodity {}", format_amount(SAMPLE_AMOUNT, *c, true)))
                .collect();
            v.sort();
            v
        };
        ledger_output.push_str("decimal-mark .\n");
        for d in sorted_commodity_declarations {
            ledger_output.push_str(&d);
            ledger_output.push('\n');
        }
    }

    fn print_payees(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let payees: HashSet<&String> = financial_records
            .label_lookup
            .values()
            .map(|label| &label.payee)
            .collect();
        let sorted_payee_declarations = {
            let mut v: Vec<String> = payees.iter().map(|p| format!("payee {}", p)).collect();
            v.sort();
            v
        };
        for p in sorted_payee_declarations {
            ledger_output.push_str(&p);
            ledger_output.push('\n');
        }
    }

    fn print_transactions(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let sorted_transactions = {
            let mut v: Vec<&Transaction> = financial_records.transactions.iter().collect();
            v.sort_by_key(|tx| tx.date);
            v
        };
        for tx in sorted_transactions {
            let label = financial_records.label_lookup.get(&tx.spec_id).map_or(
                "(unknown)".to_string(),
                |label| {
                    if let Some(comment) = &tx.comment {
                        format!("{} | {}: {}", label.payee, comment, label.description)
                    } else {
                        format!("{} | {}", label.payee, label.description)
                    }
                },
            );
            ledger_output.push_str(&format!("{} ({}) {}\n", tx.date, tx.spec_id, label));
            for posting in &tx.postings {
                let cashflow_tag = posting
                    .source_account
                    .as_ref()
                    .unwrap_or(&posting.account)
                    .cashflow_tag(posting.amount)
                    .map(|tag| format!("{}: {}", CashflowTracingTag::key(), tag.value()));
                let custom_tags = posting
                    .custom_tags
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<String>>();
                let tag_str = match cashflow_tag
                    .into_iter()
                    .chain(custom_tags.into_iter())
                    .collect::<Vec<String>>()
                {
                    tags if tags.is_empty() => "".to_string(),
                    tags => format!("       ; {}", tags.join(", ")),
                };
                let posting_line = format_posting_line(
                    &posting.account.ledger(),
                    &format_amount(posting.amount, posting.currency, false),
                );
                ledger_output.push_str(&format!("{}{}\n", posting_line, tag_str));
            }
            for annotation in financial_records
                .annotations_lookup
                .get(&tx.spec_id)
                .unwrap_or(&vec![])
            {
                format_note(&annotation.to_string())
                    .iter()
                    .for_each(|line| {
                        ledger_output.push_str(line);
                        ledger_output.push('\n');
                    });
            }
            ledger_output.push('\n');
        }
    }

    fn print_assertions(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let sorted_assertions = {
            let mut v: Vec<&Assertion> = financial_records.assertions.iter().collect();
            v.sort_by_key(|tx| tx.date);
            v
        };
        for assertion in sorted_assertions {
            ledger_output.push_str(&format!("{} <assertion>\n", assertion.date));
            let right = format!(
                "0 == {}",
                format_amount(assertion.balance, assertion.currency, false)
            );
            ledger_output.push_str(&format!(
                "{}\n",
                format_posting_line(&assertion.account.ledger(), &right)
            ));
            ledger_output.push('\n');
        }
    }

    fn print_ledger_extensions(
        &self,
        ledger_output: &mut String,
        financial_records: &FinancialRecords,
    ) {
        // Extract/dedup account declarations across all raw extension chunks,
        // and print once at the start of this section.
        let account_statements = financial_records
            .ledger_extensions
            .iter()
            .flat_map(|entry| extract_and_normalize_account_declarations(entry))
            .collect::<BTreeSet<_>>();
        for account_statement in account_statements.iter() {
            ledger_output.push_str(account_statement);
            ledger_output.push('\n');
        }
        if !account_statements.is_empty() {
            ledger_output.push('\n');
        }

        // Print each raw chunk verbatim except:
        // - account declarations are removed (already printed above),
        // - posting lines are spacing-normalized.
        for raw in financial_records.ledger_extensions.iter() {
            let normalized = extract_and_normalize_non_account_lines(raw);
            if normalized.is_empty() {
                continue;
            }
            ledger_output.push_str(&normalized);
            ledger_output.push_str("\n\n");
        }
    }

    fn print_eoy_entries(&self, ledger_output: &mut String, mut entries: Vec<&EndOfYearEntry>) {
        entries.sort_by_key(|e| *e.date());

        // Extract (and dedup) account declarations from each entry, to print
        // them once at the start of the section.
        let account_statements = entries
            .iter()
            .flat_map(|e| account_declarations_from_eoy_entry(e))
            .collect::<BTreeSet<_>>();
        for account_statement in account_statements.iter() {
            ledger_output.push_str(account_statement);
            ledger_output.push('\n');
        }
        if !account_statements.is_empty() {
            ledger_output.push('\n');
        }

        // Build ledger entries for each entry.
        for entry in entries.iter() {
            match entry {
                EndOfYearEntry::Close {
                    date,
                    postings,
                    total,
                    logic,
                    currency,
                } => {
                    let (title, destination_account): (&str, Account) = match logic {
                        CloseLogic::Retain => ("Retain Earnings", RETAINED_EARNINGS.clone().into()),
                    };

                    // Write title.
                    ledger_output.push_str(&format!(
                        "{} {}  ; close:{}\n",
                        date,
                        title,
                        entry.year()
                    ));

                    // Write debit entries.
                    for (account, amount) in postings {
                        let right = format!(
                            "{} = {}",
                            format_amount(*amount, *currency, true),
                            format_amount(0.0, *currency, true)
                        );
                        ledger_output
                            .push_str(&format!("{}\n", format_posting_line(account, &right)));
                    }

                    // Write credit entry.
                    if let Some(total) = total {
                        ledger_output.push_str(&format!(
                            "{}\n",
                            format_posting_line(
                                &destination_account.ledger(),
                                &format_amount(-*total, *currency, true),
                            )
                        ));
                    } else {
                        ledger_output.push_str(&format!(
                            "{}{}\n",
                            POSTING_INDENT,
                            destination_account.ledger()
                        ));
                    }
                }
                EndOfYearEntry::Correction {
                    macro_output,
                    notes,
                    ..
                } => {
                    let normalized = extract_and_normalize_non_account_lines(macro_output);
                    let tagged =
                        attach_transaction_tag(normalized, &format!("correction:{}", entry.year()));
                    let with_notes = attach_transaction_notes(tagged, notes);
                    ledger_output.push_str(&with_notes);
                    ledger_output.push('\n');
                }
            }

            ledger_output.push('\n');
        }
    }
}

// Building / manipulating account declarations. Ex:
// "account Assets:Cash     ; type: C"
// ----------------------------------------------------------------------------

fn format_account_declaration(account: &Account) -> String {
    format_account_declaration_raw(&account.ledger(), account.type_tag())
}

fn format_account_declaration_raw(ledger: &str, type_tag: char) -> String {
    format!("account {:81}  ; type: {}", ledger, type_tag)
}

fn is_account_declaration(line: &str) -> bool {
    line.trim_start().starts_with("account ")
}

fn parse_account_declaration(line: &str) -> Option<(&str, char)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("account")?.trim_start();
    let (name_part, type_part) = rest.split_once(';')?;
    let name = name_part.trim();

    let type_str = type_part
        .trim()
        .strip_prefix("type")?
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .trim();

    let mut ch_iter = type_str.chars();
    let ch = ch_iter.next()?;
    let is_single = ch_iter.next().is_none();

    Some((name, ch)).filter(|(n, _)| !n.is_empty() && is_single)
}

// Building / manipulating indented posting lines. Ex:
// "    Account Name      Amount"
// ----------------------------------------------------------------------------

fn format_posting_line(left: &str, right: &str) -> String {
    let content_width = POSTING_TOTAL_WIDTH.saturating_sub(char_width(POSTING_INDENT));
    let body = join_and_pad_between(left, right, content_width, POSTING_MIN_GAP);
    format!("{}{}", POSTING_INDENT, body)
}

fn parse_posting_line(line: &str) -> Option<(&str, &str)> {
    if is_account_declaration(line) {
        return None;
    }
    let body = line.strip_prefix(POSTING_INDENT)?;
    if body.trim_start().starts_with(';') {
        return None;
    }
    let split_at = body.match_indices("  ").last().map(|(idx, _)| idx)?;
    let left = body[..split_at].trim_end();
    let right = body[split_at..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left, right))
}

// Building / manipulating indented note lines. Ex:
// "    ; Note"
// ----------------------------------------------------------------------------

fn format_note(note: &str) -> Vec<String> {
    let wrapped = textwrap::wrap(note, 94);
    let prefix = "    ;";

    let mut lines = vec![prefix.to_string()];
    for line in wrapped {
        lines.push(format!("{} {}", prefix, line));
    }
    lines
}

// Helpers for manipulating existing ledger content.
// ----------------------------------------------------------------------------

/// Normalize the inner padding of *existing* account declarations, as detected
/// from the raw ledger content.
fn extract_and_normalize_account_declarations(ledger_content: &str) -> Vec<String> {
    ledger_content
        .lines()
        .filter(|line| is_account_declaration(line))
        .filter_map(parse_account_declaration)
        .map(|(name, tag)| format_account_declaration_raw(name, tag))
        .collect()
}

/// Normalize the inner padding of *existing* posting lines, as detected from
/// the raw ledger content.
fn extract_and_normalize_non_account_lines(ledger_content: &str) -> String {
    ledger_content
        .lines()
        .filter(|line| !is_account_declaration(line))
        .map(|line| {
            if let Some((left, right)) = parse_posting_line(line) {
                format_posting_line(left, right)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Attach a tag to all transaction entries detected in the ledger content.
fn attach_transaction_tag(ledger_content: String, tag: &str) -> String {
    let lines: Vec<&str> = ledger_content.lines().collect();
    lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            if is_transaction_header_line(&lines, idx) {
                if line.contains(&format!("; {}", tag)) {
                    (*line).to_string()
                } else {
                    format!("{}  ; {}", line, tag)
                }
            } else {
                (*line).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Attach notes to all transaction entries detected in the ledger content.
fn attach_transaction_notes(ledger_content: String, notes: &[String]) -> String {
    if notes.is_empty() {
        return ledger_content.to_string();
    }

    let lines: Vec<&str> = ledger_content.lines().collect();
    let mut output_lines = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if is_transaction_header_line(&lines, index) {
            output_lines.push(line.to_string());
            index += 1;

            while index < lines.len() && is_indented_line(lines[index]) {
                output_lines.push(lines[index].to_string());
                index += 1;
            }

            for note in notes {
                output_lines.extend(format_note(note));
            }
            continue;
        }

        output_lines.push(line.to_string());
        index += 1;
    }

    output_lines.join("\n")
}

// EOY-entry helpers.
// ----------------------------------------------------------------------------

fn eoy_entries_by_year(entries: &Vec<EndOfYearEntry>) -> BTreeMap<i32, Vec<&EndOfYearEntry>> {
    entries.iter().fold(
        BTreeMap::<i32, Vec<&EndOfYearEntry>>::new(),
        |mut acc: BTreeMap<i32, Vec<&EndOfYearEntry>>, entry: &EndOfYearEntry| {
            acc.entry(entry.year()).or_default().push(entry);
            acc
        },
    )
}

fn account_declarations_from_eoy_entry(entry: &EndOfYearEntry) -> Vec<String> {
    match entry {
        EndOfYearEntry::Close { logic, .. } => match logic {
            CloseLogic::Retain => {
                vec![format_account_declaration(
                    &(RETAINED_EARNINGS.clone().into()),
                )]
            }
        },
        EndOfYearEntry::Correction { macro_output, .. } => {
            extract_and_normalize_account_declarations(macro_output)
        }
    }
}

// Generic helpers.
// ----------------------------------------------------------------------------

fn join_and_pad_between(left: &str, right: &str, total_width: usize, min_gap: usize) -> String {
    let min_width = char_width(left) + char_width(right) + min_gap;
    let gap = if min_width >= total_width {
        min_gap
    } else {
        total_width - char_width(left) - char_width(right)
    };
    format!("{}{}{}", left, " ".repeat(gap), right)
}

fn char_width(s: &str) -> usize {
    s.chars().count()
}

fn is_indented_line(line: &str) -> bool {
    line.starts_with(POSTING_INDENT)
}

fn is_transaction_header_line(lines: &[&str], index: usize) -> bool {
    let starts_with_date = |line: &str| {
        let bytes = line.as_bytes();
        if bytes.len() < 10 {
            return false;
        }
        if !bytes[0..4].iter().all(u8::is_ascii_digit) {
            return false;
        }
        if bytes[4] != b'-' {
            return false;
        }
        if !bytes[5..7].iter().all(u8::is_ascii_digit) {
            return false;
        }
        if bytes[7] != b'-' {
            return false;
        }
        if !bytes[8..10].iter().all(u8::is_ascii_digit) {
            return false;
        }
        if bytes.len() == 10 {
            return true;
        }
        bytes[10].is_ascii_whitespace()
    };
    lines.len() > index + 1 && starts_with_date(lines[index]) && is_indented_line(lines[index + 1])
}
