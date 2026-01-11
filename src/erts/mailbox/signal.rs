use crate::lang::InternalPid;
use crate::lang::Term;

#[derive(Debug)]
pub(crate) enum Signal {
  Message(InternalPid, Term),
}
