# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. There is exactly one valid
feature combination:

| # | Cargo feature set | CMake configuration | compile check |
|---|-------------------|---------------------|---------------|
| 1 | empty set (`--no-default-features`) | default | [x] |

## Runtime configurations

The sole public entry point is the lowest-level API. Rows are the cross-product
that the C control flow distinguishes: whether filtering takes its keep/drop
branches, filtered length modulo four (which controls missing-character
defaults), and the independent `c3 == '='` / `c4 == '='` output suppressions.
Randomized rows sample every decode class (`A-Z`, `a-z`, `0-9`, `+`, and the
default-63 `/`/`=` class).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `decode_base64` | all bytes retained; filtered length `4n`; `c3 != '='`, `c4 != '='` | [x] |
| 2 | `decode_base64` | all bytes retained; filtered length `4n+1`; missing `c2`, `c3`, `c4` default to `A` in the final group | [x] |
| 3 | `decode_base64` | all bytes retained; filtered length `4n+2`; missing `c3`, `c4` default to `A` in the final group | [x] |
| 4 | `decode_base64` | all bytes retained; filtered length `4n+3`; missing `c4` defaults to `A` in the final group | [x] |
| 5 | `decode_base64` | all bytes retained; full group has `c3 == '='`, `c4 != '='`; second output suppressed, third emitted | [x] |
| 6 | `decode_base64` | all bytes retained; full group has `c3 != '='`, `c4 == '='`; second output emitted, third suppressed | [x] |
| 7 | `decode_base64` | all bytes retained; full group has `c3 == '='`, `c4 == '='`; second and third outputs suppressed | [x] |
| 8 | `decode_base64` | ignored bytes interspersed; filtered length `4n`; `c3 != '='`, `c4 != '='` | [x] |
| 9 | `decode_base64` | ignored bytes interspersed; filtered length `4n+1`; missing `c2`, `c3`, `c4` default to `A` | [x] |
| 10 | `decode_base64` | ignored bytes interspersed; filtered length `4n+2`; missing `c3`, `c4` default to `A` | [x] |
| 11 | `decode_base64` | ignored bytes interspersed; filtered length `4n+3`; missing `c4` defaults to `A` | [x] |
| 12 | `decode_base64` | ignored bytes interspersed; `c3 == '='`, `c4 != '='` | [x] |
| 13 | `decode_base64` | ignored bytes interspersed; `c3 != '='`, `c4 == '='` | [x] |
| 14 | `decode_base64` | ignored bytes interspersed; `c3 == '='`, `c4 == '='` | [x] |
| 15 | `decode_base64` | nonempty source contains no base64 bytes; filtered length zero; returns allocated empty string | [x] |
| 16 | `decode_base64` | embedded NUL terminates processing; bytes after the first NUL are ignored | [x] |
| 17 | `decode_base64` | long multi-group input, exercising one/many loop boundaries and allocation sizing | [x] |

There are no runtime options, modes, flags, element types, byte-order settings,
or alternate public wrappers in the header or implementation.
