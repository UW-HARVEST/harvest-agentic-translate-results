# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C ground
truth). Both are loaded as shared objects with `libloading` and called **only**
through their exported C symbols, so the `#[no_mangle] extern "C"` wrappers are
part of what is under test.

## Completion gate

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** — 16/16 exported, symbol diff empty, `ldd -r` clean on both, identical allocator imports |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | **PASS** — 48 `C*` rows + 11 `S*` composition/exhaustive rows, 0 unchecked |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | **PASS** — 69 `E*` rows + 6 `B*` boundary rows, 0 unchecked |
| All of the above under every feature combination | **PASS** — the crate declares no `[features]`, so `DEFAULT`, `--no-default-features` and `--all-features` are the complete set; all three verified, each against **both** the release and the debug cdylib |

Reproduce:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && ./run_all.sh     # -> "ALL CHECKS PASSED"
```

## Test inventory

137 `#[test]` functions in 8 files (plus the shared harness):

| file | tests | scope |
|------|------:|-------|
| `tests/common/mod.rs` | — | dual-`dlopen` loader, `DiffArr`/`DiffMap` macro-level drivers, state snapshotter, `SelfKeys`, slot/bucket targeting, SplitMix64 PRNG |
| `tests/phase_b_hash.rs` | 18 | `rand_seed`, `hash_bytes`, `hash_string` — incl. exhaustive sweeps |
| `tests/phase_b_array.rs` | 20 | `arrgrowf`, `arrfreef`, the whole `arr*` macro pipeline, `arr_ins`, `strkey` |
| `tests/phase_b_map_binary.rs` | 22 | `STBDS_HM_BINARY` maps, growth/shrink/tombstone ladders, `keyoffset` |
| `tests/phase_b_map_string.rs` | 14 | `STBDS_HM_STRING` × all four `STBDS_SH_*` key-storage modes |
| `tests/phase_b_arena.rs` | 12 | `stralloc` / `strreset`, block growth, saturation, shift-count masking |
| `tests/phase_b_stress.rs` | 7 | long randomized runs mixing every entry point |
| `tests/phase_c_errors.rs` | 38 | one test per `ERRORS.md` rejection row + generic FFI boundaries |
| `tests/phase_c_aborts.rs` | 6 | subprocess crash-parity for the fatal rows |
| `tools/check_alloc_trace.sh` | 6 scenarios | allocation-call-sequence parity via an `LD_PRELOAD` allocator interposer |

## What "identical" means here

After **every** operation the harness compares, byte for byte:

* the function's return value (and NULL-ness of returned pointers);
* `stbds_array_header`: `length`, `capacity`, `temp`, `hash_table != NULL`;
* every live element's bytes (keys resolved through the `char*` indirection for
  the storage modes that keep a pointer, so only genuinely defined bytes are
  compared);
* the whole `stbds_hash_index`: `slot_count`, `used_count`, all three
  thresholds, `tombstone_count`, `seed`, `slot_count_log2`, the embedded
  `stbds_string_arena` (`remaining`/`block`/`mode`/chain shape), and **every
  bucket's full `hash[8]` and `index[8]` arrays`;
* `stbds_hash_index::temp_key` at exactly the points where the C defines it
  (immediately after an insert);
* for fatal inputs: the exact termination status (exit code / signal);
* the exact sequence of `malloc`/`calloc`/`realloc`/`free` calls and their sizes
  (`tools/check_alloc_trace.sh`) — 261 allocator calls across 6 scenarios.

The maps and arrays are driven through the *macro-level* protocol
(`stbds_hmput`/`stbds_shput`/`stbds_hmgeti`/`stbds_hmgeti_ts`/`stbds_hmdel`/
`stbds_hmdefault`, `arrput`/`arrpop`/`arraddn`/`arrins`/`arrdel`/`arrdelswap`/
`arrsetlen`/`arrsetcap`/`arrfree`), not just the raw functions, so the composed
pipeline is what gets compared.

## Divergences found and fixed in the Rust

| # | issue | fix |
|---|-------|-----|
| 1 | Every `STBDS_ASSERT` was translated as `debug_assert!`, but `c_src/CMakeLists.txt` compiles with `C_FLAGS = -fPIC` and no `NDEBUG`, so glibc `assert` is **live**. Invalid input that aborts the C would have been silently ignored by the Rust release build. | all 8 sites changed to `assert!` (verified by `abort_hmdel_mode2_swap`: C and Rust both SIGABRT) |
| 2 | Several additions/subtractions were plain `+`/`-`/`+=`. The C wraps silently; a Rust build with `overflow-checks = on` (the default `dev` profile) would panic instead. | all such sites use explicit `wrapping_add`/`wrapping_sub` (enforced by running the whole suite against the debug cdylib) |
| 3 | `stbds_stralloc`/`stbds_strreset` used `(*p).field` on pointers the C never NULL-checks (a failed `realloc`, or `a->storage == NULL` with `a->remaining >= len`). In a debug Rust build that trips the null/alignment UB check and **aborts**, where the C **faults**. C = SIGSEGV(139) vs Rust = SIGABRT(134). | raw address arithmetic via `raw_load_ptr`/`raw_store_ptr` (libc `memcpy`); both now report 139 in both profiles |
| 4 | **Allocation-call sequence.** `stbds_realloc` was a single helper, so LLVM path-split the `a == NULL` branch of `stbds_arrgrowf` into a bare `malloc(n)` — but the C passes a *runtime* pointer there (`(a) ? stbds_header(a) : 0`) and really does call `realloc`. Visible to any allocator interposer (`LD_PRELOAD`, heap profiler, sanitizer). | split into `stbds_realloc` (runtime pointer, `black_box`'d so it stays a `realloc`) and `stbds_realloc_fresh` (the C's *literal*-NULL form at `lib.c:388/873/894/906`, which a C compiler folds to `malloc`). Verified by `tools/check_alloc_trace.sh`: all 6 scenarios now match **exactly**. |

## C behaviours deliberately reproduced (not "fixed")

* `stbds_make_hash_index` never initialises `stbds_hash_index::temp_key`, and
  never carries it over on rehash/shrink/rebuild.
* In `stbds_hmput_key`, only the **first** in-bucket duplicate scan updates
  `stbds_temp_key`; the wrap-around scan does not
  (`e23_e24_temp_key_asymmetry` pins this exactly).
* `mode` and the `shmode` argument are plain `int`s with no validation:
  `mode >= 1` means "string", `(unsigned char) shmode` is stored verbatim and
  any value outside `0..3` falls through to the raw-`memcpy` `default:` label.
* `stbds_hmdel_key` with `mode >= 2` and an element swap hashes the wrong bytes
  and trips `STBDS_ASSERT(slot >= 0)` (row E67) — reproduced, not repaired.
* The sign-extension quirks in `stbds_siphash_bytes`
  (`data |= (d[3] << 24)` computed in `int`), the `hash ^= hash ^ ROTR(...)`
  no-ops in `stbds_hash_string`, and `512 << (block>>1)` with a shift count
  taken mod 64.
* `stbds_arrgrowf(NULL, _, 0, 0)` returns `NULL` without allocating.
* `stbds_hmdel_key(NULL, ...)` is the only path that returns `NULL`.

## Independent second audit

A separate, independently-written audit (its own `dlopen` harness, its own C
mirror structs, coverage counters compiled into a copy of the C, and its own
`LD_PRELOAD` allocator interposer) walked all 29 functions / macro expansions and
reported **0 divergences**, in both the release and the debug profile. Paths it
proved were actually reached include:

* the `stbds_hmput_key` second-scan duplicate hit with `mode >= STBDS_HM_STRING`
  — the branch that must *not* update `stbds_temp_key` (225 hits);
* the `stbds_make_hash_index` rehash wrap-around scan (1128) and multi-bucket
  probe advance (127);
* `stbds_hmdel_key` shrink (10) and tombstone rebuild (111);
* tombstone reuse in `stbds_hmput_key` (11 009);
* both `stbds_stralloc` big-block branches and every siphash tail case.

It also cross-checked `strkey` against `sprintf("test_%d")` for 3 000 000 random
`int` values plus every decimal boundary and both extremes, and confirmed the
exported dynamic-symbol sets are identical with no extras.

Two omissions it flagged as having no behavioural effect, which are intentional:

* `lib.c:832` `STBDS_ASSERT(table->used_count >= 0)` is not translated —
  `used_count` is `size_t`, so the predicate is vacuously true.
* `lib.c:305` `STBDS_STATS(++stbds_array_grow)` is not translated —
  `#define STBDS_STATS(x)` expands to nothing.

## Known non-reachable residue

If `realloc` itself fails (true OOM) the C writes through the resulting NULL in
`stbds_arrgrowf` / `stbds_make_hash_index` and faults; a **debug** Rust build
would trip the null-pointer UB check and abort instead. This cannot be triggered
from outside the library (there is no way to make the allocator fail through the
FFI surface) and is identical in the shipped release profile. It is recorded in
`ERRORS.md` rather than papered over.

The same applies to the allocation *entry point* in the debug profile: an
unoptimised Rust build emits `realloc(NULL, n)` where the C compiler folded the
literal-NULL form to `malloc(n)`. The two are equivalent by the C standard
(`realloc(NULL, n) == malloc(n)`), the sizes and ordering are identical, and the
shipped **release** cdylib matches the C exactly call-for-call.
`tools/check_alloc_trace.sh` reports "exact" for release and "normalised" for
debug, and fails only if the allocation sizes/ordering themselves differ.
