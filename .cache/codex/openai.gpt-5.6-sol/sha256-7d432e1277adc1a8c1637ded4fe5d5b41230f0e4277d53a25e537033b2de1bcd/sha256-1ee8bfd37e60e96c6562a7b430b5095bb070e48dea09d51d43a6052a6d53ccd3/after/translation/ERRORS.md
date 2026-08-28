# Error Surface

This table comes from every rejection branch in `src/goto.c`. There are no
assertions, enums, error macros, explicit filename null checks, or numeric
min/max constants in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 [x] | `forward_goto_example` | `x < 0` | writes `Error: negative input\n` to stderr and returns `-1` |
| 2 [x] | `open_with_cleanup` | `fopen(filename, "r")` returns `NULL` | writes `Error: opening or processing file <filename>\n` to stderr and returns `NULL` |
| 3 [x] | `open_with_cleanup` | the `fgets` loop ends and `ferror(fp)` is nonzero | writes the file error to stderr, closes `fp`, and returns `NULL` |
| 4 [x] | `driver` | `forward_goto_example(num)` returns `-1` (`num < 0`) | returns `-1` immediately; `filename` is not accessed |
| 5 [x] | `driver` | `open_with_cleanup(filename)` returns `NULL` because `fopen` failed | returns `-2` after the processing and goto-output messages |
| 6 [x] | `driver` | `open_with_cleanup(filename)` returns `NULL` because reading set `ferror` | returns `-2` after the processing and goto-output messages |

Generic FFI boundaries not explicitly rejected by C are tracked by tests:

- null `filename` with negative `num` (short-circuited and never accessed);
- null `filename` on a path that reaches `fopen` (process-level behavior);
- empty filename;
- `INT_MIN`, `-1`, `0`, and `INT_MAX`;
- no length parameters or enum parameters exist in this API.
