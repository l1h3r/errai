use bitflags::bitflags;

bitflags! {
  #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub(crate) struct ProcFlags: u32 {
    const TRAP_EXIT = 1 << 22;
  }
}
