// Crate-internal.
// ---

pub(crate) mod standard_decorators {
    pub(crate) mod card_fx;
    pub(crate) mod korea_vat;
    pub(crate) mod payment_fee;
}

// Public exports.
// ---

pub mod exports {
    // This mod represents how clients see the library, and can differ from the
    // internal structure.
    //
    // The contents of this mod are re-exported in the root of the crate.

    pub mod standard_decorators {
        pub use crate::impl_ext::standard_decorators::card_fx::*;
        pub use crate::impl_ext::standard_decorators::korea_vat::*;
        pub use crate::impl_ext::standard_decorators::payment_fee::*;
    }
}
