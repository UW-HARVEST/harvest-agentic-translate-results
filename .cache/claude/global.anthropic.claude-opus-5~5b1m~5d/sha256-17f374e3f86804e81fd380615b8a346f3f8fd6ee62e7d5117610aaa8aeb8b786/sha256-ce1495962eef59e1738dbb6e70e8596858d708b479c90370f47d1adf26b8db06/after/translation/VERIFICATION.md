# VERIFICATION.md — how this translation was verified

The library under test is an inlined copy of `stb_ds.h` (`c_src/src/lib.c`,
958 lines, one translation unit) plus the `strkey` / `arr_del` helpers.
`c_src/include/lib.h` declares only `void arr_del(int)`, but the `.so` exports
**16** symbols; all of them are part of the verification surface.

## How to reproduce

```sh
./run_all.sh          # builds the C .so, then, for every feature set:
                      #   cargo check / cargo build --release / symbol diff /
                      #   cargo test --release
```

> **Trap worth knowing:** `cargo test` does **not** rebuild a `cdylib`-only lib
> target, so the `.so` the tests `dlopen` can be stale while the test binaries
> are fresh.  `run_all.sh` always runs `cargo build --release` first, and
> `tests/common/mod.rs` additionally *refuses to run* if
> `target/release/libarr_del_lib.so` is older than `src/lib.rs`.

## Method

Every assertion crosses the FFI boundary.  Both libraries are loaded with
`libloading` (`tests/common/mod.rs`) and only their **exported** symbols are
called — never a Rust function directly — so the `#[unsafe(no_mangle)] extern
"C"` wrappers are part of what is tested.

`tests/common/mod.rs` mirrors the C structs (`stbds_array_header`,
`stbds_hash_index`, `stbds_hash_bucket`, `stbds_string_arena`,
`stbds_string_block`) so a test can snapshot the **complete** observable state of
a map after every single operation:

* array header: `length`, `capacity`, `temp`, `hash_table != NULL`
* hash index: `slot_count`, `slot_count_log2`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`
* embedded arena: `remaining`, `block`, `mode`, block-chain length
* **every** bucket slot: `hash[8]` and `index[8]`
* **every** element: key bytes (or, for pointer keys, the pointed-to C string)
  plus the value region

`DualMap` drives the C map and the Rust map in lockstep and compares that whole
snapshot after each `hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key` /
`hmput_default`, as well as the `stbds_temp` value the `stbds_hm*` macros read.

Randomised rows use a fixed-seed splitmix64 PRNG, so every run is reproducible.

### Things that are *not* compared, and why

| field | why |
|-------|-----|
| `stbds_hash_index::temp_key` | `stbds_make_hash_index` never initialises it, so it is indeterminate except immediately after a string-mode `stbds_hmput_key`. It is read only there, via `map_temp_key()` (`tests/errors.rs::e49_*`, `tests/maps_string.rs::r51_*`). |
| element bytes beyond the key that the test did not write | `stbds_hmput_key` only `memcpy`s `keysize` bytes, leaving the rest of a freshly grown element indeterminate. The tests therefore write the whole value region themselves (exactly like the `stbds_hmput` macro writes `t[i].value`). |
| raw pointers (`storage`, strdup'd/arena keys) | different heap addresses in the two libraries; compared by *contents* and by chain length instead. |

### Global state

Both `.so`s carry their own `static size_t stbds_hash_seed`, which every fresh
`stbds_make_hash_index` advances.  For the two libraries' bucket layouts to be
comparable the seed sequences must stay in lockstep, so every seed-sensitive
test takes a process-wide lock (`reset_seeds()` returns a guard).  The same lock
protects `strkey`'s static buffer.  Without this the suite fails
non-deterministically under `cargo test`'s default multi-threading — this was a
real bug in the first version of the harness.

## Phase A — surface

* `SYMBOLS.md` — 16/16 C symbols exported by the Rust `.so`, 0 missing, 0 extra.
  Checked programmatically by `tests/symbols.rs` (it shells out to `nm -D`) and
  by `run_all.sh`'s `diff`.
* `ERRORS.md` — 54 rows, one per distinct rejection/fault the C source contains
  (every `return -1` / `return 0` / early `return a`, all seven
  `STBDS_ASSERT`s, every null and range check, both min/max arena constants).
* `CONFIGS.md` — 82 rows over the cross product of the axes the C actually
  branches on (`mode`, `table->string.mode`, the four map-creation entry points,
  `elemsize`/`keysize` shapes, table load/grow/shrink/rebuild, probe path,
  delete position, hash length classes, arena block growth).

## Phase B — valid paths

`tests/hash.rs`, `tests/arrays.rs`, `tests/maps_binary.rs`,
`tests/maps_string.rs`, `tests/arena_misc.rs`, `tests/probe_paths.rs`.

All 82 `CONFIGS.md` rows pass across randomised inputs.  The tests drive the
**low-level** exports directly (`stbds_arrgrowf`, `stbds_hmput_key`,
`stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_shmode_func`, `stbds_stralloc`),
re-implementing the `stbds_arrput` / `stbds_hmput` / `stbds_shput` /
`stbds_hmdel` macro bodies in the test so the composed pipeline is exercised,
not just one wrapper at a time.

## Phase C — error paths

`tests/errors.rs` (18 tests) covers every `ERRORS.md` row whose C behaviour is a
sentinel return, asserting the *same* sentinel (`-1`, `0`, `NULL`, `temp == 0`
vs `temp == 1`) — never merely "both failed".

`tests/crash_parity.rs` covers the rows whose C behaviour is a **fault or
abort**.  Those cannot be observed in-process, so each case runs in a child
process (once against the C `.so`, once against the Rust `.so`) and the two
termination statuses are compared.  The test also asserts that the C really did
fail, so a case cannot silently rot into a no-op.

| case | C result | Rust result |
|------|----------|-------------|
| `stbds_arrgrowf` OOM → store through `NULL+32` | SIGSEGV | SIGSEGV |
| `stbds_arrfreef(NULL)` → `free(NULL-32)` | SIGABRT | SIGABRT |
| `stbds_hmget_key_ts(…, temp = NULL)` | SIGSEGV | SIGSEGV |
| `stbds_hash_string(NULL)` | SIGSEGV | SIGSEGV |
| `stbds_hash_bytes(NULL, 16)` | SIGSEGV | SIGSEGV |
| `stbds_is_key_equal` with a NULL stored key → `strcmp(k, NULL)` | SIGSEGV | SIGSEGV |
| `stbds_stralloc` with `remaining >= len` but `storage == NULL` | SIGSEGV | SIGSEGV |
| `stbds_stralloc(arena, NULL)` | SIGSEGV | SIGSEGV |
| `stbds_strreset(NULL)` | SIGSEGV | SIGSEGV |
| `stbds_hmdel_key` relocation re-find fails → `assert(slot >= 0)` | SIGABRT | SIGABRT |
| `stbds_hmdel_key` with `mode == 2` + relocation → same assert | SIGABRT | SIGABRT |

### One fix made to the Rust during Phase C

The C `.so` is compiled with `C_FLAGS = -fPIC` and **no** `-DNDEBUG`, so all
seven `STBDS_ASSERT`s are live in the reference library.  The translation had
turned them into comments, which meant that for out-of-contract inputs the C
aborted while the Rust silently continued.  The `assert!`s are now present in
`src/lib.rs`; with `panic = "abort"` in `[profile.release]` a failing one aborts
with SIGABRT exactly like C's `assert`.  The one C assert that is a tautology on
`size_t` (`STBDS_ASSERT(table->used_count >= 0)`) is kept as a comment.

## Phase D — symbol parity and feature combinations

```
$ diff <(nm -D --defined-only c_src/build/*.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libarr_del_lib.so \
           | awk '$(NF-1)=="T"{print $NF}' | sort)
$          # empty
```

`translation/Cargo.toml` declares **no** `[features]` table, so the only
distinct configurations are the default one, `--no-default-features` and
`--all-features`; `run_all.sh` enumerates them from `Cargo.toml` (so a future
feature is picked up automatically) and runs check + build + symbol diff + the
full test suite for each.

## Suite-sensitivity check (mutation testing)

Because "all tests pass" is only meaningful if the tests can fail, 29 deliberate
bugs were injected into `src/lib.rs` one at a time, rebuilt and run against the
suite.  Reproduce with `mutation-testing/run.sh`; the per-mutant verdicts and
justifications are in `mutation-testing/RESULTS.md`.  Results:

* **24 caught.**
* **5 survived, and all five are provably equivalent mutants:**

| mutant | why it cannot be detected |
|--------|---------------------------|
| `else if (min_cap < 4)` → `< 5` in `stbds_arrgrowf` | the branch is only reached when `min_cap >= 2*arrcap`; the two versions differ only for `min_cap == 4`, where both leave the capacity at 4. |
| `if (hash < 2)` → `< 3` | differs only for a full 64-bit hash of exactly 2 (p ≈ 2⁻⁶⁴). |
| `strlen(str)+1` → `+2` in `stbds_strdup` | allocates and copies one extra byte; the resulting C string, and therefore every observable, is identical. |
| `(n + a - 1) & ~(a-1)` → `(n + a) & ~(a-1)` in `STBDS_ALIGN_FWD` | the argument is always `table + 104` with `table` 16-byte aligned, i.e. `≡ 8 (mod 16)`, so it is never 64-byte aligned and the two expressions always agree. |
| a `.wrapping_sub(0)` no-op inserted in `stbds_stralloc` | syntactically different, semantically identical (author error while generating the mutant). |

The first mutation run exposed one **genuine coverage gap**: the multi-bucket
probe walk (`pos += step; step += 8; pos &= mask`), which exists in
`stbds_hm_find_slot`, `stbds_hmput_key` *and* `stbds_make_hash_index`, is
unreachable by property testing because it needs two *consecutive* completely
full 8-slot buckets while the table never exceeds 75 % load.
`tests/probe_paths.rs` now builds the bucket array by hand — byte-identically in
both maps — to force 2+ hops through all three copies of that walk, including
the tombstone-reuse and wrap-around-duplicate variants (CONFIGS rows 71–77).
The corresponding mutants are caught after that addition — one of them
(`step += 8` → `+= 16` inside `stbds_hmput_key`) makes the Rust probe loop
*never terminate* where the C returns, so `tests/probe_paths.rs` also installs a
30-second watchdog that aborts with an explicit message rather than hanging the
run.

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust | PASS |
| Phase B: every `CONFIGS.md` row passes across randomised inputs | PASS (82/82) |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | PASS (all non-`n/a` rows; `n/a` rows justified inline) |
| all of the above under every feature combination | PASS (no `[features]`; default / `--no-default-features` / `--all-features`) |
