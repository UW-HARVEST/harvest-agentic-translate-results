# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. Every early `return`, every
`STBDS_ASSERT`, every null check, every sentinel value and every range/boundary
constant the C code actually tests is one row.

`STBDS_ASSERT` is `assert` (`lib.c:3`). The CMake project sets no
`CMAKE_BUILD_TYPE` and no `-DNDEBUG`, so **asserts are live in the C `.so`**
(`nm -D --undefined-only` on it shows `U __assert_fail@GLIBC_2.2.5`), and a
firing assert prints to stderr and raises **SIGABRT**.

**All seven asserts are therefore translated**, routed through glibc's
`__assert_fail` with the same assertion text, function name and line number, so
a failing assert produces the same message body and the same SIGABRT. (An
earlier revision of the Rust omitted them; `stbds_hmdel_key`'s
`assert(slot >= 0)` at `lib.c:846` turned out to be **reachable** — see row 37 —
where the C aborted and the Rust silently performed an out-of-bounds write.)

Sentinels used by the library:
`STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`.

| # | function | trigger (exact invalid input / condition) | expected C result | status |
|---|----------|-------------------------------------------|-------------------|--------|
| 1 | `stbds_arrgrowf` (`lib.c:286`) | `min_cap <= stbds_arrcap(a)` — requested capacity already satisfied | returns `a` unchanged (identical pointer), no realloc, header untouched | [x] |
| 2 | `stbds_arrgrowf` (`lib.c:283`) | `stbds_arrlen(a)+addlen > min_cap` — `min_cap` too small for the request | `min_cap` silently raised to `min_len`; never under-allocates | [x] |
| 3 | `stbds_arrgrowf` (`lib.c:289`) | `min_cap < 2*arrcap(a)` — growth request smaller than doubling | `min_cap` raised to `2*arrcap(a)` | [x] |
| 4 | `stbds_arrgrowf` (`lib.c:291`) | `min_cap < 4` on a fresh (`a == NULL`) array | `min_cap` raised to 4 | [x] |
| 5 | `stbds_arrgrowf` (`lib.c:300`) | `a == NULL` (no existing header to inherit) | fresh header: `length=0`, `hash_table=NULL`, `temp=0`, `capacity=min_cap` | [x] |
| 6 | `stbds_arrgrowf` | `elemsize == 0` | allocates only the header; returns valid non-NULL pointer, `capacity=min_cap` | [x] |
| 7 | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0 && a == NULL` → `min_len = 0`, `min_cap = 0`; `0 <= arrcap(NULL)==0` | **returns NULL** (the `min_cap <= arrcap` early-out fires) | [x] |
| 8 | `stbds_hmfree_func` (`lib.c:573`) | `a == NULL` | returns immediately, no free, no crash | [x] |
| 9 | `stbds_hmfree_func` (`lib.c:574`) | `stbds_hash_table(a) == NULL` (array never had a hash index) | skips strdup-scan and `strreset`; still frees `hash_table` (NULL) and header | [x] |
| 10 | `stbds_hmfree_func` (`lib.c:575`) | `string.mode != STBDS_SH_STRDUP` | does **not** free per-entry key pointers | [x] |
| 11 | `stbds_hmget_key_ts` (`lib.c:634`) | `a == NULL` | allocates a 1-element array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL `arr+elemsize` | [x] |
| 12 | `stbds_hmget_key_ts` (`lib.c:644`) | `a != NULL` but `hash_table == 0` (e.g. only `hmput_default` was called) | `*temp = -1`, returns `a` unchanged | [x] |
| 13 | `stbds_hmget_key_ts` (`lib.c:648`) | key absent → `stbds_hm_find_slot` returns `< 0` | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | [x] |
| 14 | `stbds_hmget_key` (`lib.c:663`) | any of rows 11–13 | same as `_ts` **and** `stbds_temp(arr) == -1` is observable in the header | [x] |
| 15 | `stbds_hm_find_slot` (`lib.c:609`,`620`) | probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` | returns `-1` (miss) — never loops forever on a non-full table | [x] |
| 16 | `stbds_hm_find_slot` (`lib.c:596`) | computed hash `< 2` (would collide with EMPTY/DELETED) | `hash += 2` before probing; lookup still succeeds | [x] |
| 17 | `stbds_hmput_key` (`lib.c:719`) | computed hash `< 2` | `hash += 2` before probing | [x] |
| 18 | `stbds_hmput_key` (`lib.c:686`) | `a == NULL` | bootstraps a 1-element zeroed array before inserting | [x] |
| 19 | `stbds_hmput_key` (`lib.c:698`) | `table == NULL` | fresh index of `STBDS_BUCKET_LENGTH (8)` slots; `string.mode` set to `SH_DEFAULT` iff `mode >= STBDS_HM_STRING` else `0` | [x] |
| 20 | `stbds_hmput_key` (`lib.c:698`) | `used_count >= used_count_threshold` (`= slot_count - slot_count/4`; 6 for 8 slots) | table doubled and rehashed; old table freed | [x] |
| 21 | `stbds_hmput_key` (`lib.c:730`) | duplicate key | no new slot, `used_count` unchanged, `stbds_temp` = existing index, array length unchanged | [x] |
| 22 | `stbds_hmput_key` (`lib.c:766`) | a tombstone was seen before the empty slot | reuses the tombstone, `--tombstone_count` | [x] |
| 23 | `stbds_hmput_key` (`lib.c:778`) | `assert((size_t)i+1 <= stbds_arrcap(a))` | assert-abort; unreachable — the preceding `arrgrowf` guarantees it. Translated anyway | [x] (translated; unreachable) |
| 24 | `stbds_hmput_key` (`lib.c:789`) | `table->string.mode` is not one of `SH_STRDUP/SH_ARENA/SH_DEFAULT` (i.e. `SH_NONE=0` or any out-of-range value from `shmode_func`) | `default:` branch — raw `memcpy` of `keysize` bytes, key pointer **not** stored | [x] |
| 25 | `stbds_hmput_key` / `stbds_hmget_key*` / `stbds_hmdel_key` (`lib.c:560`,`590`,`713`) | out-of-range `mode` int: any `mode >= 1` (e.g. 2, 7, 1000, `INT_MAX`) | treated as **string** mode (`mode >= STBDS_HM_STRING`) | [x] |
| 26 | same as 25 | negative `mode` (e.g. `-1`, `INT_MIN`) | treated as **binary** mode (`memcmp`/`hash_bytes`) | [x] |
| 27 | `stbds_hmdel_key` (`lib.c:836`,`842`) | `mode == STBDS_HM_STRING` exactly (`== 1`, not `>= 1`) gates the strdup-free and the re-find-by-string-pointer | `mode == 2` in a string-keyed map takes the **binary** re-find branch | [x] |
| 28 | `stbds_hmput_default` (`lib.c:669`) | `a == NULL` | allocates 1-element zeroed array, returns `arr+elemsize` (non-NULL) | [x] |
| 29 | `stbds_hmput_default` (`lib.c:669`) | `a != NULL` but `header(a-elemsize)->length == 0` | re-grows and bumps length to 1 | [x] |
| 30 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (identical pointer) | [x] |
| 31 | `stbds_hmdel_key` (`lib.c:809`) | `a == NULL` | **returns `NULL`** (`return 0`) | [x] |
| 32 | `stbds_hmdel_key` (`lib.c:816`) | `hash_table == 0` | returns `a`, and `stbds_temp(arr) == 0` (delete reported as "not found") | [x] |
| 33 | `stbds_hmdel_key` (`lib.c:821`) | key absent → `find_slot < 0` | returns `a`, `stbds_temp(arr) == 0`, length unchanged | [x] |
| 34 | `stbds_hmdel_key` (`lib.c:831`) | key present | `stbds_temp(arr) == 1`, `--used_count`, `++tombstone_count`, `hash[i]=HASH_DELETED(1)`, `index[i]=INDEX_DELETED(-2)`, `length -= 1` | [x] |
| 35 | `stbds_hmdel_key` (`lib.c:828`) | `assert(slot < (ptrdiff_t) table->slot_count)` | assert-abort; unreachable — `find_slot` masks `pos` with `slot_count-1`. Translated anyway | [x] (translated; unreachable) |
| 36 | `stbds_hmdel_key` (`lib.c:832`) | `assert(table->used_count >= 0)` | `used_count` is `size_t`, so the comparison is a tautology — never fires. Translated anyway | [x] (translated; tautology) |
| 37 | `stbds_hmdel_key` (`lib.c:846`) | `assert(slot >= 0)` after re-finding the moved last element. **REACHABLE**: `mode == 2` makes `lib.c:842`'s `mode == STBDS_HM_STRING` false, so the re-find at `lib.c:845` passes the element's raw bytes as the key while `stbds_hm_find_slot` still hashes them as a *string* (`mode >= STBDS_HM_STRING` is true) → miss → `slot == -1`. Requires deleting a NON-last element | **SIGABRT** + `lib.c:846: stbds_hmdel_key: Assertion \`slot >= 0' failed.` | [x] |
| 38 | `stbds_hmdel_key` (`lib.c:849`) | `assert(b->index[i] == final_index)` | assert-abort; unreachable in-invariant. Translated anyway | [x] (translated; unreachable) |
| 39 | `stbds_hmdel_key` (`lib.c:839`) | deleted element **is** the last one (`old_index == final_index`) | no `memmove`, no re-find, no index fix-up | [x] |
| 40 | `stbds_hmdel_key` (`lib.c:854`) | `used_count < used_count_shrink_threshold (slot_count/4)` **and** `slot_count > 8` | table halved + rehashed, old freed | [x] |
| 41 | `stbds_hmdel_key` (`lib.c:854`) | shrink condition true but `slot_count == 8` | no shrink (`used_count_shrink_threshold` forced to 0 at `lib.c:399`) | [x] |
| 42 | `stbds_hmdel_key` (`lib.c:858`) | `tombstone_count > tombstone_count_threshold ((slot_count>>3)+(slot_count>>4))` | table rebuilt at the **same** size, old freed | [x] |
| 43 | `stbds_make_hash_index` (`lib.c:399`) | `slot_count <= STBDS_BUCKET_LENGTH (8)` | `used_count_shrink_threshold` forced to `0` | [x] |
| 44 | `stbds_make_hash_index` (`lib.c:401`) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` — fires for `slot_count <= 2` | assert-abort; unreachable — `slot_count` is always `8`, `2*n`, or `n>>1` guarded by `n > 8`. Translated anyway | [x] (translated; unreachable) |
| 45 | `stbds_make_hash_index` (`lib.c:403`) | `ot == NULL` | `string` zeroed, `seed = stbds_hash_seed`, and the **global seed is advanced** `seed = seed*a + b` | [x] |
| 46 | `stbds_make_hash_index` (`lib.c:426`) | `ot != NULL` | `string` and `seed` inherited, global seed **not** advanced, entries rehashed | [x] |
| 47 | `stbds_stralloc` (`lib.c:885`) | `len > a->remaining` | new block allocated; `a->block` incremented while `blocksize < 1<<20` | [x] |
| 48 | `stbds_stralloc` (`lib.c:890`) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` **not** incremented (saturates) | [x] |
| 49 | `stbds_stralloc` (`lib.c:893`) | `len > blocksize` (oversized string) | dedicated block; spliced as `a->storage->next` if storage exists, else becomes storage with `remaining = 0`; returns `sb->storage` directly | [x] |
| 50 | `stbds_stralloc` (`lib.c:913`) | `assert(len <= a->remaining)` | assert-abort; unreachable — every path above either returns early or sets `remaining >= len`. Translated anyway | [x] (translated; unreachable) |
| 51 | `stbds_stralloc` | empty string `""` → `len == 1` | allocates 1 byte from the arena, returns a pointer to `'\0'` | [x] |
| 52 | `stbds_strreset` (`lib.c:924`) | `a->storage == NULL` (already-empty / zeroed arena) | loop body never runs; arena memset to 0; idempotent | [x] |
| 53 | `stbds_hash_string` (`lib.c:480`) | empty string `""` | loop skipped; still returns a well-defined avalanche of `seed` | [x] |
| 54 | `stbds_hash_string` (`lib.c:481`) | bytes with the high bit set (`0x80..0xFF`) | added as **`unsigned char`** (no sign extension) | [x] |
| 55 | `stbds_hash_bytes` (`lib.c:522`) | `len == 0` (and `p` unread) | tail `switch (0)` → `case 0: break`; returns hash of the length-only word | [x] |
| 56 | `stbds_hash_bytes` (`lib.c:532`) | `len % 8 == 1..7` — each of the 7 fall-through `case` labels | byte-wise fall-through accumulation into `data` | [x] |
| 57 | `stbds_hash_bytes` (`lib.c:523`) | tail/body byte with `d[3] >= 0x80` | `d[3] << 24` overflows `int` → negative → **sign-extended** into the whole upper half of `size_t` | [x] |
| 58 | `stbds_hash_bytes` (`lib.c:531`) | any `len` | `data` seeded with `len << 56`, so only `len & 0xFF` influences the top byte (`len` 1 vs 257 differ elsewhere via the body) | [x] |
| 59 | `stbds_shmode_func` (`lib.c:803`) | `mode` outside `0..3` (e.g. `4`, `-1`, `256`, `1000`) | truncated by `(unsigned char) mode`; `256 -> 0`, `-1 -> 255`, `1000 -> 232`; later hits `hmput_key`'s `default:` memcpy branch | [x] |
| 60 | `stbds_shmode_func` (`lib.c:798`) | `elemsize == 0` | `arrgrowf(0,0,0,1)` still returns a header-only allocation; `ARR_TO_HASH` returns the array pointer itself | [x] |
| 61 | `stbds_arrfreef` (`lib.c:314`) | `a == NULL` | computes `NULL - sizeof(header)` and calls `free` on it → **undefined / crash in both**; not exercised | [x] (UB, not tested) |
| 62 | `strkey` (`lib.c:941`) | any `int`, incl. `INT_MIN`, `-1`, `0` | `sprintf(buffer,"test_%d",n)` into the shared 256-byte static; returns that same static pointer every call | [x] |
| 63 | `helxo` (`lib.c:945`) | any `char`, incl. `0`, `'\n'`, `0x7F`, `-1`/`0xFF` | 5 fixed lines; the 4th line's char is `letter` (the `"jen"` key already exists so only `.value` is overwritten) | [x] |
| 64 | `stbds_stralloc` (`lib.c:888`) | caller-supplied arena with `a->block >= 128`, so the shift count `a->block >> 1` is `>= 64` — formally UB in C | the C `.so` compiles it to `shlq %cl`, which **masks the count to its low 6 bits**, and returns normally (e.g. `block=128` → `blocksize=512`, `remaining=506`; `block=254` → `blocksize=0` → oversized-block path, `remaining=0`) | [x] |

## Notes on assert rows

All seven C asserts (rows 23, 35, 36, 37, 38, 44, 50) are **translated**, not
omitted, because the C `.so` is built without `-DNDEBUG`. Six of them are
unreachable while the caller respects the library's invariants, and the seventh
(row 37) is reachable and is covered by a process-level differential test.

## Divergences found and fixed

Two real divergences were found *after* the in-process suite was already green;
both needed an out-of-process harness (`tests/aborts.rs`) because one side
terminates the process.

**1. `stbds_stralloc` shift count (row 64).** `lib.c:888` shifts by
`a->block >> 1`, and `a->block` is an `unsigned char` the *caller* owns, so the
count can be up to 127. The Rust used a bare `<<`, which panics on shift
overflow in debug builds (`attempt to shift left with overflow`) and aborts,
whereas the C returns normally. Verified against the C `.so`: `block = 128`
gives `blocksize = 512` (count `64 & 63 == 0`) and `block = 254` gives
`blocksize = 0` (count 63). Fixed by masking the count with
`& (STBDS_SIZE_T_BITS - 1)`, matching the `shlq %cl` the C compiles to.
Release Rust was already accidentally correct (LLVM emits the same masked
shift), so this was a debug-only break — but a hard abort versus a normal
return.

**2. Missing `STBDS_ASSERT`s (row 37).** All seven asserts had been dropped.
Six are unreachable, but `assert(slot >= 0)` at `lib.c:846` is reachable with
`mode == 2`: `lib.c:842` tests `mode == STBDS_HM_STRING` (false) while
`stbds_hm_find_slot` tests `mode >= STBDS_HM_STRING` (true), so the re-find
hashes the element's raw bytes as a string, misses, and yields `slot == -1`.
The C aborts; the Rust computed `storage.offset(-1)` and wrote out of bounds.
Fixed by translating all seven asserts through `__assert_fail`.

## Known non-divergence: allocation failure

The C never checks any allocation result (`lib.c:297`, `388`, `873`, `894`,
`906`). When `realloc` returns NULL the C dereferences it, which happens to
SIGSEGV. Rust *release* reproduces that exactly (both die with signal 11), but
Rust *debug* inserts a null-pointer-dereference check and turns the same UB into
a SIGABRT with a diagnostic. This is only observable when an allocation actually
fails — i.e. only outside the C's defined behaviour — and it is the reason
`tests/aborts.rs` classifies each `block` value dynamically from the C's own
exit status instead of assuming a fixed outcome.

## Row → test mapping (all rows PASS)

| rows | test |
|------|------|
| 1–7 | `tests/errors.rs::err_rows1_7_arrgrowf` (+ `tests/lowlevel.rs::row07_err_arrgrowf_null_when_nothing_requested`) |
| 8–10 | `tests/errors.rs::err_rows8_10_hmfree_func_null_and_tableless` |
| 11–15 | `tests/errors.rs::err_rows11_15_lookup_sentinels` |
| 16, 17 | `tests/errors.rs::err_rows16_17_hash_below_two_guard` |
| 18–22 | `tests/errors.rs::err_rows18_22_hmput_key_paths` |
| 24 | `tests/hashmap.rs::rows52_56_shmode_out_of_range_and_none` |
| 25, 26 | `tests/errors.rs::err_rows25_26_mode_classification` |
| 27 | `tests/errors.rs::err_row27_hmdel_tests_mode_equality_not_ge` |
| 28–30 | `tests/errors.rs::err_rows28_30_hmput_default` |
| 31–34 | `tests/errors.rs::err_rows31_34_hmdel_sentinels` |
| 39–43 | `tests/errors.rs::err_rows39_43_delete_rehash_triggers` |
| 45, 46 | `tests/errors.rs::err_rows45_46_seed_advance_and_inheritance` |
| 47–52 | `tests/errors.rs::err_rows47_52_stralloc_boundaries` |
| 53–58 | `tests/errors.rs::err_rows53_58_hash_boundaries` |
| 59, 60 | `tests/errors.rs::err_rows59_60_shmode_func` |
| 62 | `tests/errors.rs::err_row62_strkey_extremes` |
| 63 | `tests/errors.rs::err_row63_helxo_every_char` (all 256 byte values, stdout byte-for-byte) |
| 37, 64 | `tests/aborts.rs::abort_row37_hmdel_mode2_assert_slot_ge_zero`, `abort_row64_stralloc_shift_count_masking` (out-of-process: compares child exit status, signal and stderr) |
| generic | `tests/errors.rs::err_generic_null_and_zero_boundaries` — NULL `a`, NULL `key`, `elemsize`/`keysize` 0, `keyoffset` 0/1/8, `mode` ∈ {-1, 0, 1, 2, 999} on `hmfree_func`, `hmdel_key`, `hmget_key_ts`, `hash_bytes` |

### How rows 16/17 were made testable

`stbds_hash_string` starts with `hash ^= seed` (`lib.c:483`), which for the
EMPTY string cancels the seed entirely; the result is therefore `K + seed` for a
seed-independent constant `K`. The test reads `K = hash_string("", 0)` out of
the library and then calls `stbds_rand_seed(0 - K)` / `stbds_rand_seed(1 - K)`,
so the next freshly created hash index has a seed for which the empty-string key
hashes to exactly `0` and exactly `1` — the two values the
`if (hash < 2) hash += 2` guard exists for. The test then asserts the key is
stored at slot 2 (resp. 3) with `hash[]` entry 2 (resp. 3) in **both**
libraries, and that a subsequent `hmget_key` and `hmdel_key` still find it. This
is the only reachable way to hit that branch; brute-forcing a 64-bit siphash
output below 2 is not feasible.

### Out-of-range enum values across the FFI boundary

`mode` is a plain `int` in C, so any `int` is a real input.
`err_rows25_26_mode_classification` builds the same map twice per library — once
with `mode = m` and once with the reference value (`1` for every `m >= 1`, `0`
for every `m < 0`) — for `m ∈ {0, 1, 2, 3, 7, 1000, INT_MAX, -1, -2, INT_MIN}`,
and asserts C == Rust *and* `m` == reference. `err_rows59_60_shmode_func` covers
`stbds_shmode_func`'s `(unsigned char)` truncation for
`mode ∈ {0,1,2,3,4,255,256,257,1000,-1,-256,INT_MAX,INT_MIN}`, asserting the
exact stored `string.mode` byte (e.g. `256 -> 0`, `1000 -> 232`, `-1 -> 255`).

### Rows intentionally NOT executed, with justification

* **Rows 23, 35, 36, 37, 38, 44, 50 (asserts).** Each is a `STBDS_ASSERT` that
  cannot fire for any input a caller can supply while the library's own
  invariants hold, so omitting it in Rust is behaviour-preserving. Row 36 is a
  tautology (`size_t >= 0`). Row 44's `slot_count` is always `8`, `2*n`, or
  `n>>1` guarded by `n > 8`, all of which satisfy the assertion. The surrounding
  branch of each is nonetheless driven to completion by
  `err_rows39_43_delete_rehash_triggers` and
  `rows37_38_41_43_tombstone_rebuild_and_shrink`.
* **Row 61 (`stbds_arrfreef(NULL)`).** The C computes `NULL - 32` and calls
  `free` on it — undefined behaviour that crashes; there is nothing to compare.
* **`stbds_strreset(NULL)` / `stbds_stralloc(NULL, ...)`.** The C dereferences
  `a` unconditionally (`lib.c:885`, `lib.c:923`); NULL is UB, not a checked
  error.
* **`keysize > elemsize` in `stbds_hmput_key`.** The C `memcpy`s `keysize` bytes
  into an `elemsize` slot, overrunning into the next element and possibly past
  `capacity`; that is a heap overflow, not a checked rejection.
