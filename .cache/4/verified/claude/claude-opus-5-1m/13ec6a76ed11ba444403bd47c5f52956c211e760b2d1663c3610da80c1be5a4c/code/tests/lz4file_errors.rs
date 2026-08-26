//! Phase C — error-path differential tests for `lz4file.c`.
//!
//! One `#[test]` per row (or per tightly-related group of rows) of the
//! `## lz4file.c` section of `ERRORS.md` — rows **222 … 250**.
//!
//! Every case constructs the exact invalid input/condition named by the row,
//! calls BOTH the C `.so` and the Rust `.so`, and asserts they return the
//! **identical raw `size_t`** and therefore the identical
//! `LZ4F_getErrorCode()` value (`err::ERROR_parameter_null` == 21,
//! `err::ERROR_io_write` == 22, `err::ERROR_io_read` == 23, …).  It is never
//! sufficient here that "both failed somehow".
//!
//! `tests/lz4file_diff.rs` already sweeps the happy paths and has one broad
//! `lz4file_error_paths` test; rows that it also touches are cross-referenced
//! in the per-row comments, but each row still gets its own row-labelled test
//! with an exact expected error code.
//!
//! A complete row 222–250 -> covering-test map (with the reason for every row
//! that provably cannot be tested) is at the END of this file.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

/// Harness rule: both the C-side and the Rust-side output buffers are
/// pre-filled with this byte and compared in FULL.
const SENTINEL: u8 = 0xAA;

// ---------------------------------------------------------------------------
// libc — NOT the library under test.
// ---------------------------------------------------------------------------
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn freopen(path: *const c_char, mode: *const c_char, fp: *mut c_void) -> *mut c_void;
    fn fclose(fp: *mut c_void) -> c_int;
    fn fflush(fp: *mut c_void) -> c_int;
    fn fseek(fp: *mut c_void, off: i64, whence: c_int) -> c_int;
    fn rewind(fp: *mut c_void);
}
const SEEK_END: c_int = 2;

// ---------------------------------------------------------------------------
// Signatures — verified against c_src/include/lz4file.h / c_src/src/lz4file.c
// ---------------------------------------------------------------------------
type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut c_void) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut c_void) -> usize;

// frame-level helpers, used only to MANUFACTURE inputs (always taken from the
// C library so the input itself is never in question).
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressFrameBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;

#[derive(Clone, Copy)]
struct Fns {
    tag: &'static str,
    write_open: FnWriteOpen,
    write: FnWrite,
    write_close: FnWriteClose,
    read_open: FnReadOpen,
    read: FnRead,
    read_close: FnReadClose,
}

/// `(C, Rust)` — obtained exclusively through the shared-library export tables.
fn apis() -> (Fns, Fns) {
    let (cwo, rwo) = both::<FnWriteOpen>("LZ4F_writeOpen");
    let (cw, rw) = both::<FnWrite>("LZ4F_write");
    let (cwc, rwc) = both::<FnWriteClose>("LZ4F_writeClose");
    let (cro, rro) = both::<FnReadOpen>("LZ4F_readOpen");
    let (cr, rr) = both::<FnRead>("LZ4F_read");
    let (crc, rrc) = both::<FnReadClose>("LZ4F_readClose");
    assert_ne!(
        cw as usize, rw as usize,
        "C and Rust LZ4F_write resolved to the same address — the differential \
         comparison would be vacuous"
    );
    assert_ne!(cro as usize, rro as usize, "LZ4F_readOpen aliased");
    (
        Fns {
            tag: "C",
            write_open: cwo,
            write: cw,
            write_close: cwc,
            read_open: cro,
            read: cr,
            read_close: crc,
        },
        Fns {
            tag: "Rust",
            write_open: rwo,
            write: rw,
            write_close: rwc,
            read_open: rro,
            read: rr,
            read_close: rrc,
        },
    )
}

fn ret_str(r: usize) -> String {
    if lz4f_is_error(r) {
        format!("ERROR({})", lz4f_error_code(r))
    } else {
        format!("{}", r)
    }
}

/// C and Rust must return the SAME raw value, and that value must be exactly
/// the LZ4F error code named by the ERRORS.md row.
#[track_caller]
fn same_err(label: &str, c: usize, r: usize, expect: i32) {
    assert_eq!(
        c,
        r,
        "{}: C and Rust disagree: C={} (raw {:#x}) Rust={} (raw {:#x})",
        label,
        ret_str(c),
        c,
        ret_str(r),
        r
    );
    assert!(
        lz4f_is_error(c),
        "{}: expected an LZ4F error, got {}",
        label,
        ret_str(c)
    );
    assert_eq!(
        lz4f_error_code(c),
        expect,
        "{}: expected LZ4F_getErrorCode()=={}, got {}",
        label,
        expect,
        lz4f_error_code(c)
    );
}

/// C and Rust must return the SAME non-error value.
#[track_caller]
fn same_ok(label: &str, c: usize, r: usize) {
    assert_eq!(
        c,
        r,
        "{}: C and Rust disagree: C={} Rust={}",
        label,
        ret_str(c),
        ret_str(r)
    );
    assert!(
        !lz4f_is_error(c),
        "{}: expected success, got {}",
        label,
        ret_str(c)
    );
}

// ---------------------------------------------------------------------------
// Temp files (unique per test, removed on drop)
// ---------------------------------------------------------------------------

struct TmpFile {
    path: String,
    cpath: std::ffi::CString,
}

impl TmpFile {
    fn new(name: &str) -> Self {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let dir = dir.trim_end_matches('/').to_string();
        let path = format!("{}/lz4file_errors_{}_{}.lz4", dir, std::process::id(), name);
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
    /// `freopen` the SAME `FILE*` onto this path in `mode` — used to turn a
    /// live, already-written-to stream into one whose `fwrite` must fail.
    fn reopen(&self, fp: *mut c_void, mode: &str) -> *mut c_void {
        let m = std::ffi::CString::new(mode).unwrap();
        let n = unsafe { freopen(self.cpath.as_ptr(), m.as_ptr(), fp) };
        assert!(!n.is_null(), "freopen({}, {}) failed", self.path, mode);
        n
    }
    fn put(&self, data: &[u8]) {
        std::fs::write(&self.path, data).unwrap_or_else(|e| panic!("write {}: {}", self.path, e));
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap_or_else(|e| panic!("read {}: {}", self.path, e))
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Input manufacturing (all via the C library / raw byte surgery)
// ---------------------------------------------------------------------------

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

fn c_frame(data: &[u8], prefs: &LZ4F_preferences_t) -> Vec<u8> {
    let (bound, _) = both::<FnCompressFrameBound>("LZ4F_compressFrameBound");
    let (frame, _) = both::<FnCompressFrame>("LZ4F_compressFrame");
    unsafe {
        let cap = bound(data.len(), prefs as *const LZ4F_preferences_t).max(64);
        let mut buf = vec![0u8; cap];
        let n = frame(
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

/// LZ4 frame header size implied by the FLG byte (lz4frame.c:1391).
fn header_size(f: &[u8]) -> usize {
    let flg = f[4];
    7 + if (flg >> 3) & 1 == 1 { 8 } else { 0 } + if flg & 1 == 1 { 4 } else { 0 }
}

/// Recompute the 1-byte header checksum, mirroring
/// `LZ4F_headerChecksum(srcPtr+4, frameHeaderSize-5)` (lz4frame.c:349-353),
/// using the library's own exported `LZ4_XXH32`.
fn refresh_header_checksum(f: &mut [u8]) {
    let (xxh, _) = both::<FnXxh32>("LZ4_XXH32");
    let fhs = header_size(f);
    let h = unsafe { xxh(f.as_ptr().add(4) as *const c_void, fhs - 5, 0) };
    f[fhs - 1] = (h >> 8) as u8;
}

// ---------------------------------------------------------------------------
// Read driver
// ---------------------------------------------------------------------------

struct ReadRun {
    open_ret: usize,
    state_null_after_open: bool,
    read_rets: Vec<usize>,
    close_ret: usize,
    buf: Vec<u8>,
}

/// `readOpen` -> up to `ncalls` `LZ4F_read(.., want)` -> `readClose`, on a
/// fresh temp file holding `file`.  The destination buffer is 0xAA-prefilled
/// and returned in full.
fn drive_read(f: &Fns, file: &[u8], want: usize, ncalls: usize, name: &str) -> ReadRun {
    let t = TmpFile::new(&format!("{}_{}", name, f.tag));
    t.put(file);
    let fp = t.open("rb");
    let mut buf = vec![SENTINEL; want * ncalls + 8];
    unsafe {
        let mut st: *mut c_void = std::ptr::null_mut();
        let open_ret = (f.read_open)(&mut st, fp);
        let state_null_after_open = st.is_null();
        let mut read_rets = Vec::new();
        if !lz4f_is_error(open_ret) && !st.is_null() {
            let mut off = 0usize;
            for _ in 0..ncalls {
                let r = (f.read)(st, buf.as_mut_ptr().add(off) as *mut c_void, want);
                read_rets.push(r);
                if lz4f_is_error(r) || r == 0 {
                    break;
                }
                off += r;
            }
        }
        let close_ret = (f.read_close)(st);
        fclose(fp);
        ReadRun {
            open_ret,
            state_null_after_open,
            read_rets,
            close_ret,
            buf,
        }
    }
}

/// Drive both libraries over the same bytes and require full agreement
/// (return values, state NULL-ness, and the complete 0xAA-prefilled buffer).
/// Returns `(C run, Rust run)` so callers can additionally pin exact codes.
fn read_both(
    file: &[u8],
    want: usize,
    ncalls: usize,
    name: &str,
    label: &str,
) -> (ReadRun, ReadRun) {
    let (c, r) = apis();
    let lc = drive_read(&c, file, want, ncalls, name);
    let lr = drive_read(&r, file, want, ncalls, name);
    assert_eq!(
        lc.open_ret, lr.open_ret,
        "{}: LZ4F_readOpen return C={} Rust={}",
        label,
        ret_str(lc.open_ret),
        ret_str(lr.open_ret)
    );
    assert_eq!(
        lc.state_null_after_open, lr.state_null_after_open,
        "{}: state NULL-ness after LZ4F_readOpen",
        label
    );
    assert_eq!(
        lc.read_rets.iter().map(|x| ret_str(*x)).collect::<Vec<_>>(),
        lr.read_rets.iter().map(|x| ret_str(*x)).collect::<Vec<_>>(),
        "{}: LZ4F_read returns",
        label
    );
    assert_eq!(
        lc.close_ret, lr.close_ret,
        "{}: LZ4F_readClose return C={} Rust={}",
        label,
        ret_str(lc.close_ret),
        ret_str(lr.close_ret)
    );
    assert_bytes_eq(&format!("{}: read buffer", label), &lc.buf, &lr.buf);
    (lc, lr)
}

/// The first `LZ4F_read` return value.
fn first_read(run: &ReadRun) -> usize {
    *run.read_rets.first().expect("no LZ4F_read call was made")
}

// ===========================================================================
// ERRORS.md rows 222, 223 — LZ4F_readOpen NULL arguments (lz4file.c:79-81)
// ===========================================================================

/// row 222: `fp == NULL`  -> `parameter_null` (21), `*lz4fRead` untouched.
/// row 223: `lz4fRead == NULL` (out-parameter NULL) -> `parameter_null` (21).
#[test]
fn row_222_223_read_open_null_arguments() {
    let (c, r) = apis();
    let good = c_frame(b"payload for a valid frame", &prefs_of(4, 0, 1, 0, 0, 0, 0, 1));
    let t = TmpFile::new("ro_null");
    t.put(&good);

    unsafe {
        // ---- row 222: fp == NULL. The C returns BEFORE touching *lz4fRead,
        // so a poisoned out-pointer must survive unchanged.
        let poison = 0x5A5A_0000_0000_1234usize as *mut c_void;
        let mut cst: *mut c_void = poison;
        let mut rst: *mut c_void = poison;
        let cv = (c.read_open)(&mut cst, std::ptr::null_mut());
        let rv = (r.read_open)(&mut rst, std::ptr::null_mut());
        same_err(
            "row 222: LZ4F_readOpen(&st, NULL)",
            cv,
            rv,
            err::ERROR_parameter_null,
        );
        assert_eq!(
            cst, poison,
            "row 222: C must leave *lz4fRead untouched when fp==NULL"
        );
        assert_eq!(
            rst, poison,
            "row 222: Rust must leave *lz4fRead untouched when fp==NULL"
        );

        // ---- row 223: lz4fRead == NULL, with a perfectly good FILE*.
        let fp = t.open("rb");
        let cv = (c.read_open)(std::ptr::null_mut(), fp);
        let rv = (r.read_open)(std::ptr::null_mut(), fp);
        same_err(
            "row 223: LZ4F_readOpen(NULL, fp)",
            cv,
            rv,
            err::ERROR_parameter_null,
        );
        fclose(fp);

        // ---- rows 222+223 together: both NULL.
        let cv = (c.read_open)(std::ptr::null_mut(), std::ptr::null_mut());
        let rv = (r.read_open)(std::ptr::null_mut(), std::ptr::null_mut());
        same_err(
            "rows 222/223: LZ4F_readOpen(NULL, NULL)",
            cv,
            rv,
            err::ERROR_parameter_null,
        );
    }
}

// ===========================================================================
// ERRORS.md row 226 — LZ4F_readOpen's unconditional 19-byte fread
//                     (lz4file.c:95-99) -> io_read
// ===========================================================================

/// row 226, three independent ways to make `fread(buf,1,19,fp)` come up short:
///   (a) the file is shorter than `LZ4F_HEADER_SIZE_MAX` (19) — including a
///       *perfectly valid* 15-byte `.lz4` frame, which this API therefore
///       cannot open at all;
///   (b) the stream is already positioned at EOF;
///   (c) the `FILE*` was opened WRITE-ONLY (`"wb"`), so `fread` fails outright.
/// In all three cases the handle must be freed and `*lz4fRead` set to NULL.
/// (`tests/lz4file_diff.rs::lz4file_error_paths` also sweeps truncation
/// lengths 0..=19; this test pins the exact code and the NULL-ing.)
#[test]
fn row_226_read_open_short_or_failing_fread_io_read() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2226);
    let good = c_frame(&gen_shape(&mut rng, 3, 4096), &prefs_of(4, 0, 1, 1, 0, 0, 0, 1));
    assert!(good.len() > 19);

    // (a) every length below LZ4F_HEADER_SIZE_MAX, plus a genuinely valid but
    //     tiny frame (empty content, 7-byte header + 4-byte endMark == 11 B).
    let tiny_valid = c_frame(&[], &prefs_of(4, 0, 0, 0, 0, 0, 0, 1));
    assert!(
        tiny_valid.len() < LZ4F_HEADER_SIZE_MAX,
        "expected a valid frame shorter than 19 bytes, got {}",
        tiny_valid.len()
    );
    for n in 0..LZ4F_HEADER_SIZE_MAX {
        let short = &good[..n];
        let label = format!("row 226a: {}-byte file", n);
        let (run, rrun) = read_both(short, 128, 1, "ro_short", &label);
        same_err(&label, run.open_ret, rrun.open_ret, err::ERROR_io_read);
        assert!(
            run.state_null_after_open,
            "{}: *lz4fRead must be NULL after the io_read failure",
            label
        );
        // and readClose(NULL) then reports parameter_null (row 234)
        assert_eq!(
            lz4f_error_code(run.close_ret),
            err::ERROR_parameter_null,
            "{}: readClose on the NULLed handle",
            label
        );
    }
    {
        let label = format!("row 226a: valid {}-byte frame", tiny_valid.len());
        let (run, rrun) = read_both(&tiny_valid, 128, 1, "ro_tiny", &label);
        same_err(&label, run.open_ret, rrun.open_ret, err::ERROR_io_read);
        assert!(run.state_null_after_open, "{}: handle must be NULLed", label);
    }

    // (b) stream already at EOF.
    unsafe {
        let t = TmpFile::new("ro_eof");
        t.put(&good);
        let mut rets = [0usize; 2];
        let mut nulls = [false; 2];
        for (i, f) in [&c, &r].iter().enumerate() {
            let fp = t.open("rb");
            assert_eq!(fseek(fp, 0, SEEK_END), 0, "fseek to EOF");
            let mut st: *mut c_void = std::ptr::null_mut();
            rets[i] = (f.read_open)(&mut st, fp);
            nulls[i] = st.is_null();
            fclose(fp);
        }
        same_err(
            "row 226b: LZ4F_readOpen on a stream at EOF",
            rets[0],
            rets[1],
            err::ERROR_io_read,
        );
        assert_eq!(nulls[0], nulls[1]);
        assert!(nulls[0], "row 226b: handle must be NULLed");
    }

    // (c) FILE* opened write-only.
    unsafe {
        let t = TmpFile::new("ro_wronly");
        t.put(&good);
        let mut rets = [0usize; 2];
        let mut nulls = [false; 2];
        for (i, f) in [&c, &r].iter().enumerate() {
            // "wb" truncates, so re-seed the contents for each library.
            t.put(&good);
            let fp = t.open("wb");
            let mut st: *mut c_void = std::ptr::null_mut();
            rets[i] = (f.read_open)(&mut st, fp);
            nulls[i] = st.is_null();
            fclose(fp);
        }
        same_err(
            "row 226c: LZ4F_readOpen on a write-only FILE*",
            rets[0],
            rets[1],
            err::ERROR_io_read,
        );
        assert_eq!(nulls[0], nulls[1]);
        assert!(nulls[0], "row 226c: handle must be NULLed");
    }
}

// ===========================================================================
// ERRORS.md row 227 — LZ4F_getFrameInfo failure inside LZ4F_readOpen
//                     (lz4file.c:101-106): the code is returned VERBATIM
// ===========================================================================

/// row 227, one sub-case per distinct `LZ4F_getFrameInfo` rejection, each
/// pinned to its exact code so "returned verbatim" is actually verified:
///   bad magic          -> `frameType_unknown`      (13)
///   FLG reserved bit   -> `reservedFlag_set`       (8)
///   FLG version != 1   -> `headerVersion_wrong`    (6)
///   BD blockSizeID < 4 -> `maxBlockSize_invalid`   (2)
///   BD reserved bits   -> `reservedFlag_set`       (8)
///   bad HC byte        -> `headerChecksum_invalid` (17)
#[test]
fn row_227_read_open_frame_info_errors_verbatim() {
    let mut rng = Rng::new(0x2227);
    let data = gen_shape(&mut rng, 4, 3000);
    // 7-byte header (no contentSize, no dictID) so the mutated byte offsets
    // below are unambiguous.
    let good = c_frame(&data, &prefs_of(4, 0, 0, 0, 0, 0, 0, 1));
    assert_eq!(header_size(&good), 7, "expected a 7-byte frame header");

    let mut cases: Vec<(String, Vec<u8>, i32)> = Vec::new();

    // bad magic (lz4frame.c:1366-1370)
    for &delta in &[1u32, 0x10, 0x1000, 0xFFFF_FFFF] {
        let mut f = good.clone();
        let m = u32::from_le_bytes([f[0], f[1], f[2], f[3]]).wrapping_add(delta);
        f[..4].copy_from_slice(&m.to_le_bytes());
        cases.push((
            format!("bad magic +{:#x}", delta),
            f,
            err::ERROR_frameType_unknown,
        ));
    }
    // FLG reserved bit 1 (lz4frame.c:1384)
    {
        let mut f = good.clone();
        f[4] |= 0x02;
        cases.push(("FLG reserved bit".into(), f, err::ERROR_reservedFlag_set));
    }
    // FLG version field (lz4frame.c:1385)
    for &v in &[0u8, 2, 3] {
        let mut f = good.clone();
        f[4] = (f[4] & 0x3F) | (v << 6);
        cases.push((
            format!("FLG version={}", v),
            f,
            err::ERROR_headerVersion_wrong,
        ));
    }
    // BD blockSizeID < 4 (lz4frame.c:1410)
    for &bsid in &[0u8, 1, 2, 3] {
        let mut f = good.clone();
        f[5] = bsid << 4;
        cases.push((
            format!("BD blockSizeID={}", bsid),
            f,
            err::ERROR_maxBlockSize_invalid,
        ));
    }
    // BD reserved bit 7 and reserved low nibble (lz4frame.c:1409, 1411)
    for &bd in &[0xC0u8, 0x41, 0x4F] {
        let mut f = good.clone();
        f[5] = bd;
        cases.push((
            format!("BD reserved bits {:#04x}", bd),
            f,
            err::ERROR_reservedFlag_set,
        ));
    }
    // header checksum (lz4frame.c:1417-1418)
    for &x in &[0xFFu8, 0x01, 0x80] {
        let mut f = good.clone();
        f[6] ^= x;
        cases.push((
            format!("HC ^= {:#04x}", x),
            f,
            err::ERROR_headerChecksum_invalid,
        ));
    }

    for (name, f, expect) in &cases {
        let label = format!("row 227: {}", name);
        let (run, rrun) = read_both(f, 4096, 1, "ro_hdr", &label);
        same_err(&label, run.open_ret, rrun.open_ret, *expect);
        assert!(
            run.state_null_after_open,
            "{}: handle must be freed and NULLed",
            label
        );
        assert_eq!(
            lz4f_error_code(run.close_ret),
            err::ERROR_parameter_null,
            "{}: readClose on the NULLed handle",
            label
        );
    }
}

// ===========================================================================
// ERRORS.md rows 230, 231 — LZ4F_read NULL arguments (lz4file.c:145-146)
// ===========================================================================

/// row 230: `lz4fRead == NULL` -> `parameter_null` (21), for EVERY size,
///          including 0 and `usize::MAX` (the check precedes the loop).
/// row 231: `buf == NULL` with a perfectly VALID state -> `parameter_null`,
///          again including `size == 0`.
/// Also pins the well-defined boundary `LZ4F_read(valid, valid, 0)` == 0.
#[test]
fn row_230_231_read_null_state_and_null_buffer() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2230);
    let data = gen_shape(&mut rng, 2, 2048);
    let good = c_frame(&data, &prefs_of(4, 0, 1, 0, 0, 0, 0, 1));

    unsafe {
        // ---- row 230: NULL state. Sizes never get used, so even usize::MAX is
        // safe to pass.
        let mut b = vec![SENTINEL; 64];
        for &size in &[0usize, 1, 19, 64, usize::MAX / 2, usize::MAX] {
            let cv = (c.read)(
                std::ptr::null_mut(),
                b.as_mut_ptr() as *mut c_void,
                size,
            );
            let rv = (r.read)(
                std::ptr::null_mut(),
                b.as_mut_ptr() as *mut c_void,
                size,
            );
            same_err(
                &format!("row 230: LZ4F_read(NULL, buf, {})", size),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }
        assert!(
            b.iter().all(|&x| x == SENTINEL),
            "row 230: the destination buffer must not be touched"
        );
        // NULL state AND NULL buffer
        for &size in &[0usize, 32, usize::MAX] {
            let cv = (c.read)(std::ptr::null_mut(), std::ptr::null_mut(), size);
            let rv = (r.read)(std::ptr::null_mut(), std::ptr::null_mut(), size);
            same_err(
                &format!("rows 230/231: LZ4F_read(NULL, NULL, {})", size),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }

        // ---- row 231: NULL buffer, VALID state.
        let mut rets: Vec<Vec<usize>> = Vec::new();
        let mut zero_rets: Vec<usize> = Vec::new();
        for f in [&c, &r] {
            let t = TmpFile::new(&format!("rd_nullbuf_{}", f.tag));
            t.put(&good);
            let fp = t.open("rb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.read_open)(&mut st, fp);
            assert!(
                !lz4f_is_error(o) && !st.is_null(),
                "{}: readOpen on a good frame must succeed, got {}",
                f.tag,
                ret_str(o)
            );
            let mut v = Vec::new();
            for &size in &[0usize, 1, 16, usize::MAX] {
                v.push((f.read)(st, std::ptr::null_mut(), size));
            }
            // boundary: size == 0 with a valid buffer returns 0, not an error
            let mut small = vec![SENTINEL; 8];
            zero_rets.push((f.read)(st, small.as_mut_ptr() as *mut c_void, 0));
            assert!(
                small.iter().all(|&x| x == SENTINEL),
                "{}: size==0 must not write",
                f.tag
            );
            rets.push(v);
            assert!(!lz4f_is_error((f.read_close)(st)));
            fclose(fp);
        }
        for (i, &size) in [0usize, 1, 16, usize::MAX].iter().enumerate() {
            same_err(
                &format!("row 231: LZ4F_read(valid, NULL, {})", size),
                rets[0][i],
                rets[1][i],
                err::ERROR_parameter_null,
            );
        }
        same_ok(
            "boundary: LZ4F_read(valid, valid, 0)",
            zero_rets[0],
            zero_rets[1],
        );
        assert_eq!(zero_rets[0], 0, "LZ4F_read(.., 0) must return 0");
    }
}

// ===========================================================================
// ERRORS.md row 232 — the `else { RETURN_ERROR(io_read); }` arm of LZ4F_read
//                     (lz4file.c:159-163) is DEAD CODE
// ===========================================================================

/// row 232 is unreachable: `ret` is a `size_t` (lz4file.c:151), so after
/// `if (ret > 0)` (lz4file.c:155) and `else if (ret == 0)` (lz4file.c:159)
/// the trailing `else` at lz4file.c:161-163 can never be entered — a failed
/// `fread` is indistinguishable from EOF and takes the `break` at
/// lz4file.c:160.
///
/// What IS observable (and what this test asserts C and Rust agree on) is the
/// consequence: a truncated frame yields a SHORT read with **no** error
/// indication.  `LZ4F_read` returns the number of bytes decoded so far and
/// `LZ4F_readClose` still reports success.
#[test]
fn row_232_read_short_read_at_eof_is_not_an_error() {
    let mut rng = Rng::new(0x2232);
    // shape 0 == incompressible random bytes, so the frame really is ~20 KB and
    // the truncation points below genuinely destroy payload.
    let data = gen_shape(&mut rng, 0, 20_000);
    let good = c_frame(&data, &prefs_of(4, LZ4F_blockIndependent, 0, 0, 0, 0, 0, 1));
    assert!(
        good.len() > 10_000,
        "expected an incompressible frame, got {} bytes",
        good.len()
    );

    // Truncate inside the payload (never inside the 19-byte header, which
    // would be row 226 instead).
    for &cut in &[20usize, 25, 40, 100, 1000] {
        if cut >= good.len() {
            continue;
        }
        let label = format!("row 232: frame truncated to {} bytes", cut);
        let (run, rrun) = read_both(&good[..cut], data.len() + 4096, 2, "rd_trunc", &label);
        same_ok(
            &format!("{}: readOpen", label),
            run.open_ret,
            rrun.open_ret,
        );
        let n = first_read(&run);
        assert_eq!(n, first_read(&rrun), "{}: LZ4F_read return", label);
        assert!(
            !lz4f_is_error(n),
            "{}: a short/failed fread must NOT surface as an error, got {}",
            label,
            ret_str(n)
        );
        assert!(
            n < data.len(),
            "{}: expected a short read, got {} of {}",
            label,
            n,
            data.len()
        );
        assert!(
            !lz4f_is_error(run.close_ret),
            "{}: readClose must still succeed",
            label
        );
    }

    // A complete frame read past its end: the extra call returns 0, not an error.
    let label = "row 232: reading past the end of a complete frame";
    let (run, rrun) = read_both(&good, data.len() + 4096, 3, "rd_pasteof", label);
    same_ok("row 232: readOpen (complete frame)", run.open_ret, rrun.open_ret);
    assert_eq!(
        first_read(&run),
        data.len(),
        "row 232: the whole payload should come back"
    );
    assert_eq!(
        run.read_rets.get(1).copied(),
        Some(0),
        "row 232: the read past EOF must return 0 (break), not io_read"
    );
}

// ===========================================================================
// ERRORS.md row 233 — LZ4F_decompress failure inside LZ4F_read
//                     (lz4file.c:166-173): the code is returned VERBATIM
// ===========================================================================

/// row 233, one sub-case per distinct `LZ4F_decompress` rejection, each pinned
/// to its exact code:
///   block-header size > maxBlockSize -> `maxBlockSize_invalid`   (2)
///   corrupt block checksum           -> `blockChecksum_invalid`  (7)
///   undecodable LZ4 block payload    -> `decompressionFailed`    (16)
///   corrupt content checksum         -> `contentChecksum_invalid`(18)
///   declared contentSize too large   -> `frameSize_wrong`        (14)
/// The row also notes the handle is NOT freed, so `LZ4F_readClose` must still
/// succeed afterwards — asserted for every case.
#[test]
fn row_233_read_decompress_errors_verbatim() {
    let mut rng = Rng::new(0x2233);
    let data = gen_shape(&mut rng, 3, 3000);

    let mut cases: Vec<(String, Vec<u8>, i32)> = Vec::new();

    // --- (1) block header claims more than maxBlockSize (lz4frame.c:1737-1738)
    {
        let base = c_frame(&data, &prefs_of(4, LZ4F_blockIndependent, 0, 0, 0, 0, 0, 1));
        let hs = header_size(&base);
        for &sz in &[65_537u32, 0x0010_0000, 0x7FFF_FFFF] {
            let mut f = base.clone();
            f[hs..hs + 4].copy_from_slice(&sz.to_le_bytes());
            cases.push((
                format!("block size field = {:#x}", sz),
                f,
                err::ERROR_maxBlockSize_invalid,
            ));
        }
    }

    // --- (2) block checksum (one block, blockChecksum on, contentChecksum off,
    //         so the layout is header | size(4) | data | blockCRC(4) | endMark(4))
    {
        let mut f = c_frame(
            &data,
            &prefs_of(4, LZ4F_blockIndependent, 0, LZ4F_blockChecksumEnabled, 0, 0, 0, 1),
        );
        let n = f.len();
        f[n - 5] ^= 0xFF; // last byte of the block checksum
        cases.push((
            "corrupt block checksum".into(),
            f,
            err::ERROR_blockChecksum_invalid,
        ));
    }

    // --- (3) an undecodable LZ4 block: token 0x10 promises one literal plus a
    //         2-byte match offset, but the block is only 1 byte long, so
    //         LZ4_decompress_safe rejects it.
    {
        let hdr = c_frame(&[], &prefs_of(4, LZ4F_blockIndependent, 0, 0, 0, 0, 0, 1));
        let h = &hdr[..header_size(&hdr)];
        let mut f = h.to_vec();
        let bad_block: [u8; 1] = [0x10];
        for _ in 0..3 {
            f.extend_from_slice(&(bad_block.len() as u32).to_le_bytes());
            f.extend_from_slice(&bad_block);
        }
        f.extend_from_slice(&0u32.to_le_bytes()); // endMark
        assert!(f.len() >= LZ4F_HEADER_SIZE_MAX);
        cases.push((
            "undecodable LZ4 block payload".into(),
            f,
            err::ERROR_decompressionFailed,
        ));
    }

    // --- (4) content checksum: last 4 bytes of the frame
    {
        let mut f = c_frame(
            &data,
            &prefs_of(4, LZ4F_blockIndependent, LZ4F_contentChecksumEnabled, 0, 0, 0, 0, 1),
        );
        let n = f.len();
        f[n - 1] ^= 0xFF;
        cases.push((
            "corrupt content checksum".into(),
            f,
            err::ERROR_contentChecksum_invalid,
        ));
    }

    // --- (5) declared contentSize larger than the real payload: patch the
    //         8-byte field and repair the header checksum so the frame is
    //         otherwise perfectly well formed.
    {
        let mut f = c_frame(
            &data,
            &prefs_of(4, LZ4F_blockIndependent, 0, 0, data.len() as u64, 0, 0, 1),
        );
        assert_eq!(header_size(&f), 15, "expected magic|FLG|BD|contentSize|HC");
        let lie = (data.len() as u64) + 1;
        f[6..14].copy_from_slice(&lie.to_le_bytes());
        refresh_header_checksum(&mut f);
        cases.push((
            format!("declared contentSize {} vs real {}", lie, data.len()),
            f,
            err::ERROR_frameSize_wrong,
        ));
    }

    for (name, f, expect) in &cases {
        let label = format!("row 233: {}", name);
        // Ask for MORE than the payload so the frame suffix is actually reached.
        let (run, rrun) = read_both(f, data.len() + 4096, 1, "rd_dec", &label);
        same_ok(&format!("{}: readOpen", label), run.open_ret, rrun.open_ret);
        same_err(
            &format!("{}: LZ4F_read", label),
            first_read(&run),
            first_read(&rrun),
            *expect,
        );
        assert!(
            !lz4f_is_error(run.close_ret),
            "{}: the handle is NOT freed by LZ4F_read, so readClose must \
             succeed, got {}",
            label,
            ret_str(run.close_ret)
        );
    }
}

// ===========================================================================
// ERRORS.md row 234 — LZ4F_readClose(NULL) (lz4file.c:185-186)
// ===========================================================================

/// row 234: `lz4fRead == NULL` -> `parameter_null` (21).  The C tolerates it
/// (explicit NULL check, no dereference), so it is safe to call repeatedly.
#[test]
fn row_234_read_close_null() {
    let (c, r) = apis();
    unsafe {
        for i in 0..3 {
            let cv = (c.read_close)(std::ptr::null_mut());
            let rv = (r.read_close)(std::ptr::null_mut());
            same_err(
                &format!("row 234: LZ4F_readClose(NULL) call #{}", i),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 235, 236 — LZ4F_writeOpen NULL arguments (lz4file.c:222-223)
// ===========================================================================

/// row 235: `fp == NULL` -> `parameter_null` (21); `*lz4fWrite` untouched.
/// row 236: `lz4fWrite == NULL` -> `parameter_null` (21).
/// Both are checked with a non-NULL prefs and with a NULL prefs.
#[test]
fn row_235_236_write_open_null_arguments() {
    let (c, r) = apis();
    let prefs = prefs_of(4, 0, 1, 1, 0, 0, 0, 1);
    let t = TmpFile::new("wo_null");
    t.put(b"placeholder");

    unsafe {
        for &use_prefs in &[true, false] {
            let pp = if use_prefs {
                &prefs as *const LZ4F_preferences_t
            } else {
                std::ptr::null()
            };

            // row 235: NULL fp — the C returns before writing *lz4fWrite.
            let poison = 0x5A5A_0000_0000_4321usize as *mut c_void;
            let mut cst: *mut c_void = poison;
            let mut rst: *mut c_void = poison;
            let cv = (c.write_open)(&mut cst, std::ptr::null_mut(), pp);
            let rv = (r.write_open)(&mut rst, std::ptr::null_mut(), pp);
            same_err(
                &format!("row 235: LZ4F_writeOpen(&st, NULL, prefs={})", use_prefs),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
            assert_eq!(cst, poison, "row 235: C must leave *lz4fWrite untouched");
            assert_eq!(rst, poison, "row 235: Rust must leave *lz4fWrite untouched");

            // row 236: NULL out-parameter, with a real writable FILE*.
            let fp = t.open("wb");
            let cv = (c.write_open)(std::ptr::null_mut(), fp, pp);
            let rv = (r.write_open)(std::ptr::null_mut(), fp, pp);
            same_err(
                &format!("row 236: LZ4F_writeOpen(NULL, fp, prefs={})", use_prefs),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
            fclose(fp);
            assert_eq!(
                t.bytes().len(),
                0,
                "row 236: nothing may be written when the args are rejected"
            );

            // both NULL
            let cv = (c.write_open)(std::ptr::null_mut(), std::ptr::null_mut(), pp);
            let rv = (r.write_open)(std::ptr::null_mut(), std::ptr::null_mut(), pp);
            same_err(
                "rows 235/236: LZ4F_writeOpen(NULL, NULL, ..)",
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 238 — LZ4F_writeOpen blockSizeID switch `default:`
//                     (lz4file.c:244-246)
// ===========================================================================

/// row 238: `prefsPtr != NULL` and `blockSizeID` outside {0,4,5,6,7} ->
/// `maxBlockSize_invalid` (2), handle freed and `*lz4fWrite` set to NULL, and
/// nothing at all written to the file.  (`tests/lz4file_diff.rs::
/// lz4file_error_paths` sweeps a subset of these ids; here the extremes of
/// `int` are added and the file emptiness is pinned.)
#[test]
fn row_238_write_open_invalid_block_size_id() {
    let (c, r) = apis();
    for &bsid in &[
        c_int::MIN,
        -1000,
        -7,
        -1,
        1,
        2,
        3,
        8,
        9,
        99,
        0x7FFF,
        c_int::MAX,
    ] {
        let p = prefs_of(bsid, 0, 0, 0, 0, 0, 0, 0);
        let mut rets = [0usize; 2];
        let mut nulls = [false; 2];
        let mut files: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for (i, f) in [&c, &r].iter().enumerate() {
                let t = TmpFile::new(&format!("wo_bsid_{}", f.tag));
                let fp = t.open("wb");
                let mut st: *mut c_void = std::ptr::null_mut();
                rets[i] = (f.write_open)(&mut st, fp, &p as *const LZ4F_preferences_t);
                nulls[i] = st.is_null();
                fflush(fp);
                fclose(fp);
                files.push(t.bytes());
            }
        }
        let label = format!("row 238: LZ4F_writeOpen(blockSizeID={})", bsid);
        same_err(&label, rets[0], rets[1], err::ERROR_maxBlockSize_invalid);
        assert_eq!(nulls[0], nulls[1], "{}: state NULL-ness", label);
        assert!(nulls[0], "{}: *lz4fWrite must be NULL", label);
        assert_bytes_eq(&format!("{}: file bytes", label), &files[0], &files[1]);
        assert_eq!(files[0].len(), 0, "{}: nothing may be written", label);
    }
}

// ===========================================================================
// ERRORS.md row 242 — the frame-header fwrite fails (lz4file.c:271-274)
// ===========================================================================

/// row 242: `fwrite(buf, 1, ret, fp) != ret` while writing the frame header
/// -> `io_write` (22), handle freed and `*lz4fWrite` set to NULL.
///
/// Reproduced with a `FILE*` opened READ-ONLY (`"rb"`): glibc rejects the
/// write immediately (the stream has no write buffer), so this is the branch
/// the C actually detects.  Also reproduced by `freopen`-ing a writable
/// stream into read-only mode just before `LZ4F_writeOpen`.
#[test]
fn row_242_write_open_header_io_write() {
    let (c, r) = apis();
    let prefs = prefs_of(4, 0, 1, 1, 0, 0, 0, 1);

    // (a) plain read-only FILE*
    for &use_prefs in &[true, false] {
        let pp = if use_prefs {
            &prefs as *const LZ4F_preferences_t
        } else {
            std::ptr::null()
        };
        let mut rets = [0usize; 2];
        let mut nulls = [false; 2];
        unsafe {
            for (i, f) in [&c, &r].iter().enumerate() {
                let t = TmpFile::new(&format!("wo_ro_{}", f.tag));
                t.put(b"the file must exist for \"rb\" to succeed");
                let fp = t.open("rb");
                let mut st: *mut c_void = std::ptr::null_mut();
                rets[i] = (f.write_open)(&mut st, fp, pp);
                nulls[i] = st.is_null();
                fclose(fp);
            }
        }
        same_err(
            &format!("row 242a: writeOpen on a read-only FILE* (prefs={})", use_prefs),
            rets[0],
            rets[1],
            err::ERROR_io_write,
        );
        assert_eq!(nulls[0], nulls[1]);
        assert!(nulls[0], "row 242a: *lz4fWrite must be NULL");
    }

    // (b) same FILE* object, flipped to read-only by freopen
    {
        let mut rets = [0usize; 2];
        let mut nulls = [false; 2];
        unsafe {
            for (i, f) in [&c, &r].iter().enumerate() {
                let t = TmpFile::new(&format!("wo_reopen_{}", f.tag));
                let fp = t.open("wb");
                let fp = t.reopen(fp, "rb");
                let mut st: *mut c_void = std::ptr::null_mut();
                rets[i] = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
                nulls[i] = st.is_null();
                fclose(fp);
            }
        }
        same_err(
            "row 242b: writeOpen after freopen(.., \"rb\")",
            rets[0],
            rets[1],
            err::ERROR_io_write,
        );
        assert_eq!(nulls[0], nulls[1]);
        assert!(nulls[0]);
    }
}

// ===========================================================================
// ERRORS.md rows 243, 244 — LZ4F_write NULL arguments (lz4file.c:288-289)
// ===========================================================================

/// row 243: `lz4fWrite == NULL` -> `parameter_null` (21), for every size
///          including 0 and `usize::MAX`.
/// row 244: `buf == NULL` with a VALID state -> `parameter_null`, including
///          `size == 0` (the check is unconditional, before the loop).
/// Also pins the well-defined boundary `LZ4F_write(valid, valid, 0)` == 0.
#[test]
fn row_243_244_write_null_state_and_null_buffer() {
    let (c, r) = apis();
    let prefs = prefs_of(4, 0, 1, 1, 0, 0, 0, 1);
    let src = vec![0x5Au8; 128];

    unsafe {
        // ---- row 243: NULL state, any size (the size is never used).
        for &size in &[0usize, 1, 128, usize::MAX / 2, usize::MAX] {
            let cv = (c.write)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_void,
                size,
            );
            let rv = (r.write)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_void,
                size,
            );
            same_err(
                &format!("row 243: LZ4F_write(NULL, buf, {})", size),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }
        for &size in &[0usize, 64, usize::MAX] {
            let cv = (c.write)(std::ptr::null_mut(), std::ptr::null(), size);
            let rv = (r.write)(std::ptr::null_mut(), std::ptr::null(), size);
            same_err(
                &format!("rows 243/244: LZ4F_write(NULL, NULL, {})", size),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }

        // ---- row 244: NULL buffer, VALID state.
        let sizes = [0usize, 1, 128, usize::MAX];
        let mut rets: Vec<Vec<usize>> = Vec::new();
        let mut zero_rets: Vec<usize> = Vec::new();
        let mut closes: Vec<usize> = Vec::new();
        let mut files: Vec<Vec<u8>> = Vec::new();
        for f in [&c, &r] {
            let t = TmpFile::new(&format!("wr_nullbuf_{}", f.tag));
            let fp = t.open("wb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
            assert!(
                !lz4f_is_error(o) && !st.is_null(),
                "{}: writeOpen must succeed, got {}",
                f.tag,
                ret_str(o)
            );
            let mut v = Vec::new();
            for &size in &sizes {
                v.push((f.write)(st, std::ptr::null(), size));
            }
            // boundary: size 0 with a real pointer is a no-op returning 0
            zero_rets.push((f.write)(st, src.as_ptr() as *const c_void, 0));
            rets.push(v);
            closes.push((f.write_close)(st));
            fflush(fp);
            fclose(fp);
            files.push(t.bytes());
        }
        for (i, &size) in sizes.iter().enumerate() {
            same_err(
                &format!("row 244: LZ4F_write(valid, NULL, {})", size),
                rets[0][i],
                rets[1][i],
                err::ERROR_parameter_null,
            );
        }
        same_ok(
            "boundary: LZ4F_write(valid, valid, 0)",
            zero_rets[0],
            zero_rets[1],
        );
        assert_eq!(zero_rets[0], 0, "LZ4F_write(.., 0) must return 0");
        // NULL-buffer rejection does NOT latch errCode, so writeClose still
        // finalises the (empty) frame identically in both libraries.
        same_ok("row 244: writeClose afterwards", closes[0], closes[1]);
        assert_bytes_eq("row 244: resulting file", &files[0], &files[1]);
    }
}

// ===========================================================================
// ERRORS.md rows 246 + 249 — the payload fwrite fails, and LZ4F_writeClose
//                            then MASKS the latched error
//                            (lz4file.c:305-308 and lz4file.c:325)
// ===========================================================================

/// row 246: `fwrite(dstBuf, 1, ret, fp) != ret` for a compressed chunk ->
///          `io_write` (22); `lz4fWrite->errCode` is latched to the same value.
/// row 249: because `errCode != LZ4F_OK_NoError`, `LZ4F_writeClose` skips the
///          whole finalize block and returns `LZ4F_OK_NoError` (**0**) — the
///          latched error is silently discarded and no endMark is written, so
///          the file on disk is a truncated frame.
///
/// The stream is broken AFTER a successful `LZ4F_writeOpen` by `freopen`-ing
/// the same `FILE*` into read-only mode, so `fwrite` fails deterministically
/// (no dependence on stdio buffer sizes).  `autoFlush=1` guarantees the
/// compressUpdate return value is > 0, which is what makes the `fwrite`
/// mismatch observable at all.
#[test]
fn row_246_249_write_payload_io_write_and_close_masks_it() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2246);
    let data = gen_shape(&mut rng, 0, 40_000); // incompressible => big blocks
    let prefs = prefs_of(4, LZ4F_blockIndependent, 1, 1, 0, 0, 0, 1);

    let mut w1 = [0usize; 2];
    let mut w2 = [0usize; 2];
    let mut closes = [0usize; 2];
    let mut files: Vec<Vec<u8>> = Vec::new();

    unsafe {
        for (i, f) in [&c, &r].iter().enumerate() {
            let t = TmpFile::new(&format!("wr_iow_{}", f.tag));
            let fp = t.open("wb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
            assert!(
                !lz4f_is_error(o) && !st.is_null(),
                "{}: writeOpen must succeed first, got {}",
                f.tag,
                ret_str(o)
            );
            // break the stream: same FILE*, now read-only
            let fp = t.reopen(fp, "rb");
            w1[i] = (f.write)(st, data.as_ptr() as *const c_void, data.len());
            // a second write keeps failing the same way
            w2[i] = (f.write)(st, data.as_ptr() as *const c_void, data.len());
            closes[i] = (f.write_close)(st);
            fclose(fp);
            files.push(t.bytes());
        }
    }

    same_err(
        "row 246: LZ4F_write with a broken stream",
        w1[0],
        w1[1],
        err::ERROR_io_write,
    );
    same_err(
        "row 246: second LZ4F_write with a broken stream",
        w2[0],
        w2[1],
        err::ERROR_io_write,
    );
    // row 249: the latched error is DISCARDED — writeClose returns exactly 0.
    assert_eq!(
        closes[0], closes[1],
        "row 249: writeClose return C={} Rust={}",
        ret_str(closes[0]),
        ret_str(closes[1])
    );
    assert_eq!(
        closes[0], 0,
        "row 249: LZ4F_writeClose must return LZ4F_OK_NoError (0) even though \
         LZ4F_write latched io_write; got {}",
        ret_str(closes[0])
    );
    assert_bytes_eq(
        "row 249: truncated frame left on disk",
        &files[0],
        &files[1],
    );
    // The frame really is unterminated: only the header made it out (it was
    // still in the stdio buffer and got flushed by freopen).
    assert!(
        files[0].len() < data.len(),
        "row 249: the file must be a truncated frame, got {} bytes",
        files[0].len()
    );
}

// ===========================================================================
// ERRORS.md row 247 — LZ4F_writeClose(NULL) (lz4file.c:321-323)
// ===========================================================================

/// row 247: `lz4fWrite == NULL` -> `parameter_null` (21).  The C tolerates it
/// (explicit NULL check before any dereference), so repeated calls are safe;
/// this is also the only well-defined form of a "double close", since a real
/// `LZ4F_writeClose` frees the handle (lz4file.c:339) and re-using that
/// pointer would be a use-after-free.
#[test]
fn row_247_write_close_null() {
    let (c, r) = apis();
    unsafe {
        for i in 0..3 {
            let cv = (c.write_close)(std::ptr::null_mut());
            let rv = (r.write_close)(std::ptr::null_mut());
            same_err(
                &format!("row 247: LZ4F_writeClose(NULL) call #{}", i),
                cv,
                rv,
                err::ERROR_parameter_null,
            );
        }
    }

    // The documented close-then-close-again pattern: after a successful close
    // the caller must NULL its handle; closing the NULL is parameter_null.
    let prefs = prefs_of(4, 0, 0, 0, 0, 0, 0, 1);
    let mut first = [0usize; 2];
    let mut second = [0usize; 2];
    unsafe {
        for (i, f) in [&c, &r].iter().enumerate() {
            let t = TmpFile::new(&format!("wc_double_{}", f.tag));
            let fp = t.open("wb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
            assert!(!lz4f_is_error(o));
            first[i] = (f.write_close)(st);
            st = std::ptr::null_mut();
            second[i] = (f.write_close)(st);
            fclose(fp);
        }
    }
    same_ok("row 247: first writeClose", first[0], first[1]);
    same_err(
        "row 247: second writeClose on the NULLed handle",
        second[0],
        second[1],
        err::ERROR_parameter_null,
    );
}

// ===========================================================================
// ERRORS.md row 248 — LZ4F_compressEnd fails inside LZ4F_writeClose
//                     (lz4file.c:326-331)
// ===========================================================================

/// row 248: the `LZ4F_compressEnd` error is returned verbatim.  The reachable
/// case is `frameSize_wrong` (14): declare `frameInfo.contentSize` at
/// `LZ4F_writeOpen` and then feed a different number of bytes.  The handle is
/// still freed via the `out:` label, and the (unterminated-but-flushed) file
/// must match byte for byte.
/// (`tests/lz4file_diff.rs::write_content_size_enforcement` covers the same
/// row from the happy-path side; this test pins the exact code across a range
/// of declared/actual mismatches, in both directions.)
#[test]
fn row_248_write_close_compress_end_frame_size_wrong() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2248);
    let data = gen_shape(&mut rng, 5, 5000);

    for &declared in &[
        1u64,
        (data.len() as u64) - 1,
        (data.len() as u64) + 1,
        data.len() as u64 * 2,
        1u64 << 40,
    ] {
        let prefs = prefs_of(4, 0, 1, 0, declared, 0, 0, 1);
        let mut opens = [0usize; 2];
        let mut writes = [0usize; 2];
        let mut closes = [0usize; 2];
        let mut files: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for (i, f) in [&c, &r].iter().enumerate() {
                let t = TmpFile::new(&format!("wc_csz_{}", f.tag));
                let fp = t.open("wb");
                let mut st: *mut c_void = std::ptr::null_mut();
                opens[i] = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
                assert!(!lz4f_is_error(opens[i]) && !st.is_null());
                writes[i] = (f.write)(st, data.as_ptr() as *const c_void, data.len());
                closes[i] = (f.write_close)(st);
                fflush(fp);
                fclose(fp);
                files.push(t.bytes());
            }
        }
        let label = format!(
            "row 248: declared contentSize {} vs {} bytes written",
            declared,
            data.len()
        );
        same_ok(&format!("{}: writeOpen", label), opens[0], opens[1]);
        same_ok(&format!("{}: LZ4F_write", label), writes[0], writes[1]);
        same_err(
            &format!("{}: LZ4F_writeClose", label),
            closes[0],
            closes[1],
            err::ERROR_frameSize_wrong,
        );
        assert_bytes_eq(&format!("{}: file bytes", label), &files[0], &files[1]);
    }
}

// ===========================================================================
// ERRORS.md row 250 — the frame-footer fwrite fails (lz4file.c:333-335)
// ===========================================================================

/// row 250: `errCode` is still `LZ4F_OK_NoError`, `LZ4F_compressEnd` succeeds,
/// but `fwrite(dstBuf, 1, ret, fp) != ret` for the endMark (+ optional content
/// checksum) -> `io_write` (22).  The handle is still freed.
///
/// This is the exact complement of row 249: the same broken stream yields 22
/// here (nothing latched) and 0 there (io_write latched by `LZ4F_write`),
/// which is precisely the masking bug that row 249 describes.
#[test]
fn row_250_write_close_footer_io_write() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2250);
    let data = gen_shape(&mut rng, 2, 4096);

    for &cc in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
        for &write_something in &[false, true] {
            let prefs = prefs_of(4, LZ4F_blockIndependent, cc, 0, 0, 0, 0, 1);
            let mut closes = [0usize; 2];
            let mut files: Vec<Vec<u8>> = Vec::new();
            unsafe {
                for (i, f) in [&c, &r].iter().enumerate() {
                    let t = TmpFile::new(&format!("wc_iow_{}", f.tag));
                    let fp = t.open("wb");
                    let mut st: *mut c_void = std::ptr::null_mut();
                    let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
                    assert!(!lz4f_is_error(o) && !st.is_null());
                    if write_something {
                        // Small enough to stay inside the stdio buffer, so this
                        // write succeeds and does NOT latch errCode.
                        let w = (f.write)(st, data.as_ptr() as *const c_void, 8);
                        assert!(
                            !lz4f_is_error(w),
                            "{}: the priming write must succeed, got {}",
                            f.tag,
                            ret_str(w)
                        );
                    }
                    // break the stream only now
                    let fp = t.reopen(fp, "rb");
                    closes[i] = (f.write_close)(st);
                    fclose(fp);
                    files.push(t.bytes());
                }
            }
            let label = format!(
                "row 250: writeClose footer fwrite failure (cc={}, primed={})",
                cc, write_something
            );
            same_err(&label, closes[0], closes[1], err::ERROR_io_write);
            assert_bytes_eq(&format!("{}: file bytes", label), &files[0], &files[1]);
        }
    }
}

// ===========================================================================
// Extra boundary coverage requested alongside the rows above: NULL/huge sizes
// and the "use after close" question.
// ===========================================================================

/// `LZ4F_read` / `LZ4F_write` after the matching `*Close` is a **use after
/// free**: `LZ4F_readClose` -> `LZ4F_freeReadFile` -> `free(lz4fRead)`
/// (lz4file.c:63, reached from lz4file.c:187) and `LZ4F_writeClose` ->
/// `LZ4F_freeWriteFile` -> `free(state)` (lz4file.c:207, reached from
/// lz4file.c:339).  Neither library can detect it, so it is NOT exercised;
/// what IS well defined is that the caller NULLs its handle, which both
/// libraries then reject identically.  This test pins that, plus the huge-size
/// and zero-size boundaries on a closed/NULL handle.
#[test]
fn boundary_after_close_nulled_handle_and_extreme_sizes() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2299);
    let data = gen_shape(&mut rng, 4, 1024);
    let prefs = prefs_of(4, 0, 1, 0, 0, 0, 0, 1);
    let good = c_frame(&data, &prefs);

    let mut buf = vec![SENTINEL; 256];
    unsafe {
        // read side: open, close, NULL the handle, then read/close again
        let mut reads = [0usize; 2];
        let mut closes2 = [0usize; 2];
        for (i, f) in [&c, &r].iter().enumerate() {
            let t = TmpFile::new(&format!("uac_r_{}", f.tag));
            t.put(&good);
            let fp = t.open("rb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.read_open)(&mut st, fp);
            assert!(!lz4f_is_error(o) && !st.is_null());
            let cl = (f.read_close)(st);
            assert!(!lz4f_is_error(cl), "{}: readClose", f.tag);
            st = std::ptr::null_mut();
            reads[i] = (f.read)(st, buf.as_mut_ptr() as *mut c_void, usize::MAX);
            closes2[i] = (f.read_close)(st);
            fclose(fp);
        }
        same_err(
            "boundary: LZ4F_read after close (handle NULLed), size=usize::MAX",
            reads[0],
            reads[1],
            err::ERROR_parameter_null,
        );
        same_err(
            "boundary: LZ4F_readClose twice (handle NULLed)",
            closes2[0],
            closes2[1],
            err::ERROR_parameter_null,
        );

        // write side: open, close, NULL the handle, then write/close again
        let mut writes = [0usize; 2];
        let mut closes2 = [0usize; 2];
        for (i, f) in [&c, &r].iter().enumerate() {
            let t = TmpFile::new(&format!("uac_w_{}", f.tag));
            let fp = t.open("wb");
            let mut st: *mut c_void = std::ptr::null_mut();
            let o = (f.write_open)(&mut st, fp, &prefs as *const LZ4F_preferences_t);
            assert!(!lz4f_is_error(o) && !st.is_null());
            let cl = (f.write_close)(st);
            assert!(!lz4f_is_error(cl), "{}: writeClose", f.tag);
            st = std::ptr::null_mut();
            writes[i] = (f.write)(st, data.as_ptr() as *const c_void, usize::MAX);
            closes2[i] = (f.write_close)(st);
            fflush(fp);
            fclose(fp);
        }
        same_err(
            "boundary: LZ4F_write after close (handle NULLed), size=usize::MAX",
            writes[0],
            writes[1],
            err::ERROR_parameter_null,
        );
        same_err(
            "boundary: LZ4F_writeClose twice (handle NULLed)",
            closes2[0],
            closes2[1],
            err::ERROR_parameter_null,
        );
    }
    assert!(
        buf.iter().all(|&x| x == SENTINEL),
        "boundary: no rejected call may touch the destination buffer"
    );
}

/// `LZ4F_readOpen` succeeds, is rewound, and re-opened: a sanity control that
/// the error tests above are not accidentally passing because *every* call
/// fails.  Uses the same harness style (0xAA prefill, full-buffer compare).
#[test]
fn control_valid_path_still_agrees() {
    let mut rng = Rng::new(0x22FF);
    let data = gen_shape(&mut rng, 3, 6000);
    let good = c_frame(&data, &prefs_of(5, LZ4F_blockIndependent, 1, 1, 0, 0, 0, 1));
    let (run, rrun) = read_both(&good, data.len() + 64, 1, "control", "control: valid frame");
    same_ok("control: readOpen", run.open_ret, rrun.open_ret);
    let n = first_read(&run);
    assert_eq!(n, data.len(), "control: whole payload must decode");
    assert_bytes_eq("control: decoded bytes", &data, &run.buf[..n]);
    assert!(!lz4f_is_error(run.close_ret), "control: readClose");
}

// ===========================================================================
// ROW MAP — every ERRORS.md row 222..250 and the test that covers it, or the
// precise reason it cannot be covered.
// ===========================================================================
//
// 222 LZ4F_readOpen  fp == NULL
//        -> row_222_223_read_open_null_arguments  (parameter_null, 21;
//           also asserts *lz4fRead is left untouched)
// 223 LZ4F_readOpen  lz4fRead == NULL
//        -> row_222_223_read_open_null_arguments  (21)
// 224 LZ4F_readOpen  calloc(1, sizeof(LZ4_readFile_t)) returns NULL
//        NOT TESTABLE — lz4file.c:83 calls the libc `calloc` DIRECTLY. This
//        translation unit has no `LZ4F_CustomMem` / allocator hook of any kind
//        (contrast `LZ4F_createCompressionContext_advanced`), and neither
//        `LZ4F_readOpen` nor `lz4file.h` exposes one, so the failure cannot be
//        induced from outside the library.
// 225 LZ4F_readOpen  LZ4F_createDecompressionContext fails
//        NOT TESTABLE — lz4file.c:88 hardcodes `LZ4F_VERSION`, so the only
//        possible failure is the `LZ4F_calloc(sizeof(LZ4F_dctx), cmem)` at
//        lz4frame.c:1286-1287, and the dctx is created with the DEFAULT cmem
//        (plain `calloc`) because `LZ4F_readOpen` uses the non-`_advanced`
//        entry point. No allocator hook is reachable => not inducible.
// 226 LZ4F_readOpen  fread(buf,1,19,fp) != 19
//        -> row_226_read_open_short_or_failing_fread_io_read  (io_read, 23;
//           short files 0..18 incl. a VALID 11-byte frame, stream at EOF, and
//           a write-only FILE*)
// 227 LZ4F_readOpen  LZ4F_getFrameInfo fails
//        -> row_227_read_open_frame_info_errors_verbatim  (frameType_unknown
//           13, reservedFlag_set 8, headerVersion_wrong 6,
//           maxBlockSize_invalid 2, headerChecksum_invalid 17)
// 228 LZ4F_readOpen  info.blockSizeID hits the switch `default:`
//        NOT TESTABLE — UNREACHABLE in the C. The arm is at lz4file.c:122-124.
//        `LZ4F_getFrameInfo` can only ever report blockSizeID in {0,4,5,6,7}:
//        `LZ4F_decodeHeader` rejects `blockSizeID < 4` at lz4frame.c:1410 and
//        the BD field is only 3 bits wide (lz4frame.c:1408, `& _3BITS`, max 7);
//        the sole way to get 0 is a skippable frame, whose zeroed
//        `dctx->frameInfo` (MEM_INIT at lz4frame.c:1355) yields `LZ4F_default`
//        and therefore takes the FIRST arm (lz4file.c:109-112). ERRORS.md row
//        228 itself flags the arm as "Defensive/unreachable in practice".
// 229 LZ4F_readOpen  malloc(srcBufMaxSize) returns NULL
//        NOT TESTABLE — lz4file.c:128 calls the libc `malloc` DIRECTLY with no
//        allocator hook; the size is capped at 4 MB by the switch above it, so
//        it cannot be driven to fail from outside.
// 230 LZ4F_read  lz4fRead == NULL
//        -> row_230_231_read_null_state_and_null_buffer  (21, sizes 0 ..
//           usize::MAX)
// 231 LZ4F_read  buf == NULL
//        -> row_230_231_read_null_state_and_null_buffer  (21, incl. size 0)
// 232 LZ4F_read  the `else { RETURN_ERROR(io_read) }` arm
//        NOT TESTABLE — DEAD CODE at lz4file.c:161-163. `ret` is declared
//        `size_t` at lz4file.c:151, so `if (ret > 0)` (lz4file.c:155) and
//        `else if (ret == 0)` (lz4file.c:159) between them cover every possible
//        value and the trailing `else` can never be entered; a failed `fread`
//        is indistinguishable from EOF and takes the `break` at lz4file.c:160.
//        The OBSERVABLE consequence (a silent short read) is asserted by
//        row_232_read_short_read_at_eof_is_not_an_error.
// 233 LZ4F_read  inner LZ4F_decompress fails
//        -> row_233_read_decompress_errors_verbatim  (maxBlockSize_invalid 2,
//           blockChecksum_invalid 7, frameSize_wrong 14, decompressionFailed
//           16, contentChecksum_invalid 18; the row's `allocation_failed` 9
//           variant is the un-hookable `LZ4F_malloc` inside lz4frame.c, same
//           reason as rows 224/225)
// 234 LZ4F_readClose  lz4fRead == NULL
//        -> row_234_read_close_null  (21); also asserted after every failed
//           readOpen in rows 226/227
// 235 LZ4F_writeOpen  fp == NULL
//        -> row_235_236_write_open_null_arguments  (21, *lz4fWrite untouched)
// 236 LZ4F_writeOpen  lz4fWrite == NULL
//        -> row_235_236_write_open_null_arguments  (21)
// 237 LZ4F_writeOpen  calloc(1, sizeof(LZ4_writeFile_t)) returns NULL
//        NOT TESTABLE — lz4file.c:225 calls the libc `calloc` DIRECTLY, no
//        allocator hook anywhere in lz4file.c / lz4file.h.
// 238 LZ4F_writeOpen  blockSizeID outside {0,4,5,6,7}
//        -> row_238_write_open_invalid_block_size_id  (maxBlockSize_invalid 2)
// 239 LZ4F_writeOpen  malloc(LZ4F_compressBound(..)) for dstBuf returns NULL
//        NOT TESTABLE — lz4file.c:253 calls the libc `malloc` DIRECTLY with no
//        hook; the request is bounded by
//        `LZ4F_compressBound(<=4 MB, prefs)` (lz4file.c:252) so it cannot be
//        inflated into a guaranteed failure either.
// 240 LZ4F_writeOpen  LZ4F_createCompressionContext fails
//        NOT TESTABLE — lz4file.c:259 hardcodes `LZ4F_VERSION`; the only
//        failure is the `LZ4F_calloc(sizeof(LZ4F_cctx), customMem)` at
//        lz4frame.c:598-600 reached with the DEFAULT cmem, i.e. plain
//        `calloc`, because the non-`_advanced` entry point is used.
// 241 LZ4F_writeOpen  LZ4F_compressBegin fails
//        NOT TESTABLE — the capacity passed is exactly `LZ4F_HEADER_SIZE_MAX`
//        (lz4file.c:265) so the `dstCapacity < maxFHSize` check at
//        lz4frame.c:700 cannot fire, and `dictBuffer` is NULL so the
//        `parameter_invalid` check at lz4frame.c:766-768 is unreachable. The
//        only remaining failures are the two `LZ4F_malloc` allocation checks at
//        lz4frame.c:714-722 and lz4frame.c:749-750, which use the cctx's
//        default (plain `malloc`) allocator — no hook, same reason as row 240.
// 242 LZ4F_writeOpen  header fwrite short
//        -> row_242_write_open_header_io_write  (io_write 22; read-only FILE*
//           and freopen-to-read-only)
// 243 LZ4F_write  lz4fWrite == NULL
//        -> row_243_244_write_null_state_and_null_buffer  (21, sizes 0 ..
//           usize::MAX)
// 244 LZ4F_write  buf == NULL
//        -> row_243_244_write_null_state_and_null_buffer  (21, incl. size 0)
// 245 LZ4F_write  inner LZ4F_compressUpdate fails
//        NOT TESTABLE — UNREACHABLE through this entry point.
//        `LZ4F_compressUpdateImpl` has exactly two failure returns:
//        `cStage != 1` (lz4frame.c:1005) and
//        `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)`
//        (lz4frame.c:1006-1007).
//          * `cStage` is set to 1 by the `LZ4F_compressBegin` that
//            `LZ4F_writeOpen` performs (lz4frame.c:811) and is only cleared by
//            `LZ4F_compressEnd` (lz4frame.c:1233), which `LZ4F_writeClose`
//            calls immediately before `free()`ing the handle — so any cctx
//            reachable by `LZ4F_write` always has `cStage == 1`.
//          * `LZ4F_write` chunks at `lz4fWrite->maxWriteSize` (lz4file.c:291-294)
//            and `dstBuf` is sized `LZ4F_compressBound(maxWriteSize, prefsPtr)`
//            (lz4file.c:252). `LZ4F_compressBound` already passes
//            `alreadyBuffered = (size_t)-1` when `autoFlush == 0`
//            (lz4frame.c:867-873), which `LZ4F_compressBound_internal` clamps to
//            `blockSize - 1` (lz4frame.c:390-391) — exactly the worst-case
//            `tmpInSize`. The buffer is therefore never too small.
//        (The `LZ4B_UNCOMPRESSED` check at lz4frame.c:1009-1010 belongs to
//        `LZ4F_uncompressedUpdate`, which `lz4file.c` never calls.)
// 246 LZ4F_write  payload fwrite short
//        -> row_246_249_write_payload_io_write_and_close_masks_it  (io_write 22)
// 247 LZ4F_writeClose  lz4fWrite == NULL
//        -> row_247_write_close_null  (21, incl. the well-defined
//           close-then-close-NULL pattern)
// 248 LZ4F_writeClose  LZ4F_compressEnd fails
//        -> row_248_write_close_compress_end_frame_size_wrong
//           (frameSize_wrong 14)
// 249 LZ4F_writeClose  errCode already latched => finalize skipped
//        -> row_246_249_write_payload_io_write_and_close_masks_it  (asserts the
//           return is exactly LZ4F_OK_NoError == 0 and the file is a truncated
//           frame; row_250_* is the complement that returns 22 on the SAME
//           broken stream when nothing was latched)
// 250 LZ4F_writeClose  footer fwrite short
//        -> row_250_write_close_footer_io_write  (io_write 22)
//
// Additional note (not an ERRORS.md row): calling `LZ4F_read` /
// `LZ4F_write` on a handle that was already passed to `LZ4F_readClose` /
// `LZ4F_writeClose` is a use-after-free — the handle is `free()`d at
// lz4file.c:63 and lz4file.c:207 — so it is deliberately not exercised; see
// boundary_after_close_nulled_handle_and_extreme_sizes for the well-defined
// (NULLed-handle) form.
// ===========================================================================
