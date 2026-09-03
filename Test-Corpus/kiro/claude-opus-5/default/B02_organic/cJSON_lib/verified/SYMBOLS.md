# SYMBOLS.md — exported-symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from:

```
nm -D --defined-only c_src/build/libcjson.so.1.7.19   # 78 symbols
nm -D --defined-only c_src/build/libcJSON_test.so      #  1 symbol  (driver)
nm -D --defined-only translation/target/release/libcJSON_test.so
```

The C build produces **two** shared objects (`libcjson.so` from `cJSON.c` and
`libcJSON_test.so` from `test.c`). The single Rust cdylib `libcJSON_test.so`
must export the **union** of both symbol sets (`src/lib.rs` = `cJSON.c`,
`src/driver.rs` = `test.c`).

| # | symbol | C .so | Rust .so | status |
|---|--------|-------|----------|--------|
| 1 | `cJSON_AddArrayToObject` | libcjson | yes | OK |
| 2 | `cJSON_AddBoolToObject` | libcjson | yes | OK |
| 3 | `cJSON_AddFalseToObject` | libcjson | yes | OK |
| 4 | `cJSON_AddItemReferenceToArray` | libcjson | yes | OK |
| 5 | `cJSON_AddItemReferenceToObject` | libcjson | yes | OK |
| 6 | `cJSON_AddItemToArray` | libcjson | yes | OK |
| 7 | `cJSON_AddItemToObject` | libcjson | yes | OK |
| 8 | `cJSON_AddItemToObjectCS` | libcjson | yes | OK |
| 9 | `cJSON_AddNullToObject` | libcjson | yes | OK |
| 10 | `cJSON_AddNumberToObject` | libcjson | yes | OK |
| 11 | `cJSON_AddObjectToObject` | libcjson | yes | OK |
| 12 | `cJSON_AddRawToObject` | libcjson | yes | OK |
| 13 | `cJSON_AddStringToObject` | libcjson | yes | OK |
| 14 | `cJSON_AddTrueToObject` | libcjson | yes | OK |
| 15 | `cJSON_Compare` | libcjson | yes | OK |
| 16 | `cJSON_CreateArray` | libcjson | yes | OK |
| 17 | `cJSON_CreateArrayReference` | libcjson | yes | OK |
| 18 | `cJSON_CreateBool` | libcjson | yes | OK |
| 19 | `cJSON_CreateDoubleArray` | libcjson | yes | OK |
| 20 | `cJSON_CreateFalse` | libcjson | yes | OK |
| 21 | `cJSON_CreateFloatArray` | libcjson | yes | OK |
| 22 | `cJSON_CreateIntArray` | libcjson | yes | OK |
| 23 | `cJSON_CreateNull` | libcjson | yes | OK |
| 24 | `cJSON_CreateNumber` | libcjson | yes | OK |
| 25 | `cJSON_CreateObject` | libcjson | yes | OK |
| 26 | `cJSON_CreateObjectReference` | libcjson | yes | OK |
| 27 | `cJSON_CreateRaw` | libcjson | yes | OK |
| 28 | `cJSON_CreateString` | libcjson | yes | OK |
| 29 | `cJSON_CreateStringArray` | libcjson | yes | OK |
| 30 | `cJSON_CreateStringReference` | libcjson | yes | OK |
| 31 | `cJSON_CreateTrue` | libcjson | yes | OK |
| 32 | `cJSON_Delete` | libcjson | yes | OK |
| 33 | `cJSON_DeleteItemFromArray` | libcjson | yes | OK |
| 34 | `cJSON_DeleteItemFromObject` | libcjson | yes | OK |
| 35 | `cJSON_DeleteItemFromObjectCaseSensitive` | libcjson | yes | OK |
| 36 | `cJSON_DetachItemFromArray` | libcjson | yes | OK |
| 37 | `cJSON_DetachItemFromObject` | libcjson | yes | OK |
| 38 | `cJSON_DetachItemFromObjectCaseSensitive` | libcjson | yes | OK |
| 39 | `cJSON_DetachItemViaPointer` | libcjson | yes | OK |
| 40 | `cJSON_Duplicate` | libcjson | yes | OK |
| 41 | `cJSON_GetArrayItem` | libcjson | yes | OK |
| 42 | `cJSON_GetArraySize` | libcjson | yes | OK |
| 43 | `cJSON_GetErrorPtr` | libcjson | yes | OK |
| 44 | `cJSON_GetNumberValue` | libcjson | yes | OK |
| 45 | `cJSON_GetObjectItem` | libcjson | yes | OK |
| 46 | `cJSON_GetObjectItemCaseSensitive` | libcjson | yes | OK |
| 47 | `cJSON_GetStringValue` | libcjson | yes | OK |
| 48 | `cJSON_HasObjectItem` | libcjson | yes | OK |
| 49 | `cJSON_InitHooks` | libcjson | yes | OK |
| 50 | `cJSON_InsertItemInArray` | libcjson | yes | OK |
| 51 | `cJSON_IsArray` | libcjson | yes | OK |
| 52 | `cJSON_IsBool` | libcjson | yes | OK |
| 53 | `cJSON_IsFalse` | libcjson | yes | OK |
| 54 | `cJSON_IsInvalid` | libcjson | yes | OK |
| 55 | `cJSON_IsNull` | libcjson | yes | OK |
| 56 | `cJSON_IsNumber` | libcjson | yes | OK |
| 57 | `cJSON_IsObject` | libcjson | yes | OK |
| 58 | `cJSON_IsRaw` | libcjson | yes | OK |
| 59 | `cJSON_IsString` | libcjson | yes | OK |
| 60 | `cJSON_IsTrue` | libcjson | yes | OK |
| 61 | `cJSON_Minify` | libcjson | yes | OK |
| 62 | `cJSON_Parse` | libcjson | yes | OK |
| 63 | `cJSON_ParseWithLength` | libcjson | yes | OK |
| 64 | `cJSON_ParseWithLengthOpts` | libcjson | yes | OK |
| 65 | `cJSON_ParseWithOpts` | libcjson | yes | OK |
| 66 | `cJSON_Print` | libcjson | yes | OK |
| 67 | `cJSON_PrintBuffered` | libcjson | yes | OK |
| 68 | `cJSON_PrintPreallocated` | libcjson | yes | OK |
| 69 | `cJSON_PrintUnformatted` | libcjson | yes | OK |
| 70 | `cJSON_ReplaceItemInArray` | libcjson | yes | OK |
| 71 | `cJSON_ReplaceItemInObject` | libcjson | yes | OK |
| 72 | `cJSON_ReplaceItemInObjectCaseSensitive` | libcjson | yes | OK |
| 73 | `cJSON_ReplaceItemViaPointer` | libcjson | yes | OK |
| 74 | `cJSON_SetNumberHelper` | libcjson | yes | OK |
| 75 | `cJSON_SetValuestring` | libcjson | yes | OK |
| 76 | `cJSON_Version` | libcjson | yes | OK |
| 77 | `cJSON_free` | libcjson | yes | OK |
| 78 | `cJSON_malloc` | libcjson | yes | OK |
| 79 | `driver` | libcJSON_test | yes | OK |

## Symbols present in Rust but not in C

- `cJSON_Duplicate_rec` — `cJSON_Duplicate_rec` is declared non-`static` in `cJSON.c` but the C
  build uses `-fvisibility=hidden`, so it is not in the C dynamic table.
  An extra export is harmless (no C symbol is shadowed).

## Undefined (imported) symbols in the Rust .so

`nm -D --undefined-only` on the Rust `.so` lists **only** libc / libgcc /
runtime symbols (`malloc`, `free`, `realloc`, `memcpy`, `memmove`, `memset`,
`strlen`, `strcmp`, `strncmp`, `strcpy`, `strtod`, `tolower`, `localeconv`,
`snprintf`, `sscanf`, `printf`, `exit`, `abort`, plus `_Unwind_*`/`__cxa_*`/
`pthread_*`/std-runtime imports). **0 missing non-libc symbols.**

## Result

- C exports: 79 (78 + `driver`)
- Rust exports: 80 (79 + `cJSON_Duplicate_rec`)
- **Symbol diff (C - Rust): EMPTY.**
