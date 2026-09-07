# Configuration Surface

The crate declares no Cargo features. The C API exposes no runtime mode,
option, flag, enum, format selector, or byte-order selector. The meaningful
cross-product therefore consists of the full exported entry-point set and the
input shapes distinguished by the C control flow.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `fma_array` | no options; `len == 0`; pointers may be null; loop executes zero times | [x] |
| 2 | `fma_array` | no options; `len == 1`; one output element; randomized full-width `int` operands | [x] |
| 3 | `fma_array` | no options; `len > 1`; many output elements; randomized lengths and full-width `int` operands | [x] |
| 4 | `fma_array` | no options; large valid length; fully allocated input/output arrays | [x] |
| 5 | `call_fma` | no options; `len == 0`; `data` may be null; early return | [x] |
| 6 | `call_fma` | no options; `len == 1`; returns the sole input element | [x] |
| 7 | `call_fma` | no options; `len > 1`; randomized lengths/data; returns the final input element through the composed `fma_array` path | [x] |
| 8 | `call_fma` | no options; large valid length; exercises the C VLA/composed path | [x] |
| 9 | `driver` | no options; empty or initially non-numeric input; zero successful scans; prints `0\n` | [x] |
| 10 | `driver` | no options; one valid decimal integer, including signs/whitespace; prints that value | [x] |
| 11 | `driver` | no options; 2–99 valid decimal integers; randomized count/values; prints the last value | [x] |
| 12 | `driver` | no options; valid prefix followed by malformed input before 100 values; scan stops and prints the last valid value | [x] |
| 13 | `driver` | no options; exactly 100 valid integers; reaches fixed-array capacity and prints the 100th value | [x] |
| 14 | `driver` | no options; more than 100 valid integers; ignores the suffix after the 100th and prints the 100th value | [x] |

`fma_array` with a negative `len` also executes zero loop iterations, but a
negative length is an invalid length-domain boundary and is tracked in
`ERRORS.md` rather than duplicated here.

No standalone binary executable is built by either `CMakeLists.txt` or
`Cargo.toml`, so binary stdout comparison is not applicable. The `driver`
function's stdout is captured and compared through both shared-library FFI
boundaries.
