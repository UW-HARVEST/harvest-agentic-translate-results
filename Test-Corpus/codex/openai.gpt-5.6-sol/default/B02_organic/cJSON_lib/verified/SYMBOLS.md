# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libcjson.so
nm -D --defined-only target/release/libcJSON_test.so
```

The C library exports 78 public `cJSON_*` symbols. All 78 are exported by the
Rust shared library with exact names. Rust also exports `cJSON_Duplicate_rec`
and `driver`; neither is a missing C-library symbol.

| # | C symbol | Rust export |
|---:|---|:---:|
| 1 | `cJSON_AddArrayToObject` | yes |
| 2 | `cJSON_AddBoolToObject` | yes |
| 3 | `cJSON_AddFalseToObject` | yes |
| 4 | `cJSON_AddItemReferenceToArray` | yes |
| 5 | `cJSON_AddItemReferenceToObject` | yes |
| 6 | `cJSON_AddItemToArray` | yes |
| 7 | `cJSON_AddItemToObject` | yes |
| 8 | `cJSON_AddItemToObjectCS` | yes |
| 9 | `cJSON_AddNullToObject` | yes |
| 10 | `cJSON_AddNumberToObject` | yes |
| 11 | `cJSON_AddObjectToObject` | yes |
| 12 | `cJSON_AddRawToObject` | yes |
| 13 | `cJSON_AddStringToObject` | yes |
| 14 | `cJSON_AddTrueToObject` | yes |
| 15 | `cJSON_Compare` | yes |
| 16 | `cJSON_CreateArray` | yes |
| 17 | `cJSON_CreateArrayReference` | yes |
| 18 | `cJSON_CreateBool` | yes |
| 19 | `cJSON_CreateDoubleArray` | yes |
| 20 | `cJSON_CreateFalse` | yes |
| 21 | `cJSON_CreateFloatArray` | yes |
| 22 | `cJSON_CreateIntArray` | yes |
| 23 | `cJSON_CreateNull` | yes |
| 24 | `cJSON_CreateNumber` | yes |
| 25 | `cJSON_CreateObject` | yes |
| 26 | `cJSON_CreateObjectReference` | yes |
| 27 | `cJSON_CreateRaw` | yes |
| 28 | `cJSON_CreateString` | yes |
| 29 | `cJSON_CreateStringArray` | yes |
| 30 | `cJSON_CreateStringReference` | yes |
| 31 | `cJSON_CreateTrue` | yes |
| 32 | `cJSON_Delete` | yes |
| 33 | `cJSON_DeleteItemFromArray` | yes |
| 34 | `cJSON_DeleteItemFromObject` | yes |
| 35 | `cJSON_DeleteItemFromObjectCaseSensitive` | yes |
| 36 | `cJSON_DetachItemFromArray` | yes |
| 37 | `cJSON_DetachItemFromObject` | yes |
| 38 | `cJSON_DetachItemFromObjectCaseSensitive` | yes |
| 39 | `cJSON_DetachItemViaPointer` | yes |
| 40 | `cJSON_Duplicate` | yes |
| 41 | `cJSON_GetArrayItem` | yes |
| 42 | `cJSON_GetArraySize` | yes |
| 43 | `cJSON_GetErrorPtr` | yes |
| 44 | `cJSON_GetNumberValue` | yes |
| 45 | `cJSON_GetObjectItem` | yes |
| 46 | `cJSON_GetObjectItemCaseSensitive` | yes |
| 47 | `cJSON_GetStringValue` | yes |
| 48 | `cJSON_HasObjectItem` | yes |
| 49 | `cJSON_InitHooks` | yes |
| 50 | `cJSON_InsertItemInArray` | yes |
| 51 | `cJSON_IsArray` | yes |
| 52 | `cJSON_IsBool` | yes |
| 53 | `cJSON_IsFalse` | yes |
| 54 | `cJSON_IsInvalid` | yes |
| 55 | `cJSON_IsNull` | yes |
| 56 | `cJSON_IsNumber` | yes |
| 57 | `cJSON_IsObject` | yes |
| 58 | `cJSON_IsRaw` | yes |
| 59 | `cJSON_IsString` | yes |
| 60 | `cJSON_IsTrue` | yes |
| 61 | `cJSON_Minify` | yes |
| 62 | `cJSON_Parse` | yes |
| 63 | `cJSON_ParseWithLength` | yes |
| 64 | `cJSON_ParseWithLengthOpts` | yes |
| 65 | `cJSON_ParseWithOpts` | yes |
| 66 | `cJSON_Print` | yes |
| 67 | `cJSON_PrintBuffered` | yes |
| 68 | `cJSON_PrintPreallocated` | yes |
| 69 | `cJSON_PrintUnformatted` | yes |
| 70 | `cJSON_ReplaceItemInArray` | yes |
| 71 | `cJSON_ReplaceItemInObject` | yes |
| 72 | `cJSON_ReplaceItemInObjectCaseSensitive` | yes |
| 73 | `cJSON_ReplaceItemViaPointer` | yes |
| 74 | `cJSON_SetNumberHelper` | yes |
| 75 | `cJSON_SetValuestring` | yes |
| 76 | `cJSON_Version` | yes |
| 77 | `cJSON_free` | yes |
| 78 | `cJSON_malloc` | yes |

Completion check:

- [x] Final release-build symbol diff is empty.
- [x] Rust has no undefined project/non-system symbols.
