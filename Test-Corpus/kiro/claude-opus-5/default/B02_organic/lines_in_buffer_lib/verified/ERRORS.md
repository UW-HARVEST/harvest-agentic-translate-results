# ERRORS.md — Phase A error-surface table

Mechanically derived from every rejection / early-return / bound in
`c_src/src/lib.c`. There are exactly two `return NULL` statements, no `assert`,
no error enum, no `RETURN_ERROR` macro, and no named min/max constant. The
remaining rows come from the three explicit range checks that decide whether
those `return NULL`s are reached:

- line 10: `if (bufferPtrs == NULL) return NULL;`
- line 12: loop guard `lineIndex < numLines && pos < bufferSize`
- line 17: inner guard `(pos + len < bufferSize) && buffer[pos + len] != '\0'`
- line 23: `if (pos < bufferSize) pos++;`
- line 27: `if (lineIndex != numLines) { free(bufferPtrs); return NULL; }`

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `UTIL_createLinePointers` | `malloc(numLines * 8)` fails — `numLines` so large the byte count does not wrap but cannot be allocated (e.g. `numLines = SIZE_MAX/16`) | line 10 hit → return `NULL`, nothing allocated | `err_01_malloc_failure` |
| 2 | `UTIL_createLinePointers` | `bufferSize == 0` with `numLines > 0` — loop guard `pos < bufferSize` false on entry, so zero iterations | `lineIndex(0) != numLines` → `free`, return `NULL` | `err_02_zero_buffersize_nonzero_numlines` |
| 3 | `UTIL_createLinePointers` | buffer contains FEWER NUL-separated segments than `numLines`; `pos` reaches `bufferSize` first (e.g. `"a\0"`, `bufferSize=2`, `numLines=3`) | `lineIndex < numLines` → `free`, return `NULL` | `err_03_fewer_lines_than_requested` |
| 4 | `UTIL_createLinePointers` | unterminated final segment exhausts the buffer while lines remain: `"abc"` (no NUL), `bufferSize=3`, `numLines=2` — inner guard `pos+len < bufferSize` stops at end, `pos=3`, `if (pos < bufferSize)` false | one iteration only → `lineIndex(1) != 2` → `free`, return `NULL` | `err_04_unterminated_tail_short` |
| 5 | `UTIL_createLinePointers` | `buffer == NULL` **and** `bufferSize == 0` **and** `numLines > 0` — loop never entered, so NULL is never dereferenced | `free`, return `NULL` (no crash) | `err_05_null_buffer_zero_size` |
| 6 | `UTIL_createLinePointers` | `size_t` multiplication wrap: `numLines = 2^61` → `numLines * 8 == 0` → `malloc(0)` returns a NON-NULL unique block; paired with `bufferSize == 0` so no OOB store happens | `malloc(0)` non-NULL, loop skipped, `0 != 2^61` → `free`, return `NULL` | `err_06_size_overflow_wrap_to_zero` |
| 7 | `UTIL_createLinePointers` | `size_t` multiplication wrap to a small non-zero value: `numLines = 2^61 + 1` → `numLines * 8 == 8` → tiny allocation; paired with `bufferSize == 0` | `malloc(8)` non-NULL, loop skipped, `0 != numLines` → `free`, return `NULL` | `err_07_size_overflow_wrap_to_small` |
| 8 | `UTIL_createLinePointers` | `numLines = SIZE_MAX` → `SIZE_MAX * 8 == SIZE_MAX - 7` (wrap) → allocation of ~2^64 bytes fails | line 10 hit → return `NULL` | `err_08_numlines_size_max` |
| 9 | `UTIL_createLinePointers` | `numLines == 0` with `bufferSize == 0` — degenerate but **NOT** an error path in C: `malloc(0)` is non-NULL on glibc and `0 == 0` passes the final check | returns a NON-NULL pointer to a zero-length block (must be `free`d) | `err_09_zero_lines_is_not_an_error` |
| 10 | `UTIL_createLinePointers` | `numLines == 0` with `buffer == NULL` and large `bufferSize` — no scan happens because `lineIndex < numLines` is false | returns NON-NULL zero-length block, no crash | `err_10_zero_lines_null_buffer` |
| 11 | `UTIL_createLinePointers` | `bufferSize = SIZE_MAX` (oversized length) with a small real buffer whose first bytes contain enough NULs to satisfy `numLines` — the C never validates `bufferSize` against the real allocation, it just stops once `lineIndex == numLines` | succeeds, returns pointers, provided the NULs appear before the real end of storage | `err_11_oversized_buffersize_but_enough_nuls` |

Notes on rejections that are **not differentially testable in-process** (both
implementations execute the identical undefined behaviour and abort the
process); they are recorded for completeness and covered indirectly by rows
5/6/7/10:

- `buffer == NULL` with `bufferSize > 0` and `numLines > 0` → both read
  `buffer[0]` → SIGSEGV.
- multiplication wrap (`numLines >= 2^61`) with `bufferSize > 0` → both store
  past the undersized block → heap corruption.

There is no enum in this API, so "out-of-range enum value across FFI" degenerates
to out-of-range `size_t` values, which rows 1, 6, 7, 8 and 11 cover
(`0`, `2^61`, `2^61+1`, `SIZE_MAX/16`, `SIZE_MAX`).

## Verification status

All 11 rows have a passing differential test that asserts C and Rust return the
**same** sentinel (`NULL` vs a non-NULL array with identical contents), not
merely "both failed":

- [x] 1 `err_01_malloc_failure`
- [x] 2 `err_02_zero_buffersize_nonzero_numlines`
- [x] 3 `err_03_fewer_lines_than_requested`
- [x] 4 `err_04_unterminated_tail_short`
- [x] 5 `err_05_null_buffer_zero_size`
- [x] 6 `err_06_size_overflow_wrap_to_zero`
- [x] 7 `err_07_size_overflow_wrap_to_small`
- [x] 8 `err_08_numlines_size_max`
- [x] 9 `err_09_zero_lines_is_not_an_error`
- [x] 10 `err_10_zero_lines_null_buffer`
- [x] 11 `err_11_oversized_buffersize_but_enough_nuls`

Generic-boundary tests beyond the table:

- [x] `boundary_off_by_one_around_line_count` — `numLines` exactly at / one past /
      one before the reachable line count.
- [x] `boundary_buffersize_one_step_past_range` — `bufferSize` at and one byte
      short of the real allocation.
- [x] `boundary_all_zero_arguments_repeated` — 1000× the fully degenerate call.
- [x] `boundary_out_of_range_integer_matrix` — 10×10 matrix of out-of-range
      `size_t` values (`0`, `1`, `2`, `3`, `6`, `SIZE_MAX`, `SIZE_MAX-1`,
      `SIZE_MAX/2`, `2^61`, `2^61+1`) in both positions, skipping only the
      combinations where both implementations execute identical
      process-aborting UB.
