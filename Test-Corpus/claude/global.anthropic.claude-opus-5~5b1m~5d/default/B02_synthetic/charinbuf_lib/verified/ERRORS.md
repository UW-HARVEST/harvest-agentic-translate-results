# ERRORS.md — Phase A error-surface table

Mechanically derived from every rejection / error return / range check in
`c_src/src/lib.c` (there are no `assert`s, no `RETURN_ERROR`-style macros and no
error enums in this library; the rejection vocabulary is `return NULL`,
`return 0`, `return 1`, `result = -1`).

Source scan (`grep -n 'return\|if (!\|<\|>' c_src/src/lib.c`) yields the
branches below. Every row has a differential test in
`tests/phase_c_errors.rs` (or `tests/alloc_failure.rs` for the two
out-of-memory rows), which calls **both** the C `.so` and the Rust `.so` through
`libloading` and asserts the identical error value / sentinel **and** the
identical `stdout` bytes where the function prints.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|---------------------------------------------|-------------------|------|
| E1  | `is_string_empty` | `str == NULL` (`lib.c:55  if (!str) return 1;`) | returns `1` | `e1_is_string_empty_null` |
| E2  | `is_string_empty` | `str` points at `'\0'` (empty string) (`lib.c:59 return 1;`) | returns `1` | `e2_is_string_empty_empty` |
| E3  | `find_char_in_buffer` | `buffer == NULL` (any `size`, incl. huge `size`, any `target`) (`lib.c:63 if (!buffer) return NULL;`) | returns `NULL` | `e3_find_char_null_buffer` |
| E4  | `find_char_in_buffer` | `size == 0` — `memchr` inspects no bytes even when `target` is the first byte | returns `NULL` | `e4_find_char_zero_size` |
| E5  | `find_char_in_buffer` | `target` absent from the first `size` bytes (`memchr` miss), incl. the case where it is present *after* `size` | returns `NULL` | `e5_find_char_absent` |
| E6  | `create_buffer` | `initial == NULL` (`lib.c:68 if (!initial) return NULL;`) | returns `NULL` | `e6_create_buffer_null` |
| E7  | `create_buffer` | `malloc(len+1)` fails (`lib.c:73 if (buffer)` false ⇒ **no** `strcpy`, NULL propagated) | returns `NULL`, no copy, no crash | `alloc_failure.rs::create_buffer_oom` (child process with `RLIMIT_AS`) |
| E8  | `validate_uint16_range` | `value < 0` (`lib.c:81`) — incl. `-1`, `INT_MIN` | returns `0` | `e8_validate_negative` |
| E9  | `validate_uint16_range` | `value > UINT16_MAX (65535)` (`lib.c:82`) — incl. `65536`, `INT_MAX` | returns `0` | `e9_validate_too_large` |
| E10 | `apply_operation` | `op == NULL` (`lib.c:87 if (!op) return -1;`), any `value` | returns `-1`, callee not invoked (static `counter` untouched) | `e10_apply_operation_null` |
| E11 | `charinbuf` | `mode` outside `{0,1,2,3,4}` — the `default:` label (`lib.c:204`). This is the out-of-range "enum" case: `mode` is a plain `int`, so `5`, `-1`, `INT_MIN`, `INT_MAX`, `0x1_0000_0000`-truncated values, … are all real inputs | prints `Invalid mode: %d\n`, returns `-1` | `e11_charinbuf_invalid_mode`, `e11b_charinbuf_invalid_mode_random` |
| E12 | `charinbuf` mode 0 | `value < 0` ⇒ `validate_uint16_range` rejects (`lib.c:111`) | prints `Value %d is out of range for uint16_t\n` + the `UINT16_MAX` line, returns `-1` | `e12_charinbuf_mode0_negative` |
| E13 | `charinbuf` mode 0 | `value > 65535` ⇒ `validate_uint16_range` rejects (`lib.c:111`) | prints `Value %d is out of range for uint16_t\n` + the `UINT16_MAX` line, returns `-1` | `e13_charinbuf_mode0_too_large` |
| E14 | `charinbuf` mode 0 | boundary values one step either side of the valid range: `-1`, `0`, `65535`, `65536` | `-1`, `0`, `65535`, `-1` respectively | `e14_charinbuf_mode0_boundaries` |
| E15 | `charinbuf` mode 2 | `create_buffer("Testing malloc and free")` returns NULL (`lib.c:151`) | prints `Failed to allocate buffer\n`, returns `-1` | `alloc_failure.rs::charinbuf_mode2_oom` (differential under `RLIMIT_AS`) |
| E16 | `charinbuf` mode 4 | `create_buffer(...)` returns NULL (`lib.c:184 if (buffer)` false) — no output after the header line, `result` keeps its initial `0` | returns `0` | `alloc_failure.rs::charinbuf_mode4_oom` (differential under `RLIMIT_AS`) |
| E17 | `charinbuf` mode 4 | `find_char_in_buffer` miss (`lib.c:194`) — unreachable through `charinbuf` because the fixed literal always contains `'X'`; the same C statement is reached directly through the exported `find_char_in_buffer` | `result = -1` / `NULL` | covered by E5 + `e17_mode4_never_reports_miss` |
| E18 | `charinbuf` mode 1 | `is_string_empty("Hello, World!")` non-zero (`lib.c:131` "Non-empty string check failed!") — unreachable through `charinbuf`; the branch condition is reached directly through the exported `is_string_empty` | `result` stays `0`/`1`, no `+= 10` | covered by E1/E2 + `e18_mode1_always_takes_success_branch` |

## Generic FFI boundary cases (also mandatory, tested even though not in the table)

| # | case | test |
|---|------|------|
| G1 | every pointer parameter `= NULL` (`is_string_empty`, `find_char_in_buffer`, `create_buffer`) and NULL function pointer (`apply_operation`) | `g1_all_null_pointers` |
| G2 | zero length (`find_char_in_buffer size = 0`) and empty string (`create_buffer("")` ⇒ 1-byte allocation) | `g2_zero_len_and_empty` |
| G3 | oversized length: `find_char_in_buffer(NULL, SIZE_MAX, c)` and `find_char_in_buffer(buf, SIZE_MAX, c)` where the target *is* inside the buffer, so `memchr` stops before reading out of bounds | `g3_oversized_len` |
| G4 | one step past documented ranges: `validate_uint16_range(-1/0/65535/65536)`, `charinbuf(mode=-1/5)`, `INT_MIN`/`INT_MAX` for `value`, `opt1`, `opt2` | `g4_one_past_range`, `g5_extremes_all_modes` |
| G5 | out-of-range "enum" values across FFI: `mode` = every value in `-8..=12` plus `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `0x8000_0000u32 as i32`, and 4096 random `i32`s | `e11b_charinbuf_invalid_mode_random` |
| G6 | signed/unsigned edge of `char target`: `0x00`, `0x7f`, `0x80`, `0xff` (i.e. negative `char` after the C integer promotion in the `memchr` call) | `g6_char_sign_edges` |
| G7 | signed overflow of the static counter (`increment/decrement/multiply` past `INT_MAX`/`INT_MIN`) — C wraps two's-complement at `-O0`; Rust must wrap identically instead of panicking | `g7_counter_overflow` |
| G8 | embedded NUL bytes in `create_buffer` input (only the prefix is copied) | `g8_embedded_nul` |
| G9 | a function pointer owned by a *third party* (defined in the test binary) handed to `apply_operation` — verifies the raw C ABI of the callback slot, that it is invoked exactly once, and that the argument is passed unchanged | `g9_external_callback_pointer` |
| G10 | each `.so` must keep its **own** `static counter` (no symbol-scope leakage between the two loaded libraries), and `charinbuf` must zero it on entry | `g10_counter_state_is_per_library` |

All rows above are checked off: `cargo test` (and `cargo test --release`) runs
`tests/phase_c_errors.rs` (23 tests) and `tests/alloc_failure.rs` (3 tests) with
zero failures.
