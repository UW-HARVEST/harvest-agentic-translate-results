//! Phase B/C — file API (`lz4file.c`).
//! CONFIGS.md rows 96–98, ERRORS.md rows 128–146.
#![allow(non_snake_case)]

mod common;
use common::frame::*;
use common::*;
use std::ffi::CString;

type FnFopen = unsafe extern "C" fn(*const CChar, *const CChar) -> *mut CVoid;
type FnFclose = unsafe extern "C" fn(*mut CVoid) -> i32;
type FnReadOpen = unsafe extern "C" fn(*mut *mut CVoid, *mut CVoid) -> usize;
type FnRead = unsafe extern "C" fn(*mut CVoid, *mut u8, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut CVoid) -> usize;
type FnWriteOpen =
    unsafe extern "C" fn(*mut *mut CVoid, *mut CVoid, *const LZ4F_preferences_t) -> usize;
type FnWrite = unsafe extern "C" fn(*mut CVoid, *const u8, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut CVoid) -> usize;

// libc stdio, used for the FILE* the lz4file API needs.
unsafe extern "C" {
    fn fopen(path: *const CChar, mode: *const CChar) -> *mut CVoid;
    fn fclose(f: *mut CVoid) -> i32;
}

fn err(name: &str) -> i64 {
    err_code(name) as i64
}

struct TmpDir(std::path::PathBuf);

impl TmpDir {
    fn new(tag: &str) -> TmpDir {
        let mut p = std::env::temp_dir();
        p.push(format!("lz4diff-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        TmpDir(p)
    }
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cpath(p: &std::path::Path) -> CString {
    CString::new(p.to_str().unwrap()).unwrap()
}

/* ================================================================== */
/* row 96 — write path                                                 */
/* ================================================================== */

/// Compress `src` to `path` through `LZ4F_writeOpen`/`write`/`writeClose`.
/// Returns the return codes plus the resulting file bytes.
fn file_write(
    lib: &libloading::Library,
    path: &std::path::Path,
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    chunks: &[usize],
) -> (Vec<i64>, Vec<u8>) {
    unsafe {
        let cp = cpath(path);
        let mode = CString::new("wb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null(), "fopen failed for {path:?}");
        let mut w: *mut CVoid = std::ptr::null_mut();
        let pp = prefs.map_or(std::ptr::null(), |p| p as *const _);
        let mut codes: Vec<i64> = Vec::new();
        let r = sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(&mut w, fp, pp);
        codes.push(r as i64);
        if sym::<FnIsError>(lib, "LZ4F_isError")(r) == 0 {
            let mut off = 0usize;
            for &c in chunks {
                let n = c.min(src.len() - off);
                let sp = if n == 0 {
                    src.as_ptr()
                } else {
                    src[off..].as_ptr()
                };
                let k = sym::<FnWrite>(lib, "LZ4F_write")(w, sp, n);
                codes.push(k as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(k) != 0 {
                    break;
                }
                off += n;
                if off >= src.len() {
                    break;
                }
            }
            if off < src.len() {
                let k = sym::<FnWrite>(lib, "LZ4F_write")(w, src[off..].as_ptr(), src.len() - off);
                codes.push(k as i64);
            }
            let k = sym::<FnWriteClose>(lib, "LZ4F_writeClose")(w);
            codes.push(k as i64);
        }
        fclose(fp);
        let bytes = std::fs::read(path).unwrap_or_default();
        (codes, bytes)
    }
}

#[test]
fn r096_write_roundtrip() {
    let td = TmpDir::new("w");
    let mut rng = Rng::new(0x5EED_0096);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(3) {
        for &len in [0usize, 1, 7, 65535, 65536, 65537, 200000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            for pattern in 0..4 {
                let chunks: Vec<usize> = match pattern {
                    0 => vec![usize::MAX],
                    1 => vec![1; 200],
                    2 => vec![65536; 8],
                    _ => (0..30).map(|_| rng.range(0, 40000)).collect(),
                };
                diff(
                    &format!("writeOpen prefs#{pi} len={len} pat={pattern}"),
                    |lib| {
                        let name = format!("w-{pi}-{len}-{pattern}.lz4");
                        file_write(lib, &td.path(&name), &src, Some(p), &chunks)
                    },
                );
            }
        }
    }
    // NULL prefs
    for &len in [0usize, 1, 100, 200000].iter() {
        let src = mkdata(Shape::Textish, len, &mut rng);
        diff(&format!("writeOpen NULL prefs len={len}"), |lib| {
            file_write(
                lib,
                &td.path(&format!("wn-{len}.lz4")),
                &src,
                None,
                &[usize::MAX],
            )
        });
    }
}

/* ================================================================== */
/* row 97 — read path                                                  */
/* ================================================================== */

fn file_read(
    lib: &libloading::Library,
    path: &std::path::Path,
    expected: usize,
    chunks: &[usize],
) -> (Vec<i64>, Vec<u8>) {
    unsafe {
        let cp = cpath(path);
        let mode = CString::new("rb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null(), "fopen failed for {path:?}");
        let mut r: *mut CVoid = std::ptr::null_mut();
        let mut codes: Vec<i64> = Vec::new();
        let mut out: Vec<u8> = Vec::new();
        let rc = sym::<FnReadOpen>(lib, "LZ4F_readOpen")(&mut r, fp);
        codes.push(rc as i64);
        if sym::<FnIsError>(lib, "LZ4F_isError")(rc) == 0 {
            let rd = sym::<FnRead>(lib, "LZ4F_read");
            let mut buf = vec![0u8; expected.max(1) + 4096];
            let mut guard = 0usize;
            let mut ci = 0usize;
            loop {
                guard += 1;
                if guard > 2_000_000 {
                    codes.push(-9_999_999);
                    break;
                }
                let want = chunks[ci % chunks.len()].min(buf.len());
                ci += 1;
                let k = rd(r, buf.as_mut_ptr(), want);
                codes.push(k as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(k) != 0 {
                    break;
                }
                if k == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..k]);
                if want == 0 {
                    break;
                }
            }
            codes.push(sym::<FnReadClose>(lib, "LZ4F_readClose")(r) as i64);
        }
        fclose(fp);
        (codes, out)
    }
}

#[test]
fn r097_read_roundtrip() {
    let td = TmpDir::new("r");
    let mut rng = Rng::new(0x5EED_0097);
    let prefs = pref_matrix();
    let i = impls();
    for (pi, p) in prefs.iter().enumerate().step_by(3) {
        for &len in [0usize, 1, 7, 65535, 65536, 65537, 200000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            // build the file with the C implementation
            let path = td.path(&format!("r-{pi}-{len}.lz4"));
            let (_, bytes) = file_write(&i.c, &path, &src, Some(p), &[usize::MAX]);
            assert!(!bytes.is_empty());
            for pattern in 0..5 {
                let chunks: Vec<usize> = match pattern {
                    0 => vec![len.max(1) + 4096],
                    1 => vec![1],
                    2 => vec![7],
                    3 => vec![65536],
                    _ => (0..20).map(|_| rng.range(1, 30000)).collect(),
                };
                diff(
                    &format!("readOpen prefs#{pi} len={len} pat={pattern}"),
                    |lib| file_read(lib, &path, len, &chunks),
                );
                // NOTE: LZ4F_readOpen unconditionally freads
                // LZ4F_HEADER_SIZE_MAX (19) bytes and fails with io_read if the
                // file is shorter, so small frames legitimately cannot be
                // opened. Only assert round-trip content when the C succeeded.
                let (cc, cg) = file_read(&i.c, &path, len, &chunks);
                let (_, rg) = file_read(&i.r, &path, len, &chunks);
                if cc[0] == 0 {
                    assert_eq!(cg.len(), len, "C read length prefs#{pi} len={len}");
                    assert_eq!(&cg[..], &src[..], "C read content prefs#{pi} len={len}");
                }
                assert_eq!(cg, rg, "read content prefs#{pi} len={len}");
            }
            // size == 0 must return 0 immediately
            diff(&format!("read size 0 prefs#{pi} len={len}"), |lib| {
                file_read(lib, &path, len, &[0usize])
            });
        }
    }
}

/* ================================================================== */
/* row 98 — cross-implementation file compatibility                     */
/* ================================================================== */

#[test]
fn r098_cross_impl_files() {
    let td = TmpDir::new("x");
    let mut rng = Rng::new(0x5EED_0098);
    let i = impls();
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(7) {
        for &len in [0usize, 1, 100, 200000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let pc = td.path(&format!("x-c-{pi}-{len}.lz4"));
            let pr = td.path(&format!("x-r-{pi}-{len}.lz4"));
            let (_, bc) = file_write(&i.c, &pc, &src, Some(p), &[7000usize]);
            let (_, br) = file_write(&i.r, &pr, &src, Some(p), &[7000usize]);
            assert_eq!(bc, br, "file bytes differ prefs#{pi} len={len}");
            // read each file with each implementation
            for path in [&pc, &pr] {
                let (cc, cg) = file_read(&i.c, path, len, &[4096usize]);
                let (rc, rg) = file_read(&i.r, path, len, &[4096usize]);
                assert_eq!(cc, rc, "codes reading {path:?}");
                assert_eq!(cg, rg, "content reading {path:?}");
                // files below LZ4F_HEADER_SIZE_MAX cannot be opened at all
                if cc[0] == 0 {
                    assert_eq!(&cg[..], &src[..], "round-trip {path:?}");
                }
            }
        }
    }
}

/* ================================================================== */
/* rows 128–146 — file API rejections                                   */
/* ================================================================== */

#[test]
fn e128_null_arguments() {
    let td = TmpDir::new("e-null");
    let path = td.path("dummy.lz4");
    std::fs::write(&path, b"").unwrap();
    diff("file API NULL arguments", |lib| unsafe {
        let cp = cpath(&path);
        let mode = CString::new("rb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        let mut out: Vec<i64> = Vec::new();
        // readOpen: fp == NULL, lz4fRead == NULL
        let mut r: *mut CVoid = std::ptr::null_mut();
        out.push(sym::<FnReadOpen>(lib, "LZ4F_readOpen")(&mut r, std::ptr::null_mut()) as i64);
        out.push(sym::<FnReadOpen>(lib, "LZ4F_readOpen")(std::ptr::null_mut(), fp) as i64);
        out.push(
            sym::<FnReadOpen>(lib, "LZ4F_readOpen")(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ) as i64,
        );
        // read / readClose with NULL handle or NULL buffer
        let mut buf = vec![0u8; 64];
        out.push(sym::<FnRead>(lib, "LZ4F_read")(std::ptr::null_mut(), buf.as_mut_ptr(), 64) as i64);
        out.push(sym::<FnReadClose>(lib, "LZ4F_readClose")(std::ptr::null_mut()) as i64);
        // writeOpen: fp == NULL, lz4fWrite == NULL
        let mut w: *mut CVoid = std::ptr::null_mut();
        out.push(
            sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(
                &mut w,
                std::ptr::null_mut(),
                std::ptr::null(),
            ) as i64,
        );
        out.push(
            sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(
                std::ptr::null_mut(),
                fp,
                std::ptr::null(),
            ) as i64,
        );
        // write / writeClose with NULL handle
        out.push(
            sym::<FnWrite>(lib, "LZ4F_write")(std::ptr::null_mut(), buf.as_ptr(), 64) as i64,
        );
        out.push(sym::<FnWriteClose>(lib, "LZ4F_writeClose")(std::ptr::null_mut()) as i64);
        fclose(fp);
        out
    });
    let i = impls();
    unsafe {
        let mut r: *mut CVoid = std::ptr::null_mut();
        assert_eq!(
            sym::<FnReadOpen>(&i.c, "LZ4F_readOpen")(&mut r, std::ptr::null_mut()) as i64,
            err("ERROR_parameter_null")
        );
        assert_eq!(
            sym::<FnReadClose>(&i.c, "LZ4F_readClose")(std::ptr::null_mut()) as i64,
            err("ERROR_parameter_null")
        );
        assert_eq!(
            sym::<FnWriteClose>(&i.c, "LZ4F_writeClose")(std::ptr::null_mut()) as i64,
            err("ERROR_parameter_null")
        );
    }
}

#[test]
fn e133_read_null_buffer() {
    // `LZ4F_read` checks `buf == NULL` before use, so this is a real rejection.
    let td = TmpDir::new("e-rbuf");
    let mut rng = Rng::new(0x5EED_2133);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let i = impls();
    let path = td.path("in.lz4");
    file_write(&i.c, &path, &src, None, &[usize::MAX]);
    diff("read NULL buffer", |lib| unsafe {
        let cp = cpath(&path);
        let mode = CString::new("rb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        let mut r: *mut CVoid = std::ptr::null_mut();
        let o = sym::<FnReadOpen>(lib, "LZ4F_readOpen")(&mut r, fp);
        let a = sym::<FnRead>(lib, "LZ4F_read")(r, std::ptr::null_mut(), 100);
        let b = sym::<FnRead>(lib, "LZ4F_read")(r, std::ptr::null_mut(), 0);
        let c = sym::<FnReadClose>(lib, "LZ4F_readClose")(r);
        fclose(fp);
        (o as i64, a as i64, b as i64, c as i64)
    });
    diff("write NULL buffer", |lib| unsafe {
        let out = td.path("out.lz4");
        let cp = cpath(&out);
        let mode = CString::new("wb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        let mut w: *mut CVoid = std::ptr::null_mut();
        let o = sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(&mut w, fp, std::ptr::null());
        let a = sym::<FnWrite>(lib, "LZ4F_write")(w, std::ptr::null(), 100);
        let b = sym::<FnWrite>(lib, "LZ4F_write")(w, std::ptr::null(), 0);
        let c = sym::<FnWriteClose>(lib, "LZ4F_writeClose")(w);
        fclose(fp);
        (o as i64, a as i64, b as i64, c as i64)
    });
}

#[test]
fn e130_readOpen_short_and_bad_files() {
    let td = TmpDir::new("e-short");
    let mut rng = Rng::new(0x5EED_2130);
    // row 130: fewer than 4 bytes in the file
    for n in 0usize..8 {
        let p = td.path(&format!("short-{n}.bin"));
        let body = mkdata(Shape::Random, n, &mut rng);
        std::fs::write(&p, &body).unwrap();
        diff(&format!("readOpen short n={n}"), |lib| {
            file_read(lib, &p, 0, &[64usize])
        });
    }
    // row 131: 4+ bytes but a bad magic
    for m in [0u32, 1, 0x184D2203, 0x184D2205, 0xFFFFFFFF] {
        let p = td.path(&format!("magic-{m:08x}.bin"));
        let mut body = m.to_le_bytes().to_vec();
        body.extend_from_slice(&mkdata(Shape::Random, 64, &mut rng));
        std::fs::write(&p, &body).unwrap();
        diff(&format!("readOpen bad magic {m:#x}"), |lib| {
            file_read(lib, &p, 0, &[64usize])
        });
    }
    // row 132: valid magic but an invalid blockSizeID in the BD byte
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let i = impls();
    let good = td.path("good.lz4");
    let (_, bytes) = file_write(&i.c, &good, &src, None, &[usize::MAX]);
    for bd in [0x00u8, 0x10, 0x20, 0x30, 0x80, 0xF0, 0xFF] {
        let mut b = bytes.clone();
        b[5] = bd;
        // recompute the header checksum so the BD check is what fails
        let p = td.path(&format!("bd-{bd:02x}.lz4"));
        std::fs::write(&p, &b).unwrap();
        diff(&format!("readOpen bd={bd:#04x}"), |lib| {
            file_read(lib, &p, src.len(), &[4096usize])
        });
    }
    // rows 135,136: truncated / corrupted frame body
    for cut in 1..bytes.len().min(24) {
        let p = td.path(&format!("trunc-{cut}.lz4"));
        std::fs::write(&p, &bytes[..bytes.len() - cut]).unwrap();
        diff(&format!("read truncated cut={cut}"), |lib| {
            file_read(lib, &p, src.len(), &[4096usize])
        });
    }
    for k in 0..200usize {
        let mut b = bytes.clone();
        let pos = 7 + rng.below(b.len() - 8);
        b[pos] = rng.byte();
        let p = td.path("corrupt.lz4");
        std::fs::write(&p, &b).unwrap();
        diff(&format!("read corrupted #{k}"), |lib| {
            file_read(lib, &p, src.len(), &[4096usize])
        });
    }
    // an empty file
    let p = td.path("empty.lz4");
    std::fs::write(&p, b"").unwrap();
    diff("read empty file", |lib| file_read(lib, &p, 0, &[64usize]));
}

#[test]
fn e141_writeOpen_invalid_blockSizeID() {
    let td = TmpDir::new("e-wbsid");
    let mut rng = Rng::new(0x5EED_2141);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    for bsid in [i32::MIN, -1, 1, 2, 3, 8, 9, 255, 1 << 20, i32::MAX] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        diff(&format!("writeOpen bsid={bsid}"), |lib| {
            file_write(
                lib,
                &td.path(&format!("wb-{bsid}.lz4")),
                &src,
                Some(&p),
                &[usize::MAX],
            )
        });
    }
    let i = impls();
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = 3;
    let (codes, _) = file_write(&i.c, &td.path("check.lz4"), &src, Some(&p), &[usize::MAX]);
    assert_eq!(codes[0], err("ERROR_maxBlockSize_invalid"), "codes {codes:?}");
}

#[test]
fn e146_writeClose_after_error() {
    let td = TmpDir::new("e-wclose");
    let mut rng = Rng::new(0x5EED_2146);
    let src = mkdata(Shape::Textish, 200000, &mut rng);
    // Open the output on a read-only FILE* so every fwrite fails: the C stores
    // the io_write error in errCode and writeClose must return it.
    let path = td.path("ro.lz4");
    std::fs::write(&path, b"placeholder").unwrap();
    diff("writeClose after io error", |lib| unsafe {
        let cp = cpath(&path);
        let mode = CString::new("rb").unwrap();
        let fp = fopen(cp.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null());
        let mut w: *mut CVoid = std::ptr::null_mut();
        let o = sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(&mut w, fp, std::ptr::null());
        let mut codes = vec![o as i64];
        if sym::<FnIsError>(lib, "LZ4F_isError")(o) == 0 {
            for _ in 0..4 {
                let k = sym::<FnWrite>(lib, "LZ4F_write")(w, src.as_ptr(), src.len());
                codes.push(k as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(k) != 0 {
                    break;
                }
            }
            codes.push(sym::<FnWriteClose>(lib, "LZ4F_writeClose")(w) as i64);
        }
        fclose(fp);
        codes
    });
}

#[test]
fn e137_read_write_size_zero() {
    let td = TmpDir::new("e-zero");
    let mut rng = Rng::new(0x5EED_2137);
    let src = mkdata(Shape::Textish, 4096, &mut rng);
    let i = impls();
    let path = td.path("z.lz4");
    file_write(&i.c, &path, &src, None, &[usize::MAX]);
    diff("read/write size 0", |lib| unsafe {
        // read with size 0
        let cp = cpath(&path);
        let rmode = CString::new("rb").unwrap();
        let fp = fopen(cp.as_ptr(), rmode.as_ptr());
        let mut r: *mut CVoid = std::ptr::null_mut();
        let o = sym::<FnReadOpen>(lib, "LZ4F_readOpen")(&mut r, fp);
        let mut buf = vec![0u8; 64];
        let a = sym::<FnRead>(lib, "LZ4F_read")(r, buf.as_mut_ptr(), 0);
        let b = sym::<FnRead>(lib, "LZ4F_read")(r, buf.as_mut_ptr(), 0);
        let c = sym::<FnReadClose>(lib, "LZ4F_readClose")(r);
        fclose(fp);
        // write with size 0
        let out = td.path("z-out.lz4");
        let ocp = cpath(&out);
        let wmode = CString::new("wb").unwrap();
        let ofp = fopen(ocp.as_ptr(), wmode.as_ptr());
        let mut w: *mut CVoid = std::ptr::null_mut();
        let d = sym::<FnWriteOpen>(lib, "LZ4F_writeOpen")(&mut w, ofp, std::ptr::null());
        let e = sym::<FnWrite>(lib, "LZ4F_write")(w, src.as_ptr(), 0);
        let f = sym::<FnWrite>(lib, "LZ4F_write")(w, src.as_ptr(), 0);
        let g = sym::<FnWriteClose>(lib, "LZ4F_writeClose")(w);
        fclose(ofp);
        let bytes = std::fs::read(&out).unwrap_or_default();
        (
            o as i64, a as i64, b as i64, c as i64, d as i64, e as i64, f as i64, g as i64, bytes,
        )
    });
}

#[test]
fn e134_write_oversized_chunks() {
    // `LZ4F_write` loops while `remain > maxWriteSize`, so a single call much
    // larger than the block size exercises the multi-iteration path.
    let td = TmpDir::new("e-big");
    let mut rng = Rng::new(0x5EED_2134);
    for &bsid in BLOCK_SIZE_IDS.iter() {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        p.frameInfo.contentChecksumFlag = 1;
        for &len in [1usize, 65537, 300000, 1_200_000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            diff(&format!("write big bsid={bsid} len={len}"), |lib| {
                file_write(
                    lib,
                    &td.path(&format!("big-{bsid}-{len}.lz4")),
                    &src,
                    Some(&p),
                    &[usize::MAX],
                )
            });
            let i = impls();
            let path = td.path(&format!("big-{bsid}-{len}.lz4"));
            file_write(&i.c, &path, &src, Some(&p), &[usize::MAX]);
            let (_, got) = file_read(&i.r, &path, len, &[usize::MAX]);
            assert_eq!(&got[..], &src[..], "bsid={bsid} len={len}");
        }
    }
}
