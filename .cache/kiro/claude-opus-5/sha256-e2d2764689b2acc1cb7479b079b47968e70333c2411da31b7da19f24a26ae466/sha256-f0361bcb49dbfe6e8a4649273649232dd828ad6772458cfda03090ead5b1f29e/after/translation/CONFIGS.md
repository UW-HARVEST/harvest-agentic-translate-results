# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from the branch conditions in `c_src/src/lib.c`. The axes
below are exactly the things the C code tests on:

**Axis M — `mode` argument** (`stbds_hmget_key`, `stbds_hmget_key_ts`,
`stbds_hmput_key`, `stbds_hmdel_key`). Branched on at lines 560, 590, 707,
713, 732, 836, 842:
`mode < 1` → binary (`memcmp`); `mode >= 1` → string (`strcmp`);
`mode == 1` *exactly* gates the strdup-free and the delete re-find.

**Axis S — `table->string.mode`** (`switch` at line 790, and line 576, 836).
Set either by `stbds_hmput_key` (`0` or `STBDS_SH_DEFAULT`) or explicitly by
`stbds_shmode_func`: `STBDS_SH_NONE=0`, `STBDS_SH_DEFAULT=1`,
`STBDS_SH_STRDUP=2`, `STBDS_SH_ARENA=3`, out-of-range → `default:` memcpy.

**Axis E — `elemsize` / `keysize` / `keyoffset`.** `elemsize` scales every
pointer step; `keysize` selects the siphash length path; `keyoffset` is `0`
from every macro but is a real parameter of `stbds_hmdel_key`.

**Axis L — hash input length** (`stbds_hash_bytes`): `0`, `1..7` (each of the
seven `switch` fall-through arms), `8` (one whole word, empty tail), `9..15`,
`16`, `>16`, and lengths whose 4th/8th byte has the high bit set (sign-extension
path).

**Axis T — table size / population state:** empty (no table), 8 slots, grown to
16/32/64 (`used_count >= slot_count - slot_count/4`), tombstoned
(`tombstone_count > slot_count/8 + slot_count/16`), shrunk
(`used_count < slot_count/4`).

**Axis A — array state** (`stbds_arrgrowf`): `a == NULL` vs existing; `addlen`
`0`/`1`/`n`; `min_cap` below cap (no-op), between, and above `2*cap`.

**Axis R — global seed** (`stbds_rand_seed`) — mutates `stbds_hash_seed`, which
is consumed and then advanced by every fresh `stbds_make_hash_index`. Both
libraries must produce the same seed *sequence*.

**Axis B — arena block growth** (`stbds_stralloc`): `block` 0..24,
`len <= blocksize` vs `len > blocksize`, `storage == NULL` vs not.

Every row is driven with **many randomized inputs** (fixed seed `0x5EED_1234`,
`SplitMix64`) unless the row is inherently a single shape.

| # | entry point(s) | configuration (options + input shape) | test | ✔ |
|---|----------------|----------------------------------------|------|---|
| 1 | `stbds_rand_seed` + `stbds_hash_bytes` | L = 0, random seeds | `cfg_01_hash_bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` | L = 1..7 (all tail arms), random bytes × random seeds | `cfg_02_hash_bytes_tail_arms` | [x] |
| 3 | `stbds_hash_bytes` | L = 8 exactly (one word, empty tail) | `cfg_03_hash_bytes_len8` | [x] |
| 4 | `stbds_hash_bytes` | L = 9..15 (one word + each tail arm) | `cfg_04_hash_bytes_word_plus_tail` | [x] |
| 5 | `stbds_hash_bytes` | L = 16, 24, 32, 64, 65, 127, 128 (multi-word) | `cfg_05_hash_bytes_multiword` | [x] |
| 6 | `stbds_hash_bytes` | L = 1..64, bytes forced `>= 0x80` at every position (sign-extension) | `cfg_06_hash_bytes_high_bit` | [x] |
| 7 | `stbds_hash_bytes` | seed = 0, 1, `usize::MAX`, `0x31415926` (default), random | `cfg_07_hash_bytes_seeds` | [x] |
| 8 | `stbds_hash_string` | len 0..64 ASCII, random seeds | `cfg_08_hash_string_ascii` | [x] |
| 9 | `stbds_hash_string` | bytes 0x80..0xFF (unsigned-char promotion) | `cfg_09_hash_string_high_bytes` | [x] |
| 10 | `stbds_hash_string` | seed = 0, 1, `usize::MAX`, random; long strings (256 B) | `cfg_10_hash_string_seeds` | [x] |
| 11 | `stbds_rand_seed` | seed set, then N fresh tables created → the whole LCG seed *sequence* must match | `cfg_11_seed_sequence` | [x] |
| 12 | `stbds_arrgrowf` | A: `a==NULL`, random `elemsize` ∈ {1,2,4,8,16,32}, `addlen`∈{0,1,7}, `min_cap`∈{0,1,4,5,100} | `cfg_12_arrgrowf_fresh` | [x] |
| 13 | `stbds_arrgrowf` | A: existing array, `min_cap <= cap` → no-op path | `cfg_13_arrgrowf_noop` | [x] |
| 14 | `stbds_arrgrowf` | A: existing array, `min_cap` in `(cap, 2*cap)` → doubling path | `cfg_14_arrgrowf_double` | [x] |
| 15 | `stbds_arrgrowf` | A: existing array, `min_cap > 2*cap` → exact path | `cfg_15_arrgrowf_exact` | [x] |
| 16 | `stbds_arrgrowf` | repeated append-one growth (0→1→…→2000 elements): full capacity *sequence* | `cfg_16_arrgrowf_growth_sequence` | [x] |
| 17 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free, repeated; header fields after each grow | `cfg_17_arrgrow_free_cycle` | [x] |
| 18 | `arr_push` (driver-style entry point) | `num` = 0,1,2,49,50,51,100,499,500,1000,5000 | `cfg_18_arr_push_range` | [x] |
| 19 | `strkey` | `n` = 0,1,9,10,99,100,`INT_MAX`, random, negatives | `cfg_19_strkey` | [x] |
| 20 | `stbds_hmput_default` | fresh (`a == NULL`), elemsize ∈ {8,16,32} | `cfg_20_put_default_fresh` | [x] |
| 21 | `stbds_hmput_default` | called twice (second time `length == 1` → no-op) | `cfg_21_put_default_idempotent` | [x] |
| 22 | `stbds_hmput_key` M=0 (binary) S=0 | keysize 4, elemsize 8, 1 insert | `cfg_22_bin_single` | [x] |
| 23 | `stbds_hmput_key` M=0 S=0 | keysize 4, elemsize 8, 6 inserts (fills 8-slot table to threshold) | `cfg_23_bin_to_threshold` | [x] |
| 24 | `stbds_hmput_key` M=0 S=0 | keysize 4, elemsize 8, 200 random inserts → 3 table growths | `cfg_24_bin_many_grow` | [x] |
| 25 | `stbds_hmput_key` M=0 S=0 | keysize 8 (`ptrdiff_t`-sized key), elemsize 16 | `cfg_25_bin_keysize8` | [x] |
| 26 | `stbds_hmput_key` M=0 S=0 | keysize 16 (two-word key, `stbds_struct2` shape), elemsize 32 | `cfg_26_bin_keysize16` | [x] |
| 27 | `stbds_hmput_key` M=0 S=0 | keysize 1, 2, 3, 5, 7 (odd key widths → tail arms inside the map) | `cfg_27_bin_odd_keysizes` | [x] |
| 28 | `stbds_hmput_key` M=0 S=0 | duplicate keys interleaved with new ones (update path) | `cfg_28_bin_duplicates` | [x] |
| 29 | `stbds_hmget_key` M=0 | lookups: all present keys + all absent keys, after growth | `cfg_29_bin_get_all` | [x] |
| 30 | `stbds_hmget_key_ts` M=0 | same as #29 but through the `temp`-out variant (does not touch `header->temp`) | `cfg_30_bin_get_ts` | [x] |
| 31 | `stbds_hmdel_key` M=0 S=0 keyoffset 0 | delete present, delete absent, delete-last, delete-first | `cfg_31_bin_del_basic` | [x] |
| 32 | `stbds_hmdel_key` M=0 | random put/get/del storm, 4000 ops → hits shrink + tombstone rebuild | `cfg_32_bin_storm` | [x] |
| 33 | `stbds_hmdel_key` M=0 | `keyoffset != 0` (key not at element offset 0) | `cfg_33_bin_keyoffset` | [x] |
| 34 | `stbds_hmfree_func` M=0 S=0 | free a populated binary map | `cfg_34_bin_free` | [x] |
| 35 | `stbds_hmput_key` M=1 (string) S=1 (`SH_DEFAULT`, implicit) | caller-owned key pointers, 1 insert | `cfg_35_str_default_single` | [x] |
| 36 | `stbds_hmput_key` M=1 S=1 | 200 random `strkey`-style keys → growth; `temp_key` written each time | `cfg_36_str_default_many` | [x] |
| 37 | `stbds_hmput_key` M=1 S=1 | duplicate string keys (update path sets `temp_key` from the *stored* pointer) | `cfg_37_str_default_dup` | [x] |
| 38 | `stbds_hmget_key` M=1 S=1 | present + absent string lookups | `cfg_38_str_default_get` | [x] |
| 39 | `stbds_hmdel_key` M=1 S=1 | delete present/absent/last, with swap-and-refind through `strcmp` | `cfg_39_str_default_del` | [x] |
| 40 | `stbds_shmode_func(S=2 STRDUP)` + `hmput_key` M=1 | keys copied with `stbds_strdup`; 200 inserts | `cfg_40_str_strdup_many` | [x] |
| 41 | `stbds_shmode_func(S=2)` + `hmdel_key` M=1 | delete frees the strdup'd key (`mode == 1` branch) | `cfg_41_str_strdup_del` | [x] |
| 42 | `stbds_shmode_func(S=2)` + `hmfree_func` | free walks `i=1..length` freeing each strdup'd key | `cfg_42_str_strdup_free` | [x] |
| 43 | `stbds_shmode_func(S=3 ARENA)` + `hmput_key` M=1 | keys copied into the string arena; short keys only (block 0 → 512) | `cfg_43_str_arena_short` | [x] |
| 44 | `stbds_shmode_func(S=3)` + `hmput_key` M=1 | enough keys to force several arena blocks (block 0→1→2…) | `cfg_44_str_arena_multiblock` | [x] |
| 45 | `stbds_shmode_func(S=3)` + `hmput_key` M=1 | one key longer than the current blocksize → oversized-block splice path | `cfg_45_str_arena_oversized` | [x] |
| 46 | `stbds_shmode_func(S=3)` + `hmfree_func` | `strreset` walks and frees the whole block chain | `cfg_46_str_arena_free` | [x] |
| 47 | `stbds_shmode_func(S=0 NONE)` + `hmput_key` M=0 | explicit table with `string.mode = 0` → `default:` memcpy branch | `cfg_47_shmode_none_binary` | [x] |
| 48 | `stbds_shmode_func(S=4)` (out of enum range) + `hmput_key` M=1 | `switch` falls to `default:` → `memcpy(key, keysize)` even though M says string | `cfg_48_shmode_four` | [x] |
| 49 | `stbds_shmode_func` | `mode` = 0,1,2,3,4,127,255,256,-1,`INT_MAX`,`INT_MIN` → `(unsigned char)` truncation | `cfg_49_shmode_truncation` | [x] |
| 50 | `stbds_hmput_key` M=2 / M=7 / `INT_MAX` | `mode > 1` → string path everywhere in put/get | `cfg_50_mode_gt_one` | [x] |
| 51 | `stbds_hmput_key` M=-1 / `INT_MIN` | `mode < 1` → binary path everywhere | `cfg_51_mode_negative` | [x] |
| 52 | `stbds_stralloc` | B: fresh arena, random short strings until several blocks | `cfg_52_stralloc_blocks` | [x] |
| 53 | `stbds_stralloc` | B: strings of length 511/512/513 around the first block boundary | `cfg_53_stralloc_boundary` | [x] |
| 54 | `stbds_stralloc` | B: `len > blocksize` on a *fresh* arena (`storage == NULL`) → `remaining = 0` | `cfg_54_stralloc_oversized_fresh` | [x] |
| 55 | `stbds_stralloc` | B: `len > blocksize` on a *populated* arena → splice after head | `cfg_55_stralloc_oversized_spliced` | [x] |
| 56 | `stbds_stralloc` | B: `block` driven to saturation (`>= 1<<20`) | `cfg_56_stralloc_saturate` | [x] |
| 57 | `stbds_strreset` | populated chain (many blocks) then reset, then reuse | `cfg_57_strreset_reuse` | [x] |
| 58 | full pipeline, M=0 S=0 | interleaved `put`/`get`/`get_ts`/`del`/`arrgrowf` on the *same* map, 4000 randomized ops, elemsize 8 keysize 4 | `cfg_58_pipeline_binary` | [x] |
| 59 | full pipeline, M=1 S=1 | same but string keys with caller-owned storage | `cfg_59_pipeline_str_default` | [x] |
| 60 | full pipeline, M=1 S=2 | same but STRDUP arena-free semantics | `cfg_60_pipeline_strdup` | [x] |
| 61 | full pipeline, M=1 S=3 | same but ARENA | `cfg_61_pipeline_arena` | [x] |
| 62 | full pipeline, mixed elemsize | elemsize ∈ {8,16,24,32,64} × keysize ∈ {1,2,4,8,16} cross-product, 200 ops each | `cfg_62_pipeline_elemsize_cross` | [x] |
| 63 | `stbds_rand_seed` interaction | reseed *between* table creations inside one pipeline | `cfg_63_reseed_midstream` | [x] |
| 64 | `stbds_arrgrowf` + `stbds_hmput_key` + `stbds_hmdel_key` | plain-array growth interleaved with map operations on the same heap, 1500 randomized ops (the allocators must be driven in the same order by both libraries) | `cfg_31b_mixed_array_and_map` | [x] |
