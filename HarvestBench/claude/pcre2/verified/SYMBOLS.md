# SYMBOLS.md — exported symbol parity

Derived from `nm -D --defined-only` on both shared libraries.

- C `.so`: 143 exported symbols
- Rust `.so`: 143 exported symbols
- Missing from Rust: 0 (symbol diff is EMPTY)

All symbols carry the `_8` code-unit-width suffix (PCRE2_CODE_UNIT_WIDTH=8).

| # | symbol | in C | in Rust |
|---|--------|------|---------|
| 1 | `_pcre2_OP_lengths_8` | yes | yes |
| 2 | `_pcre2_auto_possessify_8` | yes | yes |
| 3 | `_pcre2_callout_end_delims_8` | yes | yes |
| 4 | `_pcre2_callout_start_delims_8` | yes | yes |
| 5 | `_pcre2_check_escape_8` | yes | yes |
| 6 | `_pcre2_ckd_smul_8` | yes | yes |
| 7 | `_pcre2_compile_add_name_to_table8` | yes | yes |
| 8 | `_pcre2_compile_class_nested_8` | yes | yes |
| 9 | `_pcre2_compile_class_not_nested_8` | yes | yes |
| 10 | `_pcre2_compile_find_dupname_details8` | yes | yes |
| 11 | `_pcre2_compile_find_named_group8` | yes | yes |
| 12 | `_pcre2_compile_get_hash_from_name8` | yes | yes |
| 13 | `_pcre2_compile_parse_recurse_args8` | yes | yes |
| 14 | `_pcre2_compile_parse_scan_substr_args8` | yes | yes |
| 15 | `_pcre2_default_compile_context_8` | yes | yes |
| 16 | `_pcre2_default_convert_context_8` | yes | yes |
| 17 | `_pcre2_default_match_context_8` | yes | yes |
| 18 | `_pcre2_default_tables_8` | yes | yes |
| 19 | `_pcre2_eclass_8` | yes | yes |
| 20 | `_pcre2_extuni_8` | yes | yes |
| 21 | `_pcre2_find_bracket_8` | yes | yes |
| 22 | `_pcre2_hspace_list_8` | yes | yes |
| 23 | `_pcre2_is_newline_8` | yes | yes |
| 24 | `_pcre2_jit_free_8` | yes | yes |
| 25 | `_pcre2_jit_free_rodata_8` | yes | yes |
| 26 | `_pcre2_jit_get_size_8` | yes | yes |
| 27 | `_pcre2_jit_get_target_8` | yes | yes |
| 28 | `_pcre2_memctl_malloc_8` | yes | yes |
| 29 | `_pcre2_ord2utf_8` | yes | yes |
| 30 | `_pcre2_posix_class_maps8` | yes | yes |
| 31 | `_pcre2_script_run_8` | yes | yes |
| 32 | `_pcre2_strcmp_8` | yes | yes |
| 33 | `_pcre2_strcmp_c8_8` | yes | yes |
| 34 | `_pcre2_strcpy_c8_8` | yes | yes |
| 35 | `_pcre2_strlen_8` | yes | yes |
| 36 | `_pcre2_strncmp_8` | yes | yes |
| 37 | `_pcre2_strncmp_c8_8` | yes | yes |
| 38 | `_pcre2_study_8` | yes | yes |
| 39 | `_pcre2_ucd_boolprop_sets_8` | yes | yes |
| 40 | `_pcre2_ucd_caseless_sets_8` | yes | yes |
| 41 | `_pcre2_ucd_digit_sets_8` | yes | yes |
| 42 | `_pcre2_ucd_nocase_ranges_8` | yes | yes |
| 43 | `_pcre2_ucd_nocase_ranges_size_8` | yes | yes |
| 44 | `_pcre2_ucd_records_8` | yes | yes |
| 45 | `_pcre2_ucd_script_sets_8` | yes | yes |
| 46 | `_pcre2_ucd_stage1_8` | yes | yes |
| 47 | `_pcre2_ucd_stage2_8` | yes | yes |
| 48 | `_pcre2_ucd_turkish_dotted_i_caseset_8` | yes | yes |
| 49 | `_pcre2_ucp_gbtable_8` | yes | yes |
| 50 | `_pcre2_ucp_gentype_8` | yes | yes |
| 51 | `_pcre2_unicode_version_8` | yes | yes |
| 52 | `_pcre2_update_classbits_8` | yes | yes |
| 53 | `_pcre2_utf8_table1` | yes | yes |
| 54 | `_pcre2_utf8_table1_size` | yes | yes |
| 55 | `_pcre2_utf8_table2` | yes | yes |
| 56 | `_pcre2_utf8_table3` | yes | yes |
| 57 | `_pcre2_utf8_table4` | yes | yes |
| 58 | `_pcre2_utt_8` | yes | yes |
| 59 | `_pcre2_utt_names_8` | yes | yes |
| 60 | `_pcre2_utt_size_8` | yes | yes |
| 61 | `_pcre2_valid_utf_8` | yes | yes |
| 62 | `_pcre2_vspace_list_8` | yes | yes |
| 63 | `_pcre2_was_newline_8` | yes | yes |
| 64 | `_pcre2_xclass_8` | yes | yes |
| 65 | `pcre2_callout_enumerate_8` | yes | yes |
| 66 | `pcre2_code_copy_8` | yes | yes |
| 67 | `pcre2_code_copy_with_tables_8` | yes | yes |
| 68 | `pcre2_code_free_8` | yes | yes |
| 69 | `pcre2_compile_8` | yes | yes |
| 70 | `pcre2_compile_context_copy_8` | yes | yes |
| 71 | `pcre2_compile_context_create_8` | yes | yes |
| 72 | `pcre2_compile_context_free_8` | yes | yes |
| 73 | `pcre2_config_8` | yes | yes |
| 74 | `pcre2_convert_context_copy_8` | yes | yes |
| 75 | `pcre2_convert_context_create_8` | yes | yes |
| 76 | `pcre2_convert_context_free_8` | yes | yes |
| 77 | `pcre2_converted_pattern_free_8` | yes | yes |
| 78 | `pcre2_dfa_match_8` | yes | yes |
| 79 | `pcre2_general_context_copy_8` | yes | yes |
| 80 | `pcre2_general_context_create_8` | yes | yes |
| 81 | `pcre2_general_context_free_8` | yes | yes |
| 82 | `pcre2_get_error_message_8` | yes | yes |
| 83 | `pcre2_get_mark_8` | yes | yes |
| 84 | `pcre2_get_match_data_heapframes_size_8` | yes | yes |
| 85 | `pcre2_get_match_data_size_8` | yes | yes |
| 86 | `pcre2_get_ovector_count_8` | yes | yes |
| 87 | `pcre2_get_ovector_pointer_8` | yes | yes |
| 88 | `pcre2_get_startchar_8` | yes | yes |
| 89 | `pcre2_jit_compile_8` | yes | yes |
| 90 | `pcre2_jit_free_unused_memory_8` | yes | yes |
| 91 | `pcre2_jit_match_8` | yes | yes |
| 92 | `pcre2_jit_stack_assign_8` | yes | yes |
| 93 | `pcre2_jit_stack_create_8` | yes | yes |
| 94 | `pcre2_jit_stack_free_8` | yes | yes |
| 95 | `pcre2_maketables_8` | yes | yes |
| 96 | `pcre2_maketables_free_8` | yes | yes |
| 97 | `pcre2_match_8` | yes | yes |
| 98 | `pcre2_match_context_copy_8` | yes | yes |
| 99 | `pcre2_match_context_create_8` | yes | yes |
| 100 | `pcre2_match_context_free_8` | yes | yes |
| 101 | `pcre2_match_data_create_8` | yes | yes |
| 102 | `pcre2_match_data_create_from_pattern_8` | yes | yes |
| 103 | `pcre2_match_data_free_8` | yes | yes |
| 104 | `pcre2_next_match_8` | yes | yes |
| 105 | `pcre2_pattern_convert_8` | yes | yes |
| 106 | `pcre2_pattern_info_8` | yes | yes |
| 107 | `pcre2_serialize_decode_8` | yes | yes |
| 108 | `pcre2_serialize_encode_8` | yes | yes |
| 109 | `pcre2_serialize_free_8` | yes | yes |
| 110 | `pcre2_serialize_get_number_of_codes_8` | yes | yes |
| 111 | `pcre2_set_bsr_8` | yes | yes |
| 112 | `pcre2_set_callout_8` | yes | yes |
| 113 | `pcre2_set_character_tables_8` | yes | yes |
| 114 | `pcre2_set_compile_extra_options_8` | yes | yes |
| 115 | `pcre2_set_compile_recursion_guard_8` | yes | yes |
| 116 | `pcre2_set_depth_limit_8` | yes | yes |
| 117 | `pcre2_set_glob_escape_8` | yes | yes |
| 118 | `pcre2_set_glob_separator_8` | yes | yes |
| 119 | `pcre2_set_heap_limit_8` | yes | yes |
| 120 | `pcre2_set_match_limit_8` | yes | yes |
| 121 | `pcre2_set_max_pattern_compiled_length_8` | yes | yes |
| 122 | `pcre2_set_max_pattern_length_8` | yes | yes |
| 123 | `pcre2_set_max_varlookbehind_8` | yes | yes |
| 124 | `pcre2_set_newline_8` | yes | yes |
| 125 | `pcre2_set_offset_limit_8` | yes | yes |
| 126 | `pcre2_set_optimize_8` | yes | yes |
| 127 | `pcre2_set_parens_nest_limit_8` | yes | yes |
| 128 | `pcre2_set_recursion_limit_8` | yes | yes |
| 129 | `pcre2_set_recursion_memory_management_8` | yes | yes |
| 130 | `pcre2_set_substitute_callout_8` | yes | yes |
| 131 | `pcre2_set_substitute_case_callout_8` | yes | yes |
| 132 | `pcre2_substitute_8` | yes | yes |
| 133 | `pcre2_substring_copy_byname_8` | yes | yes |
| 134 | `pcre2_substring_copy_bynumber_8` | yes | yes |
| 135 | `pcre2_substring_free_8` | yes | yes |
| 136 | `pcre2_substring_get_byname_8` | yes | yes |
| 137 | `pcre2_substring_get_bynumber_8` | yes | yes |
| 138 | `pcre2_substring_length_byname_8` | yes | yes |
| 139 | `pcre2_substring_length_bynumber_8` | yes | yes |
| 140 | `pcre2_substring_list_free_8` | yes | yes |
| 141 | `pcre2_substring_list_get_8` | yes | yes |
| 142 | `pcre2_substring_nametable_scan_8` | yes | yes |
| 143 | `pcre2_substring_number_from_name_8` | yes | yes |

## Verification result

`diff <(sort c_syms) <(sort rust_syms)` on both the release and debug Rust
`.so` reports **IDENTICAL SYMBOL SETS** — 0 missing, 0 extra. Symbol parity
holds for both build profiles.
