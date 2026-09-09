# Configuration surface

Derived from the public declarations in `include/jansson.h`, the additional
dynamic symbols reported by `nm -D`, and the option/shape branches in
`dump.c`, `load.c`, `pack_unpack.c`, `value.c`, `utf.c`, `strconv.c`,
`strbuffer.c`, `hashtable.c`, and `dtoa.c`.

Coverage evidence: the fixed-seed differential suites in
`tests/differential.rs` cover these rows by API family, including randomized
load/dump, constructor/container, DTOA, UTF-8, hashtable, callback, file/fd,
variadic pack/unpack, update/copy, option, boundary, and invalid-input
matrices. Every call into both implementations is resolved from its `.so`
through `libloading`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `jansson_version_str`, `jansson_version_cmp` | exact version and comparisons below/equal/above each component | [x] |
| 2 | `dtoa_divmax`, `hashtable_seed`, `json_object_seed` | exported globals before/after explicit zero and nonzero seeds | [x] |
| 3 | `json_get_alloc_funcs`, `json_get_alloc_funcs2` | default allocator callbacks, with every output pointer present | [x] |
| 4 | `json_get_alloc_funcs`, `json_get_alloc_funcs2` | each optional output pointer independently null | [x] |
| 5 | `json_set_alloc_funcs`, `json_set_alloc_funcs2` | install and retrieve malloc/free versus malloc/realloc/free callback sets | [x] |
| 6 | `jsonp_malloc`, `jsonp_realloc`, `jsonp_free`, `jsonp_strndup` | zero, one, small, and growing allocations; byte preservation | [x] |
| 7 | `jsonp_error_init`, `jsonp_error_set_source` | null, short, and overlong source strings | [x] |
| 8 | `jsonp_error_set`, `jsonp_error_vset` | error positions/codes with short and truncating messages | [x] |
| 9 | `utf8_check_first` | ASCII, continuation, overlong leaders, valid 2/3/4-byte leaders, and `0xF5..0xFF` | [x] |
| 10 | `utf8_check_full` | valid 2/3/4-byte sequences with codepoint output present and null | [x] |
| 11 | `utf8_check_full` | bad size, bad continuation, overlong, surrogate, and above-`0x10FFFF` sequences | [x] |
| 12 | `utf8_iterate` | empty buffer, ASCII, valid multibyte, truncated, and malformed sequences | [x] |
| 13 | `utf8_check_string` | empty, ASCII, mixed valid Unicode, embedded NUL, truncated, and malformed byte strings | [x] |
| 14 | `utf8_encode` | boundaries `0`, `0x7F/80`, `0x7FF/800`, `0xFFFF/10000`, `0x10FFFF` | [x] |
| 15 | `utf8_encode` | negative and `0x110000` out-of-range codepoints | [x] |
| 16 | `strbuffer_init`, `strbuffer_value`, `strbuffer_clear`, `strbuffer_close` | empty initialized buffer and clear after content | [x] |
| 17 | `strbuffer_append_byte`, `strbuffer_append_bytes` | zero bytes, one byte, embedded NUL, and growth beyond initial capacity | [x] |
| 18 | `strbuffer_pop`, `strbuffer_steal_value` | pop empty/nonempty and steal empty/nonempty allocation | [x] |
| 19 | `hashtable_init`, `hashtable_close`, `hashtable_clear` | empty table and table after one/many inserts | [x] |
| 20 | `hashtable_set`, `hashtable_get`, `hashtable_del` | empty, ASCII, embedded-NUL, and long keys; insert, replace, hit, miss, delete | [x] |
| 21 | `hashtable_iter`, `hashtable_iter_at`, `hashtable_iter_next` | empty, one, and many entries | [x] |
| 22 | `hashtable_iter_key`, `hashtable_iter_key_len`, `hashtable_iter_value`, `hashtable_iter_set` | inspect and replace values through an iterator | [x] |
| 23 | `dtoa`, `freedtoa` | finite random doubles in modes `0..9`, digit counts negative/zero/positive | [x] |
| 24 | `dtoa_r` | finite, signed zero, subnormal, min/max finite, infinity, and NaN with adequate buffer | [x] |
| 25 | `dtoa_r` | short versus adequate caller buffers | [x] |
| 26 | `strtod__unused`, `gethex`, `jsonp_strtod` | integers, decimal fractions, hexadecimal floats, exponents, signed zero, min/max magnitudes, and overflow | [x] |
| 27 | `jsonp_dtostr` | precision `0`, `1`, `17`, and `31`; fixed/exponent thresholds and signed zero | [x] |
| 28 | `jsonp_dtostr` | output buffer sizes zero, one-short, exact, and oversized | [x] |
| 29 | `json_true`, `json_false`, `json_null` | singleton identity, type, and immortal refcount | [x] |
| 30 | `json_integer`, `json_integer_value`, `json_integer_set` | zero, signed boundaries, and randomized `int64` values | [x] |
| 31 | `json_real`, `json_real_value`, `json_real_set`, `json_number_value` | finite random values, signed zero, subnormal, and integer-to-double conversion | [x] |
| 32 | `json_real`, `json_real_set` | NaN and positive/negative infinity | [x] |
| 33 | `json_string`, `json_string_value`, `json_string_length` | empty, ASCII, embedded escapes, and valid 2/3/4-byte UTF-8 | [x] |
| 34 | `json_stringn`, `json_stringn_nocheck`, `jsonp_stringn_nocheck_own` | explicit zero/small lengths, embedded NUL, valid and invalid UTF-8 | [x] |
| 35 | `json_string_nocheck`, `json_string_set_nocheck`, `json_string_setn_nocheck` | malformed UTF-8 and explicit-length bytes | [x] |
| 36 | `json_string_set`, `json_string_setn` | empty, valid Unicode, embedded NUL, malformed, and wrong target type | [x] |
| 37 | `json_object`, `json_object_size`, `json_object_get`, `json_object_set_new` | empty/one/many ASCII NUL-terminated keys; insert and replacement | [x] |
| 38 | `json_object_getn`, `json_object_setn_new` | explicit key lengths including empty and embedded-NUL keys | [x] |
| 39 | `json_object_set_new_nocheck`, `json_object_setn_new_nocheck` | malformed UTF-8 key accepted by nocheck variants | [x] |
| 40 | `json_object_del`, `json_object_deln`, `json_object_clear` | existing/missing keys, explicit lengths, empty and many-entry objects | [x] |
| 41 | `json_object_iter`, `json_object_iter_at`, `json_object_iter_next` | empty, one, and many entries | [x] |
| 42 | `json_object_iter_key`, `json_object_iter_key_len`, `json_object_iter_value`, `json_object_key_to_iter` | key/value recovery for ASCII and embedded-NUL keys | [x] |
| 43 | `json_object_iter_set_new` | replace iterator value in one/many-entry objects | [x] |
| 44 | `json_object_update` | disjoint and overlapping empty/one/many objects | [x] |
| 45 | `json_object_update_existing` | no overlap, partial overlap, and full overlap | [x] |
| 46 | `json_object_update_missing` | no overlap, partial overlap, and full overlap | [x] |
| 47 | `json_object_update_recursive`, `do_object_update_recursive`, `jsonp_loop_check` | nested disjoint/overlapping objects and circular-reference detection | [x] |
| 48 | `json_array`, `json_array_size`, `json_array_get`, `json_array_append_new` | empty, one, many, and growth beyond initial capacity | [x] |
| 49 | `json_array_set_new` | first/middle/last valid indexes and index equal to/past length | [x] |
| 50 | `json_array_insert_new` | beginning/middle/end and index past end | [x] |
| 51 | `json_array_remove`, `json_array_clear` | first/middle/last, empty, and out-of-range indexes | [x] |
| 52 | `json_array_extend` | empty/nonempty source and destination, including self-extension | [x] |
| 53 | `json_equal` | each JSON type, equal/unequal values, nested containers, null operands, and cross-types | [x] |
| 54 | `json_copy` | each scalar/simple/container type; verify shallow container value identity | [x] |
| 55 | `json_deep_copy`, `do_deep_copy` | nested arrays/objects and cyclic inputs | [x] |
| 56 | `json_delete` | null, heap-backed types, and immortal singleton types | [x] |
| 57 | `json_pack`, `json_pack_ex`, `json_vpack_ex` | null/boolean/integer/real/string, arrays, objects, optional values, and strict ownership formats | [x] |
| 58 | `json_sprintf`, `json_vsprintf` | empty, numeric substitution, valid Unicode, and formatting that produces invalid UTF-8 | [x] |
| 59 | `json_unpack`, `json_unpack_ex`, `json_vunpack_ex` | scalar, array, and object formats with `JSON_VALIDATE_ONLY` off/on | [x] |
| 60 | `json_unpack_ex`, `json_vunpack_ex` | `JSON_STRICT` off/on with exact and extra array/object members | [x] |
| 61 | `json_loads`, `json_loadb` | object/array/string/number/true/false/null and empty input | [x] |
| 62 | `json_loadb` | explicit lengths containing embedded NUL with `JSON_ALLOW_NUL` off/on | [x] |
| 63 | `json_loads`, `json_loadb` | duplicate object keys with `JSON_REJECT_DUPLICATES` off/on | [x] |
| 64 | `json_loads`, `json_loadb` | trailing data with `JSON_DISABLE_EOF_CHECK` off/on | [x] |
| 65 | `json_loads`, `json_loadb` | scalar roots with `JSON_DECODE_ANY` off/on | [x] |
| 66 | `json_loads`, `json_loadb` | integer tokens with `JSON_DECODE_INT_AS_REAL` off/on, including overflow boundaries | [x] |
| 67 | `json_load_callback` | chunks of size zero/one/many, normal EOF, callback error, and read error | [x] |
| 68 | `json_load_file`, `json_loadf`, `json_loadfd` | valid file, empty file, malformed file, nonexistent path, valid/invalid descriptor | [x] |
| 69 | `json_dumps`, `json_dumpb` | object/array roots with default, compact, and indent values `1` and `31` | [x] |
| 70 | `json_dumps`, `json_dumpb` | Unicode strings with `JSON_ENSURE_ASCII` and slash strings with `JSON_ESCAPE_SLASH` off/on | [x] |
| 71 | `json_dumps`, `json_dumpb` | insertion order, `JSON_SORT_KEYS`, and `JSON_PRESERVE_ORDER` combinations | [x] |
| 72 | `json_dumps`, `json_dumpb` | scalar roots with `JSON_ENCODE_ANY` off/on | [x] |
| 73 | `json_dumps`, `json_dumpb` | real precision fields `0`, `1`, `17`, and `31`; fixed/exponent values | [x] |
| 74 | `json_dumps`, `json_dumpb` | `JSON_EMBED` off/on for empty and nonempty arrays/objects | [x] |
| 75 | `json_dumpb` | null buffer with size zero, undersized, exact, and oversized buffers | [x] |
| 76 | `json_dump_callback` | successful collector and callback failure before/midway through output | [x] |
| 77 | `json_dump_file`, `json_dumpf`, `json_dumpfd` | valid destination, unwritable path, closed/invalid descriptor, and write failure | [x] |
