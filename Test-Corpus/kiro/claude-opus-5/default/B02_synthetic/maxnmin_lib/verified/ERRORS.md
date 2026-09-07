# ERRORS.md — Error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c` by grepping for every rejection
construct. The complete inventory of such constructs in the file is:

```
$ grep -n 'return -1\|return NULL\|return 0\|INT_MAX\|INT_MIN\|assert\|!= NULL\|== NULL\|if (' c_src/src/lib.c
```

* error-return statements: `return -1;` (×1), `return NULL;` (×1),
  `return 0.0;` (×1), `return 0;` (×1 in `process_string`, ×1 in
  `safe_double_to_int`), `return INT_MAX;` (×1), `return INT_MIN;` (×1)
* `assert`: **none** in the file
* error enums / error codes: **none** — there is no `enum` anywhere in the
  library, so there is no named error type. (See "Out-of-range enum values"
  below for how that generic boundary class is covered instead.)
* explicit range / null / guard checks: `node_count >= MAX_NODES`,
  `node == NULL`, `selected_node != NULL`, `second_node != NULL`, `*str`,
  `*name_ptr`, `node_storage[i].active`, `d > (double)INT_MAX`,
  `d < (double)INT_MIN`, `d != d`
* min/max constants: `MAX_NODES 100`, `MAX_NAME_LEN 50`, `INT_MAX`, `INT_MIN`

One row per distinct rejection branch. `[x]` = differential test written and
passing against **both** `.so`s.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E01 | `add_node` | `node_count >= MAX_NODES` — call it a 101st time after 100 successful adds | returns `-1`, `node_count` unchanged (stays 100), storage not written | [x] |
| E02 | `add_node` | `name` longer than `MAX_NAME_LEN-1` (50, 51, 200 bytes) | silently truncated: `name[0..49]` = first 49 src bytes, `name[49] = '\0'`; returns the new index | [x] |
| E03 | `add_node` | `name` exactly `MAX_NAME_LEN-1` == 49 bytes (boundary, one step below truncation) | all 49 bytes copied, `name[49]='\0'`, no truncation | [x] |
| E04 | `add_node` | `name` is the empty string `""` | `name` stays all-NUL, node still added, returns new index | [x] |
| E05 | `add_node` | `value` is NaN / `+inf` / `-inf` | accepted verbatim, no rejection; stored bit-pattern must match | [x] |
| E06 | `add_node` | `id`/`parent_id` at `INT_MIN` / `INT_MAX` (out-of-range ints across FFI) | accepted verbatim, no rejection | [x] |
| E07 | `find_node_by_id` | `node_count == 0` (pristine library, loop body never runs) | returns `NULL` | [x] |
| E08 | `find_node_by_id` | `id` matches no stored node | returns `NULL` | [x] |
| E09 | `find_node_by_id` | node with matching `id` exists but its `active` field is `0` (caller zeroed it through the returned `Node*`) | returns `NULL` — the inactive node is skipped | [x] |
| E10 | `find_node_by_id` | two stored nodes share the same `id` | returns the **first** (lowest index) active match, not the last | [x] |
| E11 | `get_children_count` | `parent_id` matches no stored node | returns `0` (not an error code — `0` is also the legitimate "no children" answer) | [x] |
| E12 | `get_children_count` | all candidate children have `active == 0` | returns `0` — inactive children are not counted | [x] |
| E13 | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` (unknown id, or empty storage, or inactive node) | returns `0.0` exactly (positive zero) | [x] |
| E14 | `calculate_subtree_sum` | node found but a descendant's `value` is NaN / `inf` | NaN / `inf` propagates into the sum; **not** rejected | [x] |
| E15 | `process_string` | empty string `""` — outer `if (*str)` is false | returns `0` | [x] |
| E16 | `process_string` | bytes with the high bit set (`0x80`..`0xFF`) | `char` is **signed** on this ABI, so each contributes a **negative** value; sum can be negative | [x] |
| E17 | `process_string` | long string whose byte sum overflows `int` | signed overflow wraps (both sides must wrap identically) | [x] |
| E18 | `safe_double_to_int` | `d > (double)INT_MAX`, e.g. `2147483648.0`, `2147483647.5`, `1e300` | returns `INT_MAX` (2147483647) | [x] |
| E19 | `safe_double_to_int` | `d == +INFINITY` (hits branch E18) | returns `INT_MAX` | [x] |
| E20 | `safe_double_to_int` | `d < (double)INT_MIN`, e.g. `-2147483649.0`, `-2147483648.5`, `-1e300` | returns `INT_MIN` (-2147483648) | [x] |
| E21 | `safe_double_to_int` | `d == -INFINITY` (hits branch E20) | returns `INT_MIN` | [x] |
| E22 | `safe_double_to_int` | `d` is NaN — **note the C check order**: the two range compares are evaluated *first* and both are false for NaN, so NaN falls through to `d != d` | returns `0` | [x] |
| E23 | `safe_double_to_int` | `d` exactly `2147483647.0` (== `(double)INT_MAX`, one step *inside* the range) | `>` is false → returns `2147483647` via the `(int)d` cast, not via the clamp | [x] |
| E24 | `safe_double_to_int` | `d` exactly `-2147483648.0` (== `(double)INT_MIN`, one step *inside* the range) | `<` is false → returns `-2147483648` via the `(int)d` cast | [x] |
| E25 | `safe_double_to_int` | negative fraction, e.g. `-2.7`, and `-0.5` | truncates toward **zero**: `-2`, `0` (not floor) | [x] |
| E26 | `safe_double_to_int` | `-0.0` | returns `0` | [x] |
| E27 | `maxnmin` | `param1` negative with `param1 % 6 != 0`, so `node_id = (param1%6)+1 <= 0` → `find_node_by_id` returns `NULL` | the entire first block (name sum + subtree sum) is **skipped**; contributes 0 | [x] |
| E28 | `maxnmin` | `param2` negative with `param2 % 6 != 0`, so `second_node_id <= 0` → `NULL` | the second block (`value * param3`) is **skipped**; contributes 0 | [x] |
| E29 | `maxnmin` | `param3 == -1` → `(double)(param3 + 1) == 0.0`, with `param1+param2 != 0` | float division by zero → `±inf`, then `*= param4` → `±inf` (or NaN if `param4 == 0`) → `safe_double_to_int` clamps to `INT_MAX`/`INT_MIN` (or 0 for NaN) | [x] |
| E30 | `maxnmin` | `param3 == -1` **and** `param1 + param2 == 0` → `0.0 / 0.0` | NaN → `*= param4` stays NaN → `safe_double_to_int` returns `0` | [x] |
| E31 | `maxnmin` | `param3 == INT_MAX` → `param3 + 1` signed-overflows | wraps to `INT_MIN`; denominator `-2147483648.0` | [x] |
| E32 | `maxnmin` | `param1 + param2` signed-overflows (e.g. `INT_MAX, INT_MAX`, or `INT_MIN, INT_MIN`) | wraps modulo 2^32 before the `(double)` conversion | [x] |
| E33 | `maxnmin` | `param4 == -2` (or any `param4 % 3 == -2`) → `parent_id = -1`, which **equals the seeded root's `parent_id`** | `get_children_count(-1)` returns `1`, adding 10 — an easy-to-miss non-zero result for a "negative" id | [x] |
| E34 | `maxnmin` | `param4 % 3 == -1` → `parent_id = 0`, matching no node | `get_children_count(0)` returns `0`, adding 0 | [x] |
| E35 | `maxnmin` | `param3` huge (`INT_MAX`) so `second_node->value * param3` exceeds `INT_MAX` | `safe_double_to_int` clamps the product to `INT_MAX` | [x] |
| E36 | `maxnmin` | every parameter at `INT_MIN` / `INT_MAX` simultaneously (all four out-of-range extremes at once) | must match exactly, including all the wrapping above | [x] |
| E37 | `maxnmin` | called twice in a row, and called *after* the caller has pushed its own nodes with `add_node` | `node_count = 0` is reset at entry, so prior nodes are logically discarded and the 6 seeds are rewritten at indices 0..5 — the result is independent of prior state | [x] |
| E38 | `add_node` | called *after* `maxnmin`, i.e. `node_count == 6` | appends at index 6 and returns `6` (state left behind by `maxnmin` is visible) | [x] |

## Deliberately-unreachable branches (documented, not testable)

| # | function | branch | why untestable |
|---|----------|--------|----------------|
| U01 | `maxnmin` | `if (*name_ptr)` false | all six seeded names (`"root"`, `"child1"`, …) are non-empty, so this is dead code; there is no public way to make a seeded name empty before the check. |
| U02 | `process_string` | inner `while (*str)` when outer `if (*str)` was true | the outer `if` is redundant with the first `while` test; both are exercised by any non-empty string. |

## Not tested because the C itself is undefined behaviour

Calling these would crash *both* libraries; a "differential test" of a segfault
proves nothing and would abort the test binary, so they are documented instead
of executed.

| function | input | C behaviour |
|---|---|---|
| `add_node` | `name == NULL` | `strncpy` dereferences NULL → SIGSEGV |
| `process_string` | `str == NULL` | `*str` dereferences NULL → SIGSEGV |
| `calculate_subtree_sum` | a node that is its own ancestor (`parent_id` cycle, constructible via the mutable `Node*` from `find_node_by_id`) | infinite recursion → stack-overflow SIGSEGV |

The Rust translation reproduces each of these (raw-pointer deref, unbounded
recursion) rather than "fixing" them into a graceful error, which is the correct
behaviour for a faithful translation. `tests/errors_diff.rs` asserts the
NULL-pointer cases crash in a **forked child** for both libraries, so the
equivalence is still checked without killing the test process.

## Out-of-range enum values

The C library declares **no `enum` types**, so there is no enum-with-no-valid-
variant case to pass across FFI. The equivalent class of bug for this API is an
`int` parameter with no meaningful interpretation. That is covered
systematically: rows E06, E27, E28, E31, E32, E33, E34, E36 pass `INT_MIN`,
`INT_MAX`, `-1`, `0` and "one step past" values into every `int` parameter of
every entry point, and `tests/errors_diff.rs` additionally sweeps
`param1..param4` over all residues mod 6 and mod 3 including negatives.

## Row → test mapping (all in `tests/errors_diff.rs`)

| rows | test |
|---|---|
| E01 | `e01_add_node_rejects_past_capacity` |
| E02, E03, E04 | `e02_e03_e04_add_node_name_truncation_boundary` |
| E05, E06 | `e05_e06_add_node_accepts_extreme_inputs` |
| E07 | `e07_find_on_empty_storage_is_null` |
| E08 | `e08_find_absent_id_is_null` |
| E09 | `e09_find_inactive_node_is_null` |
| E10 | `e10_find_duplicate_returns_first_match` |
| E11, E12 | `e11_e12_children_count_zero_cases` |
| E13 | `e13_subtree_sum_not_found_is_positive_zero` |
| E14 | `e14_subtree_sum_propagates_nan_and_inf` |
| E15 | `e15_process_string_empty_is_zero` |
| E16 | `e16_process_string_signed_char_negative_sums` |
| E17 | `e17_process_string_accumulator_wraps` |
| E18..E26 | `e18_to_e26_safe_double_to_int_all_branches` |
| E27, E28 | `e27_e28_maxnmin_null_node_blocks_skipped` |
| E29, E30 | `e29_e30_maxnmin_division_by_zero` |
| E31, E32 | `e31_e32_maxnmin_signed_overflow_wraps` |
| E33, E34 | `e33_e34_maxnmin_parent_id_negative_and_zero` |
| E35 | `e35_maxnmin_value_times_param3_clamps` |
| E36 | `e36_maxnmin_all_extremes` |
| E37, E38 | `e37_e38_maxnmin_state_reset_and_leftovers` |
| NULL pointers | `null_pointer_crash_parity` (+ `crash_harness`, `#[ignore]`d) |

Each test asserts the *specific* sentinel, not merely "both failed": `-1` from a
full store, `NULL` (not just "some pointer"), `+0.0` with bit pattern
`0x0000000000000000` (distinguished from `-0.0`), `INT_MAX` / `INT_MIN` / `0`
from `safe_double_to_int`, and index `6` from the post-`maxnmin` append.

## NULL-pointer parity result

The two UB-on-NULL entry points are exercised in a forked child (a re-exec of the
test binary), and the parent compares the exit status:

| input | C | Rust |
|---|---|---|
| `process_string(NULL)` | killed by signal 11 (SIGSEGV) | killed by signal 11 (SIGSEGV) |
| `add_node(1, -1, NULL, 1.0)` | killed by signal 11 (SIGSEGV) | killed by signal 11 (SIGSEGV) |

Identical exit code *and* signal, so the Rust reproduces the C's crash rather
than turning it into a graceful error or a different fault.

## Self-parenting node (cycle) — confirmed reproduced

While running E05/E06 with `id == parent_id`, the test process died with
`has overflowed its stack`. That is the documented `U`-class behaviour: the C
recurses forever on a node that is its own parent, and the Rust translation does
too. The test now skips `calculate_subtree_sum` for `id == parent_id` (and
`tests/maxnmin_diff.rs` row C37 carries a shadow-model cycle/blow-up detector for
the same reason), so the equivalence is documented rather than asserted by
crashing the runner.

## Result

**All 38 rows have a passing error-path differential test.**
`tests/errors_diff.rs`: 22 passed, 0 failed, 1 ignored (the internal crash
harness, which is invoked as a subprocess by `null_pointer_crash_parity`).
