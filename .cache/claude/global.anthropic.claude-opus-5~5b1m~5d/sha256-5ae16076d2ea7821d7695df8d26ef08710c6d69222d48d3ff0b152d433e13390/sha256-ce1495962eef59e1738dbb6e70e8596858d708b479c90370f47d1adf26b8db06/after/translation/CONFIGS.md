# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes on its
runtime options and input shapes.

## The axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `mode` (per-call) | `STBDS_HM_BINARY=0` vs `mode >= STBDS_HM_STRING(1)`; `mode == 1` **exactly** in `hmdel_key` | `lib.c:560, 590, 707, 713, 732, 836, 842` |
| `string.mode` (per-table, set by `stbds_shmode_func` / `hmput_key`) | `SH_NONE=0` → `memcpy`, `SH_DEFAULT=1` → store caller ptr, `SH_STRDUP=2` → `stbds_strdup`, `SH_ARENA=3` → `stbds_stralloc` | `lib.c:785-790, 575, 836` |
| `elemsize` | `0`, `< 8`, `8`, `> 8`, non-multiple-of-key-size | every `elemsize*i` computation |
| `keysize` | `0`, `4`, `8`, `16`, `> key field` | `lib.c:563, 789` |
| `keyoffset` | `0` (all macros) and `!= 0` (`hmdel_key` parameter) | `lib.c:561, 563, 843, 845` |
| element count | `0`, `1`, `< 6` (no grow), `6` (`used_count_threshold` at 8 slots), `> 6` (grow to 16), `> 12` (grow to 32), hundreds (repeated grows) | `lib.c:698` |
| table presence | `hash_table == NULL` (array made by `hmget_key(NULL,…)`) vs `!= NULL` | `lib.c:644, 698, 816` |
| delete pattern | none / one / last element / all / enough to shrink / enough to trigger tombstone rebuild | `lib.c:839, 854, 858` |
| `slot_count` | `8` (never shrinks) vs `>= 16` | `lib.c:399, 854` |
| hash seed | default `0x31415926`, custom via `stbds_rand_seed`, `0`, `SIZE_MAX` | `lib.c:353-358, 409-412` |
| `hash_bytes` len | `0`, `1..7` (each `switch` case), `8`, `9..15`, `16`, `17`, large | `lib.c:522, 532-541` |
| `hash_bytes` content | bytes `< 0x80` vs `>= 0x80` at each of offsets 0..7 (sign-extension quirk) | `lib.c:523-524, 533-539` |
| `hash_string` content | `""`, 1 byte, long, bytes `>= 0x80`, embedded digits | `lib.c:480-481` |
| `arrgrowf` shape | `a NULL`/non-NULL × `addlen` 0/1/n × `min_cap` 0/1/small/large; the `2*cap` vs `4` clamp | `lib.c:283-301` |
| arena block growth | `a->block` 0 → 22 (`blocksize` 512 → 1 MiB), `len <= remaining`, `len > remaining`, `len > blocksize` | `lib.c:885-911` |
| `arr_push(num)` | `<=0`, `1..50`, `51..100`, `>1000` (repeated grow+free cycles) | `lib.c:951-955` |
| `strkey(n)` | `0`, small, large, negative, `INT_MIN`, `INT_MAX` | `lib.c:941` |

## Rows (each is a distinct combination the C treats differently)

Legend for "compared": `hdr` = the full `stbds_array_header`
(`length`/`capacity`/`temp`), `buf` = the raw element bytes `[0, length*elemsize)`,
`idx` = every `stbds_hash_index` scalar field, `bkt` = every bucket's
`hash[8]`/`index[8]` arrays, `ret` = returned scalar / sentinel,
`str` = the C-string contents behind stored key pointers.

### Group 1 — `stbds_hash_bytes` (lowest level, pure)

| # | entry point(s) | configuration (options set + input shape) | compared | ✔ |
|---|----------------|-------------------------------------------|----------|---|
| C1 | `stbds_hash_bytes` | `len = 0`, `p = NULL`, 64 random seeds | ret | [x] |
| C2 | `stbds_hash_bytes` | `len = 1..7` (one row per `switch` case), random bytes < 0x80, random seeds | ret | [x] |
| C3 | `stbds_hash_bytes` | `len = 1..7`, bytes forced `>= 0x80` at every offset (int sign-extension quirk) | ret | [x] |
| C4 | `stbds_hash_bytes` | `len = 8` exactly (one main-loop iteration, empty tail) | ret | [x] |
| C5 | `stbds_hash_bytes` | `len = 9..15` (main loop + each tail case) | ret | [x] |
| C6 | `stbds_hash_bytes` | `len = 16, 17, 24, 25` (multiple main-loop iterations) | ret | [x] |
| C7 | `stbds_hash_bytes` | `len = 1..256` fully random, 2000 random cases, fixed seed | ret | [x] |
| C8 | `stbds_hash_bytes` | `len = 4096` (long message), seeds `0`, `1`, `SIZE_MAX`, `0x31415926` | ret | [x] |
| C9 | `stbds_hash_bytes` | all-`0x00` and all-`0xFF` buffers, `len = 0..64` | ret | [x] |

### Group 2 — `stbds_hash_string` (pure)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C10 | `stbds_hash_string` | `""` (empty), 64 random seeds | ret | [x] |
| C11 | `stbds_hash_string` | 1-byte strings, all 255 non-NUL byte values × 4 seeds | ret | [x] |
| C12 | `stbds_hash_string` | random ASCII, length 1..64, 2000 cases | ret | [x] |
| C13 | `stbds_hash_string` | random bytes incl. `>= 0x80`, length 1..64, 2000 cases (`(unsigned char)` cast) | ret | [x] |
| C14 | `stbds_hash_string` | 4096-byte string, seeds `0` / `SIZE_MAX` | ret | [x] |
| C15 | `stbds_hash_string` | `strkey(n)` output as input, `n` = 0..1000 | ret | [x] |

### Group 3 — `stbds_rand_seed` + `stbds_make_hash_index` seed chain

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C16 | `stbds_rand_seed` → `stbds_shmode_func` | seed `0x31415926` (default), read `table->seed`, then build 8 more tables and read each seed (LCG chain) | idx | [x] |
| C17 | `stbds_rand_seed` → `stbds_shmode_func` | seeds `0`, `1`, `SIZE_MAX`, `0x8000000000000000`, 32 random seeds | idx | [x] |

### Group 4 — `stbds_arrgrowf` / `stbds_arrfreef` (raw dynamic array)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C18 | `stbds_arrgrowf` | `a=NULL`, `elemsize ∈ {0,1,4,8,20,64}`, `addlen=0`, `min_cap ∈ {0,1,2,3,4,5,100}` | hdr, null-ness | [x] |
| C19 | `stbds_arrgrowf` | `a=NULL`, `addlen ∈ {0,1,3,4,5,1000}`, `min_cap=0` (the `<4` clamp) | hdr | [x] |
| C20 | `stbds_arrgrowf` | existing array, `min_cap <= cap` (no-op early return) | hdr, identity | [x] |
| C21 | `stbds_arrgrowf` | existing array, `min_cap` between `cap+1` and `2*cap` (doubling clamp) | hdr | [x] |
| C22 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` (min_cap wins) | hdr | [x] |
| C23 | `stbds_arrgrowf` (via `arrsetcap`/`arrsetlen`) | 200 randomized `(addlen, min_cap)` sequences on one live array, `elemsize=4` | hdr, buf | [x] |
| C24 | `arrput` macro protocol (`arrgrowf`+`arrfreef`) | push `n ∈ {0,1,4,5,8,9,1000}` `i32`s, observe every capacity step | hdr, buf | [x] |
| C25 | `arrput`/`arrpop`/`arrdel`/`arrdeln`/`arrdelswap`/`arrins`/`arrinsn`/`arrsetlen`/`arraddnptr` macro protocol | 300 randomized mixed operations, `elemsize=4`, fixed seed | hdr, buf, ret | [x] |
| C26 | `arrput` macro protocol | `elemsize ∈ {1,2,4,8,16,20}` (`stbds_struct`, `stbds_struct2`), 128 pushes each | hdr, buf | [x] |
| C27 | `stbds_arrfreef` | free a live non-NULL array (no leak/crash, valgrind-clean shape) | no-crash | [x] |

### Group 5 — BINARY hash map (`mode = STBDS_HM_BINARY`)

`hmput` / `hmget` / `hmgeti` / `hmgetp` / `hmdel` / `hmlen` / `hmdefault` /
`hmfree` macro protocols over `stbds_hmput_key`, `stbds_hmget_key`,
`stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmdel_key`,
`stbds_hmfree_func`.

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C28 | `hmput`/`hmgeti` | `{int key; int value;}` (elemsize 8, keysize 4), 0 inserts (empty map) | hdr, idx, bkt | [x] |
| C29 | `hmput`/`hmgeti` | same, 1 insert | hdr, buf, idx, bkt, ret | [x] |
| C30 | `hmput`/`hmgeti` | same, 5 inserts (below `used_count_threshold=6`, no grow) | hdr, buf, idx, bkt, ret | [x] |
| C31 | `hmput`/`hmgeti` | same, 6 inserts (hits threshold → next put grows to 16 slots) | hdr, buf, idx, bkt, ret | [x] |
| C32 | `hmput`/`hmgeti` | same, 7 / 13 / 25 inserts (grow to 16 / 32 / 64) | hdr, buf, idx, bkt, ret | [x] |
| C33 | `hmput`/`hmgeti` | same, 1000 random distinct keys (repeated growth + probe wrap-around) | hdr, buf, idx, bkt, ret | [x] |
| C34 | `hmput` | same, keys with deliberate duplicates (re-put overwrites, `length` stays) | hdr, buf, idx, bkt, ret | [x] |
| C35 | `hmgeti` | lookups of present **and** absent keys interleaved | ret (`-1` vs index) | [x] |
| C36 | `hmget_key_ts` (low level) | same map, `temp` out-param instead of the header slot; verify header `temp` untouched | `*temp`, hdr | [x] |
| C37 | `hmdefault` (`hmput_default`) | on `NULL` map, then on a live map, then `hmget` of a missing key returns the default | hdr, buf | [x] |
| C38 | `hmdel` | delete 1 of 10; delete the physically-last element; delete a missing key | hdr, buf, idx, bkt, ret | [x] |
| C39 | `hmdel` | delete all 10 (drains to `length==1`), then re-insert | hdr, buf, idx, bkt, ret | [x] |
| C40 | `hmdel` | 40 inserts then delete down past `used_count_shrink_threshold` (shrink 64→32→16) | hdr, buf, idx, bkt | [x] |
| C41 | `hmdel`/`hmput` | churn: 2000 randomized put/get/del ops, keys from a small pool (tombstone rebuild + reuse) | hdr, buf, idx, bkt, ret | [x] |
| C42 | `hmput`/`hmgeti` | `{int key[2]; int b,c,d;}` — elemsize 20, keysize 8 (`stbds_struct2`) | hdr, buf, idx, bkt, ret | [x] |
| C43 | `hmput`/`hmgeti` | elemsize 16 / keysize 16 (whole element is the key) | hdr, buf, idx, bkt, ret | [x] |
| C44 | `hmput`/`hmgeti` | elemsize 1, keysize 1 (`u8` key, no value) | hdr, buf, idx, bkt, ret | [x] |
| C45 | `hmput`/`hmgeti` | elemsize 8, keysize 8 (`size_t` keys, incl. `0`, `1`, `SIZE_MAX`) | hdr, buf, idx, bkt, ret | [x] |
| C46 | `hmput`/`hmgeti` | keys chosen so `hash < 2` (`hash += 2` path, E45) | idx, bkt, ret | [x] |
| C47 | `hmfree_func` | free a populated BINARY map (`string.mode == 0`, no per-key free) | no-crash | [x] |
| C48 | all of the above | with `stbds_rand_seed(s)` for `s ∈ {0, 1, 0xdeadbeef, SIZE_MAX}` — different seed ⇒ different bucket layout | hdr, buf, idx, bkt, ret | [x] |

### Group 6 — STRING hash map, `string.mode = SH_DEFAULT` (implicit)

`shput` / `shgeti` / `shdel` protocol with `mode = STBDS_HM_STRING`, table
created implicitly by `hmput_key` (so `hmput_key` sets `string.mode=SH_DEFAULT`).

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C49 | `shput`/`shgeti` | `{char *key; int value;}` (elemsize 16, keysize 8), 1 insert | hdr, buf, idx, bkt, ret | [x] |
| C50 | `shput`/`shgeti` | same, 6 / 7 / 40 inserts using `strkey(n)`-style keys (grow path) | hdr, buf, idx, bkt, ret | [x] |
| C51 | `shput`/`shgeti` | same, 500 random distinct strings | hdr, buf, idx, bkt, ret, str | [x] |
| C52 | `shput` | duplicate key re-put → `temp_key` written from the *existing* stored pointer (E19) | idx.temp_key→str, buf | [x] |
| C53 | `shdel` | delete present / absent / last-element string keys | hdr, buf, idx, bkt, ret | [x] |
| C54 | `shput`/`shdel` | 1500 randomized ops from a 64-string pool | hdr, buf, idx, bkt, ret, str | [x] |
| C55 | `shgeti` | keys that are prefixes/suffixes of each other, and `""` | ret | [x] |
| C56 | `hmfree_func` | free a `SH_DEFAULT` string map (no per-key free, arena empty) | no-crash | [x] |

### Group 7 — STRING map, `string.mode = SH_STRDUP` (`sh_new_strdup`)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C57 | `stbds_shmode_func(e, SH_STRDUP)` | table created explicitly; check the pristine `stbds_hash_index` + array header | hdr, idx, bkt | [x] |
| C58 | `shput`/`shgeti` on strdup table | 1 / 7 / 40 / 400 inserts (keys are copied, caller buffer then scribbled over) | hdr, idx, bkt, ret, str | [x] |
| C59 | `shdel` on strdup table | delete present key (`mode==1` ⇒ the strdup'd key **is** freed, E34) | hdr, idx, bkt, ret, str | [x] |
| C60 | `shput`/`shdel` on strdup table | 1000 randomized ops (free + reuse) | hdr, idx, bkt, ret, str | [x] |
| C61 | `hmfree_func` on strdup table | frees every stored key (`lib.c:575-579`) | no-crash | [x] |

### Group 8 — STRING map, `string.mode = SH_ARENA` (`sh_new_arena`)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C62 | `stbds_shmode_func(e, SH_ARENA)` | pristine table state | hdr, idx, bkt | [x] |
| C63 | `shput` on arena table | short keys, enough of them to fill the first 512-byte block and allocate a second | idx.string (`block`,`remaining`), str | [x] |
| C64 | `shput` on arena table | keys > 512 bytes (oversized-block path, E47) mixed with short keys | idx.string, str | [x] |
| C65 | `shput` on arena table | 400 random keys of length 1..40 (block chain 512→1024→2048…) | idx.string, hdr, idx, bkt, str | [x] |
| C66 | `shput`/`shdel` on arena table | 1000 randomized ops (arena never reclaims) | idx.string, hdr, idx, bkt, str | [x] |
| C67 | `hmfree_func` on arena table | `stbds_strreset` walks and frees the whole block chain | no-crash | [x] |

### Group 9 — `string.mode = SH_NONE (0)` created via `shmode_func`

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C68 | `shmode_func(e, SH_NONE)` + `hmput_key(mode=BINARY)` | `default:` `memcpy` branch on an explicitly-created table | hdr, buf, idx, bkt, ret | [x] |
| C69 | `shmode_func(e, SH_NONE)` + `hmput_key(mode=STRING)` | STRING hashing/compare but `memcpy` storage — the `mode` vs `string.mode` cross-product corner | hdr, buf, idx, bkt, ret | [x] |
| C70 | `shmode_func(e, SH_DEFAULT)` + `hmput_key(mode=STRING)` | explicit `SH_DEFAULT` (vs implicit in C49) | hdr, buf, idx, bkt, ret | [x] |

### Group 10 — string arena directly (`stbds_stralloc` / `stbds_strreset`)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C71 | `stbds_stralloc` | zeroed arena, single 5-byte string (first block, `block` 0→1) | arena fields, str | [x] |
| C72 | `stbds_stralloc` | fill exactly to `remaining == 0`, then one more (new block) | arena fields, str | [x] |
| C73 | `stbds_stralloc` | 400 random strings of length 0..200 → block chain grows 512,512,1024,1024,… | arena fields, str | [x] |
| C74 | `stbds_stralloc` | string longer than the current `blocksize` (oversized path) | arena fields, str | [x] |
| C75 | `stbds_stralloc` | oversized string as the **very first** allocation (`storage == NULL`, E48) | arena fields, str | [x] |
| C76 | `stbds_stralloc` | drive `a->block` from 0 up to its saturation value 22 (`blocksize` 512 → 1 MiB) | arena fields | [x] |
| C77 | `stbds_stralloc` | `""` (len 1) repeatedly, 600 times | arena fields, str | [x] |
| C78 | `stbds_strreset` | empty arena / 1 block / long chain / after oversized blocks | arena fields, no-crash | [x] |

### Group 11 — top-level helpers

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C79 | `strkey` | `n ∈ {0,1,7,9,10,99,100,12345,-1,-99,INT_MIN,INT_MAX}` + 200 random `i32` | full 256-byte static buffer | [x] |
| C80 | `arr_push` | `num ∈ {0,1,2,49,50,51,100,101,500,1000,5000}` | no-crash, no-abort | [x] |
| C81 | `arr_push` | `num` negative (`-1`, `INT_MIN`) | no-crash | [x] |

### Group 12 — composed end-to-end pipelines (multi-entry-point interaction)

| # | entry point(s) | configuration | compared | ✔ |
|---|----------------|---------------|----------|---|
| C82 | `rand_seed` → `shmode_func` → `hmput_key`×N → `hmget_key` → `hmdel_key` → `hmput_key` → `hmfree_func` | full lifecycle, all 4 `string.mode`s × `mode ∈ {0,1}` (pruned to the 6 combinations the C distinguishes) | hdr, buf, idx, bkt, str | [x] |
| C83 | `hmget_key(NULL,…)` → `hmput_key` → `hmget_key` | array bootstrapped by a *get* (so `hash_table == NULL`) and then upgraded by a put | hdr, idx, bkt | [x] |
| C84 | `hmput_default` → `hmput_key` → `hmdel_key`(all) → `hmget_key` | default value survives drain-to-empty | buf, hdr | [x] |
| C85 | interleaved raw `arrgrowf` on a *hash* array | `arrsetcap` on an `hmput`-built array then more `hmput`s | hdr, idx, bkt, buf | [x] |
| C86 | randomized fuzz driver | 6 model configs × 3000 ops each (put/get/get_ts/del/default/len/free-and-restart), fixed seed, cross-checking C and Rust after **every** op | hdr, buf, idx, bkt, ret, str | [x] |

---

## Phase B completion status

**All 86 rows pass**, against both the release and the debug Rust `.so`, under
both the `default` and `--no-default-features` configurations.

| group | rows | test file | result |
|-------|------|-----------|--------|
| 1 `hash_bytes`            | C1–C9   | `tests/hash_fns.rs`       | 9/9 ✅ |
| 2 `hash_string`           | C10–C15 | `tests/hash_fns.rs`       | 6/6 ✅ |
| 3 seed chain              | C16–C17 | `tests/hash_fns.rs`       | 2/2 ✅ |
| 4 raw array               | C18–C27 | `tests/arrays.rs`         | 10/10 ✅ |
| 5 BINARY map              | C28–C48 | `tests/hashmap_binary.rs` | 21/21 ✅ |
| 6 STRING / `SH_DEFAULT`   | C49–C56 | `tests/hashmap_string.rs` | 8/8 ✅ |
| 7 STRING / `SH_STRDUP`    | C57–C61 | `tests/hashmap_string.rs` | 5/5 ✅ |
| 8 STRING / `SH_ARENA`     | C62–C67 | `tests/hashmap_string.rs` | 6/6 ✅ |
| 9 `SH_NONE` / explicit    | C68–C70 | `tests/hashmap_string.rs` | 3/3 ✅ |
| 10 string arena           | C71–C78 | `tests/arena.rs`          | 8/8 ✅ |
| 11 helpers                | C79–C81 | `tests/helpers.rs`        | 3/3 ✅ |
| 12 composed pipelines     | C82–C86 | `tests/composed.rs`       | 5/5 ✅ |

### How rows are compared

Return *pointers* cannot be compared (the two libraries own separate
allocations), so each row asserts equality of everything a real consumer can
observe, byte for byte:

* the whole `stbds_array_header` — `length`, `capacity`, `temp`, and whether
  `hash_table` is set;
* **every** scalar of the `stbds_hash_index`: `slot_count`, `slot_count_log2`,
  `used_count`, `used_count_threshold`, `used_count_shrink_threshold`,
  `tombstone_count`, `tombstone_count_threshold`, `seed`;
* **every slot of every bucket** — the full `hash[8]`/`index[8]` arrays for all
  `slot_count` slots, so any divergence in hashing, probe order, tombstoning or
  rehash placement is caught immediately;
* the embedded `stbds_string_arena` — `remaining`, `block`, `mode`, and the
  length of the `next` block chain;
* that `storage` is 64-byte aligned and lies inside its own allocation;
* the element payload for all `length` elements: raw key bytes for inline keys,
  and the pointed-to *string contents* for `SH_STRDUP`/`SH_ARENA` (where the
  pointers are allocator-dependent) — plus literal pointer equality for
  `SH_DEFAULT`, which must store the caller's pointer;
* every scalar the macros read back: the insertion index, the `hmgeti`/`shgeti`
  result, `hmgeti_ts`'s `*temp` out-param, the `hmdel` 0/1 flag, and `hmlen`.

Rows use many randomized inputs from a fixed-seed splitmix64 RNG (no external
crates), and the stateful rows re-compare the full snapshot **after every single
operation**, not just at the end.

### Rows deliberately restricted (and why)

Two combinations are inherently memory-unsafe *in the C itself*; the tests
exercise the sound part of each and document the rest rather than aborting the
process:

* **C69** (`shmode_func(_, SH_NONE)` + `mode = STBDS_HM_STRING`) stores keys via
  the `default:` `memcpy` branch but compares them via `strcmp` on
  `*(char**)elem`.  Re-putting or looking up a *present* key therefore makes the
  C reinterpret copied string bytes as a pointer and dereference it.  Only
  distinct inserts and absent-key lookups are driven (`is_key_equal` never runs
  for those, because it is reached only on an exact 64-bit hash match).
* **E34/C82 with `mode >= 2` + `SH_STRDUP`** — `stbds_hmdel_key` gates the key
  free and the re-find on `mode == STBDS_HM_STRING` *exactly*, so `mode == 2`
  takes the `memcmp` re-find with a `char**` key and then trips
  `STBDS_ASSERT(slot >= 0)`.  Deletions are issued in reverse insertion order so
  `old_index == final_index` and the re-find is skipped entirely.

Similarly, `stbds_shputs` is only used for keys that are **not already present**:
for a duplicate found in the probe loop's wrap-around half-scan the C never
refreshes `stbds_temp_key` (see `ERRORS.md` E20), so `shputs` would write a stale
pointer into `.key` and break the C's own `hmdel_key` invariant at `lib.c:849`.
`tests/torture.rs` probes with `shgeti` first for exactly this reason.

### C-source coverage reached by these rows plus Phase C

`gcc --coverage` on `c_src/src/lib.c`, driven through the differential suite:
**100.00 % of 374 lines**, 99.10 % of 221 branches (the only untaken branch edges
are the `assert()`-failure edges). See `SYMBOLS.md` for the full report.
