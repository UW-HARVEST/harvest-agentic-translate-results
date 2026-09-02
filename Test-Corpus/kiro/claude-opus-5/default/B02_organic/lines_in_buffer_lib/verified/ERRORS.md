# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. The C source contains exactly
these rejection / error constructs (there are no `RETURN_ERROR` macros, no
error enums, no `assert`, and no explicit range or null checks in this library):

```
$ grep -n 'return\|NULL\|assert\|free(' c_src/src/lib.c
8:    void* const bufferPtrs = malloc(numLines * sizeof(const char**));
10:   if (bufferPtrs == NULL) return NULL;          <-- rejection site #1
29:  if (lineIndex != numLines) {                   <-- rejection site #2
31:      free(bufferPtrs);
32:      return NULL;
34:  return linePointers;                           (success)
```

So there are **two** distinct rejection sites in the C, reachable by several
distinct triggers. One row per distinct trigger.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|----------------------------------------------|-------------------|-----|
| E1 | `UTIL_createLinePointers` | `malloc` fails: `numLines` large enough that `numLines * 8` does **not** wrap but is unsatisfiable (`numLines = 1<<60` → 2^63 bytes) | `NULL` (site #1, no `free`) | [x] |
| E2 | `UTIL_createLinePointers` | `malloc` fails, `numLines = SIZE_MAX/8` (largest non-wrapping product = `SIZE_MAX & ~7`) | `NULL` (site #1) | [x] |
| E3 | `UTIL_createLinePointers` | `bufferSize == 0`, `numLines > 0` — outer loop never entered because `pos < bufferSize` is false ⇒ `lineIndex(0) != numLines` | `NULL` (site #2, allocation `free`d) | [x] |
| E4 | `UTIL_createLinePointers` | `buffer == NULL`, `bufferSize == 0`, `numLines > 0` (null pointer never dereferenced because the loop body is not reached) | `NULL` (site #2) | [x] |
| E5 | `UTIL_createLinePointers` | `numLines` strictly greater than the number of NUL-separated records that fit in `bufferSize` (buffer exhausted first: `pos >= bufferSize` with `lineIndex < numLines`) | `NULL` (site #2) | [x] |
| E6 | `UTIL_createLinePointers` | `numLines > 0`, `bufferSize > 0`, buffer contains **no** NUL at all (one unterminated record consumes the whole buffer) and `numLines >= 2` | `NULL` (site #2) | [x] |
| E7 | `UTIL_createLinePointers` | `numLines == SIZE_MAX`, `bufferSize == 0` — product `SIZE_MAX * 8` wraps to `SIZE_MAX - 7`; `malloc` of that fails | `NULL` (site #1) | [x] |
| E8 | `UTIL_createLinePointers` | `numLines == 1<<61` (product wraps to **0**), `bufferSize == 0` — `malloc(0)` succeeds (non-NULL on glibc), loop not entered, `lineIndex != numLines` | `NULL` (site #2, `free`d) | [x] |
| E9 | `UTIL_createLinePointers` | `numLines == (1<<61)+1` (product wraps to **8**), `bufferSize == 0` — undersized `malloc` succeeds, loop not entered | `NULL` (site #2) | [x] |
| E10 | `UTIL_createLinePointers` | one-past-valid boundary: buffer holds exactly `N` NUL-terminated records, caller asks for `N + 1` | `NULL` (site #2) | [x] |

## Generic FFI boundaries also covered by the Phase C tests

| # | boundary | condition exercised | expected C result | [x] |
|---|----------|---------------------|-------------------|-----|
| E11 | null `buffer` | `buffer == NULL`, `numLines == 0`, `bufferSize > 0` (loop guard `lineIndex < numLines` short-circuits before any deref) | **non-NULL** `malloc(0)` result — *not* an error | [x] |
| E12 | zero length | `numLines == 0 && bufferSize == 0` | **non-NULL** `malloc(0)` result | [x] |
| E13 | zero length | `numLines == 0`, `bufferSize > 0`, real buffer | **non-NULL**, zero elements written | [x] |
| E14 | oversized length | `bufferSize` far larger than the real allocation but `numLines` satisfied before the excess is read (`lineIndex == numLines` exits the loop first) | success, no OOB read | [x] |
| E15 | one past range | `numLines == N` exactly (largest valid) vs `N+1` (first invalid) — pair asserted together | `non-NULL` then `NULL` | [x] |

## Out-of-range enum values

`UTIL_createLinePointers` takes **no enum parameter** — its signature is
`(char*, size_t, size_t)`. There is no C enum anywhere in
`c_src/include/lib.h` or `c_src/src/lib.c`:

```
$ grep -c 'enum' c_src/include/lib.h c_src/src/lib.c
c_src/include/lib.h:0
c_src/src/lib.c:0
```

Consequently there is no "int with no valid variant" input class for this API.
The equivalent unconstrained-scalar class is `size_t` receiving arbitrary
64-bit values with no valid meaning, which rows E1, E2, E7, E8, E9 cover
(including the values that make the internal `numLines * sizeof(const char**)`
product wrap modulo 2^64).

## Deliberately NOT executed (undefined behaviour in the C)

These are documented rather than tested because running them corrupts the heap
or segfaults in **both** implementations, so a differential assertion is
meaningless:

* `numLines * 8` wraps to a value smaller than `numLines * 8` *and*
  `bufferSize > 0` → the C writes `numLines` pointers into an undersized
  allocation (out-of-bounds heap stores). The Rust reproduces this bit-for-bit
  via `wrapping_mul` + `ptr::add`, and row E8/E9 verify the wrap arithmetic
  itself with `bufferSize == 0`, where the OOB stores cannot happen.
* `buffer == NULL` with `bufferSize > 0` **and** `numLines > 0` → C dereferences
  a null pointer. Rows E4 and E11 cover the two safe corners of this case.
* `bufferSize` larger than the caller's real buffer with `numLines` not yet
  satisfied → C reads past the end of the caller's buffer.

## Test mapping and results

Every row above has a differential test in `translation/tests/differential.rs`
that constructs the exact condition, calls **both** `.so` exports, and asserts
the same sentinel (`NULL` vs non-`NULL`) — not merely "both failed".

| row | test |
|-----|------|
| E1  | `err_e1_malloc_failure_1_shl_60` |
| E2  | `err_e2_malloc_failure_size_max_div_8` |
| E3  | `err_e3_zero_buffer_size` |
| E4  | `err_e4_null_buffer_zero_size` |
| E5  | `err_e5_more_lines_than_records` |
| E6  | `err_e6_no_terminator_multiple_lines` |
| E7  | `err_e7_size_max_num_lines` |
| E8  | `err_e8_product_wraps_to_zero` |
| E9  | `err_e9_product_wraps_to_eight` |
| E10 | `err_e10_e15_one_past_valid_boundary` |
| E11 | `err_e11_null_buffer_zero_lines` |
| E12 | `err_e12_all_zero` |
| E13 | `err_e13_zero_lines_real_buffer` |
| E14 | `err_e14_oversized_buffer_size_safe` |
| E15 | `err_e10_e15_one_past_valid_boundary` |

All 15 rows pass. `NULL` is the library's only error channel — there is no
`errno`, no out-parameter, and no error enum — so "same error code" reduces to
"same sentinel", which every row asserts explicitly via `assert_both_null` /
`assert_both_non_null` rather than by comparing two unknowns.

## Strengthening: what the return value cannot observe

Two properties of the C are invisible through the return value, so the sentinel
comparison alone would leave them unverified. Both are now pinned:

1. **The `malloc` request size**, including whether
   `numLines * sizeof(const char**)` wraps. A non-`NULL` result requires
   `numLines <= bufferSize` (each loop iteration advances `pos` by at least 1),
   and `bufferSize` is bounded by a real allocation, so on every *reachable*
   success path the product is small and cannot wrap. Rows E7–E9 therefore
   cannot distinguish `wrapping_mul` from `saturating_mul` by return value —
   both end in `NULL`.
   `tests/support/malloc_trace.c` interposes `malloc`/`free` via `LD_PRELOAD`
   and `tests/differential.rs::interpose_malloc_size_and_free_parity` asserts
   the C and Rust request byte-identical sizes (and that the size equals
   `numLines.wrapping_mul(8)`, so a matched-pair regression also fails).
   Run with `./run_with_interpose.sh`.
2. **Whether the error path calls `free`.** A leak is not visible to a caller.
   `interpose_error_path_frees` asserts free-call parity across all three
   outcomes: success (0 frees), buffer-exhausted error (exactly 1 free), and
   `malloc`-failure error (0 frees, nothing was allocated).
3. **Reads past `bufferSize`.** `guard_page_no_read_past_buffer_size` places the
   buffer so its last byte abuts a `PROT_NONE` page; any over-read faults. This
   pins the inner loop's `pos + len < bufferSize` bound, which is otherwise
   unobservable (overshooting always leaves `pos >= bufferSize`, which exits the
   outer loop with the same `lineIndex` and the same recorded offsets).
