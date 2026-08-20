# CONFIGS.md — configuration surface table (Phase B)

## Public entry points (from `c_src/include/lib.h`)

| entry point | linkage | how it is driven |
|---|---|---|
| `dequantize_granule(float*, bs_t*, L12_scale_info*, int)` | exported | called directly through the `.so` |
| `get_bits(bs_t*, int)` | `static` (lowest-level fn) | **not** exported by either `.so`; driven *indirectly but exhaustively* — every `n` it can ever see is chosen by `bitalloc[i]`, so rows that vary `bitalloc` + `bs.pos` + `bs.limit` exercise it directly (see "get_bits coverage" below) |

There are no init/config/destroy functions and no global state: the entire
configuration surface is the 4 arguments plus the two mutable structs they
point at.

## Axes the C code actually branches on

| axis | where C branches | values exercised |
|---|---|---|
| **A** `group_size` | `k < group_size` (`lib.c:27,33`), `grbuf + group_size*j` (21), `return group_size*4` (42) | `INT_MIN`, `-1`, `0`, `1`, `2`, `3`, `4`, `8`, `12`, `18`, `32`, `0x40000000`, `INT_MAX` |
| **B** `sci->total_bands` | `i < 2*total_bands` (`lib.c:22`) | `0`, `1`, `2`, `8`, `31`, `32` (⇒ `i` exactly fills `bitalloc[64]`), `33` (first OOB `i`), `64`, `255` (max) |
| **C** `bitalloc[i]` value class | `ba != 0` (24), `ba < 17` (25) | `0` (skip), `1` (min low), `2..15`, `16` (max low), `17` (min high), `18..47`, `48` (`mod==1`), `49..254`, `255` (max high) |
| **D** `bitalloc` pattern | drives the interaction between bands | all-zero, all-one-value, alternating `0`/nonzero, only-low, only-high, mixed low+high, fully random `0..=255` |
| **E** `bs->pos` start | `s = pos & 7` (4), `p = buf + (pos>>3)` (6) — 8 distinct bit alignments | `0`, `1..7` (all unaligned `s`), `8`, `13`, negative, `INT_MAX-k` (overflow) |
| **F** `bs->limit` | `pos > limit` (7) | `INT_MAX` (never rejects), exact total bit count, `limit-1` (one short), mid-stream (partial exhaustion), `0`, `-1` |
| **G** `bs->buf` bytes | `*p & (255>>s)`, `*p++` (9,12) | all `0x00`, all `0xFF`, `0xAA/0x55`, random, `NULL` (with `limit<0`) |
| **H** `n` passed to `get_bits` | `shl = n+s`, `while (shl -= 8) > 0` (10) — 1-byte vs multi-byte vs `>=32`-bit paths | `1..16` (from `ba<17`), `5..28675` (from `ba>=17`), i.e. 1-byte, 2-byte, 3-byte, 5-byte and ≥32-bit shift-masking paths |
| **I** `grbuf` prior content | never read, only written; unwritten slots must be **preserved** | pre-filled with a unique sentinel per element |
| **J** unread fields `scf`, `stereo_bands`, `scfcod` | never read by `lib.c` **except** `scfcod` via the `bitalloc` OOB (E13) | filled with random bytes; asserted byte-identical after the call |

## get_bits coverage

`get_bits` is `static`, so an external caller can only reach it through
`dequantize_granule`. Its own branch surface is fully covered by the rows below:

| `get_bits` path | reached by |
|---|---|
| early reject (`pos+n > limit`) | rows 12–16, 20, 23 |
| accept, `n + s <= 8` (loop body never runs) | `ba` ∈ {1..8} with aligned `pos` (rows 3, 5) |
| accept, 1 loop iteration (2 bytes) | `ba` ∈ {9..16} (rows 4, 6) |
| accept, ≥2 loop iterations | `ba` = 16 with `s = 7`; `ba ≥ 17` with small `mod` (rows 6, 7, 9) |
| every `s` ∈ 0..=7 | row 8 (sweeps `pos` 0..=15) |
| shift count ≥ 32 (UB masking) | rows 10, 11, 21 |
| `n <= 0` | **not reachable** via the public API: `n` is either `ba` ≥ 1 or `mod+2-(mod>>3)` ≥ 3 (smallest reachable `n` = 1, largest = 1879048195) — documented, no row |

## Rows (pruned cross-product of the axes above)

Every row is run with **many randomized inputs** (fixed seed `0x5EED_1234`,
xorshift64\*), not one hand-picked value: `bitalloc` contents, `buf` bytes, `scf`,
`scfcod` and `stereo_bands` are re-randomized on each of the row's iterations.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|---|---|---|
| 1 | `dequantize_granule` | **empty**: `total_bands=0`, `group_size` ∈ {0,1,4,12,18,32}, random `bitalloc` (must be ignored) | [x] |
| 2 | `dequantize_granule` | **all-skip**: `bitalloc` all `0`, `total_bands` ∈ {1,2,8,32}, `group_size` ∈ {1,4,12,18} — no bits consumed, `choff` still toggles | [x] |
| 3 | `dequantize_granule` + `get_bits` | **single narrow band**: `total_bands=1`, `bitalloc[0..2]` random ∈ `1..=8` (1-byte `get_bits`), `group_size` ∈ {1,2,3,4,12}, `limit=INT_MAX`, aligned `pos=0` | [x] |
| 4 | `dequantize_granule` + `get_bits` | **single wide band**: `total_bands=1`, `bitalloc` ∈ `9..=16` (2-byte `get_bits`), `group_size` ∈ {1,4,12,18} | [x] |
| 5 | `dequantize_granule` | **many low bands**: `total_bands` ∈ {2,8,31,32}, all `bitalloc` random ∈ `1..=16`, `group_size` ∈ {1,4,12}, `limit=INT_MAX` | [x] |
| 6 | `dequantize_granule` | **unaligned start**: as row 5 with `bs.pos` random ∈ `1..=7` and ∈ `8..=63` (all 8 `s` values) | [x] |
| 7 | `dequantize_granule` | **grouped/high bands only**: all `bitalloc` random ∈ `17..=24` (`mod` ∈ {3,5,9,…,255}), `total_bands` ∈ {1,2,8}, `group_size` ∈ {1,3,12} | [x] |
| 8 | `dequantize_granule` | **`s` sweep**: `bs.pos` = 0..=15 exhaustively × random low+high `bitalloc`, `total_bands=4`, `group_size=12` | [x] |
| 9 | `dequantize_granule` | **mixed low+high+skip**: `bitalloc` random over `{0} ∪ 1..=16 ∪ 17..=32`, `total_bands` ∈ {2,8,32}, `group_size` ∈ {1,4,12,18} | [x] |
| 10 | `dequantize_granule` | **full opcode range**: `bitalloc` fully random `0..=255` (incl. `mod` overflow / `n ≥ 32`), `total_bands` ∈ {1,8,32}, `group_size` ∈ {1,4,12}; each case also run with `limit=0` (always observable) | [x] |
| 11 | `dequantize_granule` | **`mod` boundary sweep**: `bitalloc[0]` = each of `16,17,18,31,32,33,47,48,49,255` × `group_size` ∈ {1,2,12}, `total_bands=1` | [x] |
| 12 | `dequantize_granule` + `get_bits` | **exact-fit reservoir**: `limit` = exactly the number of bits the granule consumes (no rejection), random low `bitalloc` | [x] |
| 13 | `dequantize_granule` + `get_bits` | **one bit short**: `limit` = exact-fit − 1 (last field rejects) | [x] |
| 14 | `dequantize_granule` + `get_bits` | **mid-stream exhaustion**: `limit` = random fraction of the exact-fit bit count | [x] |
| 15 | `dequantize_granule` + `get_bits` | **`limit=0`** with nonzero `bitalloc` — every `get_bits` rejects, `pos` still walks | [x] |
| 16 | `dequantize_granule` + `get_bits` | **`limit<0`** (`-1`, `INT_MIN`) — every `get_bits` rejects | [x] |
| 17 | `dequantize_granule` | **buffer content shapes**: `buf` = all `0x00` / all `0xFF` / `0xAA55` / random, × low & high `bitalloc` | [x] |
| 18 | `dequantize_granule` | **`group_size` shape sweep**: `group_size` ∈ {1,2,3,4,5,8,12,16,18,32} × random mixed `bitalloc`, `total_bands=8` | [x] |
| 19 | `dequantize_granule` | **negative `group_size`** (`-1`, `-7`, `INT_MIN`): no writes, but `ba>=17` bands still consume one `get_bits` each | [x] |
| 20 | `dequantize_granule` | **OOB `bitalloc` index**: `total_bands` ∈ {33,64,255} ⇒ `i` up to 509 reads `scfcod` and past the struct (tail bytes randomized) | [x] |
| 21 | `dequantize_granule` + `get_bits` | **`pos` overflow** (two observable constructions, `ba` swept 1..=16): (a) `pos=INT_MAX`, `limit=INT_MAX+n` (wrapped negative) — call #1 wraps and is accepted, #2..#4 reject; (b) `pos=INT_MAX-3n`, `limit=INT_MAX` — the *last* call wraps and is accepted, so nothing is ever rejected despite `pos > limit` | [x] |
| 22 | `dequantize_granule` | **negative `pos`**: `bs.pos` ∈ {−1,−2,−7,−8,−9,−64,−1000,−32768} (`p = buf + (pos>>3)` reads before `buf`; `buf` is re-based into a padded arena) | [x] |
| 23 | `dequantize_granule` | **huge `group_size`** (`0x40000000`, `INT_MAX`, `0x7FFFFFF0`) with `total_bands=0` ⇒ return-value overflow only, no writes | [x] |
| 24 | `dequantize_granule` | **`choff` drift / full walk**: `total_bands=255`, all `bitalloc` ∈ `1..=16`, `group_size` ∈ {1,4}, `limit=INT_MAX` — exercises the whole `576 ⇄ -558` walk and its OOB writes | [x] |
| 25 | `dequantize_granule` | **grand fuzz**: everything (all 4 args, both structs, `pos`, `limit`, `buf`) randomized together, 3000 iterations, each also re-run with `limit=INT_MIN` (always observable) | [x] |

All 25 rows are covered by `tests/differential.rs` (`phase_b_*` tests).

## Observability policy

`get_bits`'s accept/reject decisions — and hence the exact set of byte offsets
it dereferences — depend only on `(bitalloc, total_bands, group_size, bs->pos,
bs->limit)` and **never** on the reservoir contents. `reach()` in
`tests/differential.rs` models that walk exactly, so the harness knows in advance
every address both libraries will touch.

Rows 10, 11, 21 and 25 can generate `bitalloc` opcodes whose bit demand (`n` up
to 1 879 048 195) makes `bs->pos` wrap through `INT_MAX` on *rejected* reads;
once `bs->pos` is negative the `> limit` check starts passing again and C reads
from `buf + (pos>>3)`, hundreds of megabytes away. That is a genuine C segfault,
not a translation difference, and it cannot be compared in-process — so those
cases are **skipped and counted**, and every test asserts a minimum number of
*observable* cases actually ran. Observed skip rates (rest of the suite: 0):

| row | ran | skipped |
|---|---|---|
| 10 (full opcode range) | 216 | 324 |
| 11 (mod boundaries) | 675 | 75 |
| 25 (grand fuzz) | 5533 | 467 |
| E24 (0..=255 sweep) | 1666 | 126 |

Rows 10, 25 and E24 additionally run an **always-observable** `limit = 0` /
`limit = INT_MIN` companion of every case: every read is then rejected, so no
memory is touched at all, yet the full `mod` / `n` arithmetic and the wrapping
`bs->pos` walk are still compared for the whole `0..=255` opcode domain.
`ERRORS.md § In-process observability` maps each skipped mechanism to the
dedicated observable case that covers it.

## Appendix — mutation testing (does the suite have teeth?)

20 hand-written mutants were injected into `src/lib.rs`, rebuilt, and run against
the suite. **14 killed, 6 survivors, and every survivor is provably equivalent
or unobservable by construction** (proofs run as part of the review, see below):

| mutant | result |
|---|---|
| `bitalloc[i]` clamped to `i & 63` (the bug this task actually found) | **KILLED** (5 tests) |
| limit check `>` → `>=` | **KILLED** (9) |
| `bs->pos` advanced only on success | **KILLED** (20) |
| `half = 1<<(ba-1)` (drop the `-1`) | **KILLED** (34) |
| branch boundary `ba < 17` → `ba <= 17` | **KILLED** (19) |
| `2 << (ba-17)` shift count not masked | **KILLED** (7) |
| `n = mod + 2` (drop `- (mod>>3)`) | **KILLED** (21) |
| `code /= mod` before use instead of after | **KILLED** (17) |
| first partial byte not masked by `255 >> s` | **KILLED** (32) |
| `next << shl` shift count not masked | **KILLED** (16) |
| `choff = -choff` instead of `18 - choff` | **KILLED** (24) |
| `group_size * 4` saturating instead of wrapping | **KILLED** (5) |
| band loop bound `2*total_bands` → `total_bands` | **KILLED** (29) |
| `ba>=17` `get_bits` moved inside the `k` loop | **KILLED** (23) |
| `pos >> 3` logical instead of arithmetic | **KILLED** (SIGSEGV) |
| `choff` re-initialised inside the `j` loop | survives — **equivalent**: `2*total_bands` is always even, so `choff` returns to 576 at the end of every `j` iteration (verified for all `total_bands` 0..=255) |
| `(code%m).wrapping_sub(m/2) as i32` → signed subtract | survives — **equivalent**: wrapping subtraction is the same operation on `u32` and `i32` |
| `(n as u32).wrapping_add(s) as i32` → signed add | survives — **equivalent**: same for wrapping addition |
| `pos & 7` → `pos.rem_euclid(8)` | survives — **equivalent** for all `i32` |
| final `next >> -shl` not masked | survives — **equivalent on all reachable inputs**: the smallest reachable `n` is 1, so `-shl` is always in `0..=7` and the `>= 32` branch is dead |
| `group_size * j` saturating instead of wrapping | survives — **unobservable**: differs only when `group_size*j` overflows `i32`, which needs `|group_size| > INT_MAX/3`; any such case that writes performs a multi-gigabyte OOB write in C too |
