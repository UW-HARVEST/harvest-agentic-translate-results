# SYMBOLS.md — Phase A symbol map

C `.so`:    `c_src/build/libharvest-work-TpapYk.so`
Rust `.so`: `translation/target/release/libhatch_lib.so`

Source of truth: `nm -D --defined-only` on the C `.so`, filtered to `T` (global text).
The C source has exactly one translation unit (`c_src/src/lib.c`); the only file-scope
statics are `global_counter` / `global_accumulator`, which are `static` and therefore
NOT exported (they appear in neither `.so`'s dynamic symbol table — correct).

| # | C symbol (`nm -D`, type T) | Exported by Rust `.so`? | Rust item |
|---|----------------------------|-------------------------|-----------|
| 1 | `add_three`                  | YES | `add_three` |
| 2 | `apply_operation`            | YES | `apply_operation` |
| 3 | `complex_calc`               | YES | `complex_calc` |
| 4 | `compute_with_dynamic_memory`| YES | `compute_with_dynamic_memory` |
| 5 | `get_time_based_value`       | YES | `get_time_based_value` |
| 6 | `hatch`                      | YES | `hatch` |
| 7 | `increment_counter`          | YES | `increment_counter` |
| 8 | `manipulate_records`         | YES | `manipulate_records` |
| 9 | `multiply_add`               | YES | `multiply_add` |
| 10 | `process_pointer_data`      | YES | `process_pointer_data` |
| 11 | `shift_array_data`          | YES | `shift_array_data` |
| 12 | `update_accumulator`        | YES | `update_accumulator` |

## Diff result

```
$ comm -23 c_syms.txt rust_syms.txt   # in C, missing from Rust
(empty)
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`** (its only
undefined imports are `malloc`, `free`, `memmove`, `memset`, `time`, `difftime`,
`snprintf` from libc, plus the Rust runtime's own libc/`__cxa`/unwind imports).

No C module was skipped: `lib.c` is the entire library and every function in it
(including the non-header-declared ones) has a real, non-stub Rust translation.
`DataRecord` is a type, not a symbol (`sizeof == 48`, `_Alignof == 8`, verified
against C; matched by the Rust `#[repr(C)]` struct).

Verified automatically by `tests/symbols.rs::symbol_parity`.

## How to reproduce

```bash
# 1. C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust shared library (the tests dlopen target/release/libhatch_lib.so)
cd translation && cargo build --release --offline

# 3. Differential suite (all phases) — `--offline` because crates.io is unreachable
#    here and libloading 0.8.9 is already in the local registry cache.
cargo test --offline

# 4. All feature combinations (the crate declares none, so: default + --no-default-features)
./run_all_features.sh
```
