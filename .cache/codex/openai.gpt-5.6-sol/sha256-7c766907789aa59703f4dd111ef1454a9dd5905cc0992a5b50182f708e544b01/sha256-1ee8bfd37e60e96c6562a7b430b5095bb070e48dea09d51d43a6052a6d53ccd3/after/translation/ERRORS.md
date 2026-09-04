# Error Surface

Rows are derived from explicit rejection branches, null/range checks, and the
two public limits in `cJSON.c`/`cJSON.h`. Allocation-failure rows are exercised
with `cJSON_InitHooks`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `cJSON_GetStringValue` | `item == NULL` or low-byte type is not `cJSON_String` | `NULL` | [x] |
| 2 | `cJSON_GetNumberValue` | `item == NULL` or low-byte type is not `cJSON_Number` | `NaN` | [x] |
| 3 | `cJSON_SetValuestring` | `object == NULL` | `NULL` | [x] |
| 4 | `cJSON_SetValuestring` | object lacks `cJSON_String` bit | `NULL` | [x] |
| 5 | `cJSON_SetValuestring` | object has `cJSON_IsReference` bit | `NULL` | [x] |
| 6 | `cJSON_SetValuestring` | existing `object->valuestring == NULL` | `NULL` | [x] |
| 7 | `cJSON_SetValuestring` | new `valuestring == NULL` | `NULL` | [x] |
| 8 | `cJSON_SetValuestring` | source overlaps destination in in-place branch | `NULL` | [x] |
| 9 | `cJSON_SetValuestring` | duplication allocation fails | `NULL`, old value retained | [x] |
| 10 | parse number | parse buffer/content is null | reject (`false`) | [x] |
| 11 | parse number | temporary number allocation fails | parse returns `NULL` | [x] |
| 12 | parse number | `strtod` consumes no byte, e.g. `"-"` | parse returns `NULL` | [x] |
| 13 | parse string | opening byte is not `"` when string parser is entered | reject (`false`) | [x] |
| 14 | parse string | backslash is final accessible byte | parse returns `NULL` | [x] |
| 15 | parse string | closing quote is absent | parse returns `NULL` | [x] |
| 16 | parse string | output allocation fails | parse returns `NULL` | [x] |
| 17 | parse string | escape is not one of `"\\/bfnrtu` | parse returns `NULL` | [x] |
| 18 | UTF-16 escape | fewer than six bytes remain for `\\uXXXX` | parse returns `NULL` | [x] |
| 19 | UTF-16 escape | first code unit is an unpaired low surrogate | parse returns `NULL` | [x] |
| 20 | UTF-16 escape | high surrogate has no complete second escape | parse returns `NULL` | [x] |
| 21 | UTF-16 escape | high surrogate is not followed by `\\u` | parse returns `NULL` | [x] |
| 22 | UTF-16 escape | second code unit is not a low surrogate | parse returns `NULL` | [x] |
| 23 | `cJSON_ParseWithOpts` | `value == NULL` | `NULL` | [x] |
| 24 | `cJSON_ParseWithLengthOpts` | `value == NULL` | `NULL`; parse end unchanged | [x] |
| 25 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | `NULL`; error at input | [x] |
| 26 | `cJSON_ParseWithLengthOpts` | root item allocation fails | `NULL` | [x] |
| 27 | parse value | first token is not null/false/true/string/number/array/object | `NULL` | [x] |
| 28 | parse array | depth is `CJSON_NESTING_LIMIT` (1000) | `NULL` | [x] |
| 29 | parse array | input ends after `[`/whitespace | `NULL` | [x] |
| 30 | parse array | child allocation fails | `NULL` | [x] |
| 31 | parse array | an element fails to parse | `NULL` | [x] |
| 32 | parse array | closing `]` is missing/wrong | `NULL` | [x] |
| 33 | parse object | depth is `CJSON_NESTING_LIMIT` (1000) | `NULL` | [x] |
| 34 | parse object | input ends after `{`/whitespace | `NULL` | [x] |
| 35 | parse object | member allocation fails | `NULL` | [x] |
| 36 | parse object | key is missing or malformed | `NULL` | [x] |
| 37 | parse object | colon after key is missing | `NULL` | [x] |
| 38 | parse object | member value fails to parse | `NULL` | [x] |
| 39 | parse object | closing `}` is missing/wrong | `NULL` | [x] |
| 40 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and parsed value is not followed by NUL after whitespace | `NULL` | [x] |
| 41 | print core | `item == NULL` | `NULL`/`false` | [x] |
| 42 | print core | low-byte item type is unsupported/out-of-range | `NULL`/`false` | [x] |
| 43 | print raw | raw item has `valuestring == NULL` | `NULL`/`false` | [x] |
| 44 | print allocation | initial output allocation fails | `NULL` | [x] |
| 45 | print allocation | growth/reallocation fails | `NULL` and old buffer freed | [x] |
| 46 | `cJSON_PrintBuffered` | `prebuffer < 0` | `NULL` | [x] |
| 47 | `cJSON_PrintBuffered` | zero prebuffer and allocator returns `NULL` for size zero | `NULL` | [x] |
| 48 | `cJSON_PrintPreallocated` | `length < 0` | `false` | [x] |
| 49 | `cJSON_PrintPreallocated` | `buffer == NULL` | `false` | [x] |
| 50 | `cJSON_PrintPreallocated` | supplied buffer is too short (`noalloc` growth request) | `false` | [x] |
| 51 | `cJSON_GetArrayItem` | `array == NULL` | `NULL` | [x] |
| 52 | `cJSON_GetArrayItem` | `index < 0` | `NULL` | [x] |
| 53 | `cJSON_GetArrayItem` | index is at/past item count | `NULL` | [x] |
| 54 | object lookup family | object or key pointer is `NULL` | `NULL`/`false` | [x] |
| 55 | object lookup family | no key matches or encountered item has null key | `NULL`/`false` | [x] |
| 56 | `cJSON_AddItemToArray` | array/item null, or `array == item` | `false` | [x] |
| 57 | object add family | object/key/item null, or `object == item` | `false`/`NULL` | [x] |
| 58 | object add family | copied-key allocation fails | `false`/`NULL` | [x] |
| 59 | reference add family | referenced item is `NULL` or reference allocation fails | `false` | [x] |
| 60 | `cJSON_DetachItemViaPointer` | parent/item null, or non-head item has `prev == NULL` | `NULL` | [x] |
| 61 | array detach/delete family | `which < 0` or index is absent | `NULL`/no-op | [x] |
| 62 | object detach/delete family | key null/absent | `NULL`/no-op | [x] |
| 63 | `cJSON_InsertItemInArray` | `which < 0` or `newitem == NULL` | `false` | [x] |
| 64 | `cJSON_InsertItemInArray` | located non-head item has `prev == NULL` | `false` | [x] |
| 65 | `cJSON_ReplaceItemViaPointer` | parent null, parent child null, item null, or replacement null | `false` | [x] |
| 66 | `cJSON_ReplaceItemInArray` | `which < 0` or index absent | `false` | [x] |
| 67 | object replace family | replacement or key is null | `false` | [x] |
| 68 | object replace family | replacement key duplication fails | `false` | [x] |
| 69 | scalar/string/raw constructors | item allocation fails | `NULL` | [x] |
| 70 | `cJSON_CreateString` | source string is `NULL` | `NULL` | [x] |
| 71 | `cJSON_CreateRaw` | source raw string is `NULL` | `NULL` | [x] |
| 72 | numeric array constructors | `count < 0` | `NULL` | [x] |
| 73 | numeric array constructors | numbers pointer is `NULL`, including count zero | `NULL` | [x] |
| 74 | `cJSON_CreateStringArray` | `count < 0` | `NULL` | [x] |
| 75 | `cJSON_CreateStringArray` | strings pointer is `NULL`, including count zero | `NULL` | [x] |
| 76 | array constructors | child allocation fails | `NULL`, partial array deleted | [x] |
| 77 | `cJSON_Duplicate` | input item is `NULL` | `NULL` | [x] |
| 78 | `cJSON_Duplicate` | item/string/key allocation fails | `NULL`, partial duplicate deleted | [x] |
| 79 | `cJSON_Duplicate` | recursive chain reaches `CJSON_CIRCULAR_LIMIT` (10000) | `NULL` | [x] |
| 80 | type predicates | item is `NULL` | `false` | [x] |
| 81 | `cJSON_Compare` | either item null or low-byte types differ | `false` | [x] |
| 82 | `cJSON_Compare` | either low-byte type is invalid/out-of-range | `false` | [x] |
| 83 | `cJSON_Compare` | numbers differ beyond epsilon rule | `false` | [x] |
| 84 | `cJSON_Compare` | string/raw value pointer null or bytes differ | `false` | [x] |
| 85 | `cJSON_Compare` | arrays differ by element or length | `false` | [x] |
| 86 | `cJSON_Compare` | objects differ by key set or member value | `false` | [x] |
| 87 | `cJSON_Minify` | input pointer is `NULL` | no-op | [x] |
