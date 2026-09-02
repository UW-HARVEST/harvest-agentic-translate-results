# CONFIGS.md — Phase B configuration surface (valid inputs)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**A. Runtime options / modes.** The library exposes no init/option struct and no
global state. The one caller-settable "mode" is the `operation_func` callback:

| axis | values the C distinguishes |
|------|----------------------------|
| `op` passed to `process_with_foreach` | `add_operation`, `multiply_operation`, `subtract_operation`, `modulo_operation` (the exact 4 in `arrayfunc`'s `operations[]`), plus any caller-supplied callback (the type is a raw function pointer) |
| `modulo_operation` internal mode | `b == 0` (early `0`) vs `b != 0` (`idiv`) |
| `safe_double_to_int` internal mode | upper clamp / lower clamp / NaN / truncate — 4 branches, tested in that order |
| `compare_results_in_array` internal mode | guard-reject / `<` / `>` / `==` |
| `compute_weighted_sum` internal mode | `i == 0` (`weight = 1`) vs `i > 0` (`weight = i`) |

**B. Input shapes the code special-cases.**

| axis | values |
|------|--------|
| `ResultArray::count` | `0`, `1`, `2`, `9`, `10`, and (via `init_result_array`) requested `count` `> 10` clamped to `10` |
| `int` element values | `0`, `1`, `-1`, small +/-, `INT_MAX`, `INT_MIN`, `INT_MAX/2`, random 32-bit |
| `double` inputs | `+/-0.0`, fractional `<1`, negative fractional (truncation toward zero), exact integers, `2147483646.x`, `+/-2147483647/8` boundary, `nextafter` either side, `1e300`, `+/-INFINITY`, `NaN`, subnormal (`5e-324`) |
| index pairs for `compare_results_in_array` | `(i, i)`, `(i, j) i<j`, `(i, j) i>j`, in-range vs at-`count` vs past-`count`, negative |
| number of `process_with_foreach` passes | 1 pass, and the 4 chained passes `arrayfunc` performs (state carries over between passes — `value` is overwritten each pass) |
| `arrayfunc` params | all-zero, all-one, mixed sign, `INT_MAX`/`INT_MIN` in each of the 4 slots, random 32-bit quadruples |

**C. Full set of public entry points** (all 11 `T` symbols, not just the
`arrayfunc` convenience wrapper in `lib.h`): `add_operation`,
`multiply_operation`, `subtract_operation`, `modulo_operation`,
`safe_double_to_int`, `compute_scaled_value`, `compare_results_in_array`,
`init_result_array`, `process_with_foreach`, `compute_weighted_sum`,
`arrayfunc`.

**D. Compile-time configuration.** None on either side: `lib.c` has zero
`#ifdef`s and `Cargo.toml` has no `[features]` table. The default build is the
only build. (Enumerated and looped over by `run_verification.sh`.)

## Rows — one per meaningful combination the C treats differently

Every row is driven with **many randomized inputs** from a fixed-seed PRNG
(`splitmix64`, seed `0x5EED_1234_ABCD_F00D`), not a single hand-picked value,
and both `.so`s are compared byte-for-byte (all outputs are `int`, plus the
`ResultArray` bytes for the mutating entry points).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `add_operation` | random `(a,b)` over full `i32` range incl. overflow pairs; `unused1/2` random garbage (must be ignored) | [x] |
| C2 | `multiply_operation` | random `(a,b)` incl. overflowing products, `0`, `+/-1`, `INT_MIN*-1` | [x] |
| C3 | `subtract_operation` | random `(a,b)` incl. `INT_MIN - 1`, `INT_MAX - INT_MIN` | [x] |
| C4 | `modulo_operation` | random `(a,b)`, `b != 0`, `!(a==INT_MIN && b==-1)`; all four sign quadrants | [x] |
| C5 | `modulo_operation` | `b == 0` branch, random `a` | [x] |
| C6 | `safe_double_to_int` | random doubles strictly inside `(INT_MIN, INT_MAX)` → truncation branch, incl. negative fractions | [x] |
| C7 | `safe_double_to_int` | random doubles `>= 2147483647.0` → upper-clamp branch (+`INFINITY`) | [x] |
| C8 | `safe_double_to_int` | random doubles `<= -2147483648.0` → lower-clamp branch (+`-INFINITY`) | [x] |
| C9 | `safe_double_to_int` | NaN branch: random NaN payloads/signs, plus `0.0/0.0` | [x] |
| C10 | `safe_double_to_int` | random raw `u64` bit patterns reinterpreted as `f64` — hits all 4 branches incl. subnormals/inf/NaN without bias | [x] |
| C11 | `compute_scaled_value` | random `base` x random in-range `scale_factor` → truncation branch | [x] |
| C12 | `compute_scaled_value` | random `base` x extreme `scale_factor` (`+/-1e300`, `+/-inf`, `0.0`, `NaN`, subnormal) → clamp/NaN branches | [x] |
| C13 | `init_result_array` | random `values[]`, `count` in `0..=10`; compare full 248-byte `ResultArray` image | [x] |
| C14 | `init_result_array` | `count > 10` (11, 17, 64, `INT_MAX`) with a >=10-element `values[]` → clamp path; compare full struct image | [x] |
| C15 | `init_result_array` | `count = 1` and `count = 2` (one / few boundary) with `INT_MIN`/`INT_MAX` values (checks `value*1.5` double math) | [x] |
| C16 | `compare_results_in_array` | in-range `(idx1, idx2)` random pairs over `count` in `1..=10`, covering `<`, `>`, `==` branches | [x] |
| C17 | `compare_results_in_array` | `count` in `0..=10` x indices in `0..count+2` — the exhaustive cross-product of guard vs. address-compare | [x] |
| C18 | `process_with_foreach` | 1 pass, `op = add_operation`, random array of random `count` in `0..=10`; compare return value **and** mutated struct | [x] |
| C19 | `process_with_foreach` | 1 pass, `op = multiply_operation`, same shapes | [x] |
| C20 | `process_with_foreach` | 1 pass, `op = subtract_operation`, same shapes | [x] |
| C21 | `process_with_foreach` | 1 pass, `op = modulo_operation` — `b` is `rank`, so `rank == 0` takes the `b==0` early return and `rank>0` the `idiv` path | [x] |
| C22 | `process_with_foreach` | 4 chained passes in `arrayfunc`'s order (add, mul, sub, mod) on one array — state carried between passes, i.e. the composed pipeline | [x] |
| C23 | `process_with_foreach` | arbitrary caller callback (not one of the 4): constant `INT_MAX`, constant `INT_MIN`, identity-on-rank — exercises the clamp inside the loop | [x] |
| C24 | `compute_weighted_sum` | random arrays, `count` in `0..=10`; covers `weight=1` at `i=0` and `weight=i` after | [x] |
| C25 | `compute_weighted_sum` | saturating shapes: all `INT_MAX`, all `INT_MIN`, alternating extremes, `count = 10` (max weight 9) | [x] |
| C26 | `compute_weighted_sum` | called **after** `process_with_foreach` (values already rewritten by the 0.75 scaling) — the composed order `arrayfunc` uses | [x] |
| C27 | `arrayfunc` | random `(p1,p2,p3,p4)` quadruples over the full `i32` range (bulk property test, 20000 cases) | [x] |
| C28 | `arrayfunc` | small-magnitude quadruples `-4..=4` exhaustive-ish sweep (dense low-value coverage where `%` and `/` behave interestingly) | [x] |
| C29 | `arrayfunc` | boundary quadruples: each slot independently set to `INT_MIN`, `INT_MAX`, `0`, `-1`, `1` while others vary (cross-product of the 5 boundary values over 4 slots = 625 cases) | [x] |
| C30 | `arrayfunc` | quadruples engineered so `param4/2 + 1`, `param1+param2`, `param2-param3`, `param3*2` each overflow | [x] |
| C31 | full manual pipeline | drive the low-level API in `arrayfunc`'s exact sequence (`init_result_array` -> 4x `process_with_foreach` -> `compute_weighted_sum` -> `count-1` x `compare_results_in_array` -> `safe_double_to_int(result*0.333)`) and assert the hand-composed result equals **both** `.so`s' `arrayfunc`, and that the intermediate structs match byte-for-byte at every stage | [x] |
| C32 | struct ABI | `Result`/`ResultArray` size, align and field offsets identical across the FFI boundary (verified by round-tripping a struct filled by C through Rust and vice versa) | [x] |

---

## Evidence that these tests are not vacuous

Two independent guards, both enforced at test time:

1. **Stale-artifact guard** (`tests/common/mod.rs::assert_so_is_fresh`). `cargo test`
   does *not* rebuild a `crate-type = ["cdylib"]` artifact, so without this guard
   the whole suite could pass while comparing C against an old Rust `.so`. This
   actually happened during verification and was caught by mutation testing (see
   below); every test now refuses to run against a `.so` older than `src/`.
2. **Profile-provenance guard**
   (`tests/symbol_parity.rs::loaded_rust_so_belongs_to_this_profile`). Asserts the
   loaded `.so` sits in the same `target/<profile>/` directory as the test binary,
   so a `--release` run cannot silently exercise the debug artifact or vice versa.

### Mutation testing

22 mutants were injected into `translation/src/lib.rs`, rebuilt, and run against
the full suite. A mutant counts as killed if `cargo test` exits non-zero (an
assertion failure *or* a crashed test binary).

**Killed — 16:**
`cmp_swap_sign` (8 tests), `rank_off_by_one` (17), `weighted_ge` (3),
`weight0_is_zero` (3), `cmp_adds_lower_bound` (1, E11), `foreach_ne_to_lt` (1, E18),
`mod_zero_returns_one` (8), `mod_euclid` (8), `sdti_nan_returns_one` (3),
`scaled_1_5_to_1_4` (12), `foreach_0_75_to_0_7` (13), `final_scale_0_333` (5),
`weighted_0_8_to_0_9` (9), `foreach_total_order` (12),
`arrayfunc_drop_plus1` (5), `arrayfunc_p3_times3` (5), `init_clamp_11`
(crashes the harness via the resulting overflow).

**Survived — 6, each provably semantically equivalent, i.e. not a coverage gap:**

| mutant | why it cannot change behaviour |
|--------|--------------------------------|
| `count < 10` -> `count <= 10` | differs only at `count == 10`, where both yield `10` |
| `d >= INT_MAX` -> `d > INT_MAX` | differs only at `d == 2147483647.0`, which then truncates to `2147483647 == INT_MAX` |
| `d <= INT_MIN` -> `d < INT_MIN` | differs only at `d == -2147483648.0`, which then truncates to `-2147483648 == INT_MIN` |
| NaN check moved before the clamps | NaN fails both relational tests, so the order is unobservable |
| `i < arr.count - 1` -> `i < arr.count` | the extra iteration calls `compare_results_in_array(arr, 7, 8)` with `count == 8`; the `idx2 >= count` guard returns `0`, adding nothing |
| `*values.offset(i)` -> `*values.offset(i) + 0` | no-op control mutant |
