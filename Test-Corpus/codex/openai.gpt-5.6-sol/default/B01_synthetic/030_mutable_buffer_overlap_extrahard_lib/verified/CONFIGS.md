# Configuration Surface

The C source has no runtime option, mode, flag, conditional-compilation feature,
element-type choice, format choice, or byte-order branch. Its configuration
surface comes from the two exported entry points, the `i < len` loop boundary,
pointer aliasing accepted by `fma_array`, and the one/many input shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `fma_array` | negative `len`; null pointers; loop executes zero times | [x] |
| 2 | `fma_array` | zero `len`; null pointers; loop executes zero times | [x] |
| 3 | `fma_array` | one element; four distinct buffers | [x] |
| 4 | `fma_array` | many elements; four distinct buffers | [x] |
| 5 | `fma_array` | many elements; `out == mul1` | [x] |
| 6 | `fma_array` | many elements; `out == mul2` | [x] |
| 7 | `fma_array` | many elements; `out == add` | [x] |
| 8 | `fma_array` | many elements; all four pointers alias (the shape used by `driver`) | [x] |
| 9 | `fma_array` | many elements; source arrays alias each other while `out` is distinct | [x] |
| 10 | `fma_array` | many elements; shifted overlap where writes affect a later source element | [x] |
| 11 | `fma_array` | many elements; reverse shifted overlap where writes do not affect prior elements | [x] |
| 12 | `fma_array` | large input (`len == 4096`) with distinct buffers | [x] |
| 13 | `driver` | zero elements | [x] |
| 14 | `driver` | one element; output captured through C stdout | [x] |
| 15 | `driver` | many elements; output captured through C stdout | [x] |
| 16 | `driver` | large input (`len == 4096`); output captured through C stdout | [x] |

`driver` composes copy, fully aliased `fma_array`, and decimal-line output.
Rows 13–16 therefore exercise the complete pipeline through the exported ABI.
