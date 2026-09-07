# Configuration surface

The public header exposes one entry point and no runtime options, modes, flags,
pointers, lengths, formats, or feature-controlled APIs. Its input is exactly
one by-value `cb_rgb_255` containing three `unsigned char` channels.

The C source independently branches for each normalized input channel at
`channel / 255 > 0.04045`. For byte inputs this mechanically partitions each
channel into the linear-gamma range `0..=10` and power-gamma range `11..=255`.
The cross-product below is the complete public-input configuration surface.
The later apply-gamma branches are value-dependent internal branches rather
than additional caller-set configurations; exhaustive testing of all
`256^3` inputs covers every reachable instance of them.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tritanopia` | no options; `R=0..=10`, `G=0..=10`, `B=0..=10` | [x] |
| 2 | `tritanopia` | no options; `R=0..=10`, `G=0..=10`, `B=11..=255` | [x] |
| 3 | `tritanopia` | no options; `R=0..=10`, `G=11..=255`, `B=0..=10` | [x] |
| 4 | `tritanopia` | no options; `R=0..=10`, `G=11..=255`, `B=11..=255` | [x] |
| 5 | `tritanopia` | no options; `R=11..=255`, `G=0..=10`, `B=0..=10` | [x] |
| 6 | `tritanopia` | no options; `R=11..=255`, `G=0..=10`, `B=11..=255` | [x] |
| 7 | `tritanopia` | no options; `R=11..=255`, `G=11..=255`, `B=0..=10` | [x] |
| 8 | `tritanopia` | no options; `R=11..=255`, `G=11..=255`, `B=11..=255` | [x] |
