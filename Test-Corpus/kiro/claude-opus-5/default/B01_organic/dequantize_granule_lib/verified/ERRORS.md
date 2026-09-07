# ERRORS.md — Error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. Every
`return`, every conditional, and every implicit guard was enumerated:

```
$ grep -n 'return|assert|NULL|RETURN_ERROR|errno|-1' src/lib.c include/lib.h
src/lib.c:8:        return 0;                       <- the ONLY error return
src/lib.c:14:    return cache | (next >> -shl);
src/lib.c:42:    return group_size * 4;

$ grep -n 'if (|while (|for (|switch|#if' src/lib.c
7:    if ((bs->pos += n) > bs->limit)     <- guard
10:   while ((shl -= 8) > 0)              <- loop guard
20:   for (j = 0; j < 4; j++)
22:   for (i = 0; i < 2 * sci->total_bands; i++)   <- guard on total_bands
24:   if (ba != 0)                        <- guard: skip band
25:   if (ba < 17)                        <- mode split
27:   for (k = 0; k < group_size; k++)    <- guard on group_size
33:   for (k = 0; k < group_size; k++, code /= mod)
```

**There is no `RETURN_ERROR` macro, no error enum, no `assert`, no `NULL`
check, no explicit range check and no `errno` use anywhere in this library.**
`dequantize_granule` has exactly one `return` and it is unconditional
(`group_size * 4`); it can never signal failure. The only in-band rejection in
the whole library is `get_bits`'s bitstream-exhaustion guard, which is
*silent*: it returns `0` bits and still advances `bs->pos`.

Consequently the "error surface" of this library is (a) that one guard, (b) the
guards that cause work to be *skipped*, and (c) the out-of-nominal-range /
undefined-behaviour inputs the C nevertheless accepts and processes. Rows below
are one per distinct rejection/guard branch plus the generic FFI boundaries
required by Phase C. `expected C result` is what the C `.so` actually does — it
is asserted by comparing against the C `.so`, never against a guess.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `get_bits` (via `dequantize_granule`) | `(bs->pos += n) > bs->limit` — bitstream exhausted on the very first `get_bits` call (`limit == 0`, `pos == 0`, `n >= 1`) | returns `0`; `bs->pos` is **still advanced** by `n`; caller writes `0 - half` | [x] |
| E2 | `get_bits` | exhausted *mid-run*: `limit` set so the first few reads succeed and later ones trip the guard | prefix decoded from `buf`, suffix all `0` bits; `bs->pos` ends at `initial_pos + Σn` | [x] |
| E3 | `get_bits` | exact boundary `bs->pos + n == bs->limit` (guard is `>`, not `>=`) | guard **not** taken — bits are read normally | [x] |
| E4 | `get_bits` | one step past the boundary `bs->pos + n == bs->limit + 1` | guard taken — returns `0` | [x] |
| E5 | `get_bits` | `bs->limit < 0` (negative limit, e.g. `-1`) | every call trips the guard; all bits `0` | [x] |
| E6 | `get_bits` | `bs->pos != 0` at entry and not byte aligned (`pos & 7 != 0`); first byte masked with `255 >> s` | high `s` bits of the first byte are discarded | [x] |
| E7 | `get_bits` | `bs->pos` already `> bs->limit` before the call | guard taken immediately, `0` | [x] |
| E8 | `get_bits` | `shl -= 8` never `> 0` (i.e. `n + (pos&7) <= 8`), so the `while` body never runs and `cache` stays `0` | result is `(*p & (255>>s)) >> -shl` only | [x] |
| E9 | `dequantize_granule` | `sci->total_bands == 0` → `i < 0` false on entry, inner loop never runs | no writes to `grbuf`, `bs` untouched, returns `group_size * 4` | [x] |
| E10 | `dequantize_granule` | `sci->bitalloc[i] == 0` for all `i` (`ba != 0` guard rejects every band) | no `get_bits` call, no `grbuf` write, `bs->pos` unchanged, returns `group_size*4` | [x] |
| E11 | `dequantize_granule` | `group_size == 0` → `k < 0` false, both `k` loops empty | `ba<17`: no `get_bits`; `ba>=17`: **one** `get_bits` still consumed per band; returns `0` | [x] |
| E12 | `dequantize_granule` | `group_size < 0` (e.g. `-1`, `-7`) | `k` loops empty, `dst` starts *before* `grbuf`, no writes; returns `group_size*4` (negative) | [x] |
| E13 | `dequantize_granule` | `group_size` far larger than any real granule (e.g. `64`, `256`) — no clamp exists | writes `group_size` floats per active band, walking `grbuf` well past 576 | [x] |
| E14 | `dequantize_granule` | `ba == 16` — largest value still taking the `ba < 17` branch; `half = 0x7FFF` | linear path with `1<<15 - 1` bias | [x] |
| E15 | `dequantize_granule` | `ba == 17` — smallest value taking the grouped branch; `mod = 3`, `n = 5` | grouped path | [x] |
| E16 | `dequantize_granule` | `ba` in `18..=31`: `2 << (ba-17)` still in range but `n = mod+2-(mod>>3)` grows past 32 bits requested → `next << shl` with `shl >= 32` is **C UB** | x86 masks the shift count to 5 bits; Rust must reproduce it (`wrapping_shl`) | [x] |
| E17 | `dequantize_granule` | `ba >= 49`: `2 << (ba-17)` has a shift count `>= 32` — **C UB** | x86 masks count to 5 bits, so `ba` wraps mod 32 (`ba=49` ≡ `ba=17`) | [x] |
| E18 | `dequantize_granule` | `ba == 48`: shift count `31`, `2 << 31` overflows to `0`, so `mod == 1` | `code % 1 - 1/2 == 0` → every sample is `0.0`, `n == 3` | [x] |
| E19 | `dequantize_granule` | `ba` so large that `n` exceeds the buffer (`ba` ≈ 40, `n` ≈ 7.3e6 bits) | `bs->pos += n` trips the `limit` guard before any deref → `0`; no OOB read | [x] |
| E20 | `dequantize_granule` | `code % mod < mod / 2` in the grouped path — `unsigned` subtraction **wraps** before the cast to `int` | huge `unsigned` → negative `int` → negative float (must not be a saturating/checked sub) | [x] |
| E21 | `dequantize_granule` | `ba` read at `i >= 64`, i.e. `total_bands > 32`: `sci->bitalloc[i]` runs **off the 64-byte array** into `scfcod` | C reads the adjacent struct bytes; Rust must read the same bytes | [x] |
| E22 | `dequantize_granule` | `ba` read at `i >= 128`, i.e. `total_bands > 64`: index runs **past the end of `L12_scale_info`** (max `i` = 509 at `total_bands == 255`) | C reads whatever follows the struct; Rust must read the same bytes | [x] |
| E23 | `dequantize_granule` | `sci->total_bands == 255` (max `uint8_t`) → 510 bands per granule | full 510-band walk, `choff` returns to 576 each `j` (even count) | [x] |
| E24 | `dequantize_granule` | out-of-range "enum"-like ints across FFI: `group_size` = `INT_MIN`-ish / large; `ba` sweeping the **entire** `0..=255` byte domain (no valid-variant check exists) | all 256 `ba` values accepted and processed; no rejection | [x] |
| E25 | `get_bits` | `bs->buf` points at all-`0x00` bytes / all-`0xFF` bytes (degenerate content, not a rejection but the extreme of the value domain) | `0x00`: all samples `-half`; `0xFF`: all bits set | [x] |
| E26 | `dequantize_granule` | NULL pointers (`grbuf`, `bs`, `sci`) | C **has no null check** → dereference → `SIGSEGV`. Behaviour is a crash, identical in both; asserted out-of-process where the guards make the deref unreachable (`total_bands==0` + `grbuf==NULL`, `group_size<=0`) | [x] |

Notes on rows deliberately **not** present: there is no invalid `stereo_bands`
(the field is never read), no invalid `scf` (never read), and no division-by-zero
row for `code % mod` / `mod / 2` because `mod = (2 << k) + 1` is always odd and
therefore never `0`.

## Status

All 26 rows have a passing differential test in
`tests/phase_c_errors.rs` (26 tests, `26 passed; 0 failed`). Every test builds
the exact triggering condition, calls both `.so`s through `dlsym`, and asserts
the same outcome *and* the same rejection mechanism -- not merely "both failed".

Each row's test also asserts its own premise, so a row cannot pass vacuously:
E3 proves at-limit and one-past-limit actually differ; E11 proves the grouped
path really does consume `n` bits with `group_size == 0` while the linear path
consumes none; E16/E18/E19 assert the computed `n` before testing it; E20 proves
a negative sample was actually produced by the unsigned wrap; E21/E22 assert
that `ba` really was fetched from beyond `bitalloc[64]` / beyond the struct;
E24 asserts that some `ba` values really do fault in *both* libraries (and that
most do not).

### The one divergence found, and the fix

`e26_null_pointers` failed against the **debug**-profile Rust `.so`: on a NULL
`sci` the C dies with `SIGSEGV` (11) while the Rust died with `SIGABRT` (6),
because Rust's `debug_assertions`-gated UB check reported "null pointer
dereference occurred" at `src/lib.rs:152`. Since this translation must reproduce
accesses that are UB by Rust's rules, those checks are now switched off in both
profiles (`Cargo.toml`); `src/lib.rs` was not changed. The release profile
always matched.

### Extra confirmation

The whole suite also passes against a C library rebuilt with `-O3 -DNDEBUG`
(built from a copy outside `c_src/`, which was never modified), so the
translation's reproduction of the C's undefined shift counts and signed
overflow does not depend on how aggressively the C is optimised.
