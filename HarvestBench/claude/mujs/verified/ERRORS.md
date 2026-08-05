# ERRORS.md — Error / rejection surface (C ground truth)

Mechanically derived from grepping the C source for rejection paths. This
verification focuses on the **directly-callable exported C symbols** whose
error behavior can be observed across the FFI boundary without constructing an
entire `js_State` graph — primarily `js_regcomp`/`regexec` (regexp.c),
`js_strtod` (jsdtoa.c) and the UTF decode routines (utf.c) — plus the engine's
top-level error handling reachable via `js_dostring`, which converts every
`js_*error` throw into a report string + nonzero return.

Rows marked (engine) are exercised through `js_dostring`, which returns 1 and
reports the error string when the script throws.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | js_regcomp | pattern with lone `\` at end (`"\\"`) | returns NULL, errorp="unterminated escape sequence" |
| 2  | js_regcomp | invalid escape after `\` (e.g. `"\\q"`) — non-recognized | returns NULL, errorp="invalid escape sequence" (or char depending) |
| 3  | js_regcomp | bad quantifier `"a{2,1}"` / `"*"` at start | returns NULL, errorp="invalid quantifier" |
| 4  | js_regcomp | `"("` unmatched open paren | returns NULL, errorp="unmatched '('" |
| 5  | js_regcomp | `")"` unmatched close paren | returns NULL, errorp="unmatched ')'" |
| 6  | js_regcomp | `"[a-\\"` unterminated character class | returns NULL, errorp="unterminated character class" |
| 7  | js_regcomp | reversed range `"[z-a]"` | returns NULL, errorp="invalid character class range" |
| 8  | js_regcomp | back-reference to nonexistent group `"\\9"` | returns NULL, errorp="invalid back-reference" |
| 9  | js_regcomp | `> REG_MAXSUB` (16) capturing groups | returns NULL, errorp="too many captures" |
| 10 | js_regcomp | pattern length*2 > REG_MAXPROG (32768) | returns NULL, errorp="program too large" |
| 11 | js_regcomp | `"\\x"`/`"\\u"` truncated hex escape | returns NULL, errorp="unterminated escape sequence" |
| 12 | js_regcomp | huge `{n,m}` count overflowing program | returns NULL, errorp="program too large" |
| 13 | js_regexec | string that does not match pattern | returns 1 (no match), sub unchanged/cleared |
| 14 | js_regexec | pattern matches | returns 0, sub filled |
| 15 | js_strtod | non-numeric string `"abc"` | returns 0.0, endptr=start |
| 16 | js_strtod | overflow `"1e400"` | returns HUGE_VAL (inf) |
| 17 | js_strtod | underflow `"1e-400"` | returns 0.0 |
| 18 | jsU_chartorune | invalid/continuation lead byte | returns 1, *rune=Runeerror(0xFFFD) |
| 19 | jsU_runetochar | rune > Runemax(0x10FFFF) | encodes Runeerror(0xFFFD), 3 bytes |
| 20 | js_dostring (engine) | syntax error `"var ="` | returns 1, reports "SyntaxError: ..." |
| 21 | js_dostring (engine) | reference error `"nosuchvar"` | returns 1, reports "ReferenceError: ..." |
| 22 | js_dostring (engine) | type error `"null.x"` | returns 1, reports "TypeError: ..." |
| 23 | js_dostring (engine) | `throw new RangeError()` via `(1).toFixed(999)` | returns 1, reports "RangeError: ..." |
| 24 | js_dostring (engine) | `decodeURI("%")` malformed | returns 1, reports "URIError: ..." |
| 25 | js_dostring (engine) | too much recursion (deep nested parse) | returns 1, reports "SyntaxError: too much recursion" |

## Phase C status — every row covered by a passing differential test

All rows verified via `cargo test` (tests/regexp.rs, tests/dtoa.rs, tests/engine.rs):

- Rows 1-12 (regcomp rejections): `regexp::regcomp_error_parity` — asserts both
  libs return NULL with the SAME `errorp` string, or both compile.
- Rows 13-14 (regexec match/no-match): `regexp::regexp_match_valid_paths`,
  `regexp::regexp_random_fuzz` — same return code + same capture offsets.
- Rows 15-17 (strtod non-numeric/overflow/underflow): `dtoa::strtod_random_and_errors`
  — bit-identical result + identical consumed length.
- Rows 18-19 (chartorune/runetochar error runes): `utf::chartorune_random_bytes`,
  `utf::runetochar_roundtrip`.
- Rows 20-25 (engine throws: syntax/reference/type/range/URI/recursion):
  `engine::engine_error_paths_differential` — identical js_dostring return code.

Generic boundary coverage: out-of-range enum flag values across the FFI boundary
are covered by `regexp::regexp_out_of_range_flags` (cflags/eflags = INT_MIN,
INT_MAX, unused high bits). Oversized/huge inputs: "program too large" (long
pattern) and deep-recursion "too much recursion"/"call stack overflow".
