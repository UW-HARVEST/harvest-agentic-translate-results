# FEATURE_MATRIX.md — Phase D results

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one feature configuration. The only orthogonal build axis that changes
generated code is the *profile* (`overflow-checks` / `debug-assertions` /
`opt-level`), so the matrix is `{default, --no-default-features} × {dev,
release}` — all four are built and fully tested by
`translation/scripts/run_verification.sh`.

Run it with:

```
translation/scripts/run_verification.sh
```

It rebuilds the C `.so`, then for each configuration: builds the Rust cdylib,
diffs `nm -D` against the C `.so` (both directions, plus a check for undefined
`stbds_*` references), and runs the whole differential suite with
`--test-threads=1`.

## Latest run

| profile | features | `nm -D` diff | tests |
|---------|----------|--------------|-------|
| dev | `--no-default-features` | empty (16 symbols) | 77 passed, 0 failed |
| dev | default | empty (16 symbols) | 77 passed, 0 failed |
| release | `--no-default-features` | empty (16 symbols) | 77 passed, 0 failed |
| release | default | empty (16 symbols) | 77 passed, 0 failed |

`ALL CONFIGURATIONS PASS`

Test-binary breakdown (identical in every configuration):

| test binary | tests | covers |
|-------------|-------|--------|
| `phase_b_hash` | 7 | CONFIGS rows 1–7 |
| `phase_b_arr_arena` | 8 | CONFIGS rows 8–15 |
| `phase_b_map` | 24 | CONFIGS rows 16–38, 42 |
| `phase_b_driver` | 4 | CONFIGS rows 39–41 |
| `phase_c_errors` | 31 | ERRORS rows 1–23, 27–29, 33–39, 42, 43, 45 |
| `phase_c_crashes` | 2 | ERRORS rows 26, 30–32, 40, 41, 44 (8 crash-equivalence scenarios × 2 impls) |
| `smoke` | 1 | both `.so`s dlopen and resolve all 16 symbols |
| lib unit tests | 0 | (the crate has no `#[cfg(test)]` code) |

## Note on `[profile.*]`

`debug-assertions = false` had to be added to `[profile.dev]` during Phase C.
With debug assertions on, rustc instruments raw-pointer stores and turns the C's
"store through a NULL pointer" behaviour into a `null pointer dereference` panic
(→ `SIGABRT`), while the C takes `SIGSEGV`. This was caught by
`crash_equivalence_all_scenarios` / scenario `hmget_ts_null_temp` (ERRORS row 41)
and is now identical in every profile. This mirrors the pre-existing
`overflow-checks = false`, which the translation already relies on to reproduce
the C's wrap-around `size_t` arithmetic.
