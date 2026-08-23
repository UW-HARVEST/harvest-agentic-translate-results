# dictBuilder cluster contract

Read `AGENT_SPEC.md` first. Build config: ZSTD_MULTITHREAD undefined (single-thread — POOL runs jobs
synchronously via crate::common::pool), DYNAMIC_BMI2=0, LE 64-bit.

Public ZDICT types are in `c_src/src/include/zdict.h`. Define them #[repr(C)] in the module that owns
them, OR share via a small `crate::dictBuilder::zdict` types re-export. Coordinator note: to avoid
duplicate type definitions across cover/fastcover/zdict, define shared ZDICT_params_t,
ZDICT_cover_params_t, ZDICT_fastCover_params_t, ZDICT_legacy_params_t in `zdict.rs` as `pub`, and cover/
fastcover `use crate::dictBuilder::zdict::*`. COVER_* types (COVER_best_t, COVER_segment_t,
COVER_epoch_info_t, COVER_dictSelection_t) are in cover.h — define in `cover.rs` as pub; fastcover uses them.

Dependencies available:
- crate::common::{mem, bits (highbit32), zstd_internal, allocations (malloc/calloc/free/memcpy/memset/qsort),
  pool (POOL_create/POOL_add/POOL_free/POOL_sizeof/POOL_ctx), error, xxhash (ZSTD_XXH64)}.
- crate::compress::zstd_compress_internal (ZSTD_hashPtr etc for fastcover; ZSTD_loadCEntropy is in
  zstd_compress.c — declare extern "C").
- Public compress API (ZSTD_compress2, ZSTD_createCCtx, ZSTD_CCtx_*, ZSTD_compressBound,
  ZSTD_getErrorName, ZSTD_isError, ZSTD_maxCLevel, ZSTD_minCLevel) — declare extern "C" or call via
  crate::compress::zstd_compress once available. FSE_normalizeCount/writeNCount via crate::compress::fse_compress,
  HUF via crate::compress::huf_compress.

Cross-file symbols are exported #[unsafe(no_mangle)] where global in C — call siblings via extern "C" or crate path.
threading.h ZSTD_pthread_mutex_t etc: single-thread build makes these no-op types — model COVER_best_t's
mutex/cond fields as zero-sized/unit placeholders that the (synchronous) POOL path never truly blocks on;
reproduce the C control flow (which, single-threaded, runs jobs inline).

Byte-identical output. No stubs. No bug fixes. Preserve error-check order, qsort comparators (use libc qsort
with extern "C" comparator fns for identical ordering), and wrapping arithmetic.
Use INCREMENTAL writing. Do NOT edit other files or c_src/. Do NOT add mod declarations.
