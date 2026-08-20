# ERRORS.md — error-surface table (Phase A → gate for Phase C)

Every row was derived mechanically from `c_src/cJSON.c` / `c_src/test.c` by
grepping for **every** `return false`, `return NULL`, `return 0`, `return 1`,
`goto fail`, `return -1`, `exit(...)`, every explicit range/null check and every
min/max constant (`INT_MAX`, `INT_MIN`, `CJSON_NESTING_LIMIT`,
`CJSON_CIRCULAR_LIMIT`, `sizeof(number_buffer)-1`, `0x10FFFF`, `0xD800`,
`0xDBFF`, `0xDC00`, `0xDFFF`).

`cJSON_bool` is `int`; `false` = `0`, `true` = `1`. "n/a-alloc" in the *test*
column means the branch is only reachable when the allocator fails — those rows
are still tested, by installing a *fail-on-Nth-malloc* allocator through
`cJSON_InitHooks` in **both** libraries.

Legend for the last column: `[x]` = differential test written and passing.

| # | function | trigger (the exact invalid input/condition) | expected C result | done |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `cJSON_GetStringValue` | `item == NULL` | `NULL` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 2 | `cJSON_GetStringValue` | `(item->type & 0xFF) != cJSON_String` (e.g. number/array/object/raw/invalid) | `NULL` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 3 | `cJSON_GetNumberValue` | `item == NULL` | `NAN` = cJSON's own `0.0/0.0` fallback macro (the C99 `NAN` macro is hidden by `-std=c89`), i.e. bit pattern **`0xFFF8000000000000`** on x86-64 | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 4 | `cJSON_GetNumberValue` | `(item->type & 0xFF) != cJSON_Number` | `NAN` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 5 | `case_insensitive_strcmp` (via `cJSON_GetObjectItem`) | either string `NULL` (object child with `string == NULL`) | returns `1` ⇒ key never matches ⇒ `cJSON_GetObjectItem` = `NULL` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 6 | `cJSON_strdup` (via `cJSON_CreateString`) | `string == NULL` | `cJSON_CreateString(NULL)` ⇒ `NULL` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 7 | `cJSON_strdup` (via `cJSON_CreateRaw`) | `raw == NULL` | `cJSON_CreateRaw(NULL)` ⇒ `NULL` | [x] phase_c_errors::rows_1_to_7_accessor_rejections |
| 8 | `cJSON_strdup` | allocation failure | `NULL` ⇒ caller-specific failure | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 9 | `cJSON_New_Item` | allocation failure | `NULL` ⇒ every `cJSON_Create*` returns `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 10 | `parse_number` | `input_buffer == NULL` or `input_buffer->content == NULL` | `false` (unreachable from public API — content is checked by `parse_value`) | [x] UNREACHABLE (`parse_value` already rejects a NULL `content`; the public-level guard is row 50) - covered by phase_c_errors::rows_10_to_12_parse_number |
| 11 | `parse_number` | `strtod` consumes nothing (`number_c_string == after_end`), e.g. `"-"`, `"-e5"`, `"-."` | `false` ⇒ `cJSON_Parse` = `NULL`, error ptr at offset | [x] phase_c_errors::rows_10_to_12_parse_number |
| 12 | `parse_number` | temporary-buffer allocation failure | `false` ⇒ parse fails | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 13 | `cJSON_SetValuestring` | `object == NULL` | `NULL` | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 14 | `cJSON_SetValuestring` | `!(object->type & cJSON_String)` (number/array/object/raw/invalid) | `NULL` | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 15 | `cJSON_SetValuestring` | `object->type & cJSON_IsReference` (`cJSON_CreateStringReference`) | `NULL` | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 16 | `cJSON_SetValuestring` | `object->valuestring == NULL` (string item with cleared valuestring) | `NULL` | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 17 | `cJSON_SetValuestring` | `valuestring == NULL` | `NULL` | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 18 | `cJSON_SetValuestring` | `v1_len <= v2_len` **and** the two buffers overlap (`!(vs+v1 < ovs \|\| ovs+v2 < vs)`), e.g. `cJSON_SetValuestring(item, item->valuestring)` or `+1` inside it | `NULL`, `valuestring` unchanged | [x] phase_c_errors::rows_13_to_18_set_valuestring |
| 19 | `cJSON_SetValuestring` | `v1_len > v2_len` and `cJSON_strdup` fails | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 20 | `ensure` | `p == NULL` | `NULL` | [x] UNREACHABLE (every public entry point passes a non-NULL `printbuffer`; row 63 is the reachable guard) - covered by phase_c_errors::rows_56_to_71_print_entry_points |
| 21 | `ensure` | `p->buffer == NULL` (i.e. `cJSON_PrintPreallocated(item, NULL, n, f)` path is caught earlier; reached after a failed realloc) | `NULL` | [x] UNREACHABLE (`ensure` only NULLs `p->buffer` on the failure path, and every caller returns immediately) - covered by phase_c_alloc::alloc_failure_at_every_allocation |
| 22 | `ensure` | `p->length > 0 && p->offset >= p->length` | `NULL` ⇒ print fails | [x] UNREACHABLE (`ensure` guarantees `offset < length` after every successful write) - covered by phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 23 | `ensure` | `needed > INT_MAX` | `NULL` | [x] phase_c_huge::rows_23_25_ensure_int_max_guards |
| 24 | `ensure` | `p->noalloc` set and buffer must grow (`cJSON_PrintPreallocated` with too-small `length`) | `NULL` ⇒ `cJSON_PrintPreallocated` = `0` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 25 | `ensure` | `needed > INT_MAX/2` and `needed > INT_MAX` after `needed += offset+1` | `NULL` | [x] phase_c_huge::rows_23_25_ensure_int_max_guards |
| 26 | `ensure` | `hooks.reallocate != NULL` and `realloc` fails | frees buffer, `p->length = 0`, `p->buffer = NULL`, returns `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 27 | `ensure` | `hooks.reallocate == NULL` (custom hooks) and `allocate` fails | frees buffer, `p->length = 0`, `p->buffer = NULL`, returns `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation (custom hooks make `reallocate` NULL) |
| 28 | `print_number` | `output_buffer == NULL` | `false` | [x] UNREACHABLE (`print_value` is never called with a NULL buffer) - covered by phase_c_errors::rows_56_to_71_print_entry_points |
| 29 | `print_number` | `length < 0 \|\| length > sizeof(number_buffer)-1` (25) | `false` — unreachable: `%1.17g` of a `double` is ≤ 24 chars | [x] UNREACHABLE (`%1.17g` of a `double` is at most 24 chars) - the length check is still exercised by phase_b_print::c29_print_number_magnitudes and phase_b_locale::c91 (multi-byte separator) |
| 30 | `print_number` | `ensure` fails (no room for the number) | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 31 | `parse_hex4` | any of the 4 chars is not `[0-9A-Fa-f]`, e.g. `"\uZZZZ"`, `"\u00 0"` | returns `0` ⇒ `utf16_literal_to_utf8` may still succeed (`\u0000`), but a non-hex digit that yields 0 is indistinguishable — parse of `"\uZZZZ"` produces a NUL byte | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 32 | `utf16_literal_to_utf8` | fewer than 6 bytes left (`"\u12"`, `"\u"`) | `0` ⇒ `parse_string` fails ⇒ `NULL` | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 33 | `utf16_literal_to_utf8` | `first_code` in `[0xDC00,0xDFFF]` (lone low surrogate, `"\udc00"`) | `0` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 34 | `utf16_literal_to_utf8` | high surrogate but fewer than 6 bytes for the 2nd sequence (`"\ud800\u12"`) | `0` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 35 | `utf16_literal_to_utf8` | high surrogate not followed by `\u` (`"\ud800abcdef"`) | `0` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 36 | `utf16_literal_to_utf8` | 2nd code not in `[0xDC00,0xDFFF]` (`"\ud800A"`) | `0` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 37 | `utf16_literal_to_utf8` | `codepoint > 0x10FFFF` | `0` — unreachable (surrogate pair maxes at `0x10FFFF`) | [x] UNREACHABLE (a surrogate pair maxes out at 0x10FFFF) - the neighbouring branches are covered by phase_c_errors::rows_31_to_43_string_rejections |
| 38 | `parse_string` | first byte is not `"` | `false` (unreachable from `parse_value`, reachable from `parse_object` key slot: `{a:1}`) | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 39 | `parse_string` | last byte of the buffer is a backslash (`"abc\`) | `false` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 40 | `parse_string` | no closing quote before end of buffer (`"abc`) | `false` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 41 | `parse_string` | output allocation failure | `false` ⇒ parse fails | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 42 | `parse_string` | unknown escape char (`"\x"`, `"\ "`, `"\0"`) | `false` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 43 | `parse_string` | `utf16_literal_to_utf8` returned 0 | `false` ⇒ parse fails | [x] phase_c_errors::rows_31_to_43_string_rejections |
| 44 | `print_string_ptr` | `output_buffer == NULL` | `false` | [x] UNREACHABLE (`print_string_ptr` is never called with a NULL buffer) - covered by phase_c_errors::rows_56_to_71_print_entry_points |
| 45 | `print_string_ptr` | `input == NULL` and `ensure(3)` fails | `false` (printing an object whose key is `NULL`, or a `cJSON_String` with `valuestring == NULL`, into a too-small preallocated buffer) | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 46 | `print_string_ptr` | `ensure(output_length+3)` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 47 | `buffer_skip_whitespace` | `buffer == NULL` or `buffer->content == NULL` | `NULL` ⇒ `parse_value(item, NULL)` ⇒ `false` | [x] UNREACHABLE (`content` is always non-NULL once row 50 passed) - covered by phase_c_errors::rows_49_to_54_parse_entry_points |
| 48 | `skip_utf8_bom` | `buffer == NULL` / `content == NULL` / `offset != 0` | `NULL` ⇒ `buffer_skip_whitespace(NULL)` ⇒ `NULL` ⇒ parse fails | [x] UNREACHABLE (`offset` is always 0 at that call site) - covered by phase_b_parse::c38_c46_parse_documents (BOM cases) |
| 49 | `cJSON_ParseWithOpts` | `value == NULL` | `NULL` (and `global_error` is *not* updated — it was reset to `{NULL,0}`) | [x] phase_c_errors::rows_49_to_54_parse_entry_points |
| 50 | `cJSON_ParseWithLengthOpts` | `value == NULL` | `NULL`, `cJSON_GetErrorPtr() == NULL` | [x] phase_c_errors::rows_49_to_54_parse_entry_points |
| 51 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | `NULL`, error position 0, `*return_parse_end = value` | [x] phase_c_errors::rows_49_to_54_parse_entry_points |
| 52 | `cJSON_ParseWithLengthOpts` | `cJSON_New_Item` allocation failure | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 53 | `cJSON_ParseWithLengthOpts` | `parse_value` fails | `NULL`, `global_error.position = offset` (or `length-1` when `offset >= length`), `*return_parse_end` set to the same byte | [x] phase_c_errors::rows_49_to_54_parse_entry_points |
| 54 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and the byte after the value is not `\0` (`"1 x"`, `"{} "` with length excluding the NUL) | `NULL` + error ptr | [x] phase_c_errors::rows_49_to_54_parse_entry_points |
| 55 | `print` (static) | initial 256-byte buffer allocation fails | `NULL` ⇒ `cJSON_Print`/`cJSON_PrintUnformatted` = `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 56 | `print` (static) | `print_value` fails (invalid item, e.g. `type = cJSON_Invalid`, or `cJSON_Raw` with `valuestring == NULL`) | `NULL` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 57 | `print` (static) | final `realloc`/`allocate` shrink fails | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 58 | `cJSON_Print` / `cJSON_PrintUnformatted` | `item == NULL` | `NULL` (via `print_value`) | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 59 | `cJSON_PrintBuffered` | `prebuffer < 0` | `NULL` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 60 | `cJSON_PrintBuffered` | `global_hooks.allocate(prebuffer)` fails | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 61 | `cJSON_PrintBuffered` | `print_value` fails (`item == NULL`, invalid type, raw with NULL string) | `NULL` (buffer freed) | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 62 | `cJSON_PrintPreallocated` | `length < 0` | `0` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 63 | `cJSON_PrintPreallocated` | `buffer == NULL` | `0` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 64 | `cJSON_PrintPreallocated` | `length` too small for the rendered value | `0` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 65 | `cJSON_PrintPreallocated` | `item == NULL` | `0` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 66 | `parse_value` | `input_buffer == NULL` (i.e. `skip_utf8_bom`/`buffer_skip_whitespace` returned NULL) or `content == NULL` | `false` | [x] UNREACHABLE (`buffer_skip_whitespace`/`skip_utf8_bom` cannot return NULL here) - covered by phase_c_errors::rows_49_to_54_parse_entry_points |
| 67 | `parse_value` | no token matches (`"x"`, `"'a'"`, `"+1"`, `".5"`, `"nul"`, `"tru"`, `"fals"`, `""`) | `false` ⇒ `cJSON_Parse` = `NULL` | [x] phase_c_errors::rows_49_to_54_parse_entry_points + phase_b_parse::c38_c46_parse_documents |
| 68 | `print_value` | `item == NULL` | `false` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 69 | `print_value` | `output_buffer == NULL` | `false` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 70 | `print_value` | `cJSON_Raw` and `item->valuestring == NULL` | `false` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 71 | `print_value` | `(type & 0xFF)` is not one of the 8 known types (`cJSON_Invalid` = 0, `3`, `0x0F`, `0xFF`, or bits above 0xFF only) | `false` | [x] phase_c_errors::rows_56_to_71_print_entry_points |
| 72 | `parse_array` | `input_buffer->depth >= CJSON_NESTING_LIMIT` (1000) | `false` ⇒ parse of 1000-deep `[[[...]]]` fails, 999-deep succeeds | [x] phase_c_errors::rows_72_to_91_container_parse_rejections + phase_b_parse::c55_c56_nesting_limit |
| 73 | `parse_array` | first byte is not `[` | `false` (unreachable from `parse_value`) | [x] UNREACHABLE (`parse_value` only calls `parse_array` after checking for `[`) - covered by phase_c_errors::rows_72_to_91_container_parse_rejections |
| 74 | `parse_array` | end of buffer right after `[` (`"["`, `"[ "`) | `false`, `offset--` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 75 | `parse_array` | `cJSON_New_Item` for an element fails | `false` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 76 | `parse_array` | element `parse_value` fails (`"[,]"`, `"[x]"`, `"[1,]"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 77 | `parse_array` | no `]` where expected (`"[1"`, `"[1 2]"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 78 | `print_array` | `output_buffer == NULL` | `false` | [x] UNREACHABLE (`print_array` is never called with a NULL buffer) - covered by phase_c_errors::rows_56_to_71_print_entry_points |
| 79 | `print_array` | `ensure(1)` for `[` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 80 | `print_array` | element `print_value` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 81 | `print_array` | `ensure(length+1)` for `,`/`, ` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 82 | `print_array` | `ensure(2)` for `]` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 83 | `parse_object` | `depth >= CJSON_NESTING_LIMIT` | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 84 | `parse_object` | cannot access index 0 or first byte is not `{` | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 85 | `parse_object` | end of buffer right after `{` (`"{"`, `"{ "`) | `false`, `offset--` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 86 | `parse_object` | `cJSON_New_Item` for a member fails | `false` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 87 | `parse_object` | `cannot_access_at_index(buffer,1)` — nothing after the `,`/`{` (`"{\"a\":1,"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 88 | `parse_object` | key `parse_string` fails (`"{a:1}"`, `"{1:2}"`, `"{\"a:1}"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 89 | `parse_object` | missing `:` (`"{\"a\" 1}"`, `"{\"a\"}"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 90 | `parse_object` | value `parse_value` fails (`"{\"a\":}"`, `"{\"a\":x}"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 91 | `parse_object` | missing `}` (`"{\"a\":1"`, `"{\"a\":1 \"b\":2}"`) | `false` | [x] phase_c_errors::rows_72_to_91_container_parse_rejections |
| 92 | `print_object` | `output_buffer == NULL` | `false` | [x] UNREACHABLE (`print_object` is never called with a NULL buffer) - covered by phase_c_errors::rows_56_to_71_print_entry_points |
| 93 | `print_object` | `ensure(length+1)` for `{` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 94 | `print_object` | `ensure(depth)` for the indentation fails (formatted only) | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 95 | `print_object` | key `print_string_ptr` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 96 | `print_object` | `ensure(length)` for `:`/`:\t` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 97 | `print_object` | member `print_value` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 98 | `print_object` | `ensure(length+1)` for `,`/`\n` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 99 | `print_object` | final `ensure(depth+1 \| 2)` for `}` fails | `false` | [x] phase_c_errors::rows_24_30_45_46_79_99_ensure_failures |
| 100 | `cJSON_GetArraySize` | `array == NULL` | `0` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 101 | `cJSON_GetArraySize` | item that is not an array/object (no `child`) | `0` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 102 | `get_array_item` | `array == NULL` | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 103 | `get_array_item` | `index >= size` (one past the end, `SIZE_MAX`-ish large index) | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 104 | `cJSON_GetArrayItem` | `index < 0` (`-1`, `INT_MIN`) | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 105 | `get_object_item` | `object == NULL` | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 106 | `get_object_item` | `name == NULL` | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 107 | `get_object_item` | key not present (case-sensitive lookup of a differently-cased key) | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 108 | `get_object_item` | walk stops on a child whose `string == NULL` (array element, case-sensitive) | `NULL` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 109 | `cJSON_HasObjectItem` | key absent / `object == NULL` / `string == NULL` | `0` | [x] phase_c_errors2::rows_100_to_109_query_rejections |
| 110 | `create_reference` | `item == NULL` | `NULL` ⇒ `cJSON_AddItemReferenceToArray(arr, NULL)` = `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 111 | `create_reference` | `cJSON_New_Item` fails | `NULL` | [x] phase_c_errors2::rows_110_to_125_add_rejections + phase_c_alloc::alloc_failure_at_every_allocation |
| 112 | `add_item_to_array` | `item == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 113 | `add_item_to_array` | `array == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 114 | `add_item_to_array` | `array == item` (self-append) | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 115 | `add_item_to_object` | `object == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 116 | `add_item_to_object` | `string == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 117 | `add_item_to_object` | `item == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 118 | `add_item_to_object` | `object == item` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 119 | `add_item_to_object` | non-constant key and `cJSON_strdup` fails | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections + phase_c_alloc::alloc_failure_at_every_allocation |
| 120 | `cJSON_AddItemReferenceToArray` | `array == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 121 | `cJSON_AddItemReferenceToObject` | `object == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 122 | `cJSON_AddItemReferenceToObject` | `string == NULL` | `0` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 123 | `cJSON_AddNullToObject` … `cJSON_AddArrayToObject` (9 fns) | `object == NULL` or `name == NULL` ⇒ `add_item_to_object` fails | created item deleted, returns `NULL` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 124 | `cJSON_AddStringToObject` | `string == NULL` ⇒ `cJSON_CreateString(NULL)` = `NULL` ⇒ `add_item_to_object` fails | `NULL` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 125 | `cJSON_AddRawToObject` | `raw == NULL` | `NULL` | [x] phase_c_errors2::rows_110_to_125_add_rejections |
| 126 | `cJSON_DetachItemViaPointer` | `parent == NULL` | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 127 | `cJSON_DetachItemViaPointer` | `item == NULL` | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 128 | `cJSON_DetachItemViaPointer` | `item != parent->child && item->prev == NULL` (foreign / already-detached item) | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 129 | `cJSON_DetachItemFromArray` | `which < 0` | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 130 | `cJSON_DetachItemFromArray` | `which >= size` | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 131 | `cJSON_DetachItemFromObject(CaseSensitive)` | key absent / `object == NULL` / `string == NULL` | `NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 132 | `cJSON_DeleteItemFromArray` / `…FromObject` / `…CaseSensitive` | same invalid inputs as above | no-op (`cJSON_Delete(NULL)`) | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 133 | `cJSON_InsertItemInArray` | `which < 0` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 134 | `cJSON_InsertItemInArray` | `newitem == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 135 | `cJSON_InsertItemInArray` | `which >= size` ⇒ falls through to `add_item_to_array` | `1` for a real array, `0` when `array == NULL` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 136 | `cJSON_InsertItemInArray` | `after_inserted != array->child && after_inserted->prev == NULL` (corrupted list) | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 137 | `cJSON_ReplaceItemViaPointer` | `parent == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 138 | `cJSON_ReplaceItemViaPointer` | `parent->child == NULL` (empty array/object) | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 139 | `cJSON_ReplaceItemViaPointer` | `replacement == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 140 | `cJSON_ReplaceItemViaPointer` | `item == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 141 | `cJSON_ReplaceItemViaPointer` | `replacement == item` | `1`, nothing changed (early return) | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 142 | `cJSON_ReplaceItemInArray` | `which < 0` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 143 | `cJSON_ReplaceItemInArray` | `which >= size` ⇒ `item == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 144 | `replace_item_in_object` | `replacement == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 145 | `replace_item_in_object` | `string == NULL` | `0` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 146 | `replace_item_in_object` | `cJSON_strdup(string)` fails | `0` (replacement->string left NULL) | [x] phase_c_errors2::rows_126_to_147_mutation_rejections + phase_c_alloc::alloc_failure_at_every_allocation |
| 147 | `replace_item_in_object` | key absent ⇒ `get_object_item` = `NULL` ⇒ `cJSON_ReplaceItemViaPointer(item=NULL)` | `0`, but `replacement->string` **has been overwritten** with a copy of `string` | [x] phase_c_errors2::rows_126_to_147_mutation_rejections |
| 148 | `cJSON_CreateString` | `cJSON_strdup(string)` fails (`string == NULL` or alloc failure) ⇒ item deleted | `NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections |
| 149 | `cJSON_CreateRaw` | ditto | `NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections |
| 150 | `cJSON_CreateIntArray` / `Float` / `Double` / `String` | `count < 0` | `NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections |
| 151 | `cJSON_CreateIntArray` / `Float` / `Double` / `String` | `numbers`/`strings == NULL` | `NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections |
| 152 | `cJSON_CreateIntArray` / `Float` / `Double` / `String` | element creation fails (`cJSON_CreateNumber`/`CreateString` = `NULL`; `strings[i] == NULL`) | array deleted, `NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections + phase_c_alloc::alloc_failure_at_every_allocation |
| 153 | `cJSON_CreateIntArray` / … | `count == 0` (valid, but the `a && a->child` guard is the boundary) | empty array, `child == NULL` | [x] phase_c_errors2::rows_148_to_153_create_rejections |
| 154 | `cJSON_Duplicate_rec` | `item == NULL` (also `cJSON_Duplicate(NULL, 0/1)`) | `NULL` | [x] phase_c_errors2::rows_154_159_160_misc_rejections |
| 155 | `cJSON_Duplicate_rec` | `cJSON_New_Item` fails | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 156 | `cJSON_Duplicate_rec` | `valuestring` strdup fails | `NULL` (partially built item deleted) | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 157 | `cJSON_Duplicate_rec` | `string` strdup fails (non-const key) | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 158 | `cJSON_Duplicate_rec` | `depth >= CJSON_CIRCULAR_LIMIT` (10000) with `recurse != 0` | `NULL` | [x] phase_b_api::c79b_duplicate_circular_limit |
| 159 | `cJSON_Minify` | `json == NULL` | no-op, returns | [x] phase_c_errors2::rows_154_159_160_misc_rejections |
| 160 | `cJSON_Is{Invalid,False,True,Bool,Null,Number,String,Array,Object,Raw}` | `item == NULL` | `0` for **all ten**, including `cJSON_IsInvalid(NULL)` | [x] phase_c_errors2::rows_154_159_160_misc_rejections + phase_b_api::c65_type_predicates |
| 161 | `cJSON_Compare` | `a == NULL` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 162 | `cJSON_Compare` | `b == NULL` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 163 | `cJSON_Compare` | `(a->type & 0xFF) != (b->type & 0xFF)` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 164 | `cJSON_Compare` | both types equal but invalid (`0`, `3`, `0xFF`) | `0` (first `switch` default) | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 165 | `cJSON_Compare` | numbers not equal within `DBL_EPSILON` relative tolerance | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 166 | `cJSON_Compare` | `cJSON_String`/`cJSON_Raw` with `a->valuestring == NULL` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 167 | `cJSON_Compare` | `cJSON_String`/`cJSON_Raw` with `b->valuestring == NULL` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 168 | `cJSON_Compare` | strings differ | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 169 | `cJSON_Compare` | arrays of different length (`a_element != b_element` after the walk) | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 170 | `cJSON_Compare` | object: a key of `a` missing from `b` | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 171 | `cJSON_Compare` | object: a key of `b` missing from `a` (subset check) | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 172 | `cJSON_Compare` | nested child comparison fails | `0` | [x] phase_c_errors2::rows_161_to_172_compare_rejections |
| 173 | `cJSON_InitHooks` | `hooks == NULL` | resets to `malloc`/`free`/`realloc`, returns void | [x] phase_b_hooks::c3_init_hooks_reset |
| 174 | `cJSON_InitHooks` | `hooks->malloc_fn == NULL` and/or `hooks->free_fn == NULL` | the NULL one falls back to libc, and `reallocate` is `realloc` **only** when both ended up being exactly `malloc`/`free` | [x] phase_b_hooks::c5_c6_init_hooks_partial + phase_b_hooks::row174_hooks_with_real_libc_allocator |
| 175 | `cJSON_free` | `object == NULL` | `free(NULL)` — no-op | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 176 | `cJSON_malloc` | `size == 0` | whatever `malloc(0)` returns (non-NULL on glibc), must match | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 177 | `cJSON_malloc` | allocator returns `NULL` (custom failing hook) | `NULL` | [x] phase_c_alloc::alloc_failure_at_every_allocation |
| 178 | `cJSON_SetNumberHelper` | `number` is `NaN` — neither `>= INT_MAX` nor `<= INT_MIN` ⇒ `(int)NaN` | `valueint = INT_MIN` (x86-64 `cvttsd2si`), `valuedouble = NaN` | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 179 | `cJSON_CreateNumber` | `number` is `NaN` / `±Inf` / `> INT_MAX` / `< INT_MIN` | `valueint` saturated (`INT_MAX`/`INT_MIN`), `valuedouble` verbatim | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 180 | out-of-range enum/bool values across FFI | `cJSON_bool` arguments that are neither 0 nor 1 (`2`, `-1`, `INT_MIN`, `0x100`) passed to `cJSON_CreateBool`, `cJSON_Compare`, `cJSON_Duplicate`, `cJSON_PrintBuffered`, `cJSON_PrintPreallocated`, `cJSON_ParseWithOpts` | every non-zero value behaves as `true` (C tests `!= 0` / truthiness) | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 181 | out-of-range `type` values across FFI | `item->type` set to values with no valid variant (`0`, `3`, `0x0F`, `0x1FF`, `0xFF`, `INT_MIN`) then printed/compared/queried | `print_value`/`cJSON_Compare` = `0`; `cJSON_Is*` = `0` unless the masked bits happen to match | [x] phase_c_errors2::rows_175_to_181_boundaries |
| 182 | `driver` (test.c) | `print_preallocated` gets `cJSON_PrintPreallocated` = 0 for the *undersized* buffer — the expected path | prints the JSON, returns 0; a *failure* of the first `PrintPreallocated` would `return -1` ⇒ `exit(EXIT_FAILURE)` | [x] phase_b_driver::c90_driver_stdout_differential |
| 183 | `driver` (test.c) | `strings[i] == NULL` ⇒ `cJSON_CreateStringArray` returns `NULL` ⇒ `print_preallocated(NULL)` ⇒ `cJSON_Print(NULL)` = `NULL` ⇒ `strlen(NULL)` → segfault in **both** implementations | both processes die with `SIGSEGV` (11) | [x] phase_b_driver::row183_null_string_argument_kills_both_identically (child processes, both die with SIGSEGV) |
