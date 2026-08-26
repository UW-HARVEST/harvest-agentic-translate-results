# ERRORS.md — Error / rejection surface of `c_src/src/lib.c`

The C library has **no error codes, no `RETURN_ERROR` macro, no `assert`, no
`errno` use, no `return -1`, and no pointer-returning function** (grep for
`assert|return -1|return NULL|errno|ERROR` → no hits). Its entire rejection
surface consists of:

* the explicit guard `if`s that *skip* work (the C analogue of "reject input"),
* the explicit range clamps in `validate_and_normalize`,
* the sentinel substitution `result = 0777` in `findrep`,
* the implicit UB / trap paths reachable from the public ABI (null pointers,
  signed-division overflow, signed-arithmetic overflow, out-of-range `int`
  values narrowed by `memchr`).

Every row below is a distinct rejection/guard branch found in the source, with
the exact triggering input and the observable C result. Every row has a
differential test in `tests/differential.rs` (test name in the last column) and
is checked off only after that test passed against **both** `.so` files.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|----|----------|-------------------------------------------|-------------------|------|---|
| 1  | `divide_multiplier` | `b == 0` (`if (b != 0)` false, lib.c:54) | division skipped, `multiplier` unchanged, `operation_count++`, returns unchanged `multiplier` | `err_divide_by_zero` | [x] |
| 2  | `divide_multiplier` | `b == 1` (no-op division, boundary just past 0) | `multiplier /= 1` → unchanged, `operation_count++` | `err_divide_by_zero` | [x] |
| 3  | `divide_multiplier` | `b < 0` with `|multiplier| < |b|` (truncation toward zero) | `multiplier` becomes `0` or `-0`-truncated quotient (C99 trunc-toward-zero, *not* floor) | `err_divide_negative_truncation` | [x] |
| 4  | `divide_multiplier` | `multiplier == INT_MIN && b == -1` (signed division overflow, UB; x86-64 `idiv` traps) | process dies with **SIGFPE (8)** | `err_divide_int_min_by_minus_one` (subprocess) | [x] |
| 5  | `find_and_replace_char` | `search_char` not present in `str` → `memchr` returns `NULL`, `if (found)` false (lib.c:69) | string left completely unmodified, no write at all | `err_find_char_absent` | [x] |
| 6  | `find_and_replace_char` | `str` is the empty string `""` → `strlen == 0`, `memchr(s, c, 0)` → `NULL` | no write, buffer unchanged (incl. the NUL) | `err_find_char_empty_string` | [x] |
| 7  | `find_and_replace_char` | `search_char == 0` (searching for the terminator; it is *outside* the `strlen` window) | `memchr` returns `NULL` → no write | `err_find_char_zero_needle` | [x] |
| 8  | `find_and_replace_char` | `search_char` outside `unsigned char` range, e.g. `256 + 'e'`, `0x1_0000_0041` | `memchr` converts to `unsigned char`: matches `'e'` / `'A'` (low byte only) | `err_find_char_out_of_uchar_range` | [x] |
| 9  | `find_and_replace_char` | `search_char` negative, e.g. `-1`, `-128`, `INT_MIN` | converted to `unsigned char` (`0xFF`, `0x80`, `0x00`) → matches high-bit byte / never matches | `err_find_char_negative_needle` | [x] |
| 10 | `find_and_replace_char` | `str == NULL` (no null check in C; `strlen(NULL)`) | process dies with **SIGSEGV (11)** | `err_null_find_and_replace_char` (subprocess) | [x] |
| 11 | `process_octal_string` | `dest == NULL` (no null check; `strcpy(NULL, buffer)`) | process dies with **SIGSEGV (11)** | `err_null_process_octal_string` (subprocess) | [x] |
| 12 | `validate_and_normalize` | `value == 0` → `is_nonzero == 0`, outer `if` false (lib.c:81) | returns `0` **unclamped** (no lower-bound clamp) | `err_validate_rejects_nonpositive` | [x] |
| 13 | `validate_and_normalize` | `value < 0` (e.g. `-1`, `-1000`, `INT_MIN`) → `value > 0` false | returns `value` **unclamped** (negatives are *not* raised to `0100`) | `err_validate_rejects_nonpositive` | [x] |
| 14 | `validate_and_normalize` | `0 < value < 0100` (=64), i.e. below lower threshold (lib.c:82) | returns `0100` = `64` | `err_validate_clamps_low` | [x] |
| 15 | `validate_and_normalize` | `value > 0777` (=511), i.e. above upper threshold (lib.c:84) | returns `0777` = `511` | `err_validate_clamps_high` | [x] |
| 16 | `validate_and_normalize` | boundary values `1, 63, 64, 65, 510, 511, 512, INT_MAX` | `1→64, 63→64, 64→64, 65→65, 510→510, 511→511, 512→511, INT_MAX→511` | `err_validate_boundaries` | [x] |
| 17 | `findrep` | `param1..4` all `0` → `active_params == 0`, so `active_params >= mode_add` (=1) false (lib.c:132) | add step skipped entirely | `err_findrep_all_zero_params` | [x] |
| 18 | `findrep` | `active_params < 2` (exactly one non-zero param) → `>= mode_multiply` (=2) false (lib.c:137) | multiply step skipped | `err_findrep_one_active_param` | [x] |
| 19 | `findrep` | `accumulator <= 0150` (=104) → subtract step skipped (lib.c:142) | `subtract_from_accumulator` not called, `operation_count` not bumped by it | `err_findrep_accumulator_guard` | [x] |
| 20 | `findrep` | `accumulator == 0` or `multiplier == 0` → `both_active` false (lib.c:157) | `accumulator + multiplier` **not** added to `result` | `err_findrep_both_active_guard` | [x] |
| 21 | `findrep` | `multiplier <= 0100` (=64) → divide step skipped (lib.c:161) | `divide_multiplier` not called | `err_findrep_multiplier_guard` | [x] |
| 22 | `findrep` | computed `result == 0` → `!result_exists` (lib.c:169) | return value replaced by sentinel `0777` = `511` | `err_findrep_zero_result_sentinel` | [x] |
| 23 | `add_to_accumulator` / `subtract_from_accumulator` | signed overflow: `a`/`b` = `INT_MAX`/`INT_MIN` (UB in C; gcc at `-O0` wraps 2's-complement) | wrapped 32-bit result | `err_signed_overflow_add_sub` | [x] |
| 24 | `multiply_with_multiplier` | signed multiply overflow (`INT_MAX * INT_MAX`, `INT_MIN * -1`, …) | wrapped 32-bit result | `err_signed_overflow_multiply` | [x] |
| 25 | `findrep` | extreme params (`INT_MIN`, `INT_MAX`) that overflow `result` accumulation | wrapped 32-bit result, identical guard decisions | `err_findrep_extreme_params` | [x] |
| 26 | `find_and_replace_char` / `process_octal_string` | non-NUL-terminated buffer / `dest` too small | out-of-bounds read/write — UB with no defined C result; the two implementations write the *same* byte counts (`strlen(msg)+1` ≤ 42 for `process_octal_string`, exactly 1 byte for the replacement), verified by full-buffer comparison with `0xAA` canaries | `cfg_process_octal_string_*`, `cfg_find_and_replace_*` (canary compare) | [x] |
| 27 | `add_to_accumulator` / `multiply_with_multiplier` / `subtract_from_accumulator` / `divide_multiplier` | `operation_count++` at `INT_MAX` (signed overflow, UB; wraps at both `-O0` and `-O2`) — observable because `findrep` folds `operation_count * 010` into its result | wrapped 32-bit value; `findrep` results step through `… -15, -7, 1, 9 …` | `exhaustive_operation_count_wraparound` (`#[ignore]`d: 2^31 calls per library, ~40 s) | [x] |

Notes on rows 4, 10, 11: these are *fatal* in C, so they are exercised by
re-executing the test binary as a child process and comparing the terminating
signal of the C child with the Rust child.

## Divergences found and fixed (Rust side only; `c_src/` untouched)

| ERRORS row | C behaviour | Rust behaviour before the fix | fix in `src/lib.rs` |
|------------|-------------|-------------------------------|---------------------|
| 4 (`INT_MIN / -1`) | dies with **SIGFPE (8)** (`cltd; idivl` raises #DE) | returned `INT_MIN` from `wrapping_div` and kept running (exit 0) | new `c_int_div()` performs the same `cdq; idiv` via `core::arch::asm!` on x86-64 (with a `wrapping_div` fallback for architectures whose divide does not trap). The release build now emits byte-identical `cltd; idiv %esi`. |
| 10, 11 (NULL `char*`) | dies with **SIGSEGV (11)** | dies with **SIGABRT (6)**: the plain `*ptr` dereference tripped rustc's debug null-pointer UB check → non-unwinding panic → `abort()` | all raw byte accesses go through `load_u8`/`store_u8` (`ptr::read_volatile`/`write_volatile`), which fault on the bad address exactly like the C does, in **both** the dev and release profiles |
| 22 (sentinel) | `findrep` returns `0777` when the computed result is `0` | (no Rust bug — the first version of the *test* mis-modelled `operation_count`; the scenario now reaches the branch deterministically and both libraries return `511`) | — |

## Result

All 27 rows pass. Every row was run four times: {Rust dev, Rust release
(`panic = "abort"`)} × {C default cmake build `-O0`, C `CMAKE_BUILD_TYPE=Release`
`-O2` build}, in the single valid feature configuration. Driver:
`./verify_all.sh`; 52 tests per run, `52 passed; 0 failed` every time.
