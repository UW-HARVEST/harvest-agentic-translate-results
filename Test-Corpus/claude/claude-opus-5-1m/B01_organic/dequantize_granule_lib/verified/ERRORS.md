# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` (43 lines) + `c_src/include/lib.h`.

Mechanical grep results — the *complete* inventory of rejection machinery:

```
$ grep -n 'return\|assert\|if\|while\|for\|NULL\|<\|>' c_src/src/lib.c
lib.c:7    if ((bs->pos += n) > bs->limit)   <-- THE ONLY explicit range check
lib.c:8        return 0;                     <-- THE ONLY error sentinel
lib.c:10   while ((shl -= 8) > 0)
lib.c:14   return cache | (next >> -shl);
lib.c:20   for (j = 0; j < 4; j++)
lib.c:22   for (i = 0; i < 2 * sci->total_bands; i++)
lib.c:24   if (ba != 0)                      <-- band-skip check
lib.c:25   if (ba < 17)                      <-- branch selector / range check
lib.c:27   for (k = 0; k < group_size; k++)
lib.c:33   for (k = 0; k < group_size; k++, code /= mod)
lib.c:42   return group_size * 4;            <-- unconditional, never an error code
$ grep -c 'assert\|NULL\|errno\|RETURN_ERROR\|enum' c_src/src/lib.c c_src/include/lib.h
0
```

Findings that shape the table:

* There are **no** `assert`s, **no** NULL checks, **no** error enums, and **no**
  error-code return path. `dequantize_granule` **always** returns
  `group_size * 4`; it can never signal failure.
* The single rejection in the library is the bit-reservoir underflow check in
  `get_bits`, which returns the sentinel `0` **and still mutates `bs->pos`**.
* Everything else is a *silent* rejection: a guard whose failure skips work
  (`ba != 0`, `total_bands == 0`, `group_size <= 0`) or an *unchecked* access
  that C performs anyway (`bitalloc[i]` past its 64 bytes, out-of-range shift
  counts, signed overflow). Those are the rows that a happy-path test misses,
  so they are all in the table.
* **Out-of-range "enum" values:** the API declares no `enum`. Its moral
  equivalent is the `uint8_t bitalloc[i]` opcode, whose full `0..=255` domain is
  meaningful to C but where only `1..=16` is the "documented" MPEG range. Rows
  E14–E19 cover the undocumented remainder, and row E24 sweeps all 256 values.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E1 | `get_bits` (`lib.c:7`) | `bs->pos + n > bs->limit` — bit reservoir exhausted | returns `0`; **`bs->pos` is still advanced by `n`**; no byte of `bs->buf` is read | [x] |
| E2 | `get_bits` (`lib.c:7`) | boundary: `bs->pos + n == bs->limit` exactly | **not** rejected — returns the real bits (`>` not `>=`) | [x] |
| E3 | `get_bits` (`lib.c:7`) | one step past the range: `bs->pos + n == bs->limit + 1` | rejected → `0` | [x] |
| E4 | `get_bits` (`lib.c:7`) | `bs->limit < 0` (e.g. `-1`) with `pos = 0`, `n > 0` | *every* call rejects → every `ba<17` field becomes `0 - half`, every `ba>=17` `code` becomes `0` | [x] |
| E5 | `get_bits` (`lib.c:7`) | `bs->limit == INT_MAX` | never rejects → reads `buf` unbounded for as many bytes as `n` demands | [x] |
| E6 | `get_bits` (`lib.c:6`) | `bs->pos < 0` → `p = buf + (pos >> 3)` (arithmetic shift) reads **before** `buf`; `s = pos & 7` is still `0..7` | no rejection; out-of-bounds read performed | [x] |
| E7 | `get_bits` (`lib.c:7`) | `bs->pos` near `INT_MAX` so `bs->pos += n` **signed-overflows** | wraps negative ⇒ the `> limit` test spuriously **passes** and bits are read | [x] |
| E8 | `get_bits` (`lib.c:10,14`) | single-byte path: `n + s <= 8`, so the `while` body never runs. (`n <= 0` is **unreachable** through the public API — `n` is either `ba >= 1` or `mod + 2 - (mod>>3) >= 3`; the smallest reachable `n` is 1, the largest 1879048195) | reads exactly 1 byte, returns `(*p & (255>>s)) >> (8-n-s)`; the final `next >> -shl` shift count is therefore always in `0..=7` | [x] |
| E9 | `get_bits` (`lib.c:10`) | `n >= 32` (reachable: `ba>=17` asks for up to 28675 bits) | with a large `limit`: shift counts `>= 32` in `next << shl` / `next >> -shl` are UB, x86 masks them to `& 31` | [x] |
| E10 | `dequantize_granule` (`lib.c:24`) | `sci->bitalloc[i] == 0` | band skipped: **no** bits consumed, **no** writes to `dst[0..group_size]`; `dst` still advances by `choff` and `choff` still toggles | [x] |
| E11 | `dequantize_granule` (`lib.c:22`) | `sci->total_bands == 0` | `i`-loop never runs, `choff` never toggles, nothing written; returns `group_size * 4` | [x] |
| E12 | `dequantize_granule` (`lib.c:27,33`) | `group_size <= 0` (`0`, `-1`, `INT_MIN`) | `k`-loops never run → no writes; **but** the `ba>=17` branch still calls `get_bits` once per band, so `bs->pos` advances; `ba<17` bands consume nothing. Returns `group_size * 4` (≤ 0) | [x] |
| E13 | `dequantize_granule` (`lib.c:22,23`) | `sci->total_bands > 32` ⇒ `i` runs to `2*total_bands - 1` (max 509) but `bitalloc` is only 64 bytes | **unchecked OOB read**: `i in 64..=127` reads `scfcod[i-64]`, `i >= 128` reads memory past the struct. No bounds check, no rejection | [x] |
| E14 | `dequantize_granule` (`lib.c:31`) | `ba >= 49` ⇒ `2 << (ba - 17)` has shift count `>= 32` (up to 238) | UB shift; x86 masks the count to `(ba-17) & 31` | [x] |
| E15 | `dequantize_granule` (`lib.c:31,33,34`) | `ba == 48` ⇒ `2 << 31` == `0` ⇒ `mod == 1` | `mod/2 == 0`, `code % 1 == 0` → every `dst[k] = 0.0f`. `mod` is `(even)+1` hence **never 0**, so `% mod` / `/= mod` can never divide by zero | [x] |
| E16 | `dequantize_granule` (`lib.c:25,26`) | boundary `ba == 16`, the largest value taking the `ba < 17` branch | `half = (1<<15)-1 = 32767`, `get_bits(bs, 16)` | [x] |
| E17 | `dequantize_granule` (`lib.c:25,31`) | boundary `ba == 17`, the smallest value taking the `else` branch | `mod = 3`, `n = 3 + 2 - 0 = 5` | [x] |
| E18 | `dequantize_granule` (`lib.c:26`) | boundary `ba == 1` ⇒ `1 << 0` | `half == 0`, values are the raw 1-bit codes `0`/`1` | [x] |
| E19 | `dequantize_granule` (`lib.c:31,32`) | `ba == 255` (max `uint8_t`) | `mod = (2<<14)+1 = 32769`; `n = 32769 + 2 - 4096 = 28675`. With a reservoir shorter than that, `get_bits` rejects (E1) ⇒ `code = 0` ⇒ every `dst[k] = -16384.0f`; with a longer one the 28675-bit read is **accepted** (E9 shift-masking path). Both are tested | [x] |
| E20 | `dequantize_granule` (`lib.c:19,38,39`) | `choff` is initialised **outside** the `j` loop and toggles `576 ⇄ -558`; `dst` walks far past `grbuf` (max element offset `3*group_size + 5148 + group_size-1`) | **unchecked OOB writes** past `grbuf`; no bounds check. (`2*total_bands` is always even, so `choff` re-enters each `j` at `576`.) | [x] |
| E21 | `dequantize_granule` (`lib.c:21,42`) | `group_size` huge (e.g. `0x40000000`, `INT_MAX`) ⇒ `group_size * 4` and `group_size * j` **signed-overflow** | wraps: returns `0` for `0x40000000`, `-4` for `INT_MAX` | [x] |
| E22 | `dequantize_granule` | `grbuf == NULL` / `bs == NULL` / `sci == NULL` | **no null checks anywhere** ⇒ SIGSEGV. Reachable-without-deref variants are asserted instead: `grbuf == NULL` with `total_bands == 0`, and `sci == NULL`+`bs == NULL` are documented-only (would kill the harness) | [x] |
| E23 | `get_bits` (`lib.c:6,9`) | `bs->buf == NULL` **and** `limit < 0` | the limit check returns *before* `*p` is dereferenced, so a NULL `buf` is **never** touched → no crash, all fields `0 - half` | [x] |
| E24 | `dequantize_granule` (`lib.c:23–35`) | sweep: `bitalloc[i]` over its **entire** `0..=255` domain (out-of-range "opcode" values) | each value selects skip / `ba<17` / `ba>=17` per the C branches; no value is rejected | [x] |

All 24 rows are covered by `tests/differential.rs` (`phase_c_*` tests).

## In-process observability

Rows E6, E7, E9, E13, E14, E19, E20 and E24 describe inputs on which the C code
performs *unchecked* accesses. Where those accesses land at a wild address, the C
library segfaults — and so must a faithful Rust translation. The harness
therefore models `get_bits`'s accept/reject decisions exactly (they depend only
on `bitalloc`, `total_bands`, `group_size`, `pos` and `limit`, never on the
reservoir *contents*; see `reach`/`observable` in `tests/differential.rs`), and:

* re-bases `bs->buf` (`BufSpec::Rebased`) so that huge/negative `bs->pos` values
  still read inside a padded arena — this is how E6/E7 are exercised for real;
* declines (and **counts**) the residual cases whose predicted accesses leave the
  arena, because those would take the test process down rather than produce a
  comparable result. Every test asserts a minimum number of *observable* cases
  actually ran, so a row can never silently degrade to "ran nothing".

Each mechanism reachable only through a skipped case is still covered by a
dedicated observable case:

| mechanism | observable coverage |
|---|---|
| `bs->pos` advancing by a huge `n` on **reject**, incl. `int` wraparound | E24 / row10 / row25 "starved" companions (`limit = 0` / `INT_MIN`): every read rejects, so **no** memory is touched and every one of the 256 opcodes is compared |
| accepted read with `n >= 32` (UB shift counts masked to `& 31`) | E9 (`ba` 17..=37, up to 1 835 011 bits / 229 377 bytes actually read) |
| `bs->pos += n` overflow making the check pass | E7a (`pos = INT_MAX`) and E7b (`pos = INT_MAX - 3n`, the overflow lands on the last call) |
| reading below `bs->buf` via `pos >> 3` (arithmetic shift) | E6, row22 |
| `bitalloc[i]` past its 64 bytes | E13, row20 (`total_bands` up to 255 ⇒ `i` up to 509) |
