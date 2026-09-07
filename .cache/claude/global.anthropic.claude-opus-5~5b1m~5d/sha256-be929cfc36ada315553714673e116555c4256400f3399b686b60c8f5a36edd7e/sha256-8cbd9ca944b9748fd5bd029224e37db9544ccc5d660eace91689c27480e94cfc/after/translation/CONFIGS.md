# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branch points in `c_src/src/lib.c`.

## Axes the C code actually branches on

**A. hash `mode` (`int`, passed to `hmput_key`/`hmget_key{,_ts}`/`hmdel_key`)**
`stbds_is_key_equal` / `hm_find_slot` / `hmput_key` branch on `mode >= STBDS_HM_STRING (1)`;
`hmdel_key`'s strdup-free branches on `mode == 1` exactly.
→ values that matter: `0` (binary), `1` (string), `2` (string-compare but not
strdup-freeing), `-1` (binary).

**B. arena/key-ownership `string.mode` (`unsigned char`, set by `stbds_shmode_func`
or implicitly by `hmput_key`)** — drives the `switch` at :785:
`0 / STBDS_SH_NONE` → `memcpy` of `keysize` bytes; `1 / STBDS_SH_DEFAULT` → store
caller's `char*`; `2 / STBDS_SH_STRDUP` → `strdup`; `3 / STBDS_SH_ARENA` →
`stralloc`; anything else → `default:` `memcpy`.

**C. map construction path**: fresh `NULL` map + `hmput_key` (implicit
`string.mode = 0` or `1`), vs `stbds_shmode_func(elemsize, mode)` (explicit
`string.mode`), vs `stbds_hmput_default` only (map with `hash_table == NULL`).

**D. element / key shape**: `elemsize` (8, 12, 16, 24, 40), `keysize`
(0, 1, 2, 4, 8, 16 = `elemsize`), `keyoffset` (0, and non-zero for `hmdel_key`
which takes it as a parameter).

**E. table size / occupancy**: `slot_count` starts at 8 and doubles when
`used_count >= used_count_threshold` (`slot_count - slot_count/4`), so growth at
6, 12, 24, 48… entries; shrink when `used_count < slot_count>>2 && slot_count > 8`;
tombstone rebuild when `tombstone_count > (slot_count>>3)+(slot_count>>4)`.
→ counts: 0, 1, 5, 6, 7, 12, 13, 50, 200, 2000.

**F. seed**: default `0x31415926`, plus `stbds_rand_seed(s)` for `s ∈ {0, 1,
SIZE_MAX, random}`; the seed also self-mutates (`seed*a+b`) on every fresh
`make_hash_index`, so the *sequence* of allocations is observable.

**G. hash input shape**: `stbds_hash_bytes` `len ∈ 0..=64` (covers all 8
`switch` fall-through arms and the multi-`size_t` block loop) with byte values
including `>= 0x80` (int sign-extension paths at :523/:536); `stbds_hash_string`
lengths 0..64 with ASCII, high-bit (`>=0x80`) and embedded-space bytes.

**H. `stbds_arrgrowf` shape**: `a` NULL vs non-NULL; `addlen ∈ {0,1,7,1000}`;
`min_cap` below/equal/above `2*cap` and below the `4` floor; `elemsize ∈ {0,1,8,40}`.

**I. arena shape** (`stbds_stralloc`): fits in `remaining`; needs a new block;
oversize (`len > blocksize`) with `storage == NULL` vs `!= NULL`; `block`
escalating 0→22+ past `BLOCKSIZE_MAX`.

**J. operation mixes**: put-only; put+get; put+overwrite (same key twice);
put+delete+re-put (tombstone reuse); delete-down-to-empty (shrink chain);
interleaved random put/get/del; `hmput_default` before/after `hmput_key`.

## Rows

Every row is exercised with many randomized inputs (fixed seed `0xC0FFEE`),
driving **both** `.so`s through the identical operation sequence and comparing
every observable: returned `temp` index, `header->length`, `header->capacity`,
the full element array bytes, and the returned `size_t` hashes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0..=64` × 64 random buffers each, seed = default | [x] |
| 2 | `stbds_hash_bytes` | `len = 0..=64`, buffers of all-`0xFF` / high-bit bytes (int sign-extension at :523,:536) | [x] |
| 3 | `stbds_hash_bytes` | `len = 0..=64` × seeds `{0, 1, 0x31415926, SIZE_MAX, random×8}` | [x] |
| 4 | `stbds_hash_string` | `len = 0..=64` random ASCII, seed = default | [x] |
| 5 | `stbds_hash_string` | strings with bytes `>= 0x80` (the `(unsigned char)` cast at :481) + `""` | [x] |
| 6 | `stbds_hash_string` | `len = 0..=32` × seeds `{0, 1, SIZE_MAX, random×8}` | [x] |
| 7 | `stbds_rand_seed` + `stbds_hash_bytes`/`_string` | seed set to `{0,1,SIZE_MAX}` then hash — verifies the global is per-`.so` and identical | [x] |
| 8 | `stbds_arrgrowf` | `a = NULL`, `elemsize ∈ {1,4,8,12,40}`, `addlen ∈ {0,1,7}`, `min_cap ∈ {0,1,3,4,5,1000}` (cap floor + doubling) | [x] |
| 9 | `stbds_arrgrowf` | non-NULL `a`, repeated growth chain (`addlen=1` ×512) — capacity doubling sequence must match exactly | [x] |
| 10 | `stbds_arrgrowf` | non-NULL `a`, `min_cap <= arrcap` (no-grow fast path) and `min_cap == arrcap+1` (one step past) | [x] |
| 11 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free, `elemsize = 0` (degenerate zero-size element) | [x] |
| 12 | `stbds_hmput_key` (binary) | fresh NULL map, `mode=0`, `elemsize=8/keysize=4` (int→int), 1 insert | [x] |
| 13 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8/keysize=4`, counts `{5,6,7}` — straddles the first `used_count_threshold` growth | [x] |
| 14 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8/keysize=4`, counts `{12,13,50,200,2000}` random distinct keys — multiple doublings | [x] |
| 15 | `stbds_hmput_key` (binary) | `mode=0`, duplicate keys (overwrite path at :729) — random keys drawn from a small domain so ~50% collide | [x] |
| 16 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=16/keysize=8` (u64 key) and `elemsize=24/keysize=16` (2×u64 key) | [x] |
| 17 | `stbds_hmput_key` (binary) | `mode=0`, `keysize=1` and `keysize=2` (sub-word keys, many collisions) | [x] |
| 18 | `stbds_hmput_key` (binary) | `mode=-1` (negative, out-of-enum) — must take the same binary path as `mode=0` | [x] |
| 19 | `stbds_hmget_key` (binary) | after row 14's inserts: get every present key + 200 absent keys, compare `temp` | [x] |
| 20 | `stbds_hmget_key_ts` (binary) | same as row 19 but via the `ptrdiff_t *temp` out-param; header `temp` must stay untouched | [x] |
| 21 | `stbds_hmdel_key` (binary) | `mode=0`, `keyoffset=0`, delete a random 1/3 of row 14's keys, then re-get all | [x] |
| 22 | `stbds_hmdel_key` (binary) | delete **all** keys one by one from a 200-entry map — drives the shrink chain 512→8 and the tombstone rebuild | [x] |
| 23 | `stbds_hmdel_key` (binary) | put/del/put interleaved (tombstone reuse at :766) — 2000 random ops over a 64-key domain | [x] |
| 24 | `stbds_hmdel_key` (binary) | `keyoffset != 0`: `elemsize=16`, key at offset 8, `keysize=4` | [x] |
| 25 | `stbds_hmput_key` (string, implicit) | fresh NULL map, `mode=1` ⇒ `string.mode = STBDS_SH_DEFAULT` (key pointer stored), 200 random distinct strings | [x] |
| 26 | `stbds_hmput_key` (string) | `mode=1`, `SH_DEFAULT`, duplicate string keys (overwrite + `temp_key` write-back at :733) | [x] |
| 27 | `stbds_shmode_func` + `hmput_key` | `STBDS_SH_STRDUP (2)`, `mode=1`, 200 random strings, len 0..40 | [x] |
| 28 | `stbds_shmode_func` + `hmput_key` + `hmdel_key` | `STBDS_SH_STRDUP`, `mode=1`, put then delete (frees the strdup'd key at :836) | [x] |
| 29 | `stbds_shmode_func` + `hmput_key` | `STBDS_SH_ARENA (3)`, `mode=1`, 500 strings incl. very long ones (>512 and >1<<20 ⇒ oversize block path) | [x] |
| 30 | `stbds_shmode_func` + `hmput_key` | `STBDS_SH_NONE (0)` with `mode=1`: string *hashing/comparison* but `default:` `memcpy` key storage | [x] |
| 31 | `stbds_shmode_func` + `hmput_key` | `mode=2` (out-of-enum but `>= STBDS_HM_STRING`) with `string.mode` = `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA` — string hash + compare path (put/get only: deleting a non-last element with `mode != 1` aborts in the C, see ERRORS row 44) | [x] |
| 32 | `stbds_shmode_func` | `elemsize ∈ {8,16,24,40}` — element-0 zeroing, `length = 1`, fresh 8-slot table | [x] |
| 33 | `stbds_hmput_default` | on `NULL`; then `hmput_key` on the result; then `hmput_default` again (no-op) | [x] |
| 34 | `stbds_hmput_default` + `stbds_hmget_key` | map with `hash_table == NULL` ⇒ `temp = -1` path | [x] |
| 35 | `stbds_hmfree_func` | `SH_NONE`, `SH_DEFAULT`, `SH_STRDUP`, `SH_ARENA` maps, populated and empty | [x] |
| 36 | `stbds_stralloc` | fresh zeroed arena, 300 strings len 0..40 (block chain 512, 512, 1024, …) | [x] |
| 37 | `stbds_stralloc` | oversize first string (len > 512) into an empty arena (`storage == NULL` branch, `remaining = 0`) | [x] |
| 38 | `stbds_stralloc` | oversize string after the arena already has storage (`next`-splice branch) | [x] |
| 39 | `stbds_stralloc` | `block` escalated by 44 small-block allocations past the `1<<20` ceiling | [x] |
| 40 | `stbds_strreset` | after each of rows 36–39, plus on an all-zero arena, plus twice in a row | [x] |
| 41 | `strkey` | `n ∈ {0,1,-1,9,11,INT_MIN,INT_MAX}` + 200 random ints; full 256-byte static buffer compared | [x] |
| 42 | `intput` | `num ∈ {INT_MIN,-1,0,1,8,10,12,INT_MAX}` + 200 random ints excluding 9/11 (9/11 abort → ERRORS rows 38/39) | [x] |
| 43 | end-to-end pipeline | `hmput_default` → 300 binary puts → 100 gets → 100 dels → `hmfree_func`, seeded random, comparing the whole element array after every step | [x] |
| 44 | end-to-end pipeline | `sh_new_strdup` → 300 string puts → gets → dels → `hmfree_func`, comparing key *contents* (not pointers) after every step | [x] |
| 45 | end-to-end pipeline | `sh_new_arena` → 300 string puts → gets → dels → `strreset`/`hmfree_func` | [x] |

| 46 | `stbds_arrgrowf` + `stbds_hmput_default` + `hmput_key` | `hmput_default` applied to a **length-0** array from `arrgrowf` — the `header->length == 0` half of the guard that the NULL-argument half hides; `elemsize ∈ {8,16,40}` | [x] |
| 47 | `stbds_shmode_func` + `hmput_key` | `stbds_temp_key` (the scratch pointer the `hmputs`/`shputs` macros read) after a fresh insert **and** after re-putting an existing key, for `string.mode ∈ {SH_DEFAULT, SH_STRDUP, SH_ARENA}` × `mode ∈ {1,2}` | [x] |

No binary/driver target is produced by either build (`CMakeLists.txt` builds
only a `SHARED` library; `Cargo.toml` declares only `crate-type = ["cdylib"]`),
so the "compare stdout of the C and Rust binaries" gate is **N/A**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default one. Verified mechanically:

```sh
python3 - <<'EOF'
import tomllib; print(tomllib.load(open('Cargo.toml','rb')).get('features', {}))
EOF
# -> {}
```

`cargo test --no-default-features` is therefore equivalent to `cargo test` and
is run as such by `run_all.sh`.

## Result

```sh
cd translation && cargo test --offline --test phase_b -- --test-threads=1
# 47 passed; 0 failed
cd translation && cargo test --offline --test phase_d -- --test-threads=1
# 5 passed; 0 failed   (symbol parity + 1200-round cross-configuration fuzz)
```

Every row passes across its randomized inputs. On top of the table, Phase D
runs a 1200-round randomized *configuration* fuzz (`d_03`) that redraws
`elemsize`, `keysize`, `mode`, `string.mode` and the global seed each round and
then applies a random put/get/delete sequence, plus 4000-iteration hash and
arena fuzzes (`d_04`) and 5000 `strkey` / 300 `intput` calls (`d_05`).

Observables compared on every step (never pointer values, which legitimately
differ between two `malloc` arenas):

* `stbds_header`: `length`, `capacity`, `temp`;
* `stbds_hash_index`: `slot_count`, `used_count`, `used_count_threshold`,
  `used_count_shrink_threshold`, `tombstone_count`, `tombstone_count_threshold`,
  `seed`, `slot_count_log2`, `string.{remaining,block,mode}`;
* **every bucket**: all `hash[8]` and `index[8]` entries of every bucket — these
  are byte-identical because both libraries evolve the global
  `stbds_hash_seed` in lock-step;
* every element's bytes (with `char *` key fields compared by pointed-to text);
* `stbds_temp_key` where the C actually writes it (row 47);
* the raw `size_t` return of `stbds_hash_bytes` / `stbds_hash_string` and the
  `char *` text of `stbds_stralloc` / `strkey`.
