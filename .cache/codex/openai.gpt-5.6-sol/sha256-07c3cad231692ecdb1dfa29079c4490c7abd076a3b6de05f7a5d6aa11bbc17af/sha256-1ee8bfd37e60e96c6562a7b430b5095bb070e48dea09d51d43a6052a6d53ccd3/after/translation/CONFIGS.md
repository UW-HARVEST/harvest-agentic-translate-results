# Configuration Surface

Public entry points: `tritanopia` only. It accepts a fixed-shape, by-value
three-byte RGB struct. There are no runtime options, modes, flags, formats,
element types, lengths, compile-time features, or lower-level public entry
points.

The C input normalization branches independently for each channel at
`channel / 255.0 > 0.04045`. For byte inputs this partitions each channel into
low (`0..=10`) and high (`11..=255`), yielding the complete eight-row
cross-product below. Tests include `0`, `10`, `11`, and `255`, randomized
values in each partition, and exercise both post-transform gamma branches.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `tritanopia` | R low, G low, B low | [x] |
| 2 | `tritanopia` | R low, G low, B high | [x] |
| 3 | `tritanopia` | R low, G high, B low | [x] |
| 4 | `tritanopia` | R low, G high, B high | [x] |
| 5 | `tritanopia` | R high, G low, B low | [x] |
| 6 | `tritanopia` | R high, G low, B high | [x] |
| 7 | `tritanopia` | R high, G high, B low | [x] |
| 8 | `tritanopia` | R high, G high, B high | [x] |
