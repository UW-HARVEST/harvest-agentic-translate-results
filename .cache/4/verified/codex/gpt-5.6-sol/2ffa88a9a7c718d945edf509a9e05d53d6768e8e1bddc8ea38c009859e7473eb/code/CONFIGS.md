# Configuration Surface

Build-time configuration has one valid combination: no Cargo features and no
CMake options. The C public API has no runtime option, mode, flag, enum,
element-type, format, or byte-order axis.

| Cargo feature set | CMake options | [ ] |
|-------------------|---------------|-----|
| empty (`--no-default-features --features ''`) | none | [x] |

The rows below cover every public entry point and the input shapes selected by
the C loop conditions, the aliasing used by the composed `driver` path, and
the fixed 100-element input boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `fma_array` | `len < 0`; pointers may be null because the loop performs no access | [x] |
| 2 | `fma_array` | `len == 0`; pointers may be null because the loop performs no access | [x] |
| 3 | `fma_array` | `len == 1`; four distinct valid arrays | [x] |
| 4 | `fma_array` | `len > 1`; four distinct valid arrays | [x] |
| 5 | `fma_array` | `len == 1`; `out == mul1 == mul2 == add`, as composed by `driver` | [x] |
| 6 | `fma_array` | `len > 1`; `out == mul1 == mul2 == add`, as composed by `driver` | [x] |
| 7 | `driver` | `len < 0`; `out` may be null and no output is emitted | [x] |
| 8 | `driver` | `len == 0`; `out` may be null and no output is emitted | [x] |
| 9 | `driver` | `len == 1`; one transformed line is emitted | [x] |
| 10 | `driver` | `len > 1`, including a length greater than `main`'s 100-element buffer; one transformed line per element | [x] |
| 11 | `main` | zero converted integers | [x] |
| 12 | `main` | exactly one converted integer | [x] |
| 13 | `main` | 2 through 99 converted integers | [x] |
| 14 | `main` | exactly 100 converted integers | [x] |
| 15 | `main` | more than 100 convertible integers; only the first 100 are consumed and emitted | [x] |
