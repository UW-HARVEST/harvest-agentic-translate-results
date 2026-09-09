# SYMBOLS.md — dynamic-symbol parity (C .so vs Rust .so)

Generated mechanically from `nm -D --defined-only` on both shared objects.

```
C   .so: c_src/build/libpcre2.so            143 exported symbols
Rust.so: translation/target/release/libpcre2.so  143 exported symbols
missing in Rust (comm -23): 0
extra   in Rust (comm -13): 0
```

## Full symbol table

`kind` is the nm letter: `T`=text/function, `D`/`d`=initialised data, `R`/`r`=read-only data, `B`=bss.

| # | symbol | C kind | Rust kind | in Rust .so | C source | Rust source |
|---|--------|--------|-----------|-------------|----------|-------------|
| 1 | `_pcre2_OP_lengths_8` | R | R | YES | pcre2_auto_possess.c,pcre2_compile.c,pcre2_dfa_match.c | tables.rs |
| 2 | `_pcre2_auto_possessify_8` | T | T | YES | pcre2_auto_possess.c,pcre2_compile.c,pcre2_error.c | auto_possess.rs |
| 3 | `_pcre2_callout_end_delims_8` | R | R | YES | pcre2_compile.c,pcre2_printint_inc.h,pcre2_tables.c | tables.rs |
| 4 | `_pcre2_callout_start_delims_8` | R | R | YES | pcre2_compile.c,pcre2_printint_inc.h,pcre2_tables.c | tables.rs |
| 5 | `_pcre2_check_escape_8` | T | T | YES | pcre2_compile.c,pcre2_substitute.c | compile.rs |
| 6 | `_pcre2_ckd_smul_8` | T | T | YES | pcre2_chkdint.c,pcre2_compile.c | chkdint.rs |
| 7 | `_pcre2_compile_add_name_to_table8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 8 | `_pcre2_compile_class_nested_8` | T | T | YES | pcre2_compile.c,pcre2_compile_class.c | compile_class.rs |
| 9 | `_pcre2_compile_class_not_nested_8` | T | T | YES | pcre2_compile.c,pcre2_compile_class.c | compile_class.rs |
| 10 | `_pcre2_compile_find_dupname_details8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 11 | `_pcre2_compile_find_named_group8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 12 | `_pcre2_compile_get_hash_from_name8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 13 | `_pcre2_compile_parse_recurse_args8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 14 | `_pcre2_compile_parse_scan_substr_args8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c | compile_cgroup.rs |
| 15 | `_pcre2_default_compile_context_8` | D | D | YES | pcre2_context.c | context.rs |
| 16 | `_pcre2_default_convert_context_8` | D | D | YES | pcre2_context.c | context.rs |
| 17 | `_pcre2_default_match_context_8` | D | D | YES | pcre2_context.c | context.rs |
| 18 | `_pcre2_default_tables_8` | R | R | YES | pcre2_chartables.c | chartables.rs |
| 19 | `_pcre2_eclass_8` | T | T | YES | pcre2_auto_possess.c,pcre2_dfa_match.c,pcre2_match.c | xclass.rs |
| 20 | `_pcre2_extuni_8` | T | T | YES | pcre2_dfa_match.c,pcre2_extuni.c,pcre2_match.c | extuni.rs |
| 21 | `_pcre2_find_bracket_8` | T | T | YES | pcre2_compile.c,pcre2_find_bracket.c,pcre2_study.c | find_bracket.rs |
| 22 | `_pcre2_hspace_list_8` | R | R | YES | pcre2_tables.c | tables.rs |
| 23 | `_pcre2_is_newline_8` | T | T | YES | pcre2_newline.c | newline.rs |
| 24 | `_pcre2_jit_free_8` | T | T | YES | pcre2_compile.c,pcre2_jit_misc_inc.h | jit.rs |
| 25 | `_pcre2_jit_free_rodata_8` | T | T | YES | pcre2_jit_compile.c,pcre2_jit_misc_inc.h | jit.rs |
| 26 | `_pcre2_jit_get_size_8` | T | T | YES | pcre2_jit_misc_inc.h,pcre2_pattern_info.c | jit.rs |
| 27 | `_pcre2_jit_get_target_8` | T | T | YES | pcre2_config.c,pcre2_jit_misc_inc.h | jit.rs |
| 28 | `_pcre2_memctl_malloc_8` | T | T | YES | pcre2_context.c,pcre2_convert.c,pcre2_jit_misc_inc.h | context.rs |
| 29 | `_pcre2_ord2utf_8` | T | T | YES | pcre2_compile.c,pcre2_compile_class.c,pcre2_jit_compile.c | ord2utf.rs |
| 30 | `_pcre2_posix_class_maps8` | R | R | YES | pcre2_compile.c,pcre2_compile_class.c | compile_tables.rs |
| 31 | `_pcre2_script_run_8` | T | T | YES | pcre2_jit_compile.c,pcre2_match.c,pcre2_script_run.c | script_run.rs |
| 32 | `_pcre2_strcmp_8` | T | T | YES | pcre2_jit_compile.c,pcre2_match.c,pcre2_string_utils.c | string_utils.rs |
| 33 | `_pcre2_strcmp_c8_8` | T | T | YES | pcre2_compile.c,pcre2_string_utils.c,pcre2_substitute.c | string_utils.rs |
| 34 | `_pcre2_strcpy_c8_8` | T | T | YES | pcre2_config.c,pcre2_string_utils.c | string_utils.rs |
| 35 | `_pcre2_strlen_8` | T | T | YES | pcre2_compile.c,pcre2_config.c,pcre2_convert.c | string_utils.rs |
| 36 | `_pcre2_strncmp_8` | T | T | YES | pcre2_compile.c,pcre2_compile_cgroup.c,pcre2_string_utils.c | string_utils.rs |
| 37 | `_pcre2_strncmp_c8_8` | T | T | YES | pcre2_compile.c,pcre2_string_utils.c | string_utils.rs |
| 38 | `_pcre2_study_8` | T | T | YES | pcre2_compile.c,pcre2_study.c | study.rs |
| 39 | `_pcre2_ucd_boolprop_sets_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 40 | `_pcre2_ucd_caseless_sets_8` | R | R | YES | pcre2_compile.c,pcre2_compile_class.c,pcre2_ucd.c | ucd.rs |
| 41 | `_pcre2_ucd_digit_sets_8` | R | R | YES | pcre2_script_run.c,pcre2_ucd.c | ucd.rs |
| 42 | `_pcre2_ucd_nocase_ranges_8` | R | R | YES | pcre2_compile_class.c,pcre2_ucd.c | ucd.rs |
| 43 | `_pcre2_ucd_nocase_ranges_size_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 44 | `_pcre2_ucd_records_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 45 | `_pcre2_ucd_script_sets_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 46 | `_pcre2_ucd_stage1_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 47 | `_pcre2_ucd_stage2_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 48 | `_pcre2_ucd_turkish_dotted_i_caseset_8` | R | R | YES | pcre2_ucd.c | ucd.rs |
| 49 | `_pcre2_ucp_gbtable_8` | R | R | YES | pcre2_extuni.c,pcre2_jit_char_inc.h,pcre2_match.c | tables.rs |
| 50 | `_pcre2_ucp_gentype_8` | R | R | YES | pcre2_auto_possess.c,pcre2_compile.c,pcre2_compile_class.c | tables.rs |
| 51 | `_pcre2_unicode_version_8` | D | D | YES | pcre2_ucd.c | ucd.rs |
| 52 | `_pcre2_update_classbits_8` | T | T | YES | pcre2_compile_class.c | compile_class.rs |
| 53 | `_pcre2_utf8_table1` | R | R | YES | pcre2_ord2utf.c,pcre2_tables.c | tables.rs |
| 54 | `_pcre2_utf8_table1_size` | R | R | YES | pcre2_tables.c | tables.rs |
| 55 | `_pcre2_utf8_table2` | R | R | YES | pcre2_ord2utf.c,pcre2_tables.c | tables.rs |
| 56 | `_pcre2_utf8_table3` | R | R | YES | pcre2_printint_inc.h,pcre2_tables.c | tables.rs |
| 57 | `_pcre2_utf8_table4` | R | R | YES | pcre2_printint_inc.h,pcre2_tables.c,pcre2_valid_utf.c | tables.rs |
| 58 | `_pcre2_utt_8` | R | R | YES | pcre2_compile.c,pcre2_ucptables_inc.h | tables.rs |
| 59 | `_pcre2_utt_names_8` | R | R | YES | pcre2_ucptables_inc.h | tables.rs |
| 60 | `_pcre2_utt_size_8` | R | R | YES | pcre2_tables.c | tables.rs |
| 61 | `_pcre2_valid_utf_8` | T | T | YES | pcre2_compile.c,pcre2_convert.c,pcre2_dfa_match.c | valid_utf.rs |
| 62 | `_pcre2_vspace_list_8` | R | R | YES | pcre2_tables.c | tables.rs |
| 63 | `_pcre2_was_newline_8` | T | T | YES | pcre2_newline.c | newline.rs |
| 64 | `_pcre2_xclass_8` | T | T | YES | pcre2_auto_possess.c,pcre2_dfa_match.c,pcre2_match.c | xclass.rs |
| 65 | `pcre2_callout_enumerate_8` | T | T | YES | pcre2_pattern_info.c | pattern_info.rs |
| 66 | `pcre2_code_copy_8` | T | T | YES | pcre2_compile.c | compile.rs |
| 67 | `pcre2_code_copy_with_tables_8` | T | T | YES | pcre2_compile.c | compile.rs |
| 68 | `pcre2_code_free_8` | T | T | YES | pcre2_compile.c | compile.rs |
| 69 | `pcre2_compile_8` | T | T | YES | pcre2_compile.c,pcre2_context.c,pcre2_dfa_match.c | compile_main.rs |
| 70 | `pcre2_compile_context_copy_8` | T | T | YES | pcre2_context.c | context.rs |
| 71 | `pcre2_compile_context_create_8` | T | T | YES | pcre2_context.c | context.rs |
| 72 | `pcre2_compile_context_free_8` | T | T | YES | pcre2_context.c | context.rs |
| 73 | `pcre2_config_8` | T | T | YES | pcre2_config.c | config.rs |
| 74 | `pcre2_convert_context_copy_8` | T | T | YES | pcre2_context.c | context.rs |
| 75 | `pcre2_convert_context_create_8` | T | T | YES | pcre2_context.c | context.rs |
| 76 | `pcre2_convert_context_free_8` | T | T | YES | pcre2_context.c | context.rs |
| 77 | `pcre2_converted_pattern_free_8` | T | T | YES | pcre2_convert.c | convert.rs |
| 78 | `pcre2_dfa_match_8` | T | T | YES | pcre2_dfa_match.c,pcre2_extuni.c,pcre2_substring.c | dfa_match.rs |
| 79 | `pcre2_general_context_copy_8` | T | T | YES | pcre2_context.c | context.rs |
| 80 | `pcre2_general_context_create_8` | T | T | YES | pcre2_context.c,pcre2_substitute.c | context.rs |
| 81 | `pcre2_general_context_free_8` | T | T | YES | pcre2_context.c | context.rs |
| 82 | `pcre2_get_error_message_8` | T | T | YES | pcre2_error.c | error.rs |
| 83 | `pcre2_get_mark_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 84 | `pcre2_get_match_data_heapframes_size_8` | T | T | YES | pcre2_match_data.c | match_data.rs |
| 85 | `pcre2_get_match_data_size_8` | T | T | YES | pcre2_match_data.c | match_data.rs |
| 86 | `pcre2_get_ovector_count_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 87 | `pcre2_get_ovector_pointer_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 88 | `pcre2_get_startchar_8` | T | T | YES | pcre2_match_data.c | match_data.rs |
| 89 | `pcre2_jit_compile_8` | T | T | YES | pcre2_jit_compile.c | jit.rs |
| 90 | `pcre2_jit_free_unused_memory_8` | T | T | YES | pcre2_jit_misc_inc.h | jit.rs |
| 91 | `pcre2_jit_match_8` | T | T | YES | pcre2_jit_match_inc.h,pcre2_match.c | jit.rs |
| 92 | `pcre2_jit_stack_assign_8` | T | T | YES | pcre2_jit_misc_inc.h | jit.rs |
| 93 | `pcre2_jit_stack_create_8` | T | T | YES | pcre2_jit_misc_inc.h | jit.rs |
| 94 | `pcre2_jit_stack_free_8` | T | T | YES | pcre2_jit_misc_inc.h | jit.rs |
| 95 | `pcre2_maketables_8` | T | T | YES | pcre2_maketables.c | maketables.rs |
| 96 | `pcre2_maketables_free_8` | T | T | YES | pcre2_maketables.c | maketables.rs |
| 97 | `pcre2_match_8` | T | T | YES | pcre2_compile.c,pcre2_config.c,pcre2_extuni.c | matcher.rs |
| 98 | `pcre2_match_context_copy_8` | T | T | YES | pcre2_context.c | context.rs |
| 99 | `pcre2_match_context_create_8` | T | T | YES | pcre2_context.c | context.rs |
| 100 | `pcre2_match_context_free_8` | T | T | YES | pcre2_context.c | context.rs |
| 101 | `pcre2_match_data_create_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 102 | `pcre2_match_data_create_from_pattern_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 103 | `pcre2_match_data_free_8` | T | T | YES | pcre2_match_data.c,pcre2_substitute.c | match_data.rs |
| 104 | `pcre2_next_match_8` | T | T | YES | pcre2_match_next.c,pcre2_substitute.c | match_next.rs |
| 105 | `pcre2_pattern_convert_8` | T | T | YES | pcre2_convert.c | convert.rs |
| 106 | `pcre2_pattern_info_8` | T | T | YES | pcre2_pattern_info.c | pattern_info.rs |
| 107 | `pcre2_serialize_decode_8` | T | T | YES | pcre2_serialize.c | serialize.rs |
| 108 | `pcre2_serialize_encode_8` | T | T | YES | pcre2_compile_class.c,pcre2_serialize.c | serialize.rs |
| 109 | `pcre2_serialize_free_8` | T | T | YES | pcre2_serialize.c | serialize.rs |
| 110 | `pcre2_serialize_get_number_of_codes_8` | T | T | YES | pcre2_serialize.c | serialize.rs |
| 111 | `pcre2_set_bsr_8` | T | T | YES | pcre2_context.c | context.rs |
| 112 | `pcre2_set_callout_8` | T | T | YES | pcre2_context.c | context.rs |
| 113 | `pcre2_set_character_tables_8` | T | T | YES | pcre2_context.c | context.rs |
| 114 | `pcre2_set_compile_extra_options_8` | T | T | YES | pcre2_context.c | context.rs |
| 115 | `pcre2_set_compile_recursion_guard_8` | T | T | YES | pcre2_context.c | context.rs |
| 116 | `pcre2_set_depth_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 117 | `pcre2_set_glob_escape_8` | T | T | YES | pcre2_context.c | context.rs |
| 118 | `pcre2_set_glob_separator_8` | T | T | YES | pcre2_context.c | context.rs |
| 119 | `pcre2_set_heap_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 120 | `pcre2_set_match_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 121 | `pcre2_set_max_pattern_compiled_length_8` | T | T | YES | pcre2_context.c | context.rs |
| 122 | `pcre2_set_max_pattern_length_8` | T | T | YES | pcre2_context.c | context.rs |
| 123 | `pcre2_set_max_varlookbehind_8` | T | T | YES | pcre2_context.c | context.rs |
| 124 | `pcre2_set_newline_8` | T | T | YES | pcre2_context.c | context.rs |
| 125 | `pcre2_set_offset_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 126 | `pcre2_set_optimize_8` | T | T | YES | pcre2_context.c | context.rs |
| 127 | `pcre2_set_parens_nest_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 128 | `pcre2_set_recursion_limit_8` | T | T | YES | pcre2_context.c | context.rs |
| 129 | `pcre2_set_recursion_memory_management_8` | T | T | YES | pcre2_context.c | context.rs |
| 130 | `pcre2_set_substitute_callout_8` | T | T | YES | pcre2_context.c | context.rs |
| 131 | `pcre2_set_substitute_case_callout_8` | T | T | YES | pcre2_context.c | context.rs |
| 132 | `pcre2_substitute_8` | T | T | YES | pcre2_compile.c,pcre2_substitute.c | substitute.rs |
| 133 | `pcre2_substring_copy_byname_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 134 | `pcre2_substring_copy_bynumber_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 135 | `pcre2_substring_free_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 136 | `pcre2_substring_get_byname_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 137 | `pcre2_substring_get_bynumber_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 138 | `pcre2_substring_length_byname_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 139 | `pcre2_substring_length_bynumber_8` | T | T | YES | pcre2_substitute.c,pcre2_substring.c | substring.rs |
| 140 | `pcre2_substring_list_free_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 141 | `pcre2_substring_list_get_8` | T | T | YES | pcre2_substring.c | substring.rs |
| 142 | `pcre2_substring_nametable_scan_8` | T | T | YES | pcre2_substitute.c,pcre2_substring.c | substring.rs |
| 143 | `pcre2_substring_number_from_name_8` | T | T | YES | pcre2_substring.c | substring.rs |

## Result

**PASS** — the symbol diff is empty in both directions: every one of the 143 dynamic
symbols exported by the C `.so` (including all the macro-generated `PRIV(x)` →
`_pcre2_x_8` names) is exported by the Rust `.so` under the exact same name and the
same nm kind (`T` for functions, `R`/`D` for data). There are no extra Rust exports
and no Rust-side stubs: every symbol is backed by a real translation of the C body.

Undefined (imported) symbols in the Rust `.so` are libc/`libgcc`/Rust-runtime only
(`malloc`, `free`, `memcpy`, `memmove`, `memset`, `realloc`, `calloc`,
`posix_memalign`, `bcmp`, `strlen`, `tolower`, `toupper`, the `is*` ctype family,
`abort`, `__errno_location`, `_Unwind_*`, `__cxa_*`, `pthread_*`, and the
std-panic-machinery syscalls) — zero undefined non-libc symbols.
