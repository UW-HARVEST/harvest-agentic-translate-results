# CONFIGS.md — configuration-surface table (Phase A, gates Phase B)

Public API surface of `c_src/include/lib.h` is one function:

```c
int pinflate(void *in, int in_bytes, void *out, int out_bytes);
```

but the *configuration* surface is much wider than "one function, one input",
because the C branches on (a) the **address** and length of `in`, (b) the DEFLATE
stream contents, (c) the **writable** exported globals, and (d) the size/address
of `out`. The axes below were extracted from the `if` / `switch` / `case` / `?:`
sites in `c_src/src/lib.c`; there are no `#ifdef`s and no `[features]` in
`translation/Cargo.toml`, so there is exactly one build configuration.

## Axes (mechanically derived)

| axis | source site | values the C distinguishes |
|------|-------------|-----------------------------|
| `A_align` — alignment of `in` | `first_bytes = ((in+3) & ~3) - in` (lib.c:317) and the `for (i < first_bytes)` prefix loop | `0, 1, 2, 3` |
| `A_tail` — `last_bytes` | `last_bytes = (in_bytes - first_bytes) & 3`; `final_word_available = last_bytes ? 1 : 0` (lib.c:320,325) | `0, 1, 2, 3` |
| `A_words` — `word_count` | `cp_peak_bits`: `if (word_index < word_count) … else if (final_word_available)` (lib.c:101-110) | `0` (final-word path only), `1`, `>1` |
| `A_btype` | `switch (btype)` (lib.c:336) | `0` stored, `1` fixed, `2` dynamic, (`3` → ERRORS.md E6) |
| `A_bfinal` | `do … while (!bfinal)` (lib.c:334-371) | single final block, 2 blocks, 3+ blocks |
| `A_pad` — stored pre-align | `cp_read_bits(s, s->count & 7)` (lib.c:172) | `count & 7` = 0 vs ≠ 0 |
| `A_len` — stored `LEN` | `memcpy(s->out, p, LEN)` (lib.c:193) | `0`, `1`, `2`, `3`, `4`, small, `>4 KiB` |
| `A_symclass` — fixed-code class | `cp_fixed_table` runs: 144×8, 112×9, 24×7, 8×8, 32×5 (lib.c:41-53) + `if (s && len <= 9)` in `cp_build` (lib.c:158) | literals `0..143` (8-bit), `144..255` (9-bit), EOB `256` (7-bit), length syms `257..279` (7-bit), `280..287` (8-bit) |
| `A_lensym` — length symbol | `cp_len_extra_bits[symbol]` (lib.c:271) | extra-bit classes `0,1,2,3,4,5` → syms 257-264, 265-268, 269-272, 273-276, 277-280, 281-284, and the special sym `285` (`len_base` 258, 0 extra) |
| `A_distsym` — distance symbol | `cp_dist_extra_bits[distance_symbol]` (lib.c:275) | extra-bit classes `0..13` → syms 0-3, 4-5, …, 28-29 |
| `A_copy` — copy strategy | `switch (backwards_distance) { case 1: memset …; default: byte loop }` (lib.c:299-306) | `distance == 1` (RLE/`memset`), `distance < length` (self-overlapping byte loop), `distance >= length` (disjoint byte loop) |
| `A_hlit` | `nlit = 257 + read_bits(5)` (lib.c:221) | `257`, `258`, mid, `288` |
| `A_hdist` | `ndst = 1 + read_bits(5)` (lib.c:222) | `1`, `2`, mid, `32` |
| `A_hclen` | `nlen = 4 + read_bits(4)` (lib.c:223) | `4`, mid, `19` |
| `A_clsym` — code-length opcode | `switch (sym) { case 16, 17, 18, default }` (lib.c:229-244) | literal `0..15`, `16` (repeat prev, 3..6), `17` (zeros 3..10), `18` (zeros 11..138) |
| `A_codelen` — max code length | `if (s && len <= 9)` fast `lookup` fill vs. not (lib.c:158) | all lengths ≤ 9, some length in `10..15` |
| `A_tree` — which `cp_build` call | `cp_build(s, …)` for `lit` vs `cp_build(0, …)` for `dst`/`len` (lib.c:200-201, 226, 248-249) | `s != NULL` (fills `lookup`), `s == NULL` |
| `A_out` — output buffer | `out_end = out + out_bytes`; `out + 1 <= out_end`; `out + length <= out_end` (lib.c:330,258,287) | exact fit, 1 byte spare, much larger |
| `A_globals` — writable exports | `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits/base`, `cp_dist_extra_bits/base` are all non-`const` exported arrays that the decoder reads on every call | pristine, or consumer-mutated before the call |

## Rows — the pruned cross-product

Every row is driven through the **only** public entry point with the real
end-to-end pipeline (`pinflate` → `cp_read_bits`/`cp_peak_bits`/`cp_consume_bits`
→ `cp_stored`/`cp_fixed`/`cp_dynamic` → `cp_build` → `cp_decode` → `cp_block`),
i.e. the low-level bit reader, the Huffman builder and the Huffman decoder are
all exercised in situ rather than through a convenience wrapper (there is none).
Each row is run with **many randomized inputs** (fixed seed `0x5EED_1234`,
`SplitMix64`) and compared byte-for-byte between the C `.so` and the Rust `.so`,
including the returned `int`, the *whole* output buffer, `cp_error_reason`'s
string, and the process exit status/signal.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1  | `pinflate` | `A_btype=1` fixed, literals `0..143` only (8-bit codes), `A_align=0`, `A_out`=exact | [x] |
| C2  | `pinflate` | `A_btype=1` fixed, literals `144..255` only (9-bit codes) | [x] |
| C3  | `pinflate` | `A_btype=1` fixed, literals spanning both classes, random bytes | [x] |
| C4  | `pinflate` | `A_btype=1` × `A_align=0,1,2,3` (all four `in` alignments), same payload | [x] |
| C5  | `pinflate` | `A_btype=1` × `A_tail=0,1,2,3` (all four `last_bytes`, `final_word_available` both values) | [x] |
| C6  | `pinflate` | `A_btype=1` × `A_words=0` (whole stream inside prefix + final word, `word_count==0`) | [x] |
| C7  | `pinflate` | `A_btype=1` × `A_words=1` and `A_words>1` (multi-word refill loop in `cp_peak_bits`) | [x] |
| C8  | `pinflate` | `A_btype=1`, `A_lensym` extra-bit class 0 (len 3..10, syms 257-264) × `A_copy` distance ≥ length | [x] |
| C9  | `pinflate` | `A_btype=1`, `A_lensym` classes 1..5 (syms 265..284, 1..5 extra bits), randomized extra-bit values | [x] |
| C10 | `pinflate` | `A_btype=1`, `A_lensym=285` (`len_base` 258, 0 extra bits — the max-length special case) | [x] |
| C11 | `pinflate` | `A_btype=1`, `A_copy` `distance == 1` → `memset` RLE branch, lengths 3..258 | [x] |
| C12 | `pinflate` | `A_btype=1`, `A_copy` `1 < distance < length` → self-overlapping byte-copy branch | [x] |
| C13 | `pinflate` | `A_btype=1`, `A_distsym` extra-bit classes 0..7 (dist 1..256) | [x] |
| C14 | `pinflate` | `A_btype=1`, `A_distsym` extra-bit classes 8..13 (dist 257..32768), output > 32 KiB | [x] |
| C15 | `pinflate` | `A_btype=1`, symbol `280..287` (the 8-bit length-symbol tail of `cp_fixed_table`) | [x] |
| C16 | `pinflate` | `A_btype=0` stored, `A_len=0` (empty stored block), final | [x] |
| C17 | `pinflate` | `A_btype=0` stored, `A_len=1,2,3,4` (all `last_bytes` residues of the payload) | [x] |
| C18 | `pinflate` | `A_btype=0` stored, `A_len` large (> 4 KiB, spans many words) | [x] |
| C19 | `pinflate` | `A_btype=0` stored × `A_align=0,1,2,3` (`cp_ptr` arithmetic vs. prefix bytes) | [x] |
| C20 | `pinflate` | `A_btype=0` stored, non-final (`bfinal=0`): the stored payload is then re-parsed as the next block header (C never consumes it from the bit reader) | [x] |
| C21 | `pinflate` | `A_btype=2` dynamic, `A_hlit=257` minimum, `A_hdist=1` minimum, `A_hclen=4` minimum | [x] |
| C22 | `pinflate` | `A_btype=2` dynamic, `A_hlit=288`, `A_hdist=32`, `A_hclen=19` (all maxima) | [x] |
| C23 | `pinflate` | `A_btype=2` dynamic, `A_clsym=16` (repeat-previous, run lengths 3..6) present in the CL stream | [x] |
| C24 | `pinflate` | `A_btype=2` dynamic, `A_clsym=17` (short zero run, 3..10) present | [x] |
| C25 | `pinflate` | `A_btype=2` dynamic, `A_clsym=18` (long zero run, 11..138) present | [x] |
| C26 | `pinflate` | `A_btype=2` dynamic, `A_codelen` all ≤ 9 (`cp_build` `lookup` fast path fully filled) | [x] |
| C27 | `pinflate` | `A_btype=2` dynamic, `A_codelen` with lengths in `10..15` (skips the `len <= 9` `lookup` fill; binary search only) | [x] |
| C28 | `pinflate` | `A_btype=2` dynamic with matches → `A_tree` `cp_build(0, dst, …)` path with `ndst ≥ 2` | [x] |
| C29 | `pinflate` | `A_btype=2` dynamic, literal-only (no matches, distance tree degenerate/unused) | [x] |
| C30 | `pinflate` | `A_btype=2` dynamic × `A_align=0..3` × `A_tail=0..3` | [x] |
| C31 | `pinflate` | `A_btype=2` dynamic, `A_hclen` mid values so `cp_permutation_order` is only partially applied (unlisted CL symbols stay 0) | [x] |
| C32 | `pinflate` | `A_bfinal`: 2 blocks, `btype` pair `(1,1)` | [x] |
| C33 | `pinflate` | `A_bfinal`: 2 blocks, `btype` pair `(1,2)` and `(2,1)` | [x] |
| C34 | `pinflate` | `A_bfinal`: 3+ blocks, random `btype ∈ {1,2}` per block, so `cp_build` re-runs and overwrites `lit`/`dst`/`lookup` between blocks | [x] |
| C35 | `pinflate` | `A_bfinal`: block containing *only* the end-of-block symbol (empty output), single and repeated | [x] |
| C36 | `pinflate` | `A_out`: `out_bytes` exactly the decoded size (tightest accepting case for both `out+1<=out_end` and `out+length<=out_end`) | [x] |
| C37 | `pinflate` | `A_out`: `out_bytes` much larger than needed (trailing bytes of `out` must stay untouched — verified over the whole buffer) | [x] |
| C38 | `pinflate` | `A_globals`: consumer permutes `cp_dist_base` / `cp_len_base` to still-valid values before the call (both libs must read their own writable copy identically) | [x] |
| C39 | `pinflate` | `A_globals`: consumer rewrites `cp_permutation_order` to a different valid permutation before a `btype=2` block | [x] |
| C40 | `pinflate` | `A_globals`: consumer rewrites `cp_fixed_table` to another *complete* 288/32 length assignment before a `btype=1` block | [x] |
| C41 | `pinflate` | Randomized fuzz over well-formed streams: random block counts, random `btype ∈ {0,1,2}`, random payloads, random alignments/tails, random `out_bytes ≥ needed` — 4000+ cases, fixed seed | [x] |
| C42 | `pinflate` | Randomized fuzz over *arbitrary* byte strings (mostly malformed) at all alignments — asserts that C and Rust agree on return value, output bytes, `cp_error_reason` **and** exit signal (`SIGABRT`/`SIGSEGV`) | [x] |

## Where each row lives, and what "passed" required

| rows | test file | call pairs | notes |
|------|-----------|-----------:|-------|
| C1-C15  | `tests/phase_b_fixed.rs`           | 1 153 | every fixed-code class, all 29 length symbols x extra-bit values, all 30 distance symbols, both copy strategies |
| C16-C31 | `tests/phase_b_stored_dynamic.rs`  |   628 | stored framing at all alignments, dynamic HLIT/HDIST/HCLEN extremes, all three CL run opcodes, code lengths <=9 and 10..15 |
| C32-C41 | `tests/phase_b_multiblock.rs`      |   886 | multi-block sequences, output-buffer shapes, consumer-poked exports, randomized sweep |
| C42     | `tests/fuzz.rs`                    | 19 000 | five mutation strategies |
| (recon) | `tests/recon.rs`                   | 6 000 | maps which of the 16 rejection sites are reachable |

Every Phase B row additionally asserts **non-vacuity**: the C library must
return `1` and produce exactly the bytes the encoder intended. Without that, a
row where both libraries abort identically would silently "pass". Rows C38-C40
also re-run the same stream *without* the poke and require the result to differ,
which proves the Rust reads the exported (writable) table rather than a
compiled-in constant.

### Two rows needed their expectation corrected against the C

* **C17/C19 (stored blocks).** `cp_ptr()` is `words + word_index - count/8`,
  which only points at the payload when whole 32-bit words covered the 5-byte
  block header. With `in_align == 0` that needs `word_count >= 2`, i.e.
  `LEN >= 3`; for `LEN` of 1 or 2 the C copies *header* bytes into `out` and
  still returns `1`. The row asserts that behaviour rather than "correct
  decompression", because the C is the ground truth.
* **C23-C25 (CL opcode 16).** Opcode 16 repeats the *previous* code length, so
  it only appears when two adjacent symbols share a nonzero length -- which a
  sparse alphabet never produces. Dense alphabets with uniform weights were
  added so the row genuinely covers it (the test fails if any of opcodes 16, 17,
  18 is never emitted).

### Not comparable: unbounded-copy paths

1.7 % of the fuzz cases (325 / 19 000) make **both** libraries exceed the 300 ms
per-call watchdog. These are the cases where the `lens[320]` overflow rewrites
`nlit` with code-length bytes, so `cp_build`'s counting loop runs up to 2.5 x
10^8 times over memory past the frame. The C is reading its own stack there, so
no implementation can match it byte for byte; the harness counts and reports
them instead of pretending they passed.
