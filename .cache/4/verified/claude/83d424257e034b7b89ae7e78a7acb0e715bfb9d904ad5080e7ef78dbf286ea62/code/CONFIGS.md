# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/lib.c`

Derived **mechanically** from the branches the C code actually takes. Every row
is a combination of *options set* + *input shape* that the C treats differently,
and every row is exercised through **both** `.so`s with many randomized inputs
(fixed seed) and compared byte-for-byte.

## The axes the C code branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` argument (`int`, any value accepted) | `≤ 0` binary · `== 1` string · `≥ 2` string-hash-but-not-`==1` | `is_key_equal:560`, `hmput_key:707,713,732`, `hm_find_slot:590`, `hmdel_key:836,842` |
| `table->string.mode` (arena mode) | `0 STBDS_SH_NONE` · `1 DEFAULT` · `2 STRDUP` · `3 ARENA` · out-of-range (4/44/255) | `hmput_key:785-790`, `hmfree_func:575`, `hmdel_key:836` |
| how the arena mode is established | implicitly by `hmput_key:707` · explicitly by `stbds_shmode_func:803` | — |
| `elemsize` | any; `< / == / >` `keysize`; 1, 4, 8, 16, 24, 32, 64 | all pointer arithmetic |
| `keysize` | 1, 2, 4, 8, 16, 32 (binary); ignored in string modes | `is_key_equal:563`, `hmput_key:789` |
| `keyoffset` | `0` (what every stb_ds macro passes) and non-zero (`hmdel_key` param) | `is_key_equal:561,563`, `hmdel_key:843,845` |
| element count / table load | 0 · 1 · <6 · =6 (`used_count_threshold` for 8 slots) · >6 (grow to 16) · =12 · >12 (grow to 32) · hundreds (repeated grows) | `hmput_key:698` |
| insert kind | new key · duplicate key hit in the *forward* scan · duplicate key hit in the *wrap-around* scan · insert into a tombstone | `hmput_key:728,746,766` |
| delete kind | absent key · last element (`old_index == final_index`) · middle element (back-fill `memmove` + re-find) · delete that shrinks (`slot_count>>1`) · delete that rebuilds (tombstone overflow) | `hmdel_key:821,839,854,858` |
| get flavour | `hmget_key` (result in `header->temp`) · `hmget_key_ts` (result in caller's `ptrdiff_t*`) | `hmget_key:659`, `hmget_key_ts:631` |
| default element | absent · created by `hmput_default` · created implicitly by `hmput_key`/`hmget_key` on `NULL` | `hmput_default:669`, `hmput_key:686`, `hmget_key_ts:634` |
| `stbds_hash_bytes` input shape | `len` 0 · 1..7 (tail-only, `switch` cases 1-7) · 8 (one block, tail 0) · 9..15 · 16, 24, … (many blocks) · every `len % 8` residue · bytes with bit 7 set at `d[3]`/`d[7]` (the sign-extension quirk) | `siphash_bytes:522-541` |
| `seed` | `0` · `1` · default `0x31415926` · `SIZE_MAX` · random; plus the seed *sequence* driven by `stbds_rand_seed` + `make_hash_index:410-412` | `rand_seed:355`, `make_hash_index:406-413` |
| `stbds_hash_string` input shape | `""` · 1 char · 8 chars · 64 chars · bytes ≥ 0x80 (`(unsigned char)` cast) | `hash_string:480` |
| `stbds_arrgrowf` shape | `a` NULL/non-NULL × `min_cap` {`≤cap`, `<2*cap`, `≥2*cap`, `<4`} × `addlen` {0,1,n} × `elemsize` | `arrgrowf:283-292` |
| `stbds_stralloc` arena state | `remaining` {0, `≥len`, `<len`} × `storage` {NULL, non-NULL} × `block` {0,1,2,…,22,23,255} × `len` {1, 511, 512, 513, `blocksize`, `blocksize+1`, huge} | `stralloc:885-913` |
| `hm_geti(num)` | `num` ≤ 0 · 1 · 2 · 3 · odd/even · < 8 · around each rehash boundary · hundreds | `hm_geti:945` |

## Rows

`[x]` = passes across randomized inputs. Test file in the section heading.

Every row drives the library through **both** `.so`s via `libloading` and
compares an address-free snapshot of the full state after *every* operation:
array header (`length`, `capacity`, `temp`), hash index (`slot_count`,
`used_count`, all four thresholds, `tombstone_count`, `seed`, `slot_count_log2`),
the arena (`remaining`, `block`, `mode`, chain length), **every bucket's
`hash[8]` and `index[8]`**, and every element's bytes (keys compared by string
contents in the string modes). Machine addresses are deliberately excluded, as is
`hash_index::temp_key`, which `stbds_make_hash_index` leaves uninitialised.

Because addresses cannot be compared, three *whitebox invariants* are asserted on
every snapshot, for each library separately (they would otherwise be invisible):

* `hash_index::storage == STBDS_ALIGN_FWD((char*)(t+1), 64)` and 64-byte aligned,
* `malloc_usable_size(hash_table) >= (slot_count>>3)*128 + 104 + 63`,
* `malloc_usable_size(array_header) >= elemsize*capacity + 32`,

i.e. a wrong alignment expression or an under-sized `realloc` is caught even
though every comparable field would still match.

`mutation_check.sh` injects 33 single-line deviations from the C semantics into
`src/lib.rs` and checks the suite fails for each: 30 are killed and the remaining
3 are proven semantically-equivalent mutants (documented in `ERRORS.md`).

### A. Pure hash functions — lowest level (`tests/hash_fns.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p` = valid and `p = NULL`; seeds {0,1,0x31415926,SIZE_MAX} | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (tail-only; exercises `switch` cases 1-7 with full fall-through), 4096 random byte patterns × 8 random seeds | [x] |
| 3 | `stbds_hash_bytes` | `len = 8` exactly (one main-loop block, tail `case 0`), random bytes/seeds | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..15` (one block + tail 1..7), random | [x] |
| 5 | `stbds_hash_bytes` | `len = 16,24,32,…,256` (multi-block, tail 0), random | [x] |
| 6 | `stbds_hash_bytes` | `len` up to 300 random, **all bytes ≥ 0x80** — forces the `d[3]<<24` / `d[7]<<24` sign-extension into `size_t` | [x] |
| 7 | `stbds_hash_bytes` | `len` random, bytes chosen from {0x00,0x01,0x7f,0x80,0xff} only (boundary bytes) | [x] |
| 8 | `stbds_hash_string` | `""`, 1 char, 7/8/9 chars, 64 chars, 255 chars — ASCII, random seeds | [x] |
| 9 | `stbds_hash_string` | strings containing bytes 0x80..0xff (signed-`char` vs `(unsigned char)` cast) | [x] |
| 10 | `stbds_hash_string` | `hash < 2` search: many (string,seed) pairs, checking parity of the raw hash including values 0/1 | [x] |
| 11 | `stbds_rand_seed` + `stbds_hmput_key` | seed sequence: `rand_seed(s)` then N table creations ⇒ `table->seed` must follow the same LCG (`seed*a+b`) in both libs; s ∈ {0,1,0x31415926,SIZE_MAX,random} | [x] |
| 12 | `strkey` | `n` ∈ {0,1,-1,9,10,99,100,12345,-12345,INT_MAX,INT_MIN} + random | [x] |

### B. Dynamic array (`tests/array.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 13 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap=0` ⇒ early return `NULL` (no allocation); all elemsizes | [x] |
| 14 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap ∈ {1,2,3,4,5,7,8,100}` ⇒ `min_cap<4` clamp to 4; elemsize ∈ {1,2,4,8,16,24,32,64} | [x] |
| 15 | `stbds_arrgrowf` | `a=NULL`, `addlen ∈ {1,3,4,5,17}`, `min_cap=0` ⇒ `min_len` wins | [x] |
| 16 | `stbds_arrgrowf` | existing array, `min_cap ≤ cap` ⇒ same pointer returned, header unchanged | [x] |
| 17 | `stbds_arrgrowf` | existing array, `min_cap` in `(cap, 2*cap)` ⇒ capacity doubles instead | [x] |
| 18 | `stbds_arrgrowf` | existing array, `min_cap ≥ 2*cap` ⇒ capacity = `min_cap` | [x] |
| 19 | `stbds_arrgrowf` | randomized growth *pipeline*: 200 steps of `arrmaybegrow`-style pushes (`length` bumped by the caller) with random `addlen`, comparing header + payload each step; elemsize ∈ {1,4,8,16,24} | [x] |
| 20 | `stbds_arrfreef` | free a fresh array, and free an array after several grows | [x] |

### C. Binary hash map, low-level entry points (`tests/hashmap.rs`)

All rows: `keyoffset = 0`, `mode = 0`, driven exactly like the `stbds_hmput`/
`stbds_hmget`/`stbds_hmdel` macros (caller writes key+value at `header->temp`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 21 | `stbds_hmput_default` | `a=NULL` ⇒ create; then `hmput_default` again ⇒ idempotent; then set a default value | [x] |
| 22 | `stbds_hmput_default` | applied to a raw `arrgrowf` array with `length == 0` (the `length==0` branch) | [x] |
| 23 | `stbds_hmget_key` / `_ts` | on `NULL` map (no table at all) ⇒ `temp == -1` + fresh default element | [x] |
| 24 | `stbds_hmget_key` / `_ts` | on a map that has a default element but **no hash table** ⇒ `temp == -1` | [x] |
| 25 | `stbds_hmput_key` + `hmget_key` | elemsize 8 / keysize 4 (the `hm_geti` shape); 0,1,2,5,6,7,8 inserts — crosses the 8→16 rehash at `used_count == 6` | [x] |
| 26 | `stbds_hmput_key` + `hmget_key` | elemsize 8 / keysize 4; 12,13 inserts — crosses the 16→32 rehash at `used_count == 12` | [x] |
| 27 | `stbds_hmput_key` + `hmget_key` | elemsize 8 / keysize 4; 300 random inserts — several consecutive rehashes | [x] |
| 28 | `stbds_hmput_key` + `hmget_key` | elemsize 8 / keysize 8 (key fills the whole element) | [x] |
| 29 | `stbds_hmput_key` + `hmget_key` | elemsize 16 / keysize 8 | [x] |
| 30 | `stbds_hmput_key` + `hmget_key` | elemsize 16 / keysize 4 (`memcpy` copies only 4 of 16 bytes; caller fills the rest) | [x] |
| 31 | `stbds_hmput_key` + `hmget_key` | elemsize 24 / keysize 16 (the `stbds_struct2` `int key[2]` shape) | [x] |
| 32 | `stbds_hmput_key` + `hmget_key` | elemsize 32 / keysize 32 | [x] |
| 33 | `stbds_hmput_key` + `hmget_key` | elemsize 4 / keysize 4, and elemsize 2 / keysize 2, and elemsize 1 / keysize 1 (tiny elements, `hash_bytes` tail-only) | [x] |
| 34 | `stbds_hmput_key` | duplicate keys: re-put every key twice (update path, `temp` = existing index, `length` unchanged) | [x] |
| 35 | `stbds_hmget_key` vs `stbds_hmget_key_ts` | same key sequence through both flavours, hits and misses interleaved | [x] |
| 36 | `stbds_hmdel_key` | delete an absent key (no-op, `temp = 0`) | [x] |
| 37 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`, no back-fill) | [x] |
| 38 | `stbds_hmdel_key` | delete a **middle** element (back-fill `memmove` + re-find + `index` patch) | [x] |
| 39 | `stbds_hmdel_key` | delete-until-empty on an 8-slot table ⇒ `shrink_threshold == 0`, so only the *tombstone rebuild* (`tombstone_count > 1`) can fire | [x] |
| 40 | `stbds_hmdel_key` | 16-slot table, delete down past `used_count < 4` ⇒ **shrink** to 8 slots | [x] |
| 41 | `stbds_hmdel_key` | 32-slot table, delete to force **shrink twice** (32→16→8) | [x] |
| 42 | `stbds_hmput_key` after deletes | re-insert into a table full of tombstones ⇒ the `tombstone >= 0` reuse path | [x] |
| 43 | mixed pipeline | 4000 randomized ops (put / get / get_ts / del / put_default) with a small key space (heavy collisions, tombstones, grows and shrinks), snapshot-compared every op; elemsize/keysize ∈ {(8,4),(16,8),(24,16)} | [x] |
| 44 | `stbds_hmdel_key` | non-zero `keyoffset` ∈ {0,4,8,12} inside a 16-byte element. `stbds_hmput_key` hard-codes `keyoffset = 0`, so any non-zero value compares the *wrong* bytes of the element; the resulting (usually no-op) behaviour must still be bit-identical. All offsets stay inside the element so only bytes the test wrote are read | [x] |
| 45 | `stbds_hmfree_func` | free a map with a table (binary mode, `string.mode == 0`) at length 1 and at length N | [x] |
| 46 | `stbds_hmfree_func` | free a map that has a default element but no hash table | [x] |
| 47 | `mode` classification | the whole put/get/del cycle with `mode ∈ {0, -1, INT_MIN}` (all binary) — must be indistinguishable from `mode = 0` | [x] |

### D. String hash map + arena (`tests/strmap.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 48 | `stbds_shmode_func` | `mode = STBDS_SH_NONE(0)` ⇒ table exists, `string.mode = 0`, so `hmput_key` takes the `memcpy` default branch even with `mode = 1` | [x] |
| 49 | `stbds_shmode_func` | `mode = STBDS_SH_DEFAULT(1)` + `hmput_key(mode=1)`: key pointer stored by value, `temp_key` set | [x] |
| 50 | `stbds_shmode_func` | `mode = STBDS_SH_STRDUP(2)` + `hmput_key(mode=1)`: `strdup`'d keys, freed by `hmdel_key` and `hmfree_func` | [x] |
| 51 | `stbds_shmode_func` | `mode = STBDS_SH_ARENA(3)` + `hmput_key(mode=1)`: keys copied into the arena; drives `stralloc` block chain | [x] |
| 52 | `stbds_shmode_func` | out-of-range `mode ∈ {4, 5, 255, 256, 300, -1, INT_MAX, INT_MIN}` ⇒ `(unsigned char)` truncation, then the `default:` `memcpy` branch | [x] |
| 53 | implicit arena mode | `hmput_key` on `NULL` map with `mode = 1` ⇒ `string.mode` becomes `STBDS_SH_DEFAULT` automatically | [x] |
| 54 | implicit arena mode | `hmput_key` on `NULL` map with `mode = 0` ⇒ `string.mode` stays `0` | [x] |
| 55 | string map pipeline | `SH_DEFAULT`: 0,1,5,6,7,20,200 distinct keys (`strkey`-style + random ASCII, lengths 1..80) ⇒ rehashes; get hits/misses; `temp_key` checked | [x] |
| 56 | string map pipeline | `SH_STRDUP`: same shapes; plus deletes (which `free` the key when `mode == 1`) | [x] |
| 57 | string map pipeline | `SH_ARENA`: same shapes with keys long enough to span several arena blocks (lengths 1, 200, 511, 512, 513, 2000) | [x] |
| 58 | `mode ≥ 2` string map | `hmput_key`/`hmget_key` with `mode ∈ {2,3,999,INT_MAX}` — string hashing, `temp_key` **not** set in the wrap-around scan; delete restricted to the last element (row 39 of ERRORS.md) | [x] |
| 59 | mixed string pipeline | 2000 randomized ops over a 40-key space for each of `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA`, snapshot-compared every op | [x] |
| 60 | `stbds_stralloc` | fresh zeroed arena, `len ∈ {1,2,511,512}` ⇒ first 512-byte block, `remaining` bookkeeping | [x] |
| 61 | `stbds_stralloc` | fill a block exactly, then one more byte ⇒ next block at `512 << (block>>1)` | [x] |
| 62 | `stbds_stralloc` | `len > blocksize` with `storage == NULL` ⇒ oversize head block, `remaining = 0` | [x] |
| 63 | `stbds_stralloc` | `len > blocksize` with `storage != NULL` ⇒ oversize block spliced after head, `remaining` preserved | [x] |
| 64 | `stbds_stralloc` | `block` pre-set to {0,1,2,3,10,21,22,23,44,255} ⇒ block-size ladder incl. the `1<<20` clamp and the masked shift | [x] |
| 65 | `stbds_stralloc` | 500 randomized allocations (lengths 1..3000) on one arena, comparing every returned string and the whole arena state | [x] |
| 66 | `stbds_strreset` | zeroed arena (no blocks) · arena with 1 block · arena with many blocks incl. oversize ones | [x] |

### E. Composed public entry point (`tests/hm_geti.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 67 | `hm_geti` | `num ∈ {INT_MIN,-1,0,1,2,3,4,5,6,7,8,9,11,12,13,15,16,17,24,25,31,32,33,63,64,65,100,127,128,257,1000}` — every rehash/shrink boundary of the internal int map; all 12 asserts must stay quiet in both libs | [x] |
| 68 | `hm_geti` | called repeatedly (the global `stbds_hash_seed` advances) with `rand_seed` re-pinned and *not* re-pinned ⇒ identical seed evolution in both libs | [x] |
| 69 | `hm_geti` + `strkey` | interleaved with the rest of the API to prove the shared global state (`stbds_hash_seed`, `buffer`) behaves identically | [x] |

### F. High-load table states (`tests/hashmap.rs`) and degenerate sizes (`tests/errors.rs`)

Insert-driven rehashes always land ≈3 entries per 8-slot bucket, which is too
sparse for `stbds_make_hash_index`'s *quadratic* re-probe (`pos += step;
step += STBDS_BUCKET_LENGTH`) to ever be walked more than once. The **tombstone
rebuild** re-inserts a nearly-full table into the *same* number of slots (≈6
entries per bucket), which does overflow buckets — so it needs its own rows.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 70 | `stbds_hmput_key` + `stbds_hmdel_key` | 1024-slot table filled to `used_count = 760` (threshold 768), then 600 × (delete one existing key / insert one brand-new key) so `tombstone_count` climbs past its 192 threshold and forces a **rebuild at ≈6 entries per bucket**, walking deep probe chains inside `stbds_make_hash_index`; every surviving key read back afterwards; elemsize/keysize ∈ {(8,4),(16,8)} | [x] |
| 70b | same | the same high-load churn under 12 different global seeds, so the bucket-occupancy distribution (and therefore the probe-chain lengths) varies | [x] |
| 71 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` | `keysize == 0` — `memcmp(...,0)` makes every key equal and `hash_bytes(k,0,seed)` makes every key hash alike ⇒ the map collapses to one entry | [x] (`e57`) |
| 72 | all `hm*` entry points | `elemsize == 0` (with `keysize == 0`) — every element aliases the same zero-sized address; nothing is ever written to an element | [x] (`e58`) |
