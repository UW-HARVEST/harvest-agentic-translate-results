# Configuration surface

Derived from public declarations plus the flag, type, size, count, and format
branches in `value.c`, `dump.c`, `load.c`, `pack_unpack.c`, `utf.c`,
`strconv.c`, `strbuffer.c`, `hashtable.c`, `memory.c`, `dtoa.c`, and
`version.c`. Cargo declares no optional features, so the only feature
combination is the default/no-feature build.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jansson_version_str`, `jansson_version_cmp` | exact version and major/minor/micro below, equal, and above | [x] |
| 2 | `json_true`, `json_false`, `json_null` | singleton construction and repeated calls | [x] |
| 3 | `json_integer`, getters/setter | zero, ±1, `INT64_MIN`, `INT64_MAX`, randomized full-width values | [x] |
| 4 | `json_real`, getters/setter | ±0, normal, subnormal, large/small finite randomized values | [x] |
| 5 | `json_number_value` | integer, real, and every non-number type | [x] |
| 6 | `json_string[n][_nocheck]` | empty, one byte, embedded NUL with explicit length, ASCII many | [x] |
| 7 | `json_string[n][_nocheck]` | valid 2-, 3-, and 4-byte UTF-8, mixed Unicode | [x] |
| 8 | string getters/setters | shorter, equal, longer replacement; checked and nocheck forms | [x] |
| 9 | `json_object[_size]`, seed | empty object; explicit zero/nonzero seeds before construction | [x] |
| 10 | object set/get | one key; C-string and explicit-length variants | [x] |
| 11 | object set/get | embedded-NUL key through `*_n*_nocheck` | [x] |
| 12 | object set/get | replace existing key; empty/one/many keys | [x] |
| 13 | object delete/clear | delete first/middle/last; clear empty and many | [x] |
| 14 | object iterators | empty, one, many; `iter`, `iter_at`, key, key_len, value, next | [x] |
| 15 | object iterator setter | replace values while iterating | [x] |
| 16 | object update | disjoint and overlapping keys | [x] |
| 17 | object update existing | overlap plus missing source keys | [x] |
| 18 | object update missing | overlap plus missing destination keys | [x] |
| 19 | object update recursive | nested object/object merge and scalar replacement | [x] |
| 20 | `json_array[_size]` | empty array | [x] |
| 21 | array append/get | one, eight (initial capacity), nine (growth), many randomized values | [x] |
| 22 | array set | first, middle, last | [x] |
| 23 | array insert | beginning, middle, and exactly at end | [x] |
| 24 | array remove | first, middle, last from one/many | [x] |
| 25 | array clear | empty, one, many | [x] |
| 26 | array extend | empty+empty, empty+many, many+empty, many+many, self-extend | [x] |
| 27 | `json_equal` | same pointer, equal copies, unequal types, nested equal/unequal | [x] |
| 28 | `json_copy` | each of object, array, string, integer, real, true, false, null | [x] |
| 29 | `json_deep_copy`, `do_deep_copy` | nested object/array tree with shared scalar leaves | [x] |
| 30 | `json_delete` | heap values of every allocated JSON type | [x] |
| 31 | `utf8_encode` | boundaries `0`, `0x7f/80`, `0x7ff/800`, `0xffff/10000`, `0x10ffff` | [x] |
| 32 | `utf8_check_first` | ASCII; continuation; each valid 2/3/4-byte lead; invalid lead | [x] |
| 33 | `utf8_check_full` | valid 2-, 3-, 4-byte sequences with/without codepoint output | [x] |
| 34 | `utf8_iterate` | zero buffer, ASCII, and valid 2/3/4-byte sequences | [x] |
| 35 | `utf8_check_string` | empty, ASCII, and mixed valid Unicode | [x] |
| 36 | `strbuffer_*` | init/value, append byte, append zero bytes, append short bytes | [x] |
| 37 | `strbuffer_*` | append across growth boundary, pop nonempty/empty, clear, steal, close | [x] |
| 38 | `hashtable_*` | init/close empty table | [x] |
| 39 | `hashtable_*` | set/get one key, replace existing, delete | [x] |
| 40 | `hashtable_*` | zero-length and embedded-NUL keys through explicit lengths | [x] |
| 41 | `hashtable_*` | enough keys to trigger rehash | [x] |
| 42 | `hashtable_*` | iteration one/many; iter-at; iterator value replacement; clear | [x] |
| 43 | `jsonp_malloc/free/realloc/strndup` | positive sizes; grow, shrink, free-to-zero; embedded-NUL bytes | [x] |
| 44 | allocator get/set APIs | legacy malloc/free pair and malloc/realloc/free triple | [x] |
| 45 | `jsonp_error_*` | short source, source ≥80 bytes, first-error-wins behavior | [x] |
| 46 | `jsonp_loop_check` | first visit then duplicate visit using same parents table | [x] |
| 47 | `dtoa`, `dtoa_r`, `freedtoa` | modes 0–9; digits negative/zero/positive; ±0 and finite values | [x] |
| 48 | `dtoa`, `dtoa_r` | subnormal, very large/small, infinity, NaN, sign/decimal-point/end outputs | [x] |
| 49 | `strtod__unused`, `gethex` | decimal and hexadecimal floating spellings accepted by dtoa backend | [x] |
| 50 | `jsonp_strtod` | integer-like, fraction, exponent, negative and locale-independent dot | [x] |
| 51 | `jsonp_dtostr` | precision 0 and 1..31; fixed branch (`decpt -4..16`) | [x] |
| 52 | `jsonp_dtostr` | exponential branch (`decpt <= -4` or `>16`), signs and trailing `.0` | [x] |
| 53 | `json_loads` | empty object/array; object/array with one/many nested values | [x] |
| 54 | `json_loadb` | explicit zero/one/many length; embedded NUL in source buffer | [x] |
| 55 | `json_loadf`, `json_loadfd`, `json_load_file` | same valid document via stream, descriptor, and path | [x] |
| 56 | `json_load_callback` | callback chunks of 1, boundary 1024, and multiple chunks | [x] |
| 57 | all loaders | flags 0; root object and root array | [x] |
| 58 | all loaders | `JSON_DECODE_ANY`; string, integer, real, true, false, null roots | [x] |
| 59 | all loaders | `JSON_DISABLE_EOF_CHECK`; valid first value followed by extra bytes | [x] |
| 60 | all loaders | `JSON_REJECT_DUPLICATES`; unique-key object | [x] |
| 61 | all loaders | `JSON_DECODE_INT_AS_REAL`; negative/zero/positive integer tokens | [x] |
| 62 | all loaders | `JSON_ALLOW_NUL`; decoded NUL in a string value | [x] |
| 63 | all loaders | combinations of decode flags that branch independently | [x] |
| 64 | parser | number shapes: zero, negative, integer extremes, fraction, exponent ± | [x] |
| 65 | parser | string shapes: escapes, control escapes, BMP and surrogate-pair Unicode | [x] |
| 66 | parser | object/array shapes: empty, one, many, deep nesting below max | [x] |
| 67 | `json_dumps` | object/array roots, default flags, empty/one/many | [x] |
| 68 | all dump entry points | `JSON_ENCODE_ANY`; each scalar root type | [x] |
| 69 | all dump entry points | indent values 0, 1, 2, 31 on empty and nested containers | [x] |
| 70 | all dump entry points | `JSON_COMPACT` with indent zero/nonzero | [x] |
| 71 | all dump entry points | `JSON_ENSURE_ASCII`; ASCII, BMP, and non-BMP strings | [x] |
| 72 | all dump entry points | `JSON_ESCAPE_SLASH`; slash mixed with quote/backslash/control escapes | [x] |
| 73 | all dump entry points | `JSON_SORT_KEYS`; empty, one, and differently ordered many-key objects | [x] |
| 74 | all dump entry points | `JSON_PRESERVE_ORDER` alone and combined with sorting | [x] |
| 75 | all dump entry points | real precision field 0 and 1..31; fixed/exponent values | [x] |
| 76 | all dump entry points | `JSON_EMBED`; empty/nonempty object and array | [x] |
| 77 | all dump entry points | meaningful cross-combinations: compact+ASCII+slash+sort+precision+embed | [x] |
| 78 | `json_dumpb` | buffer sizes 0, one less than result, exact, and larger | [x] |
| 79 | `json_dump_callback` | callback receives multiple chunks and succeeds | [x] |
| 80 | `json_dumpf`, `json_dumpfd`, `json_dump_file` | valid writable destinations | [x] |
| 81 | `json_pack`, `json_pack_ex` | scalar formats `n,b,i,I,f,s` | [x] |
| 82 | `json_pack*` | string length forms `s#`, `s%`, concatenation `s+`, empty/embedded NUL | [x] |
| 83 | `json_pack*` | optional `s?`, `s*`, `O?`, `O*`, `o?`, `o*` with null/non-null | [x] |
| 84 | `json_pack*` | empty/one/many arrays and objects; nested formats | [x] |
| 85 | `json_pack*` | `O` incref versus `o` ownership behavior | [x] |
| 86 | `json_unpack`, `json_unpack_ex` | scalar formats `s,s%,i,I,b,f,F,o,O,n` | [x] |
| 87 | `json_unpack*` | nested object and array, empty/one/many | [x] |
| 88 | `json_unpack*` | optional object key `s?` present and absent | [x] |
| 89 | `json_unpack*` | local strict `!`, permissive `*`, and global `JSON_STRICT` | [x] |
| 90 | `json_unpack*` | `JSON_VALIDATE_ONLY` for every scalar/container format | [x] |
| 91 | `json_sprintf`, `json_vsprintf` | empty output, ASCII output, valid Unicode output, numeric formatting | [x] |
| 92 | `json_object_seed`, `hashtable_seed` | automatic seed, explicit seed, repeated calls after initialization | [x] |
| 93 | `json_dump_callback` + `json_load_callback` | randomized end-to-end round trips with randomized chunk boundaries | [x] |
| 94 | low-level value pipeline | randomized tree construction via constructors/setters then dump | [x] |
| 95 | load/dump pipeline | randomized valid JSON text load then every meaningful dump flag group | [x] |
| 96 | copy/equality pipeline | randomized tree, shallow copy/deep copy, equality before/after mutation | [x] |
| 97 | object pipeline | randomized set/replace/delete/update/iterate sequences | [x] |
| 98 | array pipeline | randomized append/insert/set/remove/extend sequences | [x] |
| 99 | error object output | loaders and pack/unpack with null and non-null `json_error_t` | [x] |
| 100 | all exported symbols | resolve each C and Rust symbol through `libloading` | [x] |
