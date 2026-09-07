# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Mechanically derived from `c_src/src/slicing.c`. Every rejection path in the
file, found by grepping for every `return`, every `printf` on an error branch,
every comparison used as a guard, and every `assert` (there are none).

Raw inventory of the C control flow (`grep -n 'return\|assert\|printf\|if (' c_src/src/slicing.c`):

```
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
65:    printf("%.*s\n", stop - start, mystr + start);
67:    return 0;
```

There are exactly **three** `return 1;` (rejection) statements, no `assert`,
no `return NULL`, no `return -1`, no error enum, and no named min/max
constants. The only implicit limits are the C integer-conversion boundaries,
which produce distinct *input classes* through the same branches; those are
listed as separate rows because they are separate inputs a caller can supply.

`len` is `size_t` (unsigned 64-bit) and `start` / `stop` are `int`. Both
`start > len` and `stop > len` therefore undergo the usual arithmetic
conversions, promoting the **signed** `int` to `size_t`: any negative index
becomes a huge unsigned value and trips the "off the end" branch.
`stop <= start` is a plain **signed** `int` comparison and runs **after** the
`stop > len` check.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `slice` | `start_ptr != NULL` and `*start_ptr > strlen(mystr)` (plain positive overshoot, e.g. `len=5`, `start=6`) | stdout `Error: start is off the end of the string!\n`; returns `1` | `e1_start_off_end_positive` | [x] |
| E2 | `slice` | `start_ptr != NULL` and `*start_ptr < 0` (negative promotes to huge `size_t`, so `start > len`), e.g. `start=-1` | same as E1: `Error: start is off the end of the string!\n`; returns `1` | `e2_start_negative` | [x] |
| E3 | `slice` | `start_ptr != NULL` and `*start_ptr == INT_MIN` / `INT_MAX` (extreme signed values, both `> len` after promotion) | same as E1; returns `1` | `e3_start_int_extremes` | [x] |
| E4 | `slice` | `start_ptr != NULL`, `*start_ptr` valid, `stop_ptr != NULL` and `*stop_ptr > strlen(mystr)` (positive overshoot) | stdout `Error: stop is off the end of the string!\n`; returns `1` | `e4_stop_off_end_positive` | [x] |
| E5 | `slice` | `stop_ptr != NULL` and `*stop_ptr < 0` (negative promotes to huge `size_t`; hits the `stop > len` branch **before** the `stop <= start` branch) | `Error: stop is off the end of the string!\n`; returns `1` — **not** the "must come after" message | `e5_stop_negative_precedence` | [x] |
| E6 | `slice` | `stop_ptr != NULL` and `*stop_ptr == INT_MIN` / `INT_MAX` | `Error: stop is off the end of the string!\n`; returns `1` | `e6_stop_int_extremes` | [x] |
| E7 | `slice` | `stop_ptr != NULL`, `*stop_ptr <= strlen(mystr)`, and `*stop_ptr < *start_ptr` (strictly before start) | stdout `Error: stop must come after start!\n`; returns `1` | `e7_stop_before_start` | [x] |
| E8 | `slice` | `stop_ptr != NULL`, `*stop_ptr <= strlen(mystr)`, and `*stop_ptr == *start_ptr` (equal — the `<=` makes an empty slice an error) | `Error: stop must come after start!\n`; returns `1` | `e8_stop_equals_start` | [x] |
| E9 | `slice` | `start_ptr == NULL` (so `start = 0`) and `stop_ptr != NULL` with `*stop_ptr == 0` — `0 <= 0` | `Error: stop must come after start!\n`; returns `1` | `e9_null_start_zero_stop` | [x] |
| E10 | `slice` | empty string (`len == 0`) with `stop_ptr != NULL`, `*stop_ptr == 0`: passes `stop > len` (`0 > 0` false), then fails `stop <= start` | `Error: stop must come after start!\n`; returns `1` | `e10_empty_string_zero_stop` | [x] |
| E11 | `slice` | error-precedence: **both** `*start_ptr > len` **and** `*stop_ptr > len`. The start check is first, so only the start message is emitted and `stop` is never read. | `Error: start is off the end of the string!\n` only; returns `1` | `e11_start_error_wins` | [x] |
| E12 | `slice` | boundary "one step past valid": `*start_ptr == len + 1` and, separately, `*stop_ptr == len + 1` (`len` itself is *valid*, `len+1` is not) | corresponding "off the end" message; returns `1` | `e13_one_past_boundary` | [x] |

### Rejections that are undefined behaviour in the C (documented, not asserted equal)

| # | trigger | C behaviour | handling |
|---|---------|-------------|----------|
| U1 | `mystr == NULL` | `strlen(NULL)` dereferences a null pointer → `SIGSEGV`. The C performs **no** null check (verified: no `if (mystr)` anywhere in the file). | The Rust translation calls the same `strlen` with the same pointer, so it faults identically. Verified out-of-process in `u1_null_string_both_crash_identically`: both libraries are called in a forked child and both children die with the same signal. Not a return-value comparison because the C has no defined return here. |
| U2 | `mystr` non-NULL but not NUL-terminated | `strlen` reads out of bounds → UB | not tested; identical `strlen` call in both. |
| U3 | `strlen(mystr) > INT_MAX` with `stop_ptr == NULL` | `stop = len` truncates `size_t`→`int`, yielding a negative `stop`; `stop - start` then feeds `%.*s` a negative precision (glibc treats negative precision as omitted). | not tested — requires a >2 GiB string. The Rust performs the same `len as c_int` truncation and the same subtraction, so the bit pattern handed to `printf` is identical. |

### Notes on "out-of-range enum values across the FFI boundary"

The public API declares **no enums** and **no mode/flag parameters**
(`c_src/include/slicing.h` is a single function declaration with three pointer
arguments). The equivalent class of "any int is a legal input" bugs is covered
by rows E2, E3, E5, E6 and E12, which pass the full signed `int` range —
including negatives and `INT_MIN`/`INT_MAX` — through `*start_ptr` /
`*stop_ptr`, plus the randomized full-`i32`-range fuzz row C13 in `CONFIGS.md`.

**Gate: 12/12 rows have a passing differential test (+ U1 verified
out-of-process). PASS.**
