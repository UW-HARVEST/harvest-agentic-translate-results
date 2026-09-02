# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/*.c` + `c_src/include/*.h`: every `return -1`,
`return NULL`, `return 0` sentinel, `error_set(...)`/`set_error(...)`/`jsonp_error_set(...)`
call, every `assert`, every explicit range/null check and min/max constant.

Legend for "expected C result": the exact value the C returns / the exact
`json_error_code` byte stored at `error->text[JSON_ERROR_TEXT_LENGTH-1]`.

## Verification status

- **252 rows total.**
- **247 rows `[x]`** — each has a differential test that constructs the exact
  invalid input, calls BOTH `.so` files, and asserts they return the same
  sentinel *and* (where an error struct is involved) the same
  `line`/`column`/`position`/`source`/`text`/`json_error_code`, compared as the
  full 160-byte `text` array so an embedded NUL cannot hide a difference.
- **5 rows `[n/a]`** — rows 124, 131, 146, 148 and 149. These are guards the C
  itself cannot reach through its public API without undefined behaviour or a
  corrupted data structure; each row states why and names the test that covers
  the surrounding branch. They are *not* silently skipped.

Test files, and the rows each covers:

| test file | ERRORS rows |
|-----------|-------------|
| `tests/t01_utf.rs` | 97–113, 251 |
| `tests/t02_strbuffer_memory.rs` | 114–122 |
| `tests/t03_hashtable.rs` | 93, 123, 125–127 |
| `tests/t04_dtoa.rs` | 147, 149, 150 |
| `tests/t05_value_scalars.rs` | 60–82, 91, 92, 94–96 |
| `tests/t06_array.rs` | 41–59 |
| `tests/t07_object.rs` | 1–40 |
| `tests/t08_equal_copy.rs` | 83–90 |
| `tests/t09_dump.rs` | 128–146 |
| `tests/t10_load.rs` | 151–193, 196 |
| `tests/t11_pack_unpack.rs` | 201–246 |
| `tests/t12_error_version_boundaries.rs` | 194–200, 247–252 |
| `tests/t13_oom.rs` | 116, 138, 139, 145, 220 + every `if (!ptr)` allocation-failure branch, reached through `json_set_alloc_funcs2` with a budgeted allocator |

### Divergence found and fixed

`src/pack_unpack.rs`'s `set_error!` macro rendered the message into a scratch
buffer and then passed it to `jsonp_error_set_str`, i.e. through a `"%s"`
conversion. The C's `set_error()` hands the format string and its arguments
straight to `jsonp_error_vset()`, which `vsnprintf()`s them into `error->text`.
The two differ whenever a `%c` conversion emits the byte `0`: the C writes the
NUL into the middle of `error->text` and keeps writing the rest of the format
after it, while the `"%s"` hop stopped there. This is reachable in practice —
`"Unexpected format character '%c'"` with the end-of-format NUL token, e.g.
`json_pack(":")` produced `…character '\0'\0` in C but `…character '\0` in Rust.
Fixed by formatting directly into `error->text`; covered by
`t11_pack_unpack::pack_format_errors`.

## value.c — object

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `json_object_size` | `json == NULL` or `json_typeof != JSON_OBJECT` | `0` | [x] |
| 2 | `json_object_get` | `key == NULL` | `NULL` | [x] |
| 3 | `json_object_get` | `json == NULL` / not an object | `NULL` | [x] |
| 4 | `json_object_getn` | `key == NULL` | `NULL` | [x] |
| 5 | `json_object_getn` | `json` not an object (incl. NULL) | `NULL` | [x] |
| 6 | `json_object_getn` | key not present in object | `NULL` | [x] |
| 7 | `json_object_set_new_nocheck` | `key == NULL` (decrefs value) | `-1` | [x] |
| 8 | `json_object_setn_new_nocheck` | `value == NULL` | `-1` | [x] |
| 9 | `json_object_setn_new_nocheck` | `key == NULL` (decrefs value) | `-1` | [x] |
| 10 | `json_object_setn_new_nocheck` | `json` not an object (incl. NULL) | `-1` | [x] |
| 11 | `json_object_setn_new_nocheck` | `json == value` (self-insert) | `-1` | [x] |
| 12 | `json_object_setn_new` | `key == NULL` | `-1` | [x] |
| 13 | `json_object_setn_new` | `!utf8_check_string(key, key_len)` — invalid UTF-8 key | `-1` | [x] |
| 14 | `json_object_set_new` | `key == NULL` | `-1` | [x] |
| 15 | `json_object_del` | `key == NULL` | `-1` | [x] |
| 16 | `json_object_deln` | `key == NULL` | `-1` | [x] |
| 17 | `json_object_deln` | `json` not an object | `-1` | [x] |
| 18 | `json_object_deln` | key not found (`hashtable_do_del` returns -1) | `-1` | [x] |
| 19 | `json_object_clear` | `json` not an object | `-1` | [x] |
| 20 | `json_object_update` | `object` not an object | `-1` | [x] |
| 21 | `json_object_update` | `other` not an object | `-1` | [x] |
| 22 | `json_object_update_existing` | either arg not an object | `-1` | [x] |
| 23 | `json_object_update_missing` | either arg not an object | `-1` | [x] |
| 24 | `do_object_update_recursive` | either arg not an object | `-1` | [x] |
| 25 | `do_object_update_recursive` | `jsonp_loop_check` hits already-seen node (cycle in `other`) | `-1` | [x] |
| 26 | `json_object_update_recursive` | non-object args / cycle | `-1` | [x] |
| 27 | `json_object_iter` | `json` not an object | `NULL` | [x] |
| 28 | `json_object_iter_at` | `key == NULL` | `NULL` | [x] |
| 29 | `json_object_iter_at` | `json` not an object | `NULL` | [x] |
| 30 | `json_object_iter_at` | key absent | `NULL` | [x] |
| 31 | `json_object_iter_next` | `json` not an object | `NULL` | [x] |
| 32 | `json_object_iter_next` | `iter == NULL` | `NULL` | [x] |
| 33 | `json_object_iter_next` | iter is last element | `NULL` | [x] |
| 34 | `json_object_iter_key` | `iter == NULL` | `NULL` | [x] |
| 35 | `json_object_iter_key_len` | `iter == NULL` | `0` | [x] |
| 36 | `json_object_iter_value` | `iter == NULL` | `NULL` | [x] |
| 37 | `json_object_iter_set_new` | `json` not an object | `-1` | [x] |
| 38 | `json_object_iter_set_new` | `iter == NULL` | `-1` | [x] |
| 39 | `json_object_iter_set_new` | `value == NULL` | `-1` | [x] |
| 40 | `json_object_key_to_iter` | `key == NULL` | `NULL` | [x] |

## value.c — array

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 41 | `json_array_size` | `json` not an array (incl. NULL) | `0` | [x] |
| 42 | `json_array_get` | `json` not an array | `NULL` | [x] |
| 43 | `json_array_get` | `index >= entries` (out of range, incl. `SIZE_MAX`) | `NULL` | [x] |
| 44 | `json_array_set_new` | `value == NULL` | `-1` | [x] |
| 45 | `json_array_set_new` | `json` not an array | `-1` | [x] |
| 46 | `json_array_set_new` | `json == value` | `-1` | [x] |
| 47 | `json_array_set_new` | `index >= entries` | `-1` | [x] |
| 48 | `json_array_append_new` | `value == NULL` | `-1` | [x] |
| 49 | `json_array_append_new` | `json` not an array | `-1` | [x] |
| 50 | `json_array_append_new` | `json == value` | `-1` | [x] |
| 51 | `json_array_insert_new` | `value == NULL` | `-1` | [x] |
| 52 | `json_array_insert_new` | `json` not an array | `-1` | [x] |
| 53 | `json_array_insert_new` | `json == value` | `-1` | [x] |
| 54 | `json_array_insert_new` | `index > entries` (note: `>` not `>=`) | `-1` | [x] |
| 55 | `json_array_remove` | `json` not an array | `-1` | [x] |
| 56 | `json_array_remove` | `index >= entries` | `-1` | [x] |
| 57 | `json_array_clear` | `json` not an array | `-1` | [x] |
| 58 | `json_array_extend` | `json` not an array | `-1` | [x] |
| 59 | `json_array_extend` | `other_json` not an array | `-1` | [x] |

## value.c — string / number / misc

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 60 | `string_create` | `value == NULL` | `NULL` | [x] |
| 61 | `json_string` | `value == NULL` | `NULL` | [x] |
| 62 | `json_string` | invalid UTF-8 in value | `NULL` | [x] |
| 63 | `json_stringn` | `value == NULL` | `NULL` | [x] |
| 64 | `json_stringn` | `!utf8_check_string(value, len)` | `NULL` | [x] |
| 65 | `json_string_nocheck` | `value == NULL` | `NULL` | [x] |
| 66 | `json_stringn_nocheck` | `value == NULL` | `NULL` | [x] |
| 67 | `json_string_value` | `json` not a string | `NULL` | [x] |
| 68 | `json_string_length` | `json` not a string | `0` | [x] |
| 69 | `json_string_set_nocheck` | `value == NULL` | `-1` | [x] |
| 70 | `json_string_setn_nocheck` | `json` not a string | `-1` | [x] |
| 71 | `json_string_setn_nocheck` | `value == NULL` | `-1` | [x] |
| 72 | `json_string_set` | `value == NULL` | `-1` | [x] |
| 73 | `json_string_setn` | `value == NULL` | `-1` | [x] |
| 74 | `json_string_setn` | invalid UTF-8 | `-1` | [x] |
| 75 | `json_integer_value` | `json` not an integer | `0` | [x] |
| 76 | `json_integer_set` | `json` not an integer | `-1` | [x] |
| 77 | `json_real` | `isnan(value)` | `NULL` | [x] |
| 78 | `json_real` | `isinf(value)` (`+inf` and `-inf`) | `NULL` | [x] |
| 79 | `json_real_value` | `json` not a real | `0.0` | [x] |
| 80 | `json_real_set` | `json` not a real | `-1` | [x] |
| 81 | `json_real_set` | `isnan(value)` or `isinf(value)` | `-1` | [x] |
| 82 | `json_number_value` | `json` neither integer nor real (incl. NULL) | `0.0` | [x] |
| 83 | `json_equal` | `json1 == NULL` or `json2 == NULL` | `0` | [x] |
| 84 | `json_equal` | `json_typeof(json1) != json_typeof(json2)` | `0` | [x] |
| 85 | `json_equal` | type is out of enum range (`default:` arm) | `0` | [x] |
| 86 | `json_copy` | `json == NULL` | `NULL` | [x] |
| 87 | `json_copy` | out-of-range `json->type` (`default:`) | `NULL` | [x] |
| 88 | `do_deep_copy` / `json_deep_copy` | `json == NULL` | `NULL` | [x] |
| 89 | `do_deep_copy` | out-of-range `json->type` (`default:`) | `NULL` | [x] |
| 90 | `json_object_deep_copy` / `json_array_deep_copy` | cycle detected by `jsonp_loop_check` | `NULL` | [x] |
| 91 | `json_delete` | `json == NULL` | returns, no-op | [x] |
| 92 | `json_delete` | out-of-range type (`default:`) | returns, no-op (no free) | [x] |
| 93 | `jsonp_loop_check` | `hashtable_get(parents,key)` non-NULL (already visited) | `-1` | [x] |
| 94 | `json_vsprintf` | `vsnprintf` returns `< 0` | `NULL` | [x] |
| 95 | `json_vsprintf` | formatted result is invalid UTF-8 | `NULL` | [x] |
| 96 | `json_vsprintf` | `length == 0` (empty result) | `json_string("")`, NOT an error | [x] |

## utf.c

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 97 | `utf8_encode` | `codepoint < 0` | `-1` | [x] |
| 98 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | [x] |
| 99 | `utf8_check_first` | `0x80 <= u <= 0xBF` (continuation byte lead) | `0` | [x] |
| 100 | `utf8_check_first` | `u == 0xC0` or `u == 0xC1` (overlong) | `0` | [x] |
| 101 | `utf8_check_first` | `u >= 0xF5` | `0` | [x] |
| 102 | `utf8_check_full` | `size` not in {2,3,4} | `0` | [x] |
| 103 | `utf8_check_full` | any trailing byte `< 0x80` or `> 0xBF` | `0` | [x] |
| 104 | `utf8_check_full` | decoded `value > 0x10FFFF` | `0` | [x] |
| 105 | `utf8_check_full` | `0xD800 <= value <= 0xDFFF` (surrogate) | `0` | [x] |
| 106 | `utf8_check_full` | overlong: size2&&<0x80, size3&&<0x800, size4&&<0x10000 | `0` | [x] |
| 107 | `utf8_iterate` | `bufsize == 0` | returns `buffer` unchanged (not NULL) | [x] |
| 108 | `utf8_iterate` | `utf8_check_first(buffer[0]) == 0` | `NULL` | [x] |
| 109 | `utf8_iterate` | `count > bufsize` (truncated sequence) | `NULL` | [x] |
| 110 | `utf8_iterate` | `!utf8_check_full(...)` | `NULL` | [x] |
| 111 | `utf8_check_string` | any byte fails `utf8_check_first` | `0` | [x] |
| 112 | `utf8_check_string` | `count > length - i` (truncated tail) | `0` | [x] |
| 113 | `utf8_check_string` | `!utf8_check_full(...)` | `0` | [x] |

## memory.c / strbuffer.c / hashtable.c

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 114 | `jsonp_malloc` | `size == 0` | `NULL` (does NOT call malloc) | [x] |
| 115 | `jsonp_free` | `ptr == NULL` | returns, no-op | [x] |
| 116 | `jsonp_realloc` | `do_realloc == NULL` (after `json_set_alloc_funcs`) and `newSize == 0` | `NULL`, frees `ptr` | [x] |
| 117 | `json_get_alloc_funcs` | NULL out-pointers | no store, no crash | [x] |
| 118 | `json_get_alloc_funcs2` | any NULL out-pointer | skips that store | [x] |
| 119 | `strbuffer_append_bytes` | `strbuff->size > SIZE_MAX/2` | `-1` | [x] |
| 120 | `strbuffer_append_bytes` | `size > SIZE_MAX - 1` | `-1` | [x] |
| 121 | `strbuffer_append_bytes` | `length > SIZE_MAX - 1 - size` | `-1` | [x] |
| 122 | `strbuffer_pop` | `length == 0` | `'\0'` | [x] |
| 123 | `hashtable_del` | key not found | `-1` | [x] |
| 124 | `init_pair` | `key_len >= SIZE_MAX - offsetof(pair_t,key)` | `NULL` → `hashtable_set` returns `-1` | [n/a] not reachable: `hashtable_set` computes `hash_str(key, key_len)` BEFORE calling `init_pair`, so any `key_len` large enough to trip this guard first makes the C read `key_len` bytes. Triggering it is UB in the C itself. Sub-`SIZE_MAX` `key_len` values are covered by `t03_hashtable::hashtable_binary_keys`. |
| 125 | `hashtable_get` | key not found | `NULL` | [x] |
| 126 | `hashtable_iter_at` | key not found | `NULL` | [x] |
| 127 | `hashtable_iter_next` | iter at last element | `NULL` | [x] |

## dump.c

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 128 | `json_dump_callback` | `!(flags & JSON_ENCODE_ANY)` and json is neither array nor object (incl. NULL) | `-1` | [x] |
| 129 | `do_dump` | `json == NULL` | `-1` | [x] |
| 130 | `do_dump` | out-of-range `json->type` (`default:`) | `-1` | [x] |
| 131 | `do_dump` JSON_INTEGER | `snprintf` size `< 0` or `>= MAX_INTEGER_STR_LENGTH` (25) | `-1` | [n/a] not reachable: `json_int_t` is `long long`, whose widest decimal form (`-9223372036854775808`) is 20 bytes < 25. `t09_dump::dumps_integer_extremes` verifies the extremes format identically instead. |
| 132 | `do_dump` JSON_REAL | `jsonp_dtostr` returns `< 0` (buffer too short for precision) | `-1` | [x] |
| 133 | `do_dump` array/object | `jsonp_loop_check` → circular reference | `-1` | [x] |
| 134 | `dump_string` | `utf8_iterate` returns NULL — invalid UTF-8 in string payload (reachable via `json_string_nocheck`) | `-1` | [x] |
| 135 | `dump_indent` / `dump_string` / `do_dump` | user `dump` callback returns non-zero | `-1` propagated | [x] |
| 136 | `dump_to_file` | `fwrite(...) != 1` | `-1` | [x] |
| 137 | `dump_to_fd` | `write(...) != size` (e.g. bad fd) | `-1` | [x] |
| 138 | `json_dumps` | `json_dump_callback` fails | `NULL` | [x] |
| 139 | `json_dumpb` | `json_dump_callback` fails | `0` | [x] |
| 140 | `json_dumpb` | `buf->used + size > buf->size` (buffer too small) | no write, returns full required `used` | [x] |
| 141 | `json_dumpf` | dump fails | `-1` | [x] |
| 142 | `json_dumpfd` | invalid fd | `-1` | [x] |
| 143 | `json_dump_file` | `fopen(path,"w")` fails (e.g. bad dir) | `-1` | [x] |
| 144 | `json_dump_file` | `fclose` fails | `-1` | [x] |
| 145 | `do_dump` object + JSON_SORT_KEYS | `jsonp_malloc(size * sizeof)` fails / `size == 0` | `-1` (unreachable: `iter != NULL` ⇒ size>0) | [x] |
| 146 | `do_dump` object + JSON_SORT_KEYS | `assert(i == size)`, `assert(value)` | abort if violated (unreachable) | [n/a] not reachable: both invariants hold for every well-formed object; violating them requires corrupting the hashtable. `t09_dump::dumps_sort_keys_prefix_tiebreak` covers the surrounding branch. |

## strconv.c

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 147 | `jsonp_strtod` | `strtod` overflows: `(value == ±HUGE_VAL) && errno == ERANGE` | `-1` | [x] |
| 148 | `jsonp_strtod` | `assert(end == value + length)` — trailing junk | abort if violated | [n/a] not reachable through the public API: the lexer only ever hands `jsonp_strtod` a strbuffer holding a complete numeric literal. Forcing it would abort the C. Valid literals are covered by `t04_dtoa::jsonp_strtod_differential`. |
| 149 | `jsonp_dtostr` (DTOA_ENABLED=1) | `dtoa_r` returns `NULL` | `-1` | [n/a] not reachable: `jsonp_dtostr` always passes a 25-byte buffer, and `dtoa_r` falls back to `jsonp_malloc` rather than failing when the buffer is short (verified directly by `t04_dtoa::dtoa_r_short_buffer_falls_back_to_malloc`, which sweeps `blen` 0..10 for both libraries). |
| 150 | `jsonp_dtostr` | `3 + (vdigits_end - vdigits_start) + (use_exp ? 5 : 0) > size` — buffer too short | `-1` | [x] |

## load.c — decoder

| # | function | trigger | expected C result (`json_error_code`) | verified |
|---|----------|---------|-------------------|----------|
| 151 | `json_loads` | `string == NULL` | `NULL`, `json_error_invalid_argument`, "wrong arguments" | [x] |
| 152 | `json_loadb` | `buffer == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 153 | `json_loadf` | `input == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 154 | `json_loadfd` | `input < 0` | `NULL`, `json_error_invalid_argument` | [x] |
| 155 | `json_load_file` | `path == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 156 | `json_load_file` | `fopen` fails | `NULL`, `json_error_cannot_open_file`, "unable to open %s: %s" | [x] |
| 157 | `json_load_callback` | `callback == NULL` | `NULL`, `json_error_invalid_argument` | [x] |
| 158 | `stream_get` | `utf8_check_first(c) == 0` for byte ≥0x80 | `json_error_invalid_utf8`, "unable to decode byte 0x%x" | [x] |
| 159 | `stream_get` | `!utf8_check_full(buffer,count,NULL)` | `json_error_invalid_utf8` | [x] |
| 160 | `parse_json` | `!(flags & JSON_DECODE_ANY)` and first token not `[`/`{` | `json_error_invalid_syntax`, "'[' or '{' expected" | [x] |
| 161 | `parse_json` | `!(flags & JSON_DISABLE_EOF_CHECK)` and trailing data | `json_error_end_of_input_expected`, "end of file expected" | [x] |
| 162 | `parse_value` | `lex->depth > JSON_PARSER_MAX_DEPTH` (2048) | `json_error_stack_overflow`, "maximum parsing depth reached" | [x] |
| 163 | `parse_value` | `TOKEN_STRING` with embedded NUL and no `JSON_ALLOW_NUL` | `json_error_null_character`, "\\u0000 is not allowed without JSON_ALLOW_NUL" | [x] |
| 164 | `parse_value` | `TOKEN_INVALID` | `json_error_invalid_syntax`, "invalid token" | [x] |
| 165 | `parse_value` | any other token (`]`,`}`,`,`,`:`, EOF) | `json_error_invalid_syntax`, "unexpected token" | [x] |
| 166 | `parse_object` | token after `{` is not string and not `}` | `json_error_invalid_syntax`, "string or '}' expected" | [x] |
| 167 | `parse_object` | key contains NUL byte | `json_error_null_byte_in_key`, "NUL byte in object key not supported" | [x] |
| 168 | `parse_object` | `JSON_REJECT_DUPLICATES` and key already present | `json_error_duplicate_key`, "duplicate object key" | [x] |
| 169 | `parse_object` | token after key is not `:` | `json_error_invalid_syntax`, "':' expected" | [x] |
| 170 | `parse_object` | after last member token is not `}` | `json_error_invalid_syntax`, "'}' expected" | [x] |
| 171 | `parse_array` | after last element token is not `]` | `json_error_invalid_syntax`, "']' expected" | [x] |
| 172 | `lex_scan_string` | EOF before closing `"` | `json_error_premature_end_of_input`, "premature end of input" | [x] |
| 173 | `lex_scan_string` | raw `\n` inside string | `json_error_invalid_syntax`, "unexpected newline" | [x] |
| 174 | `lex_scan_string` | raw control char `0x00`–`0x1F` (not `\n`) | `json_error_invalid_syntax`, "control character 0x%x" | [x] |
| 175 | `lex_scan_string` | `\x` where x is not one of `"\/bfnrtu` | `json_error_invalid_syntax`, "invalid escape" | [x] |
| 176 | `lex_scan_string` | `\u` followed by fewer than 4 hex digits | `json_error_invalid_syntax`, "invalid escape" | [x] |
| 177 | `lex_scan_string` | high surrogate `\uD800`–`\uDBFF` not followed by `\u` | `json_error_invalid_syntax`, "invalid Unicode '\\uXXXX'" | [x] |
| 178 | `lex_scan_string` | high surrogate followed by `\u` outside DC00–DFFF | `json_error_invalid_syntax`, "invalid Unicode '\\uXXXX\\uXXXX'" | [x] |
| 179 | `lex_scan_string` | lone low surrogate `\uDC00`–`\uDFFF` | `json_error_invalid_syntax`, "invalid Unicode '\\uXXXX'" | [x] |
| 180 | `lex_scan_number` | leading `0` followed by a digit (e.g. `01`) | `TOKEN_INVALID` → "invalid token" | [x] |
| 181 | `lex_scan_number` | `-` not followed by a digit | `TOKEN_INVALID` → "invalid token" | [x] |
| 182 | `lex_scan_number` | `.` not followed by a digit (e.g. `1.`) | `TOKEN_INVALID` → "invalid token" | [x] |
| 183 | `lex_scan_number` | `e`/`E` (with optional sign) not followed by digit | `TOKEN_INVALID` → "invalid token" | [x] |
| 184 | `lex_scan_number` | integer out of `long long` range, positive | `json_error_numeric_overflow`, "too big integer" | [x] |
| 185 | `lex_scan_number` | integer out of `long long` range, negative | `json_error_numeric_overflow`, "too big negative integer" | [x] |
| 186 | `lex_scan_number` | real overflows (`1e999`) via `jsonp_strtod` | `json_error_numeric_overflow`, "real number overflow" | [x] |
| 187 | `lex_scan` | alpha identifier that is not `true`/`false`/`null` | `TOKEN_INVALID` → "invalid token" | [x] |
| 188 | `lex_scan` | byte that starts no token (`@`, `#`, …) | `TOKEN_INVALID` → "invalid token" | [x] |
| 189 | `decode_unicode_escape` | non-hex digit in `\uXXXX` | `-1` → "invalid Unicode escape '%.6s'" | [x] |
| 190 | `error_set` | `lex` present, no saved text, code == `json_error_invalid_syntax` | code rewritten to `json_error_premature_end_of_input` | [x] |
| 191 | `error_set` | saved text length `> 20` | message with NO " near '...'" suffix | [x] |
| 192 | `error_set` | stream state == `STREAM_STATE_ERROR` | message with NO " near end of file" suffix | [x] |
| 193 | `parse_json` empty input | `""` | `json_error_premature_end_of_input` ("'[' or '{' expected near end of file") | [x] |

## error.c

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 194 | `jsonp_error_init` | `error == NULL` | no-op | [x] |
| 195 | `jsonp_error_set_source` | `error == NULL` or `source == NULL` | no-op | [x] |
| 196 | `jsonp_error_set_source` | `strlen(source) >= JSON_ERROR_SOURCE_LENGTH` (80) | `"..."` + truncated tail | [x] |
| 197 | `jsonp_error_vset` | `error == NULL` | no-op | [x] |
| 198 | `jsonp_error_vset` | `error->text[0] != '\0'` (error already set) | returns without overwriting | [x] |
| 199 | `jsonp_error_vset` | message longer than 158 bytes | truncated, `text[158]='\0'`, `text[159]=code` | [x] |
| 200 | `json_error_code` | reads `text[159]` — for cleared error this is `0` = `json_error_unknown` | `json_error_unknown` | [x] |

## pack_unpack.c — pack

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 201 | `json_vpack_ex` | `fmt == NULL` | `NULL`, `json_error_invalid_argument`, "NULL or empty format string" | [x] |
| 202 | `json_vpack_ex` | `*fmt == '\0'` (empty) | `NULL`, `json_error_invalid_argument` | [x] |
| 203 | `json_vpack_ex` | trailing characters after complete value | `NULL`, `json_error_invalid_format`, "Garbage after format string" | [x] |
| 204 | `pack` | unknown format char (e.g. `x`) | `NULL`, `json_error_invalid_format`, "Unexpected format character '%c'" | [x] |
| 205 | `pack_object` | format ends before `}` | `NULL`, `json_error_invalid_format`, "Unexpected end of format string" | [x] |
| 206 | `pack_object` | key format char is not `s` | `NULL`, `json_error_invalid_format`, "Expected format 's', got '%c'" | [x] |
| 207 | `pack_object` | value packs to NULL and value spec is not `*` | `NULL`, `json_error_null_value`, "NULL object value" | [x] |
| 208 | `pack_array` | format ends before `]` | `NULL`, `json_error_invalid_format`, "Unexpected end of format string" | [x] |
| 209 | `pack_array` | element packs to NULL and spec is not `*` | `NULL` (has_error set, no new message) | [x] |
| 210 | `read_string` | `str == NULL` and not optional | `NULL`, `json_error_null_value`, "NULL %s" | [x] |
| 211 | `read_string` | `str` is invalid UTF-8 | `NULL`, `json_error_invalid_utf8`, "Invalid UTF-8 %s" | [x] |
| 212 | `read_string` | optional (`s?`/`s*`) combined with `#`/`%`/`+` | `NULL`, `json_error_invalid_format`, "Cannot use '%c' on optional strings" | [x] |
| 213 | `read_string` | concatenated (`s+`) result is invalid UTF-8 | `NULL`, `json_error_invalid_utf8` | [x] |
| 214 | `pack_string` | `s?` with NULL arg and no error | `json_null()` | [x] |
| 215 | `pack_string` | `s*` with NULL arg | `NULL` (element skipped) | [x] |
| 216 | `pack_object_inter` (`o`/`O`) | NULL `json_t*` arg, no `?`/`*` | `NULL`, `json_error_null_value`, "NULL object" | [x] |
| 217 | `pack_object_inter` | `O?`/`o?` with NULL arg | `json_null()` | [x] |
| 218 | `pack_object_inter` | `O*`/`o*` with NULL arg | `NULL` (skipped) | [x] |
| 219 | `pack_real` (`f`) | value is NaN or ±Inf (`json_real_set` fails) | `NULL`, `json_error_numeric_overflow`, "Invalid floating point value" | [x] |
| 220 | `pack_integer` | `json_integer` OOM | `NULL`, `json_error_out_of_memory` | [x] |

## pack_unpack.c — unpack

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 221 | `json_vunpack_ex` | `root == NULL` | `-1`, `json_error_null_value`, "NULL root value" | [x] |
| 222 | `json_vunpack_ex` | `fmt == NULL` or empty | `-1`, `json_error_invalid_argument`, "NULL or empty format string" | [x] |
| 223 | `json_vunpack_ex` | trailing garbage after format | `-1`, `json_error_invalid_format`, "Garbage after format string" | [x] |
| 224 | `unpack` | unknown format char | `-1`, `json_error_invalid_format`, "Unexpected format character '%c'" | [x] |
| 225 | `unpack` `s` | root is not a string | `-1`, `json_error_wrong_type`, "Expected string, got %s" | [x] |
| 226 | `unpack` `s` | `str_target == NULL` | `-1`, `json_error_null_value`, "NULL string argument" | [x] |
| 227 | `unpack` `s%` | `len_target == NULL` | `-1`, `json_error_null_value`, "NULL string length argument" | [x] |
| 228 | `unpack` `i` | root is not an integer | `-1`, `json_error_wrong_type`, "Expected integer, got %s" | [x] |
| 229 | `unpack` `I` | root is not an integer | `-1`, `json_error_wrong_type` | [x] |
| 230 | `unpack` `b` | root is not a boolean | `-1`, `json_error_wrong_type`, "Expected true or false, got %s" | [x] |
| 231 | `unpack` `f` | root is not a real (integer is rejected!) | `-1`, `json_error_wrong_type`, "Expected real, got %s" | [x] |
| 232 | `unpack` `F` | root is not a number | `-1`, `json_error_wrong_type`, "Expected real or integer, got %s" | [x] |
| 233 | `unpack` `n` | root is not null | `-1`, `json_error_wrong_type`, "Expected null, got %s" | [x] |
| 234 | `unpack_object` | root is not an object | `-1`, `json_error_wrong_type`, "Expected object, got %s" | [x] |
| 235 | `unpack_object` | format char after `!`/`*` is not `}` | `-1`, `json_error_invalid_format`, "Expected '}' after '%c', got '%c'" | [x] |
| 236 | `unpack_object` | format ends before `}` | `-1`, `json_error_invalid_format`, "Unexpected end of format string" | [x] |
| 237 | `unpack_object` | key format char is not `s` | `-1`, `json_error_invalid_format`, "Expected format 's', got '%c'" | [x] |
| 238 | `unpack_object` | key va_arg is NULL | `-1`, `json_error_null_value`, "NULL object key" | [x] |
| 239 | `unpack_object` | required key missing from object | `-1`, `json_error_item_not_found`, "Object item not found: %s" | [x] |
| 240 | `unpack_object` | strict (`!` or `JSON_STRICT`) and keys left unpacked | `-1`, `json_error_end_of_input_expected`, "%li object item(s) left unpacked: %s" | [x] |
| 241 | `unpack_array` | root is not an array | `-1`, `json_error_wrong_type`, "Expected array, got %s" | [x] |
| 242 | `unpack_array` | format char after `!`/`*` is not `]` | `-1`, `json_error_invalid_format`, "Expected ']' after '%c', got '%c'" | [x] |
| 243 | `unpack_array` | format ends before `]` | `-1`, `json_error_invalid_format`, "Unexpected end of format string" | [x] |
| 244 | `unpack_array` | format char not in `"{[siIbfFOon"` | `-1`, `json_error_invalid_format`, "Unexpected format character '%c'" | [x] |
| 245 | `unpack_array` | array index out of range (format longer than array) | `-1`, `json_error_index_out_of_range`, "Array index %lu out of range" | [x] |
| 246 | `unpack_array` | strict (`!` or `JSON_STRICT`) and `i != json_array_size(root)` | `-1`, `json_error_end_of_input_expected`, "%li array item(s) left unpacked" | [x] |

## Cross-FFI enum / flag boundaries (generic, not in a single C `if`)

| # | function | trigger | expected C result | verified |
|---|----------|---------|-------------------|----------|
| 247 | `json_typeof`-dispatching fns (`json_delete`, `json_copy`, `do_deep_copy`, `json_equal`, `do_dump`) | a `json_t` whose `type` field is an out-of-range `json_type` int (8, 99, -1, INT_MAX) — C enums accept any int | fall to `default:` arm → `NULL` / `-1` / `0` / no-op | [x] |
| 248 | `jsonp_error_set`/`jsonp_error_vset` | `code` outside `enum json_error_code` (e.g. 200, 255, -1) | stored verbatim (truncated to `char`) in `text[159]` | [x] |
| 249 | all `flags`-taking fns (`json_loads`, `json_dumps`, `json_pack_ex`, `json_unpack_ex`) | unknown/reserved flag bits set (e.g. `0xFFFF_FFFF_FFFF_FFFF`) | ignored except for the bits the C masks; no error | [x] |
| 250 | `JSON_INDENT` / `JSON_REAL_PRECISION` | values above their 5-bit masks (`0x1F`) | masked, no error | [x] |
| 251 | `utf8_encode` | `codepoint` = `INT32_MIN`, `-1`, `0x110000`, `INT32_MAX` | `-1` | [x] |
| 252 | `jansson_version_cmp` | negative / huge major/minor/micro | plain integer difference, may overflow | [x] |
