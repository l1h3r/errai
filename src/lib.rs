//! Errai - A BEAM-inspired runtime

mod bifs;
mod core;
mod proc;
mod utils;

pub mod consts;
pub mod erts;
pub mod init;
pub mod node;
pub mod tyre;

pub mod error {
  //! Errai errors.

  pub use crate::core::Exception;
  pub use crate::core::ExceptionClass;
  pub use crate::core::ExceptionGroup;
}

pub mod types {
  //! Core types of the Errai runtime system.

  pub use crate::core::ProcTable;
  pub use crate::core::ProcTableFull;
  pub use crate::core::ProcTableKeys;

  pub use crate::core::Atom;
  pub use crate::core::Exit;
  pub use crate::core::Item;
  pub use crate::core::Term;

  pub use crate::core::ExternalDest;
  pub use crate::core::InternalDest;

  pub use crate::core::ExternalPid;
  pub use crate::core::InternalPid;

  pub use crate::core::ProcessId;

  pub use crate::core::ExternalRef;
  pub use crate::core::InternalRef;

  pub use crate::core::AliasRef;
  pub use crate::core::MonitorRef;
  pub use crate::core::TimerRef;
}
