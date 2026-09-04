# ERRORS.md — error / rejection surface of the C library

Derived mechanically by grepping every `return -1`, `return NULL`, `return 0` used as a
rejection, every `error_set` / `set_error` / `jsonp_error_set` call site, every `assert`,
and every explicit range / null / min-max check in `c_src/src/*.c`.

One row per *distinct* rejection branch. `[x]` = a differential test in
`translation/tests/` constructs exactly that condition, calls **both** the C `.so` and the
Rust `.so`, and asserts the identical error code / sentinel (not merely "both failed").

Test files: `tests/errors_value.rs`, `tests/errors_utf_str.rs`, `tests/errors_load.rs`,
`tests/errors_dump.rs`, `tests/errors_pack.rs`, `tests/errors_alloc.rs`,
`tests/errors_enum_oob.rs`.

## value.c — objects

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `json_object_size` | `json` is NULL | `0` | [x] |
| 2 | `json_object_size` | `json` is not JSON_OBJECT (array/string/int/real/true/false/null) | `0` | [x] |
| 3 | `json_object_get` | `key == NULL` | `NULL` | [x] |
| 4 | `json_object_getn` | `key == NULL` | `NULL` | [x] |
| 5 | `json_object_getn` | `json` NULL or not an object | `NULL` | [x] |
| 6 | `json_object_getn` | key absent from a valid object | `NULL` | [x] |
| 7 | `json_object_set_new_nocheck` | `key == NULL` (value is decref'd) | `-1` | [x] |
| 8 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` | [x] |
| 9 | `json_object_setn_new_nocheck` | `key == NULL` | `-1` | [x] |
| 10 | `json_object_setn_new_nocheck` | `json` NULL / not an object | `-1` | [x] |
| 11 | `json_object_setn_new_nocheck` | `json == value` (self-insert) | `-1` | [x] |
| 12 | `json_object_setn_new_nocheck` | `hashtable_set` fails — OOM in `init_pair`/`hashtable_do_rehash` | `-1` | [x] |
| 13 | `json_object_set_new` | `key == NULL` | `-1` | [x] |
| 14 | `json_object_setn_new` | `key == NULL` | `-1` | [x] |
| 15 | `json_object_setn_new` | `!utf8_check_string(key,key_len)` — invalid UTF-8 key | `-1` | [x] |
| 16 | `json_object_del` | `key == NULL` | `-1` | [x] |
| 17 | `json_object_deln` | `key == NULL` | `-1` | [x] |
| 18 | `json_object_deln` | `json` NULL / not an object | `-1` | [x] |
| 19 | `json_object_deln` | key not present (`hashtable_do_del` finds no pair) | `-1` | [x] |
| 20 | `json_object_clear` | `json` NULL / not an object | `-1` | [x] |
| 21 | `json_object_update` | `object` not an object | `-1` | [x] |
| 22 | `json_object_update` | `other` not an object | `-1` | [x] |
| 23 | `json_object_update_existing` | either arg not an object | `-1` | [x] |
| 24 | `json_object_update_missing` | either arg not an object | `-1` | [x] |
| 25 | `json_object_update_recursive` | either arg not an object | `-1` | [x] |
| 26 | `json_object_update_recursive` | cycle: `other` reachable from itself (`jsonp_loop_check` hit) | `-1` | [x] |
| 27 | `json_object_iter` | `json` NULL / not an object | `NULL` | [x] |
| 28 | `json_object_iter` | empty object (no pairs) | `NULL` | [x] |
| 29 | `json_object_iter_at` | `key == NULL` | `NULL` | [x] |
| 30 | `json_object_iter_at` | `json` not an object | `NULL` | [x] |
| 31 | `json_object_iter_at` | key absent | `NULL` | [x] |
| 32 | `json_object_iter_next` | `json` not an object | `NULL` | [x] |
| 33 | `json_object_iter_next` | `iter == NULL` | `NULL` | [x] |
| 34 | `json_object_iter_next` | iter is the last pair | `NULL` | [x] |
| 35 | `json_object_iter_key` | `iter == NULL` | `NULL` | [x] |
| 36 | `json_object_iter_key_len` | `iter == NULL` | `0` | [x] |
| 37 | `json_object_iter_value` | `iter == NULL` | `NULL` | [x] |
| 38 | `json_object_iter_set_new` | `json` not an object | `-1` | [x] |
| 39 | `json_object_iter_set_new` | `iter == NULL` | `-1` | [x] |
| 40 | `json_object_iter_set_new` | `value == NULL` | `-1` | [x] |
| 41 | `json_object_key_to_iter` | `key == NULL` | `NULL` | [x] |

## value.c — arrays

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 42 | `json_array_size` | not an array / NULL | `0` | [x] |
| 43 | `json_array_get` | not an array / NULL | `NULL` | [x] |
| 44 | `json_array_get` | `index >= entries` (incl. `SIZE_MAX`, and `index==entries`) | `NULL` | [x] |
| 45 | `json_array_set_new` | `value == NULL` | `-1` | [x] |
| 46 | `json_array_set_new` | not an array | `-1` | [x] |
| 47 | `json_array_set_new` | `json == value` | `-1` | [x] |
| 48 | `json_array_set_new` | `index >= entries` | `-1` | [x] |
| 49 | `json_array_append_new` | `value == NULL` | `-1` | [x] |
| 50 | `json_array_append_new` | not an array | `-1` | [x] |
| 51 | `json_array_append_new` | `json == value` | `-1` | [x] |
| 52 | `json_array_append_new` | `json_array_grow` fails (realloc NULL) | `-1` | [x] |
| 53 | `json_array_insert_new` | `value == NULL` | `-1` | [x] |
| 54 | `json_array_insert_new` | not an array | `-1` | [x] |
| 55 | `json_array_insert_new` | `json == value` | `-1` | [x] |
| 56 | `json_array_insert_new` | `index > entries` (note: `>` not `>=`) | `-1` | [x] |
| 57 | `json_array_remove` | not an array | `-1` | [x] |
| 58 | `json_array_remove` | `index >= entries` | `-1` | [x] |
| 59 | `json_array_clear` | not an array | `-1` | [x] |
| 60 | `json_array_extend` | `json` not an array | `-1` | [x] |
| 61 | `json_array_extend` | `other_json` not an array | `-1` | [x] |
| 62 | `json_array_extend` | `json_array_grow` fails | `-1` | [x] |

## value.c — strings / numbers / misc

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 63 | `json_string` | `value == NULL` | `NULL` | [x] |
| 64 | `json_stringn` | `value == NULL` | `NULL` | [x] |
| 65 | `json_stringn` | `!utf8_check_string(value,len)` | `NULL` | [x] |
| 66 | `json_string_nocheck` | `value == NULL` | `NULL` | [x] |
| 67 | `json_stringn_nocheck` | `value == NULL` (`string_create` null check) | `NULL` | [x] |
| 68 | `jsonp_stringn_nocheck_own` | `value == NULL` | `NULL` | [x] |
| 69 | `json_string_value` | not a string / NULL | `NULL` | [x] |
| 70 | `json_string_length` | not a string / NULL | `0` | [x] |
| 71 | `json_string_set` | `value == NULL` | `-1` | [x] |
| 72 | `json_string_set_nocheck` | `value == NULL` | `-1` | [x] |
| 73 | `json_string_setn_nocheck` | `json` not a string | `-1` | [x] |
| 74 | `json_string_setn_nocheck` | `value == NULL` | `-1` | [x] |
| 75 | `json_string_setn` | `value == NULL` | `-1` | [x] |
| 76 | `json_string_setn` | invalid UTF-8 value | `-1` | [x] |
| 77 | `json_integer_value` | not an integer / NULL | `0` | [x] |
| 78 | `json_integer_set` | not an integer / NULL | `-1` | [x] |
| 79 | `json_real` | `isnan(value)` | `NULL` | [x] |
| 80 | `json_real` | `isinf(value)` (both signs) | `NULL` | [x] |
| 81 | `json_real_value` | not a real / NULL | `0.0` | [x] |
| 82 | `json_real_set` | not a real / NULL | `-1` | [x] |
| 83 | `json_real_set` | NaN value | `-1` | [x] |
| 84 | `json_real_set` | ±Inf value | `-1` | [x] |
| 85 | `json_number_value` | neither integer nor real (incl. NULL) | `0.0` | [x] |
| 86 | `json_equal` | `json1 == NULL` | `0` | [x] |
| 87 | `json_equal` | `json2 == NULL` | `0` | [x] |
| 88 | `json_equal` | differing `json_typeof` | `0` | [x] |
| 89 | `json_equal` | out-of-range `type` byte in a forged `json_t` (default branch) | `0` | [x] |
| 90 | `json_copy` | `json == NULL` | `NULL` | [x] |
| 91 | `json_copy` | out-of-range `type` (default branch) | `NULL` | [x] |
| 92 | `json_deep_copy` | `json == NULL` | `NULL` | [x] |
| 93 | `do_deep_copy` | `json == NULL` | `NULL` | [x] |
| 94 | `do_deep_copy` | out-of-range `type` (default branch) | `NULL` | [x] |
| 95 | `json_deep_copy` | self-referencing array/object (loop check) | `NULL` | [x] |
| 96 | `json_delete` | `json == NULL` | no-op (no crash) | [x] |
| 97 | `json_delete` | out-of-range `type` (default: return without free) | no-op | [x] |
| 98 | `jsonp_loop_check` | key already present in `parents` | `-1` | [x] |

## utf.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 99 | `utf8_encode` | `codepoint < 0` | `-1` | [x] |
| 100 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | [x] |
| 101 | `utf8_check_first` | byte in `0x80..=0xBF` (continuation) | `0` | [x] |
| 102 | `utf8_check_first` | byte `0xC0` or `0xC1` (overlong ASCII) | `0` | [x] |
| 103 | `utf8_check_first` | byte `>= 0xF5` | `0` | [x] |
| 104 | `utf8_check_full` | `size` not in `{2,3,4}` (0,1,5,...) | `0` | [x] |
| 105 | `utf8_check_full` | a byte at 1..size-1 is `< 0x80` or `> 0xBF` | `0` | [x] |
| 106 | `utf8_check_full` | decoded `value > 0x10FFFF` | `0` | [x] |
| 107 | `utf8_check_full` | decoded value in `0xD800..=0xDFFF` (surrogate) | `0` | [x] |
| 108 | `utf8_check_full` | overlong: size 2 &lt;0x80, size 3 &lt;0x800, size 4 &lt;0x10000 | `0` | [x] |
| 109 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged (not NULL) | [x] |
| 110 | `utf8_iterate` | `utf8_check_first == 0` | `NULL` | [x] |
| 111 | `utf8_iterate` | `count > bufsize` (truncated sequence) | `NULL` | [x] |
| 112 | `utf8_iterate` | `utf8_check_full` fails | `NULL` | [x] |
| 113 | `utf8_check_string` | leading byte rejected (`count == 0`) | `0` | [x] |
| 114 | `utf8_check_string` | `count > length - i` (sequence runs past end) | `0` | [x] |
| 115 | `utf8_check_string` | `utf8_check_full` fails mid-string | `0` | [x] |

## strbuffer.c / hashtable.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 116 | `strbuffer_init` | `jsonp_malloc` returns NULL | `-1` | [x] |
| 117 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX/2` | `-1` | [x] |
| 118 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` (i.e. `SIZE_MAX`) | `-1` | [x] |
| 119 | `strbuffer_append_bytes` | `length > SIZE_MAX - 1 - size` | `-1` | [x] |
| 120 | `strbuffer_append_bytes` | `jsonp_realloc` returns NULL | `-1` | [x] |
| 121 | `strbuffer_pop` | `length == 0` | `'\0'` | [x] |
| 122 | `hashtable_init` | bucket `jsonp_malloc` fails | `-1` | [x] |
| 123 | `hashtable_del` | key not found | `-1` | [x] |
| 124 | `hashtable_get` | key not found | `NULL` | [x] |
| 125 | `hashtable_iter_at` | key not found | `NULL` | [x] |
| 126 | `hashtable_iter` | empty table | `NULL` | [x] |
| 127 | `hashtable_set` | `key_len >= SIZE_MAX - offsetof(pair_t,key)` (`init_pair` overflow guard) | `-1` | **[n/a]** unreachable without UB: `hashtable_set` computes `hash_str(key, key_len)` *before* `init_pair`, so the C reads `key_len` bytes and faults first |
| 128 | `hashtable_set` | `hashtable_do_rehash` malloc fails | `-1` | [x] |

## memory.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 129 | `jsonp_malloc` | `size == 0` | `NULL` (does not call `do_malloc`) | [x] |
| 130 | `jsonp_free` | `ptr == NULL` | no-op (does not call `do_free`) | [x] |
| 131 | `jsonp_realloc` | `do_realloc == NULL` (after `json_set_alloc_funcs`) and `newSize == 0` | `NULL` | [x] |
| 132 | `jsonp_realloc` | `do_realloc == NULL`, `do_malloc` fails | `NULL` | [x] |
| 133 | `json_get_alloc_funcs` | NULL out-pointers | no write, no crash | [x] |
| 134 | `json_get_alloc_funcs2` | NULL out-pointers | no write, no crash | [x] |

## error.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 135 | `jsonp_error_init` | `error == NULL` | no-op | [x] |
| 136 | `jsonp_error_init` | `source == NULL` | `source[0]='\0'`, line/col `-1` | [x] |
| 137 | `jsonp_error_set_source` | `error == NULL` or `source == NULL` | no-op | [x] |
| 138 | `jsonp_error_set_source` | `strlen(source) >= JSON_ERROR_SOURCE_LENGTH` (80) | `"..."` + tail truncation | [x] |
| 139 | `jsonp_error_vset` / `jsonp_error_set` | `error == NULL` | no-op | [x] |
| 140 | `jsonp_error_vset` / `jsonp_error_set` | `error->text[0] != '\0'` (already set) | no-op, keeps first error | [x] |
| 141 | `jsonp_error_set` | `msg` longer than `JSON_ERROR_TEXT_LENGTH-2` | truncated at 158 + code byte at 159 | [x] |
| 142 | `json_error_code` | reads `text[159]` — out-of-range code byte stored | round-trips the raw byte | [x] |

## strconv.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 143 | `jsonp_strtod` | value overflows to ±HUGE_VAL with `errno==ERANGE` (`1e999`) | `-1` | [x] |
| 144 | `jsonp_dtostr` | `size` too small for sign+digits+`.`+NUL(+exp) | `-1` | [x] |
| 145 | `jsonp_dtostr` | `dtoa_r` returns NULL (buffer too short) | `-1` | [x] |

## dump.c

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 146 | `json_dump_callback` | `json` is not array/object and `JSON_ENCODE_ANY` not set (incl. NULL) | `-1` | [x] |
| 147 | `json_dump_callback` | `hashtable_init(&parents_set)` fails | `-1` | [x] |
| 148 | `do_dump` | `json == NULL` with `JSON_ENCODE_ANY` | `-1` | [x] |
| 149 | `do_dump` | out-of-range `type` byte with `JSON_ENCODE_ANY` (default branch) | `-1` | [x] |
| 150 | `do_dump` | JSON_REAL whose `jsonp_dtostr` fails (precision 31 → buffer too short) | `-1` | [x] |
| 151 | `do_dump` | array/object circular reference (`jsonp_loop_check`) | `-1` | [x] |
| 152 | `do_dump` (JSON_SORT_KEYS) | `jsonp_malloc(size*sizeof(struct key_len))` fails | `-1` | [x] |
| 153 | `dump_string` | invalid UTF-8 inside a string built with `*_nocheck` (`utf8_iterate` → NULL) | `-1` | [x] |
| 154 | `do_dump` | user `dump` callback returns non-zero | `-1` propagated | [x] |
| 155 | `json_dumps` | `json_dump_callback` fails | `NULL` | [x] |
| 156 | `json_dumps` | `strbuffer_init` fails | `NULL` | [x] |
| 157 | `json_dumpb` | `json_dump_callback` fails | `0` | [x] |
| 158 | `json_dumpb` | `size` smaller than needed | returns needed length, buffer NOT written | [x] |
| 159 | `json_dumpf` | `fwrite` short write (`dump_to_file`) | `-1` | [x] |
| 160 | `json_dumpfd` | `write()` returns != size (bad fd, e.g. `-1`) | `-1` | [x] |
| 161 | `json_dump_file` | `fopen(path,"w")` fails (unwritable path) | `-1` | [x] |
| 162 | `json_dump_file` | NULL/invalid json with unwritable path (fopen checked first) | `-1` | [x] |

## load.c — decoder error codes

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 163 | `json_loads` | `string == NULL` | NULL, `json_error_invalid_argument`, line -1 | [x] |
| 164 | `json_loadb` | `buffer == NULL` | NULL, `json_error_invalid_argument` | [x] |
| 165 | `json_loadf` | `input == NULL` | NULL, `json_error_invalid_argument` | [x] |
| 166 | `json_loadfd` | `input < 0` | NULL, `json_error_invalid_argument` | [x] |
| 167 | `json_load_file` | `path == NULL` | NULL, `json_error_invalid_argument` | [x] |
| 168 | `json_load_file` | `fopen` fails | NULL, `json_error_cannot_open_file`, text `"unable to open %s: %s"` | [x] |
| 169 | `json_load_callback` | `callback == NULL` | NULL, `json_error_invalid_argument` | [x] |
| 170 | `json_load_callback` | callback returns `0` immediately (EOF) | NULL, premature end of input | [x] |
| 171 | `json_load_callback` | callback returns `(size_t)-1` | NULL, premature end of input | [x] |
| 172 | `parse_json` | top level not `[`/`{` and no `JSON_DECODE_ANY` (`"1"`, `"\"s\""`, `"true"`) | NULL, `json_error_invalid_syntax`, `"'[' or '{' expected"` | [x] |
| 173 | `parse_json` | trailing garbage without `JSON_DISABLE_EOF_CHECK` (`"[] x"`) | NULL, `json_error_end_of_input_expected` | [x] |
| 174 | `parse_value` | `lex->depth > JSON_PARSER_MAX_DEPTH` (2049 nested `[`) | NULL, `json_error_stack_overflow` | [x] |
| 175 | `parse_value` | `TOKEN_INVALID` (`"[tru]"`, `"[@]"`) | NULL, `json_error_invalid_syntax`, `"invalid token"` | [x] |
| 176 | `parse_value` | unexpected token (`"[,]"`, `"[:]"`, `"[}]"`) | NULL, `json_error_invalid_syntax`, `"unexpected token"` | [x] |
| 177 | `parse_value` | `\u0000` in string without `JSON_ALLOW_NUL` | NULL, `json_error_null_character` | [x] |
| 178 | `parse_object` | non-string, non-`}` where key expected (`"{1:2}"`) | NULL, `json_error_invalid_syntax`, `"string or '}' expected"` | [x] |
| 179 | `parse_object` | NUL byte inside key (`"{\"a\\u0000b\":1}"`) | NULL, `json_error_null_byte_in_key` | [x] |
| 180 | `parse_object` | duplicate key with `JSON_REJECT_DUPLICATES` | NULL, `json_error_duplicate_key` | [x] |
| 181 | `parse_object` | missing `:` (`"{\"a\" 1}"`) | NULL, `json_error_invalid_syntax`, `"':' expected"` | [x] |
| 182 | `parse_object` | missing closing `}` (`"{\"a\":1"`, `"{\"a\":1,"`) | NULL, `json_error_invalid_syntax` / premature end | [x] |
| 183 | `parse_array` | missing closing `]` (`"[1"`, `"[1,"`) | NULL, `json_error_invalid_syntax` / premature end | [x] |
| 184 | `lex_scan_string` | EOF inside string (`"[\"abc"`) | NULL, `json_error_premature_end_of_input` | [x] |
| 187b | `lex_scan_string` | EOF immediately after a backslash (`"[\"abc\\"`) — the escape switch is reached with `c == EOF`, so this is `"invalid escape"`, **not** a premature end | NULL, `json_error_invalid_syntax` | [x] |
| 185 | `lex_scan_string` | raw control char `< 0x20` in string | NULL, `json_error_invalid_syntax`, `"control character 0x%x"` | [x] |
| 186 | `lex_scan_string` | raw newline in string | NULL, `json_error_invalid_syntax`, `"unexpected newline"` | [x] |
| 187 | `lex_scan_string` | bad escape char (`"[\"\\x\"]"`) | NULL, `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 188 | `lex_scan_string` | `\u` with a non-hex digit (`"[\"\\uZZZZ\"]"`) | NULL, `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| 189 | `lex_scan_string` | lone high surrogate `\uD834` with no `\u` following | NULL, `"invalid Unicode '\\uD834'"` | [x] |
| 190 | `lex_scan_string` | high surrogate followed by non-low-surrogate `\uD834\u0041` | NULL, `"invalid Unicode '\\uD834\\u0041'"` | [x] |
| 191 | `lex_scan_string` | lone low surrogate `\uDD1E` | NULL, `"invalid Unicode '\\uDD1E'"` | [x] |
| 192 | `lex_scan_string` | `t = jsonp_malloc(saved_text.length+1)` fails | NULL (TOKEN_INVALID) | [x] |
| 193 | `lex_scan_number` | leading zeros (`"[01]"`) | NULL, invalid token/syntax | [x] |
| 194 | `lex_scan_number` | lone `-` (`"[-]"`) | NULL, invalid token | [x] |
| 195 | `lex_scan_number` | `.` with no following digit (`"[1.]"`, `"[1.e2]"`) | NULL, invalid token | [x] |
| 196 | `lex_scan_number` | exponent with no digits (`"[1e]"`, `"[1e+]"`) | NULL, invalid token | [x] |
| 197 | `lex_scan_number` | integer out of `json_int_t` range → `errno==ERANGE`, positive | NULL, `json_error_numeric_overflow`, `"too big integer"` | [x] |
| 198 | `lex_scan_number` | integer out of range, negative | NULL, `json_error_numeric_overflow`, `"too big negative integer"` | [x] |
| 199 | `lex_scan_number` | real overflow (`"[1e999]"`) via `jsonp_strtod` | NULL, `json_error_numeric_overflow`, `"real number overflow"` | [x] |
| 200 | `stream_get` | invalid UTF-8 lead byte in input | NULL, `json_error_invalid_utf8`, `"unable to decode byte 0x%x"` | [x] |
| 201 | `stream_get` | valid lead byte but `utf8_check_full` fails (bad continuation) | NULL, `json_error_invalid_utf8` | [x] |
| 202 | `lex_init` | `strbuffer_init` fails | NULL, and the `error` struct is left as `jsonp_error_init` left it | [x] |
| 203 | `parse_object` | `json_object_setn_new_nocheck` fails (OOM) | NULL | [x] |
| 204 | `error_set` | `saved_text.length > 20` → no `" near '...'"` context appended | plain message | [x] |
| 205 | `error_set` | empty input `""` → `json_error_invalid_syntax` upgraded to `json_error_premature_end_of_input` + `" near end of file"` | see code | [x] |

## pack_unpack.c — pack

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 206 | `json_vpack_ex` | `fmt == NULL` | NULL, `json_error_invalid_argument`, `"NULL or empty format string"` | [x] |
| 207 | `json_vpack_ex` | `*fmt == '\0'` (empty) | NULL, `json_error_invalid_argument` | [x] |
| 208 | `json_vpack_ex` | garbage after format (`"[]x"`, `"ss"`) | NULL, `json_error_invalid_format`, `"Garbage after format string"` | [x] |
| 209 | `pack` | unknown format char (`"q"`, `"]"`) | NULL, `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 210 | `pack_object` | end of format inside `{` (`"{"`, `"{s"`) | NULL, `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 211 | `pack_object` | non-`s` key format (`"{i:i}"`) | NULL, `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 212 | `pack_array` | end of format inside `[` (`"["`) | NULL, `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| 213 | `read_string` | NULL `const char*` arg for `s` (non-optional) | NULL, `json_error_null_value`, `"NULL %s"` | [x] |
| 214 | `read_string` | invalid UTF-8 string arg | NULL, `json_error_invalid_utf8`, `"Invalid UTF-8 %s"` | [x] |
| 215 | `read_string` | `s#` / `s%` / `s+` combined with `s?` or `s*` (optional) | NULL, `json_error_invalid_format`, `"Cannot use '%c' on optional strings"` | [x] |
| 216 | `read_string` | NULL arg in a `s+` concatenation chain | NULL, `json_error_null_value` | [x] |
| 217 | `read_string` | concatenated result is invalid UTF-8 (`"s+"` splitting a multi-byte char is fine; split a sequence to make it invalid) | NULL, `json_error_invalid_utf8` | [x] |
| 218 | `read_string` | `strbuffer_init` fails | NULL, `json_error_out_of_memory` | [x] |
| 219 | `pack_object_inter` (`o`/`O`) | NULL `json_t*` arg without `?`/`*` | NULL, `json_error_null_value`, `"NULL object"` | [x] |
| 220 | `pack_object` | NULL value for a key without `*` | NULL, `json_error_null_value`, `"NULL object value"` | [x] |
| 221 | `pack_real` | non-finite `double` arg (`f` with NaN / Inf) | NULL, `json_error_numeric_overflow`, `"Invalid floating point value"` | [x] |
| 222 | `pack_integer` | `json_integer` OOM | NULL, `json_error_out_of_memory` | [x] |
| 223 | `pack_object` | `json_object_setn_new_nocheck` fails | NULL, `json_error_out_of_memory`, `"Unable to add key \"%s\""` | [x] |
| 224 | `pack_array` | `json_array_append_new` fails | NULL, `json_error_out_of_memory`, `"Unable to append to array"` | [x] |

## pack_unpack.c — unpack

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 225 | `json_vunpack_ex` | `root == NULL` | `-1`, `json_error_null_value`, `"NULL root value"` | [x] |
| 226 | `json_vunpack_ex` | `fmt == NULL` | `-1`, `json_error_invalid_argument` | [x] |
| 227 | `json_vunpack_ex` | `*fmt == '\0'` | `-1`, `json_error_invalid_argument` | [x] |
| 228 | `json_vunpack_ex` | garbage after format | `-1`, `json_error_invalid_format`, `"Garbage after format string"` | [x] |
| 229 | `unpack` | unknown format char | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 230 | `unpack_object` | root is not an object | `-1`, `json_error_wrong_type`, `"Expected object, got %s"` | [x] |
| 231 | `unpack_object` | token after `!`/`*` is not `}` | `-1`, `json_error_invalid_format`, `"Expected '}' after '%c', got '%c'"` | [x] |
| 232 | `unpack_object` | end of format inside `{` | `-1`, `json_error_invalid_format` | [x] |
| 233 | `unpack_object` | non-`s` key format | `-1`, `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| 234 | `unpack_object` | NULL key arg | `-1`, `json_error_null_value`, `"NULL object key"` | [x] |
| 235 | `unpack_object` | key absent and not `s?` | `-1`, `json_error_item_not_found`, `"Object item not found: %s"` | [x] |
| 236 | `unpack_object` | `JSON_STRICT` (or `!`) with extra keys left | `-1`, `json_error_end_of_input_expected`, `"%li object item(s) left unpacked: %s"` | [x] |
| 237 | `unpack_object` | `hashtable_init(&key_set)` fails | `-1`, `json_error_out_of_memory` | [x] |
| 238 | `unpack_array` | root is not an array | `-1`, `json_error_wrong_type`, `"Expected array, got %s"` | [x] |
| 239 | `unpack_array` | token after `!`/`*` is not `]` | `-1`, `json_error_invalid_format`, `"Expected ']' after '%c', got '%c'"` | [x] |
| 240 | `unpack_array` | end of format inside `[` | `-1`, `json_error_invalid_format` | [x] |
| 241 | `unpack_array` | format char not in `unpack_value_starters` (`"{[siIbfFOon"`) | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| 242 | `unpack_array` | index past end of array | `-1`, `json_error_index_out_of_range`, `"Array index %lu out of range"` | [x] |
| 243 | `unpack_array` | `JSON_STRICT` (or `!`) with items left | `-1`, `json_error_end_of_input_expected`, `"%li array item(s) left unpacked"` | [x] |
| 244 | `unpack` `s` | root not a string | `-1`, `json_error_wrong_type`, `"Expected string, got %s"` | [x] |
| 245 | `unpack` `s` | NULL `const char**` target | `-1`, `json_error_null_value`, `"NULL string argument"` | [x] |
| 246 | `unpack` `s%` | NULL `size_t*` length target | `-1`, `json_error_null_value`, `"NULL string length argument"` | [x] |
| 247 | `unpack` `i`/`I` | root not an integer | `-1`, `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| 248 | `unpack` `b` | root not a boolean | `-1`, `json_error_wrong_type`, `"Expected true or false, got %s"` | [x] |
| 249 | `unpack` `f` | root not a real | `-1`, `json_error_wrong_type`, `"Expected real, got %s"` | [x] |
| 250 | `unpack` `F` | root neither real nor integer | `-1`, `json_error_wrong_type`, `"Expected real or integer, got %s"` | [x] |
| 251 | `unpack` `n` | root not null | `-1`, `json_error_wrong_type`, `"Expected null, got %s"` | [x] |

## Generic FFI boundary cases (required even though not table-derived)

| # | condition | expected C result | [ ] |
|---|-----------|-------------------|-----|
| 252 | NULL `json_t*` passed to every public getter/setter | per rows above; no crash | [x] |
| 253 | out-of-range `json_type` value (`-1`, `8`, `9`, `100`, `127`, `128`, `200`, `255`, `256`, `65536`, `INT_MAX`, `INT_MIN`) in a forged `json_t` crossing FFI | `json_equal`→0 (1 for the same pointer), `json_copy`/`json_deep_copy`/`do_deep_copy`→NULL, `json_delete`→no-op (does not free), `json_dumps`/`do_dump`→NULL/-1, `json_object_size`/`json_array_size`/`json_string_length`→0, `json_string_value`→NULL, `json_integer_value`/`json_real_value`/`json_number_value`→0. **Not** valid as a `json_unpack_ex` root: the C's wrong-type message uses `type_name(root) = type_names[json_typeof(root)]`, an 8-entry table indexed by the raw tag, so an out-of-range tag is an out-of-bounds read in the C itself | [x] |
| 254 | out-of-range `enum json_error_code` byte written by `jsonp_error_set` (e.g. `200`, `255`) | stored verbatim in `text[159]` | [x] |
| 255 | zero lengths: `json_stringn(p,0)`, `json_loadb(p,0)`, `json_dumpb(j,NULL,0,f)`, `strbuffer_append_bytes(sb,p,0)` | see code (empty string OK; loadb→premature end; dumpb→needed len) | [x] |
| 256 | oversized lengths | **partially [n/a]**: `json_stringn(p, SIZE_MAX)` / `json_object_setn(o,k,SIZE_MAX,v)` make the C read `len`/`key_len` bytes (`utf8_check_string`, `hash_str`) with no prior guard, i.e. UB in the C itself — not a defined input. The guards that *do* fire before any read are tested: `strbuffer_append_bytes(sb, p, SIZE_MAX)` → `-1`, `SIZE_MAX-1` → `-1`, plus `SIZE_MAX`/`SIZE_MAX-1`/`1<<62` array indices and `SIZE_MAX` `jsonp_dtostr` sizes | [x] |
| 257 | one past documented range: `JSON_INDENT(32)` wraps to 0, `JSON_REAL_PRECISION(32)` wraps to 0, unknown flag bits `1<<31` | ignored / wrapped identically | [x] |
| 258 | `json_array_get`/`set`/`insert`/`remove` with `index == entries` and `index == SIZE_MAX` | `NULL`/`-1` (insert at `entries` is VALID) | [x] |
| 259 | `jansson_version_cmp` with out-of-range/negative components | integer diff, identical | [x] |

---

## Rows that are NOT reachable without undefined behaviour in the C

Two rows above are marked **[n/a]** rather than tested.  In both cases the C
dereferences or reads the oversized length *before* the guard that would reject
it, so constructing the trigger crashes the reference implementation and there is
no observable behaviour for the Rust to match:

* row 127 / part of row 256 — `hashtable_set()` calls
  `hash_str(key, key_len)` (which hashes `key_len` bytes) before `init_pair()`
  performs the `key_len >= SIZE_MAX - offsetof(pair_t, key)` check; likewise
  `json_stringn()` calls `utf8_check_string(value, len)` with no prior bound.

Everything reachable in those rows *is* covered: the OOM route into
`hashtable_set` → `-1` (via the failing-allocator sweep in `tests/alloc.rs`), the
`strbuffer_append_bytes` overflow guards, which fire before any read, and the
full range of oversized array indices.

Three further inputs are UB in the C for the same reason and are documented
rather than exercised: a forged out-of-range `json_type` used as a
`json_unpack_ex` root (out-of-bounds `type_names[]` read), a non-NULL but invalid
object iterator (dereferenced unconditionally by `hashtable_iter_*`), and a NULL
`json_dump_callback` on a code path that actually emits output (the C calls
through the NULL pointer).  The Rust was fixed so that the last one behaves like
the C on the paths where the C does *not* call the callback — see “Bugs this table found”
below.

## Bugs this table found

1. `do_dump()` in `src/dump.rs` unwrapped the `Option<fn>` callback at function
   entry, so `json_dump_callback(json, NULL, ...)` aborted the Rust process on the
   paths where the C returns `-1` without ever touching the callback (NULL `json`,
   out-of-range type tag).  Fixed by hoisting the C's `default: return -1` check
   ahead of the unwrap.
2. `set_error()` in `src/pack_unpack.rs` routed the formatted message through
   `"%s"`, truncating it at an embedded NUL.  The C's `set_error()` calls
   `jsonp_error_vset()` with the original format and `va_list`, so
   `"Unexpected format character '%c'"` with a `'\0'` argument leaves
   `...' \0 '` in `error->text`.  Fixed with a length-aware raw-byte copy
   (`jsonp_error_set_bytes`) that reproduces `vsnprintf`'s truncation exactly.
   Reproduced by `json_unpack_ex(root, &err, 0, "{s", "a", &i)`.
