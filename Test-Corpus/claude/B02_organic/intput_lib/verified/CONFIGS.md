# CONFIGS.md — configuration-surface table for `c_src/src/lib.c`

The mirror of `ERRORS.md` for **valid** inputs. Axes were derived mechanically
from the branches the C actually takes, not from what looks important.

## Build-time configurations (Phase A enumeration)

There is exactly **one** valid build configuration:

| source of build-time variation | result |
|--------------------------------|--------|
| `Cargo.toml` `[features]` | **absent** — no features are declared, so the only feature combination is the **empty set** |
| `cfg(feature = …)` in `src/` or `tests/` | **none** (`grep -rn 'cfg(feature' src tests` is empty) |
| `build.rs` | **absent** |
| `#ifdef` / `#if` / `#elif` / `#else` in `c_src/src/lib.c` and `c_src/include/lib.h` | **none** — every `#define` is unconditional, so the C has a single compilation |
| `option()` / `if()` / `CMAKE_BUILD_TYPE` / `add_definitions` / `target_compile_*` in `c_src/CMakeLists.txt` | **none** — and because `CMAKE_BUILD_TYPE` is empty, no `-DNDEBUG` is passed, so all `assert()`s are live |

The full combination list is therefore:

```
cargo check --no-default-features --features ''      # the only combination
```

`run_all.sh` derives this list from `Cargo.toml` programmatically (so it stays
correct if features are ever added) and runs `cargo check --tests` **and** the
whole differential suite for every combination × `{dev, release}` profile.

## Axes the C branches on

### 1. Runtime options / modes settable through the public API

| axis | set by | values the C distinguishes | branch sites |
|------|--------|----------------------------|--------------|
| `mode` (per-call key-comparison mode) | `mode` argument of `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmdel_key` | `mode >= STBDS_HM_STRING (1)` ⇒ string; `< 1` ⇒ binary. `mode == STBDS_HM_STRING` exactly ⇒ extra del behaviour | L560, L590, L707, L713, L732, L836, L842 |
| `table->string.mode` (per-table key-storage mode) | `mode` argument of `stbds_shmode_func`; otherwise auto-set by the first `stbds_hmput_key` to `STBDS_SH_DEFAULT` (string mode) or `0`/`STBDS_SH_NONE` (binary mode) | `STBDS_SH_NONE (0)` ⇒ `memcpy`; `STBDS_SH_DEFAULT (1)` ⇒ store caller pointer; `STBDS_SH_STRDUP (2)` ⇒ `stbds_strdup`; `STBDS_SH_ARENA (3)` ⇒ `stbds_stralloc`; anything else ⇒ `default:` `memcpy` | L575, L707, L785–790, L803, L836 |
| global hash seed | `stbds_rand_seed(seed)`; also self-advanced by every fresh `stbds_make_hash_index(_, NULL)` (`seed = seed*a + b`) | any `size_t`; changes **every** hash and therefore every probe order and every bucket layout | L353, L355–358, L409–412 |
| table `seed` inheritance | `stbds_make_hash_index(n, ot)` with `ot != NULL` (grow/shrink/rebuild) vs `ot == NULL` (fresh) | inherit `ot->seed` + `ot->string` **or** take/advance the global seed | L403–413 |
| arena block growth | `stbds_string_arena.block` (advanced by `stbds_stralloc`) | `512u << (block>>1)`, capped by `1<<20` (`++block` skipped once capped) | L886–891 |

### 2. Input shapes the C special-cases

| axis | values the C distinguishes | branch sites |
|------|----------------------------|--------------|
| handle `a` | `NULL`; non-NULL with `hash_table == NULL`; non-NULL with a table; `length == 0` | L286, L300, L573–574, L634, L644, L669, L686, L698, L809, L816 |
| `elemsize` | `0`; `< sizeof(void*)`; `== keysize`; `> keysize`; non-power-of-two | L297, L561–563, L578, L733, L786–789, L840 |
| `keysize` | `0`; `4`; `8` (== `sizeof(size_t)`, the siphash block size); `> 8`; not a multiple of 8 | L522 (`i+8 <= len`), L532 (`switch (len-i)` cases 7…0) |
| `keyoffset` | `0` (what all the macros pass); `!= 0` | L561, L563, L733, L843, L845 |
| `stbds_hash_bytes` `len` | `0`, `1`…`7` (each `switch` fall-through case), `8`, `9`…`15`, `16`, `> 16` | L522, L531–541 |
| byte values | bytes `< 0x80` vs `>= 0x80` in positions 3 and 7 (the `d[3] << 24` / `d[7] << 24` **int** sign-extension quirk) | L523–524, L536 |
| `stbds_hash_string` input | `""`; 1 char; long; bytes `>= 0x80` (the `(unsigned char)` cast) | L480–481 |
| element count | `0`, `1`, `< used_count_threshold`, `== used_count_threshold` (⇒ grow ×2), several growths (thresholds 6, 12, 24, 48, 96, 192 for slot counts 8, 16, 32, 64, 128, 256) | L698–710 |
| probe shape | key lands in `pos&7 .. 7` (forward scan) vs `0 .. pos&7` (wrap scan) vs next bucket (`pos += step`, `step += 8`) | L604–627, L728–763 |
| put on existing key | found ⇒ set `temp` (+ `temp_key` for string mode) and return early | L729–735, L747–751 |
| delete position | `old_index == final_index` (delete last, no memmove) vs `old_index != final_index` (memmove + re-index) | L839–851 |
| delete volume | until `tombstone_count > tombstone_count_threshold` (⇒ rebuild, same slot_count) and until `used_count < used_count_shrink_threshold && slot_count > 8` (⇒ shrink ÷2; shrink threshold forced to 0 when `slot_count <= 8`) | L399–400, L854–862 |
| reuse of tombstones | put finds a `STBDS_INDEX_DELETED` slot before an empty one ⇒ `pos = tombstone; --tombstone_count` | L739–742, L755–758, L766–769 |
| `stbds_stralloc` string length | `1` (`""`); `<= remaining`; `> remaining` but `<= blocksize`; `> blocksize` (dedicated oversized block, spliced *after* head when `storage != NULL`, else becomes head with `remaining = 0`) | L885–911 |
| `strkey` `n` | `0`, positive, negative, `INT_MIN`, `INT_MAX` | L941 |
| `intput` `num` | any value ≠ 9, 11 (see `ERRORS.md` #44/#45 for 9 and 11) | L945–956 |

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0xC0FFEE_...`, see `tests/common/mod.rs`) against both `.so`s, comparing:
header (`length`, `capacity`, `temp`), all element bytes (keys dereferenced as
strings for string modes), and the whole `stbds_hash_index`
(`slot_count`, `slot_count_log2`, `used_count`, all three thresholds,
`tombstone_count`, `seed`, `string.{remaining,block,mode}`) plus every
bucket's `hash[8]` / `index[8]`.

The seeds are fixed for reproducibility, but `STBDS_DIFF_SEED=<n>` re-seeds the
whole suite so the same coverage runs over completely different random inputs:

```sh
for s in 0 1 2 7 12345 999983 4294967295; do
  STBDS_DIFF_SEED=$s cargo test              # 110 passed, every seed
  STBDS_DIFF_SEED=$s cargo test --release
done
```

Rows whose point is to *reach* a rare code path (31 rebuild, 32 tombstone reuse,
41b wrap-scan, 6/7 wrap-scan miss, 23 bucket collisions) are constructed
**deterministically** — by searching for keys with an exact probe position and
then driving the exact operation sequence — rather than hoping a random stream
happens to hit them, so their coverage does not depend on the seed.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p = NULL` and `p` valid; seeds `{0, 1, 0x31415926, SIZE_MAX, random×64}` | `cfg_01_hash_bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (every `switch (len-i)` fall-through case), random bytes ×256/len | `cfg_02_hash_bytes_tail_1_to_7` | [x] |
| 3 | `stbds_hash_bytes` | `len = 1..7` with byte 3 / byte 6 forced `>= 0x80` (sign-extension quirk of `d[3] << 24`) | `cfg_03_hash_bytes_tail_high_bit` | [x] |
| 4 | `stbds_hash_bytes` | `len = 8` exactly (one full siphash block, `len-i == 0`) | `cfg_04_hash_bytes_len8` | [x] |
| 5 | `stbds_hash_bytes` | `len = 9..15` (one block + each remainder), random ×64 each | `cfg_05_hash_bytes_len9_15` | [x] |
| 6 | `stbds_hash_bytes` | `len = 16, 24, 32` (exact multiples, `len-i == 0`) | `cfg_06_hash_bytes_multiples` | [x] |
| 7 | `stbds_hash_bytes` | `len = 17..64` random, all bytes `0x00`, all `0xFF`, and random — covers `d[3]`/`d[7]` high-bit in the *loop* body | `cfg_07_hash_bytes_len17_64` | [x] |
| 2-7b | `stbds_hash_bytes` | **exhaustive** reinforcement of rows 2..7: for every `len ∈ 0..=17` and every byte position, all 256 byte values (against both a `0x5A` and an `0xFF` background, the latter × 8 seeds). This exercises the `int`-promotion sign extension of `d[3] << 24` / `d[7] << 24` in *every* position, in the loop body and in each `switch (len - i)` fall-through case | `cfg_02_07_hash_bytes_exhaustive_bytes` | [x] |
| 8 | `stbds_hash_string` | `""`, `"a"`, 1..64 random ASCII, 1..64 random bytes `0x80..0xFF`, seeds ×8. Reinforced **exhaustively**: every single-byte string `1..=255` × 8 seeds, every byte value in each position of a 4-byte string, and all 2-byte strings over `{0x01,0x7F,0x80,0xFF} × 1..=255` — this covers the `(unsigned char) *str++` cast for every byte that is negative as a `char` | `cfg_08_hash_string` + `cfg_08_hash_string_exhaustive_bytes` | [x] |
| 9 | `stbds_rand_seed` + `stbds_hash_bytes` | seed set to `0`, `1`, `SIZE_MAX`, random; verifies the global is a *pure* input to later table creation | `cfg_09_rand_seed_is_pure_for_hashes` | [x] |
| 10 | `stbds_arrgrowf` | `a = NULL`, `elemsize ∈ {1,4,8,16,24}`, `addlen ∈ {0,1,3,7}`, `min_cap ∈ {0,1,2,4,5,17}` — full cross product, covering `min_len > min_cap`, `min_cap < 2*cap`, `min_cap < 4` | `cfg_10_arrgrowf_from_null` | [x] |
| 11 | `stbds_arrgrowf` | non-NULL `a` (grown once), re-grown with `addlen ∈ {0,1,cap,cap+1}`, `min_cap ∈ {0,cap-1,cap,cap+1,4*cap}` — covers the doubling branch and the no-op branch | `cfg_11_arrgrowf_regrow` | [x] |
| 12 | `stbds_arrgrowf` | `elemsize = 0` (header-only allocation) | `cfg_12_arrgrowf_elemsize_zero` | [x] |
| 13 | `stbds_arrgrowf` → `stbds_arrfreef` | grow then free; round-trip on a live pointer | `cfg_13_arrgrowf_arrfreef_roundtrip` | [x] |
| 14 | `stbds_hmput_default` | `a = NULL`, `elemsize ∈ {4,8,16,24}` | `cfg_14_hmput_default_null` | [x] |
| 15 | `stbds_hmput_default` | called twice (second call is a no-op because `length == 1`) | `cfg_15_hmput_default_twice` | [x] |
| 16 | `stbds_hmput_default` + `stbds_hmget_key` | default-only map (`hash_table == NULL`) then a lookup ⇒ `temp = -1` | `cfg_16_default_only_map_lookup` | [x] |
| 17 | `stbds_hmput_key` binary | `mode = 0`, `elemsize = 8`, `keysize = 4` (int→int), `n = 0,1,5,6,7,12,13,25,49,200` random keys | `cfg_17_put_int_int` | [x] |
| 18 | `stbds_hmput_key` binary | `mode = 0`, `elemsize = 16`, `keysize = 8` (i64→i64), `n` up to 300 random keys | `cfg_18_put_i64_i64` | [x] |
| 19 | `stbds_hmput_key` binary | `mode = 0`, `elemsize = 24`, `keysize = 16` (16-byte key, `len > 8` siphash path), `n` up to 200 | `cfg_19_put_key16` | [x] |
| 20 | `stbds_hmput_key` binary | `mode = 0`, `keysize = 12` (non-multiple of 8: 1 block + 4-byte tail) | `cfg_20_put_key12` | [x] |
| 21 | `stbds_hmput_key` binary | `mode = -1` and `INT_MIN` (out-of-range enum ⇒ binary) — must equal the `mode = 0` result exactly | `cfg_21_mode_negative_equals_binary` | [x] |
| 22 | `stbds_hmput_key` binary | repeated puts of **already present** keys (the found-early-return path), interleaved with new keys | `cfg_22_put_existing_keys` | [x] |
| 23 | `stbds_hmput_key` binary | keys chosen to collide in the same bucket (same `hash & (slot_count-1)`), forcing forward-scan, wrap-scan and `pos += step` bucket walks | `cfg_23_bucket_collisions` | [x] |
| 24 | `stbds_hmget_key` binary | hits and misses over a map of every size in row 17, `temp` read from the header | `cfg_24_25_get_and_get_ts` | [x] |
| 25 | `stbds_hmget_key_ts` binary | same as row 24 but `temp` written to the caller's `ptrdiff_t` (and the header left untouched) | `cfg_24_25_get_and_get_ts` | [x] |
| 26 | `stbds_hmdel_key` binary | delete the **last** element (`old_index == final_index`, no memmove) | `cfg_26_del_last` | [x] |
| 27 | `stbds_hmdel_key` binary | delete a **middle** element (memmove + re-index of the moved last element) | `cfg_27_del_middle` | [x] |
| 28 | `stbds_hmdel_key` binary | delete **every** element in insertion order, then in reverse, then in random order | `cfg_28_del_all_orders` | [x] |
| 29 | `stbds_hmdel_key` binary | delete enough from a `slot_count = 8` map that `used_count < shrink_threshold` — but `shrink_threshold == 0` for `slot_count <= 8`, so **no** shrink | `cfg_29_no_shrink_at_8_slots` | [x] |
| 30 | `stbds_hmdel_key` binary | grow past 8 slots then delete until `used_count < used_count_shrink_threshold && slot_count > 8` ⇒ **shrink** (`slot_count >> 1`, seed + arena inherited) | `cfg_30_shrink` | [x] |
| 31 | `stbds_hmdel_key` binary | delete/re-put churn until `tombstone_count > tombstone_count_threshold` ⇒ **rebuild** at the same `slot_count` | `cfg_31_rebuild_on_tombstones` | [x] |
| 32 | `stbds_hmput_key` binary | put after deletes so a **tombstone** slot is reused (`pos = tombstone; --tombstone_count`) | `cfg_32_tombstone_reuse` | [x] |
| 33 | `stbds_hmput_key`/`hmget`/`hmdel` binary | randomized 2 000-operation op-stream (put / get / get_ts / del, keys drawn from a small colliding pool so hits, misses, growth, shrink, rebuild and tombstone reuse all interleave), `elemsize`/`keysize` ∈ {(8,4),(16,8),(24,16)} | `cfg_33_random_op_stream` | [x] |
| 34 | `stbds_shmode_func` | `mode = STBDS_SH_NONE (0)`, `elemsize = 16` — table present, `string.mode = 0` ⇒ later puts `memcpy` the key bytes | `cfg_34_shmode_none` | [x] |
| 35 | `stbds_shmode_func` + `stbds_hmput_key` | `mode = STBDS_SH_DEFAULT (1)`, string keys: the caller's `char*` is stored verbatim; `temp_key` set | `cfg_35_sh_default` | [x] |
| 36 | `stbds_shmode_func` + `stbds_hmput_key` | `mode = STBDS_SH_STRDUP (2)`: every key `strdup`'d; verify stored strings, `temp_key`, and that `stbds_hmfree_func` frees them | `cfg_36_sh_strdup` | [x] |
| 37 | `stbds_shmode_func` + `stbds_hmput_key` | `mode = STBDS_SH_ARENA (3)`: keys allocated from the table's arena; verify `string.{remaining,block}` evolve identically over many puts (block growth 512, 512, 1024, …) | `cfg_37_sh_arena` | [x] |
| 38 | string map, no `shmode_func` | first `stbds_hmput_key` with `mode = 1` on a `NULL` handle ⇒ `nt->string.mode = STBDS_SH_DEFAULT` automatically | `cfg_38_string_map_from_null` | [x] |
| 39 | string map | `mode = 2` / `INT_MAX` (out-of-range enum ⇒ string path) for put + get; must equal `mode = 1` | `cfg_39_mode_above_string` | [x] |
| 40 | string map | `stbds_hmget_key`/`_ts` hits and misses; `stbds_hmdel_key` with `mode = 1` on DEFAULT / STRDUP / ARENA tables, delete-last and delete-middle | `cfg_40_string_delete` | [x] |
| 41 | string map | duplicate-key puts (found path ⇒ `temp_key` set from the *stored* pointer, no new allocation) | `cfg_41_string_duplicate_puts` | [x] |
| 41b | string map | the found-existing **forward-scan vs wrap-scan asymmetry**: C L729-735 sets `temp` **and** `temp_key`, C L747-751 sets **only** `temp`. Built by giving two keys the same initial probe position 7, so the second spills to slot 0 and its duplicate put is resolved by the wrap scan ⇒ `temp_key` must still point at the *first* key. 5 seeds × 3 storage modes | `cfg_41b_temp_key_scan_asymmetry` | [x] |
| 42 | string map | randomized 800-operation op-stream over each of `string.mode ∈ {DEFAULT, STRDUP, ARENA}` with a small pool of colliding key strings, growth + shrink + rebuild included | `cfg_42_string_op_stream` | [x] |
| 43 | `stbds_stralloc` | fresh arena; strings of length 1 (`""`), 2..40 — many allocations from one 512-byte block, then block growth (`512 << (block>>1)`) | `cfg_43_stralloc_progressive` + `cfg_43b_table_arena_survives_rehash` | [x] |
| 44 | `stbds_stralloc` | string longer than `blocksize` on a **fresh** arena (`storage == NULL`) ⇒ dedicated block becomes head, `remaining = 0` | `cfg_44_stralloc_oversized_fresh` | [x] |
| 45 | `stbds_stralloc` | string longer than `blocksize` on a **used** arena (`storage != NULL`) ⇒ dedicated block spliced in *after* the head, `remaining` preserved | `cfg_45_stralloc_oversized_used` | [x] |
| 46 | `stbds_stralloc` | drive `block` from 0 up to and past saturation (`block >= 22`, `blocksize` capped at `1<<20`) with 1 MiB+ strings | `cfg_46_stralloc_block_range` | [x] |
| 47 | `stbds_stralloc` + `stbds_strreset` | allocate N blocks then reset; arena fully zeroed, list freed | `cfg_47_strreset` | [x] |
| 48 | `stbds_hmfree_func` | binary map (`string.mode = 0`), string DEFAULT map, STRDUP map (frees each key), ARENA map (`stbds_strreset` on the table arena), and table-less map | `cfg_48_hmfree_all_flavours` | [x] |
| 49 | `strkey` | `n ∈ {0, 1, -1, 7, -7, 12345, -12345, INT_MAX, INT_MIN}` + 256 random `i32` | `cfg_49_strkey` | [x] |
| 50 | `intput` | `num ∈ {0, 1, -1, 7, 8, 10, 12, INT_MAX, INT_MIN}` + 256 random `i32` excluding 9 and 11; called repeatedly to confirm the global seed advances identically on both sides | `cfg_50_intput` + `cfg_50b_intput_repeated` | [x] |
| 51 | seed lock-step | `stbds_rand_seed(s)` then a long identical op-stream on both sides, verifying every table's inherited/fresh `seed` matches (the global is advanced once per fresh `stbds_make_hash_index`) | `cfg_51_seed_lockstep` | [x] |
| 52 | `keysize = 0` binary map | valid-but-degenerate: all keys hash identically and `memcmp(...,0) == 0` ⇒ single-entry map | `cfg_52_keysize_zero` | [x] |
| 53 | `stbds_shmode_func` + `stbds_hmput_key`/`hmget`/`hmdel` | the `mode` × `table->string.mode` cross-product cell: a `sh_new_strdup`/`sh_new_arena`/DEFAULT table driven with **binary** `mode = 0`. The two knobs are independent in the C — hashing/comparison follow `mode` (siphash + `memcmp`), key *storage* follows `string.mode` (pointer / `strdup` / `stralloc`) — so `memcmp` compares raw key bytes against a stored `char *` and never matches: every put inserts, every lookup and delete misses. 3 storage modes × 3 global seeds | `cfg_53_binary_mode_on_string_storage_table` | [x] |

| 54 | `stbds_hmdel_key` binary | a **consistent** non-zero `keyoffset` (8) on an `elemsize = 16, keysize = 4` element laid out `[key | pad | key-copy | value]`: `stbds_hmput_key` hard-codes `keyoffset = 0`, but the delete's `stbds_hm_find_slot` and its post-memmove re-index probe both read `elem + keyoffset`, so the copy makes them succeed. 6 sizes × 3 delete orders (forward / reverse / shuffled), covering both `old_index == final_index` and the memmove + re-index path | `cfg_54_del_consistent_keyoffset` | [x] |

### Deliberately excluded cell

`mode >= STBDS_HM_STRING` on a table whose `string.mode` is `STBDS_SH_NONE`
(e.g. `stbds_shmode_func(e, 0)` then string-mode puts) is the one remaining
cross-product cell and it is **not differentially testable**: the insert takes
`default:` and `memcpy`s `keysize` bytes of the key *string* into the element,
but `stbds_is_key_equal` then reads those bytes back as a `char *` and
dereferences them (C L561). The address is arbitrary key data, so the C's own
behaviour is undefined and depends on the process address space rather than on
the library. Note the first insert into an empty table never reaches
`stbds_is_key_equal` (it is only called on a hash match), so this cell cannot be
hit accidentally by the other rows.
