# Configuration Surface

Mechanically derived from the public header and the branches in
`../c_src/src/lib.c`. The library has one public entry point, no compile-time
Cargo features, and no binary target.

The runtime axes are:

- character class: decimal digit, lowercase `a`-`f`, uppercase `A`-`F`, or
  invalid byte;
- parse shape: empty, one byte, many bytes, full consumption, or a decoded
  prefix ending at an invalid byte;
- nibble state: byte boundary (`state == 0`) or half-byte (`state != 0`);
- output capacity: zero for empty input, exactly sufficient, or spare;
- `ignore`: null, non-null without a match, or matching invalid bytes at a byte
  boundary; valid hex digits are decoded before `ignore` is consulted;
- `hex_end_p`: null or non-null;
- length-delimited bytes: embedded NUL/high-bit bytes and bytes beyond
  `hex_len`.

Error-producing combinations are enumerated in `ERRORS.md`; the valid
cross-product below is pruned to combinations the C branches distinguish.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hex2bin` | Empty input; zero output capacity; `ignore == NULL`; `hex_end_p == NULL`. | [x] |
| 2 | `hex2bin` | Empty input; spare output capacity; non-null `ignore`; non-null `hex_end_p`. | [x] |
| 3 | `hex2bin` | One decoded byte from decimal digits; exactly sufficient capacity; null `ignore`; null end pointer. | [x] |
| 4 | `hex2bin` | One decoded byte using lowercase alpha digits; exactly sufficient capacity; non-null end pointer. | [x] |
| 5 | `hex2bin` | One decoded byte using uppercase alpha digits; spare capacity; non-null end pointer. | [x] |
| 6 | `hex2bin` | One decoded byte mixing numeric and alpha digit classes/case. | [x] |
| 7 | `hex2bin` | Many decoded bytes; all digit classes randomized; exactly sufficient capacity; null end pointer. | [x] |
| 8 | `hex2bin` | Many decoded bytes; all digit classes randomized; spare capacity; non-null end pointer. | [x] |
| 9 | `hex2bin` | Non-null `ignore` whose characters do not occur in fully consumed input. | [x] |
| 10 | `hex2bin` | A valid hex digit also appears in `ignore`; it is decoded because classification precedes `strchr`. | [x] |
| 11 | `hex2bin` | Matching single-byte separators are ignored at byte boundaries between decoded bytes. | [x] |
| 12 | `hex2bin` | Multiple distinct ignored bytes, including repeated leading, internal, and trailing separators at byte boundaries. | [x] |
| 13 | `hex2bin` | Embedded NUL at a byte boundary with non-null `ignore`; `strchr(ignore, '\0')` matches the terminator, so NUL is ignored. | [x] |
| 14 | `hex2bin` | Invalid first byte with non-null end pointer; returns a successful empty prefix and end offset zero. | [x] |
| 15 | `hex2bin` | Invalid ASCII byte after one or many complete bytes with non-null end pointer; returns the decoded prefix. | [x] |
| 16 | `hex2bin` | Invalid high-bit byte after complete bytes with non-null end pointer; returns the decoded prefix. | [x] |
| 17 | `hex2bin` | Backing buffer contains additional invalid bytes beyond `hex_len`; only the selected even-length prefix is consumed. | [x] |
| 18 | `hex2bin` | Null output pointer on a valid path that writes zero bytes (empty input). | [x] |
| 19 | `hex2bin` | Null hex pointer with `hex_len == 0` and null end pointer; no input dereference occurs. | [x] |
