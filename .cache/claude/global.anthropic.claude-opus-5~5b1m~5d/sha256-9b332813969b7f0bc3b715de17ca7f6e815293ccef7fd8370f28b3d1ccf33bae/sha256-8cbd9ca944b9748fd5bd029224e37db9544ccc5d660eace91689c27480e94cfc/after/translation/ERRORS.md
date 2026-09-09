# ERRORS.md — error-surface table

Derived mechanically from the C sources in `c_src/src/*.c` (every `return -1`,
`return NULL`, `return 0`-as-failure, `error_set*`/`set_error` call site,
`assert`, explicit range/null/type check and gating constant).

Column `test` names the differential test that constructs the trigger and
asserts C and Rust agree on the *exact* sentinel / `json_error_code`.
`[x]` = row has a passing differential test.

`OOM` in the trigger column marks rows reachable only by making the allocator
fail; these are exercised in `tests/c_errors_oom.rs`, which installs a budgeted
`malloc`/`realloc` into each `.so` via `json_set_alloc_funcs` /
`json_set_alloc_funcs2` and sweeps the budget from 0 upwards so that every
allocation site inside an operation fails in turn.

Rows marked `n/a` are not reachable as a *rejection*: they are either `assert()`s
the C treats as unreachable, or genuine undefined behaviour in the C (a NULL
function pointer being called, a `key_len` that would be read before the guard
runs). Each such row states why.

## Legend of `json_error_code` values

| code | value |
|---|---|
| `json_error_unknown` | 0 |
| `json_error_out_of_memory` | 1 |
| `json_error_stack_overflow` | 2 |
| `json_error_cannot_open_file` | 3 |
| `json_error_invalid_argument` | 4 |
| `json_error_invalid_utf8` | 5 |
| `json_error_premature_end_of_input` | 6 |
| `json_error_end_of_input_expected` | 7 |
| `json_error_invalid_syntax` | 8 |
| `json_error_invalid_format` | 9 |
| `json_error_wrong_type` | 10 |
| `json_error_null_character` | 11 |
| `json_error_null_value` | 12 |
| `json_error_null_byte_in_key` | 13 |
| `json_error_duplicate_key` | 14 |
| `json_error_numeric_overflow` | 15 |
| `json_error_item_not_found` | 16 |
| `json_error_index_out_of_range` | 17 |


## Where each row is tested

| ERRORS.md section | test file / functions |
|---|---|
| A, B, C (value.c) | `tests/c_errors.rs::a_*`, `b_*`, `c_*`; `tests/b_value.rs` |
| D (dump.c) | `tests/c_errors.rs::d*`; `tests/b_encode.rs::a35_*` |
| E (error.c) | `tests/c_errors.rs::e1_e7_*`; `tests/b_internals.rs::m6_m7_m8_*` |
| F (memory.c) | `tests/c_errors.rs::f_memory_rejections`; `tests/c_errors_oom.rs` |
| G (strbuffer.c) | `tests/c_errors.rs::g_strbuffer_guards`; `tests/b_internals.rs::j*` |
| H (hashtable.c) | `tests/c_errors.rs::h_hashtable_*`; `tests/b_internals.rs::i*` |
| I (version.c) | `tests/c_errors.rs::i1_i4_version_cmp` |
| J (utf.c) | `tests/c_errors.rs::j*`; `tests/b_internals.rs::k*` |
| K (strconv.c / dtoa.c) | `tests/c_errors.rs::k*`; `tests/b_internals.rs::l*`; `tests/d_symbols.rs` |
| L, M (load.c) | `tests/c_errors.rs::l_m_load_rejections_exact_codes`, `m_*` |
| N, O (pack_unpack.c) | `tests/c_errors.rs::n_*`, `o_unpack_rejections` |
| P (hashtable_seed.c) | `tests/c_errors.rs::p5_p6_q11_*`; `tests/b_internals.rs::i7_*` |
| Q (FFI boundaries) | `tests/c_errors.rs::q2_q3_*`, `q12_*` |
| all OOM rows | `tests/c_errors_oom.rs` (budgeted allocator, budget swept 0..N) |

Every OOM row is reached by installing a *budgeted* allocator through
`json_set_alloc_funcs` / `json_set_alloc_funcs2` on each `.so` independently and
sweeping the budget from 0 upwards, so each allocation site inside an operation
fails in turn. `tests/c_errors_oom.rs::zz_injection_harness_is_effective` proves
the injection is not vacuous, and `::oom_allocation_counts_match` additionally
proves C and Rust issue the *same number* of allocations — which is what makes
budget-indexed comparison meaningful.

Both the 2-arg `json_set_alloc_funcs` (which NULLs `do_realloc` and forces the
malloc+memcpy+free realloc *emulation* path) and the 3-arg
`json_set_alloc_funcs2` form are swept, since they are different code paths.

Rows are compared on the FULL `json_error_t` — `line`, `column`, `position`,
`source`, the message text, and the `json_error_code` byte at `text[159]` — not
merely on "both failed".

---

## A. `value.c` — objects

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| A1 | `json_object` | `!object` — `jsonp_malloc(sizeof(json_object_t))` returned NULL (OOM) | `NULL` | [x] |
| A2 | `json_object` | `hashtable_init(&object->hashtable)` failed (OOM) | `NULL`, `object` freed | [x] |
| A3 | `json_object_size` | `!json_is_object(json)` — NULL, array, string, int, real, true, false, null | `0` | [x] |
| A4 | `json_object_get` | `!key` — NULL key | `NULL` | [x] |
| A5 | `json_object_getn` | `!key \|\| !json_is_object(json)` | `NULL` | [x] |
| A6 | `json_object_getn` | `hashtable_get` miss — key absent | `NULL` | [x] |
| A7 | `json_object_set_new_nocheck` | `!key` | `-1`, `value` decref'd | [x] |
| A8 | `json_object_setn_new_nocheck` | `!value` — NULL value | `-1` | [x] |
| A9 | `json_object_setn_new_nocheck` | `!key \|\| !json_is_object(json) \|\| json == value` — NULL key / non-object / self-insert | `-1`, `value` decref'd | [x] |
| A10 | `json_object_setn_new_nocheck` | `hashtable_set(...)` failed (OOM) | `-1`, `value` decref'd | [x] |
| A11 | `json_object_set_new` | `!key` | `-1`, `value` decref'd | [x] |
| A12 | `json_object_setn_new` | `!key \|\| !utf8_check_string(key, key_len)` — invalid-UTF-8 key | `-1`, `value` decref'd | [x] |
| A13 | `json_object_del` | `!key` | `-1` | [x] |
| A14 | `json_object_deln` | `!key \|\| !json_is_object(json)` | `-1` | [x] |
| A15 | `json_object_deln` | `hashtable_del` — key not present | `-1` | [x] |
| A16 | `json_object_clear` | `!json_is_object(json)` (incl. NULL) | `-1` | [x] |
| A17 | `json_object_update` | `!json_is_object(object) \|\| !json_is_object(other)` | `-1` | [x] |
| A18 | `json_object_update` | `json_object_setn_nocheck` failed mid-loop (OOM) | `-1`, partial update left in place | [x] |
| A19 | `json_object_update_existing` | `!json_is_object(object) \|\| !json_is_object(other)` | `-1` | [x] |
| A20 | `json_object_update_missing` | `!json_is_object(object) \|\| !json_is_object(other)` | `-1` | [x] |
| A21 | `do_object_update_recursive` | `!json_is_object(object) \|\| !json_is_object(other)` | `-1` | [x] |
| A22 | `do_object_update_recursive` | `jsonp_loop_check(parents, other, ...)` — cycle in `other` | `-1` | [x] |
| A23 | `do_object_update_recursive` | nested `do_object_update_recursive` failed → `res = -1; break` | `-1` | [x] |
| A24 | `do_object_update_recursive` | `json_object_setn_nocheck` failed (OOM) | `-1` | [x] |
| A25 | `json_object_update_recursive` | `hashtable_init(&parents_set)` failed (OOM) | `-1` | [x] |
| A26 | `json_object_iter` | `!json_is_object(json)` | `NULL` | [x] |
| A27 | `json_object_iter_at` | `!key \|\| !json_is_object(json)` | `NULL` | [x] |
| A28 | `json_object_iter_at` | key absent | `NULL` | [x] |
| A29 | `json_object_iter_next` | `!json_is_object(json) \|\| iter == NULL` | `NULL` | [x] |
| A30 | `json_object_iter_next` | end of ordered list | `NULL` | [x] |
| A31 | `json_object_iter_key` | `!iter` | `NULL` | [x] |
| A32 | `json_object_iter_key_len` | `!iter` | `0` | [x] |
| A33 | `json_object_iter_value` | `!iter` | `NULL` | [x] |
| A34 | `json_object_iter_set_new` | `!json_is_object(json) \|\| !iter \|\| !value` | `-1`, `value` decref'd | [x] |
| A35 | `json_object_key_to_iter` | `!key` | `NULL` | [x] |
| A36 | `json_object_equal` (via `json_equal`) | `json_object_size(o1) != json_object_size(o2)` | `0` | [x] |
| A37 | `json_object_equal` (via `json_equal`) | key missing in obj2, or values differ | `0` | [x] |
| A38 | `json_object_copy` | `json_object()` failed (OOM) | `NULL` | [x] |
| A39 | `json_object_deep_copy` | `jsonp_loop_check` — self-referential object | `NULL` | [x] |
| A40 | `json_object_deep_copy` | `json_object()` failed (OOM) | `NULL` | [x] |
| A41 | `json_object_deep_copy` | `json_object_setn_new_nocheck(result, ..., do_deep_copy(...))` failed | `NULL`, `result` decref'd | [x] |
| A42 | `jsonp_loop_check` | pointer key already in `parents` set (cycle) | `-1` | [x] |
| A43 | `jsonp_loop_check` | `hashtable_set(parents, ...)` failed (OOM) | `-1` | [x] |

## B. `value.c` — arrays

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| B1 | `json_array` | `!array` — malloc of `json_array_t` failed | `NULL` | [x] |
| B2 | `json_array` | `!array->table` — malloc of `8 * sizeof(json_t*)` failed | `NULL`, `array` freed | [x] |
| B3 | `json_array_size` | `!json_is_array(json)` (incl. NULL) | `0` | [x] |
| B4 | `json_array_get` | `!json_is_array(json)` (incl. NULL) | `NULL` | [x] |
| B5 | `json_array_get` | `index >= array->entries` — index out of range (incl. `SIZE_MAX`) | `NULL` | [x] |
| B6 | `json_array_set_new` | `!value` | `-1` | [x] |
| B7 | `json_array_set_new` | `!json_is_array(json) \|\| json == value` | `-1`, `value` decref'd | [x] |
| B8 | `json_array_set_new` | `index >= array->entries` | `-1`, `value` decref'd | [x] |
| B9 | `json_array_grow` | `!new_table` — `jsonp_realloc` failed | `NULL` | [x] |
| B10 | `json_array_append_new` | `!value` | `-1` | [x] |
| B11 | `json_array_append_new` | `!json_is_array(json) \|\| json == value` | `-1`, `value` decref'd | [x] |
| B12 | `json_array_append_new` | `!json_array_grow(array, 1)` (OOM) | `-1`, `value` decref'd | [x] |
| B13 | `json_array_insert_new` | `!value` | `-1` | [x] |
| B14 | `json_array_insert_new` | `!json_is_array(json) \|\| json == value` | `-1`, `value` decref'd | [x] |
| B15 | `json_array_insert_new` | `index > array->entries` — one past end is OK, two past is not | `-1`, `value` decref'd | [x] |
| B16 | `json_array_insert_new` | `!json_array_grow(array, 1)` (OOM) | `-1`, `value` decref'd | [x] |
| B17 | `json_array_remove` | `!json_is_array(json)` | `-1` | [x] |
| B18 | `json_array_remove` | `index >= array->entries` | `-1` | [x] |
| B19 | `json_array_clear` | `!json_is_array(json)` | `-1` | [x] |
| B20 | `json_array_extend` | `!json_is_array(json) \|\| !json_is_array(other_json)` | `-1` | [x] |
| B21 | `json_array_extend` | `!json_array_grow(array, other->entries)` (OOM) | `-1` | [x] |
| B22 | `json_array_equal` (via `json_equal`) | sizes differ | `0` | [x] |
| B23 | `json_array_equal` (via `json_equal`) | element values differ | `0` | [x] |
| B24 | `json_array_copy` | `json_array()` failed (OOM) | `NULL` | [x] |
| B25 | `json_array_deep_copy` | `jsonp_loop_check` — self-referential array | `NULL` | [x] |
| B26 | `json_array_deep_copy` | `json_array()` failed (OOM) | `NULL` | [x] |
| B27 | `json_array_deep_copy` | `json_array_append_new(result, do_deep_copy(...))` failed | `NULL`, `result` decref'd | [x] |

## C. `value.c` — strings, numbers, misc

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| C1 | `string_create` | `!value` | `NULL` | [x] |
| C2 | `string_create` | `!v` — `jsonp_strndup(value, len)` failed (`!own`) | `NULL` | [x] |
| C3 | `string_create` | `!string` — malloc of `json_string_t` failed | `NULL`, `v` freed | [x] |
| C4 | `json_string_nocheck` | `!value` | `NULL` | [x] |
| C5 | `json_stringn_nocheck` | NULL value (via `string_create`) | `NULL` | [x] |
| C6 | `jsonp_stringn_nocheck_own` | NULL value (via `string_create`) | `NULL` | [x] |
| C7 | `json_string` | `!value` | `NULL` | [x] |
| C8 | `json_stringn` | `!value \|\| !utf8_check_string(value, len)` — invalid UTF-8 | `NULL` | [x] |
| C9 | `json_string_value` | `!json_is_string(json)` (incl. NULL) | `NULL` | [x] |
| C10 | `json_string_length` | `!json_is_string(json)` (incl. NULL) | `0` | [x] |
| C11 | `json_string_set_nocheck` | `!value` | `-1` | [x] |
| C12 | `json_string_setn_nocheck` | `!json_is_string(json) \|\| !value` | `-1` | [x] |
| C13 | `json_string_setn_nocheck` | `!dup` — `jsonp_strndup` failed | `-1` | [x] |
| C14 | `json_string_set` | `!value` | `-1` | [x] |
| C15 | `json_string_setn` | `!value \|\| !utf8_check_string(value, len)` | `-1` | [x] |
| C16 | `json_vsprintf` | `length < 0` — `vsnprintf(NULL,0,fmt,ap)` failed | `NULL` | [x] |
| C17 | `json_vsprintf` | `!buf` — `jsonp_malloc(length+1)` failed | `NULL` | [x] |
| C18 | `json_vsprintf` | `!utf8_check_string(buf, length)` — formatted result not valid UTF-8 | `NULL`, buf freed | [x] |
| C19 | `json_sprintf` | propagates `json_vsprintf` failure | `NULL` | [x] |
| C20 | `json_vsprintf` / `json_sprintf` | empty result (`length == 0`) — *not* an error, yields `""` string | non-NULL empty string | [x] |
| C21 | `json_integer` | `!integer` — malloc failed | `NULL` | [x] |
| C22 | `json_integer_value` | `!json_is_integer(json)` (incl. NULL, real) | `0` | [x] |
| C23 | `json_integer_set` | `!json_is_integer(json)` (incl. NULL) | `-1` | [x] |
| C24 | `json_real` | `isnan(value) \|\| isinf(value)` — NaN, +Inf, -Inf | `NULL` | [x] |
| C25 | `json_real` | `!real` — malloc failed | `NULL` | [x] |
| C26 | `json_real_value` | `!json_is_real(json)` (incl. NULL, integer) | `0.0` | [x] |
| C27 | `json_real_set` | `!json_is_real(json) \|\| isnan(value) \|\| isinf(value)` | `-1` | [x] |
| C28 | `json_number_value` | neither integer nor real (`else` branch, incl. NULL) | `0.0` | [x] |
| C29 | `json_delete` | `!json` | no-op | [x] |
| C30 | `json_delete` | `default:` — TRUE/FALSE/NULL singletons | returns without freeing | [x] |
| C31 | `json_equal` | `!json1 \|\| !json2` | `0` | [x] |
| C32 | `json_equal` | `json_typeof(json1) != json_typeof(json2)` | `0` | [x] |
| C33 | `json_equal` | `default:` unknown type | `0` | [x] |
| C34 | `json_copy` | `!json` | `NULL` | [x] |
| C35 | `json_copy` | `default:` unknown type | `NULL` | [x] |
| C36 | `json_deep_copy` | `hashtable_init(&parents_set)` failed (OOM) | `NULL` | [x] |
| C37 | `do_deep_copy` | `!json` | `NULL` | [x] |
| C38 | `do_deep_copy` | `default:` unknown type | `NULL` | [x] |

## D. `dump.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| D1 | `dump_to_strbuffer` | `strbuffer_append_bytes` failed (OOM / overflow) | `-1` | [x] |
| D2 | `dump_to_buffer` | `buf->used + size > buf->size` — output does not fit; `memcpy` skipped, `used` still advanced | returns `0` (no error); `json_dumpb` returns *required* size > `size` | [x] |
| D3 | `dump_to_file` | `fwrite(buffer, size, 1, dest) != 1` | `-1` | [x] |
| D4 | `dump_to_fd` | `write(*dest, buffer, size) != (ssize_t)size` — closed/invalid/full fd | `-1` | [x] |
| D5 | `dump_indent` | `dump("\n",1,data)` failed | `-1` | [x] |
| D6 | `dump_indent` | `dump(whitespace, cur_n, data)` failed | `-1` | [x] |
| D7 | `dump_indent` | `space && !(flags & JSON_COMPACT)` → `dump(" ",1,data)` failed | callback's nonzero value | [x] |
| D8 | `dump_string` | `dump("\"",1,data)` — opening quote write failed | `-1` | [x] |
| D9 | `dump_string` | `!end` — `utf8_iterate` returned NULL (invalid UTF-8 in string payload, only reachable via `*_nocheck`) | `-1` | [x] |
| D10 | `dump_string` | `dump(str, pos-str, data)` — literal-run write failed | `-1` | [x] |
| D11 | `dump_string` | `dump(text, length, data)` — escape-sequence write failed | `-1` | [x] |
| D12 | `dump_string` | `dump("\"",1,data)` — closing quote failed | `-1` | [x] |
| D13 | `do_dump` | `!json` | `-1` | [x] |
| D14 | `do_dump` | `JSON_INTEGER`: `size < 0 \|\| size >= MAX_INTEGER_STR_LENGTH` (25) | `-1` | [x] |
| D15 | `do_dump` | `JSON_REAL`: `jsonp_dtostr(buffer, 25, value, precision)` returned `< 0` | `-1` | [x] |
| D16 | `do_dump` | `JSON_ARRAY`: `jsonp_loop_check` — circular array reference | `-1` | [x] |
| D17 | `do_dump` | `!embed && dump("[",1,data)` failed | `-1` | [x] |
| D18 | `do_dump` | array: `dump_indent(flags, depth+1, 0, ...)` failed | `-1` | [x] |
| D19 | `do_dump` | array: element `do_dump(...)` failed (non-last) | `-1` | [x] |
| D20 | `do_dump` | array: `dump(",",1,...) \|\| dump_indent(...)` failed | `-1` | [x] |
| D21 | `do_dump` | array: last-element `do_dump(...)` failed | `-1` | [x] |
| D22 | `do_dump` | array: closing `dump_indent(flags, depth, 0, ...)` failed | `-1` | [x] |
| D23 | `do_dump` | `JSON_OBJECT`: `jsonp_loop_check` — circular object reference | `-1` | [x] |
| D24 | `do_dump` | `!embed && dump("{",1,data)` failed | `-1` | [x] |
| D25 | `do_dump` | object: `dump_indent(flags, depth+1, 0, ...)` failed | `-1` | [x] |
| D26 | `do_dump` | `JSON_SORT_KEYS`: `!keys` — `jsonp_malloc(size * sizeof(struct key_len))` failed | `-1` | [x] |
| D27 | `do_dump` | `assert(i == size)` — iterated key count != `json_object_size` | abort | n/a — `assert` the C treats as unreachable |
| D28 | `do_dump` | `assert(value)` — `json_object_getn` NULL for a sorted key | abort | n/a — `assert` the C treats as unreachable |
| D29 | `do_dump` | sorted: `dump(separator,...) \|\| do_dump(value,...)` failed | `-1`, `keys` freed | [x] |
| D30 | `do_dump` | sorted: `dump(",",1,...) \|\| dump_indent(...)` failed | `-1`, `keys` freed | [x] |
| D31 | `do_dump` | sorted: closing `dump_indent(...)` failed | `-1`, `keys` freed | [x] |
| D32 | `do_dump` | unsorted: `dump(separator,...) \|\| do_dump(iter_value,...)` failed | `-1` | [x] |
| D33 | `do_dump` | unsorted: `dump(",",1,...) \|\| dump_indent(...)` failed | `-1` | [x] |
| D34 | `do_dump` | unsorted: closing `dump_indent(...)` failed | `-1` | [x] |
| D35 | `do_dump` | `default:` unknown `json_typeof` | `-1` | [x] |
| D36 | `json_dumps` | `strbuffer_init(&strbuff)` failed (OOM) | `NULL` | [x] |
| D37 | `json_dumps` | `json_dump_callback(...)` failed | `NULL` | [x] |
| D38 | `json_dumpb` | `json_dump_callback(...)` failed | `0` (`size_t`) | [x] |
| D39 | `json_dumpf` | propagates `json_dump_callback` failure | `-1` | [x] |
| D40 | `json_dumpfd` | propagates `json_dump_callback` failure | `-1` | [x] |
| D41 | `json_dump_file` | `!output` — `fopen(path,"w")` failed (unwritable path) | `-1` | [x] |
| D42 | `json_dump_file` | `fclose(output) != 0` | `-1` | [x] |
| D43 | `json_dump_callback` | `!(flags & JSON_ENCODE_ANY)` and root is not array/object (incl. `json == NULL`) | `-1` | [x] |
| D44 | `json_dump_callback` | `hashtable_init(&parents_set)` failed (OOM) | `-1` | [x] |
| D45 | `json_dump_callback` | user callback returns nonzero on the very first chunk | `-1` | [x] |
| D46 | `json_dump_callback` | `callback == NULL` (no check in C — dereferenced) | n/a (UB in C) | n/a |

## E. `error.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| E1 | `jsonp_error_init` | `!error` — NULL error struct | no-op | [x] |
| E2 | `jsonp_error_init` | `!source` — NULL source | `source[0]='\0'`, `line=-1`, `column=-1`, `position=0`, `text[0]='\0'` | [x] |
| E3 | `jsonp_error_set_source` | `!error \|\| !source` | no-op | [x] |
| E4 | `jsonp_error_set_source` | `length >= JSON_ERROR_SOURCE_LENGTH` (80) — source too long | truncated with leading `"..."`, `extra = length - 80 + 4` | [x] |
| E5 | `jsonp_error_vset` | `!error` | no-op | [x] |
| E6 | `jsonp_error_vset` | `error->text[0] != '\0'` — error already set; first error wins | new error silently discarded | [x] |
| E7 | `jsonp_error_vset` | message longer than 158 bytes | truncated at 158, `text[158]='\0'`, `text[159]` = code | [x] |

## F. `memory.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| F1 | `jsonp_malloc` | `!size` — zero-size request | `NULL` | [x] |
| F2 | `jsonp_malloc` | `(*do_malloc)(size)` failed | `NULL` | [x] |
| F3 | `jsonp_free` | `!ptr` | no-op | [x] |
| F4 | `jsonp_realloc` | `newSize == 0` with `do_realloc == NULL` (emulation) — frees `ptr` | `NULL` | [x] |
| F5 | `jsonp_realloc` | `(*do_malloc)(newSize)` failed (emulation) — original `ptr` NOT freed | `NULL` | [x] |
| F6 | `jsonp_realloc` | `(*do_realloc)(ptr, newSize)` failed | `NULL` | [x] |
| F7 | `jsonp_strndup` | `!new_str` — `jsonp_malloc(len+1)` failed | `NULL` | [x] |
| F8 | `json_get_alloc_funcs` | `malloc_fn` and/or `free_fn` NULL | that out-param not written | [x] |
| F9 | `json_get_alloc_funcs2` | any of the three out-params NULL | that out-param not written | [x] |
| F10 | `json_set_alloc_funcs` | (behaviour gate) sets `do_realloc = NULL`, forcing malloc/free realloc emulation | `void` | [x] |
| F11 | `jsonp_realloc` | `ptr == NULL` with `newSize > 0` (emulation) — plain allocation, no `memcpy` | fresh block | [x] |

## G. `strbuffer.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| G1 | `strbuffer_init` | `jsonp_malloc(STRBUFFER_MIN_SIZE=16)` failed | `-1` | [x] |
| G2 | `strbuffer_append_bytes` | `size > STRBUFFER_SIZE_MAX - 1` — overflow guard | `-1` | [x] |
| G3 | `strbuffer_append_bytes` | `strbuff->size > STRBUFFER_SIZE_MAX / STRBUFFER_FACTOR` — overflow guard | `-1` | [x] |
| G4 | `strbuffer_append_bytes` | `strbuff->length > STRBUFFER_SIZE_MAX - 1 - size` — overflow guard | `-1` | [x] |
| G5 | `strbuffer_append_bytes` | `!new_value` — `jsonp_realloc` failed | `-1` | [x] |
| G6 | `strbuffer_append_byte` | propagates `strbuffer_append_bytes(&byte,1)` failure | `-1` | [x] |
| G7 | `strbuffer_pop` | `strbuff->length == 0` — pop from empty buffer | `'\0'` | [x] |
| G8 | `strbuffer_value` | value is NULL after `strbuffer_close`/`strbuffer_steal_value` | `NULL` | [x] |
| G9 | `strbuffer_append_bytes` | `size == 0` — zero-length append | `0`, buffer unchanged | [x] |

## H. `hashtable.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| H1 | `hashtable_find_pair` | `bucket_is_empty(...)` — bucket empty | `NULL` | [x] |
| H2 | `hashtable_find_pair` | scan ends without a `hash`/`key_len`/`memcmp` match | `NULL` | [x] |
| H3 | `hashtable_do_del` | `!pair` — key not found | `-1` | [x] |
| H4 | `hashtable_do_rehash` | `!new_buckets` — `jsonp_malloc(hashsize(order+1) * sizeof(bucket_t))` failed | `-1` | [x] |
| H5 | `hashtable_init` | `!hashtable->buckets` — `jsonp_malloc(hashsize(3) * sizeof(bucket_t))` failed | `-1` | [x] |
| H6 | `init_pair` | `key_len >= (size_t)-1 - offsetof(pair_t, key)` — key length overflow guard | `NULL` | n/a — unreachable without UB: `hashtable_set` calls `hash_str(key, key_len)` *before* `init_pair`, so reaching this guard requires first reading ~`SIZE_MAX` bytes |
| H7 | `init_pair` | `!pair` — `jsonp_malloc(offsetof + key_len + 1)` failed | `NULL` | [x] |
| H8 | `hashtable_set` | `size >= hashsize(order)` triggers rehash, and rehash failed | `-1` | [x] |
| H9 | `hashtable_set` | `!pair` — `init_pair` failed (OOM or oversized key) | `-1` | [x] |
| H10 | `hashtable_get` | key not present | `NULL` | [x] |
| H11 | `hashtable_del` | key not present | `-1` | [x] |
| H12 | `hashtable_iter_at` | key not present | `NULL` | [x] |
| H13 | `hashtable_iter_next` | `list->next == &hashtable->ordered_list` — end of iteration | `NULL` | [x] |
| H14 | `hashtable_iter` | empty table | `NULL` | [x] |
| H15 | `hashtable_set` | zero-length key (`key_len == 0`) — valid, not rejected | `0`, retrievable | [x] |

## I. `version.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| I1 | `jansson_version_cmp` | `2 - major != 0` — major mismatch | that signed difference | [x] |
| I2 | `jansson_version_cmp` | `15 - minor != 0` — minor mismatch | that signed difference | [x] |
| I3 | `jansson_version_cmp` | major+minor equal | `0 - micro` | [x] |
| I4 | `jansson_version_cmp` | extreme args `INT_MIN`/`INT_MAX` (signed overflow in C) | must match C bit-for-bit | [x] |

## J. `utf.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| J1 | `utf8_encode` | `codepoint < 0` | `-1` | [x] |
| J2 | `utf8_encode` | `codepoint > 0x10FFFF` | `-1` | [x] |
| J3 | `utf8_encode` | surrogate `0xD800..0xDFFF` — **accepted** by `utf8_encode` (no check) | `0`, 3 bytes written | [x] |
| J4 | `utf8_check_first` | `0x80 <= u <= 0xBF` — continuation byte as lead | `0` | [x] |
| J5 | `utf8_check_first` | `u == 0xC0 \|\| u == 0xC1` — overlong ASCII lead | `0` | [x] |
| J6 | `utf8_check_first` | `u >= 0xF5` — restricted lead byte | `0` | [x] |
| J7 | `utf8_check_full` | `size` not 2, 3 or 4 | `0` | [x] |
| J8 | `utf8_check_full` | byte `i` not in `0x80..0xBF` — bad continuation | `0` | [x] |
| J9 | `utf8_check_full` | `value > 0x10FFFF` | `0` | [x] |
| J10 | `utf8_check_full` | `0xD800 <= value <= 0xDFFF` — UTF-16 surrogate half | `0` | [x] |
| J11 | `utf8_check_full` | overlong: 2-byte `< 0x80`, 3-byte `< 0x800`, 4-byte `< 0x10000` | `0` | [x] |
| J12 | `utf8_iterate` | `!bufsize` — empty buffer | returns `buffer` unchanged, `*codepoint` untouched | [x] |
| J13 | `utf8_iterate` | `count <= 0` — `utf8_check_first` returned 0 | `NULL` | [x] |
| J14 | `utf8_iterate` | `count > bufsize` — truncated sequence | `NULL` | [x] |
| J15 | `utf8_iterate` | `!utf8_check_full(...)` — invalid continuation | `NULL` | [x] |
| J16 | `utf8_check_string` | `count == 0` — invalid lead byte | `0` | [x] |
| J17 | `utf8_check_string` | `count > length - i` — sequence truncated at end | `0` | [x] |
| J18 | `utf8_check_string` | `!utf8_check_full(...)` | `0` | [x] |
| J19 | `utf8_check_string` | `length == 0` — empty string | `1` (valid) | [x] |

## K. `strconv.c` / `dtoa.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| K1 | `jsonp_strtod` | `assert(end == value + length)` — `strtod` did not consume whole buffer | abort | n/a (unreachable via public API) |
| K2 | `jsonp_strtod` | `(value == ±HUGE_VAL) && errno == ERANGE` — numeric overflow (`"1e999"`) | `-1` | [x] |
| K3 | `jsonp_dtostr` | `dtoa_r(...) == NULL` — `digits[25]` too short | `-1` | n/a (unreachable) |
| K4 | `jsonp_dtostr` | `3 + ndigits + (use_exp ? 5 : 0) > size` — output buffer too short | `-1` | [x] |
| K5 | `jsonp_dtostr` | `size == 0` | `-1` | [x] |
| K6 | `dtoa` | `ndigits`/`mode` out of the documented range | must match C bit-for-bit | [x] |
| K7 | `jsonp_dtostr` | precision 0 → `mode 0` (shortest round-trip); precision 1..31 → `mode 2` | differing digit strings | [x] |

## L. `load.c` — lexer / stream

`error_set` post-processing: with non-NULL `lex`, if `saved_text` is non-empty
and `length <= 20` the text becomes `"<msg> near '<saved_text>'"`; if
`saved_text` is empty then `json_error_invalid_syntax` is **rewritten to**
`json_error_premature_end_of_input` and the text becomes
`"<msg> near end of file"` — unless `stream.state == STREAM_STATE_ERROR`, in
which case the raw message is used with no suffix.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| L1 | `error_set` | `!error` — caller passed NULL `json_error_t*` | no-op; caller still fails | [x] |
| L2 | `error_set` | `saved_text` empty && `code == json_error_invalid_syntax` | code upgraded to `json_error_premature_end_of_input` | [x] |
| L3 | `error_set` | `saved_text.length > 20` | no `" near '...'"` suffix | [x] |
| L4 | `stream_get` | `stream->state != STREAM_STATE_OK` (sticky) | returns `-1` / `-2` without reading | [x] |
| L5 | `stream_get` | `c == EOF` | `state = STREAM_STATE_EOF`, returns `-1`, no error text | [x] |
| L6 | `stream_get` | lead byte `>= 0x80` with `utf8_check_first(c) == 0` (`0x80..0xBF`, `0xC0`, `0xC1`, `>= 0xF5`) | `-2`; `json_error_invalid_utf8`, text `"unable to decode byte 0x%x"`, no suffix | [x] |
| L7 | `stream_get` | `!utf8_check_full(buffer, count, NULL)` — bad continuation / overlong / surrogate / `> U+10FFFF` | `-2`; `json_error_invalid_utf8`, `"unable to decode byte 0x%x"` | [x] |
| L8 | `stream_get` | `assert(count >= 2)` | abort | n/a (unreachable) |
| L9 | `stream_unget` | `assert(buffer_pos > 0)` / `assert(buffer[buffer_pos] == c)` | abort | n/a (unreachable) |
| L10 | `lex_unget_unsave` | `assert(c == d)` | abort | n/a (unreachable) |
| L11 | `decode_unicode_escape` | `assert(str[0] == 'u')` | abort | n/a (unreachable) |
| L12 | `decode_unicode_escape` | a char after `u` is not `[0-9a-fA-F]` | `-1` | [x] |
| L13 | `lex_scan_string` | `c == STREAM_STATE_ERROR` mid-string — invalid UTF-8 inside a string | `TOKEN_INVALID`; `json_error_invalid_utf8` | [x] |
| L14 | `lex_scan_string` | EOF before closing `"` — `"abc` | `json_error_premature_end_of_input`, `"premature end of input near end of file"` | [x] |
| L15 | `lex_scan_string` | raw `\n` inside string literal | `json_error_invalid_syntax`, `"unexpected newline"` (+ near-context) | [x] |
| L16 | `lex_scan_string` | any other raw control char `0x00..0x1F` (e.g. raw TAB, `0x01`) | `json_error_invalid_syntax`, `"control character 0x%x"` | [x] |
| L17 | `lex_scan_string` | `\u` with fewer than 4 hex digits — `"\u12g4"`, `"\u1"` | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| L18 | `lex_scan_string` | `\` + char not in `" \ / b f n r t u` — `"\x"`, `"\ "` | `json_error_invalid_syntax`, `"invalid escape"` | [x] |
| L19 | `lex_scan_string` | `jsonp_malloc(saved_text.length + 1)` failed | `TOKEN_INVALID` with **no error text at all** | [x] |
| L20 | `lex_scan_string` | 2nd pass: `decode_unicode_escape(p) < 0` on first escape | `json_error_invalid_syntax`, `"invalid Unicode escape '%.6s'"` | [x] |
| L21 | `lex_scan_string` | 2nd pass: `decode_unicode_escape` `< 0` on the second (low) surrogate escape | `json_error_invalid_syntax`, `"invalid Unicode escape '%.6s'"` | [x] |
| L22 | `lex_scan_string` | high surrogate `\uD800..\uDBFF` followed by a `\u` that is not `\uDC00..\uDFFF` — `"\uD800A"` | `json_error_invalid_syntax`, `"invalid Unicode '\\u%04X\\u%04X'"` | [x] |
| L23 | `lex_scan_string` | high surrogate not followed by `\u` at all — `"\uD800"`, `"\uD800x"` | `json_error_invalid_syntax`, `"invalid Unicode '\\u%04X'"` | [x] |
| L24 | `lex_scan_string` | lone **low** surrogate `\uDC00..\uDFFF` — `"\uDC00"` | `json_error_invalid_syntax`, `"invalid Unicode '\\u%04X'"` | [x] |
| L25 | `lex_scan_string` | `utf8_encode` non-zero → `assert(0)` | abort | n/a (unreachable) |
| L26 | `lex_scan_string` | `default: assert(0)` in short-escape switch | abort | n/a (unreachable) |
| L27 | `lex_scan_number` | leading zero followed by a digit — `01`, `-012` | `TOKEN_INVALID`, no text here → later `"invalid token"` | [x] |
| L28 | `lex_scan_number` | first significant char neither `0` nor `1-9` — `-`, `-x`, `-.5` | `TOKEN_INVALID` → `"invalid token"` | [x] |
| L29 | `lex_scan_number` | integer, `errno == ERANGE` && `intval < 0` — `-99999999999999999999` | `json_error_numeric_overflow`, `"too big negative integer"` | [x] |
| L30 | `lex_scan_number` | integer, `errno == ERANGE` && `intval >= 0` — `99999999999999999999` | `json_error_numeric_overflow`, `"too big integer"` | [x] |
| L31 | `lex_scan_number` | `assert(end == saved_text + length)` | abort | n/a (unreachable) |
| L32 | `lex_scan_number` | `.` not followed by a digit — `1.`, `1.e5` | `TOKEN_INVALID` → `"invalid token"` | [x] |
| L33 | `lex_scan_number` | `e`/`E` (+ optional sign) not followed by a digit — `1e`, `1e+`, `1ex` | `TOKEN_INVALID` → `"invalid token"` | [x] |
| L34 | `lex_scan_number` | `jsonp_strtod()` failed — real out of double range, `1e999` | `json_error_numeric_overflow`, `"real number overflow"` | [x] |
| L35 | `lex_scan` | `c == STREAM_STATE_ERROR` at token start — invalid UTF-8 outside a string | `TOKEN_INVALID`; `json_error_invalid_utf8` | [x] |
| L36 | `lex_scan` | alpha identifier not exactly `true`/`false`/`null` — `nul`, `True`, `NaN`, `foo` | `TOKEN_INVALID` → `"invalid token"` | [x] |
| L37 | `lex_scan` | any other byte — `#`, `'`, `@`, `(`, `+`, `.`, `)` | `TOKEN_INVALID` → `"invalid token"` | [x] |
| L38 | `lex_init` | `strbuffer_init(&lex->saved_text)` failed (OOM) | `-1`; callers return `NULL` with **no error text** | [x] |

## M. `load.c` — parser / entry points

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| M1 | `parse_object` | `json_object()` NULL (OOM) | `NULL`, no error text | [x] |
| M2 | `parse_object` | token after `{` or `,` is neither a string nor `}` — `{1:2}`, `{,}`, `{` | `json_error_invalid_syntax`, `"string or '}' expected"`; at EOF upgraded to `premature_end_of_input` + `" near end of file"` | [x] |
| M3 | `parse_object` | `lex_steal_string()` NULL | `NULL`, no error text, `object` leaked | [x] |
| M4 | `parse_object` | key contains an embedded NUL — `{"a b": 1}` | `json_error_null_byte_in_key`, `"NUL byte in object key not supported"` | [x] |
| M5 | `parse_object` | `flags & JSON_REJECT_DUPLICATES` and key already present — `{"a":1,"a":2}` | `json_error_duplicate_key`, `"duplicate object key"` | [x] |
| M6 | `parse_object` | token after key is not `:` — `{"a" 1}`, `{"a",1}` | `json_error_invalid_syntax`, `"':' expected"` | [x] |
| M7 | `parse_object` | recursive `parse_value` returned NULL | `NULL`, callee's error kept | [x] |
| M8 | `parse_object` | `json_object_setn_new_nocheck()` nonzero (OOM) | `NULL`, no error text | [x] |
| M9 | `parse_object` | after last member the token is not `}` — `{"a":1`, `{"a":1]` | `json_error_invalid_syntax`, `"'}' expected"` (or `premature_end_of_input` at EOF) | [x] |
| M10 | `parse_array` | `json_array()` NULL (OOM) | `NULL`, no error text | [x] |
| M11 | `parse_array` | element `parse_value` returned NULL | `NULL`, callee's error | [x] |
| M12 | `parse_array` | `json_array_append_new()` nonzero (OOM) | `NULL`, no error text | [x] |
| M13 | `parse_array` | after last element the token is not `]` — `[1,2`, `[1 2]`, `[1,]` | `json_error_invalid_syntax`, `"']' expected"` (upgraded at EOF) | [x] |
| M14 | `parse_value` | `++lex->depth > JSON_PARSER_MAX_DEPTH` (2048) — 2049 nested `[` | `json_error_stack_overflow`, `"maximum parsing depth reached"` | [x] |
| M15 | `parse_value` | string value contains NUL and `!(flags & JSON_ALLOW_NUL)` — `["a b"]` | `json_error_null_character`, `"\\u0000 is not allowed without JSON_ALLOW_NUL"` | [x] |
| M16 | `parse_value` | `case TOKEN_INVALID` | `json_error_invalid_syntax`, `"invalid token"` | [x] |
| M17 | `parse_value` | `default:` — token is `}`, `]`, `,`, `:` or `TOKEN_EOF`; `[,1]`, `[:]`, `}` | `json_error_invalid_syntax`, `"unexpected token"` (at EOF → `premature_end_of_input` + suffix) | [x] |
| M18 | `parse_value` | value constructor returned NULL (OOM) | `NULL`, no extra error text | [x] |
| M19 | `parse_json` | `!(flags & JSON_DECODE_ANY)` and first token is neither `[` nor `{` — `"foo"`, `42`, `true`, `""` | `json_error_invalid_syntax`, `"'[' or '{' expected"`; empty input → `premature_end_of_input`, `"... near end of file"` | [x] |
| M20 | `parse_json` | `parse_value` returned NULL | `NULL` | [x] |
| M21 | `parse_json` | `!(flags & JSON_DISABLE_EOF_CHECK)` and trailing token != `TOKEN_EOF` — `[1] garbage`, `{} {}` | `json_error_end_of_input_expected`, `"end of file expected"`, result decref'd | [x] |
| M22 | `json_loads` | `string == NULL` | `json_error_invalid_argument`, `"wrong arguments"`, source `"<string>"`, line/col `-1`, pos 0 | [x] |
| M23 | `json_loads` | `lex_init` failure (OOM) | `NULL`, no error text | [x] |
| M24 | `json_loadb` | `buffer == NULL` | `json_error_invalid_argument`, `"wrong arguments"`, source `"<buffer>"` | [x] |
| M25 | `json_loadb` | `lex_init` failure (OOM) | `NULL`, no error text | [x] |
| M26 | `json_loadf` | `input == NULL` | `json_error_invalid_argument`, `"wrong arguments"`, source `"<stream>"` (`"<stdin>"` if `input == stdin`) | [x] |
| M27 | `json_loadf` | `lex_init` failure (OOM) | `NULL`, no error text | [x] |
| M28 | `json_loadfd` | `input < 0` — negative fd | `json_error_invalid_argument`, `"wrong arguments"`, source `"<stream>"`/`"<stdin>"` | [x] |
| M29 | `json_loadfd` | `lex_init` failure (OOM) | `NULL`, no error text | [x] |
| M30 | `fd_get_func` | `read()` returned != 1 (EOF or error) | `EOF` — read errors indistinguishable from EOF | [x] |
| M31 | `json_load_file` | `path == NULL` | `json_error_invalid_argument`, `"wrong arguments"` | [x] |
| M32 | `json_load_file` | `fopen(path,"rb")` NULL | `json_error_cannot_open_file`, `"unable to open %s: %s"` (path, `strerror(errno)`) | [x] |
| M33 | `json_load_callback` | `callback == NULL` | `json_error_invalid_argument`, `"wrong arguments"`, source `"<callback>"` | [x] |
| M34 | `json_load_callback` | `lex_init` failure (OOM) | `NULL`, no error text | [x] |
| M35 | `callback_get` | callback returned `0` or `(size_t)-1` | `EOF` — callback error indistinguishable from EOF | [x] |
| M36 | `string_get` | `c == '\0'` in the input string | `EOF` — embedded NUL silently truncates `json_loads` input | [x] |
| M37 | `buffer_get` | `stream->pos >= stream->len` | `EOF` | [x] |
| M38 | `json_loadb` | `buflen == 0` — empty buffer | `premature_end_of_input`, `"'[' or '{' expected near end of file"` | [x] |

## N. `pack_unpack.c` — pack

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| N1 | `read_string` | `va_arg` string NULL and `!optional` (`s`, or object key) | `json_error_null_value`, `"NULL string"` / `"NULL object key"`, source `"<args>"` | [x] |
| N2 | `read_string` | NULL arg with `s?` / `s*` | **no error**; `s?` → `json_null()`, `s*` → item omitted | [x] |
| N3 | `read_string` | `!utf8_check_string(str, strlen(str))` — invalid UTF-8 in the `s` argument | `json_error_invalid_utf8`, `"Invalid UTF-8 <purpose>"`, source `"<args>"` | [x] |
| N4 | `read_string` | `optional` combined with `#`, `%`, or `+` — `"s?#"`, `"s*+"` | `json_error_invalid_format`, `"Cannot use '%c' on optional strings"`, source `"<format>"` | [x] |
| N5 | `read_string` | `strbuffer_init` failed (OOM) | `json_error_out_of_memory`, `"Out of memory"`, source `"<internal>"` | [x] |
| N6 | `read_string` | NULL string arg inside the `#`/`%`/`+` loop — `"s+"` with a NULL piece | `json_error_null_value`, `"NULL <purpose>"`, source `"<args>"` | [x] |
| N7 | `read_string` | `strbuffer_append_bytes` failed (OOM) | `json_error_out_of_memory`, `"Out of memory"`, source `"<internal>"` | [x] |
| N8 | `read_string` | concatenated / length-limited result fails `utf8_check_string` — `"s#"` cutting a UTF-8 sequence | `json_error_invalid_utf8`, `"Invalid UTF-8 <purpose>"`, source `"<args>"` | [x] |
| N9 | `pack_object` | format ends before `}` — `"{"`, `"{s:i"` | `json_error_invalid_format`, `"Unexpected end of format string"`, source `"<format>"` | [x] |
| N10 | `pack_object` | key-position token is not `s` — `"{i:i}"`, `"{[]:i}"` | `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| N11 | `pack_object` | value `pack()` returned NULL and next token is not `*` — `"{s:o}"` with NULL json | `json_error_null_value`, `"NULL object value"`, source `"<args>"` | [x] |
| N12 | `pack_object` | `json_object_setn_new_nocheck()` nonzero | `json_error_out_of_memory`, `"Unable to add key \"%s\""`, source `"<internal>"` | [x] |
| N13 | `pack_array` | format ends before `]` — `"["`, `"[i"` | `json_error_invalid_format`, `"Unexpected end of format string"`, source `"<format>"` | [x] |
| N14 | `pack_array` | element `pack()` returned NULL and next token is not `*` | `NULL`, `has_error = 1`, **no message of its own** (inner error kept) | [x] |
| N15 | `pack_array` | `json_array_append_new()` nonzero | `json_error_out_of_memory`, `"Unable to append to array"`, source `"<internal>"` | [x] |
| N16 | `pack_string` | `read_string` returned NULL | `json_null()` if token was `?` and `!has_error`; else `NULL` | [x] |
| N17 | `pack_object_inter` | `va_arg(json_t*)` NULL for `o`/`O` and next token is neither `?` nor `*` | `json_error_null_value`, `"NULL object"`, source `"<args>"` | [x] |
| N18 | `pack_object_inter` | NULL json with `?` → `json_null()`; with `*` → `NULL` | no error set (silent omission) | [x] |
| N19 | `pack_integer` | `json_integer(value)` NULL (OOM) | `json_error_out_of_memory`, `"Out of memory"`, source `"<internal>"` | [x] |
| N20 | `pack_real` | `json_real(0.0)` NULL (OOM) | `json_error_out_of_memory`, `"Out of memory"`, source `"<internal>"` | [x] |
| N21 | `pack_real` | `json_real_set(json, value)` failed — NaN / ±Inf for `f` | `json_error_numeric_overflow`, `"Invalid floating point value"`, source `"<args>"` | [x] |
| N22 | `pack` | format char not in `{ [ s n b i I f O o` — `"x"`, `"F"`, `"}"`, `"]"`, `"#"`, `"%"`, `"+"`, `"!"`, `"?"`, `"*"` in value position | `json_error_invalid_format`, `"Unexpected format character '%c'"`, source `"<format>"` | [x] |
| N23 | `json_vpack_ex` | `!fmt \|\| !*fmt` — NULL or empty format | `json_error_invalid_argument`, `"NULL or empty format string"`, source `"<format>"`, line/col `-1`, pos 0 | [x] |
| N24 | `json_vpack_ex` | `pack()` returned NULL | `NULL` | [x] |
| N25 | `json_vpack_ex` | extra non-ignored chars after the top-level value — `"[]{}"`, `"ii"` | `json_error_invalid_format`, `"Garbage after format string"`, value decref'd | [x] |
| N26 | `pack_object` | `json_object()` NULL — **not checked** in C | NULL object used downstream | n/a |

## O. `pack_unpack.c` — unpack

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| O1 | `unpack_object` | `hashtable_init(&key_set)` failed (OOM) | `-1`; `json_error_out_of_memory`, `"Out of memory"`, source `"<internal>"` | [x] |
| O2 | `unpack_object` | `root && !json_is_object(root)` for format `{` | `-1`; `json_error_wrong_type`, `"Expected object, got %s"`, source `"<validation>"` | [x] |
| O3 | `unpack_object` | tokens after `!` or `*` inside an object format — `"{s:i!s:i}"` | `-1`; `json_error_invalid_format`, `"Expected '}' after '%c', got '%c'"` | [x] |
| O4 | `unpack_object` | format ends before `}` — `"{"`, `"{s:i"` | `-1`; `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| O5 | `unpack_object` | key-position token not `s` — `"{i:i}"` | `-1`; `json_error_invalid_format`, `"Expected format 's', got '%c'"` | [x] |
| O6 | `unpack_object` | `va_arg` key pointer NULL | `-1`; `json_error_null_value`, `"NULL object key"`, source `"<args>"` | [x] |
| O7 | `unpack_object` | key absent and not marked optional (`s?`) | `-1`; `json_error_item_not_found`, `"Object item not found: %s"`, source `"<validation>"` | [x] |
| O8 | `unpack_object` | recursive `unpack()` failed | `-1`, inner error kept | [x] |
| O9 | `unpack_object` | strict (`!` in format or `JSON_STRICT`) and object has un-unpacked keys | `-1`; `json_error_end_of_input_expected`, `"%li object item(s) left unpacked: %s"` (comma-joined; `"<unknown>"` if strbuffer failed) | [x] |
| O10 | `unpack_array` | `root && !json_is_array(root)` for format `[` | `-1`; `json_error_wrong_type`, `"Expected array, got %s"` | [x] |
| O11 | `unpack_array` | tokens after `!`/`*` inside array format — `"[i!i]"` | `-1`; `json_error_invalid_format`, `"Expected ']' after '%c', got '%c'"` | [x] |
| O12 | `unpack_array` | format ends before `]` — `"["`, `"[i"` | `-1`; `json_error_invalid_format`, `"Unexpected end of format string"` | [x] |
| O13 | `unpack_array` | token not in `unpack_value_starters = "{[siIbfFOon"` — `"[x]"`, `"[#]"`, `"[%]"`, `"[+]"`, `"[?]"` | `-1`; `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| O14 | `unpack_array` | `json_array_get(root, i)` NULL — more format items than array elements | `-1`; `json_error_index_out_of_range`, `"Array index %lu out of range"` | [x] |
| O15 | `unpack_array` | recursive `unpack()` failed | `-1` | [x] |
| O16 | `unpack_array` | strict and `i != json_array_size(root)` — array longer than format | `-1`; `json_error_end_of_input_expected`, `"%li array item(s) left unpacked"` | [x] |
| O17 | `unpack` `'s'` | `root && !json_is_string(root)` | `-1`; `json_error_wrong_type`, `"Expected string, got %s"` | [x] |
| O18 | `unpack` `'s'` | `const char **` target NULL (and not `JSON_VALIDATE_ONLY`) | `-1`; `json_error_null_value`, `"NULL string argument"` | [x] |
| O19 | `unpack` `'s%'` | `size_t *` length target NULL | `-1`; `json_error_null_value`, `"NULL string length argument"` | [x] |
| O20 | `unpack` `'i'` | `root && !json_is_integer(root)` | `-1`; `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| O21 | `unpack` `'I'` | `root && !json_is_integer(root)` | `-1`; `json_error_wrong_type`, `"Expected integer, got %s"` | [x] |
| O22 | `unpack` `'b'` | `root && !json_is_boolean(root)` | `-1`; `json_error_wrong_type`, `"Expected true or false, got %s"` | [x] |
| O23 | `unpack` `'f'` | `root && !json_is_real(root)` — an **integer is rejected** by `f` | `-1`; `json_error_wrong_type`, `"Expected real, got %s"` | [x] |
| O24 | `unpack` `'F'` | `root && !json_is_number(root)` | `-1`; `json_error_wrong_type`, `"Expected real or integer, got %s"` | [x] |
| O25 | `unpack` `'n'` | `root && !json_is_null(root)` | `-1`; `json_error_wrong_type`, `"Expected null, got %s"` | [x] |
| O26 | `unpack` default | any other format char — `"x"`, `"}"`, `"#"`, `"+"` at value position | `-1`; `json_error_invalid_format`, `"Unexpected format character '%c'"` | [x] |
| O27 | `unpack` `i I b f F o O` | target pointer NULL — **no NULL check** (unlike `'s'`) | UB in C | n/a |
| O28 | `json_vunpack_ex` | `root == NULL` | `-1`; `json_error_null_value`, `"NULL root value"`, source `"<root>"`, line/col `-1`, pos 0 | [x] |
| O29 | `json_vunpack_ex` | `!fmt \|\| !*fmt` | `-1`; `json_error_invalid_argument`, `"NULL or empty format string"`, source `"<format>"` | [x] |
| O30 | `json_vunpack_ex` | `unpack()` returned nonzero | `-1`, inner error preserved | [x] |
| O31 | `json_vunpack_ex` | trailing tokens after top-level value — `"[i]x"`, `"ii"` | `-1`; `json_error_invalid_format`, `"Garbage after format string"` | [x] |

## P. `hashtable_seed.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| P1 | `seed_from_urandom` | `open("/dev/urandom", O_RDONLY) == -1` | `1`, `*seed` untouched | n/a (environment) |
| P2 | `seed_from_urandom` | `read(...) != 4` | `1` | n/a (environment) |
| P3 | `generate_seed` | all strong sources failed | silently falls back to `seed_from_timestamp_and_pid` | n/a (environment) |
| P4 | `generate_seed` | `seed == 0` after generation | forced to `1` | n/a (nondeterministic) |
| P5 | `json_object_seed` | `hashtable_seed != 0` — already seeded | silent no-op; caller's seed ignored | [x] |
| P6 | `json_object_seed` | caller passes `seed == 0` | `new_seed = generate_seed()` (nondeterministic) | [x] |
| P7 | `json_object_seed` | lost the seeding race | busy-waits on `sched_yield()`; no failure path | n/a (threading) |

## Q. Generic FFI-boundary boundaries (not in the C source's own checks)

| # | entry point | trigger | expected C result | test |
|---|-------------|---------|-------------------|------|
| Q1 | every `json_*` getter/setter taking `json_t*` | NULL pointer | per-function sentinel above; must never crash | [x] |
| Q2 | `json_loads` / `json_loadb` / `json_dumps` | `flags` with all bits set (`SIZE_MAX`) — every flag on at once, including undefined bits | must match C | [x] |
| Q3 | `json_dumps` | out-of-range indent (`JSON_INDENT(32)` wraps to 0) and precision (`JSON_REAL_PRECISION(32)` wraps to 0) | must match C | [x] |
| Q4 | `json_array_get` / `set_new` / `insert_new` / `remove` | index `SIZE_MAX`, `len`, `len+1` | `NULL` / `-1` | [x] |
| Q5 | `json_dumpb` | `size == 0` with NULL buffer | required size, no write | [x] |
| Q6 | `json_dumpb` | `size` one byte short of the required length | required size, buffer partially/not written | [x] |
| Q7 | `utf8_encode` | out-of-range enum-like ints: `-1`, `0x110000`, `INT_MAX`, `INT_MIN` | `-1` | [x] |
| Q8 | `utf8_check_full` | `size` = 0, 1, 5, `SIZE_MAX` (one step past valid 2..4) | `0` | [x] |
| Q9 | `json_dumpfd` / `json_loadfd` | fd `-1`, and a valid-but-closed fd | `-1` / `NULL` | [x] |
| Q10 | `jsonp_dtostr` | `precision` out of `0..31`: `-1`, `32`, `INT_MAX` | must match C | [x] |
| Q11 | `json_object_seed` | `seed` = `SIZE_MAX` (truncated to `uint32_t` internally) | must match C | [x] |
| Q12 | `json_string_setn*` / `json_stringn*` | `len == 0`; `len` > actual buffer is UB and excluded | valid empty string | [x] |
| Q13 | `dtoa` | `mode` outside 0..9 and negative `ndigits` (C enum accepts any int) | must match C | [x] |
