# Configuration-surface table

Mechanically derived from the public header and the branches in `cJSON.c`.
Rows combine public entry points where the C code deliberately shares an
implementation and treats the listed shape/mode identically.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---:|---|---|:---:|
| 1 | `cJSON_Version` | fixed version string and repeated calls | [x] |
| 2 | `cJSON_InitHooks`, `cJSON_malloc`, `cJSON_free` | default hooks reset with NULL | [x] |
| 3 | same | custom malloc + custom free (manual grow path; no realloc) | [x] |
| 4 | `cJSON_CreateNull/True/False/Bool` | false, true, and arbitrary nonzero boolean values | [x] |
| 5 | `cJSON_CreateNumber`, `cJSON_SetNumberHelper`, `cJSON_GetNumberValue` | finite integral values inside `int` range | [x] |
| 6 | same | finite fractional values, `INT_MIN/MAX` boundaries, saturation beyond range, NaN, ±infinity | [x] |
| 7 | `cJSON_CreateString`, `cJSON_GetStringValue`, `cJSON_SetValuestring` | empty/short/escaped/high-byte strings; replacement shorter/equal/longer | [x] |
| 8 | `cJSON_CreateRaw`, `cJSON_CreateStringReference` | owned raw and non-owning string reference | [x] |
| 9 | `cJSON_CreateArray/Object`, `cJSON_CreateArrayReference/ObjectReference` | empty and non-empty owned/reference containers | [x] |
| 10 | all ten `cJSON_Is*` predicates | every valid low-byte type plus ownership flag bits | [x] |
| 11 | `cJSON_CreateIntArray` | count 0, 1, and many; negative/zero/extreme integers | [x] |
| 12 | `cJSON_CreateFloatArray` | count 0, 1, and many; fractions, finite extremes, NaN, ±infinity | [x] |
| 13 | `cJSON_CreateDoubleArray` | count 0, 1, and many; fractions, finite extremes, NaN, ±infinity | [x] |
| 14 | `cJSON_CreateStringArray` | count 0, 1, and many; empty and escaped strings | [x] |
| 15 | `cJSON_AddItemToArray`, `cJSON_GetArraySize`, `cJSON_GetArrayItem` | empty→one→many; first/middle/last positions | [x] |
| 16 | `cJSON_AddItemToObject`, `cJSON_GetObjectItem`, `cJSON_HasObjectItem` | copied key; case-insensitive matching and duplicate-key first match | [x] |
| 17 | `cJSON_AddItemToObjectCS`, `cJSON_GetObjectItemCaseSensitive` | constant key; exact-case hit and case-mismatch miss | [x] |
| 18 | `cJSON_AddItemReferenceToArray/Object` | referenced scalar/container; source remains independently usable | [x] |
| 19 | `cJSON_AddNull/True/False/BoolToObject` | all boolean/null convenience variants | [x] |
| 20 | `cJSON_AddNumber/String/RawToObject` | scalar convenience variants across randomized values/bytes | [x] |
| 21 | `cJSON_AddObject/ArrayToObject` | nested empty and populated containers | [x] |
| 22 | `cJSON_DetachItemViaPointer` | detach first, middle, and last child | [x] |
| 23 | `cJSON_DetachItemFromArray`, `cJSON_DeleteItemFromArray` | first/middle/last index and append-list prev invariant | [x] |
| 24 | `cJSON_DetachItemFromObject`, `cJSON_DeleteItemFromObject` | case-insensitive key | [x] |
| 25 | case-sensitive object detach/delete variants | exact-case key and case-mismatch | [x] |
| 26 | `cJSON_InsertItemInArray` | insert before first/middle; index past end appends | [x] |
| 27 | `cJSON_ReplaceItemViaPointer` | first/middle/last and replacement==item | [x] |
| 28 | `cJSON_ReplaceItemInArray` | first/middle/last index | [x] |
| 29 | `cJSON_ReplaceItemInObject` | case-insensitive replacement and replacement-key rewrite | [x] |
| 30 | `cJSON_ReplaceItemInObjectCaseSensitive` | exact-case replacement and case-mismatch | [x] |
| 31 | `cJSON_Duplicate` | recurse=0 shallow scalar/container copy | [x] |
| 32 | `cJSON_Duplicate` | recurse!=0 nested arrays/objects and ownership flags | [x] |
| 33 | `cJSON_Compare` | every scalar type equal/unequal; pointer identity | [x] |
| 34 | `cJSON_Compare` | arrays empty/one/many, order and length differences | [x] |
| 35 | `cJSON_Compare` | objects with key order changes; case_sensitive=0/1; subset/superset | [x] |
| 36 | `cJSON_Parse` | null/boolean/number/string/array/object roots | [x] |
| 37 | `cJSON_ParseWithOpts` | `require_null_terminated` 0/1; parse-end NULL/non-NULL; trailing whitespace/garbage | [x] |
| 38 | `cJSON_ParseWithLength` | exact length with NUL, exact JSON bytes without NUL, prefix/truncated lengths | [x] |
| 39 | `cJSON_ParseWithLengthOpts` | all option cross-products: parse-end × require-NUL × explicit-length shape | [x] |
| 40 | all parse entry points | leading whitespace and optional UTF-8 BOM at offset zero | [x] |
| 41 | all parse entry points | number grammar branches: sign, integer, fraction, exponent, saturation, terminator | [x] |
| 42 | all parse entry points | strings: no escapes, simple escapes, control escapes, BMP UTF-16, surrogate pairs, embedded high bytes | [x] |
| 43 | all parse entry points | arrays: empty/one/many/nested/mixed values | [x] |
| 44 | all parse entry points | objects: empty/one/many/nested, duplicate and escaped keys | [x] |
| 45 | `cJSON_GetErrorPtr` + parse-end | success resets global error; failures expose identical offset | [x] |
| 46 | `cJSON_Print`, `cJSON_PrintUnformatted` | every value type; formatted/unformatted nested arrays and objects | [x] |
| 47 | same | number branches: integer fast path, 15/17-digit precision, NaN/±infinity→`null` | [x] |
| 48 | same | string escaping: quote, slash/backslash, controls `<32`, high bytes | [x] |
| 49 | `cJSON_PrintBuffered` | fmt=0/1 and prebuffer 0, tiny, exact-ish, oversized | [x] |
| 50 | `cJSON_PrintPreallocated` | format=0/1 and exact-success/extra-space/insufficient buffers | [x] |
| 51 | parse→mutate→print composed pipeline | randomized nested documents through low-level getters/setters/list operations | [x] |
| 52 | `cJSON_Minify` | whitespace, `//`, `/* */`, slash not comment, quoted comments, escaped quotes, unterminated comments/strings | [x] |
| 53 | `cJSON_Delete` | owned tree, reference tree, const-key children, sibling chain | [x] |
| 54 | all allocation-producing APIs | default realloc path and custom-hook manual-copy path | [x] |
| 55 | `cJSON_malloc`, `cJSON_free` | zero, small, and randomized allocation sizes | [x] |

Build configurations found in `Cargo.toml`:

- One configuration: no Cargo features are declared.
- Library target only (`cdylib`); no Rust binary target exists.

Completion check:

- [x] Every row above passes randomized differential coverage where data values apply.
