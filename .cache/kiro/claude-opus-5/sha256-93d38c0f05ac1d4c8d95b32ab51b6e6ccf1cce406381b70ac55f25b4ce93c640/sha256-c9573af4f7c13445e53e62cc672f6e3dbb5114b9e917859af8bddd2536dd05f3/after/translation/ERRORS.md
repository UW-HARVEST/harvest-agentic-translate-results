# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c`. Every `assert`, every `return NULL`,
every `return <sentinel>`, and every explicit range / mask / min-max check in the
`valid_1..valid_4` macros gets its own row. Rows are checked off only when a
differential test constructs that exact condition, calls **both** `.so` files,
and asserts the **same** result (same sentinel / same returned offset / same
error return), not merely "both failed".

Legend for "expected C result":
* `abort` = `__assert_fail` → SIGABRT (the C `.so` is built with asserts LIVE:
  `nm -D` shows `U __assert_fail@GLIBC_2.2.5`, and `CMakeLists.txt` sets no
  build type, hence no `-DNDEBUG`).
* `NULL` = the function returns a null pointer.
* `ptr@k` = `w_utf8_drop` returns `string + k`, i.e. the address of the first
  byte it refused; `w_utf8_filter` correspondingly drops or replaces that byte.

## Group 1 — assertions

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `w_utf8_drop` | `string == NULL` | `assert(string != NULL)` fails → abort (SIGABRT) | [x] |
| 2 | `w_utf8_filter` | `string == NULL` (either value of `replacement`) | `assert(string != NULL)` fails → abort (SIGABRT) | [x] |

## Group 2 — allocation-failure error returns

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 3 | `w_utf8_filter` | input is fully valid UTF-8 (`*valid == '\0'`) and `strdup` fails | returns `NULL` (the unchecked `copy` from `strdup` is propagated) | [x] |
| 4 | `w_utf8_filter` | input contains an invalid byte and `malloc(strlen+1)` fails | `if (copy == NULL) return NULL;` → `NULL` | [x] |
| 5 | `w_utf8_filter` | `replacement != 0`, an invalid byte reached, and `realloc(copy, size + 4096)` fails | `if (copy == NULL) return NULL;` → `NULL` (original buffer leaked — replicated) | [x] |

## Group 3 — `w_utf8_drop` rejection sentinel (the "error" of the scanner)

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 6 | `w_utf8_drop` | first byte is invalid | `ptr@0` (== `string`) | [x] |
| 7 | `w_utf8_drop` | invalid byte after `k` valid bytes | `ptr@k` | [x] |
| 8 | `w_utf8_drop` | no invalid byte at all (incl. the empty string) | pointer to the terminating `'\0'` (`ptr@strlen`) | [x] |

## Group 4 — `valid_1` rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 9 | `valid_1` (both) | `(x[0] & 0x80) != 0` — byte `0x80..0xFF` is not a 1-byte form; falls through to `valid_2` | not accepted as 1 byte | [x] |

## Group 5 — `valid_2` rejections (`110xxxxx 10xxxxxx`)

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 10 | `valid_2` (both) | `(x[0] & 0xE0) != 0xC0` — lead byte outside `0xC0..0xDF` | falls through to `valid_3` | [x] |
| 11 | `valid_2` (both) | `x[0] < (char)0xC2` **as a signed char** — lead byte `0xC0` or `0xC1` (overlong) | rejected → `ptr@k` / dropped / replaced | [x] |
| 12 | `valid_2` (both) | `(x[1] & 0xC0) != 0x80` — 2nd byte not a continuation, **including `x[1] == '\0'`** (truncated tail `"\xC2"`) | rejected → `ptr@k` | [x] |

## Group 6 — `valid_3` rejections (`1110xxxx 10xxxxxx 10xxxxxx`)

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 13 | `valid_3` (both) | `(x[0] & 0xF0) != 0xE0` — lead byte outside `0xE0..0xEF` | falls through to `valid_4` | [x] |
| 14 | `valid_3` (both) | `(x[1] & 0xC0) != 0x80` (incl. `x[1] == '\0'`, truncated `"\xE0"`) | rejected → `ptr@k` | [x] |
| 15 | `valid_3` (both) | `(x[2] & 0xC0) != 0x80` (incl. `x[2] == '\0'`, truncated `"\xE0\xA0"`) | rejected → `ptr@k` | [x] |
| 16 | `valid_3` (both) | `x[0] == 0xE0 && (unsigned char)x[1] < 0xA0` — overlong 3-byte (`x[1]` in `0x80..0x9F`) | rejected → `ptr@k` | [x] |
| 17 | `valid_3` (both) | `x[0] == 0xED && (unsigned char)x[1] >= 0xA0` — UTF-16 surrogate half U+D800..U+DFFF | rejected → `ptr@k` | [x] |
| 18 | `valid_3` (both) | `x[0] == 0xEF && (unsigned char)x[1] > 0xBF` — **unreachable** (`x[1] & 0xC0 == 0x80` already forces `x[1] <= 0xBF`); dead branch kept verbatim in the Rust | never triggers; `0xEF` leads stay accepted | [x] |

## Group 7 — `valid_4` rejections (`11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`)

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 19 | `valid_4` (both) | `(x[0] & 0xF8) != 0xF0` — lead byte outside `0xF0..0xF7`; final `else` → rejection | rejected → `ptr@k` | [x] |
| 20 | `valid_4` (both) | `(unsigned char)x[0] > 0xF4` — lead `0xF5`, `0xF6`, `0xF7` (max-constant check) | rejected → `ptr@k` | [x] |
| 21 | `valid_4` (both) | `(x[1] & 0xC0) != 0x80` (incl. `'\0'`, truncated `"\xF0"`) | rejected → `ptr@k` | [x] |
| 22 | `valid_4` (both) | `(x[2] & 0xC0) != 0x80` (incl. `'\0'`, truncated `"\xF0\x90"`) | rejected → `ptr@k` | [x] |
| 23 | `valid_4` (both) | `(x[3] & 0xC0) != 0x80` (incl. `'\0'`, truncated `"\xF0\x90\x80"`) | rejected → `ptr@k` | [x] |
| 24 | `valid_4` (both) | `x[0] == 0xF0 && (unsigned char)x[1] < 0x90` — overlong 4-byte | rejected → `ptr@k` | [x] |
| 25 | `valid_4` (both) | `x[0] == 0xF4 && (unsigned char)x[1] > 0x8F` — beyond U+10FFFF | rejected → `ptr@k` | [x] |

## Group 8 — generic FFI-boundary boundaries (required even though not in the C source)

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 26 | `w_utf8_filter` | out-of-range `_Bool` byte: `replacement == 2, 3, 0x7F, 0x80, 0xFF` (C `_Bool`/enums accept any int over FFI) | compiled C uses `cmpb $0x0` ⇒ **any non-zero is true**; identical output to `replacement == 1` | [x] |
| 27 | both | zero length: `""` (pointer valid, first byte `'\0'`) | `w_utf8_drop` → `ptr@0`; `w_utf8_filter` → `strdup("")` = `""` | [x] |
| 28 | both | "oversized" input: length ≫ `REPLACEMENT_INC` (4096) with ≥ 1365 invalid bytes so the `repl < 3` realloc path fires repeatedly | no error; output must match byte-for-byte | [x] |
| 29 | `valid_2` | one step either side of the `0xC2` min constant: leads `0xC1` (reject) / `0xC2` (accept) | `ptr@k` vs. accepted | [x] |
| 30 | `valid_4` | one step past the `0xF4` max constant: `0xF4`(accept)/`0xF5`(reject), and `0xF4 0x8F`(accept)/`0xF4 0x90`(reject) | `ptr@k` vs. accepted | [x] |
| 31 | `valid_3` | one step past the `0xA0` boundaries: `0xE0 0x9F`(reject)/`0xE0 0xA0`(accept), `0xED 0x9F`(accept)/`0xED 0xA0`(reject) | `ptr@k` vs. accepted | [x] |
| 32 | `valid_4` | one step past the `0x90` boundary: `0xF0 0x8F`(reject)/`0xF0 0x90`(accept) | `ptr@k` vs. accepted | [x] |
| 33 | `w_utf8_drop` | continuation byte with nothing before it (`0x80..0xBF` alone) | `ptr@0` | [x] |
| 34 | `w_utf8_drop` | every single lead byte `0x01..0xFF` in isolation (exhaustive 1-byte sweep) | must agree for all 255 | [x] |
| 35 | both | exhaustive 2-byte and 3-byte sweeps over all byte pairs / triples (no interior `'\0'`) | must agree for all 65 025 pairs / 16 581 375 triples | [x] |

Rows 3–5 are exercised with a `malloc`/`realloc`/`strdup` interposer
(`LD_PRELOAD`) that fails only allocations of one exact requested size, so the
failure lands precisely on the C/Rust call site under test and nowhere else.
Rows 1–2 are exercised in forked child processes and compared on `WTERMSIG` /
exit status.

## Row → test mapping (all rows checked off against real runs)

| rows | test file | tests |
|------|-----------|-------|
| 1, 2 | `tests/phase_c_abort.rs` | `err01_and_err02_null_pointer_aborts_identically` (+ `child_null_probe`) |
| 3, 4, 5 | `tests/phase_c_alloc_failure.rs` | `err03_strdup_failure_returns_null`, `err04_malloc_failure_returns_null`, `err05_realloc_failure_returns_null`, `err03_04_05_baseline_without_injection` |
| 6 – 33 | `tests/phase_c_errors.rs` | `err06_…` … `err33_bare_continuation_bytes` (28 tests) |
| 34, 35 | `tests/phase_b_exhaustive.rs` | `row03_and_err34_exhaustive_single_byte`, `row04_exhaustive_two_bytes_drop`, `row04_exhaustive_two_bytes_filter`, `row05_exhaustive_three_bytes_drop`, `row05_exhaustive_three_bytes_filter`, `row05b_exhaustive_four_byte_leads` |

Every one of these asserts the *specific* sentinel the C source dictates
(exact returned offset, exact `NULL`, exact terminating signal), not just that
both implementations "failed somehow".

## Findings — two real divergences were found and fixed in the Rust

1. **Row 1/2 (NULL pointer).** The Rust used `debug_assert!`, so the release
   `cdylib` dereferenced NULL and died with **SIGSEGV** while the C `.so` (built
   with asserts live) dies with **SIGABRT**. Changed to `assert!`. Confirmed
   load-bearing: reverting it to `debug_assert!` makes
   `err01_and_err02_null_pointer_aborts_identically` fail.
2. **Row 26 (out-of-range `_Bool`).** The Rust wrapper took `replacement: bool`,
   which is undefined behaviour for byte values other than 0/1 — exactly the
   value class a C caller can legally pass. Changed the exported parameter to
   `u8` with `!= 0`, matching the `cmpb $0x0` the C compiler emits. Confirmed
   load-bearing: changing it to `== 1` makes 5 tests fail.

## Mutation check (does the suite actually detect divergence?)

Deliberately wrong Rust variants were built and the suite re-run:

| injected divergence | detected |
|---------------------|----------|
| `valid_4` max constant `0xF4` → `0xF5` | yes |
| `valid_3` surrogate clause (`0xED`) removed | yes |
| `replacement != 0` → `replacement == 1` | yes (5 tests) |
| `assert!` → `debug_assert!` on the NULL check | yes |
| `if repl < 3` → `if repl < 4` | **no — provably behaviour-preserving**: `repl` only ever reaches 0, 1, 2 or 3 at the check, and moving the `realloc` one replacement earlier changes only the (already over-sized) buffer capacity, never an output byte |
| `(*x as i8) >= (0xC2 as i8)` → `*x >= 0xC2` | **no — provably equivalent**: the preceding `(*x & 0xE0) == 0xC0` already confines `*x` to `0xC0..=0xDF`, where the signed and unsigned comparisons select the identical set `0xC2..=0xDF` |
