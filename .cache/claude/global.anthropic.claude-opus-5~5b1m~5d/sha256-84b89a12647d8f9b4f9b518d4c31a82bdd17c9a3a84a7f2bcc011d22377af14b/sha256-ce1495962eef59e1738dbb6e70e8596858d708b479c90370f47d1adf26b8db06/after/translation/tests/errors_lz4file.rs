//! Per-row differential tests for ERRORS.md rows 160..184 — the
//! `## lz4file.c (stdio file API)` section.
//!
//! EXACTLY ONE `#[test] fn err_NNN_...()` per ERRORS.md row, so that the audit
//! trail from an ERRORS.md row to the test that pins it is mechanical.
//!
//! Rules obeyed here:
//!   * every call goes through a `.so` export via `libloading` — no Rust
//!     function is ever called directly;
//!   * the C library and the Rust library each get their OWN scratch file,
//!     their OWN `FILE*` and their OWN opaque `LZ4_readFile_t` /
//!     `LZ4_writeFile_t` handle, always opened and closed by the SAME library;
//!   * only return values and file bytes are compared.
//!
//! Where a row is not reachable through the public lz4file API (the allocation
//! failures — lz4file.c calls libc `calloc`/`malloc` directly and offers no
//! custom-allocator hook — and the `size_t` "negative fread" branch), the test
//! documents WHY in a comment and asserts the closest reachable behaviour
//! instead of pretending to force the branch.
#![allow(unused_imports, non_snake_case, dead_code)]

mod common;
use common::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// libc stdio (linked from the process's own libc)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn fseek(f: *mut c_void, off: c_long, whence: c_int) -> c_int;
    fn ftell(f: *mut c_void) -> c_long;
    fn fread(p: *mut c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn fwrite(p: *const c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn setvbuf(f: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut c_void;
}

const SEEK_SET: c_int = 0;
const SEEK_END: c_int = 2;
const IONBF: c_int = 2;

/// `LZ4F_HEADER_SIZE_MAX` — the fixed amount `LZ4F_readOpen` demands up front
/// and the buffer size it hands to `LZ4F_compressBegin`.
const HEADER_SIZE_MAX: usize = 19;

// ---------------------------------------------------------------------------
// lz4file + lz4frame FFI signatures
// ---------------------------------------------------------------------------

type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type FnClose = unsafe extern "C" fn(*mut c_void) -> usize;

type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompressBeginF =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnCompressBoundF = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;

#[derive(Copy, Clone)]
struct FileApi {
    tag: &'static str,
    wopen: FnWriteOpen,
    write: FnWrite,
    wclose: FnClose,
    ropen: FnReadOpen,
    read: FnRead,
    rclose: FnClose,
    create_cctx: FnCreateCtx,
    free_cctx: FnFreeCtx,
    create_dctx: FnCreateCtx,
    free_dctx: FnFreeCtx,
    compress_begin: FnCompressBeginF,
    compress_bound: FnCompressBoundF,
    error_name: FnGetErrorName,
    xxh32: FnXXH32,
}

macro_rules! pair {
    ($l:expr, $t:ty, $n:expr) => {{
        let (a, b) = $l.sym::<$t>($n);
        (*a, *b)
    }};
}

unsafe fn apis() -> (FileApi, FileApi) {
    let l = libs();
    // Paranoia: the two libraries must really be two distinct code objects.
    {
        let (a, b) = l.sym::<FnWriteOpen>("LZ4F_writeOpen");
        assert_ne!(
            *a as usize, *b as usize,
            "harness bug: LZ4F_writeOpen resolved to the same address in both libraries"
        );
    }
    let (wo_c, wo_r) = pair!(l, FnWriteOpen, "LZ4F_writeOpen");
    let (w_c, w_r) = pair!(l, FnWrite, "LZ4F_write");
    let (wc_c, wc_r) = pair!(l, FnClose, "LZ4F_writeClose");
    let (ro_c, ro_r) = pair!(l, FnReadOpen, "LZ4F_readOpen");
    let (rd_c, rd_r) = pair!(l, FnRead, "LZ4F_read");
    let (rc_c, rc_r) = pair!(l, FnClose, "LZ4F_readClose");
    let (cc_c, cc_r) = pair!(l, FnCreateCtx, "LZ4F_createCompressionContext");
    let (fc_c, fc_r) = pair!(l, FnFreeCtx, "LZ4F_freeCompressionContext");
    let (cd_c, cd_r) = pair!(l, FnCreateCtx, "LZ4F_createDecompressionContext");
    let (fd_c, fd_r) = pair!(l, FnFreeCtx, "LZ4F_freeDecompressionContext");
    let (cb_c, cb_r) = pair!(l, FnCompressBeginF, "LZ4F_compressBegin");
    let (bd_c, bd_r) = pair!(l, FnCompressBoundF, "LZ4F_compressBound");
    let (en_c, en_r) = pair!(l, FnGetErrorName, "LZ4F_getErrorName");
    let (xx_c, xx_r) = pair!(l, FnXXH32, "LZ4_XXH32");
    (
        FileApi {
            tag: "C",
            wopen: wo_c,
            write: w_c,
            wclose: wc_c,
            ropen: ro_c,
            read: rd_c,
            rclose: rc_c,
            create_cctx: cc_c,
            free_cctx: fc_c,
            create_dctx: cd_c,
            free_dctx: fd_c,
            compress_begin: cb_c,
            compress_bound: bd_c,
            error_name: en_c,
            xxh32: xx_c,
        },
        FileApi {
            tag: "Rust",
            wopen: wo_r,
            write: w_r,
            wclose: wc_r,
            ropen: ro_r,
            read: rd_r,
            rclose: rc_r,
            create_cctx: cc_r,
            free_cctx: fc_r,
            create_dctx: cd_r,
            free_dctx: fd_r,
            compress_begin: cb_r,
            compress_bound: bd_r,
            error_name: en_r,
            xxh32: xx_r,
        },
    )
}

// ---------------------------------------------------------------------------
// Scratch files (RAII: removed on drop, unique per test, honours $TMPDIR)
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("errlz4file_{}_{}_{}.lz4", std::process::id(), tag, n));
        let _ = std::fs::remove_file(&p);
        Scratch { path: p }
    }
    fn path(&self) -> &Path {
        &self.path
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap_or_default()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// One scratch file per library, so the two implementations never share state.
fn scratch_pair(tag: &str) -> (Scratch, Scratch) {
    (Scratch::new(&format!("{tag}_c")), Scratch::new(&format!("{tag}_r")))
}

unsafe fn open_file(path: &Path, mode: &str) -> *mut c_void {
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    let cm = CString::new(mode).unwrap();
    let fp = fopen(cp.as_ptr(), cm.as_ptr());
    assert!(!fp.is_null(), "fopen({}, {mode}) failed", path.display());
    fp
}

/// Create `path` with exactly `bytes`, using raw stdio.
unsafe fn write_raw(path: &Path, bytes: &[u8]) {
    let fp = open_file(path, "wb");
    if !bytes.is_empty() {
        let n = fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), fp);
        assert_eq!(n, bytes.len(), "fwrite({}) short", path.display());
    }
    assert_eq!(fflush(fp), 0, "fflush({}) failed", path.display());
    fclose(fp);
}

/// An unbuffered `FILE*` over a fixed-capacity memory buffer: writes beyond
/// `cap` fail, which is how the `io_write` rows are forced.
struct MemFile {
    buf: Vec<u8>,
    fp: *mut c_void,
}

impl MemFile {
    unsafe fn new(cap: usize) -> MemFile {
        let mut buf = vec![0u8; cap.max(1)];
        let mode = CString::new("wb").unwrap();
        let fp = fmemopen(buf.as_mut_ptr() as *mut c_void, cap, mode.as_ptr());
        assert!(!fp.is_null(), "fmemopen(cap={cap}) failed");
        assert_eq!(setvbuf(fp, ptr::null_mut(), IONBF, 0), 0, "setvbuf(IONBF) failed");
        MemFile { buf, fp }
    }
}

impl Drop for MemFile {
    fn drop(&mut self) {
        unsafe {
            fclose(self.fp);
        }
    }
}

// ---------------------------------------------------------------------------
// Small drivers
// ---------------------------------------------------------------------------

/// `LZ4F_readOpen` + immediate `LZ4F_readClose`; checks the handle-NULLing
/// contract on the error path.
unsafe fn read_open_only(api: &FileApi, path: &Path) -> usize {
    let fp = open_file(path, "rb");
    let mut h: *mut c_void = ptr::null_mut();
    let ret = (api.ropen)(&mut h, fp);
    if ret == 0 {
        assert!(!h.is_null(), "{}: readOpen returned 0 with a NULL handle", api.tag);
        assert_eq!((api.rclose)(h), 0, "{}: readClose failed", api.tag);
    } else {
        assert!(h.is_null(), "{}: handle not nulled after readOpen error", api.tag);
    }
    fclose(fp);
    ret
}

/// Full `readOpen` / repeated `read` / `readClose` over `path`.
struct ReadOut {
    open: usize,
    reads: Vec<usize>,
    data: Vec<u8>,
    close: Option<usize>,
}

unsafe fn read_all(api: &FileApi, path: &Path, chunk: usize, cap: usize) -> ReadOut {
    assert!(chunk > 0);
    let fp = open_file(path, "rb");
    let mut h: *mut c_void = ptr::null_mut();
    let open = (api.ropen)(&mut h, fp);
    let mut reads = Vec::new();
    let mut data = Vec::new();
    let mut close = None;
    if open == 0 {
        assert!(!h.is_null(), "{}: readOpen 0 with NULL handle", api.tag);
        let mut buf = vec![0u8; chunk];
        loop {
            let ret = (api.read)(h, buf.as_mut_ptr() as *mut c_void, chunk);
            reads.push(ret);
            if is_err_range(ret) || ret == 0 {
                break;
            }
            assert!(ret <= chunk, "{}: read returned {ret} > {chunk}", api.tag);
            data.extend_from_slice(&buf[..ret]);
            if data.len() >= cap {
                break;
            }
        }
        close = Some((api.rclose)(h));
    } else {
        assert!(h.is_null(), "{}: handle not nulled after readOpen error", api.tag);
    }
    fclose(fp);
    ReadOut { open, reads, data, close }
}

/// `writeOpen` / `write` each chunk / `writeClose` on a real file.
struct WriteOut {
    open: usize,
    writes: Vec<usize>,
    close: Option<usize>,
}

unsafe fn write_all(
    api: &FileApi,
    path: &Path,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
) -> WriteOut {
    let fp = open_file(path, "wb");
    let mut h: *mut c_void = ptr::null_mut();
    let pp = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => ptr::null(),
    };
    let open = (api.wopen)(&mut h, fp, pp);
    let mut writes = Vec::new();
    let mut close = None;
    if open == 0 {
        assert!(!h.is_null(), "{}: writeOpen 0 with NULL handle", api.tag);
        for ch in chunks {
            writes.push((api.write)(h, ch.as_ptr() as *const c_void, ch.len()));
        }
        close = Some((api.wclose)(h));
    } else {
        assert!(
            h.is_null(),
            "{}: writeOpen failed ({open:#x}) but left a non-NULL handle",
            api.tag
        );
    }
    fclose(fp);
    WriteOut { open, writes, close }
}

#[track_caller]
fn same(ctx: &str, a: usize, b: usize) {
    assert_eq!(
        a as isize, b as isize,
        "{ctx}: C={a:#x} Rust={b:#x} (as LZ4F codes C={} Rust={})",
        (0usize).wrapping_sub(a) as isize,
        (0usize).wrapping_sub(b) as isize
    );
}

#[track_caller]
fn same_and_is(ctx: &str, a: usize, b: usize, expect: usize) {
    same(ctx, a, b);
    assert_eq!(
        a as isize, expect as isize,
        "{ctx}: expected {expect:#x} (LZ4F code {}), got {a:#x} (LZ4F code {})",
        (0usize).wrapping_sub(expect) as isize,
        (0usize).wrapping_sub(a) as isize
    );
}

#[track_caller]
fn assert_ok(ctx: &str, v: Option<usize>) -> usize {
    let v = v.unwrap_or_else(|| panic!("{ctx}: call was never made"));
    assert!(
        !is_err_range(v),
        "{ctx}: expected success, got {v:#x} (LZ4F code {})",
        (0usize).wrapping_sub(v)
    );
    v
}

fn prefs_with(bsid: c_uint) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p
}

/// Hand-built LZ4 frame header: magic, FLG, BD and the real header checksum
/// (`(XXH32(FLG..BD) >> 8) & 0xFF`), computed through the `.so`'s own
/// `LZ4_XXH32` export in BOTH libraries (they must agree).
unsafe fn header_crc(api_c: &FileApi, api_r: &FileApi, body: &[u8]) -> u8 {
    let hc = (api_c.xxh32)(body.as_ptr() as *const c_void, body.len(), 0);
    let hr = (api_r.xxh32)(body.as_ptr() as *const c_void, body.len(), 0);
    assert_eq!(hc, hr, "LZ4_XXH32 disagrees between the libraries");
    ((hc >> 8) & 0xFF) as u8
}

/// 7-byte header (`FLG`,`BD`, correct HC), padded with `pad` to `len` bytes so
/// that `LZ4F_readOpen`'s unconditional 19-byte `fread` succeeds.
unsafe fn valid_shaped_header(
    c: &FileApi,
    r: &FileApi,
    flg: u8,
    bd: u8,
    len: usize,
    pad: u8,
) -> Vec<u8> {
    let body = [flg, bd];
    let hc = header_crc(c, r, &body);
    let mut v = vec![0x04, 0x22, 0x4D, 0x18, flg, bd, hc];
    v.resize(len.max(7), pad);
    v
}

// ===========================================================================
// Row 160 — LZ4F_readOpen: fp == NULL or lz4fRead == NULL -> err(21)
// ===========================================================================

#[test]
fn err_160_readopen_null_params() {
    unsafe {
        let (c, r) = apis();

        // (a) fp == NULL, valid out-pointer. The out-pointer must be left
        // untouched (the NULL check precedes the calloc).
        for _ in 0..8 {
            let mut hc: *mut c_void = ptr::null_mut();
            let mut hr: *mut c_void = ptr::null_mut();
            let a = (c.ropen)(&mut hc, ptr::null_mut());
            let b = (r.ropen)(&mut hr, ptr::null_mut());
            same_and_is("row160 readOpen(fp=NULL)", a, b, err(21));
            assert!(hc.is_null() && hr.is_null(), "row160: handle written despite err(21)");
        }

        // A non-NULL sentinel out-value must survive untouched as well.
        let sentinel = 0x1234usize as *mut c_void;
        let mut hc = sentinel;
        let mut hr = sentinel;
        let a = (c.ropen)(&mut hc, ptr::null_mut());
        let b = (r.ropen)(&mut hr, ptr::null_mut());
        same_and_is("row160 readOpen(fp=NULL, sentinel)", a, b, err(21));
        assert_eq!(hc, sentinel, "row160: C overwrote the out-pointer on err(21)");
        assert_eq!(hr, sentinel, "row160: Rust overwrote the out-pointer on err(21)");

        // (b) lz4fRead == NULL with a real, valid frame behind `fp`.
        let (sc, sr) = scratch_pair("r160");
        let mut rng = Rng::new(160);
        let payload = gen(&mut rng, Shape::TextLike, 4096);
        assert_ok("row160 setup C", write_all(&c, sc.path(), None, &[&payload]).close);
        assert_ok("row160 setup Rust", write_all(&r, sr.path(), None, &[&payload]).close);
        let fpc = open_file(sc.path(), "rb");
        let fpr = open_file(sr.path(), "rb");
        let a = (c.ropen)(ptr::null_mut(), fpc);
        let b = (r.ropen)(ptr::null_mut(), fpr);
        same_and_is("row160 readOpen(handle=NULL)", a, b, err(21));
        // Nothing may have been consumed from the stream.
        assert_eq!(ftell(fpc), 0, "row160: C consumed input on err(21)");
        assert_eq!(ftell(fpr), 0, "row160: Rust consumed input on err(21)");
        fclose(fpc);
        fclose(fpr);

        // (c) both NULL.
        let a = (c.ropen)(ptr::null_mut(), ptr::null_mut());
        let b = (r.ropen)(ptr::null_mut(), ptr::null_mut());
        same_and_is("row160 readOpen(both NULL)", a, b, err(21));
    }
}

// ===========================================================================
// Row 161 — LZ4F_readOpen: calloc(1, sizeof(LZ4_readFile_t)) == NULL -> err(9)
// ===========================================================================

/// UNFORCEABLE through the public API: lz4file.c:83 calls libc `calloc`
/// directly and lz4file exposes no custom-allocator hook (unlike lz4frame's
/// `LZ4F_createCompressionContext_advanced` + `LZ4F_CustomMem`), so the
/// allocation cannot be made to fail from a test that only drives the exported
/// symbols. Interposing `malloc`/`calloc` would poison the whole test process
/// (the harness itself allocates), so this test asserts the two closest
/// reachable facts instead:
///   1. the success path of that very statement: `LZ4F_readOpen` returns 0 and
///      publishes a NON-NULL handle in both libraries (i.e. the calloc'd state
///      is what gets stored in `*lz4fRead`), and the state is really zeroed —
///      `srcBufNext`/`srcBufSize` start out consistent, which is observable as
///      a first `LZ4F_read` that returns the very first payload bytes;
///   2. the error VALUE this row would report is identical in both libraries:
///      `LZ4F_getErrorName(err(9))` returns the same C string bytes.
#[test]
fn err_161_readopen_calloc_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(161);

        for iter in 0..12 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = rng.range(1, 40_000);
            let payload = gen(&mut rng, shape, len);
            let (sc, sr) = scratch_pair("r161");
            assert_ok("row161 setup C", write_all(&c, sc.path(), None, &[&payload]).close);
            assert_ok("row161 setup Rust", write_all(&r, sr.path(), None, &[&payload]).close);

            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "rb");
                let mut h: *mut c_void = ptr::null_mut();
                let open = (api.ropen)(&mut h, fp);
                assert_eq!(
                    open, 0,
                    "row161 iter={iter} {}: readOpen should succeed, got {open:#x}",
                    api.tag
                );
                assert!(
                    !h.is_null(),
                    "row161 iter={iter} {}: readOpen returned 0 but published a NULL state",
                    api.tag
                );
                // The freshly calloc'd state must be zeroed: the first read
                // starts at the beginning of the payload.
                let want = len.min(97);
                let mut buf = vec![0u8; want];
                let got = (api.read)(h, buf.as_mut_ptr() as *mut c_void, want);
                assert_eq!(got, want, "row161 {}: first read short ({got})", api.tag);
                assert_eq!(&buf[..], &payload[..want], "row161 {}: wrong first bytes", api.tag);
                assert_eq!((api.rclose)(h), 0, "row161 {}: readClose", api.tag);
                fclose(fp);
            }
        }

        // The error value this row would have produced, compared across libs.
        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(
            cn.to_bytes(),
            rn.to_bytes(),
            "row161: LZ4F_getErrorName(err(9)) differs: C={cn:?} Rust={rn:?}"
        );
        assert_eq!(
            cn.to_bytes(),
            b"ERROR_allocation_failed",
            "row161: unexpected name for err(9): {cn:?}"
        );
    }
}

// ===========================================================================
// Row 162 — LZ4F_readOpen: LZ4F_createDecompressionContext failed -> forwarded
// ===========================================================================

/// UNFORCEABLE: the only failure mode of
/// `LZ4F_createDecompressionContext(&dctx, LZ4F_VERSION)` (lz4frame.c:1301) is
/// its internal allocation returning NULL, and lz4file passes a fixed, valid
/// version with a non-NULL out-pointer. As in row 161 there is no allocator
/// hook on this path, so the branch cannot be reached from the exported API.
/// Closest reachable assertions:
///   1. the exact call lz4file makes — `LZ4F_createDecompressionContext(&ctx,
///      LZ4F_VERSION)` — returns 0 with a non-NULL ctx in BOTH libraries, and
///      `LZ4F_freeDecompressionContext` accepts it (and NULL) identically;
///   2. `LZ4F_readOpen` really does forward a `getFrameInfo`-class error code
///      verbatim rather than rewriting it (row 164 proves the forwarding
///      mechanism that this row shares);
///   3. the error name for err(9) — what would be forwarded — matches.
#[test]
fn err_162_readopen_createdctx_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();

        // 1. the exact call site, and its free counterpart.
        for _ in 0..16 {
            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            let ra = (c.create_dctx)(&mut a, LZ4F_VERSION);
            let rb = (r.create_dctx)(&mut b, LZ4F_VERSION);
            same_and_is("row162 createDecompressionContext(LZ4F_VERSION)", ra, rb, 0);
            assert!(!a.is_null() && !b.is_null(), "row162: NULL dctx despite success");
            let fa = (c.free_dctx)(a);
            let fb = (r.free_dctx)(b);
            same("row162 freeDecompressionContext", fa, fb);
            // free(NULL) is tolerated, like free()
            let fa = (c.free_dctx)(ptr::null_mut());
            let fb = (r.free_dctx)(ptr::null_mut());
            same_and_is("row162 freeDecompressionContext(NULL)", fa, fb, 0);
        }

        // 2. the forwarding mechanism: a header error surfaces from readOpen
        //    with the *inner* code, not a generic one.
        let hdr = valid_shaped_header(&c, &r, 0x40, 0x70, 32, 0);
        let mut bad = hdr.clone();
        bad[0] = 0x05; // break the magic -> frameType_unknown err(13)
        let (sc, sr) = scratch_pair("r162");
        write_raw(sc.path(), &bad);
        write_raw(sr.path(), &bad);
        let a = read_open_only(&c, sc.path());
        let b = read_open_only(&r, sr.path());
        same_and_is("row162 readOpen forwards the inner error verbatim", a, b, err(13));

        // 3. the code this row would forward.
        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(cn.to_bytes(), rn.to_bytes(), "row162: getErrorName(err(9)) differs");
    }
}

// ===========================================================================
// Row 163 — LZ4F_readOpen: short header read -> err(23) io_read
// ===========================================================================

#[test]
fn err_163_readopen_short_header_read_is_io_read() {
    unsafe {
        let (c, r) = apis();

        // Every file length below LZ4F_HEADER_SIZE_MAX (19) must fail the
        // unconditional up-front fread, whatever the bytes are.
        let mut rng = Rng::new(163);
        for n in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 11, 15, 17, 18] {
            for trial in 0..4 {
                let data: Vec<u8> = match trial {
                    0 => (0..n).map(|i| (i as u8).wrapping_mul(7)).collect(),
                    1 => vec![0u8; n],
                    2 => {
                        // a real frame prefix, truncated
                        let full = valid_shaped_header(&c, &r, 0x40, 0x70, 19, 0xAA);
                        full[..n].to_vec()
                    }
                    _ => gen(&mut rng, Shape::Incompressible, n),
                };
                let (sc, sr) = scratch_pair("r163");
                write_raw(sc.path(), &data);
                write_raw(sr.path(), &data);
                assert_eq!(sc.bytes().len(), n);
                let a = read_open_only(&c, sc.path());
                let b = read_open_only(&r, sr.path());
                same_and_is(
                    &format!("row163 readOpen on a {n}-byte file (trial {trial})"),
                    a,
                    b,
                    err(23),
                );
            }
        }

        // Exactly 19 bytes of a valid header is enough: the boundary is tight.
        let ok = valid_shaped_header(&c, &r, 0x40, 0x70, HEADER_SIZE_MAX, 0);
        let (sc, sr) = scratch_pair("r163ok");
        write_raw(sc.path(), &ok);
        write_raw(sr.path(), &ok);
        let a = read_open_only(&c, sc.path());
        let b = read_open_only(&r, sr.path());
        same_and_is("row163 readOpen on exactly 19 bytes", a, b, 0);
    }
}

// ===========================================================================
// Row 164 — LZ4F_readOpen: LZ4F_getFrameInfo failed -> forwarded verbatim
// ===========================================================================

#[test]
fn err_164_readopen_getframeinfo_failure_is_forwarded() {
    unsafe {
        let (c, r) = apis();

        // Hand-built 19+ byte headers, each breaking exactly one rule that
        // LZ4F_decodeHeader checks, in the order it checks them.
        let mut cases: Vec<(String, Vec<u8>, usize)> = Vec::new();

        // bad magic number -> frameType_unknown err(13)
        for wrong in [0x00u8, 0x05, 0xFF] {
            let mut h = valid_shaped_header(&c, &r, 0x40, 0x70, 24, 0);
            h[0] = wrong;
            cases.push((format!("bad magic byte0={wrong:#x}"), h, err(13)));
        }
        {
            let mut h = valid_shaped_header(&c, &r, 0x40, 0x70, 24, 0);
            h[3] = 0x19;
            cases.push(("bad magic byte3".into(), h, err(13)));
        }
        // FLG reserved bit 1 set -> reservedFlag_set err(8)
        cases.push((
            "FLG bit1 set".into(),
            valid_shaped_header(&c, &r, 0x42, 0x70, 24, 0),
            err(8),
        ));
        // FLG version field (bits 6..7) != 1 -> headerVersion_wrong err(6)
        for ver in [0u8, 2, 3] {
            let flg = (ver << 6) | 0x00;
            cases.push((
                format!("FLG version={ver}"),
                valid_shaped_header(&c, &r, flg, 0x70, 24, 0),
                err(6),
            ));
        }
        // BD reserved bit 7 set -> reservedFlag_set err(8)
        cases.push((
            "BD bit7 set".into(),
            valid_shaped_header(&c, &r, 0x40, 0xF0, 24, 0),
            err(8),
        ));
        // BD low nibble non-zero -> reservedFlag_set err(8)
        for low in [1u8, 2, 7, 0x0F] {
            cases.push((
                format!("BD low nibble={low:#x}"),
                valid_shaped_header(&c, &r, 0x40, 0x70 | low, 24, 0),
                err(8),
            ));
        }
        // valid FLG/BD but a wrong header-checksum byte -> err(17)
        for delta in [1u8, 2, 0x80, 0xFF] {
            let mut h = valid_shaped_header(&c, &r, 0x40, 0x70, 24, 0);
            h[6] = h[6].wrapping_add(delta);
            cases.push((format!("HC off by {delta:#x}"), h, err(17)));
        }
        // contentSize flag set (FLG bit3) makes the header 15 bytes; a wrong HC
        // at the new position is still err(17)
        {
            let flg = 0x48u8;
            let bd = 0x70u8;
            let mut body = vec![flg, bd];
            body.extend_from_slice(&1234u64.to_le_bytes());
            let hc = header_crc(&c, &r, &body);
            let mut h = vec![0x04u8, 0x22, 0x4D, 0x18];
            h.extend_from_slice(&body);
            h.push(hc ^ 0xFF);
            h.resize(24, 0);
            cases.push(("15-byte header, wrong HC".into(), h, err(17)));
        }

        for (name, bytes, expect) in cases {
            let (sc, sr) = scratch_pair("r164");
            write_raw(sc.path(), &bytes);
            write_raw(sr.path(), &bytes);
            let a = read_open_only(&c, sc.path());
            let b = read_open_only(&r, sr.path());
            same_and_is(&format!("row164 [{name}] readOpen"), a, b, expect);
        }
    }
}

// ===========================================================================
// Row 165 — LZ4F_readOpen: info.blockSizeID not in {0,4,5,6,7} -> err(2)
// ===========================================================================

/// Note on reachability: lz4file.c:108-125 switches on the *decoded*
/// `info.blockSizeID` and has a `default:` arm returning
/// `LZ4F_ERROR_maxBlockSize_invalid`. That arm is DEAD CODE, because
/// `LZ4F_decodeHeader` (lz4frame.c:1410) already rejects `blockSizeID < 4` with
/// the very same `err(2)` and the field is only 3 bits wide, so a decoded value
/// is always in `{4,5,6,7}`. `err(2)` is therefore still exactly what the
/// caller observes — it just comes from `LZ4F_getFrameInfo` (row 164's
/// forwarding path) rather than from lz4file's own switch. This test pins the
/// observable contract: blockSizeID bytes 1..3 -> err(2) even with a CORRECT
/// header checksum (proving the blockSizeID check precedes the HC check), and
/// 4..7 open successfully.
#[test]
fn err_165_readopen_invalid_blocksizeid_is_maxblocksize_invalid() {
    unsafe {
        let (c, r) = apis();

        // BD = blockSizeID<<4; low nibble and bit 7 clear so blockSizeID is the
        // only thing wrong. Header checksum is correct in every case.
        for bsid in [0u8, 1, 2, 3] {
            for flg in [0x40u8, 0x60, 0x44, 0x50, 0x64] {
                let h = valid_shaped_header(&c, &r, flg, bsid << 4, 24, 0);
                // sanity: the HC byte really is right
                let hc = header_crc(&c, &r, &[flg, bsid << 4]);
                assert_eq!(h[6], hc);
                let (sc, sr) = scratch_pair("r165bad");
                write_raw(sc.path(), &h);
                write_raw(sr.path(), &h);
                let a = read_open_only(&c, sc.path());
                let b = read_open_only(&r, sr.path());
                same_and_is(
                    &format!("row165 readOpen blockSizeID byte {bsid} flg={flg:#x}"),
                    a,
                    b,
                    err(2),
                );
            }
        }

        // 4,5,6,7 are accepted (and drive srcBufMaxSize 64K/256K/1M/4M).
        for bsid in [4u8, 5, 6, 7] {
            let h = valid_shaped_header(&c, &r, 0x40, bsid << 4, HEADER_SIZE_MAX, 0);
            let (sc, sr) = scratch_pair("r165ok");
            write_raw(sc.path(), &h);
            write_raw(sr.path(), &h);
            let a = read_open_only(&c, sc.path());
            let b = read_open_only(&r, sr.path());
            same_and_is(
                &format!("row165 readOpen blockSizeID byte {bsid} must succeed"),
                a,
                b,
                0,
            );
        }

        // And a real frame for each valid blockSizeID round-trips, which is the
        // positive form of the same switch.
        let mut rng = Rng::new(165);
        let payload = gen(&mut rng, Shape::TextLike, 120_000);
        for bsid in [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            let p = prefs_with(bsid);
            let (sc, sr) = scratch_pair("r165rt");
            assert_ok("row165 write C", write_all(&c, sc.path(), Some(&p), &[&payload]).close);
            assert_ok("row165 write Rust", write_all(&r, sr.path(), Some(&p), &[&payload]).close);
            let a = read_all(&c, sc.path(), 40_000, payload.len() + 1);
            let b = read_all(&r, sr.path(), 40_000, payload.len() + 1);
            same_and_is(&format!("row165 readOpen bsid={bsid}"), a.open, b.open, 0);
            assert_eq!(a.data, payload, "row165 bsid={bsid}: C round-trip");
            assert_eq!(b.data, payload, "row165 bsid={bsid}: Rust round-trip");
        }
    }
}

// ===========================================================================
// Row 166 — LZ4F_readOpen: malloc(srcBufMaxSize) == NULL -> err(9)
// ===========================================================================

/// UNFORCEABLE: lz4file.c:128 calls libc `malloc` directly with no
/// custom-allocator hook, so the NULL return cannot be produced from a test
/// that only drives exported symbols (and interposing `malloc` process-wide
/// would break the test harness itself). Closest reachable assertions — the
/// *success* half of the same statement, i.e. that the buffer really is
/// allocated with the size the `switch` above it selected:
///   * for every valid blockSizeID (64 KB / 256 KB / 1 MB / 4 MB) `readOpen`
///     succeeds and a frame LARGER than `srcBufMaxSize` reads back byte-exact,
///     which requires the buffer to be at least that big and to be refilled in
///     `srcBufMaxSize` gulps;
///   * both libraries consume the whole file (identical `ftell`), i.e. they use
///     the same gulp size;
///   * `LZ4F_getErrorName(err(9))` — the value this row reports — matches.
#[test]
fn err_166_readopen_srcbuf_malloc_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(166);

        for (bsid, srcbuf_max) in [
            (LZ4F_DEFAULT, 64 * 1024usize),
            (LZ4F_MAX64KB, 64 * 1024),
            (LZ4F_MAX256KB, 256 * 1024),
            (LZ4F_MAX1MB, 1024 * 1024),
            (LZ4F_MAX4MB, 4 * 1024 * 1024),
        ] {
            // Incompressible data, so the compressed frame really exceeds
            // srcBufMaxSize and forces several fread gulps.
            let payload = gen(&mut rng, Shape::Incompressible, srcbuf_max + 5000);
            let p = prefs_with(bsid);
            let (sc, sr) = scratch_pair("r166");
            assert_ok("row166 write C", write_all(&c, sc.path(), Some(&p), &[&payload]).close);
            assert_ok("row166 write Rust", write_all(&r, sr.path(), Some(&p), &[&payload]).close);
            let flen = sc.bytes().len();
            assert!(
                flen > srcbuf_max,
                "row166 bsid={bsid}: frame ({flen}) must exceed srcBufMaxSize ({srcbuf_max})"
            );

            let mut tells = Vec::new();
            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "rb");
                let mut h: *mut c_void = ptr::null_mut();
                let open = (api.ropen)(&mut h, fp);
                assert_eq!(open, 0, "row166 bsid={bsid} {}: readOpen {open:#x}", api.tag);
                assert!(!h.is_null());
                let mut out = vec![0u8; payload.len() + 64];
                let mut got = 0usize;
                loop {
                    let n = (api.read)(h, out[got..].as_mut_ptr() as *mut c_void, 33_333);
                    assert!(!is_err_range(n), "row166 {}: read error {n:#x}", api.tag);
                    if n == 0 {
                        break;
                    }
                    got += n;
                }
                assert_eq!(got, payload.len(), "row166 bsid={bsid} {}: short", api.tag);
                assert_eq!(&out[..got], &payload[..], "row166 bsid={bsid} {}: content", api.tag);
                assert_eq!((api.rclose)(h), 0, "row166 {}: readClose", api.tag);
                tells.push(ftell(fp));
                fclose(fp);
            }
            assert_eq!(
                tells[0], tells[1],
                "row166 bsid={bsid}: stream position after a full read differs (C={} Rust={})",
                tells[0], tells[1]
            );
            assert_eq!(
                tells[0] as usize, flen,
                "row166 bsid={bsid}: expected the whole {flen}-byte frame consumed, ftell={}",
                tells[0]
            );
        }

        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(cn.to_bytes(), rn.to_bytes(), "row166: getErrorName(err(9)) differs");
    }
}

// ===========================================================================
// Row 167 — LZ4F_read: lz4fRead == NULL or buf == NULL -> err(21)
// ===========================================================================

#[test]
fn err_167_read_null_params() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(167);
        let payload = gen(&mut rng, Shape::TextLike, 20_000);
        let (sc, sr) = scratch_pair("r167");
        assert_ok("row167 write C", write_all(&c, sc.path(), None, &[&payload]).close);
        assert_ok("row167 write Rust", write_all(&r, sr.path(), None, &[&payload]).close);

        let fpc = open_file(sc.path(), "rb");
        let fpr = open_file(sr.path(), "rb");
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        assert_eq!((c.ropen)(&mut hc, fpc), 0);
        assert_eq!((r.ropen)(&mut hr, fpr), 0);

        // buf == NULL: size 0 and a spread of non-zero sizes.
        let mut sizes = vec![0usize, 1, 2, 100, 65536, usize::MAX];
        for _ in 0..24 {
            sizes.push(rng.range(1, 4_000_000));
        }
        for &n in &sizes {
            // valid handle, NULL buffer
            let a = (c.read)(hc, ptr::null_mut(), n);
            let b = (r.read)(hr, ptr::null_mut(), n);
            same_and_is(&format!("row167 read(buf=NULL,size={n})"), a, b, err(21));
            // NULL handle, valid buffer
            let mut buf = [0u8; 8];
            let a = (c.read)(ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, n);
            let b = (r.read)(ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, n);
            same_and_is(&format!("row167 read(handle=NULL,size={n})"), a, b, err(21));
            // both NULL
            let a = (c.read)(ptr::null_mut(), ptr::null_mut(), n);
            let b = (r.read)(ptr::null_mut(), ptr::null_mut(), n);
            same_and_is(&format!("row167 read(both NULL,size={n})"), a, b, err(21));
        }

        // None of the rejected calls may have touched the stream: the payload
        // still reads back whole.
        let mut oc = vec![0u8; payload.len()];
        let mut or = vec![0u8; payload.len()];
        let a = (c.read)(hc, oc.as_mut_ptr() as *mut c_void, payload.len());
        let b = (r.read)(hr, or.as_mut_ptr() as *mut c_void, payload.len());
        same_and_is("row167 read after rejected calls", a, b, payload.len());
        assert_eq!(oc, payload, "row167: C payload mismatch");
        assert_eq!(or, payload, "row167: Rust payload mismatch");
        same("row167 readClose", (c.rclose)(hc), (r.rclose)(hr));
        fclose(fpc);
        fclose(fpr);
    }
}

// ===========================================================================
// Row 168 — LZ4F_read: the "negative fread" branch -> err(23)
// ===========================================================================

/// UNREACHABLE: lz4file.c:159-163 tests `ret > 0` / `ret == 0` / `else`, but
/// `ret` is a `size_t`, so the trailing `else` (the `RETURN_ERROR(io_read)`
/// arm) can never be taken — an `fread` failure is indistinguishable from EOF
/// here and lands in the `ret == 0` arm, which `break`s out of the loop. The
/// REACHABLE consequence, pinned below: a request larger than what remains in
/// the frame returns a SHORT COUNT (never `err(23)`), and the count is exactly
/// the number of payload bytes still available; a further read returns 0.
#[test]
fn err_168_read_negative_fread_branch_is_unreachable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(168);

        for iter in 0..24 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = match rng.below(3) {
                0 => rng.range(1, 200),
                1 => rng.range(1, 30_000),
                _ => rng.range(60_000, 200_000),
            };
            let payload = gen(&mut rng, shape, len);
            let (sc, sr) = scratch_pair("r168");
            assert_ok("row168 write C", write_all(&c, sc.path(), None, &[&payload]).close);
            assert_ok("row168 write Rust", write_all(&r, sr.path(), None, &[&payload]).close);

            // How many bytes to take before the over-large request.
            let prefix = rng.below(len);
            let over = len + rng.range(1, 100_000);

            let mut seqs = Vec::new();
            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "rb");
                let mut h: *mut c_void = ptr::null_mut();
                assert_eq!((api.ropen)(&mut h, fp), 0, "row168 {}: readOpen", api.tag);
                let mut got = Vec::new();
                if prefix > 0 {
                    let mut b = vec![0u8; prefix];
                    let n = (api.read)(h, b.as_mut_ptr() as *mut c_void, prefix);
                    got.push(n);
                    assert_eq!(n, prefix, "row168 {}: prefix read", api.tag);
                    assert_eq!(&b[..], &payload[..prefix]);
                }
                let mut b = vec![0u8; over];
                let n = (api.read)(h, b.as_mut_ptr() as *mut c_void, over);
                got.push(n);
                assert!(
                    !is_err_range(n),
                    "row168 {}: EOF must not report an error, got {n:#x}",
                    api.tag
                );
                assert_eq!(
                    n,
                    len - prefix,
                    "row168 iter={iter} {}: expected the exact short count {}, got {n}",
                    api.tag,
                    len - prefix
                );
                assert_eq!(&b[..n], &payload[prefix..], "row168 {}: tail content", api.tag);
                // and once more, now completely at EOF
                let z = (api.read)(h, b.as_mut_ptr() as *mut c_void, over);
                got.push(z);
                assert_eq!(z, 0, "row168 {}: read past EOF must be 0, got {z:#x}", api.tag);
                assert_eq!((api.rclose)(h), 0, "row168 {}: readClose", api.tag);
                fclose(fp);
                seqs.push(got);
            }
            assert_eq!(
                seqs[0].iter().map(|&x| x as isize).collect::<Vec<_>>(),
                seqs[1].iter().map(|&x| x as isize).collect::<Vec<_>>(),
                "row168 iter={iter}: read sequences differ"
            );
        }
    }
}

// ===========================================================================
// Row 169 — LZ4F_read: LZ4F_decompress error returned verbatim
// ===========================================================================

/// Frame layout of a single-block frame, so corruption can be aimed exactly.
struct Layout {
    hlen: usize,
    blk_data: usize, // offset of the compressed block payload
    blk_size: usize, // its length
    blk_crc: usize,  // offset of the 4-byte block checksum (if enabled)
}

fn layout(bytes: &[u8], content_size: bool, dict_id: bool, block_crc: bool) -> Layout {
    let hlen = 7 + if content_size { 8 } else { 0 } + if dict_id { 4 } else { 0 };
    let bh = u32::from_le_bytes([bytes[hlen], bytes[hlen + 1], bytes[hlen + 2], bytes[hlen + 3]]);
    let size = (bh & 0x7FFF_FFFF) as usize;
    assert!(size > 0, "layout: expected a non-empty first block");
    let data = hlen + 4;
    Layout {
        hlen,
        blk_data: data,
        blk_size: size,
        blk_crc: if block_crc { data + size } else { usize::MAX },
    }
}

#[test]
fn err_169_read_forwards_decompress_errors() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(169);

        // Drive both libraries over identical (separately written) files.
        let check = |name: &str, bc: &[u8], br: &[u8], expect: Option<usize>, chunk: usize| {
            let (sc, sr) = scratch_pair("r169");
            write_raw(sc.path(), bc);
            write_raw(sr.path(), br);
            let a = read_all(&c, sc.path(), chunk, usize::MAX);
            let b = read_all(&r, sr.path(), chunk, usize::MAX);
            same(&format!("row169 [{name}] readOpen"), a.open, b.open);
            assert_eq!(
                a.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
                b.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
                "row169 [{name}]: LZ4F_read sequences differ (C={:#x?} Rust={:#x?})",
                a.reads,
                b.reads
            );
            assert_eq!(
                a.close.map(|x| x as isize),
                b.close.map(|x| x as isize),
                "row169 [{name}]: readClose differs"
            );
            if let Some(e) = expect {
                let last = *a.reads.last().expect("row169: no reads happened");
                assert_eq!(
                    last as isize, e as isize,
                    "row169 [{name}]: expected the final LZ4F_read to be {e:#x} (LZ4F code {}), \
                     got {last:#x} (LZ4F code {}); full sequence {:#x?}",
                    (0usize).wrapping_sub(e) as isize,
                    (0usize).wrapping_sub(last) as isize,
                    a.reads
                );
            }
            (a.reads, a.data)
        };

        // ---- (a) corrupted CONTENT checksum -> err(18) contentChecksum_invalid
        // The read request must exceed the payload so the loop keeps going and
        // the trailer is actually decoded.
        {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
            for trial in 0..6 {
                let plen = rng.range(64, 30_000);
                let payload = gen(&mut rng, ALL_SHAPES[trial % ALL_SHAPES.len()], plen);
                let (sc, sr) = scratch_pair("r169cc");
                assert_ok("row169 write C", write_all(&c, sc.path(), Some(&p), &[&payload]).close);
                assert_ok("row169 write Rust", write_all(&r, sr.path(), Some(&p), &[&payload]).close);
                let mut bc = sc.bytes();
                let mut br = sr.bytes();
                assert_eq!(bc, br, "row169: the two writers produced different frames");
                let n = bc.len();
                let i = n - 4 + (trial % 4);
                bc[i] ^= 0x01 << (trial % 8);
                br[i] ^= 0x01 << (trial % 8);
                check(
                    &format!("content checksum flipped at {i} (trial {trial})"),
                    &bc,
                    &br,
                    Some(err(18)),
                    payload.len() + 4096,
                );
            }
        }

        // ---- (b) corrupted BLOCK checksum -> err(7) blockChecksum_invalid
        {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
            for trial in 0..8 {
                let plen = rng.range(64, 40_000);
                let payload = gen(&mut rng, ALL_SHAPES[trial % ALL_SHAPES.len()], plen);
                let (sc, sr) = scratch_pair("r169bc");
                assert_ok("row169 write C", write_all(&c, sc.path(), Some(&p), &[&payload]).close);
                assert_ok("row169 write Rust", write_all(&r, sr.path(), Some(&p), &[&payload]).close);
                let mut bc = sc.bytes();
                let mut br = sr.bytes();
                assert_eq!(bc, br, "row169: the two writers produced different frames");
                let l = layout(&bc, false, false, true);
                assert!(l.blk_crc + 4 <= bc.len());
                let i = l.blk_crc + (trial % 4);
                bc[i] ^= 0x80 >> (trial % 8);
                br[i] ^= 0x80 >> (trial % 8);
                check(
                    &format!("block checksum flipped at {i} (trial {trial})"),
                    &bc,
                    &br,
                    Some(err(7)),
                    payload.len() + 4096,
                );
            }
        }

        // ---- (b2) a byte flipped inside the block PAYLOAD with block
        // checksums on is caught by the block CRC -> err(7) as well.
        {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
            for trial in 0..8 {
                let plen = rng.range(200, 40_000);
                let payload = gen(&mut rng, Shape::TextLike, plen);
                let (sc, sr) = scratch_pair("r169pl");
                assert_ok("row169 write C", write_all(&c, sc.path(), Some(&p), &[&payload]).close);
                assert_ok("row169 write Rust", write_all(&r, sr.path(), Some(&p), &[&payload]).close);
                let mut bc = sc.bytes();
                let mut br = sr.bytes();
                let l = layout(&bc, false, false, true);
                let i = l.blk_data + rng.below(l.blk_size);
                let m = 1u8 << rng.below(8);
                bc[i] ^= m;
                br[i] ^= m;
                check(
                    &format!("payload byte {i} flipped, block CRC on (trial {trial})"),
                    &bc,
                    &br,
                    Some(err(7)),
                    payload.len() + 4096,
                );
            }
        }

        // ---- (c) an outright invalid compressed block -> err(16)
        // decompressionFailed. Hand-built: one "compressed" block of a single
        // 0xF0 byte, i.e. a token announcing 15+ literals with no bytes left.
        {
            // NB: the block must be >= 4 bytes so that
            // 7 (header) + 4 (block header) + len + 4 (endMark) >= 19 and no
            // padding is needed AFTER the endMark — trailing padding would be
            // decoded as a second frame header and report err(13) instead.
            let hdr = valid_shaped_header(&c, &r, 0x40, 0x70, 7, 0);
            for &bad in &[
                &[0xF0u8, 0xFF, 0xFF, 0xFF][..], // literal-length continuation runs off the end
                &[0xFF, 0xFF, 0xFF, 0xFF][..],
                &[0x40, 0xAA, 0xBB, 0xCC][..], // announces 4 literals, only 3 bytes left
                &[0x11, 0xAA, 0x01, 0x00][..], // match with offset 1 but no history
                &[0x11, 0xAA, 0xFF, 0xFF][..], // match offset far before the output start
                &[0xF0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF][..],
            ] {
                let mut f = hdr.clone();
                f.extend_from_slice(&(bad.len() as u32).to_le_bytes());
                f.extend_from_slice(bad);
                f.extend_from_slice(&0u32.to_le_bytes()); // endMark
                assert!(f.len() >= HEADER_SIZE_MAX, "row169: crafted frame too short");
                check(
                    &format!("invalid block {bad:02x?}"),
                    &f,
                    &f,
                    Some(err(16)),
                    64 * 1024,
                );
            }
        }

        // ---- (d) a TRUNCATED frame. lz4file cannot distinguish a truncated
        // file from EOF (see row 168: the `fread == 0` arm just `break`s), so
        // truncation is NOT reported as an error — it yields a short count.
        // Both libraries must agree on the exact counts.
        {
            let payload = gen(&mut rng, Shape::TextLike, 50_000);
            let (sc, sr) = scratch_pair("r169tr");
            assert_ok("row169 write C", write_all(&c, sc.path(), None, &[&payload]).close);
            assert_ok("row169 write Rust", write_all(&r, sr.path(), None, &[&payload]).close);
            let bc = sc.bytes();
            let br = sr.bytes();
            assert_eq!(bc, br);
            for cut in [
                HEADER_SIZE_MAX,
                HEADER_SIZE_MAX + 1,
                bc.len() / 4,
                bc.len() / 2,
                bc.len() - 5,
                bc.len() - 4,
                bc.len() - 1,
            ] {
                let (reads, data) = check(
                    &format!("truncated to {cut} of {}", bc.len()),
                    &bc[..cut],
                    &br[..cut],
                    None,
                    payload.len() + 4096,
                );
                assert!(
                    reads.iter().all(|&x| !is_err_range(x)),
                    "row169: truncation must not be reported as an error, got {reads:#x?}"
                );
                let total: usize = reads.iter().sum();
                assert_eq!(total, data.len());
                assert!(
                    total <= payload.len(),
                    "row169: a frame truncated to {cut} produced {total} > {} bytes",
                    payload.len()
                );
                assert_eq!(
                    &data[..], &payload[..total],
                    "row169: the bytes recovered from a frame truncated to {cut} are not a                      prefix of the payload"
                );
                // Cutting into the compressed data really does lose payload;
                // cutting only the trailer keeps it all (the missing endMark is
                // indistinguishable from EOF).
                if cut <= bc.len() / 2 {
                    assert!(
                        total < payload.len(),
                        "row169: a frame truncated to {cut} still yielded the whole payload"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 170 — LZ4F_readClose(NULL) -> err(21)
// ===========================================================================

#[test]
fn err_170_readclose_null_handle() {
    unsafe {
        let (c, r) = apis();

        // Repeated, to prove it is stateless and never frees anything.
        for _ in 0..64 {
            let a = (c.rclose)(ptr::null_mut());
            let b = (r.rclose)(ptr::null_mut());
            same_and_is("row170 readClose(NULL)", a, b, err(21));
        }

        // A real handle still closes with LZ4F_OK_NoError, and the NULL calls
        // in between change nothing.
        let mut rng = Rng::new(170);
        for _ in 0..6 {
            let plen = rng.range(1, 20_000);
            let payload = gen(&mut rng, Shape::Compressible, plen);
            let (sc, sr) = scratch_pair("r170");
            assert_ok("row170 write C", write_all(&c, sc.path(), None, &[&payload]).close);
            assert_ok("row170 write Rust", write_all(&r, sr.path(), None, &[&payload]).close);
            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "rb");
                let mut h: *mut c_void = ptr::null_mut();
                assert_eq!((api.ropen)(&mut h, fp), 0);
                assert_eq!(
                    (api.rclose)(ptr::null_mut()),
                    err(21),
                    "row170 {}: readClose(NULL) with a live handle open",
                    api.tag
                );
                assert_eq!((api.rclose)(h), 0, "row170 {}: readClose(handle)", api.tag);
                assert_eq!(
                    (api.rclose)(ptr::null_mut()),
                    err(21),
                    "row170 {}: readClose(NULL) after a real close",
                    api.tag
                );
                fclose(fp);
            }
        }
    }
}

// ===========================================================================
// Write-side helpers
// ===========================================================================

/// The frame-header length `LZ4F_writeOpen` will `fwrite`, obtained through the
/// same `LZ4F_compressBegin(cctx, buf, LZ4F_HEADER_SIZE_MAX, prefs)` call
/// lz4file.c:265 makes. Both libraries must report the same length and produce
/// the same header bytes.
unsafe fn header_len(c: &FileApi, r: &FileApi, prefs: Option<&LZ4F_preferences_t>) -> usize {
    let pp = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => ptr::null(),
    };
    let mut lens = Vec::new();
    let mut bufs: Vec<Vec<u8>> = Vec::new();
    for api in [c, r] {
        let mut ctx: *mut c_void = ptr::null_mut();
        assert_eq!(
            (api.create_cctx)(&mut ctx, LZ4F_VERSION),
            0,
            "{}: createCompressionContext",
            api.tag
        );
        assert!(!ctx.is_null());
        let mut buf = vec![0u8; HEADER_SIZE_MAX];
        let n = (api.compress_begin)(ctx, buf.as_mut_ptr() as *mut c_void, HEADER_SIZE_MAX, pp);
        assert!(!is_err_range(n), "{}: compressBegin failed {n:#x}", api.tag);
        assert!(n <= HEADER_SIZE_MAX);
        buf.truncate(n);
        lens.push(n);
        bufs.push(buf);
        (api.free_cctx)(ctx);
    }
    assert_eq!(lens[0], lens[1], "header_len differs: C={} Rust={}", lens[0], lens[1]);
    assert_eq!(bufs[0], bufs[1], "frame header bytes differ between the libraries");
    lens[0]
}

/// Drive writeOpen/write/writeClose against an unbuffered fixed-capacity
/// `fmemopen` stream, so any write past `cap` is short.
unsafe fn write_to_mem(
    api: &FileApi,
    cap: usize,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
) -> WriteOut {
    let mf = MemFile::new(cap);
    let pp = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => ptr::null(),
    };
    let mut h: *mut c_void = ptr::null_mut();
    let open = (api.wopen)(&mut h, mf.fp, pp);
    let mut writes = Vec::new();
    let mut close = None;
    if open == 0 {
        assert!(!h.is_null(), "{}: writeOpen 0 with a NULL handle", api.tag);
        for ch in chunks {
            writes.push((api.write)(h, ch.as_ptr() as *const c_void, ch.len()));
        }
        close = Some((api.wclose)(h));
    } else {
        assert!(h.is_null(), "{}: writeOpen {open:#x} left a non-NULL handle", api.tag);
    }
    drop(mf);
    WriteOut { open, writes, close }
}

/// Same, but on `/dev/full`: every write fails at the device. `buffered=false`
/// makes even the small frame header fail.
unsafe fn write_to_dev_full(
    api: &FileApi,
    buffered: bool,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
) -> WriteOut {
    let fp = open_file(Path::new("/dev/full"), "wb");
    if !buffered {
        assert_eq!(setvbuf(fp, ptr::null_mut(), IONBF, 0), 0, "setvbuf(IONBF)");
    }
    let pp = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => ptr::null(),
    };
    let mut h: *mut c_void = ptr::null_mut();
    let open = (api.wopen)(&mut h, fp, pp);
    let mut writes = Vec::new();
    let mut close = None;
    if open == 0 {
        assert!(!h.is_null(), "{}: writeOpen 0 with a NULL handle", api.tag);
        for ch in chunks {
            writes.push((api.write)(h, ch.as_ptr() as *const c_void, ch.len()));
        }
        close = Some((api.wclose)(h));
    } else {
        assert!(h.is_null(), "{}: writeOpen {open:#x} left a non-NULL handle", api.tag);
    }
    fclose(fp);
    WriteOut { open, writes, close }
}

#[track_caller]
fn same_write_out(ctx: &str, a: &WriteOut, b: &WriteOut) {
    same(&format!("{ctx}: writeOpen"), a.open, b.open);
    assert_eq!(
        a.writes.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        b.writes.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        "{ctx}: LZ4F_write returns differ (C={:#x?} Rust={:#x?})",
        a.writes,
        b.writes
    );
    assert_eq!(
        a.close.map(|x| x as isize),
        b.close.map(|x| x as isize),
        "{ctx}: LZ4F_writeClose differs (C={:#x?} Rust={:#x?})",
        a.close,
        b.close
    );
}

// ===========================================================================
// Row 171 — LZ4F_writeOpen: fp == NULL or lz4fWrite == NULL -> err(21)
// ===========================================================================

#[test]
fn err_171_writeopen_null_params() {
    unsafe {
        let (c, r) = apis();
        let p_default = LZ4F_preferences_t::default();
        let mut p_odd = LZ4F_preferences_t::default();
        p_odd.frameInfo.blockSizeID = 3; // invalid, but the NULL check comes first
        p_odd.compressionLevel = 12;

        for prefs in [
            None,
            Some(&p_default as *const LZ4F_preferences_t),
            Some(&p_odd as *const LZ4F_preferences_t),
        ] {
            let pp = prefs.unwrap_or(ptr::null());

            // (a) fp == NULL: the out-pointer must not be touched at all.
            let sentinel = 0x2468usize as *mut c_void;
            let mut hc = sentinel;
            let mut hr = sentinel;
            let a = (c.wopen)(&mut hc, ptr::null_mut(), pp);
            let b = (r.wopen)(&mut hr, ptr::null_mut(), pp);
            same_and_is("row171 writeOpen(fp=NULL)", a, b, err(21));
            assert_eq!(hc, sentinel, "row171: C overwrote the out-pointer");
            assert_eq!(hr, sentinel, "row171: Rust overwrote the out-pointer");

            // (b) lz4fWrite == NULL with a real, writable file: nothing may be
            // written to it.
            let (sc, sr) = scratch_pair("r171");
            let fpc = open_file(sc.path(), "wb");
            let fpr = open_file(sr.path(), "wb");
            let a = (c.wopen)(ptr::null_mut(), fpc, pp);
            let b = (r.wopen)(ptr::null_mut(), fpr, pp);
            same_and_is("row171 writeOpen(handle=NULL)", a, b, err(21));
            assert_eq!(fflush(fpc), 0);
            assert_eq!(fflush(fpr), 0);
            assert_eq!(ftell(fpc), 0, "row171: C wrote bytes despite err(21)");
            assert_eq!(ftell(fpr), 0, "row171: Rust wrote bytes despite err(21)");
            fclose(fpc);
            fclose(fpr);
            assert!(sc.bytes().is_empty() && sr.bytes().is_empty());

            // (c) both NULL.
            let a = (c.wopen)(ptr::null_mut(), ptr::null_mut(), pp);
            let b = (r.wopen)(ptr::null_mut(), ptr::null_mut(), pp);
            same_and_is("row171 writeOpen(both NULL)", a, b, err(21));
        }
    }
}

// ===========================================================================
// Row 172 — LZ4F_writeOpen: calloc(1, sizeof(LZ4_writeFile_t)) == NULL -> err(9)
// ===========================================================================

/// UNFORCEABLE for the same reason as row 161: lz4file.c:225 calls libc
/// `calloc` directly and there is no allocator hook on this path; interposing
/// `calloc` process-wide would break the harness itself. Closest reachable
/// assertions:
///   1. the success half of that statement — `writeOpen` returns 0 and
///      publishes a NON-NULL handle in both libraries;
///   2. the state really is ZEROED by `calloc`, which is observable through
///      `errCode`: an immediate `LZ4F_writeClose` performs `compressEnd`
///      (rather than taking row 184's "an error was latched" shortcut), so it
///      returns the trailer length and the file is a valid, readable frame;
///   3. `LZ4F_getErrorName(err(9))` — the value this row reports — matches.
#[test]
fn err_172_writeopen_calloc_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();

        for bsid in [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            for prefs in [None, Some(prefs_with(bsid))] {
                let (sc, sr) = scratch_pair("r172");
                let mut outs = Vec::new();
                for api in [&c, &r] {
                    let path = if api.tag == "C" { sc.path() } else { sr.path() };
                    let fp = open_file(path, "wb");
                    let pp = match &prefs {
                        Some(p) => p as *const LZ4F_preferences_t,
                        None => ptr::null(),
                    };
                    let mut h: *mut c_void = ptr::null_mut();
                    let open = (api.wopen)(&mut h, fp, pp);
                    assert_eq!(open, 0, "row172 {}: writeOpen {open:#x}", api.tag);
                    assert!(
                        !h.is_null(),
                        "row172 {}: writeOpen returned 0 but published a NULL state",
                        api.tag
                    );
                    // errCode was zeroed by calloc: compressEnd really runs.
                    let close = (api.wclose)(h);
                    fclose(fp);
                    let n = assert_ok("row172: writeClose after a bare open", Some(close));
                    assert!(n > 0, "row172 {}: writeClose wrote no trailer", api.tag);
                    outs.push(n);
                }
                same("row172 writeClose trailer length", outs[0], outs[1]);
                same_full_buffers("row172 empty-frame bytes", &sc.bytes(), &sr.bytes());
                // and the produced frame is a valid, readable, empty frame
                let a = read_all(&c, sc.path(), 4096, usize::MAX);
                let b = read_all(&r, sr.path(), 4096, usize::MAX);
                same("row172 readOpen of the produced frame", a.open, b.open);
                assert!(a.data.is_empty() && b.data.is_empty());
            }
        }

        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(cn.to_bytes(), rn.to_bytes(), "row172: getErrorName(err(9)) differs");
    }
}

// ===========================================================================
// Row 173 — LZ4F_writeOpen: blockSizeID not in {0,4,5,6,7} -> err(2)
// ===========================================================================

#[test]
fn err_173_writeopen_invalid_blocksizeid_is_maxblocksize_invalid() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(173);

        // A fixed list plus a randomized sweep of every other u32 value.
        let mut bad: Vec<c_uint> = vec![1, 2, 3, 8, 9, 100, 0xFFFF, u32::MAX, u32::MAX - 1];
        while bad.len() < 60 {
            let v = rng.next_u32();
            if !matches!(v, 0 | 4 | 5 | 6 | 7) {
                bad.push(v);
            }
        }

        for &bsid in &bad {
            let mut p = prefs_with(bsid);
            // vary the other fields too: none of them may rescue the bad ID
            p.compressionLevel = [0i32, 1, 9, 12][rng.below(4)];
            p.autoFlush = rng.below(2) as c_uint;
            p.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
            p.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;

            let (sc, sr) = scratch_pair("r173");
            let mut rets = Vec::new();
            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "wb");
                let mut h: *mut c_void = ptr::null_mut();
                let ret = (api.wopen)(&mut h, fp, &p);
                // the out-handle must be left NULL (freeAndNullWriteFile)
                assert!(
                    h.is_null(),
                    "row173 {} bsid={bsid}: handle not nulled after err(2)",
                    api.tag
                );
                assert_eq!(fflush(fp), 0);
                assert_eq!(ftell(fp), 0, "row173 {}: bytes written despite err(2)", api.tag);
                fclose(fp);
                rets.push(ret);
            }
            same_and_is(&format!("row173 writeOpen(blockSizeID={bsid})"), rets[0], rets[1], err(2));
            assert!(sc.bytes().is_empty() && sr.bytes().is_empty());
        }

        // The five accepted values really are accepted.
        let payload = gen(&mut rng, Shape::TextLike, 5000);
        for bsid in [0u32, 4, 5, 6, 7] {
            let p = prefs_with(bsid);
            let (sc, sr) = scratch_pair("r173ok");
            let a = write_all(&c, sc.path(), Some(&p), &[&payload]);
            let b = write_all(&r, sr.path(), Some(&p), &[&payload]);
            same_write_out(&format!("row173 writeOpen(blockSizeID={bsid})"), &a, &b);
            same_and_is(&format!("row173 writeOpen({bsid}) must succeed"), a.open, b.open, 0);
            assert_ok("row173 writeClose", a.close);
            same_full_buffers("row173 frame bytes", &sc.bytes(), &sr.bytes());
        }
    }
}

// ===========================================================================
// Row 174 — LZ4F_writeOpen: malloc(LZ4F_compressBound(maxWriteSize, prefs)) -> err(9)
// ===========================================================================

/// UNFORCEABLE: lz4file.c:253 calls libc `malloc` directly (no allocator hook).
/// Closest reachable assertions, all about the very expression whose result is
/// passed to that `malloc`:
///   1. `LZ4F_compressBound(maxWriteSize, prefsPtr)` — the exact allocation
///      size — is byte-for-byte identical in both libraries for every
///      `blockSizeID` (with the `maxWriteSize` lz4file derives from it) and for
///      a large randomized sweep of preference combinations, including
///      `prefsPtr == NULL`;
///   2. the buffer really is big enough for what lz4file then does with it: a
///      single `LZ4F_write` of exactly `maxWriteSize` bytes (the largest chunk
///      lz4file ever hands to `LZ4F_compressUpdate`) and of `maxWriteSize + 1`
///      succeed for every `blockSizeID`, with the worst-case checksum settings;
///   3. `LZ4F_getErrorName(err(9))` matches.
#[test]
fn err_174_writeopen_dstbuf_malloc_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(174);

        // 1. the malloc size itself
        let sizes = [
            (LZ4F_DEFAULT, 64 * 1024usize),
            (LZ4F_MAX64KB, 64 * 1024),
            (LZ4F_MAX256KB, 256 * 1024),
            (LZ4F_MAX1MB, 1024 * 1024),
            (LZ4F_MAX4MB, 4 * 1024 * 1024),
        ];
        // prefsPtr == NULL -> lz4file uses maxWriteSize = 64 KB
        let a = (c.compress_bound)(64 * 1024, ptr::null());
        let b = (r.compress_bound)(64 * 1024, ptr::null());
        same_and_is("row174 compressBound(64KB, NULL)", a, b, a);
        assert!(a > 64 * 1024, "row174: implausible bound {a}");

        for (bsid, mws) in sizes {
            for iter in 0..40 {
                let mut p = prefs_with(bsid);
                if iter > 0 {
                    p.frameInfo.blockMode = rng.below(2) as c_uint;
                    p.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
                    p.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;
                    p.frameInfo.contentSize = if rng.below(2) == 0 { 0 } else { rng.next_u64() };
                    p.frameInfo.dictID = rng.next_u32();
                    p.compressionLevel = [0i32, 1, 2, -1, 9, 10, 12][rng.below(7)];
                    p.autoFlush = rng.below(2) as c_uint;
                    p.favorDecSpeed = rng.below(2) as c_uint;
                }
                let a = (c.compress_bound)(mws, &p);
                let b = (r.compress_bound)(mws, &p);
                same(&format!("row174 compressBound({mws}, bsid={bsid} iter={iter})"), a, b);
                assert!(
                    !is_err_range(a) && a >= mws,
                    "row174: implausible bound {a} for {mws}"
                );
            }
        }

        // 2. the buffer is sufficient for the largest chunk lz4file passes on
        for (bsid, mws) in sizes {
            let payload = gen(&mut rng, Shape::Incompressible, mws + 1);
            for autoflush in [0u32, 1] {
                let mut p = prefs_with(bsid);
                p.autoFlush = autoflush;
                p.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
                p.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
                for &n in &[mws - 1, mws, mws + 1] {
                    let (sc, sr) = scratch_pair("r174");
                    let a = write_all(&c, sc.path(), Some(&p), &[&payload[..n]]);
                    let b = write_all(&r, sr.path(), Some(&p), &[&payload[..n]]);
                    same_write_out(
                        &format!("row174 bsid={bsid} autoFlush={autoflush} n={n}"),
                        &a,
                        &b,
                    );
                    assert_eq!(a.open, 0);
                    assert_eq!(
                        a.writes,
                        vec![n],
                        "row174 bsid={bsid} n={n}: LZ4F_write must not fail (got {:#x?})",
                        a.writes
                    );
                    assert_ok("row174 writeClose", a.close);
                    same_full_buffers("row174 frame bytes", &sc.bytes(), &sr.bytes());
                }
            }
        }

        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(cn.to_bytes(), rn.to_bytes(), "row174: getErrorName(err(9)) differs");
    }
}

// ===========================================================================
// Row 175 — LZ4F_writeOpen: LZ4F_createCompressionContext failed -> forwarded
// ===========================================================================

/// UNFORCEABLE: `LZ4F_createCompressionContext(&cctx, LZ4F_VERSION)` only fails
/// when its internal allocation returns NULL, and lz4file passes a fixed valid
/// version with a non-NULL out-pointer — no allocator hook exists on this path.
/// Closest reachable assertions:
///   1. the exact call lz4file makes returns `LZ4F_OK_NoError` with a non-NULL
///      context in BOTH libraries, repeatedly, and
///      `LZ4F_freeCompressionContext` accepts the context and NULL identically;
///   2. `writeOpen` demonstrably forwards a non-io error code unchanged rather
///      than rewriting it (row 173's `err(2)` travels through the same
///      `freeAndNull` + `return` shape);
///   3. the code that would be forwarded, `err(9)`, has the same name in both.
#[test]
fn err_175_writeopen_createcctx_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();

        for _ in 0..16 {
            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            let ra = (c.create_cctx)(&mut a, LZ4F_VERSION);
            let rb = (r.create_cctx)(&mut b, LZ4F_VERSION);
            same_and_is("row175 createCompressionContext(LZ4F_VERSION)", ra, rb, 0);
            assert!(!a.is_null() && !b.is_null(), "row175: NULL cctx despite success");
            let fa = (c.free_cctx)(a);
            let fb = (r.free_cctx)(b);
            same_and_is("row175 freeCompressionContext", fa, fb, 0);
            let fa = (c.free_cctx)(ptr::null_mut());
            let fb = (r.free_cctx)(ptr::null_mut());
            same_and_is("row175 freeCompressionContext(NULL)", fa, fb, 0);
        }

        // 2. the forwarding shape, via the reachable err(2) sibling
        let p = prefs_with(3);
        let (sc, sr) = scratch_pair("r175");
        let a = write_all(&c, sc.path(), Some(&p), &[]);
        let b = write_all(&r, sr.path(), Some(&p), &[]);
        same_and_is("row175 writeOpen forwards err(2) unchanged", a.open, b.open, err(2));

        let cn = CStr::from_ptr((c.error_name)(err(9)));
        let rn = CStr::from_ptr((r.error_name)(err(9)));
        assert_eq!(cn.to_bytes(), rn.to_bytes(), "row175: getErrorName(err(9)) differs");
    }
}

// ===========================================================================
// Row 176 — LZ4F_writeOpen: LZ4F_compressBegin failed -> forwarded
// ===========================================================================

/// UNFORCEABLE through lz4file: the only capacity check in
/// `LZ4F_compressBegin_internal` is `dstCapacity < LZ4F_HEADER_SIZE_MAX (19)`
/// (lz4frame.c:700), and lz4file always passes a 19-byte stack buffer with
/// exactly `LZ4F_HEADER_SIZE_MAX` as the capacity, so that branch can never
/// fire from `LZ4F_writeOpen`. Closest reachable assertions, made directly on
/// `LZ4F_compressBegin` through the `.so`:
///   1. with `dstCapacity == 19` — lz4file's call — it never errors, returns
///      the same length and writes the same header bytes in both libraries, for
///      a randomized sweep of preferences (this is what `header_len` checks);
///   2. with `dstCapacity < 19` it returns `err(11) dstMaxSize_tooSmall` in
///      both libraries — that is the exact code `LZ4F_writeOpen` would forward
///      if it ever passed a short buffer.
#[test]
fn err_176_writeopen_compressbegin_failure_is_unforceable() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(176);

        // 1. lz4file's own call: capacity == LZ4F_HEADER_SIZE_MAX
        assert_eq!(header_len(&c, &r, None), 7, "row176: default header is 7 bytes");
        for iter in 0..40 {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB][rng.below(5)];
            p.frameInfo.blockMode = rng.below(2) as c_uint;
            p.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
            p.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;
            p.frameInfo.contentSize = if rng.below(2) == 0 { 0 } else { rng.next_u64() >> 1 };
            p.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() };
            p.compressionLevel = [0i32, 1, 2, 9, 12][rng.below(5)];
            p.autoFlush = rng.below(2) as c_uint;
            p.favorDecSpeed = rng.below(2) as c_uint;
            let n = header_len(&c, &r, Some(&p));
            let expect = 7
                + if p.frameInfo.contentSize != 0 { 8 } else { 0 }
                + if p.frameInfo.dictID != 0 { 4 } else { 0 };
            assert_eq!(
                n, expect,
                "row176 iter={iter}: header length {n} != expected {expect}"
            );
            assert!(n <= HEADER_SIZE_MAX);
        }

        // 2. the code that WOULD be forwarded, if the buffer were ever short
        for cap in 0..HEADER_SIZE_MAX {
            let mut rets = Vec::new();
            for api in [&c, &r] {
                let mut ctx: *mut c_void = ptr::null_mut();
                assert_eq!((api.create_cctx)(&mut ctx, LZ4F_VERSION), 0);
                let mut buf = vec![0u8; HEADER_SIZE_MAX];
                let ret = (api.compress_begin)(ctx, buf.as_mut_ptr() as *mut c_void, cap, ptr::null());
                (api.free_cctx)(ctx);
                rets.push(ret);
            }
            same_and_is(
                &format!("row176 compressBegin(dstCapacity={cap})"),
                rets[0],
                rets[1],
                err(11),
            );
        }
        // and at exactly 19 it succeeds
        let mut rets = Vec::new();
        for api in [&c, &r] {
            let mut ctx: *mut c_void = ptr::null_mut();
            assert_eq!((api.create_cctx)(&mut ctx, LZ4F_VERSION), 0);
            let mut buf = vec![0u8; HEADER_SIZE_MAX];
            let ret = (api.compress_begin)(
                ctx,
                buf.as_mut_ptr() as *mut c_void,
                HEADER_SIZE_MAX,
                ptr::null(),
            );
            (api.free_cctx)(ctx);
            rets.push(ret);
        }
        same_and_is("row176 compressBegin(dstCapacity=19)", rets[0], rets[1], 7);
    }
}

// ===========================================================================
// Row 177 — LZ4F_writeOpen: short write of the frame header -> err(22)
// ===========================================================================

#[test]
fn err_177_writeopen_short_header_write_is_io_write() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(177);

        // (a) /dev/full, unbuffered so even a 7-byte header reaches the device.
        for _ in 0..4 {
            let a = write_to_dev_full(&c, false, None, &[]);
            let b = write_to_dev_full(&r, false, None, &[]);
            same_write_out("row177 /dev/full", &a, &b);
            same_and_is("row177 writeOpen on /dev/full", a.open, b.open, err(22));
            assert!(a.writes.is_empty() && a.close.is_none());
        }

        // (b) an fmemopen stream too small for the header, for every header
        // length lz4file can produce (7, 11, 15, 19 bytes).
        for (name, prefs) in [
            ("default (7-byte header)", None),
            ("dictID (11)", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.dictID = 0xDEAD_BEEF;
                Some(p)
            }),
            ("contentSize (15)", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentSize = 123_456;
                Some(p)
            }),
            ("contentSize+dictID (19)", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentSize = 999;
                p.frameInfo.dictID = 7;
                Some(p)
            }),
        ] {
            let hlen = header_len(&c, &r, prefs.as_ref());
            for cap in 1..hlen {
                let a = write_to_mem(&c, cap, prefs.as_ref(), &[]);
                let b = write_to_mem(&r, cap, prefs.as_ref(), &[]);
                same_write_out(&format!("row177 {name} cap={cap}"), &a, &b);
                same_and_is(
                    &format!("row177 {name}: writeOpen with cap={cap} < hlen={hlen}"),
                    a.open,
                    b.open,
                    err(22),
                );
                assert!(a.writes.is_empty() && a.close.is_none());
            }
            // exactly enough room for the header: writeOpen succeeds
            let a = write_to_mem(&c, hlen, prefs.as_ref(), &[]);
            let b = write_to_mem(&r, hlen, prefs.as_ref(), &[]);
            same_write_out(&format!("row177 {name} cap=hlen"), &a, &b);
            same_and_is(
                &format!("row177 {name}: writeOpen with cap == hlen must succeed"),
                a.open,
                b.open,
                0,
            );
        }

        // (c) randomized: a random too-small capacity, random preferences.
        for iter in 0..20 {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB][rng.below(5)];
            p.frameInfo.contentSize = if rng.below(2) == 0 { 0 } else { 4242 };
            p.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() | 1 };
            p.compressionLevel = [0i32, 1, 9, 12][rng.below(4)];
            let hlen = header_len(&c, &r, Some(&p));
            let cap = rng.range(1, hlen - 1);
            let a = write_to_mem(&c, cap, Some(&p), &[]);
            let b = write_to_mem(&r, cap, Some(&p), &[]);
            same_write_out(&format!("row177 iter={iter} cap={cap} hlen={hlen}"), &a, &b);
            same_and_is(
                &format!("row177 iter={iter}: cap={cap} < hlen={hlen}"),
                a.open,
                b.open,
                err(22),
            );
        }
    }
}

// ===========================================================================
// Row 178 — LZ4F_write: lz4fWrite == NULL or buf == NULL -> err(21)
// ===========================================================================

#[test]
fn err_178_write_null_params() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(178);
        let (sc, sr) = scratch_pair("r178");
        let fpc = open_file(sc.path(), "wb");
        let fpr = open_file(sr.path(), "wb");
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        assert_eq!((c.wopen)(&mut hc, fpc, ptr::null()), 0);
        assert_eq!((r.wopen)(&mut hr, fpr, ptr::null()), 0);

        let mut sizes = vec![0usize, 1, 2, 100, 65535, 65536, 65537, usize::MAX];
        for _ in 0..24 {
            sizes.push(rng.range(1, 4_000_000));
        }
        for &n in &sizes {
            // valid handle, NULL buffer (both n == 0 and n != 0)
            let a = (c.write)(hc, ptr::null(), n);
            let b = (r.write)(hr, ptr::null(), n);
            same_and_is(&format!("row178 write(buf=NULL,size={n})"), a, b, err(21));
            // NULL handle, valid buffer
            let data = [0xA5u8; 8];
            let a = (c.write)(ptr::null_mut(), data.as_ptr() as *const c_void, n);
            let b = (r.write)(ptr::null_mut(), data.as_ptr() as *const c_void, n);
            same_and_is(&format!("row178 write(handle=NULL,size={n})"), a, b, err(21));
            // both NULL
            let a = (c.write)(ptr::null_mut(), ptr::null(), n);
            let b = (r.write)(ptr::null_mut(), ptr::null(), n);
            same_and_is(&format!("row178 write(both NULL,size={n})"), a, b, err(21));
        }

        // The rejected calls must not have latched errCode: a real write and a
        // real close still succeed and the frame is valid.
        let payload = gen(&mut rng, Shape::TextLike, 12_345);
        let a = (c.write)(hc, payload.as_ptr() as *const c_void, payload.len());
        let b = (r.write)(hr, payload.as_ptr() as *const c_void, payload.len());
        same_and_is("row178 write after rejected calls", a, b, payload.len());
        let a = (c.wclose)(hc);
        let b = (r.wclose)(hr);
        same("row178 writeClose after rejected calls", a, b);
        assert_ok("row178 writeClose", Some(a));
        fclose(fpc);
        fclose(fpr);
        same_full_buffers("row178 frame bytes", &sc.bytes(), &sr.bytes());
        let ra = read_all(&c, sc.path(), 4096, usize::MAX);
        let rb = read_all(&r, sr.path(), 4096, usize::MAX);
        assert_eq!(ra.data, payload, "row178: C frame does not round-trip");
        assert_eq!(rb.data, payload, "row178: Rust frame does not round-trip");
    }
}

// ===========================================================================
// Row 179 — LZ4F_write: LZ4F_compressUpdate error, latched in errCode
// ===========================================================================

/// NOT FORCEABLE through lz4file, and this is a property of the code rather
/// than of the test: `LZ4F_compressUpdate` has exactly two failure modes
/// (lz4frame.c:1005-1010) and lz4file can trigger neither.
///   * `cStage != 1` -> `compressionState_uninitialized`: `LZ4F_writeOpen`
///     always leaves the cctx in stage 1 via `LZ4F_compressBegin`, and
///     `compressUpdate`/`flush` keep it there; the only thing that resets the
///     stage is `LZ4F_compressEnd`, which lz4file calls from `LZ4F_writeClose`
///     immediately before freeing the state.
///   * `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)`
///     -> `dstMaxSize_tooSmall`: lz4file's `dstBuf` is exactly
///     `LZ4F_compressBound(maxWriteSize, prefs)` and it caps every chunk at
///     `maxWriteSize`. The internal bound only charges an extra partial block
///     for the buffered bytes when `flush` is set (i.e. `autoFlush == 1`), and
///     with `autoFlush == 1` nothing is ever left buffered, so the requirement
///     never exceeds the allocated size.
/// So `errCode` can only ever be latched by the `io_write` sibling (row 180).
/// This test therefore (1) shows the `compressUpdate` call site cannot fail
/// over a wide sweep of chunk sizes, block sizes and autoFlush settings, and
/// (2) pins the LATCHING contract observably: once an error is latched,
/// subsequent writes keep failing and `LZ4F_writeClose` returns
/// `LZ4F_OK_NoError` (row 184).
#[test]
fn err_179_write_compressupdate_error_is_latched() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(179);

        // 1. the compressUpdate call site never errors
        for (bsid, mws) in [
            (LZ4F_DEFAULT, 64 * 1024usize),
            (LZ4F_MAX64KB, 64 * 1024),
            (LZ4F_MAX256KB, 256 * 1024),
        ] {
            for autoflush in [0u32, 1] {
                for bcrc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                    let mut p = prefs_with(bsid);
                    p.autoFlush = autoflush;
                    p.frameInfo.blockChecksumFlag = bcrc;
                    p.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;

                    // chunk patterns that leave partial data buffered and then
                    // submit a full maxWriteSize chunk on top of it.
                    let big = gen(&mut rng, Shape::Incompressible, 2 * mws + 17);
                    let patterns: Vec<Vec<usize>> = vec![
                        vec![1, mws],
                        vec![mws - 1, mws],
                        vec![mws / 2, mws, mws / 3, mws],
                        vec![mws + 1, mws + 1],
                        vec![17, 1, mws, 1, mws - 1],
                        vec![2 * mws + 17],
                    ];
                    for pat in patterns {
                        let chunks: Vec<&[u8]> = pat.iter().map(|&n| &big[..n]).collect();
                        let (sc, sr) = scratch_pair("r179");
                        let a = write_all(&c, sc.path(), Some(&p), &chunks);
                        let b = write_all(&r, sr.path(), Some(&p), &chunks);
                        same_write_out(
                            &format!("row179 bsid={bsid} af={autoflush} bcrc={bcrc} pat={pat:?}"),
                            &a,
                            &b,
                        );
                        assert_eq!(a.open, 0);
                        assert_eq!(
                            a.writes, pat,
                            "row179 bsid={bsid} pat={pat:?}: LZ4F_write must never fail, got {:#x?}",
                            a.writes
                        );
                        assert_ok("row179 writeClose", a.close);
                        same_full_buffers("row179 frame bytes", &sc.bytes(), &sr.bytes());
                    }
                }
            }
        }

        // 2. the latching contract, forced through the reachable io_write path
        let payload = gen(&mut rng, Shape::Incompressible, 4 * 1024 * 1024);
        let mut outs = Vec::new();
        for api in [&c, &r] {
            let fp = open_file(Path::new("/dev/full"), "wb");
            let mut h: *mut c_void = ptr::null_mut();
            let open = (api.wopen)(&mut h, fp, ptr::null());
            assert_eq!(open, 0, "row179 {}: writeOpen on /dev/full (buffered)", api.tag);
            let w1 = (api.write)(h, payload.as_ptr() as *const c_void, payload.len());
            let w2 = (api.write)(h, payload.as_ptr() as *const c_void, 4096);
            let close = (api.wclose)(h);
            fclose(fp);
            outs.push((w1, w2, close));
        }
        same("row179 first write on /dev/full", outs[0].0, outs[1].0);
        same("row179 second write after latching", outs[0].1, outs[1].1);
        same("row179 writeClose after latching", outs[0].2, outs[1].2);
        assert_eq!(
            outs[0].0,
            err(22),
            "row179: the first failing write must report err(22), got {:#x}",
            outs[0].0
        );
        assert_eq!(
            outs[0].2, 0,
            "row179: writeClose after a latched error must return 0, got {:#x}",
            outs[0].2
        );
    }
}

// ===========================================================================
// Row 180 — LZ4F_write: short write of a compressed chunk -> err(22), latched
// ===========================================================================

#[test]
fn err_180_write_short_chunk_write_is_io_write() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(180);
        let hlen = header_len(&c, &r, None);

        // (a) autoFlush = 1 -> every LZ4F_write emits a block immediately, so a
        // stream with room for the header only fails on the first write.
        let mut p = LZ4F_preferences_t::default();
        p.autoFlush = 1;
        let hlen_af = header_len(&c, &r, Some(&p));
        for iter in 0..12 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = rng.range(1, 20_000);
            let payload = gen(&mut rng, shape, len);
            // capacities from "header only" up to a few bytes of slack: the
            // block never fits, so the write is always short.
            let cap = hlen_af + rng.below(4);
            let a = write_to_mem(&c, cap, Some(&p), &[&payload, &payload]);
            let b = write_to_mem(&r, cap, Some(&p), &[&payload, &payload]);
            same_write_out(&format!("row180a iter={iter} cap={cap} len={len}"), &a, &b);
            same_and_is(&format!("row180a iter={iter}: writeOpen"), a.open, b.open, 0);
            assert_eq!(
                a.writes.len(),
                2,
                "row180a: expected two write attempts, got {:#x?}",
                a.writes
            );
            same_and_is(
                &format!("row180a iter={iter}: first LZ4F_write"),
                a.writes[0],
                b.writes[0],
                err(22),
            );
            same_and_is(
                &format!("row180a iter={iter}: second LZ4F_write (already latched)"),
                a.writes[1],
                b.writes[1],
                err(22),
            );
            // row 184: the latched error makes writeClose return OK_NoError
            same_and_is(
                &format!("row180a iter={iter}: writeClose after the latch"),
                a.close.unwrap(),
                b.close.unwrap(),
                0,
            );
        }

        // (b) autoFlush = 0 -> nothing is emitted until a full block is
        // complete, so the short write happens on the chunk that crosses
        // maxWriteSize (64 KB by default).
        for iter in 0..6 {
            let payload = gen(&mut rng, Shape::Incompressible, 64 * 1024 + 1);
            let cap = hlen + rng.below(8);
            let a = write_to_mem(&c, cap, None, &[&payload]);
            let b = write_to_mem(&r, cap, None, &[&payload]);
            same_write_out(&format!("row180b iter={iter} cap={cap}"), &a, &b);
            same_and_is(&format!("row180b iter={iter}: writeOpen"), a.open, b.open, 0);
            same_and_is(
                &format!("row180b iter={iter}: LZ4F_write crossing the block size"),
                a.writes[0],
                b.writes[0],
                err(22),
            );
            same_and_is(
                &format!("row180b iter={iter}: writeClose after the latch"),
                a.close.unwrap(),
                b.close.unwrap(),
                0,
            );
        }

        // (c) /dev/full, buffered: the header slips through stdio's buffer but
        // the bulk data reaches the device and fails.
        let payload = gen(&mut rng, Shape::Incompressible, 4 * 1024 * 1024);
        let a = write_to_dev_full(&c, true, None, &[&payload]);
        let b = write_to_dev_full(&r, true, None, &[&payload]);
        same_write_out("row180c /dev/full", &a, &b);
        same_and_is("row180c writeOpen", a.open, b.open, 0);
        same_and_is("row180c LZ4F_write", a.writes[0], b.writes[0], err(22));
        same_and_is("row180c writeClose", a.close.unwrap(), b.close.unwrap(), 0);
    }
}

// ===========================================================================
// Row 181 — LZ4F_writeClose(NULL) -> err(21)
// ===========================================================================

#[test]
fn err_181_writeclose_null_handle() {
    unsafe {
        let (c, r) = apis();

        for _ in 0..64 {
            let a = (c.wclose)(ptr::null_mut());
            let b = (r.wclose)(ptr::null_mut());
            same_and_is("row181 writeClose(NULL)", a, b, err(21));
        }

        // Interleaved with a live handle: the NULL calls change nothing.
        let mut rng = Rng::new(181);
        for _ in 0..6 {
            let plen = rng.range(1, 20_000);
            let payload = gen(&mut rng, Shape::TextLike, plen);
            let (sc, sr) = scratch_pair("r181");
            for api in [&c, &r] {
                let path = if api.tag == "C" { sc.path() } else { sr.path() };
                let fp = open_file(path, "wb");
                let mut h: *mut c_void = ptr::null_mut();
                assert_eq!((api.wopen)(&mut h, fp, ptr::null()), 0);
                assert_eq!(
                    (api.wclose)(ptr::null_mut()),
                    err(21),
                    "row181 {}: writeClose(NULL) with a live handle",
                    api.tag
                );
                assert_eq!(
                    (api.write)(h, payload.as_ptr() as *const c_void, payload.len()),
                    payload.len()
                );
                let n = (api.wclose)(h);
                assert_ok("row181 real writeClose", Some(n));
                assert_eq!(
                    (api.wclose)(ptr::null_mut()),
                    err(21),
                    "row181 {}: writeClose(NULL) after a real close",
                    api.tag
                );
                fclose(fp);
            }
            same_full_buffers("row181 frame bytes", &sc.bytes(), &sr.bytes());
        }
    }
}

// ===========================================================================
// Row 182 — LZ4F_writeClose: LZ4F_compressEnd failed -> forwarded (goto out)
// ===========================================================================

#[test]
fn err_182_writeclose_compressend_failure_is_forwarded() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(182);

        // A declared frameInfo.contentSize that does not match the bytes
        // actually written makes LZ4F_compressEnd report
        // LZ4F_ERROR_frameSize_wrong (lz4frame.c:1235-1238) = err(14);
        // LZ4F_writeClose takes `goto out`, frees the state and returns it.
        for iter in 0..24 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let actual = rng.range(1, 40_000);
            let payload = gen(&mut rng, shape, actual);
            // a declared size different from `actual`, both larger and smaller
            let declared = match iter % 4 {
                0 => actual as u64 + 1,
                1 => (actual as u64).saturating_sub(1).max(1),
                2 => actual as u64 + rng.range(1, 1_000_000) as u64,
                _ => rng.range(1, actual.max(2)) as u64,
            };
            if declared == actual as u64 {
                continue;
            }
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentSize = declared;
            p.autoFlush = (iter % 2) as c_uint;
            p.frameInfo.contentChecksumFlag = (iter % 3 == 0) as c_uint;

            let (sc, sr) = scratch_pair("r182");
            let a = write_all(&c, sc.path(), Some(&p), &[&payload]);
            let b = write_all(&r, sr.path(), Some(&p), &[&payload]);
            same_write_out(
                &format!("row182 iter={iter} declared={declared} actual={actual}"),
                &a,
                &b,
            );
            same_and_is(&format!("row182 iter={iter}: writeOpen"), a.open, b.open, 0);
            assert_eq!(
                a.writes,
                vec![actual],
                "row182 iter={iter}: the writes themselves must succeed, got {:#x?}",
                a.writes
            );
            same_and_is(
                &format!("row182 iter={iter}: writeClose (declared {declared} != actual {actual})"),
                a.close.unwrap(),
                b.close.unwrap(),
                err(14),
            );
            // compressEnd failed before writing anything, so the file holds only
            // the header + the blocks emitted by LZ4F_write, identically.
            same_full_buffers("row182 frame bytes", &sc.bytes(), &sr.bytes());
        }

        // Control: the exact declared size closes cleanly.
        for _ in 0..4 {
            let plen = rng.range(1, 40_000);
            let payload = gen(&mut rng, Shape::TextLike, plen);
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentSize = plen as u64;
            let (sc, sr) = scratch_pair("r182ok");
            let a = write_all(&c, sc.path(), Some(&p), &[&payload]);
            let b = write_all(&r, sr.path(), Some(&p), &[&payload]);
            same_write_out("row182 control", &a, &b);
            assert_ok("row182 control writeClose", a.close);
            same_full_buffers("row182 control bytes", &sc.bytes(), &sr.bytes());
            let ra = read_all(&c, sc.path(), 8192, usize::MAX);
            assert_eq!(ra.data, payload, "row182 control: no round-trip");
        }
    }
}

// ===========================================================================
// Row 183 — LZ4F_writeClose: short write of the frame trailer -> err(22)
// ===========================================================================

#[test]
fn err_183_writeclose_short_trailer_write_is_io_write() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(183);

        for ccrc in [LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
            for iter in 0..6 {
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let plen = rng.range(64, 30_000);
                let payload = gen(&mut rng, shape, plen);
                // autoFlush so the data is written by LZ4F_write and ONLY the
                // trailer is left for LZ4F_writeClose.
                let mut p = LZ4F_preferences_t::default();
                p.autoFlush = 1;
                p.frameInfo.contentChecksumFlag = ccrc;

                // Probe the exact geometry on a real file.
                let (sp, _unused) = scratch_pair("r183probe");
                let probe = write_all(&c, sp.path(), Some(&p), &[&payload]);
                let trailer = assert_ok("row183 probe writeClose", probe.close);
                let full = sp.bytes().len();
                assert!(trailer >= 4 && full > trailer);
                let data_end = full - trailer;

                // Every capacity that holds the header + data but not the whole
                // trailer must make writeClose report err(22).
                for cap in data_end..full {
                    let a = write_to_mem(&c, cap, Some(&p), &[&payload]);
                    let b = write_to_mem(&r, cap, Some(&p), &[&payload]);
                    same_write_out(
                        &format!("row183 ccrc={ccrc} iter={iter} cap={cap} full={full}"),
                        &a,
                        &b,
                    );
                    same_and_is("row183 writeOpen", a.open, b.open, 0);
                    assert_eq!(
                        a.writes,
                        vec![plen],
                        "row183 cap={cap}: the data writes must succeed, got {:#x?}",
                        a.writes
                    );
                    same_and_is(
                        &format!("row183 ccrc={ccrc} cap={cap}: writeClose trailer short write"),
                        a.close.unwrap(),
                        b.close.unwrap(),
                        err(22),
                    );
                }

                // Exactly enough room for everything: writeClose succeeds and
                // reports the trailer length.
                let a = write_to_mem(&c, full, Some(&p), &[&payload]);
                let b = write_to_mem(&r, full, Some(&p), &[&payload]);
                same_write_out(&format!("row183 cap=full={full}"), &a, &b);
                same_and_is(
                    &format!("row183 cap=full={full}: writeClose"),
                    a.close.unwrap(),
                    b.close.unwrap(),
                    trailer,
                );
            }
        }
    }
}

// ===========================================================================
// Row 184 — LZ4F_writeClose after a latched LZ4F_write error -> 0 (OK_NoError)
// ===========================================================================

#[test]
fn err_184_writeclose_after_latched_write_error_returns_ok() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(184);
        let mut p = LZ4F_preferences_t::default();
        p.autoFlush = 1;
        let hlen_af = header_len(&c, &r, Some(&p));

        // (a) fmemopen with room for the header only: the first write latches
        // err(22), and writeClose must then return EXACTLY 0 — the earlier
        // error is silently dropped and no trailer is even attempted.
        for iter in 0..16 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let plen = rng.range(1, 30_000);
            let payload = gen(&mut rng, shape, plen);
            let nwrites = 1 + rng.below(3);
            let chunks: Vec<&[u8]> = (0..nwrites).map(|_| &payload[..]).collect();
            let a = write_to_mem(&c, hlen_af, Some(&p), &chunks);
            let b = write_to_mem(&r, hlen_af, Some(&p), &chunks);
            same_write_out(&format!("row184a iter={iter} nwrites={nwrites}"), &a, &b);
            same_and_is("row184a writeOpen", a.open, b.open, 0);
            for (i, (&x, &y)) in a.writes.iter().zip(b.writes.iter()).enumerate() {
                same_and_is(&format!("row184a write #{i}"), x, y, err(22));
            }
            assert_eq!(
                a.close.unwrap(),
                0,
                "row184a: writeClose after a latched error must be exactly 0, got {:#x}",
                a.close.unwrap()
            );
            assert_eq!(
                b.close.unwrap(),
                0,
                "row184a: Rust writeClose after a latched error must be exactly 0, got {:#x}",
                b.close.unwrap()
            );
        }

        // (b) the same through /dev/full (buffered stdio), with a payload big
        // enough to defeat the stdio buffer.
        for iter in 0..3 {
            let payload = gen(&mut rng, Shape::Incompressible, 4 * 1024 * 1024);
            let a = write_to_dev_full(&c, true, None, &[&payload]);
            let b = write_to_dev_full(&r, true, None, &[&payload]);
            same_write_out(&format!("row184b iter={iter}"), &a, &b);
            same_and_is("row184b writeOpen", a.open, b.open, 0);
            same_and_is("row184b LZ4F_write", a.writes[0], b.writes[0], err(22));
            assert_eq!(
                a.close.unwrap(),
                0,
                "row184b: writeClose after a latched error must be exactly 0, got {:#x}",
                a.close.unwrap()
            );
            assert_eq!(b.close.unwrap(), 0, "row184b: Rust writeClose must be exactly 0");
        }

        // (c) contrast: with NO error latched, writeClose returns the trailer
        // length (non-zero), not 0 — so the 0 above really is the row-184
        // shortcut and not just "success".
        for _ in 0..4 {
            let plen = rng.range(1, 20_000);
            let payload = gen(&mut rng, Shape::TextLike, plen);
            let (sc, sr) = scratch_pair("r184c");
            let a = write_all(&c, sc.path(), Some(&p), &[&payload]);
            let b = write_all(&r, sr.path(), Some(&p), &[&payload]);
            same_write_out("row184c control", &a, &b);
            let n = assert_ok("row184c writeClose", a.close);
            assert!(
                n > 0,
                "row184c: an un-latched writeClose must report the trailer length"
            );
            same("row184c writeClose", a.close.unwrap(), b.close.unwrap());
        }
    }
}
