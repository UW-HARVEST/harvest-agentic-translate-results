//! Harness self-test / non-vacuity check.
//!
//! A differential suite is worthless if its comparisons cannot fail.  This file
//! proves that (a) the data actually being compared is substantial and
//! non-trivial, and (b) every comparison primitive the suite relies on really
//! does fire on a difference.
mod common;
use common::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn must_panic<F: FnOnce()>(what: &str, f: F) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    assert!(r.is_err(), "{}: comparison did NOT fail on a difference", what);
}

#[test]
fn comparison_primitives_detect_differences() {
    // assert_bytes_eq
    must_panic("assert_bytes_eq (one byte)", || {
        assert_bytes_eq("x", &[1, 2, 3, 4], &[1, 2, 9, 4])
    });
    must_panic("assert_bytes_eq (length)", || {
        assert_bytes_eq("x", &[1, 2, 3], &[1, 2, 3, 4])
    });
    // Diag equality
    let a = Diag {
        warnings: vec!["w".into()],
        errors: vec![],
    };
    let b = Diag {
        warnings: vec!["W".into()],
        errors: vec![],
    };
    assert_ne!(a, b, "Diag must compare message text");
    let c = Diag {
        warnings: vec![],
        errors: vec!["w".into()],
    };
    assert_ne!(a, c, "Diag must distinguish warnings from errors");
    // png_row_info equality
    let mut r1 = png_row_info::default();
    let mut r2 = png_row_info::default();
    assert_eq!(r1, r2);
    r2.pixel_depth = 1;
    assert_ne!(r1, r2, "png_row_info must compare every field");
    r1.pixel_depth = 1;
    r2.rowbytes = 1;
    assert_ne!(r1, r2);
}

/// The write path must really be producing a substantial PNG for every format,
/// and the reader must really be returning the pixels (not zeroes).
#[test]
fn compared_data_is_non_trivial() {
    let mut rng = Rng::new(0xdead_c0de_0f0f_0001);
    let mut total_written = 0usize;
    let mut total_read = 0usize;
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            unsafe {
                // --- write ---
                let api = c_api();
                set_current_api(api);
                diag_reset();
                let mut sess = WriteSess::new(api);
                let (png, info) = (sess.png, sess.info);
                let (w, h) = (23u32, 11u32);
                let pd = channels_of(ct) * bd as u32;
                let rows: Vec<Vec<u8>> =
                    (0..h).map(|_| rng.bytes(rowbytes(pd, w))).collect();
                let npal = if ct == PNG_COLOR_TYPE_PALETTE {
                    1usize << bd
                } else {
                    0
                };
                let palette: Vec<png_color> = (0..npal)
                    .map(|_| png_color {
                        red: rng.u8(),
                        green: rng.u8(),
                        blue: rng.u8(),
                    })
                    .collect();
                let ok = guard(|| {
                    (api.png_set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                    if !palette.is_empty() {
                        (api.png_set_PLTE)(
                            png,
                            info,
                            palette.as_ptr(),
                            palette.len() as c_int,
                        );
                    }
                    (api.png_write_info)(png, info);
                    let mut rp: Vec<png_bytep> =
                        rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
                    (api.png_write_image)(png, rp.as_mut_ptr());
                    (api.png_write_end)(png, info);
                })
                .is_some();
                assert!(ok, "ct={} bd={} il={}: write failed", ct, bd, il);
                let bytes = std::mem::take(&mut sess.sink.buf);
                let _ = diag_take();
                drop(sess);
                assert!(
                    bytes.len() > 60,
                    "ct={} bd={} il={}: PNG is implausibly small ({} bytes)",
                    ct,
                    bd,
                    il,
                    bytes.len()
                );
                assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
                assert_eq!(&bytes[bytes.len() - 4..], b"\xaeB`\x82", "IEND CRC");
                total_written += bytes.len();

                // --- read it back and check the pixels really came through ---
                for api in both() {
                    set_current_api(api);
                    diag_reset();
                    let s = ReadSess::new(api, &bytes);
                    let mut got: Vec<Vec<u8>> = Vec::new();
                    let ok = guard(|| {
                        (api.png_read_info)(s.png, s.info);
                        let rbz = (api.png_get_rowbytes)(s.png, s.info);
                        let hh = (api.png_get_image_height)(s.png, s.info);
                        let mut buf: Vec<Vec<u8>> =
                            (0..hh).map(|_| vec![0u8; rbz]).collect();
                        let mut ptrs: Vec<png_bytep> =
                            buf.iter_mut().map(|r| r.as_mut_ptr()).collect();
                        (api.png_read_image)(s.png, ptrs.as_mut_ptr());
                        (api.png_read_end)(s.png, s.end);
                        got = buf;
                    })
                    .is_some();
                    let _ = diag_take();
                    assert!(ok, "{} ct={} bd={}: read failed", api.name, ct, bd);
                    assert_eq!(got.len(), h as usize);
                    // The decoded rows must equal the rows that were written,
                    // masking the padding bits of a partial final byte.
                    let m = ((pd as u64 * w as u64) & 7) as u32;
                    for (y, row) in got.iter().enumerate() {
                        let mut want = rows[y].clone();
                        let mut have = row.clone();
                        if m != 0 {
                            let last = ((pd as u64 * w as u64) / 8) as usize;
                            let keep = !(0xffu8 >> m);
                            want[last] &= keep;
                            have[last] &= keep;
                        }
                        assert_eq!(
                            want, have,
                            "{} ct={} bd={} il={}: row {} did not round-trip",
                            api.name, ct, bd, il, y
                        );
                    }
                    let nonzero: usize = got.iter().flatten().filter(|&&b| b != 0).count();
                    assert!(
                        nonzero * 4 > got.iter().map(|r| r.len()).sum::<usize>(),
                        "{} ct={} bd={}: decoded image is mostly zeroes",
                        api.name,
                        ct,
                        bd
                    );
                    total_read += got.iter().map(|r| r.len()).sum::<usize>();
                }
            }
        }
    }
    assert!(total_written > 20_000, "only {} bytes written", total_written);
    assert!(total_read > 30_000, "only {} bytes read", total_read);
    eprintln!(
        "non-vacuity: {} PNG bytes produced, {} decoded pixel bytes compared",
        total_written, total_read
    );
}

/// The two libraries really are two DIFFERENT shared objects.
#[test]
fn the_two_libraries_are_distinct() {
    let c = c_api();
    let r = rs_api();
    assert_ne!(
        c.png_write_row as usize, r.png_write_row as usize,
        "both handles resolved to the same code!"
    );
    assert_ne!(c.png_read_row as usize, r.png_read_row as usize);
    assert_ne!(c.png_set_IHDR as usize, r.png_set_IHDR as usize);
    // ... and they agree on the public version, so the comparison is fair.
    unsafe {
        assert_eq!(
            (c.png_access_version_number)(),
            (r.png_access_version_number)()
        );
    }
}
