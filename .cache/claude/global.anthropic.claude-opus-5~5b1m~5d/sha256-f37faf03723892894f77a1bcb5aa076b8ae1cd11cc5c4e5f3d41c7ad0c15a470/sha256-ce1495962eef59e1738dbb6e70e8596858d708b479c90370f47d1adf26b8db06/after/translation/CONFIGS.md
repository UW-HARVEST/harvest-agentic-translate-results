# CONFIGS.md — Configuration surface table (valid inputs)

## Axes actually present in the C source

This library has **no runtime options, no mode/flag setters, no global state,
no `#ifdef`, and no init/teardown**. `grep -nE '#if|static|extern|global'
c_src/src/lib.c` finds nothing configurable. `translation/Cargo.toml` has **no
`[features]` section**, so there is exactly one build configuration.

The axes the C code actually branches on are therefore the *input shapes*:

| axis | values the C distinguishes | source |
|---|---|---|
| **A1** `safe_double_to_int` value class | `> INT_MAX` / `< INT_MIN` / NaN / in-range | `lib.c:40-47` |
| **A2** truncation direction | positive fraction (toward 0 = down) vs negative fraction (toward 0 = up) vs exact integer vs `±0.0` vs subnormal | `(int)d` cast, `lib.c:47` |
| **A3** `process_with_fallthrough` `code` | `5`,`4`,`3`,`2`,`1` (fall-through chain, distinct added totals), `0` (discards `base_value`), `default` | `lib.c:54-72` |
| **A4** `base_value` magnitude | small, near `INT_MAX` (wraps), near `INT_MIN` | `result += …`, `lib.c:56-64` |
| **A5** `copy_data_block` buffer content | zeroed / all-`0xFF` / random incl. **padding bytes** / non-ASCII & un-terminated `label` / extreme `value` (NaN, inf, subnormal) | `memcpy(…, sizeof(DataBlock))`, `lib.c:78` |
| **A6** `copy_data_block` aliasing | disjoint buffers, `dest == src` | `lib.c:78` |
| **A7** `handle_pointer_operations` `value` | small, negative, `INT_MAX`/`INT_MIN` (`*2` wraps), `±(INT_MAX/2)` boundary | `lib.c:83` |
| **A8** `overunder` `a mod 6` residue | `0..5` for `a >= 0`, and `-5..-1` for `a < 0` (C `%` truncates toward zero ⇒ `default`) | `lib.c:115` |
| **A9** `overunder` `d*d + a*a` | no overflow (positive ⇒ real `sqrt`), overflow to negative (⇒ NaN ⇒ `conv4 = 0`), zero | `lib.c:106` |
| **A10** `overunder` scaling clamps | `a*1.5`, `b*2.7`, `c/3.3` each in range vs clamped to `INT_MAX`/`INT_MIN` | `lib.c:103-105` |
| **A11** `overunder` `total` accumulation | no wrap vs two's-complement wrap | `lib.c:133-134,152` |
| **A12** observable channel | return value **and** the 6 `printf` lines written to `stdout` (`result_1`, `result_2`, `Converted values`, `Switch fall-through`, `Copied block` incl. `%.2f`, `Pointer operation`, `Overflow`/`Underflow`, `Array copied`) | `lib.c:100-154` |

## Entry points

All five exported symbols are driven **directly** through their `.so` exports —
the low-level `safe_double_to_int`, `process_with_fallthrough`,
`copy_data_block`, `handle_pointer_operations` as well as the composed
one-shot wrapper `overunder` (which is the only symbol in `include/lib.h`).

## Rows (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (see `tests/harness/mod.rs`), not a single hand-picked value, and
compared byte-for-byte between the C and Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `safe_double_to_int` | A1=in-range, A2=exact integers, randomized over full `int` range (`d = k as f64`, 20 000 samples) | [x] |
| 2  | `safe_double_to_int` | A1=in-range, A2=positive fraction, randomized mantissas ⇒ truncation toward zero | [x] |
| 3  | `safe_double_to_int` | A1=in-range, A2=negative fraction, randomized ⇒ truncation toward zero (up) | [x] |
| 4  | `safe_double_to_int` | A2=`+0.0`, `-0.0`, smallest subnormals `±5e-324`, `±1e-300` | [x] |
| 5  | `safe_double_to_int` | A1 boundaries: exactly `±2147483647.0`, `±2147483648.0`, `2147483646.5`, `-2147483647.5`, each ULP-neighbour | [x] |
| 6  | `safe_double_to_int` | A1=out-of-range, randomized huge magnitudes (`1e9..1e300`, both signs) | [x] |
| 7  | `safe_double_to_int` | **fully random bit patterns** reinterpreted as `f64` (covers NaN/inf/subnormal/normal jointly, 50 000 samples) | [x] |
| 8  | `process_with_fallthrough` | A3=`5` (falls 5→4→3→2→1, +150) × A4 randomized `base_value` over full range | [x] |
| 9  | `process_with_fallthrough` | A3=`4` (falls 4→3→2→1, +100) × A4 randomized | [x] |
| 10 | `process_with_fallthrough` | A3=`3` (falls 3→2→1, +60) × A4 randomized | [x] |
| 11 | `process_with_fallthrough` | A3=`2` (falls 2→1, +30) × A4 randomized | [x] |
| 12 | `process_with_fallthrough` | A3=`1` (+10) × A4 randomized | [x] |
| 13 | `process_with_fallthrough` | A3=`0` (result forced to `0`) × A4 randomized — `base_value` must be discarded | [x] |
| 14 | `process_with_fallthrough` | A3=`default` × A4 randomized (see ERRORS rows 11-13) | [x] |
| 15 | `process_with_fallthrough` | A3×A4 fully randomized `(code, base_value)` pairs over the full `int` range, 50 000 samples | [x] |
| 16 | `copy_data_block` | A5=zeroed source, A6=disjoint — all 40 bytes incl. padding compared | [x] |
| 17 | `copy_data_block` | A5=all-`0xFF` source, A6=disjoint | [x] |
| 18 | `copy_data_block` | A5=fully random 40-byte patterns (random padding, random `label` with **no NUL**, random `value` bit patterns incl. NaN/inf), A6=disjoint, 5 000 samples | [x] |
| 19 | `copy_data_block` | A5=random, A6=`dest == src` (self-copy must be a no-op in both) | [x] |
| 20 | `copy_data_block` | A5=structured `DataBlock` built field-wise (`id`, `value`, `label`) then read back field-wise through the FFI struct layout | [x] |
| 21 | `handle_pointer_operations` | A7 randomized over the full `int` range, 50 000 samples (covers `*2` wrap) | [x] |
| 22 | `handle_pointer_operations` | A7 boundaries: `0`, `±1`, `INT_MAX/2`, `INT_MAX/2 + 1`, `INT_MIN/2`, `INT_MAX`, `INT_MIN` | [x] |
| 23 | `overunder` | A8=`0` (`a % 6 == 0`, `switch_result` forced to `0`), other args randomized small | [x] |
| 24 | `overunder` | A8=`1` × randomized `b,c,d` (small, no clamping, no overflow) | [x] |
| 25 | `overunder` | A8=`2` × randomized small args | [x] |
| 26 | `overunder` | A8=`3` × randomized small args | [x] |
| 27 | `overunder` | A8=`4` × randomized small args | [x] |
| 28 | `overunder` | A8=`5` × randomized small args | [x] |
| 29 | `overunder` | A8 negative residues `-1..-5` (`a < 0`) ⇒ `default` arm × randomized args | [x] |
| 30 | `overunder` | A9=no overflow: `\|a\|,\|d\| <= 46340` ⇒ `d*d + a*a` stays positive, real `sqrt` (checks the `sqrt` + truncation path densely, 5 000 samples) | [x] |
| 31 | `overunder` | A9=overflow to negative ⇒ `sqrt(NaN)` ⇒ `conv4 = 0`, randomized large `a,d` | [x] |
| 32 | `overunder` | A9=`a == 0 && d == 0` ⇒ `sqrt(0)` | [x] |
| 33 | `overunder` | A10=`a*1.5` clamps to `INT_MAX`/`INT_MIN` (`\|a\| > 2/3·INT_MAX`) | [x] |
| 34 | `overunder` | A10=`b*2.7` clamps (`\|b\| > INT_MAX/2.7`) | [x] |
| 35 | `overunder` | A10=`c/3.3` never clamps but exercises division rounding, randomized full-range `c` | [x] |
| 36 | `overunder` | A11=`total` wraps: all of `a,b,c,d` near `INT_MAX` / near `INT_MIN` | [x] |
| 37 | `overunder` | `(a,b,c,d)` fully random over the entire `int` range, 3 000 samples (joint cross-product of A8-A11) | [x] |
| 38 | `overunder` | all-zero arguments `(0,0,0,0)`; all-`±1`; `(INT_MAX, INT_MAX, INT_MAX, INT_MAX)`; `(INT_MIN, INT_MIN, INT_MIN, INT_MIN)`; 16-way sign cross-product of `INT_MAX/INT_MIN` | [x] |
| 39 | `overunder` | **A12 stdout**: all 6 `printf` groups captured via `dup2` on fd 1 and compared byte-for-byte C vs Rust, over randomized args (incl. the `%.2f` formatting of `value` and the `%s` `label`) | [x] |
| 40 | `overunder` | A12 stdout for the ERRORS-row-30 case: `Copied block: … label=Source` and the trailing-pad invariant | [x] |
| 41 | composed | `overunder` return value cross-checked against an independent recomposition built from the four low-level `.so` exports, proving the pipeline wiring (not just each wrapper) matches | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]`, so the complete set of combinations is:

| combo | command |
|---|---|
| default (= only combo) | `cargo test --release` |
| explicit no-default     | `cargo test --release --no-default-features` |
| all-features            | `cargo test --release --all-features` |

All three are run by `run_all_configs.sh`.

## Verification results

All rows were driven through the `.so` exports of BOTH libraries with
`libloading` and compared byte-for-byte (return value **and** captured
`stdout`). Reproduce with `./run_all_configs.sh`.

```
suite `phase_b_valid`: 43 passed; 0 failed
suite `phase_c_errors`: 30 passed; 0 failed
```

Every row above is checked off, and each was additionally re-run against the C
library rebuilt at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` (via the
`DIFFTEST_C_SO` override, without touching `c_src/`) crossed with both Rust
profiles — 20 extra combinations, all matching. This matters because the C
relies on signed-overflow wrap-around in several places, and it confirms the
Rust `wrapping_*` choices agree with what gcc actually emits at every
optimization level rather than only at the default one.

### Divergence found and fixed during Phase B/C

`copy_data_block` (and the `array2`/`array1` copy inside `overunder`) was
translated with `core::ptr::copy_nonoverlapping`. That intrinsic carries
debug-only preconditions — pointers non-null, ranges non-overlapping — which
made the Rust `.so` **panic** in `debug` builds on two inputs the C simply
accepts: `dest == src` (CONFIGS row 19) and a NULL argument (ERRORS rows
17-19). Since `lib.c:78` calls `memcpy`, the fix was to call libc `memcpy`
directly, which restores identical behaviour in every profile. This was
invisible in `--release` and is exactly the kind of bug the debug/release ×
feature sweep is there to catch.
