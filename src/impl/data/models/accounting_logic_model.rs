use crate::entities::{AccountingLogic, CommonStockWhileUnpaid, Handlers, ShareIssuanceCostBookTo};

use super::iso_date_model::ISODateModel;

#[derive(Debug, serde_derive::Deserialize)]
pub(crate) enum CommonStockWhileUnpaidModel {
    ReceivableAsset,
    NegativeEquity,
}

#[derive(Debug, serde_derive::Deserialize)]
pub(crate) enum ShareIssuanceCostBookToModel {
    RetainedEarnings,
    DiscountOnStockIssuance,
}

/// The accounting logic model over the handler types of `H`.
pub(crate) type AccountingLogicModelOf<H> = AccountingLogicModel<
    <H as Handlers>::E,
    <H as Handlers>::A,
    <H as Handlers>::I,
    <H as Handlers>::R,
    <H as Handlers>::S,
>;

#[derive(Debug, serde_derive::Deserialize)]
pub(crate) enum AccountingLogicModel<E, A, I, R, S> {
    CommonStock {
        subscriber: S,
        while_unpaid: CommonStockWhileUnpaidModel,
    },
    ShareIssuanceCost {
        book_to: ShareIssuanceCostBookToModel,
    },
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

impl<E, A, I, R, S> From<AccountingLogicModel<E, A, I, R, S>> for AccountingLogic<E, A, I, R, S> {
    fn from(val: AccountingLogicModel<E, A, I, R, S>) -> Self {
        match val {
            AccountingLogicModel::CommonStock {
                subscriber,
                while_unpaid,
            } => AccountingLogic::CommonStock {
                subscriber,
                while_unpaid: match while_unpaid {
                    CommonStockWhileUnpaidModel::ReceivableAsset => {
                        CommonStockWhileUnpaid::ReceivableAsset
                    }
                    CommonStockWhileUnpaidModel::NegativeEquity => {
                        CommonStockWhileUnpaid::NegativeEquity
                    }
                },
            },
            AccountingLogicModel::ShareIssuanceCost { book_to } => {
                AccountingLogic::ShareIssuanceCost {
                    book_to: match book_to {
                        ShareIssuanceCostBookToModel::RetainedEarnings => {
                            ShareIssuanceCostBookTo::RetainedEarnings
                        }
                        ShareIssuanceCostBookToModel::DiscountOnStockIssuance => {
                            ShareIssuanceCostBookTo::DiscountOnStockIssuance
                        }
                    },
                }
            }
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
