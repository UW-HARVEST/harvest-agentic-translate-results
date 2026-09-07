# Error surface

Mechanical search covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error-return statements/macros, assertions, null checks, range checks,
min/max constants, and enums.

The C implementation contains no error return, sentinel, assertion, null
check, range check, enum, or explicit rejection branch. Its only public
function returns `void`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `flip_horizontal` | `img == NULL` | process receives `SIGSEGV` while reading `img->pix` | [x] |
| 2 | `flip_horizontal` | `img->pix == NULL`, `w == 1`, `h == 2` | process receives `SIGSEGV` on the first pixel read | [x] |
| 3 | `flip_horizontal` | zero dimensions: `w == 0`, `h == 0`, `pix == NULL` | returns normally; no pixels accessed | [x] |
| 4 | `flip_horizontal` | zero width with positive height: `w == 0`, `h > 1`, `pix == NULL` | returns normally; inner loop has zero iterations | [x] |
| 5 | `flip_horizontal` | negative height (`h == -1`) with null pixel storage | returns normally because `h / 2` is `0` and the outer loop has zero iterations | [x] |
| 6 | `flip_horizontal` | negative width (`w == -1`) with `h == 1` and null pixel storage | returns normally because the outer loop has zero iterations | [x] |
| 7 | `flip_horizontal` | oversized width (`w == INT_MAX`) with `h == 1` and null pixel storage | returns normally because the outer loop has zero iterations | [x] |
| 8 | `flip_horizontal` | extreme negative height (`h == INT_MIN`) with `w == 1` and null pixel storage | returns normally because `h / 2` is negative and the outer loop has zero iterations | [x] |

There are no enum parameters and no documented finite maximum dimension.
Positive dimensions that enter the loops but overflow C signed multiplication
or imply storage beyond the provided allocation have undefined behavior, so
they do not define a stable C error result to compare.
