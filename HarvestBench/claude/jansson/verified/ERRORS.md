# ERRORS.md — Error-surface table (derived from C source)

Every distinct rejection/error branch in the public API. Rust must return the
same sentinel (NULL / -1 / 0 / error code).
> **Status: ALL rows covered by passing differential tests in
> `tests/phase_c_errors.rs`** (rows 1-97). Each test constructs the exact
> invalid input and asserts C and Rust return the identical sentinel
> (NULL / -1 / 0) and, for parser errors, the identical json_error_code,
> line, column, position, AND full error text/source strings.



## value.c — construction / getters / setters

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 1 | json_string | value == NULL | NULL |
| 2 | json_string | value not valid UTF-8 | NULL |
| 3 | json_stringn | value == NULL | NULL |
| 4 | json_stringn | value not valid UTF-8 (len bytes) | NULL |
| 5 | json_string_nocheck | value == NULL | NULL |
| 6 | json_stringn_nocheck | value == NULL | NULL |
| 7 | json_string_value | not a JSON_STRING | NULL |
| 8 | json_string_length | not a JSON_STRING | 0 |
| 9 | json_string_set | value == NULL | -1 |
| 10 | json_string_set | value not valid UTF-8 | -1 |
| 11 | json_string_setn | value not valid UTF-8 | -1 |
| 12 | json_string_set_nocheck | value == NULL | -1 |
| 13 | json_string_setn_nocheck | not JSON_STRING or value NULL | -1 |
| 14 | json_integer_value | not a JSON_INTEGER | 0 |
| 15 | json_integer_set | not a JSON_INTEGER | -1 |
| 16 | json_real | value is NaN or Inf | NULL |
| 17 | json_real_value | not a JSON_REAL | 0.0 |
| 18 | json_real_set | not JSON_REAL, or NaN/Inf | -1 |
| 19 | json_number_value | not integer nor real | 0.0 |
| 20 | json_object_size | not a JSON_OBJECT | 0 |
| 21 | json_object_get | key == NULL | NULL |
| 22 | json_object_getn | key NULL or not object | NULL |
| 23 | json_object_set_new / _nocheck | key == NULL | -1 (decref value) |
| 24 | json_object_setn_new_nocheck | value == NULL | -1 |
| 25 | json_object_setn_new_nocheck | key NULL, not object, or json==value | -1 (decref) |
| 26 | json_object_setn_new | key NULL or invalid UTF-8 | -1 (decref) |
| 27 | json_object_del | key == NULL | -1 |
| 28 | json_object_deln | key NULL or not object | -1 |
| 29 | json_object_deln | key not present (hashtable_del) | -1 |
| 30 | json_object_clear | not a JSON_OBJECT | -1 |
| 31 | json_object_update* | either arg not object | -1 |
| 32 | json_object_iter | not a JSON_OBJECT | NULL |
| 33 | json_object_iter_at | key NULL or not object | NULL |
| 34 | json_object_iter_next | not object or iter NULL | NULL |
| 35 | json_object_iter_key | iter == NULL | NULL |
| 36 | json_object_iter_key_len | iter == NULL | 0 |
| 37 | json_object_iter_value | iter == NULL | NULL |
| 38 | json_object_iter_set_new | not object, iter NULL, or value NULL | -1 (decref) |
| 39 | json_object_key_to_iter | key == NULL | NULL |
| 40 | json_array_size | not a JSON_ARRAY | 0 |
| 41 | json_array_get | not array, or index >= entries | NULL |
| 42 | json_array_set_new | value NULL | -1 |
| 43 | json_array_set_new | not array or json==value | -1 (decref) |
| 44 | json_array_set_new | index >= entries | -1 (decref) |
| 45 | json_array_append_new | value NULL | -1 |
| 46 | json_array_append_new | not array or json==value | -1 (decref) |
| 47 | json_array_insert_new | value NULL | -1 |
| 48 | json_array_insert_new | not array or json==value | -1 (decref) |
| 49 | json_array_insert_new | index > entries | -1 (decref) |
| 50 | json_array_remove | not array | -1 |
| 51 | json_array_remove | index >= entries | -1 |
| 52 | json_array_clear | not array | -1 |
| 53 | json_array_extend | either arg not array | -1 |
| 54 | json_equal | either arg NULL | 0 |
| 55 | json_equal | differing types | 0 |
| 56 | json_copy | json NULL | NULL |
| 57 | json_deep_copy | json NULL | NULL |

## load.c — parser

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 58 | json_loads | input == NULL | NULL, error invalid_argument |
| 59 | json_loadb | buffer == NULL | NULL, error invalid_argument |
| 60 | json_loadf | input == NULL | NULL, error invalid_argument |
| 61 | json_load_file | path == NULL | NULL, error invalid_argument |
| 62 | json_load_file | file cannot be opened | NULL, error cannot_open_file |
| 63 | json_load_callback | callback NULL | NULL, error invalid_argument |
| 64 | parser | premature end of input | NULL, error premature_end_of_input |
| 65 | parser | invalid UTF-8 in stream | NULL, error invalid_utf8 |
| 66 | parser | unexpected newline in string | NULL, error invalid_syntax |
| 67 | parser | control char in string | NULL, error invalid_syntax |
| 68 | parser | invalid escape | NULL, error invalid_syntax |
| 69 | parser | invalid \uXXXX hex | NULL, error invalid_syntax |
| 70 | parser | integer too big | NULL, error numeric_overflow |
| 71 | parser | real overflow | NULL, error numeric_overflow |
| 72 | parser | string or '}' expected | NULL, error invalid_syntax |
| 73 | parser | NUL byte in key (no ALLOW_NUL) | NULL, error null_byte_in_key |
| 74 | parser | duplicate key + REJECT_DUPLICATES | NULL, error duplicate_key |
| 75 | parser | ':' expected | NULL, error invalid_syntax |
| 76 | parser | '}' expected | NULL, error invalid_syntax |
| 77 | parser | ']' expected | NULL, error invalid_syntax |
| 78 | parser | max parse depth reached | NULL, error stack_overflow |
| 79 | parser | NUL char in string (no ALLOW_NUL) | NULL, error null_character |
| 80 | parser | invalid token | NULL, error invalid_syntax |
| 81 | parser | '[' or '{' expected (no DECODE_ANY) | NULL, error invalid_syntax |
| 82 | parser | trailing garbage (no DISABLE_EOF_CHECK) | NULL, error end_of_input_expected |
| 83 | parser | empty input | NULL, error premature_end_of_input |

## dump.c — encoder

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 84 | json_dumps | root not container & no ENCODE_ANY | NULL |
| 85 | json_dumpb | root not container & no ENCODE_ANY | 0 |
| 86 | json_dump* | invalid type (do_dump default) | -1 |
| 87 | json_dumps | json == NULL | NULL |
| 88 | json_dump_callback | callback NULL semantics | see code |

## pack_unpack.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 89 | json_pack | NULL fmt | NULL, error invalid_argument |
| 90 | json_pack | empty/invalid fmt | NULL, error invalid_format |
| 91 | json_unpack | NULL root | -1 |
| 92 | json_unpack | type mismatch | -1, error wrong_type |
| 93 | json_unpack | STRICT extra keys | -1 |
| 94 | json_vunpack_ex | NULL fmt | -1, error invalid_argument |

## utf.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 95 | utf8_encode | codepoint > 0x10FFFF | -1 (invalid) |
| 96 | utf8_check_string | invalid sequence | 0 |
| 97 | utf8_check_first | invalid first byte | 0 |
