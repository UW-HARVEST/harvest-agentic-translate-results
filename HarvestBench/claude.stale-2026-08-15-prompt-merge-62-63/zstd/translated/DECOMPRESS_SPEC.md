# Decompress cluster contract

Shared module ALREADY EXISTS: `crate::decompress::zstd_decompress_internal` (import as
`use crate::decompress::zstd_decompress_internal::*;`). It provides:
- Base tables: `LL_base`, `OF_base`, `OF_bits`, `ML_base`, consts `LLFSELog`,`OffFSELog`,`MLFSELog`.
- `ZSTD_seqSymbol` {nextState:u16, nbAdditionalBits:u8, nbBits:u8, baseValue:u32},
  `ZSTD_seqSymbol_header` {fastMode:u32, tableLog:u32}, `seqsymbol_table_size(log)`.
- `ZSTD_entropyDTables_t`, `HUF_DTable`=u32, workspace size consts.
- `ZSTD_DCtx`/`ZSTD_DCtx_s` (FULL struct, layout verified == C, size 95992). Fields listed in the file.
- `ZSTD_DDict`/`ZSTD_DDict_s` (dictBuffer,dictContent,dictSize,entropy,dictID,entropyPresent,cMem).
- `ZSTD_DDictHashSet`, `ZSTD_FrameHeader`, dStage/dStreamStage/dictUses/litLocation consts,
  `ZSTD_frame`/`ZSTD_skippableFrame`, `ZSTD_dctx_get_bmi2()`->0.
- Various size consts: `ZSTD_FRAMEHEADERSIZE_MAX`=18, `ZSTD_LITBUFFEREXTRASIZE`, `WILDCOPY_OVERLENGTH`, etc.

Common layer (crate::common::*): error{code,error,err_is_error,err_get_error_name}, mem::*,
bits::{highbit32,...}, bitstream::*, fse::* (FSE_DTable, fse inlines, FSE_readNCount etc exported),
huf_common::* (HUF flags/consts, HUF_readStats_wksp), allocations (malloc/calloc/free/memcpy/memmove/memset,
ZSTD_customMem, zstd_custom_malloc/calloc/free), zstd_internal::* (MINMATCH, MaxLL/ML/Off, repStartValue,
ZSTD_blockHeaderSize, blockType consts bt_raw/rle/compressed/reserved, WILDCOPY, zstd_wildcopy, etc).
Public types in crate::zstd_h::* (ZSTD_inBuffer/outBuffer, ZSTD_DCtx params enums, magic numbers,
ZSTD_CONTENTSIZE_UNKNOWN/ERROR, ZSTD_dParameter values, format/checksum enums).

HUF decode fns are exported C symbols by crate::decompress::huf_decompress (HUF_decompress1X_usingDTable,
HUF_decompress4X_usingDTable, HUF_decompress1X1_DCtx_wksp, HUF_decompress1X_DCtx_wksp,
HUF_decompress4X_hufOnly_wksp, HUF_readDTableX1_wksp, HUF_readDTableX2_wksp, HUF_selectDecoder) — call them.

Legacy decoders exported by crate::legacy::zstd_vNN (ZSTDv0N_..., ZBUFFv0N_...). Use for legacy frames.

Cross-file: functions defined in one decompress file but used by another are exported C symbols
(#[unsafe(no_mangle)]) so you can just declare `extern "C" { fn ZSTD_xxx(...) -> ...; }` OR call via
`crate::decompress::<module>::ZSTD_xxx`. Prefer calling the sibling module path when the sibling exports it.

Build config: DYNAMIC_BMI2=0, ZSTD_LEGACY_SUPPORT=5, single-threaded, no ASM. Take default code paths.
LE 64-bit. Byte-identical. No stubs. No bug fixes. Preserve error-check order & wrapping arithmetic.
Use INCREMENTAL writing (Write header + `// __APPEND_HERE__` marker, then Edit repeatedly) to avoid timeouts.
