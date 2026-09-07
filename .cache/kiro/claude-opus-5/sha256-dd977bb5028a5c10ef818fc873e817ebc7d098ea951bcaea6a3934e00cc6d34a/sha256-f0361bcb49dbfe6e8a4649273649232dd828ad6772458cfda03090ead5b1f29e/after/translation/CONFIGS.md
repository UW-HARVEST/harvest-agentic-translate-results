# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branch points in `c_src/src/lib.c`.

## Axes the C code actually branches on

**A1 — `mode` argument** (`stbds_hmput_key` / `hmget_key` / `hmget_key_ts` /
`hmdel_key`). Every branch is `mode >= STBDS_HM_STRING` (i.e. `>= 1`) except
`hmdel_key`'s strdup-free which uses `mode == STBDS_HM_STRING` exactly:
`0` (`HM_BINARY`), `1` (`HM_STRING`), `2` (`HM_PTR_TO_STRING`, referenced by
the `pshput`/`pshget` macros), and out-of-range ints.

**A2 — `table->string.mode`** (set by `shmode_func` / implicitly by
`hmput_key`), switched on at lib.c:786:
`SH_NONE=0`, `SH_DEFAULT=1`, `SH_STRDUP=2`, `SH_ARENA=3`, plus `default:`
(anything else) → binary `memcpy`.

**A3 — table creation path**: `hmput_key` bootstrapping its own table
(`string.mode` = `SH_DEFAULT` if `mode>=1` else `0`) vs. `shmode_func`
pre-creating the table (`string.mode` = the requested value).

**A4 — `elemsize`**: the struct layout the caller declares. Distinct shapes:
8 (`{i32 key; i32 value}`), 16 (`{char* key; i32 value}` / `{i64;i64}`),
24 (`{i32 key[2]; i32 b,c,d}` padded), and large (128).

**A5 — `keysize`**: `0`, `< elemsize`, `== elemsize`, `sizeof(char*)`.
Drives `memcmp`/`memcpy` length and `hash_bytes` length.

**A6 — element count / table geometry**: 0, 1, 7 (below the first grow),
8, 9 (crosses `used_count_threshold` = 6 → grow to 16), and hundreds
(repeated doubling: 8→16→32→64→…​), plus the shrink threshold
(`used_count < slot_count>>2`) and tombstone rebuild threshold
(`tombstone_count > (slot_count>>3)+(slot_count>>4)`).

**A7 — key value shapes for `hash_bytes`**: lengths 0..64 (the `i+8<=len`
loop plus each of the 8 `switch` fall-through cases), and byte values with /
without bit 7 set in position 3 mod 8 (the sign-extension quirk).

**A8 — global seed** (`stbds_rand_seed`): the seed is a mutable global that
`make_hash_index` **advances** (`seed = seed*a + b`) on every fresh table, so
table layout depends on how many tables were made before. Configurations:
default seed `0x31415926`, `0`, `SIZE_MAX`, and a randomised seed.

**A9 — `stralloc` arena state**: fresh (`{0}`), mid-block with
`remaining > len`, `remaining < len` (new geometric block), oversized string
(`len > blocksize`) with `storage == NULL` vs `storage != NULL`, and
`block` saturated at the 1 MiB cap.

**A10 — `arrgrowf` growth mode**: no-op, first alloc (`min_cap`→4), doubling
(`min_cap < 2*cap`), explicit large `min_cap`, `addlen`-driven.

**A11 — entry point level**: the low-level exports directly
(`arrgrowf`, `hash_bytes`, `hash_string`, `hm_*_key*`, `shmode_func`,
`stralloc`, `strreset`) vs. the one-shot driver `str_dups` and the
`strkey` helper.

**A12 — `#ifdef` / feature axes**: none. `c_src/src/lib.c` `#define`s all of
its configuration unconditionally (`STBDS_SIPHASH_2_4`, `STBDS_HAS_TYPEOF`,
`STBDS_BUCKET_LENGTH 8`, `STBDS_REALLOC`/`FREE` = libc) and
`translation/Cargo.toml` declares **no `[features]`**, so the only build
configuration is the default one. (Verified: `grep -c '^\[features\]'
Cargo.toml` → 0.)

## Configuration rows

Every row is exercised with **many randomised inputs** (`SplitMix64`, fixed
seed `0x9E3779B97F4A7C15`) through both `.so`s, comparing return values and the
full resulting memory image (header + hash index + elements) byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap=0` → no-op path (A10) | [x] |
| 2 | `stbds_arrgrowf` | `a=NULL`, random `addlen` 0..3, `min_cap=0` → clamp to 4 (A10) | [x] |
| 3 | `stbds_arrgrowf` | `a=NULL`, random `min_cap` 1..4096, `elemsize` ∈ {1,4,8,16,24,128} (A4,A10) | [x] |
| 4 | `stbds_arrgrowf` | grow existing array repeatedly, `addlen=1` → doubling chain 4,8,16,… (A6,A10) | [x] |
| 5 | `stbds_arrgrowf` | grow existing array with `min_cap` > `2*cap` (explicit-capacity path) | [x] |
| 6 | `stbds_arrgrowf` + `stbds_arrfreef` | full alloc/free round trip, random `elemsize`, verify header fields | [x] |
| 7 | `stbds_hash_bytes` | `len = 0` (A7) | [x] |
| 8 | `stbds_hash_bytes` | `len` 1..7, random bytes — each `switch` fall-through case (A7) | [x] |
| 9 | `stbds_hash_bytes` | `len` 8..64, random bytes — the main 8-byte loop + every tail (A7) | [x] |
| 10 | `stbds_hash_bytes` | random bytes forced to have `d[3] & 0x80` set in every 8-byte block (sign-extension quirk, A7) | [x] |
| 11 | `stbds_hash_bytes` | random `seed` ∈ {0, 1, 0x31415926, SIZE_MAX, random} × random len (A8) — seed is XOR-cancelled, must match anyway | [x] |
| 12 | `stbds_hash_string` | `""`, 1-char, random ASCII 1..64, random bytes 0x01..0xff (high-bit chars → `(unsigned char)` promotion) (A7) | [x] |
| 13 | `stbds_hash_string` | random `seed` ∈ {0, 1, 0x31415926, SIZE_MAX, random} (A8) | [x] |
| 14 | `stbds_rand_seed` | set seed, then create a table, observe the advanced seed through subsequent table layout (A8) | [x] |
| 15 | `stbds_hmput_key` (binary) | `mode=0`, self-bootstrapped table (A3), `elemsize=8`, `keysize=4`, 1 insert | [x] |
| 16 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8`, `keysize=4`, 7 inserts (no grow) (A6) | [x] |
| 17 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8`, `keysize=4`, 9 inserts (crosses grow to 16) (A6) | [x] |
| 18 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=8`, `keysize=4`, 500 random inserts (repeated doubling) (A6) | [x] |
| 19 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=16`, `keysize=8`, 200 random `i64` keys (A4,A5) | [x] |
| 20 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=24`, `keysize=8` (`stbds_struct2`: `int key[2]`) (A4,A5) | [x] |
| 21 | `stbds_hmput_key` (binary) | `mode=0`, `elemsize=128`, `keysize=128` (`keysize == elemsize`) (A4,A5) | [x] |
| 22 | `stbds_hmput_key` (binary) | `mode=0`, `keysize=0` — all keys compare equal (A5) | [x] |
| 23 | `stbds_hmput_key` (binary) | duplicate keys re-put (update path, `temp` = existing index) (A6) | [x] |
| 24 | `stbds_hmput_key` (string, self-bootstrapped) | `mode=1`, table auto-created → `string.mode = SH_DEFAULT` (A2,A3) | [x] |
| 25 | `stbds_shmode_func` + `hmput_key` | `SH_STRDUP` table, `mode=1`, random strings (A2,A3) | [x] |
| 26 | `stbds_shmode_func` + `hmput_key` | `SH_ARENA` table, `mode=1`, random strings incl. > 512 B (A2,A3,A9) | [x] |
| 27 | `stbds_shmode_func` + `hmput_key` | `SH_DEFAULT` table, `mode=1`, caller-owned key pointers (A2,A3) | [x] |
| 28 | `stbds_shmode_func` + `hmput_key` | `SH_NONE` (0) table with `mode=1` → `default:` `memcpy` branch (A2) | [x] |
| 29 | `stbds_shmode_func` + `hmput_key` | `mode=2` (`HM_PTR_TO_STRING`) with `SH_DEFAULT` — `>=STRING` true, `==STRING` false (A1) | [x] |
| 30 | `stbds_hmget_key` | binary mode, hit and miss, `elemsize`/`keysize` cross product from rows 15–22 (A1,A4,A5) | [x] |
| 31 | `stbds_hmget_key_ts` | binary mode, out-param `temp`, hit and miss (low-level entry point) (A1) | [x] |
| 32 | `stbds_hmget_key` / `_ts` | string mode over `SH_STRDUP`/`SH_ARENA`/`SH_DEFAULT` tables (A1,A2) | [x] |
| 33 | `stbds_hmput_default` | `a=NULL`; then `a` non-NULL with `length==0`; then non-NULL with `length>0` (no-op) | [x] |
| 34 | `stbds_hmput_default` + `hmput_key` | default element present, then random inserts (interaction) | [x] |
| 35 | `stbds_hmdel_key` | binary, delete the last element (`old_index == final_index`, no memmove) (A6) | [x] |
| 36 | `stbds_hmdel_key` | binary, delete a middle element (`old_index != final_index` → memmove + re-slot) (A6) | [x] |
| 37 | `stbds_hmdel_key` | binary, delete enough to cross `used_count_shrink_threshold` → shrink rehash (A6) | [x] |
| 38 | `stbds_hmdel_key` | binary, delete/insert churn to cross `tombstone_count_threshold` → same-size rebuild (A6) | [x] |
| 39 | `stbds_hmdel_key` | string mode over `SH_STRDUP` (frees the duped key) (A1,A2) | [x] |
| 40 | `stbds_hmdel_key` | string mode over `SH_ARENA` / `SH_DEFAULT` (no free) (A1,A2) | [x] |
| 40b | `stbds_hmdel_key` | `mode = 2` (`HM_PTR_TO_STRING`) — the re-slot lookup passes the *address* of the key pointer while `find_slot` still hashes it as a string, so `STBDS_ASSERT(slot >= 0)` fires; compared in forked children (A1) | [x] |
| 41 | `stbds_hmdel_key` | random insert/delete/get churn, 2000 ops, binary mode, `elemsize=16` (full pipeline) | [x] |
| 42 | `stbds_hmdel_key` | random insert/delete/get churn, 2000 ops, string mode over `SH_STRDUP` | [x] |
| 43 | `stbds_hmfree_func` | table with `SH_STRDUP` (frees every duped key) vs `SH_ARENA` (strreset) vs no table (A2) | [x] |
| 44 | `stbds_stralloc` | fresh arena `{0}`, one short string (first 512 B block) (A9) | [x] |
| 45 | `stbds_stralloc` | many short strings from one block until `remaining` is exhausted → next block, `block` increments (A9) | [x] |
| 46 | `stbds_stralloc` | oversized string (`len > blocksize`) with `storage == NULL` → block + `remaining = 0` (A9) | [x] |
| 47 | `stbds_stralloc` | oversized string with `storage != NULL` → spliced after head, `remaining` untouched (A9) | [x] |
| 48 | `stbds_stralloc` | `a->block` driven up to the 1 MiB saturation point (A9) | [x] |
| 49 | `stbds_stralloc` | random mixed short/long strings, 400 ops, verifying returned contents + arena state (A9) | [x] |
| 50 | `stbds_strreset` | fresh arena, single-block arena, multi-block arena; then reuse after reset (A9) | [x] |
| 51 | `strkey` | `n` ∈ {0, 1, -1, 9, 10, 99, 100, INT_MAX, INT_MIN, random} (A11) | [x] |
| 52 | `str_dups` | `num` ∈ {0, 1, 2, 3, 7, 8, 9, 100, 1000, -1, INT_MIN} — stdout compared byte-for-byte (A11) | [x] |
| 53 | `str_dups` | called repeatedly in one process (the advancing global seed makes each call's table differ) (A8,A11) | [x] |
| 54 | end-to-end pipeline | `rand_seed` → `shmode_func(SH_STRDUP)` → N random `shput` → random `shget` → random `shdel` → `hmfree_func`, comparing every intermediate `temp`/`length`/element (A1,A2,A6,A8) | [x] |
| 55 | end-to-end pipeline | same for `SH_ARENA` | [x] |
| 56 | end-to-end pipeline | same for binary `hmput`/`hmget`/`hmdel` with `elemsize=24`, `keysize=8` | [x] |

## Build configurations

`translation/Cargo.toml` has no `[features]` table, so there is exactly one
feature combination (the default / empty one). `cargo check
--no-default-features` is also verified to succeed and produce the same symbol
set. There is no `[[bin]]` target and `c_src/CMakeLists.txt` builds only
`add_library(... SHARED)`, so there is **no binary executable to diff**.

## Where each row is tested

| rows | file |
|------|------|
| 1–14 | `tests/phase_b_low_level.rs` (`row_01`…`row_14`) |
| 15–34 | `tests/phase_b_hashmap.rs` (`row_15`…`row_34`) |
| 35–56 | `tests/phase_b_delete_arena.rs` (`row_35`…`row_56`; 54 and 55 share `row_54_55_string_pipeline`) |

Randomisation: every property-style row uses `common::Rng` (SplitMix64) seeded
from the fixed constant `0x9E3779B97F4A7C15` XORed with the row number, so runs
are reproducible. The global hash seed inside each `.so` is reset with
`stbds_rand_seed` before every scenario and is itself an axis (rows 11, 13, 14
and the `seed` loops throughout), because `stbds_make_hash_index` advances it.

Comparison granularity: after **every** operation both maps are snapshotted and
compared field by field — header `length`/`capacity`/`temp`, the whole
`stbds_hash_index` (slot count, all four thresholds, used/tombstone counts,
stored seed, `slot_count_log2`, arena mode/block/remaining), every bucket's
`hash[8]` and `index[8]`, and every element's defined key and value bytes.
Padding the library never writes is excluded, since it holds indeterminate
`realloc` leftovers in both libraries.

## Verification status

All 56 rows pass, under both the `dev` and the `release` Rust profile, against
the `dlopen`ed C `.so`. Run with `./phase_d.sh`.
