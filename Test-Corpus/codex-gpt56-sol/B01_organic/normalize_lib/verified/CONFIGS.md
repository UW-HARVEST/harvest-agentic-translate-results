# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no CMake
options, conditional sources, or compile definitions. The complete feature
matrix therefore contains one configuration:

| # | Cargo features | C configuration | check command | status |
|---|----------------|-----------------|---------------|--------|
| F1 | none (`--no-default-features`) | default CMake configuration | `cargo check --no-default-features --features ""` | [x] |

## Runtime Configurations

The public surface has no runtime option or mode setters and one entry point:
`normalize`. Rows below derive from the two C conditions (`sum > 0.0f` and
`dest != src`), the two loops, IEEE-754 values that select each condition, and
the pointer layouts observable through the public API.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C1 | `normalize` | `size == 0`, distinct valid pointers; zero-iteration loops and zero-byte `memset` | [x] |
| C2 | `normalize` | `size == 0`, `dest == src`; zero-iteration loops and skipped `memset` | [x] |
| C3 | `normalize` | `size == 1`, finite value with positive square, distinct buffers | [x] |
| C4 | `normalize` | `size > 1`, finite values with finite positive sum, distinct buffers | [x] |
| C5 | `normalize` | `size > 1`, finite positive sum, in-place (`dest == src`) | [x] |
| C6 | `normalize` | finite positive sum, forward overlap (`dest == src + 1`) | [x] |
| C7 | `normalize` | finite positive sum, backward overlap (`src == dest + 1`) | [x] |
| C8 | `normalize` | many signed zero values, distinct buffers; `sum == 0`, destination zero-filled | [x] |
| C9 | `normalize` | many signed zero values, in-place; `sum == 0`, destination left unchanged | [x] |
| C10 | `normalize` | nonzero tiny values whose squared sum underflows to zero, distinct buffers | [x] |
| C11 | `normalize` | nonzero tiny values whose squared sum underflows to zero, in-place | [x] |
| C12 | `normalize` | input containing NaN, distinct buffers; comparison is false and destination is zero-filled | [x] |
| C13 | `normalize` | input containing NaN, in-place; comparison is false and input is unchanged | [x] |
| C14 | `normalize` | finite values whose squared sum overflows to positive infinity | [x] |
| C15 | `normalize` | input containing positive or negative infinity; positive-sum branch with zero reciprocal | [x] |
| C16 | `normalize` | `sum <= 0` path with partially overlapping distinct pointers; destination range is zero-filled | [x] |
| C17 | `normalize` | large valid vectors (4096 elements), finite positive sum | [x] |
| C18 | `normalize` | mixed signs and magnitudes with randomized finite bit patterns and finite positive sum | [x] |
| C19 | `normalize` | positive sum with an output buffer initialized to arbitrary bytes | [x] |
| C20 | `normalize` | zero/NaN sum with an output buffer initialized to arbitrary bytes | [x] |
