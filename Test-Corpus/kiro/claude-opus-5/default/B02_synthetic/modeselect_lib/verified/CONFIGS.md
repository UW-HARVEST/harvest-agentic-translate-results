# CONFIGS.md — Phase A configuration-surface table (valid inputs)

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

**Runtime options/modes/flags:** the library has none — no setters, no context
struct, no globals, no `#ifdef`. `translation/Cargo.toml` has no `[features]`
table, so there is exactly one build configuration. The configuration surface is
therefore entirely the **input shape** axes below, crossed per entry point.

Axes the C branches on:

* `classify_mode`: which of the 4 `strcmp` literals matches (`src/lib.c:30-38`) → 5 outcomes.
* `apply_multiplier`: `level` selects one of 5 distinct *fall-through* accumulations
  (case 4 adds `0xFF+0xAB+0x7E+0x1C+0x05`, case 3 drops `0xFF`, …, case 0 adds only
  `0x05`) plus `default`; independently, `base` magnitude (overflow vs not).
* `convert_time_factor` / `convert_negative_overflow`: magnitude class of the
  product (in-`int`-range / overflow+ / overflow− / zero / subnormal / NaN / inf),
  and sign.
* `get_modified_time`: sign and magnitude of each of the two `int` products, and
  whether either product or the sum overflows `int`.
* `hash_time_value`: byte pattern of `t` (all 8 bytes are consumed, with `i%4`
  re-using shift positions for bytes 4..7), sign of `t`, high bytes zero vs set.
* `modeselect`: cross product of `mode_selector % 4` (4 in-bounds modes) ×
  `complexity % 5` (5 fall-through levels, plus negative → `default`) ×
  `seed % 24` × `time_offset` overflow class, plus stdout formatting
  (`%s`, `%d`, `%X` of negatives, `%.2e`, `%ld`).

All 7 exported entry points are covered directly, including the six low-level
ones — not only the `modeselect` one-shot wrapper (rows 33-46 exercise the
composed pipeline, rows 1-32 the primitives underneath it).

Every row is driven with **many randomized inputs** from a fixed-seed PRNG
(seed `0x9E3779B97F4A7C15`, SplitMix64), not a single hand-picked value, and
asserted byte-identical between the C `.so` and the Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `classify_mode` | exact literal `"standard"` → `0x10` | [x] |
| 2  | `classify_mode` | exact literal `"enhanced"` → `0x20` | [x] |
| 3  | `classify_mode` | exact literal `"turbo"` → `0x30` | [x] |
| 4  | `classify_mode` | exact literal `"extreme"` → `0x40` | [x] |
| 5  | `classify_mode` | randomized non-matching ASCII strings, lengths 0..32 (incl. embedded high bytes 0x80-0xFF) | [x] |
| 6  | `classify_mode` | randomized single-byte mutations of each of the 4 literals (same length, one byte changed) | [x] |
| 7  | `classify_mode` | each literal truncated at every prefix length, and each extended by 1..4 random bytes | [x] |
| 8  | `apply_multiplier` | `level = 0`, randomized `base` over full `int` range | [x] |
| 9  | `apply_multiplier` | `level = 1`, randomized `base` (fall-through 1→0) | [x] |
| 10 | `apply_multiplier` | `level = 2`, randomized `base` (fall-through 2→1→0) | [x] |
| 11 | `apply_multiplier` | `level = 3`, randomized `base` (fall-through 3→2→1→0) | [x] |
| 12 | `apply_multiplier` | `level = 4`, randomized `base` (full fall-through 4→3→2→1→0) | [x] |
| 13 | `apply_multiplier` | `level ∈ {0..4}`, `base` at `INT_MAX`, `INT_MAX-k` for k=0..0x300 → signed overflow wrap | [x] |
| 14 | `apply_multiplier` | `level ∈ {0..4}`, `base = INT_MIN`, `0`, `-1` (boundary bases) | [x] |
| 15 | `convert_time_factor` | product in `int` range: `factor` randomized in `(-2.1e-3, 2.1e-3)` | [x] |
| 16 | `convert_time_factor` | product in range, `factor` tiny (`1e-300`..`1e-12`) → truncates toward 0 | [x] |
| 17 | `convert_time_factor` | `factor = ±0.0`, `±f64::MIN_POSITIVE`, subnormals | [x] |
| 18 | `convert_time_factor` | boundary sweep: `factor*1e12` straddling `±2147483647`/`±2147483648` | [x] |
| 19 | `convert_time_factor` | product out of range: randomized `\|factor\| > 2.15e-3`, both signs, up to `f64::MAX` | [x] |
| 20 | `convert_time_factor` | randomized *bit patterns* reinterpreted as `f64` (covers NaN/inf/subnormal uniformly) | [x] |
| 21 | `convert_negative_overflow` | product in `int` range: `value` randomized in `(-2.1e-6, 2.1e-6)` (note the `-1e15` sign flip) | [x] |
| 22 | `convert_negative_overflow` | `value = ±0.0` (→ `∓0.0`), subnormals, tiny magnitudes | [x] |
| 23 | `convert_negative_overflow` | boundary sweep: `value*-1e15` straddling `±2147483647`/`±2147483648` | [x] |
| 24 | `convert_negative_overflow` | product out of range, both signs, up to `f64::MAX` | [x] |
| 25 | `convert_negative_overflow` | randomized `f64` bit patterns (NaN/inf/subnormal) | [x] |
| 26 | `get_modified_time` | both offsets small & non-overflowing (`\|days\| ≤ 1000`, `\|hours\| ≤ 1000`), all 4 sign combinations | [x] |
| 27 | `get_modified_time` | `offset_days` alone overflows `*86400` (`\|days\| > 24855`), randomized | [x] |
| 28 | `get_modified_time` | `offset_hours` alone overflows `*3600` (`\|hours\| > 596523`), randomized | [x] |
| 29 | `get_modified_time` | both products fit but their **sum** overflows `int` | [x] |
| 30 | `get_modified_time` | fully randomized over the whole `int × int` space, plus `{0, ±1, INT_MIN, INT_MAX}²` corners | [x] |
| 31 | `hash_time_value` | `t` small non-negative (`0..=1<<20`) — high 5 bytes zero | [x] |
| 32 | `hash_time_value` | `t` realistic epoch values, `t` negative, `t` with every byte ≥ 0x80 (exercises the `<<24` sign-bit shift), `t ∈ {0, ±1, i64::MIN, i64::MAX}`, and fully randomized `i64` | [x] |
| 33 | `modeselect` | `mode_selector % 4 = 0` (`"standard"`) × `complexity % 5 = 0`, randomized `time_offset`/`seed` | [x] |
| 34 | `modeselect` | `mode_selector % 4 = 1` (`"enhanced"`) × `complexity % 5 = 1` | [x] |
| 35 | `modeselect` | `mode_selector % 4 = 2` (`"turbo"`) × `complexity % 5 = 2` | [x] |
| 36 | `modeselect` | `mode_selector % 4 = 3` (`"extreme"`) × `complexity % 5 = 3` | [x] |
| 37 | `modeselect` | full cross product `mode_selector % 4 ∈ {0,1,2,3}` × `complexity % 5 ∈ {0,1,2,3,4}` (20 combos), randomized `time_offset`/`seed` per combo | [x] |
| 38 | `modeselect` | `complexity < 0` → `complexity_level ∈ {-1,-2,-3,-4}` → `apply_multiplier` `default` → `0xDEAD` | [x] |
| 39 | `modeselect` | `seed = 0` (`factor1 = 0.0`, prints `0.00e+00`, `Result 1: 0`) | [x] |
| 40 | `modeselect` | `time_offset = 0` (`factor2 = -0.0`, prints `-0.00e+00`) — sign-of-zero formatting | [x] |
| 41 | `modeselect` | `seed > 0` and `seed < 0` → `Result 1` overflow path; `seed % 24` positive/negative/zero | [x] |
| 42 | `modeselect` | `time_offset` large positive/negative → `get_modified_time` `int` overflow inside the pipeline | [x] |
| 43 | `modeselect` | corner arguments: each of the 4 params ∈ `{0, 1, -1, 4, 5, 23, 24, INT_MAX, INT_MIN}` (in-bounds `mode_selector` only) | [x] |
| 44 | `modeselect` | `mode_selector = INT_MIN` and negative multiples of 4 (`-4, -8, …`) → in-bounds index 0 | [x] |
| 45 | `modeselect` | **stdout** byte-for-byte comparison of all 8 printed lines for every configuration above (captured via `dup2` around each FFI call) | [x] |
| 46 | `modeselect` | fully randomized 4-tuples (`mode_selector ≥ 0`), 2000 iterations, return value **and** stdout compared | [x] |

## Test-sensitivity validation (why "all rows pass" is meaningful here)

Every row above passed on the first run, which on its own proves nothing — a
suite that asserts too little also passes. `mutation_check.sh` therefore injects
39 realistic C-to-Rust translation bugs into `src/lib.rs`, rebuilds, and requires
the suite to FAIL for each one:

```
=== mutation summary: 39/39 caught, 0 survived ===
```

Covered bug classes include: Rust's saturating `as` cast replacing x86
`cvttsd2si` INT_MIN semantics, `rem_euclid` replacing C's truncating `%`,
`saturating_add`/64-bit math replacing wrapping overflow, each individual
`switch` fall-through step being dropped, byte-order and shift errors in
`hash_time_value`, the `default:`/`0xDEAD` sentinel, and `printf` format-string
differences (`%.2e` vs `%.3e`, `%X` vs `%d`, a missing leading newline).

Three mutations are documented as **provably equivalent** (undetectable by any
test, because the C lines have no observable effect for any input) rather than
counted as gaps — see the notes at the bottom of `mutation_check.sh` and the test
`modeselect_result1_result2_xor_steps_are_provably_dead`.

## The one inherent divergence

`modeselect` with `mode_selector % 4 ∈ {-1,-2,-3}` reads past the start of the C's
4-element local `modes` array. From the `-O0` disassembly, `modes` is at
`-0x70(%rbp)` with the spilled arguments directly below, so the C hands `strcmp`
a pointer assembled from its own argument bytes (`-1`, `-2`) or from stack
garbage below `%rsp` (`-3`). Observed outcomes for `-3` differ between processes
of the same build. There is no input-determined value for the Rust to reproduce,
so `mode_selector` is restricted accordingly in Phase B and the asymmetry is
asserted explicitly in Phase C row 32. This is the only behaviour in the library
that is not byte-identical, and it is unreachable for any `mode_selector >= 0`.
