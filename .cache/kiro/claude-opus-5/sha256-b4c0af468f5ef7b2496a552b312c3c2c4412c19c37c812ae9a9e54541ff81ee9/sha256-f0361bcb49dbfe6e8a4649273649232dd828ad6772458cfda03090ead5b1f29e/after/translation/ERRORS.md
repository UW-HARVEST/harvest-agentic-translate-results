# ERRORS.md — error-surface table

Derived mechanically from `c_src/cJSON.c` (and `c_src/cJSON.h`). Every row is a
distinct way the C library rejects/errors. `[x]` = differential test exists and
passes (see `translation/tests/`).

Legend for reachability: rows marked **(alloc)** are reachable only when an
allocation fails.  `cJSON_InitHooks` lets a test install a hook allocator that
fails the k-th request, so these rows ARE differentially tested — see
`tests/c_errors.rs::alloc_failure_paths_match`, which drives 17 operations,
asserts C and Rust perform the **same number** of allocations, and then forces
each allocation in turn to fail and compares the results.  Only four rows remain
untestable through the FFI boundary and are marked `inspection`:
rows 27 and 64 (unreachable — no input can reach them), row 61 (needs a >2 GiB
output buffer) and row 166 (the C dereferences NULL, i.e. undefined behaviour,
so there is no defined result to compare).

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | cJSON_GetStringValue | `!cJSON_IsString(item)` (NULL, or non-String type) | `NULL` | [x] |
| 2 | cJSON_GetNumberValue | `!cJSON_IsNumber(item)` (NULL, or non-Number type) | `(double)NAN` | [x] |
| 3 | cJSON_SetValuestring | `object == NULL` | `NULL` | [x] |
| 4 | cJSON_SetValuestring | `!(object->type & cJSON_String)` | `NULL` | [x] |
| 5 | cJSON_SetValuestring | `object->type & cJSON_IsReference` (string reference) | `NULL` | [x] |
| 6 | cJSON_SetValuestring | `object->valuestring == NULL` | `NULL` | [x] |
| 7 | cJSON_SetValuestring | `valuestring == NULL` | `NULL` | [x] |
| 8 | cJSON_SetValuestring | overlapping buffers: `v1_len <= v2_len` and ranges overlap | `NULL` | [x] |
| 9 | cJSON_SetValuestring | **(alloc)** `cJSON_strdup` fails on the longer-string path | `NULL` | [x] |
| 10 | cJSON_ParseWithOpts | `NULL == value` | `NULL` | [x] |
| 11 | cJSON_ParseWithLengthOpts | `value == NULL` | `NULL` | [x] |
| 12 | cJSON_ParseWithLengthOpts | `0 == buffer_length` | `NULL` | [x] |
| 13 | cJSON_ParseWithLengthOpts | `!parse_value(...)` — malformed JSON | `NULL`, `cJSON_GetErrorPtr()` set, `*return_parse_end` set to error position | [x] |
| 14 | cJSON_ParseWithLengthOpts | `require_null_terminated` and `(buffer.offset >= buffer.length) \|\| buffer_at_offset[0] != '\0'` (trailing garbage) | `NULL` | [x] |
| 15 | cJSON_ParseWithLengthOpts | **(alloc)** `cJSON_New_Item` returns NULL | `NULL` | [x] |
| 16 | cJSON_Parse | any failure of `cJSON_ParseWithOpts` (NULL / malformed) | `NULL` | [x] |
| 17 | cJSON_ParseWithLength | any failure of `cJSON_ParseWithLengthOpts` | `NULL` | [x] |
| 18 | parse_number (via Parse*) | `(input_buffer == NULL) \|\| (input_buffer->content == NULL)` | parse fails → `NULL` | [x] |
| 19 | parse_number (via Parse*) | `number_c_string == after_end` — strtod consumed nothing (e.g. `-`, `.`, `e5`) | parse fails → `NULL` | [x] |
| 20 | parse_number (via Parse*) | **(alloc)** number scratch buffer alloc fails | `NULL` | [x] |
| 21 | parse_hex4 (via Parse*) | escape digit not in `[0-9A-Fa-f]` (e.g. `"\uZZZZ"`) | returns 0 → utf16 conv fails → parse `NULL` | [x] |
| 22 | utf16_literal_to_utf8 (via Parse*) | `(input_end - first_sequence) < 6` — input ends inside `\u` | parse fails → `NULL` | [x] |
| 23 | utf16_literal_to_utf8 (via Parse*) | `first_code` in `0xDC00..0xDFFF` — lone low surrogate | parse fails → `NULL` | [x] |
| 24 | utf16_literal_to_utf8 (via Parse*) | high surrogate then `(input_end - second_sequence) < 6` | parse fails → `NULL` | [x] |
| 25 | utf16_literal_to_utf8 (via Parse*) | high surrogate then `second_sequence[0] != '\\' \|\| [1] != 'u'` | parse fails → `NULL` | [x] |
| 26 | utf16_literal_to_utf8 (via Parse*) | high surrogate then `second_code < 0xDC00 \|\| > 0xDFFF` | parse fails → `NULL` | [x] |
| 27 | utf16_literal_to_utf8 (via Parse*) | resulting `codepoint > 0x10FFFF` | parse fails → `NULL` | inspection (unreachable via \u) |
| 28 | parse_string (via Parse*) | `buffer_at_offset[0] != '"'` — not a string | parse fails → `NULL` | [x] |
| 29 | parse_string (via Parse*) | trailing backslash: `(input_end + 1 - content) >= length` | parse fails → `NULL` | [x] |
| 30 | parse_string (via Parse*) | unterminated string: `(input_end - content) >= length \|\| *input_end != '"'` | parse fails → `NULL` | [x] |
| 31 | parse_string (via Parse*) | escape at very end: `(input_end - input_pointer) < 1` | parse fails → `NULL` | [x] |
| 32 | parse_string (via Parse*) | unknown escape char (`default:` in escape switch, e.g. `"\x"`) | parse fails → `NULL` | [x] |
| 33 | parse_string (via Parse*) | **(alloc)** output buffer alloc fails | `NULL` | [x] |
| 34 | parse_value (via Parse*) | `(input_buffer == NULL) \|\| (input_buffer->content == NULL)` | `false` → `NULL` | [x] |
| 35 | parse_value (via Parse*) | first char matches no value type (e.g. `x`, `'`, `+1`, `nul`, `tru`, `fals`) | `false` → `NULL` | [x] |
| 36 | parse_array (via Parse*) | `input_buffer->depth >= CJSON_NESTING_LIMIT` (1000) | `false` → `NULL` | [x] |
| 37 | parse_array (via Parse*) | `buffer_at_offset[0] != '['` | `false` → `NULL` | [x] |
| 38 | parse_array (via Parse*) | `cannot_access_at_index(input_buffer, 0)` after `[` / after `,` (truncated) | `false` → `NULL` | [x] |
| 39 | parse_array (via Parse*) | element `!parse_value(...)` (e.g. `[x]`) | `false` → `NULL` | [x] |
| 40 | parse_array (via Parse*) | `cannot_access_at_index(...) \|\| buffer_at_offset[0] != ']'` (e.g. `[1`, `[1}`) | `false` → `NULL` | [x] |
| 41 | parse_array (via Parse*) | **(alloc)** `cJSON_New_Item` for element fails | `NULL` | [x] |
| 42 | parse_object (via Parse*) | `input_buffer->depth >= CJSON_NESTING_LIMIT` (1000) | `false` → `NULL` | [x] |
| 43 | parse_object (via Parse*) | `buffer_at_offset[0] != '{'` | `false` → `NULL` | [x] |
| 44 | parse_object (via Parse*) | `cannot_access_at_index(...)` after `{` / after `,` (truncated) | `false` → `NULL` | [x] |
| 45 | parse_object (via Parse*) | key `!parse_string(...)` (e.g. `{1:2}`, `{a:1}`) | `false` → `NULL` | [x] |
| 46 | parse_object (via Parse*) | missing colon: `buffer_at_offset[0] != ':'` (e.g. `{"a" 1}`) | `false` → `NULL` | [x] |
| 47 | parse_object (via Parse*) | value `!parse_value(...)` (e.g. `{"a":x}`) | `false` → `NULL` | [x] |
| 48 | parse_object (via Parse*) | `cannot_access_at_index(...) \|\| buffer_at_offset[0] != '}'` (e.g. `{"a":1`) | `false` → `NULL` | [x] |
| 49 | parse_object (via Parse*) | **(alloc)** `cJSON_New_Item` for member fails | `NULL` | [x] |
| 50 | cJSON_Print | `print()` fails (render failure, e.g. `item == NULL`, or Invalid type) | `NULL` | [x] |
| 51 | cJSON_PrintUnformatted | `print()` fails (same triggers as row 50, `format=false`) | `NULL` | [x] |
| 52 | cJSON_Print / PrintUnformatted | **(alloc)** initial `allocate`, `print_value`, or final `reallocate`/`allocate` fails | `NULL` | [x] |
| 53 | cJSON_PrintBuffered | `prebuffer < 0` | `NULL` | [x] |
| 54 | cJSON_PrintBuffered | `!print_value(...)` (item NULL / Invalid type / Raw with NULL valuestring) | `NULL` | [x] |
| 55 | cJSON_PrintBuffered | **(alloc)** `allocate(prebuffer)` fails | `NULL` | [x] |
| 56 | cJSON_PrintPreallocated | `length < 0` | `false` (0) | [x] |
| 57 | cJSON_PrintPreallocated | `buffer == NULL` | `false` (0) | [x] |
| 58 | cJSON_PrintPreallocated | `!print_value(...)` — buffer too small; `ensure()` hits `p->noalloc` | `false` (0) | [x] |
| 59 | ensure (via Print*) | `(p == NULL) \|\| (p->buffer == NULL)` | `NULL` → print fails | [x] (via row 58) |
| 60 | ensure (via Print*) | `(p->length > 0) && (p->offset >= p->length)` — invalid offset | `NULL` → print fails | [x] (via row 58) |
| 61 | ensure (via Print*) | `needed > INT_MAX` | `NULL` → print fails | inspection (needs >2GB) |
| 62 | ensure (via Print*) | `p->noalloc` and growth required | `NULL` → print fails | [x] |
| 63 | print_number (via Print*) | `output_buffer == NULL` | `false` | [x] (via row 58) |
| 64 | print_number (via Print*) | `(length < 0) \|\| (length > 25)` — sprintf overrun | `false` | inspection (unreachable) |
| 65 | print_value (via Print*) | `(item == NULL) \|\| (output_buffer == NULL)` | `false` | [x] |
| 66 | print_value (via Print*) | `cJSON_Raw` with `item->valuestring == NULL` | `false` → `NULL`/`false` | [x] |
| 67 | print_value (via Print*) | `default:` — unknown/`cJSON_Invalid` type | `false` → `NULL`/`false` | [x] |
| 68 | cJSON_GetArraySize | `array == NULL` | `0` | [x] |
| 69 | cJSON_GetArrayItem | `index < 0` | `NULL` | [x] |
| 70 | cJSON_GetArrayItem | `array == NULL` | `NULL` | [x] |
| 71 | cJSON_GetArrayItem | `index >= size` — walked past end | `NULL` | [x] |
| 72 | cJSON_GetObjectItem / …CaseSensitive | `(object == NULL) \|\| (name == NULL)` | `NULL` | [x] |
| 73 | cJSON_GetObjectItem / …CaseSensitive | key not present (`current_element == NULL \|\| ->string == NULL`) | `NULL` | [x] |
| 74 | cJSON_HasObjectItem | `cJSON_GetObjectItem(...) == NULL` (NULL object, NULL string, or absent key) | `0` | [x] |
| 75 | cJSON_AddItemToArray | `item == NULL` | `false` | [x] |
| 76 | cJSON_AddItemToArray | `array == NULL` | `false` | [x] |
| 77 | cJSON_AddItemToArray | `array == item` — self reference | `false` | [x] |
| 78 | create_reference (via AddItemReference*) | `item == NULL` | `NULL` → add fails `false` | [x] |
| 79 | create_reference (via AddItemReference*) | **(alloc)** `cJSON_New_Item` fails | `NULL` | [x] |
| 80 | add_item_to_object (via AddItemToObject/CS, Add*ToObject) | `object == NULL` | `false` | [x] |
| 81 | add_item_to_object | `string == NULL` | `false` | [x] |
| 82 | add_item_to_object | `item == NULL` | `false` | [x] |
| 83 | add_item_to_object | `object == item` — self reference | `false` | [x] |
| 84 | add_item_to_object | **(alloc)** `cJSON_strdup(key)` fails (non-const key) | `false` | [x] |
| 85 | cJSON_AddItemReferenceToArray | `array == NULL` | `false` | [x] |
| 86 | cJSON_AddItemReferenceToObject | `object == NULL` | `false` | [x] |
| 87 | cJSON_AddItemReferenceToObject | `string == NULL` | `false` | [x] |
| 88 | cJSON_AddNullToObject | `add_item_to_object` fails (NULL object or NULL name) | `NULL` | [x] |
| 89 | cJSON_AddTrueToObject | `add_item_to_object` fails | `NULL` | [x] |
| 90 | cJSON_AddFalseToObject | `add_item_to_object` fails | `NULL` | [x] |
| 91 | cJSON_AddBoolToObject | `add_item_to_object` fails | `NULL` | [x] |
| 92 | cJSON_AddNumberToObject | `add_item_to_object` fails | `NULL` | [x] |
| 93 | cJSON_AddStringToObject | `add_item_to_object` fails | `NULL` | [x] |
| 94 | cJSON_AddStringToObject | `string == NULL` → `cJSON_CreateString(NULL)` → strdup(NULL) guard → item NULL | `NULL` | [x] |
| 95 | cJSON_AddRawToObject | `add_item_to_object` fails | `NULL` | [x] |
| 96 | cJSON_AddRawToObject | `raw == NULL` → `cJSON_CreateRaw(NULL)` fails | `NULL` | [x] |
| 97 | cJSON_AddObjectToObject | `add_item_to_object` fails | `NULL` | [x] |
| 98 | cJSON_AddArrayToObject | `add_item_to_object` fails | `NULL` | [x] |
| 99 | cJSON_DetachItemViaPointer | `parent == NULL` | `NULL` | [x] |
| 100 | cJSON_DetachItemViaPointer | `item == NULL` | `NULL` | [x] |
| 101 | cJSON_DetachItemViaPointer | `(item != parent->child) && (item->prev == NULL)` — not a member/corrupt | `NULL` | [x] |
| 102 | cJSON_DetachItemFromArray | `which < 0` | `NULL` | [x] |
| 103 | cJSON_DetachItemFromArray | `which >= size` → `get_array_item` NULL → detach NULL | `NULL` | [x] |
| 104 | cJSON_DetachItemFromObject / …CaseSensitive | object NULL / string NULL / key absent | `NULL` | [x] |
| 105 | cJSON_DeleteItemFromArray | `which` out of range → detach NULL → `cJSON_Delete(NULL)` | no-op (void) | [x] |
| 106 | cJSON_DeleteItemFromObject / …CaseSensitive | key absent / NULL args | no-op (void) | [x] |
| 107 | cJSON_InsertItemInArray | `which < 0` | `false` | [x] |
| 108 | cJSON_InsertItemInArray | `newitem == NULL` | `false` | [x] |
| 109 | cJSON_InsertItemInArray | `which >= size` (`after_inserted == NULL`) → falls back to `add_item_to_array` | `true` if array non-NULL & != newitem, else `false` | [x] |
| 110 | cJSON_InsertItemInArray | `(after_inserted != array->child) && (after_inserted->prev == NULL)` — corrupt | `false` | [x] |
| 111 | cJSON_ReplaceItemViaPointer | `parent == NULL` | `false` | [x] |
| 112 | cJSON_ReplaceItemViaPointer | `parent->child == NULL` (empty container) | `false` | [x] |
| 113 | cJSON_ReplaceItemViaPointer | `replacement == NULL` | `false` | [x] |
| 114 | cJSON_ReplaceItemViaPointer | `item == NULL` | `false` | [x] |
| 115 | cJSON_ReplaceItemViaPointer | `replacement == item` — self replace | `true` (no-op success) | [x] |
| 116 | cJSON_ReplaceItemInArray | `which < 0` | `false` | [x] |
| 117 | cJSON_ReplaceItemInArray | `which >= size` → item NULL | `false` | [x] |
| 118 | replace_item_in_object (via ReplaceItemInObject/CS) | `replacement == NULL` | `false` | [x] |
| 119 | replace_item_in_object | `string == NULL` | `false` | [x] |
| 120 | replace_item_in_object | key absent → `get_object_item` NULL | `false` | [x] |
| 121 | replace_item_in_object | **(alloc)** `cJSON_strdup(string)` fails | `false` | [x] |
| 122 | cJSON_CreateString | `string == NULL` → `cJSON_strdup(NULL)` returns NULL | `NULL` (item deleted) | [x] |
| 123 | cJSON_CreateRaw | `raw == NULL` → `cJSON_strdup(NULL)` returns NULL | `NULL` (item deleted) | [x] |
| 124 | cJSON_CreateNull/True/False/Bool/Number/Array/Object/*Reference | **(alloc)** `cJSON_New_Item` fails | `NULL` | [x] |
| 125 | cJSON_CreateIntArray | `count < 0` | `NULL` | [x] |
| 126 | cJSON_CreateIntArray | `numbers == NULL` | `NULL` | [x] |
| 127 | cJSON_CreateFloatArray | `count < 0` | `NULL` | [x] |
| 128 | cJSON_CreateFloatArray | `numbers == NULL` | `NULL` | [x] |
| 129 | cJSON_CreateDoubleArray | `count < 0` | `NULL` | [x] |
| 130 | cJSON_CreateDoubleArray | `numbers == NULL` | `NULL` | [x] |
| 131 | cJSON_CreateStringArray | `count < 0` | `NULL` | [x] |
| 132 | cJSON_CreateStringArray | `strings == NULL` | `NULL` | [x] |
| 133 | cJSON_CreateStringArray | element string is NULL → `cJSON_CreateString(NULL)` NULL → `!n` | `NULL` (array deleted) | [x] |
| 134 | cJSON_Create*Array | **(alloc)** `cJSON_CreateNumber` fails mid-loop (`!n`) | `NULL` | [x] |
| 135 | cJSON_Duplicate | `item == NULL` (`!item` → fail) | `NULL` | [x] |
| 136 | cJSON_Duplicate | **(alloc)** `cJSON_New_Item` fails | `NULL` | [x] |
| 137 | cJSON_Duplicate | String/Raw with non-NULL valuestring, **(alloc)** strdup fails | `NULL` | [x] |
| 138 | cJSON_Duplicate | `item->string != NULL`, **(alloc)** strdup fails | `NULL` | [x] |
| 139 | cJSON_Duplicate | `depth >= CJSON_CIRCULAR_LIMIT` (10000) — circular/too deep | `NULL` | [x] |
| 140 | cJSON_Duplicate | recursive child duplicate fails (`!newchild`) | `NULL` | [x] (via 139) |
| 141 | cJSON_Minify | `json == NULL` | early return (void, no crash) | [x] |
| 142 | cJSON_IsInvalid | `item == NULL` | `false` (0) | [x] |
| 143 | cJSON_IsFalse | `item == NULL` | `false` (0) | [x] |
| 144 | cJSON_IsTrue | `item == NULL` | `false` (0) | [x] |
| 145 | cJSON_IsBool | `item == NULL` | `false` (0) | [x] |
| 146 | cJSON_IsNull | `item == NULL` | `false` (0) | [x] |
| 147 | cJSON_IsNumber | `item == NULL` | `false` (0) | [x] |
| 148 | cJSON_IsString | `item == NULL` | `false` (0) | [x] |
| 149 | cJSON_IsArray | `item == NULL` | `false` (0) | [x] |
| 150 | cJSON_IsObject | `item == NULL` | `false` (0) | [x] |
| 151 | cJSON_IsRaw | `item == NULL` | `false` (0) | [x] |
| 152 | cJSON_Compare | `a == NULL` | `false` | [x] |
| 153 | cJSON_Compare | `b == NULL` | `false` | [x] |
| 154 | cJSON_Compare | `(a->type & 0xFF) != (b->type & 0xFF)` | `false` | [x] |
| 155 | cJSON_Compare | validity switch `default:` — type byte is not a valid cJSON type (out-of-range enum) | `false` | [x] |
| 156 | cJSON_Compare | Number: `!compare_double(a->valuedouble, b->valuedouble)` | `false` | [x] |
| 157 | cJSON_Compare | String/Raw: `a->valuestring == NULL \|\| b->valuestring == NULL` | `false` | [x] |
| 158 | cJSON_Compare | String/Raw: `strcmp(a,b) != 0` | `false` | [x] |
| 159 | cJSON_Compare | Array: element mismatch | `false` | [x] |
| 160 | cJSON_Compare | Array: `a_element != b_element` — different lengths | `false` | [x] |
| 161 | cJSON_Compare | Object: key of `a` absent in `b` (`b_element == NULL`) | `false` | [x] |
| 162 | cJSON_Compare | Object: value mismatch a→b | `false` | [x] |
| 163 | cJSON_Compare | Object: key of `b` absent in `a` (`a_element == NULL`) | `false` | [x] |
| 164 | cJSON_Compare | Object: value mismatch b→a | `false` | [x] |
| 165 | cJSON_Compare | final switch `default:` (unhandled valid type byte) | `false` | [x] |
| 166 | cJSON_SetNumberHelper | `object == NULL` — **no guard**, C dereferences NULL | UB / SIGSEGV (not tested; both sides must not "handle" it) | n/a |
| 167 | cJSON_Delete | `item == NULL` | no-op (void) | [x] |
| 168 | cJSON_InitHooks | `hooks == NULL` | resets to malloc/free/realloc, void | [x] |
| 169 | cJSON_free | `object == NULL` | `free(NULL)`, no-op | [x] |
| 170 | cJSON_malloc | `size == 0` | implementation-defined non-NULL/NULL from malloc(0) | [x] |

## Generic FFI-boundary boundaries also covered

| # | condition | covered by |
|---|-----------|-----------|
| G1 | NULL pointer to every pointer-taking public fn | tests `error_paths::null_pointers_everywhere` |
| G2 | zero length (`cJSON_ParseWithLength(s,0)`, `cJSON_PrintPreallocated(item,buf,0,f)`) | rows 12, 56/58 |
| G3 | oversized/negative length (`prebuffer = INT_MIN/-1/INT_MAX`, `length = -1`) | rows 53, 56 |
| G4 | one past valid range (`index = size`, `which = size`, `count = 0`) | rows 71, 103, 117 |
| G5 | **out-of-range enum across FFI**: `item->type` set to values with no valid variant (`0x00`, `0x07`, `0x40`, `0xFF`, `0x1000`, `-1`) then passed to `cJSON_Print*`, `cJSON_Compare`, `cJSON_Is*`, `cJSON_Duplicate`, `cJSON_Delete` | tests `error_paths::out_of_range_type_enum` |
| G6 | `cJSON_bool` arguments given non-0/1 ints (`2`, `-1`, `0x100`) — C treats any non-zero as true | tests `error_paths::nonboolean_bool_args` |
| G7 | `cJSON_InitHooks` with a `cJSON_Hooks` whose fns are NULL / partially set | row 168 + configs |
