# Error-Surface Table

Mechanically derived from every error-producing branch, null check, and
error/sentinel return in `../c_src/src/goto.c`. There are no assertions,
enums, public length parameters, or named min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `forward_goto_example` | `x < 0` (including `-1` and `INT_MIN`) | writes `Error: negative input\n` to stderr and returns `-1` | [x] |
| 2 | `open_with_cleanup` | `fopen(filename, "r")` returns `NULL` for a non-null pathname | writes `Error: opening or processing file <filename>\n` to stderr and returns `NULL` | [x] |
| 3 | `open_with_cleanup` | `fgets` stops and `ferror(fp) != 0` (a directory stream on this platform) | writes `Error: opening or processing file <filename>\n`, closes `fp`, and returns `NULL` | [x] |
| 4 | `driver` | `forward_goto_example(num) == -1`, caused by `num < 0` | returns `-1` without opening the filename | [x] |
| 5 | `driver` | `open_with_cleanup(filename) == NULL` because `fopen` failed | returns `-2` after the processing/goto output and file error | [x] |
| 6 | `driver` | `open_with_cleanup(filename) == NULL` because `ferror(fp) != 0` | returns `-2` after the processing/goto output and file error | [x] |
| 7 | `open_with_cleanup` | generic pointer boundary: `filename == NULL`, making `fopen` return `NULL` on the target libc | writes `Error: opening or processing file (null)\n` to stderr and returns `NULL` | [x] |
| 8 | `driver` | generic pointer boundary: `num >= 0` and `filename == NULL` | returns `-2`; stdout contains processing/goto output and stderr contains the `(null)` file error | [x] |

Generic boundary audit: zero is a valid `num` and is covered in
`CONFIGS.md`; the API has no length parameters or enum parameters, so
zero/oversized lengths and out-of-range enum discriminants are not applicable.
