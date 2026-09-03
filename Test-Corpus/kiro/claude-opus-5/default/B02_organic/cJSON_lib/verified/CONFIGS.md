# CONFIGS.md — configuration surface (valid inputs) of `c_src/cJSON.c` + `c_src/test.c`

Derived mechanically from the C source: every runtime flag the public API can
set, every `if`/`switch`/`#ifdef` the C branches on, and every input **shape**
that is special-cased. One row per combination the C actually distinguishes.

## Axes found in the C source

**Runtime flags (all `cJSON_bool`, i.e. any `int` across FFI)**

| flag | set by | branch it controls |
|------|--------|--------------------|
| `require_null_terminated` | `cJSON_ParseWithOpts`, `cJSON_ParseWithLengthOpts` | trailing-garbage check |
| `format` / `fmt` | `cJSON_Print`(1), `cJSON_PrintUnformatted`(0), `cJSON_PrintBuffered`, `cJSON_PrintPreallocated` | `printbuffer.format` → indentation, `", "` vs `","`, `":\t"` vs `":"`, `"{\n"` vs `"{"` |
| `noalloc` | `cJSON_PrintPreallocated`(1) vs all others(0) | `ensure` refuses to grow |
| `case_sensitive` | `cJSON_GetObjectItem`(0)/`…CaseSensitive`(1), `Detach…`, `Replace…`, `cJSON_Compare` | `strcmp` vs `case_insensitive_strcmp` |
| `recurse` | `cJSON_Duplicate` | children copied or not |
| `boolean` | `cJSON_CreateBool`, `cJSON_AddBoolToObject` | `cJSON_True` vs `cJSON_False` |
| `constant_key` | `cJSON_AddItemToObject`(0) vs `cJSON_AddItemToObjectCS`(1) | `cJSON_StringIsConst`, key strdup'd or aliased |
| `cJSON_IsReference` | `cJSON_Create{String,Object,Array}Reference`, `cJSON_AddItemReferenceTo{Array,Object}` | `cJSON_Delete` skips child/valuestring; `cJSON_Duplicate` clears the bit |
| `prebuffer` | `cJSON_PrintBuffered` | initial `printbuffer.length` → number of `ensure` growth steps |
| `length` | `cJSON_PrintPreallocated` | success vs `false` |
| `buffer_length` | `cJSON_ParseWithLength{,Opts}` | `can_read`/`can_access_at_index` bounds; NUL not required |
| hooks | `cJSON_InitHooks` | `reallocate != NULL` → realloc path in `ensure`/`print`; `NULL` → allocate+memcpy path |
| `ENABLE_LOCALES` | compile-time (**ON** in `CMakeLists.txt`) | `get_decimal_point()` = `localeconv()->decimal_point[0]` |

**Input shapes the C special-cases**

- 8 item types (`cJSON_Invalid/False/True/NULL/Number/String/Array/Object/Raw`), masked `& 0xFF`; plus the `cJSON_IsReference`/`cJSON_StringIsConst` high bits.
- Numbers: `d == (double)valueint` (integer fast path) · `%1.15g` round-trip OK · `%1.15g` round-trip fails ⇒ `%1.17g` · `isnan`/`isinf` ⇒ `"null"` · `INT_MAX`/`INT_MIN` saturation · `-0.0`.
- Strings: empty · no-escape fast path (`memcpy`) · escape path (`\" \\ \b \f \n \r \t`) · `< 32` ⇒ `\u00xx` · raw bytes `> 127` passed through · `\u` BMP · `\u` surrogate pair.
- Containers: empty · 1 element · many · nesting depth 1 … 999 / 1000 / 1001 (`CJSON_NESTING_LIMIT`).
- Objects: empty key · duplicate keys · keys differing only in case · `string == NULL` child.
- Parse input: leading whitespace (`<= 32`) · UTF-8 BOM · trailing garbage · non-NUL-terminated slice · exact-length slice.
- `cJSON_Minify`: whitespace · `//` comment · `/* */` comment · `/` not a comment · string with `\"` · unterminated forms.
- Print buffer growth: fits in the initial 256 bytes vs forces one or more `ensure` reallocations (`needed*2` vs `INT_MAX` clamp).

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | covering test | [ ] |
|---|----------------|--------------------------------------------|---------------|-----|
| 1 | `cJSON_Parse` → `cJSON_Print` | round-trip, randomized JSON documents (all 8 types mixed), `format=1` || `phase_b_valid::row_1_2_15_35_randomized_documents` | [x] |
| 2 | `cJSON_Parse` → `cJSON_PrintUnformatted` | same corpus, `format=0` || `phase_b_valid::row_1_2_15_35_randomized_documents` | [x] |
| 3 | `cJSON_ParseWithOpts` | `return_parse_end = NULL`, `require_null_terminated = 0` || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 4 | `cJSON_ParseWithOpts` | `return_parse_end != NULL`, `require_null_terminated = 0`, trailing garbage present — compare returned end offset || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 5 | `cJSON_ParseWithOpts` | `return_parse_end != NULL`, `require_null_terminated = 1`, clean input || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 6 | `cJSON_ParseWithOpts` | `require_null_terminated` ∈ {2, -1, INT_MAX, INT_MIN} (out-of-range bool) || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 7 | `cJSON_ParseWithLength` | `buffer_length` = exact `strlen` (no NUL visible) || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 8 | `cJSON_ParseWithLength` | `buffer_length` = `strlen + 1` || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 9 | `cJSON_ParseWithLength` | `buffer_length` < `strlen` (truncated mid-token, every truncation point) || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 10 | `cJSON_ParseWithLength` | `buffer_length` > `strlen + 1` (embedded NUL then garbage) || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 11 | `cJSON_ParseWithLengthOpts` | all 4 args exercised: length × `return_parse_end` × `require_null_terminated` || `phase_b_valid::rows_3_11_parse_entry_points_matrix` | [x] |
| 12 | `cJSON_Parse` | leading whitespace / tabs / CR / LF / bytes `<= 32` before the value || `phase_b_valid::row_12_whitespace_sprinkled_documents` | [x] |
| 13 | `cJSON_Parse` | UTF-8 BOM `EF BB BF` prefix (and BOM-only input) || `phase_b_valid::row_13_utf8_bom` | [x] |
| 14 | `cJSON_Parse` + `cJSON_GetErrorPtr` | failing parses — compare error offset for every prefix of a corpus || `phase_b_valid::row_14_error_pointer_for_every_prefix` | [x] |
| 15 | `cJSON_Parse` | scalar `null` / `true` / `false` alone || `phase_b_valid::row_15_16_scalars_and_numbers` | [x] |
| 16 | `cJSON_Parse` | numbers: integers, negatives, `0`, `-0`, exponents, `1e309` (inf), `1e-320` (subnormal), 17-sig-digit values, `2147483647`, `2147483648`, `-2147483648`, `-2147483649` || `phase_b_valid::row_15_16_scalars_and_numbers` | [x] |
| 17 | `cJSON_Parse` | strings: empty, ASCII, all single-char escapes, `\u0000`–`\uFFFF` sweep, surrogate pairs, raw UTF-8 ≥ 0x80, control bytes || `phase_b_valid::row_17_strings_and_escapes` | [x] |
| 18 | `cJSON_Parse` | arrays: `[]`, `[x]`, many elements, mixed types || `phase_b_valid::row_18_19_containers` | [x] |
| 19 | `cJSON_Parse` | objects: `{}`, one key, many keys, duplicate keys, empty key `""`, keys differing only in case || `phase_b_valid::row_18_19_containers` | [x] |
| 20 | `cJSON_Parse` | nesting depth 1, 2, 998, 999, 1000, 1001 for `[` and `{` (`CJSON_NESTING_LIMIT`) || `phase_b_valid::row_20_nesting_limit + phase_c_errors::rows_85_95_nesting_limit_exact_boundary` | [x] |
| 21 | `cJSON_Print` | randomized trees, `format=1`, output larger than the 256-byte initial buffer (forces `ensure` growth) || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 22 | `cJSON_PrintUnformatted` | same trees, `format=0` || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 23 | `cJSON_PrintBuffered` | `prebuffer` ∈ {0, 1, 2, 8, 255, 256, 257, exact, exact+1, 4096} × `fmt` ∈ {0,1} || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 24 | `cJSON_PrintBuffered` | `fmt` ∈ {2, -1, INT_MAX} (out-of-range bool) || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 25 | `cJSON_PrintPreallocated` | `length` = exact rendered length + 1 (success), `fmt` ∈ {0,1} || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 26 | `cJSON_PrintPreallocated` | `length` swept from 0 to len+8 — find the exact success threshold (`noalloc` + `ensure`) || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 27 | `cJSON_PrintPreallocated` | `format` ∈ {2, -1} || `phase_b_valid::row_1_2_15_35_randomized_documents (render_all + preallocated_profile)` | [x] |
| 28 | `cJSON_Print*` | item types: Invalid(0), False, True, NULL, Number, String, Array, Object, Raw || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 29 | `cJSON_Print*` | `cJSON_Raw` item whose `valuestring` is arbitrary text (copied verbatim) || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 30 | `cJSON_Print*` | numbers requiring the `%1.17g` fallback vs the `%1.15g` path vs the integer `%d` path || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 31 | `cJSON_Print*` | number = NaN, +inf, -inf ⇒ `"null"`; `-0.0` || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 32 | `cJSON_Print*` | strings on the no-escape `memcpy` fast path vs the escape path vs `<32` `\u00xx` path || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 33 | `cJSON_Print*` | `valuestring == NULL` on a String item ⇒ `""` || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 34 | `cJSON_Print*` | object with `string == NULL` key ⇒ `""` key || `phase_b_valid::rows_28_34_print_handbuilt_items` | [x] |
| 35 | `cJSON_Print*` (formatted) | nested objects/arrays depth 1..6 — indentation (`depth` tabs) || `phase_b_valid::row_18_19_containers + row_1_2_15_35_randomized_documents` | [x] |
| 36 | `cJSON_CreateNumber` | value sweep: ints, fractions, `INT_MAX±1`, `INT_MIN±1`, `1e300`, `-1e300`, NaN, ±inf, `-0.0` — compare `valueint`+`valuedouble` bits || `phase_b_valid::row_36_create_number_sweep` | [x] |
| 37 | `cJSON_CreateBool` | `boolean` ∈ {0, 1, 2, -1, INT_MAX, INT_MIN} || `phase_b_api::row_37_52_create_bool_and_add_bool` | [x] |
| 38 | `cJSON_CreateString` / `CreateRaw` | empty, ASCII, embedded escapes, high bytes || `phase_b_api::row_38_create_string_and_raw` | [x] |
| 39 | `cJSON_CreateStringReference` | valid string, then `cJSON_Print` and `cJSON_Delete` (`cJSON_IsReference` ⇒ valuestring not freed) || `phase_b_api::row_39_40_reference_items` | [x] |
| 40 | `cJSON_CreateObjectReference` / `CreateArrayReference` | referencing a live child list; print + delete || `phase_b_api::row_39_40_reference_items` | [x] |
| 41 | `cJSON_CreateIntArray` | `count` ∈ {0, 1, 2, 16, 256} × randomized `i32` values incl. `INT_MIN`/`INT_MAX` || `phase_b_api::rows_41_44_typed_array_constructors` | [x] |
| 42 | `cJSON_CreateFloatArray` | `count` ∈ {0,1,2,16} × randomized `f32` incl. subnormals, NaN, ±inf || `phase_b_api::rows_41_44_typed_array_constructors` | [x] |
| 43 | `cJSON_CreateDoubleArray` | `count` ∈ {0,1,2,16} × randomized `f64` incl. NaN, ±inf, `-0.0` || `phase_b_api::rows_41_44_typed_array_constructors` | [x] |
| 44 | `cJSON_CreateStringArray` | `count` ∈ {0,1,2,7,16} × randomized strings (as used by `driver`) || `phase_b_api::rows_41_44_typed_array_constructors` | [x] |
| 45 | `cJSON_AddItemToArray` | append into empty / 1-element / n-element array; then print + `GetArraySize` || `phase_b_api::rows_45_48_add_item_to_object_const_vs_copied` | [x] |
| 46 | `cJSON_AddItemToObject` | `constant_key = 0` — key strdup'd, `cJSON_StringIsConst` cleared || `phase_b_api::rows_45_48_add_item_to_object_const_vs_copied` | [x] |
| 47 | `cJSON_AddItemToObjectCS` | `constant_key = 1` — key aliased, `cJSON_StringIsConst` set; verify `type` bits and print || `phase_b_api::rows_45_48_add_item_to_object_const_vs_copied` | [x] |
| 48 | `cJSON_AddItemToObject` then re-add same item under a new key | old `item->string` freed (non-const) vs kept (const) || `phase_b_api::rows_45_48_add_item_to_object_const_vs_copied` | [x] |
| 49 | `cJSON_AddItemReferenceToArray` | reference to a subtree; print parent, delete parent, subtree survives || `phase_b_api::rows_49_50_add_item_reference` | [x] |
| 50 | `cJSON_AddItemReferenceToObject` | same, keyed || `phase_b_api::rows_49_50_add_item_reference` | [x] |
| 51 | `cJSON_Add{Null,True,False,Bool,Number,String,Raw,Object,Array}ToObject` | all 9 helpers on a fresh object, then print || `phase_b_api::row_37_52_create_bool_and_add_bool (build_kitchen_sink)` | [x] |
| 52 | `cJSON_AddBoolToObject` | `boolean` ∈ {0,1,2,-1} || `phase_b_api::row_37_52_create_bool_and_add_bool (build_kitchen_sink)` | [x] |
| 53 | `cJSON_GetArraySize` | array of size 0..n; object; scalar; reference item || `phase_b_api::rows_53_58_queries` | [x] |
| 54 | `cJSON_GetArrayItem` | `index` = 0, mid, size-1, size, size+1 over sizes 0..n || `phase_b_api::rows_53_58_queries` | [x] |
| 55 | `cJSON_GetObjectItem` | `case_sensitive = 0`: exact key, differing-case key, absent key, duplicate keys (first match) || `phase_b_api::rows_53_58_queries` | [x] |
| 56 | `cJSON_GetObjectItemCaseSensitive` | `case_sensitive = 1`: same set || `phase_b_api::rows_53_58_queries` | [x] |
| 57 | `cJSON_HasObjectItem` | present / absent / differing case || `phase_b_api::rows_53_58_queries` | [x] |
| 58 | `cJSON_GetStringValue` / `cJSON_GetNumberValue` | on each of the 9 item types || `phase_b_api::rows_53_58_queries` | [x] |
| 59 | `cJSON_DetachItemViaPointer` | detach first / middle / last / only element of an array and of an object; then print parent || `phase_b_api::rows_59_60_62_detach_and_delete_array` | [x] |
| 60 | `cJSON_DetachItemFromArray` | `which` = 0, mid, last, out-of-range, over sizes 0..n || `phase_b_api::rows_59_60_62_detach_and_delete_array` | [x] |
| 61 | `cJSON_DetachItemFromObject{,CaseSensitive}` | present, differing case, absent || `phase_b_api::rows_61_63_detach_and_delete_object` | [x] |
| 62 | `cJSON_DeleteItemFromArray` | `which` = 0, mid, last, out-of-range || `phase_b_api::rows_59_60_62_detach_and_delete_array` | [x] |
| 63 | `cJSON_DeleteItemFromObject{,CaseSensitive}` | present, differing case, absent || `phase_b_api::rows_61_63_detach_and_delete_object` | [x] |
| 64 | `cJSON_InsertItemInArray` | `which` = 0 (head), mid, size-1, size (append fallback), size+1, over sizes 0..n || `phase_b_api::row_64_insert_item_in_array` | [x] |
| 65 | `cJSON_ReplaceItemViaPointer` | replace first (single-element and multi-element), middle, last; `replacement == item` || `phase_b_api::rows_65_66_replace_in_array` | [x] |
| 66 | `cJSON_ReplaceItemInArray` | `which` = 0, mid, last, out-of-range || `phase_b_api::rows_65_66_replace_in_array` | [x] |
| 67 | `cJSON_ReplaceItemInObject` | `case_sensitive = 0`: present, differing case, absent || `phase_b_api::rows_67_68_replace_in_object` | [x] |
| 68 | `cJSON_ReplaceItemInObjectCaseSensitive` | `case_sensitive = 1`: same set || `phase_b_api::rows_67_68_replace_in_object` | [x] |
| 69 | `cJSON_Duplicate` | `recurse = 0` on every item type (children dropped) || `phase_b_api::rows_69_73_duplicate` | [x] |
| 70 | `cJSON_Duplicate` | `recurse = 1` on deep randomized trees || `phase_b_api::rows_69_73_duplicate` | [x] |
| 71 | `cJSON_Duplicate` | `recurse` ∈ {2, -1} (out-of-range bool) || `phase_b_api::rows_69_73_duplicate` | [x] |
| 72 | `cJSON_Duplicate` | item with `cJSON_IsReference` set (bit must be cleared in the copy) || `phase_b_api::rows_69_73_duplicate` | [x] |
| 73 | `cJSON_Duplicate` | item with `cJSON_StringIsConst` key (pointer aliased, not strdup'd) || `phase_b_api::rows_69_73_duplicate` | [x] |
| 74 | `cJSON_Duplicate` at nesting depth 10, 9998, 9999, 10000, 10001, 10002 | `CJSON_CIRCULAR_LIMIT` (10000). `cJSON_Duplicate_rec` itself is `-fvisibility=hidden` in the C build, so its `depth` argument is driven indirectly by tree depth || `phase_b_api::row_74_duplicate_circular_limit` | [x] |
| 75 | `cJSON_Compare` | `case_sensitive = 0`, equal / unequal pairs across all 9 types || `phase_b_api::rows_75_78_compare` | [x] |
| 76 | `cJSON_Compare` | `case_sensitive = 1`, same pairs plus case-differing object keys || `phase_b_api::rows_75_78_compare` | [x] |
| 77 | `cJSON_Compare` | `case_sensitive` ∈ {2, -1}; `a == b` aliasing; nested arrays/objects; subset objects || `phase_b_api::rows_75_78_compare` | [x] |
| 78 | `cJSON_Compare` | Number pairs straddling the `compare_double` / `DBL_EPSILON` threshold || `phase_b_api::rows_75_78_compare` | [x] |
| 79 | `cJSON_Minify` | whitespace only, `//` comment (with and without trailing `\n`), `/* */`, unterminated `/*`, lone `/`, string containing `\"`, `//` inside a string, unterminated string, empty input || `phase_b_api::row_79_minify_edge_cases` | [x] |
| 80 | `cJSON_Minify` | randomized JSON documents with injected comments/whitespace — compare minified bytes || `phase_b_api::row_80_minify_randomized` | [x] |
| 81 | `cJSON_SetValuestring` | new shorter than old (in-place `strcpy`), equal length, longer (realloc), on a Raw item, on a reference item || `phase_b_api::row_81_set_valuestring` | [x] |
| 82 | `cJSON_SetNumberHelper` | value sweep incl. `INT_MAX`/`INT_MIN` boundaries, NaN, ±inf — compare `valueint`/`valuedouble` || `phase_b_valid::row_82_set_number_helper_sweep` | [x] |
| 83 | `cJSON_Version` | returned string bytes || `phase_b_valid::rows_83_84_version_malloc_free` | [x] |
| 84 | `cJSON_malloc` / `cJSON_free` | default hooks; sizes 0, 1, 4096 || `phase_b_valid::rows_83_84_version_malloc_free` | [x] |
| 85 | `cJSON_InitHooks` | `hooks = NULL` (reset) then full parse/print round-trip || `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 86 | `cJSON_InitHooks` | custom malloc **and** custom free ⇒ `reallocate = NULL` ⇒ `ensure`/`print` take the allocate+memcpy path; full round-trip with growth || `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 87 | `cJSON_InitHooks` | `hooks->malloc_fn = NULL`, `free_fn` custom || `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 88 | `cJSON_InitHooks` | `hooks->free_fn = NULL`, `malloc_fn` custom || `phase_bc_hooks::rows_85_88_hook_configurations` | [x] |
| 89 | `cJSON_InitHooks` | custom hooks that count allocations — compare the **allocation sequence** (sizes, order, count) between C and Rust for a fixed workload || `phase_bc_hooks::row_86_89_allocation_sequence_matches + ::init_hooks_realloc_selection_is_observable_via_alloc_trace` | [x] |
| 90 | `driver` (`test.c`) | full end-to-end run with randomized `strings[7]`, `numbers[3][3]`, `ids[4]`, `record fields[2]` — compare captured stdout byte-for-byte || `phase_b_driver::row_90_driver_stdout_matches` | [x] |
| 91 | composed pipeline | parse → mutate (insert/replace/detach) → duplicate → compare → print, randomized program of operations, fixed seed || `phase_b_api::rows_91_92_composed_pipeline` | [x] |
| 92 | print after mutation | print an object/array whose links were rewired by `Detach`/`Insert`/`Replace` (exercises `child->prev` bookkeeping) || `phase_b_api::rows_91_92_composed_pipeline` | [x] |
| 93 | locale | `ENABLE_LOCALES=ON`; process locale left at `"C"` ⇒ `decimal_point == '.'` (the only locale guaranteed present); both libraries observe the same `localeconv()` || `all of the above (process locale left at "C"; both libraries call the same `localeconv`)` | [x] |

## Result

All **93** rows pass across randomized inputs (fixed seeds, see
`Rng::new(0x5EED_....)` in each test) under every configuration verified by
`scripts/verify_all.sh`.

### Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so there is exactly one
feature configuration. `scripts/verify_all.sh` still enumerates the table
mechanically and runs the whole suite for the default set *and* for
`--no-default-features`, in both the `dev` and `release` cargo profiles
(`release` additionally enables `panic = "abort"`), giving 4 verified
configurations. If a `[features]` table is ever added, the script expands to the
full power set automatically.
