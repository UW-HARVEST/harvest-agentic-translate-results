# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libdriver.so`, compared against `translation/target/release/libdriver.so`.

## C translation units

| C file | translated? | notes |
|--------|-------------|-------|
| `c_src/include/driver.h` | yes | declares `driver` only |
| `c_src/src/driver.c` | yes | `fma_array`, `call_fma`, `driver` (all three have external linkage; none are `static`) |

There is exactly ONE C source file. No module was skipped.

## Global text symbols (`T`) exported by the C `.so`

| # | symbol | C signature | exported by Rust `.so`? |
|---|--------|-------------|--------------------------|
| 1 | `fma_array` | `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)` | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn fma_array`) |
| 2 | `call_fma`  | `int call_fma(const int *data, int len)` | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn call_fma`) |
| 3 | `driver`    | `void driver(const char *in)` | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn driver`) |

No macro-generated symbols exist in this library (no function-defining macros in
`driver.c` or `driver.h`).

## Diff result

```
$ comm -23 <(c_symbols) <(rust_symbols)   # in C but not in Rust
(empty)
```

`tests/symbols.rs::c_symbols_are_all_exported_by_rust` enforces this
mechanically at test time (it shells out to `nm -D`), so the parity check is
re-verified on every `cargo test` run.

## Undefined (imported) symbols in the Rust `.so`

All non-libc undefined symbols must be absent. The Rust `.so` imports only libc
/ `ld.so` symbols (`sscanf`, `printf`, `memset`, `malloc`, `_Unwind_*`,
`__cxa_*`, `__libc_start_main`-family, etc.). Verified by
`tests/symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
build configuration is the default one (`--no-default-features` is equivalent).
Phase D's "repeat for every feature combination" therefore collapses to the
single default configuration; the test harness still runs the
`--no-default-features` variant explicitly to prove it.
