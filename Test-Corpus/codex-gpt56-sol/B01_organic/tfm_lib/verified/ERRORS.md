# Error Surface

The source scan covered `RETURN_ERROR`, error-valued `return` statements,
`NULL`, `assert`, `if`, `switch`, preprocessor conditionals, enums, and
min/max constants in all files under `c_src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no explicit rejection branches. `tfm` returns `void`, and the only
`if` in the implementation selects an arithmetic path rather than rejecting
input.

## Generic Boundary Probes

These are required FFI boundary probes, not rejection branches found in C.

| # | function | boundary input | observed/expected C result | Status |
|---|----------|----------------|----------------------------|--------|
| G1 | `tfm` | `dest = NULL`, `src = NULL`, `count = 0` | Returns normally; pointers are not dereferenced | [x] |
| G2 | `tfm` | `dest = NULL`, `src = NULL`, `count = INT_MIN` | Returns normally; pointers are not dereferenced | [x] |
| G3 | `tfm` | non-null sentinel buffers, `count = -1` | Returns normally; destination is unchanged | [x] |
| G4 | `tfm` | non-null sentinel buffers, `count = 0` | Returns normally; destination is unchanged | [x] |
| G5 | `tfm` | `dest = NULL`, valid source, `count = 1` | Process receives `SIGSEGV` on the first store | [x] |
| G6 | `tfm` | valid destination, `src = NULL`, `count = 1` | Process receives `SIGSEGV` on the first load | [x] |
| G7 | `tfm` | storage for one item followed by an inaccessible guard page, `count = 2` | Process receives `SIGSEGV` when the oversized length reaches the guard page | [x] |

The API declares no enums and documents no numeric range for float elements or
for `count`; therefore there is no out-of-range enum value or documented
one-past-range scalar to probe.
