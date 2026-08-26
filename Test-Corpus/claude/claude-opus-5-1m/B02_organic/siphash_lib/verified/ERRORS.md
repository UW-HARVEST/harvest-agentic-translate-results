# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c` (126 lines) and `c_src/include/lib.h`.

## 1. Mechanical grep of every rejection construct

```
$ grep -n "return" src/lib.c include/lib.h
src/lib.c:107:  return v0 ^ v1 ^ v2 ^ v3;
src/lib.c:111:  return stbds_siphash_bytes(p, len, seed);

$ grep -n -E "RETURN_ERROR|assert|NULL|errno|-1|abort|exit|perror|enum|#ifdef|#if |#define" \
      src/lib.c include/lib.h
(no matches)
```

**Findings:**

* There are exactly **two `return` statements** in the whole library, and **neither is an
  error return** — line 107 returns the finalized hash `v0^v1^v2^v3`, line 111 forwards
  that value. `siphash` returns `void`.
* **No** error-return macro, **no** `RETURN_ERROR`, **no** `return -1`, **no**
  `return NULL`, **no** error enum, **no** `errno` use, **no** `assert`, **no**
  `abort`/`exit`, **no** null-pointer check, **no** explicit range check, and **no**
  min/max constant.
* There are **no `enum` types anywhere** in the public API, so there is no
  "out-of-range enum value across FFI" surface for this library. The only non-pointer,
  non-size parameter is `int init`, for which **every** one of the 2^32 `int` values is a
  valid input (see rows 8–11) — it is not an enum and has no rejected values.
* Both public functions therefore have a **total** input domain: they accept every
  argument value and *cannot* signal failure. Invalid input is not *rejected*; it is
  either (a) absorbed by defined-but-surprising integer semantics, or (b) an
  out-of-bounds read whose consequence is a fault.

The "error surface" of this library is thus the set of *implicit* rejections listed
below. Rows 1–7 are the only branch-level "do nothing / take no bytes" outcomes the C
code actually contains; rows 8–11 are the generic FFI boundaries mandated for every C
API (null pointers, zero and oversized lengths, one-step-past-range values).

## 2. The table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `stbds_siphash_bytes` line 48 `switch (len - i)` → `case 0` (line 63–64) | `len - i == 0`, i.e. `len` is an exact multiple of `sizeof(size_t)` (incl. `len == 0`) | falls straight to `break`: **no tail byte is OR'd**; `data` stays `len << 56`; still returns a hash (no error) | `error_paths.rs::row01_tail_case_zero_takes_no_bytes` | [x] |
| 2  | `stbds_siphash_bytes` line 48 `switch (len - i)` → **no matching case** (implicit `default`) | `len - i > 7`. Unreachable by construction: the loop at line 18 runs while `i + 8 <= len`, so on exit `len - i ∈ 0..=7`. Asserted structurally over `len ∈ 0..=4096`. | switch body skipped entirely; identical to row 1 | `error_paths.rs::row02_switch_default_unreachable` | [x] |
| 3  | `stbds_hash_bytes` | `len == 0` **and** `p == NULL` | **no dereference happens** (loop body never runs, `case 0` takes no bytes) → returns the "empty input" hash for that seed, *not* a crash and *not* an error code | `error_paths.rs::row03_null_ptr_len_zero` | [x] |
| 4  | `stbds_hash_bytes` | `len == 0` and `p` a valid non-null pointer | same value as row 3 — the buffer contents are irrelevant when `len == 0` | `error_paths.rs::row04_len_zero_ignores_buffer` | [x] |
| 5  | `stbds_siphash_bytes` line 47 `data = len << 56` | `len >= 256`, i.e. `len` "one step past" the range that fits in the 8 bits the expression keeps | shift discards all but `len & 0xFF`; **no error** — `len` and `len + 256` contribute the same tail bits (they still differ via loop count) | `error_paths.rs::row05_len_shift_truncates_to_low_byte` | [x] |
| 6  | `stbds_siphash_bytes` line 56 `data \|= (d[3] << 24)` (tail `case 4`) | tail byte `d[3] >= 0x80` — signed `int` overflow, then sign-extension into `size_t` | bits 31..63 of `data` are all forced to 1, **masking out** `len << 56` **and** tail bytes `d[4]`/`d[5]`/`d[6]`; consequence: `len` 4,5,6,7 collide and `d[4..7]` are ignored. No error is reported. | `error_paths.rs::row06_tail_sign_extension_collision` | [x] |
| 7  | `stbds_siphash_bytes` line 20 `data = d[0] \| … \| (d[3] << 24)` (main loop) | block byte `d[3] >= 0x80` — same signed overflow / sign-extension | bits 32..63 of `data` are forced to 1 **before** line 21 ORs the high word in, so the high 32 bits stay `0xFFFFFFFF` and `d[4..7]` are ignored for that block. No error. | `error_paths.rs::row07_block_sign_extension_swallows_high_word` | [x] |
| 8  | `stbds_hash_bytes` | `p == NULL`, `len > 0` (1, 7, 8, 9, 4096) | out-of-bounds read of address 0 → process dies on **SIGSEGV**; there is no null check to return an error code | `error_paths.rs::row08_null_ptr_nonzero_len_faults` (forked child, compares signal) | [x] |
| 9  | `stbds_hash_bytes` | oversized `len`: valid 4 KiB buffer, `len = 1 GiB` | reads past the mapping → **SIGSEGV**; no length validation exists | `error_paths.rs::row09_oversized_len_faults` (forked child, compares signal) | [x] |
| 10 | `stbds_hash_bytes` | `seed` one step past / at the extremes of its range: `0`, `1`, `usize::MAX` (so `~seed == 0`), `1 << 63`, `usize::MAX - 1` | every value is valid; `seed` and `~seed` both feed `v0..v3`. No rejection. | `error_paths.rs::row10_seed_extremes` | [x] |
| 11 | `siphash` | `init` at/past the `int` extremes: `INT_MIN`, `INT_MAX` (where `z++` at line 118 overflows), `-1`, `0`. Also the out-of-declared-domain case: `init` is not an enum, so all 2^32 values are in range. | `mem[i] = z` truncates the `int` to `unsigned char`; `z++` wraps `INT_MAX → INT_MIN`. Prints 64 lines to stdout, returns `void`. No rejection. | `error_paths_siphash.rs::row11_siphash_int_extremes` | [x] |

## 3. Not testable by assertion

None. Rows 8 and 9 are the only faulting cases, and they are covered by forking a child
process per implementation and asserting **both** die from the **same** signal
(`WTERMSIG == SIGSEGV`), rather than merely "both failed somehow".
