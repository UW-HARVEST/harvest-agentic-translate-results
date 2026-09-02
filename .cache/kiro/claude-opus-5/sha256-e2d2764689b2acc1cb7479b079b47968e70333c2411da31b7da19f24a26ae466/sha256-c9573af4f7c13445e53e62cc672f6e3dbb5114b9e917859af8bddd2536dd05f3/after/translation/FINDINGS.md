# FINDINGS.md — divergences found and fixed

The C source in `c_src/` is the ground truth; every fix below changed only
`translation/src/lib.rs`. Nothing in `c_src/` was modified (only `c_src/build/`
was created by CMake, as instructed).

## 1. Non-wrapping integer arithmetic (real divergence, fixed)

`src/lib.rs` used plain Rust operators in ~24 places where the C original relies
on `size_t` / `int` **wrap-around**. With overflow checks enabled (any debug
build of the `cdylib`) those operators panic, whereas the C code wraps silently.

The divergence was caught by `err04_used_count_underflow_does_not_abort` running
against the *debug* `.so`: `stbds_hmdel_key` does

```c
--table->used_count;
STBDS_ASSERT(table->used_count >= 0);   /* size_t: always true */
```

so a `used_count` of 0 wraps to `SIZE_MAX` and the assert can never fire. The
Rust `(*table).used_count -= 1` aborted with an overflow panic instead. Release
builds hid this because `overflow-checks` defaults to off there.

Fixed by switching to explicit wrapping operations. Sites (grouped):

| function | expressions changed |
|----------|--------------------|
| `stbds_make_hash_index` | hash-index allocation size, `used_count_threshold`, `tombstone_count_threshold`, the assert's sum |
| `stbds_siphash_bytes` | block-loop bound `i + 8`, `i += 8`, tail `len - i` |
| `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_hmput_key` | `length += 1` on a fresh array |
| `stbds_hmput_key` | `slot_count * 2`, `--tombstone_count`, `++used_count`, `length = i + 1`, `index = i - 1`, `temp = i - 1` |
| `stbds_hmdel_key` | `final_index = arrlen - 1 - 1`, `--used_count`, `++tombstone_count`, `length -= 1` |
| `stbds_hmfree_func` | loop counter |
| `stbds_strdup`, `stbds_stralloc` | `strlen + 1`, both block allocation sizes, `remaining -= len` |
| `arr_push` | `length + 1`, `length = idx + 1`, `j += 1`, `i += 50` (`int` overflow is UB in C but wraps on gcc/x86-64) |

Left alone deliberately, because overflow is impossible: `hash += 2` (guarded by
`hash < 2`), the bucket-scan counters `z`/`i` (bounded by 8), `stbds_log2`'s `n`
(bounded by 64), and `c_strlen`'s counter.

## 2. Things that looked like divergences but were not

* **`stbds_hash_index::temp_key` is never initialised.** `stbds_make_hash_index`
  sets every field except `temp_key`, on fresh tables *and* on rehash. It is only
  written by `stbds_hmput_key` on an insert (`string.mode` 1/2/3) or on a hit
  found in the first bucket scan. Comparing it at any other moment compares
  indeterminate heap bytes; the first version of the harness did exactly that and
  produced a spurious mismatch (`Some("xxxxxxxx!")` vs `None`) plus a SIGSEGV.
  It is now excluded from the structural snapshot and checked only right after an
  insert.
* **Element bytes outside the key are indeterminate straight out of
  `stbds_hmput_key`.** The `STBDS_SH_NONE` arm `memcpy`s only `keysize` bytes;
  the C macros then assign the whole struct. The harness now writes the complete
  element, mirroring `stbds_hmputs`.
* **`stbds_string_arena::storage` is a pointer**, so it necessarily differs
  between the two libraries. Compared as "null / non-null" plus the scalar
  fields.

## 3. C behaviour deliberately reproduced rather than "fixed"

* `STBDS_ASSERT(table->used_count >= 0)` on a `size_t` — a no-op; the Rust
  translation omits it, which is observationally identical.
* `stbds_hmdel_key` tests `mode == STBDS_HM_STRING` **exactly** while
  `stbds_is_key_equal` tests `mode >= STBDS_HM_STRING`. With `mode >= 2` the
  string comparison path is taken but the strdup'd key is not freed and the
  hole-filling re-lookup hands raw element bytes to `stbds_hash_string`, which
  fails and trips `STBDS_ASSERT(slot >= 0)`. Both libraries abort identically
  (`err05_hmdel_relookup_assert`).
* `stbds_stralloc`'s `512 << (a->block >> 1)` has an unbounded shift count. On
  x86-64 the count is masked to 63, so `block >= 110` yields `blocksize == 0`.
  Rust reproduces this with `wrapping_shl`, verified for every `block` value
  whose blocksize is small enough to allocate (`row50`, `err50`).
* `stbds_shmode_func` stores `(unsigned char)mode`, so `256 → 0` and `-1 → 255`.
  Reproduced and asserted (`row42`, `err48`).
* The `d[3] << 24` / `d[7] << 24` expressions in `stbds_siphash_bytes` are `int`
  expressions that sign-extend into the upper half of `size_t`. Reproduced, and
  covered specifically by `row05_hash_bytes_high_bit`.

## 4. Harness-integrity bug found by a negative control (fixed)

`cargo test` rebuilds the cdylib into `target/<profile>/deps/libarr_push_lib.so`
but only `cargo build` refreshes the uplifted copy at
`target/<profile>/libarr_push_lib.so`. The first version of the harness loaded
the uplifted copy, so a plain `cargo test` could compare the C library against a
**stale** Rust `.so` — every test would pass no matter what changed in
`src/lib.rs`. Two fixes:

* `tests/common/mod.rs` now enumerates all candidate paths and loads the
  **most recently modified** one (`RUST_SO` still overrides).
* `Cargo.toml` gained `"rlib"` alongside `"cdylib"` so the integration test
  targets have a dependency edge on the library and cargo rebuilds it. The
  exported symbol set is unchanged (16/16, verified before and after).

This was caught by deliberately injecting a one-bit change into
`stbds_hash_string` and observing that the tests still passed.

### Negative-control results (after the fix)

Each injection was applied to `src/lib.rs`, tested, then reverted:

| injected change | expected to break | result |
|-----------------|-------------------|--------|
| `stbds_hash_string`: `hash << 6` → `hash << 7` | string hashing | row07, row35, row41, err34/35 FAIL; row17, err27 unaffected (correct — they never hash a string) |
| `stbds_siphash_bytes`: `v2 ^= 0xff` → `0xfe` | binary hashing | row03, row28, err27 FAIL |
| `STBDS_STRING_ARENA_BLOCKSIZE_MIN` 512 → 511 | arena | row47, err38/39/40/41 FAIL |
| `stbds_arrgrowf`: `min_cap = 4` → `5` | array growth | row10, row15, err23/24/25 FAIL |
| `stbds_hmdel_key`: `INDEX_DELETED` → `INDEX_EMPTY` | tombstones | row32, err36/37 FAIL |
| `make_hash_index`: LCG low word `0x87b0b0fd` → `…fc` | per-table seed | row08, row52 FAIL |
| `make_hash_index`: `t->seed = hash_seed` → `+1` | per-table seed | row08, row21 FAIL |

One injection turned out to be a genuine no-op rather than a coverage gap:
changing the `v32` argument of `stbds_load_32_or_64(2147001325, hi, lo)` cannot
change the result, because the macro computes
`(hi << 32) ^ (low32(lo ^ v32) ^ v32)` and `v32` cancels for `lo < 2^32`.

## 5. Verification status

* `SYMBOLS.md` — 16/16 exported symbols match; symbol diff empty in both
  directions; no unexpected undefined non-libc symbols.
* `CONFIGS.md` — 53/53 rows pass with randomized inputs.
* `ERRORS.md` — 50/50 rows have a passing error-path differential test.
* Every one of the above holds for `debug` **and** `release`, and for both
  feature configurations the crate admits (default and `--no-default-features`;
  no `[features]` table exists).

Reproduce everything with `cd translation && ./verify.sh`.
