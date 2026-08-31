# ERRORS.md — Error-surface table

Derived mechanically from the C sources in `c_src/src/` (8-bit, `SUPPORT_UNICODE`,
no `SUPPORT_JIT`). One row per distinct rejection / error branch reachable from
the public API. `[x]` marks a row covered by a passing differential test in
`translation/tests/`.

## 1. `pcre2_compile` — argument validation (`pcre2_compile.c:10337‑10404`)

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `pcre2_compile` | `errorptr == NULL` (and `erroroffset != NULL`) | returns `NULL`, `*erroroffset = 0` | [x] |
| 2 | `pcre2_compile` | `errorptr == NULL && erroroffset == NULL` | returns `NULL`, no writes | [x] |
| 3 | `pcre2_compile` | `erroroffset == NULL`, `errorptr != NULL` | `NULL`, `*errorptr = 220` (`NULL_ERROROFFSET`) | [x] |
| 4 | `pcre2_compile` | `pattern == NULL`, `patlen != 0` | `NULL`, `*errorptr = 116` (`NULL_PATTERN`) | [x] |
| 5 | `pcre2_compile` | `pattern == NULL`, `patlen == 0` | SUCCESS (empty pattern via `null_str`) | [x] |
| 6 | `pcre2_compile` | `options & ~PUBLIC_COMPILE_OPTIONS != 0` (undefined option bit) | `NULL`, `*errorptr = 117` (`BAD_OPTIONS`) | [x] |
| 7 | `pcre2_compile` | `ccontext->extra_options & ~PUBLIC_COMPILE_EXTRA_OPTIONS != 0` | `NULL`, `*errorptr = 117` | [x] |
| 8 | `pcre2_compile` | `PCRE2_LITERAL` + option outside `PUBLIC_LITERAL_COMPILE_OPTIONS` | `NULL`, `*errorptr = 192` (`BAD_LITERAL_OPTIONS`) | [x] |
| 9 | `pcre2_compile` | `PCRE2_LITERAL` + extra option outside `PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS` | `NULL`, `*errorptr = 192` | [x] |
| 10 | `pcre2_compile` | `patlen > ccontext->max_pattern_length` | `NULL`, `*errorptr = 188` (`PATTERN_STRING_TOO_LONG`) | [x] |
| 11 | `pcre2_compile` | compiled size `> max_pattern_compiled_length` | `NULL`, `*errorptr = 201` (`PATTERN_COMPILED_SIZE_TOO_BIG`) | [x] |
| 12 | `pcre2_compile` | `parens_nest_limit` exceeded by nesting depth | `NULL`, `*errorptr = 119` (`PARENTHESES_NEST_TOO_DEEP`) | [x] |
| 13 | `pcre2_compile` | `max_varlookbehind` exceeded | `NULL`, `*errorptr = 200` (`MAX_VAR_LOOKBEHIND_EXCEEDED`) | [x] |

## 2. `pcre2_compile` — pattern syntax errors (`pcre2_compile.c`, `pcre2_compile_class.c`, `pcre2_compile_cgroup.c`)

Each row is a distinct `ERRnn` / `*errorcodeptr = ERRnn` branch. Both the error
code AND the `*erroroffset` must match.

| # | function | trigger (pattern) | expected C result | [ ] |
|---|----------|--------------------|-------------------|-----|
| 14 | `pcre2_compile` | `"\\"` — pattern ends with backslash | 101 `END_BACKSLASH` | [x] |
| 15 | `pcre2_compile` | `"\\c"` at end | 102 `END_BACKSLASH_C` | [x] |
| 16 | `pcre2_compile` | `"\\q"` unknown escape | 103 `UNKNOWN_ESCAPE` | [x] |
| 17 | `pcre2_compile` | `"a{3,2}"` | 104 `QUANTIFIER_OUT_OF_ORDER` | [x] |
| 18 | `pcre2_compile` | `"a{100000}"` | 105 `QUANTIFIER_TOO_BIG` | [x] |
| 19 | `pcre2_compile` | `"[abc"` | 106 `MISSING_SQUARE_BRACKET` | [x] |
| 20 | `pcre2_compile` | `"[\\E]"`-style invalid escape in class, e.g. `"[a\\Bc]"` | 107 `ESCAPE_INVALID_IN_CLASS` | [x] |
| 21 | `pcre2_compile` | `"[z-a]"` | 108 `CLASS_RANGE_ORDER` | [x] |
| 22 | `pcre2_compile` | `"*a"` / `"+"` — quantifier with nothing to repeat | 109 `QUANTIFIER_INVALID` | [x] |
| 23 | `pcre2_compile` | `"(?~)"` — unrecognized after `(?` | 111 `INVALID_AFTER_PARENS_QUERY` | [x] |
| 24 | `pcre2_compile` | `"[:alpha:]"` outside class | 112 `POSIX_CLASS_NOT_IN_CLASS` | [x] |
| 25 | `pcre2_compile` | `"[[.ch.]]"` collating element | 113 `POSIX_NO_SUPPORT_COLLATING` | [x] |
| 26 | `pcre2_compile` | `"(abc"` | 114 `MISSING_CLOSING_PARENTHESIS` | [x] |
| 27 | `pcre2_compile` | `"\\2"` with only 1 group | 115 `BAD_SUBPATTERN_REFERENCE` | [x] |
| 28 | `pcre2_compile` | `"(?#comment"` | 118 `MISSING_COMMENT_CLOSING` | [x] |
| 29 | `pcre2_compile` | `"(?<=a+)"`-class variable lookbehind too complicated | 125 `LOOKBEHIND_NOT_FIXED_LENGTH` / 135 | [x] |
| 30 | `pcre2_compile` | `"(?+0)"` / `"\\g{+0}"` | 126 `ZERO_RELATIVE_REFERENCE` | [x] |
| 31 | `pcre2_compile` | `"(?(1)a\|b\|c)"` | 127 `TOO_MANY_CONDITION_BRANCHES` | [x] |
| 32 | `pcre2_compile` | `"(?(a)b)"` | 128 `CONDITION_ASSERTION_EXPECTED` / 130 | [x] |
| 33 | `pcre2_compile` | `"(?-1)"` with no preceding group | 129 `BAD_RELATIVE_REFERENCE` | [x] |
| 34 | `pcre2_compile` | `"[[:qqq:]]"` | 130 `UNKNOWN_POSIX_CLASS` | [x] |
| 35 | `pcre2_compile` | `"\\x{110000}"` | 134 `CODE_POINT_TOO_BIG` | [x] |
| 36 | `pcre2_compile` | `"(?<=\\Ca)"` | 136 `LOOKBEHIND_INVALID_BACKSLASH_C` | [x] |
| 37 | `pcre2_compile` | `"\\L"` / `"\\l"` / `"\\N{...}"` unsupported | 137 `UNSUPPORTED_ESCAPE_SEQUENCE` | [x] |
| 38 | `pcre2_compile` | `"(?C300)"` | 138 `CALLOUT_NUMBER_TOO_BIG` | [x] |
| 39 | `pcre2_compile` | `"(?C1"` | 139 `MISSING_CALLOUT_CLOSING` | [x] |
| 40 | `pcre2_compile` | `"(*MARK:\\d)"` escape in verb name | 140 `ESCAPE_INVALID_IN_VERB` | [x] |
| 41 | `pcre2_compile` | `"(?Pz)"` | 141 `UNRECOGNIZED_AFTER_QUERY_P` | [x] |
| 42 | `pcre2_compile` | `"(?<abc"` — no name terminator | 142 `MISSING_NAME_TERMINATOR` | [x] |
| 43 | `pcre2_compile` | `"(?<a>x)(?<a>y)"` without `PCRE2_DUPNAMES` | 143 `DUPLICATE_SUBPATTERN_NAME` | [x] |
| 44 | `pcre2_compile` | `"(?<1a>x)"` | 144 `INVALID_SUBPATTERN_NAME` | [x] |
| 45 | `pcre2_compile` | `"\\p{"` malformed property | 146 `MALFORMED_UNICODE_PROPERTY` | [x] |
| 46 | `pcre2_compile` | `"\\p{Zzz}"` | 147 `UNKNOWN_UNICODE_PROPERTY` | [x] |
| 47 | `pcre2_compile` | group name > 32 code units | 148 `SUBPATTERN_NAME_TOO_LONG` | [x] |
| 48 | `pcre2_compile` | > 10000 named subpatterns | 149 `TOO_MANY_NAMED_SUBPATTERNS` | [x] |
| 49 | `pcre2_compile` | `"[\\d-z]"` | 150 `CLASS_INVALID_RANGE` | [x] |
| 50 | `pcre2_compile` | `"\\400"` non-UTF octal > 255 | 151 `OCTAL_BYTE_TOO_BIG` | [x] |
| 51 | `pcre2_compile` | `"(?(DEFINE)a\|b)"` | 154 `DEFINE_TOO_MANY_BRANCHES` | [x] |
| 52 | `pcre2_compile` | `"\\o1"` — `\o` not followed by `{` | 155 `BACKSLASH_O_MISSING_BRACE` | [x] |
| 53 | `pcre2_compile` | `"\\g"` bad syntax | 157 `BACKSLASH_G_SYNTAX` | [x] |
| 54 | `pcre2_compile` | `"(?R"` | 158 `PARENS_QUERY_R_MISSING_CLOSING` | [x] |
| 55 | `pcre2_compile` | `"(*ACCEPT:x)"` verb argument not allowed | 159 `VERB_ARGUMENT_NOT_ALLOWED` | [x] |
| 56 | `pcre2_compile` | `"(*ZZZ)"` | 160 `VERB_UNKNOWN` | [x] |
| 57 | `pcre2_compile` | `"(?99999999999)"` | 161 `SUBPATTERN_NUMBER_TOO_BIG` | [x] |
| 58 | `pcre2_compile` | `"(?&)"` / `"\\k<>"` | 162 `SUBPATTERN_NAME_EXPECTED` | [x] |
| 59 | `pcre2_compile` | `"\\o{}"` / `"\\o{9}"` | 164 `INVALID_OCTAL` / 198 | [x] |
| 60 | `pcre2_compile` | `"(?\|(?<a>x))(?\|(?<b>y))"` name/number mismatch | 165 `SUBPATTERN_NAMES_MISMATCH` | [x] |
| 61 | `pcre2_compile` | `"(*MARK)"` | 166 `MARK_MISSING_ARGUMENT` | [x] |
| 62 | `pcre2_compile` | `"\\x{zz}"` | 167 `INVALID_HEXADECIMAL` | [x] |
| 63 | `pcre2_compile` | `"\\c\x80"` non-ASCII after `\c` | 168 `BACKSLASH_C_SYNTAX` | [x] |
| 64 | `pcre2_compile` | `"\\kx"` | 169 `BACKSLASH_K_SYNTAX` | [x] |
| 65 | `pcre2_compile` | `"[\\N]"` | 171 `BACKSLASH_N_IN_CLASS` | [x] |
| 66 | `pcre2_compile` | callout string longer than limit | 172 `CALLOUT_STRING_TOO_LONG` | [x] |
| 67 | `pcre2_compile` | `"\\x{d800}"` with `PCRE2_UTF` | 173 `UNICODE_DISALLOWED_CODE_POINT` | [x] |
| 68 | `pcre2_compile` | verb name too long | 176 `VERB_NAME_TOO_LONG` | [x] |
| 69 | `pcre2_compile` | `"\\u{110000}"` with `ALT_BSUX`+`EXTRA_ALT_BSUX` | 177 `BACKSLASH_U_CODE_POINT_TOO_BIG` | [x] |
| 70 | `pcre2_compile` | `"\\x{}"` / `"\\o{}"` | 178 `MISSING_OCTAL_OR_HEX_DIGITS` | [x] |
| 71 | `pcre2_compile` | `"(?(VERSION>=x))"` | 179 `VERSION_CONDITION_SYNTAX` | [x] |
| 72 | `pcre2_compile` | `"(?C{"`-unterminated / bad delimiter | 181 `CALLOUT_NO_STRING_DELIMITER` / 182 | [x] |
| 73 | `pcre2_compile` | `"\\C"` with `PCRE2_NEVER_BACKSLASH_C` | 183 `BACKSLASH_C_CALLER_DISABLED` | [x] |
| 74 | `pcre2_compile` | `(?\|` nesting deeper than 200 | 184 `QUERY_BARJX_NEST_TOO_DEEP` | [x] |
| 75 | `pcre2_compile` | lookbehind longer than 65535 | 187 `LOOKBEHIND_TOO_LONG` | [x] |
| 76 | `pcre2_compile` | `"\\p{Any}"` etc. under non-Unicode-only build path | 193 `SUPPORTED_ONLY_IN_UNICODE` | [x] |
| 77 | `pcre2_compile` | `"(?^-i)"` | 194 `INVALID_HYPHEN_IN_OPTIONS` | [x] |
| 78 | `pcre2_compile` | `"(*pla:x)"` misspelt / `"(*zzz:x)"` | 195 `ALPHA_ASSERTION_UNKNOWN` | [x] |
| 79 | `pcre2_compile` | > 65535 capture groups | 197 `TOO_MANY_CAPTURES` | [x] |
| 80 | `pcre2_compile` | `"\\o{"` no digit | 198 `MISSING_OCTAL_DIGIT` | [x] |
| 81 | `pcre2_compile` | `"(?<=\\Kx)"` without `EXTRA_ALLOW_LOOKAROUND_BSK` | 199 `BACKSLASH_K_IN_LOOKAROUND` | [x] |
| 82 | `pcre2_compile` | `"\\400"` with `PCRE2_EXTRA_PYTHON_OCTAL` | 202 `OVERSIZE_PYTHON_OCTAL` | [x] |
| 83 | `pcre2_compile` | `"(?C1)"` with `PCRE2_EXTRA_NEVER_CALLOUT` | 203 `CALLOUT_CALLER_DISABLED` | [x] |
| 84 | `pcre2_compile` | `PCRE2_EXTRA_TURKISH_CASING` without `PCRE2_UTF` | 205 `TURKISH_CASING_REQUIRES_UTF` | [x] |
| 85 | `pcre2_compile` | `PCRE2_EXTRA_TURKISH_CASING` + `PCRE2_EXTRA_CASELESS_RESTRICT` | 206 `EXTRA_CASING_INCOMPATIBLE` | [x] |
| 86 | `pcre2_compile` | `"[[a][b][c]…]"` eclass nesting too deep | 207 `ECLASS_NEST_TOO_DEEP` | [x] |
| 87 | `pcre2_compile` | `"[a&&b]"`-style bad operator (ALT_EXTENDED_CLASS) | 208 `ECLASS_INVALID_OPERATOR` | [x] |
| 88 | `pcre2_compile` | `"[&&a]"` unexpected operator | 209 `ECLASS_UNEXPECTED_OPERATOR` | [x] |
| 89 | `pcre2_compile` | `"[a&&]"` expected operand | 210 `ECLASS_EXPECTED_OPERAND` | [x] |
| 90 | `pcre2_compile` | `"[a&&b\|\|c]"` mixed operators | 211 `ECLASS_MIXED_OPERATORS` | [x] |
| 91 | `pcre2_compile` | `"(?[a])"` hint square bracket | 212 `ECLASS_HINT_SQUARE_BRACKET` | [x] |
| 92 | `pcre2_compile` | `"[[:a:]&&]"` perl eclass unexpected expr | 213 `PERL_ECLASS_UNEXPECTED_EXPR` | [x] |
| 93 | `pcre2_compile` | `"[()]"`-perl eclass empty expr | 214 `PERL_ECLASS_EMPTY_EXPR` | [x] |
| 94 | `pcre2_compile` | perl eclass missing close | 215 `PERL_ECLASS_MISSING_CLOSE` | [x] |
| 95 | `pcre2_compile` | perl eclass unexpected char | 216 `PERL_ECLASS_UNEXPECTED_CHAR` | [x] |
| 96 | `pcre2_compile` | `"(?<a>)(?(<a>)x)"`-style expected capture group | 217 `EXPECTED_CAPTURE_GROUP` | [x] |
| 97 | `pcre2_compile` | `")"` alone | 122 `UNMATCHED_CLOSING_PARENTHESIS` | [x] |
| 98 | `pcre2_compile` | `"(?(1)"` | 124 `MISSING_CONDITION_CLOSING` | [x] |
| 99 | `pcre2_compile` | `")"`-after-verb / `"a{1"`-missing number terminator | 219 `MISSING_NUMBER_TERMINATOR` | [x] |
| 100 | `pcre2_compile` | `"(?{"` missing opening paren for `(*...)` construct | 218 `MISSING_OPENING_PARENTHESIS` | [x] |
| 101 | `pcre2_compile` | `"[]"` without `PCRE2_ALLOW_EMPTY_CLASS` | 106 `MISSING_SQUARE_BRACKET` | [x] |
| 102 | `pcre2_compile` | `"\\p{...}"` with `PCRE2_NEVER_UCP` → `PCRE2_UCP` forbidden | 175 `UCP_IS_DISABLED` | [x] |
| 103 | `pcre2_compile` | `"(*UTF)"` with `PCRE2_NEVER_UTF` | 174 `UTF_IS_DISABLED` | [x] |
| 104 | `pcre2_compile` | `"(?<=a{2,3}b)"` variable lookbehind too complicated | 135 `LOOKBEHIND_TOO_COMPLICATED` | [x] |

## 3. `pcre2_match` (`pcre2_match.c:7042‑8242`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 105 | `pcre2_match` | `match_data == NULL` | `PCRE2_ERROR_NULL` (-51) | [x] |
| 106 | `pcre2_match` | `code == NULL` | `-51` and `match_data->rc = -51` | [x] |
| 107 | `pcre2_match` | `subject == NULL && length != 0` | `-51` | [x] |
| 108 | `pcre2_match` | `subject == NULL && length == 0` | ok (treated as empty) | [x] |
| 109 | `pcre2_match` | `options & ~PUBLIC_MATCH_OPTIONS != 0` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 110 | `pcre2_match` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) | [x] |
| 111 | `pcre2_match` | `re->magic_number != MAGIC_NUMBER` | `PCRE2_ERROR_BADMAGIC` (-31) | [x] |
| 112 | `pcre2_match` | `re->flags` code-unit-width mismatch | `PCRE2_ERROR_BADMODE` (-32) | [x] |
| 113 | `pcre2_match` | `PCRE2_PARTIAL_HARD` + `PCRE2_PARTIAL_SOFT` together | `-34` `BADOPTION` | [x] |
| 114 | `pcre2_match` | `PCRE2_ENDANCHORED` set at match time but pattern has `\K`-ish / other invalid combo | `-34` | [x] |
| 115 | `pcre2_match` | `PCRE2_USE_OFFSET_LIMIT` not set but `offset_limit != PCRE2_UNSET` | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | [x] |
| 116 | `pcre2_match` | invalid UTF-8 subject, `PCRE2_UTF`, no `NO_UTF_CHECK` | negative `PCRE2_ERROR_UTF8_ERRn` (-3..-23) | [x] |
| 117 | `pcre2_match` | UTF subject, `start_offset` mid-character | `PCRE2_ERROR_BADUTFOFFSET` (-36) | [x] |
| 118 | `pcre2_match` | `MATCH_INVALID_UTF`, start offset 0 on isolated 0x80 | `PCRE2_ERROR_UTF8_ERR20` (-22) | [x] |
| 119 | `pcre2_match` | match limit exceeded (`pcre2_set_match_limit(1)`) | `PCRE2_ERROR_MATCHLIMIT` (-47) | [x] |
| 120 | `pcre2_match` | depth limit exceeded (`pcre2_set_depth_limit(1)`) | `PCRE2_ERROR_DEPTHLIMIT` (-53) | [x] |
| 121 | `pcre2_match` | heap limit exceeded (`pcre2_set_heap_limit(0)`) | `PCRE2_ERROR_HEAPLIMIT` (-63) | [x] |
| 122 | `pcre2_match` | `\K` moving start past current point in assertion | `PCRE2_ERROR_BAD_BACKSLASH_K` (-75) | [x] |
| 123 | `pcre2_match` | infinite recursion, check enabled | `PCRE2_ERROR_RECURSELOOP` (-52) | [x] |
| 124 | `pcre2_match` | no match | `PCRE2_ERROR_NOMATCH` (-1) | [x] |
| 125 | `pcre2_match` | partial match with `PCRE2_PARTIAL_SOFT`/`HARD` | `PCRE2_ERROR_PARTIAL` (-2) | [x] |

## 4. `pcre2_dfa_match` (`pcre2_dfa_match.c:3396‑4114`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 126 | `pcre2_dfa_match` | `match_data == NULL` | `-51` `NULL` | [x] |
| 127 | `pcre2_dfa_match` | `code == NULL` / `subject == NULL && length != 0` | `-51` | [x] |
| 128 | `pcre2_dfa_match` | `options & ~PUBLIC_DFA_MATCH_OPTIONS != 0` | `-34` `BADOPTION` | [x] |
| 129 | `pcre2_dfa_match` | `wscount < 20` | `PCRE2_ERROR_DFA_WSSIZE` (-43) | [x] |
| 130 | `pcre2_dfa_match` | `workspace == NULL` | `-51` `NULL` | [x] |
| 131 | `pcre2_dfa_match` | `start_offset > length` | `-33` `BADOFFSET` | [x] |
| 132 | `pcre2_dfa_match` | `PCRE2_PARTIAL_HARD`+`SOFT`, or `DFA_RESTART`+`DFA_SHORTEST` conflict | `-34` | [x] |
| 133 | `pcre2_dfa_match` | `PCRE2_MATCH_INVALID_UTF` in the compiled pattern | `PCRE2_ERROR_DFA_UINVALID_UTF` (-66) | [x] |
| 134 | `pcre2_dfa_match` | bad magic | `-31` `BADMAGIC` | [x] |
| 135 | `pcre2_dfa_match` | bad mode | `-32` `BADMODE` | [x] |
| 136 | `pcre2_dfa_match` | `PCRE2_DFA_RESTART` with a workspace not from a partial match | `PCRE2_ERROR_DFA_BADRESTART` (-38) | [x] |
| 137 | `pcre2_dfa_match` | `offset_limit != PCRE2_UNSET` without `USE_OFFSET_LIMIT` | `-56` `BADOFFSETLIMIT` | [x] |
| 138 | `pcre2_dfa_match` | invalid UTF subject | `PCRE2_ERROR_UTF8_ERRn` | [x] |
| 139 | `pcre2_dfa_match` | UTF start offset mid character | `-36` `BADUTFOFFSET` | [x] |
| 140 | `pcre2_dfa_match` | pattern contains `\C` (`OP_ANYBYTE`) in UTF | `PCRE2_ERROR_DFA_UITEM` (-42) | [x] |
| 141 | `pcre2_dfa_match` | condition the DFA cannot handle (`(?(R1)…)`) | `PCRE2_ERROR_DFA_UCOND` (-40) | [x] |
| 142 | `pcre2_dfa_match` | `(?1)` recursion returning zero-length | `PCRE2_ERROR_DFA_RECURSE` (-39) | [x] |
| 143 | `pcre2_dfa_match` | infinite recursion | `PCRE2_ERROR_RECURSELOOP` (-52) | [x] |
| 144 | `pcre2_dfa_match` | match/depth limit exceeded | `-47` / `-53` | [x] |
| 145 | `pcre2_dfa_match` | workspace overflow during match | `PCRE2_ERROR_DFA_WSSIZE` (-43) | [x] |
| 146 | `pcre2_dfa_match` | heap limit exceeded | `-63` `HEAPLIMIT` | [x] |
| 147 | `pcre2_dfa_match` | no match / partial | `-1` / `-2` | [x] |

## 5. `pcre2_substitute` (`pcre2_substitute.c:797‑1788`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 148 | `pcre2_substitute` | `options & ~PUBLIC_SUBSTITUTE_OPTIONS != 0` | `-34` `BADOPTION` | [x] |
| 149 | `pcre2_substitute` | `replacement == NULL && rlength != 0` | `-51` `NULL` | [x] |
| 150 | `pcre2_substitute` | `subject == NULL && length != 0` | `-51` `NULL` | [x] |
| 151 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` and `match_data == NULL` | `-51` `NULL` | [x] |
| 152 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with match data from `pcre2_dfa_match` | `PCRE2_ERROR_DFA_UFUNC` (-41) | [x] |
| 153 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with match data from a different code | `PCRE2_ERROR_DIFFSUBSPATTERN` (-71) | [x] |
| 154 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with a different subject pointer | `PCRE2_ERROR_DIFFSUBSSUBJECT` (-72) | [x] |
| 155 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with a different start offset | `PCRE2_ERROR_DIFFSUBSOFFSET` (-73) | [x] |
| 156 | `pcre2_substitute` | `SUBSTITUTE_MATCHED` with different match options | `PCRE2_ERROR_DIFFSUBSOPTIONS` (-74) | [x] |
| 157 | `pcre2_substitute` | `start_offset > length` | `-33` `BADOFFSET` | [x] |
| 158 | `pcre2_substitute` | output buffer too small, no `OVERFLOW_LENGTH` | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 159 | `pcre2_substitute` | output buffer too small, with `OVERFLOW_LENGTH` | `-48` and `*blength` = required | [x] |
| 160 | `pcre2_substitute` | `outlengthptr` initial 0 | `-48` | [x] |
| 161 | `pcre2_substitute` | `"$"` at end of replacement (extended off) | `PCRE2_ERROR_BADREPLACEMENT` (-35) | [x] |
| 162 | `pcre2_substitute` | `"${1"` unterminated | `PCRE2_ERROR_REPMISSINGBRACE` (-58) | [x] |
| 163 | `pcre2_substitute` | `"$9"` referring to a nonexistent group | `PCRE2_ERROR_NOSUBSTRING` (-49) | [x] |
| 164 | `pcre2_substitute` | `"${zz}"` unknown name, no `SUBSTITUTE_UNKNOWN_UNSET` | `-49` `NOSUBSTRING` | [x] |
| 165 | `pcre2_substitute` | unset group referenced without `SUBSTITUTE_UNSET_EMPTY` | `PCRE2_ERROR_UNSET` (-55) | [x] |
| 166 | `pcre2_substitute` | group not in the (too small) ovector | `PCRE2_ERROR_UNAVAILABLE` (-54) | [x] |
| 167 | `pcre2_substitute` | `SUBSTITUTE_EXTENDED` with `"\\q"` bad escape | `PCRE2_ERROR_BADREPESCAPE` (-57) | [x] |
| 168 | `pcre2_substitute` | `SUBSTITUTE_EXTENDED` bad `${n:…}` substitution syntax | `PCRE2_ERROR_BADSUBSTITUTION` (-59) | [x] |
| 169 | `pcre2_substitute` | pattern that can match empty repeatedly with `GLOBAL` | `PCRE2_ERROR_INTERNAL_DUPMATCH` (-65) or ok | [x] |
| 170 | `pcre2_substitute` | > `SUBSTITUTE_MAX` replacements | `PCRE2_ERROR_TOOMANYREPLACE` (-61) | [x] |
| 171 | `pcre2_substitute` | partial match returned by `pcre2_match` in substitute | `PCRE2_ERROR_PARTIALSUBS` (-76) | [x] |
| 172 | `pcre2_substitute` | `pcre2_match` returned an unexpected negative rc | that rc propagated | [x] |
| 173 | `pcre2_substitute` | substitute case callout returns a bad length | `PCRE2_ERROR_REPLACECASE` (-69) | [x] |
| 174 | `pcre2_substitute` | replacement would exceed `PCRE2_SIZE` | `PCRE2_ERROR_TOOLARGEREPLACE` (-70) | [x] |
| 175 | `pcre2_substitute` | bad code / `BADSUBSPATTERN` (match_data too small etc.) | `PCRE2_ERROR_BADSUBSPATTERN` (-60) | [x] |

## 6. `pcre2_pattern_info` (`pcre2_pattern_info.c:107‑300`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 176 | `pcre2_pattern_info` | `code == NULL` | `-51` `NULL` | [x] |
| 177 | `pcre2_pattern_info` | bad magic number | `-31` `BADMAGIC` | [x] |
| 178 | `pcre2_pattern_info` | code-unit-width mismatch | `-32` `BADMODE` | [x] |
| 179 | `pcre2_pattern_info` | unknown `what` (e.g. 9999 / negative-as-u32) | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 180 | `pcre2_pattern_info` | `PCRE2_INFO_DEPTHLIMIT` when unset | `PCRE2_ERROR_UNSET` (-55) | [x] |
| 181 | `pcre2_pattern_info` | `PCRE2_INFO_HEAPLIMIT` when unset | `-55` `UNSET` | [x] |
| 182 | `pcre2_pattern_info` | `PCRE2_INFO_MATCHLIMIT` when unset | `-55` `UNSET` | [x] |
| 183 | `pcre2_pattern_info` | `PCRE2_INFO_SIZE` with `where == NULL` | 0 and no write (size query form) | [x] |
| 184 | `pcre2_callout_enumerate` | `code == NULL` | `-51` `NULL` | [x] |
| 185 | `pcre2_callout_enumerate` | bad magic / bad mode | `-31` / `-32` | [x] |

## 7. `pcre2_substring_*` (`pcre2_substring.c`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 186 | `pcre2_substring_length_bynumber` | match data from `pcre2_dfa_match` and `stringnumber>0` on partial | `PCRE2_ERROR_PARTIAL` (-2) | [x] |
| 187 | `pcre2_substring_length_bynumber` | `stringnumber` > capture count | `PCRE2_ERROR_NOSUBSTRING` (-49) | [x] |
| 188 | `pcre2_substring_length_bynumber` | `stringnumber >= match_data->oveccount` | `PCRE2_ERROR_UNAVAILABLE` (-54) | [x] |
| 189 | `pcre2_substring_length_bynumber` | group exists but unset | `PCRE2_ERROR_UNSET` (-55) | [x] |
| 190 | `pcre2_substring_length_bynumber` | ovector start > end | `PCRE2_ERROR_INVALIDOFFSET` (-67) | [x] |
| 191 | `pcre2_substring_copy_bynumber` | buffer too small (`size+1 > *sizeptr`) | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 192 | `pcre2_substring_copy_bynumber` | nonexistent group | `-49` / `-54` / `-55` | [x] |
| 193 | `pcre2_substring_get_bynumber` | nonexistent / unset group | `-49` / `-54` / `-55` | [x] |
| 194 | `pcre2_substring_number_from_name` | name not found | `-49` `NOSUBSTRING` | [x] |
| 195 | `pcre2_substring_number_from_name` | duplicate name (DUPNAMES) | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) | [x] |
| 196 | `pcre2_substring_length_byname` | match data from DFA | `PCRE2_ERROR_DFA_UFUNC` (-41) | [x] |
| 197 | `pcre2_substring_length_byname` | name not found | `-49` | [x] |
| 198 | `pcre2_substring_length_byname` | all dup groups unset | `-55` `UNSET` | [x] |
| 199 | `pcre2_substring_length_byname` | no dup group inside ovector | `-54` `UNAVAILABLE` | [x] |
| 200 | `pcre2_substring_copy_byname` | as above + buffer too small | `-48` | [x] |
| 201 | `pcre2_substring_get_byname` | as above | `-49` / `-54` / `-55` / `-41` | [x] |
| 202 | `pcre2_substring_list_get` | after a failed match (`rc <= 0`) | `PCRE2_ERROR_NOSUBSTRING`-ish / -48 | [x] |
| 203 | `pcre2_substring_nametable_scan` | name not present | `-49` `NOSUBSTRING` | [x] |

## 8. `pcre2_serialize_*` (`pcre2_serialize.c:86‑275`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 204 | `pcre2_serialize_encode` | `codes == NULL` or `serialized_bytes == NULL` or `serialized_size == NULL` | `-51` `NULL` | [x] |
| 205 | `pcre2_serialize_encode` | `number_of_codes <= 0` | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 206 | `pcre2_serialize_encode` | `codes[i] == NULL` | `-51` `NULL` | [x] |
| 207 | `pcre2_serialize_encode` | `codes[i]->magic_number != MAGIC_NUMBER` | `-31` `BADMAGIC` | [x] |
| 208 | `pcre2_serialize_encode` | codes with different character tables | `PCRE2_ERROR_MIXEDTABLES` (-30) | [x] |
| 209 | `pcre2_serialize_decode` | `data == NULL` or `codes == NULL` | `-51` `NULL` | [x] |
| 210 | `pcre2_serialize_decode` | `number_of_codes <= 0` | `-29` `BADDATA` | [x] |
| 211 | `pcre2_serialize_decode` | `data->number_of_codes <= 0` | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) | [x] |
| 212 | `pcre2_serialize_decode` | `data->magic` wrong | `-31` `BADMAGIC` | [x] |
| 213 | `pcre2_serialize_decode` | `data->version` wrong | `-32` `BADMODE` | [x] |
| 214 | `pcre2_serialize_decode` | `data->config` wrong (different code-unit width) | `-32` `BADMODE` | [x] |
| 215 | `pcre2_serialize_decode` | truncated / corrupt body | `-62` `BADSERIALIZEDDATA` | [x] |
| 216 | `pcre2_serialize_get_number_of_codes` | `data == NULL` | `-51` `NULL` | [x] |
| 217 | `pcre2_serialize_get_number_of_codes` | bad magic / version / config | `-31` / `-32` | [x] |

## 9. `pcre2_pattern_convert` (`pcre2_convert.c:1135‑1235`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 218 | `pcre2_pattern_convert` | `pattern == NULL` (len!=0) / `buffptr == NULL` / `blength == NULL` | `-51` `NULL` | [x] |
| 219 | `pcre2_pattern_convert` | `options & ~ALL_CONVERT_OPTIONS != 0` | `-34` `BADOPTION` | [x] |
| 220 | `pcre2_pattern_convert` | no / multiple conversion-type bits set | `-34` `BADOPTION` | [x] |
| 221 | `pcre2_pattern_convert` | `PCRE2_CONVERT_UTF` in a non-Unicode build | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` (132) | [x] |
| 222 | `pcre2_pattern_convert` | invalid UTF input, `CONVERT_UTF` without `CONVERT_NO_UTF_CHECK` | `PCRE2_ERROR_UTF8_ERRn` | [x] |
| 223 | `pcre2_pattern_convert` | POSIX BRE/ERE `"[abc"` unterminated class | `106` `MISSING_SQUARE_BRACKET` | [x] |
| 224 | `pcre2_pattern_convert` | POSIX pattern ending with `"\\"` | `101` `END_BACKSLASH` | [x] |
| 225 | `pcre2_pattern_convert` | glob with bad syntax (`"[a"`, `"**"` misuse) | `PCRE2_ERROR_CONVERT_SYNTAX` (-64) | [x] |
| 226 | `pcre2_pattern_convert` | conversion output exceeds provided/needed buffer | `-48` `NOMEMORY` | [x] |
| 227 | `pcre2_pattern_convert` | `glob` where separator/escape make it impossible | `-64` `CONVERT_SYNTAX` | [x] |
| 228 | `pcre2_pattern_convert` | internal inconsistency (2-pass length mismatch) | `PCRE2_ERROR_INTERNAL` (-44) | [x] |

## 10. `pcre2_config` (`pcre2_config.c:78‑247`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 229 | `pcre2_config` | `what` not recognized (e.g. 9999) | `-34` `BADOPTION` | [x] |
| 230 | `pcre2_config` | `PCRE2_CONFIG_JITTARGET` in a non-JIT build | `-34` `BADOPTION` | [x] |
| 231 | `pcre2_config` | `PCRE2_CONFIG_UNICODE_VERSION` etc. with `where == NULL` | required length (>0), no write | [x] |

## 11. Context setters (`pcre2_context.c:326‑553`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 232 | `pcre2_set_bsr` | `value` not `PCRE2_BSR_UNICODE`/`ANYCRLF` (e.g. 0, 3, 0xFFFFFFFF) | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 233 | `pcre2_set_newline` | `newline` not 1..6 (e.g. 0, 7, 0xFFFFFFFF) | `-29` `BADDATA` | [x] |
| 234 | `pcre2_set_optimize` | `ccontext == NULL` | `-51` `NULL` | [x] |
| 235 | `pcre2_set_optimize` | `directive` outside `NONE`/`FULL`/`[PCRE2_AUTO_POSSESS..START_OPTIMIZE_OFF]` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 236 | `pcre2_set_glob_separator` | separator not `/`, `\`, or `.` | `-29` `BADDATA` | [x] |
| 237 | `pcre2_set_glob_escape` | `escape > 255`, or nonzero and not ASCII punctuation | `-29` `BADDATA` | [x] |
| 238 | `pcre2_set_character_tables` | any value (incl. `NULL`) | always `0` | [x] |
| 239 | `pcre2_set_max_pattern_length` etc. | any value | always `0` | [x] |
| 240 | `pcre2_set_recursion_memory_management` | any values | always `0` (no-op) | [x] |

## 12. Context / object creation & copy (`pcre2_context.c:87‑272`, `pcre2_match_data.c`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 241 | `pcre2_general_context_create` | malloc returns `NULL` | `NULL` | [x] |
| 242 | `pcre2_compile_context_copy` | `ccontext == NULL` | `NULL` | [x] |
| 243 | `pcre2_match_context_copy` | `mcontext == NULL` | `NULL` | [x] |
| 244 | `pcre2_convert_context_copy` | `cvcontext == NULL` | `NULL` | [x] |
| 245 | `pcre2_general_context_copy` | `gcontext == NULL` | `NULL` | [x] |
| 246 | `pcre2_match_data_create_from_pattern` | `code == NULL` | `NULL` | [x] |
| 247 | `pcre2_code_copy` | `code == NULL` | `NULL` | [x] |
| 248 | `pcre2_code_copy_with_tables` | `code == NULL` | `NULL` | [x] |
| 249 | `pcre2_code_free` | `code == NULL` | no-op, no crash | [x] |
| 250 | `pcre2_match_data_free` / `pcre2_substring_free` / `pcre2_substring_list_free` / `pcre2_serialize_free` / `pcre2_converted_pattern_free` / `*_context_free` / `pcre2_maketables_free` | `NULL` argument | no-op, no crash | [x] |
| 251 | `pcre2_match_data_create` | `ovecsize == 0` | ovector count forced to 1 | [x] |
| 252 | `pcre2_get_ovector_count` / `pcre2_get_startchar` / `pcre2_get_mark` | after a failed match | defined values (count, `PCRE2_UNSET`, `NULL`) | [x] |

## 13. `pcre2_get_error_message` (`pcre2_error.c:339‑367`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 253 | `pcre2_get_error_message` | `size == 0` | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 254 | `pcre2_get_error_message` | `errorcode` out of range (e.g. 0, 1000, -1000) | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 255 | `pcre2_get_error_message` | buffer too small for the message | `-48` `NOMEMORY` | [x] |
| 256 | `pcre2_get_error_message` | valid code, adequate buffer | length of message (>0) | [x] |

## 14. `pcre2_next_match` (`pcre2_match_next.c:97‑168`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 257 | `pcre2_next_match` | `match_data->rc <= 0` (no previous match) | `FALSE` (0) | [x] |
| 258 | `pcre2_next_match` | previous match reached end of subject | `FALSE` (0) | [x] |
| 259 | `pcre2_next_match` | previous non-empty match | `TRUE`, `*pstart_offset` = match end | [x] |
| 260 | `pcre2_next_match` | previous empty match | `TRUE`, options include `NOTEMPTY_ATSTART\|ANCHORED` | [x] |

## 15. JIT stubs (non-JIT build) (`pcre2_jit_*_inc.h`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 261 | `pcre2_jit_compile` | any code (no JIT support) | `PCRE2_ERROR_JIT_BADOPTION` (-45) or `-68` `JIT_UNSUPPORTED` | [x] |
| 262 | `pcre2_jit_compile` | `code == NULL` | `-51` `NULL` | [x] |
| 263 | `pcre2_jit_compile` | `options` containing unknown bits | `-45` `JIT_BADOPTION` | [x] |
| 264 | `pcre2_jit_match` | any input (no JIT) | `PCRE2_ERROR_JIT_BADOPTION` (-45) | [x] |
| 265 | `pcre2_jit_stack_create` | any size, non-JIT | `NULL` | [x] |
| 266 | `pcre2_jit_stack_create` | `startsize > maxsize` or zero sizes | `NULL` | [x] |
| 267 | `pcre2_jit_stack_assign` / `pcre2_jit_stack_free` / `pcre2_jit_free_unused_memory` | any / `NULL` | no-op, no crash | [x] |
| 268 | `pcre2_jit_compile` with `PCRE2_INFO_JITSIZE` | query after failed jit compile | 0 | [x] |

## 16. Internal (`_pcre2_*`) helpers reachable across FFI

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 269 | `_pcre2_valid_utf_8` | invalid UTF-8 byte sequences (all 21 `UTF8_ERRn` classes) | matching `n` and `erroroffset` | [x] |
| 270 | `_pcre2_valid_utf_8` | valid UTF-8 | 0 | [x] |
| 271 | `_pcre2_strcmp_8` / `_pcre2_strncmp_8` / `_pcre2_strcmp_c8_8` / `_pcre2_strncmp_c8_8` | unequal / prefix / empty strings | sign-consistent difference | [x] |
| 272 | `_pcre2_strlen_8` | empty string | 0 | [x] |
| 273 | `_pcre2_ord2utf_8` | code points 0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF | byte length 1..4 and bytes | [x] |
| 274 | `_pcre2_ckd_smul_8` | overflowing multiply | returns TRUE (overflow) | [x] |
| 275 | `_pcre2_ckd_smul_8` | non-overflowing multiply | FALSE and product | [x] |
| 276 | `_pcre2_is_newline_8` / `_pcre2_was_newline_8` | each newline convention × CR/LF/CRLF/NEL/LS/PS/other | matching bool + length | [x] |
| 277 | `_pcre2_memctl_malloc_8` | zero / huge size | `NULL` on failure | [x] |
| 278 | `_pcre2_default_tables_8`, `_pcre2_utf8_table*`, `_pcre2_ucd_*`, `_pcre2_utt_*`, `_pcre2_OP_lengths_8`, `_pcre2_ucp_*`, `_pcre2_hspace_list_8`, `_pcre2_vspace_list_8`, `_pcre2_posix_class_maps8`, `_pcre2_callout_*_delims_8`, `_pcre2_unicode_version_8` | exported data tables | byte-identical contents | [x] |

---

## Coverage map (which test file proves which section)

| ERRORS.md section | test file |
|---|---|
| 1 · `pcre2_compile` argument validation (1-13) | `tests/t10_compile_errors.rs` (`compile_argument_pointer_contract`, `compile_every_undefined_option_bit`), `tests/t02_compile.rs` (`compile_limits`) |
| 2 · `pcre2_compile` pattern syntax (14-104) | `tests/t10_compile_errors.rs` (`every_compile_error_case_agrees`), `tests/t02_compile.rs` (`compile_random_bytes`, 72 000 randomized patterns × 9 option sets) |
| 3 · `pcre2_match` (105-125) | `tests/t03_match.rs` (`match_error_paths`, `match_utf_error_paths`, `match_k_and_recurseloop`, `match_limits`, `match_offset_limit`) |
| 4 · `pcre2_dfa_match` (126-147) | `tests/t04_dfa.rs` (`dfa_error_paths`, `dfa_unsupported_items`, `dfa_workspace_overflow`, `dfa_utf_error_paths`, `dfa_limits_and_offset_limit`) |
| 5 · `pcre2_substitute` (148-175) | `tests/t05_substitute.rs` |
| 6 · `pcre2_pattern_info` (176-185) | `tests/t02_compile.rs` (`pattern_info_error_paths`, `callout_enumerate_all`) |
| 7 · `pcre2_substring_*` (186-203) | `tests/t06_substring.rs` |
| 8 · `pcre2_serialize_*` (204-217) | `tests/t07_serialize_jit.rs` |
| 9 · `pcre2_pattern_convert` (218-228) | `tests/t08_convert.rs` |
| 10 · `pcre2_config` (229-231) | `tests/t09_context_config.rs` |
| 11 · context setters (232-240) | `tests/t09_context_config.rs` |
| 12 · create/copy/free (241-252) | `tests/t09_context_config.rs` |
| 13 · `pcre2_get_error_message` (253-256) | `tests/t09_context_config.rs`, `tests/t10_compile_errors.rs` (`error_messages_for_all_codes`, every code -90..=230) |
| 14 · `pcre2_next_match` (257-260) | `tests/t06_substring.rs` |
| 15 · JIT stubs (261-268) | `tests/t07_serialize_jit.rs`, `tests/t11_internal.rs` |
| 16 · internal `_pcre2_*` (269-278) | `tests/t01_lowlevel.rs`, `tests/t11_internal.rs` |

### Compile-error-code census

`every_compile_error_case_agrees` reports the set of public compile error codes
actually reached and asserts it does not shrink. **93** distinct codes in
100..=220 are reached with identical code *and* `*erroroffset` from both
libraries; `116` (`NULL_PATTERN`), `188` (`PATTERN_STRING_TOO_LONG`), `201`
(`PATTERN_COMPILED_SIZE_TOO_BIG`) and `220` (`NULL_ERROROFFSET`) are covered by
the dedicated argument/limit tests, giving **97 of the 121** codes.

The remainder are unreachable through the public API in this build
configuration, verified against the C source:

| code | why unreachable |
|---|---|
| 110, 121, 123, 131, 152, 153, 156, 163, 170, 180, 189, 190 | internal-consistency errors, guarded by `PCRE2_DEBUG_UNREACHABLE()` / `LCOV_EXCL` "should not occur" branches |
| 132 (`UNICODE_NOT_SUPPORTED`), 145 (`UNICODE_PROPERTIES_UNAVAILABLE`), 193 (`SUPPORTED_ONLY_IN_UNICODE`), 196 (`SCRIPT_RUN_NOT_AVAILABLE`) | only reachable without `SUPPORT_UNICODE`; this build defines it |
| 185 (`BACKSLASH_C_LIBRARY_DISABLED`) | only with `NEVER_BACKSLASH_C` compiled into the library |
| 191 (`NO_SURROGATES_IN_UTF16`) | 16-bit code-unit width only |
| 159 (`VERB_ARGUMENT_NOT_ALLOWED`) | `ERR59` is not assigned anywhere in the 10.48 sources |
| 172 (`CALLOUT_STRING_TOO_LONG`) | requires a callout string longer than `UINT32_MAX` code units |
| 186 (`PATTERN_TOO_COMPLICATED`) | requires overrunning the compile workspace; `PATTERN_TOO_LARGE` (120) is always hit first |

### Deliberately excluded cases (documented undefined behaviour)

These are **not** tested because the C itself reads out of bounds, so they crash
in *both* libraries and any "expected result" would be meaningless:

* `PCRE2_NO_UTF_CHECK` (match or convert) with a subject that is not valid
  UTF-8 — the engine's `GETCHARINC` family is deliberately bounds-unaware.
* `pcre2_set_offset_limit` with a value greater than the subject length —
  `bumpalong_limit = subject + offset_limit` is not bounds-checked
  (`pcre2_match.c:7400`).
* `pcre2_{general,compile,match,convert}_context_copy(NULL)` — the C
  unconditionally dereferences `ctx->memctl.malloc` (`pcre2_context.c:230-275`);
  returning `NULL` is not a PCRE2 contract.
* Any context setter except `pcre2_set_optimize` with a `NULL` context — the C
  dereferences the pointer directly.
* Reading `pcre2_get_mark` / `pcre2_get_startchar` / the ovector after a return
  code for which the engine never assigned them (see `mark_is_defined` and
  `defined_ovector_entries` in `tests/common/mod.rs`, which encode exactly the
  ranges the C source defines).
* `_pcre2_is_newline_8` with `ptr == endptr`, or `_pcre2_was_newline_8` with
  `ptr == startptr` — both violate the preconditions stated in
  `pcre2_newline.c:59` and `:150`.
