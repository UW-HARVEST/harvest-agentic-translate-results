# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually distinguishes

The C has **no** runtime options, no flags, no modes, no `#ifdef`, and no
branches. The surface is therefore defined by (a) which entry point is called,
(b) the value of the single `int` argument, and (c) the accumulated state of
the file-scope global `the_house`, which is the library's *only* hidden input.

**Axis 1 — entry point** (both are dynamically exported; see `SYMBOLS.md`)
* `run(int extra_bedrooms)` — the LOW-LEVEL entry point (4 prints, 1 floor, +1.0 bathroom, 1 bedroom add)
* `driver(int x)` — the convenience wrapper: `run(x); run(x);` (8 prints, 2 floors, +2.0 bathrooms, value applied twice)

**Axis 2 — `int` argument value class** (drives `bedrooms += extra_bedrooms` and the `%d` formatting)
* `0` (identity), small positive, small negative, `+1`/`-1`,
  `INT_MAX`, `INT_MIN`, `INT_MAX/2`, values chosen to land the accumulator
  exactly on `INT_MAX` / `INT_MIN` / `0`, and uniformly-random full-range ints.

**Axis 3 — accumulated global state** (no reset entry point exists)
* pristine (`floors=2, bedrooms=5, bathrooms=2.5`) — only observable on the very first call in a process
* after N calls: `floors = 2+N`, `bathrooms = 2.5+N`, `bedrooms` = running wrapped sum
* `bedrooms` currently positive / currently negative / currently zero
* `bedrooms` near `INT_MAX` / near `INT_MIN` (so the next add wraps)

**Axis 4 — call-sequence shape** (state persistence is the pipeline under test)
* single call; two calls; many calls (empty / one / many)
* homogeneous `run`-only sequence; homogeneous `driver`-only sequence;
  **interleaved** `run`/`driver` sequence (the composed pipeline — invisible to per-function tests)

**Axis 5 — printed-output shape** (what `%d` / `%.1f` must render identically)
* 1-digit, multi-digit, and 10-digit `bedrooms`/`floors`; negative (leading `-`);
  `bathrooms` magnitude growing (`2.5` → `12.5` → `102.5` → …), always `.5`-exact

## Configuration table

One row per meaningful combination the C treats differently. Every row is
exercised with MANY randomized inputs (fixed seed, `SEED = 0x5EED_1234_ABCD_F00D`),
not a single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `run` | pristine global state, `extra_bedrooms = 0` — first-ever call in the process, identity add | [x] |
| 2  | `run` | `extra_bedrooms = 0` repeated (state advances, bedrooms unchanged; isolates floors/bathrooms accumulation) | [x] |
| 3  | `run` | small positive `extra_bedrooms` in `1..=100`, randomized, many iterations | [x] |
| 4  | `run` | small negative `extra_bedrooms` in `-100..=-1`, randomized — drives `bedrooms` negative, `%d` prints `-` | [x] |
| 5  | `run` | `extra_bedrooms = +1` / `-1` boundary steps, alternating | [x] |
| 6  | `run` | uniformly-random full-range `i32` (all bit patterns), many iterations | [x] |
| 7  | `run` | `extra_bedrooms = INT_MAX`, repeated (accumulator wraps every call) | [x] |
| 8  | `run` | `extra_bedrooms = INT_MIN`, repeated (accumulator wraps every call) | [x] |
| 9  | `run` | `extra_bedrooms = INT_MAX/2`, `INT_MIN/2`, `2^k` powers-of-two sweep for k=0..31 | [x] |
| 10 | `run` | large positive values that push `bedrooms` past `INT_MAX` (positive→negative wrap) | [x] |
| 11 | `run` | large negative values that push `bedrooms` below `INT_MIN` (negative→positive wrap) | [x] |
| 12 | `driver` | pristine-ish state, `x = 0` — wrapper applies identity twice, 8 lines | [x] |
| 13 | `driver` | small positive / small negative randomized `x`, many iterations | [x] |
| 14 | `driver` | uniformly-random full-range `i32`, many iterations | [x] |
| 15 | `driver` | `x = INT_MAX`, `INT_MIN` (value applied twice ⇒ double wrap in one call) | [x] |
| 16 | `driver` | `x` = powers-of-two sweep k=0..31 | [x] |
| 17 | `run` + `driver` interleaved | randomized interleaving of both entry points with randomized args — the composed pipeline over shared global state, many iterations | [x] |
| 18 | `run` + `driver` interleaved | interleaving with adversarial args (`INT_MAX`/`INT_MIN`/`0`/`±1`) so wraps occur at arbitrary points in the sequence | [x] |
| 19 | `run` | state driven so `bedrooms` lands exactly on `0`, then `INT_MAX`, then `INT_MIN` (exact boundary landings, computed from the tracked accumulator) | [x] |
| 20 | `run`/`driver` | long endurance sequence (≥2000 mixed calls) — `floors`/`bathrooms` grow to multi-digit; verifies no drift in `%d` width or `%.1f` magnitude between C and Rust | [x] |
| 21 | `run` | output-shape sweep: args chosen so `bedrooms` renders 1-, 2-, 5-, 10-digit and negative forms | [x] |
| 22 | `driver` | called as the very first symbol resolved from the `.so` (fresh-process ordering: `driver` before `run` ever runs) — separate test binary so global state is pristine | [x] |

## Row → test mapping

Rows 2–21 live in `tests/configs.rs`, named `rowN_*`. Rows 1 and 22 require
**pristine** global state (`the_house` has no reset entry point, so its static
initialiser is observable only on the first call in a fresh process), so each
gets its own test binary — and each contains exactly ONE `#[test]`, since a
second test in the same binary could consume the pristine state first:

| row | test |
|-----|------|
| 1  | `tests/first_call_run.rs::row1_pristine_state_first_ever_call_to_run` |
| 2–21 | `tests/configs.rs::row2_*` … `row21_*` |
| 22 | `tests/first_call_driver.rs::row22_pristine_state_first_ever_call_to_driver` |

## How each row is checked

Every row drives BOTH `.so`s through `dlopen`/`dlsym` (never a direct Rust
call — the crate is `cdylib`-only, so the tests *cannot* link it), captures
each library's `printf` output by redirecting fd 1 with `dup2`, and asserts the
two byte strings are equal. Randomized rows use SplitMix64 seeded from `SEED`.

Because the library's only state is a global with no reset, the two libraries
are driven in **lockstep**: each step calls C then Rust with the same argument
under one process-wide lock, so both globals always observe the identical
operation sequence. An independent Rust model of the C semantics is compared
against the captured bytes on every step, which detects harness desync and
capture contamination rather than letting them pass silently.
