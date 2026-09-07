# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-cp21cR.so`

Collected mechanically with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-cp21cR.so
nm -D --defined-only target/release/libfallcalc_lib.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `allocate_and_compute` | `T` | `allocate_and_compute` | present |
| `fallcalc` | `T` | `fallcalc` | present |
| `foreach_sum` | `T` | `foreach_sum` | present |
| `process_array_reverse` | `T` | `process_array_reverse` | present |
| `safe_double_to_int` | `T` | `safe_double_to_int` | present |
| `switch_fallthrough_calculator` | `T` | `switch_fallthrough_calculator` | present |

Missing defined C symbols in Rust: **0**

The C library's undefined dynamic references are only `malloc` and `free`
(libc), plus the usual weak toolchain hooks. The Rust cdylib has additional
Rust runtime/libc/libgcc dependencies, but no undefined project-library
symbols.

## Completion gate

- [x] Defined dynamic-symbol diff is empty.
- [x] All 162 `CONFIGS.md` rows pass randomized differential tests.
- [x] The project has no C or Rust executable target to compare.
- [x] All 17 `ERRORS.md` rows pass differential tests.
- [x] Default and `--no-default-features` builds both pass; `Cargo.toml`
  declares no named features.
