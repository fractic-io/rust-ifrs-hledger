mod r#impl;
pub(crate) use r#impl::data;
pub(crate) use r#impl::domain;
pub use r#impl::exports::*;
pub(crate) use r#impl::presentation;

mod impl_ext;
pub mod ext {
    pub use super::impl_ext::exports::*;
    pub(crate) use super::impl_ext::standard_decorators;
}

pub mod errors;
pub mod util;
