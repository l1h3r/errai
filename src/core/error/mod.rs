//! Error handling utilities for system errors.

mod macros;

pub(crate) use self::macros::fatal;
pub(crate) use self::macros::raise;
