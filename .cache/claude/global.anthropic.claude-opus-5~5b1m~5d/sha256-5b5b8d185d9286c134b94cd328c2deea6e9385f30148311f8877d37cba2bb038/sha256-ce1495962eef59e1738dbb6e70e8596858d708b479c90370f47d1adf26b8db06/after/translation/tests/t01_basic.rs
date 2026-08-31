//! Phase B foundation: version/error/bound helpers and the one-shot
//! compress+decompress round trip across every input shape and length edge.
//!
//! Everything here goes through `dlopen`'d exports of BOTH libraries.

mod common;
use common::*;

type Fn_u32 = unsafe extern "C" fn() -> u32;
type Fn_cstr = unsafe extern "C" fn() -> *const std::os::raw::c_char;
type Fn_sz_sz = unsafe extern "C" fn(usize) -> usize;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_getErrorName = unsafe extern "C" fn(usize) -> *const std::os::raw::c_char;
type Fn_getErrorCode = unsafe extern "C" fn(usize) -> i32;
type Fn_i_v = unsafe extern "C" fn() -> i32;

type Fn_compress =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> usize;
type Fn_decompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type Fn_getFCS = unsafe extern "C" fn(*const u8, usize) -> u64;

fn cstr(p: *const std::os::raw::c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn version_and_identity() {
    let i = impls();

    let (c, r) = i.pair::<Fn_u32>("ZSTD_versionNumber");
    unsafe { assert_eq_dbg("ZSTD_versionNumber", c(), r()) };

    let (c, r) = i.pair::<Fn_cstr>("ZSTD_versionString");
    unsafe { assert_eq_dbg("ZSTD_versionString", cstr(c()), cstr(r())) };

    for name in ["ZSTD_minCLevel", "ZSTD_maxCLevel", "ZSTD_defaultCLevel"] {
        let (c, r) = i.pair::<Fn_i_v>(name);
        unsafe { assert_eq_dbg(name, c(), r()) };
    }
}

/// `ZSTD_compressBound` / `ZSTD_decompressBound` must agree exactly, including
/// at the overflow guard (ZSTD_MAX_INPUT_SIZE).
#[test]
fn compress_bound_matches() {
    let i = impls();
    let (c, r) = i.pair::<Fn_sz_sz>("ZSTD_compressBound");

    let mut sizes: Vec<usize> = EDGE_LENS.to_vec();
    sizes.extend([
        1 << 20,
        1 << 24,
        (1 << 30) + 1,
        usize::MAX / 8,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ]);
    let mut rng = Rng::new(0xB0110);
    for _ in 0..200 {
        sizes.push(rng.next_u64() as usize);
    }

    for s in sizes {
        unsafe { assert_eq_dbg(&format!("ZSTD_compressBound({s})"), c(s), r(s)) };
    }
}

/// The whole error-reporting surface: every `size_t` in a wide sweep is asked
/// whether it is an error, what its name is and what its code is. This pins the
/// error-code mapping used by every Phase C assertion.
#[test]
fn error_surface_matches() {
    let i = impls();
    let (c_is, r_is) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_nm, r_nm) = i.pair::<Fn_getErrorName>("ZSTD_getErrorName");
    let (c_cd, r_cd) = i.pair::<Fn_getErrorCode>("ZSTD_getErrorCode");
    let (c_sn, r_sn) = i.pair::<unsafe extern "C" fn(i32) -> *const std::os::raw::c_char>(
        "ZSTD_getErrorString",
    );

    // every plausible negated error code, plus valid-looking sizes
    let mut probes: Vec<usize> = Vec::new();
    for e in 0..=200usize {
        probes.push(0usize.wrapping_sub(e)); // -0 .. -200
    }
    probes.extend([0, 1, 2, 100, 1 << 20, usize::MAX / 2]);

    for p in probes {
        let tag = format!("code {p:#x}");
        unsafe {
            assert_eq_dbg(&format!("ZSTD_isError({tag})"), c_is(p), r_is(p));
            assert_eq_dbg(&format!("ZSTD_getErrorName({tag})"), cstr(c_nm(p)), cstr(r_nm(p)));
            assert_eq_dbg(&format!("ZSTD_getErrorCode({tag})"), c_cd(p), r_cd(p));
        }
    }

    // ZSTD_getErrorString over the enum range *and* out-of-range enum values —
    // C enums accept any int, so these are real inputs.
    for e in -5i32..=130 {
        unsafe {
            assert_eq_dbg(
                &format!("ZSTD_getErrorString({e})"),
                cstr(c_sn(e)),
                cstr(r_sn(e)),
            );
        }
    }
}

/// The core happy path: one-shot compress at every level over every shape and
/// every length edge, asserting the *compressed bytes* are identical (not just
/// that both round-trip) and that each library can decode the other's frame.
#[test]
fn oneshot_roundtrip_all_shapes_levels() {
    let i = impls();
    let (c_comp, r_comp) = i.pair::<Fn_compress>("ZSTD_compress");
    let (c_dec, r_dec) = i.pair::<Fn_decompress>("ZSTD_decompress");
    let (c_fcs, r_fcs) = i.pair::<Fn_getFCS>("ZSTD_getFrameContentSize");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("ZSTD_compressBound");

    let mut rng = Rng::new(0xC0FFEE);
    // a representative level sweep incl. negative/fast levels, 0 (=default) and max
    let levels = [-131_072, -1000, -50, -5, -1, 0, 1, 2, 3, 5, 9, 12, 17, 19, 22];

    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 7, 64, 1000, 5000, 70_000, 131_072, 131_073, 300_000] {
            let src = gen_shape(shape, len, &mut rng);
            for &lvl in &levels {
                let cap = unsafe { c_bound(len) };
                let mut cbuf = vec![0xAAu8; cap];
                let mut rbuf = vec![0x55u8; cap];

                let cn = unsafe { c_comp(cbuf.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
                let rn = unsafe { r_comp(rbuf.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };

                let tag = format!("compress shape={shape:?} len={len} lvl={lvl}");
                assert_eq_dbg(&tag, cn, rn);
                assert!(unsafe { !c_is_err(i, cn) }, "{tag}: C errored {cn:#x}");
                assert_bytes_eq(&tag, &cbuf[..cn], &rbuf[..rn]);

                // frame content size agreement
                unsafe {
                    assert_eq_dbg(
                        &format!("{tag} / getFrameContentSize"),
                        c_fcs(cbuf.as_ptr(), cn),
                        r_fcs(rbuf.as_ptr(), rn),
                    );
                }

                // cross-decode: C frame -> Rust decoder and vice versa
                let mut d1 = vec![0u8; len + 1];
                let mut d2 = vec![0u8; len + 1];
                let n1 = unsafe { r_dec(d1.as_mut_ptr(), len + 1, cbuf.as_ptr(), cn) };
                let n2 = unsafe { c_dec(d2.as_mut_ptr(), len + 1, rbuf.as_ptr(), rn) };
                assert_eq_dbg(&format!("{tag} / rust decodes C frame"), n1, len);
                assert_eq_dbg(&format!("{tag} / C decodes rust frame"), n2, len);
                assert_bytes_eq(&format!("{tag} / plaintext"), &src, &d1[..n1]);
                assert_bytes_eq(&format!("{tag} / plaintext"), &src, &d2[..n2]);
            }
        }
    }
}

unsafe fn c_is_err(i: &Impls, code: usize) -> bool {
    let (c, _) = i.pair::<Fn_isError>("ZSTD_isError");
    c(code) != 0
}

/// Decompression of randomized *valid* frames must agree, and both libraries
/// must report the same result for every truncation of a valid frame — this
/// exercises the incomplete-input paths of the frame/block decoder.
#[test]
fn decompress_truncated_frames_agree() {
    let i = impls();
    let (c_comp, _) = i.pair::<Fn_compress>("ZSTD_compress");
    let (c_dec, r_dec) = i.pair::<Fn_decompress>("ZSTD_decompress");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("ZSTD_compressBound");

    let mut rng = Rng::new(0x7A0D);
    for &shape in &ALL_SHAPES {
        let len = rng.range(200, 4000);
        let src = gen_shape(shape, len, &mut rng);
        let cap = unsafe { c_bound(len) };
        let mut frame = vec![0u8; cap];
        let n = unsafe { c_comp(frame.as_mut_ptr(), cap, src.as_ptr(), len, 3) };

        for cut in 0..=n {
            let mut d1 = vec![0u8; len + 16];
            let mut d2 = vec![0u8; len + 16];
            let a = unsafe { c_dec(d1.as_mut_ptr(), d1.len(), frame.as_ptr(), cut) };
            let b = unsafe { r_dec(d2.as_mut_ptr(), d2.len(), frame.as_ptr(), cut) };
            assert_eq_dbg(
                &format!("truncated decompress shape={shape:?} cut={cut}/{n}"),
                a,
                b,
            );
            if a == len {
                assert_bytes_eq("truncated-but-complete payload", &d1[..a], &d2[..b]);
            }
        }
    }
}

/// Bit-flip corruption sweep: both libraries must reject (or accept) each
/// corrupted frame identically, with the same error code.
#[test]
fn decompress_corrupted_frames_agree() {
    let i = impls();
    let (c_comp, _) = i.pair::<Fn_compress>("ZSTD_compress");
    let (c_dec, r_dec) = i.pair::<Fn_decompress>("ZSTD_decompress");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("ZSTD_compressBound");
    let (c_cd, r_cd) = i.pair::<Fn_getErrorCode>("ZSTD_getErrorCode");

    let mut rng = Rng::new(0xDEAD_BEEF);
    for &shape in &ALL_SHAPES {
        for &lvl in &[1i32, 3, 9, 19] {
            let len = rng.range(500, 3000);
            let src = gen_shape(shape, len, &mut rng);
            let cap = unsafe { c_bound(len) };
            let mut base = vec![0u8; cap];
            let n = unsafe { c_comp(base.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };

            for _ in 0..120 {
                let mut f = base[..n].to_vec();
                let pos = rng.below(n);
                f[pos] ^= 1u8 << rng.below(8);

                let mut d1 = vec![0u8; len + 64];
                let mut d2 = vec![0u8; len + 64];
                let a = unsafe { c_dec(d1.as_mut_ptr(), d1.len(), f.as_ptr(), f.len()) };
                let b = unsafe { r_dec(d2.as_mut_ptr(), d2.len(), f.as_ptr(), f.len()) };
                let tag = format!("corrupt shape={shape:?} lvl={lvl} pos={pos}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} errcode"), c_cd(a), r_cd(b)) };
                if a <= len {
                    assert_bytes_eq(&format!("{tag} payload"), &d1[..a], &d2[..b]);
                }
            }
        }
    }
}
