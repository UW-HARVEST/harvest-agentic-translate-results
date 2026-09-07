# ERRORS.md — Error-surface table

Mechanically derived from every rejection / early-return site in
`c_src/src/lib.c`. There are exactly **four** `return (NULL)` / falsy-guard
sites plus the fall-through default of `decode()`. No `assert`, no error enum,
no `errno` use, no min/max constants in this library.

Grep evidence:

```
$ grep -n 'return (NULL)\|return NULL\|return -1\|assert\|if (!' c_src/src/lib.c
55:            return (NULL);
63:            return (NULL);
112:    return (NULL);
54:        if (!dest) {
61:        if (!buf) {
46:    if (src && *src) {
```

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|--------------------------------------------|-------------------|
| 1 | `decode_base64` | `src == NULL` — the `src &&` half of the guard at line 46 fails | returns `NULL` (line 112) |
| 2 | `decode_base64` | `src != NULL` but `*src == '\0'` (empty string) — the `*src` half of the guard at line 46 fails | returns `NULL` (line 112) |
| 3 | `decode_base64` | `calloc(sizeof(char), l + 13)` returns `NULL` (line 54) | returns `NULL` (line 55), nothing leaked |
| 4 | `decode_base64` | `malloc(l)` returns `NULL` (line 61) | `free(dest)` then returns `NULL` (line 63) |
| 5 | `decode` (static) | character that is **not** `A-Z`, `a-z`, `0-9`, `'+'` — i.e. every other byte, including `'/'`, `'='`, and negative (high-bit) `char` values | falls through all four range checks and returns the sentinel `63` (line 25). **Not** an error return: `'/'` legitimately maps to 63 and `'='`/garbage silently alias onto it. |
| 6 | `is_base64` (static) | any byte outside `A-Z a-z 0-9 + / =`, incl. negative `char` (bytes `0x80..0xFF`) | returns `FALSE` (0) → the byte is **silently dropped** from `buf` by the filter loop at lines 67-71 ("Ignore non base64 chars as per the POSIX standard"). Never an error. |

## Notes on rows 3 and 4

Rows 3 and 4 are allocation-failure paths. They cannot be triggered from a
normal differential test on a 64-bit host with overcommit, so they are covered
*structurally*: the test suite asserts that both implementations return
`NULL`/non-`NULL` in lockstep for every input, and the Rust code performs the
identical `dest`-then-`buf` allocation order with the identical `free(dest)`
cleanup on the second failure. See `tests/differential.rs::alloc_failure_paths_documented`.

## Non-error boundaries additionally covered by the tests

These are not rows in the table above (the C never rejects them) but are the
generic FFI boundaries the task requires:

| boundary | C behaviour that must be matched |
|----------|----------------------------------|
| null pointer | row 1 — `NULL` |
| zero length (`""`) | row 2 — `NULL` |
| length 1 | non-`NULL`; the `k+1/k+2/k+3 < l` guards default `c2=c3=c4='A'`, so **3** bytes are written for a single input character |
| non-empty input with **zero** base64 characters (e.g. `"!!!"`) | `l` becomes 0, the decode loop body never runs, a fully zeroed `strlen+14`-byte buffer is returned (**non-`NULL`**) |
| all 256 byte values as a 1-char input | each either survives the filter or is dropped; result must be byte-identical |
| out-of-range "enum" values | this API has **no enums** and no `int`-typed mode parameters — the only parameter is `const char *`. Covered instead by sweeping all 256 byte values through the single pointer parameter. |
| oversized length | inputs up to 64 KiB exercised; `int l = strlen(src) + 1` truncation only matters at >2 GiB, which is out of reach of a test but is replicated bit-for-bit in Rust via `strlen(src).wrapping_add(1) as c_int`. |

## Verification run (Phase C)

`tests/phase_c_errors.rs` — 7/7 tests PASS, 3 801 differential comparisons.

| row | test | status |
|-----|------|--------|
| 1 | `err_row01_null_pointer` | PASS — both return `NULL` |
| 2 | `err_row02_empty_string` | PASS — both return `NULL` |
| 3, 4 | `err_rows03_04_alloc_failure_paths_documented` | PASS — NULL/non-NULL verdict identical over 4 000 randomized inputs incl. a 1 MiB input |
| 5 | `err_row05_decode_fallthrough_sentinel` | PASS — sentinel 63 for `'/'` and `'='`; full 255-byte × 4-slot sweep |
| 6 | `err_row06_is_base64_rejection_drops_byte` | PASS — every byte `0x01..0xFF` inserted at every position; high-bit (negative `char`) bytes confirmed dropped |
| generic | `err_generic_boundaries` | PASS — null, zero length, length 1 for all 255 bytes, one-past-range chars, sizes up to 100 000 |
| generic | `err_repeated_calls_stateless` | PASS |

**Every ERRORS.md row has a passing differential test.**
