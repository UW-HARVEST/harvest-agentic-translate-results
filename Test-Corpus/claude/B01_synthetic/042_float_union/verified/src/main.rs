// Rust translation of c_src/src/main.c — executable entry point.
//
// The whole translation lives in `imp.rs`; it is compiled into this binary and
// (via `lib.rs`) into the shared object used by the differential tests, so that
// both artefacts are built from one and the same source.
//
// `imp.rs` is included with `#[path]` rather than through the `driver` library
// crate on purpose: `lib.rs` exports a `#[no_mangle] extern "C" fn main`, which
// would collide with this binary's own `main` symbol at link time.

#[path = "imp.rs"]
mod imp;

fn main() {
    imp::run();
}
