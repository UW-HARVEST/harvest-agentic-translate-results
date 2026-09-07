# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h` by grepping
for every rejection construct:

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|if *\(|switch|#if|-1|<=|>=|== *0|!= *0' src/lib.c include/lib.h
```

Full result set (4 hits, none of which is a rejection):

| line | text | classification |
|------|------|----------------|
| 18 | `for (i = 0; i + sizeof(size_t) <= len;` | loop bound, not a check |
| 48 | `switch (len - i) {` | tail dispatch, `case 0..7`, **no `default:`** |
| 107 | `return v0 ^ v1 ^ v2 ^ v3;` | success return (hash value) |
| 111 | `return stbds_siphash_bytes(p, len, seed);` | success return (pass-through) |

## Findings

The C library has **no error surface**:

* 0 error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, error enums)
* 0 `assert` / `abort` / `exit`
* 0 explicit range checks, null checks, or min/max constants
* 0 `#ifdef` configuration branches
* 0 enum parameters (so no out-of-range-enum class of input exists)
* Both public functions are **total** on their declared parameter types:
  `stbds_hash_bytes` always returns a `size_t` hash, `siphash` always returns
  `void` after printing exactly 64 lines.

Therefore the table rows below are the *generic C-API boundaries* the task
requires to be covered even when absent from the source, plus every branch the
tail `switch` distinguishes. "Expected C result" is defined as *whatever the C
`.so` does*, and each row's test asserts Rust reproduces it bit-for-bit — the
row is not a claim that C rejects the input.

## Error / boundary table

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | No dereference occurs (main loop body never runs since `0 + 8 <= 0` is false; tail `switch (0-0)` takes `case 0: break`). Returns the well-defined constant hash of the empty input. Must not crash. | [x] |
| E2 | `stbds_hash_bytes` | `len == 0` on a valid non-null `p` | Identical value to E1 — the pointer is never read. | [x] |
| E3 | `stbds_hash_bytes` | `len == 0`, `p` = one-past-the-end pointer of a buffer | Same constant as E1/E2, no read. | [x] |
| E4 | `stbds_hash_bytes` | `len` one step past the buffer's valid range is **not** rejected — C has no length validation. Tested as: `len == n` for a buffer of exactly `n` bytes (the maximal legal length) to pin the boundary that the next value would cross. | Reads exactly `len` bytes and hashes them; no rejection, no truncation. | [x] |
| E5 | `stbds_hash_bytes` | `len - i == 0`, i.e. `len % 8 == 0` and `len > 0` (tail `switch` `case 0`) | `data` = `len << 56` only; no tail bytes mixed in. | [x] |
| E6 | `stbds_hash_bytes` | tail `switch` `case 1` (`len % 8 == 1`) | falls through to `case 0: break`; `data |= d[0]`. | [x] |
| E7 | `stbds_hash_bytes` | tail `switch` `case 2` | `data |= (d[1] << 8)` then falls through case 1. | [x] |
| E8 | `stbds_hash_bytes` | tail `switch` `case 3` | `data |= (d[2] << 16)` then falls through. | [x] |
| E9 | `stbds_hash_bytes` | tail `switch` `case 4` with `d[3] >= 0x80` | `(d[3] << 24)` is a **signed-`int` overflow** whose result is negative; converting to `size_t` sign-extends, setting the whole upper 32 bits of `data`. Rust must reproduce, not "fix". | [x] |
| E10 | `stbds_hash_bytes` | tail `switch` `case 4` with `d[3] < 0x80` | no sign extension; only bits 24..31 set. | [x] |
| E11 | `stbds_hash_bytes` | tail `switch` `case 5` | `data |= ((size_t)d[4] << 16) << 16` (net shift 32). | [x] |
| E12 | `stbds_hash_bytes` | tail `switch` `case 6` | `data |= ((size_t)d[5] << 20) << 20` (net shift **40**, not 48). | [x] |
| E13 | `stbds_hash_bytes` | tail `switch` `case 7` | `data |= ((size_t)d[6] << 24) << 24` (net shift 48). | [x] |
| E14 | `stbds_hash_bytes` | main-loop `d[3] >= 0x80` (signed overflow in the *body*, line 20) | low word sign-extends → upper 32 bits of `data` all set before the high word is OR'd in. | [x] |
| E15 | `stbds_hash_bytes` | main-loop `d[7] >= 0x80` (line 21) | the sign-extended `int` is `<< 16 << 16`, so the extension bits shift out — upper bits come only from the byte value. Different outcome from E14 despite the same construct. | [x] |
| E16 | `stbds_hash_bytes` | `len == SIZE_MAX` style oversized length — cannot be executed safely (C would read out of bounds and segfault). Covered instead at the largest length the loop arithmetic still distinguishes: `len` values where `len << 56` aliases (`len` and `len + 256` produce the same `data` seed contribution). | `data`'s top byte is `len & 0xff` only; higher bits of `len` never reach the hash. | [x] |
| E17 | `stbds_hash_bytes` | `seed` = 0, `SIZE_MAX`, and arbitrary values | The seed **cancels out**: each `v` is `^ seed` (or `^ ~seed`) twice (lines 10-13 then 14-17), so the hash is independent of `seed`. Both C and Rust must return the same value for *all* seeds at fixed data. | [x] |
| E18 | `siphash` | `init` negative (e.g. `INT_MIN`, `-1`) | `int z = init; mem[i] = z;` truncates to `unsigned char` — implementation-defined narrowing, no rejection. 64 lines printed. | [x] |
| E19 | `siphash` | `init == INT_MAX` — `z++` **signed overflow** after the first iteration | C (gcc, `-fwrapv`-less) wraps to `INT_MIN`; the low byte sequence continues `0xff, 0x00, 0x01, ...`. Rust uses `wrapping_add` to match. | [x] |
| E20 | `siphash` | `init` = 0 and 255 (byte-boundary wrap of the `mem` fill) | 64 lines of 8 `0x%02x` values; byte-identical stdout. | [x] |

All 20 rows have a passing differential test in
`translation/tests/differential.rs` (see `phase_c_*` tests).
