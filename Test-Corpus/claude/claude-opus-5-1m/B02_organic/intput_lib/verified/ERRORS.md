# ERRORS.md — error-surface table for `c_src/src/lib.c`

Derived mechanically by grepping the C translation unit for **every** rejection
path: `STBDS_ASSERT(...)`, `return -1`, `return 0`, `return a` (early / no-op
return), `return;`, every `== NULL` / `!= NULL` / `== 0` guard, every explicit
range or threshold comparison, and every min/max constant:

```sh
grep -n 'STBDS_ASSERT\|return -1\|return 0;\|return a;\|return;\|== NULL\|!= NULL\|== 0)' c_src/src/lib.c
```

## How this library signals rejection

There is **no** error enum and **no** `errno` use. Rejections take exactly four
forms:

| form | meaning |
|------|---------|
| `-1` written to `*temp` / the array header's `temp` (`STBDS_INDEX_EMPTY`) | "key not present" |
| `0` / `NULL` return | only `stbds_hmdel_key(NULL, …)` |
| unchanged-pointer return, no state change | the handle is handed straight back |
| `assert()` failure | glibc prints `<argv0>: <file>:<line>: <func>: Assertion \`<expr>' failed.` then `abort()` ⇒ **SIGABRT (6)** |

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `NDEBUG`, so **every
assert is live**. The Rust reproduces them by calling glibc `__assert_fail` with
the identical expression string, the identical `__FILE__`
(`$CARGO_MANIFEST_DIR/c_src/src/lib.c` — verified byte-equal against
`strings libtranslated_rust.so`), line number and function name.

`elemsize`, `keysize`, `keyoffset`, `addlen`, `min_cap`, `len` are all `size_t`
and `mode` is a plain `int`, so the C validates **nothing**: every row below is
an input the C really does accept and process, and the Rust must process it
identically.

## Reading the table

* **test** — the `#[test]` that covers the row. Test files:
  `tests/c_errors.rs` (fatal + core rejection rows) and `tests/c_errors2.rs`
  (non-fatal / edge-value rows).
* Rows whose expected result is fatal run the scenario in a **child process**
  (`crash_child_runner` re-executes the test binary) for both `.so`s and compare
  exit status **and** the glibc assert text.

## Error / rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `stbds_arrgrowf` (L286-287) | `min_cap <= stbds_arrcap(a)` after the `min_len` bump, e.g. `(NULL, 8, 0, 0)` | returns `a` **unchanged** (here `NULL`); no `realloc`, no header written | `err_01_arrgrowf_nogrow` | [x] |
| 2 | `stbds_arrgrowf` (L286-287) | non-NULL `a` with `capacity = 4` and `max(len+addlen, min_cap) <= 4` | returns the *same* pointer; `length`/`capacity`/`temp` untouched | `err_02_arrgrowf_nogrow_nonnull` | [x] |
| 3 | `stbds_arrfreef` (L312-315) | `a == NULL` ⇒ `free((stbds_array_header *) NULL - 1)` = `free((void *) -32)` | invalid free ⇒ glibc kills the process (**SIGSEGV 11**). The identical bogus address is handed to the identical libc `free` on both sides | `err_03_arrfreef_null` | [x] |
| 4 | `stbds_hmfree_func` (L573) | `a == NULL` | `return;` immediately — no `free`, no arena reset | `err_04_hmfree_null` | [x] |
| 5 | `stbds_hmfree_func` (L574) | map with `hash_table == NULL` (built only by `stbds_hmput_default`, or a raw `stbds_arrgrowf` array) | skips the STRDUP sweep and `stbds_strreset`; `free(NULL)` then `free(header)` | `err_05_hmfree_no_table` | [x] |
| 6 | `stbds_hm_find_slot` (L609-610) | key absent; the forward scan (`i = pos&7 … 7`) hits `hash[i] == STBDS_HASH_EMPTY` | `return -1` ⇒ `temp = -1` / delete no-op | `err_06_07_find_slot_miss_fwd_and_wrap` | [x] |
| 7 | `stbds_hm_find_slot` (L620-621) | key absent; slots `pos&7 … 7` are all occupied and non-matching, so the wrap-around scan (`i = 0 … pos&7`) hits `HASH_EMPTY`. Constructed by filling slots 4–7 of an 8-slot table and probing a missing key with `pos == 5` | `return -1` (the *second* return site) | `err_06_07_find_slot_miss_fwd_and_wrap` | [x] |
| 8 | `stbds_hmget_key_ts` (L634-639) | `a == NULL` | allocates `capacity = 4`, `length = 1`, element 0 zeroed, `*temp = -1`, returns a non-NULL hash-side pointer | `err_08_get_ts_null` | [x] |
| 9 | `stbds_hmget_key_ts` (L644-645) | `a != NULL` but `stbds_header(raw_a)->hash_table == 0` | `*temp = -1`; returns `a` unchanged; **no** table allocated | `err_09_get_ts_no_table` | [x] |
| 10 | `stbds_hmget_key_ts` (L648-649) | table present, key not in the map (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`; `length`, `used_count`, `tombstone_count` and every bucket unchanged | `err_10_get_ts_missing_key` | [x] |
| 11 | `stbds_hmget_key` (L663) | any of the three misses above | additionally writes the `-1` into `stbds_header(raw_a)->temp` | `err_11_get_key_miss_temp` | [x] |
| 12 | `stbds_hmget_key` (L663) | `a == NULL`: `temp` is written into the *freshly allocated* header | header `temp = -1`, `length = 1`, `capacity = 4` | `err_12_get_key_null` | [x] |
| 13 | `stbds_hmput_default` (L669) | `a == NULL` | allocates, `length = 1`, element 0 zeroed, returns the hash-side pointer (accepted, not rejected) | `err_13_put_default_null` | [x] |
| 14 | `stbds_hmput_default` (L669) | `a != NULL` with a forged `length == 0` | `stbds_arrgrowf(a,e,0,1)` is a no-op (cap 4 ≥ 1), `length` back to 1, element 0 **re-zeroed** | `err_14_put_default_len0` | [x] |
| 15 | `stbds_hmput_key` (L686-691) | `a == NULL` | bootstraps the map: `length = 2`, `capacity = 4`, `slot_count = 8`, `used_count = 1`, `temp = 0` | `err_15_put_null` | [x] |
| 16 | `stbds_hmput_key` (L778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | **unreachable**: L774-775 grows the array to `i+1` immediately above. Evidence: 13 500 randomized put/delete operations across 3 element shapes × 3 seeds never abort on either side | `err_16_21_unreachable_asserts_never_fire` | [x] |
| 17 | `stbds_make_hash_index` (L401) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count ∈ {0,1,2}`. Reached by forging `table->slot_count = 1` + `used_count >= used_count_threshold` so the next `stbds_hmput_key` grows to `slot_count = 2` | **SIGABRT**, `lib.c:401`, `stbds_make_hash_index`, `t->used_count_threshold + t->tombstone_count_threshold < t->slot_count` | `err_17_make_hash_index_assert` | [x] |
| 18 | `stbds_hmdel_key` (L809-810) | `a == NULL` (for every `mode`, incl. `-1`, `2`, `INT_MAX`, `INT_MIN`) | `return 0` (**NULL**) — the library's only NULL-returning path | `err_18_del_null` | [x] |
| 19 | `stbds_hmdel_key` (L816-817) | `hash_table == 0` | sets `stbds_temp(raw_a) = 0` (even if it was garbage), returns `a`; nothing deleted, `length` unchanged | `err_19_del_no_table` | [x] |
| 20 | `stbds_hmdel_key` (L821-822) | key not present (`stbds_hm_find_slot < 0`) | `stbds_temp(raw_a) = 0`, returns `a`; `length`, `used_count`, `tombstone_count`, buckets all unchanged | `err_20_del_missing_key` | [x] |
| 21 | `stbds_hmdel_key` (L828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | **unreachable**: `stbds_hm_find_slot` masks `pos &= slot_count-1` before every probe and returns `(pos & ~7) + i` with `i < 8`. Same fuzz evidence as #16 | `err_16_21_unreachable_asserts_never_fire` | [x] |
| 22 | `stbds_hmdel_key` (L832) | `STBDS_ASSERT(table->used_count >= 0)` — a **tautology**, `used_count` is `size_t`, so gcc eliminates it entirely. Verified: `strings` on `lib.c.o` contains no `"table->used_count >= 0"`, and `objdump -d` shows exactly **9** `__assert_fail` call sites whose line arguments are `401, 778, 828, 846, 849, 913, 953, 954, 955` — 832 is absent; the Rust likewise has exactly 9 `stbds_assert!` sites. Forcing `used_count = 0` before a delete makes `--used_count` wrap to `SIZE_MAX` and the (compiled-out) assert of course still passes | delete succeeds, `used_count == SIZE_MAX`, no shrink (`SIZE_MAX < 0` is false), map still usable. Rust must wrap identically (it uses `wrapping_sub`, not `-=`) | `err_22_used_count_underflow_is_not_an_error` | [x] |
| 23 | `stbds_hmdel_key` (L846) | `STBDS_ASSERT(slot >= 0)`: the relocated last element cannot be re-found. Constructed with `keyoffset = 4` on an `elemsize = 8, keysize = 4` map whose element 0 has `value == key`, so the offset-4 probe "matches" but the re-probe (of the moved element's *value* field) does not | **SIGABRT**, `lib.c:846`, `stbds_hmdel_key`, `slot >= 0` | `err_23_del_reindex_assert` | [x] |
| 24 | `stbds_hmdel_key` (L849) | `STBDS_ASSERT(b->index[i] == final_index)`: the re-probe finds *a* slot, but not the moved element's. Constructed by duplicating element 2's key bytes into element 0 and repointing key 3's slot at index 0 | **SIGABRT**, `lib.c:849`, `stbds_hmdel_key`, `b->index[i] == final_index` | `err_24_del_reindex_wrong_slot` | [x] |
| 25 | `stbds_stralloc` (L913) | `STBDS_ASSERT(len <= a->remaining)` | **unreachable for a self-consistent arena**: the grow branch either returns early (`len > blocksize`) or sets `remaining = blocksize >= len`. Evidence: 6 rounds × 230 allocations walking `±2` around every `512 << b` boundary never abort on either side, and the arena state stays identical | `err_25_stralloc_remaining_assert_unreachable` | [x] |
| 26 | `stbds_stralloc` (L914) | forged arena `{ storage = NULL, remaining = SIZE_MAX }` ⇒ `len <= remaining` skips the grow branch, then `p = a->storage->storage + remaining - len` wraps to a low address and `memmove` writes there | **SIGSEGV** on both sides (identical wrapped destination; note `&raw mut (*NULL).storage` is pure offset arithmetic in both languages, so neither faults before the `memmove`) | `err_26_stralloc_null_storage` | [x] |
| 27 | `stbds_stralloc` (L888) | forged `a->block` with `(block>>1) >= 64` ⇒ `512u << (block>>1)` is **UB in C**; gcc/x86-64 emits `shlq %cl`, whose count is taken mod 64. Tested with `block ∈ {110…133, 250…255}`, i.e. the values whose wrapped `blocksize` is `0` or ≤ 2 KiB. `(block>>1) % 64 ∈ 13…54` asks `realloc` for 2 GiB…8 EiB, which fails in **both** libraries and then dereferences NULL — that outcome is row #50's | Rust `wrapping_shl` uses the same mod-64 rule ⇒ identical `blocksize`, identical branch, identical arena state | `err_27_stralloc_block_ub_shift` | [x] |
| 28 | `stbds_stralloc` (L890-891) | `block` at/over saturation: `512 << (block>>1) >= 1<<20` ⇔ `block >= 22` | `++a->block` is **skipped**; `blocksize` stays capped; repeated allocations keep the same blocksize | `err_28_stralloc_block_saturation` | [x] |
| 29 | `stbds_strreset` (L920) | `a == NULL` ⇒ `x = a->storage` reads through NULL | **SIGSEGV** on both sides | `err_29_strreset_null` | [x] |
| 30 | `stbds_strreset` (L920-930) | arena with `storage == NULL` but non-zero `remaining` / `block` / `mode` (incl. all-`0xFF`) | the `while` body never runs, but `memset(a, 0, 24)` still clears `remaining`, `block` **and** `mode` | `err_30_strreset_empty` | [x] |
| 31 | `stbds_hash_bytes` (L553) | `p == NULL`, `len == 0` | no bytes read; a pure function of `seed`. 518 seeds incl. `0`, `1`, `SIZE_MAX`, `1<<63` | `err_31_hash_bytes_null_zero` | [x] |
| 32 | `stbds_hash_string` (L480) | `str = ""` (immediate NUL) | the `while` never runs; the result is `F(0) + seed`. 518 seeds | `err_32_hash_string_empty` | [x] |
| 33 | `stbds_hash_string` (L480) | `str == NULL` ⇒ `while (*str)` reads through NULL | **SIGSEGV** on both sides | `err_33_hash_string_null` | [x] |
| 34 | out-of-range enum: `mode` at L560/L590/L713/L732 | `mode ∈ {-1, -2, -1000, INT_MIN, INT_MIN+1}` — no valid `STBDS_HM_*` variant | `mode >= STBDS_HM_STRING` is **false** ⇒ full **binary** path. A 300-step put/get/get_ts/delete trace must be *identical* to the `mode == 0` trace, snapshot for snapshot | `err_34_mode_negative` | [x] |
| 35 | out-of-range enum: same sites | `mode ∈ {2, 3, 4, 255, 65536, INT_MAX}` | `mode >= STBDS_HM_STRING` is **true** ⇒ **string** path (`stbds_hash_string`/`strcmp`) and `nt->string.mode = STBDS_SH_DEFAULT`. A 120-step trace must be identical to `mode == 1` | `err_35_mode_above_string` | [x] |
| 36 | out-of-range enum: `stbds_hmdel_key` (L836, L842) | `mode ∈ {2, 3, INT_MAX}` on a string map | `mode == STBDS_HM_STRING` is **false** ⇒ the STRDUP `free` is **skipped** and the re-index would take the *binary* branch. Tested delete-last only (`old_index == final_index`), so the address-dependent re-index is never entered; all 3 storage modes × 3 modes × 5 sizes | `err_36_del_mode2_string_map` | [x] |
| 37 | `stbds_shmode_func` (L803) | `mode` outside `{0,1,2,3}`: `(unsigned char) mode` truncates. **(a)** truncation ∉ `{1,2,3}` (`-1→255`, `-2→254`, `4`, `5`, `127`, `128`, `200`, `255`, `256→0`, `512→0`, `1000→232`, `65536→0`, `INT_MAX→255`, `INT_MIN→0`) ⇒ `switch (table->string.mode)` falls to `default:` ⇒ binary `memcpy` store. **(b)** truncation ∈ `{1,2,3}` (`257`/`0x10001`/`-255 → 1`, `258`/`-254 → 2`, `259`/`-253 → 3`) ⇒ **aliases** the corresponding valid `STBDS_SH_*` mode exactly | `table->string.mode` == the truncated byte; (a) full binary put/get/delete cycle works; (b) the whole 121-snapshot trace equals the trace of `stbds_shmode_func(e, alias)` | `err_37a_shmode_out_of_range_default_branch`, `err_37b_shmode_out_of_range_aliases_valid_modes` | [x] |
| 38 | `stbds_hmput_key` (L789) / `hmget` / `hmdel` | `keysize == 0` on a binary map | `stbds_hash_bytes(key,0,seed)` hashes zero bytes ⇒ every key hashes the same, and `memcmp(_,_,0) == 0` ⇒ the first hash-matching slot always "matches" ⇒ the map degenerates to a single entry (`length == 2`, `used_count == 1`) that keeps being overwritten; every lookup returns `0`; one delete empties it | `err_38_keysize_zero` | [x] |
| 39 | `stbds_arrgrowf` / `stbds_hmput_key` | `elemsize == 0`. Only `elemsize == 0 && keysize == 0` is *well defined*: the data region has capacity `0*4 == 0` bytes, so `memcpy(dst, key, keysize)` with `keysize > 0` writes past the 32-byte header allocation — an out-of-bounds heap write by construction, whose behaviour is undefined for **both** languages and therefore not differentially testable. With `keysize == 0` the whole zero-size path is exercised: header-only allocation, `arr_to_hash`/`hash_to_arr` become identities, all elements alias offset 0, `memcpy(_,_,0)` is a no-op | put/get/get_ts/delete all resolve to element 0, `length` 1↔2, and `stbds_hmput_default`/`stbds_hmget_key`/`stbds_hmfree_func` also work at `elemsize == 0` | `err_39_elemsize_zero` | [x] |
| 40 | `stbds_hmdel_key` (L843/L845) | `keyoffset != 0` while `stbds_hmput_key` (which hard-codes `keyoffset = 0`) stored the keys at offset 0, and the value bytes are chosen `!= key` so the offset-4 `memcmp` cannot accidentally match | `stbds_hm_find_slot` misses ⇒ `temp = 0`, no deletion, map fully intact afterwards (row #20's path via a different trigger) | `err_40_del_keyoffset_mismatch` | [x] |
| 41 | `stbds_hm_find_slot` (L596) / `stbds_hmput_key` (L719) | a key whose raw hash is `0` (`STBDS_HASH_EMPTY`) or `1` (`STBDS_HASH_DELETED`). Reachable through `stbds_hash_string`: for `""` the accumulator equals `seed`, so `hash ^= seed` makes it 0 and the result is `F(0) + seed`; forging `table->seed = 0 - F(0)` (resp. `1 - F(0)`) therefore yields a raw hash of exactly 0 (resp. 1) | `if (hash < 2) hash += 2;` — the value is **bumped**, not rejected: 0 ⇒ **2** and 1 ⇒ **3**. The bumped hash is what lands in the bucket, the raw marker value never appears on a live slot, and the key stays findable/deletable alongside 20 normal keys | `err_41_hash_below_two_is_bumped` | [x] |
| 42 | `intput` (L953) | `STBDS_ASSERT(hmget(intmap, 9) == num)` | never fires: `hmput(intmap, 9, num)` is the last write to key 9. Verified for 400 `num` values × 4 global seeds | `err_42_43_46_intput_extremes` | [x] |
| 43 | `intput` (L954) | `STBDS_ASSERT(hmget(intmap, 11) == 3)` | never fires: `hmput(intmap, 11, 3)` is the last write to key 11 (also when `num == 11`) | `err_42_43_46_intput_extremes` | [x] |
| 44 | `intput` (L955) | `STBDS_ASSERT(hmget(intmap, num) == 7)` with `num == 9` — `hmput(intmap, 9, num)` overwrote key 9's value with `9` | **SIGABRT**, `lib.c:955`, `intput`, `hmget(intmap, num) == 7` | `err_44_intput_9_aborts` | [x] |
| 45 | `intput` (L955) | the same assert with `num == 11` — `hmput(intmap, 11, 3)` overwrote key 11's value with `3` | **SIGABRT**, `lib.c:955`, `intput`, `hmget(intmap, num) == 7` | `err_45_intput_11_aborts` | [x] |
| 46 | `intput` (L945) | `num ∈ {0, 1, -1, 2, 3, 8, 10, 12, -9, -11, INT_MAX, INT_MIN, …}` + 386 random values ≠ 9, 11 | all three asserts hold; returns normally; the global `stbds_hash_seed` advances identically (observed through a fresh table) | `err_42_43_46_intput_extremes`, `cfg_50_intput` | [x] |
| 47 | `strkey` (L939-943) | `n = INT_MIN` — `sprintf("test_%d")` must emit `test_-2147483648` (16 chars + NUL) into the 256-byte static buffer | identical NUL-terminated bytes, length 16, no overflow; and a short key written after a long one is still correctly terminated (the buffer is reused) | `err_47_strkey_int_min`, `cfg_49_strkey` | [x] |
| 48 | `stbds_hmget_key_ts` (L638) | `temp == NULL` out-parameter (with `a == NULL`, so `*temp = STBDS_INDEX_EMPTY` runs) | **SIGSEGV** on both sides | `err_48_get_ts_null_temp` | [x] |
| 49 | `stbds_arrgrowf` (L297) | `elemsize * min_cap` overflows `size_t`: wrapping to `0` — `(1<<63, 4)`, `(1<<62, 8)`, `(1<<61, 16)`, `(1<<60, 16)`, `(1<<32, 1<<32)`, `(1<<63, 8)` — and wrapping to a small **non-zero** size — `(1, SIZE_MAX)`, `(2, SIZE_MAX)` ⇒ `realloc(31)` / `realloc(30)`, whose glibc usable size still covers the 32-byte header | the wrapped size (`0 + 32`) is what `realloc` gets; the allocation **succeeds** and the 32-byte header fits exactly. Rust's `wrapping_mul`/`wrapping_add` produce the identical size argument and identical header (`length`, `capacity`, `temp`, `hash_table`) | `err_49_arrgrowf_size_overflow` | [x] |
| 50 | `stbds_arrgrowf` (L297-303) | `realloc` fails (`elemsize = 1`, `min_cap = 1<<63` ⇒ 8 EiB) ⇒ `b = NULL + 32`, then `stbds_header(b)->length = 0` writes to address 0 | **SIGSEGV** on both sides | `err_50_arrgrowf_oom` | [x] |

## Translation defect found and fixed by this table

Row #3 initially **diverged**: the C's `free((stbds_array_header *) NULL - 1)`
segfaulted while the Rust aborted. The Rust used `ptr::offset(-1)`, which is
UB-checked; the C computes the address with plain wrapping pointer arithmetic.
Fixed by switching `stbds_header`, `STBDS_HASH_TO_ARR`, `STBDS_ARR_TO_HASH` and
every counter update to `wrapping_*` so the Rust matches C for **all** inputs,
including NULL and wild pointers and every `size_t` under/overflow:

* `src/types.rs` — `stbds_header`, `stbds_hash_to_arr`, `stbds_arr_to_hash`
* `src/hash.rs` — `t+1` address math, threshold sums, `slot_count - 1`, probe
  stepping, string/byte pointer walks
* `src/hmap.rs` — bucket indexing, `slot_count - 1`, `used_count`,
  `tombstone_count`, `length`, `i - 1`, `hash += 2`, `step +=`
* `src/arena.rs` — `strlen + 1`, `remaining -= len`
* `src/api.rs` — element indexing, `buffer` writes

## Note on Rust debug builds (rows 3, 26, 29, 33, 48, 50)

The shipped artifact is the **release** `cdylib` (`[profile.release]
panic = "abort"`). Under release, all six fatal rows are byte-identical to the C
(same signal, same message) — verified with `cargo test --release`.

A **debug** build additionally enables `-C debug-assertions`, which makes rustc
emit `ub_checks` around every raw-pointer *dereference*; for rows 29, 33, 48 and
50 those checks turn the SIGSEGV into a SIGABRT *before* the faulting access.
That is a property of the debug build flags, not of the translation, so
`diff_crash_segv` requires exact equality only when `!cfg!(debug_assertions)`
and otherwise just requires both sides to die fatally. Rows 3 and 26 match
exactly in *both* profiles, because the wrapping pointer arithmetic means no
Rust dereference happens before libc/`memmove` faults.
