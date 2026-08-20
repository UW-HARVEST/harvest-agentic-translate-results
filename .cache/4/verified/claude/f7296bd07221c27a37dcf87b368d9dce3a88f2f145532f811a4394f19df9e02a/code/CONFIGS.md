# CONFIGS.md — configuration-surface table (Phase A → gate for Phase B)

## 1. Build-time configurations (enumerated first, per the task)

### Cargo features

`translated_rust/Cargo.toml` declares **no `[features]` table** and no
`default` feature, therefore the crate has exactly **one** valid feature
combination:

| # | feature combination | command |
|---|---------------------|---------|
| F0 | *(empty — the only one)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`cargo check --no-default-features` and `cargo build --release
--no-default-features` both succeed with zero warnings-as-errors, and the whole
Phase B + Phase C suite is executed under this combination (which is also the
default one, i.e. `--all-features` and `--no-default-features` resolve to the
same unit).

### CMake options in `c_src/CMakeLists.txt`

| option | default | affects generated code? | handling |
|--------|---------|-------------------------|----------|
| `ENABLE_LOCALES` | **ON** | **yes** — `get_decimal_point()` uses `localeconv()->decimal_point[0]` when defined, hard-codes `'.'` otherwise | The Rust translation implements the `ON` (default) variant, i.e. it calls `localeconv()`. The `.so` under test is built with the default, so both sides read the same locale. Row **C91** drives the whole print/parse surface under `C`, `de_DE.utf8`, `fr_FR.utf8` and `ps_AF.utf8` (a *multi-byte* separator) and requires byte-identical results. |
| `ENABLE_PUBLIC_SYMBOLS` | ON | visibility only (`-fvisibility=hidden` + `CJSON_API_VISIBILITY`) | See `SYMBOLS.md`; the Rust `.so` exports the same set. |
| `ENABLE_HIDDEN_SYMBOLS` | OFF | visibility only | n/a |
| `ENABLE_CUSTOM_COMPILER_FLAGS` | ON | warnings only | n/a |
| `ENABLE_SANITIZERS` / `ENABLE_SAFE_STACK` | OFF | instrumentation only | n/a |
| `BUILD_SHARED_LIBS` / `CJSON_*_SHARED_LIBS` / `ENABLE_CJSON_VERSION_SO` | ON | link form only | shared build used |
| `ENABLE_CJSON_UTILS` | OFF | would add `cJSON_Utils.c` | that file does not exist in `c_src/`, so nothing to translate |
| `ENABLE_CJSON_TEST` | ON | builds `test.c` into `libcJSON_test.so` (exports `driver`) | translated in `src/test_driver.rs`, covered by row C90 |
| `CJSON_NESTING_LIMIT` | 1000 (header) | parse depth limit | rows C55/C56 |
| `CJSON_CIRCULAR_LIMIT` | 10000 (header) | `cJSON_Duplicate` depth limit | ERRORS row 158 |

## 2. Runtime configuration axes the C code actually branches on

* **A1 allocator hooks** (`cJSON_InitHooks`): default (`malloc`/`free`/`realloc`)
  · custom malloc+free (⇒ `global_hooks.reallocate == NULL`, which switches
  `ensure()` and `print()` to the *copy* path instead of the *realloc* path)
  · only `malloc_fn` set · only `free_fn` set · `NULL` (reset).
* **A2 print format** (`format`/`fmt`): 0 (unformatted) · 1 (formatted:
  newlines + `\t` indentation, `, ` in arrays, `:\t` in objects) · non-0/1
  truthy values.
* **A3 print sink**: growing buffer (`cJSON_Print`, `cJSON_PrintUnformatted`)
  · fixed prebuffer that still grows (`cJSON_PrintBuffered`) · caller buffer
  with `noalloc` (`cJSON_PrintPreallocated`).
* **A4 prebuffer size**: 0 · 1 · 2 · small (many `ensure` reallocations) ·
  exact · oversized.
* **A5 parse entry point / options**: `cJSON_Parse` · `cJSON_ParseWithLength`
  · `cJSON_ParseWithOpts` · `cJSON_ParseWithLengthOpts`; `buffer_length` =
  `strlen`+1 · `strlen` (no NUL visible) · shorter (truncating) · longer;
  `require_null_terminated` = 0 · 1 · truthy-other; `return_parse_end` = NULL ·
  non-NULL.
* **A6 case sensitivity**: `cJSON_GetObjectItem` (insensitive) vs
  `…CaseSensitive`; same for `Detach…`, `Delete…`, `ReplaceItemInObject…`;
  `cJSON_Compare(case_sensitive = 0/1)`.
* **A7 key ownership**: `cJSON_AddItemToObject` (strdup'ed key) vs
  `cJSON_AddItemToObjectCS` (`cJSON_StringIsConst`) — changes `cJSON_Delete`,
  `cJSON_Duplicate` and `replace_item_in_object`.
* **A8 references**: `cJSON_CreateStringReference` /
  `cJSON_CreateObjectReference` / `cJSON_CreateArrayReference` /
  `cJSON_AddItemReferenceToArray` / `…ToObject` set `cJSON_IsReference`, which
  changes `cJSON_Delete`, `cJSON_Duplicate` and `cJSON_SetValuestring`.
* **A9 duplicate mode**: `recurse` = 0 · 1.
* **A10 value shape**: the 8 valid `type & 0xFF` variants + invalid ones; number
  magnitude classes; string content classes; container size classes
  (0 / 1 / many / nested / 999-deep / 1000-deep); BOM; whitespace forms;
  duplicate object keys; `NULL` keys/values.

## 3. Configuration rows (cross-product pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** (deterministic
`xorshift64*`, fixed seed `0x2545F4914F6CDD1D`) unless the row is a fixed
boundary, and every row asserts byte-for-byte equality of *all* observable
outputs: return values, printed bytes, `cJSON_GetErrorPtr()` offsets, and the
complete resulting item graph (`type`, `valuestring`, `string`, `valueint`,
`valuedouble` **bit pattern**, and the `child`/`next`/`prev` topology).

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| C1 | `cJSON_Version` | no input | [x] phase_b_core::c1_version |
| C2 | `cJSON_malloc` / `cJSON_free` | default hooks; sizes 0, 1, 8, 4096 | [x] phase_b_core::c2_malloc_free |
| C3 | `cJSON_InitHooks` | `NULL` (reset) → then allocate/print, exercising the `realloc` path | [x] phase_b_hooks::c3_init_hooks_reset |
| C4 | `cJSON_InitHooks` | custom `malloc_fn` + `free_fn` ⇒ `reallocate == NULL`: `print`/`ensure` take the *copy* path; parse + print a large document | [x] phase_b_hooks::c4_init_hooks_custom |
| C5 | `cJSON_InitHooks` | only `malloc_fn` set (`free_fn == NULL`) | [x] phase_b_hooks::c5_c6_init_hooks_partial |
| C6 | `cJSON_InitHooks` | only `free_fn` set (`malloc_fn == NULL`) | [x] phase_b_hooks::c5_c6_init_hooks_partial |
| C7 | `cJSON_CreateNull` / `True` / `False` / `Array` / `Object` | fresh item; check `type`, all fields | [x] phase_b_core::c7_create_scalars |
| C8 | `cJSON_CreateBool` | `boolean` = 0, 1, 2, -1, `INT_MIN`, `INT_MAX`, 256 (out-of-range `cJSON_bool` across FFI) | [x] phase_b_core::c8_create_bool_out_of_range |
| C9 | `cJSON_CreateNumber` | 0, ±1, ±0.5, `INT_MAX`, `INT_MAX±1`, `INT_MIN`, `INT_MIN∓1`, ±`inf`, `NaN`, denormal, 1e300, -0.0, randomized doubles | [x] phase_b_core::c9_create_number |
| C10 | `cJSON_CreateString` | empty, 1 char, ASCII, all 7 one-char escapes, bytes 1..31, byte 127, high bytes (0x80..0xFF), long (>256), randomized | [x] phase_b_core::c10_create_string |
| C11 | `cJSON_CreateStringReference` | literal string; `valuestring` aliases the input, `cJSON_IsReference` set; then `cJSON_Delete` | [x] phase_b_core::c11_create_string_reference |
| C12 | `cJSON_CreateRaw` | `"[1,2]"`, `"garbage"`, empty string | [x] phase_b_core::c12_create_raw |
| C13 | `cJSON_CreateObjectReference` / `cJSON_CreateArrayReference` | child = real array/object; then print + delete (child must survive) | [x] phase_b_core::c13_container_references |
| C14 | `cJSON_CreateIntArray` | `count` = 0, 1, 2, 7, 64; values incl. `INT_MAX`, `INT_MIN`, randomized | [x] phase_b_core::c14_create_int_array |
| C15 | `cJSON_CreateFloatArray` | `count` = 0, 1, 3, 32; values incl. ±inf, NaN, 0.1f, `FLT_MAX`, randomized | [x] phase_b_core::c15_create_float_array |
| C16 | `cJSON_CreateDoubleArray` | `count` = 0, 1, 3, 32; values incl. 1e-300, 1e300, NaN, randomized | [x] phase_b_core::c16_create_double_array |
| C17 | `cJSON_CreateStringArray` | `count` = 0, 1, 7, 32; strings needing escapes; randomized | [x] phase_b_core::c17_create_string_array |
| C18 | `cJSON_AddItemToArray` | append into empty / 1-element / many-element array; check `child->prev` bookkeeping after each append | [x] phase_b_core::c18_add_item_to_array |
| C19 | `cJSON_AddItemToObject` | 1 / many members; keys needing escaping; duplicate keys; long keys | [x] phase_b_core::c19_add_item_to_object |
| C20 | `cJSON_AddItemToObjectCS` | const key ⇒ `cJSON_StringIsConst`; re-add an item that already had a strdup'ed key (frees the old one) | [x] phase_b_core::c20_c21_add_item_to_object_cs |
| C21 | `cJSON_AddItemToObject` on an item that already has a `StringIsConst` key | old key must **not** be freed, new type clears the flag | [x] phase_b_core::c20_c21_add_item_to_object_cs |
| C22 | `cJSON_AddItemReferenceToArray` | reference to a live item, then delete the array (referenced item survives) | [x] phase_b_core::c22_c23_item_references |
| C23 | `cJSON_AddItemReferenceToObject` | ditto with a key | [x] phase_b_core::c22_c23_item_references |
| C24 | `cJSON_AddNullToObject` / `True` / `False` / `Bool(0,1,2)` / `Number` / `String` / `Raw` / `Object` / `Array` | all 9 helpers on a fresh object, then print | [x] phase_b_core::c24_add_helpers |
| C25 | `cJSON_Print` (formatted, growing buffer) | each of the 8 value types standalone | [x] phase_b_print::c25_c26_print_every_type |
| C26 | `cJSON_PrintUnformatted` | each of the 8 value types standalone | [x] phase_b_print::c25_c26_print_every_type |
| C27 | `cJSON_Print` | nested object/array mix, depth 1..8, randomized shapes (exercises `depth` indentation) | [x] phase_b_print::c27_c28_print_random_graphs + phase_b_extra::extra_print_deeply_nested (depth 1..999) |
| C28 | `cJSON_PrintUnformatted` | same shapes as C27 | [x] phase_b_print::c27_c28_print_random_graphs + phase_b_extra::extra_print_deeply_nested (depth 1..999) |
| C29 | `cJSON_Print` | numbers of every magnitude class (drives `%d` vs `%1.15g` vs `%1.17g` and the `sscanf` round-trip check) | [x] phase_b_print::c29_print_number_magnitudes |
| C30 | `cJSON_PrintUnformatted` | strings with every escape class (drives `escape_characters == 0` fast path vs the escaping loop and `u%04x`) | [x] phase_b_print::c30_print_string_escapes |
| C31 | `cJSON_Print` | empty array `[]`, empty object `{}`, array of empties, object with empty-string key | [x] phase_b_print::c31_print_empties |
| C32 | `cJSON_Print` | document large enough to force several `ensure()` reallocations (>256, >512, >100 KiB) | [x] phase_b_print::c32_print_large_documents |
| C33 | `cJSON_PrintBuffered` | `fmt` = 0 and 1 × `prebuffer` = 0, 1, 2, 16, 256, exact, 65536 | [x] phase_b_print::c33_c34_print_buffered |
| C34 | `cJSON_PrintBuffered` | `fmt` = 2 / -1 (truthy non-1 across FFI) | [x] phase_b_print::c33_c34_print_buffered |
| C35 | `cJSON_PrintPreallocated` | `format` = 0/1 × `length` = exact, exact+1, exact+5, generous | [x] phase_b_print::c35_c36_c37_print_preallocated + c35_c36_preallocated_random |
| C36 | `cJSON_PrintPreallocated` | `format` = 0/1 × `length` = 1 byte short, half, 1, 0 (all must fail identically **and** leave identical partial bytes in the buffer) | [x] phase_b_print::c35_c36_c37_print_preallocated + c35_c36_preallocated_random |
| C37 | `cJSON_PrintPreallocated` | `format` = 2 (truthy non-1) | [x] phase_b_print::c35_c36_c37_print_preallocated + c35_c36_preallocated_random |
| C38 | `cJSON_Parse` | the 8 value types as top-level documents | [x] phase_b_parse::c38_c46_parse_documents |
| C39 | `cJSON_Parse` | numbers: `0`, `-0`, `1e5`, `1E+5`, `1e-5`, `0.5`, `-0.5`, `1.7976931348623157e308`, `1e309` (inf), `1e-320`, `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `12345678901234567890`, randomized decimal renderings | [x] phase_b_parse::c38_c46_parse_documents |
| C40 | `cJSON_Parse` | numbers with junk-but-accepted `strtod` prefixes: `0x10`, `01`, `1.2.3`, `1e`, `1e+`, `--1`, `-+1`, `1-2` (each consumed partially — top-level parse then succeeds or fails exactly like C) | [x] phase_b_parse::c38_c46_parse_documents |
| C41 | `cJSON_Parse` | strings: empty, all 7 escapes, `\/`, `A`, `é`, `€`, `😀` (surrogate pair), `\u0000`, raw UTF-8 bytes, 8-bit bytes | [x] phase_b_parse::c38_c46_parse_documents |
| C42 | `cJSON_Parse` | arrays: `[]`, `[1]`, `[1,2,3]`, whitespace variants `[ 1 , 2 ]`, nested `[[[]]]`, mixed types, randomized | [x] phase_b_parse::c38_c46_parse_documents |
| C43 | `cJSON_Parse` | objects: `{}`, one member, many members, duplicate keys, whitespace variants, nested, keys with escapes, randomized | [x] phase_b_parse::c38_c46_parse_documents |
| C44 | `cJSON_Parse` | leading/trailing whitespace, `\t\r\n`, all bytes ≤ 32 as whitespace | [x] phase_b_parse::c38_c46_parse_documents |
| C45 | `cJSON_Parse` | UTF-8 BOM prefix (`EF BB BF`) + value; BOM alone; BOM with < 4 accessible bytes (`skip_utf8_bom` needs `can_access_at_index(buffer,4)`) | [x] phase_b_parse::c38_c46_parse_documents |
| C46 | `cJSON_Parse` | trailing garbage after a complete value (`"1 garbage"`, `"{} x"`) — accepted, `cJSON_Parse` does not require NUL termination | [x] phase_b_parse::c38_c46_parse_documents |
| C47 | `cJSON_ParseWithLength` | `buffer_length` = `strlen+1` (canonical) | [x] phase_b_parse::c47_c48_parse_with_length |
| C48 | `cJSON_ParseWithLength` | `buffer_length` = `strlen` (no visible NUL) — changes `can_read`/`can_access_at_index` decisions for `null`/`true`/`false`/numbers at the very end of the buffer | [x] phase_b_parse::c47_c48_parse_with_length |
| C49 | `cJSON_ParseWithLength` | `buffer_length` shorter than the value (truncation at every offset of a fixed document) | [x] phase_b_parse::c49_parse_with_length_truncated |
| C50 | `cJSON_ParseWithLength` | `buffer_length` longer than the string (reads past the NUL) | [x] phase_b_parse::c50_parse_with_length_longer |
| C51 | `cJSON_ParseWithOpts` | `require_null_terminated` = 0, `return_parse_end` = NULL | [x] phase_b_parse::c51_c54_parse_with_opts |
| C52 | `cJSON_ParseWithOpts` | `require_null_terminated` = 0, `return_parse_end` != NULL — compare the returned **offset** for success and failure inputs | [x] phase_b_parse::c51_c54_parse_with_opts |
| C53 | `cJSON_ParseWithOpts` | `require_null_terminated` = 1 (and 2/-1 truthy) on clean and on trailing-garbage input | [x] phase_b_parse::c51_c54_parse_with_opts |
| C54 | `cJSON_ParseWithLengthOpts` | full cross product: length ∈ {exact, exact+1, short} × `require_null_terminated` ∈ {0,1} × `return_parse_end` ∈ {NULL, ptr} | [x] phase_b_parse::c51_c54_parse_with_opts |
| C55 | `cJSON_Parse` | nesting depth 1, 2, 998, 999 (accepted) for both `[` and `{` | [x] phase_b_parse::c55_c56_nesting_limit |
| C56 | `cJSON_Parse` | nesting depth 1000, 1001, 2000 (rejected at `CJSON_NESTING_LIMIT`) | [x] phase_b_parse::c55_c56_nesting_limit |
| C57 | `cJSON_GetErrorPtr` | after a successful parse (reset) and after each failing parse (offset check) | [x] phase_b_parse::c57_error_pointer |
| C58 | `cJSON_GetArraySize` | array with 0 / 1 / many elements; object; non-container item; `NULL` | [x] phase_b_api::c58_get_array_size |
| C59 | `cJSON_GetArrayItem` | index 0, middle, last, last+1, large, on arrays of size 0/1/many; also on objects | [x] phase_b_api::c59_get_array_item |
| C60 | `cJSON_GetObjectItem` | exact key, differently-cased key, absent key, empty-string key, key on an array (children without `string`) | [x] phase_b_api::c60_c61_c62_object_lookup + phase_b_extra::extra_object_lookup_all_byte_values |
| C61 | `cJSON_GetObjectItemCaseSensitive` | same inputs as C60 | [x] phase_b_api::c60_c61_c62_object_lookup + phase_b_extra::extra_object_lookup_all_byte_values |
| C62 | `cJSON_HasObjectItem` | present / absent / differently-cased | [x] phase_b_api::c60_c61_c62_object_lookup + phase_b_extra::extra_object_lookup_all_byte_values |
| C63 | `cJSON_GetStringValue` | string item, string-reference item, non-string items | [x] phase_b_api::c63_c64_value_accessors |
| C64 | `cJSON_GetNumberValue` | number item (incl. NaN/inf payloads — compared as bit patterns), non-number items | [x] phase_b_api::c63_c64_value_accessors |
| C65 | `cJSON_Is*` (10 fns) | every function × every one of the 8 types × reference/const-key variants × invalid types (`0`, `3`, `0x0F`, `0xFF`, `0x1FF`, `INT_MIN`) | [x] phase_b_api::c65_type_predicates (all 1024 type values x 10 predicates) |
| C66 | `cJSON_SetNumberHelper` | number classes of C9 on a number item; check `valueint`/`valuedouble` and the return value | [x] phase_b_api::c66_set_number_helper |
| C67 | `cJSON_SetValuestring` | new string shorter than / equal to / longer than the old one, on a heap string item; then print | [x] phase_b_api::c67_set_valuestring |
| C68 | `cJSON_DetachItemViaPointer` | detach first / middle / last / only element of an array and of an object; then print both the parent and the detached item | [x] phase_b_api::c68_detach_via_pointer |
| C69 | `cJSON_DetachItemFromArray` | `which` = 0, middle, last, last+1 on sizes 1/2/5 | [x] phase_b_api::c69_c71_detach_delete_from_array |
| C70 | `cJSON_DetachItemFromObject` / `…CaseSensitive` | exact and differently-cased key, first/middle/last member | [x] phase_b_api::c70_c72_detach_delete_from_object |
| C71 | `cJSON_DeleteItemFromArray` | index 0, middle, last, out of range | [x] phase_b_api::c69_c71_detach_delete_from_array |
| C72 | `cJSON_DeleteItemFromObject` / `…CaseSensitive` | present / absent / differently-cased key | [x] phase_b_api::c70_c72_detach_delete_from_object |
| C73 | `cJSON_InsertItemInArray` | `which` = 0 (front), middle, last, `size` (⇒ append), `size+3` on sizes 0/1/2/5 | [x] phase_b_api::c73_insert_item_in_array |
| C74 | `cJSON_ReplaceItemViaPointer` | replace first / middle / last / only element of an array and of an object | [x] phase_b_api::c74_c75_replace |
| C75 | `cJSON_ReplaceItemInArray` | `which` = 0, middle, last, out of range | [x] phase_b_api::c74_c75_replace |
| C76 | `cJSON_ReplaceItemInObject` / `…CaseSensitive` | exact / differently-cased / absent key; replacement with and without an existing (const or heap) key | [x] phase_b_api::c76_replace_in_object |
| C77 | `cJSON_Duplicate` | `recurse` = 0 on every type (children must **not** be copied) | [x] phase_b_api::c77_c78_duplicate |
| C78 | `cJSON_Duplicate` | `recurse` = 1 (and 2/-1) on nested arrays/objects, incl. `StringIsConst` keys and `cJSON_IsReference` items (flag is cleared) | [x] phase_b_api::c77_c78_duplicate |
| C79 | `cJSON_Duplicate_rec` | called directly with `depth` = 0, 1, 9998, 9999, 10000 × `recurse` 0/1 | [x] phase_b_api::c79a_duplicate_rec_depth0 + c79b_duplicate_circular_limit |
| C80 | `cJSON_Compare` | `case_sensitive` = 0 and 1 × equal / unequal pairs of every type | [x] phase_b_api::c80_c84_compare + phase_b_extra::extra_compare_random_numbers + extra_compare_duplicate_keys |
| C81 | `cJSON_Compare` | numbers within/outside the `DBL_EPSILON` relative tolerance (incl. 0 vs 0, -0.0 vs 0.0, NaN vs NaN, inf vs inf) | [x] phase_b_api::c80_c84_compare + phase_b_extra::extra_compare_random_numbers + extra_compare_duplicate_keys |
| C82 | `cJSON_Compare` | nested arrays (same/different length, same/different order) | [x] phase_b_api::c80_c84_compare + phase_b_extra::extra_compare_random_numbers + extra_compare_duplicate_keys |
| C83 | `cJSON_Compare` | objects: same members different order; differently-cased keys with `case_sensitive` 0 vs 1; `a` ⊂ `b`; `b` ⊂ `a` | [x] phase_b_api::c80_c84_compare + phase_b_extra::extra_compare_random_numbers + extra_compare_duplicate_keys |
| C84 | `cJSON_Compare` | identical pointer (`a == b`), and self-comparison of an invalid-typed item | [x] phase_b_api::c80_c84_compare + phase_b_extra::extra_compare_random_numbers + extra_compare_duplicate_keys |
| C85 | `cJSON_Minify` | plain whitespace; `//` comment (with and without terminating newline); `/* */` comment (terminated and unterminated); lone `/`; strings containing `//`, `/*`, spaces, `\"`, `\\`; empty input; randomized JSON documents | [x] phase_b_pipeline::c85_minify |
| C86 | `cJSON_Minify` → `cJSON_Parse` | minify a whitespace/comment-laden document, then parse the result (pipeline) | [x] phase_b_pipeline::c86_minify_then_parse |
| C87 | parse → mutate → print pipeline | parse a document, insert/replace/detach members, print formatted and unformatted, compare bytes and the final graph | [x] phase_b_pipeline::c87_parse_mutate_print_pipeline |
| C88 | print → parse round trip | print a randomly generated graph, re-parse it, compare the two graphs and the second printing | [x] phase_b_pipeline::c88_round_trip |
| C89 | custom hooks + full pipeline | `cJSON_InitHooks(custom)` then C87's pipeline (exercises `reallocate == NULL` inside `ensure` and `print`) | [x] phase_b_hooks::c89_pipeline_with_custom_hooks |
| C90 | `driver` (`test.c` entry point) | the exact argument shapes of the C `main`-style caller: 7 strings, 3×3 int matrix, 4 ids, 2 records — captured **stdout** compared byte-for-byte; plus randomized string/number payloads | [x] phase_b_driver::c90_driver_stdout_differential |
| C91 | every print/parse entry point | `ENABLE_LOCALES` path: `setlocale(LC_ALL, ...)` to `C` · `de_DE.utf8` / `fr_FR.utf8` (single-byte `,` separator) · `ps_AF.utf8` (**multi-byte** U+066B separator, which `get_decimal_point()` truncates to its first byte and therefore mangles the output) | [x] phase_b_locale::c91_comma_decimal_locale |
| C92 | `cJSON_Parse` → print / lookup / duplicate / compare | strings and object keys containing an embedded NUL produced by `\u0000`, so every later `strlen`-based step truncates | [x] phase_b_extra::extra_embedded_nul_strings |
