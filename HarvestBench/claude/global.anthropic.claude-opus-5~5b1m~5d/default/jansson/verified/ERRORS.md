# ERRORS.md — Exhaustive error-surface table for jansson 2.15 (`c_src/`)

Derived mechanically from `c_src/include/jansson.h`, `c_src/include/jansson_private.h`,
`c_src/include/jansson_config.h` (`JSON_PARSER_MAX_DEPTH 2048`, `JSON_INTEGER_IS_LONG_LONG 1`),
`c_src/include/jansson_private_config.h` (`DTOA_ENABLED 1`, `HAVE_ATOMIC_BUILTINS`, `HAVE_UNISTD_H`,
`USE_URANDOM`) and every `.c` file in `c_src/src/` (for `dtoa.c`, only the paths reachable from the
exported `dtoa`, `dtoa_r`, `gethex`, `freedtoa` and from `jsonp_dtostr` / `jsonp_strtod`).

Reference constants: `JSON_ERROR_TEXT_LENGTH 160`, `JSON_ERROR_SOURCE_LENGTH 80`,
`JSON_PARSER_MAX_DEPTH 2048`, `JSON_MAX_INDENT 0x1F`, `MAX_INTEGER_STR_LENGTH 25`,
`MAX_REAL_STR_LENGTH 25`, `MAX_BUF_LEN 1024`, `STRBUFFER_MIN_SIZE 16`, `INITIAL_HASHTABLE_ORDER 3`.

`enum json_error_code` numeric values (what `json_error_code(&err)` returns):
`0 unknown`, `1 out_of_memory`, `2 stack_overflow`, `3 cannot_open_file`, `4 invalid_argument`,
`5 invalid_utf8`, `6 premature_end_of_input`, `7 end_of_input_expected`, `8 invalid_syntax`,
`9 invalid_format`, `10 wrong_type`, `11 null_character`, `12 null_value`, `13 null_byte_in_key`,
`14 duplicate_key`, `15 numeric_overflow`, `16 item_not_found`, `17 index_out_of_range`.

All "expected C result" entries below were verified by compiling test programs against
`c_src/build/libjansson.so` unless the row is called out in the **Notes** section as untestable.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `json_object_size` | `json` is `NULL` (`!json_is_object`) | `0` | [x] |
| 2 | `json_object_size` | `json` is not `JSON_OBJECT` (e.g. `json_integer(1)`) | `0` | [x] |
| 3 | `json_object_get` | `key == NULL` | `NULL` (returns before `strlen`) | [x] |
| 4 | `json_object_getn` | `key == NULL` | `NULL` | [x] |
| 5 | `json_object_getn` | `json == NULL` or `json_typeof(json) != JSON_OBJECT` | `NULL` | [x] |
| 6 | `json_object_getn` / `json_object_get` | key absent from the hashtable (`hashtable_get` finds no pair) | `NULL` | [x] |
| 7 | `json_object_set_new_nocheck` | `key == NULL` | `-1`; `value` is decref'd (leak-free) | [x] |
| 8 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` (checked before `key`/type) | [x] |
| 9 | `json_object_setn_new_nocheck` | `key == NULL` (with non-NULL value) | `-1`; `value` decref'd | [x] |
| 10 | `json_object_setn_new_nocheck` | `json` is not an object (`json_integer(1)`, `NULL`, array) | `-1`; `value` decref'd | [x] |
| 11 | `json_object_setn_new_nocheck` | `json == value` (self insertion, e.g. `json_object_set(o,"k",o)`) | `-1`; `value` decref'd | [x] |
| 12 | `json_object_setn_new_nocheck` | `hashtable_set` fails (rehash malloc or `init_pair` malloc fails) | `-1`; `value` decref'd || [x] |
| 13 | `json_object_set_new` | `key == NULL` | `-1`; `value` decref'd | [x] |
| 14 | `json_object_setn_new` | `key == NULL` | `-1`; `value` decref'd | [x] |
| 15 | `json_object_setn_new` | `key` is not valid UTF-8 for `key_len` bytes (e.g. `"\xff"`, len 1) | `-1`; `value` decref'd (note `json_object_set_nocheck` accepts it and returns `0`) | [x] |
| 16 | `json_object_del` | `key == NULL` | `-1` | [x] |
| 17 | `json_object_deln` | `key == NULL` | `-1` | [x] |
| 18 | `json_object_deln` / `json_object_del` | `json` is not an object | `-1` | [x] |
| 19 | `json_object_deln` / `json_object_del` | key not present (`hashtable_do_del` finds no pair) | `-1` | [x] |
| 20 | `json_object_clear` | `json` is not an object (or `NULL`) | `-1` | [x] |
| 21 | `json_object_update` | `object` is not an object | `-1` | [x] |
| 22 | `json_object_update` | `other` is not an object | `-1` | [x] |
| 23 | `json_object_update` | inner `json_object_setn_nocheck` fails because `other` holds `object` as a value (`p={"k":o}`; `json_object_update(o,p)` → `json == value`) | `-1`, iteration aborted mid-way (partial update possible) | [x] |
| 24 | `json_object_update_existing` | `object` is not an object | `-1` | [x] |
| 25 | `json_object_update_existing` | `other` is not an object | `-1` | [x] |
| 26 | `json_object_update_missing` | `object` is not an object | `-1` | [x] |
| 27 | `json_object_update_missing` | `other` is not an object | `-1` | [x] |
| 28 | `json_object_update_recursive` | `object` is not an object | `-1` | [x] |
| 29 | `json_object_update_recursive` | `other` is not an object | `-1` | [x] |
| 30 | `json_object_update_recursive` | circular reference: `other` is reachable from itself along keys that are objects in `object` too (build `X={}`, `Y={"a":X}`, then `X["a"]=Y`; `object = {"a":{"a":{"a":1}}}`; call with `other=X`) — `jsonp_loop_check` finds the pointer already in `parents` | `-1` | [x] |
| 31 | `json_object_update_recursive` | inner `json_object_setn_nocheck` fails (self-insertion or OOM) | `-1` | [x] |
| 32 | `json_object_update_recursive` | `hashtable_init(&parents_set)` fails (malloc) | `-1` | [x] |
| 33 | `json_object_iter` | `json` is not an object (or `NULL`) | `NULL` | [x] |
| 34 | `json_object_iter` | object is empty (`hashtable_iter_next` hits `&ordered_list`) | `NULL` | [x] |
| 35 | `json_object_iter_at` | `key == NULL` | `NULL` | [x] |
| 36 | `json_object_iter_at` | `json` is not an object | `NULL` | [x] |
| 37 | `json_object_iter_at` | key not present | `NULL` | [x] |
| 38 | `json_object_iter_next` | `json` is not an object | `NULL` | [x] |
| 39 | `json_object_iter_next` | `iter == NULL` | `NULL` | [x] |
| 40 | `json_object_iter_next` | `iter` points at the last pair in insertion order | `NULL` | [x] |
| 41 | `json_object_iter_key` | `iter == NULL` | `NULL` | [x] |
| 42 | `json_object_iter_key_len` | `iter == NULL` | `0` | [x] |
| 43 | `json_object_iter_value` | `iter == NULL` | `NULL` | [x] |
| 44 | `json_object_iter_set_new` | `json` is not an object (e.g. `json_array()`) | `-1`; `value` decref'd | [x] |
| 45 | `json_object_iter_set_new` | `iter == NULL` | `-1`; `value` decref'd | [x] |
| 46 | `json_object_iter_set_new` | `value == NULL` | `-1` | [x] |
| 47 | `json_object_key_to_iter` | `key == NULL` | `NULL` | [x] |
| 48 | `json_array_size` | `json` is `NULL` or not `JSON_ARRAY` | `0` | [x] |
| 49 | `json_array_get` | `json` is `NULL` or not an array | `NULL` | [x] |
| 50 | `json_array_get` | `index >= array->entries` (e.g. `json_array_get(json_array(), 0)`) | `NULL` | [x] |
| 51 | `json_array_set_new` | `value == NULL` | `-1` (checked first) | [x] |
| 52 | `json_array_set_new` | `json` is not an array | `-1`; `value` decref'd | [x] |
| 53 | `json_array_set_new` | `json == value` (self insertion) | `-1`; `value` decref'd | [x] |
| 54 | `json_array_set_new` | `index >= array->entries` | `-1`; `value` decref'd | [x] |
| 55 | `json_array_append_new` | `value == NULL` | `-1` | [x] |
| 56 | `json_array_append_new` | `json` is not an array | `-1`; `value` decref'd | [x] |
| 57 | `json_array_append_new` | `json == value` (`json_array_append(a,a)`) | `-1`; `value` decref'd | [x] |
| 58 | `json_array_append_new` | `json_array_grow` fails (`jsonp_realloc` returns `NULL`) | `-1`; `value` decref'd || [x] |
| 59 | `json_array_insert_new` | `value == NULL` | `-1` | [x] |
| 60 | `json_array_insert_new` | `json` is not an array | `-1`; `value` decref'd | [x] |
| 61 | `json_array_insert_new` | `json == value` | `-1`; `value` decref'd | [x] |
| 62 | `json_array_insert_new` | `index > array->entries` (note: `==` is legal, e.g. insert at 1 into an empty array) | `-1`; `value` decref'd | [x] |
| 63 | `json_array_insert_new` | `json_array_grow` fails | `-1`; `value` decref'd | [x] |
| 64 | `json_array_remove` | `json` is not an array | `-1` | [x] |
| 65 | `json_array_remove` | `index >= array->entries` | `-1` | [x] |
| 66 | `json_array_clear` | `json` is not an array | `-1` | [x] |
| 67 | `json_array_extend` | `json` is not an array | `-1` | [x] |
| 68 | `json_array_extend` | `other` is not an array | `-1` | [x] |
| 69 | `json_array_extend` | `json_array_grow(array, other->entries)` fails | `-1` (no refcounts touched) | [x] |
| 70 | `json_string_nocheck` | `value == NULL` | `NULL` | [x] |
| 71 | `json_stringn_nocheck` | `value == NULL` (`string_create` `!value`) | `NULL` | [x] |
| 72 | `json_stringn_nocheck` | `jsonp_strndup(value, len)` fails (malloc) | `NULL` || [x] |
| 73 | `json_stringn_nocheck` | `jsonp_malloc(sizeof(json_string_t))` fails | `NULL`; the duplicated buffer is freed || [x] |
| 74 | `json_string` | `value == NULL` | `NULL` | [x] |
| 75 | `json_stringn` | `value == NULL` | `NULL` | [x] |
| 76 | `json_stringn` / `json_string` | `!utf8_check_string(value, len)` (e.g. `"\xff"`) | `NULL` | [x] |
| 77 | `jsonp_stringn_nocheck_own` | `value == NULL` | `NULL` | [x] |
| 78 | `json_string_value` | `json` is `NULL` or not `JSON_STRING` | `NULL` | [x] |
| 79 | `json_string_length` | `json` is `NULL` or not `JSON_STRING` | `0` | [x] |
| 80 | `json_string_set_nocheck` | `value == NULL` | `-1` | [x] |
| 81 | `json_string_setn_nocheck` | `json` is not a string | `-1` | [x] |
| 82 | `json_string_setn_nocheck` | `value == NULL` | `-1` | [x] |
| 83 | `json_string_setn_nocheck` | `jsonp_strndup` fails | `-1`; the target string is left unchanged || [x] |
| 84 | `json_string_set` | `value == NULL` | `-1` | [x] |
| 85 | `json_string_setn` | `value == NULL` | `-1` | [x] |
| 86 | `json_string_setn` / `json_string_set` | `!utf8_check_string(value, len)` | `-1` | [x] |
| 87 | `json_vsprintf` / `json_sprintf` | `vsnprintf(NULL,0,fmt,ap)` returns `< 0` (encoding error) | `NULL` | [x] |
| 88 | `json_vsprintf` / `json_sprintf` | formatted result is not valid UTF-8 (`json_sprintf("%s","\xff")`) | `NULL`; the temp buffer is freed | [x] |
| 89 | `json_vsprintf` / `json_sprintf` | `jsonp_malloc(length+1)` fails | `NULL` || [x] |
| 90 | `json_integer` | `jsonp_malloc(sizeof(json_integer_t))` fails | `NULL` || [x] |
| 91 | `json_integer_value` | `json` is `NULL` or not `JSON_INTEGER` | `0` | [x] |
| 92 | `json_integer_set` | `json` is not `JSON_INTEGER` | `-1` | [x] |
| 93 | `json_real` | `isnan(value)` (`json_real(NAN)`) | `NULL` | [x] |
| 94 | `json_real` | `isinf(value)` (`json_real(INFINITY)` or `-INFINITY`) | `NULL` | [x] |
| 95 | `json_real` | `jsonp_malloc(sizeof(json_real_t))` fails | `NULL` || [x] |
| 96 | `json_real_value` | `json` is `NULL` or not `JSON_REAL` | `0` (as `double`) | [x] |
| 97 | `json_real_set` | `json` is not `JSON_REAL` | `-1` | [x] |
| 98 | `json_real_set` | `isnan(value)` | `-1`; value unchanged | [x] |
| 99 | `json_real_set` | `isinf(value)` | `-1`; value unchanged | [x] |
| 100 | `json_number_value` | `json` is neither integer nor real (`NULL`, string, array, bool, null) | `0.0` | [x] |
| 101 | `json_object` | `jsonp_malloc(sizeof(json_object_t))` fails | `NULL` || [x] |
| 102 | `json_object` | `hashtable_init` fails (bucket malloc) | `NULL`; the object struct is freed || [x] |
| 103 | `json_array` | `jsonp_malloc(sizeof(json_array_t))` fails | `NULL` || [x] |
| 104 | `json_array` | `jsonp_malloc(8 * sizeof(json_t*))` for the table fails | `NULL`; the array struct is freed || [x] |
| 105 | `json_delete` | `json == NULL` | returns immediately, no free | [x] |
| 106 | `json_delete` | `json_typeof(json)` is `JSON_TRUE`/`JSON_FALSE`/`JSON_NULL` (the `default:` arm) | returns without freeing (singletons) | [x] |
| 107 | `json_equal` | `value1 == NULL` | `0` | [x] |
| 108 | `json_equal` | `value2 == NULL` | `0` | [x] |
| 109 | `json_equal` | `json_typeof(v1) != json_typeof(v2)` (e.g. integer vs string) | `0` | [x] |
| 110 | `json_equal` | both objects, `json_object_size` differs | `0` | [x] |
| 111 | `json_equal` | both objects, a key of `value1` is absent from `value2` (inner `json_equal(v, NULL)`) | `0` | [x] |
| 112 | `json_equal` | both arrays, `json_array_size` differs | `0` | [x] |
| 113 | `json_equal` | both arrays, some element pair is unequal | `0` | [x] |
| 114 | `json_equal` | both strings, lengths differ or `memcmp` differs (`"a"` vs `"ab"`) | `0` | [x] |
| 115 | `json_equal` | both integers with different values | `0` | [x] |
| 116 | `json_equal` | both reals with different values | `0` | [x] |
| 117 | `json_copy` | `json == NULL` | `NULL` | [x] |
| 118 | `json_copy` | `json_object()` inside `json_object_copy` fails (OOM) | `NULL` | [x] |
| 119 | `json_copy` | `json_array()` inside `json_array_copy` fails (OOM) | `NULL` | [x] |
| 120 | `json_deep_copy` | `json == NULL` | `NULL` | [x] |
| 121 | `json_deep_copy` | `hashtable_init(&parents_set)` fails | `NULL` || [x] |
| 122 | `json_deep_copy` | circular reference through arrays: `a=[b]`, `b=[a]`, copy `a` — `jsonp_loop_check` sees `a` again | `NULL` | [x] |
| 123 | `json_deep_copy` | circular reference through objects: `a={"b":b}`, `b={"a":a}`, copy `a` | `NULL` | [x] |
| 124 | `jsonp_loop_check` | `json` pointer key already present in `parents` | `-1` | [x] |
| 125 | `jsonp_loop_check` | `hashtable_set(parents, ...)` fails (OOM) | `-1` | [x] |
| 126 | `utf8_encode` | `codepoint < 0` (e.g. `-1`) | `-1`, `*size` untouched || [x] |
| 127 | `utf8_encode` | `codepoint > 0x10FFFF` (e.g. `0x110000`) | `-1`, `*size` untouched || [x] |
| 128 | `utf8_check_first` | byte in `0x80..0xBF` (bare continuation byte) | `0` || [x] |
| 129 | `utf8_check_first` | byte `== 0xC0` or `== 0xC1` (overlong ASCII lead) | `0` || [x] |
| 130 | `utf8_check_first` | byte `>= 0xF5` (`0xF5..0xFF`) | `0` || [x] |
| 131 | `utf8_check_full` | `size` is not 2, 3 or 4 (e.g. `1` or `5`) | `0` || [x] |
| 132 | `utf8_check_full` | any byte at index `>= 1` is `< 0x80` or `> 0xBF` (e.g. `"\xC2\x41"`) | `0` || [x] |
| 133 | `utf8_check_full` | decoded value `> 0x10FFFF` (e.g. `"\xF4\x90\x80\x80"`) | `0` || [x] |
| 134 | `utf8_check_full` | decoded value in `0xD800..0xDFFF` (surrogate, e.g. `"\xED\xA0\x80"`) | `0` || [x] |
| 135 | `utf8_check_full` | overlong: `size == 2 && value < 0x80` (e.g. `"\xC0\x80"` — also caught by `utf8_check_first`) | `0` || [x] |
| 136 | `utf8_check_full` | overlong: `size == 3 && value < 0x800` (e.g. `"\xE0\x80\x80"`) | `0` || [x] |
| 137 | `utf8_check_full` | overlong: `size == 4 && value < 0x10000` (e.g. `"\xF0\x80\x80\x80"`) | `0` || [x] |
| 138 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged (non-`NULL`), `*codepoint` untouched || [x] |
| 139 | `utf8_iterate` | `utf8_check_first(buffer[0]) == 0` (e.g. `"\xFF"`) | `NULL` || [x] |
| 140 | `utf8_iterate` | multi-byte sequence truncated: `count > bufsize` (e.g. `"\xC2"`, bufsize 1) | `NULL` || [x] |
| 141 | `utf8_iterate` | `utf8_check_full` rejects the sequence | `NULL` || [x] |
| 142 | `utf8_check_string` | some byte fails `utf8_check_first` | `0` || [x] |
| 143 | `utf8_check_string` | multi-byte sequence runs past `length` (`count > length - i`) | `0` || [x] |
| 144 | `utf8_check_string` | `utf8_check_full` rejects a sequence | `0` || [x] |
| 145 | `json_loads` | `string == NULL` | `NULL`; code `4` `json_error_invalid_argument`, text `"wrong arguments"`, source `"<string>"`, line `-1`, column `-1`, position `0` | [x] |
| 146 | `json_loadb` | `buffer == NULL` | `NULL`; code `4`, text `"wrong arguments"`, source `"<buffer>"` | [x] |
| 147 | `json_loadf` | `input == NULL` | `NULL`; code `4`, text `"wrong arguments"`, source `"<stream>"` | [x] |
| 148 | `json_loadfd` | `input < 0` (e.g. `-1`) | `NULL`; code `4`, text `"wrong arguments"`, source `"<stream>"` | [x] |
| 149 | `json_load_file` | `path == NULL` | `NULL`; code `4`, text `"wrong arguments"`, source `""` | [x] |
| 150 | `json_load_file` | `fopen(path,"rb")` fails (e.g. `"/nonexistent/x.json"`) | `NULL`; code `3` `json_error_cannot_open_file`, text `"unable to open /nonexistent/x.json: No such file or directory"`, source = the path | [x] |
| 151 | `json_load_callback` | `callback == NULL` | `NULL`; code `4`, text `"wrong arguments"`, source `"<callback>"` | [x] |
| 152 | `json_load_callback` | callback returns `0` or `(size_t)-1` before a complete value (`callback_get` maps both to `EOF`) | `NULL`; code `6` `json_error_premature_end_of_input`, text `"'[' or '{' expected near end of file"` | [x] |
| 153 | `json_loads` (all `json_load*`) | no `JSON_DECODE_ANY` and the first token is a scalar (`"1"`, `"\"x\""`, `"true"`) | `NULL`; code `8` `json_error_invalid_syntax`, text `"'[' or '{' expected near '1'"`, line 1, column 1, position 1 | [x] |
| 154 | `json_loads` | no `JSON_DECODE_ANY` and input is `""` (EOF immediately) | `NULL`; code `6` (invalid_syntax upgraded to premature_end because no saved text), text `"'[' or '{' expected near end of file"`, position 0 | [x] |
| 155 | `json_loads` | `JSON_DECODE_ANY` and input is `""` (`parse_value` `default:` on `TOKEN_EOF`) | `NULL`; code `6`, text `"unexpected token near end of file"` | [x] |
| 156 | `json_loads` | trailing content and no `JSON_DISABLE_EOF_CHECK` (e.g. `"[1] x"`, `"{} {}"`) | `NULL`; code `7` `json_error_end_of_input_expected`, text `"end of file expected near 'x'"`; the parsed value is decref'd | [x] |
| 157 | `json_loads` | nesting deeper than `JSON_PARSER_MAX_DEPTH` (2049 `'['` characters) | `NULL`; code `2` `json_error_stack_overflow`, text `"maximum parsing depth reached near '['"`, column/position 2049 | [x] |
| 158 | `json_loads` | string contains `\u0000` and `JSON_ALLOW_NUL` not set (`"[\"a\\u0000b\"]"`) | `NULL`; code `11` `json_error_null_character`, text `"\\u0000 is not allowed without JSON_ALLOW_NUL near '\"a\\u0000b\"'"` | [x] |
| 159 | `json_loads` | `TOKEN_INVALID` from an unknown bareword (`"[tru]"`, `"nul"` with `JSON_DECODE_ANY`) | `NULL`; code `8`, text `"invalid token near 'tru'"` | [x] |
| 160 | `json_loads` | `TOKEN_INVALID` from a byte that starts no token (`"[@]"`, `"[#]"`) | `NULL`; code `8`, text `"invalid token near '@'"` | [x] |
| 161 | `json_loads` | value position holds a structural token (`"[,]"`, `"[:]"`, `"[1,2,]"` → `']'` where a value is expected) | `NULL`; code `8`, text `"unexpected token near ','"` / `"unexpected token near ']'"` | [x] |
| 162 | `json_loads` | `jsonp_stringn_nocheck_own` OOM for a `TOKEN_STRING` value | `NULL`, error struct left with whatever the lexer set (usually nothing) | [x] |
| 163 | `json_loads` | `json_integer` OOM for a `TOKEN_INTEGER` value | `NULL` | [x] |
| 164 | `json_loads` | `json_object()` OOM inside `parse_object` | `NULL` | [x] |
| 165 | `json_loads` | object member position is not a string and not `'}'` (`"{1:2}"`, `"{,}"`, `"{\"a\":1,}"`) | `NULL`; code `8`, text `"string or '}' expected near '1'"` / `"... near '}'"` | [x] |
| 166 | `json_loads` | object key contains a NUL byte (`"{\"a\\u0000b\":1}"`, even with `JSON_ALLOW_NUL`) | `NULL`; code `13` `json_error_null_byte_in_key`, text `"NUL byte in object key not supported near '\"a\\u0000b\"'"` | [x] |
| 167 | `json_loads` | `JSON_REJECT_DUPLICATES` and a repeated key (`"{\"a\":1,\"a\":2}"`) | `NULL`; code `14` `json_error_duplicate_key`, text `"duplicate object key near '\"a\"'"` | [x] |
| 168 | `json_loads` | token after an object key is not `':'` (`"{\"a\" 1}"`) | `NULL`; code `8`, text `"':' expected near '1'"` | [x] |
| 169 | `json_loads` | `json_object_setn_new_nocheck` fails while building the object (OOM) | `NULL`; object decref'd | [x] |
| 170 | `json_loads` | after an object member, token is neither `','` nor `'}'` (`"{\"a\":1 \"b\":2}"`) | `NULL`; code `8`, text `"'}' expected near '\"b\"'"` | [x] |
| 171 | `json_loads` | object not terminated before EOF (`"{\"a\":1"`) | `NULL`; code `6` (upgraded), text `"'}' expected near end of file"` | [x] |
| 172 | `json_loads` | `json_array()` OOM inside `parse_array` | `NULL` | [x] |
| 173 | `json_loads` | after an array element, token is neither `','` nor `']'` (`"[1 2]"`) | `NULL`; code `8`, text `"']' expected near '2'"` | [x] |
| 174 | `json_loads` | array not terminated before EOF (`"[1,2"`) | `NULL`; code `6`, text `"']' expected near end of file"`, position 4 | [x] |
| 175 | `json_loads` | `json_array_append_new` fails while building the array (OOM) | `NULL`; array decref'd | [x] |
| 176 | `json_loads` | EOF inside a string literal (`"[\"abc"`) | `NULL`; code `6` `json_error_premature_end_of_input`, text `"premature end of input near '\"abc'"` | [x] |
| 177 | `json_loads` | raw `'\n'` (0x0A) inside a string literal | `NULL`; code `8`, text `"unexpected newline near '\"a'"` | [x] |
| 178 | `json_loads` | any other raw control byte `0x00..0x1F` inside a string (raw TAB) | `NULL`; code `8`, text `"control character 0x9 near '\"a'"` | [x] |
| 179 | `json_loads` | `\u` escape not followed by 4 hex digits (`"[\"\\u12\"]"`, `"[\"\\uZZZZ\"]"`, `"[\"\\uD800\\uZZZZ\"]"`) | `NULL`; code `8`, text `"invalid escape near '\"\\u12\"'"` | [x] |
| 180 | `json_loads` | backslash followed by a character other than `" \ / b f n r t u` (`"[\"\\x\"]"`) | `NULL`; code `8`, text `"invalid escape near '\"\\x'"` | [x] |
| 181 | `json_loads` | backslash at end of input (`"[\"a\\"`) | `NULL`; code `8`, text `"invalid escape near '\"a\\'"` | [x] |
| 182 | `json_loads` | high surrogate `\uD800..\uDBFF` not followed by a `\u` escape (`"[\"\\uD800\"]"`, `"[\"\\uD800x\"]"`) | `NULL`; code `8`, text `"invalid Unicode '\\uD800' near '\"\\uD800\"'"` | [x] |
| 183 | `json_loads` | high surrogate followed by a `\u` escape outside `DC00..DFFF` (`"[\"\\uD800\\u0041\"]"`) | `NULL`; code `8`, text `"invalid Unicode '\\uD800\\u0041' near '\"\\uD800\\u0041\"'"` | [x] |
| 184 | `json_loads` | lone low surrogate `\uDC00..\uDFFF` (`"[\"\\uDC00\"]"`) | `NULL`; code `8`, text `"invalid Unicode '\\uDC00' near '\"\\uDC00\"'"` | [x] |
| 185 | `json_loads` | `jsonp_malloc(saved_text.length+1)` for the decoded string fails | token stays `TOKEN_INVALID` → `NULL`; code `8`, text `"invalid token near ..."` | [x] |
| 186 | `json_loads` | leading zero followed by a digit (`"[01]"`) | `NULL`; code `8`, text `"invalid token near '0'"` | [x] |
| 187 | `json_loads` | `'-'` not followed by a digit (`"[-]"`, `"[-x]"`) | `NULL`; code `8`, text `"invalid token near '-'"` | [x] |
| 188 | `json_loads` | `'.'` not followed by a digit (`"[1.]"`, `"[1.e5]"`) | `NULL`; code `8`, text `"invalid token near '1.'"` | [x] |
| 189 | `json_loads` | `'e'`/`'E'` (with optional sign) not followed by a digit (`"[1e]"`, `"[1e+]"`) | `NULL`; code `8`, text `"invalid token near '1e'"` / `"... '1e+'"` | [x] |
| 190 | `json_loads` | integer literal above `JSON_INTEGER_MAX` (`"[9223372036854775808]"`), `strtoll` sets `ERANGE` and `intval >= 0` | `NULL`; code `15` `json_error_numeric_overflow`, text `"too big integer near '9223372036854775808'"` | [x] |
| 191 | `json_loads` | integer literal below `JSON_INTEGER_MIN` (`"[-9223372036854775809]"`), `ERANGE` and `intval < 0` | `NULL`; code `15`, text `"too big negative integer near '-9223372036854775809'"` | [x] |
| 192 | `json_loads` | real literal overflowing `double` (`"[1e999]"`, also with `JSON_DECODE_ANY`) — `jsonp_strtod` returns `-1` | `NULL`; code `15`, text `"real number overflow near '1e999'"` | [x] |
| 193 | `json_loads` | first byte `>= 0x80` that `utf8_check_first` rejects (`0x80..0xBF`, `0xC0`, `0xC1`, `0xF5..0xFF`), e.g. input `"[\"\xff\"]"` or `"[\xc0\x80]"` | `NULL`; code `5` `json_error_invalid_utf8`, text `"unable to decode byte 0xff"` (no `near` context — stream is in `STREAM_STATE_ERROR`) | [x] |
| 194 | `json_loads` | multi-byte sequence rejected by `utf8_check_full` (bad continuation `"\xc2\x41"`, surrogate `"\xed\xa0\x80"`, overlong `"\xe0\x80\x80"`) | `NULL`; code `5`, text `"unable to decode byte 0xc2"` / `0xed` / `0xe0` | [x] |
| 195 | `json_loads` | leading UTF-8 BOM `"\xef\xbb\xbf[]"` without `JSON_DECODE_ANY` — BOM is a valid UTF-8 char but not a token | `NULL`; code `8`, text `"'[' or '{' expected near '<U+FEFF>'"` | [x] |
| 196 | `json_loadb` | `buflen` shorter than the value (`json_loadb("[1]", 2, 0, &e)`) | `NULL`; code `6`, text `"']' expected near end of file"` | [x] |
| 197 | `json_loads` | `lex_init` fails because `strbuffer_init` OOMs | `NULL`; **error struct is not filled in** (only `text[0]` was zeroed by `jsonp_error_init`, so `json_error_code()` reads uninitialised `text[159]`) | [x] |
| 198 | `json_dumps` | `strbuffer_init(&strbuff)` fails (OOM) | `NULL` | [x] |
| 199 | `json_dumps` | `json_dump_callback` returns non-zero for any reason below | `NULL` | [x] |
| 200 | `json_dump_callback` | `json` is `NULL` and `JSON_ENCODE_ANY` is not set | `-1` (before `hashtable_init`) | [x] |
| 201 | `json_dump_callback` | `json` is a scalar (integer/real/string/bool/null) and `JSON_ENCODE_ANY` is not set | `-1` | [x] |
| 202 | `json_dump_callback` | `hashtable_init(&parents_set)` fails (OOM) | `-1` | [x] |
| 203 | `json_dump_callback` | `json == NULL` with `JSON_ENCODE_ANY` set (`do_dump` `!json`) | `-1` | [x] |
| 204 | `json_dump_callback` | user `callback` returns non-zero on any chunk | `-1` | [x] |
| 205 | `json_dumps` / `json_dump_callback` | circular reference through arrays (`a=[b]`, `b=[a]`) — `jsonp_loop_check` inside the `JSON_ARRAY` arm | `NULL` / `-1` | [x] |
| 206 | `json_dumps` / `json_dump_callback` | circular reference through objects (`a={"b":b}`, `b={"a":a}`) — `jsonp_loop_check` inside the `JSON_OBJECT` arm (also with `JSON_SORT_KEYS`) | `NULL` / `-1` | [x] |
| 207 | `json_dumps` | string value contains invalid UTF-8 (built via `json_string_nocheck("\xff")`) — `utf8_iterate` returns `NULL` in `dump_string` | `NULL` | [x] |
| 208 | `json_dumps` | `jsonp_dtostr` returns `-1` for a `JSON_REAL`: `JSON_REAL_PRECISION(n)` with `n` in `22..24` for `json_real(0.1)` (formatted length + 3 exceeds `MAX_REAL_STR_LENGTH` 25) | `NULL` | [x] |
| 209 | `json_dumps` | `jsonp_dtostr` returns `-1` because `dtoa_r` cannot fit: `JSON_REAL_PRECISION(n)` with `n >= 25` (`blen 25 <= ndigits`) | `NULL` | [x] |
| 210 | `json_dumps` | `JSON_SORT_KEYS` and `jsonp_malloc(size * sizeof(struct key_len))` fails | `NULL` | [x] |
| 211 | `json_dumps` | `dump_indent` chunk write fails (callback error on `"\n"` or the whitespace run) | `NULL` / `-1` | [x] |
| 212 | `json_dumpb` | `json_dump_callback` fails for any reason above (e.g. scalar without `JSON_ENCODE_ANY`, invalid UTF-8 string) | `0` (**not** `-1`; indistinguishable from an empty dump) | [x] |
| 213 | `json_dumpb` | `size` smaller than the required output (`json_dumpb(v, NULL, 0, JSON_ENCODE_ANY)`) | not an error: returns the total required byte count (e.g. `5` for `12345`), nothing written | [x] |
| 214 | `json_dumpf` | `fwrite(buffer, size, 1, dest) != 1` (e.g. `FILE*` opened read-only) | `-1` | [x] |
| 215 | `json_dumpfd` | `write(*dest, buffer, size)` returns `!= size` (e.g. `output == -1`) | `-1` | [x] |
| 216 | `json_dump_file` | `fopen(path,"w")` fails (e.g. `"/nonexistent/x.json"`) | `-1` | [x] |
| 217 | `json_dump_file` | inner `json_dumpf` fails | `-1` (file still closed) | [x] |
| 218 | `json_dump_file` | `fclose(output) != 0` (deferred write error, ENOSPC) | `-1` even if the dump itself succeeded | [x] |
| 219 | `json_dumps` | `snprintf` of a `JSON_INTEGER` returns `< 0` or `>= MAX_INTEGER_STR_LENGTH` (25) | `NULL` | [-] unreachable: `%lld` of a `json_int_t` is at most 20 chars, always < `MAX_INTEGER_STR_LENGTH` 25, and glibc `snprintf` cannot fail for an integer conversion, so neither disjunct of the guard can ever be true (documented by a passing invariant test in a12_errors_dump.rs) |
| 220 | `json_dumps` | `do_dump` `default:` arm — `json->type` outside `JSON_OBJECT..JSON_NULL` | `NULL` / `-1` | [x] |
| 221 | `json_vpack_ex` / `json_pack_ex` / `json_pack` | `fmt == NULL` | `NULL`; code `4` `json_error_invalid_argument`, text `"NULL or empty format string"`, source `"<format>"`, line `-1`, column `-1`, position `0` | [x] |
| 222 | `json_vpack_ex` / `json_pack_ex` / `json_pack` | `*fmt == '\0'` (empty format) | `NULL`; code `4`, text `"NULL or empty format string"` | [x] |
| 223 | `json_vpack_ex` | tokens remain after the first complete value (`"[]]"`, `"{s:i}x"`, `"ii"`, `"s#?"`) | `NULL`; code `9` `json_error_invalid_format`, text `"Garbage after format string"`; the built value is decref'd | [x] |
| 224 | `json_vpack_ex` | unrecognised format character in `pack` `default:` (`"q"`, `"x"`, `"}"`, `"]"`) | `NULL`; code `9`, text `"Unexpected format character 'q'"`, source `"<format>"`, line 1, column 1, position 1 | [x] |
| 225 | `json_vpack_ex` | `'{'` reached end of format before `'}'` (`"{"`, `"{s:i"`) | `NULL`; code `9`, text `"Unexpected end of format string"` | [x] |
| 226 | `json_vpack_ex` | object key position is not `'s'` (`"{i:i}"`) | `NULL`; code `9`, text `"Expected format 's', got 'i'"`, column 2 | [x] |
| 227 | `json_vpack_ex` | object key argument is `NULL` (`json_pack("{s:i}", NULL, 2)`) | `NULL`; code `12` `json_error_null_value`, text `"NULL object key"`, source `"<args>"` | [x] |
| 228 | `json_vpack_ex` | object key argument is not valid UTF-8 (`json_pack("{s:i}", "\xff", 2)`, or `"{s#:i}"` with `"\xff",1`) | `NULL`; code `5` `json_error_invalid_utf8`, text `"Invalid UTF-8 object key"`, source `"<args>"` | [x] |
| 229 | `json_vpack_ex` | object value packs to `NULL` and the value token is not `'*'` (`json_pack("{s:s}","k",NULL)`) | `NULL`; code `12`, text `"NULL string"` (the *first* error set wins; `jsonp_error_vset` ignores the later `"NULL object value"`) | [x] |
| 230 | `json_vpack_ex` | `json_object_setn_new_nocheck` fails while packing an object (OOM) | `NULL`; code `1` `json_error_out_of_memory`, text `"Unable to add key \"<key>\""` | [x] |
| 231 | `json_vpack_ex` | `'['` reached end of format before `']'` (`"["`, `"[i"`) | `NULL`; code `9`, text `"Unexpected end of format string"` | [x] |
| 232 | `json_vpack_ex` | array element packs to `NULL` and the value token is not `'*'` (`json_pack("[s]", NULL)`) | `NULL`; code `12`, text `"NULL string"` | [x] |
| 233 | `json_vpack_ex` | `json_array_append_new` fails while packing an array (OOM) | `NULL`; code `1`, text `"Unable to append to array"` | [x] |
| 234 | `json_vpack_ex` | `'s'` with a `NULL` argument and no `?`/`*` modifier (`json_pack("s", NULL)`) | `NULL`; code `12`, text `"NULL string"`, source `"<args>"`, column 1 | [x] |
| 235 | `json_vpack_ex` | `'s'` with a non-UTF-8 argument (`json_pack("s", "\xff")`, `json_pack("s#", "\xff\xfe", 2)`) | `NULL`; code `5`, text `"Invalid UTF-8 string"`, source `"<args>"` | [x] |
| 236 | `json_vpack_ex` | `'s?'` with a `NULL` argument | **not an error**: returns `json_null()` in that slot, whole pack succeeds | [x] |
| 237 | `json_vpack_ex` | `'s*'` with a `NULL` argument at top level (`json_pack("s*", NULL)`) | `NULL` with **no error recorded** (`text[0] == '\0'`, `json_error_code` reads `0`) | [x] |
| 238 | `json_vpack_ex` | `'?'` or `'*'` combined with `'#'`, `'%'` or `'+'` (`"s*+"`, `"s?+"`) | `NULL`; code `9`, text `"Cannot use '+' on optional strings"`, source `"<format>"` | [x] |
| 239 | `json_vpack_ex` | `strbuffer_init` fails in the `'#'`/`'%'`/`'+'` concatenation path (OOM) | `NULL`; code `1`, text `"Out of memory"`, source `"<internal>"` | [x] |
| 240 | `json_vpack_ex` | any `NULL` argument in the `'+'` concatenation chain (`json_pack("s++","a",NULL,"c")`) | `NULL`; code `12`, text `"NULL string"`, source `"<args>"`, column 2 | [x] |
| 241 | `json_vpack_ex` | `'#'`/`'%'` with a `NULL` string pointer (`json_pack("s#", NULL, 3)`) | `NULL`; code `12`, text `"NULL string"` | [x] |
| 242 | `json_vpack_ex` | `strbuffer_append_bytes` fails during concatenation (OOM / length overflow) | `NULL`; code `1`, text `"Out of memory"`, source `"<internal>"` | [x] |
| 243 | `json_vpack_ex` | concatenated result is not valid UTF-8 | `NULL`; code `5`, text `"Invalid UTF-8 string"`, source `"<args>"` | [x] |
| 244 | `json_vpack_ex` | `'O'` with a `NULL` `json_t*` (`json_pack("O", NULL)`) | `NULL`; code `12`, text `"NULL object"`, source `"<args>"` | [x] |
| 245 | `json_vpack_ex` | `'o'` with a `NULL` `json_t*` and no `?`/`*` (`json_pack("o", NULL)`) | `NULL`; code `12`, text `"NULL object"` | [x] |
| 246 | `json_vpack_ex` | `'o*'`/`'O*'` with `NULL` at top level (`json_pack("o*", NULL)`) | `NULL` with **no error recorded** (code reads `0`); inside `[...]` or `{...}` the slot is simply omitted and the pack succeeds | [x] |
| 247 | `json_vpack_ex` | `'i'`/`'I'` and `json_integer` OOMs | `NULL`; code `1`, text `"Out of memory"`, source `"<internal>"` | [x] |
| 248 | `json_vpack_ex` | `'f'` and `json_real(0.0)` OOMs | `NULL`; code `1`, text `"Out of memory"` | [x] |
| 249 | `json_vpack_ex` | `'f'` with a NaN or ±Inf double (`json_pack("f", NAN)`, `json_pack("f", INFINITY)`) — `json_real_set` fails | `NULL`; code `15` `json_error_numeric_overflow`, text `"Invalid floating point value"`, source `"<args>"` | [x] |
| 250 | `json_vpack_ex` | format ends right after a key with no value (`"{s"`) — `pack` sees `token == '\0'` | `NULL`; code `9`, text `"Unexpected format character ''"` | [x] |
| 251 | `json_vunpack_ex` / `json_unpack_ex` / `json_unpack` | `root == NULL` | `-1`; code `12` `json_error_null_value`, text `"NULL root value"`, source `"<root>"`, line `-1`, column `-1` | [x] |
| 252 | `json_vunpack_ex` | `fmt == NULL` | `-1`; code `4`, text `"NULL or empty format string"`, source `"<format>"` | [x] |
| 253 | `json_vunpack_ex` | `*fmt == '\0'` | `-1`; code `4`, text `"NULL or empty format string"` | [x] |
| 254 | `json_vunpack_ex` | tokens remain after the format is consumed (`"{}}"`) | `-1`; code `9`, text `"Garbage after format string"` | [x] |
| 255 | `json_vunpack_ex` | unrecognised format character in `unpack` `default:` (`"q"`) | `-1`; code `9`, text `"Unexpected format character 'q'"`, source `"<format>"` | [x] |
| 256 | `json_vunpack_ex` | `'s'` and `root` is not a string (`json_unpack(obj, "{s:s}", "a", &p)` where `obj["a"]` is an integer) | `-1`; code `10` `json_error_wrong_type`, text `"Expected string, got integer"`, source `"<validation>"` | [x] |
| 257 | `json_vunpack_ex` | `'s'` and the `const char**` target is `NULL` (and `JSON_VALIDATE_ONLY` not set) | `-1`; code `12`, text `"NULL string argument"`, source `"<args>"` | [x] |
| 258 | `json_vunpack_ex` | `'s%'` and the `size_t*` length target is `NULL` | `-1`; code `12`, text `"NULL string length argument"` | [x] |
| 259 | `json_vunpack_ex` | `'i'` and `root` is not `JSON_INTEGER` | `-1`; code `10`, text `"Expected integer, got <type>"` | [x] |
| 260 | `json_vunpack_ex` | `'I'` and `root` is not `JSON_INTEGER` | `-1`; code `10`, text `"Expected integer, got <type>"` | [x] |
| 261 | `json_vunpack_ex` | `'b'` and `root` is neither `JSON_TRUE` nor `JSON_FALSE` | `-1`; code `10`, text `"Expected true or false, got integer"` | [x] |
| 262 | `json_vunpack_ex` | `'f'` and `root` is not `JSON_REAL` (an integer is rejected — use `'F'`) | `-1`; code `10`, text `"Expected real, got integer"` | [x] |
| 263 | `json_vunpack_ex` | `'F'` and `root` is neither integer nor real | `-1`; code `10`, text `"Expected real or integer, got <type>"` | [x] |
| 264 | `json_vunpack_ex` | `'n'` and `root` is not `JSON_NULL` | `-1`; code `10`, text `"Expected null, got integer"` | [x] |
| 265 | `json_vunpack_ex` | `'{'` and `hashtable_init(&key_set)` fails (OOM) | `-1`; code `1`, text `"Out of memory"`, source `"<internal>"` | [x] |
| 266 | `json_vunpack_ex` | `'{'` and `root` is not an object (`json_unpack(array, "{s:i}", "a", &i)`) | `-1`; code `10`, text `"Expected object, got array"`, source `"<validation>"` | [x] |
| 267 | `json_vunpack_ex` | a token follows `'!'` or `'*'` inside `{}` instead of `'}'` (`"{s:i!s:s}"`, `"{*s:i}"`) | `-1`; code `9`, text `"Expected '}' after '!', got 's'"` (or `"after '*'"`) | [x] |
| 268 | `json_vunpack_ex` | `'{'` never closed (`"{s:i"`) | `-1`; code `9`, text `"Unexpected end of format string"` | [x] |
| 269 | `json_vunpack_ex` | object key position is not `'s'`/`'!'`/`'*'`/`'}'` (`"{i:i}"`) | `-1`; code `9`, text `"Expected format 's', got 'i'"` | [x] |
| 270 | `json_vunpack_ex` | object key argument is `NULL` (`json_unpack(obj, "{s:i}", NULL, &i)`) | `-1`; code `12`, text `"NULL object key"`, source `"<args>"` | [x] |
| 271 | `json_vunpack_ex` | key absent from `root` and not marked `'?'` (`json_unpack(obj, "{s:i}", "zz", &i)`) | `-1`; code `16` `json_error_item_not_found`, text `"Object item not found: zz"`, source `"<validation>"` | [x] |
| 272 | `json_vunpack_ex` | trailing `'!'` in `{}` (or `JSON_STRICT`) and some object keys were never unpacked (`json_unpack_ex(obj, &e, 0, "{!}")`) | `-1`; code `7` `json_error_end_of_input_expected`, text `"2 object item(s) left unpacked: a, b"`, source `"<validation>"` | [x] |
| 273 | `json_vunpack_ex` | `JSON_STRICT` plus an optional `'?'` key was used, forcing the full key sweep even when counts match | `-1`; code `7`, text `"<n> object item(s) left unpacked: <comma-separated keys>"` | [x] |
| 274 | `json_vunpack_ex` | strict object check where `strbuffer_init`/`strbuffer_append_bytes` for the key list fails | `-1`; code `7`, text `"<n> object item(s) left unpacked: <unknown>"` | [x] |
| 275 | `json_vunpack_ex` | `'['` and `root` is not an array (`json_unpack(obj, "[i]", &i)`) | `-1`; code `10`, text `"Expected array, got object"`, source `"<validation>"` | [x] |
| 276 | `json_vunpack_ex` | a token follows `'!'` or `'*'` inside `[]` instead of `']'` (`"[i!i]"`) | `-1`; code `9`, text `"Expected ']' after '!', got 'i'"` | [x] |
| 277 | `json_vunpack_ex` | `'['` never closed (`"[i"`) | `-1`; code `9`, text `"Unexpected end of format string"` | [x] |
| 278 | `json_vunpack_ex` | inside `[]`, format character is not in `"{[siIbfFOon"` and not `'!'`/`'*'`/`']'` (`"[q]"`) | `-1`; code `9`, text `"Unexpected format character 'q'"` | [x] |
| 279 | `json_vunpack_ex` | array index past the end (`json_unpack(json_pack("[i,i]",1,2), "[iii]", &a,&b,&c)`) — `json_array_get` returns `NULL` | `-1`; code `17` `json_error_index_out_of_range`, text `"Array index 2 out of range"`, source `"<validation>"` | [x] |
| 280 | `json_vunpack_ex` | trailing `'!'` in `[]` (or `JSON_STRICT`) and `i != json_array_size(root)` (`json_unpack_ex(arr,&e,0,"[!]")`) | `-1`; code `7`, text `"2 array item(s) left unpacked"`, source `"<validation>"` | [x] |
| 281 | `jsonp_error_init` | `error == NULL` | no-op, no crash || [x] |
| 282 | `jsonp_error_init` | `source == NULL` (as used by `json_load_file(NULL,...)`) | `error->source[0] = '\0'` (empty source string) || [x] |
| 283 | `jsonp_error_set_source` | `error == NULL` or `source == NULL` | no-op || [x] |
| 284 | `jsonp_error_set_source` | `strlen(source) >= JSON_ERROR_SOURCE_LENGTH` (80), e.g. 199 `'x'` | source is truncated to 79 chars, prefixed with `"..."` and keeping the tail || [x] |
| 285 | `jsonp_error_vset` / `jsonp_error_set` | `error == NULL` | no-op || [x] |
| 286 | `jsonp_error_vset` / `jsonp_error_set` | `error->text[0] != '\0'` (an error was already recorded) | silently returns; the *first* code/text/line/column/position are preserved (verified: first `wrong_type`/`"first"` survives a later `invalid_utf8`/`"second"`) || [x] |
| 287 | `jsonp_error_vset` | message longer than `JSON_ERROR_TEXT_LENGTH - 2` (399 `'y'`) | text truncated to 158 bytes; `text[158] = '\0'`, `text[159] = code` || [x] |
| 288 | `strbuffer_init` | `jsonp_malloc(16)` fails | `-1`; `strbuff->value` left `NULL` || [x] |
| 289 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` (`strbuffer_append_bytes(&sb, "x", (size_t)-1)`) | `-1`, buffer unchanged || [x] |
| 290 | `strbuffer_append_bytes` | `strbuff->length > SIZE_MAX - 1 - size` (length overflow) | `-1` || [x] |
| 291 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX / 2` (doubling would overflow) | `-1` || [x] |
| 292 | `strbuffer_append_bytes` | `jsonp_realloc(value, size, new_size)` fails | `-1`, buffer unchanged || [x] |
| 293 | `strbuffer_pop` | `strbuff->length == 0` | returns `'\0'` (0), length stays 0 || [x] |
| 294 | `strbuffer_value` | called after `strbuffer_steal_value` or on a failed `strbuffer_init` | `NULL` || [x] |
| 295 | `hashtable_init` | `jsonp_malloc(8 * sizeof(bucket_t))` fails | `-1` || [x] |
| 296 | `hashtable_set` | `hashtable_do_rehash` fails (`jsonp_malloc` of the new bucket array) when `size >= hashsize(order)` | `-1`; the old bucket array is still live || [x] |
| 297 | `hashtable_set` | `init_pair`: `key_len >= (size_t)-1 - offsetof(pair_t, key)` | `-1` (guard only; see Notes — `hash_str` reads the key first and faults) || [-] unreachable: dead code — `hash_str(key, key_len)` reads the key before `init_pair`'s guard, so a key_len large enough to trip it faults in `hashlittle` first. Guard neighbourhood verified in a15. |
| 298 | `hashtable_set` | `init_pair`: `jsonp_malloc(offsetof(pair_t,key) + key_len + 1)` fails | `-1` || [x] |
| 299 | `hashtable_get` | key not present | `NULL` || [x] |
| 300 | `hashtable_del` | key not present | `-1` || [x] |
| 301 | `hashtable_iter` / `hashtable_iter_next` | ordered list is empty / iterator is at the last pair | `NULL` || [x] |
| 302 | `hashtable_iter_at` | key not present | `NULL` || [x] |
| 303 | `jsonp_malloc` | `size == 0` | `NULL` **without** calling `do_malloc` || [x] |
| 304 | `jsonp_malloc` | `do_malloc(size)` returns `NULL` | `NULL` || [x] |
| 305 | `jsonp_free` | `ptr == NULL` | no-op, `do_free` not called || [x] |
| 306 | `jsonp_realloc` | `do_realloc == NULL` (installed via `json_set_alloc_funcs`) and `newSize == 0` | frees `ptr` if non-`NULL`, returns `NULL` || [x] |
| 307 | `jsonp_realloc` | `do_realloc == NULL` and `do_malloc(newSize)` returns `NULL` | `NULL`; the original `ptr` is **not** freed || [x] |
| 308 | `jsonp_strndup` | `jsonp_malloc(len + 1)` fails | `NULL` || [x] |
| 309 | `json_get_alloc_funcs` | `malloc_fn == NULL` and/or `free_fn == NULL` | those out-params are skipped; no crash || [x] |
| 310 | `json_get_alloc_funcs2` | any of `malloc_fn`/`realloc_fn`/`free_fn` is `NULL` | those out-params are skipped; no crash || [x] |
| 311 | `json_set_alloc_funcs` | called with `NULL` function pointers | accepted with no validation; every later allocation calls a `NULL` pointer → crash || [-] partially unreachable: the crash half is UB. The observable half (NULL hooks stored verbatim, realloc slot nulled) IS verified in a15. |
| 312 | `jsonp_strtod` | overflow: `(value == HUGE_VAL \|\| value == -HUGE_VAL) && errno == ERANGE` (strbuffer holds `"1e999"`) | `-1`, `*out` untouched || [x] |
| 313 | `jsonp_strtod` | underflow (strbuffer holds `"1e-999"`, `strtod` sets `ERANGE` but returns ~0) | **not** an error: returns `0`, `*out = 0.0` || [x] |
| 314 | `jsonp_strtod` | strbuffer content is not a complete `double` literal (e.g. `"abc"`, `"1x"`) — `assert(end == value + length)` | `assert` fires → `SIGABRT` (asserts are enabled: `CMAKE_BUILD_TYPE` is empty, so no `-DNDEBUG`) || [-] unreachable: live `assert(end == value + length)` -> SIGABRT. The guard (every caller passes a fully-consumed buffer) is verified in a15 over ~4000 lexer-accepted literals. |
| 315 | `jsonp_dtostr` | `dtoa_r(value, mode, precision, ..., buffer, size)` returns `NULL` because `size <= ndigits`; with `size == 25` this means `precision >= 25` (mode 2) | `-1` || [x] |
| 316 | `jsonp_dtostr` | formatted output does not fit: `3 + (vdigits_end - vdigits_start) + (use_exp ? 5 : 0) > size`; e.g. `jsonp_dtostr(buf, 25, 0.1, 22)`, `(buf,25,1e300,21)`, `(buf,25,1e-300,21)` | `-1`, `buffer` contents unspecified || [x] |
| 317 | `jsonp_dtostr` | `size` too small for even a short value (`jsonp_dtostr(buf, 1, 1.0, 0)`) | `-1` || [x] |
| 318 | `dtoa_r` | `buf != NULL` and `blen <= (size_t)i` where `i` is the needed digit count (18 for mode 0/1, `ndigits` for mode 2/4): `dtoa_r(0.1, 0, 0, &dp, &sg, &rve, buf, 4)`, `dtoa_r(0.1, 2, 25, ..., buf, 25)` | returns `NULL`; `*rve` is set to `NULL + i` (undefined pointer arithmetic), `*decpt` left from the earlier stage || [x] |
| 319 | `dtoa_r` | `dd` is ±0.0 and `blen <= 1` (`nrv_alloc("0", buf, blen, rve, 1)`) | returns `NULL`, `*decpt == 1`, `*sign` set || [x] |
| 320 | `dtoa_r` | `dd` is NaN and `blen <= 3` | returns `NULL`, `*decpt == 9999` || [x] |
| 321 | `dtoa_r` | `dd` is ±Infinity and `blen <= 8` | returns `NULL`, `*decpt == 9999`, `*sign == 1` for `-inf` || [x] |
| 322 | `dtoa_r` | `mode < 0` or `mode > 9` (`dtoa_r(1.5, -5, ...)`, `dtoa_r(1.5, 99, ...)`) | **not** an error: `mode` is silently clamped to `0`, output is the shortest round-trip form || [x] |
| 323 | `dtoa_r` / `dtoa` | `Balloc` → `MALLOC` returns `NULL` (OOM) | `rv->sign = rv->wds = 0` dereferences `NULL` → segfault (no allocation check in `Balloc`) || [-] unreachable: `Balloc` does not check MALLOC, so OOM is a NULL deref (UB in the C). |
| 324 | `dtoa` | any failure — it always calls `dtoa_r(..., buf = 0, blen = 0)`, so it allocates and the `blen`/`nrv_alloc` short-buffer paths cannot trigger | only the OOM crash of row 323 is reachable || [-] unreachable: `dtoa` always allocates (`dtoa_r(..., buf=0, blen=0)`), so only row 323's crash remains. Verified in a15 that `dtoa` never returns NULL for any input. |
| 325 | `freedtoa` | `s == NULL` | dereferences `((int*)s - 1)` to recover `b->k` → segfault (no `NULL` check) || [-] unreachable: `freedtoa(NULL)` derefs `((int*)s - 1)` (UB in the C). Freelist recycling correctness verified in a15 over 20000 alloc/free cycles. |
| 326 | `gethex` | binary exponent `e > emax` (hex literal like `"0x1p+99999"`) → `ovfl`/`ovfl1` | `errno = ERANGE`; `*rvp` set to `+Infinity` (`word0 = Exp_mask`, `word1 = 0`); function returns `void` || [x] |
| 327 | `gethex` | `big && !esign` (exponent digits overflow 32 bits, positive) with `rounding == Round_near` | `errno = ERANGE`; `*rvp` = ±Infinity via `ovfl1` || [x] |
| 328 | `gethex` | `big && !esign` with `rounding == Round_up`/`Round_down` on the sign-matching side | no `errno`; `*rvp` set to `DBL_MAX` (`Big0`/`Big1`) via `ret_big` || [x] |
| 329 | `gethex` | `big && esign` (huge negative exponent) → `retz` | `errno = ERANGE`; `*rvp = 0.0` || [x] |
| 330 | `gethex` | `e < emin` and `n >= nbits` (underflow past the denormal range) → `retz` | `errno = ERANGE`; `*rvp = 0.0` || [x] |
| 331 | `gethex` | underflow with `rounding` biased toward the value (`Round_up` and `!sign`, `Round_down` and `sign`, or `Round_near` at the halfway point) → `ret_tiny` | `errno = ERANGE`; `*rvp` = smallest denormal (`word0 = 0`, `word1 = 1`) || [x] |
| 332 | `gethex` | no significant hex digits (`zret`, e.g. `"0x0"`, `"0x.0"`) → `retz1` | `*rvp = 0.0`, `errno` **not** set || [x] |
| 333 | `gethex` | no hex digit at all after `0x` (`!havedig`) | `*sp` is rewound to `s0 - 1` so the caller sees the literal as unconsumed || [x] |
| 334 | `gethex` | rounding-up carry pushes `++e > Emax` → `goto ovfl` | `errno = ERANGE`; `*rvp` = +Infinity || [x] |
| 335 | `strtod__unused` | input is not a number (`"abc"`) | returns `0.0`, `*se` set back to the input start (dead code: never called by the library; `jsonp_strtod` uses the libc `strtod`) || [x] |
| 336 | `json_object_seed` | `generate_seed()` computes `0` | forced to `1` (a zero seed would re-trigger auto-seeding forever) | [-] not forceable in-process: `generate_seed()`'s inputs are 4 bytes of `/dev/urandom` or `gettimeofday() ^ getpid()`, none of which can be steered so their XOR comes out `0`. The only observable consequence of that line — the installed seed is never `0` — IS asserted (a13 `seed_fallback_without_urandom_subprocess`). |
| 337 | `json_object_seed` | `seed_from_urandom`: `open("/dev/urandom", O_RDONLY)` returns `-1`, or the read is short | returns `1` → falls back to `seed_from_timestamp_and_pid` | [x] |
| 338 | `jansson_version_cmp` | version mismatch (`jansson_version_cmp(3,0,0)`) | non-zero difference (`2 - 3 == -1`); not an error path, no error struct || [x] |
| 339 | `json_loadf` (via `json_load_file`) | `fgetc` returns `EOF` mid-value (truncated file) | `NULL`; code `6`, text `"']' expected near end of file"` (or the analogous `'}' expected`) | [x] |
| 340 | `json_loadfd` | `read(fd, &c, 1)` returns `!= 1` (closed/empty fd) — `fd_get_func` maps to `EOF` | `NULL`; code `6`, text `"'[' or '{' expected near end of file"` | [x] |
| 341 | `stream_get` (all `json_load*`) | multi-byte sequence truncated at end of input: `stream->get` returns `EOF` (`-1`) for a continuation byte, so `utf8_check_full` fails | `NULL`; code `5` `json_error_invalid_utf8`, text `"unable to decode byte 0x<lead>"` | [x] |
| 342 | `decode_unicode_escape` | a character in `str[1..4]` is not a hex digit | `-1` → caller emits `json_error_invalid_syntax` `"invalid Unicode escape '%.6s'"` (see Notes: unreachable, the first lexer pass already validated the hex digits) || [-] unreachable: the first `lex_scan_string` pass already validated the four hex digits. Guard verified exhaustively in a15. |
| 343 | `lex_scan_string` (2nd pass) | `utf8_encode(value, t, &length)` returns non-zero | `assert(0)` → `SIGABRT` (see Notes: unreachable, `value` is range- and surrogate-checked first) | [-] unreachable: assert(0) after utf8_encode; value is range- and surrogate-checked first (SIGABRT, not testable in-process) |
| 344 | `lex_scan_string` (2nd pass) | escape character reaches the `switch` `default:` | `assert(0)` → `SIGABRT` (see Notes: unreachable) | [-] unreachable: assert(0) in the escape switch default; the first pass already rejected every illegal escape |
| 345 | `stream_get` | `utf8_check_first(c)` returns `1` for `0x80 <= c <= 0xFF` | `assert(count >= 2)` → `SIGABRT` (see Notes: unreachable) | [-] unreachable: assert(count >= 2); utf8_check_first returns 0 (handled above) or 2..4 for bytes >= 0x80 |
| 346 | `stream_unget` | `stream->buffer_pos == 0`, or `stream->buffer[buffer_pos] != c` | `assert` → `SIGABRT` (see Notes: internal lexer invariants) || [-] unreachable: `stream_unget` invariant — every unget is paired with a preceding get of the same byte. Verified in a15 over 6000 randomized token boundaries. |
| 347 | `lex_unget_unsave` | `strbuffer_pop` does not return the ungotten char | `assert(c == d)` → `SIGABRT` (see Notes: internal invariant) | [-] unreachable: assert(c == d); every unget is paired with a preceding get of the same byte |
| 348 | `lex_scan_number` | `strtoll` did not consume the whole saved literal | `assert(end == saved_text + lex->saved_text.length)` → `SIGABRT` (see Notes: unreachable) | [-] unreachable: assert(end == saved_text + length); the lexer validated the literal char-by-char |
| 349 | `decode_unicode_escape` | `str[0] != 'u'` | `assert(str[0] == 'u')` → `SIGABRT` (see Notes: unreachable) || [-] unreachable: `assert(str[0] == 'u')`; the caller only reaches it via a validated `\u` escape. Guard verified in a15. |
| 350 | `json_dumps` with `JSON_SORT_KEYS` | number of iterated keys `!= json_object_size(json)` | `assert(i == size)` → `SIGABRT` (see Notes: unreachable) | [-] live `assert()` → SIGABRT, and unreachable: the key array is filled straight from the object's own iterator (the invariant is asserted through the FFI instead) |
| 351 | `json_dumps` with `JSON_SORT_KEYS` | `json_object_getn(json, key->key, key->len)` returns `NULL` for a key just harvested from the iterator | `assert(value)` → `SIGABRT` (see Notes: unreachable) | [-] live `assert()` → SIGABRT, and unreachable: every key was just harvested from the same object (the invariant is asserted through the FFI instead) |
| 352 | `json_dumps` with `JSON_SORT_KEYS` | `dump_string(key->key, ...)` fails (invalid UTF-8 key created via `json_object_set_nocheck`) | return value is **ignored**; the malformed key is emitted and only the following `dump(separator)`/`do_dump` can report `-1` | [x] |
| 353 | `json_dumps` without `JSON_SORT_KEYS` | `dump_string(key, key_len, ...)` fails for an invalid-UTF-8 key | return value is **ignored** (same swallowed-error bug as row 352) | [x] |
| 354 | `json_dumps` | `jsonp_realloc(result, strbuff.size, strbuff.length + 1)` shrink fails | **not** an error: the original (larger) buffer is returned | [x] |
| 355 | `json_loads` | `JSON_INDENT`/`JSON_COMPACT`/encoder flags passed to a decoder (or `JSON_DECODE_*` passed to an encoder) | silently ignored — there is no flag validation anywhere in the library | [x] |
| 356 | `json_dumps` | `JSON_INDENT(n)` with `n > JSON_MAX_INDENT` (31) | silently masked to `n & 0x1F`; `n == 32` behaves like `JSON_COMPACT`-less zero indent | [x] |

## Notes — error paths that are not practically testable through the FFI boundary

* **`jsonp_malloc` / `jsonp_realloc` / `jsonp_strndup` failures** (rows 12, 32, 58, 63, 69, 72, 73, 83, 89, 90, 95, 101–104, 118, 119, 121, 125, 162–164, 169, 172, 175, 185, 197, 198, 202, 210, 230, 233, 239, 242, 247, 248, 265, 288, 292, 295, 296, 298, 304, 307, 308) are only reachable when the allocator returns `NULL`. From an FFI test they can be reached only by installing a failing allocator with `json_set_alloc_funcs`/`json_set_alloc_funcs2` (which is itself an exported symbol, so this *is* testable, but requires global process state and is not reachable with plain arguments). `hashtable_do_rehash` additionally requires ≥ 8 keys before the rehash is attempted at all.
* **`hashtable_set` key-length overflow guard (row 297)** is dead in practice: `hashtable_set` computes `hash_str(key, key_len)` *before* calling `init_pair`, so a `key_len` near `SIZE_MAX` segfaults inside `hashlittle` first (empirically confirmed: `hashtable_set(&h, "k", (size_t)-1, json_null())` → `SIGSEGV`). The same applies to reaching it through `json_object_setn_new_nocheck`.
* **`Balloc` OOM (rows 323, 324)** — `dtoa.c`'s `Balloc` never checks the `MALLOC` result, so an allocation failure is a `NULL` dereference rather than an error return. Not testable without an allocator hook, and the observable behaviour is a crash, not a value.
* **`freedtoa(NULL)` (row 325)** crashes rather than returning; it is "testable" only as a crash, so it is not a useful parity assertion.
* **`gethex` (rows 326–334)** is an exported symbol but is only *called* from `strtod__unused`, which the library never invokes (`jsonp_strtod` uses the libc `strtod`). Calling `gethex` directly through the FFI additionally requires fabricating the private `U` union (a `double`-sized union) and a `Bigint` freelist state, and it returns `void` — the outcome is only observable via `errno` and the mutated `*rvp`. Its error paths therefore cannot be exercised through any public `json_*` entry point.
* **`strtod__unused` (row 335)** is exported but unreachable from any public API; its name marks it as dead code.
* **`assert()` rows 343–351** guard internal invariants that the surrounding code already establishes:
  * `assert(count >= 2)` in `stream_get` — `utf8_check_first` returns `0`, `2`, `3` or `4` for bytes `>= 0x80`, and `0` is handled by the `goto out` above.
  * `assert(stream->buffer_pos > 0)` / `assert(stream->buffer[buffer_pos] == c)` in `stream_unget` and `assert(c == d)` in `lex_unget_unsave` — every `unget` is paired with a preceding `get` of the same character.
  * `assert(str[0] == 'u')` and the two `assert(0)`s in `lex_scan_string`'s second pass — the first pass already validated that every `\` is followed by a legal escape character and that every `\u` has 4 hex digits, and the decoded value is bounds- and surrogate-checked before `utf8_encode`.
  * `assert(end == saved_text + length)` in `lex_scan_number` — the lexer only hands `strtoll` a literal it just validated character-by-character.
  * `assert(i == size)` / `assert(value)` in the `JSON_SORT_KEYS` path — the key array is filled straight from the object's own iterator.
  Row 342 (`decode_unicode_escape` returning `-1` on the second pass, and its `"invalid Unicode escape '%.6s'"` message) is unreachable for the same reason: `["\uZZZZ"]` and `["\uD800\uZZZZ"]` both fail in the first pass with `"invalid escape"` instead (empirically confirmed).
  These asserts *are* compiled in (`CMAKE_BUILD_TYPE` is empty in `c_src/build/CMakeCache.txt`, so `-DNDEBUG` is absent), but no argument combination reaches them, so a Rust port may implement them as `unreachable!()`/`debug_assert!` without observable difference.
* **`jsonp_strtod`'s `assert` (row 314)** *is* reachable by calling the exported `jsonp_strtod` directly with a hand-built `strbuffer_t` holding a non-numeric string. It aborts the process, so it is testable only as a crash-expectation, not as a return value.
* **`assert(be >= -51)`, `assert(be >= 0 && be <= 4)`, `assert(eulp <= 4)` in `dtoa.c`** compile to nothing: `dtoa.c` does `#define assert(x) /*nothing*/` unless `DEBUG` is defined, and it is not. They have no observable effect at all.
* **`json_equal`'s `switch` `default:` arm (returning `0`)** is dead: `JSON_TRUE`/`JSON_FALSE`/`JSON_NULL` are singletons, so the earlier `json1 == json2` test already returned `1` for them, and every other `json_type` has an explicit case. The same applies to `json_copy`'s and `do_deep_copy`'s `default:` arms and `do_dump`'s `default:` arm (row 220) — they require a corrupted `json->type`.
* **`parse_object`'s `if (!key) return NULL;`** (after `lex_steal_string`) is unreachable: `lex_steal_string` only returns `NULL` when `lex->token != TOKEN_STRING`, and the branch above already rejected that case. Note that this path would also leak the `json_object` (no `json_decref`).
* **`json_dumpf`'s `fwrite` failure (row 214)** is awkward to trigger: writing to `/dev/full` returns success because `stdio` buffers (empirically confirmed: `json_dumpf` to `/dev/full` returned `0`). A reliable trigger is a `FILE*` opened read-only (`fopen(path, "r")`) or an already-`fclose`d stream.
* **`json_dump_file`'s `fclose` failure (row 218)** needs a full filesystem or a closed descriptor; not reachable from arguments alone.
* **Rows 197 and 237/246** document cases where the library returns a failure indicator *without* filling in the error struct. In row 197 `json_error_code()` reads uninitialised memory (`jsonp_error_init` only clears `text[0]`, never `text[159]`); a port that zeroes the whole struct will differ observably there, so tests should not assert on `json_error_code` for those paths.
