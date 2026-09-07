# VERIFICATION.md — completion gate

Reproduce everything with `./verify.sh` (phases A–D) and `./mutate.sh`
(negative control).

## Completion gate

- [x] **`SYMBOLS.md`**: `nm -D` shows **0 missing** and 0 unresolved non-libc
      symbols in the Rust `.so`. The C `.so` exports 5 symbols; the Rust `.so`
      exports the same 5, with identical names. `ldd -r` reports 0 unresolved
      symbols for both the debug and the release artifact. No stubs were used;
      the whole of `c_src` (2 files) was already translated.
- [x] **Phase B**: all **27 rows** of `CONFIGS.md` pass, across randomized
      inputs (fixed seeds; ~2000 random `f32` bit patterns for `bad`, ~2000 for
      `driver` pairs, plus exhaustive single-byte and per-exponent sweeps).
- [x] **Binary**: not applicable — `c_src/CMakeLists.txt` has no
      `add_executable` and `Cargo.toml` has no `[[bin]]`. Equivalent end-to-end
      coverage is `CONFIGS.md` rows 26–27, which drive the top-level `driver`
      entry point and mixed multi-call sessions through the `.so`.
- [x] **Phase C**: all **28 rows** of `ERRORS.md` have a passing differential
      test that pins the exact expected bytes (not merely "both failed").
- [x] **Every feature combination**: `Cargo.toml` declares no `[features]`
      table, so the only configurations are the default and
      `--no-default-features`. Both were run, against **both** the debug and the
      release (`panic = "abort"`, opt-level 3) `.so` — 4 combinations total,
      60/60 tests green in each.

## Result

**No divergences were found. `src/lib.rs` required no changes** — its md5 is
identical before and after verification (`7b90d765989aa12f0ad5994b6b63ddb6`).
The translation is byte-exact against the C on every input exercised.

The pre-existing translation was already correct on the three points most
likely to diverge, and the tests confirm each:

1. **`(int)` conversion of an out-of-range / NaN double.** The C relies on
   x86-64 `cvttsd2si`, which yields the "integer indefinite" value `INT_MIN`.
   Rust's `as` **saturates** instead, so a naive translation would print
   `2147483647` where C prints `-2147483648`. `c_double_to_int` emulates the
   hardware; mutant M1 proves the tests catch the saturating version.
2. **Division precision.** The C divides the `double` literal `100.0` by the
   widened `float`, so the quotient carries f64 precision. Rounding it to f32
   changes the truncated integer for ~2.7% of all `f32` inputs (mutant M8).
3. **NaN and the `fabs(data) > 0.000001` guard.** Because every `>` comparison
   against NaN is false, `good(NaN)` takes the *else* branch and prints the
   divide-by-zero message rather than dividing. Covered by `ERRORS.md` row 4.

## Test inventory

| file | tests | scope |
|------|-------|-------|
| `tests/common/mod.rs` | — | harness: dual `dlopen`, fd-1 stdout capture, PRNG, staleness guard |
| `tests/smoke.rs` | 4 | harness self-check + hand-verified `driver(2.0, 4.0)` output |
| `tests/phase_b_configs.rs` | 27 | one test per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 29 | one test per `ERRORS.md` row + a harness sanity check |
| **total** | **60** | |

Both libraries are loaded via `libloading` and every call crosses the FFI
boundary through an exported symbol, so the `#[no_mangle] extern "C"` wrappers
are themselves under test. The Rust crate is never linked or called directly.

## Two methodology traps that were found and closed

These are worth recording because each one would have produced a **false pass**:

1. **`cargo test` does not rebuild a `cdylib`.** The first mutation run reported
   60/60 green for six deliberately broken versions of `lib.rs`, because the
   tests were loading a stale `target/release/libdriver.so`. Closed two ways:
   `assert_fresh()` in the harness refuses to run when a `.so` is older than its
   source (verified: touching `src/lib.rs` without rebuilding now aborts with
   `STALE ARTIFACT`), and `verify.sh`/`mutate.sh` always `cargo build` first.
2. **libtest writes its own progress text to fd 1.** With more than one test
   thread, `test foo ... ok` landed inside the capture window and corrupted the
   comparison. Closed by `.cargo/config.toml` setting `RUST_TEST_THREADS=1`,
   flushing Rust's `std::io::stdout` *and* libc's stdio before redirecting, and
   an assertion that fails loudly if the thread count is overridden.

## Negative control (`./mutate.sh`)

Green tests only mean something if they go red on a wrong translation. Each
mutant is injected into `src/lib.rs`, the cdylib is rebuilt, and the suite is
re-run. **10 / 10 caught:**

| mutant | injected defect | result |
|--------|-----------------|--------|
| M1 | saturating cast instead of `INT_MIN` indefinite | CAUGHT (8 tests) |
| M2 | `goodB2G` threshold `1e-6` → `1e-7` | CAUGHT (7) |
| M3 | `printLine` passes `line` as the *format string* | CAUGHT (15) |
| M4 | NaN converts to `0` instead of `INT_MIN` | CAUGHT (6) |
| M5 | `floor` instead of truncate-toward-zero | CAUGHT (12) |
| M6 | `goodG2B` constant `2.0` → `4.0` | CAUGHT (10) |
| M7 | `driver` label typo | CAUGHT (6) |
| M8 | quotient rounded to f32 precision | CAUGHT (9) |
| M9 | `printLine` NULL guard inverted | CAUGHT (7) |
| M10 | `printIntLine` uses `%u` instead of `%d` | CAUGHT (16) |

One mutant is expected to survive, and does:

| | change | why surviving is correct |
|---|--------|--------------------------|
| E1 | `>` → `>=` at the `1e-6` threshold | For these to differ, some `f32` would have to widen to *exactly* `1e-6`. None does: `1e-6` is not representable in 24 significant bits, and `1e-6f` widens to `9.99999997475e-7`. Verified against the three nearest `f32` neighbours — a provable semantic equivalence, not a coverage gap. |

## Notes on the C being ground truth

`bad()` has no divide-by-zero guard — that is the deliberately injected defect
this test case exists to demonstrate (the `good`/`bad` naming is the Juliet/CWE
convention). Division by zero and out-of-range float→int conversion are C
undefined behaviour; the observable behaviour of the compiled `.so` on this
target was taken as ground truth and pinned in `ERRORS.md` rows 9–18, verified
by linking a probe program directly against `libdriver.so`. The C was not
"fixed", and its `else`-on-NaN quirk and truncate-toward-zero semantics were
replicated rather than corrected.
