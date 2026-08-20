# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Mirror of `ERRORS.md` for **valid** inputs. Rows are the pruned cross-product of
the axes the C source actually branches on.

## Axes derived from `c_src/src/lib.c`

| axis | values the C distinguishes | where |
|---|---|---|
| **A. entry point** | all 12 exported functions — the low-level leaves (`add_three`, `multiply_add`, `complex_calc`, `increment_counter`, `update_accumulator`, `process_pointer_data`, `shift_array_data`, `compute_with_dynamic_memory`, `get_time_based_value`, `manipulate_records`), the higher-order dispatcher (`apply_operation`), and the one-shot wrapper (`hatch`) | whole file |
| **B. hidden global state** | `global_counter` ∈ {0, >0, <0, near-`INT_MAX`}; `global_accumulator` ∈ {0, >0, <0, near-overflow}. This is *persistent per `.so`* and is the library's only "option/mode": it silently re-configures `complex_calc` (L56), `process_pointer_data` (L75) and `hatch` (L174). | L29–30, L36, L40, L56, L75, L174 |
| **C. call sequence / arity** | `update_accumulator` is **non-commutative** (`acc*2+v`), so the *order and count* of mutator calls is a real configuration axis: 0 calls, 1 call, N calls, interleaved with `increment_counter`, interleaved with `hatch` | L40, L128–132 |
| **D. callback identity** | `apply_operation`'s `op`: `add_three`, `multiply_add`, `complex_calc` (state-dependent), a caller-supplied external callback, a cross-library callback | L43–44, L136–143 |
| **E. buffer shape** | `size`/`num_records` ∈ {0, 1, 2, 3, small, many}; `shift_by`/`shift` ∈ {1 (min valid), mid, len-1 (max valid)} — the two `if` guards (L67, L111) and the `memmove`/`memset` split points | L66–71, L108–121 |
| **F. element data** | `int` payloads ∈ {zeros, random, `INT_MIN`, `INT_MAX`, mixed signs}; `DataRecord` fields (`id`, `value`, `timestamp`, `name[32]`) — exercises the 48-byte `repr(C)` stride | L68–69, L112–118 |
| **G. scalar magnitude** | `int` args ∈ {0, ±1, small, large, `INT_MAX`, `INT_MIN`}; for `get_time_based_value` specifically: `|seed| < 596524` (no overflow) vs `≥ 596524` (`seed*3600` wraps), and `diff/100` positive vs **negative** (truncation direction) | L48, L52, L56, L82, L101–105 |
| **H. allocation size** | `compute_with_dynamic_memory` `count` ∈ {1, 2, 8 (the value `hatch` uses), 1000, 1<<22 (16 MiB)} | L79 |
| **I. side effects** | not just the return value: `shift_array_data` mutates `arr`; `manipulate_records` `memmove`s the array. Both must be compared **byte-for-byte after the call**, plus guard bytes past the end must be unwritten. | L68–69, L112 |

There are **no** `#ifdef`s, no `enum`s, no option/flag setters and no build
options (`CMakeLists.txt` defines none; `Cargo.toml` has no `[features]`), so
axes B and C *are* the runtime "mode" surface.

## Row table

Every row is driven with **many randomized inputs** (fixed-seed SplitMix64,
`SEED = 0x5DEECE66D` — see `tests/common/mod.rs`), and both `.so`s are called
through `libloading` in the identical order so their hidden state stays in
lockstep. `[x]` = passes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `add_three` | 4096 random `(a,b,c)` over the full `i32` range | [x] |
| C2 | `add_three` | full cross product of the 11 `EDGE` scalars `{INT_MIN, INT_MIN+1, −10⁶, −1000, −1, 0, 1, 1000, 10⁶, INT_MAX−1, INT_MAX}³` = 1331 combinations | [x] |
| C3 | `multiply_add` | 4096 random `(a,b,c)` over the full `i32` range | [x] |
| C4 | `multiply_add` | degenerate multipliers `b` ∈ {0, 1, −1, 2, −2, `INT_MIN`, `INT_MAX`} × `a` ∈ `EDGE` (11) × `c` ∈ {0, ±1, `INT_MIN`, `INT_MAX`} = 385 combinations | [x] |
| C5 | `complex_calc` | **state B = pristine (counter 0)**, 1024 random `(a,b,c)` | [x] |
| C6 | `complex_calc` | **state B = counter > 0**, 1024 random `(a,b,c)` | [x] |
| C7 | `complex_calc` | **state B = counter < 0**, 1024 random `(a,b,c)` | [x] |
| C8 | `complex_calc` | **state B = counter near `INT_MAX`** (result wraps), 1024 random `(a,b,c)` | [x] |
| C9 | `increment_counter` | sequence C = 256 random **positive** deltas from pristine; effect observed via `complex_calc` | [x] |
| C10 | `increment_counter` | sequence C = 256 random **mixed-sign** deltas | [x] |
| C11 | `increment_counter` | sequence driven deliberately **past `INT_MAX`** (wrap-around) | [x] |
| C12 | `update_accumulator` | single call from pristine; effect observed via `process_pointer_data` | [x] |
| C13 | `update_accumulator` | sequence C = 256 random values (doubling ⇒ wraps every ~31 calls; **order-sensitive**) | [x] |
| C14 | `increment_counter` + `update_accumulator` | 512-step randomly **interleaved** mutator sequence (both states live at once) | [x] |
| C15 | `increment_counter`, `update_accumulator` | `unused_param` ∈ {0, 999, 888, −1, `INT_MIN`, `INT_MAX`} — must be ignored | [x] |
| C16 | `apply_operation` | `op = add_three` (D), 1024 random `(a,b,c)` | [x] |
| C17 | `apply_operation` | `op = multiply_add` (D), 1024 random `(a,b,c)` | [x] |
| C18 | `apply_operation` | `op = complex_calc` (D) with **state B = pristine** | [x] |
| C19 | `apply_operation` | `op = complex_calc` (D) with **state B ≠ 0** — state must flow through the indirect call | [x] |
| C20 | `apply_operation` | `op` = **caller-supplied external callback** (defined in the test binary) — verifies the raw fn-pointer ABI | [x] |
| C21 | `apply_operation` | `op` = **cross-library callback**: C's `apply_operation` given the Rust `.so`'s `add_three`/`complex_calc`, and vice-versa | [x] |
| C22 | `shift_array_data` | `size = 2`, `shift_by = 1` (smallest valid shift), random data | [x] |
| C23 | `shift_array_data` | `size = 3`, `shift_by` ∈ {1, 2} (= `size−1`, max valid) | [x] |
| C24 | `shift_array_data` | `size = 10`, `shift_by` ∈ {1, 5, 9}, random data — mid split | [x] |
| C25 | `shift_array_data` | `size = 1000` (many), 64 random `shift_by` ∈ 1..999, random data | [x] |
| C26 | `shift_array_data` | data = extreme payloads {all `INT_MIN`, all `INT_MAX`, all 0, alternating} × `size = 16` | [x] |
| C27 | `shift_array_data` | **guard bytes**: 64-element buffer, `size = 16`, assert elements 16..64 are unmodified (axis I) | [x] |
| C28 | `process_pointer_data` | **state B = pristine**, 1024 random `(value, multiplier)` | [x] |
| C29 | `process_pointer_data` | **state B: accumulator ≠ 0** (positive, negative, near-overflow), 1024 random pairs | [x] |
| C30 | `process_pointer_data` | `multiplier` ∈ {0, 1, −1, `INT_MIN`, `INT_MAX`} × `value` ∈ {0, ±1, `INT_MIN`, `INT_MAX`} | [x] |
| C31 | `process_pointer_data` | `ptr` = **interior** pointer (`&arr[k]`, k ∈ 0..len) of a larger array — reads exactly one element | [x] |
| C32 | `compute_with_dynamic_memory` | `count = 1` (one element), random `base` | [x] |
| C33 | `compute_with_dynamic_memory` | `count = 2`, random `base` | [x] |
| C34 | `compute_with_dynamic_memory` | `count = 8` (the value `hatch` uses), random `base` | [x] |
| C35 | `compute_with_dynamic_memory` | `count = 1000` (many), 256 random `base` incl. extremes (sum wraps) | [x] |
| C36 | `compute_with_dynamic_memory` | `count = 1<<22` (16 MiB allocation, axis H) — large-but-valid | [x] |
| C37 | `compute_with_dynamic_memory` | `count` = 128 random values in 1..4096 × random `base` | [x] |
| C38 | `get_time_based_value` | `seed = 0`, and `seed` ∈ ±1 | [x] |
| C39 | `get_time_based_value` | `|seed| < 596524` — no `int` overflow of `seed*3600`; 1024 random | [x] |
| C40 | `get_time_based_value` | `|seed| ≥ 596524` — `seed*3600` wraps; 1024 random, both signs (checks truncation-toward-zero of a negative quotient) | [x] |
| C41 | `get_time_based_value` | 4096 random seeds over the **full** `i32` range | [x] |
| C42 | `manipulate_records` | `num_records = 1`, `shift = 0`; random fields | [x] |
| C43 | `manipulate_records` | `num_records = 2`, `shift = 1` (smallest valid `memmove`); random fields | [x] |
| C44 | `manipulate_records` | `num_records = 5`, `shift = 2` (exactly what `hatch` does); random fields | [x] |
| C45 | `manipulate_records` | `num_records = 10`, `shift` ∈ {1, 5, 9 = `num−1`}; random fields | [x] |
| C46 | `manipulate_records` | `num_records = 64` (many), 64 random `shift` ∈ 1..63, all 4 struct fields random (48-byte stride / ABI check) | [x] |
| C47 | `manipulate_records` | **post-call byte comparison** of the whole `48*n`-byte array (`memmove` side effect, axis I) + guard records past `num_records` unmodified | [x] |
| C48 | `manipulate_records` | `.value` fields near `INT_MAX`/`INT_MIN` so `total` wraps; `num_records = 32`, `shift = 3` | [x] |
| C49 | `hatch` | first call from **pristine** state, params = `(1,2,3,4)` and 3 other fixed vectors | [x] |
| C50 | `hatch` | 512 random `(p1,p2,p3,p4)` over the full `i32` range (state accumulates across calls — axis C) | [x] |
| C51 | `hatch` | all params 0; then all params ±1 | [x] |
| C52 | `hatch` | params ∈ {`INT_MIN`, `INT_MAX`}⁴ (16 combinations) | [x] |
| C53 | `hatch` | 20 **repeated** calls with the same params — accumulator doubles each time, so every call must differ and still match | [x] |
| C54 | `hatch` + mutators | `hatch` **interleaved** with direct `increment_counter`/`update_accumulator`/`complex_calc` calls (shared-state coupling, axis B×C) | [x] |
| C55 | `hatch` | called after state has been driven to overflow (counter ≈ `INT_MAX`, accumulator wrapped) | [x] |
| C56 | all 12 | **full-pipeline replay**: 2000-step randomized sequence mixing *every* entry point and both state axes, comparing every return value and every buffer byte at every step | [x] |

## How each row is checked

`tests/valid_paths.rs` runs every row inside one `#[test]` (so the two `.so`s'
hidden `static` state advances in a deterministic order) and, per call, asserts a
**triple** equality:

```
C .so return value  ==  Rust .so return value  ==  independent model of the C source
```

The third term (`tests/common/mod.rs`: `model_gtbv`, `model_compute`,
`model_manipulate`, `model_shift`, `model_hatch`, plus a shadow copy of
`global_counter` / `global_accumulator`) is what makes the comparison
non-vacuous: "both libraries agree on a wrong value" cannot pass.

For the buffer-mutating entry points the **post-call byte image** is compared as
well, including guard elements/records placed past the requested length, so a
too-long `memmove`/`memset` is caught even when the return value is unaffected.

The hidden state is driven to exact values by inverting the mutators:

* `set_counter(t)` → `increment_counter(t − counter)`
* `set_accum(t)` → `update_accumulator(t − 2·accum)`  (since `acc' = 2·acc + v`)

## Results

```
$ cargo test --test valid_paths -- --nocapture
CONFIGS.md: all 56 rows exercised
test phase_b_all_config_rows ... ok
```

* **56/56 rows pass** in the debug profile and in the release profile, under
  `--no-default-features` and under the default feature set (the only two, since
  no `[features]` exist).
* `tests/doc_coverage.rs::configs_md_rows_match_the_tests` asserts that the row
  ids in this file are *exactly* `CONFIG_ROWS` in `tests/common/mod.rs`, and
  `configs_and_errors_rows_are_contiguous` asserts `C1..=C56` with no gaps — so a
  row cannot be silently dropped from either side.
* Suite adequacy is demonstrated by `mutation_check.sh`: **22/22** injected
  divergences in `src/lib.rs` are detected (see `ERRORS.md` → *Mutation
  adequacy* for the one equivalent mutant that was analysed and replaced).
