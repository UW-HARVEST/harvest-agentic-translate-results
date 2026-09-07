# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/slicing.c`. Every rejection path in the C
source is one row. `slice()` is the only function, so all rows belong to it.

Mechanical inventory of the C source's rejection machinery:

```
$ grep -n 'return\|printf\|assert\|if' c_src/src/slicing.c
```

* `return 1;` — 3 occurrences (three distinct `if` branches, each preceded by
  its own `printf` diagnostic).
* `return 0;` — 1 occurrence (success).
* `assert` — 0 occurrences.
* No error enum; the API contract is `int` (`0` = success, `1` = rejected).
* No min/max constants; the only bound is the runtime `size_t len = strlen(mystr)`.
* No null check on `mystr` (dereferenced unconditionally by `strlen`).
* Null checks on `start_ptr` and `stop_ptr` are **not** rejections — null means
  "use the default" (`0` and `len` respectively).

The three checks in source order are:

1. `if (start_ptr) { start = *start_ptr; if (start > len) ... }`
2. `if (stop_ptr)  { stop  = *stop_ptr;  if (stop  > len) ... }`
3. `                                     if (stop <= start) ... }`

Checks 1 and 2 compare an `int` against a `size_t`. Under the usual arithmetic
conversions the `int` is converted to `size_t`, so **negative bounds
sign-extend into huge unsigned values and are rejected by these checks**, never
reaching the signed comparison in check 3. Check 3 is a plain signed `int`
comparison. This asymmetry is intentional in the table below.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `slice` | `start_ptr != NULL` and `*start_ptr > len` (e.g. `len=5`, `*start_ptr=6`) | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 2 | `slice` | `start_ptr != NULL` and `*start_ptr < 0` (e.g. `-1`) — sign-extends to a huge `size_t`, so it trips check 1, **not** check 3 | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 3 | `slice` | `start_ptr != NULL` and `*start_ptr == INT_MIN` (extreme negative; `(size_t)INT_MIN = 0xFFFFFFFF80000000`) | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 4 | `slice` | `start_ptr != NULL` and `*start_ptr == INT_MAX` with `len < INT_MAX` | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 5 | `slice` | `start_ptr != NULL`, `*start_ptr > len`, **and** `stop_ptr` also invalid — check 1 must win (precedence / short-circuit ordering) | prints only the **start** message; returns `1` (the stop messages are never printed) | [x] |
| 6 | `slice` | `stop_ptr != NULL`, `*start_ptr` valid (or `start_ptr == NULL`), and `*stop_ptr > len` (e.g. `len=5`, `*stop_ptr=6`) | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 7 | `slice` | `stop_ptr != NULL` and `*stop_ptr < 0` (e.g. `-1`) — sign-extends to a huge `size_t`, so it trips check 2 (`stop > len`), **not** check 3 | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 8 | `slice` | `stop_ptr != NULL` and `*stop_ptr == INT_MIN` | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 9 | `slice` | `stop_ptr != NULL` and `*stop_ptr == INT_MAX` with `len < INT_MAX` | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 10 | `slice` | `stop_ptr != NULL`, both bounds in `[0, len]`, and `*stop_ptr < *start_ptr` (e.g. `len=5`, `start=4`, `stop=2`) | prints `Error: stop must come after start!\n`; returns `1` | [x] |
| 11 | `slice` | `stop_ptr != NULL`, both bounds in `[0, len]`, and `*stop_ptr == *start_ptr` (empty half-open range is rejected, e.g. `start=stop=3`) | prints `Error: stop must come after start!\n`; returns `1` | [x] |
| 12 | `slice` | `stop_ptr != NULL`, `*stop_ptr == 0`, `start_ptr == NULL` (so `start` defaults to `0`) — `0 <= 0` trips check 3 | prints `Error: stop must come after start!\n`; returns `1` | [x] |
| 13 | `slice` | `stop_ptr != NULL`, `*stop_ptr == *start_ptr == len` (both at the exact upper boundary; check 2 passes because `len > len` is false, then check 3 fires) | prints `Error: stop must come after start!\n`; returns `1` | [x] |
| 14 | `slice` | `len == 0` (empty string) and `stop_ptr != NULL` with `*stop_ptr == 0` | prints `Error: stop must come after start!\n`; returns `1` | [x] |
| 15 | `slice` | `len == 0` and `start_ptr != NULL` with `*start_ptr == 1` (one step past the valid range `[0,0]`) | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 16 | `slice` | one step past the valid range: `*start_ptr == len + 1` for several `len` | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 17 | `slice` | one step past the valid range: `*stop_ptr == len + 1` for several `len` | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |

## Generic FFI-boundary boundaries also covered in Phase C

These are not distinct C branches but are required generic coverage:

| # | condition | expected C result | [x] |
|---|-----------|-------------------|-----|
| G1 | `start_ptr == NULL`, `stop_ptr == NULL` (both defaults) | prints the whole string; returns `0` | [x] |
| G2 | `start_ptr == NULL`, `stop_ptr != NULL` | uses `start = 0`; returns `0`/`1` per checks 2–3 | [x] |
| G3 | `start_ptr != NULL`, `stop_ptr == NULL` | uses `stop = len`; returns `0` if `start <= len` | [x] |
| G4 | zero length (`mystr = ""`) with every null/non-null pointer combination | see rows 14–15, G1–G3 | [x] |
| G5 | "oversized length": `*start_ptr` / `*stop_ptr` at `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `±2^31` wraparound neighbours | rows 3–4, 8–9 | [x] |
| G6 | out-of-range "enum" values across the FFI boundary — `slice` takes no enum parameter, so the equivalent surface is the *unconstrained `int`* bounds: the full `i32` domain is fuzzed, including values with no valid interpretation | must match C for every `i32` | [x] |
| G7 | `mystr == NULL` | **UB in C** (`strlen(NULL)` segfaults); not a defined rejection, so deliberately not exercised — documented here for completeness | n/a | n/a |

Row G6 note: the C prototype has no `enum` parameter, so there is no invalid
enum variant to smuggle across the boundary. The closest analogue — an `int`
whose value has no meaningful interpretation — is covered exhaustively at the
boundaries and randomly over the whole `i32` range in Phase C.
