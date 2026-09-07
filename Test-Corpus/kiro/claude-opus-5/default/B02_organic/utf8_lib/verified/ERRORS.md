# ERRORS.md — Error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c`. Every `assert`, every `return`
that reports failure, and every conjunct inside the `valid_1` … `valid_4`
macros that can make a byte be *rejected* is listed.

Raw inventory of rejection/error statements in the C:

* `w_utf8_drop`: `assert(string != NULL);`
* `w_utf8_drop`: `return string;` inside the loop — the "this byte is not
  valid UTF-8" rejection (the only rejection signal this function has).
* `w_utf8_filter`: `assert(string != NULL);`
* `w_utf8_filter`: `if (copy == NULL) { return NULL; }` after `malloc`
* `w_utf8_filter`: `if (copy == NULL) { return NULL; }` after `realloc`
* `valid_1` — 1 failing conjunct; `valid_2` — 3; `valid_3` — 6; `valid_4` — 7.
  A byte is rejected only when **all four** macros fail, so the rows below
  enumerate the distinct byte patterns that make that happen.

There is no error enum and no `RETURN_ERROR` macro in this library; the two
failure channels are `NULL` (allocation) and `abort()` via `assert`. A
rejected *byte* is signalled by `w_utf8_drop` returning a pointer to it, and
by `w_utf8_filter` either dropping it or emitting `EF BF BD`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `w_utf8_drop` | `string == NULL` | `assert` fails → `__assert_fail` → `SIGABRT` | [x] |
| 2  | `w_utf8_filter` | `string == NULL` | `assert` fails → `__assert_fail` → `SIGABRT` | [x] |
| 3  | `w_utf8_drop` | lone continuation byte `0x80`–`0xBF` at pos p (fails `valid_1` mask, `valid_2`/`3`/`4` lead masks) | returns `string + p` | [x] |
| 4  | `w_utf8_drop` | lead `0xC0` or `0xC1` (`(x[0]&0xE0)==0xC0` but `x[0] < (char)0xC2` signed) | returns pointer to that byte | [x] |
| 5  | `w_utf8_drop` | lead `0xC2`–`0xDF` followed by a byte with `(x[1]&0xC0) != 0x80` (incl. ASCII, another lead) | returns pointer to the lead byte | [x] |
| 6  | `w_utf8_drop` | lead `0xC2`–`0xDF` as the **last** byte before `NUL` (truncated 2-byte, `x[1]==0`) | returns pointer to the lead byte | [x] |
| 7  | `w_utf8_drop` | `0xE0` followed by `0x80`–`0x9F` (overlong 3-byte: `x[0]==0xE0 && x[1] < 0xA0`) | returns pointer to `0xE0` | [x] |
| 8  | `w_utf8_drop` | `0xED` followed by `0xA0`–`0xBF` (UTF-16 surrogate half: `x[0]==0xED && x[1] >= 0xA0`) | returns pointer to `0xED` | [x] |
| 9  | `w_utf8_drop` | lead `0xE0`–`0xEF` with `(x[1]&0xC0) != 0x80` (bad/absent 2nd byte, incl. `NUL` truncation) | returns pointer to the lead byte | [x] |
| 10 | `w_utf8_drop` | lead `0xE0`–`0xEF`, good 2nd byte, `(x[2]&0xC0) != 0x80` (bad/absent 3rd byte, incl. `NUL` truncation) | returns pointer to the lead byte | [x] |
| 11 | `w_utf8_drop` | `0xEF` followed by a byte `> 0xBF` (`x[0]==0xEF` guard) — unreachable in practice because row 9's `(x[1]&0xC0)==0x80` already bounds `x[1] <= 0xBF`; asserted to be a no-op difference | returns pointer to `0xEF` only when row 9/10 also fail | [x] |
| 12 | `w_utf8_drop` | `0xF0` followed by `0x80`–`0x8F` (overlong 4-byte: `x[0]==0xF0 && x[1] < 0x90`) | returns pointer to `0xF0` | [x] |
| 13 | `w_utf8_drop` | `0xF4` followed by `0x90`–`0xBF` (beyond U+10FFFF: `x[0]==0xF4 && x[1] > 0x8F`) | returns pointer to `0xF4` | [x] |
| 14 | `w_utf8_drop` | lead `0xF5`, `0xF6`, `0xF7` (`(x[0]&0xF8)==0xF0` passes but `(unsigned char)x[0] > 0xF4`) | returns pointer to that byte | [x] |
| 15 | `w_utf8_drop` | lead `0xF8`–`0xFF` (fails every lead mask incl. `(x[0]&0xF8)==0xF0`) | returns pointer to that byte | [x] |
| 16 | `w_utf8_drop` | lead `0xF0`–`0xF4` with `(x[1]&0xC0) != 0x80` (bad/absent 2nd byte) | returns pointer to the lead byte | [x] |
| 17 | `w_utf8_drop` | lead `0xF0`–`0xF4`, good 2nd, `(x[2]&0xC0) != 0x80` (bad/absent 3rd byte) | returns pointer to the lead byte | [x] |
| 18 | `w_utf8_drop` | lead `0xF0`–`0xF4`, good 2nd+3rd, `(x[3]&0xC0) != 0x80` (bad/absent 4th byte) | returns pointer to the lead byte | [x] |
| 19 | `w_utf8_filter` | any rejected byte (rows 3–18) with `replacement == false` | byte dropped from output | [x] |
| 20 | `w_utf8_filter` | any rejected byte (rows 3–18) with `replacement == true` | `EF BF BD` emitted in its place | [x] |
| 21 | `w_utf8_filter` | `malloc(strlen+1)` returns `NULL` | function returns `NULL` | n/a (cannot be induced without an allocator interposer; both impls call the *same* libc `malloc` and have byte-identical `if (copy == NULL) return NULL;` control flow) |
| 22 | `w_utf8_filter` | `realloc(copy, size)` returns `NULL` | function returns `NULL` (original block leaked) | n/a (same reasoning as row 21) |
| 23 | `w_utf8_filter` | out-of-range `_Bool` argument (`2`, `0xFF`, high garbage bits) passed across FFI — a C `_Bool` parameter accepts any int-sized value from a foreign caller | whatever the C's `if (replacement)` does with that byte; Rust must match | [x] |
| 24 | `w_utf8_filter` | zero-length input (`""`), i.e. `w_utf8_drop` immediately at `NUL` | `strdup("")` → 1-byte `""` copy, non-NULL | [x] |
| 25 | `w_utf8_filter` | first byte already invalid (`i == 0`, `memcpy` of length 0) | no `memcpy`, filtering starts at offset 0 | [x] |
| 26 | `w_utf8_filter` | ≥ 1366 rejected bytes with `replacement == true` — drives `repl` from 4096 down past 3 and forces a **second** `realloc` (`4096 % 3 == 1`, so `repl` hits `1 < 3` on the 1366th replacement) | second `realloc`, output stays correct | [x] |
| 27 | `w_utf8_drop` / `w_utf8_filter` | input that is entirely valid (no rejection at all) — `*valid == '\0'` fast path | `w_utf8_drop` returns pointer to the `NUL`; `w_utf8_filter` returns `strdup(string)` | [x] |

## Divergences found and fixed in the Rust (the C was never touched)

1. **Rows 1 & 2 — NULL pointer signal mismatch.** The C's
   `assert(string != NULL)` is compiled in: the CMake build passes no
   `-DNDEBUG` (only `-fPIC`) and `__assert_fail` is an undefined symbol of
   `libdriver.so`, so `w_utf8_drop(NULL)` / `w_utf8_filter(NULL, …)` die with
   `SIGABRT`. The Rust used `debug_assert!`, which is compiled out of a release
   `cdylib`, so it fell through to a null dereference and died with `SIGSEGV`.
   Changed both to `assert!`; with `panic = "abort"` in the release profile
   the Rust now aborts with `SIGABRT` like the C.
2. **Row 23 — `_Bool` argument.** `w_utf8_filter`'s second parameter was typed
   `bool` in Rust. A foreign caller can place any byte in that register, and a
   `bool` holding something other than 0/1 is undefined behaviour in Rust, so
   the match with the C was only luck. Retyped to `u8` (ABI-identical to
   `_Bool`) with `if replacement != 0`, which is exactly what the C's whole-byte
   `cmpb $0x0` test does. Verified for all 256 byte values.

## Harness validation (negative control)

Six deliberate mutations were injected into the Rust, rebuilt, and the suite
re-run; the source was restored afterwards (verified byte-identical).

| mutation | caught |
|----------|--------|
| `valid_2` lead floor `0xC2` → `0xC0` | yes |
| `assert!` → `debug_assert!` (null handling) | yes (rows 1 & 2) |
| replacement byte `0xBD` → `0xBE` | yes |
| `valid_3` surrogate test `< 0xA0` → `<= 0xA0` | yes (row 8) |
| `REPLACEMENT_INC` 4096 → 4095 | no — not observable |
| `repl -= 3` → `repl -= 2` | no — not observable |

The two survivors only change how much *slack* is over-allocated. Both remain
sufficient (each replacement needs 2 spare bytes; 4095 slack per 1365
replacements and 4096 slack per 2048 replacements both cover it), so no output
byte and no return value changes. They are semantically equivalent through the
public API rather than undetected divergences. The real Rust keeps the C's
exact constants (`4096`, `repl -= 3`).
