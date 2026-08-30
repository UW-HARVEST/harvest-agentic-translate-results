# Configuration Surface

The public dynamic entry points are `run` and `driver`. There are no runtime
options, modes, flags, enums, lengths, byte-order choices, preprocessor feature
branches, or Cargo features. `run` is the lowest-level entry point and has no
conditional branch. `driver` has one valid/error branch around base-10
`strtol`; valid conversion does not require consuming the full string.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `run` | direct low-level call; randomized C-layout house fields and negative, zero, and positive `extra_bedrooms`, constrained so both signed additions are defined | [x] |
| 2 | `run` | direct low-level call at defined integer boundaries (`floors <= INT_MAX-1`; bedroom plus extra remains in range), including `-0.0`, finite fractions, infinities, and NaNs for `bathrooms` | [x] |
| 3 | `driver` -> `run` twice | randomized decimal lexical forms whose parsed value is exactly zero | [x] |
| 4 | `driver` -> `run` twice | randomized positive decimal in the C `int` range | [x] |
| 5 | `driver` -> `run` twice | randomized negative decimal in the C `int` range | [x] |
| 6 | `driver` -> `run` twice | exact decimal boundary `INT_MIN` | [x] |
| 7 | `driver` -> `run` twice | exact decimal boundary `INT_MAX` | [x] |
| 8 | `driver` -> `run` twice | valid decimal with leading C-locale whitespace | [x] |
| 9 | `driver` -> `run` twice | valid decimal with explicit `+` sign and/or leading zeroes | [x] |
| 10 | `driver` -> `run` twice | valid decimal prefix followed by randomized nonnumeric suffix (trailing input is intentionally accepted) | [x] |

Feature combination: the manifest has no `[features]` table, so the sole code
configuration is equivalent under default and `--no-default-features`.
