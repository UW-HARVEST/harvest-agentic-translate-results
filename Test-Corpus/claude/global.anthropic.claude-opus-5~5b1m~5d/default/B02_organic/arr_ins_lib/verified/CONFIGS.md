# CONFIGS.md — configuration-surface table (Phase A → Phase B)

Mechanically derived from the branch structure of `c_src/src/lib.c`.

## Axes the C actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `mode` (hash-map compare/hash mode) | `mode < 1` ⇒ **binary** (`memcmp` + `stbds_hash_bytes`); `mode >= 1` ⇒ **string** (`strcmp` + `stbds_hash_string`). Only `STBDS_HM_BINARY=0` / `STBDS_HM_STRING=1` named, any `int` accepted | lib.c:560, 590, 713 |
| `table->string.mode` (key-storage mode) | `STBDS_SH_NONE=0` / other ⇒ `memcpy` raw bytes; `SH_DEFAULT=1` ⇒ store caller pointer; `SH_STRDUP=2` ⇒ `malloc` copy; `SH_ARENA=3` ⇒ arena copy | lib.c:785-790 |
| how the table is created | implicitly by `stbds_hmput_key` (`string.mode = mode>=1 ? SH_DEFAULT : 0`) **or** explicitly by `stbds_shmode_func(elemsize, mode)` | lib.c:707, 796-805 |
| `elemsize` | `0`, `< 8`, `8`, `16`, non-power-of-two, large; determines array stride and header math | everywhere |
| `keysize` | `0`, `1`, `2`, `4`, `8`, `> elemsize`; only used by the binary path (`memcmp`, `hash_bytes`) and by the `default:` `memcpy` | lib.c:563, 789 |
| `keyoffset` | `0` (all internal callers) or non-zero (public `stbds_hmdel_key` parameter) | lib.c:561-563, 807 |
| element count | `0`, `1`, `2`, `7`, `8` (bucket boundary), `9`, `>= used_count_threshold` (6 for 8 slots) forcing rehash, hundreds forcing repeated doubling | lib.c:698-710 |
| `slot_count` | `8` (initial / minimum), then `16, 32, 64 …`; shrink back down but never below `8` | lib.c:702, 854 |
| probe path shape | in-bucket hit (`i = pos&7 … 7`), wrap-around scan (`i = 0 … pos&7`), multi-bucket probe (`pos += step; step += 8`) | lib.c:604-628, 728-764 |
| tombstones | none / one / above `tombstone_count_threshold` (`slot_count>>3 + slot_count>>4`) | lib.c:396, 858 |
| `stbds_hash_bytes` input shape | `len = 0..40`; `len % 8 == 0..7` (tail `switch` fall-through cases 0-7); bytes with the high bit set at positions 3 and 7 (sign-extension quirk) | lib.c:522-541 |
| `stbds_hash_string` input shape | `""`, 1 char, long, bytes `>= 0x80`, embedded high bytes | lib.c:477-491 |
| `seed` | default `0x31415926`, `0`, `1`, `SIZE_MAX`, random; **advanced** by every fresh `stbds_make_hash_index` (`seed = seed*a + b`) | lib.c:353-358, 410-412 |
| `stbds_arrgrowf` shape | `a` NULL / non-NULL; `addlen` `0`/`1`/`n`/`SIZE_MAX`; `min_cap` `0`/small/`> 2*cap`/`< 2*cap` (doubling), `< 4` (clamp), `<= cap` (no-op) | lib.c:276-310 |
| arena shape | `remaining` `0`/small/large; `block` `0..22` (saturation), forced `>= 128` (shift-count overflow); string `len` `1`/`< blocksize`/`> blocksize`; `storage` NULL / chain | lib.c:881-918 |

## Configuration rows

`t` = user pointer (`arr_base + elemsize`). Every row is exercised with **many
seeded-random inputs** (`SplitMix64`, fixed seeds) and compared byte-for-byte
between the C and Rust `.so`s: return values, `stbds_array_header`
(`length`/`capacity`/`temp`), all element bytes (keys resolved through the
`char*` indirection where the mode stores pointers), and the entire
`stbds_hash_index` (all scalar fields + every bucket's `hash[8]`/`index[8]`).

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| C01 | `stbds_rand_seed` + `stbds_hash_bytes` | `len = 0`, random seeds | `c01_hash_bytes_len0` | [x] |
| C02 | `stbds_hash_bytes` | `len = 1..7` (every tail `switch` case), random bytes, random seeds | `c02_hash_bytes_short_tails` | [x] |
| C03 | `stbds_hash_bytes` | `len = 8..64`, `len % 8 == 0` (body loop only, no tail) | `c03_hash_bytes_aligned` | [x] |
| C04 | `stbds_hash_bytes` | `len = 9..71`, `len % 8 != 0` (body loop **and** tail) | `c04_hash_bytes_body_plus_tail` | [x] |
| C05 | `stbds_hash_bytes` | bytes forced `>= 0x80` at indices 3 / 7 / all (sign-extension quirk) | `c05_hash_bytes_high_bit_bytes` | [x] |
| C06 | `stbds_hash_bytes` | seed `= 0`, `1`, `SIZE_MAX`, `0x31415926`, random 64-bit | `c06_hash_bytes_seeds` | [x] |
| C07 | `stbds_hash_string` | `""`, 1..64 random ASCII chars, random seeds | `c07_hash_string_ascii` | [x] |
| C08 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` | `c08_hash_string_high_bytes` | [x] |
| C09 | `stbds_arrgrowf` | `a = NULL`, cross-product of `elemsize ∈ {0,1,4,8,16,24}` × `addlen ∈ {0,1,3,64}` × `min_cap ∈ {0,1,3,4,5,100}` | `c09_arrgrowf_fresh_matrix` | [x] |
| C10 | `stbds_arrgrowf` | existing array, repeated growth (doubling path `min_cap < 2*cap`), random `addlen` | `c10_arrgrowf_doubling` | [x] |
| C11 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` (explicit `arrsetcap` shape) | `c11_arrgrowf_explicit_cap` | [x] |
| C12 | `stbds_arrgrowf` | existing array, `min_cap <= cap` (no-op return) and payload preserved across realloc | `c12_arrgrowf_noop_and_preserve` | [x] |
| C13 | `stbds_arrfreef` | free a grown array (address parity not compared; header content before free is) | `c13_arrfreef` | [x] |
| C14 | array macro pipeline `arrput`/`arraddn`/`arrins`/`arrdel`/`arrdelswap`/`arrsetlen`/`arrsetcap`/`arrpop` driven through `stbds_arrgrowf` | random op sequences (300 ops) on `int`, `u8`, 24-byte struct elements | `c14_array_macro_pipeline` | [x] |
| C15 | `arr_ins` | `num ∈ {0,1,4,-1,INT_MIN,INT_MAX}` + random | `c15_arr_ins` | [x] |
| C16 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,INT_MIN,INT_MAX}` + random | `c16_strkey` | [x] |
| C17 | `stbds_hmput_default` | `a = NULL`; then again on the result (no-op); `elemsize ∈ {1,4,8,16,24}` | `c17_hmput_default` | [x] |
| C18 | `stbds_hmput_key` binary | `mode = 0`, `elemsize = 8`, `keysize = 4` (int key/int value), **1** key | `c18_hm_binary_one` | [x] |
| C19 | `stbds_hmput_key` binary | `mode = 0`, insert `n = 0..40` distinct random keys (crosses `used_count_threshold` at 6, 12, 24 …) | `c19_hm_binary_many` | [x] |
| C20 | `stbds_hmput_key` binary | duplicate keys re-put (both the in-bucket and the wrap-around scan) | `c20_hm_binary_duplicates` | [x] |
| C21 | `stbds_hmput_key`+`stbds_hmget_key` binary | interleaved put/get, hits and misses, `elemsize ∈ {8,16,24}`, `keysize ∈ {1,2,4,8}` | `c21_hm_binary_put_get_matrix` | [x] |
| C22 | `stbds_hmget_key_ts` binary | same as C21 but through the `_ts` entry point, `temp` out-param compared | `c22_hm_binary_get_ts` | [x] |
| C23 | `stbds_hmdel_key` binary | delete existing (last element / middle element), `keyoffset = 0` | `c23_hm_binary_del` | [x] |
| C24 | `stbds_hmdel_key` binary | delete enough to cross `used_count_shrink_threshold` (table shrink) | `c24_hm_binary_del_shrink` | [x] |
| C25 | `stbds_hmdel_key` binary | delete pattern that crosses `tombstone_count_threshold` (table rebuild, same `slot_count`) | `c25_hm_binary_del_rebuild` | [x] |
| C26 | `stbds_hmdel_key` binary | `keyoffset != 0` (key not at element offset 0) | `c26_hm_binary_keyoffset` | [x] |
| C27 | full binary map pipeline | 400 random ops (`put`/`get`/`get_ts`/`del`/`default`) with a fixed seed, snapshot after **every** op | `c27_hm_binary_random_pipeline` | [x] |
| C28 | `stbds_hmput_key` string, implicit table | `mode = 1`, table created by `hmput_key` ⇒ `string.mode = SH_DEFAULT`; keys are caller-owned `char*` | `c28_sh_default_implicit` | [x] |
| C29 | `stbds_shmode_func(elemsize, STBDS_SH_STRDUP)` + `hmput_key`/`hmget_key`/`hmdel_key`/`hmfree_func` | `mode = 1`, strdup key ownership, random string keys, deletes free the copies | `c29_sh_strdup_pipeline` | [x] |
| C30 | `stbds_shmode_func(elemsize, STBDS_SH_ARENA)` + full pipeline | arena key storage; arena `block`/`remaining` progression compared | `c30_sh_arena_pipeline` | [x] |
| C31 | `stbds_shmode_func(elemsize, STBDS_SH_NONE)` + `hmput_key(mode=1)` | string hashing/compare but raw `memcpy` storage (mixed-mode quirk) | `c31_sh_none_string_mode` | [x] |
| C32 | `stbds_shmode_func(elemsize, STBDS_SH_DEFAULT)` + full pipeline | explicit `SH_DEFAULT` | `c32_sh_default_explicit` | [x] |
| C33 | string map, key shapes | `""`, 1-char, 8-char, 64-char, high-byte, common-prefix keys (`test_0..test_N` from `strkey`) | `c33_sh_key_shapes` | [x] |
| C34 | string map, size | `n = 0..40` distinct string keys (crosses every rehash boundary) | `c34_sh_many_keys` | [x] |
| C35 | full string map pipeline | 400 random ops with a fixed seed × each of the 4 `STBDS_SH_*` modes, snapshot after every op | `c35_sh_random_pipeline` | [x] |
| C36 | `stbds_hmfree_func` | after a binary map / `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA` map, and on a table-less array | `c36_hmfree_all_modes` | [x] |
| C37 | `stbds_stralloc` | fresh zeroed arena, strings of `len` `1`, `10`, `511`, `512`, `513`, `1000` (both branches of `len > blocksize`) | `c37_stralloc_shapes` | [x] |
| C38 | `stbds_stralloc` | many sequential allocations from one arena until `block` saturates at 22 (`512<<11 == 1<<20`) | `c38_stralloc_block_growth` | [x] |
| C39 | `stbds_stralloc` | pre-set `remaining`/`block`/`storage` combinations, incl. `block ∈ {0,1,2,21,22,23,63,64,127,255}` | `c39_stralloc_preset_arena` | [x] |
| C40 | `stbds_strreset` | empty arena, 1-block arena, multi-block arena, arena with an oversized spliced block | `c40_strreset_shapes` | [x] |
| C41 | `stbds_rand_seed` interaction | seed set, then N maps created (each `make_hash_index` advances the global seed) — verifies the seed LCG and per-table `seed` capture | `c41_seed_lcg_progression` | [x] |
| C42 | seed = default (never call `rand_seed`) | fresh library load, first table uses `0x31415926` | `c42_default_seed` | [x] |
| C43 | `stbds_hmput_key` | `keysize = 0`, binary mode (all keys equal) | `c43_keysize_zero` | [x] |
| C44 | `stbds_hmput_key` | `elemsize = 0` | `c44_elemsize_zero` | [x] |
| C45 | `stbds_hmput_key` | keys engineered to land in the same bucket (forces wrap-around scan + multi-bucket probe + `pos += step`) | `c45_forced_collisions` | [x] |
| C46 | `stbds_hmput_key`/`hmdel_key` | grow to `slot_count = 64+`, then delete down to `8` and grow again (rehash of a rehashed table) | `c46_grow_shrink_grow` | [x] |
| C47 | `stbds_hash_bytes` / `stbds_hash_string` | agreement of the two hashes for the *same* buffer when driven through the map (`mode` 0 vs 1 on identical bytes) | `c47_hash_fn_cross_mode` | [x] |
| C48 | `stbds_arrgrowf` | `elemsize` large (`1024`), `min_cap` large (`4096`) — real allocation sizes | `c48_arrgrowf_large` | [x] |

## Cross-cutting randomized runs (`tests/phase_b_stress.rs`)

These are not extra rows so much as a *composition* check: the per-row tests each
pin one configuration, while these interleave them so that composed states get
exercised (rehash of an already-rehashed table, tombstone reuse across a shrink,
arena growth while a map churns, several maps alive at once with different
captured seeds). Every operation is still compared byte-for-byte immediately.

| # | entry point(s) | configuration | test | ✔ |
|---|----------------|---------------|------|---|
| S1 | `hash_bytes` + `hash_string` | 1500 random inputs, `len` up to 4096, seeds `0`/`SIZE_MAX`/powers of two/random | `stress_hash_long_buffers` | [x] |
| S2 | full `arr*` macro pipeline | 600 random ops × `elemsize ∈ {1,2,3,5,8,13,16,24,33,64}` (incl. non-power-of-two strides) | `stress_array_pipeline` | [x] |
| S3 | binary map | 700 random ops × `elemsize ∈ {8,16,24,40}` × `mode ∈ {0,-1,INT_MIN}` | `stress_map_binary` | [x] |
| S4 | lazily-created string map | 700 random ops × `elemsize ∈ {8,16,24,40}` × `mode ∈ {1,2,INT_MAX}` | `stress_map_string_lazy` | [x] |
| S5 | `shmode_func` map | 400 random ops × `shmode ∈ {0,1,2,3,7,255}` × `elemsize ∈ {8,16,32}` × `mode ∈ {1,2}` | `stress_map_string_shmodes` | [x] |
| S6 | arena + map interleaved | 800 ops mixing an independent `stralloc`/`strreset` arena with an `SH_ARENA` map | `stress_arena_and_map_interleaved` | [x] |
| S7 | `rand_seed` mid-flight | 14 maps created under 14 different seeds, then 600 interleaved ops across all of them | `stress_reseed_midflight` | [x] |
| S8 | `hash_bytes` | **exhaustive**: every byte value 0..255 at every position, `len = 1..9`, 4 seeds | `exhaustive_hash_bytes_single_byte` | [x] |
| S9 | `hash_string` | **exhaustive**: every 1-byte and every 2-byte non-NUL string (65 280 inputs) | `exhaustive_hash_string_short` | [x] |
| S10 | `hash_bytes` | **exhaustive**: every single-bit-set 16-byte buffer × `len = 1..16`, plus every all-ones prefix × `len = 0..16` | `exhaustive_hash_bytes_bit_patterns` | [x] |
| S11 | `strkey` | every decimal-digit boundary ±2, both `int` extremes, and a 4096-point strided sweep of the whole `int` range | `exhaustive_strkey_boundaries` | [x] |

## Test-file map

| file | rows |
|------|------|
| `tests/common/mod.rs` | harness: dual-`dlopen` loader, `DiffArr` (array-macro driver), `DiffMap` (map-macro driver), state snapshotter, `SelfKeys`, slot/bucket-targeting helpers |
| `tests/phase_b_hash.rs` | C01–C08 + E53–E58, B06, S8–S11 (exhaustive sweeps) |
| `tests/phase_b_array.rs` | C09–C16, C48 + E01–E06, E60–E62, B01/B03 (array half) |
| `tests/phase_b_map_binary.rs` | C17–C27, C41–C47 + E18/E19/E31/E43 |
| `tests/phase_b_map_string.rs` | C28–C36 + E21/E22/E26/E45/E64 |
| `tests/phase_b_arena.rs` | C37–C40 + E46–E52 |
| `tests/phase_b_stress.rs` | S1–S7 |
| `tests/phase_c_errors.rs` (cont.) | E23/E24 `temp_key` asymmetry (`e23_e24_temp_key_asymmetry`) |
| `tests/phase_c_errors.rs` | E07–E44, E59, E63 + B01–B05 |
| `tests/phase_c_aborts.rs` | E65–E69 (subprocess crash-parity) |
| `tools/check_alloc_trace.sh` + `tools/alloc_tracer.c` + `tools/alloc_driver.c` | allocation-call-sequence parity (6 scenarios) |

## Feature combinations (Phase D)

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one configuration:

* default (== `--no-default-features`, == all-features)

`run_all.sh` enumerates the combinations mechanically from `Cargo.toml`
(the power set of any declared features plus the three canonical
configurations) and, for each one, runs:

1. `cargo check`
2. `cargo build --release` + `./check_symbols.sh` (Phase A/D symbol diff)
3. the whole differential suite against the **release** cdylib
4. `cargo build` (debug) + the whole suite again against the **debug** cdylib

Step 4 matters: the debug profile turns on `overflow-checks` and
`debug-assertions`, so it proves that every wrapping computation is written as an
explicit `wrapping_*` (the C wraps silently) and that no reachable code path
relies on Rust's debug-only pointer UB checks.

Measured result: **ALL CHECKS PASSED** for `DEFAULT`, `--no-default-features`
and `--all-features`, in both cdylib profiles.
