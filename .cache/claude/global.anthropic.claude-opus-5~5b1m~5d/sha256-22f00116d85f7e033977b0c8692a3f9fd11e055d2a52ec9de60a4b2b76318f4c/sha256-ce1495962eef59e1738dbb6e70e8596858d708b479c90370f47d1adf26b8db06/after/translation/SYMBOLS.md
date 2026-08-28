# SYMBOLS.md — dynamic-symbol surface (Phase A / Phase D)

Generated mechanically by `nm -D` on the built shared objects; re-checked for
every build profile and feature combination by `verify.sh`.

```
# C side — two objects, mirroring c_src/CMakeLists.txt
nm -D --defined-only c_src/build/libcjson.so.1.7.19   # the cJSON library (cJSON.c)
nm -D --defined-only c_src/build/libcJSON_test.so      # the test driver   (test.c)
# Rust side — one object; cJSON.c and test.c are both translated into it
nm -D --defined-only translation/target/<profile>/libcJSON_test.so
```

* C exports (union of both objects): **79**  (78 from `libcjson.so` + 1 from `libcJSON_test.so`)
* Rust exports (release): **80**;  (debug): **80**
* C symbols MISSING from the Rust `.so`: **0** (release), **0** (debug)
* Extra symbols in the Rust `.so`: **1**
* Undefined non-libc / non-unwind symbols in the Rust `.so`: **0**

## Every C dynamic symbol, and its Rust counterpart

| # | symbol | declared in | C object | in Rust `.so`? |
|---|--------|-------------|----------|----------------|
| 1 | `cJSON_AddArrayToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 2 | `cJSON_AddBoolToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 3 | `cJSON_AddFalseToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 4 | `cJSON_AddItemReferenceToArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 5 | `cJSON_AddItemReferenceToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 6 | `cJSON_AddItemToArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 7 | `cJSON_AddItemToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 8 | `cJSON_AddItemToObjectCS` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 9 | `cJSON_AddNullToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 10 | `cJSON_AddNumberToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 11 | `cJSON_AddObjectToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 12 | `cJSON_AddRawToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 13 | `cJSON_AddStringToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 14 | `cJSON_AddTrueToObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 15 | `cJSON_Compare` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 16 | `cJSON_CreateArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 17 | `cJSON_CreateArrayReference` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 18 | `cJSON_CreateBool` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 19 | `cJSON_CreateDoubleArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 20 | `cJSON_CreateFalse` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 21 | `cJSON_CreateFloatArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 22 | `cJSON_CreateIntArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 23 | `cJSON_CreateNull` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 24 | `cJSON_CreateNumber` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 25 | `cJSON_CreateObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 26 | `cJSON_CreateObjectReference` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 27 | `cJSON_CreateRaw` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 28 | `cJSON_CreateString` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 29 | `cJSON_CreateStringArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 30 | `cJSON_CreateStringReference` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 31 | `cJSON_CreateTrue` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 32 | `cJSON_Delete` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 33 | `cJSON_DeleteItemFromArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 34 | `cJSON_DeleteItemFromObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 35 | `cJSON_DeleteItemFromObjectCaseSensitive` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 36 | `cJSON_DetachItemFromArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 37 | `cJSON_DetachItemFromObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 38 | `cJSON_DetachItemFromObjectCaseSensitive` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 39 | `cJSON_DetachItemViaPointer` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 40 | `cJSON_Duplicate` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 41 | `cJSON_GetArrayItem` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 42 | `cJSON_GetArraySize` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 43 | `cJSON_GetErrorPtr` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 44 | `cJSON_GetNumberValue` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 45 | `cJSON_GetObjectItem` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 46 | `cJSON_GetObjectItemCaseSensitive` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 47 | `cJSON_GetStringValue` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 48 | `cJSON_HasObjectItem` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 49 | `cJSON_InitHooks` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 50 | `cJSON_InsertItemInArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 51 | `cJSON_IsArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 52 | `cJSON_IsBool` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 53 | `cJSON_IsFalse` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 54 | `cJSON_IsInvalid` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 55 | `cJSON_IsNull` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 56 | `cJSON_IsNumber` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 57 | `cJSON_IsObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 58 | `cJSON_IsRaw` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 59 | `cJSON_IsString` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 60 | `cJSON_IsTrue` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 61 | `cJSON_Minify` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 62 | `cJSON_Parse` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 63 | `cJSON_ParseWithLength` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 64 | `cJSON_ParseWithLengthOpts` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 65 | `cJSON_ParseWithOpts` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 66 | `cJSON_Print` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 67 | `cJSON_PrintBuffered` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 68 | `cJSON_PrintPreallocated` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 69 | `cJSON_PrintUnformatted` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 70 | `cJSON_ReplaceItemInArray` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 71 | `cJSON_ReplaceItemInObject` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 72 | `cJSON_ReplaceItemInObjectCaseSensitive` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 73 | `cJSON_ReplaceItemViaPointer` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 74 | `cJSON_SetNumberHelper` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 75 | `cJSON_SetValuestring` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 76 | `cJSON_Version` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 77 | `cJSON_free` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 78 | `cJSON_malloc` | `c_src/cJSON.h` | `libcjson.so` (cJSON.c) | yes |
| 79 | `driver` | `c_src/test.c:41` | `libcJSON_test.so` (test.c) | yes |

Each one is exported from Rust by an `#[unsafe(no_mangle)] pub unsafe extern "C" fn`
wrapper with the identical linker name (`translation/src/cjson.rs` for the 78
library entry points, `translation/src/driver.rs` for `driver`). Nothing is
stubbed: every wrapper contains the translated body of the corresponding C
function.

## Extra symbols exported by the Rust `.so` only

| symbol | why |
|--------|-----|
| `cJSON_Duplicate_rec` | `cJSON.c` declares it with external linkage (`cJSON * cJSON_Duplicate_rec(const cJSON *, size_t, cJSON_bool);` at line 2734, defined at 2741) but *without* the `CJSON_PUBLIC` visibility attribute. Because `ENABLE_PUBLIC_SYMBOLS` is `ON` by default, CMake compiles with `-fvisibility=hidden`, so the C `.so` keeps it out of its dynamic table. Exporting it from Rust is a strict superset — it cannot hide a missing symbol. |

## C symbols missing from the Rust `.so`

**(none — the symbol diff is empty in every profile and feature combination.)**

## Undefined symbols in the Rust `.so`

Every undefined symbol resolves against the platform C library, libgcc's
unwinder or the dynamic loader.  That is the same set `cJSON.c` / `test.c`
import — `malloc`/`free`/`realloc`, `strlen`/`strcmp`/`strncmp`/`strcpy`/`memcpy`,
`strtod`/`sprintf`/`sscanf`/`printf`, `tolower`, `localeconv`, `exit` (see
`translation/src/cshim.rs`) — plus the Rust standard library's own libc usage.
Re-using the platform C library for `strtod`, `sprintf` and `sscanf` is what
makes number formatting and parsing byte-identical.

Non-libc / non-unwind undefined symbols: **0**

<details><summary>full <code>nm -D --undefined-only</code> output (release)</summary>

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
_Unwind_Backtrace@GCC_3.3
_Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0
_Unwind_GetIPInfo@GCC_4.2.0
_Unwind_GetLanguageSpecificData@GCC_3.0
_Unwind_GetRegionStart@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0
_Unwind_Resume@GCC_3.0
_Unwind_SetGR@GCC_3.0
_Unwind_SetIP@GCC_3.0
__cxa_finalize@GLIBC_2.2.5
__cxa_thread_atexit_impl@GLIBC_2.18
__errno_location@GLIBC_2.2.5
__gmon_start__
__tls_get_addr@GLIBC_2.3
abort@GLIBC_2.2.5
bcmp@GLIBC_2.2.5
calloc@GLIBC_2.2.5
close@GLIBC_2.2.5
dl_iterate_phdr@GLIBC_2.2.5
exit@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat64@GLIBC_2.33
getcwd@GLIBC_2.2.5
getenv@GLIBC_2.2.5
gettid@GLIBC_2.30
localeconv@GLIBC_2.2.5
lseek64@GLIBC_2.2.5
malloc@GLIBC_2.2.5
memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5
memset@GLIBC_2.2.5
mmap64@GLIBC_2.2.5
munmap@GLIBC_2.2.5
open64@GLIBC_2.2.5
posix_memalign@GLIBC_2.2.5
printf@GLIBC_2.2.5
pthread_key_create@GLIBC_2.34
pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34
puts@GLIBC_2.2.5
read@GLIBC_2.2.5
readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5
realpath@GLIBC_2.3
sprintf@GLIBC_2.2.5
sscanf@GLIBC_2.2.5
stat64@GLIBC_2.33
statx@GLIBC_2.28
strcmp@GLIBC_2.2.5
strcpy@GLIBC_2.2.5
strlen@GLIBC_2.2.5
strncmp@GLIBC_2.2.5
strtod@GLIBC_2.2.5
syscall@GLIBC_2.2.5
tolower@GLIBC_2.2.5
write@GLIBC_2.2.5
writev@GLIBC_2.2.5
```
</details>

## Verdict

`nm -D` shows **0** C symbols missing from the Rust `.so` and **0** undefined
non-libc symbols, in the `release` and `debug` profiles and under every Cargo
feature combination (`verify.sh` asserts this for each).  Phase A / Phase D
symbol-parity requirement: **MET**.
