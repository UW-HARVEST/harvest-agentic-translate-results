# Error Surface

Rows are derived from explicit rejection branches in `c_src/cJSON.c`. Internal
branches are listed under the public entry points through which they are
reachable. Allocation failures are injected with `cJSON_InitHooks`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| E001 | `cJSON_GetStringValue` | `item == NULL` | `NULL` | [x] |
| E002 | `cJSON_GetStringValue` | low-byte type is not `cJSON_String` | `NULL` | [x] |
| E003 | `cJSON_GetNumberValue` | `item == NULL` | NaN | [x] |
| E004 | `cJSON_GetNumberValue` | low-byte type is not `cJSON_Number` | NaN | [x] |
| E005 | `cJSON_ParseWithOpts` | `value == NULL` | `NULL` | [x] |
| E006 | `cJSON_ParseWithLengthOpts` | `value == NULL` | `NULL`; error pointer remains null | [x] |
| E007 | `cJSON_ParseWithLengthOpts` | `buffer_length == 0` | `NULL`; parse end points at input | [x] |
| E008 | parse family | no token matches at current offset | `NULL`; error at token | [x] |
| E009 | parse family | number starts with `-` but `strtod` consumes nothing | `NULL` | [x] |
| E010 | parse family | opening string has no closing quote in supplied length | `NULL` | [x] |
| E011 | parse family | string ends with a backslash | `NULL` | [x] |
| E012 | parse family | string contains unsupported escape | `NULL` | [x] |
| E013 | parse family | `\u` escape has fewer than four hex digits | `NULL` | [x] |
| E014 | parse family | first UTF-16 code unit is an unpaired low surrogate | `NULL` | [x] |
| E015 | parse family | high surrogate lacks a following `\uXXXX` sequence | `NULL` | [x] |
| E016 | parse family | high surrogate is followed by a non-low-surrogate code unit | `NULL` | [x] |
| E017 | parse family | array nesting depth reaches `CJSON_NESTING_LIMIT` (1000) | `NULL` | [x] |
| E018 | parse family | array input ends immediately after `[`/whitespace | `NULL` | [x] |
| E019 | parse family | an array element is invalid | `NULL` | [x] |
| E020 | parse family | array has a trailing comma | `NULL` | [x] |
| E021 | parse family | array is missing closing `]` | `NULL` | [x] |
| E022 | parse family | object nesting depth reaches `CJSON_NESTING_LIMIT` (1000) | `NULL` | [x] |
| E023 | parse family | object input ends immediately after `{`/whitespace | `NULL` | [x] |
| E024 | parse family | object has a trailing comma / nothing follows comma | `NULL` | [x] |
| E025 | parse family | object key is not a valid JSON string | `NULL` | [x] |
| E026 | parse family | object key is not followed by `:` | `NULL` | [x] |
| E027 | parse family | object value is invalid | `NULL` | [x] |
| E028 | parse family | object is missing closing `}` | `NULL` | [x] |
| E029 | `cJSON_ParseWithLengthOpts` | `require_null_terminated != 0` and parsed value is followed by non-whitespace garbage | `NULL`; parse end at garbage | [x] |
| E030 | parse family | root allocation fails | `NULL` | [x] |
| E031 | parse family | temporary number-string allocation fails | `NULL` | [x] |
| E032 | parse family | decoded string allocation fails | `NULL` | [x] |
| E033 | parse family | array child allocation fails | `NULL` | [x] |
| E034 | parse family | object child allocation fails | `NULL` | [x] |
| E035 | `cJSON_Print` / `cJSON_PrintUnformatted` | `item == NULL` | `NULL` | [x] |
| E036 | print family | item low-byte type is not a supported cJSON type | `NULL` / `0` | [x] |
| E037 | print family | raw item has `valuestring == NULL` | `NULL` / `0` | [x] |
| E038 | print family | initial output allocation fails | `NULL` | [x] |
| E039 | print family | output growth allocation/reallocation fails | `NULL` | [x] |
| E040 | print family | final shrink/copy allocation fails | `NULL` | [x] |
| E041 | `cJSON_PrintBuffered` | `prebuffer < 0` | `NULL` | [x] |
| E042 | `cJSON_PrintBuffered` | initial `prebuffer` allocation returns null | `NULL` | [x] |
| E043 | `cJSON_PrintPreallocated` | `length < 0` | `0` | [x] |
| E044 | `cJSON_PrintPreallocated` | `buffer == NULL` | `0` | [x] |
| E045 | `cJSON_PrintPreallocated` | supplied buffer is too short for output plus terminator | `0` | [x] |
| E046 | `cJSON_SetValuestring` | `object == NULL` | `NULL` | [x] |
| E047 | `cJSON_SetValuestring` | object does not have the string type bit | `NULL` | [x] |
| E048 | `cJSON_SetValuestring` | object has `cJSON_IsReference` | `NULL` | [x] |
| E049 | `cJSON_SetValuestring` | `object->valuestring == NULL` | `NULL` | [x] |
| E050 | `cJSON_SetValuestring` | `valuestring == NULL` | `NULL` | [x] |
| E051 | `cJSON_SetValuestring` | source and existing destination ranges overlap | `NULL` | [x] |
| E052 | `cJSON_SetValuestring` | replacement duplication allocation fails | `NULL`; old value retained | [x] |
| E053 | `cJSON_GetArraySize` | `array == NULL` | `0` | [x] |
| E054 | `cJSON_GetArrayItem` | `array == NULL` | `NULL` | [x] |
| E055 | `cJSON_GetArrayItem` | `index < 0` | `NULL` | [x] |
| E056 | `cJSON_GetArrayItem` | index is at or beyond child count | `NULL` | [x] |
| E057 | object lookup / `cJSON_HasObjectItem` | `object == NULL` | `NULL` / `0` | [x] |
| E058 | object lookup / `cJSON_HasObjectItem` | key pointer is `NULL` | `NULL` / `0` | [x] |
| E059 | object lookup / `cJSON_HasObjectItem` | no matching key exists | `NULL` / `0` | [x] |
| E060 | object lookup | encountered candidate has `string == NULL` | `NULL` | [x] |
| E061 | `cJSON_AddItemToArray` | `array == NULL` | `0` | [x] |
| E062 | `cJSON_AddItemToArray` | `item == NULL` | `0` | [x] |
| E063 | `cJSON_AddItemToArray` | `array == item` | `0` | [x] |
| E064 | `cJSON_AddItemToObject` / `CS` | object, key, or item is null | `0` | [x] |
| E065 | `cJSON_AddItemToObject` / `CS` | `object == item` | `0` | [x] |
| E066 | `cJSON_AddItemToObject` | key duplication allocation fails | `0` | [x] |
| E067 | `cJSON_AddItemReferenceToArray` | `array == NULL` | `0` | [x] |
| E068 | `cJSON_AddItemReferenceToArray` | source item is null or reference allocation fails | `0` | [x] |
| E069 | `cJSON_AddItemReferenceToObject` | object or key is null | `0` | [x] |
| E070 | `cJSON_AddItemReferenceToObject` | source item is null or reference/key allocation fails | `0` | [x] |
| E071 | `cJSON_AddNullToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E072 | `cJSON_AddTrueToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E073 | `cJSON_AddFalseToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E074 | `cJSON_AddBoolToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E075 | `cJSON_AddNumberToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E076 | `cJSON_AddStringToObject` | create/add fails due null object/key/value or allocation failure | `NULL` | [x] |
| E077 | `cJSON_AddRawToObject` | create/add fails due null object/key/value or allocation failure | `NULL` | [x] |
| E078 | `cJSON_AddObjectToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E079 | `cJSON_AddArrayToObject` | create/add fails due null object/key or allocation failure | `NULL` | [x] |
| E080 | `cJSON_DetachItemViaPointer` | parent or item is null | `NULL` | [x] |
| E081 | `cJSON_DetachItemViaPointer` | item is not first child and has `prev == NULL` | `NULL` | [x] |
| E082 | `cJSON_DetachItemFromArray` | `which < 0` | `NULL` | [x] |
| E083 | `cJSON_DetachItemFromArray` | index is out of range / array null | `NULL` | [x] |
| E084 | object detach/delete family | key is null or absent / object null | `NULL` detach or no-op delete | [x] |
| E085 | `cJSON_InsertItemInArray` | `which < 0` | `0` | [x] |
| E086 | `cJSON_InsertItemInArray` | `newitem == NULL` | `0` | [x] |
| E087 | `cJSON_InsertItemInArray` | array is null (including append path) | `0` | [x] |
| E088 | `cJSON_InsertItemInArray` | target non-head child has `prev == NULL` | `0` | [x] |
| E089 | `cJSON_ReplaceItemViaPointer` | parent null, parent child null, item null, or replacement null | `0` | [x] |
| E090 | `cJSON_ReplaceItemInArray` | `which < 0` | `0` | [x] |
| E091 | `cJSON_ReplaceItemInArray` | index out of range / array null | `0` | [x] |
| E092 | object replace family | replacement or key is null | `0` | [x] |
| E093 | object replace family | key duplication allocation fails | `0` | [x] |
| E094 | object replace family | key not found / object null | `0` | [x] |
| E095 | `cJSON_CreateString` | input string is null | `NULL` | [x] |
| E096 | `cJSON_CreateString` | item or string duplication allocation fails | `NULL` | [x] |
| E097 | `cJSON_CreateRaw` | input raw string is null | `NULL` | [x] |
| E098 | `cJSON_CreateRaw` | item or raw duplication allocation fails | `NULL` | [x] |
| E099 | numeric array creators | `count < 0` | `NULL` | [x] |
| E100 | numeric array creators | numbers pointer is null (including count zero) | `NULL` | [x] |
| E101 | numeric array creators | array/item allocation fails | `NULL` | [x] |
| E102 | `cJSON_CreateStringArray` | `count < 0` | `NULL` | [x] |
| E103 | `cJSON_CreateStringArray` | strings pointer is null (including count zero) | `NULL` | [x] |
| E104 | `cJSON_CreateStringArray` | an element pointer is null | `NULL` | [x] |
| E105 | `cJSON_CreateStringArray` | array/item/string allocation fails | `NULL` | [x] |
| E106 | `cJSON_Duplicate` | source item is null | `NULL` | [x] |
| E107 | `cJSON_Duplicate` | item/value/key/child allocation fails | `NULL` | [x] |
| E108 | `cJSON_Duplicate` | recursive depth reaches `CJSON_CIRCULAR_LIMIT` (10000) | `NULL` | [x] |
| E109 | `cJSON_IsInvalid` and all nine other `cJSON_Is*` functions | `item == NULL` | `0` | [x] |
| E110 | `cJSON_Compare` | either item null or low-byte types differ | `0` | [x] |
| E111 | `cJSON_Compare` | low-byte type is not one of the eight supported types | `0` | [x] |
| E112 | `cJSON_Compare` | number values differ beyond epsilon | `0` | [x] |
| E113 | `cJSON_Compare` | string/raw value pointer is null or bytes differ | `0` | [x] |
| E114 | `cJSON_Compare` | corresponding array element differs | `0` | [x] |
| E115 | `cJSON_Compare` | array lengths differ | `0` | [x] |
| E116 | `cJSON_Compare` | object key is missing under selected case mode | `0` | [x] |
| E117 | `cJSON_Compare` | object value differs under selected case mode | `0` | [x] |
| E118 | `cJSON_malloc` | active allocation hook rejects requested size | `NULL` | [x] |
| E119 | `cJSON_Minify` | `json == NULL` | no-op | [x] |

The public API has no enum-typed arguments. Its `cJSON_bool` arguments are
plain `int`; all nonzero values, including values outside `{0,1}`, intentionally
take the true branch and are covered as valid configurations.
