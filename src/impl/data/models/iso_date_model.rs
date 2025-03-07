use std::str::FromStr;

use chrono::NaiveDate;
use fractic_server_error::ServerError;
use serde::Deserialize;

use crate::errors::InvalidIsoDate;

#[derive(Debug)]
pub(crate) struct ISODateModel(NaiveDate);
impl FromStr for ISODateModel {
    type Err = ServerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| InvalidIsoDate::with_debug(s, &e))?;
        Ok(ISODateModel(d))
    }
}
impl<'de> Deserialize<'de> for ISODateModel {
    fn deserialize<D>(deserializer: D) -> Result<ISODateModel, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ISODateModel::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Into<NaiveDate> for ISODateModel {
    fn into(self) -> NaiveDate {
        self.0
    }
}
