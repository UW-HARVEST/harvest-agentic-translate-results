# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/*.c` (every `return -1` / `return NULL` /
`return 0`-as-sentinel / `error_set(...)` / `assert` / explicit range or null
check). One row per distinct rejection.

`json_error_code` values (from `jansson.h`): 0 unknown, 1 out_of_memory,
2 stack_overflow, 3 cannot_open_file, 4 invalid_argument, 5 invalid_utf8,
6 premature_end_of_input, 7 end_of_input_expected, 8 invalid_syntax,
9 invalid_format, 10 wrong_type, 11 null_character, 12 null_value,
13 null_byte_in_key, 14 duplicate_key, 15 numeric_overflow, 16 item_not_found,
17 index_out_of_range.

## utf.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 1 | `utf8_encode` | `codepoint < 0` | `-1`, `*size` untouched |
| 2 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1`, `*size` untouched |
| 3 | `utf8_check_first` | byte in `0x80..=0xBF` (continuation) | `0` |
| 4 | `utf8_check_first` | byte `0xC0` or `0xC1` (overlong ASCII) | `0` |
| 5 | `utf8_check_first` | byte `>= 0xF5` | `0` |
| 6 | `utf8_check_full` | `size` not in {2,3,4} | `0` |
| 7 | `utf8_check_full` | any trailing byte `< 0x80` or `> 0xBF` | `0` |
| 8 | `utf8_check_full` | decoded `value > 0x10FFFF` | `0` |
| 9 | `utf8_check_full` | decoded value in `0xD800..=0xDFFF` (surrogate) | `0` |
| 10 | `utf8_check_full` | overlong: size 2 && v<0x80, size 3 && v<0x800, size 4 && v<0x10000 | `0` |
| 11 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged (NOT NULL), `*codepoint` untouched |
| 12 | `utf8_iterate` | `utf8_check_first(buffer[0]) == 0` | `NULL` |
| 13 | `utf8_iterate` | `count > bufsize` (truncated sequence) | `NULL` |
| 14 | `utf8_iterate` | `utf8_check_full` fails | `NULL` |
| 15 | `utf8_check_string` | any byte fails `utf8_check_first` | `0` |
| 16 | `utf8_check_string` | `count > length - i` (truncated at end) | `0` |
| 17 | `utf8_check_string` | `utf8_check_full` fails mid-string | `0` |

## strbuffer.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 18 | `strbuffer_init` | `jsonp_malloc(16)` returns NULL | `-1` |
| 19 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX/2` | `-1` |
| 20 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | `-1` |
| 21 | `strbuffer_append_bytes` | `strbuff->length > SIZE_MAX - 1 - size` | `-1` |
| 22 | `strbuffer_append_bytes` | `jsonp_realloc` returns NULL | `-1` |
| 23 | `strbuffer_pop` | `strbuff->length == 0` | `'\0'` (0), length stays 0 |

## memory.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 24 | `jsonp_malloc` | `size == 0` | `NULL` (does not call allocator) |
| 25 | `jsonp_free` | `ptr == NULL` | returns without calling free |
| 26 | `jsonp_realloc` | `do_realloc == NULL` (after `json_set_alloc_funcs`) && `newSize == 0` | frees `ptr`, returns `NULL` |
| 27 | `jsonp_strndup` | `jsonp_malloc(len+1)` returns NULL | `NULL` |
| 28 | `json_get_alloc_funcs` | either out-pointer NULL | that one skipped, no crash |
| 29 | `json_get_alloc_funcs2` | any out-pointer NULL | that one skipped, no crash |

## error.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 30 | `jsonp_error_init` | `error == NULL` | no-op |
| 31 | `jsonp_error_set_source` | `error == NULL` or `source == NULL` | no-op |
| 32 | `jsonp_error_set_source` | `strlen(source) >= 80` | source becomes `"..."` + tail; `extra = len-80+4` |
| 33 | `jsonp_error_vset` | `error == NULL` | no-op |
| 34 | `jsonp_error_vset` | `error->text[0] != '\0'` (already set) | no-op, first error preserved |
| 35 | `jsonp_error_vset` | message longer than 158 bytes | truncated, `text[158]='\0'`, `text[159]=code` |

## strconv.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 36 | `jsonp_strtod` | `strtod` returns ±HUGE_VAL and `errno == ERANGE` | `-1`, `*out` untouched |
| 37 | `jsonp_strtod` | `end != value + length` | `assert` → `abort()` |
| 38 | `jsonp_dtostr` | `dtoa_r(...) == NULL` | `-1` |
| 39 | `jsonp_dtostr` | `3 + (vdigits_end - vdigits_start) + (use_exp?5:0) > size` | `-1` |

## hashtable.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 40 | `hashtable_init` | `jsonp_malloc(8 * sizeof(bucket))` returns NULL | `-1` |
| 41 | `hashtable_set` | `init_pair` alloc fails / `key_len` overflow guard | `-1` |
| 42 | `hashtable_get` | key not present | `NULL` |
| 43 | `hashtable_del` | key not present | `-1` |
| 44 | `hashtable_iter` | table empty | `NULL` |
| 45 | `hashtable_iter_at` | key not present | `NULL` |
| 46 | `hashtable_iter_next` | iterator at last element | `NULL` |

## value.c — objects

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 47 | `json_object_size` | `json` NULL or not `JSON_OBJECT` | `0` |
| 48 | `json_object_get` | `key == NULL` | `NULL` |
| 49 | `json_object_get` / `getn` | `json` NULL or not object | `NULL` |
| 50 | `json_object_getn` | `key == NULL` | `NULL` |
| 51 | `json_object_set_new_nocheck` | `key == NULL` | `-1` (decrefs value) |
| 52 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` |
| 53 | `json_object_setn_new_nocheck` | `json` not object | `-1` (decrefs value) |
| 54 | `json_object_setn_new_nocheck` | `json == value` (self-insert) | `-1` (decrefs value) |
| 55 | `json_object_set_new` | `key == NULL` | `-1` |
| 56 | `json_object_setn_new` | `key` not valid UTF-8 | `-1` |
| 57 | `json_object_del` | `key == NULL` | `-1` |
| 58 | `json_object_deln` | `json` not object | `-1` |
| 59 | `json_object_deln` | key not present | `-1` |
| 60 | `json_object_clear` | `json` not object | `-1` |
| 61 | `json_object_update` | `object` or `other` not object | `-1` |
| 62 | `json_object_update_existing` | `object` or `other` not object | `-1` |
| 63 | `json_object_update_missing` | `object` or `other` not object | `-1` |
| 64 | `json_object_update_recursive` | `object` or `other` not object | `-1` |
| 65 | `json_object_update_recursive` | cyclic `other` (loop check hit) | `-1` |
| 66 | `json_object_iter` | `json` not object | `NULL` |
| 67 | `json_object_iter_at` | `key == NULL` or `json` not object | `NULL` |
| 68 | `json_object_iter_next` | `json` not object or `iter == NULL` | `NULL` |
| 69 | `json_object_iter_key` | `iter == NULL` | `NULL` |
| 70 | `json_object_iter_key_len` | `iter == NULL` | `0` |
| 71 | `json_object_iter_value` | `iter == NULL` | `NULL` |
| 72 | `json_object_iter_set_new` | `json` not object, or `iter` NULL, or `value` NULL | `-1` |
| 73 | `json_object_key_to_iter` | `key == NULL` | `NULL` |

## value.c — arrays

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 74 | `json_array_size` | `json` NULL or not `JSON_ARRAY` | `0` |
| 75 | `json_array_get` | `json` not array | `NULL` |
| 76 | `json_array_get` | `index >= entries` (incl. `SIZE_MAX`) | `NULL` |
| 77 | `json_array_set_new` | `value == NULL` | `-1` |
| 78 | `json_array_set_new` | `json` not array, or `json == value` | `-1` |
| 79 | `json_array_set_new` | `index >= entries` | `-1` |
| 80 | `json_array_append_new` | `value == NULL` | `-1` |
| 81 | `json_array_append_new` | `json` not array, or `json == value` | `-1` |
| 82 | `json_array_insert_new` | `value == NULL` | `-1` |
| 83 | `json_array_insert_new` | `json` not array, or `json == value` | `-1` |
| 84 | `json_array_insert_new` | `index > entries` (note `>` not `>=`) | `-1` |
| 85 | `json_array_remove` | `json` not array | `-1` |
| 86 | `json_array_remove` | `index >= entries` | `-1` |
| 87 | `json_array_clear` | `json` not array | `-1` |
| 88 | `json_array_extend` | either arg not array | `-1` |

## value.c — scalars / misc

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 89 | `json_string` / `json_stringn` / `json_string_nocheck` / `json_stringn_nocheck` | `value == NULL` | `NULL` |
| 90 | `json_stringn` | `value` not valid UTF-8 over `len` | `NULL` |
| 91 | `json_string_value` | `json` not string | `NULL` |
| 92 | `json_string_length` | `json` not string | `0` |
| 93 | `json_string_set_nocheck` / `setn_nocheck` | `value == NULL` | `-1` |
| 94 | `json_string_setn_nocheck` | `json` not string | `-1` |
| 95 | `json_string_set` / `setn` | `value == NULL` | `-1` |
| 96 | `json_string_setn` | `value` not valid UTF-8 | `-1` |
| 97 | `json_integer_value` | `json` not integer | `0` |
| 98 | `json_integer_set` | `json` not integer | `-1` |
| 99 | `json_real` | `isnan(value)` | `NULL` |
| 100 | `json_real` | `isinf(value)` (±inf) | `NULL` |
| 101 | `json_real_value` | `json` not real | `0.0` |
| 102 | `json_real_set` | `json` not real, or NaN, or inf | `-1` |
| 103 | `json_number_value` | `json` neither integer nor real (incl. NULL) | `0.0` |
| 104 | `json_equal` | `json1 == NULL` or `json2 == NULL` | `0` |
| 105 | `json_equal` | types differ | `0` |
| 106 | `json_equal` | invalid/out-of-range `type` byte (default arm) | `0` |
| 107 | `json_copy` | `json == NULL` | `NULL` |
| 108 | `json_copy` | invalid `type` byte (default arm) | `NULL` |
| 109 | `do_deep_copy` / `json_deep_copy` | `json == NULL` | `NULL` |
| 110 | `do_deep_copy` | invalid `type` byte (default arm) | `NULL` |
| 111 | `json_deep_copy` | cyclic container (loop check hit) | `NULL` |
| 112 | `json_vsprintf` | `vsnprintf` returns `< 0` | `NULL` |
| 113 | `json_vsprintf` | formatted result not valid UTF-8 | `NULL` |
| 114 | `jsonp_loop_check` | `json` already in `parents` | `-1` |
| 115 | `json_delete` | type is TRUE/FALSE/NULL or invalid (`default`) | returns, no free |

## dump.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 116 | `do_dump` | `json == NULL` | `-1` |
| 117 | `do_dump` (integer) | `snprintf` size `<0` or `>= 25` | `-1` |
| 118 | `do_dump` (real) | `jsonp_dtostr < 0` | `-1` |
| 119 | `do_dump` (array/object) | circular reference (loop check) | `-1` |
| 120 | `do_dump` | callback returns non-zero at any point | `-1` |
| 121 | `dump_string` | `utf8_iterate` returns NULL (invalid UTF-8 in string) | `-1` |
| 122 | `json_dump_callback` | top-level not array/object and `JSON_ENCODE_ANY` unset | `-1` |
| 123 | `json_dumps` | `json_dump_callback` fails | `NULL` |
| 124 | `json_dumps` | `json == NULL` | `NULL` |
| 125 | `json_dumpb` | dump fails / `json == NULL` | `0` |
| 126 | `json_dumpb` | `size` too small for output | `0` (partial buffer written) |
| 127 | `json_dumpf` | `json == NULL` | `-1` |
| 128 | `json_dump_file` | `fopen(path,"w")` fails | `-1` |
| 129 | `json_dumpfd` | `write` short/fails, or bad fd | `-1` |
| 130 | `json_dump_callback` | `callback == NULL` | crash-free path not guaranteed; C derefs → tested only with non-NULL |

## load.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 131 | `json_loads` | `string == NULL` | `NULL`, code `4` invalid_argument, text `"wrong arguments"` |
| 132 | `json_loadb` | `buffer == NULL` | `NULL`, code `4` `"wrong arguments"` |
| 133 | `json_loadf` | `input == NULL` | `NULL`, code `4` `"wrong arguments"` |
| 134 | `json_loadfd` | `input < 0` | `NULL`, code `4` `"wrong arguments"` |
| 135 | `json_load_file` | `path == NULL` | `NULL`, code `4` `"wrong arguments"` |
| 136 | `json_load_file` | `fopen` fails (nonexistent path) | `NULL`, code `3` cannot_open_file, text `"unable to open <path>: <strerror>"` |
| 137 | `json_load_callback` | `callback == NULL` | `NULL`, code `4` `"wrong arguments"` |
| 138 | `stream_get` | invalid UTF-8 byte in input | code `5` invalid_utf8, text `"unable to decode byte 0x%x"` |
| 139 | `lex_scan_string` | EOF inside string literal | code `6` premature_end_of_input |
| 140 | `lex_scan_string` | literal newline inside string | code `8` invalid_syntax, `"unexpected newline"` |
| 141 | `lex_scan_string` | control char `< 0x20` inside string | code `8`, `"control character 0x%x"` |
| 142 | `lex_scan_string` | invalid escape char (e.g. `"\q"`) | code `8`, `"invalid escape"` |
| 143 | `lex_scan_string` | `\u` with non-hex / short digits | code `8`, `"invalid Unicode escape '%.6s'"` |
| 144 | `lex_scan_string` | lone high surrogate not followed by `\u` low | code `8`, `"invalid Unicode '\\u...'"` |
| 145 | `lex_scan_string` | lone low surrogate `\uDC00` | code `8`, `"invalid Unicode '\\u...'"` |
| 146 | `lex_scan_string` | `\u0000` escape | code `8`, `"\\u0000 is not allowed"` |
| 147 | `lex_scan_number` | integer literal overflows `json_int_t`, negative | code `15` numeric_overflow, `"too big negative integer"` |
| 148 | `lex_scan_number` | integer literal overflows `json_int_t`, positive | code `15`, `"too big integer"` |
| 149 | `lex_scan_number` | real literal overflows double | code `15`, `"real number overflow"` |
| 150 | `lex_scan_number` | leading zero followed by digit (`01`) | token split → syntax error at top level |
| 151 | `lex_scan_number` | `-` with no digits, or `1.` / `1e` with no digits | `-1` (invalid token) → code `8` |
| 152 | `parse_object` | key not string and not `}` | code `8`, `"string or '}' expected"` |
| 153 | `parse_object` | NUL byte inside object key | code `13` null_byte_in_key, `"NUL byte in object key not supported"` |
| 154 | `parse_object` | duplicate key with `JSON_REJECT_DUPLICATES` | code `14` duplicate_key, `"duplicate object key"` |
| 155 | `parse_object` | missing `:` after key | code `8`, `"':' expected"` |
| 156 | `parse_object` | missing `}` / bad separator | code `8`, `"'}' expected"` |
| 157 | `parse_array` | missing `]` / bad separator | code `8`, `"']' expected"` |
| 158 | `parse_value` | nesting depth `> 2048` | code `2` stack_overflow, `"maximum parsing depth reached"` |
| 159 | `parse_value` | `\u0000` decoded into string without `JSON_ALLOW_NUL` | code `11` null_character, `"\\u0000 is not allowed without JSON_ALLOW_NUL"` |
| 160 | `parse_value` | `TOKEN_INVALID` | code `8`, `"invalid token"` |
| 161 | `parse_value` | unexpected token (e.g. `}` at value position) | code `8`, `"unexpected token"` |
| 162 | `parse_json` | top-level scalar without `JSON_DECODE_ANY` | code `8`, `"'[' or '{' expected"` |
| 163 | `parse_json` | trailing garbage without `JSON_DISABLE_EOF_CHECK` | code `7` end_of_input_expected, `"end of file expected near '%s'"` |
| 164 | `error_set` | error at EOF with `invalid_syntax` and no context | code rewritten to `6` premature_end_of_input |
| 165 | `json_loads` | empty string `""` | `NULL`, code `6` premature_end_of_input |

## pack_unpack.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 166 | `json_vpack_ex` | `fmt == NULL` | `NULL`, code `4` invalid_argument, `"NULL or empty format string"`, source `"<format>"` |
| 167 | `json_vpack_ex` | `*fmt == '\0'` | `NULL`, code `4`, same text |
| 168 | `json_vpack_ex` | garbage after complete format (e.g. `"[]]"`) | `NULL`, code `9` invalid_format, `"Garbage after format string"` |
| 169 | `pack` | unknown format char (e.g. `'x'`) | `NULL`, code `9`, `"Unexpected format character 'x'"` |
| 170 | `pack_object` | format ends after `{` | `NULL`, code `9`, `"Unexpected end of format string"` |
| 171 | `pack_object` | non-`s` key format (e.g. `"{i:i}"`) | `NULL`, code `9`, `"Expected format 's', got 'i'"` |
| 172 | `pack_object` | NULL object value and no `*` | `NULL`, code `12` null_value, `"NULL object value"` |
| 173 | `pack_array` | format ends after `[` | `NULL`, code `9`, `"Unexpected end of format string"` |
| 174 | `read_string` (`s`) | NULL `const char*` arg, not optional | `NULL`, code `12` null_value, `"NULL string"`, source `"<args>"` |
| 175 | `read_string` (`s`) | invalid UTF-8 arg | `NULL`, code `5` invalid_utf8, `"Invalid UTF-8 string"` |
| 176 | `read_string` | `#`/`%`/`+` combined with optional `?` | `NULL`, code `9`, `"Cannot use '%c' on optional strings"` |
| 177 | `pack_object_inter` (`O`/`o`) | NULL `json_t*` and no `?`/`*` | `NULL`, code `12`, `"NULL object"` |
| 178 | `pack_real` (`f`) | value is NaN/inf (json_real_set fails) | `NULL`, code `15` numeric_overflow, `"Invalid floating point value"` |
| 179 | `json_vunpack_ex` | `root == NULL` | `-1`, code `12` null_value, `"NULL root value"`, source `"<root>"` |
| 180 | `json_vunpack_ex` | `fmt == NULL` or empty | `-1`, code `4`, `"NULL or empty format string"` |
| 181 | `json_vunpack_ex` | garbage after format | `-1`, code `9`, `"Garbage after format string"` |
| 182 | `unpack_object` | root not object | `-1`, code `10` wrong_type, `"Expected object, got <type>"` |
| 183 | `unpack_object` | non-`s` key format | `-1`, code `9`, `"Expected format 's', got '%c'"` |
| 184 | `unpack_object` | format ends after `{` | `-1`, code `9`, `"Unexpected end of format string"` |
| 185 | `unpack_object` | NULL key va_arg | `-1`, code `12`, `"NULL object key"` |
| 186 | `unpack_object` | key missing and not optional | `-1`, code `16` item_not_found, `"Object item not found: <key>"` |
| 187 | `unpack_object` | `!` strict and object has extra keys | `-1`, code `7` end_of_input_expected, `"N object item(s) left unpacked: ..."` |
| 188 | `unpack_object` | chars after `!`/`*` before `}` | `-1`, code `9`, `"Expected '}' after '%c', got '%c'"` |
| 189 | `unpack_array` | root not array | `-1`, code `10`, `"Expected array, got <type>"` |
| 190 | `unpack_array` | array shorter than format | `-1`, code `17` index_out_of_range, `"Array index N out of range"` |
| 191 | `unpack_array` | `!` strict and array longer than format | `-1`, code `7`, `"N array item(s) left unpacked"` |
| 192 | `unpack_array` | chars after `!`/`*` before `]` | `-1`, code `9`, `"Expected ']' after '%c', got '%c'"` |
| 193 | `unpack_array` | format char not in value-starters set | `-1`, code `9`, `"Unexpected format character '%c'"` |
| 194 | `unpack` `s` | root not string | `-1`, code `10`, `"Expected string, got <type>"` |
| 195 | `unpack` `s` | NULL `const char**` target, not VALIDATE_ONLY | `-1`, code `12`, `"NULL string argument"` |
| 196 | `unpack` `s%` | NULL `size_t*` length target | `-1`, code `12`, `"NULL string length argument"` |
| 197 | `unpack` `i`/`I` | root not integer | `-1`, code `10`, `"Expected integer, got <type>"` |
| 198 | `unpack` `b` | root not boolean | `-1`, code `10`, `"Expected true or false, got <type>"` |
| 199 | `unpack` `f` | root not real | `-1`, code `10`, `"Expected real, got <type>"` |
| 200 | `unpack` `F` | root neither real nor integer | `-1`, code `10`, `"Expected real or integer, got <type>"` |
| 201 | `unpack` `n` | root not null | `-1`, code `10`, `"Expected null, got <type>"` |
| 202 | `unpack` | unknown format char | `-1`, code `9`, `"Unexpected format character '%c'"` |
| 203 | `json_unpack` (no error arg) | any of the above | same return value, no error struct written |

## Generic FFI boundary cases (not in a single C branch, but valid inputs)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 204 | all `json_*` getters/predicates | `json == NULL` | type-check macros short-circuit → sentinel (0 / NULL / -1) |
| 205 | `json_typeof`-driven `switch`es | `json_t.type` set to out-of-range int (e.g. 99, -1) via forged struct | `default` arm: `json_equal`→0, `json_copy`→NULL, `do_deep_copy`→NULL, `json_delete`→no-op, `do_dump`→-1 |
| 206 | `json_loads`/`json_dumps` | `flags` with unknown high bits set | unknown bits ignored |
| 207 | `json_dumpb` | `size == 0` with non-NULL buffer | `0` |
| 208 | `json_loadb` | `buflen == 0` | `NULL`, code `6` premature_end_of_input |
| 209 | `jansson_version_cmp` | any int triple incl. negatives / INT_MIN | `2-major`, else `15-minor`, else `0-micro` (int wrap as C does) |

## Status

**All 209 rows have a passing error-path differential test.** Tests live in
`translation/tests/phase_c_errors.rs`; each asserts the same return
value/sentinel *and*, where the C sets one, the byte-identical `json_error_t`
(all 160 raw `text` bytes including the trailing error-code byte, all 80 raw
`source` bytes, plus `line`, `column`, `position`).

| rows | test |
|---|---|
| 1-17 | `err_rows_01_17_utf` |
| 18-23 | `err_rows_18_23_strbuffer` |
| 24-29 | `err_rows_24_29_memory` |
| 30-35 | `err_rows_30_35_error` |
| 36-39 | `err_rows_36_39_strconv` |
| 40-46 | `err_rows_40_46_hashtable` |
| 47-73 | `err_rows_47_73_object_rejections` |
| 74-88 | `err_rows_74_88_array_rejections` |
| 65, 89-115, 205 | `err_rows_89_115_scalar_and_misc_rejections` |
| 116-130, 207 | `err_rows_116_130_dump` |
| 131-137, 208 | `err_rows_131_137_load_null_args` |
| 136 | `err_row_136_load_file_cannot_open` |
| 138-165 | `err_rows_138_165_parse_errors` |
| 158 | `err_row_158_stack_overflow` |
| 166-178 | `err_rows_166_178_pack` |
| 179-203 | `err_rows_179_203_unpack` |
| 204, 206, 209 | `err_rows_204_209_generic_boundary` |
| error-text truncation / raw bytes | `err_long_and_raw_error_texts` |

Beyond the table, the generic boundaries are covered: NULL pointers into every
public entry point, zero and oversized lengths (`0`, `SIZE_MAX`, `SIZE_MAX-1`),
one-past-range indices, and **out-of-range `json_type` values forged across the
FFI boundary** (`type` = 8, 9, 99, −1, `INT_MAX`, `INT_MIN`) fed to
`json_equal`, `json_copy`, `json_deep_copy`, `json_dumps` and `json_delete` so
every `default:` arm is exercised in both libraries.

## Divergence found and fixed

`pack_unpack.c`'s `set_error()` calls `jsonp_error_vset()` directly, i.e. it
formats straight into `error->text` with `vsnprintf`. The Rust `set_error!`
macro rendered the message into a scratch buffer and then re-emitted it through
`jsonp_error_set_str` (`"%s"`), which stops at the first NUL. When the format
string ends, `"Unexpected format character '%c'"` / `"Expected format 's', got
'%c'"` receive the NUL terminator as `%c`, so the C text is
`Unexpected format character '\0'\0` while the Rust text was cut to
`Unexpected format character '\0`.

Fix: added `error::jsonp_error_set_raw()`, which copies the rendered bytes
verbatim while reproducing `vsnprintf(text, 159, …)` truncation semantics, and
pointed `pack_unpack.rs`'s `set_error!` at it (passing the `snprintf` return
value so over-long messages truncate at 158 bytes exactly as C does).

Note that `load.c`'s `error_set()` legitimately *does* round-trip through
`"%s"`, so its truncation-at-NUL behaviour is correct and was left alone.
