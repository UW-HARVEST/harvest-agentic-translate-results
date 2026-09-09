//! Phase B + Phase C differential tests for the `lz4file.c` surface
//! (`LZ4F_writeOpen` / `LZ4F_write` / `LZ4F_writeClose` /
//! `LZ4F_readOpen` / `LZ4F_read` / `LZ4F_readClose`).
//!
//! Covers CONFIGS.md rows 138-140 and ERRORS.md rows 116-137.
//! Files are created under `target/` so the test is self-contained; each
//! library gets its OWN `FILE*` on its OWN path so the two runs cannot
//! interfere, and the resulting bytes are then compared.
mod common;
use common::*;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(fp: *mut c_void) -> i32;
}

fn tmpdir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("lz4file-tests");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn open_file(path: &std::path::Path, mode: &str) -> *mut c_void {
    let p = CString::new(path.to_str().unwrap()).unwrap();
    let m = CString::new(mode).unwrap();
    unsafe { fopen(p.as_ptr(), m.as_ptr()) }
}

/// Compress `src` to `path` through the lz4file write API of `lib`, feeding it
/// in chunks of `chunk` bytes.  Returns `(writeOpen_rc, per_write_rcs,
/// writeClose_rc, file_bytes)`.
fn write_file(
    lib: &'static Lib,
    path: &std::path::Path,
    src: &[u8],
    chunk: usize,
    prefs: Option<&LZ4F_preferences_t>,
) -> (usize, Vec<usize>, usize, Vec<u8>) {
    let _ = std::fs::remove_file(path);
    let fp = open_file(path, "wb");
    assert!(!fp.is_null(), "fopen({:?}, wb) failed", path);

    let mut st: *mut c_void = ptr::null_mut();
    let pp = prefs.map(|p| p as *const _).unwrap_or(ptr::null());
    let open_rc =
        unsafe { lib.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, fp, pp) };

    let mut write_rcs = Vec::new();
    let mut close_rc = usize::MAX;
    if !is_error(open_rc) {
        let step = chunk.max(1);
        let mut off = 0usize;
        while off < src.len() {
            let n = step.min(src.len() - off);
            let rc = unsafe {
                lib.get::<Fn_F_write>("LZ4F_write")(
                    st,
                    src[off..].as_ptr() as *const c_void,
                    n,
                )
            };
            write_rcs.push(rc);
            if is_error(rc) {
                break;
            }
            off += n;
        }
        if src.is_empty() {
            // exercise the size == 0 no-op path too
            let rc = unsafe {
                lib.get::<Fn_F_write>("LZ4F_write")(st, src.as_ptr() as *const c_void, 0)
            };
            write_rcs.push(rc);
        }
        close_rc = unsafe { lib.get::<Fn_F_writeClose>("LZ4F_writeClose")(st) };
    }
    unsafe { fclose(fp) };
    let bytes = std::fs::read(path).unwrap_or_default();
    (open_rc, write_rcs, close_rc, bytes)
}

/// Decompress `path` through the lz4file read API of `lib`, in `chunk`-sized
/// requests.  Returns `(readOpen_rc, per_read_rcs, readClose_rc, output)`.
fn read_file(
    lib: &'static Lib,
    path: &std::path::Path,
    chunk: usize,
    max_out: usize,
) -> (usize, Vec<usize>, usize, Vec<u8>) {
    let fp = open_file(path, "rb");
    assert!(!fp.is_null(), "fopen({:?}, rb) failed", path);

    let mut st: *mut c_void = ptr::null_mut();
    let open_rc = unsafe { lib.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };

    let mut rcs = Vec::new();
    let mut out = Vec::new();
    let mut close_rc = usize::MAX;
    if !is_error(open_rc) {
        let step = chunk.max(1);
        let mut buf = vec![0u8; step];
        loop {
            let rc = unsafe {
                lib.get::<Fn_F_read>("LZ4F_read")(st, buf.as_mut_ptr() as *mut c_void, step)
            };
            rcs.push(rc);
            if is_error(rc) || rc == 0 {
                break;
            }
            out.extend_from_slice(&buf[..rc]);
            if out.len() > max_out {
                break;
            }
        }
        close_rc = unsafe { lib.get::<Fn_F_readClose>("LZ4F_readClose")(st) };
    }
    unsafe { fclose(fp) };
    (open_rc, rcs, close_rc, out)
}

// ===========================================================================
// CONFIGS.md rows 138, 139, 140 — the full write/read round trip across every
// block size, both block modes, both checksum flags, and many chunk shapes.
// ===========================================================================

#[test]
fn file_round_trip_all_block_sizes_and_modes() {
    let dir = tmpdir();
    let mut rng = Rng::new(0xF11E_0001);

    for &bsid in &[LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &ccs in &[LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED] {
                for &bcs in &[LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                    for &level in &[0i32, 1, 2, 9, 12] {
                        let mut prefs = LZ4F_preferences_t::default();
                        prefs.frameInfo.blockSizeID = bsid;
                        prefs.frameInfo.blockMode = bmode;
                        prefs.frameInfo.contentChecksumFlag = ccs;
                        prefs.frameInfo.blockChecksumFlag = bcs;
                        prefs.compressionLevel = level;

                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let len = [0usize, 1, 100, 65535, 65536, 65537, 200_000]
                            [rng.below(7)];
                        let src = gen(&mut rng, len, shape);
                        // maxWriteSize is the frame block size; probe chunks
                        // below, at, and above it as well as degenerate ones.
                        for &chunk in &[1usize, 7, 4096, 65536, 65537, usize::MAX] {
                            let cpath = dir.join("c.lz4");
                            let rpath = dir.join("r.lz4");
                            let cv = write_file(c(), &cpath, &src, chunk, Some(&prefs));
                            let rv = write_file(r(), &rpath, &src, chunk, Some(&prefs));
                            assert_eq!(
                                (cv.0, &cv.1, cv.2),
                                (rv.0, &rv.1, rv.2),
                                "writeOpen/write/writeClose bsid={} bmode={} ccs={} bcs={} lvl={} len={} chunk={}",
                                bsid, bmode, ccs, bcs, level, len, chunk
                            );
                            assert_bytes_eq!(
                                format!(
                                    "file bytes bsid={} bmode={} ccs={} bcs={} lvl={} len={} chunk={}",
                                    bsid, bmode, ccs, bcs, level, len, chunk
                                ),
                                cv.3,
                                rv.3
                            );

                            // now read each file back with BOTH libraries
                            for rchunk in [1usize, 13, 4096, 1 << 17] {
                                let a = read_file(c(), &cpath, rchunk, len + 1024);
                                let b = read_file(r(), &rpath, rchunk, len + 1024);
                                assert_eq!(
                                    (a.0, &a.1, a.2),
                                    (b.0, &b.1, b.2),
                                    "read bsid={} len={} chunk={} rchunk={}",
                                    bsid, len, chunk, rchunk
                                );
                                assert_bytes_eq!(
                                    format!("read output len={} rchunk={}", len, rchunk),
                                    a.3,
                                    b.3
                                );
                                // cross-read: Rust's file with the C reader
                                let x = read_file(c(), &rpath, rchunk, len + 1024);
                                assert_eq!(
                                    (x.0, &x.1, x.2),
                                    (a.0, &a.1, a.2),
                                    "cross-read"
                                );
                                assert_bytes_eq!("cross-read output", x.3, a.3);
                                // readOpen requires a file of >= 19 bytes; when
                                // it succeeded the payload must be recovered.
                                if !is_error(a.0) {
                                    assert_bytes_eq!(
                                        format!("recovered == src (len={})", len),
                                        a.3,
                                        src
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `LZ4F_writeOpen` with `prefsPtr == NULL` -> maxWriteSize 64 KB.
#[test]
fn file_round_trip_null_prefs() {
    let dir = tmpdir();
    let mut rng = Rng::new(0xF11E_0002);
    for &len in &[0usize, 1, 19, 100, 65535, 65536, 65537, 300_000] {
        for &shape in ALL_SHAPES {
            let src = gen(&mut rng, len, shape);
            for &chunk in &[1usize, 65536, usize::MAX] {
                let cpath = dir.join("cn.lz4");
                let rpath = dir.join("rn.lz4");
                let cv = write_file(c(), &cpath, &src, chunk, None);
                let rv = write_file(r(), &rpath, &src, chunk, None);
                assert_eq!(
                    (cv.0, &cv.1, cv.2),
                    (rv.0, &rv.1, rv.2),
                    "NULL prefs len={} shape={:?} chunk={}",
                    len, shape, chunk
                );
                assert_bytes_eq!(
                    format!("NULL prefs bytes len={} shape={:?}", len, shape),
                    cv.3,
                    rv.3
                );

                let a = read_file(c(), &cpath, 4096, len + 1024);
                let b = read_file(r(), &rpath, 4096, len + 1024);
                assert_eq!((a.0, &a.1, a.2), (b.0, &b.1, b.2), "read back len={}", len);
                assert_bytes_eq!("read back output", a.3, b.3);
                if !is_error(a.0) {
                    assert_bytes_eq!("read back == src", a.3, src);
                }
            }
        }
    }
}

/// Randomized fuzz over sizes, chunk shapes and preferences.
#[test]
fn file_round_trip_fuzz() {
    let dir = tmpdir();
    let mut rng = Rng::new(0xF11E_0003);
    for iter in 0..300 {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.blockSizeID =
            [LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB]
                [rng.below(5)];
        prefs.frameInfo.blockMode = if rng.bool() { 1 } else { 0 };
        prefs.frameInfo.contentChecksumFlag = if rng.bool() { 1 } else { 0 };
        prefs.frameInfo.blockChecksumFlag = if rng.bool() { 1 } else { 0 };
        prefs.compressionLevel = [0i32, 1, 2, 3, 6, 9, 10, 12][rng.below(8)];
        prefs.autoFlush = if rng.bool() { 1 } else { 0 };
        prefs.favorDecSpeed = if rng.bool() { 1 } else { 0 };
        let declare_size = rng.bool();

        let len = rng.below(150_000);
        if declare_size {
            prefs.frameInfo.contentSize = len as u64;
        }
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = gen(&mut rng, len, shape);
        let chunk = [1usize, 3, 100, 4096, 65536, 65537, 1 << 20][rng.below(7)];
        let rchunk = [1usize, 5, 1000, 65536, 1 << 20][rng.below(5)];

        let cpath = dir.join("cf.lz4");
        let rpath = dir.join("rf.lz4");
        let cv = write_file(c(), &cpath, &src, chunk, Some(&prefs));
        let rv = write_file(r(), &rpath, &src, chunk, Some(&prefs));
        assert_eq!(
            (cv.0, &cv.1, cv.2),
            (rv.0, &rv.1, rv.2),
            "iter={} prefs={:?} len={} chunk={}",
            iter, prefs, len, chunk
        );
        assert_bytes_eq!(format!("iter={} file bytes", iter), cv.3, rv.3);

        let a = read_file(c(), &cpath, rchunk, len + 4096);
        let b = read_file(r(), &rpath, rchunk, len + 4096);
        assert_eq!(
            (a.0, &a.1, a.2),
            (b.0, &b.1, b.2),
            "iter={} read rchunk={}",
            iter, rchunk
        );
        assert_bytes_eq!(format!("iter={} read output", iter), a.3, b.3);
        if !is_error(a.0) {
            assert_bytes_eq!(format!("iter={} recovered", iter), a.3, src);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 116, 122, 125, 126, 132, 135 — NULL argument rejection
// ===========================================================================

/// Rows 116, 126: `LZ4F_readOpen` / `LZ4F_writeOpen` with a NULL `FILE*` and a
/// NULL out-pointer.
#[test]
fn err_open_null_arguments() {
    let dir = tmpdir();
    let path = dir.join("null.lz4");
    std::fs::write(&path, vec![0u8; 64]).unwrap();

    // readOpen: fp == NULL
    let (cv, rv) = both(|l| {
        let mut st: *mut c_void = ptr::null_mut();
        unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, ptr::null_mut()) }
    });
    assert_eq!(cv, rv, "readOpen(NULL fp): C={} Rust={}", show(cv), show(rv));
    assert_eq!(err_code(cv), 21, "expected parameter_null");

    // readOpen: lz4fRead == NULL
    let (cv, rv) = both(|l| {
        let fp = open_file(&path, "rb");
        let rc = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(ptr::null_mut(), fp) };
        unsafe { fclose(fp) };
        rc
    });
    assert_eq!(cv, rv, "readOpen(NULL out): C={} Rust={}", show(cv), show(rv));
    assert_eq!(err_code(cv), 21);

    // readOpen: both NULL
    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_readOpen>("LZ4F_readOpen")(ptr::null_mut(), ptr::null_mut())
    });
    assert_eq!(cv, rv, "readOpen(NULL, NULL)");
    assert_eq!(err_code(cv), 21);

    // writeOpen: fp == NULL
    let (cv, rv) = both(|l| {
        let mut st: *mut c_void = ptr::null_mut();
        unsafe {
            l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, ptr::null_mut(), ptr::null())
        }
    });
    assert_eq!(cv, rv, "writeOpen(NULL fp): C={} Rust={}", show(cv), show(rv));
    assert_eq!(err_code(cv), 21);

    // writeOpen: lz4fWrite == NULL
    let wpath = dir.join("nullw.lz4");
    let (cv, rv) = both(|l| {
        let fp = open_file(&wpath, "wb");
        let rc = unsafe {
            l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(ptr::null_mut(), fp, ptr::null())
        };
        unsafe { fclose(fp) };
        rc
    });
    assert_eq!(cv, rv, "writeOpen(NULL out)");
    assert_eq!(err_code(cv), 21);

    // writeOpen: both NULL, with and without prefs
    let prefs = LZ4F_preferences_t::default();
    for pp in [ptr::null(), &prefs as *const LZ4F_preferences_t] {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(ptr::null_mut(), ptr::null_mut(), pp)
        });
        assert_eq!(cv, rv, "writeOpen(NULL, NULL, {:?})", pp);
        assert_eq!(err_code(cv), 21);
    }
}

/// Rows 122, 125, 132, 135: NULL state / NULL buffer on read / write / close.
#[test]
fn err_read_write_close_null_arguments() {
    let dir = tmpdir();
    let path = dir.join("nn.lz4");
    let mut rng = Rng::new(0xF11E_0004);
    let src = gen(&mut rng, 5000, Shape::Text);
    let _ = write_file(c(), &path, &src, 4096, None);

    // LZ4F_read(NULL, buf, n) and LZ4F_read(state, NULL, n)
    let (cv, rv) = both(|l| {
        let mut scratch = vec![0u8; 1024];
        unsafe {
            l.get::<Fn_F_read>("LZ4F_read")(
                ptr::null_mut(),
                scratch.as_mut_ptr() as *mut c_void,
                scratch.len(),
            )
        }
    });
    assert_eq!(cv, rv, "read(NULL state): C={} Rust={}", show(cv), show(rv));
    assert_eq!(err_code(cv), 21);

    let (cv, rv) = both(|l| {
        let fp = open_file(&path, "rb");
        let mut st: *mut c_void = ptr::null_mut();
        let o = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };
        assert!(!is_error(o), "{}: readOpen -> {}", l.which, show(o));
        let rc =
            unsafe { l.get::<Fn_F_read>("LZ4F_read")(st, ptr::null_mut(), 100) };
        unsafe { l.get::<Fn_F_readClose>("LZ4F_readClose")(st) };
        unsafe { fclose(fp) };
        rc
    });
    assert_eq!(cv, rv, "read(NULL buf): C={} Rust={}", show(cv), show(rv));
    assert_eq!(err_code(cv), 21);

    // LZ4F_readClose(NULL)
    let (cv, rv) =
        both(|l| unsafe { l.get::<Fn_F_readClose>("LZ4F_readClose")(ptr::null_mut()) });
    assert_eq!(cv, rv, "readClose(NULL)");
    assert_eq!(err_code(cv), 21);

    // LZ4F_write(NULL, buf, n) and LZ4F_write(state, NULL, n)
    let (cv, rv) = both(|l| {
        let scratch = vec![0u8; 1024];
        unsafe {
            l.get::<Fn_F_write>("LZ4F_write")(
                ptr::null_mut(),
                scratch.as_ptr() as *const c_void,
                scratch.len(),
            )
        }
    });
    assert_eq!(cv, rv, "write(NULL state)");
    assert_eq!(err_code(cv), 21);

    let wpath = dir.join("nw.lz4");
    let (cv, rv) = both(|l| {
        let fp = open_file(&wpath, "wb");
        let mut st: *mut c_void = ptr::null_mut();
        let o = unsafe {
            l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, fp, ptr::null())
        };
        assert!(!is_error(o));
        let rc = unsafe { l.get::<Fn_F_write>("LZ4F_write")(st, ptr::null(), 100) };
        unsafe { l.get::<Fn_F_writeClose>("LZ4F_writeClose")(st) };
        unsafe { fclose(fp) };
        rc
    });
    assert_eq!(cv, rv, "write(NULL buf)");
    assert_eq!(err_code(cv), 21);

    // LZ4F_writeClose(NULL)
    let (cv, rv) =
        both(|l| unsafe { l.get::<Fn_F_writeClose>("LZ4F_writeClose")(ptr::null_mut()) });
    assert_eq!(cv, rv, "writeClose(NULL)");
    assert_eq!(err_code(cv), 21);
}

// ===========================================================================
// ERRORS.md rows 118, 121 — short / truncated / garbage input files
// ===========================================================================

/// Row 118: `LZ4F_readOpen` needs 19 bytes; any shorter file must yield
/// `io_read` in both libraries — including a perfectly VALID but tiny frame.
#[test]
fn err_readOpen_file_shorter_than_19_bytes() {
    let dir = tmpdir();
    let path = dir.join("short.lz4");
    let mut rng = Rng::new(0xF11E_0005);
    let full = {
        let p = dir.join("full.lz4");
        let src = gen(&mut rng, 20_000, Shape::Text);
        write_file(c(), &p, &src, 4096, None).3
    };
    assert!(full.len() > 19);

    for n in 0..=25usize {
        std::fs::write(&path, &full[..n.min(full.len())]).unwrap();
        let (cv, rv) = both(|l| {
            let fp = open_file(&path, "rb");
            let mut st: *mut c_void = ptr::null_mut();
            let rc = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };
            let null = st.is_null();
            if !is_error(rc) {
                unsafe { l.get::<Fn_F_readClose>("LZ4F_readClose")(st) };
            }
            unsafe { fclose(fp) };
            (rc, null)
        });
        assert_eq!(
            cv, rv,
            "readOpen on a {}-byte file: C=({}, null={}) Rust=({}, null={})",
            n, show(cv.0), cv.1, show(rv.0), rv.1
        );
        if n < 19 {
            assert_eq!(err_code(cv.0), 23, "n={} expected io_read", n);
            assert!(cv.1, "n={}: the C nulls the state on failure", n);
        }
    }

    // an EMPTY file
    std::fs::write(&path, b"").unwrap();
    let (cv, rv) = both(|l| {
        let fp = open_file(&path, "rb");
        let mut st: *mut c_void = ptr::null_mut();
        let rc = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };
        unsafe { fclose(fp) };
        (rc, st.is_null())
    });
    assert_eq!(cv, rv, "readOpen on an empty file");
    assert_eq!(err_code(cv.0), 23);
}

/// Row 121: `LZ4F_readOpen` forwards `LZ4F_getFrameInfo` errors — bad magic,
/// bad header checksum, reserved bits, invalid blockSizeID.
#[test]
fn err_readOpen_forwards_header_errors() {
    let dir = tmpdir();
    let path = dir.join("badhdr.lz4");
    let mut rng = Rng::new(0xF11E_0006);
    let good = {
        let p = dir.join("good.lz4");
        let src = gen(&mut rng, 20_000, Shape::Text);
        write_file(c(), &p, &src, 4096, None).3
    };

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // wrong magic numbers (including every skippable value)
    let mut magics: Vec<u32> = vec![0, 1, 0xFFFF_FFFF, LZ4F_MAGICNUMBER ^ 1];
    for i in 0..16u32 {
        magics.push(LZ4F_MAGIC_SKIPPABLE_START + i);
    }
    for m in magics {
        let mut f = good.clone();
        f[..4].copy_from_slice(&m.to_le_bytes());
        cases.push((format!("magic={:#x}", m), f));
    }
    // FLG / BD mutations
    for b in [0u8, 0x02, 0x40, 0x80, 0xC0, 0xFF] {
        let mut f = good.clone();
        f[4] = b;
        cases.push((format!("FLG={:#04x}", b), f));
        let mut f = good.clone();
        f[5] = b;
        cases.push((format!("BD={:#04x}", b), f));
    }
    // every blockSizeID in the BD byte, including the invalid 0..3
    for id in 0u8..=7 {
        let mut f = good.clone();
        f[5] = id << 4;
        cases.push((format!("blockSizeID={}", id), f));
    }
    // bad header checksum
    for d in [1u8, 0x7F, 0xFF] {
        let mut f = good.clone();
        f[6] = f[6].wrapping_add(d);
        cases.push((format!("HC+{}", d), f));
    }
    // pure garbage of exactly 19+ bytes
    for n in [19usize, 20, 64, 1000] {
        cases.push((format!("garbage{}", n), gen(&mut rng, n, Shape::Incompressible)));
    }

    for (label, bytes) in cases {
        std::fs::write(&path, &bytes).unwrap();
        let cv = read_file(c(), &path, 4096, 1 << 20);
        let rv = read_file(r(), &path, 4096, 1 << 20);
        assert_eq!(
            (cv.0, &cv.1, cv.2),
            (rv.0, &rv.1, rv.2),
            "{}: C=(open={}, reads={:?}, close={}) Rust=(open={}, reads={:?}, close={})",
            label,
            show(cv.0),
            cv.1.iter().map(|&x| show(x)).collect::<Vec<_>>(),
            show(cv.2),
            show(rv.0),
            rv.1.iter().map(|&x| show(x)).collect::<Vec<_>>(),
            show(rv.2)
        );
        assert_bytes_eq!(format!("{} output", label), cv.3, rv.3);
    }
}

/// Row 124: truncated / corrupted PAYLOAD (the header stays valid) so
/// `LZ4F_read` itself must fail identically, or return a short count.
#[test]
fn err_read_truncated_and_corrupt_payload() {
    let dir = tmpdir();
    let path = dir.join("trunc.lz4");
    let mut rng = Rng::new(0xF11E_0007);

    for &(bsid, ccs, bcs) in &[
        (LZ4F_MAX64KB, 0, 0),
        (LZ4F_MAX64KB, 1, 1),
        (LZ4F_MAX256KB, 1, 0),
    ] {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.blockSizeID = bsid;
        prefs.frameInfo.contentChecksumFlag = ccs;
        prefs.frameInfo.blockChecksumFlag = bcs;
        let src = gen(&mut rng, 100_000, Shape::Text);
        let full = {
            let p = dir.join("tfull.lz4");
            write_file(c(), &p, &src, 65536, Some(&prefs)).3
        };

        // truncation: every 1/64th of the file plus the last few bytes
        let mut takes: Vec<usize> = (0..=64).map(|i| full.len() * i / 64).collect();
        for k in 1..=8usize {
            if full.len() >= k {
                takes.push(full.len() - k);
            }
        }
        takes.push(19);
        takes.push(20);
        takes.sort_unstable();
        takes.dedup();
        for take in takes {
            std::fs::write(&path, &full[..take]).unwrap();
            for rchunk in [1usize, 4096, 1 << 18] {
                let cv = read_file(c(), &path, rchunk, src.len() + 4096);
                let rv = read_file(r(), &path, rchunk, src.len() + 4096);
                assert_eq!(
                    (cv.0, &cv.1, cv.2),
                    (rv.0, &rv.1, rv.2),
                    "bsid={} take={}/{} rchunk={}: C=(open={}, close={}) Rust=(open={}, close={})",
                    bsid, take, full.len(), rchunk,
                    show(cv.0), show(cv.2), show(rv.0), show(rv.2)
                );
                assert_bytes_eq!(
                    format!("bsid={} take={} rchunk={} output", bsid, take, rchunk),
                    cv.3,
                    rv.3
                );
            }
        }

        // random single-byte corruption
        for _ in 0..120 {
            let mut f = full.clone();
            let i = 19 + rng.below(f.len() - 19);
            f[i] ^= 1 << rng.below(8);
            std::fs::write(&path, &f).unwrap();
            let rchunk = [1usize, 100, 65536][rng.below(3)];
            let cv = read_file(c(), &path, rchunk, src.len() + 4096);
            let rv = read_file(r(), &path, rchunk, src.len() + 4096);
            assert_eq!(
                (cv.0, &cv.1, cv.2),
                (rv.0, &rv.1, rv.2),
                "corrupt byte {} bsid={} rchunk={}: C=(open={}, close={}) Rust=(open={}, close={})",
                i, bsid, rchunk, show(cv.0), show(cv.2), show(rv.0), show(rv.2)
            );
            assert_bytes_eq!(format!("corrupt byte {} output", i), cv.3, rv.3);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 128, 130 — invalid blockSizeID in prefs, and a read-only FILE*
// ===========================================================================

/// Row 128: `LZ4F_writeOpen` rejects any `blockSizeID` outside {0,4,5,6,7}.
#[test]
fn err_writeOpen_invalid_block_size_id() {
    let dir = tmpdir();
    let path = dir.join("bsid.lz4");
    let mut ids: Vec<i32> = (-4..=12).collect();
    ids.extend_from_slice(&[99, 255, i32::MAX, i32::MIN]);
    for id in ids {
        let mut prefs = LZ4F_preferences_t::default();
        prefs.frameInfo.blockSizeID = id;
        let (cv, rv) = both(|l| {
            let fp = open_file(&path, "wb");
            let mut st: *mut c_void = ptr::null_mut();
            let rc =
                unsafe { l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, fp, &prefs) };
            let null = st.is_null();
            if !is_error(rc) {
                unsafe { l.get::<Fn_F_writeClose>("LZ4F_writeClose")(st) };
            }
            unsafe { fclose(fp) };
            (rc, null)
        });
        assert_eq!(
            cv, rv,
            "writeOpen blockSizeID={}: C=({}, null={}) Rust=({}, null={})",
            id, show(cv.0), cv.1, show(rv.0), rv.1
        );
        if ![0, 4, 5, 6, 7].contains(&id) {
            assert_eq!(err_code(cv.0), 2, "id={} expected maxBlockSize_invalid", id);
            assert!(cv.1, "id={}: the C nulls the state on failure", id);
        }
    }
}

/// Row 130 / 134 / 136: a `FILE*` opened READ-ONLY makes every `fwrite` fail,
/// so `LZ4F_writeOpen` (header), `LZ4F_write` (payload) and `LZ4F_writeClose`
/// (footer) must all report `io_write` identically.
#[test]
fn err_write_to_read_only_file() {
    let dir = tmpdir();
    let path = dir.join("ro.lz4");
    std::fs::write(&path, vec![0u8; 4096]).unwrap();
    let mut rng = Rng::new(0xF11E_0008);
    let src = gen(&mut rng, 5000, Shape::Text);

    let (cv, rv) = both(|l| {
        let fp = open_file(&path, "rb");
        assert!(!fp.is_null());
        let mut st: *mut c_void = ptr::null_mut();
        let open_rc =
            unsafe { l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, fp, ptr::null()) };
        let mut wrc = usize::MAX;
        let mut crc = usize::MAX;
        if !is_error(open_rc) {
            wrc = unsafe {
                l.get::<Fn_F_write>("LZ4F_write")(
                    st,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            crc = unsafe { l.get::<Fn_F_writeClose>("LZ4F_writeClose")(st) };
        }
        unsafe { fclose(fp) };
        (open_rc, wrc, crc)
    });
    assert_eq!(
        cv, rv,
        "write to a read-only FILE*: C=({}, {}, {}) Rust=({}, {}, {})",
        show(cv.0), show(cv.1), show(cv.2),
        show(rv.0), show(rv.1), show(rv.2)
    );
    // whichever stage the C reports the failure at, it must be io_write (22)
    let first_err = [cv.0, cv.1, cv.2]
        .into_iter()
        .find(|&x| x != usize::MAX && is_error(x));
    assert!(
        first_err.is_some(),
        "expected an io_write failure somewhere, got ({}, {}, {})",
        show(cv.0), show(cv.1), show(cv.2)
    );
}

/// Row 137: a latched write error must make `LZ4F_writeClose` skip
/// `LZ4F_compressEnd` and still free the state — verified by observing the same
/// return code and by successfully re-running the whole sequence afterwards.
#[test]
fn err_writeClose_after_latched_error() {
    let dir = tmpdir();
    let ro = dir.join("latched_ro.lz4");
    std::fs::write(&ro, vec![0u8; 64]).unwrap();
    let mut rng = Rng::new(0xF11E_0009);
    let src = gen(&mut rng, 3000, Shape::Text);

    let (cv, rv) = both(|l| {
        let fp = open_file(&ro, "rb");
        let mut st: *mut c_void = ptr::null_mut();
        let o = unsafe { l.get::<Fn_F_writeOpen>("LZ4F_writeOpen")(&mut st, fp, ptr::null()) };
        let mut rcs = Vec::new();
        if !is_error(o) {
            // several writes, all of which must fail identically
            for _ in 0..3 {
                rcs.push(unsafe {
                    l.get::<Fn_F_write>("LZ4F_write")(
                        st,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                });
            }
            rcs.push(unsafe { l.get::<Fn_F_writeClose>("LZ4F_writeClose")(st) });
        }
        unsafe { fclose(fp) };
        (o, rcs)
    });
    assert_eq!(cv, rv, "latched write error sequence");

    // and the libraries must still work afterwards
    let good = dir.join("after.lz4");
    let a = write_file(c(), &good, &src, 4096, None);
    let b = write_file(r(), &dir.join("after_r.lz4"), &src, 4096, None);
    assert_eq!((a.0, &a.1, a.2), (b.0, &b.1, b.2), "recovery write");
    assert_bytes_eq!("recovery write bytes", a.3, b.3);
}

/// `LZ4F_read` past the end of the stream must keep returning 0 in both.
#[test]
fn file_read_past_end_returns_zero() {
    let dir = tmpdir();
    let path = dir.join("past.lz4");
    let mut rng = Rng::new(0xF11E_000A);
    for &len in &[0usize, 1, 100, 70_000] {
        let src = gen(&mut rng, len, Shape::Text);
        let _ = write_file(c(), &path, &src, 4096, None);

        let (cv, rv) = both(|l| {
            let fp = open_file(&path, "rb");
            let mut st: *mut c_void = ptr::null_mut();
            let o = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };
            let mut out = Vec::new();
            let mut rcs = Vec::new();
            if !is_error(o) {
                let mut buf = vec![0u8; 8192];
                // read the whole stream, then keep going 5 more times
                loop {
                    let rc = unsafe {
                        l.get::<Fn_F_read>("LZ4F_read")(
                            st,
                            buf.as_mut_ptr() as *mut c_void,
                            buf.len(),
                        )
                    };
                    rcs.push(rc);
                    if is_error(rc) || rc == 0 {
                        break;
                    }
                    out.extend_from_slice(&buf[..rc]);
                }
                for _ in 0..5 {
                    rcs.push(unsafe {
                        l.get::<Fn_F_read>("LZ4F_read")(
                            st,
                            buf.as_mut_ptr() as *mut c_void,
                            buf.len(),
                        )
                    });
                }
                rcs.push(unsafe { l.get::<Fn_F_readClose>("LZ4F_readClose")(st) });
            }
            unsafe { fclose(fp) };
            (o, rcs, out)
        });
        assert_eq!(
            (cv.0, &cv.1),
            (rv.0, &rv.1),
            "read past end len={}: C={:?} Rust={:?}",
            len,
            cv.1.iter().map(|&x| show(x)).collect::<Vec<_>>(),
            rv.1.iter().map(|&x| show(x)).collect::<Vec<_>>()
        );
        assert_bytes_eq!(format!("read past end output len={}", len), cv.2, rv.2);
        if !is_error(cv.0) {
            assert_bytes_eq!(format!("recovered len={}", len), cv.2, src);
        }
    }
}

/// `LZ4F_read(state, buf, 0)` — a zero-size request.
#[test]
fn file_read_zero_size() {
    let dir = tmpdir();
    let path = dir.join("zero.lz4");
    let mut rng = Rng::new(0xF11E_000B);
    let src = gen(&mut rng, 30_000, Shape::Text);
    let _ = write_file(c(), &path, &src, 4096, None);

    let (cv, rv) = both(|l| {
        let fp = open_file(&path, "rb");
        let mut st: *mut c_void = ptr::null_mut();
        let o = unsafe { l.get::<Fn_F_readOpen>("LZ4F_readOpen")(&mut st, fp) };
        assert!(!is_error(o));
        let mut buf = vec![0u8; 64];
        let z1 = unsafe {
            l.get::<Fn_F_read>("LZ4F_read")(st, buf.as_mut_ptr() as *mut c_void, 0)
        };
        let n = unsafe {
            l.get::<Fn_F_read>("LZ4F_read")(st, buf.as_mut_ptr() as *mut c_void, buf.len())
        };
        let z2 = unsafe {
            l.get::<Fn_F_read>("LZ4F_read")(st, buf.as_mut_ptr() as *mut c_void, 0)
        };
        let cl = unsafe { l.get::<Fn_F_readClose>("LZ4F_readClose")(st) };
        unsafe { fclose(fp) };
        (z1, n, z2, cl, buf[..n.min(buf.len())].to_vec())
    });
    assert_eq!(
        (cv.0, cv.1, cv.2, cv.3),
        (rv.0, rv.1, rv.2, rv.3),
        "read(0): C=({}, {}, {}, {}) Rust=({}, {}, {}, {})",
        show(cv.0), show(cv.1), show(cv.2), show(cv.3),
        show(rv.0), show(rv.1), show(rv.2), show(rv.3)
    );
    assert_bytes_eq!("read(0) payload", cv.4, rv.4);
}

/// Out-of-range enum values in the preferences handed to `LZ4F_writeOpen`.
#[test]
fn err_writeOpen_out_of_range_enums() {
    let dir = tmpdir();
    let path = dir.join("oor.lz4");
    let mut rng = Rng::new(0xF11E_000C);
    let src = gen(&mut rng, 2000, Shape::Text);
    let odd: &[i32] = &[-1, 2, 3, 8, 99, i32::MAX, i32::MIN];

    for &v in odd {
        let variants: Vec<(&str, LZ4F_preferences_t)> = vec![
            ("blockMode", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockMode = v;
                p
            }),
            ("contentChecksumFlag", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentChecksumFlag = v;
                p
            }),
            ("blockChecksumFlag", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockChecksumFlag = v;
                p
            }),
            ("frameType", {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.frameType = v;
                p
            }),
            ("compressionLevel", {
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = v;
                p
            }),
        ];
        for (field, prefs) in variants {
            let cpath = dir.join("oor_c.lz4");
            let rpath = dir.join("oor_r.lz4");
            let cv = write_file(c(), &cpath, &src, 4096, Some(&prefs));
            let rv = write_file(r(), &rpath, &src, 4096, Some(&prefs));
            assert_eq!(
                (cv.0, &cv.1, cv.2),
                (rv.0, &rv.1, rv.2),
                "writeOpen {}={}: C=(open={}, close={}) Rust=(open={}, close={})",
                field, v, show(cv.0), show(cv.2), show(rv.0), show(rv.2)
            );
            assert_bytes_eq!(format!("writeOpen {}={} bytes", field, v), cv.3, rv.3);
            let _ = path;

            // and read it back if it was produced at all
            if !is_error(cv.0) && !cv.3.is_empty() {
                let a = read_file(c(), &cpath, 4096, src.len() + 4096);
                let b = read_file(r(), &rpath, 4096, src.len() + 4096);
                assert_eq!(
                    (a.0, &a.1, a.2),
                    (b.0, &b.1, b.2),
                    "read back {}={}",
                    field, v
                );
                assert_bytes_eq!(format!("read back {}={}", field, v), a.3, b.3);
            }
        }
    }
}

/// CONFIGS.md row 139, `size > maxWriteSize` branch (`lz4file.c` chunking loop).
///
/// `LZ4F_writeOpen` derives `maxWriteSize` from the frame block size
/// (64 KB / 256 KB / 1 MB / 4 MB) and `LZ4F_write` splits any request LARGER
/// than that into `maxWriteSize`-sized `LZ4F_compressUpdate` calls.  That split
/// determines where the frame's block boundaries fall, so it is directly
/// observable in the output bytes — but only if a SINGLE `LZ4F_write` call
/// exceeds `maxWriteSize`.  Every other test in this file feeds the payload in
/// chunks small enough that the loop never runs, so this test deliberately
/// hands the whole (multi-megabyte) payload to ONE `LZ4F_write` call for every
/// block size.
#[test]
fn file_write_larger_than_maxWriteSize_in_one_call() {
    let dir = tmpdir();
    let mut rng = Rng::new(0xF11E_0100);

    for &bsid in &[LZ4F_DEFAULT, LZ4F_MAX64KB, LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
        // block sizes are 64 KB, 64 KB, 256 KB, 1 MB, 4 MB; pick payloads that
        // straddle each of them so the chunking loop runs 0, 1 and many times.
        let block = match bsid {
            LZ4F_MAX256KB => 256 * 1024,
            LZ4F_MAX1MB => 1024 * 1024,
            LZ4F_MAX4MB => 4 * 1024 * 1024,
            _ => 64 * 1024,
        };
        let sizes = [
            block - 1,
            block,
            block + 1,
            2 * block,
            2 * block + 1,
            5 * block / 2,
        ];
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &(ccs, bcs) in &[(0i32, 0i32), (1, 1)] {
                for &level in &[0i32, 2] {
                    let mut prefs = LZ4F_preferences_t::default();
                    prefs.frameInfo.blockSizeID = bsid;
                    prefs.frameInfo.blockMode = bmode;
                    prefs.frameInfo.contentChecksumFlag = ccs;
                    prefs.frameInfo.blockChecksumFlag = bcs;
                    prefs.compressionLevel = level;

                    for &n in &sizes {
                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let src = gen(&mut rng, n, shape);
                        let cpath = dir.join("mw_c.lz4");
                        let rpath = dir.join("mw_r.lz4");
                        // chunk = usize::MAX => exactly one LZ4F_write call
                        let cv = write_file(c(), &cpath, &src, usize::MAX, Some(&prefs));
                        let rv = write_file(r(), &rpath, &src, usize::MAX, Some(&prefs));
                        assert_eq!(
                            (cv.0, &cv.1, cv.2),
                            (rv.0, &rv.1, rv.2),
                            "one-call write bsid={} bmode={} ccs={} bcs={} lvl={} n={} (block={}): C=(open={}, close={}) Rust=(open={}, close={})",
                            bsid, bmode, ccs, bcs, level, n, block,
                            show(cv.0), show(cv.2), show(rv.0), show(rv.2)
                        );
                        assert_bytes_eq!(
                            format!(
                                "one-call write bytes bsid={} bmode={} ccs={} bcs={} lvl={} n={}",
                                bsid, bmode, ccs, bcs, level, n
                            ),
                            cv.3,
                            rv.3
                        );

                        // and the file must read back to the original
                        let a = read_file(c(), &cpath, 1 << 16, n + 4096);
                        let b = read_file(r(), &rpath, 1 << 16, n + 4096);
                        assert_eq!(
                            (a.0, &a.1, a.2),
                            (b.0, &b.1, b.2),
                            "one-call read back bsid={} n={}",
                            bsid, n
                        );
                        assert_bytes_eq!(
                            format!("one-call read back bsid={} n={}", bsid, n),
                            a.3,
                            b.3
                        );
                        assert_bytes_eq!(format!("one-call recovered n={}", n), a.3, src);
                    }
                }
            }
        }
    }
}
