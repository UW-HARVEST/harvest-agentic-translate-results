# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-H0AFEq.so
nm -D --defined-only translation/target/release/libmodeselect_lib.so
```

The C project builds a single translation unit (`c_src/src/lib.c`); there is no
driver binary (`CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)`).
`c_src/include/lib.h` declares only `modeselect`, but the `.so` exports every
non-`static` function in the TU, so all seven are part of the ABI surface and all
seven are verified.

## Exported (dynamic, defined) symbols

| # | symbol | C signature | in C `.so` | in Rust `.so` | status |
|---|--------|-------------|-----------|---------------|--------|
| 1 | `classify_mode`            | `int classify_mode(const char *mode)`                  | T | T | OK |
| 2 | `apply_multiplier`         | `int apply_multiplier(int base, int level)`             | T | T | OK |
| 3 | `convert_time_factor`      | `int convert_time_factor(double factor)`               | T | T | OK |
| 4 | `convert_negative_overflow`| `int convert_negative_overflow(double value)`          | T | T | OK |
| 5 | `get_modified_time`        | `time_t get_modified_time(int days, int hours)`        | T | T | OK |
| 6 | `hash_time_value`          | `int hash_time_value(time_t t)`                        | T | T | OK |
| 7 | `modeselect`               | `int modeselect(int, int, int, int)`                   | T | T | OK |

**Missing from Rust: none.** No macro-generated symbols exist in the C source
(no function-like macros define exported names), so nothing else is expected.

## Undefined (imported) symbols in the Rust `.so`

The Rust `.so` imports the same three libc functions the C `.so` does
(`printf@GLIBC_2.2.5`, `strcmp@GLIBC_2.2.5`, `time@GLIBC_2.2.5`) — deliberately,
so `printf` formatting (`%.2e` rounding, `%X` casing) and `strcmp` semantics come
from the identical code path. All remaining undefined symbols are libc / libgcc
unwinder / Rust-runtime imports (`malloc`, `memcpy`, `_Unwind_*`, `abort`, …).

**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one:

```
cargo test                                # default (== all features)
cargo test --no-default-features          # identical: no features exist
```

Both are exercised by `run_all.sh`.
