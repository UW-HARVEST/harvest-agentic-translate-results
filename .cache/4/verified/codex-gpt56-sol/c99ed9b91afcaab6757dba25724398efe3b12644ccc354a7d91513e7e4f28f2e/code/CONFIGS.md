# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional source selection. There is exactly one valid
configuration:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `cargo check/test --no-default-features --features ''` | default | [x] |

## Runtime Matrix

The public API consists only of `tritanopia(cb_rgb_255)`. Input shape is fixed:
one three-byte RGB value, with each channel covering all values `0..=255`.
There are no runtime modes, flags, formats, counts, or byte-order options.

The C implementation has six data-dependent branches. Mask bits `0x01`,
`0x02`, and `0x04` mean the remove-gamma high branch for R, G, and B
respectively (`input / 255 > 0.04045`, exactly input `>= 11`). Bits `0x08`,
`0x10`, and `0x20` mean the apply-gamma high branch for transformed R, G, and
B respectively (`value > 0.00313080495356037151702786377709`).

The rows below are the reachable cross-product, mechanically enumerated over
all 16,777,216 inputs using the C operations. The count column independently
accounts for the complete domain.

| # | entry point(s) | configuration (branch mask; witness RGB; domain count) | [ ] |
|---|----------------|--------------------------------------------------------|-----|
| 1 | `tritanopia` | `0x00`; `(0,0,0)`; 1,295 | [x] |
| 2 | `tritanopia` | `0x01`; `(11,0,6)`; 15 | [x] |
| 3 | `tritanopia` | `0x02`; `(0,11,0)`; 59 | [x] |
| 4 | `tritanopia` | `0x04`; `(0,0,11)`; 2,398 | [x] |
| 5 | `tritanopia` | `0x05`; `(11,0,11)`; 610 | [x] |
| 6 | `tritanopia` | `0x08`; `(10,3,0)`; 36 | [x] |
| 7 | `tritanopia` | `0x09`; `(11,0,0)`; 29,630 | [x] |
| 8 | `tritanopia` | `0x0a`; `(9,11,0)`; 7 | [x] |
| 9 | `tritanopia` | `0x0b`; `(11,11,0)`; 1,470 | [x] |
| 10 | `tritanopia` | `0x0d`; `(11,6,11)`; 52,800 | [x] |
| 11 | `tritanopia` | `0x32`; `(0,11,6)`; 2,290 | [x] |
| 12 | `tritanopia` | `0x34`; `(0,0,44)`; 27,247 | [x] |
| 13 | `tritanopia` | `0x35`; `(11,0,44)`; 111,560 | [x] |
| 14 | `tritanopia` | `0x36`; `(0,11,11)`; 344,182 | [x] |
| 15 | `tritanopia` | `0x37`; `(11,11,16)`; 1,306,364 | [x] |
| 16 | `tritanopia` | `0x3a`; `(0,44,0)`; 27,289 | [x] |
| 17 | `tritanopia` | `0x3b`; `(11,11,6)`; 658,805 | [x] |
| 18 | `tritanopia` | `0x3d`; `(11,10,13)`; 495,305 | [x] |
| 19 | `tritanopia` | `0x3e`; `(0,47,11)`; 316,093 | [x] |
| 20 | `tritanopia` | `0x3f`; `(11,11,11)`; 13,399,761 | [x] |

Domain count check: `1,295 + 15 + 59 + 2,398 + 610 + 36 + 29,630 + 7 +
1,470 + 52,800 + 2,290 + 27,247 + 111,560 + 344,182 + 1,306,364 + 27,289 +
658,805 + 495,305 + 316,093 + 13,399,761 = 16,777,216`.
