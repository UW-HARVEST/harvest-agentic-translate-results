# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public surface

`c_src/include/lib.h` declares exactly one function:

```c
int pinflate(void *in, int in_bytes, void *out, int out_bytes);
```

There is **no** options struct, no init/reset call and no `#ifdef`-gated
feature.  Everything the library does is selected by the *content* of the input
bit stream and by the *shape* of the two buffers.  In addition, seven **mutable
globals** are exported (`cp_fixed_table`, `cp_permutation_order`,
`cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base`,
`cp_error_reason`) — these are genuine run-time "options" an external caller can
set, and the code reads them on every call, so they are treated as an axis.

There is **no binary/driver target** in `CMakeLists.txt` (only
`add_library(${project_name} SHARED src/lib.c)`), and the Rust crate declares
only `crate-type = ["cdylib"]` with no `[[bin]]`, so the "run the C and Rust
binaries and compare stdout" part of Phase B does not apply — there is no
executable to run.  Its purpose is served instead by `tests/fuzz_sweep.rs`, which
executes each case in a forked child and compares the two libraries' full
per-case transcripts (return value, whole output buffer, `cp_error_reason`,
exported tables, terminating signal and the exact bytes each wrote to stderr).

## Axes the C code branches on

| axis | values the code distinguishes | where |
|------|-------------------------------|-------|
| `BTYPE` | `0` stored, `1` fixed Huffman, `2` dynamic Huffman, `3` reserved(error) | `switch (btype)` lib.c:339 |
| `BFINAL` | last block / more blocks (`do … while (!bfinal)`) → 1 block vs. many | lib.c:337,371 |
| block sequence | mixes of stored+fixed+dynamic in one stream | lib.c:336-371 |
| `in` alignment | `first_bytes = align4(in) - in` ∈ {0,1,2,3}: prologue byte-by-byte load vs. straight word load | lib.c:320-325 |
| `in_bytes` mod 4 | `last_bytes` ∈ {0,1,2,3}: `final_word_available` 0 or 1 → different `cp_peak_bits` tail path | lib.c:323,326-329 |
| `in_bytes` size | tiny (`< 4`, so `word_count == 0` and *everything* comes from `first_bytes`/`final_word`), small, large (many word loads) | lib.c:322 |
| `cp_build` `s` argument | non-NULL (fills the 512-entry `lookup` table *and* the sorted tree) vs. NULL (tree only). Called both ways: `lit` with `s`, `dst`/`len` with `0` | lib.c:139-169, 200-201, 229, 251-252 |
| code length ≤ 9 vs > 9 | `if (s && len <= 9)` — the `lookup` fast path is only populated for short codes; a tree with 10..15-bit codes takes the other branch | lib.c:158 |
| `cp_decode` `lo` result | `lo > 0` (normal) vs `lo == 0` (`tree[-1]`, aliased struct member) | lib.c:208-215 |
| symbol class in `cp_block` | `< 256` literal, `== 256` end-of-block, `> 256` length/distance | lib.c:258,270,307 |
| `backwards_distance` | `== 1` (`memset` fast path) vs `!= 1` (byte copy loop); and overlapping (`distance < length`) vs non-overlapping | lib.c:299-306 |
| length symbol | 257..264 (0 extra bits), 265..284 (1..5 extra bits), 285 (`len_base` 258, 0 extra), 286/287 (`len_base == 0` → length 0) | `cp_len_extra_bits`/`cp_len_base`, lib.c:272-273 |
| distance symbol | 0..3 (0 extra bits), 4..29 (1..13 extra bits), 30/31 (`dist_base == 0` → distance 0 ⇒ `out - 0 >= begin` passes, `memset`-not-taken byte loop over itself) | lib.c:274-277 |
| code-length symbol | 0..15 literal length, `16` copy-previous (+2 bits), `17` short zero run (+3 bits), `18` long zero run (+7 bits) | `switch (sym)` lib.c:233-249 |
| `HLIT`/`HDIST`/`HCLEN` | `nlit` 257..288, `ndst` 1..32, `nlen` 4..19 — boundary values each | lib.c:224-226 |
| stored `LEN` | `0`, `1`, large; and `LEN` vs remaining input (`bits_left/8 <= LEN`) | lib.c:173-193 |
| `out_bytes` | exactly the decoded size, larger, `0`, smaller than needed | lib.c:260,288,332 |
| globals | pristine vs. caller-modified (`cp_error_reason` pre-set, `cp_fixed_table` perturbed) | lib.c:40-70 |

## Rows (combinations tested)

Every row is run with **many randomized inputs** (fixed seed, deterministic
xorshift64* PRNG in `tests/common/mod.rs`).  Rows 1-34 live in `tests/valid.rs`
and run in-process; rows 35-39 live in `tests/fuzz_sweep.rs` and run each case in
a `fork()`ed grandchild so that aborts, segfaults and infinite loops are recorded
and compared instead of being fatal.

Unless stated otherwise a row is additionally crossed with the 16
`(in-alignment × in_bytes mod 4)` combinations by `diff_matrix` /
`diff_all_alignments`, which place the same stream at offsets 0..3 of a 4-aligned
allocation and append 0..3 trailing bytes.

What is compared per case: the return value, the **entire** output buffer
(including `OUT_SLACK` = 64 KiB + 64 bytes past `out_bytes`, so the unchecked
`memcpy` in `cp_stored` is covered), the `cp_error_reason` **string contents**,
and the contents of all seven exported tables.

`[x]` = passes across the randomized inputs, for both the release and the debug
Rust cdylib.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pinflate` | BTYPE=0 stored, BFINAL=1, `LEN` = remaining input, random payload, 200 random sizes 0..4096, `out_bytes` exact and with slack | [x] |
| 2 | `pinflate` | BTYPE=0 stored, `LEN == 0` (empty stored block), `out_bytes` 0/1/64 | [x] |
| 3 | `pinflate` | BTYPE=0 stored, `LEN == 1`, payload `0x00/0x01/0x7f/0x80/0xff` | [x] |
| 4 | `pinflate` | BTYPE=0 stored, 64 random sizes × 4 `in` alignments (exercises the `first_bytes` prologue and the `final_word` tail) | [x] |
| 5 | `pinflate` | BTYPE=1 fixed, literals only, 300 random lengths 1..400, plus one 8 KiB block over the full alignment matrix | [x] |
| 6 | `pinflate` | BTYPE=1 fixed, single literal — **all 256 byte values**, covering both the 8-bit (0..143) and 9-bit (144..255) fixed-code classes | [x] |
| 7 | `pinflate` | BTYPE=1 fixed, empty block (immediate end-of-block symbol 256), `out_bytes` 0 and 8 | [x] |
| 8 | `pinflate` | BTYPE=1 fixed, `distance == 1` → `memset` fast path; 120 random lengths plus boundaries 3/4/10/11/257/258 | [x] |
| 9 | `pinflate` | BTYPE=1 fixed, `distance < length` (self-overlapping byte-copy loop), 200 random combinations | [x] |
| 10 | `pinflate` | BTYPE=1 fixed, `distance > length` (non-overlapping copy), 200 random combinations | [x] |
| 11 | `pinflate` | BTYPE=1 fixed, **every** length symbol 257..285 at its minimum *and* maximum extra-bit value (all `cp_len_extra_bits` classes 0..5, plus the 258 special case) | [x] |
| 12 | `pinflate` | BTYPE=1 fixed, **every** distance symbol 0..29 at its minimum *and* maximum extra-bit value (distances 1..32768, 40 KiB of prior output) | [x] |
| 13 | `pinflate` | BTYPE=1 fixed, maximum length 258 at the maximum available distance, full alignment matrix | [x] |
| 14 | `pinflate` | BTYPE=2 dynamic, `HLIT`/`HDIST` minimum (`nlit=257`, `ndst=1`), with both a 1-bit distance code and an *empty* distance tree (`ndst` becomes 0), plus 80 randomized trees | [x] |
| 14b | `pinflate` | BTYPE=2 dynamic, `HCLEN` at its **minimum of 4** (only permutation slots 16/17/18/0 available), all-zero length vector → empty literal tree → `cp_decode`'s `lo == 0` / `tree[-1]` branch. Run in children because it can trip the row-16 assert. | [x] |
| 15 | `pinflate` | BTYPE=2 dynamic, `HLIT`/`HDIST`/`HCLEN` at their **maxima** (`nlit=288`, `ndst=32`, `nlen=19`), 30 randomized literal+match streams | [x] |
| 16 | `pinflate` | BTYPE=2 dynamic, code-length stream using symbol **16** (copy-previous run, +2 extra bits) | [x] |
| 17 | `pinflate` | BTYPE=2 dynamic, code-length stream using symbol **17** (short zero run, +3 extra bits) | [x] |
| 18 | `pinflate` | BTYPE=2 dynamic, code-length stream using symbol **18** (long zero run 11..138, +7 extra bits), plus 60 cases using all three repeat codes at once | [x] |
| 19 | `pinflate` | BTYPE=2 dynamic, tree containing codes **longer than 9 bits** (unary code shape), so `cp_build`'s `len <= 9` `lookup` fast path is skipped | [x] |
| 20 | `pinflate` | BTYPE=2 dynamic, tree whose codes are **all ≤ 9 bits** (full 257-symbol balanced code), so every symbol goes through the `lookup` table | [x] |
| 21 | `pinflate` | BTYPE=2 dynamic, literals only, randomized `nlit` 257..288 and `ndst` 1..32 with an empty distance tree | [x] |
| 22 | `pinflate` | BTYPE=2 dynamic, literals **and** matches (both `cp_decode` trees; `dst` built with `s == NULL`), 120 randomized trees/streams | [x] |
| 23 | `pinflate` | BTYPE=2 dynamic produced by a **real encoder** (`flate2`), compression levels **0..9** × {random, repetitive, constant} data × 12 sizes 0..8192 | [x] |
| 23b | `pinflate` | `flate2` levels 1/6/9 over 70 000-byte repetitive and random inputs, full alignment matrix | [x] |
| 24 | `pinflate` | multi-block: stored → fixed → dynamic, `BFINAL` only on the last (`count` loop iterates). Note `cp_stored` does **not** advance the bit reader past the payload, so a non-final stored block leaves the C mid-stream — the row's point is that both libraries do the *same* thing. | [x] |
| 25 | `pinflate` | multi-block: 2..8 fixed blocks with back-references crossing block boundaries, 80 randomized streams | [x] |
| 26 | `pinflate` | multi-block: 2..5 dynamic blocks with **different** trees, so `cp_build` re-runs and re-zeroes `lookup`; mixes ≤ 9-bit and > 9-bit trees | [x] |
| 27 | `pinflate` | `out_bytes` exactly equal to the decoded size (tightest passing bound for `ERRORS.md` rows 3 and 5) | [x] |
| 28 | `pinflate` | `out_bytes` much larger than needed; the untouched tail of `out` must be byte-identical | [x] |
| 29 | `pinflate` | `in_bytes < 4`, so `word_count == 0` and every bit comes from the `first_bytes` prologue and/or `final_word` | [x] |
| 30 | `pinflate` | large input: 200 000 literals (> 64 KiB of compressed data), all 4 alignments, thousands of word loads | [x] |
| 31 | `pinflate` + globals | `cp_error_reason` pre-set by the caller before a **successful** call: the C never clears it, so it must read back unchanged | [x] |
| 32 | `pinflate` + globals | caller **rotates** `cp_permutation_order` (8 rotations) in both libraries and decodes a stream built with the rotated order — proves the code reads the exported global, not a private copy | [x] |
| 33 | `pinflate` + globals | caller rewrites `cp_len_base[0]` / `cp_dist_base[0]` in both libraries; the decode must follow the modified tables (and the effect is asserted, not just compared) | [x] |
| 34 | `pinflate` + globals | caller replaces `cp_fixed_table` with a **different valid** Huffman length assignment (Kraft-checked) and decodes a BTYPE=1 block built for it | [x] |
| 35 | `pinflate` | fuzz sweep: **6 000** random byte strings, lengths 0..64, all 4 alignments, `out_bytes` 0..256 | [x] |
| 35b | `pinflate` | fuzz sweep: **4 500** cases with a *plausible* block header (BTYPE 0/1/2 forced) followed by random payload, so the decoder proper is reached instead of failing in the first three bits | [x] |
| 36 | `pinflate` | fuzz sweep: **2 500** valid `flate2` streams (all levels, random and repetitive data) with 1..8 random **bit flips** | [x] |
| 36b | `pinflate` | fuzz sweep: **6 171** cases — valid streams **truncated** at every prefix length (0..24 bytes removed) | [x] |
| 36c | `pinflate` | fuzz sweep: **2 000** cases over the *argument* space — `in_bytes` 0 / negative / short / long, `out_bytes` 0 / negative / short / 1 MiB, and `out == NULL` | [x] |
| 36d | `pinflate` + globals | fuzz sweep: **1 200** cases with **poisoned exported tables** (arbitrary `uint8_t` written into `cp_fixed_table` / `cp_len_extra_bits` / `cp_dist_extra_bits`, singly and in combination) — the closest thing this API has to an out-of-range enum value | [x] |
| 37 | `pinflate` | fuzz sweep: **4 000** dynamic headers built specifically to make `cp_dynamic`'s unchecked `lens[]` writes **overshoot** the array and clobber its own stack frame (`HCLEN = 4`, only symbols 0/16/17/18, long runs). 656 of these hang; the row asserts that at least one does, so the code path is provably exercised. | [x] |
| 38 | `pinflate` | fuzz sweep: **4 000** dynamic headers with the **full** 19-symbol code-length alphabet and random symbol/extra-bit sequences, so `cp_build` also receives partially-clobbered length vectors | [x] |
| 39 | `pinflate` | fuzz sweep: **2 000** stored blocks whose `LEN` **exceeds** `out_bytes`, i.e. the unchecked `memcpy` in `cp_stored`; both libraries must write the same bytes up to 64 KiB past the caller's buffer | [x] |

Sweep totals (rows 35-39): **32 371** cases — 14 541 return normally, 16 726 die
by signal (with byte-identical `assert()` text and the same signal number) and
1 104 hang identically.  **Zero divergences.**

Per-row observed outcomes (from the test output; `asserts by line` counts which
`assert()` in `lib.c` fired):

```
[cfg35]   6000 cases: 4124 returned, 1867 signal,    9 hung; {115:189, 125:294, 127:44,  217:1340}
[cfg35b]  4500 cases: 2830 returned, 1666 signal,    4 hung; {115:205, 125:145, 127:48,  217:1268}
[cfg36]   2500 cases: 1583 returned,  867 signal,   50 hung; {115:31,  125:23,           217:813}
[cfg36b]  6171 cases: 2196 returned, 3975 signal,    0 hung; {115:2971,125:843, 127:161}
[cfg36c]  2000 cases: 1147 returned,  853 signal,    0 hung; {115:81,  125:692}
[cfg36d]  1200 cases:  626 returned,  574 signal,    0 hung; {115:1,   123:16,  154:554, 217:3}
[cfg37]   4000 cases:    0 returned, 3344 signal,  656 hung; {115:320, 125:11,  127:7,   217:3006}
[cfg38]   4000 cases:   35 returned, 3580 signal,  385 hung; {115:1536,125:49,  127:15,  217:1980}
[cfg39]   2000 cases: 2000 returned,    0 signal,    0 hung; {}
```

## Symbol parity (`tests/symbols.rs`)

| # | check | [x] |
|---|-------|-----|
| S1 | `nm -D --defined-only`: every C symbol is exported by the Rust `.so`, and the list is exactly the eight documented in `SYMBOLS.md` | [x] |
| S2 | every symbol also resolves through `dlopen`/`dlsym` in **both** libraries | [x] |
| S3 | the Rust `.so` has no undefined symbols outside libc / the Rust runtime | [x] |
| S4 | `nm -D -S`: the exported data symbols have the **C sizes** (320, 19, 31, 124, 32, 128, 8) in both libraries | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`** section and has no optional
dependencies, so `--no-default-features` is identical to the default build.
`./check_features.sh` derives the feature list from `Cargo.toml` mechanically
(so it keeps working if features are ever added), then builds and runs the whole
suite for:

1. `<default>`
2. `--no-default-features`
3. the powerset of any declared features (currently empty)
4. **the debug-profile cdylib** — an extra axis, because the debug profile turns
   on Rust's integer-overflow checks and `debug_assert!`s and is therefore a
   different code path through the same source.  All 73 tests pass there too,
   which rules out the translation being accidentally correct only because
   release mode wraps silently.  (One documented exception: `out == NULL` with a
   dereferencing stream — see `ERRORS.md` row G9.)

Each configuration re-runs `./check_symbols.sh` as well, so the symbol diff is
checked per configuration and not just once.

Result: **ALL CONFIGURATIONS PASSED.**
