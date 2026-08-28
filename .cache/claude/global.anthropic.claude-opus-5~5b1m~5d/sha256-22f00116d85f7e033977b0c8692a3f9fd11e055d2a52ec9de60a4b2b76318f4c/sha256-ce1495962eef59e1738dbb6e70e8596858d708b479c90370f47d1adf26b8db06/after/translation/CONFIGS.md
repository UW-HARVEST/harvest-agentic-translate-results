# CONFIGS.md — configuration-surface table (Phase A/B)

Derived mechanically from the branches `c_src/cJSON.c` actually takes on
*valid* input. Axes were enumerated from the public header
(`c_src/cJSON.h`) plus every `if` / `switch` / `#ifdef` the C code evaluates on a
runtime option or on the shape of the data.

## Axes the C code branches on

### A. Allocator configuration (`cJSON_InitHooks`, lines 176–205)
Selects `global_hooks.{allocate,deallocate,reallocate}` and therefore which of
two *completely different* buffer-growth paths `ensure` (502–530) and `print`
(1227–1248) take:

| id | `cJSON_InitHooks` argument | resulting `reallocate` | growth path |
|----|-----------------------------|------------------------|-------------|
| A0 | never called / `NULL` (reset) | `realloc` | `ensure`: 505 `reallocate`; `print`: 1229 `reallocate` |
| A1 | `{malloc_fn = libc malloc, free_fn = libc free}` | `realloc` (both equal the libc originals, 201) | same as A0 |
| A2 | `{malloc_fn = custom, free_fn = custom}` | `NULL` | `ensure`: 518 `allocate` + `memcpy` + `deallocate`; `print`: 1237 `allocate` + `memcpy` (`cjson_min` clamp) |
| A3 | `{malloc_fn = custom, free_fn = NULL}` | `NULL` (`deallocate` = libc `free`, `allocate` ≠ `malloc`) | manual path |
| A4 | `{malloc_fn = NULL, free_fn = custom}` | `NULL` | manual path |
| A5 | `{malloc_fn = NULL, free_fn = NULL}` | `realloc` | same as A0 |

### B. Print entry point × `format` flag
`cJSON_Print` (format=1) · `cJSON_PrintUnformatted` (format=0) ·
`cJSON_PrintBuffered(prebuffer, fmt)` (`noalloc=0`, caller-chosen initial size) ·
`cJSON_PrintPreallocated(buf, length, format)` (`noalloc=1`, **no** growth at all).
`format` gates 9 distinct branches: `print_array` 1589/1596, `print_object`
1749/1758/1766/1795/1809/1820/1830/1835.

### C. Parse entry point × options
`cJSON_Parse` · `cJSON_ParseWithLength(len)` · `cJSON_ParseWithOpts(end, rnt)` ·
`cJSON_ParseWithLengthOpts(len, end, rnt)`.
Options: `return_parse_end` NULL/non-NULL (1149), `require_null_terminated`
0/non-0 (1141), and `buffer_length` exact / longer / shorter than `strlen+1`
(`can_read`/`can_access_at_index`, 266–268).

### D. Value type (`print_value` switch on `type & 0xFF`, 1394; `parse_value`, 1339–1379)
`cJSON_NULL` · `cJSON_False` · `cJSON_True` · `cJSON_Number` · `cJSON_Raw` ·
`cJSON_String` · `cJSON_Array` · `cJSON_Object`, each optionally OR-ed with
`cJSON_IsReference` (256) and/or `cJSON_StringIsConst` (512) — both of which are
masked out of the switch but change `cJSON_Delete` (226/230/235),
`cJSON_Duplicate` (2760/2773), `add_item_to_object` (2060), `replace_item_in_object`
(2387) and `cJSON_SetValuestring` (403).

### E. Number shape (`print_number`, 558–625; `parse_number`, 274–375)
`isnan`/`isinf` → `"null"` · `d == (double)valueint` → `"%d"` ·
`"%1.15g"` round-trips (`sscanf` + `compare_double`) · otherwise `"%1.17g"`.
Saturation boundaries `>= INT_MAX`, `<= (double)INT_MIN`, else `(int)d`.
Locale decimal point substitution (`get_decimal_point`, `ENABLE_LOCALES`).

### F. String shape (`print_string_ptr` 916–1035; `parse_string` 786–913)
`NULL` → `""` · no escapes needed (`memcpy` fast path, 976) · one-char escapes
`" \ \b \f \n \r \t` · bytes `< 32` → `\u00xx` (5 extra chars) · bytes `> 127`
copied verbatim · parse-side escapes `\b \f \n \r \t \" \\ \/` and `\uXXXX`
(1/2/3/4-byte UTF-8, surrogate pairs).

### G. Container shape
0 / 1 / many children; nesting depth 1 … 999 (limit 1000); arrays vs objects vs
mixed; `child->prev` circular back-pointer maintenance
(`add_item_to_array` 1992–2007, `parse_array` 1536, `cJSON_Create*Array` 2606).

### H. Lookup / mutation index & key
`case_sensitive` 0/1 (`get_object_item`, 1913); index 0 / middle / last;
key present / duplicate keys / differing case.

### I. `cJSON_Duplicate` `recurse` 0/1 (2780) and `cJSON_Compare` `case_sensitive` 0/1.

### J. `cJSON_Minify` token classes (2887–2917)
whitespace ` \t\r\n` · `//` line comment · `/* */` block comment · lone `/` ·
string literal with escaped quotes (`minify_string`, 2854–2874) · other bytes.

---

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** from a fixed-seed PRNG
(`harness::Rng`) and asserted byte-for-byte / field-for-field between the C and
the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | covering test | [x] |
|---|----------------|--------------------------------------------|---------------|-----|
| 1 | `cJSON_Version` | no input | `create.rs::cfg01_version` | [x] |
| 2 | `cJSON_malloc` / `cJSON_free` | sizes 0, 1, 8, 4096, 1 MiB (A0) | `create.rs::cfg02_malloc_free` | [x] |
| 3 | `cJSON_CreateNull/True/False` | — (check `type`, all fields zero) | `create.rs::cfg03_04_09_trivial_constructors` | [x] |
| 4 | `cJSON_CreateBool` | `boolean` = 0, 1, and out-of-range ints 2/-1/`INT_MIN` | `create.rs::cfg03_04_09_trivial_constructors` | [x] |
| 5 | `cJSON_CreateNumber` | random `json_f64()`: 0, -0, ±small ints, `INT_MAX`/`INT_MIN` ± ε, huge, denormal, ±inf, NaN → check `type`/`valueint`/`valuedouble` bits | `create.rs::cfg05_create_number` | [x] |
| 6 | `cJSON_CreateString` | empty / ASCII / all one-char-escape bytes / bytes 1..31 / high bytes 128..255 / long | `create.rs::cfg06_07_08_string_constructors` | [x] |
| 7 | `cJSON_CreateRaw` | same shapes as row 6 | `create.rs::cfg06_07_08_string_constructors` | [x] |
| 8 | `cJSON_CreateStringReference` | `type == String\|IsReference`, `valuestring` aliases the caller's buffer | `create.rs::cfg06_07_08_string_constructors` | [x] |
| 9 | `cJSON_CreateArray` / `cJSON_CreateObject` | empty containers | `create.rs::cfg03_04_09_trivial_constructors` | [x] |
| 10 | `cJSON_CreateObjectReference` / `cJSON_CreateArrayReference` | non-NULL child; `type` has `IsReference`; `cJSON_Delete` must not free the child | `create.rs::cfg10_container_references` | [x] |
| 11 | `cJSON_CreateIntArray` | `count` = 0, 1, 2, 3, 17, 256 × random `int` values incl. `INT_MIN`/`INT_MAX` | `create.rs::cfg11_14_typed_arrays` | [x] |
| 12 | `cJSON_CreateFloatArray` | `count` = 0, 1, many × random `f32` incl. ±inf/NaN/denormal (note the `float→double` widening) | `create.rs::cfg11_14_typed_arrays` | [x] |
| 13 | `cJSON_CreateDoubleArray` | `count` = 0, 1, many × random `f64` from `json_f64()` | `create.rs::cfg11_14_typed_arrays` | [x] |
| 14 | `cJSON_CreateStringArray` | `count` = 0, 1, many × strings from row 6's pool | `create.rs::cfg11_14_typed_arrays` | [x] |
| 15 | `cJSON_GetArraySize` / `cJSON_GetArrayItem` | arrays of size 0/1/5/64, index 0 / middle / last / `size` / `size+1` | `create.rs::cfg15_get_array_size_and_item` | [x] |
| 16 | `cJSON_GetObjectItem` (case-insensitive) | key exact / different case / mixed case / duplicate keys / absent | `create.rs::cfg16_18_object_lookup` | [x] |
| 17 | `cJSON_GetObjectItemCaseSensitive` | same keys as row 16 | `create.rs::cfg16_18_object_lookup` | [x] |
| 18 | `cJSON_HasObjectItem` | same keys as row 16 | `create.rs::cfg16_18_object_lookup` | [x] |
| 19 | `cJSON_GetStringValue` / `cJSON_GetNumberValue` | one item of each of the 9 types | `create.rs::cfg19_20_accessors_and_predicates` | [x] |
| 20 | all 10 `cJSON_Is*` predicates | one item of each of the 9 types, ± `IsReference`, ± `StringIsConst`, plus raw `type` values 0/3/0x0A/0xFF/256/512 | `create.rs::cfg19_20_accessors_and_predicates` | [x] |
| 21 | `cJSON_AddItemToArray` | append into empty / 1-element / n-element array; append the same item twice | `create.rs::cfg21_23_add_item` | [x] |
| 22 | `cJSON_AddItemToObject` | keys: empty, ASCII, needing escapes, duplicate; item already having a `string` (freed) | `create.rs::cfg21_23_add_item` | [x] |
| 23 | `cJSON_AddItemToObjectCS` | constant key ⇒ `type \|= StringIsConst`; then `cJSON_Delete` must not free the key | `create.rs::cfg21_23_add_item` | [x] |
| 24 | `cJSON_AddItemReferenceToArray` / `…ToObject` | referencing a live subtree; printing and deleting the parent | `create.rs::cfg24_item_references` | [x] |
| 25 | `cJSON_AddNull/True/False/Bool/Number/String/Raw/Object/ArrayToObject` | all 9 helpers on a fresh object, randomized names/values | `create.rs::cfg25_add_helpers` | [x] |
| 26 | `cJSON_Print` (A0, format=1) | every type from row 20 and randomized nested trees | `print.rs::cfg26_31_print_entry_points_{random_trees,explicit_shapes}` | [x] |
| 27 | `cJSON_PrintUnformatted` (A0, format=0) | same trees as row 26 | `print.rs::cfg26_31_print_entry_points_{random_trees,explicit_shapes}` | [x] |
| 28 | `cJSON_PrintBuffered` (A0, fmt=1) | `prebuffer` = 0, 1, 2, 8, 255, 256, 257, 1024, 65536 (crosses the `ensure` grow/`INT_MAX/2` logic) | `print.rs::cfg28_29_print_buffered_prebuffer_sweep` | [x] |
| 29 | `cJSON_PrintBuffered` (A0, fmt=0) | same prebuffer sweep | `print.rs::cfg28_29_print_buffered_prebuffer_sweep` | [x] |
| 30 | `cJSON_PrintPreallocated` (format=1) | `length` = exact, exact-1, exact+1, exact+5, 0, 1 — must agree on both the return value **and** the bytes written into the buffer | `print.rs::cfg30_31_print_preallocated_length_sweep` | [x] |
| 31 | `cJSON_PrintPreallocated` (format=0) | same length sweep | `print.rs::cfg30_31_print_preallocated_length_sweep` | [x] |
| 32 | `print_number` via all 4 print entry points | numbers that take each of the 4 formatting branches: NaN/±inf → `null`; integral == `valueint`; 15-significant-digit round-trip; 17-digit fallback; plus `valueint` desynchronised from `valuedouble` via a direct field write | `print.rs::cfg32_print_number_branches` | [x] |
| 33 | `print_string_ptr` via all 4 print entry points | `NULL` `valuestring`; no-escape fast path; every one-char escape; every byte 1..31; bytes 128..255; strings whose escape count changes the `output_length` arithmetic | `print.rs::cfg33_print_string_branches` | [x] |
| 34 | `print_object` (format=1) nesting | depth 1..8 objects/arrays → the `depth` indentation loops (1774, 1838) | `print.rs::cfg34_formatted_nesting_depth` | [x] |
| 35 | `cJSON_Parse` | valid JSON of every type, randomized generated documents | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 36 | `cJSON_ParseWithLength` | `len` = `strlen+1` (canonical), `strlen` (no NUL visible), `strlen+2`, and `len` shorter than the document | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 37 | `cJSON_ParseWithOpts(end = &p, rnt = 0)` | valid JSON with trailing garbage — `*end` offset compared | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 38 | `cJSON_ParseWithOpts(end = &p, rnt = 1)` | valid JSON, exact / with trailing whitespace / with trailing garbage | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 39 | `cJSON_ParseWithOpts(end = NULL, rnt = 1)` | same inputs, `return_parse_end` omitted | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 40 | `cJSON_ParseWithLengthOpts(len, end, rnt)` | full 2×2×(4 length variants) cross-product | `parse.rs::cfg35_43_{valid,invalid}_documents_all_modes` | [x] |
| 41 | parse: leading UTF-8 BOM (`skip_utf8_bom`, 1085) | `"\xEF\xBB\xBF" + doc`, and a BOM shorter than 4 readable bytes | `parse.rs::cfg35_43_valid_documents_all_modes (BOM corpus)` | [x] |
| 42 | parse: leading/interior/trailing whitespace (`buffer_skip_whitespace`, 1064) | all bytes `<= 32` as separators, incl. `\0`-adjacent edge where `offset == length` | `parse.rs::cfg42_whitespace_boundaries` | [x] |
| 43 | parse: numbers | `0`, `-0`, integers, `1e309` (→inf), `-1e309`, `1e-400` (→0), 15-digit, 17-digit, `INT_MAX`/`INT_MIN` boundaries, leading zeros, `1.`, `.5`-style rejects (see ERRORS) | `parse.rs::cfg43_parse_numbers` | [x] |
| 44 | parse: strings | every escape form, `\u` BMP, `\u` surrogate pairs (1/2/3/4-byte UTF-8 output), raw high bytes, empty string, embedded NUL via ` ` | `parse.rs::cfg44_parse_strings_exhaustive` | [x] |
| 45 | parse: arrays | `[]`, `[1]`, `[1,2,…]`, whitespace variants, nested to depth 999 | `parse.rs::cfg45_46_nesting` | [x] |
| 46 | parse: objects | `{}`, one key, many keys, duplicate keys, nested to depth 999, whitespace variants | `parse.rs::cfg45_46_nesting` | [x] |
| 47 | parse → print round-trip | parse each generated document, then print with all 4 entry points × `format` and compare | `parse.rs::cfg47_round_trip_generated` | [x] |
| 48 | `cJSON_GetErrorPtr` | after a successful parse, after each failing parse variant, after `cJSON_ParseWithOpts(NULL, …)` — compared as an **offset** into the input | `parse.rs::cfg48_error_ptr_state_machine` | [x] |
| 49 | `cJSON_Minify` | whitespace only, `//` comment (with/without trailing newline), `/* */` comment (unterminated), lone `/`, strings containing `\"`, `\\`, `//`, `/*`, and mixtures; buffer contents after the call compared byte-for-byte over the whole original length | `mutate.rs::cfg49_minify_{explicit,randomized}` | [x] |
| 50 | `cJSON_Duplicate(recurse = 0)` | every type, ± `IsReference`, ± `StringIsConst`, with/without children | `mutate.rs::cfg50_51_duplicate` | [x] |
| 51 | `cJSON_Duplicate(recurse = 1)` | nested trees to depth 8, arrays/objects/mixed, `StringIsConst` keys | `mutate.rs::cfg50_51_duplicate` | [x] |
| 52 | `cJSON_Compare(case_sensitive = 0)` | equal / unequal pairs of every type; objects with keys differing only in case; different orders; subsets | `mutate.rs::cfg52_53_compare` | [x] |
| 53 | `cJSON_Compare(case_sensitive = 1)` | same pairs as row 52 | `mutate.rs::cfg52_53_compare` | [x] |
| 54 | `cJSON_DetachItemViaPointer` | detach first / middle / last / only child of an array and of an object; then re-print the parent and the detached item | `mutate.rs::cfg54_60_detach_replace_via_pointer` | [x] |
| 55 | `cJSON_DetachItemFromArray` | index 0 / middle / last on sizes 1..6 | `mutate.rs::cfg54_55_57_59_detach_delete_insert_by_index` | [x] |
| 56 | `cJSON_DetachItemFromObject` / `…CaseSensitive` | key present (exact & other case) at first / middle / last position | `mutate.rs::cfg56_58_62_object_key_operations` | [x] |
| 57 | `cJSON_DeleteItemFromArray` | index 0 / middle / last | `mutate.rs::cfg54_55_57_59_detach_delete_insert_by_index` | [x] |
| 58 | `cJSON_DeleteItemFromObject` / `…CaseSensitive` | key at first / middle / last | `mutate.rs::cfg56_58_62_object_key_operations` | [x] |
| 59 | `cJSON_InsertItemInArray` | `which` = 0 / middle / last / `size` (append) on sizes 0..5 | `mutate.rs::cfg54_55_57_59_detach_delete_insert_by_index` | [x] |
| 60 | `cJSON_ReplaceItemViaPointer` | replace first / middle / last / only child of an array and an object | `mutate.rs::cfg54_60_detach_replace_via_pointer` | [x] |
| 61 | `cJSON_ReplaceItemInArray` | `which` = 0 / middle / last | `mutate.rs::cfg54_55_57_59_detach_delete_insert_by_index` | [x] |
| 62 | `cJSON_ReplaceItemInObject` / `…CaseSensitive` | key at first / middle / last, other-case key | `mutate.rs::cfg56_58_62_object_key_operations` | [x] |
| 63 | `cJSON_SetNumberHelper` | random `json_f64()` incl. saturation boundaries and NaN; return value + `valueint`/`valuedouble` | `mutate.rs::cfg63_set_number_helper` | [x] |
| 64 | `cJSON_SetValuestring` | new shorter / equal / longer than old; non-overlapping buffers; String and `String\|IsReference` items | `mutate.rs::cfg64_set_valuestring` | [x] |
| 65 | `cJSON_Delete` | every type, containers, `IsReference` items, `StringIsConst` keys, sibling chains (the `while (item)` loop) | `mutate.rs::cfg65_delete_sibling_chains` | [x] |
| 66 | `cJSON_InitHooks(NULL)` = **A0** | rows 26–34 re-run: `realloc` growth path | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 67 | **A1** libc `malloc`/`free` passed explicitly | rows 26–34 re-run: must still pick `realloc` (line 201 pointer equality) | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 68 | **A2** custom `malloc`+`free` | rows 26–34 re-run: `reallocate == NULL` ⇒ manual `allocate`+`memcpy`+`deallocate` in `ensure`/`print` | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 69 | **A3** custom `malloc`, `free_fn = NULL` | rows 26–34 re-run: manual path | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 70 | **A4** `malloc_fn = NULL`, custom `free` | rows 26–34 re-run: manual path | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 71 | **A5** both `NULL` | rows 26–34 re-run: back to `realloc` path | `hooks.rs::cfg66_71_all_hook_configurations + cfg66_71_cross_configuration_lifetimes` | [x] |
| 72 | `driver()` (`c_src/test.c`) | the full composed pipeline: create → `cJSON_Print` → `cJSON_PrintPreallocated` (sufficient **and** insufficient buffer) → `cJSON_Delete`, over randomized `strings[7]`, `numbers[3][3]`, `ids[4]`, `record[2]`; stdout compared byte-for-byte | `driver.rs::driver_differential` | [x] |
| 73 | direct `cJSON` struct manipulation (lowest level) | caller-built nodes with hand-set `type`/`valueint`/`valuedouble`/`valuestring`/`string`/`child`/`next`/`prev`, then fed to `print`/`Compare`/`Duplicate`/`Delete` — exercises paths no convenience wrapper reaches | `print.rs::cfg73_fabricated_nodes` | [x] |
| 74 | locale decimal point (`ENABLE_LOCALES` is ON) | `get_decimal_point()` under the default `"C"` locale; `parse_number`'s `.`→locale substitution and `print_number`'s reverse substitution | `print.rs::cfg74_locale_decimal_point` | [x] |

## Additional rows: allocator-observable behaviour

Two classes of defect are invisible to output comparison — an allocation that is
one byte too small, and a missing `deallocate`.  `cJSON_InitHooks` makes both
observable from outside, so `tests/guarded.rs` installs an allocator that wraps
every block in magic header/footer canaries (validated on free) and counts
allocations and frees.  Each row requires the C and the Rust library to make the
**same number of allocator calls, free everything they allocated, and never
overwrite a canary**.

| # | entry point(s) | configuration (options set + input shape) | covering test | [x] |
|---|----------------|--------------------------------------------|---------------|-----|
| 75 | build → all 4 print entry points → `cJSON_Delete` | guarded allocator; 160 randomized trees, depth 0–3, every print entry point and both formats | `guarded.rs::guarded_allocator_build_print_delete` | [x] |
| 76 | all 4 parse entry points | guarded allocator; 26 documents (valid and failing) × 4 entry points — failing parses must free everything too | `guarded.rs::guarded_allocator_parse` | [x] |
| 77 | `cJSON_AddItemToObject` / `…CS` re-key, `cJSON_ReplaceItemInObject`, `cJSON_SetValuestring` | guarded allocator; exercises `add_item_to_object`'s "free the previous key" branch and its `cJSON_StringIsConst` guard (a missing free here is a pure leak) | `guarded.rs::guarded_allocator_rekey` | [x] |
| 78 | `cJSON_Duplicate` / `cJSON_Compare` | guarded allocator; 60 randomized trees × `recurse` 0/1 × `case_sensitive` 0/1 | `guarded.rs::guarded_allocator_duplicate_and_compare` | [x] |
| 79 | `cJSON_Create*Array`, all 9 `cJSON_Add*ToObject`, both reference helpers | guarded allocator; counts 0/1/2/17/40 | `guarded.rs::guarded_allocator_typed_arrays_and_helpers` | [x] |
| 80 | detach / delete / insert / replace by index and by key | guarded allocator; object sizes 0–5 × every index and key class | `guarded.rs::guarded_allocator_mutations` | [x] |

## C-side build options (not Rust features)

`c_src/CMakeLists.txt` exposes compile-time options that change the C library
itself rather than its runtime configuration.  The libraries under test are built
with the defaults, which is the configuration the Rust translation targets:

| option | default | effect | how the translation matches it |
|--------|---------|--------|--------------------------------|
| `ENABLE_LOCALES` | `ON` | `get_decimal_point()` reads `localeconv()->decimal_point[0]` instead of hard-coding `'.'` | the Rust `get_decimal_point` also calls `localeconv()`; row 74 |
| `ENABLE_PUBLIC_SYMBOLS` | `ON` | adds `-fvisibility=hidden` and makes `CJSON_PUBLIC` expand to `__attribute__((visibility("default")))` | this is what determines the 79-symbol export list in SYMBOLS.md |
| `ENABLE_HIDDEN_SYMBOLS` | `OFF` | would define `CJSON_HIDE_SYMBOLS` and export nothing | n/a |
| `ENABLE_CUSTOM_COMPILER_FLAGS` | `ON` | compiles with `-std=c89`, which is why glibc's `NAN` is **not** defined and cJSON's own `#define NAN 0.0/0.0` (a *negative* quiet NaN) is used | the Rust translation returns the same `0xFFF8000000000000` bit pattern from `cJSON_GetNumberValue`; ERRORS.md rows 3/4 |
| `CJSON_NESTING_LIMIT` | 1000 | parse depth limit | rows 45/46, ERRORS.md rows 71/80 |
| `CJSON_CIRCULAR_LIMIT` | 10000 | `cJSON_Duplicate` depth limit | ERRORS.md rows 168/169 |
| build type | unset (`-O0`) | no optimisation | the suite is run against the Rust `.so` built in **both** the `release` and the `debug` profile (`verify.sh`), so unoptimised Rust codegen — which additionally enables overflow checks — is covered too |
