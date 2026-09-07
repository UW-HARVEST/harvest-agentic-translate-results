# SYMBOLS.md — Public symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-ATf57c.so
nm -D --defined-only translation/target/release/liboverunder_lib.so
```

## C source inventory (`c_src/src/lib.c`)

All non-`static` functions in the single C translation unit:

| C function | signature |
|---|---|
| `safe_double_to_int` | `int safe_double_to_int(double d)` |
| `process_with_fallthrough` | `int process_with_fallthrough(int code, int base_value)` |
| `copy_data_block` | `void copy_data_block(DataBlock *dest, const DataBlock *src)` |
| `handle_pointer_operations` | `int handle_pointer_operations(int value)` |
| `overunder` | `int overunder(int a, int b, int c, int d)` (the only symbol declared in `include/lib.h`) |

No macro-generated symbols exist: `MAKE_VAR_NAME` / `PRINT_VAR` only paste
*local variable* names and format-string literals, so they generate no external
linkage.

## Exported-symbol table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|--------------|--------|
| 1 | `copy_data_block`          | T | T | OK |
| 2 | `handle_pointer_operations`| T | T | OK |
| 3 | `overunder`                | T | T | OK |
| 4 | `process_with_fallthrough` | T | T | OK |
| 5 | `safe_double_to_int`       | T | T | OK |

**Missing from Rust: 0. Extra in Rust: 0.** The symbol diff is EMPTY.

## Undefined (imported) symbols

The C `.so` imports only libc/libm: `memcpy`, `printf`, `putchar`, `sqrt`,
`strncpy`, plus the usual weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`.

The Rust `.so` imports `printf` (the translation deliberately calls the *same*
libc `printf` so that formatting and the `stdout` buffer are shared), plus the
Rust-runtime/libc set (`memcpy`, `memmove`, `memset`, `malloc`, `free`,
`_Unwind_*`, `dl_iterate_phdr`, …). All are libc / libgcc_s / ld.so provided.

**0 missing or undefined non-libc symbols in the Rust `.so`.** Verified with
`ldd -r translation/target/release/liboverunder_lib.so` (no "undefined symbol"
lines).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only configuration is the default (empty) feature set. `--no-default-features`
and the default build are the same build. There is no `[[bin]]`/`src/main.rs`
in the crate and no `add_executable` in `c_src/CMakeLists.txt`, so **the project
builds no driver binary** — the "compare binary stdout" gate is satisfied
vacuously; stdout is nevertheless compared byte-for-byte through the FFI
boundary by capturing fd 1 around each call (see `tests/differential.rs`).
