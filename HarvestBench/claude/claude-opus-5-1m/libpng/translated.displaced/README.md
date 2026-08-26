# libpng 1.6.59 translated to Rust

A complete, faithful translation of the C library in `c_src/` (libpng 1.6.59.git,
the configuration recorded in `c_src/include/pnglibconf.h`) into a Rust `cdylib`
that exports the **same ABI** and produces **byte-identical output**.

```
cargo build --release      →  target/release/libpng.so
```

## Layout

| Path | Contents |
|---|---|
| `src/ctypes.rs` | C types (`pngconf.h`), libc bindings, zlib bindings, the private setjmp/longjmp pair |
| `src/pngtypes.rs` | every structure (`png_struct`, `png_info`, `png_control`, `png_image`, …) with **C-identical layout** |
| `src/pngconsts.rs` | every `PNG_*`/`png_*` constant and every function-like preprocessor macro, as `#[inline] fn`s |
| `src/png.rs`, `src/pngerror.rs`, … | one module per C source file, assembled with `include!` from the per-chunk translations (`src/<file>_pN.rs`) |
| `src/png_tables.rs` | the sRGB tables, transcribed and verified byte for byte against the C `.so` |
| `src/layout.rs` | `#[cfg(test)]` checks: struct offsets/sizes and the setjmp/longjmp pair |
| `verify/` | the equivalence test harnesses and scripts (see below) |
| `SIGNATURES.md` | the required Rust signature of all 381 exported functions, derived from the headers |
| `CONVENTIONS.md` | the translation rules that were followed |

Every C file maps 1:1 onto a Rust module; C `static` functions became private
Rust functions, and each of the 381 exported functions is
`#[unsafe(no_mangle)] pub unsafe extern "C" fn` with the signature from the
public/private headers (including the internal ones declared with
`PNG_INTERNAL_FUNCTION`, which the C build also exports).

## Verification

```
./verify/check_symbols.sh    # exported symbol sets must be equal
./verify/run_compare.sh      # behavioural diff, C build vs Rust build
cargo test                   # struct layout + setjmp/longjmp unit tests
```

* **ABI**: `nm -D` on the C `.so` and on the Rust `.so` list exactly the same
  384 symbols — 381 functions plus the read-only data `png_sRGB_table`,
  `png_sRGB_base`, `png_sRGB_delta`.
* **Layout**: 241 struct offsets/sizes (including `png_struct` = 1232 bytes,
  `png_info` = 352 bytes, `jmp_buf`, `z_stream`) are asserted equal to the C
  values.
* **Behaviour**: `verify/harness.c` and `verify/harness2.c` drive the public API
  through writing (15 colour-type/bit-depth combinations × filter/compression
  settings × all ancillary chunks), reading (19 transform sets each, sequential,
  high-level and progressive), the simplified API (memory/file/stdio, colour
  mapped and linear formats, background composition), user callbacks, custom
  memory handlers, hand-crafted datastreams (mis-ordered chunks, broken ICC
  profiles, MNG filtering) and the error paths.  The two builds emit **identical
  bytes** for all 26,702 lines of hashes, chunk dumps, warnings and error
  messages.  Coverage measurement (`gcov` on the C build) shows the harnesses
  execute 516 of the 528 C functions and 73% of all C lines.

## zlib

libpng does not implement DEFLATE; it calls zlib.  This translation links the
**same system zlib** the C reference build links (`libz.so.1`), because that is
the only way to guarantee byte-identical compressed output — a different DEFLATE
implementation (e.g. a pure-Rust one) produces valid but different IDAT bytes.
`build.rs` locates the library and, on systems that ship only the versioned
SONAME, creates a link inside `OUT_DIR`.  The resulting `.so` has exactly the
same dependency set as the C one.

## Notes on fidelity

* Bugs and quirks of the C code are reproduced, not fixed (e.g. the missing
  `mins = sum` in the Paeth branch of `png_write_find_filter`, the `hIST`
  position rule that rejects libpng's own output, the `png_do_expand` 4-bit
  `shift = 4`, the unchecked `png_ptr` dereferences in some setters, and the
  "Out of memory"/"Out of Memory" spelling difference).
* Error and warning strings, the order of validation checks, integer promotion
  and wrap-around behaviour, and all floating point maths (through the same libm
  `pow`/`floor`/`modf`/`frexp`) are preserved.
* libpng's *internal* jmp_buf (`png_control::error_buf`, used by
  `png_safe_execute`) uses a private setjmp/longjmp pair implemented in
  `src/ctypes.rs`; the application-visible `jmp_buf` keeps the glibc layout and
  is always used through the caller supplied `longjmp_fn`, exactly as in C.
