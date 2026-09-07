# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The C source contains **no**
`assert`, no `RETURN_ERROR`-style macro, no `errno` use, no `STATUS_ERROR` /
`STATUS_WARNING` return path (both enumerators are defined but never assigned —
only `STATUS_SUCCESS` is ever stored), and no `return -1`. The complete set of
rejection / early-out constructs found by grepping the source is:

```
lib.c:52   char valid = op_char && (op_char >= '1' && op_char <= '5');   // 3 rejecting conditions
lib.c:75   if (b == 0) { return 0; }        // divide_operation
lib.c:82   if (b == 0) { return 0; }        // modulo_operation
lib.c:99   default: return add_operation;   // select_operation fallback
lib.c:116  calloc(count, sizeof(...))       // may return NULL (unchecked by C)
lib.c:123  if (*history == NULL) { ... }    // lazy-allocate branch
lib.c:128  if (*history_count < 10) { ... } // silently drops the record when full
lib.c:143  if (!is_valid) { validation_char = '1'; }  // mathop's invalid-input fallback
```

Constants that act as limits: the literal `10` (history capacity, appears twice:
`allocate_results(10)` and `*history_count < 10`), the literal `128` (`param1 %
128` in `mathop`), the literal `5` (`% 5` operation wrap, and `'5'` upper bound),
and `sizeof(ComputationResult)` = 24.

## Table

Every row has a differential test that constructs the exact condition, calls
**both** `.so` files through `libloading`, and asserts the same error code /
sentinel / termination signal — not merely "both failed".

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `is_valid_operation` | `op_char == 0` (NUL) — first operand of `&&` is false | returns `false` (byte `0`) | `err01_is_valid_operation_nul` | [x] |
| 2  | `is_valid_operation` | `op_char != 0` and `op_char < '1'` — all of `1..=48` | returns `false` (byte `0`) | `err02_is_valid_operation_below_range` | [x] |
| 3  | `is_valid_operation` | `op_char > '5'` — all of `54..=127` | returns `false` (byte `0`) | `err03_is_valid_operation_above_range` | [x] |
| 4  | `is_valid_operation` | `op_char < 0` (signed `char`) — all of `-128..=-1` | returns `false` (byte `0`) | `err04_is_valid_operation_negative_char` | [x] |
| 5  | `divide_operation` | `b == 0`, every boundary `a` + 2000 random `a` | returns the `0` sentinel, no trap | `err05_divide_by_zero_returns_zero` | [x] |
| 6  | `modulo_operation` | `b == 0`, every boundary `a` + 2000 random `a` | returns the `0` sentinel, no trap | `err06_modulo_by_zero_returns_zero` | [x] |
| 7  | `divide_operation` | `a == INT_MIN && b == -1` | `idiv` overflow → **SIGFPE** (UB in C). Compared out-of-process via `fork()`; both sides must raise signal 8. | `err07_divide_int_min_by_minus_one_traps_identically` | [x] |
| 8  | `modulo_operation` | `a == INT_MIN && b == -1` | same as #7 | `err08_modulo_int_min_by_minus_one_traps_identically` | [x] |
| 9  | `select_operation` | `op` has no valid enumerator: `0`, `6`, `-1`, `INT_MIN`, `INT_MAX`, 3000 random ints ∉ {1..5} | `default:` → returns `add_operation` (**not** NULL, no error) | `err09_select_operation_out_of_range_enum` | [x] |
| 10 | `get_operation_priority` | `op` out of enum range (`0`, negative, `6`, `INT_MAX`, 3000 random) | no check: `op * 10`, wrapping on overflow | `err10_get_operation_priority_no_validation` | [x] |
| 11 | `allocate_results` | `count < 0` → sign-extended to a huge `size_t` | `calloc` fails → `NULL`; **C does not check this**. Asserted NULL on both sides. | `err11_allocate_results_negative_count_returns_null` | [x] |
| 12 | `allocate_results` | `count == 0` | `calloc(0, 24)` → glibc returns a unique non-`NULL` pointer; both sides agree | `err12_allocate_results_zero_count` | [x] |
| 13 | `allocate_results` | `count` huge but positive (`INT_MAX`, `1<<30`, …) → `count * 24` bytes | Both sides forward the identical `(nmemb, size)` pair to the same `calloc`, so the assertion is that their NULL-ness **agrees** (whether the allocator succeeds depends on the host's overcommit settings and is not a property of the translation) | `err13_allocate_results_huge_count` | [x] |
| 14 | `perform_computation_with_history` | `*history == NULL` on entry with a nonzero `*history_count` (`1, 5, 9, 10, 11, 12, 1000, INT_MAX, -1, INT_MIN`) | lazily allocates 10 slots and **resets `*history_count` to 0**, silently discarding the caller's count; ends at `1` | `err14_null_history_discards_caller_count` | [x] |
| 15 | `perform_computation_with_history` | `*history_count == 10` (at capacity) | record **silently dropped**: buffer untouched, count unchanged; the arithmetic result is still returned | `err15_pcwh_at_capacity_drops_record` | [x] |
| 16 | `perform_computation_with_history` | `*history_count > 10` (`11, 12, 20, 1000, INT_MAX-1, INT_MAX`) | `< 10` false → same silent drop | `err16_pcwh_over_capacity_drops_record` | [x] |
| 17 | `perform_computation_with_history` | `*history_count < 0` (`-16..=-1`) with non-NULL history | `< 10` is **true** → writes at a negative offset (out-of-bounds, UB) and still increments. Run against a 16-element padded buffer so the write lands in owned memory; all `26 × 24` bytes compared, and the write is asserted to have landed in the padding. | `err17_pcwh_negative_count_writes_before_buffer` | [x] |
| 18 | `perform_computation_with_history` | `op` out of enum range (`0, 6, 7, -1, -5, 1000, INT_MIN, INT_MAX` + 500 random) | no rejection: `default` → `add_operation`; asserted equal to `a + b` | `err18_pcwh_out_of_range_enum` | [x] |
| 19 | `perform_computation_with_history` | `history == NULL`, `history_count == NULL`, and both NULL | unchecked dereference → **SIGSEGV**. Compared out-of-process via `fork()`; both sides must raise signal 11. | `err19_pcwh_null_pointer_args_fault_identically` | [x] |
| 20 | `perform_computation_with_history` | `op ∈ {OP_DIVIDE, OP_MODULO}` with `b == 0`, at counts `0/4/9` | inner op returns `0`; a record with `value == 0`, `status == STATUS_SUCCESS` is still appended and the count still increments | `err20_pcwh_zero_divisor_still_records` | [x] |
| 21 | `mathop` | `param1 % 128` ∉ `49..=53` — tested over **all 128 residues × 5 signs of `param1`**, plus paired rejected/accepted values | `is_valid == false` → `validation_char` reset to `'1'`, then **never read again**, so neither the return value nor any printf line changes | `err21_to_25_mathop_rejection_paths` (row 21 block) | [x] |
| 22 | `mathop` | `param3 < 0` → negative `param3 % 5` → `selected_op ∈ -3..=0`, out of enum range (`-1..-10`, `INT_MIN`, `INT_MIN+1`, `-2147483645`) | no rejection: `get_operation_priority` yields a ≤ 0 priority and `select_operation` picks `add_operation` | `err21_to_25_mathop_rejection_paths` (row 22 block) | [x] |
| 23 | `mathop` | `param4 == -1` → `(param4+1) % 5 + 1 == 1`; `param4 < -1` → negative `second_op` (`-1..-7`, `-11`, `INT_MIN`, `INT_MIN+1`) | no rejection: out-of-range `second_op` → `add_operation` | `err21_to_25_mathop_rejection_paths` (row 23 block) | [x] |
| 24 | `mathop` | called ≥ 6 times in one process (history saturates at 10 after 5 calls) | further records dropped; `History entries: 10` printed on both sides forever; return value unaffected | `err21_to_25_mathop_rejection_paths` (row 24 block) + `phase_b_mathop_all_rows` (trajectory 2,4,6,8,10,10,10,10) | [x] |
| 25 | `mathop` | `param1 == INT_MIN` (`INT_MIN % 128 == 0`, and `INT_MIN` fed to the inner op) crossed with `param2/param3/param4 ∈ {INT_MIN, -1, 0, 1, INT_MAX}` | no rejection; signed overflow wraps identically | `err21_to_25_mathop_rejection_paths` (rows 25/25b) | [x] |
| 26 | `get_computation_timestamp` | none — no failure path (`time()`'s `-1` error return is not checked) | returns `time() >> 29` (arithmetic shift) | `err26_get_computation_timestamp_no_failure_path` | [x] |

## Generic FFI-boundary sweep (beyond the table)

`generic_out_of_range_enum_sweep` feeds ~2000 randomized plus 15 hand-picked
`int` values — including every enum boundary (`0`, `6`, `-1`, `-6`, `INT_MIN`,
`INT_MAX`) — into **every** entry point that takes an enum or a `char`
(`get_operation_priority`, `select_operation`, `is_valid_operation`,
`perform_computation_with_history`), asserting identical results and identical
24-byte history contents. Null pointers (row 19), zero lengths (row 12),
oversized lengths (rows 11/13) and one-step-past-range enum values (rows 9/10/18)
are all covered.

## Divergence found and fixed

| where | C behaviour | Rust behaviour before the fix | fix |
|-------|-------------|-------------------------------|-----|
| `perform_computation_with_history`, row 19 (`history == NULL`) | unchecked load → **SIGSEGV** (signal 11) | rustc inserts a null-pointer check for a plain `*p` dereference whenever `-Cdebug-assertions` is on, turning the fault into `panic_nounwind` → **SIGABRT** (signal 6) | the loads/stores now go through `core::ptr::read` / `core::ptr::write` (and `&raw mut (*slot).field` for the field stores, which keeps C's field-wise store pattern and leaves the struct padding untouched). Those forms emit no check, so the fault is SIGSEGV in **both** the dev and release profiles. |

The trap-preserving `idiv` in `c_divrem` and the null-fault fix were both
confirmed load-bearing by negative controls: replacing `c_divrem` with the
portable `wrapping_div` fallback makes rows 7–8 fail, and reverting to plain
derefs makes row 19 fail.
