# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`. There are **no**
`#ifdef`s, no compile-time options, no runtime global state and no init/teardown
in the C (`grep -c '#ifdef\|#if \|static ' c_src/src/lib.c` → 0), and the Rust
`Cargo.toml` declares **no `[features]`** — so there is exactly **one** feature
combination (`--no-default-features` and default are identical; verified in
Phase D).

The axes are therefore purely *data* axes:

| axis | values the C distinguishes |
|------|----------------------------|
| **A. entry point** | 11 external functions; the low-level ones (`add/multiply/subtract/modulo_operation`, `safe_double_to_int`) → the mid-level ones (`compute_scaled_value`, `init_result_array`, `compare_results_in_array`) → the composed ones (`process_with_foreach`, `compute_weighted_sum`) → the one-shot wrapper (`arrayfunc`) |
| **B. `operation_func` selected** | 4 built-in ops (`add`, `multiply`, `subtract`, `modulo`) + an arbitrary caller-supplied C or Rust function pointer |
| **C. `ResultArray.count` shape** | `0` (empty) / `1` (one) / `2..9` (many) / `10` (full — the clamp boundary) / `>10` passed to `init_result_array` (clamped) / negative (accepted by the clamp) |
| **D. `init_result_array` `count` vs. array capacity** | `count < 10`, `count == 10`, `count > 10`, `count <= 0` |
| **E. numeric magnitude of element values** | small (no overflow) / large enough that `*1.5`, `*0.75`, `*weight*0.8` leave int range / `INT32_MIN` / `INT32_MAX` / `0` / negative |
| **F. `safe_double_to_int` input class** | in-range positive / in-range negative / `>= INT32_MAX` / `<= INT32_MIN` / NaN / `±INF` / `±0.0` / subnormal / fractional (truncation direction) |
| **G. `compare_results_in_array` index pair** | `idx1 < idx2` / `idx1 > idx2` / `idx1 == idx2` / either `>= count` / either negative |
| **H. pipeline composition** | `init` → `process_with_foreach` applied 1×, 2×, 3×, 4× in a row (state carries over: each pass rewrites `value` from `safe_double_to_int(op(...)*0.75)`) → `compute_weighted_sum` → `compare_*` chain, i.e. exactly what `arrayfunc` does, but driven manually so intermediate `ResultArray` bytes are compared too |
| **I. observable output** | return value **and** the full 248-byte `ResultArray` (all 10 `Result` slots + `count`), compared byte-for-byte incl. the `double scaled` bit patterns and padding-free layout |

Every row is exercised with **many randomized inputs** (`SplitMix64`, fixed seed
`0x5EED_1234_ABCD_EF01`), not a single hand-picked value, and both libraries are
called only through `libloading` symbols from their `.so` files.

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `add_operation` | 20 000 random `(a,b)` incl. `INT32_MIN/MAX/0/±1` boundary sweep; `unused1/unused2` randomized to prove they are ignored | `cfg_c1_add_operation` | [x] |
| C2 | `multiply_operation` | 20 000 random `(a,b)`, overflow-heavy (large × large) + boundary sweep | `cfg_c2_multiply_operation` | [x] |
| C3 | `subtract_operation` | 20 000 random `(a,b)` + boundary sweep | `cfg_c3_subtract_operation` | [x] |
| C4 | `modulo_operation` | 20 000 random `(a,b)` with `b` forced to 0 in ~1/8 of cases, both signs, `INT32_MIN % -1` | `cfg_c4_modulo_operation` | [x] |
| C5 | `safe_double_to_int` | 30 000 random `f64` bit patterns (uniform over all 64-bit patterns → NaN/INF/subnormal appear naturally) | `cfg_c5_sdti_random_bits` | [x] |
| C6 | `safe_double_to_int` | axis F exhaustively: the exact clamp boundaries `±2147483647.0`, `±2147483648.0`, `nextafter` on both sides, `±INF`, quiet/negative NaN, `±0.0`, subnormals, `±0.5`, `±1.5` | `cfg_c6_sdti_boundaries` | [x] |
| C7 | `compute_scaled_value` | 20 000 random `(int base, double scale)` with `scale` drawn from a mix of small factors, huge factors, NaN, `±INF`, `0.0` | `cfg_c7_compute_scaled_value` | [x] |
| C8 | `init_result_array` | `count` in `0..=12` × random `values[]`; whole 248-byte `ResultArray` compared (both sides pre-filled with the same non-zero poison so untouched slots are diffed too) | `cfg_c8_init_count_sweep` | [x] |
| C9 | `init_result_array` | axis D boundary: `count == 9, 10, 11` with `values[]` long enough; asserts only 10 slots are written and `count` is clamped | `cfg_c9_init_clamp_boundary` | [x] |
| C10 | `init_result_array` | `count` random in `1..=10` with extreme values (`INT32_MIN/MAX`) so `value*1.5` produces large/inexact doubles | `cfg_c10_init_extreme_values` | [x] |
| C11 | `compare_results_in_array` | random `count` in `0..=10` × all index pairs `idx1,idx2 ∈ -2..=12` (axis G cross-product), 200 random arrays | `cfg_c11_compare_index_matrix` | [x] |
| C12 | `process_with_foreach` (low-level, direct) | `op = add_operation` from the **same** `.so` under test; `count` in `0..=10` × random values; return value + full struct compared | `cfg_c12_foreach_add` | [x] |
| C13 | `process_with_foreach` | `op = multiply_operation` (overflow-prone), `count` in `0..=10` × random values | `cfg_c13_foreach_multiply` | [x] |
| C14 | `process_with_foreach` | `op = subtract_operation`, `count` in `0..=10` × random values | `cfg_c14_foreach_subtract` | [x] |
| C15 | `process_with_foreach` | `op = modulo_operation` (hits the `b==0` guard whenever `rank == 0`), `count` in `0..=10` × random values | `cfg_c15_foreach_modulo` | [x] |
| C16 | `process_with_foreach` | `op` = **caller-supplied Rust `extern "C"` closure-free fn** passed into *both* libraries (proves the C and Rust call the pointer with identical arguments); the shim records `(a,b,unused1,unused2)` per call and the two call traces are compared | `cfg_c16_foreach_custom_op_trace` | [x] |
| C17 | `process_with_foreach` | repeated application, axis H: same array run through the same `op` 1,2,3,4 times, comparing the struct after **each** pass | `cfg_c17_foreach_repeated_passes` | [x] |
| C18 | `compute_weighted_sum` | `count` in `0..=10` × random values incl. extremes (exercises weight `1` at `i==0` and weight `i` elsewhere, plus per-term clamping) | `cfg_c18_weighted_sum_sweep` | [x] |
| C19 | full manual pipeline (low-level entry points composed) | `init_result_array(count)` → `process_with_foreach` × all 4 ops in order → `compute_weighted_sum` → `compare_results_in_array` chain; `count` in `0..=10`, random values; struct compared after every stage | `cfg_c19_manual_pipeline` | [x] |
| C20 | full manual pipeline, permuted ops | same as C19 but with a random permutation of the 4 ops (and repeats), 300 random configurations — the composed-pipeline order axis | `cfg_c20_pipeline_op_permutations` | [x] |
| C21 | `arrayfunc` (one-shot wrapper) | 100 000 random `(param1..param4)` over the full `int` range | `cfg_c21_arrayfunc_random` | [x] |
| C22 | `arrayfunc` | small-magnitude grid `-4..=4` on all four params (6561 cases, exhaustive) — the value-dependent paths `modulo` `b==0`, negative `/2` truncation | `cfg_c22_arrayfunc_small_grid` | [x] |
| C23 | `arrayfunc` | boundary sweep: every param independently set to `INT32_MIN`, `INT32_MIN+1`, `-1`, `0`, `1`, `INT32_MAX-1`, `INT32_MAX` (cross-product 7^4 = 2401) | `cfg_c23_arrayfunc_boundary_grid` | [x] |
| C24 | struct/ABI shape | `sizeof`/offsets of `Result` (24; 0/8/16) and `ResultArray` (248; 0/240) inferred through the FFI by having C `init_result_array` write a Rust-declared struct and vice-versa | `cfg_c24_struct_layout_crosswrite` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. There is no driver binary, so the
"compare stdout byte-for-byte" gate is not applicable (`grep -c add_executable
c_src/CMakeLists.txt` → 0).
