use chrono::NaiveDate;

use crate::entities::TransactionSpecId;

#[derive(Debug)]
pub enum NoteType {
    Tmp,
}

#[derive(Debug)]
pub struct Note {
    pub spec_id: TransactionSpecId,
    pub accrual_date: NaiveDate,
    pub until: Option<NaiveDate>,
    pub entity: String,
    pub description: String,
    pub note: NoteType,
}
