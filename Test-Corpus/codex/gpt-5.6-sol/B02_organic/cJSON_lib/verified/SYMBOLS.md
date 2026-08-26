# Exported Symbol Surface

Derived with:

```text
nm -D --defined-only c_src/build/libcjson.so
nm -D --defined-only c_src/build/libcJSON_test.so
nm -D --defined-only target/debug/libcJSON_test.so
```

The C surface is the union of the default build's two shared libraries. The
core library contributes 78 `CJSON_PUBLIC` symbols and the test library
contributes `driver`.

| # | C library | symbol | Rust export |
|---|-----------|--------|-------------|
| 1 | libcjson.so | `cJSON_AddArrayToObject` | [x] |
| 2 | libcjson.so | `cJSON_AddBoolToObject` | [x] |
| 3 | libcjson.so | `cJSON_AddFalseToObject` | [x] |
| 4 | libcjson.so | `cJSON_AddItemReferenceToArray` | [x] |
| 5 | libcjson.so | `cJSON_AddItemReferenceToObject` | [x] |
| 6 | libcjson.so | `cJSON_AddItemToArray` | [x] |
| 7 | libcjson.so | `cJSON_AddItemToObject` | [x] |
| 8 | libcjson.so | `cJSON_AddItemToObjectCS` | [x] |
| 9 | libcjson.so | `cJSON_AddNullToObject` | [x] |
| 10 | libcjson.so | `cJSON_AddNumberToObject` | [x] |
| 11 | libcjson.so | `cJSON_AddObjectToObject` | [x] |
| 12 | libcjson.so | `cJSON_AddRawToObject` | [x] |
| 13 | libcjson.so | `cJSON_AddStringToObject` | [x] |
| 14 | libcjson.so | `cJSON_AddTrueToObject` | [x] |
| 15 | libcjson.so | `cJSON_Compare` | [x] |
| 16 | libcjson.so | `cJSON_CreateArray` | [x] |
| 17 | libcjson.so | `cJSON_CreateArrayReference` | [x] |
| 18 | libcjson.so | `cJSON_CreateBool` | [x] |
| 19 | libcjson.so | `cJSON_CreateDoubleArray` | [x] |
| 20 | libcjson.so | `cJSON_CreateFalse` | [x] |
| 21 | libcjson.so | `cJSON_CreateFloatArray` | [x] |
| 22 | libcjson.so | `cJSON_CreateIntArray` | [x] |
| 23 | libcjson.so | `cJSON_CreateNull` | [x] |
| 24 | libcjson.so | `cJSON_CreateNumber` | [x] |
| 25 | libcjson.so | `cJSON_CreateObject` | [x] |
| 26 | libcjson.so | `cJSON_CreateObjectReference` | [x] |
| 27 | libcjson.so | `cJSON_CreateRaw` | [x] |
| 28 | libcjson.so | `cJSON_CreateString` | [x] |
| 29 | libcjson.so | `cJSON_CreateStringArray` | [x] |
| 30 | libcjson.so | `cJSON_CreateStringReference` | [x] |
| 31 | libcjson.so | `cJSON_CreateTrue` | [x] |
| 32 | libcjson.so | `cJSON_Delete` | [x] |
| 33 | libcjson.so | `cJSON_DeleteItemFromArray` | [x] |
| 34 | libcjson.so | `cJSON_DeleteItemFromObject` | [x] |
| 35 | libcjson.so | `cJSON_DeleteItemFromObjectCaseSensitive` | [x] |
| 36 | libcjson.so | `cJSON_DetachItemFromArray` | [x] |
| 37 | libcjson.so | `cJSON_DetachItemFromObject` | [x] |
| 38 | libcjson.so | `cJSON_DetachItemFromObjectCaseSensitive` | [x] |
| 39 | libcjson.so | `cJSON_DetachItemViaPointer` | [x] |
| 40 | libcjson.so | `cJSON_Duplicate` | [x] |
| 41 | libcjson.so | `cJSON_GetArrayItem` | [x] |
| 42 | libcjson.so | `cJSON_GetArraySize` | [x] |
| 43 | libcjson.so | `cJSON_GetErrorPtr` | [x] |
| 44 | libcjson.so | `cJSON_GetNumberValue` | [x] |
| 45 | libcjson.so | `cJSON_GetObjectItem` | [x] |
| 46 | libcjson.so | `cJSON_GetObjectItemCaseSensitive` | [x] |
| 47 | libcjson.so | `cJSON_GetStringValue` | [x] |
| 48 | libcjson.so | `cJSON_HasObjectItem` | [x] |
| 49 | libcjson.so | `cJSON_InitHooks` | [x] |
| 50 | libcjson.so | `cJSON_InsertItemInArray` | [x] |
| 51 | libcjson.so | `cJSON_IsArray` | [x] |
| 52 | libcjson.so | `cJSON_IsBool` | [x] |
| 53 | libcjson.so | `cJSON_IsFalse` | [x] |
| 54 | libcjson.so | `cJSON_IsInvalid` | [x] |
| 55 | libcjson.so | `cJSON_IsNull` | [x] |
| 56 | libcjson.so | `cJSON_IsNumber` | [x] |
| 57 | libcjson.so | `cJSON_IsObject` | [x] |
| 58 | libcjson.so | `cJSON_IsRaw` | [x] |
| 59 | libcjson.so | `cJSON_IsString` | [x] |
| 60 | libcjson.so | `cJSON_IsTrue` | [x] |
| 61 | libcjson.so | `cJSON_Minify` | [x] |
| 62 | libcjson.so | `cJSON_Parse` | [x] |
| 63 | libcjson.so | `cJSON_ParseWithLength` | [x] |
| 64 | libcjson.so | `cJSON_ParseWithLengthOpts` | [x] |
| 65 | libcjson.so | `cJSON_ParseWithOpts` | [x] |
| 66 | libcjson.so | `cJSON_Print` | [x] |
| 67 | libcjson.so | `cJSON_PrintBuffered` | [x] |
| 68 | libcjson.so | `cJSON_PrintPreallocated` | [x] |
| 69 | libcjson.so | `cJSON_PrintUnformatted` | [x] |
| 70 | libcjson.so | `cJSON_ReplaceItemInArray` | [x] |
| 71 | libcjson.so | `cJSON_ReplaceItemInObject` | [x] |
| 72 | libcjson.so | `cJSON_ReplaceItemInObjectCaseSensitive` | [x] |
| 73 | libcjson.so | `cJSON_ReplaceItemViaPointer` | [x] |
| 74 | libcjson.so | `cJSON_SetNumberHelper` | [x] |
| 75 | libcjson.so | `cJSON_SetValuestring` | [x] |
| 76 | libcjson.so | `cJSON_Version` | [x] |
| 77 | libcjson.so | `cJSON_free` | [x] |
| 78 | libcjson.so | `cJSON_malloc` | [x] |
| 79 | libcJSON_test.so | `driver` | [x] |

Missing C symbols in Rust: **0**.

Rust additionally exports `cJSON_Duplicate_rec`; it is a non-public helper in
the C source and does not hide or replace any C export.
