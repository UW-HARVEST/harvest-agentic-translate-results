# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-return / bound in
`c_src/src/lib.c` and `c_src/include/lib.h`. There are no `assert`s, no error
enums and no `RETURN_ERROR` macros in this library; every rejection is a bare
`return false` (`(cJSON_bool)0`). Success is `return true` (`(cJSON_bool)1`).

Grep evidence:

```
$ grep -n 'return\|assert\|NULL\|INT_M\|<\|>' c_src/src/lib.c
8:  #define can_access_at_index(buffer, index) ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
23: if ((input_buffer == NULL) || (input_buffer->content == NULL))
25:     return false;
64: if (number_c_string == NULL)
66:     return false; /* allocation failure */
85: if (number_c_string == after_end)
89:     return false; /* parse_error */
95: if (number >= INT_MAX)
99: else if (number <= (double)INT_MIN)
113: return true;
```

## Error surface

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|----------------------------------------------|-------------------|------|----|
| 1 | `parse_number` | `input_buffer == NULL` (lib.c:23) | returns `false` (0); `item` left completely untouched | `err_null_input_buffer` | [x] |
| 2 | `parse_number` | `input_buffer->content == NULL` (lib.c:23), any `length`/`offset` | returns `false` (0); `item` untouched, `offset` unchanged | `err_null_content` | [x] |
| 3 | `parse_number` | `malloc(number_string_length + 1) == NULL` (lib.c:64) | returns `false` (0) — allocation failure; `offset` not advanced. FORCED for real: a 256 MiB virtual run of `'1'` (one 1 MiB `memfd` mapped `MAP_FIXED` 256×, so physical cost stays ~1 MiB) is parsed in a `fork()`ed child with `RLIMIT_AS` capped just above the mapping, so the library's `malloc(256 MiB + 1)` returns NULL. Observed: C and Rust BOTH return false with `offset` untouched. | `err_malloc_failure_returns_false` (+ `alloc_large_but_successful` for the taken-branch mirror) | [x] |
| 4 | `parse_number` | `strtod` consumed nothing → `number_c_string == after_end` (lib.c:85): the scanned prefix is empty because the first byte is not in `[0-9+-.eE]` (e.g. `"abc"`, `"null"`, `" 1"`, `"\0"`, `"{"`) | returns `false` (0); `item` untouched, `input_buffer->offset` unchanged | `err_non_numeric_first_byte`, `err_random_non_numeric` | [x] |
| 5 | `parse_number` | `strtod` consumed nothing although a prefix WAS scanned: prefix is in `[0-9+-.eE]*` but not a valid strtod prefix — `"+"`, `"-"`, `"."`, `"e"`, `"E"`, `"+-"`, `"-."`, `"e5"`, `"E+1"`, `".e1"`, `"+e"`, `"--1"`, `"++"` | returns `false` (0); `item` untouched, `offset` unchanged | `err_scanned_but_unparsable`, `err_random_sign_exp_soup` | [x] |
| 6 | `parse_number` | empty accessible window: `offset >= length` (so `can_access_at_index` is false at `i == 0`) with non-NULL `content` → `number_string_length == 0`, `strtod("")` consumes nothing | returns `false` (0) | `err_empty_window`, `err_offset_ge_length` | [x] |
| 7 | `parse_number` | `length == 0` with non-NULL `content` | returns `false` (0) | `err_zero_length` | [x] |
| 8 | `parse_number` | `offset > length` (offset past the end, still `offset + i < length` false) | returns `false` (0); no read of `content` | `err_offset_past_end` | [x] |
| 9 | `parse_number` | `offset + i` wraps `size_t` — `offset == SIZE_MAX`, `length` small: `can_access_at_index` computes `SIZE_MAX + 0 < length` = false at i=0, so window is empty | returns `false` (0) | `err_offset_size_max` | [x] |
| 10 | `parse_number` | `item == NULL` **on the success path** — the C never null-checks `item` (lib.c:92 `item->valuedouble = number`) → SIGSEGV. Bug preserved in Rust. | SIGSEGV in both — and the Rust MUST NOT "fix" it by adding a null check. Each implementation is run in its own `fork()`ed child and the wait statuses are compared. Observed: both die on signal 11. | `err_null_item_on_success_path_faults_identically` | [x] |
| 11 | `parse_number` | `item == NULL` on an **error** path (rows 1,2,4,5,6,7,8,9) — returns before touching `item` | returns `false` (0), no crash | `err_null_item_on_error_paths` | [x] |
| 12 | `parse_number` | oversized `length` = `SIZE_MAX` with a short real buffer, but the first byte terminates the scan (`default:` → `goto loop_end`) so no OOB read occurs | returns `false` (0) for non-numeric first byte | `err_size_max_length` | [x] |
| 13 | `parse_number` | saturation low bound: `number <= (double)INT_MIN` (lib.c:99) — `"-1e999"` (→ `-inf`), `"-2147483648"`, `"-2147483649"`, `"-1e300"` | returns `true`; `valueint == INT_MIN` (-2147483648), `valuedouble` = parsed value | `bound_int_min`, `cfg15_saturation` | [x] |
| 14 | `parse_number` | saturation high bound: `number >= INT_MAX` (lib.c:95) — `"1e999"` (→ `+inf`), `"2147483647"`, `"2147483648"`, `"1e300"` | returns `true`; `valueint == INT_MAX` (2147483647) | `bound_int_max`, `cfg15_saturation` | [x] |
| 15 | `parse_number` | one step *inside* the saturation range: `"2147483646"`, `"-2147483647"`, `"2147483646.9"` | returns `true`; `valueint` = truncate-toward-zero of the double | `bound_one_inside` | [x] |
| 16 | `parse_number` | subnormal / underflow input `"1e-999"` — `strtod` sets `ERANGE` and returns `0.0` but DOES consume the text | returns `true`; `valuedouble == 0.0`, `valueint == 0` | `bound_underflow` | [x] |
| 17 | `parse_number` | out-of-range "enum" value in `cJSON.type` before the call (C enums/ints accept any value): pre-set `item->type` to `-1`, `0x7FFFFFFF`, `INT_MIN`, `12345` | on success `type` is overwritten with `cJSON_Number` (8) regardless; on failure `type` keeps the garbage value | `err_garbage_type_field` | [x] |
| 18 | `parse_number` | garbage `depth` field (unused by `parse_number`): `0`, `SIZE_MAX` | ignored; `depth` returned unmodified | `cfg_depth_ignored` | [x] |
| 19 | `parse_number` | non-NUL-terminated `content` whose numeric run reaches exactly `length` (no `'\0'` available) — the `for` loop stops on the bound, not on a terminator | returns `true`; parses only the in-window bytes | `cfg_no_terminator` | [x] |
| 20 | `parse_number` | embedded interior NUL inside the window (`"12\0 34"`): `'\0'` hits `default:` → `goto loop_end` after 2 bytes | returns `true`; `offset += 2` | `err_interior_nul` | [x] |
