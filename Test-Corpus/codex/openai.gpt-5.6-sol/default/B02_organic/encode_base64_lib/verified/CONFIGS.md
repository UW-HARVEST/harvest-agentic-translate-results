# Configuration Surface

The sole public entry point is `encode_base64`. It has no compile-time
features, enums, byte-order settings, or mutable state. The rows below cover
the runtime sizing mode and every data shape/value class distinguished by the
C branches. Alphabet-class rows overlap the sizing rows intentionally: they
make every branch of the private `encode` helper observable through the public
FFI entry point.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `encode_base64` | `size == 0`; `src` is an empty NUL-terminated string, so `strlen(src) == 0` and the loop is skipped | [x] |
| 2 | `encode_base64` | `size == 0`; nonempty NUL-terminated input with derived length `1 mod 3` | [x] |
| 3 | `encode_base64` | `size == 0`; nonempty NUL-terminated input with derived length `2 mod 3` | [x] |
| 4 | `encode_base64` | `size == 0`; nonempty NUL-terminated input with derived length `0 mod 3` | [x] |
| 5 | `encode_base64` | explicit positive `size`, `size mod 3 == 1`; binary input may contain embedded NUL and high-bit bytes | [x] |
| 6 | `encode_base64` | explicit positive `size`, `size mod 3 == 2`; binary input may contain embedded NUL and high-bit bytes | [x] |
| 7 | `encode_base64` | explicit positive `size`, `size mod 3 == 0`; binary input may contain embedded NUL and high-bit bytes | [x] |
| 8 | `encode_base64` | explicit `size` in `-3..=-1`; C does not reject it, allocation succeeds, and the loop is skipped | [x] |
| 9 | `encode_base64` | emitted six-bit values in `0..=25`, selecting `A` through `Z` | [x] |
| 10 | `encode_base64` | emitted six-bit values in `26..=51`, selecting `a` through `z` | [x] |
| 11 | `encode_base64` | emitted six-bit values in `52..=61`, selecting `0` through `9` | [x] |
| 12 | `encode_base64` | emitted six-bit value `62`, selecting `+` | [x] |
| 13 | `encode_base64` | emitted six-bit value `63`, selecting `/` | [x] |

All rows pass with default features and `--no-default-features`.
