# ERRORS.md — Error-surface table

Mechanically derived from the C source's top-level rejection paths in each
public entry point. Each row is a distinct invalid-input/condition rejection.
Differential tests construct the exact condition, call BOTH C and Rust via the
`.so`, and assert the SAME error code / sentinel.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `pcre2_config_8` | unknown `what` selector (e.g. 9999) | `PCRE2_ERROR_BADOPTION` (-34) |
| 2 | `pcre2_config_8` | `PCRE2_CONFIG_JIT`/`UNICODE` with too-small `where` semantics — N/A returns value; `PCRE2_CONFIG_JITTARGET` where=NULL returns size | int size / value |
| 3 | `pcre2_compile_8` | NULL `errorcode` pointer | returns NULL (cannot report) |
| 4 | `pcre2_compile_8` | invalid option bits `options & ~PUBLIC_COMPILE_OPTIONS` | NULL, errorcode `PCRE2_ERROR_BADOPTION` |
| 5 | `pcre2_compile_8` | invalid extra option bits | NULL, errorcode set |
| 6 | `pcre2_compile_8` | NULL pattern with non-zero length | NULL, errorcode `ERR16` (NULL arg) |
| 7 | `pcre2_compile_8` | syntactically invalid pattern e.g. `"("` | NULL, errorcode `ERR14` (missing `)`) |
| 8 | `pcre2_compile_8` | unmatched `)` e.g. `")"` | NULL, errorcode `ERR22` |
| 9 | `pcre2_compile_8` | bad quantifier e.g. `"a{2,1}"` | NULL, errorcode `ERR4` |
| 10 | `pcre2_compile_8` | nothing to repeat e.g. `"*a"` | NULL, errorcode `ERR9` |
| 11 | `pcre2_compile_8` | unterminated class e.g. `"[a"` | NULL, errorcode `ERR6` |
| 12 | `pcre2_match_8` | NULL `match_data` | `PCRE2_ERROR_NULL` (-51) |
| 13 | `pcre2_match_8` | NULL `code` or NULL `subject` | `PCRE2_ERROR_NULL` (-51) |
| 14 | `pcre2_match_8` | invalid option bits `options & ~PUBLIC_MATCH_OPTIONS` | `PCRE2_ERROR_BADOPTION` (-34) |
| 15 | `pcre2_match_8` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) |
| 16 | `pcre2_match_8` | non-matching subject | `PCRE2_ERROR_NOMATCH` (-1) |
| 17 | `pcre2_match_8` | invalid UTF subject w/o `NO_UTF_CHECK` | `PCRE2_ERROR_UTF8_*` (<-1) |
| 18 | `pcre2_dfa_match_8` | NULL `match_data` | `PCRE2_ERROR_NULL` (-51) |
| 19 | `pcre2_dfa_match_8` | NULL `code`/`subject`/`workspace` | `PCRE2_ERROR_NULL` (-51) |
| 20 | `pcre2_dfa_match_8` | invalid option bits | `PCRE2_ERROR_BADOPTION` (-34) |
| 21 | `pcre2_dfa_match_8` | `wscount < 20` | `PCRE2_ERROR_DFA_WSSIZE` (-19) |
| 22 | `pcre2_dfa_match_8` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) |
| 23 | `pcre2_dfa_match_8` | no match | `PCRE2_ERROR_NOMATCH` (-1) |
| 24 | `pcre2_pattern_info_8` | NULL `code` | `PCRE2_ERROR_NULL` (-51) |
| 25 | `pcre2_pattern_info_8` | unknown `what` selector | `PCRE2_ERROR_BADOPTION` (-34) |
| 26 | `pcre2_pattern_info_8` | `PCRE2_INFO_*LIMIT` when limit unset | `PCRE2_ERROR_UNSET` (-55) |
| 27 | `pcre2_substring_length_bynumber_8` | stringnumber past top capture | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 28 | `pcre2_substring_length_bynumber_8` | stringnumber >= oveccount | `PCRE2_ERROR_UNAVAILABLE` (-54) |
| 29 | `pcre2_substring_length_bynumber_8` | valid group index that is unset | `PCRE2_ERROR_UNSET` (-55) |
| 30 | `pcre2_substring_copy_bynumber_8` | buffer too small | `PCRE2_ERROR_NOMEMORY` (-48) |
| 31 | `pcre2_substring_number_from_name_8` | name not in pattern | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 32 | `pcre2_substring_nametable_scan_8` | name not in pattern | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 33 | `pcre2_substitute_8` | partial option without `REPLACEMENT_ONLY` | `PCRE2_ERROR_BADOPTION` (-34) |
| 34 | `pcre2_substitute_8` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) |
| 35 | `pcre2_substitute_8` | output buffer too small (no OVERFLOW_LENGTH) | `PCRE2_ERROR_NOMEMORY` (-48) |
| 36 | `pcre2_substitute_8` | bad replacement e.g. lone `"$"` | `PCRE2_ERROR_BADREPLACEMENT` (-35) |
| 37 | `pcre2_substitute_8` | `${` with no closing brace | `PCRE2_ERROR_REPMISSINGBRACE` (-58) |
| 38 | `pcre2_substitute_8` | `$99` unknown group (no UNKNOWN_UNSET) | `PCRE2_ERROR_NOSUBSTRING` (-49) |
| 39 | `pcre2_get_error_message_8` | unknown error number | non-zero / `PCRE2_ERROR_BADDATA` semantics |
| 40 | `pcre2_get_error_message_8` | zero-length buffer | `PCRE2_ERROR_NOMEMORY` (-48) |
| 41 | `pcre2_serialize_decode_8` | NULL / bad serialized data | `PCRE2_ERROR_NULL` / `PCRE2_ERROR_BADMAGIC` |
| 42 | `pcre2_serialize_encode_8` | NULL codes / zero count | `PCRE2_ERROR_BADDATA` (-30) |
| 43 | `pcre2_maketables_8` | (allocation) — returns NULL on OOM | NULL |
| 44 | `pcre2_set_newline_8` | out-of-range newline value | returns 1 (rejected) |
| 45 | `pcre2_set_bsr_8` | out-of-range bsr value | returns 1 (rejected) |
| 46 | `pcre2_pattern_convert_8` | invalid glob options | `PCRE2_ERROR_*` convert error |
</content>
