# Error Surface

The three explicit rejection branches come from `src/slicing.c:45`,
`src/slicing.c:55`, and `src/slicing.c:59`. Row 4 records the generic null
boundary required for every public API; C does not check it before `strlen`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `slice` | `start_ptr != NULL` and `(size_t)*start_ptr > strlen(mystr)`; this includes every negative `int` on this platform | prints `Error: start is off the end of the string!\n` and returns `1` | [x] |
| 2 | `slice` | start passed validation, `stop_ptr != NULL`, and `(size_t)*stop_ptr > strlen(mystr)`; this includes every negative `int` on this platform | prints `Error: stop is off the end of the string!\n` and returns `1` | [x] |
| 3 | `slice` | start and stop passed their upper-bound checks, `stop_ptr != NULL`, and `*stop_ptr <= start` | prints `Error: stop must come after start!\n` and returns `1` | [x] |
| 4 | `slice` | `mystr == NULL` (with either optional index pointer null or non-null) | no explicit rejection; this build terminates with `SIGSEGV` in `strlen(NULL)` | [x] |

There is no length parameter and no enum parameter. Zero-length input and null
`start_ptr`/`stop_ptr` are valid configurations in `CONFIGS.md`; oversized and
one-past-end indices are covered by rows 1 and 2.
