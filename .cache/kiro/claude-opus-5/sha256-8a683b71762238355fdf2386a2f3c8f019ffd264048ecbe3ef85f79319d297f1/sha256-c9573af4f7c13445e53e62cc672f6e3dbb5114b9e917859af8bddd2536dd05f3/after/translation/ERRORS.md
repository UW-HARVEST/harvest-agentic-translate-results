# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c`. The library is stb_ds: it has **no
error-code return values**; every rejection is expressed as one of

* a sentinel index (`STBDS_INDEX_EMPTY == -1`, `STBDS_INDEX_DELETED == -2`),
* an early `return` of the input pointer / `NULL` / `0`,
* an `assert()` (`#define STBDS_ASSERT assert`, `NDEBUG` is **not** defined by
  `c_src/CMakeLists.txt`, so asserts are live and `abort()` the process),
* or undefined behaviour that is nevertheless *identical* in both builds
  because both go through the same glibc `realloc`/`free`.

`ptr::eq`-style pointer identity is never comparable across the two libraries;
"returns `a` unchanged" is asserted as *the same pointer the callee was given*.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `stbds_hmfree_func` | `a == NULL` (line 573 `if (a == NULL) return;`) | returns immediately, no free, no crash | `err_01_hmfree_null` |
| 2 | `stbds_hmfree_func` | `a != NULL` but `stbds_header(a)->hash_table == NULL` (line 574 false) | skips strreset, `free(NULL)`, frees header only | `err_02_hmfree_no_table` |
| 3 | `stbds_hmfree_func` | table present, `string.mode != STBDS_SH_STRDUP` | does **not** free element key pointers | `err_03_hmfree_no_strdup_mode` |
| 4 | `stbds_hm_find_slot` (via `hmget`) | key absent, probe hits `STBDS_HASH_EMPTY` bucket (lines 610/621) | `-1` | `err_04_get_missing_key` |
| 5 | `stbds_hmget_key_ts` | `a == NULL` | allocates 1-elem array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash ptr | `err_05_get_ts_null_a` |
| 6 | `stbds_hmget_key_ts` | `a != NULL`, `hash_table == 0` (line 644) | `*temp = -1`, returns `a` unchanged | `err_06_get_ts_no_table` |
| 7 | `stbds_hmget_key_ts` | key absent → `slot < 0` (line 649) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err_07_get_ts_missing` |
| 8 | `stbds_hmget_key` | same three cases, propagated into `stbds_header(t-1)->temp` | `temp == -1` | `err_08_get_key_temp_minus1` |
| 9 | `stbds_hmdel_key` | `a == NULL` (line 809) | returns `0` (NULL) | `err_09_del_null_a` |
| 10 | `stbds_hmdel_key` | `a != NULL`, `hash_table == 0` (line 816) | `temp` set to 0, returns `a` unchanged | `err_10_del_no_table` |
| 11 | `stbds_hmdel_key` | key absent → `slot < 0` (line 822) | `temp == 0`, length unchanged, returns `a` | `err_11_del_missing_key` |
| 12 | `stbds_hmdel_key` | delete the same key twice | 2nd call: `temp == 0`, length unchanged | `err_12_del_twice` |
| 13 | `stbds_hmput_default` | `a == NULL` | allocates, `length == 1` | `err_13_put_default_null` |
| 14 | `stbds_hmput_default` | `a != NULL` and `length == 0` (line 669, 2nd disjunct) | re-grows and bumps length to 1 | `err_14_put_default_len0` |
| 15 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` unchanged (no realloc) | `err_15_put_default_noop` |
| 16 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` (line ~290) | returns `a` unchanged, **no** realloc | `err_16_arrgrowf_noop` |
| 17 | `stbds_arrgrowf` | `a == NULL, addlen == 0, min_cap == 0` | `min_cap` bumped to 4, fresh header `len=0,temp=0,table=NULL` | `err_17_arrgrowf_zero` |
| 18 | `stbds_arrgrowf` | `elemsize == 0` (degenerate but legal input) | `realloc(NULL, 0*cap+32)`, `capacity = 4` | `err_18_arrgrowf_elemsize0` |
| 19 | `stbds_arrgrowf` | `elemsize = 1<<40`, `min_cap = 1<<20` ⇒ `realloc` fails, `b == NULL`, then `(char*)NULL+32` is written through | SIGSEGV — **verified equal** (both children die with signal 11) | `err_19_arrgrowf_oom` (subprocess, signal compared; skipped for the `debug` cdylib, see note A) |
| 20 | `stbds_arrfreef` | `a == NULL` → `free((header*)NULL - 1)` = `free(0xffff…ffe0)` | SIGSEGV — **verified equal** (both children die with signal 11) | `err_20_arrfreef_null` (subprocess, signal compared) |
| 21 | `stbds_hash_bytes` | `len == 0` (pointer never dereferenced) | `data = 0 << 56`, deterministic hash of the empty string | `err_21_hash_bytes_len0` |
| 22 | `stbds_hash_bytes` | `p == NULL` **and** `len == 0` | same as row 21, no deref | `err_22_hash_bytes_null_len0` |
| 23 | `stbds_hash_string` | empty string `""` | loop body never runs, hash of bare seed | `err_23_hash_string_empty` |
| 24 | `stbds_hash_string` | bytes >= 0x80 (`(unsigned char)` cast ⇒ zero-extension, not sign) | zero-extended byte contribution | `err_24_hash_string_high_bytes` |
| 25 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` (big-string path) | dedicated block spliced in; when `a->storage == NULL` also sets `remaining = 0` | `err_25_stralloc_oversize` |
| 26 | `stbds_stralloc` | `a->block` pre-set >= 22 (blocksize saturates at `1<<20`, `++a->block` suppressed) | `block` stops incrementing | `err_26_stralloc_block_saturate` |
| 27 | `stbds_stralloc` | `a->block` pre-set to 126..255 ⇒ `512 << (block>>1)` shift count >= 63 | x86-64 `shl` masks the count to 6 bits; the Rust uses `wrapping_shl` to make that explicit rather than relying on LLVM | `err_27_stralloc_shift_overflow`, `cfg_38_stralloc_block_field` (all 256 `block` values) |
| 27b | `stbds_stralloc` | `a->block = 63` ⇒ blocksize `512<<31` = 1 TiB, `realloc` fails, `sb->next` written through NULL | SIGSEGV — **verified equal** | `err_27b_stralloc_oom` (subprocess; skipped for the `debug` cdylib, see note A) |
| 28 | `stbds_stralloc` | `assert(len <= a->remaining)` (line 913) — unreachable from the public API, only via a hand-forged arena with `storage != NULL, remaining < len` and `len <= blocksize`… guarded by the realloc above | asserted unreachable; documented, not triggerable without also corrupting `storage` | n/a (documented) |
| 29 | `stbds_strreset` | `a->storage == NULL` (empty arena) | loop skipped, arena memset to 0 | `err_29_strreset_empty` |
| 30 | `stbds_strreset` | already-reset arena reset again | idempotent, all-zero | `err_30_strreset_twice` |
| 31 | `stbds_is_key_equal` / `mode` | **out-of-range enum-ish `mode`**: any `int`. `mode >= STBDS_HM_STRING (1)` selects `strcmp`, else `memcmp`. So `mode = 2, 7, 1000, INT_MAX` behave as STRING; `mode = 0, -1, -5, INT_MIN` behave as BINARY | dispatch is a `>=` test, not an equality test | `err_31_mode_out_of_range` |
| 32 | `stbds_hmput_key` | `mode >= 1` on a **fresh** table ⇒ `nt->string.mode = STBDS_SH_DEFAULT`; `mode < 1` ⇒ `0` | `string.mode` = 1 resp. 0 | `err_32_put_fresh_mode` |
| 33 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING (== 1 exactly)` gates the strdup `free` — `mode == 2` (also "string" for hashing) does **not** free and takes the **binary** re-find path | asymmetric `==` vs `>=`; faithful quirk. Deleting the LAST element skips the re-find and is well-defined | `err_33_del_mode2_asymmetry`, `cfg_33_str_map_del_mode2_last` |
| 33b | `stbds_hmdel_key` | `mode == 2` deleting a **non-last** element: the binary re-find hashes the slot's pointer bytes as a string, finds nothing, and trips `STBDS_ASSERT(slot >= 0)` | SIGABRT — **verified equal** | `err_33b_del_mode2_nonlast_aborts` (subprocess) |
| 34 | `stbds_shmode_func` | out-of-range `mode`: `(unsigned char) mode` truncation, e.g. `mode = 256` ⇒ `string.mode = 0`, `mode = -1` ⇒ `255`, `mode = 259` ⇒ `3` (ARENA) | truncating cast | `err_34_shmode_out_of_range` |
| 35 | `stbds_shmode_func` | `string.mode` set to `0` or an undefined value (e.g. `4`) then `hmput_key` with `mode = 1` ⇒ `switch` `default:` ⇒ `memcpy(key, keysize)` copies the first `keysize` BYTES OF THE STRING into the key slot instead of a pointer | falls into the binary branch | `err_35_shmode_undefined_mode`, `cfg_29_str_map_sh_none`, `cfg_30_str_map_undefined_sh_mode` |
| 35b | `stbds_hm_find_slot` | a **lookup** on such a map reinterprets those copied bytes as a `char*` and dereferences it | SIGSEGV — **verified equal** | `err_35b_sh_none_lookup_segv` (subprocess) |
| 36 | `stbds_hmput_key` | `keysize == 0` with `string.mode == 0` ⇒ `memcpy(...,0)`; every key hashes identically (`hash_bytes(p,0,seed)`) so the 1st key wins forever | 2nd distinct key is reported as a hit | `err_36_put_keysize0` |
| 37 | `stbds_hmdel_key` | `keyoffset` non-zero (key not first member) | re-find uses `+keyoffset` | `err_37_del_keyoffset` |
| 38 | `stbds_hmdel_key` | `assert(slot < table->slot_count)`, `assert(slot >= 0)`, `assert(b->index[i] == final_index)` (lines 828/846/849) | unreachable when the table invariants hold; reachable only by corrupting the table, which is not a valid FFI input | n/a (documented) |
| 39 | `stbds_hmput_key` | `assert((size_t)i+1 <= stbds_arrcap(a))` (line 778) | unreachable — the preceding `arrgrowf` guarantees it | n/a (documented) |
| 40 | `stbds_make_hash_index` | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` (line 401) | holds for every `slot_count` the code can produce (`8, 16, 32, …`) | n/a (documented) |
| 41 | `arr_ins` | any `int num`, incl. `INT_MIN`, `INT_MAX`, `0`, `4` | 3 asserts per iteration, all satisfied for every `num`; returns void | `err_41_arr_ins_extremes` |
| 42 | `strkey` | `INT_MIN` (`"test_-2147483648"`, 17 bytes incl. NUL, fits the 256-byte static buffer) | exact `printf("%d")` formatting | `err_42_strkey_extremes` |
| 43 | `stbds_hmget_key` / `hmdel_key` | table that has been grown *and* shrunk (`used_count < used_count_shrink_threshold && slot_count > 8`) and rebuilt (`tombstone_count > tombstone_count_threshold`) — the tombstone/`STBDS_HASH_DELETED` paths | tombstone slots are skipped, then reclaimed | `err_43_tombstone_paths` |
| 44 | `stbds_hmput_key` | re-put an existing key after deletions so a **tombstone** is reused (`tombstone >= 0` at `found_empty_slot`) | `--tombstone_count`, slot reused | `err_44_tombstone_reuse` |

## Note A — the two OOM rows and the `debug` cdylib

Rows 19 and 27b are "`realloc` returned NULL and the C writes through it".  The
**release** cdylib — the artifact that corresponds to the C `.so`, and the one
`[profile.release] panic = "abort"` in `Cargo.toml` is written for — faults with
exactly the same signal as the C.  A cdylib built with `debug_assertions` also
carries rustc's "null pointer dereference occurred" check, which converts the
SIGSEGV into a SIGABRT *before* the faulting store.  That is Rust runtime
instrumentation rather than a translation difference, so `same_termination_oom`
skips those two rows (with a printed SKIP) when the suite is deliberately run
against the debug artifact via `DIFF_RUST_PROFILE=debug`.  Every other row —
including the three other crash rows (20, 33b, 35b) — passes under both
profiles.

## Rows deliberately marked `n/a`

Rows 28, 38, 39 and 40 are `assert()`s that the surrounding code makes
unreachable for every input that can cross the FFI boundary; reaching them
requires corrupting the library's own heap structures, which is not an input.
They are listed so the audit is complete, not skipped.

## Result

44 distinct rejection/error behaviours enumerated; 40 have an executing
differential test (`tests/phase_c.rs`), 4 are documented-unreachable asserts.
**0 divergences remain.**
