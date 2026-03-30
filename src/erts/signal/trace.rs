macro_rules! trace_span {
  ($parent:expr, $name:expr, $($fields:tt)*) => {
    ::tracing::span!(
      target: "errai",
      parent: $parent,
      ::tracing::Level::TRACE,
      $name,
      $($fields)*
    )
  };
}

macro_rules! trace_enter {
  ($parent:expr) => {
    ::tracing::trace!(
      name: "signal",
      target: "errai",
      parent: $parent,
      action = "enter",
    );
  };
}

macro_rules! trace_leave {
  ($parent:expr, $result:expr) => {
    ::tracing::trace!(
      name: "signal",
      target: "errai",
      parent: $parent,
      action = "leave",
      result = $result,
    );
  };
}

pub(crate) use trace_enter;
pub(crate) use trace_leave;
pub(crate) use trace_span;
