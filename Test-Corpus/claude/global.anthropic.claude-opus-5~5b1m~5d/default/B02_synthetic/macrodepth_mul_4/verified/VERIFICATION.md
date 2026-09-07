# VERIFICATION.md — completion gate

Reproduce everything from the repository root:

```bash
./build_c.sh      # C .so + driver for all 24 (OP, REPEAT) configs -> cbuild/
./build_rust.sh   # Rust cdylib + driver for all 24 configs        -> rustbuild/
./check_all.sh    # cargo check --all-targets for every feature combo
./symdiff.sh      # nm -D parity, C .so vs Rust .so, all 24 configs
./run_tests.sh    # Phases B+C for every feature combo (rebuilds the .so first)
```

Nothing under `c_src/` is modified; `build_c.sh` only reads `c_src/src/*.c`
and writes to `cbuild/`.

## Gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      `./symdiff.sh` → `symbol parity OK for all 24 configs (0 missing, 0 unexpected undefined)`.
      All 8 C exports (`op_add`, `op_sub`, `op_mul`, `G_OP`, `G_OP_NAME`,
      `helper_call`, `helper_ptr`, `use_generated`) are exported by the Rust
      `cdylib` with identical names, `nm` types and object sizes. Nothing was
      stubbed: `mdcore.c` and `mdmain.c` are both fully translated
      (`src/mdcore.rs`, `src/main.rs`) and `mdmacros.h` is translated into
      `src/mdconfig.rs` + Cargo features.
- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      168 rows (24 builds × 7 entry-point/input-shape groups) + row 0, driven by
      `tests/phase_b_valid.rs` with a fixed-seed SplitMix64 generator
      (boundary set + 512/256/128 random inputs per row) and byte-for-byte
      stdout comparison. Every call goes through `dlsym` into both `.so`s.
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential test.**
      25 rows / 23 tests in `tests/phase_c_errors.rs` (rows 18–19 are build-time
      and are covered by `check_all.sh` + `configs_00_build_matrix_matches`).
- [x] **All of the above hold under EVERY feature combination.**
      `./check_all.sh` → 26/26 combos compile with 0 warnings.
      `./run_tests.sh` → `combos passed=26 failed=0`, 32 tests each
      (24 real `(OP, REPEAT)` combos + no-feature and all-feature degenerate
      combos).

## Divergence found and fixed

| # | symptom | root cause | fix |
|---|---------|-----------|-----|
| 1 | The Rust `.so` placed `G_OP` and `G_OP_NAME` in `.data.rel.ro`, which full RELRO maps read-only; a store through either exported symbol (legal C — both are mutable objects in the C `.so`'s `.data`) would have faulted. | `#[no_mangle] pub static` is an immutable Rust static. | `#[no_mangle] pub static mut` in `src/mdcore.rs` (+ `g_op()` / `g_op_name()` accessors used by `main.rs`); both objects are now in `.data` like the C ones. Regression-tested by `err_16_g_op_is_writable` / `err_17_g_op_name_is_writable`. |

## Test-suite self-validation (mutation checks)

To confirm the differential tests can actually fail, two deliberate mutations
were injected into `src/mdconfig.rs` and then reverted:

| mutation | detected by |
|----------|-------------|
| `dispatch_rep` accepting `0..=7` instead of `0..=6` (i.e. adding a `case 7:` the C `switch` does not have) | `b_06_use_generated`, `b_07_driver_pipeline`, `b_08_printf_text_oracle`, `err_04_dispatch_n_seven` — for `add,5` **and** `mul,7` |
| off-by-one in `run_loop` (`while i <= REPEAT`) | `b_05_helper_call`, `b_07_driver_pipeline`, `b_08_printf_text_oracle` for `add,5` (invisible for `sub,0`, where the extra `acc -= 0` step is genuinely a no-op) |

Both mutations were reverted and the full matrix re-run green afterwards.
