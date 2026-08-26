# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. The C code contains **no**
`assert`, no `RETURN_ERROR`-style macro, no `errno` use, and never
returns a negative sentinel; the `StatusCode` enumerators `STATUS_ERROR`
(-1) and `STATUS_WARNING` (1) are **declared but never assigned** — only
`STATUS_SUCCESS` is ever written (`lib.c:130`). Consequently every
"rejection" in this library is one of:

* a guard that substitutes a fallback value (`return 0`, `default:`,
  `validation_char = '1'`),
* a boolean predicate returning `false`,
* a silent no-op when a capacity limit is hit,
* an unchecked libc failure that is propagated as a NULL pointer.

Every one of them is enumerated below — one row per distinct rejection
branch / condition, at the exact source line that performs it.

| # | function (line) | trigger (exact invalid input/condition) | expected C result | test |
|---|-----------------|------------------------------------------|-------------------|------|
| E1 | `is_valid_operation` (`lib.c:53`) | `op_char == 0` (`op_char &&` is false) | returns `false` | `e1_is_valid_zero` |
| E2 | `is_valid_operation` (`lib.c:53`) | `op_char != 0 && op_char < '1'` (i.e. 1..47 and all negative values −128..−1) | returns `false` | `e2_is_valid_below_range` |
| E3 | `is_valid_operation` (`lib.c:53`) | `op_char > '5'` (54..127) | returns `false` | `e3_is_valid_above_range` |
| E4 | `divide_operation` (`lib.c:75-77`) | `b == 0` (divide by zero guard) | returns `0`, no trap | `e4_divide_by_zero` |
| E5 | `modulo_operation` (`lib.c:82-84`) | `b == 0` (modulo by zero guard) | returns `0`, no trap | `e5_modulo_by_zero` |
| E6 | `select_operation` (`lib.c:100-101`) | `op` matches no `case`: `op <= 0` or `op >= 6`, incl. out-of-range enum ints passed over FFI (`0`, `6`, `-1`, `INT_MIN`, `INT_MAX`) | falls through `default:` → returns `add_operation` | `e6_select_operation_out_of_range_enum` |
| E7 | `allocate_results` (`lib.c:113`) | `count < 0` → `(size_t)count` sign-extends to ~1.8e19, `calloc` overflow-checks `nmemb*size` | `calloc` fails → returns `NULL` (no NULL check in C) | `e7_allocate_negative_count` |
| E8 | `allocate_results` (`lib.c:113`) | `count` huge but positive (`INT_MAX`, `INT_MAX/24+1`) → allocation far larger than RAM | `calloc` returns `NULL` (or a valid block if the OS overcommits — both libraries use the *same* libc `calloc`, so the outcome must be identical) | `e8_allocate_huge_count` |
| E9 | `allocate_results` (`lib.c:113`) | `count == 0` → `calloc(0, 24)` degenerate request | glibc returns a **non-NULL** unique minimal block | `e9_allocate_zero_count` |
| E10 | `perform_computation_with_history` (`lib.c:122-125`) | `*history == NULL` (uninitialised history) | lazily allocates 10 slots and **resets `*history_count` to 0** (discarding the caller's count) | `e10_history_null_resets_count` |
| E11 | `perform_computation_with_history` (`lib.c:127`) | `*history_count >= 10` (history full) | computation still performed and returned, but the record is **silently dropped**: no write, `*history_count` not incremented | `e11_history_full_silent_drop` |
| E12 | `perform_computation_with_history` (`lib.c:127-128`) | `*history_count < 0` (negative count passes the `< 10` check) | **no rejection**: writes at a negative index `(*history)[negative]` and increments toward 0 (out-of-bounds write; reproduced, not fixed) | `e12_history_negative_count_writes_oob` |
| E13 | `perform_computation_with_history` (`lib.c:118`, via E6) | `op` out of range 1..5 | `select_operation` default → `add_operation` used for the record | `e13_history_out_of_range_op` |
| E14 | `perform_computation_with_history` (`lib.c:120`) | `op == OP_DIVIDE`/`OP_MODULO` with `b == 0` | inner guard returns `0`; a record with `value == 0`, `status == STATUS_SUCCESS` is appended | `e14_history_div_mod_by_zero` |
| E15 | `mathop` (`lib.c:142-146`) | `is_valid_operation((char)(param1 % 128))` is false (e.g. `param1 % 128` not in '1'..'5') | `validation_char = '1'` fallback — a **dead store**, `validation_char` is never read again, so the return value and all output are unaffected | `phase_c_mathop_error_rows` (E15 section) |
| E16 | `mathop` (`lib.c:148`) | `param3 % 5 == -1` (`param3` ≡ −1 mod 5) → `selected_op == 0`, an out-of-range enum | `select_operation` default → `add_operation`; `get_operation_priority(0) == 0` | `phase_c_mathop_error_rows` (E16 section) |
| E17 | `mathop` (`lib.c:148`) | `param3 < 0` with `param3 % 5 <= -2` → `selected_op` negative (−3..−1) | default → `add_operation`; **negative** `operation_priority` (−30..−10) added to the result | `phase_c_mathop_error_rows` (E17 section) |
| E18 | `mathop` (`lib.c:156`) | `param4 == INT_MAX` → `param4 + 1` signed overflow (UB; gcc wraps to `INT_MIN`) → `second_op == -2` | wraps, default → `add_operation`, no trap | `phase_c_mathop_error_rows` (E18 section) |
| E19 | `mathop` (`lib.c:117-135`, repeated calls) | ≥ 5 successive `mathop` calls (2 records each) saturate the 10-slot static history | further records silently dropped (E11); `"History entries: 10"` printed forever; **return value unaffected** | `phase_c_mathop_error_rows` (E19 section) |
| E20 | `perform_computation_with_history` (`lib.c:122`) | `history == NULL` (null out-param) — `*history` is dereferenced with no null check | both libraries fault with **SIGSEGV** (signal 11). Compared for real, not merely documented: the call is made in a `fork()`ed child and the two children's termination signals are asserted equal. Same for a NULL `history_count` | `e20_null_history_pointer_faults_identically` |
| E21 | `divide_operation`/`modulo_operation` (`lib.c:78`, `lib.c:85`) | `a == INT_MIN && b == -1` | signed-overflow **undefined behaviour**; gcc emits `idiv` → SIGFPE. No defined C result to match; Rust returns `INT_MIN`/`0` via `wrapping_div`/`wrapping_rem`. Excluded from differential fuzzing by the `mathop_is_ub`/`is_idiv_ub` generator guards; both behaviours pinned down in a `fork()`ed child | `e21_int_min_div_minus_one_documented` |

## Generic FFI-boundary cases (covered even though not distinct C branches)

| # | case | expectation | test |
|---|------|-------------|------|
| G1 | out-of-range enum ints for `Operation` across FFI (`select_operation`, `perform_computation_with_history`, and via `mathop`) | identical `default:` behaviour in both | `e6_*`, `e13_*`, `phase_c_mathop_error_rows` (G1 section) |
| G2 | `INT_MIN` / `INT_MAX` operands to all five math ops (except E21) | identical two's-complement wrap | `c3_*`–`c9_*` boundary sweeps in `phase_b_pure.rs` |
| G3 | zero and oversized lengths for `allocate_results` | identical NULL/non-NULL verdict | `e7_*`, `e8_*`, `e9_*` |
| G4 | one step past valid enum range (`0`, `6`) | identical fallback | `e6_*` |
| G5 | `history_count` exactly at the limit (`9`, `10`, `11`) | write at 9; drop at 10 and 11 | `e11_*`, `c19_*`, `c20_*`, `c21_*` |
| G6 | `param1 = INT_MIN` in `mathop` (`INT_MIN % 128 == 0` → `is_valid_operation(0)` false, E1+E15) | identical | `phase_c_mathop_error_rows` (E15 section) |
| G7 | a **misaligned** `*history` (buffer skewed by 1..7 bytes) — the C does an unaligned store, which x86 performs silently | byte-identical memory effects and identical `history_count` in both | `g7_misaligned_history_buffer` |

## Notes discovered while deriving the table

* **E14 is unreachable through `mathop`'s *second* computation.** `b` is `param4`
  there, and `param4 == 0` forces `second_op = ((0+1) % 5) + 1 = 2` (multiply),
  so `divide`/`modulo` can never see `b == 0` on that call. Asserted explicitly
  in `phase_b_mathop.rs` (row C31) rather than left implicit. The *first*
  computation does reach it (`param2 == 0` with `selected_op ∈ {4,5}`).
* `STATUS_ERROR` / `STATUS_WARNING` are dead enumerators: no C path ever stores
  them, so no test can observe them being produced. They are used as *poison*
  values in the test buffers instead, which is what makes "the C did not write
  here" assertions meaningful.
* `mathop`'s return value never depends on the static history state (the record
  is written after the value is computed and is never read back), which is why
  a saturated history changes only the `"History entries: 10"` line. Confirmed by the E19
  section of `phase_c_mathop_error_rows` and by row C32.

## Verification status

Every row E1–E21 and G1–G7 has a differential test that constructs the exact
condition and asserts C and Rust agree on the *same* sentinel/fallback:

| rows | test file |
|------|-----------|
| E1–E14, E20, E21, G1–G5, G7 | `tests/phase_c_errors.rs` (`e1_*` … `e21_*`, `g7_*`) |
| E15–E19, G1, G6 | `tests/phase_c_mathop_errors.rs::phase_c_mathop_error_rows` |

**Divergences found and fixed during Phase C**

* `history == NULL` (E20) originally diverged: the C reference faulted with
  **SIGSEGV** while the Rust aborted with **SIGABRT**, because rustc inserts a
  null-pointer UB check on `*ptr` derefs in debug builds. Fixed in
  `src/lib.rs` by routing every caller-pointer access through byte-sized
  volatile loads/stores (`raw_load`/`raw_store`/`slot`), which reproduces the C's
  raw-memory semantics — including its fault behaviour on NULL and its
  tolerance of misaligned pointers (G7) — in every build profile.

**Remaining known divergence: E21 only.** `divide_operation`/`modulo_operation`
with `(INT_MIN, -1)` is signed-overflow *undefined behaviour* in C, so there is
no defined C result to be byte-identical to; the reference build traps with
SIGFPE and the Rust returns the wrapped value. `e21_int_min_div_minus_one_documented`
pins both behaviours down and verifies that every *defined* neighbour of that
input matches exactly.
