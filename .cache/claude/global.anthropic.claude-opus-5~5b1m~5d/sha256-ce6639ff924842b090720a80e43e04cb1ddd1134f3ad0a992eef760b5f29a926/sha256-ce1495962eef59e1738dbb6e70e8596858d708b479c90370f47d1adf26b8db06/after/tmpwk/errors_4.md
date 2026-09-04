| # | function | trigger (exact invalid input/condition) | expected C result |
|---|---|---|---|
| 1 | `hex` (regexp.c:101) | non-hex digit in `\x`/`\u` escape: `js_regcomp("a\\xZZ", 0, &err)` (JS: `new RegExp('a\\xZZ')`) | `die()` longjmp; `regcompx` returns NULL, `*errorp = "invalid escape sequence"`; JS: SyntaxError `regular expression: invalid escape sequence` |
| 2 | `dec` (regexp.c:108) | non-digit inside `{...}` count: `js_regcomp("a{b}", 0, &err)`; also `"a{,2}"` | NULL, `*errorp = "invalid quantifier"`; JS: SyntaxError `regular expression: invalid quantifier` |
| 3 | `nextrune` (regexp.c:128) | pattern ends with a lone backslash: `js_regcomp("a\\", 0, &err)` (pattern text `a\`) | NULL, `*errorp = "unterminated escape sequence"` |
| 4 | `nextrune` case `'c'` (regexp.c:138) | `\c` with nothing after it: `js_regcomp("a\\c", 0, &err)` (pattern text `a\c`) | NULL, `*errorp = "unterminated escape sequence"` |
| 5 | `nextrune` case `'x'` (regexp.c:143) | fewer than 2 bytes after `\x`: `js_regcomp("a\\x4", 0, &err)` (pattern text `a\x4`) | NULL, `*errorp = "unterminated escape sequence"` |
| 6 | `nextrune` case `'u'` (regexp.c:153) | fewer than 4 bytes after `\u`: `js_regcomp("a\\u12", 0, &err)` (pattern text `a\u12`) | NULL, `*errorp = "unterminated escape sequence"` |
| 7 | `nextrune` (regexp.c:170) | identity escape of a unicode letter or `_` (not in `ESCAPES`): `js_regcomp("a\\y", 0, &err)` or `js_regcomp("a\\_", 0, &err)` | NULL, `*errorp = "invalid escape character"` |
| 8 | `lexcount` (regexp.c:186) | min repeat count reaches REPINF(255): `js_regcomp("a{255}", 0, &err)` | NULL, `*errorp = "numeric overflow"` |
| 9 | `lexcount` (regexp.c:200) | max repeat count reaches REPINF(255): `js_regcomp("a{1,255}", 0, &err)` | NULL, `*errorp = "numeric overflow"` |
| 10 | `newcclass` (regexp.c:213) | more than REG_MAXCLASS(128) character classes: pattern `"[a]"` repeated 129 times | NULL, `*errorp = "too many character classes"` |
| 11 | `addrange` (regexp.c:224) | reversed class range `a > b`: `js_regcomp("[z-a]", 0, &err)` | NULL, `*errorp = "invalid character class range"` |
| 12 | `addrange` (regexp.c:253) | more than 31 non-mergeable spans in one class (`cc->end+2 >= spans+REG_MAXSPAN(64)`): 40 singleton escapes spaced 4 apart so that none merge, i.e. `js_regcomp("[" + "\u2000\u2004\u2008...\u209C" + "]", 0, &err)` | NULL, `*errorp = "too many character class ranges"` |
| 13 | `lexclass` (regexp.c:322) | unterminated `[`: `js_regcomp("[a", 0, &err)` or `js_regcomp("[a-", 0, &err)` | NULL, `*errorp = "unterminated character class"` |
| 14 | `newrep` (regexp.c:493) | unbounded repeat (`max == REPINF`) of an atom that can match empty: `js_regcomp("()*", 0, &err)`, `"()+"`, `"(?:){2,}"`, `"(?:a*)*"` | NULL, `*errorp = "infinite loop matching the empty string"` |
| 15 | `parseatom` (regexp.c:541) | back-reference to group 0 / to a group that does not exist / not yet closed: `js_regcomp("\\1", 0, &err)`, `js_regcomp("(a)\\2", 0, &err)` | NULL, `*errorp = "invalid back-reference"` |
| 16 | `parseatom` (regexp.c:552) | `g->nsub == REG_MAXSUB(16)` — the 16th capture group: pattern `"()"` repeated 16 times | NULL, `*errorp = "too many captures"` |
| 17 | `parseatom` (regexp.c:557) | capturing group never closed: `js_regcomp("(a", 0, &err)` | NULL, `*errorp = "unmatched '('"` |
| 18 | `parseatom` (regexp.c:563) | non-capturing group never closed: `js_regcomp("(?:a", 0, &err)` | NULL, `*errorp = "unmatched '('"` |
| 19 | `parseatom` (regexp.c:570) | positive lookahead never closed: `js_regcomp("(?=a", 0, &err)` | NULL, `*errorp = "unmatched '('"` |
| 20 | `parseatom` (regexp.c:577) | negative lookahead never closed: `js_regcomp("(?!a", 0, &err)` | NULL, `*errorp = "unmatched '('"` |
| 21 | `parseatom` (regexp.c:580) | token in atom position that is not a valid atom (bare quantifier / EOF after `\|`): `js_regcomp("*a", 0, &err)`, `js_regcomp("+", 0, &err)`, `js_regcomp("?", 0, &err)`, `js_regcomp("{2}", 0, &err)` | NULL, `*errorp = "syntax error"` |
| 22 | `parserep` (regexp.c:598) | `{M,N}` with `N < M`: `js_regcomp("a{2,1}", 0, &err)` | NULL, `*errorp = "invalid quantifier"` |
| 23 | `count` (regexp.c:661) | parse-tree recursion depth > REG_MAXREC(4096) — right-leaning P_CAT chain: pattern of 4100 `a` characters | NULL, `*errorp = "stack overflow"` |
| 24 | `count` (regexp.c:672) | instruction count for one P_REP node `< 0` or `> REG_MAXPROG(32768)`: `js_regcomp("(?:a{254}){254}", 0, &err)` | NULL, `*errorp = "program too large"` |
| 25 | `regcompx` (regexp.c:916) | `alloc(ctx, NULL, sizeof(Reprog))` returns NULL (allocator/OOM) | NULL, `*errorp = "cannot allocate regular expression"` |
| 26 | `regcompx` (regexp.c:922) | `strlen(pattern) * 2 > REG_MAXPROG(32768)`, i.e. pattern longer than 16384 bytes: pattern of 16385 `a` characters | NULL, `*errorp = "program too large"` |
| 27 | `regcompx` (regexp.c:926) | `alloc(ctx, NULL, sizeof(Renode) * n)` returns NULL for the parse-node list | NULL, `*errorp = "cannot allocate regular expression parse list"` |
| 28 | `regcompx` (regexp.c:940) | leftover `)` after parse: `js_regcomp("a)", 0, &err)`, `js_regcomp("(a))", 0, &err)` | NULL, `*errorp = "unmatched ')'"` |
| 29 | `regcompx` (regexp.c:942) | `g.lookahead != EOF` after `parsealt` — defensive; `parsecat`/`parsealt` can only stop on EOF, `\|` or `)`, so unreachable via any pattern | NULL, `*errorp = "syntax error"` |
| 30 | `regcompx` (regexp.c:951) | total program size `6 + count()` `> REG_MAXPROG(32768)` (sum over P_CAT children, no per-node overflow): pattern `"a{99}"` repeated 400 times | NULL, `*errorp = "program too large"` |
| 31 | `regcompx` (regexp.c:956) | `alloc(ctx, NULL, n * sizeof(Reinst))` returns NULL | NULL, `*errorp = "cannot allocate regular expression instruction list"` |
| 32 | `regcompx` (regexp.c:961) | `alloc(ctx, NULL, ncclass * sizeof(Reclass))` returns NULL (pattern contains at least one class, e.g. `"[a]"`) | NULL, `*errorp = "cannot allocate regular expression character class list"` |
| 33 | `match` (regexp.c:1075) | backtracking recursion `depth > REG_MAXREC(4096)`: `js_regexec(js_regcomp("a*",0,&e), <4999 'a' chars>, &m, 0)` | `match` returns -1, `js_regexec` returns -1; JS: `js_error(J, "regexec failed")` -> Error `regexec failed` |
| 34 | `match` I_ANYNL (regexp.c:1116) | unanchored search scan runs off the end of the subject: `js_regexec(js_regcomp("b",0,&e), "aaa", &m, 0)` | returns 1 (no match); JS `exec` -> `null`, `test` -> `false` |
| 35 | `match` I_ANY (regexp.c:1121) | `.` at end of subject: `js_regexec(js_regcomp("a.",0,&e), "a", &m, 0)` | returns 1 (no match) |
| 36 | `match` I_ANY (regexp.c:1124) | `.` positioned on a newline rune (0xA, 0xD, 0x2028, 0x2029): `js_regexec(js_regcomp("a.b",0,&e), "a\nb", &m, 0)` | returns 1 (no match) |
| 37 | `match` I_CHAR (regexp.c:1128) | literal char required at end of subject: `js_regexec(js_regcomp("ab",0,&e), "a", &m, 0)` | returns 1 (no match) |
| 38 | `match` I_CHAR (regexp.c:1133) | literal char mismatch (after `canon()` if REG_ICASE): `js_regexec(js_regcomp("ab",0,&e), "ax", &m, 0)` | returns 1 (no match) |
| 39 | `match` I_CCLASS (regexp.c:1137) | class required at end of subject: `js_regexec(js_regcomp("a[b]",0,&e), "a", &m, 0)` | returns 1 (no match) |
| 40 | `match` I_CCLASS (regexp.c:1141) | REG_ICASE: rune not in class under `incclasscanon`: `js_regexec(js_regcomp("a[b]",REG_ICASE,&e), "aZ", &m, 0)` | returns 1 (no match) |
| 41 | `match` I_CCLASS (regexp.c:1144) | case-sensitive: rune not in class under `incclass`: `js_regexec(js_regcomp("a[b]",0,&e), "az", &m, 0)` | returns 1 (no match) |
| 42 | `match` I_NCCLASS (regexp.c:1149) | negated class required at end of subject: `js_regexec(js_regcomp("a[^b]",0,&e), "a", &m, 0)` | returns 1 (no match) |
| 43 | `match` I_NCCLASS (regexp.c:1152) | REG_ICASE: rune IS in negated class under `incclasscanon`: `js_regexec(js_regcomp("a[^b]",REG_ICASE,&e), "aB", &m, 0)` | returns 1 (no match) |
| 44 | `match` I_NCCLASS (regexp.c:1155) | case-sensitive: rune IS in negated class: `js_regexec(js_regcomp("a[^b]",0,&e), "ab", &m, 0)` | returns 1 (no match) |
| 45 | `match` I_REF (regexp.c:1163) | REG_ICASE back-reference text mismatch via `strncmpcanon`: `js_regexec(js_regcomp("(a)\\1",REG_ICASE,&e), "ab", &m, 0)` | returns 1 (no match) |
| 46 | `match` I_REF (regexp.c:1166) | case-sensitive back-reference mismatch via `strncmp`: `js_regexec(js_regcomp("(a)\\1",0,&e), "ab", &m, 0)` | returns 1 (no match) |
| 47 | `match` I_BOL (regexp.c:1185) | `^` where `sp==bol` but REG_NOTBOL is set (or `sp!=bol` and no preceding newline / REG_NEWLINE clear): `js_regexec(js_regcomp("^a",0,&e), "a", &m, REG_NOTBOL)` | returns 1 (no match); reached from `Rp_test`/`exec` when `re->last > 0` on a `/g` regexp |
| 48 | `match` I_EOL (regexp.c:1197) | `$` where `*sp != 0` and (REG_NEWLINE clear or `*sp` is not a newline): `js_regexec(js_regcomp("a$",0,&e), "ab", &m, 0)` | returns 1 (no match) |
| 49 | `match` I_WORD (regexp.c:1202) | `\b` at a position that is not a word boundary: `js_regexec(js_regcomp("a\\b",0,&e), "ab", &m, 0)` | returns 1 (no match) |
| 50 | `match` I_NWORD (regexp.c:1209) | `\B` at a position that IS a word boundary: `js_regexec(js_regcomp("a\\B",0,&e), "a ", &m, 0)` | returns 1 (no match) |
| 51 | `match` default (regexp.c:1222) | `pc->opcode` not one of the I_* opcodes (corrupted/foreign `Reprog` passed to `js_regexec`); unreachable via `regcompx` | returns 1 (no match) |
| 52 | `strncmpcanon` (regexp.c:1056) | REG_ICASE back-reference compare where subject `a` ends before `n` runes consumed | returns -1 (non-zero) -> I_REF reports no match |
| 53 | `strncmpcanon` (regexp.c:1057) | REG_ICASE back-reference compare where captured text `b` ends before `n` runes consumed | returns 1 (non-zero) -> I_REF reports no match |
| 54 | `js_newregexpx` (jsregexp.c:37-38) | any `js_regcompx` failure (rows 1-32), e.g. `js_newregexp(J, "a{2,1}", 0)` / `new RegExp('[a')` | `js_syntaxerror(J, "regular expression: %s", error)` -> SyntaxError, e.g. `regular expression: invalid quantifier` |
| 55 | `escaperegexp` (jsregexp.c:13) | `js_malloc(J, n+1)` fails while escaping `/` in the source text (`new RegExp('a/b')` under memory exhaustion) | `js_outofmemory` -> thrown Error `out of memory` |
| 56 | `js_newregexpx` (jsregexp.c:41) | `js_strdup(J, pattern)` fails on the clone path (`new RegExp(/a/)` under memory exhaustion) | `js_outofmemory` -> thrown Error `out of memory` |
| 57 | `js_RegExp_prototype_exec` (jsregexp.c:63-66) | `/g` regexp whose `lastIndex` exceeds the subject length: `r=/a/g; r.lastIndex=99; r.exec("a")` | resets `re->last = 0`, `js_pushnull` -> returns `null` (no error) |
| 58 | `js_RegExp_prototype_exec` (jsregexp.c:76-77) | `js_regexec` returns < 0 (row 33): `/a*/.exec(<4999 'a' chars>)` | `js_error(J, "regexec failed")` -> Error `regexec failed` |
| 59 | `js_RegExp_prototype_exec` (jsregexp.c:93-96) | `js_regexec` returns 1, i.e. no match (rows 34-51): `/b/.exec("aaa")` | resets `re->last = 0` if `/g`, `js_pushnull` -> returns `null` |
| 60 | `Rp_test` (jsregexp.c:112-115) | `/g` regexp whose `lastIndex` exceeds the subject length: `r=/a/g; r.lastIndex=99; r.test("a")` | resets `re->last = 0`, `js_pushboolean(J, 0)` -> `false` |
| 61 | `Rp_test` (jsregexp.c:125-126) | `js_regexec` returns < 0 (row 33): `/a*/.test(<4999 'a' chars>)` | `js_error(J, "regexec failed")` -> Error `regexec failed` |
| 62 | `Rp_test` (jsregexp.c:134-137) | `js_regexec` returns 1, i.e. no match: `/b/.test("aaa")` | resets `re->last = 0` if `/g`, `js_pushboolean(J, 0)` -> `false` |
| 63 | `js_toregexp` via `Rp_test` (jsregexp.c:107) | `this` is not a RegExp object: `RegExp.prototype.test.call({}, "a")` | `js_typeerror(J, "not a regexp")` -> TypeError `not a regexp` |
| 64 | `js_toregexp` via `Rp_toString` (jsregexp.c:198) | `this` is not a RegExp object: `RegExp.prototype.toString.call(1)` | `js_typeerror(J, "not a regexp")` -> TypeError `not a regexp` |
| 65 | `js_toregexp` via `Rp_exec` (jsregexp.c:221) | `this` is not a RegExp object: `RegExp.prototype.exec.call("x", "a")` | `js_typeerror(J, "not a regexp")` -> TypeError `not a regexp` |
| 66 | `Rp_toString` (jsregexp.c:205) | `js_malloc(J, strlen(re->source)+6)` fails | `js_try` handler frees `out` and `js_throw`s -> Error `out of memory` |
| 67 | `jsB_new_RegExp` (jsregexp.c:148-149) | flags argument supplied while cloning a RegExp: `new RegExp(/a/, "g")` | `js_typeerror(J, "cannot supply flags when creating one RegExp from another")` -> TypeError |
| 68 | `jsB_new_RegExp` (jsregexp.c:172) | flag character other than `g`/`i`/`m`: `new RegExp("a", "x")` | `js_syntaxerror(J, "invalid regular expression flag: '%c'", *s)` -> SyntaxError `invalid regular expression flag: 'x'` |
| 69 | `jsB_new_RegExp` (jsregexp.c:175) | `g` given more than once: `new RegExp("a", "gg")` | SyntaxError `invalid regular expression flag: 'g'` |
| 70 | `jsB_new_RegExp` (jsregexp.c:176) | `i` given more than once: `new RegExp("a", "ii")` | SyntaxError `invalid regular expression flag: 'i'` |
| 71 | `jsB_new_RegExp` (jsregexp.c:177) | `m` given more than once: `new RegExp("a", "mm")` | SyntaxError `invalid regular expression flag: 'm'` |
| 72 | `chartorune` (utf.c:78) | second byte is not a `10xxxxxx` continuation: `chartorune(&r, "\xC2\x20")` | `*rune = Runeerror (0xFFFD)`, returns 1 (no error raised) |
| 73 | `chartorune` (utf.c:80-82) | lead byte in `0x80..0xBF` (stray continuation byte): `chartorune(&r, "\x80")` | `*rune = 0xFFFD`, returns 1 |
| 74 | `chartorune` (utf.c:84-85) | overlong 2-byte form, decoded `l <= Rune1 (0x7F)`: `chartorune(&r, "\xC1\x81")` | `*rune = 0xFFFD`, returns 1 |
| 75 | `chartorune` (utf.c:94-96) | third byte is not a continuation byte: `chartorune(&r, "\xE0\xA0\x20")` | `*rune = 0xFFFD`, returns 1 |
| 76 | `chartorune` (utf.c:98-100) | overlong 3-byte form, decoded `l <= Rune2 (0x7FF)`: `chartorune(&r, "\xE0\x80\x80")` | `*rune = 0xFFFD`, returns 1 |
| 77 | `chartorune` (utf.c:110-111) | fourth byte is not a continuation byte: `chartorune(&r, "\xF0\x90\x80\x20")` | `*rune = 0xFFFD`, returns 1 |
| 78 | `chartorune` (utf.c:114-116) | overlong 4-byte form, decoded `l <= Rune3 (0xFFFF)`: `chartorune(&r, "\xF0\x80\x80\x80")` | `*rune = 0xFFFD`, returns 1 |
| 79 | `chartorune` (utf.c:117-118) | 4-byte sequence decoding above `Runemax (0x10FFFF)`: `chartorune(&r, "\xF4\x90\x80\x80")` | `*rune = 0xFFFD`, returns 1 |
| 80 | `chartorune` (utf.c:113 false -> `bad`) | lead byte `>= T5 (0xF8)`, i.e. 5/6-byte form or `0xFE`/`0xFF`: `chartorune(&r, "\xF8\x88\x80\x80\x80")`, `chartorune(&r, "\xFF")` | `*rune = 0xFFFD`, returns 1 |
| 81 | `chartorune` (utf.c:58-61) | overlong-NUL special case `0xC0 0x80` accepted instead of rejected: `chartorune(&r, "\xC0\x80")` | `*rune = 0`, returns 2 (2 bytes consumed) |
| 82 | `runetochar` (utf.c:167-168) | rune above `Runemax`: `Rune c = 0x110000; runetochar(buf, &c)` | silently substitutes `Runeerror`; writes `EF BF BD`, returns 3 (same via `runelen(0x110000) == 3`) |
| 83 | `runetochar` (utf.c:137-142) | rune value 0: `Rune c = 0; runetochar(buf, &c)` | writes overlong `C0 80`, returns 2 (never a 1-byte NUL) |
| 84 | `ucd_bsearch` (utf.c:212-214) | codepoint below the first table entry / not covered by any entry: `toupperrune(0x2FFFF)`, `isalpharune('$')` | returns NULL -> `tolowerrune`/`toupperrune` return `c` unchanged; `islowerrune`/`isupperrune`/`isalpharune` return 0 |
| 85 | `tolowerrune_full` (utf.c:291-294) | codepoint with no full lowercase mapping: `tolowerrune_full('a')` | returns NULL (caller must fall back) |
| 86 | `toupperrune_full` (utf.c:301-304) | codepoint with no full uppercase mapping: `toupperrune_full('A')` | returns NULL (caller must fall back) |
| 87 | `minus` (jsdtoa.c:386) | `assert(x.e == y.e)` — `js_grisu2` invariant broken (Wp/Wm exponents differ after `multiply`) | `assert` failure: `Assertion 'x.e == y.e' failed`, `abort()` (SIGABRT); compiled out under `NDEBUG` |
| 88 | `minus` (jsdtoa.c:387) | `assert(x.f >= y.f)` — `Wp.f < Wm.f` after `Wm.f++; Wp.f--` (degenerate/denormal input to `js_grisu2`) | `assert` failure: `Assertion 'x.f >= y.f' failed`, `abort()` (SIGABRT); compiled out under `NDEBUG` |
| 89 | `js_strtod` (jsdtoa.c:618-622) | second `.` or first non-digit terminates the mantissa scan: `js_strtod("1.2.3", &end)` | mantissa scan stops at the second `.`; returns 1.2 with `*endPtr` pointing at `".3"` (trailing text rejected) |
| 90 | `js_strtod` (jsdtoa.c:641-643) | mantissa longer than 18 significant digits: `js_strtod("12345678901234567890123.5", &end)` | digits past 18 are dropped (`fracExp = decPt - 18`); returns 1.2345678901234568e22, `errno` untouched |
| 91 | `js_strtod` (jsdtoa.c:647-650) | no mantissa digits at all: `js_strtod("abc", &end)`, `js_strtod("", &end)`, `js_strtod(".", &end)` | `fraction = 0.0`, `*endPtr = string` (zero characters consumed) -> caller sees "not a number"; returns 0.0 (or -0.0 if a `-` was seen) |
| 92 | `js_strtod` (jsdtoa.c:694-699) | exponent digits accumulate to `exp >= INT_MAX/100`: `js_strtod("1e99999999999999999999", &end)` | remaining exponent digits skipped without accumulating (guards `int` overflow); then clamped by row 94 -> `inf`, `errno = ERANGE` |
| 93 | `js_strtod` (jsdtoa.c:714-717) | `exp < -maxExponent (-511)`: `js_strtod("1e-999", &end)` | clamps `exp = 511`, `expSign = TRUE`, sets `errno = ERANGE`; returns 0.0 (underflow) |
| 94 | `js_strtod` (jsdtoa.c:718-721) | `exp > maxExponent (511)`: `js_strtod("1e999", &end)` | clamps `exp = 511`, `expSign = FALSE`, sets `errno = ERANGE`; returns `inf` (overflow) |
