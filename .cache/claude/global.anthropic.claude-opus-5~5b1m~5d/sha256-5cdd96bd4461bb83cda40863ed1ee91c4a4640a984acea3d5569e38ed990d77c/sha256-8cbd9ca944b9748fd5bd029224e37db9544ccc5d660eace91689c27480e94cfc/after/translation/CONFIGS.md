# CONFIGS.md — configuration surface (valid inputs)

Axes the C code actually branches on (`c_src/src/lib.c`):

* **`mode` argument** of `hmget_key{,_ts}` / `hmput_key` / `hmdel_key`.
  Branches are `mode >= STBDS_HM_STRING(1)` (hash/compare as C string) in
  `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_hmput_key`, and
  `mode == STBDS_HM_STRING` **exactly** in `stbds_hmdel_key` (strdup free +
  key re-find).  Distinct classes: `<=0` (binary, incl. negatives),
  `1` (string), `2` (`PTR_TO_STRING`), `>2`.
* **`string.mode`** of the hash index (`STBDS_SH_NONE 0`, `SH_DEFAULT 1`,
  `SH_STRDUP 2`, `SH_ARENA 3`, out-of-enum): selects the `switch` in
  `hmput_key` (memcpy vs. strdup vs. arena vs. store-pointer) and the teardown
  in `hmfree_func` / free in `hmdel_key`.  Set implicitly by `hmput_key`
  (0 or 1) or explicitly by `stbds_shmode_func`.
* **`elemsize` / `keysize`**: `elemsize` drives all the pointer arithmetic;
  `keysize` drives `memcmp` and the siphash length.  Siphash special-cases
  `len % 8` in a fall-through `switch` (cases 0..7) and loops per 8 bytes, so
  every `keysize` in 0..=17 is a distinct shape.
* **`keyoffset`** (only `hmdel_key` takes it; the macros pass
  `offsetof(elem,key)`, which is 0 for the standard layout but non-zero for
  key-not-first structs).
* **element count / table size**: `hash_table == NULL`, 1 element, the
  bucket boundary (7/8/9), the growth threshold (`used_count >= 3/4 slot_count`
  → rehash to `2n`), the shrink threshold (`used_count < 1/4 slot_count` and
  `slot_count > 8` → rehash to `n/2`), and the tombstone threshold
  (`> 3/16 slot_count` → rebuild at `n`).
* **global seed**: `stbds_rand_seed` seeds `stbds_hash_seed`, which is consumed
  *and advanced* (`seed*a+b`) by every fresh `stbds_make_hash_index`, so the
  whole hash layout depends on call history.  Both libraries must be re-seeded
  identically before every comparison.
* **`arrgrowf` shape**: `a` NULL vs. non-NULL × `addlen` 0 vs. n ×
  `min_cap` below/above `arrcap` and below/above 4.
* **string arena shape** (`stralloc`): fresh arena, string fits remaining,
  string forces a new block, string bigger than a whole block, `a->block`
  saturation at `1<<20`, arena with/without an existing head block.
* **`strkey` / `arr_ins`**: `int` argument value (positive, 0, negative,
  `INT_MIN`, `INT_MAX`).

Full public entry-point list (all 16 exported symbols) is covered; the
"low level" ones (`stbds_arrgrowf`, `stbds_hash_bytes`, `stbds_hash_string`,
`stbds_stralloc`, `stbds_strreset`) are driven directly, not only through the
map wrappers.

| #  | entry point(s) | configuration (options set + input shape) | status |
|----|----------------|--------------------------------------------|-----|
| 1  | `stbds_arrgrowf` | `a=NULL`, `elemsize ∈ {1,2,4,8,16,24,32}`, `addlen ∈ {0,1,3,17}`, `min_cap ∈ {0,1,4,5,100}` — fresh allocation, capacity-rounding rules | [x] |
| 2  | `stbds_arrgrowf` | non-NULL `a` from a previous grow, request **already satisfied** (`min_cap <= arrcap`) → pointer returned unchanged | [x] |
| 3  | `stbds_arrgrowf` | non-NULL `a`, doubling path (`min_cap < 2*arrcap`) vs. explicit-larger path, `length` preserved | [x] |
| 4  | `stbds_arrgrowf` + `stbds_arrfreef` | repeated grow chain (append-loop shape, 0..300 elements) then free; compare header (`length`,`capacity`,`temp`,`hash_table`) at every step | [x] |
| 5  | `stbds_hash_bytes` | `len ∈ 0..=24` (all 8-byte-block counts × all `len%8` fall-through cases), random bytes, `seed` random | [x] |
| 6  | `stbds_hash_bytes` | `len ∈ 0..=24`, bytes with the **high bit set** in positions 3 and 7 (exercises the sign-extension of `d[3]<<24`) | [x] |
| 7  | `stbds_hash_bytes` | large `len` (64, 65, 1024, 4096) random data, `seed ∈ {0, 1, usize::MAX, random}` | [x] |
| 8  | `stbds_hash_string` | `""`, 1..64-char ASCII, bytes `>= 0x80` (signed-char cast), `seed ∈ {0,1,MAX,random}` | [x] |
| 9  | `stbds_rand_seed` + `stbds_hmput_key` | seed set to `{0x31415926, 0, 1, usize::MAX, random}`; verify the seed advance is identical by building the same map twice | [x] |
| 10 | `stbds_hmput_key` / `hmget_key` | `mode=0` (binary), `elemsize=8`, `keysize=4` (`{int key; int value;}`), 1 element | [x] |
| 11 | `stbds_hmput_key` / `hmget_key` | `mode=0`, `keysize=4`, counts 0,1,2,7,8,9 → crosses the 8-slot bucket and the 3/4 growth threshold | [x] |
| 12 | `stbds_hmput_key` / `hmget_key` | `mode=0`, `keysize=4`, 500 random keys with duplicates (overwrite path + several rehashes) | [x] |
| 13 | `stbds_hmput_key` / `hmget_key` | `mode=0`, `keysize=8`, `elemsize=16` (`{size_t key; size_t value;}`), 300 random keys | [x] |
| 14 | `stbds_hmput_key` / `hmget_key` | `mode=0`, `keysize=16`, `elemsize=32` (2-word key, `stbds_struct2`-like), 300 random keys | [x] |
| 15 | `stbds_hmput_key` / `hmget_key` | `mode=0`, odd `keysize ∈ {1,2,3,5,6,7}` with matching `elemsize` (siphash tail cases inside a live map) | [x] |
| 16 | `stbds_hmget_key_ts` | `a=NULL` first call; then `_ts` lookups of present/absent keys, checking both `*temp` and that `header->temp` is **not** touched (contrast with `hmget_key`) | [x] |
| 17 | `stbds_hmget_key` | `hash_table == NULL` state (array built by `hmput_default` only) → `temp = -1` | [x] |
| 18 | `stbds_hmput_default` | `a=NULL`; then `a` non-NULL with `length>0` (no-op); `elemsize ∈ {8,16,32}` | [x] |
| 19 | `stbds_hmput_default` + `hmput_key` + `hmget_key` | default element present, then normal puts/gets (index base offset by the default slot) | [x] |
| 20 | `stbds_hmdel_key` | `mode=0`, `keyoffset=0`, delete existing / missing keys interleaved with puts, 300 random ops | [x] |
| 21 | `stbds_hmdel_key` | `mode=0`, delete enough to cross the **shrink** threshold (`used_count < slot_count/4`, `slot_count>8`) | [x] |
| 22 | `stbds_hmdel_key` | `mode=0`, put/delete churn crossing the **tombstone** threshold (`> 3/16 slot_count`) → same-size rebuild | [x] |
| 23 | `stbds_hmdel_key` | `mode=0`, `keyoffset != 0` (`{int b; int key;}`-style layout, key at offset 4/8) | [x] |
| 24 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` | `mode=1` (`STBDS_HM_STRING`) with the implicit `SH_DEFAULT` arena mode: caller-owned `char*` keys, 200 random strings, gets + deletes | [x] |
| 25 | `stbds_shmode_func(elemsize, SH_STRDUP=2)` then `hmput_key(mode=1)` | strdup'ed keys, put/get/delete (delete must `free` the dup), then `hmfree_func` (frees every key) | [x] |
| 26 | `stbds_shmode_func(elemsize, SH_ARENA=3)` then `hmput_key(mode=1)` | arena-allocated keys, 200 strings of mixed length incl. > blocksize, then `hmfree_func` (`strreset`) | [x] |
| 27 | `stbds_shmode_func(elemsize, SH_NONE=0)` then `hmput_key(mode=1)` | `string.mode==0` → `default:` branch = raw `memcpy(keysize)` even though `mode>=STRING`; compare resulting element bytes | [x] |
| 28 | `stbds_shmode_func(elemsize, SH_DEFAULT=1)` then `hmput_key(mode=1)` | explicit `SH_DEFAULT`, pointer-store path | [x] |
| 29 | `hmput_key`/`hmget_key`/`hmdel_key` with `mode=2` (`PTR_TO_STRING`) | `mode >= STRING` for hash/compare, but `mode != STRING` in `hmdel_key` → no strdup free, binary key re-find; combined with `string.mode=STRDUP` and `=DEFAULT` | [x] |
| 30 | `hmput_key`/`hmget_key` with out-of-range `mode` (`3`, `1000`, `-1`, `INT_MIN`) | `mode >= 1` string path for `3`/`1000`; `mode <= 0` binary path for `-1`/`INT_MIN` | [x] |
| 31 | `stbds_shmode_func` with out-of-enum `mode` (`4`, `255`, `256`, `259`, `-1`) | `(unsigned char)mode` truncation; subsequent `hmput_key` `switch` behaviour | [x] |
| 32 | `stbds_hmfree_func` | `string.mode ∈ {NONE, DEFAULT, STRDUP, ARENA}` × `hash_table` NULL / non-NULL × `length` 1 / many | [x] |
| 33 | `stbds_stralloc` | fresh zeroed arena, strings of length 1, 10, 511, 512, 513 → first-block sizing and the `len > blocksize` path | [x] |
| 34 | `stbds_stralloc` | many sequential allocations driving `a->block` 0→22 (blocksize 512 … 1<<20) and its saturation; compare `remaining`, `block`, and the block chain shape | [x] |
| 35 | `stbds_stralloc` | interleaved small/huge (`> blocksize`) strings so the over-sized block is spliced after the head block | [x] |
| 36 | `stbds_stralloc` + `stbds_strreset` | reset of an arena with 0, 1 and N blocks; arena struct must be fully zeroed | [x] |
| 37 | `strkey` | `n ∈ {0, 1, 9, 10, 99, 12345, -1, -9, -12345, INT_MAX, INT_MIN}` and repeated calls (shared static buffer) | [x] |
| 38 | `arr_ins` | `num ∈ {0, 1, 4, -1, INT_MAX, INT_MIN, random}` — runs to completion without aborting in either library (all 5 insert positions) | [x] |
| 39 | end-to-end mixed workload | random op stream (put / get / get_ts / del / put_default) over `mode ∈ {0,1,2}` × `string.mode ∈ {NONE,DEFAULT,STRDUP,ARENA}`, comparing the full array payload, `header->{length,capacity,temp}` and every `hash_index` scalar field after each op | [x] |
| 40 | `stbds_arrgrowf` interop with the map | `hmput_key` growth path where `arrgrowf` reallocates the element array while the hash index stays put (`hash_table` pointer preserved) | [x] |

No `[features]` section exists in `translation/Cargo.toml`, so there is exactly
one feature combination (the default, empty one); every row is verified under it.

## Test coverage (Phase B)

Rows 1-9 and 33-38 are covered by `tests/phase_b_lowlevel.rs`
(`row01_*` … `row09_*`, `row33_*` … `row38_*`); rows 10-32, 39 and 40 by
`tests/phase_b_map.rs` (`row10_*` … `row32_*`, `row39_*`, `row40_*`).  The test
name carries the row number, so `cargo test --release row21` runs exactly that
row.  Each row drives BOTH `.so`s through the same randomised sequence (fixed
PRNG seeds, xorshift64*) and compares, after every single operation:

* the array header (`length`, `capacity`, `temp`, whether `hash_table` is set),
* all `length * elemsize` payload bytes (with `char*` key words blanked and the
  pointed-to strings compared separately),
* every scalar of `stbds_hash_index` (`slot_count`, `used_count`, all three
  thresholds, `tombstone_count`, `seed`, `slot_count_log2`), the whole
  `string` arena (`remaining`, `block`, `mode`, block-chain length), and every
  `hash[8]` / `index[8]` entry of every bucket,
* the return value / `*temp` out-parameter of the call itself, and
  `stbds_temp_key` where the C code writes it.

The project builds no binary/driver (`c_src/CMakeLists.txt` only declares
`add_library(... SHARED)`, and the crate declares only
`crate-type = ["cdylib"]`), so there is no stdout comparison to make.

Run everything (all rows, all configurations) with `./verify.sh`.
