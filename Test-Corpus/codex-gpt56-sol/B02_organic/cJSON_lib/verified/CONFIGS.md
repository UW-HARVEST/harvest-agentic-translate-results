# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table and no default features. There is exactly
one valid Rust feature combination:

| # | `cargo` feature combination | [ ] |
|---|-----------------------------|-----|
| F001 | `--no-default-features` (empty feature set) | [x] |

The C default used as ground truth is the option state produced by the checked
in `CMakeLists.txt`: custom flags on, sanitizers/safe-stack off, public symbols
on, hidden symbols off, shared libraries on, target export on, shared+static
off, shared override off, override value on, SO versioning on, utils off, and
the test library on.

Mechanically, the CMake file defines ten independent boolean options plus the
three valid sanitizer/safe-stack states (`off/off`, `on/off`, `off/on`).
`ENABLE_CJSON_UTILS=ON` is not a valid build in this snapshot because
`cJSON_Utils.c` and `cJSON_Utils.h` are absent. Thus the source snapshot has
`3 * 2^10 = 3072` syntactically valid CMake option combinations, but these are
C build-instrumentation/product-layout combinations, not Rust features or
different cJSON API semantics. The mandated differential ground truth is the
default state above.

## Runtime And Input Matrix

Rows come from branches in `cJSON.h` and `cJSON.c`. Each row is exercised with
multiple deterministic randomized values where the shape admits variation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C001 | `cJSON_Version` | no arguments; stable `1.7.19` byte string | [x] |
| C002 | `cJSON_InitHooks`, `cJSON_malloc`, `cJSON_free` | null hooks reset; allocate/free sizes 0, 1, small, large | [x] |
| C003 | `cJSON_InitHooks`, allocation-using APIs | custom malloc+free, custom malloc only, custom free only | [x] |
| C004 | `cJSON_Delete` | null, singleton, sibling chain, recursive owned children, reference children, constant keys | [x] |
| C005 | `cJSON_Parse` | scalar literals: null, false, true | [x] |
| C006 | `cJSON_Parse` | numbers: zero, signed integers, fractions, exponent forms, near `INT_MIN/MAX`, overflow, underflow | [x] |
| C007 | `cJSON_Parse` | strings: empty, ASCII, all simple escapes, control escapes, BMP Unicode, surrogate pairs | [x] |
| C008 | `cJSON_Parse` | arrays: empty, one, many, mixed, nested below depth limit | [x] |
| C009 | `cJSON_Parse` | objects: empty, one, many, duplicate keys, nested below depth limit | [x] |
| C010 | `cJSON_ParseWithOpts` | `return_parse_end` null/non-null; `require_null_terminated` 0 | [x] |
| C011 | `cJSON_ParseWithOpts` | `require_null_terminated` 1 and noncanonical nonzero; leading/trailing whitespace | [x] |
| C012 | `cJSON_ParseWithLength` | exact NUL-inclusive, exact non-NUL, truncated, and oversized supplied lengths | [x] |
| C013 | `cJSON_ParseWithLengthOpts` | embedded NUL; `require_null_terminated` 0/nonzero; parse-end returned | [x] |
| C014 | parse family, `cJSON_GetErrorPtr` | UTF-8 BOM at offset zero versus BOM bytes elsewhere | [x] |
| C015 | `cJSON_Print`, `cJSON_PrintUnformatted` | each of null/bool/number/string/raw/array/object | [x] |
| C016 | print family | finite integer, finite noninteger at 15/17-digit paths, NaN, +Inf, -Inf | [x] |
| C017 | print family | strings with quotes, backslashes, named controls, other bytes below 0x20, and UTF-8 | [x] |
| C018 | `cJSON_Print` | formatted empty/nonempty/nested arrays and objects | [x] |
| C019 | `cJSON_PrintUnformatted` | compact empty/nonempty/nested arrays and objects | [x] |
| C020 | `cJSON_PrintBuffered` | `fmt` 0, 1, noncanonical nonzero; prebuffer 0, 1, exact, undersized, oversized | [x] |
| C021 | `cJSON_PrintPreallocated` | `format` 0/1/noncanonical; exact+5 and oversized writable buffers | [x] |
| C022 | `cJSON_PrintPreallocated` | one byte below required capacity returns false | [x] |
| C023 | `cJSON_GetStringValue`, `cJSON_GetNumberValue` | matching string/number types with modifier bits set | [x] |
| C024 | all ten `cJSON_Is*` | every low-byte type 0, 1, 2, 4, 8, 16, 32, 64, 128 plus modifier bits | [x] |
| C025 | `cJSON_CreateNull`, `CreateTrue`, `CreateFalse` | inspect ABI fields and print each result | [x] |
| C026 | `cJSON_CreateBool` | boolean 0, 1, negative, and other nonzero values | [x] |
| C027 | `cJSON_CreateNumber`, `cJSON_SetNumberHelper` | finite random values, integral boundaries, NaN, infinities; integer saturation | [x] |
| C028 | `cJSON_CreateString`, `cJSON_CreateRaw` | empty and randomized escaped/plain byte strings | [x] |
| C029 | `cJSON_CreateStringReference` | null and non-null borrowed value; reference modifier and printing | [x] |
| C030 | `cJSON_CreateArray`, `cJSON_CreateObject` | empty containers | [x] |
| C031 | `cJSON_CreateArrayReference`, `cJSON_CreateObjectReference` | null and non-null borrowed child chains | [x] |
| C032 | `cJSON_CreateIntArray` | count 0, 1, many; zero, negative, and boundary integers | [x] |
| C033 | `cJSON_CreateFloatArray` | count 0, 1, many; finite, NaN, infinities | [x] |
| C034 | `cJSON_CreateDoubleArray` | count 0, 1, many; finite, NaN, infinities | [x] |
| C035 | `cJSON_CreateStringArray` | count 0, 1, many; empty and escaped strings | [x] |
| C036 | `cJSON_AddItemToArray` | append into empty, one-item, and many-item arrays | [x] |
| C037 | `cJSON_AddItemToObject` | copied key into empty and populated objects; replace prior owned key on item | [x] |
| C038 | `cJSON_AddItemToObjectCS` | constant key into empty/populated objects; modifier bit set | [x] |
| C039 | `cJSON_AddItemReferenceToArray` | reference scalar/container into empty/populated arrays | [x] |
| C040 | `cJSON_AddItemReferenceToObject` | reference scalar/container under randomized copied key | [x] |
| C041 | `cJSON_AddNullToObject` | empty/populated object and randomized key | [x] |
| C042 | `cJSON_AddTrueToObject` | empty/populated object and randomized key | [x] |
| C043 | `cJSON_AddFalseToObject` | empty/populated object and randomized key | [x] |
| C044 | `cJSON_AddBoolToObject` | boolean 0 and arbitrary nonzero; randomized key | [x] |
| C045 | `cJSON_AddNumberToObject` | finite, boundary, NaN, and infinite number; randomized key | [x] |
| C046 | `cJSON_AddStringToObject` | empty/escaped/randomized value and key | [x] |
| C047 | `cJSON_AddRawToObject` | scalar/container raw bytes and randomized key | [x] |
| C048 | `cJSON_AddObjectToObject` | add empty child then populate it | [x] |
| C049 | `cJSON_AddArrayToObject` | add empty child then populate it | [x] |
| C050 | `cJSON_GetArraySize`, `cJSON_GetArrayItem` | empty, one, many; first/middle/last indexes | [x] |
| C051 | `cJSON_GetObjectItem` | first/middle/last key; exact and differently-cased query | [x] |
| C052 | `cJSON_GetObjectItemCaseSensitive` | exact key hit and differently-cased miss | [x] |
| C053 | `cJSON_HasObjectItem` | present exact/different case and absent keys | [x] |
| C054 | `cJSON_DetachItemViaPointer` | detach only, first, middle, and last child | [x] |
| C055 | `cJSON_DetachItemFromArray`, `cJSON_DeleteItemFromArray` | remove first/middle/last by index | [x] |
| C056 | `cJSON_DetachItemFromObject`, `cJSON_DeleteItemFromObject` | remove exact/different-case key | [x] |
| C057 | `cJSON_DetachItemFromObjectCaseSensitive`, `cJSON_DeleteItemFromObjectCaseSensitive` | exact-case removal and different-case no-op | [x] |
| C058 | `cJSON_InsertItemInArray` | empty array and indexes 0/past-end append | [x] |
| C059 | `cJSON_InsertItemInArray` | populated array at first/middle/last positions | [x] |
| C060 | `cJSON_ReplaceItemViaPointer` | replacement equals item; replace only/first/middle/last | [x] |
| C061 | `cJSON_ReplaceItemInArray` | replace first/middle/last by index | [x] |
| C062 | `cJSON_ReplaceItemInObject` | exact/different-case query, preserving requested replacement key spelling | [x] |
| C063 | `cJSON_ReplaceItemInObjectCaseSensitive` | exact-case hit and different-case miss | [x] |
| C064 | `cJSON_SetValuestring` | shorter/equal nonoverlapping value reuses allocation | [x] |
| C065 | `cJSON_SetValuestring` | longer value allocates replacement | [x] |
| C066 | `cJSON_Duplicate` | recurse 0/1/noncanonical nonzero for scalar, array, object | [x] |
| C067 | `cJSON_Duplicate` | source has reference and constant-key modifier bits | [x] |
| C068 | `cJSON_Compare` | same pointer and distinct equal values for all eight supported types | [x] |
| C069 | `cJSON_Compare` | arrays empty/one/many/nested, equal and unequal | [x] |
| C070 | `cJSON_Compare` | objects reordered; key case mode 0/1/noncanonical nonzero | [x] |
| C071 | `cJSON_Minify` | whitespace outside strings; whitespace inside strings | [x] |
| C072 | `cJSON_Minify` | line comments, block comments, slash not starting a comment | [x] |
| C073 | `cJSON_Minify` | escaped quote/backslash inside strings | [x] |
| C074 | `cJSON_GetErrorPtr` | successful parse clears error; failures at start/middle/end | [x] |
| C075 | `driver` | canonical string/number/id/record arrays through exported test entry point | [x] |

