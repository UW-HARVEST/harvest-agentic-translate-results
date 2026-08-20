# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes on
runtime options and input shapes. Every row is exercised against **both** the
C `.so` and the Rust `.so` through `libloading`, with **many randomized inputs
per row** (fixed-seed xorshift PRNG, see `tests/common/mod.rs::Rng`), and the
full observable state is compared byte-for-byte.

## The axes the C code branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `mode` (hash-map key mode) | `< STBDS_HM_STRING` ⇒ binary `memcmp`/`siphash`; `>= STBDS_HM_STRING` ⇒ `strcmp`/`stbds_hash_string`; `== STBDS_HM_STRING` exactly, in `stbds_hmdel_key` only | L560, L590, L713, L836, L842 |
| `table->string.mode` (storage mode) | `STBDS_SH_NONE(0)`/default ⇒ `memcpy` key bytes; `STBDS_SH_DEFAULT(1)` ⇒ store caller's `char*`; `STBDS_SH_STRDUP(2)` ⇒ `stbds_strdup`; `STBDS_SH_ARENA(3)` ⇒ `stbds_stralloc` | L785-790, L575, L836 |
| how the table is created | `stbds_shmode_func` (explicit `string.mode`) vs. implicit bootstrap inside `stbds_hmput_key` (`string.mode = mode>=1 ? SH_DEFAULT : 0`) | L707, L802 |
| `elemsize` | any; drives `STBDS_HASH_TO_ARR`/`ARR_TO_HASH` stride and the `realloc` size | everywhere |
| `keysize` | `0..8` (siphash tail cases), `8` (exactly one block, no tail), `9..15`, `16`, `>16` (multi-block loop) | L522-541 |
| key byte values | bytes `>= 0x80` in positions 3 and 7 trigger the `int`-promotion sign-extension in the siphash loop and in tail `case 4/3/2/1` | L523-524, L536-539 |
| `keyoffset` | `0` (all the `hm*`/`sh*` macros) and non-zero (`STBDS_OFFSETOF` for `hmdel`/`shdel` on structs whose key is not first) | L558-563, L843-845 |
| `seed` | global `stbds_hash_seed`, initial `0x31415926`, settable via `stbds_rand_seed`, and **mutated** by every fresh `stbds_make_hash_index` (`seed = seed*a + b`) | L353-358, L409-412 |
| table `slot_count` | `8` (initial), grows `*2` when `used_count >= slot_count - slot_count/4`, shrinks `/2` when `used_count < slot_count/4 && slot_count > 8`, rebuilds same size when `tombstone_count > slot_count/8 + slot_count/16` | L698-710, L854-862 |
| element count | `0`, `1`, `<6` (no growth), `6` (first growth), `12`, `24`, ... (repeated growth) | L698 |
| delete shape | key absent; delete tail element (`old_index == final_index`); delete interior element (`memmove` + slot re-find); delete-all; interleaved delete/insert (tombstone reuse) | L821-851, L766-769 |
| `stbds_arrgrowf` shape | `a == NULL` vs. non-NULL; `min_cap <= cap` (early out); `min_cap < 2*cap` (double); `min_cap < 4` (clamp to 4); `min_cap` large | L283-292 |
| arena block shape | `len <= remaining` (bump); `len > remaining` and `len <= blocksize` (new block, `blocksize = 512 << (block>>1)`); `len > blocksize` (big block) with `storage == NULL` / `!= NULL`; `block` saturation at `blocksize >= 1<<20` | L885-911 |
| `hash < 2` | `stbds_hash_string`/`stbds_hash_bytes` results `0` and `1` are bumped to `2`/`3` so they can never collide with `STBDS_HASH_EMPTY`/`STBDS_HASH_DELETED` | L596, L719 |
| `strkey` / `sh_geti` | `n` sign & digit count; `num` = 0/1/2/3 (loop-bound corners), 6 & 12 (table growth), `num % 4` (which keys get deleted) | L939-985 |

## Rows

Public entry points are abbreviated: `grow`=`stbds_arrgrowf`,
`free`=`stbds_arrfreef`, `seed`=`stbds_rand_seed`, `hs`=`stbds_hash_string`,
`hb`=`stbds_hash_bytes`, `hmf`=`stbds_hmfree_func`, `gk`=`stbds_hmget_key`,
`gkts`=`stbds_hmget_key_ts`, `pd`=`stbds_hmput_default`,
`pk`=`stbds_hmput_key`, `sm`=`stbds_shmode_func`, `dk`=`stbds_hmdel_key`,
`sa`=`stbds_stralloc`, `sr`=`stbds_strreset`.

### Lowest level: pure hash functions

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 1  | `hb` | `len == 0` (no block, tail `case 0`) × 64 random seeds | [x] |
| 2  | `hb` | `len == 1` (tail `case 1`) × 256 random byte/seed pairs, incl. bytes `>= 0x80` | [x] |
| 3  | `hb` | `len == 2` (tail `case 2` fallthrough) × 256 random | [x] |
| 4  | `hb` | `len == 3` (tail `case 3`) × 256 random | [x] |
| 5  | `hb` | `len == 4` (tail `case 4`, `d[3] << 24` sign-extension) × 256 random incl. `d[3] >= 0x80` | [x] |
| 6  | `hb` | `len == 5` (tail `case 5`, `(size_t)d[4] << 16 << 16`) × 256 random | [x] |
| 7  | `hb` | `len == 6` (tail `case 6`) × 256 random | [x] |
| 8  | `hb` | `len == 7` (tail `case 7`) × 256 random | [x] |
| 9  | `hb` | `len == 8` (exactly one main-loop block, empty tail) × 256 random incl. `d[3]`/`d[7] >= 0x80` | [x] |
| 10 | `hb` | `len == 9..64`, every tail length `0..7`, × 512 random buffers/seeds | [x] |
| 11 | `hb` | `len == 65..4096` (many main-loop blocks) × 64 random buffers | [x] |
| 12 | `hb` | boundary seeds `0`, `1`, `0x31415926`, `SIZE_MAX`, `SIZE_MAX-1`, `1<<63` × lens `0..16` | [x] |
| 13 | `hs` | empty string, `len 1..64` random ASCII, random bytes `0x01..0xFF` (incl. `>= 0x80` ⇒ `(unsigned char)` promotion) × 256 | [x] |
| 14 | `hs` | long strings (256, 1024 bytes) × boundary seeds `0`, `1`, `SIZE_MAX`, `1<<63` | [x] |
| 15 | `seed` + `pk` | `stbds_rand_seed(s)` for `s` in `{0, 1, 0x31415926, SIZE_MAX, random×32}`, then create 4 tables and compare each table's captured `seed` and the evolved global seed | [x] |

### Dynamic array

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 16 | `grow` | `a == NULL`, `addlen == 0`, `min_cap` in `0,1,2,3,4,5,7,8,100,4096` (clamp-to-4 and pass-through paths) × `elemsize` in `1,2,4,8,16,17,64` | [x] |
| 17 | `grow` | `a == NULL`, `min_cap == 0`, `addlen` in `1..2048` (`min_len` drives `min_cap`) × `elemsize` in `1,4,16,64` | [x] |
| 18 | `grow` | non-NULL `a`, `min_cap <= cap` early-out; header/pointer must be untouched × random `elemsize` | [x] |
| 19 | `grow` | non-NULL `a`, `min_cap == cap+1` ⇒ doubling; chained 12 times so `cap` goes `4,8,16,...,8192` × `elemsize` in `1,8,16,64` | [x] |
| 20 | `grow` | non-NULL `a`, `min_cap == 10*cap` ⇒ `min_cap` wins over doubling × random | [x] |
| 21 | `grow` | non-NULL `a`, `addlen > 0` with `length` pre-set to random values ⇒ `min_len = length + addlen` path | [x] |
| 22 | `grow` + `free` | grow, write a random payload, re-grow (realloc preserving payload), verify payload + header, then `free` | [x] |

### Hash map — binary mode (`mode = STBDS_HM_BINARY`, `string.mode = 0`)

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 23 | `pk`,`gk`,`gkts`,`dk`,`hmf` | `mode=0`, `keysize=4`, `elemsize=8`, insert `n` random distinct keys for `n` in `1..40` (crosses growth at 6, 12, 24) then look up every key + 20 absent keys | [x] |
| 24 | same | `mode=0`, `keysize=8`, `elemsize=16` (the natural `{void*,int}` shape) × `n` in `1..40` | [x] |
| 25 | same | `mode=0`, `keysize=1`, `elemsize=4` — only 256 distinct keys ⇒ forces real collisions | [x] |
| 26 | same | `mode=0`, `keysize=16`, `elemsize=24` — 2 siphash main-loop blocks | [x] |
| 27 | same | `mode=0`, `keysize=0`, `elemsize=8` — degenerate: all keys equal, so every insert overwrites slot 0 | [x] |
| 28 | same | `mode=0`, `keysize=3` (tail-only siphash) `elemsize=8`, `n` in `1..40` | [x] |
| 29 | `pk` | duplicate-key insert: put the same key `k` times (`k` in `2..8`) ⇒ found-key path, `temp` stable, `used_count` and `length` must not grow | [x] |
| 30 | `pk`,`dk` | delete the **tail** entry (`old_index == final_index`) ⇒ no `memmove`, no slot re-find | [x] |
| 31 | `pk`,`dk` | delete an **interior** entry ⇒ `memmove` of the tail element + re-find + `b->index[i] = old_index` fixup | [x] |
| 32 | `pk`,`dk` | delete every key in insertion order; then in reverse order; then in random order (`n` in `1..40`) | [x] |
| 33 | `pk`,`dk` | interleaved insert/delete so a tombstone is **reused** by a later insert (`tombstone >= 0` at `found_empty_slot`) | [x] |
| 34 | `pk`,`dk` | enough deletes at `slot_count >= 16` to trip `used_count < slot_count>>2` ⇒ table **shrink** (`slot_count>>1`) | [x] |
| 35 | `pk`,`dk` | delete pattern that trips `tombstone_count > slot_count/8 + slot_count/16` ⇒ same-size **rebuild** | [x] |
| 36 | `pk`,`dk` | `slot_count == 8` (fewer than 6 entries): `used_count_shrink_threshold` forced to 0 ⇒ never shrinks | [x] |
| 37 | `pd`,`gk` | `pd` on `NULL`, then `gk` (no table yet ⇒ `temp = -1`), then `pk` (table created) — the `hmdefault` bootstrap order | [x] |
| 38 | `pd` | `pd` twice in a row, and `pd` after `pk` (`length != 0` ⇒ no-op) | [x] |
| 39 | `gkts` | `gkts` with a caller-supplied `temp` out-param on: NULL map, table-less map, hit, miss × `n` in `1..40` | [x] |
| 40 | `dk` | `keyoffset != 0`: `elemsize=16`, key stored at offset 8, `keysize=8`, binary mode, insert via a hand-written element + `pk`-compatible layout, then `dk` with `keyoffset=8` | [x] |
| 41 | `hmf` | `hmf` on a binary map with a live table (frees table + header, no per-element free) | [x] |
| 42 | `pk`…`hmf` | full life-cycle × 200 randomized op scripts (`put`/`get`/`del` chosen at random, `keysize` and `elemsize` random per script) — the fuzz row | [x] |

### Hash map — string modes

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 43 | `pk`,`gk`,`dk`,`hmf` | `mode=1`, map bootstrapped by `pk(NULL,…)` ⇒ `string.mode = STBDS_SH_DEFAULT` (stores the caller's pointer) × `n` in `1..40` random keys | [x] |
| 44 | `sm`,`pk`,`hmf` | `sm(elemsize, STBDS_SH_NONE)` then `mode=1` puts ⇒ `switch` `default:` ⇒ `memcpy` of the first `keysize` bytes of the key's **text** into the element. Insert-only is well defined (`stbds_is_key_equal` is never reached unless the full 64-bit hashes collide) and is verified here; the *lookup*, which reinterprets that text as a `char*`, is fatal and is ERRORS.md row 44b | [x] |
| 45 | `sm`,`pk`,`gk`,`dk`,`hmf` | `sm(elemsize, STBDS_SH_DEFAULT)` + `mode=1` × `n` in `1..40` | [x] |
| 46 | `sm`,`pk`,`gk`,`dk`,`hmf` | `sm(elemsize, STBDS_SH_STRDUP)` + `mode=1` × `n` in `1..40`; keys must be `strdup`ed (stored pointer `!=` caller's) and freed on `dk`/`hmf` | [x] |
| 47 | `sm`,`pk`,`gk`,`dk`,`hmf` | `sm(elemsize, STBDS_SH_ARENA)` + `mode=1` × `n` in `1..40`; keys copied into the arena, `string.remaining`/`block` evolve | [x] |
| 48 | `sm`,`pk` | string keys of length `0` (empty), `1`, `7`, `8`, `9`, `63`, `64`, `511`, `512`, `513`, `4096` under `SH_STRDUP` **and** `SH_ARENA` (crosses `blocksize` in the arena) | [x] |
| 49 | `sm`,`pk` | many string keys under `SH_ARENA` so `a->block` increments repeatedly (`512 → 512 → 1024 → …`) | [x] |
| 50 | `sm`,`pk`,`dk` | `SH_STRDUP` + duplicate key put ⇒ found-key path sets `stbds_temp_key`; second put must **not** strdup again | [x] |
| 51 | `sm`,`pk`,`dk` | `SH_STRDUP` + `dk(mode=1)` ⇒ the stored key is `free`d and the tail element `memmove`d in; re-find uses `*(char**)` | [x] |
| 52 | `sm`,`pk`,`dk` | `SH_ARENA` + `dk(mode=1)` ⇒ **no** free (arena owns), same `memmove`/re-find | [x] |
| 53 | `sm`,`pk`,`dk` | `SH_DEFAULT` + `dk(mode=1)` ⇒ no free, caller's pointers still valid afterwards | [x] |
| 54 | `sm`,`pk`,`gk` | keys that are **prefixes** of each other (`"a"`,`"ab"`,`"abc"`) and keys differing only in the last byte ⇒ exercises `strcmp` vs. hash collisions | [x] |
| 55 | `sm`,`pk`,`gk` | keys with bytes `>= 0x80` (UTF-8 / Latin-1) ⇒ `(unsigned char) *str++` promotion in `stbds_hash_string` | [x] |
| 56 | `sm`,`pk`,`dk`,`hmf` | full string life-cycle × 200 randomized op scripts, `string.mode` chosen at random from all 4 values | [x] |
| 57 | `hmf` | `hmf` on `SH_STRDUP` (per-element `free` loop over `i in 1..length`), `SH_ARENA` (`strreset`), `SH_DEFAULT`, `SH_NONE` | [x] |

### String arena (lowest level, called directly)

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 58 | `sa`,`sr` | fresh zeroed arena, one string of length `0..500` (fits the first 512-byte block) × 256 random | [x] |
| 59 | `sa`,`sr` | fresh arena, many strings until `remaining` is exhausted and a second block is allocated (`block` 0→1→2…) × random lengths | [x] |
| 60 | `sa`,`sr` | `len > blocksize` on a fresh arena (big-block path, `storage == NULL`) — strings of `600`, `1024`, `65536` bytes | [x] |
| 61 | `sa`,`sr` | `len > blocksize` after a normal block exists (big block spliced after the head, `remaining` preserved) | [x] |
| 62 | `sa`,`sr` | `len == remaining` exactly, and `len == remaining + 1` (boundary either side of the new-block test) | [x] |
| 63 | `sa` | pre-set `a->block` to each of `0..30` on an otherwise empty arena ⇒ `blocksize = 512 << (block>>1)` and the `BLOCKSIZE_MAX` saturation of `++a->block` | [x] |
| 64 | `sa`,`sr` | 2000 random-length strings (`1..1500`) in one arena ⇒ mixed bump / new-block / big-block, then `sr`; every returned string's contents verified | [x] |
| 65 | `sr` | `sr` on a zeroed arena, on a 1-block arena, on a many-block arena, twice in a row (idempotent) | [x] |

### Test-driver entry points

| #  | entry point(s) | configuration (options set + input shape) | ✓ |
|----|----------------|-------------------------------------------|---|
| 66 | `strkey` | `n` in `0,1,2,9,10,11,99,100,101,999,1000,12345,-1,-9,-10,-12345,INT_MAX,INT_MIN` + 256 random `i32` ⇒ returned C string compared byte-for-byte (and the pointer must be stable across calls) | [x] |
| 67 | `sh_geti` | `num` in `0,1,2,3,4,5,6,7,8,12,13,16,17,24,32,33,64,100,257,1000` — stdout captured in a subprocess and compared byte-for-byte, plus exit status | [x] |
| 68 | `sh_geti` | `num` negative / `INT_MIN` (all loops skipped) — subprocess stdout + status | [x] |
| 69 | `seed` + `sh_geti` | `stbds_rand_seed(s)` before `sh_geti(num)` for `s` in `{0, 1, 7, 0xdeadbeef, 0x31415926, SIZE_MAX}` × `num` in `{8, 16, 33}` ⇒ different probe orders and hence different code paths / different `STBDS_ASSERT` exposure. NOTE: `sh_geti` prints `strmap[z]` for `z in 0..shlen`, i.e. in ARRAY (insertion) order, so stdout is provably seed-INDEPENDENT; what each seed verifies is that every internal assert still holds and the child still exits 0, on both libraries | [x] |
| 70 | `sh_geti` | called twice in the same process (the global `stbds_hash_seed` has evolved by then, so the second call probes differently) ⇒ combined stdout must match C byte-for-byte and the child must still exit 0 | [x] |


## Extra rows added during verification

| #   | entry point(s) | configuration | ✓ |
|-----|----------------|---------------|---|
| E1  | `pk`,`gk`,`dk`,`hmf` | **deep-growth soak**: 1200 distinct binary keys inserted (table doubles `8→16→…→2048`) then deleted in shuffled order (table shrinks all the way back to 8); full state compared periodically — `cfg_binary_deep_growth_soak` | [x] |
| E2  | `sm`,`pk`,`gk`,`dk`,`hmf` | the same soak for string maps: 800 distinct string keys × `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA`, up to 1024+ slots and back; every stored key's text re-verified — `cfg_string_deep_growth_soak` | [x] |
| E3  | `pk`,`gk`,`dk` | `mode` in `{0, -1, -2, INT_MIN}` driven through a full life-cycle (all take the binary path) — `cfg_binary_negative_modes` | [x] |
| E4  | `pk` | no in-use bucket slot may ever hold the reserved hashes `0`/`1` (the `if (hash < 2) hash += 2` guard) — `cfg_hash_never_0_or_1_in_buckets` | [x] |
| E5  | `pk` | both branches of `stbds_hmput_key`'s found-key path are exercised, and the wrap-around branch's omission of the `stbds_temp_key` update is reproduced — `cfg50_strdup_duplicate_put` counts first-loop vs. wrap-loop hits and requires both to be non-zero | [x] |

## Harness validation (mutation testing)

To prove these rows can actually fail, eight deliberate bugs were injected into
`src/` one at a time; every one was caught:

| mutation | tests that failed |
|----------|-------------------|
| `stbds_hash_string`: `rotate_right(hash,22)` → `21` | 6 |
| `stbds_arrgrowf`: `min_cap < 4` → `min_cap < 3` | 4 |
| `stbds_make_hash_index`: `shrink_threshold = slot_count>>2` → `>>1` | 6 |
| `stbds_siphash_bytes`: drop the `int`-promotion sign-extension of the main-loop low word | 6 |
| `stbds_siphash_bytes`: drop the sign-extension of tail `case 4` | 5 |
| `stbds_hmput_key`: "fix" the stb bug by also setting `temp_key` in the wrap-around loop | 1 |
| `strkey`: drop the `-` sign for negative `n` | 2 |
| `stbds_stralloc`: `blocksize < BLOCKSIZE_MAX` → `<=` | 1 |

## Row → test mapping

Every row above is covered by the test named here; the number in the test name
is the row it implements (tests named `cfgA_B_*` cover rows A..B).

| rows | test | file |
|------|------|------|
| 1–9   | `cfg01_09_hash_bytes_tail_cases`   | `tests/hash.rs` |
| 10    | `cfg10_hash_bytes_9_to_64`         | `tests/hash.rs` |
| 11    | `cfg11_hash_bytes_large`           | `tests/hash.rs` |
| 12    | `cfg12_hash_bytes_boundary_seeds`   | `tests/hash.rs` |
| 13    | `cfg13_hash_string_short`          | `tests/hash.rs` |
| 14    | `cfg14_hash_string_long`           | `tests/hash.rs` |
| 15    | `cfg15_rand_seed_evolution`        | `tests/hash.rs` |
| 16    | `cfg16_arrgrowf_null_min_caps`     | `tests/arrays.rs` |
| 17    | `cfg17_arrgrowf_null_addlen`       | `tests/arrays.rs` |
| 18    | `cfg18_arrgrowf_early_out`         | `tests/arrays.rs` |
| 19    | `cfg19_arrgrowf_doubling`          | `tests/arrays.rs` |
| 20    | `cfg20_arrgrowf_min_cap_wins`      | `tests/arrays.rs` |
| 21    | `cfg21_arrgrowf_length_plus_addlen`| `tests/arrays.rs` |
| 22    | `cfg22_arrgrowf_payload_roundtrip` | `tests/arrays.rs` |
| 23–28 | `cfg23_28_binary_shapes`           | `tests/maps_binary.rs` |
| 27    | `cfg27_keysize_zero`               | `tests/maps_binary.rs` |
| 29    | `cfg29_duplicate_puts`             | `tests/maps_binary.rs` |
| 30–31 | `cfg30_31_tail_vs_interior_delete` | `tests/maps_binary.rs` |
| 32    | `cfg32_delete_all_orders`          | `tests/maps_binary.rs` |
| 33    | `cfg33_tombstone_reuse`            | `tests/maps_binary.rs` |
| 34–36 | `cfg34_36_shrink_and_rebuild`      | `tests/maps_binary.rs` |
| 37–38 | `cfg37_38_hmput_default`           | `tests/maps_binary.rs` |
| 39    | `cfg39_hmget_key_ts_states`        | `tests/maps_binary.rs` |
| 40    | `cfg40_keyoffset_nonzero`          | `tests/maps_binary.rs` |
| 41    | `cfg41_hmfree_binary`              | `tests/maps_binary.rs` |
| 42    | `cfg42_binary_fuzz`                | `tests/maps_binary.rs` |
| 43    | `cfg43_bootstrapped_default_mode`  | `tests/maps_string.rs` |
| 44    | `cfg44_sh_none_string_insert_only` (+ ERRORS.md 44b `fatal_sh_none_string_lookup`) | `tests/maps_string.rs` |
| 45    | `cfg45_sh_default`                 | `tests/maps_string.rs` |
| 46    | `cfg46_sh_strdup`                  | `tests/maps_string.rs` |
| 47    | `cfg47_sh_arena`                   | `tests/maps_string.rs` |
| 48    | `cfg48_key_length_boundaries`      | `tests/maps_string.rs` |
| 49    | `cfg49_arena_block_growth`         | `tests/maps_string.rs` |
| 50    | `cfg50_strdup_duplicate_put`       | `tests/maps_string.rs` |
| 51–53 | `cfg51_53_string_delete_modes`     | `tests/maps_string.rs` |
| 54    | `cfg54_prefix_keys`                | `tests/maps_string.rs` |
| 55    | `cfg55_high_byte_keys`             | `tests/maps_string.rs` |
| 56    | `cfg56_string_fuzz`                | `tests/maps_string.rs` |
| 57    | `cfg57_hmfree_string_modes`        | `tests/maps_string.rs` |
| 58    | `cfg58_arena_single_short_string`  | `tests/arena.rs` |
| 59    | `cfg59_arena_many_blocks`          | `tests/arena.rs` |
| 60    | `cfg60_arena_big_block_fresh`      | `tests/arena.rs` |
| 61    | `cfg61_arena_big_block_after_head` | `tests/arena.rs` |
| 62    | `cfg62_arena_exact_fit_boundary`   | `tests/arena.rs` |
| 63    | `cfg63_arena_preset_block`         | `tests/arena.rs` |
| 64    | `cfg64_arena_fuzz`                 | `tests/arena.rs` |
| 65    | `cfg65_strreset_shapes`            | `tests/arena.rs` |
| 66    | `cfg66_strkey`                     | `tests/driver.rs` |
| 67    | `cfg67_sh_geti_positive`           | `tests/driver.rs` |
| 68    | `cfg68_sh_geti_nonpositive`        | `tests/driver.rs` |
| 69    | `cfg69_sh_geti_seeded`             | `tests/driver.rs` |
| 70    | `cfg70_sh_geti_twice`              | `tests/driver.rs` |
| E1    | `cfg_binary_deep_growth_soak`      | `tests/maps_binary.rs` |
| E2    | `cfg_string_deep_growth_soak`      | `tests/maps_string.rs` |
| E3    | `cfg_binary_negative_modes`        | `tests/maps_binary.rs` |
| E4    | `cfg_hash_never_0_or_1_in_buckets` | `tests/hash.rs` |
| E5    | `cfg50_strdup_duplicate_put`       | `tests/maps_string.rs` |
