// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory, 2025).
//
// The C build globs c_src/src/*.c into one shared object; that single
// translation unit (src/lib.c) exports ten public symbols, all of which are
// re-exported here from the modules below:
//
//   counter.rs  -- increment_counter, decrement_counter, multiply_counter,
//                  reset_counter
//   helpers.rs  -- is_string_empty, find_char_in_buffer, create_buffer,
//                  validate_uint16_range, apply_operation
//   charinbuf.rs -- charinbuf   (the only symbol declared in include/lib.h)
//
// No namespace-renaming macros are present in the public header, so each
// #[unsafe(no_mangle)] name is also the final linker symbol.

mod charinbuf;
mod counter;
mod cstd;
mod helpers;

// Re-exported so the definitions are reachable from the crate root as well; the
// #[unsafe(no_mangle)] attributes on the items themselves are what create the
// exported C symbols.
pub use charinbuf::charinbuf;
pub use counter::{decrement_counter, increment_counter, multiply_counter, reset_counter};
pub use helpers::{
    apply_operation, create_buffer, find_char_in_buffer, is_string_empty, validate_uint16_range,
};
