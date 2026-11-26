use crate::entities::BackingAccount;

#[derive(Debug, serde_derive::Deserialize)]
pub enum BackingAccountModel<R, C, S> {
    Reimburse(R),
    Cash(C),
    ContributedSurplus(S),
}

impl<R, C, S> From<BackingAccountModel<R, C, S>> for BackingAccount<R, C, S> {
    fn from(val: BackingAccountModel<R, C, S>) -> Self {
        match val {
            BackingAccountModel::Reimburse(r) => BackingAccount::Reimburse(r),
            BackingAccountModel::Cash(c) => BackingAccount::Cash(c),
            BackingAccountModel::ContributedSurplus(s) => BackingAccount::ContributedSurplus(s),
        }
    }
}
