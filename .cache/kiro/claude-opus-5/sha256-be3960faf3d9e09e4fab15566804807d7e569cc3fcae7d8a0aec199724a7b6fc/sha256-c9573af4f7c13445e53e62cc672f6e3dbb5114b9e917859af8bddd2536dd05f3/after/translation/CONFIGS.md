# CONFIGS.md — Configuration-surface table (valid inputs)

Mechanically derived from the axes the C code actually branches on.

## Axes found in the C source

| axis | values the C distinguishes | where |
|---|---|---|
| `js_newstate` flags | `0`, `JS_STRICT` (bit 0); other bits ignored | `jsstate.c:204` |
| allocator | default (`alloc == NULL`) vs. custom `js_Alloc` | `jsstate.c:186`, `js_malloc`/`js_realloc`/`js_free` |
| `js_setreport` | report callback unset vs. set | `jserror.c`, `js_report` |
| `js_setlimit` | `runlimit`/`memlimit` unset (0) vs. set | `jsstate.c`, `jsgc.c` |
| regexp compile flags | `0`, `REG_ICASE`, `REG_NEWLINE`, `REG_ICASE\|REG_NEWLINE` | `regexp.c` `regcompx` |
| regexp exec flags | `0`, `REG_NOTBOL` | `regexp.c` `regexec` |
| JS RegExp flags | `JS_REGEXP_G` (1) / `I` (2) / `M` (4) — 8 combos | `jsregexp.c:160-177` |
| property attributes | `JS_READONLY` (1) / `JS_DONTENUM` (2) / `JS_DONTCONF` (4) — 8 combos | `jsproperty.c`, `jsrun.c` |
| `js_pushiterator` | `own = 0` vs `own = 1` | `jsproperty.c:290` |
| `js_gc` | `report = 0` vs `report = 1` | `jsgc.c` |
| value type | 7 `js_type()` results: undefined, null, boolean, number, string, function, object | `jsvalue.c`, `mujs.h` |
| string shape | empty / 1 byte / ASCII / 2-3-4-byte UTF-8 / invalid UTF-8 / embedded NUL / long (> shortstring inline limit) | `jsvalue.c`, `jsintern.c`, `utf.c` |
| number shape | `+0`, `-0`, subnormal, NaN, `+-Inf`, `+-1`, `2^31-1`, `2^31`, `-2^31`, `2^32`, `2^16` boundaries, huge, fractional | `jsvalue.c` `jsV_numberto*` |
| radix | 2..36 for `Number.prototype.toString` | `jsnumber.c:40` |
| precision | `toFixed`/`toExponential` 0..20, `toPrecision` 1..21 | `jsnumber.c:134-168` |
| array shape | empty / 1 / many / sparse (holes) / flat vs. unflattened | `jsarray.c`, `jsV_resizearray`, `jsR_unflattenarray` |
| stack index | positive, negative, `0`, `-1`, top | `jsrun.c` `stackidx` |
| numeric parse | decimal / hex `0x` / octal-ish / leading+trailing space / sign / exponent | `jsdtoa.c` `js_strtod`, `js_strtol` |

No `[features]` section exists in `translation/Cargo.toml`, so the crate has
exactly **one** feature configuration (the default). Phase D's "every feature
combination" therefore collapses to the default build, which is verified below.
`cargo check`/`cargo test --no-default-features` are still run to confirm.

## Status

All **62 rows** pass: for each one, both `.so`s are driven through the same
sequence of exported calls in that configuration and every intermediate
observation is compared byte-for-byte. Rows are exercised with many randomized
inputs from a fixed seed (`Rng`, xorshift64*), not a single hand-picked value.
Every row is run twice, once with `js_newstate(..., 0)` and once with
`js_newstate(..., JS_STRICT)`.

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [x] | covering test |
|---|----------------|--------------------------------------------|-----|----------------|
| 1 | `jsU_chartorune` / `jsU_runetochar` / `jsU_runelen` | round-trip over every rune 0..0x10FFFF | [x] | `leaf_pure::utf_runetochar_runelen_every_rune` |
| 2 | `jsU_chartorune` | all 256 single bytes as a lead byte | [x] | `leaf_pure::utf_chartorune_all_lead_bytes` |
| 3 | `jsU_chartorune` | randomized 1-4 byte sequences incl. malformed | [x] | `leaf_pure::utf_chartorune_all_lead_bytes` |
| 4 | `jsU_isalpharune` / `islowerrune` / `isupperrune` | every rune 0..0x10FFFF | [x] | `leaf_pure::utf_case_tables_every_rune` |
| 5 | `jsU_tolowerrune` / `jsU_toupperrune` | every rune 0..0x10FFFF | [x] | `leaf_pure::utf_case_tables_every_rune` |
| 6 | `jsU_tolowerrune_full` / `jsU_toupperrune_full` | every rune 0..0x10FFFF, NULL and multi-rune results | [x] | `leaf_pure::utf_case_tables_every_rune` |
| 7 | `js_utflen` / `js_utfptrtoidx` | empty / ASCII / multi-byte / invalid UTF-8 | [x] | `leaf_pure::utf_len_and_ptrtoidx` |
| 8 | `jsY_iswhite` / `jsY_isnewline` / `jsY_ishex` / `jsY_tohex` | every int in -1..0x2100 | [x] | `leaf_pure::lex_char_class_helpers` |
| 9 | `jsY_tokenstring` | every token id 0..300 (incl. out-of-range) | [x] | `leaf_pure::lex_tokenstring_all_ids` |
| 10 | `jsY_findword` | sorted lists of size 0/1/many, hit and miss | [x] | `leaf_pure::lex_findword` |
| 11 | `js_itoa` | `INT_MIN`, `INT_MAX`, 0, randomized ints | [x] | `leaf_pure::dtoa_itoa` |
| 12 | `js_grisu2` | randomized positive doubles + all binade boundaries | [x] | `leaf_pure::dtoa_grisu2` |
| 13 | `js_fmtexp` | e in -400..400 | [x] | `leaf_pure::dtoa_fmtexp` |
| 14 | `js_strtod` | decimal / hex / exponent / signs / whitespace / junk, randomized | [x] | `leaf_pure::dtoa_strtod` |
| 15 | `js_strtol` | radix 2..36 × valid/invalid digits, randomized | [x] | `leaf_pure::dtoa_strtol` |
| 16 | `js_stringtofloat` | same shapes as `js_strtod` | [x] | `leaf_pure::dtoa_stringtofloat` |
| 17 | `jsV_numbertoint32` / `touint32` / `toint16` / `touint16` / `tointeger` | full number-shape axis + randomized doubles | [x] | `leaf_pure::value_number_coercions` |
| 18 | `js_regcomp` + `js_regexec` | cflags `0`, eflags `0` — literal patterns, randomized subjects | [x] | `regexp_engine::regexp_handwritten_corpus_all_flags + regexp_randomized_property` |
| 19 | `js_regcomp` + `js_regexec` | cflags `REG_ICASE`, eflags `0` | [x] | `regexp_engine::regexp_handwritten_corpus_all_flags + regexp_randomized_property` |
| 20 | `js_regcomp` + `js_regexec` | cflags `REG_NEWLINE`, eflags `0` | [x] | `regexp_engine::regexp_handwritten_corpus_all_flags + regexp_randomized_property` |
| 21 | `js_regcomp` + `js_regexec` | cflags `REG_ICASE\|REG_NEWLINE`, eflags `0` | [x] | `regexp_engine::regexp_handwritten_corpus_all_flags + regexp_randomized_property` |
| 22 | `js_regcomp` + `js_regexec` | each cflags combo × eflags `REG_NOTBOL` | [x] | `regexp_engine::regexp_handwritten_corpus_all_flags + regexp_exec_edge_cases` |
| 23 | `js_regcompx` | custom allocator, all cflags combos, `regfreex` | [x] | `regexp_engine::regexp_custom_allocator` |
| 24 | `js_regexec` | captures: 0, 1, up to REG_MAXSUB-1 groups; `sub` NULL vs non-NULL | [x] | `regexp_engine::regexp_capture_counts` |
| 25 | `js_regcomp` + `js_regexec` | anchors `^`/`$`, lookahead `(?=)`/`(?!)`, backrefs, classes, quantifiers `*+?{n,m}` — randomized pattern generator | [x] | `regexp_engine::regexp_randomized_property{,_long_subjects}` |
| 26 | `js_newstate(NULL,NULL,0)` | non-strict, default allocator: `js_dostring` over the JS program corpus | [x] | `interp_lang::* , interp_builtins::* (flags=0)` |
| 27 | `js_newstate(NULL,NULL,JS_STRICT)` | strict mode: same corpus | [x] | `interp_lang::* , interp_builtins::* (flags=1)` |
| 28 | `js_newstate` with custom `js_Alloc` | flags `0` and `JS_STRICT`, same corpus | [x] | `state_api::state_custom_allocator` |
| 29 | `js_newstate` flags out of range (`0xFFFE`, `-1`, `0xFFFF`) | only bit 0 honoured; behaviour equals `0` / `JS_STRICT` | [x] | `state_api::state_flags_including_out_of_range, interp_errors::err_out_of_range_enum_values` |
| 30 | `js_setreport` set | program that triggers `js_report` (unhandled error in `js_dostring`) | [x] | `state_api::state_report_callback` |
| 31 | `js_gc(J, 0)` and `js_gc(J, 1)` | after allocating garbage; then continue running | [x] | `state_api::state_gc` |
| 32 | `js_setlimit` | runlimit/memlimit unset vs. set (large, and small enough to trip) | [x] | `state_api::state_setlimit` |
| 33 | push/type API | `js_pushundefined/null/boolean/number/string/lstring/literal/global` × `js_type`/`js_typeof`/`js_is*` | [x] | `state_api::api_predicates_and_conversions_over_all_values` |
| 34 | `js_pushlstring` | embedded NUL, invalid UTF-8, length 0, long strings | [x] | `state_api::api_pushlstring_lengths` |
| 35 | stack manipulation | `js_gettop/pop/rot/copy/remove/insert/replace/dup/dup2/rot2/rot3/rot4/rot2pop1/rot3pop2` over positive and negative indices | [x] | `state_api::api_stack_manipulation, api_stack_overflow, api_randomized_scripts` |
| 36 | conversion API | `js_toboolean/tonumber/tostring/tointeger/toint32/touint32/toint16/touint16` over all 7 types × number/string shapes | [x] | `state_api::api_predicates_and_conversions_over_all_values` |
| 37 | `js_try*` API | `js_trystring/trynumber/tryinteger/tryboolean/tryrepr` on coercible and non-coercible values | [x] | `state_api::api_predicates_and_conversions_over_all_values` |
| 38 | `js_newobject/newarray/newboolean/newnumber/newstring/newobjectx` | then property get/set/has/del | [x] | `state_api::api_predicates_and_conversions_over_all_values, api_properties_all_attribute_combos` |
| 39 | `js_defproperty` / `js_defglobal` | all 8 attribute combos, then read/write/delete/enumerate | [x] | `state_api::api_properties_all_attribute_combos, api_globals_all_attribute_combos` |
| 40 | `js_defaccessor` | all 8 attribute combos, getter+setter, getter only, setter only | [x] | `state_api::api_properties_all_attribute_combos` |
| 41 | index API | `js_getlength/setlength/hasindex/getindex/setindex/delindex` — empty/1/many/sparse arrays | [x] | `state_api::api_indices_and_lengths, api_array_length_limits` |
| 42 | `js_pushiterator(idx, 0)` and `(idx, 1)` + `js_nextiterator` | plain object, array, object with `JS_DONTENUM` props, prototype chain | [x] | `state_api::api_iterators` |
| 43 | `js_newregexp` | all 8 `JS_REGEXP_*` flag combos, then `exec`/`test`/`String.replace` | [x] | `state_api::api_newregexp_all_flags` |
| 44 | `js_newcfunction` / `js_newcfunctionx` / `js_newcconstructor` | length 0/1/n, `js_currentfunction`, `js_currentfunctiondata`, finalize | [x] | `state_api::api_cfunction_data, api_predicates_and_conversions_over_all_values` |
| 45 | `js_newuserdata` / `js_newuserdatax` | tag match/mismatch, `has`/`put`/`delete` callbacks, `js_isuserdata`, `js_touserdata` | [x] | `state_api::api_userdata_and_registry, api_userdatax_callbacks` |
| 46 | `js_ref` / `js_unref` / `js_getregistry` / `js_setregistry` / `js_delregistry` | round-trip, missing key | [x] | `state_api::api_userdata_and_registry` |
| 47 | `js_getglobal/setglobal/delglobal` | existing, missing, shadowed names | [x] | `state_api::api_globals_all_attribute_combos` |
| 48 | `js_compare/equal/strictequal/instanceof/concat` | cross-product of the 7 types | [x] | `state_api::api_operators_cross_product` |
| 49 | `js_repr` / `js_torepr` / `js_tryrepr` | all 7 types, nested objects/arrays, cyclic | [x] | `state_api::api_repr` |
| 50 | `js_ploadstring` + `js_pcall` | separate load/call, `n` = 0/1/many arguments | [x] | `state_api::state_pload_pcall_pconstruct` |
| 51 | `js_pconstruct` | `n` = 0/1/many, constructor returning object vs. primitive | [x] | `state_api::state_pload_pcall_pconstruct` |
| 52 | `js_dostring` | JS corpus: operators, closures, prototypes, `try/catch/finally`, `switch`, labels, `for-in`, `with` (non-strict) | [x] | `interp_lang::lang_expressions_and_statements, lang_lexer_and_asi, lang_randomized_programs` |
| 53 | `js_dostring` | `Number.prototype.toString` radix 2..36 over randomized doubles | [x] | `interp_builtins::builtin_number_tostring_radix` |
| 54 | `js_dostring` | `toFixed` 0..20, `toExponential` 0..20, `toPrecision` 1..21 over randomized doubles | [x] | `interp_builtins::builtin_number_formatting_precision` |
| 55 | `js_dostring` | `String` methods over ASCII/UTF-8/empty/long inputs, randomized | [x] | `interp_builtins::builtin_string_methods` |
| 56 | `js_dostring` | `Array` methods incl. `sort` with and without comparator, sparse arrays, randomized | [x] | `interp_builtins::builtin_array_methods` |
| 57 | `js_dostring` | `Math` functions over randomized doubles and special values | [x] | `interp_builtins::builtin_math` |
| 58 | `js_dostring` | `JSON.parse`/`JSON.stringify` with replacer, reviver, indent string/number | [x] | `interp_builtins::builtin_json` |
| 59 | `js_dostring` | `Date` construction/getters/`toISOString`/parsing (fixed TZ=UTC) | [x] | `interp_builtins::builtin_date` |
| 60 | `js_dostring` | `RegExp` via JS literals: `exec`/`test`/`match`/`replace`/`split`/`search` × g/i/m | [x] | `interp_lang::lang_regexp_via_js` |
| 61 | `js_dostring` | deep recursion / large arrays / GC pressure (exercises `jsgc.c`, `jsR_unflattenarray`) | [x] | `interp_lang::lang_gc_and_large_data` |
| 62 | `jsV_numbertostring` | full number-shape axis + randomized doubles (via `js_dostring` `String(n)` and directly) | [x] | `leaf_pure::value_numbertostring, interp_builtins::builtin_number_to_string_conversion` |
