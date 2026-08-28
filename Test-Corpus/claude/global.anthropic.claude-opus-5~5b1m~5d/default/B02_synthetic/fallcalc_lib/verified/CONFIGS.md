# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`: every **valid** input configuration the C actually
branches on. Derived mechanically from the `if` / `switch` / loop-guard
structure of `c_src/src/lib.c`, not from what looks important.

## Axes the C branches on

This library has **no** runtime options, no global state, no `#ifdef`, and no
compile-time feature flags — `grep -c '#if\|#ifdef\|static ' c_src/src/lib.c` is
`0`. The configuration surface is therefore entirely made of **input shapes**:

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A. `double` class | NaN, +Inf, -Inf, `>= 2^31-1`, `<= -2^31`, in-range +/-, `0.0`, `-0.0`, subnormal, fractional (truncation direction) | `safe_double_to_int` L49-64 |
| B. `count` shape | `<0`, `0`, `1`, many | `process_array_reverse` L71, `foreach_sum` L130 (`FOREACH`) |
| C. element values | positive, negative, mixed, magnitudes that overflow the `int` accumulator | both sum loops |
| D. `operation` arm | `0` (3-deep fallthrough), `1` (2-deep), `2` (`break`), `3` (2-deep), `4` (`break`), `default` | `switch_fallthrough_calculator` L82-97 |
| E. `value` magnitude | small, and large enough that `*8`, `*3`, `+0200` wrap `int` | same |
| F. `size` shape | `<0`, `0`, `1`, small-many, huge | `allocate_and_compute` L103-117 |
| G. `multiplier` class | `0.0`, `1.5`, negative, huge (`->Inf`), NaN, subnormal | same |
| H. `param3 > 0200` flag | true / false — the `result \|= 0200` bit | `fallcalc` L167 |
| I. `param3 % 5` sign | `0..4` (real arm) vs `-4..-1` (default arm, E8) | `fallcalc` L158 |
| J. `param4 % 10 + 1` sign | `>0` (alloc succeeds) vs `<=0` (alloc returns `-1`) | `fallcalc` L163 |
| K. `param1`/`param2` magnitude | small, and large enough to wrap `param1 * 0100 + param2` and the float math | `fallcalc` L140, L160 |

## Rows (pruned cross-product of the axes the code actually distinguishes)

Every row is driven through **both** `.so`s via `libloading` with many
randomized inputs (fixed seed, deterministic SplitMix64 PRNG) unless marked
*exhaustive*.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `safe_double_to_int` | axis A: in-range positive fractional values (truncation toward zero) | [x] |
| C2 | `safe_double_to_int` | axis A: in-range negative fractional values (truncation toward zero) | [x] |
| C3 | `safe_double_to_int` | axis A: `0.0`, `-0.0`, `+/-MIN_POSITIVE`, subnormals, tiny fractions | [x] |
| C4 | `safe_double_to_int` | axis A: exact `int` boundaries `+/-2147483646/7/8` and one ULP either side | [x] |
| C5 | `safe_double_to_int` | axis A: uniformly random `u64` reinterpreted as `f64` (all classes incl. NaN/Inf) | [x] |
| C6 | `process_array_reverse` | axis B=`0` / B=`1`, `end` = last element of a real buffer | [x] |
| C7 | `process_array_reverse` | axis B=many (2..64) x axis C=small positive values | [x] |
| C8 | `process_array_reverse` | axis B=many x axis C=full-range random `i32` (accumulator wraps) | [x] |
| C9 | `process_array_reverse` | axis B=many x axis C=all `i32::MAX` / all `i32::MIN` (forced overflow) | [x] |
| C10 | `foreach_sum` | axis B=`0` / B=`1` (`FOREACH` degenerate iterations) | [x] |
| C11 | `foreach_sum` | axis B=many (2..64) x axis C=full-range random `i32` (wrapping) | [x] |
| C12 | `foreach_sum` + `process_array_reverse` | same buffer through both: forward vs backward traversal must agree | [x] |
| C13 | `switch_fallthrough_calculator` | axis D=`0` x axis E=small/random/extreme `value` (`*8` then `+0200` then `&0777`) | [x] |
| C14 | `switch_fallthrough_calculator` | axis D=`1` x axis E (`+0200` then `&0777`) | [x] |
| C15 | `switch_fallthrough_calculator` | axis D=`2` x axis E (`&0777` only) | [x] |
| C16 | `switch_fallthrough_calculator` | axis D=`3` x axis E (`*3` then `+0100`, **no** mask) | [x] |
| C17 | `switch_fallthrough_calculator` | axis D=`4` x axis E (`+0100`, **no** mask) | [x] |
| C18 | `switch_fallthrough_calculator` | axis D=`default` x axis E (negative + `>4` operations) | [x] |
| C19 | `switch_fallthrough_calculator` | *exhaustive* over `operation` in `-8..=12` x a fixed set of extreme `value`s | [x] |
| C20 | `allocate_and_compute` | axis F=`0` (`malloc(0)`, both loops skipped) x axis G | [x] |
| C21 | `allocate_and_compute` | axis F=`1` x axis G=`1.5` (the value `fallcalc` uses) | [x] |
| C22 | `allocate_and_compute` | axis F=`2..=64` x axis G=`1.5` (float accumulation order) | [x] |
| C23 | `allocate_and_compute` | axis F small-many x axis G=random finite (incl. negative, subnormal) | [x] |
| C24 | `allocate_and_compute` | axis F small-many x axis G=`0.0` / `-0.0` (sum stays zero / neg-zero) | [x] |
| C25 | `allocate_and_compute` | axis F large-many (1..=4096) x axis G=huge (`sum` overflows to `+/-Inf` -> clamp) | [x] |
| C26 | `allocate_and_compute` | axis F=`1..=10` — exactly the range `fallcalc` can request — x G=`1.5` | [x] |
| C27 | `fallcalc` | axis I=`0..4` x axis H=false x axis J=positive: each real switch arm reachable | [x] |
| C28 | `fallcalc` | axis I=negative (default arm) x axis J=negative (inner `-1`) | [x] |
| C29 | `fallcalc` | axis H=true (`param3 > 0200`) x axis I both signs | [x] |
| C30 | `fallcalc` | axis H boundary: `param3` in `126..=130` (`> 0200` is strict) | [x] |
| C31 | `fallcalc` | axis K=small `param1`/`param2` (`-1000..=1000`), all `param3`/`param4` residues | [x] |
| C32 | `fallcalc` | axis K=full-range random `i32` for all four params (wrapping + float rounding) | [x] |
| C33 | `fallcalc` | axis K=extremes: every combination drawn from `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` (*exhaustive* 7^4 = 2401) | [x] |
| C34 | `fallcalc` | *exhaustive* over `param3 % 5` x `param4 % 10` residue classes (5 x 10 x 2 signs) | [x] |
| C35 | composed pipeline | `fallcalc` recomputed from the five low-level exports of the **other** library (cross-checks the composition, not just each part) | [x] |

All 35 rows pass. C1–C26 live in `tests/phase_b_low_level.rs`, C27–C35 in
`tests/phase_b_fallcalc.rs`, keyed by the row id in the test name.

## Suite validity (does it have teeth?)

Passing tests only mean something if the suite can fail. `mutation_check.sh`
compiles 25 deliberately-broken copies of `c_src/src/lib.c` (in `$TMPDIR` —
`c_src/` is never modified), points the suite at each via `C_SO_PATH`, and
requires rejection: wrong octal constants, removed `switch` fallthroughs,
`ptr--` → `ptr++`, `>=` → `>` on the flag guard, off-by-one array init and
allocation size, perturbed float coefficients, a flipped sign, and a changed
`-1` sentinel.

Result under **both** the dev and release profiles: **25 caught, 0 missed**,
plus 2 mutants proven *semantically equivalent* rather than missed — relaxing
`d >= (double)INT_MAX` to `>` (and the `INT_MIN` mirror) changes nothing,
because at exactly `(double)INT_MAX` the fallthrough `(int)d` already yields
`INT_MAX`. Verified by brute force: 0 differences across 5,173,210 doubles
including exhaustive one-ULP sweeps around both thresholds. A control run with
an unmutated rebuild passes, confirming the mutation harness's `gcc` flags
reproduce the CMake build.

## Soak

`tests/soak.rs` (`#[ignore]`d; run with `-- --ignored`) adds 13.75M randomized
differential cases, concentrated on the only rounding-sensitive code — the
`fallcalc` float expression and the `allocate_and_compute` accumulator. Passes
in dev and release, ruling out FMA contraction, x87 excess precision and
reassociation differences.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only feature
configurations are `<default>`, `--no-default-features` and `--all-features`
(all identical, since there is no `default` feature). There are no
`#[cfg(feature = ...)]` attributes in `src/lib.rs`, so no code path can differ
between them.

Because a *feature* axis alone would be vacuous here, `run_all_feature_combos.sh`
crosses the 3 feature configurations with 11 build configurations that genuinely
change generated code — which is what actually mattered, since the one real bug
in this translation only appeared at `-O2`+:

| profile config | why it matters |
|----------------|----------------|
| `dev` | `debug-assertions` + `overflow-checks` on by default |
| `dev+overflow-checks` | proves no `wrapping_*` path was written as `+`/`*` |
| `dev+opt2` | optimizations with debug assertions still on |
| `dev+debug-assertions-off` | assertions off, unoptimized |
| `release` | `panic = "abort"`, `opt-level=3` |
| `release+overflow-checks` | optimized *and* overflow-checked |
| `release+opt3` / `release+opt-s` | different inlining/vectorization decisions |
| `release+lto-thin` / `release+lto-fat` | cross-crate optimization (set via `CARGO_PROFILE_RELEASE_LTO`, since `-C lto` conflicts with cargo's `-C embed-bitcode=no`) |
| `release+codegen-units=1` | whole-`.so` optimization, most aggressive elision |

**Result: 33 configurations, 55 tests each, 0 failures.** The script also runs
`cargo check --all-targets` for every feature configuration.

The feature/profile sweep is what caught the release-only `malloc` elision bug
documented in `ERRORS.md`; the dev profile alone would have reported everything
green.
