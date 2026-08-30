# VERIFICATION.md — result of the A→D differential verification

Library: `c_src/src/driver.c` (MIT Lincoln Laboratory CWE-369 divide-by-zero
test case) vs. the Rust translation in `src/lib.rs`.

## Completion gate

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** — symbol diff is empty (5/5) |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | **PASS** — 28/28 rows, 28 tests |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | **PASS** — 13/13 rows, 16 tests |
| All of the above under every feature combination | **PASS** — see matrix below |

48 tests total: 28 (Phase B) + 16 (Phase C) + 4 (Phase D).

## Feature / profile matrix

`Cargo.toml` declares **no `[features]`**, and `src/lib.rs` contains no
`#[cfg]`/`feature =` gates, so there is exactly one feature configuration.
Verified anyway across all flag spellings and both profiles:

| invocation | result |
|------------|--------|
| `cargo test` (debug, default) | 48 passed, 0 failed |
| `cargo test --no-default-features` | 48 passed, 0 failed |
| `cargo test --all-features` | 48 passed, 0 failed |
| `cargo test --release` | 48 passed, 0 failed |
| `cargo test --release --no-default-features` | 48 passed, 0 failed |
| `cargo test --release --all-features` | 48 passed, 0 failed |

## Behaviours confirmed identical to the C

- `printLine(NULL)` is a silent no-op; any other pointer prints `<bytes>\n`
  verbatim, including non-UTF-8 bytes and `printf` conversion specifiers (the
  argument is never used as a format string).
- `printIntLine` matches `%d` across the entire 32-bit domain.
- `bad(data)` computes `(int)(100.0 / data)` in **double** precision (the `100.0`
  literal is a `double`, so the `float` operand is promoted) and reproduces the
  x86-64 `cvttsd2si` "integer indefinite" result `-2147483648` for NaN and for
  every quotient outside `int` range — including `data == ±0.0` (the CWE-369
  path) and tiny/subnormal `data`. `data == ±inf` yields `0`, not an error.
- `good(data)` always prints `50` from `goodG2B`, then takes `goodB2G`'s
  threshold branch on `fabs((double)data) > 0.000001`.
- `driver` emits the exact 6-line transcript in the exact order.

### Subtlety worth recording

`goodB2G` compares against the **double** literal `0.000001`, while `data` is a
`float`. The `float` nearest `1e-6` is `9.99999997475e-07`, which is *below*
`1e-6`, so `good(1e-6f)` takes the **rejection** branch; one ULP up crosses the
threshold and prints `99999988`. Both implementations agree (`ERRORS.md` E12,
`CONFIGS.md` C19).

## Negative control: mutation testing

Passing tests prove nothing unless the harness can actually *detect* divergence,
so 20 deliberate bugs were injected into `src/lib.rs` and the suite re-run.

**16 of 16 non-equivalent mutants were caught. 0 real bugs escaped.**

| mutant | verdict |
|--------|---------|
| M1 saturating cast instead of `INT_MIN` indefinite | caught (15 tests) |
| M2 drop the `NaN → INT_MIN` case | caught (7) |
| M3 `round` instead of `trunc` | caught (16) |
| M4 upper overflow bound off-by-one (`>=` → `>`) | caught (4) |
| M6 drop the `NULL` check in `printLine` | caught (2) |
| M7 invert the `NULL` check | caught (26) |
| M8 `printLine` drops the newline | caught (25) |
| M9 `printIntLine` uses `%u` instead of `%d` | caught (26) |
| M10 one-character typo in the divide-by-zero message | caught (13) |
| M11 `goodG2B` constant `2.0` → `4.0` | caught (18) |
| M12 swap `goodG2B` / `goodB2G` order | caught (15) |
| M13 `driver` omits the `Finished good()` line | caught (9) |
| M14 `driver` swaps the `good()` / `bad()` arguments | caught (8) |
| M15 `driver` calls `bad` before `good` | caught (9) |
| M16 drop `fabs` in the threshold test | caught (10) |
| M19 remove `#[no_mangle]` from `bad` | caught (Phase D, 2) |
| M20 rename the exported `good` symbol | rejected: does not compile |
| M5 lower bound `<` → `<=` | escaped — **proven equivalent** |
| M17 `f32` threshold instead of `f64` | escaped — **proven equivalent** |
| M18 threshold `>` → `>=` | escaped — **proven equivalent** |

The three escapes are *equivalent mutants*, not coverage gaps. Each was proven
indistinguishable by exhaustive enumeration of **all 2^32 `f32` bit patterns**
(0 differing inputs):

- **M5**: when `truncated == -2147483648.0` exactly, the original falls through
  to `as c_int`, which yields `INT_MIN` — the same value the mutant returns early.
- **M17 / M18**: the `f32` nearest `1e-6` is `9.99999997475e-07` and the next
  `f32` up is `1.00000003e-06`. No `f32` lands in `[9.99999997475e-07, 1e-6]`,
  so neither the comparison precision nor `>` vs `>=` is observable.

## Two harness defects the negative control exposed

Both would have made the entire suite pass **vacuously**, and neither was
visible from green test output — this is why the mutation step was run:

1. `ensure_rust_so()` originally returned early if `libdriver.so` already
   existed. `cargo test` builds the crate as an *rlib* for the test binaries and
   never refreshes the `cdylib`, so edits to `src/lib.rs` were tested against a
   stale artifact.
2. After forcing a rebuild, the inner `cargo build` still shared the target
   directory with the outer `cargo test`. Cargo's fingerprints are mtime-based,
   and when `src/lib.rs` was rewritten in the same wall-clock second as the
   previous build, cargo judged the crate fresh and left the old `.so` in place.

Fix: the cdylib is now built into a dedicated, wiped `target/difftest-cdylib`
directory on first use, with an explicit assertion that the artifact is not
older than `src/lib.rs`.

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo test -- --test-threads=1
```

`--test-threads=1` is recommended because the harness captures output by
`dup2`-ing over fd 1, which is process-global; the harness serializes captures
with a mutex, so parallel runs are correct but slower.

Both `.so`s are loaded with `libloading` and every call crosses the FFI boundary
through the exported C ABI symbols — no Rust function is ever called directly,
so the `#[no_mangle]` export wrappers are themselves under test.
