# Configuration surface

Build-time configuration: CMake fixes `PCRE2_CODE_UNIT_WIDTH=8` and `SUPPORT_UNICODE`; JIT is not enabled. `Cargo.toml` has no features, so the only Rust feature combination is the empty set (`--no-default-features`).

Rows are derived from every public dynamic entry point, public option/selector define, and C-special-cased input shape.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|:---:|
| 1 | `pcre2_callout_enumerate_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 2 | `pcre2_code_copy_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 3 | `pcre2_code_copy_with_tables_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 4 | `pcre2_code_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 5 | `pcre2_compile_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 6 | `pcre2_compile_context_copy_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 7 | `pcre2_compile_context_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 8 | `pcre2_compile_context_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 9 | `pcre2_config_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 10 | `pcre2_convert_context_copy_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 11 | `pcre2_convert_context_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 12 | `pcre2_convert_context_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 13 | `pcre2_converted_pattern_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 14 | `pcre2_dfa_match_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 15 | `pcre2_general_context_copy_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 16 | `pcre2_general_context_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 17 | `pcre2_general_context_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 18 | `pcre2_get_error_message_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 19 | `pcre2_get_mark_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 20 | `pcre2_get_match_data_heapframes_size_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 21 | `pcre2_get_match_data_size_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 22 | `pcre2_get_ovector_count_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 23 | `pcre2_get_ovector_pointer_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 24 | `pcre2_get_startchar_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 25 | `pcre2_jit_compile_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 26 | `pcre2_jit_free_unused_memory_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 27 | `pcre2_jit_match_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 28 | `pcre2_jit_stack_assign_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 29 | `pcre2_jit_stack_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 30 | `pcre2_jit_stack_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 31 | `pcre2_maketables_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 32 | `pcre2_maketables_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 33 | `pcre2_match_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 34 | `pcre2_match_context_copy_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 35 | `pcre2_match_context_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 36 | `pcre2_match_context_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 37 | `pcre2_match_data_create_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 38 | `pcre2_match_data_create_from_pattern_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 39 | `pcre2_match_data_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 40 | `pcre2_next_match_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 41 | `pcre2_pattern_convert_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 42 | `pcre2_pattern_info_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 43 | `pcre2_serialize_decode_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 44 | `pcre2_serialize_encode_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 45 | `pcre2_serialize_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 46 | `pcre2_serialize_get_number_of_codes_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 47 | `pcre2_set_bsr_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 48 | `pcre2_set_callout_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 49 | `pcre2_set_character_tables_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 50 | `pcre2_set_compile_extra_options_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 51 | `pcre2_set_compile_recursion_guard_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 52 | `pcre2_set_depth_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 53 | `pcre2_set_glob_escape_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 54 | `pcre2_set_glob_separator_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 55 | `pcre2_set_heap_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 56 | `pcre2_set_match_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 57 | `pcre2_set_max_pattern_compiled_length_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 58 | `pcre2_set_max_pattern_length_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 59 | `pcre2_set_max_varlookbehind_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 60 | `pcre2_set_newline_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 61 | `pcre2_set_offset_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 62 | `pcre2_set_optimize_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 63 | `pcre2_set_parens_nest_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 64 | `pcre2_set_recursion_limit_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 65 | `pcre2_set_recursion_memory_management_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 66 | `pcre2_set_substitute_callout_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 67 | `pcre2_set_substitute_case_callout_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 68 | `pcre2_substitute_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 69 | `pcre2_substring_copy_byname_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 70 | `pcre2_substring_copy_bynumber_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 71 | `pcre2_substring_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 72 | `pcre2_substring_get_byname_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 73 | `pcre2_substring_get_bynumber_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 74 | `pcre2_substring_length_byname_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 75 | `pcre2_substring_length_bynumber_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 76 | `pcre2_substring_list_free_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 77 | `pcre2_substring_list_get_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 78 | `pcre2_substring_nametable_scan_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 79 | `pcre2_substring_number_from_name_8` | `default valid invocation; allocated objects use default contexts` | [x] |
| 80 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ANCHORED=0x80000000u; valid ASCII pattern and matching subject [pcre2.h:102]` | [x] |
| 81 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NO_UTF_CHECK=0x40000000u; valid ASCII pattern and matching subject [pcre2.h:103]` | [x] |
| 82 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ENDANCHORED=0x20000000u; valid ASCII pattern and matching subject [pcre2.h:104]` | [x] |
| 83 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ALLOW_EMPTY_CLASS=0x00000001u /* C */; valid ASCII pattern and matching subject [pcre2.h:116]` | [x] |
| 84 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ALT_BSUX=0x00000002u /* C */; valid ASCII pattern and matching subject [pcre2.h:117]` | [x] |
| 85 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_AUTO_CALLOUT=0x00000004u /* C */; valid ASCII pattern and matching subject [pcre2.h:118]` | [x] |
| 86 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_CASELESS=0x00000008u /* C */; valid ASCII pattern and matching subject [pcre2.h:119]` | [x] |
| 87 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_DOLLAR_ENDONLY=0x00000010u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:120]` | [x] |
| 88 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_DOTALL=0x00000020u /* C */; valid ASCII pattern and matching subject [pcre2.h:121]` | [x] |
| 89 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_DUPNAMES=0x00000040u /* C */; valid ASCII pattern and matching subject [pcre2.h:122]` | [x] |
| 90 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_EXTENDED=0x00000080u /* C */; valid ASCII pattern and matching subject [pcre2.h:123]` | [x] |
| 91 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_FIRSTLINE=0x00000100u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:124]` | [x] |
| 92 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_MATCH_UNSET_BACKREF=0x00000200u /* C J M */; valid ASCII pattern and matching subject [pcre2.h:125]` | [x] |
| 93 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_MULTILINE=0x00000400u /* C */; valid ASCII pattern and matching subject [pcre2.h:126]` | [x] |
| 94 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NEVER_UCP=0x00000800u /* C */; valid ASCII pattern and matching subject [pcre2.h:127]` | [x] |
| 95 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NEVER_UTF=0x00001000u /* C */; valid ASCII pattern and matching subject [pcre2.h:128]` | [x] |
| 96 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NO_AUTO_CAPTURE=0x00002000u /* C */; valid ASCII pattern and matching subject [pcre2.h:129]` | [x] |
| 97 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NO_AUTO_POSSESS=0x00004000u /* C */; valid ASCII pattern and matching subject [pcre2.h:130]` | [x] |
| 98 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NO_DOTSTAR_ANCHOR=0x00008000u /* C */; valid ASCII pattern and matching subject [pcre2.h:131]` | [x] |
| 99 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NO_START_OPTIMIZE=0x00010000u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:132]` | [x] |
| 100 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_UCP=0x00020000u /* C J M D */; valid ASCII pattern and matching subject [pcre2.h:133]` | [x] |
| 101 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_UNGREEDY=0x00040000u /* C */; valid ASCII pattern and matching subject [pcre2.h:134]` | [x] |
| 102 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_UTF=0x00080000u /* C J M D */; valid ASCII pattern and matching subject [pcre2.h:135]` | [x] |
| 103 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_NEVER_BACKSLASH_C=0x00100000u /* C */; valid ASCII pattern and matching subject [pcre2.h:136]` | [x] |
| 104 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ALT_CIRCUMFLEX=0x00200000u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:137]` | [x] |
| 105 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ALT_VERBNAMES=0x00400000u /* C */; valid ASCII pattern and matching subject [pcre2.h:138]` | [x] |
| 106 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_USE_OFFSET_LIMIT=0x00800000u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:139]` | [x] |
| 107 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_EXTENDED_MORE=0x01000000u /* C */; valid ASCII pattern and matching subject [pcre2.h:140]` | [x] |
| 108 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_LITERAL=0x02000000u /* C */; valid ASCII pattern and matching subject [pcre2.h:141]` | [x] |
| 109 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_MATCH_INVALID_UTF=0x04000000u /* J M D */; valid ASCII pattern and matching subject [pcre2.h:142]` | [x] |
| 110 | `pcre2_compile_8 -> pcre2_match_8` | `compile option PCRE2_ALT_EXTENDED_CLASS=0x08000000u /* C */; valid ASCII pattern and matching subject [pcre2.h:143]` | [x] |
| 111 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES=0x00000001u /* C */; valid pattern exercising that option [pcre2.h:147]` | [x] |
| 112 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL=0x00000002u /* C */; valid pattern exercising that option [pcre2.h:148]` | [x] |
| 113 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_MATCH_WORD=0x00000004u /* C */; valid pattern exercising that option [pcre2.h:149]` | [x] |
| 114 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_MATCH_LINE=0x00000008u /* C */; valid pattern exercising that option [pcre2.h:150]` | [x] |
| 115 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ESCAPED_CR_IS_LF=0x00000010u /* C */; valid pattern exercising that option [pcre2.h:151]` | [x] |
| 116 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ALT_BSUX=0x00000020u /* C */; valid pattern exercising that option [pcre2.h:152]` | [x] |
| 117 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK=0x00000040u /* C */; valid pattern exercising that option [pcre2.h:153]` | [x] |
| 118 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_CASELESS_RESTRICT=0x00000080u /* C */; valid pattern exercising that option [pcre2.h:154]` | [x] |
| 119 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ASCII_BSD=0x00000100u /* C */; valid pattern exercising that option [pcre2.h:155]` | [x] |
| 120 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ASCII_BSS=0x00000200u /* C */; valid pattern exercising that option [pcre2.h:156]` | [x] |
| 121 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ASCII_BSW=0x00000400u /* C */; valid pattern exercising that option [pcre2.h:157]` | [x] |
| 122 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ASCII_POSIX=0x00000800u /* C */; valid pattern exercising that option [pcre2.h:158]` | [x] |
| 123 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_ASCII_DIGIT=0x00001000u /* C */; valid pattern exercising that option [pcre2.h:159]` | [x] |
| 124 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_PYTHON_OCTAL=0x00002000u /* C */; valid pattern exercising that option [pcre2.h:160]` | [x] |
| 125 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_NO_BS0=0x00004000u /* C */; valid pattern exercising that option [pcre2.h:161]` | [x] |
| 126 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_NEVER_CALLOUT=0x00008000u /* C */; valid pattern exercising that option [pcre2.h:162]` | [x] |
| 127 | `pcre2_set_compile_extra_options_8 -> pcre2_compile_8` | `extra compile option PCRE2_EXTRA_TURKISH_CASING=0x00010000u /* C */; valid pattern exercising that option [pcre2.h:163]` | [x] |
| 128 | `pcre2_match_8` | `runtime option PCRE2_NOTBOL=0x00000001u; valid compiled pattern and subject [pcre2.h:179]` | [x] |
| 129 | `pcre2_match_8` | `runtime option PCRE2_NOTEOL=0x00000002u; valid compiled pattern and subject [pcre2.h:180]` | [x] |
| 130 | `pcre2_match_8` | `runtime option PCRE2_NOTEMPTY=0x00000004u /* ) These two must be kept */; valid compiled pattern and subject [pcre2.h:181]` | [x] |
| 131 | `pcre2_match_8` | `runtime option PCRE2_NOTEMPTY_ATSTART=0x00000008u /* ) adjacent to each other. */; valid compiled pattern and subject [pcre2.h:182]` | [x] |
| 132 | `pcre2_match_8` | `runtime option PCRE2_PARTIAL_SOFT=0x00000010u; valid compiled pattern and subject [pcre2.h:183]` | [x] |
| 133 | `pcre2_match_8` | `runtime option PCRE2_PARTIAL_HARD=0x00000020u; valid compiled pattern and subject [pcre2.h:184]` | [x] |
| 134 | `pcre2_dfa_match_8` | `runtime option PCRE2_DFA_RESTART=0x00000040u /* pcre2_dfa_match() only */; valid compiled pattern and subject [pcre2.h:185]` | [x] |
| 135 | `pcre2_dfa_match_8` | `runtime option PCRE2_DFA_SHORTEST=0x00000080u /* pcre2_dfa_match() only */; valid compiled pattern and subject [pcre2.h:186]` | [x] |
| 136 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_GLOBAL=0x00000100u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:187]` | [x] |
| 137 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_EXTENDED=0x00000200u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:188]` | [x] |
| 138 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_UNSET_EMPTY=0x00000400u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:189]` | [x] |
| 139 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_UNKNOWN_UNSET=0x00000800u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:190]` | [x] |
| 140 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_OVERFLOW_LENGTH=0x00001000u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:191]` | [x] |
| 141 | `pcre2_match_8` | `runtime option PCRE2_NO_JIT=0x00002000u /* not for pcre2_dfa_match() */; valid compiled pattern and subject [pcre2.h:192]` | [x] |
| 142 | `pcre2_match_8` | `runtime option PCRE2_COPY_MATCHED_SUBJECT=0x00004000u; valid compiled pattern and subject [pcre2.h:193]` | [x] |
| 143 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_LITERAL=0x00008000u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:194]` | [x] |
| 144 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_MATCHED=0x00010000u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:195]` | [x] |
| 145 | `pcre2_substitute_8` | `runtime option PCRE2_SUBSTITUTE_REPLACEMENT_ONLY=0x00020000u /* pcre2_substitute() only */; valid compiled pattern and subject [pcre2.h:196]` | [x] |
| 146 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_UTF=0x00000001u; valid source pattern [pcre2.h:201]` | [x] |
| 147 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_NO_UTF_CHECK=0x00000002u; valid source pattern [pcre2.h:202]` | [x] |
| 148 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_POSIX_BASIC=0x00000004u; valid source pattern [pcre2.h:203]` | [x] |
| 149 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_POSIX_EXTENDED=0x00000008u; valid source pattern [pcre2.h:204]` | [x] |
| 150 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_GLOB=0x00000010u; valid source pattern [pcre2.h:205]` | [x] |
| 151 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR=0x00000030u; valid source pattern [pcre2.h:206]` | [x] |
| 152 | `pcre2_pattern_convert_8` | `conversion mode PCRE2_CONVERT_GLOB_NO_STARSTAR=0x00000050u; valid source pattern [pcre2.h:207]` | [x] |
| 153 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_CR=1; pattern and subject containing line boundaries [pcre2.h:213]` | [x] |
| 154 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_LF=2; pattern and subject containing line boundaries [pcre2.h:214]` | [x] |
| 155 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_CRLF=3; pattern and subject containing line boundaries [pcre2.h:215]` | [x] |
| 156 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_ANY=4; pattern and subject containing line boundaries [pcre2.h:216]` | [x] |
| 157 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_ANYCRLF=5; pattern and subject containing line boundaries [pcre2.h:217]` | [x] |
| 158 | `pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8` | `newline convention PCRE2_NEWLINE_NUL=6; pattern and subject containing line boundaries [pcre2.h:218]` | [x] |
| 159 | `pcre2_set_bsr_8 -> pcre2_compile_8 -> pcre2_match_8` | `backslash-R convention PCRE2_BSR_UNICODE=1; Unicode and CR/LF subjects [pcre2.h:220]` | [x] |
| 160 | `pcre2_set_bsr_8 -> pcre2_compile_8 -> pcre2_match_8` | `backslash-R convention PCRE2_BSR_ANYCRLF=2; Unicode and CR/LF subjects [pcre2.h:221]` | [x] |
| 161 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_ALLOPTIONS=0 on a valid compiled pattern [pcre2.h:446]` | [x] |
| 162 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_ARGOPTIONS=1 on a valid compiled pattern [pcre2.h:447]` | [x] |
| 163 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_BACKREFMAX=2 on a valid compiled pattern [pcre2.h:448]` | [x] |
| 164 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_BSR=3 on a valid compiled pattern [pcre2.h:449]` | [x] |
| 165 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_CAPTURECOUNT=4 on a valid compiled pattern [pcre2.h:450]` | [x] |
| 166 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_FIRSTCODEUNIT=5 on a valid compiled pattern [pcre2.h:451]` | [x] |
| 167 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_FIRSTCODETYPE=6 on a valid compiled pattern [pcre2.h:452]` | [x] |
| 168 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_FIRSTBITMAP=7 on a valid compiled pattern [pcre2.h:453]` | [x] |
| 169 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_HASCRORLF=8 on a valid compiled pattern [pcre2.h:454]` | [x] |
| 170 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_JCHANGED=9 on a valid compiled pattern [pcre2.h:455]` | [x] |
| 171 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_JITSIZE=10 on a valid compiled pattern [pcre2.h:456]` | [x] |
| 172 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_LASTCODEUNIT=11 on a valid compiled pattern [pcre2.h:457]` | [x] |
| 173 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_LASTCODETYPE=12 on a valid compiled pattern [pcre2.h:458]` | [x] |
| 174 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_MATCHEMPTY=13 on a valid compiled pattern [pcre2.h:459]` | [x] |
| 175 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_MATCHLIMIT=14 on a valid compiled pattern [pcre2.h:460]` | [x] |
| 176 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_MAXLOOKBEHIND=15 on a valid compiled pattern [pcre2.h:461]` | [x] |
| 177 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_MINLENGTH=16 on a valid compiled pattern [pcre2.h:462]` | [x] |
| 178 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_NAMECOUNT=17 on a valid compiled pattern [pcre2.h:463]` | [x] |
| 179 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_NAMEENTRYSIZE=18 on a valid compiled pattern [pcre2.h:464]` | [x] |
| 180 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_NAMETABLE=19 on a valid compiled pattern [pcre2.h:465]` | [x] |
| 181 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_NEWLINE=20 on a valid compiled pattern [pcre2.h:466]` | [x] |
| 182 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_DEPTHLIMIT=21 on a valid compiled pattern [pcre2.h:467]` | [x] |
| 183 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_RECURSIONLIMIT=21 /* Obsolete synonym */ on a valid compiled pattern [pcre2.h:468]` | [x] |
| 184 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_SIZE=22 on a valid compiled pattern [pcre2.h:469]` | [x] |
| 185 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_HASBACKSLASHC=23 on a valid compiled pattern [pcre2.h:470]` | [x] |
| 186 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_FRAMESIZE=24 on a valid compiled pattern [pcre2.h:471]` | [x] |
| 187 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_HEAPLIMIT=25 on a valid compiled pattern [pcre2.h:472]` | [x] |
| 188 | `pcre2_pattern_info_8` | `selector PCRE2_INFO_EXTRAOPTIONS=26 on a valid compiled pattern [pcre2.h:473]` | [x] |
| 189 | `pcre2_config_8` | `selector PCRE2_CONFIG_BSR=0 with correctly typed output storage [pcre2.h:477]` | [x] |
| 190 | `pcre2_config_8` | `selector PCRE2_CONFIG_JIT=1 with correctly typed output storage [pcre2.h:478]` | [x] |
| 191 | `pcre2_config_8` | `selector PCRE2_CONFIG_JITTARGET=2 with correctly typed output storage [pcre2.h:479]` | [x] |
| 192 | `pcre2_config_8` | `selector PCRE2_CONFIG_LINKSIZE=3 with correctly typed output storage [pcre2.h:480]` | [x] |
| 193 | `pcre2_config_8` | `selector PCRE2_CONFIG_MATCHLIMIT=4 with correctly typed output storage [pcre2.h:481]` | [x] |
| 194 | `pcre2_config_8` | `selector PCRE2_CONFIG_NEWLINE=5 with correctly typed output storage [pcre2.h:482]` | [x] |
| 195 | `pcre2_config_8` | `selector PCRE2_CONFIG_PARENSLIMIT=6 with correctly typed output storage [pcre2.h:483]` | [x] |
| 196 | `pcre2_config_8` | `selector PCRE2_CONFIG_DEPTHLIMIT=7 with correctly typed output storage [pcre2.h:484]` | [x] |
| 197 | `pcre2_config_8` | `selector PCRE2_CONFIG_RECURSIONLIMIT=7 /* Obsolete synonym */ with correctly typed output storage [pcre2.h:485]` | [x] |
| 198 | `pcre2_config_8` | `selector PCRE2_CONFIG_STACKRECURSE=8 /* Obsolete */ with correctly typed output storage [pcre2.h:486]` | [x] |
| 199 | `pcre2_config_8` | `selector PCRE2_CONFIG_UNICODE=9 with correctly typed output storage [pcre2.h:487]` | [x] |
| 200 | `pcre2_config_8` | `selector PCRE2_CONFIG_UNICODE_VERSION=10 with correctly typed output storage [pcre2.h:488]` | [x] |
| 201 | `pcre2_config_8` | `selector PCRE2_CONFIG_VERSION=11 with correctly typed output storage [pcre2.h:489]` | [x] |
| 202 | `pcre2_config_8` | `selector PCRE2_CONFIG_HEAPLIMIT=12 with correctly typed output storage [pcre2.h:490]` | [x] |
| 203 | `pcre2_config_8` | `selector PCRE2_CONFIG_NEVER_BACKSLASH_C=13 with correctly typed output storage [pcre2.h:491]` | [x] |
| 204 | `pcre2_config_8` | `selector PCRE2_CONFIG_COMPILED_WIDTHS=14 with correctly typed output storage [pcre2.h:492]` | [x] |
| 205 | `pcre2_config_8` | `selector PCRE2_CONFIG_TABLES_LENGTH=15 with correctly typed output storage [pcre2.h:493]` | [x] |
| 206 | `pcre2_config_8` | `selector PCRE2_CONFIG_EFFECTIVE_LINKSIZE=16 with correctly typed output storage [pcre2.h:494]` | [x] |
| 207 | `pcre2_compile_8` | `pattern shape: empty` | [x] |
| 208 | `pcre2_compile_8` | `pattern shape: one literal byte` | [x] |
| 209 | `pcre2_compile_8` | `pattern shape: many literals` | [x] |
| 210 | `pcre2_compile_8` | `pattern shape: alternation, captures, named captures, backreferences` | [x] |
| 211 | `pcre2_compile_8` | `pattern shape: greedy, lazy, and possessive quantifiers at boundaries 0, 1, 65535` | [x] |
| 212 | `pcre2_compile_8` | `pattern shape: lookahead, fixed lookbehind, variable lookbehind` | [x] |
| 213 | `pcre2_compile_8` | `pattern shape: 8-bit UTF-8 with Unicode properties and classes` | [x] |
| 214 | `pcre2_compile_8` | `length shape: explicit zero, explicit byte length, PCRE2_ZERO_TERMINATED` | [x] |
| 215 | `pcre2_match_8` | `subject shape: empty, one byte, many bytes` | [x] |
| 216 | `pcre2_match_8` | `subject shape: embedded NUL with explicit length` | [x] |
| 217 | `pcre2_match_8` | `subject shape: valid multibyte UTF-8 at start, middle, and end` | [x] |
| 218 | `pcre2_match_8` | `start offset shape: zero, interior code-unit boundary, subject length` | [x] |
| 219 | `pcre2_dfa_match_8` | `workspace shape: minimum viable and larger workspace` | [x] |
| 220 | `pcre2_substitute_8` | `replacement shape: empty, literal, numbered capture, named capture, case transform` | [x] |
| 221 | `pcre2_substitute_8` | `output shape: exact capacity, excess capacity, overflow-length query` | [x] |
| 222 | `pcre2_serialize_encode_8 -> pcre2_serialize_decode_8` | `code count shape: one and many` | [x] |
| 223 | `pcre2_match_data_create_8` | `ovector pair count shape: zero, one, many, maximum uint32_t allocation failure` | [x] |
| 224 | `pcre2_general_context_create_8` | `allocator shape: libc-compatible custom allocator and default context` | [x] |
| 225 | `pcre2_maketables_8` | `character table shape: default general context and custom general context` | [x] |
