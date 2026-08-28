# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-yo8lXq.so`

Rust library:
`target/release/libfallcalc_lib.so`

The table is derived from `nm -D --defined-only` on the reference library.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `allocate_and_compute` | `T` | `allocate_and_compute` | present |
| 2 | `fallcalc` | `T` | `fallcalc` | present |
| 3 | `foreach_sum` | `T` | `foreach_sum` | present |
| 4 | `process_array_reverse` | `T` | `process_array_reverse` | present |
| 5 | `safe_double_to_int` | `T` | `safe_double_to_int` | present |
| 6 | `switch_fallthrough_calculator` | `T` | `switch_fallthrough_calculator` | present |

Missing defined symbols: **0**

The reference library's undefined dynamic symbols are `free` and `malloc`
(libc), plus the weak toolchain symbols `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. They are
not library API exports.

## Completion Gate

- [x] Every defined C dynamic symbol is defined by the Rust cdylib.
- [x] The symbol diff has zero missing and zero extra API exports.
- [x] Differential tests pass with default features and
  `--no-default-features` (the manifest declares no named features).
