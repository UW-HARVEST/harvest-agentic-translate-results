# ERRORS.md — error / rejection surface table (Phase A → Phase C)

Derived mechanically from `c_src/src/lib.c` and `c_src/src/main.c`.  The library
has **no** error enum, no `errno`, and no `RETURN_ERROR`-style macro: every
rejection is either

* an early `return` producing a sentinel value (`0`) or leaving the buffer
  untouched, or
* a silent **substitution of a default** for an out-of-range parameter, or
* a **clamp** of an out-of-range internal value, or
* (in `main.c`) a `fprintf(stderr, …)` + `return 1`.

Every such site in the C source is one row below.  `L` = source line in the
respective file.

## `c_src/src/lib.c`

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|-------------------------------------------|-------------------|
| 1  | `process_buffer` (L55) | `buffer == NULL` (any `length`, any `flags`) | returns `0`, no memory touched |
| 2  | `process_buffer` (L55) | `length == 0` (non-NULL buffer) | returns `0`, buffer untouched |
| 3  | `process_buffer` (L55) | `buffer == NULL && length == 0` | returns `0` |
| 4  | `process_buffer` (L61-62) | `flags & 0x01` and `param1 % (int)length == 0` (e.g. `param1 == 0`, `param1 == ±length`, `param1 == ±k*length`) | `rotate_buffer` **not called**, buffer unchanged, returns `length` |
| 5  | `process_buffer` (L61) | `flags & 0x01` and `length == 1` ⇒ `param1 % 1 == 0` | rotate skipped, returns `1` |
| 6  | `process_buffer` (L68) | `flags & 0x02` and `param1 <= 0` (incl. `INT_MIN`) | `threshold` silently defaults to `3` |
| 7  | `process_buffer` (L68) | `flags & 0x02` and `param1 > 255` (incl. `256`, `INT_MAX`) | `threshold` silently defaults to `3` |
| 8  | `process_buffer` (L73) | `flags & 0x04` and `param2 == 0` | `preserve_order = 0` → unordered de-dup path |
| 9  | `process_buffer` (L78) | `flags & 0x08` and `new_len < 2` (i.e. `new_len` is `0` or `1`) | `interleave_halves` **not called** |
| 10 | `process_buffer` (L82) | `flags & 0x10` and `new_len < 4` | `reverse_segments` **not called** |
| 11 | `process_buffer` (L83) | `flags & 0x10` and `param1 <= 0` | `seg_size` silently defaults to `4` |
| 12 | `process_buffer` (L84) | `flags & 0x10`, `new_len >= 4` and `seg_size > new_len` | `reverse_segments` **not called**, buffer unchanged |
| 13 | `process_buffer` (L60-87) | `flags` bits `0x20 … 0x8000_0000` set (no valid meaning — the "out-of-range variant" of the flag word), e.g. `flags == 0xFFFF_FFE0` | all unknown bits ignored; behaves exactly like the masked value `flags & 0x1F` |
| 14 | `rotate_buffer` (L97) | `len <= 1` | early `return`, buffer unchanged |
| 15 | `rotate_buffer` (L100-102) | `offset % (int)len == 0` after normalisation | early `return`, buffer unchanged |
| 16 | `rotate_buffer` (L101) | `offset < 0` after `%` (i.e. `param1 < 0`) | `offset += len` — rejected/normalised into `[1, len)`, rotation is *right* by `len+offset` |
| 17 | `compact_runs` (L144) | a run longer than `255` bytes (needs `len > 255`) | `run_len` **clamped to 255**; the tail of the run is re-scanned as a fresh run |
| 18 | `compact_runs` (L142) | `run_len < threshold` | run is *not* compacted, copied verbatim |
| 19 | `compact_runs` (L150) | `read + run_len == len` (final run) | tail `memmove` skipped |
| 20 | `remove_duplicates` (L173) | `len <= 1` | returns `len` unchanged, buffer untouched |
| 21 | `interleave_halves` (L217) | `len < 2` | early `return` (unreachable via `process_buffer`, which already guards — row 9) |
| 22 | `reverse_segments` (L253) | `seg_size <= 1` (reachable with `param1 == 1`) | early `return`, buffer unchanged |
| 23 | `reverse_segments` (L253) | `len < seg_size` | early `return` (unreachable via `process_buffer` — row 12) |
| 24 | `reverse_segments` (L275) | `remainder <= 1` (`len % seg_size` is `0` or `1`) | trailing partial segment left un-reversed |

### Undefined behaviour in the C source (documented, deliberately *not* tested)

These are not "rejections" — the C code simply has no defence, so *any* Rust
behaviour is admissible.  They are listed so the gap is explicit rather than a
blind spot.

| # | function | trigger | C behaviour |
|---|----------|---------|-------------|
| U1 | `process_buffer` (L61) | `length != 0` but `(int)length == 0` (e.g. `length == 2^32`) | `param1 % 0` → `SIGFPE` |
| U2 | `process_buffer` (L61) | `param1 == INT_MIN` and `(int)length == -1` (e.g. `length == 2^32-1`) | `INT_MIN % -1` → `SIGFPE` on x86-64 |
| U3 | `rotate_buffer` (L120) | large-offset branch with `len - offset > 256` (needs `len > 512`) | writes past `uint8_t temp[256]` → stack smash |
| U4 | `compact_runs` (L152) | `threshold == 1` (`param1 == 1`) with a caller buffer of exactly `length` bytes | logical length grows up to `2*length`, writing past the caller's allocation (this is what `main.c`'s `uint8_t buffer[256]` does) |

The Rust translation is given the same `2 * length` write window through the FFI
wrapper (`src/ffi.rs::view_len`), so U4 is *reproduced* rather than diverged
from; the differential harness always allocates that window plus a guard so the
comparison itself stays well defined.

## `c_src/src/main.c` (CLI surface)

| #  | site | trigger | expected C result |
|----|------|---------|-------------------|
| 25 | L40 `scanf("%u", &flags)` | EOF / no digits at all (empty input, `"abc"`, `"-"`, `"+"`) | `stderr: "Error reading flags\n"`, exit status `1`, no stdout |
| 26 | L46 `scanf("%d", &param1)` | EOF or non-numeric token in position 2 | `stderr: "Error reading param1\n"`, exit `1` |
| 27 | L52 `scanf("%d", &param2)` | EOF or non-numeric token in position 3 | `stderr: "Error reading param2\n"`, exit `1` |
| 28 | L58 `scanf("%zu", &length)` | EOF or non-numeric token in position 4 | `stderr: "Error reading length\n"`, exit `1` |
| 29 | L63 `length > 256` | `length` in `257 … ULONG_MAX` (incl. `"-1"` → `ULONG_MAX` via `strtoul`) | `stderr: "Error: length <n> exceeds maximum 256\n"`, exit `1` |
| 30 | L71 `scanf("%u", &byte)` | fewer than `length` numeric tokens follow | `stderr: "Error reading byte <i>\n"` for the first missing index, exit `1` |
| 31 | L40/46/52/71 | value out of range for the destination type (`"4294967296"`, `"99999999999999999999999"`, `"-5"`) | **accepted**: `strtoul`/`strtol` saturate at `ULONG_MAX`/`LONG_MAX`/`LONG_MIN`, negatives wrap, then the result is truncated to `unsigned`/`int`/`uint8_t` |
| 32 | L63 | `length == 256` (boundary, one below the rejection) | accepted |
| 33 | L69-76 | `length == 0` | no bytes read, `process_buffer` returns `0`, stdout is `"0\n"` |

## Coverage check-list (Phase C gate)

Every row has a differential test that constructs the exact condition, calls
**both** shared objects (or both executables) and asserts the *same* sentinel /
message / exit status - never merely "both failed".

| row | test | [x] |
|-----|------|-----|
| 1, 3 | `error_paths::row01_null_buffer_returns_zero` | [x] |
| 2 | `error_paths::row02_zero_length_returns_zero_and_leaves_buffer` | [x] |
| 4 | `error_paths::row04_rotate_skipped_when_offset_folds_to_zero` | [x] |
| 5 | `error_paths::row05_length_one_rotate_never_runs` | [x] |
| 6, 7 | `error_paths::row06_row07_threshold_out_of_range_defaults_to_three` | [x] |
| 8 | `error_paths::row08_param2_zero_selects_unordered_path` | [x] |
| 9, 21 | `error_paths::row09_row21_interleave_skipped_below_two` | [x] |
| 10 | `error_paths::row10_reverse_skipped_below_four` | [x] |
| 11 | `error_paths::row11_seg_size_defaults_to_four` | [x] |
| 12, 23 | `error_paths::row12_row23_seg_size_above_length_rejected` | [x] |
| 13 | `error_paths::row13_unknown_flag_bits_are_ignored`, `valid_paths::x9b_unknown_bits_equal_masked_value` | [x] |
| 14 | `error_paths::row14_rotate_len_le_one_guard` | [x] |
| 15 | `error_paths::row04_...` (offset folds to 0 inside `rotate_buffer` too) | [x] |
| 16 | `error_paths::row16_negative_offset_is_normalised` | [x] |
| 17 | `error_paths::row17_run_length_clamped_to_255` | [x] |
| 18 | `error_paths::row18_short_runs_kept_verbatim` | [x] |
| 19 | `error_paths::row19_final_run_no_tail_move` | [x] |
| 20 | `error_paths::row20_dedup_len_le_one` | [x] |
| 22 | `error_paths::row22_seg_size_one_rejected` | [x] |
| 24 | `error_paths::row24_remainder_le_one_not_reversed`, `row24b_remainder_above_one_is_reversed` | [x] |
| 25 | `driver_cli::row25_flags_unreadable` | [x] |
| 26 | `driver_cli::row26_param1_unreadable` | [x] |
| 27 | `driver_cli::row27_param2_unreadable` | [x] |
| 28 | `driver_cli::row28_length_unreadable` | [x] |
| 29 | `driver_cli::row29_length_above_maximum` | [x] |
| 30 | `driver_cli::row30_missing_data_bytes` | [x] |
| 31 | `driver_cli::row31_out_of_range_values_accepted` | [x] |
| 32 | `driver_cli::row32_length_exactly_256` | [x] |
| 33 | `driver_cli::row33_length_zero` | [x] |

### Generic C-ABI boundaries (not tied to a single row)

| condition | test | [x] |
|-----------|------|-----|
| NULL pointer × 11 flag values × 12 `param1` × 12 `param2` × 16 lengths | `error_paths::row01_null_buffer_returns_zero` | [x] |
| NULL pointer with oversized lengths (`2^16 … usize::MAX`, incl. `2^32`, `2^32+1`, `u32::MAX`) | `error_paths::row01b_null_buffer_oversized_lengths` | [x] |
| zero length with every flag/param combination | `error_paths::row02_…`, `generic_all_32_flag_values_at_every_guard_boundary` | [x] |
| one step past every documented range (`param1` `-1/0/1/254/255/256/257`, `len±1`, `INT_MIN±1`, `INT_MAX∓1`) | `error_paths::generic_one_step_past_every_documented_range` | [x] |
| out-of-range "enum" values across FFI: all 2^32 flag bits sampled, `flags = 0x20`, `0x8000_0000`, `0xFFFF_FFE0`, `0xFFFF_FFFF` | `error_paths::row13_…`, `generic_extreme_param_values_full_flag_range`, `valid_paths::x9_…`, `x9b_…` | [x] |
| `int` extremes for both mode parameters | `error_paths::generic_extreme_param_values_full_flag_range` | [x] |
| non-UTF-8 / embedded NUL bytes on stdin, truncated input, no trailing newline | `driver_cli::non_utf8_and_nul_bytes_on_stdin`, `no_trailing_newline_and_partial_tokens` | [x] |
