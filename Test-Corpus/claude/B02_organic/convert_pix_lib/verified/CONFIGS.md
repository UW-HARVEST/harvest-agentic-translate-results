# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Mechanically derived from the axes `c_src/src/lib.c` actually branches on.
Every row is a group in `tests/diff.rs`; run them with

```sh
cargo test --test diff                 # everything (~4 min)
DIFF_ONLY=cfg   cargo test --test diff # just the valid-path rows of this file
./run_all_configs.sh                   # every build configuration
```

Each group loads **both** `.so` files through `libloading`, runs every case in a
forked grandchild (so aborts / segfaults / non-termination are observable per
case), and requires the C and the Rust results to be identical in

* the value `cp_inflate` returned,
* every byte of the output arena (including the bytes the C code overruns into),
* the NUL-terminated `cp_error_reason` string,
* a hash of the whole input arena (nothing may be written back through `in`),
* the process termination (exit code, fatal signal, or SIGALRM for a
  non-terminating input).

## Build-time axes

`Cargo.toml` has **no `[features]`** (`cargo metadata` → `"features": {}`) and
`c_src/` contains **no `#if`/`#ifdef`/`#define`** and no CMake `option()`.
⇒ the feature power-set has exactly one element (the empty set), so
`--no-default-features` ≡ the default build.  The suite is additionally run in
the `release` profile, because that is where Rust drops the debug
integer-overflow checks that C never had.

| configuration | result |
|---------------|--------|
| `cargo test --test diff` (dev, default features) | 84 groups / **8015 cases** / 626 identical non-zero terminations / **0 failures** |
| `cargo test --no-default-features --test diff` | 84 groups / **8015 cases** / **0 failures** |
| `cargo test --release --test diff` | 84 groups / **8015 cases** / **0 failures** |

## Runtime axes the C code branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| entry point | `cp_inflate`, `convert_pix` | the only two `T` symbols |
| exported writable tables | `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base` — all `D` (mutable) and all read *live* on every call | lib.c:35–64, 194–195, 222, 267–271 |
| `cp_error_reason` | `B` symbol, written by 6 error sites, never cleared | lib.c:34 |
| `btype` | `0` stored, `1` fixed, `2` dynamic, `3` reserved | lib.c:333 `switch` |
| `bfinal` | `0` → loop again, `1` → stop (single- and multi-block streams) | lib.c:330/365 `do..while(!bfinal)` |
| `in` pointer alignment | `(uintptr_t)in & 3 ∈ {0,1,2,3}` → `first_bytes ∈ {0,3,2,1}` (pre-word byte fold-in loop) | lib.c:314–319 |
| `in_bytes` mod 4 | `last_bytes ∈ {0,1,2,3}` → `final_word_available` 0/1 and partial-word fold-in | lib.c:317, 320–323, 99–104 |
| `in_bytes` size class | `< first_bytes` (negative `word_count` numerator), `0`, `1..3` (`word_count==0`), `4..7` (1 word), `>= 8` (multi-word), negative, `INT_MIN`, values where `in_bytes*8` overflows | lib.c:313–317 |
| `cp_peak_bits` source | full 32-bit word branch vs. partial `final_word` branch vs. no refill | lib.c:93–105 |
| literal vs match | `symbol < 256`, `symbol == 256` (EOB), `symbol > 256` | lib.c:252/264/301 |
| length symbol class | `257..264` (0 extra bits), `265..268` (1), `269..272` (2), `273..276` (3), `277..280` (4), `281..284` (5), `285` (0 extra, len 258), `286/287` (`len_base==0`) | `cp_len_extra_bits`, `cp_len_base` |
| distance symbol class | `0..3` (0 extra), `4..5` (1) … `28..29` (13), `30/31` (`dist_base==0`) | `cp_dist_extra_bits`, `cp_dist_base` |
| copy strategy | `backwards_distance == 1` → `memset`; otherwise byte-at-a-time forward copy (so `length > distance` overlaps and self-replicates) | lib.c:293–300 |
| dynamic header | `HLIT ∈ 0..31` → `nlit 257..288`; `HDIST ∈ 0..31` → `ndst 1..32`; `HCLEN ∈ 0..15` → `nlen 4..19` | lib.c:218–220 |
| code-length symbols | `0..15` literal length, `16` repeat-previous `3..6`, `17` zero-run `3..10`, `18` zero-run `11..138` | lib.c:227–243 |
| `cp_build` lookup fill | `s != NULL && len <= 9` fills `s->lookup` (memset + fill), `s == NULL` skips | lib.c:143–158 |
| stored block | `LEN == 0`, `1`, small, large; must be the last block and exactly consume the input (`bits_left/8 == LEN`) | lib.c:164–192 |
| `out_bytes` | exact fit, slack, `0`, negative, `INT_MIN`; plus the *unchecked* `memcpy` in `cp_stored` that writes past `out_end` | lib.c:187, 254, 282 |
| `convert_pix` `bpp` | `1` (grey), `2` (grey+alpha), `3` (RGB), `4` (RGBA), anything else (no store, `src` still advances) | lib.c:477–490 |
| `convert_pix` `w`,`h` | `0`, `1`, `2`, many, negative, `INT_MIN`, `INT_MAX` | lib.c:474–476 |

## Row table

Every row uses many randomised inputs from a fixed-seed xorshift PRNG.

| #  | entry point(s) | configuration (options set + input shape) | test group | cases | ✔ |
|----|----------------|-------------------------------------------|------------|-------|---|
| 1  | exported data symbols | read all six tables + `cp_error_reason` right after `dlopen` and compare every byte | `cfg01_table_contents` | 1 | [x] |
| 2  | `convert_pix` | `bpp=1`, `(w,h)` ∈ {(1,1),(1,7),(7,1),(3,5),(17,13),(64,3)}, random `src` | `cfg02_convert_pix_bpp1` | 256 | [x] |
| 3  | `convert_pix` | `bpp=2`, same shape sweep | `cfg03_convert_pix_bpp2` | 256 | [x] |
| 4  | `convert_pix` | `bpp=3`, same shape sweep | `cfg04_convert_pix_bpp3` | 256 | [x] |
| 5  | `convert_pix` | `bpp=4`, same shape sweep | `cfg05_convert_pix_bpp4` | 256 | [x] |
| 6  | `convert_pix` | `bpp ∈ {0,5,6,255,-1,-4,INT_MIN,INT_MAX}` (no `switch` arm) → `dst` untouched, `src` walk must not fault | `cfg06_convert_pix_bpp_out_of_range` | 32 | [x] |
| 7  | `convert_pix` | degenerate shapes `w=0`, `h=0`, `w<0`, `h<0`, `INT_MIN`, `(INT_MAX,0)`, `(0,10^6)`, `(0,2·10^7)`, and NULL `src`/`dst` | `cfg07_convert_pix_degenerate` | 84 | [x] |
| 8  | `cp_inflate` / stored | `btype=0`, `bfinal=1`, `LEN=0`, `out_bytes ∈ {0,16}` | `cfg08_stored_len0` | 2 | [x] |
| 9  | `cp_inflate` / stored | `LEN ∈ {1,2,3,4,7,8,31,255,4096}`, random payload, `out_bytes` exact | `cfg09_stored_lengths` | 36 | [x] |
| 10 | `cp_inflate` / stored | alignment `(in&3) ∈ {0,1,2,3}` (i.e. `first_bytes` 0/3/2/1) × `LEN ∈ {0..9,100,1000}` | `cfg10_stored_alignment` | 48 | [x] |
| 11 | `cp_inflate` / stored | `out_bytes > LEN` (slack), arena pre-filled with a pattern | `cfg11_stored_out_slack` | 5 | [x] |
| 12 | `cp_inflate` / stored | `out_bytes < LEN` — the C has **no** bounds check on this `memcpy`, so both must overrun identically (`LEN` up to 65535) | `cfg12_stored_memcpy_overrun` | 4 | [x] |
| 13 | `cp_inflate` / fixed | empty block (EOB only), `out_bytes ∈ {0,1,64}` | `cfg13_fixed_empty` | 3 | [x] |
| 14 | `cp_inflate` / fixed | literals only, both fixed-code length classes (8-bit syms 0..143, 9-bit syms 144..255), strings of 1..300 bytes | `cfg14_fixed_literals` | 512 | [x] |
| 15 | `cp_inflate` / fixed | length symbols `257..264` (0 extra bits) × distances 1..4 | `cfg15_fixed_len257_264` | 32 | [x] |
| 16 | `cp_inflate` / fixed | length symbols `265..284` (1..5 extra bits, random extra values) × **every** distance class `0..29` (0..13 extra bits, distances up to 32768) | `cfg16_fixed_len_dist_classes` | 600 | [x] |
| 17 | `cp_inflate` / fixed | length symbol `285` (258 bytes) × distances 1, 2, 257, 258, 4096, 32768 | `cfg17_fixed_len285` | 6 | [x] |
| 18 | `cp_inflate` / fixed | `backwards_distance == 1` → `memset` branch, every length 3..258 | `cfg18_fixed_memset_dist1` | 256 | [x] |
| 19 | `cp_inflate` / fixed | overlapping match `length > distance` (`distance ∈ {2,3,5}`) → byte-wise self-replication | `cfg19_fixed_overlap` | 18 | [x] |
| 20 | `cp_inflate` / fixed | randomised literal + match token streams (own LZ77 encoder), outputs up to 4 KiB | `cfg20_fixed_random_tokens` | 512 | [x] |
| 21 | `cp_inflate` / fixed | reserved length symbols `286`,`287` and reserved distance symbols `30`,`31` (`*_base == 0` → length/distance 0) | `cfg21_reserved_len_dist_symbols` | 12 | [x] |
| 22 | `cp_inflate` / dynamic | random *complete* Huffman trees, literals only, no run codes, `HCLEN=19` | `cfg22_dynamic_literals` | 256 | [x] |
| 23 | `cp_inflate` / dynamic | code-length symbol `16` (repeat previous, 3..6) exercised, always with `n > 0` | `cfg23_dynamic_clsym16` | 64 | [x] |
| 24 | `cp_inflate` / dynamic | code-length symbol `17` (zero run 3..10) exercised | `cfg24_dynamic_clsym17` | 64 | [x] |
| 25 | `cp_inflate` / dynamic | code-length symbol `18` (zero run 11..138) exercised | `cfg25_dynamic_clsym18` | 64 | [x] |
| 26 | `cp_inflate` / dynamic | `nlen` sweep 5..19 (i.e. the 4-bit `HCLEN` field 1..15) — every prefix of `cp_permutation_order` that can express a non-empty code-length tree. `nlen == 4` can only express the *empty* tree, which is covered by `abort_dynamic_empty_cl_tree` (row 20 of ERRORS.md). | `cfg26_dynamic_hclen_sweep` | 15 | [x] |
| 27 | `cp_inflate` / dynamic | `nlit ∈ {257,258,270,288}` × `ndst ∈ {1,2,15,32}` | `cfg27_dynamic_hlit_hdist` | 16 | [x] |
| 28 | `cp_inflate` / dynamic | dynamic blocks with matches (multi-symbol distance tree) | `cfg28_dynamic_matches` | 256 | [x] |
| 29 | `cp_inflate` / multi-block | `bfinal=0` chains of 2..4 blocks, random `btype` per block (fixed↔dynamic, run codes on/off) | `cfg29_multiblock` | 256 | [x] |
| 30 | `cp_inflate` / multi-block | non-final **fixed** block followed by a final **stored** block (drives `cp_ptr` on a non-initial block) × 12 literal counts × 7 stored lengths | `cfg30_fixed_then_stored` | 84 | [x] |
| 31 | `cp_inflate` | `in_bytes` size classes 1..7 (`word_count == 0`, only `final_word`), exact, +1..+7 trailing bytes, and ≥ 4 KiB (many words) — crossed with alignment 0..3 | `cfg31_input_size_alignment` | 64 | [x] |
| 32 | `cp_inflate` | 0..23 junk bits after the final EOB — must be ignored identically | `cfg32_trailing_garbage` | 24 | [x] |
| 33 | `cp_inflate` + tables | `cp_fixed_table` replaced by a different **valid** length assignment (256×8 bit + 32×13 bit) in both libraries, then a fixed block encoded with the *new* codes ⇒ proves the table is read live | `cfg33_mutate_fixed_table` | 32 | [x] |
| 34 | `cp_inflate` + tables | `cp_permutation_order` rotated by 5 and the dynamic header emitted in the rotated order | `cfg34_mutate_permutation_order` | 16 | [x] |
| 35 | `cp_inflate` + tables | `cp_len_extra_bits` / `cp_len_base` mutated (including a random full-range `u32` base) then a fixed block with matches | `cfg35_36_mutate_len_dist_tables` | 48 | [x] |
| 36 | `cp_inflate` + tables | `cp_dist_extra_bits` / `cp_dist_base` mutated, same shape | `cfg35_36_mutate_len_dist_tables` | 48 | [x] |
| 37 | `cp_inflate` | property fuzz: valid streams from the test's own deflate encoder (random block count, random `btype`, random literal/match mix, random alignment and out slack) | `fuzz37_valid_streams` | 2000 | [x] |
| 38 | `cp_inflate` | property fuzz: 1..3 random **bit flips** in a valid stream | `fuzz38_mutated_streams` | 600 | [x] |
| 39 | `cp_inflate` | property fuzz: fully random input bytes, `in_bytes ∈ 1..64`, random alignment | `fuzz39_random_bytes` | 800 | [x] |
| 40 | `cp_error_reason` | the string behind the exported pointer is compared byte-for-byte in **every** `cp_inflate` case above, and `ERRORS.md` additionally pins the exact expected literal | all inflate groups | 8000+ | [x] |

## Undefined behaviour in the C source, and how far it is reproduced

The C code has four places where it is formally undefined but where the compiled
library nevertheless has one definite behaviour.  Three of them are reproduced
exactly; the fourth cannot be and is called out here.

1. **`cp_dynamic` writes past the end of `uint8_t lens[288+32]`** — a code-18
   run adds up to 138 entries while the loop bound is at most 320, so the C code
   scribbles over the rest of its own stack frame.  `src/lib.rs` models that
   frame byte-for-byte (offsets read out of `objdump -d` on the C `.so`), which
   makes all three regimes match: overshoot into `lenlens`
   (`ovs_a_into_lenlens`), overshoot that zeroes `sym`/`nlen`/`ndst`/`nlit` and
   collapses the loop bound (`ovs_b_into_locals`), and overshoot that reaches the
   storage of `n` itself, snapping `n` back to 256 so that the inner run loop
   **never terminates** (`ovs_c_nonterminating`, `ovs_d_transition_band`, and 12
   of the 600 `fuzz38` cases).  Before the frame was modelled, `fuzz38` case 100
   diverged as `C = timeout / RUST = SIGABRT`.
2. **`cp_dynamic` reads `lens[-1]`** when code 16 arrives with `n == 0`.  In the
   C frame that byte is the most significant byte of the spilled `s` pointer,
   i.e. `0x00` on x86-64 Linux; the Rust frame model puts the same pointer at the
   same offset, so the read yields the same byte.
3. **`cp_build` indexes `int counts[16]` with a raw code length** (reachable by
   writing ≥ 16 into the exported `cp_fixed_table`).  Lengths 16..63 land on
   `first[]`/`codes[]`/loop counters that are all re-initialised before use, so
   the C is unaffected and `assert(len < 16)` fires either way — verified for
   16, 17, 20, 31, 32, 40, 47, 63, 100 and 255 (`abort_build_len_ge16_*`).
4. **NOT reproduced:** writing a value ≥ 28 into the exported
   `cp_permutation_order` makes the C store outside `uint8_t lenlens[19]` and
   over `cp_dynamic`'s (and eventually `cp_inflate`'s) locals.  The Rust
   translation keeps the store inside a 256-byte buffer, so values 0..27 behave
   identically and 28..255 do not.  Every value ≥ 19 violates the array's
   contract (it is a permutation of 0..18), and reproducing it would require
   modelling the *caller's* frame as well.  Row 34 therefore mutates the table
   only with legal permutations.
5. Also not reproducible in principle: an *observably* out-of-range
   literal/length symbol would make `cp_block` read past the end of
   `cp_len_extra_bits[31]`, whose neighbours differ between the two `.so`
   layouts.  This is unreachable — `cp_decode` can only return `lo - 1 >= 0`
   because `search >= 0xFFFF` and `tree[0] <= (287<<4)|15`, and the empty-tree
   case (`hi == 0`) always aborts in `cp_decode` first
   (`abort_dynamic_empty_cl_tree`).

One purely *temporal* difference is documented rather than papered over: for
`convert_pix(bpp, w<=0, h)` the row loop only performs `src++`, so the `-O0` C
build really executes all `h` iterations while an optimised Rust build deletes
the dead loop.  The observable result (`dst` untouched) is identical; row 7
therefore uses large-but-tractable `h` instead of `INT_MAX`.
