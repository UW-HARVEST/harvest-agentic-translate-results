# ERRORS.md — error-surface table (Phase A → Phase C)

Derived mechanically from `c_src/src/lib.c`. The whole library is one function,
so this table is the *complete* enumeration of its rejection paths.

## Mechanical grep of every rejection / error construct in the C

```
c_src/src/lib.c:10:    if (bufferPtrs == NULL) return NULL;
c_src/src/lib.c:27:    if (lineIndex != numLines) {
c_src/src/lib.c:29:        free(bufferPtrs);
c_src/src/lib.c:30:        return NULL;
c_src/src/lib.c:33:    return linePointers;          <- the only success return
```

* `assert` / `NDEBUG`               : **none** (`grep -c assert c_src -r` → 0)
* error enums / `RETURN_ERROR` macro : **none**
* explicit range / bounds checks     : the three loop guards
  `lineIndex < numLines`, `pos < bufferSize`, `pos + len < bufferSize`,
  and the post-increment guard `if (pos < bufferSize) pos++`
* min/max constants                  : **none** declared; the only implicit
  limit is `SIZE_MAX` wraparound in `numLines * sizeof(const char**)`
* null checks                        : exactly one — `bufferPtrs == NULL`.
  **`buffer` is never null-checked**, so a null `buffer` with
  `bufferSize > 0` is dereferenced (rows 8/9 below).

So there are exactly **two distinct `return NULL` sites**, reachable through
several distinct triggers. One row per distinct trigger:

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `UTIL_createLinePointers` | `numLines` so large that `numLines * 8` exceeds addressable memory ⇒ `malloc` returns `NULL` (`numLines = 1<<60` ⇒ 2^63 bytes) | `NULL` (line 10), nothing allocated | `err_01_malloc_failure_huge_numlines` | [x] |
| 2 | `UTIL_createLinePointers` | same, near the very top of the range: `numLines = SIZE_MAX` ⇒ `SIZE_MAX*8` wraps to `SIZE_MAX-7`, still an impossible `malloc` | `NULL` (line 10) | `err_02_malloc_failure_size_max` | [x] |
| 3 | `UTIL_createLinePointers` | `numLines = SIZE_MAX/8 + 1 = 1<<61` ⇒ `numLines*8` **wraps to 0** ⇒ `malloc(0)` **succeeds**; with `bufferSize = 0` the loop body never runs so no OOB write happens, and `lineIndex(0) != numLines` | `NULL` via line 27–30 (`free` then `NULL`) — *not* via line 10 | `err_03_size_wrap_to_zero_bufsize_zero` | [x] |
| 4 | `UTIL_createLinePointers` | `numLines = (1<<61) + 1` ⇒ `numLines*8` wraps to `8` ⇒ `malloc(8)` succeeds; `bufferSize = 0` ⇒ `lineIndex(0) != numLines` | `NULL` via line 27–30 | `err_04_size_wrap_to_eight_bufsize_zero` | [x] |
| 5 | `UTIL_createLinePointers` | `bufferSize == 0` while `numLines > 0` (outer loop never entered because `pos < bufferSize` is false) | `NULL` via line 27–30 | `err_05_zero_buffersize_nonzero_numlines` | [x] |
| 6 | `UTIL_createLinePointers` | buffer contains **fewer** NUL-separated segments than `numLines` (buffer exhausted first, e.g. `"a\0b\0"` with `numLines = 5`) | `NULL` via line 27–30 | `err_06_fewer_segments_than_numlines` | [x] |
| 7 | `UTIL_createLinePointers` | off-by-one past the valid range: buffer holds exactly `k` segments, caller asks for `k + 1` | `NULL` via line 27–30 | `err_07_one_past_valid_numlines` | [x] |
| 8 | `UTIL_createLinePointers` | `buffer == NULL`, `bufferSize == 0`, `numLines > 0` — null pointer is *never dereferenced* because the loop guard fails first | `NULL` via line 27–30 (**no crash**) | `err_08_null_buffer_zero_size_nonzero_lines` | [x] |
| 9 | `UTIL_createLinePointers` | `buffer == NULL`, `bufferSize == 0`, `numLines == 0` — degenerate but **valid**: `malloc(0)` succeeds and `lineIndex == numLines` | **non-NULL** pointer (caller must `free`), zero elements written | `err_09_null_buffer_zero_size_zero_lines` | [x] |
| 10 | `UTIL_createLinePointers` | `bufferSize = SIZE_MAX` with a tiny real buffer — the C reads past the real allocation | **UB in both** (`buffer[pos+len]` walks off the end). Documented, *not* executed: both libraries would fault identically. | n/a — documented divergence-free UB | [x] |
| 11 | `UTIL_createLinePointers` | `numLines` whose `*8` wraps to a small non-zero value **and** `bufferSize > 0` (e.g. `numLines = 1<<61`, `bufferSize = 4`) — `malloc(0)` succeeds then `linePointers[0]` is written OOB | **heap overflow / UB in both**. Documented, *not* executed. | n/a — documented divergence-free UB | [x] |
| 12 | `UTIL_createLinePointers` | out-of-range "enum-like" integer values across the FFI boundary: `size_t` has no invalid variants, but every distinguished bit pattern is fed anyway — `0`, `1`, `SIZE_MAX`, `SIZE_MAX-1`, `1<<63`, `1<<61`, `(1<<61)±1`, `1<<60`, `SIZE_MAX/8`, `SIZE_MAX/8+1` for **both** `numLines` and `bufferSize` (paired so that no UB row above is hit) | identical result from both libraries | `err_12_extreme_scalar_matrix` | [x] |

## Notes on rows 10 & 11

These are the two genuinely undefined-behaviour inputs of the C API. The Rust
translation reproduces the C's *instruction sequence* for them (`wrapping_mul`
for the size, `wrapping_add` + `read()` for the buffer walk, plain `add` for the
element store), so it faults in the same place — but executing them inside the
test harness would abort the test process, so they are recorded here and
deliberately not run. Every *defined* rejection path (rows 1–9, 12) has a real
differential test.
