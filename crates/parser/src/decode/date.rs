use crate::utils::is_ascii_digit;

impl_stack_string! {
  /// A generic date string in YYYYMMDD format.
  @ident = Date;
  @bytes = 8;
  @check = is_ascii_digit;
}
