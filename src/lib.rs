//! Errai - A BEAM-inspired runtime

mod bifs;
mod core;

pub mod erts;
pub mod init;
pub mod lang;
pub mod node;
pub mod tyre;

pub mod error {
  pub use crate::core::Exception;
  pub use crate::core::ExceptionClass;
  pub use crate::core::ExceptionGroup;
}
