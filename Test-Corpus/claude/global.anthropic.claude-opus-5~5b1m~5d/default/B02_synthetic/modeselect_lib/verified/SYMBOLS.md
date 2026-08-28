# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from:

```
nm -D c_src/build/libharvest-work-lpPs9a.so          # C ground truth
nm -D translation/target/release/libmodeselect_lib.so # Rust translation
```

## C translation units

`c_src/` contains exactly one translation unit, `src/lib.c` (140 lines), plus the
public header `include/lib.h` (which declares only `modeselect`).  There is **no
untranslated module**: every function defined in `lib.c` has a counterpart in
`translation/src/lib.rs`.

| C definition (`c_src/src/lib.c`) | line | Rust definition (`translation/src/lib.rs`) | line |
|---|---|---|---|
| `int classify_mode(const char *mode)`               | 29  | `classify_mode`               | 98  |
| `int apply_multiplier(int base, int level)`         | 42  | `apply_multiplier`            | 121 |
| `int convert_time_factor(double factor)`            | 65  | `convert_time_factor`         | 163 |
| `int convert_negative_overflow(double value)`       | 72  | `convert_negative_overflow`   | 175 |
| `time_t get_modified_time(int, int)`                | 79  | `get_modified_time`           | 187 |
| `int hash_time_value(time_t t)`                     | 86  | `hash_time_value`             | 204 |
| `int modeselect(int, int, int, int)`                | 99  | `modeselect`                  | 224 |

No macro-generated symbols, no `static`/internal-linkage helpers, no global
data objects, no weak aliases, no versioned symbols.

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `apply_multiplier`          | `T` | `T` | OK |
| 2 | `classify_mode`             | `T` | `T` | OK |
| 3 | `convert_negative_overflow` | `T` | `T` | OK |
| 4 | `convert_time_factor`       | `T` | `T` | OK |
| 5 | `get_modified_time`         | `T` | `T` | OK |
| 6 | `hash_time_value`           | `T` | `T` | OK |
| 7 | `modeselect`                | `T` | `T` | OK |

**Missing from Rust `.so`: 0.**  Symbol diff (`comm -23`) is empty — verified by
`tests/differential.rs::phase_d_symbol_parity`, which shells out to `nm -D` on
both objects at test time and asserts the difference is empty.

## Weak / undefined symbols

Both objects import only libc (and, for Rust, the unwinder + Rust-runtime libc
calls).  These are not part of the library's own surface:

* C `U`: `printf@GLIBC_2.2.5`, `strcmp@GLIBC_2.2.5`, `time@GLIBC_2.2.5`
* C `w`: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
  `__cxa_finalize`, `__gmon_start__`
* Rust `U`/`w`: the same three libc functions (`printf`, `strcmp`, `time` — the
  translation deliberately calls the *identical* libc entry points so formatting
  and comparison semantics cannot drift), plus `_Unwind_*`, `malloc`/`free`,
  `memcpy`, `mmap64`, `pthread_key_*`, etc. from the Rust standard library.
  All resolve inside `libc.so.6` / `libgcc_s.so.1`; there are **0 unresolved
  non-libc symbols**.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
complete set of feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | *(default = empty)* | `cargo test` |
| 2 | *(no-default-features = empty, identical)* | `cargo test --no-default-features` |
| 3 | *(all-features = empty, identical)* | `cargo test --all-features` |

`tests/` is additionally run against both the `dev` and `release` profiles,
because `[profile.release] panic = "abort"` and optimisation level are the only
build-configuration axes that exist in this crate.
