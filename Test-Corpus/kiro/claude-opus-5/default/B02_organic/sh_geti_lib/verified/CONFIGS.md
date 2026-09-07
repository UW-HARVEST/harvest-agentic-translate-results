# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branch points in `c_src/src/lib.c`. There are no
Cargo features and no `#ifdef` compile-time options in the C (the `#define`s at
the top are all unconditional), so the configuration surface is entirely
**runtime**.

## Axes the C actually branches on

**A1. `mode` argument** (`stbds_hmget_key`, `stbds_hmget_key_ts`,
`stbds_hmput_key`, `stbds_hmdel_key`). Two distinct comparisons exist:
`mode >= STBDS_HM_STRING(1)` (in `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_hmput_key`) and `mode == STBDS_HM_STRING` (twice in
`stbds_hmdel_key`). ⇒ four meaningful classes: `mode < 0`, `mode == 0`
(BINARY), `mode == 1` (STRING), `mode >= 2` (out-of-range enum: string-ish for
hashing/compare, binary-ish inside `hmdel_key`).

**A2. table `string.mode`** — set either implicitly by `stbds_hmput_key`
(`SH_NONE` for binary, `SH_DEFAULT` for string) or explicitly by
`stbds_shmode_func(elemsize, mode)` which does `(unsigned char) mode`.
The `switch` in `stbds_hmput_key` distinguishes `SH_STRDUP(2)`, `SH_ARENA(3)`,
`SH_DEFAULT(1)` and `default:` (`SH_NONE(0)` **and every out-of-range value**
≥ 4, which take the `memcpy` path).

**A3. element size / key size** — `elemsize` and `keysize` are free
parameters. Branch-relevant shapes: `keysize == elemsize` (pure-key element),
`keysize < elemsize` (key + payload), `keysize == 0`, `keysize == 8`
(pointer-sized, what the `sh*` macros use), `keysize == 16`, and sizes that
are/aren't multiples of 8 (drives the `stbds_hash_bytes` tail `switch`).

**A4. `keyoffset`** — only `stbds_hmdel_key` takes it (`0` from the `sh*`
macros, non-zero from `STBDS_OFFSETOF` on structs where `key` isn't first).

**A5. table size / load** — `slot_count` starts at 8 and doubles when
`used_count >= slot_count - slot_count/4`; halves when
`used_count < slot_count/4 && slot_count > 8`; rebuilds in place when
`tombstone_count > slot_count/8 + slot_count/16`. ⇒ element counts must span
the growth (6, 7, 12, 24, 48, 100, 1000) and the shrink/rebuild thresholds.

**A6. `stbds_arrgrowf` shape** — `a == NULL` vs not; `min_cap <= arrcap`
(early return) vs `min_cap < 2*arrcap` vs `min_cap < 4`; `addlen == 0` vs `> 0`.

**A7. hash seed** — `stbds_rand_seed` mutates the file-static
`stbds_hash_seed`; every `stbds_make_hash_index(_, NULL)` consumes it and
advances it by `seed = seed*a + b`. So seed *state* is an axis, and it must be
re-synchronised between the two `.so`s before every comparison.

**A8. string-arena shape** (`stbds_stralloc` / `stbds_strreset`) —
`len <= remaining` (fast path) vs `len > remaining` with `len <= blocksize`
(new block) vs `len > blocksize` (dedicated block) × `storage == NULL` vs not;
plus `a->block` growth up to the `1<<20` saturation.

**A9. `stbds_hash_bytes` input shape** — `len` 0 / 1..7 / 8 / 9..15 / 16 /
large; bytes with the high bit set at offset 3 and 7 of each word
(sign-extension quirk); `seed` = 0 / `SIZE_MAX` / random.

**A10. `sh_geti(num)`** — `num` ≤ 0, 1, 2, 3, 4, small odd, small even, large;
covers both `j` iterations (`SH_STRDUP` then `SH_ARENA`).

## Rows (each = one differential test case)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len == 0`, random seeds || [x] |
| 2 | `stbds_hash_bytes` | `len` 1..7 (tail-only), random bytes incl. high-bit, random seeds || [x] |
| 3 | `stbds_hash_bytes` | `len == 8` (exactly one word), random bytes, seeds 0 / `SIZE_MAX` / random || [x] |
| 4 | `stbds_hash_bytes` | `len` 9..15 (one word + tail 1..7) || [x] |
| 5 | `stbds_hash_bytes` | `len` 16..256, multiples and non-multiples of 8 || [x] |
| 6 | `stbds_hash_bytes` | bytes forced to have the high bit set at every offset ≡ 3 or 7 (mod 8) — sign-extension path || [x] |
| 7 | `stbds_hash_string` | `""`, 1-char, ASCII, bytes ≥ 0x80, length 1..200, random seeds || [x] |
| 8 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to 0 / 1 / `SIZE_MAX` / random, then observe the seed advance through repeated table creation || [x] |
| 9 | `stbds_arrgrowf` | `a == NULL`, `addlen`/`min_cap` ∈ {0,1,2,3,4,5,17,1000} × `elemsize` ∈ {1,4,8,16,24} || [x] |
| 10 | `stbds_arrgrowf` | non-NULL `a`, `min_cap <= arrcap` (early-return branch) || [x] |
| 11 | `stbds_arrgrowf` | non-NULL `a`, doubling branch (`min_cap < 2*arrcap`) || [x] |
| 12 | `stbds_arrgrowf` | non-NULL `a`, `min_cap > 2*arrcap` (explicit large cap) || [x] |
| 13 | `stbds_arrgrowf` → `stbds_arrfreef` | grow chain then free (non-NULL only) || [x] |
| 14 | `stbds_hmput_default` | `a == NULL` || [x] |
| 15 | `stbds_hmput_default` | called twice (second call is a no-op, `length != 0`) || [x] |
| 16 | `stbds_hmput_default` then `stbds_hmget_key` | default set, table still NULL ⇒ `temp == -1` || [x] |
| 17 | `stbds_hmput_key` BINARY (`mode=0`) | `elemsize=16, keysize=8`, 1 key || [x] |
| 18 | `stbds_hmput_key` BINARY | `elemsize=16, keysize=8`, N = 6,7,12,24,48,100,1000 random `u64` keys (crosses every growth threshold) || [x] |
| 19 | `stbds_hmput_key` BINARY | `keysize=4`, `elemsize=8` (int keys) || [x] |
| 20 | `stbds_hmput_key` BINARY | `keysize=16`, `elemsize=32` (two-word struct key, `stbds_struct2` shape) || [x] |
| 21 | `stbds_hmput_key` BINARY | `keysize == elemsize` (no payload) || [x] |
| 22 | `stbds_hmput_key` BINARY | `keysize == 0` (degenerate) || [x] |
| 23 | `stbds_hmput_key` BINARY | duplicate keys re-put (hits the "found existing" return in the *first* inner loop and, for offset keys, the second) || [x] |
| 24 | `stbds_hmput_key` STRING (`mode=1`) via `stbds_shmode_func(SH_DEFAULT)` | caller-owned keys, N = 1,6,12,48,200 || [x] |
| 25 | `stbds_hmput_key` STRING via `stbds_shmode_func(SH_STRDUP)` | keys copied with `stbds_strdup`, N = 1,6,12,48,200 || [x] |
| 26 | `stbds_hmput_key` STRING via `stbds_shmode_func(SH_ARENA)` | keys copied into the arena, N = 1,6,12,48,200, incl. strings > 512 bytes || [x] |
| 27 | `stbds_hmput_key` STRING, table auto-created (`hash_table == NULL`, no `shmode_func`) | `string.mode` becomes `SH_DEFAULT` implicitly || [x] |
| 28 | `stbds_hmput_key` | `shmode_func` with out-of-range mode `4` / `255` ⇒ `default:` `memcpy` branch even though `mode >= 1` || [x] |
| 29 | `stbds_hmput_key` | `mode = 2` and `mode = INT_MAX` (out-of-range enum, string-ish hashing) || [x] |
| 30 | `stbds_hmput_key` | `mode = -1` and `mode = INT_MIN` (out-of-range enum, binary-ish) || [x] |
| 31 | `stbds_hmget_key` BINARY | present keys, absent keys, and keys differing only in the last byte || [x] |
| 32 | `stbds_hmget_key_ts` BINARY | same as 31 but reading `*temp` instead of `header->temp`; `a == NULL` bootstrap || [x] |
| 33 | `stbds_hmget_key` STRING | present / absent / prefix / empty-string keys, all three `string.mode`s || [x] |
| 34 | `stbds_hmget_key_ts` STRING | same, `temp` out-param || [x] |
| 35 | `stbds_hmdel_key` BINARY, `keyoffset = 0` | delete present key (last element), delete non-last element (triggers relocation), delete absent key || [x] |
| 36 | `stbds_hmdel_key` BINARY, `keyoffset != 0` | key not at element offset 0 (`elemsize=24, keyoffset=8, keysize=8`) || [x] |
| 37 | `stbds_hmdel_key` STRING (`mode=1`) `SH_DEFAULT` | delete with relocation || [x] |
| 38 | `stbds_hmdel_key` STRING (`mode=1`) `SH_STRDUP` | delete frees the duplicated key || [x] |
| 39 | `stbds_hmdel_key` STRING (`mode=1`) `SH_ARENA` | delete does *not* free (arena owns) || [x] |
| 40 | `stbds_hmdel_key` `mode = 2` | out-of-range enum: `mode >= 1` for hashing but `mode == 1` is false ⇒ no strdup-free and raw-address relocation lookup || [x] |
| 41 | `stbds_hmdel_key` | delete-all in insertion order (walks the shrink path repeatedly) || [x] |
| 42 | `stbds_hmdel_key` | delete-all in reverse order || [x] |
| 43 | `stbds_hmdel_key` | random interleaved put/get/del, N = 500 ops, fixed seed (tombstone rebuild + shrink + regrow) || [x] |
| 44 | `stbds_hmdel_key` | delete from a table whose `hash_table` is NULL (after `hmput_default` only) || [x] |
| 45 | `stbds_hmfree_func` | `SH_NONE` table (binary) || [x] |
| 46 | `stbds_hmfree_func` | `SH_DEFAULT` table || [x] |
| 47 | `stbds_hmfree_func` | `SH_STRDUP` table (frees every element key) || [x] |
| 48 | `stbds_hmfree_func` | `SH_ARENA` table (calls `stbds_strreset` on the arena) || [x] |
| 49 | `stbds_hmfree_func` | array with `hash_table == NULL` || [x] |
| 50 | `stbds_shmode_func` | `elemsize` ∈ {8,16,24,32}, `mode` ∈ {0,1,2,3,4,255,-1} — observe `string.mode` and the initial `length`/`temp` || [x] |
| 51 | `stbds_stralloc` | fresh arena, strings shorter than 512 (fast/new-block paths), 1..300 strings || [x] |
| 52 | `stbds_stralloc` | fresh arena, first string longer than 512 (`len > blocksize`, `storage == NULL` ⇒ `remaining = 0`) || [x] |
| 53 | `stbds_stralloc` | arena with an existing block, then a string longer than `blocksize` (`storage != NULL` splice-after path) || [x] |
| 54 | `stbds_stralloc` | enough allocations to walk `a->block` from 0 up through the `1<<20` saturation || [x] |
| 55 | `stbds_stralloc` | empty strings `""` only, many of them (`len == 1`) || [x] |
| 56 | `stbds_strreset` | empty arena; arena with 1 block; arena with many blocks || [x] |
| 57 | `strkey` | `n` ∈ {0, 1, 9, 10, 99, 12345, -1, `INT_MIN`, `INT_MAX`} || [x] |
| 58 | `sh_geti` (full end-to-end driver, stdout compared) | `num` = 0 || [x] |
| 59 | `sh_geti` | `num` = 1, 2, 3, 4, 5 || [x] |
| 60 | `sh_geti` | `num` = 7 (odd), 8, 16, 17, 33 (crosses several table growths) || [x] |
| 61 | `sh_geti` | `num` = 100, 257 (arena blocks > 512 bytes of keys, many shrinks) || [x] |
| 62 | `sh_geti` | `num` < 0 (`-1`, `INT_MIN`) || [x] |
| 63 | `sh_geti` | after `stbds_rand_seed(s)` for several `s` (different hash order ⇒ different stdout order) || [x] |

There is **no binary/driver target** in this project (`CMakeLists.txt` builds
only `add_library(... SHARED)`; `Cargo.toml` declares only a `cdylib`), so the
"compare C and Rust stdout" requirement is satisfied by capturing the `stdout`
that `sh_geti` writes from inside each `.so` (rows 58–63).

## Where each row is tested

| rows | test file |
|------|-----------|
| 1–13 | `tests/phase_b_low.rs` |
| 14–50 | `tests/phase_b_map.rs` |
| 51–57 | `tests/phase_b_arena.rs` |
| 58–63 | `tests/phase_b_shgeti.rs` |

All 63 rows pass with randomized inputs at fixed seeds, against both the
`debug` and the `release` Rust `.so`.

## Harness properties worth knowing

* **`cargo test` does not rebuild a `cdylib`.** The harness therefore refuses
  to run if `target/*/libsh_geti_lib.so` is older than `src/lib.rs`
  (`tests/common/mod.rs::rust_so_path`). Without that guard the whole suite
  silently validates a stale library — this was caught during mutation testing.
* **`stdout` capture forks.** libtest prints its own `test ... ok` lines from
  another thread, and an in-process `dup2` of fd 1 captures those too. Capture
  therefore runs the closure in a forked child. A sequence that relies on the
  global hash seed advancing must run inside ONE capture call.
* **Both libraries hold mutable global state** (`stbds_hash_seed`) and share
  fd 1, so every test takes a process-wide mutex and calls `stbds_rand_seed`
  on both libraries before comparing.
* **Pointer values are never compared across libraries.** The two `.so`s have
  separate allocation histories, so comparisons use structural equivalents:
  string *contents* for keys, chain-index + offset for arena pointers, and a
  poked-sentinel classification for the uninitialised `temp_key` field.

## Deliberately not compared (and why)

* `table->temp_key` raw value — `stbds_make_hash_index` never initialises it
  and never copies it on rehash, so it is genuine garbage until a string-mode
  put assigns it. Compared as `Sentinel / Elem(i) / Other` instead.
* Element bytes outside `[0, keysize)` and the test-written payload — the C
  only `memcpy`s `keysize` bytes, so the rest of a freshly grown element is
  uninitialised in both.
* Whether `realloc` returns the same address — an allocator artifact. The
  *semantic* early-return decision (`max(min_cap, len+addlen) <= capacity`) is
  compared instead.

## Verification limits (honest gaps)

Two behaviours are provably not observable through the exported surface, and
are noted rather than claimed as covered:

1. **`hash < 2 ? hash += 2` in `stbds_hm_find_slot` / `stbds_hmput_key`.**
   Reaching it needs an input whose 64-bit hash is exactly 0, 1 or 2. Both
   hash functions are exported and were shown to agree across every length
   0–256, every tail length, all-`0x00`/`0xFF`/`0x80` fills, high-bit-at-
   offset-3/7 patterns and 6 seeds, so the branch is verified by hash equality
   rather than by hitting it. Confirmed computationally unreachable, not
   overlooked.
2. **Which `string.mode` `sh_geti` picks on each of its two passes.**
   `SH_STRDUP` and `SH_ARENA` produce byte-identical stdout (the printed key
   *contents* and value do not depend on where the key is stored), and
   `sh_geti` returns `void` and frees everything. Swapping the two modes inside
   `sh_geti` is therefore invisible from outside. The *behaviour* of each mode
   is still verified directly via `stbds_shmode_func` plus a full
   put/get/del/free cycle (rows 24–26, 37–39, 45–48, 50).

## Mutation testing (harness sensitivity)

Because every row passed on the first run, the harness itself was validated by
injecting 30 deliberate bugs into `translation/src/lib.rs`, rebuilding, and
re-running. Results:

* **26 caught** — including both siphash sign-extension casts, the
  `hash_string` mixing step, `probe_position`, `log2`, `load_32_or_64`, both
  probe-step growth rules, the growth / shrink / tombstone-rebuild thresholds,
  the `min_cap < 4` and `min_cap < 2*cap` clamps, `arrgrowf`'s header init,
  `bucket->index = i-1`, `final_index`, `temp = 1` on a successful delete,
  the `hmget_key_ts` bootstrap sentinel, `(unsigned char) mode` truncation,
  `shmode_func`'s initial length, `hmput_default`'s zeroing, the arena block
  saturation and splice order, `stralloc`'s pointer arithmetic, `strkey`'s
  format string, the seed-advance constants, siphash's round counts, the
  `stbds_temp_key` second-probe-loop quirk, the `mode == 1` exact tests in
  `hmdel_key`, and `hmfree_func`'s loop start index.
* **4 semantically equivalent** (correctly not caught, verified by hand):
  `min_len > min_cap` → `>=` (the assignment is a no-op at equality);
  `strreset` zeroing 23 instead of 24 bytes (bytes 18–23 are struct padding);
  and the two limits listed in the section above.

Three real coverage gaps were found this way and closed: the `2*cap <= min_cap
< 3*cap` window in `stbds_arrgrowf`, the strdup-key free in `stbds_hmdel_key`
for `mode >= 2`, and `stbds_hmfree_func`'s loop start index.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. Verified by
`cargo read-manifest | jq .features` → `{}`; `cargo check` and
`cargo check --no-default-features` are both run by `verify.sh`.
Both the dev and release profiles are exercised: the full suite runs twice,
once with `RUST_SO` pointing at `target/debug/libsh_geti_lib.so` and once at
`target/release/libsh_geti_lib.so` (which is the `panic = "abort"` build).

### C build configuration

The ground-truth C build is the one given in the task (no `CMAKE_BUILD_TYPE`,
hence **no `-DNDEBUG`**, hence `assert` is live). The suite was additionally
run against a `-DCMAKE_BUILD_TYPE=Release` C build: 27 of 28 tests still pass,
which shows the C's `int`-overflow UB in `stbds_siphash_bytes` is not compiled
differently at `-O2`. The one failure is `abort_parity`, because `-DNDEBUG`
removes the `assert` the C would otherwise die on — a build-configuration
difference, not a translation divergence. The Rust's `stbds_assert!` always
aborts, matching the specified (non-`NDEBUG`) C build.
