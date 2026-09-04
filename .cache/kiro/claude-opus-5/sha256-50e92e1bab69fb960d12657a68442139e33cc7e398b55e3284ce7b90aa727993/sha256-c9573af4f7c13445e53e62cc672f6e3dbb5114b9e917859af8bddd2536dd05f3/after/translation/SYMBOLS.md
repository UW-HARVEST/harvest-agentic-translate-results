# SYMBOLS.md — exported-symbol parity (C `libpcre2.so` vs Rust `libpcre2.so`)

Generated mechanically by `nm -D --defined-only` on both shared objects
(regenerate with `./verify_all.sh`).

```
C    .so: c_src/build/libpcre2.so
Rust .so: translation/target/release/libpcre2.so
C exports:         143
Rust exports:      143
MISSING from Rust: 0
EXTRA in Rust:     0
```

**Symbol diff is empty**: every symbol the C `.so` exports is exported by the
Rust `.so` under the exact same name (including the macro-generated `_8`
suffixed names and the `_pcre2_*` private exports), and the Rust `.so` exports
nothing extra.

## Undefined (imported) symbols in the Rust .so

```
_ITM_deregisterTMCloneTable _ITM_registerTMCloneTable _Unwind_Backtrace@GCC_3.3 _Unwind_GetDataRelBase@GCC_3.0 _Unwind_GetIP@GCC_3.0 _Unwind_GetIPInfo@GCC_4.2.0 _Unwind_GetLanguageSpecificData@GCC_3.0 _Unwind_GetRegionStart@GCC_3.0 _Unwind_GetTextRelBase@GCC_3.0 _Unwind_Resume@GCC_3.0 _Unwind_SetGR@GCC_3.0 _Unwind_SetIP@GCC_3.0 __cxa_finalize@GLIBC_2.2.5 __cxa_thread_atexit_impl@GLIBC_2.18 __errno_location@GLIBC_2.2.5 __gmon_start__ __tls_get_addr@GLIBC_2.3 abort@GLIBC_2.2.5 bcmp@GLIBC_2.2.5 calloc@GLIBC_2.2.5 close@GLIBC_2.2.5 dl_iterate_phdr@GLIBC_2.2.5 free@GLIBC_2.2.5 fstat64@GLIBC_2.33 getcwd@GLIBC_2.2.5 getenv@GLIBC_2.2.5 gettid@GLIBC_2.30 isalnum@GLIBC_2.2.5 isalpha@GLIBC_2.2.5 iscntrl@GLIBC_2.2.5 isgraph@GLIBC_2.2.5 islower@GLIBC_2.2.5 isprint@GLIBC_2.2.5 ispunct@GLIBC_2.2.5 isspace@GLIBC_2.2.5 isupper@GLIBC_2.2.5 isxdigit@GLIBC_2.2.5 lseek64@GLIBC_2.2.5 malloc@GLIBC_2.2.5 memchr@GLIBC_2.2.5 memcpy@GLIBC_2.14 memmove@GLIBC_2.2.5 memset@GLIBC_2.2.5 mmap64@GLIBC_2.2.5 munmap@GLIBC_2.2.5 open64@GLIBC_2.2.5 posix_memalign@GLIBC_2.2.5 pthread_key_create@GLIBC_2.34 pthread_key_delete@GLIBC_2.34 pthread_setspecific@GLIBC_2.34 read@GLIBC_2.2.5 readlink@GLIBC_2.2.5 realloc@GLIBC_2.2.5 realpath@GLIBC_2.3 stat64@GLIBC_2.33 statx@GLIBC_2.28 strlen@GLIBC_2.2.5 syscall@GLIBC_2.2.5 tolower@GLIBC_2.2.5 toupper@GLIBC_2.2.5 write@GLIBC_2.2.5 writev@GLIBC_2.2.5 
```

Non-libc / non-compiler-runtime unresolved symbols: **0**
(everything above resolves to glibc, the GCC unwinder or the ELF/TM stubs).

## Symbol table

Sizes are the ELF `st_size` values; for data objects the whole extent is
compared byte-for-byte by `tests/t01_primitives.rs::data_tables_identical`.

| # | symbol | C type/size | Rust type/size | in both |
|---|--------|-------------|----------------|---------|
| 1 | `_pcre2_OP_lengths_8` | R / 0x00000000000000ad | R / 0x00000000000000ad | yes |
| 2 | `_pcre2_auto_possessify_8` | T / 0x0000000000000534 | T / 0x0000000000000343 | yes |
| 3 | `_pcre2_callout_end_delims_8` | R / 0x0000000000000024 | R / 0x0000000000000024 | yes |
| 4 | `_pcre2_callout_start_delims_8` | R / 0x0000000000000024 | R / 0x0000000000000024 | yes |
| 5 | `_pcre2_check_escape_8` | T / 0x00000000000012f7 | T / 0x0000000000001204 | yes |
| 6 | `_pcre2_ckd_smul_8` | T / 0x0000000000000033 | T / 0x0000000000000010 | yes |
| 7 | `_pcre2_compile_add_name_to_table8` | T / 0x0000000000000216 | T / 0x00000000000002b9 | yes |
| 8 | `_pcre2_compile_class_nested_8` | T / 0x000000000000064e | T / 0x00000000000002c3 | yes |
| 9 | `_pcre2_compile_class_not_nested_8` | T / 0x000000000000129a | T / 0x0000000000002515 | yes |
| 10 | `_pcre2_compile_find_dupname_details8` | T / 0x00000000000001cd | T / 0x00000000000001db | yes |
| 11 | `_pcre2_compile_find_named_group8` | T / 0x00000000000000be | T / 0x00000000000000b8 | yes |
| 12 | `_pcre2_compile_get_hash_from_name8` | T / 0x000000000000003e | T / 0x0000000000000012 | yes |
| 13 | `_pcre2_compile_parse_recurse_args8` | T / 0x0000000000000377 | T / 0x00000000000003f9 | yes |
| 14 | `_pcre2_compile_parse_scan_substr_args8` | T / 0x00000000000002e4 | T / 0x0000000000000247 | yes |
| 15 | `_pcre2_default_compile_context_8` | D / 0x0000000000000058 | D / 0x0000000000000058 | yes |
| 16 | `_pcre2_default_convert_context_8` | D / 0x0000000000000020 | D / 0x0000000000000020 | yes |
| 17 | `_pcre2_default_match_context_8` | D / 0x0000000000000060 | D / 0x0000000000000060 | yes |
| 18 | `_pcre2_default_tables_8` | R / 0x0000000000000440 | R / 0x0000000000000440 | yes |
| 19 | `_pcre2_eclass_8` | T / 0x00000000000001ae | T / 0x0000000000000102 | yes |
| 20 | `_pcre2_extuni_8` | T / 0x00000000000006d2 | T / 0x0000000000000521 | yes |
| 21 | `_pcre2_find_bracket_8` | T / 0x0000000000000278 | T / 0x000000000000023b | yes |
| 22 | `_pcre2_hspace_list_8` | R / 0x0000000000000050 | R / 0x0000000000000050 | yes |
| 23 | `_pcre2_is_newline_8` | T / 0x0000000000000350 | T / 0x00000000000001a5 | yes |
| 24 | `_pcre2_jit_free_8` | T / 0x000000000000000f | T / 0x0000000000000001 | yes |
| 25 | `_pcre2_jit_free_rodata_8` | T / 0x000000000000000f | T / 0x0000000000000001 | yes |
| 26 | `_pcre2_jit_get_size_8` | T / 0x000000000000000f | T / 0x0000000000000003 | yes |
| 27 | `_pcre2_jit_get_target_8` | T / 0x000000000000000d | T / 0x0000000000000008 | yes |
| 28 | `_pcre2_memctl_malloc_8` | T / 0x00000000000000b0 | T / 0x0000000000000067 | yes |
| 29 | `_pcre2_ord2utf_8` | T / 0x0000000000000099 | T / 0x00000000000000bc | yes |
| 30 | `_pcre2_posix_class_maps8` | R / 0x00000000000000a8 | R / 0x00000000000000a8 | yes |
| 31 | `_pcre2_script_run_8` | T / 0x0000000000000975 | T / 0x00000000000006c7 | yes |
| 32 | `_pcre2_strcmp_8` | T / 0x000000000000006f | T / 0x0000000000000035 | yes |
| 33 | `_pcre2_strcmp_c8_8` | T / 0x000000000000006f | T / 0x0000000000000035 | yes |
| 34 | `_pcre2_strcpy_c8_8` | T / 0x0000000000000051 | T / 0x0000000000000026 | yes |
| 35 | `_pcre2_strlen_8` | T / 0x0000000000000030 | T / 0x000000000000001a | yes |
| 36 | `_pcre2_strncmp_8` | T / 0x0000000000000069 | T / 0x0000000000000035 | yes |
| 37 | `_pcre2_strncmp_c8_8` | T / 0x0000000000000069 | T / 0x0000000000000035 | yes |
| 38 | `_pcre2_study_8` | T / 0x0000000000000479 | T / 0x00000000000004d3 | yes |
| 39 | `_pcre2_ucd_boolprop_sets_8` | R / 0x00000000000005f8 | R / 0x00000000000005f8 | yes |
| 40 | `_pcre2_ucd_caseless_sets_8` | R / 0x00000000000001d8 | R / 0x00000000000001d8 | yes |
| 41 | `_pcre2_ucd_digit_sets_8` | R / 0x0000000000000138 | R / 0x0000000000000138 | yes |
| 42 | `_pcre2_ucd_nocase_ranges_8` | R / 0x0000000000000150 | R / 0x0000000000000150 | yes |
| 43 | `_pcre2_ucd_nocase_ranges_size_8` | R / 0x0000000000000004 | R / 0x0000000000000004 | yes |
| 44 | `_pcre2_ucd_records_8` | R / 0x0000000000004944 | R / 0x0000000000004944 | yes |
| 45 | `_pcre2_ucd_script_sets_8` | R / 0x0000000000000770 | R / 0x0000000000000770 | yes |
| 46 | `_pcre2_ucd_stage1_8` | R / 0x0000000000004400 | R / 0x0000000000004400 | yes |
| 47 | `_pcre2_ucd_stage2_8` | R / 0x0000000000013a00 | R / 0x0000000000013a00 | yes |
| 48 | `_pcre2_ucd_turkish_dotted_i_caseset_8` | R / 0x0000000000000004 | R / 0x0000000000000004 | yes |
| 49 | `_pcre2_ucp_gbtable_8` | R / 0x000000000000003c | R / 0x000000000000003c | yes |
| 50 | `_pcre2_ucp_gentype_8` | R / 0x0000000000000078 | R / 0x0000000000000078 | yes |
| 51 | `_pcre2_unicode_version_8` | D / 0x0000000000000008 | D / 0x0000000000000008 | yes |
| 52 | `_pcre2_update_classbits_8` | T / 0x00000000000004da | T / 0x000000000000049a | yes |
| 53 | `_pcre2_utf8_table1` | R / 0x0000000000000018 | R / 0x0000000000000018 | yes |
| 54 | `_pcre2_utf8_table1_size` | R / 0x0000000000000004 | R / 0x0000000000000004 | yes |
| 55 | `_pcre2_utf8_table2` | R / 0x0000000000000018 | R / 0x0000000000000018 | yes |
| 56 | `_pcre2_utf8_table3` | R / 0x0000000000000018 | R / 0x0000000000000018 | yes |
| 57 | `_pcre2_utf8_table4` | R / 0x0000000000000040 | R / 0x0000000000000040 | yes |
| 58 | `_pcre2_utt_8` | R / 0x0000000000000c24 | R / 0x0000000000000c24 | yes |
| 59 | `_pcre2_utt_names_8` | R / 0x0000000000000efa | R / 0x0000000000000efa | yes |
| 60 | `_pcre2_utt_size_8` | R / 0x0000000000000008 | R / 0x0000000000000008 | yes |
| 61 | `_pcre2_valid_utf_8` | T / 0x0000000000000578 | T / 0x0000000000000379 | yes |
| 62 | `_pcre2_vspace_list_8` | R / 0x0000000000000020 | R / 0x0000000000000020 | yes |
| 63 | `_pcre2_was_newline_8` | T / 0x0000000000000371 | T / 0x00000000000001c0 | yes |
| 64 | `_pcre2_xclass_8` | T / 0x0000000000000fe9 | T / 0x0000000000000aad | yes |
| 65 | `pcre2_callout_enumerate_8` | T / 0x0000000000000451 | T / 0x000000000000025c | yes |
| 66 | `pcre2_code_copy_8` | T / 0x00000000000000af | T / 0x0000000000000076 | yes |
| 67 | `pcre2_code_copy_with_tables_8` | T / 0x000000000000011b | T / 0x00000000000000bf | yes |
| 68 | `pcre2_code_free_8` | T / 0x00000000000000a2 | T / 0x0000000000000070 | yes |
| 69 | `pcre2_compile_8` | T / 0x0000000000001a71 | T / 0x0000000000001d72 | yes |
| 70 | `pcre2_compile_context_copy_8` | T / 0x0000000000000055 | T / 0x0000000000000059 | yes |
| 71 | `pcre2_compile_context_create_8` | T / 0x00000000000000c3 | T / 0x00000000000000d4 | yes |
| 72 | `pcre2_compile_context_free_8` | T / 0x0000000000000032 | T / 0x0000000000000023 | yes |
| 73 | `pcre2_config_8` | T / 0x0000000000000206 | T / 0x00000000000000a1 | yes |
| 74 | `pcre2_convert_context_copy_8` | T / 0x0000000000000055 | T / 0x0000000000000039 | yes |
| 75 | `pcre2_convert_context_create_8` | T / 0x0000000000000083 | T / 0x000000000000008c | yes |
| 76 | `pcre2_convert_context_free_8` | T / 0x0000000000000032 | T / 0x0000000000000023 | yes |
| 77 | `pcre2_converted_pattern_free_8` | T / 0x000000000000003e | T / 0x0000000000000027 | yes |
| 78 | `pcre2_dfa_match_8` | T / 0x0000000000001a92 | T / 0x000000000000111b | yes |
| 79 | `pcre2_general_context_copy_8` | T / 0x0000000000000055 | T / 0x0000000000000039 | yes |
| 80 | `pcre2_general_context_create_8` | T / 0x0000000000000085 | T / 0x0000000000000048 | yes |
| 81 | `pcre2_general_context_free_8` | T / 0x0000000000000032 | T / 0x0000000000000023 | yes |
| 82 | `pcre2_get_error_message_8` | T / 0x000000000000010e | T / 0x0000000000000136 | yes |
| 83 | `pcre2_get_mark_8` | T / 0x0000000000000012 | T / 0x0000000000000005 | yes |
| 84 | `pcre2_get_match_data_heapframes_size_8` | T / 0x0000000000000012 | T / 0x0000000000000005 | yes |
| 85 | `pcre2_get_match_data_size_8` | T / 0x0000000000000021 | T / 0x000000000000000c | yes |
| 86 | `pcre2_get_ovector_count_8` | T / 0x0000000000000015 | T / 0x0000000000000005 | yes |
| 87 | `pcre2_get_ovector_pointer_8` | T / 0x0000000000000012 | T / 0x0000000000000005 | yes |
| 88 | `pcre2_get_startchar_8` | T / 0x0000000000000012 | T / 0x0000000000000005 | yes |
| 89 | `pcre2_jit_compile_8` | T / 0x000000000000008f | T / 0x0000000000000057 | yes |
| 90 | `pcre2_jit_free_unused_memory_8` | T / 0x000000000000000b | T / 0x0000000000000001 | yes |
| 91 | `pcre2_jit_match_8` | T / 0x0000000000000030 | T / 0x000000000000000e | yes |
| 92 | `pcre2_jit_stack_assign_8` | T / 0x0000000000000013 | T / 0x0000000000000001 | yes |
| 93 | `pcre2_jit_stack_create_8` | T / 0x0000000000000017 | T / 0x0000000000000003 | yes |
| 94 | `pcre2_jit_stack_free_8` | T / 0x000000000000000b | T / 0x0000000000000001 | yes |
| 95 | `pcre2_maketables_8` | T / 0x000000000000069b | T / 0x0000000000000371 | yes |
| 96 | `pcre2_maketables_free_8` | T / 0x0000000000000044 | T / 0x0000000000000031 | yes |
| 97 | `pcre2_match_8` | T / 0x000000000000235a | T / 0x00000000000017b4 | yes |
| 98 | `pcre2_match_context_copy_8` | T / 0x0000000000000055 | T / 0x0000000000000059 | yes |
| 99 | `pcre2_match_context_create_8` | T / 0x00000000000000cb | T / 0x00000000000000d4 | yes |
| 100 | `pcre2_match_context_free_8` | T / 0x0000000000000032 | T / 0x0000000000000023 | yes |
| 101 | `pcre2_match_data_create_8` | T / 0x0000000000000093 | T / 0x000000000000009e | yes |
| 102 | `pcre2_match_data_create_from_pattern_8` | T / 0x0000000000000050 | T / 0x000000000000007b | yes |
| 103 | `pcre2_match_data_free_8` | T / 0x0000000000000091 | T / 0x0000000000000076 | yes |
| 104 | `pcre2_next_match_8` | T / 0x0000000000000120 | T / 0x00000000000000d6 | yes |
| 105 | `pcre2_pattern_convert_8` | T / 0x0000000000000374 | T / 0x00000000000024a0 | yes |
| 106 | `pcre2_pattern_info_8` | T / 0x0000000000000438 | T / 0x00000000000001bc | yes |
| 107 | `pcre2_serialize_decode_8` | T / 0x0000000000000312 | T / 0x0000000000000342 | yes |
| 108 | `pcre2_serialize_encode_8` | T / 0x0000000000000292 | T / 0x0000000000000238 | yes |
| 109 | `pcre2_serialize_free_8` | T / 0x000000000000003e | T / 0x0000000000000027 | yes |
| 110 | `pcre2_serialize_get_number_of_codes_8` | T / 0x0000000000000065 | T / 0x0000000000000033 | yes |
| 111 | `pcre2_set_bsr_8` | T / 0x0000000000000031 | T / 0x0000000000000014 | yes |
| 112 | `pcre2_set_callout_8` | T / 0x000000000000002f | T / 0x000000000000000b | yes |
| 113 | `pcre2_set_character_tables_8` | T / 0x000000000000001f | T / 0x0000000000000007 | yes |
| 114 | `pcre2_set_compile_extra_options_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 115 | `pcre2_set_compile_recursion_guard_8` | T / 0x000000000000002f | T / 0x000000000000000b | yes |
| 116 | `pcre2_set_depth_limit_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 117 | `pcre2_set_glob_escape_8` | T / 0x000000000000004f | T / 0x000000000000002f | yes |
| 118 | `pcre2_set_glob_separator_8` | T / 0x0000000000000035 | T / 0x0000000000000023 | yes |
| 119 | `pcre2_set_heap_limit_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 120 | `pcre2_set_match_limit_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 121 | `pcre2_set_max_pattern_compiled_length_8` | T / 0x000000000000001f | T / 0x0000000000000007 | yes |
| 122 | `pcre2_set_max_pattern_length_8` | T / 0x000000000000001f | T / 0x0000000000000007 | yes |
| 123 | `pcre2_set_max_varlookbehind_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 124 | `pcre2_set_newline_8` | T / 0x0000000000000031 | T / 0x0000000000000014 | yes |
| 125 | `pcre2_set_offset_limit_8` | T / 0x000000000000001f | T / 0x0000000000000007 | yes |
| 126 | `pcre2_set_optimize_8` | T / 0x00000000000000b9 | T / 0x0000000000000052 | yes |
| 127 | `pcre2_set_parens_nest_limit_8` | T / 0x000000000000001c | T / 0x0000000000000006 | yes |
| 128 | `pcre2_set_recursion_limit_8` | T / 0x0000000000000022 | T / 0x0000000000000006 | yes |
| 129 | `pcre2_set_recursion_memory_management_8` | T / 0x000000000000001b | T / 0x0000000000000003 | yes |
| 130 | `pcre2_set_substitute_callout_8` | T / 0x000000000000002f | T / 0x000000000000000b | yes |
| 131 | `pcre2_set_substitute_case_callout_8` | T / 0x000000000000002f | T / 0x000000000000000b | yes |
| 132 | `pcre2_substitute_8` | T / 0x0000000000002b8f | T / 0x0000000000002c80 | yes |
| 133 | `pcre2_substring_copy_byname_8` | T / 0x00000000000000f1 | T / 0x00000000000002ad | yes |
| 134 | `pcre2_substring_copy_bynumber_8` | T / 0x00000000000000b2 | T / 0x000000000000012e | yes |
| 135 | `pcre2_substring_free_8` | T / 0x000000000000003e | T / 0x0000000000000027 | yes |
| 136 | `pcre2_substring_get_byname_8` | T / 0x00000000000000f1 | T / 0x000000000000023a | yes |
| 137 | `pcre2_substring_get_bynumber_8` | T / 0x00000000000000db | T / 0x000000000000018a | yes |
| 138 | `pcre2_substring_length_byname_8` | T / 0x00000000000000eb | T / 0x0000000000000280 | yes |
| 139 | `pcre2_substring_length_bynumber_8` | T / 0x0000000000000169 | T / 0x00000000000000d4 | yes |
| 140 | `pcre2_substring_list_free_8` | T / 0x000000000000003e | T / 0x0000000000000027 | yes |
| 141 | `pcre2_substring_list_get_8` | T / 0x00000000000002bc | T / 0x00000000000002bb | yes |
| 142 | `pcre2_substring_nametable_scan_8` | T / 0x00000000000001d7 | T / 0x00000000000001b9 | yes |
| 143 | `pcre2_substring_number_from_name_8` | T / 0x000000000000002c | T / 0x00000000000001bc | yes |
