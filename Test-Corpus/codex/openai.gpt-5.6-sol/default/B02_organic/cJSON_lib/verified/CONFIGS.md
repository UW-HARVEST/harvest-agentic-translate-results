# Configuration surface

Mechanical source: all 79 symbols in `SYMBOLS.md`, the public declarations in
`../c_src/cJSON.h`, and branches on public arguments/data shape in
`../c_src/cJSON.c` and `../c_src/test.c`.

There are no Cargo features declared in `Cargo.toml`; the feature matrix is the
default build and `--no-default-features`, which are equivalent but are both
run at the completion gate. CMake's `ENABLE_LOCALES` is not enabled by the
provided build command and is therefore outside the built C binary's runtime
surface.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C01 | `cJSON_Version` | repeated calls; exact static bytes `1.7.19` | [x] |
| C02 | `cJSON_malloc`, `cJSON_free`, `cJSON_InitHooks` | default allocator; sizes 0, 1, small, and large; reset with null hooks | [x] |
| C03 | `cJSON_InitHooks`, all allocating APIs | custom malloc/free with no realloc, exercising manual print-buffer growth | [x] |
| C04 | `cJSON_GetErrorPtr` | before parse, after successful parse, and after failures at randomized offsets | [x] |
| C05 | `cJSON_CreateNull`, `cJSON_CreateTrue`, `cJSON_CreateFalse`, `cJSON_CreateBool` | null/true/false plus boolean values 0, 1, negative, and greater than 1 | [x] |
| C06 | `cJSON_IsInvalid`, `cJSON_IsFalse`, `cJSON_IsTrue`, `cJSON_IsBool`, `cJSON_IsNull`, `cJSON_IsNumber`, `cJSON_IsString`, `cJSON_IsArray`, `cJSON_IsObject`, `cJSON_IsRaw` | each base type, each type with reference/const flags, invalid type, and cross-type checks | [x] |
| C07 | `cJSON_CreateNumber`, `cJSON_GetNumberValue`, `cJSON_SetNumberHelper` | random finite integral values inside `int` range | [x] |
| C08 | same number entry points | random finite fractional and exponential-scale values | [x] |
| C09 | same number entry points | `INT_MIN/MAX` boundaries, one step outside, huge finite values, ±0, NaN, and ±infinity | [x] |
| C10 | `cJSON_CreateString`, `cJSON_GetStringValue`, `cJSON_SetValuestring` | empty, ASCII, UTF-8, quotes, slashes, controls; replacement shorter/equal/longer | [x] |
| C11 | `cJSON_CreateRaw`, `cJSON_IsRaw` | empty and arbitrary raw JSON fragments | [x] |
| C12 | `cJSON_CreateStringReference`, `cJSON_CreateArrayReference`, `cJSON_CreateObjectReference`, `cJSON_Delete` | null/non-null referenced values and children; deletion preserves referenced storage | [x] |
| C13 | `cJSON_CreateArray`, `cJSON_CreateObject`, `cJSON_Delete` | empty containers and recursively nested owned containers | [x] |
| C14 | `cJSON_CreateIntArray` | count 0, 1, and many randomized `int` values including extrema | [x] |
| C15 | `cJSON_CreateFloatArray` | count 0, 1, and many randomized finite/special `float` values | [x] |
| C16 | `cJSON_CreateDoubleArray` | count 0, 1, and many randomized finite/special `double` values | [x] |
| C17 | `cJSON_CreateStringArray` | count 0, 1, and many strings including empty/escaped/UTF-8 | [x] |
| C18 | `cJSON_AddItemToArray`, `cJSON_GetArraySize`, `cJSON_GetArrayItem` | append to empty/nonempty array; retrieve first/middle/last | [x] |
| C19 | `cJSON_AddItemToObject`, object getters, `cJSON_HasObjectItem` | owned key on empty/nonempty object; exact and differently cased lookup | [x] |
| C20 | `cJSON_AddItemToObjectCS`, object getters | constant key flag and exact/differently cased lookup | [x] |
| C21 | `cJSON_AddItemReferenceToArray` | reference scalar/container into empty/nonempty array; source remains independently usable | [x] |
| C22 | `cJSON_AddItemReferenceToObject` | reference scalar/container with randomized keys; source remains independently usable | [x] |
| C23 | `cJSON_AddNullToObject`, `cJSON_AddTrueToObject`, `cJSON_AddFalseToObject`, `cJSON_AddBoolToObject` | each helper; bool 0 and arbitrary nonzero | [x] |
| C24 | `cJSON_AddNumberToObject` | integral, fractional, saturated, NaN, and infinite numbers | [x] |
| C25 | `cJSON_AddStringToObject`, `cJSON_AddRawToObject` | empty and randomized escaped strings/raw fragments | [x] |
| C26 | `cJSON_AddObjectToObject`, `cJSON_AddArrayToObject` | empty and then populated nested containers | [x] |
| C27 | `cJSON_GetArraySize` | called on array, object, scalar with manually attached child, and empty/null shapes | [x] |
| C28 | `cJSON_GetArrayItem` | first/middle/last and one-past index on arrays and objects | [x] |
| C29 | `cJSON_GetObjectItem`, `cJSON_HasObjectItem` | case-insensitive hit/miss across first/middle/last member | [x] |
| C30 | `cJSON_GetObjectItemCaseSensitive` | case-sensitive hit/miss across first/middle/last member | [x] |
| C31 | `cJSON_DetachItemViaPointer` | detach sole, head, middle, and tail node; inspect detached links and remaining JSON | [x] |
| C32 | `cJSON_DetachItemFromArray`, `cJSON_DeleteItemFromArray` | sole/head/middle/tail indices | [x] |
| C33 | `cJSON_DetachItemFromObject`, `cJSON_DeleteItemFromObject` | differently cased key selects sole/head/middle/tail | [x] |
| C34 | `cJSON_DetachItemFromObjectCaseSensitive`, `cJSON_DeleteItemFromObjectCaseSensitive` | exact-case key selects sole/head/middle/tail | [x] |
| C35 | `cJSON_InsertItemInArray` | insert before head/middle/tail and append at/after count | [x] |
| C36 | `cJSON_ReplaceItemViaPointer` | replace sole/head/middle/tail and replacement is same pointer | [x] |
| C37 | `cJSON_ReplaceItemInArray` | replace sole/head/middle/tail | [x] |
| C38 | `cJSON_ReplaceItemInObject` | case-insensitive replacement of sole/head/middle/tail; replacement key rewritten | [x] |
| C39 | `cJSON_ReplaceItemInObjectCaseSensitive` | exact-case replacement of sole/head/middle/tail; replacement key rewritten | [x] |
| C40 | `cJSON_Duplicate` | recurse 0 for every scalar/container type; no child copied | [x] |
| C41 | `cJSON_Duplicate` | recurse nonzero (1, -1, 2) for empty and deep mixed trees, references, and const keys | [x] |
| C42 | `cJSON_Compare` | identical pointer and separately allocated false/true/null values | [x] |
| C43 | `cJSON_Compare` | finite integral/fractional numbers at equal, epsilon-equal, and unequal values | [x] |
| C44 | `cJSON_Compare` | strings/raw values equal and unequal | [x] |
| C45 | `cJSON_Compare` | arrays empty/one/many, equal and order/length/value differences | [x] |
| C46 | `cJSON_Compare` | objects empty/one/many with reordered keys and subset/value differences | [x] |
| C47 | `cJSON_Compare` | object key case mode 0 versus arbitrary nonzero values | [x] |
| C48 | `cJSON_Minify` | whitespace variants around JSON tokens | [x] |
| C49 | `cJSON_Minify` | one-line and multiline comments, terminated and unterminated | [x] |
| C50 | `cJSON_Minify` | strings containing whitespace, comment markers, escaped quotes, and backslashes | [x] |
| C51 | `cJSON_Parse` | null/false/true literals with leading/trailing whitespace and trailing garbage accepted | [x] |
| C52 | `cJSON_Parse` | random signed integers, fractions, exponent spellings, overflow/underflow, and prefix parsing | [x] |
| C53 | `cJSON_Parse` | empty/ASCII/control-escape strings and all simple escapes | [x] |
| C54 | `cJSON_Parse` | UTF-16 escapes producing 1/2/3-byte UTF-8 and valid surrogate pairs producing 4-byte UTF-8 | [x] |
| C55 | `cJSON_Parse` | arrays empty/one/many and recursively mixed with source whitespace | [x] |
| C56 | `cJSON_Parse` | objects empty/one/many, duplicate keys, escaped keys, and recursively mixed values | [x] |
| C57 | `cJSON_ParseWithOpts` | `return_parse_end` null/non-null; `require_null_terminated` 0 with trailing bytes | [x] |
| C58 | `cJSON_ParseWithOpts` | `require_null_terminated` arbitrary nonzero; exact NUL after optional whitespace | [x] |
| C59 | `cJSON_ParseWithLength` | explicit lengths ending at token, including no NUL; lengths including NUL/trailing bytes | [x] |
| C60 | `cJSON_ParseWithLengthOpts` | cross-product of parse-end null/non-null, require-null 0/nonzero, and explicit short/exact/long lengths | [x] |
| C61 | all parse entry points | UTF-8 BOM at offset zero with enough lookahead; same bytes away from offset zero/not enough lookahead | [x] |
| C62 | `cJSON_Print`, `cJSON_PrintUnformatted` | every scalar type; formatted flag fixed true/false by wrapper | [x] |
| C63 | `cJSON_Print`, `cJSON_PrintUnformatted` | empty/one/many arrays and nested arrays | [x] |
| C64 | `cJSON_Print`, `cJSON_PrintUnformatted` | empty/one/many objects and nested mixed trees; formatting bytes compared exactly | [x] |
| C65 | all print entry points | strings with no escapes, each short escape, controls `< 0x20`, UTF-8, and null value pointer behavior | [x] |
| C66 | all print entry points | numbers integral by `valueint`, 15-digit recoverable, 17-digit fallback, NaN, and ±infinity | [x] |
| C67 | all print entry points | raw values empty and nonempty | [x] |
| C68 | `cJSON_PrintBuffered` | prebuffer 0, 1, exact-ish, 256, and smaller/larger than output; format 0/nonzero | [x] |
| C69 | `cJSON_PrintPreallocated` | exact successful buffer sizes plus five-byte safety margin; format 0/nonzero | [x] |
| C70 | parse then print pipeline | randomized valid JSON through each parser, then every printer, comparing bytes and tree access | [x] |
| C71 | construct then mutate pipeline | randomized low-level construction, add/insert/replace/detach, duplicate, compare, and print | [x] |
| C72 | `cJSON_Delete` | delete null, scalar, sibling chain, owned deep tree, and reference tree | [x] |
| C73 | all constructors/mutators/printers | deterministic fail-after custom allocator at each allocation position, followed by hook reset | [x] |
| C74 | `driver` from `libcJSON_test.so` and Rust `.so` | seven strings, 3x3 ints, four IDs, and two records with randomized consumer-owned values | [x] |
| C75 | every exported symbol | resolve exact symbol name with `libloading` from both C/Rust surfaces | [x] |
