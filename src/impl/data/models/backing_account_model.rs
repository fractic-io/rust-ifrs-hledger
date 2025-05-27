use crate::entities::BackingAccount;

#[derive(Debug, serde_derive::Deserialize)]
pub enum BackingAccountModel<R, C, S> {
    Reimburse(R),
    Cash(C),
    ContributedSurplus(S),
}

impl<R, C, S> Into<BackingAccount<R, C, S>> for BackingAccountModel<R, C, S> {
    fn into(self) -> BackingAccount<R, C, S> {
        match self {
            BackingAccountModel::Reimburse(r) => BackingAccount::Reimburse(r),
            BackingAccountModel::Cash(c) => BackingAccount::Cash(c),
            BackingAccountModel::ContributedSurplus(s) => BackingAccount::ContributedSurplus(s),
        }
    }
}
