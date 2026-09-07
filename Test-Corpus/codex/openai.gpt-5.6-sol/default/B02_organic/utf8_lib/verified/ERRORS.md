# Error surface

The malformed-sequence rows below are derived from the individual constraints
in `valid_1` through `valid_4` and the final rejection branch in
`w_utf8_drop`. For each such row, `w_utf8_filter` consumes the same rejected
byte: it removes it when `replacement == false` and emits one UTF-8 U+FFFD
sequence when `replacement == true`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `w_utf8_drop` | `string == NULL` | assertion failure (`SIGABRT`) | [x] |
| 2 | `w_utf8_filter` | `string == NULL` | assertion failure (`SIGABRT`) | [x] |
| 3 | `w_utf8_drop`, `w_utf8_filter` | first rejected byte is a continuation byte `0x80..0xBF` | drop returns its offset; filter removes/replaces that byte | [x] |
| 4 | `w_utf8_drop`, `w_utf8_filter` | two-byte lead is `0xC0` or `0xC1` (overlong encoding) | drop returns its offset; filter removes/replaces that byte | [x] |
| 5 | `w_utf8_drop`, `w_utf8_filter` | lead `0xC2..0xDF` is followed by a non-continuation byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 6 | `w_utf8_drop`, `w_utf8_filter` | lead `0xE0..0xEF` has a non-continuation second byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 7 | `w_utf8_drop`, `w_utf8_filter` | three-byte sequence has a valid second byte but a non-continuation third byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 8 | `w_utf8_drop`, `w_utf8_filter` | `0xE0` has second byte below `0xA0` (overlong encoding) | drop returns its offset; filter removes/replaces `0xE0` | [x] |
| 9 | `w_utf8_drop`, `w_utf8_filter` | `0xED` has second byte `0xA0..0xBF` (UTF-16 surrogate range) | drop returns its offset; filter removes/replaces `0xED` | [x] |
| 10 | `w_utf8_drop`, `w_utf8_filter` | lead `0xF0..0xF4` has a non-continuation second byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 11 | `w_utf8_drop`, `w_utf8_filter` | four-byte sequence has valid second but non-continuation third byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 12 | `w_utf8_drop`, `w_utf8_filter` | four-byte sequence has valid second and third but non-continuation fourth byte, including NUL truncation | drop returns its offset; filter removes/replaces the lead byte | [x] |
| 13 | `w_utf8_drop`, `w_utf8_filter` | `0xF0` has second byte below `0x90` (overlong encoding) | drop returns its offset; filter removes/replaces `0xF0` | [x] |
| 14 | `w_utf8_drop`, `w_utf8_filter` | `0xF4` has second byte above `0x8F` (above U+10FFFF) | drop returns its offset; filter removes/replaces `0xF4` | [x] |
| 15 | `w_utf8_drop`, `w_utf8_filter` | lead byte is `0xF5..0xF7` (explicit maximum lead is `0xF4`) | drop returns its offset; filter removes/replaces that byte | [x] |
| 16 | `w_utf8_drop`, `w_utf8_filter` | lead byte is `0xF8..0xFF` (does not match any UTF-8 width) | drop returns its offset; filter removes/replaces that byte | [x] |
| 17 | `w_utf8_filter` | valid-string fast path and `strdup` cannot allocate | returns `NULL` | [x] |
| 18 | `w_utf8_filter` | malformed-string path and initial `malloc(strlen(string) + 1)` cannot allocate | returns `NULL` | [x] |
| 19 | `w_utf8_filter` | `replacement == true` and `realloc` for a 4096-byte replacement increment cannot allocate | returns `NULL` | [x] |

The `0xEF` second-byte `<= 0xBF` expression is mechanically present in C but
cannot reject independently: the preceding continuation-byte predicate already
restricts that byte to `0x80..0xBF`. Its reachable boundary is covered in the
configuration table.
