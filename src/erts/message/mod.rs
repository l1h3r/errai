use crate::lang::DynPid;
use crate::lang::ExitReason;

#[derive(Debug)]
pub enum Message<T> {
  Term(T),
  Exit(DynPid, ExitReason),
}
