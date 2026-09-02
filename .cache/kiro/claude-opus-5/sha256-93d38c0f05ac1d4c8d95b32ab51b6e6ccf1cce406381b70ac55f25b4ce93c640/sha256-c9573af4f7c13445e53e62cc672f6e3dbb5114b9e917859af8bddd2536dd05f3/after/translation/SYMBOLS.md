# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory

`c_src/CMakeLists.txt` builds exactly one translation unit into `libdriver.so`:

```
add_library(driver SHARED src/lib.c)
```

So `c_src/src/lib.c` is the *entire* library — there is no second module that
could have been skipped by the translation step. `c_src/include/lib.h` declares
only `w_utf8_filter`; `w_utf8_drop` is defined non-`static` in `lib.c` and is
therefore also exported even though no header declares it.

The `valid_1`/`valid_2`/`valid_3`/`valid_4` helpers are **object-like/function-like
macros**, not functions, so they produce no symbols of their own. `REPLACEMENT_INC`
likewise. There are no macro-generated exported symbols in this library.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `w_utf8_drop`   | `T` | `T` | `const char *(*)(const char *)` — not in any header, still exported |
| 2 | `w_utf8_filter` | `T` | `T` | `char *(*)(const char *, _Bool)` — declared in `include/lib.h` |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

Nothing needed a new `#[no_mangle]` wrapper and no C module was left
untranslated — `lib.c` is fully covered by `translation/src/lib.rs`.

## Undefined symbols in the Rust `.so`

All undefined symbols in `translation/target/release/libdriver.so` are libc or
unwinder imports (`malloc`, `realloc`, `strdup`, `strlen`, `memcpy`, `abort`,
`free`, `_Unwind_*`, `__errno_location`, `pthread_*`, `dl_iterate_phdr`, …).
The `_Unwind_*` / `dl_iterate_phdr` / `open64` / `readlink` group comes from the
std panic + backtrace machinery pulled in by the `assert!` calls, not from
missing translated code.

**Missing / undefined non-libc symbols in Rust: 0.**

The Rust `.so` also imports the same four allocator/string functions the C `.so`
imports (`malloc`, `realloc`, `strdup`, `strlen`, `memcpy`), which is required
for behavioural parity: buffers returned to the caller must be `free()`-able by
the caller, so the translation deliberately uses the C allocator rather than
Rust's.

## Signature note (ABI-relevant, checked against generated code)

The C `_Bool` parameter of `w_utf8_filter` is one byte and the compiled C tests
it with `cmpb $0x0,-0x3c(%rbp)` — **any non-zero byte is "true"**. Rust's `bool`
is undefined behaviour for byte values other than `0`/`1`, so the exported Rust
wrapper takes `replacement: u8` (identical ABI — low byte of the second integer
argument register) and compares `!= 0`. This keeps the two identical for
out-of-range boolean bytes such as `2` or `0xFF` coming across the FFI boundary.

## Verified output

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001169 T w_utf8_drop
0000000000001375 T w_utf8_filter

$ nm -D --defined-only translation/target/release/libdriver.so
00000000000117b0 T w_utf8_drop
00000000000118b0 T w_utf8_filter
```

Enforced as a test, not a claim: `tests/phase_d_symbols.rs`

* `phase_d_every_c_symbol_is_exported_by_rust` — set-difference of `nm -D
  --defined-only` (C minus Rust) must be empty; also asserts both entry points
  are present by name.
* `phase_d_no_unexpected_undefined_symbols` — every undefined symbol in the Rust
  `.so` must be a libc / unwinder import.

Both pass. **Missing symbols: 0. Undefined non-libc symbols: 0.**
