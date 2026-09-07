# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return` that
yields a rejection/sentinel value, every explicit range/NULL check, and every
`min`/`max` constant. There are no `assert`s and no error enums in this
library; rejection is expressed as `NULL`, `-1`, `0` or `1` sentinels.

Grep basis:

```sh
grep -n 'return NULL\|return -1\|return 0\|return 1\|if (!\|if (value\|UINT16_MAX\|default:' c_src/src/lib.c
```

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `is_string_empty` | `str == NULL` (`if (!str) return 1;`, line 55) | returns `1` | [x] `err_is_string_empty_null` |
| 2 | `is_string_empty` | `str` points at `'\0'` (empty string) — falls through `if (*str)` to line 59 | returns `1` | [x] `err_is_string_empty_empty` |
| 3 | `find_char_in_buffer` | `buffer == NULL` (`if (!buffer) return NULL;`, line 63) — even with `size > 0` and any `target` | returns `NULL` | [x] `err_find_char_null_buffer` |
| 4 | `find_char_in_buffer` | non-NULL buffer, `size == 0` → `memchr(p, c, 0)` must not read | returns `NULL` | [x] `err_find_char_zero_size` |
| 5 | `find_char_in_buffer` | `target` not present in the first `size` bytes (incl. `target` present only *past* `size`, and `target == '\0'` with `size` shorter than the NUL) | returns `NULL` | [x] `err_find_char_not_found` |
| 6 | `create_buffer` | `initial == NULL` (`if (!initial) return NULL;`, line 68) | returns `NULL` | [x] `err_create_buffer_null` |
| 7 | `create_buffer` | `malloc` returns `NULL` (`if (buffer)` guard, line 73) — no copy performed, `NULL` propagated | returns `NULL` | [x] `err_create_buffer_alloc_fail_contract` (documented; not force-failable without interposing malloc — the guarded-copy path is asserted structurally) |
| 8 | `validate_uint16_range` | `value < 0` (line 81), i.e. `-1`, `INT_MIN`, any negative | returns `0` | [x] `err_validate_uint16_negative` |
| 9 | `validate_uint16_range` | `value > UINT16_MAX` (65535) (line 82), i.e. `65536`, `INT_MAX` | returns `0` | [x] `err_validate_uint16_too_large` |
| 10 | `validate_uint16_range` | boundary accepted: `value == 0` and `value == 65535` are **valid** (return `1`) — one step past on both sides is rejected | returns `1` | [x] `err_validate_uint16_boundaries` |
| 11 | `apply_operation` | `op == NULL` (`if (!op) return -1;`, line 87), any `value` | returns `-1` | [x] `err_apply_operation_null_fp` |
| 12 | `charinbuf` | `mode` has no `case` — `default:` at line 204, i.e. any `mode` outside `0..=4` (negative, `5`, `INT_MIN`, `INT_MAX`) — an out-of-range "enum-like" int across the FFI boundary | prints `Invalid mode: %d\n`, returns `-1` | [x] `err_charinbuf_invalid_mode` |
| 13 | `charinbuf` mode 0 | `value < 0` or `value > 65535` → `validate_uint16_range` fails, line 111 | prints `Value %d is out of range for uint16_t\n`, returns `-1` | [x] `err_charinbuf_mode0_out_of_range` |
| 14 | `charinbuf` mode 2 | allocation of `"Testing malloc and free"` fails, line 151 | prints `Failed to allocate buffer\n`, returns `-1` | [x] `err_charinbuf_mode2_alloc_fail_contract` (documented; unreachable without malloc interposition) |
| 15 | `charinbuf` mode 4 | `found_pos == NULL` (search char absent), line 194 | prints `Character 'X' not found\n`, returns `-1` | [x] `err_charinbuf_mode4_notfound_contract` (unreachable: the literal always contains `'X'`; asserted that both libs agree it *is* found at the same index) |
| 16 | `charinbuf` mode 4 | `buffer == NULL` (line 184 `if (buffer)` has **no** `else`) → `result` keeps its initial `0` | returns `0` | [x] `err_charinbuf_mode4_null_buffer_contract` (unreachable without malloc interposition) |

## Generic FFI boundary cases also covered

| # | case | test |
|---|------|------|
| G1 | NULL pointer to every pointer-taking export (`is_string_empty`, `find_char_in_buffer`, `create_buffer`) | [x] `err_all_null_pointers` |
| G2 | zero length (`find_char_in_buffer` `size == 0`) | [x] `err_find_char_zero_size` |
| G3 | oversized / absurd length is **not** guarded by the C — excluded on purpose (`memchr` would read out of bounds in both languages; not a defined input) | n/a (documented) |
| G4 | out-of-range enum-like `int` for `mode`: `-2147483648`, `-1`, `5`, `6`, `100`, `2147483647` | [x] `err_charinbuf_invalid_mode` |
| G5 | one step past documented valid range for `validate_uint16_range`: `-1`, `65536` | [x] `err_validate_uint16_too_large`, `err_validate_uint16_negative` |
| G6 | extreme `int` inputs to the counter ops (overflow of `+`, `-`, `*`) | [x] `cfg_counter_extremes` (Phase B) |
| G7 | `target` byte with the high bit set (negative `char`) in `find_char_in_buffer` | [x] `cfg_find_char_high_bit` (Phase B) |
