use crate::lang::InternalPid;
use crate::lang::Term;

pub(crate) enum Signal {
  Message(InternalPid, Term),
}
