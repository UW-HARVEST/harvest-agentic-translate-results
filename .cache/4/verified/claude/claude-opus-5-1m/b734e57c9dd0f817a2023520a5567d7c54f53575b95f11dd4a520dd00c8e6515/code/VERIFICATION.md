# VERIFICATION.md — C-vs-Rust differential verification of zstd 1.5.7

The C in `c_src/` is the ground truth. This document records how the Rust
translation in `src/` was checked against it, what was found, and exactly which
gates were met.

Everything here is reproducible:

```
./run_difftests.sh      # build both .so's, check symbol parity, run the suite,
                        # fold row coverage back into CONFIGS.md / ERRORS.md
./coverage.py --check   # non-zero exit if any CONFIGS/ERRORS row is unaccounted for
./feature_matrix.sh --test   # every valid Cargo feature combination
./overflow_audit.sh     # the same suite against an overflow-checked Rust .so
```

## Result

| gate | result |
|---|---|
| `nm -D` symbols: C exports present in the Rust `.so` | **615 / 615**, 0 missing, 0 extra |
| Rust `.so` unresolved non-libc symbols | **0** |
| `CONFIGS.md` rows passing across randomized inputs | **433 / 433** |
| `ERRORS.md` rows with a passing error-path test, or excluded with evidence | **1142 / 1142** (956 tested, 186 excluded) |
| differential tests | **351 passed, 0 failed** |
| valid Cargo feature combinations, each fully verified | **1 / 1** |
| overflow-checked (`overflow-checks = on`) run of the same suite | **clean** |
| compiler warnings, library and tests | **0** |

## How the comparison is made

Both shared objects are loaded into the test process with `libloading`
(`RTLD_LOCAL`, so their identically-named exports do not collide), and **every**
call goes through `dlsym`. The Rust crate is never linked or called directly, so
the `#[no_mangle]` / `extern "C"` export wrappers are part of what is under test
rather than bypassed.

`tests/common/mod.rs` provides the harness:

* `diff(label, |lib| ...)` runs the closure against the C library, then the Rust
  library, and panics with both values if they differ. `diff_bytes` additionally
  reports the first differing byte index with surrounding context.
* `R` renders a `size_t` return as `Ok(n)` or `Err(code, name)` via
  `ZSTD_getErrorCode`/`ZSTD_getErrorName`, so a divergence names the error rather
  than showing `-42` as a huge unsigned.
* Comparisons include the **whole destination buffer**, not just the reported
  length, so bytes the callee did not write are compared too. Where a test
  deliberately drives a tight `dstCapacity`, `dst` is placed inside a larger
  allocation with a canary so an over-write is caught rather than silently
  tolerated.
* Out-params are pre-poisoned (e.g. `ZSTD_FrameHeader` filled with a sentinel)
  so "left untouched" is itself an observable.
* Inputs are property-style sweeps from a fixed-seed splitmix64 PRNG over ten
  distinct corpus shapes (zeros, single byte, incompressible random, 4-symbol
  alphabet, English-like text, long-range repeats, mixed runs, counter, periodic,
  sparse) and a size list that straddles every documented boundary.

Three safeguards keep the suite honest:

* **Stale-library guard.** `cargo test --test X` does *not* rebuild the `cdylib`,
  so it is possible to test an old `.so`. `pair()` compares the `.so`'s mtime
  against `src/` and refuses to run if it is older. (This caught a real
  false-negative during development.)
* **Coverage from execution, not comments.** Each test calls
  `covers(&["CFG:…", "ERR:file:line"])`; `run_difftests.sh` wipes the tag
  directory first and only folds the tags into the tables **if the whole run was
  green**, so a check-box cannot be set by a test that recorded its tags and then
  failed.
* **Crash localisation.** `ZSTD_DIFF_TRACE=1` prints each case label before it
  runs — the only way to find a `SIGSEGV`/`SIGFPE` inside either `.so`, which
  leaves no Rust backtrace.

## Phase A — the surface, derived mechanically

| artifact | contents |
|---|---|
| `SYMBOLS.md` | all 615 `nm -D` exports, grouped by originating C translation unit, each marked present in the Rust `.so`; plus the undefined-symbol check |
| `ERRORS.md` | **1142 rows**, one per distinct rejection site in `c_src/` — every `RETURN_ERROR`/`RETURN_ERROR_IF`/`ERROR(...)`, every `return NULL`, every `BOUNDCHECK`, every explicit range/null/stage check, with macro constants resolved to numbers and the expected numeric `ZSTD_error_*` code |
| `CONFIGS.md` | **433 rows**, one per meaningful combination of runtime option and input shape that the C actually branches on, covering the full public surface down to the lowest-level exports |

`phaseA/` holds the per-area chunks the two tables are assembled from.

The build configuration the rows were derived under (from `c_src/CMakeLists.txt`)
materially changes the surface, and is recorded in both tables:
`ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`, no
`ZSTD_MULTITHREAD`, no `-mavx2`/`-mbmi2`, `DEBUGLEVEL=0`, no `CMAKE_BUILD_TYPE`
(so `-O0`). Consequences that shaped the tests:

* `DEBUGLEVEL=0` means every `assert()` is `((void)0)`. Preconditions guarded
  *only* by an assertion are therefore unenforced, and the reference C can crash
  instead of returning an error. Those inputs are out of contract — see below.
* No `ZSTD_MULTITHREAD`: `nbWorkers`/`jobSize`/`overlapLog`/`rsyncable` have
  bounds `{0,0}` and reject non-zero with `parameter_unsupported` (40);
  `ZSTDMT_createCCtx_advanced` returns `NULL` unconditionally, so `cctx->mtctx`
  is permanently `NULL` and the whole ZSTDMT error surface is unreachable;
  `POOL_*` are synchronous stubs; `ZSTD_createThreadPool`/`ZSTD_freeThreadPool`
  are **not exported at all** (asserted).
* `ZSTD_LEGACY_SUPPORT=5`: `ZSTD_isLegacy` recognises only v0.5/v0.6/v0.7, so a
  v0.1–v0.4 magic yields `prefix_unknown` (10), **not** `version_unsupported`
  (12) — while `zstd_v01.c`…`zstd_v04.c` are still compiled and still export
  their symbols.
* `DYNAMIC_BMI2=0` and no `__BMI2__` ⇒ `ZSTD_ENABLE_ASM_X86_64_BMI2 == 0`: no
  Huffman assembly, and `HUF_flags_bmi2` / `ZSTD_d_disableHuffmanAssembly` are
  no-ops (asserted byte-identical). `HUF_DISABLE_FAST_DECODE` is *not* set, so
  the Huffman fast **C** decode loops are live and their rejections are reachable.
* `ZSTD_ARCH_X86_SSE2` *is* defined (the row-based match finder's SSE2 tag mask is
  live); `ZSTD_ARCH_X86_AVX2` is not.

## Phase B / C — the tests

| file | tests | area |
|---|---|---|
| `t00_smoke.rs` | 4 | harness; every C export is `dlsym`-reachable in the Rust `.so` |
| `t10_entropy.rs` | 27 | FSE / HUF / HIST / xxhash / POOL / divsufsort — the lowest-level exports, driven stage by stage |
| `t11_lowlevel_errors.rs` | 40 | error paths of the same, plus `zstd_cwksp`, literals/sequences encoders, ZSTDMT stubs |
| `t20_params.rs` | 46 | all 39 `ZSTD_c_*` and 7 `ZSTD_d_*` parameters: bounds, clamp-vs-reject, read-back, stage checks, out-of-range enum values |
| `t21_compress_core.rs` | 15 | one-shot compress/decompress over every level × corpus × size; frame-header shape |
| `t22_compress_matchfinders.rs` | 16 | the match-finder matrix: 9 strategies, row finder, window sliding/extDict, LDM, super-blocks, block splitters |
| `t23_streaming.rs` | 17 | `compressStream2`/`decompressStream` under randomized chunk schedules, stable buffers, `windowLogMax`, progression |
| `t24_dict.rs` | 20 | dictionary builders (byte-exact trained dictionaries) and every way to use a dictionary |
| `t25_frame_block_api.rs` | 23 | frame inspection, the block API, bufferless begin/continue/end, static contexts, custom allocators |
| `t26_sequences.rs` | 24 | the sequence API, including 1296 randomized invalid-sequence mutations |
| `t28_deprecated_legacy.rs` | 43 | the `ZBUFF_*` API and the legacy `ZSTDv0x_*` decoders (safe subset) |
| `t30_decompress_errors.rs` | 35 | the decompressor's 309 rejection sites, plus a 3200-case corruption fuzz |
| `t40_gaps.rs` | 41 | the remaining rows: `ZSTD_loadCEntropy`, the `ZSTD_ldm_*` exports, magicless format, in-place decode, literals-buffer split |

Beyond byte equality, several tests assert *structural* equality that a
functional test would miss: counting custom allocators confirm both libraries
perform the **same number of allocations of the same sizes in the same order**,
which pins the `zstd_cwksp` workspace layout; `ZSTD_estimate*Size` is compared
exactly for ~700 parameter combinations; and `ZSTD_sizeof_*` pins the translated
struct sizes.

## What was found and fixed

Five divergences, all in `src/`, all in the same class: the C relies on integer
wrap-around and the Rust wrote plain arithmetic. In the shipped profile
(`overflow-checks = false`) Rust wraps too, so four of the five were *latent* —
correct only by virtue of a compiler flag. The overflow audit turned each into a
loud panic naming the line.

1. **`common/bits.rs` — `ZSTD_countLeadingZeros32/64` at 0.** *A real behavioural
   divergence, not just a latent one.* `ZSTD_highbit32(0)` is reachable in
   contract (e.g. `FSE_optimalTableLog_internal(0, 1, 0, 0)`, and
   `FSE_buildDTable_wksp` on an all-low-probability distribution). The C is
   nominally UB, but this build's `-O0` codegen is
   `bsr -0x4(%rbp),%eax ; xor $0x1f,%eax`, and `bsr` with a zero source leaves
   its destination untouched; the only caller in the library loads `val` into
   `%eax` first, so the result is `0 ^ 31 == 31`, i.e. `ZSTD_highbit32(0) == 0`.
   31 is also exactly what `ZSTD_countLeadingZeros32_fallback(0)` computes, so the
   two C implementations agree and 31 is the value to reproduce. The Rust
   returned `leading_zeros() == 32`, giving `ZSTD_highbit32(0) == 0xFFFFFFFF`, and
   `FSE_optimalTableLog_internal(0,1,0,0)` returned 11 where the C returns 5.
   Fixed, and the DeBruijn fallback is now a faithful transliteration too.
2. **`compress/huf_compress.rs`** — `op[0] = (BYTE)(128 + (maxSymbolValue-1))` with
   `maxSymbolValue == 0`, which `HIST_count` produces for any constant input. Now
   `128u32.wrapping_add(maxSymbolValue.wrapping_sub(1))`.
3. **`common/fse_decompress.rs`** — `tableDecode[highThreshold--]` on a `U32` when
   *every* symbol is low-probability (a distribution `FSE_readNCount` accepts).
   Now `wrapping_sub`.
4. **`compress/zstd_compress_sequences.rs`** — the unchecked `[nbSeq-1]` in
   `ZSTD_encodeSequences` / `ZSTD_buildCTable`. Now an explicitly wrapping index
   and `wrapping_add` on the pointer.
5. **`compress/zstd_compress_p3.rs`, `zstd_compress_p6.rs`,
   `zstd_compress_internal.rs`** — `ZSTD_mergeBlockDelimiters`' literal-length
   sum; the missing `FORWARD_IF_ERROR` after `ZSTD_writeFrameHeader` in
   `ZSTD_compressSequences*` (an upstream defect that moves `op` 70 bytes
   *backwards*); `remaining -= block.blockSize` with no bound; and
   `matchLength - MINMATCH` in `ZSTD_storeSeqOnly`.

Nothing in `c_src/` was modified.

## Out-of-contract inputs, and why they are excluded

`DEBUGLEVEL=0` erases every `assert()`, so a number of documented preconditions
are unenforced and the reference C crashes or reads indeterminate memory rather
than returning an error. There is no C behaviour to match, so those inputs are
deliberately not driven; each exclusion is recorded at the point of exclusion in
the test and in the `reach` column of `ERRORS.md`. The main ones, each verified
against the reference `.so` rather than assumed:

* `FSE_normalizeCount(total = 0)` — `ZSTD_div64(1<<62, 0)`, SIGFPE.
* `HUF_buildCTable_wksp(maxNbBits < ceil(log2(cardinality)))` — `HUF_setMaxHeight`
  walks off `huffNode[]`, SIGSEGV. (Measured: cardinality 27 crashes at
  `maxNbBits <= 4` and succeeds from 5; 60 crashes at `<= 5`; 256 at `<= 6`.)
* Decoding with an unpopulated `HUF_DTable` (`tableLog == 0`) —
  `BIT_lookBitsFast` shifts by `(64-0) & 63 == 0` and returns the whole bit
  container as the table index.
* `XXH*_reset/update/digest(NULL, …)` — documented `@pre statePtr must not be
  NULL`, enforced only by `XXH_ASSERT`. (`XXH*_freeState(NULL)` *is* in contract
  and is tested.)
* `ZSTD_estimate*Size_usingCCtxParams` with LDM enabled but `ldmMinMatch == 0` —
  `ZSTD_ldm_getMaxNbSeq` divides by it; the estimators never call
  `ZSTD_ldm_adjustParameters`, which is what fills the default. SIGFPE.
* `ZSTD_getFrameContentSize(NULL, >= 4)` — `ZSTD_isLegacy` does `MEM_readLE32(src)`
  with no NULL guard. (`ZSTD_getFrameHeader(NULL, 8)` *is* checked, and is tested.)
* Arbitrary bytes to the legacy v0.5–v0.7 decoders. The corruption fuzz therefore
  never touches the 4 magic bytes and re-rolls any buffer containing a legacy
  magic anywhere; the legacy tests use only inputs whose rejection was confirmed
  in the source to precede any bitstream walking, plus structurally valid frames
  built from `bt_raw`/`bt_rle`/`bt_end` blocks.
* `ZSTD_c_validateSequences == 0` with mutated sequences — the header documents
  this as undefined behaviour.

Two **upstream C defects** were found. Both are faithfully reproduced by the
translation (byte-for-byte, same offsets), so neither warranted a change:
`ZSTD_compressSequences*` writing 70 bytes below `dst` when
`dstCapacity < 18` (the missing `FORWARD_IF_ERROR` above), and
`ZSTD_compressSubBlock_literal` (`zstd_compress_superblock.c:71`) `memcpy`ing the
Huffman description with no check against `dstSize`. Both are contained in the
tests by a canary past `dstCapacity`.

One place where the C is *not self-consistent* is excluded from byte comparison
with the reason recorded: on an error return from `ZSTD_decompress`, the partial
contents of `dst` are indeterminate, because `ZSTD_execSequence` wild-copies
literals (deliberately over-reading the buffer's `WILDCOPY_OVERLENGTH` padding)
*before* validating the offset. Repeating the identical call against the C `.so`
yields different trailing bytes. The suite compares the full `dst` on success and
only the API-reported observables on error.

## `ERRORS.md` accounting

| | rows |
|---|---|
| tested differentially (same error code *and* name asserted) | 956 |
| `UNREACHABLE` — dominated by an earlier check, justification in the row | 151 |
| `UNSAFE-UB` — the reference C is undefined on the only reaching input | 23 |
| `ALLOC-ONLY` — only an internal allocation failure reaches it, and no public injection point exists | 12 |
| **total** | **1142** |

The 186 exclusions each carry their justification in the `reach` cell, together
with the classification Phase A originally derived, so the correction is
auditable. Rows reachable through a `ZSTD_create*_advanced` **custom allocator**
were *not* excluded — they are tested with an `extern "C"` allocator that fails at
the Nth call, sweeping N.

## Feature combinations

`Cargo.toml` declares no `[features]` table, so there is exactly **one** valid
configuration. `feature_matrix.sh` derives that mechanically (it parses the
`[features]` table and takes the power set of the non-default features), so a
feature added later is picked up automatically instead of silently escaping the
matrix. `./feature_matrix.sh --test` runs `cargo check`, `cargo check --tests`
and the full differential suite for every enumerated combination; all pass.

## Notes for future work

* `t00_smoke.rs` intentionally emits no coverage tags — it verifies the harness
  and symbol reachability rather than a `CONFIGS`/`ERRORS` row. All twelve other
  suites write one tag file each.
* Three tests share process-wide `static`s (a sequence-producer log and two
  counting allocators) because an `extern "C"` callback has nowhere else to
  record what it saw. Each such test holds a `Mutex<()>` for its whole body; the
  guards are load-bearing, and removing one produces a phantom "divergence"
  whose compressed output is in fact identical.
* `t24_dict.rs` exercises `ZDICT_trainFromBuffer_fastCover` with `f = 31`, whose
  lazily-paged `calloc`s reserve ~12 GB of address space. It is safe, but under
  concurrent memory pressure an unrelated `create*` call elsewhere can transiently
  return `NULL`.
