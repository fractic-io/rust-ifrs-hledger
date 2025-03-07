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
        pub(crate) mod financial_records;
        pub(crate) mod handlers;
        pub(crate) mod transaction;
        pub(crate) mod transaction_spec;
    }
    pub(crate) mod logic {
        pub(crate) mod ifrs_logic;
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
    pub(crate) mod hledger_printer;
}

// Public exports.
// ---

pub mod exports {
    // This mod represents how clients see the library, and can differ from the
    // internal structure.
    //
    // The contents of this mod are re-exported in the root of the crate.

    pub mod entities {
        pub use crate::domain::entities::account::*;
        pub use crate::domain::entities::financial_records::*;
        pub use crate::domain::entities::handlers::*;
        pub use crate::domain::entities::transaction::*;
        pub use crate::domain::entities::transaction_spec::*;
    }
}
