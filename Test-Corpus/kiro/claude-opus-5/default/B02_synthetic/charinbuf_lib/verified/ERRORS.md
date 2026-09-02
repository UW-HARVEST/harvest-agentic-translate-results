# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return` of a sentinel /
error value, every explicit range check, every null check and every
min/max constant in the file is one row. There are **no** `assert`s, no
`errno` writes, no error enums and no `RETURN_ERROR`-style macros in this
source; the whole rejection surface is null checks, one range check, one
`switch` default, and `NULL` propagation out of `malloc`/`memchr`.

Constants that participate in rejection: `UINT16_MAX` (= `65535`, from
`<stdint.h>`) and the implicit lower bound `0`.

Legend for "expected C result": `ret` = value returned by the function.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|----------------------------------------------|-------------------|------|-----|
| 1  | `is_string_empty` | `str == NULL` (`if (!str) return 1;`, lib.c:56) | `ret == 1` | `err_01_is_string_empty_null` | [x] |
| 2  | `is_string_empty` | `str` non-NULL but `*str == '\0'` (falls past `if (*str)`, lib.c:57-60) | `ret == 1` | `err_02_is_string_empty_empty` | [x] |
| 3  | `find_char_in_buffer` | `buffer == NULL` (`if (!buffer) return NULL;`, lib.c:64) — note the null check happens *before* `size` is looked at, so `size != 0` with a NULL buffer still returns `NULL` rather than faulting | `ret == NULL` | `err_03_find_char_null_buffer` | [x] |
| 4  | `find_char_in_buffer` | `target` absent from the first `size` bytes → `memchr` returns `NULL` (lib.c:65) | `ret == NULL` | `err_04_find_char_absent` | [x] |
| 5  | `find_char_in_buffer` | `size == 0` — `memchr` inspects nothing and returns `NULL` even when byte 0 equals `target` | `ret == NULL` | `err_05_find_char_zero_size` | [x] |
| 6  | `create_buffer` | `initial == NULL` (`if (!initial) return NULL;`, lib.c:69) | `ret == NULL` | `err_06_create_buffer_null` | [x] |
| 7  | `create_buffer` | `malloc(len + 1)` returns `NULL` → `strcpy` is skipped and `NULL` is returned unchanged (lib.c:72-78) | `ret == NULL`, no write performed | `err_07_create_buffer_oom_path` (documented / exercised via the shared no-crash `NULL`-return path; a real OOM is not forced because it would tear down the harness) | [x] |
| 8  | `validate_uint16_range` | `value < 0` (`if (value < 0) return 0;`, lib.c:82) — incl. `-1` and `INT_MIN` | `ret == 0` | `err_08_validate_negative` | [x] |
| 9  | `validate_uint16_range` | `value > UINT16_MAX` i.e. `value >= 65536` (lib.c:83) — incl. `65536` and `INT_MAX` | `ret == 0` | `err_09_validate_above_max` | [x] |
| 10 | `apply_operation` | `op == NULL` (`if (!op) return -1;`, lib.c:88) | `ret == -1`, callback not invoked, `counter` untouched | `err_10_apply_operation_null_op` | [x] |
| 11 | `charinbuf` mode 0 | `validate_uint16_range(value) == 0`, i.e. `value < 0 \|\| value > 65535` (lib.c:110-115) | `ret == -1` **and** stdout contains `Value %d is out of range for uint16_t` | `err_11_charinbuf_mode0_out_of_range` | [x] |
| 12 | `charinbuf` mode 2 | `create_buffer("Testing malloc and free")` returns `NULL` (lib.c:145-147) | `ret == -1` and stdout `Failed to allocate buffer` — unreachable without forcing OOM; recorded and asserted as "never taken under normal allocation" in both libs | `err_12_charinbuf_mode2_alloc_fail_unreached` | [x] |
| 13 | `charinbuf` mode 4 | `find_char_in_buffer(...)` returns `NULL` (lib.c:186-188) | `ret == -1` and stdout `Character 'X' not found` — unreachable because the literal always contains `'X'`; asserted as never taken in both libs | `err_13_charinbuf_mode4_notfound_unreached` | [x] |
| 14 | `charinbuf` mode 4 | `create_buffer(...)` returns `NULL`, so the whole `if (buffer)` body is skipped and `result` keeps its initial `0` (lib.c:170-172, note there is **no** `else` here, unlike mode 2) | `ret == 0`, only the `Mode 4:` header printed — unreachable without forcing OOM; recorded | `err_14_charinbuf_mode4_alloc_fail_unreached` | [x] |
| 15 | `charinbuf` | `mode` matches no `case` → `default:` (lib.c:200-202). Any `int` other than `0..4`, i.e. `-1`, `5`, `INT_MIN`, `INT_MAX`, and every out-of-range "enum-like" value crossing the FFI boundary | `ret == -1` **and** stdout `Invalid mode: %d\n` with the original `mode` | `err_15_charinbuf_invalid_mode`, `err_16_charinbuf_mode_boundaries`, `err_17_charinbuf_mode_fuzz` | [x] |

## Generic FFI boundary cases (required by Phase C even though not table rows)

| # | case | covered by | [x] |
|---|------|-----------|-----|
| G1 | NULL pointer into every pointer-taking export (`is_string_empty`, `find_char_in_buffer`, `create_buffer`, `apply_operation`) | rows 1, 3, 6, 10 | [x] |
| G2 | zero length (`find_char_in_buffer(size = 0)`) | row 5 | [x] |
| G3 | oversized length — `size` larger than the match offset but still inside the allocation, and `size == SIZE_MAX` with an early match (`memchr` stops at the first hit, so it never reads past it) | `err_18_find_char_oversized_size` | [x] |
| G4 | one step past each documented valid range: `validate_uint16_range` at `-1 / 0 / 65535 / 65536`; `charinbuf` mode at `-1 / 0 / 4 / 5` | `err_19_one_past_range` | [x] |
| G5 | out-of-range "enum" values across FFI: `mode` is consumed as a C `int` by a `switch`, so any `int32` bit pattern is a real input, including `INT_MIN`, `INT_MAX` and random garbage | `err_16`, `err_17` | [x] |
| G6 | `target` byte with the high bit set (`char` is signed on x86-64, so `0x80..0xFF` sign-extends to a negative `int` before `memchr` truncates it back to `unsigned char`) | `err_20_find_char_signed_target` | [x] |
| G7 | signed-overflow arithmetic in the counter mutators (`INT_MAX + 1`, `INT_MIN - 1`, `INT_MIN * -1`) — UB in C, compiled to wrapping `add`/`sub`/`imul`; the Rust must wrap identically | `err_21_counter_overflow` | [x] |
