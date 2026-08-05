# CONFIGS.md — Configuration / valid-input surface (C ground truth)

No Cargo features and no CMake build toggles exist → ONE build configuration.
The axes below are the *runtime* options/flags and input SHAPES the C code
branches on. Rows are meaningful combinations exercised via the exported .so
symbols of BOTH libraries and compared byte-for-byte.

Axes:
- **js_newstate flags**: `0` vs `JS_STRICT` (J->strict/default_strict).
- **regexp cflags**: `0`, `REG_ICASE`, `REG_NEWLINE`, and combos.
- **regexp eflags**: `0`, `REG_NOTBOL`.
- **input shapes**: empty / one / many; ASCII vs multibyte UTF-8; boundary
  runes (0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF); numbers
  (int, fractional, negatives, subnormal, huge, NaN/Inf textual).

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | jsU_runelen | every rune boundary 0,0x7F,0x80,0x7FF,0x800,0xFFFF,0x10000,0x10FFFF,>max | [x] |
| 2  | jsU_runetochar + jsU_chartorune | round-trip random runes 0..0x110000 (byte-identical encoding & decode length) | [x] |
| 3  | jsU_chartorune | random raw byte buffers (valid + invalid UTF-8 sequences) | [x] |
| 4  | jsU_isalpharune/islowerrune/isupperrune | random runes across full range | [x] |
| 5  | jsU_tolowerrune/toupperrune | random runes across full range | [x] |
| 6  | js_itoa | full int32 range incl INT_MIN, 0, negatives, random | [x] |
| 7  | js_strtod | random numeric strings: ints, decimals, exp, signs, hex, inf, whitespace | [x] |
| 8  | js_grisu2 + js_fmtexp | random finite doubles → digit buffer + K exponent | [x] |
| 9  | js_regcomp(cflags=0)+regexec(eflags=0) | literal, `.`, `*+?`, alternation, groups; matching & non-matching strings | [x] |
| 10 | js_regcomp(REG_ICASE) | case-insensitive matching random inputs | [x] |
| 11 | js_regcomp(REG_NEWLINE) | `^`/`$`/`.` with embedded newlines | [x] |
| 12 | js_regexec(REG_NOTBOL) | `^`-anchored patterns with NOTBOL set | [x] |
| 13 | js_regexec | capture group offsets (sub[].sp/ep) for multi-group patterns | [x] |
| 14 | js_regcomp REG_ICASE|REG_NEWLINE combo | combined flags | [x] |
| 15 | js_newstate(0) + js_dostring | arithmetic, string ops, arrays: `Math`, `String.prototype`, `Array` | [x] |
| 16 | js_newstate(JS_STRICT) + js_dostring | strict-mode-sensitive scripts | [x] |
| 17 | js_dostring | Number formatting: `(n).toString()`, toFixed, toPrecision, radix | [x] |
| 18 | js_dostring | JSON.stringify / JSON.parse round-trips | [x] |
| 19 | js_dostring | RegExp via JS: `str.replace`, `.match`, `.split` with flags g/i/m | [x] |
| 20 | js_dostring | Date-independent: parseInt/parseFloat/encodeURI/decodeURI | [x] |
| 21 | js_newstate/js_pushnumber/js_tostring | push/convert primitives across FFI, gettop/pop | [x] |

## Phase B status — all rows pass across randomized inputs

Verified via `cargo test`:
- Rows 1-5 → tests/utf.rs (100k-200k randomized runes/byte-buffers per row).
- Rows 6-8 → tests/dtoa.rs (200k random ints/doubles; 50k numeric strings).
- Rows 9-14 → tests/regexp.rs (pattern×subject×flag matrix + 8k-pattern fuzz).
- Rows 15-21 → tests/engine.rs (100+ JS expressions × {default, JS_STRICT};
  FFI primitive round-trips; 50k value-dependent ToInt32/ToUint32/ToInt16/
  ToUint16/ToInteger conversions).

Every comparison is C-.so-vs-Rust-.so through libloading; on any divergence the
Rust side is fixed (C is ground truth). No divergences remain.
