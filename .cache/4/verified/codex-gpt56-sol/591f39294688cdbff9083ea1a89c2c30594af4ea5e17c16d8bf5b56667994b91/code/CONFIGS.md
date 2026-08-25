# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
option or conditional source. There is one valid build-time combination:

| # | Cargo feature combination | C configuration | [ ] |
|---|---------------------------|-----------------|-----|
| F1 | no features (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

The sole public entry point is `colourblind`. Its only control-flow axis is
the three-case `cb_impairment` switch. The public signature has three mutable
float pointers without `restrict`, so all five set partitions of those
pointers are valid input shapes. Each row is tested with deterministic,
randomized raw `f32` bit patterns, including signed zero, subnormal, normal,
infinite, and NaN values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `colourblind` | `cbProtanopia`; R, G, B distinct | [x] |
| 2 | `colourblind` | `cbProtanopia`; R=G, B distinct | [x] |
| 3 | `colourblind` | `cbProtanopia`; R=B, G distinct | [x] |
| 4 | `colourblind` | `cbProtanopia`; G=B, R distinct | [x] |
| 5 | `colourblind` | `cbProtanopia`; R=G=B | [x] |
| 6 | `colourblind` | `cbDeuteranopia`; R, G, B distinct | [x] |
| 7 | `colourblind` | `cbDeuteranopia`; R=G, B distinct | [x] |
| 8 | `colourblind` | `cbDeuteranopia`; R=B, G distinct | [x] |
| 9 | `colourblind` | `cbDeuteranopia`; G=B, R distinct | [x] |
| 10 | `colourblind` | `cbDeuteranopia`; R=G=B | [x] |
| 11 | `colourblind` | `cbTritanopia`; R, G, B distinct | [x] |
| 12 | `colourblind` | `cbTritanopia`; R=G, B distinct | [x] |
| 13 | `colourblind` | `cbTritanopia`; R=B, G distinct | [x] |
| 14 | `colourblind` | `cbTritanopia`; G=B, R distinct | [x] |
| 15 | `colourblind` | `cbTritanopia`; R=G=B | [x] |
