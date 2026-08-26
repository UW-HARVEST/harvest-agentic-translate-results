# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from
`grep -nE 'return|ASSERT|== NULL|== 0|< 0' c_src/src/lib.c`.

The library has **no error-code return convention** (no `errno`, no error enum,
no `RETURN_ERROR` macro).  Every "rejection" is one of

* an **early return of a sentinel** (`NULL` / the unmodified input pointer /
  `-1` / `STBDS_INDEX_EMPTY` written through an out-param), or
* a **`STBDS_ASSERT` (= glibc `assert`) → `__assert_fail` → `SIGABRT`**, or
* an **unchecked pointer dereference → `SIGSEGV`** (the C code's contract).

Each row is one distinct rejection site.  `[x]` = a differential test exists and
passes for both `.so`s.

| # | function (C line) | trigger (exact invalid input / condition) | expected C result | test | [x] |
|---|-------------------|-------------------------------------------|-------------------|------|-----|
| 1 | `stbds_arrgrowf` (287) | `min_cap <= stbds_arrcap(a)` after `min_len` clamp — e.g. `(NULL, 16, 0, 0)` | returns `a` **unchanged** (so `NULL` in → `NULL` out, no allocation) | `errors.rs::e01_arrgrowf_no_grow` | [x] |
| 2 | `stbds_arrgrowf` (297) | `elemsize*min_cap + 32` wraps mod 2^64 — e.g. `(NULL, 16, 0, 1<<60)` | `realloc(NULL, 32)`; header written; `capacity == 1<<60` | `errors.rs::e02_arrgrowf_size_overflow` | [x] |
| 3 | `stbds_hmfree_func` (573) | `a == NULL` | returns immediately, no free, no crash | `errors.rs::e03_hmfree_null` | [x] |
| 4 | `stbds_hmfree_func` (574) | `stbds_header(a)->hash_table == NULL` (map created by `hmget_key(NULL,…)` / `hmput_default(NULL,…)`) | skips strdup-free + `strreset`; frees `hash_table`(NULL) + header | `errors.rs::e04_hmfree_no_table` | [x] |
| 5 | `stbds_hm_find_slot` (610) | probe walks into a slot with `hash == STBDS_HASH_EMPTY` in the *upper* half of the bucket → key absent | `-1` | `errors.rs::e05_find_slot_miss` (via `hmget_key`) | [x] |
| 6 | `stbds_hm_find_slot` (621) | same, in the *wrapped* (`0..limit`) half of the bucket | `-1` | `errors.rs::e05_find_slot_miss` (randomised keys hit both halves) | [x] |
| 7 | `stbds_hmget_key_ts` (634) | `a == NULL` | allocates 1-elem sentinel array, `*temp = -1`, returns `ARR_TO_HASH(a)` | `errors.rs::e07_hmget_ts_null` | [x] |
| 8 | `stbds_hmget_key_ts` (644) | `a != NULL` but `hash_table == NULL` | `*temp = -1`, returns `a` unchanged (pointer identity) | `errors.rs::e08_hmget_ts_no_table` | [x] |
| 9 | `stbds_hmget_key_ts` (648) | key not in table (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `errors.rs::e09_hmget_miss` | [x] |
| 10 | `stbds_hmget_key` (663) | any of rows 7–9 | writes `temp` into `header(HASH_TO_ARR(p))->temp`; returns same `p` | `errors.rs::e10_hmget_key_temp` | [x] |
| 11 | `stbds_hmput_default` (669) | `a == NULL` | grows, `length += 1`, zeroes elem 0, returns `ARR_TO_HASH` | `errors.rs::e11_hmput_default_null` | [x] |
| 12 | `stbds_hmput_default` (669) | `a != NULL` **and** `header(HASH_TO_ARR(a))->length == 0` (reachable: `ARR_TO_HASH(arrgrowf(NULL,es,0,1))`) | same as row 11 but reuses the allocation | `errors.rs::e12_hmput_default_len0` | [x] |
| 13 | `stbds_hmput_default` (675) | `length != 0` | returns `a` **unchanged** (no allocation) | `errors.rs::e13_hmput_default_noop` | [x] |
| 14 | `stbds_hmput_key` (686) | `a == NULL` | allocates sentinel elem 0, then proceeds to insert | `errors.rs::e14_hmput_key_null` | [x] |
| 15 | `stbds_hmput_key` (730-734) | duplicate key found in the **upper** bucket half | does *not* append; `temp = existing index`; for `mode>=1` also sets `temp_key` | `errors.rs::e15_hmput_dup` | [x] |
| 16 | `stbds_hmput_key` (747-750) | duplicate key found in the **wrapped** bucket half | does *not* append; `temp = existing index`; **`temp_key` NOT set** (C quirk, preserved) | `errors.rs::e16_hmput_dup_wrapped` | [x] |
| 17 | `stbds_hmput_key` (778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | `SIGABRT` — invariant, unreachable through the public API (the preceding `arrgrowf` guarantees it) | `errors.rs::unreachable_assert_invariants` (asserts `length <= capacity` after every op of a 2500-op randomised workload) | [x] |
| 18 | `stbds_hmdel_key` (809) | `a == NULL` | returns `0` (**NULL**), no side effects | `errors.rs::e18_hmdel_null` | [x] |
| 19 | `stbds_hmdel_key` (816) | `hash_table == NULL` | sets `header->temp = 0`, returns `a` unchanged | `errors.rs::e19_hmdel_no_table` | [x] |
| 20 | `stbds_hmdel_key` (821) | key absent (`slot < 0`) | `header->temp = 0`, returns `a`, length unchanged | `errors.rs::e20_hmdel_miss` | [x] |
| 21 | `stbds_hmdel_key` (828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | `SIGABRT` — invariant, `find_slot` can only return `< slot_count` | `errors.rs::unreachable_assert_invariants` | [x] |
| 22 | `stbds_hmdel_key` (832) | `STBDS_ASSERT(table->used_count >= 0)` on a `size_t` | **dead code** — gcc folds it away; the assertion string is absent from the C `.so`'s `.rodata`.  Rust correctly omits it. | `errors.rs::e22_dead_assert_absent_from_c_so` (byte-scans both `.so`s) | [x] |
| 23 | `stbds_hmdel_key` (846) | `STBDS_ASSERT(slot >= 0)` — reachable by passing `keyoffset != 0` (which `hmput_key` ignores) so the post-`memmove` re-find misses | `SIGABRT`, `"slot >= 0"`, line 846, `stbds_hmdel_key` | `errors.rs::e23_hmdel_keyoffset_abort` (forked child, compares signal) | [x] |
| 24 | `stbds_hmdel_key` (849) | `STBDS_ASSERT(b->index[i] == final_index)` | `SIGABRT` — invariant given `keyoffset == 0` | `errors.rs::unreachable_assert_invariants`; the reachable `keyoffset != 0` variant is row 23 | [x] |
| 25 | `stbds_make_hash_index` (401) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count <= 2` | `SIGABRT` — unreachable: the only call sites pass `8`, `slot_count*2`, or `slot_count>>1` guarded by `slot_count > 8` | `errors.rs::unreachable_assert_invariants` (asserts `slot_count >= 8` and the threshold inequality after every op; also asserts `slot_count == 8` was actually observed) | [x] |
| 26 | `stbds_stralloc` (893-904) | `len > blocksize` (huge string, or `a->block` forced large so `512u<<(block>>1)` shifts out) | dedicated block; **returns early**; when `a->storage == NULL` also sets `a->remaining = 0` | `arena.rs::stralloc_big_block`, `errors.rs::e26_stralloc_block_shift` | [x] |
| 27 | `stbds_stralloc` (913) | `STBDS_ASSERT(len <= a->remaining)` | `SIGABRT` — invariant: the `len > blocksize` branch returns first, else `remaining = blocksize >= len` | `arena.rs` (2000 random + 255 `block` values, never fires in either library) | [x] |
| 28 | `stbds_stralloc` (914) | caller-supplied arena with `storage == NULL` **and** `remaining >= len` | `a->storage->storage` → **NULL deref → SIGSEGV** | `errors.rs::e28_stralloc_null_storage` (forked child) | [x] |
| 29 | `stbds_strreset` (924) | `a->storage == NULL` (fresh / already-reset arena) | loop body never runs; arena zeroed | `arena.rs::strreset_empty` | [x] |
| 30 | `sh_puts` (959/960/961) | three `STBDS_ASSERT`s on the single arena-mode entry | `SIGABRT` — invariants; hold for every `num` | `sh_puts.rs::sh_puts_matrix` / `::sh_puts_random` / `::sh_puts_repeated` (never fires; identical stdout) | [x] |
| 31 | `stbds_hash_bytes` | `p == NULL, len == 0` | no dereference; returns the "empty" siphash for the seed | `errors.rs::e31_hash_bytes_null_zero` | [x] |
| 32 | `stbds_hash_bytes` | `p == NULL, len > 0` | **SIGSEGV** | `errors.rs::e32_hash_bytes_null_nonzero` (forked child) | [x] |
| 33 | `stbds_hash_string` | `str == NULL` | **SIGSEGV** | `errors.rs::e33_hash_string_null` (forked child) | [x] |
| 34 | `stbds_arrfreef` | `a == NULL` → `free((char*)NULL - 32)` | glibc "free(): invalid pointer" → **SIGABRT** | `errors.rs::e34_arrfreef_null` (forked child) | [x] |
| 35 | `stbds_stralloc` | `a == NULL` | **SIGSEGV** (`strlen` runs first, then `a->remaining`) | `errors.rs::e35_stralloc_null_arena` (forked child) | [x] |
| 36 | `stbds_strreset` | `a == NULL` | **SIGSEGV** | `errors.rs::e36_strreset_null` (forked child) | [x] |
| 37 | out-of-range enum: `mode` for `hmput_key`/`hmget_key`/`hmget_key_ts` | `mode` is `int`; the test is `mode >= STBDS_HM_STRING(1)`.  Values `2, 3, 7, INT_MAX` ⇒ **string** mode; `-1, INT_MIN` ⇒ **binary** mode | no rejection — silently classified by `>= 1` | `enums.rs::mode_out_of_range_put_get` | [x] |
| 38 | out-of-range enum: `mode` for `hmdel_key` | `hmdel_key` uses **`mode == STBDS_HM_STRING`** (exact `== 1`) for the strdup-free and for the post-`memmove` key re-read, but `find_slot`/`is_key_equal` use `mode >= 1`.  So `mode == 2` takes the *string* hash path and the *binary* re-find path | no rejection — divergent internal branch, must be reproduced | `enums.rs::hmdel_mode_two` | [x] |
| 39 | out-of-range enum: `mode` for `stbds_shmode_func` | `(unsigned char) mode` is stored in `string.mode`; `switch` in `hmput_key` has a `default:` (raw `memcpy`).  `mode = 0,4,5,255,256,-1,INT_MIN,INT_MAX` are all accepted | no rejection — `mode & 0xff` selects the branch, `default` ⇒ `memcpy(keysize)` | `enums.rs::shmode_out_of_range` | [x] |
| 40 | zero length: `keysize == 0` in `hmput_key`/`hmget_key`/`hmdel_key` (binary mode) | `hash_bytes(key,0,seed)` is key-independent and `memcmp(...,0) == 0` ⇒ **every** key matches the first one | 2nd and later distinct keys are treated as duplicates; map never grows past 1 entry | `errors.rs::e40_keysize_zero` | [x] |
| 41 | one past valid range: `stbds_shmode_func` `elemsize` smaller than the key it will store | `elemsize = 8`, then string insert writes an 8-byte `char*` at `a+8*1` — exactly in range; `elemsize = 4` would overflow | `elemsize = 8` is the smallest safe value and is accepted | `enums.rs::shmode_out_of_range` (elemsize 8) | [x] |
| 42 | `stbds_hmget_key_ts` `temp == NULL` | unconditional `*temp = …` | **SIGSEGV** | `errors.rs::e42_hmget_ts_null_temp` (forked child) | [x] |

## Notes

### `STBDS_ASSERT` message parity (resolved)

`STBDS_ASSERT` expands to glibc `assert`, whose message embeds `__FILE__`.  CMake
compiles the C translation unit with an **absolute** path, so the C `.so`'s
`.rodata` holds `/…/translated_rust/c_src/src/lib.c`.  A Rust `cdylib` has no
`__FILE__` for the C source, so `build.rs` canonicalises `c_src/src/lib.c` at
build time and feeds the resulting absolute path to the assert wrapper via
`env!()`.  Row 23 fires a *reachable* assertion in both libraries and compares
stderr byte-for-byte; it passes, e.g.

```
<prog>: /…/translated_rust/c_src/src/lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.
```

for both.  (If `c_src/src/lib.c` is absent at Rust build time the wrapper falls
back to `"src/lib.c"`; the abort *behaviour* — `SIGABRT`, same assertion text,
function name and line number — is unaffected.)

### `mode != STBDS_HM_STRING` + `memmove` fix-up is address-dependent

`stbds_hmdel_key` with `mode >= 2` takes the *string* path in `find_slot` but the
*binary* path for the post-`memmove` re-find, which hands `&elem.key` — the
address of the pointer field — to `find_slot`, where `mode >= 1` makes
`stbds_hash_string` hash **the raw pointer bytes as text**.  The result therefore
depends on heap addresses, so it is not reproducible even for the C library
against itself unless the stored key pointers are identical.
`enums.rs::hmdel_mode_two` pins that down by using `STBDS_SH_DEFAULT`, where the
key pointers are the *caller's* buffers and hence bit-identical in both
libraries; both then abort with the same `slot >= 0` message (verified by
`enums.rs::diag_mode_two_aborts`).

### Rust `debug` profile: UB checks turn `SIGSEGV` into `SIGABRT`

Rows 28, 32, 33, 35, 36 and 42 dereference a null pointer, which is *undefined
behaviour* in the C.  The **release** cdylib — the shipped artefact — segfaults
exactly like the C.  A **debug** cdylib additionally carries rustc's
`-C debug-assertions` UB checks, which detect the null dereference and `panic!`
*before* the load; because the panic escapes an `extern "C"` function it is
converted to `SIGABRT` instead of `SIGSEGV`.  That is a property of the Rust
debug profile, not a behavioural difference of the translation, so
`common::assert_fatal_equivalent` accepts `SIGSEGV` (C) vs `SIGABRT` (Rust
debug) while still requiring both to die on the same input — and requiring an
exact signal match for the release build.  Everything else in this table matches
exactly under **both** profiles.
