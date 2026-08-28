# ERRORS.md — error / rejection surface table (Phase A → Phase C)

Mechanically derived from `c_src/src/lib.c`. Every distinct rejection path,
sentinel return, `STBDS_ASSERT` (== `assert`, **live**: `C_FLAGS = -fPIC`, no
`NDEBUG`), null check, range check and min/max constant gets one row.

`t` denotes the "user" pointer the stb_ds macros hand around
(`t == arr_base + elemsize`); `raw` denotes `arr_base`.
Sentinels: `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`.

Test file: `tests/phase_c_errors.rs` (abort-parity rows: `tests/phase_c_aborts.rs`).

| # | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|---|----------|-------------------------------------------|-------------------|------|---|
| E01 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap` (lib.c:286) | returns `a` **unchanged**, no realloc, header untouched | `e01_arrgrowf_nogrow` | [x] |
| E02 | `stbds_arrgrowf` | `a == NULL` (lib.c:300) | fresh block; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap,addlen,4)` | `e02_arrgrowf_null_a` | [x] |
| E03 | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0 && a == NULL` → `min_len=0`, `min_cap=0`; `0 <= arrcap(NULL)=0` | returns `NULL` (no allocation at all) | `e03_arrgrowf_zero_zero_null` | [x] |
| E04 | `stbds_arrgrowf` | `min_cap < 4` clamp (lib.c:291) — e.g. `addlen=1,min_cap=0` on `NULL` | `capacity == 4` | `e04_arrgrowf_min_cap_clamp` | [x] |
| E05 | `stbds_arrgrowf` | `elemsize == 0` | allocates `0*min_cap + 32` bytes, `capacity = min_cap` | `e05_arrgrowf_elemsize_zero` | [x] |
| E06 | `stbds_arrgrowf` | `addlen` huge so `arrlen+addlen` wraps (`addlen = SIZE_MAX`) | `min_len` wraps to `len-1`; then `min_cap` logic on the wrapped value | `e06_arrgrowf_addlen_wrap` | [x] |
| E07 | `stbds_hmfree_func` | `a == NULL` (lib.c:573) | returns immediately, no free | `e07_hmfree_null` | [x] |
| E08 | `stbds_hmfree_func` | `a != NULL` but `stbds_header(a)->hash_table == NULL` (lib.c:574) | skips strdup-sweep + `strreset`; frees `hash_table` (NULL, no-op) and header | `e08_hmfree_no_table` | [x] |
| E09 | `stbds_hmfree_func` | `hash_table != NULL` and `string.mode != STBDS_SH_STRDUP` (lib.c:575) | no per-element `free`; only `strreset` + 2 frees | `e09_hmfree_table_not_strdup` | [x] |
| E10 | `stbds_hm_find_slot` | probe reaches a bucket slot with `hash == STBDS_HASH_EMPTY` (lib.c:609/620) | returns `-1` (key absent) | `e10_find_slot_miss` (via `hmget_key`) | [x] |
| E11 | `stbds_hmget_key_ts` | `a == NULL` (lib.c:634) | allocates 1-element default row, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL `t` | `e11_hmget_ts_null_a` | [x] |
| E12 | `stbds_hmget_key_ts` | `a != NULL`, `hash_table == 0` (lib.c:644) | `*temp = -1`, returns `a` unchanged, header untouched | `e12_hmget_ts_no_table` | [x] |
| E13 | `stbds_hmget_key_ts` | key not present, `slot < 0` (lib.c:648) | `*temp = STBDS_INDEX_EMPTY (-1)` | `e13_hmget_ts_absent_key` | [x] |
| E14 | `stbds_hmget_key` | any of E11–E13 | same as `_ts` **and** `stbds_header(t-elemsize)->temp == *temp` | `e14_hmget_key_writes_temp` | [x] |
| E15 | `stbds_hmget_key` / `_ts` | `mode` out of range **negative** (`-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING(1)` false ⇒ **binary** path (`memcmp`/`hash_bytes`) | `e15_mode_negative` | [x] |
| E16 | `stbds_hmget_key` / `_ts` | `mode` out of range **> 1** (`2`, `7`, `INT_MAX`) | `mode >= 1` true ⇒ **string** path (`strcmp`/`hash_string`) | `e16_mode_above_range` | [x] |
| E17 | `stbds_hmput_default` | `a == NULL` (lib.c:669) | allocates `min_cap=1`→`capacity=4`, `length=1`, element 0 zeroed | `e17_hmput_default_null` | [x] |
| E18 | `stbds_hmput_default` | `a != NULL` but `stbds_header(raw)->length == 0` (lib.c:669) | grows/reuses, `length` becomes 1, element 0 zeroed | `e18_hmput_default_len0` | [x] |
| E19 | `stbds_hmput_default` | `a != NULL`, `length != 0` | returns `a` **unchanged** (element 0 NOT zeroed) | `e19_hmput_default_noop` | [x] |
| E20 | `stbds_hmput_key` | `a == NULL` (lib.c:686) | creates default row first, then inserts; `temp == 0` for the 1st real key | `e20_hmput_key_null_a` | [x] |
| E21 | `stbds_hmput_key` | `table == NULL` (lib.c:698) | fresh index with `slot_count = STBDS_BUCKET_LENGTH (8)`; `string.mode = (mode>=1 ? SH_DEFAULT : 0)` | `e21_hmput_key_first_table` | [x] |
| E22 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` (= `slot_count - slot_count/2^2`) | rehash to `slot_count*2`, old table freed | `e22_hmput_key_grow_threshold` | [x] |
| E23 | `stbds_hmput_key` | duplicate key found in the **first** (`i = pos&7 .. 7`) scan (lib.c:729) | no new element; `temp = existing index`; string mode also sets `table->temp_key` | `e23_hmput_dup_first_scan`, `e23_e24_temp_key_asymmetry` | [x] |
| E24 | `stbds_hmput_key` | duplicate key found in the **wrap-around** (`i = 0 .. pos&7`) scan (lib.c:747) | no new element; `temp = existing index`; **`temp_key` NOT updated** (C quirk) | `e24_hmput_dup_wrap_scan`, `e23_e24_temp_key_asymmetry` | [x] |
| E25 | `stbds_hmput_key` | probe hits a tombstone (`hash==DELETED`, `index==STBDS_INDEX_DELETED`) before an empty slot (lib.c:739/755) | reuses tombstone slot, `--tombstone_count`, `++used_count` | `e25_hmput_reuses_tombstone` | [x] |
| E26 | `stbds_hmput_key` | `(size_t)i+1 > stbds_arrcap(a)` → realloc (lib.c:774) | array grown; `STBDS_ASSERT(i+1 <= arrcap)` holds | `e26_hmput_array_regrow` | [x] |
| E27 | `stbds_hmput_key` | `table->string.mode == STBDS_SH_DEFAULT (1)` | key slot stores the **caller's pointer** verbatim; `temp_key` = same pointer | `e27_put_mode_default` | [x] |
| E28 | `stbds_hmput_key` | `table->string.mode == STBDS_SH_STRDUP (2)` | key slot = fresh `malloc`'d copy; `temp_key` = that copy | `e28_put_mode_strdup` | [x] |
| E29 | `stbds_hmput_key` | `table->string.mode == STBDS_SH_ARENA (3)` | key slot = arena pointer; arena `remaining`/`block` advance | `e29_put_mode_arena` | [x] |
| E30 | `stbds_hmput_key` | `table->string.mode` = `STBDS_SH_NONE(0)` or **any other value** (default label, lib.c:789) | `memcpy(a+elemsize*i, key, keysize)` — raw bytes, `temp_key` untouched | `e30_put_mode_default_label` | [x] |
| E31 | `stbds_hmput_key` | `keysize == 0` with binary mode | `hash_bytes(key,0,seed)`, `memcmp(...,0)==0` ⇒ **every** key collides on slot 0 | `e31_put_keysize_zero` | [x] |
| E32 | `stbds_hmdel_key` | `a == NULL` (lib.c:809) | returns `0` (**NULL**) — the only NULL-returning path | `e32_hmdel_null_a` | [x] |
| E33 | `stbds_hmdel_key` | `hash_table == 0` (lib.c:816) | `stbds_header(raw)->temp = 0`; returns `a` unchanged | `e33_hmdel_no_table` | [x] |
| E34 | `stbds_hmdel_key` | key absent, `slot < 0` (lib.c:821) | `temp = 0`; returns `a`; `length`, `used_count`, `tombstone_count` unchanged | `e34_hmdel_absent_key` | [x] |
| E35 | `stbds_hmdel_key` | key present | `temp = 1`, `hash[i]=STBDS_HASH_DELETED(1)`, `index[i]=STBDS_INDEX_DELETED(-2)`, `--used_count`, `++tombstone_count`, `--length` | `e35_hmdel_present` | [x] |
| E36 | `stbds_hmdel_key` | `old_index == final_index` (deleting the last element) | **no** `memmove`, **no** second `find_slot` | `e36_hmdel_last_element` | [x] |
| E37 | `stbds_hmdel_key` | `old_index != final_index` | last element moved into the hole; its slot's `index` patched (`STBDS_ASSERT(slot>=0)`, `STBDS_ASSERT(b->index[i]==final_index)`) | `e37_hmdel_swap_in_last` | [x] |
| E38 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold (slot_count>>2)` **and** `slot_count > 8` | table rebuilt at `slot_count>>1`, old freed | `e38_hmdel_shrink` | [x] |
| E39 | `stbds_hmdel_key` | `slot_count <= STBDS_BUCKET_LENGTH (8)` → `used_count_shrink_threshold` forced to `0` (lib.c:399) | never shrinks below 8 slots | `e39_hmdel_no_shrink_at_8` | [x] |
| E40 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold (slot_count>>3 + slot_count>>4)` and no shrink | table rebuilt at the **same** `slot_count`, tombstones cleared | `e40_hmdel_rebuild_tombstones` | [x] |
| E41 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING(1)` **and** `string.mode == STBDS_SH_STRDUP(2)` | the stored key copy is `free`d before the swap | `e41_hmdel_strdup_frees_key` | [x] |
| E42 | `stbds_hmdel_key` | `mode >= 2` (out-of-range enum) with a strdup table | `mode == STBDS_HM_STRING` is **false** ⇒ key copy **leaked**, but `find_slot`/`is_key_equal` still take the *string* path (`mode >= 1`) | `e42_hmdel_mode_two` | [x] |
| E43 | `stbds_hmdel_key` | `keyoffset != 0` | offset added on top of `elemsize*i` in `is_key_equal` and in the re-find after the swap | `e43_hmdel_keyoffset` | [x] |
| E44 | `stbds_shmode_func` | `mode` out of enum range (`-1`, `4`, `255`, `256`, `INT_MAX`) | `h->string.mode = (unsigned char) mode` — **truncated**, no validation; later `switch` falls to `default` (raw `memcpy`) | `e44_shmode_out_of_range` | [x] |
| E45 | `stbds_shmode_func` | `elemsize == 0` | `arrgrowf(0,0,0,1)` → 32-byte block, `length=1`, table with 8 slots | `e45_shmode_elemsize_zero` | [x] |
| E46 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` (lib.c:893) — oversized string | dedicated block spliced **after** `a->storage` (or set as storage with `remaining=0`); returns `sb->storage` | `e46_stralloc_oversized` | [x] |
| E47 | `stbds_stralloc` | first call on a zeroed arena (`storage == NULL`, `remaining == 0`) with `len <= 512` | new 512-byte block (`BLOCKSIZE_MIN`), `remaining = 512`, `block` → 1 | `e47_stralloc_first_block` | [x] |
| E48 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` → `a->block` stops incrementing (lib.c:890) | `block` saturates at 22 (`512<<11 == 1<<20`) | `e48_stralloc_block_saturates` | [x] |
| E49 | `stbds_stralloc` | `a->block` forced huge (e.g. 200, 255) — shift count `>= 64` | `(size_t)512 << (block>>1)` uses only the low 6 bits of the count (x86-64 `shl`) ⇒ `blocksize` may become `0`, then oversized path | `e49_stralloc_block_shift_overflow` | [x] |
| E50 | `stbds_stralloc` | empty string `""` (`len == 1`) | still consumes 1 byte of `remaining` | `e50_stralloc_empty_string` | [x] |
| E51 | `stbds_strreset` | already-empty arena (`storage == NULL`) | loop body never runs; whole 24-byte arena zeroed | `e51_strreset_empty` | [x] |
| E52 | `stbds_strreset` | arena with a chain of blocks | every block freed, arena zeroed | `e52_strreset_chain` | [x] |
| E53 | `stbds_hash_bytes` | `len == 0` | `data = 0 << 56`, `switch(0)` → `break`; well-defined value, `p` never dereferenced | `e53_hash_bytes_len0` | [x] |
| E54 | `stbds_hash_bytes` | `len == 1..7` (short tail, `switch` fall-through) | tail bytes folded with the sign-extension quirk at `case 4: data |= (d[3] << 24)` | `e54_hash_bytes_tails` | [x] |
| E55 | `stbds_hash_bytes` | tail byte `d[3] >= 0x80` (and `d[7] >= 0x80` in the body loop) | signed `int` shift ⇒ all high bits set after widening to `size_t` | `e55_hash_bytes_high_bit` | [x] |
| E56 | `stbds_hash_bytes` | `len == 0` with `p == NULL` | no dereference ⇒ returns a value, does not crash | `e56_hash_bytes_null_len0` | [x] |
| E57 | `stbds_hash_string` | empty string `""` | while loop skipped; avalanche runs on `seed` alone | `e57_hash_string_empty` | [x] |
| E58 | `stbds_hash_string` | bytes `>= 0x80` (`(unsigned char) *str`) | added as **unsigned** 0x80..0xFF, not sign-extended | `e58_hash_string_high_bytes` | [x] |
| E59 | `stbds_hm_find_slot` / `stbds_hmput_key` | computed `hash < 2` (collides with EMPTY/DELETED markers) | `hash += 2` before probing (lib.c:596, 719) | `e59_hash_lt_2_bumped` | [x] |
| E60 | `strkey` | `n < 0`, `n == INT_MIN`, `n == INT_MAX` | `sprintf("test_%d")` ⇒ `test_-2147483648`, `test_2147483647`; 256-byte static buffer, no bounds check | `e60_strkey_extremes` | [x] |
| E61 | `arr_ins` | any `int num` incl. `INT_MIN`/`INT_MAX`/`4` | 5 iterations, both `STBDS_ASSERT`s hold, no observable output (void) | `e61_arr_ins_all` | [x] |
| E62 | `stbds_arrfreef` | valid `a` (only defined use) | frees `stbds_header(a)`; `a == NULL` would `free((char*)NULL-32)` ⇒ UB, **not** a defined rejection | `e62_arrfreef_valid` | [x] |
| E63 | `stbds_make_hash_index` (via `stbds_hmput_key`) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | holds for every reachable `slot_count` (≥8, power of two); would `abort()` otherwise | `e63_make_hash_index_assert_holds` | [x] |
| E64 | `stbds_hmput_key` | `mode >= 1` (string) but `table->string.mode == 0` (table made by `shmode_func(_,0)`) | hashing/compare use **strings** while storage uses **`memcpy` of `keysize` bytes** — mixed-mode C quirk | `e64_string_mode_none_table` | [x] |
| E65 | abort parity | `stbds_stralloc` with `a->remaining` big enough to pass `len > remaining` but `storage == NULL` (`remaining = 64, storage = NULL`) | `p = a->storage->storage + ...` ⇒ NULL deref ⇒ **SIGSEGV (139)** in both | `abort_stralloc_null_storage` (subprocess) | [x] |
| E66 | abort parity | assert liveness: the C links `__assert_fail` (no `NDEBUG`), so the Rust must use `assert!`, not `debug_assert!` | both libraries contain live assert machinery | `abort_assert_is_live` | [x] |
| E67 | `stbds_hmdel_key` | `mode >= 2` **and** an element swap (`old_index != final_index`). The re-find uses `mode == STBDS_HM_STRING ? *(char**)elem : (char*)elem`; for `mode >= 2` the test is false while `stbds_hm_find_slot` still hashes as a *string*, so the wrong bytes are hashed, `slot == -1`, and `STBDS_ASSERT(slot >= 0)` fires | **SIGABRT (134)** in both (glibc `assert` / Rust `assert!`) | `abort_hmdel_mode2_swap` (subprocess) | [x] |
| E68 | abort parity | `stbds_arrfreef(NULL)` ⇒ `free((char*)NULL - 32)` | **SIGSEGV (139)** inside glibc `free` in both | `abort_arrfreef_null` (subprocess) | [x] |
| E69 | abort parity | `stbds_stralloc` with `a->block = 200` ⇒ `(size_t)512 << (100 & 63)` = 32 TiB ⇒ `realloc` returns NULL ⇒ `sb->next = ...` writes through NULL | **SIGSEGV (139)** in both (the Rust stores through libc `memcpy` so it faults instead of tripping a debug-profile UB check) | `abort_stralloc_absurd_blocksize` (subprocess) | [x] |

## Boundary / generic-FFI coverage required by Phase C

| # | area | inputs exercised | test | ✔ |
|---|------|------------------|------|---|
| B01 | null pointers | `hmfree_func(NULL)`, `hmdel_key(NULL)`, `hmget_key(NULL)`, `hmget_key_ts(NULL)`, `hmput_key(NULL)`, `hmput_default(NULL)`, `arrgrowf(NULL)`, `hash_bytes(NULL,0)` | `b01_null_pointers` | [x] |
| B02 | zero lengths | `keysize = 0`, `elemsize = 0`, `len = 0`, `addlen = 0`, `min_cap = 0`, `""` strings | `b02_zero_lengths` | [x] |
| B03 | oversized lengths | `addlen = SIZE_MAX`, `min_cap = SIZE_MAX/2`, arena `block = 255`, `keysize` > element size | `b03_oversized_lengths` | [x] |
| B04 | one step past valid range | `mode = -1, 0, 1, 2` (`STBDS_HM_*` has only 0,1); `shmode` `mode = -1,0,1,2,3,4,255,256` (`STBDS_SH_*` has only 0..3) | `b04_one_past_range` | [x] |
| B05 | out-of-range enum across FFI | `mode = INT_MIN`, `INT_MAX`, `0x7fffffff`, `0x100`, `1000` for every `mode`-taking entry point | `b05_enum_out_of_range` | [x] |
| B06 | seed extremes | `stbds_rand_seed(0)`, `SIZE_MAX`, `1`, and the built-in default `0x31415926` | `b06_seed_extremes` | [x] |

## Fatal-input parity table (measured)

`tests/phase_c_aborts.rs` runs each fatal row in a subprocess against each
`.so` and requires identical termination. Measured with both the **release** and
the **debug** Rust cdylib:

| row | case | C | Rust (release) | Rust (debug) |
|-----|------|---|----------------|--------------|
| E65 | `stralloc_null_storage`     | 139 (SIGSEGV) | 139 | 139 |
| E69 | `stralloc_absurd_blocksize` | 139 (SIGSEGV) | 139 | 139 |
| E67 | `hmdel_mode2_swap`          | 134 (SIGABRT) | 134 | 134 |
| E68 | `arrfreef_null`             | 139 (SIGSEGV) | 139 | 139 |
| —   | `harness_sanity` (control)  | 0             | 0   | 0   |

Reaching this parity required two changes to the translation:

1. every `STBDS_ASSERT` site uses `assert!` instead of `debug_assert!` (the C is
   built without `NDEBUG`);
2. `stbds_stralloc` / `stbds_strreset` store through the block chain with raw
   address arithmetic + libc `memcpy` (`raw_store_ptr` / `raw_load_ptr`), because
   a plain `(*p).field = v` on a NULL `p` trips Rust's debug-profile
   null/alignment UB check and aborts, where the C faults.

### Known, non-reachable residue

If `realloc` itself fails (true OOM), the C writes through the resulting NULL in
`stbds_arrgrowf` / `stbds_make_hash_index` and faults. In a **debug** Rust build
those same writes would trip the null-pointer UB check and abort instead. This
cannot be triggered through the FFI surface (there is no way to make the
allocator fail from the outside), it is identical in the shipped **release**
profile, and it is therefore not a row of the reachable error surface.

## Inputs that are heap-corrupting in the C itself (excluded by construction)

These are documented so the test suite's guards are not mistaken for gaps:

| input | why it is excluded |
|-------|--------------------|
| `stbds_hmput_key(NULL, elemsize < 8, key, _, mode >= 1)` | STRING mode stores a `char *` at element offset 0, so the C writes 8 bytes into a smaller element and corrupts the heap. `e20_hmput_key_null_a` skips `elemsize < 8` for string mode. |
| `keysize > elemsize` in binary mode | the C `memcpy`s `keysize` bytes into an `elemsize` element. Tests keep `keysize <= elemsize`. |
| `a->block` in 24..109 / 152..237 with `remaining == 0` | `512 << (block>>1)` (count taken mod 64) yields 2^21..2^63 and the C requests that many bytes. `c39_stralloc_preset_arena` restricts to the `block` values whose blocksize is either <= 1 MiB or collapses to 0; the extreme is covered by `abort_stralloc_absurd_blocksize`. |
| `elemsize * min_cap` overflowing in `stbds_arrgrowf` | the C allocates a wrapped (tiny) block and then writes `min_cap` elements into it. Tests use `elemsize == 0` when exercising `min_cap` near `SIZE_MAX`. |
