# PCRE2 (8-bit) Error-Surface Table

Mechanically derived from `c_src/` and **verified against the built
`c_src/build/libpcre2.so`** by calling every public entry point through FFI and
recording the actual return value. Every row marked with a concrete trigger was
executed; rows explicitly annotated `(internal)` / `(unreachable in this build)`
were located by grep but could not be driven from outside.

## Build configuration that this table describes

Read from `c_src/CMakeLists.txt` + `c_src/src/config.h`, and confirmed at
runtime via `pcre2_config_8`:

| knob | value | consequence |
|---|---|---|
| `PCRE2_CODE_UNIT_WIDTH` | `8` | only the `*_8` entry points exist |
| `SUPPORT_UNICODE` | **defined** (added by CMake, *not* by `config.h`) | `PCRE2_UTF` / `PCRE2_UCP` work; `ERR32`, `ERR45`, `ERR96` unreachable |
| `SUPPORT_JIT` | **not** defined | `pcre2_jit_compile_8` always fails; `pcre2_jit_match_8` always fails; `pcre2_jit_stack_create_8` always `NULL` |
| `LINK_SIZE` | 2 | |
| `MAX_NAME_SIZE` | 128 | group-name length limit (`ERR48`) |
| `MAX_NAME_COUNT` | 10000 | number of named groups (`ERR49`) |
| `MAX_GROUP_NUMBER` | 65535 | `ERR61` / `ERR97` |
| `MAX_REPEAT_COUNT` | 65535 | `ERR5` |
| `PARENS_NEST_LIMIT` | 250 (default, settable) | `ERR19` |
| `ECLASS_NEST_LIMIT` | 15 | `ERR107` |
| `MATCH_LIMIT` / `MATCH_LIMIT_DEPTH` | 10000000 | `PCRE2_ERROR_MATCHLIMIT` / `DEPTHLIMIT` |
| `HEAP_LIMIT` | 20000000 (KiB) | `PCRE2_ERROR_HEAPLIMIT` |
| `MAX_MARK` | 255 | `ERR76` |
| `EBCDIC` | not defined | affects `ERR68` message/behaviour |

**Numbering convention.** Compile-time errors are `ERRn` where the numeric value
returned through `*errorcode` is `COMPILE_ERROR_BASE + n = 100 + n`
(`pcre2_internal.h:216`, `pcre2_compile.h:53`). Match-time errors are the
negative `PCRE2_ERROR_*` constants in `c_src/include/pcre2.h:350-441`.
`ERR59` is a retired slot ("obsolete error (should not occur)") and is never
assigned anywhere in the sources.

---

## A. `pcre2_compile_8` — argument / option / context validation

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `pcre2_compile_8` | `errorptr == NULL` (any pattern) | `NULL`; `*erroroffset = 0` if non-NULL; **no** error code written |
| 2 | `pcre2_compile_8` | `erroroffset == NULL`, `errorptr != NULL` | `NULL` + `*errorptr = ERR120 (=220)` "erroroffset passed as NULL" |
| 3 | `pcre2_compile_8` | `pattern == NULL` with `patlen == 1` (non-zero) | `NULL` + `ERR16 (=116)` "pattern passed as NULL with non-zero length" |
| 4 | `pcre2_compile_8` | `pattern == NULL`, `patlen == 0` | **success** — treated as empty pattern (not an error; boundary row) |
| 5 | `pcre2_compile_8` | `options = 0x80000000`… any bit outside `PUBLIC_COMPILE_OPTIONS` (e.g. `0x00000001`, `0x10000000`) | `NULL` + `ERR17 (=117)` "unrecognised compile-time option bit(s)" |
| 6 | `pcre2_compile_8` | `pcre2_set_compile_extra_options_8(cc, 0x80000000)` — bit outside `PUBLIC_COMPILE_EXTRA_OPTIONS` | `NULL` + `ERR17 (=117)` |
| 7 | `pcre2_compile_8` | `options = PCRE2_LITERAL\|PCRE2_DOTALL` (any bit outside `PUBLIC_LITERAL_COMPILE_OPTIONS` together with `PCRE2_LITERAL`) | `NULL` + `ERR92 (=192)` "invalid option bits with PCRE2_LITERAL" |
| 8 | `pcre2_compile_8` | `PCRE2_LITERAL` + extra option outside `PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS` (e.g. `PCRE2_EXTRA_ALT_BSUX`) | `NULL` + `ERR92 (=192)` |
| 9 | `pcre2_compile_8` | `pcre2_set_max_pattern_length_8(cc, 1)` then compile `"ab"` (patlen 2 > limit) | `NULL` + `ERR88 (=188)` "pattern string is longer than the limit set by the application" |
| 10 | `pcre2_compile_8` | `pcre2_set_max_pattern_compiled_length_8(cc, 1)` then compile `"abcdef"` | `NULL` + `ERR101 (=201)` "compiled pattern would be longer than the limit set by the application" |
| 11 | `pcre2_compile_8` | `PCRE2_NEVER_UTF` + pattern `(*UTF)a` | `NULL` + `ERR74 (=174)` "using UTF is disabled by the application" |
| 12 | `pcre2_compile_8` | `PCRE2_NEVER_UCP` + pattern `(*UCP)a` | `NULL` + `ERR75 (=175)` "using UCP is disabled by the application" |
| 13 | `pcre2_compile_8` | `PCRE2_NEVER_BACKSLASH_C` + pattern `\C` | `NULL` + `ERR83 (=183)` "using \\C is disabled by the application" |
| 14 | `pcre2_compile_8` | `PCRE2_EXTRA_NEVER_CALLOUT` + pattern `(?C1)` | `NULL` + `ERR103 (=203)` "using callouts is disabled by the application" |
| 15 | `pcre2_compile_8` | `PCRE2_EXTRA_NO_BS0` + pattern `\0` | `NULL` + `ERR98 (=198)` "octal digit missing after \\0" |
| 16 | `pcre2_compile_8` | `PCRE2_EXTRA_PYTHON_OCTAL` + pattern `\400` | `NULL` + `ERR102 (=202)` "octal value given by \\ddd is greater than \\377" |
| 17 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` with neither `PCRE2_UTF` nor `PCRE2_UCP` | `NULL` + `ERR104 (=204)` "PCRE2_EXTRA_TURKISH_CASING require Unicode (UTF or UCP) mode" |
| 18 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` + `PCRE2_UCP` but **not** `PCRE2_UTF` (8-bit lib) | `NULL` + `ERR105 (=205)` "PCRE2_EXTRA_TURKISH_CASING requires UTF in 8-bit mode" |
| 19 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING\|PCRE2_EXTRA_CASELESS_RESTRICT` + `PCRE2_UTF` | `NULL` + `ERR106 (=206)` "…are not compatible" |
| 20 | `pcre2_compile_8` | `pcre2_set_compile_recursion_guard_8(cc, guard, …)` where `guard` returns non-zero, pattern `(((a)))` | `NULL` + `ERR33 (=133)` "parentheses are too deeply nested (stack check)" |
| 21 | `pcre2_compile_8` | general context whose `malloc` returns `NULL` (heap allocation of the compiled block fails) | `NULL` + `ERR21 (=121)` "failed to allocate heap memory" |
| 22 | `pcre2_compile_8` | `PCRE2_UTF` + malformed UTF-8 pattern bytes (e.g. `"\x80"`), `PCRE2_NO_UTF_CHECK` not set | `NULL` + the negative `PCRE2_ERROR_UTF8_ERRn` from `_pcre2_valid_utf_8` (e.g. `-22`); `*erroroffset` = byte offset |
| 23 | `pcre2_compile_8` | `PCRE2_UTF\|PCRE2_UCP` when the library is built **without** `SUPPORT_UNICODE` | `NULL` + `ERR32 (=132)` — **unreachable in this build** (Unicode is compiled in) |
| 24 | `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` + `PCRE2_UTF` in **16-bit** mode | `NULL` + `ERR91 (=191)` — **unreachable in this build** (8-bit only) |
| 25 | `pcre2_compile_8` | `\C` when library built with `NEVER_BACKSLASH_C` | `NULL` + `ERR85 (=185)` — **unreachable in this build** |
| 26 | `pcre2_compile_8` | invalid `newline_convention` reaching the compile-time `switch` default | `NULL` + `ERR56 (=156)` (internal; `pcre2_set_newline_8` already rejects bad values) |

## B. `pcre2_compile_8` — bad pattern strings (`ERR1`…`ERR119`)

Every trigger below was executed and produced exactly the stated code.
`*erroroffset` is also set; representative offsets are noted where useful.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 27 | `pcre2_compile_8` | pattern `a\` — backslash at end of pattern | `NULL` + `ERR1 (=101)`, `*erroroffset = 2` |
| 28 | `pcre2_compile_8` | pattern `\c` — `\c` at end of pattern | `NULL` + `ERR2 (=102)`, offset 2 |
| 29 | `pcre2_compile_8` | pattern `\q` — unrecognized character after `\` | `NULL` + `ERR3 (=103)`, offset 2 |
| 30 | `pcre2_compile_8` | pattern `a{2,1}` — quantifier max < min | `NULL` + `ERR4 (=104)`, offset 5 |
| 31 | `pcre2_compile_8` | pattern `a{65536}` — count > `MAX_REPEAT_COUNT` (65535) | `NULL` + `ERR5 (=105)`, offset 7 |
| 32 | `pcre2_compile_8` | pattern `[a` — no closing `]` | `NULL` + `ERR6 (=106)`, offset 2 |
| 33 | `pcre2_compile_8` | pattern `[\A]` — escape invalid inside a class (also `[\Z]`, `[\z]`, `[\G]`, `[\R]`, `[\X]`, `[\K]`) | `NULL` + `ERR7 (=107)`, offset 3 |
| 34 | `pcre2_compile_8` | pattern `[b-a]` — class range end < start | `NULL` + `ERR8 (=108)`, offset 4 |
| 35 | `pcre2_compile_8` | pattern `*a` — quantifier with nothing to repeat (also `a**`, `+b`, `?x`, `{2}`) | `NULL` + `ERR9 (=109)`, offset 1 |
| 36 | `pcre2_compile_8` | "internal error: unexpected repeat" — non-repeatable META reaching the repeat handler | `NULL` + `ERR10 (=110)` **(internal)** |
| 37 | `pcre2_compile_8` | pattern `(?%)` — unrecognized character after `(?` / `(?-` | `NULL` + `ERR11 (=111)`, offset 3 |
| 38 | `pcre2_compile_8` | pattern `[:alpha:]` — POSIX class syntax used outside a class | `NULL` + `ERR12 (=112)`, offset 9 |
| 39 | `pcre2_compile_8` | pattern `[[.ch.]]` — POSIX collating element | `NULL` + `ERR13 (=113)`, offset 7 |
| 40 | `pcre2_compile_8` | pattern `(a` — missing closing parenthesis | `NULL` + `ERR14 (=114)`, offset 2 |
| 41 | `pcre2_compile_8` | pattern `\1` (or `(a)\2`, `(?2)`, `\g{2}`) — reference to non-existent subpattern | `NULL` + `ERR15 (=115)`, offset 2 |
| 42 | `pcre2_compile_8` | see row 3 — `pattern == NULL` with `patlen != 0` | `NULL` + `ERR16 (=116)` |
| 43 | `pcre2_compile_8` | see rows 5-6 — undefined option / extra-option bit | `NULL` + `ERR17 (=117)` |
| 44 | `pcre2_compile_8` | pattern `(?#abc` — unterminated `(?#` comment | `NULL` + `ERR18 (=118)`, offset 6 |
| 45 | `pcre2_compile_8` | pattern `"("*260 + "a" + ")"*260` — nesting > `parens_nest_limit` (250) | `NULL` + `ERR19 (=119)`, offset 251 |
| 46 | `pcre2_compile_8` | pattern `(?:(?:(?:(?:a{255}){255}){255}){255})` — compiled size > `MAX_PATTERN_SIZE` / length counter overflow | `NULL` + `ERR20 (=120)` "regular expression is too large" |
| 47 | `pcre2_compile_8` | see row 21 — allocator failure during compile (also in `pcre2_compile_class.c:1127`, `pcre2_compile_cgroup.c:384,531`) | `NULL` + `ERR21 (=121)` |
| 48 | `pcre2_compile_8` | pattern `a)` — unmatched `)` | `NULL` + `ERR22 (=122)`, offset 2 |
| 49 | `pcre2_compile_8` | "internal error: code overflow" (`pcre2_compile.c:10995`) | `NULL` + `ERR23 (=123)` **(internal)** |
| 50 | `pcre2_compile_8` | pattern `(a)(*scs:(1x)b)` — capture list item followed by neither `,` nor `)`; also `(?(1)a` shapes reaching `pcre2_compile.c:2815/5590` | `NULL` + `ERR24 (=124)` "missing closing parenthesis for condition", offset 11 |
| 51 | `pcre2_compile_8` | pattern `(?<=a*)` (also `(?<=a+)`, `(?<=a{0,})`) — unbounded lookbehind | `NULL` + `ERR25 (=125)` "length of lookbehind assertion is not limited" |
| 52 | `pcre2_compile_8` | pattern `\g{+0}` (also `\g{-0}`, `(?-0)`) — relative reference of zero | `NULL` + `ERR26 (=126)`, offset 2 |
| 53 | `pcre2_compile_8` | pattern `(a)(?(1)a\|b\|c)` — conditional group with 3 branches | `NULL` + `ERR27 (=127)`, offset 3 |
| 54 | `pcre2_compile_8` | pattern `(?(?i)a)` — `(?(?` not followed by an assertion | `NULL` + `ERR28 (=128)` "atomic assertion expected after (?( or (?(?C)", offset 3 |
| 55 | `pcre2_compile_8` | pattern `(?+a)` — no digit after `(?+` | `NULL` + `ERR29 (=129)`, offset 4 |
| 56 | `pcre2_compile_8` | pattern `[[:foo:]]` — unknown POSIX class name | `NULL` + `ERR30 (=130)`, offset 8 |
| 57 | `pcre2_compile_8` | "internal error in pcre2_study()" (`pcre2_compile.c:11254`) | `NULL` + `ERR31 (=131)` **(internal)** |
| 58 | `pcre2_compile_8` | `PCRE2_UTF`/`PCRE2_UCP` with no Unicode support | `NULL` + `ERR32 (=132)` — **unreachable in this build** |
| 59 | `pcre2_compile_8` | see row 20 — stack-guard callback returns non-zero | `NULL` + `ERR33 (=133)` |
| 60 | `pcre2_compile_8` | pattern `\x{110000}` (also `\o{4200000}`) — code point > 0x10FFFF | `NULL` + `ERR34 (=134)`, offset 9 |
| 61 | `pcre2_compile_8` | pattern `(?<=` + `(?\|a\|b)`×2001 + `)x` — lookbehind length computation exceeds the 2000-iteration cap (`pcre2_compile.c:9600`) | `NULL` + `ERR35 (=135)` "lookbehind is too complicated", offset 0 |
| 62 | `pcre2_compile_8` | `PCRE2_UTF` + pattern `(?<=\C)a` — `\C` inside a lookbehind in UTF mode (compiles WITHOUT `PCRE2_UTF`) | `NULL` + `ERR36 (=136)` "\\C is not allowed in a lookbehind assertion in UTF-8 mode", offset 0 |
| 63 | `pcre2_compile_8` | pattern `\L` (also `\F`, `\l`, `\U`, `\u`, `\N{name}`) | `NULL` + `ERR37 (=137)`, offset 2 |
| 64 | `pcre2_compile_8` | pattern `(?C256)` — callout number > 255 | `NULL` + `ERR38 (=138)`, offset 6 |
| 65 | `pcre2_compile_8` | pattern `(?C1x` — no `)` after `(?C<number>` | `NULL` + `ERR39 (=139)`, offset 4 |
| 66 | `pcre2_compile_8` | `PCRE2_ALT_VERBNAMES` + pattern `(*MARK:\d)` — non-literal escape inside a verb name | `NULL` + `ERR40 (=140)`, offset 9 |
| 67 | `pcre2_compile_8` | pattern `(?Px)` — unrecognized character after `(?P` | `NULL` + `ERR41 (=141)`, offset 4 |
| 68 | `pcre2_compile_8` | pattern `(?<ab` — unterminated group name | `NULL` + `ERR42 (=142)`, offset 5 |
| 69 | `pcre2_compile_8` | pattern `(?<a>x)(?<a>y)` without `PCRE2_DUPNAMES` | `NULL` + `ERR43 (=143)`, offset 12 |
| 70 | `pcre2_compile_8` | pattern `(?<1a>x)` — group name starting with a digit | `NULL` + `ERR44 (=144)`, offset 4 |
| 71 | `pcre2_compile_8` | `\p{L}` / `\P{L}` / `\X` without Unicode support | `NULL` + `ERR45 (=145)` — **unreachable in this build** |
| 72 | `pcre2_compile_8` | pattern `\p` (also `\p{`, `\P`) — malformed `\p`/`\P` | `NULL` + `ERR46 (=146)`, offset 2 |
| 73 | `pcre2_compile_8` | pattern `\p{Foo}` — unknown Unicode property name | `NULL` + `ERR47 (=147)`, offset 7 |
| 74 | `pcre2_compile_8` | pattern `(?<` + `n`×129 + `>a)` — subpattern name longer than `MAX_NAME_SIZE` (128) | `NULL` + `ERR48 (=148)`, offset 132 |
| 75 | `pcre2_compile_8` | 10005 distinct named groups `(?<n0>a)(?<n1>a)…` — more than `MAX_NAME_COUNT` (10000) | `NULL` + `ERR49 (=149)` |
| 76 | `pcre2_compile_8` | pattern `[\d-z]` — class range with a class escape as an endpoint | `NULL` + `ERR50 (=150)` "invalid range in character class", offset 4 |
| 77 | `pcre2_compile_8` | pattern `\777` in 8-bit non-UTF mode — octal value > `\377` | `NULL` + `ERR51 (=151)`, offset 4 |
| 78 | `pcre2_compile_8` | "internal error: overran compiling workspace" (`pcre2_compile.c:6170`) | `NULL` + `ERR52 (=152)` **(internal)** |
| 79 | `pcre2_compile_8` | "previously-checked referenced subpattern not found" (`pcre2_compile.c:6991`, `pcre2_compile_cgroup.c:235`) | `NULL` + `ERR53 (=153)` **(internal)** |
| 80 | `pcre2_compile_8` | pattern `(?(DEFINE)(a)\|(b))` — DEFINE with more than one branch | `NULL` + `ERR54 (=154)`, offset 3 |
| 81 | `pcre2_compile_8` | pattern `\o1` — missing `{` after `\o` | `NULL` + `ERR55 (=155)`, offset 2 |
| 82 | `pcre2_compile_8` | unknown newline setting reaching the compile switch default | `NULL` + `ERR56 (=156)` **(internal)** |
| 83 | `pcre2_compile_8` | pattern `\g` (also `\g?`) — `\g` not followed by name/number | `NULL` + `ERR57 (=157)`, offset 2 |
| 84 | `pcre2_compile_8` | pattern `(?R` — no `)` after `(?R` | `NULL` + `ERR58 (=158)`, offset 3 |
| 85 | `pcre2_compile_8` | `ERR59` — retired slot, "obsolete error (should not occur)"; not assigned anywhere in `c_src` | never returned |
| 86 | `pcre2_compile_8` | pattern `(*ZZZ)` (also `(*MARK`, `(*THEN:` malformed) — unknown/malformed verb | `NULL` + `ERR60 (=160)`, offset 5 |
| 87 | `pcre2_compile_8` | pattern `(?99999)` — group number > `MAX_GROUP_NUMBER` (65535) | `NULL` + `ERR61 (=161)`, offset 7 |
| 88 | `pcre2_compile_8` | pattern `(?&)` (also `(?P>)`) — subpattern name expected | `NULL` + `ERR62 (=162)`, offset 3 |
| 89 | `pcre2_compile_8` | "internal error: parsed pattern overflow" (`pcre2_compile.c:3193,3269,5912`) | `NULL` + `ERR63 (=163)` **(internal)** |
| 90 | `pcre2_compile_8` | pattern `\o{1z}` — non-octal character inside `\o{}` | `NULL` + `ERR64 (=164)`, offset 5 |
| 91 | `pcre2_compile_8` | pattern `(?\|(?<a>x)\|(?<b>y))` — different names for the same group number | `NULL` + `ERR65 (=165)`, offset 16 |
| 92 | `pcre2_compile_8` | pattern `(*MARK)` — `(*MARK)` with no argument | `NULL` + `ERR66 (=166)`, offset 6 |
| 93 | `pcre2_compile_8` | pattern `\x{1z}` — non-hex character inside `\x{}` | `NULL` + `ERR67 (=167)`, offset 5 |
| 94 | `pcre2_compile_8` | pattern `\c` followed by the literal byte 0x7F (also 0x00, 0x80, 0xFF in non-UTF mode) — `\c` must be followed by a printable ASCII character | `NULL` + `ERR68 (=168)`, offset 3 |
| 95 | `pcre2_compile_8` | pattern `\k` — `\k` not followed by `{`, `<` or `'` | `NULL` + `ERR69 (=169)`, offset 2 |
| 96 | `pcre2_compile_8` | "unknown meta code in check_lookbehinds()" (`pcre2_compile.c:10127`) | `NULL` + `ERR70 (=170)` **(internal)** |
| 97 | `pcre2_compile_8` | pattern `[\N]` — `\N` inside a class | `NULL` + `ERR71 (=171)`, offset 3 |
| 98 | `pcre2_compile_8` | callout string argument longer than `UINT32_MAX` code units (`pcre2_compile.c:5364`) | `NULL` + `ERR72 (=172)` — needs a >4 GiB pattern, effectively **unreachable** |
| 99 | `pcre2_compile_8` | `PCRE2_UTF` + pattern `\x{d800}` (also `[\x{d800}]`, `\N{U+D800}`) — surrogate code point | `NULL` + `ERR73 (=173)`, offset 7 |
| 100 | `pcre2_compile_8` | see row 11 — `PCRE2_NEVER_UTF` + `(*UTF)` | `NULL` + `ERR74 (=174)` |
| 101 | `pcre2_compile_8` | see row 12 — `PCRE2_NEVER_UCP` + `(*UCP)` | `NULL` + `ERR75 (=175)` |
| 102 | `pcre2_compile_8` | pattern `(*MARK:` + `m`×256 + `)a` — verb name longer than `MAX_MARK` (255) | `NULL` + `ERR76 (=176)`, offset 263 |
| 103 | `pcre2_compile_8` | `PCRE2_ALT_BSUX\|PCRE2_EXTRA_ALT_BSUX` + pattern `\u{110000}` | `NULL` + `ERR77 (=177)`, offset 10 |
| 104 | `pcre2_compile_8` | `PCRE2_UTF` + pattern `\N{U+}` (also `\x{}`, `\o{}`, `\x{ }` in any mode) — digits missing | `NULL` + `ERR78 (=178)`, offset 5 |
| 105 | `pcre2_compile_8` | pattern `(?(VERSION>=x)a)` — bad `(?(VERSION` condition | `NULL` + `ERR79 (=179)`, offset 12 |
| 106 | `pcre2_compile_8` | "unknown opcode in auto_possessify()" (`pcre2_compile.c:11092`) | `NULL` + `ERR80 (=180)` **(internal)** |
| 107 | `pcre2_compile_8` | pattern `(?C{abc` — unterminated string callout delimiter | `NULL` + `ERR81 (=181)`, offset 3 |
| 108 | `pcre2_compile_8` | pattern `(?C*abc*)` — `*` is not a legal callout string delimiter | `NULL` + `ERR82 (=182)`, offset 4 |
| 109 | `pcre2_compile_8` | see row 13 — `PCRE2_NEVER_BACKSLASH_C` + `\C` | `NULL` + `ERR83 (=183)` |
| 110 | `pcre2_compile_8` | `parens_nest_limit` raised to 100000 + pattern `(?\|`×400 + `a` + `)`×400 (also `(?J:`×400, `(?x:`×400) — `(?\|`/`(?J:`/`(?x:` nesting deeper than 255 | `NULL` + `ERR84 (=184)`, offset 1127 (with the DEFAULT nest limit of 250 you get `ERR19 (=119)` first) |
| 111 | `pcre2_compile_8` | `\C` in a library built with `NEVER_BACKSLASH_C` | `NULL` + `ERR85 (=185)` — **unreachable in this build** |
| 112 | `pcre2_compile_8` | pre-compile workspace approaching `workspace_size - WORK_SIZE_SAFETY_MARGIN` (`pcre2_compile.c:6179`) | `NULL` + `ERR86 (=186)` "regular expression is too complicated" — reachable only with pathological patterns |
| 113 | `pcre2_compile_8` | pattern `(?<=` + `a`×70000 + `)b` — lookbehind longer than `LOOKBEHIND_MAX` (65535) | `NULL` + `ERR87 (=187)` "lookbehind assertion is too long", offset 0 |
| 114 | `pcre2_compile_8` | see row 9 — `max_pattern_length` exceeded | `NULL` + `ERR88 (=188)` |
| 115 | `pcre2_compile_8` | "unknown code in parsed pattern" (`pcre2_compile.c:8399`) | `NULL` + `ERR89 (=189)` **(internal)** |
| 116 | `pcre2_compile_8` | "bad code value in parsed_skip()" (`pcre2_compile.c:9981`) | `NULL` + `ERR90 (=190)` **(internal)** |
| 117 | `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` in UTF-16 mode | `NULL` + `ERR91 (=191)` — **unreachable in this build** |
| 118 | `pcre2_compile_8` | see rows 7-8 — bad option bits together with `PCRE2_LITERAL` | `NULL` + `ERR92 (=192)` |
| 119 | `pcre2_compile_8` | pattern `\N{U+41}` **without** `PCRE2_UTF` | `NULL` + `ERR93 (=193)` "\\N{U+dddd} is supported only in Unicode (UTF) mode", offset 8 |
| 120 | `pcre2_compile_8` | pattern `(?^i-m)a` — `-` after `^` in an option group (also `(?i-m-s)`) | `NULL` + `ERR94 (=194)` "invalid hyphen in option setting", offset 5 |
| 121 | `pcre2_compile_8` | pattern `(*zzz:a)` — unknown `(*alpha_assertion)` | `NULL` + `ERR95 (=195)`, offset 5 |
| 122 | `pcre2_compile_8` | `(*script_run:…)` without Unicode support | `NULL` + `ERR96 (=196)` — **unreachable in this build** |
| 123 | `pcre2_compile_8` | pattern `(a)`×70000 — more than 65535 capturing groups | `NULL` + `ERR97 (=197)` |
| 124 | `pcre2_compile_8` | see row 15 — `PCRE2_EXTRA_NO_BS0` + `\0` | `NULL` + `ERR98 (=198)` |
| 125 | `pcre2_compile_8` | pattern `(?=a\K)` (or `(?<=\Ka)`) without `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` | `NULL` + `ERR99 (=199)`, offset 7 |
| 126 | `pcre2_compile_8` | pattern `(?<=a{0,300}b)` — variable lookbehind branch longer than `max_varlookbehind` (default 255) | `NULL` + `ERR100 (=200)` "branch too long in variable-length lookbehind assertion" |
| 127 | `pcre2_compile_8` | see row 10 — `max_pattern_compiled_length` exceeded | `NULL` + `ERR101 (=201)` |
| 128 | `pcre2_compile_8` | see row 16 — `PCRE2_EXTRA_PYTHON_OCTAL` + `\400` | `NULL` + `ERR102 (=202)` |
| 129 | `pcre2_compile_8` | see row 14 — `PCRE2_EXTRA_NEVER_CALLOUT` + `(?C1)` | `NULL` + `ERR103 (=203)` |
| 130 | `pcre2_compile_8` | see row 17 | `NULL` + `ERR104 (=204)` |
| 131 | `pcre2_compile_8` | see row 18 | `NULL` + `ERR105 (=205)` |
| 132 | `pcre2_compile_8` | see row 19 | `NULL` + `ERR106 (=206)` |
| 133 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `"["×16 + "a" + "]"×16` — nesting ≥ `ECLASS_NEST_LIMIT` (15) | `NULL` + `ERR107 (=207)`, offset 15 |
| 134 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `[a---b]` — triple-repeated set operator | `NULL` + `ERR108 (=208)` "invalid operator in extended character class", offset 5 |
| 135 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `[&&a]` — operator with no preceding operand | `NULL` + `ERR109 (=209)`, offset 3 |
| 136 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `[a&&]` — operator with no following operand | `NULL` + `ERR110 (=210)`, offset 5 |
| 137 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `[a&&b\|\|c]` — mixed operator precedence | `NULL` + `ERR111 (=211)`, offset 7 |
| 138 | `pcre2_compile_8` | `PCRE2_ALT_EXTENDED_CLASS` + pattern `[[a]` — unterminated extended class | `NULL` + `ERR112 (=212)`, offset 4 |
| 139 | `pcre2_compile_8` | pattern `(?[[a] [b]])` — implicit union in a Perl extended class | `NULL` + `ERR113 (=213)` "unexpected expression … (no preceding operator)", offset 8 |
| 140 | `pcre2_compile_8` | pattern `(?[])` — empty extended class expression | `NULL` + `ERR114 (=214)`, offset 4 |
| 141 | `pcre2_compile_8` | pattern `(?[[a]]x` — `]` closing `(?[` not followed by `)` | `NULL` + `ERR115 (=215)`, offset 7 |
| 142 | `pcre2_compile_8` | pattern `(?[[a] @ [b]])` — `@` is not valid inside `(?[...])` | `NULL` + `ERR116 (=216)`, offset 8 |
| 143 | `pcre2_compile_8` | pattern `(a)(*scs:(x)b)` — capture list item that is neither number nor `<name>`/`'name'` | `NULL` + `ERR117 (=217)` "expected capture group number or name", offset 10 |
| 144 | `pcre2_compile_8` | pattern `(a)(*scs:1)b)` — `(*scs:` capture list not starting with `(` | `NULL` + `ERR118 (=218)` "missing opening parenthesis", offset 9 |
| 145 | `pcre2_compile_8` | pattern `\g{1x` (also `\g{+1x}`) — missing terminator in a subpattern-number reference | `NULL` + `ERR119 (=219)`, offset 4 |
| 146 | `pcre2_compile_8` | see row 2 — `erroroffset == NULL` | `NULL` + `ERR120 (=220)` |

## C. `pcre2_match_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 147 | `pcre2_match_8` | `match_data == NULL` | `PCRE2_ERROR_NULL (-51)` (returned directly; nothing written) |
| 148 | `pcre2_match_8` | `code == NULL` (`match_data` valid) | `PCRE2_ERROR_NULL (-51)`, also stored in `match_data->rc` |
| 149 | `pcre2_match_8` | `subject == NULL` with `length = 3` | `PCRE2_ERROR_NULL (-51)` |
| 150 | `pcre2_match_8` | `subject == NULL`, `length = 0` | **success path** — treated as empty string (boundary row) |
| 151 | `pcre2_match_8` | `options = PCRE2_UTF (0x00080000)` — any bit outside `PUBLIC_MATCH_OPTIONS` | `PCRE2_ERROR_BADOPTION (-34)` |
| 152 | `pcre2_match_8` | `start_offset = 4`, `length = 3` | `PCRE2_ERROR_BADOFFSET (-33)` |
| 153 | `pcre2_match_8` | `code` pointing at non-PCRE2 memory (e.g. a zeroed 4 KiB buffer) — `magic_number != MAGIC_NUMBER` | `PCRE2_ERROR_BADMAGIC (-31)` |
| 154 | `pcre2_match_8` | code compiled by a different code-unit width (`(re->flags & PCRE2_MODE_MASK) != 1`), or forged flags | `PCRE2_ERROR_BADMODE (-32)` |
| 155 | `pcre2_match_8` | `PCRE2_PARTIAL_HARD\|PCRE2_ENDANCHORED` (or `PARTIAL_SOFT` + `ENDANCHORED` on the pattern) | `PCRE2_ERROR_BADOPTION (-34)` |
| 156 | `pcre2_match_8` | `pcre2_set_offset_limit_8(mc, 1)` but pattern compiled without `PCRE2_USE_OFFSET_LIMIT` | `PCRE2_ERROR_BADOFFSETLIMIT (-56)` |
| 157 | `pcre2_match_8` | pattern `abc`, subject `xyz` | `PCRE2_ERROR_NOMATCH (-1)` |
| 158 | `pcre2_match_8` | pattern `abc`, subject `ab`, `PCRE2_PARTIAL_SOFT` (or `PARTIAL_HARD`) | `PCRE2_ERROR_PARTIAL (-2)` |
| 159 | `pcre2_match_8` | `pcre2_set_match_limit_8(mc, 1)` + `PCRE2_NO_START_OPTIMIZE` pattern `a*b` on `"aaaaaaaaaa"` | `PCRE2_ERROR_MATCHLIMIT (-47)` |
| 160 | `pcre2_match_8` | `pcre2_set_depth_limit_8(mc, 1)` + `PCRE2_NO_START_OPTIMIZE` pattern `a*b` on `"aaaaaaaaaa"` | `PCRE2_ERROR_DEPTHLIMIT (-53)` |
| 161 | `pcre2_match_8` | `pcre2_set_heap_limit_8(mc, 0)` + pattern `(a+)+b` on a 400-char subject (heapframe vector cannot grow) | `PCRE2_ERROR_HEAPLIMIT (-63)` |
| 162 | `pcre2_match_8` | allocator failure while growing the heapframe vector (`pcre2_match.c:768,793,7537`) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 163 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` and the subject copy allocation fails (`pcre2_match.c:7226,8195`) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 164 | `pcre2_match_8` | pattern `(?1)((?1))` (or `((?2))((?1))`, `(a\|(?R))*`) on `"aaa"` — recursion re-entered at the same position | `PCRE2_ERROR_RECURSELOOP (-52)` |
| 165 | `pcre2_match_8` | `PCRE2_UTF` pattern, subject byte `0x80` at offset 0 | `PCRE2_ERROR_UTF8_ERR20 (-22)` "isolated byte with 0x80 bit set" |
| 166 | `pcre2_match_8` | `PCRE2_UTF` pattern, subject `"\xc3\xa9a"`, `start_offset = 1` (mid-character) | `PCRE2_ERROR_BADUTFOFFSET (-36)` |
| 167 | `pcre2_match_8` | `PCRE2_UTF` pattern, subject `"a\xc3"` (truncated 2-byte sequence) | `PCRE2_ERROR_UTF8_ERR1 (-3)` "1 byte missing at end" |
| 168 | `pcre2_match_8` | `PCRE2_UTF` pattern, subject byte `0xfe` | `PCRE2_ERROR_UTF8_ERR21 (-23)` "illegal byte (0xfe or 0xff)" |
| 169 | `pcre2_match_8` | `match_data` from `pcre2_match_data_create_8(1, …)` with pattern `(a)(b)(c)` that matches | returns **`0`** (success, ovector too small) — not an error code |
| 170 | `pcre2_match_8` | `\K` reached via a subroutine call from inside a lookaround so that `Fstart_match` moves outside `[start_offset, Feptr]`, without `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` | `PCRE2_ERROR_BAD_BACKSLASH_K (-75)` — reachable only with contrived recursion (`pcre2_match.c:1030`); direct `(?=a\K)` is rejected at compile time (`ERR99`) |
| 171 | `pcre2_match_8` | corrupted/overwritten compiled pattern reaching an unknown opcode (`pcre2_match.c:2876,3229,3507,…,6941`) | `PCRE2_ERROR_INTERNAL (-44)` **(internal)** |
| 172 | `pcre2_match_8` | `heapframes_size` computation overflow — `max_size < frame_size` (`pcre2_match.c:7521`) | `PCRE2_ERROR_HEAPLIMIT (-63)` **(internal)** |

## D. `pcre2_dfa_match_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 173 | `pcre2_dfa_match_8` | `match_data == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 174 | `pcre2_dfa_match_8` | `code == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 175 | `pcre2_dfa_match_8` | `subject == NULL` with `length = 1` | `PCRE2_ERROR_NULL (-51)` |
| 176 | `pcre2_dfa_match_8` | `workspace == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 177 | `pcre2_dfa_match_8` | `options = PCRE2_DISABLE_RECURSELOOP_CHECK (0x00040000)` — any bit outside `PUBLIC_DFA_MATCH_OPTIONS` (also `PCRE2_NO_JIT`) | `PCRE2_ERROR_BADOPTION (-34)` |
| 178 | `pcre2_dfa_match_8` | `wscount = 19` (< 20) | `PCRE2_ERROR_DFA_WSSIZE (-43)` |
| 179 | `pcre2_dfa_match_8` | `start_offset = 5`, `length = 1` | `PCRE2_ERROR_BADOFFSET (-33)` |
| 180 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_HARD\|PCRE2_ENDANCHORED` | `PCRE2_ERROR_BADOPTION (-34)` |
| 181 | `pcre2_dfa_match_8` | pattern compiled with `PCRE2_MATCH_INVALID_UTF` | `PCRE2_ERROR_DFA_UINVALID_UTF (-66)` |
| 182 | `pcre2_dfa_match_8` | `code` pointing at non-PCRE2 memory. NOTE: `pcre2_dfa_match.c:3420` tests `PCRE2_MATCH_INVALID_UTF` in `overall_options` BEFORE the magic-number check at `:3425`, so junk whose `overall_options` happens to carry that bit yields `DFA_UINVALID_UTF` instead | `PCRE2_ERROR_BADMAGIC (-31)` for an all-zero buffer; `PCRE2_ERROR_BADMAGIC (-31)` **or** `PCRE2_ERROR_DFA_UINVALID_UTF (-66)` for arbitrary junk |
| 183 | `pcre2_dfa_match_8` | code from a different code-unit width | `PCRE2_ERROR_BADMODE (-32)` |
| 184 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` with a fresh/garbage workspace (`workspace[0] & ~1 != 0` or `workspace[1]` out of range) | `PCRE2_ERROR_DFA_BADRESTART (-38)` |
| 185 | `pcre2_dfa_match_8` | `pcre2_set_offset_limit_8(mc, 1)` without `PCRE2_USE_OFFSET_LIMIT` at compile time | `PCRE2_ERROR_BADOFFSETLIMIT (-56)` |
| 186 | `pcre2_dfa_match_8` | pattern `\C` compiled with `PCRE2_UTF` (`OP_ANYBYTE`) | `PCRE2_ERROR_DFA_UITEM (-42)` |
| 187 | `pcre2_dfa_match_8` | pattern `a\Kb`, or `(a)\1`, or `(a)\1+`, or `(a)(*scs:(1)a)` — opcode unsupported by the DFA | `PCRE2_ERROR_DFA_UITEM (-42)` |
| 188 | `pcre2_dfa_match_8` | pattern `(a)(?(1)b\|c)` — backreference condition | `PCRE2_ERROR_DFA_UCOND (-40)` |
| 189 | `pcre2_dfa_match_8` | pattern `(?1)((?1))` (or `(a?(?1)?)` with a large workspace) on `"aaa"` | `PCRE2_ERROR_RECURSELOOP (-52)` |
| 190 | `pcre2_dfa_match_8` | recursion returns 0 matches because the internal ovector is exhausted (`pcre2_dfa_match.c:2995`) | `PCRE2_ERROR_DFA_RECURSE (-39)` |
| 191 | `pcre2_dfa_match_8` | pattern `(?:a\|b\|c\|d\|e\|f\|g\|h){1,400}` on a 480-char subject with `wscount = 22` — active/new state vectors overflow | `PCRE2_ERROR_DFA_WSSIZE (-43)` |
| 192 | `pcre2_dfa_match_8` | `pcre2_set_heap_limit_8(mc, 1)` + pattern `(a(?1)?)` on 100 `a`s — recursion workspace cannot grow within the heap limit | `PCRE2_ERROR_HEAPLIMIT (-63)` |
| 193 | `pcre2_dfa_match_8` | allocator failure in `more_workspace` / subject copy (`pcre2_dfa_match.c:447`, `:4068`) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 194 | `pcre2_dfa_match_8` | `pcre2_set_match_limit_8(mc, 1)` + pattern `(a(?1)?)` on `"aaa"` | `PCRE2_ERROR_MATCHLIMIT (-47)` |
| 195 | `pcre2_dfa_match_8` | `pcre2_set_depth_limit_8(mc, 0)` + pattern `(a(?1)?)` on `"aaa"` | `PCRE2_ERROR_DEPTHLIMIT (-53)` |
| 196 | `pcre2_dfa_match_8` | `PCRE2_UTF` pattern, subject byte `0x80` | `PCRE2_ERROR_UTF8_ERR20 (-22)` |
| 197 | `pcre2_dfa_match_8` | `PCRE2_UTF` pattern, subject `"\xc3\xa9"`, `start_offset = 1` | `PCRE2_ERROR_BADUTFOFFSET (-36)` |
| 198 | `pcre2_dfa_match_8` | pattern `zzz`, subject `aaa` | `PCRE2_ERROR_NOMATCH (-1)` |
| 199 | `pcre2_dfa_match_8` | pattern `abc`, subject `ab`, `PCRE2_PARTIAL_SOFT` | `PCRE2_ERROR_PARTIAL (-2)` |
| 200 | `pcre2_dfa_match_8` | corrupted pattern reaching `internal_dfa_match` sanity check (`pcre2_dfa_match.c:3576`) | `PCRE2_ERROR_INTERNAL (-44)` **(internal)** |

## E. `pcre2_substitute_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 201 | `pcre2_substitute_8` | `options` containing a bit that is neither a `SUBSTITUTE_OPTIONS` bit nor a `PUBLIC_MATCH_OPTIONS` bit, e.g. `PCRE2_UTF (0x00080000)` | `PCRE2_ERROR_BADOPTION (-34)` (forwarded from `pcre2_match`) |
| 202 | `pcre2_substitute_8` | `PCRE2_PARTIAL_SOFT` (or `PARTIAL_HARD`) without `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` | `PCRE2_ERROR_BADOPTION (-34)` |
| 203 | `pcre2_substitute_8` | `replacement == NULL` with `rlength = 1` | `PCRE2_ERROR_NULL (-51)` |
| 204 | `pcre2_substitute_8` | `subject == NULL` with `length = 1` | `PCRE2_ERROR_NULL (-51)` |
| 205 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with `match_data == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 206 | `pcre2_substitute_8` | `start_offset = 9` with `length = 3` | `PCRE2_ERROR_BADOFFSET (-33)` |
| 207 | `pcre2_substitute_8` | output buffer of 3 units, `PCRE2_SUBSTITUTE_GLOBAL`, pattern `a`, subject `aaaa`, replacement `XXXXXXXXXX` | `PCRE2_ERROR_NOMEMORY (-48)`; with `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` also `-48` but `*blength` = required size |
| 208 | `pcre2_substitute_8` | replacement `$` (a `$` at the very end / `$` before an illegal char) | `PCRE2_ERROR_BADREPLACEMENT (-35)` |
| 209 | `pcre2_substitute_8` | replacement `${1` — unterminated `${…}` | `PCRE2_ERROR_REPMISSINGBRACE (-58)` |
| 210 | `pcre2_substitute_8` | replacement `${1:-x}` **without** `PCRE2_SUBSTITUTE_EXTENDED` | `PCRE2_ERROR_REPMISSINGBRACE (-58)` |
| 211 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` + replacement `\q` — bad escape in replacement | `PCRE2_ERROR_BADREPESCAPE (-57)` |
| 212 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` + replacement `${1:?x}` — unknown `${name:…}` operator | `PCRE2_ERROR_BADSUBSTITUTION (-59)` |
| 213 | `pcre2_substitute_8` | replacement `${9}` or `$9` when the pattern has 1 group | `PCRE2_ERROR_NOSUBSTRING (-49)` |
| 214 | `pcre2_substitute_8` | replacement `${zz}` — unknown group name | `PCRE2_ERROR_NOSUBSTRING (-49)`; with `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` → `PCRE2_ERROR_UNSET (-55)` |
| 215 | `pcre2_substitute_8` | pattern `(a)\|(b)` on `"a"`, replacement `[$2]` — referenced group unset | `PCRE2_ERROR_UNSET (-55)`; with `PCRE2_SUBSTITUTE_UNSET_EMPTY` → success |
| 216 | `pcre2_substitute_8` | replacement `$2`, `match_data` created with `oveccount = 1` for pattern `(a)(b)` | `PCRE2_ERROR_UNAVAILABLE (-54)` |
| 217 | `pcre2_substitute_8` | replacement `$+` with a pattern that has **no** capture groups | `PCRE2_ERROR_NOSUBSTRING (-49)` |
| 218 | `pcre2_substitute_8` | replacement `$+`, pattern `(a)(b)`, `match_data` with `oveccount = 2` (< `top_bracket+1`) | `PCRE2_ERROR_UNAVAILABLE (-54)` |
| 219 | `pcre2_substitute_8` | replacement `$+`, pattern `a(b)?` on `"a"` — all groups unset | `PCRE2_ERROR_UNSET (-55)` |
| 220 | `pcre2_substitute_8` | pattern `(?=a\K)` compiled with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`, subject `a` — `ovector[1] < ovector[0]` | `PCRE2_ERROR_BADSUBSPATTERN (-60)` |
| 221 | `pcre2_substitute_8` | `PCRE2_PARTIAL_SOFT\|PCRE2_SUBSTITUTE_REPLACEMENT_ONLY`, pattern `abc`, subject `ab` | `PCRE2_ERROR_PARTIAL (-2)` |
| 222 | `pcre2_substitute_8` | `PCRE2_PARTIAL_SOFT\|PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` + replacement `$'` (or `$_`) with a *successful* match | `PCRE2_ERROR_PARTIALSUBS (-76)` |
| 223 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` + `code` different from `match_data->code` | `PCRE2_ERROR_DIFFSUBSPATTERN (-71)` |
| 224 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` + a different subject pointer/length than the prior `pcre2_match_8` call | `PCRE2_ERROR_DIFFSUBSSUBJECT (-72)` |
| 225 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` + `start_offset` different from the prior match | `PCRE2_ERROR_DIFFSUBSOFFSET (-73)` |
| 226 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED\|PCRE2_ANCHORED` when the prior match had no `PCRE2_ANCHORED` | `PCRE2_ERROR_DIFFSUBSOPTIONS (-74)` |
| 227 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a `match_data` filled by `pcre2_dfa_match_8` | `PCRE2_ERROR_DFA_UFUNC (-41)` |
| 228 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a `match_data` whose stored `rc` is an error (e.g. `-33` from a previous bad-offset match) | that stored error is returned verbatim, e.g. `PCRE2_ERROR_BADOFFSET (-33)` |
| 229 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a `match_data` holding `PCRE2_ERROR_NOMATCH` | returns `0` (no substitutions), not an error |
| 230 | `pcre2_substitute_8` | `pcre2_set_substitute_case_callout_8` callback returning `PCRE2_UNSET`, replacement `\Ux\E` with `PCRE2_SUBSTITUTE_EXTENDED` | `PCRE2_ERROR_REPLACECASE (-69)` |
| 231 | `pcre2_substitute_8` | replacement expansion whose total length overflows `PCRE2_SIZE` (`pcre2_substitute.c:1780`) | `PCRE2_ERROR_TOOLARGEREPLACE (-70)` **(internal / needs ~SIZE_MAX bytes)** |
| 232 | `pcre2_substitute_8` | more than `INT_MAX` substitutions with `PCRE2_SUBSTITUTE_GLOBAL` (`pcre2_substitute.c:1034`) | `PCRE2_ERROR_TOOMANYREPLACE (-61)` **(impractical)** |
| 233 | `pcre2_substitute_8` | internal match block allocation failure (`pcre2_substitute.c:895,909,1750,1772`) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 234 | `pcre2_substitute_8` | two consecutive matches that do not advance (`pcre2_substitute.c:1021`) | `PCRE2_ERROR_INTERNAL_DUPMATCH (-65)` **(internal)** |
| 235 | `pcre2_substitute_8` | `PCRE2_UTF` pattern + malformed UTF-8 in the *replacement*, `PCRE2_NO_UTF_CHECK` unset | negative `PCRE2_ERROR_UTF8_ERRn` from `_pcre2_valid_utf_8` |
| 236 | `pcre2_substitute_8` | `code == NULL` | **undefined behaviour / SIGSEGV** — `code->overall_options` is read before any NULL check (`pcre2_substitute.c:758`) |
| 237 | `pcre2_substitute_8` | `blength == NULL` (`outlengthptr`) | **undefined behaviour / SIGSEGV** — `*blength` is read/written unconditionally (`pcre2_substitute.c:777-779`); *verified: the C library crashes* |

## F. `pcre2_substring_*`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 238 | `pcre2_substring_length_bynumber_8` | `stringnumber = 9` for a pattern with 3 groups | `PCRE2_ERROR_NOSUBSTRING (-49)` |
| 239 | `pcre2_substring_length_bynumber_8` | `stringnumber` within `top_bracket` but `>= match_data->oveccount` (e.g. group 1 with `oveccount = 1`) | `PCRE2_ERROR_UNAVAILABLE (-54)` |
| 240 | `pcre2_substring_length_bynumber_8` | group exists and is in range but `ovector[2n] == PCRE2_UNSET` (pattern `(a)(b)?` on `"a"`, group 2) | `PCRE2_ERROR_UNSET (-55)` |
| 241 | `pcre2_substring_length_bynumber_8` | called after `pcre2_match_8` returned `PCRE2_ERROR_NOMATCH` | `PCRE2_ERROR_NOMATCH (-1)` (the stored `match_data->rc` is returned) |
| 242 | `pcre2_substring_length_bynumber_8` | called with `stringnumber > 0` after `PCRE2_ERROR_PARTIAL` | `PCRE2_ERROR_PARTIAL (-2)` (`stringnumber == 0` succeeds) |
| 243 | `pcre2_substring_length_bynumber_8` | DFA-produced `match_data`, `stringnumber >= oveccount` | `PCRE2_ERROR_UNAVAILABLE (-54)` |
| 244 | `pcre2_substring_length_bynumber_8` | DFA-produced `match_data`, `count != 0 && stringnumber >= count` | `PCRE2_ERROR_UNSET (-55)` |
| 245 | `pcre2_substring_length_bynumber_8` | `ovector` offsets beyond `subject_length` (`pcre2_substring.c:347`) | `PCRE2_ERROR_INVALIDOFFSET (-67)` **(internal, marked unreachable)** |
| 246 | `pcre2_substring_copy_bynumber_8` | `*sizeptr = 1` but the substring is 1 unit long (needs size+1) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 247 | `pcre2_substring_copy_bynumber_8` | any error from `pcre2_substring_length_bynumber_8` (rows 238-245) | that error, propagated unchanged |
| 248 | `pcre2_substring_get_bynumber_8` | allocator returns `NULL` for the new buffer | `PCRE2_ERROR_NOMEMORY (-48)` |
| 249 | `pcre2_substring_copy_byname_8` / `get_byname` / `length_byname` | `match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER` (match done with `pcre2_dfa_match_8`) | `PCRE2_ERROR_DFA_UFUNC (-41)` |
| 250 | `pcre2_substring_length_byname_8` | `stringname = "zz"` not present in the name table | `PCRE2_ERROR_NOSUBSTRING (-49)` (from `nametable_scan`) |
| 251 | `pcre2_substring_length_byname_8` | name exists but no matching group is inside `oveccount` | `PCRE2_ERROR_UNAVAILABLE (-54)` |
| 252 | `pcre2_substring_length_byname_8` | name exists, group is in range but unset (pattern `(a)(b)?(?<n>c)?` on `"a"`, name `n`) | `PCRE2_ERROR_UNSET (-55)` |
| 253 | `pcre2_substring_nametable_scan_8` | `stringname` not found | `PCRE2_ERROR_NOSUBSTRING (-49)` |
| 254 | `pcre2_substring_nametable_scan_8` | `firstptr == NULL && lastptr == NULL`, name duplicated (`(?<n>a)\|(?<n>b)` with `PCRE2_DUPNAMES`) | `PCRE2_ERROR_NOUNIQUESUBSTRING (-50)` |
| 255 | `pcre2_substring_number_from_name_8` | name not present | `PCRE2_ERROR_NOSUBSTRING (-49)` |
| 256 | `pcre2_substring_number_from_name_8` | duplicated name with `PCRE2_DUPNAMES` | `PCRE2_ERROR_NOUNIQUESUBSTRING (-50)` |
| 257 | `pcre2_substring_list_get_8` | `match_data->rc < 0` (previous match failed) | that negative `rc` (e.g. `-1`) |
| 258 | `pcre2_substring_list_get_8` | allocator returns `NULL` | `PCRE2_ERROR_NOMEMORY (-48)` |
| 259 | `pcre2_substring_free_8` / `pcre2_substring_list_free_8` | argument `NULL` | no-op, returns void (boundary row) |

## G. `pcre2_serialize_*`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 260 | `pcre2_serialize_encode_8` | `codes == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 261 | `pcre2_serialize_encode_8` | `serialized_bytes == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 262 | `pcre2_serialize_encode_8` | `serialized_size == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 263 | `pcre2_serialize_encode_8` | `number_of_codes = 0` | `PCRE2_ERROR_BADDATA (-29)` |
| 264 | `pcre2_serialize_encode_8` | `number_of_codes = -1` (negative `int32_t` across FFI) | `PCRE2_ERROR_BADDATA (-29)` |
| 265 | `pcre2_serialize_encode_8` | `codes[1] == NULL` with `number_of_codes = 2` | `PCRE2_ERROR_NULL (-51)` |
| 266 | `pcre2_serialize_encode_8` | `codes[0]` pointing at non-PCRE2 memory | `PCRE2_ERROR_BADMAGIC (-31)` |
| 267 | `pcre2_serialize_encode_8` | two codes where one was compiled with `pcre2_set_character_tables_8(cc, pcre2_maketables_8(NULL))` and the other with the built-in tables | `PCRE2_ERROR_MIXEDTABLES (-30)` |
| 268 | `pcre2_serialize_encode_8` | `memctl->malloc` returns `NULL` | `PCRE2_ERROR_NOMEMORY (-48)` |
| 269 | `pcre2_serialize_decode_8` | `bytes == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 270 | `pcre2_serialize_decode_8` | `codes == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 271 | `pcre2_serialize_decode_8` | `number_of_codes = 0` (or negative) | `PCRE2_ERROR_BADDATA (-29)` |
| 272 | `pcre2_serialize_decode_8` | stream with `data->number_of_codes` overwritten with 0 (bytes 12-15 zeroed) | `PCRE2_ERROR_BADSERIALIZEDDATA (-62)` |
| 273 | `pcre2_serialize_decode_8` | stream with byte 0 flipped — `data->magic != SERIALIZED_DATA_MAGIC` | `PCRE2_ERROR_BADMAGIC (-31)` |
| 274 | `pcre2_serialize_decode_8` | stream with byte 4 flipped — `data->version != SERIALIZED_DATA_VERSION` | `PCRE2_ERROR_BADMODE (-32)` |
| 275 | `pcre2_serialize_decode_8` | stream with byte 8 flipped — `data->config != SERIALIZED_DATA_CONFIG` (width/link-size mismatch) | `PCRE2_ERROR_BADMODE (-32)` |
| 276 | `pcre2_serialize_decode_8` | a per-code `blocksize <= sizeof(pcre2_real_code)` inside the stream | `PCRE2_ERROR_BADSERIALIZEDDATA (-62)` |
| 277 | `pcre2_serialize_decode_8` | decoded code with `magic_number != MAGIC_NUMBER`, or `name_entry_size > MAX_NAME_SIZE+IMM2_SIZE+1`, or `name_count > MAX_NAME_COUNT (10000)` | `PCRE2_ERROR_BADSERIALIZEDDATA (-62)` |
| 278 | `pcre2_serialize_decode_8` | tables or code allocation fails | `PCRE2_ERROR_NOMEMORY (-48)` |
| 279 | `pcre2_serialize_get_number_of_codes_8` | `bytes == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 280 | `pcre2_serialize_get_number_of_codes_8` | corrupted magic | `PCRE2_ERROR_BADMAGIC (-31)` |
| 281 | `pcre2_serialize_get_number_of_codes_8` | corrupted version or config | `PCRE2_ERROR_BADMODE (-32)` |
| 282 | `pcre2_serialize_free_8` | `bytes == NULL` | no-op (boundary row) |

## H. Contexts and `pcre2_set_*`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 283 | `pcre2_set_bsr_8` | `value = 0` (or `3`, or `0xffffffff`) — not `PCRE2_BSR_UNICODE(1)`/`PCRE2_BSR_ANYCRLF(2)` | `PCRE2_ERROR_BADDATA (-29)` |
| 284 | `pcre2_set_newline_8` | `newline = 0` (or `7`, or `0x7fffffff`) — outside 1…6 | `PCRE2_ERROR_BADDATA (-29)` |
| 285 | `pcre2_set_optimize_8` | `ccontext == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 286 | `pcre2_set_optimize_8` | `directive = 2` (between `PCRE2_OPTIMIZATION_FULL(1)` and `PCRE2_AUTO_POSSESS(64)`) | `PCRE2_ERROR_BADOPTION (-34)` |
| 287 | `pcre2_set_optimize_8` | `directive = 63` (just below the valid 64…69 range) | `PCRE2_ERROR_BADOPTION (-34)` |
| 288 | `pcre2_set_optimize_8` | `directive = 70` (just above `PCRE2_START_OPTIMIZE_OFF(69)`) | `PCRE2_ERROR_BADOPTION (-34)` |
| 289 | `pcre2_set_glob_separator_8` | `separator = 0x41 ('A')` — not `/`, `\` or `.` | `PCRE2_ERROR_BADDATA (-29)` |
| 290 | `pcre2_set_glob_escape_8` | `escape = 0x41 ('A')` — non-zero and not in the glob punctuation set | `PCRE2_ERROR_BADDATA (-29)` |
| 291 | `pcre2_set_glob_escape_8` | `escape = 256` (> 255) | `PCRE2_ERROR_BADDATA (-29)` |
| 292 | `pcre2_set_glob_escape_8` | `escape = 0` | `0` (explicitly allowed — "no escape"; boundary row) |
| 293 | `pcre2_general_context_create_8` | `private_malloc` returns `NULL` | `NULL` |
| 294 | `pcre2_compile_context_create_8` / `match_context_create_8` / `convert_context_create_8` | allocation failure via the supplied general context | `NULL` |
| 295 | `pcre2_general_context_copy_8` / `compile_context_copy_8` / `match_context_copy_8` / `convert_context_copy_8` | allocation failure | `NULL` |
| 296 | `pcre2_general_context_copy_8` (and the other `*_copy_8`) | argument `NULL` | **undefined behaviour / SIGSEGV** — `gcontext->memctl.malloc` is dereferenced with no NULL check (`pcre2_context.c:381`) |
| 297 | `pcre2_set_character_tables_8` / `set_max_pattern_length_8` / `set_max_pattern_compiled_length_8` / `set_max_varlookbehind_8` / `set_parens_nest_limit_8` / `set_compile_extra_options_8` / `set_compile_recursion_guard_8` / `set_callout_8` / `set_substitute_callout_8` / `set_substitute_case_callout_8` / `set_heap_limit_8` / `set_match_limit_8` / `set_depth_limit_8` / `set_offset_limit_8` / `set_recursion_limit_8` / `set_recursion_memory_management_8` | any value, including `0` and `0xffffffff` | always `0` — **no validation at all**; a `NULL` context is dereferenced (UB) |
| 298 | `pcre2_*_context_free_8` | argument `NULL` | no-op (boundary row) |

## I. `pcre2_config_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 299 | `pcre2_config_8` | `what = 0x7fffffff`, `where != NULL` — unknown selector hits the second `switch` default | `PCRE2_ERROR_BADOPTION (-34)` |
| 300 | `pcre2_config_8` | `what = 0x7fffffff`, `where == NULL` — unknown selector hits the length-request `switch` default | `PCRE2_ERROR_BADOPTION (-34)` |
| 301 | `pcre2_config_8` | `what = 17` (one past `PCRE2_CONFIG_EFFECTIVE_LINKSIZE = 16`) | `PCRE2_ERROR_BADOPTION (-34)` |
| 302 | `pcre2_config_8` | `what = PCRE2_CONFIG_JITTARGET (2)` in this **non-JIT** build (either `where` value) | `PCRE2_ERROR_BADOPTION (-34)` |
| 303 | `pcre2_config_8` | `what` valid but `where` pointing to fewer than `sizeof(uint32_t)` writable bytes | **undefined behaviour** — the size must be obtained with `where == NULL` first |

## J. `pcre2_pattern_info_8` and `pcre2_callout_enumerate_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 304 | `pcre2_pattern_info_8` | `code == NULL` (with `where != NULL`) | `PCRE2_ERROR_NULL (-51)` |
| 305 | `pcre2_pattern_info_8` | `what = 0x7fffffff` (or `27`, one past `PCRE2_INFO_EXTRAOPTIONS = 26`), `where != NULL` | `PCRE2_ERROR_BADOPTION (-34)` |
| 306 | `pcre2_pattern_info_8` | `what = 0x7fffffff`, `where == NULL` (length request) | `PCRE2_ERROR_BADOPTION (-34)` |
| 307 | `pcre2_pattern_info_8` | `code` pointing at non-PCRE2 memory | `PCRE2_ERROR_BADMAGIC (-31)` |
| 308 | `pcre2_pattern_info_8` | code compiled for a different code-unit width | `PCRE2_ERROR_BADMODE (-32)` |
| 309 | `pcre2_pattern_info_8` | `what = PCRE2_INFO_DEPTHLIMIT (21)` on a pattern with no `(*LIMIT_DEPTH=…)` | `PCRE2_ERROR_UNSET (-55)` (the value `UINT32_MAX` **is still written** to `*where`) |
| 310 | `pcre2_pattern_info_8` | `what = PCRE2_INFO_HEAPLIMIT (25)` with no `(*LIMIT_HEAP=…)` | `PCRE2_ERROR_UNSET (-55)` |
| 311 | `pcre2_pattern_info_8` | `what = PCRE2_INFO_MATCHLIMIT (14)` with no `(*LIMIT_MATCH=…)` | `PCRE2_ERROR_UNSET (-55)` |
| 312 | `pcre2_pattern_info_8` | `code == NULL` **and** `where == NULL` | returns the field size (`4` for `PCRE2_INFO_ALLOPTIONS`) — the NULL check happens *after* the length branch |
| 313 | `pcre2_callout_enumerate_8` | `code == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 314 | `pcre2_callout_enumerate_8` | `code` pointing at non-PCRE2 memory | `PCRE2_ERROR_BADMAGIC (-31)` |
| 315 | `pcre2_callout_enumerate_8` | code from a different code-unit width | `PCRE2_ERROR_BADMODE (-32)` |
| 316 | `pcre2_callout_enumerate_8` | callback returning a non-zero value | that value is returned verbatim |
| 317 | `pcre2_callout_enumerate_8` | `callback == NULL` on a pattern that contains a callout | **undefined behaviour / SIGSEGV** — no NULL check on `callback` |

## K. `pcre2_pattern_convert_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 318 | `pcre2_pattern_convert_8` | `pattern == NULL` with `plength = 1` | `PCRE2_ERROR_NULL (-51)`, `*bufflenptr = 0` |
| 319 | `pcre2_pattern_convert_8` | `bufflenptr == NULL` | `PCRE2_ERROR_NULL (-51)` |
| 320 | `pcre2_pattern_convert_8` | `options = 0` — no `PCRE2_CONVERT_*` type bit set | `PCRE2_ERROR_BADOPTION (-34)`, `*bufflenptr = 0` |
| 321 | `pcre2_pattern_convert_8` | `options = PCRE2_CONVERT_GLOB\|PCRE2_CONVERT_POSIX_BASIC` — more than one type | `PCRE2_ERROR_BADOPTION (-34)` |
| 322 | `pcre2_pattern_convert_8` | `options = PCRE2_CONVERT_GLOB\|0x8000` — undefined bit set | `PCRE2_ERROR_BADOPTION (-34)` |
| 323 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_EXTENDED` + pattern `[a` (unterminated bracket); also glob `[a`, `[!]`, `[]` | `PCRE2_ERROR_MISSING_SQUARE_BRACKET (=106)` — **positive** compile-error code |
| 324 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC` or `POSIX_EXTENDED` + pattern `a\` | `PCRE2_ERROR_END_BACKSLASH (=101)` — **positive** code |
| 325 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` + pattern `[b-a]` (or `[z-a]`, `[!b-a]`) — glob class range out of order | `PCRE2_ERROR_CONVERT_SYNTAX (-64)`, `*bufflenptr` = error offset |
| 326 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` + pattern `[a-[:alpha:]]` — POSIX class as a range endpoint | `PCRE2_ERROR_CONVERT_SYNTAX (-64)` |
| 327 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB\|PCRE2_CONVERT_UTF` with `glob_separator >= 128` or `glob_escape >= 128` | `PCRE2_ERROR_CONVERT_SYNTAX (-64)` (currently only reachable if the setters are bypassed) |
| 328 | `pcre2_pattern_convert_8` | caller-supplied buffer of 2 units for a 10-char glob (`*buffptr != NULL`, `*bufflenptr` too small) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 329 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` + malformed UTF-8 pattern (`"\x80"`), `PCRE2_CONVERT_NO_UTF_CHECK` unset | negative `PCRE2_ERROR_UTF8_ERRn` (here `-22`); `*bufflenptr` = error offset |
| 330 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` in a build without `SUPPORT_UNICODE` | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED (=132)` — **unreachable in this build** |
| 331 | `pcre2_pattern_convert_8` | output buffer allocation failure on the second pass | `PCRE2_ERROR_NOMEMORY (-48)`, `*bufflenptr = 0` |
| 332 | `pcre2_pattern_convert_8` | `pattype` reaching the converter `switch` default, or the two-pass loop falling through | `PCRE2_ERROR_INTERNAL (-44)` **(internal, `PCRE2_DEBUG_UNREACHABLE`)** |
| 333 | `pcre2_converted_pattern_free_8` | argument `NULL` | no-op (boundary row) |

## L. `pcre2_match_data_*`, `pcre2_code_copy*`, `pcre2_maketables*`, `pcre2_jit_*`, `pcre2_next_match_8`, `pcre2_get_error_message_8`

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 334 | `pcre2_match_data_create_8` | `oveccount = 0` | **clamped up to 1** — never fails on this input (`pcre2_match_data.c:55`) |
| 335 | `pcre2_match_data_create_8` | `oveccount = 0xffffffff` | **clamped down to `UINT16_MAX` (65535)** |
| 336 | `pcre2_match_data_create_8` | allocator returns `NULL` | `NULL` |
| 337 | `pcre2_match_data_create_from_pattern_8` | `code == NULL` | `NULL` |
| 338 | `pcre2_match_data_free_8` | argument `NULL` | no-op (boundary row) |
| 339 | `pcre2_get_mark_8` / `get_ovector_pointer_8` / `get_ovector_count_8` / `get_startchar_8` / `get_match_data_size_8` / `get_match_data_heapframes_size_8` | `match_data == NULL` | **undefined behaviour / SIGSEGV** — no NULL checks and no error return channel |
| 340 | `pcre2_get_mark_8` | `match_data` from a match that did not set a mark | returns `NULL` (valid result, not an error) |
| 341 | `pcre2_code_copy_8` | `code == NULL` | `NULL` |
| 342 | `pcre2_code_copy_8` | `code->memctl.malloc` returns `NULL` | `NULL` |
| 343 | `pcre2_code_copy_with_tables_8` | `code == NULL` | `NULL` |
| 344 | `pcre2_code_copy_with_tables_8` | code block allocation, or the second (tables) allocation, fails | `NULL` (the first block is freed on the second failure) |
| 345 | `pcre2_code_free_8` | argument `NULL` | no-op (boundary row) |
| 346 | `pcre2_maketables_8` | allocator returns `NULL` | `NULL` |
| 347 | `pcre2_maketables_free_8` | `tables == NULL` | no-op (boundary row) |
| 348 | `pcre2_jit_compile_8` | `code == NULL` with `options` NOT containing `PCRE2_JIT_TEST_ALLOC` (the `TEST_ALLOC` branch at `pcre2_jit_compile.c:14316` runs BEFORE the NULL check, so `jit_compile(NULL, 0x200)` gives `-68` and `jit_compile(NULL, 0x201)` gives `-45`) | `PCRE2_ERROR_NULL (-51)` |
| 349 | `pcre2_jit_compile_8` | `options = 0x8000` — bit outside `PUBLIC_JIT_COMPILE_OPTIONS` | `PCRE2_ERROR_JIT_BADOPTION (-45)` |
| 350 | `pcre2_jit_compile_8` | `options = PCRE2_JIT_COMPLETE (1)` in this **non-JIT** build | `PCRE2_ERROR_JIT_BADOPTION (-45)` |
| 351 | `pcre2_jit_compile_8` | `options = PCRE2_JIT_TEST_ALLOC (0x200)` alone, non-JIT build | `PCRE2_ERROR_JIT_UNSUPPORTED (-68)` |
| 352 | `pcre2_jit_compile_8` | `options = PCRE2_JIT_TEST_ALLOC\|PCRE2_JIT_COMPLETE (0x201)` — `TEST_ALLOC` must be used alone | `PCRE2_ERROR_JIT_BADOPTION (-45)` |
| 353 | `pcre2_jit_compile_8` | `PCRE2_JIT_INVALID_UTF` after a previous successful non-invalid-UTF JIT compile (JIT builds only) | `PCRE2_ERROR_JIT_BADOPTION (-45)` |
| 354 | `pcre2_jit_match_8` | any arguments in this **non-JIT** build | `PCRE2_ERROR_JIT_BADOPTION (-45)`, also stored in `match_data->rc` |
| 355 | `pcre2_jit_match_8` | JIT stack limit reached (JIT builds only) | `PCRE2_ERROR_JIT_STACKLIMIT (-46)` — **unreachable in this build** |
| 356 | `pcre2_jit_stack_create_8` | `startsize = 0` **or** `maxsize = 0` **or** `maxsize > SIZE_MAX - STACK_GROWTH_RATE` | `NULL` |
| 357 | `pcre2_jit_stack_create_8` | any valid sizes in this **non-JIT** build (e.g. `1, 1`) | `NULL` (unconditional) |
| 358 | `pcre2_jit_stack_free_8` / `pcre2_jit_stack_assign_8` / `pcre2_jit_free_unused_memory_8` | `NULL` arguments | no-op (boundary rows) |
| 359 | `pcre2_next_match_8` | `match_data->rc < 0` (last match failed or errored) | returns `FALSE (0)`; `*pstart_offset` / `*poptions` untouched |
| 360 | `pcre2_next_match_8` | last match was empty and at the end of the subject | returns `FALSE (0)` |
| 361 | `pcre2_next_match_8` | `match_data == NULL`, or `pstart_offset`/`poptions == NULL` while `TRUE` is about to be returned | **undefined behaviour / SIGSEGV** — no NULL checks |
| 362 | `pcre2_get_error_message_8` | `size = 0` | `PCRE2_ERROR_NOMEMORY (-48)` |
| 363 | `pcre2_get_error_message_8` | buffer smaller than the message (e.g. `size = 2` for error `-1`) | `PCRE2_ERROR_NOMEMORY (-48)` |
| 364 | `pcre2_get_error_message_8` | `enumber = 9999` (above the compile-error table) | `PCRE2_ERROR_BADDATA (-29)` |
| 365 | `pcre2_get_error_message_8` | `enumber = -9999` (below the match-error table) | `PCRE2_ERROR_BADDATA (-29)` |
| 366 | `pcre2_get_error_message_8` | `enumber = 0` ("no error" slot is deliberately not addressable) | `PCRE2_ERROR_BADDATA (-29)` |
| 367 | `pcre2_get_error_message_8` | `buffer == NULL` with `size != 0` | **undefined behaviour / SIGSEGV** — no NULL check |

## M. `_pcre2_valid_utf_8` (UTF-8 validity, surfaced by `pcre2_compile_8` with `PCRE2_UTF`, `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_substitute_8`, `pcre2_pattern_convert_8` with `PCRE2_CONVERT_UTF`)

All rows: the negative code is returned by the enclosing public function and
`*erroroffset` / `match_data->startchar` / `*bufflenptr` is set to the offset of
the offending code unit. Only reachable when `PCRE2_NO_UTF_CHECK` is **not**
set. In this 8-bit build, `PCRE2_ERROR_UTF16_ERR1..3 (-24..-26)` and
`PCRE2_ERROR_UTF32_ERR1..2 (-27..-28)` are compiled but unreachable.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 368 | `_pcre2_valid_utf_8` | subject/pattern ends after a lead byte needing 1 more byte, e.g. `"\xc3"` | `PCRE2_ERROR_UTF8_ERR1 (-3)` "1 byte missing at end" |
| 369 | `_pcre2_valid_utf_8` | 2 or more continuation bytes missing from a 3-byte form, e.g. `"\xe2"` — NOTE: `"\xe2\x82"` (only ONE byte short) gives `UTF8_ERR1`, because `pcre2_valid_utf.c:154-166` selects the code from `ab - length` | `PCRE2_ERROR_UTF8_ERR2 (-4)` |
| 370 | `_pcre2_valid_utf_8` | 2 or more continuation bytes missing from a 4-byte form, e.g. `"\xf0"` — `"\xf0\x9f\x98"` (one byte short) gives `UTF8_ERR1` | `PCRE2_ERROR_UTF8_ERR3 (-5)` |
| 371 | `_pcre2_valid_utf_8` | 2 or more continuation bytes missing from a 5-byte form, e.g. `"\xf8"` — `"\xf8\x88\x80\x80"` (one byte short) gives `UTF8_ERR1` | `PCRE2_ERROR_UTF8_ERR4 (-6)` |
| 372 | `_pcre2_valid_utf_8` | 2 or more continuation bytes missing from a 6-byte form, e.g. `"\xfc"` — `"\xfc\x84\x80\x80\x80"` (one byte short) gives `UTF8_ERR1` | `PCRE2_ERROR_UTF8_ERR5 (-7)` |
| 373 | `_pcre2_valid_utf_8` | byte 2 of a multi-byte sequence has top bits ≠ `0x80`, e.g. `"\xc3\x41"` | `PCRE2_ERROR_UTF8_ERR6 (-8)` |
| 374 | `_pcre2_valid_utf_8` | byte 3 top bits ≠ `0x80`, e.g. `"\xe2\x82\x41"` | `PCRE2_ERROR_UTF8_ERR7 (-9)` |
| 375 | `_pcre2_valid_utf_8` | byte 4 top bits ≠ `0x80`, e.g. `"\xf0\x9f\x98\x41"` | `PCRE2_ERROR_UTF8_ERR8 (-10)` |
| 376 | `_pcre2_valid_utf_8` | byte 5 top bits ≠ `0x80`, e.g. `"\xf8\x88\x80\x80\x41"` | `PCRE2_ERROR_UTF8_ERR9 (-11)` |
| 377 | `_pcre2_valid_utf_8` | byte 6 top bits ≠ `0x80`, e.g. `"\xfc\x84\x80\x80\x80\x41"` | `PCRE2_ERROR_UTF8_ERR10 (-12)` |
| 378 | `_pcre2_valid_utf_8` | complete 5-byte sequence (banned by RFC 3629), e.g. `"\xf8\x88\x80\x80\x80"` | `PCRE2_ERROR_UTF8_ERR11 (-13)` |
| 379 | `_pcre2_valid_utf_8` | complete 6-byte sequence, e.g. `"\xfc\x84\x80\x80\x80\x80"` | `PCRE2_ERROR_UTF8_ERR12 (-14)` |
| 380 | `_pcre2_valid_utf_8` | 4-byte sequence encoding > `0x10FFFF`, e.g. `"\xf4\x90\x80\x80"` | `PCRE2_ERROR_UTF8_ERR13 (-15)` |
| 381 | `_pcre2_valid_utf_8` | encoded surrogate `0xD800`-`0xDFFF`, e.g. `"\xed\xa0\x80"` | `PCRE2_ERROR_UTF8_ERR14 (-16)` |
| 382 | `_pcre2_valid_utf_8` | overlong 2-byte form, e.g. `"\xc0\x80"` or `"\xc1\xbf"` | `PCRE2_ERROR_UTF8_ERR15 (-17)` |
| 383 | `_pcre2_valid_utf_8` | overlong 3-byte form, e.g. `"\xe0\x80\x80"` | `PCRE2_ERROR_UTF8_ERR16 (-18)` |
| 384 | `_pcre2_valid_utf_8` | overlong 4-byte form, e.g. `"\xf0\x80\x80\x80"` | `PCRE2_ERROR_UTF8_ERR17 (-19)` |
| 385 | `_pcre2_valid_utf_8` | overlong 5-byte form, e.g. `"\xf8\x80\x80\x80\x80"` | `PCRE2_ERROR_UTF8_ERR18 (-20)` |
| 386 | `_pcre2_valid_utf_8` | overlong 6-byte form, e.g. `"\xfc\x80\x80\x80\x80\x80"` | `PCRE2_ERROR_UTF8_ERR19 (-21)` |
| 387 | `_pcre2_valid_utf_8` | isolated continuation byte, e.g. `"\x80"` or `"\xbf"` | `PCRE2_ERROR_UTF8_ERR20 (-22)` — also returned directly by `pcre2_match_8` when the *first* code unit at `start_offset == 0` is not a character start |
| 388 | `_pcre2_valid_utf_8` | byte `0xfe` or `0xff` | `PCRE2_ERROR_UTF8_ERR21 (-23)` |
| 389 | `pcre2_match_8` / `pcre2_dfa_match_8` | `start_offset > 0` pointing at a continuation byte (`NOT_FIRSTCU`), e.g. subject `"\xc3\xa9"` with `start_offset = 1` | `PCRE2_ERROR_BADUTFOFFSET (-36)` (checked *before* `valid_utf`) |

## N. Generic FFI-boundary rows (every public entry point)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 390 | all `pcre2_*` taking `options`/`what`/`value` `uint32_t` | pass `0xffffffff` — C accepts any `int`/`uint32_t` for what are logically enums | `PCRE2_ERROR_BADOPTION (-34)` for `config`/`pattern_info`/`match`/`dfa_match`/`substitute`/`convert`/`set_optimize`; `PCRE2_ERROR_BADDATA (-29)` for `set_bsr`/`set_newline`/`set_glob_*`; `ERR17` for `pcre2_compile_8` |
| 391 | all `pcre2_*` taking `options` | pass a *valid-for-another-function* bit, e.g. `PCRE2_SUBSTITUTE_GLOBAL` to `pcre2_match_8`, or `PCRE2_DFA_RESTART` to `pcre2_match_8`, or `PCRE2_NO_JIT` to `pcre2_dfa_match_8` | `PCRE2_ERROR_BADOPTION (-34)` |
| 392 | `pcre2_compile_8`, `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_substitute_8`, `pcre2_pattern_convert_8` | `length = PCRE2_ZERO_TERMINATED (SIZE_MAX)` on a buffer with **no** NUL terminator | **undefined behaviour** — `_pcre2_strlen_8` reads past the end |
| 393 | `pcre2_compile_8`, `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_substitute_8` | `length = SIZE_MAX - 1` (huge but not the sentinel) | **undefined behaviour** — no upper bound check besides `max_pattern_length` for compile |
| 394 | `pcre2_compile_8` | `patlen = 0` with a non-NULL pattern | success: matches the empty string (boundary row) |
| 395 | `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_substitute_8` | `start_offset == length` (exactly at the end) | legal; `NOMATCH`/match at end (boundary row, **not** `BADOFFSET`) |
| 396 | `pcre2_match_8`, `pcre2_dfa_match_8` | `start_offset = SIZE_MAX` | `PCRE2_ERROR_BADOFFSET (-33)` |
| 397 | `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_pattern_info_8`, `pcre2_callout_enumerate_8`, `pcre2_serialize_encode_8`, `pcre2_substitute_8` | `code` pointing at arbitrary (non-PCRE2) memory — these six check `re->magic_number` BEFORE any other field (`pcre2_pattern_info.c:112`, `pcre2_match.c`, `pcre2_dfa_match.c`, `pcre2_serialize.c`; `substitute` inherits it from the internal `pcre2_match` call) | `PCRE2_ERROR_BADMAGIC (-31)` |
| 397a | `pcre2_code_copy_8`, `pcre2_code_copy_with_tables_8`, `pcre2_substring_*_8` | `code` pointing at arbitrary memory — **no magic check at all**: `pcre2_code_copy` calls `code->memctl.malloc(code->blocksize, …)` straight away (`pcre2_compile.c`), and `pcre2_substring.c` contains no `BADMAGIC` test, reading `name_count`/`name_entry_size` directly | **undefined behaviour / SIGSEGV** — verified: the C `.so` crashes. (With an all-zero buffer the substring functions happen to return `PCRE2_ERROR_NOSUBSTRING (-49)` because `name_count` reads as 0, but that is incidental.) |
| 398 | `pcre2_match_8`, `pcre2_dfa_match_8`, `pcre2_pattern_info_8`, `pcre2_callout_enumerate_8` | `code` with valid magic but wrong `PCRE2_MODE_MASK` bits (cross-width code, or forged bytes) | `PCRE2_ERROR_BADMODE (-32)` |
| 399 | `pcre2_dfa_match_8` | `wscount` correct but `workspace` smaller than `wscount * sizeof(int)` | **undefined behaviour** — the size is trusted |
| 400 | `pcre2_serialize_encode_8`, `pcre2_serialize_decode_8` | `number_of_codes` larger than the actual array length | **undefined behaviour** (decode clamps to `data->number_of_codes`, encode does not) |
| 401 | `pcre2_substring_length_bynumber_8`, `copy_bynumber`, `get_bynumber` | `stringnumber = 0xffffffff` | `PCRE2_ERROR_NOSUBSTRING (-49)` (or `UNAVAILABLE (-54)` for DFA match data) |
| 402 | `pcre2_substring_*_byname_8`, `pcre2_substring_number_from_name_8`, `pcre2_substring_nametable_scan_8` | `stringname == NULL` | **undefined behaviour / SIGSEGV** — `_pcre2_strcmp_8` dereferences it |
| 403 | `pcre2_substring_copy_bynumber_8`, `copy_byname` | `sizeptr == NULL` | **undefined behaviour** — `*sizeptr` is read |
| 404 | `pcre2_substring_free_8`, `pcre2_substring_list_free_8`, `pcre2_serialize_free_8` | pointer that did not come from the matching `*_get_*`/`encode` call | **undefined behaviour** — a hidden `pcre2_memctl` immediately below the pointer is dereferenced |
| 405 | `pcre2_code_free_8`, `pcre2_match_data_free_8`, `pcre2_*_context_free_8`, `pcre2_jit_stack_free_8`, `pcre2_maketables_free_8`, `pcre2_converted_pattern_free_8` | `NULL` | all are explicit no-ops (boundary rows) |
| 406 | `pcre2_code_free_8` | double free of the same `pcre2_code` | **undefined behaviour** (no sentinel/poisoning) |
| 407 | `pcre2_match_8`, `pcre2_dfa_match_8` | `match_data` created from a *different* pattern (fewer ovector pairs than the pattern's groups) | legal; returns `0` when the ovector is too small (boundary row) |
| 408 | `pcre2_substitute_8` | `buffer == NULL` with `*blength != 0` | **undefined behaviour** — `buffer` is written without a NULL check (use `*blength = 0` + `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` for a length-only run) |
| 409 | `pcre2_general_context_create_8` | `private_malloc == NULL` or `private_free == NULL` | accepted; **undefined behaviour** at first allocation (no validation in `pcre2_context.c`) |
| 410 | `pcre2_set_character_tables_8` | `tables == NULL`, then compile | accepted (`return 0`); the default tables are **not** substituted → **undefined behaviour** during compile |
