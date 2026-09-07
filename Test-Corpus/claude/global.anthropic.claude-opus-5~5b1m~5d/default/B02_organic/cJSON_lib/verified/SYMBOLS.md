# SYMBOLS.md — exported-symbol parity

Generated mechanically:
```
nm -D --defined-only c_src/build/libcjson.so.1.7.19   # C library
nm -D --defined-only c_src/build/libcJSON_test.so     # C driver (test.c)
nm -D --defined-only translation/target/release/libcJSON_test.so  # Rust (both merged)
```

The Rust crate builds a single cdylib `libcJSON_test.so` that contains BOTH the
cJSON library translation (`src/cjson.rs`) and the driver translation (`src/driver.rs`),
so it must export the union of the two C .so symbol sets.

## C `libcjson.so` (78 symbols) + C `libcJSON_test.so` (1 symbol: `driver`)

| # | symbol | in C | in Rust |
|---|--------|------|---------|
| 1 | `cJSON_AddArrayToObject` | libcjson | yes |
| 2 | `cJSON_AddBoolToObject` | libcjson | yes |
| 3 | `cJSON_AddFalseToObject` | libcjson | yes |
| 4 | `cJSON_AddItemReferenceToArray` | libcjson | yes |
| 5 | `cJSON_AddItemReferenceToObject` | libcjson | yes |
| 6 | `cJSON_AddItemToArray` | libcjson | yes |
| 7 | `cJSON_AddItemToObject` | libcjson | yes |
| 8 | `cJSON_AddItemToObjectCS` | libcjson | yes |
| 9 | `cJSON_AddNullToObject` | libcjson | yes |
| 10 | `cJSON_AddNumberToObject` | libcjson | yes |
| 11 | `cJSON_AddObjectToObject` | libcjson | yes |
| 12 | `cJSON_AddRawToObject` | libcjson | yes |
| 13 | `cJSON_AddStringToObject` | libcjson | yes |
| 14 | `cJSON_AddTrueToObject` | libcjson | yes |
| 15 | `cJSON_Compare` | libcjson | yes |
| 16 | `cJSON_CreateArray` | libcjson | yes |
| 17 | `cJSON_CreateArrayReference` | libcjson | yes |
| 18 | `cJSON_CreateBool` | libcjson | yes |
| 19 | `cJSON_CreateDoubleArray` | libcjson | yes |
| 20 | `cJSON_CreateFalse` | libcjson | yes |
| 21 | `cJSON_CreateFloatArray` | libcjson | yes |
| 22 | `cJSON_CreateIntArray` | libcjson | yes |
| 23 | `cJSON_CreateNull` | libcjson | yes |
| 24 | `cJSON_CreateNumber` | libcjson | yes |
| 25 | `cJSON_CreateObject` | libcjson | yes |
| 26 | `cJSON_CreateObjectReference` | libcjson | yes |
| 27 | `cJSON_CreateRaw` | libcjson | yes |
| 28 | `cJSON_CreateString` | libcjson | yes |
| 29 | `cJSON_CreateStringArray` | libcjson | yes |
| 30 | `cJSON_CreateStringReference` | libcjson | yes |
| 31 | `cJSON_CreateTrue` | libcjson | yes |
| 32 | `cJSON_Delete` | libcjson | yes |
| 33 | `cJSON_DeleteItemFromArray` | libcjson | yes |
| 34 | `cJSON_DeleteItemFromObject` | libcjson | yes |
| 35 | `cJSON_DeleteItemFromObjectCaseSensitive` | libcjson | yes |
| 36 | `cJSON_DetachItemFromArray` | libcjson | yes |
| 37 | `cJSON_DetachItemFromObject` | libcjson | yes |
| 38 | `cJSON_DetachItemFromObjectCaseSensitive` | libcjson | yes |
| 39 | `cJSON_DetachItemViaPointer` | libcjson | yes |
| 40 | `cJSON_Duplicate` | libcjson | yes |
| 41 | `cJSON_GetArrayItem` | libcjson | yes |
| 42 | `cJSON_GetArraySize` | libcjson | yes |
| 43 | `cJSON_GetErrorPtr` | libcjson | yes |
| 44 | `cJSON_GetNumberValue` | libcjson | yes |
| 45 | `cJSON_GetObjectItem` | libcjson | yes |
| 46 | `cJSON_GetObjectItemCaseSensitive` | libcjson | yes |
| 47 | `cJSON_GetStringValue` | libcjson | yes |
| 48 | `cJSON_HasObjectItem` | libcjson | yes |
| 49 | `cJSON_InitHooks` | libcjson | yes |
| 50 | `cJSON_InsertItemInArray` | libcjson | yes |
| 51 | `cJSON_IsArray` | libcjson | yes |
| 52 | `cJSON_IsBool` | libcjson | yes |
| 53 | `cJSON_IsFalse` | libcjson | yes |
| 54 | `cJSON_IsInvalid` | libcjson | yes |
| 55 | `cJSON_IsNull` | libcjson | yes |
| 56 | `cJSON_IsNumber` | libcjson | yes |
| 57 | `cJSON_IsObject` | libcjson | yes |
| 58 | `cJSON_IsRaw` | libcjson | yes |
| 59 | `cJSON_IsString` | libcjson | yes |
| 60 | `cJSON_IsTrue` | libcjson | yes |
| 61 | `cJSON_Minify` | libcjson | yes |
| 62 | `cJSON_Parse` | libcjson | yes |
| 63 | `cJSON_ParseWithLength` | libcjson | yes |
| 64 | `cJSON_ParseWithLengthOpts` | libcjson | yes |
| 65 | `cJSON_ParseWithOpts` | libcjson | yes |
| 66 | `cJSON_Print` | libcjson | yes |
| 67 | `cJSON_PrintBuffered` | libcjson | yes |
| 68 | `cJSON_PrintPreallocated` | libcjson | yes |
| 69 | `cJSON_PrintUnformatted` | libcjson | yes |
| 70 | `cJSON_ReplaceItemInArray` | libcjson | yes |
| 71 | `cJSON_ReplaceItemInObject` | libcjson | yes |
| 72 | `cJSON_ReplaceItemInObjectCaseSensitive` | libcjson | yes |
| 73 | `cJSON_ReplaceItemViaPointer` | libcjson | yes |
| 74 | `cJSON_SetNumberHelper` | libcjson | yes |
| 75 | `cJSON_SetValuestring` | libcjson | yes |
| 76 | `cJSON_Version` | libcjson | yes |
| 77 | `cJSON_free` | libcjson | yes |
| 78 | `cJSON_malloc` | libcjson | yes |
| 79 | `driver` | libcJSON_test | yes |

## Extra symbols exported by Rust but not by the C .so

| symbol | note |
|--------|------|
| `cJSON_Duplicate_rec` | non-`static` in `cJSON.c` (declared `cJSON * cJSON_Duplicate_rec(...)`, no `CJSON_PUBLIC`), hidden only because CMake adds `-fvisibility=hidden`. Extra visibility is harmless: no C caller can reference it and the behaviour is identical. |

## Result

**0 symbols missing from the Rust `.so`.** Verified with:
```
comm -23 <(nm -D --defined-only libcjson.so.1.7.19|awk "{print \$3}"|sort) \
        <(nm -D --defined-only libcJSON_test.so|awk "{print \$3}"|sort)   # -> empty
```

No undefined non-libc symbols in the Rust `.so`:
```
_ITM_deregisterTMCloneTable _ITM_registerTMCloneTable _Unwind_Backtrace@GCC_3.3 _Unwind_GetDataRelBase@GCC_3.0 _Unwind_GetIP@GCC_3.0 _Unwind_GetIPInfo@GCC_4.2.0 _Unwind_GetLanguageSpecificData@GCC_3.0 _Unwind_GetRegionStart@GCC_3.0 _Unwind_GetTextRelBase@GCC_3.0 _Unwind_Resume@GCC_3.0 _Unwind_SetGR@GCC_3.0 _Unwind_SetIP@GCC_3.0 __cxa_finalize@GLIBC_2.2.5 __cxa_thread_atexit_impl@GLIBC_2.18 __errno_location@GLIBC_2.2.5 __gmon_start__ __tls_get_addr@GLIBC_2.3 abort@GLIBC_2.2.5 bcmp@GLIBC_2.2.5 calloc@GLIBC_2.2.5 close@GLIBC_2.2.5 dl_iterate_phdr@GLIBC_2.2.5 exit@GLIBC_2.2.5 free@GLIBC_2.2.5 fstat64@GLIBC_2.33 getcwd@GLIBC_2.2.5 getenv@GLIBC_2.2.5 gettid@GLIBC_2.30 localeconv@GLIBC_2.2.5 lseek64@GLIBC_2.2.5 malloc@GLIBC_2.2.5 memcpy@GLIBC_2.14 memmove@GLIBC_2.2.5 memset@GLIBC_2.2.5 mmap64@GLIBC_2.2.5 munmap@GLIBC_2.2.5 open64@GLIBC_2.2.5 posix_memalign@GLIBC_2.2.5 printf@GLIBC_2.2.5 pthread_key_create@GLIBC_2.34 pthread_key_delete@GLIBC_2.34 pthread_setspecific@GLIBC_2.34 puts@GLIBC_2.2.5 read@GLIBC_2.2.5 readlink@GLIBC_2.2.5 realloc@GLIBC_2.2.5 realpath@GLIBC_2.3 sprintf@GLIBC_2.2.5 sscanf@GLIBC_2.2.5 stat64@GLIBC_2.33 statx@GLIBC_2.28 strcmp@GLIBC_2.2.5 strcpy@GLIBC_2.2.5 strlen@GLIBC_2.2.5 strncmp@GLIBC_2.2.5 strtod@GLIBC_2.2.5 syscall@GLIBC_2.2.5 tolower@GLIBC_2.2.5 write@GLIBC_2.2.5 writev@GLIBC_2.2.5 
```

## How this was verified (reproducible)

```bash
# C
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# Rust (the test harness rebuilds this automatically into target/so/)
cd translation && cargo build --offline --lib --target-dir target/so
# diff
comm -23 <(nm -D --defined-only c_src/build/libcjson.so.1.7.19 | awk '{print $3}' | sort) \
         <(nm -D --defined-only translation/target/so/debug/libcJSON_test.so | awk '{print $3}' | sort)
```

Checked for `target/so/debug`, `target/so/release` and `target/release`:
**missing = (empty) in all three**, extra = `cJSON_Duplicate_rec`, `driver`.

Every one of the 79 symbols is additionally *called* through `dlsym` by the test
harness (`tests/common/mod.rs` loads all 78 cJSON entry points by name and
`tests/phase_b_driver.rs` loads `driver`), so a symbol that existed but was wired
to the wrong implementation would fail a differential test rather than pass
silently.  `cJSON_Duplicate_rec` is the single exception: it is hidden in the C
`.so`, so it can only be exercised indirectly via `cJSON_Duplicate`.
