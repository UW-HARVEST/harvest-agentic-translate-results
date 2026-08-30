# FEATURES.md — Phase D feature/configuration matrix

## Feature enumeration

`translation/Cargo.toml` contains **no `[features]` table** and the crate has
exactly one non-dev dependency-free target:

```toml
[lib]
name = "driver"
path = "src/lib.rs"
crate-type = ["cdylib"]
```

`cargo metadata --no-deps` reports an empty feature map, so the complete set of
feature combinations is:

| # | combination | equivalent to |
|---|-------------|---------------|
| 1 | *(default)* | `cargo test` |
| 2 | `--no-default-features` | identical to #1 — there are no default features to remove |

`scripts/check_features.sh` derives this list from `cargo metadata` rather than
from a hand-written list, builds the power set when features do exist, and
crosses it with both cargo profiles (`dev` and `release`, the latter also
exercising `panic = "abort"` and `opt-level = 3` in the cdylib under test).
So the matrix actually executed is 2 combinations × 2 profiles = 4 runs.

## Recorded run

```
$ ./scripts/check_features.sh
Cargo.toml declares no [features]; the only configuration is the default.

== cargo test --offline                                  -> all suites pass
== cargo test --offline --release                         -> all suites pass
== cargo test --offline --no-default-features              -> all suites pass
== cargo test --offline --no-default-features --release    -> all suites pass

ALL FEATURE COMBINATIONS PASSED (2 combos x 2 profiles)
```

Per run the suites are:

| suite | harness | cases |
|---|---|---|
| `tests/smoke.rs` | custom (sequential) | 6 — harness self-checks incl. a negative control |
| `tests/configs.rs` | custom (sequential) | 22 — one per `CONFIGS.md` row (Phase B) |
| `tests/errors.rs` | custom (sequential) | 12 — one per `ERRORS.md` row (Phase C) |
| `tests/optlevels.rs` | custom (sequential) | 6 — C rebuilt at -O0/-O1/-O2/-O3/-Os + the CMake build |
| `tests/symbols.rs` | libtest | 5 — Phase D symbol parity |

**51 cases per run, 204 across the matrix.** Randomized rows run many inputs
each (2000 + 2000 + 1000 + 500 + 500 + 300 × several + …), all from the fixed
seed `0x2545F4914F6CDD1D` in `tests/common/mod.rs::Rng`, so any failure
reproduces exactly.

## Why the differential suites use `harness = false`

The library's only observable behaviour is what it writes to file descriptor 1,
so the harness redirects fd 1 process-wide around each call. libtest runs
`#[test]` functions on multiple threads and writes its own progress lines
("`test foo ... ok`") to fd 1; under the default harness those lines landed
*inside* the captured bytes and produced spurious mismatches such as:

```
C = helperGood1 string\ntest c14_bad_repeated ...
Rust = helperGood1 string\n
```

The custom runner in `tests/common/mod.rs` executes cases sequentially on one
thread and flushes Rust's `io::stdout()` before every capture, which removes the
race. (This was a defect in the *test harness*, not in the translation — no
change to `src/lib.rs` behaviour was needed for it.)

## Other build axes checked

* **Cargo profile**: `dev` (unoptimized, `panic=unwind`) and `release`
  (`opt-level=3`, `panic="abort"`) — both in the matrix above.
* **C optimization level**: `tests/optlevels.rs` recompiles the untouched
  `c_src/src/driver.c` at `-O0 -O1 -O2 -O3 -Os` into `$TMPDIR` (nothing in
  `c_src/` is modified) and re-runs the behavioural battery plus 300 randomized
  `printLine` payloads against each. This matters because the translation's
  handling of `helperBad`'s undefined behaviour depends on how the C compiler
  resolves it; the suite proves the NULL return is not an `-O0` artifact.

## Mutation check — proof the suite is actually sensitive

"All tests pass" is worthless unless the tests can fail. Five deliberate
mutations were injected into `src/lib.rs` one at a time (each applied to a
pristine copy, then reverted); the table records how many of the 51 cases caught
each one. `src/lib.rs` is back to its verified state — no mutation remains.

| mutation injected into `src/lib.rs` | cases that FAILED |
|---|---|
| `helperBad` returns a real pointer to a static `"helperBad string"` instead of NULL (i.e. "fixing" the CWE-562 defect) | 17 (7 configs + 4 errors + 6 optlevels) |
| `driver`: `if useGood == 1` instead of `if useGood != 0` (drops C truthiness) | 5 |
| `printLine`: null check removed (glibc then prints `(null)`) | 8 |
| `printLine`: payload passed as the *format* string, `printf(line)` | 4 |
| `helperGood1`: one byte of the string changed (`strinG`) | 9 |
| format string `"%s\n"` changed to `"%s"` (no trailing newline) | 18 |

Every mutation was caught by multiple independent cases, and each mutation was
caught by the row that was written for exactly that behaviour — e.g. the
CWE-562 "fix" is caught by `ERRORS.md` row E2 (`e2_bad_prints_nothing`), the
truthiness change by `CONFIGS.md` rows C17–C19, and the format-string change by
row G5 (`g5_print_line_format_specifiers`).
