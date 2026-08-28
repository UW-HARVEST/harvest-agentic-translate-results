# CONFIGS.md — Configuration surface table (Phase A, gated in Phase B)

Derived mechanically from the branches in `c_src/src/lib.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` (int, passed across FFI) | `< STBDS_HM_STRING (i.e. <1)` → binary/`memcmp`+`hash_bytes`; `>= 1` → string/`strcmp`+`hash_string` | `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_hmput_key` |
| `mode == STBDS_HM_STRING` exactly | strdup key is freed on delete only when `mode == 1` (`==`, not `>=`) | `stbds_hmdel_key` |
| `table->string.mode` | `STBDS_SH_NONE(0)`/default → `memcpy(key)`; `STBDS_SH_DEFAULT(1)` → store caller pointer; `STBDS_SH_STRDUP(2)` → `stbds_strdup`; `STBDS_SH_ARENA(3)` → `stbds_stralloc`; anything else → `default:` `memcpy` | `stbds_hmput_key` switch, `stbds_hmfree_func`, `stbds_hmdel_key` |
| how the map is created | implicitly by `stbds_hmput_key(NULL,…)`; by `stbds_hmget_key*(NULL,…)` (array but **no** hash table); by `stbds_hmput_default(NULL,…)`; by `stbds_shmode_func(elemsize,mode)` | 4 distinct entry points |
| `elemsize` / `keysize` | any; `keysize==0`, `keysize<elemsize` (padding), `keysize==elemsize`, 1/4/8/16/20-byte elements | `memcpy`, `memcmp`, pointer arithmetic |
| table load | `used_count >= used_count_threshold` → grow `slot_count*2`; `used_count < used_count_shrink_threshold && slot_count > 8` → shrink `>>1`; `tombstone_count > tombstone_count_threshold` → rebuild same size | `stbds_hmput_key`, `stbds_hmdel_key` |
| `slot_count` | 8 (initial, `shrink_threshold` forced to 0), 16, 32, 64, … | `stbds_make_hash_index` |
| probe path | first inner loop (`i = pos&7 .. 7`) vs. wrap-around loop (`i = 0 .. pos&7`) vs. next bucket (`pos += step`) | `stbds_hm_find_slot`, `stbds_hmput_key` |
| delete position | `old_index == final_index` (last element) vs. `!=` (relocate final element + re-find its slot) | `stbds_hmdel_key` |
| `stbds_hash_bytes` `len` | `len % 8` ∈ 0..7 (switch fall-through), `len == 0`, `len < 8`, `len == 8`, `len > 8`; bytes with the high bit set at offsets 3 and 7 (sign-extension quirks) | `stbds_siphash_bytes` |
| `stbds_hash_string` length | 0, 1, …, long; bytes ≥ 0x80 (`(unsigned char)` cast) | `stbds_hash_string` |
| `seed` | arbitrary `size_t`, incl. 0 and `SIZE_MAX`; the library-global seed is advanced by every fresh `stbds_make_hash_index` and reset by `stbds_rand_seed` | `stbds_hash_*`, `stbds_make_hash_index` |
| `stbds_arrgrowf` | `a==NULL` vs. not; `min_cap <= arrcap` (early return); `min_cap < 2*arrcap` (doubling); `min_cap < 4` (bump to 4); `addlen` 0 vs. >0 | `stbds_arrgrowf` |
| arena `remaining` vs `len` | `len <= remaining` (bump alloc); `len > remaining && len > blocksize` (dedicated block, `a->storage` null vs. non-null); `len > remaining && len <= blocksize` (new block) | `stbds_stralloc` |
| arena `block` | 0…21 → `blocksize = 512<<(block>>1)` grows, `block` incremented; ≥22 → `blocksize == 1<<20`, `block` frozen | `stbds_stralloc` |
| `strkey` `n` | 0, positive, negative, `INT_MIN`, `INT_MAX` | `sprintf("%d")` |
| `arr_del` `num` | any `int`; the loop covers `i = 0,1,2,3` → `arrdel` at each index incl. the last (0-length `memmove`) | `arr_del` |

## Rows (each is a combination the C treats differently)

`R` = randomized/property-style over many inputs with a fixed seed.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p = NULL` | [x] |
| 2 | `stbds_hash_bytes` | `len ∈ 1..=7` (every `switch` fall-through case), random bytes, random seeds — R | [x] |
| 3 | `stbds_hash_bytes` | `len == 8` exactly (one main-loop iteration, `rem == 0`) — R | [x] |
| 4 | `stbds_hash_bytes` | `len ∈ 9..=64`, i.e. `k` main-loop iterations + every tail remainder — R | [x] |
| 5 | `stbds_hash_bytes` | bytes forced ≥ 0x80 at offsets 3 and 7 of every 8-byte group (`int` sign-extension quirk) — R | [x] |
| 6 | `stbds_hash_bytes` | `len ∈ 1..=8` with `d[3] ≥ 0x80` in the **tail** (`case 4..7` sign-extension) — R | [x] |
| 7 | `stbds_hash_bytes` | seeds `0`, `1`, `SIZE_MAX`, `0x31415926`, random — R | [x] |
| 8 | `stbds_hash_bytes` | large buffer (1 KiB … 4 KiB) — R | [x] |
| 9 | `stbds_hash_string` | empty string, random seeds — R | [x] |
| 10 | `stbds_hash_string` | ASCII strings length 1..=64, random seeds — R | [x] |
| 11 | `stbds_hash_string` | strings containing bytes 0x80..0xFF — R | [x] |
| 12 | `stbds_rand_seed` + `stbds_hmput_key(NULL,…)` | seed set explicitly; observe `table->seed` and the global seed advance over N successive fresh tables — R | [x] |
| 13 | `stbds_arrgrowf` | `a == NULL`, `elemsize ∈ {1,4,8,16,20}`, `addlen ∈ {0,1,3}`, `min_cap ∈ {0,1,4,5,17}` — full cross product | [x] |
| 14 | `stbds_arrgrowf` | existing array, `min_cap <= arrcap` → early return, pointer identity + header unchanged | [x] |
| 15 | `stbds_arrgrowf` | existing array, doubling path (`min_cap < 2*arrcap`) — repeated growth sequence, capacity trace compared | [x] |
| 16 | `stbds_arrgrowf` | existing array, `min_cap >= 2*arrcap` (jump growth) — R | [x] |
| 17 | `stbds_arrgrowf` + `stbds_arrfreef` | grow N times with random `addlen`, write payload each time, compare full payload + header, then free — R | [x] |
| 18 | `stbds_hmput_key` (binary, `mode=0`) | fresh map (`a=NULL`), `elemsize=8/keysize=4`, 1 insert | [x] |
| 19 | `stbds_hmput_key` (binary) | `elemsize=8/keysize=4`, N ∈ 1..=200 random `i32` keys (drives grow at 6, 12, 24 … used_count thresholds) — R | [x] |
| 20 | `stbds_hmput_key` (binary) | `elemsize=16/keysize=8` (`u64` keys) — R | [x] |
| 21 | `stbds_hmput_key` (binary) | `elemsize=20/keysize=8` (`stbds_struct2`-shaped, key smaller than element, padding) — R | [x] |
| 22 | `stbds_hmput_key` (binary) | `elemsize=1/keysize=1` (minimum sizes) — R | [x] |
| 23 | `stbds_hmput_key` (binary) | re-inserting **duplicate** keys (hits both the forward and the wrap-around duplicate branches) — R | [x] |
| 24 | `stbds_hmput_key` (binary) | keys chosen from a tiny domain so many collide in the same bucket → exercises `pos += step` re-probing — R | [x] |
| 25 | `stbds_hmget_key` (binary) | lookup of present and absent keys after N inserts; compare `temp` for every probe — R | [x] |
| 26 | `stbds_hmget_key_ts` (binary) | same as 25 but through the `ts` entry point with an explicit `temp` out-param — R | [x] |
| 27 | `stbds_hmget_key_ts` | `a == NULL` → allocates array with no hash table; then `hmget_key_ts` again on that array (table == NULL branch) | [x] |
| 28 | `stbds_hmdel_key` (binary) | delete the **last** element (`old_index == final_index`) | [x] |
| 29 | `stbds_hmdel_key` (binary) | delete a **middle** element (relocation + re-find) — R | [x] |
| 30 | `stbds_hmdel_key` (binary) | interleaved random put/get/del over 300 ops, full table snapshot after each op — R | [x] |
| 31 | `stbds_hmdel_key` (binary) | delete enough to trip `used_count < used_count_shrink_threshold && slot_count > 8` → shrink — R | [x] |
| 32 | `stbds_hmdel_key` (binary) | delete/insert cycles that trip `tombstone_count > tombstone_count_threshold` → same-size rebuild — R | [x] |
| 33 | `stbds_hmput_default` | `a == NULL`, `elemsize ∈ {1,4,8,16,20}` | [x] |
| 34 | `stbds_hmput_default` | existing map with `length != 0` → unchanged | [x] |
| 35 | `stbds_hmput_default` | array created by `stbds_arrgrowf` with `length == 0` → grow + `length = 1` | [x] |
| 36 | `stbds_hmput_default` then `stbds_hmput_key` | default element preserved (index 0) while real elements are appended — R | [x] |
| 37 | `stbds_shmode_func` | `mode = STBDS_SH_NONE(0)`, elemsize 16 | [x] |
| 38 | `stbds_shmode_func` | `mode = STBDS_SH_DEFAULT(1)` | [x] |
| 39 | `stbds_shmode_func` | `mode = STBDS_SH_STRDUP(2)` | [x] |
| 40 | `stbds_shmode_func` | `mode = STBDS_SH_ARENA(3)` | [x] |
| 41 | `stbds_shmode_func` + `stbds_hmput_key(mode=1)` | `STBDS_SH_DEFAULT` table, N random strings (caller keeps the storage) — R | [x] |
| 42 | `stbds_shmode_func` + `stbds_hmput_key(mode=1)` | `STBDS_SH_STRDUP` table, N random strings, compare stored key **contents** — R | [x] |
| 43 | `stbds_shmode_func` + `stbds_hmput_key(mode=1)` | `STBDS_SH_ARENA` table, N random strings (drives `stbds_stralloc` incl. block growth) — R | [x] |
| 44 | `stbds_shmode_func` + `stbds_hmput_key(mode=1)` | `STBDS_SH_NONE` table with string mode → `default:` branch copies the first `keysize` bytes of the *string itself* (not a pointer), while `stbds_is_key_equal` later re-interprets them as a `char*`. Only safe for pairwise hash-distinct keys — R | [x] |
| 44b | `stbds_shmode_func(DEFAULT/STRDUP/ARENA)` + `stbds_hmput_key(mode=0)` | key *storage* follows `table->string.mode` (a `char*` is stored) but comparison/hashing follows `mode==0` (`memcmp`/`hash_bytes` over the caller's key bytes) — every put therefore appends — R | [x] |
| 45 | `stbds_hmput_key(mode=1)` on `a == NULL` | implicit table creation sets `string.mode = STBDS_SH_DEFAULT` — R | [x] |
| 46 | `stbds_hmput_key(mode=0)` on `a == NULL` | implicit table creation sets `string.mode = 0` — R | [x] |
| 47 | `stbds_hmget_key(mode=1)` | string lookup: present / absent keys on DEFAULT, STRDUP and ARENA tables — R | [x] |
| 48 | `stbds_hmdel_key(mode=1)` | string delete on a `STBDS_SH_DEFAULT` table (no free) — R | [x] |
| 49 | `stbds_hmdel_key(mode=1)` | string delete on a `STBDS_SH_STRDUP` table (key **is** freed, then final element relocated) — R | [x] |
| 50 | `stbds_hmdel_key(mode=1)` | string delete on a `STBDS_SH_ARENA` table (no free) — R | [x] |
| 51 | `stbds_hmput_key` duplicate string key | duplicate hit in the **forward** loop → `table->temp_key` updated; duplicate hit in the **wrap-around** loop → `temp_key` NOT updated | [x] |
| 52 | `stbds_hmfree_func` | binary map | [x] |
| 53 | `stbds_hmfree_func` | `STBDS_SH_DEFAULT` string map | [x] |
| 54 | `stbds_hmfree_func` | `STBDS_SH_STRDUP` string map (frees every stored key) | [x] |
| 55 | `stbds_hmfree_func` | `STBDS_SH_ARENA` string map (`stbds_strreset` on the arena) | [x] |
| 56 | `stbds_hmfree_func` | array with `hash_table == NULL` (from `stbds_hmget_key_ts(NULL,…)`) | [x] |
| 57 | `stbds_stralloc` | fresh arena (`{0,0,0,0}`), short string (`len <= 512`) → new 512-byte block, `remaining` trace | [x] |
| 58 | `stbds_stralloc` | fresh arena, string longer than 512 → dedicated block, `a->storage == NULL` branch, `remaining = 0` | [x] |
| 59 | `stbds_stralloc` | non-empty arena, string longer than `blocksize` → dedicated block spliced after `a->storage`, `remaining` untouched | [x] |
| 60 | `stbds_stralloc` | many strings until `block` reaches 22 / `blocksize` saturates at `1<<20` — R | [x] |
| 61 | `stbds_stralloc` | random mix of short/long strings, `remaining`/`block`/returned contents compared after each call — R | [x] |
| 62 | `stbds_strreset` | arena with 0, 1 and many blocks → all fields zeroed | [x] |
| 63 | `strkey` | `n ∈ {0, 1, -1, 9, 10, 99, 100, INT_MIN, INT_MAX}` + random `i32` — R | [x] |
| 64 | `arr_del` | `num ∈ {0, 1, -1, INT_MIN, INT_MAX}` + random `i32` — R | [x] |
| 65 | `stbds_hmput_key` | `keysize == 0` (degenerate key) — every key aliases | [x] |
| 66 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | out-of-enum `mode` values `2, 7, 255, INT_MAX` (string path) and `-1, INT_MIN` (binary path) — R | [x] |
| 67 | `stbds_shmode_func` | out-of-enum `mode` values `4, 7, 255, 256, -1, INT_MIN` → `(unsigned char)mode`, `default:` branch | [x] |
| 68 | `stbds_hmput_key` + `stbds_hmdel_key` | large stress: 2000 mixed ops on a binary map, snapshot every 25 ops (drives grow at 8→16→32→64→128, shrink, rebuild) — R | [x] |
| 69 | `stbds_hmput_key` + `stbds_hmdel_key` | large stress: 1000 mixed ops on a `STBDS_SH_STRDUP` string map — R | [x] |
| 70 | full pipeline | `shmode_func(ARENA)` → N `hmput_key` → M `hmdel_key` → `hmget_key` for all keys → `hmfree_func`; compare snapshot at every step — R | [x] |

## Additional rows (added after coverage analysis — see `VERIFICATION.md`)

Mutation testing showed that the multi-bucket probe walk

```c
pos += step;  step += STBDS_BUCKET_LENGTH;  pos &= (table->slot_count-1);
```

— which exists in **three** places (`stbds_hm_find_slot`, `stbds_hmput_key`,
`stbds_make_hash_index`) — is essentially unreachable by property testing,
because it needs two *consecutive* completely-full 8-slot buckets and the table
never exceeds 75 % load.  Rows 71..=77 therefore build the bucket array by hand,
byte-identically in the C map and the Rust map, and then drive the real exported
entry points over it (`tests/probe_paths.rs`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 71 | `stbds_hmget_key` / `stbds_hmget_key_ts` | crafted table: HIT after **two** bucket hops (`step` 8 then 16) | [x] |
| 72 | `stbds_hmget_key` / `stbds_hmget_key_ts` / `stbds_hmdel_key` | crafted table: MISS after two bucket hops | [x] |
| 73 | `stbds_hmput_key` | crafted table: INSERT reaches `found_empty_slot` from the third bucket | [x] |
| 74 | `stbds_hmput_key` | crafted table: two hops **and** a tombstone recorded in the first bucket → `pos = tombstone`, `--tombstone_count` | [x] |
| 75 | `stbds_hm_find_slot` + `stbds_hmput_key` | crafted table: duplicate key found in the **wrap-around** inner loop (`i < limit`) | [x] |
| 76 | `stbds_hmput_key` → `stbds_make_hash_index` | crafted table: GROW 32→64 where 17 old entries all want bucket 0 → the rehash walk hops twice | [x] |
| 77 | `stbds_hmdel_key` → `stbds_make_hash_index` | crafted table: SHRINK 64→32 with the same 17-entry pile-up | [x] |
| 78 | `stbds_arrgrowf` | `elemsize == 0` (only the header is allocated) × the full addlen/min_cap cross product | [x] |
| 79 | `stbds_hash_string` | long strings (255, 256, 511, 512, 1024, 4095, 4096, 8193) incl. bytes ≥ 0x80 | [x] |
| 80 | `stbds_stralloc` / `stbds_strreset` | arena whose `mode` byte is non-zero (1,2,3,7,255): `stralloc` must not touch it, `strreset` must zero it | [x] |
| 81 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` | `keysize` 16 and 24 (2 and 3 SipHash main-loop iterations per key) with `elemsize` 24/32, plus `elemsize == 12` (odd stride) | [x] |
| 82 | `stbds_hmdel_key` | `keyoffset != 0` (8), with each element's value half kept byte-identical to its key so the offset is consistent | [x] |

## Branches that are provably untestable

| branch | why |
|--------|-----|
| `if (hash < 2) hash += 2;` in `stbds_hm_find_slot` / `stbds_hmput_key` | requires a full 64-bit SipHash/`hash_string` output of exactly 0 or 1; probability 2⁻⁶³. Both implementations contain the identical statement (verified by inspection). |
| `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | the only `slot_count` values that reach `stbds_make_hash_index` are 8, `slot_count*2` and `slot_count>>1` (guarded by `slot_count > 8`), for all of which the invariant holds. |
| `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` in `stbds_hmput_key` | `stbds_arrgrowf(a, elemsize, 1, 0)` is called immediately before whenever the capacity is short. |
| `STBDS_ASSERT(len <= a->remaining)` in `stbds_stralloc` | either the over-sized branch returned early, or `remaining` was just set to `blocksize >= len`. |
| `void arr_del(int)` result | returns `void`, allocates and frees everything it touches, and mutates no global. The only observable property is "does not fault / corrupt the heap", which row 64 checks. |
