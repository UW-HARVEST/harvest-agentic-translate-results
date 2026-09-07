# CONFIGS.md — Phase B configuration surface

Axes the C code actually branches on, derived from `c_src/src/lib.c`.

## Axis 1 — `mode` argument of `hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key`
Branch sites: `is_key_equal` (`mode >= STBDS_HM_STRING`, lib.c:560),
`hm_find_slot` (:590), `hmput_key` (:707, :713, :732), `hmdel_key` (:836, :842).
Distinct behaviours: **binary** (`mode < 1`) vs **string** (`mode >= 1`), plus
`hmdel_key`'s extra exact test `mode == STBDS_HM_STRING` (so `mode == 2`
hashes as a string but re-looks-up as *binary* and skips the strdup free).

## Axis 2 — `string.mode` of the hash table (the `switch` at lib.c:785)
`STBDS_SH_NONE (0)` → `default:` `memcpy`; `STBDS_SH_DEFAULT (1)` → store the
caller's pointer; `STBDS_SH_STRDUP (2)` → `stbds_strdup`;
`STBDS_SH_ARENA (3)` → `stbds_stralloc`. Set either implicitly by `hmput_key`
(:707: `DEFAULT` if `mode>=1` else `0`) or explicitly by `stbds_shmode_func`
(:803, `(unsigned char) mode`, any int).

## Axis 3 — `elemsize` / `keysize` / `keyoffset` shapes
`elemsize` scales every pointer walk (`elemsize*i`); `keysize` drives `memcmp` /
`hash_bytes` length; `keyoffset` shifts the key inside the element (only
`hmdel_key` takes it — the get/put entry points hard-code `keyoffset = 0`).

## Axis 4 — table growth / shrink / rebuild state machine
`hmput_key` grows when `used_count >= used_count_threshold` (:698, `slot_count*2`);
`hmdel_key` shrinks when `used_count < used_count_shrink_threshold && slot_count > 8`
(:854, `slot_count>>1`) and rebuilds when
`tombstone_count > tombstone_count_threshold` (:858, same `slot_count`).
Thresholds for `slot_count = 8`: grow at 6, shrink disabled (`:399`), rebuild at >1.

## Axis 5 — hash-function input shape
`siphash_bytes`: whole 8-byte blocks (`i + 8 <= len`) plus a `switch (len - i)`
tail with all 8 fall-through arms; `hash_string`: byte loop terminated by NUL.
`hash < 2 → hash += 2` collision fix-up at :596 / :719.

## Axis 6 — string-arena block-size state machine
`stralloc`: fast path (`len <= remaining`), fresh-block path
(`len <= blocksize = 512 << (block>>1)`), oversize path (`len > blocksize`,
two sub-cases: `storage == NULL` vs `storage != NULL`), and the
`blocksize < 1<<20` saturation of `a->block`.

## Axis 7 — array header (`arrgrowf`) growth policy
`min_len = arrlen + addlen`; `min_len > min_cap → min_cap = min_len`;
`min_cap <= arrcap → return a`; `min_cap < 2*arrcap → 2*arrcap`;
`else if min_cap < 4 → 4`; `a == NULL` initialises `length/hash_table/temp`.

---

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | len 0..80 exhaustive + random byte content, seed = 0 (covers every `switch(len-i)` arm and 0..10 whole blocks) | [x] |
| C2 | `stbds_hash_bytes` | random len 0..64, random seeds incl. 0, 1, `usize::MAX`, `0x31415926`, `0x8000..0` (sign-bit seeds exercise `~seed`) | [x] |
| C3 | `stbds_hash_bytes` | byte content forced to high-bit values (`0x80..0xFF`) — exercises the `(d[3]<<24)` signed-`int` sign-extension in both the block loop and the tail | [x] |
| C4 | `stbds_hash_string` | random ASCII strings len 0..64, seeds as C2 | [x] |
| C5 | `stbds_hash_string` | strings with bytes `0x80..0xFF` (the `(unsigned char)*str` promotion) | [x] |
| C6 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to 0 / 1 / `usize::MAX` / default, then a table created — checks both the seed stored in the table and the `seed = seed*a + b` advance of the global | [x] |
| C7 | `stbds_arrgrowf` | `a == NULL`, `(addlen, min_cap)` over the cross-product `{0,1,2,5,17}×{0,1,3,4,5,9,64}` — covers `min_len>min_cap`, `min_cap<4`, exact-fit | [x] |
| C8 | `stbds_arrgrowf` | existing array, repeated growth `addlen = 1` 200× — exercises `min_cap < 2*arrcap` doubling and the `return a` early-out | [x] |
| C9 | `stbds_arrgrowf` | existing array, `min_cap` far above `2*arrcap` (jump growth), and `min_cap <= arrcap` (no-op) | [x] |
| C10 | `stbds_arrgrowf` + `stbds_arrfreef` | `elemsize` ∈ {1,2,4,8,16,24,switch} — element payload written and read back through the raw pointer | [x] |
| C11 | `stbds_hmput_key` / `stbds_hmget_key` | **binary** mode 0, `elemsize = 16`, `keysize = 4` (`int` key), N = 1 insert | [x] |
| C12 | `stbds_hmput_key` / `stbds_hmget_key` | binary mode 0, `elemsize = 16`, `keysize = 4`, N = 5 (below grow threshold 6) | [x] |
| C13 | `stbds_hmput_key` / `stbds_hmget_key` | binary mode 0, N = 6..7 → crosses `used_count_threshold`, table grows 8→16 | [x] |
| C14 | `stbds_hmput_key` / `stbds_hmget_key` | binary mode 0, N = 500 random keys → multiple grows 8→…→1024, with duplicate keys re-put (the `temp = index` hit path) | [x] |
| C15 | `stbds_hmput_key` | binary mode 0, `keysize = 8` (`size_t`/pointer-sized key), `elemsize = 16` | [x] |
| C16 | `stbds_hmput_key` | binary mode 0, `keysize = 16` == `elemsize = 16` (key fills the element, no value) | [x] |
| C17 | `stbds_hmput_key` | binary mode 0, `keysize = 2` (`short` key) and `keysize = 1` (`char` key) — heavy hash collisions | [x] |
| C18 | `stbds_hmput_key` | binary mode 0, `elemsize = 24` with `keysize = 8` — non-power-of-two-ish stride | [x] |
| C19 | `stbds_hmget_key_ts` | binary mode 0, the low-level `*temp` out-param form, present and absent keys, `a == NULL` bootstrap | [x] |
| C20 | `stbds_hmget_key_ts` | string mode 1, `*temp` out-param, present/absent | [x] |
| C21 | `stbds_hmput_key` | **string** mode 1 with **implicit** `string.mode = STBDS_SH_DEFAULT` (table auto-created by `hmput_key`, lib.c:707) — caller's pointer stored verbatim, `temp_key` set | [x] |
| C22 | `stbds_shmode_func(STBDS_SH_STRDUP)` + `hmput_key` mode 1 | **strdup** table: key heap-duplicated, `temp_key` = the dup | [x] |
| C23 | `stbds_shmode_func(STBDS_SH_ARENA)` + `hmput_key` mode 1 | **arena** table: key allocated from the table's `stbds_string_arena` | [x] |
| C24 | `stbds_shmode_func(STBDS_SH_NONE)` + `hmput_key` mode 1 | `string.mode == 0` → `default:` `memcpy(keysize)` even though `mode == 1` | [x] |
| C25 | `stbds_shmode_func(STBDS_SH_DEFAULT)` + `hmput_key` mode 1 | explicit `DEFAULT` (same as C21 but via `shmode_func`, i.e. the table pre-exists) | [x] |
| C26 | `hmput_key` mode 1, all four `string.mode`s | N = 200 random distinct strings → grows 8→…→512; then re-put every key (hit path, `temp_key` refresh) | [x] |
| C27 | `hmput_key` mode 1 | strings sharing long prefixes / differing only in the last byte / duplicated (`"a"`,`"a"`) | [x] |
| C28 | `hmput_key` mode 1 | empty-string key `""` mixed with non-empty keys | [x] |
| C29 | `hmput_key` mode 2 (PTR_TO_STRING value) | hashes/compares as string (`mode >= 1`) on a `DEFAULT` table | [x] |
| C30 | `hmdel_key` | binary mode 0, `keyoffset = 0`, delete the **last** element (`old_index == final_index`, no `memmove`) | [x] |
| C31 | `hmdel_key` | binary mode 0, delete a **middle** element (`old_index != final_index` → `memmove` + slot re-index) | [x] |
| C32 | `hmdel_key` | binary mode 0, N = 200 inserts then delete every key in random order → drives shrink (`:854`) and rebuild (`:858`) repeatedly | [x] |
| C33 | `hmdel_key` | binary mode 0, interleaved put/del/get sequence of 2000 random ops (tombstone reuse at `:766`) | [x] |
| C34 | `hmdel_key` | string mode 1 on a `DEFAULT` table, middle + last deletes | [x] |
| C35 | `hmdel_key` | string mode 1 on a `STRDUP` table — the `mode == STBDS_HM_STRING && string.mode == STRDUP` free at `:836` | [x] |
| C36 | `hmdel_key` | string mode 1 on an `ARENA` table (no per-key free) | [x] |
| C37 | `hmdel_key` | `keyoffset = 8` with `elemsize = 24`, binary mode 0 (key not at offset 0) | [x] |
| C38 | `hmdel_key` | `keyoffset = 8` with `elemsize = 24`, string mode 1 (`*(char**)(elem+8)`) | [x] |
| C39 | `hmput_default` | `a == NULL` bootstrap; then `hmdefault`-style value write into `t[-1]` | [x] |
| C40 | `hmput_default` | on an already-populated table (pass-through, `:675`) | [x] |
| C41 | `hmput_default` | on a table whose `length == 0` (raw `arrgrowf` array, `:669` second clause) | [x] |
| C42 | `hmfree_func` | `DEFAULT` table (no per-key free, arena empty) | [x] |
| C43 | `hmfree_func` | `STRDUP` table with N keys → frees each `*(char**)(a + elemsize*i)` for `i` in `1..length` | [x] |
| C44 | `hmfree_func` | `ARENA` table with N keys → `strreset` walks the block chain | [x] |
| C45 | `hmfree_func` | binary table (`string.mode == 0`) | [x] |
| C46 | `stbds_stralloc` | fresh arena, N random short strings (len < 512) → block chain growth, `block` counter advance | [x] |
| C47 | `stbds_stralloc` | fresh arena, one oversize string (len > 512) → `storage == NULL` sub-case, `remaining` forced to 0 | [x] |
| C48 | `stbds_stralloc` | non-fresh arena then an oversize string → `storage != NULL` sub-case (insert *after* head, `remaining` preserved) | [x] |
| C49 | `stbds_stralloc` | 9000 x 511-byte payloads: every exhaustion of `remaining` takes a fresh block and advances `a->block`, marching it 0 → **22**, where `512<<11 == 1<<20` stops the increment; 3000 further allocations must not push it past 22 | [x] |
| C50 | `stbds_stralloc` | exact block-boundary lengths: `len == blocksize`, `len == blocksize+1`, `len == remaining`, `len == remaining+1` | [x] |
| C51 | `stbds_stralloc` + `stbds_strreset` | alloc N, reset, alloc N again (arena reuse from a zeroed state) | [x] |
| C52 | `strkey` | `n` over `{0,±1,±9,±10,±99,±100,±12345, INT_MAX, INT_MIN}` and 200 random `i32` | [x] |
| C53 | `sh_puts` | `num` ∈ `{0,1,2,3,4,5,7,8,9,10,63,64,65,100,511,512,513,1000,5000,20000, -1,-2,-1000, INT_MIN}` — stdout compared byte-for-byte (fd-1 `dup2` capture around each `.so`'s own `printf`). `INT_MAX` excluded: see `ERRORS.md` row U5 | [x] |
| C54 | full pipeline (`shmode_func` → `hmput_key` → `hmget_key` → `hmdel_key` → `hmfree_func`) | the `sh_new_arena` / `shputs` / `shgets` / `shdel` / `shfree` macro expansion driven manually, all four `string.mode`s × 300 random ops | [x] |
| C55 | full pipeline, binary (`hmput_key` → `hmget_key_ts` → `hmdel_key` → `hmfree_func`) | the `hmput` / `hmgeti_ts` / `hmdel` / `hmfree` macro expansion, `keysize` ∈ {1,2,4,8,16} × 300 random ops | [x] |
| C56 | cross-check of global seed state | interleave `shmode_func` on C and Rust without `rand_seed` between them and verify the tables' `seed` fields and the advanced global stay in lockstep | [x] |

## Catch-all rows added during verification

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C57 | `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmput_default`/`hmdel_key`/`hmfree_func` | 12 `(elemsize, keysize)` shapes x `mode` ∈ {0, -1, INT_MIN} x 1200 random mixed ops each, with a deliberately tiny key space so duplicates, hash collisions and tombstone reuse are frequent | [x] |
| C58 | same, string side | 3 `string.mode`s x 3 `elemsize`s x `mode` ∈ {1, 2, 9} x 700 random mixed ops, keys drawn from tiny/medium/200-byte shapes | [x] |
| C59 | branch probes | assert that the runs actually reach: `hmput_key` tombstone reuse (lib.c:766), `make_hash_index` rehash-on-grow (`ot != NULL`), a completely full bucket (so the wrapped `i < limit` scan runs), and the `hash < 2 -> hash += 2` fix-up (lib.c:596/:719) | [x] |
| C60 | live `STBDS_ASSERT` parity | `slot_count` driven 8 → >= 4096 → 8; 3000 `stralloc`s whose lengths straddle `remaining`/`blocksize` exactly; `sh_puts` over many `num` — none of A1/A2/A3/A7/A8 may fire in either library | [x] |
