// Crate-internal.
// ---

pub(crate) mod data {
    pub(crate) mod datasources {
        pub(crate) mod balances_csv_datasource;
        pub(crate) mod transactions_csv_datasource;
    }
    pub(crate) mod models {
        pub(crate) mod accounting_amount_model;
        pub(crate) mod accounting_logic_model;
        pub(crate) mod backing_account_model;
        pub(crate) mod iso_date_model;
    }
    pub(crate) mod repositories {
        pub(crate) mod records_repository_impl;
    }
}

pub(crate) mod domain {
    pub(crate) mod entities {
        pub(crate) mod account;
        pub(crate) mod annotation;
        pub(crate) mod assertion;
        pub(crate) mod assertion_spec;
        pub(crate) mod cashflow_tracing_tag;
        pub(crate) mod decorator_logic;
        pub(crate) mod financial_records;
        pub(crate) mod handlers;
        pub(crate) mod transaction;
        pub(crate) mod transaction_spec;
    }
    pub(crate) mod logic {
        pub(crate) mod account_impl;
        pub(crate) mod annotation_processor;
        pub(crate) mod decorator_processor;
        pub(crate) mod spec_processor;
        mod utils;
    }
    pub(crate) mod repositories {
        pub(crate) mod records_repository;
    }
    pub(crate) mod usecases {
        pub(crate) mod process_usecase;
    }
}

pub(crate) mod presentation {
    pub(crate) mod account_fmt;
    pub(crate) mod cashflow_tracing_tag_fmt;
    pub(crate) mod hledger_printer;
    pub(crate) mod utils;
}

// Public exports.
// ---

#[doc(hidden)]
#[allow(unused_imports)]
pub mod exports {
    // This mod represents how clients see the library, and can differ from the
    // internal structure.
    //
    // The contents of this mod are re-exported in the root of the crate.

    pub mod entities {
        pub use crate::domain::entities::account::*;
        pub use crate::domain::entities::annotation::*;
        pub use crate::domain::entities::assertion::*;
        pub use crate::domain::entities::assertion_spec::*;
        pub use crate::domain::entities::cashflow_tracing_tag::*;
        pub use crate::domain::entities::decorator_logic::*;
        pub use crate::domain::entities::financial_records::*;
        pub use crate::domain::entities::handlers::*;
        pub use crate::domain::entities::transaction::*;
        pub use crate::domain::entities::transaction_spec::*;
    }
}
