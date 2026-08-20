# CONFIGS.md — Phase B configuration-surface table

## Axes mechanically derived from `c_src/src/lib.c`

**Build-time axes** — none. `Cargo.toml` has no `[features]`,
`c_src/CMakeLists.txt` has no options, and neither source contains an `#ifdef`
or `#[cfg(feature = …)]`. Exactly one configuration exists (the empty feature
set); every row below is therefore verified once, under
`cargo test --no-default-features`.

**Runtime axes the C code actually branches on**

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A1 `operation_func` selection | `add` / `multiply` / `subtract` / `modulo` / caller-supplied callback | `arrayfunc` op table, `process_with_foreach` indirect call |
| A2 `ResultArray::count` | `<0` (UB, documented), `0`, `1`, `2`, `9`, `10` (clamp), `>10` (past `data[10]`) | `init_result_array` clamp, `FOREACH` `!=` test, `compute_weighted_sum` `<` test, `compare_results_in_array` `>=` test |
| A3 `init_result_array` `count` argument | `<0`, `0`, `1`, `9`, `10`, `11`, `1000` | `count < 10 ? count : 10` |
| A4 element `value` magnitude | `0`, `±1`, small, `±2^20`, `±2^30`, `INT32_MIN`, `INT32_MAX` (drives the `safe_double_to_int` clamp inside `process_with_foreach` and `compute_weighted_sum`) | lines 129-131, 146-147 |
| A5 `safe_double_to_int` input class | `NaN` (both signs, quiet+signalling), `±INF`, `>= INT32_MAX`, `== INT32_MAX`, `(0, INT32_MAX)`, `±0.0`, `(-INT32_MIN, 0)`, `== INT32_MIN`, `<= INT32_MIN`, subnormal, exact halves, `x.5` / `-x.5` truncation | lines 76-85 |
| A6 `compute_scaled_value` scale class | `0.0`, `-0.0`, `1.0`, `1.5`, `0.333`, `0.75`, `0.8`, huge (`1e300`), tiny (`1e-300`), `NaN`, `±INF`, negative | line 89 |
| A7 `compare_results_in_array` index pair ordering | `idx1 < idx2`, `idx1 > idx2`, `idx1 == idx2`; each with both indices in range, one out of range, both out of range, and negative | lines 94-106 |
| A8 `arrayfunc` parameter shape | all-zero, all-positive, all-negative, mixed sign, `INT32_MIN`/`INT32_MAX` corners, values that make `param1+param2` / `param2-param3` / `param3*2` overflow, `param4` odd/even/negative-odd/`INT32_MIN` | lines 161-163 |
| A9 pipeline composition | single call to a leaf; `init` → one `process_with_foreach`; `init` → all four ops in sequence (state carried between ops); `init` → ops → `compute_weighted_sum`; the whole `arrayfunc` end-to-end | lines 168-184 |
| A10 array memory pre-state | zeroed buffer, `0xAA`-poisoned buffer (exposes any element the C leaves untouched) | `init_result_array` only writes `count` elements |

## Rows (cross-product pruned to what the C distinguishes)

Each row is exercised with **many randomized inputs** from a fixed-seed
SplitMix64 generator plus the hand-picked boundary values for its axis, and each
row asserts C-vs-Rust equality of every returned `int` **and** of the full
post-call `ResultArray` state (`count`, and per element `value`, `rank`, and the
raw 64-bit pattern of `scaled`).

### Level 0 — leaf arithmetic

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `add_operation` | full 15x15 boundary matrix `{INT32_MIN, INT32_MIN+1, -2^30, -2^16, -3, -2, -1, 0, 1, 2, 3, 2^16, 2^30, INT32_MAX-1, INT32_MAX}²` x 3 `(unused1, unused2)` pairs, plus 4096 random `(a,b)`; includes overflowing sums | [x] |
| C2 | `multiply_operation` | same input matrix, incl. overflowing products (`INT32_MIN*-1`, `2^30*4`, …) | [x] |
| C3 | `subtract_operation` | same input matrix, incl. overflowing differences (`INT32_MIN-1`) | [x] |
| C4 | `modulo_operation` | same input matrix **minus** the `(INT32_MIN,-1)` trap pair; `b == 0` row is E1; negative-dividend / negative-divisor sign behaviour | [x] |
| C5 | `safe_double_to_int` | A5: 48 hand-picked class representatives (both NaN signs, quiet + signalling, subnormals, exact halves), the 8 exact `nextafter` neighbours of the two comparison constants `±2147483647.0 / -2147483648.0`, 8192 random raw `f64` bit patterns (hits NaNs/INFs/huge/tiny) and 8192 random values clustered around the `i32` range | [x] |
| C6 | `compute_scaled_value` | A4 x A6 full cross-product (11 bases x 19 scales = 209 combinations) + 4096 random `(int, near-range f64)` + 4096 random `(int, raw-bits f64)` pairs | [x] |

### Level 1 — array primitives, driven directly

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C7 | `init_result_array` | A3 `count = 0`, zeroed buffer → nothing written, `count = 0` | [x] |
| C8 | `init_result_array` | A3 `count = 1`, `0xAA` buffer → element 0 written, elements 1..9 keep poison | [x] |
| C9 | `init_result_array` | A3 `count = 9`, random values incl. `INT32_MIN/MAX` | [x] |
| C10 | `init_result_array` | A3 `count = 10` (exact clamp boundary) | [x] |
| C11 | `init_result_array` | A3 `count ∈ {11, 12, 100, 1000, INT32_MAX}` (all clamp to 10; `values[10..]` must stay unread) | [x] |
| C12 | `init_result_array` | A3 `count = -1` / `-1000` (negative stored verbatim, `data[]` untouched) — see E14 | [x] |
| C13 | `init_result_array` | called twice on the same array with decreasing counts (stale tail elements must survive identically) | [x] |
| C14 | `compare_results_in_array` | A7 with `count = 10`: all 100 `(idx1, idx2)` pairs in `0..10` | [x] |
| C15 | `compare_results_in_array` | A7 with `count ∈ {0,1,2,5,9,10}` × `idx1,idx2 ∈ {-3,-1,0,1,count-1,count,count+1,10,11,INT32_MAX}` (full grid) | [x] |
| C16 | `compare_results_in_array` | `count ∈ {INT32_MAX, INT32_MAX-1, 2^20}` with huge indices (guard passes, pointer compare decides), plus 20 000 fully random `(count, idx1, idx2)` triples | [x] |
| C17 | `compute_weighted_sum` | `count = 0` → 0; `count = 1` → the `weight = 1` special case (E22) | [x] |
| C18 | `compute_weighted_sum` | `count ∈ 2..=10` with random values; verifies `weight = i` for `i >= 1` | [x] |
| C19 | `compute_weighted_sum` | `count = 10` with `INT32_MIN`/`INT32_MAX` values → per-term clamp + wrapping accumulate (A4) | [x] |
| C20 | `compute_weighted_sum` | called on a `0xAA`-poisoned array with `count = 3` (only the first 3 elements may be read) | [x] |
| C21 | `process_with_foreach` | A1 × A2: each of the four exported ops × `count ∈ {0,1,2,9,10}`, random values | [x] |
| C22 | `process_with_foreach` | A1 caller-supplied callback defined in the test binary (identity-of-`a`, `INT32_MAX`-returning, `INT32_MIN`-returning, `a*b`-overflowing) — exercises the indirect call with a pointer the library has never seen | [x] |
| C23 | `process_with_foreach` | A1 callback that returns huge values so the write-back `safe_double_to_int(result*0.75)` clamps (E19), `count = 10` | [x] |
| C24 | `process_with_foreach` | `count ∈ {11, 12}` on a **padded** 512-byte buffer, all four ops: `FOREACH` walks past `data[10]`, reading/writing over the `count` field. All 512 bytes compared | [x] |
| C25 | `process_with_foreach` | same op applied 6 times in a row so each pass sees the previous pass's mutated `value`/`scaled` state — the composed-pipeline case that per-call tests miss | [x] |

### Level 2 — composed pipelines and the public entry point

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C26 | `init_result_array` → `process_with_foreach`(add,mul,sub,mod in order) → `compute_weighted_sum` → `compare_results_in_array` loop → `safe_double_to_int` | A9 hand-composed replica of `arrayfunc`'s body, driven from the test through the `.so` exports only, over 2048 random 8-value arrays and all A4 boundary values. Full array state compared after **every** stage. | [x] |
| C27 | same chain as C26 | same as C26 but `count ∈ {0,1,2,5,9,10}` instead of 8 | [x] |
| C28 | same chain as C26 | 8 different operation orders (reversed, all-same, duplicated, empty, single, pairs) x `count ∈ {0,1,8,10}` | [x] |
| C29 | `arrayfunc` | A8 all-zero, all-ones, `{0,1,-1,2,-2}⁴` exhaustive (625 tuples) | [x] |
| C30 | `arrayfunc` | A8 corner matrix `{INT32_MIN, INT32_MIN+1, -2^30, -1, 0, 1, 2^30, INT32_MAX-1, INT32_MAX}⁴` (6561 tuples) — covers every overflow site of lines 162-163 and `param4/2` at `INT32_MIN` (E25) | [x] |
| C31 | `arrayfunc` | 200 000 uniform-random `(p1,p2,p3,p4)` from the fixed seed | [x] |
| C32 | `arrayfunc` | 20 000 random tuples drawn from a "small magnitude" distribution (`-1000..1000`) where no clamping occurs, so the non-saturating arithmetic path dominates | [x] |
| C33 | `arrayfunc` | odd/even/negative-odd `param4` (division truncation toward zero) × sign combinations of `param1..3` | [x] |

## Row -> test mapping

| rows | test file :: test |
|------|-------------------|
| C1..C4 | `phase_b_leaves.rs :: c1_add_operation`, `c2_multiply_operation`, `c3_subtract_operation`, `c4_modulo_operation` |
| C5, C6 | `phase_b_leaves.rs :: c5_safe_double_to_int`, `c6_compute_scaled_value` |
| C7..C13 | `phase_b_arrays.rs :: c7_init_count_zero` .. `c13_init_twice_decreasing_counts` |
| C14..C16 | `phase_b_arrays.rs :: c14_compare_all_pairs_count_ten`, `c15_compare_full_grid`, `c16_compare_huge_count` |
| C17..C20 | `phase_b_arrays.rs :: c17_weighted_sum_zero_and_one` .. `c20_weighted_sum_poisoned_prefix` |
| C21..C25 | `phase_b_arrays.rs :: c21_foreach_builtin_ops` .. `c25_foreach_repeated_passes` |
| C26..C28 | `phase_b_pipeline.rs :: c26_pipeline_replica_of_arrayfunc`, `c27_pipeline_other_counts`, `c28_pipeline_reordered_and_duplicated_ops` |
| C29..C33 | `phase_b_pipeline.rs :: c29_arrayfunc_small_exhaustive` .. `c33_arrayfunc_param4_division_truncation` |
| layout / symbols / cross-library calls | `phase_d_parity.rs` (6 tests) |

## How each row is compared

Every row asserts, through the two `.so` handles only:

1. the returned `int` is bit-identical;
2. `ResultArray::count` is identical;
3. for all 10 elements: `value`, `rank`, and `scaled` **as raw IEEE-754 bits**
   are identical (so `-0.0` vs `0.0` and NaN payload differences would fail);
4. for row C24 (the deliberate out-of-bounds walk) the entire 512-byte padded
   buffer is compared byte-for-byte.

Struct *padding* bytes (`Result` offsets 20..24, `ResultArray` 244..248) are
excluded from 1-3: C's compound-literal assignment
(`arr->data[i] = (Result){…}`) leaves them indeterminate, so they are not part of
the observable contract. `phase_d_layout_parity` pins down every non-padding
offset explicitly and additionally asserts that neither library writes past
`sizeof(ResultArray)`.

## Randomisation

All randomised rows use the SplitMix64 generator in `tests/common/mod.rs` with a
per-row fixed seed, so every run is reproducible. `Rng::interesting_i32()` mixes
uniform draws with `INT32_MIN`/`INT32_MAX`, tiny values, and values clustered
around the `INT32_MIN/2` and `INT32_MAX/2` overflow thresholds.

Approximate differential call volume per full run: ~1.1 M `arrayfunc` /
primitive invocations across the 33 rows.

## Configuration coverage

There is a single feature combination, and it is verified in **both** cargo
profiles:

| profile | Rust artifact | result |
|---------|---------------|--------|
| `dev` (debug assertions, overflow checks on) | `target/debug/libarrayfunc_lib.so` | 72/72 tests pass |
| `release` (`opt-level=3`, `panic="abort"`) | `target/release/libarrayfunc_lib.so` | 72/72 tests pass |

Run everything with `./verify_all.sh [--offline]`.
