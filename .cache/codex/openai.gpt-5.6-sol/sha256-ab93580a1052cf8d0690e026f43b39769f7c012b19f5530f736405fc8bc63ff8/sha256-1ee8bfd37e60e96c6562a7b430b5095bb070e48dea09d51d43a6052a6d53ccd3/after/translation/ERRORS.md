# Error surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns, error
enums, assertions, explicit range/null checks, and min/max constants in
`../c_src/include` and `../c_src/src`. None occur. `rgb_to_hsv` returns `void`
and performs no source-defined input rejection.

The mandatory generic pointer boundaries are listed below. They are outside
the C language's defined behavior and therefore have no error code or sentinel;
the differential test invokes each case in an isolated process and compares
the observed process termination.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `rgb_to_hsv` | `dest == NULL`, `src` points to three readable floats | invalid write; process receives `SIGSEGV` on this build | [x] |
| 2 | `rgb_to_hsv` | `src == NULL`, `dest` points to three writable floats | invalid read; process receives `SIGSEGV` on this build | [x] |

## Inapplicable generic boundaries

There is no length parameter, so zero and oversized lengths cannot be passed.
There is no enum parameter, so an out-of-range enum cannot be passed. The
fixed input and output width is three `float` elements, established solely by
the unconditional `src[0..2]` reads and `dest[0..2]` writes.
The header documents no numeric valid range, so there is no numeric
one-past-range rejection to exercise.
