# Error Surface

The API has no error enum or error sentinel. It returns `0` for each rejected
header/comparison and `1` for a match. Rows 1-5 are the mechanically enumerated
validity checks in `hdr_valid`, in C short-circuit evaluation order.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---:|---|---|---:|:---:|
| 1 | `hdr_compare` / `hdr_valid(h2)` | `h2[0] != 0xff` | `0` | [x] |
| 2 | `hdr_compare` / `hdr_valid(h2)` | row 1 passes, but `(h2[1] & 0xf0) != 0xf0 && (h2[1] & 0xfe) != 0xe2` | `0` | [x] |
| 3 | `hdr_compare` / `hdr_valid(h2)` | rows 1-2 pass, but `((h2[1] >> 1) & 3) == 0` | `0` | [x] |
| 4 | `hdr_compare` / `hdr_valid(h2)` | rows 1-3 pass, but `(h2[2] >> 4) == 15` | `0` | [x] |
| 5 | `hdr_compare` / `hdr_valid(h2)` | rows 1-4 pass, but `((h2[2] >> 2) & 3) == 3` | `0` | [x] |

## Generic FFI Boundaries

The public function has no length argument, enum, min/max constant, or
out-of-range scalar parameter. It unconditionally requires each pointer that
is reached by short-circuit evaluation to address at least three readable
bytes. Consequently, zero/oversized lengths and out-of-range enum values are
not representable in this API.

The C source has no null check. Null dereferences are undefined C behavior, so
they have no portable expected return value. The differential suite
characterizes them in isolated subprocesses on the test platform and requires
both shared libraries to terminate the same way.

| # | function | boundary condition | expected C result on test platform | tested |
|---:|---|---|---|:---:|
| 6 | `hdr_compare` | `h2 == NULL` | subprocess termination signal | [x] |
| 7 | `hdr_compare` | valid `h2`, `h1 == NULL` | subprocess termination signal | [x] |
| 8 | `hdr_compare` | row-1-invalid `h2`, `h1 == NULL` | `0` because `h1` is not evaluated | [x] |
