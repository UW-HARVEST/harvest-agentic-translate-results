# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the branches the C code takes. Axes:

* **P** print mode: `cJSON_Print` (format=1) / `cJSON_PrintUnformatted` (format=0) /
  `cJSON_PrintBuffered(prebuffer, fmt)` / `cJSON_PrintPreallocated(buf, len, fmt)` (noalloc=true)
* **R** parse entry: `cJSON_Parse` / `cJSON_ParseWithLength(len)` /
  `cJSON_ParseWithOpts(&end, req_nul)` / `cJSON_ParseWithLengthOpts(len, &end, req_nul)`
* **CS** case sensitivity: `cJSON_GetObjectItem` (insensitive) vs `…CaseSensitive`
* **REC** `cJSON_Duplicate(item, recurse)` 0/1
* **CK** constant key: `cJSON_AddItemToObject` (strdup) vs `cJSON_AddItemToObjectCS` (const)
* **H** allocator hooks: default vs `cJSON_InitHooks(custom)` — note this also switches
  `reallocate` to NULL (only set when `allocate==malloc && deallocate==free`), which
  changes the `ensure()` and `print()` growth paths.
* **SHAPE** input shape (numbers, strings/escapes, containers, depth, flags)

`[x]` = differential test passes across randomized inputs (fixed seed).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | cJSON_Version | no args; returns "1.7.19" | [x] |
| 2 | cJSON_malloc / cJSON_free | default hooks; sizes 1,8,64,4096 round-trip | [x] |
| 3 | cJSON_InitHooks(NULL) | resets to malloc/free/realloc; then full parse+print pipeline still works | [x] |
| 4 | cJSON_InitHooks(hooks with both malloc_fn+free_fn) | `reallocate == NULL` path: `ensure()` uses allocate+memcpy, `print()` uses allocate+memcpy instead of realloc; run print of large doc through it | [x] |
| 5 | cJSON_InitHooks(hooks, only malloc_fn set) | deallocate stays `free`; `reallocate` NULL (allocate != malloc) | [x] |
| 6 | cJSON_InitHooks(hooks, only free_fn set) | allocate stays `malloc`; `reallocate` NULL (deallocate != free) | [x] |
| 7 | cJSON_CreateNull / True / False | scalar creators; type byte + print in both formats | [x] |
| 8 | cJSON_CreateBool(b) | b = 0, 1, and non-0/1 ints (2, -1, 0x100) | [x] |
| 9 | cJSON_CreateNumber(d) | d integer-representable → `%d` print path; randomized ints incl. 0, ±1, INT_MAX, INT_MIN | [x] |
| 10 | cJSON_CreateNumber(d) | d needing `%1.15g`; randomized doubles in ±1e±[0..300] | [x] |
| 11 | cJSON_CreateNumber(d) | d needing `%1.17g` (15g round-trip fails); randomized full-precision doubles | [x] |
| 12 | cJSON_CreateNumber(d) | d ≥ INT_MAX / ≤ INT_MIN → `valueint` saturation | [x] |
| 13 | cJSON_CreateNumber(d) | d = NaN, +inf, -inf → prints `null` | [x] |
| 14 | cJSON_CreateNumber(d) | d = -0.0, DBL_MIN, DBL_MAX, DBL_EPSILON subnormals | [x] |
| 15 | cJSON_CreateString(s) | ASCII, empty string, 1-char, long (>4096) | [x] |
| 16 | cJSON_CreateString(s) | s containing every escape-triggering char: `"` `\` `\b` `\f` `\n` `\r` `\t` | [x] |
| 17 | cJSON_CreateString(s) | s containing control chars 0x01..0x1F → `\uXXXX` print path | [x] |
| 18 | cJSON_CreateString(s) | s containing UTF-8 multibyte (2/3/4-byte) — copied through unescaped | [x] |
| 19 | cJSON_CreateString(s) | randomized bytes 0x01..0xFF, mixed escape/no-escape (escape_characters==0 fast path vs slow) | [x] |
| 20 | cJSON_CreateStringReference(s) | sets `cJSON_IsReference`; print, Delete (must not free), Duplicate strips flag | [x] |
| 21 | cJSON_CreateRaw(s) | raw JSON emitted verbatim; both print formats; nested in array/object | [x] |
| 22 | cJSON_CreateArray / CreateObject | empty container; print format=0 `[]`/`{}` vs format=1 | [x] |
| 23 | cJSON_CreateObjectReference(child) / CreateArrayReference(child) | `cJSON_IsReference` set; print, Delete, Duplicate | [x] |
| 24 | cJSON_CreateIntArray | count = 0, 1, 2, and randomized 3..64 with randomized int values incl. INT_MIN/INT_MAX | [x] |
| 25 | cJSON_CreateFloatArray | count = 0, 1, randomized 2..64; randomized f32 values, incl. values whose f64 promotion needs 17g | [x] |
| 26 | cJSON_CreateDoubleArray | count = 0, 1, randomized 2..64; randomized f64 values | [x] |
| 27 | cJSON_CreateStringArray | count = 0, 1, randomized 2..64; randomized strings with escapes | [x] |
| 28 | cJSON_AddItemToArray | build array element-by-element: 0→1 (empty-list branch), 1→2, n→n+1 (append-via-prev) | [x] |
| 29 | cJSON_AddItemToObject (CK=strdup) | non-constant key: `cJSON_StringIsConst` cleared, key strdup'd; then Delete frees it | [x] |
| 30 | cJSON_AddItemToObjectCS (CK=const) | constant key: `cJSON_StringIsConst` set; Delete must not free key | [x] |
| 31 | cJSON_AddItemReferenceToArray | wraps item in reference; original still owned by caller | [x] |
| 32 | cJSON_AddItemReferenceToObject | reference + non-constant key | [x] |
| 33 | cJSON_AddNullToObject / AddTrueToObject / AddFalseToObject | convenience wrappers, returned item pointer + resulting print | [x] |
| 34 | cJSON_AddBoolToObject | boolean = 0, 1, and non-0/1 | [x] |
| 35 | cJSON_AddNumberToObject | randomized numbers, all three print paths | [x] |
| 36 | cJSON_AddStringToObject | randomized strings with escapes | [x] |
| 37 | cJSON_AddRawToObject | raw payloads | [x] |
| 38 | cJSON_AddObjectToObject / AddArrayToObject | nested container creation, then populate the returned child | [x] |
| 39 | cJSON_GetArraySize | size 0, 1, and randomized 2..64 | [x] |
| 40 | cJSON_GetArrayItem | index 0, size-1, mid; randomized index over randomized array | [x] |
| 41 | cJSON_GetObjectItem (CS=0) | key match differing only in case; duplicate keys (first match wins); `tolower` on non-ASCII bytes | [x] |
| 42 | cJSON_GetObjectItemCaseSensitive (CS=1) | exact key match only | [x] |
| 43 | cJSON_HasObjectItem | present / absent / case-differing (uses insensitive lookup) | [x] |
| 44 | cJSON_GetStringValue | String item, and String-reference item | [x] |
| 45 | cJSON_GetNumberValue | Number item; randomized values incl. NaN/inf | [x] |
| 46 | cJSON_Is* (all 10) | called on one item of EVERY type incl. references and const-string flags — full 10×N matrix | [x] |
| 47 | cJSON_SetNumberHelper | randomized doubles on a Number item: saturation of `valueint`, return value | [x] |
| 48 | cJSON_SetValuestring | new string SHORTER or EQUAL length → in-place `strcpy` branch | [x] |
| 49 | cJSON_SetValuestring | new string LONGER → strdup + free branch; randomized lengths | [x] |
| 50 | cJSON_Print (P=format 1) | scalar, empty array, empty object, flat array, flat object, nested mix; randomized trees | [x] |
| 51 | cJSON_PrintUnformatted (P=format 0) | same shapes as row 50 | [x] |
| 52 | cJSON_PrintBuffered(prebuffer, fmt=1) | prebuffer = 0, 1, 2, 16, exact-size, huge (forces `ensure()` doubling loop repeatedly) | [x] |
| 53 | cJSON_PrintBuffered(prebuffer, fmt=0) | same prebuffer sweep, unformatted | [x] |
| 54 | cJSON_PrintPreallocated(buf, len, fmt=1) | len exactly sufficient (success), len = needed+5 (success) | [x] |
| 55 | cJSON_PrintPreallocated(buf, len, fmt=0) | len exactly sufficient / generous, unformatted | [x] |
| 56 | cJSON_PrintPreallocated | len = needed-1 (noalloc reject) — partial buffer contents compared too | [x] |
| 57 | cJSON_Parse | randomized valid JSON documents (round-trip via generator) | [x] |
| 58 | cJSON_Parse | numbers: `0`, `-0`, `1`, `-1`, `1.5`, `1e5`, `1E+5`, `1e-5`, `1e400` (inf), `-1e400`, `1e-400` (0), 20-digit ints | [x] |
| 59 | cJSON_Parse | strings: all escapes `\" \\ \/ \b \f \n \r \t`, `\u0041`, `\u00e9`, `\u20ac`, valid surrogate pair `\ud83d\ude00` | [x] |
| 60 | cJSON_Parse | leading UTF-8 BOM `\xEF\xBB\xBF` (only skipped at offset 0) | [x] |
| 61 | cJSON_Parse | leading/interior/trailing whitespace incl. all bytes ≤ 0x20 | [x] |
| 62 | cJSON_Parse | `[]`, `{}`, single-element `[1]` / `{"a":1}`, many-element (64) | [x] |
| 63 | cJSON_Parse | nesting depth 1, 2, 100, 999 (just under CJSON_NESTING_LIMIT) for arrays and objects | [x] |
| 64 | cJSON_ParseWithLength(len = strlen+1) | equivalent to Parse | [x] |
| 65 | cJSON_ParseWithLength(len < strlen+1) | truncates the document mid-token — buffer bound enforced, no NUL needed | [x] |
| 66 | cJSON_ParseWithLength(len > payload, no NUL) | non-NUL-terminated buffer parsed by length only | [x] |
| 67 | cJSON_ParseWithOpts(&end, req_nul=0) | trailing garbage allowed; `*return_parse_end` offset checked on success | [x] |
| 68 | cJSON_ParseWithOpts(&end, req_nul=1) | trailing whitespace allowed, trailing garbage rejected; `*end` on failure | [x] |
| 69 | cJSON_ParseWithOpts(NULL, req_nul=0/1) | `return_parse_end == NULL` branch not taken | [x] |
| 70 | cJSON_ParseWithLengthOpts(len, &end, req_nul=0) | full low-level entry, all 4 flag/len combos × randomized docs | [x] |
| 71 | cJSON_ParseWithLengthOpts(len, &end, req_nul=1) | ditto with null-termination required | [x] |
| 72 | cJSON_GetErrorPtr | after a successful parse and after each failing parse — global offset pointer | [x] |
| 73 | parse → print round trip (P=1) | randomized docs: `cJSON_Parse` then `cJSON_Print`, byte-compare | [x] |
| 74 | parse → print round trip (P=0) | randomized docs: `cJSON_Parse` then `cJSON_PrintUnformatted` | [x] |
| 75 | parse → print round trip (PrintBuffered, both fmt) | randomized docs × prebuffer sweep | [x] |
| 76 | parse → print round trip (PrintPreallocated, both fmt) | randomized docs, exact and short buffers | [x] |
| 77 | cJSON_Duplicate(REC=1) | deep copy of randomized nested trees; then Compare and Print both copies | [x] |
| 78 | cJSON_Duplicate(REC=0) | shallow copy — children NOT copied; print of shallow copy differs from original | [x] |
| 79 | cJSON_Duplicate | items with `cJSON_IsReference` (flag stripped) and `cJSON_StringIsConst` (key not strdup'd) | [x] |
| 80 | cJSON_Duplicate | depth just under CJSON_CIRCULAR_LIMIT vs at/over it | [x] |
| 81 | cJSON_Compare(case_sensitive=1) | equal / unequal randomized trees; objects with same keys different order | [x] |
| 82 | cJSON_Compare(case_sensitive=0) | object keys differing only in case → equal | [x] |
| 83 | cJSON_Compare | `a == b` identical pointer shortcut; every type pair (10×10 matrix) | [x] |
| 84 | cJSON_Compare | Number values near `compare_double` epsilon threshold (equal-within-epsilon) | [x] |
| 85 | cJSON_DetachItemViaPointer | detach first child / middle child / last child / only child of array and object | [x] |
| 86 | cJSON_DetachItemFromArray | which = 0, mid, size-1 over randomized arrays | [x] |
| 87 | cJSON_DetachItemFromObject (CS=0) / …CaseSensitive (CS=1) | key match with/without case difference | [x] |
| 88 | cJSON_DeleteItemFromArray | which = 0, mid, size-1; resulting print compared | [x] |
| 89 | cJSON_DeleteItemFromObject / …CaseSensitive | present key, both case modes | [x] |
| 90 | cJSON_InsertItemInArray | which = 0 (head insert), mid, size-1, size (append fallback) | [x] |
| 91 | cJSON_ReplaceItemViaPointer | replace head child, middle, last, only child (child->prev == child branch) in array and object | [x] |
| 92 | cJSON_ReplaceItemInArray | which = 0, mid, size-1 | [x] |
| 93 | cJSON_ReplaceItemInObject (CS=0) / …CaseSensitive (CS=1) | present key, both case modes; replacement gets the key strdup'd | [x] |
| 94 | cJSON_Minify | whitespace-only, `//` line comment, `/* */` block comment, string containing `//` and `\"`, unterminated comment, randomized formatted JSON | [x] |
| 95 | cJSON_Delete | every type, references, const keys, deep trees (no leak/double-free observable via subsequent ops) | [x] |
| 96 | cJSON_SetIntValue / SetNumberValue / SetBoolValue (header macros) | replicated call sequences on Number/Bool items; observable via GetNumberValue + Print | [x] |
| 97 | driver (test.c translation) | C `libcJSON_test.so` vs Rust `.so`: same `strings[7]`, `numbers[3][3]`, `ids[4]`, `record[2]` inputs; stdout compared byte-for-byte, incl. `1.0/0.0` → `null` case and the `PrintPreallocated`-too-small path | [x] |
| 98 | driver | randomized `strings`/`numbers`/`ids`/`record` payloads (incl. escapes and extreme doubles), stdout byte-compared | [x] |
| 99 | full pipeline composition | Parse → mutate (Add/Detach/Replace/Insert) → Duplicate → Compare → Print, randomized op sequences with a fixed seed | [x] |
| 100 | full pipeline under custom hooks | row 99 repeated after `cJSON_InitHooks(custom)` so the no-`reallocate` growth path is exercised end to end | [x] |
| 101 | all number entry points, `LC_NUMERIC` = `de_DE.utf8` | `ENABLE_LOCALES` is ON in the C build, so `get_decimal_point()` returns `,`: `parse_number` rewrites `.`→`,` before `strtod` and `print_number` rewrites `,`→`.` on output. Exercised over the full number corpus + randomized documents (test asserts `localeconv()->decimal_point == ','`, so the branch is provably taken) | [x] |
| 102 | all number entry points, `LC_ALL=C` | baseline decimal point `.` | [x] |
| 103 | Print / PrintUnformatted / PrintBuffered / PrintPreallocated at nesting depth 1…999 | `print_array`/`print_object` emit `depth` indentation tabs per level when `format=1`; deep trees drive many more `ensure()` growth rounds, and `PrintPreallocated` is checked at exactly-sufficient and one-byte-short lengths at every depth | [x] |
| 104 | mixed-ownership container + `cJSON_Delete` | one container holding an owned child, a `cJSON_IsReference` child and a `cJSON_StringIsConst` key; after `Delete` the referenced tree must still be intact and printable | [x] |
| 105 | every operation under a hook allocator | C and Rust must perform the **same number** of allocations for 17 representative operations (Parse, Print, PrintUnformatted, PrintBuffered, Duplicate, all constructors, Add*, Replace*, SetValuestring) — an allocation-count difference would mean the translation restructured the algorithm | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`** table, so the only cargo
feature configuration is the default (empty) one. Verified mechanically:

```
$ grep -c '^\[features\]' translation/Cargo.toml   # 0
```

Therefore "every feature combination" = the single default configuration. The
compile-time axes that DO exist live in the C build (`ENABLE_LOCALES`,
`CJSON_NESTING_LIMIT`, `CJSON_CIRCULAR_LIMIT`, visibility macros); the C library
under test is built with the CMake defaults (`ENABLE_LOCALES=ON`,
`ENABLE_PUBLIC_SYMBOLS=ON`) and the Rust translation is compared against exactly
that build. The locale axis is exercised at runtime in rows 9–14 and 58 (the
`get_decimal_point()` path) under the process's active `LC_NUMERIC`.

## Where each row is verified

| test file | rows |
|-----------|------|
| `tests/a_symbols.rs` | symbol resolution smoke test |
| `tests/b_configs_core.rs` | 1-49, 96 |
| `tests/b_configs_parse_print.rs` | 50-96, 99-100, 103-104 |
| `tests/b_driver.rs` | 97-98 |
| `tests/b_locale.rs` | 101-102 |
| `tests/c_errors.rs` | every `ERRORS.md` row, G1-G7, row 105 |

Every test loads BOTH shared objects through `libloading` and calls only
exported symbols; no Rust function is called directly, so the `#[no_mangle]`
wrappers are themselves under test.  `run_verification.sh` rebuilds the C
libraries and runs the whole suite against BOTH the debug and the release
cdylib, then diffs `nm -D`.

The harness refuses to run against a stale `.so`: `cargo test` does not rebuild
a `cdylib` that no Rust target links against, so `tests/harness/mod.rs` compares
the `.so` mtime against `src/**/*.rs` and fails loudly instead of silently
verifying an old binary.
