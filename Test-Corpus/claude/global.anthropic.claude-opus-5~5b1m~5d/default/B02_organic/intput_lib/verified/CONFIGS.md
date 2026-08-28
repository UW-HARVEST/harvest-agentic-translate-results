# CONFIGS.md — Configuration surface table (valid inputs)

Derived mechanically from the `if` / `switch` / comparison branches in
`c_src/src/lib.c`. `lib.h` exposes only `intput`, but the `.so` exports the full
low-level `stb_ds` C-API (see `SYMBOLS.md`), so **all 16 entry points** are
driven directly, not just the `intput` convenience wrapper.

## Axes the C code actually branches on

**A1 — `mode` (int) passed to `hmput_key` / `hmget_key` / `hmget_key_ts` /
`hmdel_key`.** Branches: `mode >= STBDS_HM_STRING` (lines 560, 590, 713, 732,
865) → *string* hashing/compare; `mode == STBDS_HM_STRING` **exactly**
(lines 836, 842) → strdup-free + key-deref on re-find. So three distinct
classes: `mode < 1`, `mode == 1`, `mode > 1` (e.g. 2 = `STBDS_HM_PTR_TO_STRING`,
which this TU never `#define`s but which callers can pass).

**A2 — `string.mode` (unsigned char) of the table's arena.** Set either
implicitly by `hmput_key` on the first insert (line 707: `SH_DEFAULT` if
`mode>=1` else `0`) or explicitly by `shmode_func` (line 803, truncating cast).
`switch (table->string.mode)` at line 785 distinguishes
`SH_STRDUP(2)` / `SH_ARENA(3)` / `SH_DEFAULT(1)` / `default:` (0 and any other
byte). `hmfree_func` (575) and `hmdel_key` (836) additionally test
`== SH_STRDUP`.

**A3 — table lifecycle / load factor.** `hmput_key` line 698:
`table == NULL` → 8 slots; `used_count >= used_count_threshold` → double.
Thresholds from lines 395-397: `uct = n - n/4`, `tct = n/8 + n/16`,
`ucst = n/4` (forced to 0 when `n <= 8`). `hmdel_key` line 854 shrinks
(`used_count < ucst && slot_count > 8`), line 858 rebuilds
(`tombstone_count > tct`). Distinct shapes: 0, 1, 5, 6 (grow), 11, 12 (grow),
23, 24 (grow), 100, 500 elements.

**A4 — probe geometry.** `hm_find_slot` / `hmput_key` each have a
forward scan `i = pos&7 .. 7` and a wrap scan `i = 0 .. pos&7`, plus the
multi-bucket step `pos += step; step += 8`. Reached only by collisions →
requires many random keys and/or `keysize == 0`. Tombstone reuse
(`tombstone >= 0`, line 766) requires delete-then-insert.

**A5 — `elemsize` / `keysize` / `keyoffset` shapes.** `elemsize` scales every
pointer computation; `keysize` drives `memcmp`/`memcpy` and `hash_bytes`;
`keyoffset` is only non-zero via `hmdel_key`'s 5th parameter
(`STBDS_OFFSETOF(t,key)`).

**A6 — hash seed.** `stbds_hash_seed` starts at `0x31415926`, is copied into
each fresh table (line 409) and then advanced by
`seed = seed*a + b` (line 412). `stbds_rand_seed` overrides it. Inherited
unchanged on rehash/shrink (line 405).

**A7 — raw-array shapes for `arrgrowf`.** `a == NULL` vs not; `min_len > min_cap`;
`min_cap <= arrcap`; `min_cap < 2*arrcap` (doubling) vs `min_cap < 4` (floor 4).

**A8 — byte-string shapes for `hash_bytes`.** `len` residue mod 8 selects the
`switch (len - i)` fall-through case (7…0); bytes ≥ 0x80 exercise the
`int`-promotion sign extension in `data |= (d[3] << 24)`.

**A9 — C-string shapes for `hash_string` / `stralloc`.** empty, 1 char, ≥ 8,
bytes ≥ 0x80 (`(unsigned char) *str++`), long.

**A10 — arena shapes for `stralloc`.** `len <= remaining` (fast path);
`len > remaining && len <= blocksize` (new 512<<(block>>1) block);
`len > blocksize` (dedicated block) × `storage == NULL` vs `!= NULL`;
`block` saturation at `blocksize >= 1<<20`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B1  | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap=0` → the `min_cap <= arrcap` early return fires first, so **NULL** is returned and nothing is allocated (A7) | [x] |
| B2  | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap ∈ {1,2,3,4,5,7,100,1000}`, `elemsize ∈ {1,2,4,8,12,16,64}` | [x] |
| B3  | `stbds_arrgrowf` | `a=NULL`, `addlen ∈ {1,3,4,5,17,64}`, `min_cap=0` → `min_len > min_cap` path | [x] |
| B4  | `stbds_arrgrowf` | `a` non-NULL, `min_cap <= arrcap` → early `return a` (no realloc, header untouched) | [x] |
| B5  | `stbds_arrgrowf` | `a` non-NULL, doubling path `min_cap < 2*arrcap` | [x] |
| B6  | `stbds_arrgrowf` | `a` non-NULL, `min_cap >= 2*arrcap` (explicit big `min_cap`) | [x] |
| B7  | `stbds_arrgrowf` + payload | randomized `arrput`-style sequences (500 pushes) over `elemsize ∈ {1,2,4,8,12,16,64}`; compare length/capacity/temp + full payload bytes | [x] |
| B8  | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (valgrind-free smoke: no crash, identical pre-free state) | [x] |
| B9  | `stbds_rand_seed` + `stbds_hmput_key` | seed ∈ {0 (default 0x31415926), 1, 0xdeadbeef, usize::MAX, random×8}; verify `table->seed` and the global advance across 5 fresh tables (A6) | [x] |
| B10 | `stbds_hash_bytes` | `len=0` with `p=NULL` **and** `p` non-NULL; seeds {0,1,MAX,rand} (A8) | [x] |
| B11 | `stbds_hash_bytes` | `len ∈ 1..=8` (tail cases 1..7 + one full block), random bytes, 200 iterations/len | [x] |
| B12 | `stbds_hash_bytes` | `len ∈ 9..=40` (full blocks + every tail residue), random bytes | [x] |
| B13 | `stbds_hash_bytes` | `len ∈ {64, 100, 255, 1000}`, random bytes; also all-`0x00`, all-`0xFF`, all-`0x80` (sign-extension) | [x] |
| B14 | `stbds_hash_bytes` | unaligned `p` (offset 1..7 into buffer), `len ∈ 1..24` | [x] |
| B15 | `stbds_hash_string` | `""`, `"a"`, 1..64 random ASCII, seeds {0,1,MAX,rand} (A9) | [x] |
| B16 | `stbds_hash_string` | bytes 0x80..0xFF in the string (unsigned-char promotion), lengths 1..32 | [x] |
| B17 | `stbds_hash_string` | long strings (256, 1000, 4096 chars) | [x] |
| B18 | `stbds_hmput_key` (mode 0) | `elemsize=8, keysize=4, keyoffset=0`; 1 insert into `a=NULL` (table creation, A3) | [x] |
| B19 | `stbds_hmput_key` (mode 0) | 5 inserts (below `uct=6`, no grow) | [x] |
| B20 | `stbds_hmput_key` (mode 0) | 6 inserts → grow 8→16; 12 → 16→32; 24 → 32→64 (A3 boundaries) | [x] |
| B21 | `stbds_hmput_key` (mode 0) | 100 and 500 random distinct keys → repeated grows, deep bucket comparison after every op (A3, A4) | [x] |
| B22 | `stbds_hmput_key` (mode 0) | re-put of existing keys interleaved with new keys (existing-key path, line 730) | [x] |
| B23 | `stbds_hmget_key` (mode 0) | present / absent keys on maps of size 0,1,5,6,50 | [x] |
| B24 | `stbds_hmget_key_ts` (mode 0) | present / absent; verify `*temp` **and** that `header->temp` is *not* written (contrast with `hmget_key`) | [x] |
| B25 | `stbds_hmdel_key` (mode 0) | delete 1 of 1 (`old_index == final_index`), 1 of many (memmove + re-find), last element | [x] |
| B26 | `stbds_hmdel_key` (mode 0) | delete until `tombstone_count > tct` → rebuild at same slot_count (line 858) | [x] |
| B27 | `stbds_hmdel_key` (mode 0) | grow to 32/64 then delete until `used_count < ucst` → shrink (line 854) | [x] |
| B28 | `stbds_hmdel_key` (mode 0) | delete-then-insert → tombstone reuse (`tombstone >= 0`, line 766) | [x] |
| B29 | mixed (mode 0) | 2000 randomized ops (put/get/get_ts/del/default) against a Rust-side reference model, deep snapshot compare each step; seeds 1..5 | [x] |
| B30 | `stbds_hmput_key` (mode 0) | `keysize ∈ {1,2,4,8,16}` with `elemsize ∈ {keysize+4 … 64}`, random keys (A5) | [x] |
| B31 | `stbds_hmput_key` (mode 0) | `keysize=0` → every hash-matching slot compares equal (A5 degenerate) | [x] |
| B32 | `stbds_hmdel_key` (mode 0) | `keyoffset ∈ {0,4,8}` (key not first field), elemsize 16/24 | [x] |
| B33 | `stbds_hmput_default` | on `a=NULL`; on an `arrgrowf` array with `length==0`; twice; before and after `hmput_key`; then read `t[-1]` | [x] |
| B34 | `stbds_hmfree_func` | binary map (no arena), sizes 0/1/6/50 | [x] |
| B35 | `stbds_hmput_key` (mode 1) | `a=NULL` → implicit `string.mode = SH_DEFAULT` (A2 line 707); keys are caller pointers | [x] |
| B36 | `stbds_shmode_func(SH_STRDUP=2)` + `hmput_key(mode 1)` | keys strdup'd (line 786); 1/6/12/100 keys; compare key *contents* | [x] |
| B37 | `stbds_shmode_func(SH_ARENA=3)` + `hmput_key(mode 1)` | keys arena-allocated (line 787); short + long keys crossing the 512-byte block boundary | [x] |
| B38 | `stbds_shmode_func(SH_NONE=0)` + `hmput_key(mode 1)` | `switch` `default:` → `memcpy(key, keysize)` even though mode is *string* (A2) | [x] |
| B39 | `stbds_shmode_func(SH_DEFAULT=1)` + `hmput_key(mode 1)` | explicit default mode | [x] |
| B40 | `stbds_hmget_key` (mode 1) | present / absent string keys for each of the 4 `string.mode` values | [x] |
| B41 | `stbds_hmdel_key` (mode 1) | delete for each `string.mode`; `SH_STRDUP` also frees the key (line 837) | [x] |
| B42 | `stbds_hmfree_func` | `SH_STRDUP` map (frees every key, line 578) / `SH_ARENA` (strreset, 580) / `SH_DEFAULT` / `SH_NONE` | [x] |
| B43 | `stbds_hmput_key` (mode 1) | duplicate string key → existing-key path sets `temp_key` (line 733); verify `temp_key` contents | [x] |
| B44 | mode 2 (`STBDS_HM_PTR_TO_STRING`) | `hmput_key`/`hmget_key` with `mode=2` → string hashing but `hmdel_key`'s `mode==1` tests are false (A1) | [x] |
| B45 | mode ∈ {-1, INT_MIN, -1000} | binary class; must be byte-identical to `mode=0` runs | [x] |
| B46 | mode ∈ {7, 1000, INT_MAX} | string class for put/get; `hmdel_key` takes the `mode != 1` sub-path | [x] |
| B47 | string keys shapes | `""`, 1 char, keys differing only in the last byte, 200-byte keys, keys with bytes ≥ 0x80 (A9) — for `SH_STRDUP` and `SH_ARENA` | [x] |
| B48 | `stbds_stralloc` | fresh zeroed arena, `len < 512` → new 512 block, `remaining` bookkeeping (A10) | [x] |
| B49 | `stbds_stralloc` | fill a block exactly then one more alloc → 2nd block, `block` increments 0→1→2… | [x] |
| B50 | `stbds_stralloc` | `len > blocksize` (4096-byte string, `block=0`) with `storage == NULL` → dedicated block **and** `remaining = 0` | [x] |
| B51 | `stbds_stralloc` | `len > blocksize` with `storage != NULL` → dedicated block spliced *after* head; `remaining` untouched | [x] |
| B52 | `stbds_stralloc` | pre-set `block ∈ 0..=20` → `blocksize = 512<<(block>>1)`; verify `block` increment stops at `blocksize >= 1<<20` (saturates at 22) | [x] |
| B53 | `stbds_stralloc` | 300 random strings (len 0..900) into one arena; compare every returned string + arena fields (A10 mixed) | [x] |
| B54 | `stbds_strreset` | populated arena (multi-block), empty arena, twice in a row | [x] |
| B55 | `strkey` | `n ∈ {0,1,-1,9,11,10,99,100,-99, INT_MIN, INT_MAX}` + 200 random ints; compare returned bytes | [x] |
| B56 | `strkey` | two consecutive calls (static buffer aliasing) | [x] |
| B57 | `intput` | `num ∉ {9,11}`: `{0,1,-1,2,7,8,10,12,100,-100, INT_MIN, INT_MAX}` + 200 random — must return normally | [x] |
| B58 | composed pipeline (mode 0) | `rand_seed(s)` → 60 puts → 20 dels → 30 puts → 40 gets → `hmput_default` → `hmfree`, for `s ∈ 6 values`, deep compare at every step | [x] |
| B59 | composed pipeline (mode 1, `SH_STRDUP`) | `shmode_func` → 60 `shput` → 20 `shdel` → 30 `shput` → gets → `hmfree`; deep compare at every step | [x] |
| B60 | composed pipeline (mode 1, `SH_ARENA`) | same as B59 with `SH_ARENA` plus long keys forcing dedicated arena blocks | [x] |
| B61 | composed pipeline (mode 1, `SH_DEFAULT`) | same as B59, caller-owned key pointers | [x] |
| B62 | element padding | key at offset 0 (`keysize=4`) inside `elemsize=16`/`24`/`64` elements with random filler → verifies only `keysize` bytes participate in compare/copy | [x] |
| B63 | `hmput_default` + string modes | `hmput_default` on a `SH_STRDUP`/`SH_ARENA` map (element `-1` is zeroed, key ptr NULL) then `hmfree_func` (loop starts at `i=1`, line 577) | [x] |
| B64 | `stbds_shmode_func` | `elemsize ∈ {8,12,16,24,64}` × `mode ∈ {0,1,2,3}`; verify fresh 8-slot table, `length=1`, seed advance | [x] |
| B65 | `stbds_hash_bytes` + `stbds_hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` (mode 0) | **multi-bucket probing (A4), deterministic**: grow the table to 1024 slots, then use the library's own `stbds_hash_bytes` (through the FFI, with the table's actual seed) to *select* 24 keys that all probe into bucket 0, forcing the `pos += step; step += 8` continuation plus the wrap scan; then present-get, missing-get, delete and re-insert inside the saturated bucket (tombstone reuse on a long probe chain). 3 seeds | [x] |
