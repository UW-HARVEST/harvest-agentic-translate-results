# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Mechanically derived from the branches the C code actually takes.

## Axes found in `c_src/src/lib.c`

**`unfilter(int w, int h, int bpp, uint8_t *raw)`** (`lib.c:417`)

| axis | values the C code distinguishes | where |
|------|--------------------------------|-------|
| row-0 filter byte | `0`, `1`, `2` (no-op!), `3`, `4`, else → `return 0` | `switch (*raw++)` 422-441 |
| row-`y≥1` filter byte | `0`, `1`, `2`, `3`, `4`, else → `return 0` | `switch (*raw++)` 446-475 |
| `h` | `<= 0` (row-0 block skipped entirely), `== 1` (row loop never runs), `>= 2` | `if (h > 0)` 421, `for (y = 1; y < h; …)` 445 |
| `len = w * bpp` vs `bpp` | `len <= bpp` (only the `x < bpp` prologues run — and for filters 2/3/4 those run **even when `bpp > len`**, writing past the scanline), `len > bpp` | 426/450/457/462/468 |
| `len` | `0` (all inner loops empty), `> 0`, `< 0` (pointer walks backwards) | 418 |
| row-0 vs row-`y` semantics | row 0 starts its loops at `x = bpp` and has no `prev`; filter 2 is a no-op and filter 4 degenerates to `cp_paeth(a,0,0) == a` | 422-441 |
| byte values | wrap-around of `uint8_t +=`; `cp_paeth`'s three-way selection (`pa<=pb&&pa<=pc`, `pb<=pc`, else) | 378-383 |

**`cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`** (`lib.c:314`)

| axis | values the C code distinguishes | where |
|------|--------------------------------|-------|
| `BTYPE` | `0` stored, `1` static, `2` dynamic, `3` error | `switch (btype)` 339-369 |
| `BFINAL` | `0` → loop again (multi-block), `1` → stop | `while (!bfinal)` 371 |
| `((size_t)in) & 3` | `0` → `first_bytes = 0`; `1,2,3` → `first_bytes = 3,2,1` head bytes pre-loaded into `s->bits` | 320-325 |
| `(in_bytes - first_bytes) & 3` | `0` → `final_word_available = 0`; `1,2,3` → partial final word loaded with `count += bits_left` | 323-329 |
| lit/len symbol class | `< 256` literal, `== 256` end-of-block, `> 256` length code (symbol−257 ∈ 0…30, incl. 29/30 whose `cp_len_base` is 0) | 257-308 |
| `backwards_distance` | `== 1` → `memset` fast path; `!= 1` → byte-copy loop (incl. `0` from `cp_dist_base[30/31]`, and overlapping copies) | 299-306 |
| extra-bit counts | `cp_len_extra_bits[0…30]` = 0…5, `cp_dist_extra_bits[0…31]` = 0…13 → `cp_read_bits(s, 0)` vs `cp_read_bits(s, n)` | 273/276 |
| dynamic header | `HLIT` (nlit 257…288), `HDIST` (ndst 1…32), `HCLEN` (nlen 4…19) | 224-228 |
| code-length alphabet | literal 0…15, `16` copy-previous, `17` short zero-run, `18` long zero-run | `switch (sym)` 233-249 |
| `cp_build` caller | `s != NULL` (memsets + fills `s->lookup`, only for the literal tree) vs `s == NULL` | 149/158 |
| code length | `1…9` (lookup table filled) vs `10…14` (no lookup entry); `first[15]` excludes 15-bit codes from `nlit` | 158-164, 167 |
| stored `LEN` | `>= remaining` (required, else E12); `0`,`1`,`2` desynchronise `cp_ptr` through the final-word path; `>= 3` yields the correct source pointer | 170-198 |
| `out_bytes` | exactly enough, more than enough, too small (→ E13/E15) | 260/288 |
| exported globals | `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base` are **mutable** and re-read on every call | 41-70 |

All rows use **many randomized inputs** (deterministic `SplitMix64`, fixed seed
per row) and compare, through `dlsym` on both `.so`s: the return value, the whole
output/scanline buffer byte-for-byte, the `cp_error_reason` C string, the
termination signal and the child's `stderr`. Two extra safety nets:

* the *identical* input buffer is handed to both libraries, so the C code's
  deliberate out-of-bounds head/tail reads observe the same bytes;
* every `cp_inflate` row is additionally checked against an **independent
  transcription of `lib.c`** (`tests/common/cmodel.rs`), and every `unfilter`
  row against an independent reference model in `unfilter_diff.rs`, so a row
  cannot pass because both libraries are equally wrong. (This actually caught a
  misreading of `unfilter`'s `case 2`, whose first loop is bounded by `bpp`, not
  by `len`.)

## `unfilter` configurations — `tests/unfilter_diff.rs`

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| U1 | `unfilter` | `h = 1`, filter `0`, `bpp ∈ 1…8`, `w ∈ 1…17`, random bytes (300 cases) | `u1_h1_filter0` | [x] |
| U2 | `unfilter` | `h = 1`, filter `1` (Sub, starts at `x = bpp`) | `u2_h1_filter1` | [x] |
| U3 | `unfilter` | `h = 1`, filter `2` (Up → C no-op) | `u3_h1_filter2` | [x] |
| U4 | `unfilter` | `h = 1`, filter `3` (`raw[x-bpp]/2`, no `prev`) | `u4_h1_filter3` | [x] |
| U5 | `unfilter` | `h = 1`, filter `4` (`cp_paeth(raw[x-bpp],0,0)`) | `u5_h1_filter4` | [x] |
| U6 | `unfilter` | `h ∈ 2…9`, **every** row filter `0` | `u6_multirow_filter0` | [x] |
| U7 | `unfilter` | `h ∈ 2…9`, every row filter `1` | `u7_multirow_filter1` | [x] |
| U8 | `unfilter` | `h ∈ 2…9`, every row filter `2` | `u8_multirow_filter2` | [x] |
| U9 | `unfilter` | `h ∈ 2…9`, every row filter `3` (avg with `prev`) | `u9_multirow_filter3` | [x] |
| U10 | `unfilter` | `h ∈ 2…9`, every row filter `4` (full Paeth, all 3 branches) | `u10_multirow_filter4` | [x] |
| U11 | `unfilter` | `h ∈ 2…12`, **per-row random** filter from `{0,1,2,3,4}`, random `w ∈ 1…20`, `bpp ∈ 1…8` (600 cases) | `u11_multirow_mixed_filters` | [x] |
| U12 | `unfilter` | `w = 1` ⇒ `len == bpp`: only the `x < bpp` prologues run; `bpp ∈ 1…8` × filters 0…4 × `h ∈ 1…6` | `u12_w1_len_eq_bpp` | [x] |
| U13 | `unfilter` | `w = 0`, `bpp = 0`, both ⇒ `len == 0`; filters 0…4 × `h ∈ 0…6` | `u13_zero_len` | [x] |
| U14 | `unfilter` | extreme byte patterns (all `0x00`, all `0xFF`, all `0x80`, `00/FF` alternating, `01/FE`, `i*37`) ⇒ maximal wrap-around and Paeth ties; filters 0…4 × `bpp ∈ 1…8` × `h ∈ 1…5` | `u14_extreme_byte_patterns` | [x] |
| U15 | `unfilter` | large image `w = 64, h = 48, bpp ∈ {1,2,3,4,6,8}`, per-row random filters | `u15_large_image` | [x] |
| U16 | `unfilter` | `bpp` vs `len` boundaries: `(w,bpp)` ∈ {(1,8),(2,8),(1,1),(2,1),(64,1),(3,5),(5,3),(17,7)} × filters 0…4 × `h ∈ 1…4` | `u16_bpp_vs_len_boundaries` | [x] |
| U17 | `unfilter` | wide randomized sweep: `w ∈ 0…24`, `bpp ∈ 0…8`, `h ∈ 0…10`, per-row filter random over `0…4` with 1-in-6 chance of an **invalid** byte `5…255` (4000 cases; asserted to produce both accepted and rejected results) | `u17_wide_random_sweep` | [x] |

## `cp_inflate` configurations — `tests/inflate_diff.rs`

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| I1 | `cp_inflate` | `BTYPE=0` stored, `BFINAL=1`, `LEN = in_bytes − 5 ≥ 3` (the only stored shape the C accepts), `LEN ∈ 3…2000` random payloads (250 cases, output verified) | `i1_stored_block` | [x] |
| I2 | `cp_inflate` | `BTYPE=0` stored with `LEN ∈ 0…6` — the final-word load desynchronises `cp_ptr()`, so the C reads the payload from the wrong offset (a quirk, compared as-is) | `i2_stored_tiny_len` | [x] |
| I3 | `cp_inflate` | `BTYPE=1` static, literals only, 1…500 random bytes (250 cases) covering the 8- and 9-bit code classes | `i3_fixed_literals` | [x] |
| I4 | `cp_inflate` | `BTYPE=1` static, **empty** block (end-of-block symbol only), `out_bytes ∈ {0,1,16}` | `i4_fixed_empty` | [x] |
| I5 | `cp_inflate` | `BTYPE=1` static, `backwards_distance == 1` (`memset` path), **every** length 3…258 | `i5_fixed_dist1_memset` | [x] |
| I6 | `cp_inflate` | `BTYPE=1` static, `distance > 1`, overlapping (`distance < length`) and non-overlapping, plus a second match on top (400 cases) | `i6_fixed_dist_gt1` | [x] |
| I7 | `cp_inflate` | `BTYPE=1` static, **every** length symbol 257…285 × **every** distance symbol 0…29 with random extra-bit values — all extra-bit widths 0…5 / 0…13 | `i7_fixed_all_len_dist_symbols` | [x] |
| I8 | `cp_inflate` | `BTYPE=1` static, **every** literal 0…255 (8-bit and 9-bit codes) in one block, plus length symbols 280…285 (the trailing 8-bit code block) | `i8_fixed_all_literals` | [x] |
| I9 | `cp_inflate` | `BTYPE=2` dynamic, hand-built canonical trees, literals only, randomized `HLIT`/`HDIST`/alphabet size (200 cases) | `i9_dynamic_literals` | [x] |
| I10 | `cp_inflate` | `BTYPE=2` dynamic whose code-length stream uses symbols `16` **and** `17` **and** `18` (asserted to really occur) | `i10_dynamic_cl_repeats` | [x] |
| I11 | `cp_inflate` | `BTYPE=2` dynamic with distance symbols `30`/`31` ⇒ `cp_dist_base == 0` ⇒ `backwards_distance == 0` (byte-copy loop with `src == dst`) | `i11_dynamic_dist_sym_30_31` | [x] |
| I12 | `cp_inflate` | `BTYPE=2` dynamic with length symbols `286`/`287` ⇒ `cp_len_base == 0` ⇒ `length == 0`, × distance symbols 0/1/3/5 | `i12_dynamic_len_sym_286_287` | [x] |
| I13 | `cp_inflate` | multi-block: 2…4 chained static blocks, `BFINAL` only on the last (150 cases) | `i13_multi_fixed_blocks` | [x] |
| I14 | `cp_inflate` | multi-block mixing `BTYPE`: dynamic → static(final) **and** static → dynamic(final) (120 cases each) | `i14_multi_mixed_blocks` | [x] |
| I15 | `cp_inflate` | dynamic tree of depth `2…14` (lengths `> 9` get **no** `s->lookup` entry), each with `HCLEN` minimal and forced to 19 | `i15_dynamic_deep_tree` | [x] |
| I16 | `cp_inflate` | `HLIT ∈ {257,258,287,288}` × `HDIST ∈ {1,2,31,32}` × both code-length encodings | `i16_dynamic_header_extremes` | [x] |
| I17 | `cp_inflate` | real zlib raw deflate (`flate2`) at level `0` (stored blocks), sizes 0…2048 | `i17_flate2_level0` | [x] |
| I18 | `cp_inflate` | real raw deflate, levels `1…9`, **random** payloads (literal-heavy) of 0…3000 bytes | `i18_flate2_random` | [x] |
| I19 | `cp_inflate` | real raw deflate, levels `1…9`, **highly repetitive** payloads (long runs, periodic patterns ⇒ long matches, distance 1) | `i19_flate2_repetitive` | [x] |
| I20 | `cp_inflate` | real raw deflate, levels `1…9`, **text-like** payloads (small alphabet, mixed matches) | `i20_flate2_textlike` | [x] |
| I21 | `cp_inflate` | input-pointer alignment `in & 3 ∈ {0,1,2,3}` (`first_bytes = 0,3,2,1`) × a 6-stream corpus (static-lit, static-match, dynamic, stored, real zlib, multi-block) | `i21_input_alignment` | [x] |
| I22 | `cp_inflate` | trailing padding `0…7` bytes ⇒ `(in_bytes − first_bytes) & 3` takes all 4 values (`final_word_available`, `count += bits_left`) × all 4 alignments | `i22_input_tail_length` | [x] |
| I23 | `cp_inflate` | `out_bytes` exactly the decompressed size and `+1`, `+7`, `+1000` (the untouched tail is asserted to stay `0xA5`) | `i23_out_bytes_exact_vs_slack` | [x] |
| I24 | `cp_inflate` | 1…33 random garbage bytes after the final block (must be ignored) | `i24_trailing_garbage` | [x] |
| I25 | `cp_inflate` + globals | `cp_len_base[0]` mutated to 3/4/5/9/64 ⇒ length symbol 257 decodes to a different match length; output verified against a reference model | `globals_diff.rs::i25_mutate_len_base` | [x] |
| I26 | `cp_inflate` + globals | `cp_dist_base[0]` mutated to 1/2/3/8 (output verified) and `cp_len_extra_bits[0]`/`cp_dist_extra_bits[0]` to 0/1/2/5/13 (different bit consumption) | `globals_diff.rs::i26_mutate_dist_base_and_extra_bits` | [x] |
| I27 | globals | initial contents of all 7 exported data objects read through `dlsym` are byte-identical, and their `nm -S` sizes match | `symbols.rs::exported_globals_have_identical_contents`, `exported_object_sizes_match` | [x] |
| I28 | `cp_inflate` + globals | `cp_fixed_table` replaced by a *different but still complete* canonical assignment (literals 9 bits, 256…287 6 bits); streams encoded with the new table must decode, and the same stream must **fail** with the pristine table | `globals_diff.rs::i28_mutate_fixed_table` | [x] |
| I29 | `cp_inflate` + globals | `cp_permutation_order` reversed (still a permutation of 0…18) with the dynamic header written in the new order | `globals_diff.rs::i29_mutate_permutation_order` | [x] |
| I30 | `cp_inflate` | static block with payload length 0…64 × alignment 0…3 ⇒ the block ends at every phase relative to the 32-bit word loads of `cp_peak_bits` | `i30_word_boundary_sweep` | [x] |

`globals_diff.rs::mutations_are_isolated_to_the_child` additionally proves that
the per-call mutations really are per-call (fork copy-on-write), so the rows
above cannot leak into each other.

## Build-time configuration surface

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` has no
`option()`/`add_definitions()`/`#ifdef` either (one source file, `SHARED`, links
`m`; `NDEBUG` is never defined, which is why the `assert()`s in `ERRORS.md` §A
are live). The complete set of valid build configurations is therefore

| # | configuration | command | ✔ |
|---|---------------|---------|---|
| B1 | no features (`--no-default-features`) | `scripts/check_all_features.sh test` | [x] |
| B2 | default features (identical set, distinct cargo invocation) | `scripts/check_all_features.sh test` | [x] |
| B3 | release profile (`opt-level=3`, `panic="abort"`) | `scripts/run_tests.sh --release` | [x] |

All three run the *whole* suite (83 tests) and pass.
