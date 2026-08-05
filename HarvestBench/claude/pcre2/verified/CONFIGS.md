# CONFIGS.md — Configuration-surface table

Mechanically derived from the runtime options / input shapes the C code
branches on. Each row is a meaningful combination that the C treats
differently. Differential tests drive BOTH C and Rust `.so` through their
exports with MANY randomized inputs (fixed seed) per row and assert
byte-identical output (ovector, return code, produced strings).

Entry points cover the full pipeline: low-level `pcre2_compile_8` →
`pcre2_match_8` / `pcre2_dfa_match_8` → ovector / substring / substitute, plus
config and serialization.

| # | entry point(s) | configuration (options + input shape) | [ ] |
|---|----------------|----------------------------------------|-----|
| 1 | compile+match | literal pattern, ASCII subject, single match | [x] |
| 2 | compile+match | pattern with `.` / `*` / `+` quantifiers, many random subjects | [x] |
| 3 | compile+match | anchored `^...$`, matching & non-matching subjects | [x] |
| 4 | compile+match | alternation `a|bb|ccc`, random subjects | [x] |
| 5 | compile+match | capture groups, verify full ovector pairs | [x] |
| 6 | compile+match | named capture groups `(?<name>...)` | [x] |
| 7 | compile+match | character classes `[a-z0-9]`, negated `[^...]` | [x] |
| 8 | compile+match | `PCRE2_CASELESS` option | [x] |
| 9 | compile+match | `PCRE2_MULTILINE` with embedded newlines | [x] |
| 10 | compile+match | `PCRE2_DOTALL` | [x] |
| 11 | compile+match | `PCRE2_EXTENDED` (whitespace/comments in pattern) | [x] |
| 12 | compile+match | `PCRE2_UTF` with multibyte UTF-8 subjects | [x] |
| 13 | compile+match | `PCRE2_UCP` unicode property `\p{L}` etc. | [x] |
| 14 | compile+match | backreferences `(a)\1` | [x] |
| 15 | compile+match | lookahead / lookbehind assertions | [x] |
| 16 | compile+match | bounded quantifiers `a{2,4}` | [x] |
| 17 | compile+match | `start_offset` > 0 (mid-subject start) | [x] |
| 18 | compile+match | `PCRE2_ANCHORED` / `PCRE2_ENDANCHORED` match options | [x] |
| 19 | compile+match | `PCRE2_NOTBOL` / `PCRE2_NOTEOL` / `PCRE2_NOTEMPTY` | [x] |
| 20 | compile+match | empty subject / length 0 | [x] |
| 21 | compile+match | `PCRE2_ZERO_TERMINATED` length sentinel | [x] |
| 22 | compile+dfa_match | same patterns via DFA engine, workspace given | [x] |
| 23 | compile+dfa_match | `PCRE2_DFA_SHORTEST` option | [x] |
| 24 | compile+match | newline conventions CR / LF / CRLF / ANYCRLF / ANY | [x] |
| 25 | compile+match | `\R` / BSR settings (unicode vs anycrlf) | [x] |
| 26 | pattern_info | INFO_SIZE, CAPTURECOUNT, NAMECOUNT, ALLOPTIONS on varied patterns | [x] |
| 27 | substring | copy_bynumber / get_bynumber / length_bynumber after match | [x] |
| 28 | substring | copy_byname / get_byname / number_from_name | [x] |
| 29 | substring | substring_list_get / nametable_scan | [x] |
| 30 | substitute | plain `$1` group refs, single & global | [x] |
| 31 | substitute | `PCRE2_SUBSTITUTE_GLOBAL` multi-replace | [x] |
| 32 | substitute | `PCRE2_SUBSTITUTE_LITERAL` | [x] |
| 33 | substitute | `PCRE2_SUBSTITUTE_EXTENDED` `\U \L \u \l` case forcing | [x] |
| 34 | substitute | `${name}` / `${1:-default}` / `${1:+set:unset}` | [x] |
| 35 | substitute | `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` size computation | [x] |
| 36 | substitute | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` | [x] |
| 37 | serialize | encode then decode a compiled pattern, re-match | [x] |
| 38 | maketables | build tables, compile with custom tables, match | [x] |
| 39 | config | all `PCRE2_CONFIG_*` selectors return same values | [x] |
| 40 | get_error_message | all common negative error codes give same text | [x] |
| 41 | jit_compile | jit_compile returns JIT-unsupported (no JIT build), match falls back | [x] |
</content>
