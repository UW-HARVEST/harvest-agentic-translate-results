# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

**Runtime option/mode flags:** none. There is no init/config struct, no global
state, no setter, and no `#ifdef` in the source. The only `#include`-guarded
comment is on `<limits.h>`. So the configuration surface is entirely
*(entry point) × (input shape)*.

**Public entry points (all 6 exported symbols, lowest-level first):**

1. `safe_double_to_int(double)` — leaf
2. `process_array_reverse(int *end, int count)` — leaf
3. `switch_fallthrough_calculator(int value, int operation)` — leaf
4. `foreach_sum(int *array, int count)` — leaf
5. `allocate_and_compute(int size, double multiplier)` — calls (1)
6. `fallcalc(int, int, int, int)` — the composed pipeline: calls (4), (2), (3), (1), (5)

**Input-shape axes the code special-cases:**

- `safe_double_to_int`: NaN / ±inf / `>= 2^31-1` / `<= -2^31` / in-range-negative /
  in-range-positive / zero-ish. (4 explicit branches + fallthrough.)
- `process_array_reverse`: `count` = 0 / 1 / many; descending pointer walk; sum
  overflow.
- `switch_fallthrough_calculator`: `operation` ∈ {0,1,2,3,4,default}; two distinct
  fallthrough chains (0→1→2 with `break`, 3→4 with `break`); `value` sign and
  magnitude (`*8`, `*3`, `+128`, `+64`, `&511` overflow behaviour).
- `foreach_sum`: `count` = 0 / 1 / many; `FOREACH` macro's `keep` toggling.
- `allocate_and_compute`: `size` negative / 0 / 1 / many / large; `multiplier`
  finite / 0 / negative / ±inf / NaN / extreme; sum saturation.
- `fallcalc`: `param3 % 5` selects the switch case (5 valid + negative→default);
  `param4 % 10 + 1` selects the inner alloc size (negative / 0 / positive);
  `param3 > 128` toggles the `|= 0200` flag; each param's sign and magnitude
  drive signed-overflow wrap in `param1 * 0100`, `(i+1)*8 + param1`, and the
  final `& 0777`.

## Configuration rows

Every row is driven with **many randomized inputs** (fixed seed, SplitMix64) in
`translation/tests/differential.rs`, comparing the C `.so` and the Rust `.so`
through `libloading`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| C1  | `safe_double_to_int` | in-range positive finite, fractional (truncation toward zero) | [x] |
| C2  | `safe_double_to_int` | in-range negative finite, fractional (truncation toward zero) | [x] |
| C3  | `safe_double_to_int` | zeros & subnormals: `0.0`, `-0.0`, `±MIN_POSITIVE`, `±5e-324` | [x] |
| C4  | `safe_double_to_int` | exact clamp boundaries and their `nextafter` neighbours on both sides of `±2^31` | [x] |
| C5  | `safe_double_to_int` | out-of-range finite magnitudes (`±1e10 … ±DBL_MAX`) | [x] |
| C6  | `safe_double_to_int` | non-finite: `+inf`, `-inf`, `NaN`, `-NaN`, signalling-NaN bit pattern | [x] |
| C7  | `safe_double_to_int` | uniformly random 64-bit bit patterns reinterpreted as `double` (hits all branches incl. exotic NaNs) | [x] |
| C8  | `process_array_reverse` | `count = 0`, `end` pointing anywhere (incl. `NULL`) | [x] |
| C9  | `process_array_reverse` | `count = 1`, single random element | [x] |
| C10 | `process_array_reverse` | `count = 2..64` random elements, `end` = last element of buffer (normal reverse walk) | [x] |
| C11 | `process_array_reverse` | `count = n` but `end` = interior element so the walk covers a random sub-window (low-level use, not just "end of buffer") | [x] |
| C12 | `process_array_reverse` | elements chosen to overflow the `int` accumulator repeatedly (`±INT_MAX/2`-scale values) | [x] |
| C13 | `process_array_reverse` | `count < 0` | [x] |
| C14 | `foreach_sum` | `count = 0` (incl. `NULL` array) | [x] |
| C15 | `foreach_sum` | `count = 1` | [x] |
| C16 | `foreach_sum` | `count = 2..64` random elements | [x] |
| C17 | `foreach_sum` | elements chosen to overflow the `int` accumulator | [x] |
| C18 | `foreach_sum` | `count < 0` | [x] |
| C19 | `foreach_sum` vs `process_array_reverse` | same buffer driven through both (order-independence of the two summations) | [x] |
| C20 | `switch_fallthrough_calculator` | `operation = 0` (chain `*8` → `+128` → `&511`), random `value` incl. overflow-inducing magnitudes | [x] |
| C21 | `switch_fallthrough_calculator` | `operation = 1` (`+128` → `&511`) | [x] |
| C22 | `switch_fallthrough_calculator` | `operation = 2` (`&511` only) — includes negative `value` (implementation-defined-looking `&` on negatives) | [x] |
| C23 | `switch_fallthrough_calculator` | `operation = 3` (`*3` → `+64`, no mask) — result may exceed 511 and may overflow | [x] |
| C24 | `switch_fallthrough_calculator` | `operation = 4` (`+64` only, no mask) | [x] |
| C25 | `switch_fallthrough_calculator` | `operation` = random out-of-range int (default branch) | [x] |
| C26 | `allocate_and_compute` | `size = 0`, random `multiplier` | [x] |
| C27 | `allocate_and_compute` | `size = 1`, random `multiplier` | [x] |
| C28 | `allocate_and_compute` | `size = 2..64` (typical), random finite `multiplier` | [x] |
| C29 | `allocate_and_compute` | `size` large enough that `sum` saturates to `INT_MAX` (`size ≈ 10^4`, `multiplier > 0`) | [x] |
| C30 | `allocate_and_compute` | `size` large, `multiplier < 0` ⇒ saturates to `INT_MIN` | [x] |
| C31 | `allocate_and_compute` | `multiplier = 0.0` / `-0.0` ⇒ `sum = 0` | [x] |
| C32 | `allocate_and_compute` | `multiplier = ±inf` ⇒ element 0 yields `0 * NaN` ⇒ `sum = NaN` ⇒ `0` | [x] |
| C33 | `allocate_and_compute` | `multiplier = NaN` ⇒ `0` | [x] |
| C34 | `allocate_and_compute` | `multiplier` extreme finite (`±1e300`) ⇒ `sum` overflows to `±inf` | [x] |
| C35 | `allocate_and_compute` | `size < 0` (malloc failure path) with random `multiplier` | [x] |
| C36 | `fallcalc` | `param3 % 5 == 0` (switch case 0), other params random | [x] |
| C37 | `fallcalc` | `param3 % 5 == 1` | [x] |
| C38 | `fallcalc` | `param3 % 5 == 2` | [x] |
| C39 | `fallcalc` | `param3 % 5 == 3` | [x] |
| C40 | `fallcalc` | `param3 % 5 == 4` | [x] |
| C41 | `fallcalc` | `param3 < 0` so `param3 % 5 ∈ {-4..-1}` ⇒ switch default | [x] |
| C42 | `fallcalc` | `param4 % 10 + 1 > 0` (inner alloc succeeds) | [x] |
| C43 | `fallcalc` | `param4 % 10 + 1 == 0` (i.e. `param4 % 10 == -1`) ⇒ inner `malloc(0)` ⇒ `0` | [x] |
| C44 | `fallcalc` | `param4 % 10 + 1 < 0` ⇒ inner malloc failure ⇒ `-1` contribution | [x] |
| C45 | `fallcalc` | `param3 > 128` ⇒ `result \|= 0200` taken | [x] |
| C46 | `fallcalc` | `param3 <= 128` ⇒ flag not taken (incl. `param3 == 128` exactly) | [x] |
| C47 | `fallcalc` | `param1` magnitude large enough to overflow `param1 * 0100` and `(i+1)*8 + param1` | [x] |
| C48 | `fallcalc` | `param2` magnitude large enough to overflow `base_value` and the `*8`/`*3` switch chains | [x] |
| C49 | `fallcalc` | `floating_calc` saturates (`param1`/`param2`/`param3` near `INT_MIN`/`INT_MAX`) | [x] |
| C50 | `fallcalc` | full corner grid: every param independently ∈ {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `128`, `129`, `INT_MAX-1`, `INT_MAX`} | [x] |
| C51 | `fallcalc` | uniformly random 32-bit values in all four params (broad property test) | [x] |
| C52 | `fallcalc` | small-magnitude random params (dense coverage of the ordinary path) | [x] |
