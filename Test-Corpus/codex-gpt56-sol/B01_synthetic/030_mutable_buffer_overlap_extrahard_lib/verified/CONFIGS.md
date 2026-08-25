# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional sources/definitions. The complete valid feature set is:

| # | Cargo invocation feature set | CMake configuration | [ ] |
|---|------------------------------|---------------------|-----|
| 1 | `--no-default-features` (empty set) | Default configuration | [x] |

## Runtime Configurations

The public surface is the union of the installed header and dynamic exports:
`driver` and the lower-level `fma_array`. The C source has no runtime options,
modes, flags, formats, element-type choices, or byte-order branches. Its
observable axes are loop cardinality and legal pointer aliasing. Full and
partial output/input aliases matter because each loop iteration reads before
writing and can affect later iterations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `fma_array` | `len == 0`; non-null, disjoint buffers; no iterations | [x] |
| 2 | `fma_array` | `len == 1`; four disjoint buffers | [x] |
| 3 | `fma_array` | `len > 1`; four disjoint buffers | [x] |
| 4 | `fma_array` | `len > 1`; `out == mul1`, other inputs disjoint | [x] |
| 5 | `fma_array` | `len > 1`; `out == mul2`, other inputs disjoint | [x] |
| 6 | `fma_array` | `len > 1`; `out == add`, multiplicands disjoint | [x] |
| 7 | `fma_array` | `len > 1`; `out == mul1 == mul2 == add` (the composition used by `driver`) | [x] |
| 8 | `fma_array` | `len > 1`; input-only alias `mul1 == mul2`, output and add disjoint | [x] |
| 9 | `fma_array` | `len > 1`; partial alias with `out == input + 1`, so writes affect later reads | [x] |
| 10 | `fma_array` | `len > 1`; partial alias with `input == out + 1`, so reads lead writes | [x] |
| 11 | `driver` | `len == 0`; empty input and empty stdout | [x] |
| 12 | `driver` | `len == 1`; one input element and one printed transformed value | [x] |
| 13 | `driver` | `len > 1`; many elements and ordered line output | [x] |
| 14 | `driver` | large valid `len`; VLA/copy/transform/print pipeline | [x] |

All numeric-value rows use randomized full-width `int` inputs in addition to
cardinality and alias coverage.
