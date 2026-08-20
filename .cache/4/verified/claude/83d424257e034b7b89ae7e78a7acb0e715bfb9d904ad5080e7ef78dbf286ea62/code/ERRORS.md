# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived **mechanically** from the C source: every `return` that reports "no
result", every `STBDS_ASSERT` (`assert`, live because the CMake build defines no
`NDEBUG`), every explicit null check, every range/threshold comparison, and
every min/max constant. One row per distinct rejection.

Sentinels used by this library: `-1` = `STBDS_INDEX_EMPTY` (key absent),
`-2` = `STBDS_INDEX_DELETED`, `0`/`NULL` = "nothing to do", `hash 0` =
`STBDS_HASH_EMPTY`, `hash 1` = `STBDS_HASH_DELETED`.
Constants: `STBDS_BUCKET_LENGTH 8`, `STBDS_CACHE_LINE_SIZE 64`,
`STBDS_STRING_ARENA_BLOCKSIZE_MIN 512`, `STBDS_STRING_ARENA_BLOCKSIZE_MAX 1<<20`,
`STBDS_SIPHASH_C_ROUNDS 2`, `STBDS_SIPHASH_D_ROUNDS 4`.

Legend for the last column: `[x]` = a differential test exists and passes
(`tests/errors.rs`, unless stated).

| # | function (C line) | trigger (exact invalid input / condition) | expected C result | [ ] |
|---|-------------------|-------------------------------------------|-------------------|-----|
| 1 | `stbds_arrgrowf` (286-287) | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap` — e.g. `a=NULL, elemsize=8, addlen=0, min_cap=0` | early `return a` (⇒ `NULL`), **no allocation** | [x] |
| 2 | `stbds_arrgrowf` (286-287) | existing array, `min_cap` ≤ current capacity (e.g. cap 4, `min_cap=4`, `addlen=0`) | returns the *same* pointer, header untouched | [x] |
| 3 | `stbds_arrgrowf` (300-303) | `a == NULL` (fresh alloc) | `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_len,min_cap,4 or 2*cap)` | [x] |
| 4 | `stbds_arrgrowf` (289-292) | `min_cap < 2*arrcap` ⇒ `min_cap = 2*arrcap`; else `min_cap < 4` ⇒ `min_cap = 4` | capacity growth rule (boundary `min_cap` = 3 / 4 / 5) | [x] |
| 5 | `stbds_arrfreef` (312-315) | `a == NULL` — **no null check in C**: `free((char*)NULL - 32)` | fatal: process killed by a signal (glibc "invalid pointer") — identical in both libs | [x] (subprocess) |
| 6 | `stbds_make_hash_index` (401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)`; fires for `slot_count == 0` | `SIGABRT` | unreachable — `static`, and every caller passes `8` or `2^k, k≥3`. Documented, not testable through the `.so` |
| 7 | `stbds_hash_bytes` (522-541) | `len == 0` (`p` may even be `NULL` — no byte is dereferenced) | well-defined hash of the length-only `data = 0<<56` | [x] |
| 8 | `stbds_hash_bytes` | `p == NULL, len > 0` | `SIGSEGV` in both | [x] (subprocess) |
| 9 | `stbds_hash_string` (480) | `str == NULL` — dereferenced immediately | `SIGSEGV` in both | [x] (subprocess) |
| 10 | `stbds_hash_string` (480) | `str == ""` (empty) | loop skipped; deterministic hash of `seed` alone | [x] |
| 11 | `stbds_is_key_equal` (560) | `mode` out of the documented enum: any `mode >= 1` ⇒ `strcmp` path, any `mode <= 0` (incl. `-1`, `INT_MIN`) ⇒ `memcmp` path | no rejection: C enums accept any `int`; classification is by `>= STBDS_HM_STRING` | [x] |
| 12 | `stbds_hmfree_func` (573) | `a == NULL` | explicit `return`, no-op | [x] |
| 13 | `stbds_hmfree_func` (574) | `stbds_hash_table(a) == NULL` (raw array with no index) | skips key/arena cleanup, frees `hash_table` (NULL) + header | [x] |
| 13b | `stbds_hmfree_func` (575-580) | table present: keys `free`d only when `string.mode == STBDS_SH_STRDUP`, and the loop starts at `i = 1` so the zeroed default element is skipped; then `stbds_strreset` releases the whole arena block chain | omitting either is invisible in the data structures (everything is freed right after), so it is detected with a glibc tcache-LIFO probe | [x] (`e13b`) |
| 14 | `stbds_hm_find_slot` (609-610) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` in the *forward* scan ⇒ key absent | `return -1` | [x] |
| 15 | `stbds_hm_find_slot` (620-621) | probe hits `STBDS_HASH_EMPTY` in the *wrap-around* scan ⇒ key absent | `return -1` | [x] |
| 16 | `stbds_hmget_key_ts` (634-639) | `a == NULL` | allocates 1 zeroed default element, `*temp = -1`, returns `ARR_TO_HASH` | [x] |
| 17 | `stbds_hmget_key_ts` (644-645) | array exists but `hash_table == 0` (e.g. straight from `arrgrowf`+`ARR_TO_HASH`, or after `hmput_default` only) | `*temp = -1`, returns `a` unchanged | [x] |
| 18 | `stbds_hmget_key_ts` (648-649) | key absent (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | [x] |
| 19 | `stbds_hmget_key_ts` (638/645/649) | `temp == NULL` — written unconditionally, no null check | `SIGSEGV` in both | [x] (subprocess) |
| 20 | `stbds_hmget_key` (662-664) | all of 16/17/18, reported via `stbds_header(p-elemsize)->temp` | `temp == -1` | [x] |
| 21 | `stbds_hmput_default` (669) | `a == NULL` | allocate 1 zeroed element, `length=1` | [x] |
| 22 | `stbds_hmput_default` (669) | `a != NULL` but `stbds_header(HASH_TO_ARR(a))->length == 0` | grow, `length += 1`, zero element 0 | [x] |
| 23 | `stbds_hmput_default` (675) | `a != NULL` and `length != 0` | `return a` **unchanged** (idempotent, does *not* reset the default) | [x] |
| 24 | `stbds_hmput_key` (686-691) | `a == NULL` | creates the raw array + zeroed default element before anything else | [x] |
| 25 | `stbds_hmput_key` (698-702) | `table == NULL` | new index with `slot_count = STBDS_BUCKET_LENGTH = 8` | [x] |
| 26 | `stbds_hmput_key` (698-702) | `used_count >= used_count_threshold` (`slot_count - slot_count/4`; 6 for 8, 12 for 16 …) | rehash into `slot_count*2` | [x] |
| 27 | `stbds_hmput_key` (707) | fresh table, `mode` out of range: `mode >= 1` ⇒ `string.mode = STBDS_SH_DEFAULT(1)`, else `0` | classification by `>= 1`, not by enum membership | [x] |
| 28 | `stbds_hmput_key` (719) / `stbds_hm_find_slot` (596) | computed `hash < 2` (collides with `HASH_EMPTY`/`HASH_DELETED`) | `hash += 2` fixup, applied identically on put and get | **dead code in practice** — see below |
| 29 | `stbds_hmput_key` (766-769) | an `STBDS_INDEX_DELETED` tombstone was seen before the empty slot | insert at the tombstone, `--tombstone_count` | [x] |
| 30 | `stbds_hmput_key` (778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | `SIGABRT` | unreachable — guaranteed by the `if` at 774. Documented |
| 31 | `stbds_hmput_key` (785-790) | `table->string.mode` not in {1,2,3} (`STBDS_SH_NONE`=0 or an out-of-range value such as 4 / 44 / 255) | `default:` ⇒ `memcpy(elem, key, keysize)` — the key is copied by value, *not* by pointer | [x] |
| 32 | `stbds_shmode_func` (803) | `mode` out of the enum: truncated by `(unsigned char) mode` ⇒ `256→0`, `300→44`, `-1→255`, `4→4` | accepted; changes which branch of row 31 is taken later | [x] |
| 33 | `stbds_hmdel_key` (809-810) | `a == NULL` | `return 0` (⇒ `NULL`) | [x] |
| 34 | `stbds_hmdel_key` (815-817) | `hash_table == 0` | `stbds_temp(raw_a) = 0`; `return a` unchanged | [x] |
| 35 | `stbds_hmdel_key` (821-822) | key absent (`slot < 0`) | `stbds_temp(raw_a) = 0`; `return a`, `length` unchanged | [x] |
| 36 | `stbds_hmdel_key` (828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | `SIGABRT` | unreachable — `find_slot` masks `pos` with `slot_count-1`. Documented |
| 37 | `stbds_hmdel_key` (832) | `STBDS_ASSERT(table->used_count >= 0)` on a **`size_t`** | the comparison is *always true*: it can never fire, even after `--used_count` wraps to `SIZE_MAX` | [x] (wrap reproduced: delete on a table whose `used_count` is 0) |
| 38 | `stbds_hmdel_key` (836-837) | `mode == STBDS_HM_STRING` **exactly** *and* `string.mode == STBDS_SH_STRDUP` ⇒ frees the key. `mode = 2/3/999` hash as strings yet **skip** the free | the difference is a leak, not a state change, so it is detected with a glibc tcache-LIFO probe: `malloc` of the key's size class returns the key's own address iff it was just freed | [x] (`e38`, checked for `mode` = 1/2/3/999) |
| 39 | `stbds_hmdel_key` (839) | `old_index == final_index` (deleting the last element) | no back-fill `memmove`, no re-find, no second assert | [x] |
| 40 | `stbds_hmdel_key` (842-845) | `mode == STBDS_HM_STRING` uses `*(char**)`; **every other mode** (incl. the string-hashing 2/3/999) uses the *raw address* of the key field | different key is hashed on the back-fill re-find | [x] |
| 41 | `stbds_hmdel_key` (846) | `STBDS_ASSERT(slot >= 0)` — **reachable**: `mode > 1` + `old_index != final_index` (row 40 hashes the pointer bytes, which do not match any stored hash) | `SIGABRT` | [x] (subprocess; deterministic because the test hands *both* libraries the same key pointers) |
| 42 | `stbds_hmdel_key` (849) | `STBDS_ASSERT(b->index[i] == final_index)` after a successful re-find | `SIGABRT` | unreachable for `mode <= 1`; for `mode > 1` covered by row 41's abort. Documented |
| 43 | `stbds_hmdel_key` (854) | `used_count < used_count_shrink_threshold` **and** `slot_count > 8` (shrink threshold is forced to `0` when `slot_count <= 8`, so an 8-slot table never shrinks) | rebuild index at `slot_count >> 1` | [x] |
| 44 | `stbds_hmdel_key` (858) | `tombstone_count > tombstone_count_threshold` (`slot_count/8 + slot_count/16`; `1` for 8 slots, `3` for 16 …) | rebuild index at the same `slot_count`, clearing tombstones | [x] |
| 45 | `stbds_hmdel_key` | `keyoffset` pointing outside the element (e.g. `keyoffset = elemsize`) | garbage compare ⇒ almost always row 35 (`slot < 0`, no-op) — must match bit-for-bit | [x] |
| 46 | `stbds_stralloc` (885) | `len > a->remaining` | allocate a new block | [x] |
| 47 | `stbds_stralloc` (888) | `blocksize = 512 << (a->block >> 1)`; `a->block` is `unsigned char` so the shift count can reach 127 ⇒ C UB, x86-64 `shl` masks it to 6 bits | Rust must mirror the masked shift (`wrapping_shl`) | [x] |
| 48 | `stbds_stralloc` (890-891) | `blocksize < 1<<20` ⇒ `++a->block`; at `block >= 22` the block size saturates and `block` stops growing | max block size clamp | [x] |
| 49 | `stbds_stralloc` (893-904) | `len > blocksize` **and** `a->storage != NULL` | oversize block spliced *after* the head (`sb->next = storage->next; storage->next = sb`), `remaining` unchanged, returns `sb->storage` | [x] |
| 50 | `stbds_stralloc` (893-904) | `len > blocksize` **and** `a->storage == NULL` | oversize block becomes the head, `remaining = 0`, returns `sb->storage` | [x] |
| 51 | `stbds_stralloc` (913) | `STBDS_ASSERT(len <= a->remaining)` | `SIGABRT` | unreachable through the documented flow (the branches above always leave `remaining >= len`); only a caller-corrupted arena reaches it, and that segfaults first. Documented |
| 52 | `stbds_stralloc` | `a == NULL` or `str == NULL` | `SIGSEGV` in both | [x] (subprocess) |
| 53 | `stbds_strreset` (923-928) | `a->storage == NULL` (zeroed / already-reset arena) | frees nothing, just `memset(a,0,24)` | [x] |
| 54 | `stbds_strreset` | `a == NULL` | `SIGSEGV` in both | [x] (subprocess) |
| 55 | `hm_geti` (952,954,955,959-962,967,968,972,973,977) | 12 `STBDS_ASSERT`s over the whole int-map lifecycle | none may fire for **any** `int num` (incl. `0`, negatives, `INT_MIN`) ⇒ normal return | [x] (`tests/hm_geti.rs`) |
| 56 | `strkey` (939-943) | unbounded `sprintf` into `static char buffer[256]`; also `n = INT_MIN` (11 chars) | no overflow possible; returns the *same* static buffer each call (previous result is clobbered) | [x] |
| 57 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` (563, 789) | `keysize == 0`: `memcmp(...,0) == 0` makes **every** key compare equal and `hash_bytes(k,0,seed)` makes every key hash the same | the map collapses to a single entry; fully defined, no rejection | [x] (`e57`) |
| 58 | all `hm*` entry points | `elemsize == 0`: every element aliases the same zero-sized address and `memcpy(...,keysize=0)` writes nothing | fully defined degenerate map; `length`/`capacity` still grow | [x] (`e58`) |
| 59 | `stbds_arrgrowf` (297-299) | oversized request: `elemsize * min_cap + 32` where `realloc` fails ⇒ `b = NULL + 32` and the header write lands on address 0 | `SIGSEGV` in both (no allocation-failure check anywhere in the C) | [x] (subprocess, `e01b`) |

## Notes on "unreachable" / vacuous rows

Rows 6, 30, 36, 42, 51 guard *internal invariants* of `static` helpers. They
cannot be triggered through any exported symbol with any argument values, so
there is no input a differential test could construct. They are listed for
completeness and the Rust translation reproduces each check verbatim
(`STBDS_ASSERT!` → `eprintln!` + `abort()`, i.e. the same `SIGABRT`).

Row 37 is the opposite case: the C check exists but is *vacuous* because
`used_count` is unsigned. The Rust translation must therefore **not** turn it
into a real check — and it does not (the assert is dropped with a comment, and
`used_count` is decremented with `wrapping_sub`, matching the C wrap-around).
`e37` forces `used_count` to wrap to `SIZE_MAX` and asserts both libraries keep
running and agree.

Row 28 (`if (hash < 2) hash += 2;`) is **dead code**: it only triggers when a
64-bit siphash / string hash comes out exactly `0` or `1`, i.e. with probability
`2^-63` per lookup. `row10_hash_string_and_bytes_reserved_hash_values` computes
400 000 hashes over random keys and seeds and reports `0` values below 2, and
`mutation_check.sh` confirms that removing the fixup from either the put side
(`src/lib.rs:849`) or the get side (`src/lib.rs:666`) cannot be observed through
the `.so`. The Rust reproduces both sites verbatim
(`if hash < 2 { hash = hash.wrapping_add(2); }` vs. C's
`if (hash < 2) hash += 2;` at `lib.c:596` and `lib.c:719`), which is the strongest
statement that can be made about an unobservable branch.

## Findings that changed the translation / the harness

1. **`stbds_hash_index::temp_key` is never initialised.**
   `stbds_make_hash_index` assigns every field of the freshly `realloc`'d struct
   *except* `temp_key`, so its value is indeterminate until a string-mode
   `stbds_hmput_key` writes it. It is therefore excluded from the state snapshots
   and compared only where the C actually defines it (right after a put, by
   contents rather than by address).
2. **rustc's debug assertions changed C's `SIGSEGV` into `SIGABRT`.**
   With `debug-assertions = on` (the cargo `dev` default) rustc injects null- and
   alignment-checks around raw-pointer dereferences, so every null-pointer row
   above (5, 8, 9, 19, 52, 54, 59) died with `SIGABRT` in the Rust `.so` while the
   C `.so` died with `SIGSEGV`. `[profile.dev] debug-assertions = false` +
   `overflow-checks = false` in `Cargo.toml` makes the `dev` artifact behave like
   the `release` one (which already matched C), so all seven rows now agree.
3. **`cargo test` does not rebuild a `cdylib`-only lib target**, so the `.so`
   under test is easily stale. `tests/common/mod.rs` now refuses to run when
   `target/<profile>/libhm_geti_lib.so` is older than `src/lib.rs`. (This was
   found by `mutation_check.sh`: the first run reported 0 of 16 mutations caught,
   because the mutated source never reached the `.so`.)
4. **Leaks and pointer arithmetic need non-state observations.** Rows 13b and 38
   are pure allocator behaviour, and `hash_index::storage`'s alignment plus the
   two `realloc` sizes are pure address arithmetic — none of them changes any
   comparable field. They are covered by a glibc tcache-LIFO `malloc` probe and by
   `malloc_usable_size` / `STBDS_ALIGN_FWD` assertions applied to both libraries.

## Mutation evidence

`./mutation_check.sh` injects 33 single-line deviations from the C semantics into
`src/lib.rs`, rebuilds the `.so` and re-runs the whole suite for each:

```
mutations killed:              30 / 33
proven equivalent (expected):   3 / 33
real coverage gaps:             0
```

The three survivors are proven equivalent mutants, not blind spots:

| mutation | why no input can distinguish it |
|----------|---------------------------------|
| `find_slot`: `hash < 1` instead of `hash < 2` | needs a raw hash of exactly `1` (`p = 2^-64`) |
| `hmput_key`: fixup removed entirely | needs a raw hash of `0` or `1` (`p = 2^-63`) |
| `arrgrowf`: `min_cap <= 2*cap` instead of `<` | the only extra case is `min_cap == 2*cap`, whose body assigns `min_cap = 2*cap` — a no-op |
