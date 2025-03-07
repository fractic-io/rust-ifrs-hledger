// ---------------------------------------------------------------------
// Main processing function.
// Reads CSV content and returns (ledger_output, list of notes).
// ---------------------------------------------------------------------

impl<A, I, E, C, R> CsvRecord<A, I, E, C, R>
where
    A: AssetHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    I: IncomeHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    E: ExpenseHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: CashHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    R: ReimbursableEntityHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    pub fn parse(
        accrual_date: &str,
        until: Option<&str>,
        payment_date: &str,
        accounting_logic: &str,
        decorators: &str,
        entity: impl Into<String>,
        description: impl Into<String>,
        amount: &str,
        commodity: impl Into<String>,
        backing_account: &str,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            accrual_date: ISODate::from_str(accrual_date)?,
            until: until.map(ISODate::from_str).transpose()?,
            payment_date: ISODate::from_str(payment_date)?,
            accounting_logic: from_str(accounting_logic)
                .map_err(|e| InvalidRon::with_debug("AccountingLogic", &e))?,
            decorators: from_str(&format!("[{}]", decorators))
                .map_err(|e| InvalidRon::with_debug("Decorator", &e))?,
            entity: entity.into(),
            description: description.into(),
            amount: AccountingAmount::from_str(amount)?,
            commodity: commodity.into(),
            backing_account: from_str(backing_account)
                .map_err(|e| InvalidRon::with_debug("BackingAccount", &e))?,
        })
    }
}

pub struct FinancialRecords {
    pub ledger: String,
    pub notes: Vec<String>,
}

pub fn process_csv<A, I, E, C, R>(csv_content: &str) -> Result<FinancialRecords, ServerError>
where
    A: AssetHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    I: IncomeHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    E: ExpenseHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: CashHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    R: ReimbursableEntityHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let mut ledger_transactions: Vec<LedgerTransaction> = Vec::new();
    let mut global_notes: Vec<String> = Vec::new();
    // For VariableExpense historical data.
    let mut var_exp_history: HashMap<ExpenseAccount, Vec<(NaiveDate, f64)>> = HashMap::new();

    let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
    for result in rdr.records() {
        let record = result.map_err(|e| InvalidCsv::with_debug(&e))?;
        // Map CSV columns (order as specified):
        let csv_record = CsvRecord::<A, I, E, C, R>::parse(
            record.get(0).unwrap_or(""),
            match record.get(1) {
                Some(s) if !s.is_empty() => Some(s),
                _ => None,
            },
            record.get(2).unwrap_or(""),
            record.get(3).unwrap_or(""),
            record.get(4).unwrap_or(""),
            record.get(5).unwrap_or(""),
            record.get(6).unwrap_or(""),
            record.get(7).unwrap_or("0"),
            record.get(8).unwrap_or(""),
            record.get(9).unwrap_or(""),
        )?;

        let mut txs: Vec<LedgerTransaction> = Vec::new();
        // Process based on accounting logic.
        match csv_record.accounting_logic {
            AccountingLogic::SimpleExpense(expense) => {
                if csv_record.until.is_some() {
                    return Err(AccrualRangeNotSupported::new(
                        "SimpleExpense",
                        &csv_record.description,
                    ));
                }
                let transactions = process_simple_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Capitalize(asset) => {
                if csv_record.until.is_some() {
                    return Err(AccrualRangeNotSupported::new(
                        "Capitalize",
                        &csv_record.description,
                    ));
                }
                let transactions = process_capitalize(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    asset,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Amortize(asset) => {
                let transactions = process_amortize(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("Amortize", &csv_record.description)
                        })?
                        .0,
                    asset,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::FixedExpense(expense) => {
                let transactions = process_fixed_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("FixedExpense", &csv_record.description)
                        })?
                        .0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpense(expense) => {
                let transactions = process_variable_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("VariableExpense", &csv_record.description)
                        })?
                        .0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpenseInit { account, estimate } => {
                let transactions = process_variable_expense_init(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new(
                                "VariableExpenseInit",
                                &csv_record.description,
                            )
                        })?
                        .0,
                    account,
                    estimate,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ImmaterialIncome(income) => {
                let transactions = process_immaterial_income(
                    csv_record.payment_date.0,
                    income,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial income applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::ImmaterialExpense(expense) => {
                let transactions = process_immaterial_expense(
                    csv_record.payment_date.0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial expense applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::Reimburse(reimbursable) => {
                let transactions = process_reimburse(
                    csv_record.payment_date.0,
                    reimbursable,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ClearVat { from, to } => {
                let transactions = process_clear_vat(
                    csv_record.accrual_date.0,
                    match csv_record.backing_account {
                        BackingAccount::Cash(b) => Ok(b.account()),
                        _ => Err(ClearVatInvalidBackingAccount::with_debug(
                            &csv_record.description,
                            &csv_record.backing_account,
                        )),
                    }?,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    from.0,
                    to.0,
                )?;
                txs.extend(transactions);
            }
        }
        // Process decorators.
        let side_txs = apply_decorators(
            &mut txs,
            &csv_record.decorators,
            csv_record.accrual_date.0,
            csv_record.payment_date.0,
            &csv_record.commodity,
            &csv_record.description,
        )?;
        txs.extend(side_txs);
        // Append all transactions for this CSV record.
        ledger_transactions.extend(txs);
    }

    // Format ledger transactions into hledger format.
    let mut ledger_output = String::new();
    for tx in ledger_transactions {
        ledger_output.push_str(&format!("{} {}\n", tx.date, tx.description));
        for posting in tx.postings {
            ledger_output.push_str(&format!(
                "    {:30} {:10.2} {}\n",
                posting.account.ledger(),
                posting.amount,
                posting.commodity
            ));
        }
        for note in tx.notes {
            ledger_output.push_str(&format!("    ; {}\n", note));
        }
        ledger_output.push('\n');
    }
    Ok(FinancialRecords {
        ledger: ledger_output,
        notes: global_notes,
    })
}
