# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-out / sentinel / assert /
range-check / min-max constant in `c_src/src/lib.c`. This library has **no
error-code return convention**: it rejects exclusively via (a) early `return`
with a sentinel pointer/index, (b) `STBDS_ASSERT` (live — the `.so` imports
`__assert_fail`, see `SYMBOLS.md`), and (c) sentinel index values
`STBDS_INDEX_EMPTY = -1` / `STBDS_INDEX_DELETED = -2`.

Sentinel constants: `STBDS_INDEX_EMPTY -1`, `STBDS_INDEX_DELETED -2`,
`STBDS_HASH_EMPTY 0`, `STBDS_HASH_DELETED 1`,
`STBDS_STRING_ARENA_BLOCKSIZE_MIN 512u`, `STBDS_STRING_ARENA_BLOCKSIZE_MAX 1<<20`,
`STBDS_BUCKET_LENGTH 8`, `STBDS_CACHE_LINE_SIZE 64`,
`STBDS_HM_BINARY 0`, `STBDS_HM_STRING 1`,
`enum { STBDS_SH_NONE=0, STBDS_SH_DEFAULT=1, STBDS_SH_STRDUP=2, STBDS_SH_ARENA=3 }`.

## Sentinel / early-return rejections (all testable)

| # | function | trigger (exact invalid input/condition) | expected C result | test | ok |
|---|----------|------------------------------------------|-------------------|------|----|
| E1 | `stbds_arrgrowf` (lib.c:286) | `min_cap <= stbds_arrcap(a)` — request no bigger than current capacity | returns `a` **unchanged** (same pointer, no realloc), header untouched | `e1_arrgrowf_no_grow` | [x] |
| E2 | `stbds_arrgrowf` (lib.c:286) | `a == NULL` **and** `min_cap == 0 && addlen == 0` → `min_cap(0) <= arrcap(NULL)(0)` | returns `NULL` (no allocation at all) | `e2_arrgrowf_null_zero` | [x] |
| E3 | `stbds_arrgrowf` (lib.c:283/289/291) | `a == NULL`, `addlen=0`, `min_cap=1` → clamped up to 4 (`min_cap < 4`) | fresh block, `capacity == 4`, `length == 0`, `hash_table == NULL`, `temp == 0` | `e3_arrgrowf_min_cap_4` | [x] |
| E4 | `stbds_arrgrowf` | `elemsize == 0` (degenerate element size) | allocates only the 32-byte header; `capacity == min_cap`, no crash | `e4_arrgrowf_elemsize_zero` | [x] |
| E5 | `stbds_arrfreef` (lib.c:314) | `a == NULL` → `free(NULL - 32)` = `free((void*)-32)` | **invalid free / UB in C**; not exercised (would abort the harness). Rust does the byte-identical `wrapping_sub` + `free`. See row U1. | `e5_arrfreef_null_is_documented_not_run` (documented) | [x] |
| E6 | `stbds_hmfree_func` (lib.c:573) | `a == NULL` | returns immediately, no free, no crash | `e6_hmfree_null` | [x] |
| E7 | `stbds_hmfree_func` (lib.c:574) | `stbds_header(a)->hash_table == NULL` (array built by `arrgrowf` only, never `hmput_key`) | skips strdup-free loop and `strreset`; frees `hash_table(NULL)` + header | `e7_hmfree_no_table` | [x] |
| E8 | `stbds_hm_find_slot` (lib.c:610) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` in the *upper* scan → key absent | returns `-1` | via `e9`/`e10` | [x] |
| E9 | `stbds_hm_find_slot` (lib.c:621) | probe hits `STBDS_HASH_EMPTY` in the *wrapped* (`i < limit`) scan → key absent | returns `-1` | `e9_get_absent_key` | [x] |
| E10 | `stbds_hmget_key_ts` (lib.c:634) | `a == NULL` | allocates 1-elem zeroed array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL `arr+elemsize` | `e10_hmget_ts_null_a` | [x] |
| E11 | `stbds_hmget_key_ts` (lib.c:644) | `a != NULL` but `header(raw_a)->hash_table == 0` | `*temp = -1`, returns `a` unchanged | `e11_hmget_ts_no_table` | [x] |
| E12 | `stbds_hmget_key_ts` (lib.c:648) | slot lookup fails (`slot < 0`) — key not in a populated table | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `e9_get_absent_key` | [x] |
| E13 | `stbds_hmget_key` (lib.c:663) | any of E10–E12 | additionally writes `temp` into `header(p-elemsize)->temp`; for E10 that header is the freshly allocated one | `e13_hmget_temp_written` | [x] |
| E14 | `stbds_hmput_default` (lib.c:669) | `a == NULL` | grows a 1-elem array, `length == 1`, elem zeroed, returns `arr+elemsize` | `e14_hmput_default_null` | [x] |
| E15 | `stbds_hmput_default` (lib.c:669) | `a != NULL` and `header(raw_a)->length == 0` | same grow-and-zero path taken again | `e15_hmput_default_len0` | [x] |
| E16 | `stbds_hmput_default` (lib.c:675) | `a != NULL`, `length != 0` | returns `a` **unchanged**, nothing written | `e16_hmput_default_passthru` | [x] |
| E17 | `stbds_hmdel_key` (lib.c:809) | `a == NULL` | returns `0` / `NULL` | `e17_hmdel_null` | [x] |
| E18 | `stbds_hmdel_key` (lib.c:816) | `header(raw_a)->hash_table == 0` | sets `temp = 0`, returns `a` unchanged, length unchanged | `e18_hmdel_no_table` | [x] |
| E19 | `stbds_hmdel_key` (lib.c:821) | `slot < 0` — deleting a key that is not present | `temp == 0` (the "not deleted" sentinel `shdel` returns), returns `a`, length unchanged, `used_count` unchanged | `e19_hmdel_absent` | [x] |
| E20 | `stbds_hmdel_key` (lib.c:831) | key **is** present | `temp == 1`, `length -= 1`, tombstone written (`hash=1`, `index=-2`) | `e20_hmdel_present` | [x] |
| E21 | `stbds_hmdel_key` (lib.c:854) | `used_count < used_count_shrink_threshold && slot_count > 8` | table rebuilt at `slot_count>>1` | `e21_e22_hmdel_shrink_and_rebuild`, `c32_...` | [x] |
| E22 | `stbds_hmdel_key` (lib.c:858) | `tombstone_count > tombstone_count_threshold` (and no shrink) | table rebuilt at same `slot_count` | `e21_e22_hmdel_shrink_and_rebuild`, `c32_...` | [x] |
| E23 | `stbds_hash_string` (lib.c:480) | empty string `""` — loop body never runs | still returns a well-defined hash of just the seed | `e23_hash_string_empty` | [x] |
| E24 | `stbds_hash_bytes` (lib.c:522/532) | `len == 0` — main loop skipped, `switch(0)` falls to `break`, no byte read (`p` may even be dangling) | hash of `len<<56` only | `e24_hash_bytes_len0` | [x] |
| E25 | `stbds_hash_bytes` (lib.c:532) | `len - i` in 1..7 — every fall-through arm of the `switch` | one distinct value per remainder | `e25_hash_bytes_all_remainders` | [x] |
| E26 | `stbds_stralloc` (lib.c:893) | `len > blocksize` (string longer than the current block size, e.g. 5000 bytes on a fresh arena) | dedicated over-size block; on a fresh arena also forces `a->remaining = 0` | `e26_stralloc_oversize` | [x] |
| E27 | `stbds_stralloc` (lib.c:890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` — `a->block` saturates and stops incrementing | `a->block` frozen at the value where `512<<(block>>1) >= 1<<20` | `e27_stralloc_block_saturates` | [x] |
| E28 | `stbds_stralloc` (lib.c:884) | empty string `""` → `len == 1` | consumes exactly 1 byte of the arena | `e28_stralloc_empty` | [x] |
| E29 | `stbds_strreset` (lib.c:924) | `a->storage == NULL` (already-empty / fresh arena) | while-loop never runs; arena memset to all-zero | `e29_strreset_empty` | [x] |
| E30 | `stbds_strreset` | called twice in a row | second call is a no-op on an all-zero arena | `e29_strreset_empty` | [x] |
| E31 | `strkey` (lib.c:941) | `n == INT_MIN` (`-2147483648`) — the `%d` boundary | `"test_-2147483648"` | `e31_strkey_boundaries` | [x] |
| E32 | `sh_puts` (lib.c:951) | `num <= 0` — the `stralloc` loop body never runs | still prints exactly `"a <num>\n"` | `e32_sh_puts_nonpositive` | [x] |

## Out-of-range enum / mode values crossing the FFI boundary

C `mode` / `int mode` parameters accept **any** `int`. The only tests the C
performs are `mode >= STBDS_HM_STRING (1)` and a `switch` on
`table->string.mode` (an `unsigned char`).

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| M1 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` (lib.c:560,590,713) | `mode = 2` (`STBDS_HM_PTR_TO_STRING`, no named constant in this TU) | `mode >= 1` → treated exactly as `STBDS_HM_STRING` for hashing & compare | `m1_mode_two_is_string` | [x] |
| M2 | same | `mode = 7` / `mode = INT_MAX` (far past any valid variant) | `mode >= 1` → string behaviour | `m2_mode_large` | [x] |
| M3 | same | `mode = -1` / `mode = INT_MIN` (negative, no valid variant) | `mode < 1` → **binary** behaviour (`memcmp` / `hash_bytes`) | `m3_mode_negative` | [x] |
| M4 | `stbds_hmdel_key` (lib.c:836,842) | `mode = 2` on a strdup table — `mode == STBDS_HM_STRING` is **false** for 2 | the strdup key is **NOT** freed (leak) and `hm_find_slot` re-lookup uses the **binary** branch `(char*)a+off` rather than `*(char**)` | `m4_hmdel_mode2_binary_relookup` | [x] |
| M5 | `stbds_shmode_func` (lib.c:803) | `mode` outside `0..3`: `4`, `255`, `256` (→ `0` after `(unsigned char)`), `-1` (→ `255`) | `string.mode = (unsigned char) mode`; later `switch` in `hmput_key` hits `default:` → `memcpy(key, keysize)` binary-copy path | `m5_shmode_out_of_range` | [x] |
| M6 | `stbds_hmput_key` (lib.c:785) | `string.mode == STBDS_SH_NONE (0)` while `mode == STBDS_HM_STRING` — reachable via `shmode_func(elemsize, 0)` then a string put | `default:` arm → `memcpy` of `keysize` bytes over the key slot, `temp_key` **not** set | `m6_shmode_none_string_put` | [x] |
| M7 | `stbds_hmfree_func` (lib.c:575) | `string.mode` is any value != `STBDS_SH_STRDUP` (incl. out-of-range 255) | strdup-free loop skipped | `m5_shmode_out_of_range` | [x] |

## Generic FFI boundary cases

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| G1 | `stbds_hash_bytes` | `p = NULL, len = 0` | no deref, hash of `len<<56` | `e24_hash_bytes_len0` | [x] |
| G2 | `stbds_hash_bytes` | `len` = 1,7,8,9,15,16,17,63,64,65 (boundaries of the 8-byte block loop) | matching hashes | `e25_hash_bytes_all_remainders` | [x] |
| G3 | `stbds_hash_string` | seed = 0, 1, `usize::MAX`, `0x31415926` | matching hashes | `b_hash_string_random` | [x] |
| G4 | `stbds_rand_seed` | `0`, `usize::MAX` — resets the global seed that `make_hash_index` consumes | subsequent `shmode_func`/`hmput_key` tables get the identical `seed`, and the global advances identically | `g4_rand_seed_extremes` | [x] |
| G5 | `stbds_hmget_key_ts` | `temp` points at writable storage; `keysize` = 0 | `memcmp(...,0)` == 0 → *every* slot with a matching hash "matches" | `g5_keysize_zero` | [x] |
| G6 | `stbds_hmput_key` | `keysize` larger than `elemsize` (over-long key copy) | `memcpy` of `keysize` bytes — C overruns the element; both must overrun identically. Tested with `keysize == elemsize` (max safe) and `keysize < elemsize`. | `g6_keysize_variants` | [x] |
| G7 | `stbds_hmdel_key` | `keyoffset != 0` (non-zero offset of the key inside the element) | key compared/hashed at `elem + keyoffset` | `g7_keyoffset_nonzero` | [x] |
| G8 | `sh_puts` | `num` = `0`, `1`, `INT_MAX`, `INT_MIN`, `-1` | stdout byte-identical | `g8_sh_puts_stdout` | [x] |
| G9 | `strkey` | `n` = 0, 1, -1, 9, 10, 99, 100, `INT_MAX`, `INT_MIN` | byte-identical NUL-terminated string | `e31_strkey_boundaries` | [x] |

## Live `STBDS_ASSERT`s (the C build has NO `-DNDEBUG`)

The C `.so` imports `__assert_fail`, so every `STBDS_ASSERT` is **live**: on
failure the C writes glibc's assertion message to stderr and dies by `SIGABRT`.

**Verification finding:** `A5` (and, on other inputs, `A6`) turned out to be
**reachable from the public API** — the original Rust translation made
`stbds_assert!` a no-op, which silently corrupted memory (`storage[-1]`) where the
C aborts. That was a real divergence, found by `d2_fuzz_string_mixed`. The Rust
`stbds_assert!` now calls the **same `__assert_fail`** with the same assertion
text, line number and function name, so both libraries abort identically.
(The `__FILE__` prefix differs — the C records the absolute compile-time path,
the Rust records `src/lib.c` — because that string is a build-environment
artefact, not library behaviour. Everything after `lib.c:` matches byte for byte.)

| # | site | assertion | reachable? | test | ok |
|---|------|-----------|------------|------|----|
| A1 | lib.c:401 `make_hash_index` | `used_count_threshold + tombstone_count_threshold < slot_count` | **No.** `slot_count` is only ever `8`, `2n` or `n>>1` (guarded `> 8`), i.e. a power of two `>= 8`; the LHS is `(n-n/4)+(n/8+n/16) = 15n/16 < n` for `n >= 16` and `6+1 = 7 < 8` for `n = 8`. Exercised for every reachable `slot_count` from 8 to >= 4096 and back. | `x2_remaining_assertions_do_not_fire` | [x] |
| A2 | lib.c:778 `hmput_key` | `(size_t) i+1 <= stbds_arrcap(a)` | **No.** Guarded two lines above by `if ((size_t) i+1 > stbds_arrcap(a)) a = arrgrowf(a, elemsize, 1, 0)`, whose post-condition is `cap >= len+1`. | all Phase B put rows | [x] |
| A3 | lib.c:828 `hmdel_key` | `slot < (ptrdiff_t) table->slot_count` | **No.** `hm_find_slot` returns `(pos & ~7) + i` with `pos` masked by `slot_count-1` and `i < 8`. | all Phase B/C delete rows | [x] |
| A4 | lib.c:832 `hmdel_key` | `table->used_count >= 0` | **No** — vacuous: `used_count` is a `size_t` (gcc folds it away). Mirrored structurally in Rust under `#[allow(unused_comparisons)]`. | n/a | [x] |
| A5 | lib.c:846 `hmdel_key` | `slot >= 0` | **YES.** `hmdel_key(map, e, key, ks, 0, mode)` with `mode >= 2` (so `mode >= STBDS_HM_STRING` is true but `mode == STBDS_HM_STRING` is false) **and** `old_index != final_index`: the post-`memmove` re-lookup at lib.c:842-845 takes the *binary* branch and passes `(char*)a + elemsize*old_index + keyoffset`, yet `hm_find_slot` still hashes it *as a string* — so it hashes the key POINTER BYTES, misses, returns `-1`, and the assert fires. Both libraries must abort with `SIGABRT` and the same message. | `x1_assert_slot_ge_zero_aborts_identically` (child process) | [x] |
| A6 | lib.c:849 `hmdel_key` | `b->index[i] == final_index` | **YES in principle** — same input class as A5, on the (astronomically unlikely) inputs where the bogus binary/string re-lookup happens to *find* a slot pointing at a different index. Unreachable deterministically; the Rust assertion is live and identical, so any occurrence aborts in both. | covered by the live-assert wiring | [x] |
| A7 | lib.c:913 `stralloc` | `len <= a->remaining` | **No.** Either the fast path already had `len <= remaining`, or a block with `remaining = blocksize >= len` was just installed, or the `len > blocksize` branch returned early. Hammered with 3000 allocations whose lengths straddle `remaining` and `blocksize` exactly. | `x2_remaining_assertions_do_not_fire` | [x] |
| A8 | lib.c:959-961 `sh_puts` | `*strmap[0].key == 'a'`, `strmap[0].key != s.key`, `strmap[0].value == s.value` | **No.** Invariants of `sh_new_arena` + `shputs`; hold for every `num`. A failure in either library would abort the run. | `x2_remaining_assertions_do_not_fire`, `c53_sh_puts_stdout_byte_for_byte` | [x] |

## Additional unrunnable-in-C inputs (documented, deliberately not executed)

These are inputs where the **C itself** performs UB / aborts / allocates
unboundedly, so a differential test cannot observe a comparable result. Each is
translated byte-identically in the Rust and is recorded here for completeness.

| # | input | what the C does |
|---|-------|-----------------|
| U1 | `stbds_arrfreef(NULL)` | `free((char*)NULL - 32)` — invalid free, glibc aborts (row E5). |
| U2 | `shmode_func(e, STBDS_SH_NONE)` (or any `string.mode` outside 1..3) followed by a **lookup** with `mode >= STBDS_HM_STRING` | the key slot holds raw string BYTES, so `is_key_equal` dereferences them as a `char *` → SIGSEGV. Covered put-only by `c24` / `m5` / `m6`. |
| U3 | `stbds_shputs` on an **existing** key | the wrapped-scan hit branch (lib.c:746-751) sets `stbds_temp(a)` but not `stbds_temp_key(a)`, so a STALE `temp_key` (another element's pointer) is copied into the element; in `STBDS_SH_STRDUP` mode `hmfree_func` then double-frees. State compared, then deliberately leaked, by `c26b_shputs_existing_key_aliases_temp_key`. |
| U4 | `keysize > elemsize` in `hmput_key` | `memcpy(elem, key, keysize)` runs past the element and possibly past the array — heap corruption. `keysize == elemsize` (the maximum safe value) is covered by `g6`. |
| U5 | `sh_puts(INT_MAX)` | `for (i=0;i<num;++i) stralloc(...)` → 2^31 arena allocations (tens of GB). All small/negative `num` are covered (rows E32/G8). |
| U6 | `hash_string(NULL, seed)`, `stralloc(NULL, s)`, `strreset(NULL)` | unconditional NULL dereference in the C. |
| U7 | a hand-built `stbds_string_arena` with `storage == NULL` but `remaining >= len` | `a->storage->storage` → NULL dereference. |
| U8 | `hm_find_slot` on a table with **no** `STBDS_HASH_EMPTY` slot and an absent key | infinite loop (the C relies on `used_count_threshold + tombstone_count_threshold < slot_count` plus the rebuild to guarantee an empty slot always exists). |
