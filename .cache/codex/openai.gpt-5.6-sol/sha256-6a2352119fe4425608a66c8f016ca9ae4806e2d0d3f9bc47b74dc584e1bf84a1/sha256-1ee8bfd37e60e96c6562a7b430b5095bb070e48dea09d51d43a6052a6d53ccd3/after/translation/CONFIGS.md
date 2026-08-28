# Configuration Surface

The only public entry point is `colourblind`. Its switch distinguishes the
three `cb_impairment` values. Each selected helper snapshots the three pointed
to floats and then writes `R`, `G`, and `B` in order, making the five pointer
alias partitions observable input shapes. There are no runtime flags, lengths,
formats, element types, byte-order options, conditional-compilation branches,
or lower-level public entry points.

Every row is exercised with reproducible randomized `f32` bit patterns,
including ordinary finite values, signed zeros, subnormals, infinities, and
NaNs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `colourblind` | `cbProtanopia`; `R`, `G`, and `B` are distinct pointers | [x] |
| 2 | `colourblind` | `cbProtanopia`; `R == G`, with distinct `B` | [x] |
| 3 | `colourblind` | `cbProtanopia`; `R == B`, with distinct `G` | [x] |
| 4 | `colourblind` | `cbProtanopia`; `G == B`, with distinct `R` | [x] |
| 5 | `colourblind` | `cbProtanopia`; `R == G == B` | [x] |
| 6 | `colourblind` | `cbDeuteranopia`; `R`, `G`, and `B` are distinct pointers | [x] |
| 7 | `colourblind` | `cbDeuteranopia`; `R == G`, with distinct `B` | [x] |
| 8 | `colourblind` | `cbDeuteranopia`; `R == B`, with distinct `G` | [x] |
| 9 | `colourblind` | `cbDeuteranopia`; `G == B`, with distinct `R` | [x] |
| 10 | `colourblind` | `cbDeuteranopia`; `R == G == B` | [x] |
| 11 | `colourblind` | `cbTritanopia`; `R`, `G`, and `B` are distinct pointers | [x] |
| 12 | `colourblind` | `cbTritanopia`; `R == G`, with distinct `B` | [x] |
| 13 | `colourblind` | `cbTritanopia`; `R == B`, with distinct `G` | [x] |
| 14 | `colourblind` | `cbTritanopia`; `G == B`, with distinct `R` | [x] |
| 15 | `colourblind` | `cbTritanopia`; `R == G == B` | [x] |
