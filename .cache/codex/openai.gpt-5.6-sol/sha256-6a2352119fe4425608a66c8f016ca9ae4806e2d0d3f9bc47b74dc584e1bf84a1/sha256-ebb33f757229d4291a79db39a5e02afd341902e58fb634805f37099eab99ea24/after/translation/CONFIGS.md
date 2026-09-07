# Configuration surface

The public headers expose one entry point and one runtime option:
`colourblind(cb_impairment, float *, float *, float *)`, where the switch has
three cases. The implementation reads all three pointed-to values before
writing outputs in `R`, `G`, `B` order, making the five pointer-equivalence
layouts below observably distinct. Every row is exercised with deterministic
randomized values spanning all `f32` bit patterns, including signed zero,
subnormals, infinities, and NaNs.

There are no Cargo features and no C preprocessor configuration branches, so
there is one effective build configuration.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `colourblind` | Protanopia (`0`); `R`, `G`, `B` distinct | [x] |
| 2 | `colourblind` | Protanopia (`0`); `R == G`, `B` distinct | [x] |
| 3 | `colourblind` | Protanopia (`0`); `R == B`, `G` distinct | [x] |
| 4 | `colourblind` | Protanopia (`0`); `G == B`, `R` distinct | [x] |
| 5 | `colourblind` | Protanopia (`0`); `R == G == B` | [x] |
| 6 | `colourblind` | Deuteranopia (`1`); `R`, `G`, `B` distinct | [x] |
| 7 | `colourblind` | Deuteranopia (`1`); `R == G`, `B` distinct | [x] |
| 8 | `colourblind` | Deuteranopia (`1`); `R == B`, `G` distinct | [x] |
| 9 | `colourblind` | Deuteranopia (`1`); `G == B`, `R` distinct | [x] |
| 10 | `colourblind` | Deuteranopia (`1`); `R == G == B` | [x] |
| 11 | `colourblind` | Tritanopia (`2`); `R`, `G`, `B` distinct | [x] |
| 12 | `colourblind` | Tritanopia (`2`); `R == G`, `B` distinct | [x] |
| 13 | `colourblind` | Tritanopia (`2`); `R == B`, `G` distinct | [x] |
| 14 | `colourblind` | Tritanopia (`2`); `G == B`, `R` distinct | [x] |
| 15 | `colourblind` | Tritanopia (`2`); `R == G == B` | [x] |
