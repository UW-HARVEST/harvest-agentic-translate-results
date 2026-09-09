# Configuration surface

Mechanically based on every exported public entry point and every public option
constant family in `pcre2.h`, plus input shapes explicitly distinguished by the APIs.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---:|----------------|--------------------------------------------|-----|
| 1 | `pcre2_callout_enumerate_8` | baseline valid call; null/default context where permitted | [x] |
| 2 | `pcre2_code_copy_8` | baseline valid call; null/default context where permitted | [x] |
| 3 | `pcre2_code_copy_with_tables_8` | baseline valid call; null/default context where permitted | [x] |
| 4 | `pcre2_code_free_8` | baseline valid call; null/default context where permitted | [x] |
| 5 | `pcre2_compile_8` | baseline valid call; null/default context where permitted | [x] |
| 6 | `pcre2_compile_context_copy_8` | baseline valid call; null/default context where permitted | [x] |
| 7 | `pcre2_compile_context_create_8` | baseline valid call; null/default context where permitted | [x] |
| 8 | `pcre2_compile_context_free_8` | baseline valid call; null/default context where permitted | [x] |
| 9 | `pcre2_config_8` | baseline valid call; null/default context where permitted | [x] |
| 10 | `pcre2_convert_context_copy_8` | baseline valid call; null/default context where permitted | [x] |
| 11 | `pcre2_convert_context_create_8` | baseline valid call; null/default context where permitted | [x] |
| 12 | `pcre2_convert_context_free_8` | baseline valid call; null/default context where permitted | [x] |
| 13 | `pcre2_converted_pattern_free_8` | baseline valid call; null/default context where permitted | [x] |
| 14 | `pcre2_dfa_match_8` | baseline valid call; null/default context where permitted | [x] |
| 15 | `pcre2_general_context_copy_8` | baseline valid call; null/default context where permitted | [x] |
| 16 | `pcre2_general_context_create_8` | baseline valid call; null/default context where permitted | [x] |
| 17 | `pcre2_general_context_free_8` | baseline valid call; null/default context where permitted | [x] |
| 18 | `pcre2_get_error_message_8` | baseline valid call; null/default context where permitted | [x] |
| 19 | `pcre2_get_mark_8` | baseline valid call; null/default context where permitted | [x] |
| 20 | `pcre2_get_match_data_heapframes_size_8` | baseline valid call; null/default context where permitted | [x] |
| 21 | `pcre2_get_match_data_size_8` | baseline valid call; null/default context where permitted | [x] |
| 22 | `pcre2_get_ovector_count_8` | baseline valid call; null/default context where permitted | [x] |
| 23 | `pcre2_get_ovector_pointer_8` | baseline valid call; null/default context where permitted | [x] |
| 24 | `pcre2_get_startchar_8` | baseline valid call; null/default context where permitted | [x] |
| 25 | `pcre2_jit_compile_8` | baseline valid call; null/default context where permitted | [x] |
| 26 | `pcre2_jit_free_unused_memory_8` | baseline valid call; null/default context where permitted | [x] |
| 27 | `pcre2_jit_match_8` | baseline valid call; null/default context where permitted | [x] |
| 28 | `pcre2_jit_stack_assign_8` | baseline valid call; null/default context where permitted | [x] |
| 29 | `pcre2_jit_stack_create_8` | baseline valid call; null/default context where permitted | [x] |
| 30 | `pcre2_jit_stack_free_8` | baseline valid call; null/default context where permitted | [x] |
| 31 | `pcre2_maketables_8` | baseline valid call; null/default context where permitted | [x] |
| 32 | `pcre2_maketables_free_8` | baseline valid call; null/default context where permitted | [x] |
| 33 | `pcre2_match_8` | baseline valid call; null/default context where permitted | [x] |
| 34 | `pcre2_match_context_copy_8` | baseline valid call; null/default context where permitted | [x] |
| 35 | `pcre2_match_context_create_8` | baseline valid call; null/default context where permitted | [x] |
| 36 | `pcre2_match_context_free_8` | baseline valid call; null/default context where permitted | [x] |
| 37 | `pcre2_match_data_create_8` | baseline valid call; null/default context where permitted | [x] |
| 38 | `pcre2_match_data_create_from_pattern_8` | baseline valid call; null/default context where permitted | [x] |
| 39 | `pcre2_match_data_free_8` | baseline valid call; null/default context where permitted | [x] |
| 40 | `pcre2_next_match_8` | baseline valid call; null/default context where permitted | [x] |
| 41 | `pcre2_pattern_convert_8` | baseline valid call; null/default context where permitted | [x] |
| 42 | `pcre2_pattern_info_8` | baseline valid call; null/default context where permitted | [x] |
| 43 | `pcre2_serialize_decode_8` | baseline valid call; null/default context where permitted | [x] |
| 44 | `pcre2_serialize_encode_8` | baseline valid call; null/default context where permitted | [x] |
| 45 | `pcre2_serialize_free_8` | baseline valid call; null/default context where permitted | [x] |
| 46 | `pcre2_serialize_get_number_of_codes_8` | baseline valid call; null/default context where permitted | [x] |
| 47 | `pcre2_set_bsr_8` | baseline valid call; null/default context where permitted | [x] |
| 48 | `pcre2_set_callout_8` | baseline valid call; null/default context where permitted | [x] |
| 49 | `pcre2_set_character_tables_8` | baseline valid call; null/default context where permitted | [x] |
| 50 | `pcre2_set_compile_extra_options_8` | baseline valid call; null/default context where permitted | [x] |
| 51 | `pcre2_set_compile_recursion_guard_8` | baseline valid call; null/default context where permitted | [x] |
| 52 | `pcre2_set_depth_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 53 | `pcre2_set_glob_escape_8` | baseline valid call; null/default context where permitted | [x] |
| 54 | `pcre2_set_glob_separator_8` | baseline valid call; null/default context where permitted | [x] |
| 55 | `pcre2_set_heap_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 56 | `pcre2_set_match_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 57 | `pcre2_set_max_pattern_compiled_length_8` | baseline valid call; null/default context where permitted | [x] |
| 58 | `pcre2_set_max_pattern_length_8` | baseline valid call; null/default context where permitted | [x] |
| 59 | `pcre2_set_max_varlookbehind_8` | baseline valid call; null/default context where permitted | [x] |
| 60 | `pcre2_set_newline_8` | baseline valid call; null/default context where permitted | [x] |
| 61 | `pcre2_set_offset_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 62 | `pcre2_set_optimize_8` | baseline valid call; null/default context where permitted | [x] |
| 63 | `pcre2_set_parens_nest_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 64 | `pcre2_set_recursion_limit_8` | baseline valid call; null/default context where permitted | [x] |
| 65 | `pcre2_set_recursion_memory_management_8` | baseline valid call; null/default context where permitted | [x] |
| 66 | `pcre2_set_substitute_callout_8` | baseline valid call; null/default context where permitted | [x] |
| 67 | `pcre2_set_substitute_case_callout_8` | baseline valid call; null/default context where permitted | [x] |
| 68 | `pcre2_substitute_8` | baseline valid call; null/default context where permitted | [x] |
| 69 | `pcre2_substring_copy_byname_8` | baseline valid call; null/default context where permitted | [x] |
| 70 | `pcre2_substring_copy_bynumber_8` | baseline valid call; null/default context where permitted | [x] |
| 71 | `pcre2_substring_free_8` | baseline valid call; null/default context where permitted | [x] |
| 72 | `pcre2_substring_get_byname_8` | baseline valid call; null/default context where permitted | [x] |
| 73 | `pcre2_substring_get_bynumber_8` | baseline valid call; null/default context where permitted | [x] |
| 74 | `pcre2_substring_length_byname_8` | baseline valid call; null/default context where permitted | [x] |
| 75 | `pcre2_substring_length_bynumber_8` | baseline valid call; null/default context where permitted | [x] |
| 76 | `pcre2_substring_list_free_8` | baseline valid call; null/default context where permitted | [x] |
| 77 | `pcre2_substring_list_get_8` | baseline valid call; null/default context where permitted | [x] |
| 78 | `pcre2_substring_nametable_scan_8` | baseline valid call; null/default context where permitted | [x] |
| 79 | `pcre2_substring_number_from_name_8` | baseline valid call; null/default context where permitted | [x] |
| 80 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ANCHORED` = `0x80000000u` | [x] |
| 81 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ENDANCHORED` = `0x20000000u` | [x] |
| 82 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ALLOW_EMPTY_CLASS` = `0x00000001u` | [x] |
| 83 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ALT_BSUX` = `0x00000002u` | [x] |
| 84 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_AUTO_CALLOUT` = `0x00000004u` | [x] |
| 85 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_CASELESS` = `0x00000008u` | [x] |
| 86 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_DOLLAR_ENDONLY` = `0x00000010u` | [x] |
| 87 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_DOTALL` = `0x00000020u` | [x] |
| 88 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_DUPNAMES` = `0x00000040u` | [x] |
| 89 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_EXTENDED` = `0x00000080u` | [x] |
| 90 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_FIRSTLINE` = `0x00000100u` | [x] |
| 91 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_MATCH_UNSET_BACKREF` = `0x00000200u` | [x] |
| 92 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_MULTILINE` = `0x00000400u` | [x] |
| 93 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NEVER_UCP` = `0x00000800u` | [x] |
| 94 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NEVER_UTF` = `0x00001000u` | [x] |
| 95 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NO_AUTO_CAPTURE` = `0x00002000u` | [x] |
| 96 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NO_AUTO_POSSESS` = `0x00004000u` | [x] |
| 97 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NO_DOTSTAR_ANCHOR` = `0x00008000u` | [x] |
| 98 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NO_START_OPTIMIZE` = `0x00010000u` | [x] |
| 99 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_UCP` = `0x00020000u` | [x] |
| 100 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_UNGREEDY` = `0x00040000u` | [x] |
| 101 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_UTF` = `0x00080000u` | [x] |
| 102 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NEVER_BACKSLASH_C` = `0x00100000u` | [x] |
| 103 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ALT_CIRCUMFLEX` = `0x00200000u` | [x] |
| 104 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ALT_VERBNAMES` = `0x00400000u` | [x] |
| 105 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_USE_OFFSET_LIMIT` = `0x00800000u` | [x] |
| 106 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_EXTENDED_MORE` = `0x01000000u` | [x] |
| 107 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_LITERAL` = `0x02000000u` | [x] |
| 108 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_MATCH_INVALID_UTF` = `0x04000000u` | [x] |
| 109 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_ALT_EXTENDED_CLASS` = `0x08000000u` | [x] |
| 110 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_AUTO_POSSESS` = `64` | [x] |
| 111 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_AUTO_POSSESS_OFF` = `65` | [x] |
| 112 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_MATCH_CONTEXT_FUNCTIONS` = `\` | [x] |
| 113 | `pcre2_compile_8, pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_MATCH_FUNCTIONS` = `\` | [x] |
| 114 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES` = `0x00000001u` | [x] |
| 115 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL` = `0x00000002u` | [x] |
| 116 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_MATCH_WORD` = `0x00000004u` | [x] |
| 117 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_MATCH_LINE` = `0x00000008u` | [x] |
| 118 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ESCAPED_CR_IS_LF` = `0x00000010u` | [x] |
| 119 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ALT_BSUX` = `0x00000020u` | [x] |
| 120 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK` = `0x00000040u` | [x] |
| 121 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_CASELESS_RESTRICT` = `0x00000080u` | [x] |
| 122 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ASCII_BSD` = `0x00000100u` | [x] |
| 123 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ASCII_BSS` = `0x00000200u` | [x] |
| 124 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ASCII_BSW` = `0x00000400u` | [x] |
| 125 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ASCII_POSIX` = `0x00000800u` | [x] |
| 126 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_ASCII_DIGIT` = `0x00001000u` | [x] |
| 127 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_PYTHON_OCTAL` = `0x00002000u` | [x] |
| 128 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_NO_BS0` = `0x00004000u` | [x] |
| 129 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_NEVER_CALLOUT` = `0x00008000u` | [x] |
| 130 | `pcre2_set_compile_extra_options_8 + pcre2_compile_8` | `PCRE2_EXTRA_TURKISH_CASING` = `0x00010000u` | [x] |
| 131 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NOTBOL` = `0x00000001u` | [x] |
| 132 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NOTEOL` = `0x00000002u` | [x] |
| 133 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NOTEMPTY` = `0x00000004u` | [x] |
| 134 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NOTEMPTY_ATSTART` = `0x00000008u` | [x] |
| 135 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_PARTIAL_SOFT` = `0x00000010u` | [x] |
| 136 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_PARTIAL_HARD` = `0x00000020u` | [x] |
| 137 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_DFA_RESTART` = `0x00000040u` | [x] |
| 138 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_DFA_SHORTEST` = `0x00000080u` | [x] |
| 139 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_NO_JIT` = `0x00002000u` | [x] |
| 140 | `pcre2_match_8, pcre2_dfa_match_8` | `PCRE2_COPY_MATCHED_SUBJECT` = `0x00004000u` | [x] |
| 141 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_COMPLETE` = `0x00000001u` | [x] |
| 142 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_PARTIAL_SOFT` = `0x00000002u` | [x] |
| 143 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_PARTIAL_HARD` = `0x00000004u` | [x] |
| 144 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_INVALID_UTF` = `0x00000100u` | [x] |
| 145 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_TEST_ALLOC` = `0x00000200u` | [x] |
| 146 | `pcre2_jit_compile_8, pcre2_jit_match_8` | `PCRE2_JIT_FUNCTIONS` = `\` | [x] |
| 147 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_GLOBAL` = `0x00000100u` | [x] |
| 148 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_EXTENDED` = `0x00000200u` | [x] |
| 149 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNSET_EMPTY` = `0x00000400u` | [x] |
| 150 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_UNKNOWN_UNSET` = `0x00000800u` | [x] |
| 151 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` = `0x00001000u` | [x] |
| 152 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_LITERAL` = `0x00008000u` | [x] |
| 153 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_MATCHED` = `0x00010000u` | [x] |
| 154 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_REPLACEMENT_ONLY` = `0x00020000u` | [x] |
| 155 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_CASE_LOWER` = `1` | [x] |
| 156 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_CASE_UPPER` = `2` | [x] |
| 157 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_CASE_TITLE_FIRST` = `3` | [x] |
| 158 | `pcre2_substitute_8` | `PCRE2_SUBSTITUTE_FUNCTION` = `\` | [x] |
| 159 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_UTF` = `0x00000001u` | [x] |
| 160 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_NO_UTF_CHECK` = `0x00000002u` | [x] |
| 161 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_BASIC` = `0x00000004u` | [x] |
| 162 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_POSIX_EXTENDED` = `0x00000008u` | [x] |
| 163 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB` = `0x00000010u` | [x] |
| 164 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR` = `0x00000030u` | [x] |
| 165 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_GLOB_NO_STARSTAR` = `0x00000050u` | [x] |
| 166 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_CONTEXT_FUNCTIONS` = `\` | [x] |
| 167 | `pcre2_pattern_convert_8` | `PCRE2_CONVERT_FUNCTIONS` = `\` | [x] |
| 168 | `pcre2_config_8` | `PCRE2_CONFIG_BSR` = `0` | [x] |
| 169 | `pcre2_config_8` | `PCRE2_CONFIG_JIT` = `1` | [x] |
| 170 | `pcre2_config_8` | `PCRE2_CONFIG_JITTARGET` = `2` | [x] |
| 171 | `pcre2_config_8` | `PCRE2_CONFIG_LINKSIZE` = `3` | [x] |
| 172 | `pcre2_config_8` | `PCRE2_CONFIG_MATCHLIMIT` = `4` | [x] |
| 173 | `pcre2_config_8` | `PCRE2_CONFIG_NEWLINE` = `5` | [x] |
| 174 | `pcre2_config_8` | `PCRE2_CONFIG_PARENSLIMIT` = `6` | [x] |
| 175 | `pcre2_config_8` | `PCRE2_CONFIG_DEPTHLIMIT` = `7` | [x] |
| 176 | `pcre2_config_8` | `PCRE2_CONFIG_RECURSIONLIMIT` = `7` | [x] |
| 177 | `pcre2_config_8` | `PCRE2_CONFIG_STACKRECURSE` = `8` | [x] |
| 178 | `pcre2_config_8` | `PCRE2_CONFIG_UNICODE` = `9` | [x] |
| 179 | `pcre2_config_8` | `PCRE2_CONFIG_UNICODE_VERSION` = `10` | [x] |
| 180 | `pcre2_config_8` | `PCRE2_CONFIG_VERSION` = `11` | [x] |
| 181 | `pcre2_config_8` | `PCRE2_CONFIG_HEAPLIMIT` = `12` | [x] |
| 182 | `pcre2_config_8` | `PCRE2_CONFIG_NEVER_BACKSLASH_C` = `13` | [x] |
| 183 | `pcre2_config_8` | `PCRE2_CONFIG_COMPILED_WIDTHS` = `14` | [x] |
| 184 | `pcre2_config_8` | `PCRE2_CONFIG_TABLES_LENGTH` = `15` | [x] |
| 185 | `pcre2_config_8` | `PCRE2_CONFIG_EFFECTIVE_LINKSIZE` = `16` | [x] |
| 186 | `pcre2_pattern_info_8` | `PCRE2_INFO_ALLOPTIONS` = `0` | [x] |
| 187 | `pcre2_pattern_info_8` | `PCRE2_INFO_ARGOPTIONS` = `1` | [x] |
| 188 | `pcre2_pattern_info_8` | `PCRE2_INFO_BACKREFMAX` = `2` | [x] |
| 189 | `pcre2_pattern_info_8` | `PCRE2_INFO_BSR` = `3` | [x] |
| 190 | `pcre2_pattern_info_8` | `PCRE2_INFO_CAPTURECOUNT` = `4` | [x] |
| 191 | `pcre2_pattern_info_8` | `PCRE2_INFO_FIRSTCODEUNIT` = `5` | [x] |
| 192 | `pcre2_pattern_info_8` | `PCRE2_INFO_FIRSTCODETYPE` = `6` | [x] |
| 193 | `pcre2_pattern_info_8` | `PCRE2_INFO_FIRSTBITMAP` = `7` | [x] |
| 194 | `pcre2_pattern_info_8` | `PCRE2_INFO_HASCRORLF` = `8` | [x] |
| 195 | `pcre2_pattern_info_8` | `PCRE2_INFO_JCHANGED` = `9` | [x] |
| 196 | `pcre2_pattern_info_8` | `PCRE2_INFO_JITSIZE` = `10` | [x] |
| 197 | `pcre2_pattern_info_8` | `PCRE2_INFO_LASTCODEUNIT` = `11` | [x] |
| 198 | `pcre2_pattern_info_8` | `PCRE2_INFO_LASTCODETYPE` = `12` | [x] |
| 199 | `pcre2_pattern_info_8` | `PCRE2_INFO_MATCHEMPTY` = `13` | [x] |
| 200 | `pcre2_pattern_info_8` | `PCRE2_INFO_MATCHLIMIT` = `14` | [x] |
| 201 | `pcre2_pattern_info_8` | `PCRE2_INFO_MAXLOOKBEHIND` = `15` | [x] |
| 202 | `pcre2_pattern_info_8` | `PCRE2_INFO_MINLENGTH` = `16` | [x] |
| 203 | `pcre2_pattern_info_8` | `PCRE2_INFO_NAMECOUNT` = `17` | [x] |
| 204 | `pcre2_pattern_info_8` | `PCRE2_INFO_NAMEENTRYSIZE` = `18` | [x] |
| 205 | `pcre2_pattern_info_8` | `PCRE2_INFO_NAMETABLE` = `19` | [x] |
| 206 | `pcre2_pattern_info_8` | `PCRE2_INFO_NEWLINE` = `20` | [x] |
| 207 | `pcre2_pattern_info_8` | `PCRE2_INFO_DEPTHLIMIT` = `21` | [x] |
| 208 | `pcre2_pattern_info_8` | `PCRE2_INFO_RECURSIONLIMIT` = `21` | [x] |
| 209 | `pcre2_pattern_info_8` | `PCRE2_INFO_SIZE` = `22` | [x] |
| 210 | `pcre2_pattern_info_8` | `PCRE2_INFO_HASBACKSLASHC` = `23` | [x] |
| 211 | `pcre2_pattern_info_8` | `PCRE2_INFO_FRAMESIZE` = `24` | [x] |
| 212 | `pcre2_pattern_info_8` | `PCRE2_INFO_HEAPLIMIT` = `25` | [x] |
| 213 | `pcre2_pattern_info_8` | `PCRE2_INFO_EXTRAOPTIONS` = `26` | [x] |
| 214 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_CR` = `1` | [x] |
| 215 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_LF` = `2` | [x] |
| 216 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_CRLF` = `3` | [x] |
| 217 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_ANY` = `4` | [x] |
| 218 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_ANYCRLF` = `5` | [x] |
| 219 | `pcre2_set_newline_8 + compile/match` | `PCRE2_NEWLINE_NUL` = `6` | [x] |
| 220 | `pcre2_set_bsr_8 + compile/match` | `PCRE2_BSR_UNICODE` = `1` | [x] |
| 221 | `pcre2_set_bsr_8 + compile/match` | `PCRE2_BSR_ANYCRLF` = `2` | [x] |
| 222 | `pcre2_compile_8` | pattern length: zero / one / many / PCRE2_ZERO_TERMINATED | [x] |
| 223 | `pcre2_compile_8` | pattern bytes: ASCII / UTF-8 multibyte / embedded NUL | [x] |
| 224 | `pcre2_compile_8` | captures: zero / one / many; named / duplicate-named | [x] |
| 225 | `pcre2_match_8` | subject length: zero / one / many; start offset 0 / middle / end | [x] |
| 226 | `pcre2_match_8` | result shape: no match / empty match / one capture / many captures | [x] |
| 227 | `pcre2_dfa_match_8` | workspace: minimum / larger; first call / DFA restart | [x] |
| 228 | `pcre2_next_match_8` | previous match: nonempty / empty at start / empty at end | [x] |
| 229 | `pcre2_substitute_8` | replacement: empty / literal / numbered ref / named ref / extended | [x] |
| 230 | `pcre2_substitute_8` | output: exact fit / oversized / overflow-length query | [x] |
| 231 | `pcre2_pattern_convert_8` | source: POSIX basic / POSIX extended / glob / UTF | [x] |
| 232 | `pcre2_serialize_encode_8, pcre2_serialize_decode_8` | code count: one / many | [x] |
| 233 | `pcre2_substring_*_8` | capture: set / unset / empty; by number / by name | [x] |
| 234 | `pcre2_get_error_message_8` | buffer: exact fit / oversized | [x] |
| 235 | `pcre2_callout_enumerate_8` | callouts: none / numeric / string / automatic | [x] |
| 236 | `pcre2_jit_stack_create_8` | stack sizes: start equals max / start below max | [x] |
| 237 | `context create/copy/free APIs` | default allocator / custom allocator; original / copy | [x] |
