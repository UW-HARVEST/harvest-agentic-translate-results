# ERROR-SURFACE TABLE — PCRE2 10.48, 8-bit, SUPPORT_UNICODE, no JIT

## Provenance

Every row below was derived **mechanically from the C sources** in `c_src/src/*.c`,
`c_src/src/*.h` and `c_src/include/pcre2.h` — by grepping and reading every
`return PCRE2_ERROR_*`, every `*errorcodeptr = ERR<n>` / `errorcode = ERR<n>`
assignment, every `return NULL` in a public constructor, every explicit
bounds/NULL/magic/mode check on public arguments, and every `PCRE2_ASSERT` on a
public input path. No documentation was consulted.

### Build configuration these rows assume (read out of `c_src/CMakeLists.txt` and `c_src/src/config.h`)

| macro | value | consequence |
|---|---|---|
| `PCRE2_CODE_UNIT_WIDTH` | `8` | public symbols carry the `_8` suffix; `MAX_NON_UTF_CHAR` = `0xff`; `MAX_MARK` = 255 |
| `SUPPORT_UNICODE` | **defined** (CMake) | `SUPPORT_WIDE_CHARS` defined; `ERR32`/`ERR45`/`ERR96` unreachable |
| `SUPPORT_JIT` | **undefined** | all `pcre2_jit_*` entry points are stubs |
| `PCRE2_DEBUG` | **undefined** | `PCRE2_ASSERT(x)` and `PCRE2_DEBUG_UNREACHABLE()` expand to **no-ops**; the `LCOV_EXCL`-marked "internal error" branches are the actual behaviour |
| `EBCDIC` | undefined | the `#else` EBCDIC arms of `\c` handling are not compiled |
| `NEVER_BACKSLASH_C` | undefined | `ERR85` unreachable |
| `HAVE_BUILTIN_MUL_OVERFLOW` | undefined | `_pcre2_ckd_smul_8` never reports overflow on a 64-bit host |
| `LINK_SIZE` | `2` | `MAX_PATTERN_SIZE` = `1 << 16` = **65536** code units |

### Compile-time error numbering

`pcre2_compile.c` sets an internal `ERR<n>` value; `pcre2_compile.h:53` defines
`enum { ERR0 = COMPILE_ERROR_BASE, ERR1, ... ERR120 }` and
`pcre2_internal.h:216` defines `COMPILE_ERROR_BASE 100`. Therefore
**`ERR<n>` == the public constant with numeric value `100 + n`** (e.g. `ERR4` ==
`PCRE2_ERROR_QUANTIFIER_OUT_OF_ORDER` == 104). On any compile failure
`pcre2_compile_8` returns `NULL`, writes the numeric code to `*errorptr`, and
writes a byte offset into the pattern to `*erroroffset`
(`pcre2_compile.c:11322`).

### Relevant limit constants

`MAX_GROUP_NUMBER` 65535 · `MAX_REPEAT_COUNT` 65535 · `REPEAT_UNLIMITED` 65536 ·
`MAX_NAME_SIZE` 128 · `MAX_NAME_COUNT` 10000 · `PARENS_NEST_LIMIT` 250 (default) ·
`MATCH_LIMIT` 10000000 · `MATCH_LIMIT_DEPTH` 10000000 · `HEAP_LIMIT` 20000000 ·
`MAX_VARLOOKBEHIND` 255 (default) · `LOOKBEHIND_MAX` 65535 ·
`ECLASS_NEST_LIMIT` 15 · `MAX_PATTERN_SIZE` 65536 · `MAX_MARK` 255 ·
`MAGIC_NUMBER` 0x50435245 · serialize magic 0x50523253.

---

### pcre2_compile.c — entry-point argument validation

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `pcre2_compile_8` | `errorptr == NULL` (any pattern). `*erroroffset` is set to 0 if `erroroffset != NULL`; no error code can be reported (pcre2_compile.c:10340) | `NULL`, no errorcode written |
| 2 | `pcre2_compile_8` | `erroroffset == NULL` with `errorptr != NULL` (pcre2_compile.c:10345) | `NULL`, `*errorptr = PCRE2_ERROR_NULL_ERROROFFSET` (220) |
| 3 | `pcre2_compile_8` | `pattern == NULL` with `patlen != 0` (e.g. `patlen = 1`, or `patlen = PCRE2_ZERO_TERMINATED`). Note `pattern==NULL && patlen==0` is legal (pcre2_compile.c:10355-10363) | `NULL`, `*errorptr = PCRE2_ERROR_NULL_PATTERN` (116), `*erroroffset = 0` |
| 4 | `pcre2_compile_8` | `options` contains a bit outside `PUBLIC_COMPILE_OPTIONS` (pcre2_compile.c:693-700), e.g. `options = 0x00000001` (unused low bit) or `0x80000000` (pcre2_compile.c:10377) | `NULL`, `*errorptr = PCRE2_ERROR_BAD_OPTIONS` (117), `*erroroffset = 0` |
| 5 | `pcre2_compile_8` | `ccontext->extra_options` (set via `pcre2_set_compile_extra_options_8`) contains a bit outside `PUBLIC_COMPILE_EXTRA_OPTIONS` (pcre2_compile.c:706-714), e.g. `0x80000000` (pcre2_compile.c:10378) | `NULL`, `*errorptr = PCRE2_ERROR_BAD_OPTIONS` (117), `*erroroffset = 0` |
| 6 | `pcre2_compile_8` | `PCRE2_LITERAL` set together with an option outside `PUBLIC_LITERAL_COMPILE_OPTIONS`, e.g. `PCRE2_LITERAL\|PCRE2_CASELESS` is legal but `PCRE2_LITERAL\|PCRE2_MULTILINE` is not (pcre2_compile.c:10384) | `NULL`, `*errorptr = PCRE2_ERROR_BAD_LITERAL_OPTIONS` (192), `*erroroffset = 0` |
| 7 | `pcre2_compile_8` | `PCRE2_LITERAL` set together with an extra option outside `PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS`, e.g. `PCRE2_LITERAL` + `PCRE2_EXTRA_ALT_BSUX` (pcre2_compile.c:10386) | `NULL`, `*errorptr = PCRE2_ERROR_BAD_LITERAL_OPTIONS` (192), `*erroroffset = 0` |
| 8 | `pcre2_compile_8` | `patlen > ccontext->max_pattern_length`, e.g. `pcre2_set_max_pattern_length_8(cc, 3)` then compile `"abcd"` (pcre2_compile.c:10399) | `NULL`, `*errorptr = PCRE2_ERROR_PATTERN_STRING_TOO_LONG` (188), `*erroroffset = 0` |
| 9 | `pcre2_compile_8` | Pattern-start verb `(*LIMIT_MATCH=`/`(*LIMIT_DEPTH=`/`(*LIMIT_HEAP=` with no digits or no closing `)`, e.g. `"(*LIMIT_MATCH=x)a"` or `"(*LIMIT_HEAP=99"` (pcre2_compile.c:10547-10552) | `NULL`, `*errorptr = PCRE2_ERROR_VERB_UNKNOWN` (160) |
| 10 | `pcre2_compile_8` | `PCRE2_UTF` (or `(*UTF)`) together with `PCRE2_NEVER_UTF` in `options`, e.g. `pcre2_compile_8("(*UTF)a", …, PCRE2_NEVER_UTF, …)` (pcre2_compile.c:10622) | `NULL`, `*errorptr = PCRE2_ERROR_UTF_IS_DISABLED` (174) |
| 11 | `pcre2_compile_8` | `PCRE2_UTF` set, `PCRE2_NO_UTF_CHECK` not set, and the **pattern** is not valid UTF-8, e.g. pattern bytes `0xFF` (pcre2_compile.c:10627, via `PRIV(valid_utf)`) | `NULL`, `*errorptr` = one of `PCRE2_ERROR_UTF8_ERR1..ERR21` (-3..-23), `*erroroffset` = offset of the bad code unit (set by `valid_utf`) |
| 12 | `pcre2_compile_8` | `PCRE2_UCP` (or `(*UCP)`) together with `PCRE2_NEVER_UCP`, e.g. `pcre2_compile_8("(*UCP)a", …, PCRE2_NEVER_UCP, …)` (pcre2_compile.c:10643) | `NULL`, `*errorptr = PCRE2_ERROR_UCP_IS_DISABLED` (175) |
| 13 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` with neither `PCRE2_UTF` nor `PCRE2_UCP` (pcre2_compile.c:10653) | `NULL`, `*errorptr = PCRE2_ERROR_EXTRA_CASING_REQUIRES_UNICODE` (204) |
| 14 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` with `PCRE2_UCP` but **without** `PCRE2_UTF` (8-bit-only check) (pcre2_compile.c:10660) | `NULL`, `*errorptr = PCRE2_ERROR_TURKISH_CASING_REQUIRES_UTF` (205) |
| 15 | `pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` together with `PCRE2_EXTRA_CASELESS_RESTRICT` (plus `PCRE2_UTF`) (pcre2_compile.c:10667) | `NULL`, `*errorptr = PCRE2_ERROR_EXTRA_CASING_INCOMPATIBLE` (206) |
| 16 | `pcre2_compile_8` | `re_blocksize > ccontext->max_pattern_compiled_length`, e.g. `pcre2_set_max_pattern_compiled_length_8(cc, 1)` then compile `"abc"` (pcre2_compile.c:10873) | `NULL`, `*errorptr = PCRE2_ERROR_PATTERN_COMPILED_SIZE_TOO_BIG` (201), `*erroroffset = 0` |
| 17 | `pcre2_compile_8` | Compiled form plus the cumulative `cb.char_lists_size` exceeds `MAX_PATTERN_SIZE` (65536 code units), e.g. ~9000 repetitions of `[\x{100}-\x{102}]` with `PCRE2_UTF`. NB **not** `"(?:aaaa…){65535}"`: a pattern whose `length` alone overflows is caught by the per-item check in `compile_branch` first (rows 212/213), so reaching *this* site needs the character-list contribution (pcre2_compile.c:10840-10849) | `NULL`, `*errorptr = PCRE2_ERROR_PATTERN_TOO_LARGE` (120), `*erroroffset = 0` |
| 18 | `pcre2_compile_8` | `pcre2_set_compile_recursion_guard_8(cc, cb, data)` whose `cb` returns non-zero; compile any pattern containing a group, e.g. `"(a)"` (pcre2_compile.c:8598-8603) | `NULL`, `*errorptr = PCRE2_ERROR_PARENTHESES_STACK_CHECK` (133), `*erroroffset = 0` |
| 19 | `pcre2_compile_8` | `ccontext->newline_convention` outside 1..6 — unreachable through the public API because `pcre2_set_newline_8` validates; only via a hand-built `pcre2_compile_context` (pcre2_compile.c:10714-10717) | `NULL`, `*errorptr = PCRE2_ERROR_INTERNAL_UNKNOWN_NEWLINE` (156) |
| 20 | `pcre2_code_copy_8` | `code == NULL` (pcre2_compile.c:1137) | `NULL` (no error code exists on this API) |
| 21 | `pcre2_code_copy_8` | `code->memctl.malloc(code->blocksize, …)` returns NULL — compile with a general context whose `private_malloc` fails (pcre2_compile.c:1139) | `NULL` |
| 22 | `pcre2_code_copy_with_tables_8` | `code == NULL` (pcre2_compile.c:1171) | `NULL` |
| 23 | `pcre2_code_copy_with_tables_8` | allocation of `code->blocksize + TABLES_LENGTH (1088) + sizeof(PCRE2_SIZE)` fails (pcre2_compile.c:1174) | `NULL` |
| 24 | `pcre2_code_free_8` | `code == NULL` — guarded no-op (pcre2_compile.c:1204) | `void`, no error |

### pcre2_compile.c — heap-allocation failures (all reported as `ERR21` = 121)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 25 | `pcre2_compile_8` | `malloc` of the heap parsed-pattern vector fails (only attempted when `parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE`; use a long pattern + a failing `private_malloc`) (pcre2_compile.c:10748-10754) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121), `*erroroffset = 0` |
| 26 | `pcre2_compile_8` | `malloc` of the `groupinfo` vector fails (needs a lookbehind and `bracount >= GROUPINFO_DEFAULT_SIZE/2`) (pcre2_compile.c:10779-10786) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121), `*erroroffset = 0` |
| 27 | `pcre2_compile_8` | `malloc` of the `pcre2_real_code` block (`re_blocksize`) fails (pcre2_compile.c:10881-10888) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121), `*erroroffset = 0` |
| 28 | `pcre2_compile_8` | `malloc` of the enlarged named-group list fails (needs > `NAMED_GROUP_LIST_SIZE` named groups) (pcre2_compile.c:5772-5779) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121) |
| 29 | `pcre2_compile_8` | `compile_optimize_class` allocation fails while compiling a wide/UTF character class such as `"[\\x{100}-\\x{200}]"` with `PCRE2_UTF` (pcre2_compile_class.c:1127) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121) |
| 30 | `pcre2_compile_8` | `malloc` of the capture bitmap for a `(*scs:…)` group fails, e.g. `"(a)(*scs:(1)b)"` (pcre2_compile_cgroup.c:384) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121) |
| 31 | `pcre2_compile_8` | `malloc` of the `recurse_arguments` block for `(?1(2))`-style recursion arguments fails (pcre2_compile_cgroup.c:531) | `NULL`, `*errorptr = PCRE2_ERROR_HEAP_FAILED` (121) |

### pcre2_compile.c — escape-sequence errors (`PRIV(check_escape)`, `get_ucp`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 32 | `pcre2_compile_8` | Pattern ends with a lone backslash: `a\` (pcre2_compile.c:1504-1507) | `NULL`, `PCRE2_ERROR_END_BACKSLASH` (101) |
| 33 | `pcre2_compile_8` | Pattern ends with `\c`: `a\c` (pcre2_compile.c:2163-2166) | `NULL`, `PCRE2_ERROR_END_BACKSLASH_C` (102) |
| 34 | `pcre2_compile_8` | Unrecognised alphanumeric escape: `\y` (also `\i`, `\j`, `\m`, `\q`, `\T`, …) (pcre2_compile.c:2211-2213) | `NULL`, `PCRE2_ERROR_UNKNOWN_ESCAPE` (103) |
| 35 | `pcre2_compile_8` | `\c` followed by a non-printable ASCII byte (`< 32` or `> 126`), e.g. `\c` + byte 0x01, or `\c` + 0x7F (pcre2_compile.c:2174-2178) | `NULL`, `PCRE2_ERROR_BACKSLASH_C_SYNTAX` (168) |
| 36 | `pcre2_compile_8` | `\F`, `\l` or `\L` (pcre2_compile.c:1633-1637) | `NULL`, `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` (137) |
| 37 | `pcre2_compile_8` | `\u` when neither `PCRE2_ALT_BSUX` nor `PCRE2_EXTRA_ALT_BSUX` is set (pcre2_compile.c:1647-1649) | `NULL`, `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` (137) |
| 38 | `pcre2_compile_8` | `\U` when neither `PCRE2_ALT_BSUX` nor `PCRE2_EXTRA_ALT_BSUX` is set (pcre2_compile.c:1715-1716) | `NULL`, `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` (137) |
| 39 | `pcre2_compile_8` | `\N{` inside a character class: `[\N{2}]` (pcre2_compile.c:1582-1586) | `NULL`, `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` (137) |
| 40 | `pcre2_compile_8` | `\N{name}` outside a class where `{name}` is not a valid quantifier: `\N{abc}` (pcre2_compile.c:1593-1598) | `NULL`, `PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE` (137) |
| 41 | `pcre2_compile_8` | `\N{U+0041}` **without** `PCRE2_UTF` (pcre2_compile.c:1559-1576) | `NULL`, `PCRE2_ERROR_SUPPORTED_ONLY_IN_UNICODE` (193) |
| 42 | `pcre2_compile_8` | `\u{…}` with `PCRE2_EXTRA_ALT_BSUX` whose accumulated value overflows 32 bits, e.g. `\u{fffffffff}` (pcre2_compile.c:1663-1667) | `NULL`, `PCRE2_ERROR_BACKSLASH_U_CODE_POINT_TOO_BIG` (177) |
| 43 | `pcre2_compile_8` | `\u{110000}` with `PCRE2_EXTRA_ALT_BSUX` + `PCRE2_UTF` (value > 0x10FFFF) (pcre2_compile.c:1702) | `NULL`, `PCRE2_ERROR_BACKSLASH_U_CODE_POINT_TOO_BIG` (177) |
| 44 | `pcre2_compile_8` | `\\u0100` (backslash-u followed by the four hex digits `0100`) with `PCRE2_ALT_BSUX` and **no** `PCRE2_UTF` (in 8-bit `MAX_NON_UTF_CHAR` = 0xff) (pcre2_compile.c:1708) | `NULL`, `PCRE2_ERROR_BACKSLASH_U_CODE_POINT_TOO_BIG` (177) |
| 45 | `pcre2_compile_8` | `\u{d800}` with `PCRE2_EXTRA_ALT_BSUX` + `PCRE2_UTF` and no `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` (pcre2_compile.c:1704-1706) | `NULL`, `PCRE2_ERROR_UNICODE_DISALLOWED_CODE_POINT` (173) |
| 46 | `pcre2_compile_8` | `\o{154000}` (= 0xD800) with `PCRE2_UTF` and no `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` (pcre2_compile.c:2011-2015) | `NULL`, `PCRE2_ERROR_UNICODE_DISALLOWED_CODE_POINT` (173) |
| 47 | `pcre2_compile_8` | `\x{d800}` with `PCRE2_UTF` and no `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` (pcre2_compile.c:2092-2096) | `NULL`, `PCRE2_ERROR_UNICODE_DISALLOWED_CODE_POINT` (173) |
| 48 | `pcre2_compile_8` | `\o{4000}` in 8-bit non-UTF mode (value > 0xFF), or `\o{4200000}` with `PCRE2_UTF` (value > 0x10FFFF) (pcre2_compile.c:2006-2010) | `NULL`, `PCRE2_ERROR_CODE_POINT_TOO_BIG` (134) |
| 49 | `pcre2_compile_8` | `\x{100}` in 8-bit non-UTF mode (> `MAX_NON_UTF_CHAR` = 0xff), or `\x{110000}` with `PCRE2_UTF` (pcre2_compile.c:2087-2091) | `NULL`, `PCRE2_ERROR_CODE_POINT_TOO_BIG` (134) |
| 50 | `pcre2_compile_8` | `\o` not followed by `{`: `\o7` (pcre2_compile.c:1971-1974) | `NULL`, `PCRE2_ERROR_BACKSLASH_O_MISSING_BRACE` (155) |
| 51 | `pcre2_compile_8` | `\o{}` (empty braces, or `\o{` at end of pattern) (pcre2_compile.c:1979-1982) | `NULL`, `PCRE2_ERROR_MISSING_OCTAL_OR_HEX_DIGITS` (178) |
| 52 | `pcre2_compile_8` | `\x{}` (empty braces), or `\x{` at end of pattern, or `\N{U+}` with `PCRE2_UTF` (pcre2_compile.c:2058-2061) | `NULL`, `PCRE2_ERROR_MISSING_OCTAL_OR_HEX_DIGITS` (178) |
| 53 | `pcre2_compile_8` | `\x` not followed by a hex digit, e.g. `\xz` or `\x` at end of pattern (Perl-style `\x`, i.e. without `PCRE2_ALT_BSUX`) (pcre2_compile.c:2123-2127) | `NULL`, `PCRE2_ERROR_MISSING_OCTAL_OR_HEX_DIGITS` (178) |
| 54 | `pcre2_compile_8` | `\o{12x}` — a non-octal character where the closing `}` was expected (pcre2_compile.c:2020-2023) | `NULL`, `PCRE2_ERROR_INVALID_OCTAL` (164) |
| 55 | `pcre2_compile_8` | `\x{1z}` — a non-hex character where the closing `}` was expected (pcre2_compile.c:2107-2110) | `NULL`, `PCRE2_ERROR_INVALID_HEXADECIMAL` (167) |
| 56 | `pcre2_compile_8` | `\400` (octal value > 0xFF) in 8-bit **non-UTF** mode (pcre2_compile.c:1954-1956) | `NULL`, `PCRE2_ERROR_OCTAL_BYTE_TOO_BIG` (151) |
| 57 | `pcre2_compile_8` | `\400` with `PCRE2_EXTRA_PYTHON_OCTAL` set (value > 0xFF is always forbidden) (pcre2_compile.c:1953) | `NULL`, `PCRE2_ERROR_OVERSIZE_PYTHON_OCTAL` (202) |
| 58 | `pcre2_compile_8` | `\0` (single zero digit, no following octal digit) with `PCRE2_EXTRA_NO_BS0` set (pcre2_compile.c:1962-1963) | `NULL`, `PCRE2_ERROR_MISSING_OCTAL_DIGIT` (198) |
| 59 | `pcre2_compile_8` | `\g` as the last two characters of the pattern (pcre2_compile.c:1749-1752) | `NULL`, `PCRE2_ERROR_BACKSLASH_G_SYNTAX` (157) |
| 60 | `pcre2_compile_8` | `\g` followed by an undelimited non-number: `\gx` (pcre2_compile.c:1824-1828) | `NULL`, `PCRE2_ERROR_BACKSLASH_G_SYNTAX` (157) |
| 61 | `pcre2_compile_8` | `\g` followed by something other than `{`, `<` or `'` reached via the parse loop, e.g. `\g=` (pcre2_compile.c:3745-3749) | `NULL`, `PCRE2_ERROR_BACKSLASH_G_SYNTAX` (157) |
| 62 | `pcre2_compile_8` | `\k` not followed by `{`, `<` or `'`, e.g. `\kx` or `\k` at end of pattern (pcre2_compile.c:3745-3749) | `NULL`, `PCRE2_ERROR_BACKSLASH_K_SYNTAX` (169) |
| 63 | `pcre2_compile_8` | `\g{1a}` — number read but no closing `}`, e.g. `(a)\g{1a}` (pcre2_compile.c:1811-1815) | `NULL`, `PCRE2_ERROR_MISSING_NUMBER_TERMINATOR` (219) |
| 64 | `pcre2_compile_8` | `\g<1a>` — number read but no closing `>`, e.g. `(a)\g<1a` (pcre2_compile.c:3764-3768) | `NULL`, `PCRE2_ERROR_MISSING_NUMBER_TERMINATOR` (219) |
| 65 | `pcre2_compile_8` | `\g{-0}` or `\g{+0}` (signed zero relative reference); also `(?+0)` / `(?-0)` (pcre2_compile.c:1299-1305) | `NULL`, `PCRE2_ERROR_ZERO_RELATIVE_REFERENCE` (126) |
| 66 | `pcre2_compile_8` | `\g{-1}` with no preceding capture group (relative reference points before group 1) (pcre2_compile.c:1308-1312) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 67 | `pcre2_compile_8` | `\g0` or `\g{0}` (absolute reference to group 0) (pcre2_compile.c:1832-1836) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 68 | `pcre2_compile_8` | `\g{70000}` — braced group number > `MAX_GROUP_NUMBER` (65535) (pcre2_compile.c:1803) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 69 | `pcre2_compile_8` | `\g70000` — undelimited group number > 65535 (pcre2_compile.c:1824) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 70 | `pcre2_compile_8` | `\g<70000>` — angle-bracketed recursion number > 65535 (pcre2_compile.c:3761) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 71 | `pcre2_compile_8` | `\79999` with `PCRE2_EXTRA_PYTHON_OCTAL` (backreference number too big). NB **not** `\70000`: the three-digit-octal peek at pcre2_compile.c:1873 succeeds for `\700` and reports ERR102 (202) instead, so the digit after the first must not be octal (pcre2_compile.c:1885-1888) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 72 | `pcre2_compile_8` | `\800000` in default (Perl) mode — `read_number` fails, sentinel `INT_MAX > MAX_GROUP_NUMBER`. NB **not** `\70000`: the `s < 10 \|\| c >= '8' \|\| s <= bracount` test at pcre2_compile.c:1911 is false for a leading `7`, so that falls through to the octal reader and reports ERR51 (151) instead (pcre2_compile.c:1908-1924) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 73 | `pcre2_compile_8` | `\p` as the last two characters of the pattern (pcre2_compile.c:2274 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 74 | `pcre2_compile_8` | `\p{` at end of pattern (pcre2_compile.c:2289 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 75 | `pcre2_compile_8` | `\p{L` — property name runs into the end of the pattern with no `}` (pcre2_compile.c:2295 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 76 | `pcre2_compile_8` | `\p{L!}` — a name character outside the accepted `'&'`..`'z'` range. NB **not** `\p{L+}`: `'+'` (0x2B) is inside that range, is accepted into the name, and then reports ERR47 (147) instead (pcre2_compile.c:2322 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 77 | `pcre2_compile_8` | `\p{` + 49 or more name characters without a `}` (the `name[50]` buffer fills and the loop exits with `c != '}'`) (pcre2_compile.c:2337 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 78 | `pcre2_compile_8` | `\p9` — unbraced `\p` followed by a non-ASCII-letter (pcre2_compile.c:2354 → 2451) | `NULL`, `PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY` (146) |
| 79 | `pcre2_compile_8` | `\p{Zz}` — syntactically valid but unknown property name (binary-chop miss) (pcre2_compile.c:2448) | `NULL`, `PCRE2_ERROR_UNKNOWN_UNICODE_PROPERTY` (147) |
| 80 | `pcre2_compile_8` | `\p{xx:yy}` where the part before `:`/`=` is not `bidiclass`/`bc`/`script`/`sc`/`scriptextensions`/`scx`, e.g. `\p{foo:Latin}` (pcre2_compile.c:2396-2400) | `NULL`, `PCRE2_ERROR_UNKNOWN_UNICODE_PROPERTY` (147) |
| 81 | `pcre2_compile_8` | `\p{sc:Lu}` — the name after `sc:`/`scx:` resolves to a non-script property, so the script-type `switch` falls through to `break` (pcre2_compile.c:2431-2448) | `NULL`, `PCRE2_ERROR_UNKNOWN_UNICODE_PROPERTY` (147) |
| 82 | `pcre2_compile_8` | `\X`, `\p`, `\P` in a build without `SUPPORT_UNICODE` — **not compiled here** (pcre2_compile.c:3686, 3732, 4559) | `NULL`, `PCRE2_ERROR_UNICODE_PROPERTIES_UNAVAILABLE` (145) — unreachable in this build |
| 83 | `pcre2_compile_8` | `\C` with `PCRE2_NEVER_BACKSLASH_C` in `options` (pcre2_compile.c:3664-3667) | `NULL`, `PCRE2_ERROR_BACKSLASH_C_CALLER_DISABLED` (183) |
| 84 | `pcre2_compile_8` | `\C` in a build with the `NEVER_BACKSLASH_C` macro — **not defined here** (pcre2_compile.c:3661) | `NULL`, `PCRE2_ERROR_BACKSLASH_C_LIBRARY_DISABLED` (185) — unreachable in this build |
| 85 | `pcre2_compile_8` | `(?=\K)` — `\K` inside any lookaround without `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` (also `(?<=a\K)`, `(?!\K)`) (pcre2_compile.c:8338-8342) | `NULL`, `PCRE2_ERROR_BACKSLASH_K_IN_LOOKAROUND` (199) |
| 86 | `pcre2_compile_8` | `PCRE2_UTF` set together with `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` — 16-bit-only check, **not compiled** in 8-bit mode (pcre2_compile.c:10631-10636) | `NULL`, `PCRE2_ERROR_NO_SURROGATES_IN_UTF16` (191) — unreachable in this build |

### pcre2_compile.c — quantifier errors

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 87 | `pcre2_compile_8` | `a{2,1}` — max less than min (pcre2_compile.c:1431-1435) | `NULL`, `PCRE2_ERROR_QUANTIFIER_OUT_OF_ORDER` (104) |
| 88 | `pcre2_compile_8` | `a{65536}` — min > `MAX_REPEAT_COUNT` (65535) (pcre2_compile.c:1402) | `NULL`, `PCRE2_ERROR_QUANTIFIER_TOO_BIG` (105) |
| 89 | `pcre2_compile_8` | `a{,65536}` — max too big with the min omitted (pcre2_compile.c:1407) | `NULL`, `PCRE2_ERROR_QUANTIFIER_TOO_BIG` (105) |
| 90 | `pcre2_compile_8` | `a{1,65536}` — max too big with a min present (pcre2_compile.c:1426) | `NULL`, `PCRE2_ERROR_QUANTIFIER_TOO_BIG` (105) |
| 91 | `pcre2_compile_8` | Quantifier with no repeatable item before it: `*a`, `+a`, `?a`, `{2}a`, `(?i)*`, `\b*`, `^*` (pcre2_compile.c:3847-3851) | `NULL`, `PCRE2_ERROR_QUANTIFIER_INVALID` (109) |
| 92 | `pcre2_compile_8` | Internal: a quantifier applied to a non-character-type opcode (`op_previous >= OP_EODN \|\| op_previous <= OP_WORD_BOUNDARY`) — dead branch, `PCRE2_DEBUG_UNREACHABLE` (pcre2_compile.c:7857-7861) | `NULL`, `PCRE2_ERROR_INTERNAL_UNEXPECTED_REPEAT` (110) |

### pcre2_compile.c — character-class errors

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 93 | `pcre2_compile_8` | `[` as the very last character of the pattern (class-start loop hits end of input) (pcre2_compile.c:4181-4187) | `NULL`, `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106) |
| 94 | `pcre2_compile_8` | `[a` — class contents run to the end of the pattern with no `]` (pcre2_compile.c:4698-4707) | `NULL`, `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106) |
| 95 | `pcre2_compile_8` | `[\B]`, `[\R]` or `[\X]` — multi-character escapes forbidden in a class (pcre2_compile.c:4502-4506) | `NULL`, `PCRE2_ERROR_ESCAPE_INVALID_IN_CLASS` (107) |
| 96 | `pcre2_compile_8` | `[\A]`, `[\Z]`, `[\z]`, `[\G]`, `[\K]` or `[\C]` — anchors/`\K`/`\C` forbidden in a class (pcre2_compile.c:4572-4579) | `NULL`, `PCRE2_ERROR_ESCAPE_INVALID_IN_CLASS` (107) |
| 97 | `pcre2_compile_8` | `[\N]` (pcre2_compile.c:4508-4510) | `NULL`, `PCRE2_ERROR_BACKSLASH_N_IN_CLASS` (171) |
| 98 | `pcre2_compile_8` | `[b-a]` — range endpoints out of order (pcre2_compile.c:4666-4670) | `NULL`, `PCRE2_ERROR_CLASS_RANGE_ORDER` (108) |
| 99 | `pcre2_compile_8` | `[a-[:digit:]]` — a POSIX class used as the upper end of a range (pcre2_compile.c:4029-4034) | `NULL`, `PCRE2_ERROR_CLASS_INVALID_RANGE` (150) |
| 100 | `pcre2_compile_8` | `[[:digit:]-a]` — hyphen after a POSIX class starts a forbidden range (pcre2_compile.c:4044-4049) | `NULL`, `PCRE2_ERROR_CLASS_INVALID_RANGE` (150) |
| 101 | `pcre2_compile_8` | `[a-\d]` — a multi-character escape used as the upper end of a range (pcre2_compile.c:4591-4595) | `NULL`, `PCRE2_ERROR_CLASS_INVALID_RANGE` (150) |
| 102 | `pcre2_compile_8` | `[\d-\w]` — hyphen after a multi-character escape followed by another escape (pcre2_compile.c:4600-4605) | `NULL`, `PCRE2_ERROR_CLASS_INVALID_RANGE` (150) |
| 103 | `pcre2_compile_8` | `[\d-a]` — hyphen after a multi-character escape followed by a literal (pcre2_compile.c:4680-4685) | `NULL`, `PCRE2_ERROR_CLASS_INVALID_RANGE` (150) |
| 104 | `pcre2_compile_8` | `[:alpha:]` — POSIX class syntax used at the top level, not inside a class (pcre2_compile.c:3935-3941, colon case) | `NULL`, `PCRE2_ERROR_POSIX_CLASS_NOT_IN_CLASS` (112) |
| 105 | `pcre2_compile_8` | `[.ch.]` or `[=ch=]` at the top level (pcre2_compile.c:3935-3941, non-colon case) | `NULL`, `PCRE2_ERROR_POSIX_NO_SUPPORT_COLLATING` (113) |
| 106 | `pcre2_compile_8` | `[[.ch.]]` or `[[=ch=]]` — collating element inside a class (pcre2_compile.c:4061-4066) | `NULL`, `PCRE2_ERROR_POSIX_NO_SUPPORT_COLLATING` (113) |
| 107 | `pcre2_compile_8` | `[[:foo:]]` — unknown POSIX class name (also `[[:^foo:]]`) (pcre2_compile.c:4074-4080) | `NULL`, `PCRE2_ERROR_UNKNOWN_POSIX_CLASS` (130) |
| 108 | `pcre2_compile_8` | A **single** class whose own character list pushes the pattern past `MAX_PATTERN_SIZE` (65536), e.g. one class holding 17000 distinct code points above 0xFFFF (`[\x{10000}\x{10002}…]`) with `PCRE2_UTF`; each such item costs 4 bytes of list. Many *separate* wide classes hit row 17's site instead, because this check only sees one class's list at a time. Unlike row 17 this site does **not** zero `cb.erroroffset`, so `*erroroffset` is left pointing into the pattern (pcre2_compile_class.c:1771) | `NULL`, `PCRE2_ERROR_PATTERN_TOO_LARGE` (120) |

### pcre2_compile.c — extended character classes (`PCRE2_ALT_EXTENDED_CLASS`, `(?[...])`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 109 | `pcre2_compile_8` | **16** nested `[` with `PCRE2_ALT_EXTENDED_CLASS`, e.g. `[[[[[[[[[[[[[[[[a]]]]]]]]]]]]]]]]` — the guard is `class_depth_m1 >= ECLASS_NEST_LIMIT - 1` with `ECLASS_NEST_LIMIT` = 15, so exactly 15 nested `[` still compiles (pcre2_compile.c:4166-4171) | `NULL`, `PCRE2_ERROR_ECLASS_NEST_TOO_DEEP` (207) |
| 110 | `pcre2_compile_8` | `[a---b]` with `PCRE2_ALT_EXTENDED_CLASS` — triple-repeated set operator (also `[a\|\|\|b]`, `[a&&&b]`, `[a~~~b]`) (pcre2_compile.c:4415-4419) | `NULL`, `PCRE2_ERROR_ECLASS_INVALID_OPERATOR` (208) |
| 111 | `pcre2_compile_8` | `[--a]` with `PCRE2_ALT_EXTENDED_CLASS` — set operator with no preceding operand (pcre2_compile.c:4423-4426) | `NULL`, `PCRE2_ERROR_ECLASS_UNEXPECTED_OPERATOR` (209) |
| 112 | `pcre2_compile_8` | `(?[+[a]])` — Perl-extended binary operator with no preceding operand (also `(?[\|[a]])`, `(?[-[a]])`, `(?[&[a]])`, `(?[^[a]])`) (pcre2_compile.c:4350-4353) | `NULL`, `PCRE2_ERROR_ECLASS_UNEXPECTED_OPERATOR` (209) |
| 113 | `pcre2_compile_8` | `[a--]` with `PCRE2_ALT_EXTENDED_CLASS`, or `(?[[a]+])` — class ends immediately after an operator (pcre2_compile.c:4296-4300) | `NULL`, `PCRE2_ERROR_ECLASS_EXPECTED_OPERAND` (210) |
| 114 | `pcre2_compile_8` | `[[a]--[b]&&[c]]` with `PCRE2_ALT_EXTENDED_CLASS` — mixed operator precedence at one nesting level (pcre2_compile.c:4430-4435) | `NULL`, `PCRE2_ERROR_ECLASS_MIXED_OPERATORS` (211) |
| 115 | `pcre2_compile_8` | `[[a]` with `PCRE2_ALT_EXTENDED_CLASS` (end of pattern at depth 0 having seen depth 1) (pcre2_compile.c:4702-4704) | `NULL`, `PCRE2_ERROR_ECLASS_HINT_SQUARE_BRACKET` (212) |
| 116 | `pcre2_compile_8` | `(?[[a][b]])` — implicit union: a nested `[` immediately after an operand (pcre2_compile.c:4158-4163) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` (213) |
| 117 | `pcre2_compile_8` | `(?[[a][:digit:]])` — a POSIX class immediately after an operand (pcre2_compile.c:4053-4058) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` (213) |
| 118 | `pcre2_compile_8` | `(?[[a]![b]])` — unary `!` immediately after an operand (pcre2_compile.c:4383-4386) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` (213) |
| 119 | `pcre2_compile_8` | `(?[[a]\d])` — a class escape immediately after an operand (pcre2_compile.c:4609-4613) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` (213) |
| 120 | `pcre2_compile_8` | Escaped literal immediately after an operand inside `(?[...])`, e.g. `(?[[a]\x41])` (pcre2_compile.c:4655-4659) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR` (213) |
| 121 | `pcre2_compile_8` | `(?[])` — empty Perl-extended class (pcre2_compile.c:4303-4307) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_EMPTY_EXPR` (214) |
| 122 | `pcre2_compile_8` | `(?[[a]]` — the outermost `]` is not followed by `)` (pcre2_compile.c:4320-4326) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_MISSING_CLOSE` (215) |
| 123 | `pcre2_compile_8` | `(?[\Qa\E])` — a **non-empty** `\Q…\E` inside a Perl-extended class (pcre2_compile.c:3991-3995) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_CHAR` (216) |
| 124 | `pcre2_compile_8` | `(?[a])` — an unescaped literal (or a bare `-`) inside `(?[...])` (pcre2_compile.c:4623-4627) | `NULL`, `PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_CHAR` (216) |
| 125 | `pcre2_compile_8` | `(?[([a]]` — `]` seen while inside a `(`-nested Perl-extended sub-expression (depth != 0) (pcre2_compile.c:4282-4287) | `NULL`, `PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS` (114) |
| 126 | `pcre2_compile_8` | `(?[(` — a `(`-nested Perl-extended sub-expression runs to the end of the pattern (pcre2_compile.c:4181-4185) | `NULL`, `PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS` (114) |
| 127 | `pcre2_compile_8` | `(?[([a]` — end of pattern while still inside a `(`-nested Perl-extended sub-expression (pcre2_compile.c:4698-4701) | `NULL`, `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106) — **corrected against the C**: the `errorcode = ERR14` written at pcre2_compile.c:4700 is immediately overwritten, because the next statement is a bare `if (ALT_EXT && …) … else errorcode = ERR6;` whose `else` arm always runs in Perl-extended mode. ERR14 is therefore dead at this site |
| 128 | `pcre2_compile_8` | `(?[)` — `)` at Perl-extended class depth 0 (pcre2_compile.c:4288-4292) | `NULL`, `PCRE2_ERROR_UNMATCHED_CLOSING_PARENTHESIS` (122) |

### pcre2_compile.c — group / parenthesis structure

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 129 | `pcre2_compile_8` | `(a` — an unclosed group at end of pattern (also `(?:a`, `(?(1)a`, `(?&name`, `(?1`, `(?<`+`=a`) (pcre2_compile.c:5920-5921) | `NULL`, `PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS` (114) |
| 130 | `pcre2_compile_8` | `(a)(*scs:(1` — capture list runs to end of pattern. NB **not** `(a)(*scs:(1a`: any non-`,`/non-`)` character after a complete entry is reported as ERR24 (124) instead, so the pattern must stop exactly at the end of an entry (pcre2_compile.c:2809, 2823-2824) | `NULL`, `PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS` (114) |
| 131 | `pcre2_compile_8` | `a)` — unmatched closing parenthesis at `nest_depth == 0` (pcre2_compile.c:5860-5864) | `NULL`, `PCRE2_ERROR_UNMATCHED_CLOSING_PARENTHESIS` (122) |
| 132 | `pcre2_compile_8` | 251 nested `(` (default `parens_nest_limit` = 250); lower it with `pcre2_set_parens_nest_limit_8(cc, 1)` and compile `"((a))"` (pcre2_compile.c:3238-3242) | `NULL`, `PCRE2_ERROR_PARENTHESES_NEST_TOO_DEEP` (119) |
| 133 | `pcre2_compile_8` | The `nest_save` stack (sized from `COMPILE_WORK_SIZE`) overflows for `(?\|`/`(?i:`/`(?x:` nesting — hundreds of nested option groups such as `(?i:(?i:(?i:…)))` (pcre2_compile.c:4997-5002) | `NULL`, `PCRE2_ERROR_QUERY_BARJX_NEST_TOO_DEEP` (184) |
| 134 | `pcre2_compile_8` | `nest_save` stack overflow while entering a `(*asr:` / `(*atomic_script_run:` group (pcre2_compile.c:4853-4858) | `NULL`, `PCRE2_ERROR_QUERY_BARJX_NEST_TOO_DEEP` (184) |
| 135 | `pcre2_compile_8` | `nest_save` stack overflow while entering a conditional-assertion group `(?(?=…)…)` (pcre2_compile.c:5666-5671) | `NULL`, `PCRE2_ERROR_QUERY_BARJX_NEST_TOO_DEEP` (184) |
| 136 | `pcre2_compile_8` | `(?#comment` — bracketed comment with no closing `)` (pcre2_compile.c:3483-3487) | `NULL`, `PCRE2_ERROR_MISSING_COMMENT_CLOSING` (118) |
| 137 | `pcre2_compile_8` | 65536 capturing groups (`cb->bracount >= MAX_GROUP_NUMBER`), e.g. `"()"` repeated 65536 times (pcre2_compile.c:4735-4739) | `NULL`, `PCRE2_ERROR_TOO_MANY_CAPTURES` (197) |
| 138 | `pcre2_compile_8` | 65536th **named** capturing group (`cb->bracount >= MAX_GROUP_NUMBER` in the `DEFINE_NAME` path) (pcre2_compile.c:5696-5700) | `NULL`, `PCRE2_ERROR_TOO_MANY_CAPTURES` (197) |
| 139 | `pcre2_compile_8` | 10001 distinct named groups (`MAX_NAME_COUNT` = 10000) (pcre2_compile.c:5707-5711) | `NULL`, `PCRE2_ERROR_TOO_MANY_NAMED_SUBPATTERNS` (149) |
| 140 | `pcre2_compile_8` | `(?z)` — unrecognised character in an option setting (also `(?-z)`, `(?q:a)`) (pcre2_compile.c:5127-5129) | `NULL`, `PCRE2_ERROR_INVALID_AFTER_PARENS_QUERY` (111) |
| 141 | `pcre2_compile_8` | `(?^-i)` — `-` after `^` in an option setting (`hyphenok` was cleared); also `(?i-s-x)` (second hyphen) (pcre2_compile.c:5052-5057) | `NULL`, `PCRE2_ERROR_INVALID_HYPHEN_IN_OPTIONS` (194) |
| 142 | `pcre2_compile_8` | `(?Px)` — `(?P` followed by something other than `<`, `>` or `=` (pcre2_compile.c:5194-5198) | `NULL`, `PCRE2_ERROR_UNRECOGNIZED_AFTER_QUERY_P` (141) |
| 143 | `pcre2_compile_8` | `(?Rx)` — `(?R` not followed by `)` or `(` (pcre2_compile.c:5213-5217) | `NULL`, `PCRE2_ERROR_PARENS_QUERY_R_MISSING_CLOSING` (158) |
| 144 | `pcre2_compile_8` | `(?+a)` — `(?+` not followed by a digit (pcre2_compile.c:5230-5235) | `NULL`, `PCRE2_ERROR_BAD_RELATIVE_REFERENCE` (129) |
| 145 | `pcre2_compile_8` | `(?70000)` — recursion/subroutine call number > 65535 (also `(?+70000)`) (pcre2_compile.c:5241-5244) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 146 | `pcre2_compile_8` | `(?2)a` — subroutine call to a group number greater than the total group count (pcre2_compile.c:8184-8190) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 147 | `pcre2_compile_8` | `\1` with no capture group in the pattern (backreference number > `bracount`) (pcre2_compile.c:8140-8146) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 148 | `pcre2_compile_8` | `\k<xyz>` / `(?P=xyz)` / `\g{xyz}` where no group is named `xyz` (pcre2_compile.c:7140-7146) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |

### pcre2_compile.c — subpattern names

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 149 | `pcre2_compile_8` | `(?<` at end of pattern — `read_name` finds no characters for a group name (also `(?'`, `(?&`) (pcre2_compile.c:2604-2608, `is_group` arm) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NAME_EXPECTED` (162) |
| 150 | `pcre2_compile_8` | `(?<>a)` — empty group name (pcre2_compile.c:2683-2687) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NAME_EXPECTED` (162) |
| 151 | `pcre2_compile_8` | `(?<abc` — group name not followed by its terminator (also `(?'abc`, `\k<abc`, `(?P=abc`) (pcre2_compile.c:2690-2696) | `NULL`, `PCRE2_ERROR_MISSING_NAME_TERMINATOR` (142) |
| 152 | `pcre2_compile_8` | `(?<1a>x)` — group name starting with an ASCII digit (non-UTF path) (pcre2_compile.c:2656-2661) | `NULL`, `PCRE2_ERROR_INVALID_SUBPATTERN_NAME` (144) |
| 153 | `pcre2_compile_8` | `PCRE2_UTF` set and a group name starting with a Unicode `Nd` character, e.g. `(?<\xd9\xa1a>x)` (U+0661 ARABIC-INDIC DIGIT ONE) (pcre2_compile.c:2629-2634) | `NULL`, `PCRE2_ERROR_INVALID_SUBPATTERN_NAME` (144) |
| 154 | `pcre2_compile_8` | Group name longer than `MAX_NAME_SIZE` (128) code units, e.g. `(?<` + 129 `a`s + `>x)` (pcre2_compile.c:2671-2675) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NAME_TOO_LONG` (148) |
| 155 | `pcre2_compile_8` | `(?<a>x)(?<a>y)` without `PCRE2_DUPNAMES` / `(?J)` (pcre2_compile.c:5736-5740) | `NULL`, `PCRE2_ERROR_DUPLICATE_SUBPATTERN_NAME` (143) |
| 156 | `pcre2_compile_8` | `(?\|(?<a>x)\|(?<b>y))` — two different names assigned to the same group number in a `(?\|` group (pcre2_compile.c:5757-5761) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NAMES_MISMATCH` (165) |

### pcre2_compile.c — verbs and alpha assertions

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 157 | `pcre2_compile_8` | `read_name` finding no verb characters at all (pcre2_compile.c:2604-2607, `!is_group` arm) | `NULL`, `PCRE2_ERROR_VERB_UNKNOWN` (160) — **corrected against the C**: unreachable in any build. Both `!is_group` callers (pcre2_compile.c:4763 and 4884) are guarded by `else if (ptrend - ptr <= 1 \|\| (c = ptr[1]) == ')') break;` at pcre2_compile.c:4749, so `read_name` is never entered unless there is at least one code unit after the `*`, and `read_name`'s own `ptr >= ptrend` test is made *after* consuming that `*`. `(*` and `(*)` therefore report ERR9 (109) for the stray quantifier, and `(*:ABC)` is a legal `(*MARK:ABC)` synonym (`verbs[0]` has an empty name). ERR60 is still reachable from rows 158-160 |
| 158 | `pcre2_compile_8` | `(*ACCEPT.)` — verb name not followed by `:` or `)` (pcre2_compile.c:4886-4891) | `NULL`, `PCRE2_ERROR_VERB_UNKNOWN` (160) |
| 159 | `pcre2_compile_8` | `(*FOO)` — verb name not in `{"", MARK, ACCEPT, F, FAIL, COMMIT, PRUNE, SKIP, THEN}` (pcre2_compile.c:4903-4907) | `NULL`, `PCRE2_ERROR_VERB_UNKNOWN` (160) |
| 160 | `pcre2_compile_8` | `(*MARK:abc` — verb name argument runs to the end of the pattern with no `)` (pcre2_compile.c:5873-5877) | `NULL`, `PCRE2_ERROR_VERB_UNKNOWN` (160) |
| 161 | `pcre2_compile_8` | `(*MARK)` — `(*MARK)` requires a non-empty argument (also `(*MARK:)`) (pcre2_compile.c:4917-4921) | `NULL`, `PCRE2_ERROR_MARK_MISSING_ARGUMENT` (166) |
| 162 | `pcre2_compile_8` | `(*MARK:` + 256 characters + `)` — argument longer than `MAX_MARK` (255) code units (pcre2_compile.c:4364-4369) | `NULL`, `PCRE2_ERROR_VERB_NAME_TOO_LONG` (176) |
| 163 | `pcre2_compile_8` | `(*MARK:\d)` with `PCRE2_ALT_VERBNAMES` — a non-data escape inside a verb name (pcre2_compile.c:3413-3415) | `NULL`, `PCRE2_ERROR_ESCAPE_INVALID_IN_VERB` (140) |
| 164 | `pcre2_compile_8` | `(*pla)` — alpha-assertion name not followed by `:` (pcre2_compile.c:4766-4770) | `NULL`, `PCRE2_ERROR_ALPHA_ASSERTION_UNKNOWN` (195) |
| 165 | `pcre2_compile_8` | `(*foo:a)` — alpha-assertion name not in the `alasnames` table (`pla plb napla naplb nla nlb positive_lookahead positive_lookbehind non_atomic_positive_lookahead non_atomic_positive_lookbehind negative_lookahead negative_lookbehind scs scan_substring atomic sr asr script_run atomic_script_run`) (pcre2_compile.c:4782-4786) | `NULL`, `PCRE2_ERROR_ALPHA_ASSERTION_UNKNOWN` (195) |
| 166 | `pcre2_compile_8` | `(*sr:a)` / `(*script_run:a)` in a build without `SUPPORT_UNICODE` — **not compiled here** (pcre2_compile.c:4872) | `NULL`, `PCRE2_ERROR_SCRIPT_RUN_NOT_AVAILABLE` (196) — unreachable in this build |
| 167 | `pcre2_compile_8` | An `alasmeta` entry whose meta value is not handled by the dispatch switch — dead `default:` (pcre2_compile.c:4805-4808) | `NULL`, `PCRE2_ERROR_INTERNAL_BAD_CODE` (189) |

### pcre2_compile.c — `(*scs:...)` / recursion capture lists

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 168 | `pcre2_compile_8` | `(*scs:a)` — no `(` after the `scs:` (pcre2_compile.c:2743-2747) | `NULL`, `PCRE2_ERROR_MISSING_OPENING_PARENTHESIS` (218) |
| 169 | `pcre2_compile_8` | `(a)(*scs:(` — capture list ends immediately after `(` (pcre2_compile.c:2754-2758) | `NULL`, `PCRE2_ERROR_EXPECTED_CAPTURE_GROUP` (217) |
| 170 | `pcre2_compile_8` | `(a)(*scs:(x)b)` — capture-list entry is neither a number nor `<name>` nor `'name'` (pcre2_compile.c:2777-2785) | `NULL`, `PCRE2_ERROR_EXPECTED_CAPTURE_GROUP` (217) |
| 171 | `pcre2_compile_8` | `(a)(*scs:(1;2)b)` — capture-list entries separated by something other than `,` (pcre2_compile.c:2813-2817) | `NULL`, `PCRE2_ERROR_MISSING_CONDITION_CLOSING` (124) |
| 172 | `pcre2_compile_8` | `(a)(*scs:(0)b)` — capture-list group number 0 (pcre2_compile.c:2765-2769) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 173 | `pcre2_compile_8` | `(a)(*scs:(70000)b)` — capture-list group number > 65535 (pcre2_compile.c:2761) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 174 | `pcre2_compile_8` | `(a)(*scs:(<xyz>)b)` — capture-list name not defined anywhere in the pattern (pcre2_compile_cgroup.c:297) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 175 | `pcre2_compile_8` | `(a)(*scs:(2)b)` — capture-list group number greater than the total group count (pcre2_compile_cgroup.c:326) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 176 | `pcre2_compile_8` | Duplicate-name slot not found in the name table when building a `(*scs:)` capture set — dead branch (pcre2_compile_cgroup.c:235) | `NULL`, `PCRE2_ERROR_INTERNAL_MISSING_SUBPATTERN` (153) |

### pcre2_compile.c — conditional groups

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 177 | `pcre2_compile_8` | `(?(?i)a)` — `(?(` followed by something that is not an assertion (pcre2_compile.c:3546-3551) | `NULL`, `PCRE2_ERROR_CONDITION_ASSERTION_EXPECTED` (128) |
| 178 | `pcre2_compile_8` | `(?(?C1)\Qa\E)` — a non-empty `\Q…\E` where a conditional assertion was expected. NB **not** `(?(\Qa\E)b)`: `expect_cond_assert` is only set when the code unit right after `(?(` is `?` or `*` (pcre2_compile.c:5441), so a leading `\` is read as a group name and gives ERR62 (162). A callout is needed to get `expect_cond_assert` down to 1 while leaving the `\Q` as the next item (pcre2_compile.c:3432-3438) | `NULL`, `PCRE2_ERROR_CONDITION_ASSERTION_EXPECTED` (128) |
| 179 | `pcre2_compile_8` | `(?(*atomic:a)b)` — an alpha assertion that is not a plain lookaround used as a condition. NB **not** `(?((*atomic:a))b)`: the `*` must follow `(?(` directly, otherwise `expect_cond_assert` is never set and the `(` is read as a group name (ERR62, 162) (pcre2_compile.c:4792-4797) | `NULL`, `PCRE2_ERROR_CONDITION_ASSERTION_EXPECTED` (128) |
| 180 | `pcre2_compile_8` | `(?(1a)b)` — condition not terminated by `)` (also `(?(<n>x)b)`) (pcre2_compile.c:5588-5592) | `NULL`, `PCRE2_ERROR_MISSING_CONDITION_CLOSING` (124) |
| 181 | `pcre2_compile_8` | `(?(0)a)` — conditional group number 0 (also `(?(-0)a)` → see row 65) (pcre2_compile.c:5454-5458) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 182 | `pcre2_compile_8` | `(?(70000)a)` — conditional group number > 65535 (pcre2_compile.c:5450) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 183 | `pcre2_compile_8` | `(?(2)a)` — conditional group number greater than the total group count (pcre2_compile.c:6812-6818) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 184 | `pcre2_compile_8` | `(?(R70000)a)` — `R<digits>` condition whose digits exceed 65535 (pcre2_compile.c:6691-6697) | `NULL`, `PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG` (161) |
| 185 | `pcre2_compile_8` | `(?(xyz)a)` where no group is named `xyz` (and it is not `R<digits>` naming an existing group) (pcre2_compile.c:6698-6704) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 186 | `pcre2_compile_8` | `(a)(?(1)a\|b\|c)` — conditional group with more than two branches. NB group 1 must actually exist, otherwise ERR15 (115) is reported first (pcre2_compile.c:7006-7011) | `NULL`, `PCRE2_ERROR_TOO_MANY_CONDITION_BRANCHES` (127) |
| 187 | `pcre2_compile_8` | `(?(DEFINE)a\|b)` — `DEFINE` group with more than one branch (pcre2_compile.c:6989-6995) | `NULL`, `PCRE2_ERROR_DEFINE_TOO_MANY_BRANCHES` (154) |
| 188 | `pcre2_compile_8` | `(?(VERSION<=10.0)a)` — operator other than `=` or `>=` after `VERSION` (also `(?(VERSIONx)a)`) (pcre2_compile.c:5486-5491) | `NULL`, `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` (179) |
| 189 | `pcre2_compile_8` | `(?(VERSION>=1001)a)` — major version > 1000 (pcre2_compile.c:5493) | `NULL`, `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` (179) |
| 190 | `pcre2_compile_8` | `(?(VERSION>=10.x)a)` — no digit after the `.` (pcre2_compile.c:5498-5503) | `NULL`, `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` (179) |
| 191 | `pcre2_compile_8` | `(?(VERSION>=10.1001)a)` — minor version > 1000 (pcre2_compile.c:5504) | `NULL`, `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` (179) |
| 192 | `pcre2_compile_8` | `(?(VERSION>=10.0x)a)` — version condition not terminated by `)` (pcre2_compile.c:5507-5512) | `NULL`, `PCRE2_ERROR_VERSION_CONDITION_SYNTAX` (179) |

### pcre2_compile.c — callouts

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 193 | `pcre2_compile_8` | `(?C1)` with `PCRE2_EXTRA_NEVER_CALLOUT` (also `PCRE2_AUTO_CALLOUT` is unaffected; this checks only explicit `(?C`) (pcre2_compile.c:5288-5292) | `NULL`, `PCRE2_ERROR_CALLOUT_CALLER_DISABLED` (203) |
| 194 | `pcre2_compile_8` | `(?C256)` — numeric callout argument > 255 (pcre2_compile.c:5383-5387) | `NULL`, `PCRE2_ERROR_CALLOUT_NUMBER_TOO_BIG` (138) |
| 195 | `pcre2_compile_8` | `(?C1x` — callout not terminated by `)` (also `(?C{abc}x`) (pcre2_compile.c:5394-5398) | `NULL`, `PCRE2_ERROR_MISSING_CALLOUT_CLOSING` (139) |
| 196 | `pcre2_compile_8` | `(?Cxabc)` — the character after `(?C` is not one of the allowed string delimiters (` " ' ` `` ` `` `^ % # $ {`) and not a digit or `)` (pcre2_compile.c:5340-5344) | `NULL`, `PCRE2_ERROR_CALLOUT_BAD_STRING_DELIMITER` (182) |
| 197 | `pcre2_compile_8` | `(?C{abc` — string-argument callout with no terminating delimiter (pcre2_compile.c:5351-5356) | `NULL`, `PCRE2_ERROR_CALLOUT_NO_STRING_DELIMITER` (181) |
| 198 | `pcre2_compile_8` | Callout string longer than `UINT32_MAX` code units — requires a >4 GiB pattern on LP64 (pcre2_compile.c:5362-5366) | `NULL`, `PCRE2_ERROR_CALLOUT_STRING_TOO_LONG` (172) |

### pcre2_compile.c — lookbehind analysis

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 199 | `pcre2_compile_8` | `(?<=a*)b` — a lookbehind branch of unbounded length (also `(?<=\X)`, `(?<=(?R))`) (pcre2_compile.c:9947-9950, `ISNOTFIXED`) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_NOT_FIXED_LENGTH` (125) |
| 200 | `pcre2_compile_8` | `(?<=\X)b` — `get_branchlength` returns -1 for `ESC_X` without setting an error code, so `check_lookbehind` supplies the default (pcre2_compile.c:10040-10044) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_NOT_FIXED_LENGTH` (125) |
| 201 | `pcre2_compile_8` | Lookbehind whose analysis needs more than 2000 `get_branchlength` calls; one call is made per branch, so `(?<=(?:a\|a\|…\|a))` with 2101 alternatives suffices (pcre2_compile.c:9598-9603) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_TOO_COMPLICATED` (135) |
| 202 | `pcre2_compile_8` | `(?<=\C)a` with `PCRE2_UTF` (`\C` has no fixed length in UTF mode) (pcre2_compile.c:9697-9702) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_INVALID_BACKSLASH_C` (136) |
| 203 | `pcre2_compile_8` | Repetition inside a lookbehind whose `(max-1) * lastitemlength` overflows `INT_MAX`, e.g. `(?<=(?:a{65535}){65535})` (pcre2_compile.c:9931-9936) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_TOO_LONG` (187) |
| 204 | `pcre2_compile_8` | A lookbehind branch longer than `LOOKBEHIND_MAX` (65535) characters, e.g. `(?<=a{65535}a{2})` (pcre2_compile.c:9958-9962) | `NULL`, `PCRE2_ERROR_LOOKBEHIND_TOO_LONG` (187) |
| 205 | `pcre2_compile_8` | Variable-length lookbehind whose maximum exceeds `cb->max_varlookbehind` (default 255), e.g. `(?<=a{1,256})b`; also settable via `pcre2_set_max_varlookbehind_8` (pcre2_compile.c:10065-10070) | `NULL`, `PCRE2_ERROR_MAX_VAR_LOOKBEHIND_EXCEEDED` (200) |
| 206 | `pcre2_compile_8` | `\k<xyz>` / `(?&xyz)` inside a lookbehind where no group is named `xyz` (pcre2_compile.c:9780-9786) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 207 | `pcre2_compile_8` | `(?<=\2)a` / `(?<=(?2))a` inside a lookbehind referring to a group number > `bracount` (pcre2_compile.c:9827-9833) | `NULL`, `PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE` (115) |
| 208 | `pcre2_compile_8` | Unhandled META code in `parsed_skip()` — dead branch (pcre2_compile.c:9978-9981) | `NULL`, `PCRE2_ERROR_INTERNAL_BAD_CODE_IN_SKIP` (190) |
| 209 | `pcre2_compile_8` | Unrecognised META code in `check_lookbehinds()` — dead `default:` (pcre2_compile.c:10124-10127) | `NULL`, `PCRE2_ERROR_INTERNAL_BAD_CODE_LOOKBEHINDS` (170), `*erroroffset = 0` |

### pcre2_compile.c — internal / resource-exhaustion branches (`PCRE2_DEBUG_UNREACHABLE` is a no-op in this build)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 210 | `pcre2_compile_8` | Pre-compile pass writes past `cb->start_workspace + workspace_size` (`COMPILE_WORK_SIZE`) — dead branch guarded by row 211 (pcre2_compile.c:6167-6172) | `NULL`, `PCRE2_ERROR_INTERNAL_OVERRAN_WORKSPACE` (152), `*erroroffset = 0` |
| 211 | `pcre2_compile_8` | Pre-compile pass exceeds `workspace_size - WORK_SIZE_SAFETY_MARGIN` (6000-100 = 5900 code units), i.e. a single pattern item compiles to an enormous amount of code. Concretely: 1200 nested **capturing** parentheses (`"("*1200 + "a" + ")"*1200`) with `pcre2_set_parens_nest_limit_8(cc, 100000)`, since each level advances the workspace pointer by `sizeof(OP_CBRA)` = 1+LINK_SIZE+IMM2_SIZE = 5 and `code` is only rewound between *top-level* items. `(?:` cannot be used because it also consumes a `nest_save` slot (limit 375, row 133). In 8-bit mode a big character class does **not** work: wide characters always go into a separately-allocated character list, never inline into the workspace (pcre2_compile.c:6176-6181) | `NULL`, `PCRE2_ERROR_PATTERN_TOO_COMPLICATED` (186), `*erroroffset = 0` |
| 212 | `pcre2_compile_8` | `*lengthptr` accumulation overflows `OFLOW_MAX` during the pre-compile pass (item length, group length, or group replication for `{n}`/`{n,}`/`{n,m}`), e.g. `"(?:a{65535}){65535}"` (pcre2_compile.c:6198-6203, 7023-7027, 7478-7482, 7649-7653, 7699-7703) | `NULL`, `PCRE2_ERROR_PATTERN_TOO_LARGE` (120) |
| 213 | `pcre2_compile_8` | `compile_regex` branch-length accumulation overflows `OFLOW_MAX` (pcre2_compile.c:8805-8809) | `NULL`, `PCRE2_ERROR_PATTERN_TOO_LARGE` (120) |
| 214 | `pcre2_compile_8` | `parsed_pattern` write pointer reaches `parsed_pattern_end` (pre-write guard in the literal loop, the main loop, and at `PARSED_END`) — dead branch (pcre2_compile.c:3190-3195, 3264-3271, 5909-5914) | `NULL`, `PCRE2_ERROR_INTERNAL_PARSED_OVERFLOW` (163) |
| 215 | `pcre2_compile_8` | Second-pass `usedlength > length` (real compile emitted more code than the pre-compile estimate) — dead branch (pcre2_compile.c:10992-10998) | `NULL`, `PCRE2_ERROR_INTERNAL_CODE_OVERFLOW` (123), `*erroroffset = 0` |
| 216 | `pcre2_compile_8` | `PRIV(find_bracket)` fails to locate a previously-validated recursion target during the recursion-offset fixup pass — dead branch (pcre2_compile.c:11050-11055) | `NULL`, `PCRE2_ERROR_INTERNAL_MISSING_SUBPATTERN` (153) |
| 217 | `pcre2_compile_8` | `PRIV(auto_possessify)` returns non-zero (unknown opcode) — dead branch; only when `PCRE2_OPTIM_AUTO_POSSESS` is enabled (pcre2_compile.c:11087-11094) | `NULL`, `PCRE2_ERROR_INTERNAL_BAD_CODE_AUTO_POSSESS` (180), `*erroroffset = 0` |
| 218 | `pcre2_compile_8` | `PRIV(study)` returns non-zero (`SSB_UNKNOWN`, or `find_minlength` returning -2/-3) — dead branch; only when `PCRE2_OPTIM_START_OPTIMIZE` is enabled (pcre2_compile.c:11249-11257, pcre2_study.c:1938/2068/2074) | `NULL`, `PCRE2_ERROR_INTERNAL_STUDY_ERROR` (131), `*erroroffset = 0` |
| 219 | `pcre2_compile_8` | Parsed-pattern value `>= META_END` that the compile switch does not handle — dead `default:` (pcre2_compile.c:8396-8401) | `NULL`, `PCRE2_ERROR_INTERNAL_BAD_CODE` (189) |
| 220 | `pcre2_compile_8` | `(*UTF)` / `PCRE2_UTF` / `PCRE2_UCP` in a build without `SUPPORT_UNICODE` — **not compiled here** (pcre2_compile.c:10606-10612) | `NULL`, `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` (132) — unreachable in this build |
| 221 | `pcre2_compile_8` | `PCRE2_ERROR_VERB_ARGUMENT_NOT_ALLOWED` (159) — the message slot is retained but no code path sets `ERR59` anywhere in the sources (`grep 'ERR59'` finds no assignment) | never returned |

### pcre2_match.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 222 | `pcre2_match_8` | `match_data == NULL` (pcre2_match.c:7042) | `PCRE2_ERROR_NULL` (-51) |
| 223 | `pcre2_match_8` | `code == NULL` with a valid `match_data` (pcre2_match.c:7043) | `PCRE2_ERROR_NULL` (-51) |
| 224 | `pcre2_match_8` | `subject == NULL` with `length != 0`. Note `subject==NULL && length==0` is legal (remapped to an internal empty string at pcre2_match.c:7038) (pcre2_match.c:7043) | `PCRE2_ERROR_NULL` (-51) |
| 225 | `pcre2_match_8` | `options` contains a bit outside `PUBLIC_MATCH_OPTIONS` (`ANCHORED\|ENDANCHORED\|NOTBOL\|NOTEOL\|NOTEMPTY\|NOTEMPTY_ATSTART\|NO_UTF_CHECK\|PARTIAL_HARD\|PARTIAL_SOFT\|NO_JIT\|COPY_MATCHED_SUBJECT\|DISABLE_RECURSELOOP_CHECK`, pcre2_match.c:71-75), e.g. `PCRE2_CASELESS` (0x8) or `0x80000000` (pcre2_match.c:7045) | `PCRE2_ERROR_BADOPTION` (-34) |
| 226 | `pcre2_match_8` | `start_offset > length`, e.g. subject `"abc"`, `length = 3`, `start_offset = 4` (also `start_offset > strlen(subject)` when `length == PCRE2_ZERO_TERMINATED`) (pcre2_match.c:7056) | `PCRE2_ERROR_BADOFFSET` (-33) |
| 227 | `pcre2_match_8` | `re->magic_number != MAGIC_NUMBER` (0x50435245) — pass a zero-filled heap block or any non-PCRE2 buffer cast to `pcre2_code *` (pcre2_match.c:7060) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 228 | `pcre2_match_8` | `(re->flags & PCRE2_MODE_MASK) != 1` — a pattern compiled by `pcre2_compile_16`/`pcre2_compile_32` handed to `pcre2_match_8` (pcre2_match.c:7065) | `PCRE2_ERROR_BADMODE` (-32) |
| 229 | `pcre2_match_8` | `PCRE2_PARTIAL_HARD` or `PCRE2_PARTIAL_SOFT` in `options` while `PCRE2_ENDANCHORED` is set in `options` **or** in `re->overall_options` (pcre2_match.c:7111) | `PCRE2_ERROR_BADOPTION` (-34) |
| 230 | `pcre2_match_8` | `pcre2_set_offset_limit_8(mc, n)` with `n != PCRE2_UNSET` but the pattern was **not** compiled with `PCRE2_USE_OFFSET_LIMIT` (pcre2_match.c:7118) | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) |
| 231 | `pcre2_match_8` | UTF pattern, no `PCRE2_NO_UTF_CHECK`, `start_offset > 0`, and `subject[start_offset]` is a continuation byte (`(b & 0xc0) == 0x80`) — subject `"\xc3\xa9"` with `start_offset = 1` (pcre2_match.c:7295) | `PCRE2_ERROR_BADUTFOFFSET` (-36) |
| 232 | `pcre2_match_8` | UTF pattern, no `PCRE2_NO_UTF_CHECK`, `start_offset == 0`, and `subject[0]` is an isolated continuation byte — subject `"\x80abc"` (pcre2_match.c:7297) | `PCRE2_ERROR_UTF8_ERR20` (-22) |
| 233 | `pcre2_match_8` | UTF pattern, **no** `PCRE2_MATCH_INVALID_UTF`, no `PCRE2_NO_UTF_CHECK`, and malformed UTF-8 anywhere in the scanned subject — e.g. `"\xc3"` → -3, `"\xed\xa0\x80"` → -16, `"\xfe"` → -23. `match_data->startchar` is set to the absolute offset of the bad code unit (pcre2_match.c:7358) | one of `PCRE2_ERROR_UTF8_ERR1..ERR21` (-3..-23) |
| 234 | `pcre2_match_8` | Same malformed UTF-8 but the pattern **was** compiled with `PCRE2_MATCH_INVALID_UTF` — no UTF error is returned; `end_subject` is truncated at the bad code unit and matching continues fragment-by-fragment (pcre2_match.c:7358-7383, restart loop 8109-8163) | no UTF error; ends as `PCRE2_ERROR_NOMATCH` (-1), a match, or `PCRE2_ERROR_PARTIAL` (-2) |
| 235 | `pcre2_match_8` | `re->newline_convention` outside CR/LF/CRLF/ANY/ANYCRLF/NUL — only via a corrupted `pcre2_code` (pcre2_match.c:7478) | `PCRE2_ERROR_INTERNAL` (-44) |
| 236 | `pcre2_match_8` | `pcre2_set_heap_limit_8(mc, 0)` (or `(*LIMIT_HEAP=0)`) so that `1024 * heap_limit < frame_size` — fires before any matching (pcre2_match.c:7521) | `PCRE2_ERROR_HEAPLIMIT` (-63) |
| 237 | `pcre2_match_8` | `malloc` of the initial `heapframes` vector fails (general context with a failing `private_malloc`) (pcre2_match.c:7534) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 238 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` with `length != 0` and a successful match, where the subject-copy `malloc` fails (pcre2_match.c:8194) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 239 | `pcre2_match_8` | `mb->match_call_count >= mb->match_limit`: `pcre2_set_match_limit_8(mc, 1)`, or catastrophic backtracking such as `/(a+)+b/` on 30 `a`s with the default limit 10000000 (pcre2_match.c:873) | `PCRE2_ERROR_MATCHLIMIT` (-47) |
| 240 | `pcre2_match_8` | `Frdepth >= mb->match_limit_depth`: `pcre2_set_depth_limit_8(mc, 1)` with any pattern needing one backtrack frame, or deep recursion `/a(?1)?z/` on a long subject (pcre2_match.c:874) | `PCRE2_ERROR_DEPTHLIMIT` (-53) |
| 241 | `pcre2_match_8` | Frame vector must grow but `match_data->heapframes_size == PCRE2_SIZE_MAX - 1` (pcre2_match.c:768) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 242 | `pcre2_match_8` | Frame vector must grow and `newsize/1024 >= mb->heap_limit` with `heap_limit <= current_size/1024` — e.g. `pcre2_set_heap_limit_8(mc, 20)` with `/(a+)*b/` on a long `"aaaa…"` subject (pcre2_match.c:778) | `PCRE2_ERROR_HEAPLIMIT` (-63) |
| 243 | `pcre2_match_8` | After clamping frame-vector growth to the heap limit, the permitted `newsize - usedsize` is still less than one `frame_size` (pcre2_match.c:791) | `PCRE2_ERROR_HEAPLIMIT` (-63) |
| 244 | `pcre2_match_8` | `malloc` of the doubled frame vector returns NULL during deep backtracking (pcre2_match.c:793) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 245 | `pcre2_match_8` | `OP_RECURSE` re-enters the same group number at the same subject position with `last_used_ptr` unchanged, and `PCRE2_DISABLE_RECURSELOOP_CHECK` is **not** set — e.g. pattern `(?1)()` or `(a(?2))((?1))` on `"a"` (pcre2_match.c:5729) | `PCRE2_ERROR_RECURSELOOP` (-52) |
| 246 | `pcre2_match_8` | At whole-pattern success, `Fstart_match < start_subject+start_offset` or `Fstart_match > Feptr` — `\K` inside a lookaround reached via recursion, with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` set at compile time so the compile-time check (row 85) was skipped (pcre2_match.c:1030) | `PCRE2_ERROR_BAD_BACKSLASH_K` (-75) |
| 247 | `pcre2_match_8` | `OP_CLOSE` walks the `last_group_offset` chain and reaches `PCRE2_UNSET` — corrupted heapframes only (pcre2_match.c:909) | `PCRE2_ERROR_INTERNAL` (-44) |
| 248 | `pcre2_match_8` | `OP_ACCEPT` inside a recursion walks the group chain looking for `GF_RECURSE` and reaches `PCRE2_UNSET` (pcre2_match.c:951) | `PCRE2_ERROR_INTERNAL` (-44) |
| 249 | `pcre2_match_8` | End of whole-pattern recursion (`OP_BRA` with `Fcurrent_recurse == 0`) but `Flast_group_offset == PCRE2_UNSET` (pcre2_match.c:6377) | `PCRE2_ERROR_INTERNAL` (-44) |
| 250 | `pcre2_match_8` | Unrecognised `proptype` / `Lctype` in any of the 11 `OP_PROP` / char-type repeat switches (single, fixed, minimizing, maximizing × UTF/non-UTF) — corrupted bytecode only (pcre2_match.c:2876, 3229, 3507, 3762, 4051, 4208, 4355, 4626, 4947, 5207) | `PCRE2_ERROR_INTERNAL` (-44) |
| 251 | `pcre2_match_8` | Main opcode dispatch `switch(Fop)` reaches `default:` — unknown/corrupted opcode (pcre2_match.c:6889) | `PCRE2_ERROR_INTERNAL` (-44) |
| 252 | `pcre2_match_8` | `RETURN_SWITCH` on `Freturn_id` reaches `default:` — corrupted frame return id (pcre2_match.c:6941) | `PCRE2_ERROR_INTERNAL` (-44) |
| 253 | `pcre2_match_8` | No starting position yields a match: bumpalong exhausted, anchored failure, `(*COMMIT)`, `PCRE2_FIRSTLINE` newline reached, or `start_match > bumpalong_limit` — e.g. pattern `xyz` on `"abc"` (pcre2_match.c:8242) | `PCRE2_ERROR_NOMATCH` (-1) |
| 254 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT` (or `_HARD`) and `mb->hitend` set with no complete match — pattern `abcd` on `"ab"` (pcre2_match.c:8232) | `PCRE2_ERROR_PARTIAL` (-2) |
| 255 | `pcre2_match_8` | `PCRE2_PARTIAL_HARD` and end of subject reached past `start_used_ptr` (the `SCHECK_PARTIAL()` macro) — pattern `abcd` + `PCRE2_PARTIAL_HARD` on `"ab"` (pcre2_match.c:629) | `PCRE2_ERROR_PARTIAL` (-2) |
| 256 | `pcre2_match_8` | `PCRE2_PARTIAL_HARD` with `PCRE2_NEWLINE_CRLF` and the subject ending in a lone `\r`: `.`, `\z`, `\Z`, `$`, multiline `$`, or a repeated `.`/backreference stopping on that CR — e.g. pattern `.` on `"a\r"` with `PCRE2_PARTIAL_HARD` (pcre2_match.c:1070, 3279, 3535, 4110, 4241, 4740, 4992, 5401, 6596, 6615, 6625, 6663) | `PCRE2_ERROR_PARTIAL` (-2) |
| 257 | `pcre2_match_8` | Successful match but `mb->end_offset_top >= 2 * match_data->oveccount` — `pcre2_match_data_create_8(1, NULL)` with pattern `(a)(b)` on `"ab"`. **Not an error**: `rc` is clamped to 0 and `ovector[0..1]` still hold the whole match (pcre2_match.c:8180) | `0` (success, ovector too small) |
| 258 | `pcre2_match_8` | A callout set by `pcre2_set_callout_8` returns a value `> 0` (e.g. `return 1;`) — converted locally to `MATCH_NOMATCH` (pcre2_match.c:5992, 6029) | `PCRE2_ERROR_NOMATCH` (-1) |
| 259 | `pcre2_match_8` | A callout returns a **negative** value other than `PCRE2_ERROR_NOMATCH`, e.g. `return -37;` — propagated unchanged (pcre2_match.c:5993, 6030 → 8215) | the callout's own value (e.g. `PCRE2_ERROR_CALLOUT` (-37)) |
| 260 | `pcre2_match_8` | JIT-only branches (mid-character `start_offset`, isolated 0x80, `valid_utf` failure, `COPY_MATCHED_SUBJECT` malloc failure before/after a JIT run) — **not compiled** because `SUPPORT_JIT` is undefined (pcre2_match.c:7161, 7163, 7200-7205, 7225) | unreachable in this build |

### pcre2_match_next.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 261 | `pcre2_next_match_8` | The preceding `pcre2_match_8` stored a negative `match_data->rc` — any error, including `PCRE2_ERROR_NOMATCH` (-1) and `PCRE2_ERROR_PARTIAL` (-2). `*pstart_offset` / `*poptions` untouched (pcre2_match_next.c:108-109) | `FALSE` (0) |
| 262 | `pcre2_next_match_8` | Non-empty match that made no progress (`ovector[0] != start_offset && ovector[1] == start_offset`, i.e. `\K` pushed the start back) **and** `start_offset >= match_data->subject_length` (pcre2_match_next.c:133-134) | `FALSE` (0) |
| 263 | `pcre2_next_match_8` | Previous match was empty (`ovector[0] == ovector[1]`) and located at `>= match_data->subject_length` (pcre2_match_next.c:151-152) | `FALSE` (0) |
| 264 | `pcre2_next_match_8` | `PCRE2_ASSERT(ovector[1] >= start_offset)` — a `match_data` never filled by a successful match (so `ovector[1] == PCRE2_UNSET`) or hand-corrupted. **No-op** in this build (`PCRE2_DEBUG` undefined) (pcre2_match_next.c:116) | no diagnostic in this build; `abort()` if `PCRE2_DEBUG` is enabled |
| 265 | `pcre2_next_match_8` | `do_bumpalong` indexes `subject[offset]` with no bounds check when `match_data->subject`/`subject_length` are inconsistent (pcre2_match_next.c:61) | no error return; undefined behaviour |

### pcre2_study.c (reached only from `pcre2_compile_8`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 266 | `_pcre2_study_8` | `set_start_bits` returns `SSB_UNKNOWN` (3) — an opcode unknown to the start-bits scanner, or an unknown XCLASS sub-opcode (pcre2_study.c:1132, 1783 → 1938) | returns `1`; `pcre2_compile_8` maps it to `NULL` + `PCRE2_ERROR_INTERNAL_STUDY_ERROR` (131) |
| 267 | `_pcre2_study_8` | `find_minlength` returns `-2` — `_pcre2_find_bracket_8` cannot locate a group referenced by a name-table entry or a numeric backreference (pcre2_study.c:500, 560 → 2068) | returns `2`; → `NULL` + `PCRE2_ERROR_INTERNAL_STUDY_ERROR` (131) |
| 268 | `_pcre2_study_8` | `find_minlength` returns `-3` — opcode not listed in the min-length scanner, or the scan loop falls off the end (pcre2_study.c:758, 765 → 2074) | returns `3`; → `NULL` + `PCRE2_ERROR_INTERNAL_STUDY_ERROR` (131) |
| 269 | `_pcre2_study_8` | `find_minlength` returns `-1` — opcode counter > 1000, pattern contains `(*ACCEPT)`, or `\C` in UTF mode (pcre2_study.c:129, 222, 388). **Not an error** (pcre2_study.c:2063-2064) | returns `0`; `re->minlength` left at 0 |
| 270 | `_pcre2_study_8` | `set_start_bits` returns `SSB_TOODEEP` (4) at recursion depth > 1000, or `SSB_FAIL` (0) for `\C`/non-`PT_CLIST` `OP_PROP`/`OP_EXTUNI`. **Not an error** (pcre2_study.c:1106, 1208, 1223, 1623, 1720, 1735) | returns `0`; no `PCRE2_FIRSTMAPSET` flag, no start bitmap |

### pcre2_jit_compile.c (non-JIT stubs; `SUPPORT_JIT` undefined)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 271 | `pcre2_jit_compile_8` | `PCRE2_JIT_TEST_ALLOC` combined with any other bit, e.g. `PCRE2_JIT_TEST_ALLOC\|PCRE2_JIT_COMPLETE` (pcre2_jit_compile.c:14318) | `PCRE2_ERROR_JIT_BADOPTION` (-45) |
| 272 | `pcre2_jit_compile_8` | `options == PCRE2_JIT_TEST_ALLOC` exactly (checked **before** the `code == NULL` test, so `pcre2_jit_compile_8(NULL, PCRE2_JIT_TEST_ALLOC)` also lands here) (pcre2_jit_compile.c:14324) | `PCRE2_ERROR_JIT_UNSUPPORTED` (-68) |
| 273 | `pcre2_jit_compile_8` | `code == NULL` with any `options` other than exactly `PCRE2_JIT_TEST_ALLOC` (pcre2_jit_compile.c:14328) | `PCRE2_ERROR_NULL` (-51) |
| 274 | `pcre2_jit_compile_8` | `options` contains a bit outside `PUBLIC_JIT_COMPILE_OPTIONS` (`JIT_COMPLETE\|JIT_PARTIAL_SOFT\|JIT_PARTIAL_HARD\|JIT_INVALID_UTF`, pcre2_jit_compile.c:14289), e.g. `0x10` (pcre2_jit_compile.c:14331) | `PCRE2_ERROR_JIT_BADOPTION` (-45) |
| 275 | `pcre2_jit_compile_8` | Any otherwise-valid call, including `options == 0` and `options == PCRE2_JIT_COMPLETE`. Side effect first: `PCRE2_JIT_INVALID_UTF` still ORs `PCRE2_MATCH_INVALID_UTF` into `re->overall_options` (pcre2_jit_compile.c:14371) before the error is returned (pcre2_jit_compile.c:14381) | `PCRE2_ERROR_JIT_BADOPTION` (-45) |
| 276 | `pcre2_jit_match_8` | Every call, unconditionally. `match_data` is dereferenced (`match_data->rc = …`), so `match_data == NULL` is a NULL dereference rather than a clean error (pcre2_jit_match_inc.h:103) | `PCRE2_ERROR_JIT_BADOPTION` (-45), also stored in `match_data->rc` |
| 277 | `pcre2_jit_stack_create_8` | Every call, unconditionally — `startsize`, `maxsize` and `gcontext` are all discarded, so `startsize == 0`, `maxsize == 0` and `startsize > maxsize` are indistinguishable (pcre2_jit_misc_inc.h:134) | `NULL` |

**No-rejection JIT stubs** (listed for completeness, not table rows): `pcre2_jit_free_unused_memory_8`, `pcre2_jit_stack_assign_8`, `pcre2_jit_stack_free_8`, `_pcre2_jit_free_8`, `_pcre2_jit_free_rodata_8` accept every argument including `NULL` and do nothing; `_pcre2_jit_get_size_8` always returns `0` (this is what makes `PCRE2_INFO_JITSIZE` report 0); `_pcre2_jit_get_target_8` always returns the string `"JIT is not supported"`.

### pcre2_dfa_match.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 278 | `pcre2_dfa_match_8` | `match_data == NULL` (pcre2_dfa_match.c:3396) | `PCRE2_ERROR_NULL` (-51) |
| 279 | `pcre2_dfa_match_8` | `code == NULL` with a valid `match_data` (pcre2_dfa_match.c:3397) | `PCRE2_ERROR_NULL` (-51) |
| 280 | `pcre2_dfa_match_8` | `subject == NULL` with `length != 0` (`NULL` + length 0 is remapped at pcre2_dfa_match.c:3393) (pcre2_dfa_match.c:3397) | `PCRE2_ERROR_NULL` (-51) |
| 281 | `pcre2_dfa_match_8` | `workspace == NULL` (9th argument) (pcre2_dfa_match.c:3397) | `PCRE2_ERROR_NULL` (-51) |
| 282 | `pcre2_dfa_match_8` | `options` contains a bit outside `PUBLIC_DFA_MATCH_OPTIONS` (`ANCHORED\|ENDANCHORED\|NOTBOL\|NOTEOL\|NOTEMPTY\|NOTEMPTY_ATSTART\|NO_UTF_CHECK\|PARTIAL_HARD\|PARTIAL_SOFT\|DFA_SHORTEST\|DFA_RESTART\|COPY_MATCHED_SUBJECT`, pcre2_dfa_match.c:83-88) — concretely `PCRE2_NO_JIT` (0x2000), `PCRE2_DISABLE_RECURSELOOP_CHECK` (0x40000), or `0x00000200` (pcre2_dfa_match.c:3399) | `PCRE2_ERROR_BADOPTION` (-34) |
| 283 | `pcre2_dfa_match_8` | `wscount < 20`, e.g. `wscount = 19` or `0` (pcre2_dfa_match.c:3407) | `PCRE2_ERROR_DFA_WSSIZE` (-43) |
| 284 | `pcre2_dfa_match_8` | `start_offset > length`, e.g. subject `"abc"`, `length = 3`, `start_offset = 4` (pcre2_dfa_match.c:3408) | `PCRE2_ERROR_BADOFFSET` (-33) |
| 285 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_HARD`/`_SOFT` while `PCRE2_ENDANCHORED` is set in `options` or in `re->overall_options` (pcre2_dfa_match.c:3414) | `PCRE2_ERROR_BADOPTION` (-34) |
| 286 | `pcre2_dfa_match_8` | Pattern compiled with `PCRE2_MATCH_INVALID_UTF` (e.g. `pcre2_compile_8("abc", …, PCRE2_UTF\|PCRE2_MATCH_INVALID_UTF, …)`) (pcre2_dfa_match.c:3419) | `PCRE2_ERROR_DFA_UINVALID_UTF` (-66) |
| 287 | `pcre2_dfa_match_8` | `re->magic_number != MAGIC_NUMBER` (0x50435245) (pcre2_dfa_match.c:3425) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 288 | `pcre2_dfa_match_8` | `(re->flags & PCRE2_MODE_MASK) != 1` — a 16-/32-bit compiled pattern (pcre2_dfa_match.c:3430) | `PCRE2_ERROR_BADMODE` (-32) |
| 289 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` with invalid workspace contents: `(workspace[0] & ~1) != 0`, or `workspace[1] < 1` (e.g. an all-zero workspace), or `workspace[1] > (wscount-2)/INTS_PER_STATEBLOCK` (pcre2_dfa_match.c:3454-3458) | `PCRE2_ERROR_DFA_BADRESTART` (-38) |
| 290 | `pcre2_dfa_match_8` | `pcre2_set_offset_limit_8(mc, n)` with `n != PCRE2_UNSET` but the pattern was not compiled with `PCRE2_USE_OFFSET_LIMIT` (pcre2_dfa_match.c:3504) | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) |
| 291 | `pcre2_dfa_match_8` | `re->newline_convention` outside CR/LF/NUL/CRLF/ANY/ANYCRLF — corrupted `pcre2_code` only (pcre2_dfa_match.c:3574) | `PCRE2_ERROR_INTERNAL` (-44) |
| 292 | `pcre2_dfa_match_8` | UTF pattern, no `PCRE2_NO_UTF_CHECK`, `start_offset > 0`, and `subject[start_offset]` is a continuation byte — subject `"\xC3\xA9"`, `start_offset = 1` (pcre2_dfa_match.c:3598) | `PCRE2_ERROR_BADUTFOFFSET` (-36) |
| 293 | `pcre2_dfa_match_8` | UTF pattern, no `PCRE2_NO_UTF_CHECK`, subject not valid UTF-8 — `"\x80"` → -22, `"\xC3"` → -3, `"\xF5\x80\x80\x80"` → -15, `"\xC0\x80"` → -17 (pcre2_dfa_match.c:3620-3625) | one of `PCRE2_ERROR_UTF8_ERR1..ERR21` (-3..-23) |
| 294 | `pcre2_dfa_match_8` | `PCRE2_COPY_MATCHED_SUBJECT`, a successful match, and `match_data->memctl.malloc(length)` returns NULL (pcre2_dfa_match.c:4064-4068) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 295 | `pcre2_dfa_match_8` | Quantified `\C`: pattern `\C*`, `\C+`, `\C?`, `\C{3}` or `\C{2,3}` (opcode `>= OP_TYPESTAR` with data byte `OP_ANYBYTE`) (pcre2_dfa_match.c:825) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 296 | `pcre2_dfa_match_8` | Unquantified `\C`: pattern `a\Cb` or `\C` (pcre2_dfa_match.c:3258, `default:`) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 297 | `pcre2_dfa_match_8` | Any back reference: pattern `(a)\1`, `(?i)(a)\1`, `(a)\g{1}`, or `(?J)(?<n>a)(?<n>b)\k<n>` (`OP_REF`/`OP_REFI`/`OP_DNREF`/`OP_DNREFI`) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 298 | `pcre2_dfa_match_8` | `\K`: pattern `ab\Kcd` (`OP_SET_SOM`) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 299 | `pcre2_dfa_match_8` | Backtracking-control verbs: pattern `a(*MARK:X)b`, `a(*PRUNE)b`, `a(*SKIP)b`, `a(*THEN)b`, `a(*COMMIT)b` or `a(*ACCEPT)b`. Note `(*FAIL)`/`(*F)` **is** supported (pcre2_dfa_match.c:2781) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 300 | `pcre2_dfa_match_8` | Script run: pattern `(*script_run:\w+)` or `(*sr:ab)` (`OP_SCRIPT_RUN`) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 301 | `pcre2_dfa_match_8` | Non-atomic lookaround: pattern `(*napla:a)b` or `(*naplb:a)b` (`OP_ASSERT_NA`/`OP_ASSERTBACK_NA`) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 302 | `pcre2_dfa_match_8` | Scan-substring assertion: pattern `(a)(*scs:(1)b)` (`OP_ASSERT_SCS`) (pcre2_dfa_match.c:3258) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 303 | `pcre2_dfa_match_8` | `OP_RECURSE` followed by `OP_CREF`, i.e. a subroutine call with an argument list: pattern `(a)(?1(1))` (pcre2_dfa_match.c:2943) | `PCRE2_ERROR_DFA_UITEM` (-42) |
| 304 | `pcre2_dfa_match_8` | Condition on whether a group is set (`OP_CREF`/`OP_DNCREF`/`OP_DNRREF`): pattern `(a)?(?(1)b\|c)`, `(?<n>a)?(?(n)b\|c)`, or `(?J)(?<n>a)(?<n>b)(?(R&n)x\|y)` (pcre2_dfa_match.c:2856) | `PCRE2_ERROR_DFA_UCOND` (-40) |
| 305 | `pcre2_dfa_match_8` | `OP_RREF` with a value other than `RREF_ANY`, i.e. a recursion test for a specific group: pattern `(a)(?(R1)b\|c)` or `(?<n>a)(?(R&n)b\|c)`. Bare `(?(R)b\|c)` **is** supported (pcre2_dfa_match.c:2875) | `PCRE2_ERROR_DFA_UCOND` (-40) |
| 306 | `pcre2_dfa_match_8` | Nested `internal_dfa_match` for `OP_RECURSE` returns 0 because its fixed 1000-slot local ovector overflowed — pattern `(a+)(?1)` against more than 500 `a` characters (pcre2_dfa_match.c:2995) | `PCRE2_ERROR_DFA_RECURSE` (-39) |
| 307 | `pcre2_dfa_match_8` | `OP_RECURSE` repeats the same group number at the same subject position with the same `last_used_ptr` — e.g. pattern `((?2))((?1))` (pcre2_dfa_match.c:2960-2966) | `PCRE2_ERROR_RECURSELOOP` (-52) |
| 308 | `pcre2_dfa_match_8` | State-list overflow inside `internal_dfa_match` (`ADD_ACTIVE`/`ADD_ACTIVE_DATA`/`ADD_NEW`/`ADD_NEW_DATA` exceed the derived `wscount`) — call with the minimum legal `wscount = 20` and pattern `(a\|b\|c\|d\|e\|f\|g\|h\|i\|j\|k\|l\|m\|n\|o\|p)+` on `"abcdefghijklmnop"` (pcre2_dfa_match.c:496, 506, 515, 525) | `PCRE2_ERROR_DFA_WSSIZE` (-43) |
| 309 | `pcre2_dfa_match_8` | `mb->match_call_count >= mb->match_limit` (counts `internal_dfa_match` calls): `pcre2_set_match_limit_8(mc, 1)` plus a pattern with a nested construct such as `(?:(?=a)a)+` or `(a)(?1)` (pcre2_dfa_match.c:566) | `PCRE2_ERROR_MATCHLIMIT` (-47) |
| 310 | `pcre2_dfa_match_8` | `rlevel > mb->match_limit_depth`: `pcre2_set_depth_limit_8(mc, 1)` plus a nested-assertion pattern such as `(?=(?=(?=a)))a` (pcre2_dfa_match.c:567) | `PCRE2_ERROR_DEPTHLIMIT` (-53) |
| 311 | `pcre2_dfa_match_8` | `more_workspace()` cannot grow the recursion workspace within `mb->heap_limit`: `pcre2_set_heap_limit_8(mc, 0)` plus a pattern that recurses past the initial 20 KiB block, e.g. `(a)(?1)` on a long subject (pcre2_dfa_match.c:445) | `PCRE2_ERROR_HEAPLIMIT` (-63) |
| 312 | `pcre2_dfa_match_8` | `more_workspace()` heap allocation fails: `mb->memctl.malloc(newsize * sizeof(int))` returns NULL (pcre2_dfa_match.c:447) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 313 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_HARD` and `OP_EOD` (`\z`) reached at/after end of subject — pattern `abc\z`, subject `"abc"` (pcre2_dfa_match.c:962-964) | `PCRE2_ERROR_PARTIAL` (-2) |
| 314 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_HARD` and `OP_EODN` (`\Z`) reached at end of subject — pattern `abc\Z`, subject `"abc"` (pcre2_dfa_match.c:1012-1016) | `PCRE2_ERROR_PARTIAL` (-2) |
| 315 | `pcre2_dfa_match_8` | No new states but `could_continue` at end of subject with `PCRE2_PARTIAL_HARD`, or `PCRE2_PARTIAL_SOFT` with no complete match — pattern `abcd`, subject `"abc"` (pcre2_dfa_match.c:3293) | `PCRE2_ERROR_PARTIAL` (-2) |
| 316 | `pcre2_dfa_match_8` | Match found but `PCRE2_ENDANCHORED` (in `options` or `re->overall_options`) and `ptr < end_subject` — pattern `ab` + `PCRE2_ENDANCHORED`, subject `"abc"` (pcre2_dfa_match.c:3305-3308) | `PCRE2_ERROR_NOMATCH` (-1) |
| 317 | `pcre2_dfa_match_8` | Bumpalong exhausted or start optimizations prove failure (`re->minlength` > remaining subject, first/required code unit absent, `start_match > bumpalong_limit`) — pattern `xyz`, subject `"abc"` (pcre2_dfa_match.c:4110-4114) | `PCRE2_ERROR_NOMATCH` (-1) |
| 318 | `pcre2_dfa_match_8` | `match_count * 2 > offsetcount` — `pcre2_match_data_create_8(1, NULL)` with pattern `a\|ab\|abc` on `"abc"`. **Not an error**; the longest match is still in `ovector[0..1]` (pcre2_dfa_match.c:880-881) | `0` ("offsets overflowed, longest matches present") |
| 319 | `pcre2_dfa_match_8` | `pcre2_match_data_create_8(0, NULL)` — `oveccount` is clamped to 1 by `pcre2_match_data_create_8` (pcre2_match_data.c:57), so `offsetcount` is 2 and this is indistinguishable from row 318. `PCRE2_ERROR_UNSET` (-55) is **never** produced by `pcre2_dfa_match_8` (pcre2_dfa_match.c:880-887) | `0` |
| 320 | `pcre2_dfa_match_8` | A callout in a `(?(?C1)…)` condition returns a negative value — `pcre2_set_callout_8` returning e.g. -99 with an auto-callout pattern (pcre2_dfa_match.c:2845) | the callout's own negative value (verbatim) |
| 321 | `pcre2_dfa_match_8` | A callout for `OP_CALLOUT`/`OP_CALLOUT_STR` returns a negative value — pattern `a(?C1)b` with a callout returning `PCRE2_ERROR_CALLOUT` (-37) (pcre2_dfa_match.c:3250) | the callout's own negative value (e.g. -37) |
| 322 | `pcre2_dfa_match_8` | An error from a nested `internal_dfa_match` inside a lookaround / conditional assertion / `OP_RECURSE` / `OP_BRAPOS` / `OP_ONCE` — e.g. pattern `(?=\C)a` yields -42 from the inner call (pcre2_dfa_match.c:2822, 2921, 3025, 3083, 3236) | the nested error code, verbatim |
| 323 | `pcre2_dfa_match_8` | `PCRE2_ERROR_DFA_UFUNC` (-41) is **not** returned by `pcre2_dfa_match_8`; it is returned when a DFA-produced `match_data` (`matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER`) is later passed to `pcre2_substring_get_bynumber_8` / `_copy_bynumber_8` / `_length_bynumber_8` (pcre2_substring.c:75, 163, 270) or `pcre2_substitute_8` (pcre2_substitute.c:850) | `PCRE2_ERROR_DFA_UFUNC` (-41) from those functions |

### pcre2_substitute.c

On every `PTREXIT` error (-35/-49/-54/-55/-57/-58/-59/-76) `*blength` is set to
`ptr - replacement`, i.e. the offset in the **replacement** string where the
problem was found (pcre2_substitute.c:1790-1791). Early errors set
`*blength = PCRE2_UNSET` (pcre2_substitute.c:781).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 324 | `pcre2_substitute_8` | `PCRE2_PARTIAL_HARD` or `PCRE2_PARTIAL_SOFT` without `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` (pcre2_substitute.c:796) | `PCRE2_ERROR_BADOPTION` (-34) |
| 325 | `pcre2_substitute_8` | `replacement == NULL` with `rlength != 0` (pcre2_substitute.c:802) | `PCRE2_ERROR_NULL` (-51) |
| 326 | `pcre2_substitute_8` | `subject == NULL` with `length != 0` (pcre2_substitute.c:813) | `PCRE2_ERROR_NULL` (-51) |
| 327 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` set but `match_data == NULL` (pcre2_substitute.c:827) | `PCRE2_ERROR_NULL` (-51) |
| 328 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a `match_data` whose stored `rc` is a real error (not `NOMATCH`), e.g. left over from a `pcre2_match_8` that returned -36 (pcre2_substitute.c:845) | `match_data->rc` verbatim |
| 329 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a `match_data` produced by `pcre2_dfa_match_8` (pcre2_substitute.c:849) | `PCRE2_ERROR_DFA_UFUNC` (-41) |
| 330 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` where `code != match_data->code`: match with pattern A, substitute with pattern B and A's match data (pcre2_substitute.c:852) | `PCRE2_ERROR_DIFFSUBSPATTERN` (-71) |
| 331 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` where the subject pointer or length differ from the recorded match: `pcre2_match_8(re,"abc",3,…)` then substitute with a different buffer pointer or a different `length` (pcre2_substitute.c:858-864) | `PCRE2_ERROR_DIFFSUBSSUBJECT` (-72) |
| 332 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` where `start_offset != match_data->start_offset`: match at 0, substitute at 1 (pcre2_substitute.c:866) | `PCRE2_ERROR_DIFFSUBSOFFSET` (-73) |
| 333 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` where the non-substitute, non-`NO_UTF_CHECK` match options differ from those recorded: match with `PCRE2_NOTBOL`, substitute without it (pcre2_substitute.c:869) | `PCRE2_ERROR_DIFFSUBSOPTIONS` (-74) |
| 334 | `pcre2_substitute_8` | `match_data == NULL` and `pcre2_match_data_create_from_pattern_8` fails (failing allocator) (pcre2_substitute.c:894) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 335 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` and the internal `pcre2_match_data_create_8(match_data->oveccount, …)` fails (pcre2_substitute.c:908) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 336 | `pcre2_substitute_8` | UTF pattern, no `PCRE2_NO_UTF_CHECK`, and the **replacement** string is invalid UTF-8, e.g. `replacement = "\x80"` or `"\xC3"` (pcre2_substitute.c:940-944) | one of `PCRE2_ERROR_UTF8_ERR1..ERR21` (-3..-23) |
| 337 | `pcre2_substitute_8` | `start_offset > length`: subject `"abc"`, `length = 3`, `start_offset = 4` (pcre2_substitute.c:957) | `PCRE2_ERROR_BADOFFSET` (-33) |
| 338 | `pcre2_substitute_8` | An `options` bit that survives the substitute-bit stripping but is not in `PUBLIC_MATCH_OPTIONS`, e.g. `PCRE2_DFA_SHORTEST` (0x80) or `PCRE2_DFA_RESTART` (0x40) — propagated from the internal `pcre2_match_8` (pcre2_substitute.c:996 → pcre2_match.c:7045) | `PCRE2_ERROR_BADOPTION` (-34) |
| 339 | `pcre2_substitute_8` | Any other `pcre2_match_8` error propagated verbatim: invalid UTF subject (-3..-23), `BADMAGIC` (-31), `BADMODE` (-32), `BADUTFOFFSET` (-36), `MATCHLIMIT` (-47), `NOMEMORY` (-48), `DEPTHLIMIT` (-53), `HEAPLIMIT` (-63), `BAD_BACKSLASH_K` (-75) (pcre2_substitute.c:996) | the `pcre2_match_8` code, verbatim |
| 340 | `pcre2_substitute_8` | `PCRE2_PARTIAL_HARD\|PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` and the match is partial — pattern `abcd`, subject `"abc"` (pcre2_substitute.c:996) | `PCRE2_ERROR_PARTIAL` (-2) |
| 341 | `pcre2_substitute_8` | After a match, `ovector[1] < ovector[0]` or `ovector[0] < start_offset` — `\K` made the match end before it starts or start before the current scan point. **Verified witness:** pattern `(?<=\Ka)b` compiled with `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` (without it `pcre2_compile_8` rejects `\K` in a lookaround; with it `pcre2_match_8` no longer returns -75 either), subject `"aab"`, `start_offset = 2`. (`a\K` with `PCRE2_SUBSTITUTE_GLOBAL` does **not** reach this branch: `\K` after a one-code-unit match leaves `ovector[0] == start_offset`, and `pcre2_substitute_8("aaa", "a\K", "X", GLOBAL)` returns 3.) (pcre2_substitute.c:1000-1004) | `PCRE2_ERROR_BADSUBSPATTERN` (-60) |
| 342 | `pcre2_substitute_8` | Global loop makes no progress and the match is not empty-after-non-empty — internal invariant, `PCRE2_DEBUG_UNREACHABLE` (pcre2_substitute.c:1014-1022) | `PCRE2_ERROR_INTERNAL_DUPMATCH` (-65) |
| 343 | `pcre2_substitute_8` | `subs == INT_MAX` (2147483647 substitutions already made) with `PCRE2_SUBSTITUTE_GLOBAL` — needs a > 2 GiB subject (pcre2_substitute.c:1030-1035) | `PCRE2_ERROR_TOOMANYREPLACE` (-61) |
| 344 | `pcre2_substitute_8` | Replacement ends immediately after `$`: `"$"` or `"abc$"` (pcre2_substitute.c:1110) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 345 | `pcre2_substitute_8` | Replacement ends after `${`: `"${"` (pcre2_substitute.c:1221) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 346 | `pcre2_substitute_8` | Replacement ends after `$<`: `"$<"` (pcre2_substitute.c:1230) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 347 | `pcre2_substitute_8` | Replacement ends after `${*`: `"${*"` (pcre2_substitute.c:1237) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 348 | `pcre2_substitute_8` | `read_name_subst()` fails — empty name, first character not a word character, or name longer than `MAX_NAME_SIZE`, which is **128** (config.h:223), not 32: `"$-"`, `"$ "`, `"${}"`, `"${:"`, `"$<>"`, `"${*}"`, or `"${"` + **129** `a`s + `"}"` (a 35-character name is accepted) (pcre2_substitute.c:1274) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 349 | `pcre2_substitute_8` | `$<name` with no closing `>`: `"$<name"` or `"$<name}"` (pcre2_substitute.c:1327) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 350 | `pcre2_substitute_8` | `${*name}` where `name` is not exactly `MARK` (case-sensitive): `"${*FOO}"` or `"${*mark}"` (pcre2_substitute.c:1353) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 351 | `pcre2_substitute_8` | More than 10 levels of nested `${name:+…}` / `${name:-…}` (`ptrstackptr >= PTR_STACK_SIZE` = 20, 2 slots per level) with `PCRE2_SUBSTITUTE_EXTENDED`, e.g. 11 nested `${1:-…}` (pcre2_substitute.c:1433) | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 352 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `PRIV(check_escape)` rejects a backslash sequence in the replacement: `"\q"`, `"\y"`, `"\x{110000}"`, `"\o{}"`, `"\c"` at end, `"\N{U+41}"` without `PCRE2_UTF` (pcre2_substitute.c:1545) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 353 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `\g` not followed by `<`: `"\g"` at end, or `"\gA"` (pcre2_substitute.c:1583, via pcre2_compile.c:1759-1762) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 354 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `\g<` followed by an invalid/empty name: `"\g<>"` or `"\g<->"` (pcre2_substitute.c:1588) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 355 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `\g<name` with no closing `>`: `"\g<name"` (pcre2_substitute.c:1593) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 356 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `\g<1a` — number read but no closing `>` (pcre2_compile.c:1777, `ERR119` mapped through the substitute path) (pcre2_substitute.c:1545) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 357 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `\g<70000>` — group number > `MAX_GROUP_NUMBER` (pcre2_compile.c:1767) (pcre2_substitute.c:1545) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 358 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and a character-class/assertion escape in the replacement: `"\d"`, `"\w"`, `"\s"`, `"\A"`, `"\z"`, `"\Z"`, `"\B"`, `"\R"`, `"\X"`, `"\C"`, `"\K"`, `"\p{L}"`, `"\h"`, `"\N"` (pcre2_substitute.c:1611) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 359 | `pcre2_substitute_8` | `find_text_end()` hits an invalid escape inside an extended `${name:+…}` / `${name:-…}` body: `"${1:-\q}"`, `"${1:+\d:x}"` (pcre2_substitute.c:135-138, 164-170) | `PCRE2_ERROR_BADREPESCAPE` (-57) |
| 360 | `pcre2_substitute_8` | `${…}` group reference not closed by `}`: `"${1"`, `"${name"`, `"${1x"`, `"${name)"` (pcre2_substitute.c:1316) | `PCRE2_ERROR_REPMISSINGBRACE` (-58) |
| 361 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and `find_text_end()` reaches the end of the replacement without the terminating `}`: `"${1:-abc"`, `"${1:+abc"`, `"${1:+set:unset"` (pcre2_substitute.c:175) | `PCRE2_ERROR_REPMISSINGBRACE` (-58) |
| 362 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` and the character after `:` in `${name:…}` is neither `+` nor `-`: `"${1:xy}"`, `"${1:=ab}"`, `"${name:?ab}"`, and also `"${1:x}"` (6 code units: the `ptr < repend - 2` guard **does** pass, so this yields -59, not -58). It is `"${1:x"` and `"${1:"` — no closing brace — that are too short for the guard and yield -58 instead. (pcre2_substitute.c:1294) | `PCRE2_ERROR_BADSUBSTITUTION` (-59) |
| 363 | `pcre2_substitute_8` | `$+` (highest captured group) in a pattern with no capture groups, without `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` — pattern `abc`, replacement `"$+"` (pcre2_substitute.c:1191) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 364 | `pcre2_substitute_8` | Numeric group reference > `code->top_bracket` without `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` — pattern `(a)`, replacement `"$2"` or `"${99}"` (pcre2_substitute.c:1263) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 365 | `pcre2_substitute_8` | Named group reference to a name absent from the pattern's name table, without `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` — pattern `(?<a>x)`, replacement `"${b}"` / `"$<b>"` / `"\g<b>"` (pcre2_substitute.c:1369-1378) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 366 | `pcre2_substitute_8` | `$+` where `match_data->oveccount < code->top_bracket + 1` — pattern `(a)(b)(c)` with `pcre2_match_data_create_8(2, NULL)`, replacement `"$+"` (pcre2_substitute.c:1200) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 367 | `pcre2_substitute_8` | `$+` where every capture group is unset and `PCRE2_SUBSTITUTE_UNSET_EMPTY` is not set — pattern `a(x)?`, subject `"a"`, replacement `"$+"` (pcre2_substitute.c:1210) | `PCRE2_ERROR_UNSET` (-55) |
| 368 | `pcre2_substitute_8` | Reference to an existing but unset group, plain substitution, without `PCRE2_SUBSTITUTE_UNSET_EMPTY` — pattern `a(x)?`, subject `"a"`, replacement `"$1"` or `"${1}"` (pcre2_substitute.c:1405-1416) | `PCRE2_ERROR_UNSET` (-55) |
| 369 | `pcre2_substitute_8` | Reference to a non-existent group **with** `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` but **without** `PCRE2_SUBSTITUTE_UNSET_EMPTY` — pattern `(a)`, replacement `"$2"`: -49 is converted to -55 then still errors (pcre2_substitute.c:1409-1416) | `PCRE2_ERROR_UNSET` (-55) |
| 370 | `pcre2_substitute_8` | `$'` (text after the match) with partial matching (`PCRE2_PARTIAL_HARD\|PCRE2_SUBSTITUTE_REPLACEMENT_ONLY`) (pcre2_substitute.c:1153) | `PCRE2_ERROR_PARTIALSUBS` (-76) |
| 371 | `pcre2_substitute_8` | `$_` (entire input string) with partial matching (pcre2_substitute.c:1170) | `PCRE2_ERROR_PARTIALSUBS` (-76) |
| 372 | `pcre2_substitute_8` | Output buffer too small **without** `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` — pattern `a`, subject `"aaa"`, replacement `"bbbb"`, `*blength = 4`; also fires when `*blength` is exactly the result length with no room for the terminating NUL (pcre2_substitute.c:649, 675) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 373 | `pcre2_substitute_8` | Output buffer too small **with** `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` — the required size is written to `*blength` and the same code is returned (pcre2_substitute.c:1748-1752) | `PCRE2_ERROR_NOMEMORY` (-48), `*blength` = required length |
| 374 | `pcre2_substitute_8` | A `substitute_case_callout` set by `pcre2_set_substitute_case_callout_8` returns `PCRE2_SIZE_MAX` (`~(PCRE2_SIZE)0`) — replacement `"\U$1"` / `"\u$1"` with `PCRE2_SUBSTITUTE_EXTENDED` (pcre2_substitute.c:527, 583, 708) | `PCRE2_ERROR_REPLACECASE` (-69) |
| 375 | `pcre2_substitute_8` | `PCRE2_SIZE` overflow while accumulating `extra_needed` with `PCRE2_SUBSTITUTE_GLOBAL\|PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` (pcre2_substitute.c:643, 695, 723, 1701, 1752). Real subjects can never accumulate that much; the reachable witness is a `substitute_case_callout` that claims to need `~(PCRE2_SIZE)0 - 1` code units, e.g. pattern `(a)`, subject `"ax"`, replacement `"\Uabc"`, `PCRE2_SUBSTITUTE_EXTENDED\|PCRE2_SUBSTITUTE_OVERFLOW_LENGTH`, `*blength = 32`, which trips the check at :1752 | `PCRE2_ERROR_TOOLARGEREPLACE` (-70) |
| 376 | `pcre2_substitute_8` | `mcontext->substitute_callout` returns non-zero, rejecting a substitution. **Not an error**; a negative return additionally clears `PCRE2_SUBSTITUTE_GLOBAL` (pcre2_substitute.c:1655-1668) | no error; `rc` = number of substitutions actually made |
| 377 | `pcre2_substitute_8` | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) is **not** reachable here: `pcre2_substring_nametable_scan_8` is called with non-NULL `firstptr`/`lastptr`, and that code requires `firstptr == NULL` (pcre2_substring.c:517) (pcre2_substitute.c:1371-1395) | never returned by `pcre2_substitute_8` |
| 378 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_LITERAL` makes the whole replacement literal, so none of rows 344-362 can fire — replacement `"${"` with `PCRE2_SUBSTITUTE_LITERAL` succeeds and inserts `${` verbatim (pcre2_substitute.c:1054-1057) | no error; `rc >= 0` |
| 379 | `pcre2_substitute_8` | Without `PCRE2_SUBSTITUTE_EXTENDED`, backslash is a literal, so `"\q"` succeeds (inserts `\q`) and `"${1:-x}"` yields -58 rather than being parsed as an extended construct (pcre2_substitute.c:1290-1291, 1472-1473) | no error; `rc >= 0` |

### pcre2_substring.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 380 | `pcre2_substring_copy_byname_8` | `match_data` was populated by `pcre2_dfa_match_8` (`matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER`) (pcre2_substring.c:74) | `PCRE2_ERROR_DFA_UFUNC` (-41) |
| 381 | `pcre2_substring_copy_byname_8` | `stringname` absent from the pattern's name table — pattern `(?<abc>a)`, ask for `"xyz"` (pcre2_substring.c:78) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 382 | `pcre2_substring_copy_byname_8` | Name exists but every name-table entry for it has group number `>= match_data->oveccount` — pattern `(a)(b)(?<n>c)` with `pcre2_match_data_create_8(1, NULL)` (pcre2_substring.c:79, 90) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 383 | `pcre2_substring_copy_byname_8` | Name in range but unset — pattern `(?<n>a)\|b` matched against `"b"` (`ovector[2] == PCRE2_UNSET`) (pcre2_substring.c:87, 90) | `PCRE2_ERROR_UNSET` (-55) |
| 384 | `pcre2_substring_copy_byname_8` | Buffer too small — propagated from `pcre2_substring_copy_bynumber_8` (see row 386) (pcre2_substring.c:92) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 385 | `pcre2_substring_copy_bynumber_8` | Any error from `pcre2_substring_length_bynumber_8` is propagated verbatim (rows 393-399) (pcre2_substring.c:122) | the propagated code (e.g. -49, -54, -55, -2, -1) |
| 386 | `pcre2_substring_copy_bynumber_8` | Buffer too small: `*sizeptr <= size` where `size` is the captured length (needs `size+1` for the NUL) — pattern `(abc)` on `"abc"`, group 1, `*sizeptr = 3`. Neither `buffer` nor `*sizeptr` is modified (pcre2_substring.c:124) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 387 | `pcre2_substring_get_byname_8` | `match_data` was populated by `pcre2_dfa_match_8` (pcre2_substring.c:162) | `PCRE2_ERROR_DFA_UFUNC` (-41) |
| 388 | `pcre2_substring_get_byname_8` | `stringname` absent from the pattern's name table (pcre2_substring.c:166) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 389 | `pcre2_substring_get_byname_8` | Name exists but all its group numbers are `>= match_data->oveccount` (pcre2_substring.c:167, 178) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 390 | `pcre2_substring_get_byname_8` | Name in range but `ovector[n*2] == PCRE2_UNSET` — pattern `(?<n>a)\|b` on `"b"`; `*stringptr`/`*sizeptr` untouched (pcre2_substring.c:175, 178) | `PCRE2_ERROR_UNSET` (-55) |
| 391 | `pcre2_substring_get_bynumber_8` | Any error from `pcre2_substring_length_bynumber_8`, propagated verbatim; `*stringptr`/`*sizeptr` untouched (pcre2_substring.c:211) | the propagated code |
| 392 | `pcre2_substring_get_bynumber_8` | `_pcre2_memctl_malloc_8(sizeof(pcre2_memctl) + size + 1, …)` returns NULL — build the `match_data` from a general context whose `private_malloc` fails (pcre2_substring.c:213) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 393 | `pcre2_substring_length_bynumber_8` | `match_data->rc == PCRE2_ERROR_PARTIAL` (last match used `PCRE2_PARTIAL_SOFT`/`_HARD` and returned -2) with `stringnumber > 0` (pcre2_substring.c:317) | `PCRE2_ERROR_PARTIAL` (-2) |
| 394 | `pcre2_substring_length_bynumber_8` | `match_data->rc < 0` and not `PCRE2_ERROR_PARTIAL` — e.g. the last `pcre2_match_8` returned `PCRE2_ERROR_NOMATCH` (-1) or `PCRE2_ERROR_MATCHLIMIT` (-47) (pcre2_substring.c:322) | `match_data->rc` verbatim |
| 395 | `pcre2_substring_length_bynumber_8` | Non-DFA match and `stringnumber > match_data->code->top_bracket` — pattern `(a)` (top_bracket 1), ask for group 2 (pcre2_substring.c:326) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 396 | `pcre2_substring_length_bynumber_8` | Non-DFA match, `stringnumber <= top_bracket` but `>= match_data->oveccount` — pattern `(a)(b)` with `pcre2_match_data_create_8(2, NULL)`, ask for group 2 (pcre2_substring.c:328) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 397 | `pcre2_substring_length_bynumber_8` | Non-DFA match, group in range but `ovector[stringnumber*2] == PCRE2_UNSET` — pattern `(a)\|b` on `"b"`, ask for group 1 (pcre2_substring.c:330) | `PCRE2_ERROR_UNSET` (-55) |
| 398 | `pcre2_substring_length_bynumber_8` | DFA match and `stringnumber >= match_data->oveccount` — `pcre2_dfa_match_8` with `pcre2_match_data_create_8(1, NULL)`, ask for group 1 (pcre2_substring.c:335) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 399 | `pcre2_substring_length_bynumber_8` | DFA match, `match_data->rc != 0` and `stringnumber >= (uint32_t)match_data->rc` — `pcre2_dfa_match_8` on pattern `abc` (rc == 1) with oveccount 4, ask for group 1 (pcre2_substring.c:336) | `PCRE2_ERROR_UNSET` (-55) |
| 400 | `pcre2_substring_length_bynumber_8` | `ovector[n*2] > match_data->subject_length` or `ovector[n*2+1] > match_data->subject_length` — reachable only by hand-writing the ovector obtained from `pcre2_get_ovector_pointer_8` (pcre2_substring.c:344-348) | `PCRE2_ERROR_INVALIDOFFSET` (-67) |
| 401 | `pcre2_substring_length_byname_8` | `match_data` was populated by `pcre2_dfa_match_8` (pcre2_substring.c:269) | `PCRE2_ERROR_DFA_UFUNC` (-41) |
| 402 | `pcre2_substring_length_byname_8` | `stringname` absent from the pattern's name table (pcre2_substring.c:273) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 403 | `pcre2_substring_length_byname_8` | Name exists but all its group numbers are `>= match_data->oveccount` (pcre2_substring.c:274, 285) | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 404 | `pcre2_substring_length_byname_8` | Name in range but `ovector[n*2] == PCRE2_UNSET`; `*sizeptr` untouched (pcre2_substring.c:282, 285) | `PCRE2_ERROR_UNSET` (-55) |
| 405 | `pcre2_substring_list_get_8` | `match_data->rc < 0` — the stored code is returned verbatim and `*listptr`/`*lengthsptr` are untouched (pcre2_substring.c:389) | `match_data->rc` verbatim (e.g. -1, -2) |
| 406 | `pcre2_substring_list_get_8` | `_pcre2_memctl_malloc_8` returns NULL for the single combined block (pcre2_substring.c:403) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 407 | `pcre2_substring_nametable_scan_8` | `firstptr == NULL` and `lastptr == NULL`, the name is found, but it has more than one table entry — pattern `(?J)(?<n>a)\|(?<n>b)` scanned for `"n"` (pcre2_substring.c:516) | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) |
| 408 | `pcre2_substring_nametable_scan_8` | Binary chop exhausts without a match, including `code->name_count == 0` (pattern `(a)` scanned for `"n"`). `*firstptr`/`*lastptr` are not written (pcre2_substring.c:494-525) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 409 | `pcre2_substring_number_from_name_8` | `stringname` absent from the pattern's name table (pcre2_substring.c:550) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 410 | `pcre2_substring_number_from_name_8` | Name present more than once under `PCRE2_DUPNAMES`, e.g. `(?J)(?<n>a)\|(?<n>b)` with `"n"` (pcre2_substring.c:550) | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) |
| 411 | `pcre2_substring_free_8` | `string == NULL` — guarded no-op (pcre2_substring.c:239) | `void`, no error |
| 412 | `pcre2_substring_list_free_8` | `list == NULL` — guarded no-op (pcre2_substring.c:453) | `void`, no error |

### pcre2_match_data.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 413 | `pcre2_match_data_create_8` | `oveccount == 0` — silently raised to 1; **not** an error (pcre2_match_data.c:57) | no error; `pcre2_get_ovector_count_8` afterwards returns 1 |
| 414 | `pcre2_match_data_create_8` | `oveccount > UINT16_MAX`, e.g. 100000 or `UINT32_MAX` — silently clamped to 65535; **not** an error (pcre2_match_data.c:58) | no error; `pcre2_get_ovector_count_8` returns 65535 |
| 415 | `pcre2_match_data_create_8` | `_pcre2_memctl_malloc_8(offsetof(pcre2_match_data, ovector) + 2*oveccount*sizeof(PCRE2_SIZE), gcontext)` fails — `gcontext` created by `pcre2_general_context_create_8` with a `private_malloc` returning NULL (pcre2_match_data.c:59-62) | `NULL` (no error code) |
| 416 | `pcre2_match_data_create_from_pattern_8` | `code == NULL` (checked before `gcontext` is examined) (pcre2_match_data.c:84) | `NULL` |
| 417 | `pcre2_match_data_create_from_pattern_8` | Inner `pcre2_match_data_create_8(top_bracket+1, gcontext)` allocation fails; with `gcontext == NULL` the allocator comes from `code` itself (pcre2_match_data.c:85-87) | `NULL` |
| 418 | `pcre2_match_data_free_8` | `match_data == NULL` — guarded no-op (pcre2_match_data.c:99) | `void`, no error |

**No-validation accessors** (not table rows): `pcre2_get_mark_8`, `pcre2_get_ovector_pointer_8`, `pcre2_get_ovector_count_8`, `pcre2_get_startchar_8`, `pcre2_get_match_data_size_8`, `pcre2_get_match_data_heapframes_size_8` all dereference `match_data` unconditionally — `NULL` is a segfault, never an error return (pcre2_match_data.c:120, 132, 144, 156, 168, 181).

### pcre2_pattern_info.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 419 | `pcre2_pattern_info_8` | `where != NULL`, valid 8-bit `code`, and `what` outside 0..26 — e.g. `what = 27`, `100`, or `0xFFFFFFFF` (note `what` is `uint32_t`, so `-1` becomes 4294967295). `*where` is never written (pcre2_pattern_info.c:242) | `PCRE2_ERROR_BADOPTION` (-34) |
| 420 | `pcre2_pattern_info_8` | `where == NULL` (length query) with `what` outside the recognised set and a valid `code` — the length switch falls through to the main switch `default:` (pcre2_pattern_info.c:66-105 → 242) | `PCRE2_ERROR_BADOPTION` (-34) |
| 421 | `pcre2_pattern_info_8` | `code == NULL` with `where != NULL`, any `what` — e.g. `pcre2_pattern_info_8(NULL, PCRE2_INFO_SIZE, &sz)`. Converse: `pcre2_pattern_info_8(NULL, PCRE2_INFO_SIZE, NULL)` returns `sizeof(size_t)` = 8 and does **not** error, because the length switch runs first (pcre2_pattern_info.c:107) | `PCRE2_ERROR_NULL` (-51) |
| 422 | `pcre2_pattern_info_8` | `code == NULL` **and** `where == NULL` **and** `what` unrecognised (e.g. 27) — falls out of the length switch into the NULL test (pcre2_pattern_info.c:66-105 → 107) | `PCRE2_ERROR_NULL` (-51) |
| 423 | `pcre2_pattern_info_8` | `re->magic_number != MAGIC_NUMBER` (0x50435245) — a zero-filled heap block or a raw serialized byte stream cast to `pcre2_code *` (pcre2_pattern_info.c:112) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 424 | `pcre2_pattern_info_8` | Magic OK but `(re->flags & 1) == 0` — a pattern compiled by `pcre2_compile_16`/`pcre2_compile_32` (pcre2_pattern_info.c:116) | `PCRE2_ERROR_BADMODE` (-32) |
| 425 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_MATCHLIMIT` (14) on a pattern with no `(*LIMIT_MATCH=n)` and no `pcre2_set_match_limit_8` on the compile context (`re->limit_match == UINT32_MAX`). `*(uint32_t*)where` **is** written with `UINT32_MAX` before returning (pcre2_pattern_info.c:209-210) | `PCRE2_ERROR_UNSET` (-55), `*where = 4294967295` |
| 426 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_DEPTHLIMIT` (21) on a pattern with no depth limit set (pcre2_pattern_info.c:141-142) | `PCRE2_ERROR_UNSET` (-55), `*where = 4294967295` |
| 427 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_HEAPLIMIT` (25) on a pattern with no heap limit set (pcre2_pattern_info.c:178-179) | `PCRE2_ERROR_UNSET` (-55), `*where = 4294967295` |
| 428 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_JITSIZE` (10) — `SUPPORT_JIT` undefined, so `(size_t)0` is always written, even after a `pcre2_jit_compile_8` attempt (pcre2_pattern_info.c:186-193) | `0` (success), `*(size_t*)where = 0` |
| 429 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_FIRSTBITMAP` (7) on a pattern with `PCRE2_FIRSTMAPSET` clear, e.g. `abc` (pcre2_pattern_info.c:159-161) | `0` (success), `*(const uint8_t**)where = NULL` |
| 430 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_FIRSTCODETYPE` (6) with neither `PCRE2_FIRSTSET` nor `PCRE2_STARTLINE`, e.g. `[ab]c` or `\d+` (pcre2_pattern_info.c:149-151) | `0` (success), `*(uint32_t*)where = 0` |
| 431 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_FIRSTCODEUNIT` (5) with `PCRE2_FIRSTSET` clear, e.g. `[ab]c` (pcre2_pattern_info.c:154-156) | `0` (success), `*(uint32_t*)where = 0` |
| 432 | `pcre2_pattern_info_8` | `what == PCRE2_INFO_LASTCODETYPE` (12) or `PCRE2_INFO_LASTCODEUNIT` (11) with `PCRE2_LASTSET` clear, e.g. `a.*` (pcre2_pattern_info.c:195-201) | `0` (success), `*(uint32_t*)where = 0` |
| 433 | `pcre2_callout_enumerate_8` | `code == NULL` (checked before magic/mode) (pcre2_pattern_info.c:276) | `PCRE2_ERROR_NULL` (-51) |
| 434 | `pcre2_callout_enumerate_8` | `re->magic_number != MAGIC_NUMBER` (pcre2_pattern_info.c:285) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 435 | `pcre2_callout_enumerate_8` | `(re->flags & 1) == 0` — a 16-/32-bit pattern (pcre2_pattern_info.c:289) | `PCRE2_ERROR_BADMODE` (-32) |
| 436 | `pcre2_callout_enumerate_8` | The user `callback` returns non-zero at an `OP_CALLOUT` (pattern `a(?C1)b`) or an `OP_CALLOUT_STR` (pattern `a(?C{txt})b`); enumeration stops immediately (pcre2_pattern_info.c:405, 418) | the callback's own non-zero `int`, verbatim |

### pcre2_context.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 437 | `pcre2_set_bsr_8` | `value` other than `PCRE2_BSR_UNICODE` (1) or `PCRE2_BSR_ANYCRLF` (2) — test 0, 3, 4, `0xFFFFFFFF` (pcre2_context.c:344) | `PCRE2_ERROR_BADDATA` (-29) |
| 438 | `pcre2_set_newline_8` | `newline` outside 1..6 (`CR`=1, `LF`=2, `CRLF`=3, `ANY`=4, `ANYCRLF`=5, `NUL`=6) — test 0, 7, 8, `0xFFFFFFFF` (pcre2_context.c:377) | `PCRE2_ERROR_BADDATA` (-29) |
| 439 | `pcre2_set_optimize_8` | `ccontext == NULL` (the only setter in the file with a NULL check) (pcre2_context.c:414) | `PCRE2_ERROR_NULL` (-51) |
| 440 | `pcre2_set_optimize_8` | `directive` in the gap 2..63, i.e. above `PCRE2_OPTIMIZATION_FULL` (1) but below `PCRE2_AUTO_POSSESS` (64) — test 2, 3, 63 (pcre2_context.c:438) | `PCRE2_ERROR_BADOPTION` (-34) |
| 441 | `pcre2_set_optimize_8` | `directive > PCRE2_START_OPTIMIZE_OFF` (69) — test 70, 1000, `0xFFFFFFFF`. The accepted set is exactly {0, 1, 64, 65, 66, 67, 68, 69} (pcre2_context.c:438) | `PCRE2_ERROR_BADOPTION` (-34) |
| 442 | `pcre2_set_glob_separator_8` | `separator` not one of `'/'` (47), `'\\'` (92) or `'.'` (46) — test 0, 44, 45, 48, 58, 97, 256, `0xFFFFFFFF` (pcre2_context.c:531) | `PCRE2_ERROR_BADDATA` (-29) |
| 443 | `pcre2_set_glob_escape_8` | `escape > 255` — test 256, 1000, `0xFFFFFFFF` (pcre2_context.c:550) | `PCRE2_ERROR_BADDATA` (-29) |
| 444 | `pcre2_set_glob_escape_8` | `escape != 0`, `escape <= 255`, but not an ASCII punctuation character (`strchr(globpunct, escape) == NULL`). Accepted: 0 (disables escaping) plus 33-47, 58-64, 91-96, 123-126. Rejected: 1-32, 48-57, 65-90, 97-122, 127-255 — test 32, 48, 65, 97, 127, 200 (pcre2_context.c:550) | `PCRE2_ERROR_BADDATA` (-29) |
| 445 | `_pcre2_memctl_malloc_8` | `memctl->malloc` (or plain `malloc` when `memctl == NULL`) returns NULL for the requested size (pcre2_context.c:87) | `NULL` |
| 446 | `pcre2_general_context_create_8` | A non-NULL user `private_malloc` returns NULL for `sizeof(pcre2_real_general_context)`. Passing `NULL` for `private_malloc`/`private_free` is legal (defaults are substituted) (pcre2_context.c:118) | `NULL` |
| 447 | `pcre2_compile_context_create_8` | `gcontext` whose `malloc` returns NULL for `sizeof(pcre2_real_compile_context)` (pcre2_context.c:152) | `NULL` |
| 448 | `pcre2_match_context_create_8` | `gcontext` whose `malloc` returns NULL for `sizeof(pcre2_real_match_context)` (pcre2_context.c:188) | `NULL` |
| 449 | `pcre2_convert_context_create_8` | `gcontext` whose `malloc` returns NULL for `sizeof(pcre2_real_convert_context)` (pcre2_context.c:218) | `NULL` |
| 450 | `pcre2_general_context_copy_8` | Source context's `memctl.malloc` returns NULL. `gcontext == NULL` is an unchecked dereference, not an error (pcre2_context.c:236) | `NULL` |
| 451 | `pcre2_compile_context_copy_8` | Source context's `memctl.malloc` returns NULL; `ccontext == NULL` is UB (pcre2_context.c:248) | `NULL` |
| 452 | `pcre2_match_context_copy_8` | Source context's `memctl.malloc` returns NULL; `mcontext == NULL` is UB (pcre2_context.c:260) | `NULL` |
| 453 | `pcre2_convert_context_copy_8` | Source context's `memctl.malloc` returns NULL; `ccontext == NULL` is UB (pcre2_context.c:272) | `NULL` |
| 454 | `pcre2_general_context_free_8` / `pcre2_compile_context_free_8` / `pcre2_match_context_free_8` / `pcre2_convert_context_free_8` | Argument is `NULL` — guarded no-op; the custom `free` is **not** called (pcre2_context.c:285, 293, 301, 309) | `void`, no error |

**Setters with NO rejection path** (not table rows; they always return 0 and store the value verbatim, so out-of-range values are only diagnosed later, if at all): `pcre2_set_character_tables_8` (accepts `NULL` and garbage pointers, pcre2_context.c:329), `pcre2_set_max_pattern_length_8` (:351), `pcre2_set_max_pattern_compiled_length_8` (:358), `pcre2_set_max_varlookbehind_8` (:384), `pcre2_set_parens_nest_limit_8` (:391), `pcre2_set_compile_extra_options_8` (:398 — invalid extra bits become row 5), `pcre2_set_compile_recursion_guard_8` (:406), `pcre2_set_callout_8` (:450), `pcre2_set_substitute_callout_8` (:460), `pcre2_set_substitute_case_callout_8` (:471), `pcre2_set_heap_limit_8` (:479), `pcre2_set_match_limit_8` (:486), `pcre2_set_depth_limit_8` (:493), `pcre2_set_offset_limit_8` (:500), `pcre2_set_recursion_limit_8` (:510), `pcre2_set_recursion_memory_management_8` (:518, a complete no-op).

### pcre2_config.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 455 | `pcre2_config_8` | `where == NULL` (length query) and `what` outside 0..16. The only valid `what` values are 0=`BSR`, 1=`JIT`, 2=`JITTARGET`, 3=`LINKSIZE`, 4=`MATCHLIMIT`, 5=`NEWLINE`, 6=`PARENSLIMIT`, 7=`DEPTHLIMIT`, 8=`STACKRECURSE`, 9=`UNICODE`, 10=`UNICODE_VERSION`, 11=`VERSION`, 12=`HEAPLIMIT`, 13=`NEVER_BACKSLASH_C`, 14=`COMPILED_WIDTHS`, 15=`TABLES_LENGTH`, 16=`EFFECTIVE_LINKSIZE` — test 17, 18, 100, `0xFFFFFFFF` (pcre2_config.c:77) | `PCRE2_ERROR_BADOPTION` (-34) |
| 456 | `pcre2_config_8` | `where != NULL` and `what` outside 0..16 — second, independent `default:` in the value-producing switch (pcre2_config.c:107) | `PCRE2_ERROR_BADOPTION` (-34) |
| 457 | `pcre2_config_8` | `what == PCRE2_CONFIG_JITTARGET` (2) with **either** `where == NULL` or `where != NULL` — `SUPPORT_JIT` is undefined, so the `#else` arm rejects unconditionally and there is no way to obtain a JIT-target length (pcre2_config.c:98-101, 160) | `PCRE2_ERROR_BADOPTION` (-34) |
| 458 | `pcre2_config_8` | `what == PCRE2_CONFIG_UNICODE_VERSION` (10) or `PCRE2_CONFIG_VERSION` (11) with a `where` buffer smaller than the returned length (7 and 21 code units respectively) — there is **no** size parameter and **no** truncation check, so this is a silent buffer overflow, never an error code (pcre2_config.c:198-207, 236-243) | no error branch; UB. Returns 7 / 21 |
| 459 | `pcre2_config_8` | Buffer too small or misaligned for the 14 integer queries (`what` in {0,1,3,4,5,6,7,8,9,12,13,14,15,16}) with `where != NULL` — the code does `*((uint32_t *)where) = …` unconditionally (pcre2_config.c:80-94) | no error branch; UB. The length query returns 4 |

### pcre2_serialize.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 460 | `pcre2_serialize_encode_8` | `codes == NULL` (with `number_of_codes = 1` and valid `serialized_bytes`/`serialized_size`) (pcre2_serialize.c:85) | `PCRE2_ERROR_NULL` (-51) |
| 461 | `pcre2_serialize_encode_8` | `serialized_bytes == NULL` (pcre2_serialize.c:85) | `PCRE2_ERROR_NULL` (-51) |
| 462 | `pcre2_serialize_encode_8` | `serialized_size == NULL` (pcre2_serialize.c:85) | `PCRE2_ERROR_NULL` (-51) |
| 463 | `pcre2_serialize_encode_8` | `number_of_codes <= 0` with all three pointers non-NULL — test 0, -1, `INT32_MIN` (pcre2_serialize.c:88) | `PCRE2_ERROR_BADDATA` (-29) |
| 464 | `pcre2_serialize_encode_8` | Some `codes[i] == NULL`, e.g. `codes = {valid, NULL}` with `number_of_codes = 2` (pcre2_serialize.c:96) | `PCRE2_ERROR_NULL` (-51) |
| 465 | `pcre2_serialize_encode_8` | `codes[i]->magic_number != MAGIC_NUMBER` (0x50435245) — a `pcre2_match_data` or a zero-filled block cast to `pcre2_code *`, or a real code with the `uint32_t` at offset 88 corrupted (pcre2_serialize.c:98) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 466 | `pcre2_serialize_encode_8` | Two or more codes whose `tables` pointers differ — compile A with a default compile context and B with a context whose tables came from `pcre2_maketables_8()`, then pass `{A, B}` (pcre2_serialize.c:101) | `PCRE2_ERROR_MIXEDTABLES` (-30) |
| 467 | `pcre2_serialize_encode_8` | `memctl->malloc` returns NULL for `total_size + sizeof(pcre2_memctl)` (pcre2_serialize.c:107) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 468 | `pcre2_serialize_decode_8` | `bytes == NULL` with a valid `codes` array and `number_of_codes >= 1` (pcre2_serialize.c:176) | `PCRE2_ERROR_NULL` (-51) |
| 469 | `pcre2_serialize_decode_8` | `codes == NULL` with a valid serialized stream (pcre2_serialize.c:176) | `PCRE2_ERROR_NULL` (-51) |
| 470 | `pcre2_serialize_decode_8` | Caller's `number_of_codes <= 0` — test 0, -1 (pcre2_serialize.c:177) | `PCRE2_ERROR_BADDATA` (-29) |
| 471 | `pcre2_serialize_decode_8` | Stream's own `number_of_codes <= 0` — overwrite the `int32_t` at stream offset 12 with 0 or a negative value (header: magic@0, version@4, config@8, number_of_codes@12) (pcre2_serialize.c:178) | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) |
| 472 | `pcre2_serialize_decode_8` | `data->magic != 0x50523253` — overwrite stream bytes 0..3 (e.g. an all-zero header) (pcre2_serialize.c:179) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 473 | `pcre2_serialize_decode_8` | `data->version != (PCRE2_MAJOR \| (PCRE2_MINOR << 16))` = `10 \| (48 << 16)` = 3145738 (0x0030000A) — overwrite stream bytes 4..7 with e.g. 0x0027000A, keeping magic and config correct (pcre2_serialize.c:180) | `PCRE2_ERROR_BADMODE` (-32) |
| 474 | `pcre2_serialize_decode_8` | `data->config != (sizeof(PCRE2_UCHAR) \| (sizeof(void*) << 8) \| (sizeof(PCRE2_SIZE) << 16))` = `1 \| (8 << 8) \| (8 << 16)` = 526337 (0x00080801) on LP64 — overwrite stream bytes 8..11 with e.g. 0x00080802 (pcre2_serialize.c:181) | `PCRE2_ERROR_BADMODE` (-32) |
| 475 | `pcre2_serialize_decode_8` | The per-code `blocksize` read out of the stream is `<= sizeof(pcre2_real_code)` (152 on LP64 x86-64) — overwrite the `PCRE2_SIZE` at stream offset `16 + 1088 + 72` = 1176 with 0, 1, or 152. Goes through `cleanup:`, which frees the tables and any already-decoded codes and sets `codes[j] = NULL` (pcre2_serialize.c:209-212) | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) |
| 476 | `pcre2_serialize_decode_8` | `memctl->malloc` returns NULL for the tables copy (`TABLES_LENGTH + sizeof(PCRE2_SIZE)` = 1096 bytes) (pcre2_serialize.c:191) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 477 | `pcre2_serialize_decode_8` | `_pcre2_memctl_malloc_8(blocksize, gcontext)` returns NULL for one of the code blocks — a counting allocator that succeeds for the 1096-byte tables request and fails on the next call. Goes through `cleanup:` (pcre2_serialize.c:217-222) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 478 | `pcre2_serialize_decode_8` | Copied `dst_re->magic_number != MAGIC_NUMBER` — with a correct header and a plausible `blocksize` (> 152), corrupt the `uint32_t` at stream offset `16 + 1088 + 88` = 1192 (pcre2_serialize.c:229-234) | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) |
| 479 | `pcre2_serialize_decode_8` | Copied `dst_re->name_entry_size > MAX_NAME_SIZE + IMM2_SIZE + 1` = 131 — set the `uint16_t` at stream offset `16 + 1088 + 142` = 1246 to 132 or 65535 (pcre2_serialize.c:230-234) | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) |
| 480 | `pcre2_serialize_decode_8` | Copied `dst_re->name_count > MAX_NAME_COUNT` (10000) — set the `uint16_t` at stream offset `16 + 1088 + 144` = 1248 to 10001 or 65535 (pcre2_serialize.c:231-234) | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) |
| 481 | `pcre2_serialize_get_number_of_codes_8` | `bytes == NULL` (pcre2_serialize.c:272) | `PCRE2_ERROR_NULL` (-51) |
| 482 | `pcre2_serialize_get_number_of_codes_8` | `data->magic != 0x50523253` — e.g. a 16-byte all-zero buffer (pcre2_serialize.c:273) | `PCRE2_ERROR_BADMAGIC` (-31) |
| 483 | `pcre2_serialize_get_number_of_codes_8` | `data->version != 3145738` with magic correct (pcre2_serialize.c:274) | `PCRE2_ERROR_BADMODE` (-32) |
| 484 | `pcre2_serialize_get_number_of_codes_8` | `data->config != 526337` (LP64) with magic and version correct (pcre2_serialize.c:275) | `PCRE2_ERROR_BADMODE` (-32) |
| 485 | `pcre2_serialize_get_number_of_codes_8` | Unlike `pcre2_serialize_decode_8`, `data->number_of_codes` is **not** validated: a header with correct magic/version/config but `number_of_codes` = 0 or negative is returned verbatim (pcre2_serialize.c:277) | `data->number_of_codes` (raw `int32_t`, may be `<= 0`) |
| 486 | `pcre2_serialize_free_8` | `bytes == NULL` — guarded no-op; the hidden `pcre2_memctl` at `bytes - 24` is not touched (pcre2_serialize.c:288) | `void`, no error |
| 487 | `pcre2_serialize_free_8` | A non-NULL pointer that did **not** come from `pcre2_serialize_encode_8`: the function reads a `pcre2_memctl` from `bytes - 24` and calls the function pointer found there — no validation branch exists (pcre2_serialize.c:290-291) | no error path; UB |
| 488 | `pcre2_maketables_8` | Allocation of `TABLES_LENGTH` (1088) bytes fails — a `gcontext` whose `memctl.malloc` returns NULL (with `gcontext == NULL` plain `malloc` is used) (pcre2_maketables.c:94) | `NULL` |

### pcre2_error.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 489 | `pcre2_get_error_message_8` | `size == 0`, with any `enumber` and any `buffer` (even `NULL`) — checked before anything is written (pcre2_error.c:339) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 490 | `pcre2_get_error_message_8` | `enumber` in 0..99, i.e. `0 <= enumber < COMPILE_ERROR_BASE` — the "invalid error number" arm selects an empty one-entry list `"\0"` with `n = 1`, so the counting loop hits the terminator immediately. Test 0, 1, 50, 99 with `size >= 1` (pcre2_error.c:351-360) | `PCRE2_ERROR_BADDATA` (-29) |
| 491 | `pcre2_get_error_message_8` | `enumber >= 221`, i.e. above `PCRE2_ERROR_NULL_ERROROFFSET` (220 = index 120 of `compile_error_texts`) — test 221, 300, 1000, `INT_MAX` (pcre2_error.c:357-361) | `PCRE2_ERROR_BADDATA` (-29) |
| 492 | `pcre2_get_error_message_8` | `enumber <= -77`, i.e. below `PCRE2_ERROR_PARTIALSUBS` (-76 = index 76 of `match_error_texts`) — test -77, -100, -1000, `INT_MIN` (pcre2_error.c:357-361) | `PCRE2_ERROR_BADDATA` (-29) |
| 493 | `pcre2_get_error_message_8` | Buffer too small for a valid error number (`size - 1 <= strlen(message)`), e.g. `enumber = -1` (`"no match"`, 8 chars) with `size` in 1..8. The buffer is still filled with the truncated prefix and NUL-terminated at `buffer[size-1]` (pcre2_error.c:363-371, 382-383) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 494 | `pcre2_get_error_message_8` | `buffer == NULL` with `size > 0` — `buffer[i] = …` runs unconditionally; there is no NULL check (pcre2_error.c:370, 382) | no error path; UB |

### pcre2_convert.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 495 | `pcre2_pattern_convert_8` | `pattern == NULL` with `plength != 0` (e.g. `plength = 1` or `PCRE2_ZERO_TERMINATED`). Note `pattern==NULL && plength==0` is legal (pcre2_convert.c:1132) | `PCRE2_ERROR_NULL` (-51), `*bufflenptr = 0` |
| 496 | `pcre2_pattern_convert_8` | `bufflenptr == NULL` with a valid pattern (pcre2_convert.c:1132) | `PCRE2_ERROR_NULL` (-51) |
| 497 | `pcre2_pattern_convert_8` | `options` contains a bit outside `ALL_OPTIONS` (0x7F), e.g. `PCRE2_CONVERT_GLOB\|0x00000080` or `\|0x80000000` (pcre2_convert.c:1138) | `PCRE2_ERROR_BADOPTION` (-34), `*bufflenptr = 0` |
| 498 | `pcre2_pattern_convert_8` | More than one type bit in `TYPE_OPTIONS` (0x1C), e.g. `PCRE2_CONVERT_POSIX_BASIC\|PCRE2_CONVERT_POSIX_EXTENDED` (0x0C) or `PCRE2_CONVERT_GLOB\|PCRE2_CONVERT_POSIX_BASIC` (0x14) (pcre2_convert.c:1139) | `PCRE2_ERROR_BADOPTION` (-34), `*bufflenptr = 0` |
| 499 | `pcre2_pattern_convert_8` | No type bit set: `options == 0`, or `PCRE2_CONVERT_UTF` (0x01) alone, or `PCRE2_CONVERT_NO_UTF_CHECK` (0x02) alone (pcre2_convert.c:1140) | `PCRE2_ERROR_BADOPTION` (-34), `*bufflenptr = 0` |
| 500 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` without `PCRE2_CONVERT_NO_UTF_CHECK` and an invalid UTF-8 pattern, e.g. `options = PCRE2_CONVERT_GLOB\|PCRE2_CONVERT_UTF` with pattern byte `0xFF` (pcre2_convert.c:1162-1167) | one of `PCRE2_ERROR_UTF8_ERR1..ERR21` (-3..-23), `*bufflenptr` = the UTF error offset |
| 501 | `pcre2_pattern_convert_8` | Second-pass output-buffer allocation fails: `buffptr != NULL`, `*buffptr == NULL`, and the convert context's `malloc` returns NULL (pcre2_convert.c:1218-1224) | `PCRE2_ERROR_NOMEMORY` (-48), `*bufflenptr = 0` |
| 502 | `pcre2_pattern_convert_8` | `switch(pattype)` `default:` (already validated at :1138-1144) or fall-out past the two-iteration `for` loop — dead branches (pcre2_convert.c:1203-1206, 1233-1235) | `PCRE2_ERROR_INTERNAL` (-44) |
| 503 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC` or `_EXTENDED` with a pattern ending in a lone backslash, e.g. `"a\\"` (pcre2_convert.c:303) | `PCRE2_ERROR_END_BACKSLASH` (101), `*bufflenptr = plength` |
| 504 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC`/`_EXTENDED` with an unterminated character class, e.g. `"[abc"`, `"["`, `"[[:alpha:"` (pcre2_convert.c:378-379) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106), `*bufflenptr = plength` |
| 505 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC`/`_EXTENDED` with a caller-supplied buffer too small — `*buffptr` non-NULL and `*bufflenptr` smaller than the needed size (the `PUTCHARS`/`COPY_SPECIAL`/`ESCAPE_LITERAL` macros and the literal-copy paths all check `p + n > endp`). Smallest case: any pattern with `*bufflenptr = 1`, which cannot hold the leading `(*NUL)` (pcre2_convert.c:172, 208, 224, 241, 242, 253, 291, 297, 308, 309, 339, 366, 370) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 506 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with `"["` as the last character (pcre2_convert.c:651-655) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106), `*bufflenptr = 1` |
| 507 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with a glob ending right after a class negator: `"[!"` or `"[^"` (pcre2_convert.c:662-666) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106), `*bufflenptr = 2` |
| 508 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with an unterminated bracket expression: `"[abc"`, `"[a-"`, or `"[a\\"` (pcre2_convert.c:727, 755, 786, 802-803) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106), `*bufflenptr` = offset reached |
| 509 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with a POSIX class as the upper end of a range: `"[a-[:digit:]]"` (pcre2_convert.c:762-766) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64), `*bufflenptr = 4` |
| 510 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with an out-of-order range (`prev_c > c`): `"[z-a]"` (pcre2_convert.c:768-772) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64), `*bufflenptr = 4` |
| 511 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with a glob ending in the escape character (default `'\\'` on non-Windows): `"a\\"` (pcre2_convert.c:1052-1058) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64), `*bufflenptr = 2` |
| 512 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with a caller-supplied buffer too small: `*buffptr` non-NULL with `*bufflenptr = 1` and pattern `"a"` (pcre2_convert.c:1082-1083) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 513 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB\|PCRE2_CONVERT_UTF` with a glob separator or escape `>= 128` — unreachable via the public API because `pcre2_set_glob_separator_8`/`pcre2_set_glob_escape_8` reject those (rows 442-444) (pcre2_convert.c:868-873) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64) — unreachable via the public API |
| 514 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` with a malformed POSIX class inside a bracket expression: `"[[:alph"`, `"[[:alpha]]"`, `"[[:bogus:]]"` — `convert_glob_parse_class` returns 0 (not an error), `[` is then treated literally and the class ends up unterminated (pcre2_convert.c:511, 520, 527 → 803) | typically `PCRE2_ERROR_MISSING_SQUARE_BRACKET` (106) |
| 515 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` in a build without `SUPPORT_UNICODE` — **not compiled here** (pcre2_convert.c:1156) | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` (132) — unreachable in this build |
| 516 | `pcre2_converted_pattern_free_8` | `converted == NULL` — guarded no-op. A pointer not obtained from `pcre2_pattern_convert_8` reads a bogus `pcre2_memctl` at `converted - sizeof(pcre2_memctl)`; no validation branch exists (pcre2_convert.c:1250-1259) | `void`, no error |

### pcre2_valid_utf.c

`PRIV(valid_utf)` is exported as `_pcre2_valid_utf_8`. It is reached from the
public API by: `pcre2_compile_8` with `PCRE2_UTF` and no `PCRE2_NO_UTF_CHECK`
(row 11); `pcre2_match_8` (row 233) and `pcre2_dfa_match_8` (row 293) with
`PCRE2_UTF` and no `PCRE2_NO_UTF_CHECK`; `pcre2_substitute_8` on the replacement
(row 336); and `pcre2_pattern_convert_8` with `PCRE2_CONVERT_UTF` (row 500).
Offsets below assume the bad sequence starts at offset 0.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 517 | `_pcre2_valid_utf_8` | 2-byte lead byte `0xC2` as the final byte of the string (`ab == 1`, 0 bytes remain). `*erroroffset` = offset of the lead byte (pcre2_valid_utf.c:160) | `PCRE2_ERROR_UTF8_ERR1` (-3) |
| 518 | `_pcre2_valid_utf_8` | 3-byte lead byte `0xE1` as the final byte (`ab == 2`, 0 remain). `*erroroffset` = offset of the lead byte (pcre2_valid_utf.c:161) | `PCRE2_ERROR_UTF8_ERR2` (-4) |
| 519 | `_pcre2_valid_utf_8` | 4-byte lead byte `0xF0` as the final byte, or `0xF8 0x80` at the end (`ab == 3`, 0 remain) (pcre2_valid_utf.c:162) | `PCRE2_ERROR_UTF8_ERR3` (-5) |
| 520 | `_pcre2_valid_utf_8` | 5-byte lead byte `0xF8` as the final byte, or `0xFC 0x80` at the end (`ab == 4`, 0 remain) (pcre2_valid_utf.c:163) | `PCRE2_ERROR_UTF8_ERR4` (-6) |
| 521 | `_pcre2_valid_utf_8` | 6-byte lead byte `0xFC` as the final byte (`ab == 5`, 0 remain) (pcre2_valid_utf.c:164) | `PCRE2_ERROR_UTF8_ERR5` (-7) |
| 522 | `_pcre2_valid_utf_8` | 2nd byte not `10xxxxxx`: bytes `0xC2 0x41`. `*erroroffset` = `(p-string)-1` = offset of the `0xC2` (pcre2_valid_utf.c:174) | `PCRE2_ERROR_UTF8_ERR6` (-8) |
| 523 | `_pcre2_valid_utf_8` | 3rd byte not `10xxxxxx`: bytes `0xE1 0x80 0x41` (also from the 4-, 5- and 6-byte arms, e.g. `0xF0 0x90 0x41 0x80`). `*erroroffset` = `(p-string)-2` (pcre2_valid_utf.c:201, 223, 254, 280) | `PCRE2_ERROR_UTF8_ERR7` (-9) |
| 524 | `_pcre2_valid_utf_8` | 4th byte not `10xxxxxx`: bytes `0xF0 0x90 0x80 0x41` (also from the 5- and 6-byte arms). `*erroroffset` = `(p-string)-3` (pcre2_valid_utf.c:228, 259, 285) | `PCRE2_ERROR_UTF8_ERR8` (-10) |
| 525 | `_pcre2_valid_utf_8` | 5th byte not `10xxxxxx`: bytes `0xF8 0x88 0x80 0x80 0x41` (also from the 6-byte arm). `*erroroffset` = `(p-string)-4` (pcre2_valid_utf.c:264, 290) | `PCRE2_ERROR_UTF8_ERR9` (-11) |
| 526 | `_pcre2_valid_utf_8` | 6th byte not `10xxxxxx`: bytes `0xFC 0x84 0x80 0x80 0x80 0x41`. `*erroroffset` = `(p-string)-5` (pcre2_valid_utf.c:295) | `PCRE2_ERROR_UTF8_ERR10` (-12) |
| 527 | `_pcre2_valid_utf_8` | Well-formed but RFC-3629-forbidden 5-byte character: bytes `0xF8 0x88 0x80 0x80 0x80` (`ab == 4`, not overlong because `0x88 & 0x38 != 0`). `*erroroffset` = `(p-string)-ab` (pcre2_valid_utf.c:312) | `PCRE2_ERROR_UTF8_ERR11` (-13) |
| 528 | `_pcre2_valid_utf_8` | Well-formed but RFC-3629-forbidden 6-byte character: bytes `0xFC 0x84 0x80 0x80 0x80 0x80` (`ab == 5`, not overlong because `0x84 & 0x3C != 0`) (pcre2_valid_utf.c:312) | `PCRE2_ERROR_UTF8_ERR12` (-14) |
| 529 | `_pcre2_valid_utf_8` | 4-byte character above U+10FFFF: bytes `0xF5 0x80 0x80 0x80` (`c > 0xF4`), or `0xF4 0x90 0x80 0x80` (`c == 0xF4 && d > 0x8F`). `*erroroffset` = `(p-string)-3` (pcre2_valid_utf.c:238) | `PCRE2_ERROR_UTF8_ERR13` (-15) |
| 530 | `_pcre2_valid_utf_8` | 3-byte encoding of a surrogate: bytes `0xED 0xA0 0x80` (U+D800; `c == 0xED && d >= 0xA0`). `*erroroffset` = `(p-string)-2` (pcre2_valid_utf.c:211) | `PCRE2_ERROR_UTF8_ERR14` (-16) |
| 531 | `_pcre2_valid_utf_8` | Overlong 2-byte sequence: bytes `0xC0 0x80` or `0xC1 0xBF` (test `(c & 0x3E) == 0`). `*erroroffset` = `(p-string)-1` (pcre2_valid_utf.c:189) | `PCRE2_ERROR_UTF8_ERR15` (-17) |
| 532 | `_pcre2_valid_utf_8` | Overlong 3-byte sequence: bytes `0xE0 0x80 0x80` (`c == 0xE0 && (d & 0x20) == 0`). `*erroroffset` = `(p-string)-2` (pcre2_valid_utf.c:206) | `PCRE2_ERROR_UTF8_ERR16` (-18) |
| 533 | `_pcre2_valid_utf_8` | Overlong 4-byte sequence: bytes `0xF0 0x80 0x80 0x80` (`c == 0xF0 && (d & 0x30) == 0`). `*erroroffset` = `(p-string)-3` (pcre2_valid_utf.c:233) | `PCRE2_ERROR_UTF8_ERR17` (-19) |
| 534 | `_pcre2_valid_utf_8` | Overlong 5-byte sequence: bytes `0xF8 0x80 0x80 0x80 0x80` (`c == 0xF8 && (d & 0x38) == 0`). Reachable despite the "won't ever occur" comment, because this test precedes the `ab > 3` test. `*erroroffset` = `(p-string)-4` (pcre2_valid_utf.c:269) | `PCRE2_ERROR_UTF8_ERR18` (-20) |
| 535 | `_pcre2_valid_utf_8` | Overlong 6-byte sequence: bytes `0xFC 0x80 0x80 0x80 0x80 0x80` (`c == 0xFC && (d & 0x3C) == 0`). `*erroroffset` = `(p-string)-5` (pcre2_valid_utf.c:300) | `PCRE2_ERROR_UTF8_ERR19` (-21) |
| 536 | `_pcre2_valid_utf_8` | Isolated continuation byte — any byte in `0x80`..`0xBF` not preceded by a lead byte, e.g. a single `0x80` or `0xBF`. `*erroroffset` = `p - string` (pcre2_valid_utf.c:145) | `PCRE2_ERROR_UTF8_ERR20` (-22) |
| 537 | `_pcre2_valid_utf_8` | Illegal byte `0xFE` or `0xFF` anywhere, e.g. a single `0xFF`. `*erroroffset` = `p - string` (pcre2_valid_utf.c:151) | `PCRE2_ERROR_UTF8_ERR21` (-23) |

### Other files — no rejection paths

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 538 | `_pcre2_ord2utf_8` | No validation at all: a `cvalue` whose `(int)` cast exceeds `0x7FFFFFFF` falls out of the `utf8_table1` scan at `i == 0`, and surrogates / values > 0x10FFFF are happily encoded. Callers must pre-validate (pcre2_ord2utf.c:77-95) | no error path; returns 1..6 (code units written) |
| 539 | `_pcre2_is_newline_8` / `_pcre2_was_newline_8` | No error path. `FALSE` is the normal "not a newline" result. Callers must guarantee `ptr < endptr` / `ptr > startptr` and, in UTF mode, character-boundary alignment — none of these is checked (pcre2_newline.c:74-141, 164-237) | no error path; `FALSE` |
| 540 | `_pcre2_strcmp_8` / `_pcre2_strcmp_c8_8` / `_pcre2_strncmp_8` / `_pcre2_strncmp_c8_8` / `_pcre2_strlen_8` / `_pcre2_strcpy_c8_8` | No error paths; `-1`/`1` are ordering results. Non-zero-terminated input or an undersized destination reads/writes out of bounds undetected (pcre2_string_utils.c:63-197) | no error path |
| 541 | `_pcre2_ckd_smul_8` | `HAVE_BUILTIN_MUL_OVERFLOW` is undefined and `INT64_OR_DOUBLE` is `int64_t`, so the test is `sizeof(int64_t) > sizeof(PCRE2_SIZE) && m > PCRE2_SIZE_MAX`. On a 64-bit host the first conjunct is false, so overflow is **never** reported; on a 32-bit host `a = b = 100000` fires. The caller maps `TRUE` to `ERR20` (pcre2_chkdint.c:82-85 → pcre2_compile.c:7480, 7651, 7701) | `TRUE` → `PCRE2_ERROR_PATTERN_TOO_LARGE` (120); never `TRUE` on a 64-bit host |
| 542 | `pcre2_maketables_free_8` | No guard for `tables == NULL`: with `gcontext == NULL` it calls `free(NULL)` (harmless), but with a non-NULL `gcontext` it calls the user's `free` with a NULL block (pcre2_maketables.c:170-173) | `void`, no error return |
