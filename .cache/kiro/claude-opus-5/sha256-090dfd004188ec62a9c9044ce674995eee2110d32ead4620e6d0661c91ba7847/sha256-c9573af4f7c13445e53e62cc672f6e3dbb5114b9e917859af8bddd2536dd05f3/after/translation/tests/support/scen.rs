//! Scenario registry.  A scenario is identified by a string of the form
//! `kind|key=value|key=value`; the same string is passed to the child process
//! for each library, so both sides run bit-identical code.

use super::mkpng::{self, PngBuilder};
use super::pngdefs::*;
use super::rng::Rng;
use super::{api, cb_error, cb_flush, cb_read, cb_warn, cb_write, g, rec};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_void, CString};

/* ------------------------------------------------------------------ */
/* name parsing                                                        */
/* ------------------------------------------------------------------ */

pub struct Args {
    pub kind: String,
    pub m: BTreeMap<String, String>,
}

impl Args {
    pub fn parse(name: &str) -> Args {
        let mut it = name.split('|');
        let kind = it.next().unwrap_or("").to_string();
        let mut m = BTreeMap::new();
        for kv in it {
            if kv.is_empty() {
                continue;
            }
            let mut p = kv.splitn(2, '=');
            let k = p.next().unwrap().to_string();
            let v = p.next().unwrap_or("").to_string();
            m.insert(k, v);
        }
        Args { kind, m }
    }
    pub fn s(&self, k: &str, d: &str) -> String {
        self.m.get(k).cloned().unwrap_or_else(|| d.to_string())
    }
    pub fn i(&self, k: &str, d: i64) -> i64 {
        self.m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    pub fn u(&self, k: &str, d: u32) -> u32 {
        self.m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    pub fn has(&self, k: &str) -> bool {
        self.m.contains_key(k)
    }
}

pub fn lookup(name: &str) -> Option<Box<dyn Fn()>> {
    let a = Args::parse(name);
    let f: fn(&Args) = match a.kind.as_str() {
        "util" => run_util,
        "sg" => run_setget,
        "wr" => run_write,
        "rd" => run_read,
        "prog" => run_progressive,
        "sr" => run_simple_read,
        "sw" => run_simple_write,
        "err" => super::errscen::run_err,
        "lim" => run_limits,
        "unk" => run_unknown,
        "ut" => run_usertransform,
        "mng" => run_mng,
        "crc" => run_crc,
        "fpget" => run_fpget,
        "fileio" => run_fileio,
        "freedata" => run_freedata,
        "heur" => run_heuristics,
        "mut" => run_mutate,
        "sfuzz" => run_simple_fuzz,
        _ => return None,
    };
    Some(Box::new(move || f(&a)))
}

/* ------------------------------------------------------------------ */
/* small helpers                                                       */
/* ------------------------------------------------------------------ */

pub fn ver() -> *const c_char {
    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char
}

/// Create a write struct wired to the recording callbacks.
pub unsafe fn new_write() -> (PngPtr, InfoPtr) {
    let a = api();
    let png = (a.png_create_write_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
    assert!(!png.is_null());
    let info = (a.png_create_info_struct)(png);
    assert!(!info.is_null());
    (a.png_set_write_fn)(png, 0x1234 as *mut c_void, Some(cb_write), Some(cb_flush));
    g().wbuf.clear();
    g().flushes = 0;
    (png, info)
}

/// Create a read struct fed from `src`.
pub unsafe fn new_read(src: &[u8]) -> (PngPtr, InfoPtr, InfoPtr) {
    let a = api();
    let png = (a.png_create_read_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
    assert!(!png.is_null());
    let info = (a.png_create_info_struct)(png);
    let end = (a.png_create_info_struct)(png);
    assert!(!info.is_null() && !end.is_null());
    g().rbuf = src.to_vec();
    g().rpos = 0;
    (a.png_set_read_fn)(png, 0x5678 as *mut c_void, Some(cb_read));
    (png, info, end)
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

/* ------------------------------------------------------------------ */
/* pure utility functions                                              */
/* ------------------------------------------------------------------ */

fn run_util(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    match a.s("f", "").as_str() {
        "version" => unsafe {
            r.kv("access_version_number", (api.png_access_version_number)());
            r.cstr("copyright", (api.png_get_copyright)(std::ptr::null_mut()));
            r.cstr("header_ver", (api.png_get_header_ver)(std::ptr::null_mut()));
            r.cstr("header_version", (api.png_get_header_version)(std::ptr::null_mut()));
            r.cstr("libpng_ver", (api.png_get_libpng_ver)(std::ptr::null_mut()));
        },
        "sigcmp" => unsafe {
            // every start/num combination over the real signature and mutations
            let mut rng = Rng::new(seed);
            for trial in 0..64 {
                let mut sig = mkpng::SIG;
                if trial > 0 {
                    let idx = rng.below(8) as usize;
                    sig[idx] ^= 1 << rng.below(8);
                }
                for start in 0..10usize {
                    for num in 0..10usize {
                        let v = (api.png_sig_cmp)(sig.as_ptr(), start, num);
                        r.kv(&format!("sigcmp/{trial}/{start}/{num}"), v);
                    }
                }
            }
            // NULL pointer with zero length
            r.kv("sigcmp/null0", (api.png_sig_cmp)(std::ptr::null(), 0, 0));
        },
        "intfns" => unsafe {
            let mut rng = Rng::new(seed);
            for i in 0..512 {
                let mut b = [0u8; 4];
                rng.fill(&mut b);
                if i < 8 {
                    // boundary patterns
                    b = match i {
                        0 => [0, 0, 0, 0],
                        1 => [0xff, 0xff, 0xff, 0xff],
                        2 => [0x80, 0, 0, 0],
                        3 => [0x7f, 0xff, 0xff, 0xff],
                        4 => [0x80, 0, 0, 1],
                        5 => [0, 0, 0, 1],
                        6 => [0xff, 0xff, 0xff, 0xfe],
                        _ => [0x81, 0x02, 0x03, 0x04],
                    };
                }
                r.kv(&format!("u32/{i}"), (api.png_get_uint_32)(b.as_ptr()));
                r.kv(&format!("u16/{i}"), (api.png_get_uint_16)(b.as_ptr()));
                r.kv(&format!("i32/{i}"), (api.png_get_int_32)(b.as_ptr()));
                let mut o = [0u8; 4];
                (api.png_save_uint_32)(o.as_mut_ptr(), u32::from_be_bytes(b));
                r.bytes(&format!("su32/{i}"), &o);
                (api.png_save_int_32)(o.as_mut_ptr(), i32::from_be_bytes(b));
                r.bytes(&format!("si32/{i}"), &o);
                let mut o2 = [0u8; 2];
                (api.png_save_uint_16)(o2.as_mut_ptr(), (u32::from_be_bytes(b) & 0xffff) as c_int);
                r.bytes(&format!("su16/{i}"), &o2);
            }
        },
        "uint31" => unsafe {
            // png_get_uint_31 errors on values > 0x7fffffff, so use a live struct
            let png = (api.png_create_read_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
            let mut rng = Rng::new(seed);
            for i in 0..256 {
                let mut b = [0u8; 4];
                rng.fill(&mut b);
                b[0] &= 0x7f; // stay in range
                r.kv(&format!("u31/{i}"), (api.png_get_uint_31)(png, b.as_ptr()));
            }
            r.kv("u31/max", (api.png_get_uint_31)(png, [0x7f, 0xff, 0xff, 0xff].as_ptr()));
            r.kv("u31/zero", (api.png_get_uint_31)(png, [0, 0, 0, 0].as_ptr()));
            let mut p = png;
            (api.png_destroy_read_struct)(&mut p, std::ptr::null_mut(), std::ptr::null_mut());
        },
        "graypal" => unsafe {
            for bd in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, -1] {
                let mut pal = vec![png_color::default(); 256];
                (api.png_build_grayscale_palette)(bd, pal.as_mut_ptr());
                let flat: Vec<u8> = pal.iter().flat_map(|c| [c.red, c.green, c.blue]).collect();
                r.digest(&format!("graypal/{bd}"), &flat);
                r.bytes(&format!("graypal_head/{bd}"), &flat[..24]);
            }
            // NULL palette must be a no-op, not a crash
            (api.png_build_grayscale_palette)(8, std::ptr::null_mut());
            r.line("graypal/null ok");
        },
        "rfc1123" => unsafe {
            let mut rng = Rng::new(seed);
            for i in 0..256 {
                let t = if i < 8 {
                    match i {
                        0 => png_time { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
                        1 => png_time { year: 2024, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
                        2 => png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
                        3 => png_time { year: 65535, month: 13, day: 32, hour: 24, minute: 60, second: 61 },
                        4 => png_time { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
                        5 => png_time { year: 2000, month: 2, day: 29, hour: 12, minute: 30, second: 30 },
                        6 => png_time { year: 9999, month: 12, day: 1, hour: 1, minute: 1, second: 1 },
                        _ => png_time { year: 1, month: 1, day: 1, hour: 1, minute: 1, second: 1 },
                    }
                } else {
                    png_time {
                        year: rng.next_u32() as u16,
                        month: rng.byte(),
                        day: rng.byte(),
                        hour: rng.byte(),
                        minute: rng.byte(),
                        second: rng.byte(),
                    }
                };
                let mut buf = [0i8; 40];
                let ok = (api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, &t);
                let bytes: Vec<u8> = buf.iter().map(|&c| c as u8).collect();
                let n = bytes.iter().position(|&c| c == 0).unwrap_or(40);
                r.kv(
                    &format!("rfc/{i}"),
                    format!("{ok} {:?}", String::from_utf8_lossy(&bytes[..n])),
                );
            }
            // NULL time
            let mut buf = [0i8; 40];
            r.kv(
                "rfc/nulltime",
                (api.png_convert_to_rfc1123_buffer)(buf.as_mut_ptr() as *mut c_char, std::ptr::null()),
            );
            // NULL out buffer
            let t = png_time { year: 2000, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
            r.kv(
                "rfc/nullout",
                (api.png_convert_to_rfc1123_buffer)(std::ptr::null_mut(), &t),
            );
        },
        "timet" => unsafe {
            let mut rng = Rng::new(seed);
            for i in 0..128 {
                let t: i64 = if i < 6 {
                    [0i64, 1, 946684800, 2147483647, 1000000000, 253402300799][i]
                } else {
                    (rng.next_u32() as i64) % 4_000_000_000
                };
                let mut pt = png_time::default();
                (api.png_convert_from_time_t)(&mut pt, t);
                r.kv(
                    &format!("timet/{i}"),
                    format!("{} {} {} {} {} {}", pt.year, pt.month, pt.day, pt.hour, pt.minute, pt.second),
                );
            }
        },
        other => panic!("unknown util f={other}"),
    }
}

/* ------------------------------------------------------------------ */
/* info setter / getter round trips                                    */
/* ------------------------------------------------------------------ */

fn run_setget(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    let group = a.s("g", "");
    unsafe {
        let (png, info) = new_write();
        let mut rng = Rng::new(seed);
        match group.as_str() {
            "ihdr" => {
                let cts = [0i32, 2, 3, 4, 6];
                for i in 0..200 {
                    let ct = rng.pick(&cts);
                    let bds: &[i32] = match ct {
                        0 => &[1, 2, 4, 8, 16],
                        3 => &[1, 2, 4, 8],
                        _ => &[8, 16],
                    };
                    let bd = rng.pick(bds);
                    let w = rng.range(1, 64);
                    let h = rng.range(1, 64);
                    let il = rng.below(2) as i32;
                    (api.png_set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                    let mut ow = 0u32;
                    let mut oh = 0u32;
                    let (mut obd, mut oct, mut oil, mut ocm, mut ofm) = (0, 0, 0, 0, 0);
                    let ret = (api.png_get_IHDR)(
                        png, info, &mut ow, &mut oh, &mut obd, &mut oct, &mut oil, &mut ocm, &mut ofm,
                    );
                    r.kv(
                        &format!("ihdr/{i}"),
                        format!("{ret} {ow} {oh} {obd} {oct} {oil} {ocm} {ofm}"),
                    );
                    r.kv(&format!("rowbytes/{i}"), (api.png_get_rowbytes)(png, info));
                    r.kv(&format!("channels/{i}"), (api.png_get_channels)(png, info));
                    r.kv(&format!("w/{i}"), (api.png_get_image_width)(png, info));
                    r.kv(&format!("h/{i}"), (api.png_get_image_height)(png, info));
                    r.kv(&format!("bd/{i}"), (api.png_get_bit_depth)(png, info));
                    r.kv(&format!("ct/{i}"), (api.png_get_color_type)(png, info));
                    r.kv(&format!("ft/{i}"), (api.png_get_filter_type)(png, info));
                    r.kv(&format!("it/{i}"), (api.png_get_interlace_type)(png, info));
                    r.kv(&format!("ctp/{i}"), (api.png_get_compression_type)(png, info));
                }
                // partial-output pointers (all NULL) must still return the flag
                r.kv(
                    "ihdr/nullouts",
                    (api.png_get_IHDR)(
                        png,
                        info,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    ),
                );
            }
            "gama" => {
                for i in 0..300 {
                    let v = match i % 6 {
                        0 => rng.next_u32() as i32 % 1_000_000,
                        1 => 0,
                        2 => 1,
                        3 => 100_000,
                        4 => PNG_FP_MAX,
                        _ => -(rng.next_u32() as i32 % 100_000),
                    };
                    (api.png_set_gAMA_fixed)(png, info, v);
                    let mut got = 0i32;
                    let ret = (api.png_get_gAMA_fixed)(png, info, &mut got);
                    let mut gd = 0f64;
                    let retd = (api.png_get_gAMA)(png, info, &mut gd);
                    r.kv(&format!("gama/{i}"), format!("{v} -> {ret} {got} / {retd} {gd:.9}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_gAMA as c_int);
                }
                for i in 0..100 {
                    let d = (rng.next_u32() as f64) / 1e6;
                    (api.png_set_gAMA)(png, info, d);
                    let mut got = 0i32;
                    let ret = (api.png_get_gAMA_fixed)(png, info, &mut got);
                    r.kv(&format!("gamad/{i}"), format!("{d:.9} -> {ret} {got}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_gAMA as c_int);
                }
            }
            "chrm" => {
                for i in 0..200 {
                    let mut v = [0i32; 8];
                    for x in v.iter_mut() {
                        *x = (rng.next_u32() % 200_000) as i32;
                    }
                    if i % 7 == 0 {
                        v = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
                    }
                    (api.png_set_cHRM_fixed)(png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
                    let mut o = [0i32; 8];
                    let ret = (api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7],
                    );
                    r.kv(&format!("chrm/{i}"), format!("{ret} {o:?}"));
                    let mut x = [0i32; 9];
                    let ret2 = (api.png_get_cHRM_XYZ_fixed)(
                        png, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5],
                        &mut x[6], &mut x[7], &mut x[8],
                    );
                    r.kv(&format!("chrmxyz/{i}"), format!("{ret2} {x:?}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_cHRM as c_int);
                }
                for i in 0..100 {
                    let mut v = [0i32; 9];
                    for x in v.iter_mut() {
                        *x = (rng.next_u32() % 300_000) as i32;
                    }
                    (api.png_set_cHRM_XYZ_fixed)(
                        png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8],
                    );
                    let mut o = [0i32; 8];
                    let ret = (api.png_get_cHRM_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7],
                    );
                    r.kv(&format!("chrmset/{i}"), format!("{ret} {o:?}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_cHRM as c_int);
                }
            }
            "plte" => {
                for i in 0..100 {
                    let n = rng.range(1, 256) as i32;
                    let mut pal = vec![png_color::default(); n as usize];
                    for p in pal.iter_mut() {
                        p.red = rng.byte();
                        p.green = rng.byte();
                        p.blue = rng.byte();
                    }
                    (api.png_set_IHDR)(png, info, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                    (api.png_set_PLTE)(png, info, pal.as_ptr(), n);
                    let mut got: *mut png_color = std::ptr::null_mut();
                    let mut gn = 0i32;
                    let ret = (api.png_get_PLTE)(png, info, &mut got, &mut gn);
                    let flat: Vec<u8> = if got.is_null() {
                        Vec::new()
                    } else {
                        (0..gn as usize)
                            .flat_map(|k| {
                                let c = *got.add(k);
                                [c.red, c.green, c.blue]
                            })
                            .collect()
                    };
                    r.kv(&format!("plte/{i}"), format!("{ret} {gn}"));
                    r.digest(&format!("plted/{i}"), &flat);
                    r.kv(&format!("pltemax/{i}"), (api.png_get_palette_max)(png, info));
                }
            }
            "trns" => {
                for i in 0..120 {
                    let n = rng.range(1, 256) as i32;
                    let mut al = vec![0u8; n as usize];
                    rng.fill(&mut al);
                    let c16 = png_color_16 {
                        index: rng.byte(),
                        red: rng.next_u32() as u16,
                        green: rng.next_u32() as u16,
                        blue: rng.next_u32() as u16,
                        gray: rng.next_u32() as u16,
                    };
                    let ct = rng.pick(&[0i32, 2, 3, 4, 6]);
                    (api.png_set_IHDR)(png, info, 8, 8, 8, ct, 0, 0, 0);
                    if ct == 3 {
                        let pal = vec![png_color::default(); 256];
                        (api.png_set_PLTE)(png, info, pal.as_ptr(), 256);
                    }
                    (api.png_set_tRNS)(png, info, al.as_ptr(), n, &c16);
                    let mut ga: *mut u8 = std::ptr::null_mut();
                    let mut gn = 0i32;
                    let mut gc: *mut png_color_16 = std::ptr::null_mut();
                    let ret = (api.png_get_tRNS)(png, info, &mut ga, &mut gn, &mut gc);
                    r.kv(&format!("trns/{i}"), format!("ct={ct} n={n} ret={ret} gn={gn}"));
                    if !ga.is_null() && gn > 0 {
                        r.digest(&format!("trnsa/{i}"), std::slice::from_raw_parts(ga, gn as usize));
                    }
                    if !gc.is_null() {
                        let c = *gc;
                        r.kv(
                            &format!("trnsc/{i}"),
                            format!("{} {} {} {} {}", c.index, c.red, c.green, c.blue, c.gray),
                        );
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_tRNS as c_int);
                }
            }
            "misc" => {
                for i in 0..80 {
                    let rx = rng.next_u32() % 100000;
                    let ry = rng.next_u32() % 100000;
                    let ut = (rng.below(4)) as c_int;
                    (api.png_set_pHYs)(png, info, rx, ry, ut);
                    let (mut a1, mut a2, mut a3) = (0u32, 0u32, 0i32);
                    let ret = (api.png_get_pHYs)(png, info, &mut a1, &mut a2, &mut a3);
                    r.kv(&format!("phys/{i}"), format!("{ret} {a1} {a2} {a3}"));
                    r.kv(&format!("ppm/{i}"), (api.png_get_pixels_per_meter)(png, info));
                    r.kv(&format!("xppm/{i}"), (api.png_get_x_pixels_per_meter)(png, info));
                    r.kv(&format!("yppm/{i}"), (api.png_get_y_pixels_per_meter)(png, info));
                    r.kv(&format!("par/{i}"), (api.png_get_pixel_aspect_ratio_fixed)(png, info));
                    r.kv(&format!("ppi/{i}"), (api.png_get_pixels_per_inch)(png, info));
                    r.kv(&format!("xppi/{i}"), (api.png_get_x_pixels_per_inch)(png, info));
                    r.kv(&format!("yppi/{i}"), (api.png_get_y_pixels_per_inch)(png, info));
                    let (mut d1, mut d2, mut d3) = (0u32, 0u32, 0i32);
                    r.kv(
                        &format!("physdpi/{i}"),
                        format!("{} {d1} {d2} {d3}", (api.png_get_pHYs_dpi)(png, info, &mut d1, &mut d2, &mut d3)),
                    );

                    let ox = rng.next_u32() as i32;
                    let oy = rng.next_u32() as i32;
                    let out = (rng.below(3)) as c_int;
                    (api.png_set_oFFs)(png, info, ox, oy, out);
                    let (mut b1, mut b2, mut b3) = (0i32, 0i32, 0i32);
                    let ret2 = (api.png_get_oFFs)(png, info, &mut b1, &mut b2, &mut b3);
                    r.kv(&format!("offs/{i}"), format!("{ret2} {b1} {b2} {b3}"));
                    r.kv(&format!("xop/{i}"), (api.png_get_x_offset_pixels)(png, info));
                    r.kv(&format!("yop/{i}"), (api.png_get_y_offset_pixels)(png, info));
                    r.kv(&format!("xom/{i}"), (api.png_get_x_offset_microns)(png, info));
                    r.kv(&format!("yom/{i}"), (api.png_get_y_offset_microns)(png, info));
                    r.kv(&format!("xoi/{i}"), (api.png_get_x_offset_inches_fixed)(png, info));
                    r.kv(&format!("yoi/{i}"), (api.png_get_y_offset_inches_fixed)(png, info));

                    let sb = png_color_8 {
                        red: rng.byte() % 17,
                        green: rng.byte() % 17,
                        blue: rng.byte() % 17,
                        gray: rng.byte() % 17,
                        alpha: rng.byte() % 17,
                    };
                    (api.png_set_IHDR)(png, info, 8, 8, 8, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0, 0);
                    (api.png_set_sBIT)(png, info, &sb);
                    let mut gsb: *mut png_color_8 = std::ptr::null_mut();
                    let ret3 = (api.png_get_sBIT)(png, info, &mut gsb);
                    if !gsb.is_null() {
                        let c = *gsb;
                        r.kv(
                            &format!("sbit/{i}"),
                            format!("{ret3} {} {} {} {} {}", c.red, c.green, c.blue, c.gray, c.alpha),
                        );
                    } else {
                        r.kv(&format!("sbit/{i}"), format!("{ret3} none"));
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_sBIT as c_int);

                    let intent = (rng.below(6)) as c_int;
                    (api.png_set_sRGB)(png, info, intent);
                    let mut gi = 0i32;
                    r.kv(
                        &format!("srgb/{i}"),
                        format!("{intent} -> {} {gi}", {
                            let v = (api.png_get_sRGB)(png, info, &mut gi);
                            v
                        }),
                    );
                    (api.png_set_invalid)(png, info, PNG_INFO_sRGB as c_int);

                    let t = png_time {
                        year: rng.next_u32() as u16 % 3000,
                        month: rng.byte() % 14,
                        day: rng.byte() % 33,
                        hour: rng.byte() % 25,
                        minute: rng.byte() % 61,
                        second: rng.byte() % 62,
                    };
                    (api.png_set_tIME)(png, info, &t);
                    let mut gt: *mut png_time = std::ptr::null_mut();
                    let ret4 = (api.png_get_tIME)(png, info, &mut gt);
                    if !gt.is_null() {
                        let v = *gt;
                        r.kv(
                            &format!("time/{i}"),
                            format!("{ret4} {} {} {} {} {} {}", v.year, v.month, v.day, v.hour, v.minute, v.second),
                        );
                    } else {
                        r.kv(&format!("time/{i}"), format!("{ret4} none"));
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_tIME as c_int);
                }
            }
            "scal" => {
                for i in 0..80 {
                    let unit = (rng.below(4)) as c_int;
                    let w = format!("{}.{}", rng.below(1000), rng.below(100000));
                    let h = format!("{}e{}", rng.below(1000), rng.below(10));
                    let cw = cs(&w);
                    let ch = cs(&h);
                    (api.png_set_sCAL_s)(png, info, unit, cw.as_ptr(), ch.as_ptr());
                    let mut gu = 0i32;
                    let mut gw: *mut c_char = std::ptr::null_mut();
                    let mut gh: *mut c_char = std::ptr::null_mut();
                    let ret = (api.png_get_sCAL_s)(png, info, &mut gu, &mut gw, &mut gh);
                    r.kv(&format!("scals/{i}"), format!("{ret} {gu}"));
                    r.cstr(&format!("scalw/{i}"), gw);
                    r.cstr(&format!("scalh/{i}"), gh);
                    let mut fu = 0i32;
                    let mut fw = 0i32;
                    let mut fh = 0i32;
                    let retf = (api.png_get_sCAL_fixed)(png, info, &mut fu, &mut fw, &mut fh);
                    r.kv(&format!("scalfx/{i}"), format!("{retf} {fu} {fw} {fh}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_sCAL as c_int);
                }
                for i in 0..40 {
                    let unit = 1 + (rng.below(2)) as c_int;
                    let fw = (rng.next_u32() % 1_000_000) as i32 + 1;
                    let fh = (rng.next_u32() % 1_000_000) as i32 + 1;
                    (api.png_set_sCAL_fixed)(png, info, unit, fw, fh);
                    let mut gu = 0i32;
                    let mut gw: *mut c_char = std::ptr::null_mut();
                    let mut gh: *mut c_char = std::ptr::null_mut();
                    let ret = (api.png_get_sCAL_s)(png, info, &mut gu, &mut gw, &mut gh);
                    r.kv(&format!("scalfixed/{i}"), format!("{ret} {gu}"));
                    r.cstr(&format!("scalfw/{i}"), gw);
                    r.cstr(&format!("scalfh/{i}"), gh);
                    (api.png_set_invalid)(png, info, PNG_INFO_sCAL as c_int);
                }
            }
            "newchunks" => {
                for i in 0..80 {
                    let cp = rng.byte();
                    let tf = rng.byte();
                    let mc = rng.byte();
                    let fr = rng.byte() % 2;
                    (api.png_set_cICP)(png, info, cp, tf, mc, fr);
                    let (mut a1, mut a2, mut a3, mut a4) = (0u8, 0u8, 0u8, 0u8);
                    let ret = (api.png_get_cICP)(png, info, &mut a1, &mut a2, &mut a3, &mut a4);
                    r.kv(&format!("cicp/{i}"), format!("{ret} {a1} {a2} {a3} {a4}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_cICP as c_int);

                    let m1 = rng.next_u32() & 0x7fffffff;
                    let m2 = rng.next_u32() & 0x7fffffff;
                    (api.png_set_cLLI_fixed)(png, info, m1, m2);
                    let (mut b1, mut b2) = (0u32, 0u32);
                    let ret2 = (api.png_get_cLLI_fixed)(png, info, &mut b1, &mut b2);
                    r.kv(&format!("clli/{i}"), format!("{ret2} {b1} {b2}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_cLLI as c_int);

                    let mut v = [0i32; 8];
                    for x in v.iter_mut() {
                        *x = (rng.next_u32() % 131_000) as i32;
                    }
                    let l1 = rng.next_u32() & 0x7fffffff;
                    let l2 = rng.next_u32() & 0x7fffffff;
                    (api.png_set_mDCV_fixed)(
                        png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], l1, l2,
                    );
                    let mut o = [0i32; 8];
                    let (mut o1, mut o2) = (0u32, 0u32);
                    let ret3 = (api.png_get_mDCV_fixed)(
                        png, info, &mut o[0], &mut o[1], &mut o[2], &mut o[3], &mut o[4], &mut o[5],
                        &mut o[6], &mut o[7], &mut o1, &mut o2,
                    );
                    r.kv(&format!("mdcv/{i}"), format!("{ret3} {o:?} {o1} {o2}"));
                    (api.png_set_invalid)(png, info, PNG_INFO_mDCV as c_int);
                }
            }
            "text" => {
                for i in 0..60 {
                    let key = format!("Key{}", i);
                    let txt: String = (0..rng.below(200)).map(|_| (0x20 + rng.byte() % 0x5f) as char).collect();
                    let ck = cs(&key);
                    let ct = cs(&txt);
                    let comp = rng.pick(&[-1i32, 0, 1, 2]);
                    let lang = cs("en");
                    let lk = cs("Key");
                    let t = png_text {
                        compression: comp,
                        key: ck.as_ptr() as *mut c_char,
                        text: ct.as_ptr() as *mut c_char,
                        text_length: 0,
                        itxt_length: 0,
                        lang: if comp >= 1 { lang.as_ptr() as *mut c_char } else { std::ptr::null_mut() },
                        lang_key: if comp >= 1 { lk.as_ptr() as *mut c_char } else { std::ptr::null_mut() },
                    };
                    (api.png_set_text)(png, info, &t, 1);
                    let mut gt: *mut png_text = std::ptr::null_mut();
                    let mut gn = 0i32;
                    let ret = (api.png_get_text)(png, info, &mut gt, &mut gn);
                    r.kv(&format!("text/{i}"), format!("{ret} {gn}"));
                    if !gt.is_null() && gn > 0 {
                        let last = *gt.add((gn - 1) as usize);
                        r.kv(&format!("textc/{i}"), last.compression);
                        r.cstr(&format!("textk/{i}"), last.key);
                        r.cstr(&format!("textt/{i}"), last.text);
                        r.kv(&format!("textlen/{i}"), last.text_length);
                        r.kv(&format!("textilen/{i}"), last.itxt_length);
                    }
                }
                (api.png_free_data)(png, info, PNG_FREE_TEXT, -1);
                let mut gt: *mut png_text = std::ptr::null_mut();
                let mut gn = 0i32;
                r.kv(
                    "text/afterfree",
                    format!("{} {gn}", (api.png_get_text)(png, info, &mut gt, &mut gn)),
                );
            }
            "iccp" => {
                for i in 0..40 {
                    let n = rng.range(132, 400) as usize;
                    let mut prof = vec![0u8; n];
                    rng.fill(&mut prof);
                    // plausible ICC header so png_set_iCCP's checks pass
                    prof[0..4].copy_from_slice(&(n as u32).to_be_bytes());
                    prof[8] = 2;
                    prof[9] = 0x10;
                    prof[12..16].copy_from_slice(b"mntr");
                    prof[16..20].copy_from_slice(b"RGB ");
                    prof[20..24].copy_from_slice(b"XYZ ");
                    prof[36..40].copy_from_slice(b"acsp");
                    let ntags = 1u32;
                    prof[128..132].copy_from_slice(&ntags.to_be_bytes());
                    let name = cs(&format!("ICC{i}"));
                    (api.png_set_iCCP)(png, info, name.as_ptr(), 0, prof.as_ptr(), n as u32);
                    let mut gname: *mut c_char = std::ptr::null_mut();
                    let mut gct = 0i32;
                    let mut gp: *mut u8 = std::ptr::null_mut();
                    let mut gl = 0u32;
                    let ret = (api.png_get_iCCP)(png, info, &mut gname, &mut gct, &mut gp, &mut gl);
                    r.kv(&format!("iccp/{i}"), format!("{ret} {gct} {gl}"));
                    r.cstr(&format!("iccpn/{i}"), gname);
                    if !gp.is_null() && gl > 0 {
                        r.digest(&format!("iccpd/{i}"), std::slice::from_raw_parts(gp, gl as usize));
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_iCCP as c_int);
                }
            }
            "splt" => {
                for i in 0..40 {
                    let n = rng.range(1, 32) as i32;
                    let mut ent = vec![png_sPLT_entry::default(); n as usize];
                    for e in ent.iter_mut() {
                        e.red = rng.next_u32() as u16;
                        e.green = rng.next_u32() as u16;
                        e.blue = rng.next_u32() as u16;
                        e.alpha = rng.next_u32() as u16;
                        e.frequency = rng.next_u32() as u16;
                    }
                    let name = cs(&format!("splt{i}"));
                    let s = png_sPLT_t {
                        name: name.as_ptr() as *mut c_char,
                        depth: if rng.bool() { 8 } else { 16 },
                        entries: ent.as_mut_ptr(),
                        nentries: n,
                    };
                    (api.png_set_sPLT)(png, info, &s, 1);
                    let mut got: *mut png_sPLT_t = std::ptr::null_mut();
                    let cnt = (api.png_get_sPLT)(png, info, &mut got);
                    r.kv(&format!("splt/{i}"), cnt);
                    if !got.is_null() && cnt > 0 {
                        let last = *got.add((cnt - 1) as usize);
                        r.cstr(&format!("spltn/{i}"), last.name);
                        r.kv(&format!("spltd/{i}"), format!("{} {}", last.depth, last.nentries));
                        let flat: Vec<u8> = (0..last.nentries as usize)
                            .flat_map(|k| {
                                let e = *last.entries.add(k);
                                let mut b = Vec::new();
                                b.extend_from_slice(&e.red.to_le_bytes());
                                b.extend_from_slice(&e.green.to_le_bytes());
                                b.extend_from_slice(&e.blue.to_le_bytes());
                                b.extend_from_slice(&e.alpha.to_le_bytes());
                                b.extend_from_slice(&e.frequency.to_le_bytes());
                                b
                            })
                            .collect();
                        r.digest(&format!("splte/{i}"), &flat);
                    }
                }
                (api.png_free_data)(png, info, PNG_FREE_SPLT, -1);
                let mut got: *mut png_sPLT_t = std::ptr::null_mut();
                r.kv("splt/afterfree", (api.png_get_sPLT)(png, info, &mut got));
            }
            "pcal" => {
                for i in 0..40 {
                    let purpose = cs(&format!("purpose{i}"));
                    let units = cs("units");
                    let nparams = rng.below(4) as i32;
                    let pstrings: Vec<CString> =
                        (0..nparams).map(|k| cs(&format!("{}.{}", k, rng.below(1000)))).collect();
                    let mut pptrs: Vec<*mut c_char> =
                        pstrings.iter().map(|s| s.as_ptr() as *mut c_char).collect();
                    let x0 = rng.next_u32() as i32;
                    let x1 = rng.next_u32() as i32;
                    let ty = rng.below(4) as c_int;
                    (api.png_set_pCAL)(
                        png,
                        info,
                        purpose.as_ptr(),
                        x0,
                        x1,
                        ty,
                        nparams,
                        units.as_ptr(),
                        if nparams > 0 { pptrs.as_mut_ptr() } else { std::ptr::null_mut() },
                    );
                    let mut gp: *mut c_char = std::ptr::null_mut();
                    let mut gx0 = 0i32;
                    let mut gx1 = 0i32;
                    let mut gty = 0i32;
                    let mut gnp = 0i32;
                    let mut gu: *mut c_char = std::ptr::null_mut();
                    let mut gparams: *mut *mut c_char = std::ptr::null_mut();
                    let ret = (api.png_get_pCAL)(
                        png, info, &mut gp, &mut gx0, &mut gx1, &mut gty, &mut gnp, &mut gu,
                        &mut gparams,
                    );
                    r.kv(&format!("pcal/{i}"), format!("{ret} {gx0} {gx1} {gty} {gnp}"));
                    r.cstr(&format!("pcalp/{i}"), gp);
                    r.cstr(&format!("pcalu/{i}"), gu);
                    for k in 0..gnp as usize {
                        if !gparams.is_null() {
                            r.cstr(&format!("pcalpar/{i}/{k}"), *gparams.add(k));
                        }
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_pCAL as c_int);
                }
            }
            "exif" => {
                for i in 0..40 {
                    let n = rng.range(8, 64);
                    let mut e = vec![0u8; n as usize];
                    rng.fill(&mut e);
                    if rng.bool() {
                        e[0] = b'I';
                        e[1] = b'I';
                    } else {
                        e[0] = b'M';
                        e[1] = b'M';
                    }
                    (api.png_set_eXIf_1)(png, info, n, e.as_mut_ptr());
                    let mut gn = 0u32;
                    let mut gp: *mut u8 = std::ptr::null_mut();
                    let ret = (api.png_get_eXIf_1)(png, info, &mut gn, &mut gp);
                    r.kv(&format!("exif/{i}"), format!("{ret} {gn}"));
                    if !gp.is_null() && gn > 0 {
                        r.digest(&format!("exifd/{i}"), std::slice::from_raw_parts(gp, gn as usize));
                    }
                }
            }
            "hist" => {
                for i in 0..40 {
                    let n = rng.range(1, 256) as i32;
                    let mut pal = vec![png_color::default(); n as usize];
                    for p in pal.iter_mut() {
                        p.red = rng.byte();
                    }
                    (api.png_set_IHDR)(png, info, 8, 8, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
                    (api.png_set_PLTE)(png, info, pal.as_ptr(), n);
                    let hist: Vec<u16> = (0..n as usize).map(|_| rng.next_u32() as u16).collect();
                    (api.png_set_hIST)(png, info, hist.as_ptr());
                    let mut gh: *mut u16 = std::ptr::null_mut();
                    let ret = (api.png_get_hIST)(png, info, &mut gh);
                    r.kv(&format!("hist/{i}"), format!("{ret} n={n}"));
                    if !gh.is_null() {
                        let flat: Vec<u8> = (0..n as usize)
                            .flat_map(|k| (*gh.add(k)).to_le_bytes())
                            .collect();
                        r.digest(&format!("histd/{i}"), &flat);
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_hIST as c_int);
                }
            }
            "bkgd" => {
                for i in 0..60 {
                    let ct = rng.pick(&[0i32, 2, 3, 4, 6]);
                    let bd = if ct == 3 { 8 } else { *rng.pick(&[&8i32, &16]) };
                    (api.png_set_IHDR)(png, info, 8, 8, bd, ct, 0, 0, 0);
                    if ct == 3 {
                        let pal = vec![png_color::default(); 16];
                        (api.png_set_PLTE)(png, info, pal.as_ptr(), 16);
                    }
                    let b = png_color_16 {
                        index: rng.byte(),
                        red: rng.next_u32() as u16,
                        green: rng.next_u32() as u16,
                        blue: rng.next_u32() as u16,
                        gray: rng.next_u32() as u16,
                    };
                    (api.png_set_bKGD)(png, info, &b);
                    let mut gb: *mut png_color_16 = std::ptr::null_mut();
                    let ret = (api.png_get_bKGD)(png, info, &mut gb);
                    if !gb.is_null() {
                        let v = *gb;
                        r.kv(
                            &format!("bkgd/{i}"),
                            format!("{ret} {} {} {} {} {}", v.index, v.red, v.green, v.blue, v.gray),
                        );
                    } else {
                        r.kv(&format!("bkgd/{i}"), format!("{ret} none"));
                    }
                    (api.png_set_invalid)(png, info, PNG_INFO_bKGD as c_int);
                }
            }
            other => panic!("unknown sg group {other}"),
        }
        r.kv("warn_flushes", g().flushes);
        let mut p = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut p, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* synthesised PNG sources                                             */
/* ------------------------------------------------------------------ */

/// A plausible, structurally valid minimal ICC profile that passes
/// `png_icc_check_header` / `png_icc_check_tag_table`, including the D50 PCS
/// illuminant libpng insists on (`png.c: D50_nCIEXYZ`).
pub fn make_icc(gray: bool) -> Vec<u8> {
    let total = 164usize;
    let mut p = vec![0u8; total];
    p[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    p[4..8].copy_from_slice(b"none");
    p[8..12].copy_from_slice(&0x0210_0000u32.to_be_bytes());
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(if gray { b"GRAY" } else { b"RGB " });
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p[64..68].copy_from_slice(&0u32.to_be_bytes());
    // PCS illuminant: D50 as an ICC XYZNumber.
    p[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    p[128..132].copy_from_slice(&1u32.to_be_bytes());
    p[132..136].copy_from_slice(b"wtpt");
    p[136..140].copy_from_slice(&144u32.to_be_bytes());
    p[140..144].copy_from_slice(&20u32.to_be_bytes());
    p[144..148].copy_from_slice(b"XYZ ");
    p[152..164].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    p
}

/// Generate the raw (unfiltered) pixel rows of a non-interlaced image.
fn gen_rows(w: u32, h: u32, bd: u8, ct: u8, npal: u32, rng: &mut Rng) -> Vec<Vec<u8>> {
    let rb = mkpng::rowbytes(w, bd, ct);
    let mut rows = Vec::with_capacity(h as usize);
    for _ in 0..h {
        let mut row = vec![0u8; rb];
        rng.fill(&mut row);
        if ct == 3 && bd == 8 && npal > 0 && npal < 256 {
            for b in row.iter_mut() {
                *b %= npal as u8;
            }
        }
        rows.push(row);
    }
    rows
}

pub struct Synth {
    pub png: Vec<u8>,
    pub npal: u32,
}

/// Build a complete PNG datastream.
pub fn synth(ct: u8, bd: u8, il: u8, w: u32, h: u32, extras: &str, split: usize, seed: u64) -> Synth {
    let mut rng = Rng::new(seed ^ 0xA5A5_1234);
    let npal: u32 = if ct == 3 {
        match bd {
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 1 + rng.below(256),
        }
    } else {
        0
    };

    let mut b = PngBuilder::new();
    b.ihdr(w, h, bd, ct, il);

    if extras.contains("gama") {
        b.chunk(b"gAMA", &45455u32.to_be_bytes());
    }
    if extras.contains("srgb") {
        b.chunk(b"sRGB", &[0u8]);
    }
    if extras.contains("chrm") {
        let vals: [u32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
        let mut d = Vec::new();
        for v in vals {
            d.extend_from_slice(&v.to_be_bytes());
        }
        b.chunk(b"cHRM", &d);
    }
    if extras.contains("iccp") {
        let prof = make_icc(ct == 0 || ct == 4);
        let mut d = Vec::new();
        d.extend_from_slice(b"ICC\0");
        d.push(0);
        d.extend_from_slice(&mkpng::zlib_stored(&prof));
        b.chunk(b"iCCP", &d);
    }
    if extras.contains("cicp") {
        b.chunk(b"cICP", &[9u8, 16, 0, 1]);
    }
    if extras.contains("clli") {
        let mut d = Vec::new();
        d.extend_from_slice(&10_000_000u32.to_be_bytes());
        d.extend_from_slice(&4_000_000u32.to_be_bytes());
        b.chunk(b"cLLI", &d);
    }
    if extras.contains("mdcv") {
        let mut d = Vec::new();
        for v in [31270u16, 32900, 64000, 33000, 30000, 60000, 15000, 6000] {
            d.extend_from_slice(&v.to_be_bytes());
        }
        d.extend_from_slice(&10_000_000u32.to_be_bytes());
        d.extend_from_slice(&50u32.to_be_bytes());
        b.chunk(b"mDCV", &d);
    }
    if extras.contains("sbit") {
        let n = match ct {
            0 => 1,
            2 | 3 => 3,
            4 => 2,
            _ => 4,
        };
        let d = vec![if bd == 16 { 13u8 } else { bd.max(1) }; n];
        b.chunk(b"sBIT", &d);
    }

    if ct == 3 {
        let mut d = Vec::with_capacity(npal as usize * 3);
        for _ in 0..npal {
            d.push(rng.byte());
            d.push(rng.byte());
            d.push(rng.byte());
        }
        b.chunk(b"PLTE", &d);
    } else if extras.contains("plte") {
        let mut d = Vec::new();
        for _ in 0..16 {
            d.push(rng.byte());
            d.push(rng.byte());
            d.push(rng.byte());
        }
        b.chunk(b"PLTE", &d);
    }

    if extras.contains("trns") {
        match ct {
            0 => {
                let v = (rng.next_u32() % (1u32 << bd.min(16) as u32)) as u16;
                b.chunk(b"tRNS", &v.to_be_bytes());
            }
            2 => {
                let mut d = Vec::new();
                for _ in 0..3 {
                    d.extend_from_slice(&((rng.next_u32() % (1u32 << bd as u32)) as u16).to_be_bytes());
                }
                b.chunk(b"tRNS", &d);
            }
            3 => {
                let n = 1 + rng.below(npal.max(1));
                let mut d = vec![0u8; n as usize];
                rng.fill(&mut d);
                b.chunk(b"tRNS", &d);
            }
            _ => {}
        }
    }
    if extras.contains("bkgd") {
        match ct {
            0 | 4 => {
                let v = (rng.next_u32() % (1u32 << bd.min(16) as u32)) as u16;
                b.chunk(b"bKGD", &v.to_be_bytes());
            }
            2 | 6 => {
                let mut d = Vec::new();
                for _ in 0..3 {
                    d.extend_from_slice(&((rng.next_u32() % (1u32 << bd as u32)) as u16).to_be_bytes());
                }
                b.chunk(b"bKGD", &d);
            }
            3 => {
                b.chunk(b"bKGD", &[(rng.below(npal.max(1))) as u8]);
            }
            _ => {}
        }
    }
    if extras.contains("hist") && ct == 3 {
        let mut d = Vec::new();
        for _ in 0..npal {
            d.extend_from_slice(&(rng.next_u32() as u16).to_be_bytes());
        }
        b.chunk(b"hIST", &d);
    }
    if extras.contains("phys") {
        let mut d = Vec::new();
        d.extend_from_slice(&2835u32.to_be_bytes());
        d.extend_from_slice(&2835u32.to_be_bytes());
        d.push(1);
        b.chunk(b"pHYs", &d);
    }
    if extras.contains("offs") {
        let mut d = Vec::new();
        d.extend_from_slice(&(-12i32).to_be_bytes());
        d.extend_from_slice(&34i32.to_be_bytes());
        d.push(1);
        b.chunk(b"oFFs", &d);
    }
    if extras.contains("scal") {
        let mut d = Vec::new();
        d.push(1);
        d.extend_from_slice(b"1.5\0");
        d.extend_from_slice(b"2.5");
        b.chunk(b"sCAL", &d);
    }
    if extras.contains("pcal") {
        let mut d = Vec::new();
        d.extend_from_slice(b"purpose\0");
        d.extend_from_slice(&0i32.to_be_bytes());
        d.extend_from_slice(&255i32.to_be_bytes());
        d.push(0);
        d.push(2);
        d.extend_from_slice(b"units\0");
        d.extend_from_slice(b"1.0\0");
        d.extend_from_slice(b"2.0");
        b.chunk(b"pCAL", &d);
    }
    if extras.contains("splt") {
        let mut d = Vec::new();
        d.extend_from_slice(b"sp\0");
        d.push(8);
        for _ in 0..4 {
            d.extend_from_slice(&[rng.byte(), rng.byte(), rng.byte(), rng.byte()]);
            d.extend_from_slice(&(rng.next_u32() as u16).to_be_bytes());
        }
        b.chunk(b"sPLT", &d);
    }
    if extras.contains("text") {
        b.chunk(b"tEXt", b"Title\0hello world");
        let mut z = Vec::new();
        z.extend_from_slice(b"Comment\0");
        z.push(0);
        z.extend_from_slice(&mkpng::zlib_stored(b"compressed comment text"));
        b.chunk(b"zTXt", &z);
        let mut it = Vec::new();
        it.extend_from_slice(b"Desc\0");
        it.push(0);
        it.push(0);
        it.extend_from_slice(b"en\0");
        it.extend_from_slice(b"Beschr\0");
        it.extend_from_slice(b"international text");
        b.chunk(b"iTXt", &it);
        let mut itz = Vec::new();
        itz.extend_from_slice(b"DescZ\0");
        itz.push(1);
        itz.push(0);
        itz.extend_from_slice(b"en\0");
        itz.extend_from_slice(b"BeschrZ\0");
        itz.extend_from_slice(&mkpng::zlib_stored(b"compressed international text"));
        b.chunk(b"iTXt", &itz);
    }
    if extras.contains("time") {
        let mut d = Vec::new();
        d.extend_from_slice(&2024u16.to_be_bytes());
        d.extend_from_slice(&[6, 15, 12, 30, 45]);
        b.chunk(b"tIME", &d);
    }
    if extras.contains("exif") {
        let mut d = Vec::new();
        d.extend_from_slice(b"II");
        d.extend_from_slice(&[42, 0, 8, 0, 0, 0, 0, 0]);
        b.chunk(b"eXIf", &d);
    }
    if extras.contains("unk") {
        b.chunk(b"prVt", b"private safe-to-copy");
        b.chunk(b"prIv", b"private unsafe");
    }

    let raw = if il == 0 {
        let rows = gen_rows(w, h, bd, ct, npal, &mut rng);
        mkpng::filtered_none(&rows)
    } else {
        let mut r2 = Rng::new(seed ^ 0x1234_5678);
        mkpng::filtered_adam7(w, h, bd, ct, &mut |_pass, _row, cols| {
            let rb = mkpng::rowbytes(cols, bd, ct);
            let mut v = vec![0u8; rb];
            r2.fill(&mut v);
            if ct == 3 && bd == 8 && npal > 0 && npal < 256 {
                for x in v.iter_mut() {
                    *x %= npal as u8;
                }
            }
            v
        })
    };
    b.idat(&raw, split);

    if extras.contains("tail") {
        b.chunk(b"tEXt", b"After\0after idat");
        let mut d = Vec::new();
        d.extend_from_slice(&2020u16.to_be_bytes());
        d.extend_from_slice(&[1, 2, 3, 4, 5]);
        b.chunk(b"tIME", &d);
        b.chunk(b"prVt", b"trailing private");
    }

    Synth { png: b.iend(), npal }
}

/* ------------------------------------------------------------------ */
/* write path                                                          */
/* ------------------------------------------------------------------ */

/// Number of *input* bytes libpng expects per row given the write transforms.
fn write_input_rowbytes(w: u32, bd: u8, ct: u8, tr: &str) -> usize {
    let mut chans = mkpng::channels(ct);
    let mut depth = bd as u32;
    if tr.contains("packing") && bd < 8 {
        depth = 8;
    }
    if tr.contains("filler") {
        chans += 1;
    }
    let bits = w as u64 * chans as u64 * depth as u64;
    ((bits + 7) / 8) as usize
}

fn run_write(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let il = a.u("il", 0) as u8;
    let w = a.u("w", 17);
    let h = a.u("h", 13);
    let seed = a.u("seed", 1) as u64;
    let tr = a.s("tr", "none");
    let mode = a.s("mode", "rows");
    let extras = a.s("x", "none");
    let iters = a.u("n", 1);

    unsafe {
        for it in 0..iters {
            let mut rng = Rng::new(seed.wrapping_add(it as u64 * 7919));
            let (png, info) = new_write();

            if a.has("lvl") {
                (api.png_set_compression_level)(png, a.i("lvl", -1) as c_int);
            }
            if a.has("strat") {
                (api.png_set_compression_strategy)(png, a.i("strat", 0) as c_int);
            }
            if a.has("wb") {
                (api.png_set_compression_window_bits)(png, a.i("wb", 15) as c_int);
            }
            if a.has("ml") {
                (api.png_set_compression_mem_level)(png, a.i("ml", 8) as c_int);
            }
            if a.has("cbuf") {
                (api.png_set_compression_buffer_size)(png, a.u("cbuf", 8192) as usize);
            }
            if a.has("tlvl") {
                (api.png_set_text_compression_level)(png, a.i("tlvl", -1) as c_int);
            }
            if a.has("filt") {
                (api.png_set_filter)(png, 0, a.i("filt", 0xff) as c_int);
            }
            if a.has("flush") {
                (api.png_set_flush)(png, a.i("flush", 0) as c_int);
            }
            if a.has("wstat") {
                (api.png_set_write_status_fn)(png, Some(super::cb_write_status));
            }

            let npal: u32 = if ct == 3 {
                match bd {
                    1 => 2,
                    2 => 4,
                    4 => 16,
                    _ => 1 + rng.below(256),
                }
            } else {
                0
            };

            (api.png_set_IHDR)(png, info, w, h, bd as c_int, ct as c_int, il as c_int, 0, 0);

            if ct == 3 {
                let mut pal = vec![png_color::default(); npal as usize];
                for p in pal.iter_mut() {
                    p.red = rng.byte();
                    p.green = rng.byte();
                    p.blue = rng.byte();
                }
                (api.png_set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
            }

            if extras.contains("gama") {
                (api.png_set_gAMA_fixed)(png, info, 45455);
            }
            if extras.contains("srgb") {
                (api.png_set_sRGB)(png, info, PNG_sRGB_INTENT_PERCEPTUAL);
            }
            if extras.contains("chrm") {
                (api.png_set_cHRM_fixed)(png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000);
            }
            if extras.contains("phys") {
                (api.png_set_pHYs)(png, info, 2835, 2835, PNG_RESOLUTION_METER);
            }
            if extras.contains("offs") {
                (api.png_set_oFFs)(png, info, -12, 34, PNG_OFFSET_MICROMETER);
            }
            if extras.contains("time") {
                let t = png_time { year: 2024, month: 6, day: 15, hour: 12, minute: 30, second: 45 };
                (api.png_set_tIME)(png, info, &t);
            }
            if extras.contains("text") {
                let k = cs("Title");
                let v = cs("hello world");
                let t = png_text {
                    compression: -1,
                    key: k.as_ptr() as *mut c_char,
                    text: v.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t, 1);
                let k2 = cs("Comment");
                let v2 = cs("this text is long enough to be worth compressing, over and over and over");
                let t2 = png_text {
                    compression: 0,
                    key: k2.as_ptr() as *mut c_char,
                    text: v2.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t2, 1);
                let k3 = cs("Desc");
                let v3 = cs("international");
                let l3 = cs("en");
                let lk3 = cs("Beschreibung");
                let t3 = png_text {
                    compression: 1,
                    key: k3.as_ptr() as *mut c_char,
                    text: v3.as_ptr() as *mut c_char,
                    lang: l3.as_ptr() as *mut c_char,
                    lang_key: lk3.as_ptr() as *mut c_char,
                    ..Default::default()
                };
                (api.png_set_text)(png, info, &t3, 1);
            }
            if extras.contains("trns") {
                match ct {
                    0 => {
                        let c = png_color_16 { gray: 3, ..Default::default() };
                        (api.png_set_tRNS)(png, info, std::ptr::null(), 0, &c);
                    }
                    2 => {
                        let c = png_color_16 { red: 1, green: 2, blue: 3, ..Default::default() };
                        (api.png_set_tRNS)(png, info, std::ptr::null(), 0, &c);
                    }
                    3 => {
                        let n = npal.min(8) as c_int;
                        let al = vec![128u8; n as usize];
                        (api.png_set_tRNS)(png, info, al.as_ptr(), n, std::ptr::null());
                    }
                    _ => {}
                }
            }
            if extras.contains("bkgd") {
                let c = png_color_16 { index: 0, red: 7, green: 8, blue: 9, gray: 5 };
                (api.png_set_bKGD)(png, info, &c);
            }
            if extras.contains("sbit") {
                let v = if bd == 16 { 13 } else { bd.max(1) };
                let sb = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
                (api.png_set_sBIT)(png, info, &sb);
            }
            if extras.contains("iccp") {
                let prof = make_icc(ct == 0 || ct == 4);
                let name = cs("ICC");
                (api.png_set_iCCP)(png, info, name.as_ptr(), 0, prof.as_ptr(), prof.len() as u32);
            }
            if extras.contains("unk") {
                let data = b"private payload".to_vec();
                let u = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: data.as_ptr() as *mut u8,
                    size: data.len(),
                    location: PNG_HAVE_IHDR as u8,
                };
                (api.png_set_unknown_chunks)(png, info, &u, 1);
                (api.png_set_unknown_chunk_location)(png, info, 0, PNG_HAVE_IHDR);
                (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            }

            if tr.contains("bgr") {
                (api.png_set_bgr)(png);
            }
            if tr.contains("swap16") {
                (api.png_set_swap)(png);
            }
            if tr.contains("packing") {
                (api.png_set_packing)(png);
            }
            if tr.contains("packswap") {
                (api.png_set_packswap)(png);
            }
            if tr.contains("invmono") {
                (api.png_set_invert_mono)(png);
            }
            if tr.contains("invalpha") {
                (api.png_set_invert_alpha)(png);
            }
            if tr.contains("swapalpha") {
                (api.png_set_swap_alpha)(png);
            }
            if tr.contains("shift") {
                let v = if bd == 16 { 13 } else { bd.max(1) };
                let sb = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
                (api.png_set_shift)(png, &sb);
            }
            if tr.contains("filler") {
                let after = if tr.contains("filler_after") { PNG_FILLER_AFTER } else { PNG_FILLER_BEFORE };
                (api.png_set_filler)(png, 0, after);
            }

            let irb = write_input_rowbytes(w, bd, ct, &tr);
            let mut rows: Vec<Vec<u8>> = Vec::new();
            for _ in 0..h {
                let mut row = vec![0u8; irb];
                rng.fill(&mut row);
                if ct == 3 && bd == 8 && npal > 0 {
                    for b in row.iter_mut() {
                        *b %= npal as u8;
                    }
                }
                rows.push(row);
            }
            let mut rowptrs: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();

            match mode.as_str() {
                "png" => {
                    (api.png_set_rows)(png, info, rowptrs.as_mut_ptr());
                    let t = a.i("wt", 0) as c_int;
                    (api.png_write_png)(png, info, t, std::ptr::null_mut());
                }
                "image" => {
                    (api.png_write_info)(png, info);
                    if il == 1 {
                        r.kv("passes", (api.png_set_interlace_handling)(png));
                    }
                    (api.png_write_image)(png, rowptrs.as_mut_ptr());
                    (api.png_write_end)(png, info);
                }
                "chunks" => {
                    (api.png_write_sig)(png);
                    let mut d = Vec::new();
                    d.extend_from_slice(&w.to_be_bytes());
                    d.extend_from_slice(&h.to_be_bytes());
                    d.push(bd);
                    d.push(ct);
                    d.push(0);
                    d.push(0);
                    d.push(il);
                    (api.png_write_chunk)(png, b"IHDR".as_ptr(), d.as_ptr(), d.len());
                    (api.png_write_chunk_start)(png, b"prVt".as_ptr(), 12);
                    (api.png_write_chunk_data)(png, b"abcdef".as_ptr(), 6);
                    (api.png_write_chunk_data)(png, b"ghijkl".as_ptr(), 6);
                    (api.png_write_chunk_end)(png);
                    (api.png_write_chunk)(png, b"IEND".as_ptr(), std::ptr::null(), 0);
                }
                "split" => {
                    (api.png_write_info_before_PLTE)(png, info);
                    (api.png_write_info)(png, info);
                    let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
                    r.kv("passes", passes);
                    for _p in 0..passes {
                        for row in rows.iter() {
                            (api.png_write_row)(png, row.as_ptr());
                        }
                    }
                    (api.png_write_end)(png, info);
                }
                _ => {
                    (api.png_write_info)(png, info);
                    let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
                    r.kv("passes", passes);
                    for _p in 0..passes {
                        let mut i = 0usize;
                        while i < rows.len() {
                            let n = (rows.len() - i).min(3);
                            (api.png_write_rows)(png, rowptrs[i..].as_mut_ptr(), n as u32);
                            i += n;
                        }
                    }
                    (api.png_write_end)(png, info);
                }
            }

            let out = std::mem::take(&mut g().wbuf);
            if out.len() <= 4096 {
                r.bytes(&format!("out/{it}"), &out);
            } else {
                r.digest(&format!("out/{it}"), &out);
            }
            r.kv(&format!("flushes/{it}"), g().flushes);
            r.kv(&format!("iostate/{it}"), (api.png_get_io_state)(png));
            r.kv(&format!("iochunk/{it}"), (api.png_get_io_chunk_type)(png));
            let notes = std::mem::take(&mut g().notes);
            r.digest(&format!("notes/{it}"), notes.join(";").as_bytes());

            let mut p = png;
            let mut ip = info;
            (api.png_destroy_write_struct)(&mut p, &mut ip);
        }
    }
}

/* ------------------------------------------------------------------ */
/* read path                                                          */
/* ------------------------------------------------------------------ */

/// Dump every getter for `info`.
///
/// `trns_content` must be false for a *post-transform* info struct: after a
/// PNG_QUANTIZE transform on a non-palette image the reference C leaves
/// `info_ptr->num_trans` non-zero while `info_ptr->trans_alpha` still points at
/// memory that was never written for that colour type.  The byte the C reads
/// there changes when unrelated allocations move, so it is not a function of the
/// input and cannot be compared; `num_trans` and the pointer's nullness still
/// are.
unsafe fn dump_info_ex(
    png: PngPtr,
    info: InfoPtr,
    r: &mut super::Rec,
    pre: &str,
    trns_content: bool,
) {
    let api = api();
    // png_get_IHDR re-runs png_check_IHDR on the stored values, which is fatal
    // for an info struct that never had an IHDR (the end_info passed to
    // png_read_end).  Probe with the non-validating accessors first.
    let pw = (api.png_get_image_width)(png, info);
    let ph = (api.png_get_image_height)(png, info);
    r.kv(&format!("{pre}/wh"), format!("{pw} {ph}"));
    if pw != 0 && ph != 0 {
        let mut w = 0u32;
        let mut h = 0u32;
        let (mut bd, mut ct, mut il, mut cm, mut fm) = (0i32, 0i32, 0i32, 0i32, 0i32);
        let ret =
            (api.png_get_IHDR)(png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm);
        r.kv(&format!("{pre}/ihdr"), format!("{ret} {w} {h} {bd} {ct} {il} {cm} {fm}"));
    } else {
        r.kv(&format!("{pre}/ihdr"), "unset");
    }
    r.kv(&format!("{pre}/rowbytes"), (api.png_get_rowbytes)(png, info));
    r.kv(&format!("{pre}/channels"), (api.png_get_channels)(png, info));
    let mut validmask = 0u32;
    for flag in [
        PNG_INFO_gAMA, PNG_INFO_sBIT, PNG_INFO_cHRM, PNG_INFO_PLTE, PNG_INFO_tRNS, PNG_INFO_bKGD,
        PNG_INFO_hIST, PNG_INFO_pHYs, PNG_INFO_oFFs, PNG_INFO_tIME, PNG_INFO_pCAL, PNG_INFO_sRGB,
        PNG_INFO_iCCP, PNG_INFO_sPLT, PNG_INFO_sCAL, PNG_INFO_IDAT, PNG_INFO_eXIf, PNG_INFO_cICP,
        PNG_INFO_cLLI, PNG_INFO_mDCV,
    ] {
        let v = (api.png_get_valid)(png, info, flag);
        if v != 0 {
            validmask |= flag;
        }
        r.kv(&format!("{pre}/valid/{flag:x}"), v);
    }
    r.kv(&format!("{pre}/validmask"), format!("{validmask:x}"));

    let mut gfix = 0i32;
    r.kv(
        &format!("{pre}/gama"),
        format!("{} {gfix}", (api.png_get_gAMA_fixed)(png, info, &mut gfix)),
    );
    let mut intent = 0i32;
    r.kv(
        &format!("{pre}/srgb"),
        format!("{} {intent}", (api.png_get_sRGB)(png, info, &mut intent)),
    );
    let mut c = [0i32; 8];
    r.kv(
        &format!("{pre}/chrm"),
        format!(
            "{} {c:?}",
            (api.png_get_cHRM_fixed)(
                png, info, &mut c[0], &mut c[1], &mut c[2], &mut c[3], &mut c[4], &mut c[5],
                &mut c[6], &mut c[7]
            )
        ),
    );
    let mut pal: *mut png_color = std::ptr::null_mut();
    let mut np = 0i32;
    let pr = (api.png_get_PLTE)(png, info, &mut pal, &mut np);
    r.kv(&format!("{pre}/plte"), format!("{pr} {np}"));
    if !pal.is_null() && np > 0 {
        let flat: Vec<u8> = (0..np as usize)
            .flat_map(|k| {
                let c = *pal.add(k);
                [c.red, c.green, c.blue]
            })
            .collect();
        r.digest(&format!("{pre}/plted"), &flat);
    }
    let mut ta: *mut u8 = std::ptr::null_mut();
    let mut tn = 0i32;
    let mut tc: *mut png_color_16 = std::ptr::null_mut();
    let tr_ = (api.png_get_tRNS)(png, info, &mut ta, &mut tn, &mut tc);
    r.kv(&format!("{pre}/trns"), format!("{tr_} {tn}"));
    r.kv(&format!("{pre}/trnsa_null"), ta.is_null());
    if trns_content && !ta.is_null() && tn > 0 {
        r.digest(&format!("{pre}/trnsa"), std::slice::from_raw_parts(ta, tn as usize));
    }
    if !tc.is_null() {
        let v = *tc;
        r.kv(
            &format!("{pre}/trnsc"),
            format!("{} {} {} {} {}", v.index, v.red, v.green, v.blue, v.gray),
        );
    }
    let mut bk: *mut png_color_16 = std::ptr::null_mut();
    let bkr = (api.png_get_bKGD)(png, info, &mut bk);
    if !bk.is_null() {
        let v = *bk;
        r.kv(
            &format!("{pre}/bkgd"),
            format!("{bkr} {} {} {} {} {}", v.index, v.red, v.green, v.blue, v.gray),
        );
    } else {
        r.kv(&format!("{pre}/bkgd"), format!("{bkr} none"));
    }
    let mut sb: *mut png_color_8 = std::ptr::null_mut();
    let sbr = (api.png_get_sBIT)(png, info, &mut sb);
    if !sb.is_null() {
        let v = *sb;
        r.kv(
            &format!("{pre}/sbit"),
            format!("{sbr} {} {} {} {} {}", v.red, v.green, v.blue, v.gray, v.alpha),
        );
    } else {
        r.kv(&format!("{pre}/sbit"), format!("{sbr} none"));
    }
    let mut hist: *mut u16 = std::ptr::null_mut();
    let hr = (api.png_get_hIST)(png, info, &mut hist);
    r.kv(&format!("{pre}/hist"), hr);
    if !hist.is_null() && np > 0 {
        let flat: Vec<u8> = (0..np as usize).flat_map(|k| (*hist.add(k)).to_le_bytes()).collect();
        r.digest(&format!("{pre}/histd"), &flat);
    }
    let (mut rx, mut ry, mut ut) = (0u32, 0u32, 0i32);
    r.kv(
        &format!("{pre}/phys"),
        format!("{} {rx} {ry} {ut}", (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ut)),
    );
    let (mut ox, mut oy, mut ou) = (0i32, 0i32, 0i32);
    r.kv(
        &format!("{pre}/offs"),
        format!("{} {ox} {oy} {ou}", (api.png_get_oFFs)(png, info, &mut ox, &mut oy, &mut ou)),
    );
    let mut su = 0i32;
    let mut sw: *mut c_char = std::ptr::null_mut();
    let mut sh: *mut c_char = std::ptr::null_mut();
    r.kv(
        &format!("{pre}/scal"),
        format!("{} {su}", (api.png_get_sCAL_s)(png, info, &mut su, &mut sw, &mut sh)),
    );
    r.cstr(&format!("{pre}/scalw"), sw);
    r.cstr(&format!("{pre}/scalh"), sh);
    let mut tp: *mut png_time = std::ptr::null_mut();
    let tir = (api.png_get_tIME)(png, info, &mut tp);
    if !tp.is_null() {
        let v = *tp;
        r.kv(
            &format!("{pre}/time"),
            format!("{tir} {} {} {} {} {} {}", v.year, v.month, v.day, v.hour, v.minute, v.second),
        );
    } else {
        r.kv(&format!("{pre}/time"), format!("{tir} none"));
    }
    let mut iname: *mut c_char = std::ptr::null_mut();
    let mut ictype = 0i32;
    let mut iprof: *mut u8 = std::ptr::null_mut();
    let mut ilen = 0u32;
    let ir = (api.png_get_iCCP)(png, info, &mut iname, &mut ictype, &mut iprof, &mut ilen);
    r.kv(&format!("{pre}/iccp"), format!("{ir} {ictype} {ilen}"));
    r.cstr(&format!("{pre}/iccpname"), iname);
    if !iprof.is_null() && ilen > 0 {
        r.digest(&format!("{pre}/iccpd"), std::slice::from_raw_parts(iprof, ilen as usize));
    }
    let mut splt: *mut png_sPLT_t = std::ptr::null_mut();
    let sn = (api.png_get_sPLT)(png, info, &mut splt);
    r.kv(&format!("{pre}/splt"), sn);
    for k in 0..sn as usize {
        let e = *splt.add(k);
        r.cstr(&format!("{pre}/spltn/{k}"), e.name);
        r.kv(&format!("{pre}/spltd/{k}"), format!("{} {}", e.depth, e.nentries));
    }
    let mut txt: *mut png_text = std::ptr::null_mut();
    let mut tnum = 0i32;
    let tres = (api.png_get_text)(png, info, &mut txt, &mut tnum);
    r.kv(&format!("{pre}/text"), format!("{tres} {tnum}"));
    for k in 0..tnum as usize {
        let e = *txt.add(k);
        r.kv(&format!("{pre}/textc/{k}"), e.compression);
        r.cstr(&format!("{pre}/textk/{k}"), e.key);
        r.cstr(&format!("{pre}/textt/{k}"), e.text);
        r.kv(&format!("{pre}/textl/{k}"), format!("{} {}", e.text_length, e.itxt_length));
        r.cstr(&format!("{pre}/textlang/{k}"), e.lang);
        r.cstr(&format!("{pre}/textlk/{k}"), e.lang_key);
    }
    let mut pp: *mut c_char = std::ptr::null_mut();
    let (mut x0, mut x1, mut ty, mut npar) = (0i32, 0i32, 0i32, 0i32);
    let mut un: *mut c_char = std::ptr::null_mut();
    let mut params: *mut *mut c_char = std::ptr::null_mut();
    let pcr = (api.png_get_pCAL)(png, info, &mut pp, &mut x0, &mut x1, &mut ty, &mut npar, &mut un, &mut params);
    r.kv(&format!("{pre}/pcal"), format!("{pcr} {x0} {x1} {ty} {npar}"));
    r.cstr(&format!("{pre}/pcalp"), pp);
    r.cstr(&format!("{pre}/pcalu"), un);
    for k in 0..npar as usize {
        if !params.is_null() {
            r.cstr(&format!("{pre}/pcalpar/{k}"), *params.add(k));
        }
    }
    let mut enum_ = 0u32;
    let mut eptr: *mut u8 = std::ptr::null_mut();
    let er = (api.png_get_eXIf_1)(png, info, &mut enum_, &mut eptr);
    r.kv(&format!("{pre}/exif"), format!("{er} {enum_}"));
    if !eptr.is_null() && enum_ > 0 {
        r.digest(&format!("{pre}/exifd"), std::slice::from_raw_parts(eptr, enum_ as usize));
    }
    let (mut p1, mut p2, mut p3, mut p4) = (0u8, 0u8, 0u8, 0u8);
    r.kv(
        &format!("{pre}/cicp"),
        format!("{} {p1} {p2} {p3} {p4}", (api.png_get_cICP)(png, info, &mut p1, &mut p2, &mut p3, &mut p4)),
    );
    let (mut l1, mut l2) = (0u32, 0u32);
    r.kv(
        &format!("{pre}/clli"),
        format!("{} {l1} {l2}", (api.png_get_cLLI_fixed)(png, info, &mut l1, &mut l2)),
    );
    let mut mv = [0i32; 8];
    let (mut m1, mut m2) = (0u32, 0u32);
    r.kv(
        &format!("{pre}/mdcv"),
        format!(
            "{} {mv:?} {m1} {m2}",
            (api.png_get_mDCV_fixed)(
                png, info, &mut mv[0], &mut mv[1], &mut mv[2], &mut mv[3], &mut mv[4], &mut mv[5],
                &mut mv[6], &mut mv[7], &mut m1, &mut m2
            )
        ),
    );
    let mut unk: *mut png_unknown_chunk = std::ptr::null_mut();
    let un_ = (api.png_get_unknown_chunks)(png, info, &mut unk);
    r.kv(&format!("{pre}/unknown"), un_);
    for k in 0..un_ as usize {
        let e = *unk.add(k);
        r.kv(
            &format!("{pre}/unk/{k}"),
            format!("{:?} {} {}", String::from_utf8_lossy(&e.name[..4]), e.size, e.location),
        );
        if !e.data.is_null() && e.size > 0 {
            r.digest(&format!("{pre}/unkd/{k}"), std::slice::from_raw_parts(e.data, e.size));
        }
    }
    r.kv(&format!("{pre}/palettemax"), (api.png_get_palette_max)(png, info));
    let sig = (api.png_get_signature)(png, info);
    if !sig.is_null() {
        r.bytes(&format!("{pre}/sig"), std::slice::from_raw_parts(sig, 8));
    }
}

#[inline]
unsafe fn dump_info(png: PngPtr, info: InfoPtr, r: &mut super::Rec, pre: &str) {
    dump_info_ex(png, info, r, pre, true)
}

unsafe fn apply_read_transforms(png: PngPtr, tr: &str, bd: u8) {
    let api = api();
    for t in tr.split('+') {
        match t {
            "" | "none" => {}
            "expand" => (api.png_set_expand)(png),
            "expandgray" => (api.png_set_expand_gray_1_2_4_to_8)(png),
            "pal2rgb" => (api.png_set_palette_to_rgb)(png),
            "trns2alpha" => (api.png_set_tRNS_to_alpha)(png),
            "expand16" => (api.png_set_expand_16)(png),
            "bgr" => (api.png_set_bgr)(png),
            "gray2rgb" => (api.png_set_gray_to_rgb)(png),
            "rgb2gray" => (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -1, -1),
            "rgb2graywarn" => (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_WARN, 21260, 71520),
            "stripalpha" => (api.png_set_strip_alpha)(png),
            "swapalpha" => (api.png_set_swap_alpha)(png),
            "invalpha" => (api.png_set_invert_alpha)(png),
            "swap16" => (api.png_set_swap)(png),
            "packing" => (api.png_set_packing)(png),
            "packswap" => (api.png_set_packswap)(png),
            "invmono" => (api.png_set_invert_mono)(png),
            "strip16" => (api.png_set_strip_16)(png),
            "scale16" => (api.png_set_scale_16)(png),
            "filler_before" => (api.png_set_filler)(png, 0x55, PNG_FILLER_BEFORE),
            "filler_after" => (api.png_set_filler)(png, 0x55, PNG_FILLER_AFTER),
            "addalpha_before" => (api.png_set_add_alpha)(png, 0xaa, PNG_FILLER_BEFORE),
            "addalpha_after" => (api.png_set_add_alpha)(png, 0xaa, PNG_FILLER_AFTER),
            "shift" => {
                let v = if bd == 16 { 13 } else { bd.max(1) };
                let sb = png_color_8 { red: v, green: v, blue: v, gray: v, alpha: v };
                (api.png_set_shift)(png, &sb);
            }
            "gamma" => (api.png_set_gamma_fixed)(png, 220000, 45455),
            "gammahigh" => (api.png_set_gamma_fixed)(png, 100000, 100000),
            "alphapng" => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_PNG, PNG_DEFAULT_sRGB),
            "alphastd" => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_STANDARD, PNG_FP_1),
            "alphaopt" => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_OPTIMIZED, PNG_DEFAULT_sRGB),
            "alphabroken" => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_BROKEN, PNG_DEFAULT_sRGB),
            "background" => {
                let c = png_color_16 { index: 0, red: 0x8000, green: 0x4000, blue: 0x2000, gray: 0x6000 };
                (api.png_set_background_fixed)(png, &c, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
            }
            "backgroundexp" => {
                let c = png_color_16 { index: 1, red: 3, green: 2, blue: 1, gray: 2 };
                (api.png_set_background_fixed)(png, &c, PNG_BACKGROUND_GAMMA_FILE, 1, 100000);
            }
            "backgroundunique" => {
                let c = png_color_16 { index: 0, red: 0x1000, green: 0x2000, blue: 0x3000, gray: 0x4000 };
                (api.png_set_background_fixed)(png, &c, PNG_BACKGROUND_GAMMA_UNIQUE, 0, 60000);
            }
            "quantize" => {
                let mut pal = vec![png_color::default(); 256];
                for (i, p) in pal.iter_mut().enumerate() {
                    p.red = (i * 7) as u8;
                    p.green = (i * 11) as u8;
                    p.blue = (i * 13) as u8;
                }
                let hist: Vec<u16> = (0..256u16).collect();
                (api.png_set_quantize)(png, pal.as_mut_ptr(), 256, 16, hist.as_ptr(), 1);
            }
            "interlace" => {
                (api.png_set_interlace_handling)(png);
            }
            other => panic!("unknown read transform {other}"),
        }
    }
}

fn run_read(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let il = a.u("il", 0) as u8;
    let w = a.u("w", 17);
    let h = a.u("h", 13);
    let seed = a.u("seed", 1) as u64;
    let tr = a.s("tr", "none");
    let mode = a.s("mode", "image");
    let extras = a.s("x", "none");
    let split = a.u("split", 0) as usize;
    let iters = a.u("n", 1);

    unsafe {
        for it in 0..iters {
            let s = synth(ct, bd, il, w, h, &extras, split, seed.wrapping_add(it as u64 * 104729));
            r.digest(&format!("src/{it}"), &s.png);
            let (png, info, end) = new_read(&s.png);

            if a.has("crc") {
                (api.png_set_crc_action)(png, a.i("crc", 0) as c_int, a.i("crca", 0) as c_int);
            }
            if a.has("benign") {
                (api.png_set_benign_errors)(png, a.i("benign", 1) as c_int);
            }
            if a.has("keep") {
                (api.png_set_keep_unknown_chunks)(png, a.i("keep", 0) as c_int, std::ptr::null(), 0);
            }
            if a.has("opt") {
                r.kv(
                    "setoption",
                    (api.png_set_option)(png, a.i("opt", 0) as c_int, a.i("optv", 3) as c_int),
                );
            }
            if a.has("idx") {
                (api.png_set_check_for_invalid_index)(png, a.i("idx", 1) as c_int);
            }
            if a.has("rstat") {
                (api.png_set_read_status_fn)(png, Some(super::cb_read_status));
            }
            if a.has("mng") {
                r.kv("mng", (api.png_permit_mng_features)(png, a.u("mng", 5)));
            }
            if a.has("ulim") {
                (api.png_set_user_limits)(png, a.u("ulim", 1000000), a.u("ulim", 1000000));
            }

            if mode == "png" {
                // Supply our own row buffers so that the bytes libpng does NOT
                // write (the padding bits of a sub-8-bit row, which
                // png_combine_row deliberately preserves) are deterministic
                // instead of whatever png_malloc happened to return.
                let cap = (w as usize * 8 + 64).max(64);
                let mut rows: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0xA5u8; cap]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_set_rows)(png, info, rp.as_mut_ptr());
                let t = a.i("rt", 0) as c_int;
                (api.png_read_png)(png, info, t, std::ptr::null_mut());
                dump_info_ex(png, info, r, &format!("after/{it}"), false);
                let got = (api.png_get_rows)(png, info);
                r.kv(&format!("rowsptr_ours/{it}"), got == rp.as_mut_ptr());
                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let n = rb.min(cap);
                let flat: Vec<u8> = rows[..hh as usize].iter().flat_map(|v| v[..n].to_vec()).collect();
                r.digest(&format!("rows/{it}"), &flat);
                if flat.len() <= 2048 {
                    r.bytes(&format!("rowsx/{it}"), &flat);
                }
                let guards: Vec<u8> = rows[..hh as usize].iter().flat_map(|v| v[n..].to_vec()).collect();
                r.digest(&format!("guard/{it}"), &guards);
                (api.png_set_rows)(png, info, std::ptr::null_mut());
            } else {
                (api.png_read_info)(png, info);
                dump_info(png, info, r, &format!("info/{it}"));
                r.kv(&format!("iostate/{it}"), (api.png_get_io_state)(png));
                r.kv(&format!("iochunk/{it}"), (api.png_get_io_chunk_type)(png));

                apply_read_transforms(png, &tr, bd);
                let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
                r.kv(&format!("passes/{it}"), passes);

                if mode == "startimage" {
                    // png_start_read_image instead of png_read_update_info: the
                    // info struct is *not* updated, so allocate generously and
                    // compare the whole (prefilled) buffer.
                    (api.png_start_read_image)(png);
                    let cap = (w as usize * 8 + 64).max(64);
                    let hh = h as usize;
                    let mut rows: Vec<Vec<u8>> = (0..hh).map(|_| vec![0xA5u8; cap]).collect();
                    let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                    (api.png_read_image)(png, rp.as_mut_ptr());
                    let flat: Vec<u8> = rows.concat();
                    r.digest(&format!("rows/{it}"), &flat);
                    if flat.len() <= 2048 {
                        r.bytes(&format!("rowsx/{it}"), &flat);
                    }
                    r.kv(&format!("png_rowbytes/{it}"), (api.png_get_rowbytes)(png, info));
                    (api.png_read_end)(png, end);
                    dump_info(png, end, r, &format!("end/{it}"));
                    r.kv(&format!("consumed/{it}"), g().rpos);
                    let mut p = png;
                    let mut ip = info;
                    let mut ep = end;
                    (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
                    continue;
                }

                (api.png_read_update_info)(png, info);
                dump_info_ex(png, info, r, &format!("upd/{it}"), false);
                r.kv(&format!("rgb2gray/{it}"), (api.png_get_rgb_to_gray_status)(png));

                let rb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                // Pre-fill with a pattern (not zero) so that bytes/bits libpng
                // deliberately preserves rather than writes are compared too.
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rb + 8]).collect();
                let mut disp: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0x5Au8; rb + 8]).collect();

                match mode.as_str() {
                    "row" => {
                        for _p in 0..passes {
                            for y in 0..hh as usize {
                                (api.png_read_row)(png, rows[y].as_mut_ptr(), disp[y].as_mut_ptr());
                            }
                        }
                    }
                    "rowonly" => {
                        for _p in 0..passes {
                            for y in 0..hh as usize {
                                (api.png_read_row)(png, rows[y].as_mut_ptr(), std::ptr::null_mut());
                            }
                        }
                    }
                    "disponly" => {
                        for _p in 0..passes {
                            for y in 0..hh as usize {
                                (api.png_read_row)(png, std::ptr::null_mut(), disp[y].as_mut_ptr());
                            }
                        }
                    }
                    "rows" => {
                        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                        let mut dp: Vec<*mut u8> = disp.iter_mut().map(|v| v.as_mut_ptr()).collect();
                        for _p in 0..passes {
                            let mut y = 0usize;
                            while y < hh as usize {
                                let n = (hh as usize - y).min(3);
                                (api.png_read_rows)(png, rp[y..].as_mut_ptr(), dp[y..].as_mut_ptr(), n as u32);
                                y += n;
                            }
                        }
                    }
                    _ => {
                        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                        (api.png_read_image)(png, rp.as_mut_ptr());
                    }
                }

                let flat: Vec<u8> = rows.iter().flat_map(|v| v[..rb].to_vec()).collect();
                r.digest(&format!("rows/{it}"), &flat);
                if flat.len() <= 2048 {
                    r.bytes(&format!("rowsx/{it}"), &flat);
                }
                let flatd: Vec<u8> = disp.iter().flat_map(|v| v[..rb].to_vec()).collect();
                r.digest(&format!("disp/{it}"), &flatd);
                // guard bytes must be untouched
                let guards: Vec<u8> = rows.iter().flat_map(|v| v[rb..].to_vec()).collect();
                r.digest(&format!("guard/{it}"), &guards);

                (api.png_read_end)(png, end);
                dump_info(png, end, r, &format!("end/{it}"));
                let notes = std::mem::take(&mut g().notes);
                r.digest(&format!("notes/{it}"), notes.join(";").as_bytes());
            }

            r.kv(&format!("consumed/{it}"), g().rpos);
            let mut p = png;
            let mut ip = info;
            let mut ep = end;
            (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
        }
    }
}

/* ------------------------------------------------------------------ */
/* progressive read                                                    */
/* ------------------------------------------------------------------ */

static mut PROG_ROWS: Option<Vec<Vec<u8>>> = None;
static mut PROG_RB: usize = 0;

unsafe extern "C" fn prog_info(png: PngPtr, info: InfoPtr) {
    let api = api();
    let r = rec();
    dump_info(png, info, r, "prog/info");
    if (api.png_get_interlace_type)(png, info) == 1 {
        (api.png_set_interlace_handling)(png);
    }
    (api.png_read_update_info)(png, info);
    let rb = (api.png_get_rowbytes)(png, info);
    let h = (api.png_get_image_height)(png, info);
    PROG_RB = rb;
    PROG_ROWS = Some((0..h as usize).map(|_| vec![0u8; rb]).collect());
    r.kv("prog/rowbytes", rb);
}

unsafe extern "C" fn prog_row(png: PngPtr, new_row: *mut u8, row_num: u32, pass: c_int) {
    let api = api();
    g().notes.push(format!("row {row_num} {pass} {}", !new_row.is_null()));
    if let Some(rows) = (*std::ptr::addr_of_mut!(PROG_ROWS)).as_mut() {
        if (row_num as usize) < rows.len() {
            let dst = rows[row_num as usize].as_mut_ptr();
            (api.png_progressive_combine_row)(png, dst, new_row);
        }
    }
}

unsafe extern "C" fn prog_end(png: PngPtr, info: InfoPtr) {
    let r = rec();
    dump_info(png, info, r, "prog/end");
    r.line("prog/end reached");
}

fn run_progressive(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let il = a.u("il", 0) as u8;
    let w = a.u("w", 17);
    let h = a.u("h", 13);
    let seed = a.u("seed", 1) as u64;
    let extras = a.s("x", "none");
    let split = a.u("split", 0) as usize;
    let feed = a.u("feed", 7) as usize;
    let pause = a.u("pause", 0);

    unsafe {
        let s = synth(ct, bd, il, w, h, &extras, split, seed);
        r.digest("src", &s.png);
        let png = (api.png_create_read_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
        let info = (api.png_create_info_struct)(png);
        (api.png_set_progressive_read_fn)(
            png,
            0x99 as *mut c_void,
            Some(prog_info),
            Some(prog_row),
            Some(prog_end),
        );
        r.kv("progptr", (api.png_get_progressive_ptr)(png) as usize);

        let mut data = s.png.clone();
        let mut i = 0usize;
        let mut chunks = 0u32;
        while i < data.len() {
            let n = (data.len() - i).min(feed.max(1));
            (api.png_process_data)(png, info, data[i..].as_mut_ptr(), n);
            i += n;
            chunks += 1;
            if pause != 0 && chunks % pause == 0 {
                let left = (api.png_process_data_pause)(png, 0);
                r.kv(&format!("pause/{chunks}"), left);
                i -= left;
            }
            let skip = (api.png_process_data_skip)(png);
            if skip != 0 {
                r.kv(&format!("skip/{chunks}"), skip);
                i += skip as usize;
            }
        }
        r.kv("feeds", chunks);
        if let Some(rows) = (*std::ptr::addr_of_mut!(PROG_ROWS)).as_ref() {
            let flat: Vec<u8> = rows.iter().flat_map(|v| v.clone()).collect();
            r.digest("rows", &flat);
            if flat.len() <= 2048 {
                r.bytes("rowsx", &flat);
            }
        }
        let notes = std::mem::take(&mut g().notes);
        r.digest("notes", notes.join(";").as_bytes());
        r.kv("notecount", notes.len());
        let mut p = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut p, &mut ip, std::ptr::null_mut());
    }
}

/* ------------------------------------------------------------------ */
/* simplified read / write                                             */
/* ------------------------------------------------------------------ */

fn run_simple_read(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let il = a.u("il", 0) as u8;
    let w = a.u("w", 17);
    let h = a.u("h", 13);
    let seed = a.u("seed", 1) as u64;
    let extras = a.s("x", "none");
    let outfmt = a.u("fmt", PNG_FORMAT_RGBA);
    let stride_sign = a.i("neg", 0);
    let usebg = a.i("bg", 0);
    let flags = a.u("flags", 0);
    let iters = a.u("n", 1);

    unsafe {
        for it in 0..iters {
            let s = synth(ct, bd, il, w, h, &extras, 0, seed.wrapping_add(it as u64 * 6151));
            r.digest(&format!("src/{it}"), &s.png);
            let mut img = png_image::default();
            let ok = (api.png_image_begin_read_from_memory)(
                &mut img,
                s.png.as_ptr() as *const c_void,
                s.png.len(),
            );
            r.kv(
                &format!("begin/{it}"),
                format!(
                    "{ok} {} {} fmt={:x} flags={:x} cme={} woe={} {:?}",
                    img.width, img.height, img.format, img.flags, img.colormap_entries,
                    img.warning_or_error, img.msg()
                ),
            );
            if ok == 0 {
                (api.png_image_free)(&mut img);
                continue;
            }
            img.format = outfmt;
            img.flags |= flags;
            let stride = image_row_stride(&img) as i32;
            let stride = if stride_sign != 0 { -stride } else { stride };
            let bufsz = image_size(&img).max(1);
            let cmsz = colormap_size(&img).max(4 * 256);
            // libpng does the bottom-up offsetting itself:
            //   first_row = buffer + (height-1) * (-row_stride)     [pngread.c]
            // so the caller always passes the *lowest* address.  Note that the
            // adjustment above is in bytes while the per-row step is in
            // components, so with a negative stride and a 16-bit output format
            // libpng writes outside the nominal buffer; the slack below keeps
            // those writes observable (and comparable) instead of corrupting the
            // heap.
            let pad = if stride < 0 { 4 * bufsz + 64 } else { 0 };
            let mut buf = vec![0xA5u8; pad + bufsz + pad + 16];
            let mut cmap = vec![0x5Au8; cmsz + 16];
            let bg = png_color { red: 0x40, green: 0x80, blue: 0xc0 };
            let bufptr = buf.as_mut_ptr().add(pad);
            let ok2 = (api.png_image_finish_read)(
                &mut img,
                if usebg != 0 { &bg } else { std::ptr::null() },
                bufptr as *mut c_void,
                stride,
                cmap.as_mut_ptr() as *mut c_void,
            );
            r.kv(
                &format!("finish/{it}"),
                format!(
                    "{ok2} cme={} woe={} {:?}",
                    img.colormap_entries, img.warning_or_error, img.msg()
                ),
            );
            r.digest(&format!("buf/{it}"), &buf[pad..pad + bufsz]);
            if bufsz <= 2048 {
                r.bytes(&format!("bufx/{it}"), &buf[pad..pad + bufsz]);
            }
            r.digest(&format!("bufbefore/{it}"), &buf[..pad]);
            r.digest(&format!("bufafter/{it}"), &buf[pad + bufsz..]);
            let used = colormap_size(&img);
            r.digest(&format!("cmap/{it}"), &cmap[..used.min(cmap.len())]);
            (api.png_image_free)(&mut img);
        }
    }
}

fn run_simple_write(a: &Args) {
    let api = api();
    let r = rec();
    let fmt = a.u("fmt", PNG_FORMAT_RGBA);
    let w = a.u("w", 17);
    let h = a.u("h", 13);
    let seed = a.u("seed", 1) as u64;
    let conv8 = a.i("c8", 0) as c_int;
    let cme = a.u("cme", 0);
    let flags = a.u("flags", 0);
    let neg = a.i("neg", 0);
    let iters = a.u("n", 1);

    unsafe {
        for it in 0..iters {
            let mut rng = Rng::new(seed.wrapping_add(it as u64 * 3571));
            let mut img = png_image::default();
            img.width = w;
            img.height = h;
            img.format = fmt;
            img.flags = flags;
            img.colormap_entries = cme;
            let stride = image_row_stride(&img) as i32;
            let bufsz = image_size(&img).max(1);
            let mut buf = vec![0u8; bufsz];
            rng.fill(&mut buf);
            let cmsz = colormap_size(&img).max(1);
            let mut cmap = vec![0u8; cmsz];
            rng.fill(&mut cmap);

            // first ask for the required size
            let mut need: usize = 0;
            let ok0 = (api.png_image_write_to_memory)(
                &mut img,
                std::ptr::null_mut(),
                &mut need,
                conv8,
                buf.as_ptr() as *const c_void,
                if neg != 0 { -stride } else { stride },
                if cme > 0 { cmap.as_ptr() as *const c_void } else { std::ptr::null() },
            );
            r.kv(
                &format!("size/{it}"),
                format!("{ok0} {need} woe={} {:?}", img.warning_or_error, img.msg()),
            );
            if ok0 == 0 {
                (api.png_image_free)(&mut img);
                continue;
            }
            let mut out = vec![0u8; need + 16];
            let mut cap = need;
            let ok1 = (api.png_image_write_to_memory)(
                &mut img,
                out.as_mut_ptr() as *mut c_void,
                &mut cap,
                conv8,
                buf.as_ptr() as *const c_void,
                if neg != 0 { -stride } else { stride },
                if cme > 0 { cmap.as_ptr() as *const c_void } else { std::ptr::null() },
            );
            r.kv(
                &format!("write/{it}"),
                format!("{ok1} {cap} woe={} {:?}", img.warning_or_error, img.msg()),
            );
            let n = cap.min(out.len());
            if n <= 4096 {
                r.bytes(&format!("png/{it}"), &out[..n]);
            } else {
                r.digest(&format!("png/{it}"), &out[..n]);
            }
            r.digest(&format!("tailguard/{it}"), &out[n..]);
            (api.png_image_free)(&mut img);

            // round trip: read the produced stream back
            if ok1 != 0 && n > 0 {
                let mut img2 = png_image::default();
                let rok = (api.png_image_begin_read_from_memory)(
                    &mut img2,
                    out.as_ptr() as *const c_void,
                    n,
                );
                r.kv(
                    &format!("rt_begin/{it}"),
                    format!(
                        "{rok} {} {} {:x} {:x} {} {:?}",
                        img2.width, img2.height, img2.format, img2.flags, img2.colormap_entries,
                        img2.msg()
                    ),
                );
                if rok != 0 {
                    let bs = image_size(&img2).max(1);
                    let mut b2 = vec![0u8; bs];
                    let mut cm2 = vec![0u8; colormap_size(&img2).max(4 * 256)];
                    let rok2 = (api.png_image_finish_read)(
                        &mut img2,
                        std::ptr::null(),
                        b2.as_mut_ptr() as *mut c_void,
                        image_row_stride(&img2) as i32,
                        cm2.as_mut_ptr() as *mut c_void,
                    );
                    r.kv(&format!("rt_finish/{it}"), format!("{rok2} {:?}", img2.msg()));
                    r.digest(&format!("rt_buf/{it}"), &b2);
                }
                (api.png_image_free)(&mut img2);
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* user limits                                                         */
/* ------------------------------------------------------------------ */

fn run_limits(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    unsafe {
        let mut rng = Rng::new(seed);
        let png = (api.png_create_read_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
        r.kv("def_width_max", (api.png_get_user_width_max)(png));
        r.kv("def_height_max", (api.png_get_user_height_max)(png));
        r.kv("def_cache_max", (api.png_get_chunk_cache_max)(png));
        r.kv("def_malloc_max", (api.png_get_chunk_malloc_max)(png));
        r.kv("def_cbuf", (api.png_get_compression_buffer_size)(png));
        for i in 0..64 {
            let a1 = rng.next_u32();
            let b1 = rng.next_u32();
            (api.png_set_user_limits)(png, a1, b1);
            r.kv(&format!("lim/{i}"), format!("{} {}", (api.png_get_user_width_max)(png), (api.png_get_user_height_max)(png)));
            (api.png_set_chunk_cache_max)(png, a1);
            r.kv(&format!("cache/{i}"), (api.png_get_chunk_cache_max)(png));
            (api.png_set_chunk_malloc_max)(png, b1 as usize);
            r.kv(&format!("malloc/{i}"), (api.png_get_chunk_malloc_max)(png));
        }
        // option matrix, including out-of-range and odd option numbers
        for opt in -4i32..24 {
            for onoff in [-1i32, 0, 1, 2, 3] {
                r.kv(
                    &format!("opt/{opt}/{onoff}"),
                    (api.png_set_option)(png, opt, onoff),
                );
            }
        }
        // MNG features
        for m in 0u32..8 {
            r.kv(&format!("mng/{m}"), (api.png_permit_mng_features)(png, m));
        }
        // error pointer plumbing
        (api.png_set_error_fn)(png, 0xdead as *mut c_void, Some(cb_error), Some(cb_warn));
        r.kv("errptr", (api.png_get_error_ptr)(png) as usize);
        (api.png_set_read_fn)(png, 0xbeef as *mut c_void, Some(cb_read));
        r.kv("ioptr", (api.png_get_io_ptr)(png) as usize);
        (api.png_set_user_transform_info)(png, 0xcafe as *mut c_void, 8, 3);
        r.kv("utptr", (api.png_get_user_transform_ptr)(png) as usize);
        (api.png_set_read_user_chunk_fn)(png, 0xf00d as *mut c_void, None);
        r.kv("ucptr", (api.png_get_user_chunk_ptr)(png) as usize);
        r.kv("memptr", (api.png_get_mem_ptr)(png) as usize);
        r.kv("currow", (api.png_get_current_row_number)(png));
        r.kv("curpass", (api.png_get_current_pass_number)(png));
        // compression buffer size round trip
        for sz in [1usize, 2, 8, 100, 8192, 65536] {
            (api.png_set_compression_buffer_size)(png, sz);
            r.kv(&format!("cbuf/{sz}"), (api.png_get_compression_buffer_size)(png));
        }
        // png_malloc / png_calloc / png_free
        for sz in [1usize, 7, 64, 1000] {
            let p = (api.png_malloc)(png, sz);
            r.kv(&format!("malloc_ok/{sz}"), !p.is_null());
            (api.png_free)(png, p);
            let q = (api.png_calloc)(png, sz);
            if !q.is_null() {
                let sl = std::slice::from_raw_parts(q as *const u8, sz);
                r.kv(&format!("calloc_zero/{sz}"), sl.iter().all(|&b| b == 0));
            }
            (api.png_free)(png, q);
            let w2 = (api.png_malloc_warn)(png, sz);
            r.kv(&format!("mallocwarn_ok/{sz}"), !w2.is_null());
            (api.png_free)(png, w2);
            let d = (api.png_malloc_default)(png, sz);
            r.kv(&format!("mallocdef_ok/{sz}"), !d.is_null());
            (api.png_free_default)(png, d);
        }
        (api.png_free)(png, std::ptr::null_mut());
        r.line("free null ok");
        let mut p = png;
        (api.png_destroy_read_struct)(&mut p, std::ptr::null_mut(), std::ptr::null_mut());
        r.kv("destroyed_null", p as usize);
    }
}

/* ------------------------------------------------------------------ */
/* unknown-chunk handling matrix                                       */
/* ------------------------------------------------------------------ */

unsafe extern "C" fn user_chunk_cb(_png: PngPtr, chunk: *mut png_unknown_chunk) -> c_int {
    let c = &*chunk;
    g().notes.push(format!(
        "uchunk {:?} {}",
        String::from_utf8_lossy(&c.name[..4]),
        c.size
    ));
    // "handled" for one specific name, "not handled" otherwise
    if &c.name[..4] == b"prVt" {
        1
    } else {
        0
    }
}

fn run_unknown(a: &Args) {
    let api = api();
    let r = rec();
    let keep = a.i("keep", 0) as c_int;
    let percc = a.s("list", "");
    let usecb = a.i("cb", 0);
    let seed = a.u("seed", 1) as u64;

    unsafe {
        let s = synth(2, 8, 0, 9, 7, "unktailtext", 0, seed);
        r.digest("src", &s.png);
        let (png, info, end) = new_read(&s.png);
        (api.png_set_keep_unknown_chunks)(png, keep, std::ptr::null(), 0);
        if !percc.is_empty() {
            let mut list = Vec::new();
            for name in percc.split(',') {
                let b = name.as_bytes();
                for k in 0..5 {
                    list.push(if k < b.len() { b[k] } else { 0 });
                }
            }
            let n = (list.len() / 5) as c_int;
            (api.png_set_keep_unknown_chunks)(png, a.i("keep2", 3) as c_int, list.as_ptr(), n);
            for name in percc.split(',') {
                let mut nb = [0u8; 5];
                for (k, c) in name.bytes().take(4).enumerate() {
                    nb[k] = c;
                }
                r.kv(&format!("handleas/{name}"), (api.png_handle_as_unknown)(png, nb.as_ptr()));
            }
        }
        for probe in ["IHDR", "PLTE", "IDAT", "IEND", "gAMA", "prVt", "prIv", "zzzz"] {
            let mut nb = [0u8; 5];
            for (k, c) in probe.bytes().take(4).enumerate() {
                nb[k] = c;
            }
            r.kv(&format!("probe/{probe}"), (api.png_handle_as_unknown)(png, nb.as_ptr()));
        }
        if usecb != 0 {
            (api.png_set_read_user_chunk_fn)(png, 0x1 as *mut c_void, Some(user_chunk_cb));
        }
        (api.png_read_info)(png, info);
        dump_info(png, info, r, "info");
        let rb = (api.png_get_rowbytes)(png, info);
        let hh = (api.png_get_image_height)(png, info);
        let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0u8; rb]).collect();
        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
        (api.png_read_image)(png, rp.as_mut_ptr());
        (api.png_read_end)(png, end);
        dump_info(png, end, r, "end");
        let flat: Vec<u8> = rows.iter().flat_map(|v| v.clone()).collect();
        r.digest("rows", &flat);
        let notes = std::mem::take(&mut g().notes);
        r.kv("notes", notes.join(";"));
        let mut p = png;
        let mut ip = info;
        let mut ep = end;
        (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
    }
}

/* ------------------------------------------------------------------ */
/* user transform callbacks                                            */
/* ------------------------------------------------------------------ */

unsafe extern "C" fn user_xform(png: PngPtr, row_info: *mut png_row_info, data: *mut u8) {
    let api = api();
    let ri = *row_info;
    g().notes.push(format!(
        "ut w={} rb={} ct={} bd={} ch={} pd={} row={} pass={}",
        ri.width,
        ri.rowbytes,
        ri.color_type,
        ri.bit_depth,
        ri.channels,
        ri.pixel_depth,
        (api.png_get_current_row_number)(png),
        (api.png_get_current_pass_number)(png)
    ));
    if !data.is_null() && ri.rowbytes > 0 {
        let s = std::slice::from_raw_parts_mut(data, ri.rowbytes);
        for (i, b) in s.iter_mut().enumerate() {
            *b ^= 0x5A ^ (i as u8);
        }
    }
}

fn run_usertransform(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let il = a.u("il", 0) as u8;
    let w = a.u("w", 17);
    let h = a.u("h", 9);
    let seed = a.u("seed", 1) as u64;
    let side = a.s("side", "read");
    let tr = a.s("tr", "none");

    unsafe {
        if side == "read" {
            let s = synth(ct, bd, il, w, h, "none", 0, seed);
            r.digest("src", &s.png);
            let (png, info, end) = new_read(&s.png);
            (api.png_read_info)(png, info);
            apply_read_transforms(png, &tr, bd);
            (api.png_set_read_user_transform_fn)(png, Some(user_xform));
            if a.has("uti") {
                (api.png_set_user_transform_info)(png, 0x77 as *mut c_void, a.i("utd", 8) as c_int, a.i("utc", 3) as c_int);
            }
            r.kv("utptr", (api.png_get_user_transform_ptr)(png) as usize);
            let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
            (api.png_read_update_info)(png, info);
            let rb = (api.png_get_rowbytes)(png, info);
            r.kv("rowbytes", rb);
            let hh = (api.png_get_image_height)(png, info);
            let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rb + 8]).collect();
            let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
            let _ = passes;
            (api.png_read_image)(png, rp.as_mut_ptr());
            (api.png_read_end)(png, end);
            let flat: Vec<u8> = rows.iter().flat_map(|v| v[..rb].to_vec()).collect();
            r.digest("rows", &flat);
            if flat.len() <= 2048 {
                r.bytes("rowsx", &flat);
            }
            let notes = std::mem::take(&mut g().notes);
            r.kv("notecount", notes.len());
            r.digest("notes", notes.join(";").as_bytes());
            let mut p = png;
            let mut ip = info;
            let mut ep = end;
            (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
        } else {
            let mut rng = Rng::new(seed);
            let (png, info) = new_write();
            (api.png_set_IHDR)(png, info, w, h, bd as c_int, ct as c_int, il as c_int, 0, 0);
            if ct == 3 {
                let pal = vec![png_color { red: 1, green: 2, blue: 3 }; 16];
                (api.png_set_PLTE)(png, info, pal.as_ptr(), 16);
            }
            (api.png_set_write_user_transform_fn)(png, Some(user_xform));
            if a.has("uti") {
                (api.png_set_user_transform_info)(png, 0x88 as *mut c_void, a.i("utd", 8) as c_int, a.i("utc", 3) as c_int);
            }
            r.kv("utptr", (api.png_get_user_transform_ptr)(png) as usize);
            (api.png_write_info)(png, info);
            let passes = if il == 1 { (api.png_set_interlace_handling)(png) } else { 1 };
            let irb = mkpng::rowbytes(w, bd, ct);
            let rows: Vec<Vec<u8>> = (0..h as usize)
                .map(|_| {
                    let mut v = vec![0u8; irb];
                    rng.fill(&mut v);
                    if ct == 3 {
                        for b in v.iter_mut() {
                            *b %= 16;
                        }
                    }
                    v
                })
                .collect();
            for _p in 0..passes {
                for row in rows.iter() {
                    (api.png_write_row)(png, row.as_ptr());
                }
            }
            (api.png_write_end)(png, info);
            let out = std::mem::take(&mut g().wbuf);
            if out.len() <= 4096 {
                r.bytes("out", &out);
            } else {
                r.digest("out", &out);
            }
            let notes = std::mem::take(&mut g().notes);
            r.kv("notecount", notes.len());
            r.digest("notes", notes.join(";").as_bytes());
            let mut p = png;
            let mut ip = info;
            (api.png_destroy_write_struct)(&mut p, &mut ip);
        }
    }
}

/* ------------------------------------------------------------------ */
/* MNG extensions (filter method 64, empty PLTE)                       */
/* ------------------------------------------------------------------ */

fn run_mng(a: &Args) {
    let api = api();
    let r = rec();
    let which = a.s("f", "filter64");
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let seed = a.u("seed", 1) as u64;
    let permit = a.u("permit", 5);

    unsafe {
        match which.as_str() {
            "filter64" | "filter64sig" => {
                // A stream whose IHDR declares the MNG intrapixel filter method.
                // libpng only *accepts* filter method 64 when it did not itself
                // validate the first 3 signature bytes (png.c: the
                // PNG_HAVE_PNG_SIGNATURE test), so the "sig" variant hands the
                // stream over with png_set_sig_bytes(4).
                let mut rng = Rng::new(seed);
                let w = a.u("w", 13);
                let h = a.u("h", 7);
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(w, h, bd, ct, 0, 64, 0));
                let rb = mkpng::rowbytes(w, bd, ct);
                let mut raw = Vec::new();
                for _ in 0..h {
                    raw.push(0u8);
                    let mut row = vec![0u8; rb];
                    rng.fill(&mut row);
                    raw.extend_from_slice(&row);
                }
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                r.digest("src", &v);
                let skip = if which == "filter64sig" { a.u("skip", 4) as usize } else { 0 };
                let (png, info, end) = new_read(&v[skip..]);
                if skip > 0 {
                    (api.png_set_sig_bytes)(png, skip as c_int);
                }
                r.kv("permit", (api.png_permit_mng_features)(png, permit));
                (api.png_read_info)(png, info);
                dump_info(png, info, r, "info");
                (api.png_read_update_info)(png, info);
                let rbb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rbb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                let flat: Vec<u8> = rows.concat();
                r.digest("rows", &flat);
                if flat.len() <= 2048 {
                    r.bytes("rowsx", &flat);
                }
                let mut p = png;
                let mut ip = info;
                let mut ep = end;
                (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
            }
            "sigbytes" => {
                let s = synth(ct, bd, 0, a.u("w", 11), a.u("h", 5), "gamatext", 0, seed);
                let skip = a.u("skip", 0) as usize;
                r.digest("src", &s.png);
                let (png, info, end) = new_read(&s.png[skip..]);
                (api.png_set_sig_bytes)(png, skip as c_int);
                (api.png_read_info)(png, info);
                dump_info(png, info, r, "info");
                let rbb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rbb.max(1)]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                r.digest("rows", &rows.concat());
                let mut p = png;
                let mut ip = info;
                let mut ep = end;
                (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
            }
            "write64" => {
                let mut rng = Rng::new(seed);
                let w = a.u("w", 13);
                let h = a.u("h", 7);
                let (png, info) = new_write();
                r.kv("permit", (api.png_permit_mng_features)(png, permit));
                (api.png_set_IHDR)(png, info, w, h, bd as c_int, ct as c_int, 0, 0, 64);
                (api.png_write_info)(png, info);
                let irb = mkpng::rowbytes(w, bd, ct);
                for _ in 0..h {
                    let mut row = vec![0u8; irb];
                    rng.fill(&mut row);
                    (api.png_write_row)(png, row.as_ptr());
                }
                (api.png_write_end)(png, info);
                let out = std::mem::take(&mut g().wbuf);
                r.bytes("out", &out);
                let mut p = png;
                let mut ip = info;
                (api.png_destroy_write_struct)(&mut p, &mut ip);
            }
            "emptyplte" => {
                let mut v = mkpng::SIG.to_vec();
                v.extend_from_slice(&mkpng::ihdr(4, 3, 8, 3, 0, 0, 0));
                v.extend_from_slice(&mkpng::chunk(b"PLTE", &[]));
                let raw = mkpng::filtered_none(&vec![vec![0u8; 4]; 3]);
                v.extend_from_slice(&mkpng::chunk(b"IDAT", &mkpng::zlib_stored(&raw)));
                v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
                r.digest("src", &v);
                let (png, info, end) = new_read(&v);
                r.kv("permit", (api.png_permit_mng_features)(png, permit));
                (api.png_read_info)(png, info);
                dump_info(png, info, r, "info");
                let rbb = (api.png_get_rowbytes)(png, info);
                let hh = (api.png_get_image_height)(png, info);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rbb.max(1)]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                (api.png_read_end)(png, end);
                r.digest("rows", &rows.concat());
                let mut p = png;
                let mut ip = info;
                let mut ep = end;
                (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
            }
            other => panic!("unknown mng f={other}"),
        }
    }
}

/* ------------------------------------------------------------------ */
/* CRC error x png_set_crc_action matrix                               */
/* ------------------------------------------------------------------ */

fn run_crc(a: &Args) {
    let api = api();
    let r = rec();
    let target = a.s("chunk", "gAMA");
    let crit = a.i("crit", 0) as c_int;
    let anc = a.i("anc", 0) as c_int;

    unsafe {
        let mut v = mkpng::SIG.to_vec();
        let ihdr_data = {
            let mut d = Vec::new();
            d.extend_from_slice(&6u32.to_be_bytes());
            d.extend_from_slice(&4u32.to_be_bytes());
            d.extend_from_slice(&[8, 2, 0, 0, 0]);
            d
        };
        if target == "IHDR" {
            v.extend_from_slice(&mkpng::chunk_bad_crc(b"IHDR", &ihdr_data));
        } else {
            v.extend_from_slice(&mkpng::chunk(b"IHDR", &ihdr_data));
        }
        let gama = 45455u32.to_be_bytes();
        if target == "gAMA" {
            v.extend_from_slice(&mkpng::chunk_bad_crc(b"gAMA", &gama));
        } else {
            v.extend_from_slice(&mkpng::chunk(b"gAMA", &gama));
        }
        let raw = mkpng::filtered_none(&vec![vec![0x33u8; 18]; 4]);
        let z = mkpng::zlib_stored(&raw);
        if target == "IDAT" {
            v.extend_from_slice(&mkpng::chunk_bad_crc(b"IDAT", &z));
        } else {
            v.extend_from_slice(&mkpng::chunk(b"IDAT", &z));
        }
        if target == "tEXt" {
            v.extend_from_slice(&mkpng::chunk_bad_crc(b"tEXt", b"Key\0value"));
        } else {
            v.extend_from_slice(&mkpng::chunk(b"tEXt", b"Key\0value"));
        }
        if target == "IEND" {
            v.extend_from_slice(&mkpng::chunk_bad_crc(b"IEND", &[]));
        } else {
            v.extend_from_slice(&mkpng::chunk(b"IEND", &[]));
        }
        r.digest("src", &v);
        let (png, info, end) = new_read(&v);
        (api.png_set_crc_action)(png, crit, anc);
        (api.png_read_info)(png, info);
        dump_info(png, info, r, "info");
        let rb = (api.png_get_rowbytes)(png, info);
        let hh = (api.png_get_image_height)(png, info);
        let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rb.max(1)]).collect();
        let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
        (api.png_read_image)(png, rp.as_mut_ptr());
        (api.png_read_end)(png, end);
        dump_info(png, end, r, "end");
        r.bytes("rows", &rows.concat());
        let mut p = png;
        let mut ip = info;
        let mut ep = end;
        (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
    }
}

/* ------------------------------------------------------------------ */
/* floating-point getters                                              */
/* ------------------------------------------------------------------ */

fn run_fpget(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    unsafe {
        let mut rng = Rng::new(seed);
        let (png, info) = new_write();
        (api.png_set_benign_errors)(png, 1);
        for i in 0..80 {
            let mut v = [0i32; 8];
            for x in v.iter_mut() {
                *x = (rng.next_u32() % 120_000) as i32;
            }
            (api.png_set_cHRM_fixed)(png, info, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
            let mut d = [0f64; 8];
            let ret = (api.png_get_cHRM)(
                png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4], &mut d[5],
                &mut d[6], &mut d[7],
            );
            r.kv(
                &format!("chrm/{i}"),
                format!("{ret} {}", d.iter().map(|x| format!("{x:.9}")).collect::<Vec<_>>().join(",")),
            );
            let mut x = [0f64; 9];
            let ret2 = (api.png_get_cHRM_XYZ)(
                png, info, &mut x[0], &mut x[1], &mut x[2], &mut x[3], &mut x[4], &mut x[5],
                &mut x[6], &mut x[7], &mut x[8],
            );
            r.kv(
                &format!("chrmxyz/{i}"),
                format!("{ret2} {}", x.iter().map(|v| format!("{v:.9}")).collect::<Vec<_>>().join(",")),
            );
            (api.png_set_invalid)(png, info, PNG_INFO_cHRM as c_int);

            let l1 = rng.next_u32() & 0x7fff_ffff;
            let l2 = rng.next_u32() & 0x7fff_ffff;
            (api.png_set_cLLI_fixed)(png, info, l1, l2);
            let (mut c1, mut c2) = (0f64, 0f64);
            r.kv(
                &format!("clli/{i}"),
                format!("{} {c1:.9} {c2:.9}", (api.png_get_cLLI)(png, info, &mut c1, &mut c2)),
            );
            (api.png_set_invalid)(png, info, PNG_INFO_cLLI as c_int);

            let mut m = [0i32; 8];
            for x in m.iter_mut() {
                *x = (rng.next_u32() % 131_000) as i32;
            }
            (api.png_set_mDCV_fixed)(png, info, m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], l1 % 100_000, l2 % 100_000);
            let mut md = [0f64; 10];
            let retm = (api.png_get_mDCV)(
                png, info, &mut md[0], &mut md[1], &mut md[2], &mut md[3], &mut md[4], &mut md[5],
                &mut md[6], &mut md[7], &mut md[8], &mut md[9],
            );
            r.kv(
                &format!("mdcv/{i}"),
                format!("{retm} {}", md.iter().map(|v| format!("{v:.9}")).collect::<Vec<_>>().join(",")),
            );
            (api.png_set_invalid)(png, info, PNG_INFO_mDCV as c_int);

            let unit = 1 + (rng.below(2)) as c_int;
            let sw = format!("{}.{}", 1 + rng.below(500), rng.below(100000));
            let sh = format!("{}.{}", 1 + rng.below(500), rng.below(100000));
            let cw = cs(&sw);
            let ch = cs(&sh);
            (api.png_set_sCAL_s)(png, info, unit, cw.as_ptr(), ch.as_ptr());
            let mut u = 0i32;
            let (mut fw, mut fh) = (0f64, 0f64);
            r.kv(
                &format!("scal/{i}"),
                format!("{} {u} {fw:.9} {fh:.9}", (api.png_get_sCAL)(png, info, &mut u, &mut fw, &mut fh)),
            );
            (api.png_set_invalid)(png, info, PNG_INFO_sCAL as c_int);

            let rx = rng.next_u32() % 100_000;
            let ry = rng.next_u32() % 100_000;
            (api.png_set_pHYs)(png, info, rx, ry, PNG_RESOLUTION_METER);
            r.kv(&format!("aspect/{i}"), format!("{:.9}", (api.png_get_pixel_aspect_ratio)(png, info)));
            let ox = rng.next_u32() as i32;
            let oy = rng.next_u32() as i32;
            (api.png_set_oFFs)(png, info, ox, oy, PNG_OFFSET_MICROMETER);
            r.kv(&format!("xin/{i}"), format!("{:.9}", (api.png_get_x_offset_inches)(png, info)));
            r.kv(&format!("yin/{i}"), format!("{:.9}", (api.png_get_y_offset_inches)(png, info)));

            let gf = (rng.next_u32() % 500_000) as i32;
            (api.png_set_gAMA_fixed)(png, info, gf);
            let mut gd = 0f64;
            r.kv(&format!("gama/{i}"), format!("{} {gd:.12}", (api.png_get_gAMA)(png, info, &mut gd)));
            (api.png_set_invalid)(png, info, PNG_INFO_gAMA as c_int);
        }
        let mut p = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut p, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* stdio-based entry points                                            */
/* ------------------------------------------------------------------ */

fn run_fileio(a: &Args) {
    let api = api();
    let r = rec();
    let ct = a.u("ct", 2) as u8;
    let bd = a.u("bd", 8) as u8;
    let w = a.u("w", 15);
    let h = a.u("h", 9);
    let seed = a.u("seed", 1) as u64;
    let mode = a.s("m", "lowlevel");

    unsafe {
        let dir = std::env::temp_dir().join(format!("pngdiff-fileio-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.png");
        let cpath = cs(path.to_str().unwrap());

        match mode.as_str() {
            "lowlevel" => {
                // write via png_init_io, then read it back via png_init_io
                let mut rng = Rng::new(seed);
                let fmode = cs("wb");
                let f = super::api_fopen(cpath.as_ptr(), fmode.as_ptr());
                r.kv("fopen_w", !f.is_null());
                let (png, info) = {
                    let png = (api.png_create_write_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
                    let info = (api.png_create_info_struct)(png);
                    (api.png_init_io)(png, f);
                    (png, info)
                };
                (api.png_set_IHDR)(png, info, w, h, bd as c_int, ct as c_int, 0, 0, 0);
                if ct == 3 {
                    let pal = vec![png_color { red: 9, green: 8, blue: 7 }; 8];
                    (api.png_set_PLTE)(png, info, pal.as_ptr(), 8);
                }
                (api.png_write_info)(png, info);
                let irb = mkpng::rowbytes(w, bd, ct);
                for _ in 0..h {
                    let mut row = vec![0u8; irb];
                    rng.fill(&mut row);
                    if ct == 3 {
                        for b in row.iter_mut() {
                            *b %= 8;
                        }
                    }
                    (api.png_write_row)(png, row.as_ptr());
                }
                (api.png_write_end)(png, info);
                let mut p = png;
                let mut ip = info;
                (api.png_destroy_write_struct)(&mut p, &mut ip);
                super::api_fclose(f);

                let bytes = std::fs::read(&path).unwrap_or_default();
                r.bytes("file", &bytes);

                let rmode = cs("rb");
                let f2 = super::api_fopen(cpath.as_ptr(), rmode.as_ptr());
                r.kv("fopen_r", !f2.is_null());
                let png2 = (api.png_create_read_struct)(ver(), std::ptr::null_mut(), Some(cb_error), Some(cb_warn));
                let info2 = (api.png_create_info_struct)(png2);
                let end2 = (api.png_create_info_struct)(png2);
                (api.png_init_io)(png2, f2);
                (api.png_read_info)(png2, info2);
                dump_info(png2, info2, r, "info");
                let rb = (api.png_get_rowbytes)(png2, info2);
                let hh = (api.png_get_image_height)(png2, info2);
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png2, rp.as_mut_ptr());
                (api.png_read_end)(png2, end2);
                r.digest("rows", &rows.concat());
                let mut p2 = png2;
                let mut ip2 = info2;
                let mut ep2 = end2;
                (api.png_destroy_read_struct)(&mut p2, &mut ip2, &mut ep2);
                super::api_fclose(f2);
            }
            "simple" => {
                let mut rng = Rng::new(seed);
                let fmt = a.u("fmt", PNG_FORMAT_RGBA);
                let mut img = png_image::default();
                img.width = w;
                img.height = h;
                img.format = fmt;
                let sz = image_size(&img).max(1);
                let mut buf = vec![0u8; sz];
                rng.fill(&mut buf);
                let ok = (api.png_image_write_to_file)(
                    &mut img,
                    cpath.as_ptr(),
                    a.i("c8", 0) as c_int,
                    buf.as_ptr() as *const c_void,
                    0,
                    std::ptr::null(),
                );
                r.kv("write", format!("{ok} {:?}", img.msg()));
                (api.png_image_free)(&mut img);
                let bytes = std::fs::read(&path).unwrap_or_default();
                if bytes.len() <= 4096 {
                    r.bytes("file", &bytes);
                } else {
                    r.digest("file", &bytes);
                }

                let mut img2 = png_image::default();
                let rok = (api.png_image_begin_read_from_file)(&mut img2, cpath.as_ptr());
                r.kv(
                    "begin",
                    format!("{rok} {} {} {:x} {:?}", img2.width, img2.height, img2.format, img2.msg()),
                );
                if rok != 0 {
                    img2.format = fmt;
                    let bs = image_size(&img2).max(1);
                    let mut b2 = vec![0xA5u8; bs];
                    let mut cm = vec![0u8; 4 * 256];
                    let rok2 = (api.png_image_finish_read)(
                        &mut img2,
                        std::ptr::null(),
                        b2.as_mut_ptr() as *mut c_void,
                        image_row_stride(&img2) as i32,
                        cm.as_mut_ptr() as *mut c_void,
                    );
                    r.kv("finish", format!("{rok2} {:?}", img2.msg()));
                    r.digest("buf", &b2);
                }
                (api.png_image_free)(&mut img2);
            }
            "stdio" => {
                let mut rng = Rng::new(seed);
                let fmt = a.u("fmt", PNG_FORMAT_RGB);
                let mut img = png_image::default();
                img.width = w;
                img.height = h;
                img.format = fmt;
                let sz = image_size(&img).max(1);
                let mut buf = vec![0u8; sz];
                rng.fill(&mut buf);
                let wmode = cs("wb");
                let f = super::api_fopen(cpath.as_ptr(), wmode.as_ptr());
                let ok = (api.png_image_write_to_stdio)(
                    &mut img,
                    f,
                    0,
                    buf.as_ptr() as *const c_void,
                    0,
                    std::ptr::null(),
                );
                r.kv("write", format!("{ok} {:?}", img.msg()));
                (api.png_image_free)(&mut img);
                super::api_fclose(f);
                let bytes = std::fs::read(&path).unwrap_or_default();
                r.digest("file", &bytes);

                let rmode = cs("rb");
                let f2 = super::api_fopen(cpath.as_ptr(), rmode.as_ptr());
                let mut img2 = png_image::default();
                let rok = (api.png_image_begin_read_from_stdio)(&mut img2, f2);
                r.kv("begin", format!("{rok} {} {} {:x}", img2.width, img2.height, img2.format));
                if rok != 0 {
                    img2.format = fmt;
                    let bs = image_size(&img2).max(1);
                    let mut b2 = vec![0xA5u8; bs];
                    let rok2 = (api.png_image_finish_read)(
                        &mut img2,
                        std::ptr::null(),
                        b2.as_mut_ptr() as *mut c_void,
                        image_row_stride(&img2) as i32,
                        std::ptr::null_mut(),
                    );
                    r.kv("finish", format!("{rok2} {:?}", img2.msg()));
                    r.digest("buf", &b2);
                }
                (api.png_image_free)(&mut img2);
                super::api_fclose(f2);
            }
            other => panic!("unknown fileio mode {other}"),
        }
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}

/* ------------------------------------------------------------------ */
/* png_free_data / png_data_freer / png_destroy_info_struct            */
/* ------------------------------------------------------------------ */

fn run_freedata(a: &Args) {
    let api = api();
    let r = rec();
    let mask = a.u("mask", 0xffff);
    unsafe {
        let s = synth(3, 8, 0, 9, 5, "gamachrmsbittrnsbkgdhistphysoffsscalpcalsplttextexificcpunktail", 0, 4242);
        let (png, info, end) = new_read(&s.png);
        (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
        (api.png_read_info)(png, info);
        dump_info(png, info, r, "before");
        (api.png_free_data)(png, info, mask, -1);
        dump_info(png, info, r, "after");
        (api.png_data_freer)(png, info, 2, mask);
        (api.png_data_freer)(png, info, 1, mask);
        (api.png_set_invalid)(png, info, 0xffff);
        dump_info(png, info, r, "invalidated");
        let mut ip2 = info;
        (api.png_destroy_info_struct)(png, &mut ip2);
        r.kv("destroyed", ip2 as usize == 0);
        let mut p = png;
        let mut ep = end;
        (api.png_destroy_read_struct)(&mut p, std::ptr::null_mut(), &mut ep);
        r.line("done");
    }
}

/* ------------------------------------------------------------------ */
/* deprecated filter heuristics                                        */
/* ------------------------------------------------------------------ */

fn run_heuristics(a: &Args) {
    let api = api();
    let r = rec();
    let method = a.i("hm", 1) as c_int;
    let nw = a.i("nw", 3) as c_int;
    unsafe {
        let mut rng = Rng::new(a.u("seed", 1) as u64);
        let (png, info) = new_write();
        (api.png_set_benign_errors)(png, 1);
        let weights: Vec<f64> = (0..nw.max(0) as usize).map(|i| 1.0 + i as f64).collect();
        let costs: Vec<f64> = (0..5usize).map(|i| 1.0 + 0.25 * i as f64).collect();
        (api.png_set_filter_heuristics)(
            png,
            method,
            nw,
            if weights.is_empty() { std::ptr::null() } else { weights.as_ptr() },
            costs.as_ptr(),
        );
        let fw: Vec<i32> = (0..nw.max(0) as usize).map(|i| 100000 + i as i32 * 1000).collect();
        let fc: Vec<i32> = (0..5usize).map(|i| 100000 + i as i32 * 25000).collect();
        (api.png_set_filter_heuristics_fixed)(
            png,
            method,
            nw,
            if fw.is_empty() { std::ptr::null() } else { fw.as_ptr() },
            fc.as_ptr(),
        );
        (api.png_set_IHDR)(png, info, 33, 11, 8, PNG_COLOR_TYPE_RGB, 0, 0, 0);
        (api.png_set_filter)(png, 0, PNG_ALL_FILTERS);
        (api.png_write_info)(png, info);
        for _ in 0..11 {
            let mut row = vec![0u8; 99];
            rng.fill(&mut row);
            (api.png_write_row)(png, row.as_ptr());
        }
        (api.png_write_end)(png, info);
        let out = std::mem::take(&mut g().wbuf);
        r.bytes("out", &out);
        let mut p = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut p, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* mutation fuzzing of a valid datastream                              */
/* ------------------------------------------------------------------ */

/// Take a rich but valid PNG, corrupt `k` bytes at pseudo-random offsets and
/// read it end to end.  This drives the chunk-level rejection paths in
/// `pngrutil.c` in bulk: bad lengths, bad chunk names, CRC failures, malformed
/// chunk payloads, broken zlib streams, out-of-place chunks.
fn run_mutate(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    let k = a.u("k", 1);
    let iters = a.u("n", 8);
    let x = a.s("x", "gamachrmsbittrnsbkgdphysoffsscalpcalsplttextexifcicpcllimdcviccpunktail");
    let benign = a.i("benign", -1);
    let fixcrc = a.i("fixcrc", 0);

    unsafe {
        let base = synth(
            a.u("ct", 3) as u8,
            a.u("bd", 8) as u8,
            a.u("il", 0) as u8,
            a.u("w", 12),
            a.u("h", 6),
            &x,
            a.u("split", 0) as usize,
            seed,
        )
        .png;
        r.kv("baselen", base.len());
        for it in 0..iters {
            let mut rng = Rng::new(seed ^ ((it as u64 + 1) * 0x9E37_79B9_7F4A_7C15));
            let mut data = base.clone();
            for _ in 0..k {
                // never touch the 8 signature bytes: those paths are covered
                // explicitly and would stop every mutation at the same place
                let off = 8 + (rng.next_u32() as usize % (data.len() - 8));
                data[off] ^= 1u8 << rng.below(8);
            }
            if fixcrc != 0 {
                data = recrc(&data);
            }
            r.digest(&format!("m/{it}/src"), &data);

            let (png, info, end) = new_read(&data);
            if benign >= 0 {
                (api.png_set_benign_errors)(png, benign as c_int);
            }
            (api.png_read_info)(png, info);
            dump_info(png, info, r, &format!("m/{it}/info"));
            let rb = (api.png_get_rowbytes)(png, info);
            let hh = (api.png_get_image_height)(png, info);
            if hh > 0 && hh <= 4096 && rb > 0 && rb <= (1 << 20) {
                let mut rows: Vec<Vec<u8>> = (0..hh as usize).map(|_| vec![0xA5u8; rb]).collect();
                let mut rp: Vec<*mut u8> = rows.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(png, rp.as_mut_ptr());
                r.digest(&format!("m/{it}/rows"), &rows.concat());
            } else {
                r.kv(&format!("m/{it}/rows"), "skipped");
            }
            (api.png_read_end)(png, end);
            dump_info(png, end, r, &format!("m/{it}/end"));
            r.kv(&format!("m/{it}/consumed"), g().rpos);
            let mut p = png;
            let mut ip = info;
            let mut ep = end;
            (api.png_destroy_read_struct)(&mut p, &mut ip, &mut ep);
        }
    }
}

/// Recompute every chunk CRC so a mutation exercises the *content* checks
/// rather than stopping at the CRC test.
fn recrc(data: &[u8]) -> Vec<u8> {
    let mut out = data[..8].to_vec();
    let mut i = 8usize;
    while i + 8 <= data.len() {
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if len > 0x7fff_ffff || i + 12 + len > data.len() {
            out.extend_from_slice(&data[i..]);
            return out;
        }
        let name: [u8; 4] = [data[i + 4], data[i + 5], data[i + 6], data[i + 7]];
        let body = &data[i + 8..i + 8 + len];
        out.extend_from_slice(&mkpng::chunk(&name, body));
        i += 12 + len;
    }
    if i < data.len() {
        out.extend_from_slice(&data[i..]);
    }
    out
}

/* ------------------------------------------------------------------ */
/* simplified-API fuzzing                                              */
/* ------------------------------------------------------------------ */

fn run_simple_fuzz(a: &Args) {
    let api = api();
    let r = rec();
    let seed = a.u("seed", 1) as u64;
    let iters = a.u("n", 8);
    let mut rng = Rng::new(seed);
    let cts: [(u8, u8); 15] = [
        (0, 1), (0, 2), (0, 4), (0, 8), (0, 16),
        (2, 8), (2, 16),
        (3, 1), (3, 2), (3, 4), (3, 8),
        (4, 8), (4, 16),
        (6, 8), (6, 16),
    ];
    let fmts: [u32; 19] = [
        0, 1, 0x21, 2, 0x12, 3, 0x23, 0x13, 0x33, 4, 5, 6, 7, 8, 9, 0x0a, 0x0b, 0x1a, 0x2b,
    ];
    let extras = ["none", "gama", "srgb", "chrm", "trns", "bkgd", "iccp", "gamachrm", "sbit"];

    unsafe {
        for it in 0..iters {
            let (ct, bd) = rng.pick(&cts);
            let il = (rng.below(2)) as u8;
            let w = rng.range(1, 33);
            let h = rng.range(1, 17);
            let x = rng.pick(&extras);
            let fmt = rng.pick(&fmts);
            let neg = rng.bool();
            let usebg = rng.bool();
            let flags = rng.pick(&[0u32, 1, 2, 4, 5]);
            let s = synth(ct, bd, il, w, h, x, 0, seed ^ (it as u64 * 7717));
            r.kv(
                &format!("s/{it}/cfg"),
                format!("ct={ct} bd={bd} il={il} {w}x{h} x={x} fmt={fmt:#x} neg={neg} bg={usebg} flags={flags:#x}"),
            );
            r.digest(&format!("s/{it}/src"), &s.png);

            let mut img = png_image::default();
            let ok = (api.png_image_begin_read_from_memory)(
                &mut img,
                s.png.as_ptr() as *const c_void,
                s.png.len(),
            );
            r.kv(
                &format!("s/{it}/begin"),
                format!("{ok} {} {} {:x} {:x} {} {} {:?}", img.width, img.height, img.format,
                        img.flags, img.colormap_entries, img.warning_or_error, img.msg()),
            );
            if ok == 0 {
                (api.png_image_free)(&mut img);
                continue;
            }
            img.format = fmt;
            img.flags |= flags;
            let stride0 = image_row_stride(&img) as i32;
            let stride = if neg { -stride0 } else { stride0 };
            let bufsz = image_size(&img).max(1);
            let cmsz = colormap_size(&img).max(4 * 256);
            // See run_simple_read: libpng offsets the bottom-up base itself and
            // mixes byte / component units while doing it, so leave slack on
            // both sides and compare it.
            let pad = if stride < 0 { 4 * bufsz + 64 } else { 0 };
            let mut buf = vec![0xA5u8; pad + bufsz + pad + 16];
            let mut cmap = vec![0x5Au8; cmsz + 16];
            let bg = png_color { red: 0x11, green: 0x77, blue: 0xdd };
            let ptr = buf.as_mut_ptr().add(pad);
            let ok2 = (api.png_image_finish_read)(
                &mut img,
                if usebg { &bg } else { std::ptr::null() },
                ptr as *mut c_void,
                stride,
                cmap.as_mut_ptr() as *mut c_void,
            );
            r.kv(
                &format!("s/{it}/finish"),
                format!("{ok2} {} {} {:?}", img.colormap_entries, img.warning_or_error, img.msg()),
            );
            r.digest(&format!("s/{it}/buf"), &buf[pad..pad + bufsz]);
            r.digest(&format!("s/{it}/before"), &buf[..pad]);
            r.digest(&format!("s/{it}/after"), &buf[pad + bufsz..]);
            r.digest(&format!("s/{it}/cmap"), &cmap[..colormap_size(&img).min(cmap.len())]);
            (api.png_image_free)(&mut img);
        }
    }
}
