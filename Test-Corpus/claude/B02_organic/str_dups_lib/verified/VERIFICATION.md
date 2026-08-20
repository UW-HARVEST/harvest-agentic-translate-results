# VERIFICATION.md — completion gate

C-to-Rust differential verification of the stb_ds-based library in `c_src/`.
Every assertion below is reproduced by `./run_tests.sh` (which also builds both
shared libraries) plus `./check_rows.sh`.

## Artifacts

| file | purpose |
|------|---------|
| `SYMBOLS.md`  | Phase A — every `nm -D` symbol of the C `.so` mapped to the Rust `.so` |
| `ERRORS.md`   | Phase A — error-surface table (94 rows), Phase C test per row |
| `CONFIGS.md`  | Phase A — configuration-surface table (78 rows), Phase B test per row |
| `tests/harness/mod.rs` | loads BOTH `.so`s with `libloading`, state snapshotting, PRNG, stdout capture, crash-equivalence |
| `tests/configs.rs` | Phase B rows 1-24, 73-77 (29 tests) |
| `tests/maps.rs`    | Phase B rows 25-72 (49 tests) |
| `tests/errors.rs`  | Phase C rows 1-91 (91 tests + 17 `#[ignore]`d child cases) |
| `check_symbols.sh` | `nm -D` diff C vs Rust + undefined-symbol allowlist |
| `check_features.sh`| enumerates the feature power set from `Cargo.toml`, checks/builds/tests each |
| `check_rows.sh`    | proves every documented row has a real, passing test |
| `run_tests.sh`     | one-shot driver for all of the above |

**The Rust crate is never linked into the tests.**  `Cargo.toml` keeps
`crate-type = ["cdylib"]`; the tests `dlopen` `target/release/libstr_dups_lib.so`
and call its exported symbols, so the `#[unsafe(no_mangle)] extern "C"` wrappers
are themselves under test.

## Build-time configurations

* `c_src/CMakeLists.txt`: one unconditional `add_library(<name> SHARED src/lib.c)`,
  no `option()`, no `target_compile_definitions`, no `#ifdef`-selected variants.
* `Cargo.toml`: **no `[features]` entries** ⇒ the feature power set is
  `{ {} }` — a single combination, identical to the default build.
  `check_features.sh` derives this from `Cargo.toml` rather than assuming it, and
  runs `cargo check --all-targets --no-default-features`,
  `cargo build --release --no-default-features`, `./check_symbols.sh` and the
  full test suite for every element of the power set.

```
declared features: 0 (none)
feature combinations to verify: 1
ALL 1 FEATURE COMBINATION(S) OK
```

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust.**
      C exports 16, Rust exports 16, `comm -23` and `comm -13` are both empty;
      all undefined symbols in the Rust `.so` are libc / unwinder imports.
      `./check_symbols.sh` → `SYMBOL PARITY OK`.
      The assertion expression strings, `__PRETTY_FUNCTION__` names and line
      numbers baked into the two binaries are byte-identical (only the
      compile-time `__FILE__` *directory* differs — see `ERRORS.md` note (F)).

- [x] **Phase B: every row in `CONFIGS.md` passes across randomised inputs.**
      78 rows / 78 tests, driven with fixed-seed xorshift PRNG streams
      (thousands of inputs per row; the op-stream rows run 3000 randomised
      operations each and snapshot-compare after *every* operation).
      `check_rows.sh` → `unchecked rows in CONFIGS.md: 0`.

- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential test.**
      94 rows / 91 parent tests + 17 child crash cases.
      `check_rows.sh` → `unchecked rows in ERRORS.md: 0`.
      Crash rows compare the terminating **signal**, the **exit code** and the
      normalised `assert()` **diagnostic** — never merely "both failed".
      Generic boundaries (null pointers, zero/oversized lengths, out-of-range
      enum values across the FFI boundary) are rows 88-91.

- [x] **All of the above hold under every feature combination.**
      One combination exists and it is fully verified.
      Additionally the entire suite is re-run against the **dev-profile** `.so`
      (`RUST_SO=target/debug/libstr_dups_lib.so`), which enables
      `overflow-checks` and `debug-assertions`: a clean run there proves no
      arithmetic in the translation overflows where the C wraps.

```
Running tests/configs.rs   test result: ok. 29 passed; 0 failed
Running tests/errors.rs    test result: ok. 91 passed; 0 failed; 17 ignored
Running tests/maps.rs      test result: ok. 49 passed; 0 failed
```

## What "compared" means

Pointer *values* are never compared (the two libraries own separate
allocations).  After every operation `harness::Snapshot` captures, for both
libraries:

* `stbds_array_header`: `length`, `capacity`, `hash_table != NULL`, `temp`;
* `stbds_hash_index`: `slot_count`, `used_count`, `used_count_threshold`,
  `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, `string.mode`,
  `string.block`, `string.remaining`, the arena block-chain length, and
  `temp_key` (whenever it is well-defined);
* **every** bucket slot: the full `hash[]` / `index[]` arrays for all
  `slot_count` slots;
* **every** element byte from raw index 0 (the sentinel) to `length-1`, with
  library-owned `char *` keys normalised to the NUL-terminated bytes they point
  at and caller-owned keys compared by address (the *same* pointer is handed to
  both libraries).

`str_dups` is compared by capturing its `printf` output byte-for-byte
(fd 1 redirected to a temp file), which is why the suite must run with
`--test-threads=1`.

## Anti-blind-spot findings (C quirks the tests pin down)

These are behaviours the Rust must reproduce; mutation-testing confirmed that
"fixing" any of them makes the suite fail:

1. `stbds_arrgrowf(NULL, e, 0, 0)` returns **NULL without allocating**
   (`min_cap <= arrcap(NULL)` hits first) — `ERRORS.md` row 3.
2. `stbds_hmput_key`'s **wrap-around** duplicate branch (L746-751) sets `temp`
   but, unlike the forward branch (L729-735), does **not** refresh
   `stbds_temp_key` — `ERRORS.md` row 30.  Mutation: adding the missing
   `temp_key` write is caught by `err_hmput_duplicate_wraparound_no_tempkey`
   and `cfg_str_duplicates_temp_key`.
3. `stbds_hmdel_key` compares `mode == STBDS_HM_STRING` **exactly** (L836/L842)
   while `hm_find_slot` uses `mode >= STBDS_HM_STRING`, so `mode == 2` on a
   string table skips the `strdup` free and then re-finds through the *binary*
   branch, tripping `assert(slot >= 0)` — `ERRORS.md` rows 50/51.
   Mutation: relaxing `==` to `>=` is caught by 30 tests.
4. `stbds_siphash_bytes` builds `data` from `int` shifts, so `d[3] >= 0x80` /
   `d[7] >= 0x80` **sign-extend** into the upper 32 bits — `ERRORS.md` row 13.
   Mutation: dropping the sign extension is caught by 6 hash tests.
5. `data = len << 56` keeps only `len & 0xff`, so `hash_bytes` is *not* injective
   in `len` — `ERRORS.md` row 14.
6. A `string.mode` outside `{1,2,3}` makes `hmput_key` `memcpy` the key **text**
   into the element, after which any *string* lookup dereferences those bytes as
   a `char *` and segfaults — `ERRORS.md` rows 37/37b/37c.
7. `512 << (block>>1)` in `stbds_stralloc` is UB for `block >= 128`; x86-64
   masks the shift count to 6 bits and Rust's `wrapping_shl` matches, so
   `block == 255` yields `blocksize == 0` and `++a->block` wraps `255 → 0`
   — `ERRORS.md` row 67.  For `block == 108` the 2^63 `realloc` fails and the C
   writes through NULL — row 67b.
8. `make_hash_index` forces `used_count_shrink_threshold = 0` for
   `slot_count <= 8`, so a table never shrinks below `STBDS_BUCKET_LENGTH`
   — `ERRORS.md` row 76.  Mutation: `<=` → `<` is caught by 39 tests.
9. `if (blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX) ++a->block` saturates at
   `block == 22` (`512<<11 == 1<<20` is *not* `< 1<<20`) — `ERRORS.md` row 66.
   Mutation: `<` → `<=` is caught by `cfg_stralloc_block_presets`.
10. `stbds_make_hash_index` does **not** copy `temp_key` from the old table, and
    `stbds_hmdel_key` `free()`s the `strdup`ed key that `temp_key` may still
    point at — so `temp_key` is only well-defined right after a string-mode
    insert (the harness models exactly that).
11. `printf("%s %d\n", strmap[z], strmap[z].value)` in `str_dups` passes the
    whole 16-byte struct as the first variadic argument; the SysV AMD64 ABI puts
    `key` in the first integer register and `value` in the second, so `%s`/`%d`
    consume `(key, value)` and the third argument is ignored.  The Rust
    translation passes `(key, value, value)` and produces byte-identical output
    (`cfg_str_dups_stdout` over 13 values of `num`).

## Reproduce

```sh
./run_tests.sh                  # build both .so, symbols, features, full suite
./check_rows.sh                 # row <-> test bookkeeping
# extra: overflow-checks sweep against the dev-profile .so
cargo build && RUST_SO=$PWD/target/debug/libstr_dups_lib.so \
  cargo test -- --test-threads=1
```
