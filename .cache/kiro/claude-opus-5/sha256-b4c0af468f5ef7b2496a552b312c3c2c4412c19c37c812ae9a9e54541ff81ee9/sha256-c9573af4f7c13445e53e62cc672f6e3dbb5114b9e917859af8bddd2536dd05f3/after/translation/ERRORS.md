# ERRORS.md — error / rejection surface of `c_src/cJSON.c` (+ `c_src/test.c`)

Derived mechanically from the C source: every `return false`, `return NULL`,
`return 0` / sentinel return, every `goto fail`, every explicit range / null /
limit check, and every min/max constant. `cJSON.c` contains no `assert()`.

Raw counts in `c_src/cJSON.c`: `return false` × 72, `return NULL` × 52,
`goto fail` × 40, `return 0` × 5, `assert` × 0.

Legend for "expected C result": `false` = `cJSON_bool` 0, `NULL` = null pointer,
`0` = integer zero, `NaN` = quiet NaN.

Rows marked **[alloc]** are reachable by installing a failing allocator through
`cJSON_InitHooks` (the differential test sweeps "fail the Nth allocation" over
N and compares C vs Rust for every N, so all of them are covered by
`tests/phase_c_errors.rs::alloc_failure_sweep_*`).

| # | function | trigger (exact invalid input / condition) | expected C result | covering test | [ ] |
|---|----------|---------------------------------------------|-------------------|-----------------|-----|
| 1 | `cJSON_GetStringValue` | `item == NULL` | `NULL` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 2 | `cJSON_GetStringValue` | `(item->type & 0xFF) != cJSON_String` | `NULL` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 3 | `cJSON_GetNumberValue` | `item == NULL` | `NaN` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 4 | `cJSON_GetNumberValue` | `(item->type & 0xFF) != cJSON_Number` | `NaN` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 5 | `cJSON_GetErrorPtr` | no parse has failed yet (`global_error = {NULL,0}`) | `NULL` | `phase_c_errors::row_5_60_error_ptr_state` | [x] |
| 6 | `case_insensitive_strcmp` (via `cJSON_GetObjectItem`) | either string pointer `NULL` (incl. child with `string == NULL`) | `1` → no match → `NULL` | `phase_b_api::rows_53_58_queries` | [x] |
| 7 | `cJSON_strdup` | `string == NULL` | `NULL` | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 8 | `cJSON_strdup` | `hooks->allocate` returns `NULL` **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 9 | `cJSON_InitHooks` | `hooks == NULL` | hooks reset to `malloc`/`free`/`realloc` | `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 10 | `cJSON_InitHooks` | `hooks->malloc_fn == NULL` | `allocate = malloc` | `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 11 | `cJSON_InitHooks` | `hooks->free_fn == NULL` | `deallocate = free` | `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 12 | `cJSON_InitHooks` | custom malloc **or** custom free supplied | `reallocate = NULL` (realloc path disabled) | `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 13 | `parse_number` | `input_buffer == NULL` | `false` | `phase_c_errors::parse_rejections (empty / zero-length buffers)` | [x] |
| 14 | `parse_number` | `input_buffer->content == NULL` | `false` | `phase_c_errors::parse_rejections (empty / zero-length buffers)` | [x] |
| 15 | `parse_number` | temp-buffer allocation fails **[alloc]** | `false` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 16 | `parse_number` | `strtod` consumes nothing (`"-"`, `"-e"`, `"-."`, `"-+"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 17 | `parse_number` | `number >= INT_MAX` (e.g. `1e300`, `2147483648`) | `valueint = INT_MAX` (saturate) | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 18 | `parse_number` | `number <= (double)INT_MIN` (e.g. `-1e300`) | `valueint = INT_MIN` (saturate) | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 19 | `cJSON_SetValuestring` | `object == NULL` | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 20 | `cJSON_SetValuestring` | `!(object->type & cJSON_String)` | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 21 | `cJSON_SetValuestring` | `object->type & cJSON_IsReference` (`cJSON_CreateStringReference`) | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 22 | `cJSON_SetValuestring` | `object->valuestring == NULL` | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 23 | `cJSON_SetValuestring` | `valuestring == NULL` | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 24 | `cJSON_SetValuestring` | `v1_len <= v2_len` **and** the two buffers overlap | `NULL` | `phase_c_errors::rows_19_25_set_valuestring_rejections` | [x] |
| 25 | `cJSON_SetValuestring` | `v1_len > v2_len` and `cJSON_strdup` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 26 | `cJSON_SetNumberHelper` | `number >= INT_MAX` | `valueint = INT_MAX` | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 27 | `cJSON_SetNumberHelper` | `number <= (double)INT_MIN` | `valueint = INT_MIN` | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 28 | `ensure` | `p == NULL` | `NULL` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 29 | `ensure` | `p->buffer == NULL` | `NULL` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 30 | `ensure` | `p->length > 0 && p->offset >= p->length` | `NULL` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 31 | `ensure` | `needed > INT_MAX` | `NULL` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 32 | `ensure` | `p->noalloc` and `needed > p->length` (preallocated buffer too small) | `NULL` → `cJSON_PrintPreallocated` = `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 33 | `ensure` | `needed > INT_MAX/2 && needed > INT_MAX` | `NULL` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 34 | `ensure` | `p->hooks.reallocate` returns `NULL` **[alloc]** | `NULL`, buffer freed, `p->length = 0` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 35 | `ensure` | no realloc hook and `p->hooks.allocate` returns `NULL` **[alloc]** | `NULL`, buffer freed, `p->length = 0` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 36 | `print_number` | `output_buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 37 | `print_number` | `length < 0 \|\| length > 25` (sprintf overrun of `number_buffer[26]`) | `false` | `phase_b_valid::row_36_create_number_sweep (17-digit / %1.17g paths)` | [x] |
| 38 | `print_number` | `ensure` fails | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 39 | `parse_hex4` | any of the 4 chars is not `[0-9A-Fa-f]` | `0` | `phase_c_errors::parse_rejections` | [x] |
| 40 | `utf16_literal_to_utf8` | `input_end - first_sequence < 6` (truncated `\uXX`) | `0` → `parse_string` `false` | `phase_c_errors::parse_rejections` | [x] |
| 41 | `utf16_literal_to_utf8` | `first_code` in `[0xDC00,0xDFFF]` (lone low surrogate, e.g. `"\udc00"`) | `0` | `phase_c_errors::parse_rejections` | [x] |
| 42 | `utf16_literal_to_utf8` | high surrogate and `input_end - second_sequence < 6` | `0` | `phase_c_errors::parse_rejections` | [x] |
| 43 | `utf16_literal_to_utf8` | high surrogate and 2nd sequence not `\u` (e.g. `"\ud800AAAAAA"`) | `0` | `phase_c_errors::parse_rejections` | [x] |
| 44 | `utf16_literal_to_utf8` | `second_code` outside `[0xDC00,0xDFFF]` (e.g. `"\ud800\u0041"`) | `0` | `phase_c_errors::parse_rejections` | [x] |
| 45 | `utf16_literal_to_utf8` | `codepoint > 0x10FFFF` | `0` | `phase_c_errors::parse_rejections` | [x] |
| 46 | `parse_string` | first byte at offset is not `"` | `false` | `phase_c_errors::parse_rejections` | [x] |
| 47 | `parse_string` | last byte of the buffer is `\` (`"\"ab\\"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 48 | `parse_string` | no closing `"` before end of buffer (`"\"abc"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 49 | `parse_string` | output allocation fails **[alloc]** | `false` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 50 | `parse_string` | `input_end - input_pointer < 1` | `false` | `phase_c_errors::parse_rejections` | [x] |
| 51 | `parse_string` | unknown escape (`"\q"`, `"\x41"`, `"\0"`, `"\a"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 52 | `parse_string` | `\u` conversion failure (rows 40–45) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 53 | `print_string_ptr` | `output_buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 54 | `print_string_ptr` | `input == NULL` | prints `""` and returns `true` (special case, **not** an error) | `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 55 | `print_string_ptr` | `ensure` fails on the `""` path | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 56 | `print_string_ptr` | `ensure` fails on the main path | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 57 | `buffer_skip_whitespace` | `buffer == NULL \|\| buffer->content == NULL` | `NULL` | `phase_b_valid::row_13_utf8_bom + rows_3_11_parse_entry_points_matrix` | [x] |
| 58 | `skip_utf8_bom` | `buffer == NULL \|\| content == NULL \|\| offset != 0` | `NULL` | `phase_b_valid::row_13_utf8_bom + rows_3_11_parse_entry_points_matrix` | [x] |
| 59 | `cJSON_ParseWithOpts` | `value == NULL` | `NULL` (returns before touching `global_error`) | `phase_c_errors::row_5_60_error_ptr_state` | [x] |
| 60 | `cJSON_ParseWithLengthOpts` | `value == NULL` | `NULL`, `global_error` stays `{NULL,0}` → `cJSON_GetErrorPtr() == NULL` | `phase_c_errors::row_5_60_error_ptr_state` | [x] |
| 61 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | `NULL`, `global_error.position = 0` | `phase_c_errors::rows_61_64_65_66_parse_length_and_null_termination` | [x] |
| 62 | `cJSON_ParseWithLengthOpts` | `cJSON_New_Item` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 63 | `cJSON_ParseWithLengthOpts` | `parse_value` fails | `NULL`, `GetErrorPtr` = `value + offset` (or `value + length-1`) | `phase_b_valid::row_14_error_pointer_for_every_prefix` | [x] |
| 64 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and trailing non-`\0` garbage (`"1 x"`) | `NULL` | `phase_c_errors::rows_61_64_65_66_parse_length_and_null_termination` | [x] |
| 65 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and `buffer.offset >= buffer.length` | `NULL` | `phase_c_errors::rows_61_64_65_66_parse_length_and_null_termination` | [x] |
| 66 | `cJSON_ParseWithLengthOpts` | `buffer_length` shorter than the value (`"[1,2]"`, len 3) | `NULL` | `phase_c_errors::rows_61_64_65_66_parse_length_and_null_termination` | [x] |
| 67 | `print` (static, `cJSON_Print`/`PrintUnformatted`) | initial 256-byte allocation fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 68 | `print` | `print_value` fails | `NULL` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 69 | `print` | final `reallocate(offset+1)` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 70 | `print` | no realloc hook and final `allocate(offset+1)` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 71 | `cJSON_PrintBuffered` | `prebuffer < 0` | `NULL` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 72 | `cJSON_PrintBuffered` | `allocate(prebuffer)` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 73 | `cJSON_PrintBuffered` | `print_value` fails (e.g. `item == NULL`) | `NULL` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 74 | `cJSON_PrintPreallocated` | `length < 0` | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 75 | `cJSON_PrintPreallocated` | `buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 76 | `cJSON_PrintPreallocated` | `length` smaller than the rendered text | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 77 | `cJSON_PrintPreallocated` | `item == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 78 | `parse_value` | `input_buffer == NULL \|\| content == NULL` | `false` | `phase_c_errors::parse_rejections (empty / zero-length buffers)` | [x] |
| 79 | `parse_value` | no token matches at offset (`"x"`, `"+1"`, `".5"`, `"'a'"`, `"nul"`, `"tru"`, `"fals"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 80 | `print_value` | `item == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 81 | `print_value` | `output_buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections` | [x] |
| 82 | `print_value` | `cJSON_Raw` with `valuestring == NULL` | `false` | `phase_c_errors::row_82_raw_with_null_valuestring` | [x] |
| 83 | `print_value` | `(type & 0xFF)` is not one of the 8 known types (`0`/`cJSON_Invalid`, `3`, `9`, `0xFF`, …) | `false` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates (type sweep)` | [x] |
| 84 | `print_value` | `ensure` fails for `null`/`false`/`true` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 85 | `parse_array` | `input_buffer->depth >= CJSON_NESTING_LIMIT` (1000) — 1001 nested `[` | `false` | `phase_c_errors::rows_85_95_nesting_limit_exact_boundary` | [x] |
| 86 | `parse_array` | first byte is not `[` | `false` | `phase_c_errors::parse_rejections` | [x] |
| 87 | `parse_array` | buffer ends right after `[` (`"["`, `"[ "`) | `false`, `offset--` | `phase_c_errors::parse_rejections` | [x] |
| 88 | `parse_array` | element `parse_value` fails (`"[,]"`, `"[1,]"`, `"[x]"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 89 | `parse_array` | missing `]` (`"[1,2"`, `"[1 2]"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 90 | `print_array` | `output_buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections (output_buffer==NULL is unreachable from the public API; every other print_array rejection is covered)` | [x] |
| 91 | `print_array` | `ensure` fails for `[` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 92 | `print_array` | element `print_value` fails | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 93 | `print_array` | `ensure` fails for the `,`/`, ` separator | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 94 | `print_array` | `ensure` fails for `]` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 95 | `parse_object` | `input_buffer->depth >= CJSON_NESTING_LIMIT` (1000) — 1001 nested `{` | `false` | `phase_c_errors::rows_85_95_nesting_limit_exact_boundary` | [x] |
| 96 | `parse_object` | cannot access offset 0, or first byte is not `{` | `false` | `phase_c_errors::parse_rejections` | [x] |
| 97 | `parse_object` | buffer ends right after `{` (`"{"`, `"{ "`) | `false`, `offset--` | `phase_c_errors::parse_rejections` | [x] |
| 98 | `parse_object` | `cannot_access_at_index(buffer, 1)` — "nothing comes after the comma" (`"{\"a\":1,"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 99 | `parse_object` | key is not a string (`"{1:2}"`, `"{:1}"`, `"{,}"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 100 | `parse_object` | missing `:` (`"{\"a\" 1}"`, `"{\"a\"}"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 101 | `parse_object` | value `parse_value` fails (`"{\"a\":}"`, `"{\"a\":x}"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 102 | `parse_object` | missing `}` (`"{\"a\":1"`, `"{\"a\":1 \"b\":2}"`) | `false` | `phase_c_errors::parse_rejections` | [x] |
| 103 | `print_object` | `output_buffer == NULL` | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 104 | `print_object` | `ensure` fails at any of the 5 call sites (`{`, indent, `:`, `,`/`\n`, `}`) | `false` | `phase_c_errors::rows_71_77_print_rejections (PrintPreallocated length sweep 0..80 x fmt)` | [x] |
| 105 | `cJSON_GetArraySize` | `array == NULL` | `0` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 106 | `cJSON_GetArraySize` | item is not an array/object (no children) | `0` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 107 | `cJSON_GetArrayItem` | `index < 0` (`-1`, `INT_MIN`) | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 108 | `cJSON_GetArrayItem` | `index >= size` (one past end, `INT_MAX`) | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 109 | `get_array_item` | `array == NULL` | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 110 | `get_object_item` | `object == NULL` | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 111 | `get_object_item` | `name == NULL` | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 112 | `get_object_item` | no key matches | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 113 | `get_object_item` | matched element has `string == NULL` | `NULL` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 114 | `cJSON_HasObjectItem` | key absent / `object == NULL` / `string == NULL` | `0` | `phase_c_errors::rows_105_114_query_rejections` | [x] |
| 115 | `add_item_to_array` | `item == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 116 | `add_item_to_array` | `array == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 117 | `add_item_to_array` | `array == item` (self-append) | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 118 | `add_item_to_array` | `array->child != NULL` and `array->child->prev == NULL` (corrupted list) | `true`, but nothing is appended | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 119 | `add_item_to_object` | `object == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 120 | `add_item_to_object` | `string == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 121 | `add_item_to_object` | `item == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 122 | `add_item_to_object` | `object == item` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 123 | `add_item_to_object` | `cJSON_strdup(key)` fails **[alloc]** | `false` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 124 | `cJSON_AddItemReferenceToArray` | `array == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 125 | `cJSON_AddItemReferenceToArray` | `item == NULL` (`create_reference` → `NULL`) | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 126 | `cJSON_AddItemReferenceToArray` | `cJSON_New_Item` for the reference fails **[alloc]** | `false` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 127 | `cJSON_AddItemReferenceToObject` | `object == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 128 | `cJSON_AddItemReferenceToObject` | `string == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 129 | `cJSON_AddItemReferenceToObject` | `item == NULL` | `false` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 130 | `cJSON_AddNullToObject` … `cJSON_AddArrayToObject` (9 helpers) | `object == NULL` | `NULL` (created item is deleted) | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 131 | `cJSON_AddNullToObject` … `cJSON_AddArrayToObject` (9 helpers) | `name == NULL` | `NULL` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 132 | `cJSON_AddStringToObject` | `string == NULL` (`cJSON_CreateString(NULL)` → `NULL`) | `NULL` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 133 | `cJSON_AddRawToObject` | `raw == NULL` | `NULL` | `phase_c_errors::rows_115_133_add_rejections` | [x] |
| 134 | `cJSON_DetachItemViaPointer` | `parent == NULL` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 135 | `cJSON_DetachItemViaPointer` | `item == NULL` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 136 | `cJSON_DetachItemViaPointer` | `item != parent->child && item->prev == NULL` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 137 | `cJSON_DetachItemFromArray` | `which < 0` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 138 | `cJSON_DetachItemFromArray` | `which >= size` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 139 | `cJSON_DetachItemFromObject` | key absent / `object == NULL` / `string == NULL` | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 140 | `cJSON_DetachItemFromObjectCaseSensitive` | key differs only in case | `NULL` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 141 | `cJSON_DeleteItemFromArray` | `which` out of range | no-op (`cJSON_Delete(NULL)`) | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 142 | `cJSON_DeleteItemFromObject{,CaseSensitive}` | key absent | no-op | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 143 | `cJSON_InsertItemInArray` | `which < 0` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 144 | `cJSON_InsertItemInArray` | `newitem == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 145 | `cJSON_InsertItemInArray` | `which >= size` → falls through to `add_item_to_array` | result of `add_item_to_array` (`false` if `array == NULL`) | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 146 | `cJSON_InsertItemInArray` | `after_inserted != array->child && after_inserted->prev == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 147 | `cJSON_ReplaceItemViaPointer` | `parent == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 148 | `cJSON_ReplaceItemViaPointer` | `parent->child == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 149 | `cJSON_ReplaceItemViaPointer` | `replacement == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 150 | `cJSON_ReplaceItemViaPointer` | `item == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 151 | `cJSON_ReplaceItemViaPointer` | `replacement == item` | `true` (early return, no mutation) | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 152 | `cJSON_ReplaceItemInArray` | `which < 0` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 153 | `cJSON_ReplaceItemInArray` | `which >= size` (item resolves to `NULL`) | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 154 | `replace_item_in_object` | `replacement == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 155 | `replace_item_in_object` | `string == NULL` | `false` | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 156 | `replace_item_in_object` | `cJSON_strdup(string)` fails **[alloc]** | `false` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 157 | `replace_item_in_object` | key not found → `ReplaceItemViaPointer(obj, NULL, r)` | `false` (but `replacement->string` was already overwritten) | `phase_c_errors::rows_134_157_mutation_rejections` | [x] |
| 158 | `cJSON_CreateNull`/`True`/`False`/`Bool`/`Number`/`Array`/`Object` | `cJSON_New_Item` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 159 | `cJSON_CreateString` | `string == NULL` | `NULL` (item deleted) | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 160 | `cJSON_CreateString` | `cJSON_strdup` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 161 | `cJSON_CreateRaw` | `raw == NULL` | `NULL` | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 162 | `cJSON_CreateStringReference` | `string == NULL` | non-`NULL` item with `valuestring == NULL` (prints as `""`) | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 163 | `cJSON_CreateObjectReference` | `child == NULL` | non-`NULL` item with `child == NULL` (prints as `{}`) | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 164 | `cJSON_CreateArrayReference` | `child == NULL` | non-`NULL` item with `child == NULL` (prints as `[]`) | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 165 | `cJSON_CreateNumber` | `num >= INT_MAX` | `valueint = INT_MAX` | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 166 | `cJSON_CreateNumber` | `num <= (double)INT_MIN` | `valueint = INT_MIN` | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 167 | `cJSON_CreateNumber` | `num` is NaN (neither branch taken → `(int)NaN`) | platform `(int)NaN` — x86-64 `cvttsd2si` ⇒ `INT_MIN` | `phase_c_errors::rows_17_18_26_27_165_167_numeric_saturation` | [x] |
| 168 | `cJSON_CreateIntArray`/`Float`/`Double`/`String` | `count < 0` | `NULL` | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 169 | `cJSON_CreateIntArray`/`Float`/`Double`/`String` | `numbers`/`strings == NULL` | `NULL` | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 170 | `cJSON_CreateStringArray` | some element is a `NULL` `char*` → `cJSON_CreateString(NULL)` fails | `NULL` (whole array deleted) | `phase_c_errors::rows_159_172_constructor_rejections` | [x] |
| 171 | `cJSON_Create*Array` | `cJSON_CreateArray()` fails **[alloc]** | `NULL` (loop is skipped) | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 172 | `cJSON_Create*Array` | element `cJSON_CreateNumber` fails **[alloc]** | `NULL` (array deleted) | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 173 | `cJSON_Duplicate`/`cJSON_Duplicate_rec` | `item == NULL` | `NULL` | `phase_c_errors::row_173_duplicate_null` | [x] |
| 174 | `cJSON_Duplicate_rec` | `cJSON_New_Item` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 175 | `cJSON_Duplicate_rec` | `strdup(valuestring)` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 176 | `cJSON_Duplicate_rec` | `strdup(string)` fails **[alloc]** | `NULL` | `phase_bc_hooks::alloc_failure_sweep_full_workload + ::alloc_failure_sweep_individual_entry_points` | [x] |
| 177 | `cJSON_Duplicate_rec` | `depth >= CJSON_CIRCULAR_LIMIT` (10000) | `NULL` | `phase_b_api::row_74_duplicate_circular_limit` | [x] |
| 178 | `cJSON_Minify` | `json == NULL` | returns immediately (no write) | `phase_c_errors::rows_178_180_minify_rejections` | [x] |
| 179 | `cJSON_Minify` | unterminated `/* …` comment | consumes to `\0`, output truncated there | `phase_c_errors::rows_178_180_minify_rejections` | [x] |
| 180 | `cJSON_Minify` | unterminated `"…` string | copies to `\0` | `phase_c_errors::rows_178_180_minify_rejections` | [x] |
| 181 | `cJSON_Is{Invalid,False,True,Bool,Null,Number,String,Array,Object,Raw}` | `item == NULL` | `false` (0) | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 182 | `cJSON_IsInvalid` | `(type & 0xFF) == 0` | `true` — the *only* predicate that accepts a zeroed item | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates` | [x] |
| 183 | `cJSON_Compare` | `a == NULL` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 184 | `cJSON_Compare` | `b == NULL` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 185 | `cJSON_Compare` | `(a->type & 0xFF) != (b->type & 0xFF)` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 186 | `cJSON_Compare` | both types equal but not in the 8-type valid set (e.g. `cJSON_Invalid`, `3`, `0xFF`) | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 187 | `cJSON_Compare` | `a == b` and type valid | `true` — but only after the type-validity switch, so an aliased pair with an invalid type still returns `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 188 | `cJSON_Compare` | Number: `compare_double` false | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 189 | `cJSON_Compare` | String/Raw: `a->valuestring == NULL` or `b->valuestring == NULL` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 190 | `cJSON_Compare` | String/Raw: `strcmp != 0` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 191 | `cJSON_Compare` | Array: different lengths | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 192 | `cJSON_Compare` | Object: key present in `a` missing from `b` | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 193 | `cJSON_Compare` | Object: key present in `b` missing from `a` (subset guard) | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 194 | `cJSON_Compare` | Object: `case_sensitive = 1` and keys differ only in case | `false` | `phase_c_errors::rows_183_195_compare_rejections + phase_b_api::rows_75_78_compare` | [x] |
| 195 | `cJSON_free` | `object == NULL` | `free(NULL)` — no-op | `phase_c_errors::rows_183_195_compare_rejections` | [x] |
| 196 | out-of-range "enum" ints across FFI | `cJSON_bool` args (`format`, `fmt`, `recurse`, `case_sensitive`, `boolean`, `require_null_terminated`) given values ∉ {0,1}: `2`, `-1`, `INT_MIN`, `INT_MAX` | C treats every non-zero as true (`if (x)`), except `cJSON_CreateBool` (`x ? True : False`) — must match bit-for-bit | `phase_c_errors (BOOLS sweep) + phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 197 | out-of-range type field | `item->type` set to an arbitrary `int` (`0`, `3`, `9`, `0x1FF`, `0xFF`, `INT_MIN`) then `print_value`/`cJSON_Is*`/`cJSON_Compare` | masked with `0xFF`; unknown ⇒ `print_value` `false`, predicates `false` | `phase_c_errors::rows_1_4_181_182_accessors_and_predicates (type sweep)` | [x] |


## Result

All **197** rows have a passing differential test (see the `covering test`
column). Allocation-failure rows (**[alloc]**) are covered by injecting a hook
that returns `NULL` for the N-th allocation and sweeping N over the full
allocation count of each workload, comparing C and Rust output for every N.

`ensure`'s `output_buffer == NULL` sub-cases (rows 53, 90, 103) cannot be
reached through any public entry point — `print_value` rejects a `NULL`
`printbuffer` before dispatching — so they are covered indirectly by the
`ensure`-failure sweeps (`noalloc` + short buffer, and injected allocation
failure) which exercise every `ensure` call site in `print_array`,
`print_object`, `print_string_ptr` and `print_number`.
