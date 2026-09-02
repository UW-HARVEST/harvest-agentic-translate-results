# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping **every** `return`,
`if`, and comparison against a limit constant. There are **no** `assert`s
(`grep -c assert c_src/src/lib.c` → 0), no error enums, and no
`RETURN_ERROR`-style macros in this library; rejection is expressed via
sentinel return values (`-1`, `NULL`, `0.0`, `INT_MAX`, `INT_MIN`, `0`) and via
silently-skipped blocks.

Limit constants found: `MAX_NODES = 100`, `MAX_NAME_LEN = 50`, `INT_MAX`,
`INT_MIN`.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `add_node` | `node_count >= MAX_NODES` (L45): the 101st and every later insertion | returns `-1`; `node_count` stays `100`; storage untouched | `err_01_add_node_capacity_exhausted` | [x] |
| 2  | `add_node` | boundary: exactly the 100th insertion (`node_count == 99`) — must still succeed | returns `99` | `err_02_add_node_last_slot_succeeds` | [x] |
| 3  | `add_node` | `name` longer than `MAX_NAME_LEN - 1` (49) — `strncpy` truncates, no source NUL copied | returns index; stored name = first 49 bytes, `name[49] == '\0'` | `err_03_add_node_name_overlong_truncates` | [x] |
| 4  | `add_node` | `name = NULL` → `strncpy(dst, NULL, 49)` dereferences NULL (UB) | process dies on SIGSEGV | `err_04_null_name_crashes_both` (subprocess-isolated) | [x] || 5  | `find_node_by_id` | no stored node has `id == id` (L64–69) | returns `NULL` | `err_05_find_absent_id_returns_null` | [x] |
| 6  | `find_node_by_id` | called while `node_count == 0` (empty store) — loop body never runs | returns `NULL` | `err_06_find_on_empty_store_returns_null` | [x] |
| 7  | `find_node_by_id` | a node with that `id` exists but `active == 0` (L65 `&& active`) | returns `NULL` (node is invisible) | `err_07_find_inactive_returns_null` | [x] |
| 8  | `find_node_by_id` | out-of-range/extremal ids: `INT_MIN`, `INT_MAX`, `0`, `-1` | returns `NULL` (no node uses them) | `err_08_find_extremal_ids_return_null` | [x] |
| 9  | `get_children_count` | no node has `parent_id == parent_id` | returns `0` | `err_09_children_count_no_match_zero` | [x] |
| 10 | `get_children_count` | matching children exist but all have `active == 0` (L75) | returns `0` | `err_10_children_count_all_inactive_zero` | [x] |
| 11 | `get_children_count` | empty store (`node_count == 0`) | returns `0` | `err_11_children_count_empty_store_zero` | [x] |
| 12 | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` (L84–85) | returns `0.0` (positive zero) | `err_12_subtree_sum_absent_node_zero` | [x] |
| 13 | `calculate_subtree_sum` | root exists but every child has `active == 0` (L91) | returns only the root's own `value` | `err_13_subtree_sum_inactive_children` | [x] |
| 14 | `calculate_subtree_sum` | node's `value` is NaN / ±inf — no guard, propagates | returns NaN / ±inf verbatim (bitwise compare) | `err_14_subtree_sum_nonfinite_value` | [x] |
| 15 | `process_string` | `*str == '\0'` (L102 guard false) — empty string | returns `0` | `err_15_process_string_empty_returns_zero` | [x] |
| 16 | `process_string` | `str = NULL` → `*str` dereferences NULL (UB) | process dies on SIGSEGV | `err_16_null_string_crashes_both` (subprocess-isolated) | [x] |
| 17 | `process_string` | bytes ≥ 0x80: `char` is signed on this ABI so `(int)(*str)` sign-extends negative | running total decreases; result may be negative | `err_17_process_string_high_bit_bytes_signed` | [x] |
| 18 | `process_string` | accumulator overflows `int` (very long high-value string) | wraps (gcc `-O0` two's complement) | `err_18_process_string_accumulator_overflow` | [x] |
| 19 | `safe_double_to_int` | `d > (double)INT_MAX` (L113) | returns `INT_MAX` | `err_19_sdti_above_int_max` | [x] |
| 20 | `safe_double_to_int` | `d < (double)INT_MIN` (L116) | returns `INT_MIN` | `err_20_sdti_below_int_min` | [x] |
| 21 | `safe_double_to_int` | `d != d`, i.e. NaN (L120) — reached only *after* the two range tests, both of which are false for NaN | returns `0` | `err_21_sdti_nan_returns_zero` | [x] |
| 22 | `safe_double_to_int` | `d == +INFINITY` → caught by L113 | returns `INT_MAX` | `err_22_sdti_pos_inf` | [x] |
| 23 | `safe_double_to_int` | `d == -INFINITY` → caught by L116 | returns `INT_MIN` | `err_23_sdti_neg_inf` | [x] |
| 24 | `safe_double_to_int` | one step past the range: `nextafter((double)INT_MAX, +inf)` and `nextafter((double)INT_MIN, -inf)` | `INT_MAX` / `INT_MIN` | `err_24_sdti_one_step_past_range` | [x] |
| 25 | `safe_double_to_int` | exactly on the range boundary: `(double)INT_MAX`, `(double)INT_MIN` — **not** rejected (strict `>` / `<`) | `INT_MAX` / `INT_MIN` via `(int)d`, not via the guards | `err_25_sdti_exact_boundaries` | [x] |
| 26 | `safe_double_to_int` | signalling/quiet NaN with a non-canonical payload, and `-NaN` | returns `0` for every NaN bit pattern | `err_26_sdti_nan_payloads` | [x] |
| 27 | `safe_double_to_int` | `-0.0` — passes all guards, `(int)(-0.0) == 0` | returns `0` | `err_27_sdti_negative_zero` | [x] |
| 28 | `maxnmin` | `(param1 % 6) + 1` names no node ⇒ `selected_node == NULL` (L142). C `%` truncates toward zero, so any `param1 < 0` with `param1 % 6 != 0` yields an id `<= 0`. | whole first block skipped (no name sum, no subtree sum) | `err_28_maxnmin_selected_node_null` | [x] |
| 29 | `maxnmin` | `(param2 % 6) + 1` names no node ⇒ `second_node == NULL` (L158) | second block skipped (no `value * param3` term) | `err_29_maxnmin_second_node_null` | [x] |
| 30 | `maxnmin` | `*name_ptr == '\0'` (L145) — guard is false only for an empty name; unreachable for the six hard-coded nodes but a real branch | `process_string` not called | `err_30_maxnmin_empty_name_branch` | [x] |
| 31 | `maxnmin` | `param3 == -1` ⇒ `(double)(param3 + 1) == 0.0` ⇒ division by zero | `±inf` (or NaN if numerator is 0), then `* param4`; `safe_double_to_int` maps NaN→`0`, `+inf`→`INT_MAX`, `-inf`→`INT_MIN` | `err_31_maxnmin_div_by_zero` | [x] |
| 32 | `maxnmin` | `param3 == -1` **and** `param1 + param2 == 0` ⇒ `0.0 / 0.0` = NaN | NaN → final term `0` | `err_32_maxnmin_zero_over_zero_nan` | [x] |
| 33 | `maxnmin` | `param3 == -1` and `param4 == 0` ⇒ `±inf * 0.0` = NaN | NaN → final term `0` | `err_33_maxnmin_inf_times_zero_nan` | [x] |
| 34 | `maxnmin` | `param3 == INT_MAX` ⇒ `param3 + 1` signed overflow ⇒ wraps to `INT_MIN` | denominator `-2147483648.0` | `err_34_maxnmin_param3_overflow` | [x] |
| 35 | `maxnmin` | `param1 + param2` signed overflow (e.g. both `INT_MAX`) | wraps to `-2` before the `(double)` cast | `err_35_maxnmin_sum_overflow` | [x] |
| 36 | `maxnmin` | all four params `INT_MIN` / `INT_MAX` (extremal corners) | matches C exactly | `err_36_maxnmin_extremal_corners` | [x] |
| 37 | `maxnmin` | `param4 % 3` negative ⇒ `parent_id <= 0` ⇒ `get_children_count` returns 0 | `children * 10 == 0` | `err_37_maxnmin_parent_id_nonpositive` | [x] |
| 38 | `maxnmin` | called after the store was filled to `MAX_NODES` — `maxnmin` resets `node_count = 0` first, so it always succeeds and leaves `node_count == 6` | identical result to a fresh call; store observably rebuilt | `err_38_maxnmin_resets_full_store` | [x] |
| 39 | out-of-range "enum"/int across FFI | this ABI has no C `enum` parameter; the equivalent class is an arbitrary `int` with no meaningful variant fed to every `int` parameter (`add_node` id/parent_id, `find_node_by_id`, `get_children_count`, `calculate_subtree_sum`, all four `maxnmin` params) | no validation anywhere; both sides must agree bit-for-bit on the full `i32` domain | `err_39_arbitrary_int_domain_all_entry_points` | [x] |

## Deliberately not tested (unbounded recursion, UB with no defined result)
`calculate_subtree_sum` recurses on any node whose `parent_id` equals the
current `node_id`. A self-parented node (`add_node(7, 7, …)`) or a
`parent_id`/`id` cycle recurses until the stack is exhausted in **both**
implementations. The Rust translation reproduces the structure faithfully
(no cycle detection was added). Exercising it would only crash the harness,
so it is documented rather than executed. `err_04` / `err_16` cover the two
NULL-deref UB sites in an isolated subprocess instead.

## Note on rows 4 and 16 (the two NULL-dereference sites)

`add_node`'s `strncpy` and `process_string`'s `*str` both dereference their
pointer argument with no NULL check, so `NULL` is undefined behaviour in C. The
C `.so` faults with `SIGSEGV`, and so does the **release** Rust `.so` — the
artifact that corresponds to the C shared library.

A **debug** Rust build additionally carries rustc's `-C debug-assertions` UB
checks, which notice the NULL dereference before it faults and panic; a panic
escaping an `extern "C"` function aborts, so the debug `.so` dies with
`SIGABRT` instead of `SIGSEGV`. That is the compiler's deliberate tripwire on
input that has no defined behaviour in C either, not a behavioural divergence
in the translation. `assert_deadly_signals_match` therefore requires:

- C always faults with `SIGSEGV`;
- the release Rust `.so` faults with the *same* signal as C;
- the debug Rust `.so` also dies (never returns), with `SIGABRT`.

Both are verified: `run_all_combos.sh` runs the suite against both profiles.

## Divergence found and fixed

Phase B/C found exactly one real divergence, in the floating-point accumulation
of `calculate_subtree_sum`. It is documented in `CONFIGS.md` (rows 45–47) and in
the `addsd` helper in `src/lib.rs`: gcc lowers `sum += calculate_subtree_sum(...)`
to `addsd %xmm1,%xmm0` with the **child's** value in the destination register, and
x86 `addsd` returns the destination operand when it is a NaN — so the child's NaN
sign and payload win over the accumulator's, the opposite of what `sum += child`
produces in Rust. Reachable from the public API because `add_node` accepts an
arbitrary `double`.
