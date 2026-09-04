# VERIFICATION.md — completion gate

Differential verification of the Rust translation of zstd 1.5.7 against the C
reference. Every assertion below is produced by running BOTH shared libraries in
one process through `libloading`/`dlsym` and comparing results:

* C reference : `c_src/build/libzstd.so`
  (built with `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`,
  no `ZSTD_MULTITHREAD`, no `-DNDEBUG` so `DEBUGLEVEL` is unset and `assert()`
  is a no-op everywhere except `dictBuilder/divsufsort.c`)
* Rust under test : `translation/target/release/libzstd.so`

No test ever calls a Rust function directly and the crate is `crate-type =
["cdylib"]` only, so the `#[no_mangle] extern "C"` export wrappers are what is
exercised. The two objects are provably distinct (`SONAME libzstd.so` on the C
side, none on the Rust cdylib;
`tests/phase_b_streaming.rs::sanity_two_distinct_libraries` asserts differing
symbol addresses).

## Gate

- [x] **`SYMBOLS.md`**: `nm -D --defined-only` — C exports **615**, Rust exports
      **615**, **0 missing**, **0 extra**. `tests/phase_a_smoke.rs` re-checks
      this two ways at test time (`nm` diff and a `dlsym` probe of all 615
      names), and both `g_debuglevel` / `g_ZSTD_threading_useless_symbol` data
      symbols are compared by value. 0 non-libc undefined symbols in the Rust
      `.so`.
- [x] **Phase B**: all **171** rows of `CONFIGS.md` pass, each across randomized
      inputs with a fixed seed.
- [x] **Phase C**: all **1269** rows of `ERRORS.md` have a passing error-path
      differential test that compares the exact `ZSTD_ErrorCode` / sentinel
      (never merely "both failed"), plus the destination buffer and every
      out-param.
- [x] **Every feature combination**: `Cargo.toml` declares **no `[features]`**
      and `grep -rn 'cfg(feature' src/` finds **0** hits, so there is exactly
      **one** configuration. `./run_all_features.sh` derives the list
      mechanically and runs check + build + symbol diff + full test suite for
      each combination; it reports `ALL FEATURE COMBINATIONS PASSED (1
      combination(s))`.

## Test inventory (287 tests, all passing)

| file | tests | scope |
|---|---:|---|
| `tests/phase_a_smoke.rs` | 4 | symbol parity via `nm` and `dlsym`, data symbols, smoke round trip |
| `tests/phase_b_core.rs` | 24 | CONFIGS rows 1–30: version/bounds/pure helpers, one-shot compress+decompress, frame introspection |
| `tests/phase_b_params.rs` | 24 | rows 31–64: `ZSTD_CCtx` parameter cross-product, `ZSTD_CCtx_params` object |
| `tests/phase_b_streaming.rs` | 19 | rows 65–81: `compressStream2` endOp scripts, legacy streaming API, `decompressStream`, stable buffers |
| `tests/phase_b_bufferless.rs` | 16 | rows 82–96: bufferless compress/decompress, block API, seq store, block splitter |
| `tests/phase_b_static.rs` | 4 | rows 97–102: `ZSTD_initStatic*` over workspace-size ladders |
| `tests/phase_b_dict.rs` | 20 | rows 103–119: CDict/DDict lifecycles, `forceAttachDict`, `refMultipleDDicts`, dictIDs, `loadCEntropy`/`loadDEntropy` |
| `tests/phase_b_sequences.rs` | 5 | rows 120–125: `generateSequences`, `compressSequences(AndLiterals)`, LDM entry points |
| `tests/phase_b_entropy.rs` | 17 | rows 126–146: XXH32/64, HIST, all 16 exported FSE and 28 exported HUF entry points, `ZSTD_buildFSETable`, matchfinder table builders, all 41 `ZSTD_compressBlock_*` |
| `tests/phase_b_dictbuilder.rs` | 12 | rows 147–156: `ZDICT_*` trainers, `COVER_*`, `divsufsort`/`divbwt` |
| `tests/phase_b_legacy.rs` | 32 | rows 157–165: all **206** exported `ZSTDv0*`/`ZBUFFv0*`/`FSEv0*`/`HUFv0*` symbols + the legacy dispatch path |
| `tests/phase_b_misc.rs` | 5 | rows 166–171: ZBUFF round trips, ZSTDMT and POOL in the non-MT build |
| `tests/phase_c_params.rs` | 6 | `parameter_outOfBound` / `_unsupported` / `_combination_unsupported`, every out-of-range enum across the FFI |
| `tests/phase_c_compress.rs` | 21 | compression-side rejections: `dstSize_tooSmall`, `srcSize_wrong`, `stage_wrong`, `stabilityCondition_notRespected`, `memory_allocation` (budgeted allocator), `workSpace_tooSmall`, `externalSequences_invalid`, `sequenceProducer_failed`, `cannotProduce_uncompressedBlock`, every `create*`/`initStatic*` NULL return |
| `tests/phase_c_decompress.rs` | 10 | `prefix_unknown`, truncation, `checksum_wrong` (every bit of the trailer), `dstSize_tooSmall`, `frameParameter_windowTooLarge`, `dictionary_wrong`/`_corrupted`, corruption fuzzing, block-level errors |
| `tests/phase_c_entropy.rs` | 36 | every rejection site in `entropy_common.c`, `fse_decompress.c`, `bitstream.h`, `fse_compress.c`, `huf_compress.c`, `hist.c`, `huf_decompress.c` |
| `tests/phase_c_dictbuilder.rs` | 19 | every rejection site in `zdict.c`, `cover.c`, `fastcover.c`, `divsufsort.c`, `zstd_ddict.c` |
| `tests/phase_c_nulls.rs` | 7 | generic boundaries: `free(NULL)`, `sizeof(NULL)`, the `(src,srcSize)` family, the `(dst,cap,src,n)` matrix, NULL dictionaries, oversized lengths, NULL out-params |
| `tests/phase_c_misc.rs` | 6 | `error_private.h`, `xxhash.h`, `pool.c`, `threading.c`, `zstd_cwksp.h`, `zstdmt_compress.c`, ZBUFF error paths |

## Divergences found in the Rust translation

**None.** Every difference investigated during this campaign resolved to one of:

1. a **C precondition violation** by the test itself (13 rows, table `P1`–`P13`
   in `CONFIGS.md`),
2. an **upstream C crash / out-of-bounds access** that both libraries reproduce
   identically (13 rows, table `X1`–`X13` in `CONFIGS.md`), or
3. a value the C leaves **unspecified** — uninitialised struct fields or
   uninitialised context memory leaked into `dst` on an error path (table
   `U1`–`U2` in `CONFIGS.md`; for `U2` the Rust was *proved* faithful by
   re-running the same inputs through `ZSTD_initStaticDCtx` /
   `ZSTD_initStaticDStream` over a workspace pre-filled with 0x11 / 0x22 / 0x00,
   where C and Rust emit byte-identical output).

Notable upstream defects surfaced along the way (all documented with file and
line evidence in `CONFIGS.md`): a 70-byte write **below** `dst` in
`ZSTD_compressSequences` on a short destination; a super-block write **past**
`dst + dstCapacity` when `ZSTD_c_targetCBlockSize` is set; a
`stableIn_notConsumed` underflow that survives `ZSTD_CCtx_reset`; and five
divide-by-zero / OOB sites reachable only by violating documented
preconditions.

## Suite sensitivity

Mutation-tested: replacing `ZSTD_hash4` with a constant fails **38+ tests across
5 files**. See "Suite sensitivity (mutation testing)" in `CONFIGS.md`, including
the two *semantically neutral* mutations that correctly fail nothing and the
row-32/33 coverage gap the experiment exposed.

## Reproducing

```sh
# build the C reference
(cd c_src && mkdir -p build && cd build \
   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . -j"$(nproc)")

# build + verify the Rust port, over every feature combination
cd translation && ./run_all_features.sh

# regenerate the Phase A artifacts (see tools/README.md)
python3 translation/tools/extract2.py
python3 translation/tools/gen_errors.py
python3 translation/tools/gen_symbols.py
```
