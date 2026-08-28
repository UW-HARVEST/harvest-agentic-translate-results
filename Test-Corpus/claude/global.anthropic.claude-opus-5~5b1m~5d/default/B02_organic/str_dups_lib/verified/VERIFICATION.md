# VERIFICATION.md — completion gate

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `../c_src/src/lib.c` (a subset of `stb_ds`: growable arrays,
open-addressed hash maps with three string-key ownership modes, a string arena,
plus the `strkey` / `str_dups` drivers).

## How to reproduce

```sh
# 1. build the C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. everything: symbol parity + the whole suite, for every feature
#    combination and both build profiles
cd ../../translation && ./run_all_configs.sh
```

Every test loads **both** `.so`s with `libloading` and calls only their exported
symbols — no Rust function is ever called directly, so the `#[no_mangle]`
`extern "C"` wrappers are part of what is under test.

## Method

Pointer *values* cannot be compared across two independently-`malloc`ing
libraries, so each test drives an identical operation sequence against both and
compares an address-independent **byte log** of the resulting state:

* array header — `length`, `capacity`, `temp`, `hash_table != NULL`;
* `stbds_hash_index` — `slot_count`, `used_count`, all four thresholds, `seed`,
  `slot_count_log2`, and the embedded arena's `remaining`/`block`/`mode`;
* the **entire bucket array** — every `hash[8]` and `index[8]` of every bucket;
* every element — raw bytes for binary keys, the pointed-to *string* plus the
  remaining element bytes for string keys;
* `hash_index::temp_key`, dereferenced only while provably live (see `TkValid`
  in `tests/common/mod.rs`);
* `stbds_stralloc` results as `(chain index of the owning block, offset in that
  block)`;
* `str_dups`'s `printf` output, captured by `dup2`-ing fd 1 to a temp file;
* for the abort paths, the **termination signal of a forked child per
  implementation** (`diff_child`).

Inputs are property-style and randomized from a fixed seed
(`0xC0FFEE00_12345678`), so runs are reproducible; the process-global
`stbds_hash_seed` is re-synchronised with `stbds_rand_seed` before each scenario
and every test holds a process-wide lock (cargo runs tests in parallel threads
inside one process, and both libraries have mutable globals).

## Gate

- [x] **`SYMBOLS.md`** — `nm -D` diff between the two `.so`s is **empty in both
      directions** (16/16 symbols, exact names); 0 undefined non-libc symbols in
      the Rust `.so`. No stubs: every export is a real translation, and every
      export is driven by at least one test.
- [x] **Phase B** — all **64** rows of `CONFIGS.md` (55 original + 9 added while
      testing) pass across randomized inputs.
- [x] **Phase C** — all **60** rows of `ERRORS.md` have a passing error-path
      differential test (52 test functions; some cover several rows).
- [x] **Phase D** — the crate declares **no cargo features** (`cargo metadata`
      reports `features: {}`), so the default build is the only feature
      configuration. `run_all_configs.sh` still enumerates the powerset
      mechanically and additionally runs the whole suite against **both** the
      `dev` and the `release` cdylib, since profiles were the axis that actually
      mattered here (see divergence 2 below).

```
=== combo: default features   profile: dev ===
  PASS cargo check / cargo build
  PASS nm -D: all 16 C symbols exported by the Rust .so
  PASS nm -D: no extra public symbols
  PASS nm -D: 0 undefined non-libc symbols
  PASS differential suite vs target/debug/libstr_dups_lib.so: 120 tests passed

=== combo: default features   profile: release ===
  PASS cargo check / cargo build
  PASS nm -D: all 16 C symbols exported by the Rust .so
  PASS nm -D: no extra public symbols
  PASS nm -D: 0 undefined non-libc symbols
  PASS differential suite vs target/release/libstr_dups_lib.so: 120 tests passed

ALL CONFIGURATIONS PASS
```

## Divergences found and fixed in the Rust

1. **All ten `STBDS_ASSERT`s were missing.** `c_src/CMakeLists.txt` sets no
   build type, hence no `-DNDEBUG`, and `nm -D` on the C `.so` confirms
   `U __assert_fail`: assertions are **live**, and a failure is an observable
   `abort()`. Most of them are unreachable, but
   `STBDS_ASSERT(slot >= 0)` at `c_src/src/lib.c:846` is **not**: it fires for
   `stbds_hmdel_key` with a non-zero `keyoffset`, and for any `mode >= 2` on a
   string map (line 845 then hashes the *address* of the moved element instead
   of its key string). Where the C stopped hard, the Rust was computing
   `storage.offset(-1)` and writing `b->index[7] = old_index` into the
   `stbds_hash_index` header — silent memory corruption. Fixed by
   transliterating all ten asserts (`STBDS_ASSERT!` in `src/lib.rs`, which
   writes the glibc-style diagnostic to fd 2 and calls `abort()`); the vacuous
   `used_count >= 0` on a `size_t` is documented in place rather than emitted,
   because in C it is always true even after the counter wraps to `SIZE_MAX`.
   Covered by `err_39_hmdel_nonzero_keyoffset`,
   `err_34_hmdel_mode_ge_2_mid_delete_aborts` (fork-based signal comparison) and
   `err_24`/`err_32_34_35`/`err_33`/`err_40`/`err_45` (which prove the others
   stay unreachable).

2. **Rust's debug-only UB checks changed behaviour.** `stbds_hash_string(NULL,
   seed)` segfaults in C (SIGSEGV/11); with `debug-assertions = on` the Rust
   `.so` aborted instead (SIGABRT/6) from rustc's injected null-pointer-deref
   check. The same class of check turned the C's legal-on-x86 *unaligned*
   `char *` store — any `elemsize` that is not a multiple of 8,
   `c_src/src/lib.c:786-788` — into an abort as well. Neither check is C
   semantics, so `Cargo.toml` now sets `debug-assertions = false` and
   `overflow-checks = false` for the `dev` and `test` profiles: every profile now
   behaves like the release artifact, and the suite is run against both.
   Covered by `err_60_hash_string_null_aborts_identically` and
   `cfg55b_fuzz_randomized_string_shapes`.

## C behaviour deliberately reproduced, not "fixed"

* `stbds_hmput_key`'s two inner probe loops are **asymmetric**: the forward loop
  refreshes `hash_table->temp_key` on a duplicate hit (line 732-733), the
  wrap-around loop does not (line 746-759). Combined with
  `stbds_make_hash_index` never initialising `temp_key`, `stbds_shputs` +
  `STBDS_SH_STRDUP` + a wrap-around duplicate can alias two entries onto one
  `strdup`'d key and double-free it in `stbds_hmfree_func`. The translation
  reproduces this exactly; the tests route around the abort (see the note on
  `cfg43b_shputs_writes_temp_key_back`).
* `str_dups` calls `printf("%s %d\n", strmap[z], strmap[z].value)`, passing a
  16-byte struct **by value** through varargs. On SysV AMD64 it occupies two
  INTEGER parameter slots, so `%s` consumes `key`, `%d` consumes `value`, and
  the explicit third argument is ignored. Verified byte-for-byte via captured
  stdout (`cfg53`, `err_56_57`) and by linking a real C driver against each
  `.so` — both print `a <num>`.
* `stbds_stralloc`'s `512u << (a->block >> 1)` is UB once `block >= 128`; x86-64
  masks the shift count to 6 bits and the Rust uses `wrapping_shl`, so both
  agree (`cfg21`, `err_44`).
* `stbds_hash_bytes` relies on `d[3] << 24` overflowing `int` and then being
  sign-extended into `size_t`, in both the 8-byte main loop and the
  fall-through tail `switch`. Reproduced literally and pinned down by
  `cfg08`/`cfg09`/`err_49`, which sweep every tail length 1..7 × every
  high-bit-byte position.

## Cases excluded from byte-for-byte comparison (and why)

Each of these is genuinely address- or allocator-dependent, i.e. two correct
implementations may legitimately differ. They are documented at the test that
excludes them rather than silently skipped.

| case | why it is not comparable |
|---|---|
| `stbds_hmdel_key` with `mode >= 2` and `old_index != final_index` | `c_src/src/lib.c:845` hashes the *address* of the moved element. Compared instead as "both abort with the same signal" (`err_34`). |
| `hash_index::temp_key` after a growth-triggering duplicate put, or after a `SH_STRDUP` delete | uninitialised `realloc` memory / a freed pointer. Tracked precisely by `TkValid` and compared only while live. |
| element bytes past what the C wrote (`keysize < elemsize` tails, `keysize > elemsize` overruns) | uninitialised `realloc` padding. Tests fill the tails deterministically, or dump only the provably-written range (`err_55`). |
| `elemsize == 0` with `string.mode ∈ {1,2,3}` | the C stores an 8-byte `char *` into a zero-byte element, i.e. a heap overflow past a header-only allocation (`cfg51`). |
| `mode >= 1` lookups on a `string.mode == SH_NONE` map | the insert arm `memcpy`s the key *pointer bytes* into the element, so a lookup `strcmp`s through them as if they were a `char *`. Only the well-defined distinct-key inserts are compared (`cfg50c`). |
| `stbds_arrgrowf` with an `elemsize` that makes `elemsize*min_cap + 32` wrap below 32 | the header write would then corrupt the heap in both libraries. `err_05` uses `elemsize ∈ {0,1}` so the wrapped request still holds the header. |
| `a->block` values whose masked shift yields a huge non-zero `blocksize` (e.g. 200 → `<< 36`) | a 32 TiB `malloc` that fails and is then dereferenced — aborts both and measures nothing (`cfg21`, `err_44`). |

## Test inventory

| file | tests | scope |
|---|---|---|
| `tests/common/mod.rs` | — | harness: `libloading` binding of all 16 symbols, mirrored C layouts, state snapshotting, `diff`, `diff_child`, `capture_stdout`, `TkValid`, xorshift RNG |
| `tests/smoke.rs` | 4 | both `.so`s load, all 16 symbols resolve, the mirrored C layouts match real C memory, seed self-advance |
| `tests/phase_b_low.rs` | 22 | `CONFIGS.md` 1-22 (+18b): `arrgrowf`/`arrfreef`, `hash_bytes`, `hash_string`, `rand_seed`, `stralloc`/`strreset` |
| `tests/phase_b_map.rs` | 14 | `CONFIGS.md` 23-36: binary-key maps — keysize/elemsize sweeps, growth thresholds, duplicates, deletes, shrink, tombstone rebuild |
| `tests/phase_b_string.rs` | 19 | `CONFIGS.md` 37-51: every `string.mode`, out-of-range `shmode_func`/`mode` enums, `elemsize == 0` |
| `tests/phase_b_top.rs` | 9 | `CONFIGS.md` 33b, 52-55b: `strkey`, `str_dups` stdout, randomized whole-pipeline op-stream fuzzing |
| `tests/phase_c_errors.rs` | 52 | `ERRORS.md` 1-60, including four fork-based abort comparisons |
| **total** | **120** | run against both the dev and the release cdylib |

## Is the suite actually sensitive? (mutation testing)

Passing tests only mean something if they would have failed. `mutation_check.py`
injects one C-semantics-breaking change into `src/lib.rs` at a time, **rebuilds
the cdylib**, and reruns the whole suite. Latest run (`mutation_report.txt`):

```
CAUGHT  33
MISSED  5
SKIPPED 0
UNEXPLAINED SURVIVORS: NONE
```

Caught mutations span every function: the two siphash sign-extension sites, the
`hash_string` rotate/`unsigned char` cast/`+ seed`, all four
`stbds_hash_index` thresholds, the seed capture point, the `temp_key`
refresh/no-refresh asymmetry, the `bucket->index = i-1` base, tombstone reuse,
`final_index`, the `temp = 0/1` delete sentinel, the rebuild gate, the
`*temp = -1` miss sentinel, `hmput_default`'s `length == 0` disjunct, the
`hmfree_func` sweep start index, `shmode_func`'s `(unsigned char)` truncation,
the arena block-size/`++block`/splice logic, `strreset`'s `memset`,
`arrgrowf`'s `temp` init and doubling factor, `stbds_hmlen`'s `-1`, `strkey`'s
format string, and the reachable assert pair.

Two things came out of this that matter more than the score:

### A third real defect: the suite could pass vacuously

`cargo test` does **not** rebuild a `cdylib`-only lib target. Because the
integration tests `dlopen` the library rather than link it, cargo does not treat
the cdylib as a dependency of the test targets — so editing `src/lib.rs` and
running `cargo test` leaves `target/debug/libstr_dups_lib.so` byte-identical
(verified by md5) and every differential test passes against the *previous*
build. The first mutation run scored 0/10 for exactly this reason.

`tests/common/mod.rs::assert_fresh()` now compares the `.so`'s mtime against
`src/lib.rs` and `Cargo.toml` and fails loudly with an actionable message, and
`run_all_configs.sh` always `cargo build`s first.

### Aborting children need an incrementally-flushed log

`diff_child` originally buffered a child's log in memory and wrote it at the
end, so a child killed by a live `STBDS_ASSERT` wrote *nothing*. Two children
that both abort then looked like agreement no matter where they aborted.
`Log::to_fd` now writes each record through immediately, so the surviving prefix
records how far each side got. This is what makes
`drop BOTH reachable asserts (:846 and :849)` a caught mutation.

### Surviving mutations, all accounted for

| surviving mutation | why it is not a blind spot |
|---|---|
| `make_hash_index: cache-line alignment 64 -> 32` | internal allocation layout only; bucket contents, thresholds and every probe result are unchanged, and no address is ever compared |
| `hmdel_key: shrink gate slot_count > 8 -> >= 8` | **dead clause**: at `slot_count == 8` `make_hash_index` forces `used_count_shrink_threshold = 0` (c_src/src/lib.c:399-400) and `used_count < 0` is vacuously false for a `size_t`, so the extra test can never change the outcome |
| `hmdel_key: strdup free guard mode == -> mode >=` | the only difference is whether a `strdup`'d key leaks; the element is already past `length`, so `hmfree_func` never touches it either way. Detecting it would need an allocator hook |
| `str_dups: sh_new_strdup -> sh_new_arena` | `str_dups` returns `void` and its only observable is stdout; both key-ownership modes satisfy the three asserts and print the identical `a <num>` line |
| `str_dups: skips the strreset` | a pure memory leak; stdout is byte-identical |
| dropping **any single** unreachable `STBDS_ASSERT` (`:401`, `:778`, `:828`, `:913`, `:960-962`) | these asserts are provably unreachable — which is exactly what `ERRORS.md` rows 24/32/33/35/40/45/57 claim and what `err_24`, `err_32_34_35`, `err_33`, `err_40`, `err_45` and `err_56_57` verify. Removing an unreachable check is behaviourally equivalent |
| dropping **either one** of the reachable pair `:846` / `:849` | the two guard the same re-lookup, so removing one still aborts at the other. Removing **both** IS caught, by `err_34` and `err_39` |

Every survivor is either a behaviourally equivalent mutant or a memory-only
effect that a caller of the public C API cannot observe. `mutation_check.py`
exits non-zero if any *unexplained* survivor appears.

The `stralloc: len > blocksize -> len >= blocksize` mutant DID survive the first
run — a genuine gap at the `len == blocksize` boundary — and is now caught by the
newly added `cfg18b_stralloc_len_exactly_blocksize` (`CONFIGS.md` row 18b).
