//! Phase B — CONFIGS.md section E (continued): the exported row-level
//! primitives, called directly on both `.so`s over the full cross-product of
//! bit depth / colour type / width, with randomized row contents.
mod common;
use common::*;

/// A row_info + row buffer pair, identical for both libraries.
fn make_row(rng: &mut Rng, color_type: c_int, bit_depth: c_int, width: u32) -> (png_row_info, Vec<u8>) {
    let channels = channels_of(color_type);
    let pixel_depth = channels * bit_depth as u32;
    let rb = rowbytes(pixel_depth, width);
    let ri = png_row_info {
        width,
        rowbytes: rb,
        color_type: color_type as u8,
        bit_depth: bit_depth as u8,
        channels: channels as u8,
        pixel_depth: pixel_depth as u8,
    };
    (ri, rng.bytes(rb + 16))
}

const WIDTHS: [u32; 10] = [1, 2, 3, 4, 5, 7, 8, 9, 16, 33];

#[test]
fn do_bgr_swap_invert_packswap() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x0102_0304_0506_0701);
    unsafe {
        // Sweep every legal combination *and* the illegal ones the functions
        // are documented to ignore (they all guard on row_info fields).
        for ct in [0i32, 1, 2, 3, 4, 5, 6, 7] {
            for bd in [1i32, 2, 4, 8, 16] {
                for &w in &WIDTHS {
                    for rep in 0..3 {
                        let (ri0, buf0) = make_row(&mut rng, ct, bd, w);
                        for which in 0..4 {
                            let mut cri = ri0;
                            let mut rri = ri0;
                            let mut cb = buf0.clone();
                            let mut rb = buf0.clone();
                            match which {
                                0 => {
                                    (c.png_do_bgr)(&mut cri, cb.as_mut_ptr());
                                    (r.png_do_bgr)(&mut rri, rb.as_mut_ptr());
                                }
                                1 => {
                                    (c.png_do_swap)(&mut cri, cb.as_mut_ptr());
                                    (r.png_do_swap)(&mut rri, rb.as_mut_ptr());
                                }
                                2 => {
                                    (c.png_do_invert)(&mut cri, cb.as_mut_ptr());
                                    (r.png_do_invert)(&mut rri, rb.as_mut_ptr());
                                }
                                _ => {
                                    (c.png_do_packswap)(&mut cri, cb.as_mut_ptr());
                                    (r.png_do_packswap)(&mut rri, rb.as_mut_ptr());
                                }
                            }
                            assert_eq!(
                                cri, rri,
                                "row_info after op {} ct={} bd={} w={} rep={}",
                                which, ct, bd, w, rep
                            );
                            assert_bytes_eq(
                                &format!("op {} ct={} bd={} w={} rep={}", which, ct, bd, w, rep),
                                &cb,
                                &rb,
                            );
                        }
                        // NOTE: png_do_bgr/_swap/_invert/_packswap have no NULL
                        // guard in the C -- they dereference row_info and row
                        // unconditionally, so NULL is not a testable input.
                    }
                }
            }
        }
    }
}

#[test]
fn do_strip_channel() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x0a0b_0c0d_0e0f_0011);
    unsafe {
        for ct in [0i32, 2, 4, 6] {
            for bd in [1i32, 2, 4, 8, 16] {
                for &w in &WIDTHS {
                    for at_start in [0i32, 1, -1, 7] {
                        let (ri0, buf0) = make_row(&mut rng, ct, bd, w);
                        let mut cri = ri0;
                        let mut rri = ri0;
                        let mut cb = buf0.clone();
                        let mut rb = buf0.clone();
                        (c.png_do_strip_channel)(&mut cri, cb.as_mut_ptr(), at_start);
                        (r.png_do_strip_channel)(&mut rri, rb.as_mut_ptr(), at_start);
                        assert_eq!(
                            cri, rri,
                            "strip_channel row_info ct={} bd={} w={} at_start={}",
                            ct, bd, w, at_start
                        );
                        assert_bytes_eq(
                            &format!(
                                "strip_channel ct={} bd={} w={} at_start={}",
                                ct, bd, w, at_start
                            ),
                            &cb,
                            &rb,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn do_read_interlace() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x1111_2222_3333_4441);
    unsafe {
        for ct in [0i32, 2, 3, 4, 6] {
            for bd in [1i32, 2, 4, 8, 16] {
                if ct == 3 && bd == 16 {
                    continue;
                }
                let channels = channels_of(ct);
                let pd = channels * bd as u32;
                for pass in 0..7i32 {
                    for &w in &WIDTHS {
                        for &transformations in &[0u32, 0x0010_0000u32, 0xffff_ffff] {
                            let final_width = w * PNG_PASS_INC[pass as usize];
                            let need = rowbytes(pd, final_width) + 32;
                            let src = rng.bytes(need);
                            let ri0 = png_row_info {
                                width: w,
                                rowbytes: rowbytes(pd, w),
                                color_type: ct as u8,
                                bit_depth: bd as u8,
                                channels: channels as u8,
                                pixel_depth: pd as u8,
                            };
                            let mut cri = ri0;
                            let mut rri = ri0;
                            let mut cb = src.clone();
                            let mut rb = src.clone();
                            (c.png_do_read_interlace)(
                                &mut cri,
                                cb.as_mut_ptr(),
                                pass,
                                transformations,
                            );
                            (r.png_do_read_interlace)(
                                &mut rri,
                                rb.as_mut_ptr(),
                                pass,
                                transformations,
                            );
                            assert_eq!(
                                cri, rri,
                                "read_interlace row_info ct={} bd={} pass={} w={} t={:#x}",
                                ct, bd, pass, w, transformations
                            );
                            assert_bytes_eq(
                                &format!(
                                    "read_interlace ct={} bd={} pass={} w={} t={:#x}",
                                    ct, bd, pass, w, transformations
                                ),
                                &cb,
                                &rb,
                            );
                        }
                    }
                }
                // NULL guards
                let mut cri = png_row_info::default();
                let mut rri = png_row_info::default();
                (c.png_do_read_interlace)(&mut cri, std::ptr::null_mut(), 0, 0);
                (r.png_do_read_interlace)(&mut rri, std::ptr::null_mut(), 0, 0);
                assert_eq!(cri, rri);
                let mut b = [0u8; 8];
                (c.png_do_read_interlace)(std::ptr::null_mut(), b.as_mut_ptr(), 0, 0);
                (r.png_do_read_interlace)(std::ptr::null_mut(), b.as_mut_ptr(), 0, 0);
            }
        }
    }
}

#[test]
fn do_write_interlace() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x2222_3333_4444_5551);
    unsafe {
        for ct in [0i32, 2, 3, 4, 6] {
            for bd in [1i32, 2, 4, 8, 16] {
                if ct == 3 && bd == 16 {
                    continue;
                }
                let channels = channels_of(ct);
                let pd = channels * bd as u32;
                for pass in 0..7i32 {
                    for &w in &[1u32, 2, 3, 4, 5, 7, 8, 9, 16, 17, 33, 64, 65] {
                        let need = rowbytes(pd, w) + 32;
                        let src = rng.bytes(need);
                        let ri0 = png_row_info {
                            width: w,
                            rowbytes: rowbytes(pd, w),
                            color_type: ct as u8,
                            bit_depth: bd as u8,
                            channels: channels as u8,
                            pixel_depth: pd as u8,
                        };
                        let mut cri = ri0;
                        let mut rri = ri0;
                        let mut cb = src.clone();
                        let mut rb = src.clone();
                        (c.png_do_write_interlace)(&mut cri, cb.as_mut_ptr(), pass);
                        (r.png_do_write_interlace)(&mut rri, rb.as_mut_ptr(), pass);
                        assert_eq!(
                            cri, rri,
                            "write_interlace row_info ct={} bd={} pass={} w={}",
                            ct, bd, pass, w
                        );
                        assert_bytes_eq(
                            &format!("write_interlace ct={} bd={} pass={} w={}", ct, bd, pass, w),
                            &cb,
                            &rb,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn read_filter_row_all_filters() {
    let mut rng = Rng::new(0x3333_4444_5555_6661);
    unsafe {
        // Only the (colour type, bit depth) pairs the PNG format allows: for an
        // illegal pair such as RGB@4 the pixel depth is 12 and the C
        // PNG_ROWBYTES macro yields rowbytes < bpp, which makes
        // png_read_filter_row_avg's `rowbytes - bpp` underflow (C UB).
        for (ct, bd) in legal_ihdr() {
            {
                let channels = channels_of(ct);
                let pd = channels * bd as u32;
                for &w in &[1u32, 2, 3, 5, 8, 17, 64] {
                    let rb_len = rowbytes(pd, w);
                    let row0 = rng.bytes(rb_len + 8);
                    let prev = rng.bytes(rb_len + 8);
                    // filter values 0..=4 are valid; 5.. hits the default branch
                    for filter in 0..7i32 {
                        let mut outs: Vec<(Vec<u8>, png_row_info, Diag, bool)> = Vec::new();
                        for api in both() {
                            let s = ReadSess::new(api, &[]);
                            let mut ri = png_row_info {
                                width: w,
                                rowbytes: rb_len,
                                color_type: ct as u8,
                                bit_depth: bd as u8,
                                channels: channels as u8,
                                pixel_depth: pd as u8,
                            };
                            let mut buf = row0.clone();
                            diag_reset();
                            let ok = guard(|| {
                                (api.png_read_filter_row)(
                                    s.png,
                                    &mut ri,
                                    buf.as_mut_ptr(),
                                    prev.as_ptr(),
                                    filter,
                                )
                            });
                            outs.push((buf, ri, diag_take(), ok.is_some()));
                        }
                        assert_eq!(
                            outs[0].1, outs[1].1,
                            "read_filter_row row_info ct={} bd={} w={} f={}",
                            ct, bd, w, filter
                        );
                        assert_eq!(
                            outs[0].2, outs[1].2,
                            "read_filter_row diag ct={} bd={} w={} f={}",
                            ct, bd, w, filter
                        );
                        assert_eq!(outs[0].3, outs[1].3, "read_filter_row error parity");
                        assert_bytes_eq(
                            &format!("read_filter_row ct={} bd={} w={} f={}", ct, bd, w, filter),
                            &outs[0].0,
                            &outs[1].0,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn check_keyword() {
    let mut rng = Rng::new(0x4444_5555_6666_7771);
    unsafe {
        let mut keys: Vec<Vec<u8>> = vec![
            b"Title".to_vec(),
            b"".to_vec(),
            b" leading".to_vec(),
            b"trailing ".to_vec(),
            b"double  space".to_vec(),
            b"tab\there".to_vec(),
            b"\x01control".to_vec(),
            b"\x7fdel".to_vec(),
            b"\xa0nbsp".to_vec(),
            b"\xff high".to_vec(),
            (0..79).map(|_| b'k').collect(),
            (0..80).map(|_| b'k').collect(),
            (0..200).map(|_| b'k').collect(),
            b"   ".to_vec(),
            b"a b".to_vec(),
        ];
        for _ in 0..800 {
            let n = rng.below(100) as usize;
            keys.push(
                (0..n)
                    .map(|_| {
                        let x = rng.u8();
                        if x == 0 {
                            b'x'
                        } else {
                            x
                        }
                    })
                    .collect(),
            );
        }
        for k in &keys {
            let ck = std::ffi::CString::new(k.clone()).unwrap();
            let mut outs: Vec<(u32, Vec<u8>, Diag, bool)> = Vec::new();
            for api in both() {
                let s = WriteSess::new(api);
                let mut newk = vec![0u8; 90];
                diag_reset();
                let n = guard(|| (api.png_check_keyword)(s.png, ck.as_ptr(), newk.as_mut_ptr()));
                outs.push((n.unwrap_or(u32::MAX), newk, diag_take(), n.is_some()));
            }
            assert_eq!(outs[0].0, outs[1].0, "check_keyword len for {:?}", k);
            assert_eq!(outs[0].1, outs[1].1, "check_keyword out for {:?}", k);
            assert_eq!(outs[0].2, outs[1].2, "check_keyword diag for {:?}", k);
            assert_eq!(outs[0].3, outs[1].3, "check_keyword parity for {:?}", k);
        }
    }
}
