# fractic-ifrs-hledger

Generates IFRS-style hledger journals from Fractic's company transaction CSVs:
CSV datasources → domain `FinancialRecords` (accounts, transactions,
assertions, end-of-year entries, cash-flow tracing tags) → a `ProcessUsecase`
→ the hledger printer. Public, provided as-is (`fractic-io/rust-ifrs-hledger`,
identity `fractic`). The expected CSV format is documented only by the code.

## Read first

- `~/.agents/AGENTS.md` (workspace orientation; loaded automatically by Claude
  Code, read it if you are Codex or Grok) and `~/.agents/guides/rust.md`
  (§2 "library crates", §4 layering).

## Layout

Library with `src/impl/mod.rs` as the gatekeeper: `impl/domain/{entities,
logic,repositories,usecases}`, `impl/data/{datasources,models,repositories}`,
`impl/presentation/` (hledger formatting), `impl/exports` (entities and
`format_amount`), `impl_ext/` → `ext` (optional extensions), plus
`errors.rs`, `util.rs`. `res/cash_flow_statement_template.txt` is embedded.

## Working here

- No flake here: `nix develop /efs/fractic/storyengine/util --command cargo
  test`.
- Accounting rules live in `domain/logic/*_processor.rs`; input parsing in
  `data/`; output formatting in `presentation/`. Keep that separation when
  adding a rule, a CSV column or an output section.
- Commit / PR titles: imperative, trailing period.
