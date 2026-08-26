# Translation contract (READ FIRST)

We are translating the zstd C library (in `c_src/`) to Rust, file by file, into
`src/`. The result is a `cdylib` that must export the **same symbols** and
produce **byte-identical output** as the C build.

## Golden rules

1. **Transliterate, do not redesign.** Mirror the C control flow, the C
   arithmetic, the C order of operations and the C order of error checks
   *exactly*. Do NOT fix bugs. Do NOT "improve" anything.
2. Use **raw pointers** (`*const u8`, `*mut u32`, ...) and `unsafe` blocks so
   pointer arithmetic matches C. Do not try to use slices/iterators for the
   hot paths that do pointer arithmetic.
3. Keep **C identifiers verbatim** (`ZSTD_compressBlock_fast`, `nbSeq`, `ip`,
   `oend`, ...). Lints for naming are already disabled crate-wide.
4. **Arithmetic must wrap, never panic.** The release profile has
   `overflow-checks = false`, but be explicit where C relies on wrapping:
   use `wrapping_add/sub/mul`, and `x as i32 as u32`-style casts to mirror C
   integer conversions. Signed/unsigned conversions must match C semantics
   (C `(U32)someNegativeInt` == Rust `x as u32`).
5. For pointer subtraction that may go before the start of a buffer (common in
   zstd, e.g. `iend - 8` when the buffer is shorter), use
   `wrapping_sub`/`wrapping_add`/`wrapping_offset` rather than `sub`/`add`.
6. `assert()` is compiled out (DEBUGLEVEL == 0) — **drop all asserts**.
   `DEBUGLOG`/`RAWLOG`/`ZSTD_STATIC_ASSERT`/`DEBUG_STATIC_ASSERT` — drop them.
   `ZSTD_TRACE` is 0, so all `ZSTD_trace*` calls compile out.
7. Build-time configuration for this port (**already decided, do not deviate**):
   - `ZSTD_LEGACY_SUPPORT = 5`  (so `ZSTD_LEGACY_SUPPORT >= 1..5` are true,
     `>= 6`/`>= 7`/`>= 8` are false; legacy v01..v04 *source files* are still
     compiled and still export their symbols)
   - `XXH_NAMESPACE = ZSTD_`
   - `DYNAMIC_BMI2 = 0`, `STATIC_BMI2 = 0`  → always take the "default"
     (non-bmi2) code path; keep the `bmi2`/`flags` parameters in signatures but
     ignore them, exactly like the C does when `DYNAMIC_BMI2 == 0`.
   - `ZSTD_MULTITHREAD` is **not** defined → single-threaded `POOL_*` stubs,
     `ZSTDMT_*` compiles its non-thread paths.
   - `DEBUGLEVEL = 0`, `NDEBUG`, no
     `ZSTD_STRIP_ERROR_STRINGS`, no `HUF_FORCE_DECOMPRESS_X1/X2`,
     no `ZSTD_NO_INTRINSICS`, no `ZSTD_LINUX_KERNEL`,
     no `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`,
     no `ZSTD_ADDRESS_SANITIZER` / `ZSTD_MEMORY_SANITIZER`.
   - **`ZSTD_TRACE == 1`** (gcc has weak symbols) — so `ZSTD_CCtx.traceCtx` and
     `ZSTD_DCtx.traceCtx` fields **do exist** (already in the Rust structs) and
     `sizeof` depends on them. BUT the weak hooks `ZSTD_trace_compress_begin`,
     `ZSTD_trace_compress_end`, `ZSTD_trace_decompress_begin`,
     `ZSTD_trace_decompress_end` are *undefined* (NULL) in this build, so every
     `#if ZSTD_TRACE` body reduces to: set `traceCtx = 0` (the
     `(ZSTD_trace_x_begin != NULL) ? ... : 0` test is always false) and do
     nothing on end. Translate them that way — i.e. `(*cctx).traceCtx = 0;` for
     the begin hooks and nothing at all for the end hooks. Do NOT define or
     import any `ZSTD_trace_*` symbol.
   - Target is x86_64 linux (little endian, 64-bit): `MEM_64bits()==1`,
     `MEM_isLittleEndian()==1`.
   - `ZSTD_ARCH_X86_SSE2` is **not** used for `ZSTD_copy16` (gcc path uses the
     `copy16_buf` memcpy pair) — but any of these produce identical bytes.
8. **Exported symbols**: every function that the C library exports must be
   ```rust
   #[unsafe(no_mangle)]
   pub unsafe extern "C" fn NAME(...) -> ... { }
   ```
   with the **final linker name** (e.g. legacy files use macros that rename
   `FSE_decompress` to `FSEv05_decompress`; use the renamed name).
   `static`/internal C functions become plain (non-`no_mangle`) Rust `unsafe fn`.
   Do NOT add `no_mangle` to functions that C declared `static`.
9. Structs shared across translation units, or reachable from the public API,
   must be `#[repr(C)]`.
10. Allocation must go through libc so behaviour matches:
    `crate::libc::{malloc, calloc, free, memcpy, memmove, memset, qsort, qsort_r}`
    and the helpers `crate::libc::{ZSTD_memcpy, ZSTD_memmove, ZSTD_memset}`.
    **`qsort`/`qsort_r` must be libc's** so tie-breaking matches byte-for-byte.
11. Prefer `while` loops over `for` when translating C `for` loops with
    non-trivial increments/continues, so `continue` semantics stay correct.
    Remember: C `continue` in a `for` loop still runs the increment; Rust's does
    not.

## Existing shared modules (use these, don't re-create them)

| Rust path | C origin | provides |
|---|---|---|
| `crate::libc` | `zstd_deps.h` | malloc/calloc/free/memcpy/memmove/memset/memcmp/qsort/qsort_r, `ZSTD_memcpy`, `ZSTD_memmove`, `ZSTD_memset` |
| `crate::zstd_h` | `zstd.h` | all public types/enums/constants (`ZSTD_customMem`, `ZSTD_bounds`, `ZSTD_Sequence`, `ZSTD_compressionParameters`, `ZSTD_parameters`, `ZSTD_frameParameters`, `ZSTD_FrameHeader`, `ZSTD_inBuffer`, `ZSTD_outBuffer`, `ZSTD_frameProgression`, `ZSTD_strategy`, `ZSTD_cParameter`, `ZSTD_dParameter`, `ZSTD_ResetDirective`, `ZSTD_EndDirective`, `ZSTD_dictContentType_e`, `ZSTD_dictLoadMethod_e`, `ZSTD_format_e`, `ZSTD_ParamSwitch_e`, `ZSTD_dictAttachPref_e`, `ZSTD_literalCompressionMode_e`, `ZSTD_forceIgnoreChecksum_e`, `ZSTD_refMultipleDDicts_e`, `ZSTD_SequenceFormat_e`, `ZSTD_FrameType_e`, `ZSTD_nextInputType_e`, `ZSTD_sequenceProducer_F`, `ZSTD_VERSION_*`, `ZSTD_MAGIC*`, `ZSTD_BLOCKSIZE_MAX`, `ZSTD_WINDOWLOG_*`, ...) |
| `crate::common::mem` | `mem.h` | `BYTE,U8,S8,U16,S16,U32,S32,U64,S64`, `MEM_read*/MEM_write*/MEM_readLE*/MEM_writeLE*/MEM_readBE*/MEM_writeBE*`, `MEM_32bits`, `MEM_64bits`, `MEM_isLittleEndian`, `MEM_swap*` |
| `crate::common::bits` | `bits.h` | `ZSTD_highbit32`, `ZSTD_countLeadingZeros32/64`, `ZSTD_countTrailingZeros32/64`, `ZSTD_NbCommonBytes`, `ZSTD_rotateRight_U16/32/64` |
| `crate::common::error_private` | `error_private.h`, `zstd_errors.h` | `ERROR(ZSTD_error_xxx)` (const fn), `ERR_isError`, `ERR_getErrorCode`, `ERR_getErrorName`, `ERR_getErrorString`, and every `ZSTD_error_*` constant |
| `crate::common::bitstream` | `bitstream.h` | `BIT_CStream_t`, `BIT_DStream_t`, `BIT_DStream_status` (+ `BIT_DStream_unfinished/endOfBuffer/completed/overflow`), `BIT_init*`, `BIT_addBits*`, `BIT_flushBits*`, `BIT_closeCStream`, `BIT_lookBits*`, `BIT_readBits*`, `BIT_skipBits`, `BIT_reloadDStream*`, `BIT_endOfDStream`, `BIT_mask`, `STREAM_ACCUMULATOR_MIN*`, `BitContainerType` |
| `crate::common::fse` | `fse.h` | `FSE_CTable`, `FSE_DTable`, `FSE_CState_t`, `FSE_DState_t`, `FSE_symbolCompressionTransform`, `FSE_DTableHeader`, `FSE_decode_t`, `FSE_repeat*`, all `FSE_*_SIZE*` helpers, `FSE_initCState`, `FSE_initCState2`, `FSE_encodeSymbol`, `FSE_flushCState`, `FSE_getMaxNbBits`, `FSE_bitCost`, `FSE_initDState`, `FSE_peekSymbol`, `FSE_updateState`, `FSE_decodeSymbol`, `FSE_decodeSymbolFast`, `FSE_endOfDState`, `FSE_MAX_*`, `FSE_MIN_TABLELOG`, `FSE_TABLESTEP` |
| `crate::common::huf` | `huf.h` | `HUF_CElt`, `HUF_DTable`, `HUF_CTableHeader`, `HUF_repeat*`, `HUF_flags_*`, `HUF_TABLELOG_*`, `HUF_SYMBOLVALUE_MAX`, `HUF_*_WORKSPACE_SIZE*`, `HUF_BLOCKSIZE_MAX`, `HUF_CTABLE_SIZE*`, `HUF_DTABLE_SIZE`, `HUF_COMPRESSBOUND`, `HUF_BLOCKBOUND` |
| `crate::common::zstd_internal` | `zstd_internal.h`, `allocations.h` | `MIN`, `MAX`, `BOUNDED`, `ZSTD_OPT_NUM`, `ZSTD_REP_NUM`, `repStartValue`, `BIT0..BIT7`, `ZSTD_fcs_fieldSize`, `ZSTD_did_fieldSize`, `ZSTD_FRAMEIDSIZE`, `ZSTD_BLOCKHEADERSIZE`, `ZSTD_blockHeaderSize`, `blockType_e` (`bt_raw`/`bt_rle`/`bt_compressed`/`bt_reserved`), `ZSTD_FRAMECHECKSUMSIZE`, `MIN_CBLOCK_SIZE`, `MIN_SEQUENCES_SIZE`, `MIN_LITERALS_FOR_4_STREAMS`, `SymbolEncodingType_e` (`set_basic`/`set_rle`/`set_compressed`/`set_repeat`), `LONGNBSEQ`, `MINMATCH`, `Litbits`, `LitHufLog`, `MaxLit`, `MaxML`, `MaxLL`, `MaxOff`, `DefaultMaxOff`, `MaxSeq`, `MLFSELog`, `LLFSELog`, `OffFSELog`, `MaxFSELog`, `MaxMLBits`, `MaxLLBits`, `ZSTD_MAX_HUF_HEADER_SIZE`, `ZSTD_MAX_FSE_HEADERS_SIZE`, `LL_bits`, `LL_defaultNorm`, `LL_defaultNormLog`, `LL_DEFAULTNORMLOG`, `ML_bits`, `ML_defaultNorm`, `ML_defaultNormLog`, `ML_DEFAULTNORMLOG`, `OF_defaultNorm`, `OF_defaultNormLog`, `OF_DEFAULTNORMLOG`, `ZSTD_copy8`, `ZSTD_copy16`, `WILDCOPY_OVERLENGTH`, `WILDCOPY_VECLEN`, `ZSTD_overlap_e` (`ZSTD_no_overlap`/`ZSTD_overlap_src_before_dst`), `ZSTD_wildcopy`, `ZSTD_limitCopy`, `ZSTD_WORKSPACETOOLARGE_FACTOR`, `ZSTD_WORKSPACETOOLARGE_MAXDURATION`, `ZSTD_bufferMode_e`, `ZSTD_frameSizeInfo`, `blockProperties_t`, `ZSTD_defaultCMem`, `ZSTD_customMalloc`, `ZSTD_customCalloc`, `ZSTD_customFree`. It also re-exports everything from `crate::zstd_h`. |
| `crate::common::pool` | `pool.c/h` | `POOL_ctx`, `POOL_function`, `POOL_create`, `POOL_create_advanced`, `POOL_free`, `POOL_joinJobs`, `POOL_resize`, `POOL_add`, `POOL_tryAdd`, `POOL_sizeof` |
| `crate::common::xxhash` | `xxhash.*` | `XXH32_state_t`, `XXH64_state_t`, `XXH32_canonical_t`, `XXH64_canonical_t`, `XXH_errorcode`, `XXH_OK`, and the exported `ZSTD_XXH*` functions (call them by their `ZSTD_XXH64_*` names) |
| `crate::common::entropy_common` | `entropy_common.c` | `FSE_versionNumber`, `FSE_isError`, `FSE_getErrorName`, `HUF_isError`, `HUF_getErrorName`, `FSE_readNCount`, `FSE_readNCount_bmi2`, `HUF_readStats`, `HUF_readStats_wksp` |
| `crate::common::fse_decompress` | `fse_decompress.c` | `FSE_buildDTable_wksp`, `FSE_buildDTable_internal`, `FSE_decompress_wksp_bmi2` |

| `crate::compress::zstd_cwksp` | `zstd_cwksp.h` | `ZSTD_cwksp`, `ZSTD_cwksp_alloc_phase_e`, `ZSTD_cwksp_static_alloc_e` (`ZSTD_cwksp_dynamic_alloc`/`ZSTD_cwksp_static_alloc`), and every `ZSTD_cwksp_*` helper |
| `crate::compress::zstd_compress_internal` | `zstd_compress_internal.h` | `ZSTD_CCtx`, `ZSTD_CStream`, `ZSTD_CDict`, `ZSTD_CCtx_params`, `SeqDef`, `SeqStore_t`, `ZSTD_MatchState_t`, `ZSTD_blockState_t`, `ZSTD_compressedBlockState_t`, `ZSTD_window_t`, `optState_t`, `ldmState_t`, `ldmParams_t`, `ldmEntry_t`, `ldmMatchCandidate_t`, `RawSeqStore_t`, `rawSeq`, `kNullRawSeqStore`, `ZSTD_match_t`, `ZSTD_optimal_t`, `ZSTD_entropyCTables_t`, `ZSTD_hufCTables_t`, `ZSTD_fseCTables_t`, `ZSTD_*CTablesMetadata_t`, `SeqCollector`, `ZSTD_blockSplitCtx`, `ZSTD_prefixDict`, `ZSTD_localDict`, `Repcodes_t`, `ZSTD_SequencePosition`, `BlockSummary`, `ZSTD_SequenceLength`, `ZSTD_threadPool`, `ZSTD_TraceCtx`, all the enums (`ZSTDcs_*`, `zcss_*`, `ZSTD_llt_*`, `zop_*`, `ZSTDb_*`, `ZSTD_dtlm_*`, `ZSTD_tfp_*`, `ZSTD_noDict`/`ZSTD_extDict`/`ZSTD_dictMatchState`/`ZSTD_dedicatedDictSearch`, `ZSTD_cpm_*`), the constants (`kSearchStrength`, `HASH_READ_SIZE`, `ZSTD_DUBT_UNSORTED_MARK`, `ZSTD_OPT_SIZE`, `ZSTD_WINDOW_START_INDEX`, `ZSTD_ROW_HASH_CACHE_SIZE`, `LDM_BATCH_SIZE`, `ZSTD_MAX_NB_BLOCK_SPLITS`, `COMPRESS_SEQUENCES_WORKSPACE_SIZE`, `ENTROPY_WORKSPACE_SIZE`, `TMP_WORKSPACE_SIZE`, `ZSTD_SLIPBLOCK_WORKSPACESIZE`, `ZSTD_CURRENT_MAX`, `ZSTD_CHUNKSIZE_MAX`, `ZSTD_SHORT_CACHE_TAG_BITS/MASK`), and all inlined helpers (`ZSTD_getSequenceLength`, `ZSTD_LLcode`, `ZSTD_MLcode`, `ZSTD_cParam_withinBounds`, `ZSTD_selectAddr`, `ZSTD_noCompressBlock`, `ZSTD_rleCompressBlock`, `ZSTD_minGain`, `ZSTD_literalsCompressionIsDisabled`, `ZSTD_safecopyLiterals`, `REPCODE*_TO_OFFBASE`, `REPCODE_TO_OFFBASE`, `OFFSET_TO_OFFBASE`, `OFFBASE_IS_OFFSET`, `OFFBASE_IS_REPCODE`, `OFFBASE_TO_OFFSET`, `OFFBASE_TO_REPCODE`, `ZSTD_storeSeq`, `ZSTD_storeSeqOnly`, `ZSTD_updateRep`, `ZSTD_newRep`, `ZSTD_count`, `ZSTD_count_2segments`, all `ZSTD_hash*`, `ZSTD_ipow`, `ZSTD_rollingHash_*`, all `ZSTD_window_*`, `ZSTD_matchState_dictMode`, `ZSTD_getLowestMatchIndex`, `ZSTD_getLowestPrefixIndex`, `ZSTD_index_overlap_check`, `ZSTD_writeTaggedIndex`, `ZSTD_comparePackedTags`, `ZSTD_hasExtSeqProd`, `ZSTD_BlockCompressor_f`) |
| `crate::compress::clevels` | `clevels.h` | `ZSTD_MAX_CLEVEL`, `ZSTD_defaultCParameters` |
| `crate::decompress::zstd_decompress_internal` | `zstd_decompress_internal.h` | `ZSTD_DCtx`, `ZSTD_DStream`, `ZSTD_DDict`, `ZSTD_entropyDTables_t`, `ZSTD_seqSymbol`, `ZSTD_seqSymbol_header`, `ZSTD_DDictHashSet`, `LL_base`, `OF_base`, `OF_bits`, `ML_base`, `SEQSYMBOL_TABLE_SIZE`, `ZSTD_BUILD_FSE_TABLE_WKSP_SIZE*`, `ZSTD_HUFFDTABLE_CAPACITY_LOG`, `ZSTD_LITBUFFEREXTRASIZE`, `ZSTD_DECODER_INTERNAL_BUFFER`, `ZSTDds_*`, `zdss_*`, `ZSTD_use_indefinitely`/`ZSTD_dont_use`/`ZSTD_use_once`, `ZSTD_not_in_dst`/`ZSTD_in_dst`/`ZSTD_split`, `ZSTD_DCtx_get_bmi2` |

**Every one of the structs above has been verified to have byte-identical
`sizeof`/`align`/field offsets to the C version. Do not change their field
lists or order.**

Note: `ZSTD_isError` / `FSE_isError` / `HUF_isError` used *inline* inside C code
are macros for `ERR_isError`. Prefer
`crate::common::error_private::ERR_isError(x) != 0`.

## Cross-module calls

If module A needs a function that another module B defines, and that function is
an **exported** symbol (it appears in `SYMBOLS_BY_FILE.txt`), you may either use
the Rust path `crate::compress::b::NAME` or declare
```rust
extern "C" { fn NAME(args...) -> ret; }
```
Prefer the Rust path when the target module already exists; use `extern "C"` when
it doesn't exist yet (that keeps your file compiling independently). For C
functions that were `static`, they must be private to your module.

## Verifying your work

`./check.sh <module> [<module> ...]` type-checks the named modules together with
the stable core, in a throw-away crate, so other people's in-progress files
can't break your build. Module paths are relative to `src/` without `.rs`,
e.g. `./check.sh compress/zstd_fast`.

## Idioms

```rust
// C: RETURN_ERROR_IF(cond, dstSize_tooSmall, "...");
if cond { return ERROR(ZSTD_error_dstSize_tooSmall); }

// C: FORWARD_IF_ERROR(f(...), "");
{ let err_code = f(...); if ERR_isError(err_code) != 0 { return err_code; } }

// C: CHECK_F(f(...));
{ let e = f(...); if ERR_isError(e) != 0 { return e; } }

// C: size_t const x = ...;  ->  let x: usize = ...;
// C: BYTE* op = (BYTE*)dst;  ->  let mut op = dst as *mut BYTE;
// C: op[3]                   ->  *op.add(3)
// C: *op++ = v;              ->  *op = v; op = op.add(1);
// C: (size_t)(op - ostart)   ->  op.offset_from(ostart) as usize
```

For a C `static const` lookup table use a Rust `static NAME: [T; N] = [...];`.

For C unions or nested anonymous structs, use `#[repr(C)]` structs; if a real
union is needed use `#[repr(C)] union`.

## Deliverable

Write the assigned Rust file(s) completely — **no `todo!()`, no stubs, no
"... rest omitted"**. Every function in the assigned C file must be present.
Do not edit anything under `c_src/`. Do not edit `src/lib.rs` unless told to.
