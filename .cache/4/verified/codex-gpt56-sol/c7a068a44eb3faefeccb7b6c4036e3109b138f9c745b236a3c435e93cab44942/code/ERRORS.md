# Error Surface

Rows are mechanically derived from explicit checks, error setters, sentinel
returns, range checks, and assertions in the C sources. Allocation failures are
induced with the public allocator hooks. Internal propagation sites with the
same triggering failed callback/allocation are grouped only when they are the
same rejection as observed at the named entry point.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---:|---|---|---|:---:|
| 1 | `jsonp_malloc` | `size == 0` | `NULL` | [x] |
| 2 | `jsonp_realloc` | null realloc hook and `newSize == 0` | frees non-null input, returns `NULL` | [x] |
| 3 | `jsonp_strndup` | allocator rejects `len + 1` | `NULL` | [x] |
| 4 | `strbuffer_init` | 16-byte allocation fails | `-1` | [x] |
| 5 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | `-1` | [x] |
| 6 | `strbuffer_append_bytes` | `length > SIZE_MAX - 1 - size` | `-1` | [x] |
| 7 | `strbuffer_append_bytes` | growth realloc fails | `-1` | [x] |
| 8 | `utf8_encode` | codepoint `< 0` | `-1` | [x] |
| 9 | `utf8_encode` | codepoint `> 0x10FFFF` | `-1` | [x] |
| 10 | `utf8_check_first` | continuation, `C0/C1`, or byte `>= F5` | `0` | [x] |
| 11 | `utf8_check_full` | size not 2, 3, or 4 | `0` | [x] |
| 12 | `utf8_check_full` | non-continuation trailing byte | `0` | [x] |
| 13 | `utf8_check_full` | decoded value `> 0x10FFFF` | `0` | [x] |
| 14 | `utf8_check_full` | decoded value in `0xD800..=0xDFFF` | `0` | [x] |
| 15 | `utf8_check_full` | overlong 2/3/4-byte encoding | `0` | [x] |
| 16 | `utf8_iterate` | invalid first byte | `NULL` | [x] |
| 17 | `utf8_iterate` | multibyte count exceeds buffer size | `NULL` | [x] |
| 18 | `utf8_iterate` | full multibyte validation fails | `NULL` | [x] |
| 19 | `utf8_check_string` | invalid first byte in range | `0` | [x] |
| 20 | `utf8_check_string` | truncated multibyte sequence | `0` | [x] |
| 21 | `utf8_check_string` | invalid full multibyte sequence | `0` | [x] |
| 22 | `jsonp_strtod` | parsed value is `+/-HUGE_VAL` and `errno == ERANGE` | `-1` | [x] |
| 23 | `jsonp_strtod` | end pointer differs from `value + length` | C assertion | [x] |
| 24 | `jsonp_dtostr` | output buffer too short | `-1` | [x] |
| 25 | `dtoa_r` | caller buffer too short | `NULL` | [x] |
| 26 | `json_object` | object or hashtable allocation fails | `NULL` | [x] |
| 27 | `json_object_get` | key is null | `NULL` | [x] |
| 28 | `json_object_getn` | key null or value not object | `NULL` | [x] |
| 29 | object set-new variants | value is null | `-1` | [x] |
| 30 | object set-new variants | key is null | decrefs value, returns `-1` | [x] |
| 31 | object set-new variants | target is not object | decrefs value, returns `-1` | [x] |
| 32 | object set-new variants | target pointer equals value | decrefs value, returns `-1` | [x] |
| 33 | checked object set variants | key bytes are invalid UTF-8 | decrefs value, returns `-1` | [x] |
| 34 | object set-new variants | hashtable allocation/rehash fails | decrefs value, returns `-1` | [x] |
| 35 | `json_object_del` | key is null | `-1` | [x] |
| 36 | `json_object_deln` | key null or target not object | `-1` | [x] |
| 37 | `json_object_deln` | key absent | `-1` | [x] |
| 38 | `json_object_clear` | target not object | `-1` | [x] |
| 39 | object update family | either operand not object | `-1` | [x] |
| 40 | recursive object update | cycle detected in source graph | `-1` | [x] |
| 41 | `json_object_iter` | target not object | `NULL` | [x] |
| 42 | `json_object_iter_at` | key null, non-object, or absent key | `NULL` | [x] |
| 43 | `json_object_iter_next` | non-object or null iterator | `NULL` | [x] |
| 44 | iterator key/value family | iterator null | key/value `NULL`, key length `0` | [x] |
| 45 | `json_object_iter_set_new` | non-object, null iterator, or null value | decrefs value, returns `-1` | [x] |
| 46 | `json_object_key_to_iter` | key null | `NULL` | [x] |
| 47 | `json_array` | structure or initial table allocation fails | `NULL` | [x] |
| 48 | `json_array_get` | target not array or index `>= entries` | `NULL` | [x] |
| 49 | `json_array_set_new` | value null | `-1` | [x] |
| 50 | `json_array_set_new` | non-array, self value, or index `>= entries` | decrefs value, returns `-1` | [x] |
| 51 | `json_array_append_new` | value null | `-1` | [x] |
| 52 | `json_array_append_new` | non-array or self value | decrefs value, returns `-1` | [x] |
| 53 | `json_array_append_new` | growth allocation fails | decrefs value, returns `-1` | [x] |
| 54 | `json_array_insert_new` | value null | `-1` | [x] |
| 55 | `json_array_insert_new` | non-array, self value, or index `> entries` | decrefs value, returns `-1` | [x] |
| 56 | `json_array_insert_new` | growth allocation fails | decrefs value, returns `-1` | [x] |
| 57 | `json_array_remove` | non-array or index `>= entries` | `-1` | [x] |
| 58 | `json_array_clear` | target not array | `-1` | [x] |
| 59 | `json_array_extend` | either operand not array | `-1` | [x] |
| 60 | `json_array_extend` | growth allocation fails | `-1` | [x] |
| 61 | string constructors | value null | `NULL` | [x] |
| 62 | checked string constructors | invalid UTF-8 in explicit range | `NULL` | [x] |
| 63 | string constructors | copy or object allocation fails | `NULL` | [x] |
| 64 | string getters | target not string | value `NULL`, length `0` | [x] |
| 65 | string setters | value null | `-1` | [x] |
| 66 | string setters | target not string | `-1` | [x] |
| 67 | checked string setters | invalid UTF-8 in explicit range | `-1` | [x] |
| 68 | string setters | duplication allocation fails | `-1` | [x] |
| 69 | `json_integer` | allocation fails | `NULL` | [x] |
| 70 | integer getter/setter | target not integer | getter `0`, setter `-1` | [x] |
| 71 | `json_real` | NaN or either infinity | `NULL` | [x] |
| 72 | `json_real` | allocation fails | `NULL` | [x] |
| 73 | real getter/setter | target not real | getter `0.0`, setter `-1` | [x] |
| 74 | `json_real_set` | NaN or either infinity | `-1` | [x] |
| 75 | `json_equal` | either pointer null or types differ | `0` | [x] |
| 76 | copy family | input null or type enum outside `JSON_OBJECT..JSON_NULL` | `NULL` | [x] |
| 77 | deep-copy/update/dump | cyclic object or array graph | `NULL`, `-1`, or failed dump as declared | [x] |
| 78 | load string scanner | EOF before closing quote without a pending escape | `NULL`, `json_error_premature_end_of_input` | [x] |
| 79 | load string scanner | unescaped byte `0x00..0x1F` | `NULL`, `json_error_invalid_syntax` | [x] |
| 80 | load string scanner | non-hex digit in `\uXXXX` | `NULL`, `json_error_invalid_syntax` | [x] |
| 81 | load string scanner | escape other than `"\/bfnrtu` | `NULL`, `json_error_invalid_syntax` | [x] |
| 82 | load string decoder | invalid high/low surrogate pairing | `NULL`, `json_error_invalid_syntax` | [x] |
| 83 | load stream | invalid or truncated raw UTF-8 | `NULL`, `json_error_invalid_utf8` | [x] |
| 84 | load number scanner | leading zero followed by digit | `NULL`, `json_error_invalid_syntax` | [x] |
| 85 | load number scanner | decimal point without following digit | `NULL`, `json_error_invalid_syntax` | [x] |
| 86 | load number scanner | exponent without following digit | `NULL`, `json_error_invalid_syntax` | [x] |
| 87 | load number conversion | integer outside `json_int_t` range | `NULL`, `json_error_numeric_overflow` | [x] |
| 88 | load number conversion | real conversion overflows | `NULL`, `json_error_numeric_overflow` | [x] |
| 89 | object parser | token after `{` is neither string nor `}` | `NULL`, `json_error_invalid_syntax` | [x] |
| 90 | object parser | decoded key contains NUL | `NULL`, `json_error_null_byte_in_key` | [x] |
| 91 | object parser | duplicate key with `JSON_REJECT_DUPLICATES` | `NULL`, `json_error_duplicate_key` | [x] |
| 92 | object parser | colon absent after key | `NULL`, `json_error_invalid_syntax` | [x] |
| 93 | object parser | closing `}` absent | `NULL`, syntax or premature-end error | [x] |
| 94 | array parser | closing `]` absent or separator invalid | `NULL`, syntax or premature-end error | [x] |
| 95 | value parser | nesting depth reaches 2048 | `NULL`, `json_error_stack_overflow` | [x] |
| 96 | value parser | decoded string contains NUL without `JSON_ALLOW_NUL` | `NULL`, `json_error_null_character` | [x] |
| 97 | value parser | invalid/unexpected token | `NULL`, `json_error_invalid_syntax` | [x] |
| 98 | top-level parser | scalar without `JSON_DECODE_ANY` | `NULL`, `json_error_invalid_syntax` | [x] |
| 99 | top-level parser | trailing token without `JSON_DISABLE_EOF_CHECK` | `NULL`, `json_error_end_of_input_expected` | [x] |
| 100 | load entry points | null string/buffer/`FILE*`/callback/path, negative fd | `NULL`, `json_error_invalid_argument` | [x] |
| 101 | `json_load_file` | path cannot be opened | `NULL`, `json_error_cannot_open_file` | [x] |
| 102 | load entry points | lexer buffer allocation fails | `NULL` | [x] |
| 103 | `json_dump_callback` | scalar without `JSON_ENCODE_ANY` | `-1` | [x] |
| 104 | dump family | input null or invalid type enum | callback/file return `-1`, string `NULL`, buffer `0` | [x] |
| 105 | dump family | circular container reference | failed dump sentinel | [x] |
| 106 | dump family | user callback rejects a chunk | `-1` | [x] |
| 107 | `json_dump_file` | output path cannot be opened | `-1` | [x] |
| 108 | dump formatting | allocator fails for parent set/output/sorted-key array | failed dump sentinel | [x] |
| 109 | pack entry points | format null or empty | `NULL`, `json_error_invalid_argument` | [x] |
| 110 | pack string | required string/key argument null | `NULL`, `json_error_null_value` | [x] |
| 111 | pack string | invalid UTF-8 argument | `NULL`, `json_error_invalid_utf8` | [x] |
| 112 | pack string | `#`, `%`, or `+` used on optional string | `NULL`, `json_error_invalid_format` | [x] |
| 113 | pack object/array | format ends before closing delimiter | `NULL`, `json_error_invalid_format` | [x] |
| 114 | pack object | next member format is not `s` | `NULL`, `json_error_invalid_format` | [x] |
| 115 | pack object | required packed value is null | `NULL`, `json_error_null_value` | [x] |
| 116 | pack `o`/`O` | required object argument null | `NULL`, `json_error_null_value` | [x] |
| 117 | pack real | NaN or infinity | `NULL`, `json_error_numeric_overflow` | [x] |
| 118 | pack dispatcher | unknown format character | `NULL`, `json_error_invalid_format` | [x] |
| 119 | pack entry points | nonignored characters remain after one value | `NULL`, `json_error_invalid_format` | [x] |
| 120 | unpack entry points | root null | `-1`, `json_error_null_value` | [x] |
| 121 | unpack entry points | format null or empty | `-1`, `json_error_invalid_argument` | [x] |
| 122 | unpack object/array | root has wrong type | `-1`, `json_error_wrong_type` | [x] |
| 123 | unpack object/array | format token after `!`/`*` before close | `-1`, `json_error_invalid_format` | [x] |
| 124 | unpack object/array | format ends before closing delimiter | `-1`, `json_error_invalid_format` | [x] |
| 125 | unpack object | member token is not `s` or key argument null | `-1`, invalid-format or null-value code | [x] |
| 126 | unpack object | required key absent | `-1`, `json_error_item_not_found` | [x] |
| 127 | strict unpack object | object members remain unpacked | `-1`, `json_error_end_of_input_expected` | [x] |
| 128 | unpack array | format token cannot start a value | `-1`, `json_error_invalid_format` | [x] |
| 129 | unpack array | requested index absent | `-1`, `json_error_index_out_of_range` | [x] |
| 130 | strict unpack array | array items remain unpacked | `-1`, `json_error_end_of_input_expected` | [x] |
| 131 | unpack scalar | root type mismatches `s/i/I/b/f/F/n` | `-1`, `json_error_wrong_type` | [x] |
| 132 | unpack string | output string or length pointer null | `-1`, `json_error_null_value` | [x] |
| 133 | unpack dispatcher | unknown format character | `-1`, `json_error_invalid_format` | [x] |
| 134 | unpack entry points | garbage remains after one format value | `-1`, `json_error_invalid_format` | [x] |
| 135 | hashtable pair creation | `key_len >= SIZE_MAX - offsetof(pair_t,key)` | `-1` from `hashtable_set` | [x] |
| 136 | hashtable operations | allocation/rehash fails | init/set return `-1` | [x] |
| 137 | `jsonp_loop_check` | pointer key already present | `-1` | [x] |
| 138 | load string scanner | EOF immediately after `\` or before four `\u` hex digits are complete | `NULL`, `json_error_invalid_syntax` | [x] |
| 139 | pack modified string | scratch-buffer initialization or growth allocation fails | `NULL`, `json_error_out_of_memory` | [x] |
| 140 | pack object | object-member insertion allocation fails | `NULL`, `json_error_out_of_memory` | [x] |
| 141 | pack array | append growth allocation fails | `NULL`, `json_error_out_of_memory` | [x] |
| 142 | pack integer/real | scalar allocation fails | `NULL`, `json_error_out_of_memory` | [x] |
| 143 | unpack object | temporary key-set initialization fails | `-1`, `json_error_out_of_memory` | [x] |
| 144 | `json_vsprintf` | `vsnprintf` length calculation returns a negative value | `NULL` | [x] |
| 145 | `json_vsprintf` | formatted-output allocation fails | `NULL` | [x] |
| 146 | `json_vsprintf` | formatted output is invalid UTF-8 | `NULL` | [x] |
