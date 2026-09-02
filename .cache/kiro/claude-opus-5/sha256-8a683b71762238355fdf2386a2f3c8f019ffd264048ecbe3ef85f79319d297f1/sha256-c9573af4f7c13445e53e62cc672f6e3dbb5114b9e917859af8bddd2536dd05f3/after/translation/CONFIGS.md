# CONFIGS.md — configuration surface table (Phase B gate)

Axes the C code actually branches on (derived from `c_src/src/lib.c`):

* **entry point** — the 16 exported symbols (`SYMBOLS.md`). The *low-level*
  `stbds_*_key` functions are the real API; the `hmput`/`shput`/`arrput` macros in
  the header are thin wrappers, so the tests drive the low-level functions
  directly and re-implement the macro glue in the harness.
* **`mode`** (`int`) — `mode >= STBDS_HM_STRING(1)` ⇒ string hashing + `strcmp`;
  `mode < 1` ⇒ byte hashing + `memcmp`. `stbds_hmdel_key` additionally tests
  `mode == 1` *exactly* for the strdup-free path.
* **`string.mode`** (`unsigned char`, the `switch` in `stbds_hmput_key`) —
  `STBDS_SH_NONE(0)` ⇒ `memcpy` key bytes, `STBDS_SH_DEFAULT(1)` ⇒ store caller
  pointer, `STBDS_SH_STRDUP(2)` ⇒ `stbds_strdup`, `STBDS_SH_ARENA(3)` ⇒
  `stbds_stralloc`, anything else ⇒ `default:` = `memcpy`. Set either implicitly
  by `hmput_key` on a fresh table, or explicitly by `stbds_shmode_func`.
* **`elemsize` / `keysize` / `keyoffset`** — arbitrary; the sizes select
  different `siphash` paths (`keysize` 0..7 = tail-only, 8 = one block,
  9..15, 16, 24, 31, 32, …) and different `memcmp` widths.
* **table size / load** — `slot_count` starts at 8 and doubles when
  `used_count >= used_count_threshold`; it halves when
  `used_count < used_count_shrink_threshold && slot_count > 8`; it is rebuilt in
  place when `tombstone_count > tombstone_count_threshold`. Element counts
  0 / 1 / 6 / 7 / 8 / 100 / 1000 cross all three.
* **probe shape** — first inner loop (`i = pos&7 .. 7`) vs. the wrap-around loop
  (`i = 0 .. pos&7`) vs. multi-bucket probing (`pos += step; step += 8`).
  Reached by filling buckets, i.e. by element count and by seed.
* **seed** — `stbds_rand_seed()`; the global seed is also advanced by every
  `stbds_make_hash_index(_, NULL)`, so seed state is part of the configuration.
* **array shape** (`stbds_arrgrowf`) — `a == NULL` vs. non-NULL, `addlen`
  0/1/n, `min_cap` below / equal / above capacity, `elemsize` 0/1/4/8/16/64.
* **arena shape** (`stbds_stralloc`) — `remaining` sufficient vs. not,
  `len <= blocksize` vs. `len > blocksize`, `storage == NULL` vs. not,
  `block` 0..23 (saturating) and 24..255 (shift-count overflow).

Every row is exercised with many randomized inputs from a fixed-seed
`SplitMix64` PRNG (`tests/common/mod.rs`), and every row compares C vs. Rust
byte-for-byte over: the returned array elements, the array header
(`length`, `capacity`, `temp`), and the whole `stbds_hash_index` scalar state
including all bucket `hash[]` / `index[]` slots.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len` = 0..64, random bytes, random seed (all siphash tail cases 0..7 and 1..8 blocks) | [x] |
| 2 | `stbds_hash_bytes` | bytes with the high bit set at offsets 3 and 7 (sign-extension quirk in the `int` block load) | [x] |
| 3 | `stbds_hash_bytes` | `len` 65..600, random (many-block loop) | [x] |
| 4 | `stbds_hash_string` | random NUL-terminated strings, length 0..64, random seed | [x] |
| 5 | `stbds_hash_string` | strings containing bytes 0x80..0xFF | [x] |
| 6 | `stbds_rand_seed` + `stbds_shmode_func` | seed = 0, 1, 0x31415926, `usize::MAX`, random; observe the seed LCG through consecutive fresh tables (`seed` field + bucket layout) | [x] |
| 7 | `stbds_arrgrowf` | `a == NULL`, `elemsize` ∈ {1,4,8,16,64}, `addlen` ∈ {0,1,2,7}, `min_cap` ∈ {0,1,4,5,100} | [x] |
| 8 | `stbds_arrgrowf` | grow an existing array repeatedly (doubling path `min_cap < 2*cap`), random `addlen` | [x] |
| 9 | `stbds_arrgrowf` | existing array, `min_cap <= cap` (no-op path) | [x] |
| 10 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` (jump path) | [x] |
| 11 | `stbds_arrgrowf` + `stbds_arrfreef` | alloc/grow/free cycle, verify header after each step | [x] |
| 12 | `arr_ins` | `num` ∈ {0,1,4,-1,`INT_MIN`,`INT_MAX`} + random (asserts must hold in both) | [x] |
| 13 | `strkey` | `n` ∈ {0,-1,`INT_MIN`,`INT_MAX`} + random; compare returned C string bytes | [x] |
| 14 | `stbds_hmput_default` | fresh (`NULL`), then again on the same pointer (no-op), `elemsize` ∈ {8,16,32} | [x] |
| 15 | `stbds_hmput_key` binary, `elemsize=8`/`keysize=4` (`{i32 key; i32 val}`) | insert 0/1/6/7/8/100/1000 random keys — crosses the first table growth (8→16) and several more | [x] |
| 16 | `stbds_hmput_key` binary, `elemsize=16`/`keysize=8` (`{i64 key; i64 val}`) | insert 1000 random keys | [x] |
| 17 | `stbds_hmput_key` binary, `elemsize=24`/`keysize=8`, `keyoffset=0` (`{i32 key[2]; i32 b,c,d}` shape) | insert 500 random 2-word keys | [x] |
| 18 | `stbds_hmput_key` binary | re-put existing keys (found-key path, both inner loops), interleaved with new keys | [x] |
| 19 | `stbds_hmget_key` binary | lookup present and absent keys after each insert batch; compare `temp` | [x] |
| 20 | `stbds_hmget_key_ts` binary | same as 19 but through the `temp` out-parameter; also `a == NULL` | [x] |
| 21 | `stbds_hmdel_key` binary, `keyoffset=0` | delete random subsets; crosses the swap-with-last path, the shrink path and the tombstone-rebuild path | [x] |
| 22 | `stbds_hmdel_key` binary, `keyoffset != 0` (`{i32 pad; i32 key; …}`) | delete random subsets | [x] |
| 23 | `stbds_hmput_key` after deletes | tombstone reuse (`tombstone >= 0` at `found_empty_slot`) | [x] |
| 24 | full binary map lifecycle + `stbds_hmfree_func` | put/get/del/free, `elemsize` ∈ {8,16,24}, 3 random seeds | [x] |
| 25 | `stbds_hmput_key` string, implicit `SH_DEFAULT` (fresh table, `mode=1`) | insert 200 random keys, caller owns the key storage | [x] |
| 26 | `stbds_shmode_func(SH_STRDUP)` + `stbds_hmput_key` string | insert 200 random keys; keys are `strdup`ed, compare string contents; then `hmfree_func` (frees the dups) | [x] |
| 27 | `stbds_shmode_func(SH_ARENA)` + `stbds_hmput_key` string | insert 200 random keys of length 1..40 (single arena block) | [x] |
| 28 | `stbds_shmode_func(SH_ARENA)` + `stbds_hmput_key` string | keys longer than the first arena blocksize (512) ⇒ dedicated-block path, mixed with short keys | [x] |
| 29 | `stbds_shmode_func(SH_NONE)` + `stbds_hmput_key` `mode=1` | `string.mode == 0` ⇒ `default:` `memcpy` branch **with** string hashing | [x] |
| 30 | `stbds_shmode_func(4)` (undefined `string.mode`) + `stbds_hmput_key` `mode=1` | `default:` `memcpy` branch | [x] |
| 31 | string map + `stbds_hmget_key` `mode=1` | lookup present / absent / prefix / suffix keys | [x] |
| 32 | string map + `stbds_hmdel_key` `mode=1` | delete subsets in each of the 4 `string.mode`s (0,1,2,3) | [x] |
| 33 | string map + `stbds_hmdel_key` `mode=2` | `mode>=1` (string hashing) but `mode!=1` ⇒ no strdup free **and** binary re-find path | [x] |
| 34 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | `mode` ∈ {-2147483648, -1, 0, 1, 2, 7, 2147483647} (out-of-range enum values across FFI) | [x] |
| 35 | `stbds_shmode_func` | `mode` ∈ {0,1,2,3,4,255,256,259,-1,`INT_MIN`,`INT_MAX`} (truncating cast) | [x] |
| 36 | `stbds_stralloc` | fresh arena, keys of length 1..600 in random order (fills blocks, grows `block`) | [x] |
| 37 | `stbds_stralloc` | strings longer than the current blocksize (dedicated block), first-with-`storage==NULL` and later-with-`storage!=NULL` | [x] |
| 38 | `stbds_stralloc` | pre-set `a->block` ∈ {0,1,10,11,22,23,24,63,64,127,128,200,255} (blocksize saturation + shift-count masking) | [x] |
| 39 | `stbds_strreset` | empty arena, arena with 1 block, arena with many blocks, double reset | [x] |
| 40 | `stbds_hmput_key` with `keysize` 0..40 | selects every siphash tail/block case inside the map | [x] |
| 41 | end-to-end randomized model check, binary map | 5 000 random ops (put / get / get_ts / del / put_default) against a shadow model, compared C vs Rust after every op | [x] |
| 42 | end-to-end randomized model check, string map × `string.mode` ∈ {0,1,2,3} | 2 000 random ops each | [x] |
| 43 | binary map, `keysize` ∈ {1,2} | tiny keyspace (≤ 256 / ≤ 65536 distinct keys) × 4 000 random put/get/get_ts/del ops × 3 seeds — dominated by the "key already present" branch and by tombstone churn | [x] |
| 44 | binary map, `elemsize=16`/`keysize=8` | fill to 760 elements (1024-slot table) then hover at ~75 % load for 3 000 delete+insert+hit+miss cycles × 5 seeds — forces the bucket-tail loop, the wrap-around loop `i in 0..pos&7`, and multi-bucket probing `pos += step; step += 8` | [x] |
| 45 | string map × `string.mode` ∈ {1,2,3} | same high-load hovering with 1 200 keys of length 0..60, checking `temp_key` after every put | [x] |

## Result

45 configuration rows, every one exercised against BOTH `.so`s with a fixed-seed
PRNG and compared byte-for-byte (`tests/phase_b.rs`).  **0 divergences remain.**

Each row's comparison covers, after every operation:

* every element byte of the array (`length * elemsize`), or - for string maps,
  where the key slot holds a non-comparable pointer - the key's string CONTENT
  plus the value bytes;
* the array header: `length`, `capacity`, `temp`, and whether `hash_table` is set;
* the whole `stbds_hash_index`: `slot_count`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, `string.remaining`,
  `string.block`, `string.mode`, and every `hash[]` / `index[]` slot of every
  bucket;
* the value returned by the call (index / `temp` / hash / string bytes).
