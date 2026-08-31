# Error Surface

Mechanically derived from explicit rejection branches, error setters, assertions,
and limits in `c_src/src/*.c`. Allocation-failure rows are driven with the public
allocator hooks. Assertions describe internal invariant failures and expect
`SIGABRT`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|----------|---------------------------------------------|-------------------|
| 1 | `utf8_encode` | `codepoint < 0` | [x] `-1` |
| 2 | `utf8_encode` | `codepoint > 0x10ffff` | [x] `-1` |
| 3 | `utf8_check_first` | continuation byte `0x80..0xbf` | [ ] `0` |
| 4 | `utf8_check_first` | overlong lead byte `0xc0` or `0xc1` | [ ] `0` |
| 5 | `utf8_check_first` | restricted/invalid lead byte `>= 0xf5` | [ ] `0` |
| 6 | `utf8_check_full` | `size` is not 2, 3, or 4 | [ ] `0` |
| 7 | `utf8_check_full` | a trailing byte is outside `0x80..0xbf` | [ ] `0` |
| 8 | `utf8_check_full` | decoded value is above `0x10ffff` | [ ] `0` |
| 9 | `utf8_check_full` | decoded value is a surrogate `0xd800..0xdfff` | [ ] `0` |
| 10 | `utf8_check_full` | sequence is an overlong encoding | [ ] `0` |
| 11 | `utf8_iterate` | first byte has `utf8_check_first(...) == 0` | [ ] `NULL` |
| 12 | `utf8_iterate` | multibyte sequence is truncated (`count > bufsize`) | [ ] `NULL` |
| 13 | `utf8_iterate` | multibyte sequence fails `utf8_check_full` | [ ] `NULL` |
| 14 | `utf8_check_string` | invalid first byte | [ ] `0` |
| 15 | `utf8_check_string` | truncated multibyte sequence | [ ] `0` |
| 16 | `utf8_check_string` | invalid full multibyte sequence | [ ] `0` |
| 17 | `strbuffer_init` | initial 16-byte allocation fails | [ ] `-1` |
| 18 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | [ ] `-1` |
| 19 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX / 2` | [ ] `-1` |
| 20 | `strbuffer_append_bytes` | `strbuff->length > SIZE_MAX - 1 - size` | [ ] `-1` |
| 21 | `strbuffer_append_bytes` | growth reallocation fails | [ ] `-1` |
| 22 | `jsonp_malloc` | `size == 0` | [x] `NULL` |
| 23 | `jsonp_realloc` | emulated realloc with `newSize == 0` | [ ] frees input and returns `NULL` |
| 24 | `jsonp_strndup` | allocation fails | [ ] `NULL` |
| 25 | `jsonp_strtod` | parsed value is `+/-HUGE_VAL` with `errno == ERANGE` | [ ] `-1` |
| 26 | `jsonp_strtod` | parser end pointer differs from buffer end | [ ] `SIGABRT` assertion |
| 27 | `jsonp_dtostr` | `dtoa_r` cannot fit its internal 25-byte digit buffer | [ ] `-1` |
| 28 | `jsonp_dtostr` | caller buffer is shorter than computed representation | [ ] `-1` |
| 29 | `hashtable_init` | bucket allocation fails | [ ] `-1` |
| 30 | `hashtable_set` | rehash allocation fails at load ratio 1 | [ ] `-1` |
| 31 | `hashtable_set` | `key_len >= SIZE_MAX - offsetof(pair_t, key)` | [ ] `-1` |
| 32 | `hashtable_set` | pair allocation fails | [ ] `-1` |
| 33 | `hashtable_get` | key is absent | [ ] `NULL` |
| 34 | `hashtable_del` | key is absent | [ ] `-1` |
| 35 | `hashtable_iter_at` | key is absent | [ ] `NULL` |
| 36 | `hashtable_iter_next` | iterator is the final ordered item | [ ] `NULL` |
| 37 | `jsonp_loop_check` | pointer key is already in `parents` | [ ] `-1` |
| 38 | `json_object` | object or initial hash-table allocation fails | [ ] `NULL` |
| 39 | `json_object_size` | argument is null or not an object | [x] `0` |
| 40 | `json_object_get` / `json_object_getn` | key is null, value is not an object, or key is absent | [ ] `NULL` |
| 41 | `json_object_set_new_nocheck` | key is null | [ ] decrefs value and returns `-1` |
| 42 | `json_object_setn_new_nocheck` | value is null | [ ] `-1` |
| 43 | `json_object_setn_new_nocheck` | key is null, target is not an object, or target equals value | [ ] decrefs value and returns `-1` |
| 44 | `json_object_setn_new_nocheck` | underlying hash-table insertion fails | [ ] decrefs value and returns `-1` |
| 45 | `json_object_set_new` / `json_object_setn_new` | key is null | [ ] decrefs value and returns `-1` |
| 46 | `json_object_setn_new` | key bytes are invalid UTF-8 | [ ] decrefs value and returns `-1` |
| 47 | `json_object_del` / `json_object_deln` | key is null or target is not an object | [ ] `-1` |
| 48 | `json_object_deln` | key does not exist | [ ] `-1` |
| 49 | `json_object_clear` | target is null or not an object | [ ] `-1` |
| 50 | `json_object_update` / `json_object_update_existing` / `json_object_update_missing` | either operand is not an object | [ ] `-1` |
| 51 | `do_object_update_recursive` | either operand is not an object | [ ] `-1` |
| 52 | `do_object_update_recursive` | `other` is already in the parent set | [ ] `-1` |
| 53 | `json_object_update_recursive` | parent-set initialization fails | [ ] `-1` |
| 54 | `json_object_iter` / `json_object_iter_at` | target is not an object or key is null | [ ] `NULL` |
| 55 | `json_object_iter_next` | target is not an object or iterator is null | [ ] `NULL` |
| 56 | `json_object_iter_key` / `json_object_iter_value` / `json_object_key_to_iter` | iterator/key is null | [ ] `NULL` |
| 57 | `json_object_iter_key_len` | iterator is null | [ ] `0` |
| 58 | `json_object_iter_set_new` | target is not an object, iterator is null, or value is null | [ ] decrefs value and returns `-1` |
| 59 | `json_array` | object or initial 8-entry table allocation fails | [ ] `NULL` |
| 60 | `json_array_size` | argument is null or not an array | [x] `0` |
| 61 | `json_array_get` | argument is not an array or `index >= entries` | [ ] `NULL` |
| 62 | `json_array_set_new` | value is null | [ ] `-1` |
| 63 | `json_array_set_new` | target is not an array or target equals value | [ ] decrefs value and returns `-1` |
| 64 | `json_array_set_new` | `index >= entries` | [ ] decrefs value and returns `-1` |
| 65 | `json_array_append_new` | value is null, target is not an array, or target equals value | [ ] `-1` |
| 66 | `json_array_append_new` | backing-table growth fails | [ ] decrefs value and returns `-1` |
| 67 | `json_array_insert_new` | value is null, target is not an array, or target equals value | [ ] `-1` |
| 68 | `json_array_insert_new` | `index > entries` | [ ] decrefs value and returns `-1` |
| 69 | `json_array_insert_new` | backing-table growth fails | [ ] decrefs value and returns `-1` |
| 70 | `json_array_remove` | target is not an array or `index >= entries` | [ ] `-1` |
| 71 | `json_array_clear` | target is not an array | [ ] `-1` |
| 72 | `json_array_extend` | either operand is not an array or growth fails | [ ] `-1` |
| 73 | `json_string` / `json_string_nocheck` / `json_stringn_nocheck` | value pointer is null | [ ] `NULL` |
| 74 | `json_string` / `json_stringn` | bytes are invalid UTF-8 | [ ] `NULL` |
| 75 | string constructors | duplicate/string-object allocation fails | [ ] `NULL` |
| 76 | `json_string_value` / `json_string_length` | argument is null or not a string | [ ] `NULL` / `0` |
| 77 | `json_string_set_nocheck` / `json_string_setn_nocheck` | target is not a string or value is null | [ ] `-1` |
| 78 | `json_string_setn_nocheck` | duplicate allocation fails | [ ] `-1` |
| 79 | `json_string_set` / `json_string_setn` | value is null or bytes are invalid UTF-8 | [ ] `-1` |
| 80 | `json_integer_value` / `json_integer_set` | argument is not an integer | [ ] `0` / `-1` |
| 81 | `json_real` | value is NaN or infinity | [x] `NULL` |
| 82 | `json_real_set` | target is not real, or value is NaN/infinity | [ ] `-1` |
| 83 | `json_real_value` / `json_number_value` | argument has wrong type | [ ] `0.0` |
| 84 | `json_equal` | either pointer is null, types differ, or enum type is invalid | [ ] `0` |
| 85 | `json_copy` / `json_deep_copy` / `do_deep_copy` | argument is null or enum type is invalid | [ ] `NULL` |
| 86 | `json_deep_copy` | parent-set initialization fails or a container cycle is found | [ ] `NULL` |
| 87 | `json_delete` | null pointer or out-of-range `json_type` | [ ] no-op |
| 88 | `json_dump_callback` | scalar root without `JSON_ENCODE_ANY` | [x] `-1` |
| 89 | `json_dump_callback` | callback is invoked and returns nonzero | [ ] `-1` |
| 90 | `json_dump_callback` | root is null, enum type is invalid, cycle found, or parent-set initialization fails | [ ] `-1` |
| 91 | `json_dumpb` | dump rejects input | [ ] `0` |
| 92 | `json_dump_file` | path cannot be opened or close fails | [ ] `-1` |
| 93 | `json_dumpfd` / `json_dumpf` | write fails | [ ] `-1` |
| 94 | sorted object dump | allocated key array fails | [ ] `-1` |
| 95 | sorted object dump | iterated item count differs from object size | [ ] `SIGABRT` assertion |
| 96 | sorted object dump | sorted key lookup unexpectedly returns null | [ ] `SIGABRT` assertion |
| 97 | parser stream | malformed UTF-8 lead/trailing sequence | [x] `NULL`, error `json_error_invalid_utf8` |
| 98 | string lexer | EOF before closing quote | [ ] `NULL`, error `json_error_premature_end_of_input` |
| 99 | string lexer | unescaped newline/control byte | [ ] `NULL`, error `json_error_invalid_syntax` |
| 100 | string lexer | invalid backslash or `\u` escape | [ ] `NULL`, error `json_error_invalid_syntax` |
| 101 | string lexer | lone high surrogate, invalid low surrogate, or lone low surrogate | [ ] `NULL`, error `json_error_invalid_syntax` |
| 102 | number lexer | leading zero followed by a digit, missing fraction digit, or missing exponent digit | [ ] `NULL`, syntax error |
| 103 | number lexer | integer outside `json_int_t` range | [x] `NULL`, error `json_error_numeric_overflow` |
| 104 | number lexer | real conversion overflows | [x] `NULL`, error `json_error_numeric_overflow` |
| 105 | `json_loads` | input pointer is null | [ ] `NULL`, error `json_error_invalid_argument` |
| 106 | `json_loadb` | buffer is null with nonzero length | [x] `NULL`, error `json_error_invalid_argument` |
| 107 | `json_loadf` | `FILE *` is null | [ ] `NULL`, error `json_error_invalid_argument` |
| 108 | `json_loadfd` | descriptor is negative or unreadable | [x] `NULL`, error `json_error_invalid_argument` or parse failure |
| 109 | `json_load_file` | path is null | [ ] `NULL`, error `json_error_invalid_argument` |
| 110 | `json_load_file` | file does not exist/cannot open | [x] `NULL`, error `json_error_cannot_open_file` |
| 111 | `json_load_callback` | callback pointer is null | [x] `NULL`, error `json_error_invalid_argument` |
| 112 | parser | nesting depth exceeds `JSON_PARSER_MAX_DEPTH` (2048) | [x] `NULL`, error `json_error_stack_overflow` |
| 113 | parser object | key contains decoded NUL | [x] `NULL`, error `json_error_null_byte_in_key` |
| 114 | parser object | duplicate key with `JSON_REJECT_DUPLICATES` | [x] `NULL`, error `json_error_duplicate_key` |
| 115 | parser object/array | missing key, colon, comma, value, or closing delimiter | [ ] `NULL`, syntax/premature-end error |
| 116 | parser root | scalar root without `JSON_DECODE_ANY` | [x] `NULL`, error `json_error_invalid_syntax` |
| 117 | parser root | trailing token without `JSON_DISABLE_EOF_CHECK` | [x] `NULL`, error `json_error_end_of_input_expected` |
| 118 | parser string | decoded NUL without `JSON_ALLOW_NUL` | [x] `NULL`, error `json_error_null_character` |
| 119 | `json_pack_ex` / `json_vpack_ex` | format pointer is null or empty | [x] `NULL`, error `json_error_invalid_argument` |
| 120 | pack string/object | required argument is null | [ ] `NULL`, error `json_error_null_value` |
| 121 | pack string | string/key bytes are invalid UTF-8 | [ ] `NULL`, error `json_error_invalid_utf8` |
| 122 | pack object | key format lacks a value, key is not followed by `:`, or key/value separator is malformed | [ ] `NULL`, error `json_error_invalid_format` |
| 123 | pack object/array | closing delimiter is absent or format ends early | [ ] `NULL`, error `json_error_invalid_format` |
| 124 | pack real | argument is NaN or infinity | [x] `NULL`, error `json_error_numeric_overflow` |
| 125 | pack | unexpected format character | [x] `NULL`, error `json_error_invalid_format` |
| 126 | pack | non-whitespace remains after one complete value | [x] `NULL`, error `json_error_invalid_format` |
| 127 | `json_unpack_ex` / `json_vunpack_ex` | root is null | [x] `-1`, error `json_error_null_value` |
| 128 | `json_unpack_ex` / `json_vunpack_ex` | format pointer is null or empty | [ ] `-1`, error `json_error_invalid_argument` |
| 129 | unpack object/array/scalar | root type does not match format | [ ] `-1`, error `json_error_wrong_type` |
| 130 | unpack object | requested nonoptional key is absent | [x] `-1`, error `json_error_item_not_found` |
| 131 | unpack array | requested index is outside root array | [x] `-1`, error `json_error_index_out_of_range` |
| 132 | strict unpack | object keys or array items remain | [ ] `-1`, error `json_error_end_of_input_expected` |
| 133 | unpack format | token follows `!`/`*`, delimiter is absent, or character is unexpected | [ ] `-1`, error `json_error_invalid_format` |
| 134 | unpack string | output string or length pointer is null without `JSON_VALIDATE_ONLY` | [x] `-1`, error `json_error_null_value` |
| 135 | unpack | non-whitespace remains after one complete format | [ ] `-1`, error `json_error_invalid_format` |
| 136 | `jsonp_error_set_source` / `jsonp_error_vset` | destination error pointer is null | [ ] no-op |
| 137 | `jsonp_error_vset` | error text is already nonempty | [x] preserves first error |
| 138 | `jsonp_error_set` | `code` is outside the C enum range | [x] stores low byte unchanged in `text[159]` |
