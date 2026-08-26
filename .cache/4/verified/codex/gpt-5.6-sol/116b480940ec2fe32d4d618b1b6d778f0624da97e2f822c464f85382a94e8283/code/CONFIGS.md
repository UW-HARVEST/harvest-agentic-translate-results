# Configuration Surface

## Build-time configurations

`Cargo.toml` declares no features and no defaults. `c_src/CMakeLists.txt`
declares no options or conditional compilation. The complete build-time
configuration set therefore has one member:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features --features ""` | default | [x] |

## Runtime and input configurations

The table is derived from both public declarations in `include/match.h`, the
loop boundaries in both C sources, `N_SMOOTH == 16`, the early-return
comparison in `match`, and pointer mutation/alias behavior. The public header
uses `double`, but `spectral_contrast.c` omits that header: on the default
compiler, `<math.h>` defines its private `float_t` as `float`. Thus
`spectral_contrast` reinterprets public `double *` storage as `float *`, and
`match` passes its preprocessed `double` VLAs through that implementation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `spectral_contrast` | `length == 0`; empty/null buffers | [x] |
| 2 | `spectral_contrast` | `length == 1`; disjoint finite nonzero vectors | [x] |
| 3 | `spectral_contrast` | `length > 1`; disjoint finite vectors with positive, negative, and mixed signs | [x] |
| 4 | `spectral_contrast` | `length > 1`; exact alias (`a == b`) | [x] |
| 5 | `spectral_contrast` | `length > 1`; partially overlapping buffers | [x] |
| 6 | `spectral_contrast` | positive length; one or both vectors have zero magnitude | [x] |
| 7 | `spectral_contrast` | positive length; inputs contain `NaN`, `+Inf`, or `-Inf` | [x] |
| 8 | `match` | `bins == 1`; total comparison takes the early `return 0` branch | [x] |
| 9 | `match` | `bins == 1`; full pipeline, including skipped differentiation loop and forced final zero | [x] |
| 10 | `match` | `2 <= bins < N_SMOOTH`; early total rejection | [x] |
| 11 | `match` | `2 <= bins < N_SMOOTH`; full pipeline and final comparison true | [x] |
| 12 | `match` | `2 <= bins < N_SMOOTH`; full pipeline and final comparison false | [x] |
| 13 | `match` | `bins == N_SMOOTH`; exact smoothing-kernel width | [x] |
| 14 | `match` | `bins > N_SMOOTH`; full-width interior windows and truncated tail windows | [x] |
| 15 | `match` | positive `bins`; exact alias (`test == reference`) | [x] |
| 16 | `match` | positive `bins`; finite `threshold <= 0` | [x] |
| 17 | `match` | positive `bins`; finite `threshold > 1` | [x] |
| 18 | `match` | positive `bins`; `threshold` is `NaN`, `+Inf`, or `-Inf` | [x] |
