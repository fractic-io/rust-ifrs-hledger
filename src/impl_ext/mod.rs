// Crate-internal.
// ---

pub(crate) mod cash_flow_statement {
    pub(crate) mod generator;
}

pub(crate) mod standard_accounts {
    pub(crate) mod core;
    pub(crate) mod vat;
}

pub(crate) mod standard_decorators {
    pub(crate) mod card_fx;
    pub(crate) mod payment_fee;
    pub(crate) mod vat_korea;
}

// Public exports.
// ---

pub mod exports {
    // This mod represents how clients see the library, and can differ from the
    // internal structure.
    //
    // The contents of this mod are re-exported in the root of the crate.

    pub mod cash_flow_statement {
        pub use crate::impl_ext::cash_flow_statement::generator::*;
    }

    pub mod standard_accounts {
        pub use crate::impl_ext::standard_accounts::core::*;
        pub use crate::impl_ext::standard_accounts::vat::*;
    }

    pub mod standard_decorators {
        pub use crate::impl_ext::standard_decorators::card_fx::*;
        pub use crate::impl_ext::standard_decorators::payment_fee::*;
        pub use crate::impl_ext::standard_decorators::vat_korea::*;
    }
}
