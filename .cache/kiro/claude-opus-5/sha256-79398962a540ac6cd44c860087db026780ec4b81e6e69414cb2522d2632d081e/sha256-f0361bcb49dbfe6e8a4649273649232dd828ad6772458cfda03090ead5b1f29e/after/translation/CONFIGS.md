# CONFIGS.md — configuration / valid-input surface table (Phase B)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `mode` (lookup/insert/delete) | `STBDS_HM_BINARY` (0), `STBDS_HM_STRING` (1) — tested as `mode >= 1` / `mode == 1` | `stbds_is_key_equal`, `hm_find_slot`, `hmput_key`, `hmdel_key` |
| `table->string.mode` | `SH_NONE`(0) `SH_DEFAULT`(1) `SH_STRDUP`(2) `SH_ARENA`(3) | `switch` in `hmput_key`, `hmfree_func`, `hmdel_key` |
| map bootstrap path | `hmput_key(NULL,…)` / `hmput_default(NULL,…)` / `shmode_func(elemsize,mode)` | 3 distinct entry points |
| `elemsize` | key-only (8), key+int (16, padded), key+payload (24, 32), non-power-of-2 (12, 20), `1`, `0` | pervasive `elemsize*i` arithmetic |
| `keysize` | 1, 2, 4, 8 (pointer/`size_t`), 16, 3/5/7 (odd tails) | `memcmp`, `hash_bytes` |
| `keyoffset` (delete only) | `0` and non-zero (`STBDS_OFFSETOF`) | `hmdel_key` only — `hmput/hmget` hard-code `0` |
| table `slot_count` | 8 → 16 → 32 → 64 → 128 (grow when `used_count >= slot_count - slot_count/4`) | `hmput_key` grow branch |
| delete aftermath | (a) neither, (b) **shrink** (`used_count < slot_count>>2 && slot_count>8`), (c) **tombstone rebuild** (`tombstone_count > (sc>>3)+(sc>>4)`) | `hmdel_key` tail |
| delete position | `old_index == final_index` (last element) vs `!=` (swap-with-last + re-find) | `hmdel_key` |
| tombstone reuse on insert | `tombstone >= 0` (reuse) vs `-1` (fresh empty slot) | `hmput_key` `found_empty_slot` |
| probe path | in-bucket hit, wrap-around (`0..limit`) loop, multi-bucket quadratic probe (`step += 8`) | both probe loops |
| `arrgrowf` growth rule | `min_cap <= cap` (no-op) · `min_cap < 2*cap` (double) · `min_cap < 4` (bump to 4) · else exact | `stbds_arrgrowf` |
| arena block size | `512 << (block>>1)`: 512, 512, 1024, 1024, …, saturating at 1 MiB; small-fit vs new-block vs oversized-block | `stbds_stralloc` |
| `hash_bytes` shape | `len` a multiple of 8 (main loop only), `len % 8 != 0` (tail `switch` fall-through 7…1), `len < 8` (tail only), `len == 0` | `stbds_siphash_bytes` |
| byte values | `< 0x80` vs `>= 0x80` at `d[3]`/`d[7]` (sign-extension), all-zero, all-`0xFF` | `stbds_siphash_bytes` loader |
| `seed` | `0`, `1`, `0x31415926` (default), `SIZE_MAX`, random | `hash_string`, `hash_bytes` |
| global seed state | untouched (default `0x31415926`) vs `stbds_rand_seed(x)`; advances `seed*a+b` per fresh index | `stbds_make_hash_index` |
| `sh_puts` `num` | `≤0`, 1, small, ≥ 512-byte-block-crossing, large | `sh_puts` stralloc loop |

## Rows (one per combination the C treats differently)

Each row is driven with **many** randomized inputs (fixed seed
`0x5eed_1234`), through the `.so` exports of both libraries.

### Pure hashing (lowest level)

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len == 0`, seeds {0,1,default,SIZE_MAX,random×64} | [x] |
| 2 | `stbds_hash_bytes` | `len` 1..7 (tail-only, every `switch` fall-through arm), random bytes | [x] |
| 3 | `stbds_hash_bytes` | `len == 8`, 16, 24, 32, 64 (exact multiples — main loop only) | [x] |
| 4 | `stbds_hash_bytes` | `len` 9..71 non-multiples (main loop **+** tail) | [x] |
| 5 | `stbds_hash_bytes` | high-bit bytes forced at `d[3]`/`d[7]` (sign-extension path) | [x] |
| 6 | `stbds_hash_bytes` | all-`0x00` and all-`0xFF` buffers, len 0..40 | [x] |
| 7 | `stbds_hash_string` | empty string, seeds {0,1,default,SIZE_MAX} | [x] |
| 8 | `stbds_hash_string` | ASCII strings len 1..64, random | [x] |
| 9 | `stbds_hash_string` | strings with bytes `0x80..0xFF` (`(unsigned char)` cast path) | [x] |
| 10 | `stbds_rand_seed` + `stbds_shmode_func` | seed the global, then observe the seed that lands in a fresh index (through `hash`-visible behaviour) | [x] |

### Dynamic array core

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 11 | `stbds_arrgrowf` | `a=NULL`, random `elemsize` ∈{1,4,8,12,16,20,32}, `addlen`∈{0..8}, `min_cap`∈{0..8} → capacity/length/temp/hash_table | [x] |
| 12 | `stbds_arrgrowf` | grow chain: repeated calls on the returned pointer, `addlen=1`, `min_cap=0` (doubling rule 4→8→16…) | [x] |
| 13 | `stbds_arrgrowf` | `min_cap <= arrcap(a)` — no-op branch, pointer identity preserved | [x] |
| 14 | `stbds_arrgrowf` | `min_cap` between `cap` and `2*cap` (rounds up to `2*cap`) and `> 2*cap` (exact) | [x] |
| 15 | `stbds_arrgrowf` + `stbds_arrfreef` | alloc → write payload → free, elemsize sweep (no leak / no crash, header offset identical) | [x] |

### Hash map — binary keys (`STBDS_HM_BINARY`)

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 16 | `stbds_hmput_key`, `stbds_hmget_key` | bootstrap from `NULL`, `keysize=4`, `elemsize=8`, insert 1 key, get hit + miss | [x] |
| 17 | `stbds_hmput_key`, `stbds_hmget_key` | insert 1..40 random `u32` keys (crosses 8→16→32→64 table growth), get every key + 40 misses | [x] |
| 18 | `stbds_hmput_key`, `stbds_hmget_key` | `keysize=8` (`size_t` keys), `elemsize=16`, 1..64 keys | [x] |
| 19 | `stbds_hmput_key`, `stbds_hmget_key` | `keysize=1` (byte keys — forces heavy hash collisions/probing), all 256 values | [x] |
| 20 | `stbds_hmput_key`, `stbds_hmget_key` | `keysize=16`, `elemsize=24`, 1..32 keys (odd `elemsize` stride) | [x] |
| 21 | `stbds_hmput_key`, `stbds_hmget_key` | `keysize` 3/5/7 (odd — `memcmp` tail + `hash_bytes` tail), 1..32 keys | [x] |
| 22 | `stbds_hmput_key` | re-put an existing key (update path: `is_key_equal` hit → `temp = index`, `length` unchanged) | [x] |
| 23 | `stbds_hmget_key_ts` | same as #17 but through the `_ts` variant, checking `*temp` rather than `header->temp` | [x] |
| 24 | `stbds_hmput_default` | fresh `NULL` map, then `hmput_key` on top (slot `[-1]` default element preserved) | [x] |
| 25 | `stbds_hmput_default` | called on an existing non-empty map (no-op branch) | [x] |
| 26 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`, no swap) | [x] |
| 27 | `stbds_hmdel_key` | delete a **middle** element (`old_index != final_index` → memmove + re-find + index patch) | [x] |
| 28 | `stbds_hmdel_key` | randomized insert/delete/get sequence, 200 ops, `keysize=4` — drives shrink (`used_count < slot_count>>2`) | [x] |
| 29 | `stbds_hmdel_key` | delete-heavy sequence that crosses `tombstone_count_threshold` → in-place **rebuild** | [x] |
| 30 | `stbds_hmput_key` after deletes | insert into a table containing tombstones (`tombstone >= 0` reuse branch) | [x] |
| 31 | `stbds_hmdel_key` | non-zero `keyoffset` (key not at element offset 0) | [x] |
| 32 | `stbds_hmfree_func` | free a binary map with a live table (`string.mode == 0` ⇒ no key frees) | [x] |

### Hash map — string keys

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 33 | `stbds_hmput_key(mode=STRING)` from `NULL` | `string.mode` auto-set to `SH_DEFAULT`: key **pointer** stored verbatim, `temp_key` set | [x] |
| 34 | `stbds_shmode_func(SH_DEFAULT)` + `hmput_key/hmget_key` | 1..40 random strings, get hit + miss, `temp_key` identity | [x] |
| 35 | `stbds_shmode_func(SH_STRDUP)` + `hmput_key/hmget_key` | keys `strdup`'d — stored pointer `!=` input pointer, contents equal | [x] |
| 36 | `stbds_shmode_func(SH_ARENA)` + `hmput_key/hmget_key` | keys arena-allocated; 1..64 keys crossing the 512-byte block boundary | [x] |
| 37 | `stbds_shmode_func(SH_NONE)` + `hmput_key(mode=STRING)` | `switch` `default:` ⇒ `memcpy(elem,key,keysize)` even though `mode` is STRING | [x] |
| 38 | `stbds_hmput_key(mode=STRING)` | re-put an existing string key (found branch sets `temp_key` from the stored key) | [x] |
| 39 | `stbds_hmput_key(mode=STRING)` | keys sharing long prefixes / differing only in the last byte (`strcmp` discrimination) | [x] |
| 40 | `stbds_hmput_key(mode=STRING)` | empty-string key `""` | [x] |
| 41 | `stbds_hmdel_key(mode=STRING)` on `SH_STRDUP` | delete frees the dup'd key; swap-with-last re-finds by **dereferenced** key pointer | [x] |
| 42 | `stbds_hmdel_key(mode=STRING)` on `SH_ARENA` | delete does **not** free (arena owns); shrink/rebuild preserve `string` arena across index copy | [x] |
| 43 | `stbds_hmfree_func` on `SH_STRDUP` map | frees every key `i` in `1..length`, then `strreset` | [x] |
| 44 | `stbds_hmfree_func` on `SH_ARENA` map | `strreset` frees the block list, keys not individually freed | [x] |
| 45 | randomized string map churn | 200 mixed put/get/del ops over `SH_ARENA`, `SH_STRDUP`, `SH_DEFAULT` (3 sub-rows) | [x] |

### String arena

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 46 | `stbds_stralloc` | fresh arena, one short string (first block, `block` 0→1, `remaining` bookkeeping) | [x] |
| 47 | `stbds_stralloc` | many short strings filling and overflowing the 512-byte block (`block` 1→2→3…, block-size doubling every other step) | [x] |
| 48 | `stbds_stralloc` | string longer than the current `blocksize` (oversized-block branch) on an **empty** arena | [x] |
| 49 | `stbds_stralloc` | oversized string on a **non-empty** arena (spliced after head, `remaining` untouched) | [x] |
| 50 | `stbds_stralloc` | interleaved small/oversized allocations, 200 randomized lengths 0..4096 | [x] |
| 51 | `stbds_stralloc` | `block` driven to saturation (`512<<(block>>1) >= 1 MiB`) via ~44 forced new blocks | [x] |
| 52 | `stbds_strreset` | reset a multi-block arena, then reuse it (fields back to 0, `block` restarts) | [x] |

### Top-level driver

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 53 | `strkey` | `n` ∈ {0,1,9,10,99,100,12345, negatives, INT_MAX, INT_MIN} + 256 random ints | [x] |
| 54 | `sh_puts` | `num` ∈ {0,1,2,7,8,9,64,100,1000} — stdout captured & compared byte-for-byte | [x] |
| 55 | `sh_puts` | `num` ≤ 0 and `num` = INT_MIN / large negative | [x] |
| 56 | `sh_puts` | repeated calls in one process (global `stbds_hash_seed` advances; `buffer` reused) | [x] |
| 57 | `sh_puts` | `num` large enough to cross several arena block sizes (2000, 5000) | [x] |

### Cross-cutting combinations (no single row owns them)

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 58 | `stbds_hmget_key_ts` | STRING mode × `{SH_DEFAULT, SH_STRDUP, SH_ARENA}`, 45 keys, hits + misses | [x] |
| 59 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | `keysize == 0`: `hash_bytes(_,0,_)` collapses all keys to one hash and `memcmp(_,_,0)==0` makes every key equal, so the map collapses to one element | [x] |
| 60 | `stbds_hash_bytes` | unaligned buffer pointers (offsets 0..16) × lengths 0..63 | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `translation/verify.sh` derives the feature
list mechanically from `Cargo.toml` (it would enumerate the full power set if
any existed) and confirms a single configuration; `cargo check
--no-default-features` resolves to the same feature set and compiles clean.

Both `cargo test` (debug) **and** `cargo test --release` are run, so the Rust
`.so` is exercised with overflow checks on *and* off — the two builds load
`target/debug/libsh_puts_lib.so` and `target/release/libsh_puts_lib.so`
respectively.

There is also **no `[[bin]]` target and no `add_executable` in
`c_src/CMakeLists.txt`** — the project builds a shared library only, so the
"compare binary stdout" clause is satisfied by rows 54–57, which capture and
compare `sh_puts`' `printf` output at the file-descriptor level.
