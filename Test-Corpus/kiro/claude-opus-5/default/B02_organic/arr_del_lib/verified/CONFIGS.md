# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the `if` / `switch` / loop branches in
`c_src/src/lib.c` and from the full set of `.so` exports in `SYMBOLS.md`
(NOT just the convenience wrappers — `stbds_arrgrowf`, `stbds_hm*_key*`,
`stbds_shmode_func`, `stbds_stralloc` are all driven directly).

## Axes the C actually branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `mode` (int, hash-map key comparison) | `< 1` → BINARY (`memcmp` + `stbds_hash_bytes`); `>= 1` → STRING (`strcmp` + `stbds_hash_string`); **and separately** `== 1` exactly, in two `stbds_hmdel_key` branches | `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_hmput_key`, `stbds_hmdel_key` |
| `table->string.mode` (unsigned char) | `STBDS_SH_NONE=0` / `DEFAULT=1` / `STRDUP=2` / `ARENA=3` / anything else → `default` | `switch` in `stbds_hmput_key`, `stbds_hmfree_func`, `stbds_hmdel_key` |
| table creation path | implicit (first `stbds_hmput_key`, `string.mode` derived from `mode`) vs explicit (`stbds_shmode_func`) vs never (`stbds_hmput_default` / `stbds_hmget_key` on NULL) | `stbds_hmput_key`, `stbds_shmode_func` |
| `slot_count` | 8 (initial, shrink disabled) / 16 / 32 / … (doubling on load, halving on shrink) | `stbds_make_hash_index`, `stbds_hmput_key`, `stbds_hmdel_key` |
| `elemsize` | 0, 1, 4, 8, 12, 16, 24, non-multiple-of-8 (e.g. 3, 12) | all `hm*` + `arrgrowf` |
| `keysize` | 0, 1, 4, 8, 16 (and `keysize != elemsize`) | `stbds_hash_bytes`, `memcmp`, `memcpy` |
| `keyoffset` | 0 (what all `hm*put/get` hard-code) vs caller-supplied non-zero in `stbds_hmdel_key` | `stbds_hmdel_key`, `stbds_is_key_equal` |
| element count | 0 / 1 / 6 (just below 8-slot threshold) / 7+ (forces first grow) / 50+ (multiple grows) | `stbds_hmput_key` |
| deletion pattern | none / one / last element / non-last (triggers move-last + slot re-find) / all / enough to shrink / enough to exceed the tombstone threshold | `stbds_hmdel_key` |
| `stbds_hash_bytes` `len` | 0..24 covering every `len % 8` residue plus multi-block lengths | `stbds_siphash_bytes` loop + tail `switch` |
| byte values | zero bytes, bytes with the high bit set (exercises the sign-extending `d[3] << 24`) | `stbds_siphash_bytes` |
| `seed` | 0, 1, `0x31415926` (default), `usize::MAX`, random | `stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed` |
| global seed state | default `0x31415926` vs `stbds_rand_seed(x)`; advanced by `seed = seed*a + b` on every fresh `stbds_make_hash_index` | `stbds_rand_seed`, `stbds_make_hash_index` |
| string content | `""` / 1 char / short / >8 / >64 / high-bit (non-ASCII) bytes / strings colliding in the first bucket scan | `stbds_hash_string`, `strcmp` |
| `arrgrowf` shape | `a==NULL` vs existing; `min_cap<=cap` (early out); `min_cap < 2*cap` (double); `min_cap >= 2*cap` and `< 4` (bump to 4); `addlen` 0 vs >0 | `stbds_arrgrowf` |
| arena `block` | 0, 1, 2, 3, 21, 22 (clamp point), 23, 40, 64, 100 | `stbds_stralloc` |
| arena `remaining` / `storage` | `remaining >= len` (fast path); `remaining < len` with `storage==NULL`; `remaining < len` with `storage!=NULL`; `len > blocksize` (oversize block) | `stbds_stralloc` |
| arena chain length | 0 / 1 / many blocks | `stbds_strreset` |

## Rows (one per combination the C treats differently)

Every row is exercised against BOTH `.so`s with many randomized inputs
(fixed seed `0xC0FFEE`) and compared byte-for-byte: return pointers'
*contents*, array header (`length`/`capacity`/`temp`), full hash-index struct,
and every bucket's `hash[]`/`index[]`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | `len = 0`, `p = NULL` and `p` valid; seeds {0, 1, default, MAX, random} | [x] |
| C2 | `stbds_hash_bytes` | `len = 1..7` (every tail residue), random bytes incl. high-bit set | [x] |
| C3 | `stbds_hash_bytes` | `len = 8, 16, 24` (whole blocks, no tail) | [x] |
| C4 | `stbds_hash_bytes` | `len = 9..31` (blocks + every tail residue) | [x] |
| C5 | `stbds_hash_bytes` | `len` large (64..4096), random buffers | [x] |
| C6 | `stbds_hash_bytes` | all-zero buffer, all-`0xFF` buffer, at each `len % 8` | [x] |
| C7 | `stbds_hash_string` | `""`, `"a"`, 2..40 random ASCII, seeds {0, 1, default, MAX} | [x] |
| C8 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (signed-char promotion) | [x] |
| C9 | `stbds_hash_string` | long strings (128..1024 chars) | [x] |
| C10 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to {0, 1, MAX, random}, then observe `table->seed` and the advanced global across 8 consecutive table creations | [x] |
| C11 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 0` (degenerate) | [x] |
| C12 | `stbds_arrgrowf` | `a = NULL`, `addlen` 0..8 × `min_cap` 0..8 × `elemsize` {0,1,4,8,12} (cross product) | [x] |
| C13 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → early-out path | [x] |
| C14 | `stbds_arrgrowf` | existing array, `min_cap` in `(cap, 2*cap)` → doubling path | [x] |
| C15 | `stbds_arrgrowf` | existing array, `min_cap >= 2*cap` → exact `min_cap` path | [x] |
| C16 | `stbds_arrgrowf` | repeated growth chain (20 successive `addlen=1` grows) tracking `capacity` sequence 4,8,16,… | [x] |
| C17 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (non-NULL) — no crash, header round-trip | [x] |
| C18 | `stbds_hmput_default` | `a = NULL`, `elemsize` {1,4,8,16,24} | [x] |
| C19 | `stbds_hmput_default` | called twice (2nd sees `length != 0`) | [x] |
| C20 | `stbds_hmput_default` | on an array whose `length` was reset to 0 | [x] |
| C21 | `stbds_hmput_key` BINARY | `mode = 0`, `elemsize = 8`, `keysize = 4`, 1 key | [x] |
| C22 | `stbds_hmput_key` BINARY | `mode = 0`, 6 keys (below 8-slot threshold, no grow) | [x] |
| C23 | `stbds_hmput_key` BINARY | `mode = 0`, 7 keys (crosses `used_count_threshold` → first grow to 16) | [x] |
| C24 | `stbds_hmput_key` BINARY | `mode = 0`, 200 random keys (several grows: 8→16→32→64→128→256) | [x] |
| C25 | `stbds_hmput_key` BINARY | duplicate keys interleaved with new keys (both the first-scan and wrapped-scan found paths) | [x] |
| C26 | `stbds_hmput_key` BINARY | `elemsize` {4,8,12,16,24} × `keysize` {1,4,8,16}, 40 random keys each | [x] |
| C27 | `stbds_hmput_key` BINARY | `keysize = 0` (all keys hash identically → maximal probing) | [x] |
| C28 | `stbds_hmput_key` STRING, implicit `SH_DEFAULT` | `mode = 1`, `elemsize = 16`, keys from `strkey(i)`, 1 / 6 / 7 / 60 keys | [x] |
| C29 | `stbds_hmput_key` STRING | `mode = 2` and `mode = 7` (out-of-range but `>= 1`) | [x] |
| C30 | `stbds_hmput_key` STRING | `mode = -1` and `INT_MIN` (out-of-range, `< 1` → BINARY) | [x] |
| C31 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_STRDUP` (2), `mode = 1`, 1 / 7 / 60 keys — keys are `strdup`ed, `temp_key` set | [x] |
| C32 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_ARENA` (3), `mode = 1`, 1 / 7 / 60 keys of varying length (drives `stbds_stralloc` from inside the map) | [x] |
| C33 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_NONE` (0) with `mode = 1` → `default:` `memcpy` branch despite string mode | [x] |
| C34 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_DEFAULT` (1) with `mode = 0` → pointer stored, `memcmp` compare | [x] |
| C35 | `stbds_shmode_func` | `mode` out of range: `-1` (→ 255), `256` (→ 0), `4`, `1000` — then a `hmput_key` to hit `switch default` | [x] |
| C36 | `stbds_hmget_key_ts` | miss on NULL / un-hashed / populated table; hit on first key, last key, middle key | [x] |
| C37 | `stbds_hmget_key_ts` | BINARY, 60-key table, look up all 60 keys + 60 absent keys, checking `*temp` each time | [x] |
| C38 | `stbds_hmget_key_ts` | STRING `SH_STRDUP` / `SH_ARENA` / `SH_DEFAULT`, all keys + absent keys | [x] |
| C39 | `stbds_hmget_key` | same as C37/C38 but through the non-`_ts` wrapper (checks `stbds_temp(raw_a)` is written) | [x] |
| C40 | `stbds_hmdel_key` BINARY | delete the only element | [x] |
| C41 | `stbds_hmdel_key` BINARY | delete the **last** element (`old_index == final_index`, no move) | [x] |
| C42 | `stbds_hmdel_key` BINARY | delete a **non-last** element (move-last + slot re-find) | [x] |
| C43 | `stbds_hmdel_key` BINARY | delete every element in insertion order, then in reverse order | [x] |
| C44 | `stbds_hmdel_key` BINARY | 200-key table, random delete/insert mix (drives shrink + tombstone rebuild) | [x] |
| C45 | `stbds_hmdel_key` BINARY | delete enough from a 64-slot table to cross `used_count_shrink_threshold` (shrink to 32) | [x] |
| C46 | `stbds_hmdel_key` BINARY | delete/reinsert loop on a 16-slot table to exceed `tombstone_count_threshold` (rebuild at same size) | [x] |
| C47 | `stbds_hmdel_key` STRING `SH_STRDUP` | `mode = 1` → frees the duped key + string re-find path | [x] |
| C48 | `stbds_hmdel_key` STRING `SH_ARENA` | `mode = 1` → no free, string re-find path | [x] |
| C49 | `stbds_hmdel_key` STRING `SH_DEFAULT` | `mode = 1` → no free, string re-find path | [x] |
| C50 | `stbds_hmdel_key` | `mode = 2` on a `SH_STRDUP` map: `strcmp` find but BINARY free/re-find branches | [x] |
| C51 | `stbds_hmdel_key` | `keyoffset` {0, 4, 8} on a BINARY map | [x] |
| C52 | `stbds_hmfree_func` | `SH_NONE` / `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA` maps, 0 / 1 / 60 elements | [x] |
| C53 | `stbds_hmfree_func` | array with `hash_table == NULL` (from `stbds_hmput_default` only) | [x] |
| C54 | `stbds_stralloc` | fresh zeroed arena, `len` 1..600 (crosses the 512 blocksize) | [x] |
| C55 | `stbds_stralloc` | many successive allocations from one arena (block growth 512→512→1024→…), verifying `remaining`/`block`/chain | [x] |
| C56 | `stbds_stralloc` | `remaining >= len` fast path (no new block) | [x] |
| C57 | `stbds_stralloc` | `len > blocksize` with `storage == NULL` (oversize, `remaining` → 0) | [x] |
| C58 | `stbds_stralloc` | `len > blocksize` with `storage != NULL` (oversize spliced after head, `remaining` kept) | [x] |
| C59 | `stbds_stralloc` | `block` preset to {0,1,2,3,21,22,23} (clamp boundary at 1<<20) | [x] |
| C60 | `stbds_stralloc` | `block` preset to {24,32,40,63,64,100,127,255} (out of the library's own range; `size_t` shift semantics) | [x] |
| C61 | `stbds_strreset` | arena with 0 / 1 / 8 blocks (chain walk) — arena zeroed afterwards | [x] |
| C62 | `strkey` | `n` = 0, 1, -1, 12345, `INT_MAX`, `INT_MIN`, plus 200 random ints | [x] |
| C63 | `arr_del` | `num` = 0, 1, -1, `INT_MAX`, `INT_MIN`, plus 200 random ints (void, must not crash) | [x] |
| C64 | full pipeline, BINARY | randomized 2000-op script (put/get/get_ts/del/free) over `elemsize`×`keysize` combos, comparing the whole map state after **every** op | [x] |
| C65 | full pipeline, STRING × all 4 `string.mode`s | randomized 2000-op script over `strkey`-style and random-length keys, comparing the whole map state after **every** op | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `cargo check`/`cargo test` with
`--no-default-features` is equivalent to the default build. Verified by
`scripts/feature_matrix.sh`.

## Verification

All 65 rows are exercised by `tests/phase_b_low.rs` (C1–C20, C54–C63; 28 tests)
and `tests/phase_b_map.rs` (C21–C53, C64–C65; 35 tests), each row driven with
many randomized inputs from a fixed-seed xorshift64* PRNG.

Comparison is not just return values: after **every** call the harness snapshots
and byte-compares

* the array header — `length`, `capacity`, `hash_table != NULL`, `temp`;
* every field of `stbds_hash_index` — `slot_count`, `used_count`, all three
  thresholds, `tombstone_count`, `seed`, `slot_count_log2`, and the embedded
  arena's `remaining` / `block` / `mode` / chain length;
* **every slot of every bucket** — `hash[j]` and `index[j]` for all
  `slot_count` slots;
* every element's key and value bytes (keys resolved through the `char *` for
  pointer key modes, so differing heap addresses do not matter);
* `stbds_temp_key`, on the calls where the C actually writes it.

Result: **63/63 tests pass** (28 + 35), against both the release and the debug
Rust `.so`.

### Things deliberately NOT compared

* Raw pointer values and whether `realloc` moved a block — allocator artifacts,
  not library semantics.
* `stbds_hash_index::temp_key` when the C did not write it on that call. The C
  never initialises it in `stbds_make_hash_index`, and its duplicate-key hit in
  the *wrapped* bucket scan omits the write, so it can hold a pointer to
  already-freed memory. That is indeterminate memory, not an observable.
* Element bytes past `keysize + valsize` — uninitialised padding on both sides.

### Bug found and fixed in this phase

`stbds_stralloc` computed `STBDS_STRING_ARENA_BLOCKSIZE_MIN << (block >> 1)` in
`u32`, but the C casts to `(size_t)` first and so shifts in 64 bits. `a->block`
is caller-visible state that reaches 255, making the shift count reach 127, so
the two differed for a whole range of `block` values. Fixed in `src/lib.rs` by
shifting a `usize` with `wrapping_shl` (matching x86-64 `shl`, which masks the
count by 63).
