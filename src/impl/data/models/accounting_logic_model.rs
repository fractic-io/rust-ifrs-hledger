use crate::entities::{AccountingLogic, CommonStockWhileUnpaid};

use super::iso_date_model::ISODateModel;

#[derive(Debug, serde_derive::Deserialize)]
pub enum CommonStockWhileUnpaidModel {
    Asset,
    NegativeEquity,
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum AccountingLogicModel<E, A, I, R, S> {
    CommonStock {
        subscriber: S,
        while_unpaid: CommonStockWhileUnpaidModel,
    },
    CostOfEquity,
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
    ReimbursePartial(R),
    ClearVat {
        from: ISODateModel,
        to: ISODateModel,
    },
}

impl<E, A, I, R, S> Into<AccountingLogic<E, A, I, R, S>> for AccountingLogicModel<E, A, I, R, S> {
    fn into(self) -> AccountingLogic<E, A, I, R, S> {
        match self {
            AccountingLogicModel::CommonStock {
                subscriber,
                while_unpaid,
            } => AccountingLogic::CommonStock {
                subscriber,
                while_unpaid: match while_unpaid {
                    CommonStockWhileUnpaidModel::Asset => CommonStockWhileUnpaid::Asset,
                    CommonStockWhileUnpaidModel::NegativeEquity => {
                        CommonStockWhileUnpaid::NegativeEquity
                    }
                },
            },
            AccountingLogicModel::CostOfEquity => AccountingLogic::CostOfEquity,
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
            AccountingLogicModel::ReimbursePartial(r) => AccountingLogic::ReimbursePartial(r),
            AccountingLogicModel::ClearVat { from, to } => AccountingLogic::ClearVat {
                from: from.into(),
                to: to.into(),
            },
        }
    }
}
