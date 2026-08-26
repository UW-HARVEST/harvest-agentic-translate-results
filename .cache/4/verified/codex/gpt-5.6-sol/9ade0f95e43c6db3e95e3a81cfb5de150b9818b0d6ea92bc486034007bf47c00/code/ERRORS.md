# Error Surface

Mechanical searches of `c_src/src/main.c` found no `RETURN_ERROR`, error enum,
`return -1`, `return NULL`, `assert`, explicit range check, null check, or
minimum/maximum constant. Neither public entry point returns a documented error
or rejection sentinel.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are therefore zero source-derived error rows to check.

## Generic FFI boundaries

| boundary | applicability |
|----------|---------------|
| null pointer | Not applicable: neither `driver(char)` nor `main(void)` accepts a pointer. |
| zero length | Not applicable: neither entry point accepts a length; byte value zero is valid and is covered by `CONFIGS.md`. |
| oversized length | Not applicable: neither entry point accepts a length. |
| one-past-range value | Not applicable at the ABI: C `char` accepts every bit pattern and the ABI truncates wider integer register values to `char`. |
| out-of-range enum | Not applicable: neither entry point accepts an enum. |
| empty input | Valid for `main`; `getchar()` returns `EOF`, converts to `char`, and is covered by `CONFIGS.md`. |

## Completion

- [x] Every source-derived rejection row has a differential test (zero rows).
- [x] Every applicable generic FFI boundary is represented in the configuration surface.
