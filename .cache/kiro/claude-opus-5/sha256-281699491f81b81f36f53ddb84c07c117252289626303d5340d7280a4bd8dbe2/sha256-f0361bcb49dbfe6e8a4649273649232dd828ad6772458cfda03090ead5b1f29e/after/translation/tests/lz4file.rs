//! Phase B — CONFIGS.md group 11: `lz4file.c`. Drives the FILE*-based API on
//! real temporary files, comparing the produced bytes and the read-back payload.
mod common;
use common::frame::*;
use common::*;
use std::ffi::CString;

const SEED: u64 = 0x46_494C_4500_0011;

fn is_err(v: usize) -> bool {
    v > (0usize.wrapping_sub(24))
}

type FnFopen = unsafe extern "C" fn(*const i8, *const i8) -> *mut u8;
type FnFclose = unsafe extern "C" fn(*mut u8) -> i32;
type FnWriteOpen = unsafe extern "C" fn(*mut *mut u8, *mut u8, *const Prefs) -> usize;
type FnWrite = unsafe extern "C" fn(*mut u8, *const u8, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut u8) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut u8, *mut u8) -> usize;
type FnRead = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut u8) -> usize;

/// libc `fopen`/`fclose` come from the process, not from either lz4 library.
fn libc_fns() -> (FnFopen, FnFclose) {
    // The test binary is dynamically linked against libc, so these resolve in
    // the global scope.
    let lib = unsafe { libloading::os::unix::Library::this() };
    let fopen: libloading::os::unix::Symbol<FnFopen> =
        unsafe { lib.get(b"fopen\0") }.expect("fopen");
    let fclose: libloading::os::unix::Symbol<FnFclose> =
        unsafe { lib.get(b"fclose\0") }.expect("fclose");
    (*fopen, *fclose)
}

fn tmp_path(tag: &str) -> String {
    format!(
        "{}/lz4file_diff_{}_{}.lz4",
        std::env::temp_dir().display(),
        std::process::id(),
        tag
    )
}

/// Write `src` through `LZ4F_write*` using `lib` (0 = C, 1 = Rust) and return
/// the resulting file bytes plus every API return value observed.
fn write_file(
    which: usize,
    path: &str,
    src: &[u8],
    p: *const Prefs,
    chunks: &[usize],
) -> (Vec<usize>, Vec<u8>) {
    let (fopen, fclose) = libc_fns();
    let (cwo, rwo) = syms::<FnWriteOpen>("LZ4F_writeOpen");
    let (cw, rw) = syms::<FnWrite>("LZ4F_write");
    let (cwc, rwc) = syms::<FnWriteClose>("LZ4F_writeClose");
    let (wo, w, wc) = if which == 0 { (cwo, cw, cwc) } else { (rwo, rw, rwc) };

    let cpath = CString::new(path).unwrap();
    let mode = CString::new("wb").unwrap();
    let fp = unsafe { fopen(cpath.as_ptr(), mode.as_ptr()) };
    assert!(!fp.is_null(), "fopen({path}) failed");

    let mut rets = Vec::new();
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let r = unsafe { wo(&mut ctx, fp, p) };
    rets.push(r);
    if !is_err(r) {
        let mut off = 0usize;
        let mut i = 0usize;
        while off < src.len() {
            let n = chunks[i % chunks.len()].min(src.len() - off);
            let n = if n == 0 { 1 } else { n };
            let r = unsafe { w(ctx, src.as_ptr().add(off), n) };
            rets.push(r);
            if is_err(r) {
                break;
            }
            off += n;
            i += 1;
        }
        // a zero-length write must be a no-op returning 0
        rets.push(unsafe { w(ctx, src.as_ptr(), 0) });
        rets.push(unsafe { wc(ctx) });
    }
    unsafe { fclose(fp) };
    let bytes = std::fs::read(path).unwrap_or_default();
    (rets, bytes)
}

/// Read a file back through `LZ4F_read*`.
fn read_file(which: usize, path: &str, chunk: usize, expect_len: usize) -> (Vec<usize>, Vec<u8>) {
    let (fopen, fclose) = libc_fns();
    let (cro, rro) = syms::<FnReadOpen>("LZ4F_readOpen");
    let (cr, rr) = syms::<FnRead>("LZ4F_read");
    let (crc, rrc) = syms::<FnReadClose>("LZ4F_readClose");
    let (ro, rd, rc) = if which == 0 { (cro, cr, crc) } else { (rro, rr, rrc) };

    let cpath = CString::new(path).unwrap();
    let mode = CString::new("rb").unwrap();
    let fp = unsafe { fopen(cpath.as_ptr(), mode.as_ptr()) };
    assert!(!fp.is_null(), "fopen({path}) for read failed");

    let mut rets = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let r = unsafe { ro(&mut ctx, fp) };
    rets.push(r);
    if !is_err(r) {
        // zero-length read must return 0 without touching the stream
        let mut scratch = vec![0u8; chunk.max(1)];
        rets.push(unsafe { rd(ctx, scratch.as_mut_ptr(), 0) });
        let mut guard = 0usize;
        loop {
            guard += 1;
            assert!(guard < 4_000_000, "read did not terminate");
            let n = unsafe { rd(ctx, scratch.as_mut_ptr(), chunk.max(1)) };
            rets.push(n);
            if is_err(n) || n == 0 {
                break;
            }
            out.extend_from_slice(&scratch[..n]);
            if out.len() > expect_len + 1024 {
                break;
            }
        }
        rets.push(unsafe { rc(ctx) });
    }
    unsafe { fclose(fp) };
    (rets, out)
}


/// `LZ4F_readOpen` performs a single `fread` of the FULL `LZ4F_HEADER_SIZE_MAX`
/// (19) bytes and fails with `LZ4F_ERROR_io_read` if the file is shorter
/// (lz4file.c:92-97). Frames for tiny payloads are shorter than that, so the
/// read chain legitimately errors out; the payload can only be compared to the
/// original when no call in the chain reported an error.
fn chain_ok(rets: &[usize]) -> bool {
    !rets.iter().any(|&v| is_err(v))
}

/// Rows 188-199: write and read across the preference matrix, write-chunk
/// shapes and read-chunk shapes; then cross-library round-trips.
#[test]
fn g11_write_read_matrix() {
    let mut rng = Rng::new(SEED);
    let cpath = tmp_path("c");
    let rpath = tmp_path("r");

    for (desc, p) in prefs_matrix_small().iter() {
        for &len in &[0usize, 1, 100, 65535, 65536, 65537, 200_000] {
            let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
            // write-chunk shapes: whole, 1 byte, exactly a block, block+-1, mixed
            let bs = match p.frame_info.block_size_id {
                5 => 256 * 1024,
                6 => 1024 * 1024,
                7 => 4 * 1024 * 1024,
                _ => 64 * 1024,
            };
            let chunk_sets: Vec<Vec<usize>> = vec![
                vec![len.max(1)],
                vec![1],
                vec![bs],
                vec![bs - 1],
                vec![bs + 1],
                vec![7, 100, 4096, 70000],
            ];
            for (ci, chunks) in chunk_sets.iter().enumerate() {
                if ci == 1 && len > 65537 {
                    continue; // 1-byte writes over 200 KB are slow and redundant
                }
                let (cr, cb) = write_file(0, &cpath, &src, p, chunks);
                let (rr, rb) = write_file(1, &rpath, &src, p, chunks);
                let lbl = format!("writeOpen/write/writeClose [{desc}] len={len} chunks={ci}");
                assert_eq!(cr, rr, "{lbl}: return codes C={cr:x?} R={rr:x?}");
                assert_eq!(cb.len(), rb.len(), "{lbl}: file length differs");
                assert_eq!(cb, rb, "{lbl}: file bytes differ");

                // read the C-produced file with both, and the Rust one with both
                for &read_chunk in &[1usize, 100, bs, bs + 1, len.max(1)] {
                    if read_chunk == 1 && len > 65537 {
                        continue;
                    }
                    for (src_tag, fpath) in [("Cfile", &cpath), ("Rfile", &rpath)] {
                        let (cret, cout) = read_file(0, fpath, read_chunk, len);
                        let (rret, rout) = read_file(1, fpath, read_chunk, len);
                        let l2 = format!("{lbl} read({src_tag}, chunk={read_chunk})");
                        assert_eq!(cret, rret, "{l2}: return codes C={cret:x?} R={rret:x?}");
                        assert_eq!(cout, rout, "{l2}: payload differs");
                        if chain_ok(&cret) {
                            assert_eq!(cout, src, "{l2}: payload != original");
                        }
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&rpath);
}

/// Row 192: compression levels through the file API.
#[test]
fn g11_levels_and_null_prefs() {
    let mut rng = Rng::new(SEED ^ 1);
    let cpath = tmp_path("lc");
    let rpath = tmp_path("lr");
    for &lvl in &[-5i32, -1, 0, 1, 2, 3, 9, 10, 12, 13, 100] {
        for &declared in &[false, true] {
            for &len in &[0usize, 1, 1000, 200_000] {
                let mut p = Prefs::default();
                p.compression_level = lvl;
                p.frame_info.content_checksum_flag = 1;
                p.frame_info.block_checksum_flag = 1;
                if declared {
                    p.frame_info.content_size = len as u64;
                }
                let src = mkdata(Shape::Textish, len, &mut rng);
                let (cr, cb) = write_file(0, &cpath, &src, &p, &[4096]);
                let (rr, rb) = write_file(1, &rpath, &src, &p, &[4096]);
                let lbl = format!("file lvl={lvl} declared={declared} len={len}");
                assert_eq!(cr, rr, "{lbl}: write returns");
                assert_eq!(cb, rb, "{lbl}: file bytes");
                let (cret, cout) = read_file(0, &cpath, 4096, len);
                let (rret, rout) = read_file(1, &cpath, 4096, len);
                assert_eq!(cret, rret, "{lbl}: read returns");
                assert_eq!(cout, rout, "{lbl}: read payload");
                if chain_ok(&cret) {
                    assert_eq!(cout, src, "{lbl}: payload != original");
                }
            }
        }
    }
    // Row 188: prefs == NULL
    for &len in &[0usize, 1, 1000, 200_000] {
        let src = mkdata(Shape::Textish, len, &mut rng);
        let (cr, cb) = write_file(0, &cpath, &src, std::ptr::null(), &[100]);
        let (rr, rb) = write_file(1, &rpath, &src, std::ptr::null(), &[100]);
        assert_eq!(cr, rr, "file NULL prefs len={len}: write returns");
        assert_eq!(cb, rb, "file NULL prefs len={len}: bytes");
        let (cret, cout) = read_file(0, &cpath, 777, len);
        let (rret, rout) = read_file(1, &cpath, 777, len);
        assert_eq!(cret, rret);
        assert_eq!(cout, rout);
        if chain_ok(&cret) {
            assert_eq!(cout, src);
        }
    }
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&rpath);
}
