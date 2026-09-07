# CONFIGS.md — Phase B configuration surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C actually branches on

There are **no runtime option/mode/flag setters** in the API, and no `#ifdef`,
`switch`, or `enum` anywhere in the source (verified by grep — see `ERRORS.md`).
The axes that the C code genuinely distinguishes are therefore:

1. **Hidden mutable state** — the two file-scope `static int`s
   (`global_counter`, `global_accumulator`). Three of the entry points read them
   (`complex_calc`, `process_pointer_data`, `hatch`) and three write them
   (`increment_counter`, `update_accumulator`, `hatch`). So *the same call with
   the same arguments returns different values depending on call history*. Every
   row is therefore tested both from a pristine library and from an accumulated
   state.
2. **Function-pointer dispatch** — `apply_operation(operation_func, ...)` takes
   one of three concrete callees (`add_three`, `multiply_add`, `complex_calc`);
   `hatch` also dispatches through `modifier_func`. `complex_calc` additionally
   couples dispatch to axis 1.
3. **Shift-vs-size relation** — the guards `shift_by > 0 && shift_by < size`
   (`shift_array_data`) and `shift > 0 && shift < num_records`
   (`manipulate_records`) split the input space into "shift applied" and
   "silently skipped". `manipulate_records` splits *again*, because its summation
   loop bound `num_records - shift` is **not** gated by that guard.
4. **Element count / buffer shape** — `0`, `1`, `many`, and the `malloc`-driven
   sizes in `compute_with_dynamic_memory(base, count)`; `int` arrays vs
   `DataRecord` (48-byte, `time_t`- and `char[32]`-containing) arrays.
5. **Value magnitude** — small values vs `INT_MIN`/`INT_MAX` neighbourhoods,
   which selects between plain arithmetic and two's-complement wrap in every
   arithmetic function, and in `get_time_based_value`'s `seed * 3600`.
6. **Entry-point level** — the header exports only `hatch`, but the `.so`
   exports 11 more lower-level functions. Rows exercise all 11 **directly**, and
   `hatch` as the composed pipeline, and mixed interleavings of the two.

## Rows

Every row is driven with many randomized inputs from a fixed-seed PRNG
(`SplitMix64`, seed `0x5EED_1234_ABCD_0001`), not a single hand-picked value.
Both `.so`s are always called in identical order so that axis 1 stays in lockstep.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `add_three` | pristine state; random `(a,b,c)` over full `i32` incl. boundary triples | [x] |
| 2 | `multiply_add` | pristine state; random `(a,b,c)` over full `i32` incl. boundary triples | [x] |
| 3 | `complex_calc` | `global_counter == 0`; random `(a,b,c)` full `i32` | [x] |
| 4 | `complex_calc` | `global_counter` non-zero (pre-driven by random `increment_counter` calls); random `(a,b,c)` | [x] |
| 5 | `increment_counter` | random value sequence, effect read back through `complex_calc(0,0,0)` after each call; includes accumulation past `INT_MAX` | [x] |
| 6 | `update_accumulator` | random value sequence, effect read back through `process_pointer_data(&0, 0)` after each call; includes `*2` overflow | [x] |
| 7 | `apply_operation` | `op = add_three` (same-library pointer), random `(a,b,c)` | [x] |
| 8 | `apply_operation` | `op = multiply_add` (same-library pointer), random `(a,b,c)` | [x] |
| 9 | `apply_operation` | `op = complex_calc` (same-library pointer) + `global_counter != 0`, random `(a,b,c)` | [x] |
| 10 | `apply_operation` | **cross-library** dispatch: C's `apply_operation` given Rust's `add_three`/`multiply_add` pointer and vice versa (pure callees, so results must still match) | [x] |
| 11 | `shift_array_data` | guard true: random `size ∈ 1..=64`, random `shift_by ∈ 1..size`, random `i32` array contents; full buffer compared | [x] |
| 12 | `shift_array_data` | guard true, `shift_by == size - 1` (max valid) and `shift_by == 1` (min valid), `size ∈ {2,3,64}` | [x] |
| 13 | `shift_array_data` | guard false: `shift_by ∈ {0, -1, size, size+1, INT_MIN}`; buffer must be untouched | [x] |
| 14 | `shift_array_data` | degenerate shapes `size ∈ {0, 1}` with every `shift_by ∈ {-1,0,1,2}` | [x] |
| 15 | `process_pointer_data` | `global_accumulator == 0`; random `*ptr`, random `multiplier`, full `i32` range (overflow) | [x] |
| 16 | `process_pointer_data` | `global_accumulator != 0`; random `*ptr`, `multiplier` | [x] |
| 17 | `compute_with_dynamic_memory` | `count == 0` and `count == 1`, random `base` | [x] |
| 18 | `compute_with_dynamic_memory` | `count ∈ 2..=64` random, random `base` full `i32` (sum overflow) | [x] |
| 19 | `compute_with_dynamic_memory` | large `count ∈ 1_000..=200_000` random, random `base` (heap path + heavy wrap) | [x] |
| 20 | `compute_with_dynamic_memory` | `base` at `INT_MIN`/`INT_MAX` with `count == 8` (the value `hatch` uses) | [x] |
| 21 | `get_time_based_value` | `seed == 0`; and random `seed` with `|seed| <= 596_523` (no `int` overflow in `seed*3600`) | [x] |
| 22 | `get_time_based_value` | random `seed` with `|seed| > 596_523` so `seed*3600` wraps, plus `INT_MIN`/`INT_MAX` | [x] |
| 23 | `manipulate_records` | guard true: random `num_records ∈ 2..=32`, random `shift ∈ 1..num_records`, random `id`/`value`/`timestamp`/`name` bytes; return value **and** the whole post-memmove buffer compared byte-for-byte | [x] |
| 24 | `manipulate_records` | `shift == 0` (no memmove, sums all `num_records`), random contents | [x] |
| 25 | `manipulate_records` | `shift == num_records - 1` (max valid) and `shift == 1` (min valid), `num_records ∈ {2, 5, 32}` | [x] |
| 26 | `manipulate_records` | `num_records == 5, shift == 2` — the exact shape `hatch` uses — random `value`s | [x] |
| 27 | `manipulate_records` | `value` fields near `INT_MAX`/`INT_MIN` so the running `total` wraps | [x] |
| 28 | `manipulate_records` | degenerate `num_records ∈ {0, 1}` × `shift ∈ {0, 1}` | [x] |
| 29 | `hatch` | pristine library, single call, random `(p1,p2,p3,p4)` full `i32` | [x] |
| 30 | `hatch` | pristine library, single call, boundary params: all 4 params drawn from `{0, 1, -1, INT_MIN, INT_MAX, INT_MIN+1, INT_MAX-1}` (cross-product sample) | [x] |
| 31 | `hatch` | **stateful repetition**: 200 consecutive calls with random params, every intermediate return compared (exercises `global_accumulator`'s `*2` growth to saturation and `global_counter` accumulation) | [x] |
| 32 | `hatch` | interleaved with direct `increment_counter` / `update_accumulator` calls between `hatch` invocations | [x] |
| 33 | all 12 entry points | **mixed random operation stream**: 2 000 randomly chosen calls across every exported function with random valid arguments, applied to both libraries in identical order; every return value and every mutated buffer compared | [x] |

Note on row 10: `complex_calc` is deliberately **excluded** from cross-library
dispatch, because it reads its own library's `global_counter` — a C→Rust
cross-call there would legitimately mix two independent statics and is not a
translation-correctness signal.

Note on `hatch` and `time()`: `hatch` calls `time()` (via
`get_time_based_value` and when filling `DataRecord::timestamp`), but the
returned value never reaches the result — `get_time_based_value` only uses
`difftime(current, current - seed*3600)`, which is exactly `seed*3600`
independent of the clock, and `timestamp` is never summed. So `hatch` is
deterministic and byte-comparable.

## Row → test mapping

Every row has a differential test in `tests/phase_b_valid.rs`, named
`row<NN>_...` matching the row number. All 33 pass.

Because the two `.so`s carry *mutable global state* and `dlopen` de-duplicates by
path (one copy of each library, and therefore one copy of each `static`, per test
process), the harness:

- serialises every test on a process-wide mutex, and
- zeroes both libraries' statics before each test via exported symbols only
  (`increment_counter(-counter)`, and `update_accumulator(-(2*acc))`, which
  cancels because `acc*2 + (-(acc*2)) == 0` under two's-complement wrapping),
  asserting afterwards that both really read back 0.

Without this, parallel tests interleave their mutations of the two libraries
differently and produce spurious "divergences" — which is exactly what the first
run of this suite reported before the harness was corrected.

## Beyond the table

`tests/soak.rs` adds volume and exhaustiveness on top of the per-row sampling:

| test | coverage |
|------|----------|
| `soak_mixed_stream_many_seeds` | 16 independent seeds × 20 000 mixed calls across all 12 entry points (320 000 operations) |
| `soak_shift_array_exhaustive_small_grid` | every `(size, shift_by)` in `[-2,24] × [-4,28]`, 3 random buffers each |
| `soak_manipulate_records_exhaustive_small_grid` | every `(num_records, shift)` in `[-2,12] × [-12,16]`, return value *and* full buffer bytes |
| `soak_dynamic_memory_exhaustive_small_grid` | every `count` in `[-8,72]` × 8 random bases |
| `soak_pure_ops_exhaustive_small_grid` | all 13³ small-integer triples through `add_three` / `multiply_add` / `complex_calc` |
| `soak_get_time_based_value_overflow_boundary` | ±40 around every `seed*3600` int-overflow threshold |
| `soak_hatch_grid_and_state` | all 7⁴ small-parameter `hatch` combinations from pristine state, comparing the return value **and both statics afterwards** |

## Completion gate

- [x] All 33 rows pass across randomized inputs (`cargo test --test phase_b_valid`: 33 passed).
- [x] Low-level entry points are driven directly, not only through `hatch`
      (rows 1–28 call the 11 non-header functions; rows 29–33 exercise the
      composed pipeline and mixed interleavings).
- [x] Binary/stdout comparison: **not applicable** — `c_src/CMakeLists.txt`
      declares only `add_library(... SHARED ...)` and the crate declares only a
      `cdylib`. Enforced by `phase_d_project_builds_no_binary_executable`, which
      fails if an executable target is ever added.
