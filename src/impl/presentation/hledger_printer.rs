use std::collections::{BTreeMap, BTreeSet, HashSet};

use iso_currency::Currency;

use crate::{
    entities::{
        Account, Assertion, CashflowTracingTag, CloseLogic, FinancialRecords, Operation,
        Transaction,
    },
    ext::standard_accounts::RETAINED_EARNINGS,
    presentation::utils::header_comment,
};

use super::utils::format_amount;

pub(crate) struct HledgerPrinter;

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

        for (year, operations) in operations_by_year(&financial_records.operations) {
            ledger_output.push_str("\n\n");
            ledger_output.push_str(&header_comment(&format!("{} Operations", year)));
            self.print_operations(&mut ledger_output, operations);
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
            let mut v: Vec<String> = accounts.into_iter().map(account_declaration).collect();
            v.sort();
            v
        };
        for d in sorted_account_declarations {
            ledger_output.push_str(&d);
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
                .map(|c| format!("commodity {}\n", format_amount(SAMPLE_AMOUNT, *c, true)))
                .collect();
            v.sort();
            v
        };
        ledger_output.push_str("decimal-mark .\n");
        for d in sorted_commodity_declarations {
            ledger_output.push_str(&d);
        }
    }

    fn print_payees(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let payees: HashSet<&String> = financial_records
            .label_lookup
            .values()
            .map(|label| &label.payee)
            .collect();
        let sorted_payee_declarations = {
            let mut v: Vec<String> = payees.iter().map(|p| format!("payee {}\n", p)).collect();
            v.sort();
            v
        };
        for p in sorted_payee_declarations {
            ledger_output.push_str(&p);
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
                ledger_output.push_str(&format!(
                    "    {:75} {:>20}{}\n",
                    posting.account.ledger(),
                    format_amount(posting.amount, posting.currency, false),
                    tag_str
                ));
            }
            for annotation in financial_records
                .annotations_lookup
                .get(&tx.spec_id)
                .unwrap_or(&vec![])
            {
                let s = annotation.to_string();
                let wrapped = textwrap::wrap(&s, 94);
                let prefix = "    ;";
                ledger_output.push_str(&format!("{}\n", prefix));
                for line in wrapped {
                    ledger_output.push_str(&format!("{} {}\n", prefix, line));
                }
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
            ledger_output.push_str(&format!(
                "    {:70} 0 == {:>20}\n",
                assertion.account.ledger(),
                format_amount(assertion.balance, assertion.currency, false),
            ));
            ledger_output.push('\n');
        }
    }

    fn print_operations(&self, ledger_output: &mut String, mut operations: Vec<&Operation>) {
        operations.sort_by_key(|operation| *operation.date());

        // Extract (and dedup) account declarations from each operation, to
        // print them once at the start of the section.
        let account_statements = operations
            .iter()
            .flat_map(|operation| accounts_used_by(operation))
            .collect::<BTreeSet<_>>();
        for account_statement in account_statements {
            ledger_output.push_str(&format!("{}\n", account_statement));
        }
        if !operations.is_empty() {
            ledger_output.push('\n');
        }

        // Build ledger entries for each operation.
        for (i, operation) in operations.iter().enumerate() {
            match operation {
                Operation::Close {
                    date,
                    entries,
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
                        date.format("%Y")
                    ));

                    // Write debit entries.
                    for (account, amount) in entries {
                        let right = format!(
                            "{} = {}",
                            format_amount(*amount, *currency, true),
                            format_amount(0.0, *currency, true)
                        );
                        ledger_output.push_str(&format!(
                            "{}\n",
                            format_posting_line_width_100(account, &right)
                        ));
                    }

                    // Write credit entry.
                    if let Some(total) = total {
                        ledger_output.push_str(&format!(
                            "{}\n",
                            format_posting_line_width_100(
                                &destination_account.ledger(),
                                &format_amount(*total, *currency, true),
                            )
                        ));
                    } else {
                        ledger_output.push_str(&format!("    {}\n", destination_account.ledger()));
                    }
                }
                Operation::Correction { result, .. } => {
                    let normalized = normalize_formatting(result)
                        .join("\n")
                        .trim_end()
                        .to_string();
                    ledger_output.push_str(&normalized);
                }
            }

            if i + 1 < operations.len() {
                ledger_output.push('\n');
            }
        }
        ledger_output.push('\n');
    }
}

// Helpers.
// ----------------------------------------------------------------------------

fn account_declaration(account: &Account) -> String {
    format!(
        "account {:81}  ; type: {}\n",
        account.ledger(),
        account.type_tag()
    )
}

fn operations_by_year(operations: &Vec<Operation>) -> BTreeMap<i32, Vec<&Operation>> {
    operations.iter().fold(
        BTreeMap::<i32, Vec<&Operation>>::new(),
        |mut acc: BTreeMap<i32, Vec<&Operation>>, operation: &Operation| {
            acc.entry(operation.year()).or_default().push(operation);
            acc
        },
    )
}

fn accounts_used_by(operation: &Operation) -> Vec<String> {
    match operation {
        Operation::Close { logic, .. } => match logic {
            CloseLogic::Retain => {
                vec![account_declaration(&(RETAINED_EARNINGS.clone().into()))]
            }
        },
        Operation::Correction { result, .. } => result
            .lines()
            .filter(|line| line.trim_start().starts_with("account "))
            .map(|line| line.to_string())
            .collect(),
    }
}

fn normalize_formatting(entry: &str) -> Vec<String> {
    entry
        .lines()
        .filter(|line| !line.trim_start().starts_with("account "))
        .map(|line| {
            if is_resizable_posting_line(line) {
                resize_posting_line_to_width(line, 100)
            } else {
                line.to_string()
            }
        })
        .collect()
}

fn is_resizable_posting_line(line: &str) -> bool {
    line.starts_with("    ")
        && !line.trim_start().starts_with(';')
        && split_posting_line(line).is_some()
}

fn resize_posting_line_to_width(line: &str, width: usize) -> String {
    if char_width(line) >= width {
        return line.to_string();
    }
    let Some((left, right)) = split_posting_line(line) else {
        return line.to_string();
    };
    format_posting_line_with_width(left, right, width)
}

fn format_posting_line_width_100(left: &str, right: &str) -> String {
    format_posting_line_with_width(left, right, 100)
}

fn format_posting_line_with_width(left: &str, right: &str, width: usize) -> String {
    const INDENT: &str = "    ";
    let content_width = width.saturating_sub(char_width(INDENT));
    let body = pad_between(left, right, content_width, 2);
    format!("{}{}", INDENT, body)
}

fn split_posting_line(line: &str) -> Option<(&str, &str)> {
    let body = line.strip_prefix("    ")?;
    let split_at = body.match_indices("  ").last().map(|(idx, _)| idx)?;
    let left = body[..split_at].trim_end();
    let right = body[split_at..].trim();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some((left, right))
}

fn pad_between(left: &str, right: &str, total_width: usize, min_gap: usize) -> String {
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
