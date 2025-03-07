use crate::entities::BackingAccount;

#[derive(Debug, serde_derive::Deserialize)]
pub enum BackingAccountModel<R, C> {
    Reimburse(R),
    Cash(C),
}

impl<R, C> Into<BackingAccount<R, C>> for BackingAccountModel<R, C> {
    fn into(self) -> BackingAccount<R, C> {
        match self {
            BackingAccountModel::Reimburse(r) => BackingAccount::Reimburse(r),
            BackingAccountModel::Cash(c) => BackingAccount::Cash(c),
        }
    }
}
