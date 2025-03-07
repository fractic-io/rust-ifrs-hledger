use crate::entities::AccountingLogic;

use super::iso_date_model::ISODateModel;

#[derive(Debug, serde_derive::Deserialize)]
pub enum AccountingLogicModel<E, A, I, R> {
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit {
        account: E,
        estimate: i64,
    },
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ClearVat {
        from: ISODateModel,
        to: ISODateModel,
    },
}

impl<E, A, I, R> Into<AccountingLogic<E, A, I, R>> for AccountingLogicModel<E, A, I, R> {
    fn into(self) -> AccountingLogic<E, A, I, R> {
        match self {
            AccountingLogicModel::SimpleExpense(e) => AccountingLogic::SimpleExpense(e),
            AccountingLogicModel::Capitalize(a) => AccountingLogic::Capitalize(a),
            AccountingLogicModel::Amortize(a) => AccountingLogic::Amortize(a),
            AccountingLogicModel::FixedExpense(e) => AccountingLogic::FixedExpense(e),
            AccountingLogicModel::VariableExpense(e) => AccountingLogic::VariableExpense(e),
            AccountingLogicModel::VariableExpenseInit { account, estimate } => {
                AccountingLogic::VariableExpenseInit { account, estimate }
            }
            AccountingLogicModel::ImmaterialIncome(i) => AccountingLogic::ImmaterialIncome(i),
            AccountingLogicModel::ImmaterialExpense(e) => AccountingLogic::ImmaterialExpense(e),
            AccountingLogicModel::Reimburse(r) => AccountingLogic::Reimburse(r),
            AccountingLogicModel::ClearVat { from, to } => AccountingLogic::ClearVat {
                from: from.into(),
                to: to.into(),
            },
        }
    }
}
