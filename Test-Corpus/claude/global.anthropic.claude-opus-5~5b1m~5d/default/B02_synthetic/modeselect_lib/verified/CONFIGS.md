# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

There is no runtime option struct, no global state, and no `#ifdef` in the C
source — the "configuration" of this library is entirely **the shape of the
arguments**. The axes, grepped from the source:

* **A1 — `classify_mode` string identity**: 4 exact-match branches (`"standard"`,
  `"enhanced"`, `"turbo"`, `"extreme"`) + the no-match fall-through (lib.c:30-39).
* **A2 — `apply_multiplier` level**: 5 fall-through `case` labels `4→3→2→1→0`
  (each a *different* accumulated sum) + `default` (lib.c:45-59). 6 distinct paths.
* **A3 — `apply_multiplier` base magnitude**: small / near `INT_MAX` / near
  `INT_MIN` (signed overflow in `result +=`).
* **A4 — double → int conversion domain** (`convert_time_factor` ×`1e12`,
  `convert_negative_overflow` ×`-1e15`): in-range, out-of-range, `NaN`, `±inf`,
  `±0.0`, denormal, exactly-at-`INT_MIN`/`INT_MAX`, negative vs positive.
* **A5 — `get_modified_time` offsets**: sign of `offset_days`, sign of
  `offset_hours`, zero, `int`-overflowing products, `int`-overflowing sum.
* **A6 — `hash_time_value` `t` byte pattern**: `sizeof(time_t) == 8` so the loop
  runs 8 times with `i % 4` wrapping twice — zero, small positive, negative,
  `INT64_MIN`, `INT64_MAX`, all-`0xFF`, random 64-bit patterns.
* **A7 — `modeselect` `mode_selector % 4`**: selects which of the 4 modes, i.e.
  couples A1 to the top-level entry point. Values `0,1,2,3` and `4k+r`.
* **A8 — `modeselect` `complexity % 5`**: selects the A2 path (`0..4`), negative
  `complexity` reaches `default`.
* **A9 — `modeselect` `seed % 24`**: feeds `offset_hours`; sign follows `seed`.
* **A10 — `modeselect` `seed`/`time_offset` magnitude**: drives A4 via
  `seed*1e8` and `time_offset*-1e7`, and the `result1 & 0xFF` /
  `result2 & 0xFF00` mixing masks.
* **A11 — entry-point level**: the six *low-level* exports are called directly
  (not only through the `modeselect` convenience wrapper), because
  `c_src/include/lib.h` hides them but the `.so` exports them.

`sizeof(time_t)` is 8 on this ABI, and `time(NULL) >> 29` is constant for ~17
years, so `get_modified_time`/`modeselect` are deterministic *within a test run*
and are compared C-vs-Rust back to back.

## Rows (pruned cross-product of the axes the C distinguishes)

Every row is driven with **many randomized inputs** (`SeedableRng`-style xorshift,
fixed seed `0x5EED_1234_ABCD_EF01`) unless it is a pure enumeration.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|------------------------------------------|------|-----|
| 1 | `classify_mode` | each of the 4 exact literals `"standard"/"enhanced"/"turbo"/"extreme"` | `cfg_01_classify_mode_exact_literals` | [x] |
| 2 | `classify_mode` | 4096 random byte strings (len 0..16, bytes 1..=255) — mostly non-matching, exercises `strcmp` on arbitrary data | `cfg_02_classify_mode_random_strings` | [x] |
| 3 | `classify_mode` | random strings drawn from a small alphabet so near-misses of the literals occur often (1-char mutations of the 4 literals) | `cfg_03_classify_mode_near_misses` | [x] |
| 4 | `apply_multiplier` | `level` enumerated `0..=4` (the 5 distinct fall-through sums) × `base` = `0` | `cfg_04_apply_multiplier_levels_base_zero` | [x] |
| 5 | `apply_multiplier` | `level` `0..=4` × 4096 random `base` (full `i32` range, incl. overflow) | `cfg_05_apply_multiplier_levels_random_base` | [x] |
| 6 | `apply_multiplier` | `level` `0..=4` × `base` ∈ {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`} (boundary bases, A3) | `cfg_06_apply_multiplier_boundary_bases` | [x] |
| 7 | `apply_multiplier` | 4096 fully random `(base, level)` pairs — level mostly outside `0..=4` | `cfg_07_apply_multiplier_random_pairs` | [x] |
| 8 | `convert_time_factor` | in-range domain: `factor` ∈ `(-2.147e-3, 2.147e-3)` random, so `factor*1e12` fits in `int` (the only non-overflowing shape) | `cfg_08_convert_time_factor_in_range` | [x] |
| 9 | `convert_time_factor` | random `f64` over many magnitudes (`2^-60 … 2^60`, both signs) — mixed in-range / out-of-range | `cfg_09_convert_time_factor_random_magnitudes` | [x] |
| 10 | `convert_time_factor` | random raw 64-bit bit patterns reinterpreted as `f64` (includes NaNs, infinities, denormals, negative zero) | `cfg_10_convert_time_factor_random_bits` | [x] |
| 11 | `convert_negative_overflow` | in-range domain: `value` ∈ `(-2.147e-6, 2.147e-6)` random, so `value*-1e15` fits in `int`; checks the sign flip | `cfg_11_convert_negative_overflow_in_range` | [x] |
| 12 | `convert_negative_overflow` | random `f64` over many magnitudes, both signs | `cfg_12_convert_negative_overflow_random_magnitudes` | [x] |
| 13 | `convert_negative_overflow` | random raw 64-bit bit patterns as `f64` | `cfg_13_convert_negative_overflow_random_bits` | [x] |
| 14 | `get_modified_time` | `(days, hours)` = `(0,0)` and small non-overflowing values, both signs (A5 sign cross-product) | `cfg_14_get_modified_time_small_offsets` | [x] |
| 15 | `get_modified_time` | 4096 random `(days, hours)` over the full `i32` range (products and sums overflow `int`) | `cfg_15_get_modified_time_random_offsets` | [x] |
| 16 | `get_modified_time` | boundary offsets: each of `{INT_MIN, INT_MIN+1, -24856, -1, 0, 1, 24855, 24856, INT_MAX}` × `{INT_MIN, -1, 0, 1, 23, 596523, INT_MAX}` | `cfg_16_get_modified_time_boundary_offsets` | [x] |
| 17 | `hash_time_value` | `t` enumerated over boundary values `{0, 1, -1, i64::MIN, i64::MAX, 0xFFFF_FFFF, 0x1_0000_0000, time(NULL)>>29}` | `cfg_17_hash_time_value_boundaries` | [x] |
| 18 | `hash_time_value` | 8192 random 64-bit `t` values (exercises the `i%4` byte-lane wrap over all 8 bytes) | `cfg_18_hash_time_value_random` | [x] |
| 19 | `hash_time_value` | `t` values with exactly one byte non-zero, for each of the 8 byte positions and each of 256 byte values — isolates the `<< ((i%4)*8)` lane mapping | `cfg_19_hash_time_value_single_byte_lanes` | [x] |
| 20 | `modeselect` | full 4×5 cross-product of `mode_selector ∈ 0..4` (A7) × `complexity ∈ 0..5` (A8) with `time_offset = seed = 0` | `cfg_20_modeselect_mode_x_complexity` | [x] |
| 21 | `modeselect` | `mode_selector ∈ 0..4` × `complexity ∈ 0..5` × `seed ∈ {0,1,23,24,25}` (A9 `%24` wrap) × `time_offset ∈ {0,1,-1}` | `cfg_21_modeselect_mode_complexity_seed_offset` | [x] |
| 22 | `modeselect` | 2048 randomized `(mode_selector, time_offset, complexity, seed)` with `mode_selector >= 0` (in-bounds index) over the full `i32` range — drives A4/A10 overflow mixing | `cfg_22_modeselect_random_full_range` | [x] |
| 23 | `modeselect` | randomized with `seed` restricted to `|seed| < 22` so `seed*1e8*1e12` stays in-range and `result1 & 0xFF` is *non-zero* (otherwise the overflow sentinel masks the mixing) | `cfg_23_modeselect_seed_in_range_conversion` | [x] |
| 24 | `modeselect` | randomized with `time_offset` restricted so `time_offset*-1e7*-1e15` is in-range, making `result2 & 0xFF00` meaningful | `cfg_24_modeselect_time_offset_in_range_conversion` | [x] |
| 25 | `modeselect` | boundary parameter tuples: every component ∈ `{0, 1, 3, 4, 5, 23, 24, INT_MAX}` (non-negative selector) | `cfg_25_modeselect_boundary_tuples` | [x] |
| 26 | **stdout** of `modeselect` (all 8 `printf` calls) | stdout of C vs Rust captured byte-for-byte in a forked child, over the 4×5 mode/complexity grid plus randomized tuples — covers `%s`, `%d`, `%X`, `%ld`, `%.2e` formatting | `cfg_26_modeselect_stdout_byte_identical` | [x] |
| 27 | **composed pipeline** | `get_modified_time` → `hash_time_value` chained through the `.so`s (the exact composition `modeselect` performs), randomized, so a bug that cancels inside `modeselect` is still caught | `cfg_27_pipeline_get_modified_time_into_hash` | [x] |
| 28 | **composed pipeline** | `classify_mode` fed by the 4 mode literals → value folded exactly as `modeselect` does, cross-checked against `modeselect`'s own arithmetic | `cfg_28_pipeline_classify_into_modeselect_arithmetic` | [x] |

## C codegen sensitivity (measured, not assumed)

`c_src/CMakeLists.txt` sets no optimisation flags, so the ground-truth build is
gcc's default (`-O0`). `run_all.sh` additionally re-runs the entire suite against
the same C source rebuilt at `-O0`, `-O2` and `-O3 -fwrapv`; all three agree with
the Rust on every row.

At plain `-O3`, gcc **stops agreeing with itself**: `hash_time_value(0)` returns
`0x7E4B2761` instead of the `0xF6F605A` produced at `-O0`, `-O2` and
`-O3 -fwrapv`, because `hash *= 0x1F` overflows `int` (undefined behaviour) and
`-O3` reassociates the loop on that assumption. `0xF6F605A` is the wrapping
two's-complement value, which is what the Rust reproduces, so the translation
matches the C as the project actually builds it. `-O3` without `-fwrapv` is
therefore excluded from the sweep as a C-side UB artefact, not a translation
defect.

## Binary executable

`c_src/CMakeLists.txt` builds **no executable** (`add_library` only, no `main` in
`lib.c`), so the "compare C and Rust binary stdout" gate is satisfied by row 26,
which compares the library's own stdout byte-for-byte instead.
