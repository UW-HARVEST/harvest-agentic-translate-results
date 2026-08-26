//! Phase C — the read/write transform pipeline.
//!
//! Covers CONFIGS.md rows
//!   * C-67 … C-93  → `single`, `single_2` … `single_6` (one read transform at a time)
//!   * C-94         → `combinations`
//!   * C-7          → `gamma_tables`
//!   * C-95         → `update_info`
//!   * C-96 … C-105 → `write_side`
//!   * C-106        → `mng_intrapixel`
//!
//! Every case builds a random source image, writes it once with the **C**
//! library to get a byte-identical input file, then reads that file back with
//! *both* libraries with the transform(s) installed and compares the decoded
//! rows plus everything `png_get_*` reports and every warning.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_int, c_void};

/* ------------------------------------------------------------------ */
/* plumbing                                                            */
/* ------------------------------------------------------------------ */

/// A stable address handed to `png_set_user_transform_info` so that
/// `png_get_user_transform_ptr` can be checked.
static COOKIE: u8 = 0x5a;

fn cookie() -> *mut c_void {
    &COOKIE as *const u8 as *mut c_void
}

/// Install a fresh `Tls` and make the C `Api` current for the duration of `f`.
/// Used to build the reference input files outside of `assert_same`.
fn with_c_tls<R>(f: impl FnOnce(&'static Api) -> R) -> R {
    let mut state = Box::new(Tls::default());
    let prev = set_tls(&mut *state as *mut Tls);
    let api: &'static Api = &libs().c;
    let prev_api = set_cur_api(api as *const Api);
    let r = f(api);
    set_cur_api(prev_api);
    set_tls(prev);
    r
}

/* ------------------------------------------------------------------ */
/* the ancillary chunks a source file may carry                        */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Aux {
    trns: bool,
    sbit: bool,
    bkgd: bool,
    hist: bool,
    chrm: bool,
    gama: Option<i32>,
    srgb: Option<c_int>,
}

const AUX_NONE: Aux = Aux {
    trns: false,
    sbit: false,
    bkgd: false,
    hist: false,
    chrm: false,
    gama: None,
    srgb: None,
};

const AUX_TRNS: Aux = Aux {
    trns: true,
    ..AUX_NONE
};

const AUX_TRNS_BKGD: Aux = Aux {
    trns: true,
    bkgd: true,
    ..AUX_NONE
};

const AUX_ALL: Aux = Aux {
    trns: true,
    sbit: true,
    bkgd: true,
    hist: true,
    chrm: true,
    gama: Some(45455),
    srgb: None,
};

/// Install `aux` on a *write* struct; runs between `png_set_PLTE` and
/// `png_write_info`.
unsafe fn install_aux(
    api: &Api,
    png: *mut PngStruct,
    info: *mut PngInfo,
    img: &Img,
    aux: &Aux,
    seed: u64,
) {
    let mut rng = Rng::new(seed ^ 0xa11c_0de5);
    let bd = img.bit_depth as u32;
    let gray_max: u32 = 1u32 << bd; // 1<<16 fits in u32
    let comp_max: u32 = if bd == 16 { 65536 } else { 256 };

    if aux.trns {
        match img.color_type {
            PNG_COLOR_TYPE_PALETTE => {
                let n = img.palette.len();
                let t: Vec<u8> = (0..n).map(|_| rng.pick(&[0u8, 1, 0x40, 0x80, 0xfe, 0xff])).collect();
                (api.png_set_tRNS)(png, info, t.as_ptr(), n as c_int, core::ptr::null());
            }
            PNG_COLOR_TYPE_GRAY => {
                let c = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: (rng.u32() % gray_max) as u16,
                };
                (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
            }
            PNG_COLOR_TYPE_RGB => {
                let c = png_color_16 {
                    index: 0,
                    red: (rng.u32() % comp_max) as u16,
                    green: (rng.u32() % comp_max) as u16,
                    blue: (rng.u32() % comp_max) as u16,
                    gray: 0,
                };
                (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
            }
            // libpng refuses tRNS with an alpha channel (a fatal app warning on
            // a write struct), so do not even try.
            _ => {}
        }
    }

    if aux.sbit {
        let maxb = if img.color_type == PNG_COLOR_TYPE_PALETTE {
            8u32
        } else {
            bd
        };
        // Per channel, so that the maximum over R/G/B (which is what selects
        // the 16-bit gamma table shift in png_build_gamma_table) differs from
        // the individual values.
        let s = png_color_8 {
            red: (1 + rng.u32() % maxb) as u8,
            green: (1 + rng.u32() % maxb) as u8,
            blue: (1 + rng.u32() % maxb) as u8,
            gray: (1 + rng.u32() % maxb) as u8,
            alpha: (1 + rng.u32() % maxb) as u8,
        };
        (api.png_set_sBIT)(png, info, &s);
    }

    if aux.bkgd {
        let b = if img.color_type == PNG_COLOR_TYPE_PALETTE {
            png_color_16 {
                index: rng.below(img.palette.len().max(1)) as u8,
                red: 0,
                green: 0,
                blue: 0,
                gray: 0,
            }
        } else if img.color_type & PNG_COLOR_MASK_COLOR != 0 {
            png_color_16 {
                index: 0,
                red: (rng.u32() % comp_max) as u16,
                green: (rng.u32() % comp_max) as u16,
                blue: (rng.u32() % comp_max) as u16,
                gray: 0,
            }
        } else {
            png_color_16 {
                index: 0,
                red: 0,
                green: 0,
                blue: 0,
                gray: (rng.u32() % gray_max) as u16,
            }
        };
        (api.png_set_bKGD)(png, info, &b);
    }

    if aux.hist && img.color_type == PNG_COLOR_TYPE_PALETTE {
        let h: Vec<u16> = (0..img.palette.len())
            .map(|_| (rng.u32() % 5000) as u16)
            .collect();
        (api.png_set_hIST)(png, info, h.as_ptr());
    }

    if let Some(g) = aux.gama {
        (api.png_set_gAMA_fixed)(png, info, g);
    }
    if let Some(i) = aux.srgb {
        (api.png_set_sRGB)(png, info, i);
    }
    if aux.chrm && aux.srgb.is_none() {
        // sRGB-ish primaries
        (api.png_set_cHRM_fixed)(
            png, info, 31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000,
        );
    }
}

/// Write `img` (+ `aux`) with the **C** library and return the file bytes.
unsafe fn make_file(img: &Img, aux: &Aux, seed: u64) -> Vec<u8> {
    with_c_tls(|api| {
        let w = write_image(api, img, &WriteOpts::default(), &mut |a, png, info| {
            // png_struct::num_palette_max is 0 (i.e. "check enabled") in a
            // freshly calloc'ed struct, and a write-side benign error is fatal
            // in this build, so the reference writer would refuse to store the
            // deliberately out-of-range palette indices the C-93 cases need.
            (a.png_set_check_for_invalid_index)(png, 0);
            install_aux(a, png, info, img, aux, seed)
        });
        assert_eq!(
            w.guard,
            Guard::Ok,
            "reference write failed for ct={} bd={} aux={:?}: {:?}",
            img.color_type,
            img.bit_depth,
            aux,
            tls().trace
        );
        assert!(!w.bytes.is_empty());
        w.bytes
    })
}

/* ------------------------------------------------------------------ */
/* the read transforms                                                 */
/* ------------------------------------------------------------------ */

#[derive(Clone, Debug)]
enum Tr {
    PaletteToRgb,
    ExpandGray,
    TrnsToAlpha,
    Expand,
    Expand16,
    Strip16,
    Scale16,
    StripAlpha,
    SwapAlpha,
    InvertAlpha,
    Filler(u32, c_int),
    AddAlpha(u32, c_int),
    Bgr,
    Swap,
    Packing,
    Packswap,
    Shift(png_color_8),
    InvertMono,
    GrayToRgb,
    /// `(use the _fixed entry point, error action, red, green)` — coefficients
    /// are fixed point; the floating point entry point gets `x / 100000.0`.
    RgbToGray(bool, c_int, i32, i32),
    /// `(palette, maximum_colors, histogram, full_quantize)`
    Quantize(Vec<png_color>, c_int, Option<Vec<u16>>, c_int),
    /// `(fixed, background, gamma code, need_expand, background gamma)`
    Background(bool, png_color_16, c_int, c_int, i32),
    /// `(fixed, mode, output gamma)`
    AlphaMode(bool, c_int, i32),
    /// `(fixed, screen gamma, file gamma)`
    Gamma(bool, i32, i32),
    Interlace,
    /// `png_set_read_user_transform_fn` + `(user depth, user channels)`
    User(c_int, c_int),
    CheckIndex(c_int),
}

const N_KINDS: usize = 27;

unsafe fn apply(api: &Api, png: *mut PngStruct, t: &Tr) {
    match t {
        Tr::PaletteToRgb => (api.png_set_palette_to_rgb)(png),
        Tr::ExpandGray => (api.png_set_expand_gray_1_2_4_to_8)(png),
        Tr::TrnsToAlpha => (api.png_set_tRNS_to_alpha)(png),
        Tr::Expand => (api.png_set_expand)(png),
        Tr::Expand16 => (api.png_set_expand_16)(png),
        Tr::Strip16 => (api.png_set_strip_16)(png),
        Tr::Scale16 => (api.png_set_scale_16)(png),
        Tr::StripAlpha => (api.png_set_strip_alpha)(png),
        Tr::SwapAlpha => (api.png_set_swap_alpha)(png),
        Tr::InvertAlpha => (api.png_set_invert_alpha)(png),
        Tr::Filler(v, loc) => (api.png_set_filler)(png, *v, *loc),
        Tr::AddAlpha(v, loc) => (api.png_set_add_alpha)(png, *v, *loc),
        Tr::Bgr => (api.png_set_bgr)(png),
        Tr::Swap => (api.png_set_swap)(png),
        Tr::Packing => (api.png_set_packing)(png),
        Tr::Packswap => (api.png_set_packswap)(png),
        Tr::Shift(s) => (api.png_set_shift)(png, s),
        Tr::InvertMono => (api.png_set_invert_mono)(png),
        Tr::GrayToRgb => (api.png_set_gray_to_rgb)(png),
        Tr::RgbToGray(fixed, action, r, g) => {
            if *fixed {
                (api.png_set_rgb_to_gray_fixed)(png, *action, *r, *g);
            } else {
                (api.png_set_rgb_to_gray)(
                    png,
                    *action,
                    *r as f64 / 100000.0,
                    *g as f64 / 100000.0,
                );
            }
            log(format!(
                "rgb_to_gray_status(after set)={}",
                (api.png_get_rgb_to_gray_status)(png)
            ));
        }
        Tr::Quantize(pal, max, hist, full) => {
            // png_set_quantize rewrites the caller's palette in place, so hand
            // it a fresh copy each time or the second library would see
            // different input.
            let mut p = pal.clone();
            let hp = match hist {
                Some(h) => h.as_ptr(),
                None => core::ptr::null(),
            };
            (api.png_set_quantize)(png, p.as_mut_ptr(), p.len() as c_int, *max, hp, *full);
            log(format!("quantize palette after={:?}", p));
        }
        Tr::Background(fixed, back, code, expand, gamma) => {
            if *fixed {
                (api.png_set_background_fixed)(png, back, *code, *expand, *gamma);
            } else {
                (api.png_set_background)(png, back, *code, *expand, *gamma as f64 / 100000.0);
            }
        }
        Tr::AlphaMode(fixed, mode, g) => {
            if *fixed {
                (api.png_set_alpha_mode_fixed)(png, *mode, *g);
            } else {
                (api.png_set_alpha_mode)(png, *mode, *g as f64 / 100000.0);
            }
        }
        Tr::Gamma(fixed, scrn, file) => {
            if *fixed {
                (api.png_set_gamma_fixed)(png, *scrn, *file);
            } else {
                (api.png_set_gamma)(png, *scrn as f64 / 100000.0, *file as f64 / 100000.0);
            }
        }
        Tr::Interlace => {
            log(format!(
                "interlace_handling={}",
                (api.png_set_interlace_handling)(png)
            ));
        }
        Tr::User(depth, ch) => {
            (api.png_set_read_user_transform_fn)(png, Some(user_transform_cb));
            (api.png_set_user_transform_info)(png, cookie(), *depth, *ch);
            log(format!(
                "user_transform_ptr_ok={}",
                (api.png_get_user_transform_ptr)(png) == cookie()
            ));
        }
        Tr::CheckIndex(allowed) => (api.png_set_check_for_invalid_index)(png, *allowed),
    }
}

/// Everything the info struct can tell us that `log_info` does not cover.
unsafe fn log_extra(api: &Api, png: *mut PngStruct, info: *mut PngInfo, tag: &str) {
    let mut pal: *mut png_color = core::ptr::null_mut();
    let mut np: c_int = 0;
    if (api.png_get_PLTE)(png, info, &mut pal, &mut np) != 0 && !pal.is_null() && np > 0 {
        log(format!(
            "{}: PLTE n={} {:?}",
            tag,
            np,
            core::slice::from_raw_parts(pal, np as usize)
        ));
    } else {
        log(format!("{}: PLTE none", tag));
    }

    let mut ta: *mut u8 = core::ptr::null_mut();
    let mut nt: c_int = 0;
    let mut tc: *mut png_color_16 = core::ptr::null_mut();
    if (api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc) != 0 {
        let alphas = if !ta.is_null() && nt > 0 {
            format!("{:?}", core::slice::from_raw_parts(ta, nt as usize))
        } else {
            "-".to_string()
        };
        let col = if tc.is_null() {
            "-".to_string()
        } else {
            format!("{:?}", *tc)
        };
        log(format!("{}: tRNS n={} a={} c={}", tag, nt, alphas, col));
    } else {
        log(format!("{}: tRNS none", tag));
    }

    let mut bk: *mut png_color_16 = core::ptr::null_mut();
    if (api.png_get_bKGD)(png, info, &mut bk) != 0 && !bk.is_null() {
        log(format!("{}: bKGD {:?}", tag, *bk));
    } else {
        log(format!("{}: bKGD none", tag));
    }

    let mut sb: *mut png_color_8 = core::ptr::null_mut();
    if (api.png_get_sBIT)(png, info, &mut sb) != 0 && !sb.is_null() {
        log(format!("{}: sBIT {:?}", tag, *sb));
    } else {
        log(format!("{}: sBIT none", tag));
    }

    let mut g: i32 = 0;
    log(format!(
        "{}: gAMA r={} v={}",
        tag,
        (api.png_get_gAMA_fixed)(png, info, &mut g),
        g
    ));
    let mut intent: c_int = -1;
    log(format!(
        "{}: sRGB r={} intent={}",
        tag,
        (api.png_get_sRGB)(png, info, &mut intent),
        intent
    ));
}

/// Drive a whole read with `trs` installed right after `png_read_info`, and
/// record everything.  `updates` is how many times `png_read_update_info` is
/// called (0, 1 or 2).
unsafe fn tr_read(
    api: &Api,
    data: &[u8],
    trs: &[Tr],
    mode: RowMode,
    updates: u32,
) -> Vec<Vec<u8>> {
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));

    let mut out: Vec<Vec<u8>> = Vec::new();
    let guard = guarded(api, png, &mut || {
        (api.png_read_info)(png, info);
        log_info(api, png, info, "after read_info");
        log_extra(api, png, info, "after read_info");
        for t in trs {
            apply(api, png, t);
        }
        for i in 0..updates {
            (api.png_read_update_info)(png, info);
            log_info(api, png, info, &format!("after update_info#{}", i));
            log_extra(api, png, info, &format!("after update_info#{}", i));
        }
        let h = (api.png_get_image_height)(png, info) as usize;
        let w = (api.png_get_image_width)(png, info) as usize;
        let rb = (api.png_get_rowbytes)(png, info);
        let passes = if (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7 {
            7
        } else {
            1
        };
        log(format!("rowbytes={} w={} h={} passes={}", rb, w, h, passes));
        // Always give the row buffers room for the widest possible transformed
        // row (16-bit RGBA) so that a wrong `rowbytes` cannot corrupt memory;
        // the whole buffer is compared, so a size difference is still visible.
        let cap = rb.max(w * 8 + 16).max(1);
        out = vec![vec![0u8; cap]; h];
        match mode {
            RowMode::None => {}
            RowMode::Row | RowMode::RowDisplay => {
                let mut disp = vec![vec![0u8; cap]; h];
                for _ in 0..passes {
                    for y in 0..h {
                        let d = if mode == RowMode::RowDisplay {
                            disp[y].as_mut_ptr()
                        } else {
                            core::ptr::null_mut()
                        };
                        (api.png_read_row)(png, out[y].as_mut_ptr(), d);
                    }
                }
                if mode == RowMode::RowDisplay {
                    for (y, d) in disp.iter().enumerate() {
                        log(format!("display {}: {:02x?}", y, d));
                    }
                }
            }
            RowMode::Rows(n) => {
                let n = n.max(1);
                for _ in 0..passes {
                    let mut y = 0;
                    while y < h {
                        let k = n.min(h - y);
                        let mut ptrs: Vec<*mut u8> =
                            out[y..y + k].iter().map(|r| r.as_ptr() as *mut u8).collect();
                        (api.png_read_rows)(png, ptrs.as_mut_ptr(), core::ptr::null_mut(), k as u32);
                        y += k;
                    }
                }
            }
            RowMode::Image => {
                let mut ptrs: Vec<*mut u8> = out.iter().map(|r| r.as_ptr() as *mut u8).collect();
                (api.png_read_image)(png, ptrs.as_mut_ptr());
            }
        }
        (api.png_read_end)(png, info);
        log_info(api, png, info, "after read_end");
        log_extra(api, png, info, "after read_end");
    });
    log(format!("guard={:?}", guard));
    log(format!(
        "rgb_to_gray_status={} palette_max={} row={} pass={}",
        (api.png_get_rgb_to_gray_status)(png),
        (api.png_get_palette_max)(png, info),
        (api.png_get_current_row_number)(png),
        (api.png_get_current_pass_number)(png)
    ));
    for (y, r) in out.iter().enumerate() {
        log(format!("row {}: {:02x?}", y, r));
    }
    destroy_read(api, png, info);
    out
}

/// Run the same read against both libraries and compare.
#[track_caller]
fn diff(tag: &str, file: &[u8], trs: &[Tr], mode: RowMode) {
    assert_same(tag, |api| unsafe {
        let mut o = Outcome::default();
        let rows = tr_read(api, file, trs, mode, 1);
        for r in &rows {
            o.output.extend_from_slice(r);
        }
        o
    });
}

/* ------------------------------------------------------------------ */
/* random parameter generators                                         */
/* ------------------------------------------------------------------ */

fn rand_loc(rng: &mut Rng) -> c_int {
    if rng.bool() {
        PNG_FILLER_BEFORE
    } else {
        PNG_FILLER_AFTER
    }
}

fn rand_filler(rng: &mut Rng) -> u32 {
    match rng.below(5) {
        0 => 0,
        1 => 0xff,
        2 => 0xffff,
        3 => rng.u32() & 0xffff,
        _ => (rng.u32() & 0xff) | 0x5a00,
    }
}

fn rand_shift(rng: &mut Rng, bd: c_int) -> png_color_8 {
    let m = bd as u32;
    png_color_8 {
        red: (1 + rng.u32() % m) as u8,
        green: (1 + rng.u32() % m) as u8,
        blue: (1 + rng.u32() % m) as u8,
        gray: (1 + rng.u32() % m) as u8,
        alpha: (1 + rng.u32() % m) as u8,
    }
}

/// Gamma values inside libpng's supported range (0.01 … 100).
const GAMMAS: [i32; 8] = [1000, 10000, 30000, 45455, 100000, 220000, 500000, 1000000];

fn rand_quantize(rng: &mut Rng, big: bool) -> Tr {
    let n = if big {
        1 + rng.below(48)
    } else {
        1 + rng.below(12)
    };
    let pal: Vec<png_color> = (0..n)
        .map(|_| png_color {
            red: rng.u8(),
            green: rng.u8(),
            blue: rng.u8(),
        })
        .collect();
    let max = (1 + rng.below(n + 4)) as c_int;
    let hist = if rng.bool() {
        Some((0..n).map(|_| (rng.u32() % 4096) as u16).collect())
    } else {
        None
    };
    let full = if rng.bool() { 1 } else { 0 };
    Tr::Quantize(pal, max, hist, full)
}

/// A random background colour that is *inside* libpng's domain.
///
/// For a palette image the background has to be an 8-bit value in the output
/// colour space: the palette branch of `png_init_read_transformations`
/// (pngrtran.c, `back.red = png_ptr->gamma_table[png_ptr->background.red]`)
/// indexes the 256-entry gamma table with it directly, so a 16-bit value is an
/// out-of-bounds read inside libpng itself.  Every other colour type only ever
/// feeds the background through `png_gamma_correct`, which is computational.
fn rand_back(rng: &mut Rng, ct: c_int, np: usize) -> png_color_16 {
    let m: u32 = if ct == PNG_COLOR_TYPE_PALETTE {
        256
    } else {
        65536
    };
    png_color_16 {
        index: rng.below(np.max(1)) as u8,
        red: (rng.u32() % m) as u16,
        green: (rng.u32() % m) as u16,
        blue: (rng.u32() % m) as u16,
        gray: (rng.u32() % m) as u16,
    }
}

fn rand_background(rng: &mut Rng, ct: c_int, np: usize) -> Tr {
    let back = rand_back(rng, ct, np);
    let code = rng.pick(&[
        PNG_BACKGROUND_GAMMA_UNKNOWN,
        PNG_BACKGROUND_GAMMA_SCREEN,
        PNG_BACKGROUND_GAMMA_FILE,
        PNG_BACKGROUND_GAMMA_UNIQUE,
    ]);
    let expand = if rng.bool() { 1 } else { 0 };
    Tr::Background(rng.bool(), back, code, expand, rng.pick(&GAMMAS))
}

/// A random transform of kind `kind` (0 … `N_KINDS`-1) legal to *install*
/// (no deliberately invalid arguments) for a file of shape `(ct, bd)` with a
/// palette of `np` entries.
fn rand_tr(rng: &mut Rng, kind: usize, ct: c_int, bd: c_int, np: usize) -> Tr {
    match kind {
        0 => Tr::PaletteToRgb,
        1 => Tr::ExpandGray,
        2 => Tr::TrnsToAlpha,
        3 => Tr::Expand,
        4 => Tr::Expand16,
        5 => Tr::Strip16,
        6 => Tr::Scale16,
        7 => Tr::StripAlpha,
        8 => Tr::SwapAlpha,
        9 => Tr::InvertAlpha,
        10 => Tr::Filler(rand_filler(rng), rand_loc(rng)),
        11 => Tr::AddAlpha(rand_filler(rng), rand_loc(rng)),
        12 => Tr::Bgr,
        13 => Tr::Swap,
        14 => Tr::Packing,
        15 => Tr::Packswap,
        16 => Tr::Shift(rand_shift(rng, bd)),
        17 => Tr::InvertMono,
        18 => Tr::GrayToRgb,
        19 => {
            let action = rng.pick(&[PNG_ERROR_ACTION_NONE, PNG_ERROR_ACTION_WARN]);
            let (r, g) = match rng.below(4) {
                0 => (-1, -1),
                1 => (30000, 60000),
                2 => (0, 0),
                _ => {
                    let r = rng.below(100001) as i32;
                    (r, rng.below((100001 - r) as usize) as i32)
                }
            };
            Tr::RgbToGray(rng.bool(), action, r, g)
        }
        20 => rand_quantize(rng, false),
        21 => rand_background(rng, ct, np),
        22 => Tr::AlphaMode(
            rng.bool(),
            rng.pick(&[
                PNG_ALPHA_PNG,
                PNG_ALPHA_STANDARD,
                PNG_ALPHA_OPTIMIZED,
                PNG_ALPHA_BROKEN,
            ]),
            rng.pick(&GAMMAS),
        ),
        23 => Tr::Gamma(rng.bool(), rng.pick(&GAMMAS), rng.pick(&GAMMAS)),
        24 => Tr::Interlace,
        // Only *shrinking* overrides are safe in an arbitrary combination: a
        // user pixel depth larger than the row really has makes libpng copy
        // uninitialised `big_row_buf` bytes out of png_combine_row (the buffer
        // is png_malloc'ed, not calloc'ed, for non-interlaced images), which is
        // not something two implementations can agree on.  C-92 (`single_6`)
        // covers the full range of overrides on its own.
        25 => Tr::User(rng.pick(&[0, 1]), rng.pick(&[0, 1])),
        _ => Tr::CheckIndex(if rng.bool() { 1 } else { 0 }),
    }
}

/// A small random image of the given shape.  The widths are chosen to hit every
/// sub-byte packing remainder as well as a few multi-word rows.
const WIDTHS: [u32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 15, 16, 17, 23, 32, 33];

fn rand_img(rng: &mut Rng, ct: c_int, bd: c_int) -> Img {
    let w = rng.pick(&WIDTHS);
    let h = 1 + rng.below(5) as u32;
    Img::random(rng, w, h, ct, bd)
}

/* ================================================================== */
/* C-67 … C-71: the expand family                                      */
/* ================================================================== */

#[test]
fn single() {
    let variants: Vec<(&str, Vec<Tr>)> = vec![
        ("palette_to_rgb", vec![Tr::PaletteToRgb]),
        ("expand_gray_1_2_4_to_8", vec![Tr::ExpandGray]),
        ("tRNS_to_alpha", vec![Tr::TrnsToAlpha]),
        ("expand", vec![Tr::Expand]),
        ("expand_16", vec![Tr::Expand16]),
        ("expand+expand_16", vec![Tr::Expand, Tr::Expand16]),
        ("palette_to_rgb+expand_gray", vec![Tr::PaletteToRgb, Tr::ExpandGray]),
    ];
    let auxes = [AUX_NONE, AUX_TRNS, AUX_TRNS_BKGD, AUX_ALL];

    for (vi, (name, trs)) in variants.iter().enumerate() {
        for (ct, bd) in VALID_SHAPES {
            for rep in 0..4u64 {
                let seed = 0xc067
                    ^ ((vi as u64) << 48)
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ (rep << 16);
                let mut rng = Rng::new(seed);
                let mut img = rand_img(&mut rng, ct, bd);
                if rep == 3 {
                    img.interlace = PNG_INTERLACE_ADAM7;
                }
                let mut trs = trs.clone();
                if img.interlace == PNG_INTERLACE_ADAM7 {
                    trs.insert(0, Tr::Interlace);
                }
                for (ai, aux) in auxes.iter().enumerate() {
                    let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                    let mode = match rep {
                        0 => RowMode::Row,
                        1 => RowMode::Image,
                        2 => RowMode::Rows(3),
                        _ => RowMode::RowDisplay,
                    };
                    diff(
                        &format!(
                            "{} ct={} bd={} {}x{} il={} rep={} aux{}",
                            name, ct, bd, img.w, img.h, img.interlace, rep, ai
                        ),
                        &file,
                        &trs,
                        mode,
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/* C-72 … C-78: 16→8, alpha shuffling and the filler                   */
/* ================================================================== */

#[test]
fn single_2() {
    let mut rng = Rng::new(0xc072);
    let mut variants: Vec<(String, Vec<Tr>)> = vec![
        ("strip_16".to_string(), vec![Tr::Strip16]),
        ("scale_16".to_string(), vec![Tr::Scale16]),
        ("strip_16+scale_16".to_string(), vec![Tr::Strip16, Tr::Scale16]),
        ("strip_alpha".to_string(), vec![Tr::StripAlpha]),
        ("swap_alpha".to_string(), vec![Tr::SwapAlpha]),
        ("invert_alpha".to_string(), vec![Tr::InvertAlpha]),
        (
            "swap_alpha+invert_alpha".to_string(),
            vec![Tr::SwapAlpha, Tr::InvertAlpha],
        ),
        (
            "expand+strip_alpha".to_string(),
            vec![Tr::Expand, Tr::StripAlpha],
        ),
    ];
    // C-77 / C-78: filler value x BEFORE/AFTER, on its own and after an expand
    // (which is what makes it applicable to the low bit depth / palette shapes).
    for v in [0u32, 0xff, 0xffff, 0x1234] {
        for loc in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
            variants.push((format!("filler({:#x},{})", v, loc), vec![Tr::Filler(v, loc)]));
            variants.push((
                format!("add_alpha({:#x},{})", v, loc),
                vec![Tr::AddAlpha(v, loc)],
            ));
            variants.push((
                format!("expand+filler({:#x},{})", v, loc),
                vec![Tr::Expand, Tr::Filler(v, loc)],
            ));
            variants.push((
                format!("gray_to_rgb+add_alpha({:#x},{})", v, loc),
                vec![Tr::GrayToRgb, Tr::AddAlpha(v, loc)],
            ));
        }
    }
    // and a handful of purely random filler values
    for _ in 0..6 {
        let v = rand_filler(&mut rng);
        let l = rand_loc(&mut rng);
        variants.push((format!("rand filler({:#x},{})", v, l), vec![Tr::Filler(v, l)]));
    }

    let auxes = [AUX_NONE, AUX_TRNS];
    for (vi, (name, trs)) in variants.iter().enumerate() {
        for (ct, bd) in VALID_SHAPES {
            for rep in 0..3u64 {
                let seed = 0xc072
                    ^ ((vi as u64) << 48)
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ (rep << 8);
                let mut r = Rng::new(seed);
                let mut img = rand_img(&mut r, ct, bd);
                let mut trs = trs.clone();
                if rep == 2 {
                    img.interlace = PNG_INTERLACE_ADAM7;
                    trs.insert(0, Tr::Interlace);
                }
                for (ai, aux) in auxes.iter().enumerate() {
                    let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                    diff(
                        &format!(
                            "{} ct={} bd={} {}x{} il={} rep={} aux{}",
                            name, ct, bd, img.w, img.h, img.interlace, rep, ai
                        ),
                        &file,
                        &trs,
                        if rep == 1 { RowMode::Image } else { RowMode::Row },
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/* C-79 … C-85: byte/bit level transforms                              */
/* ================================================================== */

#[test]
fn single_3() {
    let mut variants: Vec<(String, Vec<Tr>)> = vec![
        ("bgr".to_string(), vec![Tr::Bgr]),
        ("swap".to_string(), vec![Tr::Swap]),
        ("packing".to_string(), vec![Tr::Packing]),
        ("packswap".to_string(), vec![Tr::Packswap]),
        ("packing+packswap".to_string(), vec![Tr::Packing, Tr::Packswap]),
        ("invert_mono".to_string(), vec![Tr::InvertMono]),
        ("gray_to_rgb".to_string(), vec![Tr::GrayToRgb]),
        ("gray_to_rgb+bgr".to_string(), vec![Tr::GrayToRgb, Tr::Bgr]),
        (
            "expand+packswap".to_string(),
            vec![Tr::Expand, Tr::Packswap],
        ),
        (
            "invert_mono+packing".to_string(),
            vec![Tr::InvertMono, Tr::Packing],
        ),
    ];

    for (vi, (name, trs)) in variants.clone().iter().enumerate() {
        for (ct, bd) in VALID_SHAPES {
            for rep in 0..3u64 {
                let seed = 0xc079
                    ^ ((vi as u64) << 48)
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ (rep << 8);
                let mut r = Rng::new(seed);
                let mut img = rand_img(&mut r, ct, bd);
                let mut trs = trs.clone();
                if rep == 2 {
                    img.interlace = PNG_INTERLACE_ADAM7;
                    trs.insert(0, Tr::Interlace);
                }
                for (ai, aux) in [AUX_NONE, AUX_TRNS].iter().enumerate() {
                    let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                    diff(
                        &format!(
                            "{} ct={} bd={} {}x{} il={} rep={} aux{}",
                            name, ct, bd, img.w, img.h, img.interlace, rep, ai
                        ),
                        &file,
                        &trs,
                        RowMode::Row,
                    );
                }
            }
        }
    }
    variants.clear();

    // C-83: png_set_shift with random valid sBIT values <= bit depth, with the
    // sBIT chunk present and absent, plus deliberately invalid shift values
    // (which libpng rejects with a fatal application error).
    for (ct, bd) in VALID_SHAPES {
        for rep in 0..4u64 {
            let seed = 0xc083 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ (rep << 8);
            let mut r = Rng::new(seed);
            let img = rand_img(&mut r, ct, bd);
            let shift = rand_shift(&mut r, bd);
            for (ai, aux) in [
                AUX_NONE,
                Aux {
                    sbit: true,
                    ..AUX_NONE
                },
            ]
            .iter()
            .enumerate()
            {
                let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                diff(
                    &format!("shift {:?} ct={} bd={} aux{}", shift, ct, bd, ai),
                    &file,
                    &[Tr::Shift(shift)],
                    RowMode::Row,
                );
                // shift combined with expand (the palette short-cut in
                // png_init_read_transformations is skipped when EXPAND is set)
                diff(
                    &format!("expand+shift {:?} ct={} bd={} aux{}", shift, ct, bd, ai),
                    &file,
                    &[Tr::Expand, Tr::Shift(shift)],
                    RowMode::Row,
                );
            }
            // invalid: zero and one past the bit depth
            let file = unsafe { make_file(&img, &AUX_NONE, seed) };
            for bad in [
                png_color_8 {
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                    alpha: 0,
                },
                png_color_8 {
                    red: (bd + 1) as u8,
                    green: (bd + 1) as u8,
                    blue: (bd + 1) as u8,
                    gray: (bd + 1) as u8,
                    alpha: (bd + 1) as u8,
                },
            ] {
                diff(
                    &format!("bad shift {:?} ct={} bd={}", bad, ct, bd),
                    &file,
                    &[Tr::Shift(bad)],
                    RowMode::Row,
                );
            }
        }
    }
}

/* ================================================================== */
/* C-86, C-87: rgb_to_gray and quantize                                */
/* ================================================================== */

#[test]
fn single_4() {
    // C-86: error action x default/explicit coefficients x cHRM present/absent
    let coeffs: [(i32, i32); 6] = [
        (-1, -1),
        (0, 0),
        (30000, 60000),
        (50000, 50000),
        (100000, 0),
        (70000, 70000), // out of range: red+green > 1.0 -> app warning
    ];
    for (ct, bd) in VALID_SHAPES {
        for action in [
            PNG_ERROR_ACTION_NONE,
            PNG_ERROR_ACTION_WARN,
            PNG_ERROR_ACTION_ERROR,
        ] {
            for (ci, &(r, g)) in coeffs.iter().enumerate() {
                for fixed in [false, true] {
                    let seed = 0xc086
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((action as u64) << 24)
                        ^ ((ci as u64) << 16)
                        ^ (fixed as u64);
                    let mut rng = Rng::new(seed);
                    let img = rand_img(&mut rng, ct, bd);
                    for (ai, aux) in [
                        AUX_NONE,
                        Aux {
                            chrm: true,
                            ..AUX_NONE
                        },
                        Aux {
                            trns: true,
                            chrm: true,
                            gama: Some(45455),
                            ..AUX_NONE
                        },
                    ]
                    .iter()
                    .enumerate()
                    {
                        let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                        diff(
                            &format!(
                                "rgb_to_gray fixed={} action={} coeff=({},{}) ct={} bd={} aux{}",
                                fixed, action, r, g, ct, bd, ai
                            ),
                            &file,
                            &[Tr::RgbToGray(fixed, action, r, g)],
                            RowMode::Row,
                        );
                    }
                }
            }
        }
    }
    // invalid error action -> png_error
    {
        let mut rng = Rng::new(0xc0861);
        let img = rand_img(&mut rng, PNG_COLOR_TYPE_RGB, 8);
        let file = unsafe { make_file(&img, &AUX_NONE, 0xc0861) };
        for bad in [0, 4, -1] {
            diff(
                &format!("rgb_to_gray bad action {}", bad),
                &file,
                &[Tr::RgbToGray(true, bad, -1, -1)],
                RowMode::Row,
            );
        }
    }

    // C-87: png_set_quantize
    for (ct, bd) in VALID_SHAPES {
        for rep in 0..6u64 {
            let seed = 0xc087 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ (rep << 8);
            let mut rng = Rng::new(seed);
            let img = rand_img(&mut rng, ct, bd);
            let q = rand_quantize(&mut rng, rep == 0);
            let file = unsafe { make_file(&img, &AUX_NONE, seed) };
            diff(
                &format!("quantize ct={} bd={} rep={}", ct, bd, rep),
                &file,
                core::slice::from_ref(&q),
                RowMode::Row,
            );
            // and after an expand / strip_16, which is what makes the RGB
            // quantize path reachable from the 16-bit and palette shapes
            diff(
                &format!("expand+strip16+quantize ct={} bd={} rep={}", ct, bd, rep),
                &file,
                &[Tr::Expand, Tr::Strip16, q.clone()],
                RowMode::Row,
            );
        }
    }
    // explicit num_palette / maximum_colors corners
    {
        let mut rng = Rng::new(0xc0872);
        let img = rand_img(&mut rng, PNG_COLOR_TYPE_RGB, 8);
        let file = unsafe { make_file(&img, &AUX_NONE, 0xc0872) };
        for &(n, max) in &[(1usize, 1i32), (1, 256), (8, 1), (8, 4), (32, 5), (256, 16)] {
            for full in [0, 1] {
                for with_hist in [false, true] {
                    let mut r = Rng::new(0xc0873 ^ (n as u64) ^ ((max as u64) << 16));
                    let pal: Vec<png_color> = (0..n)
                        .map(|_| png_color {
                            red: r.u8(),
                            green: r.u8(),
                            blue: r.u8(),
                        })
                        .collect();
                    let hist = if with_hist {
                        Some((0..n).map(|_| (r.u32() % 900) as u16).collect())
                    } else {
                        None
                    };
                    diff(
                        &format!(
                            "quantize n={} max={} full={} hist={}",
                            n, max, full, with_hist
                        ),
                        &file,
                        &[Tr::Quantize(pal, max, hist, full)],
                        RowMode::Row,
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/* C-88 … C-90: background, alpha mode, gamma                          */
/* ================================================================== */

#[test]
fn single_5() {
    // C-88: png_set_background / png_set_background_fixed
    for (ct, bd) in VALID_SHAPES {
        for code in [
            PNG_BACKGROUND_GAMMA_UNKNOWN,
            PNG_BACKGROUND_GAMMA_SCREEN,
            PNG_BACKGROUND_GAMMA_FILE,
            PNG_BACKGROUND_GAMMA_UNIQUE,
        ] {
            for expand in [0, 1] {
                for fixed in [false, true] {
                    let seed = 0xc088
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((code as u64) << 24)
                        ^ ((expand as u64) << 16)
                        ^ (fixed as u64);
                    let mut rng = Rng::new(seed);
                    let img = rand_img(&mut rng, ct, bd);
                    let back = rand_back(&mut rng, ct, img.palette.len());
                    let bg = rng.pick(&GAMMAS);
                    for (ai, aux) in [AUX_NONE, AUX_TRNS, AUX_ALL].iter().enumerate() {
                        let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                        diff(
                            &format!(
                                "background fixed={} code={} expand={} ct={} bd={} aux{}",
                                fixed, code, expand, ct, bd, ai
                            ),
                            &file,
                            &[Tr::Background(fixed, back, code, expand, bg)],
                            RowMode::Row,
                        );
                        // combined with an expand and a gamma, the two things
                        // that change how the background is interpreted
                        diff(
                            &format!(
                                "background+expand+gamma fixed={} code={} expand={} ct={} bd={} aux{}",
                                fixed, code, expand, ct, bd, ai
                            ),
                            &file,
                            &[
                                Tr::Expand,
                                Tr::Background(fixed, back, code, expand, bg),
                                Tr::Gamma(fixed, 220000, 45455),
                            ],
                            RowMode::Row,
                        );
                    }
                }
            }
        }
    }

    // C-89: png_set_alpha_mode / _fixed
    for (ct, bd) in VALID_SHAPES {
        for mode in [
            PNG_ALPHA_PNG,
            PNG_ALPHA_STANDARD,
            PNG_ALPHA_OPTIMIZED,
            PNG_ALPHA_BROKEN,
            9, // invalid -> png_error("invalid alpha mode")
        ] {
            for g in [100000, 220000, 45455, PNG_FP_1, 500000] {
                for fixed in [false, true] {
                    let seed = 0xc089
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((mode as u64) << 24)
                        ^ ((g as u64) << 8)
                        ^ (fixed as u64);
                    let mut rng = Rng::new(seed);
                    let img = rand_img(&mut rng, ct, bd);
                    for (ai, aux) in [AUX_NONE, AUX_TRNS].iter().enumerate() {
                        let file = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                        diff(
                            &format!(
                                "alpha_mode fixed={} mode={} g={} ct={} bd={} aux{}",
                                fixed, mode, g, ct, bd, ai
                            ),
                            &file,
                            &[Tr::AlphaMode(fixed, mode, g)],
                            RowMode::Row,
                        );
                    }
                }
            }
        }
    }

    // C-90: png_set_gamma / _fixed, gAMA / sRGB present and absent
    let pairs: [(i32, i32); 9] = [
        (100000, 100000),
        (220000, 45455),
        (45455, 220000),
        (100000, 45455),
        (220000, 220000),
        (1000000, 1000),
        (1000, 1000000),
        (30000, 300000),
        (100000, 1),      // below the supported range -> app warning
    ];
    for (ct, bd) in VALID_SHAPES {
        for (pi, &(scrn, file_g)) in pairs.iter().enumerate() {
            for fixed in [false, true] {
                let seed = 0xc090
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ ((pi as u64) << 16)
                    ^ (fixed as u64);
                let mut rng = Rng::new(seed);
                let img = rand_img(&mut rng, ct, bd);
                for (ai, aux) in [
                    AUX_NONE,
                    Aux {
                        gama: Some(45455),
                        ..AUX_NONE
                    },
                    Aux {
                        gama: Some(100000),
                        ..AUX_NONE
                    },
                    Aux {
                        srgb: Some(PNG_sRGB_INTENT_PERCEPTUAL),
                        ..AUX_NONE
                    },
                ]
                .iter()
                .enumerate()
                {
                    let f = unsafe { make_file(&img, aux, seed ^ (ai as u64)) };
                    diff(
                        &format!(
                            "gamma fixed={} scrn={} file={} ct={} bd={} aux{}",
                            fixed, scrn, file_g, ct, bd, ai
                        ),
                        &f,
                        &[Tr::Gamma(fixed, scrn, file_g)],
                        RowMode::Row,
                    );
                }
            }
        }
    }
    // zero / negative gamma and the "well known" flag values
    {
        let mut rng = Rng::new(0xc0901);
        let img = rand_img(&mut rng, PNG_COLOR_TYPE_RGB, 8);
        let f = unsafe { make_file(&img, &AUX_NONE, 0xc0901) };
        for &(s, g) in &[
            (0, 100000),
            (100000, 0),
            (-1, -1),
            (-2, -2),
            (-1, 100000),
            (100000, -2),
        ] {
            diff(
                &format!("gamma_fixed flags scrn={} file={}", s, g),
                &f,
                &[Tr::Gamma(true, s, g)],
                RowMode::Row,
            );
        }
    }
}

/* ================================================================== */
/* C-91 … C-93: interlace handling, user transform, invalid index       */
/* ================================================================== */

#[test]
fn single_6() {
    // C-91: png_set_interlace_handling and its return value
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for rep in 0..2u64 {
                let seed = 0xc091
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ ((il as u64) << 24)
                    ^ (rep << 8);
                let mut rng = Rng::new(seed);
                let mut img = rand_img(&mut rng, ct, bd);
                img.interlace = il;
                let file = unsafe { make_file(&img, &AUX_NONE, seed) };
                for (name, trs) in [
                    ("interlace", vec![Tr::Interlace]),
                    ("interlace+expand", vec![Tr::Interlace, Tr::Expand]),
                    (
                        "interlace+packing+bgr",
                        vec![Tr::Interlace, Tr::Packing, Tr::Bgr],
                    ),
                ] {
                    for mode in [RowMode::Row, RowMode::RowDisplay] {
                        diff(
                            &format!(
                                "{} ct={} bd={} il={} {}x{} {:?}",
                                name, ct, bd, il, img.w, img.h, mode
                            ),
                            &file,
                            &trs,
                            mode,
                        );
                    }
                }
            }
        }
    }

    // C-92: png_set_read_user_transform_fn + png_set_user_transform_info.
    //
    // The user depth/channels are only varied *downwards*: a user pixel depth
    // larger than the row really has makes png_combine_row copy uninitialised
    // bytes out of libpng's `big_row_buf` (png_malloc'ed, not calloc'ed, for
    // non-interlaced images — see png_read_start_row in pngrutil.c), which is
    // genuinely unspecified rather than a translation difference.
    for (ct, bd) in VALID_SHAPES {
        let depths: Vec<c_int> = [0, 1, 2, 4, 8, 16]
            .into_iter()
            .filter(|&d| d == 0 || d <= bd)
            .collect();
        let chans: Vec<c_int> = (0..=channels_of(ct) as c_int).collect();
        for depth in depths {
            for ch in chans.iter().copied() {
                let seed = 0xc092
                    ^ ((ct as u64) << 40)
                    ^ ((bd as u64) << 32)
                    ^ ((depth as u64) << 16)
                    ^ (ch as u64);
                let mut rng = Rng::new(seed);
                let img = rand_img(&mut rng, ct, bd);
                let file = unsafe { make_file(&img, &AUX_NONE, seed) };
                diff(
                    &format!("user d={} ch={} ct={} bd={}", depth, ch, ct, bd),
                    &file,
                    &[Tr::User(depth, ch)],
                    RowMode::Row,
                );
                diff(
                    &format!("expand+user d={} ch={} ct={} bd={}", depth, ch, ct, bd),
                    &file,
                    &[Tr::Expand, Tr::User(depth, ch)],
                    RowMode::Row,
                );
            }
        }
    }

    // C-93: png_set_check_for_invalid_index, on and off, with in-range and
    // out-of-range palette indices.
    for bd in [1, 2, 4, 8] {
        for short in [false, true] {
            for allowed in [-1, 0, 1] {
                for rep in 0..3u64 {
                    let seed = 0xc093
                        ^ ((bd as u64) << 32)
                        ^ ((short as u64) << 24)
                        ^ (((allowed + 1) as u64) << 16)
                        ^ (rep << 8);
                    let mut rng = Rng::new(seed);
                    let mut img = rand_img(&mut rng, PNG_COLOR_TYPE_PALETTE, bd);
                    if short {
                        // Fewer palette entries than the bit depth allows, so
                        // the random indices run off the end of the palette.
                        let n = 1 + rng.below((1usize << bd.min(8)).max(2) - 1);
                        img.palette.truncate(n.max(1));
                    }
                    let file = unsafe { make_file(&img, &AUX_NONE, seed) };
                    diff(
                        &format!(
                            "check_index allowed={} bd={} short={} np={} rep={}",
                            allowed,
                            bd,
                            short,
                            img.palette.len(),
                            rep
                        ),
                        &file,
                        &[Tr::CheckIndex(allowed)],
                        RowMode::Row,
                    );
                    diff(
                        &format!(
                            "check_index+expand allowed={} bd={} short={} np={} rep={}",
                            allowed,
                            bd,
                            short,
                            img.palette.len(),
                            rep
                        ),
                        &file,
                        &[Tr::CheckIndex(allowed), Tr::Expand],
                        RowMode::Row,
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/* C-94: randomised combinations of 2..6 read transforms               */
/* ================================================================== */

#[test]
fn combinations() {
    let mut rng = Rng::new(0xc094);
    for (ct, bd) in VALID_SHAPES {
        for round in 0..80u64 {
            let seed = 0xc094 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ (round << 8);
            let mut r = Rng::new(seed);
            let mut img = rand_img(&mut r, ct, bd);
            if round % 6 == 5 {
                img.interlace = PNG_INTERLACE_ADAM7;
            }
            let aux = match round % 4 {
                0 => AUX_NONE,
                1 => AUX_TRNS,
                2 => AUX_TRNS_BKGD,
                _ => AUX_ALL,
            };
            let file = unsafe { make_file(&img, &aux, seed) };

            let n = 2 + r.below(5); // 2..6 transforms
            let mut kinds: Vec<usize> = Vec::new();
            while kinds.len() < n {
                let k = r.below(N_KINDS);
                if !kinds.contains(&k) {
                    kinds.push(k);
                }
            }
            kinds.sort_unstable(); // a canonical install order
            let np = img.palette.len();
            let trs: Vec<Tr> = kinds
                .iter()
                .map(|&k| rand_tr(&mut r, k, ct, bd, np))
                .collect();
            let names: Vec<String> = trs
                .iter()
                .map(|t| format!("{:?}", t).split('(').next().unwrap().to_string())
                .collect();
            let mode = match round % 3 {
                0 => RowMode::Row,
                1 => RowMode::Image,
                _ => RowMode::Rows(2),
            };
            diff(
                &format!(
                    "combo ct={} bd={} il={} round={} [{}]",
                    ct,
                    bd,
                    img.interlace,
                    round,
                    names.join(",")
                ),
                &file,
                &trs,
                mode,
            );
            let _ = rng.u32();
        }
    }
}

/* ================================================================== */
/* C-7: png_build_gamma_table / png_destroy_gamma_table                */
/* ================================================================== */

#[test]
fn gamma_tables() {
    let shapes: [(c_int, c_int); 9] = [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ];
    // (screen, file) pairs including equal, unity and extreme values
    let pairs: [(i32, i32); 8] = [
        (100000, 100000),
        (100000, 45455),
        (220000, 45455),
        (45455, 220000),
        (220000, 220000),
        (1000000, 1000),
        (1000, 1000000),
        (250000, 40000),
    ];
    let auxes = [
        AUX_NONE,
        Aux {
            gama: Some(45455),
            ..AUX_NONE
        },
        Aux {
            gama: Some(100000),
            ..AUX_NONE
        },
        Aux {
            srgb: Some(PNG_sRGB_INTENT_RELATIVE),
            ..AUX_NONE
        },
        // png_build_gamma_table derives the 16-bit gamma_shift from sig_bit, so
        // an sBIT chunk selects a different set of tables entirely.
        Aux {
            sbit: true,
            ..AUX_NONE
        },
        Aux {
            sbit: true,
            gama: Some(45455),
            ..AUX_NONE
        },
        Aux {
            sbit: true,
            trns: true,
            gama: Some(220000),
            ..AUX_NONE
        },
    ];

    for &(ct, bd) in &shapes {
        for (ai, aux) in auxes.iter().enumerate() {
            let seed = 0xc007 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ ((ai as u64) << 16);
            let mut rng = Rng::new(seed);
            let img = rand_img(&mut rng, ct, bd);
            let file = unsafe { make_file(&img, aux, seed) };

            for &(scrn, fg) in &pairs {
                for fixed in [false, true] {
                    diff(
                        &format!(
                            "gamma_table ct={} bd={} aux{} scrn={} file={} fixed={}",
                            ct, bd, ai, scrn, fg, fixed
                        ),
                        &file,
                        &[Tr::Gamma(fixed, scrn, fg)],
                        RowMode::Row,
                    );
                }
            }
            // the same tables reached through png_set_alpha_mode
            for mode in [
                PNG_ALPHA_PNG,
                PNG_ALPHA_STANDARD,
                PNG_ALPHA_OPTIMIZED,
                PNG_ALPHA_BROKEN,
            ] {
                for g in [100000, 220000, 45455] {
                    for fixed in [false, true] {
                        diff(
                            &format!(
                                "gamma_table alpha_mode ct={} bd={} aux{} mode={} g={} fixed={}",
                                ct, bd, ai, mode, g, fixed
                            ),
                            &file,
                            &[Tr::AlphaMode(fixed, mode, g)],
                            RowMode::Row,
                        );
                    }
                }
            }
            // gamma + 16-to-8 and gamma + expand_16, which select the other
            // table sizes inside png_build_gamma_table
            for extra in [
                vec![Tr::Gamma(true, 220000, 45455), Tr::Strip16],
                vec![Tr::Gamma(true, 220000, 45455), Tr::Scale16],
                vec![Tr::Gamma(true, 220000, 45455), Tr::Expand16],
                vec![Tr::Gamma(true, 45455, 220000), Tr::Expand, Tr::Expand16],
            ] {
                let names: Vec<String> = extra
                    .iter()
                    .map(|t| format!("{:?}", t).split('(').next().unwrap().to_string())
                    .collect();
                diff(
                    &format!(
                        "gamma_table ct={} bd={} aux{} [{}]",
                        ct,
                        bd,
                        ai,
                        names.join(",")
                    ),
                    &file,
                    &extra,
                    RowMode::Row,
                );
            }
        }
    }
}

/* ================================================================== */
/* C-95: png_read_update_info once, twice and not at all               */
/* ================================================================== */

#[test]
fn update_info() {
    for (ct, bd) in VALID_SHAPES {
        let seed = 0xc095 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32);
        let mut rng = Rng::new(seed);
        let img = rand_img(&mut rng, ct, bd);
        for (ai, aux) in [AUX_NONE, AUX_TRNS].iter().enumerate() {
            let file = unsafe { make_file(&img, aux, seed ^ ai as u64) };
            for (tname, trs) in [
                ("none", vec![]),
                ("expand", vec![Tr::Expand]),
                ("packing+bgr", vec![Tr::Packing, Tr::Bgr]),
                ("gray_to_rgb+add_alpha", vec![Tr::GrayToRgb, Tr::AddAlpha(0xff, PNG_FILLER_AFTER)]),
                ("strip_16+invert_mono", vec![Tr::Strip16, Tr::InvertMono]),
            ] {
                for updates in [0u32, 1, 2] {
                    let tag = format!(
                        "update_info x{} {} ct={} bd={} aux{}",
                        updates, tname, ct, bd, ai
                    );
                    assert_same(&tag, |api| unsafe {
                        let mut o = Outcome::default();
                        let rows = tr_read(api, &file, &trs, RowMode::Row, updates);
                        for r in &rows {
                            o.output.extend_from_slice(r);
                        }
                        o
                    });
                }
            }
        }
    }

    // png_start_read_image interacting with png_read_update_info
    for (ct, bd) in VALID_SHAPES {
        let seed = 0xc0951 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32);
        let mut rng = Rng::new(seed);
        let img = rand_img(&mut rng, ct, bd);
        let file = unsafe { make_file(&img, &AUX_TRNS, seed) };
        for order in 0..4 {
            assert_same(
                &format!("start_read_image order={} ct={} bd={}", order, ct, bd),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = file.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                    let mut rows: Vec<Vec<u8>> = Vec::new();
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        (api.png_set_expand)(png);
                        match order {
                            0 => {
                                (api.png_start_read_image)(png);
                            }
                            1 => {
                                (api.png_start_read_image)(png);
                                (api.png_read_update_info)(png, info);
                            }
                            2 => {
                                (api.png_read_update_info)(png, info);
                                (api.png_start_read_image)(png);
                            }
                            _ => {
                                (api.png_start_read_image)(png);
                                (api.png_start_read_image)(png);
                            }
                        }
                        log_info(api, png, info, "after init");
                        let h = (api.png_get_image_height)(png, info) as usize;
                        let w = (api.png_get_image_width)(png, info) as usize;
                        let rb = (api.png_get_rowbytes)(png, info).max(w * 8 + 16);
                        rows = vec![vec![0u8; rb]; h];
                        for y in 0..h {
                            (api.png_read_row)(png, rows[y].as_mut_ptr(), core::ptr::null_mut());
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    for r in &rows {
                        o.output.extend_from_slice(r);
                    }
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }
}

/* ================================================================== */
/* C-96 … C-105: the write side                                        */
/* ================================================================== */

#[derive(Clone, Debug)]
enum W {
    Bgr,
    Swap,
    Packing,
    Packswap,
    Shift(png_color_8),
    InvertMono,
    InvertAlpha,
    SwapAlpha,
    Filler(u32, c_int),
    User(c_int, c_int),
}

unsafe fn w_apply(api: &Api, png: *mut PngStruct, w: &W) {
    match w {
        W::Bgr => (api.png_set_bgr)(png),
        W::Swap => (api.png_set_swap)(png),
        W::Packing => (api.png_set_packing)(png),
        W::Packswap => (api.png_set_packswap)(png),
        W::Shift(s) => (api.png_set_shift)(png, s),
        W::InvertMono => (api.png_set_invert_mono)(png),
        W::InvertAlpha => (api.png_set_invert_alpha)(png),
        W::SwapAlpha => (api.png_set_swap_alpha)(png),
        W::Filler(v, loc) => (api.png_set_filler)(png, *v, *loc),
        W::User(depth, ch) => {
            (api.png_set_write_user_transform_fn)(png, Some(user_transform_cb));
            (api.png_set_user_transform_info)(png, cookie(), *depth, *ch);
            log(format!(
                "w user_transform_ptr_ok={}",
                (api.png_get_user_transform_ptr)(png) == cookie()
            ));
        }
    }
}

/// The number of bytes per row the application must supply for `ws`.
fn usr_rowbytes(img: &Img, ws: &[W]) -> usize {
    let mut bd = img.bit_depth as usize;
    let mut ch = channels_of(img.color_type);
    for w in ws {
        match w {
            W::Packing if bd < 8 => bd = 8,
            W::Filler(_, _) => match img.color_type {
                PNG_COLOR_TYPE_RGB => ch = 4,
                PNG_COLOR_TYPE_GRAY if img.bit_depth >= 8 => ch = 2,
                _ => {}
            },
            _ => {}
        }
    }
    png_rowbytes(bd * ch, img.w as usize)
}

/// Drive a whole write with the write-side transforms `ws` installed *after*
/// `png_write_info` (`png_write_IHDR` resets `usr_bit_depth`/`usr_channels`,
/// so they have to go there — this is what `png_write_png` does too).
unsafe fn w_write(api: &Api, img: &Img, ws: &[W], rows: &[Vec<u8>], sbit: Option<png_color_8>) {
    let (png, info) = new_write(api);
    (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    let g = guarded(api, png, &mut || {
        (api.png_set_IHDR)(
            png,
            info,
            img.w,
            img.h,
            img.bit_depth,
            img.color_type,
            img.interlace,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if img.color_type == PNG_COLOR_TYPE_PALETTE && !img.palette.is_empty() {
            (api.png_set_PLTE)(png, info, img.palette.as_ptr(), img.palette.len() as c_int);
        }
        if let Some(s) = sbit {
            (api.png_set_sBIT)(png, info, &s);
        }
        (api.png_write_info)(png, info);
        for w in ws {
            w_apply(api, png, w);
        }
        let passes = if img.interlace == PNG_INTERLACE_ADAM7 {
            (api.png_set_interlace_handling)(png)
        } else {
            1
        };
        log(format!("w passes={}", passes));
        for _ in 0..passes {
            for r in rows {
                (api.png_write_row)(png, r.as_ptr() as *mut u8);
            }
        }
        (api.png_write_end)(png, info);
    });
    log(format!("w guard={:?}", g));
    destroy_write(api, png, info);
}

#[track_caller]
fn diff_write(tag: &str, img: &Img, ws: &[W], rows: &[Vec<u8>], sbit: Option<png_color_8>) {
    assert_same(tag, |api| unsafe {
        let mut o = Outcome::default();
        w_write(api, img, ws, rows, sbit);
        o.output = std::mem::take(&mut tls().output);
        o
    });
}

#[test]
fn write_side() {
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let seed = 0xc096 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ ((il as u64) << 24);
            let mut rng = Rng::new(seed);
            let mut img = rand_img(&mut rng, ct, bd);
            img.interlace = il;
            let shift = rand_shift(&mut rng, bd);
            let sb = png_color_8 {
                red: shift.red,
                green: shift.green,
                blue: shift.blue,
                gray: shift.gray,
                alpha: shift.alpha,
            };

            let mut sets: Vec<(String, Vec<W>)> = vec![
                ("bgr".to_string(), vec![W::Bgr]),                       // C-96
                ("swap".to_string(), vec![W::Swap]),                     // C-97
                ("packing".to_string(), vec![W::Packing]),               // C-98
                ("packswap".to_string(), vec![W::Packswap]),             // C-99
                ("packing+packswap".to_string(), vec![W::Packing, W::Packswap]),
                (format!("shift {:?}", shift), vec![W::Shift(shift)]),   // C-100
                ("invert_mono".to_string(), vec![W::InvertMono]),         // C-101
                ("invert_alpha".to_string(), vec![W::InvertAlpha]),       // C-102
                ("swap_alpha".to_string(), vec![W::SwapAlpha]),           // C-103
                (
                    "swap_alpha+invert_alpha".to_string(),
                    vec![W::SwapAlpha, W::InvertAlpha],
                ),
                ("user(0,0)".to_string(), vec![W::User(0, 0)]),           // C-105
                ("user(8,3)".to_string(), vec![W::User(8, 3)]),
                ("bgr+swap+user".to_string(), vec![W::Bgr, W::Swap, W::User(0, 0)]),
            ];
            // C-104: strip filler on write
            for loc in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
                for v in [0u32, 0xff] {
                    sets.push((format!("filler({},{})", v, loc), vec![W::Filler(v, loc)]));
                }
                sets.push((
                    format!("filler+bgr({})", loc),
                    vec![W::Filler(0, loc), W::Bgr],
                ));
            }

            for (name, ws) in &sets {
                let rb = usr_rowbytes(&img, ws);
                let mut r = Rng::new(seed ^ name.len() as u64 ^ (name.as_bytes()[0] as u64) << 8);
                let rows: Vec<Vec<u8>> = (0..img.h).map(|_| r.bytes(rb)).collect();
                for with_sbit in [false, true] {
                    diff_write(
                        &format!(
                            "w {} ct={} bd={} il={} {}x{} rb={} sbit={}",
                            name, ct, bd, il, img.w, img.h, rb, with_sbit
                        ),
                        &img,
                        ws,
                        &rows,
                        if with_sbit { Some(sb) } else { None },
                    );
                }
            }
        }
    }

    // Deliberately invalid write-side shift values.
    for (ct, bd) in VALID_SHAPES {
        let seed = 0xc100 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32);
        let mut rng = Rng::new(seed);
        let img = rand_img(&mut rng, ct, bd);
        let rows = img.rows.clone();
        for bad in [
            png_color_8 {
                red: 0,
                green: 0,
                blue: 0,
                gray: 0,
                alpha: 0,
            },
            png_color_8 {
                red: (bd + 1) as u8,
                green: (bd + 1) as u8,
                blue: (bd + 1) as u8,
                gray: (bd + 1) as u8,
                alpha: (bd + 1) as u8,
            },
        ] {
            diff_write(
                &format!("w bad shift {:?} ct={} bd={}", bad, ct, bd),
                &img,
                &[W::Shift(bad)],
                &rows,
                None,
            );
        }
    }
}

/* ================================================================== */
/* C-106: png_permit_mng_features + PNG_INTRAPIXEL_DIFFERENCING        */
/* ================================================================== */

/// Write `img` with `filter_type`, `mng_features` permitted and (optionally)
/// no PNG signature at all — the last is what makes filter method 64 legal.
unsafe fn mng_write(
    api: &Api,
    img: &Img,
    mask: u32,
    filter_type: c_int,
    write_sig: bool,
) -> Vec<u8> {
    let (png, info) = new_write(api);
    (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
    let g = guarded(api, png, &mut || {
        if !write_sig {
            (api.png_set_sig_bytes)(png, 8);
        }
        log(format!("permit={}", (api.png_permit_mng_features)(png, mask)));
        (api.png_set_IHDR)(
            png,
            info,
            img.w,
            img.h,
            img.bit_depth,
            img.color_type,
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            filter_type,
        );
        if img.color_type == PNG_COLOR_TYPE_PALETTE && !img.palette.is_empty() {
            (api.png_set_PLTE)(png, info, img.palette.as_ptr(), img.palette.len() as c_int);
        }
        (api.png_write_info)(png, info);
        log_info(api, png, info, "mng after write_info");
        for r in &img.rows {
            (api.png_write_row)(png, r.as_ptr() as *mut u8);
        }
        (api.png_write_end)(png, info);
    });
    log(format!("mng write guard={:?}", g));
    let out = std::mem::take(&mut tls().output);
    destroy_write(api, png, info);
    out
}

unsafe fn mng_read(api: &Api, data: &[u8], mask: u32, sig_bytes: c_int) -> Vec<Vec<u8>> {
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
    let mut out: Vec<Vec<u8>> = Vec::new();
    let g = guarded(api, png, &mut || {
        if sig_bytes > 0 {
            (api.png_set_sig_bytes)(png, sig_bytes);
        }
        log(format!("permit={}", (api.png_permit_mng_features)(png, mask)));
        (api.png_read_info)(png, info);
        log_info(api, png, info, "mng after read_info");
        (api.png_read_update_info)(png, info);
        let h = (api.png_get_image_height)(png, info) as usize;
        let w = (api.png_get_image_width)(png, info) as usize;
        let rb = (api.png_get_rowbytes)(png, info).max(w * 8 + 16);
        out = vec![vec![0u8; rb]; h];
        for y in 0..h {
            (api.png_read_row)(png, out[y].as_mut_ptr(), core::ptr::null_mut());
        }
        (api.png_read_end)(png, info);
    });
    log(format!("mng read guard={:?}", g));
    for (y, r) in out.iter().enumerate() {
        log(format!("mng row {}: {:02x?}", y, r));
    }
    destroy_read(api, png, info);
    out
}

#[test]
fn mng_intrapixel() {
    let shapes: [(c_int, c_int); 6] = [
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ];
    let masks = [
        0u32,
        PNG_FLAG_MNG_EMPTY_PLTE as u32,
        PNG_FLAG_MNG_FILTER_64 as u32,
        PNG_ALL_MNG_FEATURES as u32,
    ];

    for &(ct, bd) in &shapes {
        for &mask in &masks {
            for &filt in &[PNG_FILTER_TYPE_BASE, PNG_INTRAPIXEL_DIFFERENCING] {
                for &write_sig in &[true, false] {
                    let seed = 0xc106
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((mask as u64) << 16)
                        ^ ((filt as u64) << 8)
                        ^ (write_sig as u64);
                    let mut rng = Rng::new(seed);
                    let img = rand_img(&mut rng, ct, bd);

                    let tag = format!(
                        "mng write ct={} bd={} mask={} filt={} sig={}",
                        ct, bd, mask, filt, write_sig
                    );
                    let mut file = Vec::new();
                    assert_same(&tag, |api| unsafe {
                        let mut o = Outcome::default();
                        let bytes = mng_write(api, &img, mask, filt, write_sig);
                        if api.which == "C" {
                            file = bytes.clone();
                        }
                        o.output = bytes;
                        o
                    });
                    if file.is_empty() {
                        continue;
                    }
                    // Read the datastream back.  A stream written without a
                    // signature must be read with png_set_sig_bytes(8); one
                    // written with a signature is read normally, and also with
                    // the signature stripped so that MNG filter 64 is legal.
                    let sig_bytes = if write_sig { 0 } else { 8 };
                    for &rmask in &masks {
                        assert_same(
                            &format!(
                                "mng read ct={} bd={} wmask={} filt={} sig={} rmask={}",
                                ct, bd, mask, filt, write_sig, rmask
                            ),
                            |api| unsafe {
                                let mut o = Outcome::default();
                                let rows = mng_read(api, &file, rmask, sig_bytes);
                                for r in &rows {
                                    o.output.extend_from_slice(r);
                                }
                                o
                            },
                        );
                    }
                    if write_sig && file.len() > 8 {
                        let headless = file[8..].to_vec();
                        for &rmask in &masks {
                            assert_same(
                                &format!(
                                    "mng read headless ct={} bd={} wmask={} filt={} rmask={}",
                                    ct, bd, mask, filt, rmask
                                ),
                                |api| unsafe {
                                    let mut o = Outcome::default();
                                    let rows = mng_read(api, &headless, rmask, 8);
                                    for r in &rows {
                                        o.output.extend_from_slice(r);
                                    }
                                    o
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}
