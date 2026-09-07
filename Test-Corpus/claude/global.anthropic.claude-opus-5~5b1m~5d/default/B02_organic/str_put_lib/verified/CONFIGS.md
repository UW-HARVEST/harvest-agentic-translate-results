# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C in `c_src/src/lib.c` actually
takes. There is no build-time configuration (`#ifdef`) surface left: the file
hard-`#define`s `STBDS_HAS_TYPEOF`, `STBDS_SIPHASH_2_4`-style constants,
`STBDS_BUCKET_LENGTH 8`, `STBDS_REALLOC/FREE` to libc, and
`STBDS_ASSERT` to `assert`. All variability is therefore **runtime**.

## Axes the C branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` (hm/get/put/del) | `< 1` → memcmp/binary; `>= 1` → strcmp/string; `== 1` **exactly** in `hmdel_key`'s strdup-free and key-fixup | `:560`, `:707`, `:713`, `:836`, `:842` |
| `table->string.mode` | `STBDS_SH_NONE(0)` / `SH_DEFAULT(1)` / `SH_STRDUP(2)` / `SH_ARENA(3)` / out-of-range (`(unsigned char)mode`) | `:785`–`:790`, `:575`, `:836` |
| table lifecycle | no table (`hash_table==NULL`) / fresh 8-slot / grown (16, 32, 64, …) / shrunk / rebuilt-after-tombstones | `:698`, `:702`, `:854`, `:858` |
| `elemsize` / `keysize` | `keysize == elemsize` (pure key array), `keysize < elemsize` (key+value), `elemsize` 4/8/12/16/24/32, `keysize` 1/2/4/8/16 | all `elemsize*i` arithmetic |
| element count | 0 / 1 / 6 (below 8-slot threshold) / 7 (== `used_count_threshold` → grow) / many (multiple doublings) | `:698` |
| deletion pattern | none / delete-absent / delete-head / delete-tail (`old_index == final_index`) / delete-middle (triggers the relocate + re-find + `index` fixup) / delete-all (shrink chain) | `:839` |
| `arrgrowf(a, elemsize, addlen, min_cap)` | `a` NULL vs non-NULL × `addlen` 0/1/n × `min_cap` 0/1/3/4/exact-cap/2×cap/huge | `:283`–`:293` |
| `hash_bytes` length | 0, 1..7 (tail only), 8 (one word, no tail), 9..15, 16, 17, 64, 1000 | `:522`, `:532` |
| `hash_bytes`/`hash_string` byte values | all-zero, all-0xff, high-bit-set at offsets 3 and 7 (int sign-extension), random | `:523`, `:524`, `:536` |
| `hash_string` length | `""`, 1 char, 7, 8, long; bytes >= 0x80 (`(unsigned char)` cast) | `:480` |
| `seed` | 0, 1, `0x31415926` (initial), `SIZE_MAX`, random; plus seed evolution through `stbds_make_hash_index` | `:409`–`:412` |
| `stbds_rand_seed` | changes the seed used by the *next* fresh table, and the LCG chain thereafter | `:355` |
| arena `remaining` vs `len` | `len <= remaining` (bump) / `len > remaining && len <= blocksize` (new block) / `len > blocksize` (dedicated block) with `storage` NULL vs non-NULL | `:885`–`:911` |
| arena `block` counter | 0 (512) / 1 (512) / 2 (1024) / … / 24+ (saturated at 1<<20, no increment) / 110..127 (value overflow → 0) / >=128 (shift count >= 64) | `:888`–`:891` |
| `shmode_func(elemsize, mode)` | mode 0/1/2/3 and out-of-range 4/99/255/256/-1 | `:796` |
| `str_put(num)` | 0, 1, 2, 10, 100, 1000 (drives arena block growth), negative | `:945` |
| `strkey(n)` | 0, -1, `INT_MIN`, `INT_MAX` | `:939` |

## Configuration rows

Each row is exercised with **many randomized inputs, fixed seed** (`SPLITMIX64`,
seed `0x243F6A8885A308D3`), driving C and Rust in lockstep and comparing every
observable byte (return pointers' *contents*, header `length`/`capacity`/`temp`,
hash-index fields, arena block chains, hash values, stdout).

| #  | entry point(s) | configuration (options set + input shape) | ✔ |
|----|----------------|-------------------------------------------|---|
| 1  | `stbds_hash_bytes` | `len = 0`, random seeds | [x] |
| 2  | `stbds_hash_bytes` | `len = 1..7` (tail-only), random bytes, random seeds | [x] |
| 3  | `stbds_hash_bytes` | `len = 8` exactly (single word, empty tail) | [x] |
| 4  | `stbds_hash_bytes` | `len = 9..15` (one word + tail) | [x] |
| 5  | `stbds_hash_bytes` | `len = 16, 17, 24, 64, 1000` (multi-word) | [x] |
| 6  | `stbds_hash_bytes` | bytes forced all-`0x00`, all-`0xFF`, and `d[3]`/`d[7] >= 0x80` (int sign-extension paths) | [x] |
| 7  | `stbds_hash_bytes` | seeds `0`, `1`, `SIZE_MAX`, `0x31415926`, random | [x] |
| 8  | `stbds_hash_string` | `""` / 1 / 7 / 8 / 9..64-char ASCII, random seeds | [x] |
| 9  | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (unsigned-char cast) | [x] |
| 10 | `stbds_rand_seed` + `stbds_hmput_key` | seed set to 0 / 1 / SIZE_MAX / random, then a fresh table is built; verifies the seed LCG chain across several fresh tables | [x] |
| 11 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 0/1/3/4/5/100`, `elemsize = 1/4/8/16/32` | [x] |
| 12 | `stbds_arrgrowf` | `a = NULL`, `addlen = 1..64`, `min_cap = 0` | [x] |
| 13 | `stbds_arrgrowf` | non-NULL `a`, request that fits (`<= cap`) ⇒ no-op | [x] |
| 14 | `stbds_arrgrowf` | non-NULL `a`, request `cap+1` ⇒ doubling floor `2*cap` | [x] |
| 15 | `stbds_arrgrowf` | non-NULL `a`, `min_cap` far above `2*cap` ⇒ exact `min_cap` | [x] |
| 16 | `stbds_arrgrowf` | repeated random grow chain (100 steps), header fields tracked each step | [x] |
| 17 | `stbds_arrfreef` | non-NULL array from `arrgrowf` | [x] |
| 18 | `stbds_hmput_default` | `a = NULL`, `elemsize` 4/8/16/32 | [x] |
| 19 | `stbds_hmput_default` | called twice (second call must be a no-op) | [x] |
| 20 | `stbds_hmput_default` | on a map already populated by `hmput_key` | [x] |
| 21 | `stbds_hmput_key` | `mode = 0` (binary), `keysize = elemsize` (set-like), 1 insert | [x] |
| 22 | `stbds_hmput_key` | `mode = 0`, `keysize = 4`, `elemsize = 8` (int→int map), 1/2/6/7/8 inserts (crosses the 8-slot grow threshold) | [x] |
| 23 | `stbds_hmput_key` | `mode = 0`, `keysize = 4`, `elemsize = 8`, N = 1..300 random distinct keys (multiple doublings + rehash) | [x] |
| 24 | `stbds_hmput_key` | `mode = 0`, `keysize = 8`, `elemsize = 16` (64-bit key) random N | [x] |
| 25 | `stbds_hmput_key` | `mode = 0`, `keysize = 16`, `elemsize = 32` (`int key[2]`-style struct key) random N | [x] |
| 26 | `stbds_hmput_key` | `mode = 0`, duplicate keys interleaved (update path, first scan loop) | [x] |
| 27 | `stbds_hmput_key` | `mode = 0`, `keysize = 1` and `2` (sub-word keys, heavy hash collisions) | [x] |
| 28 | `stbds_hmput_key` | `mode = 1` (string) on a NULL map ⇒ `string.mode = SH_DEFAULT`, N random strings | [x] |
| 29 | `stbds_hmput_key` | `mode = 1`, duplicate string keys (update path; `temp_key` must be the *stored* pointer) | [x] |
| 30 | `stbds_hmput_key` | `mode = 2` and `99` (out-of-range enum ⇒ still string path) | [x] |
| 31 | `stbds_hmput_key` | `mode = -1` / `INT_MIN` (negative ⇒ binary path) | [x] |
| 32 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_STRDUP(2)` map, N random strings, keys must be heap copies distinct from input | [x] |
| 33 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_ARENA(3)` map, N random strings incl. > 512 bytes (drives the arena inside the table) | [x] |
| 34 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_DEFAULT(1)` map (keys stored by pointer) | [x] |
| 35 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_NONE(0)` map with `mode=1` puts ⇒ `default:` `memcpy(keysize)` branch | [x] |
| 36 | `stbds_shmode_func` | out-of-range `mode` = 4 / 99 / 255 / 256 / -1, then a put (truncation ⇒ which switch arm) | [x] |
| 37 | `stbds_hmget_key` | binary map, all present keys + absent keys | [x] |
| 38 | `stbds_hmget_key` | string map (`SH_DEFAULT`, `SH_STRDUP`, `SH_ARENA`), present + absent | [x] |
| 39 | `stbds_hmget_key` | on the `a == NULL` path (creates the map) | [x] |
| 40 | `stbds_hmget_key` | on a map with `hash_table == NULL` (from `hmput_default` only) | [x] |
| 41 | `stbds_hmget_key_ts` | same as rows 37–40 but through the `temp` out-parameter (header `temp` must stay untouched) | [x] |
| 42 | `stbds_hmget_key_ts` | interleaved with `hmget_key` on the same map (checks header `temp` vs out-param independence) | [x] |
| 43 | `stbds_hmdel_key` | binary map, delete the tail element (`old_index == final_index`, no relocate) | [x] |
| 44 | `stbds_hmdel_key` | binary map, delete a middle element (relocate + re-find + `index` fixup) | [x] |
| 45 | `stbds_hmdel_key` | binary map, delete every key in random order (drives shrink at `used_count < slot_count/4`) | [x] |
| 46 | `stbds_hmdel_key` | binary map, delete/insert churn to exceed `tombstone_count_threshold` ⇒ same-size rebuild | [x] |
| 47 | `stbds_hmdel_key` | binary map, delete an absent key | [x] |
| 48 | `stbds_hmdel_key` | `SH_DEFAULT` string map (`mode = 1`), delete head/middle/tail/all | [x] |
| 49 | `stbds_hmdel_key` | `SH_STRDUP` string map, `mode = 1` ⇒ stored key freed | [x] |
| 50 | `stbds_hmdel_key` | `SH_STRDUP` string map, `mode = 2` ⇒ key **not** freed, non-string fixup branch | [x] |
| 51 | `stbds_hmdel_key` | `SH_ARENA` string map, delete-all | [x] |
| 52 | `stbds_hmdel_key` | full random churn: 500 mixed put/get/del ops, binary keys, invariants compared each step | [x] |
| 53 | `stbds_hmdel_key` | full random churn: 500 mixed put/get/del ops, `SH_STRDUP` string keys | [x] |
| 54 | `stbds_hmfree_func` | `SH_NONE` / `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA` maps, empty and populated | [x] |
| 55 | `stbds_stralloc` | fresh arena (`{0}`), single short string (block 0 ⇒ 512-byte block) | [x] |
| 56 | `stbds_stralloc` | fresh arena, N random short strings — bump path until block exhausted, then next block (block counter 0→1→2→…) | [x] |
| 57 | `stbds_stralloc` | string longer than the current `blocksize` (e.g. 600, 5000 bytes) with `storage == NULL` ⇒ dedicated block, `remaining = 0` | [x] |
| 58 | `stbds_stralloc` | same but with `storage != NULL` ⇒ dedicated block spliced *after* the head | [x] |
| 59 | `stbds_stralloc` | pre-set `a->block` = 0,1,2,10,23,24,25 (crosses the `1<<20` saturation) | [x] |
| 60 | `stbds_stralloc` | pre-set `a->block` = 110..=127 (`512 << shift` overflows to 0 ⇒ dedicated-block path) and 128..=160 (shift count >= 64, C UB; gcc's `shl` masks `& 63`) — both must agree bit for bit. `block >= 46` is NOT tested: the C itself asks `realloc` for >= 4 GiB, gets NULL and segfaults | [x] |
| 61 | `stbds_strreset` | all-zero arena (idempotent), 1-block, many-block, mixed dedicated+normal blocks | [x] |
| 62 | `strkey` | `n` = 0, 1, -1, 42, `INT_MAX`, `INT_MIN`, random | [x] |
| 63 | `str_put` | `num` = 0 (stdout compared byte-for-byte) | [x] |
| 64 | `str_put` | `num` = 1, 2, 3, 10, 63, 64, 100, 1000, 10000 (arena block growth), and negative `-1`, `INT_MIN` | [x] |
| 65 | `str_put` | called repeatedly in one process (global `stbds_hash_seed` evolution must match) | [x] |
| 66 | full pipeline | `shmode_func(SH_ARENA)` → many `hmput_key` → `hmget_key` all → `hmdel_key` all → `hmfree_func`, elemsize 16, randomized 200 keys | [x] |
| 67 | full pipeline | `arrgrowf`-backed manual array + `hmput_default` + `hmput_key` + `hmdel_key` + `hmfree_func`, binary mode, randomized | [x] |

## Verification status

All 67 rows pass. They are implemented across:

| file | rows | tests |
|------|------|-------|
| `tests/phase_b_hash.rs`       | 1-10  | 10 |
| `tests/phase_b_arr.rs`        | 11-17 | 7 |
| `tests/phase_b_map_binary.rs` | 18-27, 37, 39-47, 52, 67 | 23 |
| `tests/phase_b_map_string.rs` | 28-36, 38, 48-51, 53-54, 66 | 18 |
| `tests/phase_b_arena.rs`      | 55-62 | 8 |
| `tests/phase_b_strput.rs`     | 63-65 | 1 (merged, see below) |

Every row is driven with **many randomized inputs from a fixed-seed splitmix64
PRNG** (`Rng::with(<row number>)`), and after *every* operation the full state of
both libraries is snapshotted and compared: header `length`/`capacity`/`temp`,
all `length` element payloads, the key text behind every stored key pointer, and
every field of the hash index including all `slot_count` bucket
`(hash, index)` pairs and the embedded arena's `remaining`/`block`/`mode`/chain
length. Divergences are reported as a first-difference field diff
(`common::compact_diff`).

### Notes on what deliberately is NOT byte-compared

* **Uninitialised element bytes.** `stbds_hmput_key` writes only `keysize` key
  bytes; the value bytes are raw `realloc` memory. The harness therefore fills
  `keysize..elemsize` of every touched element with a deterministic pattern
  (`MapPair::fill_tail`) before comparing. Two `arrgrowf` calls in two different
  libraries legitimately return different garbage there.
* **`table->temp_key` in general.** `stbds_make_hash_index` never initialises
  `temp_key`, and the wrap-around scan loop of `stbds_hmput_key` deliberately
  does not refresh it, so its value is raw heap garbage in a wide range of
  states. It IS compared where the C defines it: after every insert of a
  brand-new key (`temp_key_after_every_insert`).
* **Absolute pointer values.** Compared structurally instead: whether
  `stbds_arrgrowf` returned *the same* pointer (no-op path), whether
  `stbds_stralloc` returned a block start and which block of the chain it is
  (`Loc` in `tests/phase_b_arena.rs`), and whether `SH_STRDUP`/`SH_ARENA` stored
  a copy rather than the caller's pointer.
* **`stbds_shmode_func(SH_NONE)` / truncated-mode maps with more than a handful
  of string keys.** That configuration makes `hmput_key` take the `default:`
  `memcpy` arm, which stores the first `keysize` bytes of the *string data*, and
  a later `strcmp` would read those bytes back as a pointer — UB in the C too.
  Rows 35 / 36 therefore keep those maps at 3-4 entries so no hash collision can
  reach the `strcmp`.

### Binary / driver stdout

The project builds **no executable**, only a shared library, so there is no pair
of binaries to diff. The nearest equivalent is the public driver `str_put`,
which writes to stdout; `tests/phase_b_strput.rs` redirects the process-wide
fd 1 with `dup2`, runs the C's `str_put` and the Rust's `str_put` on the same
input, and compares the captured bytes **byte for byte** for
`num` ∈ {0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100,
127, 128, 129, 255, 256, 511, 512, 1000, 4095, 4096, 10000}, 160 random values,
the negatives {-1, -2, -1000, INT_MIN}, and five 200-call sequences from
different `stbds_rand_seed` starting points (so the evolution of the global seed
is compared too). It also covers `printf("%s %d\n", strmap[z], strmap[z].value)`
— the C passes the 16-byte struct **by value** into the varargs, so `%s` and
`%d` consume its two eightbytes; the Rust reproduces that by passing
`(e.key, e.value)`. Rows 63-65 live in one `#[test]` because the redirect is
process-wide and libtest's own progress output would otherwise land in the
capture.

## Feature combinations

`Cargo.toml` has **no `[features]`** section, so there is exactly one feature
combination. `run_verification.sh` enumerates the powerset of the declared
features anyway (falling back to `default` + `--no-default-features`) and runs
the whole suite plus the `nm -D` symbol diff for each; both configurations pass.
