# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from the branch conditions in `c_src/src/lib.c`.  There is
no `#ifdef`-selected feature in the C (the single TU hard-codes
`STBDS_BUCKET_LENGTH 8`, `STBDS_SIPHASH_C_ROUNDS 2`, `STBDS_SIPHASH_D_ROUNDS 4`,
`STBDS_REALLOC`/`STBDS_FREE` = `realloc`/`free`, `STBDS_ASSERT` = `assert`, and
`sizeof(size_t) == 8` is enforced by a negative-array-size typedef), and the Rust
crate declares **no `[features]`** — so there is exactly one build
configuration.  All variability is therefore *runtime*.

## Axes the C actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` (hash/compare mode) | `mode >= STBDS_HM_STRING(1)` → `stbds_hash_string`+`strcmp`; `mode < 1` → `stbds_hash_bytes`+`memcmp`. `stbds_hmdel_key` additionally tests `mode == 1` **exactly** | lib.c:560, 594, 706, 855, 858 |
| `table->string.mode` (key ownership) | `STBDS_SH_NONE(0)`/`default` → `memcpy` key bytes; `STBDS_SH_DEFAULT(1)` → store caller pointer; `STBDS_SH_STRDUP(2)` → `stbds_strdup`; `STBDS_SH_ARENA(3)` → `stbds_stralloc` | lib.c:789-793 |
| how the map is created | implicitly by `hmput_key(NULL,…)`, by `hmget_key(NULL,…)`, by `hmput_default(NULL,…)`, or explicitly by `shmode_func(elemsize,mode)` | lib.c:634, 670, 682, 798 |
| `elemsize` | any; drives header/payload stride. `8` (ptr only), `16` (`{char*,int}`), `12`/`24`/`40` (odd strides) | everywhere |
| `keysize` | `0`, `1`, `2`, `4`, `8`, `16`, `>8`; only used by `memcmp`/`memcpy`/`hash_bytes` in binary mode | lib.c:563, 706, 792 |
| `keyoffset` | `0` (all `hm*`/`sh*` macros pass 0 or `STBDS_OFFSETOF(t,key)` = 0 for key-first structs); non-zero is reachable through the raw `stbds_hmdel_key` ABI | lib.c:808 |
| table `slot_count` | grows `8 → 16 → 32 → …` when `used_count >= slot_count-(slot_count>>2)`; shrinks `>>1` when `used_count < slot_count>>2` | lib.c:701, 866 |
| array `capacity` growth path | `min_cap <= cap` (no-op), `min_cap < 2*cap` (double), `min_cap < 4` (bump to 4), else exact | lib.c:287-293 |
| probe path | in-bucket forward scan (`i = pos&7 .. 7`) vs wrap-around scan (`i = 0 .. pos&7`) vs next-bucket step (`step += 8`) | lib.c:604-627, 719-751 |
| tombstones | `hmput_key` reuses the first tombstone; `hmdel_key` triggers rebuild when `tombstone_count > (sc>>3)+(sc>>4)` | lib.c:754, 870 |
| global `stbds_hash_seed` | mutated by every fresh `stbds_make_hash_index(_, NULL)`; resettable via `stbds_rand_seed` | lib.c:359, 419 |
| `stbds_hash_bytes` length shape | `len==0`; `len<8` (tail cases 1..7); `len==8k`; `len==8k+r` (r=1..7); high-bit bytes (sign-extension) | lib.c:521-543 |
| `stbds_hash_string` input shape | `""`; ASCII; bytes ≥ 0x80; long (> 64 chars, rotate wrap) | lib.c:481-486 |
| arena block progression | `remaining` sufficient; `len <= blocksize` (fresh 512<<(b>>1) block); `len > blocksize` with/without existing `storage`; `block` saturation at 1 MiB | lib.c:883-918 |
| `str_put` / `strkey` input | `num <= 0`, `1`, small, large enough to grow arena blocks; `n` negative | lib.c:941-967 |

## Rows (one per meaningful combination)

Every row is exercised in `tests/configs.rs` against **both** `.so`s with many
seeded-random inputs (`SEED = 0x5EED_1234_ABCD_0001`), not a single value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, random seeds | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (each tail `case`), random bytes incl. `>= 0x80`, random seeds | [x] |
| 3 | `stbds_hash_bytes` | `len = 8, 16, 24, 32` (exact multiples, full-word loop only), random bytes | [x] |
| 4 | `stbds_hash_bytes` | `len = 8k + r`, `k = 1..8`, `r = 1..7` (loop + tail), random bytes | [x] |
| 5 | `stbds_hash_bytes` | `len = 1..40` with all-`0xFF` / all-`0x00` / high-bit-only buffers (sign-extension corners) | [x] |
| 6 | `stbds_hash_bytes` | `len` large (256, 1024, 4096) random buffers, seed = 0 / `usize::MAX` / random | [x] |
| 7 | `stbds_hash_string` | `""`, random seeds | [x] |
| 8 | `stbds_hash_string` | random ASCII, length 1..64, random seeds | [x] |
| 9 | `stbds_hash_string` | random bytes `0x80..0xFF` (no NUL), length 1..64 | [x] |
| 10 | `stbds_hash_string` | long strings (256, 4096 chars) | [x] |
| 11 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to 0 / 1 / `0x31415926` / random; observe `HashIndex::seed` of the next 8 fresh indices (global-state progression) | [x] |
| 12 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 0` → early-out returning `NULL` | [x] |
| 13 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 1..8` (the `< 4` bump), several `elemsize` | [x] |
| 14 | `stbds_arrgrowf` | `a = NULL`, `addlen = 1..64`, `min_cap = 0` | [x] |
| 15 | `stbds_arrgrowf` | existing `a`, `min_cap <= cap` (no-op path) — pointer and header unchanged | [x] |
| 16 | `stbds_arrgrowf` | existing `a`, repeated `addlen = 1` pushes → doubling sequence `4,8,16,…` over 200 iterations, `elemsize ∈ {1,4,8,12,16,40}` | [x] |
| 17 | `stbds_arrgrowf` | existing `a`, `min_cap` far above `2*cap` (exact-size path) | [x] |
| 18 | `stbds_stralloc` | fresh arena, single string, `len` ∈ {1, 2, 511, 512, 513} (block-boundary shapes) | [x] |
| 19 | `stbds_stralloc` | fresh arena, many random strings (len 1..100, 300 of them) → block chain + `block` counter progression | [x] |
| 20 | `stbds_stralloc` | fresh arena, first string `len > 512` (oversized, `storage == NULL` path) then more strings | [x] |
| 21 | `stbds_stralloc` | arena with existing head block, then `len > blocksize` (oversized splice-after-head path, `remaining` preserved) | [x] |
| 22 | `stbds_stralloc` | drive `a->block` to saturation (strings sized to force ≥ 12 block allocations, 512→1 MiB) | [x] |
| 22b | `stbds_stralloc` | explicit sweep of the caller-visible `a->block` field over its whole reachable range `0..=22` × `a->mode ∈ {0,1,3,255}` × `len ∈ {1,8,600,5000}`, each followed by 5 more allocations | [x] |
| 23 | `stbds_stralloc` + `stbds_strreset` | alloc N, reset, alloc N again (arena reuse; `block`/`remaining`/`storage` all zeroed) | [x] |
| 24 | `stbds_strreset` | fresh/zeroed arena (no blocks) — idempotent | [x] |
| 25 | `stbds_hmput_default` | `a = NULL`, `elemsize ∈ {8,12,16,40}` | [x] |
| 26 | `stbds_hmput_default` | called twice (second call is the `length != 0` no-op) | [x] |
| 27 | `stbds_hmput_default` | on a map already built by `hmput_key` (no-op, default element preserved) | [x] |
| 28 | `stbds_hmget_key_ts` | `a = NULL` (bootstrap + `*temp = -1`) | [x] |
| 29 | `stbds_hmget_key_ts` | array with no index (`hash_table == NULL`), binary and string mode | [x] |
| 30 | `stbds_hmget_key_ts` | populated binary map, hit and miss, random `u32`/`u64`/16-byte keys | [x] |
| 31 | `stbds_hmget_key_ts` | populated string map, hit and miss | [x] |
| 32 | `stbds_hmget_key` | same as 28–31, additionally asserting `header->temp` | [x] |
| 33 | `stbds_hmput_key` | mode = BINARY, `string.mode = 0`, `keysize = 4`, `elemsize = 8`, 1 insert | [x] |
| 34 | `stbds_hmput_key` | mode = BINARY, `keysize ∈ {1,2,4,8,16}`, `elemsize ∈ {8,16,24,40}`, 1..300 random inserts (drives `slot_count` 8→512) | [x] |
| 35 | `stbds_hmput_key` | mode = BINARY, duplicate keys re-put (in-bucket hit path, `temp` = existing index, length unchanged) | [x] |
| 36 | `stbds_hmput_key` | mode = BINARY, keys engineered to collide in the same bucket (forces wrap-around scan + `step += 8` probing) | [x] |
| 37 | `stbds_hmput_key` | mode = STRING via implicit create (`string.mode` becomes `STBDS_SH_DEFAULT`), 1..300 random keys | [x] |
| 38 | `stbds_hmput_key` | mode = STRING, duplicate keys re-put → `temp_key` update path | [x] |
| 39 | `stbds_hmput_key` on `shmode_func(_, STBDS_SH_STRDUP)` | mode = STRING, 1..300 random keys (keys `strdup`ed, then `hmfree_func` frees them) | [x] |
| 40 | `stbds_hmput_key` on `shmode_func(_, STBDS_SH_ARENA)` | mode = STRING, 1..300 random keys (keys arena-copied; exercises `stralloc` inside the map) | [x] |
| 41 | `stbds_hmput_key` on `shmode_func(_, STBDS_SH_NONE)` | mode = STRING (hash/compare as string) but `default:` `memcpy` of the `char*` key bytes | [x] |
| 42 | `stbds_hmput_key` on `shmode_func(_, STBDS_SH_DEFAULT)` | mode = STRING, keys are caller-owned pointers stored verbatim | [x] |
| 43 | `stbds_hmput_key` on `shmode_func(_, STBDS_SH_NONE)` | mode = BINARY, `keysize ∈ {4,8}` | [x] |
| 44 | `stbds_hmput_key` | insert exactly up to and past `used_count_threshold` (6, 12, 24, 48 …) to hit each rehash boundary and check the rehashed bucket layout | [x] |
| 45 | `stbds_hmdel_key` | `keyoffset = 0`, mode = BINARY, delete existing key (mid-array → last-element move-down + slot re-find) | [x] |
| 46 | `stbds_hmdel_key` | mode = BINARY, delete the **last** element (`old_index == final_index`, no move) | [x] |
| 47 | `stbds_hmdel_key` | mode = BINARY, delete-all in random order (300 keys) → repeated shrink/rebuild | [x] |
| 48 | `stbds_hmdel_key` | mode = BINARY, interleaved put/del/get, 2000 random ops (tombstone reuse) | [x] |
| 49 | `stbds_hmdel_key` | mode = STRING on `STBDS_SH_DEFAULT` map | [x] |
| 50 | `stbds_hmdel_key` | mode = STRING on `STBDS_SH_STRDUP` map (the `free()`-the-key branch) | [x] |
| 51 | `stbds_hmdel_key` | mode = STRING on `STBDS_SH_ARENA` map (no free; arena keeps the bytes) | [x] |
| 52 | `stbds_hmdel_key` | delete enough to cross `used_count_shrink_threshold` with `slot_count > 8` (shrink path) | [x] |
| 53 | `stbds_hmdel_key` | delete/re-put pattern that crosses `tombstone_count_threshold` (same-size rebuild path) | [x] |
| 54 | `stbds_hmdel_key` | non-zero `keyoffset` (raw ABI): `elemsize = 24`, key at offset 8, binary and string mode | [x] |
| 55 | `stbds_hmfree_func` | `string.mode = 0` map | [x] |
| 56 | `stbds_hmfree_func` | `string.mode = STBDS_SH_STRDUP` map with N keys (frees elements `1..length`) | [x] |
| 57 | `stbds_hmfree_func` | `string.mode = STBDS_SH_ARENA` map (arena `strreset`) | [x] |
| 58 | `stbds_hmfree_func` | array with `hash_table == NULL` | [x] |
| 59 | `stbds_shmode_func` | `elemsize ∈ {8,16,24,40}` × `mode ∈ {0,1,2,3}` — full cross-product of the `STBDS_SH_*` enum | [x] |
| 60 | `strkey` | `n ∈ {0, 1, 9, 10, 99, 100, 12345, -1, -12345, INT_MIN, INT_MAX}` + 200 random `i32` | [x] |
| 61 | `str_put` | `num ∈ {0, 1, 2, 7, 8, 64, 100, 1000}` — stdout captured and compared byte-for-byte | [x] |
| 62 | `str_put` | `num < 0` (`-1`, `INT_MIN`) — loop skipped | [x] |
| 63 | end-to-end pipeline | `shmode_func(STRDUP)` → 200 `hmput_key` → 100 `hmget_key` → 100 `hmdel_key` → 100 `hmput_key` → `hmfree_func`, full structural dump compared after **every** op | [x] |
| 64 | end-to-end pipeline | same as 63 for `ARENA`, `DEFAULT`, `NONE`, and BINARY-mode maps | [x] |
| 65 | end-to-end pipeline | `hmput_default` + `hmput_key` + `hmget_key` mixed with an explicit `stbds_rand_seed` reseed between maps | [x] |
| 66 | `stbds_arrgrowf` + `stbds_arrfreef` | grow to 200 elements then free (round-trip; no leak/mismatch in header) | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, therefore the only
combination is the default one:

```
default (no features)
```

`tests/feature_matrix.sh` enumerates the feature list from `Cargo.toml` and runs
`cargo check` / `cargo build --release` / `cargo test --release` plus an `nm -D`
symbol diff for every combination; with zero declared features it verifies the
default configuration and `--no-default-features`.  Both pass with an empty
symbol diff.

## What the differential comparison actually compares

Pointers differ between the two libraries by construction, so every row is
compared through a canonical structural dump (`tests/common/mod.rs`):

* `stbds_array_header`: `length`, `capacity`, `temp`, and whether `hash_table`
  is set;
* every live element, byte for byte — with a `char *` key rendered as the
  *pointed-to string* so that `STRDUP`/`ARENA` copies compare by content;
* `stbds_hash_index`: `slot_count`, `used_count`, both thresholds,
  `tombstone_count`, `tombstone_count_threshold`, `seed`, `slot_count_log2`, and
  the embedded `stbds_string_arena`'s `remaining` / `block` / `mode`;
* the `STBDS_ALIGN_FWD` result for `storage` as three address-independent
  predicates (64-byte aligned, inside the over-allocated padding);
* every bucket's full `hash[8]` and `index[8]` array;
* for `str_put`, the bytes written to fd 1, captured in a forked child.

Two fields are deliberately excluded because the C leaves them
**uninitialised**, so they hold different heap garbage in the two processes:

* `stbds_hash_index::temp_key` after a rehash — `stbds_make_hash_index` never
  writes it and only copies `string` and `seed` from the old table.  Tests prime
  it to `0` before a `hmput_key` call and compare it only when that call did not
  rehash, which turns "did the library write `temp_key`?" into an observable
  fact (this is what makes rows 21/22 of `ERRORS.md` testable).
* element bytes outside the key region — `hmput_key` only writes the key, the
  `stbds_hmput` macro writes the value.  The tests perform that value store
  themselves (`fill_value`) so the whole element becomes defined.
