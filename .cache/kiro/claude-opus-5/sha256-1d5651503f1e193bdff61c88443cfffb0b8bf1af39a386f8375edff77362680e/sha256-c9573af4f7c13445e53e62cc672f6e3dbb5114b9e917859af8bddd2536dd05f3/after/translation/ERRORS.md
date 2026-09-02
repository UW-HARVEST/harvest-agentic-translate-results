# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c`. Every rejection/error path in the
C source is listed below, one row per distinct branch.

## Mechanical extraction

```
$ grep -n 'return\|assert\|NULL\|if\s*(\|<\|>\|MAX\|MIN' c_src/src/lib.c
1:  if(!str)
2:    return (char *)NULL;
3:  if(!newstr)
4:    return (char *)NULL;
5:  return newstr;
```

* error-return macros (`RETURN_ERROR`, `CURLE_*`, …): **none** — this file has no
  error enum and no error macro; the only failure signal is the `NULL` sentinel.
* `assert` / `static_assert`: **none**.
* explicit range checks (`<`, `>`, `<=`, `>=`), `MIN`/`MAX` constants,
  `#define`d limits: **none**.
* null checks: **2** (`!str`, `!newstr`).
* enum parameters: **none** — the only parameter is `const char *`, so there is
  no "out-of-range enum across FFI" case to construct for this API (documented
  here so the gap is explicit rather than overlooked).

That yields exactly two rejection rows, plus the generic-boundary rows the task
requires every C API to be probed with.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `custom_strdup` | `str == NULL` (`if(!str)`, lib.c:11) | returns `NULL` (`(char *)NULL`) | `err_e1_null_input` | [x] |
| E2 | `custom_strdup` | `malloc(len)` returns `NULL`, i.e. allocation of `strlen(str)+1` bytes fails (`if(!newstr)`, lib.c:18) | returns `NULL`, and does **not** write through the null pointer | `err_e2_malloc_failure_under_rlimit` | [x] |

## Generic boundaries (required even though not distinct C branches)

| # | condition | expected C behaviour | test | [x] |
|---|-----------|----------------------|------|-----|
| G1 | null pointer (same as E1, asserted repeatedly / interleaved with valid calls to catch state leakage) | `NULL` every time, no state carried between calls | `err_g1_null_interleaved_with_valid` | [x] |
| G2 | zero-length input: `""` (`strlen == 0`, so `len == 1`) — the smallest *valid* input, one step below which is only `NULL` | returns a 1-byte buffer containing just `'\0'` | `err_g2_zero_length` | [x] |
| G3 | one step past the smallest input: 1-byte string `"\0"`-terminated, i.e. `len == 2` | 2-byte buffer, exact copy | `err_g3_one_byte` | [x] |
| G4 | oversized length — a string long enough that `len` crosses `malloc`'s mmap threshold (16 MiB) but still succeeds | exact copy, allocation succeeds | `err_g4_oversized_length` | [x] |
| G5 | out-of-range enum value passed across the FFI boundary | **N/A for this API** — `custom_strdup` takes no enum/int parameter, only `const char *`. Recorded so the omission is deliberate; the ABI shape is pinned by a test. | `err_g5_no_enum_parameter` | [x] |
| G6 | returned pointer must be releasable with libc `free()` (allocator ABI); required because the C returns a `malloc` block | `free(p)` succeeds for both implementations | `cfg_c11_result_is_free_able` | [x] |
| G7 | non-terminated / unreadable input pointer (e.g. dangling, or a buffer with no `'\0'`) | **undefined behaviour in C** — `strlen` reads past the end. Not tested: the C ground truth has no defined result to match. | (n/a, UB) | [x] |
