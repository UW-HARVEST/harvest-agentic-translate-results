# ERRORS.md — Error-Surface Table (Phase C gate)

Derived mechanically from the C source by grepping every error-return branch
(`return -1`, `return NULL`, `return 0`, `error_set*`, `set_error`, `assert`,
range/NULL checks, min/max constants) in `c_src/src/*.c`.

## Cross-cutting mechanics that every row depends on

- **First error wins.** `jsonp_error_vset` (`error.c:47`) returns immediately if
  `error->text[0] != '\0'`. A later error never overwrites an earlier one.
- **Error code lives in the text buffer.** `text[159]` holds the
  `enum json_error_code`; message is `vsnprintf`-capped at 159 then
  `text[158]='\0'`, so the effective max message length is 158 chars.
- **`error_set` (`load.c:85`) appends context**: `" near '<saved_text>'"` only when
  `saved_text[0]` is non-empty AND `saved_text.length <= 20`. When `saved_text` is
  empty, `json_error_invalid_syntax` is silently **promoted** to
  `json_error_premature_end_of_input` and `" near end of file"` is appended —
  unless `stream.state == STREAM_STATE_ERROR` (UTF-8 errors get no suffix).
- **`jsonp_error_set_source`** truncates a source >= 80 chars to `"..."` + tail.
- **`jsonp_malloc(0)` returns NULL** (`memory.c:26`) — a real error path.
- **No flag validation anywhere.** Unknown flag bits are ignored;
  `JSON_INDENT(n)`/`JSON_REAL_PRECISION(n)` mask with `0x1F` and wrap silently.

## Status legend

| status | meaning |
|--------|---------|
| `TEST`  | differential test constructs the condition directly |
| `OOM`   | reachable only by injecting a failing allocator via `json_set_alloc_funcs` |
| `UB`    | the C performs **no** check (documented above as UB). Calling it would crash *both* libraries, so it is deliberately NOT exercised; recorded for completeness. |
| `INT`   | internal/defensive branch not reachable through the public API (documented; not exercised) |

Out-of-range `json_type` enum values ARE exercised (`TEST`) wherever the C has a
`default:` branch, since C enums accept any `int` across the FFI boundary.

## value.c

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 1 | `jsonp_loop_check` | pointer already present in `parents` (cycle re-reached) | `-1` | TEST |
| 2 | `jsonp_loop_check` | `hashtable_set` fails while recording pointer key | `-1` | OOM |
| 3 | `json_object` | `jsonp_malloc(sizeof(json_object_t))` fails | `NULL` | OOM |
| 4 | `json_object` | `hashtable_init` bucket malloc fails; object freed | `NULL` | OOM |
| 5 | `json_object_size` | `json == NULL` | `0` | TEST |
| 6 | `json_object_size` | non-`JSON_OBJECT` type | `0` | TEST |
| 7 | `json_object_get` | `key == NULL` | `NULL` | TEST |
| 8 | `json_object_get` | `json == NULL` or not object | `NULL` | TEST |
| 9 | `json_object_get` | key absent | `NULL` | TEST |
| 10 | `json_object_getn` | `key == NULL` | `NULL` | TEST |
| 11 | `json_object_getn` | `json == NULL` or not object | `NULL` | TEST |
| 12 | `json_object_getn` | key/key_len pair absent | `NULL` | TEST |
| 13 | `json_object_set_new_nocheck` | `key == NULL` (value decref'd) | `-1` | TEST |
| 14 | `json_object_setn_new_nocheck` | `value == NULL` (checked before key/type) | `-1` | TEST |
| 15 | `json_object_setn_new_nocheck` | `key == NULL` | `-1` | TEST |
| 16 | `json_object_setn_new_nocheck` | `json == NULL` or not object | `-1` | TEST |
| 17 | `json_object_setn_new_nocheck` | `json == value` (direct self-insert) | `-1` | TEST |
| 18 | `json_object_setn_new_nocheck` | `hashtable_set` fails (rehash/init_pair OOM) | `-1` | OOM |
| 19 | `json_object_set_new` | `key == NULL` | `-1` | TEST |
| 20 | `json_object_setn_new` | `key == NULL` | `-1` | TEST |
| 21 | `json_object_setn_new` | key not valid UTF-8, e.g. `"\xff"` | `-1` | TEST |
| 22 | `json_object_del` | `key == NULL` | `-1` | TEST |
| 23 | `json_object_deln` | `key == NULL` | `-1` | TEST |
| 24 | `json_object_deln` | `json == NULL` or not object | `-1` | TEST |
| 25 | `json_object_deln` | key not found | `-1` | TEST |
| 26 | `json_object_clear` | `json == NULL` or not object | `-1` | TEST |
| 27 | `json_object_update` | `object` NULL or not object | `-1` | TEST |
| 28 | `json_object_update` | `other` NULL or not object | `-1` | TEST |
| 29 | `json_object_update` | inner `setn_nocheck` fails (`other` contains `object`) | `-1` | TEST |
| 30 | `json_object_update_existing` | `object` NULL or not object | `-1` | TEST |
| 31 | `json_object_update_existing` | `other` NULL or not object | `-1` | TEST |
| 32 | `json_object_update_missing` | `object` NULL or not object | `-1` | TEST |
| 33 | `json_object_update_missing` | `other` NULL or not object | `-1` | TEST |
| 34 | `do_object_update_recursive` | `object` NULL or not object | `-1` | TEST |
| 35 | `do_object_update_recursive` | `other` NULL or not object | `-1` | TEST |
| 36 | `do_object_update_recursive` | `jsonp_loop_check` fails (`other` cyclic/ancestor) | `-1` | TEST |
| 37 | `do_object_update_recursive` | recursive call on nested object pair fails | `-1` | TEST |
| 38 | `do_object_update_recursive` | leaf `json_object_setn_nocheck` fails | `-1` | OOM |
| 39 | `json_object_update_recursive` | `hashtable_init(&parents_set)` fails | `-1` | OOM |
| 40 | `json_object_iter` | `json == NULL` or not object | `NULL` | TEST |
| 41 | `json_object_iter` | object is empty | `NULL` | TEST |
| 42 | `json_object_iter_at` | `key == NULL` | `NULL` | TEST |
| 43 | `json_object_iter_at` | `json == NULL` or not object | `NULL` | TEST |
| 44 | `json_object_iter_at` | key absent | `NULL` | TEST |
| 45 | `json_object_iter_next` | `json == NULL` or not object | `NULL` | TEST |
| 46 | `json_object_iter_next` | `iter == NULL` | `NULL` | TEST |
| 47 | `json_object_iter_next` | iter is the last element | `NULL` | TEST |
| 48 | `json_object_iter_key` | `iter == NULL` | `NULL` | TEST |
| 49 | `json_object_iter_key_len` | `iter == NULL` | `0` | TEST |
| 50 | `json_object_iter_value` | `iter == NULL` | `NULL` | TEST |
| 51 | `json_object_iter_set_new` | `json == NULL` or not object | `-1` | TEST |
| 52 | `json_object_iter_set_new` | `iter == NULL` | `-1` | TEST |
| 53 | `json_object_iter_set_new` | `value == NULL` | `-1` | TEST |
| 54 | `json_object_key_to_iter` | `key == NULL` | `NULL` | TEST |
| 55 | `json_equal` (object) | sizes differ | `0` | TEST |
| 56 | `json_equal` (object) | a key of o1 missing in o2, or values differ | `0` | TEST |
| 57 | `json_copy` (object) | `json_object()` allocation fails | `NULL` | OOM |
| 58 | `do_deep_copy` (object) | `jsonp_loop_check` fails (self-referential) | `NULL` | TEST |
| 59 | `do_deep_copy` (object) | `json_object()` fails | `NULL` | OOM |
| 60 | `do_deep_copy` (object) | inner `do_deep_copy` returned NULL | `NULL` | OOM |
| 61 | `json_array` | `jsonp_malloc(sizeof(json_array_t))` fails | `NULL` | OOM |
| 62 | `json_array` | table `jsonp_malloc(8 * sizeof(json_t*))` fails | `NULL` | OOM |
| 63 | `json_array_size` | `json == NULL` or not array | `0` | TEST |
| 64 | `json_array_get` | `json == NULL` or not array | `NULL` | TEST |
| 65 | `json_array_get` | `index >= entries` (incl. empty array) | `NULL` | TEST |
| 66 | `json_array_set_new` | `value == NULL` | `-1` | TEST |
| 67 | `json_array_set_new` | `json == NULL` or not array | `-1` | TEST |
| 68 | `json_array_set_new` | `json == value` (self-insert) | `-1` | TEST |
| 69 | `json_array_set_new` | `index >= entries` (set is NOT append) | `-1` | TEST |
| 70 | `json_array_grow` | `jsonp_realloc` fails | `NULL` | OOM |
| 71 | `json_array_append_new` | `value == NULL` | `-1` | TEST |
| 72 | `json_array_append_new` | `json == NULL` or not array | `-1` | TEST |
| 73 | `json_array_append_new` | `json == value` (self-append) | `-1` | TEST |
| 74 | `json_array_append_new` | `json_array_grow` fails | `-1` | OOM |
| 75 | `json_array_insert_new` | `value == NULL` | `-1` | TEST |
| 76 | `json_array_insert_new` | `json == NULL` or not array | `-1` | TEST |
| 77 | `json_array_insert_new` | `json == value` | `-1` | TEST |
| 78 | `json_array_insert_new` | `index > entries` (`== entries` IS allowed) | `-1` | TEST |
| 79 | `json_array_insert_new` | `json_array_grow` fails | `-1` | OOM |
| 80 | `json_array_remove` | `json == NULL` or not array | `-1` | TEST |
| 81 | `json_array_remove` | `index >= entries` | `-1` | TEST |
| 82 | `json_array_clear` | `json == NULL` or not array | `-1` | TEST |
| 83 | `json_array_extend` | `json` NULL or not array | `-1` | TEST |
| 84 | `json_array_extend` | `other_json` NULL or not array | `-1` | TEST |
| 85 | `json_array_extend` | `json_array_grow` fails | `-1` | OOM |
| 86 | `json_equal` (array) | sizes differ | `0` | TEST |
| 87 | `json_equal` (array) | any element pair not equal | `0` | TEST |
| 88 | `json_copy` (array) | `json_array()` fails | `NULL` | OOM |
| 89 | `do_deep_copy` (array) | `jsonp_loop_check` fails (self-referential) | `NULL` | TEST |
| 90 | `do_deep_copy` (array) | `json_array()` fails | `NULL` | OOM |
| 91 | `do_deep_copy` (array) | inner `do_deep_copy` returned NULL | `NULL` | OOM |
| 92 | `string_create` | `value == NULL` | `NULL` | TEST |
| 93 | `string_create` | `jsonp_strndup` fails (OOM, or `len == (size_t)-1`) | `NULL` | TEST |
| 94 | `string_create` | `jsonp_malloc(sizeof(json_string_t))` fails | `NULL` | OOM |
| 95 | `json_string_nocheck` | `value == NULL` | `NULL` | TEST |
| 96 | `json_stringn_nocheck` | `value == NULL` | `NULL` | TEST |
| 97 | `jsonp_stringn_nocheck_own` | `value == NULL` (buffer NOT freed) | `NULL` | TEST |
| 98 | `json_string` | `value == NULL` | `NULL` | TEST |
| 99 | `json_stringn` | `value == NULL` | `NULL` | TEST |
| 100 | `json_stringn` | invalid UTF-8 over `[0,len)`: `"\x80"`, `"\xC0\x80"`, `"\xED\xA0\x80"`, truncated `"\xE2\x82"` | `NULL` | TEST |
| 101 | `json_string_value` | `json == NULL` or not string | `NULL` | TEST |
| 102 | `json_string_length` | `json == NULL` or not string | `0` | TEST |
| 103 | `json_string_set_nocheck` | `value == NULL` | `-1` | TEST |
| 104 | `json_string_setn_nocheck` | `json == NULL` or not string | `-1` | TEST |
| 105 | `json_string_setn_nocheck` | `value == NULL` | `-1` | TEST |
| 106 | `json_string_setn_nocheck` | `jsonp_strndup` fails (old value intact) | `-1` | OOM |
| 107 | `json_string_set` | `value == NULL` | `-1` | TEST |
| 108 | `json_string_setn` | `value == NULL` | `-1` | TEST |
| 109 | `json_string_setn` | value not valid UTF-8 over `[0,len)` | `-1` | TEST |
| 110 | `json_vsprintf`/`json_sprintf` | `vsnprintf(NULL,0,fmt,ap) < 0` (encoding error) | `NULL` | INT |
| 111 | `json_vsprintf`/`json_sprintf` | `jsonp_malloc(length+1)` fails | `NULL` | OOM |
| 112 | `json_vsprintf`/`json_sprintf` | result not valid UTF-8, e.g. `json_sprintf("%s","\xff")` | `NULL` | TEST |
| 113 | `json_integer` | `jsonp_malloc` fails | `NULL` | OOM |
| 114 | `json_integer_value` | `json == NULL` or not integer (incl. `JSON_REAL`) | `0` | TEST |
| 115 | `json_integer_set` | `json == NULL` or not integer | `-1` | TEST |
| 116 | `json_real` | value is NaN | `NULL` | TEST |
| 117 | `json_real` | value is +Inf or -Inf | `NULL` | TEST |
| 118 | `json_real` | `jsonp_malloc` fails | `NULL` | OOM |
| 119 | `json_real_value` | `json == NULL` or not real (incl. `JSON_INTEGER`) | `0.0` | TEST |
| 120 | `json_real_set` | `json == NULL` or not real | `-1` | TEST |
| 121 | `json_real_set` | value is NaN | `-1` | TEST |
| 122 | `json_real_set` | value is +Inf or -Inf | `-1` | TEST |
| 123 | `json_number_value` | neither integer nor real (incl. NULL) | `0.0` | TEST |
| 124 | `json_delete` | `json == NULL` | early return | TEST |
| 125 | `json_delete` | type is TRUE/FALSE/NULL **or out-of-range enum** -> `default: return;` | void, no free | TEST |
| 126 | `json_equal` | `json1 == NULL` | `0` | TEST |
| 127 | `json_equal` | `json2 == NULL` | `0` | TEST |
| 128 | `json_equal` | types differ, e.g. `json_integer(1)` vs `json_real(1.0)` | `0` | TEST |
| 129 | `json_equal` | **default branch on out-of-range `json_type`** | `0` | TEST |
| 130 | `json_copy` | `json == NULL` | `NULL` | TEST |
| 131 | `json_copy` | **default branch on out-of-range `json_type`** | `NULL` | TEST |
| 132 | `json_deep_copy` | `hashtable_init(&parents_set)` fails | `NULL` | OOM |
| 133 | `do_deep_copy` | `json == NULL` | `NULL` | TEST |
| 134 | `do_deep_copy` | **default branch on out-of-range `json_type`** | `NULL` | TEST |

## load.c

All public decode entry points return `NULL` on failure. Error codes/messages as noted.
Note: `U0000` below denotes the six-character JSON escape backslash-u-0-0-0-0.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 135 | `stream_get` | lead byte `0x80-0xBF`, `0xC0`, `0xC1`, or `0xF5-0xFF` (`utf8_check_first==0`) | `NULL`, `json_error_invalid_utf8`, `"unable to decode byte 0x%x"` (no near-suffix) | TEST |
| 136 | `stream_get` | valid lead but `utf8_check_full` fails: bad continuation / overlong / surrogate `\xED\xA0\x80` / EOF mid-sequence | `NULL`, `json_error_invalid_utf8` (reports LEAD byte) | TEST |
| 137 | `lex_scan_string` | EOF before closing quote, e.g. `["abc` | `NULL`, `json_error_premature_end_of_input` | TEST |
| 138 | `lex_scan_string` | raw newline `0x0A` inside a string literal | `NULL`, `json_error_invalid_syntax`, `"unexpected newline"` | TEST |
| 139 | `lex_scan_string` | raw control char `0x00-0x1F` other than `0x0A` inside a string (e.g. TAB) | `NULL`, `json_error_invalid_syntax`, `"control character 0x%x"` | TEST |
| 140 | `lex_scan_string` | `\u` with fewer than 4 hex digits / a non-hex digit, e.g. `["\u12z4"]`, `["\u00"]` | `NULL`, `json_error_invalid_syntax`, `"invalid escape"` | TEST |
| 141 | `lex_scan_string` | backslash + char not in `"\/bfnrtu`, e.g. `["\x"]`, `["\a"]` | `NULL`, `json_error_invalid_syntax`, `"invalid escape"` | TEST |
| 142 | `lex_scan_string` | 2nd pass `decode_unicode_escape` returns -1 (defensive; hex already validated) | `NULL`, `"invalid Unicode escape '%.6s'"` | INT |
| 143 | `lex_scan_string` | high surrogate then a `\u` whose 4 digits fail to decode | `NULL`, `"invalid Unicode escape '%.6s'"` | TEST |
| 144 | `lex_scan_string` | high surrogate `D800-DBFF` then a `\uXXXX` NOT in `DC00-DFFF`, e.g. `["\uD888㈐"]` | `NULL`, `"invalid Unicode ..."` | TEST |
| 145 | `lex_scan_string` | lone high surrogate not followed by `\u`, e.g. `["\uD888"]`, `["\uDB00x"]` | `NULL`, `"invalid Unicode ..."` | TEST |
| 146 | `lex_scan_string` | lone LOW surrogate `DC00-DFFF`, e.g. `["\uDC00"]` | `NULL`, `"invalid Unicode ..."` | TEST |
| 147 | `lex_scan_string` | `jsonp_malloc(saved_text.length+1)` fails, `goto out` with no error_set | `NULL`, `"invalid token"` | OOM |
| 148 | `lex_scan_number` | leading zero followed by a digit, e.g. `[01]`, `[-012]` | `NULL`, `json_error_invalid_syntax`, `"invalid token"` | TEST |
| 149 | `lex_scan_number` | `-` not followed by a digit, e.g. `[-]`, `[-x]`, `[-.5]` | `NULL`, `"invalid token"` | TEST |
| 150 | `lex_scan_number` | integer literal below `json_int_t` min, e.g. `[-9223372036854775809]` | `NULL`, `json_error_numeric_overflow`, `"too big negative integer"` | TEST |
| 151 | `lex_scan_number` | integer literal above `json_int_t` max, e.g. `[9223372036854775808]` | `NULL`, `json_error_numeric_overflow`, `"too big integer"` | TEST |
| 152 | `lex_scan_number` | `.` not followed by a digit, e.g. `[1.]`, `[1.e5]` | `NULL`, `"invalid token"` | TEST |
| 153 | `lex_scan_number` | `e`/`E` (optionally signed) not followed by a digit, e.g. `[1e]`, `[1e+]`, `[1E-x]` | `NULL`, `"invalid token"` | TEST |
| 154 | `lex_scan_number` | real literal overflows double (`jsonp_strtod` -> -1), e.g. `[1e999]`, `[-1e309]` | `NULL`, `json_error_numeric_overflow`, `"real number overflow"` | TEST |
| 155 | `lex_scan` | alphabetic identifier not exactly `true`/`false`/`null`, e.g. `[nul]`, `[TRUE]`, `[foo]` | `NULL`, `json_error_invalid_syntax`, `"invalid token"` | TEST |
| 156 | `lex_scan` | any other leading char (not ws, braces, brackets, colon, comma, quote, digit, `-`, alpha), e.g. single-quote, `#`, `@` | `NULL`, `"invalid token"` | TEST |
| 157 | `parse_object` | `json_object()` fails (OOM), no error set | `NULL` | OOM |
| 158 | `parse_object` | key token is not `TOKEN_STRING`, e.g. `{1:2}`, `{,}`, `{"a":1,}`, `{:}` | `NULL`, `"string or '}' expected"` | TEST |
| 159 | `parse_object` | `lex_steal_string` NULL though token is STRING (defensive; also leaks object) | `NULL`, no error set | INT |
| 160 | `parse_object` | object key contains an embedded NUL from a `U0000` escape (fails even with `JSON_ALLOW_NUL`) | `NULL`, `json_error_null_byte_in_key` | TEST |
| 161 | `parse_object` | `JSON_REJECT_DUPLICATES` set and a key repeats, e.g. `{"a":1,"a":2}` | `NULL`, `json_error_duplicate_key` | TEST |
| 162 | `parse_object` | token after key is not `:`, e.g. `{"a" 1}`, `{"a",1}` | `NULL`, `"':' expected"` | TEST |
| 163 | `parse_object` | member value `parse_value` fails (inner code preserved) | `NULL` | TEST |
| 164 | `parse_object` | `json_object_setn_new_nocheck` fails, no error set | `NULL` | OOM |
| 165 | `parse_object` | after a member, next token neither `,` nor `}`, e.g. `{"a":1 "b":2}` | `NULL`, `"'}' expected"` | TEST |
| 166 | `parse_object` | EOF before `}`, e.g. `{"a":1` | `NULL`, promoted `json_error_premature_end_of_input` | TEST |
| 167 | `parse_array` | `json_array()` fails (OOM), no error set | `NULL` | OOM |
| 168 | `parse_array` | element `parse_value` fails, incl. trailing comma `[1,]` | `NULL`, `"unexpected token"` | TEST |
| 169 | `parse_array` | `json_array_append_new` fails, no error set | `NULL` | OOM |
| 170 | `parse_array` | after an element, next token neither `,` nor `]`, e.g. `[1 2]` | `NULL`, `"']' expected"` | TEST |
| 171 | `parse_array` | EOF before `]`, e.g. `[`, `[1`, `[1,` | `NULL`, promoted `json_error_premature_end_of_input` | TEST |
| 172 | `parse_value` | `lex->depth > JSON_PARSER_MAX_DEPTH` (2048): 2049 nested `[` | `NULL`, `json_error_stack_overflow`, `"maximum parsing depth reached"` | TEST |
| 173 | `parse_value` | decoded string contains a NUL (via `U0000`) and `JSON_ALLOW_NUL` is NOT set | `NULL`, `json_error_null_character` | TEST |
| 174 | `parse_value` | `jsonp_stringn_nocheck_own` / `json_integer` / `json_real` returned NULL (OOM) | `NULL`, no error set | OOM |
| 175 | `parse_value` | `token == TOKEN_INVALID` with no error set by the lexer | `NULL`, `json_error_invalid_syntax`, `"invalid token"` | TEST |
| 176 | `parse_value` | default: token is `}` `]` `:` `,` or EOF, e.g. `[}]`, `[,1]`, `[:]` | `NULL`, `"unexpected token"` (at EOF -> promoted) | TEST |
| 177 | `parse_json` | no `JSON_DECODE_ANY` and first token not `[`/`{`, e.g. `"str"`, `1`, `true` | `NULL`, `"'[' or '{' expected"` | TEST |
| 178 | `parse_json` | no `JSON_DECODE_ANY` and input is the empty string | `NULL`, promoted premature-end | TEST |
| 179 | `parse_json` | no `JSON_DISABLE_EOF_CHECK` and trailing non-whitespace, e.g. `[1] [2]`, `{} x` | `NULL`, `json_error_end_of_input_expected` | TEST |
| 180 | `json_loads` | `string == NULL` | `NULL`, `json_error_invalid_argument`, `"wrong arguments"`, line=-1 col=-1 pos=0, source `<string>` | TEST |
| 181 | `json_loads` | `lex_init` fails (strbuffer OOM), error text left empty | `NULL` | OOM |
| 182 | `json_loads` | an embedded NUL byte in the C string is treated as EOF | `NULL`, premature end | TEST |
| 183 | `json_loadb` | `buffer == NULL` | `NULL`, `json_error_invalid_argument`, source `<buffer>` | TEST |
| 184 | `json_loadb` | `lex_init` fails | `NULL`, no error text | OOM |
| 185 | `json_loadf` | `input == NULL` | `NULL`, `json_error_invalid_argument`, source `<stream>` | TEST |
| 186 | `json_loadf` | `lex_init` fails | `NULL`, no error text | OOM |
| 187 | `json_loadfd` | `input < 0` (negative fd) | `NULL`, `json_error_invalid_argument`, source `<stream>` | TEST |
| 188 | `json_loadfd` | `lex_init` fails | `NULL`, no error text | OOM |
| 189 | `json_loadfd` | `read()` returns != 1 (closed/invalid fd) -> EOF | `NULL`, premature end | TEST |
| 190 | `json_load_file` | `path == NULL` | `NULL`, `json_error_invalid_argument`, empty `source` | TEST |
| 191 | `json_load_file` | `fopen(path,"rb")` NULL (missing file / permission denied / is-a-dir) | `NULL`, `json_error_cannot_open_file`, `"unable to open %s: %s"`, source = path (truncated to `...`+tail if >= 80) | TEST |
| 192 | `json_load_callback` | `callback == NULL` | `NULL`, `json_error_invalid_argument`, source `<callback>` | TEST |
| 193 | `json_load_callback` | `lex_init` fails | `NULL`, no error text | OOM |
| 194 | `json_load_callback` | callback returns `0` or `(size_t)-1` mid-document -> EOF | `NULL`, premature end | TEST |

**Live asserts in load.c** (the build sets no `-DNDEBUG`, so these abort rather than being
elided): `assert(count >= 2)`:175; `assert(stream->buffer_pos > 0)` and
`assert(buffer[pos]==c)`:221,223; `assert(c == d)` in `lex_unget_unsave`:255;
`assert(str[0]=='u')`:278; `assert(0)` after a failed `utf8_encode`:417; `assert(0)` in the
escape switch default:442; `assert(end == saved_text + length)`:514. All are internal
invariants not reachable through the public API (status `INT`).
## dump.c

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 195 | `dump_to_strbuffer` | `strbuffer_append_bytes` fails (OOM / size overflow) | `-1` | OOM |
| 196 | `dump_to_buffer` | `buf->used + size > buf->size`: NO error, silently skips the memcpy and returns `0`; `json_dumpb` still returns the REQUIRED length | `0` (truncated output) | TEST |
| 197 | `dump_to_file` | `fwrite(buffer,size,1,dest) != 1` (read-only FILE*, disk full) | `-1` | TEST |
| 198 | `dump_to_fd` | `write(fd,buffer,size) != size` (bad fd, EPIPE) | `-1` | TEST |
| 199 | `dump_indent` | `dump("\n",1)` callback returns non-zero | `-1` | TEST |
| 200 | `dump_indent` | `dump(whitespace,cur_n)` callback returns non-zero | `-1` | TEST |
| 201 | `dump_string` | opening `dump("\"",1)` fails | `-1` | TEST |
| 202 | `dump_string` | `utf8_iterate` returns NULL: value holds invalid UTF-8 (only reachable via `json_string_nocheck` / `json_stringn_nocheck` / `jsonp_stringn_nocheck_own`) or a truncated multi-byte sequence | `-1` | TEST |
| 203 | `dump_string` | `dump()` of the plain unescaped run fails | `-1` | TEST |
| 204 | `dump_string` | `dump(text,length)` of an escape sequence fails | `-1` | TEST |
| 205 | `dump_string` | closing `dump("\"",1)` fails | `-1` | TEST |
| 206 | `do_dump` | `json == NULL` | `-1` | TEST |
| 207 | `do_dump` (INTEGER) | `snprintf` returns `<0` or `>= MAX_INTEGER_STR_LENGTH` (25) | `-1` | INT |
| 208 | `do_dump` (REAL) | `jsonp_dtostr` returns `<0` (25-byte buffer too short), e.g. `JSON_REAL_PRECISION(31)` on `1.0/3.0` | `-1` | TEST |
| 209 | `do_dump` (ARRAY) | `jsonp_loop_check` fails: circular reference | `-1` | TEST |
| 210 | `do_dump` (ARRAY) | `dump("[",1)` fails | `-1` | TEST |
| 211 | `do_dump` (ARRAY) | `dump("]",1)` fails on the empty-array early return | `-1` | TEST |
| 212 | `do_dump` (ARRAY) | `dump_indent(depth+1)` fails | `-1` | TEST |
| 213 | `do_dump` (ARRAY) | recursive `do_dump` of element i fails | `-1` | TEST |
| 214 | `do_dump` (ARRAY) | `dump(",",1)` or `dump_indent` between elements fails | `-1` | TEST |
| 215 | `do_dump` (ARRAY) | final `dump_indent(depth)` or `dump("]",1)` fails | `-1` | TEST |
| 216 | `do_dump` (OBJECT) | `jsonp_loop_check` fails: circular reference | `-1` | TEST |
| 217 | `do_dump` (OBJECT) | `dump("{",1)` fails | `-1` | TEST |
| 218 | `do_dump` (OBJECT) | `dump("}",1)` fails on the empty-object early return | `-1` | TEST |
| 219 | `do_dump` (OBJECT) | `dump_indent(depth+1)` fails | `-1` | TEST |
| 220 | `do_dump` (OBJECT, SORT_KEYS) | `jsonp_malloc(size * sizeof(struct key_len))` fails | `-1` | OOM |
| 221 | `do_dump` (OBJECT, SORT_KEYS) | `dump(separator)` or recursive `do_dump(value)` fails | `-1` | TEST |
| 222 | `do_dump` (OBJECT, SORT_KEYS) | `dump(",")` or `dump_indent` between members fails | `-1` | TEST |
| 223 | `do_dump` (OBJECT, SORT_KEYS) | trailing `dump_indent(depth)` fails | `-1` | TEST |
| 224 | `do_dump` (OBJECT, unsorted) | `dump(separator)` or recursive `do_dump(value)` fails | `-1` | TEST |
| 225 | `do_dump` (OBJECT, unsorted) | `dump(",")` or `dump_indent` between members fails | `-1` | TEST |
| 226 | `do_dump` (OBJECT, unsorted) | trailing `dump_indent(depth)` fails | `-1` | TEST |
| 227 | `do_dump` (OBJECT) | QUIRK: `dump_string()` for the KEY has its return value IGNORED in both branches, so an invalid-UTF-8 KEY does NOT fail the dump (partial output) | no error | TEST |
| 228 | `do_dump` | **default: out-of-range `json_type` ("not reached")** | `-1` | TEST |
| 229 | `json_dumps` | `strbuffer_init` fails (OOM) | `NULL` | OOM |
| 230 | `json_dumps` | `json_dump_callback` returns non-zero | `NULL` | TEST |
| 231 | `json_dumps` | (non-error) `jsonp_realloc` shrink fails -> keeps the oversized pointer | non-NULL | OOM |
| 232 | `json_dumpb` | `json_dump_callback` returns non-zero | `0` | TEST |
| 233 | `json_dumpf` | `json_dump_callback` returns non-zero | `-1` | TEST |
| 234 | `json_dumpfd` | `json_dump_callback` returns non-zero | `-1` | TEST |
| 235 | `json_dump_file` | `fopen(path,"w")` fails (bad dir / permission) | `-1` | TEST |
| 236 | `json_dump_file` | `fclose(output) != 0` (flush error) — masks the dump result | `-1` | INT |
| 237 | `json_dump_file` | `json_dumpf` result propagated | `-1` | TEST |
| 238 | `json_dump_callback` | no `JSON_ENCODE_ANY` and root is neither array nor object — incl. `json == NULL`, string, integer, `json_true()` | `-1` | TEST |
| 239 | `json_dump_callback` | `hashtable_init(&parents_set)` fails (OOM) | `-1` | OOM |
| 240 | `json_dump_callback` | `do_dump` returns non-zero | `-1` | TEST |

Live asserts in dump.c: `assert(i == size)`:354 and `assert(value)`:364 (SORT_KEYS path) — internal invariants (`INT`).

## pack_unpack.c

`type_name(x)` = `type_names[json_typeof(x)]` (`pack_unpack.c:40`) — the ONLY switch-on-type
site with **no** default and **no** bounds check; an out-of-range `json_type` is an
out-of-bounds read fed straight into the error message (`UB`).

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 241 | `read_string` | next token is not `#`/`%`/`+`, arg `str == NULL`, not optional, e.g. `json_pack("s", NULL)` | `NULL`, `json_error_null_value`, `"NULL string"` / `"NULL object key"`, source `<args>` | TEST |
| 242 | `read_string` | arg non-NULL but invalid UTF-8, e.g. `json_pack("s","\xff\xff")` | `NULL`, `json_error_invalid_utf8`, `"Invalid UTF-8 string"`/`"... object key"`, source `<args>` | TEST |
| 243 | `read_string` | optional string (`s?`/`s*`) combined with `#`, `%` or `+`, e.g. `json_pack("s?#","x",1)` | `NULL`, `json_error_invalid_format`, `"Cannot use '%c' on optional strings"`, source `<format>` | TEST |
| 244 | `read_string` | `strbuffer_init` fails on the `#`/`%`/`+` path | `NULL`, `json_error_out_of_memory`, source `<internal>` | OOM |
| 245 | `read_string` | in the `+` concat loop any `va_arg` str is NULL, e.g. `json_pack("s++","a",NULL,"c")` | `NULL`, `json_error_null_value`, `"NULL string"` | TEST |
| 246 | `read_string` | `strbuffer_append_bytes` fails while concatenating | `NULL`, `json_error_out_of_memory`, source `<internal>` | OOM |
| 247 | `read_string` | the CONCATENATED result is invalid UTF-8 (pieces split a multi-byte sequence) | `NULL`, `json_error_invalid_utf8`, source `<args>` | TEST |
| 248 | `pack_object` | format ends inside `{` (token `\0`), e.g. `json_pack("{")`, `json_pack("{s:i")` | `NULL`, `json_error_invalid_format`, `"Unexpected end of format string"` | TEST |
| 249 | `pack_object` | key format char is not `s`, e.g. `json_pack("{i:i}",1,2)` | `NULL`, `json_error_invalid_format`, `"Expected format 's', got '%c'"` | TEST |
| 250 | `pack_object` | value `pack()` returned NULL and trailing token is not `*`, e.g. `json_pack("{s:o}","k",NULL)` | `NULL`, `json_error_null_value`, `"NULL object value"` (first-error-wins may keep an inner message) | TEST |
| 251 | `pack_object` | `json_object_setn_new_nocheck` fails | `NULL`, `json_error_out_of_memory`, `"Unable to add key \"%s\""`, source `<internal>` | OOM |
| 252 | `pack_object` | `json_object()` returned NULL (OOM); unchecked, falls through | `NULL` | OOM |
| 253 | `pack_array` | format ends inside `[` (token `\0`), e.g. `json_pack("[")`, `json_pack("[i")` | `NULL`, `json_error_invalid_format`, `"Unexpected end of format string"` | TEST |
| 254 | `pack_array` | `pack()` NULL and element trailing token is not `*`; sets has_error with no new set_error, e.g. `json_pack("[o]",NULL)` | `NULL`, `json_error_null_value`, `"NULL object"` | TEST |
| 255 | `pack_array` | `json_array_append_new` fails | `NULL`, `json_error_out_of_memory`, `"Unable to append to array"` | OOM |
| 256 | `pack_array` | `json_array()` returned NULL (OOM); unchecked | `NULL` | OOM |
| 257 | `pack_string` | `read_string` NULL and modifier is not `?` | `NULL`, inner code | TEST |
| 258 | `pack_object_inter` (`o`/`O`) | `va_arg` json_t* NULL and trailing token neither `?` nor `*`, e.g. `json_pack("o",NULL)` | `NULL`, `json_error_null_value`, `"NULL object"`, source `<args>` | TEST |
| 259 | `pack_integer` (`i`/`I`) | `json_integer()` fails | `NULL`, `json_error_out_of_memory`, source `<internal>` | OOM |
| 260 | `pack_real` (`f`) | `json_real(0.0)` fails | `NULL`, `json_error_out_of_memory`, source `<internal>` | OOM |
| 261 | `pack_real` (`f`) | `json_real_set` fails: arg is NaN, +Inf or -Inf, e.g. `json_pack_ex(&e,0,"f",NAN)` | `NULL`, `json_error_numeric_overflow`, `"Invalid floating point value"`, source `<args>` | TEST |
| 262 | `pack` | default: format char not one of `{[siIbfFOon`, e.g. `json_pack("x")`, `json_pack("]")`, `json_pack("r")`, `json_pack("F")` | `NULL`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | TEST |
| 263 | `json_vpack_ex`/`json_pack`/`json_pack_ex` | `fmt == NULL` | `NULL`, `json_error_invalid_argument`, `"NULL or empty format string"`, line=-1 col=-1 pos=0, source `<format>` | TEST |
| 264 | `json_vpack_ex`/`json_pack`/`json_pack_ex` | `fmt == ""` | `NULL`, `json_error_invalid_argument`, source `<format>` | TEST |
| 265 | `json_vpack_ex` | leftover format chars after a complete value, e.g. `json_pack("ii",1,2)`, `json_pack("[]}")` | `NULL`, `json_error_invalid_format`, `"Garbage after format string"` | TEST |
| 266 | `unpack_object` | `hashtable_init(&key_set)` fails | `-1`, `json_error_out_of_memory`, source `<internal>` | OOM |
| 267 | `unpack_object` | root non-NULL and not `JSON_OBJECT`, e.g. `json_unpack(json_array(),"{s:i}","k",&i)` | `-1`, `json_error_wrong_type`, `"Expected object, got %s"`, source `<validation>` | TEST |
| 268 | `unpack_object` | more tokens AFTER `!` or `*`, e.g. `"{s:i!s:i}"` | `-1`, `json_error_invalid_format`, `"Expected '}' after '%c', got '%c'"` | TEST |
| 269 | `unpack_object` | format ends inside `{`, e.g. `"{"`, `"{s:i"` | `-1`, `json_error_invalid_format`, `"Unexpected end of format string"` | TEST |
| 270 | `unpack_object` | key format char is not `s`/`!`/`*`/`}`, e.g. `"{i:i}"` | `-1`, `json_error_invalid_format`, `"Expected format 's', got '%c'"` | TEST |
| 271 | `unpack_object` | `va_arg` key pointer is NULL, e.g. `json_unpack(o,"{s:i}",NULL,&i)` | `-1`, `json_error_null_value`, `"NULL object key"`, source `<args>` | TEST |
| 272 | `unpack_object` | key absent from root and NOT optional (`s?`), e.g. `json_unpack(json_object(),"{s:i}","missing",&i)` | `-1`, `json_error_item_not_found`, `"Object item not found: %s"`, source `<validation>` | TEST |
| 273 | `unpack_object` | recursive `unpack()` of the member fails | `-1`, inner code | TEST |
| 274 | `unpack_object` | strict (`!` or `JSON_STRICT`) and root has unreferenced keys | `-1`, `json_error_end_of_input_expected`, `"%li object item(s) left unpacked: %s"`, source `<validation>` | TEST |
| 275 | `unpack_array` | root non-NULL and not `JSON_ARRAY`, e.g. `json_unpack(json_object(),"[i]",&i)` | `-1`, `json_error_wrong_type`, `"Expected array, got %s"` | TEST |
| 276 | `unpack_array` | more tokens AFTER `!` or `*`, e.g. `"[i!i]"` | `-1`, `json_error_invalid_format`, `"Expected ']' after '%c', got '%c'"` | TEST |
| 277 | `unpack_array` | format ends inside `[`, e.g. `"["`, `"[i"` | `-1`, `json_error_invalid_format`, `"Unexpected end of format string"` | TEST |
| 278 | `unpack_array` | element format char not in `"{[siIbfFOon"`, e.g. `"[x]"`, `"[#]"`, `"[%]"`, `"[?i]"` | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | TEST |
| 279 | `unpack_array` | `json_array_get(root,i)` NULL: format wants more elements than present, e.g. `"[i,i]"` on `[1]` | `-1`, `json_error_index_out_of_range`, `"Array index %lu out of range"` | TEST |
| 280 | `unpack_array` | recursive `unpack()` of the element fails | `-1`, inner code | TEST |
| 281 | `unpack_array` | strict and `i != json_array_size(root)` | `-1`, `json_error_end_of_input_expected`, `"%li array item(s) left unpacked"` | TEST |
| 282 | `unpack` (`s`) | root non-NULL and not `JSON_STRING` | `-1`, `json_error_wrong_type`, `"Expected string, got %s"` | TEST |
| 283 | `unpack` (`s`) | `va_arg const char**` target NULL (and not `JSON_VALIDATE_ONLY`) | `-1`, `json_error_null_value`, `"NULL string argument"` | TEST |
| 284 | `unpack` (`s%`) | `va_arg size_t*` length target NULL | `-1`, `json_error_null_value`, `"NULL string length argument"` | TEST |
| 285 | `unpack` (`i`) | root non-NULL and not `JSON_INTEGER` (a REAL fails here) | `-1`, `json_error_wrong_type`, `"Expected integer, got %s"` | TEST |
| 286 | `unpack` (`I`) | root non-NULL and not `JSON_INTEGER` | `-1`, `json_error_wrong_type` | TEST |
| 287 | `unpack` (`b`) | root non-NULL and neither `JSON_TRUE` nor `JSON_FALSE` | `-1`, `json_error_wrong_type`, `"Expected true or false, got %s"` | TEST |
| 288 | `unpack` (`f`) | root non-NULL and not `JSON_REAL` (an INTEGER fails here) | `-1`, `json_error_wrong_type`, `"Expected real, got %s"` | TEST |
| 289 | `unpack` (`F`) | root non-NULL and neither INTEGER nor REAL | `-1`, `json_error_wrong_type`, `"Expected real or integer, got %s"` | TEST |
| 290 | `unpack` (`n`) | root non-NULL and not `JSON_NULL` | `-1`, `json_error_wrong_type`, `"Expected null, got %s"` | TEST |
| 291 | `unpack` (`o`/`O`) | NO type check and NO NULL check on the `json_t**` target | `0` (writing through NULL is UB) | UB |
| 292 | `unpack` (`i`/`I`/`b`/`f`/`F`) | targets are NOT NULL-checked (unlike `s`) | `0` (UB) | UB |
| 293 | `unpack` | default: format char not in `{[siIbfFOon`, e.g. `"x"`, `"!"`, `"r"`, `"s#"` | `-1`, `json_error_invalid_format`, `"Unexpected format character '%c'"` | TEST |
| 294 | `json_vunpack_ex`/`json_unpack`/`json_unpack_ex` | `root == NULL` | `-1`, `json_error_null_value`, `"NULL root value"`, line=-1 col=-1 pos=0, source `<root>` | TEST |
| 295 | `json_vunpack_ex`/`json_unpack`/`json_unpack_ex` | `fmt == NULL` | `-1`, `json_error_invalid_argument`, source `<format>` | TEST |
| 296 | `json_vunpack_ex`/`json_unpack`/`json_unpack_ex` | `fmt == ""` | `-1`, `json_error_invalid_argument`, source `<format>` | TEST |
| 297 | `json_vunpack_ex` | leftover format chars, e.g. `"{}}"`, `"[i]i"` | `-1`, `json_error_invalid_format`, `"Garbage after format string"` | TEST |
| 298 | `type_name` | out-of-range `json_type` -> OOB read of `type_names[8]` | undefined | UB |

## strconv.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 299 | `jsonp_strtod` | `strtod` yields +/-`HUGE_VAL` AND `errno == ERANGE`, e.g. `[1e999]`, `[-1e400]` | `-1` (caller sets `json_error_numeric_overflow`) | TEST |
| 300 | `jsonp_strtod` | NOT an error: underflow `1e-999` sets ERANGE but value != HUGE_VAL | `0`, `*out == 0.0` | TEST |
| 301 | `jsonp_dtostr` | `dtoa_r` returns NULL (25-byte `digits[]` too short; "should not happen") | `-1` | INT |
| 302 | `jsonp_dtostr` | required `3 + (vdigits_end - vdigits_start) + (use_exp ? 5 : 0) > size`, e.g. `json_dumps(json_real(1.0/3.0), JSON_ENCODE_ANY\|JSON_REAL_PRECISION(31))` | `-1` -> `do_dump` -1 -> `json_dumps` NULL | TEST |
| 303 | `jsonp_dtostr` (DTOA_ENABLED=0) | `snprintf("%.*g")` returns `< 0` | `-1` | DEAD |
| 304 | `jsonp_dtostr` (DTOA_ENABLED=0) | `(size_t)ret >= size` | `-1` | DEAD |
| 305 | `jsonp_dtostr` (DTOA_ENABLED=0) | no `.`/`e` in output and `length + 3 >= size` | `-1` | DEAD |

Rows 303-305 are in the `#if DTOA_ENABLED == 0` branch. The build sets `DTOA_ENABLED 1`, so
that code does not compile (status `DEAD`). Assert: `assert(end == strbuffer->value + length)`
(`strconv.c:53`) is an internal invariant (`INT`).

## utf.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 306 | `utf8_encode` | `codepoint < 0` | `-1` | TEST |
| 307 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | TEST |
| 308 | `utf8_encode` | QUIRK: surrogates `0xD800-0xDFFF` are ACCEPTED and encoded (no rejection) | `0` | TEST |
| 309 | `utf8_check_first` | byte in `0x80-0xBF` (bare continuation byte) | `0` | TEST |
| 310 | `utf8_check_first` | byte `0xC0` or `0xC1` (overlong 2-byte lead) | `0` | TEST |
| 311 | `utf8_check_first` | byte `>= 0xF5` | `0` | TEST |
| 312 | `utf8_check_first` | NOTE: byte `0x00` returns `1` — NUL is a valid 1-byte char everywhere `utf8_check_string` is used | `1` | TEST |
| 313 | `utf8_check_full` | `size` is not 2, 3 or 4 | `0` | TEST |
| 314 | `utf8_check_full` | any byte at index `1..size-1` is `< 0x80` or `> 0xBF` | `0` | TEST |
| 315 | `utf8_check_full` | decoded value `> 0x10FFFF`, e.g. `"\xF4\xBF\xBF\xBF"` | `0` | TEST |
| 316 | `utf8_check_full` | decoded value in `0xD800-0xDFFF`, e.g. `"\xED\xA0\x80"` | `0` | TEST |
| 317 | `utf8_check_full` | overlong: size 2 and value `<0x80`; size 3 and value `<0x800` (`"\xE0\x80\x80"`); size 4 and value `<0x10000` (`"\xF0\x80\x80\x80"`) | `0` | TEST |
| 318 | `utf8_iterate` | `bufsize == 0` -> returns `buffer` UNCHANGED and does NOT write `*codepoint` (sentinel, not NULL) | `buffer` | TEST |
| 319 | `utf8_iterate` | `utf8_check_first(buffer[0]) == 0` | `NULL` | TEST |
| 320 | `utf8_iterate` | `count > bufsize` (sequence truncated by the buffer end) | `NULL` | TEST |
| 321 | `utf8_iterate` | `utf8_check_full` fails | `NULL` | TEST |
| 322 | `utf8_check_string` | some byte has `utf8_check_first == 0` | `0` | TEST |
| 323 | `utf8_check_string` | multi-byte lead at index i with `count > length - i`, e.g. `("\xE2\x82", 2)` | `0` | TEST |
| 324 | `utf8_check_string` | `utf8_check_full` fails for a sequence | `0` | TEST |

## hashtable.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 325 | `hashtable_init` | `jsonp_malloc(8 * sizeof(bucket_t))` fails | `-1` | OOM |
| 326 | `hashtable_do_rehash` | `jsonp_malloc(new_size * sizeof(bucket_t))` fails | `-1` | OOM |
| 327 | `init_pair` | `key_len >= (size_t)-1 - offsetof(pair_t,key)` (integer-overflow guard) | `NULL` | TEST |
| 328 | `init_pair` | `jsonp_malloc(offsetof + key_len + 1)` fails | `NULL` | OOM |
| 329 | `hashtable_set` | load ratio exceeded and `hashtable_do_rehash` fails | `-1` | OOM |
| 330 | `hashtable_set` | `init_pair` NULL (key_len overflow or OOM); value NOT decref'd here | `-1` | TEST |
| 331 | `hashtable_find_pair` | bucket is empty | `NULL` | TEST |
| 332 | `hashtable_find_pair` | no pair matches (hash, key_len, memcmp) | `NULL` | TEST |
| 333 | `hashtable_get` | key/key_len not present | `NULL` | TEST |
| 334 | `hashtable_do_del` / `hashtable_del` | key/key_len not present | `-1` | TEST |
| 335 | `hashtable_iter` | hashtable is empty | `NULL` | TEST |
| 336 | `hashtable_iter_at` | key/key_len not present | `NULL` | TEST |
| 337 | `hashtable_iter_next` | iter is the last element | `NULL` | TEST |
| 338 | `hashtable_iter_key`/`_key_len`/`_value`/`_set` | NO NULL check on iter | UB | UB |
| 339 | `hashtable_get`/`_del`/`_iter_at` | NO NULL check on key (`hash_str(NULL, len>0)`) | UB | UB |

## memory.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 340 | `jsonp_malloc` | `size == 0` (explicit early return) | `NULL` | TEST |
| 341 | `jsonp_malloc` | underlying `do_malloc` returns NULL | `NULL` | OOM |
| 342 | `jsonp_free` | `ptr == NULL` | no-op | TEST |
| 343 | `jsonp_realloc` | `do_realloc == NULL` and `newSize == 0` -> frees ptr, returns NULL | `NULL` | TEST |
| 344 | `jsonp_realloc` | `do_realloc != NULL` and it returns NULL | `NULL` | OOM |
| 345 | `jsonp_realloc` | emulation path: `do_malloc(newSize)` returns NULL (old ptr left intact, NOT freed) | `NULL` | OOM |
| 346 | `jsonp_strndup` | `jsonp_malloc(len+1)` fails, incl. `len == (size_t)-1` wrapping to `malloc(0)` | `NULL` | TEST |
| 347 | `json_set_alloc_funcs` | no validation: a NULL `malloc_fn`/`free_fn` is accepted (crashes later); silently resets `do_realloc` to NULL | void | UB |
| 348 | `json_get_alloc_funcs`/`_2` | NULL out-parameters are skipped individually | void | TEST |

## error.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 349 | `jsonp_error_init` | `error == NULL` | no-op | TEST |
| 350 | `jsonp_error_init` | `source == NULL` -> `error->source[0] = '\0'` | void | TEST |
| 351 | `jsonp_error_set_source` | `error == NULL` | no-op | TEST |
| 352 | `jsonp_error_set_source` | `source == NULL` | no-op | TEST |
| 353 | `jsonp_error_set_source` | `strlen(source) >= 80`: truncated to `"..."` + last 76 chars | void | TEST |
| 354 | `jsonp_error_vset` | `error == NULL` | no-op | TEST |
| 355 | `jsonp_error_vset` | `error->text[0] != '\0'`: message/line/column/position/code all DISCARDED (first error wins) | void | TEST |
| 356 | `jsonp_error_vset` | message longer than 158 chars: `vsnprintf` caps at 159, then `text[158]='\0'` | void, code still at `text[159]` | TEST |
| 357 | `json_error_code` (inline) | reads `text[159]`; on a never-populated `json_error_t` this is whatever byte is there | enum value | TEST |

## strbuffer.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 358 | `strbuffer_init` | `jsonp_malloc(STRBUFFER_MIN_SIZE == 16)` fails | `-1` | OOM |
| 359 | `strbuffer_append_bytes` | growth needed and `strbuff->size > SIZE_MAX/2` | `-1` | TEST |
| 360 | `strbuffer_append_bytes` | growth needed and `size > SIZE_MAX-1` | `-1` | TEST |
| 361 | `strbuffer_append_bytes` | growth needed and `strbuff->length > SIZE_MAX-1-size` | `-1` | TEST |
| 362 | `strbuffer_append_bytes` | `jsonp_realloc` fails (old buffer preserved) | `-1` | OOM |
| 363 | `strbuffer_append_byte` | propagates `strbuffer_append_bytes` failure | `-1` | TEST |
| 364 | `strbuffer_pop` | `strbuff->length == 0` (empty) | `'\0'` | TEST |
| 365 | `strbuffer_value` | after `strbuffer_steal_value`, value is NULL | `NULL` | TEST |
| 366 | `strbuffer_clear` | after `strbuffer_steal_value`, dereferences NULL `value[0]` | UB | UB |

## version.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 367 | `jansson_version_str` | no failure mode | `"2.15.0"` | TEST |
| 368 | `jansson_version_cmp` | requested newer than 2.15.0 -> negative (major, then minor, then micro) | negative int | TEST |
| 369 | `jansson_version_cmp` | requested older -> positive; equal -> 0 | int | TEST |

## hashtable_seed.c

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 370 | `seed_from_urandom` | `open("/dev/urandom", O_RDONLY) == -1` | `1` (falls back to timestamp+pid) | INT |
| 371 | `seed_from_urandom` | `read()` returns `!= sizeof(uint32_t)` | `1` (falls back) | INT |
| 372 | `generate_seed` | computed seed `== 0` -> forced to `1` (0 is the unseeded sentinel) | `1` | INT |
| 373 | `json_object_seed` | no error return at all; `json_object_seed(0)` means autoseed; a non-zero seed is IGNORED if `hashtable_seed` is already non-zero | void | TEST |

## Verification record

Every row was exercised against BOTH `.so` files through their exported symbols only
(via `libloading`), comparing the return value AND the full `json_error_t` snapshot
(line, column, position, source, message text, and the error code smuggled into
`text[159]`) — not merely "both failed somehow".

| rows | where tested |
|------|--------------|
| 1-134 (`value.c`) | `tests/phase_b_value.rs`, `tests/phase_c_boundaries.rs` |
| 135-179 (lexer/parser) | `tests/phase_c_lex_errors.rs` |
| 180-194 (load entry points) | `tests/phase_c_entry_errors.rs`, `tests/phase_b_load.rs` |
| 195-240 (`dump.c`) | `tests/phase_c_entry_errors.rs`, `tests/phase_b_dump.rs` |
| 241-298 (`pack_unpack.c`) | `tests/phase_b_packunpack.rs` |
| 299-305 (`strconv.c`) | `tests/phase_c_entry_errors.rs` (the `JSON_REAL_PRECISION` cut-over), `tests/phase_b_lowlevel.rs` |
| 306-324 (`utf.c`) | `tests/phase_c_boundaries.rs`, `tests/phase_b_lowlevel.rs` |
| 325-339 (`hashtable.c`) | `tests/phase_b_lowlevel.rs` |
| 340-348 (`memory.c`) | `tests/phase_c_boundaries.rs`, `tests/phase_c_oom.rs` |
| 349-357 (`error.c`) | `tests/phase_c_entry_errors.rs`, `tests/phase_c_boundaries.rs` |
| 358-366 (`strbuffer.c`) | `tests/phase_b_lowlevel.rs` |
| 367-373 (`version.c`, seeding) | `tests/phase_c_boundaries.rs`, `tests/phase_b_lowlevel.rs` |
| all 58 `OOM` rows | `tests/phase_c_oom.rs` |

### How the `OOM` rows are reached

They are unreachable with a working allocator, so `tests/phase_c_oom.rs` installs a
failing allocator through the public `json_set_alloc_funcs` / `json_set_alloc_funcs2`
hooks and **fails the Nth allocation for N = 1..K**, walking the failure point
through every internal allocation site of each operation. Both the malloc-only mode
(which forces `jsonp_realloc`'s malloc+memcpy+free emulation path) and the
malloc+realloc mode are swept.

That sweep also asserts something no happy-path test can: **the C and the Rust
perform the same number of allocations, in the same order**, for every operation
tested. Two guards keep the sweep from going vacuous — it asserts the operation
allocated at least once through the hook, and that injecting failures actually
changed the outcome for at least one N.

### Rows deliberately NOT exercised, with reasons

- **7 `UB` rows** (291, 292, 298, 338, 339, 347, 366): the C performs **no** check, so
  the call is undefined behavior and crashes *both* libraries identically. Verified
  by construction rather than by test — e.g. `json_stringn(_, (size_t)-1)` makes
  `utf8_check_string` scan `(size_t)-1` bytes, and `hashtable_get(_, NULL, len)` hands
  NULL to `hash_str`. Attempting these segfaults the C too, so a "differential test"
  would only compare two crashes.
- **9 `INT` rows** (110, 142, 159, 207, 236, 301, 370, 371, 372): defensive branches not
  reachable through the public API. Notably row 142 is unreachable *by construction* —
  `lex_scan_string`'s first pass already validates the hex digits, so the second pass's
  `decode_unicode_escape` failure cannot fire; the observed message for
  `["\uD888\uZZZZ"]` is `"invalid escape"` (row 140), which IS tested.
- **3 `DEAD` rows** (303, 304, 305): inside `#if DTOA_ENABLED == 0`. The build sets
  `DTOA_ENABLED 1`, so that code does not compile at all.

### The "Rust panics where C wraps" divergence class

Probing these boundary rows exposed a whole class of real divergence: arithmetic
translated from C that **panics in Rust but wraps in C**. A panic crossing the FFI
boundary aborts the process — behavior the C never exhibits. Most instances are
invisible in a default release build (overflow checks off) and only appear under
the debug profile or with `-C overflow-checks=on`.

Confirmed panics reproduced against the C, then fixed:

| # | site | C expression | trigger |
|---|------|--------------|---------|
| D1 | `memory.rs` `jsonp_strndup` | `jsonp_malloc(len + 1)` | `len == (size_t)-1` wraps to `malloc(0)` -> NULL (row 346) |
| D2 | `version.rs` `jansson_version_cmp` | `JANSSON_MAJOR_VERSION - major` | `major == INT_MIN` (rows 368-369) |
| D3 | `dtoa.rs:2006` | `*decpt = k + 1` | `dtoa_r(1.0, 3, INT_MIN, ..)` reaches `no_digits` with `k == INT_MAX`; C reports `decpt == -2147483648` |
| D4 | `load.rs` `callback_get` | `c = stream->data[stream->pos]` | a `json_load_callback_t` returning more than `MAX_BUF_LEN` leaves `pos >= 1024`; C does an unchecked read into adjacent struct fields |
| D5 | `strbuffer.rs:92` | `strbuff->size - strbuff->length` | a caller-owned `strbuffer_t` with `length > size`; C wraps and skips the grow branch |

Item 4 is the most serious: **slice bounds checks are on in every profile**, so that
one aborted even in the shipped release build.

Four further sites were hardened where the C is likewise unguarded and the overflow
is reachable in principle: `dtoa.rs:1158,1160` (`ndigits + k + 1`), `dump.rs:59,67,258`
(`buf->used + size`, `k1->len - k2->len`), `value.rs:642,648-651` (`json_array_grow`'s
`max(size + amount, size * 2)`), and the `line`/`column` counters in `load.rs` and
`pack_unpack.rs`.

Sites deliberately left with plain operators, because the C's own guards make
overflow impossible and adding a guard would itself be a divergence:
`strbuffer.c:66-69`'s `SIZE_MAX` checks (whose left-to-right short-circuit bounds
`length + size + 1` by exactly `SIZE_MAX`), and `init_pair`'s
`key_len >= (size_t)-1 - offsetof(pair_t, key)` guard.

Verified clean by running the entire suite three ways — release, debug
(overflow-checks on), and `RUSTFLAGS="-C overflow-checks=on" cargo test --release`
— with zero panics, plus a byte-identical differential driver under the
overflow-checks build.
