# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the `if` / `switch` branches in `c_src/src/lib.c`.
Every row is exercised through the `.so` exports of **both** libraries with many
randomized inputs (fixed seed, see `tests/harness/mod.rs::Rng`).

## Axes the C actually branches on

**`unfilter(w, h, bpp, raw)`**

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| row-0 filter byte | `0` (nop), `1` (sub), `2` (**nop — differs from rows ≥1!**), `3` (`raw[x-bpp]/2`, no `prev`), `4` (`paeth(a,0,0)`), `>4` (reject) | `switch (*raw++)` in the `if (h > 0)` block |
| row-`y≥1` filter byte | `0`, `1` (sub, with a `+= 0` pre-loop), `2` (up), `3` (`(raw[x-bpp]+prev[x])/2`), `4` (full paeth), `>4` (reject) | second `switch (*raw++)` |
| `h` | `<0`, `0`, `1` (row-0 formulas only), `2`, `many` | `if (h > 0)`, `for (y=1; y<h; ...)` |
| `w` | `0` (`len==0`), `1`, `many` | `len = w*bpp` |
| `bpp` | `0`, `1`, `2`, `3`, `4`, `8`, `>len`, `<0` | loop bounds `x=bpp`, `x<bpp` |
| data | full `0..=255`, randomized — drives `wrapping_add` and all three paeth tie-breaks (`pa<=pb && pa<=pc`, `pb<=pc`, else) | `cp_paeth` |

**`cp_inflate(in, in_bytes, out, out_bytes)`**

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `btype` | `0` stored, `1` fixed-Huffman, `2` dynamic-Huffman, `3` reject | `switch (btype)` |
| `bfinal` | `1` (single block), `0`+`1` (multi-block `do..while(!bfinal)`) | `do { ... } while (!bfinal)` |
| **input pointer alignment** `first_bytes` | `0`, `1`, `2`, `3` — the C realigns `in` up to a 4-byte boundary and pre-loads the leading bytes into `s->bits` by a *different* code path than the word loop | `first_bytes = ((in+3) & ~3) - in` |
| **input tail** `last_bytes` | `0`, `1`, `2`, `3` — a non-word-multiple tail is loaded via `final_word` / `final_word_available`, again a distinct path | `last_bytes = (in_bytes - first_bytes) & 3` |
| literal symbols | `< 256` only | `if (symbol < 256)` |
| length symbols | `257..264` (0 extra bits), `265..284` (1–5 extra bits), `285` (len 258) | `cp_len_extra_bits`, `cp_len_base` |
| distance symbols | `0..3` (0 extra bits), `4..29` (1–13 extra bits) | `cp_dist_extra_bits`, `cp_dist_base` |
| **copy path** | `backwards_distance == 1` → `memset` fast path; `> 1` → byte-at-a-time loop (**overlapping** copies must replicate byte-by-byte semantics) | `switch (backwards_distance)` |
| dynamic-block shape | `HCLEN` 4..19, code-length symbols `0..15` literal, `16` (copy prev 3–6), `17` (zeros 3–10), `18` (zeros 11–138) | `switch (sym)` in `cp_dynamic` |
| `HLIT`/`HDIST` | `257..288` / `1..32` | `nlit`, `ndst` |
| lookup table | `cp_build(s, ...)` fills `s->lookup` for `len <= 9`; `cp_build(0, ...)` skips it | `if (s && len <= 9)` |
| stored-block `LEN` | `0`, `1`, `many`; must satisfy `bits_left/8 <= LEN` | `cp_stored` |
| output fit | exact fit, slack, one byte short | `out_end` checks |

**Writable exported globals** — `cp_fixed_table`, `cp_len_base`,
`cp_len_extra_bits`, `cp_dist_base`, `cp_dist_extra_bits`,
`cp_permutation_order` are all non-`const` `D`-section symbols, so a caller can
mutate them and change decoding. That is a genuine configuration axis.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `unfilter` | `h=1` (row-0 path only), filter `0`, `w∈{1,7,64}`, `bpp∈{1,2,3,4}`, random data | [x] |
| C2 | `unfilter` | `h=1`, filter `1`, `w∈{1,7,64}`, `bpp∈{1,2,3,4}`, random data | [x] |
| C3 | `unfilter` | `h=1`, filter `2` (row-0 no-op, unlike rows ≥1), same shapes | [x] |
| C4 | `unfilter` | `h=1`, filter `3` (row-0 `raw[x-bpp]/2`), same shapes | [x] |
| C5 | `unfilter` | `h=1`, filter `4` (row-0 `paeth(a,0,0)`), same shapes | [x] |
| C6 | `unfilter` | `h≥2`, row-0 filter `f0∈0..=4` × row-1 filter `f1∈0..=4` (25 combos), `w∈{1,7}`, `bpp∈{1,3,4}`, random data | [x] |
| C7 | `unfilter` | `h∈{2,3,8,17}`, **per-row random** filter bytes `0..=4`, `w∈{1,2,5,16,64}`, `bpp∈{1,2,3,4,8}`, random data — the composed multi-row pipeline | [x] |
| C8 | `unfilter` | `bpp > len` shapes: `(w=1,bpp=1..8)`, `(w=2,bpp=5)`, all filters, `h≥2` (row-`y` `x<bpp` pre-loop writes past `len`) | [x] |
| C9 | `unfilter` | `bpp=0` and `w=0` (`len==0`) with `h∈{0,1,5}`, all filters | [x] |
| C10 | `unfilter` | `h∈{0,-1,-1000}` (loop bodies skipped) | [x] |
| C11 | `unfilter` | `bpp<0` (`bpp=-1,-3`), `w∈{1,4}`, `h∈{1,3}`, filters `0..=4`, buffer with slack before `raw` so negative indices stay in the shared mapping | [x] |
| C12 | `unfilter` | paeth tie-break saturation: data restricted to `{0,1,127,128,254,255}` so `pa==pb`, `pa==pc`, `pb==pc` all occur, filter `4`, `h≥2`, `bpp∈{1,3,4}` | [x] |
| C13 | `cp_inflate` btype=0 | single stored block, `bfinal=1`, `LEN∈{0,1,2,3,4,5,16,255,256,1000}`, random payload, `out_bytes` exact | [x] |
| C14 | `cp_inflate` btype=0 | stored block × input alignment `first_bytes∈{0,1,2,3}` (input placed at `base+k`) — coverage of all four values is asserted, not assumed | [x] |
| C15 | `cp_inflate` btype=0 | stored block × input tail `last_bytes∈{0,1,2,3}` | [x] |
| C16 | `cp_inflate` btype=1 | fixed-Huffman, literals only (symbols `0..=255` random), lengths 1..300, `bfinal=1`, exact `out_bytes` | [x] |
| C17 | `cp_inflate` btype=1 | fixed-Huffman, literals + matches with `distance == 1` (the `memset` path), random lengths `3..=258` | [x] |
| C18 | `cp_inflate` btype=1 | fixed-Huffman, literals + matches with `distance > 1` **overlapping** (`distance < length`) — byte-at-a-time semantics | [x] |
| C19 | `cp_inflate` btype=1 | fixed-Huffman sweeping **every** length symbol `257..=285` × representative distance symbols `0..=29`, all extra-bit widths | [x] |
| C20 | `cp_inflate` btype=1 | fixed-Huffman × input alignment `first_bytes∈{0,1,2,3}` × tail `last_bytes∈{0,1,2,3}` (16 combos) | [x] |
| C21 | `cp_inflate` btype=2 | dynamic-Huffman, `HCLEN` minimal (4), literals only | [x] |
| C22 | `cp_inflate` btype=2 | dynamic-Huffman, code-length symbol `16` (repeat-previous 3–6) present | [x] |
| C23 | `cp_inflate` btype=2 | dynamic-Huffman, code-length symbol `17` (short zero run 3–10) present | [x] |
| C24 | `cp_inflate` btype=2 | dynamic-Huffman, code-length symbol `18` (long zero run 11–138) present | [x] |
| C25 | `cp_inflate` btype=2 | dynamic-Huffman, `HLIT`/`HDIST` swept over `{257,258,270,288}` × `{1,2,17,32}` | [x] |
| C26 | `cp_inflate` btype=2 | dynamic-Huffman with matches (both `distance==1` and overlapping `distance>1`) | [x] |
| C27 | `cp_inflate` btype=2 | dynamic-Huffman × input alignment × tail (16 combos) | [x] |
| C28 | `cp_inflate` multi-block | `bfinal=0` chains: stored→fixed→stored(final), fixed→dynamic(final), dynamic→fixed→fixed(final) — cross-block match history (`s->begin` window spans blocks) | [x] |
| C29 | `cp_inflate` | matches that reach back across a block boundary (distance > bytes emitted in current block) | [x] |
| C30 | `cp_inflate` | output buffer with slack (`out_bytes` > decoded size) — asserts untouched tail bytes match too | [x] |
| C31 | `cp_inflate` | code-length-9 symbols in a dynamic block (exercises the `len <= 9` `s->lookup` fill boundary) and code-length ≥ 10 (skips it) | [x] |
| C32 | globals + `cp_inflate` | mutate exported `cp_fixed_table` (swap two equal-length entries) then run a fixed-Huffman block — decoding must change identically in both | [x] |
| C33 | globals + `cp_inflate` | mutate exported `cp_len_base` / `cp_dist_base` / `cp_len_extra_bits` / `cp_dist_extra_bits` and run a match-bearing fixed block | [x] |
| C34 | globals + `cp_inflate` | mutate exported `cp_permutation_order` (rotate) and run a dynamic block | [x] |
| C35 | `cp_inflate` | fully random byte strings as input (structured-random fuzz, 4000 cases × alignments) — compares return value, `cp_error_reason`, whole output buffer, **and** termination signal | [x] |
| C36 | `unfilter` | fully random `(w,h,bpp)` in a safe range × random data (2000 cases) | [x] |
| C37 | both | `unfilter` applied to the output of `cp_inflate` (the real PNG pipeline composition) | [x] |
| C38a | `cp_inflate` btype=2 | code-length stream engineered so `n` overruns `lens[320]` in fine increments (prefix 180..319 x `case 18` run 11..138), hitting **every** corrupted local: `lenlens`, padding, `sym`, `nlen`, `ndst`, `nlit`, the three inner counters, `n`, `iperm` | [x] |
| C38b | `cp_inflate` btype=2 | the same overflow driven through `case 17` and `case 16` instead, which corrupt with different byte values and use different counter slots | [x] |
| C38c | `cp_inflate` btype=2 | randomized code-length streams over the full `HLIT` x `HDIST` range, biased towards long `case 18` runs (1000 cases) | [x] |
| C38d | `cp_inflate` btype=2 | `case 16` as the *first* code-length symbol, so `lens[-1]` (the top byte of the spilled `s` pointer) is read | [x] |
| C38e | `cp_inflate` btype=2 | overshoot of only 1..18 bytes, i.e. the overflow stays inside `lenlens[]` | [x] |

Rows C38a–C38e exist because `ERRORS.md` section F showed the `lens[288 + 32]`
overflow is *deterministic* at `-O0`, not indeterminate — so it must be matched,
not avoided. They are driven deliberately rather than left to the C35 fuzzers.

No `[features]` section exists in `translation/Cargo.toml`, so there is exactly
one feature configuration (default = no features). Phase D's "every feature
combination" therefore collapses to the single default combination; `verify.sh`
derives this by parsing `Cargo.toml` rather than assuming it, and would enumerate
and run the full power set if features were ever added.

The project builds **no binary executable** (`[lib] crate-type = ["cdylib"]`
only; the C `CMakeLists.txt` declares only `add_library(... SHARED ...)`), so the
"compare C and Rust binary stdout" gate is not applicable.

## Two C quirks the valid-path rows had to accommodate

Both are ground truth and are matched, but they mean "valid input" does not imply
"correct output":

1. **A stored block must be the last thing in the input.** `cp_stored` rejects
   when `bits_left / 8 > LEN`, so any trailing bytes make a stored block fail with
   `"Stored block extends beyond end of input stream."` (row E12).
2. **`cp_ptr` mis-computes the stored-block source address whenever the tail path
   in `cp_peak_bits` has run.** It backs off by `count / 8`, but the tail branch
   adds `s->bits_left` to `count` rather than `last_bytes * 8`, so `count`
   over-counts and the `memcpy` reads from the wrong offset. Consequently rows
   C13/C14/C15 assert the decoded payload only for the word-aligned shapes
   (`(in_bytes - first_bytes) % 4 == 0`, where the tail path never runs) and
   compare differentially otherwise.
