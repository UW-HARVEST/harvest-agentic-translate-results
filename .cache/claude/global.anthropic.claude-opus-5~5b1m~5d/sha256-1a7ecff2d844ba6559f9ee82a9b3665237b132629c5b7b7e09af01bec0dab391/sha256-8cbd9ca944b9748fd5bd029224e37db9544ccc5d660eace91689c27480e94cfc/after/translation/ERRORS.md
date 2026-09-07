# ERRORS.md — Phase A error surface table

Mechanically derived from every rejection/error path in `c_src/src/lib.c`.
The whole file is 34 lines and contains exactly **two** `return NULL`
statements plus the implicit failure modes of `malloc`. There are no
`assert`s, no error enums, no `RETURN_ERROR` macros, and no other range checks.

Grep evidence:

```
$ grep -n 'return\|assert\|NULL\|free' c_src/src/lib.c
8:    void* const bufferPtrs = malloc(numLines * sizeof(const char**));
10:    if (bufferPtrs == NULL) return NULL;
27:    if (lineIndex != numLines) {
29:        free(bufferPtrs);
30:        return NULL;
31:    }
33:    return linePointers;
```

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `UTIL_createLinePointers` | `malloc(numLines * 8)` fails (line 10) — e.g. `numLines = SIZE_MAX/8` so the allocation request is astronomically large | returns `NULL`, nothing allocated/leaked | `err_01_malloc_failure_huge_numlines` | [x] |
| 2 | `UTIL_createLinePointers` | `malloc(numLines * 8)` fails after **wrapping** multiplication still yielding a huge value: `numLines = SIZE_MAX` → `SIZE_MAX*8 mod 2^64 = 0xFFFF_FFFF_FFFF_FFF8` | returns `NULL` | `err_02_malloc_failure_wrapping_numlines` | [x] |
| 3 | `UTIL_createLinePointers` | loop terminates with `lineIndex != numLines` (line 27) because `bufferSize == 0` while `numLines > 0`: the `pos < bufferSize` guard is false on entry so zero lines are recorded | `free(bufferPtrs)` then returns `NULL` | `err_03_zero_buffersize_nonzero_numlines` | [x] |
| 4 | `UTIL_createLinePointers` | loop terminates with `lineIndex != numLines` because the buffer holds **fewer** NUL-separated lines than `numLines` (buffer exhausted first) | `free` + `NULL` | `err_04_fewer_lines_than_requested` | [x] |
| 5 | `UTIL_createLinePointers` | `numLines * 8` wraps to exactly `0` (`numLines = 2^61`) so `malloc(0)` **succeeds** (non-NULL), but the loop then cannot reach `lineIndex == 2^61`; combined with `bufferSize == 0` the loop body never runs, so path 27 is taken | `free` + `NULL` (NOT a malloc failure) | `err_05_wrap_to_zero_malloc_succeeds_then_line_mismatch` | [x] |
| 6 | `UTIL_createLinePointers` | `buffer == NULL` together with `numLines == 0` (no dereference happens; `malloc(0)`) | returns **non-NULL** (`malloc(0)`); NOT an error | `err_06_null_buffer_zero_lines` | [x] |
| 7 | `UTIL_createLinePointers` | `buffer == NULL`, `numLines > 0`, `bufferSize == 0` (loop guard false ⇒ still no dereference) | `free` + `NULL` | `err_07_null_buffer_zero_buffersize` | [x] |
| 8 | `UTIL_createLinePointers` | `numLines` one step past the number of lines actually present (off-by-one past the valid range), buffer *does* end in NUL | `free` + `NULL` | `err_08_off_by_one_past_line_count` | [x] |
| 9 | `UTIL_createLinePointers` | buffer that is entirely non-NUL (no terminator at all) with `numLines >= 2` — only one line can ever be produced | `free` + `NULL` | `err_09_no_terminator_multiple_lines` | [x] |

## Notes on non-errors (deliberately *not* rejected by the C)

* `numLines == 0` with any `bufferSize`: loop is skipped, `lineIndex == numLines == 0`,
  so the `malloc(0)` block is returned as a **success** (non-NULL). Covered by
  row 6 and by `CONFIGS.md` row 1.
* `numLines` **smaller** than the number of lines present: success, only the
  first `numLines` pointers are filled. Not an error.
* There are **no enum parameters** in this API, so "out-of-range enum value"
  degenerates to out-of-range `size_t` values, covered by rows 1, 2, 5.
* `buffer == NULL` with `numLines > 0 && bufferSize > 0` dereferences NULL in
  the C (line 17) and is undefined behaviour / a guaranteed segfault in **both**
  implementations; it is not a rejection path and is therefore not tested.
