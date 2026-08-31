# Differential verification report — zstd v1.5.7 C → Rust

**Result: the Rust translation is byte-identical to the C on every input tested.
No divergence was found, and no change to `translation/src/**` was required.**

Reproduce everything with:

```sh
cd translation && ./run_tests.sh        # builds both .so, checks parity, runs 161 tests
cd translation && ./tools/coverage.sh   # measures true runtime symbol coverage
```

## How it is tested

Both libraries are built as shared objects and loaded side by side with
`libloading`:

| | path |
|---|---|
| C | `c_src/build/libzstd.so` (cmake, `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`, no `ZSTD_MULTITHREAD`) |
| Rust | `translation/target/release/libzstd.so` (`cargo build --release`) |

**The Rust crate is never linked or called directly.** Every call goes through
`dlsym` on the `cdylib`'s exports, so the `#[no_mangle]`/`extern "C"` wrappers
and the ABI of every struct passed by value are part of what is under test. The
shared harness is `tests/common/mod.rs`.

`Cargo.toml` declares **no `[features]`**, so there is exactly one build
configuration; the "repeat under every feature combination" requirement is
satisfied trivially and is noted rather than looped over.

## Completion gate

| gate | status | evidence |
|---|---|---|
| `nm -D`: 0 missing / 0 extra symbols in Rust | **PASS** | 615 C exports, 615 Rust exports, diff empty both ways. `SYMBOLS.md`; enforced as a test in `tests/t99_symbols.rs` |
| 0 undefined non-libc symbols in the Rust `.so` | **PASS** | all undefined entries are glibc / Rust-runtime (`SYMBOLS.md` §1) |
| Every `CONFIGS.md` row covered | **PASS** | 849 rows: 756 `[x]` direct, 93 `[i]` indirect, 0 unmarked |
| Every `ERRORS.md` row covered | **PASS** | 1227 rows: 817 `[x]` direct, 409 `[i]` indirect, 1 `[n/a]`, 0 unmarked |
| All of the above under every feature combination | **PASS (vacuous)** | no `[features]` in `Cargo.toml` — single configuration |

161 tests across 18 binaries, 0 failed:

| file | tests | area |
|---|---|---|
| `t00_meta` | 4 | proves the harness is **not vacuous** (comparator really fails; missing symbols panic; levels really change output) |
| `t01_basic` | 6 | version/error surface, `compressBound`, one-shot round trip over 8 shapes × 10 sizes × 15 levels, full truncation sweep, bit-flip corruption sweep |
| `t02_params` | 7 | every `ZSTD_c_*` id incl. out-of-range, bounds ± 1, `CCtxParams`, ~90 configuration rows through `ZSTD_compress2`, pledged size, dst-too-small sweep |
| `t03_streaming` | 7 | `compressStream2`/`decompressStream` stepped in **lockstep** (rc + in.pos + out.pos compared per call), option matrix, legacy wrappers, multi-frame, stable-buffer violations |
| `t04_dict` | 15 | 22-dictionary corpus × `dct`/`dlm` × levels × strategies × attach prefs; CDict/DDict; refPrefix; wrong-dict rejection |
| `t05_entropy` | 24 | all 70 exported FSE/HUF/HIST/xxhash symbols; table bytes compared and cross-fed between libraries |
| `t06_frame` | 8 | frame introspection on valid + malformed + truncated buffers, `getCParams`/`adjustCParams` (struct-by-value ABI), estimators, static init, skippable frames, raw block API |
| `t07_dictbuilder` | 19 | `ZDICT_*` / `COVER_*` / fastcover, full parameter axes, produced dictionaries byte-compared and round-tripped |
| `t08_legacy` | 18 | all 227 `ZSTDv0x_*`/`ZBUFFv0x_*`/`FSEv0x_*`/`HUFv0x_*` exports, incl. hand-built decodable v05–v07 frames |
| `t09_zbuff` | 10 | deprecated ZBUFF state machine, per-step return/consumed/produced/bytes, misuse ordering |
| `t10_bufferless` | 3 | `compressBegin`/`Continue`/`End` and the `nextSrcSizeToDecompress`/`nextInputType` decoder state machine, compared step by step |
| `t11_sequences` | 10 | `generateSequences` arrays compared element-wise, `mergeBlockDelimiters`, `compressSequences(AndLiterals)` |
| `t12_misc` | 7 | `POOL_*` single-threaded stubs, `divsufsort`, `divbwt`, `ERR_getErrorString`, exported globals |
| `t13_advanced` | 8 | custom allocators (incl. a failing allocator to drive `memory_allocation`), struct-by-value setters, `*_simpleArgs`, all `initCStream_*`/`reset*Stream`, `DCtx` accessors, block-header helpers |
| `t14_gaps` | 6 | literal block writers, `compressBegin_advanced`, `_public`/`_deprecated` aliases, `CCtxParams` estimators, `decodeSeqHeaders`, sequence-producer hook, `selectBlockCompressor` dispatch table |
| `t15_internals` | 7 | `get1BlockSummary`, `convertBlockSequences`, `checkContinuity`, `decodeLiteralsBlock_wrapper`, `crossEntropyCost`, `splitBlock`, `selectEncodingType` |
| `t99_symbols` | 2 | all 615 exports resolvable in **both** libraries |

## Anti-vacuity measures

A differential suite that silently compares nothing is the main risk, so:

* `t00_meta` asserts the comparator itself detects length, content and scalar
  differences, that `pair()` panics on a missing symbol, and that compression
  levels actually change the output.
* `Impls::pair()` **panics** if either library lacks the symbol — a parity gap
  becomes a test failure, never a skip.
* `t10_bufferless` asserts the C compressor did not error mid-frame, after an
  undersized-output bug in the harness made both libraries "agree" on a
  truncated frame.
* `t14::select_block_compressor_partition_matches` asserts at least 8 distinct
  block compressors are found, so the equivalence-class comparison cannot pass
  trivially.
* `tools/coverage.sh` measures **runtime** symbol usage (names are often built
  with `format!`, so a static grep undercounts), and excludes `t99_symbols`
  because it probes all 615 by design.

## Symbol coverage

534 of the 615 exports are called **directly**. The remaining 81 are
internal-linkage APIs whose signatures contain private struct types
(`ZSTD_MatchState_t*`, `SeqStore_t*`, `RawSeqStore_t*`, `ZSTD_entropyCTables_t*`,
`ZSTD_CCtx_params*`, `ZSTDMT_CCtx*`, …) with no public layout — no external
consumer can build a valid argument for them. They are reached only through the
public API and so are covered indirectly. Full per-symbol accounting and the
group-by-group justification is in `SYMBOLS.md` §3.

## Findings

### No Rust bugs

`translation/src/**` is byte-for-byte unmodified. Across ~10^6 differential
comparisons — including compressed-frame bytes, entropy tables, suffix arrays,
sequence arrays, streaming call traces and error codes — the Rust matched the C
exactly. `c_src/` was never modified.

### Places where the **C itself** is undefined, and the tests stay in contract

`DEBUGLEVEL == 0` in this build, so zstd's `assert()`s are compiled out and
several functions have unchecked preconditions. Each was diagnosed by probing
the C `.so` **alone**, confirming the C crashes or reads garbage there, and then
bounding the test to the defined domain with the reason recorded inline. The
Rust reproduces the C faithfully in every case; these are not divergences.

| function | precondition the C does not enforce | observed |
|---|---|---|
| `ZSTD_compressRleLiteralsBlock` | `dstCapacity >= 4` (only `assert`ed, then `(void)dstCapacity`) and `srcSize >= 1` (dereferences `*src` unconditionally) | segfaults in **both** libraries |
| `ZSTD_selectBlockCompressor` | `dictMode <= 3`, `strategy <= 9` — raw index into `blockCompressor[4][ZSTD_STRATEGY_MAX+1]` | reads past the table; Rust panics, C returns garbage |
| `ZSTD_estimateCCtxSize_usingCCtxParams` | with LDM enabled, **all four** `ZSTD_c_ldm*` params must be set — it divides by `ldmParams.hashRateLog` without running `ZSTD_ldm_adjustParameters` | SIGFPE in the C with 0–3 of them set |
| `ZSTD_splitBlock` | `0 <= level <= 4` (`assert`ed) and `blockSize == 128 KB` | segfaults at level 5/6 |
| `ZSTD_crossEntropyCost` | `accuracyLog <= 8` and every `norm[]` entry `< 1 << accuracyLog`, so `norm256 < 256` | reads past `kInverseProbabilityLog256[256]` |
| `ZSTD_entropyCost` (via `ZSTD_selectEncodingType`) | `nbSeq` must be the true sum of `count[]`; any `count[s] >= total` makes `norm >= 256` | same table overrun |
| `ZSTD_selectEncodingType` | `defaultNormLog` ∈ {5,6}; `prevCTable` must be a **real** `FSE_CTable` when `repeatMode != none` | walks an invalid table |
| `ZSTD_decompressStream` | `input->src` may not be NULL when `input->size > 0` | segfaults in both |
| `ZSTD_get1BlockSummary` | on its error path `blockSize`/`litSize` are **never assigned** — only `nbSequences` is meaningful | C returns stack garbage; the test compares only `nbSequences` there |
| `FSEv0X_buildDTable` | `tableLog >= FSE_MIN_TABLELOG` | reads uninitialised `symbolNext[]` |
| `HUFv07_selectDecoder` | documented `0 < cSrcSize < dstSize <= 128 KB` | SIGFPE at `dstSize == 0` |
| `HIST_countFast*` / `HIST_count_simple` | `maxSymbolValue <= 255` | `memmove` past a 4 KB workspace |
| `ZSTD_compressSequences{,AndLiterals}` | `dstCapacity >= 18`; only `assert`s the frame-header result | advances `op` by a negated error code |
| `ZDICT_trainFromBuffer` | total sample size validated against `MAX(d,8)`, but the dmer count derives from the *training* subset, so `splitPoint < 1` can make it 0 | SIGFPE in `COVER_computeEpochs` |
| `ZSTD_estimateCCtxSize` (and `CStreamSize`) | not UB, but the body loops `level = 1 .. compressionLevel`, so `i32::MAX` spins ~2^31 times **in the C** | level sweep capped at 200 |

### Behaviours worth flagging (faithfully reproduced)

* `ZSTD_c_compressionLevel` is **clamped**, not rejected, unlike every other
  bounded parameter.
* Out-of-range `ZSTD_ResetDirective` values are **silently ignored** (return 0);
  `ZSTD_dictContentType_e` / `ZSTD_dictLoadMethod_e` degrade permissively rather
  than erroring.
* `XXH_ERROR` is **dead code** in this build (`XXH_DEBUGLEVEL == 0`); the only
  failure signal is `createState()` returning NULL. `XXH_NO_XXH3` is forced, so
  no `ZSTD_XXH3_*` symbols exist.
* `ZBUFF_compressInit_advanced` passes `fParams.noDictIDFlag` straight into
  `ZSTD_c_dictIDFlag` **without inverting it** (`zbuff_compress.c:91`).
* `ZDICT_finalizeDictionary` inserts zero padding *before* the content when
  `dictContentSize < 8`.
* v0.7 one-shot vs streaming disagree: `ZSTDv07_decompressFrame` decodes RLE
  blocks but skips the checksum; `ZSTDv07_decompressContinue` verifies the
  checksum but rejects RLE with `GENERIC`.
* `ZSTD_getDecompressedSize_legacy` always returns 0 for v0.5.
* v0.2/v0.3 `ZSTDv0X_decompressDCtx` are declared but never defined.
* `POOL_*` compiles its single-threaded stub branch: `POOL_create*` returns the
  file-static `g_poolCtx` singleton and `POOL_add`/`POOL_tryAdd` run the job
  synchronously — all asserted explicitly.
* `ZSTD_createThreadPool`/`ZSTD_freeThreadPool` are inside
  `#ifdef ZSTD_MULTITHREAD` and are **absent from both** libraries; only
  `ZSTD_CCtx_refThreadPool` is exported.

### One deliberately relaxed assertion, with evidence

`cover.c:67` / `fastcover.c:54` keep a **file-static** `g_displayLevel` that
`ZDICT_optimizeTrainFromBuffer_*` copies back into the caller's
`parameters->zParams.notificationLevel`. Because `cargo test` runs tests on
parallel threads, one test's notification level leaked into another's — making
results nondeterministic *within a single library*. This was fixed properly with
a process-wide mutex around the COVER/FASTCOVER trainers rather than by weakening
the check, so all dictionary assertions remain full byte equality.
`t07::same_library_results_are_deterministic` calls all seven trainers twice
within each library and requires bit-identical output, which is the evidence
that byte-equality is a legitimate expectation.

## Artifacts

| file | contents |
|---|---|
| `SYMBOLS.md` | 615-row symbol table, parity proof, and justification for the 81 indirectly-covered symbols |
| `ERRORS.md` | 1227-row error-surface table derived by grepping every `RETURN_ERROR*`, `ERROR(`, `return NULL/-1`, `assert`, range check and gating constant, with `file:line` citations |
| `CONFIGS.md` | 849-row configuration-surface table: every runtime option, input shape and entry point the C branches on |
| `run_tests.sh` | builds both `.so`, checks symbol parity, runs the suite |
| `tools/coverage.sh` | measures true runtime symbol coverage |
