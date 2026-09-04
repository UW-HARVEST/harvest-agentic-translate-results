# CONFIGS.md — configuration-surface table (valid inputs)

Axes derived from `c_src/include/pcre2.h` (public option bits, `PCRE2_INFO_*`,
`PCRE2_CONFIG_*`, newline/BSR/optimize constants) and from the `if`/`switch`
branches those flags drive in `c_src/src/*.c`. Build: `PCRE2_CODE_UNIT_WIDTH=8`,
`SUPPORT_UNICODE` on, `SUPPORT_JIT` off.

Every row is exercised through BOTH `.so`s with **many seeded-random inputs**
(patterns and/or subjects) and the full observable output compared byte-for-byte:
return code, ovector contents, ovector count, startchar, mark, and any output
buffer.

Status: **every row passes across its randomized inputs** (run `./verify_all.sh`).

| # | entry point(s) | configuration (options set + input shape) | test | ok |
|---|----------------|--------------------------------------------|------|----|
| 1 | `pcre2_compile_8` + `pcre2_match_8` | no options; random ASCII patterns × random ASCII subjects | `t02::compile_option_matrix_interpreter` | [x] |
| 2 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_CASELESS`; mixed-case random subjects | `t02::compile_option_matrix_interpreter` | [x] |
| 3 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_MULTILINE`; subjects with embedded `\n`, `\r`, `\r\n` | `t02::compile_option_matrix_interpreter` | [x] |
| 4 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_DOTALL`; subjects with newlines | `t02::compile_option_matrix_interpreter` | [x] |
| 5 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTENDED`; patterns with whitespace/comments | `t02::compile_option_matrix_interpreter` | [x] |
| 6 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTENDED_MORE` | `t02::compile_option_matrix_interpreter` | [x] |
| 7 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UNGREEDY` | `t02::compile_option_matrix_interpreter` | [x] |
| 8 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ANCHORED` (compile) and `PCRE2_ANCHORED` (match) | `t02::compile_option_matrix_interpreter` | [x] |
| 9 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ENDANCHORED` (compile / match) | `t02::compile_option_matrix_interpreter` | [x] |
| 10 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_DOLLAR_ENDONLY` | `t02::compile_option_matrix_interpreter` | [x] |
| 11 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_FIRSTLINE`; multi-line subjects | `t02::compile_option_matrix_interpreter` | [x] |
| 12 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_NO_AUTO_CAPTURE`; patterns with `(...)` | `t02::compile_option_matrix_interpreter` | [x] |
| 13 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_DUPNAMES`; duplicate `(?<n>)` names | `t02::compile_option_matrix_interpreter` | [x] |
| 14 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_MATCH_UNSET_BACKREF` | `t02::compile_option_matrix_interpreter` | [x] |
| 15 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ALLOW_EMPTY_CLASS`; `[]` in pattern | `t02::compile_option_matrix_interpreter` | [x] |
| 16 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ALT_BSUX` and `PCRE2_EXTRA_ALT_BSUX`; `\u`/`\x` escapes | `t02::compile_option_matrix_interpreter` | [x] |
| 17 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ALT_CIRCUMFLEX` + `MULTILINE` | `t02::compile_option_matrix_interpreter` | [x] |
| 18 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ALT_VERBNAMES`; `(*MARK:...)` with escapes | `t02::compile_option_matrix_interpreter` | [x] |
| 19 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_ALT_EXTENDED_CLASS`; `[[a-z]&&[b-d]]` set operations | `t02::compile_option_matrix_interpreter` | [x] |
| 20 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_LITERAL`; patterns containing regex metacharacters | `t02::compile_option_matrix_interpreter` | [x] |
| 21 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UTF`; valid random UTF-8 patterns × valid UTF-8 subjects | `t02::compile_option_matrix_unicode` | [x] |
| 22 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UTF` + `PCRE2_UCP`; `\w \d \s \b` over non-ASCII | `t02::compile_option_matrix_unicode` | [x] |
| 23 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UTF` + `PCRE2_CASELESS`; non-ASCII case folding | `t02::compile_option_matrix_unicode` | [x] |
| 24 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UCP` without `PCRE2_UTF` | `t02::compile_option_matrix_unicode` | [x] |
| 25 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_UTF` + `PCRE2_MATCH_INVALID_UTF`; deliberately invalid UTF-8 subjects | `t02::compile_option_matrix_unicode` | [x] |
| 26 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_NO_UTF_CHECK` at compile and at match with invalid UTF | `t02::compile_option_matrix_unicode` | [x] |
| 27 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTRA_CASELESS_RESTRICT` (+`CASELESS`, +`UTF`) | `t02::compile_option_matrix_unicode` | [x] |
| 28 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTRA_TURKISH_CASING` (+`CASELESS`, +`UTF`); `i`/`I`/`İ`/`ı` | `t02::compile_option_matrix_unicode` | [x] |
| 29 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTRA_ASCII_BSD` / `BSS` / `BSW` / `POSIX` / `DIGIT` each with `UCP` | `t02::compile_option_matrix_unicode` | [x] |
| 30 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTRA_MATCH_WORD` | `t02::compile_option_matrix_unicode` | [x] |
| 31 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_EXTRA_MATCH_LINE` | `t02::compile_option_matrix_unicode` | [x] |
| 32 | `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` + `\x{d800}` (UTF and non-UTF) | `t02::compile_option_matrix_unicode` | [x] |
| 33 | `pcre2_compile_8` | `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL`; unknown escapes | `t02::compile_option_matrix_unicode` | [x] |
| 34 | `pcre2_compile_8` | `PCRE2_EXTRA_ESCAPED_CR_IS_LF`; `\r` escape | `t02::compile_option_matrix_unicode` | [x] |
| 35 | `pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK`; `\K` in lookaround | `t02::compile_option_matrix_unicode` | [x] |
| 36 | `pcre2_compile_8` | `PCRE2_EXTRA_PYTHON_OCTAL`; `\0`,`\07`,`\o{}` forms | `t02::compile_option_matrix_unicode` | [x] |
| 37 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_AUTO_CALLOUT` + `pcre2_set_callout_8` (callout blocks compared field-by-field) | `t07::auto_callout_full_field_capture + t07::explicit_callouts_and_delimiters + t07::callout_enumerate_full` | [x] |
| 38 | `pcre2_compile_8` + `pcre2_match_8` | explicit `(?C1)`/`(?C{txt})` callouts + callback | `t07::auto_callout_full_field_capture + t07::explicit_callouts_and_delimiters + t07::callout_enumerate_full` | [x] |
| 39 | `pcre2_compile_8` + `pcre2_callout_enumerate_8` | pattern with several callouts; enumerate all fields | `t07::auto_callout_full_field_capture + t07::explicit_callouts_and_delimiters + t07::callout_enumerate_full` | [x] |
| 40 | `pcre2_set_newline_8` × `pcre2_compile_8`/`match` | newline = CR, LF, CRLF, ANY, ANYCRLF, NUL (6 values) × subjects containing all newline forms, `$`/`^`/`.`/`\R` | `t02::newline_conventions` | [x] |
| 41 | `pcre2_set_bsr_8` × compile/match | BSR = UNICODE, ANYCRLF × `\R` patterns × all newline sequences | `t02::newline_conventions` | [x] |
| 42 | in-pattern startup directives | `(*UTF)`, `(*UCP)`, `(*CR)`, `(*LF)`, `(*CRLF)`, `(*ANY)`, `(*ANYCRLF)`, `(*NUL)`, `(*BSR_ANYCRLF)`, `(*BSR_UNICODE)`, `(*LIMIT_MATCH=n)`, `(*LIMIT_DEPTH=n)`, `(*LIMIT_HEAP=n)`, `(*NOTEMPTY)`, `(*NOTEMPTY_ATSTART)`, `(*NO_AUTO_POSSESS)`, `(*NO_DOTSTAR_ANCHOR)`, `(*NO_START_OPT)`, `(*NO_JIT)` | `t10::in_pattern_directives` | [x] |
| 43 | `pcre2_set_optimize_8` × compile/match | `PCRE2_OPTIMIZATION_NONE`, `FULL`, `AUTO_POSSESS(_OFF)`, `DOTSTAR_ANCHOR(_OFF)`, `START_OPTIMIZE(_OFF)` × random patterns | `t02::optimize_and_tables + t02::compile_option_matrix_interpreter` | [x] |
| 44 | `pcre2_compile_8` + `pcre2_match_8` | `PCRE2_NO_AUTO_POSSESS`, `PCRE2_NO_DOTSTAR_ANCHOR`, `PCRE2_NO_START_OPTIMIZE` (option-bit forms) | `t02::optimize_and_tables + t02::compile_option_matrix_interpreter` | [x] |
| 45 | `pcre2_set_max_varlookbehind_8` | limit 0, 1, 255, default; variable-length lookbehinds | `t02::limits_and_shapes + t08::compile_length_limits` | [x] |
| 46 | `pcre2_set_parens_nest_limit_8` | limit 0, 1, 10, default; nested groups at/below/above limit | `t02::limits_and_shapes + t08::compile_length_limits` | [x] |
| 47 | `pcre2_set_max_pattern_length_8` | length exactly equal to pattern, one less, huge | `t02::limits_and_shapes + t08::compile_length_limits` | [x] |
| 48 | `pcre2_set_max_pattern_compiled_length_8` | value at/below the compiled size | `t02::limits_and_shapes + t08::compile_length_limits` | [x] |
| 49 | `pcre2_set_character_tables_8` + `pcre2_maketables_8` | default tables vs `pcre2_maketables_8`-generated tables; `CASELESS`, `\w`, POSIX classes | `t02::optimize_and_tables + t05::maketables_identical` | [x] |
| 50 | `pcre2_match_8` | match options `NOTBOL`, `NOTEOL`, `NOTEMPTY`, `NOTEMPTY_ATSTART` (each and combined) | `t02::match_options_offsets_ovecsizes` | [x] |
| 51 | `pcre2_match_8` | `PCRE2_PARTIAL_SOFT` and `PCRE2_PARTIAL_HARD` × truncated subjects | `t02::partial_matching` | [x] |
| 52 | `pcre2_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` (then read ovector/substrings) | `t02::match_options_offsets_ovecsizes` | [x] |
| 53 | `pcre2_match_8` | `PCRE2_DISABLE_RECURSELOOP_CHECK` on recursive patterns | `t02::match_options_offsets_ovecsizes` | [x] |
| 54 | `pcre2_match_8` | `start_offset` = 0, 1, mid-string, `length` (empty tail); `length` = real and `PCRE2_ZERO_TERMINATED` | `t02::match_options_offsets_ovecsizes + t02::offset_limit + t02::limits_and_shapes` | [x] |
| 55 | `pcre2_match_8` + `pcre2_set_offset_limit_8` | `USE_OFFSET_LIMIT` compiled; limit 0, mid, `PCRE2_UNSET` | `t02::match_options_offsets_ovecsizes + t02::offset_limit + t02::limits_and_shapes` | [x] |
| 56 | `pcre2_match_8` + `pcre2_match_data_create_8` | ovecsize 0(→1), 1, 2, exact capture count, larger than needed | `t02::match_options_offsets_ovecsizes + t02::offset_limit + t02::limits_and_shapes` | [x] |
| 57 | `pcre2_match_data_create_from_pattern_8` | patterns with 0, 1, many captures | `t02::match_options_offsets_ovecsizes + t02::offset_limit + t02::limits_and_shapes` | [x] |
| 58 | `pcre2_match_8` + `pcre2_set_match_limit_8`/`depth`/`heap` | limits at boundary values (0, 1, 10, `UINT32_MAX`) | `t02::match_options_offsets_ovecsizes + t02::offset_limit + t02::limits_and_shapes` | [x] |
| 59 | `pcre2_dfa_match_8` | no options; random patterns × subjects; workspace 20, 100, 1000 | `t02::dfa_option_matrix + t02::dfa_match_options + t02::partial_matching + t08::dfa_specific_errors` | [x] |
| 60 | `pcre2_dfa_match_8` | `PCRE2_DFA_SHORTEST` | `t02::dfa_option_matrix + t02::dfa_match_options + t02::partial_matching + t08::dfa_specific_errors` | [x] |
| 61 | `pcre2_dfa_match_8` | `PCRE2_PARTIAL_SOFT`/`HARD` then `PCRE2_DFA_RESTART` continuation | `t02::dfa_option_matrix + t02::dfa_match_options + t02::partial_matching + t08::dfa_specific_errors` | [x] |
| 62 | `pcre2_dfa_match_8` | `NOTBOL`/`NOTEOL`/`NOTEMPTY`/`NOTEMPTY_ATSTART`/`ANCHORED`/`ENDANCHORED` | `t02::dfa_option_matrix + t02::dfa_match_options + t02::partial_matching + t08::dfa_specific_errors` | [x] |
| 63 | `pcre2_dfa_match_8` | `PCRE2_UTF` subjects, valid and (with `NO_UTF_CHECK`) invalid | `t02::dfa_option_matrix + t02::dfa_match_options + t02::partial_matching + t08::dfa_specific_errors` | [x] |
| 64 | `pcre2_next_match_8` | iterate all matches for random patterns/subjects (empty-match advance logic) | `t07::next_match_corpus` | [x] |
| 65 | `pcre2_substitute_8` | no options; random patterns/subjects/replacements | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 66 | `pcre2_substitute_8` | `SUBSTITUTE_GLOBAL` | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 67 | `pcre2_substitute_8` | `SUBSTITUTE_EXTENDED` (`\U \L \u \l \E`, `${n:-def}`, `${n:+a:b}`) | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 68 | `pcre2_substitute_8` | `SUBSTITUTE_LITERAL` | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 69 | `pcre2_substitute_8` | `SUBSTITUTE_UNSET_EMPTY`, `SUBSTITUTE_UNKNOWN_UNSET` (each and both) | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 70 | `pcre2_substitute_8` | `SUBSTITUTE_OVERFLOW_LENGTH` with too-small buffer, then exact-size retry | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 71 | `pcre2_substitute_8` | `SUBSTITUTE_REPLACEMENT_ONLY` (± GLOBAL) | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 72 | `pcre2_substitute_8` | `SUBSTITUTE_MATCHED` with a pre-run `pcre2_match_8` | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 73 | `pcre2_substitute_8` + `pcre2_set_substitute_callout_8` | callout invoked per substitution; all block fields compared | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 74 | `pcre2_substitute_8` + `pcre2_set_substitute_case_callout_8` | case callout used for `\U`/`\L`/`\u`/`\l` | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 75 | `pcre2_substitute_8` | replacement `PCRE2_ZERO_TERMINATED` vs explicit length; empty replacement | `t03::substitute_option_matrix + t03::substitute_buffer_sizes + t03::substitute_matched_and_offsets + t03::substitute_callouts + t03::substitute_randomized` | [x] |
| 76 | `pcre2_pattern_info_8` | every valid `PCRE2_INFO_*` (0..27) on patterns with/without captures, names, UTF, JIT, anchoring, first/req code unit, bitmap, framesize, extra options, bsr, newline, matchempty, hasbackslashc, hascrorlf, maxlookbehind, minlength, nametable | `harness Api::info (used by every t02/t10 comparison)` | [x] |
| 77 | `pcre2_config_8` | every valid `PCRE2_CONFIG_*` (0..15) incl. string and int forms, `where == NULL` size queries | `t05::config_all_options` | [x] |
| 78 | `pcre2_substring_*` | number/name access on patterns with 0/1/many captures, named + duplicate-named groups, set/unset groups, ovector exactly/under sized | `t04::substrings_by_number_and_name + t04::substrings_randomized` | [x] |
| 79 | `pcre2_substring_list_get_8` / `free` | matches with 0..N captures incl. unset middle groups | `t04::substrings_by_number_and_name + t04::substrings_randomized` | [x] |
| 80 | `pcre2_substring_nametable_scan_8` | existing single name, duplicated name, absent name, whole-table scan (`name == NULL`) | `t04::substrings_by_number_and_name + t04::substrings_randomized` | [x] |
| 81 | `pcre2_serialize_encode_8` → `decode_8` → `match` | 1 code, many codes, codes with custom tables; round-trip then match compared | `t05::serialize_encode_decode` | [x] |
| 82 | `pcre2_serialize_get_number_of_codes_8` | streams produced from 1..N codes | `t05::serialize_encode_decode` | [x] |
| 83 | `pcre2_code_copy_8` / `pcre2_code_copy_with_tables_8` | copy then match; copy of pattern with names/tables; `pcre2_pattern_info` on the copy | `t05::code_copy_variants` | [x] |
| 84 | `pcre2_pattern_convert_8` | `POSIX_BASIC`, `POSIX_EXTENDED`, `GLOB`, `GLOB_NO_WILD_SEPARATOR`, `GLOB_NO_STARSTAR`, each ± `CONVERT_UTF`, ± `NO_UTF_CHECK`, with caller buffer and with library-allocated buffer, glob separator `/ \ .` × glob escape `0 \ !` | `t06::types_x_utf_corpus + t06::glob_separator_escape_matrix + t06::seeded_random_corpus` | [x] |
| 85 | `pcre2_get_error_message_8` | all error codes -70..200 into buffers of size 1, 8, 64, 256 | `t05::error_messages` | [x] |
| 86 | `pcre2_get_startchar_8`, `get_mark_8`, `get_ovector_pointer_8`, `get_ovector_count_8` | after successful, failed, and partial matches; patterns with `(*MARK)` | `t05::match_data_sizes_and_accessors` | [x] |
| 87 | `pcre2_get_match_data_size_8`, `get_match_data_heapframes_size_8` | ovecsize 1..64, before and after a match | `t05::match_data_sizes_and_accessors` | [x] |
| 88 | `pcre2_general_context_create_8` + custom malloc/free | all creators/copiers/freers driven through a custom allocator | `t05::custom_allocator_and_context_copies` | [x] |
| 89 | `*_context_copy_8` (general/compile/match/convert) | copy a context with every setter applied, then use the copy | `t05::custom_allocator_and_context_copies` | [x] |
| 90 | `pcre2_maketables_8` / `maketables_free_8` | tables built with default and custom `gcontext`; byte-for-byte table compare | `t05::maketables_identical` | [x] |
| 91 | `_pcre2_valid_utf_8` | random valid UTF-8, all 21 invalid classes, zero length, truncated tails | `t01::valid_utf` | [x] |
| 92 | `_pcre2_ord2utf_8` | all boundary code points and random code points 0..0x10FFFF | `t01::ord2utf` | [x] |
| 93 | `_pcre2_strlen_8`, `_pcre2_strcmp_8`, `_pcre2_strncmp_8`, `_pcre2_strcmp_c8_8`, `_pcre2_strncmp_c8_8`, `_pcre2_strcpy_c8_8` | empty, equal, prefix, differing, high-bit bytes, n = 0/1/len/len+1 | `t01::string_utils` | [x] |
| 94 | `_pcre2_is_newline_8`, `_pcre2_was_newline_8` | all 6 newline types × {CR, LF, CRLF, NEL, VT, FF, LS, PS, other} × utf on/off × at start/end of buffer | `t01::newline_detection` | [x] |
| 95 | `_pcre2_ckd_smul_8` | random operands plus overflow boundaries (`SIZE_MAX`, `SIZE_MAX/2`, 0, 1, negatives) | `t01::ckd_smul` | [x] |
| 96 | `_pcre2_extuni_8` | extended grapheme clusters: base+combining, regional indicators, Hangul jamo, emoji ZWJ | `t01::extuni_and_script_run` | [x] |
| 97 | `_pcre2_script_run_8` | single-script runs, mixed-script runs, Han/Hiragana/Katakana combos, digits from different scripts | `t01::extuni_and_script_run` | [x] |
| 98 | `_pcre2_xclass_8` / `_pcre2_eclass_8` | compiled `[...]` and extended-class opcodes extracted from real patterns, probed over many code points | `t09::wide_and_extended_classes` | [x] |
| 99 | `_pcre2_find_bracket_8` | bracket 0..N present/absent, in UTF and non-UTF code | `t09::find_bracket` | [x] |
| 100 | `_pcre2_study_8` | patterns with fixed first cu, required cu, start bitmap, minimum length, anchored, `.*`-anchored | `t09::study_recomputes_identically` | [x] |
| 101 | `_pcre2_auto_possessify_8` | patterns where auto-possessification applies and where it does not; compare rewritten byte code | `t09::compiled_byte_code_identical` | [x] |
| 102 | `_pcre2_update_classbits_8` | class bitmaps from patterns with ASCII + Unicode property members | `t09::compiled_byte_code_identical + t09::wide_and_extended_classes` | [x] |
| 103 | `_pcre2_check_escape_8` | every escape letter a–z, A–Z, digits, `\x`, `\o`, `\N{U+…}`, `\g`, `\k`, `\Q…\E`, in-class and out-of-class, with each `xoptions` combination that affects it | `t08::every_escape_sequence` | [x] |
| 104 | `_pcre2_compile_*` name-table helpers (`add_name_to_table8`, `find_dupname_details8`, `find_named_group8`, `get_hash_from_name8`) | via patterns with 0/1/many/duplicate names, long names, names differing only in tail | `t09::name_table_helpers` | [x] |
| 105 | `_pcre2_compile_class_nested_8` / `not_nested_8` | nested and non-nested class forms under `ALT_EXTENDED_CLASS` and default | `t09::compiled_byte_code_identical (ALT_EXTENDED_CLASS rows)` | [x] |
| 106 | `_pcre2_compile_parse_recurse_args` / `parse_scan_substr_args` | `(?&name)`, `(?R)`, `(?1)`, `(?+1)`, `(?-1)`, `\g{…}` shapes | `t09::compiled_byte_code_identical + t02::compile_option_matrix_interpreter` | [x] |
| 107 | exported data tables | `_pcre2_OP_lengths_8`, `_pcre2_utf8_table1..4`, `_pcre2_utt_8`/`utt_names_8`/`utt_size_8`, `_pcre2_ucd_*`, `_pcre2_ucp_gbtable_8`, `_pcre2_ucp_gentype_8`, `_pcre2_hspace_list_8`, `_pcre2_vspace_list_8`, `_pcre2_posix_class_maps8`, `_pcre2_callout_*_delims_8`, `_pcre2_unicode_version_8`, `_pcre2_default_tables_8`, `_pcre2_default_*_context_8` | compared byte-for-byte between the two `.so`s | `t01::data_tables_identical` | [x] |
| 108 | `pcre2_jit_compile_8`, `jit_match_8`, `jit_stack_*`, `jit_free_unused_memory_8`, `_pcre2_jit_free_8`, `_pcre2_jit_free_rodata_8`, `_pcre2_jit_get_size_8`, `_pcre2_jit_get_target_8` | JIT-unsupported build: all documented no-JIT behaviours | `t05::jit_stubs` | [x] |
| 109 | end-to-end pipeline | compile → study(implicit) → match → substring extraction → substitute → serialize → decode → match again, over a seeded random corpus, all outputs compared | `t10::end_to_end_pipeline` | [x] |
| 110 | `pcre2_compile_8` + `pcre2_match_8` | recursion/subroutine patterns `(?R)`, `(?1)`, `(?&n)`, atomic groups, possessive quantifiers, backtracking verbs `(*PRUNE) (*SKIP) (*THEN) (*COMMIT) (*ACCEPT) (*FAIL) (*MARK)` | `t02::compile_option_matrix_interpreter + t02::randomized_compile_match + t02::zero_terminated_and_byte_shapes` | [x] |
| 111 | `pcre2_compile_8` + `pcre2_match_8` | lookarounds: `(?=) (?!) (?<=) (?<!) (?*) (?<*)` non-atomic, variable-length lookbehind | `t02::compile_option_matrix_interpreter + t02::randomized_compile_match + t02::zero_terminated_and_byte_shapes` | [x] |
| 112 | `pcre2_compile_8` + `pcre2_match_8` | quantifier shapes `{0,} {1,} {n} {n,m} {,m} ?` `+` `*` with greedy/lazy/possessive, on chars, classes, groups, and `\X`/`\R`/`\p{}` | `t02::compile_option_matrix_interpreter + t02::randomized_compile_match + t02::zero_terminated_and_byte_shapes` | [x] |
| 113 | `pcre2_compile_8` + `pcre2_match_8` | input shapes: empty subject, 1 byte, exactly at bumpalong boundaries, 4 KiB subject, subject with NUL bytes, subject with all 256 byte values | `t02::compile_option_matrix_interpreter + t02::randomized_compile_match + t02::zero_terminated_and_byte_shapes` | [x] |
