# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically by `tools/gen_symbols.sh` from `nm -D` on both shared objects.

- C   `c_src/build/libpcre2.so`            : **143** dynamic defined symbols
- Rust `translation/target/release/libpcre2.so`: **143** dynamic defined symbols
- Missing from Rust: **0**
- Extra in Rust (not in C): **0**
- Undefined non-libc symbols in Rust .so: **0**

Build config: `PCRE2_CODE_UNIT_WIDTH=8`, `SUPPORT_UNICODE`, `HAVE_CONFIG_H` (see c_src/CMakeLists.txt).
The crate declares no cargo features, so there is exactly one feature combination
(default == `--no-default-features`); Phase D feature-combo sweep is therefore a single cell.

## Full symbol table

`T`/`t` = code, `R`/`D`/`B` = data. `kind` column is the C .so binding letter.

| # | symbol | kind | origin C file | in Rust .so |
|---|--------|------|---------------|-------------|
| 1 | `_pcre2_OP_lengths_8` | R | pcre2_tables.c | yes |
| 2 | `_pcre2_auto_possessify_8` | T | pcre2_auto_possess.c | yes |
| 3 | `_pcre2_callout_end_delims_8` | R | pcre2_tables.c | yes |
| 4 | `_pcre2_callout_start_delims_8` | R | pcre2_tables.c | yes |
| 5 | `_pcre2_check_escape_8` | T | pcre2_compile.c | yes |
| 6 | `_pcre2_ckd_smul_8` | T | pcre2_chkdint.c | yes |
| 7 | `_pcre2_compile_add_name_to_table8` | T | pcre2_compile_cgroup.c | yes |
| 8 | `_pcre2_compile_class_nested_8` | T | pcre2_compile_class.c | yes |
| 9 | `_pcre2_compile_class_not_nested_8` | T | pcre2_compile_class.c | yes |
| 10 | `_pcre2_compile_find_dupname_details8` | T | pcre2_compile_cgroup.c | yes |
| 11 | `_pcre2_compile_find_named_group8` | T | pcre2_compile_cgroup.c | yes |
| 12 | `_pcre2_compile_get_hash_from_name8` | T | pcre2_compile_cgroup.c | yes |
| 13 | `_pcre2_compile_parse_recurse_args8` | T | pcre2_compile_cgroup.c | yes |
| 14 | `_pcre2_compile_parse_scan_substr_args8` | T | pcre2_compile_cgroup.c | yes |
| 15 | `_pcre2_default_compile_context_8` | D | pcre2_context.c | yes |
| 16 | `_pcre2_default_convert_context_8` | D | pcre2_context.c | yes |
| 17 | `_pcre2_default_match_context_8` | D | pcre2_context.c | yes |
| 18 | `_pcre2_default_tables_8` | R | pcre2_chartables.c | yes |
| 19 | `_pcre2_eclass_8` | T | pcre2_xclass.c | yes |
| 20 | `_pcre2_extuni_8` | T | pcre2_extuni.c | yes |
| 21 | `_pcre2_find_bracket_8` | T | pcre2_find_bracket.c | yes |
| 22 | `_pcre2_hspace_list_8` | R | pcre2_tables.c | yes |
| 23 | `_pcre2_is_newline_8` | T | pcre2_newline.c | yes |
| 24 | `_pcre2_jit_free_8` | T | pcre2_jit_compile.c | yes |
| 25 | `_pcre2_jit_free_rodata_8` | T | pcre2_jit_compile.c | yes |
| 26 | `_pcre2_jit_get_size_8` | T | pcre2_jit_compile.c | yes |
| 27 | `_pcre2_jit_get_target_8` | T | pcre2_jit_compile.c | yes |
| 28 | `_pcre2_memctl_malloc_8` | T | pcre2_context.c | yes |
| 29 | `_pcre2_ord2utf_8` | T | pcre2_ord2utf.c | yes |
| 30 | `_pcre2_posix_class_maps8` | R | pcre2_compile.c | yes |
| 31 | `_pcre2_script_run_8` | T | pcre2_script_run.c | yes |
| 32 | `_pcre2_strcmp_8` | T | pcre2_string_utils.c | yes |
| 33 | `_pcre2_strcmp_c8_8` | T | pcre2_string_utils.c | yes |
| 34 | `_pcre2_strcpy_c8_8` | T | pcre2_string_utils.c | yes |
| 35 | `_pcre2_strlen_8` | T | pcre2_string_utils.c | yes |
| 36 | `_pcre2_strncmp_8` | T | pcre2_string_utils.c | yes |
| 37 | `_pcre2_strncmp_c8_8` | T | pcre2_string_utils.c | yes |
| 38 | `_pcre2_study_8` | T | pcre2_study.c | yes |
| 39 | `_pcre2_ucd_boolprop_sets_8` | R | pcre2_ucd.c | yes |
| 40 | `_pcre2_ucd_caseless_sets_8` | R | pcre2_ucd.c | yes |
| 41 | `_pcre2_ucd_digit_sets_8` | R | pcre2_ucd.c | yes |
| 42 | `_pcre2_ucd_nocase_ranges_8` | R | pcre2_ucd.c | yes |
| 43 | `_pcre2_ucd_nocase_ranges_size_8` | R | pcre2_ucd.c | yes |
| 44 | `_pcre2_ucd_records_8` | R | pcre2_ucd.c | yes |
| 45 | `_pcre2_ucd_script_sets_8` | R | pcre2_ucd.c | yes |
| 46 | `_pcre2_ucd_stage1_8` | R | pcre2_ucd.c | yes |
| 47 | `_pcre2_ucd_stage2_8` | R | pcre2_ucd.c | yes |
| 48 | `_pcre2_ucd_turkish_dotted_i_caseset_8` | R | pcre2_ucd.c | yes |
| 49 | `_pcre2_ucp_gbtable_8` | R | pcre2_tables.c | yes |
| 50 | `_pcre2_ucp_gentype_8` | R | pcre2_tables.c | yes |
| 51 | `_pcre2_unicode_version_8` | D | pcre2_ucd.c | yes |
| 52 | `_pcre2_update_classbits_8` | T | pcre2_compile_class.c | yes |
| 53 | `_pcre2_utf8_table1` | R | pcre2_tables.c | yes |
| 54 | `_pcre2_utf8_table1_size` | R | pcre2_tables.c | yes |
| 55 | `_pcre2_utf8_table2` | R | pcre2_tables.c | yes |
| 56 | `_pcre2_utf8_table3` | R | pcre2_tables.c | yes |
| 57 | `_pcre2_utf8_table4` | R | pcre2_tables.c | yes |
| 58 | `_pcre2_utt_8` | R | pcre2_tables.c | yes |
| 59 | `_pcre2_utt_names_8` | R | pcre2_tables.c | yes |
| 60 | `_pcre2_utt_size_8` | R | pcre2_tables.c | yes |
| 61 | `_pcre2_valid_utf_8` | T | pcre2_valid_utf.c | yes |
| 62 | `_pcre2_vspace_list_8` | R | pcre2_tables.c | yes |
| 63 | `_pcre2_was_newline_8` | T | pcre2_newline.c | yes |
| 64 | `_pcre2_xclass_8` | T | pcre2_xclass.c | yes |
| 65 | `pcre2_callout_enumerate_8` | T | pcre2_pattern_info.c | yes |
| 66 | `pcre2_code_copy_8` | T | pcre2_compile.c | yes |
| 67 | `pcre2_code_copy_with_tables_8` | T | pcre2_compile.c | yes |
| 68 | `pcre2_code_free_8` | T | pcre2_compile.c | yes |
| 69 | `pcre2_compile_8` | T | pcre2_compile.c | yes |
| 70 | `pcre2_compile_context_copy_8` | T | pcre2_context.c | yes |
| 71 | `pcre2_compile_context_create_8` | T | pcre2_context.c | yes |
| 72 | `pcre2_compile_context_free_8` | T | pcre2_context.c | yes |
| 73 | `pcre2_config_8` | T | pcre2_config.c | yes |
| 74 | `pcre2_convert_context_copy_8` | T | pcre2_context.c | yes |
| 75 | `pcre2_convert_context_create_8` | T | pcre2_context.c | yes |
| 76 | `pcre2_convert_context_free_8` | T | pcre2_context.c | yes |
| 77 | `pcre2_converted_pattern_free_8` | T | pcre2_convert.c | yes |
| 78 | `pcre2_dfa_match_8` | T | pcre2_dfa_match.c | yes |
| 79 | `pcre2_general_context_copy_8` | T | pcre2_context.c | yes |
| 80 | `pcre2_general_context_create_8` | T | pcre2_context.c | yes |
| 81 | `pcre2_general_context_free_8` | T | pcre2_context.c | yes |
| 82 | `pcre2_get_error_message_8` | T | pcre2_error.c | yes |
| 83 | `pcre2_get_mark_8` | T | pcre2_match_data.c | yes |
| 84 | `pcre2_get_match_data_heapframes_size_8` | T | pcre2_match_data.c | yes |
| 85 | `pcre2_get_match_data_size_8` | T | pcre2_match_data.c | yes |
| 86 | `pcre2_get_ovector_count_8` | T | pcre2_match_data.c | yes |
| 87 | `pcre2_get_ovector_pointer_8` | T | pcre2_match_data.c | yes |
| 88 | `pcre2_get_startchar_8` | T | pcre2_match_data.c | yes |
| 89 | `pcre2_jit_compile_8` | T | pcre2_jit_compile.c | yes |
| 90 | `pcre2_jit_free_unused_memory_8` | T | pcre2_jit_compile.c | yes |
| 91 | `pcre2_jit_match_8` | T | pcre2_jit_compile.c | yes |
| 92 | `pcre2_jit_stack_assign_8` | T | pcre2_jit_compile.c | yes |
| 93 | `pcre2_jit_stack_create_8` | T | pcre2_jit_compile.c | yes |
| 94 | `pcre2_jit_stack_free_8` | T | pcre2_jit_compile.c | yes |
| 95 | `pcre2_maketables_8` | T | pcre2_maketables.c | yes |
| 96 | `pcre2_maketables_free_8` | T | pcre2_maketables.c | yes |
| 97 | `pcre2_match_8` | T | pcre2_match.c | yes |
| 98 | `pcre2_match_context_copy_8` | T | pcre2_context.c | yes |
| 99 | `pcre2_match_context_create_8` | T | pcre2_context.c | yes |
| 100 | `pcre2_match_context_free_8` | T | pcre2_context.c | yes |
| 101 | `pcre2_match_data_create_8` | T | pcre2_match_data.c | yes |
| 102 | `pcre2_match_data_create_from_pattern_8` | T | pcre2_match_data.c | yes |
| 103 | `pcre2_match_data_free_8` | T | pcre2_match_data.c | yes |
| 104 | `pcre2_next_match_8` | T | pcre2_match_next.c | yes |
| 105 | `pcre2_pattern_convert_8` | T | pcre2_convert.c | yes |
| 106 | `pcre2_pattern_info_8` | T | pcre2_pattern_info.c | yes |
| 107 | `pcre2_serialize_decode_8` | T | pcre2_serialize.c | yes |
| 108 | `pcre2_serialize_encode_8` | T | pcre2_serialize.c | yes |
| 109 | `pcre2_serialize_free_8` | T | pcre2_serialize.c | yes |
| 110 | `pcre2_serialize_get_number_of_codes_8` | T | pcre2_serialize.c | yes |
| 111 | `pcre2_set_bsr_8` | T | pcre2_context.c | yes |
| 112 | `pcre2_set_callout_8` | T | pcre2_context.c | yes |
| 113 | `pcre2_set_character_tables_8` | T | pcre2_context.c | yes |
| 114 | `pcre2_set_compile_extra_options_8` | T | pcre2_context.c | yes |
| 115 | `pcre2_set_compile_recursion_guard_8` | T | pcre2_context.c | yes |
| 116 | `pcre2_set_depth_limit_8` | T | pcre2_context.c | yes |
| 117 | `pcre2_set_glob_escape_8` | T | pcre2_context.c | yes |
| 118 | `pcre2_set_glob_separator_8` | T | pcre2_context.c | yes |
| 119 | `pcre2_set_heap_limit_8` | T | pcre2_context.c | yes |
| 120 | `pcre2_set_match_limit_8` | T | pcre2_context.c | yes |
| 121 | `pcre2_set_max_pattern_compiled_length_8` | T | pcre2_context.c | yes |
| 122 | `pcre2_set_max_pattern_length_8` | T | pcre2_context.c | yes |
| 123 | `pcre2_set_max_varlookbehind_8` | T | pcre2_context.c | yes |
| 124 | `pcre2_set_newline_8` | T | pcre2_context.c | yes |
| 125 | `pcre2_set_offset_limit_8` | T | pcre2_context.c | yes |
| 126 | `pcre2_set_optimize_8` | T | pcre2_context.c | yes |
| 127 | `pcre2_set_parens_nest_limit_8` | T | pcre2_context.c | yes |
| 128 | `pcre2_set_recursion_limit_8` | T | pcre2_context.c | yes |
| 129 | `pcre2_set_recursion_memory_management_8` | T | pcre2_context.c | yes |
| 130 | `pcre2_set_substitute_callout_8` | T | pcre2_context.c | yes |
| 131 | `pcre2_set_substitute_case_callout_8` | T | pcre2_context.c | yes |
| 132 | `pcre2_substitute_8` | T | pcre2_substitute.c | yes |
| 133 | `pcre2_substring_copy_byname_8` | T | pcre2_substring.c | yes |
| 134 | `pcre2_substring_copy_bynumber_8` | T | pcre2_substring.c | yes |
| 135 | `pcre2_substring_free_8` | T | pcre2_substring.c | yes |
| 136 | `pcre2_substring_get_byname_8` | T | pcre2_substring.c | yes |
| 137 | `pcre2_substring_get_bynumber_8` | T | pcre2_substring.c | yes |
| 138 | `pcre2_substring_length_byname_8` | T | pcre2_substring.c | yes |
| 139 | `pcre2_substring_length_bynumber_8` | T | pcre2_substring.c | yes |
| 140 | `pcre2_substring_list_free_8` | T | pcre2_substring.c | yes |
| 141 | `pcre2_substring_list_get_8` | T | pcre2_substring.c | yes |
| 142 | `pcre2_substring_nametable_scan_8` | T | pcre2_substring.c | yes |
| 143 | `pcre2_substring_number_from_name_8` | T | pcre2_substring.c | yes |

## Symbols missing from the Rust .so

_None._ Symbol diff is empty.

## Symbols exported by Rust but not by C

_None._

## Exported data-object sizes

ELF symbol sizes (`nm -D -S`) for every exported data object; a size
mismatch would mean a differently-shaped table even if the name matches.

Size mismatches: **0**

| symbol | C size | Rust size | same |
|--------|--------|-----------|------|
| `_pcre2_OP_lengths_8` | 0x00000000000000ad | 0x00000000000000ad | yes |
| `_pcre2_callout_end_delims_8` | 0x0000000000000024 | 0x0000000000000024 | yes |
| `_pcre2_callout_start_delims_8` | 0x0000000000000024 | 0x0000000000000024 | yes |
| `_pcre2_default_compile_context_8` | 0x0000000000000058 | 0x0000000000000058 | yes |
| `_pcre2_default_convert_context_8` | 0x0000000000000020 | 0x0000000000000020 | yes |
| `_pcre2_default_match_context_8` | 0x0000000000000060 | 0x0000000000000060 | yes |
| `_pcre2_default_tables_8` | 0x0000000000000440 | 0x0000000000000440 | yes |
| `_pcre2_hspace_list_8` | 0x0000000000000050 | 0x0000000000000050 | yes |
| `_pcre2_posix_class_maps8` | 0x00000000000000a8 | 0x00000000000000a8 | yes |
| `_pcre2_ucd_boolprop_sets_8` | 0x00000000000005f8 | 0x00000000000005f8 | yes |
| `_pcre2_ucd_caseless_sets_8` | 0x00000000000001d8 | 0x00000000000001d8 | yes |
| `_pcre2_ucd_digit_sets_8` | 0x0000000000000138 | 0x0000000000000138 | yes |
| `_pcre2_ucd_nocase_ranges_8` | 0x0000000000000150 | 0x0000000000000150 | yes |
| `_pcre2_ucd_nocase_ranges_size_8` | 0x0000000000000004 | 0x0000000000000004 | yes |
| `_pcre2_ucd_records_8` | 0x0000000000004944 | 0x0000000000004944 | yes |
| `_pcre2_ucd_script_sets_8` | 0x0000000000000770 | 0x0000000000000770 | yes |
| `_pcre2_ucd_stage1_8` | 0x0000000000004400 | 0x0000000000004400 | yes |
| `_pcre2_ucd_stage2_8` | 0x0000000000013a00 | 0x0000000000013a00 | yes |
| `_pcre2_ucd_turkish_dotted_i_caseset_8` | 0x0000000000000004 | 0x0000000000000004 | yes |
| `_pcre2_ucp_gbtable_8` | 0x000000000000003c | 0x000000000000003c | yes |
| `_pcre2_ucp_gentype_8` | 0x0000000000000078 | 0x0000000000000078 | yes |
| `_pcre2_unicode_version_8` | 0x0000000000000008 | 0x0000000000000008 | yes |
| `_pcre2_utf8_table1` | 0x0000000000000018 | 0x0000000000000018 | yes |
| `_pcre2_utf8_table1_size` | 0x0000000000000004 | 0x0000000000000004 | yes |
| `_pcre2_utf8_table2` | 0x0000000000000018 | 0x0000000000000018 | yes |
| `_pcre2_utf8_table3` | 0x0000000000000018 | 0x0000000000000018 | yes |
| `_pcre2_utf8_table4` | 0x0000000000000040 | 0x0000000000000040 | yes |
| `_pcre2_utt_8` | 0x0000000000000c24 | 0x0000000000000c24 | yes |
| `_pcre2_utt_names_8` | 0x0000000000000efa | 0x0000000000000efa | yes |
| `_pcre2_utt_size_8` | 0x0000000000000008 | 0x0000000000000008 | yes |
| `_pcre2_vspace_list_8` | 0x0000000000000020 | 0x0000000000000020 | yes |

## Undefined (imported) symbols in the Rust .so

All are libc / libgcc-unwind / TLS runtime imports, i.e. no unresolved PCRE2 symbol:

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
_Unwind_Backtrace@GCC_3.3
_Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0
_Unwind_GetIPInfo@GCC_4.2.0
_Unwind_GetLanguageSpecificData@GCC_3.0
_Unwind_GetRegionStart@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0
_Unwind_Resume@GCC_3.0
_Unwind_SetGR@GCC_3.0
_Unwind_SetIP@GCC_3.0
__cxa_finalize@GLIBC_2.2.5
__cxa_thread_atexit_impl@GLIBC_2.18
__errno_location@GLIBC_2.2.5
__gmon_start__
__tls_get_addr@GLIBC_2.3
abort@GLIBC_2.2.5
bcmp@GLIBC_2.2.5
calloc@GLIBC_2.2.5
close@GLIBC_2.2.5
dl_iterate_phdr@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat64@GLIBC_2.33
getcwd@GLIBC_2.2.5
getenv@GLIBC_2.2.5
gettid@GLIBC_2.30
isalnum@GLIBC_2.2.5
isalpha@GLIBC_2.2.5
iscntrl@GLIBC_2.2.5
isgraph@GLIBC_2.2.5
islower@GLIBC_2.2.5
isprint@GLIBC_2.2.5
ispunct@GLIBC_2.2.5
isspace@GLIBC_2.2.5
isupper@GLIBC_2.2.5
isxdigit@GLIBC_2.2.5
lseek64@GLIBC_2.2.5
malloc@GLIBC_2.2.5
memchr@GLIBC_2.2.5
memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5
memset@GLIBC_2.2.5
mmap64@GLIBC_2.2.5
munmap@GLIBC_2.2.5
open64@GLIBC_2.2.5
posix_memalign@GLIBC_2.2.5
pthread_key_create@GLIBC_2.34
pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34
read@GLIBC_2.2.5
readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5
realpath@GLIBC_2.3
stat64@GLIBC_2.33
statx@GLIBC_2.28
strlen@GLIBC_2.2.5
syscall@GLIBC_2.2.5
tolower@GLIBC_2.2.5
toupper@GLIBC_2.2.5
write@GLIBC_2.2.5
writev@GLIBC_2.2.5
```
