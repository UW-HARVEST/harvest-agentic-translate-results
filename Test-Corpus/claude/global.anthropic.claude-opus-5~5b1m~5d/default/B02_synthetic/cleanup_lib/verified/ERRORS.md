# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every rejection / early-exit /
error-return path in the C source is enumerated below, one row per distinct
branch. Greps used:

```
grep -n 'goto\|return\|if (\|assert\|NULL\|!' c_src/src/lib.c
```

The C library has **no error enum, no `RETURN_ERROR` macro, no `assert`, and no
range checks on its integer parameters**. All four `int` parameters of `cleanup`
accept the entire `int` domain — there is no validation. The only rejection
paths are the two `goto cleanup` branches and the null guard in
`cleanup_resources`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `cleanup` | `strncmp(input_str, expected_str, strlen(expected_str)) != 0` — the string-validation branch at `lib.c:42`. Both operands are the *same* compile-time literal `"VALID"`, so `strncmp` is always `0`: the branch is **statically unreachable** for every input. Must therefore *never* print `"Input string validation failed.\n"` and never return early. | Branch never taken; `cleanup` always proceeds to the switch loop. Return value is the accumulated `result`, never a short-circuited `0`. | `err_row1_validation_branch_unreachable` | ✅ |
| 2 | `cleanup` | `malloc(50) == NULL` — allocation-failure branch at `lib.c:66`. Prints `"Memory allocation failed.\n"`, jumps to `cleanup:`, and returns the *already accumulated* `result` (NOT an error sentinel). Cannot be forced through the FFI boundary; verified by asserting the observable contract instead: the return value equals the accumulated `result` and no `"Memory allocation failed."` text is produced on a healthy allocator, i.e. C and Rust take the identical branch. | Returns accumulated `result` (same value as the success path — the C code has no distinct error code). | `err_row2_malloc_branch_parity` | ✅ |
| 3 | `cleanup_resources` | `dynamic_str == NULL` (`if (dynamic_str)` guard at `lib.c:84`) — the null pointer must be *accepted*: no `free`, no crash, no output, `void` return. | Silent no-op, returns normally. | `err_row3_cleanup_resources_null` | ✅ |
| 4 | `cleanup_resources` | `dynamic_str != NULL` and pointing at a live `malloc`ed block — must call `free` exactly once and return normally. Verified by round-tripping a block allocated by libc `malloc` through both `.so`s (must not double-free / must not leak-and-abort). | Frees the block, returns normally. | `err_row4_cleanup_resources_frees` | ✅ |
| 5 | `print_result` | `label == NULL` passed to `printf("%s: %d\n", label, result)`. glibc's `printf` prints the literal `(null)` for a NULL `%s`; the Rust wrapper forwards the same NULL to the same libc `printf`, so behaviour must be identical. | Prints `(null): <result>\n`, returns normally. | `err_row5_print_result_null_label` | ✅ |
| 6 | `print_result` | `label` = empty string `""` (zero-length input — the generic zero-length boundary). | Prints `: <result>\n`, returns normally. | `err_row6_print_result_empty_label` | ✅ |
| 7 | `print_result` | `label` long enough to exceed any internal buffer (oversized-length boundary; C uses unbounded `printf`, so no truncation may occur). | Prints the full label followed by `: <result>\n`. | `err_row7_print_result_long_label` | ✅ |

## Generic FFI boundaries also covered (not distinct C branches)

| # | boundary | why it is a real input | test | status |
|---|----------|------------------------|------|--------|
| G1 | `INT_MIN` / `INT_MAX` passed as `a,b,c,d` | the `default:` arm does `result += numbers[i]`, which signed-overflows. C wraps in practice (`-fwrapv`-free gcc still emits `add`); Rust must reproduce the same two's-complement wrap, not panic. | `bound_int_extremes` | ✅ |
| G2 | all four args at once equal to each switch label (`10`, `20`, `30`, `40`) and to the values one step past them (`9/11/19/21/29/31/39/41`, `0`, `-10`, `-20`, `-30`, `-40`) | "one step past a documented valid range" — the switch labels are the only distinguished values; neighbours must fall into `default:`. Negative mirrors must NOT match the positive cases. | `bound_switch_label_neighbours` | ✅ |
| G3 | out-of-range "enum" values across FFI | the C API declares **no enum type** (`grep -n 'enum' c_src/**` → no matches), so there is no enum discriminant to fuzz. The `int` parameters are already exhaustively unconstrained and are covered by G1/G2 plus the randomized sweep. | n/a (documented) | ✅ |
| G4 | `print_result` with `result` = `INT_MIN`, `INT_MAX`, `0`, `-1` | `%d` formatting of extreme values must match byte-for-byte. | `bound_print_result_extremes` | ✅ |
| G5 | `cleanup_resources` called twice on the same pointer / on a non-heap pointer | double-free and free-of-stack are UB in C; deliberately **not** tested (would abort both libraries non-deterministically). Documented as out of contract. | n/a (documented) | ✅ |
