# Error Surface

Mechanically derived from every `if` branch that returns an error or null
sentinel in `c_src/src/goto.c`. The source contains no assertions, enums,
length parameters, named min/max constants, or explicit filename null check.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `forward_goto_example` | `x < 0` (`goto error`, lines 30-31) | writes `Error: negative input\n` to stderr and returns `-1` | [x] |
| 2 | `open_with_cleanup` | `fopen(filename, "r")` returns null (lines 43-45) | writes `Error: opening or processing file %s\n` to stderr and returns `NULL` | [x] |
| 3 | `open_with_cleanup` | the read loop ends and `ferror(fp) != 0` (lines 49-54) | writes the filename error, closes `fp`, and returns `NULL` | [x] |
| 4 | `driver` | `forward_goto_example(num) == -1`, reached for `num < 0` (lines 66-68) | returns `-1` without opening `filename` | [x] |
| 5 | `driver` | `open_with_cleanup(filename) == NULL` because `fopen` failed (lines 73-75) | returns `-2` after writing the processing, goto-output, and filename-error messages | [x] |
| 6 | `driver` | `open_with_cleanup(filename) == NULL` because reading set `ferror` (lines 73-75) | returns `-2` after writing the processing, goto-output, and filename-error messages | [x] |

Additional generic FFI boundaries required by Phase C:

| # | function | boundary | expected parity | verified |
|---|----------|----------|-----------------|----------|
| G1 | `open_with_cleanup` | null `filename` | libc `fopen` returns null; writes the filename error with `(null)` and returns `NULL` | [x] |
| G2 | `driver` | `num < 0`, null `filename` | returns `-1`; the filename is not evaluated | [x] |
| G3 | `driver` | `num >= 0`, null `filename` | writes the processing/goto output and `(null)` filename error, then returns `-2` | [x] |

There are no length or enum parameters, so zero/oversized lengths and invalid
enum discriminants do not apply to this ABI.
