# Differential-test harness — how to write a test file

Crate: `translated_rust` (libpng 1.6.59 C→Rust). Working dir for all commands:
`/local/home/scheschb/.harvest/work/harvest-work-4DaaTZ/translated_rust`.

The C reference `.so` is `target/cbuild/libpng.so`; the Rust `.so` is
`target/debug/liblibpng.so`. **Both are loaded with `libloading`** and every
libpng call goes through `dlsym`. Never call the Rust crate directly.

## Skeleton of a test file (`tests/<group>.rs`)

```rust
//! <group> differential tests (CONFIGS.md rows Xn..Xm)
mod support;

use std::ffi::{c_char, c_int, c_void};
use support::core::*;
use support::*;

#[test]
fn r1_read_all_colour_types() {
    for &(ct, bd) in COMBOS {
        let png_bytes = /* build input, identical for both libs */;
        diff(&format!("R1 ct={ct} bd={bd}"), |lib| {
            with_read(lib, &png_bytes, &mut |c, png, info| unsafe {
                (c.read_info)(png, info);
                log_all_info(c, png, info);
                /* ... rows ... */
                (c.read_end)(png, std::ptr::null_mut());
            })
        });
    }
}
```

`diff(label, f)` runs `f` once with the C `Lib` and once with the Rust `Lib` and
panics if the two traces differ (it prints the first differing trace line).

## The API surface

`support::core::Core::new(lib)` resolves ~200 typed function pointers
(`c.read_info`, `c.set_IHDR`, `c.write_row`, ...). Read
`tests/support/core.rs` for the exact field names/signatures — they mirror
`png.h` names with the `png_` prefix removed. It also defines all the
`PNG_*` constants, `PngColor16`, `PngColor8`, `PngTime`, `PngText`,
`PngUnknownChunk`, `PngSpltT`, `PngSpltEntry`, `PngImage`, `PngRowInfo`
(`#[repr(C)]`, verified against `png.h`).

Anything not in `Core` is fetched ad hoc, e.g.

```rust
let muldiv: unsafe extern "C" fn(*mut i32, i32, i32, i32) -> c_int = lib.f("png_muldiv");
```

Signatures MUST be taken from `c_src/include/png.h` / `c_src/include/pngpriv.h`.
Map: `png_uint_32`→`u32`, `png_int_32`/`png_fixed_point`→`i32`, `png_byte`→`u8`,
`png_uint_16`→`u16`, `size_t`→`usize`, `int`→`c_int`, `double`→`f64`,
`png_structrp`/`png_structp`/`png_const_structrp`→`Png` (= `*mut c_void`),
`png_inforp`→`Info`, `png_bytep`→`*mut u8`, `png_const_bytep`→`*const u8`,
`png_charp`→`*mut c_char`, callback pointers→`Cb` (= `*mut c_void`, pass
`my_cb as Cb`).

## Helpers you must use

* `with_write(lib, &mut |c, png, info| { ... }) -> Trace` — creates a write
  struct with the harness error/warning/write/flush callbacks and a longjmp
  landing pad, runs the body, destroys the struct. The produced bytes end up in
  `Trace::out`.
* `with_read(lib, input_bytes, &mut |c, png, info| { ... }) -> Trace` — same for
  reading; the read callback serves `input_bytes`.
* `protected(|| ...) -> c_int` — raw setjmp landing pad, for tests that build
  their own structs (e.g. `png_create_read_struct_2`). Returns 0 on normal
  completion, non-zero if libpng longjmp'ed. **Anything after a `png_error` is
  not executed**, so put the trace-relevant calls before it.
* `log(...)` — append a line to the trace. Everything you want compared must be
  logged.
* `log_all_info(c, png, info)` — logs every ancillary-chunk getter result.
* `hex(&[u8])` — hex string of a byte slice (use it to log rows/buffers).
* `cstr(*const c_char)` — safe C string → `String` (logs `<null>` for NULL).
* `Rng::new(seed)` — deterministic xorshift PRNG: `next_u32`, `below(n)`,
  `byte()`, `bytes(n)`, `f64()`. **Every test must use a fixed seed.**
* `with_session(|s| ...)` — access the session: `s.trace_alloc = true` (log
  every user-memory malloc/free size), `s.malloc_limit = Some(n)` (fail malloc
  after n allocations), `s.write_limit`, `s.input`, `s.rpos`.
* `support::pngbuild` — build/dissect PNG datastreams without either library:
  `Builder::new(w,h,depth,color).interlace(1).add(b"gAMA", data).build_valid(seed)`,
  `zlib_stored(&raw)`, `Chunk::new(b"tEXt", data).bad_crc()`, `split(png)`,
  `join(&chunks)`, `rowbytes()`, `pass_width/pass_height`, `crc32`, `adler32`.
* Callbacks provided by the harness (pass as `Cb`): `cb_error`, `cb_warning`,
  `cb_read`, `cb_write`, `cb_flush`, `cb_malloc`, `cb_free`.
* `shim().longjmp_ptr` / `shim().jmp_buf_size` for `png_set_longjmp_fn`.

## Hard rules

1. **NEVER log a pointer value, address, or anything else that legitimately
   differs between two independent libraries.** Log only null-ness, sizes,
   contents. (`log_all_info` already follows this rule.)
2. **NEVER weaken a test to make it pass.** If the C and Rust traces differ,
   that is either (a) a bug in your test/harness use, or (b) a genuine
   translation bug. Investigate; if it is (b), leave the test in place and
   REPORT it — do not `#[ignore]` it, do not relax the assertion, and do not
   edit anything in `src/` or `c_src/`.
3. Use several randomized inputs per configuration (a loop over 8..64 seeds or
   shapes), not one hand-picked value.
4. Keep individual test functions fast (whole file well under 60 s). Images of
   1..40 pixels per side are plenty; a few larger ones are fine.
5. Closures passed to `protected`/`with_*` must not own values needing `Drop`
   (a `png_error` longjmp skips destructors). Allocate `Vec`s *outside* the
   closure and use raw pointers/slices inside.
6. `libpng` writes into caller buffers: always allocate `rowbytes+8` slack and
   compare exactly what the C wrote (log the whole buffer).
7. Rows are read into buffers you own; for `png_read_row(png, row, display)` you
   may pass NULL for either but not both.
8. Only ONE `mod support;` per test file; use `#[test]` functions.

## Running

**`cargo test` does NOT rebuild the cdylib** (the test targets do not link it —
they `dlopen` it), so you MUST build first, otherwise you are testing a stale
Rust `.so`:

```bash
cd /local/home/scheschb/.harvest/work/harvest-work-4DaaTZ/translated_rust
timeout 600 cargo build && timeout 600 cargo test --test <group> 2>&1 | tail -40
# a single test:
timeout 600 cargo test --test <group> <fn_name> -- --nocapture 2>&1 | tail -40
```

If `target/cbuild/libpng.so` is missing:

```bash
cmake -S c_src -B target/cbuild -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build target/cbuild -j8
```
