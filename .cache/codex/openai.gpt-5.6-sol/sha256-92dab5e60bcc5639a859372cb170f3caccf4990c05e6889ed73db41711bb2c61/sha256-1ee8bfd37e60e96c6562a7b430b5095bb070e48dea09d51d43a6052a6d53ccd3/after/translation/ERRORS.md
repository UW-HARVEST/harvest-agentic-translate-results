# Error surface

Mechanically derived from `return -1`, `return NULL`, error-code setters,
assertions, null/type/range checks, and overflow constants in `c_src/src`.
Allocator-failure rows refer to the corresponding explicit allocation check.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `jsonp_malloc` | `size == 0` | `[x] NULL` |
| 2 | `jsonp_realloc` | emulated realloc and `newSize == 0` | `[x] free old pointer; NULL` |
| 3 | `jsonp_strndup` | allocation of `len + 1` fails | `[x] NULL` |
| 4 | `strbuffer_init` | initial allocation fails | `[x] -1` |
| 5 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX/STRBUFFER_FACTOR` | `[x] -1` |
| 6 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | `[x] -1` |
| 7 | `strbuffer_append_bytes` | `strbuff->length > SIZE_MAX - 1 - size` | `[x] -1` |
| 8 | `strbuffer_append_bytes` | growth realloc fails | `[x] -1` |
| 9 | `hashtable_del` | key is absent | `[x] -1` |
| 10 | `hashtable_init` | bucket allocation fails | `[x] -1` |
| 11 | `hashtable_set` | `key_len >= SIZE_MAX - offsetof(pair_t,key)` | `[x] -1` |
| 12 | `hashtable_set` | pair allocation fails | `[x] -1` |
| 13 | `hashtable_set` | rehash allocation fails | `[x] -1` |
| 14 | `hashtable_get` | key is absent | `[x] NULL` |
| 15 | `hashtable_iter_at` | key is absent | `[x] NULL` |
| 16 | `hashtable_iter_next` | iterator is the final ordered item | `[x] NULL` |
| 17 | `jsonp_loop_check` | pointer key already exists in `parents` | `[x] -1` |
| 18 | `utf8_encode` | code point `< 0` | `[x] -1` |
| 19 | `utf8_encode` | code point `> 0x10ffff` | `[x] -1` |
| 20 | `utf8_check_first` | continuation byte `0x80..0xbf` | `[x] 0` |
| 21 | `utf8_check_first` | overlong lead `0xc0` or `0xc1` | `[x] 0` |
| 22 | `utf8_check_first` | invalid/restricted lead `>= 0xf5` | `[x] 0` |
| 23 | `utf8_check_full` | size is not 2, 3, or 4 | `[x] 0` |
| 24 | `utf8_check_full` | a trailing byte is outside `0x80..0xbf` | `[x] 0` |
| 25 | `utf8_check_full` | decoded value exceeds `0x10ffff` | `[x] 0` |
| 26 | `utf8_check_full` | decoded value is `0xd800..0xdfff` | `[x] 0` |
| 27 | `utf8_check_full` | sequence is overlong for its byte count | `[x] 0` |
| 28 | `utf8_iterate` | first byte is not a valid leading byte | `[x] NULL` |
| 29 | `utf8_iterate` | required sequence length exceeds `bufsize` | `[x] NULL` |
| 30 | `utf8_iterate` | full multibyte sequence is invalid | `[x] NULL` |
| 31 | `utf8_check_string` | invalid leading byte occurs | `[x] 0` |
| 32 | `utf8_check_string` | multibyte sequence is truncated | `[x] 0` |
| 33 | `utf8_check_string` | multibyte sequence fails full validation | `[x] 0` |
| 34 | `jsonp_strtod` | `strtod` returns `+/-HUGE_VAL` with `ERANGE` | `[x] -1` |
| 35 | `jsonp_dtostr` | `dtoa_r` cannot fit its 25-byte digit buffer | `[x] -1` |
| 36 | `jsonp_dtostr` | rendered number plus terminator exceeds `size` | `[x] -1` |
| 37 | `json_object_get` | `key == NULL` | `[x] NULL` |
| 38 | `json_object_getn` | `key == NULL` or value is not an object | `[x] NULL` |
| 39 | `json_object_set_new_nocheck` | `key == NULL` | `[x] -1; value decref'd` |
| 40 | `json_object_setn_new_nocheck` | `value == NULL` | `[x] -1` |
| 41 | `json_object_setn_new_nocheck` | null key, non-object target, or `target == value` | `[x] -1; value decref'd` |
| 42 | `json_object_setn_new` | null key or invalid UTF-8 key | `[x] -1; value decref'd` |
| 43 | `json_object_del` | `key == NULL` | `[x] -1` |
| 44 | `json_object_deln` | null key or non-object target | `[x] -1` |
| 45 | `json_object_deln` | key is absent | `[x] -1` |
| 46 | `json_object_clear` | target is null or not an object | `[x] -1` |
| 47 | `json_object_update` | either argument is not an object | `[x] -1` |
| 48 | `json_object_update_existing` | either argument is not an object | `[x] -1` |
| 49 | `json_object_update_missing` | either argument is not an object | `[x] -1` |
| 50 | `do_object_update_recursive` | either argument is not an object | `[x] -1` |
| 51 | `do_object_update_recursive` | circular `other` graph is detected | `[x] -1` |
| 52 | `json_object_iter` | target is null or not an object | `[x] NULL` |
| 53 | `json_object_iter_at` | null key or non-object target | `[x] NULL` |
| 54 | `json_object_iter_next` | non-object target or null iterator | `[x] NULL` |
| 55 | `json_object_iter_key` | null iterator | `[x] NULL` |
| 56 | `json_object_iter_key_len` | null iterator | `[x] 0` |
| 57 | `json_object_iter_value` | null iterator | `[x] NULL` |
| 58 | `json_object_iter_set_new` | non-object, null iterator, or null value | `[x] -1` |
| 59 | `json_object_key_to_iter` | null key | `[x] NULL` |
| 60 | `json_array_get` | target is not an array | `[x] NULL` |
| 61 | `json_array_get` | `index >= entries` | `[x] NULL` |
| 62 | `json_array_set_new` | `value == NULL` | `[x] -1` |
| 63 | `json_array_set_new` | target is not array or `target == value` | `[x] -1; value decref'd` |
| 64 | `json_array_set_new` | `index >= entries` | `[x] -1; value decref'd` |
| 65 | `json_array_append_new` | `value == NULL` | `[x] -1` |
| 66 | `json_array_append_new` | target is not array or `target == value` | `[x] -1; value decref'd` |
| 67 | `json_array_insert_new` | `value == NULL` | `[x] -1` |
| 68 | `json_array_insert_new` | target is not array or `target == value` | `[x] -1; value decref'd` |
| 69 | `json_array_insert_new` | `index > entries` | `[x] -1; value decref'd` |
| 70 | `json_array_remove` | target is not an array | `[x] -1` |
| 71 | `json_array_remove` | `index >= entries` | `[x] -1` |
| 72 | `json_array_clear` | target is not an array | `[x] -1` |
| 73 | `json_array_extend` | either argument is not an array | `[x] -1` |
| 74 | `json_string`, `json_string_nocheck` | `value == NULL` | `[x] NULL` |
| 75 | `json_stringn` | null value or invalid UTF-8 in the requested length | `[x] NULL` |
| 76 | `json_string_value` | target is not a string | `[x] NULL` |
| 77 | `json_string_set`, `json_string_set_nocheck` | `value == NULL` | `[x] -1` |
| 78 | `json_string_setn_nocheck` | non-string target or null value | `[x] -1` |
| 79 | `json_string_setn` | null value or invalid UTF-8 | `[x] -1` |
| 80 | `json_integer_set` | target is not an integer | `[x] -1` |
| 81 | `json_real` | value is NaN or infinity | `[x] NULL` |
| 82 | `json_real_set` | non-real target, NaN, or infinity | `[x] -1` |
| 83 | `json_equal` | either argument is null | `[x] 0` |
| 84 | `json_copy`, `do_deep_copy` | input is null or has an out-of-range type tag | `[x] NULL` |
| 85 | `json_dump_callback` | scalar root without `JSON_ENCODE_ANY` | `[x] -1` |
| 86 | `json_dump_callback` | null JSON reaches `do_dump` | `[x] -1` |
| 87 | `json_dump_callback` | callback returns nonzero for any emitted chunk | `[x] -1` |
| 88 | `json_dump_callback` | circular array/object reference is detected | `[x] -1` |
| 89 | `json_dump_file` | output path cannot be opened | `[x] -1` |
| 90 | `json_dumpfd` | negative/invalid descriptor cannot accept full write | `[x] -1` |
| 91 | `json_loads` | input pointer is null | `[x] NULL; invalid_argument` |
| 92 | `json_loadb` | buffer pointer is null, including zero length | `[x] NULL; invalid_argument` |
| 93 | `json_loadf` | `FILE *` is null | `[x] NULL; invalid_argument` |
| 94 | `json_loadfd` | descriptor `< 0` | `[x] NULL; invalid_argument` |
| 95 | `json_load_file` | path is null | `[x] NULL; invalid_argument` |
| 96 | `json_load_file` | path cannot be opened | `[x] NULL; cannot_open_file` |
| 97 | `json_load_callback` | callback is null | `[x] NULL; invalid_argument` |
| 98 | all loaders | invalid UTF-8 byte/sequence | `[x] NULL; invalid_utf8` |
| 99 | all loaders | invalid string escape | `[x] NULL; invalid_syntax` |
| 100 | all loaders | unescaped control byte in string | `[x] NULL; invalid_syntax` |
| 101 | all loaders | lone high surrogate, lone low surrogate, or bad pair | `[x] NULL; invalid_syntax` |
| 102 | all loaders | integer below `INT64_MIN` or above `INT64_MAX` | `[x] NULL; numeric_overflow` |
| 103 | all loaders | real conversion overflows | `[x] NULL; numeric_overflow` |
| 104 | all loaders | object key contains decoded NUL | `[x] NULL; null_byte_in_key` |
| 105 | all loaders | duplicate key with `JSON_REJECT_DUPLICATES` | `[x] NULL; duplicate_key` |
| 106 | all loaders | decoded string contains NUL without `JSON_ALLOW_NUL` | `[x] NULL; null_character` |
| 107 | all loaders | nesting depth exceeds `JSON_PARSER_MAX_DEPTH` | `[x] NULL; stack_overflow` |
| 108 | all loaders | invalid token or unexpected token | `[x] NULL; invalid_syntax (or premature_end at EOF)` |
| 109 | all loaders | object lacks string key, colon, or closing brace | `[x] NULL; invalid_syntax` |
| 110 | all loaders | array lacks value or closing bracket | `[x] NULL; invalid_syntax` |
| 111 | all loaders | scalar root without `JSON_DECODE_ANY` | `[x] NULL; invalid_syntax` |
| 112 | all loaders | trailing input without `JSON_DISABLE_EOF_CHECK` | `[x] NULL; end_of_input_expected` |
| 113 | `json_pack*` | null or empty format string | `[x] NULL; invalid_argument` |
| 114 | `json_pack*` | unexpected/unterminated format character or trailing format garbage | `[x] NULL; invalid_format` |
| 115 | `json_pack*` | required string/key/object/value argument is null | `[x] NULL; null_value` |
| 116 | `json_pack*` | `#`, `%`, or `+` used on an optional string | `[x] NULL; invalid_format` |
| 117 | `json_pack*` | supplied string/key bytes are invalid UTF-8 | `[x] NULL; invalid_utf8` |
| 118 | `json_pack*` | floating argument is NaN or infinity | `[x] NULL; numeric_overflow` |
| 119 | `json_unpack*` | root is null | `[x] -1; null_value` |
| 120 | `json_unpack*` | null or empty format string | `[x] -1; invalid_argument` |
| 121 | `json_unpack*` | root type differs from object/array/string/integer/boolean/real/number/null format | `[x] -1; wrong_type` |
| 122 | `json_unpack*` | required object key is absent | `[x] -1; item_not_found` |
| 123 | `json_unpack*` | requested array index is absent | `[x] -1; index_out_of_range` |
| 124 | `json_unpack*` | null object key, string target, or string-length target argument | `[x] -1; null_value` |
| 125 | `json_unpack*` | strict object/array leaves values unpacked | `[x] -1; end_of_input_expected` |
| 126 | `json_unpack*` | token follows `!`/`*`, format ends early, invalid token, or trailing garbage | `[x] -1; invalid_format` |
| 127 | `jsonp_error_set_source` | null error or null source | `[x] no-op` |
| 128 | `jsonp_error_vset` | null error or error text already set | `[x] no-op` |
| 129 | `json_delete` | null pointer | `[x] no-op` |
| 130 | `dtoa_r` | caller buffer is too short for selected mode/digits | `[x] NULL` |

The C assertions in `dump.c`, `load.c`, `strconv.c`, and `dtoa.c` guard
internal invariants after successful lexical/conversion steps. They are not
separate caller-visible rejection returns; the differential suite exercises
the surrounding branches and would observe an assertion termination equally
when run against either shared object.
