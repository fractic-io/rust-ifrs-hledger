# fractic-ifrs-hledger

Generates IFRS-style hledger journals from company transaction CSVs: CSV
datasources → domain `FinancialRecords` (accounts, transactions, assertions,
end-of-year entries, cash-flow tracing tags) → a `ProcessUsecase` → the
hledger printer. The expected CSV format is documented only by the code.
Library layout per `/efs/fractic/_/rust.md` §2: `src/impl/mod.rs` gates
`impl/domain/{entities,logic,repositories,usecases}`,
`impl/data/{datasources,models,repositories}`, `impl/presentation/` (hledger
formatting), `impl/exports` (entities and `format_amount`), `impl_ext/` →
`ext` (optional extensions), plus `errors.rs`, `util.rs`.
`res/cash_flow_statement_template.txt` is embedded.

- Accounting rules live in `domain/logic/*_processor.rs`, input parsing in
  `data/`, output formatting in `presentation/`; keep that separation for a
  new rule, CSV column or output section.
- Commit / PR titles: imperative, trailing period.
