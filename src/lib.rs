#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("errai requires a 32-bit or 64-bit architecture");

mod utils;

pub mod core;
