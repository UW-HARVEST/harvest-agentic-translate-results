# Configuration Surface

The public API has one entry point and no runtime options or Cargo features.
Rows below are the pruned cross-product of branches in the token scanner,
`strtod` consumption behavior, buffer shape, and the three integer-result
branches in `../c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `parse_number` | offset 0; digit-only integer token; scan ends at `length`; finite value strictly inside `INT_MIN..INT_MAX` | [x] |
| 2 | `parse_number` | offset 0; leading `+` or `-`; scan ends at `length`; finite value strictly inside the integer range | [x] |
| 3 | `parse_number` | token contains `.` so the decimal-point replacement loop runs; full `strtod` consumption; finite in-range result | [x] |
| 4 | `parse_number` | token contains `e` or `E`, with optional exponent sign; full `strtod` consumption; finite in-range result | [x] |
| 5 | `parse_number` | scanner accepts the entire token alphabet, but `strtod` consumes only a numeric prefix (for example an incomplete exponent or a second sign/dot); successful partial parse advances by only the consumed prefix | [x] |
| 6 | `parse_number` | a non-number byte terminates scanning before `length`; `strtod` fully consumes the scanned numeric prefix and leaves the terminator unconsumed | [x] |
| 7 | `parse_number` | declared `length` truncates otherwise numeric backing bytes; no terminating NUL is required; parse consumes exactly the declared numeric range | [x] |
| 8 | `parse_number` | nonzero valid `offset`; bytes before the offset are ignored; successful parse advances from the original offset | [x] |
| 9 | `parse_number` | arbitrary `depth` and preinitialized `item` fields; successful parse ignores `depth`, overwrites all three item fields, and changes only `offset` in the buffer | [x] |
| 10 | `parse_number` | finite fractional values in the open integer range, including positive and negative values; `valueint` truncates toward zero | [x] |
| 11 | `parse_number` | parsed finite value is exactly or above `INT_MAX`; `valueint` saturates to `INT_MAX` | [x] |
| 12 | `parse_number` | parsed finite value is exactly or below `INT_MIN`; `valueint` saturates to `INT_MIN` | [x] |
| 13 | `parse_number` | decimal exponent overflows `double` to positive or negative infinity; `valuedouble` preserves infinity and `valueint` saturates by sign | [x] |
| 14 | `parse_number` | exponent produces subnormal, underflowed zero, or signed zero; `valuedouble` bit pattern and `valueint == 0` match C | [x] |
