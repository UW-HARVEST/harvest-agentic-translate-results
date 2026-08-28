# Configuration Surface

The public header exposes one entry point and no runtime options, modes, flags,
enums, element types, explicit lengths, or compile-time feature branches. The
rows below enumerate the valid input shapes distinguished by the C string
length calculation and NUL-inclusive copy.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `custom_strdup` | no options; non-null empty C string (`strlen == 0`), with randomized bytes after its first NUL | [x] |
| 2 | `custom_strdup` | no options; one non-NUL byte followed by NUL (`strlen == 1`) | [x] |
| 3 | `custom_strdup` | no options; two or more non-NUL bytes followed by NUL (`strlen >= 2`), including long strings | [x] |
| 4 | `custom_strdup` | no options; nonempty C string followed by NUL and additional randomized storage bytes, which must not be copied | [x] |

The allocation-failure branch is an error configuration and is tracked in
`ERRORS.md`, not duplicated here.
