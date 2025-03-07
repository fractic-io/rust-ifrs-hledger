#[derive(Debug, serde_derive::Deserialize)]
pub enum BackingAccount<R, C> {
    Reimburse(R),
    Cash(C),
}
