use crate::utils::is_utf8;

impl_stack_string! {
  /// 3-byte language code.
  @ident = Language;
  @bytes = 3;
  @check = is_utf8;
}
