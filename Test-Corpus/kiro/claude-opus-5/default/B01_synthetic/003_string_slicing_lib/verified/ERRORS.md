# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/slicing.c`. Every rejection path in the
whole library is listed. The grep basis:

```
$ grep -n 'return\|assert\|Error\|if (' c_src/src/slicing.c
43:    if (start_ptr) {
45:        if (start > len) {
46:            printf("Error: start is off the end of the string!\n");
47:            return 1;
52:    if (stop_ptr) {
54:        if (stop > len) {
55:            printf("Error: stop is off the end of the string!\n");
56:            return 1;
58:        if (stop <= start) {
59:            printf("Error: stop must come after start!\n");
60:            return 1;
68:    return 0;
```

There are exactly **three** `return 1` rejection statements and no `assert`,
no `errno` use, no error enum, and no `return NULL`. The only other exit is the
success `return 0`.

Critical C semantics that define these triggers:

* `size_t len = strlen(mystr);` — `len` is **unsigned 64-bit**.
* `start > len` and `stop > len` are therefore evaluated after the *usual
  arithmetic conversions* promote the **signed** `int` to `size_t`. A negative
  index becomes a huge unsigned value and **trips the "off the end" branch**.
* `stop <= start` is a plain **signed `int`** comparison, and it is evaluated
  **after** the `stop > len` check — so a negative `stop` reports
  "off the end of the string", never "must come after start".
* `start == len` and `stop == len` are **accepted** (the checks are strict `>`).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `slice` | `start_ptr != NULL` and `*start_ptr > (int)len`, e.g. `len=5, *start_ptr=6` | stdout `Error: start is off the end of the string!\n`; returns `1` |
| 2 | `slice` | `start_ptr != NULL` and `*start_ptr < 0` (sign-extends to a huge `size_t`, so it hits the *same* branch as #1), e.g. `*start_ptr=-1` | stdout `Error: start is off the end of the string!\n`; returns `1` |
| 3 | `slice` | `start_ptr != NULL` and `*start_ptr == INT_MIN` (extreme negative, `0xFFFFFFFF80000000` as `size_t`) | stdout `Error: start is off the end of the string!\n`; returns `1` |
| 4 | `slice` | `start_ptr != NULL` and `*start_ptr == INT_MAX` with `len < INT_MAX` | stdout `Error: start is off the end of the string!\n`; returns `1` |
| 5 | `slice` | `len == 0` (empty string) and `*start_ptr >= 1` | stdout `Error: start is off the end of the string!\n`; returns `1` |
| 6 | `slice` | start valid, `stop_ptr != NULL` and `*stop_ptr > (int)len`, e.g. `len=5, *stop_ptr=6` | stdout `Error: stop is off the end of the string!\n`; returns `1` |
| 7 | `slice` | start valid, `stop_ptr != NULL` and `*stop_ptr < 0` (unsigned promotion ⇒ "off the end", **not** the ordering error), e.g. `*stop_ptr=-1` | stdout `Error: stop is off the end of the string!\n`; returns `1` |
| 8 | `slice` | start valid, `stop_ptr != NULL`, `*stop_ptr == INT_MIN` | stdout `Error: stop is off the end of the string!\n`; returns `1` |
| 9 | `slice` | start valid, `stop_ptr != NULL`, `*stop_ptr == INT_MAX` with `len < INT_MAX` | stdout `Error: stop is off the end of the string!\n`; returns `1` |
| 10 | `slice` | `len == 0` (empty string), `stop_ptr != NULL`, `*stop_ptr >= 1` | stdout `Error: stop is off the end of the string!\n`; returns `1` |
| 11 | `slice` | `*stop_ptr <= *start_ptr` with `*stop_ptr` in range, `*stop_ptr < *start_ptr`, e.g. `len=5, start=4, stop=2` | stdout `Error: stop must come after start!\n`; returns `1` |
| 12 | `slice` | `*stop_ptr == *start_ptr` (equality also rejected, `<=`), e.g. `len=5, start=3, stop=3` | stdout `Error: stop must come after start!\n`; returns `1` |
| 13 | `slice` | `start_ptr == NULL` (⇒ `start = 0`) and `*stop_ptr == 0` — `0 <= 0` | stdout `Error: stop must come after start!\n`; returns `1` |
| 14 | `slice` | `len == 0` (empty string), `stop_ptr != NULL`, `*stop_ptr == 0` — passes the `> len` check, then `0 <= 0` | stdout `Error: stop must come after start!\n`; returns `1` |
| 15 | `slice` | `start_ptr == stop_ptr` (aliased pointer, same `int` object) ⇒ `stop == start` ⇒ ordering error, unless the value is out of range in which case #1/#6 fires first | stdout `Error: stop must come after start!\n`; returns `1` |
| 16 | `slice` | `*start_ptr == (int)len` (the accepted boundary) and `*stop_ptr == (int)len` ⇒ `stop <= start` | stdout `Error: stop must come after start!\n`; returns `1` |
| 17 | `slice` | *check-ordering precedence:* both indices invalid — `*start_ptr > len` **and** `*stop_ptr > len` | only the **start** message is printed (start is checked first); returns `1` |
| 18 | `slice` | *check-ordering precedence:* `*start_ptr` valid, `*stop_ptr` both `> len` **and** `<= start` (e.g. negative `stop` with `start=0`) | only the **stop off-the-end** message is printed; returns `1` |

## Generic FFI boundaries also covered by the tests

| boundary | C behaviour | how it is covered |
|----------|-------------|-------------------|
| `mystr == NULL` | `strlen(NULL)` dereferences a null pointer ⇒ **undefined behaviour**, in practice `SIGSEGV`. Not a *rejection* the C implements; there is no null check on `mystr`. | Tested differentially in a forked child process (`null_mystr_faults_identically`): both `.so`s must terminate with the same signal. |
| `start_ptr == NULL` | **valid** input, documented: `start = 0`. | Phase B (`CONFIGS.md` rows 1–7, 12–14, 23–24). |
| `stop_ptr == NULL` | **valid** input, documented: `stop = len`. | Phase B (`CONFIGS.md` rows 1–11, 22–24). |
| zero length | `len == 0` is a valid string; only index checks apply. | Rows 5, 10, 14 here; `CONFIGS.md` rows 1, 22. |
| oversized length | strings up to 4 KiB exercised. `len > INT_MAX` is not reachable in a test process (would need a >2 GiB allocation) and is documented as untested. | `CONFIGS.md` rows 4, 21. |
| one step past a valid range | `len+1` for both indices, and `len` (still valid) — the exact off-by-one boundary. | Rows 1, 6, 16; `CONFIGS.md` rows 11, 13, 17. |
| out-of-range enum values across FFI | **N/A** — the API has no `enum` parameter. The only "mode" selectors are the two pointer parameters (NULL / non-NULL), both of which are fully enumerated, and the two `int` indices, which are swept across their whole meaningful range including `INT_MIN`/`INT_MAX` (rows 3, 4, 8, 9). | Rows 2–4, 7–9 |
| indices written back (out-params) | C never writes through `start_ptr`/`stop_ptr` and never writes to `mystr`. | `CONFIGS.md` rows 25, 26 |

## Status

All 18 rows have a passing differential test in `tests/differential.rs`
(`phase_c_error_paths`), plus the generic-boundary tests above.

## Negative control

`./mutation_check.sh` confirms these rows are load-bearing. It injects 15
behaviour changes into `src/lib.rs` — including `>=` for `>` on each range
check, dropping the C's unsigned promotion on each index, `<` for `<=` on the
ordering check, swapping the two stop checks, swapping the two error messages,
changing the error return from `1` to `-1`, and adding a NULL check on `mystr`
that the C does not have — and every one is caught by this suite.
