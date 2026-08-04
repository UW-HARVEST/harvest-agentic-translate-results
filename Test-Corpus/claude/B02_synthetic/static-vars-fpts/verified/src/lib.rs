// Library entry point. Re-exports both internal modules and provides
// `#[no_mangle]` C ABI wrappers that match the symbols exported by the
// C shared library, so that external callers (and tests via libloading)
// see the same interface across both implementations.

pub mod analyzer;
pub mod ffi;
pub mod tokenizer;
