# Configuration surface

The public surface has one entry point and no runtime flags or compile-time
features. Its valid input shape is exactly three `unsigned char` channels.
The meaningful data-shape axes are the six gamma branch decisions in
`cbRemoveGammaRGB` and `cbApplyGammaRGB`.

Masks below use RGB bit order: bit 2 = R, bit 1 = G, bit 0 = B. An input bit is
set when that channel is greater than `0.04045` after normalization (equivalent
to an input byte greater than 10). An output bit is set when the transformed
linear channel is greater than
`0.00313080495356037151702786377709`. The reachable cross-product was
mechanically enumerated over all 16,777,216 byte triples using the C source's
own static helpers. `count` is the exact population and `first` is the first
lexicographic witness.

| # | entry point(s) | configuration (options set + input shape) | count | first | [ ] |
|---|----------------|--------------------------------------------|------:|-------|-----|
| 1 | `tritanopia` | no options; input gamma mask `000`, output gamma mask `000` | 1,295 | `(0,0,0)` | [x] |
| 2 | `tritanopia` | no options; input gamma mask `000`, output gamma mask `100` | 36 | `(10,3,0)` | [x] |
| 3 | `tritanopia` | no options; input gamma mask `001`, output gamma mask `000` | 2,398 | `(0,0,11)` | [x] |
| 4 | `tritanopia` | no options; input gamma mask `001`, output gamma mask `011` | 27,247 | `(0,0,44)` | [x] |
| 5 | `tritanopia` | no options; input gamma mask `010`, output gamma mask `000` | 59 | `(0,11,0)` | [x] |
| 6 | `tritanopia` | no options; input gamma mask `010`, output gamma mask `011` | 2,290 | `(0,11,6)` | [x] |
| 7 | `tritanopia` | no options; input gamma mask `010`, output gamma mask `100` | 7 | `(9,11,0)` | [x] |
| 8 | `tritanopia` | no options; input gamma mask `010`, output gamma mask `111` | 27,289 | `(0,44,0)` | [x] |
| 9 | `tritanopia` | no options; input gamma mask `011`, output gamma mask `011` | 344,182 | `(0,11,11)` | [x] |
| 10 | `tritanopia` | no options; input gamma mask `011`, output gamma mask `111` | 316,093 | `(0,47,11)` | [x] |
| 11 | `tritanopia` | no options; input gamma mask `100`, output gamma mask `000` | 15 | `(11,0,6)` | [x] |
| 12 | `tritanopia` | no options; input gamma mask `100`, output gamma mask `100` | 29,630 | `(11,0,0)` | [x] |
| 13 | `tritanopia` | no options; input gamma mask `101`, output gamma mask `000` | 610 | `(11,0,11)` | [x] |
| 14 | `tritanopia` | no options; input gamma mask `101`, output gamma mask `011` | 111,560 | `(11,0,44)` | [x] |
| 15 | `tritanopia` | no options; input gamma mask `101`, output gamma mask `100` | 52,800 | `(11,6,11)` | [x] |
| 16 | `tritanopia` | no options; input gamma mask `101`, output gamma mask `111` | 495,305 | `(11,10,13)` | [x] |
| 17 | `tritanopia` | no options; input gamma mask `110`, output gamma mask `100` | 1,470 | `(11,11,0)` | [x] |
| 18 | `tritanopia` | no options; input gamma mask `110`, output gamma mask `111` | 658,805 | `(11,11,6)` | [x] |
| 19 | `tritanopia` | no options; input gamma mask `111`, output gamma mask `011` | 1,306,364 | `(11,11,16)` | [x] |
| 20 | `tritanopia` | no options; input gamma mask `111`, output gamma mask `111` | 13,399,761 | `(11,11,11)` | [x] |

All rows pass the exhaustive, fixed-seed permutation test in
`tests/differential.rs` under both the default invocation and
`--no-default-features`.
