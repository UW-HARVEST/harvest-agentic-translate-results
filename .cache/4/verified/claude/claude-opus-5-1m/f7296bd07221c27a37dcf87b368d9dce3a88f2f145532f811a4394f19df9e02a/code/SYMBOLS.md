# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

## How this was produced

```sh
# C side (default CMake configuration, ENABLE_LOCALES=ON, ENABLE_PUBLIC_SYMBOLS=ON)
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only libcjson.so.1.7.19   | awk '{print $3}' | sort -u   # 78 symbols
nm -D --defined-only libcJSON_test.so     | awk '{print $3}' | sort -u   #  1 symbol (driver)

# Rust side
cargo build --release
nm -D --defined-only target/release/libcJSON_test.so | awk '{print $3}' | sort -u
```

The C build produces **two** shared objects, and the single Rust `cdylib`
(`libcJSON_test.so`) is the translation of **both** C translation units
(`cJSON.c` + `test.c`), so the reference symbol set is the *union* of the two C
`.so` exports (79 symbols).

## Result

| metric | value |
|--------|-------|
| symbols exported by the C `.so`s (union) | 79 |
| symbols exported by the Rust `.so` | 80 |
| C symbols MISSING from the Rust `.so` | **0** |
| undefined non-libc symbols in the Rust `.so` | **0** (only glibc + libgcc unwinder imports) |
| extra symbols exported by Rust | 1 (`cJSON_Duplicate_rec`, see note) |

Note on `cJSON_Duplicate_rec`: in `cJSON.c` this function is declared with
*external* linkage (`cJSON * cJSON_Duplicate_rec(const cJSON *item, size_t
depth, cJSON_bool recurse);` — it is deliberately not `static`), it is only
absent from `nm -D` because the reference CMake build adds
`-fvisibility=hidden`. The Rust translation implements it with the same
signature and semantics and exports it. This is an *extra* export, never a
missing one, so symbol parity in the required direction (C ⊆ Rust) holds.

`nm -D --undefined-only` on the Rust `.so` lists only libc/`libgcc_s`
imports: `malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memcpy`,
`memmove`, `memset`, `bcmp`, `strlen`, `strcmp`, `strncmp`, `strcpy`, `strtod`,
`tolower`, `snprintf`, `sscanf`, `printf`, `puts`, `exit`, `abort`,
`localeconv`, plus the Rust std runtime's `_Unwind_*` / `pthread_key_*` /
`dl_iterate_phdr` / file-system syscall wrappers. No cJSON symbol is undefined.

## Automated check

`tests/phase_d_symbols.rs` re-derives all of the above at test time
(`phase_d_every_c_symbol_is_exported_by_rust` and
`phase_d_no_unresolved_non_libc_symbols_in_rust`), so the parity claim is
re-verified on every `cargo test` run rather than being a one-off snapshot.

## Symbol table

| symbol | exported by C `.so` | exported by Rust `.so` |
|--------|---------------------|------------------------|
| `cJSON_AddArrayToObject` | yes | yes |
| `cJSON_AddBoolToObject` | yes | yes |
| `cJSON_AddFalseToObject` | yes | yes |
| `cJSON_AddItemReferenceToArray` | yes | yes |
| `cJSON_AddItemReferenceToObject` | yes | yes |
| `cJSON_AddItemToArray` | yes | yes |
| `cJSON_AddItemToObject` | yes | yes |
| `cJSON_AddItemToObjectCS` | yes | yes |
| `cJSON_AddNullToObject` | yes | yes |
| `cJSON_AddNumberToObject` | yes | yes |
| `cJSON_AddObjectToObject` | yes | yes |
| `cJSON_AddRawToObject` | yes | yes |
| `cJSON_AddStringToObject` | yes | yes |
| `cJSON_AddTrueToObject` | yes | yes |
| `cJSON_Compare` | yes | yes |
| `cJSON_CreateArray` | yes | yes |
| `cJSON_CreateArrayReference` | yes | yes |
| `cJSON_CreateBool` | yes | yes |
| `cJSON_CreateDoubleArray` | yes | yes |
| `cJSON_CreateFalse` | yes | yes |
| `cJSON_CreateFloatArray` | yes | yes |
| `cJSON_CreateIntArray` | yes | yes |
| `cJSON_CreateNull` | yes | yes |
| `cJSON_CreateNumber` | yes | yes |
| `cJSON_CreateObject` | yes | yes |
| `cJSON_CreateObjectReference` | yes | yes |
| `cJSON_CreateRaw` | yes | yes |
| `cJSON_CreateString` | yes | yes |
| `cJSON_CreateStringArray` | yes | yes |
| `cJSON_CreateStringReference` | yes | yes |
| `cJSON_CreateTrue` | yes | yes |
| `cJSON_Delete` | yes | yes |
| `cJSON_DeleteItemFromArray` | yes | yes |
| `cJSON_DeleteItemFromObject` | yes | yes |
| `cJSON_DeleteItemFromObjectCaseSensitive` | yes | yes |
| `cJSON_DetachItemFromArray` | yes | yes |
| `cJSON_DetachItemFromObject` | yes | yes |
| `cJSON_DetachItemFromObjectCaseSensitive` | yes | yes |
| `cJSON_DetachItemViaPointer` | yes | yes |
| `cJSON_Duplicate` | yes | yes |
| `cJSON_GetArrayItem` | yes | yes |
| `cJSON_GetArraySize` | yes | yes |
| `cJSON_GetErrorPtr` | yes | yes |
| `cJSON_GetNumberValue` | yes | yes |
| `cJSON_GetObjectItem` | yes | yes |
| `cJSON_GetObjectItemCaseSensitive` | yes | yes |
| `cJSON_GetStringValue` | yes | yes |
| `cJSON_HasObjectItem` | yes | yes |
| `cJSON_InitHooks` | yes | yes |
| `cJSON_InsertItemInArray` | yes | yes |
| `cJSON_IsArray` | yes | yes |
| `cJSON_IsBool` | yes | yes |
| `cJSON_IsFalse` | yes | yes |
| `cJSON_IsInvalid` | yes | yes |
| `cJSON_IsNull` | yes | yes |
| `cJSON_IsNumber` | yes | yes |
| `cJSON_IsObject` | yes | yes |
| `cJSON_IsRaw` | yes | yes |
| `cJSON_IsString` | yes | yes |
| `cJSON_IsTrue` | yes | yes |
| `cJSON_Minify` | yes | yes |
| `cJSON_Parse` | yes | yes |
| `cJSON_ParseWithLength` | yes | yes |
| `cJSON_ParseWithLengthOpts` | yes | yes |
| `cJSON_ParseWithOpts` | yes | yes |
| `cJSON_Print` | yes | yes |
| `cJSON_PrintBuffered` | yes | yes |
| `cJSON_PrintPreallocated` | yes | yes |
| `cJSON_PrintUnformatted` | yes | yes |
| `cJSON_ReplaceItemInArray` | yes | yes |
| `cJSON_ReplaceItemInObject` | yes | yes |
| `cJSON_ReplaceItemInObjectCaseSensitive` | yes | yes |
| `cJSON_ReplaceItemViaPointer` | yes | yes |
| `cJSON_SetNumberHelper` | yes | yes |
| `cJSON_SetValuestring` | yes | yes |
| `cJSON_Version` | yes | yes |
| `cJSON_free` | yes | yes |
| `cJSON_malloc` | yes | yes |
| `driver` | yes | yes |
