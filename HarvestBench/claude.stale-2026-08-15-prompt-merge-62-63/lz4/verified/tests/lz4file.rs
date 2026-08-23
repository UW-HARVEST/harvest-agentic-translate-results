//! CONFIGS.md rows 144-150 + ERRORS.md rows 204-230 — `lz4file.c` (`FILE*` API).
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols (`LZ4F_readOpen`, `LZ4F_read`, `LZ4F_readClose`, `LZ4F_writeOpen`,
//! `LZ4F_write`, `LZ4F_writeClose`) and compares every return value plus the
//! resulting file bytes / decoded bytes.
//!
//! Both shared objects are linked against the SAME libc, so a `FILE*` obtained
//! from `fopen` is usable by either one. Even so, each library always gets its
//! OWN temp file and its OWN `FILE*`; the two files' bytes are then compared.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn fseek(f: *mut c_void, off: i64, whence: c_int) -> c_int;
}
const SEEK_END: c_int = 2;

const MODE_WB: &[u8] = b"wb\0";
const MODE_RB: &[u8] = b"rb\0";

fn mode(m: &[u8]) -> *const c_char {
    m.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// lz4file.h signatures
// ---------------------------------------------------------------------------
/// `LZ4F_errorCode_t LZ4F_readOpen(LZ4_readFile_t** lz4fRead, FILE* fp)`
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
/// `size_t LZ4F_read(LZ4_readFile_t* lz4fRead, void* buf, size_t size)`
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
/// `LZ4F_errorCode_t LZ4F_readClose(LZ4_readFile_t* lz4fRead)`
type FnReadClose = unsafe extern "C" fn(*mut c_void) -> usize;
/// `LZ4F_errorCode_t LZ4F_writeOpen(LZ4_writeFile_t**, FILE*, const LZ4F_preferences_t*)`
type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
/// `size_t LZ4F_write(LZ4_writeFile_t* lz4fWrite, const void* buf, size_t size)`
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
/// `LZ4F_errorCode_t LZ4F_writeClose(LZ4_writeFile_t* lz4fWrite)`
type FnWriteClose = unsafe extern "C" fn(*mut c_void) -> usize;

// ---------------------------------------------------------------------------
// `LZ4F_preferences_t` / `LZ4F_frameInfo_t` mirrors
// (field-by-field from c_src/include/lz4frame.h:175-198; every enum is a plain
//  C `int`, `contentSize` is `unsigned long long`, `dictID` is `unsigned`)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LZ4F_frameInfo_t {
    blockSizeID: c_int,          // LZ4F_blockSizeID_t
    blockMode: c_int,            // LZ4F_blockMode_t
    contentChecksumFlag: c_int,  // LZ4F_contentChecksum_t
    frameType: c_int,            // LZ4F_frameType_t
    contentSize: u64,            // unsigned long long
    dictID: c_uint,              // unsigned
    blockChecksumFlag: c_int,    // LZ4F_blockChecksum_t
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LZ4F_preferences_t {
    frameInfo: LZ4F_frameInfo_t,
    compressionLevel: c_int,
    autoFlush: c_uint,
    favorDecSpeed: c_uint,
    reserved: [c_uint; 3],
}

const _: () = assert!(core::mem::size_of::<LZ4F_frameInfo_t>() == 32);
const _: () = assert!(core::mem::size_of::<LZ4F_preferences_t>() == 56);
const _: () = assert!(core::mem::align_of::<LZ4F_preferences_t>() == 8);

fn prefs_of(bsid: c_int, bmode: c_int, cc: c_int, bc: c_int, lvl: c_int) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = bmode;
    p.frameInfo.contentChecksumFlag = cc;
    p.frameInfo.blockChecksumFlag = bc;
    p.compressionLevel = lvl;
    p
}

// ---------------------------------------------------------------------------
// Temp files
// ---------------------------------------------------------------------------
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A uniquely named temp file that is removed when the value is dropped.
struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "lz4file_diff_{}_{}_{}.lz4",
            std::process::id(),
            tag,
            n
        ));
        let _ = std::fs::remove_file(&p);
        Tmp(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn cpath(&self) -> CString {
        CString::new(self.0.as_os_str().as_bytes()).expect("path has no NUL")
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Data generation
// ---------------------------------------------------------------------------
/// `gen_data`, but ALWAYS backed by a real allocation with >= 64 spare bytes,
/// so even a zero-length source has a valid (non-dangling) address. See the
/// identical helper in `tests/hc.rs` for the full rationale.
fn gen_src(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_data(shape, len, rng);
    if v.capacity() < len + 64 {
        v.reserve(len + 64);
    }
    v
}

// ---------------------------------------------------------------------------
// Write driver
// ---------------------------------------------------------------------------
#[derive(Debug)]
struct WriteOut {
    open: usize,
    writes: Vec<usize>,
    close: usize,
    bytes: Vec<u8>,
}

/// `writeOpen` + a sequence of `write`s (chunk sizes taken cyclically from
/// `chunks`) + `writeClose`, all through ONE library, into its own temp file.
fn run_write(
    wo: &FnWriteOpen,
    w: &FnWrite,
    wc: &FnWriteClose,
    prefs: Option<&LZ4F_preferences_t>,
    data: &[u8],
    chunks: &[usize],
    tag: &str,
) -> WriteOut {
    assert!(!chunks.is_empty());
    let tmp = Tmp::new(tag);
    let cp = tmp.cpath();
    let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_WB)) };
    assert!(!fp.is_null(), "fopen(wb) failed for {:?}", tmp.path());

    let pp: *const LZ4F_preferences_t = match prefs {
        Some(p) => p as *const LZ4F_preferences_t,
        None => std::ptr::null(),
    };
    let mut h: *mut c_void = std::ptr::null_mut();
    let open = unsafe { wo(&mut h, fp, pp) };

    let mut writes = Vec::new();
    let mut close = 0usize;
    if !is_lz4f_error(open) {
        let mut off = 0usize;
        let mut ci = 0usize;
        while off < data.len() {
            let n = chunks[ci % chunks.len()].max(1).min(data.len() - off);
            ci += 1;
            let r = unsafe { w(h, data.as_ptr().add(off) as *const c_void, n) };
            writes.push(r);
            if is_lz4f_error(r) {
                break;
            }
            off += n;
        }
        close = unsafe { wc(h) };
    }
    unsafe {
        fclose(fp);
    }
    let bytes = std::fs::read(tmp.path()).unwrap_or_default();
    WriteOut {
        open,
        writes,
        close,
        bytes,
    }
}

fn cmp_write(c: &WriteOut, r: &WriteOut, ctx: &str) {
    assert_ret_eq(c.open, r.open, &format!("{ctx}: LZ4F_writeOpen"));
    assert_eq!(
        c.writes.len(),
        r.writes.len(),
        "{ctx}: number of LZ4F_write calls differs (C={} Rust={})",
        c.writes.len(),
        r.writes.len()
    );
    for (i, (a, b)) in c.writes.iter().zip(r.writes.iter()).enumerate() {
        assert_ret_eq(*a, *b, &format!("{ctx}: LZ4F_write #{i}"));
    }
    assert_ret_eq(c.close, r.close, &format!("{ctx}: LZ4F_writeClose"));
    assert_bytes_eq(&c.bytes, &r.bytes, &format!("{ctx}: output file bytes"));
}

/// Write `data` through both libraries and return the (identical) file bytes.
#[allow(clippy::too_many_arguments)]
fn write_both(
    wo: &(
        libloading::Symbol<'static, FnWriteOpen>,
        libloading::Symbol<'static, FnWriteOpen>,
    ),
    w: &(
        libloading::Symbol<'static, FnWrite>,
        libloading::Symbol<'static, FnWrite>,
    ),
    wc: &(
        libloading::Symbol<'static, FnWriteClose>,
        libloading::Symbol<'static, FnWriteClose>,
    ),
    prefs: Option<&LZ4F_preferences_t>,
    data: &[u8],
    chunks: &[usize],
    ctx: &str,
) -> Vec<u8> {
    let c = run_write(&wo.0, &w.0, &wc.0, prefs, data, chunks, "c");
    let r = run_write(&wo.1, &w.1, &wc.1, prefs, data, chunks, "r");
    cmp_write(&c, &r, ctx);
    c.bytes
}

// ---------------------------------------------------------------------------
// Read driver
// ---------------------------------------------------------------------------
#[derive(Debug)]
struct ReadOut {
    open: usize,
    reads: Vec<usize>,
    data: Vec<u8>,
    close: usize,
}

/// `readOpen` + repeated `read`s (chunk sizes cycled from `chunks`) until EOF
/// or an error + `readClose`, all through ONE library.
fn run_read(
    ro: &FnReadOpen,
    rd: &FnRead,
    rc: &FnReadClose,
    file: &[u8],
    chunks: &[usize],
    tag: &str,
) -> ReadOut {
    assert!(!chunks.is_empty());
    let tmp = Tmp::new(tag);
    std::fs::write(tmp.path(), file).expect("write temp input");
    let cp = tmp.cpath();
    let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
    assert!(!fp.is_null(), "fopen(rb) failed for {:?}", tmp.path());

    let mut h: *mut c_void = std::ptr::null_mut();
    let open = unsafe { ro(&mut h, fp) };
    let mut reads = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut close = 0usize;
    if !is_lz4f_error(open) {
        let cap = *chunks.iter().max().unwrap();
        let mut buf = vec![0u8; cap.max(1)];
        let mut ci = 0usize;
        loop {
            let n = chunks[ci % chunks.len()];
            ci += 1;
            let r = unsafe { rd(h, buf.as_mut_ptr() as *mut c_void, n) };
            reads.push(r);
            if is_lz4f_error(r) || r == 0 {
                break;
            }
            assert!(r <= n, "LZ4F_read returned {r} > requested {n}");
            out.extend_from_slice(&buf[..r]);
            assert!(ci < 8_000_000, "LZ4F_read loop runaway");
        }
        close = unsafe { rc(h) };
    }
    unsafe {
        fclose(fp);
    }
    ReadOut {
        open,
        reads,
        data: out,
        close,
    }
}

fn cmp_read(c: &ReadOut, r: &ReadOut, ctx: &str) {
    assert_ret_eq(c.open, r.open, &format!("{ctx}: LZ4F_readOpen"));
    assert_eq!(
        c.reads.len(),
        r.reads.len(),
        "{ctx}: number of LZ4F_read calls differs (C={} Rust={})",
        c.reads.len(),
        r.reads.len()
    );
    for (i, (a, b)) in c.reads.iter().zip(r.reads.iter()).enumerate() {
        assert_ret_eq(*a, *b, &format!("{ctx}: LZ4F_read #{i}"));
    }
    assert_bytes_eq(&c.data, &r.data, &format!("{ctx}: decoded bytes"));
    assert_ret_eq(c.close, r.close, &format!("{ctx}: LZ4F_readClose"));
}

/// Read `file` through both libraries; returns the (identical) decoded bytes.
/// A frame file shorter than `LZ4F_HEADER_SIZE_MAX` (19) can never be opened:
/// `LZ4F_readOpen` insists on `fread`ing exactly 19 bytes (ERRORS.md row 206).
/// Otherwise the decoded content must equal the original payload.
fn expect_round_trip(out: &ReadOut, file: &[u8], data: &[u8], ctx: &str) {
    if file.len() < LZ4F_HEADER_SIZE_MAX {
        assert_eq!(
            out.open,
            lz4f_err(23),
            "{ctx}: a {}-byte file must fail readOpen with io_read",
            file.len()
        );
        return;
    }
    assert!(
        !is_lz4f_error(out.open),
        "{ctx}: readOpen failed with {:#x}",
        out.open
    );
    assert_bytes_eq(&out.data, data, &format!("{ctx}: round trip"));
    assert_ret_eq(out.close, 0usize, &format!("{ctx}: readClose"));
}

fn read_both(
    ro: &(
        libloading::Symbol<'static, FnReadOpen>,
        libloading::Symbol<'static, FnReadOpen>,
    ),
    rd: &(
        libloading::Symbol<'static, FnRead>,
        libloading::Symbol<'static, FnRead>,
    ),
    rc: &(
        libloading::Symbol<'static, FnReadClose>,
        libloading::Symbol<'static, FnReadClose>,
    ),
    file: &[u8],
    chunks: &[usize],
    ctx: &str,
) -> ReadOut {
    let c = run_read(&ro.0, &rd.0, &rc.0, file, chunks, "c");
    let r = run_read(&ro.1, &rd.1, &rc.1, file, chunks, "r");
    cmp_read(&c, &r, ctx);
    c
}

// ===========================================================================
// CONFIGS.md row 144 — writeOpen(prefs = NULL) + write sweep + writeClose
// ===========================================================================
#[test]
fn row144_write_prefs_null_chunk_sweep() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    let mut rng = Rng::new(0x144_0001);

    // (write chunk size, total sizes to try with it)
    let cases: &[(usize, &[usize])] = &[
        (1, &[0, 1, 13, 1000, 65536, 70000]),
        (2, &[1, 7, 65535, 65536, 70000]),
        (7, &[7, 100, 65537, 131_072]),
        (64, &[64, 65536, 200_000]),
        (1000, &[999, 65536, 200_000]),
        (65536, &[65536, 131_072, 200_000]),
        (1 << 20, &[100, 200_000, 300_000]),
    ];

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &(chunk, totals) in cases {
            for &total in totals {
                let data = gen_src(shape, total, &mut rng);
                let ctx = format!("row144 shape={shape:?} chunk={chunk} total={total}");
                write_both(&wo, &w, &wc, None, &data, &[chunk], &ctx);
            }
        }
    }
}

// ===========================================================================
// CONFIGS.md row 145 — full preference matrix
//   blockSizeID {0,4,5,6,7} x blockMode {0,1} x contentChecksum {0,1}
//   x blockChecksum {0,1} x compressionLevel {1,9,12}
// ===========================================================================
#[test]
fn row145_write_config_matrix() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    let mut rng = Rng::new(0x145_0001);

    // Chunk pattern deliberately leaves a partial block buffered between calls.
    let chunks: &[usize] = &[7000, 65536, 100];

    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &lvl in &[1, 9, 12] {
                        // The level-12 optimal parser is expensive in a debug
                        // build, so shrink the payload for it.
                        let total = match lvl {
                            1 => 400_000usize,
                            9 => 250_000,
                            _ => 90_000,
                        };
                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let data = gen_src(shape, total, &mut rng);
                        let p = prefs_of(bsid, bmode, cc, bc, lvl);
                        let ctx = format!(
                            "row145 bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl} \
                             shape={shape:?} total={total}"
                        );
                        let f = write_both(&wo, &w, &wc, Some(&p), &data, chunks, &ctx);
                        assert!(f.len() >= 11, "{ctx}: implausibly short frame file");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS.md row 146 — RANDOM write chunk sizes, totals crossing block bounds
// ===========================================================================
#[test]
fn row146_write_random_chunks() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    let mut rng = Rng::new(0x146_0001);

    for iter in 0..90usize {
        let bsid = *[0, 4, 5, 6, 7].get(iter % 5).unwrap();
        let bmode = (iter as c_int) & 1;
        let cc = ((iter as c_int) >> 1) & 1;
        let bc = ((iter as c_int) >> 2) & 1;
        let lvl = *[0, 1, 3, 9].get(iter % 4).unwrap();
        let p = prefs_of(bsid, bmode, cc, bc, lvl);

        // Totals deliberately straddle the 64 KB / 256 KB block boundaries.
        let total = *[
            65535usize, 65536, 65537, 131_071, 131_072, 131_073, 262_143, 262_144, 262_145,
            300_000,
        ]
        .get(iter % 10)
        .unwrap();
        let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
        let data = gen_src(shape, total, &mut rng);

        // Random chunk sizes: a mix of tiny, mid and > block-size requests.
        let nch = rng.range(3, 12);
        let chunks: Vec<usize> = (0..nch)
            .map(|_| match rng.below(4) {
                0 => rng.range(1, 16),
                1 => rng.range(1, 4096),
                2 => rng.range(1, 100_000),
                _ => rng.range(1, 1 << 20),
            })
            .collect();

        let ctx = format!(
            "row146 iter={iter} bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl} \
             total={total} shape={shape:?} chunks={chunks:?}"
        );
        write_both(&wo, &w, &wc, Some(&p), &data, &chunks, &ctx);
    }
}

// ===========================================================================
// CONFIGS.md row 147 — readOpen + read sweep + readClose over row-145 files
// ===========================================================================
#[test]
fn row147_read_chunk_sweep() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);
    let mut rng = Rng::new(0x147_0001);

    // (read chunk size, payload size used to build the file)
    let read_cases: &[(usize, usize)] = &[
        (1, 20_000),
        (2, 20_000),
        (7, 40_000),
        (64, 200_000),
        (1000, 200_000),
        (1 << 20, 200_000),
    ];

    for &bsid in &[0, 4, 5] {
        for &bmode in &[0, 1] {
            for &(cc, bc) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                let lvl = 1; // cheap: the read path is what is under test here
                let p = prefs_of(bsid, bmode, cc, bc, lvl);
                for &(chunk, total) in read_cases {
                    let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                    let data = gen_src(shape, total, &mut rng);
                    let ctx = format!(
                        "row147 bsid={bsid} bmode={bmode} cc={cc} bc={bc} \
                         chunk={chunk} total={total} shape={shape:?}"
                    );
                    let file =
                        write_both(&wo, &w, &wc, Some(&p), &data, &[65536], &format!("{ctx} write"));
                    let out = read_both(&ro, &rd, &rc, &file, &[chunk], &ctx);
                    expect_round_trip(&out, &file, &data, &ctx);
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS.md row 148 — RANDOM read chunk sizes
// ===========================================================================
#[test]
fn row148_read_random_chunks() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);
    let mut rng = Rng::new(0x148_0001);

    for iter in 0..60usize {
        let bsid = *[0, 4, 5, 6, 7].get(iter % 5).unwrap();
        let bmode = (iter as c_int) & 1;
        let cc = ((iter as c_int) >> 1) & 1;
        let bc = ((iter as c_int) >> 2) & 1;
        let p = prefs_of(bsid, bmode, cc, bc, 1);
        let total = *[1usize, 100, 65535, 65536, 65537, 131_072, 200_000]
            .get(iter % 7)
            .unwrap();
        let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
        let data = gen_src(shape, total, &mut rng);

        let nch = rng.range(2, 10);
        let chunks: Vec<usize> = (0..nch)
            .map(|_| match rng.below(3) {
                0 => rng.range(1, 40),
                1 => rng.range(1, 9000),
                _ => rng.range(1, 300_000),
            })
            .collect();

        let ctx = format!(
            "row148 iter={iter} bsid={bsid} bmode={bmode} cc={cc} bc={bc} \
             total={total} shape={shape:?} chunks={chunks:?}"
        );
        let file = write_both(&wo, &w, &wc, Some(&p), &data, &[100_000], &format!("{ctx} write"));
        let out = read_both(&ro, &rd, &rc, &file, &chunks, &ctx);
        expect_round_trip(&out, &file, &data, &ctx);
    }
}

// ===========================================================================
// CONFIGS.md row 149 — CROSS round trip (C-written file read by Rust and back)
// ===========================================================================
#[test]
fn row149_cross_round_trip() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);
    let mut rng = Rng::new(0x149_0001);

    for iter in 0..40usize {
        let bsid = *[0, 4, 5, 6, 7].get(iter % 5).unwrap();
        let bmode = (iter as c_int) & 1;
        let cc = ((iter as c_int) >> 1) & 1;
        let bc = ((iter as c_int) >> 2) & 1;
        let lvl = *[0, 1, 9].get(iter % 3).unwrap();
        let p = prefs_of(bsid, bmode, cc, bc, lvl);
        let total = *[0usize, 1, 5000, 65536, 131_073, 200_000].get(iter % 6).unwrap();
        let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
        let data = gen_src(shape, total, &mut rng);
        let ctx = format!(
            "row149 iter={iter} bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl} total={total}"
        );

        // Two independent files, one written by each library.
        let cw = run_write(&wo.0, &w.0, &wc.0, Some(&p), &data, &[9000], "c");
        let rw = run_write(&wo.1, &w.1, &wc.1, Some(&p), &data, &[9000], "r");
        cmp_write(&cw, &rw, &format!("{ctx}: write"));

        let chunks: &[usize] = &[4096];
        // C-written file -> Rust reader, and Rust-written file -> C reader.
        let r_of_c = run_read(&ro.1, &rd.1, &rc.1, &cw.bytes, chunks, "r");
        let c_of_r = run_read(&ro.0, &rd.0, &rc.0, &rw.bytes, chunks, "c");
        // Plus the same-library baselines.
        let c_of_c = run_read(&ro.0, &rd.0, &rc.0, &cw.bytes, chunks, "c");
        let r_of_r = run_read(&ro.1, &rd.1, &rc.1, &rw.bytes, chunks, "r");

        cmp_read(&c_of_c, &r_of_c, &format!("{ctx}: C file, C vs Rust reader"));
        cmp_read(&c_of_r, &r_of_r, &format!("{ctx}: Rust file, C vs Rust reader"));
        expect_round_trip(&r_of_c, &cw.bytes, &data, &format!("{ctx}: Rust reads C file"));
        expect_round_trip(&c_of_r, &rw.bytes, &data, &format!("{ctx}: C reads Rust file"));
    }
}

// ===========================================================================
// CONFIGS.md row 150 — multiple concatenated frames / trailing garbage
// ===========================================================================
#[test]
fn row150_multiframe_and_trailing_garbage() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);
    let mut rng = Rng::new(0x150_0001);

    // ---- multiple concatenated frames -----------------------------------
    for &nframes in &[2usize, 3, 5] {
        for &(cc, bc) in &[(0, 0), (1, 1)] {
            let mut file: Vec<u8> = Vec::new();
            let mut plain: Vec<u8> = Vec::new();
            for k in 0..nframes {
                let bsid = *[0, 4, 5, 6, 7].get(k % 5).unwrap();
                let p = prefs_of(bsid, (k as c_int) & 1, cc, bc, 1);
                let total = *[1000usize, 65537, 40, 200_000, 7].get(k % 5).unwrap();
                let data = gen_src(ALL_SHAPES[k % ALL_SHAPES.len()], total, &mut rng);
                let ctx = format!("row150 multiframe n={nframes} k={k}");
                let f = write_both(&wo, &w, &wc, Some(&p), &data, &[12345], &ctx);
                file.extend_from_slice(&f);
                plain.extend_from_slice(&data);
            }
            for &chunk in &[7usize, 1000, 1 << 20] {
                let ctx = format!("row150 multiframe n={nframes} cc={cc} bc={bc} chunk={chunk}");
                let out = read_both(&ro, &rd, &rc, &file, &[chunk], &ctx);
                assert!(!is_lz4f_error(out.open), "{ctx}: readOpen");
                assert_bytes_eq(
                    &out.data,
                    &plain[..],
                    &format!("{ctx}: concatenated-frame content"),
                );
            }
        }
    }

    // ---- trailing garbage after a complete frame -------------------------
    let p = prefs_of(4, 0, 0, 0, 1);
    let data = gen_src(Shape::Texty, 50_000, &mut rng);
    let frame = write_both(&wo, &w, &wc, Some(&p), &data, &[7777], "row150 garbage base");

    for &ngarbage in &[1usize, 3, 4, 5, 6, 7, 19, 64, 1000] {
        let mut file = frame.clone();
        // Deliberately NOT a valid (or skippable) magic number.
        let mut g = vec![0u8; ngarbage];
        rng.fill(&mut g);
        g[0] = 0xAB;
        file.extend_from_slice(&g);
        for &chunk in &[64usize, 4096, 1 << 20] {
            let ctx = format!("row150 trailing garbage n={ngarbage} chunk={chunk}");
            let out = read_both(&ro, &rd, &rc, &file, &[chunk], &ctx);
            // Whatever the C decides (silent EOF or frameType_unknown), the
            // Rust must decide the same; cmp_read already asserted that.
            assert!(
                out.data.len() <= data.len(),
                "{ctx}: decoded more than the payload"
            );
            assert_bytes_eq(
                &out.data,
                &data[..out.data.len()],
                &format!("{ctx}: decoded prefix"),
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 204-205 — LZ4F_readOpen NULL parameters
// ===========================================================================
#[test]
fn err204_205_read_open_null_params() {
    sym!(ro, "LZ4F_readOpen", FnReadOpen);

    // row 204: fp == NULL
    let mut h: *mut c_void = std::ptr::null_mut();
    let (c, r) = unsafe {
        (
            ro.0(&mut h, std::ptr::null_mut()),
            ro.1(&mut h, std::ptr::null_mut()),
        )
    };
    assert_ret_eq(c, r, "row204 readOpen(fp=NULL)");
    assert_eq!(c, lz4f_err(21), "row204 expects parameter_null");

    // row 205: out-pointer == NULL (with a real FILE*)
    let tmp = Tmp::new("err205");
    std::fs::write(tmp.path(), b"whatever, never read").unwrap();
    let cp = tmp.cpath();
    let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
    assert!(!fp.is_null());
    let (c, r) = unsafe { (ro.0(std::ptr::null_mut(), fp), ro.1(std::ptr::null_mut(), fp)) };
    unsafe { fclose(fp) };
    assert_ret_eq(c, r, "row205 readOpen(out=NULL)");
    assert_eq!(c, lz4f_err(21), "row205 expects parameter_null");

    // both NULL
    let (c, r) = unsafe {
        (
            ro.0(std::ptr::null_mut(), std::ptr::null_mut()),
            ro.1(std::ptr::null_mut(), std::ptr::null_mut()),
        )
    };
    assert_ret_eq(c, r, "row204/205 readOpen(NULL, NULL)");
    assert_eq!(c, lz4f_err(21));
}

// ===========================================================================
// ERRORS.md rows 206-207 — file shorter than LZ4F_HEADER_SIZE_MAX / at EOF
// ===========================================================================
#[test]
fn err206_207_read_open_short_file_and_eof() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rc, "LZ4F_readClose", FnReadClose);

    // A *valid* but very short frame: 7-byte header + 4-byte endMark = 11 < 19.
    let empty: Vec<u8> = Vec::with_capacity(64);
    let valid_short = write_both(&wo, &w, &wc, None, &empty, &[1], "err206 short frame");
    assert!(
        valid_short.len() < LZ4F_HEADER_SIZE_MAX,
        "expected a sub-19-byte valid frame, got {}",
        valid_short.len()
    );

    let mut cases: Vec<Vec<u8>> = vec![
        Vec::new(),                          // empty file
        vec![0x04],                          // 1 byte
        vec![0x04, 0x22, 0x4D, 0x18],        // magic only
        vec![0x04, 0x22, 0x4D, 0x18, 0x64],  // magic + FLG
        valid_short.clone(),                 // 11 bytes, VALID frame
    ];
    // every length 0..19
    for n in 0..LZ4F_HEADER_SIZE_MAX {
        let mut v = valid_short.clone();
        v.resize(n, 0);
        cases.push(v);
    }

    for (i, f) in cases.iter().enumerate() {
        assert!(f.len() < LZ4F_HEADER_SIZE_MAX);
        let ctx = format!("row206 case {i} len={}", f.len());
        for (which, (roF, rcF)) in [(&ro.0, &rc.0), (&ro.1, &rc.1)].into_iter().enumerate() {
            let tmp = Tmp::new("err206");
            std::fs::write(tmp.path(), f).unwrap();
            let cp = tmp.cpath();
            let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
            assert!(!fp.is_null());
            let mut h: *mut c_void = std::ptr::null_mut();
            let ret = unsafe { roF(&mut h, fp) };
            assert_eq!(
                ret,
                lz4f_err(23),
                "{ctx}: {} readOpen must report io_read",
                if which == 0 { "C" } else { "Rust" }
            );
            assert!(h.is_null(), "{ctx}: handle must be NULLed on failure");
            unsafe { fclose(fp) };
            let _ = rcF; // readClose is not applicable on a failed open
        }
    }

    // row 207: a long-enough file, but the stream is already positioned at EOF.
    let p = prefs_of(4, 0, 0, 0, 1);
    let mut rng = Rng::new(0x207);
    let data = gen_src(Shape::Random, 5000, &mut rng);
    let full = write_both(&wo, &w, &wc, Some(&p), &data, &[5000], "err207 base");
    assert!(full.len() > LZ4F_HEADER_SIZE_MAX);
    for (which, roF) in [&ro.0, &ro.1].into_iter().enumerate() {
        let tmp = Tmp::new("err207");
        std::fs::write(tmp.path(), &full).unwrap();
        let cp = tmp.cpath();
        let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
        assert!(!fp.is_null());
        assert_eq!(unsafe { fseek(fp, 0, SEEK_END) }, 0, "fseek to EOF");
        let mut h: *mut c_void = std::ptr::null_mut();
        let ret = unsafe { roF(&mut h, fp) };
        unsafe { fclose(fp) };
        assert_eq!(
            ret,
            lz4f_err(23),
            "row207 {} readOpen at EOF must report io_read",
            if which == 0 { "C" } else { "Rust" }
        );
    }
}

// ===========================================================================
// ERRORS.md rows 208-211 — >= 19 bytes with a broken frame header
// ===========================================================================
#[test]
fn err208_211_read_open_bad_header() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);

    let mut rng = Rng::new(0x208);
    let p = prefs_of(4, 0, 0, 0, 1);
    let data = gen_src(Shape::Texty, 4000, &mut rng);
    // prefsPtr with no contentSize/dictID => a 7-byte header (HC at index 6).
    let good = write_both(&wo, &w, &wc, Some(&p), &data, &[4000], "err208 base");
    assert!(good.len() > 64);
    assert_eq!(&good[..4], &[0x04, 0x22, 0x4D, 0x18], "LZ4 magic");

    let open_one = |roF: &FnReadOpen, f: &[u8]| -> usize {
        let tmp = Tmp::new("err208");
        std::fs::write(tmp.path(), f).unwrap();
        let cp = tmp.cpath();
        let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
        assert!(!fp.is_null());
        let mut h: *mut c_void = std::ptr::null_mut();
        let ret = unsafe { roF(&mut h, fp) };
        unsafe { fclose(fp) };
        if is_lz4f_error(ret) {
            assert!(h.is_null(), "handle must be NULLed on failure");
        }
        ret
    };

    let check = |ctx: &str, f: &[u8], want: usize| {
        let c = open_one(&ro.0, f);
        let r = open_one(&ro.1, f);
        assert_ret_eq(c, r, ctx);
        assert_eq!(c, want, "{ctx}: unexpected C error code");
    };

    // row 208: >= 19 bytes, bad magic.
    for (tag, m) in [
        ("zero", [0u8, 0, 0, 0]),
        ("off-by-one", [0x05, 0x22, 0x4D, 0x18]),
        ("legacy", [0x02, 0x21, 0x4C, 0x18]),
        ("random", [0xDE, 0xAD, 0xBE, 0xEF]),
    ] {
        let mut f = good.clone();
        f[..4].copy_from_slice(&m);
        check(&format!("row208 bad magic ({tag})"), &f, lz4f_err(13));
    }

    // row 209: FLG reserved bit (bit 1) set.
    {
        let mut f = good.clone();
        f[4] |= 0x02;
        check("row209 FLG reserved bit", &f, lz4f_err(8));
    }

    // row 211: FLG version field != 1 (checked BEFORE the header checksum).
    for v in [0u8, 2, 3] {
        let mut f = good.clone();
        f[4] = (f[4] & 0x3F) | (v << 6);
        check(&format!("row211 FLG version={v}"), &f, lz4f_err(6));
    }

    // row 210: intact FLG/BD, wrong header checksum byte (index hSize-1 == 6).
    for delta in [1u8, 0x80, 0xFF] {
        let mut f = good.clone();
        f[6] = f[6].wrapping_add(delta);
        check(
            &format!("row210 bad header checksum (+{delta})"),
            &f,
            lz4f_err(17),
        );
    }

    // Also exercise the BD byte checks that readOpen forwards from
    // LZ4F_getFrameInfo (reserved bit / reserved nibble / small blockSizeID).
    {
        let mut f = good.clone();
        f[5] |= 0x80; // BD bit 7 reserved
        check("row209 BD reserved bit7", &f, lz4f_err(8));
    }
    for bsid in [0u8, 1, 2, 3] {
        let mut f = good.clone();
        f[5] = (f[5] & 0x8F) | (bsid << 4);
        check(
            &format!("row209 BD blockSizeID={bsid}"),
            &f,
            lz4f_err(2),
        );
    }
    {
        let mut f = good.clone();
        f[5] |= 0x0F; // BD low nibble reserved
        check("row209 BD reserved nibble", &f, lz4f_err(8));
    }
}

// ===========================================================================
// ERRORS.md rows 212-216 — LZ4F_read error surface
// ===========================================================================
#[test]
fn err212_216_read_errors() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);

    let mut rng = Rng::new(0x212);

    // row 212: NULL handle.
    let mut scratch = vec![0u8; 64];
    let (c, r) = unsafe {
        (
            rd.0(std::ptr::null_mut(), scratch.as_mut_ptr() as *mut c_void, 10),
            rd.1(std::ptr::null_mut(), scratch.as_mut_ptr() as *mut c_void, 10),
        )
    };
    assert_ret_eq(c, r, "row212 read(NULL handle)");
    assert_eq!(c, lz4f_err(21));
    // ... even with size 0 (the NULL check happens first).
    let (c, r) = unsafe {
        (
            rd.0(std::ptr::null_mut(), scratch.as_mut_ptr() as *mut c_void, 0),
            rd.1(std::ptr::null_mut(), scratch.as_mut_ptr() as *mut c_void, 0),
        )
    };
    assert_ret_eq(c, r, "row212 read(NULL handle, size 0)");
    assert_eq!(c, lz4f_err(21));

    // Build a normal frame that both libraries can open.
    let p = prefs_of(4, 0, 1, 0, 1);
    let data = gen_src(Shape::Texty, 40_000, &mut rng);
    let file = write_both(&wo, &w, &wc, Some(&p), &data, &[40_000], "err212 base");

    // rows 213-215 need a live handle from each library.
    for (which, (roF, rdF, rcF)) in [(&ro.0, &rd.0, &rc.0), (&ro.1, &rd.1, &rc.1)]
        .into_iter()
        .enumerate()
    {
        let name = if which == 0 { "C" } else { "Rust" };
        let tmp = Tmp::new("err213");
        std::fs::write(tmp.path(), &file).unwrap();
        let cp = tmp.cpath();
        let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
        assert!(!fp.is_null());
        let mut h: *mut c_void = std::ptr::null_mut();
        let open = unsafe { roF(&mut h, fp) };
        assert_eq!(open, 0, "{name}: readOpen on a valid frame");
        assert!(!h.is_null());

        // row 213: NULL buf.
        let ret = unsafe { rdF(h, std::ptr::null_mut(), 16) };
        assert_eq!(ret, lz4f_err(21), "{name}: row213 read(buf=NULL)");
        let ret = unsafe { rdF(h, std::ptr::null_mut(), 0) };
        assert_eq!(ret, lz4f_err(21), "{name}: row213 read(buf=NULL, size 0)");

        // row 214: size == 0 with a valid handle+buf.
        let mut buf = vec![0u8; 1 << 16];
        let ret = unsafe { rdF(h, buf.as_mut_ptr() as *mut c_void, 0) };
        assert_eq!(ret, 0, "{name}: row214 read(size=0)");

        // Drain the whole frame ...
        let mut got: Vec<u8> = Vec::new();
        loop {
            let n = unsafe { rdF(h, buf.as_mut_ptr() as *mut c_void, buf.len()) };
            assert!(!is_lz4f_error(n), "{name}: unexpected read error {n:#x}");
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
        }
        assert_bytes_eq(&got, &data[..], &format!("{name}: drained content"));

        // row 215: further reads past EOF => 0, not an error, repeatedly.
        for k in 0..4 {
            let n = unsafe { rdF(h, buf.as_mut_ptr() as *mut c_void, buf.len()) };
            assert_eq!(n, 0, "{name}: row215 read #{k} past EOF");
        }
        let ret = unsafe { rcF(h) };
        assert_eq!(ret, 0, "{name}: readClose after EOF");
        unsafe { fclose(fp) };
    }

    // row 216: corrupt payload mid-file => the SAME forwarded error from both.
    let mut nerrors = 0usize;
    let mut seen: Vec<usize> = Vec::new();
    for &(cc, bc) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
        let p = prefs_of(4, 0, cc, bc, 1);
        let data = gen_src(Shape::Texty, 30_000, &mut rng);
        let base = write_both(
            &wo,
            &w,
            &wc,
            Some(&p),
            &data,
            &[30_000],
            &format!("err216 base cc={cc} bc={bc}"),
        );
        // Corrupt bytes spread over the compressed block payload (which starts
        // right after the 7-byte header + the 4-byte block-size field).
        let starts = [11usize, 12, 20, 100, 1000, base.len() / 2, base.len() - 8];
        for &off in starts.iter() {
            if off >= base.len() {
                continue;
            }
            for &x in &[0x01u8, 0x7F, 0xFF] {
                let mut f = base.clone();
                f[off] ^= x;
                let ctx = format!("row216 cc={cc} bc={bc} off={off} xor={x:#x}");
                let out = read_both(&ro, &rd, &rc, &f, &[4096], &ctx);
                let last = *out.reads.last().unwrap();
                if is_lz4f_error(last) {
                    nerrors += 1;
                    let code = (0usize).wrapping_sub(last);
                    if !seen.contains(&code) {
                        seen.push(code);
                    }
                }
                assert_ret_eq(out.close, 0usize, &format!("{ctx}: readClose"));
            }
        }
        // Also corrupt the 4-byte block-size field itself (=> maxBlockSize
        // rejection, error 2, or a decompression failure).
        for off in 7..11usize {
            let mut f = base.clone();
            f[off] ^= 0x40;
            let ctx = format!("row216 blockheader cc={cc} bc={bc} off={off}");
            let out = read_both(&ro, &rd, &rc, &f, &[4096], &ctx);
            let last = *out.reads.last().unwrap();
            if is_lz4f_error(last) {
                nerrors += 1;
                let code = (0usize).wrapping_sub(last);
                if !seen.contains(&code) {
                    seen.push(code);
                }
            }
        }
    }
    assert!(
        nerrors > 0,
        "row216: no corruption produced a forwarded error - the test is vacuous"
    );
    seen.sort_unstable();
    assert!(
        seen.iter().all(|&c| (2..=24).contains(&c)),
        "row216: unexpected error ordinals {seen:?}"
    );
}

// ===========================================================================
// ERRORS.md rows 217-218 — LZ4F_readClose
// ===========================================================================
#[test]
fn err217_218_read_close() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);
    sym!(ro, "LZ4F_readOpen", FnReadOpen);
    sym!(rd, "LZ4F_read", FnRead);
    sym!(rc, "LZ4F_readClose", FnReadClose);

    // row 217: NULL handle.
    let (c, r) = unsafe { (rc.0(std::ptr::null_mut()), rc.1(std::ptr::null_mut())) };
    assert_ret_eq(c, r, "row217 readClose(NULL)");
    assert_eq!(c, lz4f_err(21));

    // row 218: valid handle on a TRUNCATED frame still closes with 0.
    let mut rng = Rng::new(0x218);
    let p = prefs_of(4, 0, 1, 1, 1);
    let data = gen_src(Shape::Random, 120_000, &mut rng);
    let full = write_both(&wo, &w, &wc, Some(&p), &data, &[65536], "err218 base");
    for &keep_num in &[1usize, 2, 3, 4, 5, 7, 9] {
        let keep = (full.len() * keep_num / 10).max(LZ4F_HEADER_SIZE_MAX);
        let f = full[..keep.min(full.len())].to_vec();
        let ctx = format!("row218 truncated to {keep}/{}", full.len());
        let out = read_both(&ro, &rd, &rc, &f, &[8192], &ctx);
        assert!(!is_lz4f_error(out.open), "{ctx}: readOpen");
        assert_ret_eq(out.close, 0usize, &format!("{ctx}: readClose == 0"));
        // A truncated frame decodes a prefix of the payload without an error.
        assert!(out.data.len() <= data.len(), "{ctx}: too much output");
        assert_bytes_eq(
            &out.data,
            &data[..out.data.len()],
            &format!("{ctx}: partial content"),
        );
    }
}

// ===========================================================================
// ERRORS.md rows 219-223 — LZ4F_writeOpen error surface
// ===========================================================================
#[test]
fn err219_223_write_open() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);

    // row 219: fp == NULL
    let p = prefs_of(4, 0, 0, 0, 1);
    let mut h: *mut c_void = std::ptr::null_mut();
    let (c, r) = unsafe {
        (
            wo.0(&mut h, std::ptr::null_mut(), &p),
            wo.1(&mut h, std::ptr::null_mut(), &p),
        )
    };
    assert_ret_eq(c, r, "row219 writeOpen(fp=NULL)");
    assert_eq!(c, lz4f_err(21));
    let (c, r) = unsafe {
        (
            wo.0(&mut h, std::ptr::null_mut(), std::ptr::null()),
            wo.1(&mut h, std::ptr::null_mut(), std::ptr::null()),
        )
    };
    assert_ret_eq(c, r, "row219 writeOpen(fp=NULL, prefs=NULL)");
    assert_eq!(c, lz4f_err(21));

    // row 220: out-pointer == NULL
    let tmp = Tmp::new("err220");
    let cp = tmp.cpath();
    let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_WB)) };
    assert!(!fp.is_null());
    let (c, r) = unsafe {
        (
            wo.0(std::ptr::null_mut(), fp, &p),
            wo.1(std::ptr::null_mut(), fp, &p),
        )
    };
    assert_ret_eq(c, r, "row220 writeOpen(out=NULL)");
    assert_eq!(c, lz4f_err(21));
    unsafe { fclose(fp) };

    // row 221: blockSizeID outside {0,4,5,6,7}
    for bsid in [1i32, 2, 3, 8, 9, -1, 100, i32::MIN, i32::MAX] {
        let mut bp = LZ4F_preferences_t::default();
        bp.frameInfo.blockSizeID = bsid;
        for (which, (woF, _wcF)) in [(&wo.0, &wc.0), (&wo.1, &wc.1)].into_iter().enumerate() {
            let tmp = Tmp::new("err221");
            let cp = tmp.cpath();
            let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_WB)) };
            assert!(!fp.is_null());
            let mut h: *mut c_void = std::ptr::null_mut();
            let ret = unsafe { woF(&mut h, fp, &bp) };
            unsafe { fclose(fp) };
            assert_eq!(
                ret,
                lz4f_err(2),
                "row221 bsid={bsid} {}: expected maxBlockSize_invalid",
                if which == 0 { "C" } else { "Rust" }
            );
            assert!(h.is_null(), "row221 bsid={bsid}: handle must be NULLed");
            assert_eq!(
                std::fs::metadata(tmp.path()).map(|m| m.len()).unwrap_or(0),
                0,
                "row221 bsid={bsid}: nothing should have been written"
            );
        }
    }

    // row 222: prefsPtr == NULL succeeds (64 KB default block size).
    for (which, (woF, wcF)) in [(&wo.0, &wc.0), (&wo.1, &wc.1)].into_iter().enumerate() {
        let tmp = Tmp::new("err222");
        let cp = tmp.cpath();
        let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_WB)) };
        assert!(!fp.is_null());
        let mut h: *mut c_void = std::ptr::null_mut();
        let ret = unsafe { woF(&mut h, fp, std::ptr::null()) };
        assert_eq!(
            ret,
            0,
            "row222 {}: writeOpen(prefs=NULL)",
            if which == 0 { "C" } else { "Rust" }
        );
        assert!(!h.is_null());
        // NOTE: on the success path `LZ4F_writeClose` returns whatever
        // `LZ4F_compressEnd` produced (it never resets `ret` to 0), i.e. the
        // 4-byte endMark here. ERRORS.md row 230 says "0"; the C says 4, and
        // the C is authoritative.
        assert_eq!(unsafe { wcF(h) }, 4, "row222: writeClose == endMark size");
        unsafe { fclose(fp) };
    }

    // row 223: a READ-ONLY FILE* makes the header fwrite fail => io_write.
    let ro_file = Tmp::new("err223");
    std::fs::write(ro_file.path(), b"placeholder contents").unwrap();
    for &prefs_null in &[false, true] {
        for (which, woF) in [&wo.0, &wo.1].into_iter().enumerate() {
            let cp = ro_file.cpath();
            let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_RB)) };
            assert!(!fp.is_null(), "fopen(rb) failed");
            let mut h: *mut c_void = std::ptr::null_mut();
            let pp: *const LZ4F_preferences_t = if prefs_null {
                std::ptr::null()
            } else {
                &p
            };
            let ret = unsafe { woF(&mut h, fp, pp) };
            unsafe { fclose(fp) };
            assert_eq!(
                ret,
                lz4f_err(22),
                "row223 prefs_null={prefs_null} {}: expected io_write",
                if which == 0 { "C" } else { "Rust" }
            );
            assert!(h.is_null(), "row223: handle must be NULLed on failure");
        }
    }
}

// ===========================================================================
// ERRORS.md rows 224-227 — LZ4F_write error surface / return value
// ===========================================================================
#[test]
fn err224_227_write() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);

    let scratch = vec![0u8; 128];
    // row 224: NULL handle.
    for &size in &[0usize, 1, 100] {
        let (c, r) = unsafe {
            (
                w.0(
                    std::ptr::null_mut(),
                    scratch.as_ptr() as *const c_void,
                    size,
                ),
                w.1(
                    std::ptr::null_mut(),
                    scratch.as_ptr() as *const c_void,
                    size,
                ),
            )
        };
        assert_ret_eq(c, r, &format!("row224 write(NULL handle, {size})"));
        assert_eq!(c, lz4f_err(21));
    }

    let mut rng = Rng::new(0x224);
    let p = prefs_of(4, 0, 0, 0, 1);
    let data = gen_src(Shape::Texty, 200_000, &mut rng);

    for (which, (woF, wF, wcF)) in [(&wo.0, &w.0, &wc.0), (&wo.1, &w.1, &wc.1)]
        .into_iter()
        .enumerate()
    {
        let name = if which == 0 { "C" } else { "Rust" };
        let tmp = Tmp::new("err225");
        let cp = tmp.cpath();
        let fp = unsafe { fopen(cp.as_ptr(), mode(MODE_WB)) };
        assert!(!fp.is_null());
        let mut h: *mut c_void = std::ptr::null_mut();
        assert_eq!(unsafe { woF(&mut h, fp, &p) }, 0, "{name}: writeOpen");

        // row 225: NULL buf.
        for &size in &[0usize, 1, 100] {
            let ret = unsafe { wF(h, std::ptr::null(), size) };
            assert_eq!(ret, lz4f_err(21), "{name}: row225 write(buf=NULL, {size})");
        }
        // row 226: size == 0 with a valid buffer.
        let ret = unsafe { wF(h, data.as_ptr() as *const c_void, 0) };
        assert_eq!(ret, 0, "{name}: row226 write(size=0)");
        // row 227: success returns the UNCOMPRESSED byte count.
        let mut off = 0usize;
        for &n in &[1usize, 2, 7, 64, 1000, 65535, 65536, 65537, 1] {
            let n = n.min(data.len() - off);
            let ret = unsafe { wF(h, data.as_ptr().add(off) as *const c_void, n) };
            assert_eq!(ret, n, "{name}: row227 write({n}) must return {n}");
            off += n;
        }
        // Success => LZ4F_compressEnd's byte count (flushed tail + endMark).
        let cl = unsafe { wcF(h) };
        assert!(
            !is_lz4f_error(cl) && cl >= 4,
            "{name}: writeClose returned {cl:#x}"
        );
        unsafe { fclose(fp) };
    }
}

// ===========================================================================
// ERRORS.md rows 228-230 — LZ4F_writeClose
// ===========================================================================
#[test]
fn err228_230_write_close() {
    sym!(wo, "LZ4F_writeOpen", FnWriteOpen);
    sym!(w, "LZ4F_write", FnWrite);
    sym!(wc, "LZ4F_writeClose", FnWriteClose);

    // row 228: NULL handle.
    let (c, r) = unsafe { (wc.0(std::ptr::null_mut()), wc.1(std::ptr::null_mut())) };
    assert_ret_eq(c, r, "row228 writeClose(NULL)");
    assert_eq!(c, lz4f_err(21));

    let mut rng = Rng::new(0x228);
    let p = prefs_of(4, 0, 1, 1, 1);
    let data = gen_src(Shape::Random, 300_000, &mut rng);

    // row 230: a normal close writes the endMark (+ content CRC) and returns
    // the byte count LZ4F_compressEnd produced.  Feeding an exact multiple of
    // the 64 KB block size leaves nothing buffered, so the count is exactly
    // 4 (endMark) or 8 (endMark + content checksum).
    let exact = gen_src(Shape::Texty, 2 * 65536, &mut rng);
    for &(cc, bc) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
        let pp = prefs_of(4, 0, cc, bc, 1);
        let ctx = format!("row230 cc={cc} bc={bc}");
        let cwo = run_write(&wo.0, &w.0, &wc.0, Some(&pp), &exact, &[65536], "c");
        let rwo = run_write(&wo.1, &w.1, &wc.1, Some(&pp), &exact, &[65536], "r");
        cmp_write(&cwo, &rwo, &ctx);
        let tail = if cc != 0 { 8usize } else { 4 };
        assert_eq!(cwo.close, tail, "{ctx}: writeClose == endMark(+CRC) size");
        assert_eq!(
            &cwo.bytes[cwo.bytes.len() - tail..cwo.bytes.len() - tail + 4],
            &[0u8; 4],
            "{ctx}: endMark"
        );
    }

    // ... and with a partially buffered tail the count is larger, but still
    // identical between the two libraries.
    for &(cc, bc) in &[(0, 0), (1, 1)] {
        let pp = prefs_of(4, 0, cc, bc, 1);
        let ctx = format!("row230 buffered tail cc={cc} bc={bc}");
        let cwo = run_write(&wo.0, &w.0, &wc.0, Some(&pp), &data, &[65536], "c");
        let rwo = run_write(&wo.1, &w.1, &wc.1, Some(&pp), &data, &[65536], "r");
        cmp_write(&cwo, &rwo, &ctx);
        assert!(
            !is_lz4f_error(cwo.close) && cwo.close > 8,
            "{ctx}: writeClose returned {:#x}",
            cwo.close
        );
    }

    // row 229: after a FAILED LZ4F_write the latched errCode makes writeClose
    // skip LZ4F_compressEnd and report 0 (the error is masked and the file is
    // left without a footer).
    //
    // `/dev/full` accepts the buffered 7-byte header write (so writeOpen
    // succeeds) but fails the large flush inside LZ4F_write with ENOSPC.
    let dev_full = CString::new("/dev/full").unwrap();
    let mut latched: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (which, (woF, wF, wcF)) in [(&wo.0, &w.0, &wc.0), (&wo.1, &w.1, &wc.1)]
        .into_iter()
        .enumerate()
    {
        let name = if which == 0 { "C" } else { "Rust" };
        let fp = unsafe { fopen(dev_full.as_ptr(), mode(MODE_WB)) };
        assert!(!fp.is_null(), "fopen(/dev/full, wb) failed");
        let mut h: *mut c_void = std::ptr::null_mut();
        let open = unsafe { woF(&mut h, fp, &p) };
        assert_eq!(open, 0, "{name}: row229 writeOpen on /dev/full");

        // 64 KB blocks: this write completes whole blocks and therefore
        // flushes ~64 KB through fwrite, which fails on /dev/full.
        let n1 = unsafe { wF(h, data.as_ptr() as *const c_void, data.len()) };
        assert_eq!(
            n1,
            lz4f_err(22),
            "{name}: row229 LZ4F_write must report io_write on /dev/full"
        );
        // A further small write only *buffers* (no fwrite), so it succeeds even
        // though errCode is already latched - exactly what the C does.
        let n2 = unsafe { wF(h, data.as_ptr() as *const c_void, 1000) };
        // A further large write flushes again and therefore fails again.
        let n3 = unsafe { wF(h, data.as_ptr() as *const c_void, 200_000) };

        // The masked close.
        let n4 = unsafe { wcF(h) };
        assert_eq!(n4, 0, "{name}: row229 writeClose masks the latched error");
        unsafe {
            fflush(fp);
            fclose(fp)
        };
        latched.push((n1, n2, n3, n4));
    }
    assert_eq!(
        latched[0], latched[1],
        "row229: C {:x?} vs Rust {:x?}",
        latched[0], latched[1]
    );
}
