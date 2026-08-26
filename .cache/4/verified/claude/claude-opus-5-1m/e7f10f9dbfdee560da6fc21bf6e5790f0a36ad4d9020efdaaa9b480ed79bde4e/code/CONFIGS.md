# CONFIGS.md — CONFIGURATION-SURFACE TABLE (Phase B)

Mechanically derived from the branches `c_src/src/lib.c` actually takes. Every
row is a differential test that drives **both** `.so`s through their exported
symbols in that configuration and compares the results byte-for-byte.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| **A_ptr** — array argument | `NULL` / non-`NULL` | `arrgrowf` L300, `hmget_key_ts` L634, `hmput_key` L686, `hmput_default` L669, `hmdel_key` L809, `hmfree_func` L573 |
| **A_grow** — growth decision | `min_cap <= cap` (no-op) / `min_cap < 2*cap` (double) / `min_cap < 4` (bump to 4) / plain `min_cap` | `arrgrowf` L286-292 |
| **elemsize** | any; drives `elemsize*i` strides and `sizeof(header)` offset | everywhere |
| **keysize** | any; only used by the `memcmp`/`memcpy` binary path | `is_key_equal` L563, `hmput_key` L789 |
| **mode** (`STBDS_HM_*`) | `mode >= STBDS_HM_STRING(1)` → string keys / `mode < 1` → binary keys; plus the *exact* test `mode == STBDS_HM_STRING` in `hmdel_key` L836/L842 | L560, L590, L707, L713, L732, L836, L842 |
| **table->string.mode** (`STBDS_SH_*`) | `SH_STRDUP(2)` / `SH_ARENA(3)` / `SH_DEFAULT(1)` / `default:` (`SH_NONE(0)` + everything else) | `hmput_key` L785-790; also `hmfree_func` L575, `hmdel_key` L836 |
| **table existence** | `hash_table == NULL` / non-`NULL` | L644, L698, L816 |
| **table occupancy** | `used_count < threshold` / `>= threshold` (grow); `used_count < shrink_threshold` (shrink); `tombstone_count > tombstone_threshold` (rebuild) | L698, L854, L858 |
| **slot_count** | `8` (initial, `shrink_threshold` forced to 0) / `16,32,64,…` | `make_hash_index` L399 |
| **probe scan** | upper scan `[pos&7, 8)` / wrap-around scan `[0, pos&7)` / next bucket (`pos += step`) | L604/L614/L625, L728/L746/L761, L443/L452/L460 |
| **slot state** | `hash == query` (+key equal / key unequal) / `hash == EMPTY(0)` / `hash == DELETED(1)` with `index == INDEX_DELETED` (tombstone) | L605-611, L729-742 |
| **delete position** | `old_index == final_index` (last live elem, no compaction) / `old_index != final_index` (compaction + slot fix-up) | L839 |
| **entry count** | 0 / 1 / 2 / 5 / 6 (== grow threshold) / 7 / 12 (2nd grow) / 100 / 1000 | L698 |
| **arena state** | `len <= remaining` (fast path) / new block (`len <= blocksize`) / dedicated oversized block with `storage != NULL` / with `storage == NULL` | `stralloc` L885-911 |
| **arena block counter** | `0 … 22` (saturates: `512<<11 == 1<<20 == MAX`) | L888-891 |
| **hash seed** | global `stbds_hash_seed` (initial `0x31415926`, then LCG `*0x27bb2ee687b0b0fd + 0xb504f32d` per fresh index); settable via `stbds_rand_seed` | L353, L355, L409-412 |
| **hash input shape** | `len == 0`; `len` 1..7 (each `switch` fall-through); `len == 8` (one block, remainder 0); `len` 9..71 (blocks + each remainder); high-bit bytes (sign-extension of `d[3]<<24`) | `siphash_bytes` L522-541 |
| **string input shape** | empty / 1 char / long / bytes ≥ 0x80 (signed `char` → `(unsigned char)` cast) | `hash_string` L480-481 |
| **`strkey` input** | `0`, `±1`, `INT_MAX`, `INT_MIN`, random | L939-943 |
| **degenerate sizes** | `elemsize == 0` (header-only allocation); `keysize == 0` (`memcmp(...,0)` always matches, constant hash); `keysize == elemsize` (no value part); `elemsize` 256/1024/4096 | L297, L563, L789, L840 |
| **`keyoffset`** (only `hmdel_key` exposes it) | `0` (what every macro passes) / non-zero, where the stored key is at offset 0 so the comparison necessarily misses | `hmdel_key` L820/L843/L845 |
| **build profile of the Rust `.so`** | `release` (`panic = "abort"`, no `debug_assertions`) — the shipped artifact — and `debug` (UB checks on) | Cargo.toml |

Element layouts used (mirroring the C file's own test structs):

| name | C type | elemsize | keysize |
|------|--------|----------|---------|
| `I2I`  | `struct { int key; int value; }` | 8 | 4 |
| `S1`   | `stbds_struct { int key,b,c,d; }` | 16 | 4 |
| `S2`   | `stbds_struct2 { int key[2],b,c,d; }` | 16 | 8 |
| `U2U`  | `struct { size_t key; size_t value; }` | 16 | 8 |
| `B1`   | `struct { unsigned char key; unsigned char v[7]; }` | 8 | 1 |
| `BIG`  | `struct { int key; char pad[124]; }` | 128 | 4 |
| `ODD`  | `struct { char key[3]; char pad[9]; }` | 12 | 3 |
| `STR`  | `struct { char *key; int value; }` (string map) | 16 | 8 |
| `STRB` | `struct { char *key; char pad[24]; }` | 32 | 8 |

Comparison method: after each operation both libraries' full internal state is
serialised (`array_header{length,capacity,temp}`, `hash_index{slot_count,
used_count,used_count_threshold,used_count_shrink_threshold,tombstone_count,
tombstone_count_threshold,seed,slot_count_log2,string.remaining,string.block,
string.mode}`, every `hash[]`/`index[]` entry of every bucket, and the element
payload — raw bytes for binary keys, dereferenced C strings for string keys)
and the two byte strings must be equal. Pointers themselves are canonicalised
(NULL vs non-NULL, or dereferenced), everything else is compared literally.

All rows are exercised with **many randomized inputs** driven by a fixed-seed
SplitMix64 PRNG (`SEED = 0x9E3779B97F4A7C15`), so the row numbers below name a
*shape*, not a single value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap=0` → the `min_cap <= cap` early-out on a NULL array (returns NULL) | [x] |
| 2 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap ∈ {1,2,3,4}` → forced to capacity 4; `elemsize ∈ {1,4,8,12,16,128}` | [x] |
| 3 | `stbds_arrgrowf` | `a=NULL`, `addlen ∈ {1,3,4,5,17,1000}`, `min_cap=0` → `min_cap = addlen`, then the `<4` bump; random `elemsize` | [x] |
| 4 | `stbds_arrgrowf` | `a=NULL`, `addlen` and `min_cap` both non-zero with `addlen > min_cap` and `addlen < min_cap` (both orderings of L283) | [x] |
| 5 | `stbds_arrgrowf` | grow an existing array repeatedly (`addlen=1`, `min_cap=0`) 0→1→…→64 elements: exercises the `min_cap < 2*cap` doubling ladder and `length`/`temp`/`hash_table` preservation | [x] |
| 6 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → must return the *same* pointer without reallocating, header untouched | [x] |
| 7 | `stbds_arrgrowf` | existing array, `min_cap >= 2*cap` (jump straight to a big capacity, e.g. cap 4 → min_cap 1000) | [x] |
| 8 | `stbds_arrgrowf` + `stbds_arrfreef` | full alloc/write/read/free round trip with random `elemsize` and random payload; verifies the 32-byte header offset and that the data region is usable | [x] |
| 9 | `stbds_hash_bytes` | `len = 0` (pointer never read), random seeds incl. `0`, `1`, `SIZE_MAX`, `0x31415926` | [x] |
| 10 | `stbds_hash_bytes` | `len ∈ 1..7` (every `switch` fall-through arm, main loop never runs), random bytes, random seeds | [x] |
| 11 | `stbds_hash_bytes` | `len = 8` exactly (one main-loop block, remainder 0) | [x] |
| 12 | `stbds_hash_bytes` | `len ∈ 9..71` (1..8 blocks × every remainder 0..7), random bytes, random seeds | [x] |
| 13 | `stbds_hash_bytes` | all-`0x00`, all-`0xFF`, and `0x80`-heavy buffers at every `len ∈ 0..40` → exercises `d[3]<<24` / `d[7]<<24` going negative and sign-extending into `size_t` | [x] |
| 14 | `stbds_hash_bytes` | large `len` (256, 1024, 4096) random buffers | [x] |
| 15 | `stbds_hash_string` | empty string; 1-char; random ASCII of length 1..64; random seeds | [x] |
| 16 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (signed-`char` sign-extension trap) and `0x01..0x1F` | [x] |
| 17 | `stbds_hash_string` | long strings (256, 4096 chars) | [x] |
| 18 | `stbds_rand_seed` | set the global seed to `{0, 1, SIZE_MAX, 0x31415926, random}` then create a fresh index (`stbds_shmode_func`) and read back `hash_index.seed`; then create a *second* index to check the LCG advance `seed*0x27bb2ee687b0b0fd + 0xb504f32d` | [x] |
| 19 | `stbds_rand_seed` | seed set, then a full `hmput_key` sequence — the bucket layout (which slot each key lands in) must match, proving the seed feeds `hash_bytes`/`hash_string` identically | [x] |
| 20 | `stbds_hmput_default` | `a=NULL`, random `elemsize` → 1-element zeroed array | [x] |
| 21 | `stbds_hmput_default` | `a` from `arrgrowf` with `length == 0` → bumps to 1 | [x] |
| 22 | `stbds_hmput_default` | `a` already has `length >= 1` → returns unchanged (no realloc, header untouched) | [x] |
| 23 | `stbds_hmput_default` | called *after* `hmput_key` (table present) — must not disturb the hash index | [x] |
| 24 | `stbds_hmput_key` | `mode=BINARY(0)`, layout `I2I`, `a=NULL` → first insert; `string.mode` must be `SH_NONE(0)` → `memcpy` key path | [x] |
| 25 | `stbds_hmput_key` | `mode=BINARY`, layout `I2I`, 1..5 distinct random keys (below the grow threshold, `slot_count` stays 8) | [x] |
| 26 | `stbds_hmput_key` | `mode=BINARY`, layout `I2I`, exactly 6 keys → `used_count >= used_count_threshold(6)` on the *next* put → grow to `slot_count 16`; 7, 12, 13 keys → 2nd/3rd grow | [x] |
| 27 | `stbds_hmput_key` | `mode=BINARY`, layout `I2I`, 100 and 1000 random distinct keys → many grows, full rehash chains, wrap-around probe scans | [x] |
| 28 | `stbds_hmput_key` | `mode=BINARY`, **duplicate** keys interleaved with new ones → the "key found, update `temp`, return early" path (both the upper scan and the wrap-around scan) | [x] |
| 29 | `stbds_hmput_key` | `mode=BINARY`, layout `S1` (elemsize 16, keysize 4 — key smaller than the element, trailing payload must be preserved) | [x] |
| 30 | `stbds_hmput_key` | `mode=BINARY`, layout `S2` (elemsize 16, keysize 8 — 2-int composite key) | [x] |
| 31 | `stbds_hmput_key` | `mode=BINARY`, layout `U2U` (keysize 8 == `sizeof(size_t)`, siphash main loop runs once, remainder 0) | [x] |
| 32 | `stbds_hmput_key` | `mode=BINARY`, layout `B1` (keysize 1 → only 256 possible keys → forced duplicate/collision pressure at `slot_count` 8/16/32) | [x] |
| 33 | `stbds_hmput_key` | `mode=BINARY`, layout `ODD` (keysize 3, elemsize 12 — unaligned strides) | [x] |
| 34 | `stbds_hmput_key` | `mode=BINARY`, layout `BIG` (elemsize 128, keysize 4 — large strides, `elemsize*i` arithmetic) | [x] |
| 35 | `stbds_hmput_key` | `mode=STRING(1)`, layout `STR`, table auto-created by `hmput_key` → `string.mode == SH_DEFAULT(1)`, key stored as the caller's pointer, `temp_key` set | [x] |
| 36 | `stbds_hmput_key` | `mode=STRING`, `SH_DEFAULT` table, 1/2/6/7/100 random distinct strings (incl. empty string) → grows + rehash with string hashing | [x] |
| 37 | `stbds_hmput_key` | `mode=STRING`, `SH_DEFAULT` table, duplicate strings (different buffers, equal contents) → `strcmp` hit path, `temp_key` updated from the stored pointer | [x] |
| 38 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_STRDUP(2)` table, `mode=STRING`, 1/6/7/100 strings → `stbds_strdup` per key, `temp_key` = the dup | [x] |
| 39 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_ARENA(3)` table, `mode=STRING`, 1/6/7/100/2000 strings incl. some > 512 B and > 1 MiB → drives `stbds_stralloc` block growth from inside the map | [x] |
| 40 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_DEFAULT(1)` table created explicitly, `mode=STRING` | [x] |
| 41 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_NONE(0)` table created explicitly but `mode=STRING` → `default:` branch does `memcpy(dst,key,keysize)` of the *pointer bytes*; lookups then `strcmp` through it | [x] |
| 42 | `stbds_shmode_func` | out-of-range `mode ∈ {-1, 4, 5, 255, 256, 257, INT_MIN, INT_MAX}` → `(unsigned char)` truncation; `256`→`0`, `-1`→`255`; then a binary-key `hmput_key` sequence on the resulting table | [x] |
| 43 | `stbds_hmget_key` | `a=NULL` → fresh 1-element array, `temp == -1` | [x] |
| 44 | `stbds_hmget_key` | array with no hash table (built by `hmput_default`) → `temp == -1`, pointer unchanged | [x] |
| 45 | `stbds_hmget_key` | `mode=BINARY`, present keys (every inserted key looked up) and absent keys, at `slot_count` 8/16/32/128 | [x] |
| 46 | `stbds_hmget_key` | `mode=STRING`, present + absent strings, `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` tables | [x] |
| 47 | `stbds_hmget_key_ts` | same shapes as #43-#46 but through the `ptrdiff_t *temp` out-parameter; `temp` must be written even on the miss/NULL paths, and the header `temp` must be left **untouched** (unlike `hmget_key`) | [x] |
| 48 | `stbds_hmdel_key` | `a=NULL` → returns NULL | [x] |
| 49 | `stbds_hmdel_key` | array with no hash table → `temp = 0`, returns `a` | [x] |
| 50 | `stbds_hmdel_key` | `mode=BINARY`, key absent → `temp = 0`, everything unchanged | [x] |
| 51 | `stbds_hmdel_key` | `mode=BINARY`, delete the **last** live element (`old_index == final_index`) → no compaction | [x] |
| 52 | `stbds_hmdel_key` | `mode=BINARY`, delete the **first**/**middle** element (`old_index != final_index`) → compaction `memmove` + slot re-find + `index` fix-up | [x] |
| 53 | `stbds_hmdel_key` | `mode=BINARY`, `keyoffset ∈ {0, 4, 8}` (non-zero key offsets inside the element — only `hmdel_key` exposes `keyoffset`) | [x] |
| 54 | `stbds_hmdel_key` | `mode=BINARY`, delete **all** entries from a 100-entry map in insertion order / reverse order / random order → drives the shrink ladder (`slot_count` 128→64→32→16→8) and the tombstone rebuild | [x] |
| 55 | `stbds_hmdel_key` | delete-then-reinsert cycles → tombstone reuse in `hmput_key` (`tombstone >= 0` branch, `tombstone_count--`) | [x] |
| 56 | `stbds_hmdel_key` | `mode=STRING`, `SH_DEFAULT` table, delete last / middle / absent / all | [x] |
| 57 | `stbds_hmdel_key` | `mode=STRING`, `SH_STRDUP` table → the `mode == STBDS_HM_STRING && string.mode == SH_STRDUP` free branch, delete last / middle / all | [x] |
| 58 | `stbds_hmdel_key` | `mode=STRING`, `SH_ARENA` table (keys not freed on delete) | [x] |
| 59 | `stbds_hmfree_func` | `a=NULL` → no-op | [x] |
| 60 | `stbds_hmfree_func` | array with `hash_table == NULL` | [x] |
| 61 | `stbds_hmfree_func` | tables with `string.mode` `SH_NONE`/`SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA`, 0/1/100 entries → strdup sweep from index 1 and `strreset` of the arena | [x] |
| 62 | `stbds_stralloc` | fresh zeroed arena, one short string (`len <= 512`) → first block, `block` 0→1, `remaining = 512-len` | [x] |
| 63 | `stbds_stralloc` | fast path: several strings that fit in `remaining` → no new block, `block` unchanged, descending pointers inside the block | [x] |
| 64 | `stbds_stralloc` | many strings until blocks roll over 512→1024→…→1 MiB → `block` counter ladder and saturation at 22 | [x] |
| 65 | `stbds_stralloc` | oversized string (`len > blocksize`) with `storage == NULL` → becomes the head block and `remaining` forced to 0 | [x] |
| 66 | `stbds_stralloc` | oversized string with `storage != NULL` → spliced in *after* the head (`sb->next = a->storage->next; a->storage->next = sb;`), `remaining` untouched | [x] |
| 67 | `stbds_stralloc` | empty string (`len == 1`) repeatedly, and strings of length exactly `remaining` / `remaining-1` / `remaining+1` (boundary of L885) | [x] |
| 68 | `stbds_strreset` | zeroed arena (no blocks); arena with 1 block; arena with many blocks; arena with an oversized block chain; called twice (idempotent) | [x] |
| 69 | `strkey` | `n ∈ {0, 1, -1, 9, 10, 99, 100, 12345, INT_MAX, INT_MIN, random}` → the whole 256-byte static `buffer` compared, and the same call *sequence* replayed on both libs so residual bytes match | [x] |
| 70 | `arr_ins` | `num ∈ {0, 1, 4, -1, INT_MAX, INT_MIN, random}` → must return normally (both internal asserts hold) | [x] |
| 71 | mixed workload | randomized interleaving of `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key`/`hmput_default` (2000 ops) against a shadow model, `mode=BINARY`, layout `I2I`, with a state dump compared after **every** op | [x] |
| 72 | mixed workload | same 2000-op interleaving with `mode=STRING` over `SH_DEFAULT`, `SH_STRDUP` and `SH_ARENA` tables | [x] |
| 73 | mixed workload | same interleaving with out-of-range `mode` values (`-5`, `2`, `7`, `INT_MAX`) restricted to the operations that stay well-defined (put/get/get_ts, and del of the last live element) | [x] |
| 74 | cross-call global state | `stbds_rand_seed` → N × `stbds_shmode_func` → the recorded `seed` of every created index must match, proving the private `stbds_hash_seed` LCG runs in lockstep | [x] |
| 75 | `stbds_arrgrowf` / `stbds_arrfreef` | `elemsize == 0` — `elemsize*min_cap + 32 == 32`, header-only allocation, no element byte ever touched; `min_cap` 0/1/4/5/1000/`SIZE_MAX`, `addlen` 0/1/7, plus the whole grow ladder | [x] |
| 76 | `stbds_hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key` | `keysize == 0` — `stbds_hash_bytes(key,0,seed)` is constant and `memcmp(...,0) == 0` always matches, so the map degenerates to a single entry that every put updates in place; `elemsize` 8/16/40 | [x] |
| 77 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` | `keysize == elemsize` (a "set" with no value part); `elemsize` 1/2/4/8/16/32, incl. `elemsize == 1` (only 256 distinct keys → heavy duplicate + probe-chain pressure) | [x] |
| 78 | `stbds_hmput_key` / `hmdel_key` | very large `elemsize` (256, 1024, 4096) with `keysize 8` — exercises the `elemsize*i` stride arithmetic and the compaction `memmove` at scale | [x] |
| 79 | `stbds_hmget_key_ts` | the header `stbds_temp` field is pre-loaded with `-5`/`0`/`12345`/`isize::MIN`/`isize::MAX` and must be left **completely untouched** (only `stbds_hmget_key` writes it), for present and absent keys | [x] |
