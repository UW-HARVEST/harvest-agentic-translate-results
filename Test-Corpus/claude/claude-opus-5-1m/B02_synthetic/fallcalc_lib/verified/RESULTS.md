# RESULTS.md — verification record

Driver: `./verify.sh` (enumerates feature combos, builds the C reference `.so`,
builds the Rust `cdylib`, diffs `nm -D`, runs Phases B/C/D per combo per profile).

Toolchain: `rustc 1.94.0`, `cargo 1.94.0`, `gcc 11.5.0`, `cmake 3.22.2`,
x86_64-unknown-linux-gnu.

## How to reproduce

```bash
# everything (feature enumeration + C build + symbol diff + Phases B/C/D, both profiles)
./verify.sh

# or just the tests (the harness builds the C .so prerequisite itself is NOT done here,
# but it does auto-build the Rust cdylib into target/cdylib-for-tests if missing)
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..
cargo test --offline --no-default-features
cargo test --offline --no-default-features --release

# extra multi-million-case sweeps
cargo test --offline --release --test phase_e_fuzz -- --ignored --nocapture
```

Two environment overrides let the harness be pointed at alternative binaries
without touching `c_src/`:

| variable | effect |
|----------|--------|
| `FALLCALC_C_SO`    | path to the reference C `.so` (used for the `-O2` cross-check and for mutation testing) |
| `FALLCALC_RUST_SO` | path to the Rust `.so` under test |

## Phase A — surface artifacts

| artifact | contents |
|----------|----------|
| `SYMBOLS.md` | 6 C exports, all 6 present in the Rust `.so` |
| `ERRORS.md`  | 24 rejection/error rows |
| `CONFIGS.md` | 1 feature combination, 37 configuration rows |

## Phase A/D — feature combinations

`Cargo.toml` has **no `[features]` section** ⇒ exactly one valid combination.

```
=== Enumerating [features] from Cargo.toml ===
No [features] section -> exactly one valid feature combination: <none>
Combination count: 1

=== cargo check --no-default-features --features '<none>' ===
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
```

`cargo check --offline --no-default-features --all-targets` and
`cargo check --offline --all-features --all-targets`: **0 errors, 0 warnings.**

## Phase D — symbol parity (`nm -D`)

```
C exports (API): 6   Rust exports (all): 6
symbol diff EMPTY -- all 6 C API symbols exported by Rust: OK
allocate_and_compute fallcalc foreach_sum process_array_reverse
safe_double_to_int switch_fallthrough_calculator
```

Holds in **both** profiles (`target/debug/libfallcalc_lib.so` and
`target/release/libfallcalc_lib.so`). `tests/phase_d_symbols.rs` re-derives the
diff at test time (5 tests).

## Phases B & C — differential test results

Every test loads BOTH `.so` files with `libloading` and calls only exported
symbols; no Rust function is ever called directly.

### profile = debug, features = `<none>`

```
tests/phase_b_valid.rs   : 37 passed; 0 failed   (CONFIGS.md rows 1-37)
tests/phase_c_errors.rs  : 24 passed; 0 failed   (ERRORS.md rows 1-24)
tests/phase_d_symbols.rs :  5 passed; 0 failed
tests/phase_e_fuzz.rs    :  5 ignored (long sweeps, run explicitly)
```

### profile = release, features = `<none>` (`opt-level=3`, `panic="abort"`)

```
tests/phase_b_valid.rs   : 37 passed; 0 failed
tests/phase_c_errors.rs  : 24 passed; 0 failed
tests/phase_d_symbols.rs :  5 passed; 0 failed
tests/phase_e_fuzz.rs    :  5 ignored
```

`./verify.sh` final line: **`ALL CONFIGURATIONS PASS`** (exit 0).

## Extra assurance 1 — optimised C reference

The C library was additionally built out-of-source with `-DCMAKE_BUILD_TYPE=Release`
(into a scratch dir, `c_src/` untouched) and the whole suite re-run against it via
the `FALLCALC_C_SO` override:

```
FALLCALC_C_SO=<scratch>/cbuild_o2/libtranslated_rust.so cargo test
  phase_b_valid  : 37 passed; 0 failed
  phase_c_errors : 24 passed; 0 failed
  phase_d_symbols:  5 passed; 0 failed
```

The Rust translation matches the C at both `-O0` (the default/reference build)
and `-O2`, i.e. it does not depend on a particular compiler interpretation of
the signed-overflow UB in `lib.c`.

## Extra assurance 2 — long randomised sweeps (`tests/phase_e_fuzz.rs`)

```
cargo test --release --test phase_e_fuzz -- --ignored --nocapture
fuzz_switch_exhaustive_operations: 800,200 cases compared      ok
fuzz_array_apis:                    40,000 cases compared      ok
fuzz_safe_double_to_int_millions: 4,000,000 cases compared     ok
fuzz_allocate_and_compute:          40,000 cases compared      ok
fuzz_fallcalc_millions:          2,000,000 cases compared      ok
test result: ok. 5 passed; 0 failed
```

≈6.9 million additional differential comparisons, all identical.

## Extra assurance 3 — mutation testing of the test suite itself

To prove the differential tests actually discriminate (rather than passing
vacuously), 14 behaviour-changing mutants of `lib.c` were compiled in a scratch
directory (`c_src/` untouched) and the suite was run against each as the
"reference". **14 / 14 were detected:**

| mutant | mutation | failing tests |
|--------|----------|---------------|
| m03 | `isnan(d)` returns `1` instead of `0` | 8 DETECTED |
| m04 | `points == NULL` returns `-2` instead of `-1` | 17 DETECTED |
| m05 | `process_array_reverse` seeds `sum = 1` | 27 DETECTED |
| m06 | `FOREACH(..., count - 1)` off-by-one | 21 DETECTED |
| m07 | `param3 >= OCTAL_FLAG` instead of `>` | 8 DETECTED |
| m08 | `points[i].value = i*010 + 1` | 17 DETECTED |
| m09 | `allocate_and_compute(param4 % 10 + 2, …)` | 16 DETECTED |
| m10 | `data_array[i] = i*010 + param1` (drops `+1`) | 17 DETECTED |
| m11 | `isinf` sign branches swapped | 8 DETECTED |
| m12 | final mask `OCTAL_MASK_2` instead of `OCTAL_MASK_1` | 17 DETECTED |
| m13 | `case 2:` loses `result &= OCTAL_MASK_1` | 7 DETECTED |
| m14 | `default: result = 1` instead of `0` | 18 DETECTED |
| m15 | `coefficient = (double)i` (drops `* multiplier`) | 19 DETECTED |
| m16 | `param2 * 2.4` instead of `* 2.3` | 17 DETECTED |

One further mutant (`d > (double)INT_MAX` instead of `>=`) was **not** detected,
and was confirmed by inspection to be a *semantically equivalent* mutant:
`(int)2147483647.0` is exactly `INT_MAX`, so both spellings return the same
value for the only input that distinguishes them.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and **0** undefined
      non-libc symbols in the Rust `.so`.
- [x] Phase B: **all 37** `CONFIGS.md` rows pass across randomised inputs.
- [x] Phase C: **all 24** `ERRORS.md` rows have a passing error-path
      differential test (same error code / sentinel, not merely "both failed").
- [x] All of the above hold under **every** feature combination (the single
      valid combination) and under both the `debug` and `release` profiles.
- [x] No divergence found; **no changes to the Rust source were required** —
      `src/lib.rs` already reproduced the C byte-for-byte on every input tested.
- [x] `c_src/` was not modified (verified with `git status` / file mtimes;
      only the documented `c_src/build/` output directory was created).
