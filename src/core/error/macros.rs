macro_rules! fatal {
  ($error:expr) => {{
    ::std::eprintln!(
      "{}:{}: (SysInv) a system invariant has been broken: {}",
      ::std::file!(),
      ::std::line!(),
      $error,
    );

    ::std::process::abort();
  }};
}

macro_rules! raise {
  (Error, SysCap, $error:expr) => {
    ::std::panic!(
      "{}:{}: (SysCap) a system limit has been reached: {}",
      ::std::file!(),
      ::std::line!(),
      $error,
    )
  };
}

pub(crate) use fatal;
pub(crate) use raise;
