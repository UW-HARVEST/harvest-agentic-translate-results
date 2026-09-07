# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-aZUQvV.so
nm -D --defined-only translation/target/release/libcheckshift_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`); there
are no additional C source files, so there is no "whole module never
translated" gap. All 10 C-exported functions have a real Rust implementation
(no stubs, no `unimplemented!()`).

## Symbol table

| # | C symbol (`nm -D`, type) | Rust `.so` exports it | Rust item | Signature (C) |
|---|--------------------------|-----------------------|-----------|---------------|
| 1 | `multiply_with_static` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn multiply_with_static` | `int (int, int)` |
| 2 | `add_with_static` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn add_with_static` | `int (int, int)` |
| 3 | `xor_operation` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn xor_operation` | `int (int, int)` |
| 4 | `shift_with_static` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn shift_with_static` | `int (int, int)` |
| 5 | `get_operation` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn get_operation` | `operation_func (int)` |
| 6 | `execute_operation` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn execute_operation` | `int (operation_func, int, int, const char*)` |
| 7 | `compute_checksum` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn compute_checksum` | `unsigned int (int*, int)` |
| 8 | `init_state` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn init_state` | `void (ComputeState*, int)` |
| 9 | `apply_operation` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn apply_operation` | `void (ComputeState*, int, operation_func)` |
| 10 | `checkshift` (T) | YES | `#[no_mangle] pub unsafe extern "C" fn checkshift` | `int (int, int, int, int)` |

Non-exported C internals that therefore need no Rust export:

| C entity | Kind | Note |
|----------|------|------|
| `static_multiplier`, `static_addend`, `static_shift_amount` | file-static `int` | never mutated; modelled as Rust `const` |
| `ops[4]` inside `get_operation` | function-local `static` | lazily-initialised table; observable behaviour is the fixed mapping |
| `STRINGIFY`, `LOG_VALUE`, `OP_*`, `MAGIC_NUMBER`, `MASK_LOWER` | preprocessor macros | no symbols emitted |

## Symbol diff (Phase D gate)

`comm` of the two sorted symbol-name lists:

- C-only (missing from Rust): **0**
- Rust-only extra `T`/`D` symbols in the crate's own namespace: **0**
- Undefined (`U`) symbols in the Rust `.so`: libc/loader only
  (`printf`, `malloc`, `free`, `memcpy`, `__stack_chk_fail`,
  `_Unwind_Resume`, `rust_eh_personality`-class runtime entries).
  No non-libc undefined symbol.

Gate status: **PASS** — the symbol diff is empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one. `--no-default-features` and the
default build are byte-identical in terms of code paths; the test suite is run
under both to confirm.
