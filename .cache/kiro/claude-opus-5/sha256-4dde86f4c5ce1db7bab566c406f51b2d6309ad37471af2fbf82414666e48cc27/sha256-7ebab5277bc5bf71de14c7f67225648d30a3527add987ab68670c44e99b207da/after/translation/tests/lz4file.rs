//! LZ4 file API (`lz4file.c`) — driven through real `FILE*` handles.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

unsafe extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(fp: *mut c_void) -> c_int;
    fn fflush(fp: *mut c_void) -> c_int;
    fn remove(path: *const c_char) -> c_int;
}

/// `LZ4F_frameInfo_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct FrameInfo {
    block_size_id: u32,
    block_mode: u32,
    content_checksum_flag: u32,
    frame_type: u32,
    content_size: u64,
    dict_id: u32,
    block_checksum_flag: u32,
}

/// `LZ4F_preferences_t`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Preferences {
    frame_info: FrameInfo,
    compression_level: i32,
    auto_flush: u32,
    favor_dec_speed: u32,
    reserved: [u32; 3],
}

fn tmp_path(tag: &str) -> (String, CString) {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let p = format!("{}/lz4file_test_{}_{}.lz4", dir, std::process::id(), tag);
    let c = CString::new(p.clone()).unwrap();
    (p, c)
}

fn prefs_set() -> Vec<Preferences> {
    let mut v = Vec::new();
    let variants: [(u32, u32, u32, u32, i32, u32); 7] = [
        // blockSizeID, blockMode, contentChecksum, blockChecksum, level, autoFlush
        (0, 0, 0, 0, 0, 0),
        (4, 0, 1, 0, 1, 0),
        (4, 1, 0, 1, 3, 1),
        (5, 0, 1, 1, 9, 0),
        (6, 1, 1, 0, 12, 0),
        (7, 0, 0, 0, 10, 1),
        (7, 1, 1, 1, 2, 0),
    ];
    for &(bs, bm, cc, bc, lvl, af) in &variants {
        let mut p = Preferences::default();
        p.frame_info.block_size_id = bs;
        p.frame_info.block_mode = bm;
        p.frame_info.content_checksum_flag = cc;
        p.frame_info.block_checksum_flag = bc;
        p.compression_level = lvl;
        p.auto_flush = af;
        v.push(p);
    }
    v
}

fn is_lz4f_error(code: usize) -> bool {
    let (cf, rf) = pair!("LZ4F_isError", fn(usize) -> u32);
    unsafe {
        let a = cf(code);
        assert_eq!(a, rf(code), "LZ4F_isError({})", code as isize);
        a != 0
    }
}

/// Write `data` through `LZ4F_write*` with the given library side and return the
/// file contents plus the return codes of every call.
fn write_via(
    which: bool,
    data: &[u8],
    chunks: &[usize],
    prefs: Option<&Preferences>,
    tag: &str,
) -> (Vec<u8>, Vec<usize>) {
    let libs = common::libs();
    unsafe {
        let wopen = libs.get::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize>(
            which,
            "LZ4F_writeOpen",
        );
        let write = libs
            .get::<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize>(which, "LZ4F_write");
        let wclose =
            libs.get::<unsafe extern "C" fn(*mut c_void) -> usize>(which, "LZ4F_writeClose");

        let (path, cpath) = tmp_path(&format!("{}_{}", tag, which as u8));
        let mode = CString::new("wb").unwrap();
        let fp = fopen(cpath.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null(), "fopen {} failed", path);

        let mut rets = Vec::new();
        let mut h: *mut c_void = std::ptr::null_mut();
        let p = prefs.map_or(std::ptr::null(), |p| p as *const Preferences);
        rets.push(wopen(&mut h, fp, p));
        let mut pos = 0usize;
        let mut i = 0usize;
        while pos < data.len() {
            let n = chunks[i % chunks.len()].min(data.len() - pos);
            rets.push(write(h, data[pos..].as_ptr(), n));
            pos += n;
            i += 1;
        }
        rets.push(wclose(h));
        fflush(fp);
        fclose(fp);
        let bytes = std::fs::read(&path).unwrap();
        remove(cpath.as_ptr());
        (bytes, rets)
    }
}

/// Read a frame file with `LZ4F_read*` and return the decoded bytes plus the
/// return code of every call.
fn read_via(which: bool, frame: &[u8], chunks: &[usize], tag: &str) -> (Vec<u8>, Vec<usize>) {
    let libs = common::libs();
    unsafe {
        let ropen = libs.get::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize>(
            which,
            "LZ4F_readOpen",
        );
        let read = libs
            .get::<unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> usize>(which, "LZ4F_read");
        let rclose = libs.get::<unsafe extern "C" fn(*mut c_void) -> usize>(which, "LZ4F_readClose");

        let (path, cpath) = tmp_path(&format!("rd_{}_{}", tag, which as u8));
        std::fs::write(&path, frame).unwrap();
        let mode = CString::new("rb").unwrap();
        let fp = fopen(cpath.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null());

        let mut rets = Vec::new();
        let mut out: Vec<u8> = Vec::new();
        let mut h: *mut c_void = std::ptr::null_mut();
        let r = ropen(&mut h, fp);
        rets.push(r);
        if h.is_null() {
            fclose(fp);
            remove(cpath.as_ptr());
            return (out, rets);
        }
        let mut i = 0usize;
        loop {
            let n = chunks[i % chunks.len()];
            i += 1;
            let mut buf = vec![0u8; n.max(1)];
            let got = read(h, buf.as_mut_ptr(), n);
            rets.push(got);
            // errors are returned as (size_t)-errorCode, i.e. huge values
            if got == 0 || got > n {
                break;
            }
            out.extend_from_slice(&buf[..got]);
            if i > 100_000 {
                panic!("read loop did not terminate");
            }
        }
        rets.push(rclose(h));
        fclose(fp);
        remove(cpath.as_ptr());
        (out, rets)
    }
}

#[test]
fn file_write_roundtrip() {
    let chunkings: [&[usize]; 5] = [&[1], &[7, 3, 100], &[4096], &[65536, 1, 1000], &[300, 30000]];
    for (gname, g) in GENS {
        for cks in &chunkings {
            let total = if cks == &&[1usize][..] { 2_000 } else { 90_000 };
            let data = g(total, 251 + gname.len() as u64);
            for (pi, p) in prefs_set().iter().enumerate() {
                let (fc, rc) = write_via(false, &data, cks, Some(p), "w");
                let (fr, rr) = write_via(true, &data, cks, Some(p), "w");
                assert_eq!(rc, rr, "writeOpen/write/writeClose returns {} pref#{}", gname, pi);
                cmp_bytes(&fc, &fr, &format!("written frame {} pref#{}", gname, pi));

                // and both must read back to the original
                let (oc, rrc) = read_via(false, &fc, &[65536], "w");
                let (or_, rrr) = read_via(true, &fc, &[65536], "w");
                assert_eq!(rrc, rrr, "read returns {} pref#{}", gname, pi);
                cmp_bytes(&oc, &or_, &format!("read output {} pref#{}", gname, pi));
                if !is_lz4f_error(rrc[0]) {
                    assert_eq!(&oc[..], &data[..], "file roundtrip {} pref#{}", gname, pi);
                }
            }
            // NULL preferences
            let (fc, rc) = write_via(false, &data, cks, None, "wn");
            let (fr, rr) = write_via(true, &data, cks, None, "wn");
            assert_eq!(rc, rr, "writeOpen NULL prefs returns {}", gname);
            cmp_bytes(&fc, &fr, &format!("written frame NULL prefs {}", gname));
        }
    }
}

#[test]
fn file_read_various_chunkings() {
    let (c_cf, _) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    let read_chunkings: [&[usize]; 6] = [
        &[1],
        &[3, 17],
        &[4096],
        &[65536],
        &[1 << 18],
        &[100, 1, 70000],
    ];
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[0usize, 1, 100, 5000, 70000] {
                let data = g(sz, 261 + sz as u64);
                for (pi, p) in prefs_set().iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut frame = vec![0u8; bound + 64];
                    let n = c_cf(frame.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    frame.truncate(n);
                    for cks in &read_chunkings {
                        if cks[0] == 1 && sz > 5000 {
                            continue;
                        }
                        let (oc, rc) = read_via(false, &frame, cks, "r");
                        let (or_, rr) = read_via(true, &frame, cks, "r");
                        assert_eq!(
                            rc, rr,
                            "read returns {} sz={} pref#{} chunks={:?}",
                            gname, sz, pi, cks
                        );
                        cmp_bytes(
                            &oc,
                            &or_,
                            &format!("read output {} sz={} pref#{} chunks={:?}", gname, sz, pi, cks),
                        );
                        // LZ4F_readOpen unconditionally freads
                        // LZ4F_HEADER_SIZE_MAX (19) bytes and reports io_read if
                        // the file is shorter, so very small frames cannot be
                        // opened at all. Only check the payload when the open
                        // succeeded.
                        if !is_lz4f_error(rc[0]) {
                            assert_eq!(&oc[..], &data[..], "read roundtrip {} sz={}", gname, sz);
                        } else {
                            assert!(
                                frame.len() < 19,
                                "unexpected readOpen failure {} sz={} pref#{} (frame {} bytes)",
                                gname,
                                sz,
                                pi,
                                frame.len()
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn file_read_corrupt_and_truncated() {
    let (c_cf, _) = pair!(
        "LZ4F_compressFrame",
        fn(*mut u8, usize, *const u8, usize, *const Preferences) -> usize
    );
    let (c_fb, _) = pair!(
        "LZ4F_compressFrameBound",
        fn(usize, *const Preferences) -> usize
    );
    let mut rng = Rng::new(0x5EED);
    unsafe {
        for (gname, g) in GENS {
            for &sz in &[100usize, 5000, 70000] {
                let data = g(sz, 271 + sz as u64);
                for (pi, p) in prefs_set().iter().enumerate() {
                    let bound = c_fb(sz, p);
                    let mut frame = vec![0u8; bound + 64];
                    let n = c_cf(frame.as_mut_ptr(), bound, data.as_ptr(), sz, p);
                    frame.truncate(n);

                    for cut in [1usize, 2, 4, 8, 16, n / 2, n - 1] {
                        if cut >= n {
                            continue;
                        }
                        let t = &frame[..n - cut];
                        let (oc, rc) = read_via(false, t, &[4096], "tr");
                        let (or_, rr) = read_via(true, t, &[4096], "tr");
                        assert_eq!(
                            rc, rr,
                            "truncated read returns {} sz={} pref#{} cut={}",
                            gname, sz, pi, cut
                        );
                        cmp_bytes(
                            &oc,
                            &or_,
                            &format!("truncated read output {} sz={} cut={}", gname, sz, cut),
                        );
                    }
                    for _ in 0..8 {
                        let mut f = frame.clone();
                        let i = (rng.below(f.len() as u32)) as usize;
                        f[i] ^= 1 << rng.below(8);
                        let (oc, rc) = read_via(false, &f, &[4096], "cr");
                        let (or_, rr) = read_via(true, &f, &[4096], "cr");
                        assert_eq!(
                            rc, rr,
                            "corrupt read returns {} sz={} pref#{} at={}",
                            gname, sz, pi, i
                        );
                        cmp_bytes(
                            &oc,
                            &or_,
                            &format!("corrupt read output {} sz={} at={}", gname, sz, i),
                        );
                    }
                }
            }
        }
        // garbage and empty files
        let mut inputs: Vec<Vec<u8>> = vec![vec![], vec![4], vec![4, 0x22, 0x4d, 0x18]];
        for i in 0..40 {
            inputs.push(gen_random(1 + i * 3, 4000 + i as u64));
        }
        for (ii, inp) in inputs.iter().enumerate() {
            let (oc, rc) = read_via(false, inp, &[4096], "gr");
            let (or_, rr) = read_via(true, inp, &[4096], "gr");
            assert_eq!(rc, rr, "garbage read returns #{}", ii);
            cmp_bytes(&oc, &or_, &format!("garbage read output #{}", ii));
        }
    }
}

/// Write and read back through the file API on the *other* library, in both
/// directions, so the two implementations must interoperate.
#[test]
fn file_cross_interop() {
    let chunks: [usize; 3] = [1000, 65536, 7];
    for (gname, g) in GENS {
        let data = g(50_000, 281 + gname.len() as u64);
        for (pi, p) in prefs_set().iter().enumerate() {
            let (fc, _) = write_via(false, &data, &chunks, Some(p), "x");
            let (fr, _) = write_via(true, &data, &chunks, Some(p), "x");
            cmp_bytes(&fc, &fr, &format!("cross frame {} pref#{}", gname, pi));
            // C-written frame read by Rust, and vice versa
            let (a, _) = read_via(true, &fc, &[4096], "x");
            let (b, _) = read_via(false, &fr, &[4096], "x");
            assert_eq!(&a[..], &data[..], "rust reads C frame {} pref#{}", gname, pi);
            assert_eq!(&b[..], &data[..], "C reads rust frame {} pref#{}", gname, pi);
        }
    }
}
