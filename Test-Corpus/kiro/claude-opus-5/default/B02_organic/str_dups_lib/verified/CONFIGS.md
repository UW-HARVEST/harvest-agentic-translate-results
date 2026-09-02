# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `A` arena/table string mode (`stbds_hash_index.string.mode`) | `SH_NONE(0)`, `SH_DEFAULT(1)`, `SH_STRDUP(2)`, `SH_ARENA(3)` | `switch (table->string.mode)` lib.c:791; `hmfree_func` lib.c:576; `hmdel_key` lib.c:836 |
| `B` `mode` argument | `< 1` (binary: `memcmp` + `hash_bytes`), `>= 1` (string: `strcmp` + `hash_string`); `hmdel_key` additionally tests `== 1` exactly | `mode >= STBDS_HM_STRING` lib.c:567/616/706/725; `mode == STBDS_HM_STRING` lib.c:836/841 |
| `C` `elemsize` | any; distinct sizes change stride & the `key` slot layout: 8, 12, 16, 24, 40, odd (13) | every `elemsize*i` |
| `D` `keysize` | 1, 2, 3, 4, 7, 8, 9, 16, 32; only used by `memcmp`/`hash_bytes` in binary mode | lib.c:567/570 |
| `E` `keyoffset` (`hmdel_key` only; 0 everywhere else) | 0, non-zero | lib.c:565 |
| `F` table `slot_count` regime | 8 (initial), grown ×2 (`used_count >= used_count_threshold`), shrunk ÷2 (`used_count < used_count_shrink_threshold && slot_count > 8`), rebuilt same size (`tombstone_count > tombstone_count_threshold`) | lib.c:698–710, 856–863 |
| `G` probe path | in-bucket hit (`i = pos&MASK .. 7`), wrap-around scan (`0 .. pos&MASK`), multi-bucket probe (`pos += step; step += 8`) | lib.c:604–629, 731–762 |
| `H` slot state encountered | `HASH_EMPTY(0)`, `HASH_DELETED(1)` / `INDEX_DELETED(-2)` tombstone reuse, live entry | lib.c:736/739, 767 |
| `I` delete position | delete the last live element (`old_index == final_index`, no memmove), delete a middle element (memmove + re-lookup + index patch) | lib.c:838 |
| `J` `arrgrowf` shape | `a == NULL` vs existing; `addlen` 0/1/n; `min_cap` 0 / < cap / < 2·cap / ≥ 2·cap / < 4 | lib.c:280–296 |
| `K` `hash_bytes` length | 0, 1..7 (each tail `case`), 8, 9..15, 16, 17, 24, 64, 65 | lib.c:509–532 |
| `L` `hash_bytes` byte values | byte 3 / 7 / 11 with high bit set (sign-extension quirk), all-zero, all-0xff, random | lib.c:510/511/530 |
| `M` `hash_string` input | `""`, 1 char, 8 chars, 64 chars, bytes ≥ 0x80, embedded high-bit | lib.c:465–468 |
| `N` seed | default `0x31415926`, `0`, `SIZE_MAX`, arbitrary (via `stbds_rand_seed`); also the internal LCG advance in `make_hash_index(ot == NULL)` | lib.c:390/404–410 |
| `O` `stralloc` string length vs block | `len <= remaining` (fast path), `len > remaining && len <= blocksize` (new block), `len > blocksize` (oversize block) with `storage == NULL` and `storage != NULL` | lib.c:884–910 |
| `P` arena `block` progression | 0,1,2,… → `blocksize = 512 << (block>>1)`, saturating once `blocksize >= 1<<20` | lib.c:887–892 |
| `Q` `str_dups` `num` | 0, 1, 2, 8, 64, 200, 1000 (drives arena block growth) | lib.c:947 |
| `R` `hmput_default` state | `a == NULL`, `a != NULL && length == 0`, `a != NULL && length != 0` | lib.c:669 |
| `S` entry point | low-level: `arrgrowf`, `arrfreef`, `hash_bytes`, `hash_string`, `rand_seed`, `stralloc`, `strreset`, `shmode_func`, `hmput_default`, `hmput_key`, `hmget_key`, `hmget_key_ts`, `hmdel_key`, `hmfree_func`; high-level: `strkey`, `str_dups` | — |

## Rows (combinations tested)

Each row is driven with many pseudo-random inputs from a fixed-seed xorshift
PRNG (`SEED = 0x243F6A8885A308D3`), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `K`=0..80 × `L`=random bytes, seed = default | [x] |
| 2 | `stbds_hash_bytes` | `K`=8..16 × `L`=byte 3 / byte 7 / byte 11 high bit set (sign-extension quirk) | [x] |
| 3 | `stbds_hash_bytes` | `K`=0..80 × `L`=all-0x00 / all-0xff / 0x00-0x80 alternating | [x] |
| 4 | `stbds_hash_bytes` | `N`=seed 0, 1, SIZE_MAX, random × `K`=0..24 (seed provably cancels — verify both agree) | [x] |
| 5 | `stbds_hash_string` | `M`=`""`, 1, 2, 7, 8, 9, 63, 64, 255 chars ASCII random × `N`=random seeds | [x] |
| 6 | `stbds_hash_string` | `M`=bytes 0x80..0xff (signed-char promotion) × `N`=random seeds | [x] |
| 7 | `stbds_rand_seed` + `stbds_shmode_func` | `N`: seed the global LCG, then observe the seed captured by successive `make_hash_index(NULL)` calls (LCG advance) | [x] |
| 8 | `stbds_arrgrowf` | `J`: `a == NULL` × `addlen`∈{0,1,2,5,100} × `min_cap`∈{0,1,3,4,7,64} × `C` elemsize∈{1,4,8,13,16,40} | [x] |
| 9 | `stbds_arrgrowf` | `J`: existing array × `min_cap <= cap` (no-op), `cap < min_cap < 2cap`, `min_cap >= 2cap` | [x] |
| 10 | `stbds_arrgrowf` + `stbds_arrfreef` | grow chain (repeated `arrgrowf(a,e,1,0)` doubling), then free | [x] |
| 11 | `stbds_stralloc` | `O`=fast path: fresh arena, many short strings (< 512) until the block fills | [x] |
| 12 | `stbds_stralloc` | `O`=new-block path: strings sized to straddle the remaining bytes; `P` block 0→saturation | [x] |
| 13 | `stbds_stralloc` | `O`=oversize with `storage == NULL` (first call, len > 512) | [x] |
| 14 | `stbds_stralloc` | `O`=oversize with `storage != NULL` (short string first, then a > 512 string) | [x] |
| 15 | `stbds_stralloc` + `stbds_strreset` | random mixed-length sequences (1..3000 bytes), then reset, then reuse the same arena | [x] |
| 16 | `stbds_shmode_func` | `A`=`SH_NONE/SH_DEFAULT/SH_STRDUP/SH_ARENA` × `C` elemsize∈{8,12,16,24,40} | [x] |
| 17 | `stbds_hmput_default` | `R`=all three states × `C` elemsize∈{8,16,24} | [x] |
| 18 | `stbds_hmput_key` | `B`=binary, `A`=SH_NONE (table auto-created), `D` keysize∈{1,2,3,4,7,8,9,16} × `C` elemsize = keysize+8 rounded, 1 insert | [x] |
| 19 | `stbds_hmput_key` | `B`=binary, `F`=grow: 0,1,2,6,7,8,50,300 distinct random keys (crosses `used_count_threshold` several times) | [x] |
| 20 | `stbds_hmput_key` | `B`=binary, duplicate keys re-put (update path, `temp` = existing index), interleaved with new keys | [x] |
| 21 | `stbds_hmput_key` | `B`=string, `A`=SH_DEFAULT (auto, table created by `hmput_key` with `mode>=1`) × random C strings, `F` grow to 300 | [x] |
| 22 | `stbds_hmput_key` | `B`=string, `A`=SH_STRDUP (via `shmode_func`) × random C strings, `F` grow to 300 | [x] |
| 23 | `stbds_hmput_key` | `B`=string, `A`=SH_ARENA (via `shmode_func`) × random C strings incl. > 512 bytes (`O` oversize inside the map) | [x] |
| 24 | `stbds_hmput_key` | `A`=SH_NONE via `shmode_func(e, SH_NONE)` + `B`=string mode → `default:` memcpy branch with string hashing | [x] |
| 25 | `stbds_hmget_key` / `_ts` | `B`=binary, present / absent keys, on tables of size 8/16/32/64 (`F`), `G` all probe paths | [x] |
| 26 | `stbds_hmget_key` / `_ts` | `B`=string, `A`=SH_DEFAULT / SH_STRDUP / SH_ARENA, present / absent | [x] |
| 27 | `stbds_hmget_key_ts` | `a == NULL` bootstrap, then repeated `_ts` on the bootstrapped (table-less) array | [x] |
| 28 | `stbds_hmdel_key` | `B`=binary, `I`=delete last live element (no memmove) | [x] |
| 29 | `stbds_hmdel_key` | `B`=binary, `I`=delete middle element (memmove + re-lookup + `H` tombstone) | [x] |
| 30 | `stbds_hmdel_key` | `B`=binary, `F`=shrink: fill to 300 then delete until `used_count < shrink_threshold` repeatedly | [x] |
| 31 | `stbds_hmdel_key` | `B`=binary, `F`=tombstone rebuild: alternate put/del on a fixed-size table until `tombstone_count > threshold` | [x] |
| 32 | `stbds_hmdel_key` | `B`=string, `A`=SH_STRDUP (frees the duplicated key) × `I` both delete positions | [x] |
| 33 | `stbds_hmdel_key` | `B`=string, `A`=SH_DEFAULT and SH_ARENA (no key free) × `I` both | [x] |
| 34 | `stbds_hmdel_key` | `E`=non-zero `keyoffset` with `B`=binary on a struct whose key is not at offset 0 | [x] |
| 35 | `stbds_hmput_key`+`hmdel_key`+`hmget_key` | randomized op-mixture fuzz (1500 ops, binary mode, keysize 8, elemsize 16) — full pipeline, `F/G/H/I` all reached | [x] |
| 36 | `stbds_hmput_key`+`hmdel_key`+`hmget_key` | randomized op-mixture fuzz (1500 ops, string mode, SH_STRDUP) | [x] |
| 37 | `stbds_hmput_key`+`hmdel_key`+`hmget_key` | randomized op-mixture fuzz (1500 ops, string mode, SH_ARENA) | [x] |
| 38 | `stbds_hmfree_func` | `A`=each of the 4 modes, after 0 / 1 / many inserts (SH_STRDUP sweeps `i = 1..length`) | [x] |
| 39 | `strkey` | `n` = 0, 1, 9, 10, 99, 100, 12345, -1, -12345, `INT_MIN`, `INT_MAX`, random | [x] |
| 40 | `str_dups` | `Q`=0, 1, 2, 3, 8, 63, 64, 100, 500, 1000 — compare stdout byte-for-byte | [x] |
| 41 | `stbds_rand_seed` + full map pipeline | `N`: seed ∈ {0, 1, 0x31415926, SIZE_MAX, random} then run row-35 fuzz — hash/probe order is seed dependent | [x] |
| 42 | ABI/layout | `stbds_array_header` (32 B), `stbds_string_arena` (24 B), `stbds_hash_bucket` (128 B), `stbds_hash_index` (104 B) sizes/offsets agree between the two `.so`s | [x] |

## Result

All 42 rows pass across randomized inputs (fixed seed
`0x243F6A8885A308D3`) in all four build configurations. See
`FEATURE_MATRIX.md`.

Notes on how rows are driven:

* Every map row goes through the **low-level exports** (`stbds_shmode_func`,
  `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
  `stbds_hmdel_key`, `stbds_hmfree_func`) with the element/key/value writes the
  `stbds_hmput`/`shput`/`shputs` macros would perform, i.e. the composed
  pipeline, not one wrapper at a time.
* After **every** call the entire canonical state of both maps is compared:
  array header (`length`, `capacity`, `temp`, table-null-ness), every live
  element's bytes (string keys compared by content), and every field of
  `stbds_hash_index` plus all `slot_count` hash/index slots. Pointer *values*
  are reduced to `NULL`/`PTR` because the two libraries allocate independently;
  `temp_key` is excluded except where the C guarantees it was written and is
  still live (`hmput_key`'s `SH_DEFAULT/STRDUP/ARENA` branches on a fresh
  insert), since `stbds_make_hash_index` leaves it uninitialised.
* Rows 30 and 31 assert that the shrink and same-size-rebuild branches
  (lib.c:856 / lib.c:860) were actually reached, so the row cannot pass
  vacuously.
* Row 24 (`SH_NONE` table + string `mode`) is inherently self-corrupting in the
  C: the `default:` branch `memcpy`s the *string's bytes* into the element,
  which any later lookup reinterprets as a `char *`. Only the insert half is
  comparable in-process; the crashing lookup is covered as a crash-equivalence
  scenario (`sh_none_string_lookup`) in Phase C.
