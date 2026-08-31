//! Phase B — CONFIGS.md section E: the low-level / "pure" exported entry
//! points, driven with many randomized inputs (fixed seed) through BOTH .so's.
mod common;
use common::*;
use std::ffi::CString;

const N: usize = 4000;

#[test]
fn int_accessors() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x1234_5678_9abc_def0);
    unsafe {
        // Exhaustive-ish over interesting byte patterns plus random.
        let mut cases: Vec<[u8; 4]> = vec![
            [0, 0, 0, 0],
            [0xff, 0xff, 0xff, 0xff],
            [0x80, 0, 0, 0],
            [0x7f, 0xff, 0xff, 0xff],
            [0x00, 0x00, 0x00, 0x01],
            [0x80, 0x00, 0x00, 0x01],
        ];
        for _ in 0..N {
            let b = rng.u32().to_be_bytes();
            cases.push(b);
        }
        for b in &cases {
            assert_eq!(
                (c.png_get_uint_32)(b.as_ptr()),
                (r.png_get_uint_32)(b.as_ptr()),
                "png_get_uint_32 {:02x?}",
                b
            );
            assert_eq!(
                (c.png_get_int_32)(b.as_ptr()),
                (r.png_get_int_32)(b.as_ptr()),
                "png_get_int_32 {:02x?}",
                b
            );
            assert_eq!(
                (c.png_get_uint_16)(b.as_ptr()),
                (r.png_get_uint_16)(b.as_ptr()),
                "png_get_uint_16 {:02x?}",
                b
            );
            // png_get_uint_31 with a NULL png_ptr is only safe for values that
            // do not trigger png_error; the error path is covered separately.
            if u32::from_be_bytes(*b) <= PNG_UINT_31_MAX {
                assert_eq!(
                    (c.png_get_uint_31)(std::ptr::null(), b.as_ptr()),
                    (r.png_get_uint_31)(std::ptr::null(), b.as_ptr()),
                    "png_get_uint_31 {:02x?}",
                    b
                );
            }
        }
        // savers
        for _ in 0..N {
            let v = rng.u32();
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            (c.png_save_uint_32)(cb.as_mut_ptr(), v);
            (r.png_save_uint_32)(rb.as_mut_ptr(), v);
            assert_eq!(cb, rb, "png_save_uint_32 {}", v);
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            (c.png_save_int_32)(cb.as_mut_ptr(), v as i32);
            (r.png_save_int_32)(rb.as_mut_ptr(), v as i32);
            assert_eq!(cb, rb, "png_save_int_32 {}", v as i32);
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            (c.png_save_uint_16)(cb.as_mut_ptr(), v);
            (r.png_save_uint_16)(rb.as_mut_ptr(), v);
            assert_eq!(cb, rb, "png_save_uint_16 {}", v);
        }
    }
}

#[test]
fn fixed_point_arithmetic() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0xdead_beef_cafe_0001);
    unsafe {
        let mut vals: Vec<i32> = vec![
            0,
            1,
            -1,
            PNG_FP_1,
            -PNG_FP_1,
            PNG_FP_HALF,
            PNG_FP_MAX,
            PNG_FP_MIN,
            i32::MIN,
            i32::MAX,
            65535,
            65536,
            100_001,
            99_999,
        ];
        for _ in 0..200 {
            vals.push(rng.u32() as i32);
            vals.push((rng.below(200_000) as i32) - 100_000);
        }

        // png_muldiv(res, a, times, divisor)
        for _ in 0..N {
            let a = vals[rng.below(vals.len() as u32) as usize];
            let t = vals[rng.below(vals.len() as u32) as usize];
            let d = vals[rng.below(vals.len() as u32) as usize];
            let mut cr: i32 = 0x5555_5555;
            let mut rr: i32 = 0x5555_5555;
            let co = (c.png_muldiv)(&mut cr, a, t, d);
            let ro = (r.png_muldiv)(&mut rr, a, t, d);
            assert_eq!(co, ro, "png_muldiv({},{},{}) status", a, t, d);
            if co != 0 {
                assert_eq!(cr, rr, "png_muldiv({},{},{}) result", a, t, d);
            }
        }
        // NOTE: a NULL `res` is *not* a valid input -- png.c writes `*res`
        // unconditionally when `divisor != 0`, so the C dereferences it.

        for &v in &vals {
            assert_eq!(
                (c.png_reciprocal)(v),
                (r.png_reciprocal)(v),
                "png_reciprocal({})",
                v
            );
            assert_eq!(
                (c.png_gamma_significant)(v),
                (r.png_gamma_significant)(v),
                "png_gamma_significant({})",
                v
            );
        }
        for _ in 0..N {
            let a = vals[rng.below(vals.len() as u32) as usize];
            let b = vals[rng.below(vals.len() as u32) as usize];
            assert_eq!(
                (c.png_reciprocal2)(a, b),
                (r.png_reciprocal2)(a, b),
                "png_reciprocal2({},{})",
                a,
                b
            );
        }
    }
}

#[test]
fn gamma_correction_tables() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x0bad_c0de_0000_0007);
    unsafe {
        let mut gammas: Vec<i32> = vec![
            0,
            1,
            PNG_FP_1,
            PNG_FP_HALF,
            45455,
            220_000,
            100_001,
            99_999,
            PNG_FP_MAX,
        ];
        for _ in 0..64 {
            gammas.push(rng.below(400_000) as i32);
        }
        for &g in &gammas {
            for v in 0u32..256 {
                assert_eq!(
                    (c.png_gamma_8bit_correct)(v, g),
                    (r.png_gamma_8bit_correct)(v, g),
                    "png_gamma_8bit_correct({},{})",
                    v,
                    g
                );
            }
            for _ in 0..256 {
                let v = rng.below(65536);
                assert_eq!(
                    (c.png_gamma_16bit_correct)(v, g),
                    (r.png_gamma_16bit_correct)(v, g),
                    "png_gamma_16bit_correct({},{})",
                    v,
                    g
                );
            }
            for v in [0u32, 1, 32767, 32768, 65534, 65535] {
                assert_eq!(
                    (c.png_gamma_16bit_correct)(v, g),
                    (r.png_gamma_16bit_correct)(v, g),
                    "png_gamma_16bit_correct({},{})",
                    v,
                    g
                );
            }
        }
    }
}

#[test]
fn colorspace_xy_xyz_roundtrip() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0xfeed_face_1234_0009);
    unsafe {
        // sRGB primaries plus randomized (including out of range) values.
        let srgb = png_xy {
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        };
        let mut cases = vec![srgb, png_xy::default()];
        for _ in 0..N {
            cases.push(png_xy {
                redx: rng.range(-100_000, 200_000) as i32,
                redy: rng.range(-100_000, 200_000) as i32,
                greenx: rng.range(-100_000, 200_000) as i32,
                greeny: rng.range(-100_000, 200_000) as i32,
                bluex: rng.range(-100_000, 200_000) as i32,
                bluey: rng.range(-100_000, 200_000) as i32,
                whitex: rng.range(-100_000, 200_000) as i32,
                whitey: rng.range(-100_000, 200_000) as i32,
            });
        }
        for xy in &cases {
            let mut cx = png_XYZ::default();
            let mut rx = png_XYZ::default();
            let co = (c.png_XYZ_from_xy)(&mut cx, xy);
            let ro = (r.png_XYZ_from_xy)(&mut rx, xy);
            assert_eq!(co, ro, "png_XYZ_from_xy status for {:?}", xy);
            assert_eq!(cx, rx, "png_XYZ_from_xy result for {:?}", xy);
            if co == 0 {
                let mut cy = png_xy::default();
                let mut ry = png_xy::default();
                let co2 = (c.png_xy_from_XYZ)(&mut cy, &cx);
                let ro2 = (r.png_xy_from_XYZ)(&mut ry, &rx);
                assert_eq!(co2, ro2, "png_xy_from_XYZ status");
                assert_eq!(cy, ry, "png_xy_from_XYZ result");
            }
        }
        // png_xy_from_XYZ directly on random XYZ
        for _ in 0..N {
            let xyz = png_XYZ {
                red_X: rng.u32() as i32 % 300_000,
                red_Y: rng.u32() as i32 % 300_000,
                red_Z: rng.u32() as i32 % 300_000,
                green_X: rng.u32() as i32 % 300_000,
                green_Y: rng.u32() as i32 % 300_000,
                green_Z: rng.u32() as i32 % 300_000,
                blue_X: rng.u32() as i32 % 300_000,
                blue_Y: rng.u32() as i32 % 300_000,
                blue_Z: rng.u32() as i32 % 300_000,
            };
            let mut cy = png_xy::default();
            let mut ry = png_xy::default();
            assert_eq!(
                (c.png_xy_from_XYZ)(&mut cy, &xyz),
                (r.png_xy_from_XYZ)(&mut ry, &xyz),
                "png_xy_from_XYZ status {:?}",
                xyz
            );
            assert_eq!(cy, ry, "png_xy_from_XYZ result {:?}", xyz);
        }
    }
}

#[test]
fn fp_string_checkers() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x00c0_ffee_0000_0011);
    unsafe {
        let mut strings: Vec<Vec<u8>> = vec![
            b"1".to_vec(),
            b"".to_vec(),
            b"-".to_vec(),
            b"+".to_vec(),
            b".".to_vec(),
            b"-.".to_vec(),
            b"1.0".to_vec(),
            b"1.0e10".to_vec(),
            b"1.0E10".to_vec(),
            b"1.0e+10".to_vec(),
            b"1.0e-10".to_vec(),
            b"1.0e".to_vec(),
            b"1.0e+".to_vec(),
            b".5".to_vec(),
            b"0.".to_vec(),
            b"00000".to_vec(),
            b"1e999999999999".to_vec(),
            b"1.0 ".to_vec(),
            b" 1.0".to_vec(),
            b"1,0".to_vec(),
            b"nan".to_vec(),
            b"inf".to_vec(),
            b"1.0\0trailing".to_vec(),
            b"-0.0000001".to_vec(),
            b"+1.5e-3".to_vec(),
        ];
        let alphabet = b"0123456789.eE+- \0x";
        for _ in 0..N {
            let n = rng.below(12) as usize;
            strings.push(
                (0..n)
                    .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
                    .collect(),
            );
        }
        for s in &strings {
            for &size in &[0usize, 1, s.len(), s.len() + 1] {
                if size > s.len() {
                    continue;
                }
                let mut cstate: c_int = 0;
                let mut rstate: c_int = 0;
                let mut cwhere: usize = 0;
                let mut rwhere: usize = 0;
                let co = (c.png_check_fp_number)(
                    s.as_ptr() as *const c_char,
                    size,
                    &mut cstate,
                    &mut cwhere,
                );
                let ro = (r.png_check_fp_number)(
                    s.as_ptr() as *const c_char,
                    size,
                    &mut rstate,
                    &mut rwhere,
                );
                assert_eq!(co, ro, "png_check_fp_number({:?},{})", s, size);
                assert_eq!(cstate, rstate, "state for ({:?},{})", s, size);
                assert_eq!(cwhere, rwhere, "where for ({:?},{})", s, size);
                assert_eq!(
                    (c.png_check_fp_string)(s.as_ptr() as *const c_char, size),
                    (r.png_check_fp_string)(s.as_ptr() as *const c_char, size),
                    "png_check_fp_string({:?},{})",
                    s,
                    size
                );
            }
        }
    }
}

#[test]
fn ascii_from_fp_and_fixed() {
    let mut rng = Rng::new(0xabcd_0000_0000_0021);
    unsafe {
        let mut fps: Vec<f64> = vec![
            0.0,
            1.0,
            -1.0,
            0.5,
            1e-10,
            1e10,
            1.0 / 3.0,
            123456.789,
            f64::MIN_POSITIVE,
            0.1,
            0.9999999999,
            1e-300,
            1e300,
        ];
        for _ in 0..500 {
            let m = rng.u32() as f64 / 4294967296.0;
            let e = (rng.range(-20, 20)) as i32;
            fps.push(m * 10f64.powi(e));
            fps.push(-m * 10f64.powi(e));
        }
        for &v in &fps {
            for prec in [1u32, 2, 3, 5, 6, 7, 8, 15, 17] {
                let size = 64usize;
                let mut cb = vec![0u8; size];
                let mut rb = vec![0u8; size];
                let cok = guard(|| {
                    let c = c_api();
                    set_current_api(c);
                    (c.png_ascii_from_fp)(
                        std::ptr::null(),
                        cb.as_mut_ptr() as png_charp,
                        size,
                        v,
                        prec,
                    )
                });
                let rok = guard(|| {
                    let r = rs_api();
                    set_current_api(r);
                    (r.png_ascii_from_fp)(
                        std::ptr::null(),
                        rb.as_mut_ptr() as png_charp,
                        size,
                        v,
                        prec,
                    )
                });
                assert_eq!(
                    cok.is_some(),
                    rok.is_some(),
                    "png_ascii_from_fp({},{}) error parity",
                    v,
                    prec
                );
                if cok.is_some() {
                    assert_eq!(cb, rb, "png_ascii_from_fp({},{})", v, prec);
                }
            }
        }
        let mut fixed: Vec<i32> = vec![0, 1, -1, PNG_FP_1, -PNG_FP_1, i32::MAX, i32::MIN, 99999];
        for _ in 0..1000 {
            fixed.push(rng.u32() as i32);
        }
        for &v in &fixed {
            let size = 64usize;
            let mut cb = vec![0u8; size];
            let mut rb = vec![0u8; size];
            let cok = guard(|| {
                let c = c_api();
                set_current_api(c);
                (c.png_ascii_from_fixed)(
                    std::ptr::null(),
                    cb.as_mut_ptr() as png_charp,
                    size,
                    v,
                )
            });
            let rok = guard(|| {
                let r = rs_api();
                set_current_api(r);
                (r.png_ascii_from_fixed)(
                    std::ptr::null(),
                    rb.as_mut_ptr() as png_charp,
                    size,
                    v,
                )
            });
            assert_eq!(cok.is_some(), rok.is_some(), "ascii_from_fixed({})", v);
            if cok.is_some() {
                assert_eq!(cb, rb, "png_ascii_from_fixed({})", v);
            }
        }
    }
}

#[test]
fn safecat_and_format_number() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x5555_0000_0000_0031);
    unsafe {
        for _ in 0..N {
            let size = 1 + rng.below(24) as usize;
            let pos = rng.below(size as u32 + 2) as usize;
            let n = rng.below(20) as usize;
            let s: Vec<u8> = (0..n).map(|_| b'a' + (rng.u8() % 26)).collect();
            let cstr = CString::new(s.clone()).unwrap();
            let mut cb = vec![0u8; size + 8];
            let mut rb = vec![0u8; size + 8];
            let cp = (c.png_safecat)(cb.as_mut_ptr() as png_charp, size, pos, cstr.as_ptr());
            let rp = (r.png_safecat)(rb.as_mut_ptr() as png_charp, size, pos, cstr.as_ptr());
            assert_eq!(cp, rp, "png_safecat({},{},{:?}) ret", size, pos, s);
            assert_eq!(cb, rb, "png_safecat({},{},{:?}) buf", size, pos, s);
        }
        for fmt in [
            PNG_NUMBER_FORMAT_u,
            PNG_NUMBER_FORMAT_02u,
            PNG_NUMBER_FORMAT_x,
            PNG_NUMBER_FORMAT_02x,
            PNG_NUMBER_FORMAT_fixed,
        ] {
            let mut nums: Vec<usize> = vec![0, 1, 9, 10, 99, 100, 99999, 100000, 100001];
            for _ in 0..500 {
                nums.push(rng.u32() as usize);
            }
            for &v in &nums {
                let mut cb = vec![0u8; PNG_NUMBER_BUFFER_SIZE];
                let mut rb = vec![0u8; PNG_NUMBER_BUFFER_SIZE];
                let cp = (c.png_format_number)(
                    cb.as_ptr() as png_const_charp,
                    cb.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE) as png_charp,
                    fmt,
                    v,
                );
                let rp = (r.png_format_number)(
                    rb.as_ptr() as png_const_charp,
                    rb.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE) as png_charp,
                    fmt,
                    v,
                );
                assert_eq!(cb, rb, "png_format_number(fmt={},{}) buf", fmt, v);
                let coff = cp as usize - cb.as_ptr() as usize;
                let roff = rp as usize - rb.as_ptr() as usize;
                assert_eq!(coff, roff, "png_format_number(fmt={},{}) ret", fmt, v);
            }
        }
    }
}

#[test]
fn sig_cmp_and_grayscale_palette() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x7777_0000_0000_0041);
    unsafe {
        let good: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
        let mut sigs: Vec<[u8; 8]> = vec![good, [0; 8], [137, 80, 78, 71, 13, 10, 26, 11]];
        for _ in 0..N {
            let mut s = good;
            let i = rng.below(8) as usize;
            s[i] = rng.u8();
            sigs.push(s);
            sigs.push(rng.bytes(8).try_into().unwrap());
        }
        for s in &sigs {
            for start in 0usize..9 {
                for num in 0usize..9 {
                    assert_eq!(
                        (c.png_sig_cmp)(s.as_ptr(), start, num),
                        (r.png_sig_cmp)(s.as_ptr(), start, num),
                        "png_sig_cmp({:02x?},{},{})",
                        s,
                        start,
                        num
                    );
                }
            }
        }
        for bd in [-1i32, 0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 32] {
            let n = 256usize;
            let mut cp = vec![png_color::default(); n];
            let mut rp = vec![png_color::default(); n];
            (c.png_build_grayscale_palette)(bd, cp.as_mut_ptr());
            (r.png_build_grayscale_palette)(bd, rp.as_mut_ptr());
            assert_eq!(cp, rp, "png_build_grayscale_palette({})", bd);
            // NULL palette must be a no-op in both
            (c.png_build_grayscale_palette)(bd, std::ptr::null_mut());
            (r.png_build_grayscale_palette)(bd, std::ptr::null_mut());
        }
    }
}

#[test]
fn time_conversions() {
    let c = c_api();
    let r = rs_api();
    let mut rng = Rng::new(0x9999_0000_0000_0051);
    unsafe {
        let mut times: Vec<png_time> = vec![
            png_time {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2024,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 60,
            },
            png_time::default(),
            png_time {
                year: 65535,
                month: 255,
                day: 255,
                hour: 255,
                minute: 255,
                second: 255,
            },
        ];
        for _ in 0..N {
            times.push(png_time {
                year: rng.u32() as u16,
                month: rng.u8(),
                day: rng.u8(),
                hour: rng.u8(),
                minute: rng.u8(),
                second: rng.u8(),
            });
        }
        for t in &times {
            let mut cb = [0i8; 30];
            let mut rb = [0i8; 30];
            let co = (c.png_convert_to_rfc1123_buffer)(cb.as_mut_ptr() as *mut c_char, t);
            let ro = (r.png_convert_to_rfc1123_buffer)(rb.as_mut_ptr() as *mut c_char, t);
            assert_eq!(co, ro, "rfc1123_buffer status {:?}", t);
            if co != 0 {
                assert_eq!(cb, rb, "rfc1123_buffer {:?}", t);
            }
        }
        // png_convert_from_time_t / png_convert_from_struct_tm
        let mut tts: Vec<i64> = vec![0, 1, 1_000_000_000, 2_000_000_000, 951_782_400];
        for _ in 0..1000 {
            tts.push((rng.u32() as i64) % 4_000_000_000);
        }
        for &tt in &tts {
            let mut ct = png_time::default();
            let mut rt = png_time::default();
            (c.png_convert_from_time_t)(&mut ct, tt);
            (r.png_convert_from_time_t)(&mut rt, tt);
            assert_eq!(ct, rt, "png_convert_from_time_t({})", tt);
        }
        for _ in 0..1000 {
            let tm_ = tm {
                tm_sec: rng.below(70) as i32,
                tm_min: rng.below(70) as i32,
                tm_hour: rng.below(30) as i32,
                tm_mday: rng.below(40) as i32,
                tm_mon: rng.below(15) as i32,
                tm_year: rng.below(300) as i32,
                tm_wday: 0,
                tm_yday: 0,
                tm_isdst: 0,
                tm_gmtoff: 0,
                tm_zone: std::ptr::null(),
            };
            let mut ct = png_time::default();
            let mut rt = png_time::default();
            (c.png_convert_from_struct_tm)(&mut ct, &tm_);
            (r.png_convert_from_struct_tm)(&mut rt, &tm_);
            assert_eq!(ct, rt, "png_convert_from_struct_tm");
        }
    }
}

/// png_convert_to_rfc1123 needs a png_struct (it uses png_ptr->time_buffer).
#[test]
fn convert_to_rfc1123_with_struct() {
    let mut rng = Rng::new(0xaaaa_0000_0000_0061);
    let mut times: Vec<png_time> = Vec::new();
    for _ in 0..500 {
        times.push(png_time {
            year: rng.u32() as u16,
            month: rng.u8(),
            day: rng.u8(),
            hour: rng.u8(),
            minute: rng.u8(),
            second: rng.u8(),
        });
    }
    times.push(png_time {
        year: 2000,
        month: 2,
        day: 29,
        hour: 12,
        minute: 30,
        second: 15,
    });
    unsafe {
        let mut out: Vec<Vec<(Option<String>, Diag)>> = Vec::new();
        for api in both() {
            let s = ReadSess::new(api, &[]);
            let mut v = Vec::new();
            for t in &times {
                diag_reset();
                let got = guard(|| rs_str((api.png_convert_to_rfc1123)(s.png, t)));
                v.push((got.flatten(), diag_take()));
            }
            // NOTE: a NULL `ptime` is *not* a valid input -- the C dereferences
            // it before any check (png.c, png_convert_to_rfc1123_buffer).
            out.push(v);
        }
        assert_eq!(out[0], out[1], "png_convert_to_rfc1123");
    }
}
