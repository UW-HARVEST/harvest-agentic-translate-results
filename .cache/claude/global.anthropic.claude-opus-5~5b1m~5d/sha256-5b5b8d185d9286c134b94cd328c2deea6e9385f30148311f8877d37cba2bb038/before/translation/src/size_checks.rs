//! Compile-time verification that our `#[repr(C)]` structs have exactly the same
//! size and layout as the C originals (measured on the reference C build:
//! x86-64 Linux, ZSTD_LEGACY_SUPPORT=5, DYNAMIC_BMI2=0, no ZSTD_MULTITHREAD,
//! ZSTD_TRACE=1). These sizes are observable through the public API
//! (ZSTD_estimateCCtxSize, ZSTD_sizeof_*, ZSTD_estimateDDictSize, ...), so they
//! must match byte for byte.
#![allow(dead_code)]

use core::mem::{offset_of, size_of};

use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::ZSTD_cwksp;
use crate::decompress::zstd_decompress_internal::*;
use crate::xxhash::{XXH32_state_t, XXH64_state_t};
use crate::zstd_h::ZSTD_FrameHeader;

macro_rules! check_size {
    ($t:ty, $n:expr) => {
        const _: () = assert!(size_of::<$t>() == $n);
    };
}

macro_rules! check_offset {
    ($t:ty, $f:ident, $n:expr) => {
        const _: () = assert!(offset_of!($t, $f) == $n);
    };
}

check_size!(ZSTD_CCtx, 5280);
check_size!(ZSTD_CDict, 6080);
check_size!(ZSTD_DCtx, 95992);
check_size!(ZSTD_CCtx_params, 224);
check_size!(ZSTD_cwksp, 72);
check_size!(ZSTD_MatchState_t, 304);
check_size!(ZSTD_compressedBlockState_t, 5632);
check_size!(ZSTD_entropyCTables_t, 5616);
check_size!(ZSTD_hufCTables_t, 2064);
check_size!(ZSTD_fseCTables_t, 3552);
check_size!(SeqStore_t, 80);
check_size!(SeqDef, 8);
check_size!(ldmState_t, 2112);
check_size!(ldmParams_t, 24);
check_size!(optState_t, 104);
check_size!(ZSTD_blockSplitCtx, 1496);
check_size!(ZSTD_entropyCTablesMetadata_t, 312);
check_size!(ZSTD_hufCTablesMetadata_t, 144);
check_size!(ZSTD_fseCTablesMetadata_t, 168);
check_size!(ZSTD_window_t, 40);
check_size!(ZSTD_blockState_t, 320);
check_size!(RawSeqStore_t, 40);
check_size!(SeqCollector, 32);
check_size!(ZSTD_localDict, 40);
check_size!(ZSTD_prefixDict, 24);
check_size!(XXH64_state_t, 88);
check_size!(XXH32_state_t, 48);
check_size!(ZSTD_optimal_t, 28);
check_size!(ZSTD_match_t, 8);

check_size!(ZSTD_entropyDTables_t, 27292);
check_size!(ZSTD_seqSymbol, 8);
check_size!(ZSTD_DDict, 27352);
check_size!(ZSTD_FrameHeader, 48);

check_offset!(ZSTD_CCtx, workspace, 704);
check_offset!(ZSTD_CCtx, seqStore, 976);
check_offset!(ZSTD_CCtx, ldmState, 1056);
check_offset!(ZSTD_CCtx, blockState, 3224);
check_offset!(ZSTD_CCtx, blockSplitCtx, 3768);
check_offset!(ZSTD_CCtx, extSeqBuf, 5264);

check_offset!(ZSTD_DCtx, entropy, 32);
check_offset!(ZSTD_DCtx, litExtraBuffer, 30388);
check_offset!(ZSTD_DCtx, headerBuffer, 95956);
check_offset!(ZSTD_DCtx, oversizedDuration, 95976);

check_offset!(ZSTD_CDict, matchState, 104);
check_offset!(ZSTD_CDict, cBlockState, 408);
check_offset!(ZSTD_CDict, useRowMatchFinder, 6072);
