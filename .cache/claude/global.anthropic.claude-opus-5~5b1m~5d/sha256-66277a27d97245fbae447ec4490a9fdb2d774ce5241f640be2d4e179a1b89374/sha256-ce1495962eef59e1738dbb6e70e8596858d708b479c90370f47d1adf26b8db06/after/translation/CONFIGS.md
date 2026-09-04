# CONFIGS.md — configuration / valid-input surface table

## Public API surface (from `c_src/include/driver.h`)

```c
void driver(int x, int y);
```

* Entry points: **exactly one**, `driver`. There is no convenience wrapper vs.
  low-level split, no context/handle object, no init/teardown, no callbacks.
* Runtime options / modes / flags: **none**. No globals (`grep -nE
  '^[a-zA-Z].*=|static' c_src/src/driver.c` → nothing), no `#ifdef` in the
  library source, no environment lookups, so there is no option state to set.
* Cargo features in `translation/Cargo.toml`: **none declared** → the only
  build configuration is the default one (see "feature combinations" below).
* Observable output: bytes written to `stdout` via libc (`printf` → `puts`).
  The differential tests capture `stdout` and compare byte-for-byte.

## Axes the C code actually branches on

| axis | source line | distinguished values |
|------|-------------|----------------------|
| A1 loop guard `x > 0 \|\| y > 0` | 30 | `x<=0 && y<=0` (skip) vs. `x>0` vs. `y>0` |
| A2 `x == 1 && y == 4` (`goto label2`) | 33 | exactly `(1,4)` vs. everything else |
| A3 `x > 0` (`printf("x"); x--`) | 38 | `x<=0` vs. `x>0` |
| A4 `y == 0` (`continue`) | 44 | `y==0` vs. `y!=0` (incl. `y<0`!) |
| A5 `x < 3` (backward `goto label1`) | 49 | `x<3` vs. `x>=3` |
| A6 magnitude / sign shape of `x` | — | `INT_MIN`, `<0`, `0`, `1`, `2`, `3`, `4`, large |
| A7 magnitude / sign shape of `y` | — | `INT_MIN`, `<0`, `0`, `1`, `2`, `3`, `4`, `5`, large |

## Configuration rows (pruned cross product)

Every row is exercised through **both** `.so` files via `libloading` and compared
byte-for-byte. Rows marked "randomized" use many pseudo-random values from a
fixed-seed LCG (seed `0x2545F4914F6CDD1D`).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | A1 false: `x<=0 && y<=0` — randomized over `x,y ∈ [-64,0]` | `cfg_c1_guard_false` | [x] |
| C2 | `driver` | `x>0, y==0` (A3 taken, A4 `continue` every iteration, A5 depends on `x`) — randomized `x ∈ [1,300]` | `cfg_c2_positive_x_zero_y` | [x] |
| C3 | `driver` | `x==0, y>0` (A3 never taken, A5 always true → inner backward-goto loop drains `y`) — randomized `y ∈ [1,300]` | `cfg_c3_zero_x_positive_y` | [x] |
| C4 | `driver` | `x<0, y>0` (A1 satisfied only via `y`; `x` never decremented) — randomized `x ∈ [-300,-1]`, `y ∈ [1,300]` | `cfg_c4_negative_x_positive_y` | [x] |
| C5 | `driver` | A2 hit: exactly `x==1, y==4` (`goto label2` skips the `label1` block on the first pass) | `cfg_c5_goto_label2_exact` | [x] |
| C6 | `driver` | A2 near-misses: `(1,3) (1,5) (0,4) (2,4) (1,-4) (-1,4)` | `cfg_c6_goto_label2_near_misses` | [x] |
| C7 | `driver` | `x==1`, all `y ∈ [0,16]` (A5 always true, A3 taken once) | `cfg_c7_x_one_sweep_y` | [x] |
| C8 | `driver` | `x==2`, all `y ∈ [0,16]` (A5 true boundary-1) | `cfg_c8_x_two_sweep_y` | [x] |
| C9 | `driver` | `x==3`, all `y ∈ [0,16]` (A5 false boundary: falls through to the guard) | `cfg_c9_x_three_sweep_y` | [x] |
| C10 | `driver` | `x==4`, all `y ∈ [0,16]` (A5 false, then `x` decays through 3→2 flipping A5 mid-run) | `cfg_c10_x_four_sweep_y` | [x] |
| C11 | `driver` | exhaustive small grid: every `(x,y)` with `x ∈ [-6,16]`, `y ∈ [0,16]` (all A1–A5 combinations reachable without UB) | `cfg_c11_exhaustive_small_grid` | [x] |
| C12 | `driver` | exhaustive negative-`y` grid, guard-false side: every `(x,y)` with `x ∈ [-6,0]`, `y ∈ [-16,-1]` (A4 `y!=0` with A1 false) | `cfg_c12_negative_y_grid` | [x] |
| C13 | `driver` | `x>=3 && y>=1` "wide" shape (A5 false path dominates) — randomized `x ∈ [3,400]`, `y ∈ [1,400]` | `cfg_c13_wide_random` | [x] |
| C14 | `driver` | `x` large ≫ `y` and `y` large ≫ `x` (asymmetric drain order) — randomized | `cfg_c14_asymmetric_random` | [x] |
| C15 | `driver` | largest feasible magnitudes: `(3000,0) (0,3000) (3000,3000) (3000,1) (1,3000) (2,3000)` | `cfg_c15_large_feasible` | [x] |
| C16 | `driver` | `INT_MIN` / `INT_MIN+1` / `INT_MAX`-adjacent inputs on the guard-false side, plus `x=INT_MIN` with `y>0` | `cfg_c16_extremes` | [x] |
| C17 | `driver` | repeated calls in sequence (statelessness: output of call *n* must not depend on calls `0..n-1`), interleaved C/Rust | `cfg_c17_statelessness_interleaved` | [x] |
| C18 | `driver` | unbounded path prefix (`x>0, y<0` → C spins ~2^31 times): compare the first 64 KiB of stdout from a forked child of each implementation | `cfg_c18_infinite_path_prefix` | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the complete set of
feature combinations is:

| combo | command |
|-------|---------|
| default (empty) | `cargo test` |
| `--no-default-features` | `cargo test --no-default-features` |
| `--all-features` | `cargo test --all-features` |

All three are identical builds of the same code path; `./run_all_combos.sh` runs
each of them in **both** the dev and release profiles (6 runs) and then diffs
`nm -D`. Every row above passes in all 6 runs.

## How to run

```sh
./run_all_combos.sh                      # everything, both profiles, + symbol diff
RUST_TEST_THREADS=1 cargo test -- --test-threads=1   # single run
```

`driver`'s only output channel is fd 1, so the harness redirects fd 1 around each
call; the libtest harness therefore **must** be single-threaded (the harness
asserts `RUST_TEST_THREADS=1` and tells you so if it is not).

## Harness sensitivity (mutation check)

To confirm the differential tests are not vacuous, four mutations were injected
into `src/lib.rs` one at a time; each was caught (7 of 17 `configs.rs` tests
failing) and then reverted:

| mutation | caught |
|----------|--------|
| `if x < 3` → `if x <= 3` | yes |
| `x == 1 && y == 4` → `x == 1 && y == 5` | yes |
| `if y == 0` → `if y == 1` | yes |
| `print_lit(b"y\n")` → `print_lit(b"Y\n")` | yes |
