# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. There is exactly one valid
combination:

| # | Cargo features | CMake configuration | `cargo check` | [ ] |
|---|----------------|---------------------|---------------|-----|
| B1 | empty set (`--no-default-features`) | default | pass | [x] |

## Runtime Configurations

There are no runtime mode or option setters. The rows below are the
cross-product pruned to shapes distinguished by the C loop bounds, the
`len == 0` branch, the parser result branch, and the 100-element cap.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `fma_array` | `len <= 0`; all pointers may be null because the loop body is not entered; a nonempty output buffer remains unchanged | [x] |
| C2 | `fma_array` | `len == 1`; one valid element in each input and output array | [x] |
| C3 | `fma_array` | `len > 1`; multiple valid elements, including zero, positive, and negative operands whose C arithmetic is representable | [x] |
| C4 | `call_fma` | `len == 0`; `data` may be null and the early return yields zero | [x] |
| C5 | `call_fma` | `len == 1`; one valid data element | [x] |
| C6 | `call_fma` | `len > 1`; multiple valid data elements | [x] |
| C7 | `driver` | first conversion fails because input is empty, whitespace-only, or begins with a non-integer token | [x] |
| C8 | `driver` | exactly one integer is accepted, with lexical variants for sign and surrounding whitespace, then EOF or a rejecting suffix | [x] |
| C9 | `driver` | 2 through 99 integers are accepted, with varied whitespace, then EOF or a rejecting suffix | [x] |
| C10 | `driver` | at least 100 integers are available; the fixed loop cap consumes exactly 100 and ignores the remainder | [x] |
