//! Phase B/C differential tests for the LZ4 file API (lz4file.c).

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut c_void) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
type FnRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut c_void) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> u32;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressFrameBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn rewind(f: *mut c_void);
    fn fwrite(buf: *const c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn fread(buf: *mut c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn ftell(f: *mut c_void) -> i64;
    fn fseek(f: *mut c_void, off: i64, whence: c_int) -> c_int;
}
const SEEK_END: c_int = 2;
const SEEK_SET: c_int = 0;

use std::sync::atomic::{AtomicU64, Ordering};
static SEQ: AtomicU64 = AtomicU64::new(0);

/// libc `tmpfile()` is unusable here (it insists on /tmp), so create a private
/// scratch file under the crate's target/ directory instead.
unsafe fn tmpfile() -> *mut c_void {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("target");
    dir.push("filetmp");
    let _ = std::fs::create_dir_all(&dir);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = dir.join(format!("lz4file-{}-{}.tmp", std::process::id(), n));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    let mode = std::ffi::CString::new("w+b").unwrap();
    let f = fopen(cpath.as_ptr(), mode.as_ptr());
    // unlink immediately: the descriptor stays valid, the file disappears on close
    let _ = std::fs::remove_file(&path);
    assert!(!f.is_null(), "could not create scratch file {:?}", path);
    f
}

struct Api {
    write_open: FnWriteOpen,
    write: FnWrite,
    write_close: FnWriteClose,
    read_open: FnReadOpen,
    read: FnRead,
    read_close: FnReadClose,
    is_error: FnIsError,
    get_error_name: FnGetErrorName,
    compress_frame: FnCompressFrame,
    compress_frame_bound: FnCompressFrameBound,
}

fn bind(l: &Lib) -> Api {
    Api {
        write_open: l.sym("LZ4F_writeOpen"),
        write: l.sym("LZ4F_write"),
        write_close: l.sym("LZ4F_writeClose"),
        read_open: l.sym("LZ4F_readOpen"),
        read: l.sym("LZ4F_read"),
        read_close: l.sym("LZ4F_readClose"),
        is_error: l.sym("LZ4F_isError"),
        get_error_name: l.sym("LZ4F_getErrorName"),
        compress_frame: l.sym("LZ4F_compressFrame"),
        compress_frame_bound: l.sym("LZ4F_compressFrameBound"),
    }
}

fn pair() -> (Api, Api) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

unsafe fn slurp(f: *mut c_void) -> Vec<u8> {
    fflush(f);
    fseek(f, 0, SEEK_END);
    let len = ftell(f) as usize;
    fseek(f, 0, SEEK_SET);
    let mut v = vec![0u8; len];
    if len > 0 {
        let got = fread(v.as_mut_ptr() as *mut c_void, 1, len, f);
        v.truncate(got);
    }
    v
}

unsafe fn file_with(bytes: &[u8]) -> *mut c_void {
    let f = tmpfile();
    assert!(!f.is_null());
    if !bytes.is_empty() {
        assert_eq!(fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), f), bytes.len());
    }
    fflush(f);
    rewind(f);
    f
}

// --- CONFIGS: writeOpen/write/writeClose over the preferences matrix --------
#[test]
fn file_write_matrix() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5001);
    let bsids = [0i32, 4, 5, 6, 7];
    for &bsid in &bsids {
        for &bmode in &[0i32, 1] {
            for &cchk in &[0i32, 1] {
                for &bchk in &[0i32, 1] {
                    for &lvl in &[0i32, 1, 9, 12] {
                        let mut p = LZ4F_preferences_t::default();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = bmode;
                        p.frameInfo.contentChecksumFlag = cchk;
                        p.frameInfo.blockChecksumFlag = bchk;
                        p.compressionLevel = lvl;
                        for &total in &[0usize, 1, 1000, 70_000, 300_000] {
                            let data = gen(Shape::Text, total, &mut rng);
                            // random write chunking, identical for both
                            let mut chunks = Vec::new();
                            let mut left = total;
                            while left > 0 {
                                let n = rng.range(1, left.min(90_000) + 1);
                                chunks.push(n);
                                left -= n;
                            }
                            unsafe {
                                let mut results = Vec::new();
                                let mut files = Vec::new();
                                for which in 0..2 {
                                    let api = if which == 0 { &c } else { &r };
                                    let f = tmpfile();
                                    assert!(!f.is_null());
                                    let mut st: *mut c_void = std::ptr::null_mut();
                                    let mut rcs = Vec::new();
                                    let rc0 = (api.write_open)(&mut st, f, &p);
                                    rcs.push(rc0);
                                    if (api.is_error)(rc0) == 0 {
                                        let mut off = 0usize;
                                        for &n in &chunks {
                                            let rc = (api.write)(
                                                st,
                                                data.as_ptr().add(off) as *const c_void,
                                                n,
                                            );
                                            rcs.push(rc);
                                            off += n;
                                        }
                                        rcs.push((api.write_close)(st));
                                    }
                                    files.push(slurp(f));
                                    fclose(f);
                                    results.push(rcs);
                                }
                                assert_eq!(
                                    results[0],
                                    results[1],
                                    "file write rcs bsid={} bmode={} cchk={} bchk={} lvl={} total={}: {:?} vs {:?}",
                                    bsid, bmode, cchk, bchk, lvl, total,
                                    results[0].iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>(),
                                    results[1].iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>()
                                );
                                assert_bytes_eq(
                                    &format!(
                                        "file bytes bsid={} bmode={} cchk={} bchk={} lvl={} total={}",
                                        bsid, bmode, cchk, bchk, lvl, total
                                    ),
                                    &files[0],
                                    &files[1],
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // NULL prefs
    for &total in &[0usize, 1000, 200_000] {
        let data = gen(Shape::Text, total, &mut rng);
        unsafe {
            let mut files = Vec::new();
            for which in 0..2 {
                let api = if which == 0 { &c } else { &r };
                let f = tmpfile();
                let mut st: *mut c_void = std::ptr::null_mut();
                let rc0 = (api.write_open)(&mut st, f, std::ptr::null());
                assert_eq!(rc0, 0);
                assert_eq!((api.write)(st, data.as_ptr() as *const c_void, total), total);
                // NOTE: LZ4F_writeClose() returns the number of footer bytes
                // written by LZ4F_compressEnd(), not 0 (lz4file.c:326-336).
                let rcc = (api.write_close)(st);
                assert!((api.is_error)(rcc) == 0, "writeClose: {}", fmt_lz4f(rcc));
                files.push((slurp(f), rcc));
                fclose(f);
            }
            assert_eq!(files[0].1, files[1].1, "writeClose rc (NULL prefs)");
            assert_bytes_eq("file bytes NULL prefs", &files[0].0, &files[1].0);
        }
    }
}

// --- CONFIGS: readOpen/read/readClose over frame variants + read sizes ------
#[test]
fn file_read_matrix() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5002);
    for &bsid in &[0i32, 4, 5, 6, 7] {
        for &bmode in &[0i32, 1] {
            for &cchk in &[0i32, 1] {
                for &bchk in &[0i32, 1] {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.blockMode = bmode;
                    p.frameInfo.contentChecksumFlag = cchk;
                    p.frameInfo.blockChecksumFlag = bchk;
                    for &total in &[0usize, 1, 1000, 70_000, 300_000] {
                        let data = gen(Shape::Text, total, &mut rng);
                        let cap = unsafe { (c.compress_frame_bound)(total, &p) };
                        let mut frame = vec![0u8; cap];
                        let flen = unsafe {
                            (c.compress_frame)(
                                frame.as_mut_ptr() as *mut c_void,
                                cap,
                                data.as_ptr() as *const c_void,
                                total,
                                &p,
                            )
                        };
                        assert!(unsafe { (c.is_error)(flen) } == 0);
                        let frame = &frame[..flen];
                        for &rsz in &[1usize, 7, 1000, 65536, 400_000] {
                            unsafe {
                                let mut outs = Vec::new();
                                let mut rcs = Vec::new();
                                for which in 0..2 {
                                    let api = if which == 0 { &c } else { &r };
                                    let f = file_with(frame);
                                    let mut st: *mut c_void = std::ptr::null_mut();
                                    let mut these = Vec::new();
                                    let mut out = Vec::new();
                                    let rc0 = (api.read_open)(&mut st, f);
                                    these.push(rc0);
                                    if (api.is_error)(rc0) == 0 {
                                        let mut buf = vec![0u8; rsz];
                                        loop {
                                            let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, rsz);
                                            these.push(got);
                                            if (api.is_error)(got) != 0 || got == 0 {
                                                break;
                                            }
                                            out.extend_from_slice(&buf[..got]);
                                        }
                                        these.push((api.read_close)(st));
                                    }
                                    fclose(f);
                                    outs.push(out);
                                    rcs.push(these);
                                }
                                assert_eq!(
                                    rcs[0], rcs[1],
                                    "file read rcs bsid={} bmode={} cchk={} bchk={} total={} rsz={}",
                                    bsid, bmode, cchk, bchk, total, rsz
                                );
                                assert_bytes_eq(
                                    &format!(
                                        "file read output bsid={} total={} rsz={}",
                                        bsid, total, rsz
                                    ),
                                    &outs[0],
                                    &outs[1],
                                );
                                // NOTE: LZ4F_readOpen() pre-reads exactly
                                // LZ4F_HEADER_SIZE_MAX bytes and fails with
                                // LZ4F_ERROR_io_read when the whole frame is
                                // shorter than that (lz4file.c:94-98). Only
                                // compare the payload when it succeeded.
                                if (c.is_error)(rcs[0][0]) == 0 {
                                    assert_bytes_eq("file read content", &outs[0], &data);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- CONFIGS: full write-then-read round trip through the file API -----------
#[test]
fn file_round_trip() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5003);
    for iter in 0..30 {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = [0i32, 4, 5, 6, 7][rng.below(5)];
        p.frameInfo.blockMode = rng.below(2) as c_int;
        p.frameInfo.contentChecksumFlag = rng.below(2) as c_int;
        p.frameInfo.blockChecksumFlag = rng.below(2) as c_int;
        p.compressionLevel = [0i32, 1, 3, 9, 12][rng.below(5)];
        p.autoFlush = rng.below(2) as u32;
        let total = rng.range(0, 400_000);
        let data = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], total, &mut rng);
        let wchunk = [1usize, 100, 70_000, 400_000][rng.below(4)];
        let rchunk = [1usize, 100, 70_000, 400_000][rng.below(4)];
        unsafe {
            let mut outs = Vec::new();
            let mut bytes = Vec::new();
            let mut closes = Vec::new();
            for which in 0..2 {
                let api = if which == 0 { &c } else { &r };
                let f = tmpfile();
                let mut st: *mut c_void = std::ptr::null_mut();
                assert_eq!((api.write_open)(&mut st, f, &p), 0, "iter={}", iter);
                let mut off = 0usize;
                while off < total {
                    let n = wchunk.min(total - off);
                    assert_eq!((api.write)(st, data.as_ptr().add(off) as *const c_void, n), n);
                    off += n;
                }
                let rcc = (api.write_close)(st);
                assert!((api.is_error)(rcc) == 0, "iter={} writeClose: {}", iter, fmt_lz4f(rcc));
                closes.push(rcc);
                bytes.push(slurp(f));
                rewind(f);
                let mut st: *mut c_void = std::ptr::null_mut();
                let rc0 = (api.read_open)(&mut st, f);
                let mut out = Vec::new();
                if (api.is_error)(rc0) == 0 {
                    let mut buf = vec![0u8; rchunk];
                    loop {
                        let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, rchunk);
                        if (api.is_error)(got) != 0 || got == 0 {
                            break;
                        }
                        out.extend_from_slice(&buf[..got]);
                    }
                    (api.read_close)(st);
                }
                fclose(f);
                outs.push((rc0, out));
            }
            assert_eq!(closes[0], closes[1], "iter={} writeClose rc", iter);
            assert_bytes_eq(&format!("iter={} file bytes", iter), &bytes[0], &bytes[1]);
            assert_eq!(outs[0].0, outs[1].0, "iter={} readOpen rc", iter);
            assert_bytes_eq(&format!("iter={} file round trip", iter), &outs[0].1, &outs[1].1);
            if unsafe { (c.is_error)(outs[0].0) } == 0 {
                assert_bytes_eq(&format!("iter={} content", iter), &outs[0].1, &data);
            }
        }
    }
}

// --- ERRORS rows for lz4file ------------------------------------------------
#[test]
fn file_error_paths() {
    let (c, r) = pair();
    unsafe {
        // writeOpen: NULL fp
        let mut cs: *mut c_void = std::ptr::null_mut();
        let mut rs: *mut c_void = std::ptr::null_mut();
        assert_eq!(
            (c.write_open)(&mut cs, std::ptr::null_mut(), std::ptr::null()),
            (r.write_open)(&mut rs, std::ptr::null_mut(), std::ptr::null()),
            "writeOpen(NULL fp) -> parameter_null"
        );
        // writeOpen: NULL state pointer
        let f = tmpfile();
        assert_eq!(
            (c.write_open)(std::ptr::null_mut(), f, std::ptr::null()),
            (r.write_open)(std::ptr::null_mut(), f, std::ptr::null()),
            "writeOpen(NULL statePtr) -> parameter_null"
        );
        // writeOpen: invalid blockSizeID (out-of-range enum across FFI)
        for bad in [1i32, 2, 3, 8, 9, -1, 100, i32::MIN, i32::MAX] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = bad;
            let mut cs: *mut c_void = std::ptr::null_mut();
            let mut rs: *mut c_void = std::ptr::null_mut();
            let a = (c.write_open)(&mut cs, f, &p);
            let b = (r.write_open)(&mut rs, f, &p);
            assert_eq!(
                a,
                b,
                "writeOpen(blockSizeID={}) -> {} vs {}",
                bad,
                fmt_lz4f(a),
                fmt_lz4f(b)
            );
            assert_eq!(cs.is_null(), rs.is_null(), "writeOpen(blockSizeID={}) state", bad);
            if (c.is_error)(a) == 0 {
                (c.write_close)(cs);
                (r.write_close)(rs);
            }
        }
        fclose(f);

        // write: NULL state / NULL buf
        let buf = [0u8; 16];
        assert_eq!(
            (c.write)(std::ptr::null_mut(), buf.as_ptr() as *const c_void, 16),
            (r.write)(std::ptr::null_mut(), buf.as_ptr() as *const c_void, 16),
            "write(NULL state) -> parameter_null"
        );
        // writeClose: NULL
        assert_eq!(
            (c.write_close)(std::ptr::null_mut()),
            (r.write_close)(std::ptr::null_mut()),
            "writeClose(NULL) -> parameter_null"
        );
        // a valid state with a NULL buffer
        {
            let f = tmpfile();
            let mut cs: *mut c_void = std::ptr::null_mut();
            let mut rs: *mut c_void = std::ptr::null_mut();
            assert_eq!((c.write_open)(&mut cs, f, std::ptr::null()), 0);
            assert_eq!((r.write_open)(&mut rs, f, std::ptr::null()), 0);
            assert_eq!(
                (c.write)(cs, std::ptr::null(), 10),
                (r.write)(rs, std::ptr::null(), 10),
                "write(state, NULL buf) -> parameter_null"
            );
            // size 0 is a no-op success
            assert_eq!(
                (c.write)(cs, buf.as_ptr() as *const c_void, 0),
                (r.write)(rs, buf.as_ptr() as *const c_void, 0),
                "write(state, buf, 0)"
            );
            assert_eq!((c.write_close)(cs), (r.write_close)(rs));
            fclose(f);
        }

        // readOpen: NULL fp / NULL state pointer
        let mut cs: *mut c_void = std::ptr::null_mut();
        let mut rs: *mut c_void = std::ptr::null_mut();
        assert_eq!(
            (c.read_open)(&mut cs, std::ptr::null_mut()),
            (r.read_open)(&mut rs, std::ptr::null_mut()),
            "readOpen(NULL fp) -> parameter_null"
        );
        let f = tmpfile();
        assert_eq!(
            (c.read_open)(std::ptr::null_mut(), f),
            (r.read_open)(std::ptr::null_mut(), f),
            "readOpen(NULL statePtr) -> parameter_null"
        );
        fclose(f);

        // readOpen: file shorter than LZ4F_HEADER_SIZE_MAX -> io_read
        for len in 0..LZ4F_HEADER_SIZE_MAX {
            let mut hdr = vec![0u8; len];
            if len >= 4 {
                hdr[..4].copy_from_slice(&0x184D2204u32.to_le_bytes());
            }
            let f = file_with(&hdr);
            let mut cs: *mut c_void = std::ptr::null_mut();
            let a = (c.read_open)(&mut cs, f);
            rewind(f);
            let mut rs: *mut c_void = std::ptr::null_mut();
            let b = (r.read_open)(&mut rs, f);
            assert_eq!(
                a,
                b,
                "readOpen(short file len={}) -> {} vs {}",
                len,
                fmt_lz4f(a),
                fmt_lz4f(b)
            );
            assert_eq!(cs.is_null(), rs.is_null(), "readOpen(short len={}) state", len);
            if (c.is_error)(a) == 0 {
                (c.read_close)(cs);
                (r.read_close)(rs);
            }
            fclose(f);
        }

        // readOpen: garbage magic -> frameType_unknown
        let mut rng = Rng::new(0x5004);
        for _ in 0..20 {
            let junk = gen(Shape::Random, 64, &mut rng);
            let f = file_with(&junk);
            let mut cs: *mut c_void = std::ptr::null_mut();
            let a = (c.read_open)(&mut cs, f);
            rewind(f);
            let mut rs: *mut c_void = std::ptr::null_mut();
            let b = (r.read_open)(&mut rs, f);
            assert_eq!(a, b, "readOpen(garbage) -> {} vs {}", fmt_lz4f(a), fmt_lz4f(b));
            assert_eq!(cs.is_null(), rs.is_null(), "readOpen(garbage) state");
            if (c.is_error)(a) == 0 {
                (c.read_close)(cs);
                (r.read_close)(rs);
            }
            fclose(f);
        }

        // read: NULL state / NULL buf ; readClose(NULL)
        let mut b2 = [0u8; 16];
        assert_eq!(
            (c.read)(std::ptr::null_mut(), b2.as_mut_ptr() as *mut c_void, 16),
            (r.read)(std::ptr::null_mut(), b2.as_mut_ptr() as *mut c_void, 16),
            "read(NULL state) -> parameter_null"
        );
        assert_eq!(
            (c.read_close)(std::ptr::null_mut()),
            (r.read_close)(std::ptr::null_mut()),
            "readClose(NULL) -> parameter_null"
        );

        // read: valid state, NULL buffer / size 0 ; and truncated frame body
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.contentChecksumFlag = 1;
        let data = gen(Shape::Text, 50_000, &mut rng);
        let cap = (c.compress_frame_bound)(data.len(), &p);
        let mut frame = vec![0u8; cap];
        let flen = (c.compress_frame)(
            frame.as_mut_ptr() as *mut c_void,
            cap,
            data.as_ptr() as *const c_void,
            data.len(),
            &p,
        );
        let frame = &frame[..flen];
        {
            let f = file_with(frame);
            let mut cs: *mut c_void = std::ptr::null_mut();
            assert_eq!((c.read_open)(&mut cs, f), 0);
            rewind(f);
            let mut rs: *mut c_void = std::ptr::null_mut();
            assert_eq!((r.read_open)(&mut rs, f), 0);
            assert_eq!(
                (c.read)(cs, std::ptr::null_mut(), 10),
                (r.read)(rs, std::ptr::null_mut(), 10),
                "read(state, NULL buf) -> parameter_null"
            );
            assert_eq!(
                (c.read)(cs, b2.as_mut_ptr() as *mut c_void, 0),
                (r.read)(rs, b2.as_mut_ptr() as *mut c_void, 0),
                "read(state, buf, 0)"
            );
            (c.read_close)(cs);
            (r.read_close)(rs);
            fclose(f);
        }
        // truncated frame: readOpen succeeds, read() eventually stops/errors
        for cut in [
            LZ4F_HEADER_SIZE_MAX + 1,
            LZ4F_HEADER_SIZE_MAX + 5,
            flen / 2,
            flen - 1,
            flen - 4,
        ] {
            if cut >= flen || cut < LZ4F_HEADER_SIZE_MAX {
                continue;
            }
            let mut outs = Vec::new();
            for which in 0..2 {
                let api = if which == 0 { &c } else { &r };
                let f = file_with(&frame[..cut]);
                let mut st: *mut c_void = std::ptr::null_mut();
                let mut rcs = Vec::new();
                let rc0 = (api.read_open)(&mut st, f);
                rcs.push(rc0);
                let mut n = 0usize;
                if (api.is_error)(rc0) == 0 {
                    let mut buf = vec![0u8; 8192];
                    loop {
                        let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, 8192);
                        rcs.push(got);
                        if (api.is_error)(got) != 0 || got == 0 {
                            break;
                        }
                        n += got;
                    }
                    rcs.push((api.read_close)(st));
                }
                fclose(f);
                outs.push((rcs, n));
            }
            assert_eq!(
                outs[0].0,
                outs[1].0,
                "truncated read rcs cut={}: {:?} vs {:?}",
                cut,
                outs[0].0.iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>(),
                outs[1].0.iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>()
            );
            assert_eq!(outs[0].1, outs[1].1, "truncated read bytes cut={}", cut);
        }
        // corrupted frame body -> both must report the same error
        for _ in 0..40 {
            let mut bad = frame.to_vec();
            let pos = rng.range(LZ4F_HEADER_SIZE_MAX, bad.len());
            bad[pos] ^= 0xFF;
            let mut outs = Vec::new();
            for which in 0..2 {
                let api = if which == 0 { &c } else { &r };
                let f = file_with(&bad);
                let mut st: *mut c_void = std::ptr::null_mut();
                let mut rcs = Vec::new();
                let rc0 = (api.read_open)(&mut st, f);
                rcs.push(rc0);
                if (api.is_error)(rc0) == 0 {
                    let mut buf = vec![0u8; 8192];
                    loop {
                        let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, 8192);
                        rcs.push(got);
                        if (api.is_error)(got) != 0 || got == 0 {
                            break;
                        }
                    }
                    rcs.push((api.read_close)(st));
                }
                fclose(f);
                outs.push(rcs);
            }
            assert_eq!(
                outs[0],
                outs[1],
                "corrupted read rcs pos={}: {:?} vs {:?}",
                pos,
                outs[0].iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>(),
                outs[1].iter().map(|&x| fmt_lz4f(x)).collect::<Vec<_>>()
            );
        }
        let _ = (c.get_error_name)(0);
        let _ = (r.get_error_name)(0);
    }
}

// --- CONFIGS rows 141, 145: multi-megabyte writes and the documented
// --- end-to-end round trip with the "everything on" preference set.
#[test]
fn file_large_round_trip() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5005);
    let total = 5 << 20;
    let data = gen(Shape::Text, total, &mut rng);

    let mut p1 = LZ4F_preferences_t::default();
    p1.frameInfo.blockSizeID = 7; // max4MB
    p1.frameInfo.blockMode = 1; // blockIndependent
    p1.frameInfo.contentChecksumFlag = 1;
    p1.frameInfo.blockChecksumFlag = 1;
    p1.frameInfo.contentSize = total as u64;
    p1.compressionLevel = 9;
    p1.autoFlush = 1;

    let mut p2 = LZ4F_preferences_t::default();
    p2.frameInfo.blockSizeID = 4; // max64KB
    p2.frameInfo.blockMode = 0; // blockLinked

    for (tag, p) in [("all-on max4MB", p1), ("plain max64KB", p2)] {
        // 3 write chunks (one of them larger than maxWriteSize) and a single
        // 5 MB write, read back in 64 KB and in one 6 MB chunk.
        for wmode in 0..2 {
            for &rchunk in &[64 * 1024usize, 6 << 20] {
                unsafe {
                    let mut bytes = Vec::new();
                    let mut outs = Vec::new();
                    let mut closes = Vec::new();
                    for which in 0..2 {
                        let api = if which == 0 { &c } else { &r };
                        let f = tmpfile();
                        let mut st: *mut c_void = std::ptr::null_mut();
                        let rc0 = (api.write_open)(&mut st, f, &p);
                        assert_eq!(rc0, 0, "{} writeOpen", tag);
                        if wmode == 0 {
                            let parts = [total / 5, total / 2, total - total / 5 - total / 2];
                            let mut off = 0usize;
                            for &n in &parts {
                                assert_eq!(
                                    (api.write)(st, data.as_ptr().add(off) as *const c_void, n),
                                    n,
                                    "{} chunked write",
                                    tag
                                );
                                off += n;
                            }
                        } else {
                            assert_eq!(
                                (api.write)(st, data.as_ptr() as *const c_void, total),
                                total,
                                "{} single write",
                                tag
                            );
                        }
                        let rcc = (api.write_close)(st);
                        assert!((api.is_error)(rcc) == 0, "{} writeClose: {}", tag, fmt_lz4f(rcc));
                        closes.push(rcc);
                        bytes.push(slurp(f));
                        rewind(f);
                        let mut st: *mut c_void = std::ptr::null_mut();
                        let rc0 = (api.read_open)(&mut st, f);
                        assert_eq!(rc0, 0, "{} readOpen", tag);
                        let mut out = Vec::with_capacity(total);
                        let mut buf = vec![0u8; rchunk];
                        loop {
                            let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, rchunk);
                            assert!((api.is_error)(got) == 0, "{} read: {}", tag, fmt_lz4f(got));
                            if got == 0 {
                                break;
                            }
                            out.extend_from_slice(&buf[..got]);
                        }
                        assert_eq!((api.read_close)(st), 0, "{} readClose", tag);
                        fclose(f);
                        outs.push(out);
                    }
                    assert_eq!(closes[0], closes[1], "{} writeClose rc", tag);
                    assert_bytes_eq(&format!("{} file bytes wmode={} rchunk={}", tag, wmode, rchunk), &bytes[0], &bytes[1]);
                    assert_bytes_eq(&format!("{} round trip", tag), &outs[0], &outs[1]);
                    assert_bytes_eq(&format!("{} content", tag), &outs[0], &data);
                }
            }
        }
    }
}

// --- CONFIGS row 142: header lengths of 7 and 19 bytes so the readOpen
// --- pre-read leaves 12 vs 0 residual bytes.
#[test]
fn file_header_residual() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5006);
    for &(cs_flag, dict_flag) in &[(false, false), (true, false), (false, true), (true, true)] {
        for &bsid in &[0i32, 4, 5, 6, 7] {
            let total = 200_000usize;
            let data = gen(Shape::Text, total, &mut rng);
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = bsid;
            if cs_flag {
                p.frameInfo.contentSize = total as u64;
            }
            if dict_flag {
                p.frameInfo.dictID = 0x1234_5678;
            }
            unsafe {
                let mut outs = Vec::new();
                let mut bytes = Vec::new();
                for which in 0..2 {
                    let api = if which == 0 { &c } else { &r };
                    let f = tmpfile();
                    let mut st: *mut c_void = std::ptr::null_mut();
                    assert_eq!((api.write_open)(&mut st, f, &p), 0);
                    assert_eq!((api.write)(st, data.as_ptr() as *const c_void, total), total);
                    let rcc = (api.write_close)(st);
                    assert!((api.is_error)(rcc) == 0);
                    bytes.push(slurp(f));
                    rewind(f);
                    let mut st: *mut c_void = std::ptr::null_mut();
                    let rc0 = (api.read_open)(&mut st, f);
                    assert_eq!(rc0, 0, "readOpen cs={} dict={} bsid={}", cs_flag, dict_flag, bsid);
                    let mut out = Vec::new();
                    let mut buf = vec![0u8; 40_000];
                    loop {
                        let got = (api.read)(st, buf.as_mut_ptr() as *mut c_void, 40_000);
                        assert!((api.is_error)(got) == 0);
                        if got == 0 {
                            break;
                        }
                        out.extend_from_slice(&buf[..got]);
                    }
                    assert_eq!((api.read_close)(st), 0);
                    fclose(f);
                    outs.push(out);
                }
                assert_bytes_eq(
                    &format!("header residual bytes cs={} dict={} bsid={}", cs_flag, dict_flag, bsid),
                    &bytes[0],
                    &bytes[1],
                );
                assert_bytes_eq("header residual round trip", &outs[0], &outs[1]);
                assert_bytes_eq("header residual content", &outs[0], &data);
            }
        }
    }
}
