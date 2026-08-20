# VERIFICATION.md — how to reproduce, and what was found

The C in `c_src/` is the ground truth. `src/lib.rs` is the Rust translation.
Both are compiled to shared objects and compared **only through `dlopen`/`dlsym`**
— the Rust crate is never linked directly, so the `#[no_mangle] extern "C"`
export wrappers are themselves under test.

## Reproduce

```sh
# everything: enumerates the build-time configuration surface, builds the C .so,
# checks symbol parity for both Rust profiles, and runs the whole suite per combo
./scripts/check_all_features.sh
```

or manually:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..
cargo build --release          # the fatal-path tests need the release cdylib
cargo test
```

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so` and its Rust counterpart |
| `ERRORS.md` | the error-surface table: 61 rows, one per distinct rejection |
| `CONFIGS.md` | the configuration-surface table: 67 rows of valid-input combinations |

## Test suite

114 test functions across 10 integration-test binaries (~6 700 lines):

| file | covers |
|------|--------|
| `tests/common/mod.rs` | library loading, fixed-seed RNG, structural snapshotting, `fork()`-based fatal-path comparison |
| `tests/common/map.rs` | a faithful re-implementation of the `stbds_hmput`/`hmget`/`hmdel`/`hmdefault` **macros** on top of the raw exported functions, so the composed pipeline is exercised, not just individual wrappers |
| `tests/symbols.rs` | Phase A/D symbol parity |
| `tests/hash.rs` | CONFIGS 1-12 + exhaustive small-input sweeps |
| `tests/arr.rs` | CONFIGS 13-19 |
| `tests/map_binary.rs` | CONFIGS 20-39 |
| `tests/map_string.rs` | CONFIGS 40-56 |
| `tests/arena.rs` | CONFIGS 57-64 |
| `tests/crosscut.rs` | CONFIGS 65-67 (ABI layout, global-seed lockstep) |
| `tests/errors.rs`, `tests/errors2.rs` | ERRORS rows with a non-fatal result |
| `tests/errors_fatal.rs` | ERRORS rows whose result is a fatal signal, + a generic NULL/out-of-range FFI sweep (82 scenarios), + the harness self-test |

### What "compared" means

State-machine operations are compared with a full structural snapshot **after
every single operation** (`snap_map`): the array header (`length`, `capacity`,
`temp`, whether a hash table exists), every element's bytes, the entire
`stbds_hash_index` (all six counters/thresholds, `seed`, `slot_count_log2`, the
`storage` alignment relation, the whole embedded `stbds_string_arena`) and every
bucket's `hash[8]` and `index[8]`. String keys are compared by dereferenced
content, since `SH_STRDUP`/`SH_ARENA` keys live at different addresses in the two
processes.

Exhaustive (not sampled) sweeps: all 1-, 2- and 3-byte buffers through
`stbds_hash_bytes` (16.8 M inputs), all 4-byte buffers with the `d[3] << 24`
`int`-overflow boundary bytes, all 1- and 2-character strings through
`stbds_hash_string`, all 256 byte values in every siphash tail position, and
`stbds_stralloc` for every `stbds_string_arena::block` in `0..=30` and `110..=127`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing symbols. The dynamic symbol
      sets are *identical in both directions* (no C-only, no Rust-only) for
      `target/debug/libarr_push_lib.so` **and** `target/release/libarr_push_lib.so`.
      0 unresolved non-libc/non-libgcc imports. No symbol was stubbed: every one
      of the 16 has a real translated implementation.
- [x] **Phase B** — all 67 `CONFIGS.md` rows pass across randomized inputs
      (fixed seeds, reproducible).
- [x] **Phase C** — all 61 `ERRORS.md` rows have a passing differential test,
      asserting the *same* error code / sentinel / signal, not merely "both
      failed". Plus 82 generic FFI-boundary scenarios (NULL pointers, zero and
      oversized lengths, integer-overflowing sizes, and out-of-range `mode`
      enum values including negatives, `> 3`, `256`, `INT_MIN` and `INT_MAX`).
- [x] **Phase D** — the build-time configuration surface is a single
      configuration, established mechanically: `Cargo.toml` has no `[features]`
      table, `c_src/CMakeLists.txt` has no `option()`/conditional, and the C
      sources contain **0** `#ifdef`/`#ifndef`/`#if` directives.
      `scripts/check_all_features.sh` enumerates the (one-element) feature power
      set and runs `cargo check`, `cargo build` (dev **and** release), the symbol
      diff, and the full suite for it. All green.

## Findings

### No behavioural divergence was found in the translation

Every C quirk that the tests probe is reproduced exactly, including several that
a "tidied up" translation would get wrong:

1. **`stbds_hmput_key`'s asymmetric `temp_key` update** (`lib.c:732-733` vs
   `746-751`). On a duplicate put, the *forward* probe half refreshes
   `stbds_hash_index::temp_key`; the *wrap-around* half does not. Verified with
   an exact replay of the probe loop to classify which half resolved each lookup
   (604 wrap-half and 2 836 forward-half duplicate puts observed) — see
   `map_string.rs::cfg53_wraparound_duplicate_put_temp_key`.
2. **`stbds_hmdel_key`'s `mode == STBDS_HM_STRING` vs `find_slot`'s
   `mode >= STBDS_HM_STRING`** (`lib.c:842` vs `590`). For any `mode >= 2`,
   deleting a non-last element passes `find_slot` the *address* of the key
   pointer, which then hashes those pointer bytes as a string, never finds the
   slot, and trips `STBDS_ASSERT(slot >= 0)`. Both implementations abort with
   `SIGABRT` for `mode` = 2, 3, 100 and `INT_MAX`, and both succeed for
   `mode == 1`.
3. **The strdup key is leaked for `mode >= 2`** (`lib.c:836`) because that free is
   also gated on `mode == 1` exactly.
4. **Signed-`int` overflow in the siphash tail** (`lib.c:536-538`):
   `data |= (d[3] << 24)` overflows `int` for `d[3] >= 0x80`, and the negative
   result is sign-extended into the `size_t` accumulator. Reproduced, and
   verified exhaustively over all 16.8 M 3-byte inputs plus every byte value in
   every tail position.
5. **`512u << (block>>1)` wrapping to 0** for `stbds_string_arena::block >= 110`
   (well defined for unsigned in C), which flips `stralloc` onto its oversize
   path.
6. **`stbds_stralloc`'s oversize splice** inserts the new block *after* the head
   and leaves `remaining` untouched when `storage != NULL`, but sets
   `remaining = 0` when `storage == NULL`.
7. **`hmfree_func`'s strdup loop starts at index 1**, never freeing the default
   slot. Proven by planting an interior pointer there and confirming neither
   library aborts.
8. **`keysize == 0`** makes `memcmp(...,0)` always equal, collapsing every key
   onto one entry.
9. `stbds_shmode_func` truncating `mode` with `(unsigned char)`, so `256 -> 0`
   and `-1 -> 255`, which then falls through to the `default: memcpy` branch of
   `hmput_key`'s `switch`.

### One build-profile difference (asserted, not assumed)

For inputs that drive the C into **undefined behaviour** — a store through
`NULL` after a failed `realloc` (`stbds_arrgrowf`), or `a->storage->storage` with
`storage == NULL` (`stbds_stralloc`) — the C faults with `SIGSEGV`.

* `target/release/libarr_push_lib.so` (the shipping artifact; `panic = "abort"`,
  no `debug_assertions`, matching the C build which has neither `-DNDEBUG` nor
  `-O`) faults with **`SIGSEGV`, identical to the C**.
* `target/debug/libarr_push_lib.so` reports
  `"null pointer dereference occurred"` and aborts with `SIGABRT`, because the
  dev profile's `debug_assertions` insert MIR null-pointer-deref checks.

This is Rust deliberately trapping the same UB, not a difference in the
translated logic. It is pinned by
`errors_fatal.rs::dev_build_only_traps_the_same_ub`, and the fatal-path rows are
therefore compared against the release `.so`. All **non**-fatal rows are compared
against the *dev* `.so`, whose arithmetic-overflow checks are active — so no
passing row silently depends on release-mode wrap-around.

### Rows proven *unreachable* rather than triggered

Four `STBDS_ASSERT`s cannot fire through the public API. Each is documented with
the argument and backed by an invariant test over a long randomized churn rather
than being quietly skipped:

| row | assert | why unreachable |
|-----|--------|-----------------|
| 5 | `uct + tct < slot_count` (`lib.c:401`) | fails only for `slot_count <= 2`; callers only ever pass `8`, `slot_count*2`, or `slot_count>>1` guarded by `slot_count > 8`. `slot_count >= 8` and a power of two is asserted after every one of 3 000 random ops. |
| 21 | `(size_t)i+1 <= arrcap(a)` (`lib.c:778`) | `arrgrowf(a,es,1,0)` guarantees `capacity >= length+1`; `length <= capacity` asserted after every one of 4 000 random ops. |
| 27 | `slot < slot_count` (`lib.c:828`) | `find_slot` returns `(pos & ~7) + i` with `pos &= slot_count-1` and `i < 8`; every stored bucket index is range-checked after every op. |
| 34a | `len <= a->remaining` (`lib.c:913`) | exhaustive enumeration of the three paths: `len <= remaining` skips the block; `len > blocksize` returns early; otherwise `remaining` is set to `blocksize >= len`. |

Row 28 (`used_count >= 0` on a `size_t`) is a tautology; row 30 is dominated by
row 29, which fires first on every constructible input.

### Test-harness bugs found and fixed during verification

Worth recording because they are the classic false positives in this kind of
differential testing — in each case the C and the Rust agreed and the *test* was
wrong:

* `stbds_hash_index::temp_key` is **never initialised** by
  `stbds_make_hash_index` (only the `string` sub-struct is `memset`) and is only
  written on the three string-mode paths. Snapshotting it in binary mode was
  comparing indeterminate heap bytes. It is now excluded from the generic
  snapshot and checked explicitly only where the C provably writes it.
* Element **padding** is indeterminate whenever `keysize < elemsize` and the
  caller writes no value there, so every test configuration now keeps
  `keysize <= valoffset` and `valoffset + valsize == elemsize`.
* `realloc` may legitimately return the same address, so pointer identity is
  only asserted on `arrgrowf`'s `min_cap <= arrcap` early-out, which returns `a`
  without calling `realloc` at all.


## Harness validation (mutation testing)

Passing tests only mean something if they would fail on a wrong translation, so
the harness was validated by deliberately mutating `src/lib.rs` — 27 mutations,
each a plausible "tidy-up" or off-by-one a translator might introduce — and
checking that some test fails. Restored afterwards; `src/lib.rs` is unchanged
from the original translation.

**24 of 27 caught.** The three that are not are the *same* mutation site and are
genuinely unobservable:

| mutation | result |
|----------|--------|
| M2 set `temp_key` in the wrap-around probe half too (i.e. "fix" the stb quirk) | caught by `map_string.rs` |
| M3 `hmdel_key`: `mode >= 1` instead of `== 1` for the strdup free | caught by `errors.rs` (allocator-recycling detector) |
| M4 `hmdel_key`: `mode >= 1` for the re-lookup key choice | caught by `errors_fatal.rs` |
| M5/M6 drop the siphash `int` sign-extension (tail / word loop) | caught by `hash.rs` |
| M7 `hash_string`: read the byte as signed `char` | caught by `hash.rs` |
| M8 `arrgrowf`: `min_cap < 4` -> `< 8` | caught by `arr.rs` |
| M9 `arrgrowf`: `<` instead of `<=` in the early-out | caught by `arr.rs` |
| M10 `tombstone_count_threshold` drops the `>>4` term | caught by `map_binary.rs` |
| M11 forget the `slot_count <= 8` shrink-threshold reset | caught by `map_binary.rs` |
| M12 advance the global seed even when inheriting from `ot` | caught by `map_binary.rs` |
| M13 `nt->string.mode = SH_DEFAULT` even for binary mode | caught by `map_binary.rs` |
| M14 grow on `>` instead of `>=` the used-count threshold | caught by `map_binary.rs` |
| M15 `hmfree_func`: start the strdup loop at 0 instead of 1 | caught by `errors_fatal.rs` |
| M16 `stralloc`: bump `block` without the `1<<20` saturation | caught by `arena.rs` |
| M17 `stralloc`: oversize path zeroes `remaining` even when `storage != NULL` | caught by `arena.rs` |
| M18 `shmode_func`: don't truncate `mode` to `unsigned char` | caught by `errors2.rs` |
| M19 `hmget_key_ts`: `*temp = 0` instead of `-1` on a NULL table | caught by `errors.rs` |
| M20 `hmdel_key`: forget `stbds_temp(raw_a) = 0` | caught by `map_binary.rs` |
| M21 `is_key_equal`: `mode > 1` instead of `>= 1` | caught by `map_string.rs` |
| M22 `strkey`: different integer formatting | caught by `arr.rs` |
| M23 `stbds_log2`: off-by-one | caught by `map_binary.rs` |
| M24 `hmput_key`: `bucket->index = i` instead of `i-1` | caught by `map_binary.rs` |
| M25 `make_hash_index`: align `storage` to 32 instead of 64 | caught by `map_binary.rs` |
| **M1 / M1b** remove the `if (hash < 2) hash += 2;` bump (both call sites) | **NOT caught** |

M1/M1b are unobservable *by construction*: the bump only changes behaviour when
`stbds_hash_bytes`/`stbds_hash_string` return exactly `0` or `1`, i.e. a
2^-63 event for siphash-2-4 that cannot be searched for. This is precisely what
`ERRORS.md` row 45 documents, and the invariant it protects (no live bucket ever
stores hash `0` or `1`; empty slots are `hash==0,index==-1`; tombstones are
`hash==1,index==-2`) *is* asserted after every one of 3 000 randomized operations
by `errors2.rs::err45_hash_lt2_bumped`. The Rust source does contain the bump at
both sites.

## A harness bug that mutation testing exposed

The first mutation run reported many false "not caught" results. The cause was
serious and worth recording:

> Because `[lib] crate-type = ["cdylib"]` produces **no rlib**, an integration
> test has nothing to link against, so cargo leaves the `lib` target out of the
> test unit graph. **Neither `cargo test` nor `cargo test --test <name>` rebuilds
> `target/debug/libarr_push_lib.so`.** Every test was `dlopen`ing whatever `.so`
> a previous explicit `cargo build` had left behind.

This was verified directly: planting `abort()` in a hot path of `src/lib.rs` and
running `cargo test` did **not** fail. The fix is `common::ensure_built`, which
runs `cargo build --lib` (and `--release` for the fatal-path pair) before the
first `dlopen`, so the `.so` always matches the current source no matter how the
tests are invoked. `errors_fatal.rs::harness_tests_the_current_source`
additionally asserts that each `.so` is newer than its source file.

After the fix, the same `abort()` experiment fails immediately, and the mutation
results above are the post-fix numbers.

Note that this bug never invalidated any *reported* result in this verification:
`scripts/check_all_features.sh` always runs `cargo build` before `cargo test`,
and `src/lib.rs` needed no changes (no divergence was found), so the `.so` under
test was always current. The gap only mattered for the mutation experiments —
and for anyone who edits `src/lib.rs` in future.
