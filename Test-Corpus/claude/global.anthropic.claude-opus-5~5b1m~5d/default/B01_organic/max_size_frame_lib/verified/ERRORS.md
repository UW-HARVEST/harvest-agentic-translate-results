# ERRORS.md — Phase A: error-surface table

## Mechanical derivation (what was grepped, and what was found)

Ran over the **entire** C source (`c_src/include/lib.h`, `c_src/src/lib.c`):

| grep pattern | matches |
|---|---|
| `return -1` / `return NULL` / `return 0` / `RETURN_ERROR` / `ERROR` / `_ERR` / `errno` | **none** |
| `assert` | **none** |
| `goto` / `exit(` / `abort(` | **none** |
| `if` / `else` / `switch` / `case` / `while` / `for` / `do` / `?:` | **none** |
| `#if` / `#ifdef` / `#ifndef` / `#else` / `#elif` / `#define` | **none** |
| `*` (pointer decl) / `[` (array) / `struct` / `union` / `enum` | **none** (the `*` hits are multiplications only) |

**Result: the C library has ZERO explicit rejection paths.**
`max_size_frame` is a *total*, pure function of three `uint32_t` values. It
takes no pointers (so there is no null-pointer surface), no lengths/buffers (so
there is no oversized-length surface), and no enums (so there is no
out-of-range-enum-variant surface — `uint32_t` makes **all** 2³² values of each
parameter legal input that the C accepts and answers). There is no error code,
no sentinel return, and no way to make it fail.

Because there is no rejection surface to compare, the equivalent
"must-not-diverge" surface is the set of **implicit** boundaries the C
arithmetic itself distinguishes: the 4 predicates (`channels != 2`,
`channels == 2` ×2, `bitdepth != 32`), unsigned wraparound at 2³², the
multiplicative annihilations (`×0`), and the `(x + 7) / 8` ceiling. Each row
below is one such condition, with the exact value the C produces (computed
from the C semantics and asserted against the real C `.so`).

## Error / boundary surface table

`MAX` = `0xFFFFFFFF` = 4294967295. Argument order: `(blocksize, channels, bitdepth)`.

| # | function | trigger (the exact invalid/boundary input or condition) | expected C result | [x] |
|---|----------|----------------------------------------------------------|-------------------|-----|
| E1 | `max_size_frame` | `channels = 0` — annihilates `t1` via `channels * (channels != 2)`; no rejection | `18` (for any bs, bd) | [x] |
| E2 | `max_size_frame` | `blocksize = 0` — zero-length block is **not** rejected; all 3 terms vanish | `18 + channels` (`(0,2,16)` → `20`) | [x] |
| E3 | `max_size_frame` | `bitdepth = 0` with `channels != 2` — invalid FLAC depth, **accepted** | `18 + channels` (`(4096,1,0)` → `19`) | [x] |
| E4 | `max_size_frame` | `bitdepth = 0` with `channels == 2` — `t3 = bs*(0+1)` survives | `(4096,2,0)` → `532` | [x] |
| E5 | `max_size_frame` | `bitdepth = 32` exactly — `(bitdepth != 32)` is **false**, drops the +1 | `(4096,2,32)` → `32788` | [x] |
| E6 | `max_size_frame` | `bitdepth = 31` — one step below the boundary, `+1` applies | `(4096,2,31)` → `32276` | [x] |
| E7 | `max_size_frame` | `bitdepth = 33` — one step **past** the documented max depth, accepted, `+1` applies | `(4096,2,33)` → `34324` | [x] |
| E8 | `max_size_frame` | `channels = 2` exactly — stereo path (`t1 = 0`, `t2`+`t3` live) | `(4096,2,16)` → `16916` | [x] |
| E9 | `max_size_frame` | `channels = 1` — one below the stereo boundary | `(4096,1,16)` → `8211` | [x] |
| E10 | `max_size_frame` | `channels = 3` — one above the stereo boundary | `(4096,3,16)` → `24597` | [x] |
| E11 | `max_size_frame` | `blocksize = MAX` — `bs*bd` wraps mod 2³² | `(MAX,1,1)` → `19` | [x] |
| E12 | `max_size_frame` | `bitdepth = MAX` with `channels == 2` — `bitdepth + 1` wraps to `0`, killing `t3` | `(4096,2,MAX)` → `536870420` | [x] |
| E13 | `max_size_frame` | `channels = MAX` — `18 + channels` itself overflows | `(1,MAX,1)` → `17` | [x] |
| E14 | `max_size_frame` | all three = `MAX` — maximal simultaneous overflow | `(MAX,MAX,MAX)` → `17` | [x] |
| E15 | `max_size_frame` | `channels = MAX-17` — makes `18 + channels` land exactly on `0` | `(0,MAX-17,0)` → `0` | [x] |
| E16 | `max_size_frame` | sum-`+7` wraparound: `t1` near `MAX` so `+ 7` wraps past 0 | `(MAX,1,MAX)` → `20` | [x] |
| E17 | `max_size_frame` | `blocksize = 65536` — one step past FLAC's 16-bit max blocksize, accepted | `(65536,8,32)` → `2097178` | [x] |
| E18 | `max_size_frame` | division-by-8 truncation floor: `sum = 7` (`bs=0,ch=1,bd=1` → `7/8 = 0`) | `19` | [x] |
| E19 | `max_size_frame` | division-by-8 carry: `sum = 8` (`bs=1,ch=1,bd=1`) → quotient steps to `1` | `20` | [x] |
| E20 | `max_size_frame` | `bitdepth = MAX` with `channels != 2` — `bd+1` wrap is **unused** on this branch | `(1,1,MAX)` → `19` | [x] |
| E21 | `max_size_frame` | all 8 residues of `sum mod 8` — proves identical truncating (not rounding) division | `bs=0..9,ch=1,bd=1` → `19,20,20,20,20,20,20,20,20,21` | [x] |
| E22 | `max_size_frame` | division by zero is **impossible** (divisor is the literal `8`); no trap path exists in either impl | never traps, always returns | [x] |
| E23 | `max_size_frame` | N/A-by-construction rejections, asserted to be absent identically in both: null pointer, zero/oversized length, out-of-range enum variant — the API takes no pointer, no length, and no enum, so **every** bit pattern of all 3 args is valid input. Covered by exhaustive/random sweeps over the full `u32` domain incl. all values one step past every documented FLAC range (`0`, `9`, `33`, `65536`). | both return normally with identical values, never an error sentinel | [x] |

All 23 rows are exercised by `tests/errors.rs` (Phase C), each calling **both**
`.so`s through `libloading` and asserting the C value, the Rust value, and the
independently computed expected value all agree.
