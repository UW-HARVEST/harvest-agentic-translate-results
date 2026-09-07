# Phase A.2 — Error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return NULL`, every
`return -1`, every `return 0` that is a rejection, every null check, every
range check, and the `switch` default are listed. There are no `assert`s, no
error enums, and no `RETURN_ERROR`-style macros in this library; rejection is
expressed purely as sentinel return values (`NULL`, `-1`, `0`).

Constants that participate: `UINT16_MAX` (65535, `<stdint.h>`) — the only
min/max constant referenced by the source.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `is_string_empty` | `str == NULL` (`if (!str)`, lib.c:55) | returns `1` |
| 2  | `is_string_empty` | `str` non-NULL but `*str == '\0'` (falls past `if (*str)`, lib.c:56) | returns `1` (indistinguishable from the NULL case) |
| 3  | `find_char_in_buffer` | `buffer == NULL` (`if (!buffer)`, lib.c:63) — checked *before* `size`, so NULL+size>0 still returns cleanly | returns `NULL` |
| 4  | `find_char_in_buffer` | `size == 0` (memchr scans nothing; buffer may even be non-NULL/unreadable) | returns `NULL` |
| 5  | `find_char_in_buffer` | `target` absent from the first `size` bytes | returns `NULL` |
| 6  | `create_buffer` | `initial == NULL` (`if (!initial)`, lib.c:68) | returns `NULL` |
| 7  | `create_buffer` | `malloc(len+1)` returns `NULL` (`if (buffer)` guard, lib.c:73) — no `strcpy`, NULL propagated | returns `NULL` (not practically inducible; covered by inspection) |
| 8  | `validate_uint16_range` | `value < 0` (lib.c:81), incl. `INT_MIN` | returns `0` |
| 9  | `validate_uint16_range` | `value > UINT16_MAX` i.e. `>= 65536` (lib.c:82), incl. `INT_MAX` | returns `0` |
| 10 | `apply_operation` | `op == NULL` (`if (!op)`, lib.c:87) | returns `-1` |
| 11 | `apply_operation` | `op` valid but itself returns `-1` — sentinel collision, must NOT be turned into an error | returns `-1` from the callee |
| 12 | `charinbuf` mode 0 | `value` rejected by `validate_uint16_range` (`value < 0` or `> 65535`) | prints `Value %d is out of range for uint16_t\n`, then `UINT16_MAX constant value: 65535\n`; returns `-1` |
| 13 | `charinbuf` mode 1 | `is_string_empty(non_empty_string)` returning non-zero (the "check failed" branch, lib.c:131) | prints `Non-empty string check failed!\n` and does **not** add 10; unreachable for the hard-coded `"Hello, World!"` — verified by inspection |
| 14 | `charinbuf` mode 2 | `create_buffer("Testing malloc and free")` returns `NULL` (lib.c:151) | prints `Failed to allocate buffer\n`; returns `-1` (not practically inducible) |
| 15 | `charinbuf` mode 4 | `find_char_in_buffer` fails to locate `'X'` (lib.c:194) | prints `Character 'X' not found\n`; returns `-1`; unreachable for the hard-coded haystack — verified by inspection |
| 16 | `charinbuf` mode 4 | `create_buffer(...)` returns `NULL`, so the whole `if` body is skipped and *nothing* is printed after the banner | returns `0` (the initial `result`, **not** `-1`) — not practically inducible |
| 17 | `charinbuf` | `mode` outside `0..=4` — `switch` `default:` (lib.c:204). Includes `-1`, `5`, `INT_MIN`, `INT_MAX`, and any out-of-range "enum" int | prints `Invalid mode: %d\n`; returns `-1` |

## Generic FFI boundary cases also required by Phase C

| # | case | expected |
|---|------|----------|
| G1 | `is_string_empty(NULL)` | `1` (row 1) |
| G2 | `find_char_in_buffer(NULL, 0, c)` and `(NULL, huge, c)` | `NULL` (row 3) |
| G3 | `create_buffer(NULL)` | `NULL` (row 6) |
| G4 | `find_char_in_buffer(buf, size)` with `size` one past the match position | `NULL`; with `size` exactly match+1 | pointer to match |
| G5 | `validate_uint16_range` at `-1 / 0 / 1 / 65534 / 65535 / 65536 / INT_MIN / INT_MAX` | `0/1/1/1/1/0/0/0` |
| G6 | `charinbuf` `mode` one step past the valid range (`-1` and `5`) and at `INT_MIN`/`INT_MAX` | `default` branch, `-1` (row 17) |
| G7 | `find_char_in_buffer` with `target` = `'\0'` (found at the terminator only if inside `size`) and with a negative `char` (`0x80..0xFF` sign-extended) | same match position as C |
| G8 | `apply_operation` with a caller-supplied `extern "C"` callback (function pointer crossing the FFI boundary in the *other* direction) | callee's value verbatim |

## Phase C row -> test mapping (all passing)

Every row above is checked off by a named test in `tests/phase_c_errors.rs`,
which drives both `.so` files through `libloading` and asserts the identical
sentinel:

| row | test | [x] |
|-----|------|-----|
| 1 | `err01_is_string_empty_null` | [x] |
| 2 | `err02_is_string_empty_empty_matches_null_sentinel` | [x] |
| 3 | `err03_find_char_null_buffer` (size 0, 1, 64, `usize::MAX/2`, `usize::MAX`) | [x] |
| 4 | `err04_find_char_zero_size` | [x] |
| 5 | `err05_find_char_absent_target` (2 000 randomized absent targets) | [x] |
| 6 | `err06_create_buffer_null` | [x] |
| 7 | `err07_create_buffer_malloc_failure_is_structurally_equivalent` | [x] |
| 8 | `err08_validate_uint16_negative` (2 006 values) | [x] |
| 9 | `err09_validate_uint16_above_max` (2 005 values + accepted boundary) | [x] |
| 10 | `err10_apply_operation_null_op` (2 005 values) | [x] |
| 11 | `err11_apply_operation_sentinel_collision` | [x] |
| 12 | `err12_charinbuf_mode0_rejects` (306 values, return + stdout) | [x] |
| 13 | `err13_charinbuf_mode1_failed_branch_is_unreachable_in_both` | [x] |
| 14 | `err14_charinbuf_mode2_alloc_failure_branch` | [x] |
| 15 | `err15_charinbuf_mode4_not_found_branch` | [x] |
| 16 | `err16_charinbuf_mode4_alloc_failure_returns_zero` | [x] |
| 17 | `err17_charinbuf_invalid_mode` (613 modes, exact stdout `Invalid mode: %d\n`) | [x] |
| G1 | `err01_is_string_empty_null` | [x] |
| G2 | `err03_find_char_null_buffer` | [x] |
| G3 | `err06_create_buffer_null` | [x] |
| G4 | `gen_g4_find_char_size_one_step_around_match` | [x] |
| G5 | `gen_g5_validate_uint16_boundary_sweep` | [x] |
| G6 | `gen_g6_charinbuf_mode_one_past_valid_range` + `err17` | [x] |
| G7 | `gen_g7_find_char_nul_and_negative_targets` (all 256 targets x sizes 0..=6) | [x] |
| G8 | `gen_g8_apply_operation_inbound_callback` | [x] |
| — | `gen_counter_entry_points_reject_nothing` (the four counter entry points have no rejection path; agreement at the `int` extremes) | [x] |

Rows 7, 13, 14, 15 and 16 describe branches that cannot be reached from outside
the library (a `malloc` failure, or a hard-coded string literal that makes the
condition constant). For those the differential test asserts that both
implementations take the *same* reachable branch, and additionally pins the
structure of the unreachable branch in the Rust source against the C, so a
future edit cannot change its sentinel unnoticed. They are explicitly *not*
claimed as exercised at runtime.

## Out-of-range enum values

`charinbuf`'s `mode` is the only enum-like parameter, and it is declared `int`,
so every one of the 2^32 values is a legal input. `err17_charinbuf_invalid_mode`
covers 613 of them — `-1`, `5`..`8`, `1000`, `65536`, `INT_MIN`, `INT_MIN+1`,
`INT_MAX`, `INT_MAX-1` and 600 randomized values outside `0..=4` — asserting the
exact `default`-arm stdout and the `-1` return for each.
