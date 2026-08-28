# MUTATIONS.md — evidence that the differential suite is sensitive

"No divergence found" is only meaningful if the tests *could* have found one.
`scratch/mutate.py` / `scratch/mutate2.py` inject 48 plausible translation bugs
into `translation/src/lib.rs` one at a time, rebuild the `cdylib` and run the
whole suite (`cargo test --release -- --test-threads=1`).  A mutation is
**caught** when cargo exits non-zero.

The pristine source is restored afterwards
(`md5sum translation/src/lib.rs == scratch/lib.rs.pristine`).

```
$ python3 -u scratch/mutate.py      # M01..M28
$ python3 -u scratch/mutate2.py     # M29..M48   (or pass a single id, e.g. M35)
```

## Result: 43 of 48 caught; the 5 survivors are provably *equivalent* mutants

| id | mutation | caught by |
|----|----------|-----------|
| M01 | `arrgrowf` `min_cap < 4` → `< 5` | `phase_b_arr` |
| M02 | `tombstone_count_threshold` drops the `>>4` term | map tests |
| M03 | `used_count_shrink_threshold` `>>2` → `>>3` | map tests |
| M04 | `hash_string` rotate 9 → 8 | `phase_b_hash` |
| M05 | `hash_string` loses the `(unsigned char)` cast | `phase_b_hash` |
| M06 | siphash tail `case 4` zero- instead of sign-extends | `phase_b_hash` |
| M07 | siphash main-loop word zero- instead of sign-extends | `phase_b_hash` |
| M08 | siphash `D_ROUNDS` 4 → 3 | `phase_b_hash` |
| M09 | siphash `len << 56` → `len << 48` | `phase_b_hash` |
| M10 | `probe_position` shifts the hash | map tests |
| M11 | `hmput_key` stores `i` instead of `i-1` | map tests |
| M12 | `hmdel_key` `final_index` off by one | map tests |
| M14 | `stralloc` blocksize `block>>1` → `block` | `phase_b_arena` |
| M15 | `stralloc` clamp `<` → `<=` | `phase_b_arena` |
| M16 | `stralloc` returned offset off by one | `phase_b_arena` |
| M17 | `strkey` format `"test_%d"` → `"test%d"` | `phase_b_driver` |
| M18 | `hmget_key_ts` returns `-2` instead of `-1` for a table-less map | `phase_c_errors` |
| M19 | `is_key_equal` `mode >= STRING` → `== STRING` | `phase_c_errors` |
| M20 | `shmode_func` truncation off by one | `phase_c_errors` |
| M21 | table growth `slot_count*2` → `*4` | map tests |
| M22 | `used_count_threshold` `>>2` → `>>1` | map tests |
| M23 | `hmput_default` drops the `length == 0` case | `phase_c_errors` |
| M24 | `hmdel_key` success sentinel `temp = 1` → `2` | `phase_c_errors` |
| M29 | seed constant `b`'s `v64_lo` | `phase_b_hash::rand_seed_sequence` |
| M30 | seed constant `a`'s `v64_hi` | `phase_b_hash::rand_seed_sequence` |
| M31 | seed constant `a`'s `v64_lo` | `phase_b_hash::rand_seed_sequence` |
| M32 | `hmput_key` forgets `--tombstone_count` on slot reuse | map tests |
| M33 | `hmdel_key` forgets `++tombstone_count` | map tests |
| M36 | rehash drops the `used_count` copy | map tests (hangs → timeout) |
| M37 | `arrgrowf` resets `length` on realloc | map tests |
| M38 | `hmdel_key` shrink condition `<` → `<=` | map tests |
| M39 | `hmdel_key` rebuild condition `>` → `>=` | map tests |
| M40 | `stralloc` oversized splice order | `phase_b_arena` |
| M42 | `hmget_key` does not publish `temp` into the header | map tests |
| M43 | `hmput_key` probe step stops growing | map tests |
| M44 | `hmdel_key` returns NULL instead of `a` for a table-less map | `phase_c_errors` |
| M45 | `strdup` copies `strlen` bytes instead of `strlen+1` | string map tests |
| M46 | `hmfree_func` skips `strreset` (leaks arena blocks) | `phase_d_heap_parity` |
| M47 | `hmfree_func` never frees the `SH_STRDUP` keys | `phase_d_heap_parity` |
| M48 | `hmdel_key` frees the key for out-of-enum string modes too | `phase_d_heap_parity` |
| (extra) | `hmput_key` also writes `temp_key` in the wrap-around probe loop | `phase_b_tempkey::temp_key_on_update` |

### Survivors — each provably cannot change observable behaviour

| id | mutation | why it is equivalent |
|----|----------|----------------------|
| M13 | `hmfree_func`'s `SH_STRDUP` sweep starts at raw index 0 instead of 1 | raw element 0 is the *default* slot, `memset` to zero by `shmode_func`/`hmput_key`, and the `hmdefault`/`hmdefaults` macros only write `.value`. Its key field is therefore always `NULL`, and `free(NULL)` is a no-op. |
| M25 | seed constant `b`'s `v32` argument `715136305` → `+1` | `stbds_load_32_or_64(var,temp,v32,hi,lo)` computes `var = (hi<<32) ^ (low32(lo^v32) ^ v32)`; with 64-bit `size_t` the `v32` terms cancel exactly, so the argument is dead. (The bits that *do* survive — `lo` and `hi` — are covered by M29/M30/M31.) |
| M26 | `make_hash_index` aligns `storage` to 32 instead of `STBDS_CACHE_LINE_SIZE` | only changes the *address* of the bucket array inside an over-allocated block (`+ CACHE_LINE_SIZE-1` slack); no field, no returned value and no allocation size changes. Addresses are deliberately not compared (the two libraries allocate independently). |
| M34 | `arrgrowf`'s `if (min_len > min_cap)` → `>=` | the body is `min_cap = min_len`; when `min_len == min_cap` the assignment is a no-op. |
| M35 / M41 | `find_slot` / `hmput_key` wrap-around loop scans `0..=limit` (or the whole bucket) instead of `0..limit` | slots `pos&7 .. 8` were already examined by the *first* loop, which returns on a key match and on `HASH_EMPTY`, and records a tombstone. Re-examining them can neither match, nor find an empty slot, nor add a new tombstone. |

## Known blind spot (accepted)

A divergence that only reorders `free` calls, or that changes an address
without changing any allocation size, is invisible to a black-box differential
harness. `phase_d_heap_parity` closes the *net* allocation part of that gap
(byte-exact `mallinfo2` slope parity, including the places where the C leaks on
purpose); pure address differences (M26) remain, and by construction cannot
affect any value the library returns.
