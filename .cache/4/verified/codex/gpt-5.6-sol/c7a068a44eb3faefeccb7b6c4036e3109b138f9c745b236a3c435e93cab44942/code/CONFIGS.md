# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options. There is exactly one valid combination:

| # | Cargo feature set | CMake configuration | [ ] |
|---:|---|---|:---:|
| 1 | empty (`--no-default-features`) | default, `HAVE_CONFIG_H` | [x] |

## Runtime and Input Configurations

Rows are derived from public headers and branches in `value.c`, `load.c`,
`dump.c`, `pack_unpack.c`, `memory.c`, `hashtable.c`, `strbuffer.c`, `utf.c`,
`strconv.c`, `dtoa.c`, `error.c`, and `version.c`. A row may name a family when
the listed entry points share the same branch axis.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|---|---|:---:|
| 1 | `jansson_version_str`, `jansson_version_cmp` | exact version and comparisons below/equal/above each component | [x] |
| 2 | `json_true`, `json_false`, `json_null`, `json_delete` | singleton values and heap values of every JSON type | [x] |
| 3 | `json_integer`, `json_integer_value`, `json_integer_set`, `json_number_value` | zero, signed extrema, random `int64`, integer-as-number | [x] |
| 4 | `json_real`, `json_real_value`, `json_real_set`, `json_number_value` | `+0`, `-0`, normal, subnormal, min/max finite, random finite | [x] |
| 5 | string constructors/getters | checked vs nocheck; C string vs explicit length; empty, ASCII, multibyte, embedded NUL | [x] |
| 6 | string setters | checked vs nocheck; C string vs explicit length; empty, ASCII, multibyte, embedded NUL | [x] |
| 7 | `json_object`, size/get/getn/set variants | empty/one/many; replacement; C-string and length keys; embedded-NUL key with nocheck | [x] |
| 8 | object delete/clear | first/middle/last/missing key, length key, empty and many objects | [x] |
| 9 | object update family | update all/existing/missing/recursive; disjoint, overlapping, nested keys | [x] |
| 10 | object iterator family | empty/one/many, iter-at, key-to-iter, next, key length/value, replace through iterator | [x] |
| 11 | `json_object_seed` | zero autoseed and explicit nonzero seeds before object creation | [x] |
| 12 | `json_array`, size/get | empty, one, many; indices zero/middle/last | [x] |
| 13 | array set/append/insert/remove/clear | front/middle/end, capacity growth across 8 entries | [x] |
| 14 | `json_array_extend` | empty/nonempty source and destination, self-extension | [x] |
| 15 | `json_equal` | every type; equal/unequal; nested objects with differing insertion order | [x] |
| 16 | `json_copy`, `json_deep_copy`, `do_deep_copy` | every scalar; shallow/deep object and array; nested structures | [x] |
| 17 | load entry points | `json_loads`, `json_loadb`, `json_loadf`, `json_loadfd`, `json_load_file`, `json_load_callback` with object/array | [x] |
| 18 | load input shape | empty object/array, nested, empty/one/many members, whitespace, all scalar token types | [x] |
| 19 | load string shape | escapes, control escapes, BMP/non-BMP Unicode escapes, raw multibyte UTF-8 | [x] |
| 20 | load number shape | integer extrema, `-0`, decimal, exponent signs/cases, finite real boundaries | [x] |
| 21 | load flags | no flags vs `JSON_DECODE_ANY` for scalar top-level values | [x] |
| 22 | load flags | `JSON_REJECT_DUPLICATES` off/on with duplicate and unique keys | [x] |
| 23 | load flags | `JSON_DISABLE_EOF_CHECK` off/on with trailing JSON tokens/text | [x] |
| 24 | load flags | `JSON_DECODE_INT_AS_REAL` off/on for integer tokens | [x] |
| 25 | load flags | `JSON_ALLOW_NUL` off/on for `\u0000` in values; NUL keys remain rejected | [x] |
| 26 | load flags | meaningful combinations/cross-product of the five decode flags | [x] |
| 27 | dump entry points | `json_dumps`, `json_dumpb`, `json_dump_callback`, `json_dumpf`, `json_dumpfd`, `json_dump_file` | [x] |
| 28 | dump types | object/array and every scalar with `JSON_ENCODE_ANY` off/on | [x] |
| 29 | dump indentation | widths 0, 1, 2, 31 on empty/one/many/nested containers | [x] |
| 30 | dump formatting | compact off/on crossed with indentation and nested containers | [x] |
| 31 | dump strings | ensure-ASCII off/on for BMP/non-BMP; escape-slash off/on; controls/quotes/backslashes | [x] |
| 32 | dump objects | sort-keys off/on for ASCII, prefix, case, multibyte, and embedded-NUL keys | [x] |
| 33 | dump order | preserve-order bit off/on (C treats it as insertion order in either state) | [x] |
| 34 | dump embedding | `JSON_EMBED` off/on for empty/nonempty object and array | [x] |
| 35 | dump real precision | precision 0 and 1..31; fixed/exponent thresholds; extrema/subnormal/random finite values | [x] |
| 36 | `json_dumpb` | buffer null/zero, undersized, exact, oversized; returned required byte count | [x] |
| 37 | pack entry points | `json_pack`, `json_pack_ex`, `json_vpack_ex` with scalar formats `s,i,I,f,b,n,o,O` | [x] |
| 38 | pack containers | empty/one/many/nested `{}` and `[]`; ignored spaces, tabs, newlines, commas, colons | [x] |
| 39 | pack strings | plain, `#`, `%`, `+`, chained concatenation, `?` and `*` optional values | [x] |
| 40 | pack ownership | `o`/`O` and optional variants with null/non-null values | [x] |
| 41 | unpack entry points | `json_unpack`, `json_unpack_ex`, `json_vunpack_ex` for `s,s%,i,I,b,f,F,o,O,n` | [x] |
| 42 | unpack containers | empty/one/many/nested object/array; optional object keys | [x] |
| 43 | unpack strictness | default, format `!`, format `*`, and `JSON_STRICT`, with exact/extra members | [x] |
| 44 | unpack validation | `JSON_VALIDATE_ONLY` off/on across scalar and nested formats | [x] |
| 45 | `json_sprintf`, `json_vsprintf` | empty/nonempty output; strings, signed integers, reals, width/precision, UTF-8 | [x] |
| 46 | allocator API | set/get legacy malloc/free and malloc/realloc/free; direct allocate/reallocate/free | [x] |
| 47 | allocator fallback | null realloc hook; shrink, grow, and realloc-to-zero with null/non-null pointers | [x] |
| 48 | `jsonp_strndup`, owned string constructor | empty/nonempty, explicit embedded NUL, ownership transfer | [x] |
| 49 | error helpers | null/non-null error, short/long/null source, formatted message and every error-code byte | [x] |
| 50 | strbuffer API | init/clear/value/steal/close; append byte/bytes; no growth and repeated growth; pop empty/nonempty | [x] |
| 51 | UTF-8 first/full/check | all one-byte values; valid 2/3/4-byte boundaries; invalid continuation/overlong/surrogate/out-of-range | [x] |
| 52 | `utf8_iterate` | zero buffer, ASCII, 2/3/4-byte values, exact/truncated/oversized buffers; codepoint null/non-null | [x] |
| 53 | `utf8_encode` | boundary codepoints for 1/2/3/4-byte encodings, surrogate values, and Unicode maximum | [x] |
| 54 | hashtable API | init/set/get/replace/delete/clear/close; empty/one/many, growth, binary keys, seed values | [x] |
| 55 | hashtable iterator API | empty/one/many; iter-at/next/key/key-len/value/set; insertion order | [x] |
| 56 | `jsonp_loop_check` | first insertion vs duplicate pointer key; key length output null/non-null | [x] |
| 57 | `jsonp_strtod`, `strtod__unused` | signed integer/decimal/exponent, underflow/subnormal, finite extrema, end pointer | [x] |
| 58 | `jsonp_dtostr` | precision 0 and 1..31; positive/negative, fixed/exponent threshold, finite extrema | [x] |
| 59 | `dtoa`, `dtoa_r`, `freedtoa` | modes 0..9, digit counts negative/zero/positive, signs, normal/subnormal/extrema | [x] |
| 60 | `gethex`, `dtoa_divmax`, `hashtable_seed` | exported low-level state/symbol behavior exercised through parser/formatter/hash operations | [x] |
