# Error Surface

The C source contains no `RETURN_ERROR`, `return -1`, `return NULL`, error
enum, `assert`, explicit range rejection, or null-pointer rejection. Therefore
the mechanically derived source-defined rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

`call_fma(data, 0) == 0` is not an error: it is a defined valid special case
and is tracked in `CONFIGS.md`.

## Mandatory generic FFI boundaries

These cases are required by the verification protocol even though the C source
does not classify them as errors. Cases that execute undefined C behavior are
identified explicitly and are not assigned a portable result.

| # | function | boundary input | expected C result | tested |
|---|----------|----------------|-------------------|--------|
| G1 | `fma_array` | all pointers null, `len == 0` | returns normally without dereferencing a pointer | [x] |
| G2 | `fma_array` | all pointers null, `len < 0` | returns normally because the loop body is not entered | [x] |
| G3 | `fma_array` | a required pointer null, `len == 1` | undefined C behavior; on this build, child process terminates by signal | [x] |
| G4 | `call_fma` | `data == NULL`, `len == 0` | returns `0` before dereferencing `data` | [x] |
| G5 | `call_fma` | `data == NULL`, `len == 1` | undefined C behavior; on this build, child process terminates by signal | [x] |
| G6 | `driver` | `in == NULL` | undefined C behavior in `sscanf`; on this build, child process terminates by signal | [x] |
| G7 | `fma_array` | large valid length with fully allocated arrays | returns normally and writes exactly `len` outputs | [x] |
| G8 | `call_fma` | large valid length with fully allocated input | returns the last input element | [x] |

There are no enum parameters, documented numeric ranges, or public
min/max constants, so no out-of-range enum or one-past-range cases exist.
Negative `call_fma` lengths and lengths too large for the C VLA execute
undefined behavior and have no stable C result to compare.
