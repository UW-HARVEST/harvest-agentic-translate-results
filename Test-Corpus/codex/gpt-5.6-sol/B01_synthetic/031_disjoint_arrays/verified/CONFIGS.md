# Configuration Surface

## Build-Time Configuration

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or compile definitions. There is exactly one valid feature
combination:

| # | Cargo invocation configuration | CMake configuration | |
|---|--------------------------------|---------------------|-|
| 1 | `--no-default-features --features ""` (no features) | default, with requested position-independent-code setting | [x] |

## Runtime Configuration

There are no public headers and no runtime mode/option flags. The rows below
are the cross-product pruned to branches and shapes the C source actually
distinguishes: signed loop counts, zero/one/many array lengths, the 100-element
input bound, `%d` token forms, and scan termination.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `fma_array` | `len < 0`; pointers may be null because the loop executes zero times; output remains untouched | [x] |
| 2 | `fma_array` | `len == 0`; pointers may be null because the loop executes zero times; output remains untouched | [x] |
| 3 | `fma_array` | `len == 1`; valid disjoint arrays; randomized negative, zero, and positive values whose multiply-add is representable as `int` | [x] |
| 4 | `fma_array` | `len > 1`; valid disjoint arrays; randomized lengths and values whose element-wise multiply-add is representable as `int` | [x] |
| 5 | `call_fma` | `len == 0`; `data` may be null; early return is `0` | [x] |
| 6 | `call_fma`, `fma_array` | `len == 1`; randomized `int` value including `INT_MIN` and `INT_MAX`; result is the sole element | [x] |
| 7 | `call_fma`, `fma_array` | `len > 1`; randomized ordinary and large practical lengths with values including integer boundaries; result is the last element | [x] |
| 8 | `main`, `call_fma`, `fma_array` | one accepted `%d` token, including optional sign and surrounding C whitespace; prints that value | [x] |
| 9 | `main`, `call_fma`, `fma_array` | 2 through 99 accepted `%d` tokens with mixed whitespace/sign forms; prints the last value | [x] |
| 10 | `main`, `call_fma`, `fma_array` | exactly 100 accepted `%d` tokens; prints token 100 | [x] |
| 11 | `main`, `call_fma`, `fma_array` | more than 100 accepted `%d` tokens; the fixed loop bound ignores all tokens after token 100 and prints token 100 | [x] |
| 12 | `main`, `call_fma`, `fma_array` | accepted `%d` tokens at `INT_MIN`, `INT_MAX`, and platform-observed out-of-range text conversion boundaries | [x] |
