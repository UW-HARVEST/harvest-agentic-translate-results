# ERRORS.md — error-surface table

Derived mechanically from the C sources in `c_src/src` (grep for `return PCRE2_ERROR_*`,
`rc = PCRE2_ERROR_*`, `return NULL`, `*errorptr = ERRnn`, range checks and `switch` defaults).
The build has `SUPPORT_UNICODE` defined, `SUPPORT_JIT` **undefined** (see `c_src/src/config.h`),
`PCRE2_CODE_UNIT_WIDTH=8`.

Every row has a differential test in `translation/tests/` that constructs the exact
condition, calls the symbol in BOTH `.so`s, and asserts the identical return value.

Status: **all rows have a passing differential test** (see the "test" column;
run `./verify_all.sh`).

| # | function | trigger (exact invalid input/condition) | expected C result | test | ok |
|---|----------|------------------------------------------|-------------------|------|----|
| 1 | `pcre2_compile_8` | `errorptr == NULL` (`erroroffset` non-NULL, set to 0) | returns `NULL`, `*erroroffset = 0` | `t08::compile_null_pointer_arguments` | [x] |
| 2 | `pcre2_compile_8` | `erroroffset == NULL` | returns `NULL`, `*errorptr = ERR120` (=120) | `t08::compile_null_pointer_arguments` | [x] |
| 3 | `pcre2_compile_8` | `pattern == NULL`, `patlen != 0` | `NULL`, `*errorptr = ERR16` (16) | `t08::compile_null_pointer_arguments` | [x] |
| 4 | `pcre2_compile_8` | `pattern == NULL`, `patlen == 0` | success (empty pattern compiles) | `t08::compile_null_pointer_arguments` | [x] |
| 5 | `pcre2_compile_8` | `options` has a bit outside `PUBLIC_COMPILE_OPTIONS` (e.g. `0x10000000`) | `NULL`, `ERR17` (17) | `t08::compile_option_validation` | [x] |
| 6 | `pcre2_compile_8` | ccontext `extra_options` has bit outside `PUBLIC_COMPILE_EXTRA_OPTIONS` (e.g. `0x80000000`) | `NULL`, `ERR17` | `t08::compile_option_validation` | [x] |
| 7 | `pcre2_compile_8` | `PCRE2_LITERAL` + a non-literal-legal option (e.g. `PCRE2_DOTALL`) | `NULL`, `ERR92` (92) | `t08::compile_option_validation` | [x] |
| 8 | `pcre2_compile_8` | `PCRE2_LITERAL` + non-literal-legal extra option (`PCRE2_EXTRA_MATCH_WORD` is legal; `PCRE2_EXTRA_ALT_BSUX` is not) | `NULL`, `ERR92` | `t08::compile_option_validation` | [x] |
| 9 | `pcre2_compile_8` | `patlen > ccontext->max_pattern_length` (set via `pcre2_set_max_pattern_length_8`) | `NULL`, `ERR88` (88) | `t08::compile_length_limits` | [x] |
| 10 | `pcre2_compile_8` | compiled size > `max_pattern_compiled_length` | `NULL`, `ERR89` (89) | `t08::compile_length_limits` | [x] |
| 11 | `pcre2_compile_8` | `PCRE2_NEVER_UTF` + `(*UTF)` in pattern | `NULL`, `ERR74` (74) | `t08::compile_never_and_nest_limits` | [x] |
| 12 | `pcre2_compile_8` | `PCRE2_NEVER_UCP` + `(*UCP)` in pattern | `NULL`, `ERR75` (75) | `t08::compile_never_and_nest_limits` | [x] |
| 13 | `pcre2_compile_8` | `PCRE2_NEVER_BACKSLASH_C` + `\C` | `NULL`, `ERR83` (83) | `t08::compile_never_and_nest_limits` | [x] |
| 14 | `pcre2_compile_8` | `PCRE2_EXTRA_NEVER_CALLOUT` + `(?C1)` | `NULL`, `ERR103` (103) | `t08::compile_never_and_nest_limits` | [x] |
| 15 | `pcre2_compile_8` | invalid UTF-8 in pattern with `PCRE2_UTF` and without `PCRE2_NO_UTF_CHECK` | `NULL`, `ERR44`/`UTF8_ERRn` mapped compile error | `t08::compile_invalid_utf_patterns` | [x] |
| 16 | `pcre2_compile_8` | nesting deeper than `parens_nest_limit` (`pcre2_set_parens_nest_limit_8`) | `NULL`, `ERR19` (19) | `t08::compile_never_and_nest_limits` | [x] |
| 17 | `pcre2_compile_8` | variable lookbehind longer than `max_varlookbehind` | `NULL`, `ERR100` (100) | `t08::compile_never_and_nest_limits` | [x] |
| 18 | `pcre2_compile_8` | every distinct pattern-syntax rejection `ERR1..ERR120` (`\` at end, `(` unmatched, `)` unmatched, bad quantifier, bad class, bad `\P`, bad octal/hex, unknown verb, duplicate name, missing name terminator, recursion loop, invalid condition, etc.) — driven by a corpus of ≥120 invalid patterns, one per distinct error code reachable | `NULL`, identical `*errorptr` **and** `*erroroffset` | `t08::compile_error_code_corpus + compile_fuzz_error_offsets` | [x] |
| 19 | `pcre2_match_8` | `match_data == NULL` | `PCRE2_ERROR_NULL` (-51) | `t08::match_argument_validation` | [x] |
| 20 | `pcre2_match_8` | `code == NULL` | -51, and `match_data->rc` set | `t08::match_argument_validation` | [x] |
| 21 | `pcre2_match_8` | `subject == NULL` with `length != 0` | -51 | `t08::match_argument_validation` | [x] |
| 22 | `pcre2_match_8` | `subject == NULL`, `length == 0` | treated as empty string (no error) | `t08::match_argument_validation` | [x] |
| 23 | `pcre2_match_8` | `options` bit outside `PUBLIC_MATCH_OPTIONS` (e.g. `0x00080000`) | `PCRE2_ERROR_BADOPTION` (-34) | `t08::match_option_and_offset_validation` | [x] |
| 24 | `pcre2_match_8` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) | `t08::match_option_and_offset_validation` | [x] |
| 25 | `pcre2_match_8` | code block whose first field is not `MAGIC_NUMBER` (corrupted buffer) | `PCRE2_ERROR_BADMAGIC` (-31) | `t08::match_bad_magic_and_mode` | [x] |
| 26 | `pcre2_match_8` | code compiled for a different code-unit width (`flags & PCRE2_MODE_MASK` mismatch) | `PCRE2_ERROR_BADMODE` (-32) | `t08::match_bad_magic_and_mode` | [x] |
| 27 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT`/`HARD` together with `PCRE2_ENDANCHORED` (compile-time or match-time) | `PCRE2_ERROR_BADOPTION` (-34) | `t08::match_option_and_offset_validation` | [x] |
| 28 | `pcre2_match_8` | `mcontext->offset_limit != PCRE2_UNSET` but pattern lacks `PCRE2_USE_OFFSET_LIMIT` | `PCRE2_ERROR_BADOFFSETLIMIT` (-56) | `t08::match_option_and_offset_validation` | [x] |
| 29 | `pcre2_match_8` | invalid UTF-8 subject, `PCRE2_UTF`, no `PCRE2_NO_UTF_CHECK` | `PCRE2_ERROR_UTF8_ERR1..21` (-3..-23), identical code | `t08::match_invalid_utf_subjects` | [x] |
| 30 | `pcre2_match_8` | invalid UTF-8 subject with `start_offset > 0` pointing inside a sequence | `PCRE2_ERROR_BADUTFOFFSET` (-36) | `t08::match_invalid_utf_subjects` | [x] |
| 31 | `pcre2_match_8` | isolated 0x80 continuation byte at `start_offset == 0` with `MATCH_INVALID_UTF` | `PCRE2_ERROR_UTF8_ERR20` (-22) | `t08::match_invalid_utf_subjects` | [x] |
| 32 | `pcre2_match_8` | `match_limit` exceeded (`pcre2_set_match_limit_8(1)`) | `PCRE2_ERROR_MATCHLIMIT` (-47) | `t08::match_runtime_limits` | [x] |
| 33 | `pcre2_match_8` | depth limit exceeded (`pcre2_set_depth_limit_8(1)`) | `PCRE2_ERROR_DEPTHLIMIT` (-53) | `t08::match_runtime_limits` | [x] |
| 34 | `pcre2_match_8` | heap limit exceeded (`pcre2_set_heap_limit_8(0)` with deep pattern) | `PCRE2_ERROR_HEAPLIMIT` (-63) | `t08::match_runtime_limits` | [x] |
| 35 | `pcre2_match_8` | `\K` in an assertion moving start backwards where not allowed | `PCRE2_ERROR_BAD_BACKSLASH_K` (-70) | `t08::match_runtime_limits` | [x] |
| 36 | `pcre2_match_8` | infinite recursion detected | `PCRE2_ERROR_RECURSELOOP` (-52) | `t08::match_runtime_limits` | [x] |
| 37 | `pcre2_match_8` | no match | `PCRE2_ERROR_NOMATCH` (-1) | `t02::partial_matching + t02::randomized_compile_match` | [x] |
| 38 | `pcre2_match_8` | partial match requested and found | `PCRE2_ERROR_PARTIAL` (-2) | `t02::partial_matching + t02::randomized_compile_match` | [x] |
| 39 | `pcre2_dfa_match_8` | `match_data == NULL` | -51 | `t08::match_argument_validation` | [x] |
| 40 | `pcre2_dfa_match_8` | `code == NULL` or `subject == NULL` (length != 0) | -51 | `t08::match_argument_validation` | [x] |
| 41 | `pcre2_dfa_match_8` | `options` bit outside `PUBLIC_DFA_MATCH_OPTIONS` | -34 | `t08::match_argument_validation` | [x] |
| 42 | `pcre2_dfa_match_8` | `wscount < 20` | `PCRE2_ERROR_DFA_WSSIZE` (-53? see header: -47..) → `PCRE2_ERROR_DFA_WSSIZE` | `t08::match_argument_validation` | [x] |
| 43 | `pcre2_dfa_match_8` | `start_offset > length` | -33 | `t08::match_option_and_offset_validation` | [x] |
| 44 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_*` + `PCRE2_ENDANCHORED` | -34 | `t08::match_option_and_offset_validation` | [x] |
| 45 | `pcre2_dfa_match_8` | pattern compiled with `PCRE2_MATCH_INVALID_UTF` | `PCRE2_ERROR_DFA_UINVALID_UTF` (-68) | `t08::match_invalid_utf_subjects` | [x] |
| 46 | `pcre2_dfa_match_8` | bad magic number | -31 | `t08::match_bad_magic_and_mode` | [x] |
| 47 | `pcre2_dfa_match_8` | wrong mode | -32 | `t08::match_bad_magic_and_mode` | [x] |
| 48 | `pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` with workspace not from a prior partial match | `PCRE2_ERROR_DFA_BADRESTART` (-38) | `t08::dfa_specific_errors` | [x] |
| 49 | `pcre2_dfa_match_8` | offset limit set without `PCRE2_USE_OFFSET_LIMIT` | -56 | `t08::match_option_and_offset_validation` | [x] |
| 50 | `pcre2_dfa_match_8` | invalid UTF subject / bad UTF offset | `UTF8_ERRn` / -36 | `t08::match_invalid_utf_subjects` | [x] |
| 51 | `pcre2_dfa_match_8` | pattern containing `\C` (`OP_ANYBYTE`) | `PCRE2_ERROR_DFA_UITEM` (-40) | `t08::dfa_specific_errors` | [x] |
| 52 | `pcre2_dfa_match_8` | back reference in pattern (unsupported item) | `PCRE2_ERROR_DFA_UITEM` (-40) | `t08::dfa_specific_errors` | [x] |
| 53 | `pcre2_dfa_match_8` | conditional group with a non-`RREF_ANY` recursion condition / `(?(1)...)` | `PCRE2_ERROR_DFA_UCOND` (-39) | `t08::dfa_specific_errors` | [x] |
| 54 | `pcre2_dfa_match_8` | workspace overflow on deep pattern (`wscount` small but ≥20) | `PCRE2_ERROR_DFA_WSSIZE` | `t08::dfa_specific_errors` | [x] |
| 55 | `pcre2_dfa_match_8` | match/depth limit exceeded | -47 / -53 | `t08::match_runtime_limits` | [x] |
| 56 | `pcre2_next_match_8` | `match_data == NULL` | -51 | `t07::next_match_edge_cases` | [x] |
| 57 | `pcre2_next_match_8` | previous rc < 0 / match data not from a successful match | same rejection as C | `t07::next_match_edge_cases` | [x] |
| 58 | `pcre2_substitute_8` | `options` bit outside allowed set | `PCRE2_ERROR_BADOPTION` (-34) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 59 | `pcre2_substitute_8` | `replacement == NULL` with `rlength != 0` | -51 | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 60 | `pcre2_substitute_8` | `subject == NULL` with `length != 0` | -51 | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 61 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with `match_data == NULL` | -51 | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 62 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with match data from `pcre2_dfa_match` | `PCRE2_ERROR_DFA_UFUNC` (-41) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 63 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with match data from a *different* code | `PCRE2_ERROR_DIFFSUBSPATTERN` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 64 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a different subject pointer | `PCRE2_ERROR_DIFFSUBSSUBJECT` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 65 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with a different start offset | `PCRE2_ERROR_DIFFSUBSOFFSET` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 66 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` with different options | `PCRE2_ERROR_DIFFSUBSOPTIONS` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 67 | `pcre2_substitute_8` | `start_offset > length` | `PCRE2_ERROR_BADOFFSET` (-33) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 68 | `pcre2_substitute_8` | output buffer too small, without `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` | `PCRE2_ERROR_NOMEMORY` (-48) and `*outlengthptr` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 69 | `pcre2_substitute_8` | output buffer too small, WITH `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` | -48 and required length in `*outlengthptr` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 70 | `pcre2_substitute_8` | `$` followed by nothing / bad `${` group | `PCRE2_ERROR_BADREPLACEMENT` / `PCRE2_ERROR_REPMISSINGBRACE` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 71 | `pcre2_substitute_8` | `\` + invalid escape in replacement, `SUBSTITUTE_EXTENDED` | `PCRE2_ERROR_BADREPESCAPE` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 72 | `pcre2_substitute_8` | reference to non-existent group in replacement | `PCRE2_ERROR_NOSUBSTRING` (-49) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 73 | `pcre2_substitute_8` | reference to group not in (too-small) ovector | `PCRE2_ERROR_UNAVAILABLE` (-54) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 74 | `pcre2_substitute_8` | reference to unset group without `SUBSTITUTE_UNSET_EMPTY` | `PCRE2_ERROR_UNSET` (-55) | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 75 | `pcre2_substitute_8` | `SUBSTITUTE_EXTENDED` bad `${name:+a:b}` syntax | `PCRE2_ERROR_BADSUBSTITUTION` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 76 | `pcre2_substitute_8` | pattern that can match empty repeatedly with GLOBAL → too many replacements | `PCRE2_ERROR_TOOMANYREPLACE` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 77 | `pcre2_substitute_8` | partial match returned during substitute | `PCRE2_ERROR_PARTIALSUBS` | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 78 | `pcre2_substitute_8` | match error other than NOMATCH propagates | same negative code | `t03::substitute_error_paths + t03::substitute_option_matrix` | [x] |
| 79 | `pcre2_substring_length_bynumber_8` | `stringnumber >= ovector count` | `PCRE2_ERROR_UNAVAILABLE` (-54) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 80 | `pcre2_substring_length_bynumber_8` | `stringnumber` valid slot but unset | `PCRE2_ERROR_UNSET` (-55) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 81 | `pcre2_substring_length_bynumber_8` | `stringnumber > top capture` from pattern info | `PCRE2_ERROR_NOSUBSTRING` (-49) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 82 | `pcre2_substring_length_bynumber_8` | match data from partial match, `stringnumber > 0` | `PCRE2_ERROR_PARTIAL` (-2) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 83 | `pcre2_substring_length_bynumber_8` | match data from DFA match | `PCRE2_ERROR_DFA_UFUNC` (-41) *(via bynumber path only where checked)* | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 84 | `pcre2_substring_copy_bynumber_8` | buffer `size + 1 > *sizeptr` too small | `PCRE2_ERROR_NOMEMORY` (-48) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 85 | `pcre2_substring_copy_bynumber_8` | invalid group number | -49 / -54 / -55 as above | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 86 | `pcre2_substring_get_bynumber_8` | invalid group number | -49 / -54 / -55 | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 87 | `pcre2_substring_copy_byname_8` | name not present | `PCRE2_ERROR_NOSUBSTRING` (-49) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 88 | `pcre2_substring_copy_byname_8` | duplicate names, none set | `PCRE2_ERROR_UNSET` (-55) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 89 | `pcre2_substring_copy_byname_8` | duplicate names, none in ovector | `PCRE2_ERROR_UNAVAILABLE` (-54) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 90 | `pcre2_substring_copy_byname_8` | match data from DFA | `PCRE2_ERROR_DFA_UFUNC` (-41) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 91 | `pcre2_substring_get_byname_8` | name not present | -49 | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 92 | `pcre2_substring_length_byname_8` | name not present | -49 | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 93 | `pcre2_substring_number_from_name_8` | name not present | `PCRE2_ERROR_NOSUBSTRING` (-49) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 94 | `pcre2_substring_number_from_name_8` | name is duplicated (`PCRE2_DUPNAMES`) | `PCRE2_ERROR_NOUNIQUESUBSTRING` (-50) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 95 | `pcre2_substring_nametable_scan_8` | name not found | -49 | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 96 | `pcre2_substring_list_get_8` | allocation-size path / no memory | `PCRE2_ERROR_NOMEMORY` (-48) | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 97 | `pcre2_substring_list_get_8` | match data from DFA match (`oveccount` semantics) | same as C | `t04::substrings_by_number_and_name + t04::substring_after_partial_and_dfa + t04::number_from_name_uniqueness` | [x] |
| 98 | `pcre2_pattern_info_8` | `re == NULL` (with `what != PCRE2_INFO_SIZE`… all cases) | `PCRE2_ERROR_NULL` (-51) | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 99 | `pcre2_pattern_info_8` | bad magic number | `PCRE2_ERROR_BADMAGIC` (-31) | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 100 | `pcre2_pattern_info_8` | wrong mode bits | `PCRE2_ERROR_BADMODE` (-32) | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 101 | `pcre2_pattern_info_8` | `what` not a recognized `PCRE2_INFO_*` (e.g. 999, `UINT32_MAX`) | `PCRE2_ERROR_BADOPTION` (-34) | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 102 | `pcre2_pattern_info_8` | `PCRE2_INFO_DEPTHLIMIT` when unset | `PCRE2_ERROR_UNSET` (-55) | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 103 | `pcre2_pattern_info_8` | `PCRE2_INFO_HEAPLIMIT` when unset | -55 | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 104 | `pcre2_pattern_info_8` | `PCRE2_INFO_MATCHLIMIT` when unset | -55 | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 105 | `pcre2_pattern_info_8` | `PCRE2_INFO_FIRSTBITMAP` / `NAMETABLE` etc. with `where == NULL` (size query) | returns size, no error | `t05::config_all_options / t02 info comparison (harness Api::info)` | [x] |
| 106 | `pcre2_config_8` | `what` unrecognized (e.g. 999) | `PCRE2_ERROR_BADOPTION` (-34) | `t05::config_all_options` | [x] |
| 107 | `pcre2_config_8` | `PCRE2_CONFIG_JIT` / `JITTARGET` with JIT unsupported | `JITTARGET` → -34, `JIT` → 0 value | `t05::config_all_options` | [x] |
| 108 | `pcre2_config_8` | string configs (`UNICODE_VERSION`, `VERSION`) with `where == NULL` | returns required length | `t05::config_all_options` | [x] |
| 109 | `pcre2_get_error_message_8` | `bufflen == 0` | `PCRE2_ERROR_NOMEMORY` (-48) | `t05::error_messages` | [x] |
| 110 | `pcre2_get_error_message_8` | `errorcode` with no message (e.g. 0, 1000, -1000) | `PCRE2_ERROR_BADDATA` (-29) | `t05::error_messages` | [x] |
| 111 | `pcre2_get_error_message_8` | buffer smaller than the message | -48, buffer truncated identically | `t05::error_messages` | [x] |
| 112 | `pcre2_serialize_encode_8` | `codes == NULL` or `serialized_bytes == NULL` or `serialized_size == NULL` | `PCRE2_ERROR_NULL` (-51) | `t05::serialize_error_paths` | [x] |
| 113 | `pcre2_serialize_encode_8` | `number_of_codes <= 0` | `PCRE2_ERROR_BADDATA` (-29) | `t05::serialize_error_paths` | [x] |
| 114 | `pcre2_serialize_encode_8` | `codes[i] == NULL` | -51 | `t05::serialize_error_paths` | [x] |
| 115 | `pcre2_serialize_encode_8` | a code with bad magic | `PCRE2_ERROR_BADMAGIC` (-31) | `t05::serialize_error_paths` | [x] |
| 116 | `pcre2_serialize_encode_8` | codes with different character tables | `PCRE2_ERROR_MIXEDTABLES` (-30) | `t05::serialize_error_paths` | [x] |
| 117 | `pcre2_serialize_decode_8` | `bytes == NULL` or `codes == NULL` | -51 | `t05::serialize_error_paths` | [x] |
| 118 | `pcre2_serialize_decode_8` | `number_of_codes <= 0` | -29 | `t05::serialize_error_paths` | [x] |
| 119 | `pcre2_serialize_decode_8` | serialized `magic` wrong | -31 | `t05::serialize_error_paths` | [x] |
| 120 | `pcre2_serialize_decode_8` | serialized `version` wrong | `PCRE2_ERROR_BADMODE` (-32) | `t05::serialize_error_paths` | [x] |
| 121 | `pcre2_serialize_decode_8` | serialized `config` wrong (different width/UTF build) | -32 | `t05::serialize_error_paths` | [x] |
| 122 | `pcre2_serialize_decode_8` | `data->number_of_codes <= 0` | `PCRE2_ERROR_BADSERIALIZEDDATA` (-62) | `t05::serialize_error_paths` | [x] |
| 123 | `pcre2_serialize_decode_8` | truncated / inconsistent blocksize in the byte stream | -62 | `t05::serialize_error_paths` | [x] |
| 124 | `pcre2_serialize_get_number_of_codes_8` | `bytes == NULL` | -51 | `t05::serialize_error_paths` | [x] |
| 125 | `pcre2_serialize_get_number_of_codes_8` | bad magic | -31 | `t05::serialize_error_paths` | [x] |
| 126 | `pcre2_serialize_get_number_of_codes_8` | bad version / config | -32 | `t05::serialize_error_paths` | [x] |
| 127 | `pcre2_pattern_convert_8` | `pattern == NULL` / `buffptr == NULL` / `blength == NULL` | `PCRE2_ERROR_NULL` (-51) | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 128 | `pcre2_pattern_convert_8` | `options` bit outside allowed, or more than one conversion type | `PCRE2_ERROR_BADOPTION` (-34) | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 129 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` when Unicode unsupported | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` — n/a here (Unicode IS supported) | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 130 | `pcre2_pattern_convert_8` | POSIX BRE/ERE with unterminated `[` | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 131 | `pcre2_pattern_convert_8` | POSIX pattern ending with `\` | `PCRE2_ERROR_END_BACKSLASH` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 132 | `pcre2_pattern_convert_8` | POSIX `[[:foo:]]` unknown class / bad `[[.` `[[=` | `PCRE2_ERROR_CONVERT_SYNTAX` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 133 | `pcre2_pattern_convert_8` | glob pattern with `**` when `GLOB_NO_STARSTAR`, or separator misuse | `PCRE2_ERROR_CONVERT_SYNTAX` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 134 | `pcre2_pattern_convert_8` | glob `[` unterminated | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 135 | `pcre2_pattern_convert_8` | user-supplied `blength` too small (buffer given) | `PCRE2_ERROR_NOMEMORY` (-48) | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 136 | `pcre2_pattern_convert_8` | invalid UTF pattern with `PCRE2_CONVERT_UTF` and no `NO_UTF_CHECK` | `UTF8_ERRn` | `t06::null_and_option_validation + t06::error_trigger_corpus + t06::invalid_utf8_paths` | [x] |
| 137 | `pcre2_set_bsr_8` | `value` not `PCRE2_BSR_UNICODE`(1)/`ANYCRLF`(2) — e.g. 0, 3, `UINT32_MAX` | `PCRE2_ERROR_BADDATA` (-29) | `t05::setter_validation` | [x] |
| 138 | `pcre2_set_newline_8` | `newline` not in 1..6 — e.g. 0, 7, `UINT32_MAX` | `PCRE2_ERROR_BADDATA` (-29) | `t05::setter_validation` | [x] |
| 139 | `pcre2_set_optimize_8` | `ccontext == NULL` | `PCRE2_ERROR_NULL` (-51) | `t05::setter_validation` | [x] |
| 140 | `pcre2_set_optimize_8` | `directive` outside `NONE`/`FULL`/`[PCRE2_AUTO_POSSESS .. PCRE2_START_OPTIMIZE_OFF]` — e.g. 2, 63, 70, `UINT32_MAX` | `PCRE2_ERROR_BADOPTION` (-34) | `t05::setter_validation` | [x] |
| 141 | `pcre2_set_glob_separator_8` | separator not `/`, `\`, `.` — e.g. 0, `'x'`, 256 | `PCRE2_ERROR_BADDATA` (-29) | `t05::setter_validation` | [x] |
| 142 | `pcre2_set_glob_escape_8` | `escape > 255`, or non-punct non-zero (e.g. `'a'`, 0x100) | `PCRE2_ERROR_BADDATA` (-29) | `t05::setter_validation` | [x] |
| 143 | `pcre2_set_glob_escape_8` | `escape == 0` (means "no escape") | 0 (success) | `t05::setter_validation` | [x] |
| 144 | `pcre2_jit_compile_8` | `options == PCRE2_JIT_TEST_ALLOC` (JIT unsupported build) | `PCRE2_ERROR_JIT_UNSUPPORTED` (-45) | `t05::jit_stubs` | [x] |
| 145 | `pcre2_jit_compile_8` | `PCRE2_JIT_TEST_ALLOC` OR'd with anything else | `PCRE2_ERROR_JIT_BADOPTION` (-44) | `t05::jit_stubs` | [x] |
| 146 | `pcre2_jit_compile_8` | `code == NULL` (no TEST_ALLOC) | `PCRE2_ERROR_NULL` (-51) | `t05::jit_stubs` | [x] |
| 147 | `pcre2_jit_compile_8` | `options` bit outside `PUBLIC_JIT_COMPILE_OPTIONS` | `PCRE2_ERROR_JIT_BADOPTION` (-44) | `t05::jit_stubs` | [x] |
| 148 | `pcre2_jit_compile_8` | valid options, JIT unsupported | `PCRE2_ERROR_JIT_BADOPTION` (-44) | `t05::jit_stubs` | [x] |
| 149 | `pcre2_jit_compile_8` | `PCRE2_JIT_INVALID_UTF` on a code without `MATCH_INVALID_UTF` | sets `re->overall_options |= PCRE2_MATCH_INVALID_UTF`, then returns -44 | `t05::jit_stubs` | [x] |
| 150 | `pcre2_jit_match_8` | any call (JIT unsupported) | `PCRE2_ERROR_JIT_BADOPTION` (-44) | `t05::jit_stubs` | [x] |
| 151 | `pcre2_jit_stack_create_8` | any call (JIT unsupported) | `NULL` | `t05::jit_stubs` | [x] |
| 152 | `pcre2_jit_get_size` (`_pcre2_jit_get_size_8`) | any call | 0 | `t05::jit_stubs` | [x] |
| 153 | `_pcre2_jit_get_target_8` | any call | pointer to `"JIT is not supported"` | `t05::jit_stubs` | [x] |
| 154 | `pcre2_match_data_create_8` | `ovecsize == 0` | bumped to 1 internally (no error) | `t05::match_data_sizes_and_accessors` | [x] |
| 155 | `pcre2_match_data_create_from_pattern_8` | `code == NULL` | `NULL` | `t05::match_data_sizes_and_accessors` | [x] |
| 156 | `pcre2_code_copy_8` | `code == NULL` | `NULL` | `t05::code_copy_variants` | [x] |
| 157 | `pcre2_code_copy_with_tables_8` | `code == NULL` | `NULL` | `t05::code_copy_variants` | [x] |
| 158 | `pcre2_general_context_create_8` | `private_malloc`/`private_free` NULL (uses defaults) | non-NULL | `t05::custom_allocator_and_context_copies / t05::maketables_identical` | [x] |
| 159 | `pcre2_maketables_8` | `gcontext == NULL` (uses malloc) | non-NULL | `t05::custom_allocator_and_context_copies / t05::maketables_identical` | [x] |
| 160 | `pcre2_callout_enumerate_8` | `code == NULL` | `PCRE2_ERROR_NULL` (-51) | `t07::callout_enumerate_null_code + t07::callout_enumerate_nonzero_return` | [x] |
| 161 | `pcre2_callout_enumerate_8` | callback returns non-zero | that value is returned | `t07::callout_enumerate_null_code + t07::callout_enumerate_nonzero_return` | [x] |
| 162 | `pcre2_get_startchar_8` / `get_mark_8` / `get_ovector_*` | match data from a failed match | identical values | `t02::* (harness read_match) + t05::match_data_sizes_and_accessors` | [x] |
| 163 | `_pcre2_valid_utf_8` | invalid UTF-8 at every distinct `UTF8_ERRn` (1..21) | identical error code and `erroroffset` | `t01::valid_utf` | [x] |
| 164 | `_pcre2_ord2utf_8` | code point 0, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff | identical byte sequence and length | `t01::ord2utf` | [x] |
| 165 | `_pcre2_check_escape_8` | every invalid escape (`ERR1`,`ERR3`,`ERR37`,`ERR51`,`ERR55`,`ERR57`,`ERR64`,`ERR67`,`ERR73`,`ERR77`,`ERR78`,`ERR93`,`ERR98`,`ERR102`,`ERR119`) | identical `*errorcodeptr` and consumed length | `t08::every_escape_sequence` | [x] |
| 166 | `_pcre2_ckd_smul_8` | multiplication that overflows `PCRE2_SIZE` | returns TRUE (overflow) identically | `t01::ckd_smul` | [x] |
| 167 | `_pcre2_strcmp_8` / `strncmp` / `strcmp_c8` / `strncmp_c8` / `strlen` / `strcpy_c8` | empty strings, unequal lengths, high bytes | identical results | `t01::string_utils` | [x] |
| 168 | `_pcre2_is_newline_8` / `_pcre2_was_newline_8` | every newline convention × CR/LF/CRLF/NEL/LS/PS/other, at buffer boundaries | identical BOOL + length | `t01::newline_detection` | [x] |
| 169 | `_pcre2_find_bracket_8` | bracket number not present | `NULL` | `t09::find_bracket` | [x] |
| 170 | `_pcre2_script_run_8` | non-script-run sequences | `FALSE` | `t01::extuni_and_script_run` | [x] |
| 171 | `_pcre2_xclass_8` / `_pcre2_eclass_8` | chars in/out of class, negated class | identical BOOL | `t09::wide_and_extended_classes` | [x] |
| 172 | `_pcre2_study_8` | pattern with no fixed start | identical return code | `t09::study_recomputes_identically` | [x] |
| 173 | `_pcre2_auto_possessify_8` | patterns where possessification is/isn't possible | identical return code + rewritten code bytes | `t09::compiled_byte_code_identical` | [x] |
| 174 | `_pcre2_memctl_malloc_8` | size 0 / huge size (allocation failure) | `NULL` on failure, identical layout otherwise | `t01::memctl_malloc_null_on_failure` | [x] |
| 175 | `pcre2_get_match_data_size_8` / `heapframes_size_8` | fresh vs used match data | identical sizes | `t05::match_data_sizes_and_accessors` | [x] |
| 176 | out-of-range **enum-like** ints across FFI | `pcre2_config_8(-1,…)`, `pcre2_pattern_info_8(re, 0xFFFFFFFF, …)`, `pcre2_set_bsr_8(0xFFFFFFFF)`, `pcre2_set_newline_8(0xFFFFFFFF)`, `pcre2_set_optimize_8(0xFFFFFFFF)`, `pcre2_get_error_message_8(INT_MIN)`, `pcre2_pattern_convert_8` with all-bits options | identical rejection | `t05::config_all_options + t05::setter_validation + t02 info bad-what` | [x] |
