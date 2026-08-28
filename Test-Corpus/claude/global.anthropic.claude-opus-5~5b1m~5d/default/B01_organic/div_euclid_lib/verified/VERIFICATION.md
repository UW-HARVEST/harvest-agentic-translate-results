# VERIFICATION.md — final report

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Both are loaded as shared objects via `libloading` and called only
through their exported `extern "C"` symbols, so the `#[no_mangle]` wrappers are
themselves under test.

Reproduce everything with:

```bash
cd translation && ./run_all.sh
```

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | symbol diff EMPTY in all 6 configurations; C exports 1 symbol (`div_euclid`), Rust exports it; Rust's only undefined symbols are libc/libgcc-unwind |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | 43/43 rows, `tests/phase_b_configs.rs` |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** | 12/12 rows + G1–G7 generic boundaries, `tests/phase_c_errors.rs` |
| All of the above under every feature combination | **PASS** | crate declares no `[features]`; all 3 equivalent invocations x 2 profiles = 6 configurations, 59 tests each |

Totals: **59 tests, 6 configurations, 0 failures.** Roughly 8.5 million
`(v1, v2)` pairs compared per configuration.

## Result: the translation is correct

No divergence was found. `src/lib.rs` was **not modified** — it already
reproduced the C exactly, including the parts most likely to be mistranslated:

* the **dangling-`else` parse** of the nested `if`/`else` ladder (each `else`
  binds to the nearest unmatched `if`, matching the source indentation);
* **L2's non-negated remainder** (`r = v1 % (-v2)`) versus L5's negated one
  (`r = -((-v1) % (-v2))`) — an asymmetry in the C that is easy to "tidy up";
* the **comma-operator sequencing** in `q = 1, r = v1 - q * v2`, where `r`
  depends on the `q` assigned earlier in the same statement;
* the three **`INT_MIN` range checks** and the `-(v1+v2)` / `-(v1-v2)` rewrites
  that exist to avoid evaluating `-INT_MIN`;
* the one **signed-overflow** path, `div_euclid(INT_MIN, -1)`, where
  `q = INT_MAX + 1` wraps to `INT_MIN` in the `-O0` C build. Rust's
  `wrapping_add` reproduces this; a plain `+` would have panicked in debug.

### Note on the C's UB, and why the C build config matters

`c_src` is compiled with `C_FLAGS = -fPIC` and no `-O` flag, i.e. **`-O0`**.
`div_euclid(INT_MIN, -1)` executes `INT_MAX + 1`, which is undefined behaviour
in C; at `-O0` GCC emits a plain `addl` and the result wraps to `INT_MIN`, which
is what the Rust matches. This equality is a property of *this* build, not of
the C standard — at higher optimization levels GCC may exploit the UB. Flagged
for the record; no action taken, since the C as built is the ground truth.

Separately audited: **no input can crash the C.** Every `/` and `%` in `lib.c`
provably has a non-zero divisor and a non-`INT_MIN` dividend on its own path, so
`SIGFPE` (the x86 `idiv` trap on `INT_MIN / -1`) is unreachable. That is what
makes exhaustive differential sweeping safe.

## How the tests were shown to be sensitive (not just green)

Green tests prove nothing unless they can fail. Two independent checks:

### 1. Mutation campaign — `python3 mutate.py`

20 mutations, each injecting a realistic mistranslation into one branch of
`src/lib.rs`, rebuilding the `.so`, and requiring the suite to fail.

**Result: 19/20 killed.**

The single survivor, **M19**, is a *provably equivalent* mutant, not a coverage
gap: it changes `if v2 >= 0` to `if v2 > 0` in the `v1 >= 0` arm, but `v2 == 0`
has already returned at the top of the function, so `v2 >= 0` and `v2 > 0` are
logically identical there. No input can distinguish them, so no test can — and
the same equivalence holds in the C.

### 2. Negative controls on the runner

`run_all.sh` was run against a deliberately bugged `lib.rs` and correctly
reported `TESTS FAILED` with exit status 1 in all 6 configurations.

## Two harness bugs found and fixed during verification

Both would have caused **false passes**, so they are worth recording:

1. **`run_all.sh` ignored test failures.** It piped `cargo test` into `tee` and
   then keyed off the *pipeline's* exit status, which is `tee`'s. When the temp
   path was unwritable, `tee` failed, the grep never ran, and failing tests were
   reported as a pass. Fixed to capture `cargo test`'s own exit code (plus a
   floor on the number of tests that must pass, so a suite that silently
   shrinks is also caught).

2. **`cargo test` silently tested a stale `.so`.** With `crate-type =
   ["cdylib"]`, `cargo test` does not build or refresh the `cdylib`, so the
   tests `dlopen`ed whatever `libdiv_euclid_lib.so` happened to be on disk. An
   earlier build's artifact was tested instead of the current source. The
   harness originally rebuilt only when the file was *missing*, which is not
   enough. `tests/common/mod.rs::find_rust_so` now rebuilds
   **unconditionally**. Verified both ways: a poisoned-but-present `.so` is
   refreshed and the suite passes; a freshly introduced source bug is caught
   with no manual build step.

## Test inventory

| file | tests | role |
|------|-------|------|
| `tests/common/mod.rs` | — | loads both `.so`s via `dlopen`/`dlsym`; `check`/`check_eq`/`Cmp` comparators; fixed-seed PCG32 |
| `tests/phase_b_configs.rs` | 43 | one test per `CONFIGS.md` row + leaf-coverage meta-test |
| `tests/phase_c_errors.rs` | 13 | one test per `ERRORS.md` row + G1–G7 generic FFI boundaries |
| `tests/phase_d_symbols.rs` | 3 | `nm -D` symbol parity, undefined-symbol audit, `dlsym` callability |

Coverage is self-checked rather than assumed: `leaf_of()` independently
re-implements the C ladder's branch selection, and the tests assert that all ten
control paths (`v2 == 0` early return plus leaves L1–L9) are actually reached,
that row 40 ran all 4,000,000 iterations with each bulk leaf hit >500,000 times,
that row 38 made exactly 1,050,625 comparisons, and that row 41 made
>2,000,000.

## Environment

* `rustc` / `cargo` 1.94.0; `cargo` needs `--offline` (crates.io unreachable in
  this sandbox; `libloading` 0.8.9 and `cfg-if` 1.0.4 come from the local
  registry cache).
* GCC 11.5.0, CMake 3.22.2, GNU `nm` 2.41, x86-64 Linux.
* `c_src/` was not modified. The only change to `translation/` outside
  `tests/` and the Phase A/report markdown was adding
  `libloading = "0.8"` to `[dev-dependencies]`; `src/lib.rs` is unchanged.
