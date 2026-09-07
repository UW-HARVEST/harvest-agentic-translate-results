# CONFIGS.md — configuration-surface table (Phase B gate)

Axes mechanically derived from the branches in `c_src/src/lib.c`:

* **`mode` argument** (`hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key`):
  branch `mode >= STBDS_HM_STRING` (hash_string+strcmp vs hash_bytes+memcmp)
  and the distinct branch `mode == STBDS_HM_STRING` (only in `hmdel_key`).
* **`table->string.mode`** (`switch` in `hmput_key`, `if` in `hmfree_func`,
  `if` in `hmdel_key`): `STBDS_SH_NONE(0)` / `STBDS_SH_DEFAULT(1)` /
  `STBDS_SH_STRDUP(2)` / `STBDS_SH_ARENA(3)`; set implicitly by `hmput_key`
  (0 or 1) or explicitly by `stbds_shmode_func`.
* **`elemsize`** — element stride; `hash_to_arr`/`arr_to_hash`/`elem_at` all key off it.
* **`keysize`** — `memcmp`/`memcpy` width in binary mode (1/2/4/8/16/…), and the
  `sizeof(size_t)`-block structure inside siphash.
* **`len` for `hash_bytes`** — `len % 8` selects one of the 8 fall-through
  `switch` arms; `len / 8` the number of full siphash blocks; byte values with
  the high bit set select the sign-extension paths.
* **table size transitions** — `used_count >= used_count_threshold` (grow ×2),
  `used_count < used_count_shrink_threshold && slot_count > 8` (shrink ÷2),
  `tombstone_count > tombstone_count_threshold` (rebuild same size).
* **arena block progression** — `blocksize = 512 << (block>>1)`, the
  `len > blocksize` over-sized-block branch, `a->storage == NULL` vs not, and
  `blocksize >= 1<<20` saturation.
* **array growth** — `a == NULL` vs not, `addlen`/`min_cap` relation
  (`min_len > min_cap`, `min_cap <= cap`, `min_cap < 2*cap`, `min_cap < 4`).
* **global seed** — default `0x31415926` vs `stbds_rand_seed(x)`; every
  `make_hash_index(_, NULL)` advances it (`seed = seed*a + b`).

`Cargo.toml` has **no `[features]`**, so the single configuration column
"feature combo" is constant (`default` == `--no-default-features`).
The cmake project builds **only a shared library** — there is no driver binary,
so the "compare binary stdout" gate is satisfied by capturing `str_put`'s
`printf` output through a redirected fd 1 (rows 40–42).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0` (no full block, `switch` arm 0) | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7`, random bytes incl. `>= 0x80` (switch arms 1–7, no full block) | [x] |
| 3 | `stbds_hash_bytes` | `len = 8` exactly (one full block, arm 0) | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..15` (one full block + arms 1–7) | [x] |
| 5 | `stbds_hash_bytes` | `len = 16,24,…,256` multiples of 8 (many full blocks) | [x] |
| 6 | `stbds_hash_bytes` | `len` large & non-multiple (e.g. 1000, 4095) | [x] |
| 7 | `stbds_hash_bytes` | all-`0xFF` / all-`0x80` buffers (max sign-extension) | [x] |
| 8 | `stbds_hash_bytes` | `seed = 0`, `1`, `SIZE_MAX`, random 64-bit | [x] |
| 9 | `stbds_hash_string` | `""` (empty) | [x] |
| 10 | `stbds_hash_string` | 1-byte, ASCII | [x] |
| 11 | `stbds_hash_string` | random ASCII, len 1..64 | [x] |
| 12 | `stbds_hash_string` | bytes `0x80..0xFF` (the `(unsigned char)` cast path) | [x] |
| 13 | `stbds_hash_string` | long strings (1 KiB) so the rotate-add loop saturates | [x] |
| 14 | `stbds_hash_string` | `seed = 0`, `SIZE_MAX`, random | [x] |
| 15 | `stbds_rand_seed` + `stbds_shmode_func` | seed evolution across N successive index creations (`seed*a+b`) | [x] |
| 16 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 0` → returns NULL, no alloc | [x] |
| 17 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 1` → `cap = 4` | [x] |
| 18 | `stbds_arrgrowf` | `a = NULL`, `addlen = 1..3`, `min_cap = 0` → `cap = 4` | [x] |
| 19 | `stbds_arrgrowf` | `a = NULL`, `addlen = 5..1000` → `cap = addlen` | [x] |
| 20 | `stbds_arrgrowf` | `a = NULL`, `min_cap` large, `addlen` small → `cap = min_cap` | [x] |
| 21 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → returns same pointer, no change | [x] |
| 22 | `stbds_arrgrowf` | existing array, `min_cap` in `(cap, 2*cap)` → `cap = 2*cap` | [x] |
| 23 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` → `cap = min_cap` | [x] |
| 24 | `stbds_arrgrowf` | repeated doubling chain (push-like growth, 1..500 elements) | [x] |
| 25 | `stbds_arrgrowf` | `elemsize` = 1,2,4,8,16,24,32,64 | [x] |
| 26 | `stbds_stralloc` | fresh arena (`block=0`, `remaining=0`), short string → 512-byte block | [x] |
| 27 | `stbds_stralloc` | many short strings filling & re-allocating blocks (block progression 0→N) | [x] |
| 28 | `stbds_stralloc` | `len > blocksize` on empty arena → dedicated block, `remaining = 0` | [x] |
| 29 | `stbds_stralloc` | `len > blocksize` on non-empty arena → spliced block, `remaining` kept | [x] |
| 30 | `stbds_stralloc` | mixed short/over-sized sequence (randomized lengths 0..3000) | [x] |
| 31 | `stbds_stralloc` + `stbds_strreset` | reset a multi-block arena, then reuse it | [x] |
| 32 | `stbds_strreset` | arena with `storage == NULL` | [x] |
| 33 | `stbds_hmput_default` | `a = NULL`, various `elemsize` | [x] |
| 34 | `stbds_hmput_default` | `a` non-NULL with `length != 0` → unchanged | [x] |
| 35 | `stbds_hmput_default` | after `hmput_key`, then `hmput_default` (no-op path) | [x] |
| 36 | `stbds_hmget_key_ts` | `a = NULL` (bootstrap + `*temp = -1`) | [x] |
| 37 | `stbds_hmget_key_ts` | `a` from `hmput_default` (table == NULL) → `*temp = -1` | [x] |
| 38 | `stbds_hmget_key` | same shapes as 36/37 but result observed through `header->temp` | [x] |
| 39 | `str_put` | `num = 0` (no arena churn) — stdout compared byte-for-byte | [x] |
| 40 | `str_put` | `num = 1,2,3,7,8,64,100` — stdout compared byte-for-byte | [x] |
| 41 | `str_put` | `num` large (5000) so the arena grows through several block sizes | [x] |
| 42 | `str_put` | `num < 0` (`-1`, `INT_MIN`) | [x] |
| 43 | `strkey` | `n = 0, 1, -1, INT_MAX, INT_MIN` → identical `char*` contents | [x] |
| 44 | binary map: `hmput_key` | `mode = STBDS_HM_BINARY`, `keysize = 4`, `elemsize = 8`, 1 insert | [x] |
| 45 | binary map: `hmput_key` | `mode = 0`, `keysize = 4`, `elemsize = 16`, inserts 1..8 (no grow → grow at 6) | [x] |
| 46 | binary map: `hmput_key` | `mode = 0`, `keysize = 4`, 1..300 random inserts (multiple ×2 growths) | [x] |
| 47 | binary map: `hmput_key` | `mode = 0`, `keysize = 8` (`size_t`/pointer keys), 200 inserts | [x] |
| 48 | binary map: `hmput_key` | `mode = 0`, `keysize = 1` (byte keys → forced collisions/duplicates) | [x] |
| 49 | binary map: `hmput_key` | `mode = 0`, `keysize = 16`, `elemsize = 24` (multi-block siphash key) | [x] |
| 50 | binary map: `hmput_key` | duplicate keys re-inserted (upper-scan hit path, `temp` reused) | [x] |
| 51 | binary map: `hmput_key` | keys engineered to wrap the bucket (`pos & MASK != 0` → lower-scan hit path) | [x] |
| 52 | binary map: `hmget_key` | hit / miss on a populated table, `mode = 0` | [x] |
| 53 | binary map: `hmget_key_ts` | hit / miss with external `temp`, `mode = 0` | [x] |
| 54 | binary map: `hmdel_key` | delete existing key, `old_index == final_index` (last element) | [x] |
| 55 | binary map: `hmdel_key` | delete existing key, `old_index != final_index` (swap + re-lookup) | [x] |
| 56 | binary map: `hmdel_key` | delete missing key (`temp = 0`, unchanged) | [x] |
| 57 | binary map: `hmdel_key` | delete until shrink triggers (`used_count < slot_count>>2`, `slot_count > 8`) | [x] |
| 58 | binary map: `hmdel_key` | delete/insert churn triggering tombstone rebuild (`tombstone_count > threshold`) | [x] |
| 59 | binary map: full pipeline | randomized put/get/del sequence, 2000 ops, `keysize = 4`, compare full state after every op | [x] |
| 60 | binary map: full pipeline | same with `keysize = 8`, `elemsize = 32` | [x] |
| 61 | string map (`SH_DEFAULT`): `hmput_key` | `mode = STBDS_HM_STRING`, table auto-created → `string.mode = SH_DEFAULT` | [x] |
| 62 | string map (`SH_DEFAULT`) | 1..200 distinct random keys (grow ×2 several times) | [x] |
| 63 | string map (`SH_DEFAULT`) | duplicate key re-put (upper scan: `temp_key` written) | [x] |
| 64 | string map (`SH_DEFAULT`) | `hmget_key` hit/miss, `mode = 1` | [x] |
| 65 | string map (`SH_DEFAULT`) | `hmdel_key` `mode = 1` (no strdup free), swap path | [x] |
| 66 | string map (`SH_STRDUP`) via `shmode_func(elemsize, 2)` | inserts, keys duplicated with `strdup` | [x] |
| 67 | string map (`SH_STRDUP`) | 1..200 random keys, grow ×2 | [x] |
| 68 | string map (`SH_STRDUP`) | `hmdel_key` with `mode == 1` → frees stored key; swap + re-lookup | [x] |
| 69 | string map (`SH_STRDUP`) | `hmdel_key` with `mode == 2` → does **not** free, re-lookup via memcmp | [x] |
| 70 | string map (`SH_STRDUP`) | `hmfree_func` sweeps and frees all duplicated keys | [x] |
| 71 | string map (`SH_ARENA`) via `shmode_func(elemsize, 3)` | inserts, keys copied into the arena | [x] |
| 72 | string map (`SH_ARENA`) | 1..200 random keys of mixed length incl. > 512 (arena over-sized block) | [x] |
| 73 | string map (`SH_ARENA`) | `hmdel_key` + `hmfree_func` (arena `strreset`) | [x] |
| 74 | string map (`SH_NONE`) via `shmode_func(elemsize, 0)` | `mode = 1` puts hit the `default:` `memcpy` arm (binary copy of the pointer) | [x] |
| 75 | `shmode_func` | `elemsize` = 8,16,24,32 × mode = 0,1,2,3 cross-product | [x] |
| 76 | full pipeline | `shmode_func(_,2)` + randomized put/get/del 1000 ops, state compared each op | [x] |
| 77 | full pipeline | `shmode_func(_,3)` + randomized put/get/del 1000 ops | [x] |
| 78 | full pipeline | `SH_DEFAULT` string map + randomized put/get/del 1000 ops | [x] |
| 79 | seed interaction | `rand_seed(k)` before building maps, k ∈ {0,1,0x31415926,SIZE_MAX,random} × binary/string map | [x] |
| 80 | `hmfree_func` | table == NULL (from `hmput_default`) and table != NULL, each × `string.mode` 0..3 | [x] |

## Result

All 80 rows checked off. Every row is driven through **both** `.so` exports
(loaded with `libloading`; nothing is called directly) with many randomized
inputs per row from a fixed-seed xorshift RNG, and the *entire* observable state
is compared after **every** operation: the `stbds_array_header`
(`length`/`capacity`/`temp`/`hash_table` presence), each element's defined byte
ranges, and the whole `stbds_hash_index` including every bucket's `hash[8]` and
`index[8]`.

Test files and the rows they own:

| rows | file | tests |
|------|------|-------|
| 1–15  | `tests/phase_b_hash.rs`   | 15 |
| 16–32 | `tests/phase_b_arr.rs`    | 17 |
| 33–38, 44–80 | `tests/phase_b_map.rs` | 43 |
| 39–43 | `tests/phase_b_strput.rs` | 7 |

Because `stbds_hash_seed` is a mutable global *inside each `.so`* that
`stbds_make_hash_index` advances, every seed-sensitive test takes a process-wide
lock (`common::serial()`) and calls `common::reseed()` to put both libraries into
the same state first.

Note on byte-exactness: `stbds_arrgrowf` never zeroes the payload it `realloc`s,
so the padding between a key and a value (and anything past the value) is
genuinely uninitialised in C. The snapshot compares only the byte ranges the
library or the caller writes — comparing the garbage would test the allocator,
not the translation.

Reproduce with `./verify.sh` (builds both `.so`s, diffs `nm -D`, sweeps every
feature combination).
