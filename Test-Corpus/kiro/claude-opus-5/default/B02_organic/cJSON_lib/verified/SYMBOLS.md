# SYMBOLS.md — exported-symbol parity

Source of truth:

```
nm -D --defined-only c_src/build/libcjson.so.1.7.19   # 78 symbols (cJSON.c)
nm -D --defined-only c_src/build/libcJSON_test.so     #  1 symbol  (test.c -> driver)
nm -D --defined-only translation/target/release/libcJSON_test.so  # 80 symbols
```

The Rust crate (`libcJSON_test.so`) is the translation of **both** `cJSON.c`
and `test.c`, so it must export the union of the two C `.so` symbol sets (79).

Symbol diff: **0 missing**. 1 extra Rust symbol (`cJSON_Duplicate_rec`,
a `static` helper in C that Rust exports; extra exports are harmless).

| # | symbol | in C .so | in Rust .so |
|---|--------|----------|-------------|
| 1 | cJSON_AddArrayToObject | yes | yes |
| 2 | cJSON_AddBoolToObject | yes | yes |
| 3 | cJSON_AddFalseToObject | yes | yes |
| 4 | cJSON_AddItemReferenceToArray | yes | yes |
| 5 | cJSON_AddItemReferenceToObject | yes | yes |
| 6 | cJSON_AddItemToArray | yes | yes |
| 7 | cJSON_AddItemToObject | yes | yes |
| 8 | cJSON_AddItemToObjectCS | yes | yes |
| 9 | cJSON_AddNullToObject | yes | yes |
| 10 | cJSON_AddNumberToObject | yes | yes |
| 11 | cJSON_AddObjectToObject | yes | yes |
| 12 | cJSON_AddRawToObject | yes | yes |
| 13 | cJSON_AddStringToObject | yes | yes |
| 14 | cJSON_AddTrueToObject | yes | yes |
| 15 | cJSON_Compare | yes | yes |
| 16 | cJSON_CreateArray | yes | yes |
| 17 | cJSON_CreateArrayReference | yes | yes |
| 18 | cJSON_CreateBool | yes | yes |
| 19 | cJSON_CreateDoubleArray | yes | yes |
| 20 | cJSON_CreateFalse | yes | yes |
| 21 | cJSON_CreateFloatArray | yes | yes |
| 22 | cJSON_CreateIntArray | yes | yes |
| 23 | cJSON_CreateNull | yes | yes |
| 24 | cJSON_CreateNumber | yes | yes |
| 25 | cJSON_CreateObject | yes | yes |
| 26 | cJSON_CreateObjectReference | yes | yes |
| 27 | cJSON_CreateRaw | yes | yes |
| 28 | cJSON_CreateString | yes | yes |
| 29 | cJSON_CreateStringArray | yes | yes |
| 30 | cJSON_CreateStringReference | yes | yes |
| 31 | cJSON_CreateTrue | yes | yes |
| 32 | cJSON_Delete | yes | yes |
| 33 | cJSON_DeleteItemFromArray | yes | yes |
| 34 | cJSON_DeleteItemFromObject | yes | yes |
| 35 | cJSON_DeleteItemFromObjectCaseSensitive | yes | yes |
| 36 | cJSON_DetachItemFromArray | yes | yes |
| 37 | cJSON_DetachItemFromObject | yes | yes |
| 38 | cJSON_DetachItemFromObjectCaseSensitive | yes | yes |
| 39 | cJSON_DetachItemViaPointer | yes | yes |
| 40 | cJSON_Duplicate | yes | yes |
| 41 | cJSON_GetArrayItem | yes | yes |
| 42 | cJSON_GetArraySize | yes | yes |
| 43 | cJSON_GetErrorPtr | yes | yes |
| 44 | cJSON_GetNumberValue | yes | yes |
| 45 | cJSON_GetObjectItem | yes | yes |
| 46 | cJSON_GetObjectItemCaseSensitive | yes | yes |
| 47 | cJSON_GetStringValue | yes | yes |
| 48 | cJSON_HasObjectItem | yes | yes |
| 49 | cJSON_InitHooks | yes | yes |
| 50 | cJSON_InsertItemInArray | yes | yes |
| 51 | cJSON_IsArray | yes | yes |
| 52 | cJSON_IsBool | yes | yes |
| 53 | cJSON_IsFalse | yes | yes |
| 54 | cJSON_IsInvalid | yes | yes |
| 55 | cJSON_IsNull | yes | yes |
| 56 | cJSON_IsNumber | yes | yes |
| 57 | cJSON_IsObject | yes | yes |
| 58 | cJSON_IsRaw | yes | yes |
| 59 | cJSON_IsString | yes | yes |
| 60 | cJSON_IsTrue | yes | yes |
| 61 | cJSON_Minify | yes | yes |
| 62 | cJSON_Parse | yes | yes |
| 63 | cJSON_ParseWithLength | yes | yes |
| 64 | cJSON_ParseWithLengthOpts | yes | yes |
| 65 | cJSON_ParseWithOpts | yes | yes |
| 66 | cJSON_Print | yes | yes |
| 67 | cJSON_PrintBuffered | yes | yes |
| 68 | cJSON_PrintPreallocated | yes | yes |
| 69 | cJSON_PrintUnformatted | yes | yes |
| 70 | cJSON_ReplaceItemInArray | yes | yes |
| 71 | cJSON_ReplaceItemInObject | yes | yes |
| 72 | cJSON_ReplaceItemInObjectCaseSensitive | yes | yes |
| 73 | cJSON_ReplaceItemViaPointer | yes | yes |
| 74 | cJSON_SetNumberHelper | yes | yes |
| 75 | cJSON_SetValuestring | yes | yes |
| 76 | cJSON_Version | yes | yes |
| 77 | cJSON_free | yes | yes |
| 78 | cJSON_malloc | yes | yes |
| 79 | driver | yes (libcJSON_test.so) | yes |

Extra in Rust (not required, not harmful): `cJSON_Duplicate_rec`.

## Verification commands

```
$ nm -D --defined-only c_src/build/libcjson.so.1.7.19 | awk '$2~/[TDB]/{print $3}' | sort  > /tmp/c.txt
$ nm -D --defined-only c_src/build/libcJSON_test.so    | awk '$2~/[TDB]/{print $3}' | sort >> /tmp/c.txt
$ sort -u -o /tmp/c.txt /tmp/c.txt                      # 79 symbols
$ nm -D --defined-only translation/target/release/libcJSON_test.so \
      | awk '$2~/[TDB]/{print $3}' | sort > /tmp/r.txt   # 80 symbols
$ comm -23 /tmp/c.txt /tmp/r.txt                         # (empty) -> 0 missing
```

`run_verification.sh` performs this diff automatically after the test run.

### Undefined symbols in the Rust `.so`

`nm -D -u translation/target/release/libcJSON_test.so` lists only glibc
(`malloc`, `free`, `realloc`, `memcpy`, `memset`, `memmove`, `strlen`, `strcmp`,
`strncmp`, `strcpy`, `strtod`, `sscanf`, `snprintf`, `printf`, `puts`,
`tolower`, `localeconv`, `abort`, `exit`, …), the GCC unwinder (`_Unwind_*`),
and the usual weak runtime hooks (`__gmon_start__`,
`_ITM_(de)registerTMCloneTable`).  **No undefined non-libc symbol** — i.e. no
part of the library is left unimplemented and delegated back to the C.

### Note on the `.so` split

The C build produces two shared objects and the Rust crate produces one:

* `libcjson.so.1.7.19` ← `cJSON.c` (78 exports)
* `libcJSON_test.so` ← `test.c` (1 export: `driver`), links the above
* `translation/target/{debug,release}/libcJSON_test.so` ← `src/lib.rs` +
  `src/driver.rs`, i.e. the translation of **both** C files (80 exports)

The Rust `.so` must therefore satisfy the UNION of the two C symbol sets, which
it does.
