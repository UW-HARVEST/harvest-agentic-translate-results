# CONFIGS.md — Phase B configuration surface table

## Axes derived from the C source

`c_src/include/driver.h` exposes exactly one entry point, the lowest-level one:

```c
void driver(int x, int y);
```

There are **no runtime options/modes/flags**, **no `#ifdef`s**, **no global
state**, **no convenience wrappers** (so "lowest-level entry point" == the only
entry point). `translation/Cargo.toml` declares **no `[features]`**, so the only
feature combination is the default (see Phase D).

The configuration surface is therefore the cross-product of the input shapes
the code branches on. Every branch in the C body:

| line | branch | axis it creates |
|------|--------|-----------------|
| 30 | `while (x > 0 \|\| y > 0)` | sign of `x` (`>0` vs `<=0`) × sign of `y` |
| 33 | `if (x == 1 && y == 4)` | `x == 1` exactly; `y == 4` exactly (the `goto label2` special case) |
| 38 | `if (x > 0)` | `x > 0` vs `x <= 0` at the `label1` re-entry point |
| 44 | `if (y == 0)` → `continue` | `y == 0` vs `y != 0` |
| 49 | `if (x < 3)` → `goto label1` | `x < 3` vs `x >= 3` — chooses whether the body re-loops or ends |

So the distinguished value classes are:

* `x` ∈ { `<= 0` (incl. `INT_MIN`, `-1`, `0`), `1`, `2`, `>= 3` (3, 4, large, `INT_MAX`) }
* `y` ∈ { `< 0`, `0`, `1`, `2`, `3`, `4` (special), `5`, `> 5`, large }

Note a **termination constraint** read off the C: `y--` only stops at `y == 0`,
so a negative `y` combined with `x > 0` never terminates (row E9 of
`ERRORS.md`). Valid-path rows therefore keep `y >= 0` whenever `x > 0`.

## Rows (each = a combination the C treats differently)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | guard false on entry: `x <= 0 && y <= 0` — randomized over `x,y ∈ [INT_MIN, 0]` | [x] |
| C2 | `driver` | `x > 0`, `y == 0` — pure `x`-drain, `continue` path; randomized `x ∈ [1, 40]` | [x] |
| C3 | `driver` | `x <= 0`, `y > 0` — pure `y`-drain, `if (x>0)` always false; randomized `x ∈ [-40, 0]`, `y ∈ [1, 40]` | [x] |
| C4 | `driver` | `x == 1`, `y == 4` — the exact `goto label2` special case (skips `label1` on the first pass) | [x] |
| C5 | `driver` | `x == 1`, `y != 4`, `y > 0` — special case rejected on the `y` half; randomized `y ∈ [1,40] \ {4}` | [x] |
| C6 | `driver` | `x != 1`, `y == 4` — special case rejected on the `x` half; randomized `x ∈ [-10,40] \ {1}` | [x] |
| C7 | `driver` | `x == 2`, `y > 0` — `x < 3` true so the backwards `goto label1` fires; randomized `y ∈ [1,40]` | [x] |
| C8 | `driver` | `x >= 3`, `y > 0` — `x < 3` false at first, so the body ends and the `while` guard is re-tested; the run later crosses into `x < 3` and switches mode mid-run; randomized `x ∈ [3,40]`, `y ∈ [1,40]` | [x] |
| C9 | `driver` | boundary `x == 3`, `y == 1` and neighbours `x ∈ {2,3,4}` × `y ∈ {0,1,2}` — exhaustive small grid around the `x < 3` / `y == 0` boundaries | [x] |
| C10 | `driver` | boundary around the special case: `x ∈ {0,1,2}` × `y ∈ {3,4,5}` exhaustive | [x] |
| C11 | `driver` | `y` much larger than `x` (`x ∈ [0,3]`, `y ∈ [50,200]`) — many `while` iterations, `continue` never taken until the end | [x] |
| C12 | `driver` | `x` much larger than `y` (`x ∈ [50,200]`, `y ∈ [0,3]`) — long `x`-drain after `y` hits 0 | [x] |
| C13 | `driver` | exhaustive dense grid `x ∈ [-3, 12]` × `y ∈ [0, 12]` — the full cross-product of all five branches at small magnitudes | [x] |
| C14 | `driver` | randomized property sweep, fixed seed (xorshift64\*, seed `0x2545F4914F6CDD1D`), 2000 pairs with `x ∈ [-50, 300]`, `y ∈ [0, 300]` | [x] |
| C15 | `driver` | `x <= 0` with `y` at the positive extreme class — `x ∈ {INT_MIN, -1, 0}`, `y ∈ {1, 2, 4, 64}` | [x] |
| C16 | `driver` | repeated invocation / statefulness check: the same `.so` handle called many times in sequence, asserting no carried state (C has no globals; Rust must not either) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)` — there is
**no driver binary/executable target**, and `translation/Cargo.toml` declares
only library crate types (`cdylib` — the deliverable — plus `rlib`, added purely
so `cargo test` rebuilds the `.so`; see the harness note below) with no
`[[bin]]` and no `src/main.rs`. The
"compare C and Rust binary stdout" gate is therefore **not applicable**;
stdout is instead compared per-call by redirecting fd 1, which covers the same
observable surface.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table and no optional
dependencies, so the complete set of feature combinations is:

| combo | command |
|-------|---------|
| default | `cargo test --release` |
| no-default-features | `cargo test --release --no-default-features` |
| all-features | `cargo test --release --all-features` |

All three are run by `run_all_combos.sh`.

## Result

`tests/configs.rs :: phase_b_all_config_rows` runs rows C1–C16 sequentially:

```
running 1 test
test phase_b_all_config_rows ... ok
```

All 16 rows pass under all six profile × feature combinations
(`./run_all_combos.sh` → `ALL COMBINATIONS PASSED`).

## Harness note (a real bug this uncovered — in the harness, not the library)

Two traps had to be closed before these results meant anything:

1. **fd 1 is process-global.** `driver`'s only output is `printf`, so the
   harness redirects fd 1. With libtest's default thread-per-test parallelism,
   libtest's own progress banner (`test cN ... ok`) landed *inside* a capture
   window and produced four bogus "divergences". Fixed by running every row from
   a single `#[test]` and flushing Rust's `Stdout` before each capture.
2. **`cargo test` does not refresh `target/<profile>/libdriver.so`.** An
   integration test that only `dlopen`s the cdylib creates no build dependency
   on it. `cargo test` rebuilds `target/<profile>/deps/libdriver.so` but leaves
   the top-level hardlink stale, so the suite was comparing against an old
   object. Fixed by (a) adding `rlib` to `crate-type` so the lib is a build
   dependency of the tests, (b) loading the *newest* of the two candidate paths,
   and (c) asserting the chosen `.so` is not older than `src/lib.rs`.

`./mutation_check.sh` guards against a recurrence: a semantics-preserving "null
mutant" must PASS, and 14 single-branch mutations of `src/lib.rs` (one per
condition, comparison bound, decrement and printed literal in the C body) must
each FAIL. Current status: **null mutant survives, 14/14 mutants killed**.
