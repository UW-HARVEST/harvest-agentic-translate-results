# Configuration-Surface Table

The public header exposes one entry point and no runtime options, modes, flags,
enums, element types, formats, or feature-conditioned APIs. The C
implementation varies the allocation/copy size using the location of the first
NUL byte.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `custom_strdup` | no options; empty string (`str[0] == 0`), allocation/copy size 1 | [x] |
| 2 | `custom_strdup` | no options; one non-NUL byte followed by NUL, allocation/copy size 2 | [x] |
| 3 | `custom_strdup` | no options; many non-NUL ASCII bytes followed by NUL | [x] |
| 4 | `custom_strdup` | no options; many arbitrary non-NUL bytes (including high-bit bytes) followed by NUL | [x] |
| 5 | `custom_strdup` | no options; NUL-terminated prefix followed by additional bytes, proving the first NUL determines the result | [x] |

The generic null-pointer case is invalid and is tracked in `ERRORS.md`.
