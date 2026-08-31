//! Differential tests for the stdio `FILE*` API in lz4file.c.
//!
//! Covers CONFIGS.md rows 157..162 ("lz4file") and the `## lz4file.c (stdio
//! file API)` rows of ERRORS.md (160..184).
//!
//! Every call goes through a `.so` export via libloading. The C library and
//! the Rust library always get their OWN scratch file, their OWN `FILE*` and
//! their OWN opaque `LZ4_readFile_t` / `LZ4_writeFile_t` handle (created and
//! destroyed by the same library); only the resulting *file bytes* and the
//! *return values* are compared.
#![allow(unused_imports, non_snake_case)]

mod common;
use common::*;
use std::ffi::CString;
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

// ---------------------------------------------------------------------------
// lz4file FFI signatures
// ---------------------------------------------------------------------------

type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type FnClose = unsafe extern "C" fn(*mut c_void) -> usize;

#[derive(Copy, Clone)]
struct FileApi {
    tag: &'static str,
    wopen: FnWriteOpen,
    write: FnWrite,
    wclose: FnClose,
    ropen: FnReadOpen,
    read: FnRead,
    rclose: FnClose,
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
    (
        FileApi {
            tag: "C",
            wopen: wo_c,
            write: w_c,
            wclose: wc_c,
            ropen: ro_c,
            read: rd_c,
            rclose: rc_c,
        },
        FileApi {
            tag: "Rust",
            wopen: wo_r,
            write: w_r,
            wclose: wc_r,
            ropen: ro_r,
            read: rd_r,
            rclose: rc_r,
        },
    )
}

// ---------------------------------------------------------------------------
// Scratch files
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!("lz4file_test_{}_{}_{}.lz4", std::process::id(), tag, n));
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
    let n = fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), fp);
    assert_eq!(n, bytes.len(), "fwrite({}) short", path.display());
    assert_eq!(fflush(fp), 0, "fflush({}) failed", path.display());
    fclose(fp);
}

/// Read `path` back with raw stdio (`fseek`/`ftell`/`fread`).
unsafe fn read_raw(path: &Path) -> Vec<u8> {
    let fp = open_file(path, "rb");
    assert_eq!(fseek(fp, 0, SEEK_END), 0, "fseek(END) failed");
    let len = ftell(fp);
    assert!(len >= 0, "ftell failed");
    assert_eq!(fseek(fp, 0, SEEK_SET), 0, "fseek(SET) failed");
    let mut v = vec![0u8; len as usize];
    let n = fread(v.as_mut_ptr() as *mut c_void, 1, len as usize, fp);
    assert_eq!(n, len as usize, "fread short");
    fclose(fp);
    v
}

// ---------------------------------------------------------------------------
// Write / read drivers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteOutcome {
    open: usize,
    writes: Vec<usize>,
    close: Option<usize>,
}

/// Open `path` for writing, `LZ4F_writeOpen`, feed each slice of `chunks` to
/// `LZ4F_write`, then `LZ4F_writeClose`. Returns every return value.
unsafe fn drive_write(
    api: &FileApi,
    path: &Path,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
) -> WriteOutcome {
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
        assert!(!h.is_null(), "{}: writeOpen returned OK but handle is NULL", api.tag);
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
    WriteOutcome { open, writes, close }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadOutcome {
    open: usize,
    reads: Vec<usize>,
    data: Vec<u8>,
    close: Option<usize>,
}

/// Open `path`, `LZ4F_readOpen`, then read repeatedly using `sizes` cyclically
/// until EOF (a 0 return), an error, or `cap` bytes, then `LZ4F_readClose`.
unsafe fn drive_read(api: &FileApi, path: &Path, sizes: &[usize], cap: usize) -> ReadOutcome {
    assert!(sizes.iter().any(|&s| s > 0), "drive_read needs a non-zero size");
    let fp = open_file(path, "rb");
    let mut h: *mut c_void = ptr::null_mut();
    let open = (api.ropen)(&mut h, fp);
    let mut reads = Vec::new();
    let mut data = Vec::new();
    let mut close = None;
    if open == 0 {
        assert!(!h.is_null(), "{}: readOpen returned OK but handle is NULL", api.tag);
        let mut i = 0usize;
        loop {
            let want = sizes[i % sizes.len()];
            i += 1;
            if want == 0 {
                let ret = (api.read)(h, data.as_mut_ptr() as *mut c_void, 0);
                reads.push(ret);
                assert_eq!(ret, 0, "{}: LZ4F_read(size=0) should return 0", api.tag);
                continue;
            }
            let mut buf = vec![0u8; want];
            let ret = (api.read)(h, buf.as_mut_ptr() as *mut c_void, want);
            reads.push(ret);
            if is_err_range(ret) {
                break;
            }
            assert!(
                ret <= want,
                "{}: LZ4F_read returned {ret} > requested {want}",
                api.tag
            );
            data.extend_from_slice(&buf[..ret]);
            if ret == 0 || data.len() >= cap {
                break;
            }
        }
        close = Some((api.rclose)(h));
    } else {
        assert!(
            h.is_null(),
            "{}: readOpen failed ({open:#x}) but left a non-NULL handle",
            api.tag
        );
    }
    fclose(fp);
    ReadOutcome {
        open,
        reads,
        data,
        close,
    }
}

/// Full differential round-trip: write `chunks` with both libraries into their
/// own files, compare the produced bytes and every return code, then read both
/// files back with both libraries and compare everything again.
#[track_caller]
unsafe fn roundtrip(
    ctx: &str,
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[&[u8]],
    read_sizes: &[usize],
) -> Vec<u8> {
    let (c, r) = apis();
    let (sc, sr) = scratch_pair("rt");
    let wc = drive_write(&c, sc.path(), prefs, chunks);
    let wr = drive_write(&r, sr.path(), prefs, chunks);
    assert_eq!(
        wc.open as isize, wr.open as isize,
        "{ctx}: LZ4F_writeOpen return mismatch (C={:#x} Rust={:#x})",
        wc.open, wr.open
    );
    assert_eq!(
        wc.writes.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        wr.writes.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        "{ctx}: LZ4F_write return values differ"
    );
    assert_eq!(
        wc.close.map(|x| x as isize),
        wr.close.map(|x| x as isize),
        "{ctx}: LZ4F_writeClose return mismatch"
    );

    let bc = sc.bytes();
    let br = sr.bytes();
    same_full_buffers(&format!("{ctx}: compressed file bytes"), &bc, &br);

    let total: usize = chunks.iter().map(|c| c.len()).sum();
    let mut original = Vec::with_capacity(total);
    for ch in chunks {
        original.extend_from_slice(ch);
    }

    let rc = drive_read(&c, sc.path(), read_sizes, total + 4096);
    let rr = drive_read(&r, sr.path(), read_sizes, total + 4096);
    assert_eq!(
        rc.open as isize, rr.open as isize,
        "{ctx}: LZ4F_readOpen return mismatch (C={:#x} Rust={:#x})",
        rc.open, rr.open
    );
    assert_eq!(
        rc.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        rr.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
        "{ctx}: LZ4F_read return values differ"
    );
    assert_eq!(
        rc.close.map(|x| x as isize),
        rr.close.map(|x| x as isize),
        "{ctx}: LZ4F_readClose return mismatch"
    );
    same_full_buffers(&format!("{ctx}: decompressed payload"), &rc.data, &rr.data);

    // cross-read: the C-written file read by the Rust reader and vice versa
    let x1 = drive_read(&r, sc.path(), read_sizes, total + 4096);
    let x2 = drive_read(&c, sr.path(), read_sizes, total + 4096);
    assert_eq!(
        x1.open as isize, rc.open as isize,
        "{ctx}: the Rust reader disagrees with the C reader on the C-written file"
    );
    assert_eq!(
        x2.open as isize, rc.open as isize,
        "{ctx}: the C reader disagrees on the Rust-written file"
    );

    if rc.open == 0 {
        assert_eq!(
            rc.data.len(),
            original.len(),
            "{ctx}: round-tripped length {} != original {}",
            rc.data.len(),
            original.len()
        );
        assert!(rc.data == original, "{ctx}: round-tripped content differs");
        assert!(x1.data == original, "{ctx}: Rust reader mis-decoded the C-written file");
        assert!(x2.data == original, "{ctx}: C reader mis-decoded the Rust-written file");
    } else {
        // The only legitimate reason for readOpen to fail on a frame we just
        // wrote is a frame shorter than LZ4F_HEADER_SIZE_MAX (19): lz4file's
        // readOpen always demands 19 bytes up front.
        assert_eq!(
            rc.open,
            err(23),
            "{ctx}: unexpected readOpen error {:#x} on a freshly written {}-byte frame",
            rc.open,
            bc.len()
        );
        assert!(
            bc.len() < 19,
            "{ctx}: readOpen reported io_read on a {}-byte frame (>= 19)",
            bc.len()
        );
    }

    bc
}

/// `LZ4F_writeClose` returns the byte count produced by `LZ4F_compressEnd` on
/// success (only `err(22)` / a forwarded compressEnd error means failure), so
/// "success" is "not in the lz4frame error range".
#[track_caller]
fn assert_ok(ctx: &str, v: Option<usize>) -> usize {
    let v = v.unwrap_or_else(|| panic!("{ctx}: call was never made"));
    assert!(
        !is_err_range(v),
        "{ctx}: expected success, got error {:#x} (LZ4F code {})",
        v,
        (0usize).wrapping_sub(v)
    );
    v
}

fn prefs_with(bsid: c_uint) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p
}

// ===========================================================================
// Row 157 — LZ4F_writeOpen
// ===========================================================================

#[test]
fn row_157_writeOpen_prefs_null_and_every_blocksizeid() {
    unsafe {
        let mut rng = Rng::new(157);
        let payload = gen(&mut rng, Shape::TextLike, 300_000);

        // prefsPtr == NULL -> maxWriteSize 64 KB
        roundtrip("row157 prefs=NULL", None, &[&payload], &[7000]);

        // every valid blockSizeID
        for bsid in [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            let p = prefs_with(bsid);
            roundtrip(
                &format!("row157 blockSizeID={bsid}"),
                Some(&p),
                &[&payload],
                &[7000],
            );
        }
    }
}

#[test]
fn row_157_writeOpen_invalid_blocksizeid_and_null_params() {
    unsafe {
        let (c, r) = apis();

        // invalid blockSizeID -> LZ4F_ERROR_maxBlockSize_invalid == (size_t)-2
        for bsid in [1u32, 2, 3, 8, 9, 100, u32::MAX] {
            let p = prefs_with(bsid);
            let (sc, sr) = scratch_pair("w_badbsid");
            let wc = drive_write(&c, sc.path(), Some(&p), &[]);
            let wr = drive_write(&r, sr.path(), Some(&p), &[]);
            assert_eq!(
                wc.open as isize, wr.open as isize,
                "row157: writeOpen(blockSizeID={bsid}) C={:#x} Rust={:#x}",
                wc.open, wr.open
            );
            assert_eq!(
                wc.open,
                err(2),
                "row157: writeOpen(blockSizeID={bsid}) must be LZ4F_ERROR_maxBlockSize_invalid err(2), got {:#x}",
                wc.open
            );
            assert!(wc.writes.is_empty() && wc.close.is_none());
            // nothing was written to the file
            assert!(sc.bytes().is_empty(), "row157: bytes written despite the error");
            assert!(sr.bytes().is_empty(), "row157: bytes written despite the error");
        }

        // fp == NULL -> parameter_null; the handle is left untouched
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        let a = (c.wopen)(&mut hc, ptr::null_mut(), ptr::null());
        let b = (r.wopen)(&mut hr, ptr::null_mut(), ptr::null());
        assert_eq!(a as isize, b as isize, "row157: writeOpen(fp=NULL) C={a:#x} Rust={b:#x}");
        assert_eq!(a, err(21), "row157: writeOpen(fp=NULL) must be err(21), got {a:#x}");
        assert!(hc.is_null() && hr.is_null());

        // handle == NULL -> parameter_null
        let (sc, sr) = scratch_pair("w_nullhandle");
        let fpc = open_file(sc.path(), "wb");
        let fpr = open_file(sr.path(), "wb");
        let a = (c.wopen)(ptr::null_mut(), fpc, ptr::null());
        let b = (r.wopen)(ptr::null_mut(), fpr, ptr::null());
        assert_eq!(
            a as isize, b as isize,
            "row157: writeOpen(handle=NULL) C={a:#x} Rust={b:#x}"
        );
        assert_eq!(a, err(21), "row157: writeOpen(handle=NULL) must be err(21), got {a:#x}");
        fclose(fpc);
        fclose(fpr);

        // both NULL
        let a = (c.wopen)(ptr::null_mut(), ptr::null_mut(), ptr::null());
        let b = (r.wopen)(ptr::null_mut(), ptr::null_mut(), ptr::null());
        assert_eq!(a as isize, b as isize);
        assert_eq!(a, err(21));
    }
}

// ===========================================================================
// Row 158 — LZ4F_write sizes and NULL parameters
// ===========================================================================

#[test]
fn row_158_write_sizes_around_maxwritesize() {
    unsafe {
        let mut rng = Rng::new(158);
        // default prefs -> maxWriteSize == 64 KB
        const MWS: usize = 64 * 1024;
        let big = gen(&mut rng, Shape::TextLike, 5 * MWS);

        for &n in &[
            0usize,
            1,
            17,
            MWS - 1,
            MWS,
            MWS + 1,
            2 * MWS,
            2 * MWS + 3,
            5 * MWS,
        ] {
            let chunk = &big[..n];
            let out = roundtrip(
                &format!("row158 single write of {n}"),
                None,
                &[chunk],
                &[MWS],
            );
            assert!(!out.is_empty(), "row158: empty frame file for n={n}");
        }

        // several writes, mixing sizes below / at / above maxWriteSize
        let a = &big[..10];
        let b = &big[10..10 + MWS];
        let cc = &big[..MWS + 12345];
        let d: &[u8] = &[];
        roundtrip("row158 multi-write", None, &[a, d, b, d, cc, a], &[1, 3, 5000]);

        // and with a 256 KB block size (maxWriteSize 256 KB)
        let p = prefs_with(LZ4F_MAX256KB);
        roundtrip(
            "row158 multi-write 256KB",
            Some(&p),
            &[a, b, cc],
            &[100_000],
        );
    }
}

#[test]
fn row_158_write_null_buffer_and_null_handle() {
    unsafe {
        let (c, r) = apis();
        let (sc, sr) = scratch_pair("w_nullbuf");
        let fpc = open_file(sc.path(), "wb");
        let fpr = open_file(sr.path(), "wb");
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        assert_eq!((c.wopen)(&mut hc, fpc, ptr::null()), 0);
        assert_eq!((r.wopen)(&mut hr, fpr, ptr::null()), 0);

        // buf == NULL, with size 0 and non-zero: parameter_null in both cases
        for &n in &[0usize, 1, 100] {
            let a = (c.write)(hc, ptr::null(), n);
            let b = (r.write)(hr, ptr::null(), n);
            assert_eq!(
                a as isize, b as isize,
                "row158: LZ4F_write(buf=NULL,size={n}) C={a:#x} Rust={b:#x}"
            );
            assert_eq!(
                a,
                err(21),
                "row158: LZ4F_write(buf=NULL,size={n}) must be err(21), got {a:#x}"
            );
        }

        // handle == NULL
        let data = [1u8, 2, 3];
        let a = (c.write)(ptr::null_mut(), data.as_ptr() as *const c_void, 3);
        let b = (r.write)(ptr::null_mut(), data.as_ptr() as *const c_void, 3);
        assert_eq!(a as isize, b as isize, "row158: LZ4F_write(handle=NULL)");
        assert_eq!(a, err(21));
        // both NULL
        let a = (c.write)(ptr::null_mut(), ptr::null(), 0);
        let b = (r.write)(ptr::null_mut(), ptr::null(), 0);
        assert_eq!(a as isize, b as isize);
        assert_eq!(a, err(21));

        // size == 0 with a valid buffer is a no-op returning 0
        let a = (c.write)(hc, data.as_ptr() as *const c_void, 0);
        let b = (r.write)(hr, data.as_ptr() as *const c_void, 0);
        assert_eq!(a as isize, b as isize, "row158: LZ4F_write(size=0)");
        assert_eq!(a, 0, "row158: LZ4F_write(size=0) must return 0");

        // the rejected calls must not have poisoned errCode: close still works
        let a = (c.wclose)(hc);
        let b = (r.wclose)(hr);
        assert_eq!(a as isize, b as isize, "row158: writeClose after rejected writes");
        assert_ok("row158: writeClose after rejected writes", Some(a));
        fclose(fpc);
        fclose(fpr);
        same_full_buffers("row158 empty frame bytes", &sc.bytes(), &sr.bytes());
    }
}

// ===========================================================================
// Row 159 — non-default preferences forwarded to compressBegin/compressUpdate
// ===========================================================================

#[test]
fn row_159_writeOpen_forwards_non_default_preferences() {
    unsafe {
        let mut rng = Rng::new(159);
        let payload = gen(&mut rng, Shape::TextLike, 200_000);
        let small = gen(&mut rng, Shape::Compressible, 1000);

        // one axis at a time
        let mut cases: Vec<(String, LZ4F_preferences_t)> = Vec::new();
        for cc in [LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
            for bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentChecksumFlag = cc;
                p.frameInfo.blockChecksumFlag = bc;
                cases.push((format!("cc={cc} bc={bc}"), p));
            }
        }
        for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockMode = bm;
            cases.push((format!("blockMode={bm}"), p));
        }
        for lvl in [0i32, 1, 9, 10, 12] {
            let mut p = LZ4F_preferences_t::default();
            p.compressionLevel = lvl;
            cases.push((format!("level={lvl}"), p));
        }
        for af in [0u32, 1] {
            let mut p = LZ4F_preferences_t::default();
            p.autoFlush = af;
            cases.push((format!("autoFlush={af}"), p));
        }
        for did in [0u32, 1, 0xDEAD_BEEF] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.dictID = did;
            cases.push((format!("dictID={did:#x}"), p));
        }
        {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentSize = payload.len() as u64;
            cases.push(("contentSize=exact".to_string(), p));
        }
        {
            // everything at once
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = LZ4F_MAX256KB;
            p.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
            p.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
            p.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
            p.frameInfo.contentSize = payload.len() as u64;
            p.frameInfo.dictID = 0x1234_5678;
            p.compressionLevel = 12;
            p.autoFlush = 1;
            p.favorDecSpeed = 1;
            cases.push(("all-non-default".to_string(), p));
        }

        for (name, p) in &cases {
            roundtrip(
                &format!("row159 {name}"),
                Some(p),
                &[&payload],
                &[13_000],
            );
        }

        // contentSize declared but wrong -> LZ4F_compressEnd reports
        // LZ4F_ERROR_frameSize_wrong (err(14)) from LZ4F_writeClose
        let (c, r) = apis();
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.contentSize = 999_999;
        let (sc, sr) = scratch_pair("w_wrongsize");
        let wc = drive_write(&c, sc.path(), Some(&p), &[&small]);
        let wr = drive_write(&r, sr.path(), Some(&p), &[&small]);
        assert_eq!(wc, wr, "row159: wrong contentSize outcome mismatch");
        assert_eq!(wc.open, 0);
        assert_eq!(wc.writes, vec![small.len()]);
        assert_eq!(
            wc.close,
            Some(err(14)),
            "row159: writeClose with a wrong declared contentSize must be err(14), got {:?}",
            wc.close
        );
        same_full_buffers("row159 wrong-contentSize bytes", &sc.bytes(), &sr.bytes());
    }
}

// ===========================================================================
// Row 160 — LZ4F_writeClose
// ===========================================================================

#[test]
fn row_160_writeClose_normal_and_null_handle() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(160);
        let payload = gen(&mut rng, Shape::Compressible, 5000);

        // normal close: compressEnd + fwrite, returns 0
        let (sc, sr) = scratch_pair("wclose");
        let wc = drive_write(&c, sc.path(), None, &[&payload]);
        let wr = drive_write(&r, sr.path(), None, &[&payload]);
        assert_eq!(wc, wr, "row160: normal write outcome mismatch");
        // writeClose returns the compressEnd byte count on success
        let n = assert_ok("row160: normal writeClose", wc.close);
        assert!(n > 0, "row160: writeClose should report the trailer byte count");
        same_full_buffers("row160 file bytes", &sc.bytes(), &sr.bytes());

        // handle == NULL -> parameter_null
        let a = (c.wclose)(ptr::null_mut());
        let b = (r.wclose)(ptr::null_mut());
        assert_eq!(a as isize, b as isize, "row160: writeClose(NULL) C={a:#x} Rust={b:#x}");
        assert_eq!(a, err(21), "row160: writeClose(NULL) must be err(21), got {a:#x}");

        // close with no payload at all: header + endMark only
        let (sc, sr) = scratch_pair("wclose_empty");
        let wc = drive_write(&c, sc.path(), None, &[]);
        let wr = drive_write(&r, sr.path(), None, &[]);
        assert_eq!(wc, wr, "row160: empty-frame outcome mismatch");
        assert_eq!(
            wc.close,
            Some(4),
            "row160: an empty frame's writeClose writes only the 4-byte endMark"
        );
        let bytes = sc.bytes();
        same_full_buffers("row160 empty frame", &bytes, &sr.bytes());
        assert_eq!(bytes.len(), 11, "row160: expected 7-byte header + 4-byte endMark");
    }
}

/// ERRORS rows 180 + 184 — a short write inside `LZ4F_write` latches
/// `errCode = (size_t)-22` and returns `err(22)`; the following
/// `LZ4F_writeClose` then SKIPS compressEnd and returns `LZ4F_OK_NoError`.
/// `/dev/full` provides the short write (buffered stdio lets the small frame
/// header through, then the bulk data fails).
#[test]
fn row_160_writeClose_after_previous_write_error() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(1600);
        let payload = gen(&mut rng, Shape::Incompressible, 4 * 1024 * 1024);

        let mut results = Vec::new();
        for api in [&c, &r] {
            let fp = open_file(Path::new("/dev/full"), "wb");
            let mut h: *mut c_void = ptr::null_mut();
            let open = (api.wopen)(&mut h, fp, ptr::null());
            let mut w = err(0);
            let mut close = err(0);
            if open == 0 {
                w = (api.write)(h, payload.as_ptr() as *const c_void, payload.len());
                close = (api.wclose)(h);
            }
            fclose(fp);
            results.push((open, w, close));
        }
        assert_eq!(
            (results[0].0 as isize, results[0].1 as isize, results[0].2 as isize),
            (results[1].0 as isize, results[1].1 as isize, results[1].2 as isize),
            "row160: /dev/full write-error sequence differs: C={:#x?} Rust={:#x?}",
            results[0],
            results[1]
        );
        assert_eq!(results[0].0, 0, "row160: writeOpen on /dev/full should succeed (buffered header)");
        assert_eq!(
            results[0].1,
            err(22),
            "row160: LZ4F_write to /dev/full must be LZ4F_ERROR_io_write err(22), got {:#x}",
            results[0].1
        );
        assert_eq!(
            results[0].2, 0,
            "row160/ERRORS184: writeClose after a latched write error must return 0, got {:#x}",
            results[0].2
        );
    }
}

/// ERRORS row 177 — a short write of the frame header inside `LZ4F_writeOpen`
/// returns `err(22)` and frees + nulls the handle.
#[test]
fn errors_177_writeOpen_short_header_write_is_io_write() {
    unsafe {
        let (c, r) = apis();
        let mut rets = Vec::new();
        for api in [&c, &r] {
            let fp = open_file(Path::new("/dev/full"), "wb");
            // unbuffered, so even the 7-byte header write reaches the device
            assert_eq!(setvbuf(fp, ptr::null_mut(), IONBF, 0), 0);
            let mut h: *mut c_void = ptr::null_mut();
            let ret = (api.wopen)(&mut h, fp, ptr::null());
            assert!(h.is_null(), "{}: handle not nulled after io_write", api.tag);
            fclose(fp);
            rets.push(ret);
        }
        assert_eq!(
            rets[0] as isize, rets[1] as isize,
            "errors177: C={:#x} Rust={:#x}",
            rets[0], rets[1]
        );
        assert_eq!(
            rets[0],
            err(22),
            "errors177: expected LZ4F_ERROR_io_write err(22), got {:#x}",
            rets[0]
        );
    }
}

/// ERRORS row 183 — a short write of the frame trailer inside
/// `LZ4F_writeClose` returns `err(22)`. An `fmemopen` buffer sized to hold the
/// header and the data block but not the 4-byte endMark provides it.
#[test]
fn errors_183_writeClose_short_trailer_write_is_io_write() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(183);
        let payload = gen(&mut rng, Shape::Compressible, 4096);

        // Measure the exact frame size first (via a real file).
        let (sc, _sr) = scratch_pair("trailer_probe");
        let w = drive_write(&c, sc.path(), None, &[&payload]);
        assert_ok("errors183: probe writeClose", w.close);
        let full_len = sc.bytes().len();
        assert!(full_len > 8);
        let cap = full_len - 1; // one byte short of the 4-byte endMark

        let mode = CString::new("wb").unwrap();
        let mut rets = Vec::new();
        for api in [&c, &r] {
            let mut mem = vec![0u8; cap];
            let fp = fmemopen(mem.as_mut_ptr() as *mut c_void, cap, mode.as_ptr());
            assert!(!fp.is_null(), "fmemopen failed");
            assert_eq!(setvbuf(fp, ptr::null_mut(), IONBF, 0), 0);
            let mut h: *mut c_void = ptr::null_mut();
            let open = (api.wopen)(&mut h, fp, ptr::null());
            let mut wret = err(0);
            let mut close = err(0);
            if open == 0 {
                wret = (api.write)(h, payload.as_ptr() as *const c_void, payload.len());
                close = (api.wclose)(h);
            }
            fclose(fp);
            rets.push((open, wret, close));
        }
        assert_eq!(
            (rets[0].0 as isize, rets[0].1 as isize, rets[0].2 as isize),
            (rets[1].0 as isize, rets[1].1 as isize, rets[1].2 as isize),
            "errors183: sequence differs C={:#x?} Rust={:#x?}",
            rets[0],
            rets[1]
        );
        assert_eq!(rets[0].0, 0, "errors183: writeOpen should succeed");
        assert_eq!(rets[0].1, payload.len(), "errors183: LZ4F_write should succeed");
        assert_eq!(
            rets[0].2,
            err(22),
            "errors183: writeClose must report LZ4F_ERROR_io_write err(22), got {:#x}",
            rets[0].2
        );
    }
}

// ===========================================================================
// Row 161 — LZ4F_readOpen
// ===========================================================================

#[test]
fn row_161_readOpen_short_file_is_io_read() {
    unsafe {
        let (c, r) = apis();
        // any file shorter than LZ4F_HEADER_SIZE_MAX (19) fails the initial
        // fread with LZ4F_ERROR_io_read
        for n in [0usize, 1, 4, 6, 7, 11, 18] {
            let (sc, sr) = scratch_pair("short");
            let data: Vec<u8> = (0..n).map(|i| (i as u8).wrapping_mul(7)).collect();
            write_raw(sc.path(), &data);
            write_raw(sr.path(), &data);
            let a = drive_read_open_only(&c, sc.path());
            let b = drive_read_open_only(&r, sr.path());
            assert_eq!(
                a as isize, b as isize,
                "row161: readOpen on a {n}-byte file C={a:#x} Rust={b:#x}"
            );
            assert_eq!(
                a,
                err(23),
                "row161: readOpen on a {n}-byte file must be LZ4F_ERROR_io_read err(23), got {a:#x}"
            );
        }
    }
}

unsafe fn drive_read_open_only(api: &FileApi, path: &Path) -> usize {
    let fp = open_file(path, "rb");
    let mut h: *mut c_void = ptr::null_mut();
    let ret = (api.ropen)(&mut h, fp);
    if ret == 0 {
        assert!(!h.is_null());
        let cl = (api.rclose)(h);
        assert_eq!(cl, 0, "{}: readClose failed", api.tag);
    } else {
        assert!(h.is_null(), "{}: handle not nulled after readOpen error", api.tag);
    }
    fclose(fp);
    ret
}

#[test]
fn row_161_readOpen_each_blocksizeid_and_null_params() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(1610);
        let payload = gen(&mut rng, Shape::TextLike, 700_000);

        // srcBufMaxSize is derived from the frame's blockSizeID: exercise all
        for bsid in [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
            let p = prefs_with(bsid);
            roundtrip(
                &format!("row161 readOpen blockSizeID={bsid}"),
                Some(&p),
                &[&payload],
                &[65536, 1, 300_000],
            );
        }

        // fp == NULL / handle == NULL -> parameter_null
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        let a = (c.ropen)(&mut hc, ptr::null_mut());
        let b = (r.ropen)(&mut hr, ptr::null_mut());
        assert_eq!(a as isize, b as isize, "row161: readOpen(fp=NULL)");
        assert_eq!(a, err(21), "row161: readOpen(fp=NULL) must be err(21), got {a:#x}");
        assert!(hc.is_null() && hr.is_null());

        let (sc, sr) = scratch_pair("ropen_null");
        // give both files a valid frame so only the NULL handle is at fault
        let _ = drive_write(&c, sc.path(), None, &[&payload[..100]]);
        let _ = drive_write(&r, sr.path(), None, &[&payload[..100]]);
        let fpc = open_file(sc.path(), "rb");
        let fpr = open_file(sr.path(), "rb");
        let a = (c.ropen)(ptr::null_mut(), fpc);
        let b = (r.ropen)(ptr::null_mut(), fpr);
        assert_eq!(a as isize, b as isize, "row161: readOpen(handle=NULL)");
        assert_eq!(a, err(21), "row161: readOpen(handle=NULL) must be err(21), got {a:#x}");
        fclose(fpc);
        fclose(fpr);

        let a = (c.ropen)(ptr::null_mut(), ptr::null_mut());
        let b = (r.ropen)(ptr::null_mut(), ptr::null_mut());
        assert_eq!(a as isize, b as isize);
        assert_eq!(a, err(21));
    }
}

/// ERRORS rows 164 + 165 — `LZ4F_getFrameInfo` failures inside
/// `LZ4F_readOpen` are forwarded verbatim, including the invalid
/// `blockSizeID` byte (`err(2)`), and the handle is freed + nulled.
#[test]
fn row_161_errors_164_165_readOpen_bad_frame_headers() {
    unsafe {
        let (c, r) = apis();

        fn frame_header(flg: u8, bd: u8, hc: u8) -> Vec<u8> {
            let mut v = vec![0x04, 0x22, 0x4D, 0x18, flg, bd, hc];
            v.resize(32, 0); // >= 19 bytes so the initial fread succeeds
            v
        }

        // (name, bytes, expected error)
        let mut cases: Vec<(String, Vec<u8>, usize)> = Vec::new();
        // bad magic number -> frameType_unknown err(13)
        let mut bad_magic = frame_header(0x40, 0x70, 0x00);
        bad_magic[0] = 0x05;
        cases.push(("bad magic".into(), bad_magic, err(13)));
        // blockSizeID (BD>>4)&7 < 4 -> maxBlockSize_invalid err(2)
        for bsid in [0u8, 1, 2, 3] {
            cases.push((
                format!("blockSizeID byte {bsid}"),
                frame_header(0x40, bsid << 4, 0x00),
                err(2),
            ));
        }
        // FLG reserved bit 1 set -> reservedFlag_set err(8)
        cases.push(("FLG reserved bit".into(), frame_header(0x42, 0x70, 0x00), err(8)));
        // FLG version field != 1 -> headerVersion_wrong err(6)
        cases.push(("bad version".into(), frame_header(0x80, 0x70, 0x00), err(6)));
        // BD reserved bit 7 -> reservedFlag_set err(8)
        cases.push(("BD reserved bit".into(), frame_header(0x40, 0xF0, 0x00), err(8)));
        // BD low nibble non-zero -> reservedFlag_set err(8)
        cases.push(("BD low nibble".into(), frame_header(0x40, 0x71, 0x00), err(8)));
        // valid FLG/BD but a wrong header checksum -> headerChecksum_invalid err(17)
        cases.push(("bad header checksum".into(), frame_header(0x40, 0x70, 0x00), err(17)));

        for (name, bytes, expect) in cases {
            let (sc, sr) = scratch_pair("badhdr");
            write_raw(sc.path(), &bytes);
            write_raw(sr.path(), &bytes);
            let a = drive_read_open_only(&c, sc.path());
            let b = drive_read_open_only(&r, sr.path());
            assert_eq!(
                a as isize, b as isize,
                "row161 [{name}]: readOpen C={a:#x} Rust={b:#x}"
            );
            assert_eq!(
                a, expect,
                "row161 [{name}]: expected {expect:#x}, got {a:#x}"
            );
        }

        // A legacy-magic / skippable frame is accepted by getFrameInfo? -> the
        // skippable magic is recognised by LZ4F_headerSize but the frame type
        // makes blockSizeID meaningless; whatever the C library decides, the
        // Rust one must agree.
        let mut skip = vec![0x50, 0x2A, 0x4D, 0x18, 0x10, 0x00, 0x00, 0x00];
        skip.resize(64, 0xAB);
        let (sc, sr) = scratch_pair("skippable");
        write_raw(sc.path(), &skip);
        write_raw(sr.path(), &skip);
        let a = drive_read_open_only(&c, sc.path());
        let b = drive_read_open_only(&r, sr.path());
        assert_eq!(
            a as isize, b as isize,
            "row161 [skippable frame]: readOpen C={a:#x} Rust={b:#x}"
        );
    }
}

// ===========================================================================
// Row 162 — LZ4F_read / LZ4F_readClose
// ===========================================================================

#[test]
fn row_162_read_sizes_sequential_reads_and_eof_short_count() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(162);
        let payload = gen(&mut rng, Shape::TextLike, 100_000);

        let (sc, sr) = scratch_pair("read");
        let wc = drive_write(&c, sc.path(), None, &[&payload]);
        let wr = drive_write(&r, sr.path(), None, &[&payload]);
        assert_eq!(wc, wr);
        same_full_buffers("row162 frame bytes", &sc.bytes(), &sr.bytes());

        // size == 0 must return 0 without consuming anything, then a normal
        // read must still deliver the first bytes
        for api in [&c, &r] {
            let path = if api.tag == "C" { sc.path() } else { sr.path() };
            let fp = open_file(path, "rb");
            let mut h: *mut c_void = ptr::null_mut();
            assert_eq!((api.ropen)(&mut h, fp), 0);
            let mut buf = vec![0u8; 1024];
            for _ in 0..3 {
                let z = (api.read)(h, buf.as_mut_ptr() as *mut c_void, 0);
                assert_eq!(z, 0, "{}: LZ4F_read(size=0) must return 0", api.tag);
            }
            let n = (api.read)(h, buf.as_mut_ptr() as *mut c_void, 1024);
            assert_eq!(n, 1024, "{}: expected a full 1024-byte read", api.tag);
            assert_eq!(&buf[..], &payload[..1024], "{}: wrong content", api.tag);
            // read far more than what remains -> short count at EOF
            let mut big = vec![0u8; payload.len()];
            let m = (api.read)(h, big.as_mut_ptr() as *mut c_void, payload.len());
            assert_eq!(
                m,
                payload.len() - 1024,
                "{}: expected a short count of {} at EOF, got {m}",
                api.tag,
                payload.len() - 1024
            );
            assert_eq!(&big[..m], &payload[1024..], "{}: wrong tail content", api.tag);
            // a further read returns 0
            let z = (api.read)(h, big.as_mut_ptr() as *mut c_void, 100);
            assert_eq!(z, 0, "{}: read past EOF must return 0", api.tag);
            assert_eq!((api.rclose)(h), 0, "{}: readClose", api.tag);
            fclose(fp);
        }

        // many sequential reads with assorted sizes, including 0
        for sizes in [
            vec![1usize],
            vec![0, 1, 0, 2, 0, 3],
            vec![7, 1, 4096, 3],
            vec![65536],
            vec![payload.len() + 12345],
            vec![13, 13, 13, 1024 * 1024],
        ] {
            let a = drive_read(&c, sc.path(), &sizes, payload.len() + 1);
            let b = drive_read(&r, sr.path(), &sizes, payload.len() + 1);
            assert_eq!(a.open as isize, b.open as isize, "row162 sizes={sizes:?}: readOpen");
            assert_eq!(
                a.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
                b.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
                "row162 sizes={sizes:?}: LZ4F_read returns differ"
            );
            assert_eq!(a.close, b.close, "row162 sizes={sizes:?}: readClose");
            same_full_buffers(&format!("row162 sizes={sizes:?} payload"), &a.data, &b.data);
            assert!(
                a.data == payload,
                "row162 sizes={sizes:?}: decoded {} bytes, expected {}",
                a.data.len(),
                payload.len()
            );
        }
    }
}

/// `LZ4F_read` consumes the underlying `FILE*` in `srcBufMaxSize` gulps; after
/// a complete read both libraries must have left the stream at EOF. Uses raw
/// stdio (`fseek`/`ftell`/`fread`/`fwrite`/`fflush`) directly.
#[test]
fn row_162_stdio_stream_position_after_full_read() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(162_000);
        let payload = gen(&mut rng, Shape::TextLike, 250_000);
        let (sc, sr) = scratch_pair("ftell");
        assert_ok("writeClose (C)", drive_write(&c, sc.path(), None, &[&payload]).close);
        assert_ok("writeClose (Rust)", drive_write(&r, sr.path(), None, &[&payload]).close);

        // raw stdio read-back must match what std::fs sees
        assert_eq!(read_raw(sc.path()), sc.bytes(), "row162: raw read-back mismatch (C)");
        same_full_buffers("row162 frame bytes", &read_raw(sc.path()), &read_raw(sr.path()));
        let file_len = sc.bytes().len();

        let mut tells = Vec::new();
        for api in [&c, &r] {
            let path = if api.tag == "C" { sc.path() } else { sr.path() };
            let fp = open_file(path, "rb");
            let mut h: *mut c_void = ptr::null_mut();
            assert_eq!((api.ropen)(&mut h, fp), 0);
            let mut out = vec![0u8; payload.len() + 100];
            let mut got = 0usize;
            loop {
                let n = (api.read)(h, out[got..].as_mut_ptr() as *mut c_void, 65536);
                assert!(!is_err_range(n), "{}: LZ4F_read error {n:#x}", api.tag);
                if n == 0 {
                    break;
                }
                got += n;
            }
            assert_eq!(got, payload.len(), "{}: short round-trip", api.tag);
            assert_eq!(&out[..got], &payload[..], "{}: content mismatch", api.tag);
            assert_eq!((api.rclose)(h), 0, "{}: readClose", api.tag);
            tells.push(ftell(fp));
            fclose(fp);
        }
        assert_eq!(
            tells[0], tells[1],
            "row162: stream position after a full read differs (C={} Rust={})",
            tells[0], tells[1]
        );
        assert_eq!(
            tells[0] as usize, file_len,
            "row162: expected the whole {file_len}-byte file to be consumed, ftell={}",
            tells[0]
        );
    }
}

#[test]
fn row_162_read_linked_independent_and_checksums() {
    unsafe {
        let mut rng = Rng::new(1620);
        let payload = gen(&mut rng, Shape::TextLike, 400_000);
        for bm in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for cc in [LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
                for bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                    for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB] {
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = bm;
                        p.frameInfo.contentChecksumFlag = cc;
                        p.frameInfo.blockChecksumFlag = bc;
                        roundtrip(
                            &format!("row162 bm={bm} cc={cc} bc={bc} bsid={bsid}"),
                            Some(&p),
                            &[&payload],
                            &[1, 9999, 70_000],
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn row_162_read_and_readClose_null_parameters() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(16200);
        let payload = gen(&mut rng, Shape::Compressible, 3000);
        let (sc, sr) = scratch_pair("read_null");
        assert_ok("writeClose (C)", drive_write(&c, sc.path(), None, &[&payload]).close);
        assert_ok("writeClose (Rust)", drive_write(&r, sr.path(), None, &[&payload]).close);

        let fpc = open_file(sc.path(), "rb");
        let fpr = open_file(sr.path(), "rb");
        let mut hc: *mut c_void = ptr::null_mut();
        let mut hr: *mut c_void = ptr::null_mut();
        assert_eq!((c.ropen)(&mut hc, fpc), 0);
        assert_eq!((r.ropen)(&mut hr, fpr), 0);

        // buf == NULL, both with size 0 and non-zero
        for &n in &[0usize, 1, 100] {
            let a = (c.read)(hc, ptr::null_mut(), n);
            let b = (r.read)(hr, ptr::null_mut(), n);
            assert_eq!(
                a as isize, b as isize,
                "row162: LZ4F_read(buf=NULL,size={n}) C={a:#x} Rust={b:#x}"
            );
            assert_eq!(
                a,
                err(21),
                "row162: LZ4F_read(buf=NULL,size={n}) must be err(21), got {a:#x}"
            );
        }
        // handle == NULL
        let mut buf = [0u8; 16];
        let a = (c.read)(ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, 16);
        let b = (r.read)(ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, 16);
        assert_eq!(a as isize, b as isize, "row162: LZ4F_read(handle=NULL)");
        assert_eq!(a, err(21));
        let a = (c.read)(ptr::null_mut(), ptr::null_mut(), 0);
        let b = (r.read)(ptr::null_mut(), ptr::null_mut(), 0);
        assert_eq!(a as isize, b as isize);
        assert_eq!(a, err(21));

        // the rejected reads must not have disturbed the stream
        let mut out = vec![0u8; payload.len()];
        let a = (c.read)(hc, out.as_mut_ptr() as *mut c_void, payload.len());
        assert_eq!(a, payload.len(), "row162: read after rejected calls (C)");
        assert_eq!(out, payload);
        let mut out2 = vec![0u8; payload.len()];
        let b = (r.read)(hr, out2.as_mut_ptr() as *mut c_void, payload.len());
        assert_eq!(b, payload.len(), "row162: read after rejected calls (Rust)");
        assert_eq!(out2, payload);

        assert_eq!((c.rclose)(hc) as isize, (r.rclose)(hr) as isize, "row162: readClose");
        fclose(fpc);
        fclose(fpr);

        // readClose(NULL) -> parameter_null
        let a = (c.rclose)(ptr::null_mut());
        let b = (r.rclose)(ptr::null_mut());
        assert_eq!(a as isize, b as isize, "row162: readClose(NULL) C={a:#x} Rust={b:#x}");
        assert_eq!(a, err(21), "row162: readClose(NULL) must be err(21), got {a:#x}");
    }
}

/// ERRORS row 169 — an error from `LZ4F_decompress` is returned verbatim by
/// `LZ4F_read`.
#[test]
fn row_162_errors_169_read_forwards_decompress_errors() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(169);
        let payload = gen(&mut rng, Shape::TextLike, 50_000);

        // block checksums on: corrupting a payload byte must be detected
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
        p.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;

        let (sc, sr) = scratch_pair("corrupt");
        assert_ok("writeClose (C)", drive_write(&c, sc.path(), Some(&p), &[&payload]).close);
        assert_ok("writeClose (Rust)", drive_write(&r, sr.path(), Some(&p), &[&payload]).close);
        let mut bc = sc.bytes();
        let mut br = sr.bytes();
        same_full_buffers("row162 corrupt base bytes", &bc, &br);
        // flip a bit well inside the first block's compressed payload
        let idx = 40;
        bc[idx] ^= 0x55;
        br[idx] ^= 0x55;
        write_raw(sc.path(), &bc);
        write_raw(sr.path(), &br);

        let a = drive_read(&c, sc.path(), &[4096], payload.len() + 1);
        let b = drive_read(&r, sr.path(), &[4096], payload.len() + 1);
        assert_eq!(a.open as isize, b.open as isize, "row162 corrupt: readOpen");
        assert_eq!(
            a.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            b.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            "row162 corrupt: LZ4F_read returns differ (C={:#x?} Rust={:#x?})",
            a.reads,
            b.reads
        );
        assert!(
            a.reads.iter().any(|&x| is_err_range(x)),
            "row162 corrupt: expected LZ4F_read to report an error, got {:#x?}",
            a.reads
        );
        assert_eq!(a.close, b.close, "row162 corrupt: readClose");

        // truncated frame (endMark and content checksum cut off) must behave
        // identically in both libraries
        let (sc2, sr2) = scratch_pair("truncated");
        write_raw(sc2.path(), &bc[..bc.len() / 2]);
        write_raw(sr2.path(), &br[..br.len() / 2]);
        let a = drive_read(&c, sc2.path(), &[4096], payload.len() + 1);
        let b = drive_read(&r, sr2.path(), &[4096], payload.len() + 1);
        assert_eq!(a.open as isize, b.open as isize, "row162 truncated: readOpen");
        assert_eq!(
            a.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            b.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            "row162 truncated: LZ4F_read returns differ"
        );
        same_full_buffers("row162 truncated payload", &a.data, &b.data);
        assert_eq!(a.close, b.close, "row162 truncated: readClose");
    }
}

// ===========================================================================
// Property-style randomized round-trips (rows 157..162 combined)
// ===========================================================================

#[test]
fn rows_157_158_159_160_161_162_randomized_property_roundtrips() {
    unsafe {
        let mut rng = Rng::new(0xF11E_5EED);
        for iter in 0..40 {
            // random payload
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = match rng.below(4) {
                0 => rng.range(0, 64),
                1 => rng.range(0, 70_000),
                2 => rng.range(60_000, 300_000),
                _ => rng.range(0, 5000),
            };
            let payload = gen(&mut rng, shape, len);

            // random preferences (contentSize either unknown or exact)
            let use_prefs = rng.below(4) != 0;
            let mut p = LZ4F_preferences_t::default();
            if use_prefs {
                p.frameInfo.blockSizeID =
                    [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB]
                        [rng.below(5)];
                p.frameInfo.blockMode = [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT][rng.below(2)];
                p.frameInfo.contentChecksumFlag = rng.below(2) as c_uint;
                p.frameInfo.blockChecksumFlag = rng.below(2) as c_uint;
                p.frameInfo.dictID = if rng.below(2) == 0 { 0 } else { rng.next_u32() };
                p.frameInfo.contentSize = if rng.below(2) == 0 { 0 } else { len as u64 };
                p.compressionLevel = [0i32, 1, 2, -1, 9, 10, 12][rng.below(7)];
                p.autoFlush = rng.below(2) as c_uint;
                p.favorDecSpeed = rng.below(2) as c_uint;
            }

            // random write chunking
            let mut offs = Vec::new();
            let mut off = 0usize;
            while off < len {
                let cap = rng.range(1, 200_000);
                let n = rng.range(1, (len - off).min(cap));
                offs.push((off, n));
                off += n;
            }
            if offs.is_empty() {
                offs.push((0, 0));
            }
            let chunks: Vec<&[u8]> = offs.iter().map(|&(o, n)| &payload[o..o + n]).collect();

            // random read chunking
            let mut sizes = Vec::new();
            for _ in 0..rng.range(1, 5) {
                sizes.push(rng.range(1, 200_000));
            }
            if rng.below(3) == 0 {
                sizes.insert(0, 0);
            }

            roundtrip(
                &format!("property iter={iter} shape={shape:?} len={len} prefs={use_prefs}"),
                if use_prefs { Some(&p) } else { None },
                &chunks,
                &sizes,
            );
        }
    }
}

/// ERRORS rows 161/162/166/172/174/175/176 (allocation and inner-context
/// failures) cannot be triggered without an allocator hook, and row 168
/// (`fread` "negative" branch) is unreachable because `ret` is a `size_t`.
/// This test pins the reachable consequence of row 168's neighbourhood: a
/// zero-length `fread` at EOF breaks the loop and yields a short count rather
/// than `err(23)`.
#[test]
fn errors_168_eof_yields_short_count_not_io_read() {
    unsafe {
        let (c, r) = apis();
        let mut rng = Rng::new(168);
        let payload = gen(&mut rng, Shape::Compressible, 12_345);
        let (sc, sr) = scratch_pair("eof");
        assert_ok("writeClose (C)", drive_write(&c, sc.path(), None, &[&payload]).close);
        assert_ok("writeClose (Rust)", drive_write(&r, sr.path(), None, &[&payload]).close);

        let a = drive_read(&c, sc.path(), &[payload.len() * 3], payload.len() + 1);
        let b = drive_read(&r, sr.path(), &[payload.len() * 3], payload.len() + 1);
        assert_eq!(
            a.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            b.reads.iter().map(|&x| x as isize).collect::<Vec<_>>(),
            "errors168: read returns differ"
        );
        assert_eq!(
            a.reads[0],
            payload.len(),
            "errors168: expected a short count of {}, got {:#x}",
            payload.len(),
            a.reads[0]
        );
        assert!(!is_err_range(a.reads[0]), "errors168: EOF must not be err(23)");
        assert_eq!(a.data, payload);
        assert_eq!(b.data, payload);
    }
}
