# ERRORS.md — Phase C error-surface table

Every row is a *distinct* rejection / error / fallback branch found by grepping
`c_src/src/lib.c` for `return NULL`, `return -1`, `return 0`, `if (!x)`,
null checks, range checks, `default:` labels and division guards. There are no
`assert`s, no error enums and no error-return macros in this library — the whole
error surface is null sentinels, `-1`, and silent `0` / fallback values.

Rows are checked off only once a differential test constructs that exact
condition, calls BOTH the C `.so` and the Rust `.so`, and asserts the *same*
sentinel / value comes back.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `create_buffer` | `malloc(sizeof(StringBuffer))` fails (line 36 `if (!buffer)`) | returns `NULL` | [x] not host-triggerable — see note A |
| 2 | `create_buffer` | `malloc(initial_capacity)` fails (line 41) because `initial_capacity < 0` is sign-extended to a huge `size_t`: `-1` | returns `NULL`, inner `free(buffer)`, no leak | [x] |
| 3 | `create_buffer` | same as #2 with `initial_capacity == INT_MIN` (`0xFFFF_FFFF_8000_0000` bytes) | returns `NULL` | [x] |
| 4 | `create_buffer` | same as #2 with assorted negative capacities (`-2, -7, -4096, INT_MIN+1`, random) | returns `NULL` | [x] |
| 5 | `create_buffer` | `initial_capacity == 0` — *not* an error: glibc `malloc(0)` returns non-NULL, then `data[0]='\0'` writes 1 byte into a 0-size block | non-NULL buffer, `capacity==0`, `length==0` | [x] |
| 6 | `append_to_buffer` | `realloc` fails (line 61 `if (!new_data)`) because `new_capacity = required_capacity*2` overflows `int` to a negative value and is sign-extended: `length = 2_000_000_000`, non-empty `str` | returns `-1`; `data`/`capacity` left unmodified | [x] |
| 7 | `append_to_buffer` | `realloc` fails with `new_capacity` overflowing to negative from `length = INT_MAX/2 + k` for several `k` | returns `-1` | [x] |
| 8 | `append_to_buffer` | `required_capacity` itself overflows to a *negative* int (`length = INT_MAX`, non-empty `str`), so `required_capacity > capacity` is FALSE, the grow branch is skipped entirely, and `strcpy` runs at `data + INT_MAX` | returns `0`, `capacity` untouched, `length` wraps (see note B) | [x] |
| 9 | `append_to_buffer` | `buffer == NULL` → `buffer->length` deref of NULL | SIGSEGV (UB) | [x] subprocess |
| 10 | `append_to_buffer` | `str == NULL` → `strlen(NULL)` | SIGSEGV (UB) | [x] subprocess |
| 11 | `destroy_buffer` | `buffer == NULL` (line 76 `if (buffer)`) | no-op, no crash | [x] |
| 12 | `destroy_buffer` | `buffer->data == NULL` (line 77 `if (buffer->data)`) | skips `free(data)`, still frees `buffer` | [x] |
| 13 | `get_operation_name` | `op_code` outside `0..=3` — the `default:` label. Includes out-of-range "enum" ints across FFI: `4, 5, -1, -2, -3, -4, INT_MIN, INT_MAX`, random | returns `"unknown"` | [x] |
| 14 | `perform_operation` | `operation` matches none of the four names (line 107 fall-through `return 0`) — `""`, `"ADD"`, `"add "`, `" add"`, `"addx"`, `"div"`, `"unknown"`, random bytes | returns `0` | [x] |
| 15 | `perform_operation` | `operation == "divide"` and `b == 0` (line 102 `if (b != 0)` false) | returns `0` (does **not** trap) | [x] |
| 16 | `perform_operation` | `operation == "divide"`, `a == INT_MIN`, `b == -1` — quotient not representable; gcc emits a bare `idiv` | SIGFPE (UB) | [x] subprocess |
| 17 | `perform_operation` | `operation == NULL` → `strcmp(NULL, "add")` | SIGSEGV (UB) | [x] subprocess |
| 18 | `perform_operation` | signed overflow in `a+b` / `a-b` / `a*b` (`INT_MAX+1`, `INT_MIN-1`, `INT_MIN*-1`, …) — UB in C, gcc wraps | wrapped 2's-complement result | [x] |
| 19 | `buffapp` | `intermediate3 == 0` (line 143 `else`) → result replaced by `p1+p2+p3+p4` instead of a divide | returns wrapped `p1+p2+p3+p4` | [x] |
| 20 | `buffapp` | `param1 % 4` negative (C `%` truncates toward zero) → `get_operation_name` hits `default:` → `"unknown"` → `perform_operation` returns `0` | `intermediate1 == 0` | [x] |
| 21 | `buffapp` | `param3 % 4` negative → same fall-through for `intermediate2` | `intermediate2 == 0` | [x] |
| 22 | `buffapp` | `create_buffer(32)` returned NULL → `log_buffer->length = 0` derefs NULL (no null check at line 116) | SIGSEGV (UB) | [x] not host-triggerable — see note A |
| 23 | `buffapp` | `param1 == INT_MIN` → `INT_MIN % 4 == 0` (not negative) → `"add"` path, and `INT_MIN` formatted by `sprintf` | takes the `add` branch | [x] |
| 24 | `buffapp` | `result / intermediate3` is `INT_MIN / -1`. Reachable: `buffapp(0, 1073741823, 0, 1073741825)` → both halves take `add`, so `i1=1073741823`, `i2=1073741825`, `result = i1+i2 = INT_MIN` and `i3 = i1*i2 = -1` (wrapping) | SIGFPE (UB) | [x] subprocess |

## Notes

**A. Rows 1 and 22 are not host-triggerable.** Both require `malloc` of 16 bytes
(`sizeof(StringBuffer)`) to fail. `buffapp` always calls `create_buffer(32)` with
a hard-coded literal, so its `log_buffer` is never NULL on any host with a
working allocator. The tests therefore verify the *code shape* is equivalent
(Rust checks `buffer.is_null()` and returns `null_mut()` in the same position,
and Rust likewise derefs `log_buffer` unconditionally without a null check)
rather than executing the branch. Forcing it would need an allocator-failure
interposer, which would change `malloc` for *both* libraries in the same process
and so could not produce a meaningful differential.

**B. Row 8 is a genuine out-of-bounds write in the C** (`strcpy` at
`data + INT_MAX`). Because `required_capacity` wraps negative, the
`required_capacity > capacity` test is false and no reallocation happens. Rather
than let the wild store fault, the test points `buffer->data` at a 2 GiB
`PROT_READ|PROT_WRITE, MAP_NORESERVE` anonymous reservation, so `data + INT_MAX`
is legal memory. That makes the branch fully observable: the test diffs the
return value (`0`), the untouched `capacity`, the wrapped `length`, and the 64
bytes actually written at `data + INT_MAX` (both windows pre-poisoned with
`0xAA` so any difference shows up). If the 2 GiB reservation is refused, the test
falls back to asserting the branch *decision* only, and says so on stderr.

**C. UB rows (9, 10, 16, 17, 24).** These terminate the process. They are still
real differential rows: `outcome_of()` in `tests/common/mod.rs` `fork()`s a
child, performs the one call in the child against C or against Rust, and
`waitpid`s. The parent asserts both children died with the *same* signal number
(`SIGSEGV` = 11, `SIGFPE` = 8) — "both failed somehow" is not accepted, and each
test additionally pins the expected signal explicitly. `err15`/`err16` also
assert the *near-miss* inputs (`INT_MIN/1`, `(INT_MIN+1)/-1`, `INT_MAX/-1`,
`INT_MIN/-2`, `x/0`) exit **0** in both, so the tests cannot pass by having
everything crash.

## Row → test mapping (auditable)

```
grep -h '^fn err' tests/phase_c_errors.rs
```

All rows live in `tests/phase_c_errors.rs`, one test per row, named `errNN_…`:

| # | test |
|---|------|
| 1  | `err01_outer_malloc_failure_not_reachable` (documents non-reachability; see note A) |
| 2  | `err02_create_buffer_negative_one` |
| 3  | `err03_create_buffer_int_min` |
| 4  | `err04_create_buffer_assorted_negatives` |
| 5  | `err05_create_buffer_zero_capacity_succeeds` |
| 6  | `err06_append_realloc_failure_two_billion_length` |
| 7  | `err07_append_realloc_failure_around_int_max_half` |
| 8  | `err08_append_required_capacity_overflows_negative` |
| 9  | `err09_append_null_buffer_same_signal` — both SIGSEGV (11) |
| 10 | `err10_append_null_string_same_signal` — both SIGSEGV (11) |
| 11 | `err11_destroy_null_is_noop` |
| 12 | `err12_destroy_with_null_data` |
| 13 | `err13_get_operation_name_out_of_range` — out-of-range enum ints |
| 14 | `err14_perform_operation_unmatched_returns_zero` |
| 15 | `err15_divide_by_zero_returns_zero_and_does_not_trap` |
| 16 | `err16_int_min_div_minus_one_same_signal` — both SIGFPE (8) |
| 17 | `err17_perform_operation_null_operation_same_signal` — both SIGSEGV (11) |
| 18 | `err18_signed_overflow_wraps_identically` |
| 19 | `err19_buffapp_intermediate3_zero_takes_sum_fallback` |
| 20 | `err20_buffapp_negative_residue_op1_is_unknown` |
| 21 | `err21_buffapp_negative_residue_op2_is_unknown` |
| 22 | `err22_buffapp_log_buffer_never_null` (documents non-reachability; see note A) |
| 23 | `err23_buffapp_int_min_param1_takes_add_branch` |
| 24 | `err24_buffapp_final_division_traps_identically` — both SIGFPE (8) |

Generic boundary coverage required beyond the table:
`generic_zero_and_oversized_lengths` (zero-length and oversized appends,
capacity 0) and `generic_one_past_valid_range_enum_values` (`op_code` 4 and −1,
capacity −1/0/1).

Note B is superseded: row 8 is verified *without* faulting by pointing
`buffer->data` at a 2 GiB `MAP_NORESERVE` reservation, so the wild
`strcpy(data + INT_MAX, str)` lands on real memory and the written bytes, the
wrapped `length`, and the untouched `capacity` are all diffed directly.
