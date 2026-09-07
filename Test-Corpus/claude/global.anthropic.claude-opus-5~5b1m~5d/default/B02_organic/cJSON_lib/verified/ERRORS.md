# ERRORS.md — error-surface table

Derived mechanically from `c_src/cJSON.c` (+ `c_src/cJSON.h`) by grepping every
`return NULL` / `return false` / `return 0` / `goto fail` / range check / null check
that is reachable from a **public** entry point, plus every named limit constant
(`CJSON_NESTING_LIMIT`, `CJSON_CIRCULAR_LIMIT`, `INT_MAX`, `INT_MIN`).

`false` = `0` (`cJSON_bool`), `true` = `1`.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `cJSON_GetStringValue` | `item == NULL` (`cJSON_IsString` fails) | `NULL` |
| 2  | `cJSON_GetStringValue` | `(item->type & 0xFF) != cJSON_String` | `NULL` |
| 3  | `cJSON_GetNumberValue` | `item == NULL` | `NAN` (returns NaN, verified via `is_nan`) |
| 4  | `cJSON_GetNumberValue` | `(item->type & 0xFF) != cJSON_Number` | `NAN` |
| 5  | `cJSON_SetValuestring` | `object == NULL` | `NULL` |
| 6  | `cJSON_SetValuestring` | `!(object->type & cJSON_String)` (e.g. number/array/object) | `NULL` |
| 7  | `cJSON_SetValuestring` | `object->type & cJSON_IsReference` (item from `cJSON_CreateStringReference`) | `NULL` |
| 8  | `cJSON_SetValuestring` | `object->valuestring == NULL` (corrupted string item) | `NULL` |
| 9  | `cJSON_SetValuestring` | `valuestring == NULL` | `NULL` |
| 10 | `cJSON_ParseWithOpts` | `value == NULL` (`buffer_length = strlen(NULL)+…` guarded: `value == NULL` → fail) | `NULL`, `cJSON_GetErrorPtr()` set |
| 11 | `cJSON_ParseWithLengthOpts` | `value == NULL` | `NULL` |
| 12 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | `NULL` |
| 13 | `cJSON_ParseWithLength` | `value == NULL` / `buffer_length == 0` | `NULL` |
| 14 | `cJSON_Parse` | `value == NULL` | `NULL` |
| 15 | `cJSON_Parse*` | malformed JSON (`parse_value` fails): `"{"`, `"["`, `"tru"`, `"nul"`, `"fals"`, `"@"`, `""`, `"'x'"`, `"01"` (trailing), `"{\"a\"}"`, `"[1,]"`, `"{,}"` | `NULL`; `cJSON_GetErrorPtr()` points at failing byte |
| 16 | `cJSON_Parse*` | trailing garbage with `require_null_terminated != 0`: `"{}x"`, `"1 2"` | `NULL` |
| 17 | `cJSON_Parse*` | nesting deeper than `CJSON_NESTING_LIMIT` (1000): 1001 `[` | `NULL` (`input_buffer->depth >= CJSON_NESTING_LIMIT`) |
| 18 | `cJSON_Parse*` | number literal longer than 63 chars usable window / non-numeric after sign (`parse_number` → `false`) | `NULL` |
| 19 | `cJSON_Parse*` | unterminated string `"\"abc"`; bad escape `"\"\\q\""`; bad `\u` hex `"\"\\uZZZZ\""`; lone surrogate `"\"\\ud800\""`; truncated `\u` | `NULL` |
| 20 | `cJSON_Parse*` (`parse_hex4`) | non-hex digit inside `\uXXXX` | `NULL` (h = 0xffffffff → fail) |
| 21 | `cJSON_Print` | `item == NULL` (`print_value` returns false) | `NULL` |
| 22 | `cJSON_Print` | `item->type & 0xFF` is not a valid type (e.g. `type = 0` / `cJSON_Invalid`, or `type = 0x40000`) | `NULL` |
| 23 | `cJSON_Print` | `type == cJSON_Raw` with `valuestring == NULL` → `print_value` returns false. `type == cJSON_String` with `valuestring == NULL` → `print_string_ptr` emits `""` and SUCCEEDS (verified against C) | `NULL` / `"\"\""` |
| 24 | `cJSON_PrintUnformatted` | same three triggers as rows 21–23 | `NULL` |
| 25 | `cJSON_PrintBuffered` | `prebuffer < 0` (e.g. `-1`, `INT_MIN`) | `NULL` |
| 26 | `cJSON_PrintBuffered` | `item == NULL` / invalid type (`print_value` false) | `NULL` |
| 27 | `cJSON_PrintPreallocated` | `length < 0` | `false` (0) |
| 28 | `cJSON_PrintPreallocated` | `buffer == NULL` | `false` |
| 29 | `cJSON_PrintPreallocated` | buffer too small (`ensure` with `noalloc` → `NULL`) | `false` |
| 30 | `cJSON_PrintPreallocated` | `item == NULL` | `false` |
| 31 | `cJSON_PrintPreallocated` | `length == 0` with any item | `false` |
| 32 | `ensure` (via all print paths) | `needed > INT_MAX` — unreachable in-process, documented limit | `NULL` → print fails |
| 33 | `cJSON_GetArraySize` | `array == NULL` | `0` |
| 34 | `cJSON_GetArraySize` | non-array/object item (no `child`) | `0` |
| 35 | `cJSON_GetArrayItem` | `index < 0` (e.g. `-1`, `INT_MIN`) | `NULL` |
| 36 | `cJSON_GetArrayItem` | `array == NULL` | `NULL` |
| 37 | `cJSON_GetArrayItem` | `index >= size` (one past end, huge index) | `NULL` |
| 38 | `cJSON_GetObjectItem` | `object == NULL` | `NULL` |
| 39 | `cJSON_GetObjectItem` | `string == NULL` | `NULL` |
| 40 | `cJSON_GetObjectItem` | key absent | `NULL` |
| 41 | `cJSON_GetObjectItemCaseSensitive` | `object == NULL` / `string == NULL` / key absent (and key differing only in case) | `NULL` |
| 42 | `cJSON_HasObjectItem` | `object == NULL` / `string == NULL` / key absent | `false` |
| 43 | `cJSON_AddItemToArray` | `item == NULL` | `false` |
| 44 | `cJSON_AddItemToArray` | `array == NULL` | `false` |
| 45 | `cJSON_AddItemToArray` | `array == item` (self-insert) | `false` |
| 46 | `cJSON_AddItemToObject` | `object == NULL` | `false` |
| 47 | `cJSON_AddItemToObject` | `string == NULL` | `false` |
| 48 | `cJSON_AddItemToObject` | `item == NULL` | `false` |
| 49 | `cJSON_AddItemToObject` | `object == item` | `false` |
| 50 | `cJSON_AddItemToObjectCS` | same four triggers as rows 46–49 | `false` |
| 51 | `cJSON_AddItemReferenceToArray` | `array == NULL` | `false` |
| 52 | `cJSON_AddItemReferenceToArray` | `item == NULL` (`create_reference` returns NULL) | `false` |
| 53 | `cJSON_AddItemReferenceToObject` | `object == NULL` | `false` |
| 54 | `cJSON_AddItemReferenceToObject` | `string == NULL` | `false` |
| 55 | `cJSON_AddItemReferenceToObject` | `item == NULL` | `false` |
| 56 | `create_reference` (only reachable via 52/55) | `item == NULL` is its only rejection → returns `NULL`, which makes the caller return `false` | `false` |
| 57 | `cJSON_AddNullToObject` … `cJSON_AddArrayToObject` (9 fns) | `object == NULL` | `NULL` (item created then deleted) |
| 58 | `cJSON_AddNullToObject` … `cJSON_AddArrayToObject` (9 fns) | `name == NULL` | `NULL` |
| 59 | `cJSON_AddStringToObject` | `string == NULL` (`cJSON_CreateString(NULL)` → NULL) | `NULL` |
| 60 | `cJSON_AddRawToObject` | `raw == NULL` (`cJSON_CreateRaw(NULL)` → NULL) | `NULL` |
| 61 | `cJSON_DetachItemViaPointer` | `parent == NULL` | `NULL` |
| 62 | `cJSON_DetachItemViaPointer` | `item == NULL` | `NULL` |
| 63 | `cJSON_DetachItemViaPointer` | `item != parent->child && item->prev == NULL` (item not in parent) | `NULL` |
| 64 | `cJSON_DetachItemFromArray` | `which < 0` | `NULL` |
| 65 | `cJSON_DetachItemFromArray` | `array == NULL` / `which >= size` | `NULL` |
| 66 | `cJSON_DetachItemFromObject` | `object == NULL` / `string == NULL` / key absent | `NULL` |
| 67 | `cJSON_DetachItemFromObjectCaseSensitive` | same as 66 (+ wrong-case key) | `NULL` |
| 68 | `cJSON_DeleteItemFromArray` | `array == NULL`, `which < 0`, `which >= size` | no-op, no crash |
| 69 | `cJSON_DeleteItemFromObject(CaseSensitive)` | `object == NULL` / `string == NULL` / key absent | no-op, no crash |
| 70 | `cJSON_InsertItemInArray` | `which < 0` | `false` |
| 71 | `cJSON_InsertItemInArray` | `newitem == NULL` | `false` |
| 72 | `cJSON_InsertItemInArray` | `array == NULL` (falls to `add_item_to_array(NULL, …)`) | `false` |
| 73 | `cJSON_InsertItemInArray` | `which >= size` → appends instead (returns `true`); with `array == NULL` → `false` | `true` / `false` |
| 74 | `cJSON_ReplaceItemViaPointer` | `parent == NULL` | `false` |
| 75 | `cJSON_ReplaceItemViaPointer` | `parent->child == NULL` (empty parent) | `false` |
| 76 | `cJSON_ReplaceItemViaPointer` | `replacement == NULL` | `false` |
| 77 | `cJSON_ReplaceItemViaPointer` | `item == NULL` | `false` |
| 78 | `cJSON_ReplaceItemViaPointer` | `replacement == item` | `true` (early accept, no modification) |
| 79 | `cJSON_ReplaceItemInArray` | `which < 0` | `false` |
| 80 | `cJSON_ReplaceItemInArray` | `which >= size` (item lookup → NULL) | `false` |
| 81 | `cJSON_ReplaceItemInObject(CaseSensitive)` | `newitem == NULL` | `false` |
| 82 | `cJSON_ReplaceItemInObject(CaseSensitive)` | `string == NULL` | `false` |
| 83 | `cJSON_ReplaceItemInObject(CaseSensitive)` | key absent → `cJSON_ReplaceItemViaPointer(object, NULL, …)` | `false` (but `newitem->string` **is** already overwritten) |
| 84 | `cJSON_ReplaceItemInObject(CaseSensitive)` | `object == NULL` | `false` |
| 85 | `cJSON_CreateString` | `string == NULL` | `NULL` |
| 86 | `cJSON_CreateRaw` | `raw == NULL` | `NULL` |
| 87 | `cJSON_CreateStringReference` | `string == NULL` | item with `valuestring == NULL` (**not** NULL item) |
| 88 | `cJSON_CreateObjectReference` | `child == NULL` | item with `child == NULL` (**not** NULL) |
| 89 | `cJSON_CreateArrayReference` | `child == NULL` | item with `child == NULL` |
| 90 | `cJSON_CreateIntArray` | `count < 0` | `NULL` |
| 91 | `cJSON_CreateIntArray` | `numbers == NULL` | `NULL` |
| 92 | `cJSON_CreateIntArray` | `count == 0` (valid) | empty array, `child == NULL` |
| 93 | `cJSON_CreateFloatArray` | `count < 0` / `numbers == NULL` | `NULL` |
| 94 | `cJSON_CreateDoubleArray` | `count < 0` / `numbers == NULL` | `NULL` |
| 95 | `cJSON_CreateStringArray` | `count < 0` / `strings == NULL` | `NULL` |
| 96 | `cJSON_CreateStringArray` | an element of `strings[]` is `NULL` (`cJSON_CreateString(NULL)` → NULL) | `NULL`, array deleted |
| 97 | `cJSON_Duplicate` | `item == NULL` | `NULL` |
| 98 | `cJSON_Duplicate` | `recurse != 0` on a chain longer than `CJSON_CIRCULAR_LIMIT` (10000) / depth limit | `NULL` |
| 99 | `cJSON_Duplicate` | string/raw item with `valuestring == NULL` — the `if (item->valuestring)` guard skips `cJSON_strdup`, so the copy SUCCEEDS with `valuestring == NULL` (verified against C) | non-NULL item, `valuestring == NULL` |
| 100 | `cJSON_Compare` | `a == NULL` | `false` |
| 101 | `cJSON_Compare` | `b == NULL` | `false` |
| 102 | `cJSON_Compare` | `(a->type & 0xFF) != (b->type & 0xFF)` | `false` |
| 103 | `cJSON_Compare` | `a->type & 0xFF` not one of the 8 valid types (e.g. `cJSON_Invalid == 0`, `0x09`, `0xFF`) | `false` |
| 104 | `cJSON_Compare` | string item with `valuestring == NULL` on either side | `false` |
| 105 | `cJSON_Compare` | `a == b` with valid type | `true` (identity shortcut, even for `Array`/`Object`) |
| 106 | `cJSON_Compare` | object with a key present in `a` but absent in `b` (both directions checked) | `false` |
| 107 | `cJSON_Minify` | `json == NULL` | no-op, no crash |
| 108 | `cJSON_Minify` | unterminated multiline comment / unterminated string | terminates, writes `\0` |
| 109 | `cJSON_Is*` (10 predicates) | `item == NULL` | `false` |
| 110 | `cJSON_IsInvalid` | `item->type & 0xFF != cJSON_Invalid` | `false`; `type == 0` → `true` |
| 111 | `cJSON_InitHooks` | `hooks == NULL` | resets to `malloc`/`free`/`realloc`, returns void |
| 112 | `cJSON_InitHooks` | `hooks->malloc_fn == NULL` and/or `hooks->free_fn == NULL` | falls back to libc for the NULL one; `reallocate` set only if BOTH are libc |
| 113 | `cJSON_free` | `object == NULL` | no-op |
| 114 | `cJSON_malloc` | `size == 0` | implementation-defined non-crash (both must agree on NULL-ness) |
| 115 | `cJSON_SetNumberHelper` | `number >= INT_MAX` | `valueint = INT_MAX`, returns `number` |
| 116 | `cJSON_SetNumberHelper` | `number <= (double)INT_MIN` | `valueint = INT_MIN` |
| 117 | `cJSON_SetNumberHelper` | `number` is NaN (fails both compares → `(int)NaN`) | `valueint = INT_MIN` on x86-64 |
| 118 | `cJSON_CreateNumber` | `num >= INT_MAX` / `num <= INT_MIN` / NaN / ±Inf | clamped `valueint`; `valuedouble = num` |
| 119 | `cJSON_GetErrorPtr` | called after a successful parse | pointer into last error state (`global_error.json + position`) |
| 120 | out-of-range `cJSON_bool` across FFI | `cJSON_Compare(a,b,42)`, `cJSON_Duplicate(i,42)`, `cJSON_CreateBool(42)`, `cJSON_PrintBuffered(i,n,42)`, `cJSON_PrintPreallocated(i,b,n,42)`, `cJSON_ParseWithOpts(v,e,42)`, `cJSON_AddBoolToObject(o,n,42)` | any non-zero treated as true |
| 121 | out-of-range `type` field across FFI | item mutated to `type = -1`, `0x1FF`, `0x10000` then passed to `Print`/`Compare`/`Is*`/`Duplicate`/`Delete` | must match C bit-for-bit |

## Coverage — every row has a passing differential test

All tests live in `tests/phase_c_errors.rs` and call BOTH `.so`s through
`libloading`, asserting the identical error code / sentinel (not merely "both
failed").  Run with `cargo test --offline --test phase_c_errors`.

| rows | test function | status |
|------|---------------|--------|
| 1, 2 | `rows01_02_get_string_value` | [x] |
| 3, 4 | `rows03_04_get_number_value` | [x] |
| 5–9 | `rows05_09_set_valuestring` | [x] |
| 10–14 | `rows10_14_parse_null_inputs` | [x] |
| 15, 16, 18, 19, 20 | `rows15_20_malformed_json` (67 malformed inputs, error-pointer offset compared) | [x] |
| 17 | `row17_nesting_limit` (depth 1000/1001/1002/2000, `[` and `{`) | [x] |
| 21–24 | `rows21_24_print_failures` | [x] |
| 25, 26 | `rows25_26_print_buffered` | [x] |
| 27–31 | `rows27_31_print_preallocated` (every buffer length 0..len+1) | [x] |
| 32 | not reachable in-process (`needed > INT_MAX` would require a >2 GiB document); documented, no test | n/a |
| 33–37 | `rows33_37_array_access` | [x] |
| 38–42 | `rows38_42_object_access` (incl. a child with a NULL key) | [x] |
| 43–56 | `rows43_56_add_rejections` | [x] |
| 57–60 | `rows57_60_add_helpers_null` (all 9 helpers) | [x] |
| 61–69 | `rows61_69_detach_delete` | [x] |
| 70–73 | `rows70_73_insert` | [x] |
| 74–84 | `rows74_84_replace` | [x] |
| 85–96 | `rows85_96_create_rejections` | [x] |
| 97, 99 | `rows97_99_duplicate` | [x] |
| 98 | `row98_duplicate_depth_limit` (nesting 9999/10000/10001/10050 on a 256 MiB stack) | [x] |
| 100–106 | `rows100_106_compare` | [x] |
| 107, 108 | `rows107_108_minify` | [x] |
| 109, 110 | `rows109_110_predicates` | [x] |
| 111–114 | `rows111_114_hooks_and_alloc` | [x] |
| 115–118 | `rows115_118_number_clamping` | [x] |
| 119 | `row119_error_ptr_after_success` | [x] |
| 120 | `row120_out_of_range_bools` (9 out-of-range `cJSON_bool` values × 7 entry points) | [x] |
| 121 | `row121_out_of_range_types` (15 out-of-range `type` values × 4 base items × 14 entry points) | [x] |
