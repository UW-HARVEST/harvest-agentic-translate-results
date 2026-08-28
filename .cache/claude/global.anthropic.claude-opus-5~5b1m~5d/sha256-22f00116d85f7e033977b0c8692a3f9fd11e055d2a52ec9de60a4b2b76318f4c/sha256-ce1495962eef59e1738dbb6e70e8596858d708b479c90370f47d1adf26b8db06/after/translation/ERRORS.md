# ERRORS.md — error-surface table (Phase A/C)

Derived mechanically from `c_src/cJSON.c` by enumerating **every** `return false`,
`return NULL`, `return 0`, `goto fail`, sentinel return and explicit range /
null / limit check (`grep -n 'return false\|return NULL\|return 0;\|goto fail'`
matches 169 sites; deduplicated into the rows below, one row per *distinct*
rejection condition). Line numbers refer to `c_src/cJSON.c` unless noted.

Constants that bound the input space (`c_src/cJSON.h`):
`CJSON_NESTING_LIMIT = 1000`, `CJSON_CIRCULAR_LIMIT = 10000`,
`INT_MAX = 2147483647`, `INT_MIN = -2147483648`, `cJSON_IsReference = 256`,
`cJSON_StringIsConst = 512`, type mask `0xFF`.

Legend for the `test` column:
* `err_*` — name of the differential test that covers the row.  Most live in
  `translation/tests/errors.rs`; `err_hooks_*` are in `translation/tests/hooks.rs`
  and `err_ensure_size_limits` is in `translation/tests/bigalloc.rs`.
* `hooks` — reached only with a `cJSON_InitHooks` allocator that fails on demand;
  covered by the budget-allocator sweeps in `translation/tests/hooks.rs`
  (`err_hooks_constructors`, `err_hooks_parse`, `err_hooks_print`,
  `err_hooks_mutators`), which walk the allocation budget from 0 upwards and
  compare the C and Rust results **and** their allocator call counts at every
  budget.
* `unreachable` — the C branch cannot be reached through any public entry point
  (dominated by an earlier check); recorded for completeness, and the reasoning
  is given.

| # | function (C line) | trigger (exact invalid input/condition) | expected C result | test |
|---|-------------------|------------------------------------------|-------------------|------|
| 1 | `cJSON_GetStringValue` (86) | `item == NULL` | `NULL` | `err_get_string_value` |
| 2 | `cJSON_GetStringValue` (86) | `item != NULL`, `(type & 0xFF) != cJSON_String` (all 8 other types + `cJSON_Invalid`) | `NULL` | `err_get_string_value` |
| 3 | `cJSON_GetNumberValue` (96) | `item == NULL` | `(double)NAN` — bits `0xFFF8000000000000` (cJSON.c is built as C89, so glibc's `NAN` is not defined and the library's own `#define NAN 0.0/0.0` yields the *negative* default quiet NaN) | `err_get_number_value` |
| 4 | `cJSON_GetNumberValue` (96) | `item != NULL`, `(type & 0xFF) != cJSON_Number` | `(double)NAN` | `err_get_number_value` |
| 5 | `case_insensitive_strcmp` (125) via `cJSON_GetObjectItem` | either string `NULL` (child with `string == NULL`, i.e. an array element) | `1` (never equal) ⇒ `cJSON_GetObjectItem` returns `NULL` | `err_get_object_item` |
| 6 | `cJSON_strdup` (162) | `string == NULL` | `NULL` | `err_create_string_null` |
| 7 | `cJSON_strdup` (169) | `hooks->allocate` returns `NULL` | `NULL` | `hooks` |
| 8 | `cJSON_New_Item` (210–216) | `hooks->allocate` returns `NULL` | `NULL` (propagates to every `cJSON_Create*`) | `hooks` |
| 9 | `parse_number` (286) | `input_buffer == NULL \|\| content == NULL` | `false` | `unreachable` — `parse_value` (1334) already rejects both |
| 10 | `parse_number` (327) | temp-buffer `allocate` fails | `false` ⇒ parse returns `NULL` | `hooks` |
| 11 | `parse_number` (350) | `strtod` consumed nothing: input starts with a `parse_value`-accepted byte but is not a number — `"-"`, `"-e"`, `"-."`, `"-+"`, `"--1"` | `false` ⇒ `NULL` + `cJSON_GetErrorPtr` set | `err_parse_number_no_digits` |
| 12 | `cJSON_SetValuestring` (405) | `object == NULL` | `NULL` | `err_set_valuestring` |
| 13 | `cJSON_SetValuestring` (405) | `!(object->type & cJSON_String)` (Number/Array/Object/Raw/True/False/NULL/Invalid) | `NULL` | `err_set_valuestring` |
| 14 | `cJSON_SetValuestring` (405) | `object->type & cJSON_IsReference` (from `cJSON_CreateStringReference`) | `NULL` | `err_set_valuestring` |
| 15 | `cJSON_SetValuestring` (410) | `object->valuestring == NULL` (String item whose `valuestring` was cleared) | `NULL` | `err_set_valuestring` |
| 16 | `cJSON_SetValuestring` (410) | `valuestring == NULL` | `NULL` | `err_set_valuestring` |
| 17 | `cJSON_SetValuestring` (421) | `strlen(new) <= strlen(old)` **and** the two buffers overlap (`new == object->valuestring`, or a suffix/prefix of it) | `NULL`, `valuestring` unchanged | `err_set_valuestring_overlap` |
| 18 | `cJSON_SetValuestring` (429) | `strlen(new) > strlen(old)` and `cJSON_strdup` allocation fails | `NULL` | `hooks` |
| 19 | `ensure` (459) | `p == NULL` | `NULL` | `unreachable` — every caller passes `&stack_object` |
| 20 | `ensure` (459) | `p->buffer == NULL` | `NULL` | `unreachable` from a first call; reached only after an earlier `ensure` failure inside the same print, covered by row 26/27 |
| 21 | `ensure` (465) | `p->length > 0 && p->offset >= p->length` — preallocated buffer exactly filled, another `ensure` follows | `NULL` ⇒ `cJSON_PrintPreallocated` returns `0` | `err_print_preallocated_tight` (the trigger and the `0` are asserted; the guard itself is redundant — see MUTATION.md #13) |
| 22 | `ensure` (471) | `needed > INT_MAX` — a `cJSON_CreateStringReference` payload of 357 913 941 control bytes makes `print_string_ptr` request `6*n + 3 = 2147483649` | `NULL` ⇒ every print entry point rejects | `err_ensure_size_limits` (`bigalloc.rs`) |
| 23 | `ensure` (481) | `p->noalloc != 0` and `offset + needed + 1 > length` — `cJSON_PrintPreallocated` with a buffer that is too small | `NULL` ⇒ returns `0` | `err_print_preallocated_small` |
| 24 | `ensure` (494) | `needed == INT_MAX` on entry, so `needed += p->offset + 1` pushes it past `INT_MAX` — payload of 357 913 940 control bytes plus two `\n` gives `output_length + 3 == INT_MAX` exactly | `NULL` | `err_ensure_size_limits` (`bigalloc.rs`) |
| 25 | `ensure` (512) | `newsize = INT_MAX` and `hooks.reallocate` (always libc `realloc`, see A-axis in CONFIGS.md) returns `NULL`; buffer freed, `length = 0`, `buffer = NULL` | `NULL` | `err_ensure_size_limits` (`bigalloc.rs`) — a 200 MB payload puts `needed` in `(INT_MAX/2, INT_MAX]` so `newsize` becomes `INT_MAX`, and `RLIMIT_AS` is lowered for the duration of the two calls so the ~2 GiB `realloc` fails |
| 26 | `ensure` (525) | `hooks.reallocate == NULL` (custom hooks) **and** `hooks.allocate` returns `NULL` | `NULL` | `hooks` |
| 27 | `print_number` (570) | `output_buffer == NULL` | `false` | `unreachable` — `print_value` (1391) already rejects |
| 28 | `print_number` (598) | `length < 0 \|\| length > 25` (sprintf overrun of `number_buffer[26]`) | `false` | `unreachable` — `%1.17g` of any `double` is ≤ 24 chars |
| 29 | `print_number` (605) | `ensure` fails (see rows 21/23/25/26) | `false` | `err_print_preallocated_small` |
| 30 | `parse_hex4` (650) | a byte in `\uXXXX` is not `[0-9A-Fa-f]` — `"\uZZZZ"`, `"\u00g0"` | returns `0`; **not** an error — `utf16_literal_to_utf8` treats it as codepoint `U+0000` and emits one `0x00` byte, so the parse *succeeds* with an embedded NUL | `err_parse_hex4_invalid` |
| 31 | `utf16_literal_to_utf8` (678) | `input_end - first_sequence < 6` — `"\u12"`, `"\u"`, `"\u123"` at end of string | `0` ⇒ `parse_string` fails ⇒ `NULL` | `err_parse_utf16_truncated` |
| 32 | `utf16_literal_to_utf8` (687) | `first_code ∈ [0xDC00, 0xDFFF]` — lone low surrogate `"\uDC00"`…`"\uDFFF"` | `0` ⇒ `NULL` | `err_parse_utf16_lone_low_surrogate` |
| 33 | `utf16_literal_to_utf8` (700) | high surrogate then `input_end - second_sequence < 6` — `"\uD800\u12"` | `0` ⇒ `NULL` | `err_parse_utf16_truncated_pair` |
| 34 | `utf16_literal_to_utf8` (706) | high surrogate not followed by `\u` — `"\uD800xxxxxx"`, `"\uD800\\n1234"` | `0` ⇒ `NULL` | `err_parse_utf16_missing_second` |
| 35 | `utf16_literal_to_utf8` (715) | `second_code ∉ [0xDC00, 0xDFFF]` — `"\uD800A"`, `"\uD800\uD800"`, `"\uD800"` | `0` ⇒ `NULL` | `err_parse_utf16_bad_second` |
| 36 | `utf16_literal_to_utf8` (757) | `codepoint > 0x10FFFF` | `0` | `unreachable` — max surrogate-pair codepoint is exactly `0x10FFFF` |
| 37 | `parse_string` (796) | first byte is not `"` | `false` | reached through `parse_object`'s key parse (row 47) — `err_parse_object_bad_key` |
| 38 | `parse_string` (811) | last byte of the buffer is `\` — `"abc\` | `false` ⇒ `NULL` | `err_parse_string_trailing_backslash` |
| 39 | `parse_string` (820) | no closing `"` before end of buffer — `"abc` | `false` ⇒ `NULL` | `err_parse_string_unterminated` |
| 40 | `parse_string` (828) | output `allocate` fails | `false` | `hooks` |
| 41 | `parse_string` (846) | `input_end - input_pointer < 1` | `false` | `unreachable` — the `while (input_pointer < input_end)` guard makes the difference ≥ 1 |
| 42 | `parse_string` (878) | `utf16_literal_to_utf8` returned 0 (any of the conditions in rows 31–35) | `false` ⇒ `NULL` | `err_parse_utf16_truncated`, `err_parse_utf16_lone_low_surrogate`, `err_parse_utf16_truncated_pair`, `err_parse_utf16_missing_second`, `err_parse_utf16_bad_second` |
| 43 | `parse_string` (883) | unknown escape byte (`default:`) — `"\q"`, `"\x41"`, `"\ "`, `"\0"`, `"\U0041"` | `false` ⇒ `NULL` | `err_parse_string_bad_escape` |
| 44 | `print_string_ptr` (927) | `output_buffer == NULL` | `false` | `unreachable` — callers already checked |
| 45 | `print_string_ptr` (936) | `input == NULL` (String item with `valuestring == NULL`, or object key `string == NULL`) **and** `ensure(3)` fails | `false` | `err_print_preallocated_small` |
| 46 | `print_string_ptr` (972) | `ensure(output_length + 3)` fails | `false` | `err_print_preallocated_small` |
| 47 | `buffer_skip_whitespace` (1056) / `skip_utf8_bom` (1082) | `buffer == NULL \|\| content == NULL \|\| offset != 0` | `NULL` ⇒ `parse_value` gets `NULL` ⇒ `false` | `unreachable` — `cJSON_ParseWithLengthOpts` always calls it with `offset == 0` and non-NULL content |
| 48 | `cJSON_ParseWithOpts` (1099) | `value == NULL` | `NULL`; **`global_error` is NOT reset** (still whatever the previous failing parse left) | `err_parse_with_opts_null` |
| 49 | `cJSON_ParseWithLengthOpts` (1120) | `value == NULL` | `NULL`; `global_error` reset to `{NULL, 0}` ⇒ `cJSON_GetErrorPtr() == NULL` | `err_parse_with_length_opts_null` |
| 50 | `cJSON_ParseWithLengthOpts` (1120) | `buffer_length == 0` (value non-NULL) | `NULL`; `global_error = {value, 0}` ⇒ `cJSON_GetErrorPtr() == value` | `err_parse_zero_length` |
| 51 | `cJSON_ParseWithLengthOpts` (1131) | `cJSON_New_Item` fails | `NULL` | `hooks` |
| 52 | `cJSON_ParseWithLengthOpts` (1137) | `parse_value` fails (rows 11, 38–43, 53, 58–65, 68) | `NULL`; `global_error.position = offset` if `offset < length` else `length - 1` | `err_parse_error_ptr` |
| 53 | `cJSON_ParseWithLengthOpts` (1146) | `require_null_terminated != 0` and `offset >= length` or the byte at `offset` is not `\0` — trailing garbage `"1 x"`, or `cJSON_ParseWithLengthOpts("[1]", 3, _, 1)` (no room for the NUL) | `NULL` | `err_parse_require_null_terminated` |
| 54 | `print` (1216) | initial 256-byte `allocate` fails | `NULL` | `hooks` |
| 55 | `print` (1222) | `print_value` fails — `cJSON_Print(NULL)`, or an item with an unknown `type` (row 68) | `NULL` | `err_print_null_and_bad_type` |
| 56 | `print` (1231) | `hooks->reallocate` returns `NULL` when **shrinking** the buffer to `offset + 1` | `NULL` | `unreachable in practice` — `cJSON_InitHooks` can only set `reallocate` to libc `realloc` or to `NULL` (cJSON.c:200–204), so this needs libc `realloc` to fail on a shrink. glibc shrinks in place (`mremap_chunk` for mmap'd chunks, split-in-place otherwise) and never needs new address space, so no input or `RLIMIT` makes it fail. The twin branch taken when `reallocate == NULL` (row 57, `allocate` + `memcpy`) **is** exercised by `err_hooks_print`. |
| 57 | `print` (1240) | `hooks->reallocate == NULL` and the final `allocate(offset+1)` fails | `NULL` | `hooks` |
| 58 | `cJSON_PrintBuffered` (1285) | `prebuffer < 0` (`-1`, `INT_MIN`) | `NULL` | `err_print_buffered_negative` |
| 59 | `cJSON_PrintBuffered` (1291) | `allocate(prebuffer)` fails | `NULL` | `hooks` |
| 60 | `cJSON_PrintBuffered` (1304) | `print_value` fails (`item == NULL`, unknown type) | `NULL` | `err_print_buffered_bad_item` |
| 61 | `cJSON_PrintPreallocated` (1316) | `length < 0` (`-1`, `INT_MIN`) | `0` | `err_print_preallocated_negative` |
| 62 | `cJSON_PrintPreallocated` (1316) | `buffer == NULL` | `0` | `err_print_preallocated_null_buffer` |
| 63 | `cJSON_PrintPreallocated` (1326) | `print_value` fails — buffer too small, `item == NULL`, unknown type, `length == 0` | `0` | `err_print_preallocated_small` |
| 64 | `parse_value` (1334) | `input_buffer == NULL` (propagated from row 47) or `content == NULL` | `false` | `unreachable` (see row 47) |
| 65 | `parse_value` (1381) | no type matches: first byte is none of `"`/`-`/`0`-`9`/`[`/`{` and the buffer does not start with `null`/`true`/`false` — `""`, `"x"`, `"nul"`, `"tru"`, `"fals"`, `"NULL"`, `"+1"`, `".5"`, `"'a'"`, `"}"`, `"]"`, `","`, `":"` | `false` ⇒ `NULL` | `err_parse_value_no_match` |
| 66 | `print_value` (1391) | `item == NULL` | `false` | `err_print_null_and_bad_type` |
| 67 | `print_value` (1391) | `output_buffer == NULL` | `false` | `unreachable` — all three callers pass a stack object |
| 68 | `print_value` (1454) | `(type & 0xFF)` is not one of the 8 valid types — `0` (`cJSON_Invalid`), `3` (`False\|True`), `0x0A`, `0x18`, `0xFF`, `0x88`; also `type = 256` (`cJSON_IsReference` only) | `false` ⇒ `cJSON_Print` returns `NULL`, `cJSON_PrintPreallocated` returns `0` | `err_print_null_and_bad_type` |
| 69 | `print_value` (1431) | `(type & 0xFF) == cJSON_Raw` and `valuestring == NULL` | `false` | `err_print_raw_null_valuestring` |
| 70 | `print_value` (1400 / 1409 / 1418 / 1438) | `ensure` fails while emitting `null` / `false` / `true` / a Raw payload | `false` | `err_print_preallocated_small` |
| 71 | `parse_array` (1466) | `input_buffer->depth >= 1000` — 1000 nested `[` | `false` ⇒ `NULL` (1000 levels parse, 1001 fail) | `err_parse_nesting_limit` |
| 72 | `parse_array` (1473) | `buffer[offset] != '['` | `false` | `unreachable` — `parse_value` (1371) already checked the `[` |
| 73 | `parse_array` (1488) | after `[` and whitespace the buffer is exhausted — `cJSON_ParseWithLength("[", 1)`, `"[ "` (len 2) | `false`, `offset` decremented ⇒ `NULL` | `err_parse_array_truncated` |
| 74 | `parse_array` (1500) | `cJSON_New_Item` fails | `false` | `hooks` |
| 75 | `parse_array` (1522) | element `parse_value` fails — `"[,]"`, `"[1,]"`, `"[x]"`, `"[1,,2]"` | `false` ⇒ `NULL` | `err_parse_array_bad_element` |
| 76 | `parse_array` (1530) | no closing `]` — `"[1"`, `"[1 "`, `"[1,2"` | `false` ⇒ `NULL` | `err_parse_array_unclosed` |
| 77 | `print_array` (1565) | `output_buffer == NULL` | `false` | `unreachable` — the only caller is `print_value`, which rejects `output_buffer == NULL` at line 1391 before dispatching |
| 78 | `print_array` (1573 / 1593 / 1609) | `ensure` fails for `[`, `, `/`,` or `]` | `false` | `err_print_preallocated_small` |
| 79 | `print_array` (1584) | element `print_value` fails (unknown child type, Raw with NULL payload) | `false` ⇒ `cJSON_Print` returns `NULL` | `err_print_null_and_bad_type` |
| 80 | `parse_object` (1626) | `input_buffer->depth >= 1000` — 1000 nested `{"a":` | `false` ⇒ `NULL` | `err_parse_nesting_limit` |
| 81 | `parse_object` (1632) | `cannot_access_at_index(0) \|\| buffer[offset] != '{'` | `false` | `unreachable` — `parse_value` (1376) already checked |
| 82 | `parse_object` (1646) | after `{` and whitespace the buffer is exhausted — `cJSON_ParseWithLength("{", 1)` | `false` ⇒ `NULL` | `err_parse_object_truncated` |
| 83 | `parse_object` (1658) | `cJSON_New_Item` fails | `false` | `hooks` |
| 84 | `parse_object` (1677) | `cannot_access_at_index(1)` — nothing after the `{`/`,` — `cJSON_ParseWithLength("{a", 2)`, `{"a":1,` | `false` ⇒ `NULL` | `err_parse_object_nothing_after_comma` |
| 85 | `parse_object` (1685) | key `parse_string` fails — `"{x:1}"`, `"{1:2}"`, `"{'a':1}"`, `` `{"a:1}` `` | `false` ⇒ `NULL` | `err_parse_object_bad_key` |
| 86 | `parse_object` (1695) | `:` missing after the key — `{"a" 1}`, `{"a"}`, `{"a",1}` | `false` ⇒ `NULL` | `err_parse_object_missing_colon` |
| 87 | `parse_object` (1703) | value `parse_value` fails — `{"a":}`, `{"a":x}`, `{"a":,}` | `false` ⇒ `NULL` | `err_parse_object_bad_value` |
| 88 | `parse_object` (1711) | no closing `}` — `{"a":1`, `{"a":1 ` | `false` ⇒ `NULL` | `err_parse_object_unclosed` |
| 89 | `print_object` (1745) | `output_buffer == NULL` | `false` | `unreachable` — the only caller is `print_value`, which rejects `output_buffer == NULL` at line 1391 before dispatching |
| 90 | `print_object` (1753 / 1772 / 1792 / 1813 / 1833) | `ensure` fails for `{`/`\n`, the depth indent, `:`/`\t`, `,`/`\n`, or `}` | `false` | `err_print_preallocated_small` |
| 91 | `print_object` (1784) | key `print_string_ptr` fails (`ensure` failure) | `false` | `err_print_preallocated_small` |
| 92 | `print_object` (1804) | value `print_value` fails (unknown child type) | `false` | `err_print_null_and_bad_type` |
| 93 | `cJSON_GetArraySize` (1858) | `array == NULL` | `0` | `err_get_array_size` |
| 94 | `get_array_item` (1880) | `array == NULL` | `NULL` | `err_get_array_item` |
| 95 | `get_array_item` (1884–1890) | `index >= number of children` (incl. non-container items and empty arrays) | `NULL` | `err_get_array_item` |
| 96 | `cJSON_GetArrayItem` (1897) | `index < 0` (`-1`, `INT_MIN`) | `NULL` | `err_get_array_item` |
| 97 | `get_object_item` (1909) | `object == NULL` | `NULL` | `err_get_object_item` |
| 98 | `get_object_item` (1909) | `name == NULL` | `NULL` | `err_get_object_item` |
| 99 | `get_object_item` (1928) | key not present (both `case_sensitive` values) | `NULL` | `err_get_object_item` |
| 100 | `get_object_item` (1928) | the walk stops on an element whose `string == NULL` (array children) — case-sensitive loop breaks on `string == NULL` | `NULL` | `err_get_object_item` |
| 101 | `cJSON_HasObjectItem` (1947) | any of rows 97–100 | `0` | `err_get_object_item` |
| 102 | `create_reference` (1963) | `item == NULL` — `cJSON_AddItemReferenceToArray(arr, NULL)` | `NULL` ⇒ `add_item_to_array` returns `false` | `err_add_item_reference` |
| 103 | `create_reference` (1969) | `cJSON_New_Item` fails | `NULL` | `hooks` |
| 104 | `add_item_to_array` (1985) | `item == NULL` | `false` | `err_add_item_to_array` |
| 105 | `add_item_to_array` (1985) | `array == NULL` | `false` | `err_add_item_to_array` |
| 106 | `add_item_to_array` (1985) | `array == item` (self-append) | `false` | `err_add_item_to_array` |
| 107 | `add_item_to_array` (2002) | `array->child != NULL` but `array->child->prev == NULL` (corrupted list) | returns **`true`** while silently *not* linking `item` (leak/quirk) | `err_add_item_to_array_corrupt` |
| 108 | `add_item_to_object` (2041) | `object == NULL` | `false` | `err_add_item_to_object` |
| 109 | `add_item_to_object` (2041) | `string == NULL` | `false` | `err_add_item_to_object` |
| 110 | `add_item_to_object` (2041) | `item == NULL` | `false` | `err_add_item_to_object` |
| 111 | `add_item_to_object` (2041) | `object == item` | `false` | `err_add_item_to_object` |
| 112 | `add_item_to_object` (2054) | `cJSON_strdup(string)` fails (non-constant key) | `false` | `hooks` |
| 113 | `cJSON_AddItemReferenceToArray` (2086) | `array == NULL` | `false` | `err_add_item_reference` |
| 114 | `cJSON_AddItemReferenceToObject` (2096) | `object == NULL` | `false` | `err_add_item_reference` |
| 115 | `cJSON_AddItemReferenceToObject` (2096) | `string == NULL` | `false` | `err_add_item_reference` |
| 116 | `cJSON_AddItemReferenceToObject` (2099) | `item == NULL` ⇒ `create_reference` returns `NULL` | `false` | `err_add_item_reference` |
| 117 | `cJSON_AddNullToObject` (2111) | `object == NULL` or `name == NULL` | created item deleted, `NULL` | `err_add_x_to_object` |
| 118 | `cJSON_AddTrueToObject` (2123) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 119 | `cJSON_AddFalseToObject` (2135) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 120 | `cJSON_AddBoolToObject` (2147) | `object == NULL` or `name == NULL` (any `boolean` value) | `NULL` | `err_add_x_to_object` |
| 121 | `cJSON_AddNumberToObject` (2159) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 122 | `cJSON_AddStringToObject` (2171) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 123 | `cJSON_AddStringToObject` (2164) | `string == NULL` ⇒ `cJSON_CreateString(NULL)` is `NULL` ⇒ `add_item_to_object` fails | `NULL`, object unchanged | `err_add_x_to_object` |
| 124 | `cJSON_AddRawToObject` (2183) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 125 | `cJSON_AddRawToObject` (2176) | `raw == NULL` ⇒ `cJSON_CreateRaw(NULL)` is `NULL` | `NULL` | `err_add_x_to_object` |
| 126 | `cJSON_AddObjectToObject` (2195) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 127 | `cJSON_AddArrayToObject` (2207) | `object == NULL` or `name == NULL` | `NULL` | `err_add_x_to_object` |
| 128 | `cJSON_DetachItemViaPointer` (2214) | `parent == NULL` | `NULL` | `err_detach_via_pointer` |
| 129 | `cJSON_DetachItemViaPointer` (2214) | `item == NULL` | `NULL` | `err_detach_via_pointer` |
| 130 | `cJSON_DetachItemViaPointer` (2214) | `item != parent->child && item->prev == NULL` (item not in this list) | `NULL` | `err_detach_via_pointer` |
| 131 | `cJSON_DetachItemFromArray` (2250) | `which < 0` (`-1`, `INT_MIN`) | `NULL` | `err_detach_from_array` |
| 132 | `cJSON_DetachItemFromArray` (2253) | `which >= size` ⇒ `get_array_item` `NULL` | `NULL` | `err_detach_from_array` |
| 133 | `cJSON_DetachItemFromObject[CaseSensitive]` (2263/2270) | key absent / `object == NULL` / `string == NULL` | `NULL` | `err_detach_from_object` |
| 134 | `cJSON_DeleteItemFromArray` (2258) / `…FromObject[CaseSensitive]` (2277/2282) | detach returned `NULL` | no-op (`cJSON_Delete(NULL)`) | `err_delete_item_from` |
| 135 | `cJSON_InsertItemInArray` (2292) | `which < 0` | `false` | `err_insert_item_in_array` |
| 136 | `cJSON_InsertItemInArray` (2292) | `newitem == NULL` | `false` | `err_insert_item_in_array` |
| 137 | `cJSON_InsertItemInArray` (2295–2299) | `which >= size` ⇒ falls back to `add_item_to_array` — with `array == NULL` that is `false`, otherwise appends | `false` / `true` (append) | `err_insert_item_in_array` |
| 138 | `cJSON_InsertItemInArray` (2303) | `after_inserted != array->child && after_inserted->prev == NULL` | `false` | `err_insert_item_corrupt` |
| 139 | `cJSON_ReplaceItemViaPointer` (2324) | `parent == NULL` | `false` | `err_replace_via_pointer` |
| 140 | `cJSON_ReplaceItemViaPointer` (2324) | `parent->child == NULL` (empty container) | `false` | `err_replace_via_pointer` |
| 141 | `cJSON_ReplaceItemViaPointer` (2324) | `replacement == NULL` | `false` | `err_replace_via_pointer` |
| 142 | `cJSON_ReplaceItemViaPointer` (2324) | `item == NULL` | `false` | `err_replace_via_pointer` |
| 143 | `cJSON_ReplaceItemViaPointer` (2328) | `replacement == item` | `true`, nothing changed and nothing deleted | `err_replace_via_pointer` |
| 144 | `cJSON_ReplaceItemInArray` (2373) | `which < 0` | `false` | `err_replace_in_array` |
| 145 | `cJSON_ReplaceItemInArray` (2376) | `which >= size` ⇒ `item == NULL` | `false` | `err_replace_in_array` |
| 146 | `replace_item_in_object` (2383) | `replacement == NULL` | `false` | `err_replace_in_object` |
| 147 | `replace_item_in_object` (2383) | `string == NULL` | `false` | `err_replace_in_object` |
| 148 | `replace_item_in_object` (2392) | `cJSON_strdup(string)` fails | `false` | `hooks` |
| 149 | `replace_item_in_object` (2399) | key absent / `object == NULL` ⇒ `cJSON_ReplaceItemViaPointer` with `item == NULL` | `false` — **but** `replacement->string` has already been replaced and `cJSON_StringIsConst` cleared (observable side effect) | `err_replace_in_object` |
| 150 | `cJSON_CreateString` (2493) | `string == NULL` ⇒ `cJSON_strdup` `NULL` ⇒ item deleted | `NULL` | `err_create_string_null` |
| 151 | `cJSON_CreateRaw` (2543) | `raw == NULL` | `NULL` | `err_create_string_null` |
| 152 | `cJSON_CreateIntArray` (2582) | `count < 0` (`-1`, `INT_MIN`) | `NULL` | `err_create_arrays` |
| 153 | `cJSON_CreateIntArray` (2582) | `numbers == NULL` | `NULL` | `err_create_arrays` |
| 154 | `cJSON_CreateIntArray` (2593) | `cJSON_CreateNumber` fails | array deleted, `NULL` | `hooks` |
| 155 | `cJSON_CreateFloatArray` (2622) | `count < 0` | `NULL` | `err_create_arrays` |
| 156 | `cJSON_CreateFloatArray` (2622) | `numbers == NULL` | `NULL` | `err_create_arrays` |
| 157 | `cJSON_CreateFloatArray` (2633) | `cJSON_CreateNumber` fails | `NULL` | `hooks` |
| 158 | `cJSON_CreateDoubleArray` (2662) | `count < 0` | `NULL` | `err_create_arrays` |
| 159 | `cJSON_CreateDoubleArray` (2662) | `numbers == NULL` | `NULL` | `err_create_arrays` |
| 160 | `cJSON_CreateDoubleArray` (2673) | `cJSON_CreateNumber` fails | `NULL` | `hooks` |
| 161 | `cJSON_CreateStringArray` (2702) | `count < 0` | `NULL` | `err_create_arrays` |
| 162 | `cJSON_CreateStringArray` (2702) | `strings == NULL` | `NULL` | `err_create_arrays` |
| 163 | `cJSON_CreateStringArray` (2713) | any `strings[i] == NULL` ⇒ `cJSON_CreateString` `NULL` | whole array deleted, `NULL` | `err_create_arrays` |
| 164 | `cJSON_Duplicate_rec` (2751) | `item == NULL` — `cJSON_Duplicate(NULL, 0/1)` | `NULL` | `err_duplicate` |
| 165 | `cJSON_Duplicate_rec` (2757) | `cJSON_New_Item` fails | `NULL` | `hooks` |
| 166 | `cJSON_Duplicate_rec` (2768) | `cJSON_strdup(valuestring)` fails | new item deleted, `NULL` | `hooks` |
| 167 | `cJSON_Duplicate_rec` (2776) | `cJSON_strdup(string)` fails (non-const key) | `NULL` | `hooks` |
| 168 | `cJSON_Duplicate_rec` (2789) | `depth >= 10000` — a `->child` chain 10000 deep, or a self-referential `child` cycle | `NULL` | `err_duplicate_circular` |
| 169 | `cJSON_Duplicate_rec` (2794) | recursive duplicate of a child fails | `NULL` | `err_duplicate_circular` |
| 170 | `cJSON_Minify` (2880) | `json == NULL` | returns without writing | `err_minify_null` |
| 171 | `cJSON_IsInvalid` (2928) | `item == NULL` | `0` | `err_type_predicates_null` |
| 172 | `cJSON_IsFalse` (2938) | `item == NULL` | `0` | `err_type_predicates_null` |
| 173 | `cJSON_IsTrue` (2948) | `item == NULL` | `0` | `err_type_predicates_null` |
| 174 | `cJSON_IsBool` (2959) | `item == NULL` | `0` | `err_type_predicates_null` |
| 175 | `cJSON_IsNull` (2968) | `item == NULL` | `0` | `err_type_predicates_null` |
| 176 | `cJSON_IsNumber` (2978) | `item == NULL` | `0` | `err_type_predicates_null` |
| 177 | `cJSON_IsString` (2988) | `item == NULL` | `0` | `err_type_predicates_null` |
| 178 | `cJSON_IsArray` (2998) | `item == NULL` | `0` | `err_type_predicates_null` |
| 179 | `cJSON_IsObject` (3008) | `item == NULL` | `0` | `err_type_predicates_null` |
| 180 | `cJSON_IsRaw` (3018) | `item == NULL` | `0` | `err_type_predicates_null` |
| 181 | `cJSON_Compare` (3028) | `a == NULL` | `0` | `err_compare_reject` |
| 182 | `cJSON_Compare` (3028) | `b == NULL` | `0` | `err_compare_reject` |
| 183 | `cJSON_Compare` (3028) | `(a->type & 0xFF) != (b->type & 0xFF)` | `0` | `err_compare_reject` |
| 184 | `cJSON_Compare` (3045) | `(a->type & 0xFF)` not one of the 8 valid types (equal on both sides) — `0`, `3`, `0x0A`, `0xFF` | `0` | `err_compare_reject` |
| 185 | `cJSON_Compare` (3067) | `cJSON_Number` and `compare_double(a,b) == false` (incl. NaN vs NaN — `fabs(NaN) <= …` is false) | `0` | `err_compare_numbers` |
| 186 | `cJSON_Compare` (3073) | `cJSON_String`/`cJSON_Raw` and either `valuestring == NULL` | `0` | `err_compare_reject` |
| 187 | `cJSON_Compare` (3080) | `cJSON_String`/`cJSON_Raw` and `strcmp != 0` | `0` | `err_compare_strings` |
| 188 | `cJSON_Compare` (3091) | `cJSON_Array` and some element pair compares unequal | `0` | `err_compare_containers` |
| 189 | `cJSON_Compare` (3100) | `cJSON_Array` with different lengths (`a_element != b_element` after the walk) | `0` | `err_compare_containers` |
| 190 | `cJSON_Compare` (3116) | `cJSON_Object` and a key of `a` is missing from `b` (incl. `a_element->string == NULL`) | `0` | `err_compare_containers` |
| 191 | `cJSON_Compare` (3121) | `cJSON_Object` and a matched value pair compares unequal | `0` | `err_compare_containers` |
| 192 | `cJSON_Compare` (3132) | `cJSON_Object` and a key of `b` is missing from `a` (the "subset" re-check) | `0` | `err_compare_containers` |
| 193 | `cJSON_Compare` (3137) | reverse compare of a matched pair fails | `0` | `err_compare_containers` |
| 194 | `cJSON_Compare` (3145) | trailing `default:` | `0` | `unreachable` — dominated by the row-184 switch |
| 195 | `cJSON_SetNumberHelper` (378–394) | `object == NULL` | **no null check** — dereferences and crashes; documented, deliberately not exercised | `unreachable` (would be UB in C) |
| 196 | `cJSON_CreateBool` (2451) | `boolean` is neither 0 nor 1 (`2`, `-1`, `INT_MIN`, `0x10000` — a C enum/`int` can hold any value) | any non-zero ⇒ `cJSON_True`, `0` ⇒ `cJSON_False` | `err_out_of_range_int_args` |
| 197 | `cJSON_Compare` / `get_object_item` / print entry points | `case_sensitive` / `fmt` / `format` / `require_null_terminated` `cJSON_bool` arguments given out-of-range ints (`2`, `-1`, `INT_MIN`) | treated as boolean truthiness — any non-zero behaves like `1` | `err_out_of_range_int_args` |

## Coverage

Mechanically re-derived from the table above (see the audit at the end of
`verify.sh`'s output and the cross-check in the session log — every `err_*` name
mentioned here resolves to a `#[test]` function in `translation/tests/`):

| status | rows | where |
|--------|------|-------|
| a named `err_*` differential test constructs the exact condition and asserts the same sentinel on both sides | **159** | `tests/errors.rs` (66 tests), `tests/bigalloc.rs` (rows 22, 24, 25) |
| reachable only with an allocator that fails on demand; covered by the budget sweeps, which walk the budget from 0 upwards and compare results **and** allocator call counts at every budget | **21** | `tests/hooks.rs` — rows 7, 8, 10, 18, 26, 40, 51, 54, 57, 59, 74, 83, 103, 112, 148, 154, 157, 160, 165, 166, 167 |
| dead code: dominated by an earlier check, with that check named in the row | **17** | rows 9, 19, 20, 27, 28, 36, 41, 44, 47, 64, 67, 72, 77, 81, 89, 194, 195 |
| row 56 only: needs libc `realloc` to fail on a **shrink**, which glibc never does (it shrinks in place and never asks for new address space); its twin branch — the `reallocate == NULL` manual path, row 57 — **is** exercised | **1** | `err_hooks_print` covers row 57 |
| **total** | **197** | |

Beyond the table, the generic boundaries every C API has are covered explicitly:

* **null pointers** — every pointer parameter of every one of the 79 exported
  functions is exercised with `NULL` (rows 1, 3, 12, 16, 48, 49, 62, 66, 94, 97,
  98, 102, 104, 105, 108–110, 113–116, 128, 129, 136, 139, 141, 142, 146, 147,
  153, 156, 159, 162, 164, 170, 171–182, 186);
* **zero and oversized lengths** — `buffer_length` 0 (row 50), `prebuffer` 0 and
  negative (row 58), `PrintPreallocated` `length` 0/negative/`INT_MIN` (rows 61,
  63) and a sweep over *every* length from 0 to `exact + 8`
  (`err_print_preallocated_small`, `cfg30_31_print_preallocated_length_sweep`),
  plus the `INT_MAX` and `INT_MAX/2` capacity thresholds (rows 22, 24, 25);
* **one step past a documented range** — `CJSON_NESTING_LIMIT` at 998/999/1000/
  1001/1002 (rows 71, 80), `CJSON_CIRCULAR_LIMIT` at 9998…10003 (row 168), array
  indices `-1`/`size`/`size+1`/`INT_MIN`/`INT_MAX` (rows 95, 96, 131, 132, 135,
  137, 144, 145), the `INT_MAX`/`INT_MIN` number-saturation boundaries ± 1 ULP
  (rows 5 and 43 in CONFIGS.md), and every UTF-16 surrogate boundary
  (`0xD7FF`/`0xD800`/`0xDBFF`/`0xDC00`/`0xDFFF`/`0xE000`, rows 32, 35);
* **out-of-range enum / `cJSON_bool` values across the FFI boundary** — a C `int`
  parameter accepts any value, so `2`, `3`, `-1`, `0x10000`, `-0x10000`,
  `INT_MIN` and `INT_MAX` are passed to `cJSON_CreateBool`,
  `cJSON_AddBoolToObject`, `cJSON_PrintBuffered(fmt)`,
  `cJSON_PrintPreallocated(format)`, `cJSON_Compare(case_sensitive)`,
  `cJSON_Duplicate(recurse)` and `require_null_terminated` (rows 196, 197 —
  `err_out_of_range_int_args`, `err_parse_require_null_terminated`);
* **out-of-range `cJSON.type` values** — 18 fabricated `type` values with no
  valid variant (`0`, `3`, `5`, `6`, `7`, `9`, `0x0A`, `0x18`, `0x30`, `0x88`,
  `0xFF`, `0x100`, `0x200`, `0x300`, `0x1FF`, `-1`, `INT_MIN`, `INT_MAX`) are
  pushed through every print entry point, every type predicate, every accessor,
  `cJSON_Compare`, `cJSON_SetValuestring` and both container printers (rows 2, 4,
  13, 68, 79, 92, 184).

## Validation of this table

`MUTATION.md` records a mutation-testing run that injects 62 realistic
translation defects and confirms the suite kills 53 of them, with an
equivalence proof for each of the 9 survivors. That is the evidence that the rows
above are actually being *checked* rather than merely listed.
