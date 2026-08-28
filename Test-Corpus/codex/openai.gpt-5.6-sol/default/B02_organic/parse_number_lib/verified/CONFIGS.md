# Configuration Surface

The public header declares one entry point, `parse_number`. There are no
runtime options, modes, flags, compile-time feature branches, or Cargo
features. The rows below enumerate the valid-input combinations distinguished
by the scan switch, decimal replacement branch, `strtod` consumed-prefix
behavior, integer saturation branches, and buffer offset/length boundaries.
Each row is exercised with repeated fixed-seed generated inputs.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `parse_number` | unsigned decimal integer; starts at offset 0; token ends exactly at `length`; value strictly inside `INT_MIN..INT_MAX` | [x] |
| 2 | `parse_number` | leading `+` decimal integer; token terminated by a non-token byte before `length`; in-range value | [x] |
| 3 | `parse_number` | leading `-` decimal integer; token starts at a nonzero offset and ends exactly at `length`; in-range value | [x] |
| 4 | `parse_number` | leading `-` decimal integer; nonzero offset; token terminated by a non-token byte; in-range value | [x] |
| 5 | `parse_number` | decimal point with digits on both sides; exact-length token; in-range value | [x] |
| 6 | `parse_number` | decimal point with no leading digit; non-token terminator; in-range value | [x] |
| 7 | `parse_number` | decimal point with no trailing digit; exact-length token; in-range value | [x] |
| 8 | `parse_number` | multiple scanned decimal points; `strtod` succeeds on the valid numeric prefix and leaves the remainder unconsumed | [x] |
| 9 | `parse_number` | lowercase `e` exponent with unsigned exponent; exact-length token; in-range finite value | [x] |
| 10 | `parse_number` | uppercase `E` exponent with `+` exponent sign; non-token terminator; in-range finite value | [x] |
| 11 | `parse_number` | exponent with `-` exponent sign; nonzero offset; in-range finite value | [x] |
| 12 | `parse_number` | incomplete exponent after a valid mantissa; scanner includes it but `strtod` consumes only the mantissa | [x] |
| 13 | `parse_number` | extra scanned `+` or `-` after a valid numeric prefix; `strtod` consumes only the valid prefix | [x] |
| 14 | `parse_number` | parsed number strictly between `INT_MIN` and `INT_MAX`; `valueint` uses truncating `(int)number` branch | [x] |
| 15 | `parse_number` | parsed number equal to or above `INT_MAX`; `valueint` saturates to `INT_MAX` | [x] |
| 16 | `parse_number` | parsed number equal to or below `INT_MIN`; `valueint` saturates to `INT_MIN` | [x] |
| 17 | `parse_number` | positive exponent overflow parsed as positive infinity; `valueint` saturates to `INT_MAX` | [x] |
| 18 | `parse_number` | negative exponent overflow parsed as negative infinity; `valueint` saturates to `INT_MIN` | [x] |
| 19 | `parse_number` | exponent underflow parsed as zero/subnormal; `valueint` is zero | [x] |
| 20 | `parse_number` | valid numeric prefix followed by a byte outside `0-9+-.eE`; scanner stops and leaves that byte unconsumed | [x] |
| 21 | `parse_number` | nonzero initial `item` fields and nonzero `depth`; successful parse overwrites all item fields while leaving `depth` unchanged | [x] |
