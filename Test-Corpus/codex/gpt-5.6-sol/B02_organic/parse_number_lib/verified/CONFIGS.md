# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, compile definitions, conditional sources, or backend selection.
Consequently there is exactly one valid feature combination.

| # | Cargo invocation feature set | CMake configuration | compile check |
|---|---|---|---|
| F01 | empty set (`--no-default-features --features ""`) | default | [x] |

## Runtime and Input Configurations

The sole public entry point is the low-level function `parse_number`. Its
public state consists of `cJSON`, `parse_buffer`, and the byte slice selected
by `content`, `offset`, and `length`. There are no runtime modes or flags.
Rows below are the source-derived cross-product pruned to behavior that the C
scanner, libc conversion, offset update, or saturation logic distinguishes.
Each randomized row includes values across its stated shape.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|---|---|---|
| C01 | `parse_number` | no options; one-byte unsigned integer, offset zero, token ends at `length`, interior `int` result | [x] |
| C02 | `parse_number` | no options; multi-byte unsigned integer, offset zero, token ends at `length`, interior `int` result | [x] |
| C03 | `parse_number` | no options; explicit leading `+` or `-`, token ends at `length`, interior `int` result | [x] |
| C04 | `parse_number` | no options; decimal point with digits on both sides, exercising `has_decimal_point` and replacement loop | [x] |
| C05 | `parse_number` | no options; decimal point with digits on only one side (`.n` or `n.`) | [x] |
| C06 | `parse_number` | no options; lowercase exponent marker with signed or unsigned exponent | [x] |
| C07 | `parse_number` | no options; uppercase exponent marker with signed or unsigned exponent | [x] |
| C08 | `parse_number` | no options; immediate non-number delimiter after a valid prefix, exercising scanner `default` | [x] |
| C09 | `parse_number` | no options; `length` truncates a backing byte array without a NUL terminator | [x] |
| C10 | `parse_number` | no options; nonzero `offset` selects a valid numeric token inside prefix/suffix bytes | [x] |
| C11 | `parse_number` | no options; scanner accepts a longer malformed sequence but `strtod` consumes only a valid prefix (for example `1e+` or `1-2`) | [x] |
| C12 | `parse_number` | no options; zero and multi-byte leading-zero integer forms | [x] |
| C13 | `parse_number` | no options; finite value `>= INT_MAX`, including exact and one-past boundary, saturating `valueint` to `INT_MAX` | [x] |
| C14 | `parse_number` | no options; finite value `<= INT_MIN`, including exact and one-past boundary, saturating `valueint` to `INT_MIN` | [x] |
| C15 | `parse_number` | no options; extreme positive or negative exponent converted to infinity and saturated | [x] |
| C16 | `parse_number` | no options; extreme negative exponent converted to subnormal or underflowed zero | [x] |
| C17 | `parse_number` | no options; finite interior values adjacent to both `int` saturation boundaries | [x] |
| C18 | `parse_number` | no options; long numeric token (64 through 512 scanned bytes), token ends at `length` | [x] |
