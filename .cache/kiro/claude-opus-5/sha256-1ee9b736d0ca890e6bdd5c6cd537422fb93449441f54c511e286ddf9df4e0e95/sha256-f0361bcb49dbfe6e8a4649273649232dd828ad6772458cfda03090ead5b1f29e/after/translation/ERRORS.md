# ERRORS.md — Error-surface table

Derived mechanically from every rejection construct in `c_src/src/lib.c`.
Exhaustive grep for `if (`, `goto`, `return`, `assert`, `NULL`, `!`, `==`,
`!=`, range checks and size constants found exactly **three** conditional
branches (lines 42, 66, 84) and **no** `assert`, **no** `return -1`, **no**
error enum, and **no** range/bounds validation of `a`,`b`,`c`,`d`.

`cleanup` therefore has **no failing input**: every `int` quadruple is
accepted. Its two error branches are triggered by internal conditions, and
both are unreachable in practice — but they are real rows and are covered by
observing the *absence* of their diagnostics plus the resulting return value
(`goto cleanup` skips the remaining work but still returns `result`).

`print_result` and `cleanup_resources` perform no validation beyond the
single null check at line 84.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `cleanup` | `strncmp("VALID","VALID",strlen("VALID")) != 0` (line 42) — string validation fails | prints `Input string validation failed.\n`, `goto cleanup`, returns `result` (still `0`, no accumulation ran). Statically unreachable: both operands are the same literal, so the branch is never taken. Verified by asserting the diagnostic is **never** emitted for any input and the sum is always accumulated. | `e1_string_validation_branch_never_taken` | [x] |
| E2 | `cleanup` | `malloc(50 * sizeof(char))` returns `NULL` (line 66) | prints `Memory allocation failed.\n`, `goto cleanup` (skips `snprintf`/`printf`), returns the already-accumulated `result`. Unreachable for a 50-byte request. Verified by asserting the diagnostic is never emitted and `Processed numbers: numbers\n` is always printed instead. | `e2_malloc_failure_branch_never_taken` | [x] |
| E3 | `cleanup_resources` | `dynamic_str == NULL` (line 84) — null pointer passed | no-op; does **not** call `free`; returns normally (`void`). Asserted: both sides silent, no crash, over 1024 repeated calls. Note: this row cannot *distinguish* the guarded from the unguarded form, because `free(NULL)` is a defined no-op in C — verified by mutation (removing the Rust `is_null()` guard leaves all 27 tests green, i.e. the guard is genuinely unobservable rather than untested). | `e3_cleanup_resources_null` | [x] |

## Generic FFI boundary cases (not in the table above, covered anyway)

| # | function | trigger | expected C result | test | ✓ |
|---|----------|---------|-------------------|------|---|
| G1 | `print_result` | `label == NULL` → `printf("%s: %d\n", NULL, r)` | glibc prints the literal `(null)`, i.e. `(null): <r>\n`; no crash. | `g1_print_result_null_label` | [x] |
| G2 | `print_result` | `label == ""` (zero-length string) | prints `: <r>\n` | `g2_print_result_empty_label` | [x] |
| G3 | `print_result` | `label` containing `%s`/`%d`/`%n` — format metacharacters in *data* position | printed verbatim, not interpreted (label is an argument, not the format) | `g3_print_result_format_metachars` | [x] |
| G4 | `print_result` | oversized `label` (4096 bytes, no NUL until the end) | full label printed, no truncation (`printf`, not `snprintf`) | `g4_print_result_oversized_label` | [x] |
| G5 | `print_result` | `result` = `INT_MIN`, `INT_MAX`, `-1`, `0` | `%d` rendering of that exact value | `g5_print_result_extreme_results` | [x] |
| G6 | `cleanup_resources` | valid `malloc`-owned pointer | pointer is `free`d exactly once, no crash, no output | `g6_cleanup_resources_valid_ptr` | [x] |
| G7 | `cleanup` | out-of-range "enum-like" selector values passed as `int`: values one step past every `case` label (`9,11,19,21,29,31,39,41`), plus `-10,-20,-30,-40` (negated case labels), `0`, `INT_MIN`, `INT_MAX` | all fall into `default:` → `result += numbers[i]`; the C `switch` has no enum type, so any `int` is a valid input | `g7_out_of_range_case_selectors` | [x] |
| G8 | `cleanup` | signed-overflow-producing quadruples (`INT_MAX,INT_MAX,…`, `INT_MIN,INT_MIN,…`, `INT_MAX,10,…`) | C `+=` on `int` overflows; both sides must produce the identical bit pattern | `g8_overflow_quadruples` | [x] |

## Suite sensitivity (negative controls)

The rows above all pass, which is only meaningful if the harness can actually
observe a divergence. Four realistic mistranslations were injected into
`src/lib.rs`, rebuilt, and run; the source was then restored and verified
byte-identical with `diff`.

| injected mutation | tests that caught it |
|-------------------|----------------------|
| `case 10` fall-through into `case 20` removed | 7 |
| `TO_STRING(numbers)` rendered as the array contents instead of the literal text `numbers` | 11 |
| `print_result` format changed `"%s: %d\n"` → `"%s : %d\n"` | 3 |
| accumulator uses `saturating_add` instead of wrapping (`int` overflow) | 8 |
| `is_null()` guard removed from `cleanup_resources` | 0 — semantically equivalent, see E3 |
