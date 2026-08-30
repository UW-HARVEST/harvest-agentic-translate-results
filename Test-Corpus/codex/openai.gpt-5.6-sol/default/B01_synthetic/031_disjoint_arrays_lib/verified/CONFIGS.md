# Configuration Surface

The crate declares no Cargo features. The C header exposes `driver`, while the
C shared object additionally exports the lower-level `call_fma` and
`fma_array`; all three entry points are included below.

There are no runtime option, mode, flag, enum, element-type, format, or byte
order axes. The meaningful cross-product is therefore the input shapes that
the C loops and branches distinguish. Randomized rows include ordinary and
wrapping `int` arithmetic values.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `fma_array` | `len == 0`; no elements and no pointer dereference. | [x] |
| 2 | `fma_array` | `len == 1`; one output computed as `mul1[0] * mul2[0] + add[0]`. | [x] |
| 3 | `fma_array` | `len > 1`; many independently computed elements, including `int` boundary/wrapping values. | [x] |
| 4 | `call_fma` | `len == 0`; early return 0 before allocating VLAs or reading `data`. | [x] |
| 5 | `call_fma` | `len == 1`; one data element and return the sole output. | [x] |
| 6 | `call_fma` | `len > 1`; many data elements and return the last output. | [x] |
| 7 | `call_fma` | Large positive `len` beyond `driver`'s 100-element cap but within the process stack limit. | [x] |
| 8 | `driver` | Exactly one valid decimal integer, with optional sign/leading whitespace. | [x] |
| 9 | `driver` | Between 2 and 99 valid integers, using whitespace and adjacent signed-token separators accepted by `%d`. | [x] |
| 10 | `driver` | Exactly 100 valid integers; fill the fixed array and stop at the loop bound. | [x] |
| 11 | `driver` | More than 100 valid integers; ignore every token after the 100th and print the 100th. | [x] |

The zero-valid-token and valid-prefix-followed-by-malformed-token parser shapes
take the rejection branch and are listed in `ERRORS.md`.
