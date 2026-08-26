# ERRORS.md — Error-surface table

Derived **mechanically** from the C sources in `c_src/src` by grepping for every
`return -1`, `return NULL`, `return 0` (sentinel), `error_set(...)`,
`set_error(...)`, `jsonp_error_set(...)`, `assert(...)`, explicit range check,
null check and min/max constant.

One row per **distinct rejection**. `[x]` = a differential test in
`tests/` constructs exactly that condition, calls **both** the C `.so` and the
Rust `.so`, and asserts the *same* error code / sentinel.

Legend for "expected C result": the exact value the C returns, and where a
`json_error_t` is filled in, the `enum json_error_code` stored in
`text[JSON_ERROR_TEXT_LENGTH-1]` plus the exact `text` prefix.

---

## 1. `value.c`

Covered by **tests/t05_value.rs**.

| # | function | trigger (exact invalid input/condition) | expected C result | verified |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `jsonp_loop_check` | `%p` key already present in `parents` | `-1` | [x] |
| 2 | `json_object_size` | `json` NULL / not `JSON_OBJECT` | `0` | [x] |
| 3 | `json_object_get` | `key == NULL` | `NULL` | [x] |
| 4 | `json_object_getn` | `key == NULL` | `NULL` | [x] |
| 5 | `json_object_getn` | `json` NULL / not object | `NULL` | [x] |
| 6 | `json_object_getn` | key absent from table | `NULL` | [x] |
| 7 | `json_object_set_new_nocheck` | `key == NULL` | `-1` (+ decref value) | [x] |
| 8 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` | [x] |
| 9 | `json_object_setn_new_nocheck` | `key == NULL` | `-1` | [x] |
| 10 | `json_object_setn_new_nocheck` | `json` not object (incl. NULL) | `-1` | [x] |
| 11 | `json_object_setn_new_nocheck` | `json == value` (self-insert) | `-1` | [x] |
| 12 | `json_object_set_new` | `key == NULL` | `-1` | [x] |
| 13 | `json_object_setn_new` | `key == NULL` | `-1` | [x] |
| 14 | `json_object_setn_new` | `!utf8_check_string(key,key_len)` | `-1` | [x] |
| 15 | `json_object_del` | `key == NULL` | `-1` | [x] |
| 16 | `json_object_deln` | `key == NULL` | `-1` | [x] |
| 17 | `json_object_deln` | `json` not object | `-1` | [x] |
| 18 | `json_object_deln` | key not present (`hashtable_del` miss) | `-1` | [x] |
| 19 | `json_object_clear` | `json` not object | `-1` | [x] |
| 20 | `json_object_update` | `object` not object **or** `other` not object | `-1` | [x] |
| 21 | `json_object_update_existing` | `object`/`other` not object | `-1` | [x] |
| 22 | `json_object_update_missing` | `object`/`other` not object | `-1` | [x] |
| 23 | `do_object_update_recursive` | `object`/`other` not object | `-1` | [x] |
| 24 | `do_object_update_recursive` | `jsonp_loop_check` hit (cycle in `other`) | `-1` | [x] |
| 25 | `json_object_update_recursive` | `object`/`other` not object | `-1` | [x] |
| 26 | `json_object_iter` | `json` not object | `NULL` | [x] |
| 27 | `json_object_iter_at` | `key == NULL` | `NULL` | [x] |
| 28 | `json_object_iter_at` | `json` not object | `NULL` | [x] |
| 29 | `json_object_iter_at` | key absent | `NULL` | [x] |
| 30 | `json_object_iter_next` | `json` not object | `NULL` | [x] |
| 31 | `json_object_iter_next` | `iter == NULL` | `NULL` | [x] |
| 32 | `json_object_iter_next` | iter is last element | `NULL` | [x] |
| 33 | `json_object_iter_key` | `iter == NULL` | `NULL` | [x] |
| 34 | `json_object_iter_key_len` | `iter == NULL` | `0` | [x] |
| 35 | `json_object_iter_value` | `iter == NULL` | `NULL` | [x] |
| 36 | `json_object_iter_set_new` | `json` not object | `-1` | [x] |
| 37 | `json_object_iter_set_new` | `iter == NULL` | `-1` | [x] |
| 38 | `json_object_iter_set_new` | `value == NULL` | `-1` | [x] |
| 39 | `json_object_key_to_iter` | `key == NULL` | `NULL` | [x] |
| 40 | `json_array_size` | `json` NULL / not array | `0` | [x] |
| 41 | `json_array_get` | `json` not array | `NULL` | [x] |
| 42 | `json_array_get` | `index >= entries` (incl. `index = SIZE_MAX`) | `NULL` | [x] |
| 43 | `json_array_set_new` | `value == NULL` | `-1` | [x] |
| 44 | `json_array_set_new` | `json` not array | `-1` | [x] |
| 45 | `json_array_set_new` | `json == value` | `-1` | [x] |
| 46 | `json_array_set_new` | `index >= entries` | `-1` | [x] |
| 47 | `json_array_append_new` | `value == NULL` | `-1` | [x] |
| 48 | `json_array_append_new` | `json` not array | `-1` | [x] |
| 49 | `json_array_append_new` | `json == value` | `-1` | [x] |
| 50 | `json_array_insert_new` | `value == NULL` | `-1` | [x] |
| 51 | `json_array_insert_new` | `json` not array | `-1` | [x] |
| 52 | `json_array_insert_new` | `json == value` | `-1` | [x] |
| 53 | `json_array_insert_new` | `index > entries` (one past valid) | `-1` | [x] |
| 54 | `json_array_remove` | `json` not array | `-1` | [x] |
| 55 | `json_array_remove` | `index >= entries` | `-1` | [x] |
| 56 | `json_array_clear` | `json` not array | `-1` | [x] |
| 57 | `json_array_extend` | `json` not array | `-1` | [x] |
| 58 | `json_array_extend` | `other_json` not array | `-1` | [x] |
| 59 | `json_string_nocheck` | `value == NULL` | `NULL` | [x] |
| 60 | `json_stringn_nocheck` / `string_create` | `value == NULL` | `NULL` | [x] |
| 61 | `jsonp_stringn_nocheck_own` | `value == NULL` | `NULL` | [x] |
| 62 | `json_string` | `value == NULL` | `NULL` | [x] |
| 63 | `json_stringn` | `value == NULL` | `NULL` | [x] |
| 64 | `json_stringn` | `!utf8_check_string(value,len)` | `NULL` | [x] |
| 65 | `json_string_value` | `json` not string | `NULL` | [x] |
| 66 | `json_string_length` | `json` not string | `0` | [x] |
| 67 | `json_string_set_nocheck` | `value == NULL` | `-1` | [x] |
| 68 | `json_string_setn_nocheck` | `json` not string | `-1` | [x] |
| 69 | `json_string_setn_nocheck` | `value == NULL` | `-1` | [x] |
| 70 | `json_string_set` | `value == NULL` | `-1` | [x] |
| 71 | `json_string_setn` | `value == NULL` | `-1` | [x] |
| 72 | `json_string_setn` | `!utf8_check_string` | `-1` | [x] |
| 73 | `json_vsprintf`/`json_sprintf` | formatted result is invalid UTF-8 | `NULL` | [x] |
| 74 | `json_vsprintf`/`json_sprintf` | `length == 0` (empty result) | `json_string("")` | [x] |
| 75 | `json_integer_value` | `json` not integer | `0` | [x] |
| 76 | `json_integer_set` | `json` not integer | `-1` | [x] |
| 77 | `json_real` | `isnan(value)` | `NULL` | [x] |
| 78 | `json_real` | `isinf(value)` (both signs) | `NULL` | [x] |
| 79 | `json_real_value` | `json` not real | `0.0` | [x] |
| 80 | `json_real_set` | `json` not real | `-1` | [x] |
| 81 | `json_real_set` | `isnan(value)` | `-1` | [x] |
| 82 | `json_real_set` | `isinf(value)` | `-1` | [x] |
| 83 | `json_number_value` | `json` neither integer nor real (incl. NULL) | `0.0` | [x] |
| 84 | `json_delete` | `json == NULL` | no-op | [x] |
| 85 | `json_delete` | type is TRUE/FALSE/NULL or out-of-range | `default:` → return, no free | [x] |
| 86 | `json_equal` | `value1 == NULL` | `0` | [x] |
| 87 | `json_equal` | `value2 == NULL` | `0` | [x] |
| 88 | `json_equal` | differing `json_typeof` | `0` | [x] |
| 89 | `json_equal` | out-of-range identical `json_type` (`default:`) | `0` | [x] |
| 90 | `json_copy` | `json == NULL` | `NULL` | [x] |
| 91 | `json_copy` | out-of-range `json_type` (`default:`) | `NULL` | [x] |
| 92 | `json_deep_copy` / `do_deep_copy` | `json == NULL` | `NULL` | [x] |
| 93 | `do_deep_copy` | out-of-range `json_type` (`default:`) | `NULL` | [x] |
| 94 | `json_object_deep_copy` | cycle → `jsonp_loop_check` hit | `NULL` | [x] |
| 95 | `json_array_deep_copy` | cycle → `jsonp_loop_check` hit | `NULL` | [x] |

## 2. `dump.c`

Covered by **tests/t07_dump.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 96 | `do_dump` | `json == NULL` (via `JSON_ENCODE_ANY`) | `-1` | [x] |
| 97 | `do_dump` (`JSON_REAL`) | `jsonp_dtostr` fails: precision ≥ 18 needing > 25 bytes | `-1` | [x] |
| 98 | `do_dump` (`JSON_ARRAY`) | circular array reference (`jsonp_loop_check`) | `-1` | [x] |
| 99 | `do_dump` (`JSON_OBJECT`) | circular object reference | `-1` | [x] |
| 100 | `do_dump` | out-of-range `json_type` (`default:`) | `-1` | [x] |
| 101 | `dump_string` | `utf8_iterate` → NULL (invalid UTF-8 in string value) | `-1` | [x] |
| 102 | `dump_to_file` | `fwrite() != 1` (e.g. read-only `FILE*`) | `-1` | [x] |
| 103 | `dump_to_fd` | `write() != size` (e.g. closed/bad fd) | `-1` | [x] |
| 104 | `json_dumps` | `json_dump_callback` fails | `NULL` | [x] |
| 105 | `json_dumpb` | `json_dump_callback` fails | `0` | [x] |
| 106 | `json_dumpb` | `size` smaller than output (no write, count returned) | `buf.used` (> size) | [x] |
| 107 | `json_dumpf` | `output` FILE* not writable | `-1` | [x] |
| 108 | `json_dumpfd` | `output` fd invalid (`-1`) | `-1` | [x] |
| 109 | `json_dump_file` | `fopen(path,"w")` fails (bad path) | `-1` | [x] |
| 110 | `json_dump_callback` | `!JSON_ENCODE_ANY` and json not array/object | `-1` | [x] |
| 111 | `json_dump_callback` | `json == NULL` and `!JSON_ENCODE_ANY` | `-1` | [x] |
| 112 | `json_dump_callback` | user callback returns non-zero | `-1` | [x] |

## 3. `load.c`

Covered by **tests/t06_load.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 113 | `stream_get` | `utf8_check_first(c) == 0` → invalid lead byte | `NULL`, `json_error_invalid_utf8`, `"unable to decode byte 0x%x"` | [x] |
| 114 | `stream_get` | `utf8_check_full` fails → truncated/bad continuation | `NULL`, `json_error_invalid_utf8` | [x] |
| 115 | `lex_scan_string` | EOF inside string literal | `json_error_premature_end_of_input`, `"premature end of input"` | [x] |
| 116 | `lex_scan_string` | raw `\n` inside string | `json_error_invalid_syntax`, `"unexpected newline"` | [x] |
| 117 | `lex_scan_string` | other control char `0x00..0x1F` in string | `json_error_invalid_syntax`, `"control character 0x%x"` | [x] |
| 118 | `lex_scan_string` | `\u` followed by non-hex digit | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 119 | `lex_scan_string` | unknown escape char after `\` | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 120 | `lex_scan_string` | high surrogate `\uD800` with no following `\u` | `json_error_invalid_syntax`, `"invalid Unicode '\uD800'"` | [x] |
| 121 | `lex_scan_string` | high surrogate + 2nd escape outside `DC00..DFFF` | `json_error_invalid_syntax`, `"invalid Unicode '\uXXXX\uYYYY'"` | [x] |
| 122 | `lex_scan_string` | lone low surrogate `\uDC00..\uDFFF` | `json_error_invalid_syntax`, `"invalid Unicode '\uXXXX'"` | [x] |
| 123 | `lex_scan_number` | leading `0` followed by a digit (`01`) | TOKEN_INVALID → `json_error_invalid_syntax` `"invalid token"` | [x] |
| 124 | `lex_scan_number` | `-` not followed by a digit | `json_error_invalid_syntax` | [x] |
| 125 | `lex_scan_number` | integer with `errno==ERANGE`, `intval < 0` | `json_error_numeric_overflow`, `"too big negative integer"` | [x] |
| 126 | `lex_scan_number` | integer with `errno==ERANGE`, `intval >= 0` | `json_error_numeric_overflow`, `"too big integer"` | [x] |
| 127 | `lex_scan_number` | `.` not followed by a digit (`1.`) | `json_error_invalid_syntax` | [x] |
| 128 | `lex_scan_number` | `e`/`E` (or `e+`/`e-`) not followed by a digit | `json_error_invalid_syntax` | [x] |
| 129 | `lex_scan_number` | `jsonp_strtod` overflow (`1e999`) | `json_error_numeric_overflow`, `"real number overflow"` | [x] |
| 130 | `lex_scan` | alpha identifier that is not `true`/`false`/`null` | TOKEN_INVALID → `"invalid token"` | [x] |
| 131 | `lex_scan` | any other byte (e.g. `@`, `#`) | TOKEN_INVALID → `"invalid token"` | [x] |
| 132 | `parse_object` | token after `{` is neither string nor `}` | `json_error_invalid_syntax`, `"string or '}' expected"` | [x] |
| 133 | `parse_object` | ` ` embedded in an object **key** | `json_error_null_byte_in_key`, `"NUL byte in object key not supported"` | [x] |
| 134 | `parse_object` | `JSON_REJECT_DUPLICATES` and key repeats | `json_error_duplicate_key`, `"duplicate object key"` | [x] |
| 135 | `parse_object` | token after key is not `:` | `json_error_invalid_syntax`, `"':' expected"` | [x] |
| 136 | `parse_object` | token after last member is not `}` | `json_error_invalid_syntax`, `"'}' expected"` | [x] |
| 137 | `parse_array` | token after last element is not `]` | `json_error_invalid_syntax`, `"']' expected"` | [x] |
| 138 | `parse_value` | nesting depth > `JSON_PARSER_MAX_DEPTH` (2048) | `json_error_stack_overflow`, `"maximum parsing depth reached"` | [x] |
| 139 | `parse_value` | ` ` in string value without `JSON_ALLOW_NUL` | `json_error_null_character`, `"  is not allowed without JSON_ALLOW_NUL"` | [x] |
| 140 | `parse_value` | `TOKEN_INVALID` | `json_error_invalid_syntax`, `"invalid token"` | [x] |
| 141 | `parse_value` | `default:` — structural token in value position (`}`,`]`,`,`,`:`) | `json_error_invalid_syntax`, `"unexpected token"` | [x] |
| 142 | `parse_json` | `!JSON_DECODE_ANY` and root is not `[`/`{` | `json_error_invalid_syntax`, `"'[' or '{' expected"` | [x] |
| 143 | `parse_json` | `!JSON_DISABLE_EOF_CHECK` and trailing garbage | `json_error_end_of_input_expected`, `"end of file expected"` | [x] |
| 144 | `json_loads` | `string == NULL` | `NULL`, `json_error_invalid_argument`, `"wrong arguments"`, source `<string>` | [x] |
| 145 | `json_loadb` | `buffer == NULL` | `NULL`, `json_error_invalid_argument`, source `<buffer>` | [x] |
| 146 | `json_loadf` | `input == NULL` | `NULL`, `json_error_invalid_argument`, source `<stream>` | [x] |
| 147 | `json_loadfd` | `input < 0` | `NULL`, `json_error_invalid_argument`, source `<stream>` | [x] |
| 148 | `json_load_file` | `path == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 149 | `json_load_file` | `fopen` fails (nonexistent path) | `NULL`, `json_error_cannot_open_file`, `"unable to open %s: %s"` | [x] |
| 150 | `json_load_callback` | `callback == NULL` | `NULL`, `json_error_invalid_argument`, source `<callback>` | [x] |
| 151 | `json_load_callback` | callback returns `0` / `(size_t)-1` → EOF mid-parse | premature-end error | [x] |
| 152 | `error_set` | empty `saved_text` + `json_error_invalid_syntax` | code rewritten to `json_error_premature_end_of_input` | [x] |
| 153 | `error_set` | `saved_text.length > 20` | no `" near '…'"` context appended | [x] |
| 154 | `error_set` | `saved_text.length <= 20` | `"<msg> near '<text>'"` | [x] |
| 155 | `error_set` | `stream.state == STREAM_STATE_ERROR`, empty saved_text | plain msg, no `" near end of file"` | [x] |
| 156 | `error_set` | empty saved_text, non-error stream state | `"<msg> near end of file"` | [x] |

## 4. `pack_unpack.c`

Covered by **tests/t08_pack.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 157 | `read_string` | `str == NULL` and not optional | `json_error_null_value`, `"NULL <purpose>"`, source `<args>` | [x] |
| 158 | `read_string` | `!utf8_check_string(str)` | `json_error_invalid_utf8`, `"Invalid UTF-8 <purpose>"` | [x] |
| 159 | `read_string` | `s?`/`s*` combined with `#`/`%`/`+` | `json_error_invalid_format`, `"Cannot use '%c' on optional strings"`, src `<format>` | [x] |
| 160 | `read_string` | concat path (`s+`), one arg `NULL` | `json_error_null_value` | [x] |
| 161 | `read_string` | concat path result invalid UTF-8 | `json_error_invalid_utf8` | [x] |
| 162 | `pack_object` | format ends before `}` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 163 | `pack_object` | key spec is not `s` | `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 164 | `pack_object` | value packs to NULL and next token != `*` | `json_error_null_value`, `"NULL object value"` | [x] |
| 165 | `pack_array` | format ends before `]` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 166 | `pack_array` | value packs to NULL and next token != `*` | `has_error` set → `NULL` return | [x] |
| 167 | `pack_object_inter` (`o`/`O`) | arg NULL and no `?`/`*` | `json_error_null_value`, `"NULL object"` | [x] |
| 168 | `pack_real` (`f`) | value is NaN / ±Inf → `json_real_set` fails | `json_error_numeric_overflow`, `"Invalid floating point value"` | [x] |
| 169 | `pack` | unrecognised format character | `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 170 | `json_vpack_ex` | `fmt == NULL` | `NULL`, `json_error_invalid_argument`, `"NULL or empty format string"`, src `<format>` | [x] |
| 171 | `json_vpack_ex` | `fmt == ""` | same as above | [x] |
| 172 | `json_vpack_ex` | trailing characters after a complete value | `json_error_invalid_format`, `"Garbage after format string"` | [x] |
| 173 | `unpack_object` | `root` non-NULL and not an object | `json_error_wrong_type`, `"Expected object, got %s"`, src `<validation>` | [x] |
| 174 | `unpack_object` | token after `!`/`*` is not `}` | `json_error_invalid_format`, `"Expected '}' after '%c', got '%c'"` | [x] |
| 175 | `unpack_object` | format ends before `}` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 176 | `unpack_object` | key spec is not `s` | `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 177 | `unpack_object` | key vararg is NULL | `json_error_null_value`, `"NULL object key"` | [x] |
| 178 | `unpack_object` | required key absent from object | `json_error_item_not_found`, `"Object item not found: %s"` | [x] |
| 179 | `unpack_object` | strict (`!` or `JSON_STRICT`) with unconsumed keys | `json_error_end_of_input_expected`, `"%li object item(s) left unpacked: %s"` | [x] |
| 180 | `unpack_array` | `root` non-NULL and not an array | `json_error_wrong_type`, `"Expected array, got %s"` | [x] |
| 181 | `unpack_array` | token after `!`/`*` is not `]` | `json_error_invalid_format`, `"Expected ']' after '%c', got '%c'"` | [x] |
| 182 | `unpack_array` | format ends before `]` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 183 | `unpack_array` | token not in `"{[siIbfFOon"` | `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 184 | `unpack_array` | more format items than array elements | `json_error_index_out_of_range`, `"Array index %lu out of range"` | [x] |
| 185 | `unpack_array` | strict with unconsumed elements | `json_error_end_of_input_expected`, `"%li array item(s) left unpacked"` | [x] |
| 186 | `unpack` `s` | root is not a string | `json_error_wrong_type`, `"Expected string, got %s"` | [x] |
| 187 | `unpack` `s` | `const char **` target is NULL | `json_error_null_value`, `"NULL string argument"` | [x] |
| 188 | `unpack` `s%` | `size_t *` length target is NULL | `json_error_null_value`, `"NULL string length argument"` | [x] |
| 189 | `unpack` `i` | root is not an integer | `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| 190 | `unpack` `I` | root is not an integer | `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| 191 | `unpack` `b` | root is not a boolean | `json_error_wrong_type`, `"Expected true or false, got %s"` | [x] |
| 192 | `unpack` `f` | root is not a real | `json_error_wrong_type`, `"Expected real, got %s"` | [x] |
| 193 | `unpack` `F` | root is not a number | `json_error_wrong_type`, `"Expected real or integer, got %s"` | [x] |
| 194 | `unpack` `n` | root is not null | `json_error_wrong_type`, `"Expected null, got %s"` | [x] |
| 195 | `unpack` | unrecognised format character | `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 196 | `json_vunpack_ex` | `root == NULL` | `-1`, `json_error_null_value`, `"NULL root value"`, src `<root>` | [x] |
| 197 | `json_vunpack_ex` | `fmt == NULL` | `-1`, `json_error_invalid_argument`, src `<format>` | [x] |
| 198 | `json_vunpack_ex` | `fmt == ""` | `-1`, `json_error_invalid_argument` | [x] |
| 199 | `json_vunpack_ex` | trailing characters after format | `-1`, `json_error_invalid_format`, `"Garbage after format string"` | [x] |

## 5. `memory.c`

Covered by **tests/t09_misc.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 200 | `jsonp_malloc` | `size == 0` | `NULL` (no allocator call) | [x] |
| 201 | `jsonp_free` | `ptr == NULL` | no-op (no allocator call) | [x] |
| 202 | `jsonp_realloc` | `do_realloc == NULL` (after `json_set_alloc_funcs`) and `newSize == 0` | `NULL` | [x] |
| 203 | `jsonp_realloc` | `do_realloc == NULL`, `ptr == NULL`, `newSize > 0` | fresh block, no copy | [x] |
| 204 | `jsonp_strndup` | allocator returns NULL / `len+1 == 0` | `NULL` | [x] |
| 205 | `json_get_alloc_funcs` | both out-params NULL | no-op, no crash | [x] |
| 206 | `json_get_alloc_funcs2` | all three out-params NULL | no-op, no crash | [x] |

## 6. `strbuffer.c`

Covered by **tests/t02_strbuffer.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 207 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | `-1` | [x] |
| 208 | `strbuffer_append_bytes` | `strbuff->length > SIZE_MAX - 1 - size` | `-1` | [x] |
| 209 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX / 2` | `-1` | [x] |
| 210 | `strbuffer_pop` | `length == 0` (empty buffer) | `'\0'`, length stays 0 | [x] |
| 211 | `strbuffer_steal_value` | called twice | 2nd call returns `NULL` | [x] |

## 7. `utf.c`

Covered by **tests/t01_utf.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 212 | `utf8_encode` | `codepoint < 0` | `-1`, `*size` untouched | [x] |
| 213 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | [x] |
| 214 | `utf8_check_first` | continuation byte `0x80..0xBF` | `0` | [x] |
| 215 | `utf8_check_first` | `0xC0` or `0xC1` (overlong ASCII) | `0` | [x] |
| 216 | `utf8_check_first` | `>= 0xF5` | `0` | [x] |
| 217 | `utf8_check_full` | `size` not 2, 3 or 4 (0, 1, 5, …) | `0` | [x] |
| 218 | `utf8_check_full` | a byte at index ≥ 1 is not a continuation byte | `0` | [x] |
| 219 | `utf8_check_full` | decoded `value > 0x10FFFF` | `0` | [x] |
| 220 | `utf8_check_full` | decoded value is a surrogate `D800..DFFF` | `0` | [x] |
| 221 | `utf8_check_full` | overlong (2B < 0x80, 3B < 0x800, 4B < 0x10000) | `0` | [x] |
| 222 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged (not NULL) | [x] |
| 223 | `utf8_iterate` | `utf8_check_first == 0` | `NULL` | [x] |
| 224 | `utf8_iterate` | `count > bufsize` (truncated sequence) | `NULL` | [x] |
| 225 | `utf8_iterate` | `utf8_check_full` fails | `NULL` | [x] |
| 226 | `utf8_check_string` | invalid lead byte | `0` | [x] |
| 227 | `utf8_check_string` | `count > length - i` (truncated at end) | `0` | [x] |
| 228 | `utf8_check_string` | `utf8_check_full` fails mid-string | `0` | [x] |

## 8. `hashtable.c`

Covered by **tests/t03_hashtable.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 229 | `hashtable_get` | empty bucket / key absent | `NULL` | [x] |
| 230 | `hashtable_del` | key absent (`hashtable_do_del` miss) | `-1` | [x] |
| 231 | `init_pair` (via `hashtable_set`) | `key_len >= SIZE_MAX - offsetof(pair_t,key)` | `-1` | [x] |
| 232 | `hashtable_iter_at` | key absent | `NULL` | [x] |
| 233 | `hashtable_iter_next` | iterator is at the last element | `NULL` | [x] |
| 234 | `hashtable_iter` | table is empty | `NULL` | [x] |

## 9. `strconv.c`

Covered by **tests/t04_dtoa.rs, tests/t10_abort_parity.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 235 | `jsonp_strtod` | value is `±HUGE_VAL` with `errno == ERANGE` (`1e999`) | `-1` | [x] |
| 236 | `jsonp_dtostr` | buffer too small for the formatted value | `-1` | [x] |
| 237 | `jsonp_dtostr` | `size == 0` | `-1` | [x] |

## 10. `dtoa.c`

Covered by **tests/t04_dtoa.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 238 | `dtoa_r` | `buf != NULL` and `blen` too small | `NULL` (with `*rve` hint) | [x] |
| 239 | `dtoa_r` | `dd` is NaN or ±Inf | `*decpt = 9999` | [x] |
| 240 | `dtoa_r` | `mode` outside `0..9` (e.g. `-1`, `10`, `1000`) | treated as mode 0 | [x] |
| 241 | `dtoa_r` | `ndigits <= 0` in modes 2/4 | clamped to 1 significant digit | [x] |
| 242 | `gethex` | `sp` points at a string with no hex digits | `*rvp` set per C, `*sp` advanced per C | [x] |
| 243 | `strtod__unused` | malformed / empty input | `0.0`, `*se == s00` | [x] |

## 11. `error.c`

Covered by **tests/t09_misc.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 244 | `jsonp_error_init` | `error == NULL` | no-op | [x] |
| 245 | `jsonp_error_set_source` | `error == NULL` | no-op | [x] |
| 246 | `jsonp_error_set_source` | `source == NULL` | no-op | [x] |
| 247 | `jsonp_error_set_source` | `strlen(source) >= JSON_ERROR_SOURCE_LENGTH` (80) | `"..."` + tail | [x] |
| 248 | `jsonp_error_set` / `jsonp_error_vset` | `error == NULL` | no-op | [x] |
| 249 | `jsonp_error_vset` | `error->text[0] != '\0'` (already set) | returns without overwriting | [x] |
| 250 | `jsonp_error_vset` | message longer than 158 chars | truncated, `text[158]='\0'`, `text[159]=code` | [x] |
| 251 | `json_error_code` (header inline) | out-of-range code byte stored in `text[159]` | round-trips the raw byte | [x] |

## 12. Cross-cutting / FFI boundary

Covered by **tests/t05_value.rs, tests/t06_load.rs, tests/t07_dump.rs, tests/t08_pack.rs, tests/t09_misc.rs**.

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|-----|
| 252 | `json_dump_callback`, `json_dumps`, `json_dumpb` | `flags` with reserved/unknown high bits set (`1<<40`) | ignored, same output as `flags & known` | [x] |
| 253 | `json_loads` etc. | `flags` with unknown bits set | ignored | [x] |
| 254 | `json_dumps` | `JSON_INDENT(n)` with `n > 31` (masked to `n & 0x1F`) | masked indent | [x] |
| 255 | `json_dumps` | `JSON_REAL_PRECISION(n)` for every `n` in `0..31` | `-1` for the precisions whose output exceeds 25 bytes | [x] |
| 256 | all `json_*` getters | out-of-range `json_type` value (e.g. 42) in a fabricated `json_t` | matching `default:` branch behaviour | [x] |
| 257 | `jansson_version_cmp` | negative / huge major-minor-micro args | signed `diff` per C | [x] |
| 258 | `json_object_seed` | `seed == 0` (auto-seed) then non-zero | second call is a no-op (seed already set) | [x] |

---

## Results

**258 / 258 rows verified.** Every row has a differential test that constructs
exactly that condition, calls **both** the C `.so` and the Rust `.so` through
`libloading`, and asserts the same sentinel **and** — wherever a `json_error_t`
is filled in — the identical **252 raw bytes** of the struct (`eq_err`, which
diffs `json_error_t::raw()`), not merely the same error code.

Verified under all three build configurations: dev, `--no-default-features`
(the only feature combination — the crate has no `[features]`), and `--release`.

## Divergences found and fixed

Two real C-vs-Rust divergences were found by these error-path tests and fixed in
the **Rust** (the C is ground truth):

1. **`pack_unpack.c` `set_error()` — rows 162, 163, 169, 174, 176, 181, 183, 195.**
   The C passes the caller's format string and varargs *straight through* to
   `jsonp_error_vset()`, which runs `vsnprintf()` directly on `error->text`. The
   Rust formatted into a scratch buffer first and re-emitted it with `"%s"`.
   That is not equivalent: these messages use `"%c"`, and when the unexpected
   format character is `'\0'` (format string ended) the direct `vsnprintf` still
   writes the bytes *after* the embedded NUL — the C leaves
   `text[30] == '\''`, whereas the `"%s"` pass stopped at `text[29]` and left
   `text[30] == 0`. Fixed by inlining `jsonp_error_vset`'s body into the Rust
   `set_error!` macro (`src/pack_unpack.rs`). `load.c`'s `error_set()` genuinely
   *does* use the `"%s"` indirection, so `src/load.rs` was already correct and
   was left alone.

2. **`strconv.c:53` — the one live, externally reachable `assert()`.**
   ```c
   value = strtod(strbuffer->value, &end);
   assert(end == strbuffer->value + strbuffer->length);
   ```
   `c_src/CMakeLists.txt` does not define `NDEBUG`, so this assertion is live,
   and `jsonp_strtod` is an exported symbol. A caller handing it a strbuffer that
   `strtod()` does not consume in full (e.g. `"1e"`, `"zzz"`, `"1.5abc"`) makes
   the C `__assert_fail()` and abort; the Rust silently returned `0`. Fixed in
   `src/strconv.rs` by calling glibc's `__assert_fail` with the same assertion
   text, file and function name. `tests/t10_abort_parity.rs` re-executes itself
   in a child process and asserts both libraries die with **SIGABRT** and print
   the same assertion, and that neither aborts on fully-consumable input.

## Notes on rows that are unreachable in the C itself

These rows are documented as they appear in the source, but the C cannot actually
return the documented value; the tests assert the behaviour the C *really* has
and confirm the Rust matches it.

* **Row 231 (`init_pair` long-key overflow guard).** `hashtable_set` computes
  `hash_str(key, key_len)` *before* calling `init_pair`, and `hashlittle()` reads
  `key_len` bytes — so a `key_len` near `SIZE_MAX - offsetof(pair_t, key)` walks
  off the buffer and kills the process before the guard can return `-1`. Probed
  in forked children: **both** libraries die with SIGSEGV, identically. To still
  cover `hashtable_set`'s `if (!pair) return -1;` the test installs an
  always-failing `jsonp_malloc` via `json_set_alloc_funcs`; both return `-1`.
* **Rows 83, 97 (`do_dump` `JSON_INTEGER` / `JSON_REAL` buffer checks).**
  `snprintf` of a `long long` never reaches 25 bytes, so the integer branch's
  `-1` is dead. The real branch *is* reachable, but only for finite non-zero
  values: `±0.0`, NaN and `±Inf` leave `dtoa_r` through `nrv_alloc` before the
  `blen` check and still succeed at `size == 25`.
* **The remaining live `assert()`s** (`dump.c:354`, `dump.c:364`, and all of
  `load.c`) guard internal invariants that no public-API input can violate; they
  were audited individually and are unreachable from outside the library.
* **`dtoa.c`'s own asserts are disabled** by `#define assert(x) /*nothing*/` at
  `dtoa.c:242`, so they need no counterpart in the Rust.

## Test-effectiveness (mutation) check

The suite was validated by injecting single-line defects into the Rust and
confirming the tests fail (then reverting):

| mutation in `src/` | caught by |
|--------------------|-----------|
| `utf.rs`: `u < 0x80` → `u < 0x7F` in `utf8_check_first` | `t01_utf`, `t05_value`, `t06_load`, `t07_dump` |
| `strbuffer.rs`: `STRBUFFER_MIN_SIZE` 16 → 32 | `t02_strbuffer` |
| `hashtable.rs`: `INITIAL_HASHTABLE_ORDER` 3 → 4 | `t03_hashtable` |
| `strconv.rs`: `decpt > 16` → `decpt > 15` in `jsonp_dtostr` | `t04_dtoa` |
| `value.rs`: array initial capacity 8 → 4 | `t09_misc` (allocation sequences) |
| `jansson.rs`: `JSON_PARSER_MAX_DEPTH` 2048 → 2047 | `t06_load` (5 tests) |
| `dump.rs`: `compare_keys` tie-break `k1.len - k2.len` → `k2.len - k1.len` | `t07_dump` (8 tests) |
| `version.rs`: skip the minor-version comparison | `t00_smoke`, `t09_misc` |
| `strconv.rs`: remove the `jsonp_strtod` assertion | `t10_abort_parity` (2 tests) |
