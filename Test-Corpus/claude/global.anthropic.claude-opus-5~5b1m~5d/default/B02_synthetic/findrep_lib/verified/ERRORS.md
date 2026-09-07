# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, and no `return -1`/`return NULL`
statements**. Its entire rejection surface consists of:

* guard conditions that *silently skip* work (`if (b != 0)`, `if (found)`,
  `if (is_nonzero && value > 0)`),
* saturation / clamping range checks (`< 0100`, `> 0777`),
* a sentinel substitution on an empty result (`if (!result_exists) result = 0777`),
* undefined-behaviour cases reached through the FFI boundary (null pointers,
  signed overflow, `INT_MIN / -1`).

Every row is one distinct rejection branch the C actually contains.

| #  | function | trigger (exact invalid input/condition) | expected C result | test | status |
|----|----------|------------------------------------------|-------------------|------|--------|
| 1  | `divide_multiplier` | `b == 0` (`lib.c:54` guard false) | division skipped; `multiplier` unchanged; `operation_count` still incremented; returns unchanged `multiplier` | `err_01_divide_by_zero_guard` | [x] |
| 2  | `divide_multiplier` | `b == 0` repeatedly — confirms `operation_count` side effect still occurs on the rejected path (observable via `findrep`'s `+= operation_count * 010`) | `operation_count` advances once per call | `err_02_divide_by_zero_still_counts` | [x] |
| 3  | `divide_multiplier` | `b == 1` (no-op divisor, boundary just past the `!= 0` guard) | `multiplier /= 1` — unchanged value but division *is* executed | `err_03_divide_by_one` | [x] |
| 4  | `divide_multiplier` | `b == -1` with `multiplier != INT_MIN` (sign flip, one step past 0 on the negative side) | `multiplier = -multiplier` | `err_04_divide_by_negative_one` | [x] |
| 5  | `divide_multiplier` | `b` such that `|b| > |multiplier|` → truncation toward zero of a negative quotient | C integer division truncates toward zero (e.g. `-7/2 == -3`, not `-4`) | `err_05_division_truncates_toward_zero` | [x] |
| 6  | `find_and_replace_char` | `memchr` returns `NULL` — `search_char` absent from the string (`lib.c:69` guard false) | no byte written; string left byte-identical | `err_06_replace_char_not_found` | [x] |
| 7  | `find_and_replace_char` | empty string (`strlen == 0` → `memchr(s, c, 0)` always `NULL`) | no write; nothing past the NUL is touched | `err_07_replace_empty_string` | [x] |
| 8  | `find_and_replace_char` | `search_char == 0` — the NUL terminator is *outside* the `strlen` window, so it can never be found | no write (NUL is never replaced) | `err_08_replace_nul_never_found` | [x] |
| 9  | `find_and_replace_char` | `search_char` outside `unsigned char` range, e.g. `0x14F`, `'O' + 256`, `-79` (== `0xB1`) — `memchr` narrows to `unsigned char` | narrowed byte is matched: `0x14F` behaves as `0x4F` == `'O'` | `err_09_replace_char_narrowing` | [x] |
| 10 | `find_and_replace_char` | `search_char == 'X'` (the replacement byte itself) | first `'X'` overwritten with `'X'` — idempotent no-visible-change | `err_10_replace_char_is_x` | [x] |
| 11 | `find_and_replace_char` | multiple occurrences of `search_char` | only the **first** is replaced (`memchr`, not a loop) | `err_11_replace_only_first` | [x] |
| 12 | `validate_and_normalize` | `value == 0` → `is_nonzero == 0`, outer guard false | clamping skipped entirely; returns `0` (**not** `0100`) | `err_12_normalize_zero_not_clamped` | [x] |
| 13 | `validate_and_normalize` | `value < 0` → `value > 0` false, outer guard false | clamping skipped; negative value returned unchanged (**no** lower clamp) | `err_13_normalize_negative_not_clamped` | [x] |
| 14 | `validate_and_normalize` | `0 < value < 0100` (`lib.c:82`) | returns `lower_threshold` == `0100` == 64 | `err_14_normalize_lower_clamp` | [x] |
| 15 | `validate_and_normalize` | `value > 0777` (`lib.c:84`) | returns `upper_threshold` == `0777` == 511 | `err_15_normalize_upper_clamp` | [x] |
| 16 | `validate_and_normalize` | `value == 0100` exactly (boundary: `<` is strict, so NOT clamped) | returns `64` unchanged | `err_16_normalize_lower_boundary` | [x] |
| 17 | `validate_and_normalize` | `value == 0777` exactly (boundary: `>` is strict, so NOT clamped) | returns `511` unchanged | `err_17_normalize_upper_boundary` | [x] |
| 18 | `validate_and_normalize` | `value == 0o77` / `value == 0o1000` (one step *inside* each clamp) | `63 -> 64`, `512 -> 511` | `err_18_normalize_one_past_boundaries` | [x] |
| 19 | `validate_and_normalize` | `value == INT_MAX` (oversized) | clamped to `511` | `err_19_normalize_int_max` | [x] |
| 20 | `validate_and_normalize` | `value == INT_MIN` (negative extreme; note `-INT_MIN` is never computed) | returned unchanged, `INT_MIN` | `err_20_normalize_int_min` | [x] |
| 21 | `findrep` | all four params `0` → `active_params == 0`, every `>= mode_*` guard false | no operation dispatched at all; only the `memchr` offset + `operation_count*010` contribute | `err_21_findrep_all_zero_params` | [x] |
| 22 | `findrep` | computed `result == 0` → `!result_exists` (`lib.c:169`) | sentinel `0777` == 511 substituted for the real result | `err_22_findrep_zero_result_sentinel`, `err_22a_findrep_sentinel_known_hits`, `err_22b_findrep_sentinel_exhaustive_fresh_sweep` | [x] |
| 23 | `findrep` | `accumulator <= 0150` → `lib.c:142` guard false | `subtract_from_accumulator` is NOT dispatched; `operation_count` does not advance for it | `err_23_findrep_accumulator_guard_false` | [x] |
| 24 | `findrep` | `multiplier <= 0100` → `lib.c:161` guard false | `divide_multiplier` is NOT dispatched | `err_24_findrep_multiplier_guard_false` | [x] |
| 25 | `findrep` | `multiplier == 0` (reachable: any `multiply_with_multiplier` with a zero operand latches it to 0 forever) → `has_multiplier == 0`, `both_active == 0` | the `accumulator + multiplier` term is skipped; multiplier stays 0 for all later calls | `err_25_findrep_multiplier_zero_latch` | [x] |
| 26 | `findrep` | `accumulator == 0` → `has_accumulator == 0`, `both_active == 0` | `accumulator + multiplier` term skipped | `err_26_findrep_accumulator_zero` | [x] |
| 27 | `add_to_accumulator` / `subtract_from_accumulator` | signed overflow of `accumulator += (a+b)` with `a == b == INT_MAX` (C UB; gcc `-fwrapv`-less codegen wraps on x86-64) | two's-complement wraparound | `err_27_accumulator_signed_overflow` | [x] |
| 28 | `multiply_with_multiplier` | signed overflow of `multiplier *= (a*b)` with large `a`,`b` (C UB; wraps in practice) | two's-complement wraparound | `err_28_multiplier_signed_overflow` | [x] |
| 29 | `subtract_from_accumulator` | `a - b` overflow, `a == INT_MIN`, `b == INT_MAX` | wraparound of the inner subtraction *then* of the outer one | `err_29_subtract_inner_overflow` | [x] |
| 30 | `process_octal_string` | `octal_val < 0` — `%o` takes `unsigned int`, so the sign is reinterpreted, while `%d` prints it signed | e.g. `-1` -> `"Octal: 037777777777, Decimal: -1"` | `err_30_octal_negative_reinterpreted` | [x] |
| 31 | `process_octal_string` | `octal_val == INT_MIN` (longest possible output, stresses `char buffer[50]`) | `"Octal: 020000000000, Decimal: -2147483648"` (40 bytes, fits) | `err_31_octal_int_min_longest` | [x] |
| 32 | `process_octal_string` | `octal_val == 0` — `%o` of zero prints a single `0`, giving a doubled `00` after the literal `0` prefix | `"Octal: 00, Decimal: 0"` | `err_32_octal_zero_double_prefix` | [x] |

## Documented-but-untestable UB rows

These are genuine rejection-free UB paths in the C. A differential test cannot
assert "same result" because the C process dies; both implementations are
documented here instead of being asserted.

| # | function | trigger | C behaviour | why not tested |
|---|----------|---------|-------------|----------------|
| U1 | `process_octal_string` | `dest == NULL` | `strcpy` to NULL → SIGSEGV | crashes the test process; Rust also derefs NULL |
| U2 | `find_and_replace_char` | `str == NULL` | `strlen(NULL)` → SIGSEGV | same |
| U3 | `process_octal_string` | `dest` buffer shorter than the formatted message | heap/stack overflow past the caller's buffer | unbounded corruption, not a comparable value; tests always pass a 64-byte buffer as the header's contract implies |
| U4 | `divide_multiplier` | `multiplier == INT_MIN && b == -1` | x86-64 `idiv` raises `#DE` → SIGFPE | kills the process; Rust's `wrapping_div` yields `INT_MIN`. Divergence is unavoidable and is documented rather than "fixed", since matching a SIGFPE is not possible in safe Rust. Unreachable from `findrep`, whose only divisor is the constant `2`. |

## Reaching row 22 (the `result == 0` sentinel)

Random inputs never hit this branch, so it was derived analytically. On FRESH
state with `param1 = -9` and the rest `0`:

```
memchr('p' in "Function pointer example with static vars") = 9   -> result = 9
active_params == 1 >= mode_add -> add(normalize(-9), normalize(0))
                                = accumulator = -9               -> result = 0
accumulator(-9) > 0150?  no        active_params >= mode_multiply(2)?  no
both_active (acc=-9, mult=1) -> result += -9 + 1                 -> result = -8
multiplier(1) > 0100?    no
result += operation_count(1) * 010 = 8                           -> result = 0
!result_exists -> result = 0777
```

Verified against the C `.so`: `findrep(-9,0,0,0) == 511` and
`findrep(0,-9,0,0) == 511`, while the neighbours `-8` and `-10` do not return
511. `err_22b` additionally sweeps every single-nonzero-parameter value in
`-700..=700` and 2-parameter grids on fresh state, and asserts the branch is
reached at least once so the coverage cannot silently regress.

## Negative control (mutation testing)

`.scratch/mutate.sh` injects 29 known bugs into `src/lib.rs`, rebuilds, and
checks the suite catches each. Result: **24 caught, 5 uncaught, 0 skipped**.
Each uncaught mutant was analysed and is **semantically equivalent** to the
original — no test can distinguish it, so it is not a coverage gap:

| mutant | why it cannot be caught |
|--------|--------------------------|
| `{:o}` on `i32` instead of `as c_uint` | Rust's `Octal` impl for `i32` already prints the two's-complement bit pattern; verified `format!("{:o}", v) == format!("{:o}", v as u32)` for `-1`, `-9`, `INT_MIN`, `INT_MAX`, `0` |
| `value < lower_threshold` → `<=` | at `value == 64` the branch returns `lower_threshold == 64 == value` |
| `value > upper_threshold` → `>=` | at `value == 511` the branch returns `upper_threshold == 511 == value` |
| `if b != 0` → `if b != 0 && b != 1` | skipping a division by 1 is the identity |
| `selected_op(normalized_p3, normalized_p4)` args swapped | `OPERATIONS[1]` is `multiply_with_multiplier`, and `a * b` is commutative |

The sentinel mutant (`0777` → `0776`) was **uncaught before** `err_22a`/`err_22b`
were added and is **caught after**, which is what proved row 22 was a genuine
blind spot rather than an unreachable branch.
