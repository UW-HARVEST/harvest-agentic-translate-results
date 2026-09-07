# Configuration surface

The public header exposes `driver`; `nm -D` additionally exposes the
lower-level `print_foo`. There are no runtime options, modes, feature flags,
preprocessor feature branches, variable-length inputs, or alternate formats.

For `driver`, C assignment to the 2-bit and 3-bit fields distinguishes values
that fit from values that are truncated. Those two axes are crossed with both
Boolean states below. Every row samples `z` across its complete `int32_t`
domain, including `INT_MIN`, negative values, zero, positive values, and
`INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x` fits 2 bits (`0..=3`); `y` fits 3 bits (`0..=7`); `b=false`; randomized full-domain `z` | [x] |
| 2 | `driver` | `x` fits 2 bits (`0..=3`); `y` fits 3 bits (`0..=7`); `b=true`; randomized full-domain `z` | [x] |
| 3 | `driver` | `x` fits 2 bits (`0..=3`); `y` truncates (`8..=UINT_MAX`); `b=false`; randomized full-domain `z` | [x] |
| 4 | `driver` | `x` fits 2 bits (`0..=3`); `y` truncates (`8..=UINT_MAX`); `b=true`; randomized full-domain `z` | [x] |
| 5 | `driver` | `x` truncates (`4..=UINT_MAX`); `y` fits 3 bits (`0..=7`); `b=false`; randomized full-domain `z` | [x] |
| 6 | `driver` | `x` truncates (`4..=UINT_MAX`); `y` fits 3 bits (`0..=7`); `b=true`; randomized full-domain `z` | [x] |
| 7 | `driver` | `x` truncates (`4..=UINT_MAX`); `y` truncates (`8..=UINT_MAX`); `b=false`; randomized full-domain `z` | [x] |
| 8 | `driver` | `x` truncates (`4..=UINT_MAX`); `y` truncates (`8..=UINT_MAX`); `b=true`; randomized full-domain `z` | [x] |
| 9 | `print_foo` | direct low-level call; all 2-bit `x`, 3-bit `y`, and 1-bit `b` values; randomized unused storage bits and full-domain `z` | [x] |

The crate declares no Cargo features, so the only feature combination is the
default/no-feature build.
