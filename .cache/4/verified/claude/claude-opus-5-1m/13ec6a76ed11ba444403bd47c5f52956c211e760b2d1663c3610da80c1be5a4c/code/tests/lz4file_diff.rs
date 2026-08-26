//! Differential tests for ALL SIX exported symbols of `lz4file.c`:
//! `LZ4F_writeOpen`, `LZ4F_write`, `LZ4F_writeClose`,
//! `LZ4F_readOpen`,  `LZ4F_read`,  `LZ4F_readClose`.
//!
//! Real temp files are used (under `$TMPDIR`), and `FILE*` handles come from
//! libc's `fopen`/`fclose`/`fflush`/`rewind`, declared here — libc is not the
//! library under test.
//!
//! Strategy
//! --------
//! *write side*: run the identical `writeOpen`/`write`.../`writeClose` sequence
//!   against the C library and against the Rust library, each into its own temp
//!   file, then compare every return value AND the two resulting files byte for
//!   byte.
//! *read side*:  take a reference `.lz4` file, read it with both libraries using
//!   the identical sequence of `LZ4F_read` sizes, and compare every return value,
//!   the 0xAA-prefilled destination buffers in full, and the decoded content
//!   against the original bytes.  Files produced by the C writer AND by the Rust
//!   writer are both used as inputs, in both directions.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc (NOT the library under test)
// ---------------------------------------------------------------------------
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(fp: *mut c_void) -> c_int;
    fn fflush(fp: *mut c_void) -> c_int;
    fn rewind(fp: *mut c_void);
}

// ---------------------------------------------------------------------------
// Signatures — verified against c_src/src/lz4file.c / c_src/include/lz4file.h
// ---------------------------------------------------------------------------
type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut c_void) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut c_void) -> usize;

// frame-level helpers used to build reference/corrupt inputs
type FnCompressFrameBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;

const SENTINEL: u8 = 0xAA;

struct FileFns {
    write_open: FnWriteOpen,
    write: FnWrite,
    write_close: FnWriteClose,
    read_open: FnReadOpen,
    read: FnRead,
    read_close: FnReadClose,
    tag: &'static str,
}

struct Api {
    c: FileFns,
    r: FileFns,
    c_frame_bound: FnCompressFrameBound,
    c_frame: FnCompressFrame,
}

fn api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| {
        let l = libs();
        let c = FileFns {
            write_open: l.c.sym("LZ4F_writeOpen"),
            write: l.c.sym("LZ4F_write"),
            write_close: l.c.sym("LZ4F_writeClose"),
            read_open: l.c.sym("LZ4F_readOpen"),
            read: l.c.sym("LZ4F_read"),
            read_close: l.c.sym("LZ4F_readClose"),
            tag: "C",
        };
        let r = FileFns {
            write_open: l.rust.sym("LZ4F_writeOpen"),
            write: l.rust.sym("LZ4F_write"),
            write_close: l.rust.sym("LZ4F_writeClose"),
            read_open: l.rust.sym("LZ4F_readOpen"),
            read: l.rust.sym("LZ4F_read"),
            read_close: l.rust.sym("LZ4F_readClose"),
            tag: "Rust",
        };
        // Sanity: the two libraries must be distinct code objects, otherwise the
        // whole differential setup would be vacuous.
        assert_ne!(
            c.write as usize, r.write as usize,
            "C and Rust LZ4F_write resolved to the same address"
        );
        assert_ne!(
            c.read as usize, r.read as usize,
            "C and Rust LZ4F_read resolved to the same address"
        );
        Api {
            c,
            r,
            c_frame_bound: l.c.sym("LZ4F_compressFrameBound"),
            c_frame: l.c.sym("LZ4F_compressFrame"),
        }
    })
}

fn ret_str(r: usize) -> String {
    if lz4f_is_error(r) {
        format!("ERROR({})", lz4f_error_code(r))
    } else {
        format!("{}", r)
    }
}

// ---------------------------------------------------------------------------
// Temp files
// ---------------------------------------------------------------------------

struct TmpFile {
    path: String,
    cpath: std::ffi::CString,
}

impl TmpFile {
    fn new(name: &str) -> Self {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let dir = dir.trim_end_matches('/').to_string();
        let path = format!("{}/lz4file_diff_{}_{}.lz4", dir, std::process::id(), name);
        let _ = std::fs::remove_file(&path);
        let cpath = std::ffi::CString::new(path.clone()).unwrap();
        TmpFile { path, cpath }
    }
    fn open(&self, mode: &str) -> *mut c_void {
        let m = std::ffi::CString::new(mode).unwrap();
        let fp = unsafe { fopen(self.cpath.as_ptr(), m.as_ptr()) };
        assert!(!fp.is_null(), "fopen({}, {}) failed", self.path, mode);
        fp
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap_or_else(|e| panic!("read {}: {}", self.path, e))
    }
    fn put(&self, data: &[u8]) {
        std::fs::write(&self.path, data).unwrap_or_else(|e| panic!("write {}: {}", self.path, e));
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Write driver
// ---------------------------------------------------------------------------

struct WriteLog {
    open_ret: usize,
    write_rets: Vec<usize>,
    close_ret: usize,
    /// Whether the state pointer was NULL after `LZ4F_writeOpen`.
    state_null_after_open: bool,
    file: Vec<u8>,
}

/// Run `writeOpen` / `write`* / `writeClose` on one library.
fn run_write(
    f: &FileFns,
    tmp: &TmpFile,
    mode: &str,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
) -> WriteLog {
    let fp = tmp.open(mode);
    let prefs_ptr = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => std::ptr::null(),
    };
    unsafe {
        let mut st: *mut c_void = std::ptr::null_mut();
        let open_ret = (f.write_open)(&mut st, fp, prefs_ptr);
        let state_null_after_open = st.is_null();
        let mut write_rets = Vec::new();
        if !lz4f_is_error(open_ret) && !st.is_null() {
            for ch in chunks {
                let r = (f.write)(
                    st,
                    ch.as_ptr() as *const c_void,
                    ch.len(),
                );
                write_rets.push(r);
                if lz4f_is_error(r) {
                    break;
                }
            }
        }
        // LZ4F_writeClose frees the state (and tolerates NULL).
        let close_ret = (f.write_close)(st);
        fflush(fp);
        fclose(fp);
        WriteLog {
            open_ret,
            write_rets,
            close_ret,
            state_null_after_open,
            file: tmp.bytes(),
        }
    }
}

/// Do the same write sequence on both libraries and require everything to match.
fn write_both(
    name: &str,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
    label: &str,
) -> Vec<u8> {
    let a = api();
    let tc = TmpFile::new(&format!("{}_c", name));
    let tr = TmpFile::new(&format!("{}_r", name));
    let lc = run_write(&a.c, &tc, "wb", prefs, chunks);
    let lr = run_write(&a.r, &tr, "wb", prefs, chunks);

    assert_eq!(
        ret_str(lc.open_ret),
        ret_str(lr.open_ret),
        "{}: LZ4F_writeOpen return",
        label
    );
    assert_eq!(
        lc.state_null_after_open, lr.state_null_after_open,
        "{}: state NULLness after LZ4F_writeOpen",
        label
    );
    assert_eq!(
        lc.write_rets.len(),
        lr.write_rets.len(),
        "{}: number of LZ4F_write calls made",
        label
    );
    for (i, (c, r)) in lc.write_rets.iter().zip(lr.write_rets.iter()).enumerate() {
        assert_eq!(
            ret_str(*c),
            ret_str(*r),
            "{}: LZ4F_write #{} return",
            label,
            i
        );
    }
    assert_eq!(
        ret_str(lc.close_ret),
        ret_str(lr.close_ret),
        "{}: LZ4F_writeClose return",
        label
    );
    assert_bytes_eq(
        &format!("{}: written file contents", label),
        &lc.file,
        &lr.file,
    );
    lc.file
}

// ---------------------------------------------------------------------------
// Read driver
// ---------------------------------------------------------------------------

struct ReadLog {
    open_ret: usize,
    state_null_after_open: bool,
    read_rets: Vec<usize>,
    close_ret: usize,
    buf: Vec<u8>,
    produced: usize,
}

/// Run `readOpen` / `read`* / `readClose` on one library, with `plan` cycled as
/// the requested read sizes.
fn run_read(f: &FileFns, tmp: &TmpFile, plan: &[usize], out_len: usize) -> ReadLog {
    let fp = tmp.open("rb");
    unsafe {
        let mut st: *mut c_void = std::ptr::null_mut();
        let open_ret = (f.read_open)(&mut st, fp);
        let state_null_after_open = st.is_null();
        let mut buf = vec![SENTINEL; out_len];
        let mut read_rets = Vec::new();
        let mut produced = 0usize;
        if !lz4f_is_error(open_ret) && !st.is_null() {
            let mut i = 0usize;
            loop {
                assert!(i < 20_000_000, "{}: runaway LZ4F_read loop", f.tag);
                let want = plan[i % plan.len()].min(out_len - produced);
                if want == 0 {
                    break;
                }
                let r = (f.read)(st, buf.as_mut_ptr().add(produced) as *mut c_void, want);
                read_rets.push(r);
                if lz4f_is_error(r) || r == 0 {
                    break;
                }
                produced += r;
                i += 1;
            }
        } else {
            // exercise LZ4F_read on the NULL state too
            read_rets.push((f.read)(st, buf.as_mut_ptr() as *mut c_void, 16));
        }
        let close_ret = (f.read_close)(st);
        fclose(fp);
        ReadLog {
            open_ret,
            state_null_after_open,
            read_rets,
            close_ret,
            buf,
            produced,
        }
    }
}

/// Read the same file with both libraries and require everything to match.
/// Returns the C log (identical to the Rust one by construction).
fn read_both(file: &[u8], plan: &[usize], out_len: usize, name: &str, label: &str) -> ReadLog {
    let a = api();
    let tc = TmpFile::new(&format!("{}_rc", name));
    let tr = TmpFile::new(&format!("{}_rr", name));
    tc.put(file);
    tr.put(file);
    let lc = run_read(&a.c, &tc, plan, out_len);
    let lr = run_read(&a.r, &tr, plan, out_len);

    assert_eq!(
        ret_str(lc.open_ret),
        ret_str(lr.open_ret),
        "{}: LZ4F_readOpen return",
        label
    );
    assert_eq!(
        lc.state_null_after_open, lr.state_null_after_open,
        "{}: state NULLness after LZ4F_readOpen",
        label
    );
    assert_eq!(
        lc.read_rets.len(),
        lr.read_rets.len(),
        "{}: number of LZ4F_read calls (C {:?} vs Rust {:?})",
        label,
        lc.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
        lr.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>()
    );
    for (i, (c, r)) in lc.read_rets.iter().zip(lr.read_rets.iter()).enumerate() {
        assert_eq!(
            ret_str(*c),
            ret_str(*r),
            "{}: LZ4F_read #{} return",
            label,
            i
        );
    }
    assert_eq!(
        ret_str(lc.close_ret),
        ret_str(lr.close_ret),
        "{}: LZ4F_readClose return",
        label
    );
    assert_eq!(lc.produced, lr.produced, "{}: total bytes read", label);
    assert_bytes_eq(&format!("{}: read buffer", label), &lc.buf, &lr.buf);
    lc
}

/// `LZ4F_readOpen` unconditionally does `fread(buf, 1, LZ4F_HEADER_SIZE_MAX, fp)`
/// and fails with `LZ4F_ERROR_io_read` unless it gets all 19 bytes — so any
/// `.lz4` file shorter than 19 bytes simply cannot be opened for reading.
/// This helper asserts the C-documented behaviour for both cases.
fn expect_readback(file: &[u8], plan: &[usize], data: &[u8], name: &str, label: &str) {
    let lc = read_both(file, plan, data.len() + 64, name, label);
    if file.len() >= LZ4F_HEADER_SIZE_MAX {
        assert_bytes_eq(
            &format!("{}: readback content", label),
            data,
            &lc.buf[..lc.produced],
        );
    } else {
        assert_eq!(
            lz4f_error_code(lc.open_ret),
            err::ERROR_io_read,
            "{}: a {}-byte file must fail LZ4F_readOpen with io_read",
            label,
            file.len()
        );
        assert_eq!(lc.produced, 0, "{}: nothing may be produced", label);
    }
}

#[allow(clippy::too_many_arguments)]
fn prefs_of(
    bsid: c_int,
    bmode: c_int,
    cc: c_int,
    bc: c_int,
    content_size: u64,
    dict_id: c_uint,
    level: c_int,
    auto_flush: c_uint,
) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = bmode;
    p.frameInfo.contentChecksumFlag = cc;
    p.frameInfo.blockChecksumFlag = bc;
    p.frameInfo.contentSize = content_size;
    p.frameInfo.dictID = dict_id;
    p.frameInfo.frameType = LZ4F_frame;
    p.compressionLevel = level;
    p.autoFlush = auto_flush;
    p
}

/// One-shot frame built with the C library (used as a canonical reader input).
fn c_compress_frame(data: &[u8], prefs: &LZ4F_preferences_t) -> Vec<u8> {
    let a = api();
    unsafe {
        let cap = (a.c_frame_bound)(data.len(), prefs as *const LZ4F_preferences_t);
        let mut buf = vec![0u8; cap.max(32)];
        let n = (a.c_frame)(
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            data.as_ptr() as *const c_void,
            data.len(),
            prefs as *const LZ4F_preferences_t,
        );
        assert!(!lz4f_is_error(n), "LZ4F_compressFrame: {}", ret_str(n));
        buf.truncate(n);
        buf
    }
}

fn skippable_frame(magic: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + payload.len());
    v.extend_from_slice(&magic.to_le_bytes());
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

// ===========================================================================
// 1. write side: full preferences matrix
// ===========================================================================

#[test]
fn write_preferences_matrix() {
    let mut rng = Rng::new(0xF11E_0001);
    let mut n = 0usize;

    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &cc in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
                for &bc in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                    for &with_csz in &[false, true] {
                        for &did in &[0u32, 0x0BAD_F00Du32] {
                            for &af in &[0u32, 1u32] {
                                let shape = n % N_SHAPES;
                                let len = [0usize, 1, 2, 700, 5000, 40_000][n % 6];
                                let data = gen_shape(&mut rng, shape, len);
                                let prefs = prefs_of(
                                    bsid,
                                    bmode,
                                    cc,
                                    bc,
                                    if with_csz { data.len() as u64 } else { 0 },
                                    did,
                                    [0i32, 1, 3, 9, 12][n % 5],
                                    af,
                                );
                                let label = format!(
                                    "writeprefs bsid={} bmode={} cc={} bc={} csz={} did={:#x} af={} shape={} len={}",
                                    bsid, bmode, cc, bc, with_csz, did, af, shape_name(shape), len
                                );
                                let file =
                                    write_both("prefs", Some(&prefs), &[&data[..]], &label);
                                // ... and the produced file must decode back.
                                expect_readback(
                                    &file,
                                    &[data.len().max(1) + 64],
                                    &data,
                                    "prefs",
                                    &format!("{} [readback]", label),
                                );
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 5 * 2 * 2 * 2 * 2 * 2 * 2);

    // NULL prefsPtr => maxWriteSize defaults to 64 KB
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 100, 65_535, 65_536, 65_537, 200_000] {
            let data = gen_shape(&mut rng, shape, len);
            let label = format!("writeprefs NULL shape={} len={}", shape_name(shape), len);
            let file = write_both("prefsnull", None, &[&data[..]], &label);
            expect_readback(
                &file,
                &[64 * 1024],
                &data,
                "prefsnull",
                &format!("{} [readback]", label),
            );
        }
    }
}

// ===========================================================================
// 2. write side: call-size axes (the internal maxWriteSize chunk loop)
// ===========================================================================

#[test]
fn write_call_size_axes() {
    let mut rng = Rng::new(0xF11E_0002);

    // blockSizeID 4 => maxWriteSize == 64 KB, so writes bigger than that are
    // split by LZ4F_write's internal loop.
    for &bsid in &[4, 5] {
        let max_write = if bsid == 4 { 65_536usize } else { 262_144 };
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &af in &[0u32, 1u32] {
                for shape in 0..N_SHAPES {
                    let total = max_write * 2 + 1234;
                    let data = gen_shape(&mut rng, shape, total);
                    for &step in &[
                        1usize,
                        2,
                        3,
                        7,
                        1000,
                        max_write - 1,
                        max_write,
                        max_write + 1,
                        total,
                    ] {
                        // byte-at-a-time writes of a 500 KB payload are too slow
                        if step < 1000 && total > 20_000 {
                            continue;
                        }
                        let chunks: Vec<&[u8]> = data.chunks(step).collect();
                        let label = format!(
                            "writesize bsid={} bmode={} af={} shape={} total={} step={} nchunks={}",
                            bsid,
                            bmode,
                            af,
                            shape_name(shape),
                            total,
                            step,
                            chunks.len()
                        );
                        let prefs = prefs_of(bsid, bmode, 1, 1, 0, 0, 0, af);
                        let file = write_both("wsize", Some(&prefs), &chunks, &label);
                        expect_readback(
                            &file,
                            &[max_write],
                            &data,
                            "wsize",
                            &format!("{} [readback]", label),
                        );
                    }
                }
            }
        }
    }

    // Fine-grained writes on a small payload (covers step 1,2,3,7).
    for shape in 0..N_SHAPES {
        let data = gen_shape(&mut rng, shape, 3000);
        for &step in &[1usize, 2, 3, 7, 999] {
            for &af in &[0u32, 1u32] {
                let chunks: Vec<&[u8]> = data.chunks(step).collect();
                let prefs = prefs_of(4, LZ4F_blockLinked, 1, 1, 0, 0, 0, af);
                let label = format!(
                    "writesize-small shape={} step={} af={}",
                    shape_name(shape),
                    step,
                    af
                );
                let file = write_both("wsizes", Some(&prefs), &chunks, &label);
                expect_readback(
                    &file,
                    &[4096],
                    &data,
                    "wsizes",
                    &format!("{} [readback]", label),
                );
            }
        }
    }

    // zero-length writes, interleaved with real ones
    for shape in 0..N_SHAPES {
        let data = gen_shape(&mut rng, shape, 5000);
        let empty: &[u8] = &[];
        let chunks: Vec<&[u8]> = vec![
            empty,
            &data[..0],
            &data[..1000],
            empty,
            &data[1000..1000],
            &data[1000..5000],
            empty,
        ];
        let prefs = prefs_of(4, LZ4F_blockLinked, 1, 0, data.len() as u64, 0, 0, 0);
        let label = format!("write zero-length interleaved shape={}", shape_name(shape));
        let file = write_both("wzero", Some(&prefs), &chunks, &label);
        expect_readback(
            &file,
            &[1024],
            &data,
            "wzero",
            &format!("{} [readback]", label),
        );
    }

    // no write at all: writeOpen immediately followed by writeClose
    for &bsid in &[0, 4, 5, 6, 7] {
        for &cc in &[0, 1] {
            let prefs = prefs_of(bsid, LZ4F_blockLinked, cc, 0, 0, 0, 0, 0);
            let label = format!("write empty frame bsid={} cc={}", bsid, cc);
            let file = write_both("wempty", Some(&prefs), &[], &label);
            expect_readback(&file, &[4096], &[], "wempty", &format!("{} [readback]", label));
        }
    }
}

// ===========================================================================
// 3. write side: contentSize enforcement (LZ4F_compressEnd -> frameSize_wrong)
// ===========================================================================

#[test]
fn write_content_size_enforcement() {
    let mut rng = Rng::new(0xF11E_0003);
    let data = gen_shape(&mut rng, 3, 4321);

    // exact => success
    let prefs = prefs_of(4, LZ4F_blockLinked, 1, 1, data.len() as u64, 0, 0, 1);
    let label = "contentSize exact";
    let file = write_both("csz_ok", Some(&prefs), &[&data[..]], label);
    expect_readback(&file, &[8192], &data, "csz_ok", "contentSize exact read");

    // mismatching => LZ4F_writeClose must report frameSize_wrong identically
    for &declared in &[1u64, (data.len() as u64) - 1, (data.len() as u64) + 1, 1 << 40] {
        let prefs = prefs_of(4, LZ4F_blockLinked, 1, 1, declared, 0, 0, 1);
        let label = format!("contentSize declared={} actual={}", declared, data.len());
        let a = api();
        let tc = TmpFile::new("csz_bad_c");
        let tr = TmpFile::new("csz_bad_r");
        let lc = run_write(&a.c, &tc, "wb", Some(&prefs), &[&data[..]]);
        let lr = run_write(&a.r, &tr, "wb", Some(&prefs), &[&data[..]]);
        assert_eq!(
            ret_str(lc.open_ret),
            ret_str(lr.open_ret),
            "{}: writeOpen",
            label
        );
        assert_eq!(
            lc.write_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
            lr.write_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
            "{}: write returns",
            label
        );
        assert_eq!(
            ret_str(lc.close_ret),
            ret_str(lr.close_ret),
            "{}: writeClose return",
            label
        );
        assert_eq!(
            lz4f_error_code(lc.close_ret),
            err::ERROR_frameSize_wrong,
            "{}: expected frameSize_wrong",
            label
        );
        assert_bytes_eq(&format!("{}: file bytes", label), &lc.file, &lr.file);
    }
}

// ===========================================================================
// 4. read side: read-size axes over reference frames
// ===========================================================================

#[test]
fn read_size_axes() {
    let mut rng = Rng::new(0xF11E_0004);

    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for shape in 0..N_SHAPES {
                        let len = [0usize, 1, 19, 500, 70_000][shape % 5];
                        let data = gen_shape(&mut rng, shape, len);
                        // reference file built with LZ4F_compressFrame (C)
                        let prefs =
                            prefs_of(bsid, bmode, cc, bc, data.len() as u64, 0x99, 0, 1);
                        let file = c_compress_frame(&data, &prefs);
                        for &plan in &[
                            &[1usize][..],
                            &[2][..],
                            &[3][..],
                            &[7][..],
                            &[100][..],
                            &[4096][..],
                            &[65_536][..],
                            &[1 << 20][..],
                            &[1, 2, 3, 4, 5, 100, 1][..],
                            &[7, 1, 4095, 2][..],
                        ] {
                            if plan[0] < 8 && len > 5000 {
                                continue;
                            }
                            let label = format!(
                                "readsize bsid={} bmode={} cc={} bc={} shape={} len={} plan={:?}",
                                bsid,
                                bmode,
                                cc,
                                bc,
                                shape_name(shape),
                                len,
                                plan
                            );
                            expect_readback(&file, plan, &data, "rsize", &label);
                        }
                    }
                }
            }
        }
    }

    // randomised read sizes
    for iter in 0..60u32 {
        let shape = (iter as usize) % N_SHAPES;
        let len = rng.range(0, 120_000);
        let data = gen_shape(&mut rng, shape, len);
        let bsid = [0, 4, 5, 6, 7][(iter as usize) % 5];
        let prefs = prefs_of(
            bsid,
            if rng.bool() {
                LZ4F_blockLinked
            } else {
                LZ4F_blockIndependent
            },
            rng.bool() as c_int,
            rng.bool() as c_int,
            if rng.bool() { data.len() as u64 } else { 0 },
            rng.next_u32(),
            0,
            1,
        );
        let file = c_compress_frame(&data, &prefs);
        let mut plan: Vec<usize> = Vec::new();
        for _ in 0..12 {
            plan.push(rng.range(1, 20_000));
        }
        let label = format!(
            "readrand iter={} shape={} len={} bsid={} plan={:?}",
            iter,
            shape_name(shape),
            len,
            bsid,
            plan
        );
        expect_readback(&file, &plan, &data, "rrand", &label);
    }
}

// ===========================================================================
// 5. cross: write with one library, read with the other
// ===========================================================================

#[test]
fn write_read_cross_libraries() {
    let a = api();
    let mut rng = Rng::new(0xF11E_0005);

    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for shape in 0..N_SHAPES {
                let data = gen_shape(&mut rng, shape, 60_000);
                let prefs = prefs_of(bsid, bmode, 1, 1, data.len() as u64, 0, 0, 0);

                // written by C, then by Rust (files already asserted identical)
                let tc = TmpFile::new("cross_c");
                let tr = TmpFile::new("cross_r");
                let lc = run_write(&a.c, &tc, "wb", Some(&prefs), &[&data[..]]);
                let lr = run_write(&a.r, &tr, "wb", Some(&prefs), &[&data[..]]);
                let label = format!(
                    "cross bsid={} bmode={} shape={}",
                    bsid,
                    bmode,
                    shape_name(shape)
                );
                assert_eq!(
                    ret_str(lc.close_ret),
                    ret_str(lr.close_ret),
                    "{}: writeClose",
                    label
                );
                assert_bytes_eq(&format!("{}: files", label), &lc.file, &lr.file);

                // C writer file -> Rust reader, and Rust writer file -> C reader
                for &plan in &[&[7usize][..], &[4096][..], &[1 << 20][..], &[1, 3, 9999][..]] {
                    for (src_tag, src_file) in [("C-written", &lc.file), ("Rust-written", &lr.file)]
                    {
                        expect_readback(
                            src_file,
                            plan,
                            &data,
                            "cross",
                            &format!("{} [{} plan={:?}]", label, src_tag, plan),
                        );
                    }
                }

                // Single FILE* re-used: write, rewind, read back with the same handle.
                for f in [&a.c, &a.r] {
                    let t = TmpFile::new(&format!("rewind_{}", f.tag));
                    let fp = t.open("w+b");
                    unsafe {
                        let mut st: *mut c_void = std::ptr::null_mut();
                        let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
                        assert!(!lz4f_is_error(o), "{}: writeOpen", f.tag);
                        let w = (f.write)(st, data.as_ptr() as *const c_void, data.len());
                        assert_eq!(w, data.len(), "{}: write return", f.tag);
                        let c = (f.write_close)(st);
                        assert!(!lz4f_is_error(c), "{}: writeClose", f.tag);
                        fflush(fp);
                        rewind(fp);

                        let mut rst: *mut c_void = std::ptr::null_mut();
                        let ro = (f.read_open)(&mut rst, fp);
                        assert!(!lz4f_is_error(ro), "{}: readOpen after rewind", f.tag);
                        let mut buf = vec![SENTINEL; data.len() + 64];
                        let n = (f.read)(rst, buf.as_mut_ptr() as *mut c_void, buf.len());
                        assert!(!lz4f_is_error(n), "{}: read after rewind", f.tag);
                        assert_eq!(n, data.len(), "{}: bytes read after rewind", f.tag);
                        assert_bytes_eq(
                            &format!("{} rewind roundtrip content", f.tag),
                            &data,
                            &buf[..n],
                        );
                        assert!(!lz4f_is_error((f.read_close)(rst)));
                        fclose(fp);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 6. read side: multi-frame, skippable frames, short reads at EOF
// ===========================================================================

#[test]
fn read_multiframe_skippable_and_eof() {
    let mut rng = Rng::new(0xF11E_0006);

    for shape in 0..N_SHAPES {
        let d1 = gen_shape(&mut rng, shape, 5000);
        let d2 = gen_shape(&mut rng, (shape + 1) % N_SHAPES, 9000);
        let p1 = prefs_of(4, LZ4F_blockLinked, 1, 0, d1.len() as u64, 0, 0, 1);
        let p2 = prefs_of(5, LZ4F_blockIndependent, 0, 1, 0, 7, 0, 1);
        let f1 = c_compress_frame(&d1, &p1);
        let f2 = c_compress_frame(&d2, &p2);

        // (a) two concatenated frames
        let mut cat = f1.clone();
        cat.extend_from_slice(&f2);
        let mut expect = d1.clone();
        expect.extend_from_slice(&d2);
        for &plan in &[&[1usize][..], &[100][..], &[1 << 20][..], &[7, 3, 5000][..]] {
            let label = format!("multiframe shape={} plan={:?}", shape_name(shape), plan);
            expect_readback(&cat, plan, &expect, "mf", &label);
        }

        // (b) skippable frame first
        let mut buf = skippable_frame(0x184D_2A53, &gen_random(&mut rng, 500));
        buf.extend_from_slice(&f1);
        for &plan in &[&[1usize][..], &[64][..], &[1 << 20][..]] {
            let label = format!("skippable+real shape={} plan={:?}", shape_name(shape), plan);
            expect_readback(&buf, plan, &d1, "sk", &label);
        }

        // (c) trailing skippable frame
        let mut buf = f1.clone();
        buf.extend_from_slice(&skippable_frame(0x184D_2A5F, &gen_random(&mut rng, 77)));
        for &plan in &[&[64usize][..], &[1 << 20][..]] {
            let label = format!("real+skippable shape={} plan={:?}", shape_name(shape), plan);
            expect_readback(&buf, plan, &d1, "sk2", &label);
        }

        // (d) reading far past EOF must simply return short/zero
        let label = format!("read past EOF shape={}", shape_name(shape));
        expect_readback(&f1, &[d1.len() + 4096], &d1, "eof", &label);
    }

    // (e) a file that is *only* a skippable frame: nothing to read
    {
        let buf = skippable_frame(0x184D_2A50, &gen_random(&mut rng, 300));
        for &plan in &[&[1usize][..], &[4096][..]] {
            let label = format!("only-skippable plan={:?}", plan);
            let lc = read_both(&buf, plan, 4096, "onlysk", &label);
            assert_eq!(lc.produced, 0, "{}: nothing should be produced", label);
        }
    }
}

// ===========================================================================
// 7. error paths
// ===========================================================================

#[test]
fn lz4file_error_paths() {
    let a = api();
    let mut rng = Rng::new(0xF11E_0007);
    let data = gen_shape(&mut rng, 4, 30_000);
    let prefs = prefs_of(4, LZ4F_blockLinked, 1, 1, 0, 0, 0, 1);
    let good = c_compress_frame(&data, &prefs);

    unsafe {
        // ---- NULL arguments -------------------------------------------------
        let t = TmpFile::new("err_null");
        t.put(&good);

        // writeOpen(NULL, fp, prefs) / writeOpen(&st, NULL, prefs)
        {
            let fp = t.open("wb");
            let c = (a.c.write_open)(
                std::ptr::null_mut(),
                fp,
                &prefs as *const LZ4F_preferences_t,
            );
            let r = (a.r.write_open)(
                std::ptr::null_mut(),
                fp,
                &prefs as *const LZ4F_preferences_t,
            );
            assert_eq!(ret_str(c), ret_str(r), "writeOpen(NULL statePtr)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
            fclose(fp);
        }
        {
            let mut cst: *mut c_void = std::ptr::null_mut();
            let mut rst: *mut c_void = std::ptr::null_mut();
            let c = (a.c.write_open)(
                &mut cst,
                std::ptr::null_mut(),
                &prefs as *const LZ4F_preferences_t,
            );
            let r = (a.r.write_open)(
                &mut rst,
                std::ptr::null_mut(),
                &prefs as *const LZ4F_preferences_t,
            );
            assert_eq!(ret_str(c), ret_str(r), "writeOpen(NULL fp)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
            assert!(cst.is_null() && rst.is_null());
        }
        // readOpen(NULL, fp) / readOpen(&st, NULL)
        {
            let fp = t.open("rb");
            let c = (a.c.read_open)(std::ptr::null_mut(), fp);
            let r = (a.r.read_open)(std::ptr::null_mut(), fp);
            assert_eq!(ret_str(c), ret_str(r), "readOpen(NULL statePtr)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
            fclose(fp);
        }
        {
            let mut cst: *mut c_void = std::ptr::null_mut();
            let mut rst: *mut c_void = std::ptr::null_mut();
            let c = (a.c.read_open)(&mut cst, std::ptr::null_mut());
            let r = (a.r.read_open)(&mut rst, std::ptr::null_mut());
            assert_eq!(ret_str(c), ret_str(r), "readOpen(NULL fp)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
        }
        // write / read / close with NULL state or NULL buffer
        {
            let mut b = [0u8; 32];
            for &size in &[0usize, 1, 32] {
                let c = (a.c.write)(std::ptr::null_mut(), b.as_ptr() as *const c_void, size);
                let r = (a.r.write)(std::ptr::null_mut(), b.as_ptr() as *const c_void, size);
                assert_eq!(ret_str(c), ret_str(r), "write(NULL state, {})", size);
                assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);

                let c = (a.c.read)(std::ptr::null_mut(), b.as_mut_ptr() as *mut c_void, size);
                let r = (a.r.read)(std::ptr::null_mut(), b.as_mut_ptr() as *mut c_void, size);
                assert_eq!(ret_str(c), ret_str(r), "read(NULL state, {})", size);
                assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
            }
            let c = (a.c.write_close)(std::ptr::null_mut());
            let r = (a.r.write_close)(std::ptr::null_mut());
            assert_eq!(ret_str(c), ret_str(r), "writeClose(NULL)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);

            let c = (a.c.read_close)(std::ptr::null_mut());
            let r = (a.r.read_close)(std::ptr::null_mut());
            assert_eq!(ret_str(c), ret_str(r), "readClose(NULL)");
            assert_eq!(lz4f_error_code(c), err::ERROR_parameter_null);
        }
        // NULL buffer with a *valid* state
        {
            let tw = TmpFile::new("err_nullbuf_w");
            for f in [&a.c, &a.r] {
                let fp = tw.open("wb");
                let mut st: *mut c_void = std::ptr::null_mut();
                let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
                assert!(!lz4f_is_error(o));
                let w = (f.write)(st, std::ptr::null(), 16);
                assert_eq!(
                    lz4f_error_code(w),
                    err::ERROR_parameter_null,
                    "{}: write(state, NULL buf)",
                    f.tag
                );
                let _ = (f.write_close)(st);
                fclose(fp);
            }
            let tr2 = TmpFile::new("err_nullbuf_r");
            tr2.put(&good);
            for f in [&a.c, &a.r] {
                let fp = tr2.open("rb");
                let mut st: *mut c_void = std::ptr::null_mut();
                let o = (f.read_open)(&mut st, fp);
                assert!(!lz4f_is_error(o));
                let n = (f.read)(st, std::ptr::null_mut(), 16);
                assert_eq!(
                    lz4f_error_code(n),
                    err::ERROR_parameter_null,
                    "{}: read(state, NULL buf)",
                    f.tag
                );
                let _ = (f.read_close)(st);
                fclose(fp);
            }
        }

        // ---- writeOpen with an invalid blockSizeID -------------------------
        for &bsid in &[-3i32, -1, 1, 2, 3, 8, 9, 100] {
            let p = prefs_of(bsid, LZ4F_blockLinked, 0, 0, 0, 0, 0, 0);
            let tc = TmpFile::new("err_bsid_c");
            let tr = TmpFile::new("err_bsid_r");
            let lc = run_write(&a.c, &tc, "wb", Some(&p), &[&data[..]]);
            let lr = run_write(&a.r, &tr, "wb", Some(&p), &[&data[..]]);
            assert_eq!(
                ret_str(lc.open_ret),
                ret_str(lr.open_ret),
                "writeOpen(blockSizeID={})",
                bsid
            );
            assert_eq!(
                lz4f_error_code(lc.open_ret),
                err::ERROR_maxBlockSize_invalid,
                "writeOpen(blockSizeID={}) should be maxBlockSize_invalid",
                bsid
            );
            assert!(lc.state_null_after_open && lr.state_null_after_open);
            assert_eq!(
                ret_str(lc.close_ret),
                ret_str(lr.close_ret),
                "writeClose after failed writeOpen(blockSizeID={})",
                bsid
            );
            assert_eq!(lc.file.len(), 0, "nothing should have been written");
            assert_eq!(lr.file.len(), 0, "nothing should have been written");
        }

        // ---- writeOpen / write into a READ-ONLY FILE* => io_write ----------
        {
            let t = TmpFile::new("err_ro");
            t.put(&good); // make sure the file exists so "rb" succeeds
            let lc = run_write(&a.c, &t, "rb", Some(&prefs), &[&data[..]]);
            let t2 = TmpFile::new("err_ro2");
            t2.put(&good);
            let lr = run_write(&a.r, &t2, "rb", Some(&prefs), &[&data[..]]);
            assert_eq!(
                ret_str(lc.open_ret),
                ret_str(lr.open_ret),
                "writeOpen on a read-only FILE*"
            );
            assert_eq!(
                lz4f_error_code(lc.open_ret),
                err::ERROR_io_write,
                "writeOpen on a read-only FILE* must be io_write"
            );
            assert_eq!(
                lc.state_null_after_open, lr.state_null_after_open,
                "state NULLness after the io_write failure"
            );
            assert_eq!(
                ret_str(lc.close_ret),
                ret_str(lr.close_ret),
                "writeClose after the io_write failure"
            );
        }

        // ---- readOpen on files that are too short / not frames -------------
        for n in 0..=19usize {
            let mut f = good.clone();
            f.truncate(n);
            let label = format!("readOpen on a {}-byte file", n);
            let tc = TmpFile::new("err_short_c");
            let tr = TmpFile::new("err_short_r");
            tc.put(&f);
            tr.put(&f);
            let lc = run_read(&a.c, &tc, &[64], 4096);
            let lr = run_read(&a.r, &tr, &[64], 4096);
            assert_eq!(
                ret_str(lc.open_ret),
                ret_str(lr.open_ret),
                "{}: readOpen",
                label
            );
            assert_eq!(
                lc.state_null_after_open, lr.state_null_after_open,
                "{}: state NULLness",
                label
            );
            assert_eq!(
                lc.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                lr.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                "{}: read returns",
                label
            );
            assert_eq!(
                ret_str(lc.close_ret),
                ret_str(lr.close_ret),
                "{}: readClose",
                label
            );
            assert_bytes_eq(&format!("{}: buffer", label), &lc.buf, &lr.buf);
            if n < 19 {
                assert_eq!(
                    lz4f_error_code(lc.open_ret),
                    err::ERROR_io_read,
                    "{}: a file shorter than LZ4F_HEADER_SIZE_MAX must be io_read",
                    label
                );
            }
        }

        // garbage / not-a-frame inputs of >= 19 bytes
        let mut bad_inputs: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..30 {
            let n = 19 + (i * 13) % 200;
            bad_inputs.push((format!("garbage #{} ({}B)", i, n), gen_random(&mut rng, n)));
        }
        for &delta in &[1u32, 0x10, 0x1000] {
            let mut f = good.clone();
            let m = u32::from_le_bytes([f[0], f[1], f[2], f[3]]).wrapping_add(delta);
            f[..4].copy_from_slice(&m.to_le_bytes());
            bad_inputs.push((format!("bad magic +{:#x}", delta), f));
        }
        {
            let mut f = good.clone();
            f[4] |= 0x02;
            bad_inputs.push(("FLG reserved bit".into(), f));
        }
        for &v in &[0u8, 2, 3] {
            let mut f = good.clone();
            f[4] = (f[4] & 0x3F) | (v << 6);
            bad_inputs.push((format!("FLG version={}", v), f));
        }
        for &bd in &[0x00u8, 0x10, 0x20, 0x30, 0x80] {
            let mut f = good.clone();
            f[5] = bd;
            bad_inputs.push((format!("BD={:#04x}", bd), f));
        }
        {
            let mut f = good.clone();
            f[6] ^= 0xFF; // header checksum byte for a 7-byte header
            bad_inputs.push(("bad header checksum".into(), f));
        }
        {
            // corrupt block payload -> LZ4F_read must surface the decode error
            let mut f = good.clone();
            f[30] ^= 0xFF;
            bad_inputs.push(("corrupt payload".into(), f));
        }
        {
            // corrupt the trailing content checksum
            let mut f = good.clone();
            let n = f.len();
            f[n - 2] ^= 0xFF;
            bad_inputs.push(("corrupt content checksum".into(), f));
        }
        for &t in &[20usize, 25, 40, 100, 1000, 5000] {
            if t < good.len() {
                bad_inputs.push((format!("truncated frame to {}", t), good[..t].to_vec()));
            }
        }

        for (name, f) in &bad_inputs {
            for &plan in &[&[16usize][..], &[1][..], &[1 << 20][..]] {
                let label = format!("readOpen/read [{}] plan={:?}", name, plan);
                let tc = TmpFile::new("err_bad_c");
                let tr = TmpFile::new("err_bad_r");
                tc.put(f);
                tr.put(f);
                let lc = run_read(&a.c, &tc, plan, 64 * 1024);
                let lr = run_read(&a.r, &tr, plan, 64 * 1024);
                assert_eq!(
                    ret_str(lc.open_ret),
                    ret_str(lr.open_ret),
                    "{}: readOpen",
                    label
                );
                assert_eq!(
                    lc.state_null_after_open, lr.state_null_after_open,
                    "{}: state NULLness",
                    label
                );
                assert_eq!(
                    lc.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                    lr.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                    "{}: read returns",
                    label
                );
                assert_eq!(
                    ret_str(lc.close_ret),
                    ret_str(lr.close_ret),
                    "{}: readClose",
                    label
                );
                assert_eq!(lc.produced, lr.produced, "{}: produced", label);
                assert_bytes_eq(&format!("{}: buffer", label), &lc.buf, &lr.buf);
            }
        }

        // randomly mutated frames
        for iter in 0..2500u32 {
            let mut f = good.clone();
            let nmut = 1 + (iter as usize % 4);
            for _ in 0..nmut {
                let i = rng.below(f.len());
                f[i] ^= 1u8 << rng.below(8);
            }
            if iter % 4 == 0 {
                let cut = rng.range(0, f.len());
                f.truncate(cut);
            }
            let plan = [rng.range(1, 40_000)];
            let label = format!("read fuzz iter={} nmut={} plan={:?}", iter, nmut, plan);
            let tc = TmpFile::new("err_fuzz_c");
            let tr = TmpFile::new("err_fuzz_r");
            tc.put(&f);
            tr.put(&f);
            let lc = run_read(&a.c, &tc, &plan, 128 * 1024);
            let lr = run_read(&a.r, &tr, &plan, 128 * 1024);
            assert_eq!(
                ret_str(lc.open_ret),
                ret_str(lr.open_ret),
                "{}: readOpen",
                label
            );
            assert_eq!(
                lc.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                lr.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
                "{}: read returns",
                label
            );
            assert_eq!(
                ret_str(lc.close_ret),
                ret_str(lr.close_ret),
                "{}: readClose",
                label
            );
            assert_eq!(lc.produced, lr.produced, "{}: produced", label);
            assert_bytes_eq(&format!("{}: buffer", label), &lc.buf, &lr.buf);
        }
    }
}

// ===========================================================================
// 8. harness self-check: the comparison logic really does detect divergence
// ===========================================================================

#[test]
#[should_panic(expected = "SELFCHECK")]
fn harness_self_check_detects_divergence() {
    let mut rng = Rng::new(0xF11E_5E1F);
    let data = gen_shape(&mut rng, 3, 5000);
    let prefs = prefs_of(4, LZ4F_blockLinked, 1, 1, 0, 0, 0, 1);
    let good = c_compress_frame(&data, &prefs);
    let mut bad = good.clone();
    bad[40] ^= 0xFF;

    let a = api();
    let tc = TmpFile::new("selfcheck_c");
    let tr = TmpFile::new("selfcheck_r");
    tc.put(&good);
    tr.put(&bad); // deliberately different input for the Rust side
    let lc = run_read(&a.c, &tc, &[4096], data.len() + 64);
    let lr = run_read(&a.r, &tr, &[4096], data.len() + 64);
    assert_eq!(
        lc.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
        lr.read_rets.iter().map(|r| ret_str(*r)).collect::<Vec<_>>(),
        "SELFCHECK read returns"
    );
    assert_bytes_eq("SELFCHECK buffer", &lc.buf, &lr.buf);
}

// ===========================================================================
// 9. LZ4F_readOpen sizes its src buffer from the FIRST frame header only, so a
//    later frame with a much larger blockSizeID must still stream correctly.
//    A leading skippable frame makes LZ4F_readOpen fall into its
//    `LZ4F_default` case (frameInfo.blockSizeID == 0 => 64 KB src buffer).
// ===========================================================================

#[test]
fn read_src_buffer_sizing_across_frames() {
    let mut rng = Rng::new(0xF11E_0009);

    // (first frame bsid, second frame bsid)
    for &(b1, b2) in &[
        (4i32, 7i32),
        (4, 6),
        (4, 5),
        (7, 4),
        (6, 5),
        (5, 7),
    ] {
        let d1 = gen_shape(&mut rng, 2, 30_000);
        let d2 = gen_shape(&mut rng, 5, 400_000);
        let p1 = prefs_of(b1, LZ4F_blockLinked, 1, 1, d1.len() as u64, 0, 0, 1);
        let p2 = prefs_of(b2, LZ4F_blockIndependent, 1, 0, 0, 0, 0, 1);
        let mut cat = c_compress_frame(&d1, &p1);
        cat.extend_from_slice(&c_compress_frame(&d2, &p2));
        let mut expect = d1.clone();
        expect.extend_from_slice(&d2);
        for &plan in &[&[7usize][..], &[4096][..], &[1 << 20][..], &[3, 70_000][..]] {
            let label = format!("srcbuf-sizing b1={} b2={} plan={:?}", b1, b2, plan);
            expect_readback(&cat, plan, &expect, "sbs", &label);
        }
    }

    // leading skippable frame => frameInfo.blockSizeID stays 0 (LZ4F_default)
    for &bsid in &[4i32, 5, 6, 7] {
        let data = gen_shape(&mut rng, 4, 300_000);
        let prefs = prefs_of(bsid, LZ4F_blockLinked, 1, 1, data.len() as u64, 0, 0, 1);
        let mut cat = skippable_frame(0x184D_2A50, &gen_random(&mut rng, 1000));
        cat.extend_from_slice(&c_compress_frame(&data, &prefs));
        for &plan in &[&[19usize][..], &[4096][..], &[1 << 20][..]] {
            let label = format!("srcbuf-default bsid={} plan={:?}", bsid, plan);
            expect_readback(&cat, plan, &data, "sbd", &label);
        }
    }

    // three frames with alternating options through one reader
    let mut cat = Vec::new();
    let mut expect = Vec::new();
    for k in 0..3usize {
        let d = gen_shape(&mut rng, k, 50_000 * (k + 1));
        let p = prefs_of(
            [4i32, 6, 5][k],
            if k % 2 == 0 {
                LZ4F_blockLinked
            } else {
                LZ4F_blockIndependent
            },
            (k % 2) as c_int,
            ((k + 1) % 2) as c_int,
            if k == 1 { d.len() as u64 } else { 0 },
            k as c_uint,
            0,
            1,
        );
        cat.extend_from_slice(&c_compress_frame(&d, &p));
        expect.extend_from_slice(&d);
    }
    for &plan in &[&[1usize][..], &[64][..], &[1 << 20][..], &[5, 999, 65_537][..]] {
        let label = format!("three-frames plan={:?}", plan);
        expect_readback(&cat, plan, &expect, "tf", &label);
    }
}
