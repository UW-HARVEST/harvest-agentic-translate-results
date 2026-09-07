# Error-surface table

Mechanically derived from public-entry rejection checks and the internal
`return false`, `return NULL`, `goto fail`, range, depth, and null checks they
reach in `cJSON.c`. Allocation-failure rows are driven through
`cJSON_InitHooks`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---:|---|---|---|:---:|
| 1 | `cJSON_GetStringValue` | item is NULL or low-byte type is not `cJSON_String` | NULL | [x] |
| 2 | `cJSON_GetNumberValue` | item is NULL or low-byte type is not `cJSON_Number` | NaN | [x] |
| 3 | `cJSON_SetValuestring` | object is NULL | NULL | [x] |
| 4 | `cJSON_SetValuestring` | object lacks the `cJSON_String` bit | NULL | [x] |
| 5 | `cJSON_SetValuestring` | object has `cJSON_IsReference` | NULL | [x] |
| 6 | `cJSON_SetValuestring` | current `object->valuestring` is NULL | NULL | [x] |
| 7 | `cJSON_SetValuestring` | replacement `valuestring` is NULL | NULL | [x] |
| 8 | `cJSON_SetValuestring` | shorter/equal replacement overlaps current storage | NULL | [x] |
| 9 | `cJSON_SetValuestring` | longer replacement duplication allocation fails | NULL, original retained | [x] |
| 10 | `cJSON_ParseWithOpts` | `value == NULL` | NULL | [x] |
| 11 | `cJSON_ParseWithLengthOpts` | `value == NULL` | NULL; error pointer remains/reset as C specifies | [x] |
| 12 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | NULL; parse-end points at input start | [x] |
| 13 | parse entry points | first non-whitespace byte is not a recognized JSON value | NULL; exact error offset | [x] |
| 14 | parse entry points | only whitespace is available | NULL; exact final-byte error offset | [x] |
| 15 | parse entry points | string is missing closing quote | NULL; error at end | [x] |
| 16 | parse entry points | string ends with a backslash | NULL; error at backslash/end | [x] |
| 17 | parse entry points | string contains an unknown escape | NULL; error at escape | [x] |
| 18 | parse entry points | `\u` has fewer than four hex digits | NULL | [x] |
| 19 | parse entry points | `\uXXXX` contains a non-hex digit (hex parser yields zero behavior included) | same NULL/success behavior and bytes as C | [x] |
| 20 | parse entry points | first UTF-16 unit is an unpaired low surrogate `DC00..DFFF` | NULL | [x] |
| 21 | parse entry points | high surrogate `D800..DBFF` has no complete second `\uXXXX` | NULL | [x] |
| 22 | parse entry points | high surrogate is followed by something other than `\u` | NULL | [x] |
| 23 | parse entry points | second surrogate is outside `DC00..DFFF` | NULL | [x] |
| 24 | parse entry points | array input ends immediately after `[`/whitespace | NULL | [x] |
| 25 | parse entry points | array contains missing/invalid element after comma | NULL | [x] |
| 26 | parse entry points | array lacks closing `]` | NULL | [x] |
| 27 | parse entry points | object input ends immediately after `{`/whitespace | NULL | [x] |
| 28 | parse entry points | object comma is not followed by a name | NULL | [x] |
| 29 | parse entry points | object key is not a valid quoted string | NULL | [x] |
| 30 | parse entry points | object key is not followed by `:` | NULL | [x] |
| 31 | parse entry points | object value is missing/invalid | NULL | [x] |
| 32 | parse entry points | object lacks closing `}` | NULL | [x] |
| 33 | parse entry points | array/object nesting reaches `CJSON_NESTING_LIMIT` (1000) | NULL | [x] |
| 34 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and parsed JSON is followed by non-whitespace/non-NUL garbage | NULL at first garbage byte | [x] |
| 35 | parse entry points | root-item or parse temporary allocation fails | NULL | [x] |
| 36 | `cJSON_Print`, `cJSON_PrintUnformatted` | item is NULL | NULL | [x] |
| 37 | print entry points | item low-byte type is invalid/out-of-range | NULL/0 | [x] |
| 38 | print entry points | raw item has NULL `valuestring` | NULL/0 | [x] |
| 39 | print entry points | output allocation/reallocation fails | NULL/0 | [x] |
| 40 | `cJSON_PrintBuffered` | `prebuffer < 0` | NULL | [x] |
| 41 | `cJSON_PrintBuffered` | initial buffer allocation fails, including zero-size allocator returning NULL | NULL | [x] |
| 42 | `cJSON_PrintPreallocated` | `length < 0` | 0 | [x] |
| 43 | `cJSON_PrintPreallocated` | `buffer == NULL` | 0 | [x] |
| 44 | `cJSON_PrintPreallocated` | buffer length is zero or insufficient (`noalloc` ensure failure) | 0 | [x] |
| 45 | `cJSON_GetArraySize` | array is NULL | 0 | [x] |
| 46 | `cJSON_GetArrayItem` | array is NULL | NULL | [x] |
| 47 | `cJSON_GetArrayItem` | index is negative | NULL | [x] |
| 48 | `cJSON_GetArrayItem` | index is at/past child count | NULL | [x] |
| 49 | object lookup/has entry points | object or key is NULL | NULL/0 | [x] |
| 50 | object lookup entry points | key is absent or encountered child has NULL key | NULL | [x] |
| 51 | `cJSON_AddItemToArray` | array/item is NULL or `array == item` | 0 | [x] |
| 52 | `cJSON_AddItemToObject`, `cJSON_AddItemToObjectCS` | object/key/item is NULL or `object == item` | 0 | [x] |
| 53 | `cJSON_AddItemToObject` | key duplication allocation fails | 0 | [x] |
| 54 | `cJSON_AddItemReferenceToArray` | array or referenced item is NULL / reference allocation fails | 0 | [x] |
| 55 | `cJSON_AddItemReferenceToObject` | object/key/item is NULL / reference or key allocation fails | 0 | [x] |
| 56 | all `cJSON_Add*ToObject` convenience functions | object or name is NULL, child creation fails, or add fails | NULL | [x] |
| 57 | `cJSON_DetachItemViaPointer` | parent/item NULL, item is neither first child nor linked with `prev` | NULL | [x] |
| 58 | detach/delete array/object entry points | negative/out-of-range index or absent/NULL key | NULL/no-op | [x] |
| 59 | `cJSON_InsertItemInArray` | `which < 0` or `newitem == NULL` | 0 | [x] |
| 60 | `cJSON_InsertItemInArray` | array is NULL and append fallback is reached | 0 | [x] |
| 61 | `cJSON_InsertItemInArray` | target is non-head with NULL `prev` (corrupt chain) | 0 | [x] |
| 62 | `cJSON_ReplaceItemViaPointer` | parent NULL, parent child NULL, item NULL, or replacement NULL | 0 | [x] |
| 63 | `cJSON_ReplaceItemInArray` | negative/out-of-range index or NULL replacement | 0 | [x] |
| 64 | object replace entry points | key or replacement NULL, key duplication fails, or key absent | 0 | [x] |
| 65 | `cJSON_CreateString`, `cJSON_CreateRaw` | source is NULL or item/string allocation fails | NULL | [x] |
| 66 | scalar/container constructors | item allocation fails | NULL | [x] |
| 67 | array constructors | count is negative | NULL | [x] |
| 68 | array constructors | input pointer is NULL (including count zero) | NULL | [x] |
| 69 | array constructors | child allocation/string duplication fails | NULL and partial array deleted | [x] |
| 70 | `cJSON_Duplicate` | item is NULL | NULL | [x] |
| 71 | `cJSON_Duplicate` | node/string/key allocation fails | NULL | [x] |
| 72 | `cJSON_Duplicate` | recursive child traversal reaches `CJSON_CIRCULAR_LIMIT` (10000) | NULL | [x] |
| 73 | all `cJSON_Is*` predicates | item is NULL | 0 | [x] |
| 74 | `cJSON_Compare` | either input NULL, low-byte types differ, or low-byte type is invalid/out-of-range | 0 | [x] |
| 75 | `cJSON_Compare` | unequal numbers/strings/raw values, NULL string value, array length/content mismatch, or object key/value mismatch | 0 | [x] |
| 76 | `cJSON_Minify` | input is NULL | no-op | [x] |
| 77 | `cJSON_malloc` | allocator rejects requested size (including oversized size) | NULL | [x] |
| 78 | `cJSON_free`, `cJSON_Delete` | input is NULL | no-op | [x] |

Completion check:

- [x] Every row above has a passing C-vs-Rust differential assertion.
