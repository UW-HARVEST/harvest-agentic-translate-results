//! Compile-time assertions that the Rust struct layouts match the C ones.
//!
//! The expected numbers come from `tests/layout_probe.c`, which is compiled
//! against the original headers in `c_src/`.
#![allow(dead_code)]

use crate::allocations::ZSTD_customMem;
use crate::xxhash::XXH64_state_t;
use crate::zstd_decompress_internal::*;
use crate::zstd_public::*;

macro_rules! assert_size {
    ($t:ty, $n:expr) => {
        const _: () = assert!(core::mem::size_of::<$t>() == $n);
    };
}

macro_rules! assert_offset {
    ($t:ty, $f:ident, $n:expr) => {
        const _: () = assert!(core::mem::offset_of!($t, $f) == $n);
    };
}

assert_size!(ZSTD_seqSymbol, 8);
assert_size!(ZSTD_entropyDTables_t, 27292);
assert_size!(ZSTD_DCtx, 95992);
assert_size!(ZSTD_DDict, 27352);
assert_size!(ZSTD_FrameHeader, 48);
assert_size!(XXH64_state_t, 88);
assert_size!(ZSTD_customMem, 24);

const _: () = assert!(ZSTD_LITBUFFEREXTRASIZE == 65536);
const _: () = assert!(ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32 == 157);
const _: () = assert!(crate::huf::HUF_DECOMPRESS_WORKSPACE_SIZE_U32 == 640);

assert_offset!(ZSTD_DCtx, LLTptr, 0);
assert_offset!(ZSTD_DCtx, entropy, 32);
assert_offset!(ZSTD_DCtx, workspace, 27324);
assert_offset!(ZSTD_DCtx, previousDstEnd, 29888);
assert_offset!(ZSTD_DCtx, expected, 29920);
assert_offset!(ZSTD_DCtx, fParams, 29928);
assert_offset!(ZSTD_DCtx, processedCSize, 29976);
assert_offset!(ZSTD_DCtx, bType, 29992);
assert_offset!(ZSTD_DCtx, stage, 29996);
assert_offset!(ZSTD_DCtx, xxhState, 30008);
assert_offset!(ZSTD_DCtx, headerSize, 30096);
assert_offset!(ZSTD_DCtx, format, 30104);
assert_offset!(ZSTD_DCtx, litPtr, 30120);
assert_offset!(ZSTD_DCtx, customMem, 30128);
assert_offset!(ZSTD_DCtx, litSize, 30152);
assert_offset!(ZSTD_DCtx, staticSize, 30168);
assert_offset!(ZSTD_DCtx, isFrameDecompression, 30176);
assert_offset!(ZSTD_DCtx, ddictLocal, 30184);
assert_offset!(ZSTD_DCtx, dictID, 30200);
assert_offset!(ZSTD_DCtx, dictUses, 30208);
assert_offset!(ZSTD_DCtx, ddictSet, 30216);
assert_offset!(ZSTD_DCtx, refMultipleDDicts, 30224);
assert_offset!(ZSTD_DCtx, disableHufAsm, 30228);
assert_offset!(ZSTD_DCtx, maxBlockSizeParam, 30232);
assert_offset!(ZSTD_DCtx, streamStage, 30236);
assert_offset!(ZSTD_DCtx, inBuff, 30240);
assert_offset!(ZSTD_DCtx, maxWindowSize, 30264);
assert_offset!(ZSTD_DCtx, outBuff, 30272);
assert_offset!(ZSTD_DCtx, lhSize, 30304);
assert_offset!(ZSTD_DCtx, legacyContext, 30312);
assert_offset!(ZSTD_DCtx, previousLegacyVersion, 30320);
assert_offset!(ZSTD_DCtx, hostageByte, 30328);
assert_offset!(ZSTD_DCtx, noForwardProgress, 30332);
assert_offset!(ZSTD_DCtx, outBufferMode, 30336);
assert_offset!(ZSTD_DCtx, expectedOutBuffer, 30344);
assert_offset!(ZSTD_DCtx, litBuffer, 30368);
assert_offset!(ZSTD_DCtx, litBufferEnd, 30376);
assert_offset!(ZSTD_DCtx, litBufferLocation, 30384);
assert_offset!(ZSTD_DCtx, litExtraBuffer, 30388);
assert_offset!(ZSTD_DCtx, headerBuffer, 95956);
assert_offset!(ZSTD_DCtx, oversizedDuration, 95976);
assert_offset!(ZSTD_DCtx, traceCtx, 95984);

assert_offset!(ZSTD_DDict, entropy, 24);
assert_offset!(ZSTD_DDict, dictID, 27316);
assert_offset!(ZSTD_DDict, cMem, 27328);
