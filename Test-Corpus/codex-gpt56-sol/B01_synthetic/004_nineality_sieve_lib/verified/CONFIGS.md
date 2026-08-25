# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional source selection. There is exactly one valid build-time
configuration:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `--no-default-features` (empty feature set) | default |

## Runtime Configurations

The public header exposes only `void sieve(int start)`. There are no runtime
options, modes, flags, element types, formats, byte-order choices, pointers, or
counts. The C implementation branches on `val % 10 == 9`; for nonnegative
inputs, each decimal residue produces a distinct output length. Negative values
are a separate shape because C's signed remainder is negative until `val`
becomes nonnegative, so a negative value textually ending in 9 does not
terminate immediately.

Randomized cases use a fixed seed. Nonnegative rows include values near the
highest starts that reach a terminal value without signed overflow. Row 1 also
includes the zero boundary.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `sieve` | no options; nonnegative `start % 10 == 0`, including zero; print 10 values | [x] |
| 2 | `sieve` | no options; nonnegative `start % 10 == 1`; print 9 values | [x] |
| 3 | `sieve` | no options; nonnegative `start % 10 == 2`; print 8 values | [x] |
| 4 | `sieve` | no options; nonnegative `start % 10 == 3`; print 7 values | [x] |
| 5 | `sieve` | no options; nonnegative `start % 10 == 4`; print 6 values | [x] |
| 6 | `sieve` | no options; nonnegative `start % 10 == 5`; print 5 values | [x] |
| 7 | `sieve` | no options; nonnegative `start % 10 == 6`; print 4 values | [x] |
| 8 | `sieve` | no options; nonnegative `start % 10 == 7`; print 3 values | [x] |
| 9 | `sieve` | no options; nonnegative `start % 10 == 8`; print 2 values | [x] |
| 10 | `sieve` | no options; nonnegative `start % 10 == 9`; print 1 value | [x] |
| 11 | `sieve` | no options; bounded negative start (including values textually ending in 9); increment through zero and stop at positive 9 | [x] |
