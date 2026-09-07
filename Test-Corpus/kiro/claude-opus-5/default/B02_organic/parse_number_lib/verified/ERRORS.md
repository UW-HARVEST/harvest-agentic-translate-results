# ERRORS.md — error-surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c`. Every `return false`, every
NULL check, every range check and every min/max constant in the file is listed.
There are **no `assert`s** in the C source (`grep -n assert c_src/src/lib.c`
→ no matches) and **no error enums** — the only failure signal is the
`cJSON_bool` return value `false` (`(cJSON_bool)0`).

Grep evidence for the rejection sites:

```
23:    if ((input_buffer == NULL) || (input_buffer->content == NULL))
25:        return false;
64:    if (number_c_string == NULL)
66:        return false; /* allocation failure */
85:    if (number_c_string == after_end)
89:        return false; /* parse_error */
```

`can_access_at_index` (line 8) additionally contains a `buffer != NULL` test
and the range test `offset + index < length`; `buffer_at_offset` (line 10) has
no test at all. Lines 95/99 are the `INT_MAX` / `INT_MIN` saturation clamps.

## Rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `parse_number` | `input_buffer == NULL` (line 23, first disjunct) | returns `false` (0); `*item` left completely untouched; no deref of `input_buffer` | `err_row01_null_input_buffer` | [x] |
| 2 | `parse_number` | `input_buffer != NULL` but `input_buffer->content == NULL` (line 23, second disjunct) | returns `false` (0); `*item` untouched; `input_buffer->offset` untouched | `err_row02_null_content` | [x] |
| 3 | `parse_number` | `malloc(number_string_length + 1) == NULL` (line 64) — allocation failure for the temporary buffer | returns `false` (0); `*item` untouched, `offset` untouched | `err_row03_allocation_failure_documented` (not directly inducible without an allocator interposer; both sides take the same shape — documented, and the size argument `number_string_length + 1` is verified never to overflow) | [x] |
| 4 | `parse_number` | `strtod` consumes zero characters, i.e. `after_end == number_c_string` (line 85). Reached whenever the scanned prefix is not a valid `strtod` prefix. Sub-triggers, each tested: | returns `false` (0); `*item` untouched (`valuedouble` is written *after* this check); `offset` untouched; temporary buffer freed | | |
| 4a | `parse_number` | `number_string_length == 0` because the byte at `offset` is not in `[0-9+\-eE.]` (e.g. `"x1"`, `"null"`, `" 1"`, `"\0"`) → temp string is `""` | `false` | `err_row04a_zero_length_non_numeric_lead` | [x] |
| 4b | `parse_number` | `number_string_length == 0` because `offset >= length` (empty range: `length == 0`, or `offset == length`, or `offset > length`) → temp string is `""` | `false` | `err_row04b_zero_length_empty_range` | [x] |
| 4c | `parse_number` | scanned prefix is non-empty but has no valid `strtod` prefix: `"+"`, `"-"`, `"."`, `"e"`, `"E"`, `"+."`, `"-."`, `"e5"`, `"E5"`, `".e5"`, `"++1"`, `"--1"`, `"-e"`, `"..1"` | `false` | `err_row04c_nonempty_but_unparsable` | [x] |
| 5 | `parse_number` | *(boundary, not an error)* `number >= (double)INT_MAX` — line 95 — incl. `+inf` from `strtod` overflow (`"1e999"`) and exactly `2147483647` | `true`; `valueint == INT_MAX == 2147483647` | `err_row05_saturate_high` | [x] |
| 6 | `parse_number` | *(boundary, not an error)* `number <= (double)INT_MIN` — line 99 — incl. `-inf` (`"-1e999"`) and exactly `-2147483648` | `true`; `valueint == INT_MIN == -2147483648` | `err_row06_saturate_low` | [x] |

## Generic C-API boundaries also covered (beyond the table)

| # | boundary | test | ✔ |
|---|----------|------|---|
| G1 | NULL `input_buffer` (row 1) — note `item` is dereferenced **without** a NULL check by the C at line 92, so passing `item == NULL` is a segfault in *both* implementations and is deliberately **not** exercised. | `err_row01_null_input_buffer` | [x] |
| G2 | zero length: `length == 0` with non-NULL `content` | `err_row04b_zero_length_empty_range` | [x] |
| G3 | oversized / bogus length: `length == SIZE_MAX`, `length` far beyond the real allocation but with the numeric run terminated by a non-numeric byte inside the real allocation (so no OOB read actually happens) | `err_boundary_oversized_length` | [x] |
| G4 | `offset` one step past the valid range (`offset == length`) and far past it (`offset > length`), plus `offset == SIZE_MAX` (exercises the wrapping `offset + index` in `can_access_at_index`) | `err_boundary_offset_past_end` | [x] |
| G5 | one step past the documented numeric range: `"2147483647"` / `"2147483648"` and `"-2147483648"` / `"-2147483649"`; also `INT_MAX`/`INT_MIN` ±1 ULP as doubles | `err_boundary_int_range_edges` | [x] |
| G6 | out-of-range "enum" values across the FFI boundary. `lib.h` declares **no `enum`** — the only enum-like values are the `int` type tag `cJSON_Number` and the `cJSON_bool` typedef. Both are plain `int`, so any `int` is a representable input. Covered by pre-filling `item->type` with values that are *not* `cJSON_Number` (0, `INT_MIN`, `INT_MAX`, `0x7f7f7f7f`, `-1`) and `item->valueint`/`valuedouble` with sentinels, then asserting the C and Rust post-state byte images are identical — including on the failure paths where the tag must be left as the caller's out-of-range value. | `err_row06b_out_of_range_type_tag_preserved` | [x] |
| G7 | `depth` field is never read or written by the C; assert both leave it byte-identical for arbitrary (incl. `SIZE_MAX`) input values | asserted in every differential call via `assert_same` | [x] |
| G8 | non-NUL-terminated `content` (the C explicitly copies exactly `number_string_length` bytes and appends its own `'\0'`, so a buffer with no NUL and trailing garbage must behave identically) | `err_boundary_unterminated_content` | [x] |
