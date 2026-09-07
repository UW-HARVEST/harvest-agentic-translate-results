# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c`.

## Axes the C code actually distinguishes

**A1 — public entry points** (from `nm -D`, not just the header):
- `run(int extra_bedrooms)` — the LOW-LEVEL entry point (exported but absent
  from `driver.h`); one pass over the house.
- `driver(int x)` — the convenience wrapper; `run(x); run(x);`

There are no other exported functions. The four `static` helpers
(`add_floor`, `add_bedrooms`, `add_floor_to_the_house`, `print_the_house`) are
reachable only through `run`, so `run` *is* the lowest level available to a
consumer and must be driven directly.

**A2 — runtime options / modes / flags:** NONE. The C code contains zero `if`,
`switch`, `#ifdef`, global setter, or environment lookup. The only input is the
single `int` argument. Cargo.toml declares no `[features]`, so there is exactly
one build configuration.

**A3 — input shape of `extra_bedrooms` (the only datum)**, per its sole use
`the_house.bedrooms += extra_bedrooms` (signed `int` arithmetic):
- zero
- small positive / small negative
- large positive / large negative
- `INT_MAX`, `INT_MIN` (arithmetic wraps)
- values engineered so `bedrooms + extra` lands exactly on / one past
  `INT_MAX` / `INT_MIN`

**A4 — persistent-global state shape.** `static house_t the_house` is file-scope
mutable and is NOT reset between calls, so the *call sequence* is part of the
input. State fields and the shapes they take:
- `floors : int` — `++` once per `run`; shapes: initial `2`, small, large after
  many calls.
- `bedrooms : int` — `+= extra`; shapes: initial `5`, positive, negative,
  wrapped.
- `bathrooms : double` — `+= 1.0` once per `run`, printed with `%.1f`; shapes:
  initial `2.5`, small `.5` values, large magnitudes where `double`
  accumulation and `%.1f` rounding matter.
- sequence length: 0 / 1 / 2 / many calls; and *interleavings* of `run` and
  `driver`.

**A5 — output surface.** One `printf` format string,
`"The house has %d floors, %d bedrooms, and %.1f bathrooms\n"`, invoked 4× per
`run` (8× per `driver`). Compared byte-for-byte on captured `stdout`.

## Configuration table (cross-product of A1 × A3 × A4, pruned to distinct paths)

Every row: load BOTH `.so`s via `libloading`, issue the identical call sequence
to each, capture each one's `stdout`, assert byte-equality. Rows marked
*randomized* use many property-style inputs from a fixed-seed PRNG.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `run` | pristine global state, first-ever call, `extra_bedrooms = 0` (identity add; isolates the `floors++` / `bathrooms += 1.0` effects) | `configs::c01_run_zero_pristine` | [x] |
| 2 | `run` | pristine state, `extra_bedrooms = 1` (minimal positive) | `configs::c02_run_one_pristine` | [x] |
| 3 | `run` | pristine state, `extra_bedrooms = -1` (minimal negative; drives `bedrooms` downward) | `configs::c03_run_neg_one_pristine` | [x] |
| 4 | `run` | pristine state, small positive *randomized* (`1..=1000`, 200 seeded values, one per fresh process-equivalent sequence) | `configs::c04_run_small_positive_random` | [x] |
| 5 | `run` | pristine state, small negative *randomized* (`-1000..=-1`) — makes `bedrooms` cross zero and go negative (`%d` sign path) | `configs::c05_run_small_negative_random` | [x] |
| 6 | `run` | pristine state, full-range `int` *randomized* (any `i32`, 500 seeded values) | `configs::c06_run_full_range_random` | [x] |
| 7 | `run` | `extra_bedrooms = INT_MAX` — signed overflow of `bedrooms` (5 + INT_MAX wraps) | `configs::c07_run_int_max` | [x] |
| 8 | `run` | `extra_bedrooms = INT_MIN` — signed underflow of `bedrooms` | `configs::c08_run_int_min` | [x] |
| 9 | `run` | `extra_bedrooms` chosen so `bedrooms` lands **exactly** on `INT_MAX`, then one past; same for `INT_MIN` | `configs::c09_run_boundary_landings` | [x] |
| 10 | `driver` | pristine state, `x = 0` — wrapper shape: 8 lines, `floors` +2, `bathrooms` +2.0 | `configs::c10_driver_zero_pristine` | [x] |
| 11 | `driver` | pristine state, `x` small positive *randomized* (`bedrooms` gets `2x` total) | `configs::c11_driver_small_positive_random` | [x] |
| 12 | `driver` | pristine state, `x` full-range `int` *randomized* (500 seeded values) — double-wrap of `bedrooms` | `configs::c12_driver_full_range_random` | [x] |
| 13 | `driver` | `x = INT_MAX` / `INT_MIN` — `bedrooms` wraps twice within one call | `configs::c13_driver_extremes` | [x] |
| 14 | `run` ×N | **state accumulation:** repeated `run` with the *same* arg, N = 2, 3, 10 — verifies globals are not reset and `floors`/`bathrooms` track together | `configs::c14_run_repeated_same_arg` | [x] |
| 15 | `run` ×N | repeated `run` with *different* randomized args (N = 50, seeded) | `configs::c15_run_repeated_random_args` | [x] |
| 16 | `run` + `driver` interleaved | *randomized interleaving* of the two entry points with randomized args (300 ops, seeded) — the composed pipeline over shared global state | `configs::c16_interleaved_random` | [x] |
| 17 | `driver` ×N | repeated `driver` — `floors` +2 and `bathrooms` +2.0 per call, N = 20 | `configs::c17_driver_repeated` | [x] |
| 18 | `run` ×many | **`bathrooms` double-accumulation shape:** ~20 000 `run` calls with arg 0 so `bathrooms` reaches ≥ 20 000.5 and `floors` ≥ 20 000 — exercises `%.1f` on large magnitudes and wide `%d` | `configs::c18_long_accumulation` | [x] |
| 19 | `run`/`driver` | **empty sequence:** zero calls — both `.so`s load and export the symbols, no output produced (baseline; guards against constructor-time printing) | `configs::c19_no_calls_no_output` | [x] |
| 20 | `run` | fresh-state equivalence: the *same* arg replayed against a freshly `dlopen`-ed pair after `dlclose` — verifies the initial `{2, 5, 2.5}` initialiser (`.data` vs Rust `static mut`) matches, not just the deltas | `configs::c20_fresh_load_initial_state` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, and `driver.c` has
no `#ifdef`. Therefore the complete set of feature combinations is the single
default (empty) one: `cargo test` == `cargo test --no-default-features`. Both
are run by `run_all.sh` for completeness.

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED ...)` — no
`add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]` (no `[[bin]]`, no `src/main.rs`). **The project builds
no binary**, so the "compare C and Rust binary stdout" gate is not applicable;
the equivalent coverage is provided by the captured-`stdout` comparison in every
row above.
