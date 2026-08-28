# Error surface

Mechanical source: every public-function null/range/type guard and every
reachable `return false`, `return NULL`, `goto fail`, nesting limit, and
allocation-failure branch in `../c_src/cJSON.c`. There are no `assert` calls or
public enum parameters in this API. Boolean parameters are C `int` values and
therefore include out-of-range values; those cases are listed explicitly.

Allocation-failure rows are driven with `cJSON_InitHooks` and a deterministic
fail-after allocator. Rows for malformed JSON also compare the
`return_parse_end`/`cJSON_GetErrorPtr` byte offset.

| # | function | trigger (exact invalid input/condition) | expected C result | verified |
|---|----------|-----------------------------------------|-------------------|----------|
| E01 | `cJSON_Parse`, `cJSON_ParseWithOpts` | `value == NULL` | `NULL` | [x] |
| E02 | `cJSON_ParseWithLength`, `cJSON_ParseWithLengthOpts` | `value == NULL` with nonzero length | `NULL`; parse-end untouched for `ParseWithLengthOpts` | [x] |
| E03 | `cJSON_ParseWithLength`, `cJSON_ParseWithLengthOpts` | `buffer_length == 0` with nonnull value | `NULL`; error offset 0 | [x] |
| E04 | all parse entry points | root `cJSON` allocation fails | `NULL`; error offset 0 | [x] |
| E05 | all parse entry points | first non-whitespace byte is not a JSON value token | `NULL`; error points at that byte | [x] |
| E06 | all parse entry points | input contains only whitespace | `NULL`; error points at final supplied byte | [x] |
| E07 | all parse entry points | number temporary-buffer allocation fails | `NULL`; error points at number start | [x] |
| E08 | all parse entry points | number begins with `-` but `strtod` consumes no byte | `NULL`; error points at number start | [x] |
| E09 | all parse entry points | string has no closing quote before supplied length | `NULL`; error points immediately after opening quote | [x] |
| E10 | all parse entry points | final supplied string byte is a backslash | `NULL`; error points immediately after opening quote | [x] |
| E11 | all parse entry points | decoded-string allocation fails | `NULL`; error points immediately after opening quote | [x] |
| E12 | all parse entry points | string uses an escape other than `"\\/bfnrtu` | `NULL`; error points at backslash | [x] |
| E13 | all parse entry points | `\u` sequence has fewer than four hex positions before closing quote | `NULL`; error points at backslash | [x] |
| E14 | all parse entry points | first UTF-16 code unit is an unpaired low surrogate `DC00..DFFF` | `NULL`; error points at backslash | [x] |
| E15 | all parse entry points | high surrogate `D800..DBFF` lacks a complete second `\uXXXX` | `NULL`; error points at first backslash | [x] |
| E16 | all parse entry points | high surrogate is followed by six bytes not beginning `\u` | `NULL`; error points at first backslash | [x] |
| E17 | all parse entry points | second surrogate is outside `DC00..DFFF` | `NULL`; error points at first backslash | [x] |
| E18 | all parse entry points | array nesting reaches `CJSON_NESTING_LIMIT` (1000) | `NULL`; error at the 1001st `[` | [x] |
| E19 | all parse entry points | object nesting reaches `CJSON_NESTING_LIMIT` (1000) | `NULL`; error at the 1001st `{` | [x] |
| E20 | all parse entry points | array ends after `[` or whitespace without `]` | `NULL`; error at opening/last byte per length | [x] |
| E21 | all parse entry points | array child allocation fails | `NULL`; error at child position | [x] |
| E22 | all parse entry points | array element is missing or malformed after `[` or `,` | `NULL`; error at element position | [x] |
| E23 | all parse entry points | parsed array element is not followed by `,` or `]` | `NULL`; error at unexpected byte | [x] |
| E24 | all parse entry points | object ends after `{` or whitespace without `}` | `NULL`; error at opening/last byte per length | [x] |
| E25 | all parse entry points | object child allocation fails | `NULL`; error at child position | [x] |
| E26 | all parse entry points | object comma has no following byte | `NULL`; error at comma/final byte | [x] |
| E27 | all parse entry points | object key is absent, unquoted, or malformed | `NULL`; error at key position | [x] |
| E28 | all parse entry points | object key is not followed by `:` | `NULL`; error at unexpected byte | [x] |
| E29 | all parse entry points | object value is absent or malformed | `NULL`; error at value position | [x] |
| E30 | all parse entry points | parsed object member is not followed by `,` or `}` | `NULL`; error at unexpected byte | [x] |
| E31 | `cJSON_ParseWithOpts`, `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and non-whitespace trailing data remains | `NULL`; error at trailing byte | [x] |
| E32 | `cJSON_Print`, `cJSON_PrintUnformatted` | `item == NULL` | `NULL` | [x] |
| E33 | `cJSON_Print`, `cJSON_PrintUnformatted` | initial 256-byte allocation fails | `NULL` | [x] |
| E34 | all print entry points | `(item->type & 0xff)` is not one of the eight printable types | allocated printers return `NULL`; preallocated returns 0 | [x] |
| E35 | all print entry points | raw item has `valuestring == NULL` | allocated printers return `NULL`; preallocated returns 0 | [x] |
| E36 | allocated print entry points | growth allocation/reallocation fails | `NULL`, intermediate buffer freed | [x] |
| E37 | `cJSON_Print`, `cJSON_PrintUnformatted` | final shrink/copy allocation fails | `NULL`, intermediate buffer freed | [x] |
| E38 | `cJSON_PrintBuffered` | `prebuffer < 0` | `NULL` | [x] |
| E39 | `cJSON_PrintBuffered` | initial allocation for `prebuffer` fails | `NULL` | [x] |
| E40 | `cJSON_PrintPreallocated` | `buffer == NULL` | 0 | [x] |
| E41 | `cJSON_PrintPreallocated` | `length < 0` | 0 | [x] |
| E42 | `cJSON_PrintPreallocated` | buffer is too short for scalar/string/array/object output | 0 | [x] |
| E43 | `cJSON_GetStringValue` | item is `NULL` or not exact base type `cJSON_String` | `NULL` | [x] |
| E44 | `cJSON_GetNumberValue` | item is `NULL` or not exact base type `cJSON_Number` | NaN | [x] |
| E45 | all ten `cJSON_Is*` predicates | `item == NULL` | 0 | [x] |
| E46 | each exact-type `cJSON_Is*` predicate | low type byte differs, including out-of-range type values | 0 | [x] |
| E47 | `cJSON_SetValuestring` | `object == NULL` | `NULL` | [x] |
| E48 | `cJSON_SetValuestring` | object lacks `cJSON_String` bit | `NULL`, unchanged | [x] |
| E49 | `cJSON_SetValuestring` | object has `cJSON_IsReference` bit | `NULL`, unchanged | [x] |
| E50 | `cJSON_SetValuestring` | `object->valuestring == NULL` | `NULL` | [x] |
| E51 | `cJSON_SetValuestring` | replacement `valuestring == NULL` | `NULL`, unchanged | [x] |
| E52 | `cJSON_SetValuestring` | shorter/equal replacement overlaps old allocation, including same pointer | `NULL`, unchanged | [x] |
| E53 | `cJSON_SetValuestring` | longer replacement duplication allocation fails | `NULL`, unchanged | [x] |
| E54 | `cJSON_CreateString`, `cJSON_CreateRaw` | source pointer is `NULL` | `NULL` | [x] |
| E55 | all scalar/container constructors | item allocation fails | `NULL` | [x] |
| E56 | `cJSON_CreateString`, `cJSON_CreateRaw` | value duplication allocation fails after item allocation | `NULL`, item freed | [x] |
| E57 | four typed array constructors | `count < 0` | `NULL` | [x] |
| E58 | four typed array constructors | source array pointer is `NULL`, including count 0 | `NULL` | [x] |
| E59 | four typed array constructors | container allocation fails | `NULL` | [x] |
| E60 | four typed array constructors | an element allocation/value duplication fails | `NULL`, partial array freed | [x] |
| E61 | `cJSON_AddItemToArray` | array or item is `NULL`, or `array == item` | 0 | [x] |
| E62 | `cJSON_AddItemToObject`, `cJSON_AddItemToObjectCS` | object, key, or item is `NULL`, or `object == item` | 0 | [x] |
| E63 | `cJSON_AddItemToObject` | key duplication allocation fails | 0, item remains unattached | [x] |
| E64 | `cJSON_AddItemReferenceToArray` | array or referenced item is `NULL` | 0 | [x] |
| E65 | `cJSON_AddItemReferenceToArray` | reference-node allocation fails | 0 | [x] |
| E66 | `cJSON_AddItemReferenceToObject` | object, key, or referenced item is `NULL` | 0 | [x] |
| E67 | `cJSON_AddItemReferenceToObject` | reference-node or key allocation fails | 0 | [x] |
| E68 | nine `cJSON_Add*ToObject` helpers | object or name is `NULL` | `NULL`, newly created item freed | [x] |
| E69 | string/raw object-add helpers | value pointer is `NULL` | `NULL` | [x] |
| E70 | nine `cJSON_Add*ToObject` helpers | constructor or key allocation fails | `NULL`, temporary item freed | [x] |
| E71 | `cJSON_GetArraySize` | array is `NULL` | 0 | [x] |
| E72 | `cJSON_GetArrayItem` | array is `NULL`, index is negative, or index is at/past count | `NULL` | [x] |
| E73 | object get/has entry points | object or key is `NULL`, or no matching/non-null key exists | getters return `NULL`; has returns 0 | [x] |
| E74 | `cJSON_DetachItemViaPointer` | parent/item is `NULL`, or non-head item has `prev == NULL` | `NULL` | [x] |
| E75 | array detach/delete entry points | negative or out-of-range index, or null array | detach returns `NULL`; delete is a no-op | [x] |
| E76 | object detach/delete entry points | null/missing key or null object | detach returns `NULL`; delete is a no-op | [x] |
| E77 | `cJSON_InsertItemInArray` | `which < 0` or `newitem == NULL` | 0 | [x] |
| E78 | `cJSON_InsertItemInArray` | null array (so lookup and append both reject) | 0 | [x] |
| E79 | `cJSON_InsertItemInArray` | located non-head item has corrupted `prev == NULL` | 0 | [x] |
| E80 | `cJSON_ReplaceItemViaPointer` | parent/item/replacement is `NULL`, or parent has no child | 0 | [x] |
| E81 | `cJSON_ReplaceItemInArray` | negative/out-of-range index or null array/new item | 0 | [x] |
| E82 | object replace entry points | replacement or key is `NULL` | 0 | [x] |
| E83 | object replace entry points | replacement key duplication fails | 0 | [x] |
| E84 | object replace entry points | key does not select an item, including case mismatch in sensitive mode | 0 | [x] |
| E85 | `cJSON_Duplicate` | source item is `NULL` | `NULL` | [x] |
| E86 | `cJSON_Duplicate` | root/child/string/key allocation fails | `NULL`, partial duplicate freed | [x] |
| E87 | `cJSON_Duplicate` | recursive child chain reaches `CJSON_CIRCULAR_LIMIT` (10000) | `NULL`, partial duplicate freed | [x] |
| E88 | `cJSON_Compare` | either pointer is `NULL`, low type bytes differ, or type is invalid/out of range | 0 | [x] |
| E89 | `cJSON_Compare` | number values differ outside cJSON epsilon rule | 0 | [x] |
| E90 | `cJSON_Compare` | string/raw value pointer is `NULL` or bytes differ | 0 | [x] |
| E91 | `cJSON_Compare` | arrays differ by an element or length | 0 | [x] |
| E92 | `cJSON_Compare` | objects have a missing/different member under selected case mode | 0 | [x] |
| E93 | `cJSON_Minify` | input pointer is `NULL` | no-op | [x] |
| E94 | `cJSON_CreateBool`, parse/print/duplicate/compare option integers | boolean/mode argument is outside `{0,1}` | C truthiness: zero is false, every nonzero value is true | [x] |
| E95 | `cJSON_Delete`, `cJSON_free` | pointer is `NULL` | no-op | [x] |
| E96 | `cJSON_InitHooks` | hook structure is `NULL` | reset to libc `malloc/free/realloc`; no error | [x] |
