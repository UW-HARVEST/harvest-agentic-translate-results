# Dynamic symbol surface

Mechanical source: `nm -D --defined-only ../c_src/build/libcjson.so` and
`nm -D --defined-only ../c_src/build/libcJSON_test.so`.

The C build produces two shared objects. `libcjson.so` defines the 78 public
library functions declared with `CJSON_PUBLIC`; `libcJSON_test.so` defines the
public `driver` wrapper and dynamically imports the subset of cJSON used by
that wrapper. The required surface is their union.

| # | symbol | C shared object | Rust `libcJSON_test.so` |
|---|--------|-----------------|-------------------------|
| 1 | `cJSON_AddArrayToObject` | `libcjson.so` | [x] |
| 2 | `cJSON_AddBoolToObject` | `libcjson.so` | [x] |
| 3 | `cJSON_AddFalseToObject` | `libcjson.so` | [x] |
| 4 | `cJSON_AddItemReferenceToArray` | `libcjson.so` | [x] |
| 5 | `cJSON_AddItemReferenceToObject` | `libcjson.so` | [x] |
| 6 | `cJSON_AddItemToArray` | `libcjson.so` | [x] |
| 7 | `cJSON_AddItemToObject` | `libcjson.so` | [x] |
| 8 | `cJSON_AddItemToObjectCS` | `libcjson.so` | [x] |
| 9 | `cJSON_AddNullToObject` | `libcjson.so` | [x] |
| 10 | `cJSON_AddNumberToObject` | `libcjson.so` | [x] |
| 11 | `cJSON_AddObjectToObject` | `libcjson.so` | [x] |
| 12 | `cJSON_AddRawToObject` | `libcjson.so` | [x] |
| 13 | `cJSON_AddStringToObject` | `libcjson.so` | [x] |
| 14 | `cJSON_AddTrueToObject` | `libcjson.so` | [x] |
| 15 | `cJSON_Compare` | `libcjson.so` | [x] |
| 16 | `cJSON_CreateArray` | `libcjson.so` | [x] |
| 17 | `cJSON_CreateArrayReference` | `libcjson.so` | [x] |
| 18 | `cJSON_CreateBool` | `libcjson.so` | [x] |
| 19 | `cJSON_CreateDoubleArray` | `libcjson.so` | [x] |
| 20 | `cJSON_CreateFalse` | `libcjson.so` | [x] |
| 21 | `cJSON_CreateFloatArray` | `libcjson.so` | [x] |
| 22 | `cJSON_CreateIntArray` | `libcjson.so` | [x] |
| 23 | `cJSON_CreateNull` | `libcjson.so` | [x] |
| 24 | `cJSON_CreateNumber` | `libcjson.so` | [x] |
| 25 | `cJSON_CreateObject` | `libcjson.so` | [x] |
| 26 | `cJSON_CreateObjectReference` | `libcjson.so` | [x] |
| 27 | `cJSON_CreateRaw` | `libcjson.so` | [x] |
| 28 | `cJSON_CreateString` | `libcjson.so` | [x] |
| 29 | `cJSON_CreateStringArray` | `libcjson.so` | [x] |
| 30 | `cJSON_CreateStringReference` | `libcjson.so` | [x] |
| 31 | `cJSON_CreateTrue` | `libcjson.so` | [x] |
| 32 | `cJSON_Delete` | `libcjson.so` | [x] |
| 33 | `cJSON_DeleteItemFromArray` | `libcjson.so` | [x] |
| 34 | `cJSON_DeleteItemFromObject` | `libcjson.so` | [x] |
| 35 | `cJSON_DeleteItemFromObjectCaseSensitive` | `libcjson.so` | [x] |
| 36 | `cJSON_DetachItemFromArray` | `libcjson.so` | [x] |
| 37 | `cJSON_DetachItemFromObject` | `libcjson.so` | [x] |
| 38 | `cJSON_DetachItemFromObjectCaseSensitive` | `libcjson.so` | [x] |
| 39 | `cJSON_DetachItemViaPointer` | `libcjson.so` | [x] |
| 40 | `cJSON_Duplicate` | `libcjson.so` | [x] |
| 41 | `cJSON_GetArrayItem` | `libcjson.so` | [x] |
| 42 | `cJSON_GetArraySize` | `libcjson.so` | [x] |
| 43 | `cJSON_GetErrorPtr` | `libcjson.so` | [x] |
| 44 | `cJSON_GetNumberValue` | `libcjson.so` | [x] |
| 45 | `cJSON_GetObjectItem` | `libcjson.so` | [x] |
| 46 | `cJSON_GetObjectItemCaseSensitive` | `libcjson.so` | [x] |
| 47 | `cJSON_GetStringValue` | `libcjson.so` | [x] |
| 48 | `cJSON_HasObjectItem` | `libcjson.so` | [x] |
| 49 | `cJSON_InitHooks` | `libcjson.so` | [x] |
| 50 | `cJSON_InsertItemInArray` | `libcjson.so` | [x] |
| 51 | `cJSON_IsArray` | `libcjson.so` | [x] |
| 52 | `cJSON_IsBool` | `libcjson.so` | [x] |
| 53 | `cJSON_IsFalse` | `libcjson.so` | [x] |
| 54 | `cJSON_IsInvalid` | `libcjson.so` | [x] |
| 55 | `cJSON_IsNull` | `libcjson.so` | [x] |
| 56 | `cJSON_IsNumber` | `libcjson.so` | [x] |
| 57 | `cJSON_IsObject` | `libcjson.so` | [x] |
| 58 | `cJSON_IsRaw` | `libcjson.so` | [x] |
| 59 | `cJSON_IsString` | `libcjson.so` | [x] |
| 60 | `cJSON_IsTrue` | `libcjson.so` | [x] |
| 61 | `cJSON_Minify` | `libcjson.so` | [x] |
| 62 | `cJSON_Parse` | `libcjson.so` | [x] |
| 63 | `cJSON_ParseWithLength` | `libcjson.so` | [x] |
| 64 | `cJSON_ParseWithLengthOpts` | `libcjson.so` | [x] |
| 65 | `cJSON_ParseWithOpts` | `libcjson.so` | [x] |
| 66 | `cJSON_Print` | `libcjson.so` | [x] |
| 67 | `cJSON_PrintBuffered` | `libcjson.so` | [x] |
| 68 | `cJSON_PrintPreallocated` | `libcjson.so` | [x] |
| 69 | `cJSON_PrintUnformatted` | `libcjson.so` | [x] |
| 70 | `cJSON_ReplaceItemInArray` | `libcjson.so` | [x] |
| 71 | `cJSON_ReplaceItemInObject` | `libcjson.so` | [x] |
| 72 | `cJSON_ReplaceItemInObjectCaseSensitive` | `libcjson.so` | [x] |
| 73 | `cJSON_ReplaceItemViaPointer` | `libcjson.so` | [x] |
| 74 | `cJSON_SetNumberHelper` | `libcjson.so` | [x] |
| 75 | `cJSON_SetValuestring` | `libcjson.so` | [x] |
| 76 | `cJSON_Version` | `libcjson.so` | [x] |
| 77 | `cJSON_free` | `libcjson.so` | [x] |
| 78 | `cJSON_malloc` | `libcjson.so` | [x] |
| 79 | `driver` | `libcJSON_test.so` | [x] |

## Parity gate

- [x] All 79 C-defined public symbols are defined by the Rust shared object.
- [x] Rust has no unintended public exports.
- [x] All symbols have been exercised through `libloading`.
