//! Reference types for runtime objects.
//!
//! References uniquely identify monitors, timers, and aliases within the
//! runtime. They are generated atomically and are guaranteed unique within
//! a node's lifetime.

mod alias;
mod local;

pub use self::alias::AliasRef;
pub use self::alias::MonitorRef;
pub use self::alias::TimerRef;

pub use self::local::LocalRef;
