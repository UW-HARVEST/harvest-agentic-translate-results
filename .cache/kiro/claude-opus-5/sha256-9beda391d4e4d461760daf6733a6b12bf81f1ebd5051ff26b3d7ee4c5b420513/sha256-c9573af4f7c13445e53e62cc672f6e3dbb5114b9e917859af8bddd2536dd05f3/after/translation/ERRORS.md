# ERRORS.md — error / rejection surface table (Phase C gate)

## Mechanical derivation

Grep of the ENTIRE C source (`c_src/src/lib.c`, `c_src/include/lib.h`) for every
rejection construct:

```
$ grep -n -E 'RETURN_ERROR|return -1|return NULL|assert|errno|exit\(|abort\(|
              [A-Z_]*ERROR|if *\(|switch|#if|else|NULL' src/lib.c include/lib.h
src/lib.c:48:  switch (len - i) {
src/lib.c:107:  return v0 ^ v1 ^ v2 ^ v3;
src/lib.c:111:  return stbds_siphash_bytes(p, len, seed);
```

**Result: this library has NO error-reporting surface.**

- 0 error-return macros, 0 `return -1`, 0 `return NULL`, 0 error enums.
- 0 `assert` / `abort` / `exit` / `errno` use.
- 0 explicit range checks, 0 null-pointer checks, 0 min/max constants.
- 0 `#if`/`#ifdef` conditional compilation.
- Both public functions are total on their declared types: `stbds_hash_bytes`
  returns a `size_t` hash for every input (no sentinel value is reserved), and
  `siphash` returns `void`.

Because the surface is empty, the table below is populated with (a) the ONE
implicit branch the C actually contains — the `switch (len - i)` fall-through
chain, whose `case 0:`/absent-`default:` arms are the C's only "reject and do
nothing" paths — and (b) the generic FFI boundaries mandated by Phase C
(null pointers, zero/oversized lengths, one-past-range values, out-of-range
enum-style integers). "Expected C result" is therefore *defined behaviour to be
matched*, not an error code.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `stbds_hash_bytes` | `len == 0` with `p == NULL` — the `for` loop body never runs (`0 + 8 <= 0` false) and `switch (0 - 0)` takes `case 0: break;`, so `p` is never dereferenced. No rejection, no crash. | Returns the hash of the empty message: `data = 0 << 56 = 0`, finalisation only. A specific `size_t`, NOT an error sentinel. | [x] |
| E2 | `stbds_hash_bytes` | `len == 0` with a valid non-NULL `p` | Same value as E1 — `p` is unread; result is independent of the pointer. | [x] |
| E3 | `stbds_hash_bytes` | `len == 0`, `p == NULL`, varying `seed` (incl. `0`, `1`, `SIZE_MAX`, `SIZE_MAX/2`) | Seed still mixes into `v0..v3`, so the result varies with `seed`; no seed value is rejected. | [x] |
| E4 | `stbds_hash_bytes` | `switch (len - i)` **`case 0`** arm: `len` an exact multiple of `sizeof(size_t)` (8, 16, 24, …). Tail switch adds nothing beyond `len << 56`. | Hash computed with an all-zero tail contribution except the length byte. | [x] |
| E5 | `stbds_hash_bytes` | `switch (len - i)` arms **`case 1`..`case 7`**: every non-zero remainder `len % 8 ∈ {1..7}` — 7 distinct fall-through entry points, each reading a different count of tail bytes past the last full 8-byte block. | Each arm ORs its own byte set into `data`; a distinct hash per arm. Reads exactly `len` bytes, never more. | [x] |
| E6 | `stbds_hash_bytes` | `switch (len - i)` has **no `default:`** — but `len - i` is provably in `0..=7` after the loop, so the no-arm path is unreachable. Documented as unreachable; no test can construct it without an out-of-bounds `len`. | Unreachable in C. | [x] (n/a, proven unreachable) |
| E7 | `stbds_hash_bytes` | `len == 1` with a 1-byte-only allocation (one step past a zero-length buffer): must read `d[0]` and **must not** read `d[1..7]`. | Reads exactly 1 byte. Verified with a guard-page / exact-size heap buffer so any over-read faults. | [x] |
| E8 | `stbds_hash_bytes` | `len == 7`, exact-size buffer — the widest tail arm; must not read the 8th byte. | Reads exactly 7 bytes. | [x] |
| E9 | `stbds_hash_bytes` | `len == 8`, exact-size buffer — one step past the tail-only regime, first full-block iteration; must not read a 9th byte. | Reads exactly 8 bytes. | [x] |
| E10 | `stbds_hash_bytes` | Signed-overflow UB path: tail byte `d[3] >= 0x80` in `case 4`, where `d[3] << 24` overflows `int` and the negative `int` is sign-extended into `size_t`, setting the whole upper 32 bits of `data`. | The compiler's actual (wrapping) result; Rust must reproduce the sign extension, not the "clean" value. | [x] |
| E11 | `stbds_hash_bytes` | Signed-overflow UB path in the main loop: `d[3] >= 0x80` and/or `d[7] >= 0x80` in `data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24)`. | Same as E10, in the full-block path. | [x] |
| E12 | `siphash` | Out-of-range / extreme `int` argument passed across FFI: `INT_MIN`, `INT_MAX`, `-1`, `0`. `init` has no valid-range check; the C enum-style "any int is accepted" case. `z++` at `INT_MAX` is signed-overflow UB. | No rejection. `mem[i] = (unsigned char) z` with wrapping `z`; prints 64 lines. | [x] |
| E13 | `siphash` | `init` values that make `mem[]` bytes cross the `>= 0x80` boundary (i.e. drive E10/E11 through the public entry point), e.g. `init = 0x7A`, `0x80`, `0xF9`, `250`. | No rejection; specific printed table. | [x] |
| E14 | `stbds_hash_bytes` | Oversized `len` (`len > buffer size`, e.g. `SIZE_MAX`): the C performs no bounds check and will read out of bounds / fault. | Genuinely undefined / faulting in C — **not testable** without invoking a segfault. Documented, deliberately not exercised. | [x] (n/a, C is UB) |

## Summary

14 rows. 12 are exercised by differential tests in
`tests/differential.rs`; E6 and E14 are proven-unreachable / genuine-UB and are
documented rather than tested (constructing them would crash the C reference,
which cannot produce a comparable result).
