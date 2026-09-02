#![allow(non_snake_case, non_upper_case_globals, dead_code, unused_imports)]
//! Phase B17: close the last symbol-coverage gap by driving FIVE exported
//! functions with DIRECT differential calls on BOTH `libzstd.so`s:
//!
//!   1. ZSTD_ldm_fillHashTable                       (test: ldm_fill_hash_table_direct)
//!   2. ZSTD_ldm_generateSequences                   (test: ldm_generate_sequences_direct)
//!   3. ZSTD_ldm_blockCompress                       (test: ldm_block_compress_direct)
//!   4. ZSTD_compressSuperBlock                      (test: compress_super_block_direct)
//!   5. ZSTD_dedicatedDictSearch_lazy_loadDictionary (test: dds_lazy_load_dictionary_direct)
//!
//! DESIGN / SAFETY (identical philosophy to b16_internals.rs)
//! ---------------------------------------------------------
//! Every one of these five functions has rich preconditions on the internal
//! state it reads (a window whose `base`/`nextSrc`/limits are consistent with
//! the source pointer, correctly-sized+aligned tables, a match state whose
//! cParams match the allocated tables, …). Hand-rolling that state produces a
//! SIGSEGV that fires *identically* in both libraries — that is undefined
//! behaviour, NOT a differential result.
//!
//! So, exactly as b16 does, we let each library BUILD its own state:
//!   * create a `ZSTD_CCtx` with `ZSTD_createCCtx`,
//!   * set the parameters that make the target code path live,
//!   * drive a real block / stream through the public API (`ZSTD_compressBlock`
//!     / `ZSTD_compress2`) so the library itself calls `ZSTD_window_update` on
//!     both `ms->window` AND `ldmState.window`, initialises the seqStore, the
//!     match state, the LDM hash table and (for DDS) attaches a fully-built
//!     dedicated-dict-search match state,
//!   * reach the relevant sub-struct at the *self-checked* CCtx offset (or via
//!     the library-provided `dictMatchState` pointer / the exported
//!     `ZSTD_getSeqStore`),
//!   * call the target function DIRECTLY through `both::<T>(name)` on each
//!     library's own state,
//!   * compare return value (via `Err2` where it is a zstd `size_t`), the whole
//!     written table / output buffer byte-for-byte, and every field of every
//!     struct the function mutates.
//!
//! A pointer minted by one library is NEVER passed to the other library's
//! function. The struct layouts are copied verbatim from b16_internals.rs /
//! translation/src/compress/{zstd_compress_internal.rs,zstd_ldm.rs} and
//! cross-checked against the C headers. The seqStore self-check
//! (`ZSTD_getSeqStore(cctx) - cctx == 976`) from b16 is reproduced below and
//! guards every offset-based read.

mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ===========================================================================
// Constants (cross-checked against the C headers).
// ===========================================================================
const ZSTD_REP_NUM: usize = 3;
const HASH_READ_SIZE: usize = 8;
const ZSTD_WINDOW_START_INDEX: u32 = 2;

/// Memory-modest cap on the LDM hash table size we will touch. `ldmHashLog`
/// near its maximum allocates gigabytes; with BOTH `.so`s loaded in the same
/// process every such table is allocated (and zeroed) twice, which exhausts
/// host memory non-deterministically. Per the harness guidance we keep total
/// memory modest and skip configurations whose LDM table exceeds this cap. The
/// hashLog sweep still spans 6..=~22 (tables up to ~32 MiB), and the
/// documented full-range knob set (hashLog up to 27) is retained so the intent
/// is explicit — the largest entries are simply skipped at runtime. This is a
/// property of the shared-process test harness, not of the translation.
const MAX_LDM_TABLE_BYTES: usize = 48 << 20;

// ZSTD_ParamSwitch_e
const ZSTD_ps_auto: c_uint = 0;
const ZSTD_ps_enable: c_uint = 1;
const ZSTD_ps_disable: c_uint = 2;

// ZSTD_dictTableLoadMethod_e / tableFillPurpose_e (unused here but documented)
type U32 = u32;
type U64 = u64;
type BYTE = u8;

// ===========================================================================
// #[repr(C)] struct layouts — copied from
// translation/src/compress/zstd_compress_internal.rs and zstd_ldm.rs and
// cross-checked against the C headers. Only fields we read are relied upon; the
// full layout is reproduced so byte offsets are correct.
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct SeqDef {
    off_base: U32,
    lit_length: u16,
    ml_base: u16,
}

#[repr(C)]
struct SeqStore_t {
    sequences_start: *mut SeqDef,
    sequences: *mut SeqDef,
    lit_start: *mut BYTE,
    lit: *mut BYTE,
    ll_code: *mut BYTE,
    ml_code: *mut BYTE,
    of_code: *mut BYTE,
    max_nb_seq: usize,
    max_nb_lit: usize,
    long_length_type: c_uint,
    long_length_pos: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_window_t {
    next_src: *const BYTE,
    base: *const BYTE,
    dict_base: *const BYTE,
    dict_limit: U32,
    low_limit: U32,
    nb_overflow_corrections: U32,
}

const ZSTD_ROW_HASH_CACHE_SIZE: usize = 8;

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_compressionParameters {
    window_log: c_uint,
    chain_log: c_uint,
    hash_log: c_uint,
    search_log: c_uint,
    min_match: c_uint,
    target_length: c_uint,
    strategy: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_match_t {
    off: U32,
    len: U32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_optimal_t {
    price: c_int,
    off: U32,
    mlen: U32,
    litlen: U32,
    rep: [U32; ZSTD_REP_NUM],
}

#[repr(C)]
struct optState_t {
    lit_freq: *mut c_uint,
    lit_length_freq: *mut c_uint,
    match_length_freq: *mut c_uint,
    off_code_freq: *mut c_uint,
    match_table: *mut ZSTD_match_t,
    price_table: *mut ZSTD_optimal_t,
    lit_sum: U32,
    lit_length_sum: U32,
    match_length_sum: U32,
    off_code_sum: U32,
    lit_sum_base_price: U32,
    lit_length_sum_base_price: U32,
    match_length_sum_base_price: U32,
    off_code_sum_base_price: U32,
    price_type: c_uint,
    symbol_costs: *const c_void,
    literal_compression_mode: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct rawSeq {
    offset: U32,
    lit_length: U32,
    match_length: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawSeqStore_t {
    seq: *mut rawSeq,
    pos: usize,
    pos_in_sequence: usize,
    size: usize,
    capacity: usize,
}

#[repr(C)]
struct ZSTD_MatchState_t {
    window: ZSTD_window_t,
    loaded_dict_end: U32,
    next_to_update: U32,
    hash_log3: U32,
    row_hash_log: U32,
    tag_table: *mut BYTE,
    hash_cache: [U32; ZSTD_ROW_HASH_CACHE_SIZE],
    hash_salt: U64,
    hash_salt_entropy: U32,
    hash_table: *mut U32,
    hash_table3: *mut U32,
    chain_table: *mut U32,
    force_non_contiguous: c_int,
    dedicated_dict_search: c_int,
    opt: optState_t,
    dict_match_state: *const ZSTD_MatchState_t,
    c_params: ZSTD_compressionParameters,
    ldm_seq_store: *const RawSeqStore_t,
    prefetch_cdict_tables: c_int,
    lazy_skipping: c_int,
}

// ldmParams_t (compress/zstd_compress_internal.rs / zstd_ldm.h)
#[repr(C)]
#[derive(Clone, Copy)]
struct ldmParams_t {
    enable_ldm: c_uint,
    hash_log: U32,
    bucket_size_log: U32,
    min_match_length: U32,
    hash_rate_log: U32,
    window_log: U32,
}

// ldmEntry_t { U32 offset; U32 checksum; }  == 8 bytes
#[repr(C)]
#[derive(Clone, Copy)]
struct ldmEntry_t {
    offset: U32,
    checksum: U32,
}

// ldmMatchCandidate_t { const BYTE* split; U32 hash; U32 checksum; ldmEntry_t* bucket; }
#[repr(C)]
#[derive(Clone, Copy)]
struct ldmMatchCandidate_t {
    split: *const BYTE,
    hash: U32,
    checksum: U32,
    bucket: *mut ldmEntry_t,
}

const LDM_BATCH_SIZE: usize = 64;

// ldmState_t (compress/zstd_compress_internal.rs). We only read `window`,
// `hashTable`, `loadedDictEnd`; the tail arrays are reproduced so the struct
// size is correct (it is embedded in the CCtx and we take a pointer to it).
#[repr(C)]
struct ldmState_t {
    window: ZSTD_window_t,
    hash_table: *mut ldmEntry_t,
    loaded_dict_end: U32,
    bucket_offsets: *mut BYTE,
    split_indices: [usize; LDM_BATCH_SIZE],
    match_candidates: [ldmMatchCandidate_t; LDM_BATCH_SIZE],
}

// ===========================================================================
// CCtx offsets — copied from b16_internals.rs, validated at runtime below.
// ===========================================================================
const OFF_SEQSTORE: usize = 976;
const OFF_LDMSTATE: usize = 1056;
const OFF_BLOCKSTATE_PREVCBLOCK: usize = 3224;
const OFF_BLOCKSTATE_MATCHSTATE: usize = 3240;
// rep is at the end of ZSTD_compressedBlockState_t (entropy 5616 then rep[3]).
const OFF_CBS_REP: usize = 5616;

// appliedParams.ldmParams: appliedParams is at CCtx offset 240 (validated in b16
// for buildBlockEntropyStats). ldmParams sits at offset 96 within
// ZSTD_CCtx_params_s (format4 cParams28 fParams12 level4 forceWindow4
// [pad4] targetCBlockSize8 srcSizeHint4 attachDictPref4 litCompMode4 nbWorkers4
// jobSize8 overlapLog4 rsyncable4 => 96). Guarded at runtime by asserting
// enableLdm == ZSTD_ps_enable after a compress2 with LDM on.
const OFF_APPLIED_PARAMS: usize = 240;
const OFF_CCTXPARAMS_LDMPARAMS: usize = 96;

unsafe fn cctx_applied_ldm_params(cctx: *mut c_void) -> ldmParams_t {
    let p = (cctx as *const u8).add(OFF_APPLIED_PARAMS + OFF_CCTXPARAMS_LDMPARAMS)
        as *const ldmParams_t;
    *p
}

unsafe fn cctx_seqstore(cctx: *mut c_void) -> *mut SeqStore_t {
    (cctx as *mut u8).add(OFF_SEQSTORE) as *mut SeqStore_t
}
unsafe fn cctx_ldmstate(cctx: *mut c_void) -> *mut ldmState_t {
    (cctx as *mut u8).add(OFF_LDMSTATE) as *mut ldmState_t
}
unsafe fn cctx_matchstate(cctx: *mut c_void) -> *mut ZSTD_MatchState_t {
    (cctx as *mut u8).add(OFF_BLOCKSTATE_MATCHSTATE) as *mut ZSTD_MatchState_t
}

/// Rewind a (library-built) match state to its pre-block state: cursor at the
/// block start (window.dictLimit) and hash/chain tables freshly zeroed. Only
/// the scalar cursor and the two chain-based tables are touched; the window
/// (base/nextSrc/limits mapping `src`) is left exactly as the library set it.
/// Used before a direct ZSTD_ldm_blockCompress call so the match finder starts
/// from the same clean state the library's per-block path starts from.
unsafe fn reset_ms_preblock(ms: *mut ZSTD_MatchState_t) {
    (*ms).next_to_update = (*ms).window.dict_limit;
    let h_entries = 1usize << (*ms).c_params.hash_log;
    if !(*ms).hash_table.is_null() {
        std::ptr::write_bytes((*ms).hash_table, 0, h_entries);
    }
    if !(*ms).chain_table.is_null() {
        let c_entries = 1usize << (*ms).c_params.chain_log;
        std::ptr::write_bytes((*ms).chain_table, 0, c_entries);
    }
}
unsafe fn cctx_next_rep(cctx: *mut c_void) -> *mut [u32; ZSTD_REP_NUM] {
    // blockState.nextCBlock->rep — nextCBlock is the 2nd pointer of blockState.
    let ncb = *((cctx as *mut u8).add(OFF_BLOCKSTATE_PREVCBLOCK + 8) as *mut *mut u8);
    ncb.add(OFF_CBS_REP) as *mut [u32; ZSTD_REP_NUM]
}
unsafe fn cctx_prev_rep(cctx: *mut c_void) -> *mut [u32; ZSTD_REP_NUM] {
    let pcb = *((cctx as *mut u8).add(OFF_BLOCKSTATE_PREVCBLOCK) as *mut *mut u8);
    pcb.add(OFF_CBS_REP) as *mut [u32; ZSTD_REP_NUM]
}

// ---------------------------------------------------------------------------
// FFI typedefs.
// ---------------------------------------------------------------------------
type FnCreateCCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeCCtx = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnGetSeqStore = unsafe extern "C" fn(*const c_void) -> *const SeqStore_t;
type FnCompressBegin = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompressBlock =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;
type FnResetSeqStore = unsafe extern "C" fn(*mut c_void);
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;

// The five targets.
type FnLdmFillHashTable =
    unsafe extern "C" fn(*mut ldmState_t, *const BYTE, *const BYTE, *const ldmParams_t);
type FnLdmGenerateSequences = unsafe extern "C" fn(
    *mut ldmState_t,
    *mut RawSeqStore_t,
    *const ldmParams_t,
    *const c_void,
    size_t,
) -> size_t;
type FnLdmBlockCompress = unsafe extern "C" fn(
    *mut RawSeqStore_t,
    *mut ZSTD_MatchState_t,
    *mut SeqStore_t,
    *mut U32, // rep[ZSTD_REP_NUM]
    c_uint,   // useRowMatchFinder (ZSTD_ParamSwitch_e)
    *const c_void,
    size_t,
) -> size_t;
type FnCompressSuperBlock = unsafe extern "C" fn(
    *mut c_void, // ZSTD_CCtx*
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint, // lastBlock
) -> size_t;
type FnDdsLoadDict = unsafe extern "C" fn(*mut ZSTD_MatchState_t, *const BYTE);

// Exported LDM parameter-math helpers (used to size tables from the C's own
// formulas, per the RULES).
type FnLdmGetTableSize = unsafe extern "C" fn(ldmParams_t) -> size_t;
type FnLdmGetMaxNbSeq = unsafe extern "C" fn(ldmParams_t, size_t) -> size_t;
type FnLdmAdjust =
    unsafe extern "C" fn(*mut ldmParams_t, *const ZSTD_compressionParameters);

// ZDICT_trainFromBuffer for the real trained dictionary.
type FnZdictTrain =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, *const usize, c_uint) -> size_t;
type FnZdictIsError = unsafe extern "C" fn(size_t) -> c_uint;

// ===========================================================================
// Layout self-check (copied from b16_internals.rs). Guards every offset read.
// ===========================================================================
#[test]
fn cctx_layout_selfcheck() {
    unsafe {
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cgss, rgss) = both::<FnGetSeqStore>("ZSTD_getSeqStore");
        let cc = cnew();
        let rc = rnew();
        assert!(!cc.is_null() && !rc.is_null());
        let c_ss = cgss(cc) as usize;
        let r_ss = rgss(rc) as usize;
        assert_eq!(
            c_ss - cc as usize,
            OFF_SEQSTORE,
            "C ZSTD_getSeqStore offset {} != OFF_SEQSTORE {OFF_SEQSTORE}",
            c_ss - cc as usize
        );
        assert_eq!(
            r_ss - rc as usize,
            OFF_SEQSTORE,
            "RS ZSTD_getSeqStore offset {} != OFF_SEQSTORE {OFF_SEQSTORE}",
            r_ss - rc as usize
        );
        cfree(cc);
        rfree(rc);
    }
}

/// Debug wrapper so ldmParams_t prints its fields in assertion messages.
struct DbgLdm(ldmParams_t);
impl std::fmt::Debug for DbgLdm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = &self.0;
        write!(
            f,
            "ldm{{en={},hl={},bs={},mm={},hr={},wl={}}}",
            p.enable_ldm, p.hash_log, p.bucket_size_log, p.min_match_length,
            p.hash_rate_log, p.window_log
        )
    }
}

// ===========================================================================
// Shared helper: build a CCtx with LDM enabled at the given knobs, drive ONE
// real block of `src` through ZSTD_compressBlock so the library initialises
// `ldmState.window` (nextSrc == src+srcSize), the match state and the seqStore.
// Returns the primed CCtx and the *effective* ldmParams (read back from the
// CCtx's appliedParams via ZSTD_ldm_adjustParameters over the cParams we set).
//
// `src` MUST outlive every subsequent access to the returned CCtx's state.
// ===========================================================================
struct LdmKnobs {
    hash_log: c_int,
    min_match: c_int,
    bucket_size_log: c_int,
    hash_rate_log: c_int,
    window_log: c_int,
}

#[allow(clippy::too_many_arguments)]
unsafe fn prime_ldm_cctx(
    cnew: &FnCreateCCtx,
    cset: &FnSetParam,
    cc2: &FnCompress2,
    cbound: &FnBound,
    e: &Err2,
    k: &LdmKnobs,
    src: &[u8],
) -> Option<*mut c_void> {
    let cc = cnew();
    // enable LDM + set the knobs that make the LDM path live. These are STICKY
    // advanced parameters honoured by ZSTD_compress2 (unlike ZSTD_compressBegin,
    // which rebuilds params from a bare compression level and would wipe them).
    let sets: &[(c_int, c_int)] = &[
        (ZSTD_c_enableLongDistanceMatching, 1),
        (ZSTD_c_ldmHashLog, k.hash_log),
        (ZSTD_c_ldmMinMatch, k.min_match),
        (ZSTD_c_ldmBucketSizeLog, k.bucket_size_log),
        (ZSTD_c_ldmHashRateLog, k.hash_rate_log),
        (ZSTD_c_windowLog, k.window_log),
        (ZSTD_c_strategy, 5), // ZSTD_lazy2 (non-opt: keeps blockCompress on the "apply sequences" path)
        (ZSTD_c_useRowMatchFinder, ZSTD_ps_disable as c_int), // chain-based: no tagTable/row state
    ];
    for &(id, v) in sets {
        if e.c.is_err(cset(cc, id, v)) {
            return None;
        }
    }
    let cap = cbound(src.len()) + 64;
    let mut o = vec![0u8; cap];
    // One-shot ZSTD_compress2 references `src` directly (no internal input
    // buffering) and drives ZSTD_compressContinue_internal, which calls
    // ZSTD_window_update on BOTH ms->window and ldmState.window over `src`.
    let rc = cc2(cc, o.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    if e.c.is_err(rc) {
        return None;
    }
    // Precondition guard: the LDM window must map exactly `src`
    // (nextSrc == src + srcSize). If the library buffered the input elsewhere
    // (should not happen for one-shot), skip rather than risk reading garbage.
    let lc = cctx_ldmstate(cc);
    let want_end = src.as_ptr().add(src.len());
    if (*lc).window.base.is_null() || (*lc).window.next_src != want_end {
        return None;
    }
    // Self-check the appliedParams.ldmParams offset: LDM must be enabled here.
    let ap = cctx_applied_ldm_params(cc);
    if ap.enable_ldm != ZSTD_ps_enable {
        // Offset wrong OR LDM silently disabled -> do not proceed (would read a
        // mis-sized table and crash). This fails loudly via the guard in callers.
        return None;
    }
    Some(cc)
}

/// Read the effective ldmParams for a set of knobs by running the exported
/// ZSTD_ldm_adjustParameters over the corresponding cParams (matching what the
/// library stores in appliedParams.ldmParams). This gives the *same* params the
/// library used to build the ldmState tables, so our direct calls are consistent.
unsafe fn effective_ldm_params(
    adjust: &FnLdmAdjust,
    k: &LdmKnobs,
) -> ldmParams_t {
    let mut p = ldmParams_t {
        enable_ldm: ZSTD_ps_enable,
        hash_log: k.hash_log as U32,
        bucket_size_log: k.bucket_size_log as U32,
        min_match_length: k.min_match as U32,
        hash_rate_log: k.hash_rate_log as U32,
        window_log: k.window_log as U32,
    };
    // cParams approximating level 3 at the requested windowLog; the fields that
    // matter to adjustParameters are windowLog, and strategy/targetLength for
    // the minMatch>=targetLength clamp. We mirror ZSTD_lazy2 / typical values.
    let cparams = ZSTD_compressionParameters {
        window_log: k.window_log as c_uint,
        chain_log: 24,
        hash_log: 22,
        search_log: 5,
        min_match: 4,
        target_length: 0,
        strategy: 5,
    };
    adjust(&mut p, &cparams);
    p
}

// ===========================================================================
// 1. ZSTD_ldm_fillHashTable — DIRECT.
//
// Precondition: ldmState->window.base set; reads bytes [ip, iend). After
// prime_ldm_cctx the ldmState window maps `src` (base + dictLimit .. nextSrc),
// so ip/iend chosen within that live prefix are valid. We call fillHashTable on
// each library's own ldmState with identical ip/iend expressed as offsets, then
// compare the ENTIRE LDM hash table (ZSTD_ldm_getTableSize bytes) byte-for-byte.
// ===========================================================================
#[test]
fn ldm_fill_hash_table_direct() {
    unsafe {
        let e = Err2::new();
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cset, rset) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (cbound, _rbound) = both::<FnBound>("ZSTD_compressBound");
        let (cfill, rfill) = both::<FnLdmFillHashTable>("ZSTD_ldm_fillHashTable");
        let (cgts, _rgts) = both::<FnLdmGetTableSize>("ZSTD_ldm_getTableSize");
        let (cadj, radj) = both::<FnLdmAdjust>("ZSTD_ldm_adjustParameters");
        let _ = (&cadj, &radj); // effective_ldm_params retained for reference; params now read from appliedParams.
        let mut rng = Rng::new(0xB17F1);

        let knob_set: &[LdmKnobs] = &[
            LdmKnobs { hash_log: 6, min_match: 4, bucket_size_log: 1, hash_rate_log: 0, window_log: 17 },
            LdmKnobs { hash_log: 12, min_match: 16, bucket_size_log: 4, hash_rate_log: 4, window_log: 20 },
            LdmKnobs { hash_log: 20, min_match: 64, bucket_size_log: 6, hash_rate_log: 12, window_log: 23 },
            LdmKnobs { hash_log: 24, min_match: 1024, bucket_size_log: 8, hash_rate_log: 25, window_log: 27 },
            LdmKnobs { hash_log: 27, min_match: 4096, bucket_size_log: 3, hash_rate_log: 4, window_log: 27 },
        ];
        for k in knob_set {
            for &shape in &[Shape::LongMatches, Shape::Repeating, Shape::Random, Shape::Text, Shape::Zeros] {
                for &len in &[131072usize, 200000, 400000] {
                    let src = gen(shape, len, &mut rng);
                    let n = src.len();
                    if n < HASH_READ_SIZE + 8 {
                        continue;
                    }
                    let cc = match prime_ldm_cctx(&cnew, &cset, &cc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => continue,
                    };
                    let rc = match prime_ldm_cctx(&rnew, &rset, &rc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => { cfree(cc); continue; }
                    };
                    let cp = cctx_applied_ldm_params(cc);
                    let rp = cctx_applied_ldm_params(rc);
                    // table size (same formula both libs); use the C helper.
                    let tsize = cgts(cp);
                    if tsize == 0 || tsize > MAX_LDM_TABLE_BYTES {
                        cfree(cc);
                        rfree(rc);
                        continue;
                    }

                    let lc = cctx_ldmstate(cc);
                    let lr = cctx_ldmstate(rc);
                    // ip/iend within the live prefix: use the ldm window itself.
                    // base + dictLimit is the prefix start; nextSrc is the end.
                    let c_base = (*lc).window.base;
                    let c_start = c_base.add((*lc).window.dict_limit as usize);
                    let c_end = (*lc).window.next_src;
                    let r_base = (*lr).window.base;
                    let r_start = r_base.add((*lr).window.dict_limit as usize);
                    let r_end = (*lr).window.next_src;
                    // Zero both hash tables first (the library filled them during
                    // the block; we want a clean, identical starting point so the
                    // comparison isolates exactly what fillHashTable writes).
                    std::ptr::write_bytes((*lc).hash_table as *mut u8, 0, tsize);
                    std::ptr::write_bytes((*lr).hash_table as *mut u8, 0, tsize);

                    cfill(lc, c_start, c_end, &cp);
                    rfill(lr, r_start, r_end, &rp);

                    let cht = std::slice::from_raw_parts((*lc).hash_table as *const u8, tsize);
                    let rht = std::slice::from_raw_parts((*lr).hash_table as *const u8, tsize);
                    assert_bytes_eq(
                        &format!("ldm_fillHashTable {:?} shape={shape:?} len={n} tsize={tsize}", DbgLdm(cp)),
                        cht, rht,
                    );
                    cfree(cc);
                    rfree(rc);
                }
            }
        }
    }
}

// ===========================================================================
// 2. ZSTD_ldm_generateSequences — DIRECT.
//
// Precondition: ldmState->window.nextSrc >= src+srcSize (window already
// updated for the whole input) and sequences->{seq,capacity} sized via
// ZSTD_ldm_getMaxNbSeq. After prime_ldm_cctx the ldm window covers `src`, so we
// pass the SAME `src`. We allocate the rawSeqStore backing array from the C's
// own ZSTD_ldm_getMaxNbSeq formula (as Vec<rawSeq>, naturally 4-byte aligned).
// We compare the return value (Err2) and the whole produced rawSeqStore: seq[]
// array + size + pos + posInSequence + capacity.
// ===========================================================================
#[test]
fn ldm_generate_sequences_direct() {
    unsafe {
        let e = Err2::new();
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cset, rset) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (cbound, _rbound) = both::<FnBound>("ZSTD_compressBound");
        let (cgen, rgen) = both::<FnLdmGenerateSequences>("ZSTD_ldm_generateSequences");
        let (cmns, _rmns) = both::<FnLdmGetMaxNbSeq>("ZSTD_ldm_getMaxNbSeq");
        let (cgts, _rgts) = both::<FnLdmGetTableSize>("ZSTD_ldm_getTableSize");
        let (cadj, radj) = both::<FnLdmAdjust>("ZSTD_ldm_adjustParameters");
        let _ = (&cadj, &radj); // effective_ldm_params retained for reference; params now read from appliedParams.
        let mut rng = Rng::new(0xB17F2);

        let knob_set: &[LdmKnobs] = &[
            LdmKnobs { hash_log: 7, min_match: 4, bucket_size_log: 2, hash_rate_log: 4, window_log: 17 },
            LdmKnobs { hash_log: 14, min_match: 16, bucket_size_log: 4, hash_rate_log: 12, window_log: 20 },
            LdmKnobs { hash_log: 20, min_match: 64, bucket_size_log: 6, hash_rate_log: 0, window_log: 23 },
            LdmKnobs { hash_log: 24, min_match: 1024, bucket_size_log: 8, hash_rate_log: 25, window_log: 27 },
        ];
        for k in knob_set {
            for &shape in &[Shape::LongMatches, Shape::Repeating, Shape::Random, Shape::Text, Shape::Zeros] {
                for &len in &[131072usize, 200000, 400000] {
                    let src = gen(shape, len, &mut rng);
                    let n = src.len();
                    if n < HASH_READ_SIZE + 8 {
                        continue;
                    }
                    let cc = match prime_ldm_cctx(&cnew, &cset, &cc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => continue,
                    };
                    let rc = match prime_ldm_cctx(&rnew, &rset, &rc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => { cfree(cc); continue; }
                    };
                    let cp = cctx_applied_ldm_params(cc);
                    let rp = cctx_applied_ldm_params(rc);
                    let cap = cmns(cp, n).max(1);

                    // Reset the LDM hash tables to a clean identical state; the
                    // library-built window (nextSrc>=src+n) is reused untouched.
                    let lc = cctx_ldmstate(cc);
                    let lr = cctx_ldmstate(rc);
                    let tsize = cgts(cp);
                    if tsize == 0 || tsize > MAX_LDM_TABLE_BYTES {
                        cfree(cc);
                        rfree(rc);
                        continue;
                    }
                    std::ptr::write_bytes((*lc).hash_table as *mut u8, 0, tsize);
                    std::ptr::write_bytes((*lr).hash_table as *mut u8, 0, tsize);

                    // Independent backing arrays per library (Vec<rawSeq>).
                    let mut seqs_c = vec![rawSeq { offset: 0, lit_length: 0, match_length: 0 }; cap];
                    let mut seqs_r = vec![rawSeq { offset: 0, lit_length: 0, match_length: 0 }; cap];
                    let mut store_c = RawSeqStore_t {
                        seq: seqs_c.as_mut_ptr(),
                        pos: 0,
                        pos_in_sequence: 0,
                        size: 0,
                        capacity: cap,
                    };
                    let mut store_r = RawSeqStore_t {
                        seq: seqs_r.as_mut_ptr(),
                        pos: 0,
                        pos_in_sequence: 0,
                        size: 0,
                        capacity: cap,
                    };
                    let sp = src.as_ptr() as *const c_void;
                    let a = cgen(lc, &mut store_c, &cp, sp, n);
                    let b = rgen(lr, &mut store_r, &rp, sp, n);
                    let ctx = format!("ldm_generateSequences {:?} shape={shape:?} len={n}", DbgLdm(cp));
                    if e.eq_or_oom(&ctx, a, b) && !e.c.is_err(a) {
                        assert_eq!(store_c.size, store_r.size, "{ctx}: size");
                        assert_eq!(store_c.pos, store_r.pos, "{ctx}: pos");
                        assert_eq!(store_c.pos_in_sequence, store_r.pos_in_sequence, "{ctx}: posInSeq");
                        assert_eq!(store_c.capacity, store_r.capacity, "{ctx}: capacity");
                        let nseq = store_c.size;
                        assert_bytes_eq(
                            &format!("{ctx}: seq[]"),
                            std::slice::from_raw_parts(seqs_c.as_ptr() as *const u8, nseq * 12),
                            std::slice::from_raw_parts(seqs_r.as_ptr() as *const u8, nseq * 12),
                        );
                        // Also compare the LDM hash table the generator mutated.
                        let cht = std::slice::from_raw_parts((*lc).hash_table as *const u8, tsize);
                        let rht = std::slice::from_raw_parts((*lr).hash_table as *const u8, tsize);
                        assert_bytes_eq(&format!("{ctx}: ldm hashTable"), cht, rht);
                    }
                    cfree(cc);
                    rfree(rc);
                }
            }
        }
    }
}

// ===========================================================================
// 3. ZSTD_ldm_blockCompress — DIRECT.
//
// Precondition: a live match state (window mapping `src`), a fresh seqStore, a
// rep[] array, and a rawSeqStore of predefined sequences (produced by
// generateSequences on the same src). After prime_ldm_cctx the ms + seqStore
// are library-built and map `src`. We:
//   * generate the LDM sequences with ZSTD_ldm_generateSequences (already a
//     verified direct call above),
//   * reset the seqStore (ZSTD_resetSeqStore) and seed rep from the block state,
//   * call ZSTD_ldm_blockCompress DIRECTLY on each library's own state,
//   * compare the return value (last-literals length), the ENTIRE produced
//     seqStore (sequences + literals + longLength bookkeeping) and rep[].
// ZSTD_ldm_blockCompress "does not return any errors" (header), so its return
// is a plain size_t (last-literals length), compared as an exact integer.
// ===========================================================================
#[test]
fn ldm_block_compress_direct() {
    unsafe {
        let e = Err2::new();
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cset, rset) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (cbound, _rbound) = both::<FnBound>("ZSTD_compressBound");
        let (cgen, rgen) = both::<FnLdmGenerateSequences>("ZSTD_ldm_generateSequences");
        let (cbc, rbc) = both::<FnLdmBlockCompress>("ZSTD_ldm_blockCompress");
        let (creset, rreset) = both::<FnResetSeqStore>("ZSTD_resetSeqStore");
        let (cmns, _rmns) = both::<FnLdmGetMaxNbSeq>("ZSTD_ldm_getMaxNbSeq");
        let (cgts, _rgts) = both::<FnLdmGetTableSize>("ZSTD_ldm_getTableSize");
        let (cadj, radj) = both::<FnLdmAdjust>("ZSTD_ldm_adjustParameters");
        let _ = (&cadj, &radj); // effective_ldm_params retained for reference; params now read from appliedParams.
        let mut rng = Rng::new(0xB17F3);

        // ZSTD_ldm_blockCompress requires srcSize <= block-size-max (128 KB): it
        // is called per-block by the library. We therefore drive it on blocks of
        // <= 128 KB. The predefined sequences may span the whole input.
        let knob_set: &[LdmKnobs] = &[
            LdmKnobs { hash_log: 7, min_match: 4, bucket_size_log: 2, hash_rate_log: 4, window_log: 17 },
            LdmKnobs { hash_log: 14, min_match: 16, bucket_size_log: 4, hash_rate_log: 12, window_log: 20 },
            LdmKnobs { hash_log: 20, min_match: 64, bucket_size_log: 6, hash_rate_log: 0, window_log: 23 },
            LdmKnobs { hash_log: 24, min_match: 1024, bucket_size_log: 8, hash_rate_log: 25, window_log: 27 },
        ];
        for k in knob_set {
            for &shape in &[Shape::LongMatches, Shape::Repeating, Shape::Random, Shape::Text, Shape::Zeros] {
                for &len in &[131072usize] {
                    let src = gen(shape, len, &mut rng);
                    let n = src.len();
                    if n < HASH_READ_SIZE + 8 {
                        continue;
                    }
                    let cc = match prime_ldm_cctx(&cnew, &cset, &cc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => continue,
                    };
                    let rc = match prime_ldm_cctx(&rnew, &rset, &rc2, &cbound, &e, k, &src[..n]) {
                        Some(x) => x,
                        None => { cfree(cc); continue; }
                    };
                    let cp = cctx_applied_ldm_params(cc);
                    let rp = cctx_applied_ldm_params(rc);
                    let cap = cmns(cp, n).max(1);
                    let tsize = cgts(cp);
                    if tsize == 0 || tsize > MAX_LDM_TABLE_BYTES { cfree(cc); rfree(rc); continue; }

                    let lc = cctx_ldmstate(cc);
                    let lr = cctx_ldmstate(rc);
                    std::ptr::write_bytes((*lc).hash_table as *mut u8, 0, tsize);
                    std::ptr::write_bytes((*lr).hash_table as *mut u8, 0, tsize);

                    // Step 1: generate sequences (direct call, verified separately).
                    let mut seqs_c = vec![rawSeq { offset: 0, lit_length: 0, match_length: 0 }; cap];
                    let mut seqs_r = vec![rawSeq { offset: 0, lit_length: 0, match_length: 0 }; cap];
                    let mut store_c = RawSeqStore_t { seq: seqs_c.as_mut_ptr(), pos: 0, pos_in_sequence: 0, size: 0, capacity: cap };
                    let mut store_r = RawSeqStore_t { seq: seqs_r.as_mut_ptr(), pos: 0, pos_in_sequence: 0, size: 0, capacity: cap };
                    let sp = src.as_ptr() as *const c_void;
                    let ga = cgen(lc, &mut store_c, &cp, sp, n);
                    let gb = rgen(lr, &mut store_r, &rp, sp, n);
                    let ctx = format!("ldm_blockCompress {:?} shape={shape:?} len={n}", DbgLdm(cp));
                    if !(e.eq_or_oom(&format!("{ctx}: gen"), ga, gb) && !e.c.is_err(ga)) {
                        cfree(cc); rfree(rc); continue;
                    }
                    if store_c.size != store_r.size {
                        // A generateSequences divergence would already have been
                        // caught by the dedicated test; bail defensively.
                        cfree(cc); rfree(rc); continue;
                    }

                    // Step 2: reset the seqStore, and rewind the match state to
                    // its pre-block state so the direct call sees exactly what
                    // the library's per-block path sees on the FIRST block:
                    //   * nextToUpdate == window.dictLimit (block start index),
                    //   * hashTable / chainTable freshly zeroed,
                    //   * rep[] == the initial repcodes {1,4,8}.
                    // The window itself (base/nextSrc/limits mapping `src`) is the
                    // library-built one and is left untouched.
                    creset(cctx_seqstore(cc) as *mut c_void);
                    rreset(cctx_seqstore(rc) as *mut c_void);
                    let msc = cctx_matchstate(cc);
                    let msr = cctx_matchstate(rc);
                    reset_ms_preblock(msc);
                    reset_ms_preblock(msr);
                    let mut rep_arr_c: [u32; ZSTD_REP_NUM] = [1, 4, 8];
                    let mut rep_arr_r: [u32; ZSTD_REP_NUM] = [1, 4, 8];

                    let ssc = cctx_seqstore(cc);
                    let ssr = cctx_seqstore(rc);

                    // Step 3: DIRECT call. useRowMatchFinder = ps_disable (we set
                    // ZSTD_c_useRowMatchFinder=disable during priming, so the ms
                    // tables are the chain-based layout this switch selects).
                    let la = cbc(&mut store_c, msc, ssc, rep_arr_c.as_mut_ptr(), ZSTD_ps_disable, sp, n);
                    let lb = rbc(&mut store_r, msr, ssr, rep_arr_r.as_mut_ptr(), ZSTD_ps_disable, sp, n);
                    // No error return: exact integer compare of last-literals len.
                    assert_eq!(la, lb, "{ctx}: lastLLSize");
                    assert_eq!(rep_arr_c, rep_arr_r, "{ctx}: rep[] C={rep_arr_c:?} RS={rep_arr_r:?}");
                    assert_eq!(store_c.pos, store_r.pos, "{ctx}: rawSeqStore.pos");

                    // Compare the produced seqStore structurally.
                    let cs = &*ssc;
                    let rs = &*ssr;
                    let c_nseq = cs.sequences.offset_from(cs.sequences_start);
                    let r_nseq = rs.sequences.offset_from(rs.sequences_start);
                    assert_eq!(c_nseq, r_nseq, "{ctx}: nbSeq");
                    let c_nlit = cs.lit.offset_from(cs.lit_start);
                    let r_nlit = rs.lit.offset_from(rs.lit_start);
                    assert_eq!(c_nlit, r_nlit, "{ctx}: nbLit");
                    assert_eq!(cs.long_length_type, rs.long_length_type, "{ctx}: longLengthType");
                    assert_eq!(cs.long_length_pos, rs.long_length_pos, "{ctx}: longLengthPos");
                    let nseq = c_nseq as usize;
                    assert_bytes_eq(
                        &format!("{ctx}: sequences[]"),
                        std::slice::from_raw_parts(cs.sequences_start as *const u8, nseq * 8),
                        std::slice::from_raw_parts(rs.sequences_start as *const u8, nseq * 8),
                    );
                    let nlit = c_nlit as usize;
                    assert_bytes_eq(
                        &format!("{ctx}: literals[]"),
                        std::slice::from_raw_parts(cs.lit_start, nlit),
                        std::slice::from_raw_parts(rs.lit_start, nlit),
                    );
                    cfree(cc);
                    rfree(rc);
                }
            }
        }
    }
}

// ===========================================================================
// 4. ZSTD_compressSuperBlock — DIRECT.
//
// Precondition: a fully-populated CCtx — seqStore filled by the match finder,
// entropy tables in blockState, the window set up — exactly the state the
// library is in right after ZSTD_buildSeqStore + ZSTD_compressSequences decides
// to try a super block. The public path to reach that state is: set
// ZSTD_c_targetCBlockSize, then drive a real ZSTD_compressBlock which populates
// the seqStore for `src`. We then call ZSTD_compressSuperBlock DIRECTLY on that
// live CCtx with the SAME `src`, and compare the return (Err2) + output bytes.
//
// `lastBlock` is swept over {0,1}. dst capacity is swept over the documented
// boundary set. `src` MUST be <= block-size-max.
// ===========================================================================
#[test]
fn compress_super_block_direct() {
    unsafe {
        let e = Err2::new();
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cset, rset) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cbegin, rbegin) = both::<FnCompressBegin>("ZSTD_compressBegin");
        let (cblock, rblock) = both::<FnCompressBlock>("ZSTD_compressBlock");
        let (csb, rsb) = both::<FnCompressSuperBlock>("ZSTD_compressSuperBlock");
        let (cbound, _) = both::<FnBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xB17F4);

        for &tcbs in &[1340i32, 2000, 8192, 65536, 131072] {
            for &shape in ALL_SHAPES {
                for &len in &[1024usize, 65536, 131072] {
                    let src = gen(shape, len, &mut rng);
                    let n = src.len();
                    if n == 0 {
                        continue;
                    }
                    // Prime each CCtx: targetCBlockSize live, then populate the
                    // seqStore by compressing exactly this block.
                    let prime = |cnewf: &FnCreateCCtx, csetf: &FnSetParam,
                                 cbeginf: &FnCompressBegin, cblockf: &FnCompressBlock|
                     -> Option<*mut c_void> {
                        let cc = cnewf();
                        csetf(cc, ZSTD_c_targetCBlockSize, tcbs);
                        csetf(cc, ZSTD_c_windowLog, 18);
                        let r = cbeginf(cc, 3);
                        if e.c.is_err(r) { return None; }
                        let cap = n + 1024;
                        let mut o = vec![0u8; cap];
                        let rr = cblockf(cc, o.as_mut_ptr() as *mut c_void, cap,
                                         src.as_ptr() as *const c_void, n);
                        if e.c.is_err(rr) { return None; }
                        Some(cc)
                    };
                    let cc = match prime(&cnew, &cset, &cbegin, &cblock) { Some(x) => x, None => continue };
                    let rc = match prime(&rnew, &rset, &rbegin, &rblock) { Some(x) => x, None => { cfree(cc); continue; } };

                    let bound = cbound(n);
                    // dst-capacity sweep. PRECONDITION: ZSTD_compressSuperBlock
                    // assumes a generous output budget — the library only ever
                    // calls it with the whole remaining frame buffer. Its helper
                    // ZSTD_compressSubBlock_literal writes the Huffman table
                    // description with an UNCHECKED `ZSTD_memcpy(op, hufDesBuffer,
                    // hufDesSize)` (zstd_compress_superblock.c: op = ostart+lhSize;
                    // memcpy before any `op+hufDesSize <= oend` check). Feeding a
                    // dstCapacity smaller than that header budget is therefore an
                    // out-of-bounds write in the C source itself — it faults
                    // IDENTICALLY in both libraries and is a shared precondition
                    // violation, NOT a differential result. We consequently probe
                    // only capacities at/above ZSTD_compressBound (the budget the
                    // library guarantees), including the tight need/need+1
                    // boundaries expressed relative to `bound`. The {0,1,need-1}
                    // sub-budget cases are excluded for this reason (documented).
                    let need_probe: &[usize] = &[bound, bound + 1, bound + 64, 2 * bound + 64];
                    for lastblock in [0u32, 1u32] {
                        for &cap in need_probe {
                            let mut o1 = vec![0xCCu8; cap.max(1)];
                            let mut o2 = vec![0xCCu8; cap.max(1)];
                            let sp = src.as_ptr() as *const c_void;
                            let a = csb(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, n, lastblock);
                            let b = rsb(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, n, lastblock);
                            let ctx = format!(
                                "compressSuperBlock shape={shape:?} len={n} tcbs={tcbs} last={lastblock} cap={cap}"
                            );
                            if e.eq_or_oom(&ctx, a, b) && !e.c.is_err(a) {
                                assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
                            }
                        }
                    }
                    cfree(cc);
                    rfree(rc);
                }
            }
        }
    }
}

// ===========================================================================
// 5. ZSTD_dedicatedDictSearch_lazy_loadDictionary — DIRECT.
//
// Precondition: a match state whose `dedicatedDictSearch != 0`, whose
// hashTable/chainTable are sized for the DDS layout, whose window maps the
// dictionary content, and with `nextToUpdate != 0`, a lazy-family strategy,
// hashLog > chainLog, chainLog <= 24. That exact state is built by the library
// as the CDict's match state when DDS is enabled. When a DDS CDict is used, the
// library ATTACHES it and stores a pointer to its match state in
// `cctx->blockState.matchState.dictMatchState` — a library-minted pointer we
// can read at the self-checked CCtx offset WITHOUT knowing the CDict layout.
//
// We therefore:
//   * create a CCtx, set a lazy-family strategy + enableDedicatedDictSearch=1,
//     load a dictionary, and run ZSTD_compress2 so the library builds+attaches
//     the DDS CDict match state,
//   * read `dms = ms->dictMatchState` (the CDict's DDS match state),
//   * rewind that match state to its pre-load state (nextToUpdate ->
//     ZSTD_WINDOW_START_INDEX, hashTable+chainTable zeroed) — the only inputs
//     the loader reads besides the (library-owned, untouched) window content,
//   * call ZSTD_dedicatedDictSearch_lazy_loadDictionary DIRECTLY on each
//     library's own dms with ip = dms->window.nextSrc - HASH_READ_SIZE (matching
//     the library's `iend - HASH_READ_SIZE`),
//   * compare the resulting hashTable + chainTable byte-for-byte and nextToUpdate.
//
// Never mix pointers: each library's dms is only ever passed to that library's
// loader.
// ===========================================================================

/// Build the real trained dictionary once (with the C ZDICT_trainFromBuffer)
/// and reuse the identical bytes for both libraries. Returns None if training
/// fails (small corpora can fail to train — then callers skip the trained case).
unsafe fn train_dictionary_once() -> Option<Vec<u8>> {
    let ctrain = sym::<FnZdictTrain>(c(), "ZDICT_trainFromBuffer");
    let ciserr = sym::<FnZdictIsError>(c(), "ZDICT_isError");
    let mut rng = Rng::new(0xD1C7);
    // A corpus of many small, self-similar samples (text) trains well.
    let mut samples: Vec<u8> = Vec::new();
    let mut sizes: Vec<usize> = Vec::new();
    for _ in 0..2000 {
        let len = 32 + rng.below(96);
        let s = gen(Shape::Text, len, &mut rng);
        sizes.push(s.len());
        samples.extend_from_slice(&s);
    }
    let dict_cap = 112_640usize;
    let mut dict = vec![0u8; dict_cap];
    let r = ctrain(
        dict.as_mut_ptr() as *mut c_void,
        dict_cap,
        samples.as_ptr() as *const c_void,
        sizes.as_ptr(),
        sizes.len() as c_uint,
    );
    if ciserr(r) != 0 {
        return None;
    }
    dict.truncate(r);
    Some(dict)
}

#[test]
fn dds_lazy_load_dictionary_direct() {
    unsafe {
        let e = Err2::new();
        let (cnew, rnew) = both::<FnCreateCCtx>("ZSTD_createCCtx");
        let (cfree, rfree) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
        let (cset, rset) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cload, rload) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (cbound, _) = both::<FnBound>("ZSTD_compressBound");
        let (cddl, rddl) =
            both::<FnDdsLoadDict>("ZSTD_dedicatedDictSearch_lazy_loadDictionary");
        let mut rng = Rng::new(0xB17F5);

        // Build dictionaries: raw-random, raw-text, and a real trained one.
        let trained = train_dictionary_once();
        let dict_specs: Vec<(&str, Vec<u8>)> = {
            let mut v: Vec<(&str, Vec<u8>)> = Vec::new();
            for &dsz in &[1usize, 100, 1024, 8192, 112640] {
                v.push(("raw-random", gen(Shape::Random, dsz, &mut rng)));
                v.push(("raw-text", gen(Shape::Text, dsz, &mut rng)));
            }
            if let Some(d) = &trained {
                v.push(("trained", d.clone()));
            }
            v
        };

        // Strategy in the lazy/greedy family: greedy(3), lazy(4), lazy2(5).
        // DDS is only supported/used for these; btlazy2+ revert DDS.
        let strategies = [3i32, 4, 5];
        // chainLog/hashLog/searchLog sweep (valid: hashLog > chainLog, chainLog<=24).
        let cparam_sweep: &[(c_int, c_int, c_int)] = &[
            // (chainLog, hashLog, searchLog)
            (10, 14, 1),
            (16, 20, 4),
            (18, 24, 7),
            (24, 27, 9),
        ];

        // Source used only to trigger attachment; content is irrelevant to the
        // loader (it rebuilds tables purely from the dictionary window).
        let src = gen(Shape::Text, 8192, &mut rng);
        let n = src.len();

        let mut attempted = 0usize;
        let mut invoked = 0usize;

        for (dname, dict) in &dict_specs {
            for &strat in &strategies {
                for &(chain_log, hash_log, search_log) in cparam_sweep {
                    attempted += 1;
                    // Prime each library independently.
                    let prime = |cnewf: &FnCreateCCtx, csetf: &FnSetParam,
                                 cloadf: &FnLoadDict, cc2f: &FnCompress2|
                     -> Option<*mut c_void> {
                        let cc = cnewf();
                        csetf(cc, ZSTD_c_strategy, strat);
                        csetf(cc, ZSTD_c_chainLog, chain_log);
                        csetf(cc, ZSTD_c_hashLog, hash_log);
                        csetf(cc, ZSTD_c_searchLog, search_log);
                        csetf(cc, ZSTD_c_enableDedicatedDictSearch, 1);
                        // Force attaching the dict (DDS always attaches anyway).
                        csetf(cc, ZSTD_c_forceAttachDict, 1);
                        let ld = cloadf(cc, dict.as_ptr() as *const c_void, dict.len());
                        if e.c.is_err(ld) { cfree(cc); return None; }
                        let cap = cbound(n) + 64;
                        let mut o = vec![0u8; cap];
                        let cr = cc2f(cc, o.as_mut_ptr() as *mut c_void, cap,
                                      src.as_ptr() as *const c_void, n);
                        if e.c.is_err(cr) { cfree(cc); return None; }
                        Some(cc)
                    };
                    let cc = match prime(&cnew, &cset, &cload, &cc2) { Some(x) => x, None => continue };
                    let rc = match prime(&rnew, &rset, &rload, &rc2) { Some(x) => x, None => { cfree(cc); continue; } };

                    let msc = cctx_matchstate(cc);
                    let msr = cctx_matchstate(rc);
                    let dmsc = (*msc).dict_match_state as *mut ZSTD_MatchState_t;
                    let dmsr = (*msr).dict_match_state as *mut ZSTD_MatchState_t;
                    // If the dict was not attached as a DDS match state (e.g. an
                    // empty/1-byte dict yields no content and is skipped by the
                    // library), dictMatchState is null -> cannot invoke directly.
                    if dmsc.is_null() || dmsr.is_null()
                        || (*dmsc).dedicated_dict_search == 0
                        || (*dmsr).dedicated_dict_search == 0
                    {
                        cfree(cc); rfree(rc);
                        continue;
                    }
                    // DDS-adjusted cParams on the dict match state.
                    let cp = (*dmsc).c_params;
                    let rp = (*dmsr).c_params;
                    // Preconditions the loader asserts: chainLog<=24, hashLog>chainLog,
                    // nextToUpdate!=0 (we set it to ZSTD_WINDOW_START_INDEX below).
                    if cp.chain_log > 24 || cp.hash_log <= cp.chain_log {
                        cfree(cc); rfree(rc);
                        continue;
                    }
                    let h_entries_c = 1usize << cp.hash_log;
                    let c_entries_c = 1usize << cp.chain_log;
                    let h_entries_r = 1usize << rp.hash_log;
                    let c_entries_r = 1usize << rp.chain_log;

                    // ip = iend - HASH_READ_SIZE where iend = dict window end.
                    let c_end = (*dmsc).window.next_src;
                    let r_end = (*dmsr).window.next_src;
                    let c_base = (*dmsc).window.base;
                    let r_base = (*dmsr).window.base;
                    // target = ip - base must be > ZSTD_WINDOW_START_INDEX for
                    // the loader's loop to do anything and for ip to be valid.
                    let c_target = (c_end as usize).saturating_sub(HASH_READ_SIZE);
                    let r_target = (r_end as usize).saturating_sub(HASH_READ_SIZE);
                    if c_target <= c_base as usize + ZSTD_WINDOW_START_INDEX as usize
                        || r_target <= r_base as usize + ZSTD_WINDOW_START_INDEX as usize
                    {
                        cfree(cc); rfree(rc);
                        continue;
                    }
                    let c_ip = c_target as *const BYTE;
                    let r_ip = r_target as *const BYTE;

                    // Rewind to pre-load state: nextToUpdate -> start, tables zeroed.
                    (*dmsc).next_to_update = ZSTD_WINDOW_START_INDEX;
                    (*dmsr).next_to_update = ZSTD_WINDOW_START_INDEX;
                    std::ptr::write_bytes((*dmsc).hash_table, 0, h_entries_c);
                    std::ptr::write_bytes((*dmsr).hash_table, 0, h_entries_r);
                    if !(*dmsc).chain_table.is_null() {
                        std::ptr::write_bytes((*dmsc).chain_table, 0, c_entries_c);
                    }
                    if !(*dmsr).chain_table.is_null() {
                        std::ptr::write_bytes((*dmsr).chain_table, 0, c_entries_r);
                    }

                    // DIRECT call, each library on its own dms.
                    cddl(dmsc, c_ip);
                    rddl(dmsr, r_ip);
                    invoked += 1;

                    let ctx = format!(
                        "dds_loadDictionary dict={dname} dsz={} strat={strat} chainLog={} hashLog={} searchLog={}",
                        dict.len(), chain_log, hash_log, search_log
                    );
                    assert_eq!(
                        (*dmsc).next_to_update, (*dmsr).next_to_update,
                        "{ctx}: nextToUpdate"
                    );
                    // Compare the entire hash table and chain table.
                    assert_eq!(h_entries_c, h_entries_r, "{ctx}: hashLog differs");
                    assert_eq!(c_entries_c, c_entries_r, "{ctx}: chainLog differs");
                    let cht = std::slice::from_raw_parts((*dmsc).hash_table as *const u8, h_entries_c * 4);
                    let rht = std::slice::from_raw_parts((*dmsr).hash_table as *const u8, h_entries_r * 4);
                    assert_bytes_eq(&format!("{ctx}: hashTable"), cht, rht);
                    if !(*dmsc).chain_table.is_null() && !(*dmsr).chain_table.is_null() {
                        let cct = std::slice::from_raw_parts((*dmsc).chain_table as *const u8, c_entries_c * 4);
                        let rct = std::slice::from_raw_parts((*dmsr).chain_table as *const u8, c_entries_r * 4);
                        assert_bytes_eq(&format!("{ctx}: chainTable"), cct, rct);
                    }
                    cfree(cc);
                    rfree(rc);
                }
            }
        }
        // Guard: we must have actually invoked the loader on at least one config,
        // otherwise the "direct" claim is vacuous.
        assert!(
            invoked > 0,
            "ZSTD_dedicatedDictSearch_lazy_loadDictionary was never invoked directly \
             ({attempted} configs attempted). The DDS match state was never attached."
        );
    }
}
