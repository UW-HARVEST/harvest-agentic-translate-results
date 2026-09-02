# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
C_SO=$(ls c_src/build/*.so); R_SO=translation/target/debug/libmathop_lib.so
nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/r_syms.txt   # extra in Rust
```

C `.so`: `c_src/build/libharvest-work-m25tNI.so` (from `c_src/CMakeLists.txt`,
project name is derived from the parent directory name).
Rust `.so`: `translation/target/{debug,release}/libmathop_lib.so`
(`[lib] name = "mathop_lib"`, `crate-type = ["cdylib"]`).

## Table

All 12 symbols come from the single translation unit `c_src/src/lib.c`. There
are no macro-generated symbols and no second C module, so there is no
"whole file was never translated" gap to close.

| # | C symbol | C type | in C `.so` | in Rust `.so` | notes |
|---|----------|--------|-----------|---------------|-------|
| 1 | `add_operation` | `T` | yes | yes | `#[unsafe(no_mangle)] pub extern "C"` |
| 2 | `allocate_results` | `T` | yes | yes | |
| 3 | `divide_operation` | `T` | yes | yes | |
| 4 | `get_computation_timestamp` | `T` | yes | yes | returns `time_t` = `i64` |
| 5 | `get_operation_priority` | `T` | yes | yes | |
| 6 | `is_valid_operation` | `T` | yes | yes | returns `_Bool` / `bool` |
| 7 | `mathop` | `T` | yes | yes | the only symbol in `include/lib.h` |
| 8 | `modulo_operation` | `T` | yes | yes | |
| 9 | `multiply_operation` | `T` | yes | yes | |
| 10 | `perform_computation_with_history` | `T` | yes | yes | |
| 11 | `select_operation` | `T` | yes | yes | returns `MathOperation` fn pointer |
| 12 | `subtract_operation` | `T` | yes | yes | |

## Result

- C exported symbols: 12
- Rust exported symbols: 12
- **Missing from Rust: 0**
- **Extra in Rust: 0**

Undefined symbols in the Rust `.so` are all libc / libgcc-unwind imports
(`printf@GLIBC_2.2.5`, `time@GLIBC_2.2.5`, `calloc@GLIBC_2.2.5`, `malloc`,
`memcpy`, `_Unwind_*`, `__cxa_finalize`, ...). **0 undefined non-libc
symbols.**

## ABI facts confirmed by probe (x86-64 Linux, gcc)

| item | value |
|------|-------|
| `sizeof(ComputationResult)` | 24 |
| `_Alignof(ComputationResult)` | 8 |
| `offsetof(.value)` | 0 |
| `offsetof(.timestamp)` | 8 (4 bytes of padding after `value`) |
| `offsetof(.status)` | 16 (4 bytes of tail padding) |
| `sizeof(time_t)` | 8, signed |
| `sizeof(StatusCode)` / `sizeof(Operation)` | 4 (plain `int`) |
| `char` signedness | signed |

These match `#[repr(C)] struct ComputationResult { c_int, i64, c_int }` and
`type Operation = c_int` / `type c_char = i8` in `translation/src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `cargo check`/`cargo test` with
`--no-default-features` and with the (empty) set of feature combinations are
therefore all the same build; this is verified by
`scripts/check_feature_combos.sh`.

## Verification log (`scripts/verify_all.sh`)

Re-checked after every change, for each of the 4 configurations
(`--default` / `--no-default-features` x `dev` / `release`):

```
C symbols: 12   Rust symbols: 12
0 symbols missing from Rust
0 symbols extra in Rust
0 undefined non-libc symbols
phase_b_leaf:     14 passed
phase_b_composed: 17 passed
phase_c_errors:   26 passed   (+1 ignored child-process entry point)
ALL CHECKS PASSED
```
