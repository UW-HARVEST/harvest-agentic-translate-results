# Differential-test harness (read this before adding tests)

Layout:
* `tests/common/types.rs`  — all public libpng types/constants, redeclared
  (the crate is a `cdylib`, tests never link it).
* `tests/common/api.rs`    — AUTO-GENERATED `struct Api` with one field per
  exported symbol (all 384), each an `unsafe extern "C-unwind" fn` pointer.
  `Api::load(path, name)` dlopens a `.so` and resolves every symbol.
* `tests/common/mod.rs`    — the harness proper.

Use it like this:

```rust
mod common;
use common::*;

#[test]
fn my_test() {
    for api in both() {            // [c_api(), rs_api()]
        unsafe {
            set_current_api(api);  // REQUIRED before any IO/error callback fires
            diag_reset();
            let s = WriteSess::new(api);          // or ReadSess::new(api, &bytes)
            let ok = guard(|| { /* calls that may png_error */ }).is_some();
            let d = diag_take();                  // captured warnings + errors
            let out = std::mem::take(&mut s.sink.buf);  // bytes written
        }
    }
}
```

Key facts:
* `guard(f) -> Option<T>`: `png_error` in either library unwinds as a Rust
  panic (all entry points are `extern "C-unwind"`); `guard` catches it and
  returns `None`.  ALWAYS wrap anything that can error.
* `Diag { warnings: Vec<String>, errors: Vec<String> }` — compare these between
  the two libraries, not just success/failure.
* `ReadSess`/`WriteSess` own the png_struct + info_struct and destroy them on
  drop.  `WriteSess.sink.buf` is the produced byte stream, `.sink.flushes` the
  flush count.  `ReadSess.src` is the memory source.
* Helpers: `cs(&str) -> CString`, `rs_str(ptr) -> Option<String>`,
  `ver() -> CString` (PNG_LIBPNG_VER_STRING), `Rng::new(seed)` (deterministic
  xorshift: `.u8() .u32() .below(n) .range(lo,hi) .bytes(n) .bool()`),
  `rowbytes(pixel_depth, width)` (= PNG_ROWBYTES), `channels_of(color_type)`,
  `legal_ihdr() -> Vec<(color_type, bit_depth)>`, `assert_bytes_eq(label,c,r)`,
  `PNG_PASS_INC/_ROW_INC/_START_ROW/_START_COL`.

Rules:
* NEVER call the Rust crate directly; always go through `c_api()`/`rs_api()`.
* NEVER modify anything under `c_src/`.
* The C is ground truth.  If the two diverge, the *Rust* (`src/*.rs`) is wrong.
* Some C paths dereference a pointer before checking it (e.g. `png_muldiv`'s
  `res`, `png_do_bgr`'s `row`, `png_convert_to_rfc1123_buffer`'s `ptime`).
  Those are C UB, not error paths -- do not test them.
* Rows handed to the write path must be over-allocated when a transform makes
  libpng consume more application bytes per row than PNG_ROWBYTES (filler,
  packing), otherwise both libraries read past the buffer and results are
  nondeterministic.
* Build/run with:
  `cargo build --offline --release && cargo test --offline --release --test <name>`
  (the release cdylib at `target/release/liblibpng.so` is what gets loaded).
