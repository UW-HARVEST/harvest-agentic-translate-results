# ERRORS.md — error / rejection surface

Every distinct way the C source rejects or errors on input, obtained by grepping
`c_src/src/*.c` for `return -1`, `return NULL`, `return 0` guard clauses,
`error_set(...)`, `set_error(...)`, `jsonp_error_set(...)`, `assert(...)`, and
every explicit range / NULL / min-max check.  One row per distinct rejection.

`[x]` = a differential test constructs exactly that condition and asserts C and
Rust return the same error code / sentinel.

## `value.c`

| # | function | trigger (exact invalid input/condition) | expected C result | done |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `jsonp_loop_check` | pointer already in `parents` hashtable | `-1` | [x] |
| 2 | `json_object` | `jsonp_malloc` fails (custom failing allocator) | `NULL` | [x] |
| 3 | `json_object` | `hashtable_init` fails (2nd allocation fails) | `NULL` | [x] |
| 4 | `json_object_size` | `json == NULL` | `0` | [x] |
| 5 | `json_object_size` | `json` not an object (array/string/int/real/true/false/null) | `0` | [x] |
| 6 | `json_object_get` | `key == NULL` | `NULL` | [x] |
| 7 | `json_object_getn` | `key == NULL` | `NULL` | [x] |
| 8 | `json_object_getn` | `json == NULL` or not an object | `NULL` | [x] |
| 9 | `json_object_getn` | key absent from object | `NULL` | [x] |
| 10 | `json_object_set_new_nocheck` | `key == NULL` (also decrefs value) | `-1` | [x] |
| 11 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` | [x] |
| 12 | `json_object_setn_new_nocheck` | `key == NULL` | `-1` | [x] |
| 13 | `json_object_setn_new_nocheck` | `json` not an object / NULL | `-1` | [x] |
| 14 | `json_object_setn_new_nocheck` | `json == value` (self insert) | `-1` | [x] |
| 15 | `json_object_setn_new_nocheck` | `hashtable_set` fails (OOM) | `-1` | [x] |
| 16 | `json_object_set_new` | `key == NULL` | `-1` | [x] |
| 17 | `json_object_setn_new` | `key == NULL` | `-1` | [x] |
| 18 | `json_object_setn_new` | key is not valid UTF-8 (`utf8_check_string` fails) | `-1` | [x] |
| 19 | `json_object_del` | `key == NULL` | `-1` | [x] |
| 20 | `json_object_deln` | `key == NULL` | `-1` | [x] |
| 21 | `json_object_deln` | `json` not an object / NULL | `-1` | [x] |
| 22 | `json_object_deln` | key not present (`hashtable_del` miss) | `-1` | [x] |
| 23 | `json_object_clear` | `json` not an object / NULL | `-1` | [x] |
| 24 | `json_object_update` | `object` not an object | `-1` | [x] |
| 25 | `json_object_update` | `other` not an object | `-1` | [x] |
| 26 | `json_object_update_existing` | `object` or `other` not an object | `-1` | [x] |
| 27 | `json_object_update_missing` | `object` or `other` not an object | `-1` | [x] |
| 28 | `do_object_update_recursive` | `object` or `other` not an object | `-1` | [x] |
| 29 | `do_object_update_recursive` | `other` already in `parents` (cycle) | `-1` | [x] |
| 30 | `json_object_update_recursive` | `hashtable_init` fails (OOM) | `-1` | [x] |
| 31 | `json_object_update_recursive` | self-referencing object graph (cycle) | `-1` | [x] |
| 32 | `json_object_iter` | `json` not an object / NULL | `NULL` | [x] |
| 33 | `json_object_iter` | empty object | `NULL` | [x] |
| 34 | `json_object_iter_at` | `key == NULL` | `NULL` | [x] |
| 35 | `json_object_iter_at` | `json` not an object / NULL | `NULL` | [x] |
| 36 | `json_object_iter_at` | key not present | `NULL` | [x] |
| 37 | `json_object_iter_next` | `json` not an object / NULL | `NULL` | [x] |
| 38 | `json_object_iter_next` | `iter == NULL` | `NULL` | [x] |
| 39 | `json_object_iter_next` | iterator is the last element | `NULL` | [x] |
| 40 | `json_object_iter_key` | `iter == NULL` | `NULL` | [x] |
| 41 | `json_object_iter_key_len` | `iter == NULL` | `0` | [x] |
| 42 | `json_object_iter_value` | `iter == NULL` | `NULL` | [x] |
| 43 | `json_object_iter_set_new` | `json` not an object / NULL | `-1` | [x] |
| 44 | `json_object_iter_set_new` | `iter == NULL` | `-1` | [x] |
| 45 | `json_object_iter_set_new` | `value == NULL` | `-1` | [x] |
| 46 | `json_object_key_to_iter` | `key == NULL` | `NULL` | [x] |
| 47 | `json_array` | `jsonp_malloc` of the header fails | `NULL` | [x] |
| 48 | `json_array` | `jsonp_malloc` of the table fails (2nd alloc) | `NULL` | [x] |
| 49 | `json_array_size` | `json` not an array / NULL | `0` | [x] |
| 50 | `json_array_get` | `json` not an array / NULL | `NULL` | [x] |
| 51 | `json_array_get` | `index >= entries` (incl. `SIZE_MAX`, empty array) | `NULL` | [x] |
| 52 | `json_array_set_new` | `value == NULL` | `-1` | [x] |
| 53 | `json_array_set_new` | `json` not an array / NULL | `-1` | [x] |
| 54 | `json_array_set_new` | `json == value` | `-1` | [x] |
| 55 | `json_array_set_new` | `index >= entries` | `-1` | [x] |
| 56 | `json_array_append_new` | `value == NULL` | `-1` | [x] |
| 57 | `json_array_append_new` | `json` not an array / NULL | `-1` | [x] |
| 58 | `json_array_append_new` | `json == value` | `-1` | [x] |
| 59 | `json_array_append_new` | `json_array_grow` fails (OOM on realloc) | `-1` | [x] |
| 60 | `json_array_insert_new` | `value == NULL` | `-1` | [x] |
| 61 | `json_array_insert_new` | `json` not an array / NULL | `-1` | [x] |
| 62 | `json_array_insert_new` | `json == value` | `-1` | [x] |
| 63 | `json_array_insert_new` | `index > entries` | `-1` | [x] |
| 64 | `json_array_insert_new` | `json_array_grow` fails (OOM) | `-1` | [x] |
| 65 | `json_array_remove` | `json` not an array / NULL | `-1` | [x] |
| 66 | `json_array_remove` | `index >= entries` | `-1` | [x] |
| 67 | `json_array_clear` | `json` not an array / NULL | `-1` | [x] |
| 68 | `json_array_extend` | `json` not an array | `-1` | [x] |
| 69 | `json_array_extend` | `other` not an array / NULL | `-1` | [x] |
| 70 | `json_array_extend` | `json_array_grow` fails (OOM) | `-1` | [x] |
| 71 | `string_create` (via `json_stringn_nocheck`) | `value == NULL` | `NULL` | [x] |
| 72 | `string_create` | `jsonp_strndup` fails (OOM) | `NULL` | [x] |
| 73 | `string_create` | `jsonp_malloc(sizeof json_string_t)` fails (2nd alloc) | `NULL` | [x] |
| 74 | `json_string_nocheck` | `value == NULL` | `NULL` | [x] |
| 75 | `json_string` | `value == NULL` | `NULL` | [x] |
| 76 | `json_stringn` | `value == NULL` | `NULL` | [x] |
| 77 | `json_stringn` | invalid UTF-8 (every `utf8_check_string` failure class) | `NULL` | [x] |
| 78 | `json_string_value` | `json` not a string / NULL | `NULL` | [x] |
| 79 | `json_string_length` | `json` not a string / NULL | `0` | [x] |
| 80 | `json_string_set_nocheck` | `value == NULL` | `-1` | [x] |
| 81 | `json_string_setn_nocheck` | `json` not a string / NULL | `-1` | [x] |
| 82 | `json_string_setn_nocheck` | `value == NULL` | `-1` | [x] |
| 83 | `json_string_setn_nocheck` | `jsonp_strndup` fails (OOM) | `-1` | [x] |
| 84 | `json_string_set` | `value == NULL` | `-1` | [x] |
| 85 | `json_string_setn` | `value == NULL` | `-1` | [x] |
| 86 | `json_string_setn` | invalid UTF-8 | `-1` | [x] |
| 87 | `json_vsprintf`/`json_sprintf` | result is not valid UTF-8 | `NULL` | [x] |
| 88 | `json_vsprintf`/`json_sprintf` | `jsonp_malloc` for the buffer fails | `NULL` | [x] |
| 89 | `json_integer` | `jsonp_malloc` fails | `NULL` | [x] |
| 90 | `json_integer_value` | `json` not an integer / NULL | `0` | [x] |
| 91 | `json_integer_set` | `json` not an integer / NULL | `-1` | [x] |
| 92 | `json_real` | `value` is NaN | `NULL` | [x] |
| 93 | `json_real` | `value` is `+INFINITY` | `NULL` | [x] |
| 94 | `json_real` | `value` is `-INFINITY` | `NULL` | [x] |
| 95 | `json_real` | `jsonp_malloc` fails | `NULL` | [x] |
| 96 | `json_real_value` | `json` not a real / NULL | `0.0` | [x] |
| 97 | `json_real_set` | `json` not a real / NULL | `-1` | [x] |
| 98 | `json_real_set` | `value` is NaN / ±INFINITY | `-1` | [x] |
| 99 | `json_number_value` | `json` neither integer nor real (incl. NULL) | `0.0` | [x] |
| 100 | `json_delete` | `json == NULL` | no-op | [x] |
| 101 | `json_delete` | `json` is `true`/`false`/`null` or an out-of-range type tag | no-op (`default: return`) | [x] |
| 102 | `json_equal` | `json1 == NULL` | `0` | [x] |
| 103 | `json_equal` | `json2 == NULL` | `0` | [x] |
| 104 | `json_equal` | differing type tags | `0` | [x] |
| 105 | `json_equal` | out-of-range type tag on both (`default:`) | `0` | [x] |
| 106 | `json_copy` | `json == NULL` | `NULL` | [x] |
| 107 | `json_copy` | out-of-range type tag (`default:`) | `NULL` | [x] |
| 108 | `json_object_copy` (via `json_copy`) | `json_object()` fails (OOM) | `NULL` | [x] |
| 109 | `json_array_copy` (via `json_copy`) | `json_array()` fails (OOM) | `NULL` | [x] |
| 110 | `json_deep_copy` | `hashtable_init` fails (OOM) | `NULL` | [x] |
| 111 | `do_deep_copy` | `json == NULL` | `NULL` | [x] |
| 112 | `do_deep_copy` | out-of-range type tag (`default:`) | `NULL` | [x] |
| 113 | `json_object_deep_copy` | object already in `parents` (cycle) | `NULL` | [x] |
| 114 | `json_array_deep_copy` | array already in `parents` (cycle) | `NULL` | [x] |
| 115 | `json_deep_copy` | self-referencing object / array graph | `NULL` | [x] |

## `dump.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 116 | `dump_to_file` (via `json_dumpf`) | `fwrite` fails (read-only `FILE*`) | `-1` | [x] |
| 117 | `dump_to_fd` (via `json_dumpfd`) | `write` fails (bad / read-only fd) | `-1` | [x] |
| 118 | `dump_indent` (via `json_dump_callback`) | callback returns non-zero on the `"\n"` chunk | `-1` | [x] |
| 119 | `dump_string` | string content is not valid UTF-8 (`utf8_iterate` returns NULL) | `-1` | [x] |
| 120 | `dump_string` | callback fails on the opening `"` / a text chunk / the closing `"` | `-1` | [x] |
| 121 | `do_dump` | `json == NULL` | `-1` | [x] |
| 122 | `do_dump` | out-of-range type tag (`default:`) | `-1` | [x] |
| 123 | `do_dump` `JSON_REAL` | `jsonp_dtostr` returns `< 0` (buffer too short for the value/precision) | `-1` | [x] |
| 124 | `do_dump` `JSON_ARRAY` | array is part of a cycle (`jsonp_loop_check`) | `-1` | [x] |
| 125 | `do_dump` `JSON_OBJECT` | object is part of a cycle | `-1` | [x] |
| 126 | `do_dump` `JSON_SORT_KEYS` | `jsonp_malloc(size * sizeof(key_len))` fails | `-1` | [x] |
| 127 | `json_dumps` | `strbuffer_init` fails (OOM) | `NULL` | [x] |
| 128 | `json_dumps` | underlying `json_dump_callback` fails (e.g. not a container, no `ENCODE_ANY`) | `NULL` | [x] |
| 129 | `json_dumpb` | dump fails | `0` | [x] |
| 130 | `json_dumpb` | `size` smaller than the output (no write, but full length returned) | full length | [x] |
| 131 | `json_dump_file` | `fopen` fails (unwritable path) | `-1` | [x] |
| 132 | `json_dump_file` | dump itself fails (`fclose` still succeeds) | `-1` | [x] |
| 133 | `json_dump_callback` | `json` is a scalar and `JSON_ENCODE_ANY` is not set | `-1` | [x] |
| 134 | `json_dump_callback` | `json == NULL` and `JSON_ENCODE_ANY` not set | `-1` | [x] |
| 135 | `json_dump_callback` | `json == NULL` with `JSON_ENCODE_ANY` | `-1` (from `do_dump`) | [x] |
| 136 | `json_dump_callback` | `hashtable_init` for `parents_set` fails (OOM) | `-1` | [x] |
| 137 | `json_dump_callback` | callback returns non-zero at chunk *k* for every reachable *k* | `-1` | [x] |

## `load.c`

All rows are `NULL` return **plus** a specific `enum json_error_code` recorded in
`error->text[159]`; the test compares the whole 248-byte `json_error_t`.

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 138 | `json_loads` | `string == NULL` | `NULL`, `json_error_invalid_argument`, `"wrong arguments"`, source `<string>` | [x] |
| 139 | `json_loadb` | `buffer == NULL` | `NULL`, `json_error_invalid_argument`, source `<buffer>` | [x] |
| 140 | `json_loadf` | `input == NULL` | `NULL`, `json_error_invalid_argument`, source `<stream>` | [x] |
| 141 | `json_loadfd` | `input < 0` | `NULL`, `json_error_invalid_argument`, source `<stream>` | [x] |
| 142 | `json_load_file` | `path == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 143 | `json_load_file` | file cannot be opened | `NULL`, `json_error_cannot_open_file`, `"unable to open %s: %s"` | [x] |
| 144 | `json_load_callback` | `callback == NULL` | `NULL`, `json_error_invalid_argument`, source `<callback>` | [x] |
| 145 | `json_load_callback` | callback returns `(size_t)-1` | treated as EOF ⇒ premature end | [x] |
| 146 | `stream_get` | first byte is an invalid UTF-8 lead (`utf8_check_first == 0`) | `json_error_invalid_utf8`, `"unable to decode byte 0x%x"` | [x] |
| 147 | `stream_get` | truncated/invalid continuation bytes (`utf8_check_full` fails) | `json_error_invalid_utf8` | [x] |
| 148 | `lex_scan_string` | EOF inside a string literal | `json_error_premature_end_of_input` | [x] |
| 149 | `lex_scan_string` | raw newline inside a string | `json_error_invalid_syntax`, `"unexpected newline"` | [x] |
| 150 | `lex_scan_string` | other control character `0x00..0x1F` inside a string | `json_error_invalid_syntax`, `"control character 0x%x"` | [x] |
| 151 | `lex_scan_string` | `\u` followed by a non-hex digit | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 152 | `lex_scan_string` | unsupported escape letter (e.g. `\x`, `\z`, `\U`) | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 153 | `lex_scan_string` | `jsonp_malloc` for the decoded value fails | `TOKEN_INVALID` ⇒ `"invalid token"` | [x] |
| 154 | `lex_scan_string` | high surrogate `\uD800..\uDBFF` not followed by `\u` | `"invalid Unicode '\\uXXXX'"` | [x] |
| 155 | `lex_scan_string` | high surrogate followed by a non-low-surrogate `\uXXXX` | `"invalid Unicode '\\uXXXX\\uXXXX'"` | [x] |
| 156 | `lex_scan_string` | lone low surrogate `\uDC00..\uDFFF` | `"invalid Unicode '\\uXXXX'"` | [x] |
| 157 | `lex_scan_number` | leading zero followed by a digit (`01`, `-012`) | `json_error_invalid_syntax`, `"invalid token"` | [x] |
| 158 | `lex_scan_number` | `-` not followed by a digit (`-`, `-x`, `-.5`) | `json_error_invalid_syntax` | [x] |
| 159 | `lex_scan_number` | integer above `LLONG_MAX` (`ERANGE`, positive) | `json_error_numeric_overflow`, `"too big integer"` | [x] |
| 160 | `lex_scan_number` | integer below `LLONG_MIN` (`ERANGE`, negative) | `json_error_numeric_overflow`, `"too big negative integer"` | [x] |
| 161 | `lex_scan_number` | `.` not followed by a digit (`1.`, `1.e5`) | `json_error_invalid_syntax` | [x] |
| 162 | `lex_scan_number` | exponent not followed by a digit (`1e`, `1e+`, `1e-`) | `json_error_invalid_syntax` | [x] |
| 163 | `lex_scan_number` | real overflow (`1e400`) ⇒ `jsonp_strtod` returns `-1` | `json_error_numeric_overflow`, `"real number overflow"` | [x] |
| 164 | `lex_scan` | identifier that is not `true`/`false`/`null` (`tru`, `nul`, `True`) | `TOKEN_INVALID` ⇒ `"invalid token"` | [x] |
| 165 | `lex_scan` | byte that starts no token (`@`, `#`, `'`, valid multi-byte UTF-8) | `TOKEN_INVALID` ⇒ `"invalid token"` | [x] |
| 166 | `parse_object` | token after `{` is neither a string nor `}` | `json_error_invalid_syntax`, `"string or '}' expected"` | [x] |
| 167 | `parse_object` | object key contains a NUL byte (`"a b"`) | `json_error_null_byte_in_key` | [x] |
| 168 | `parse_object` | duplicate key with `JSON_REJECT_DUPLICATES` | `json_error_duplicate_key` | [x] |
| 169 | `parse_object` | missing `:` after key | `json_error_invalid_syntax`, `"':' expected"` | [x] |
| 170 | `parse_object` | value fails to parse (error propagates) | `NULL`, inner code preserved | [x] |
| 171 | `parse_object` | missing `}` / `,` before EOF | `json_error_invalid_syntax`, `"'}' expected"` | [x] |
| 172 | `parse_object` | `json_object_setn_new_nocheck` fails (OOM) | `NULL` | [x] |
| 173 | `parse_array` | missing `]` / `,` | `json_error_invalid_syntax`, `"']' expected"` | [x] |
| 174 | `parse_array` | element fails to parse | `NULL`, inner code preserved | [x] |
| 175 | `parse_value` | nesting deeper than `JSON_PARSER_MAX_DEPTH` (2048) | `json_error_stack_overflow`, `"maximum parsing depth reached"` | [x] |
| 176 | `parse_value` | ` ` in a string without `JSON_ALLOW_NUL` | `json_error_null_character` | [x] |
| 177 | `parse_value` | `TOKEN_INVALID` | `json_error_invalid_syntax`, `"invalid token"` | [x] |
| 178 | `parse_value` | unexpected structural token (`}`, `]`, `,`, `:`) | `json_error_invalid_syntax`, `"unexpected token"` | [x] |
| 179 | `parse_value` | `json_integer`/`json_real`/string allocation fails | `NULL` | [x] |
| 180 | `parse_json` | top-level value is not `[`/`{` and `JSON_DECODE_ANY` unset | `json_error_invalid_syntax`, `"'[' or '{' expected"` | [x] |
| 181 | `parse_json` | trailing content and `JSON_DISABLE_EOF_CHECK` unset | `json_error_end_of_input_expected`, `"end of file expected"` | [x] |
| 182 | `error_set` | empty input ⇒ `invalid_syntax` remapped | `json_error_premature_end_of_input`, `"… near end of file"` | [x] |
| 183 | `error_set` | context longer than 20 bytes ⇒ no `near '…'` suffix | text without context | [x] |
| 184 | `lex_init` | `strbuffer_init` fails (OOM) | `NULL`, **error left untouched** | [x] |

## `pack_unpack.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 185 | `json_vpack_ex` | `fmt == NULL` | `NULL`, `json_error_invalid_argument`, `"NULL or empty format string"` | [x] |
| 186 | `json_vpack_ex` | `fmt == ""` | `NULL`, `json_error_invalid_argument` | [x] |
| 187 | `json_vpack_ex` | garbage after a complete format (`"[]x"`, `"{}i"`) | `json_error_invalid_format`, `"Garbage after format string"` | [x] |
| 188 | `pack` | unknown format character (`"q"`, `"1"`, `"]"`) | `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 189 | `pack_object` | format ends before `}` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 190 | `pack_object` | key format char is not `s` (`"{i:i}"`) | `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 191 | `pack_object` | value is NULL and not marked `*` | `json_error_null_value`, `"NULL object value"` | [x] |
| 192 | `pack_object` | `json_object_setn_new_nocheck` fails | `json_error_out_of_memory`, `"Unable to add key \"%s\""` | [x] |
| 193 | `pack_array` | format ends before `]` | `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 194 | `read_string` | `str == NULL` and not optional | `json_error_null_value`, `"NULL %s"` (`string` / `object key`) | [x] |
| 195 | `read_string` | argument is not valid UTF-8 | `json_error_invalid_utf8`, `"Invalid UTF-8 %s"` | [x] |
| 196 | `read_string` | `#`, `%` or `+` used on an optional string (`"s?#"`, `"s*+"`) | `json_error_invalid_format`, `"Cannot use '%c' on optional strings"` | [x] |
| 197 | `read_string` | `strbuffer_init` fails (OOM) in the `+`/`#`/`%` path | `json_error_out_of_memory`, `"Out of memory"` | [x] |
| 198 | `read_string` | concatenated (`+`) result is not valid UTF-8 | `json_error_invalid_utf8` | [x] |
| 199 | `read_string` | NULL argument inside a `+` chain | `json_error_null_value` | [x] |
| 200 | `pack_object_inter` | `json == NULL` for `O`/`o` without `?`/`*` | `json_error_null_value`, `"NULL object"` | [x] |
| 201 | `pack_object_inter` | `json == NULL` for `o*`/`O*` | `NULL` value, **no** error (skipped) | [x] |
| 202 | `pack_integer` | `json_integer` fails (OOM) | `json_error_out_of_memory`, `"Out of memory"` | [x] |
| 203 | `pack_real` | `json_real(0.0)` fails (OOM) | `json_error_out_of_memory` | [x] |
| 204 | `pack_real` | value is NaN / ±INFINITY (`json_real_set` fails) | `json_error_numeric_overflow`, `"Invalid floating point value"` | [x] |
| 205 | `pack_string` | `read_string` fails, `t == '?'`, no error ⇒ `json_null()` | `json_null` | [x] |
| 206 | `json_vunpack_ex` | `root == NULL` | `-1`, `json_error_null_value`, `"NULL root value"` | [x] |
| 207 | `json_vunpack_ex` | `fmt == NULL` | `-1`, `json_error_invalid_argument` | [x] |
| 208 | `json_vunpack_ex` | `fmt == ""` | `-1`, `json_error_invalid_argument` | [x] |
| 209 | `json_vunpack_ex` | garbage after format | `-1`, `json_error_invalid_format`, `"Garbage after format string"` | [x] |
| 210 | `unpack` | unknown format character | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 211 | `unpack` `s` | root is not a string | `-1`, `json_error_wrong_type`, `"Expected string, got %s"` | [x] |
| 212 | `unpack` `s` | `const char **` target is NULL | `-1`, `json_error_null_value`, `"NULL string argument"` | [x] |
| 213 | `unpack` `s%` | `size_t *` length target is NULL | `-1`, `json_error_null_value`, `"NULL string length argument"` | [x] |
| 214 | `unpack` `i` | root is not an integer | `-1`, `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| 215 | `unpack` `I` | root is not an integer | `-1`, `json_error_wrong_type` | [x] |
| 216 | `unpack` `b` | root is not a boolean | `-1`, `json_error_wrong_type`, `"Expected true or false, got %s"` | [x] |
| 217 | `unpack` `f` | root is not a real (an *integer* also fails) | `-1`, `json_error_wrong_type`, `"Expected real, got %s"` | [x] |
| 218 | `unpack` `F` | root is neither real nor integer | `-1`, `json_error_wrong_type`, `"Expected real or integer, got %s"` | [x] |
| 219 | `unpack` `n` | root is not null | `-1`, `json_error_wrong_type`, `"Expected null, got %s"` | [x] |
| 220 | `unpack_object` | root is not an object | `-1`, `json_error_wrong_type`, `"Expected object, got %s"` | [x] |
| 221 | `unpack_object` | `hashtable_init` fails (OOM) | `-1`, `json_error_out_of_memory` | [x] |
| 222 | `unpack_object` | token after `!`/`*` is not `}` | `-1`, `json_error_invalid_format`, `"Expected '}' after '%c', got '%c'"` | [x] |
| 223 | `unpack_object` | format ends before `}` | `-1`, `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 224 | `unpack_object` | key format char is not `s` | `-1`, `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 225 | `unpack_object` | key argument is NULL | `-1`, `json_error_null_value`, `"NULL object key"` | [x] |
| 226 | `unpack_object` | required key missing from the object | `-1`, `json_error_item_not_found`, `"Object item not found: %s"` | [x] |
| 227 | `unpack_object` | `JSON_STRICT`/`!` and object has extra keys | `-1`, `json_error_end_of_input_expected`, `"%li object item(s) left unpacked: %s"` | [x] |
| 228 | `unpack_object` | `!` with an optional (`s?`) key present ⇒ `gotopt` path still reports leftovers | `-1`, same code | [x] |
| 229 | `unpack_array` | root is not an array | `-1`, `json_error_wrong_type`, `"Expected array, got %s"` | [x] |
| 230 | `unpack_array` | token after `!`/`*` is not `]` | `-1`, `json_error_invalid_format`, `"Expected ']' after '%c', got '%c'"` | [x] |
| 231 | `unpack_array` | format ends before `]` | `-1`, `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 232 | `unpack_array` | format char not in `"{[siIbfFOon"` (e.g. `%`, `#`, `?`) | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 233 | `unpack_array` | more format items than array elements | `-1`, `json_error_index_out_of_range`, `"Array index %lu out of range"` | [x] |
| 234 | `unpack_array` | `JSON_STRICT`/`!` and array has extra elements | `-1`, `json_error_end_of_input_expected`, `"%li array item(s) left unpacked"` | [x] |

## `error.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 235 | `jsonp_error_init` | `error == NULL` | no-op (no crash) | [x] |
| 236 | `jsonp_error_set_source` | `error == NULL` | no-op | [x] |
| 237 | `jsonp_error_set_source` | `source == NULL` | no-op | [x] |
| 238 | `jsonp_error_set_source` | `strlen(source) >= 80` | `"..."` + tail, exactly 80 bytes incl. NUL | [x] |
| 239 | `jsonp_error_vset` | `error == NULL` | no-op | [x] |
| 240 | `jsonp_error_vset` | `error->text[0] != '\0'` (error already set) | no-op, first error kept | [x] |
| 241 | `jsonp_error_vset` | message longer than 158 bytes | truncated, `text[158] = 0`, `text[159] = code` | [x] |
| 242 | `jsonp_error_vset` | `code` outside `enum json_error_code` (e.g. 255, -1) | stored verbatim in `text[159]` | [x] |

## `memory.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 243 | `jsonp_malloc` | `size == 0` | `NULL` (never calls `do_malloc`) | [x] |
| 244 | `jsonp_free` | `ptr == NULL` | no-op | [x] |
| 245 | `jsonp_realloc` | `do_realloc == NULL` (after `json_set_alloc_funcs`) and `newSize == 0`, `ptr != NULL` | frees, returns `NULL` | [x] |
| 246 | `jsonp_realloc` | `do_realloc == NULL` and `newSize == 0`, `ptr == NULL` | `NULL` | [x] |
| 247 | `jsonp_realloc` | `do_realloc == NULL` and `do_malloc` fails | `NULL`, `ptr` not freed | [x] |
| 248 | `jsonp_strndup` | `jsonp_malloc(len+1)` fails | `NULL` | [x] |
| 249 | `json_get_alloc_funcs` | both out-params NULL | no-op | [x] |
| 250 | `json_get_alloc_funcs2` | all three out-params NULL | no-op | [x] |

## `strbuffer.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 251 | `strbuffer_init` | `jsonp_malloc(16)` fails | `-1` | [x] |
| 252 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX/2` | `-1` | [x] |
| 253 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` (i.e. `size == SIZE_MAX`) | `-1` | [x] |
| 254 | `strbuffer_append_bytes` | `length > SIZE_MAX - 1 - size` | `-1` | [x] |
| 255 | `strbuffer_append_bytes` | `jsonp_realloc` fails | `-1` | [x] |
| 256 | `strbuffer_pop` | `length == 0` | `'\0'`, length stays 0 | [x] |

## `strconv.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 257 | `jsonp_strtod` | value overflows to ±`HUGE_VAL` with `errno == ERANGE` | `-1`, `*out` untouched | [x] |
| 258 | `jsonp_strtod` | underflow to 0 (`ERANGE` but not `HUGE_VAL`) | `0`, `*out = 0` | [x] |
| 259 | `jsonp_dtostr` | computed length + 3 (+5 for exponent) exceeds `size` | `-1` | [x] |
| 260 | `jsonp_dtostr` | `size == 0` / very small sizes for every precision | `-1` | [x] |

## `utf.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 261 | `utf8_encode` | `codepoint < 0` | `-1` | [x] |
| 262 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | [x] |
| 263 | `utf8_check_first` | continuation byte `0x80..0xBF` | `0` | [x] |
| 264 | `utf8_check_first` | `0xC0` or `0xC1` (overlong lead) | `0` | [x] |
| 265 | `utf8_check_first` | `>= 0xF5` | `0` | [x] |
| 266 | `utf8_check_full` | `size` not 2, 3 or 4 (0, 1, 5, huge) | `0` | [x] |
| 267 | `utf8_check_full` | a non-continuation byte in the sequence | `0` | [x] |
| 268 | `utf8_check_full` | decoded value `> 0x10FFFF` | `0` | [x] |
| 269 | `utf8_check_full` | decoded value in `0xD800..0xDFFF` (surrogate) | `0` | [x] |
| 270 | `utf8_check_full` | overlong encoding for the given size | `0` | [x] |
| 271 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged, `*codepoint` untouched | [x] |
| 272 | `utf8_iterate` | invalid lead byte | `NULL` | [x] |
| 273 | `utf8_iterate` | sequence truncated (`count > bufsize`) | `NULL` | [x] |
| 274 | `utf8_iterate` | `utf8_check_full` fails | `NULL` | [x] |
| 275 | `utf8_check_string` | invalid lead byte anywhere | `0` | [x] |
| 276 | `utf8_check_string` | truncated multi-byte at the end (`count > length - i`) | `0` | [x] |
| 277 | `utf8_check_string` | invalid full sequence | `0` | [x] |

## `hashtable.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 278 | `hashtable_init` | `jsonp_malloc(8 * sizeof bucket)` fails | `-1` | [x] |
| 279 | `hashtable_set` | `hashtable_do_rehash` fails (`jsonp_malloc` for new buckets) | `-1` | [x] |
| 280 | `init_pair` (via `hashtable_set`) | `key_len >= SIZE_MAX - offsetof(pair_t,key)` | `-1` | [x] |
| 281 | `init_pair` | `jsonp_malloc` for the pair fails | `-1` | [x] |
| 282 | `hashtable_get` | key not present | `NULL` | [x] |
| 283 | `hashtable_get` | same bytes but different `key_len` | `NULL` | [x] |
| 284 | `hashtable_del` | key not present | `-1` | [x] |
| 285 | `hashtable_iter` | empty hashtable | `NULL` | [x] |
| 286 | `hashtable_iter_at` | key not present | `NULL` | [x] |
| 287 | `hashtable_iter_next` | iterator already at the last element | `NULL` | [x] |

## `dtoa.c`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 288 | `dtoa_r` | caller buffer `blen` too small for the requested digits | `NULL` | [x] |
| 289 | `dtoa_r` | `mode > 5` (clamped) / negative `ndigits` | same digits as C | [x] |
| 290 | `gethex` | no hex digits after `0x` (`havedig == 0`) | `*sp` not advanced, `rvp` = 0 | [x] |
| 291 | `gethex` | exponent overflow / underflow to 0 or ±Inf | same `U` bits as C | [x] |
| 292 | `strtod__unused` | no convertible characters | `0.0`, `*se == s00` | [x] |
| 293 | `strtod__unused` | overflow / underflow (`ERANGE`) | `±HUGE_VAL` / `0` identical to C | [x] |

## Generic FFI boundary conditions (not tied to one C check)

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| 294 | every `json_*` getter | out-of-range `json_type` tag (8, 255, -1) forged in a `json_t` | same as C `default:` branch | [x] |
| 295 | `json_dumps` / `json_loads` / `json_pack_ex` | out-of-range *flag* bits (e.g. `0xFFFF_FFFF_FFFF_FFFF`) | same as C (bits ignored / masked) | [x] |
| 296 | `jsonp_error_set` | `code` value with no valid enum variant | stored verbatim | [x] |
| 297 | `json_unpack_ex` / `json_pack_ex` | flags outside `JSON_VALIDATE_ONLY\|JSON_STRICT` | ignored, same as C | [x] |
| 298 | `json_array_get` / `json_array_set_new` / `_insert_new` / `_remove` | `index == SIZE_MAX` | `NULL` / `-1` | [x] |
| 299 | `json_stringn` / `json_object_setn_new` / `hashtable_*` | `len == 0` with a non-NULL pointer | accepted (empty string/key) | [x] |
| 300 | `utf8_check_full` / `utf8_check_string` / `strbuffer_append_bytes` | `size == SIZE_MAX` | same rejection as C | [x] |

## `assert()` statements (active in the C build — `NDEBUG` is *not* defined)

`c_src/CMakeLists.txt` sets no `-DNDEBUG`, so every `assert()` in the C sources
aborts the process when it fires.  Each one is unreachable through the public
API; the reason is given below, together with the input class that proves it.
The Rust translation keeps them as `debug_assert!` (compiled out in the release
`.so` that ships), which is behaviourally equivalent for every reachable input.

| # | assert | why it cannot fire for any input reachable through the public API |
|---|--------|-------------------------------------------------------------------|
| A1 | `load.c:175` `assert(count >= 2)` | only reached for `0x80 <= c <= 0xFF`; `utf8_check_first` returns 0 (handled by the `goto out` above) or 2/3/4 for those bytes, never 1 |
| A2 | `load.c:221` `assert(stream->buffer_pos > 0)` | `stream_unget` is only called right after a successful `stream_get`, which always incremented `buffer_pos` |
| A3 | `load.c:223` `assert(stream->buffer[buffer_pos] == c)` | same: `c` is the byte that `stream_get` just consumed from that slot |
| A4 | `load.c:255` `assert(c == d)` | `lex_unget_unsave` pops the byte `lex_get_save` had just pushed |
| A5 | `load.c:278` `assert(str[0] == 'u')` | `decode_unicode_escape` is only called when `*p == 'u'` |
| A6 | `load.c:417` `assert(0)` after `utf8_encode` | the value is in `0..0x10FFFF` by construction (surrogate pairs are combined, lone surrogates rejected earlier), so `utf8_encode` cannot fail |
| A7 | `load.c:442` `assert(0)` in the escape switch | the first pass already rejected every escape letter outside `"\/bfnrt` |
| A8 | `load.c:514` `assert(end == saved_text + length)` | `saved_text` holds exactly one fully validated integer token, so `strtoll` consumes all of it |
| A9 | `dump.c:354` `assert(i == size)` | `i` counts the same hashtable iteration that `json_object_size` reports |
| A10 | `dump.c:364` `assert(value)` | the key was taken from that very object one statement earlier |
| A11 | `strconv.c:53` `assert(end == value + length)` | the strbuffer holds exactly one fully validated real token (and `to_locale` only substitutes the separator, keeping the length) |

Fuzzing (`tests/phase_b_load.rs::cfg102_mutation_fuzz`, 4000 mutated documents ×
4 flag sets, plus the 100-entry `DOCS` corpus through all six decoder entry
points) never triggered an abort in the C library, which is consistent with the
analysis above.

## Rows that are unreachable by construction

| # | row | reason |
|---|-----|--------|
| U1 | 253 / 280 (`init_pair`: `key_len >= SIZE_MAX - offsetof(pair_t, key)`) | `hashtable_set` computes `hash_str(key, key_len)` *before* calling `init_pair`, so any `key_len` large enough to trip the check would already have made `hashlittle()` read that many bytes. The check is dead code. Every *reachable* key length is covered by `cfg19to28_hashtable` and `err278to287_hashtable`. |
| U2 | 191 (`pack_object`: `"NULL object value"`) | the only ways `pack()` returns `NULL` are (a) with an error already recorded — `jsonp_error_vset` then keeps the first message — or (b) via `pack_object_inter`'s `'*'` branch, in which case `valueOptional` is also `'*'` and the `set_error` is skipped. The branch is still exercised (it sets `s->has_error`), which `err194to199_read_string` observes through the `{s:s}`-with-NULL case. |
| U3 | 300 (`utf8_check_string(ptr, SIZE_MAX)`) | the function has no up-front length check and would read `SIZE_MAX` bytes. `utf8_check_full` and `strbuffer_append_bytes` *do* check first and are covered. |

## Corrections applied while building the tests

* Rows 171 / 173: when the failure happens at end-of-input the surrounding
  `error_set()` in `load.c` rewrites `json_error_invalid_syntax` into
  `json_error_premature_end_of_input` (row 182).  Both the EOF and the
  non-EOF form of each trigger are tested.
* `{"a":1,}` reaches the *row 166* branch (`"string or '}' expected"`), not
  row 171, because `lex_scan` returns `}` at the top of the loop.

## Divergences found in the Rust translation and fixed

All three were found by the error-path tests above; the C code is the reference
in every case.

| # | file | divergence | fix |
|---|------|-----------|-----|
| D1 | `src/dump.rs` | `do_dump`, `dump_indent` and `dump_string` each began with `let f = dump.unwrap();`, i.e. the callback pointer was inspected *before* any chunk was emitted.  C only dereferences it at each emission point, so `json_dump_callback(json, NULL, data, flags)` returns `-1` in C for `json == NULL` (row 121/134/135), an out-of-range type tag (row 122), a failed circular-reference check (rows 124/125) and `JSON_EMBED` on an empty container, whereas Rust panicked/aborted. | added `dump_call()`, which matches on the `Option` at each call site; the 24 emission points now go through it |
| D2 | `src/memory.rs` | `jsonp_malloc` / `jsonp_free` / `jsonp_realloc` used `DO_MALLOC.unwrap()` / `DO_FREE.unwrap()`, so a caller that installed NULL hooks with `json_set_alloc_funcs*(NULL, ...)` got a Rust panic instead of C's call through a NULL pointer. | added `call_malloc()` / `call_free()` with the same laziness as C |
| D3 | `src/version.rs` + `Cargo.toml` | `jansson_version_cmp(INT_MIN, …)` computed `2 - major` with Rust's checked subtraction, which panics under `debug-assertions`; `dtoa.rs` likewise relies on C's wrapping unsigned arithmetic in its fast paths. | `version.rs` now uses `wrapping_sub`, and `overflow-checks = false` was added to `[profile.dev]` / `[profile.test]` so *every* profile has C's arithmetic semantics (the release profile already did) |

Two bugs in the *tests* were also found and fixed, and are worth recording
because they are easy traps for anyone extending the suite:

* `json_delete()` must not be used as a stand-alone "release" operation — it
  destroys the value regardless of its refcount.  The tests now use a local
  re-implementation of the `json_decref()` static inline from `jansson.h`.
* With `JSON_VALIDATE_ONLY`, `json_unpack_ex()` consumes *no* value arguments,
  only object keys, so a format carrying value out-parameters may not be
  combined with that flag (see `tests/phase_b_pack.rs::UNPACK_FLAGS` and
  `cfg126_validate_only`).
