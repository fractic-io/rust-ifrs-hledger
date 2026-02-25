use chrono::NaiveDate;

use crate::entities::CloseLogic;

#[derive(Debug, Clone)]
pub enum Operation {
    Close {
        date: NaiveDate,
        entries: Vec<(String, f64)>,
        logic: CloseLogic,
    },
    Correction {
        date: NaiveDate,
        result: String,
    },
}
