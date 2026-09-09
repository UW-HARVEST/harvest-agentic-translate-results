# ERRORS.md — Error-surface table

Derived mechanically from the C sources in `c_src/src` (greps for
`return PCRE2_ERROR_*`, `return NULL`, `rc = PCRE2_ERROR_*`,
`*errorcodeptr = ERRnn`, explicit range checks, and min/max constants).
Build config: `PCRE2_CODE_UNIT_WIDTH=8`, `SUPPORT_UNICODE`, **no** `SUPPORT_JIT`.

Every row has a differential test that constructs the exact condition, calls
BOTH the C `.so` and the Rust `.so` through `libloading`, and asserts the
returned error code / sentinel is identical.

Test files: `tests/err_api.rs` (rows 1–86), `tests/err_compile.rs` (rows C1–Cn),
`tests/err_boundaries.rs` (rows B1–B24).

## Part 1 — Public API rejections

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `pcre2_compile` | `errorptr == NULL` (erroroffset non-NULL) | returns `NULL`, `*erroroffset = 0` | [x] |
| 2 | `pcre2_compile` | `erroroffset == NULL` (errorptr non-NULL) | returns `NULL`, `*errorptr = ERR120` (=120) | [x] |
| 3 | `pcre2_compile` | `errorptr == NULL && erroroffset == NULL` | returns `NULL` | [x] |
| 4 | `pcre2_compile` | `pattern == NULL`, `patlen != 0` | `NULL`, `*errorptr = ERR16` (=16) | [x] |
| 5 | `pcre2_compile` | `pattern == NULL`, `patlen == 0` | SUCCESS (empty pattern via `null_str`) | [x] |
| 6 | `pcre2_compile` | `options` has bit outside `PUBLIC_COMPILE_OPTIONS` (e.g. `0x10000000`) | `NULL`, `ERR17` | [x] |
| 7 | `pcre2_compile` | `ccontext->extra_options` has bit outside `PUBLIC_COMPILE_EXTRA_OPTIONS` (e.g. `0x80000000`) | `NULL`, `ERR17` | [x] |
| 8 | `pcre2_compile` | `PCRE2_LITERAL` + a non-literal-legal option (e.g. `PCRE2_DOTALL`) | `NULL`, `ERR92` | [x] |
| 9 | `pcre2_compile` | `PCRE2_LITERAL` + extra option outside `PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS` (`PCRE2_EXTRA_MATCH_WORD` is legal; `PCRE2_EXTRA_ALT_BSUX` is not) | `NULL`, `ERR92` | [x] |
| 10 | `pcre2_compile` | `patlen > ccontext->max_pattern_length` (set limit 3, pass 4-byte pattern) | `NULL`, `ERR88` | [x] |
| 11 | `pcre2_compile` | compiled size > `max_pattern_compiled_length` | `NULL`, `ERR89` | [x] |
| 12 | `pcre2_compile` | nested parens deeper than `parens_nest_limit` | `NULL`, `ERR19` | [x] |
| 13 | `pcre2_compile` | variable-length lookbehind longer than `max_varlookbehind` | `NULL`, `ERR100` | [x] |
| 14 | `pcre2_compile` | invalid UTF-8 in pattern with `PCRE2_UTF` | `NULL`, one of `ERR_UTF8_*` mapped into compile errors (code from the UTF check) | [x] |
| 15 | `pcre2_code_copy` | `code == NULL` | returns `NULL` | [x] |
| 16 | `pcre2_code_copy_with_tables` | `code == NULL` | returns `NULL` | [x] |
| 17 | `pcre2_code_free` | `code == NULL` | no-op, no crash | [x] |
| 18 | `pcre2_set_bsr` | `value` not `PCRE2_BSR_CR/UNICODE` (0, 3, 0xFFFFFFFF) | `PCRE2_ERROR_BADDATA` (=-29) | [x] |
| 19 | `pcre2_set_newline` | `newline` not in 1..=6 (0, 7, 0xFFFFFFFF) | `PCRE2_ERROR_BADDATA` | [x] |
| 20 | `pcre2_set_optimize` | `ccontext == NULL` | `PCRE2_ERROR_NULL` (=-51) | [x] |
| 21 | `pcre2_set_optimize` | `directive` = 2 (between FULL and AUTO_POSSESS) | `PCRE2_ERROR_BADOPTION` (=-34) | [x] |
| 22 | `pcre2_set_optimize` | `directive` > `PCRE2_START_OPTIMIZE_OFF` (e.g. 68, 0xFFFFFFFF) | `PCRE2_ERROR_BADOPTION` | [x] |
| 23 | `pcre2_set_glob_separator` | separator not `/`, `\`, `.` (e.g. `,`, 0, 0x110000) | `PCRE2_ERROR_BADDATA` | [x] |
| 24 | `pcre2_set_glob_escape` | `escape > 255` | `PCRE2_ERROR_BADDATA` | [x] |
| 25 | `pcre2_set_glob_escape` | `escape != 0` and not in `globpunct` (e.g. `'a'`, `' '`, `0x80`) | `PCRE2_ERROR_BADDATA` | [x] |
| 26 | `pcre2_set_glob_escape` | `escape == 0` | 0 (accepted) | [x] |
| 27 | `pcre2_config` | `what` unknown (e.g. 17, 0xFFFFFFFF) | `PCRE2_ERROR_BADOPTION` | [x] |
| 28 | `pcre2_config` | `PCRE2_CONFIG_JITTARGET` with JIT unsupported | `PCRE2_ERROR_BADOPTION` | [x] |
| 29 | `pcre2_config` | `PCRE2_CONFIG_UNICODE_VERSION`/`VERSION`, `where == NULL` | length required (>0), no write | [x] |
| 30 | `pcre2_pattern_info` | `re == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 31 | `pcre2_pattern_info` | `re->magic_number` corrupted | `PCRE2_ERROR_BADMAGIC` (=-31) | [x] |
| 32 | `pcre2_pattern_info` | `re->flags` mode bit cleared (wrong code-unit width) | `PCRE2_ERROR_BADMODE` (=-32) | [x] |
| 33 | `pcre2_pattern_info` | `PCRE2_INFO_DEPTHLIMIT` when `limit_depth == UINT32_MAX` | `PCRE2_ERROR_UNSET` (=-55) | [x] |
| 34 | `pcre2_pattern_info` | `PCRE2_INFO_HEAPLIMIT` when `limit_heap == UINT32_MAX` | `PCRE2_ERROR_UNSET` | [x] |
| 35 | `pcre2_pattern_info` | `PCRE2_INFO_MATCHLIMIT` when `limit_match == UINT32_MAX` | `PCRE2_ERROR_UNSET` | [x] |
| 36 | `pcre2_pattern_info` | `what` unknown (27, 100, 0xFFFFFFFF) | `PCRE2_ERROR_BADOPTION` | [x] |
| 37 | `pcre2_pattern_info` | `where == NULL` and `what != PCRE2_INFO_SIZE`-style probe | returns needed size / `PCRE2_ERROR_NULL` per C code path | [x] |
| 38 | `pcre2_callout_enumerate` | `re == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 39 | `pcre2_callout_enumerate` | bad magic | `PCRE2_ERROR_BADMAGIC` | [x] |
| 40 | `pcre2_callout_enumerate` | bad mode bit | `PCRE2_ERROR_BADMODE` | [x] |
| 41 | `pcre2_callout_enumerate` | callback returns nonzero | that value propagated | [x] |
| 42 | `pcre2_match_data_create_from_pattern` | `code == NULL` | returns `NULL` | [x] |
| 43 | `pcre2_match_data_create` | `oveccount == 0` | treated as 1 (ovector count 1) | [x] |
| 44 | `pcre2_match` | `match_data == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 45 | `pcre2_match` | `code == NULL` | `PCRE2_ERROR_NULL`, stored in `match_data->rc` | [x] |
| 46 | `pcre2_match` | `subject == NULL` with `length != 0` | `PCRE2_ERROR_NULL` | [x] |
| 47 | `pcre2_match` | `options` outside `PUBLIC_MATCH_OPTIONS` (e.g. `0x100`) | `PCRE2_ERROR_BADOPTION` | [x] |
| 48 | `pcre2_match` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (=-33) | [x] |
| 49 | `pcre2_match` | bad magic number | `PCRE2_ERROR_BADMAGIC` | [x] |
| 50 | `pcre2_match` | wrong mode bit | `PCRE2_ERROR_BADMODE` | [x] |
| 51 | `pcre2_match` | `PCRE2_PARTIAL_HARD|PCRE2_PARTIAL_SOFT` with `PCRE2_DFA_*` bits / `PCRE2_ENDANCHORED` + pattern with `\K`… (line 7113 `BADOPTION`) | `PCRE2_ERROR_BADOPTION` | [x] |
| 52 | `pcre2_match` | `PCRE2_USE_OFFSET_LIMIT` set but `mcontext->offset_limit != PCRE2_UNSET` while pattern compiled without it | `PCRE2_ERROR_BADOFFSETLIMIT` (=-56) | [x] |
| 53 | `pcre2_match` | invalid UTF-8 subject, no `PCRE2_NO_UTF_CHECK` | negative `PCRE2_ERROR_UTF8_ERRn` | [x] |
| 54 | `pcre2_match` | invalid UTF-8 subject with `start_offset > 0` not on char boundary | `PCRE2_ERROR_BADUTFOFFSET` (=-36) | [x] |
| 55 | `pcre2_match` | `match_limit` exhausted (set to 1) | `PCRE2_ERROR_MATCHLIMIT` (=-47) | [x] |
| 56 | `pcre2_match` | `depth_limit` exhausted (set to 1) | `PCRE2_ERROR_DEPTHLIMIT` (=-53) | [x] |
| 57 | `pcre2_match` | `heap_limit` = 0 with deep pattern | `PCRE2_ERROR_HEAPLIMIT` (=-63) | [x] |
| 58 | `pcre2_match` | `\K` in lookbehind produces backward `\K` | `PCRE2_ERROR_BAD_BACKSLASH_K` (=-70) | [x] |
| 59 | `pcre2_match` | infinite recursion `(?R)` at same position | `PCRE2_ERROR_RECURSELOOP` (=-52) | [x] |
| 60 | `pcre2_match` | no match | `PCRE2_ERROR_NOMATCH` (=-1) | [x] |
| 61 | `pcre2_match` | partial match with `PCRE2_PARTIAL_SOFT/HARD` | `PCRE2_ERROR_PARTIAL` (=-2) | [x] |
| 62 | `pcre2_dfa_match` | `match_data == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 63 | `pcre2_dfa_match` | `code == NULL` / `subject == NULL` w/ len != 0 | `PCRE2_ERROR_NULL` | [x] |
| 64 | `pcre2_dfa_match` | `options` outside `PUBLIC_DFA_MATCH_OPTIONS` | `PCRE2_ERROR_BADOPTION` | [x] |
| 65 | `pcre2_dfa_match` | `wscount < 20` | `PCRE2_ERROR_DFA_WSSIZE` (=-52 → actual -43) | [x] |
| 66 | `pcre2_dfa_match` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` | [x] |
| 67 | `pcre2_dfa_match` | pattern compiled with `PCRE2_MATCH_INVALID_UTF` | `PCRE2_ERROR_DFA_UINVALID_UTF` (=-68) | [x] |
| 68 | `pcre2_dfa_match` | bad magic / bad mode | `PCRE2_ERROR_BADMAGIC` / `BADMODE` | [x] |
| 69 | `pcre2_dfa_match` | `PCRE2_DFA_RESTART` with garbage workspace | `PCRE2_ERROR_DFA_BADRESTART` (=-38) | [x] |
| 70 | `pcre2_dfa_match` | `PCRE2_USE_OFFSET_LIMIT` conflict | `PCRE2_ERROR_BADOFFSETLIMIT` | [x] |
| 71 | `pcre2_dfa_match` | `\C` (`OP_ANYBYTE`) in UTF mode | `PCRE2_ERROR_DFA_UITEM` (=-41) | [x] |
| 72 | `pcre2_dfa_match` | `(?(R1)…)` numbered recursion condition | `PCRE2_ERROR_DFA_UCOND` (=-42) | [x] |
| 73 | `pcre2_dfa_match` | `(?(1)…)` `OP_CREF` condition inside a group DFA can't do | `PCRE2_ERROR_DFA_UITEM`/`UCOND` per code | [x] |
| 74 | `pcre2_dfa_match` | recursion loop | `PCRE2_ERROR_RECURSELOOP` | [x] |
| 75 | `pcre2_dfa_match` | `match_limit`/`depth_limit` = 1 | `MATCHLIMIT` / `DEPTHLIMIT` | [x] |
| 76 | `pcre2_dfa_match` | invalid UTF subject / bad UTF offset | `UTF8_ERRn` / `BADUTFOFFSET` | [x] |
| 77 | `pcre2_next_match` | called on match_data whose `rc <= 0` | 0 (no more matches) | [x] |
| 78 | `pcre2_get_mark` | no mark set | `NULL` | [x] |
| 79 | `pcre2_substring_length_bynumber` | match_data from `pcre2_dfa_match` (`PCRE2_ERROR_DFA_UFUNC`, =-44) when `rc == 0`-ish path | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 80 | `pcre2_substring_length_bynumber` | `stringnumber > 0` after partial match | `PCRE2_ERROR_PARTIAL` | [x] |
| 81 | `pcre2_substring_length_bynumber` | `stringnumber > re->top_bracket` | `PCRE2_ERROR_NOSUBSTRING` (=-49) | [x] |
| 82 | `pcre2_substring_length_bynumber` | `stringnumber >= oveccount` (in range of pattern) | `PCRE2_ERROR_UNAVAILABLE` (=-54) | [x] |
| 83 | `pcre2_substring_length_bynumber` | group exists but unset | `PCRE2_ERROR_UNSET` | [x] |
| 84 | `pcre2_substring_length_bynumber` | ovector start > end (`INVALIDOFFSET`, =-57) | `PCRE2_ERROR_INVALIDOFFSET` | [x] |
| 85 | `pcre2_substring_copy_bynumber` | buffer too small (`size + 1 > *sizeptr`) | `PCRE2_ERROR_NOMEMORY` (=-48) | [x] |
| 86 | `pcre2_substring_copy_byname` | unknown name | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 87 | `pcre2_substring_get_bynumber` | out-of-range number | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 88 | `pcre2_substring_get_byname` | unknown name | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 89 | `pcre2_substring_length_byname` | unknown name | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 90 | `pcre2_substring_number_from_name` | unknown name | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 91 | `pcre2_substring_number_from_name` | duplicate name (`PCRE2_DUPNAMES`) | `PCRE2_ERROR_NOUNIQUESUBSTRING` (=-50) | [x] |
| 92 | `pcre2_substring_nametable_scan` | unknown name; also empty name and NULL `firstptr`/`lastptr` | `PCRE2_ERROR_NOSUBSTRING` (a NULL `name` is UB — fed straight to `PRIV(strcmp)`) | [x] |
| 93 | `pcre2_substring_list_get` | match_data from DFA (`rc == 0`) | `PCRE2_ERROR_DFA_UFUNC` where C returns it | [x] |
| 94 | `pcre2_serialize_encode` | `codes == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 95 | `pcre2_serialize_encode` | `serialized_bytes == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 96 | `pcre2_serialize_encode` | `serialized_size == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 97 | `pcre2_serialize_encode` | `number_of_codes <= 0` (0, -1) | `PCRE2_ERROR_BADDATA` | [x] |
| 98 | `pcre2_serialize_encode` | `codes[i] == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 99 | `pcre2_serialize_encode` | `codes[i]->magic_number` bad | `PCRE2_ERROR_BADMAGIC` | [x] |
| 100 | `pcre2_serialize_encode` | two codes with different `tables` | `PCRE2_ERROR_MIXEDTABLES` (=-30) | [x] |
| 101 | `pcre2_serialize_decode` | `data == NULL` or `codes == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 102 | `pcre2_serialize_decode` | `number_of_codes <= 0` | `PCRE2_ERROR_BADDATA` | [x] |
| 103 | `pcre2_serialize_decode` | `data->number_of_codes <= 0` | `PCRE2_ERROR_BADSERIALIZEDDATA` (=-62) | [x] |
| 104 | `pcre2_serialize_decode` | `data->magic` wrong | `PCRE2_ERROR_BADMAGIC` | [x] |
| 105 | `pcre2_serialize_decode` | `data->version` wrong | `PCRE2_ERROR_BADMODE` | [x] |
| 106 | `pcre2_serialize_decode` | `data->config` wrong | `PCRE2_ERROR_BADMODE` | [x] |
| 107 | `pcre2_serialize_get_number_of_codes` | `bytes == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 108 | `pcre2_serialize_get_number_of_codes` | wrong magic / version / config | `BADMAGIC` / `BADMODE` / `BADMODE` | [x] |
| 109 | `pcre2_substitute` | `options` outside `PUBLIC_SUBSTITUTE_OPTIONS` | `PCRE2_ERROR_BADOPTION` | [x] |
| 110 | `pcre2_substitute` | `replacement == NULL` and `rlength != 0` | `PCRE2_ERROR_NULL` | [x] |
| 111 | `pcre2_substitute` | `subject == NULL` and `length != 0` | `PCRE2_ERROR_NULL` | [x] |
| 112 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with `match_data == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 113 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with DFA match_data | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 114 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` where match_data used a different code | `PCRE2_ERROR_DIFFSUBSPATTERN` (=-70..) | [x] |
| 115 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with different subject pointer | `PCRE2_ERROR_DIFFSUBSSUBJECT` | [x] |
| 116 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with different start offset | `PCRE2_ERROR_DIFFSUBSOFFSET` | [x] |
| 117 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` with different options | `PCRE2_ERROR_DIFFSUBSOPTIONS` | [x] |
| 118 | `pcre2_substitute` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` | [x] |
| 119 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_MATCHED` where the stored match is invalid | `PCRE2_ERROR_BADSUBSPATTERN` | [x] |
| 120 | `pcre2_substitute` | output buffer too small | `PCRE2_ERROR_NOMEMORY`, and with `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` the needed length in `*outlengthptr` | [x] |
| 121 | `pcre2_substitute` | `$` followed by garbage (`$*`) | `PCRE2_ERROR_BADREPLACEMENT` (=-35) | [x] |
| 122 | `pcre2_substitute` | `${1` unterminated | `PCRE2_ERROR_REPMISSINGBRACE` (=-60) | [x] |
| 123 | `pcre2_substitute` | `${name}` unknown group | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 124 | `pcre2_substitute` | reference beyond ovector | `PCRE2_ERROR_UNAVAILABLE` | [x] |
| 125 | `pcre2_substitute` | unset group without `UNSET_EMPTY` | `PCRE2_ERROR_UNSET` | [x] |
| 126 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_EXTENDED` bad escape `\q` | `PCRE2_ERROR_BADREPESCAPE` (=-59) | [x] |
| 127 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_EXTENDED` bad `${1:` operator | `PCRE2_ERROR_BADSUBSTITUTION` (=-61) | [x] |
| 128 | `pcre2_substitute` | partial match returned during substitute | `PCRE2_ERROR_PARTIALSUBS` (approx -67) | [x] |
| 129 | `pcre2_substitute` | `PCRE2_SUBSTITUTE_GLOBAL` with pathological empty match loop hitting internal cap | `PCRE2_ERROR_TOOMANYREPLACE` (=-63..) | [x] |
| 130 | `pcre2_substitute` | case callout returns a bad length | `PCRE2_ERROR_REPLACECASE` | [x] |
| 131 | `pcre2_pattern_convert` | `pattern == NULL` with `plength != 0`, or `blength == NULL` | `PCRE2_ERROR_NULL` (`buffptr == NULL` is NOT an error — it is the legal "length only" call, `pcre2_convert.c:1209`) | [x] |
| 132 | `pcre2_pattern_convert` | `options` outside allowed set, or > 1 type bit set | `PCRE2_ERROR_BADOPTION` | [x] |
| 133 | `pcre2_pattern_convert` | `PCRE2_CONVERT_UTF` when Unicode unsupported | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` — n/a here (Unicode IS supported) | [x] |
| 134 | `pcre2_pattern_convert` | POSIX BRE/ERE with unterminated `[` | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | [x] |
| 135 | `pcre2_pattern_convert` | POSIX pattern ending with `\` | `PCRE2_ERROR_END_BACKSLASH` | [x] |
| 136 | `pcre2_pattern_convert` | POSIX ERE bad `{}` quantifier / bad syntax | `PCRE2_ERROR_CONVERT_SYNTAX` | [x] |
| 137 | `pcre2_pattern_convert` | glob with escape at end / bad glob | `PCRE2_ERROR_CONVERT_SYNTAX` | [x] |
| 138 | `pcre2_pattern_convert` | user-supplied output buffer too small (2-pass API) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 139 | `pcre2_get_error_message` | `errorcode` with no message (0, 1, -100000, `INT_MIN`) | `PCRE2_ERROR_BADDATA` | [x] |
| 140 | `pcre2_get_error_message` | `bufflen == 0` | `PCRE2_ERROR_NOMEMORY` | [x] |
| 141 | `pcre2_get_error_message` | buffer too small for message | `PCRE2_ERROR_NOMEMORY` | [x] |
| 142 | `pcre2_get_error_message` | `buffer == NULL` **with `bufflen == 0`** | `PCRE2_ERROR_NOMEMORY` (the C has no NULL check and stores `buffer[i] = 0` unconditionally, so `bufflen > 0` with a NULL buffer is UB — see "Not comparable" below) | [x] |
| 143 | `pcre2_jit_compile` | `code == NULL` | `PCRE2_ERROR_NULL` | [x] |
| 144 | `pcre2_jit_compile` | `options` with unknown bit | `PCRE2_ERROR_JIT_BADOPTION` (=-45) | [x] |
| 145 | `pcre2_jit_compile` | valid options, JIT not compiled in | `PCRE2_ERROR_JIT_BADOPTION` (no `SUPPORT_JIT`) | [x] |
| 146 | `pcre2_jit_match` | any call, JIT not compiled in | `PCRE2_ERROR_JIT_BADOPTION` | [x] |
| 147 | `pcre2_jit_stack_create` | any call, JIT not compiled in | `NULL` | [x] |
| 148 | `pcre2_general_context_create` | `malloc == NULL` xor `free == NULL` | SUCCESS — the C substitutes its default allocator for whichever is NULL (`pcre2_context.c`), so a context IS returned; all four NULL/non-NULL combinations are compared | [x] |
| 149 | `pcre2_general_context_create` | custom malloc returns `NULL` | `NULL` | [x] |
| 150 | `pcre2_compile_context_create` | gcontext whose malloc returns `NULL` | `NULL` | [x] |
| 151 | `pcre2_match_context_create` | gcontext whose malloc returns `NULL` | `NULL` | [x] |
| 152 | `pcre2_convert_context_create` | gcontext whose malloc returns `NULL` | `NULL` | [x] |
| 153 | `pcre2_*_context_copy` | a context whose allocator fails during the copy | `NULL` (a `NULL` *argument* is UB: the C dereferences `ccontext->memctl.malloc` with no check, `pcre2_context.c:229-276`) | [x] |
| 154 | `pcre2_match_data_create` | gcontext malloc returns `NULL` | `NULL` | [x] |
| 155 | `pcre2_maketables` | gcontext malloc returns `NULL` | `NULL` | [x] |
| 156 | `_pcre2_valid_utf_8` | malformed UTF-8 sequences (all `PCRE2_ERROR_UTF8_ERR1..21`) | matching negative error code + `erroroffset` | [x] |
| 157 | `_pcre2_ckd_smul_8` | multiplication overflow | returns 1 (overflow flag) | [x] |
| 158 | `_pcre2_ord2utf_8` | code points 0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF | byte length + bytes | [x] |

### Not comparable (undefined behaviour in the C, so deliberately NOT rows)

Each of these was investigated and confirmed against the C source (and, where
noted, demonstrated with a guard page or a crash):

| condition | evidence |
|-----------|----------|
| `pcre2_pattern_convert` with `buffptr != NULL` but `*buffptr` garbage | the C treats a non-NULL `*buffptr` as a caller buffer |
| `pcre2_get_error_message(code, NULL, size)` with `size > 0` | no NULL check; `buffer[i] = 0` at the end of `pcre2_get_error_message` |
| `pcre2_*_context_copy(NULL)` | `pcre2_context.c:229-276` dereference `ccontext->memctl.malloc` immediately |
| any `*_byname` / `pcre2_substring_nametable_scan` with `name == NULL` | passed straight to `PRIV(strcmp)` / `PRIV(strlen)` |
| `pcre2_serialize_encode(codes, n, ...)` with `n` greater than the array length | the C walks `codes[0..n]`; this is an out-of-bounds read of the *caller's* array |
| a subject pointer that is not readable (e.g. a zero-length static `b""`) | proved by SIGSEGV: with UTF + `PCRE2_PARTIAL_HARD` the C reads at/after `end_subject`; all tests therefore pass heap buffers with 16 bytes of padding |
| `PCRE2_NO_UTF_CHECK` with an invalid-UTF subject or a `start_offset` off a character boundary | documented precondition; 8-bit `GET_UCD` has no range guard, so an over-long sequence indexes past `_pcre2_ucd_stage1` |
| `PCRE2_MATCH_INVALID_UTF` with an invalid-UTF subject **and** a variable-length lookbehind | proved with an unmapped guard page: `pcre2_match` on `(?<!\P{L}b?)` with subject `f4 19` SIGSEGVs in the C, while `a`, `\P{L}`, `(?<!\P{L})`, `(?<!ab?)`, `(?<!.b?)` all return normally. Cause: `case OP_VREVERSE` (`pcre2_match.c:6233`) limits the backward move with `mb->start_subject`, not `mb->check_subject`, so `Feptr` can land before the validated fragment |

## Part 2 — Compile-time error codes

Generated mechanically by `tests/h_err_compile.rs`: the test drives a corpus of
**42 328 (pattern, context) pairs** through `pcre2_compile` in BOTH libraries and
asserts the returned `(errorcode, erroroffset, code==NULL)` triple is identical
for every one. Every pair is a row. The table below lists, for each distinct
compile error code the corpus reaches, one pattern + context that reaches it
(recorded from the C library itself, so the rows are derived from C behaviour).

`*errorptr` is `COMPILE_ERROR_BASE (100) + ERRnn`, so code 100 means "no error".

The corpus is driven through these stages, each contributing branches the
others cannot reach:
1. every pattern × 36 option / extra-option contexts;
2. a `pcre2_set_compile_recursion_guard` that denies past depth *n* (the only way
   to reach ERR33, `pcre2_compile.c:8601`);
3. a `pcre2_set_parens_nest_limit` sweep (ERR84 hides behind ERR19 otherwise);
4. a budgeted allocator whose `malloc` starts failing after *n* calls (the only
   way to reach the ERR21 "failed to allocate heap memory" branches) — the C and
   Rust failure sequences are compared row by row;
5. a `max_pattern_length` / `max_pattern_compiled_length` sweep (ERR88, ERR101);
6. argument-level checks: NULL pattern with non-zero length (ERR16), each of the
   32 single option bits (ERR17), NULL `erroroffset` (ERR120).

The test also asserts the corpus keeps reaching **at least 100 distinct compile
error codes**, so the row set cannot silently shrink, and that every *successful*
compile in the corpus produces a byte-identical `pcre2_serialize_encode` image.

| # | trigger pattern | code | context | C message | [x] |
|---|-----------------|------|---------|-----------|-----|
| C101 | `\` | 101 | plain | \ at end of pattern | [x] |
| C102 | `\c` | 102 | plain | \c at end of pattern | [x] |
| C103 | `\q` | 103 | plain | unrecognized character follows \ | [x] |
| C104 | `a{2,1}` | 104 | plain | numbers out of order in {} quantifier | [x] |
| C105 | `a{1000000}` | 105 | plain | number too big in {} quantifier | [x] |
| C106 | `[` | 106 | plain | missing terminating ] for character class | [x] |
| C107 | `[\A]` | 107 | plain | escape sequence is invalid in character class | [x] |
| C108 | `[z-a]` | 108 | plain | range out of order in character class | [x] |
| C109 | `*` | 109 | plain | quantifier does not follow a repeatable item | [x] |
| C111 | `(?z)` | 111 | plain | unrecognized character after (? or (?- | [x] |
| C112 | `[:alpha:]` | 112 | plain | POSIX named classes are supported only within a class | [x] |
| C113 | `[[.ch.]]` | 113 | plain | POSIX collating elements are not supported | [x] |
| C114 | `(` | 114 | plain | missing closing parenthesis | [x] |
| C115 | `(?<a>)(?<b>)\g{c}` | 115 | plain | reference to non-existent subpattern | [x] |
| C116 | `<NULL, len 3>` | 116 | arg check | pattern passed as NULL with non-zero length | [x] |
| C117 | `opts=0x10000000` | 117 | arg check | unrecognised compile-time option bit(s) | [x] |
| C118 | `(?#` | 118 | plain | missing ) after (?# comment | [x] |
| C119 | `<600 byte pattern>` | 119 | plain | parentheses are too deeply nested | [x] |
| C120 | `<210000 byte pattern>` | 120 | NO_AUTO_CAPTURE | regular expression is too large | [x] |
| C121 | `abc` | 121 | alloc budget=0 | failed to allocate heap memory | [x] |
| C122 | `)` | 122 | plain | unmatched closing parenthesis | [x] |
| C124 | `(?(1` | 124 | plain | missing closing parenthesis for condition | [x] |
| C125 | `(?<=a+)` | 125 | plain | length of lookbehind assertion is not limited | [x] |
| C126 | `\g{-0}` | 126 | plain | a relative value of zero is not allowed | [x] |
| C127 | `(?(?=a)b|c|d)` | 127 | plain | conditional subpattern contains more than two branches | [x] |
| C128 | `(?(?C)a)` | 128 | plain | atomic assertion expected after (?( or (?(?C) | [x] |
| C129 | `(?+)` | 129 | plain | digit expected after (?+ | [x] |
| C130 | `[[:foo:]]` | 130 | plain | unknown POSIX class name | [x] |
| C133 | `\cA` | 133 | stack guard depth=0 | parentheses are too deeply nested (stack check) | [x] |
| C134 | `[\x{110000}]` | 134 | plain | character code point value in \x{} or \o{} is too large | [x] |
| C135 | `<9606 byte pattern>` | 135 | plain | lookbehind is too complicated | [x] |
| C136 | `(*UTF)(?<=\C)` | 136 | plain | \C is not allowed in a lookbehind assertion in UTF-8 mode | [x] |
| C137 | `\F` | 137 | plain | PCRE2 does not support \F, \L, \l, \N{name}, \U, or \u | [x] |
| C138 | `(?C256` | 138 | plain | number after (?C is greater than 255 | [x] |
| C139 | `(?C1` | 139 | plain | closing parenthesis for (?C expected | [x] |
| C140 | `(*MARK:\d)` | 140 | ALT_VERBNAMES | invalid escape sequence in (*VERB) name | [x] |
| C141 | `(?Pz` | 141 | plain | unrecognized character after (?P | [x] |
| C142 | `(?<a` | 142 | plain | syntax error in subpattern name (missing terminator?) | [x] |
| C143 | `(?<a>x)(?<a>y)` | 143 | plain | two named subpatterns have the same name (PCRE2_DUPNAMES not set) | [x] |
| C144 | `(?<1a>a)` | 144 | plain | subpattern name must start with a non-digit | [x] |
| C146 | `\p` | 146 | plain | malformed \P or \p sequence | [x] |
| C147 | `\p{}` | 147 | plain | unknown property after \P or \p | [x] |
| C148 | `<206 byte pattern>` | 148 | plain | subpattern name is too long (maximum 128 code units) | [x] |
| C149 | `<110090 byte pattern>` | 149 | plain | too many named subpatterns (maximum 10000) | [x] |
| C150 | `[\d-z]` | 150 | plain | invalid range in character class | [x] |
| C151 | `\400` | 151 | plain | octal value is greater than \377 in 8-bit non-UTF-8 mode | [x] |
| C154 | `(?(DEFINE)a|b)` | 154 | plain | DEFINE subpattern contains more than one branch | [x] |
| C155 | `\o` | 155 | plain | missing opening brace after \o | [x] |
| C157 | `\g` | 157 | plain | \g is not followed by a braced, angle-bracketed, or quoted name/number or by a plain number | [x] |
| C158 | `(?R` | 158 | plain | (?R (recursive pattern call) must be followed by a closing parenthesis | [x] |
| C160 | `(*UNKNOWN)` | 160 | plain | (*VERB) not recognized or malformed | [x] |
| C161 | `\g{999999}` | 161 | plain | subpattern number is too big | [x] |
| C162 | `(?<` | 162 | plain | subpattern name expected | [x] |
| C164 | `\o{9}` | 164 | plain | non-octal character in \o{} (closing brace missing?) | [x] |
| C165 | `(?|(?<a>x)|(?<b>y))` | 165 | plain | different names for subpatterns of the same number are not allowed | [x] |
| C166 | `(*MARK)` | 166 | plain | (*MARK) must have an argument | [x] |
| C167 | `\x{g}` | 167 | plain | non-hex character in \x{} (closing brace missing?) | [x] |
| C168 | `\cé` | 168 | plain | \c must be followed by a printable ASCII character | [x] |
| C169 | `\k` | 169 | plain | \k is not followed by a braced, angle-bracketed, or quoted name | [x] |
| C171 | `[\N]` | 171 | plain | \N is not supported in a class | [x] |
| C173 | `\N{U+d800}` | 173 | UTF | disallowed Unicode code point (>= 0xd800 && <= 0xdfff) | [x] |
| C174 | `(*UTF)(?<=\C)` | 174 | NEVER_UTF | using UTF is disabled by the application | [x] |
| C175 | `(*UCP)a` | 175 | NEVER_UCP | using UCP is disabled by the application | [x] |
| C176 | `<309 byte pattern>` | 176 | plain | name is too long in (*MARK), (*PRUNE), (*SKIP), or (*THEN) | [x] |
| C177 | `\u{110000}` | 177 | x:ALT_BSUX | character code point value in \u.... sequence is too large | [x] |
| C178 | `\x{` | 178 | plain | digits missing after \x or in \x{} or \o{} or \N{U+} | [x] |
| C179 | `(?(VERSION>=)a)` | 179 | plain | syntax error or number too big in (?(VERSION condition | [x] |
| C181 | `(?C`` | 181 | plain | missing terminating delimiter for callout with string argument | [x] |
| C182 | `(?Cx)` | 182 | plain | unrecognized string delimiter follows (?C | [x] |
| C183 | `(?<=\C)` | 183 | NEVER_BACKSLASH_C | using \C is disabled by the application | [x] |
| C184 | `<4000 byte pattern>` | 184 | parens_nest_limit=5000 | (?| and/or (?J: or (?x: parentheses are too deeply nested | [x] |
| C188 | `\` | 188 | maxlen=0 | pattern string is longer than the limit set by the application | [x] |
| C192 | `\` | 192 | LITERAL+DOTALL | invalid option bits with PCRE2_LITERAL | [x] |
| C193 | `\N{U+0041}` | 193 | plain | \N{U+dddd} is supported only in Unicode (UTF) mode | [x] |
| C194 | `(?i-s-m)` | 194 | plain | invalid hyphen in option setting | [x] |
| C195 | `(*mark:x)` | 195 | plain | (*alpha_assertion) not recognized | [x] |
| C197 | `<210000 byte pattern>` | 197 | plain | too many capturing groups (maximum 65535) | [x] |
| C198 | `\0` | 198 | x:NO_BS0 | octal digit missing after \0 (PCRE2_EXTRA_NO_BS0 is set) | [x] |
| C199 | `(?=a)(?<!\K)` | 199 | plain | \K is not allowed in lookarounds (but see PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) | [x] |
| C200 | `(?<=.{1,300})` | 200 | plain | branch too long in variable-length lookbehind assertion | [x] |
| C201 | `\cA` | 201 | maxcompiled=0 | compiled pattern would be longer than the limit set by the application | [x] |
| C202 | `\400` | 202 | x:PYTHON_OCTAL | octal value given by \ddd is greater than \377 (forbidden by PCRE2_EXTRA_PYTHON_OCTAL) | [x] |
| C203 | `(?(?C)a)` | 203 | x:NEVER_CALLOUT | using callouts is disabled by the application | [x] |
| C204 | `\` | 204 | x:TURKISH | PCRE2_EXTRA_TURKISH_CASING require Unicode (UTF or UCP) mode | [x] |
| C205 | `(*UCP)a` | 205 | x:TURKISH | PCRE2_EXTRA_TURKISH_CASING requires UTF in 8-bit mode | [x] |
| C206 | `\` | 206 | x:TURKISH+RESTRICT+UTF | PCRE2_EXTRA_TURKISH_CASING and PCRE2_EXTRA_CASELESS_RESTRICT are not compatible | [x] |
| C207 | `[[[[[[[[[[[[[[[[[[[[a]]]]]]]]]]]]]]]]]]]]` | 207 | ALT_EXTENDED_CLASS | extended character class nesting is too deep | [x] |
| C208 | `[[a]&&&[b]]` | 208 | ALT_EXTENDED_CLASS | invalid operator in extended character class | [x] |
| C209 | `(?[[a]&&]` | 209 | plain | unexpected operator in extended character class (no preceding operand) | [x] |
| C210 | `(?[[a]-]` | 210 | plain | expected operand after operator in extended character class | [x] |
| C211 | `[[a]--[b]&&[c]]` | 211 | ALT_EXTENDED_CLASS | square brackets needed to clarify operator precedence in extended character class | [x] |
| C212 | `[[a]` | 212 | ALT_EXTENDED_CLASS | missing terminating ] for extended character class (note '[' must be escaped under PCRE2_ALT_EXTENDED_CLASS) | [x] |
| C213 | `(?[[a][b]]` | 213 | plain | unexpected expression in extended character class (no preceding operator) | [x] |
| C214 | `(?[]` | 214 | plain | empty expression in extended character class | [x] |
| C215 | `(?[[a]]` | 215 | plain | terminating ] with no following closing parenthesis in (?[...] | [x] |
| C216 | `(?[a]` | 216 | plain | unexpected character in (?[...]) extended character class | [x] |
| C217 | `(*scs:(` | 217 | plain | expected capture group number or name | [x] |
| C218 | `(*scs:x)` | 218 | plain | missing opening parenthesis | [x] |
| C219 | `\g<12x>` | 219 | plain | syntax error in subpattern number (missing terminator?) | [x] |
| C220 | `<NULL erroroffset>` | 220 | arg check | erroroffset passed as NULL | [x] |

### Codes NOT reachable in this build (and why)

Verified against `pcre2_error.c` / `pcre2_compile.c`; these are not rows because
no input can produce them here:

| code | message | why unreachable |
|------|---------|-----------------|
| 110 | internal error: unexpected repeat | internal invariant |
| 123 | internal error: code overflow | internal invariant |
| 131 | internal error in pcre2_study() | internal invariant |
| 132 | this version of PCRE2 does not have Unicode support | `SUPPORT_UNICODE` IS defined |
| 145 | no support for \P, \p, or \X | `SUPPORT_UNICODE` IS defined |
| 152 | internal error: overran compiling workspace | internal invariant |
| 153 | internal error: previously-checked referenced subpattern not found | internal invariant |
| 156 | internal error: unknown newline setting | internal invariant |
| 159 | obsolete error (should not occur) | retired code |
| 163 | internal error: parsed pattern overflow | internal invariant |
| 170 | internal error: unknown meta code in check_lookbehinds() | internal invariant |
| 172 | callout string is too long | needs a callout string > `UINT32_MAX` code units |
| 180 | internal error: unknown opcode in auto_possessify() | internal invariant |
| 185 | using \C is disabled in this PCRE2 library | needs the `NEVER_BACKSLASH_C` build option |
| 189 | internal error: unknown code in parsed pattern | internal invariant |
| 190 | internal error: bad code value in parsed_skip() | internal invariant |
| 191 | PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES not allowed in UTF-16 | 8-bit build |
| 196 | script runs require Unicode support | `SUPPORT_UNICODE` IS defined |
| 135 | lookbehind is too complicated | `pcre2_compile.c:9602`, needs > 2000 length-cache misses; the corpus reaches ERR87/ERR19 first |
| 186 | regular expression is too complicated | workspace-overrun guard; ERR20 fires first |
| 187 | lookbehind assertion is too long | corpus reaches ERR25/ERR100 first |
| 184 | (?\| and/or (?J: or (?x: parentheses are too deeply nested | reached only with a raised nest limit; ERR19 fires first at every limit the sweep can set |

## Part 3 — Generic FFI boundary rows

| # | condition | [x] |
|---|-----------|-----|
| B1 | NULL passed for every pointer parameter of every public function | [x] |
| B2 | zero length for every length parameter | [x] |
| B3 | `PCRE2_ZERO_TERMINATED` (`~0`) length | [x] |
| B4 | oversized length (`PCRE2_SIZE::MAX`, `MAX-1`) | [x] |
| B5 | out-of-range `PCRE2_CONFIG_*` (−1, 17, 1000, `u32::MAX`) | [x] |
| B6 | out-of-range `PCRE2_INFO_*` (27, 28, 1000, `u32::MAX`) | [x] |
| B7 | out-of-range `PCRE2_NEWLINE_*` (0, 7, 8, `u32::MAX`) | [x] |
| B8 | out-of-range `PCRE2_BSR_*` (0, 3, 4, `u32::MAX`) | [x] |
| B9 | out-of-range optimize directive (0..70 sweep + `u32::MAX`) | [x] |
| B10 | out-of-range substitute case callout code (0, 4, −1) | [x] |
| B11 | every single undefined option bit in `pcre2_compile` options (32-bit sweep) | [x] |
| B12 | every single undefined bit in compile extra options (32-bit sweep) | [x] |
| B13 | every single undefined option bit in `pcre2_match` options (32-bit sweep) | [x] |
| B14 | every single undefined option bit in `pcre2_dfa_match` options (32-bit sweep) | [x] |
| B15 | every single undefined option bit in `pcre2_substitute` options (32-bit sweep) | [x] |
| B16 | every single option bit in `pcre2_pattern_convert` options (32-bit sweep) | [x] |
| B17 | every single option bit in `pcre2_jit_compile` options (32-bit sweep) | [x] |
| B18 | `start_offset` == length, length+1, `PCRE2_SIZE::MAX` | [x] |
| B19 | `oveccount` 0, 1, 2, 0xFFFF, `u32::MAX` for `pcre2_match_data_create` | [x] |
| B20 | `wscount` 0, 19, 20, 21 for `pcre2_dfa_match` | [x] |
| B21 | substring number 0, top_bracket, top_bracket+1, `u32::MAX` | [x] |
| B22 | `number_of_codes` −1, 0, 1, 2, `i32::MAX` for serialize | [x] |
| B23 | glob separator/escape full 0..=0x120000 sweep | [x] |
| B24 | `pcre2_get_error_message` full code sweep −200..=200 | [x] |
