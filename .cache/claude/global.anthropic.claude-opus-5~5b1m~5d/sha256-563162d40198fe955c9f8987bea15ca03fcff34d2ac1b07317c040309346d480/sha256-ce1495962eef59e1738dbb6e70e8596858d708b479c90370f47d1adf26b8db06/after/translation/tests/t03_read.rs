//! Phase B — the read pipeline, driven through the LOW-LEVEL entry points
//! (`png_read_info` / `png_read_update_info` / `png_read_row` / `png_read_rows` /
//! `png_read_image` / `png_read_end`), the one-shot `png_read_png`, and the
//! progressive reader (`png_process_data`).
//!
//! Every read transform is exercised, alone and in combination, on every legal
//! bit-depth/colour-type/interlace shape, and the decoded rows, all the info
//! getters and the warning transcript are compared byte-for-byte.
mod common;

use common::api::{apis, Api};
use common::harness::*;
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// read configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct RCfg {
    // ---- transforms ----
    pub palette_to_rgb: bool,
    pub expand: bool,
    pub expand_gray_1_2_4_to_8: bool,
    pub trns_to_alpha: bool,
    pub expand_16: bool,
    pub gray_to_rgb: bool,
    pub rgb_to_gray: Option<(c_int, i32, i32)>,
    pub strip_16: bool,
    pub scale_16: bool,
    pub strip_alpha: bool,
    pub swap: bool,
    pub packing: bool,
    pub packswap: bool,
    pub shift: Option<png_color_8>,
    pub invert_mono: bool,
    pub invert_alpha: bool,
    pub swap_alpha: bool,
    pub bgr: bool,
    pub filler: Option<(u32, c_int)>,
    pub add_alpha: Option<(u32, c_int)>,
    pub gamma: Option<(i32, i32)>,
    pub alpha_mode: Option<(c_int, i32)>,
    pub background: Option<(png_color_16, c_int, c_int, i32)>,
    pub quantize: Option<(c_int, bool)>,
    pub interlace_handling: bool,
    // ---- options / limits ----
    pub crc_action: Option<(c_int, c_int)>,
    pub user_limits: Option<(u32, u32)>,
    pub chunk_cache_max: Option<u32>,
    pub chunk_malloc_max: Option<usize>,
    pub option: Option<(c_int, c_int)>,
    pub check_invalid_index: Option<c_int>,
    pub keep_unknown: Option<(c_int, Vec<u8>)>,
    pub benign: Option<c_int>,
    pub mng: Option<u32>,
    pub sig_bytes: Option<c_int>,
    // ---- entry point ----
    pub mode: RMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RMode {
    #[default]
    Row,
    Rows(u32),
    Image,
    /// display_row (second) argument of png_read_row also supplied
    RowDisplay,
    ReadPng(c_int),
    /// progressive reader, feeding n bytes at a time
    Progressive(usize),
}

// ---------------------------------------------------------------------------
// info dump (compared between the two libraries)
// ---------------------------------------------------------------------------

unsafe fn dump_info(a: &Api, p: png_structp, info: png_infop, tag: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut w = 0u32;
    let mut h = 0u32;
    let mut bd = 0i32;
    let mut ct = 0i32;
    let mut il = 0i32;
    let mut cm = 0i32;
    let mut fm = 0i32;
    // `png_get_IHDR` runs png_check_IHDR and raises png_error "Invalid IHDR
    // data" on an empty info struct (pngget.c:974); that rejection belongs to
    // the Phase C error tests, so only call it when an IHDR was actually read.
    // After `png_read_update_info` the info struct can also hold a
    // depth/colour-type pair that is not a legal *PNG* combination (e.g. an
    // 8-bit-per-sample palette expanded in place), which png_check_IHDR also
    // rejects.  Both guards below keep this dump on the valid path.
    let legal_combo = {
        let d = (a.png_get_bit_depth)(p, info) as i32;
        match (a.png_get_color_type)(p, info) as i32 {
            0 => matches!(d, 1 | 2 | 4 | 8 | 16),
            2 | 4 | 6 => matches!(d, 8 | 16),
            3 => matches!(d, 1 | 2 | 4 | 8),
            _ => false,
        }
    };
    if legal_combo
        && (a.png_get_image_width)(p, info) != 0
        && (a.png_get_image_height)(p, info) != 0
    {
        let r = (a.png_get_IHDR)(
            p, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm,
        );
        v.push(format!("{tag}.IHDR:{r}:{w}:{h}:{bd}:{ct}:{il}:{cm}:{fm}"));
    } else {
        v.push(format!("{tag}.IHDR:absent"));
    }
    v.push(format!("{tag}.width:{}", (a.png_get_image_width)(p, info)));
    v.push(format!("{tag}.height:{}", (a.png_get_image_height)(p, info)));
    v.push(format!("{tag}.bit_depth:{}", (a.png_get_bit_depth)(p, info)));
    v.push(format!("{tag}.color_type:{}", (a.png_get_color_type)(p, info)));
    v.push(format!("{tag}.channels:{}", (a.png_get_channels)(p, info)));
    v.push(format!("{tag}.rowbytes:{}", (a.png_get_rowbytes)(p, info)));
    v.push(format!(
        "{tag}.interlace:{}",
        (a.png_get_interlace_type)(p, info)
    ));
    v.push(format!(
        "{tag}.compression:{}",
        (a.png_get_compression_type)(p, info)
    ));
    v.push(format!("{tag}.filter:{}", (a.png_get_filter_type)(p, info)));
    for (name, flag) in [
        ("gAMA", PNG_INFO_gAMA),
        ("sBIT", PNG_INFO_sBIT),
        ("cHRM", PNG_INFO_cHRM),
        ("PLTE", PNG_INFO_PLTE),
        ("tRNS", PNG_INFO_tRNS),
        ("bKGD", PNG_INFO_bKGD),
        ("hIST", PNG_INFO_hIST),
        ("pHYs", PNG_INFO_pHYs),
        ("oFFs", PNG_INFO_oFFs),
        ("tIME", PNG_INFO_tIME),
        ("pCAL", PNG_INFO_pCAL),
        ("sRGB", PNG_INFO_sRGB),
        ("iCCP", PNG_INFO_iCCP),
        ("sPLT", PNG_INFO_sPLT),
        ("sCAL", PNG_INFO_sCAL),
        ("IDAT", PNG_INFO_IDAT),
        ("eXIf", PNG_INFO_eXIf),
        ("cICP", PNG_INFO_cICP),
        ("cLLI", PNG_INFO_cLLI),
        ("mDCV", PNG_INFO_mDCV),
    ] {
        v.push(format!(
            "{tag}.valid.{name}:{}",
            (a.png_get_valid)(p, info, flag)
        ));
    }
    // PLTE
    let mut pal: *mut png_color = std::ptr::null_mut();
    let mut npal = 0i32;
    let got = (a.png_get_PLTE)(p, info, &mut pal, &mut npal);
    v.push(format!("{tag}.PLTE:{got}:{npal}"));
    if got != 0 && !pal.is_null() {
        for i in 0..npal.max(0) as usize {
            let c = *pal.add(i);
            v.push(format!("{tag}.PLTE[{i}]:{}:{}:{}", c.red, c.green, c.blue));
        }
    }
    v.push(format!(
        "{tag}.palette_max:{}",
        (a.png_get_palette_max)(p, info)
    ));
    // tRNS
    let mut ta: *mut png_byte = std::ptr::null_mut();
    let mut nt = 0i32;
    let mut tc: *mut png_color_16 = std::ptr::null_mut();
    let got = (a.png_get_tRNS)(p, info, &mut ta, &mut nt, &mut tc);
    v.push(format!("{tag}.tRNS:{got}:{nt}"));
    if got != 0 {
        if !ta.is_null() {
            let s: Vec<u8> = (0..nt.max(0) as usize).map(|i| *ta.add(i)).collect();
            v.push(format!("{tag}.tRNS.alpha:{s:?}"));
        }
        if !tc.is_null() {
            v.push(format!("{tag}.tRNS.col:{:?}", *tc));
        }
    }
    // gAMA / sRGB / cHRM / sBIT / bKGD / hIST / pHYs / oFFs / tIME / sCAL
    let mut g = 0i32;
    v.push(format!(
        "{tag}.gAMA:{}:{g}",
        (a.png_get_gAMA_fixed)(p, info, &mut g)
    ));
    let mut si = 0i32;
    v.push(format!(
        "{tag}.sRGB:{}:{si}",
        (a.png_get_sRGB)(p, info, &mut si)
    ));
    {
        let mut a1 = [0i32; 8];
        let got = (a.png_get_cHRM_fixed)(
            p, info, &mut a1[0], &mut a1[1], &mut a1[2], &mut a1[3], &mut a1[4], &mut a1[5],
            &mut a1[6], &mut a1[7],
        );
        v.push(format!("{tag}.cHRM:{got}:{a1:?}"));
        let mut a2 = [0i32; 9];
        let got = (a.png_get_cHRM_XYZ_fixed)(
            p, info, &mut a2[0], &mut a2[1], &mut a2[2], &mut a2[3], &mut a2[4], &mut a2[5],
            &mut a2[6], &mut a2[7], &mut a2[8],
        );
        v.push(format!("{tag}.cHRM_XYZ:{got}:{a2:?}"));
    }
    let mut sb: *mut png_color_8 = std::ptr::null_mut();
    let got = (a.png_get_sBIT)(p, info, &mut sb);
    v.push(format!(
        "{tag}.sBIT:{got}:{:?}",
        if got != 0 && !sb.is_null() {
            Some(*sb)
        } else {
            None
        }
    ));
    let mut bg: *mut png_color_16 = std::ptr::null_mut();
    let got = (a.png_get_bKGD)(p, info, &mut bg);
    v.push(format!(
        "{tag}.bKGD:{got}:{:?}",
        if got != 0 && !bg.is_null() {
            Some(*bg)
        } else {
            None
        }
    ));
    let mut hi: *mut png_uint_16 = std::ptr::null_mut();
    let got = (a.png_get_hIST)(p, info, &mut hi);
    v.push(format!("{tag}.hIST:{got}"));
    if got != 0 && !hi.is_null() {
        let n = npal.max(0) as usize;
        let s: Vec<u16> = (0..n).map(|i| *hi.add(i)).collect();
        v.push(format!("{tag}.hIST.vals:{s:?}"));
    }
    let (mut rx, mut ry, mut ut) = (0u32, 0u32, 0i32);
    v.push(format!(
        "{tag}.pHYs:{}:{rx}:{ry}:{ut}",
        (a.png_get_pHYs)(p, info, &mut rx, &mut ry, &mut ut)
    ));
    let (mut ox, mut oy, mut ut2) = (0i32, 0i32, 0i32);
    v.push(format!(
        "{tag}.oFFs:{}:{ox}:{oy}:{ut2}",
        (a.png_get_oFFs)(p, info, &mut ox, &mut oy, &mut ut2)
    ));
    let mut tm: *mut png_time = std::ptr::null_mut();
    let got = (a.png_get_tIME)(p, info, &mut tm);
    v.push(format!(
        "{tag}.tIME:{got}:{:?}",
        if got != 0 && !tm.is_null() {
            Some(*tm)
        } else {
            None
        }
    ));
    {
        let (mut u, mut sw, mut sh) = (0i32, 0i32, 0i32);
        // png_get_sCAL_fixed can png_error on overflow; only call it when the
        // chunk is absent or the values are in range -- guarded by the flag.
        if (a.png_get_valid)(p, info, PNG_INFO_sCAL) == 0 {
            v.push(format!(
                "{tag}.sCAL:{}",
                (a.png_get_sCAL_fixed)(p, info, &mut u, &mut sw, &mut sh)
            ));
        }
        let (mut u2, mut sws, mut shs): (c_int, *mut c_char, *mut c_char) =
            (0, std::ptr::null_mut(), std::ptr::null_mut());
        let got = (a.png_get_sCAL_s)(p, info, &mut u2, &mut sws, &mut shs);
        v.push(format!(
            "{tag}.sCAL_s:{got}:{u2}:{}:{}",
            cstr_to_string(sws),
            cstr_to_string(shs)
        ));
    }
    // eXIf / cICP / cLLI / mDCV
    {
        let mut n = 0u32;
        let mut e: *mut png_byte = std::ptr::null_mut();
        let got = (a.png_get_eXIf_1)(p, info, &mut n, &mut e);
        v.push(format!("{tag}.eXIf:{got}:{n}"));
        if got != 0 && !e.is_null() {
            let s: Vec<u8> = (0..n as usize).map(|i| *e.add(i)).collect();
            v.push(format!("{tag}.eXIf.data:{s:02x?}"));
        }
    }
    {
        let mut b = [0u8; 4];
        let got = (a.png_get_cICP)(p, info, &mut b[0], &mut b[1], &mut b[2], &mut b[3]);
        v.push(format!("{tag}.cICP:{got}:{b:?}"));
        let mut c2 = [0u32; 2];
        let got = (a.png_get_cLLI_fixed)(p, info, &mut c2[0], &mut c2[1]);
        v.push(format!("{tag}.cLLI:{got}:{c2:?}"));
        let mut m = [0i32; 8];
        let mut lum = [0u32; 2];
        let got = (a.png_get_mDCV_fixed)(
            p, info, &mut m[0], &mut m[1], &mut m[2], &mut m[3], &mut m[4], &mut m[5], &mut m[6],
            &mut m[7], &mut lum[0], &mut lum[1],
        );
        v.push(format!("{tag}.mDCV:{got}:{m:?}:{lum:?}"));
    }
    // iCCP
    {
        let mut name: *mut c_char = std::ptr::null_mut();
        let mut comp = 0i32;
        let mut prof: *mut png_byte = std::ptr::null_mut();
        let mut plen = 0u32;
        let got = (a.png_get_iCCP)(p, info, &mut name, &mut comp, &mut prof, &mut plen);
        v.push(format!(
            "{tag}.iCCP:{got}:{}:{comp}:{plen}",
            cstr_to_string(name)
        ));
        if got != 0 && !prof.is_null() {
            let s: Vec<u8> = (0..plen as usize).map(|i| *prof.add(i)).collect();
            v.push(format!("{tag}.iCCP.data:{:02x?}", &s[..s.len().min(64)]));
        }
    }
    // pCAL
    {
        let mut purpose: *mut c_char = std::ptr::null_mut();
        let (mut x0, mut x1) = (0i32, 0i32);
        let (mut ty, mut np) = (0i32, 0i32);
        let mut units: *mut c_char = std::ptr::null_mut();
        let mut params: *mut *mut c_char = std::ptr::null_mut();
        let got = (a.png_get_pCAL)(
            p, info, &mut purpose, &mut x0, &mut x1, &mut ty, &mut np, &mut units, &mut params,
        );
        v.push(format!(
            "{tag}.pCAL:{got}:{}:{x0}:{x1}:{ty}:{np}:{}",
            cstr_to_string(purpose),
            cstr_to_string(units)
        ));
        if got != 0 && !params.is_null() {
            for i in 0..np.max(0) as usize {
                v.push(format!("{tag}.pCAL.p[{i}]:{}", cstr_to_string(*params.add(i))));
            }
        }
    }
    // sPLT
    {
        let mut e: *mut png_sPLT_t = std::ptr::null_mut();
        let n = (a.png_get_sPLT)(p, info, &mut e);
        v.push(format!("{tag}.sPLT:{n}"));
        if n > 0 && !e.is_null() {
            for i in 0..n as usize {
                let s = *e.add(i);
                v.push(format!(
                    "{tag}.sPLT[{i}]:{}:{}:{}",
                    cstr_to_string(s.name),
                    s.depth,
                    s.nentries
                ));
                if !s.entries.is_null() {
                    for j in 0..s.nentries.max(0) as usize {
                        v.push(format!("{tag}.sPLT[{i}][{j}]:{:?}", *s.entries.add(j)));
                    }
                }
            }
        }
    }
    // text
    {
        let mut tp: *mut png_text = std::ptr::null_mut();
        let mut n = 0i32;
        let got = (a.png_get_text)(p, info, &mut tp, &mut n);
        v.push(format!("{tag}.text:{got}:{n}"));
        if got > 0 && !tp.is_null() {
            for i in 0..n.max(0) as usize {
                let t = *tp.add(i);
                v.push(format!(
                    "{tag}.text[{i}]:{}:{}:{}:{}:{}:{}",
                    t.compression,
                    cstr_to_string(t.key),
                    cstr_to_string(t.text),
                    t.text_length,
                    t.itxt_length,
                    cstr_to_string(t.lang)
                ));
            }
        }
    }
    // unknown chunks
    {
        let mut u: *mut png_unknown_chunk = std::ptr::null_mut();
        let n = (a.png_get_unknown_chunks)(p, info, &mut u);
        v.push(format!("{tag}.unknown:{n}"));
        if n > 0 && !u.is_null() {
            for i in 0..n as usize {
                let c = *u.add(i);
                let name: Vec<u8> = c.name[..4].to_vec();
                let data: Vec<u8> = if c.data.is_null() {
                    Vec::new()
                } else {
                    (0..c.size).map(|j| *c.data.add(j)).collect()
                };
                v.push(format!(
                    "{tag}.unknown[{i}]:{}:{}:{}:{:02x?}",
                    String::from_utf8_lossy(&name),
                    c.size,
                    c.location,
                    &data[..data.len().min(64)]
                ));
            }
        }
    }
    // struct-level state
    v.push(format!(
        "{tag}.rgb_to_gray_status:{}",
        (a.png_get_rgb_to_gray_status)(p)
    ));
    v.push(format!("{tag}.io_state:{}", (a.png_get_io_state)(p)));
    v.push(format!(
        "{tag}.io_chunk_type:{:#x}",
        (a.png_get_io_chunk_type)(p)
    ));
    v.push(format!(
        "{tag}.user_width_max:{}",
        (a.png_get_user_width_max)(p)
    ));
    v.push(format!(
        "{tag}.user_height_max:{}",
        (a.png_get_user_height_max)(p)
    ));
    v.push(format!(
        "{tag}.chunk_cache_max:{}",
        (a.png_get_chunk_cache_max)(p)
    ));
    v.push(format!(
        "{tag}.chunk_malloc_max:{}",
        (a.png_get_chunk_malloc_max)(p)
    ));
    v.push(format!(
        "{tag}.compression_buffer_size:{}",
        (a.png_get_compression_buffer_size)(p)
    ));
    v.push(format!(
        "{tag}.current_pass:{}",
        (a.png_get_current_pass_number)(p)
    ));
    v.push(format!(
        "{tag}.current_row:{}",
        (a.png_get_current_row_number)(p)
    ));
    let sig = (a.png_get_signature)(p, info);
    if !sig.is_null() {
        let s: Vec<u8> = (0..8).map(|i| *sig.add(i)).collect();
        v.push(format!("{tag}.signature:{s:02x?}"));
    }
    v
}

// ---------------------------------------------------------------------------
// applying the transforms
// ---------------------------------------------------------------------------

unsafe fn apply_transforms(a: &Api, p: png_structp, cfg: &RCfg, pal: &mut Vec<png_color>) {
    if cfg.palette_to_rgb {
        (a.png_set_palette_to_rgb)(p);
    }
    if cfg.expand {
        (a.png_set_expand)(p);
    }
    if cfg.expand_gray_1_2_4_to_8 {
        (a.png_set_expand_gray_1_2_4_to_8)(p);
    }
    if cfg.trns_to_alpha {
        (a.png_set_tRNS_to_alpha)(p);
    }
    if cfg.expand_16 {
        (a.png_set_expand_16)(p);
    }
    if cfg.gray_to_rgb {
        (a.png_set_gray_to_rgb)(p);
    }
    if let Some((ea, r, g)) = cfg.rgb_to_gray {
        (a.png_set_rgb_to_gray_fixed)(p, ea, r, g);
    }
    if let Some((bg, code, need_expand, gamma)) = cfg.background {
        (a.png_set_background_fixed)(p, &bg, code, need_expand, gamma);
    }
    if let Some((mode, og)) = cfg.alpha_mode {
        (a.png_set_alpha_mode_fixed)(p, mode, og);
    }
    if let Some((sg, fg)) = cfg.gamma {
        (a.png_set_gamma_fixed)(p, sg, fg);
    }
    if cfg.strip_16 {
        (a.png_set_strip_16)(p);
    }
    if cfg.scale_16 {
        (a.png_set_scale_16)(p);
    }
    if cfg.strip_alpha {
        (a.png_set_strip_alpha)(p);
    }
    if let Some((maxcol, full)) = cfg.quantize {
        // build a deterministic palette + histogram for the quantizer
        if pal.is_empty() {
            let mut rng = Rng::new(0xdeadbeef);
            *pal = (0..256)
                .map(|_| png_color {
                    red: rng.next_u8(),
                    green: rng.next_u8(),
                    blue: rng.next_u8(),
                })
                .collect();
        }
        let hist: Vec<u16> = (0..pal.len() as u16).collect();
        (a.png_set_quantize)(
            p,
            pal.as_mut_ptr(),
            pal.len() as c_int,
            maxcol,
            hist.as_ptr(),
            if full { 1 } else { 0 },
        );
    }
    if cfg.packing {
        (a.png_set_packing)(p);
    }
    if cfg.swap {
        (a.png_set_swap)(p);
    }
    if cfg.packswap {
        (a.png_set_packswap)(p);
    }
    if let Some(s) = cfg.shift {
        (a.png_set_shift)(p, &s);
    }
    if cfg.invert_mono {
        (a.png_set_invert_mono)(p);
    }
    if cfg.invert_alpha {
        (a.png_set_invert_alpha)(p);
    }
    if cfg.swap_alpha {
        (a.png_set_swap_alpha)(p);
    }
    if cfg.bgr {
        (a.png_set_bgr)(p);
    }
    if let Some((f, fl)) = cfg.filler {
        (a.png_set_filler)(p, f, fl);
    }
    if let Some((f, fl)) = cfg.add_alpha {
        (a.png_set_add_alpha)(p, f, fl);
    }
}

unsafe fn apply_options(a: &Api, p: png_structp, cfg: &RCfg) {
    if let Some(b) = cfg.benign {
        (a.png_set_benign_errors)(p, b);
    }
    if let Some((c1, c2)) = cfg.crc_action {
        (a.png_set_crc_action)(p, c1, c2);
    }
    if let Some((w, h)) = cfg.user_limits {
        (a.png_set_user_limits)(p, w, h);
    }
    if let Some(m) = cfg.chunk_cache_max {
        (a.png_set_chunk_cache_max)(p, m);
    }
    if let Some(m) = cfg.chunk_malloc_max {
        (a.png_set_chunk_malloc_max)(p, m);
    }
    if let Some((o, v)) = cfg.option {
        let r = (a.png_set_option)(p, o, v);
        log_push(format!("OPTION:{o}:{v}:{r}"));
    }
    if let Some(v) = cfg.check_invalid_index {
        (a.png_set_check_for_invalid_index)(p, v);
    }
    if let Some((keep, list)) = &cfg.keep_unknown {
        if list.is_empty() {
            (a.png_set_keep_unknown_chunks)(p, *keep, std::ptr::null(), 0);
        } else {
            (a.png_set_keep_unknown_chunks)(
                p,
                *keep,
                list.as_ptr(),
                (list.len() / 5) as c_int,
            );
        }
    }
    if let Some(m) = cfg.mng {
        let r = (a.png_permit_mng_features)(p, m);
        log_push(format!("MNG:{r}"));
    }
}

// ---------------------------------------------------------------------------
// the driver
// ---------------------------------------------------------------------------

pub struct ReadOut {
    pub info_before: Vec<String>,
    pub info_after: Vec<String>,
    pub info_end: Vec<String>,
    pub rows: Vec<u8>,
    pub display: Vec<u8>,
    pub log: Vec<String>,
    pub rowbytes: usize,
}

/// Progressive-reader callbacks.  They must be plain `extern "C"` functions
/// shared by both libraries, so all state lives in the thread-local `PROG` log.
/// libpng's progressive reader delivers NO rows unless the application starts
/// the image from the info callback (`png_start_read_image` /
/// `png_read_update_info`); without it the IDAT is rejected with "Truncated
/// compressed data in IDAT" and the row callback never fires.  The function
/// pointer is resolved from whichever library is being driven.
type StartFn = unsafe extern "C" fn(png_structp);
thread_local! {
    static PROG_START: std::cell::RefCell<Option<StartFn>> =
        const { std::cell::RefCell::new(None) };
}

unsafe extern "C" fn prog_info(p: png_structp, _i: png_infop) {
    prog_push("INFO".to_string());
    if let Some(f) = PROG_START.with(|c| *c.borrow()) {
        f(p);
    }
}
unsafe extern "C" fn prog_row(
    _p: png_structp,
    row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    if row.is_null() {
        prog_push(format!("ROW:{row_num}:{pass}:NULL"));
    } else {
        // the row length is not passed to the callback; record a digest of the
        // first bytes plus a checksum computed over the recorded rowbytes.
        let n = PROG_ROWBYTES.with(|r| *r.borrow());
        let s: Vec<u8> = (0..n).map(|i| *row.add(i)).collect();
        prog_push(format!("ROW:{row_num}:{pass}:{:02x?}", s));
    }
}
unsafe extern "C" fn prog_end(_p: png_structp, _i: png_infop) {
    prog_push("END".to_string());
}

thread_local! {
    static PROG_ROWBYTES: std::cell::RefCell<usize> = const { std::cell::RefCell::new(0) };
}

unsafe fn run_read(a: &Api, is_c: bool, png: &[u8], cfg: &RCfg) -> ReadOut {
    set_cur_is_c(is_c);
    reset_all();
    in_set(png);

    let mut p = (a.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(error_cb),
        Some(warn_cb),
    );
    assert!(!p.is_null());
    let mut info = (a.png_create_info_struct)(p);
    let mut end_info = (a.png_create_info_struct)(p);
    (a.png_set_read_fn)(p, std::ptr::null_mut(), Some(read_cb));
    (a.png_set_read_status_fn)(p, Some(read_status_cb));
    apply_options(a, p, cfg);

    let mut pal: Vec<png_color> = Vec::new();

    if let RMode::Progressive(chunk) = cfg.mode {
        // the progressive API needs the row length known up-front; run a probe
        // read with the sequential API first to learn it.
        (a.png_destroy_read_struct)(&mut p, &mut info, &mut end_info);
        in_set(png);
        let mut p = (a.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warn_cb),
        );
        let mut info = (a.png_create_info_struct)(p);
        let mut end_info = (a.png_create_info_struct)(p);
        apply_options(a, p, cfg);
        (a.png_set_progressive_read_fn)(
            p,
            std::ptr::null_mut(),
            Some(prog_info),
            Some(prog_row),
            Some(prog_end),
        );
        // rowbytes for the *untransformed* image (no transforms are applied in
        // the progressive test) -- computed from the IHDR we built.
        let bd = png[8 + 8 + 8 + 8] as u32; // not used; replaced below
        let _ = bd;
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let bit_depth = png[24];
        let color_type = png[25];
        let rb = pb::rowbytes(bit_depth, color_type, w);
        PROG_ROWBYTES.with(|r| *r.borrow_mut() = rb);
        PROG_START.with(|c| {
            *c.borrow_mut() = Some(sym::<StartFn>(
                if is_c { &libs().c } else { &libs().rs },
                "png_start_read_image",
            ))
        });

        let mut i = 0usize;
        while i < png.len() {
            let n = chunk.max(1).min(png.len() - i);
            (a.png_process_data)(p, info, png[i..].as_ptr() as *mut png_byte, n);
            i += n;
        }
        let ib = dump_info(a, p, info, "prog");
        let ie = dump_info(a, p, end_info, "prog_end");
        let plog = prog_take();
        (a.png_destroy_read_struct)(&mut p, &mut info, &mut end_info);
        let mut log = log_take();
        log.extend(plog);
        return ReadOut {
            info_before: ib,
            info_after: Vec::new(),
            info_end: ie,
            rows: Vec::new(),
            display: Vec::new(),
            log,
            rowbytes: rb,
        };
    }

    if let RMode::ReadPng(transforms) = cfg.mode {
        (a.png_read_png)(p, info, transforms, std::ptr::null_mut());
        let ib = dump_info(a, p, info, "png");
        let h = (a.png_get_image_height)(p, info);
        let rb = (a.png_get_rowbytes)(p, info);
        let rows = (a.png_get_rows)(p, info);
        // `png_read_png` allocates the row buffers with `png_malloc`, i.e.
        // UNINITIALISED (pngread.c:1059).  When the pixel depth is < 8 and the
        // row does not end on a byte boundary, `png_combine_row` deliberately
        // *preserves* the caller's bits outside the image
        // (`end_byte = *end_ptr`, pngrutil.c:3266).  Those bits are therefore
        // indeterminate and differ between two different allocators, so the
        // final byte of such a row is excluded from the comparison.
        let w = (a.png_get_image_width)(p, info);
        let pd = (a.png_get_bit_depth)(p, info) as u32 * (a.png_get_channels)(p, info) as u32;
        let partial = (pd as u64 * w as u64) % 8 != 0;
        let take = if partial { rb.saturating_sub(1) } else { rb };
        let mut data = Vec::new();
        if !rows.is_null() {
            for y in 0..h as usize {
                let r = *rows.add(y);
                if !r.is_null() {
                    data.extend((0..take).map(|i| *r.add(i)));
                }
            }
        }
        (a.png_destroy_read_struct)(&mut p, &mut info, &mut end_info);
        return ReadOut {
            info_before: ib,
            info_after: Vec::new(),
            info_end: Vec::new(),
            rows: data,
            display: Vec::new(),
            log: log_take(),
            rowbytes: rb,
        };
    }

    if let Some(n) = cfg.sig_bytes {
        // consume the first n bytes ourselves then tell libpng about them
        let mut tmp = vec![0u8; n as usize];
        IN.with_read(&mut tmp);
        (a.png_set_sig_bytes)(p, n);
    }

    (a.png_read_info)(p, info);
    let info_before = dump_info(a, p, info, "before");

    apply_transforms(a, p, cfg, &mut pal);
    let passes = if cfg.interlace_handling {
        (a.png_set_interlace_handling)(p)
    } else {
        1
    };
    log_push(format!("PASSES:{passes}"));
    (a.png_read_update_info)(p, info);
    let info_after = dump_info(a, p, info, "after");

    let h = (a.png_get_image_height)(p, info) as usize;
    let rb = (a.png_get_rowbytes)(p, info);
    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb]).collect();
    let mut disp: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rb]).collect();

    match cfg.mode {
        RMode::Row => {
            for _ in 0..passes {
                for y in 0..h {
                    (a.png_read_row)(p, rows[y].as_mut_ptr(), std::ptr::null_mut());
                }
            }
        }
        RMode::RowDisplay => {
            for _ in 0..passes {
                for y in 0..h {
                    let dp = disp[y].as_mut_ptr();
                    (a.png_read_row)(p, rows[y].as_mut_ptr(), dp);
                }
            }
        }
        RMode::Rows(n) => {
            let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            for _ in 0..passes {
                let mut i = 0usize;
                while i < ptrs.len() {
                    let k = (n as usize).min(ptrs.len() - i);
                    (a.png_read_rows)(p, ptrs[i..].as_mut_ptr(), std::ptr::null_mut(), k as u32);
                    i += k;
                }
            }
        }
        RMode::Image => {
            let mut ptrs: Vec<*mut png_byte> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            (a.png_read_image)(p, ptrs.as_mut_ptr());
        }
        _ => unreachable!(),
    }

    (a.png_read_end)(p, end_info);
    let info_end = dump_info(a, p, end_info, "end");

    let flat: Vec<u8> = rows.concat();
    let flatd: Vec<u8> = disp.concat();
    (a.png_destroy_read_struct)(&mut p, &mut info, &mut end_info);
    ReadOut {
        info_before,
        info_after,
        info_end,
        rows: flat,
        display: flatd,
        log: log_take(),
        rowbytes: rb,
    }
}

/// tiny helper so `sig_bytes` can pull bytes out of the shared input buffer
trait ReadFromIn {
    fn with_read(&self, buf: &mut [u8]);
}
struct InKey;
#[allow(non_upper_case_globals)]
const IN: InKey = InKey;
impl ReadFromIn for InKey {
    fn with_read(&self, buf: &mut [u8]) {
        // reuse the harness reader by calling it with a NULL png_structp is not
        // possible; instead advance the buffer directly.
        let n = buf.len();
        let taken = harness_take(n);
        buf[..taken.len()].copy_from_slice(&taken);
    }
}

fn harness_take(n: usize) -> Vec<u8> {
    // The harness keeps (data, pos); emulate a read of n bytes.
    let mut out = vec![0u8; n];
    unsafe {
        // read_cb only needs a non-null png_structp when it has to raise an
        // error, which cannot happen while bytes remain.
        common::harness::read_cb(std::ptr::null_mut(), out.as_mut_ptr(), n);
    }
    out
}

#[track_caller]
fn diff_read(png: &[u8], cfg: &RCfg, what: &str) {
    // Set PNG_TRACE=1 to see which case is running; a `png_error` on a valid
    // path aborts the process, so this is how such a case is identified.
    if std::env::var_os("PNG_TRACE").is_some() {
        eprintln!("CASE {what}");
    }
    let b = apis();
    let c = unsafe { run_read(&b.c, true, png, cfg) };
    let r = unsafe { run_read(&b.rs, false, png, cfg) };
    eq_dbg(&format!("{what}: rowbytes"), c.rowbytes, r.rowbytes);
    eq_dbg(&format!("{what}: info before update"), c.info_before, r.info_before);
    eq_dbg(&format!("{what}: info after update"), c.info_after, r.info_after);
    eq_dbg(&format!("{what}: info at end"), c.info_end, r.info_end);
    eq_bytes(&format!("{what}: decoded rows"), &c.rows, &r.rows);
    eq_bytes(&format!("{what}: display rows"), &c.display, &r.display);
    eq_dbg(&format!("{what}: transcript"), c.log, r.log);
}

// ---------------------------------------------------------------------------
// the shapes we read
// ---------------------------------------------------------------------------

const DEPTH_TYPE: [(u8, u8); 15] = [
    (1, 0),
    (2, 0),
    (4, 0),
    (8, 0),
    (16, 0),
    (8, 2),
    (16, 2),
    (1, 3),
    (2, 3),
    (4, 3),
    (8, 3),
    (8, 4),
    (16, 4),
    (8, 6),
    (16, 6),
];

fn png_for(seed: u64, w: u32, h: u32, bd: u8, ct: u8, il: u8) -> Vec<u8> {
    pb::make_png(seed, w, h, bd, ct, il)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[test]
fn plain_read_all_shapes() {
    let widths = [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 33];
    let heights = [1u32, 2, 3, 8, 9];
    let mut seed = 0x1_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for &w in &widths {
                for &h in &heights {
                    seed += 1;
                    let png = png_for(seed, w, h, bd, ct, il);
                    let mut cfg = RCfg::default();
                    cfg.interlace_handling = il == 1;
                    cfg.mode = if il == 1 { RMode::Image } else { RMode::Row };
                    diff_read(&png, &cfg, &format!("plain {bd}/{ct}/il{il} {w}x{h}"));
                }
            }
        }
    }
}

#[test]
fn read_entry_points() {
    let mut seed = 0x2_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for mode in [
                RMode::Row,
                RMode::RowDisplay,
                RMode::Rows(1),
                RMode::Rows(2),
                RMode::Rows(3),
                RMode::Rows(100),
                RMode::Image,
            ] {
                seed += 1;
                let png = png_for(seed, 19, 6, bd, ct, il);
                let mut cfg = RCfg::default();
                cfg.mode = mode;
                cfg.interlace_handling = il == 1 && mode != RMode::Image;
                if il == 1 && mode == RMode::Image {
                    // png_read_image handles the passes itself
                }
                diff_read(&png, &cfg, &format!("entry {mode:?} {bd}/{ct}/il{il}"));
            }
        }
    }
}

#[test]
fn read_png_one_shot() {
    let masks = [
        PNG_TRANSFORM_IDENTITY,
        PNG_TRANSFORM_STRIP_16,
        PNG_TRANSFORM_STRIP_ALPHA,
        PNG_TRANSFORM_PACKING,
        PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_EXPAND,
        PNG_TRANSFORM_INVERT_MONO,
        PNG_TRANSFORM_SHIFT,
        PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_SWAP_ENDIAN,
        PNG_TRANSFORM_INVERT_ALPHA,
        PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_EXPAND_16,
        PNG_TRANSFORM_SCALE_16,
        PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SCALE_16 | PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB,
        PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_PACKING | PNG_TRANSFORM_INVERT_MONO,
    ];
    let mut seed = 0x3_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for &m in &masks {
                seed += 1;
                let png = png_for(seed, 13, 5, bd, ct, il);
                let mut cfg = RCfg::default();
                cfg.mode = RMode::ReadPng(m);
                diff_read(&png, &cfg, &format!("read_png {m:#x} {bd}/{ct}/il{il}"));
            }
        }
    }
}

#[test]
fn progressive_reader() {
    let mut seed = 0x4_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for chunk in [1usize, 2, 3, 7, 13, 64, 1000, 100000] {
                seed += 1;
                let png = png_for(seed, 17, 5, bd, ct, il);
                let mut cfg = RCfg::default();
                cfg.mode = RMode::Progressive(chunk);
                diff_read(
                    &png,
                    &cfg,
                    &format!("progressive chunk={chunk} {bd}/{ct}/il{il}"),
                );
            }
        }
    }
}

#[test]
fn single_transforms() {
    let mut seed = 0x5_0000u64;
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for k in 0..22 {
            let mut cfg = RCfg::default();
            match k {
                0 => cfg.palette_to_rgb = true,
                1 => cfg.expand = true,
                2 => cfg.expand_gray_1_2_4_to_8 = true,
                3 => cfg.trns_to_alpha = true,
                4 => {
                    cfg.expand = true;
                    cfg.expand_16 = true;
                }
                5 => cfg.gray_to_rgb = true,
                6 => cfg.strip_16 = true,
                7 => cfg.scale_16 = true,
                8 => cfg.strip_alpha = true,
                9 => cfg.swap = true,
                10 => cfg.packing = true,
                11 => cfg.packswap = true,
                12 => cfg.invert_mono = true,
                13 => cfg.invert_alpha = true,
                14 => cfg.swap_alpha = true,
                15 => cfg.bgr = true,
                16 => cfg.filler = Some((0x8000, PNG_FILLER_AFTER)),
                17 => cfg.filler = Some((0x8000, PNG_FILLER_BEFORE)),
                18 => cfg.add_alpha = Some((0xffff, PNG_FILLER_AFTER)),
                19 => cfg.add_alpha = Some((0xffff, PNG_FILLER_BEFORE)),
                20 => {
                    cfg.shift = Some(png_color_8 {
                        red: bd.min(8),
                        green: bd.min(8),
                        blue: bd.min(8),
                        gray: bd.min(8),
                        alpha: bd.min(8),
                    })
                }
                _ => cfg.gamma = Some((100000, 45455)),
            }
            // `png_set_filler` rejects low-bit-depth grey outright (Phase C
            // row) and `png_set_add_alpha` on a sub-8-bit or palette image
            // leaves libpng's info rowbytes inconsistent with the transformed
            // pixel depth, which trips the internal
            // "internal row size calculation error" (also a Phase C row).
            // Neither is a valid path, so both are exercised only for
            // bit depth >= 8 and a non-palette colour type here.
            if (cfg.filler.is_some() || cfg.add_alpha.is_some()) && (bd < 8 || ct == 3) {
                continue;
            }
            // `png_set_add_alpha`/`filler` on a palette image is a no-op the
            // library warns about; keep it, it is a valid call.
            for &il in &[0u8, 1] {
                seed += 1;
                let png = png_for(seed, 15, 4, bd, ct, il);
                let mut c2 = cfg.clone();
                c2.mode = RMode::Image;
                diff_read(&png, &c2, &format!("xform{k} {bd}/{ct}/il{il}"));
            }
        }
    }
}

#[test]
fn expand_family_with_trns() {
    // tRNS present is what makes png_set_expand / tRNS_to_alpha interesting
    let mut seed = 0x6_0000u64;
    for &(bd, ct) in &[(1u8, 0u8), (2, 0), (4, 0), (8, 0), (16, 0), (8, 2), (16, 2),
                       (1, 3), (2, 3), (4, 3), (8, 3)] {
        for &il in &[0u8, 1] {
            seed += 1;
            let mut rng = Rng::new(seed);
            let mut spec = pb::PngSpec::new(15, 4, bd, ct, il);
            if ct == 3 {
                let n = (1usize << bd).min(256);
                spec.palette = (0..n * 3).map(|_| rng.next_u8()).collect();
                spec.trns = Some((0..n).map(|_| rng.next_u8()).collect());
            } else if ct == 0 {
                let maxv: u16 = if bd == 16 { 0xffff } else { ((1u32 << bd) - 1) as u16 };
                spec.trns = Some(((maxv / 2) as u16).to_be_bytes().to_vec());
            } else {
                let maxv: u16 = if bd == 16 { 0xffff } else { 255 };
                let mut d = Vec::new();
                d.extend_from_slice(&(maxv / 2).to_be_bytes());
                d.extend_from_slice(&(maxv / 3).to_be_bytes());
                d.extend_from_slice(&(maxv / 4).to_be_bytes());
                spec.trns = Some(d);
            }
            spec.raw = if il == 1 {
                let mut r2 = Rng::new(seed ^ 7);
                pb::raw_rows_adam7(15, 4, bd, ct, &mut |_p, _y, rb| {
                    (0..rb).map(|_| r2.next_u8()).collect()
                })
            } else {
                let mut r2 = Rng::new(seed ^ 7);
                pb::raw_rows_none(15, 4, bd, ct, &mut |_y, rb| {
                    (0..rb).map(|_| r2.next_u8()).collect()
                })
            };
            let png = spec.build();

            for k in 0..8 {
                let mut cfg = RCfg::default();
                cfg.mode = RMode::Image;
                match k {
                    0 => cfg.expand = true,
                    1 => cfg.trns_to_alpha = true,
                    2 => cfg.palette_to_rgb = true,
                    3 => {
                        cfg.expand = true;
                        cfg.expand_16 = true;
                    }
                    4 => {
                        cfg.expand = true;
                        cfg.gray_to_rgb = true;
                    }
                    5 => {
                        cfg.expand = true;
                        cfg.strip_alpha = true;
                    }
                    6 => cfg.expand_gray_1_2_4_to_8 = true,
                    _ => {
                        cfg.expand = true;
                        cfg.bgr = true;
                        cfg.invert_alpha = true;
                        cfg.swap_alpha = true;
                    }
                }
                diff_read(&png, &cfg, &format!("trns-expand{k} {bd}/{ct}/il{il}"));
            }
        }
    }
}

#[test]
fn gamma_and_background_and_alpha_mode() {
    let mut seed = 0x7_0000u64;
    let gammas = [
        (100000i32, 45455i32),
        (45455, 100000),
        (220000, 45455),
        (100000, 100000),
        (1000, 100000),
        (2_000_000, 45455),
        (50000, 200000),
        (10_000_000, 45455),
        (45455, 1000),
        (45455, 10_000_000),
    ];
    // NOTE: pngrtran.c:344 rejects gamma outside
    // [PNG_LIB_GAMMA_MIN=1000, PNG_LIB_GAMMA_MAX=10000000] with
    // "gamma out of supported range"; that is a Phase C row, so only in-range
    // values appear here (plus the PNG_DEFAULT_sRGB / PNG_GAMMA_MAC_18 flags).
    for &(bd, ct) in DEPTH_TYPE.iter() {
        for &il in &[0u8, 1] {
            for &(sg, fg) in &gammas {
                seed += 1;
                let png = png_for(seed, 13, 4, bd, ct, il);
                let mut cfg = RCfg::default();
                cfg.mode = RMode::Image;
                cfg.gamma = Some((sg, fg));
                cfg.expand = ct == 3;
                diff_read(&png, &cfg, &format!("gamma {sg}/{fg} {bd}/{ct}/il{il}"));
            }
            // alpha modes
            for mode in [PNG_ALPHA_PNG, PNG_ALPHA_STANDARD, PNG_ALPHA_OPTIMIZED, PNG_ALPHA_BROKEN] {
                for og in [PNG_DEFAULT_sRGB, 100000, 45455] {
                    seed += 1;
                    let png = png_for(seed, 13, 4, bd, ct, il);
                    let mut cfg = RCfg::default();
                    cfg.mode = RMode::Image;
                    cfg.alpha_mode = Some((mode, og));
                    diff_read(&png, &cfg, &format!("alpha_mode {mode}/{og} {bd}/{ct}/il{il}"));
                }
            }
            // backgrounds
            for code in [
                PNG_BACKGROUND_GAMMA_SCREEN,
                PNG_BACKGROUND_GAMMA_FILE,
                PNG_BACKGROUND_GAMMA_UNIQUE,
            ] {
                for need_expand in [0i32, 1] {
                    seed += 1;
                    let png = png_for(seed, 13, 4, bd, ct, il);
                    let maxv: u16 = if bd == 16 { 0xffff } else { 255 };
                    let bg = png_color_16 {
                        index: 0,
                        red: maxv / 3,
                        green: maxv / 5,
                        blue: maxv / 7,
                        gray: maxv / 2,
                    };
                    let mut cfg = RCfg::default();
                    cfg.mode = RMode::Image;
                    // png_set_background needs expansion for palette images
                    cfg.expand = ct == 3 && need_expand == 0;
                    cfg.background = Some((bg, code, need_expand, 100000));
                    cfg.gamma = Some((100000, 45455));
                    diff_read(
                        &png,
                        &cfg,
                        &format!("background {code}/{need_expand} {bd}/{ct}/il{il}"),
                    );
                }
            }
        }
    }
}

#[test]
fn rgb_to_gray_all_actions() {
    let mut seed = 0x8_0000u64;
    for &(bd, ct) in &[(8u8, 2u8), (16, 2), (8, 6), (16, 6), (8, 3), (4, 3)] {
        for &il in &[0u8, 1] {
            // error_action 3 == PNG_RGB_TO_GRAY_ERR raises
            // png_error "png_do_rgb_to_gray found nongray pixel" on a colour
            // image (pngrtran.c:4969); that is a Phase C row, so the valid path
            // uses the silent (1) and warn (2) actions.
            for action in [1i32, 2] {
                for &(r, g) in &[(-1i32, -1i32), (6968, 23434), (0, 0), (100000, 0), (50000, 50000)] {
                    seed += 1;
                    let png = png_for(seed, 13, 4, bd, ct, il);
                    let mut cfg = RCfg::default();
                    cfg.mode = RMode::Image;
                    cfg.expand = ct == 3;
                    cfg.rgb_to_gray = Some((action, r, g));
                    diff_read(
                        &png,
                        &cfg,
                        &format!("rgb_to_gray {action}/{r}/{g} {bd}/{ct}/il{il}"),
                    );
                }
            }
        }
    }
}

#[test]
fn quantize_all() {
    let mut seed = 0x9_0000u64;
    for &(bd, ct) in &[(8u8, 2u8), (8, 6), (16, 2), (8, 3), (4, 3), (8, 0)] {
        for &il in &[0u8, 1] {
            for maxcol in [1i32, 2, 15, 16, 17, 255, 256] {
                for full in [false, true] {
                    seed += 1;
                    let png = png_for(seed, 13, 4, bd, ct, il);
                    let mut cfg = RCfg::default();
                    cfg.mode = RMode::Image;
                    cfg.expand = ct == 3;
                    cfg.strip_16 = bd == 16;
                    cfg.quantize = Some((maxcol, full));
                    diff_read(
                        &png,
                        &cfg,
                        &format!("quantize {maxcol}/{full} {bd}/{ct}/il{il}"),
                    );
                }
            }
        }
    }
}

#[test]
fn ancillary_chunks_on_read() {
    // every ancillary chunk libpng understands, read back through the getters
    let mut seed = 0xa_0000u64;
    let mut mk = |pre: Vec<([u8; 4], Vec<u8>)>, post: Vec<([u8; 4], Vec<u8>)>, seed: u64| {
        let mut spec = pb::PngSpec::new(9, 3, 8, 2, 0);
        spec.pre_idat = pre;
        spec.post_idat = post;
        let mut r2 = Rng::new(seed);
        spec.raw = pb::raw_rows_none(9, 3, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| r2.next_u8()).collect()
        });
        spec.build()
    };

    let cases: Vec<(&str, Vec<([u8; 4], Vec<u8>)>)> = vec![
        ("gAMA", vec![(*b"gAMA", 45455u32.to_be_bytes().to_vec())]),
        ("gAMA-0", vec![(*b"gAMA", 0u32.to_be_bytes().to_vec())]),
        ("sRGB", vec![(*b"sRGB", vec![0])]),
        ("sRGB1", vec![(*b"sRGB", vec![1])]),
        ("sRGB2", vec![(*b"sRGB", vec![2])]),
        ("sRGB3", vec![(*b"sRGB", vec![3])]),
        ("sBIT", vec![(*b"sBIT", vec![5, 6, 7])]),
        (
            "cHRM",
            vec![(
                *b"cHRM",
                [31270u32, 32900, 64000, 33000, 30000, 60000, 15000, 6000]
                    .iter()
                    .flat_map(|v| v.to_be_bytes())
                    .collect(),
            )],
        ),
        (
            "bKGD",
            vec![(*b"bKGD", vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc])],
        ),
        ("pHYs", vec![(*b"pHYs", {
            let mut d = Vec::new();
            d.extend_from_slice(&300u32.to_be_bytes());
            d.extend_from_slice(&400u32.to_be_bytes());
            d.push(1);
            d
        })]),
        ("oFFs", vec![(*b"oFFs", {
            let mut d = Vec::new();
            d.extend_from_slice(&(-7i32).to_be_bytes());
            d.extend_from_slice(&11i32.to_be_bytes());
            d.push(1);
            d
        })]),
        ("tIME", vec![(*b"tIME", {
            let mut d = Vec::new();
            d.extend_from_slice(&2024u16.to_be_bytes());
            d.extend_from_slice(&[2, 29, 23, 59, 60]);
            d
        })]),
        ("tEXt", vec![(*b"tEXt", b"Title\0hello world".to_vec())]),
        ("tEXt-empty", vec![(*b"tEXt", b"Key\0".to_vec())]),
        ("zTXt", vec![(*b"zTXt", {
            let mut d = b"Key\0".to_vec();
            d.push(0); // compression method
            d.extend_from_slice(&pb::zlib_store(b"compressed text"));
            d
        })]),
        ("iTXt", vec![(*b"iTXt", {
            let mut d = b"Key\0".to_vec();
            d.push(0); // compression flag
            d.push(0); // compression method
            d.extend_from_slice(b"en\0");
            d.extend_from_slice(b"Schl\xc3\xbcssel\0");
            d.extend_from_slice(b"international text");
            d
        })]),
        ("iTXt-z", vec![(*b"iTXt", {
            let mut d = b"Key\0".to_vec();
            d.push(1);
            d.push(0);
            d.extend_from_slice(b"en\0");
            d.extend_from_slice(b"k\0");
            d.extend_from_slice(&pb::zlib_store(b"compressed international"));
            d
        })]),
        ("sCAL", vec![(*b"sCAL", {
            let mut d = vec![1u8];
            d.extend_from_slice(b"1.5\0");
            d.extend_from_slice(b"2.5");
            d
        })]),
        ("pCAL", vec![(*b"pCAL", {
            let mut d = b"purpose\0".to_vec();
            d.extend_from_slice(&0i32.to_be_bytes());
            d.extend_from_slice(&255i32.to_be_bytes());
            d.push(0); // equation type: linear
            d.push(2); // nparams
            d.extend_from_slice(b"units\0");
            d.extend_from_slice(b"1.0\0");
            d.extend_from_slice(b"2.0");
            d
        })]),
        ("sPLT", vec![(*b"sPLT", {
            let mut d = b"name\0".to_vec();
            d.push(8);
            d.extend_from_slice(&[1, 2, 3, 4, 0, 5]);
            d.extend_from_slice(&[6, 7, 8, 9, 0, 10]);
            d
        })]),
        ("sPLT16", vec![(*b"sPLT", {
            let mut d = b"n16\0".to_vec();
            d.push(16);
            for v in 0u16..5 {
                for _ in 0..4 {
                    d.extend_from_slice(&(v * 1000).to_be_bytes());
                }
                d.extend_from_slice(&(v).to_be_bytes());
            }
            d
        })]),
        ("eXIf", vec![(*b"eXIf", b"II\x2a\x00\x08\x00\x00\x00".to_vec())]),
        ("cICP", vec![(*b"cICP", vec![9, 16, 0, 1])]),
        ("cLLI", vec![(*b"cLLI", {
            let mut d = Vec::new();
            d.extend_from_slice(&10_000_000u32.to_be_bytes());
            d.extend_from_slice(&4_000_000u32.to_be_bytes());
            d
        })]),
        ("mDCV", vec![(*b"mDCV", {
            let mut d = Vec::new();
            for v in [31270u16, 32900, 64000, 33000, 30000, 60000, 15000, 6000] {
                d.extend_from_slice(&v.to_be_bytes());
            }
            d.extend_from_slice(&10_000_000u32.to_be_bytes());
            d.extend_from_slice(&50u32.to_be_bytes());
            d
        })]),
        ("iCCP", vec![(*b"iCCP", {
            // a minimal but structurally valid ICC profile is required; use an
            // unknown-but-parsable one so the read path is exercised.  libpng
            // validates the header, so build one that passes.
            let mut prof = vec![0u8; 132];
            let len = prof.len() as u32;
            prof[0..4].copy_from_slice(&len.to_be_bytes());
            prof[4..8].copy_from_slice(b"ADBE");
            prof[8..12].copy_from_slice(&0x0400_0000u32.to_be_bytes());
            prof[12..16].copy_from_slice(b"mntr");
            prof[16..20].copy_from_slice(b"RGB ");
            prof[20..24].copy_from_slice(b"XYZ ");
            prof[36..40].copy_from_slice(b"acsp");
            // PCS illuminant D50
            prof[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
            prof[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
            prof[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
            // tag count 0
            prof[128..132].copy_from_slice(&0u32.to_be_bytes());
            let mut d = b"icc\0".to_vec();
            d.push(0);
            d.extend_from_slice(&pb::zlib_store(&prof));
            d
        })]),
        ("hIST-nopalette", vec![(*b"hIST", vec![0, 1, 0, 2])]),
        ("unknown", vec![(*b"prVt", vec![1, 2, 3, 4])]),
        ("unknown-empty", vec![(*b"prVt", vec![])]),
        // NOTE: a chunk whose first letter is upper case is *critical*; an
        // unrecognised critical chunk is the "unhandled critical chunk"
        // rejection (a Phase C row), so only ancillary names appear here.
        // Chunk-name bit rules: byte 0 lower = ancillary, byte 2 MUST be upper
        // (reserved bit), byte 3 lower = safe-to-copy.  `prVt` is ancillary +
        // safe-to-copy, `prVT` is ancillary + unsafe-to-copy.  A name with an
        // upper-case byte 0 is critical and a lower-case byte 2 is reserved --
        // both are Phase C rejections, not valid inputs.
        ("unknown-unsafe-to-copy", vec![(*b"prVT", vec![9, 8])]),
    ];

    for (name, chunks) in &cases {
        seed += 1;
        for post in [false, true] {
            let png = if post {
                mk(Vec::new(), chunks.clone(), seed)
            } else {
                mk(chunks.clone(), Vec::new(), seed)
            };
            for keep in [
                None,
                Some((PNG_HANDLE_CHUNK_ALWAYS, Vec::new())),
                Some((PNG_HANDLE_CHUNK_NEVER, Vec::new())),
                Some((PNG_HANDLE_CHUNK_IF_SAFE, Vec::new())),
                Some((PNG_HANDLE_CHUNK_AS_DEFAULT, Vec::new())),
            ] {
                let mut cfg = RCfg::default();
                cfg.mode = RMode::Image;
                cfg.keep_unknown = keep.clone();
                diff_read(
                    &png,
                    &cfg,
                    &format!("chunk {name} post={post} keep={:?}", keep.as_ref().map(|k| k.0)),
                );
            }
            // and through the progressive reader
            let mut cfg = RCfg::default();
            cfg.mode = RMode::Progressive(3);
            diff_read(&png, &cfg, &format!("chunk {name} post={post} progressive"));
        }
    }
}

#[test]
fn palette_shapes_and_idat_splitting() {
    let mut seed = 0xb_0000u64;
    // palettes of 1..256 entries at every legal depth
    for bd in [1u8, 2, 4, 8] {
        let maxn = (1usize << bd).min(256);
        for n in [1usize, 2, 3, maxn / 2 + 1, maxn] {
            if n == 0 || n > maxn {
                continue;
            }
            seed += 1;
            let mut rng = Rng::new(seed);
            let mut spec = pb::PngSpec::new(11, 3, bd, 3, 0);
            spec.palette = (0..n * 3).map(|_| rng.next_u8()).collect();
            spec.raw = pb::raw_rows_none(11, 3, bd, 3, &mut |_y, rb| {
                (0..rb).map(|_| (rng.next_u8()) & ((1u16 << bd) - 1) as u8).collect()
            });
            let png = spec.build();
            for check in [None, Some(0i32), Some(1i32)] {
                let mut cfg = RCfg::default();
                cfg.mode = RMode::Image;
                cfg.check_invalid_index = check;
                diff_read(&png, &cfg, &format!("palette {bd}bpp n={n} check={check:?}"));
            }
        }
    }
    // IDAT split across many chunks, plus a zero-length IDAT
    for nchunks in [1usize, 2, 3, 7, 50] {
        seed += 1;
        let mut rng = Rng::new(seed);
        let mut spec = pb::PngSpec::new(20, 8, 8, 2, 0);
        spec.idat_chunks = nchunks;
        spec.raw = pb::raw_rows_none(20, 8, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| rng.next_u8()).collect()
        });
        let png = spec.build();
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        diff_read(&png, &cfg, &format!("idat split {nchunks}"));
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Progressive(5);
        diff_read(&png, &cfg, &format!("idat split {nchunks} progressive"));
    }
}

#[test]
fn options_and_limits() {
    let mut seed = 0xc_0000u64;
    let png = png_for(seed, 20, 8, 8, 2, 0);
    for o in [
        PNG_MAXIMUM_INFLATE_WINDOW,
        PNG_SKIP_sRGB_CHECK_PROFILE,
        PNG_IGNORE_ADLER32,
    ] {
        for v in [PNG_OPTION_OFF, PNG_OPTION_ON] {
            let mut cfg = RCfg::default();
            cfg.mode = RMode::Image;
            cfg.option = Some((o, v));
            diff_read(&png, &cfg, &format!("option {o}={v}"));
        }
    }
    for crc in [
        (PNG_CRC_DEFAULT, PNG_CRC_DEFAULT),
        (PNG_CRC_ERROR_QUIT, PNG_CRC_ERROR_QUIT),
        (PNG_CRC_WARN_DISCARD, PNG_CRC_WARN_DISCARD),
        (PNG_CRC_WARN_USE, PNG_CRC_WARN_USE),
        (PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE),
        (PNG_CRC_NO_CHANGE, PNG_CRC_NO_CHANGE),
    ] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.crc_action = Some(crc);
        diff_read(&png, &cfg, &format!("crc_action {crc:?}"));
    }
    for lim in [(20u32, 8u32), (1_000_000, 1_000_000), (u32::MAX, u32::MAX), (0x7fff_ffff, 0x7fff_ffff)] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.user_limits = Some(lim);
        diff_read(&png, &cfg, &format!("user_limits {lim:?}"));
    }
    for m in [0u32, 1, 10, 1000, u32::MAX] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.chunk_cache_max = Some(m);
        diff_read(&png, &cfg, &format!("chunk_cache_max {m}"));
    }
    for m in [0usize, 1, 1000, 8_000_000, usize::MAX] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.chunk_malloc_max = Some(m);
        diff_read(&png, &cfg, &format!("chunk_malloc_max {m}"));
    }
    for b in [0i32, 1] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.benign = Some(b);
        diff_read(&png, &cfg, &format!("benign {b}"));
    }
    for m in [0u32, 1, 4, 5] {
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        cfg.mng = Some(m);
        diff_read(&png, &cfg, &format!("mng {m}"));
    }
    seed += 1;
    let _ = seed;
}

#[test]
fn randomised_transform_combinations() {
    let mut rng = Rng::new(0xd_0000);
    for i in 0..3000 {
        let (bd, ct) = DEPTH_TYPE[rng.below(DEPTH_TYPE.len() as u32) as usize];
        let il = if rng.bool() { 1u8 } else { 0u8 };
        let w = rng.range(1, 24);
        let h = rng.range(1, 8);
        let png = png_for(0xd_0000 + i, w, h, bd, ct, il);
        let mut cfg = RCfg::default();
        cfg.mode = RMode::Image;
        if rng.bool() {
            cfg.expand = true;
        }
        if rng.bool() {
            cfg.palette_to_rgb = true;
        }
        if rng.bool() {
            cfg.expand_gray_1_2_4_to_8 = true;
        }
        if rng.bool() {
            cfg.trns_to_alpha = true;
        }
        if rng.bool() {
            cfg.gray_to_rgb = true;
        }
        if rng.bool() {
            cfg.expand_16 = true;
        }
        if rng.bool() {
            cfg.strip_16 = true;
        } else if rng.bool() {
            cfg.scale_16 = true;
        }
        if rng.bool() {
            cfg.strip_alpha = true;
        }
        if rng.bool() {
            cfg.swap = true;
        }
        if rng.bool() {
            cfg.packing = true;
        }
        if rng.bool() {
            cfg.packswap = true;
        }
        if rng.bool() {
            cfg.invert_mono = true;
        }
        if rng.bool() {
            cfg.invert_alpha = true;
        }
        if rng.bool() {
            cfg.swap_alpha = true;
        }
        if rng.bool() {
            cfg.bgr = true;
        }
        if rng.bool() {
            cfg.gamma = Some((rng.range(1000, 500000) as i32, rng.range(1000, 500000) as i32));
        }
        // `png_set_filler` / `png_set_add_alpha` add a channel WITHOUT raising
        // the bit depth, so on a sub-8-bit or palette image (unless it is being
        // expanded first) libpng's info rowbytes and its transformed pixel depth
        // disagree and png_combine_row raises "internal row size calculation
        // error".  That is a Phase C row, so only the consistent combinations
        // appear on this valid path.
        let alpha_ok = (bd >= 8 && ct != 3) || cfg.expand;
        if alpha_ok {
            if rng.bool() {
                cfg.add_alpha = Some((
                    rng.next_u32() & 0xffff,
                    if rng.bool() { PNG_FILLER_AFTER } else { PNG_FILLER_BEFORE },
                ));
            } else if rng.bool() && ct != 3 {
                cfg.filler = Some((
                    rng.next_u32() & 0xffff,
                    if rng.bool() { PNG_FILLER_AFTER } else { PNG_FILLER_BEFORE },
                ));
            }
        }
        if rng.bool() {
            cfg.alpha_mode = Some((rng.below(4) as c_int, rng.range(1000, 500000) as i32));
        }
        diff_read(&png, &cfg, &format!("random{i} {bd}/{ct}/il{il} {w}x{h} {cfg:?}"));
    }
}
