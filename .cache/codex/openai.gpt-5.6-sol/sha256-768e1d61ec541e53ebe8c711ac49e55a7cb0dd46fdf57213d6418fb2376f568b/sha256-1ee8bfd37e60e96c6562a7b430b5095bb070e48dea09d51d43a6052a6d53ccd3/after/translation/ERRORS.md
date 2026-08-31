# Error surface

Mechanically derived from public-function null/range/option checks and explicit
`PCRE2_ERROR_*` returns in `src/*.c`. Rows are distinct caller-constructible
rejections in this 8-bit, Unicode, no-JIT build. Allocation failures are driven
through a custom general-context allocator.

Private helper assertions and `PCRE2_DEBUG_UNREACHABLE()` sites are not public
input rejections: reaching them requires corrupting an opaque context, compiled
code, or match-data block, which is undefined behavior at the C ABI. They were
still inventoried by grep: 196 `PCRE2_ASSERT`/`PCRE2_DEBUG_UNREACHABLE` sites.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---:|----------|---------------------------------------------|-------------------|:---:|
| 1 | `pcre2_config_8` | Unknown `what`, `where = NULL` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 2 | `pcre2_config_8` | Unknown `what`, non-null `where` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 3 | `pcre2_config_8` | `PCRE2_CONFIG_JITTARGET` in no-JIT build | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 4 | `pcre2_general_context_create_8` | Allocator returns null | null | [x] |
| 5 | `pcre2_*_context_create_8` | General-context allocator returns null | null | [x] |
| 6 | `pcre2_*_context_copy_8` | Source context allocator returns null | null | [x] |
| 7 | `pcre2_set_bsr_8` | Value other than 1 or 2 | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 8 | `pcre2_set_newline_8` | Value outside 1-6, including out-of-range enum values | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 9 | `pcre2_set_optimize_8` | Null context | `PCRE2_ERROR_NULL` (-51) | [x] |
| 10 | `pcre2_set_optimize_8` | Directive other than 0, 1, or 64-69 | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 11 | `pcre2_set_glob_separator_8` | Value other than slash, backslash, or dot | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 12 | `pcre2_set_glob_escape_8` | Value above 255 | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 13 | `pcre2_set_glob_escape_8` | Nonzero ASCII value not in `globpunct` | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 14 | `pcre2_compile_8` | Null `error_code` pointer | null; offset set to 0 when present | [x] |
| 15 | `pcre2_compile_8` | Null `error_offset` pointer | null; error 220 | [x] |
| 16 | `pcre2_compile_8` | Null pattern with nonzero length | null; error 116 at offset 0 | [x] |
| 17 | `pcre2_compile_8` | Any bit outside `PUBLIC_COMPILE_OPTIONS` | null; error 117 | [x] |
| 18 | `pcre2_compile_8` | Compile context has bit outside `PUBLIC_COMPILE_EXTRA_OPTIONS` | null; error 117 | [x] |
| 19 | `pcre2_compile_8` | `LITERAL` combined with a disallowed compile option | null; error 192 | [x] |
| 20 | `pcre2_compile_8` | `LITERAL` combined with a disallowed extra option | null; error 192 | [x] |
| 21 | `pcre2_compile_8` | Pattern length exceeds context maximum by one | null; error 188 | [x] |
| 22 | `pcre2_compile_8` | Compiled size exceeds context maximum by one | null; error 201 | [x] |
| 23 | `pcre2_compile_8` | `UTF` together with `NEVER_UTF` | null; error 174 | [x] |
| 24 | `pcre2_compile_8` | `UCP` together with `NEVER_UCP` | null; error 175 | [x] |
| 25 | `pcre2_compile_8` | Turkish casing without UTF/UCP | null; error 204 | [x] |
| 26 | `pcre2_compile_8` | Turkish casing with UCP but without UTF in 8-bit mode | null; error 205 | [x] |
| 27 | `pcre2_compile_8` | Turkish casing together with caseless-restrict | null; error 206 | [x] |
| 28 | `pcre2_compile_8` | Recursion guard callback rejects nesting | null; error 133 | [x] |
| 29 | `pcre2_compile_8` | Compile allocator fails | null; error 121 | [x] |
| 30 | `pcre2_compile_8` | Trailing backslash | null; error 101 | [x] |
| 31 | `pcre2_compile_8` | Unknown escape such as `\\j` | null; error 103 | [x] |
| 32 | `pcre2_compile_8` | Quantifier lower bound exceeds upper bound | null; error 104 | [x] |
| 33 | `pcre2_compile_8` | Quantifier value above supported maximum | null; error 105 | [x] |
| 34 | `pcre2_compile_8` | Unterminated character class | null; error 106 | [x] |
| 35 | `pcre2_compile_8` | Descending class range | null; error 108 | [x] |
| 36 | `pcre2_compile_8` | Invalid token after `(?` | null; error 111 | [x] |
| 37 | `pcre2_compile_8` | Unterminated group | null; error 114 | [x] |
| 38 | `pcre2_compile_8` | Unmatched closing parenthesis | null; error 122 | [x] |
| 39 | `pcre2_compile_8` | Variable-length lookbehind over configured maximum | null; error 200 | [x] |
| 40 | `pcre2_compile_8` | Duplicate name without `DUPNAMES` | null; error 143 | [x] |
| 41 | `pcre2_compile_8` | Invalid or empty capture name | null; error 144 | [x] |
| 42 | `pcre2_compile_8` | Unknown Unicode property | null; error 147 | [x] |
| 43 | `pcre2_compile_8` | Code point above `0x10ffff` | null; error 134 or 177 | [x] |
| 44 | `pcre2_compile_8` | Invalid UTF-8 pattern with UTF checks enabled | null; exact `PCRE2_ERROR_UTF8_ERR*` | [x] |
| 45 | `pcre2_pattern_info_8` | Null code for a value query | `PCRE2_ERROR_NULL` (-51) | [x] |
| 46 | `pcre2_pattern_info_8` | Unknown selector | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 47 | `pcre2_pattern_info_8` | MATCH/DEPTH/HEAP limit queried when unset | value written; `PCRE2_ERROR_UNSET` (-55) | [x] |
| 48 | `pcre2_callout_enumerate_8` | Null code | `PCRE2_ERROR_NULL` (-51) | [x] |
| 49 | `pcre2_callout_enumerate_8` | Callback returns nonzero | exact callback return | [x] |
| 50 | `pcre2_match_data_create_from_pattern_8` | Null code | null | [x] |
| 51 | `pcre2_match_8` | Null match data | `PCRE2_ERROR_NULL` (-51) | [x] |
| 52 | `pcre2_match_8` | Null code or null nonempty subject | `PCRE2_ERROR_NULL` (-51) | [x] |
| 53 | `pcre2_match_8` | Any bit outside `PUBLIC_MATCH_OPTIONS` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 54 | `pcre2_match_8` | Start offset exceeds subject length by one | `PCRE2_ERROR_BADOFFSET` (-33) | [x] |
| 55 | `pcre2_match_8` | Partial mode combined with ENDANCHORED | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 56 | `pcre2_match_8` | Context offset limit used without compile-time `USE_OFFSET_LIMIT` | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | [x] |
| 57 | `pcre2_match_8` | UTF start offset points into continuation byte | `PCRE2_ERROR_BADUTFOFFSET` (-36) | [x] |
| 58 | `pcre2_match_8` | Invalid UTF-8 subject with checks enabled | exact `PCRE2_ERROR_UTF8_ERR*` | [x] |
| 59 | `pcre2_match_8` | Match-call count reaches context match limit | `PCRE2_ERROR_MATCHLIMIT` (-47) | [x] |
| 60 | `pcre2_match_8` | Frame depth reaches context depth limit | `PCRE2_ERROR_DEPTHLIMIT` (-53) | [x] |
| 61 | `pcre2_match_8` | Heap frame request exceeds context heap limit | `PCRE2_ERROR_HEAPLIMIT` (-63) | [x] |
| 62 | `pcre2_match_8` | Match-data allocator fails when heap/copy storage is needed | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 63 | `pcre2_dfa_match_8` | Null match data | `PCRE2_ERROR_NULL` (-51) | [x] |
| 64 | `pcre2_dfa_match_8` | Null code, nonempty subject, or workspace | `PCRE2_ERROR_NULL` (-51) | [x] |
| 65 | `pcre2_dfa_match_8` | Any bit outside `PUBLIC_DFA_MATCH_OPTIONS` | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 66 | `pcre2_dfa_match_8` | Workspace count below 20 | `PCRE2_ERROR_DFA_WSSIZE` (-43) | [x] |
| 67 | `pcre2_dfa_match_8` | Start offset exceeds subject length by one | `PCRE2_ERROR_BADOFFSET` (-33) | [x] |
| 68 | `pcre2_dfa_match_8` | Partial mode combined with ENDANCHORED | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 69 | `pcre2_dfa_match_8` | `MATCH_INVALID_UTF` compiled pattern | `PCRE2_ERROR_DFA_UINVALID_UTF` (-66) | [x] |
| 70 | `pcre2_dfa_match_8` | RESTART with malformed workspace header/state count | `PCRE2_ERROR_DFA_BADRESTART` (-38) | [x] |
| 71 | `pcre2_dfa_match_8` | Context offset limit used without compile-time flag | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | [x] |
| 72 | `pcre2_dfa_match_8` | DFA-unsupported backreference/condition/item | exact `DFA_UCOND`/`DFA_UITEM` error | [x] |
| 73 | `pcre2_substring_*_byname_8` | Called on DFA match data | `PCRE2_ERROR_DFA_UFUNC` (-41) | [x] |
| 74 | `pcre2_substring_*_bynumber_8` | Capture number exceeds compiled top capture | `PCRE2_ERROR_NOSUBSTRING` (-49) | [x] |
| 75 | `pcre2_substring_*_bynumber_8` | Capture number exceeds ovector capacity | `PCRE2_ERROR_UNAVAILABLE` (-54) | [x] |
| 76 | `pcre2_substring_*_bynumber_8` | Optional capture is unset | `PCRE2_ERROR_UNSET` (-55) | [x] |
| 77 | `pcre2_substring_copy_bynumber_8` | Output capacity is less than capture length plus terminator | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 78 | `pcre2_substring_nametable_scan_8` | Name absent | `PCRE2_ERROR_NOSUBSTRING` (-49) | [x] |
| 79 | `pcre2_substring_nametable_scan_8` | Duplicate name requested as unique number | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) | [x] |
| 80 | `pcre2_serialize_encode_8` | Null codes/output-pointer/size-pointer | `PCRE2_ERROR_NULL` (-51) | [x] |
| 81 | `pcre2_serialize_encode_8` | Code count zero or negative | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 82 | `pcre2_serialize_encode_8` | Null element in codes array | `PCRE2_ERROR_NULL` (-51) | [x] |
| 83 | `pcre2_serialize_encode_8` | Codes use different character-table pointers | `PCRE2_ERROR_MIXEDTABLES` (-30) | [x] |
| 84 | `pcre2_serialize_encode_8` | Allocator fails | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 85 | `pcre2_serialize_decode_8` | Null data or codes output | `PCRE2_ERROR_NULL` (-51) | [x] |
| 86 | `pcre2_serialize_decode_8` | Requested count zero or negative | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 87 | `pcre2_serialize_decode_8` | Serialized count nonpositive | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) | [x] |
| 88 | `pcre2_serialize_decode_8` | Bad serialized magic | `PCRE2_ERROR_BADMAGIC` (-31) | [x] |
| 89 | `pcre2_serialize_decode_8` | Serialized version or ABI config mismatch | `PCRE2_ERROR_BADMODE` (-32) | [x] |
| 90 | `pcre2_serialize_decode_8` | Serialized code block too small or invalid fields | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) | [x] |
| 91 | `pcre2_serialize_get_number_of_codes_8` | Null data | `PCRE2_ERROR_NULL` (-51) | [x] |
| 92 | `pcre2_serialize_get_number_of_codes_8` | Bad magic | `PCRE2_ERROR_BADMAGIC` (-31) | [x] |
| 93 | `pcre2_serialize_get_number_of_codes_8` | Version or ABI config mismatch | `PCRE2_ERROR_BADMODE` (-32) | [x] |
| 94 | `pcre2_substitute_8` | Partial matching without REPLACEMENT_ONLY | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 95 | `pcre2_substitute_8` | Null replacement with nonzero length | `PCRE2_ERROR_NULL` (-51) | [x] |
| 96 | `pcre2_substitute_8` | Null subject with nonzero length | `PCRE2_ERROR_NULL` (-51) | [x] |
| 97 | `pcre2_substitute_8` | MATCHED option with null match data | `PCRE2_ERROR_NULL` (-51) | [x] |
| 98 | `pcre2_substitute_8` | Existing match came from DFA | `PCRE2_ERROR_DFA_UFUNC` (-41) | [x] |
| 99 | `pcre2_substitute_8` | Existing match code differs | `PCRE2_ERROR_DIFFSUBSPATTERN` (-71) | [x] |
| 100 | `pcre2_substitute_8` | Existing match subject/length differs | `PCRE2_ERROR_DIFFSUBSSUBJECT` (-72) | [x] |
| 101 | `pcre2_substitute_8` | Existing match start offset differs | `PCRE2_ERROR_DIFFSUBSOFFSET` (-73) | [x] |
| 102 | `pcre2_substitute_8` | Existing match options differ | `PCRE2_ERROR_DIFFSUBSOPTIONS` (-74) | [x] |
| 103 | `pcre2_substitute_8` | Replacement references absent capture | `PCRE2_ERROR_NOSUBSTRING` (-49) | [x] |
| 104 | `pcre2_substitute_8` | Unknown replacement name without UNKNOWN_UNSET | `PCRE2_ERROR_BADSUBSTITUTION` (-59) | [x] |
| 105 | `pcre2_substitute_8` | Replacement has missing closing brace | `PCRE2_ERROR_REPMISSINGBRACE` (-58) | [x] |
| 106 | `pcre2_substitute_8` | Replacement has malformed backslash escape | `PCRE2_ERROR_BADREPESCAPE` (-57) | [x] |
| 107 | `pcre2_substitute_8` | Output buffer too small without overflow-length option | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 108 | `pcre2_pattern_convert_8` | Null pattern with nonzero length or null length pointer | `PCRE2_ERROR_NULL` (-51) | [x] |
| 109 | `pcre2_pattern_convert_8` | Undefined option, no type, or multiple type bits | `PCRE2_ERROR_BADOPTION` (-34) | [x] |
| 110 | `pcre2_pattern_convert_8` | Invalid UTF input with checks enabled | exact `PCRE2_ERROR_UTF8_ERR*` | [x] |
| 111 | `pcre2_pattern_convert_8` | Trailing escape, malformed class, or converter syntax error | exact 101, 106, or -64 | [x] |
| 112 | `pcre2_pattern_convert_8` | Caller output buffer too small | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 113 | `pcre2_pattern_convert_8` | Library allocator fails | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 114 | `pcre2_jit_compile_8` | TEST_ALLOC mixed with any other bit | `PCRE2_ERROR_JIT_BADOPTION` (-45) | [x] |
| 115 | `pcre2_jit_compile_8` | TEST_ALLOC alone in no-JIT build | `PCRE2_ERROR_JIT_UNSUPPORTED` (-68) | [x] |
| 116 | `pcre2_jit_compile_8` | Null code (without TEST_ALLOC) | `PCRE2_ERROR_NULL` (-51) | [x] |
| 117 | `pcre2_jit_compile_8` | Undefined JIT option | `PCRE2_ERROR_JIT_BADOPTION` (-45) | [x] |
| 118 | `pcre2_jit_compile_8` | Any valid compile mode in no-JIT build | `PCRE2_ERROR_JIT_BADOPTION` (-45) | [x] |
| 119 | `pcre2_jit_match_8` | Direct call in no-JIT build | match-data `rc = PCRE2_ERROR_JIT_BADOPTION` (-45) | [x] |
| 120 | `pcre2_jit_stack_create_8` | Any sizes in no-JIT build | null | [x] |
| 121 | `pcre2_get_error_message_8` | Buffer size zero | `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
| 122 | `pcre2_get_error_message_8` | Unknown positive/negative error number | `PCRE2_ERROR_BADDATA` (-29) | [x] |
| 123 | `pcre2_get_error_message_8` | Buffer shorter than message plus terminator | truncated NUL string; `PCRE2_ERROR_NOMEMORY` (-48) | [x] |
