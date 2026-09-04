# Configuration Surface

There are no Cargo features in this crate. The rows below enumerate the
runtime flags and input shapes selected by branches in the public C API and
its parse/print pipeline.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `cJSON_Version` | version string and repeated calls | [x] |
| 2 | `cJSON_InitHooks`, `cJSON_malloc`, `cJSON_free` | default hooks (`NULL`) and nonzero allocations | [x] |
| 3 | `cJSON_InitHooks`, allocation-using APIs | custom malloc/free hooks, then restore defaults | [x] |
| 4 | `cJSON_Parse` | scalar literals: null, false, true | [x] |
| 5 | `cJSON_Parse` | numbers: integer, negative zero, fraction, exponent, `INT_MIN/MAX`, subnormal/large | [x] |
| 6 | `cJSON_Parse` | strings: empty/plain and every simple escape | [x] |
| 7 | `cJSON_Parse` | Unicode BMP escapes and valid surrogate pairs | [x] |
| 8 | `cJSON_Parse` | arrays: empty, one, many, heterogeneous, nested | [x] |
| 9 | `cJSON_Parse` | objects: empty, one, many, duplicate/case-varying keys, nested | [x] |
| 10 | `cJSON_Parse` | leading/trailing whitespace and UTF-8 BOM | [x] |
| 11 | `cJSON_ParseWithLength` | exact NUL-inclusive length, shorter bounded slice, embedded NUL | [x] |
| 12 | `cJSON_ParseWithOpts` | `require_null_terminated == 0`, parse prefix and report end | [x] |
| 13 | `cJSON_ParseWithOpts` | `require_null_terminated != 0`, whitespace then NUL | [x] |
| 14 | `cJSON_ParseWithLengthOpts` | both termination modes, explicit lengths, parse-end/error offsets | [x] |
| 15 | `cJSON_GetErrorPtr` | after successful parse and after failures at beginning/middle/end | [x] |
| 16 | `cJSON_Print`, `cJSON_PrintUnformatted` | every scalar low-byte type | [x] |
| 17 | `cJSON_Print`, `cJSON_PrintUnformatted` | empty/one/many arrays and nested arrays | [x] |
| 18 | `cJSON_Print`, `cJSON_PrintUnformatted` | empty/one/many objects and nested objects | [x] |
| 19 | print family | strings requiring no escaping, simple escapes, control-byte `\\u00xx` escapes | [x] |
| 20 | print family | numbers integral/nonintegral, precision fallback, NaN, ±infinity | [x] |
| 21 | `cJSON_PrintBuffered` | `fmt == 0` and nonzero; prebuffer 0, exact-ish, and growth-required | [x] |
| 22 | `cJSON_PrintPreallocated` | `format == 0` and nonzero; exact sufficient and oversized buffers | [x] |
| 23 | create scalar family | null/false/true and bool input zero/nonzero | [x] |
| 24 | `cJSON_CreateNumber`, `cJSON_SetNumberHelper` | below/at/inside/at/above integer saturation bounds plus NaN/infinity | [x] |
| 25 | `cJSON_CreateString`, `cJSON_CreateRaw` | empty and randomized byte strings without interior NUL | [x] |
| 26 | reference constructors | string/object/array references, reference flag and borrowed child/value behavior | [x] |
| 27 | numeric array constructors | int/float/double; count zero, one, many; boundary values | [x] |
| 28 | `cJSON_CreateStringArray` | count zero, one, many; empty and escaped strings | [x] |
| 29 | `cJSON_GetArraySize`, `cJSON_GetArrayItem` | empty/one/many and first/middle/last positions | [x] |
| 30 | object lookup family | case-insensitive and case-sensitive hit/miss; `HasObjectItem` | [x] |
| 31 | all ten type predicates | each valid base type and flag-decorated types (`IsReference`, const key) | [x] |
| 32 | `cJSON_GetStringValue`, `cJSON_GetNumberValue` | matching string/number types and flag-decorated variants | [x] |
| 33 | `cJSON_AddItemToArray` | append into empty, one-item, and many-item arrays | [x] |
| 34 | `cJSON_AddItemToObject`, `cJSON_AddItemToObjectCS` | copied key versus constant key; empty and populated object | [x] |
| 35 | object convenience add family | all nine constructors; bool zero/nonzero; scalar/container values | [x] |
| 36 | reference add family | array/object references to scalar and container items | [x] |
| 37 | `cJSON_DetachItemViaPointer` | detach head, middle, tail, and sole child | [x] |
| 38 | array detach/delete family | first/middle/last indices in one/many arrays | [x] |
| 39 | object detach/delete family | insensitive/sensitive key modes and first/middle/last members | [x] |
| 40 | `cJSON_InsertItemInArray` | insert at head, middle, end, and beyond end (append) | [x] |
| 41 | `cJSON_ReplaceItemViaPointer` | replace head/middle/tail and replacement equal to item | [x] |
| 42 | `cJSON_ReplaceItemInArray` | first/middle/last positions | [x] |
| 43 | object replace family | insensitive/sensitive key modes and case-varying names | [x] |
| 44 | `cJSON_Duplicate` | `recurse == 0` and nonzero for scalar, array, object, reference/const-key items | [x] |
| 45 | `cJSON_Compare` | all scalar types, equal/unequal values, same pointer | [x] |
| 46 | `cJSON_Compare` | arrays equal/element-different/length-different | [x] |
| 47 | `cJSON_Compare` | objects reordered; case-sensitive flag zero/nonzero; key/value differences | [x] |
| 48 | `cJSON_SetValuestring` | shorter/equal in-place replacement and longer allocated replacement | [x] |
| 49 | `cJSON_Minify` | whitespace, line/block comments, slash passthrough, strings and escaped quotes | [x] |
| 50 | `cJSON_Delete` and delete-item APIs | null, scalar, nested owned tree, and reference tree cleanup | [x] |
| 51 | `driver` | full sample pipeline: nested construction, all print modes, numeric arrays, records, and infinity | [x] |
