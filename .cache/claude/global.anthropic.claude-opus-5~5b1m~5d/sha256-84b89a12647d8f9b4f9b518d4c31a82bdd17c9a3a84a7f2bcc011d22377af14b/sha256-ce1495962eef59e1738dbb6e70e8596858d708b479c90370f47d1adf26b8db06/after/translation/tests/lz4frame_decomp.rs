//! Differential tests for the lz4frame DECOMPRESSION half of the API.
//!
//! Covers CONFIGS.md rows 142..156 and ERRORS.md rows 24..55.
//!
//! Every call goes through a `.so` export via libloading. Reference frames are
//! always produced by the **C** library (or hand-crafted), so both decoders see
//! byte-identical input. Each library creates and frees its OWN `LZ4F_dctx`
//! (and its own `LZ4F_cctx` where one is needed); contexts are never handed
//! across the boundary. Every `LZ4F_decompress` call gets its own 0xCD-filled
//! dst buffer with a guard tail and its own in/out `dstSize`/`srcSize`
//! variables; the hint, both out-params and the produced bytes are compared
//! after every single call.
#![allow(unused_imports, non_snake_case, non_upper_case_globals)]

mod common;
use common::*;
use std::collections::BTreeMap;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct CustomMem {
    alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    calloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
}

type FnCreateDCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateDCtxAdv = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
type FnFreeDCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnResetDCtx = unsafe extern "C" fn(*mut c_void);
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnGetFrameInfo =
    unsafe extern "C" fn(*mut c_void, *mut LZ4F_frameInfo_t, *const c_void, *mut usize) -> usize;
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

type FnCompressFrame =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const LZ4F_preferences_t)
        -> usize;
type FnBound1 = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompressBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnCompressBeginUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnCompressEnd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;
type FnGetBlockSize = unsafe extern "C" fn(c_uint) -> usize;

// ---------------------------------------------------------------------------
// The two decoder APIs
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct Api {
    tag: &'static str,
    create: FnCreateDCtx,
    create_adv: FnCreateDCtxAdv,
    free: FnFreeDCtx,
    reset: FnResetDCtx,
    decompress: FnDecompress,
    decompress_ud: FnDecompressUsingDict,
    get_frame_info: FnGetFrameInfo,
    header_size: FnHeaderSize,
}

macro_rules! pair {
    ($l:expr, $t:ty, $n:expr) => {{
        let (a, b) = $l.sym::<$t>($n);
        (*a, *b)
    }};
}

fn apis() -> (Api, Api) {
    use std::sync::OnceLock;
    static P: OnceLock<(Api, Api)> = OnceLock::new();
    *P.get_or_init(|| unsafe {
        let l = libs();
        {
            let (a, b) = l.sym::<FnDecompress>("LZ4F_decompress");
            assert_ne!(
                *a as usize, *b as usize,
                "harness bug: LZ4F_decompress resolved to the same address in both libraries"
            );
        }
        let (c_cr, r_cr) = pair!(l, FnCreateDCtx, "LZ4F_createDecompressionContext");
        let (c_ca, r_ca) = pair!(l, FnCreateDCtxAdv, "LZ4F_createDecompressionContext_advanced");
        let (c_fr, r_fr) = pair!(l, FnFreeDCtx, "LZ4F_freeDecompressionContext");
        let (c_rs, r_rs) = pair!(l, FnResetDCtx, "LZ4F_resetDecompressionContext");
        let (c_de, r_de) = pair!(l, FnDecompress, "LZ4F_decompress");
        let (c_du, r_du) = pair!(l, FnDecompressUsingDict, "LZ4F_decompress_usingDict");
        let (c_gi, r_gi) = pair!(l, FnGetFrameInfo, "LZ4F_getFrameInfo");
        let (c_hs, r_hs) = pair!(l, FnHeaderSize, "LZ4F_headerSize");
        (
            Api {
                tag: "C",
                create: c_cr,
                create_adv: c_ca,
                free: c_fr,
                reset: c_rs,
                decompress: c_de,
                decompress_ud: c_du,
                get_frame_info: c_gi,
                header_size: c_hs,
            },
            Api {
                tag: "Rust",
                create: r_cr,
                create_adv: r_ca,
                free: r_fr,
                reset: r_rs,
                decompress: r_de,
                decompress_ud: r_du,
                get_frame_info: r_gi,
                header_size: r_hs,
            },
        )
    })
}

// ---------------------------------------------------------------------------
// The C-side encoder used to manufacture reference frames
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct CEnc {
    compress_frame: FnCompressFrame,
    frame_bound: FnBound1,
    bound: FnBound1,
    create_cctx: FnCreateCCtx,
    free_cctx: FnFreeCCtx,
    begin: FnCompressBegin,
    begin_dict: FnCompressBeginUsingDict,
    update: FnCompressUpdate,
    end: FnCompressEnd,
    xxh32: FnXXH32,
    block_size: FnGetBlockSize,
}

fn cenc() -> &'static CEnc {
    use std::sync::OnceLock;
    static P: OnceLock<CEnc> = OnceLock::new();
    P.get_or_init(|| unsafe {
        let l = libs();
        let g = |n: &str| n.to_string();
        let _ = g;
        CEnc {
            compress_frame: *l.c.get::<FnCompressFrame>(b"LZ4F_compressFrame").unwrap(),
            frame_bound: *l.c.get::<FnBound1>(b"LZ4F_compressFrameBound").unwrap(),
            bound: *l.c.get::<FnBound1>(b"LZ4F_compressBound").unwrap(),
            create_cctx: *l.c.get::<FnCreateCCtx>(b"LZ4F_createCompressionContext").unwrap(),
            free_cctx: *l.c.get::<FnFreeCCtx>(b"LZ4F_freeCompressionContext").unwrap(),
            begin: *l.c.get::<FnCompressBegin>(b"LZ4F_compressBegin").unwrap(),
            begin_dict: *l
                .c
                .get::<FnCompressBeginUsingDict>(b"LZ4F_compressBegin_usingDict")
                .unwrap(),
            update: *l.c.get::<FnCompressUpdate>(b"LZ4F_compressUpdate").unwrap(),
            end: *l.c.get::<FnCompressEnd>(b"LZ4F_compressEnd").unwrap(),
            xxh32: *l.c.get::<FnXXH32>(b"LZ4_XXH32").unwrap(),
            block_size: *l.c.get::<FnGetBlockSize>(b"LZ4F_getBlockSize").unwrap(),
        }
    })
}

fn xxh32(b: &[u8]) -> u32 {
    unsafe { (cenc().xxh32)(b.as_ptr() as *const c_void, b.len(), 0) }
}

fn block_size_of(bsid: c_uint) -> usize {
    unsafe { (cenc().block_size)(bsid) }
}

/// One-shot reference frame built by the C library.
fn c_frame(payload: &[u8], prefs: Option<&LZ4F_preferences_t>) -> Vec<u8> {
    unsafe {
        let e = cenc();
        let pp = prefs.map_or(ptr::null(), |p| p as *const LZ4F_preferences_t);
        let bound = (e.frame_bound)(payload.len(), pp);
        assert!(!is_err_range(bound), "compressFrameBound failed {bound:#x}");
        let mut out = vec![0u8; bound + 64];
        let n = (e.compress_frame)(
            out.as_mut_ptr() as *mut c_void,
            bound,
            payload.as_ptr() as *const c_void,
            payload.len(),
            pp,
        );
        assert!(!is_err_range(n), "LZ4F_compressFrame failed {n:#x}");
        out.truncate(n);
        out
    }
}

/// Reference frame built by the C library through the low-level pipeline, with
/// an optional dictionary (`LZ4F_compressBegin_usingDict`).
fn c_frame_dict(
    payload: &[u8],
    dict: Option<&[u8]>,
    prefs: Option<&LZ4F_preferences_t>,
    chunk: usize,
) -> Vec<u8> {
    unsafe {
        let e = cenc();
        let pp = prefs.map_or(ptr::null(), |p| p as *const LZ4F_preferences_t);
        let mut cctx: *mut c_void = ptr::null_mut();
        assert_eq!((e.create_cctx)(&mut cctx, LZ4F_VERSION), 0);
        let cap = 64 + (e.bound)(payload.len().max(chunk), pp) * 4 + payload.len() * 2 + 4096;
        let mut out = vec![0u8; cap];
        let mut off = 0usize;
        let n = match dict {
            Some(d) => (e.begin_dict)(
                cctx,
                out.as_mut_ptr() as *mut c_void,
                cap,
                d.as_ptr() as *const c_void,
                d.len(),
                pp,
            ),
            None => (e.begin)(cctx, out.as_mut_ptr() as *mut c_void, cap, pp),
        };
        assert!(!is_err_range(n), "compressBegin failed {n:#x}");
        off += n;
        let step = if chunk == 0 { payload.len().max(1) } else { chunk };
        let mut i = 0usize;
        while i < payload.len() {
            let k = step.min(payload.len() - i);
            let n = (e.update)(
                cctx,
                out.as_mut_ptr().add(off) as *mut c_void,
                cap - off,
                payload.as_ptr().add(i) as *const c_void,
                k,
                ptr::null(),
            );
            assert!(!is_err_range(n), "compressUpdate failed {n:#x}");
            off += n;
            i += k;
        }
        let n = (e.end)(cctx, out.as_mut_ptr().add(off) as *mut c_void, cap - off, ptr::null());
        assert!(!is_err_range(n), "compressEnd failed {n:#x}");
        off += n;
        assert_eq!((e.free_cctx)(cctx), 0);
        out.truncate(off);
        out
    }
}

// ---------------------------------------------------------------------------
// Hand-crafted frame bytes
// ---------------------------------------------------------------------------

const MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

fn header_checksum(after_magic: &[u8]) -> u8 {
    (xxh32(after_magic) >> 8) as u8
}

/// Build a frame header: magic, FLG, BD, [contentSize], [dictID], HC.
fn craft_header(flg: u8, bd: u8, csize: Option<u64>, dict_id: Option<u32>) -> Vec<u8> {
    let mut v = Vec::from(MAGIC);
    v.push(flg);
    v.push(bd);
    if let Some(c) = csize {
        v.extend_from_slice(&c.to_le_bytes());
    }
    if let Some(d) = dict_id {
        v.extend_from_slice(&d.to_le_bytes());
    }
    let hc = header_checksum(&v[4..]);
    v.push(hc);
    v
}

/// FLG for a plain frame: version 1 plus the requested flags.
fn flg(block_independent: bool, block_crc: bool, content_size: bool, content_crc: bool, dict_id: bool) -> u8 {
    0x40 | ((block_independent as u8) << 5)
        | ((block_crc as u8) << 4)
        | ((content_size as u8) << 3)
        | ((content_crc as u8) << 2)
        | (dict_id as u8)
}

fn bd(bsid: u8) -> u8 {
    bsid << 4
}

/// Append a stored (uncompressed) block, optionally with its block checksum.
fn push_stored_block(f: &mut Vec<u8>, payload: &[u8], with_crc: bool) {
    let h = (payload.len() as u32) | 0x8000_0000;
    f.extend_from_slice(&h.to_le_bytes());
    f.extend_from_slice(payload);
    if with_crc {
        f.extend_from_slice(&xxh32(payload).to_le_bytes());
    }
}

/// A compressed block whose decoded size exceeds `maxBlockSize`, so
/// `LZ4_decompress_safe_usingDict` is guaranteed to fail (-> err(16)).
fn overflowing_block_payload(max_block_size: usize) -> Vec<u8> {
    // token: litLength 1, matchLength 15 (extended); one literal; offset 1;
    // then enough 0xFF continuation bytes to blow past maxBlockSize.
    let ext = max_block_size / 255 + 8;
    let mut v = vec![0x1Fu8, b'A', 0x01, 0x00];
    v.extend(std::iter::repeat(0xFFu8).take(ext));
    v.push(0x00);
    v
}

// ---------------------------------------------------------------------------
// Frame layout parsing (for surgical corruption)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BlockInfo {
    hdr: usize,
    payload: usize,
    len: usize,
    stored: bool,
    crc: Option<usize>,
}

#[derive(Debug, Clone)]
struct Layout {
    hsize: usize,
    blocks: Vec<BlockInfo>,
    #[allow(dead_code)]
    endmark: usize,
    content_crc: Option<usize>,
}

fn parse_frame(f: &[u8]) -> Layout {
    assert!(f.len() >= 7, "frame too short to parse");
    assert_eq!(&f[..4], &MAGIC, "parse_frame: not a plain lz4 frame");
    let flg = f[4];
    let block_crc = (flg >> 4) & 1 == 1;
    let content_crc = (flg >> 2) & 1 == 1;
    let csz = (flg >> 3) & 1 == 1;
    let did = flg & 1 == 1;
    let hsize = 7 + if csz { 8 } else { 0 } + if did { 4 } else { 0 };
    let mut off = hsize;
    let mut blocks = Vec::new();
    loop {
        assert!(off + 4 <= f.len(), "truncated block header while parsing");
        let bh = u32::from_le_bytes([f[off], f[off + 1], f[off + 2], f[off + 3]]);
        if bh == 0 {
            break;
        }
        let len = (bh & 0x7FFF_FFFF) as usize;
        let stored = bh & 0x8000_0000 != 0;
        let payload = off + 4;
        let crc = if block_crc { Some(payload + len) } else { None };
        blocks.push(BlockInfo { hdr: off, payload, len, stored, crc });
        off = payload + len + if block_crc { 4 } else { 0 };
    }
    Layout {
        hsize,
        blocks,
        endmark: off,
        content_crc: if content_crc { Some(off + 4) } else { None },
    }
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

const FILL: u8 = 0xCD;
const GUARD: usize = 64;
const ALL: usize = usize::MAX;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum DstMode {
    /// One big buffer, dst window advances contiguously (the normal pattern).
    Contig,
    /// A freshly allocated buffer for every call (all kept alive).
    Fresh,
}

#[derive(Clone)]
struct Plan {
    /// per-call src window sizes, cycled. `ALL` = everything remaining.
    src_sizes: Vec<usize>,
    /// per-call dst capacities, cycled. `ALL` = everything remaining.
    dst_caps: Vec<usize>,
    dst_null: bool,
    dst_mode: DstMode,
    max_out: usize,
    opts: LZ4F_decompressOptions_t,
    opts_null: bool,
    /// keep going after a frame ends, while input remains
    continue_frames: bool,
    /// (dict ptr as usize, dict len) fed through LZ4F_decompress_usingDict
    dict: Option<(usize, usize)>,
    dict_from_call: usize,
    /// stop the driver after this many LZ4F_decompress calls
    max_calls: usize,
}

impl Default for Plan {
    fn default() -> Plan {
        Plan {
            src_sizes: vec![ALL],
            dst_caps: vec![ALL],
            dst_null: false,
            dst_mode: DstMode::Contig,
            max_out: 0,
            opts: LZ4F_decompressOptions_t::default(),
            opts_null: true,
            continue_frames: false,
            dict: None,
            dict_from_call: 0,
            max_calls: usize::MAX,
        }
    }
}

fn plan(max_out: usize) -> Plan {
    Plan { max_out, ..Default::default() }
}

#[derive(Clone, PartialEq, Eq)]
struct Step {
    hint: usize,
    src: usize,
    dst: usize,
    req_src: usize,
    req_dst: usize,
}

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{{hint={} src={}/{} dst={}/{}}}",
            self.hint as isize, self.src, self.req_src, self.dst, self.req_dst
        )
    }
}

struct Trace {
    steps: Vec<Step>,
    out: Vec<u8>,
    full: Vec<u8>,
    free_ret: usize,
}

impl Trace {
    fn last_hint(&self) -> usize {
        self.steps.last().map(|s| s.hint).unwrap_or(usize::MAX)
    }
}

unsafe fn drive_dctx(api: &Api, dctx: *mut c_void, frame: &[u8], p: &Plan) -> (Vec<Step>, Vec<u8>, Vec<u8>) {
    let mut out: Vec<u8> = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut contig: Vec<u8> = if p.dst_mode == DstMode::Contig && !p.dst_null {
        vec![FILL; p.max_out + GUARD]
    } else {
        Vec::new()
    };
    let mut fresh: Vec<Vec<u8>> = Vec::new();
    let mut src_off = 0usize;
    let mut i = 0usize;
    let mut stall = 0usize;
    let optp: *const LZ4F_decompressOptions_t =
        if p.opts_null { ptr::null() } else { &p.opts as *const _ };
    let hard_cap = 64 * frame.len() + (1 << 16);
    loop {
        assert!(i < hard_cap, "{}: driver failed to terminate after {i} calls", api.tag);
        let want_s = p.src_sizes[i % p.src_sizes.len()];
        let s_avail = frame.len() - src_off;
        let s = if want_s == ALL { s_avail } else { want_s.min(s_avail) };
        let want_d = p.dst_caps[i % p.dst_caps.len()];
        let (dptr, d): (*mut c_void, usize) = if p.dst_null {
            (ptr::null_mut(), 0)
        } else if p.dst_mode == DstMode::Contig {
            let room = p.max_out - out.len();
            let d = if want_d == ALL { room } else { want_d.min(room) };
            (contig.as_mut_ptr().add(out.len()) as *mut c_void, d)
        } else {
            let d = if want_d == ALL { p.max_out } else { want_d };
            fresh.push(vec![FILL; d + GUARD]);
            let b = fresh.last_mut().unwrap();
            (b.as_mut_ptr() as *mut c_void, d)
        };
        let mut ds = d;
        let mut ss = s;
        let sptr = frame.as_ptr().add(src_off) as *const c_void;
        let use_dict = p.dict.is_some() && i >= p.dict_from_call;
        let ret = if use_dict {
            let (dp, dl) = p.dict.unwrap();
            (api.decompress_ud)(
                dctx,
                dptr,
                &mut ds,
                sptr,
                &mut ss,
                dp as *const c_void,
                dl,
                optp,
            )
        } else {
            (api.decompress)(dctx, dptr, &mut ds, sptr, &mut ss, optp)
        };
        steps.push(Step { hint: ret, src: ss, dst: ds, req_src: s, req_dst: d });
        assert!(ss <= s, "{}: step {i}: consumed {ss} > provided {s}", api.tag);
        assert!(ds <= d, "{}: step {i}: produced {ds} > capacity {d}", api.tag);
        if !p.dst_null {
            if p.dst_mode == DstMode::Contig {
                let n = out.len();
                let seg: Vec<u8> = contig[n..n + ds].to_vec();
                out.extend_from_slice(&seg);
            } else {
                let b = fresh.last().unwrap();
                assert!(
                    b[d..].iter().all(|&x| x == FILL),
                    "{}: step {i}: guard tail clobbered",
                    api.tag
                );
                let seg: Vec<u8> = b[..ds].to_vec();
                out.extend_from_slice(&seg);
            }
        }
        src_off += ss;
        if is_err_range(ret) {
            break;
        }
        // no input left and nothing produced: no further progress is possible
        if s == 0 && ds == 0 && ret != 0 {
            break;
        }
        if ret == 0 && (!p.continue_frames || src_off >= frame.len()) {
            break;
        }
        if ss == 0 && ds == 0 {
            stall += 1;
            if stall >= 3 {
                break;
            }
        } else {
            stall = 0;
        }
        i += 1;
        if i >= p.max_calls {
            break;
        }
    }
    if p.dst_mode == DstMode::Contig && !p.dst_null {
        assert!(
            contig[p.max_out..].iter().all(|&x| x == FILL),
            "{}: guard tail clobbered",
            api.tag
        );
    }
    let full = if contig.is_empty() { out.clone() } else { contig };
    (steps, out, full)
}

unsafe fn drive(api: &Api, frame: &[u8], p: &Plan) -> Trace {
    let mut dctx: *mut c_void = ptr::null_mut();
    let cr = (api.create)(&mut dctx, LZ4F_VERSION);
    assert_eq!(cr, 0, "{}: createDecompressionContext failed {cr:#x}", api.tag);
    let (steps, out, full) = drive_dctx(api, dctx, frame, p);
    let free_ret = (api.free)(dctx);
    Trace { steps, out, full, free_ret }
}

#[track_caller]
fn cmp_traces(ctx: &str, a: &Trace, b: &Trace) {
    let n = a.steps.len().min(b.steps.len());
    for k in 0..n {
        if a.steps[k] != b.steps[k] {
            panic!(
                "{ctx}: LZ4F_decompress call #{k} differs\n  C   : {:?}\n  Rust: {:?}\n  preceding C steps: {:?}",
                a.steps[k],
                b.steps[k],
                &a.steps[k.saturating_sub(3)..k]
            );
        }
    }
    assert_eq!(
        a.steps.len(),
        b.steps.len(),
        "{ctx}: number of LZ4F_decompress calls differs (C={} Rust={}); \
         first divergent step index {n}, C={:?} Rust={:?}",
        a.steps.len(),
        b.steps.len(),
        a.steps.get(n),
        b.steps.get(n)
    );
    same_full_buffers(&format!("{ctx}: decoded output"), &a.out, &b.out);
    same_full_buffers(&format!("{ctx}: whole dst buffer"), &a.full, &b.full);
    assert_eq!(
        a.free_ret as isize, b.free_ret as isize,
        "{ctx}: LZ4F_freeDecompressionContext returned C={} Rust={}",
        a.free_ret as isize, b.free_ret as isize
    );
}

/// Run `frame` through both decoders under `p` and compare everything.
#[track_caller]
unsafe fn diff(ctx: &str, frame: &[u8], p: &Plan) -> Trace {
    let (c, r) = apis();
    let tc = drive(&c, frame, p);
    let tr = drive(&r, frame, p);
    cmp_traces(ctx, &tc, &tr);
    tc
}

/// `diff` plus the assertion that the frame decoded cleanly to `payload`.
#[track_caller]
unsafe fn diff_ok(ctx: &str, frame: &[u8], p: &Plan, payload: &[u8]) -> Trace {
    let t = diff(ctx, frame, p);
    assert_eq!(
        t.last_hint(),
        0,
        "{ctx}: frame did not finish cleanly, last hint = {} (steps: {:?})",
        t.last_hint() as isize,
        &t.steps[t.steps.len().saturating_sub(4)..]
    );
    assert_eq!(
        t.out.len(),
        payload.len(),
        "{ctx}: decoded {} bytes, expected {}",
        t.out.len(),
        payload.len()
    );
    if let Some(i) = first_diff(&t.out, payload) {
        panic!("{ctx}: decoded content differs from the original at index {i}");
    }
    t
}

/// `diff` plus the assertion that both libraries produced exactly `code`.
#[track_caller]
unsafe fn diff_err(ctx: &str, frame: &[u8], p: &Plan, code: usize) -> Trace {
    let t = diff(ctx, frame, p);
    let last = t.last_hint();
    assert_eq!(
        last, code,
        "{ctx}: expected LZ4F error code {} (= {:#x}), got {} (= {:#x}); steps: {:?}",
        (0usize).wrapping_sub(code) as isize,
        code,
        (0usize).wrapping_sub(last) as isize,
        last,
        &t.steps[t.steps.len().saturating_sub(4)..]
    );
    t
}

fn prefs(
    bsid: c_uint,
    bm: c_uint,
    ccrc: c_uint,
    bcrc: c_uint,
    csize: u64,
    level: c_int,
) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = bm;
    p.frameInfo.contentChecksumFlag = ccrc;
    p.frameInfo.blockChecksumFlag = bcrc;
    p.frameInfo.contentSize = csize;
    p.compressionLevel = level;
    p
}

fn opts(stable_dst: c_uint, skip: c_uint) -> LZ4F_decompressOptions_t {
    let mut o = LZ4F_decompressOptions_t::default();
    o.stableDst = stable_dst;
    o.skipChecksums = skip;
    o
}

/// A spread of random frame configurations for the property-style loops.
fn random_prefs(rng: &mut Rng, len: usize) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID =
        [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB][rng.below(5)];
    p.frameInfo.blockMode = [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT][rng.below(2)];
    p.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
    p.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;
    p.frameInfo.contentSize = if rng.below(2) == 0 { 0 } else { len as u64 };
    p.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() };
    p.compressionLevel = [0i32, 1, -3, 2, 9, 12][rng.below(6)];
    p.autoFlush = rng.below(2) as c_uint;
    p.favorDecSpeed = rng.below(2) as c_uint;
    p
}

// ===========================================================================
// Row 142 — whole frame in one call, *dstSizePtr >= maxBlockSize
// ===========================================================================

#[test]
fn row_142_whole_frame_in_one_call() {
    unsafe {
        let mut rng = Rng::new(142);
        // property-style: many randomized frames, each decoded in a single call
        for iter in 0..400 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = match rng.below(4) {
                0 => rng.range(0, 64),
                1 => rng.range(0, 5000),
                2 => rng.range(60_000, 200_000),
                _ => rng.range(0, 70_000),
            };
            let payload = gen(&mut rng, shape, len);
            let p = random_prefs(&mut rng, len);
            let frame = c_frame(&payload, Some(&p));
            let ctx = format!(
                "row142 iter={iter} shape={shape:?} len={len} bsid={} bm={} ccrc={} bcrc={} lvl={}",
                p.frameInfo.blockSizeID,
                p.frameInfo.blockMode,
                p.frameInfo.contentChecksumFlag,
                p.frameInfo.blockChecksumFlag,
                p.compressionLevel
            );
            // dst capacity comfortably above maxBlockSize -> decode straight into dst
            let mut pl = plan(len + block_size_of(p.frameInfo.blockSizeID) + 4096);
            pl.src_sizes = vec![ALL];
            pl.dst_caps = vec![ALL];
            diff_ok(&ctx, &frame, &pl, &payload);
        }

        // every interesting size at default preferences
        for &n in interesting_sizes().iter() {
            let payload = gen(&mut rng, Shape::TextLike, n);
            let frame = c_frame(&payload, None);
            let pl = plan(n + 64 * 1024 + 4096);
            diff_ok(&format!("row142 size={n}"), &frame, &pl, &payload);
        }
    }
}

// ===========================================================================
// Row 143 — source fed one byte at a time (all dstage_store* stages)
// ===========================================================================

#[test]
fn row_143_source_one_byte_at_a_time() {
    unsafe {
        let mut rng = Rng::new(143);
        // Configurations chosen so that storeFrameHeader / storeBlockHeader /
        // storeCBlock / storeSuffix are all traversed. Payloads are kept modest
        // because this is one call per input byte.
        let mut cases: Vec<(String, LZ4F_preferences_t)> = Vec::new();
        for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for (ccrc, bcrc) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)] {
                let mut p = prefs(LZ4F_MAX64KB, bm, ccrc, bcrc, 0, 0);
                p.frameInfo.dictID = 0xABCD_1234;
                cases.push((format!("bm={bm} ccrc={ccrc} bcrc={bcrc} dictID"), p));
            }
        }
        {
            // contentSize + dictID -> the 19-byte maximum frame header, which is
            // buffered in two rounds (minFHSize then the full size)
            let mut p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 24_000, 0);
            p.frameInfo.dictID = 0x1234_5678;
            cases.push(("full 19-byte header".into(), p));
        }

        for (name, p) in &cases {
            let len = 24_000usize;
            let payload = gen(&mut rng, Shape::TextLike, len);
            let mut pp = *p;
            if pp.frameInfo.contentSize != 0 {
                pp.frameInfo.contentSize = len as u64;
            }
            let frame = c_frame(&payload, Some(&pp));
            let mut pl = plan(len + 64 * 1024 + 4096);
            pl.src_sizes = vec![1];
            diff_ok(&format!("row143 one-byte {name}"), &frame, &pl, &payload);
        }

        // randomized small chunk sizes, comparing the hint at every step
        for iter in 0..120 {
            let len = rng.range(0, 40_000);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let payload = gen(&mut rng, shape, len);
            let p = random_prefs(&mut rng, len);
            let frame = c_frame(&payload, Some(&p));
            let mut sizes = Vec::new();
            for _ in 0..rng.range(1, 6) {
                sizes.push(rng.range(1, 40));
            }
            let mut pl = plan(len + block_size_of(p.frameInfo.blockSizeID) + 4096);
            pl.src_sizes = sizes.clone();
            diff_ok(
                &format!("row143 chunks iter={iter} len={len} sizes={sizes:?}"),
                &frame,
                &pl,
                &payload,
            );
        }
    }
}

// ===========================================================================
// Row 144 — dst capacity < maxBlockSize (tmpOut) and dstage_flushOut split
// ===========================================================================

#[test]
fn row_144_dst_smaller_than_maxblocksize_flushout_split() {
    unsafe {
        let mut rng = Rng::new(144);
        let len = 200_000usize;
        let payload = gen(&mut rng, Shape::TextLike, len);

        for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB] {
            for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                let p = prefs(bsid, bm, 1, 1, 0, 0);
                let frame = c_frame(&payload, Some(&p));
                let mbs = block_size_of(bsid);
                // dst well below maxBlockSize -> tmpOut + flushOut over many calls
                for &cap in &[1usize, 3, 100, 4096, 65535] {
                    let mut pl = plan(len + 4096);
                    pl.dst_caps = vec![cap];
                    diff_ok(
                        &format!("row144 bsid={bsid} bm={bm} dstcap={cap}"),
                        &frame,
                        &pl,
                        &payload,
                    );
                }
                // dst much larger than maxBlockSize -> direct decode
                let mut pl = plan(len + mbs + 4096);
                pl.dst_caps = vec![mbs * 2];
                diff_ok(&format!("row144 bsid={bsid} bm={bm} large dst"), &frame, &pl, &payload);

                // alternating small / large dst: the flush is interrupted and a
                // later call takes the direct-into-dst path
                let mut pl = plan(len + mbs + 4096);
                pl.dst_caps = vec![7, mbs * 2, 1000, 3, mbs + 1];
                pl.src_sizes = vec![ALL, 13, 5000];
                diff_ok(&format!("row144 bsid={bsid} bm={bm} mixed dst"), &frame, &pl, &payload);
            }
        }

        // randomized dst capacities
        for iter in 0..120 {
            let n = rng.range(1000, 120_000);
            let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let pay = gen(&mut rng, sh, n);
            let p = random_prefs(&mut rng, n);
            let frame = c_frame(&pay, Some(&p));
            let mut caps = Vec::new();
            for _ in 0..rng.range(1, 5) {
                caps.push(rng.range(1, 3000));
            }
            let mut pl = plan(n + 4096);
            pl.dst_caps = caps.clone();
            pl.src_sizes = vec![rng.range(1, 9000)];
            diff_ok(&format!("row144 rnd iter={iter} n={n} caps={caps:?}"), &frame, &pl, &pay);
        }
    }
}

// ===========================================================================
// Row 145 — blockLinked frames and every LZ4F_updateDict branch
// ===========================================================================

#[test]
fn row_145_blocklinked_updatedict_branches() {
    unsafe {
        let mut rng = Rng::new(145);
        // >= 64 KB of history over several blocks at blockSizeID 4 (64 KB), so
        // prefix continuation, "history in dst >= 64 KB", tmpOut-resident dict
        // and the join branches are all traversed.
        for &len in &[70_000usize, 200_000, 400_000] {
            for shape in [Shape::TextLike, Shape::Compressible, Shape::Periodic] {
                let payload = gen(&mut rng, shape, len);
                let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
                let frame = c_frame(&payload, Some(&p));

                // (a) one big dst: prefix continuation, then dst history >= 64 KB
                let mut pl = plan(len + 4096);
                diff_ok(&format!("row145 len={len} {shape:?} contig-big"), &frame, &pl, &payload);

                // (b) small dst: decode into tmpOut, flush, withinTmp continuation
                pl = plan(len + 4096);
                pl.dst_caps = vec![777];
                diff_ok(&format!("row145 len={len} {shape:?} small-dst"), &frame, &pl, &payload);

                // (c) alternating: tmpOut-resident dict then a direct decode
                //     (dict == tmpOutBuffer, withinTmp == 0 -> "copy dst into
                //     tmp to complete dict")
                pl = plan(len + 70_000);
                pl.dst_caps = vec![300, 200_000, 40, 100_000];
                pl.src_sizes = vec![ALL, 37];
                diff_ok(&format!("row145 len={len} {shape:?} mixed"), &frame, &pl, &payload);

                // (d) a fresh dst buffer per call with stableDst = 0: the
                //     end-of-call history preservation must run every time
                pl = plan(len + 4096);
                pl.dst_mode = DstMode::Fresh;
                pl.dst_caps = vec![5000, 900];
                pl.src_sizes = vec![4096];
                pl.opts = opts(0, 0);
                pl.opts_null = false;
                diff_ok(&format!("row145 len={len} {shape:?} fresh-dst"), &frame, &pl, &payload);

                // (e) fresh dst buffers with stableDst = 1: history stays in the
                //     previous dst buffers (kept alive), so LZ4F_updateDict takes
                //     the "join dict & dest into tmp" branch
                pl = plan(len + 4096);
                pl.dst_mode = DstMode::Fresh;
                pl.dst_caps = vec![3000];
                pl.src_sizes = vec![2048];
                pl.opts = opts(1, 0);
                pl.opts_null = false;
                diff_ok(
                    &format!("row145 len={len} {shape:?} fresh-dst stableDst=1"),
                    &frame,
                    &pl,
                    &payload,
                );
            }
        }

        // larger block sizes with linked blocks
        for bsid in [LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            let len = 900_000usize;
            let payload = gen(&mut rng, Shape::TextLike, len);
            let p = prefs(bsid, LZ4F_BLOCK_LINKED, 1, 1, len as u64, 1);
            let frame = c_frame(&payload, Some(&p));
            let mut pl = plan(len + block_size_of(bsid) + 4096);
            diff_ok(&format!("row145 bsid={bsid} big"), &frame, &pl, &payload);
            pl = plan(len + 4096);
            pl.dst_caps = vec![1234];
            diff_ok(&format!("row145 bsid={bsid} small-dst"), &frame, &pl, &payload);
        }
    }
}

// ===========================================================================
// Row 146 — blockIndependent at every blockSizeID
// ===========================================================================

#[test]
fn row_146_blockindependent_each_blocksizeid() {
    unsafe {
        let mut rng = Rng::new(146);
        for bsid in [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            let mbs = block_size_of(bsid);
            // enough payload for several independent blocks
            for &len in &[1usize, 100, mbs.min(300_000), mbs.min(300_000) * 2 + 12345] {
                for shape in [Shape::TextLike, Shape::Incompressible] {
                    let payload = gen(&mut rng, shape, len);
                    let p = prefs(bsid, LZ4F_BLOCK_INDEPENDENT, 1, 1, 0, 0);
                    let frame = c_frame(&payload, Some(&p));
                    let ctx = format!("row146 bsid={bsid} len={len} {shape:?}");
                    let mut pl = plan(len + mbs + 4096);
                    diff_ok(&format!("{ctx} one-call"), &frame, &pl, &payload);
                    pl = plan(len + 4096);
                    pl.dst_caps = vec![611];
                    pl.src_sizes = vec![997];
                    diff_ok(&format!("{ctx} chunked"), &frame, &pl, &payload);
                }
            }
        }
    }
}

// ===========================================================================
// Row 147 — decompressOptions stableDst 0 vs 1 on a blockLinked frame
// ===========================================================================

#[test]
fn row_147_stabledst_0_and_1_on_blocklinked() {
    unsafe {
        let mut rng = Rng::new(147);
        let len = 300_000usize;
        let payload = gen(&mut rng, Shape::TextLike, len);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, len as u64, 0);
        let frame = c_frame(&payload, Some(&p));

        for stable in [0u32, 1] {
            for &cap in &[ALL, 100_000usize, 5000, 71, 1] {
                let mut pl = plan(len + 70_000);
                pl.dst_caps = vec![cap];
                pl.opts = opts(stable, 0);
                pl.opts_null = false;
                diff_ok(
                    &format!("row147 stableDst={stable} dstcap={cap}"),
                    &frame,
                    &pl,
                    &payload,
                );
            }
            // src chunked too, so the preservation code runs at many stages
            for &s in &[1usize, 5, 4096] {
                let mut pl = plan(len + 70_000);
                pl.src_sizes = vec![s];
                pl.dst_caps = vec![3000];
                pl.opts = opts(stable, 0);
                pl.opts_null = false;
                diff_ok(
                    &format!("row147 stableDst={stable} src={s}"),
                    &frame,
                    &pl,
                    &payload,
                );
            }
        }

        // stableDst flipping between calls (it is NOT sticky, unlike skipChecksums)
        let mut pl = plan(len + 70_000);
        pl.dst_caps = vec![4096];
        pl.opts = opts(1, 0);
        pl.opts_null = false;
        let (c, r) = apis();
        let mut tc: Vec<Step> = Vec::new();
        let mut tr: Vec<Step> = Vec::new();
        for (api, sink) in [(&c, &mut tc), (&r, &mut tr)] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut big = vec![FILL; len + GUARD];
            let mut src_off = 0usize;
            let mut out_off = 0usize;
            let mut i = 0usize;
            while src_off < frame.len() {
                let o = opts((i % 2) as c_uint, 0);
                let mut ds = 4096.min(len - out_off);
                let mut ss = 3000.min(frame.len() - src_off);
                let ret = (api.decompress)(
                    dctx,
                    big.as_mut_ptr().add(out_off) as *mut c_void,
                    &mut ds,
                    frame.as_ptr().add(src_off) as *const c_void,
                    &mut ss,
                    &o,
                );
                sink.push(Step { hint: ret, src: ss, dst: ds, req_src: 3000, req_dst: 4096 });
                assert!(!is_err_range(ret), "{}: unexpected error {ret:#x}", api.tag);
                src_off += ss;
                out_off += ds;
                i += 1;
                if ret == 0 {
                    break;
                }
            }
            assert_eq!(&big[..out_off], &payload[..out_off], "{}: content mismatch", api.tag);
            assert_eq!(out_off, len, "{}: short decode", api.tag);
            let _ = (api.free)(dctx);
        }
        assert_eq!(tc, tr, "row147: alternating stableDst traces differ");
    }
}

// ===========================================================================
// Row 148 — skipChecksums on a frame carrying content AND block checksums
// ===========================================================================

#[test]
fn row_148_skipchecksums_sticky_and_ignores_bad_checksums() {
    unsafe {
        let mut rng = Rng::new(148);
        let len = 150_000usize;
        let payload = gen(&mut rng, Shape::TextLike, len);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, len as u64, 0);
        let good = c_frame(&payload, Some(&p));
        let lay = parse_frame(&good);
        assert!(lay.blocks.len() >= 2, "row148: expected several blocks");
        assert!(lay.content_crc.is_some(), "row148: expected a content checksum");
        assert!(lay.blocks.iter().all(|b| !b.stored), "row148: expected compressed blocks");

        // both flags off / on, on a valid frame
        for skip in [0u32, 1] {
            for &cap in &[ALL, 5000usize] {
                let mut pl = plan(len + 70_000);
                pl.dst_caps = vec![cap];
                pl.opts = opts(0, skip);
                pl.opts_null = false;
                diff_ok(&format!("row148 valid skip={skip} cap={cap}"), &good, &pl, &payload);
            }
        }

        // --- corrupted CONTENT checksum -------------------------------------
        let mut bad_content = good.clone();
        bad_content[lay.content_crc.unwrap()] ^= 0xFF;
        let mut pl = plan(len + 70_000);
        pl.opts = opts(0, 0);
        pl.opts_null = false;
        diff_err("row148 bad content crc skip=0", &bad_content, &pl, err(18));
        // skipChecksums = 1 makes the very same frame decode successfully
        pl.opts = opts(0, 1);
        diff_ok("row148 bad content crc skip=1", &bad_content, &pl, &payload);
        pl.dst_caps = vec![333];
        diff_ok("row148 bad content crc skip=1 small dst", &bad_content, &pl, &payload);

        // --- corrupted checksum of a COMPRESSED block -----------------------
        // NOTE: lz4frame.c:1878 is *not* guarded by dctx->skipChecksum, so a
        // compressed block's trailing CRC is verified even when skipChecksums=1.
        // Both libraries must agree on that.
        let mut bad_cblock = good.clone();
        bad_cblock[lay.blocks[1].crc.unwrap()] ^= 0xFF;
        for skip in [0u32, 1] {
            let mut pl = plan(len + 70_000);
            pl.opts = opts(0, skip);
            pl.opts_null = false;
            diff_err(
                &format!("row148 bad compressed-block crc skip={skip}"),
                &bad_cblock,
                &pl,
                err(7),
            );
        }

        // --- corrupted checksum of a STORED block ---------------------------
        // dstage_getBlockChecksum *is* guarded by skipChecksum.
        let inc = gen(&mut rng, Shape::Incompressible, 40_000);
        let stored_frame = c_frame(&inc, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 0, 0)));
        let slay = parse_frame(&stored_frame);
        assert!(slay.blocks.iter().any(|b| b.stored), "row148: expected a stored block");
        let sb = slay.blocks.iter().position(|b| b.stored).unwrap();
        let mut bad_sblock = stored_frame.clone();
        bad_sblock[slay.blocks[sb].crc.unwrap()] ^= 0x80;
        let mut pl = plan(inc.len() + 70_000);
        pl.opts = opts(0, 0);
        pl.opts_null = false;
        diff_err("row148 bad stored-block crc skip=0", &bad_sblock, &pl, err(7));
        pl.opts = opts(0, 1);
        diff_ok("row148 bad stored-block crc skip=1", &bad_sblock, &pl, &inc);
        // and with the checksum arriving byte by byte (dstage_getBlockChecksum
        // buffering path)
        pl.src_sizes = vec![1];
        pl.opts = opts(0, 0);
        diff_err("row148 bad stored-block crc skip=0 bytewise", &bad_sblock, &pl, err(7));
        pl.opts = opts(0, 1);
        diff_ok("row148 bad stored-block crc skip=1 bytewise", &bad_sblock, &pl, &inc);

        // --- stickiness -----------------------------------------------------
        // skipChecksums set only on the FIRST call must disable checking for the
        // remainder of the frame (the corrupt content checksum is not caught).
        let (c, r) = apis();
        let mut results: Vec<(Vec<Step>, Vec<u8>, usize)> = Vec::new();
        for api in [&c, &r] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut big = vec![FILL; len + GUARD];
            let mut steps = Vec::new();
            let mut src_off = 0usize;
            let mut out_off = 0usize;
            let mut i = 0usize;
            loop {
                let o = opts(0, if i == 0 { 1 } else { 0 });
                let mut ds = 8192.min(len - out_off);
                let mut ss = 6000.min(bad_content.len() - src_off);
                let ret = (api.decompress)(
                    dctx,
                    big.as_mut_ptr().add(out_off) as *mut c_void,
                    &mut ds,
                    bad_content.as_ptr().add(src_off) as *const c_void,
                    &mut ss,
                    &o,
                );
                steps.push(Step { hint: ret, src: ss, dst: ds, req_src: 6000, req_dst: 8192 });
                src_off += ss;
                out_off += ds;
                if is_err_range(ret) || ret == 0 {
                    break;
                }
                i += 1;
                if ss == 0 && ds == 0 {
                    break;
                }
            }
            let fr = (api.free)(dctx);
            results.push((steps, big[..out_off].to_vec(), fr));
        }
        assert_eq!(results[0].0, results[1].0, "row148: sticky-skip traces differ");
        same_full_buffers("row148 sticky-skip output", &results[0].1, &results[1].1);
        assert_eq!(results[0].2 as isize, results[1].2 as isize);
        assert_eq!(
            results[0].0.last().unwrap().hint,
            0,
            "row148: sticky skipChecksums should let the corrupt frame finish, got {:#x}",
            results[0].0.last().unwrap().hint
        );
        assert_eq!(results[0].1, payload, "row148: sticky-skip decoded wrong content");
    }
}

// ===========================================================================
// Row 149 — corrupted content / block / header checksums
// ===========================================================================

#[test]
fn row_149_corrupted_checksums() {
    unsafe {
        let mut rng = Rng::new(149);
        let len = 120_000usize;
        let payload = gen(&mut rng, Shape::TextLike, len);
        let inc = gen(&mut rng, Shape::Incompressible, 90_000);

        // --- content checksum -> err(18) ---------------------------------
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let f = c_frame(&payload, Some(&p));
        let lay = parse_frame(&f);
        let cc = lay.content_crc.unwrap();
        assert_eq!(cc + 4, f.len(), "row149: content checksum must be the last 4 bytes");
        for k in 0..4 {
            let mut bad = f.clone();
            bad[cc + k] ^= 1 << (k as u32);
            for &s in &[ALL, 1usize, 7] {
                let mut pl = plan(len + 70_000);
                pl.src_sizes = vec![s];
                diff_err(
                    &format!("row149 content crc byte {k} src={s}"),
                    &bad,
                    &pl,
                    err(18),
                );
            }
        }

        // --- block checksum on a COMPRESSED block -> err(7) ---------------
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 1, 0, 0);
        let f = c_frame(&payload, Some(&p));
        let lay = parse_frame(&f);
        assert!(lay.blocks.len() >= 2);
        for bi in [0usize, 1, lay.blocks.len() - 1] {
            assert!(!lay.blocks[bi].stored, "row149: block {bi} unexpectedly stored");
            let mut bad = f.clone();
            bad[lay.blocks[bi].crc.unwrap() + 2] ^= 0x40;
            for &s in &[ALL, 1usize, 1000] {
                let mut pl = plan(len + 70_000);
                pl.src_sizes = vec![s];
                diff_err(
                    &format!("row149 compressed block {bi} crc src={s}"),
                    &bad,
                    &pl,
                    err(7),
                );
            }
        }

        // --- block checksum on a STORED block -> err(7) -------------------
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 1, 0, 0);
        let f = c_frame(&inc, Some(&p));
        let lay = parse_frame(&f);
        let si = lay
            .blocks
            .iter()
            .position(|b| b.stored)
            .expect("row149: expected a stored block for incompressible data");
        let mut bad = f.clone();
        bad[lay.blocks[si].crc.unwrap() + 3] ^= 0x01;
        for &s in &[ALL, 1usize, 3, 9999] {
            for &d in &[ALL, 100usize] {
                let mut pl = plan(inc.len() + 70_000);
                pl.src_sizes = vec![s];
                pl.dst_caps = vec![d];
                diff_err(
                    &format!("row149 stored block crc src={s} dst={d}"),
                    &bad,
                    &pl,
                    err(7),
                );
            }
        }

        // --- header checksum -> err(17) -----------------------------------
        let f = c_frame(&payload[..100], None);
        let lay = parse_frame(&f);
        for delta in [1u8, 0x80, 0xFF] {
            let mut bad = f.clone();
            bad[lay.hsize - 1] = bad[lay.hsize - 1].wrapping_add(delta);
            if bad[lay.hsize - 1] == f[lay.hsize - 1] {
                continue;
            }
            for &s in &[ALL, 1usize, 6] {
                let mut pl = plan(70_000);
                pl.src_sizes = vec![s];
                diff_err(&format!("row149 header crc +{delta} src={s}"), &bad, &pl, err(17));
            }
        }
        // ... including the long 19-byte header (contentSize + dictID)
        let mut pfull = prefs(LZ4F_MAX256KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 100, 0);
        pfull.frameInfo.dictID = 0xFEED_F00D;
        let f = c_frame(&payload[..100], Some(&pfull));
        let lay = parse_frame(&f);
        assert_eq!(lay.hsize, 19);
        let mut bad = f.clone();
        bad[18] ^= 0x55;
        for &s in &[ALL, 1usize, 18] {
            let mut pl = plan(70_000);
            pl.src_sizes = vec![s];
            diff_err(&format!("row149 19-byte header crc src={s}"), &bad, &pl, err(17));
        }
    }
}

// ===========================================================================
// Row 150 — invalid frame-header fields
// ===========================================================================

#[test]
fn row_150_invalid_header_fields() {
    unsafe {
        // (name, bytes, expected error)
        let mut cases: Vec<(String, Vec<u8>, usize)> = Vec::new();

        // bad magic -> err(13). 0x184D2A50..5F is skippable, so avoid it.
        for m in [
            0x184D2205u32,
            0x184D2203,
            0x184D2A60,
            0x184D2A4F,
            0x00000000,
            0xFFFFFFFF,
            0x184D2304,
        ] {
            let mut v = Vec::new();
            v.extend_from_slice(&m.to_le_bytes());
            v.push(flg(false, false, false, false, false));
            v.push(bd(4));
            let hc = header_checksum(&v[4..]);
            v.push(hc);
            v.extend_from_slice(&0u32.to_le_bytes()); // endMark, never reached
            cases.push((format!("magic {m:#010x}"), v, err(13)));
        }

        // FLG reserved bit 1 -> err(8)
        let mut v = craft_header(0x42, bd(4), None, None);
        v.extend_from_slice(&0u32.to_le_bytes());
        cases.push(("FLG bit1".into(), v, err(8)));
        let mut v = craft_header(0x7F & 0x42 | 0x40 | 0x02, bd(4), None, None);
        v.extend_from_slice(&0u32.to_le_bytes());
        cases.push(("FLG bit1 with other flags".into(), v, err(8)));

        // FLG version != 1 -> err(6) (checked after the reserved bit)
        for ver in [0u8, 2, 3] {
            let mut v = craft_header((ver << 6) | 0x00, bd(4), None, None);
            v.extend_from_slice(&0u32.to_le_bytes());
            cases.push((format!("FLG version {ver}"), v, err(6)));
        }

        // BD reserved bit 7 -> err(8)
        for extra in [0x80u8, 0xF0, 0xC0] {
            let mut v = craft_header(0x40, bd(4) | extra & 0x80, None, None);
            v.extend_from_slice(&0u32.to_le_bytes());
            cases.push((format!("BD bit7 extra={extra:#x}"), v, err(8)));
        }

        // blockSizeID < 4 -> err(2) (checked after BD bit 7)
        for bsid in [0u8, 1, 2, 3] {
            let mut v = craft_header(0x40, bd(bsid), None, None);
            v.extend_from_slice(&0u32.to_le_bytes());
            cases.push((format!("blockSizeID {bsid}"), v, err(2)));
        }

        // BD low nibble non-zero -> err(8) (checked after blockSizeID)
        for nib in [1u8, 2, 4, 8, 0x0F] {
            let mut v = craft_header(0x40, bd(4) | nib, None, None);
            v.extend_from_slice(&0u32.to_le_bytes());
            cases.push((format!("BD low nibble {nib:#x}"), v, err(8)));
        }
        // ordering check: a bad blockSizeID *and* a bad low nibble -> err(2)
        let mut v = craft_header(0x40, bd(2) | 0x0F, None, None);
        v.extend_from_slice(&0u32.to_le_bytes());
        cases.push(("blockSizeID 2 + low nibble".into(), v, err(2)));
        // ordering check: BD bit7 *and* blockSizeID 0 -> err(8)
        let mut v = craft_header(0x40, 0x80, None, None);
        v.extend_from_slice(&0u32.to_le_bytes());
        cases.push(("BD bit7 + blockSizeID 0".into(), v, err(8)));

        for (name, bytes, expect) in &cases {
            // >= maxFHSize available -> dstage_getFrameHeader fast path
            let mut padded = bytes.clone();
            padded.resize(bytes.len().max(64), 0x00);
            let pl = plan(4096);
            diff_err(&format!("row150 [{name}] one-call"), &padded, &pl, *expect);
            // one byte at a time -> dstage_storeFrameHeader path
            let mut pl = plan(4096);
            pl.src_sizes = vec![1];
            diff_err(&format!("row150 [{name}] bytewise"), &padded, &pl, *expect);
            // exactly the crafted bytes, in one call
            let pl = plan(4096);
            diff_err(&format!("row150 [{name}] exact"), bytes, &pl, *expect);
        }
    }
}

// ===========================================================================
// Row 151 — stored (uncompressed) blocks: dstage_copyDirect
// ===========================================================================

#[test]
fn row_151_stored_blocks_copydirect() {
    unsafe {
        let mut rng = Rng::new(151);
        // Incompressible payload -> LZ4F_makeBlock rewrites blocks as stored.
        for &len in &[1usize, 100, 70_000, 200_000] {
            let payload = gen(&mut rng, Shape::Incompressible, len);
            for bcrc in [0u32, 1] {
                for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    let p = prefs(LZ4F_MAX64KB, bm, 1, bcrc, len as u64, 0);
                    let f = c_frame(&payload, Some(&p));
                    let lay = parse_frame(&f);
                    assert!(
                        lay.blocks.iter().all(|b| b.stored),
                        "row151: incompressible payload should give stored blocks"
                    );
                    let ctx = format!("row151 len={len} bcrc={bcrc} bm={bm}");
                    // whole thing in one call
                    let mut pl = plan(len + 70_000);
                    diff_ok(&format!("{ctx} one-call"), &f, &pl, &payload);
                    // copy split across calls (src and dst both restricted), so
                    // dstage_copyDirect is re-entered and the following block
                    // checksum is read in pieces
                    for &(s, d) in &[(1usize, ALL), (7, 3), (13, 5000), (5000, 11), (ALL, 1)] {
                        pl = plan(len + 70_000);
                        pl.src_sizes = vec![s];
                        pl.dst_caps = vec![d];
                        diff_ok(&format!("{ctx} src={s} dst={d}"), &f, &pl, &payload);
                    }
                }
            }
        }

        // Hand-crafted stored blocks, including a zero-length one and a full
        // maxBlockSize one, with and without block checksums.
        for bcrc in [false, true] {
            let mut f = craft_header(flg(false, bcrc, false, false, false), bd(4), None, None);
            let b1 = gen(&mut rng, Shape::Incompressible, 1);
            let b2 = gen(&mut rng, Shape::Incompressible, 65_536);
            let b3 = gen(&mut rng, Shape::Incompressible, 12_345);
            push_stored_block(&mut f, &b1, bcrc);
            push_stored_block(&mut f, &b2, bcrc);
            push_stored_block(&mut f, &b3, bcrc);
            f.extend_from_slice(&0u32.to_le_bytes());
            let mut expect = Vec::new();
            expect.extend_from_slice(&b1);
            expect.extend_from_slice(&b2);
            expect.extend_from_slice(&b3);
            for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 1usize), (3, 7), (65_535, 65_537)] {
                let mut pl = plan(expect.len() + 70_000);
                pl.src_sizes = vec![s];
                pl.dst_caps = vec![d];
                diff_ok(
                    &format!("row151 crafted bcrc={bcrc} src={s} dst={d}"),
                    &f,
                    &pl,
                    &expect,
                );
            }
        }
    }
}

// ===========================================================================
// Row 152 — block size > maxBlockSize, blockHeader == 0, broken block
// ===========================================================================

#[test]
fn row_152_block_header_edge_cases() {
    unsafe {
        let mut rng = Rng::new(152);

        // --- declared block size > frame maxBlockSize -> err(2) -----------
        for bsid in [4u8, 5, 6, 7] {
            let mbs = block_size_of(bsid as c_uint);
            for &(extra, stored) in &[(1u32, false), (1, true), (0x1000, false), (0x7FFF_FFFF - mbs as u32, false)] {
                let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
                let mut bh = mbs as u32 + extra;
                if stored {
                    bh |= 0x8000_0000;
                }
                f.extend_from_slice(&bh.to_le_bytes());
                f.resize(f.len() + 64, 0xAA);
                let pl = plan(4096);
                diff_err(
                    &format!("row152 bsid={bsid} oversize block extra={extra} stored={stored}"),
                    &f,
                    &pl,
                    err(2),
                );
                let mut pl = plan(4096);
                pl.src_sizes = vec![1];
                diff_err(
                    &format!("row152 bsid={bsid} oversize block bytewise extra={extra}"),
                    &f,
                    &pl,
                    err(2),
                );
            }
            // exactly maxBlockSize is allowed (it just needs more input)
            let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
            f.extend_from_slice(&(mbs as u32 | 0x8000_0000).to_le_bytes());
            let pl = plan(4096);
            let t = diff(&format!("row152 bsid={bsid} exact maxBlockSize"), &f, &pl);
            assert!(
                !is_err_range(t.last_hint()),
                "row152: a block of exactly maxBlockSize must not be rejected, got {:#x}",
                t.last_hint()
            );
        }

        // --- blockHeader == 0 -> clean end of frame ------------------------
        for (name, extra_flg, tail) in [
            ("bare".to_string(), flg(false, false, false, false, false), Vec::<u8>::new()),
            (
                "with content checksum".to_string(),
                flg(false, false, false, true, false),
                xxh32(&[]).to_le_bytes().to_vec(),
            ),
        ] {
            let mut f = craft_header(extra_flg, bd(4), None, None);
            f.extend_from_slice(&0u32.to_le_bytes());
            f.extend_from_slice(&tail);
            for &s in &[ALL, 1usize, 5] {
                let mut pl = plan(4096);
                pl.src_sizes = vec![s];
                let t = diff_ok(&format!("row152 endMark {name} src={s}"), &f, &pl, &[]);
                assert_eq!(t.out.len(), 0);
            }
        }
        // an empty frame produced by the C library, likewise
        let f = c_frame(&[], None);
        let pl = plan(4096);
        diff_ok("row152 empty C frame", &f, &pl, &[]);

        // --- a compressed block that expands beyond maxBlockSize -> err(16)
        for bsid in [4u8, 5] {
            let mbs = block_size_of(bsid as c_uint);
            let blk = overflowing_block_payload(mbs);
            let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
            f.extend_from_slice(&(blk.len() as u32).to_le_bytes());
            f.extend_from_slice(&blk);
            f.extend_from_slice(&0u32.to_le_bytes());
            // large dst -> "decode into destination buffer" path (ERRORS row 46)
            let pl = plan(mbs * 2 + 4096);
            diff_err(&format!("row152 overflow block bsid={bsid} big dst"), &f, &pl, err(16));
            // small dst -> "decode into tmpOut" path (ERRORS row 47)
            let mut pl = plan(mbs * 2 + 4096);
            pl.dst_caps = vec![64];
            diff_err(&format!("row152 overflow block bsid={bsid} small dst"), &f, &pl, err(16));
            let mut pl = plan(mbs * 2 + 4096);
            pl.src_sizes = vec![1];
            diff_err(&format!("row152 overflow block bsid={bsid} bytewise"), &f, &pl, err(16));
        }

        // --- a truncated compressed block -> err(16) -----------------------
        let payload = gen(&mut rng, Shape::TextLike, 90_000);
        let f = c_frame(&payload, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0, 0, 0)));
        let lay = parse_frame(&f);
        assert!(!lay.blocks[0].stored);
        for cut in [1usize, 2, 5, 17] {
            let b = &lay.blocks[0];
            if b.len <= cut {
                continue;
            }
            let newlen = b.len - cut;
            let mut bad = Vec::new();
            bad.extend_from_slice(&f[..b.hdr]);
            bad.extend_from_slice(&(newlen as u32).to_le_bytes());
            bad.extend_from_slice(&f[b.payload..b.payload + newlen]);
            bad.extend_from_slice(&0u32.to_le_bytes());
            for &d in &[ALL, 64usize] {
                let mut pl = plan(200_000);
                pl.dst_caps = vec![d];
                diff_err(&format!("row152 truncated block cut={cut} dst={d}"), &bad, &pl, err(16));
            }
        }

        // --- a single-byte compressed block claiming a literal run --------
        let mut f = craft_header(flg(false, false, false, false, false), bd(4), None, None);
        f.extend_from_slice(&1u32.to_le_bytes());
        f.push(0x10);
        f.extend_from_slice(&0u32.to_le_bytes());
        diff_err("row152 one-byte bogus block", &f, &plan(70_000), err(16));
    }
}

// ===========================================================================
// Row 153 — contentSize declared but decoded total differs
// ===========================================================================

#[test]
fn row_153_contentsize_mismatch_is_framesize_wrong() {
    unsafe {
        let mut rng = Rng::new(153);
        let body = gen(&mut rng, Shape::Incompressible, 50);

        // declared 100, delivered 50 -> frameRemainingSize == 50 -> err(14)
        // declared 20, delivered 50 -> frameRemainingSize underflows -> err(14)
        for declared in [100u64, 20, 51, 49, u64::MAX] {
            for bcrc in [false, true] {
                let mut f = craft_header(
                    flg(false, bcrc, true, false, false),
                    bd(4),
                    Some(declared),
                    None,
                );
                push_stored_block(&mut f, &body, bcrc);
                f.extend_from_slice(&0u32.to_le_bytes());
                for &s in &[ALL, 1usize, 9] {
                    let mut pl = plan(70_000);
                    pl.src_sizes = vec![s];
                    diff_err(
                        &format!("row153 declared={declared} bcrc={bcrc} src={s}"),
                        &f,
                        &pl,
                        err(14),
                    );
                }
            }
        }

        // the exact size is accepted
        let mut f = craft_header(flg(false, false, true, false, false), bd(4), Some(50), None);
        push_stored_block(&mut f, &body, false);
        f.extend_from_slice(&0u32.to_le_bytes());
        diff_ok("row153 declared==delivered", &f, &plan(70_000), &body);

        // real C frames with a declared contentSize round-trip, and truncating
        // the last block of such a frame yields err(14) at dstage_getSuffix
        let payload = gen(&mut rng, Shape::TextLike, 150_000);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0, payload.len() as u64, 0);
        let f = c_frame(&payload, Some(&p));
        diff_ok("row153 C frame exact", &f, &plan(220_000), &payload);
        let lay = parse_frame(&f);
        assert!(lay.blocks.len() >= 2);
        let cutoff = lay.blocks[lay.blocks.len() - 1].hdr;
        let mut short = f[..cutoff].to_vec();
        short.extend_from_slice(&0u32.to_le_bytes());
        for &d in &[ALL, 1000usize] {
            let mut pl = plan(220_000);
            pl.dst_caps = vec![d];
            diff_err(&format!("row153 dropped last block dst={d}"), &short, &pl, err(14));
        }
    }
}

// ===========================================================================
// Row 154 — skippable frames
// ===========================================================================

fn skippable(magic: u32, content: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&magic.to_le_bytes());
    v.extend_from_slice(&(content.len() as u32).to_le_bytes());
    v.extend_from_slice(content);
    v
}

#[test]
fn row_154_skippable_frames_all_magics_and_sizes() {
    unsafe {
        let mut rng = Rng::new(154);
        for m in 0x184D2A50u32..=0x184D2A5Fu32 {
            for &n in &[0usize, 1, 3, 7, 8, 20, 1000, 70_000] {
                let content = gen(&mut rng, Shape::Incompressible, n);
                let f = skippable(m, &content);
                for &s in &[ALL, 1usize, 3, 4, 5, 8, 9, 12] {
                    let mut pl = plan(4096);
                    pl.src_sizes = vec![s];
                    let t = diff(
                        &format!("row154 magic={m:#010x} size={n} src={s}"),
                        &f,
                        &pl,
                    );
                    assert_eq!(
                        t.last_hint(),
                        0,
                        "row154 magic={m:#010x} size={n} src={s}: a complete skippable frame must return 0, got {:#x}",
                        t.last_hint()
                    );
                    assert_eq!(t.out.len(), 0, "row154: a skippable frame must produce no output");
                    let consumed: usize = t.steps.iter().map(|x| x.src).sum();
                    assert_eq!(
                        consumed,
                        f.len(),
                        "row154 magic={m:#010x} size={n} src={s}: only {consumed} of {} bytes consumed",
                        f.len()
                    );
                }
                // dstBuffer NULL: a skippable frame needs no output space at all
                let mut pl = plan(0);
                pl.dst_null = true;
                let t = diff(&format!("row154 magic={m:#010x} size={n} dst=NULL"), &f, &pl);
                assert_eq!(t.last_hint(), 0, "row154: dst=NULL skippable frame");
            }
        }

        // declared size larger than the supplied input -> the skip is split over
        // calls and the hint counts down the bytes still to skip
        for &(declared, supplied) in &[(5000u32, 100usize), (100_000, 4096), (0xFFFF_FFFF, 10)] {
            let mut f = Vec::new();
            f.extend_from_slice(&0x184D2A50u32.to_le_bytes());
            f.extend_from_slice(&declared.to_le_bytes());
            f.extend_from_slice(&gen(&mut rng, Shape::Degenerate, supplied));
            for &s in &[ALL, 1usize, 7] {
                let mut pl = plan(4096);
                pl.src_sizes = vec![s];
                let t = diff(
                    &format!("row154 short skippable declared={declared} supplied={supplied} src={s}"),
                    &f,
                    &pl,
                );
                let last = t.last_hint();
                assert!(
                    !is_err_range(last) && last != 0,
                    "row154: an incomplete skippable frame must return a positive hint, got {last:#x}"
                );
                assert_eq!(
                    last,
                    declared as usize - supplied,
                    "row154: hint must be the number of bytes left to skip"
                );
            }
        }

        // the 8-byte skippable header split across calls in every possible way
        let f = skippable(0x184D2A5F, &gen(&mut rng, Shape::TextLike, 33));
        for cut in 1..=8usize {
            let mut pl = plan(4096);
            pl.src_sizes = vec![cut, ALL];
            let t = diff(&format!("row154 header split at {cut}"), &f, &pl);
            assert_eq!(t.last_hint(), 0, "row154: header split at {cut}");
        }
    }
}

// ===========================================================================
// Row 155 — concatenated frames, srcSize=0, hint-only calls
// ===========================================================================

#[test]
fn row_155_concatenated_frames_and_hint_only_calls() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(155);

        // --- srcSize == 0 on a fresh dctx -> minFHSize (7) hint -------------
        for api in [&c, &r] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut buf = vec![FILL; 128];
            for &(dnull, srcnull) in &[(false, false), (true, false), (false, true), (true, true)] {
                let mut ds = if dnull { 0 } else { 64 };
                let mut ss = 0usize;
                let ret = (api.decompress)(
                    dctx,
                    if dnull { ptr::null_mut() } else { buf.as_mut_ptr() as *mut c_void },
                    &mut ds,
                    if srcnull { ptr::null() } else { buf.as_ptr() as *const c_void },
                    &mut ss,
                    ptr::null(),
                );
                assert_eq!(
                    ret, 7,
                    "{}: srcSize=0 on a fresh dctx must return minFHSize=7, got {ret:#x}",
                    api.tag
                );
                assert_eq!(ss, 0, "{}: *srcSizePtr must be 0", api.tag);
                assert_eq!(ds, 0, "{}: *dstSizePtr must be 0", api.tag);
            }
            assert_eq!((api.free)(dctx), 0, "{}: dStage must still be 0", api.tag);
        }

        // --- hint-only calls (dstBuffer NULL, *dstSizePtr = 0) mid-frame ----
        let payload = gen(&mut rng, Shape::TextLike, 100_000);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 0, 0);
        let frame = c_frame(&payload, Some(&p));
        let mut hints: Vec<Vec<usize>> = Vec::new();
        for api in [&c, &r] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut out = vec![FILL; payload.len() + GUARD];
            let mut h = Vec::new();
            let mut src_off = 0usize;
            let mut out_off = 0usize;
            loop {
                // hint-only probe, exactly as LZ4F_getFrameInfo does it
                let mut zd = 0usize;
                let mut zs = 0usize;
                let probe = (api.decompress)(
                    dctx,
                    ptr::null_mut(),
                    &mut zd,
                    ptr::null(),
                    &mut zs,
                    ptr::null(),
                );
                h.push(probe);
                assert_eq!(zd, 0);
                assert_eq!(zs, 0);
                if src_off >= frame.len() {
                    break;
                }
                let mut ds = (payload.len() - out_off).min(9000);
                let mut ss = (frame.len() - src_off).min(7000);
                let ret = (api.decompress)(
                    dctx,
                    out.as_mut_ptr().add(out_off) as *mut c_void,
                    &mut ds,
                    frame.as_ptr().add(src_off) as *const c_void,
                    &mut ss,
                    ptr::null(),
                );
                h.push(ret);
                assert!(!is_err_range(ret), "{}: unexpected error {ret:#x}", api.tag);
                src_off += ss;
                out_off += ds;
                if ret == 0 {
                    break;
                }
            }
            assert_eq!(&out[..out_off], &payload[..], "{}: content mismatch", api.tag);
            let _ = (api.free)(dctx);
            hints.push(h);
        }
        assert_eq!(
            hints[0].iter().map(|&x| x as isize).collect::<Vec<_>>(),
            hints[1].iter().map(|&x| x as isize).collect::<Vec<_>>(),
            "row155: interleaved hint-only / real call hints differ"
        );

        // --- two frames concatenated in one dctx ---------------------------
        let a = gen(&mut rng, Shape::TextLike, 70_000);
        let b = gen(&mut rng, Shape::Compressible, 30_000);
        let pa = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, a.len() as u64, 0);
        let pb = prefs(LZ4F_MAX256KB, LZ4F_BLOCK_INDEPENDENT, 0, 1, 0, 9);
        let mut both = c_frame(&a, Some(&pa));
        both.extend_from_slice(&c_frame(&b, Some(&pb)));
        let mut expect = a.clone();
        expect.extend_from_slice(&b);
        for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 1usize), (3000, 700), (7, 65_537)] {
            let mut pl = plan(expect.len() + 300_000);
            pl.src_sizes = vec![s];
            pl.dst_caps = vec![d];
            pl.continue_frames = true;
            diff_ok(&format!("row155 two frames src={s} dst={d}"), &both, &pl, &expect);
        }
        // three frames, the middle one empty
        let mut three = c_frame(&a, Some(&pa));
        three.extend_from_slice(&c_frame(&[], None));
        three.extend_from_slice(&c_frame(&b, Some(&pb)));
        let mut pl = plan(expect.len() + 300_000);
        pl.continue_frames = true;
        diff_ok("row155 three frames", &three, &pl, &expect);
        pl.src_sizes = vec![1];
        diff_ok("row155 three frames bytewise", &three, &pl, &expect);

        // --- a frame followed by a skippable frame -------------------------
        for m in [0x184D2A50u32, 0x184D2A5F] {
            for &n in &[0usize, 9, 5000] {
                let mut f = c_frame(&a, Some(&pa));
                f.extend_from_slice(&skippable(m, &gen(&mut rng, Shape::Degenerate, n)));
                for &s in &[ALL, 1usize, 11] {
                    let mut pl = plan(a.len() + 200_000);
                    pl.src_sizes = vec![s];
                    pl.continue_frames = true;
                    diff_ok(
                        &format!("row155 frame + skippable {m:#x}/{n} src={s}"),
                        &f,
                        &pl,
                        &a,
                    );
                }
            }
        }
        // a skippable frame FOLLOWED by a real frame
        let mut f = skippable(0x184D2A53, &gen(&mut rng, Shape::Degenerate, 77));
        f.extend_from_slice(&c_frame(&b, Some(&pb)));
        for &s in &[ALL, 1usize, 6] {
            let mut pl = plan(b.len() + 300_000);
            pl.src_sizes = vec![s];
            pl.continue_frames = true;
            diff_ok(&format!("row155 skippable + frame src={s}"), &f, &pl, &b);
        }
    }
}

// ===========================================================================
// Row 156 — LZ4F_decompress_usingDict
// ===========================================================================

#[test]
fn row_156_decompress_using_dict() {
    unsafe {
        let mut rng = Rng::new(156);
        let dict = gen(&mut rng, Shape::TextLike, 64 * 1024);
        let payload_len = 120_000usize;
        // a payload whose head repeats the dictionary tail, so the first block
        // really does reference the dictionary
        let mut payload = Vec::new();
        payload.extend_from_slice(&dict[dict.len() - 20_000..]);
        payload.extend_from_slice(&gen(&mut rng, Shape::TextLike, payload_len - 20_000));

        // --- a frame that does NOT need a dictionary ----------------------
        let plain = c_frame(&payload, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 0, 0)));
        // dict supplied before init (dStage <= dstage_init) -> applied, and the
        // decode still succeeds because nothing references it
        for (name, dp, dl) in [
            ("NULL/0", ptr::null::<u8>(), 0usize),
            ("ptr/0", dict.as_ptr(), 0),
            ("1 byte", dict.as_ptr(), 1),
            ("100 bytes", dict.as_ptr(), 100),
            ("64KB-1", dict.as_ptr(), 64 * 1024 - 1),
            ("64KB", dict.as_ptr(), 64 * 1024),
        ] {
            for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 500usize), (4096, 77)] {
                let mut pl = plan(payload.len() + 70_000);
                pl.src_sizes = vec![s];
                pl.dst_caps = vec![d];
                pl.dict = Some((dp as usize, dl));
                pl.dict_from_call = 0;
                diff_ok(
                    &format!("row156 plain frame dict={name} src={s} dst={d}"),
                    &plain,
                    &pl,
                    &payload,
                );
            }
        }
        // a dictionary larger than 64 KB (only its last 64 KB can matter)
        let big_dict = gen(&mut rng, Shape::TextLike, 300_000);
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((big_dict.as_ptr() as usize, big_dict.len()));
        diff_ok("row156 plain frame dict>64KB", &plain, &pl, &payload);

        // dict supplied mid-frame is IGNORED: the first call is a plain
        // LZ4F_decompress that consumes the header (dStage becomes
        // dstage_getBlockHeader > dstage_init), so all later dictionaries are
        // ignored by LZ4F_decompress_usingDict.
        let mut pl = plan(payload.len() + 70_000);
        pl.src_sizes = vec![7, ALL];
        pl.dict = Some((big_dict.as_ptr() as usize, big_dict.len()));
        pl.dict_from_call = 1;
        diff_ok("row156 dict mid-frame ignored", &plain, &pl, &payload);

        // --- a frame that DOES need the dictionary ------------------------
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let needy = c_frame_dict(&payload, Some(&dict), Some(&p), 0);
        // sanity: the dictionary really is required
        {
            let t = diff("row156 needy without dict", &needy, &plan(payload.len() + 70_000));
            let differs = is_err_range(t.last_hint())
                || t.out.len() != payload.len()
                || t.out != payload;
            assert!(
                differs,
                "row156: the reference frame does not actually need the dictionary"
            );
        }
        // with the matching dictionary it decodes correctly
        for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 999usize), (3, 5), (65_536, 65_536)] {
            let mut pl = plan(payload.len() + 70_000);
            pl.src_sizes = vec![s];
            pl.dst_caps = vec![d];
            pl.dict = Some((dict.as_ptr() as usize, dict.len()));
            diff_ok(&format!("row156 needy with dict src={s} dst={d}"), &needy, &pl, &payload);
        }
        // a >64 KB buffer whose LAST 64 KB is the dictionary works identically
        let mut padded = gen(&mut rng, Shape::Degenerate, 100_000);
        padded.extend_from_slice(&dict);
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((padded.as_ptr() as usize, padded.len()));
        diff_ok("row156 needy with padded dict", &needy, &pl, &payload);
        // supplying the dictionary only from the second call on -> ignored, and
        // both libraries must fail the same way
        let mut pl = plan(payload.len() + 70_000);
        pl.src_sizes = vec![19, ALL];
        pl.dict = Some((dict.as_ptr() as usize, dict.len()));
        pl.dict_from_call = 1;
        let t = diff("row156 needy dict mid-frame ignored", &needy, &pl);
        assert!(
            is_err_range(t.last_hint()) || t.out != payload,
            "row156: a mid-frame dictionary must be ignored, but the frame decoded correctly"
        );

        // blockIndependent frames: LZ4F_compressBegin_usingDict only applies the
        // dictionary to the first block, but the decoder applies it per block.
        let pi = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 0, 0);
        let needy_i = c_frame_dict(&payload, Some(&dict), Some(&pi), 0);
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((dict.as_ptr() as usize, dict.len()));
        diff_ok("row156 blockIndependent with dict", &needy_i, &pl, &payload);

        // randomized property sweep over dictionary sizes and chunkings
        for iter in 0..120 {
            let n = rng.range(0, 90_000);
            let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let pay = gen(&mut rng, sh, n);
            let dp = random_prefs(&mut rng, n);
            let dsz = [0usize, 1, 17, 4096, 65_535, 65_536, 65_537, 200_000][rng.below(8)];
            let d = gen(&mut rng, Shape::TextLike, dsz.max(1));
            let f = c_frame_dict(&pay, if dsz == 0 { None } else { Some(&d[..dsz]) }, Some(&dp), 0);
            let mut pl = plan(n + block_size_of(dp.frameInfo.blockSizeID) + 70_000);
            pl.src_sizes = vec![rng.range(1, 40_000)];
            pl.dst_caps = vec![rng.range(1, 40_000)];
            pl.dict = Some((d.as_ptr() as usize, dsz));
            diff_ok(
                &format!("row156 property iter={iter} n={n} dictSize={dsz}"),
                &f,
                &pl,
                &pay,
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 24..55
// ===========================================================================

/// `LZ4F_headerSize` in both libraries.
#[track_caller]
unsafe fn diff_header_size(ctx: &str, src: Option<&[u8]>, srcsize: usize) -> usize {
    let (c, r) = apis();
    let p = src.map_or(ptr::null(), |s| s.as_ptr() as *const c_void);
    let a = (c.header_size)(p, srcsize);
    let b = (r.header_size)(p, srcsize);
    assert_eq!(
        a as isize, b as isize,
        "{ctx}: LZ4F_headerSize C={} Rust={}",
        a as isize, b as isize
    );
    a
}

/// `LZ4F_getFrameInfo` on a fresh dctx in both libraries; compares the return
/// value, `*srcSizePtr`, the whole `LZ4F_frameInfo_t` and the subsequent
/// `LZ4F_freeDecompressionContext` return (i.e. the resulting dStage).
#[track_caller]
unsafe fn diff_get_frame_info(ctx: &str, src: &[u8], srcsize: usize) -> (usize, usize, LZ4F_frameInfo_t) {
    let (c, r) = apis();
    let mut res: Vec<(usize, usize, LZ4F_frameInfo_t, usize)> = Vec::new();
    for api in [&c, &r] {
        let mut dctx: *mut c_void = ptr::null_mut();
        assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
        // 0xAA-prefill, so "not written at all" is distinguishable
        let mut fi = LZ4F_frameInfo_t {
            blockSizeID: 0xAAAA_AAAA,
            blockMode: 0xAAAA_AAAA,
            contentChecksumFlag: 0xAAAA_AAAA,
            frameType: 0xAAAA_AAAA,
            contentSize: 0xAAAA_AAAA_AAAA_AAAA,
            dictID: 0xAAAA_AAAA,
            blockChecksumFlag: 0xAAAA_AAAA,
        };
        let mut ss = srcsize;
        let ret = (api.get_frame_info)(dctx, &mut fi, src.as_ptr() as *const c_void, &mut ss);
        let fr = (api.free)(dctx);
        res.push((ret, ss, fi, fr));
    }
    assert_eq!(
        res[0].0 as isize, res[1].0 as isize,
        "{ctx}: LZ4F_getFrameInfo return C={} Rust={}",
        res[0].0 as isize, res[1].0 as isize
    );
    assert_eq!(res[0].1, res[1].1, "{ctx}: *srcSizePtr C={} Rust={}", res[0].1, res[1].1);
    assert_eq!(
        res[0].2, res[1].2,
        "{ctx}: frameInfo C={:?} Rust={:?}",
        res[0].2, res[1].2
    );
    assert_eq!(
        res[0].3 as isize, res[1].3 as isize,
        "{ctx}: freeDecompressionContext (dStage) C={} Rust={}",
        res[0].3 as isize, res[1].3 as isize
    );
    (res[0].0, res[0].1, res[0].2)
}

/// A minimal valid plain frame header (7 bytes) plus an endMark.
fn tiny_valid_frame() -> Vec<u8> {
    let mut v = craft_header(flg(false, false, false, false, false), bd(4), None, None);
    v.extend_from_slice(&0u32.to_le_bytes());
    v
}

/// ERRORS row 24 — `LZ4F_decodeHeader` with `srcSize < minFHSize (7)`
/// (lz4frame.c:1354) is **UNREACHABLE** from the public API:
///  * `LZ4F_decompress`/`dstage_getFrameHeader` only calls it when at least
///    `maxFHSize (19)` bytes are available;
///  * `dstage_storeFrameHeader` first buffers `tmpInTarget >= minFHSize` bytes
///    and then calls it with exactly `tmpInTarget`;
///  * `LZ4F_getFrameInfo` calls it with `hSize` from `LZ4F_headerSize`, which is
///    always one of 7 / 11 / 15 / 19 and is checked against `*srcSizePtr` first
///    (that check produces lz4frame.c:1507, i.e. ERRORS row 37, instead).
/// The closest reachable observable behaviour is asserted here: fewer than 7
/// bytes never reach line 1354 — `LZ4F_getFrameInfo` reports err(12) from
/// line 1450/1507 and `LZ4F_decompress` just returns a hint.
#[test]
fn err_24_decodeHeader_srcSize_below_minFHSize_is_unreachable() {
    unsafe {
        let f = tiny_valid_frame();
        // LZ4F_getFrameInfo with 0..6 bytes: err(12), never reaching line 1354
        for n in 0..7usize {
            let (ret, ss, _fi) = diff_get_frame_info(&format!("err24 getFrameInfo n={n}"), &f, n);
            assert_eq!(
                ret,
                err(12),
                "err24: getFrameInfo with {n} bytes must be err(12), got {ret:#x}"
            );
            assert_eq!(ss, 0, "err24: *srcSizePtr must be 0 on failure");
        }
        // LZ4F_decompress with 1..6 bytes: no error at all, just a hint
        for n in 1..7usize {
            let mut pl = plan(64);
            pl.src_sizes = vec![n];
            let t = diff(&format!("err24 decompress n={n}"), &f[..n], &pl);
            let last = t.last_hint();
            assert!(
                !is_err_range(last),
                "err24: LZ4F_decompress with {n} header bytes must not fail, got {last:#x}"
            );
            assert_eq!(
                last,
                (7 - n) + 4,
                "err24: hint must be (rest of header + block header), got {last}"
            );
        }
    }
}

/// ERRORS row 25 — unknown magic number -> err(13).
#[test]
fn err_25_frame_type_unknown() {
    unsafe {
        for m in [0x184D2205u32, 0x184D2203, 0x184D2A60, 0x184D2A4F, 0, 0xFFFF_FFFF] {
            let mut f = tiny_valid_frame();
            f[..4].copy_from_slice(&m.to_le_bytes());
            f.resize(64, 0);
            diff_err(&format!("err25 magic={m:#010x} one-call"), &f, &plan(64), err(13));
            let mut pl = plan(64);
            pl.src_sizes = vec![1];
            diff_err(&format!("err25 magic={m:#010x} bytewise"), &f, &pl, err(13));
            // and via LZ4F_getFrameInfo (forwarded from LZ4F_headerSize)
            let (ret, ss, _) = diff_get_frame_info(&format!("err25 gfi {m:#010x}"), &f, f.len());
            assert_eq!(ret, err(13), "err25: getFrameInfo must forward err(13)");
            assert_eq!(ss, 0);
        }
    }
}

/// ERRORS row 26 — FLG reserved bit 1 -> err(8).
#[test]
fn err_26_flg_reserved_bit_set() {
    unsafe {
        for flgb in [0x42u8, 0x62, 0x52, 0x4A, 0x46, 0x43, 0x7F] {
            let mut f = craft_header(flgb, bd(4), None, None);
            f.resize(64, 0);
            diff_err(&format!("err26 FLG={flgb:#04x}"), &f, &plan(64), err(8));
            let mut pl = plan(64);
            pl.src_sizes = vec![1];
            diff_err(&format!("err26 FLG={flgb:#04x} bytewise"), &f, &pl, err(8));
        }
        // the reserved bit is checked BEFORE the version field
        let mut f = craft_header(0x82, bd(4), None, None);
        f.resize(64, 0);
        diff_err("err26 FLG=0x82 (bad version too)", &f, &plan(64), err(8));
    }
}

/// ERRORS row 27 — FLG version field != 1 -> err(6).
#[test]
fn err_27_header_version_wrong() {
    unsafe {
        for ver in [0u8, 2, 3] {
            for low in [0x00u8, 0x20, 0x1D] {
                let flgb = (ver << 6) | (low & 0x3D);
                let mut f = craft_header(flgb, bd(4), None, None);
                f.resize(80, 0);
                diff_err(
                    &format!("err27 version={ver} FLG={flgb:#04x}"),
                    &f,
                    &plan(64),
                    err(6),
                );
                let mut pl = plan(64);
                pl.src_sizes = vec![1];
                diff_err(
                    &format!("err27 version={ver} FLG={flgb:#04x} bytewise"),
                    &f,
                    &pl,
                    err(6),
                );
            }
        }
    }
}

/// ERRORS row 28 — BD reserved bit 7 -> err(8).
#[test]
fn err_28_bd_reserved_bit_7() {
    unsafe {
        for bdb in [0x80u8, 0xC0, 0xF0, 0x81, 0xFF, 0x8F] {
            let mut f = craft_header(0x40, bdb, None, None);
            f.resize(64, 0);
            diff_err(&format!("err28 BD={bdb:#04x}"), &f, &plan(64), err(8));
            let mut pl = plan(64);
            pl.src_sizes = vec![1];
            diff_err(&format!("err28 BD={bdb:#04x} bytewise"), &f, &pl, err(8));
        }
    }
}

/// ERRORS row 29 — blockSizeID `(BD>>4)&7` < 4 -> err(2).
#[test]
fn err_29_max_block_size_invalid() {
    unsafe {
        for bsid in [0u8, 1, 2, 3] {
            let mut f = craft_header(0x40, bd(bsid), None, None);
            f.resize(64, 0);
            diff_err(&format!("err29 blockSizeID={bsid}"), &f, &plan(64), err(2));
            let mut pl = plan(64);
            pl.src_sizes = vec![1];
            diff_err(&format!("err29 blockSizeID={bsid} bytewise"), &f, &pl, err(2));
            // via getFrameInfo the error is forwarded (ERRORS row 38)
            let (ret, ss, _) = diff_get_frame_info(&format!("err29 gfi {bsid}"), &f, f.len());
            assert_eq!(ret, err(2), "err29: getFrameInfo must forward err(2)");
            assert_eq!(ss, 0);
        }
        // 4..7 are accepted
        for bsid in [4u8, 5, 6, 7] {
            let mut f = craft_header(0x40, bd(bsid), None, None);
            f.extend_from_slice(&0u32.to_le_bytes());
            diff_ok(&format!("err29 blockSizeID={bsid} accepted"), &f, &plan(64), &[]);
        }
    }
}

/// ERRORS row 30 — BD low nibble non-zero -> err(8), checked *after* the
/// blockSizeID range test.
#[test]
fn err_30_bd_low_nibble_reserved() {
    unsafe {
        for nib in 1..16u8 {
            for bsid in [4u8, 5, 6, 7] {
                let mut f = craft_header(0x40, bd(bsid) | nib, None, None);
                f.resize(64, 0);
                diff_err(&format!("err30 bsid={bsid} nibble={nib:#x}"), &f, &plan(64), err(8));
            }
            // with an invalid blockSizeID, err(2) wins
            let mut f = craft_header(0x40, bd(3) | nib, None, None);
            f.resize(64, 0);
            diff_err(&format!("err30 bsid=3 nibble={nib:#x} -> err(2)"), &f, &plan(64), err(2));
        }
    }
}

/// ERRORS row 31 — header checksum byte mismatch -> err(17).
#[test]
fn err_31_header_checksum_invalid() {
    unsafe {
        let mut rng = Rng::new(31);
        // 7-byte, 11-byte, 15-byte and 19-byte headers
        let variants: Vec<(String, Vec<u8>)> = vec![
            ("7".into(), craft_header(flg(false, false, false, false, false), bd(4), None, None)),
            ("11".into(), craft_header(flg(false, false, false, false, true), bd(4), None, Some(0xDEAD_BEEF))),
            ("15".into(), craft_header(flg(false, false, true, false, false), bd(5), Some(1234), None)),
            ("19".into(), craft_header(flg(true, true, true, true, true), bd(7), Some(999_999), Some(0x1234_5678))),
        ];
        for (name, base) in &variants {
            assert_eq!(base.len(), name.parse::<usize>().unwrap());
            for _ in 0..8 {
                let mut f = base.clone();
                let last = f.len() - 1;
                let mut nb = rng.byte();
                if nb == f[last] {
                    nb = nb.wrapping_add(1);
                }
                f[last] = nb;
                f.resize(64, 0);
                diff_err(&format!("err31 hsize={name} HC={nb:#04x}"), &f, &plan(64), err(17));
                let mut pl = plan(64);
                pl.src_sizes = vec![1];
                diff_err(
                    &format!("err31 hsize={name} HC={nb:#04x} bytewise"),
                    &f,
                    &pl,
                    err(17),
                );
                let (ret, ss, _) =
                    diff_get_frame_info(&format!("err31 gfi hsize={name}"), &f, f.len());
                assert_eq!(ret, err(17), "err31: getFrameInfo must forward err(17)");
                assert_eq!(ss, 0);
            }
            // the untouched header is accepted
            let mut good = base.clone();
            good.extend_from_slice(&0u32.to_le_bytes());
            if !(base[4] >> 3) & 1 == 1 {
                // no declared contentSize -> an empty frame is valid
            }
        }
        // an entirely valid 19-byte header with a correct checksum decodes
        let payload = gen(&mut rng, Shape::TextLike, 1000);
        let mut p = prefs(LZ4F_MAX256KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 1000, 0);
        p.frameInfo.dictID = 0x1234_5678;
        let f = c_frame(&payload, Some(&p));
        assert_eq!(parse_frame(&f).hsize, 19);
        diff_ok("err31 valid 19-byte header", &f, &plan(400_000), &payload);
    }
}

/// ERRORS row 32 — `LZ4F_headerSize(NULL, ..)` -> err(15).
#[test]
fn err_32_headerSize_null_src() {
    unsafe {
        for n in [0usize, 1, 4, 5, 7, 19, usize::MAX] {
            let ret = diff_header_size(&format!("err32 srcSize={n}"), None, n);
            assert_eq!(
                ret,
                err(15),
                "err32: LZ4F_headerSize(NULL,{n}) must be err(15) (srcPtr_wrong), got {ret:#x}"
            );
        }
    }
}

/// ERRORS row 33 — `LZ4F_headerSize` with `srcSize < 5` -> err(12).
#[test]
fn err_33_headerSize_srcSize_too_small() {
    unsafe {
        let f = tiny_valid_frame();
        for n in 0..5usize {
            let ret = diff_header_size(&format!("err33 srcSize={n}"), Some(&f), n);
            assert_eq!(
                ret,
                err(12),
                "err33: LZ4F_headerSize(src,{n}) must be err(12), got {ret:#x}"
            );
        }
        // 5 bytes is enough to know the header size
        for n in 5..=f.len() {
            let ret = diff_header_size(&format!("err33 srcSize={n}"), Some(&f), n);
            assert_eq!(ret, 7, "err33: expected header size 7, got {ret:#x}");
        }
    }
}

/// ERRORS row 34 — `LZ4F_headerSize` with an unknown magic -> err(13); the
/// skippable magic range returns 8 and the flag bits drive 7 / 11 / 15 / 19.
#[test]
fn err_34_headerSize_magic_and_sizes() {
    unsafe {
        for m in [0x184D2205u32, 0x184D2203, 0x184D2A60, 0x184D2A4F, 0, 0xFFFF_FFFF, 0x184D2304] {
            let mut f = tiny_valid_frame();
            f[..4].copy_from_slice(&m.to_le_bytes());
            let ret = diff_header_size(&format!("err34 magic={m:#010x}"), Some(&f), f.len());
            assert_eq!(ret, err(13), "err34: magic {m:#010x} must be err(13), got {ret:#x}");
        }
        // all 16 skippable magics -> 8
        for m in 0x184D2A50u32..=0x184D2A5Fu32 {
            let f = skippable(m, &[1, 2, 3]);
            let ret = diff_header_size(&format!("err34 skippable {m:#010x}"), Some(&f), f.len());
            assert_eq!(ret, 8, "err34: skippable magic must give 8, got {ret:#x}");
        }
        // flag-driven header sizes
        for (csf, dif, want) in [(false, false, 7usize), (false, true, 11), (true, false, 15), (true, true, 19)] {
            let f = craft_header(
                flg(false, false, csf, false, dif),
                bd(4),
                if csf { Some(42) } else { None },
                if dif { Some(7) } else { None },
            );
            assert_eq!(f.len(), want);
            let ret = diff_header_size(&format!("err34 csf={csf} dif={dif}"), Some(&f), f.len());
            assert_eq!(ret, want, "err34: expected header size {want}, got {ret:#x}");
            // 5 bytes are enough to compute it
            let ret = diff_header_size(&format!("err34 csf={csf} dif={dif} n=5"), Some(&f), 5);
            assert_eq!(ret, want);
        }
    }
}

/// ERRORS row 35 — `LZ4F_getFrameInfo` while `dStage == dstage_storeFrameHeader`
/// -> err(19), `*srcSizePtr = 0`.
#[test]
fn err_35_getFrameInfo_already_started() {
    unsafe {
        let (c, r) = apis();
        let f = tiny_valid_frame();
        for stop in 1..7usize {
            let mut res: Vec<(usize, usize, usize, LZ4F_frameInfo_t, usize)> = Vec::new();
            for api in [&c, &r] {
                let mut dctx: *mut c_void = ptr::null_mut();
                assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
                // partial header -> dStage becomes dstage_storeFrameHeader (1)
                let mut ds = 0usize;
                let mut ss = stop;
                let h = (api.decompress)(
                    dctx,
                    ptr::null_mut(),
                    &mut ds,
                    f.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                );
                let mut fi = LZ4F_frameInfo_t::default();
                let mut gs = f.len();
                let g = (api.get_frame_info)(dctx, &mut fi, f.as_ptr() as *const c_void, &mut gs);
                let fr = (api.free)(dctx);
                res.push((h, g, gs, fi, fr));
            }
            assert_eq!(
                res[0].0 as isize, res[1].0 as isize,
                "err35 stop={stop}: decompress hint differs"
            );
            assert_eq!(
                res[0].1 as isize, res[1].1 as isize,
                "err35 stop={stop}: getFrameInfo C={} Rust={}",
                res[0].1 as isize, res[1].1 as isize
            );
            assert_eq!(res[0].2, res[1].2, "err35 stop={stop}: *srcSizePtr differs");
            assert_eq!(res[0].3, res[1].3, "err35 stop={stop}: frameInfo differs");
            assert_eq!(
                res[0].4 as isize, res[1].4 as isize,
                "err35 stop={stop}: free() dStage differs"
            );
            assert_eq!(
                res[0].1,
                err(19),
                "err35 stop={stop}: expected frameDecoding_alreadyStarted err(19), got {:#x}",
                res[0].1
            );
            assert_eq!(res[0].2, 0, "err35: *srcSizePtr must be 0");
            assert_eq!(
                res[0].4, 1,
                "err35: dStage must be dstage_storeFrameHeader (1), free() returned {}",
                res[0].4
            );
        }
    }
}

/// ERRORS row 36 — a `LZ4F_headerSize` failure inside `LZ4F_getFrameInfo` is
/// forwarded verbatim with `*srcSizePtr = 0`.
#[test]
fn err_36_getFrameInfo_forwards_headerSize_errors() {
    unsafe {
        let f = tiny_valid_frame();
        // srcSize < 5 -> err(12) from lz4frame.c:1450
        for n in 0..5usize {
            let (ret, ss, _) = diff_get_frame_info(&format!("err36 n={n}"), &f, n);
            assert_eq!(ret, err(12), "err36: n={n} expected err(12), got {ret:#x}");
            assert_eq!(ss, 0);
        }
        // unknown magic -> err(13) from lz4frame.c:1459
        let mut bad = f.clone();
        bad[3] = 0x19;
        let (ret, ss, _) = diff_get_frame_info("err36 bad magic", &bad, bad.len());
        assert_eq!(ret, err(13), "err36: expected err(13), got {ret:#x}");
        assert_eq!(ss, 0);
        // src == NULL -> err(15) from lz4frame.c:1446
        let (c, r) = apis();
        let mut rets = Vec::new();
        for api in [&c, &r] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut fi = LZ4F_frameInfo_t::default();
            let mut ss = 19usize;
            let ret = (api.get_frame_info)(dctx, &mut fi, ptr::null(), &mut ss);
            let fr = (api.free)(dctx);
            rets.push((ret, ss, fr));
        }
        assert_eq!(
            (rets[0].0 as isize, rets[0].1, rets[0].2 as isize),
            (rets[1].0 as isize, rets[1].1, rets[1].2 as isize),
            "err36: getFrameInfo(src=NULL) C={:?} Rust={:?}",
            rets[0],
            rets[1]
        );
        assert_eq!(
            rets[0].0,
            err(15),
            "err36: getFrameInfo(src=NULL) must forward srcPtr_wrong err(15), got {:#x}",
            rets[0].0
        );
        assert_eq!(rets[0].1, 0, "err36: *srcSizePtr must be 0");
    }
}

/// ERRORS row 37 — `*srcSizePtr < hSize` inside `LZ4F_getFrameInfo` -> err(12),
/// `*srcSizePtr = 0`.
#[test]
fn err_37_getFrameInfo_not_enough_input_for_header() {
    unsafe {
        // hSize 7 with 5 and 6 bytes available
        let f = tiny_valid_frame();
        for n in [5usize, 6] {
            let (ret, ss, _) = diff_get_frame_info(&format!("err37 hsize=7 n={n}"), &f, n);
            assert_eq!(ret, err(12), "err37: n={n} expected err(12), got {ret:#x}");
            assert_eq!(ss, 0);
        }
        // hSize 11 / 15 / 19 with every insufficient srcSize >= 5
        for (csf, dif, hsz) in [(false, true, 11usize), (true, false, 15), (true, true, 19)] {
            let mut f = craft_header(
                flg(false, false, csf, false, dif),
                bd(4),
                if csf { Some(0) } else { None },
                if dif { Some(9) } else { None },
            );
            f.extend_from_slice(&0u32.to_le_bytes());
            for n in 5..hsz {
                let (ret, ss, _) =
                    diff_get_frame_info(&format!("err37 hsize={hsz} n={n}"), &f, n);
                assert_eq!(ret, err(12), "err37: hsize={hsz} n={n} expected err(12)");
                assert_eq!(ss, 0);
            }
            // exactly hsz bytes succeeds and consumes exactly the header
            let (ret, ss, fi) =
                diff_get_frame_info(&format!("err37 hsize={hsz} n={hsz}"), &f, hsz);
            assert_eq!(ret, 4, "err37: success must return the BHSize hint 4, got {ret:#x}");
            assert_eq!(ss, hsz, "err37: must consume exactly the header");
            assert_eq!(fi.blockSizeID, 4);
        }
    }
}

/// ERRORS row 38 — a `LZ4F_decodeHeader` failure inside `LZ4F_getFrameInfo` is
/// forwarded, with `*srcSizePtr = 0`.
#[test]
fn err_38_getFrameInfo_forwards_decodeHeader_errors() {
    unsafe {
        let cases: Vec<(String, Vec<u8>, usize)> = vec![
            ("FLG bit1".into(), craft_header(0x42, bd(4), None, None), err(8)),
            ("bad version".into(), craft_header(0x80, bd(4), None, None), err(6)),
            ("BD bit7".into(), craft_header(0x40, 0xC0, None, None), err(8)),
            ("blockSizeID 3".into(), craft_header(0x40, bd(3), None, None), err(2)),
            ("BD low nibble".into(), craft_header(0x40, bd(4) | 3, None, None), err(8)),
            (
                "bad header checksum".into(),
                {
                    let mut v = craft_header(0x40, bd(4), None, None);
                    v[6] ^= 0xFF;
                    v
                },
                err(17),
            ),
        ];
        for (name, mut f, expect) in cases {
            f.resize(64, 0);
            for n in [7usize, 19, 64] {
                let (ret, ss, _) = diff_get_frame_info(&format!("err38 [{name}] n={n}"), &f, n);
                assert_eq!(ret, expect, "err38 [{name}] n={n}: expected {expect:#x}, got {ret:#x}");
                assert_eq!(ss, 0, "err38 [{name}]: *srcSizePtr must be 0");
            }
        }
        // and the success case: a cached frameInfo after decoding started
        let mut rng = Rng::new(38);
        let payload = gen(&mut rng, Shape::TextLike, 5000);
        let mut p = prefs(LZ4F_MAX256KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 5000, 0);
        p.frameInfo.dictID = 0x0BAD_F00D;
        let f = c_frame(&payload, Some(&p));
        assert_eq!(parse_frame(&f).hsize, 19);
        let (c, r) = apis();
        let mut res: Vec<(usize, usize, LZ4F_frameInfo_t, usize, usize, LZ4F_frameInfo_t)> = Vec::new();
        for api in [&c, &r] {
            let mut dctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
            let mut fi1 = LZ4F_frameInfo_t::default();
            let mut s1 = f.len();
            let g1 = (api.get_frame_info)(dctx, &mut fi1, f.as_ptr() as *const c_void, &mut s1);
            // now decoding has started: the second call reads no input at all
            let mut fi2 = LZ4F_frameInfo_t::default();
            let mut s2 = f.len();
            let g2 = (api.get_frame_info)(
                dctx,
                &mut fi2,
                f.as_ptr().add(s1) as *const c_void,
                &mut s2,
            );
            let _ = (api.free)(dctx);
            res.push((g1, s1, fi1, g2, s2, fi2));
        }
        assert_eq!(res[0].0 as isize, res[1].0 as isize, "err38: first getFrameInfo differs");
        assert_eq!(res[0].1, res[1].1, "err38: first *srcSizePtr differs");
        assert_eq!(res[0].2, res[1].2, "err38: first frameInfo differs");
        assert_eq!(res[0].3 as isize, res[1].3 as isize, "err38: second getFrameInfo differs");
        assert_eq!(res[0].4, res[1].4, "err38: second *srcSizePtr differs");
        assert_eq!(res[0].5, res[1].5, "err38: second frameInfo differs");
        assert_eq!(res[0].1, 19, "err38: header consumed");
        assert_eq!(res[0].0, 4, "err38: first call returns the BHSize hint");
        assert_eq!(res[0].4, 0, "err38: the cached path consumes no input");
        assert_eq!(res[0].2, res[0].5, "err38: the cached frameInfo must match");
        assert_eq!(res[0].2.contentSize, 5000);
        // note: LZ4F_optimalBSID downgrades the stored BD byte for small inputs,
        // so read the expected blockSizeID straight out of the frame header
        assert_eq!(res[0].2.blockSizeID, ((f[5] >> 4) & 7) as c_uint);
        assert_eq!(res[0].2.blockMode, LZ4F_BLOCK_INDEPENDENT);
        assert_eq!(res[0].2.contentChecksumFlag, 1);
        assert_eq!(res[0].2.blockChecksumFlag, 1);
        assert_eq!(res[0].2.frameType, LZ4F_FRAME);
        assert_eq!(res[0].2.dictID, 0x0BAD_F00D);
    }
}

// ---------------------------------------------------------------------------
// Error-state recovery helper
// ---------------------------------------------------------------------------

/// After an error, keep using the SAME dctx: repeat the failing call, then
/// `LZ4F_resetDecompressionContext` and decode a good frame. Every observable
/// must match between the two libraries.
#[track_caller]
unsafe fn diff_error_recovery(
    ctx: &str,
    bad: &[u8],
    expect: usize,
    good: &[u8],
    payload: &[u8],
) {
    let (c, r) = apis();
    struct Rec {
        first: (usize, usize, usize),
        again: (usize, usize, usize),
        after_reset: Vec<Step>,
        out: Vec<u8>,
        free_ret: usize,
        dstage_after_error: usize,
    }
    let mut recs: Vec<Rec> = Vec::new();
    let cap = payload.len() + 5 * 1024 * 1024;
    for api in [&c, &r] {
        let mut dctx: *mut c_void = ptr::null_mut();
        assert_eq!((api.create)(&mut dctx, LZ4F_VERSION), 0);
        let mut big = vec![FILL; cap + GUARD];
        let one = |dctx: *mut c_void, src: &[u8], buf: &mut Vec<u8>| {
            let mut ds = cap;
            let mut ss = src.len();
            let ret = (api.decompress)(
                dctx,
                buf.as_mut_ptr() as *mut c_void,
                &mut ds,
                src.as_ptr() as *const c_void,
                &mut ss,
                ptr::null(),
            );
            (ret, ss, ds)
        };
        let first = one(dctx, bad, &mut big);
        let again = one(dctx, bad, &mut big);
        // observe the dStage the error left behind, without destroying the ctx:
        // a second dctx driven identically is freed to read its stage back
        let dstage_after_error = {
            let mut probe: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut probe, LZ4F_VERSION), 0);
            let mut b2 = vec![FILL; cap + GUARD];
            let _ = one(probe, bad, &mut b2);
            (api.free)(probe)
        };
        (api.reset)(dctx);
        let mut steps = Vec::new();
        let mut out = Vec::new();
        {
            let mut src_off = 0usize;
            let mut out_off = 0usize;
            let mut buf = vec![FILL; payload.len() + 64 * 1024 + GUARD];
            loop {
                let mut ds = buf.len() - GUARD - out_off;
                let mut ss = good.len() - src_off;
                let ret = (api.decompress)(
                    dctx,
                    buf.as_mut_ptr().add(out_off) as *mut c_void,
                    &mut ds,
                    good.as_ptr().add(src_off) as *const c_void,
                    &mut ss,
                    ptr::null(),
                );
                steps.push(Step { hint: ret, src: ss, dst: ds, req_src: 0, req_dst: 0 });
                src_off += ss;
                out_off += ds;
                if is_err_range(ret) || ret == 0 || (ss == 0 && ds == 0) {
                    break;
                }
            }
            out.extend_from_slice(&buf[..out_off]);
        }
        let free_ret = (api.free)(dctx);
        recs.push(Rec { first, again, after_reset: steps, out, free_ret, dstage_after_error });
    }
    let (a, b) = (&recs[0], &recs[1]);
    assert_eq!(
        (a.first.0 as isize, a.first.1, a.first.2),
        (b.first.0 as isize, b.first.1, b.first.2),
        "{ctx}: first (failing) call differs: C={:?} Rust={:?}",
        a.first,
        b.first
    );
    assert_eq!(
        a.first.0, expect,
        "{ctx}: expected error {:#x}, got {:#x}",
        expect, a.first.0
    );
    assert_eq!(
        (a.again.0 as isize, a.again.1, a.again.2),
        (b.again.0 as isize, b.again.1, b.again.2),
        "{ctx}: repeated call on the errored dctx differs: C={:?} Rust={:?}",
        a.again,
        b.again
    );
    assert_eq!(
        a.dstage_after_error as isize, b.dstage_after_error as isize,
        "{ctx}: dStage left behind by the error differs (C={} Rust={})",
        a.dstage_after_error as isize, b.dstage_after_error as isize
    );
    assert_eq!(
        a.after_reset, b.after_reset,
        "{ctx}: post-reset decode traces differ"
    );
    same_full_buffers(&format!("{ctx}: post-reset output"), &a.out, &b.out);
    assert_eq!(
        a.free_ret as isize, b.free_ret as isize,
        "{ctx}: free() after recovery differs"
    );
    assert_eq!(a.out, payload, "{ctx}: the dctx did not recover after reset");
    assert_eq!(a.free_ret, 0, "{ctx}: a completed frame must leave dStage 0");
}

/// ERRORS row 39 — `LZ4F_decodeHeader` error on the `dstage_getFrameHeader`
/// fast path (>= maxFHSize available), lz4frame.c:1650-1651.
#[test]
fn err_39_decompress_getFrameHeader_fast_path_forwards_error() {
    unsafe {
        let mut rng = Rng::new(39);
        let payload = gen(&mut rng, Shape::TextLike, 30_000);
        let good = c_frame(&payload, None);
        let cases: Vec<(String, Vec<u8>, usize)> = vec![
            ("bad magic".into(), {
                let mut v = tiny_valid_frame();
                v[0] = 0x05;
                v
            }, err(13)),
            ("FLG bit1".into(), craft_header(0x42, bd(4), None, None), err(8)),
            ("bad version".into(), craft_header(0xC0, bd(4), None, None), err(6)),
            ("BD bit7".into(), craft_header(0x40, 0xF0, None, None), err(8)),
            ("blockSizeID 1".into(), craft_header(0x40, bd(1), None, None), err(2)),
            ("BD nibble".into(), craft_header(0x40, bd(6) | 5, None, None), err(8)),
            ("bad HC".into(), {
                let mut v = craft_header(0x40, bd(4), None, None);
                v[6] ^= 0x33;
                v
            }, err(17)),
        ];
        for (name, mut f, expect) in cases {
            // >= 19 bytes so the shortcut in dstage_getFrameHeader is taken
            f.resize(f.len().max(19) + 40, 0x5A);
            let pl = plan(4096);
            let t = diff_err(&format!("err39 [{name}]"), &f, &pl, expect);
            assert_eq!(t.steps.len(), 1, "err39 [{name}]: the error must come on the first call");
            assert_eq!(t.steps[0].src, 0, "err39 [{name}]: *srcSizePtr must be 0");
            assert_eq!(t.steps[0].dst, 0, "err39 [{name}]: *dstSizePtr must be 0");
            // the fast path fails before dStage is touched -> free() returns 0
            assert_eq!(
                t.free_ret, 0,
                "err39 [{name}]: dStage should still be dstage_getFrameHeader"
            );
            diff_error_recovery(&format!("err39 [{name}] recovery"), &f, expect, &good, &payload);
        }
    }
}

/// ERRORS row 40 — `LZ4F_decodeHeader` error after buffering the header in
/// `dstage_storeFrameHeader`, lz4frame.c:1673.
#[test]
fn err_40_decompress_storeFrameHeader_forwards_error() {
    unsafe {
        let mut rng = Rng::new(40);
        let payload = gen(&mut rng, Shape::Compressible, 12_000);
        let good = c_frame(&payload, None);
        let cases: Vec<(String, Vec<u8>, usize)> = vec![
            ("bad magic".into(), {
                let mut v = tiny_valid_frame();
                v[3] = 0x19;
                v
            }, err(13)),
            ("FLG bit1".into(), craft_header(0x42, bd(4), None, None), err(8)),
            ("bad version".into(), craft_header(0x00, bd(4), None, None), err(6)),
            ("blockSizeID 0".into(), craft_header(0x40, bd(0), None, None), err(2)),
            ("bad HC".into(), {
                let mut v = craft_header(0x40, bd(5), None, None);
                v[6] = v[6].wrapping_add(1);
                v
            }, err(17)),
        ];
        for (name, f, expect) in cases {
            // exactly 7 bytes of header: never enough for the fast path, so the
            // error is produced by the buffered call
            let exact = f[..7.min(f.len())].to_vec();
            for &chunk in &[1usize, 2, 3, 6, 7] {
                let mut pl = plan(4096);
                pl.src_sizes = vec![chunk];
                let t = diff_err(
                    &format!("err40 [{name}] chunk={chunk}"),
                    &exact,
                    &pl,
                    expect,
                );
                let last = t.steps.last().unwrap();
                assert_eq!(last.dst, 0, "err40 [{name}]: *dstSizePtr must be 0");
                // the buffered path bumped dStage to dstage_storeFrameHeader (1)
                // before failing
                assert_eq!(
                    t.free_ret, 1,
                    "err40 [{name}] chunk={chunk}: dStage should be dstage_storeFrameHeader"
                );
            }
            diff_error_recovery(
                &format!("err40 [{name}] recovery"),
                &exact,
                expect,
                &good,
                &payload,
            );
        }
        // an 11 / 15 / 19-byte header is buffered in two rounds (minFHSize then
        // the full size); a bad checksum must still be reported
        for (csf, dif) in [(false, true), (true, false), (true, true)] {
            let mut f = craft_header(
                flg(false, false, csf, false, dif),
                bd(4),
                if csf { Some(0) } else { None },
                if dif { Some(3) } else { None },
            );
            let last = f.len() - 1;
            f[last] ^= 0x77;
            for &chunk in &[1usize, 5, 8] {
                let mut pl = plan(4096);
                pl.src_sizes = vec![chunk];
                diff_err(
                    &format!("err40 long header csf={csf} dif={dif} chunk={chunk}"),
                    &f,
                    &pl,
                    err(17),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// A failing allocator, so the internal-allocation error paths are reachable
// through LZ4F_createDecompressionContext_advanced's LZ4F_CustomMem hook.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

#[repr(C)]
struct AllocState {
    count: usize,
    /// 1-based index of the allocation that must fail (0 = never fail)
    fail_at: usize,
}

unsafe extern "C" fn tst_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    let st = opaque as *mut AllocState;
    (*st).count += 1;
    if (*st).count == (*st).fail_at {
        return ptr::null_mut();
    }
    malloc(size)
}

unsafe extern "C" fn tst_free(_opaque: *mut c_void, p: *mut c_void) {
    free(p);
}

fn failing_cmem(st: &mut AllocState) -> CustomMem {
    CustomMem {
        alloc: Some(tst_alloc),
        calloc: None, // -> LZ4F_calloc uses customAlloc + memset
        free: Some(tst_free),
        opaque: st as *mut AllocState as *mut c_void,
    }
}

/// Common body for ERRORS rows 41 and 42: force allocation number `fail_at`
/// (2 = `dctx->tmpIn`, 3 = `dctx->tmpOutBuffer`) to fail inside `dstage_init`,
/// then retry with a working allocator and check the context recovers.
#[track_caller]
unsafe fn alloc_failure_case(ctx: &str, fail_at: usize, frame: &[u8], hsize: usize, payload: &[u8]) {
    let (c, r) = apis();
    let mut recs: Vec<(usize, usize, usize, usize, Vec<u8>, usize, usize)> = Vec::new();
    for api in [&c, &r] {
        let mut st = AllocState { count: 0, fail_at };
        let cm = failing_cmem(&mut st);
        let dctx = (api.create_adv)(cm, LZ4F_VERSION);
        assert!(!dctx.is_null(), "{}: create_advanced unexpectedly failed", api.tag);
        let cap = payload.len() + 8 * 1024 * 1024;
        let mut buf = vec![FILL; cap + GUARD];
        // 1st call: the header is decoded, then dstage_init's allocation fails
        let mut ds = cap;
        let mut ss = frame.len();
        let r1 = (api.decompress)(
            dctx,
            buf.as_mut_ptr() as *mut c_void,
            &mut ds,
            frame.as_ptr() as *const c_void,
            &mut ss,
            ptr::null(),
        );
        let (s1, d1) = (ss, ds);
        // 2nd call: the allocator now succeeds, and the frame body decodes.
        // The header was already consumed internally even though *srcSizePtr
        // reported 0, so resume from `hsize`.
        let mut ds2 = cap;
        let mut ss2 = frame.len() - hsize;
        let r2 = (api.decompress)(
            dctx,
            buf.as_mut_ptr() as *mut c_void,
            &mut ds2,
            frame.as_ptr().add(hsize) as *const c_void,
            &mut ss2,
            ptr::null(),
        );
        let out = buf[..ds2].to_vec();
        let fr = (api.free)(dctx);
        recs.push((r1, s1, d1, r2, out, ss2, fr));
    }
    let (a, b) = (&recs[0], &recs[1]);
    assert_eq!(
        (a.0 as isize, a.1, a.2),
        (b.0 as isize, b.1, b.2),
        "{ctx}: the failing call differs: C=({},{},{}) Rust=({},{},{})",
        a.0 as isize, a.1, a.2, b.0 as isize, b.1, b.2
    );
    assert_eq!(
        a.0,
        err(9),
        "{ctx}: expected LZ4F_ERROR_allocation_failed err(9), got {:#x}",
        a.0
    );
    assert_eq!(a.1, 0, "{ctx}: *srcSizePtr must be 0 on failure");
    assert_eq!(a.2, 0, "{ctx}: *dstSizePtr must be 0 on failure");
    assert_eq!(
        (a.3 as isize, a.5, a.6 as isize),
        (b.3 as isize, b.5, b.6 as isize),
        "{ctx}: the retry differs: C=(ret={},src={},free={}) Rust=(ret={},src={},free={})",
        a.3 as isize, a.5, a.6 as isize, b.3 as isize, b.5, b.6 as isize
    );
    same_full_buffers(&format!("{ctx}: retry output"), &a.4, &b.4);
    assert_eq!(a.3, 0, "{ctx}: the retry should complete the frame, got {:#x}", a.3);
    assert_eq!(a.4, payload, "{ctx}: the retry decoded the wrong content");
    assert_eq!(a.6, 0, "{ctx}: a completed frame leaves dStage 0");
}

/// ERRORS row 41 — `LZ4F_malloc(maxBlockSize + BFSize)` for `dctx->tmpIn`
/// returns NULL inside `dstage_init` (lz4frame.c:1686) -> err(9).
/// Forced through the public `LZ4F_CustomMem` hook of
/// `LZ4F_createDecompressionContext_advanced`: allocation #1 is the dctx itself,
/// #2 is `tmpIn`, #3 is `tmpOutBuffer`.
#[test]
fn err_41_dstage_init_tmpIn_allocation_failed() {
    unsafe {
        let mut rng = Rng::new(41);
        let payload = gen(&mut rng, Shape::TextLike, 30_000);
        for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for bsid in [LZ4F_MAX64KB, LZ4F_MAX4MB] {
                let p = prefs(bsid, bm, 1, 1, 0, 0);
                let frame = c_frame(&payload, Some(&p));
                let hsize = parse_frame(&frame).hsize;
                alloc_failure_case(
                    &format!("err41 bm={bm} bsid={bsid}"),
                    2,
                    &frame,
                    hsize,
                    &payload,
                );
            }
        }
        // allocation #1 (the dctx itself) failing makes
        // LZ4F_createDecompressionContext_advanced return NULL in both libraries
        let (c, r) = apis();
        for api in [&c, &r] {
            let mut st = AllocState { count: 0, fail_at: 1 };
            let cm = failing_cmem(&mut st);
            let d = (api.create_adv)(cm, LZ4F_VERSION);
            assert!(d.is_null(), "{}: create_advanced must return NULL", api.tag);
        }
    }
}

/// ERRORS row 42 — `LZ4F_malloc(bufferNeeded)` for `dctx->tmpOutBuffer` returns
/// NULL inside `dstage_init` (lz4frame.c:1689) -> err(9).
#[test]
fn err_42_dstage_init_tmpOutBuffer_allocation_failed() {
    unsafe {
        let mut rng = Rng::new(42);
        let payload = gen(&mut rng, Shape::Compressible, 45_000);
        for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for bsid in [LZ4F_MAX64KB, LZ4F_MAX1MB] {
                let p = prefs(bsid, bm, 0, 0, 0, 0);
                let frame = c_frame(&payload, Some(&p));
                let hsize = parse_frame(&frame).hsize;
                alloc_failure_case(
                    &format!("err42 bm={bm} bsid={bsid}"),
                    3,
                    &frame,
                    hsize,
                    &payload,
                );
            }
        }
    }
}

/// ERRORS row 43 — `nextCBlockSize > dctx->maxBlockSize` (lz4frame.c:1737-1738)
/// -> err(2).
#[test]
fn err_43_block_size_exceeds_frame_max_block_size() {
    unsafe {
        let mut rng = Rng::new(43);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        for bsid in [4u8, 5, 6, 7] {
            let mbs = block_size_of(bsid as c_uint);
            for &(delta, stored) in &[
                (1u32, false),
                (1, true),
                (2, false),
                (mbs as u32, false),
                (0x7FFF_FFFF - mbs as u32, false),
                (0x7FFF_FFFF - mbs as u32, true),
            ] {
                let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
                let mut bh = mbs as u32 + delta;
                if stored {
                    bh |= 0x8000_0000;
                }
                f.extend_from_slice(&bh.to_le_bytes());
                f.resize(f.len() + 32, 0x11);
                let name = format!("err43 bsid={bsid} delta={delta} stored={stored}");
                diff_err(&name, &f, &plan(4096), err(2));
                let mut pl = plan(4096);
                pl.src_sizes = vec![1];
                diff_err(&format!("{name} bytewise"), &f, &pl, err(2));
                // dStage is dstage_getBlockHeader (3) / dstage_storeBlockHeader (4)
                // when the error hits
                diff_error_recovery(&format!("{name} recovery"), &f, err(2), &good, &payload);
            }
        }
    }
}

/// ERRORS row 44 — stored (uncompressed) block checksum mismatch at
/// `dstage_getBlockChecksum` (lz4frame.c:1825-1829) -> err(7).
#[test]
fn err_44_stored_block_checksum_invalid() {
    unsafe {
        let mut rng = Rng::new(44);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        for &n in &[0usize, 1, 100, 65_536] {
            let body = gen(&mut rng, Shape::Incompressible, n);
            for k in 0..4usize {
                let mut f = craft_header(flg(false, true, false, false, false), bd(4), None, None);
                push_stored_block(&mut f, &body, true);
                let crc_off = f.len() - 4;
                f[crc_off + k] ^= 1 << k;
                f.extend_from_slice(&0u32.to_le_bytes());
                let name = format!("err44 n={n} crcbyte={k}");
                for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 7usize), (3, 5)] {
                    let mut pl = plan(n + 70_000);
                    pl.src_sizes = vec![s];
                    pl.dst_caps = vec![d];
                    diff_err(&format!("{name} src={s} dst={d}"), &f, &pl, err(7));
                }
                // skipChecksums = 1 makes the same frame decode
                let mut pl = plan(n + 70_000);
                pl.opts = opts(0, 1);
                pl.opts_null = false;
                diff_ok(&format!("{name} skipChecksums=1"), &f, &pl, &body);
                diff_error_recovery(&format!("{name} recovery"), &f, err(7), &good, &payload);
            }
        }
        // and on a real C frame of incompressible data
        let inc = gen(&mut rng, Shape::Incompressible, 130_000);
        let f = c_frame(&inc, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 1, 0, 0)));
        let lay = parse_frame(&f);
        let si = lay.blocks.iter().position(|b| b.stored).unwrap();
        let mut bad = f.clone();
        bad[lay.blocks[si].crc.unwrap()] ^= 0x10;
        diff_err("err44 real frame", &bad, &plan(inc.len() + 70_000), err(7));
    }
}

/// ERRORS row 45 — trailing CRC of a *compressed* block mismatches
/// (lz4frame.c:1878) -> err(7). Note this check is deliberately NOT guarded by
/// `dctx->skipChecksum`.
#[test]
fn err_45_compressed_block_checksum_invalid() {
    unsafe {
        let mut rng = Rng::new(45);
        let payload = gen(&mut rng, Shape::TextLike, 150_000);
        let good = c_frame(&payload, None);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 1, 0, 0);
        let f = c_frame(&payload, Some(&p));
        let lay = parse_frame(&f);
        assert!(lay.blocks.len() >= 2);
        for bi in 0..lay.blocks.len() {
            assert!(!lay.blocks[bi].stored);
            for k in 0..4usize {
                let mut bad = f.clone();
                bad[lay.blocks[bi].crc.unwrap() + k] ^= 1 << ((k + 1) % 8);
                let name = format!("err45 block={bi} crcbyte={k}");
                for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 33usize)] {
                    let mut pl = plan(payload.len() + 70_000);
                    pl.src_sizes = vec![s];
                    pl.dst_caps = vec![d];
                    diff_err(&format!("{name} src={s} dst={d}"), &bad, &pl, err(7));
                }
                // skipChecksums has no effect here
                let mut pl = plan(payload.len() + 70_000);
                pl.opts = opts(0, 1);
                pl.opts_null = false;
                diff_err(&format!("{name} skipChecksums=1"), &bad, &pl, err(7));
                if bi == 0 && k == 0 {
                    diff_error_recovery(
                        &format!("{name} recovery"),
                        &bad,
                        err(7),
                        &good,
                        &payload,
                    );
                }
            }
        }
        // a hand-crafted single compressed block with a wrong CRC
        let one = c_frame(&payload[..500], Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 1, 0, 0)));
        let l1 = parse_frame(&one);
        assert_eq!(l1.blocks.len(), 1);
        let mut bad = one.clone();
        bad[l1.blocks[0].crc.unwrap() + 2] ^= 0xFF;
        diff_err("err45 single block", &bad, &plan(70_000), err(7));
    }
}

/// ERRORS row 46 — `LZ4_decompress_safe_usingDict() < 0` when decoding straight
/// into `dstBuffer` (lz4frame.c:1905) -> err(16).
#[test]
fn err_46_decompression_failed_into_dst() {
    unsafe {
        let mut rng = Rng::new(46);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        for bsid in [4u8, 5, 6] {
            let mbs = block_size_of(bsid as c_uint);
            let blk = overflowing_block_payload(mbs);
            let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
            f.extend_from_slice(&(blk.len() as u32).to_le_bytes());
            f.extend_from_slice(&blk);
            f.extend_from_slice(&0u32.to_le_bytes());
            // dst capacity >= maxBlockSize -> the direct-into-dst branch
            let mut pl = plan(mbs * 2 + 4096);
            pl.dst_caps = vec![mbs + 1];
            let t = diff_err(&format!("err46 bsid={bsid} direct"), &f, &pl, err(16));
            assert_eq!(t.steps.last().unwrap().dst, 0);
            let mut pl = plan(mbs * 2 + 4096);
            pl.dst_caps = vec![mbs + 1];
            pl.src_sizes = vec![1];
            diff_err(&format!("err46 bsid={bsid} direct bytewise"), &f, &pl, err(16));
            diff_error_recovery(
                &format!("err46 bsid={bsid} recovery"),
                &f,
                err(16),
                &good,
                &payload,
            );
        }
        // real compressed blocks corrupted in place: whatever the C library
        // decides, the Rust library must agree
        let big = gen(&mut rng, Shape::TextLike, 80_000);
        let f = c_frame(&big, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0, 0, 0)));
        let lay = parse_frame(&f);
        for off in [0usize, 1, 2, 3, 40, 400] {
            if off >= lay.blocks[0].len {
                continue;
            }
            let mut bad = f.clone();
            bad[lay.blocks[0].payload + off] ^= 0xFF;
            let mut pl = plan(big.len() + 70_000);
            pl.dst_caps = vec![ALL];
            diff(&format!("err46 corrupted payload off={off}"), &bad, &pl);
        }
    }
}

/// ERRORS row 47 — `LZ4_decompress_safe_usingDict() < 0` when decoding into
/// `tmpOut` (lz4frame.c:1950) -> err(16).
#[test]
fn err_47_decompression_failed_into_tmpout() {
    unsafe {
        let mut rng = Rng::new(47);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        for bsid in [4u8, 5, 6] {
            let mbs = block_size_of(bsid as c_uint);
            let blk = overflowing_block_payload(mbs);
            for bm in [false, true] {
                let mut f = craft_header(flg(bm, false, false, false, false), bd(bsid), None, None);
                f.extend_from_slice(&(blk.len() as u32).to_le_bytes());
                f.extend_from_slice(&blk);
                f.extend_from_slice(&0u32.to_le_bytes());
                // dst capacity < maxBlockSize -> the decode-into-tmpOut branch
                for &cap in &[1usize, 64, 4096] {
                    let mut pl = plan(mbs * 2 + 4096);
                    pl.dst_caps = vec![cap];
                    diff_err(
                        &format!("err47 bsid={bsid} independent={bm} dstcap={cap}"),
                        &f,
                        &pl,
                        err(16),
                    );
                }
                let mut pl = plan(mbs * 2 + 4096);
                pl.dst_caps = vec![100];
                pl.src_sizes = vec![1];
                diff_err(
                    &format!("err47 bsid={bsid} independent={bm} bytewise"),
                    &f,
                    &pl,
                    err(16),
                );
            }
            diff_error_recovery(
                &format!("err47 bsid={bsid} recovery"),
                &{
                    let mut f = craft_header(flg(false, false, false, false, false), bd(bsid), None, None);
                    f.extend_from_slice(&(blk.len() as u32).to_le_bytes());
                    f.extend_from_slice(&blk);
                    f.extend_from_slice(&0u32.to_le_bytes());
                    f
                },
                err(16),
                &good,
                &payload,
            );
        }
    }
}

/// ERRORS row 48 — endMark reached while `dctx->frameRemainingSize != 0`
/// (lz4frame.c:1984) -> err(14).
#[test]
fn err_48_getSuffix_frame_size_wrong() {
    unsafe {
        let mut rng = Rng::new(48);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        let body = gen(&mut rng, Shape::Incompressible, 1234);
        for declared in [1u64, 1233, 1235, 5000, u64::MAX, 0xFFFF_FFFF_FFFF] {
            for ccrc in [false, true] {
                let mut f = craft_header(
                    flg(false, false, true, ccrc, false),
                    bd(4),
                    Some(declared),
                    None,
                );
                push_stored_block(&mut f, &body, false);
                f.extend_from_slice(&0u32.to_le_bytes());
                if ccrc {
                    f.extend_from_slice(&xxh32(&body).to_le_bytes());
                }
                let name = format!("err48 declared={declared} ccrc={ccrc}");
                for &s in &[ALL, 1usize, 13] {
                    let mut pl = plan(70_000);
                    pl.src_sizes = vec![s];
                    diff_err(&format!("{name} src={s}"), &f, &pl, err(14));
                }
                // the frameSize check happens before the content checksum check,
                // so skipChecksums cannot hide it
                let mut pl = plan(70_000);
                pl.opts = opts(0, 1);
                pl.opts_null = false;
                diff_err(&format!("{name} skipChecksums=1"), &f, &pl, err(14));
                diff_error_recovery(&format!("{name} recovery"), &f, err(14), &good, &payload);
            }
        }
        // contentSize == 0 is "unknown" and disables the check entirely
        let mut f = craft_header(flg(false, false, true, false, false), bd(4), Some(0), None);
        push_stored_block(&mut f, &body, false);
        f.extend_from_slice(&0u32.to_le_bytes());
        diff_ok("err48 declared=0 (unknown)", &f, &plan(70_000), &body);
    }
}

/// ERRORS row 49 — frame content checksum mismatch (lz4frame.c:2018-2021)
/// -> err(18), unless `skipChecksum` is set.
#[test]
fn err_49_content_checksum_invalid() {
    unsafe {
        let mut rng = Rng::new(49);
        let payload = gen(&mut rng, Shape::TextLike, 9000);
        let good = c_frame(&payload, None);
        let body = gen(&mut rng, Shape::Incompressible, 777);
        for k in 0..4usize {
            let mut f = craft_header(flg(false, false, false, true, false), bd(4), None, None);
            push_stored_block(&mut f, &body, false);
            f.extend_from_slice(&0u32.to_le_bytes());
            let mut crc = xxh32(&body).to_le_bytes();
            crc[k] ^= 1 << k;
            f.extend_from_slice(&crc);
            let name = format!("err49 crcbyte={k}");
            for &(s, d) in &[(ALL, ALL), (1usize, ALL), (ALL, 3usize), (2, 2)] {
                let mut pl = plan(70_000);
                pl.src_sizes = vec![s];
                pl.dst_caps = vec![d];
                diff_err(&format!("{name} src={s} dst={d}"), &f, &pl, err(18));
            }
            let mut pl = plan(70_000);
            pl.opts = opts(0, 1);
            pl.opts_null = false;
            diff_ok(&format!("{name} skipChecksums=1"), &f, &pl, &body);
            diff_error_recovery(&format!("{name} recovery"), &f, err(18), &good, &payload);
        }
        // an empty frame with a content checksum
        let mut f = craft_header(flg(false, false, false, true, false), bd(4), None, None);
        f.extend_from_slice(&0u32.to_le_bytes());
        let mut crc = xxh32(&[]).to_le_bytes();
        diff_ok("err49 empty frame correct crc", &{ let mut v = f.clone(); v.extend_from_slice(&crc); v }, &plan(64), &[]);
        crc[0] ^= 1;
        f.extend_from_slice(&crc);
        diff_err("err49 empty frame bad crc", &f, &plan(64), err(18));

        // a real C frame whose content checksum is corrupted, and the same frame
        // whose *payload* is corrupted so the recomputed checksum diverges
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let f = c_frame(&payload, Some(&p));
        let lay = parse_frame(&f);
        let mut bad = f.clone();
        bad[lay.content_crc.unwrap() + 1] ^= 0x08;
        diff_err("err49 real frame corrupt crc", &bad, &plan(70_000), err(18));
    }
}

/// ERRORS row 50 — `LZ4F_freeDecompressionContext` returns the raw `dStage`
/// (lz4frame.c:1313-1324). The exact numeric value is asserted for a fresh, a
/// mid-frame and a completed context.
#[test]
fn err_50_freeDecompressionContext_returns_raw_dStage() {
    unsafe {
        let mut rng = Rng::new(50);

        // dstage_getFrameHeader == 0 : a brand new context
        {
            let (c, r) = apis();
            for api in [&c, &r] {
                let mut d: *mut c_void = ptr::null_mut();
                assert_eq!((api.create)(&mut d, LZ4F_VERSION), 0);
                assert_eq!(
                    (api.free)(d),
                    0,
                    "{}: a fresh dctx must free with dStage 0",
                    api.tag
                );
            }
            // free(NULL) is accepted, like free()
            let (c, r) = apis();
            assert_eq!((c.free)(ptr::null_mut()), 0);
            assert_eq!((r.free)(ptr::null_mut()), 0);

            // LZ4F_createDecompressionContext(NULL, ..) -> parameter_null err(21)
            // (ERRORS row 22; the preceding assert() in lz4frame.c is compiled
            // out, so the RETURN_ERROR_IF is the observable behaviour)
            for v in [LZ4F_VERSION, 0, 99, 101] {
                let a = (c.create)(ptr::null_mut(), v);
                let b = (r.create)(ptr::null_mut(), v);
                assert_eq!(
                    a as isize, b as isize,
                    "err50: createDecompressionContext(NULL,{v}) C={a:#x} Rust={b:#x}"
                );
                assert_eq!(
                    a,
                    err(21),
                    "err50: createDecompressionContext(NULL,{v}) must be err(21), got {a:#x}"
                );
            }
            // a non-default version number is stored verbatim and does not
            // change any observable behaviour
            for v in [0u32, 1, 99, LZ4F_VERSION, 1000] {
                for api in [&c, &r] {
                    let mut d: *mut c_void = ptr::null_mut();
                    let ret = (api.create)(&mut d, v);
                    assert_eq!(ret, 0, "{}: create(version={v}) failed {ret:#x}", api.tag);
                    assert!(!d.is_null());
                    assert_eq!((api.free)(d), 0);
                }
            }
        }

        let payload = gen(&mut rng, Shape::TextLike, 200_000);
        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let frame = c_frame(&payload, Some(&p));
        let lay = parse_frame(&frame);
        assert!(lay.blocks.len() >= 2 && !lay.blocks[0].stored);

        // (name, frame slice, plan, expected dStage)
        let mut cases: Vec<(String, Vec<u8>, Plan, usize)> = Vec::new();

        // dstage_storeFrameHeader == 1 : a partially buffered header
        for n in 1..7usize {
            let mut pl = plan(4096);
            pl.src_sizes = vec![ALL];
            cases.push((format!("storeFrameHeader n={n}"), frame[..n].to_vec(), pl, 1));
        }

        // dstage_storeBlockHeader == 4 : the header is complete, no block header yet
        for n in 0..4usize {
            let pl = plan(300_000);
            cases.push((
                format!("storeBlockHeader +{n}"),
                frame[..lay.hsize + n].to_vec(),
                pl,
                4,
            ));
        }

        // dstage_storeCBlock == 8 : part of a compressed block is buffered
        for n in [1usize, 2, 50] {
            let pl = plan(300_000);
            cases.push((
                format!("storeCBlock +{n}"),
                frame[..lay.blocks[0].payload + n].to_vec(),
                pl,
                8,
            ));
        }

        // dstage_flushOut == 9 : the block was decoded into tmpOut but only
        // partially flushed (one single call, tiny dst)
        {
            let mut pl = plan(300_000);
            pl.dst_caps = vec![16];
            pl.max_calls = 1;
            cases.push(("flushOut".into(), frame.clone(), pl, 9));
        }

        // dstage_storeSuffix == 11 : part of the content checksum is buffered
        for n in [1usize, 2, 3] {
            let pl = plan(300_000);
            cases.push((
                format!("storeSuffix +{n}"),
                frame[..lay.content_crc.unwrap() + n].to_vec(),
                pl,
                11,
            ));
        }

        // dstage_copyDirect == 5 : a stored block is being copied out
        {
            let body = gen(&mut rng, Shape::Incompressible, 5000);
            let mut f = craft_header(flg(false, false, false, false, false), bd(4), None, None);
            push_stored_block(&mut f, &body, false);
            f.extend_from_slice(&0u32.to_le_bytes());
            let mut pl = plan(70_000);
            pl.dst_caps = vec![10];
            pl.max_calls = 1;
            cases.push(("copyDirect".into(), f, pl, 5));
        }

        // dstage_getBlockChecksum == 6 : the stored block is done, its checksum
        // has not arrived yet
        {
            let body = gen(&mut rng, Shape::Incompressible, 300);
            let mut f = craft_header(flg(false, true, false, false, false), bd(4), None, None);
            push_stored_block(&mut f, &body, true);
            let cut = f.len() - 4;
            let pl = plan(70_000);
            cases.push(("getBlockChecksum".into(), f[..cut].to_vec(), pl, 6));
        }

        // dstage_storeSFrameSize == 13 and dstage_skipSkippable == 14
        {
            let sk = skippable(0x184D2A50, &gen(&mut rng, Shape::Degenerate, 4000));
            let pl = plan(4096);
            cases.push(("storeSFrameSize".into(), sk[..7].to_vec(), pl, 13));
            let pl = plan(4096);
            cases.push(("skipSkippable".into(), sk[..100].to_vec(), pl, 14));
        }

        // dstage_init == 2 : reachable only when dstage_init itself fails, which
        // is covered by err_41 / err_42; here just the completed-frame case.
        {
            let pl = plan(300_000);
            cases.push(("completed frame".into(), frame.clone(), pl, 0));
        }

        for (name, f, pl, expect) in &cases {
            let t = diff(&format!("err50 [{name}]"), f, pl);
            assert_eq!(
                t.free_ret, *expect,
                "err50 [{name}]: expected dStage {expect}, free() returned {} (steps {:?})",
                t.free_ret,
                &t.steps[t.steps.len().saturating_sub(3)..]
            );
        }

        // after LZ4F_resetDecompressionContext the stage is 0 again
        let (c, r) = apis();
        for api in [&c, &r] {
            let mut d: *mut c_void = ptr::null_mut();
            assert_eq!((api.create)(&mut d, LZ4F_VERSION), 0);
            let mut ds = 0usize;
            let mut ss = 3usize;
            let _ = (api.decompress)(
                d,
                ptr::null_mut(),
                &mut ds,
                frame.as_ptr() as *const c_void,
                &mut ss,
                ptr::null(),
            );
            (api.reset)(d);
            assert_eq!((api.free)(d), 0, "{}: reset must return dStage to 0", api.tag);
        }
    }
}

/// ERRORS row 51 — `ctxTypeID_to_size()`'s fall-through `return 0`
/// (lz4frame.c:676-682) is **UNREACHABLE**: `ctxTypeID` is only ever assigned
/// the literals `1` (fast) or `2` (HC) in `LZ4F_initStream` /
/// `LZ4F_compressBegin_internal`, and the helper is called only from
/// `LZ4F_initStream`, which passes exactly those two values. There is no public
/// entry point that can set a third value, so the "not enough space allocated"
/// branch it feeds cannot be exercised.
/// The closest reachable observable is asserted instead: frames produced on
/// either side of the `LZ4HC_CLEVEL_MIN` boundary — including a single reused
/// `LZ4F_cctx` that crosses it, which is what forces the `ctxTypeID` switch and
/// the realloc — decode identically in both libraries.
#[test]
fn err_51_ctxTypeID_to_size_default_branch_is_unreachable() {
    unsafe {
        let mut rng = Rng::new(51);
        let payload = gen(&mut rng, Shape::TextLike, 120_000);
        let e = cenc();
        // one C cctx reused across the fast <-> HC boundary in both directions
        for levels in [[1i32, 9], [9, 1], [0, 12], [12, 0], [2, 1], [-4, 10]] {
            let mut cctx: *mut c_void = ptr::null_mut();
            assert_eq!((e.create_cctx)(&mut cctx, LZ4F_VERSION), 0);
            for (i, &lvl) in levels.iter().enumerate() {
                let mut p = prefs(
                    if i == 0 { LZ4F_MAX64KB } else { LZ4F_MAX1MB },
                    LZ4F_BLOCK_LINKED,
                    1,
                    1,
                    0,
                    lvl,
                );
                p.autoFlush = 1;
                let cap = (e.bound)(payload.len(), &p) + 1024;
                let mut out = vec![0u8; cap];
                let n = (e.begin)(cctx, out.as_mut_ptr() as *mut c_void, cap, &p);
                assert!(!is_err_range(n), "compressBegin failed {n:#x}");
                let mut off = n;
                let n = (e.update)(
                    cctx,
                    out.as_mut_ptr().add(off) as *mut c_void,
                    cap - off,
                    payload.as_ptr() as *const c_void,
                    payload.len(),
                    ptr::null(),
                );
                assert!(!is_err_range(n));
                off += n;
                let n = (e.end)(cctx, out.as_mut_ptr().add(off) as *mut c_void, cap - off, ptr::null());
                assert!(!is_err_range(n));
                off += n;
                out.truncate(off);
                let pl = plan(payload.len() + 2 * 1024 * 1024);
                diff_ok(
                    &format!("err51 reused cctx levels={levels:?} step={i} lvl={lvl}"),
                    &out,
                    &pl,
                    &payload,
                );
            }
            assert_eq!((e.free_cctx)(cctx), 0);
        }
    }
}

/// ERRORS row 52 — `LZ4F_makeBlock` with `cSize == 0` or `cSize >= srcSize`
/// (lz4frame.c:896) is not an error: the block is rewritten as stored with the
/// `LZ4F_BLOCKUNCOMPRESSED_FLAG`. Verified on the wire and through the decoder's
/// `dstage_copyDirect` path.
#[test]
fn err_52_incompressible_block_is_stored() {
    unsafe {
        let mut rng = Rng::new(52);
        for &n in &[1usize, 2, 16, 64, 1000, 65_536, 200_000] {
            let payload = gen(&mut rng, Shape::Incompressible, n);
            for bcrc in [0u32, 1] {
                let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, bcrc, 0, 0);
                let f = c_frame(&payload, Some(&p));
                let lay = parse_frame(&f);
                assert!(
                    lay.blocks.iter().all(|b| b.stored),
                    "err52: n={n} bcrc={bcrc}: every block of incompressible data must carry \
                     LZ4F_BLOCKUNCOMPRESSED_FLAG"
                );
                let total: usize = lay.blocks.iter().map(|b| b.len).sum();
                assert_eq!(total, n, "err52: stored blocks must hold the payload verbatim");
                // a stored block holds the source bytes verbatim
                let mut at = 0usize;
                for b in &lay.blocks {
                    assert_eq!(
                        &f[b.payload..b.payload + b.len],
                        &payload[at..at + b.len],
                        "err52: stored block at {} is not verbatim",
                        b.hdr
                    );
                    at += b.len;
                }
                let mut pl = plan(n + 70_000);
                diff_ok(&format!("err52 n={n} bcrc={bcrc}"), &f, &pl, &payload);
                pl.dst_caps = vec![13];
                pl.src_sizes = vec![7];
                diff_ok(&format!("err52 n={n} bcrc={bcrc} chunked"), &f, &pl, &payload);
            }
        }
        // a payload that compresses to exactly its own size or larger
        for n in 1..24usize {
            let payload = gen(&mut rng, Shape::Incompressible, n);
            let f = c_frame(&payload, None);
            let lay = parse_frame(&f);
            assert!(lay.blocks[0].stored, "err52: a {n}-byte block must be stored");
            diff_ok(&format!("err52 tiny n={n}"), &f, &plan(70_000), &payload);
        }
    }
}

/// ERRORS row 53 — `prefs.frameInfo.blockSizeID == 0` (lz4frame.c:740-741) is
/// not an error: it is silently replaced by `LZ4F_BLOCKSIZEID_DEFAULT`
/// (`LZ4F_max64KB` = 4), which is what lands in the BD byte of the frame.
#[test]
fn err_53_blockSizeID_zero_becomes_default() {
    unsafe {
        let mut rng = Rng::new(53);
        for &n in &[0usize, 100, 70_000, 300_000] {
            let payload = gen(&mut rng, Shape::TextLike, n);
            let mut p = prefs(LZ4F_DEFAULT, LZ4F_BLOCK_LINKED, 1, 1, 0, 0);
            p.autoFlush = 1;
            let f = c_frame(&payload, Some(&p));
            assert_eq!(
                (f[5] >> 4) & 7,
                4,
                "err53: blockSizeID 0 must be stored as LZ4F_max64KB (4)"
            );
            diff_ok(&format!("err53 n={n}"), &f, &plan(n + 70_000), &payload);
            // the decoder reports the substituted value through getFrameInfo
            let (ret, ss, fi) = diff_get_frame_info(&format!("err53 gfi n={n}"), &f, f.len());
            assert!(!is_err_range(ret));
            assert_eq!(fi.blockSizeID, LZ4F_MAX64KB, "err53: frameInfo.blockSizeID");
            assert_eq!(ss, parse_frame(&f).hsize);
        }
        // NULL preferences take the same default path
        let payload = gen(&mut rng, Shape::TextLike, 130_000);
        let f = c_frame(&payload, None);
        assert_eq!((f[5] >> 4) & 7, 4);
        diff_ok("err53 prefs=NULL", &f, &plan(200_000), &payload);
    }
}

/// ERRORS row 54 — `compressionLevel < LZ4HC_CLEVEL_MIN (2)` selects the fast
/// codec and a negative level becomes `acceleration = -level + 1`
/// (lz4frame.c:955-961, :911). Never an error; the frames must decode
/// identically in both libraries at every level.
#[test]
fn err_54_compression_level_is_reinterpreted_never_rejected() {
    unsafe {
        let mut rng = Rng::new(54);
        let payload = gen(&mut rng, Shape::TextLike, 180_000);
        for lvl in [
            i32::MIN + 1,
            -1000,
            -100,
            -10,
            -3,
            -1,
            0,
            1,
            2,
            3,
            9,
            10,
            11,
            12,
            13,
            100,
            i32::MAX,
        ] {
            for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                let p = prefs(LZ4F_MAX64KB, bm, 1, 1, 0, lvl);
                let f = c_frame(&payload, Some(&p));
                let mut pl = plan(payload.len() + 70_000);
                diff_ok(&format!("err54 level={lvl} bm={bm}"), &f, &pl, &payload);
                pl.dst_caps = vec![911];
                pl.src_sizes = vec![1777];
                diff_ok(&format!("err54 level={lvl} bm={bm} chunked"), &f, &pl, &payload);
            }
        }
    }
}

/// ERRORS row 55 — `dctx->dictSize > 1 GB` (lz4frame.c:1897-1901, :1938-1942):
/// not an error, the dictionary is silently truncated to its last 64 KB (an
/// int-overflow guard). Exercised with a real 1 GB + 1 byte dictionary buffer
/// whose last 64 KB is the dictionary the frame was actually compressed with,
/// on both the decode-into-dst and the decode-into-tmpOut paths.
#[test]
fn err_55_dictSize_above_1GB_is_truncated_to_last_64KB() {
    unsafe {
        let mut rng = Rng::new(55);
        let dict = gen(&mut rng, Shape::TextLike, 64 * 1024);
        // a payload whose head repeats the dictionary, so the dictionary matters
        let mut payload = Vec::new();
        payload.extend_from_slice(&dict[dict.len() - 30_000..]);
        payload.extend_from_slice(&gen(&mut rng, Shape::TextLike, 60_000));

        let p = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let needy = c_frame_dict(&payload, Some(&dict), Some(&p), 0);
        // sanity: the frame really needs the dictionary
        {
            let t = diff("err55 without dict", &needy, &plan(payload.len() + 70_000));
            assert!(
                is_err_range(t.last_hint()) || t.out != payload,
                "err55: the reference frame does not actually need the dictionary"
            );
        }

        // 1 GB + 1 bytes: `1 GB` in lz4frame.c is 1 << 30, so this is strictly
        // greater and triggers the truncation branch. The buffer is zero-filled
        // (lazily mapped) apart from the trailing 64 KB.
        const ONE_GB: usize = 1usize << 30;
        let mut huge = vec![0u8; ONE_GB + 1];
        let tail = huge.len() - dict.len();
        huge[tail..].copy_from_slice(&dict);
        assert!(huge.len() > ONE_GB);

        // decode-into-dst path (dst capacity >= maxBlockSize)
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((huge.as_ptr() as usize, huge.len()));
        diff_ok("err55 1GB+1 dict, direct dst", &needy, &pl, &payload);
        // decode-into-tmpOut path (dst capacity < maxBlockSize)
        let mut pl = plan(payload.len() + 70_000);
        pl.dst_caps = vec![4096];
        pl.dict = Some((huge.as_ptr() as usize, huge.len()));
        diff_ok("err55 1GB+1 dict, tmpOut", &needy, &pl, &payload);
        // and chunked input, so the dictionary is re-consulted on many calls
        let mut pl = plan(payload.len() + 70_000);
        pl.src_sizes = vec![257];
        pl.dst_caps = vec![1021];
        pl.dict = Some((huge.as_ptr() as usize, huge.len()));
        diff_ok("err55 1GB+1 dict, chunked", &needy, &pl, &payload);

        // exactly 1 GB is NOT truncated (the condition is strictly greater), and
        // with the dictionary at the tail the decode must still be correct
        let mut exact = vec![0u8; ONE_GB];
        let t2 = exact.len() - dict.len();
        exact[t2..].copy_from_slice(&dict);
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((exact.as_ptr() as usize, exact.len()));
        diff_ok("err55 exactly 1GB dict", &needy, &pl, &payload);
        drop(exact);
        drop(huge);

        // blockIndependent frames go through the same guard
        let pi = prefs(LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 0, 0);
        let needy_i = c_frame_dict(&payload, Some(&dict), Some(&pi), 0);
        let mut huge2 = vec![0u8; ONE_GB + 12345];
        let t3 = huge2.len() - dict.len();
        huge2[t3..].copy_from_slice(&dict);
        let mut pl = plan(payload.len() + 70_000);
        pl.dict = Some((huge2.as_ptr() as usize, huge2.len()));
        diff_ok("err55 1GB dict blockIndependent", &needy_i, &pl, &payload);
    }
}

// ===========================================================================
// Randomized differential fuzzing of the decoder
// ===========================================================================

/// Valid frames are generated across several configurations, then mutated with
/// random byte flips / truncations / splices. The identical bytes are fed to
/// both decoders under a random call schedule, and the whole sequence of return
/// values, out-params and produced bytes must match exactly.
#[test]
fn fuzz_mutated_frames_are_decoded_identically() {
    unsafe {
        let mut rng = Rng::new(0xF00D_BEEF);

        // --- base corpus --------------------------------------------------
        struct Base {
            name: String,
            frame: Vec<u8>,
            payload: Vec<u8>,
        }
        let mut bases: Vec<Base> = Vec::new();
        {
            let cfgs: Vec<(String, LZ4F_preferences_t, Shape, usize)> = vec![
                (
                    "linked/64K/no-crc".into(),
                    prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 0, 0, 0, 0),
                    Shape::TextLike,
                    9000,
                ),
                (
                    "linked/64K/both-crc/contentSize".into(),
                    prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 9000, 0),
                    Shape::TextLike,
                    9000,
                ),
                (
                    "independent/64K/block-crc".into(),
                    prefs(LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 0, 1, 0, 9),
                    Shape::Compressible,
                    12_000,
                ),
                (
                    "linked/256K/content-crc".into(),
                    prefs(LZ4F_MAX256KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 12),
                    Shape::Periodic,
                    11_000,
                ),
                (
                    "stored blocks/both-crc".into(),
                    prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 0, 0),
                    Shape::Incompressible,
                    8000,
                ),
                (
                    "multi-block linked/64K".into(),
                    prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 1),
                    Shape::TextLike,
                    140_000,
                ),
            ];
            for (name, mut p, shape, n) in cfgs {
                let payload = gen(&mut rng, shape, n);
                if p.frameInfo.contentSize != 0 {
                    p.frameInfo.contentSize = n as u64;
                }
                let mut frame = c_frame(&payload, Some(&p));
                if name.starts_with("multi-block") {
                    // keep the corpus small: only the first ~12 KB of the frame
                    frame.truncate(12_000);
                }
                bases.push(Base { name, frame, payload });
            }
            // a skippable frame and a two-frame concatenation
            let sk = skippable(0x184D2A57, &gen(&mut rng, Shape::Degenerate, 3000));
            bases.push(Base { name: "skippable".into(), frame: sk, payload: Vec::new() });
            let a = gen(&mut rng, Shape::TextLike, 4000);
            let b = gen(&mut rng, Shape::Compressible, 4000);
            let mut two = c_frame(&a, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_LINKED, 1, 1, 0, 0)));
            two.extend_from_slice(&c_frame(&b, Some(&prefs(LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 0, 1, 0, 0))));
            let mut ab = a.clone();
            ab.extend_from_slice(&b);
            bases.push(Base { name: "two frames".into(), frame: two, payload: ab });
        }

        // every unmutated base decodes cleanly first (except the truncated one)
        for bs in &bases {
            let mut pl = plan(bs.payload.len() + 512 * 1024);
            pl.continue_frames = true;
            let t = diff(&format!("fuzz base [{}]", bs.name), &bs.frame, &pl);
            if !bs.name.starts_with("multi-block") {
                assert_eq!(t.last_hint(), 0, "fuzz: base [{}] did not decode", bs.name);
            }
        }

        // --- mutation + differential loop ---------------------------------
        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut mutated_bytes_changed = 0usize;
        let mut identical_to_base = 0usize;
        let mut recovered_payload = 0usize;
        const ITERS: usize = 60_000;
        for iter in 0..ITERS {
            let bi = rng.below(bases.len());
            let base = &bases[bi];
            let mut f = base.frame.clone();

            let nmut = rng.range(1, 3);
            for _ in 0..nmut {
                if f.is_empty() {
                    break;
                }
                match rng.below(7) {
                    // single byte flip
                    0 | 1 => {
                        let i = rng.below(f.len());
                        f[i] ^= 1u8 << rng.below(8);
                    }
                    // single byte replacement
                    2 => {
                        let i = rng.below(f.len());
                        f[i] = rng.byte();
                    }
                    // truncation
                    3 => {
                        let n = rng.below(f.len() + 1);
                        f.truncate(n);
                    }
                    // delete a run
                    4 => {
                        let i = rng.below(f.len());
                        let n = rng.range(1, (f.len() - i).min(64));
                        f.drain(i..i + n);
                    }
                    // insert random bytes
                    5 => {
                        let i = rng.below(f.len() + 1);
                        let n = rng.range(1, 32);
                        let ins: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
                        for (k, b) in ins.into_iter().enumerate() {
                            f.insert(i + k, b);
                        }
                    }
                    // splice a run in from another base frame
                    _ => {
                        let other = &bases[rng.below(bases.len())].frame;
                        if other.is_empty() || f.is_empty() {
                            continue;
                        }
                        let n = rng.range(1, other.len().min(128));
                        let so = rng.below(other.len() - n + 1);
                        let d = rng.below(f.len());
                        let m = n.min(f.len() - d);
                        f[d..d + m].copy_from_slice(&other[so..so + m]);
                    }
                }
            }
            if f == base.frame {
                identical_to_base += 1;
            } else {
                mutated_bytes_changed += 1;
            }

            // random call schedule
            let mut pl = plan(512 * 1024);
            let mut sizes = Vec::new();
            for _ in 0..rng.range(1, 3) {
                sizes.push(match rng.below(6) {
                    0 => 1usize,
                    1 => rng.range(1, 8),
                    2 => rng.range(1, 64),
                    3 => rng.range(1, 1024),
                    4 => rng.range(1, 20_000),
                    _ => ALL,
                });
            }
            pl.src_sizes = sizes;
            let mut caps = Vec::new();
            for _ in 0..rng.range(1, 3) {
                caps.push(match rng.below(5) {
                    0 => rng.range(1, 16),
                    1 => rng.range(1, 1024),
                    2 => rng.range(1, 100_000),
                    3 => 0usize,
                    _ => ALL,
                });
            }
            if caps.iter().all(|&c| c == 0) {
                caps.push(ALL);
            }
            pl.dst_caps = caps;
            pl.continue_frames = rng.below(2) == 0;
            if rng.below(2) == 0 {
                pl.opts = opts(rng.below(2) as c_uint, rng.below(2) as c_uint);
                pl.opts_null = false;
            }
            if rng.below(8) == 0 {
                pl.dst_mode = DstMode::Fresh;
                pl.max_out = 200_000;
                pl.dst_caps = vec![rng.range(1, 20_000)];
                pl.opts = opts(0, rng.below(2) as c_uint);
                pl.opts_null = false;
            }

            let ctx = format!("fuzz iter={iter} base=[{}] len={}", base.name, f.len());
            let t = diff(&ctx, &f, &pl);

            let last = t.last_hint();
            let key = if is_err_range(last) {
                format!("err({})", (0usize).wrapping_sub(last))
            } else if last == 0 {
                if t.out == base.payload {
                    recovered_payload += 1;
                    "ok/original".to_string()
                } else {
                    "ok/other".to_string()
                }
            } else {
                "incomplete".to_string()
            };
            *tally.entry(key).or_insert(0) += 1;
        }

        println!("\n=== fuzz outcome distribution over {ITERS} mutated frames ===");
        for (k, v) in &tally {
            println!("  {k:<16} {v}");
        }
        println!(
            "  (mutations that changed bytes: {mutated_bytes_changed}, no-op mutations: \
             {identical_to_base}, frames that still decoded to the original payload: \
             {recovered_payload})"
        );

        // non-vacuity: the corpus must have exercised many distinct outcomes,
        // including several distinct decoder error codes
        let nerr = tally.keys().filter(|k| k.starts_with("err(")).count();
        assert!(
            tally.len() >= 6,
            "fuzz: only {} distinct outcomes observed, the corpus is too weak: {:?}",
            tally.len(),
            tally
        );
        assert!(
            nerr >= 5,
            "fuzz: only {nerr} distinct error codes observed: {:?}",
            tally
        );
        for want in ["err(2)", "err(16)", "err(17)", "incomplete"] {
            assert!(
                tally.contains_key(want),
                "fuzz: expected to observe {want}, got {:?}",
                tally
            );
        }
        assert!(
            tally.get("ok/original").copied().unwrap_or(0) > 0
                || tally.get("ok/other").copied().unwrap_or(0) > 0,
            "fuzz: no mutated frame ever decoded successfully: {:?}",
            tally
        );
        assert!(
            mutated_bytes_changed * 10 > ITERS * 9,
            "fuzz: too many no-op mutations ({identical_to_base})"
        );
    }
}

/// A second fuzzing stage: semi-structured *random* byte streams (not derived
/// from any valid frame), decoded with and without random dictionaries through
/// `LZ4F_decompress_usingDict`, and also pushed through `LZ4F_headerSize` and
/// `LZ4F_getFrameInfo`.
#[test]
fn fuzz_random_byte_streams_and_dictionaries() {
    unsafe {
        let mut rng = Rng::new(0x5EED_C0DE);
        let dicts: Vec<Vec<u8>> = vec![
            Vec::new(),
            gen(&mut rng, Shape::TextLike, 1),
            gen(&mut rng, Shape::TextLike, 4096),
            gen(&mut rng, Shape::TextLike, 65_535),
            gen(&mut rng, Shape::TextLike, 65_536),
            gen(&mut rng, Shape::TextLike, 65_537),
            gen(&mut rng, Shape::Compressible, 300_000),
        ];

        let mut tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut hs_tally: BTreeMap<String, usize> = BTreeMap::new();
        let mut gfi_tally: BTreeMap<String, usize> = BTreeMap::new();
        const ITERS: usize = 30_000;
        for iter in 0..ITERS {
            // --- build a semi-structured random stream ---------------------
            let mut f: Vec<u8> = Vec::new();
            match rng.below(4) {
                0 => f.extend_from_slice(&MAGIC), // valid plain magic
                1 => f.extend_from_slice(&(0x184D2A50u32 + rng.below(16) as u32).to_le_bytes()),
                2 => f.extend_from_slice(&rng.next_u32().to_le_bytes()),
                _ => {
                    // a near-miss magic
                    let mut m = MAGIC;
                    m[rng.below(4)] ^= 1u8 << rng.below(8);
                    f.extend_from_slice(&m);
                }
            }
            // FLG / BD: sometimes valid, sometimes not
            if rng.below(2) == 0 {
                f.push(flg(
                    rng.below(2) == 0,
                    rng.below(2) == 0,
                    rng.below(2) == 0,
                    rng.below(2) == 0,
                    rng.below(2) == 0,
                ));
                f.push(bd(4 + rng.below(4) as u8));
            } else {
                f.push(rng.byte());
                f.push(rng.byte());
            }
            // optional contentSize / dictID fields, driven by the FLG we wrote
            let csf = (f[4] >> 3) & 1 == 1;
            let dif = f[4] & 1 == 1;
            if csf {
                let v = match rng.below(3) {
                    0 => 0u64,
                    1 => rng.range(0, 5000) as u64,
                    _ => rng.next_u64(),
                };
                f.extend_from_slice(&v.to_le_bytes());
            }
            if dif {
                f.extend_from_slice(&rng.next_u32().to_le_bytes());
            }
            // header checksum: correct half the time
            let hc = header_checksum(&f[4..]);
            f.push(if rng.below(2) == 0 { hc } else { rng.byte() });
            // body: a mix of plausible block headers and raw noise
            let nblocks = rng.range(0, 6);
            for _ in 0..nblocks {
                let bh = match rng.below(5) {
                    0 => 0u32,
                    1 => rng.range(0, 300) as u32,
                    2 => (rng.range(0, 300) as u32) | 0x8000_0000,
                    3 => rng.next_u32(),
                    _ => 0x8000_0000 | rng.range(0, 65_600) as u32,
                };
                f.extend_from_slice(&bh.to_le_bytes());
                let n = rng.range(0, 400);
                let sh = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                f.extend_from_slice(&gen(&mut rng, sh, n));
            }
            if rng.below(2) == 0 {
                f.extend_from_slice(&0u32.to_le_bytes());
                if rng.below(2) == 0 {
                    f.extend_from_slice(&rng.next_u32().to_le_bytes());
                }
            }

            // --- random call schedule + random dictionary -----------------
            let mut pl = plan(512 * 1024);
            pl.src_sizes = vec![match rng.below(5) {
                0 => 1usize,
                1 => rng.range(1, 16),
                2 => rng.range(1, 500),
                3 => rng.range(1, 100_000),
                _ => ALL,
            }];
            pl.dst_caps = vec![match rng.below(4) {
                0 => rng.range(1, 8),
                1 => rng.range(1, 2048),
                2 => rng.range(1, 200_000),
                _ => ALL,
            }];
            pl.continue_frames = rng.below(2) == 0;
            if rng.below(2) == 0 {
                pl.opts = opts(rng.below(2) as c_uint, rng.below(2) as c_uint);
                pl.opts_null = false;
            }
            if rng.below(2) == 0 {
                let d = &dicts[rng.below(dicts.len())];
                pl.dict = Some((d.as_ptr() as usize, d.len()));
                pl.dict_from_call = rng.below(2);
            }

            let ctx = format!("fuzzB iter={iter} len={}", f.len());
            let t = diff(&ctx, &f, &pl);
            let last = t.last_hint();
            let key = if is_err_range(last) {
                format!("err({})", (0usize).wrapping_sub(last))
            } else if last == 0 {
                format!("ok/{}", if t.out.is_empty() { "empty" } else { "data" })
            } else {
                "incomplete".to_string()
            };
            *tally.entry(key).or_insert(0) += 1;

            // --- LZ4F_headerSize / LZ4F_getFrameInfo on the same bytes ----
            let n = rng.below(f.len() + 1);
            let hs = diff_header_size(&format!("{ctx} headerSize n={n}"), Some(&f), n);
            let hk = if is_err_range(hs) {
                format!("err({})", (0usize).wrapping_sub(hs))
            } else {
                format!("size {hs}")
            };
            *hs_tally.entry(hk).or_insert(0) += 1;
            let (g, _gs, _fi) = diff_get_frame_info(&format!("{ctx} getFrameInfo n={n}"), &f, n);
            let gk = if is_err_range(g) {
                format!("err({})", (0usize).wrapping_sub(g))
            } else {
                format!("hint {g}")
            };
            *gfi_tally.entry(gk).or_insert(0) += 1;
        }

        println!("\n=== fuzz stage B: {ITERS} random streams, LZ4F_decompress outcomes ===");
        for (k, v) in &tally {
            println!("  {k:<16} {v}");
        }
        println!("=== fuzz stage B: LZ4F_headerSize outcomes ===");
        for (k, v) in &hs_tally {
            println!("  {k:<16} {v}");
        }
        println!("=== fuzz stage B: LZ4F_getFrameInfo outcomes ===");
        for (k, v) in &gfi_tally {
            println!("  {k:<16} {v}");
        }

        let nerr = tally.keys().filter(|k| k.starts_with("err(")).count();
        assert!(
            tally.len() >= 6 && nerr >= 5,
            "fuzzB: distribution too narrow: {tally:?}"
        );
        for want in ["err(13)", "err(17)", "incomplete"] {
            assert!(tally.contains_key(want), "fuzzB: expected {want} in {tally:?}");
        }
        assert!(
            hs_tally.len() >= 4,
            "fuzzB: LZ4F_headerSize distribution too narrow: {hs_tally:?}"
        );
        assert!(
            gfi_tally.len() >= 4,
            "fuzzB: LZ4F_getFrameInfo distribution too narrow: {gfi_tally:?}"
        );
    }
}
