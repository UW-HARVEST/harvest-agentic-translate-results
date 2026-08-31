# Configuration Surface

Derived from the public headers, all 130 dynamic exports, and branches on flags,
types, lengths, counts, numeric ranges, and callback/file modes. Randomized rows
use a fixed seed. Multiple entry points are grouped only when they execute the
same C branch family.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|-----|
| 1 | `jansson_version_str`, `jansson_version_cmp` | exact version and comparisons below/equal/above each component | [x] |
| 2 | `utf8_check_first` | ASCII and every lead-byte class (continuation, overlong, 2/3/4-byte, restricted) | [ ] |
| 3 | `utf8_check_full`, `utf8_iterate`, `utf8_check_string` | valid 2-, 3-, and 4-byte sequences at Unicode boundaries | [ ] |
| 4 | `utf8_encode` | code points at `0`, `0x7f/80`, `0x7ff/800`, `0xffff/10000`, `0x10ffff` | [x] |
| 5 | `strbuffer_init`, `strbuffer_value`, `strbuffer_append_byte`, `strbuffer_append_bytes`, `strbuffer_pop`, `strbuffer_clear`, `strbuffer_steal_value`, `strbuffer_close` | empty, one byte, exact initial capacity, growth, and many bytes | [ ] |
| 6 | `jsonp_dtostr`, `dtoa`, `dtoa_r`, `freedtoa`, `dtoa_divmax` | finite random doubles; dtoa modes 0 through 9; precisions 0, 1, 17, 31 | [ ] |
| 7 | `strtod__unused`, `gethex`, `jsonp_strtod` | integer, fraction, exponent, negative zero, smallest/largest finite forms | [ ] |
| 8 | `jsonp_malloc`, `jsonp_realloc`, `jsonp_free`, `jsonp_strndup` | allocation, growth/shrink, duplicate lengths 0/1/many | [ ] |
| 9 | `json_set_alloc_funcs`, `json_get_alloc_funcs` | custom malloc/free pair and restoration | [ ] |
| 10 | `json_set_alloc_funcs2`, `json_get_alloc_funcs2` | custom malloc/realloc/free trio and restoration | [x] |
| 11 | `hashtable_init`, `hashtable_set`, `hashtable_get`, `hashtable_del`, `hashtable_clear`, `hashtable_close`, `hashtable_seed` | empty; replace; delete; 1/8/9/many entries to cross rehash; binary keys by explicit length | [ ] |
| 12 | `hashtable_iter`, `hashtable_iter_at`, `hashtable_iter_next`, `hashtable_iter_key`, `hashtable_iter_key_len`, `hashtable_iter_value`, `hashtable_iter_set` | insertion-order traversal, lookup iterator, replacement through iterator | [ ] |
| 13 | `json_object_seed` | explicit zero (autoseed path) and fixed nonzero seed before object creation | [ ] |
| 14 | `json_object`, `json_object_size`, `json_object_get`, `json_object_getn` | empty/one/many keys; C-string and explicit-length keys including embedded NUL | [ ] |
| 15 | `json_object_set_new`, `json_object_setn_new`, `json_object_set_new_nocheck`, `json_object_setn_new_nocheck` | valid UTF-8 checked keys; arbitrary-byte nocheck keys; insert and replace | [ ] |
| 16 | `json_object_del`, `json_object_deln`, `json_object_clear` | delete first/middle/last and clear empty/nonempty object | [ ] |
| 17 | `json_object_update`, `json_object_update_existing`, `json_object_update_missing` | disjoint and overlapping empty/one/many objects | [ ] |
| 18 | `json_object_update_recursive`, `do_object_update_recursive`, `jsonp_loop_check` | nested object/object merge and scalar replacement at multiple depths | [ ] |
| 19 | `json_object_iter`, `json_object_iter_at`, `json_object_iter_next`, `json_object_iter_key`, `json_object_iter_key_len`, `json_object_iter_value`, `json_object_key_to_iter`, `json_object_iter_set_new` | empty and insertion-order one/many traversal, binary key lengths, iterator replacement | [ ] |
| 20 | `json_array`, `json_array_size`, `json_array_get` | empty, one, eight, nine, and many elements; first/middle/last index | [ ] |
| 21 | `json_array_append_new`, `json_array_insert_new` | append and insert at front/middle/end across initial-capacity growth | [ ] |
| 22 | `json_array_set_new`, `json_array_remove`, `json_array_clear` | set/remove first/middle/last; clear empty/nonempty | [ ] |
| 23 | `json_array_extend` | empty/nonempty source and destination, including self-extension | [ ] |
| 24 | `json_string`, `json_stringn`, `json_string_nocheck`, `json_stringn_nocheck`, `jsonp_stringn_nocheck_own` | empty, ASCII, multibyte UTF-8, embedded NUL by length, arbitrary bytes on nocheck path | [ ] |
| 25 | `json_string_value`, `json_string_length`, `json_string_set`, `json_string_setn`, `json_string_set_nocheck`, `json_string_setn_nocheck` | getters and replacement across empty/short/long, UTF-8, embedded NUL, arbitrary nocheck bytes | [ ] |
| 26 | `json_sprintf`, `json_vsprintf` | empty output and formatted integer/string/real output | [ ] |
| 27 | `json_integer`, `json_integer_value`, `json_integer_set` | `i64::MIN`, `-1`, `0`, `1`, `i64::MAX`, and random values | [x] |
| 28 | `json_real`, `json_real_value`, `json_real_set` | negative/positive zero, subnormal, normal, min/max finite, and random finite values | [ ] |
| 29 | `json_number_value` | integer, real, and each nonnumber JSON type | [ ] |
| 30 | `json_true`, `json_false`, `json_null` | singleton type/refcount layout and repeated calls | [ ] |
| 31 | `json_equal` | every JSON type: same pointer, equal distinct value, unequal value/type; nested arrays/objects | [ ] |
| 32 | `json_copy`, `json_deep_copy`, `do_deep_copy` | every scalar; empty/nonempty nested array/object; verify serialized bytes and independence | [ ] |
| 33 | `json_delete` | owned object/array/string/integer/real and immortal true/false/null | [ ] |
| 34 | `json_dumps`, `json_dumpb`, `json_dump_callback` | object/array; empty/one/many; output buffer size 0/exact/short/long | [ ] |
| 35 | `json_dumps`, `json_dumpb`, `json_dump_callback` | `JSON_ENCODE_ANY` for string/integer/real/true/false/null roots | [ ] |
| 36 | dump entry points | indent values 0, 1, 4, 31 crossed with `JSON_COMPACT` off/on | [ ] |
| 37 | dump entry points | `JSON_ENSURE_ASCII` off/on for BMP and non-BMP Unicode strings | [x] |
| 38 | dump entry points | `JSON_ESCAPE_SLASH` off/on for slash-containing keys and values | [x] |
| 39 | dump entry points | `JSON_SORT_KEYS` off/on crossed with insertion orders and binary key lengths | [ ] |
| 40 | dump entry points | `JSON_PRESERVE_ORDER` off/on (flag is accepted but has no separate C branch) | [ ] |
| 41 | dump entry points | `JSON_REAL_PRECISION(n)` for 0, 1, 6, 17, 31 and random finite reals | [x] |
| 42 | dump entry points | `JSON_EMBED` off/on for arrays and objects, empty/one/many | [ ] |
| 43 | `json_dump_callback` | callback accepts all chunks and callback rejects first/later chunk | [ ] |
| 44 | `json_dumpf`, `json_dumpfd`, `json_dump_file` | writable stream/fd/path for empty and nested documents | [ ] |
| 45 | `json_loads`, `json_loadb` | object/array with empty/one/many/nested values; whitespace; explicit buffer lengths | [ ] |
| 46 | `json_loadf`, `json_loadfd`, `json_load_file`, `json_load_callback` | same valid document through stream, descriptor, path, and chunked callback (1/many-byte chunks) | [ ] |
| 47 | load entry points | `JSON_DECODE_ANY` off/on crossed with all scalar JSON roots | [ ] |
| 48 | load entry points | `JSON_REJECT_DUPLICATES` off/on crossed with unique/duplicate object keys | [ ] |
| 49 | load entry points | `JSON_DISABLE_EOF_CHECK` off/on crossed with trailing whitespace/token/data | [ ] |
| 50 | load entry points | `JSON_DECODE_INT_AS_REAL` off/on for boundary/random integers and exponent/fraction forms | [ ] |
| 51 | load entry points | `JSON_ALLOW_NUL` off/on for escaped NUL in string values; NUL in object key remains rejected | [x] |
| 52 | `json_pack`, `json_pack_ex`, `json_vpack_ex` | scalar formats `n,b,i,I,f,s,s#,s%,O,o`; optional `?/*`; owned `+` strings | [ ] |
| 53 | pack entry points | object/array formats, empty/one/many/nested, separators/whitespace, binary-length strings | [ ] |
| 54 | `json_unpack`, `json_unpack_ex`, `json_vunpack_ex` | scalar formats `s,s%,i,I,b,f,F,O,o,n` with matching values | [ ] |
| 55 | unpack entry points | object/array formats, empty/one/many/nested, optional `?`, strict `!`, ignore `*` | [ ] |
| 56 | unpack entry points | `JSON_VALIDATE_ONLY` off/on crossed with scalar and nested formats | [ ] |
| 57 | unpack entry points | `JSON_STRICT` off/on crossed with exact and extra object keys/array items | [ ] |
| 58 | `jsonp_error_init`, `jsonp_error_set_source` | null/non-null error; source empty/short/exactly 79/80/long bytes | [ ] |
| 59 | `jsonp_error_set`, `jsonp_error_vset` | every error enum and out-of-range integer; first-error-wins behavior | [ ] |
| 60 | `do_deep_copy`, `do_object_update_recursive`, `gethex`, low-level `dtoa*` | directly loaded low-level symbols with valid state/arguments, not convenience wrappers | [ ] |
