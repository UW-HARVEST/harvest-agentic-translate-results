# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `mode` (hash-map key mode) | `< 1` → BINARY (`hash_bytes`+`memcmp`); `>= 1` → STRING (`hash_string`+`strcmp`); `== 1` **exactly** in `hmdel_key` for the strdup-free and key-refetch paths | L556, L595, L714, L837, L842 |
| `table->string.mode` | `SH_NONE(0)`/other → `memcpy`; `SH_DEFAULT(1)` → store caller pointer; `SH_STRDUP(2)` → `stbds_strdup`; `SH_ARENA(3)` → `stbds_stralloc` | L785-790 |
| how the map is created | `hmput_key(NULL,…)` (implicit, `string.mode` derived from `mode`) vs `shmode_func(elemsize,mode)` (explicit `string.mode`) vs `hmput_default` vs `hmget_key*(NULL,…)` | L686-696, L802-810, L669, L634 |
| `elemsize` | any; drives header/element strides. Distinct shapes: 4 (`int`), 8 (`char*`), 16 (`stbds_struct`), 20/24 (`stbds_struct2`), 0 (degenerate) | throughout |
| `keysize` | 0, 1, 2, 4, 8, 16 — selects siphash body-loop vs tail-only, and `memcmp` width | L520-546 |
| `keyoffset` | 0 (all public wrappers) and non-zero (only reachable through `hmdel_key`'s explicit parameter) | L811, L842-844 |
| entry count | 0, 1, 5 (below threshold), 6 (`used_count_threshold` for 8 slots), 7, 8, 9, 17, 100, 1000 → drives `make_hash_index` growth chain 8→16→32→… | L698-707 |
| delete pattern | last element vs middle element; delete-all; delete→reinsert (tombstone reuse); enough deletes to cross `used_count_shrink_threshold`; enough to cross `tombstone_count_threshold` | L823-861 |
| hash value | generic; plus keys whose raw hash is `0`/`1` (the `hash += 2` fixup) | L596, L719 |
| seed | default `0x31415926`; overridden by `stbds_rand_seed` (0, 1, `SIZE_MAX`, random); per-table seed evolution `seed*a+b` | L385-389, L410-416 |
| `arrgrowf` shape | `a==NULL` vs existing; `addlen` 0/1/N; `min_cap` 0/1/`2*cap`-straddling/`<4`; the no-op early return | L262-295 |
| string-arena shape | `len <= remaining`; `remaining < len <= blocksize`; `len > blocksize` with `storage==NULL`; `len > blocksize` with `storage!=NULL`; `block` growth 512→1KiB→…→1MiB saturation | L890-921 |
| byte-order / high-bit data | key bytes `< 0x80` vs `>= 0x80` at tail positions 1..7 and body positions 0..7 (int-shift sign-extension quirks) | L520-546 |
| `arr_ins` | `num` over the whole `int` range incl. `0`, `4`, `INT_MIN`, `INT_MAX` | L942-957 |

## Configuration rows

Every row is exercised through the `.so` exports of BOTH libraries with many
randomized inputs (fixed seed `0xC0FFEE` unless noted) and compared
byte-for-byte (return values, array bytes, `stbds_array_header`
{`length`,`capacity`,`temp`}, and `stbds_hash_index`
{`slot_count`,`slot_count_log2`,`used_count`,`tombstone_count`,`*_threshold`,
`seed`,`string.{remaining,block,mode}`} plus every bucket's `hash[]`/`index[]`).

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `stbds_hash_bytes` | `len = 0` | [x] |
| 2  | `stbds_hash_bytes` | `len = 1..7` (tail-only), random bytes incl. `>= 0x80` | [x] |
| 3  | `stbds_hash_bytes` | `len = 8` (exactly one body block, empty tail) | [x] |
| 4  | `stbds_hash_bytes` | `len = 9..64` (body + every tail remainder 0..7) | [x] |
| 5  | `stbds_hash_bytes` | `len = 1..64` with all bytes `>= 0x80` (max sign-extension) | [x] |
| 6  | `stbds_hash_bytes` | random `seed` incl. `0`, `1`, `SIZE_MAX` | [x] |
| 7  | `stbds_hash_string` | `""`, 1-char, random ASCII, random high-bit (`0x80..0xFF`) bodies, len up to 64 | [x] |
| 8  | `stbds_hash_string` | random `seed` incl. `0`, `SIZE_MAX` | [x] |
| 9  | `stbds_rand_seed` + `stbds_hmput_key` | seed override observable via the per-table `seed` and its `seed*a+b` evolution over 8 successive table creations | [x] |
| 10 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap` ∈ {0,1,2,3} → forced to 4 | [x] |
| 11 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap` ∈ {4,5,…,1000} | [x] |
| 12 | `stbds_arrgrowf` | `a=NULL`, `addlen` ∈ {1,…,100}, `min_cap=0` | [x] |
| 13 | `stbds_arrgrowf` | existing array, `min_cap <= cap` and `addlen` small → early-return NOP | [x] |
| 14 | `stbds_arrgrowf` | existing array, `min_cap` straddling `2*cap` (doubling branch vs explicit) | [x] |
| 15 | `stbds_arrgrowf` | `elemsize` ∈ {1,4,8,16,20,24}, randomized `addlen`/`min_cap` grow chains of length 20 | [x] |
| 16 | `stbds_arrgrowf` + `stbds_arrfreef` | grow chain then free (no leak / no crash), `elemsize` ∈ {4,16} | [x] |
| 17 | `stbds_hmput_default` | `a=NULL`, `elemsize` ∈ {8,16,20,24} | [x] |
| 18 | `stbds_hmput_default` | on an existing non-empty map → NOP | [x] |
| 19 | `stbds_hmput_default` | on an `arrgrowf`-made array whose raw `length == 0` → re-grow path | [x] |
| 20 | `stbds_hmget_key_ts` | `a=NULL`, BINARY, `keysize=4` → `*temp=-1` + fresh 1-elem array | [x] |
| 21 | `stbds_hmget_key_ts` | map with `hash_table==NULL` (built by `hmput_default`) → `*temp=-1`, NOP | [x] |
| 22 | `stbds_hmget_key` | same as 20/21 but also asserts `header->temp` | [x] |
| 23 | `hmput_key`/`hmget_key` | BINARY, `elemsize=16`, `keysize=4`, N ∈ {1,2,5,6,7,8,9,17,100} random `i32` keys, lookup all present + absent | [x] |
| 24 | `hmput_key`/`hmget_key` | BINARY, `elemsize=20`, `keysize=8` (`int key[2]`), N ∈ {1,…,100} | [x] |
| 25 | `hmput_key`/`hmget_key` | BINARY, `keysize=1` (byte keys, forced collisions) | [x] |
| 26 | `hmput_key`/`hmget_key` | BINARY, `keysize=2` | [x] |
| 27 | `hmput_key`/`hmget_key` | BINARY, `keysize=16` (two-block siphash body) | [x] |
| 28 | `hmput_key`/`hmget_key` | BINARY, `keysize=0` (all keys alias) | [x] |
| 29 | `hmput_key` | BINARY, duplicate keys re-put (overwrite path, `temp` = existing index) | [x] |
| 30 | `hmput_key` | BINARY, 1000 keys → full growth chain 8→16→…→2048, compare every bucket | [x] |
| 31 | `hmput_key`/`hmget_key` | STRING via implicit `hmput_key(NULL,…,mode=1)` → `string.mode = SH_DEFAULT`, caller-owned keys | [x] |
| 32 | `shmode_func(SH_STRDUP)` + `hmput_key`/`hmget_key` | STRING, keys duplicated into heap, `temp_key` points at the dup | [x] |
| 33 | `shmode_func(SH_ARENA)` + `hmput_key`/`hmget_key` | STRING, keys allocated in the arena, `string.{remaining,block}` tracked | [x] |
| 34 | `shmode_func(SH_NONE)` + `hmput_key` | `mode=1` but `string.mode=0` → `default:` `memcpy` of `keysize` bytes | [x] |
| 35 | `shmode_func(4/255/256/259/-1)` + `hmput_key` | out-of-range `shmode` truncated to `unsigned char`; `259&0xFF == 3` → arena | [x] |
| 36 | `hmput_key` | `mode = 2` (`PTR_TO_STRING`): STRING hashing but `string.mode` still `SH_DEFAULT` | [x] |
| 37 | `hmput_key` | `mode` ∈ {`-1`,`INT_MIN`} → BINARY path | [x] |
| 38 | `hmput_key` | `mode` ∈ {`1000`,`INT_MAX`} → STRING path | [x] |
| 39 | `hmdel_key` | BINARY, delete the last-inserted key (`old_index == final_index`, no memmove) | [x] |
| 40 | `hmdel_key` | BINARY, delete a middle key (memmove + re-find + index patch) | [x] |
| 41 | `hmdel_key` | BINARY, random interleaved put/get/del sequence of 500 ops, `elemsize=16` | [x] |
| 42 | `hmdel_key` | BINARY, delete enough of a 100-entry map to cross `used_count_shrink_threshold` (shrink) | [x] |
| 43 | `hmdel_key` | BINARY, delete/reinsert loop to cross `tombstone_count_threshold` (rebuild) | [x] |
| 44 | `hmdel_key` | BINARY, delete every key then re-put them all (tombstone reuse) | [x] |
| 45 | `hmdel_key` | STRING + `SH_DEFAULT`, delete present and absent keys | [x] |
| 46 | `hmdel_key` | STRING + `SH_STRDUP`, `mode=1` → dup freed; then delete-all | [x] |
| 47 | `hmdel_key` | STRING + `SH_ARENA`, delete + shrink | [x] |
| 48 | `hmdel_key` | `keyoffset != 0` (e.g. `elemsize=24`, key at offset 8) with BINARY mode | [x] |
| 49 | `hmfree_func` | after a BINARY map with 100 entries | [x] |
| 50 | `hmfree_func` | after a `SH_STRDUP` map (frees each dup) | [x] |
| 51 | `hmfree_func` | after a `SH_ARENA` map (runs `strreset` over multiple blocks) | [x] |
| 52 | `stbds_stralloc` | fresh arena, `len` ∈ {1,…,64} (`remaining==0` → 512-byte block) | [x] |
| 53 | `stbds_stralloc` | repeated small allocs until a block is exhausted → next block, `block` 0→1→2… | [x] |
| 54 | `stbds_stralloc` | `len > blocksize` with `storage == NULL` (oversized first alloc, sets `remaining=0`) | [x] |
| 55 | `stbds_stralloc` | `len > blocksize` with `storage != NULL` (splice after head, `remaining` kept) | [x] |
| 56 | `stbds_stralloc` | ~4000 randomized allocs driving `block` to the `1<<20` saturation point | [x] |
| 57 | `stbds_stralloc` + `stbds_strreset` | alloc many blocks then reset → arena fully zeroed; reset twice | [x] |
| 58 | `stbds_strreset` | fresh (all-zero) arena | [x] |
| 59 | `strkey` | `n` ∈ {0,1,-1,`INT_MIN`,`INT_MAX`} + 200 random `i32` | [x] |
| 60 | `arr_ins` | `num` ∈ {0,1,4,-1,`INT_MIN`,`INT_MAX`} + 500 random `i32` (exit status + no abort) | [x] |
| 61 | end-to-end pipeline | `shmode_func(SH_STRDUP)` → 200 `shput`-style `hmput_key` → `hmget_key` all → `hmdel_key` half → re-put → `hmfree_func`, full state compare at every step | [x] |
| 62 | end-to-end pipeline | implicit BINARY `stbds_struct` map: 300 puts / interleaved gets / 150 dels / re-puts, full state compare at every step | [x] |
| 63 | end-to-end pipeline | `stbds_rand_seed(random)` before rows 61/62 so all tables use a non-default seed chain | [x] |
| 64 | `hmget_key` → `hmput_key` → `hmdel_key` | map first materialised by `hmget_key(NULL,…)` (length 1, `hash_table == NULL`), then populated — exercises hmput_key's "a != NULL but table == NULL" path, BINARY and STRING | [x] |
| 65 | `hmdel_key` → `hmget_key` → `hmput_key` | delete as the very first operation (on `NULL`, then on a table-less map), then normal use | [x] |
| 66 | `hmput_default` + `hmput_key` + `hmdel_key` | non-zero default element at index 0 must survive 300 inserts (7 table growths) and 300 deletes (all shrinks) | [x] |
| 67 | `shmode_func` + `hmput_key`/`hmdel_key` | extreme `elemsize`: 8 (element is only the key pointer, zero-length value), 40, 64, 128; and wide BINARY keys (`keysize` 24/32/64) | [x] |
| 68 | `hmput_key` + `hmdel_key` | STRING mode with `keyoffset = 8` — the `*(char **)(elem + keyoffset)` re-find branch after the delete memmove | [x] |
| 69 | `shmode_func` + `hmput_default` + `hmput_key` | `hmput_default` on a `shmode_func` map must be a NOP (`length` is already 1), for all four `shmode` values | [x] |
| 70 | `shmode_func` + `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` | 800-op randomized streams over a 50-key space (heavy collisions, tombstones, rebuilds, shrinks) × `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` × 4 random global seeds | [x] |

## Where each row is verified

| rows | test file |
|------|-----------|
| 1-9   | `tests/phase_b_hash.rs` |
| 10-22 | `tests/phase_b_array.rs` |
| 23-30, 39-44, 48, 49 | `tests/phase_b_map_binary.rs` |
| 31-38, 45-47, 50, 51 | `tests/phase_b_map_string.rs` |
| 52-58 | `tests/phase_b_arena.rs` |
| 59-63 | `tests/phase_b_misc.rs` |
| 64-70 | `tests/phase_b_extra.rs` |

Every row compares, after **every individual call**: the returned pointer's
null-ness, `stbds_array_header` {`length`, `capacity`, `temp`, `hash_table`
presence}, all `length` element payloads byte-for-byte (dereferencing the stored
`char *` in STRING modes), and the full `stbds_hash_index`
{`slot_count`, `slot_count_log2`, `used_count`, `tombstone_count`, all three
thresholds, `seed`, `string.{remaining, block, mode}`, arena block-chain length,
storage alignment} plus **every bucket's `hash[8]` and `index[8]`**.
`temp_key` is compared with a poison-pointer protocol, because
`stbds_make_hash_index` leaves the field uninitialised and `hmput_key`'s wrapped
probe half deliberately does not refresh it.

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **No driver binary exists**, so the
"compare stdout of the two binaries" requirement is not applicable.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table → the single (default)
configuration is the only one. `scripts/check_features.sh` enumerates them
mechanically and re-runs the whole suite for each.
