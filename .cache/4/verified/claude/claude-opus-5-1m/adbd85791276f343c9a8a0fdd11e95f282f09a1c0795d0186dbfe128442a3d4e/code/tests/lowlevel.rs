//! Phase C — the lowest-level exported entry points: the internal
//! (`pngpriv.h`) functions libpng exports but that are *not* part of `png.h`.
//!
//! Covers CONFIGS.md rows **C-1 … C-6, C-8 … C-20, C-22 and C-24**.
//! The remaining rows of that block are covered elsewhere:
//! C-7 by `transforms::gamma_tables`, C-21 by `chunks::iccp`,
//! C-23 by `misc::memory` and C-25 by `smoke::version_numbers_match`.
//!
//! Most of these functions take all of their state as arguments and are called
//! directly here.  A few (`png_write_find_filter`, `png_combine_row`,
//! `png_do_check_palette_indexes`, `png_reset_crc`/`png_calculate_crc`) read or
//! write `png_struct` fields that cannot be reached from outside the library;
//! for those the row is driven through the public entry point that reaches them
//! and the comment on the test says so.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* ------------------------------------------------------------------ */
/* shared fixtures                                                     */
/* ------------------------------------------------------------------ */

/// `PNG_PACKSWAP` from pngpriv.h (the only `transformations` bit
/// `png_do_read_interlace` looks at).
const PNG_PACKSWAP: u32 = 0x10000;

/// The pixel depths that can actually occur in a PNG row.  Nothing else is
/// legal: `png_read_filter_row_avg` computes `rowbytes - bpp` as a `size_t`, so
/// e.g. pixel_depth 9 (rowbytes 1, bpp 2) would underflow inside libpng itself.
const PIXEL_DEPTHS: [u8; 9] = [1, 2, 4, 8, 16, 24, 32, 48, 64];

const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

/// A `png_row_info` for a legal (colour type, bit depth) pair.
fn ri(width: u32, ct: c_int, bd: c_int) -> png_row_info {
    let ch = channels_of(ct) as u8;
    let pd = (bd as u8).wrapping_mul(ch);
    png_row_info {
        width,
        rowbytes: png_rowbytes(pd as usize, width as usize),
        color_type: ct as u8,
        bit_depth: bd as u8,
        channels: ch,
        pixel_depth: pd,
    }
}

/// A `png_row_info` described only by its pixel depth (what the filter and
/// interlace code actually looks at).
fn ri_pd(width: u32, pd: u8) -> png_row_info {
    png_row_info {
        width,
        rowbytes: png_rowbytes(pd as usize, width as usize),
        color_type: 0,
        bit_depth: if pd >= 8 { 8 } else { pd },
        channels: if pd >= 8 { pd / 8 } else { 1 },
        pixel_depth: pd,
    }
}

fn show_ri(r: &png_row_info) -> String {
    format!(
        "w={} rb={} ct={} bd={} ch={} pd={}",
        r.width, r.rowbytes, r.color_type, r.bit_depth, r.channels, r.pixel_depth
    )
}

/// Write `img` with the C library only, so that both libraries can then be
/// pointed at the very same bytes.
fn build_with_c(img: &Img, opts: &WriteOpts) -> Vec<u8> {
    let l = libs();
    unsafe {
        let mut state = Box::new(Tls::default());
        let prev = set_tls(&mut *state as *mut Tls);
        let prev_api = set_cur_api(&l.c as *const Api);
        let wr = write_plain(&l.c, img, opts);
        assert_eq!(wr.guard, Guard::Ok, "reference write failed");
        set_cur_api(prev_api);
        set_tls(prev);
        wr.bytes
    }
}

/* ================================================================== */
/* C-1  png_get_uint_32 / _16 / _int_32 / _uint_31, png_save_*         */
/* ================================================================== */

#[test]
fn byte_accessors() {
    const EDGE: [u32; 14] = [
        0,
        1,
        2,
        0x7f,
        0x80,
        0xff,
        0x0100,
        0x7fff,
        0x8000,
        0xffff,
        0x7fff_ffff,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
    ];

    assert_same("C-1 byte accessors", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC001);

        // 16-byte buffers: a 4-byte access at offset 0 is always in bounds and
        // the untouched tail is compared too.
        let mut inputs: Vec<[u8; 16]> = Vec::new();
        for v in EDGE {
            let mut b = [0xa5u8; 16];
            b[..4].copy_from_slice(&v.to_be_bytes());
            inputs.push(b);
        }
        for _ in 0..1200 {
            let mut b = [0xa5u8; 16];
            for i in 0..8 {
                b[i] = rng.u8();
            }
            inputs.push(b);
        }
        // ... and a few with every byte 0x00 / 0xff / 0x80.
        for f in [0x00u8, 0xff, 0x80, 0x7f] {
            inputs.push([f; 16]);
        }

        for b in &inputs {
            // Every offset 0..8: libpng assembles these byte by byte, so an
            // unaligned access must give exactly the same answer.
            for off in 0..8usize {
                let p = b.as_ptr().add(off);
                o.push(format!(
                    "get+{} {:02x?} u32={} u16={} i32={}",
                    off,
                    &b[off..off + 4],
                    (api.png_get_uint_32)(p),
                    (api.png_get_uint_16)(p),
                    (api.png_get_int_32)(p)
                ));
            }
        }

        // png_save_* into a pattern-filled buffer: every byte is compared, so a
        // stray write past the 4 (or 2) bytes it should touch shows up.
        for b in inputs.iter().take(400) {
            let v = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
            let mut buf = [0x5au8; 16];
            (api.png_save_uint_32)(buf.as_mut_ptr().add(4), v);
            o.push(format!("save_uint_32({:#010x}) {:02x?}", v, buf));
            let mut buf = [0x5au8; 16];
            (api.png_save_int_32)(buf.as_mut_ptr().add(4), v as i32);
            o.push(format!("save_int_32({}) {:02x?}", v as i32, buf));
            let mut buf = [0x5au8; 16];
            (api.png_save_uint_16)(buf.as_mut_ptr().add(4), (v & 0xffff) as c_uint);
            o.push(format!("save_uint_16({}) {:02x?}", v & 0xffff, buf));
            // png_save_uint_16 is also called with values that do not fit.
            let mut buf = [0x5au8; 16];
            (api.png_save_uint_16)(buf.as_mut_ptr().add(4), v as c_uint);
            o.push(format!("save_uint_16_wide({}) {:02x?}", v, buf));
        }

        // png_get_uint_31: png_errors when the value exceeds PNG_UINT_31_MAX,
        // so it gets a fresh png_struct per call.
        for (bi, b) in inputs.iter().take(320).enumerate() {
            let off = bi % 5;
            let p = b.as_ptr().add(off);
            let (png, info) = new_read(api);
            let mut got = 0u32;
            let g = guarded(api, png, &mut || {
                got = (api.png_get_uint_31)(png, p);
            });
            o.push(format!(
                "get_uint_31+{} {:02x?} guard={:?} v={}",
                off,
                &b[off..off + 4],
                g,
                got
            ));
            destroy_read(api, png, info);
        }
        // ... and with a NULL png_ptr, which is legal as long as the value is
        // in range (otherwise png_error would have nowhere to jump to).
        for v in EDGE.iter().filter(|v| **v <= PNG_UINT_31_MAX) {
            let b = v.to_be_bytes();
            o.push(format!(
                "get_uint_31(NULL,{:#010x})={}",
                v,
                (api.png_get_uint_31)(core::ptr::null_mut(), b.as_ptr())
            ));
        }
        o
    });
}

/* ================================================================== */
/* C-2  png_sig_cmp                                                    */
/* ================================================================== */

#[test]
fn sig_cmp() {
    assert_same("C-2 png_sig_cmp", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC002);
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        bufs.push(SIG.to_vec());
        // one bit / byte wrong in each position
        for k in 0..8 {
            let mut v = SIG.to_vec();
            v[k] ^= 0xff;
            bufs.push(v);
            let mut v = SIG.to_vec();
            v[k] ^= 0x01;
            bufs.push(v);
            let mut v = SIG.to_vec();
            v[k] = 0;
            bufs.push(v);
            let mut v = SIG.to_vec();
            v[k] = 0xff;
            bufs.push(v);
        }
        // prefixes of the signature, the rest zeroed
        for k in 0..=8 {
            let mut v = vec![0u8; 8];
            v[..k].copy_from_slice(&SIG[..k]);
            bufs.push(v);
        }
        for _ in 0..40 {
            bufs.push(rng.bytes(8));
        }

        for b in &bufs {
            // Only sig[start..8] is ever touched (start > 7 returns -1 at
            // once), so an 8-byte buffer is always big enough.
            for start in 0..9usize {
                for n in 0..10usize {
                    o.push(format!(
                        "sig_cmp({:02x?},{},{})={}",
                        b,
                        start,
                        n,
                        (api.png_sig_cmp)(b.as_ptr(), start, n)
                    ));
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-3  png_muldiv                                                     */
/* ================================================================== */

/// `png_muldiv_warn` does not exist in this libpng (grep `c_src` finds no
/// definition and it is not an exported symbol), so this row covers
/// `png_muldiv` only.
#[test]
fn muldiv() {
    const EDGE: [i32; 20] = [
        0,
        1,
        -1,
        2,
        -2,
        10,
        100,
        100000,
        -100000,
        50000,
        0x7fff,
        -0x8000,
        0x10000,
        65535,
        65536,
        1 << 30,
        -(1 << 30),
        i32::MAX,
        i32::MIN + 1,
        i32::MIN,
    ];

    assert_same("C-3 png_muldiv", |api| unsafe {
        let mut o = Outcome::default();
        let mut cases: Vec<(i32, i32, i32)> = Vec::new();
        for a in EDGE {
            for t in EDGE {
                for d in [0i32, 1, -1, 2, 100000, -100000, i32::MAX, i32::MIN] {
                    cases.push((a, t, d));
                }
            }
        }
        let mut rng = Rng::new(0xC003);
        for _ in 0..3000 {
            // A mix of small and full-range values so that both the exact and
            // the overflowing product are hit often.
            let pick = |r: &mut Rng| -> i32 {
                match r.below(5) {
                    0 => r.range(-10, 11) as i32,
                    1 => r.range(-200000, 200001) as i32,
                    2 => r.u32() as i32,
                    3 => (r.u32() >> 1) as i32,
                    _ => rng_fixed(r),
                }
            };
            let a = pick(&mut rng);
            let t = pick(&mut rng);
            let d = if rng.below(20) == 0 { 0 } else { pick(&mut rng) };
            cases.push((a, t, d));
        }
        // Triples engineered so that a*times/divisor lands right on the
        // +-2^31 clamp and on the .5 rounding boundary.
        for d in [1i32, 2, 3, 7, 10, 100, 1000, 100000] {
            for k in -6i64..=6 {
                let target = 2_147_483_647i64 + k;
                let a = (target / d as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                cases.push((a, d, 1));
                cases.push((target.clamp(i32::MIN as i64, i32::MAX as i64) as i32, 1, d));
                cases.push((-target.clamp(i32::MIN as i64, i32::MAX as i64) as i32, 1, d));
                cases.push((a, -d, 1));
            }
            // exact halves: a*times = divisor*n + divisor/2
            for n in 0i64..8 {
                let num = d as i64 * n + d as i64 / 2;
                cases.push((num as i32, 1, d));
                cases.push((-(num as i32), 1, d));
            }
        }

        for (a, t, d) in cases {
            // The sentinel shows whether *res was written on the failure path.
            let mut res: i32 = 0x5a5a_5a5a;
            let r = (api.png_muldiv)(&mut res, a, t, d);
            o.push(format!("muldiv({},{},{})={} res={}", a, t, d, r, res));
        }
        o
    });
}

fn rng_fixed(r: &mut Rng) -> i32 {
    let v = (r.u32() % 200_001) as i32 - 100_000;
    v
}

/* ================================================================== */
/* C-4  png_reciprocal, png_reciprocal2                                */
/* ================================================================== */

#[test]
fn reciprocal() {
    const EDGE: [i32; 18] = [
        0,
        1,
        -1,
        2,
        -2,
        5,
        100,
        -100,
        50000,
        100000,
        -100000,
        100001,
        99999,
        1 << 20,
        i32::MAX,
        i32::MIN + 1,
        i32::MIN,
        21474,
    ];

    assert_same("C-4 png_reciprocal", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC004);
        let mut vals: Vec<i32> = EDGE.to_vec();
        for _ in 0..1500 {
            vals.push(match rng.below(4) {
                0 => rng.range(-20, 21) as i32,
                1 => rng_fixed(&mut rng),
                2 => rng.u32() as i32,
                _ => (rng.u32() >> 8) as i32,
            });
        }
        for a in &vals {
            o.push(format!("reciprocal({})={}", a, (api.png_reciprocal)(*a)));
        }
        for a in EDGE {
            for b in EDGE {
                o.push(format!(
                    "reciprocal2({},{})={}",
                    a,
                    b,
                    (api.png_reciprocal2)(a, b)
                ));
            }
        }
        for _ in 0..2000 {
            let a = vals[rng.below(vals.len())];
            let b = vals[rng.below(vals.len())];
            o.push(format!(
                "reciprocal2({},{})={}",
                a,
                b,
                (api.png_reciprocal2)(a, b)
            ));
        }
        // 1e10/a and 1e15/(a*b) right on the +-2^31 clamp, and on the .5
        // rounding boundary of floor(x+.5).
        for k in -4i64..=4 {
            for a in [4i32, 5, 6, 465661, 465662, 466033, 2, 3, 21474, 21475] {
                o.push(format!(
                    "reciprocal({})={}",
                    a + k as i32,
                    (api.png_reciprocal)(a + k as i32)
                ));
                for b in [1i32, 2, 100000, 465661, 21475] {
                    o.push(format!(
                        "reciprocal2({},{})={}",
                        a + k as i32,
                        b,
                        (api.png_reciprocal2)(a + k as i32, b)
                    ));
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-5  png_fixed, png_fixed_ITU                                       */
/* ================================================================== */

#[test]
fn fixed() {
    // png_fixed accepts floor(1e5*fp+.5) in [-2147483648,2147483647];
    // png_fixed_ITU accepts floor(1e4*fp+.5) in [0,2147483647].
    let mut doubles: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1e-5,
        -1e-5,
        4.9999e-6,
        5.0001e-6,
        2.2,
        0.45455,
        21474.83647,
        21474.83648,
        -21474.83648,
        -21474.83649,
        214748.3647,
        214748.36475,
        -214748.36485,
        1e10,
        -1e10,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        1.0 / 3.0,
        -1.0 / 3.0,
        123.456789,
        99999.999995,
    ];
    let mut rng = Rng::new(0xC005);
    for _ in 0..300 {
        let m = (rng.u32() as f64) / (u32::MAX as f64);
        let e = rng.range(-8, 9) as i32;
        let s = if rng.bool() { 1.0 } else { -1.0 };
        doubles.push(s * m * 10f64.powi(e));
    }
    // straddle the two limits closely
    for k in -8i32..9 {
        doubles.push(21474.83647 + (k as f64) * 1e-5);
        doubles.push(214748.3647 + (k as f64) * 1e-4);
    }

    let long_name: String = "n".repeat(250);
    let texts: Vec<Option<String>> = vec![
        Some("gamma".to_string()),
        Some(String::new()),
        Some(long_name),
        None,
    ];

    assert_same("C-5 png_fixed / png_fixed_ITU", |api| unsafe {
        let mut o = Outcome::default();
        for (ti, t) in texts.iter().enumerate() {
            let cs_t = t.as_ref().map(|s| cs(s));
            let tp = cs_t
                .as_ref()
                .map(|c| c.as_ptr())
                .unwrap_or(core::ptr::null());
            for (di, d) in doubles.iter().enumerate() {
                // Only vary `text` for a handful of values -- the message is
                // the same for every double.
                if ti > 0 && di % 37 != 0 {
                    continue;
                }
                let (png, info) = new_read(api);
                let mut got: i32 = 0x5a5a_5a5a;
                let g = guarded(api, png, &mut || {
                    got = (api.png_fixed)(png, *d, tp);
                });
                o.push(format!(
                    "png_fixed({:e},text#{}) guard={:?} v={}",
                    d, ti, g, got
                ));
                destroy_read(api, png, info);

                let (png, info) = new_read(api);
                let mut got: u32 = 0x5a5a_5a5a;
                let g = guarded(api, png, &mut || {
                    got = (api.png_fixed_ITU)(png, *d, tp);
                });
                o.push(format!(
                    "png_fixed_ITU({:e},text#{}) guard={:?} v={}",
                    d, ti, g, got
                ));
                destroy_read(api, png, info);
            }
        }
        o
    });
}

/* ================================================================== */
/* C-6  png_gamma_significant / _8bit_correct / _16bit_correct /        */
/*      png_gamma_correct                                              */
/* ================================================================== */

#[test]
fn gamma_scalar() {
    // Gammas are only ever non-negative in libpng; a negative gamma makes
    // pow() produce a double outside the range of the result type and the C
    // cast is then undefined, so those are excluded from the *_correct calls
    // (png_gamma_significant, a pure comparison, is fed them anyway).
    let gammas: Vec<i32> = {
        let mut v = vec![
            0,
            1,
            2,
            4999,
            5000,
            5001,
            50000,
            94999,
            95000,
            95001,
            99999,
            100000,
            100001,
            104999,
            105000,
            105001,
            45455,
            220000,
            1000000,
            PNG_FP_MAX,
        ];
        let mut rng = Rng::new(0xC006);
        for _ in 0..40 {
            v.push((rng.u32() >> 1) as i32);
            v.push(rng.range(0, 300001) as i32);
        }
        v
    };
    let signif: Vec<i32> = {
        let mut v = gammas.clone();
        v.extend_from_slice(&[-1, -5000, -100000, i32::MIN, i32::MIN + 1, -95000]);
        v
    };

    assert_same("C-6 gamma scalars", |api| unsafe {
        let mut o = Outcome::default();

        for g in &signif {
            o.push(format!(
                "gamma_significant({})={}",
                g,
                (api.png_gamma_significant)(*g)
            ));
        }

        // Every 8-bit value for every gamma, one trace line per gamma.
        for g in &gammas {
            let v: Vec<u8> = (0u32..256)
                .map(|x| (api.png_gamma_8bit_correct)(x as c_uint, *g))
                .collect();
            o.push(format!("8bit gamma={} -> {:?}", g, v));
        }

        // 16-bit: the boundaries plus 2048 random values, per gamma.
        let mut rng = Rng::new(0xC006_16);
        let mut vals: Vec<u32> = vec![0, 1, 2, 255, 256, 32767, 32768, 65533, 65534, 65535];
        for _ in 0..2048 {
            vals.push(rng.u32() & 0xffff);
        }
        for g in &gammas {
            let v: Vec<u16> = vals
                .iter()
                .map(|x| (api.png_gamma_16bit_correct)(*x as c_uint, *g))
                .collect();
            o.push(format!("16bit gamma={} -> {:?}", g, v));
        }

        // png_gamma_correct dispatches on png_ptr->bit_depth, which is set by
        // png_handle_IHDR, so drive it with real 8- and 16-bit read structs
        // (and with a fresh one, whose bit_depth is still 0).
        for depth in [0u8, 8, 16] {
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            let g0 = if depth == 0 {
                Guard::Ok
            } else {
                let file = gray_file(depth as c_int);
                tls().input = file;
                tls().in_pos = 0;
                guarded(api, png, &mut || {
                    (api.png_read_info)(png, info);
                })
            };
            o.push(format!("gamma_correct setup depth={} guard={:?}", depth, g0));
            for g in gammas.iter().take(20) {
                let v: Vec<u16> = vals
                    .iter()
                    .take(80)
                    .map(|x| (api.png_gamma_correct)(png, *x as c_uint, *g))
                    .collect();
                o.push(format!("gamma_correct depth={} g={} -> {:?}", depth, g, v));
            }
            destroy_read(api, png, info);
        }
        o
    });
}

/// A tiny valid gray PNG of the requested bit depth, built with the C library.
fn gray_file(bit_depth: c_int) -> Vec<u8> {
    let mut rng = Rng::new(0x9a11 ^ bit_depth as u64);
    let img = Img::random(&mut rng, 4, 3, PNG_COLOR_TYPE_GRAY, bit_depth);
    build_with_c(&img, &WriteOpts::default())
}

/* ================================================================== */
/* C-8  png_XYZ_from_xy, png_xy_from_XYZ                               */
/* ================================================================== */

#[test]
fn xyz_xy() {
    const FP: [i32; 16] = [
        0,
        1,
        -1,
        5,
        4,
        6400,
        15000,
        30000,
        31270,
        32900,
        60000,
        100000,
        110000,
        110001,
        PNG_FP_MAX,
        i32::MIN,
    ];

    assert_same("C-8 XYZ<->xy", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC008);

        let mut xys: Vec<png_xy> = Vec::new();
        // sRGB primaries
        xys.push(png_xy {
            redx: 64000,
            redy: 33000,
            greenx: 30000,
            greeny: 60000,
            bluex: 15000,
            bluey: 6000,
            whitex: 31270,
            whitey: 32900,
        });
        // degenerate: everything equal
        for v in [0, 1, 5, 33333, 100000, 110000] {
            xys.push(png_xy {
                redx: v,
                redy: v,
                greenx: v,
                greeny: v,
                bluex: v,
                bluey: v,
                whitex: v,
                whitey: v,
            });
        }
        // one field at a time pushed to an interesting value
        for k in 0..8 {
            for v in FP {
                let mut xy = png_xy {
                    redx: 64000,
                    redy: 33000,
                    greenx: 30000,
                    greeny: 60000,
                    bluex: 15000,
                    bluey: 6000,
                    whitex: 31270,
                    whitey: 32900,
                };
                let f: &mut i32 = match k {
                    0 => &mut xy.redx,
                    1 => &mut xy.redy,
                    2 => &mut xy.greenx,
                    3 => &mut xy.greeny,
                    4 => &mut xy.bluex,
                    5 => &mut xy.bluey,
                    6 => &mut xy.whitex,
                    _ => &mut xy.whitey,
                };
                *f = v;
                xys.push(xy);
            }
        }
        for _ in 0..1500 {
            let p = |r: &mut Rng| -> i32 {
                match r.below(4) {
                    0 => r.pick(&FP),
                    1 => r.range(0, 100001) as i32,
                    2 => r.range(-10000, 130000) as i32,
                    _ => r.u32() as i32,
                }
            };
            xys.push(png_xy {
                redx: p(&mut rng),
                redy: p(&mut rng),
                greenx: p(&mut rng),
                greeny: p(&mut rng),
                bluex: p(&mut rng),
                bluey: p(&mut rng),
                whitex: p(&mut rng),
                whitey: p(&mut rng),
            });
        }
        // The eight range checks at the top of png_XYZ_from_xy reject most
        // random input before any arithmetic runs, so also draw chromaticities
        // that are guaranteed to pass them and therefore exercise the whole
        // body (the png_muldiv / png_reciprocal chain and its overflow exits).
        const LIMIT: i64 = 110_000;
        for _ in 0..2500 {
            let pair = |r: &mut Rng, ymin: i64| -> (i32, i32) {
                let x = r.range(0, LIMIT + 1);
                let y = r.range(ymin, (LIMIT - x + 1).max(ymin + 1));
                (x as i32, y as i32)
            };
            let (rx, ry) = pair(&mut rng, 0);
            let (gx, gy) = pair(&mut rng, 0);
            let (bx, by) = pair(&mut rng, 0);
            let (wx, wy) = pair(&mut rng, 5);
            xys.push(png_xy {
                redx: rx,
                redy: ry,
                greenx: gx,
                greeny: gy,
                bluex: bx,
                bluey: by,
                whitex: wx,
                whitey: wy,
            });
        }
        // ... and right on each of the eight limits.
        for k in -2i32..=2 {
            let l = LIMIT as i32;
            xys.push(png_xy {
                redx: l + k,
                redy: 0,
                greenx: l + k,
                greeny: 0,
                bluex: l + k,
                bluey: 0,
                whitex: l + k,
                whitey: 5,
            });
            xys.push(png_xy {
                redx: 1000,
                redy: l - 1000 + k,
                greenx: 1000,
                greeny: l - 1000 + k,
                bluex: 1000,
                bluey: l - 1000 + k,
                whitex: 1000,
                whitey: l - 1000 + k,
            });
            xys.push(png_xy {
                redx: 30000,
                redy: 30000,
                greenx: 30000,
                greeny: 30000,
                bluex: 30000,
                bluey: 30000,
                whitex: 30000,
                whitey: 3 + k,
            });
        }

        for xy in &xys {
            // The sentinel makes every partially-written field visible.
            let mut XYZ = png_XYZ {
                red_X: 0x5a5a_5a5a,
                red_Y: 0x5a5a_5a5a,
                red_Z: 0x5a5a_5a5a,
                green_X: 0x5a5a_5a5a,
                green_Y: 0x5a5a_5a5a,
                green_Z: 0x5a5a_5a5a,
                blue_X: 0x5a5a_5a5a,
                blue_Y: 0x5a5a_5a5a,
                blue_Z: 0x5a5a_5a5a,
            };
            let r = (api.png_XYZ_from_xy)(&mut XYZ, xy);
            o.push(format!("XYZ_from_xy({:?})={} {:?}", xy, r, XYZ));

            // and round-trip whatever came out
            let mut back = png_xy {
                redx: 0x5a5a_5a5a,
                redy: 0x5a5a_5a5a,
                greenx: 0x5a5a_5a5a,
                greeny: 0x5a5a_5a5a,
                bluex: 0x5a5a_5a5a,
                bluey: 0x5a5a_5a5a,
                whitex: 0x5a5a_5a5a,
                whitey: 0x5a5a_5a5a,
            };
            let r2 = (api.png_xy_from_XYZ)(&mut back, &XYZ);
            o.push(format!("  xy_from_XYZ={} {:?}", r2, back));
        }

        // png_xy_from_XYZ over independently chosen XYZ values.
        let mut xyzs: Vec<png_XYZ> = Vec::new();
        for v in [0i32, 1, -1, 100000, PNG_FP_MAX, i32::MIN, 33333] {
            xyzs.push(png_XYZ {
                red_X: v,
                red_Y: v,
                red_Z: v,
                green_X: v,
                green_Y: v,
                green_Z: v,
                blue_X: v,
                blue_Y: v,
                blue_Z: v,
            });
        }
        for _ in 0..1200 {
            let p = |r: &mut Rng| -> i32 {
                match r.below(3) {
                    0 => r.range(0, 100001) as i32,
                    1 => r.u32() as i32,
                    _ => r.range(-100000, 100001) as i32,
                }
            };
            xyzs.push(png_XYZ {
                red_X: p(&mut rng),
                red_Y: p(&mut rng),
                red_Z: p(&mut rng),
                green_X: p(&mut rng),
                green_Y: p(&mut rng),
                green_Z: p(&mut rng),
                blue_X: p(&mut rng),
                blue_Y: p(&mut rng),
                blue_Z: p(&mut rng),
            });
        }
        for XYZ in &xyzs {
            let mut xy = png_xy {
                redx: 0x5a5a_5a5a,
                redy: 0x5a5a_5a5a,
                greenx: 0x5a5a_5a5a,
                greeny: 0x5a5a_5a5a,
                bluex: 0x5a5a_5a5a,
                bluey: 0x5a5a_5a5a,
                whitex: 0x5a5a_5a5a,
                whitey: 0x5a5a_5a5a,
            };
            let r = (api.png_xy_from_XYZ)(&mut xy, XYZ);
            o.push(format!("xy_from_XYZ({:?})={} {:?}", XYZ, r, xy));
        }
        o
    });
}

/* ================================================================== */
/* C-9  png_check_fp_number, png_check_fp_string                        */
/* ================================================================== */

#[test]
fn fp_parse() {
    const ALPHABET: &[u8] = b"0123456789+-.eE ";

    let mut strings: Vec<Vec<u8>> = Vec::new();
    for s in [
        "", "0", "-0", "+0", "1", "-1", ".", "-.", ".5", "0.5", "5.", "1e5", "1E5", "1e+5",
        "1e-5", "1e", "1e+", ".e5", "0.e5", "1.2.3", "--1", "1-2", "1 2", " 1", "1 ", "1.5e-10",
        "0000", "00.00", "1e10000", "1e-10000", "123456789012345678901234567890",
        "1.7976931348623157e308", "2.2250738585072014e-308", "+.0e+0", "1.e5", "e5", "E", ".0",
        "0.", "-", "+", "3.14159265358979", "1e2e3", "1..2", "00", "-00.00e-00",
    ] {
        strings.push(s.as_bytes().to_vec());
    }
    let mut rng = Rng::new(0xC009);
    for _ in 0..900 {
        let n = rng.below(17);
        strings.push((0..n).map(|_| rng.pick(ALPHABET)).collect());
    }
    // a few with bytes outside the alphabet (which terminate the scan)
    for _ in 0..200 {
        let n = 1 + rng.below(12);
        strings.push((0..n).map(|_| rng.u8()).collect());
    }

    assert_same("C-9 fp parse", |api| unsafe {
        let mut o = Outcome::default();
        for s in &strings {
            // A NUL-terminated copy with slack, so `size` can exceed the string
            // length without reading uninitialised memory.
            let mut buf = vec![0u8; s.len() + 8];
            buf[..s.len()].copy_from_slice(s);
            let p = buf.as_ptr() as *const c_char;

            for size in 0..=s.len() + 2 {
                // Initial states: each PNG_FP_* bit alone, the three sticky
                // bits, the four state values and a few combinations, since
                // png_check_fp_number resumes from whatever it is handed.
                for &state0 in &[
                    0i32, 1, 2, 3, 4, 8, 16, 32, 64, 128, 256, 448, 60, 9, 17, 33, 12, 24,
                    1 | 8, 2 | 4, 2 | 8 | 64, 3 | 60, 128 | 256, 511,
                ] {
                    for &pos0 in &[0usize, 1, size] {
                        if pos0 > size {
                            continue;
                        }
                        let mut state = state0;
                        let mut pos = pos0;
                        let r = (api.png_check_fp_number)(p, size, &mut state, &mut pos);
                        o.push(format!(
                            "fp_number({:?},size={},st={},pos={})={} st={} pos={}",
                            String::from_utf8_lossy(s),
                            size,
                            state0,
                            pos0,
                            r,
                            state,
                            pos
                        ));
                    }
                }
                o.push(format!(
                    "fp_string({:?},{})={}",
                    String::from_utf8_lossy(s),
                    size,
                    (api.png_check_fp_string)(p, size)
                ));
            }
        }
        o
    });
}

/* ================================================================== */
/* C-10  png_ascii_from_fp, png_ascii_from_fixed                        */
/* ================================================================== */

#[test]
fn ascii_from() {
    // DBL_DIG is 15, so precision is clamped into 1..=16.
    const SIZES: [usize; 14] = [0, 1, 2, 5, 6, 7, 8, 10, 12, 13, 15, 20, 25, 40];
    const PRECS: [c_uint; 9] = [0, 1, 2, 3, 5, 6, 15, 16, 20];

    let mut doubles: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.1,
        0.01,
        0.001,
        0.0001,
        9.9999999,
        0.99999999,
        1e-5,
        1e5,
        1e15,
        1e16,
        1e-300,
        1e300,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1.0 / 3.0,
        2.0 / 3.0,
        3.14159265358979,
        123456.789,
        -123456.789,
        1234567890123456.0,
    ];
    let mut rng = Rng::new(0xC010);
    for _ in 0..24 {
        let m = (rng.u32() as f64) / (u32::MAX as f64);
        let e = rng.range(-20, 21) as i32;
        let s = if rng.bool() { 1.0 } else { -1.0 };
        doubles.push(s * m * 10f64.powi(e));
    }
    // Values that force the "rounding up to 10" recovery loop in
    // png_ascii_from_fp: a run of 9s that carries all the way back through the
    // digit list (and, for the 0.0nn cases, through the leading zeros).
    for k in 1..18usize {
        let nines: String = "9".repeat(k);
        for pre in ["0.", "0.0", "0.00", "9.", "99.", "0.0000"] {
            if let Ok(v) = format!("{}{}", pre, nines).parse::<f64>() {
                doubles.push(v);
                doubles.push(-v);
            }
        }
        doubles.push(1.0 - 10f64.powi(-(k as i32)));
        doubles.push(1.0 + 10f64.powi(-(k as i32)));
        doubles.push(10f64.powi(k as i32) - 0.5);
        doubles.push(10f64.powi(-(k as i32)) * 9.999999999);
    }

    let mut fixeds: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        99999,
        100000,
        100001,
        -100000,
        12345,
        -12345,
        2_147_400_000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        50000,
        -50000,
        1_000_000,
        -1_000_000,
    ];
    for _ in 0..120 {
        fixeds.push(rng.u32() as i32);
        fixeds.push(rng_fixed(&mut rng));
    }
    // png_ascii_from_fixed splits at five fractional digits and drops trailing
    // zeros, so cover every trailing-zero count and the 0x80000000 overflow
    // guard on the negative side.
    for k in 0..11u32 {
        let p = 10i64.pow(k);
        for m in [1i64, 5, 9, 10, 99] {
            let v = (m * p).min(i32::MAX as i64) as i32;
            fixeds.push(v);
            fixeds.push(-v);
        }
    }
    fixeds.push(-2_147_483_647);
    fixeds.push(100_000 - 1);
    fixeds.push(100_000 + 1);

    assert_same("C-10 ascii from fp/fixed", |api| unsafe {
        let mut o = Outcome::default();
        for &size in &SIZES {
            for &prec in &PRECS {
                for d in &doubles {
                    // 64-byte buffer, `size` never exceeds 40, so 24 bytes of
                    // pattern remain to catch an overrun.
                    let mut buf = [0x5au8; 64];
                    let (png, info) = new_read(api);
                    let g = guarded(api, png, &mut || {
                        (api.png_ascii_from_fp)(
                            png,
                            buf.as_mut_ptr() as *mut c_char,
                            size,
                            *d,
                            prec,
                        );
                    });
                    o.push(format!(
                        "ascii_from_fp(size={},prec={},{:e}) {:?} {:02x?}",
                        size, prec, d, g, buf
                    ));
                    destroy_read(api, png, info);
                }
            }
            for f in &fixeds {
                let mut buf = [0x5au8; 64];
                let (png, info) = new_read(api);
                let g = guarded(api, png, &mut || {
                    (api.png_ascii_from_fixed)(png, buf.as_mut_ptr() as *mut c_char, size, *f);
                });
                o.push(format!(
                    "ascii_from_fixed(size={},{}) {:?} {:02x?}",
                    size, f, g, buf
                ));
                destroy_read(api, png, info);
            }
        }
        o
    });
}

/* ================================================================== */
/* C-11  png_safecat, png_format_number                                 */
/* ================================================================== */

#[test]
fn safecat_format() {
    let mut strings: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"hello".to_vec(),
        b" +0000".to_vec(),
        b"0123456789".to_vec(),
        vec![b'x'; 40],
        vec![b'y'; 63],
    ];
    let mut rng = Rng::new(0xC011);
    for _ in 0..60 {
        let n = rng.below(24);
        strings.push((0..n).map(|_| 1 + (rng.u8() % 127)).collect());
    }

    assert_same("C-11 png_safecat", |api| unsafe {
        let mut o = Outcome::default();
        for s in &strings {
            let z = cs(&String::from_utf8_lossy(s).replace('\0', "?"));
            for bufsize in [0usize, 1, 2, 3, 5, 8, 16, 24, 32] {
                for pos in [0usize, 1, 2, 7, 15, 16, 17, 31, 32, 33] {
                    // 64-byte pattern buffer; bufsize <= 32 so the tail is
                    // never legitimately touched.
                    let mut buf = [0x5au8; 64];
                    let r = (api.png_safecat)(
                        buf.as_mut_ptr() as *mut c_char,
                        bufsize,
                        pos,
                        z.as_ptr(),
                    );
                    o.push(format!(
                        "safecat({:?},bufsize={},pos={})={} {:02x?}",
                        z, bufsize, pos, r, buf
                    ));
                }
            }
            // NULL string: only the terminator is written.
            let mut buf = [0x5au8; 64];
            let r = (api.png_safecat)(
                buf.as_mut_ptr() as *mut c_char,
                16,
                3,
                core::ptr::null(),
            );
            o.push(format!("safecat(NULL,16,3)={} {:02x?}", r, buf));
            // NULL buffer: nothing at all happens.
            o.push(format!(
                "safecat(NULL buf)={}",
                (api.png_safecat)(core::ptr::null_mut(), 16, 3, z.as_ptr())
            ));
        }
        o
    });

    // png_format_number(start, end, format, number) writes backwards from
    // `end` and returns a pointer into [start,end).
    let mut numbers: Vec<u64> = vec![
        0,
        1,
        9,
        10,
        11,
        99,
        100,
        12345,
        99999,
        100000,
        100001,
        1_000_000,
        0xff,
        0xdead_beef,
        u32::MAX as u64,
        i32::MAX as u64,
        u64::MAX,
        u64::MAX / 2,
        50000,
        90000,
        10_0000_0000,
    ];
    let mut rng = Rng::new(0xC011_2);
    for _ in 0..200 {
        numbers.push(match rng.below(3) {
            0 => rng.below(1_000_000) as u64,
            1 => rng.u32() as u64,
            _ => rng.next_u64(),
        });
    }

    assert_same("C-11 png_format_number", |api| unsafe {
        let mut o = Outcome::default();
        for n in &numbers {
            for fmt in [-1i32, 0, 1, 2, 3, 4, 5, 6, 24] {
                for len in [1usize, 2, 3, 4, 6, 8, 12, 24] {
                    let mut buf = [0x5au8; 64];
                    let base = buf.as_mut_ptr() as *mut c_char;
                    let start = base.add(8) as *const c_char;
                    let end = base.add(8 + len);
                    let r = (api.png_format_number)(start, end, fmt, *n as usize);
                    let off = (r as usize).wrapping_sub(base as usize);
                    o.push(format!(
                        "format_number({},fmt={},len={}) off={} {:02x?}",
                        n, fmt, len, off as isize, buf
                    ));
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-12  png_reset_crc, png_calculate_crc, png_get_io_chunk_type        */
/* ================================================================== */

/// `png_ptr->crc` is not reachable from outside the library, so the only way to
/// observe `png_reset_crc`/`png_calculate_crc` quantitatively is to recompute
/// the CRC of a chunk from inside the read callback, at the very moment libpng
/// has read the four CRC bytes and is about to compare them with
/// `png_ptr->crc` (see `png_crc_error` in pngrutil.c).  If the recomputation
/// agrees with the file the chunk is accepted, otherwise libpng reports
/// "CRC error"; the two libraries must make the same decision.
struct CrcHook {
    /// Input offset of the 4 CRC bytes of the chunk under test.
    at: usize,
    /// Bytes to feed to `png_calculate_crc`, split into pieces.
    pieces: Vec<Vec<u8>>,
    fired: u32,
}

thread_local! {
    static CRC_HOOK: std::cell::Cell<*mut CrcHook> =
        const { std::cell::Cell::new(core::ptr::null_mut()) };
}

unsafe extern "C" fn crc_read_cb(png: *mut PngStruct, data: *mut u8, len: usize) {
    let t = tls();
    let pos = t.in_pos;
    let avail = t.input.len().saturating_sub(pos);
    let n = len.min(avail);
    if n > 0 {
        core::ptr::copy_nonoverlapping(t.input.as_ptr().add(pos), data, n);
        t.in_pos += n;
    }
    if n < len {
        log(format!("read_fn: short read ({} of {})", n, len));
        (cur_api().png_error)(png, b"Read Error\0".as_ptr() as *const c_char);
    }
    let h = CRC_HOOK.with(|c| c.get());
    if !h.is_null() && len == 4 && pos == (*h).at {
        (*h).fired += 1;
        let api = cur_api();
        log(format!(
            "hook: io_chunk_type={:#x} reset+calculate {} piece(s)",
            (api.png_get_io_chunk_type)(png),
            (*h).pieces.len()
        ));
        (api.png_reset_crc)(png);
        for p in &(*h).pieces {
            (api.png_calculate_crc)(png, p.as_ptr(), p.len());
        }
    }
}

#[test]
fn crc() {
    let base = handmade_gray1x1();
    let mut rng = Rng::new(0xC012);

    // Data lengths 0..4096 and a random split into 1..n pieces.
    let lengths: Vec<usize> = vec![0, 1, 2, 3, 4, 7, 8, 15, 16, 63, 64, 255, 256, 1000, 4095, 4096];

    for (li, &len) in lengths.iter().enumerate() {
        let data = rng.bytes(len);
        let file = insert_before(&base, "IDAT", &chunk(b"abCd", &data));
        let at = split_chunks(&file)
            .iter()
            .find(|(n, _)| n == "abCd")
            .map(|(_, r)| r.end - 4)
            .expect("inserted chunk");

        // the bytes libpng itself CRCs: the 4 name bytes then the data
        let mut whole = b"abCd".to_vec();
        whole.extend_from_slice(&data);

        for split in [1usize, 2, 3, 5, 17] {
            let mut pieces: Vec<Vec<u8>> = Vec::new();
            let mut i = 0;
            let step = (whole.len() / split).max(1);
            while i < whole.len() {
                let k = step.min(whole.len() - i);
                pieces.push(whole[i..i + k].to_vec());
                i += k;
            }
            // an empty piece exercises the `length > 0` guard
            pieces.push(Vec::new());

            for corrupt in [false, true] {
                for quiet in [false, true] {
                    let mut ps = pieces.clone();
                    if corrupt {
                        // flip one byte of the CRC input
                        if ps[0].is_empty() {
                            ps[0].push(0);
                        } else {
                            ps[0][0] ^= 0x01;
                        }
                    }
                    let case = format!(
                        "C-12 crc len={} split={} corrupt={} quiet={}",
                        len, split, corrupt, quiet
                    );
                    let file = file.clone();
                    assert_same(&case, |api| unsafe {
                        let mut o = Outcome::default();
                        let mut hook = Box::new(CrcHook {
                            at,
                            pieces: ps.clone(),
                            fired: 0,
                        });
                        CRC_HOOK.with(|c| c.set(&mut *hook as *mut CrcHook));
                        tls().input = file.clone();
                        tls().in_pos = 0;
                        let (png, info) = new_read(api);
                        (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(crc_read_cb));
                        if quiet {
                            // ANCILLARY_USE|ANCILLARY_NOWARN: png_calculate_crc
                            // becomes a no-op and png_crc_error does not
                            // compare at all.
                            (api.png_set_crc_action)(
                                png,
                                PNG_CRC_QUIET_USE,
                                PNG_CRC_QUIET_USE,
                            );
                        }
                        let g = guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            log(format!(
                                "after read_info io_chunk_type={:#x}",
                                (api.png_get_io_chunk_type)(png)
                            ));
                            let rb = (api.png_get_rowbytes)(png, info);
                            let mut row = vec![0u8; rb.max(1)];
                            (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
                            o.output.extend_from_slice(&row);
                            (api.png_read_end)(png, info);
                        });
                        o.push(format!("guard={:?} fired={}", g, hook.fired));
                        destroy_read(api, png, info);
                        CRC_HOOK.with(|c| c.set(core::ptr::null_mut()));
                        o
                    });
                }
            }
        }
        let _ = li;
    }

    // Direct calls on a fresh read struct: neither library may error or warn,
    // and png_get_io_chunk_type must agree.
    assert_same("C-12 direct reset/calculate", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC012_0D);
        let (png, info) = new_read(api);
        let g = guarded(api, png, &mut || {
            for n in [0usize, 1, 2, 5, 100, 4096] {
                let d = rng.bytes(n);
                (api.png_reset_crc)(png);
                (api.png_calculate_crc)(png, d.as_ptr(), d.len());
                (api.png_calculate_crc)(png, d.as_ptr(), 0);
                (api.png_calculate_crc)(png, d.as_ptr(), d.len());
                log(format!(
                    "direct n={} io_chunk_type={:#x}",
                    n,
                    (api.png_get_io_chunk_type)(png)
                ));
            }
            (api.png_reset_crc)(png);
        });
        o.push(format!("direct guard={:?}", g));
        destroy_read(api, png, info);
        o
    });
}

/* ================================================================== */
/* C-13  png_do_bgr / _invert / _packswap / _swap / _strip_channel      */
/* ================================================================== */

#[test]
fn row_ops() {
    // Every legal (colour type, bit depth) x widths 1..17 x random rows.
    // 16 bytes of pattern follow the row so a write past `rowbytes` shows up.
    const SLACK: usize = 16;

    for (name, which) in [
        ("bgr", 0usize),
        ("invert", 1),
        ("packswap", 2),
        ("swap", 3),
    ] {
        assert_same(&format!("C-13 png_do_{}", name), |api| unsafe {
            let mut o = Outcome::default();
            let mut rng = Rng::new(0xC013 ^ which as u64);
            for (ct, bd) in VALID_SHAPES {
                for w in 1u32..=17 {
                    for _ in 0..3 {
                        let mut r = ri(w, ct, bd);
                        let mut buf = vec![0xa5u8; r.rowbytes + SLACK];
                        for i in 0..r.rowbytes {
                            buf[i] = rng.u8();
                        }
                        let before = buf.clone();
                        match which {
                            0 => (api.png_do_bgr)(&mut r, buf.as_mut_ptr()),
                            1 => (api.png_do_invert)(&mut r, buf.as_mut_ptr()),
                            2 => (api.png_do_packswap)(&mut r, buf.as_mut_ptr()),
                            _ => (api.png_do_swap)(&mut r, buf.as_mut_ptr()),
                        }
                        o.push(format!(
                            "{} ct={} bd={} w={} in={:02x?} out={:02x?} ri[{}]",
                            name,
                            ct,
                            bd,
                            w,
                            &before[..r.rowbytes.min(before.len())],
                            &buf,
                            show_ri(&r)
                        ));
                    }
                }
            }
            // ... plus row_infos described purely by pixel depth, which is what
            // png_do_packswap / png_do_swap actually branch on.
            for pd in PIXEL_DEPTHS {
                for w in 1u32..=9 {
                    let mut r = ri_pd(w, pd);
                    let mut buf = vec![0xa5u8; r.rowbytes + SLACK];
                    for i in 0..r.rowbytes {
                        buf[i] = rng.u8();
                    }
                    let before = buf.clone();
                    match which {
                        0 => (api.png_do_bgr)(&mut r, buf.as_mut_ptr()),
                        1 => (api.png_do_invert)(&mut r, buf.as_mut_ptr()),
                        2 => (api.png_do_packswap)(&mut r, buf.as_mut_ptr()),
                        _ => (api.png_do_swap)(&mut r, buf.as_mut_ptr()),
                    }
                    o.push(format!(
                        "{} pd={} w={} in={:02x?} out={:02x?} ri[{}]",
                        name,
                        pd,
                        w,
                        &before[..r.rowbytes.min(before.len())],
                        &buf,
                        show_ri(&r)
                    ));
                }
            }
            // Each of these routines dispatches on colour type, bit depth,
            // channels and rowbytes *independently*, so also feed row_infos
            // whose colour type does not match the channel count.  The buffer
            // is sized for the worst case (8 channels x 16 bits) so that a
            // routine which strides by more than `rowbytes` stays in bounds.
            for ct in 0..8u8 {
                for bd in [1u8, 2, 4, 8, 16] {
                    for ch in [1u8, 2, 3, 4] {
                        for w in [1u32, 2, 3, 5, 8] {
                            let pd = bd * ch;
                            let mut r = png_row_info {
                                width: w,
                                rowbytes: png_rowbytes(pd as usize, w as usize),
                                color_type: ct,
                                bit_depth: bd,
                                channels: ch,
                                pixel_depth: pd,
                            };
                            let n = (w as usize) * 16 + 64;
                            let mut buf = vec![0xa5u8; n];
                            for b in buf.iter_mut() {
                                *b = rng.u8();
                            }
                            let before = buf.clone();
                            match which {
                                0 => (api.png_do_bgr)(&mut r, buf.as_mut_ptr()),
                                1 => (api.png_do_invert)(&mut r, buf.as_mut_ptr()),
                                2 => (api.png_do_packswap)(&mut r, buf.as_mut_ptr()),
                                _ => (api.png_do_swap)(&mut r, buf.as_mut_ptr()),
                            }
                            o.push(format!(
                                "{} mixed ct={} bd={} ch={} w={} in={:02x?} out={:02x?} ri[{}]",
                                name,
                                ct,
                                bd,
                                ch,
                                w,
                                before,
                                buf,
                                show_ri(&r)
                            ));
                        }
                    }
                }
            }
            o
        });
    }

    // png_do_strip_channel also rewrites row_info (channels, pixel_depth,
    // colour type and rowbytes), so all of that is compared too.
    assert_same("C-13 png_do_strip_channel", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC013_5);
        for (ct, bd) in VALID_SHAPES {
            for w in 1u32..=17 {
                for at_start in [0i32, 1, 2, -1] {
                    let mut r = ri(w, ct, bd);
                    let mut buf = vec![0xa5u8; r.rowbytes + SLACK];
                    for i in 0..r.rowbytes {
                        buf[i] = rng.u8();
                    }
                    let before = buf.clone();
                    (api.png_do_strip_channel)(&mut r, buf.as_mut_ptr(), at_start);
                    o.push(format!(
                        "strip ct={} bd={} w={} at_start={} in={:02x?} out={:02x?} ri[{}]",
                        ct,
                        bd,
                        w,
                        at_start,
                        &before[..before.len() - SLACK],
                        &buf,
                        show_ri(&r)
                    ));
                }
            }
        }
        // channels 2 and 4 with the bit depths the routine rejects (1/2/4)
        for ch in [1u8, 2, 3, 4, 5] {
            for bd in [1u8, 2, 4, 8, 16] {
                for w in 1u32..=5 {
                    for at_start in [0i32, 1] {
                        let pd = bd * ch;
                        let mut r = png_row_info {
                            width: w,
                            rowbytes: png_rowbytes(pd as usize, w as usize),
                            color_type: if ch == 2 {
                                PNG_COLOR_TYPE_GRAY_ALPHA as u8
                            } else {
                                PNG_COLOR_TYPE_RGB_ALPHA as u8
                            },
                            bit_depth: bd,
                            channels: ch,
                            pixel_depth: pd,
                        };
                        let mut buf = vec![0xa5u8; r.rowbytes + SLACK];
                        for i in 0..r.rowbytes {
                            buf[i] = rng.u8();
                        }
                        let before = buf.clone();
                        (api.png_do_strip_channel)(&mut r, buf.as_mut_ptr(), at_start);
                        o.push(format!(
                            "strip ch={} bd={} w={} at={} in={:02x?} out={:02x?} ri[{}]",
                            ch,
                            bd,
                            w,
                            at_start,
                            &before[..before.len() - SLACK],
                            &buf,
                            show_ri(&r)
                        ));
                    }
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-14  png_read_filter_row                                            */
/* ================================================================== */

#[test]
fn read_filter_row() {
    // png_read_filter_row picks the paeth implementation from
    // png_ptr->pixel_depth the first time it is called (see
    // png_init_filter_functions), and png_handle_IHDR is what sets that field.
    // So the same direct calls are made on three different png_structs:
    // a fresh one (pixel_depth 0 -> multi-byte paeth), one that has read an
    // 8-bit gray IHDR (pixel_depth 8, bpp 1 -> single-byte paeth) and one that
    // has read a 16-bit RGBA IHDR (pixel_depth 64 -> multi-byte paeth).
    for setup_depth in [0i32, 8, 64] {
        assert_same(
            &format!("C-14 read_filter_row (png pixel_depth {})", setup_depth),
            |api| unsafe {
                let mut o = Outcome::default();
                let mut rng = Rng::new(0xC014 ^ setup_depth as u64);
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                if setup_depth != 0 {
                    let file = if setup_depth == 8 {
                        gray_file(8)
                    } else {
                        let mut r = Rng::new(0x1616);
                        let img =
                            Img::random(&mut r, 3, 2, PNG_COLOR_TYPE_RGB_ALPHA, 16);
                        build_with_c(&img, &WriteOpts::default())
                    };
                    tls().input = file;
                    tls().in_pos = 0;
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                    });
                    o.push(format!("setup guard={:?}", g));
                }

                for pd in PIXEL_DEPTHS {
                    for w in [1u32, 2, 3, 5, 7, 8, 9, 15, 16, 17, 32, 33] {
                        for filter in [-1i32, 0, 1, 2, 3, 4, 5, 6, 255] {
                            let mut r = ri_pd(w, pd);
                            let rb = r.rowbytes;
                            let mut row = vec![0xa5u8; rb + 16];
                            let mut prev = vec![0x5au8; rb + 16];
                            for i in 0..rb {
                                row[i] = rng.u8();
                                prev[i] = rng.u8();
                            }
                            let row0 = row.clone();
                            (api.png_read_filter_row)(
                                png,
                                &mut r,
                                row.as_mut_ptr(),
                                prev.as_ptr(),
                                filter,
                            );
                            o.push(format!(
                                "filter={} pd={} w={} rb={} in={:02x?} prev={:02x?} out={:02x?} ri[{}]",
                                filter,
                                pd,
                                w,
                                rb,
                                &row0[..rb],
                                &prev[..rb],
                                &row,
                                show_ri(&r)
                            ));
                        }
                    }
                }
                destroy_read(api, png, info);
                o
            },
        );
    }
}

/* ================================================================== */
/* C-15  png_write_find_filter                                          */
/* ================================================================== */

/// `png_write_find_filter` reads `png_ptr->row_buf`, `do_filter`, `try_row`,
/// `tst_row` and `prev_row` and then *writes* the filtered row into the IDAT
/// stream, so it can only be driven the way a real caller does: a complete
/// write with the matching filter mask.  Every filter mask 0x00..0xf8 is tried
/// over every colour type / bit depth, on an image tall enough that the first
/// row (no prev_row) and later rows are both filtered.
#[test]
fn write_find_filter() {
    for k in 0..32u32 {
        let mask = (k << 3) as c_int;
        for (ct, bd) in VALID_SHAPES {
            let mut rng = Rng::new(0xC015 ^ ((k as u64) << 16) ^ ((ct as u64) << 8) ^ bd as u64);
            let img = Img::random(&mut rng, 9, 4, ct, bd);
            let opts = WriteOpts {
                filter_mask: Some(mask),
                level: Some(6),
                ..Default::default()
            };
            let case = format!("C-15 find_filter mask={:#04x} ct={} bd={}", mask, ct, bd);
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                let wr = write_plain(api, &img, &opts);
                o.push(format!("guard={:?}", wr.guard));
                o.output = wr.bytes.clone();
                o
            });
        }
    }

    // ... and once more over a wider row, an interlaced image (which changes
    // rowbytes on every pass) and 16-bit data, for the multi-byte bpp paths.
    for &(w, h) in &[(1u32, 1u32), (2, 2), (17, 3), (33, 5), (64, 2)] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for mask in [
                PNG_NO_FILTERS,
                PNG_FILTER_NONE,
                PNG_FILTER_SUB,
                PNG_FILTER_UP,
                PNG_FILTER_AVG,
                PNG_FILTER_PAETH,
                PNG_FAST_FILTERS,
                PNG_ALL_FILTERS,
                PNG_FILTER_SUB | PNG_FILTER_PAETH,
                PNG_FILTER_AVG | PNG_FILTER_UP,
            ] {
                for (ct, bd) in [
                    (PNG_COLOR_TYPE_GRAY, 1),
                    (PNG_COLOR_TYPE_GRAY, 16),
                    (PNG_COLOR_TYPE_RGB, 8),
                    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
                    (PNG_COLOR_TYPE_PALETTE, 4),
                ] {
                    let mut rng = Rng::new(
                        0xC015_2 ^ ((w as u64) << 32) ^ ((mask as u64) << 8) ^ (il as u64) ^ bd as u64,
                    );
                    let mut img = Img::random(&mut rng, w, h, ct, bd);
                    img.interlace = il;
                    let opts = WriteOpts {
                        filter_mask: Some(mask),
                        ..Default::default()
                    };
                    let case = format!(
                        "C-15 find_filter {}x{} il={} mask={:#04x} ct={} bd={}",
                        w, h, il, mask, ct, bd
                    );
                    assert_same(&case, |api| unsafe {
                        let mut o = Outcome::default();
                        let wr = write_plain(api, &img, &opts);
                        o.push(format!("guard={:?}", wr.guard));
                        o.output = wr.bytes.clone();
                        o
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* C-16  png_combine_row                                                */
/* ================================================================== */

/// `png_combine_row` reads `transformed_pixel_depth`, `row_buf`, `width`,
/// `pass`, `info_rowbytes`, `transformations` and `interlaced` — none of which
/// is reachable from outside — so it is driven through `png_read_row` with a
/// non-NULL `display_row`, over interlaced images (display 0 *and* 1 on every
/// pass), with both destinations pre-filled with a random pattern so that the
/// partial-last-byte merge is observable.  The one thing that *can* be called
/// directly is the `pixel_depth == 0` guard on a fresh struct.
#[test]
fn combine_row() {
    for (ct, bd) in VALID_SHAPES {
        for &(w, h) in &[
            (1u32, 1u32),
            (3, 3),
            (5, 5),
            (9, 4),
            (17, 2),
            (8, 8),
            (33, 3),
            (2, 1),
            (7, 7),
        ] {
            for packswap in [false, true] {
                if packswap && bd > 8 {
                    continue;
                }
                let mut rng = Rng::new(
                    0xC016 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ ((w as u64) << 8) ^ h as u64,
                );
                let mut img = Img::random(&mut rng, w, h, ct, bd);
                img.interlace = PNG_INTERLACE_ADAM7;
                let file = build_with_c(&img, &WriteOpts::default());
                let case = format!(
                    "C-16 combine ct={} bd={} {}x{} packswap={}",
                    ct, bd, w, h, packswap
                );
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = file.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        if packswap {
                            (api.png_set_packswap)(png);
                        }
                        let passes = (api.png_set_interlace_handling)(png);
                        (api.png_read_update_info)(png, info);
                        let rb = (api.png_get_rowbytes)(png, info);
                        let hh = (api.png_get_image_height)(png, info) as usize;
                        log(format!("passes={} rowbytes={} h={}", passes, rb, hh));
                        // 16 bytes of pattern after each row catch an overrun.
                        let mut row = vec![0u8; rb + 16];
                        let mut disp = vec![0u8; rb + 16];
                        let mut fill = Rng::new(0xF111);
                        for p in 0..passes {
                            for y in 0..hh {
                                for i in 0..row.len() {
                                    row[i] = fill.u8();
                                    disp[i] = fill.u8();
                                }
                                (api.png_read_row)(
                                    png,
                                    row.as_mut_ptr(),
                                    disp.as_mut_ptr(),
                                );
                                log(format!(
                                    "p={} y={} row={:02x?} disp={:02x?}",
                                    p, y, row, disp
                                ));
                            }
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                });
            }
        }
    }

    // Non-interlaced: png_combine_row reduces to a memcpy plus the
    // partial-last-byte restore, which the pre-filled destination exposes.
    for (ct, bd) in VALID_SHAPES {
        for &(w, h) in &[(1u32, 1u32), (3, 2), (9, 2), (17, 2), (33, 2)] {
            let mut rng = Rng::new(0xC016_AA ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ w as u64);
            let img = Img::random(&mut rng, w, h, ct, bd);
            let file = build_with_c(&img, &WriteOpts::default());
            let case = format!("C-16 combine plain ct={} bd={} {}x{}", ct, bd, w, h);
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                tls().input = file.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let g = guarded(api, png, &mut || {
                    (api.png_read_info)(png, info);
                    (api.png_read_update_info)(png, info);
                    let rb = (api.png_get_rowbytes)(png, info);
                    let hh = (api.png_get_image_height)(png, info) as usize;
                    let mut row = vec![0u8; rb + 16];
                    let mut disp = vec![0u8; rb + 16];
                    let mut fill = Rng::new(0xF222);
                    for y in 0..hh {
                        for i in 0..row.len() {
                            row[i] = fill.u8();
                            disp[i] = fill.u8();
                        }
                        (api.png_read_row)(png, row.as_mut_ptr(), disp.as_mut_ptr());
                        log(format!("y={} row={:02x?} disp={:02x?}", y, row, disp));
                    }
                    (api.png_read_end)(png, info);
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            });
        }
    }

    // Direct call before any row has been transformed: png_combine_row must
    // png_error("internal row logic error") in both libraries.
    assert_same("C-16 combine_row on a fresh struct", |api| unsafe {
        let mut o = Outcome::default();
        for display in [0i32, 1, 2] {
            let (png, info) = new_read(api);
            let mut dst = vec![0u8; 64];
            let g = guarded(api, png, &mut || {
                (api.png_combine_row)(png, dst.as_mut_ptr(), display);
            });
            o.push(format!("combine(display={}) guard={:?}", display, g));
            destroy_read(api, png, info);
        }
        o
    });
}

/* ================================================================== */
/* C-17  png_do_read_interlace, png_do_write_interlace                  */
/* ================================================================== */

#[test]
fn interlace_row() {
    assert_same("C-17 png_do_read_interlace", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC017);
        for pass in 0..7i32 {
            for pd in PIXEL_DEPTHS {
                for w in [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 33] {
                    // Only PNG_PACKSWAP is looked at, but neighbouring bits are
                    // set too so that a wrong mask in the translation shows up.
                    for tr in [
                        0u32,
                        PNG_PACKSWAP,
                        0xffff_ffff,
                        !PNG_PACKSWAP,
                        PNG_PACKSWAP | 0x0f,
                        0x8000,
                        0x2_0000,
                    ] {
                        let mut r = ri_pd(w, pd);
                        let final_width = w * PASS_INC[pass as usize];
                        // Generous: room for the expanded row plus slack.
                        let need = png_rowbytes(pd as usize, final_width as usize);
                        let mut buf = vec![0u8; need + 32];
                        // The sub-byte cases read the destination bytes they
                        // partially overwrite, so pre-fill the whole buffer.
                        for b in buf.iter_mut() {
                            *b = rng.u8();
                        }
                        let before = buf.clone();
                        (api.png_do_read_interlace)(
                            &mut r,
                            buf.as_mut_ptr(),
                            pass,
                            tr,
                        );
                        o.push(format!(
                            "read_il pass={} pd={} w={} tr={:#x} in={:02x?} out={:02x?} ri[{}]",
                            pass,
                            pd,
                            w,
                            tr,
                            before,
                            buf,
                            show_ri(&r)
                        ));
                    }
                }
            }
        }
        // NULL arguments are explicitly tolerated.
        let mut r = ri_pd(4, 8);
        (api.png_do_read_interlace)(&mut r, core::ptr::null_mut(), 0, 0);
        o.push(format!("read_il NULL row ri[{}]", show_ri(&r)));
        (api.png_do_read_interlace)(core::ptr::null_mut(), core::ptr::null_mut(), 0, 0);
        o.push("read_il NULL both ok".to_string());
        o
    });

    assert_same("C-17 png_do_write_interlace", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC017_0F);
        for pass in 0..7i32 {
            for pd in PIXEL_DEPTHS {
                for w in [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 33] {
                    let mut r = ri_pd(w, pd);
                    let mut buf = vec![0u8; r.rowbytes + 32];
                    for b in buf.iter_mut() {
                        *b = rng.u8();
                    }
                    let before = buf.clone();
                    (api.png_do_write_interlace)(&mut r, buf.as_mut_ptr(), pass);
                    o.push(format!(
                        "write_il pass={} pd={} w={} in={:02x?} out={:02x?} ri[{}]",
                        pass,
                        pd,
                        w,
                        before,
                        buf,
                        show_ri(&r)
                    ));
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-18  png_check_IHDR                                                 */
/* ================================================================== */

#[test]
fn check_ihdr() {
    const DIMS: [u32; 9] = [
        0,
        1,
        7,
        8,
        1000,
        1_000_000,
        1_000_001,
        PNG_UINT_31_MAX,
        0x8000_0000,
    ];

    assert_same("C-18 png_check_IHDR legal shapes", |api| unsafe {
        let mut o = Outcome::default();
        for (ct, bd) in VALID_SHAPES {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7, PNG_INTERLACE_LAST, 7] {
                for &w in &DIMS {
                    for &h in &[1u32, 7, 1_000_000, 0] {
                        for ft in [PNG_FILTER_TYPE_BASE, PNG_INTRAPIXEL_DIFFERENCING] {
                            for mng in [0u32, PNG_FLAG_MNG_FILTER_64 as u32] {
                                let (png, info) = new_read(api);
                                (api.png_permit_mng_features)(png, mng);
                                let g = guarded(api, png, &mut || {
                                    (api.png_check_IHDR)(png, w, h, bd, ct, il, 0, ft);
                                });
                                o.push(format!(
                                    "IHDR {}x{} bd={} ct={} il={} ft={} mng={} guard={:?}",
                                    w, h, bd, ct, il, ft, mng, g
                                ));
                                destroy_read(api, png, info);
                            }
                        }
                    }
                }
            }
        }
        o
    });

    assert_same("C-18 png_check_IHDR illegal shapes", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC018);
        let mut cases: Vec<(u32, u32, c_int, c_int, c_int, c_int, c_int)> = Vec::new();
        for bd in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 32] {
            for ct in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8] {
                cases.push((8, 8, bd, ct, PNG_INTERLACE_NONE, 0, 0));
            }
        }
        for ct2 in [-1i32, 0, 1, 5, 6, 7] {
            for comp in [-1i32, 0, 1, 2, 64] {
                cases.push((8, 8, 8, ct2, PNG_INTERLACE_NONE, comp, 0));
            }
        }
        for _ in 0..400 {
            cases.push((
                rng.u32(),
                rng.u32(),
                rng.range(-2, 20) as c_int,
                rng.range(-2, 10) as c_int,
                rng.range(-1, 4) as c_int,
                rng.range(-1, 3) as c_int,
                rng.pick(&[0i32, 64, 1, -1]),
            ));
        }
        for (w, h, bd, ct, il, comp, ft) in cases {
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || {
                (api.png_check_IHDR)(png, w, h, bd, ct, il, comp, ft);
            });
            o.push(format!(
                "IHDR {}x{} bd={} ct={} il={} comp={} ft={} guard={:?}",
                w, h, bd, ct, il, comp, ft, g
            ));
            destroy_read(api, png, info);
        }
        o
    });

    // The width/height limits come from png_ptr->user_width_max /
    // user_height_max, so vary those too.
    assert_same("C-18 png_check_IHDR user limits", |api| unsafe {
        let mut o = Outcome::default();
        for (uw, uh) in [
            (0u32, 0u32),
            (1, 1),
            (7, 7),
            (8, 8),
            (1000, 1000),
            (1_000_000, 1_000_000),
            (PNG_UINT_31_MAX, PNG_UINT_31_MAX),
        ] {
            for &w in &DIMS {
                for &h in &[1u32, 8, 1000, 1_000_001] {
                    let (png, info) = new_read(api);
                    (api.png_set_user_limits)(png, uw, uh);
                    let g = guarded(api, png, &mut || {
                        (api.png_check_IHDR)(
                            png,
                            w,
                            h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                    });
                    o.push(format!(
                        "IHDR limits({},{}) {}x{} guard={:?}",
                        uw, uh, w, h, g
                    ));
                    destroy_read(api, png, info);
                }
            }
        }
        o
    });
}

/* ================================================================== */
/* C-19  png_check_keyword                                              */
/* ================================================================== */

#[test]
fn check_keyword() {
    let mut keys: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"Title".to_vec(),
        b" leading".to_vec(),
        b"trailing ".to_vec(),
        b"  both  ".to_vec(),
        b"two  spaces".to_vec(),
        b"three   spaces".to_vec(),
        b" ".to_vec(),
        b"   ".to_vec(),
        b"tab\there".to_vec(),
        b"nl\nhere".to_vec(),
        vec![b'k'; 78],
        vec![b'k'; 79],
        vec![b'k'; 80],
        vec![b'k'; 81],
        vec![b'k'; 90],
        vec![b' '; 79],
        vec![0x80u8; 4],
        vec![0xa0u8; 4],
        vec![0xa1u8; 4],
        vec![0xffu8; 4],
        vec![0x01u8, 0x02, 0x1f, 0x20, 0x21, 0x7e, 0x7f],
    ];
    // 79/80-character keys with a bad character at the very end
    for n in [78usize, 79, 80] {
        let mut v = vec![b'x'; n];
        v[n - 1] = b' ';
        keys.push(v);
        let mut v = vec![b'x'; n];
        v[n - 1] = 0x7f;
        keys.push(v);
    }
    let mut rng = Rng::new(0xC019);
    for _ in 0..500 {
        let n = rng.below(95);
        keys.push(
            (0..n)
                .map(|_| match rng.below(4) {
                    0 => b' ',
                    1 => 1 + (rng.u8() % 0x1f),
                    2 => 0x21 + (rng.u8() % 0x5e),
                    _ => {
                        let b = rng.u8();
                        if b == 0 {
                            1
                        } else {
                            b
                        }
                    }
                })
                .collect(),
        );
    }

    assert_same("C-19 png_check_keyword", |api| unsafe {
        let mut o = Outcome::default();
        for k in &keys {
            // The contract is an 80-byte new_key buffer; 128 bytes of pattern
            // makes an overrun visible.
            let mut new_key = [0x5au8; 128];
            let mut z = k.clone();
            z.push(0);
            let (png, info) = new_read(api);
            let mut r = 0u32;
            let g = guarded(api, png, &mut || {
                r = (api.png_check_keyword)(
                    png,
                    z.as_ptr() as *const c_char,
                    new_key.as_mut_ptr(),
                );
            });
            o.push(format!(
                "keyword({:?}) guard={:?} len={} new_key={:02x?}",
                String::from_utf8_lossy(k),
                g,
                r,
                new_key
            ));
            destroy_read(api, png, info);
        }
        // key == NULL
        let mut new_key = [0x5au8; 128];
        let (png, info) = new_read(api);
        let mut r = 0u32;
        let g = guarded(api, png, &mut || {
            r = (api.png_check_keyword)(png, core::ptr::null(), new_key.as_mut_ptr());
        });
        o.push(format!(
            "keyword(NULL) guard={:?} len={} new_key={:02x?}",
            g, r, new_key
        ));
        destroy_read(api, png, info);
        o
    });
}

/* ================================================================== */
/* C-20  png_zstream_error, png_reset_zstream                           */
/* ================================================================== */

#[test]
fn zstream_error() {
    // Every zlib return code -6..2, plus the values outside that range that
    // fall through to `default`.  png_zstream_error only writes
    // png_ptr->zstream.msg, which is not readable from outside, so the direct
    // calls only assert that neither library errors or warns...
    assert_same("C-20 png_zstream_error direct", |api| unsafe {
        let mut o = Outcome::default();
        for read in [true, false] {
            for ret in -8i32..=4 {
                let (png, info) = if read { new_read(api) } else { new_write(api) };
                let g = guarded(api, png, &mut || {
                    (api.png_zstream_error)(png, ret);
                    // A second call must be a no-op (msg is no longer NULL).
                    (api.png_zstream_error)(png, 0);
                });
                o.push(format!("zstream_error(read={},{}) guard={:?}", read, ret, g));
                if read {
                    destroy_read(api, png, info);
                } else {
                    destroy_write(api, png, info);
                }
            }
        }
        o
    });

    // ... and the message itself is compared through the two public paths that
    // report png_ptr->zstream.msg verbatim.
    //
    // (a) write side: an invalid deflate parameter makes png_deflate_claim call
    // png_zstream_error(png, Z_STREAM_ERROR) and png_compress_IDAT then does
    // png_error(png_ptr, png_ptr->zstream.msg).
    for (tag, method, mem_level) in [
        ("method=9", Some(9i32), None),
        ("method=7", Some(7), None),
        ("mem_level=0", None, Some(0i32)),
        ("mem_level=10", None, Some(10)),
        ("ok", None, None),
    ] {
        let case = format!("C-20 deflate claim {}", tag);
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            let mut rng = Rng::new(0xC020);
            let img = Img::random(&mut rng, 4, 3, PNG_COLOR_TYPE_RGB, 8);
            let opts = WriteOpts {
                method,
                mem_level,
                ..Default::default()
            };
            let wr = write_plain(api, &img, &opts);
            o.push(format!("guard={:?} bytes={}", wr.guard, wr.bytes.len()));
            o.output = wr.bytes.clone();
            o
        });
    }

    // (b) read side: a zTXt whose deflate stream is damaged / truncated /
    // empty.  png_inflate calls png_zstream_error(png_ptr, ret) on the way out
    // and png_decompress_chunk reports that string.
    let base = handmade_gray1x1();
    let payloads: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("header only", vec![0x78, 0x9c]),
        ("truncated", vec![0x78, 0x9c, 0x4b, 0x4c]),
        ("bad header check", vec![0x08, 0x00, 0x01, 0x02, 0x03]),
        ("big window", vec![0x88, 0x9c, 0x01, 0x02]),
        ("garbage", vec![0x78, 0x9c, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        (
            "valid",
            vec![0x78, 0x01, 0x01, 0x02, 0x00, 0xfd, 0xff, b'h', b'i', 0x01, 0x1c, 0x00, 0xd2],
        ),
    ];
    for (tag, payload) in payloads {
        let mut data = b"Key".to_vec();
        data.push(0);
        data.push(0); // compression method
        data.extend_from_slice(&payload);
        let file = insert_before(&base, "IDAT", &chunk(b"zTXt", &data));
        let case = format!("C-20 zTXt {}", tag);
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            let rr = read_plain(api, &file, &ReadOpts::default());
            o.push(format!("guard={:?}", rr.guard));
            o
        });
    }

    // png_reset_zstream: NULL png_ptr, an uninitialised zstream and a zstream
    // that a real read has initialised.
    assert_same("C-20 png_reset_zstream", |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!(
            "reset_zstream(NULL)={}",
            (api.png_reset_zstream)(core::ptr::null_mut())
        ));
        let (png, info) = new_read(api);
        o.push(format!("fresh={}", (api.png_reset_zstream)(png)));
        o.push(format!("fresh again={}", (api.png_reset_zstream)(png)));
        destroy_read(api, png, info);

        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
        tls().input = gray_file(8);
        tls().in_pos = 0;
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            let rb = (api.png_get_rowbytes)(png, info);
            let mut row = vec![0u8; rb.max(1)];
            (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
            log(format!("mid-read reset={}", (api.png_reset_zstream)(png)));
        });
        o.push(format!("after read guard={:?}", g));
        o.push(format!("after read={}", (api.png_reset_zstream)(png)));
        destroy_read(api, png, info);
        o
    });
}

/* ================================================================== */
/* C-22  png_do_check_palette_indexes, png_get_palette_max              */
/* ================================================================== */

/// `png_do_check_palette_indexes` walks `png_ptr->row_buf`, which no caller
/// outside the library can set, so it is driven the way libpng itself does:
/// `png_write_row` calls it for every palette row, and `png_do_read_transformations`
/// does the same on the read side.  Its whole effect is `png_ptr->num_palette_max`,
/// which `png_get_palette_max` reports.
#[test]
fn palette_indexes() {
    for bd in [1i32, 2, 4, 8] {
        let full = 1usize << bd;
        let nps: Vec<usize> = match bd {
            1 => vec![1, 2],
            2 => vec![1, 2, 3, 4],
            4 => vec![1, 2, 5, 15, 16],
            _ => vec![1, 2, 17, 128, 255, 256],
        };
        for np in nps {
            for w in 1u32..=9 {
                for check in [0i32, 1] {
                    let mut rng =
                        Rng::new(0xC022 ^ ((bd as u64) << 32) ^ ((np as u64) << 16) ^ w as u64);
                    // Rows whose indices span the whole 0..2^bd range, so some
                    // are inside and some outside a palette of `np` entries.
                    let mut img = Img::random(&mut rng, w, 4, PNG_COLOR_TYPE_PALETTE, bd);
                    img.palette.truncate(np);

                    /* ---- write side ---- */
                    let case = format!(
                        "C-22 write bd={} np={} w={} check={}",
                        bd, np, w, check
                    );
                    assert_same(&case, |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = new_write(api);
                        (api.png_set_write_fn)(
                            png,
                            core::ptr::null_mut(),
                            Some(write_cb),
                            Some(flush_cb),
                        );
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                img.w,
                                img.h,
                                img.bit_depth,
                                img.color_type,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (api.png_set_PLTE)(
                                png,
                                info,
                                img.palette.as_ptr(),
                                img.palette.len() as c_int,
                            );
                            (api.png_set_check_for_invalid_index)(png, check);
                            (api.png_write_info)(png, info);
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                                log(format!(
                                    "palette_max={}",
                                    (api.png_get_palette_max)(png, info)
                                ));
                            }
                            (api.png_write_end)(png, info);
                        });
                        o.push(format!(
                            "guard={:?} palette_max={}",
                            g,
                            (api.png_get_palette_max)(png, info)
                        ));
                        o.output = std::mem::take(&mut tls().output);
                        destroy_write(api, png, info);
                        o
                    });

                    /* ---- read side ---- */
                    // Build a file with the full palette (so the writer is
                    // happy) and then shorten its PLTE, which is exactly the
                    // "index outside the palette" case on read.
                    let mut fullimg = img.clone();
                    fullimg.palette = (0..full)
                        .map(|i| png_color {
                            red: i as u8,
                            green: (i * 3) as u8,
                            blue: (i * 7) as u8,
                        })
                        .collect();
                    let file = build_with_c(&fullimg, &WriteOpts::default());
                    let short_plte: Vec<u8> = fullimg.palette[..np]
                        .iter()
                        .flat_map(|c| [c.red, c.green, c.blue])
                        .collect();
                    let patched = {
                        let chunks = split_chunks(&file);
                        let r = chunks
                            .iter()
                            .find(|(n, _)| n == "PLTE")
                            .map(|(_, r)| r.clone())
                            .expect("PLTE");
                        let mut v = file[..r.start].to_vec();
                        v.extend_from_slice(&chunk(b"PLTE", &short_plte));
                        v.extend_from_slice(&file[r.end..]);
                        v
                    };
                    let case = format!("C-22 read bd={} np={} w={} check={}", bd, np, w, check);
                    assert_same(&case, |api| unsafe {
                        let mut o = Outcome::default();
                        let rr = read_image(
                            api,
                            &patched,
                            &ReadOpts {
                                rows: RowMode::Row,
                                ..Default::default()
                            },
                            &mut |api, png, info| {
                                (api.png_set_check_for_invalid_index)(png, check);
                                log(format!(
                                    "before rows palette_max={}",
                                    (api.png_get_palette_max)(png, info)
                                ));
                            },
                        );
                        o.push(format!("guard={:?}", rr.guard));
                        for r in &rr.rows {
                            o.output.extend_from_slice(r);
                        }
                        o
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* C-24  png_convert_to_rfc1123_buffer, png_convert_from_time_t,        */
/*       png_convert_from_struct_tm, png_convert_to_rfc1123             */
/* ================================================================== */

#[test]
fn time_conv() {
    let mut times: Vec<png_time> = Vec::new();
    // representative valid values and every out-of-range field
    for t in [
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 1999, month: 12, day: 31, hour: 23, minute: 59, second: 59 },
        png_time { year: 0, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 9999, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 10000, month: 6, day: 15, hour: 12, minute: 30, second: 30 },
        png_time { year: 2000, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 255, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 255, hour: 0, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 65535, month: 255, day: 255, hour: 255, minute: 255, second: 255 },
        png_time { year: 1, month: 1, day: 1, hour: 1, minute: 1, second: 1 },
        png_time { year: 999, month: 9, day: 9, hour: 9, minute: 9, second: 9 },
        png_time { year: 100, month: 10, day: 10, hour: 10, minute: 10, second: 10 },
    ] {
        times.push(t);
    }
    let mut rng = Rng::new(0xC024);
    for _ in 0..600 {
        times.push(png_time {
            year: (rng.u32() % 12000) as u16,
            month: rng.u8() % 15,
            day: rng.u8() % 35,
            hour: rng.u8() % 26,
            minute: rng.u8() % 63,
            second: rng.u8() % 63,
        });
    }
    for _ in 0..200 {
        times.push(png_time {
            year: rng.u32() as u16,
            month: rng.u8(),
            day: rng.u8(),
            hour: rng.u8(),
            minute: rng.u8(),
            second: rng.u8(),
        });
    }
    // every month index, valid and one past the end
    for m in 0u8..=13 {
        times.push(png_time { year: 2001, month: m, day: 9, hour: 5, minute: 6, second: 7 });
    }

    assert_same("C-24 png_convert_to_rfc1123_buffer", |api| unsafe {
        let mut o = Outcome::default();
        for t in &times {
            // 64 bytes of a known pattern: the first 29 are the contract, the
            // rest catches an overrun.
            let mut buf = [0x5au8; 64];
            let r = (api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, t);
            o.push(format!("rfc1123_buffer({:?})={} {:02x?}", t, r, buf));
        }
        // out == NULL returns 0 without touching anything.
        let t = times[0];
        o.push(format!(
            "rfc1123_buffer(NULL)={}",
            (api.png_convert_to_rfc1123_buffer)(core::ptr::null_mut(), &t)
        ));
        o
    });

    assert_same("C-24 png_convert_to_rfc1123", |api| unsafe {
        let mut o = Outcome::default();
        for t in times.iter().take(120) {
            let (png, info) = new_read(api);
            let mut s = String::new();
            let g = guarded(api, png, &mut || {
                let p = (api.png_convert_to_rfc1123)(png, t);
                s = if p.is_null() {
                    "<null>".to_string()
                } else {
                    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
                };
            });
            o.push(format!("rfc1123({:?}) guard={:?} {:?}", t, g, s));
            destroy_read(api, png, info);
        }
        // png_ptr == NULL returns NULL.
        let t = times[0];
        let p = (api.png_convert_to_rfc1123)(core::ptr::null_mut(), &t);
        o.push(format!("rfc1123(NULL png)={}", p.is_null()));
        o
    });

    assert_same("C-24 png_convert_from_time_t", |api| unsafe {
        let mut o = Outcome::default();
        let mut ts: Vec<i64> = vec![
            0,
            1,
            -1,
            86399,
            86400,
            951_782_400,
            1_700_000_000,
            2_147_483_647,
            2_147_483_648,
            4_294_967_295,
            4_294_967_296,
            -2_147_483_648,
            i64::MAX,
            i64::MIN,
            67_768_036_191_676_799,
            67_768_036_191_676_800,
        ];
        let mut rng = Rng::new(0xC024_0F);
        for _ in 0..300 {
            ts.push(match rng.below(3) {
                0 => rng.range(0, 4_000_000_000) as i64,
                1 => rng.u32() as i64,
                _ => rng.next_u64() as i64,
            });
        }
        for t in &ts {
            let mut pt = png_time {
                year: 0x5a5a,
                month: 0x5a,
                day: 0x5a,
                hour: 0x5a,
                minute: 0x5a,
                second: 0x5a,
            };
            (api.png_convert_from_time_t)(&mut pt, *t);
            o.push(format!("from_time_t({})={:?}", t, pt));
        }
        o
    });

    assert_same("C-24 png_convert_from_struct_tm", |api| unsafe {
        let mut o = Outcome::default();
        let mut rng = Rng::new(0xC024_1F);
        let mut tms: Vec<Tm> = Vec::new();
        for (sec, min, hour, mday, mon, year) in [
            (0i32, 0, 0, 1, 0, 70),
            (59, 59, 23, 31, 11, 124),
            (60, 60, 24, 32, 12, 0),
            (-1, -1, -1, -1, -1, -1900),
            (61, 0, 0, 1, 0, 8100),
            (0, 0, 0, 255, 255, 255),
            (0, 0, 0, 256, 256, -1),
        ] {
            tms.push(Tm {
                tm_sec: sec,
                tm_min: min,
                tm_hour: hour,
                tm_mday: mday,
                tm_mon: mon,
                tm_year: year,
                ..Default::default()
            });
        }
        for _ in 0..400 {
            tms.push(Tm {
                tm_sec: rng.range(-100, 100) as c_int,
                tm_min: rng.range(-100, 100) as c_int,
                tm_hour: rng.range(-100, 100) as c_int,
                tm_mday: rng.range(-100, 400) as c_int,
                tm_mon: rng.range(-20, 300) as c_int,
                tm_year: rng.range(-2000, 9000) as c_int,
                ..Default::default()
            });
        }
        for t in &tms {
            let mut pt = png_time {
                year: 0x5a5a,
                month: 0x5a,
                day: 0x5a,
                hour: 0x5a,
                minute: 0x5a,
                second: 0x5a,
            };
            (api.png_convert_from_struct_tm)(&mut pt, t);
            o.push(format!(
                "from_tm({},{},{},{},{},{})={:?}",
                t.tm_sec, t.tm_min, t.tm_hour, t.tm_mday, t.tm_mon, t.tm_year, pt
            ));
        }
        o
    });
}

/* keep the unused-import lint quiet: c_void is only needed by some cfgs */
const _: Option<*mut c_void> = None;
