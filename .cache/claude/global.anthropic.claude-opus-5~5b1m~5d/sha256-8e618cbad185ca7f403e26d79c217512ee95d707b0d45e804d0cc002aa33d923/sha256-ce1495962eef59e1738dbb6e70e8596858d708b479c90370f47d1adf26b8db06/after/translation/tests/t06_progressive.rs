//! Phase B — the PROGRESSIVE ("push") read API:
//! `png_set_progressive_read_fn`, `png_process_data`, `png_process_data_pause`,
//! `png_process_data_skip`, `png_progressive_combine_row`,
//! `png_get_progressive_ptr` plus the exported push helpers
//! (`png_push_read_sig`, `png_push_read_chunk`, `png_push_read_IDAT`,
//! `png_push_fill_buffer`, `png_push_save_buffer`, `png_push_restore_buffer`,
//! `png_push_process_row`, `png_push_have_info`, `png_push_have_end`,
//! `png_push_have_row`, `png_process_some_data`, `png_read_push_finish_row`).
//!
//! Method: a PNG byte stream is produced with the sequential write path
//! (verified byte-for-byte in `t03_write`), then the *same* bytes are pushed
//! through both shared objects at several chunk granularities.  Everything the
//! callbacks can observe (the `png_get_*` snapshot at info time, every
//! (row bytes, row_num, pass) triple, the end callback, the captured
//! warnings/errors) must be identical.
mod common;
use common::*;
use std::ptr::{null, null_mut};

// ---------------------------------------------------------------------------
// Source images + the (already verified) sequential writer
// ---------------------------------------------------------------------------

struct Src {
    w: u32,
    h: u32,
    bd: c_int,
    ct: c_int,
    il: c_int,
    palette: Vec<png_color>,
    rows: Vec<Vec<u8>>,
    anc: bool,
}

impl Src {
    fn gen(rng: &mut Rng, ct: c_int, bd: c_int, w: u32, h: u32, il: c_int, anc: bool) -> Src {
        let pd = channels_of(ct) * bd as u32;
        let rb = rowbytes(pd, w);
        let rows = (0..h).map(|_| rng.bytes(rb)).collect();
        let npal = if ct == PNG_COLOR_TYPE_PALETTE {
            1usize << bd
        } else {
            0
        };
        let palette = (0..npal)
            .map(|_| png_color {
                red: rng.u8(),
                green: rng.u8(),
                blue: rng.u8(),
            })
            .collect();
        Src {
            w,
            h,
            bd,
            ct,
            il,
            palette,
            rows,
            anc,
        }
    }

    fn pixel_depth(&self) -> u32 {
        channels_of(self.ct) * self.bd as u32
    }
}

/// Produce the PNG byte stream with the sequential write path.
///
/// `t03_write` already proves both libraries emit byte-identical streams for
/// exactly these configurations, so the C writer is used as the single source
/// of the bytes that are then pushed through *both* readers.
unsafe fn encode(api: &'static Api, s: &Src) -> Vec<u8> {
    set_current_api(api);
    diag_reset();
    let mut sess = WriteSess::new(api);
    let png = sess.png;
    let info = sess.info;

    let key = cs("Title");
    let txt = cs("progressive test");
    let text = [png_text {
        compression: PNG_TEXT_COMPRESSION_NONE,
        key: key.as_ptr() as png_charp,
        text: txt.as_ptr() as png_charp,
        text_length: 16,
        itxt_length: 0,
        lang: null_mut(),
        lang_key: null_mut(),
    }];
    let trns_alpha: Vec<u8> = if s.ct == PNG_COLOR_TYPE_PALETTE {
        (0..s.palette.len())
            .map(|i| (i as u8).wrapping_mul(37).wrapping_add(3))
            .collect()
    } else {
        Vec::new()
    };
    // Keep every sample <= (1 << bit_depth) - 1 so no library warns.
    let trns_col = png_color_16 {
        index: 0,
        red: 1,
        green: 1,
        blue: 1,
        gray: 1,
    };
    let bkgd = png_color_16 {
        index: 0,
        red: 1,
        green: 1,
        blue: 1,
        gray: 1,
    };

    let ok = guard(|| {
        (api.png_set_IHDR)(
            png,
            info,
            s.w,
            s.h,
            s.bd,
            s.ct,
            s.il,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if !s.palette.is_empty() {
            (api.png_set_PLTE)(png, info, s.palette.as_ptr(), s.palette.len() as c_int);
        }
        if s.anc {
            (api.png_set_gAMA)(png, info, 0.45455);
            (api.png_set_text)(png, info, text.as_ptr(), 1);
            (api.png_set_bKGD)(png, info, &bkgd as *const png_color_16);
            match s.ct {
                PNG_COLOR_TYPE_PALETTE => (api.png_set_tRNS)(
                    png,
                    info,
                    trns_alpha.as_ptr() as png_bytep,
                    trns_alpha.len() as c_int,
                    null_mut(),
                ),
                PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_RGB => (api.png_set_tRNS)(
                    png,
                    info,
                    null_mut(),
                    0,
                    &trns_col as *const png_color_16 as png_color_16p,
                ),
                // tRNS is illegal for the two color types that already carry an
                // alpha channel.
                _ => {}
            }
        }
        (api.png_write_info)(png, info);
        let mut rowps: Vec<png_bytep> = s.rows.iter().map(|r| r.as_ptr() as png_bytep).collect();
        (api.png_write_image)(png, rowps.as_mut_ptr());
        (api.png_write_end)(png, info);
    });
    let d = diag_take();
    assert!(ok.is_some(), "encode failed: {:?}", d);
    assert!(d.errors.is_empty(), "encode errors: {:?}", d);
    assert!(d.warnings.is_empty(), "encode warnings: {:?}", d);
    std::mem::take(&mut sess.sink.buf)
}

// ---------------------------------------------------------------------------
// Progressive reader harness
// ---------------------------------------------------------------------------

fn pass_cols(w: u32, pass: usize) -> u32 {
    let inc = PNG_PASS_INC[pass];
    let start = PNG_PASS_START_COL[pass];
    if w <= start {
        0
    } else {
        (w + inc - 1 - start) / inc
    }
}

/// Where (if anywhere) the callbacks call `png_process_data_pause`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PauseAt {
    Never,
    Info,
    End,
}

#[derive(Clone, Copy, Debug)]
struct Cfg {
    /// call `png_set_interlace_handling` in the info callback
    interlace_handling: bool,
    /// use `png_read_update_info` instead of `png_start_read_image`
    update_info: bool,
    /// call `png_progressive_combine_row` from the row callback
    combine: bool,
    pause_at: PauseAt,
    pause_save: c_int,
}

impl Cfg {
    fn plain() -> Cfg {
        Cfg {
            interlace_handling: true,
            update_info: false,
            combine: false,
            pause_at: PauseAt::Never,
            pause_save: 0,
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug)]
struct Res {
    ok: bool,
    diag: Diag,
    info_calls: u32,
    info_before: Vec<String>,
    info_after: Vec<String>,
    passes: c_int,
    /// (row_num, pass, current_row_number, current_pass_number, row bytes)
    rows: Vec<(png_uint_32, c_int, png_uint_32, png_byte, Option<Vec<u8>>)>,
    ends: u32,
    end_snapshot: Vec<String>,
    display: Vec<Vec<u8>>,
    pause_rets: Vec<usize>,
    ptr_ok: Vec<bool>,
}

#[repr(C)]
struct Cap {
    me: *mut Cap,
    cfg: Cfg,
    res: Res,
    // geometry, discovered in the info callback
    w: u32,
    h: u32,
    pd: u32,
    interlaced: bool,
    /// bytes of the app buffer that libpng did not consume before a
    /// `png_process_data_pause(save == 0)`; the app must supply them again.
    pending_unproc: usize,
    paused: bool,
}

impl Cap {
    fn new(cfg: Cfg) -> Cap {
        Cap {
            me: null_mut(),
            cfg,
            res: Res::default(),
            w: 0,
            h: 0,
            pd: 0,
            interlaced: false,
            pending_unproc: 0,
            paused: false,
        }
    }

    fn row_len(&self, pass: c_int) -> usize {
        let p = (pass.clamp(0, 6)) as usize;
        if self.interlaced && !self.cfg.interlace_handling {
            rowbytes(self.pd, pass_cols(self.w, p))
        } else {
            rowbytes(self.pd, self.w)
        }
    }

    unsafe fn maybe_pause(&mut self, api: &'static Api, png: png_structp, at: PauseAt) {
        if self.cfg.pause_at == at && !self.paused {
            self.paused = true;
            let r = (api.png_process_data_pause)(png, self.cfg.pause_save);
            self.res.pause_rets.push(r);
            if self.cfg.pause_save == 0 {
                self.pending_unproc = r;
            }
        }
    }
}

unsafe fn snapshot(api: &'static Api, png: png_structp, info: png_infop, tag: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let (mut w, mut h) = (0u32, 0u32);
    let (mut bd, mut ct, mut il, mut cp, mut ft) = (0i32, 0i32, 0i32, 0i32, 0i32);
    let r = (api.png_get_IHDR)(
        png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cp, &mut ft,
    );
    v.push(format!(
        "{tag} IHDR r={r} {w}x{h} bd={bd} ct={ct} il={il} cp={cp} ft={ft}"
    ));
    v.push(format!(
        "{tag} w={} h={} bd={} ct={} il={} cp={} ft={} ch={} rb={}",
        (api.png_get_image_width)(png, info),
        (api.png_get_image_height)(png, info),
        (api.png_get_bit_depth)(png, info),
        (api.png_get_color_type)(png, info),
        (api.png_get_interlace_type)(png, info),
        (api.png_get_compression_type)(png, info),
        (api.png_get_filter_type)(png, info),
        (api.png_get_channels)(png, info),
        (api.png_get_rowbytes)(png, info),
    ));
    let flags = [
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
        ("sRGB", PNG_INFO_sRGB),
        ("iCCP", PNG_INFO_iCCP),
        ("sPLT", PNG_INFO_sPLT),
        ("sCAL", PNG_INFO_sCAL),
        ("IDAT", PNG_INFO_IDAT),
        ("eXIf", PNG_INFO_eXIf),
    ];
    let mut s = String::new();
    for (n, f) in flags {
        s += &format!("{}={} ", n, (api.png_get_valid)(png, info, f));
    }
    v.push(format!("{tag} valid {s}"));

    let mut g = 0f64;
    let rg = (api.png_get_gAMA)(png, info, &mut g);
    let mut gf = 0i32;
    let rgf = (api.png_get_gAMA_fixed)(png, info, &mut gf);
    v.push(format!("{tag} gAMA r={rg} {g:.6} fixed r={rgf} {gf}"));

    let mut pal: png_colorp = null_mut();
    let mut np = 0i32;
    let rp = (api.png_get_PLTE)(png, info, &mut pal, &mut np);
    if rp != 0 && !pal.is_null() && np > 0 {
        let sl = std::slice::from_raw_parts(pal, np as usize);
        v.push(format!("{tag} PLTE r={rp} n={np} {sl:?}"));
    } else {
        v.push(format!("{tag} PLTE r={rp} n={np}"));
    }

    let mut ta: png_bytep = null_mut();
    let mut nt = 0i32;
    let mut tc: png_color_16p = null_mut();
    let rt = (api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
    let av = if !ta.is_null() && nt > 0 {
        hex(std::slice::from_raw_parts(ta, nt as usize))
    } else {
        String::from("-")
    };
    let cv = if tc.is_null() {
        String::from("-")
    } else {
        format!("{:?}", *tc)
    };
    v.push(format!("{tag} tRNS r={rt} n={nt} a={av} c={cv}"));

    let mut bg: png_color_16p = null_mut();
    let rb = (api.png_get_bKGD)(png, info, &mut bg);
    v.push(format!(
        "{tag} bKGD r={rb} {}",
        if bg.is_null() {
            String::from("-")
        } else {
            format!("{:?}", *bg)
        }
    ));

    let mut tp: png_textp = null_mut();
    let mut ntext = 0i32;
    let rtx = (api.png_get_text)(png, info, &mut tp, &mut ntext);
    let mut ts = String::new();
    if !tp.is_null() {
        for i in 0..ntext as usize {
            let t = &*tp.add(i);
            ts += &format!(
                "[c={} k={:?} t={:?} tl={} il={}]",
                t.compression,
                rs_str(t.key as png_const_charp),
                rs_str(t.text as png_const_charp),
                t.text_length,
                t.itxt_length
            );
        }
    }
    v.push(format!("{tag} text r={rtx} n={ntext} {ts}"));

    let mut sb: png_color_8p = null_mut();
    let rsb = (api.png_get_sBIT)(png, info, &mut sb);
    v.push(format!(
        "{tag} sBIT r={rsb} {}",
        if sb.is_null() {
            String::from("-")
        } else {
            format!("{:?}", *sb)
        }
    ));

    let (mut px, mut py, mut pu) = (0u32, 0u32, 0i32);
    let rph = (api.png_get_pHYs)(png, info, &mut px, &mut py, &mut pu);
    v.push(format!("{tag} pHYs r={rph} {px} {py} {pu}"));

    let mut si = 0i32;
    let rsr = (api.png_get_sRGB)(png, info, &mut si);
    v.push(format!("{tag} sRGB r={rsr} {si}"));

    let sig = (api.png_get_signature)(png, info);
    v.push(format!(
        "{tag} sig {}",
        if sig.is_null() {
            String::from("-")
        } else {
            hex(std::slice::from_raw_parts(sig, 8))
        }
    ));
    v
}

unsafe extern "C-unwind" fn on_info(png: png_structp, info: png_infop) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Cap;
    assert!(!p.is_null(), "progressive ptr lost in info callback");
    let cap = &mut *p;
    cap.res.ptr_ok.push(p == cap.me);
    cap.res.info_calls += 1;

    cap.res.info_before = snapshot(api, png, info, "before");

    cap.w = (api.png_get_image_width)(png, info);
    cap.h = (api.png_get_image_height)(png, info);
    cap.pd = (api.png_get_channels)(png, info) as u32 * (api.png_get_bit_depth)(png, info) as u32;
    cap.interlaced = (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7;

    if cap.cfg.interlace_handling {
        cap.res.passes = (api.png_set_interlace_handling)(png);
    }
    // The progressive reader never initialises the row buffers itself, so the
    // app *must* do this from the info callback (see png_push_have_info /
    // png_read_start_row).
    if cap.cfg.update_info {
        (api.png_read_update_info)(png, info);
    } else {
        (api.png_start_read_image)(png);
    }

    cap.res.info_after = snapshot(api, png, info, "after");

    if cap.cfg.combine {
        let rb = rowbytes(cap.pd, cap.w);
        // png_combine_row reads the last destination byte when the row does not
        // end on a byte boundary, so the buffer must be initialised.
        cap.res.display = (0..cap.h as usize).map(|_| vec![0u8; rb + 8]).collect();
    }

    cap.maybe_pause(api, png, PauseAt::Info);
}

unsafe extern "C-unwind" fn on_row(
    png: png_structp,
    row: png_bytep,
    row_num: png_uint_32,
    pass: c_int,
) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Cap;
    assert!(!p.is_null(), "progressive ptr lost in row callback");
    let cap = &mut *p;
    cap.res.ptr_ok.push(p == cap.me);

    let bytes = if row.is_null() {
        None
    } else {
        let n = cap.row_len(pass);
        Some(std::slice::from_raw_parts(row, n).to_vec())
    };

    if cap.cfg.combine {
        let idx = row_num as usize;
        if idx < cap.res.display.len() {
            let dp = cap.res.display[idx].as_mut_ptr();
            (api.png_progressive_combine_row)(png as png_const_structrp, dp, row as png_const_bytep);
        }
    }

    let cur_row = (api.png_get_current_row_number)(png);
    let cur_pass = (api.png_get_current_pass_number)(png);
    cap.res.rows.push((row_num, pass, cur_row, cur_pass, bytes));
}

unsafe extern "C-unwind" fn on_end(png: png_structp, info: png_infop) {
    let api = current_api();
    let p = (api.png_get_progressive_ptr)(png) as *mut Cap;
    assert!(!p.is_null(), "progressive ptr lost in end callback");
    let cap = &mut *p;
    cap.res.ptr_ok.push(p == cap.me);
    cap.res.ends += 1;
    cap.res.end_snapshot = snapshot(api, png, info, "end");
    cap.maybe_pause(api, png, PauseAt::End);
}

/// Chunk granularity used when feeding the stream.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gran {
    N(usize),
    All,
}

unsafe fn progressive(api: &'static Api, bytes: &[u8], cfg: Cfg, gran: Gran) -> Res {
    set_current_api(api);
    diag_reset();
    let v = ver();
    let png = (api.png_create_read_struct)(
        v.as_ptr(),
        null_mut(),
        Some(cb_error),
        Some(cb_warning),
    );
    assert!(!png.is_null());
    let info = (api.png_create_info_struct)(png);
    assert!(!info.is_null());

    let mut cap = Box::new(Cap::new(cfg));
    let capp: *mut Cap = &mut *cap;
    cap.me = capp;
    (api.png_set_progressive_read_fn)(
        png,
        capp as png_voidp,
        Some(on_info),
        Some(on_row),
        Some(on_end),
    );
    assert_eq!(
        (api.png_get_progressive_ptr)(png) as *mut Cap,
        capp,
        "{}: png_get_progressive_ptr",
        api.name
    );

    let mut data = bytes.to_vec();
    let ok = guard(|| {
        let step = match gran {
            Gran::N(n) => n,
            Gran::All => data.len().max(1),
        };
        let mut pos = 0usize;
        let mut spins = 0u32;
        while pos < data.len() && spins < 5_000_000 {
            spins += 1;
            let len = step.min(data.len() - pos);
            (*capp).pending_unproc = 0;
            (api.png_process_data)(png, info, data.as_mut_ptr().add(pos), len);
            let unproc = (*capp).pending_unproc.min(len);
            (*capp).pending_unproc = 0;
            // `paused` is sticky, so re-supplying the same bytes on the next
            // iteration always makes progress.
            pos += len - unproc;
        }
        // Drain anything png_push_save_buffer is still holding (this happens
        // after png_process_data_pause(save != 0)).
        for _ in 0..8 {
            if (*capp).res.ends != 0 {
                break;
            }
            let snap = (
                (*capp).res.rows.len(),
                (*capp).res.info_calls,
                (*capp).res.ends,
            );
            (api.png_process_data)(png, info, null_mut(), 0);
            if (
                (*capp).res.rows.len(),
                (*capp).res.info_calls,
                (*capp).res.ends,
            ) == snap
            {
                break;
            }
        }
    })
    .is_some();

    let diag = diag_take();
    let mut res = std::mem::take(&mut cap.res);
    res.ok = ok;
    res.diag = diag;

    let mut pp = png;
    let mut ii = info;
    (api.png_destroy_read_struct)(&mut pp, &mut ii, null_mut());
    drop(cap);
    res
}

fn diff(label: &str, bytes: &[u8], cfg: Cfg, gran: Gran) -> Res {
    unsafe {
        let c = progressive(c_api(), bytes, cfg, gran);
        let r = progressive(rs_api(), bytes, cfg, gran);
        assert_eq!(
            c.ok, r.ok,
            "{label}: error parity\n C diag {:?}\n RS diag {:?}",
            c.diag, r.diag
        );
        assert_eq!(c.diag, r.diag, "{label}: diagnostics");
        assert_eq!(c.info_calls, r.info_calls, "{label}: info callback count");
        assert_eq!(c.info_before, r.info_before, "{label}: info snapshot (before start_read_image)");
        assert_eq!(c.info_after, r.info_after, "{label}: info snapshot (after start_read_image)");
        assert_eq!(c.passes, r.passes, "{label}: png_set_interlace_handling");
        assert_eq!(c.ends, r.ends, "{label}: end callback count");
        assert_eq!(c.end_snapshot, r.end_snapshot, "{label}: end snapshot");
        assert_eq!(c.pause_rets, r.pause_rets, "{label}: png_process_data_pause");
        assert_eq!(c.ptr_ok, r.ptr_ok, "{label}: png_get_progressive_ptr");
        assert!(
            c.ptr_ok.iter().all(|b| *b),
            "{label}: progressive ptr mismatch"
        );
        assert_eq!(c.rows.len(), r.rows.len(), "{label}: row callback count");
        for (i, (cr, rr)) in c.rows.iter().zip(r.rows.iter()).enumerate() {
            assert_eq!(
                (cr.0, cr.1, cr.2, cr.3),
                (rr.0, rr.1, rr.2, rr.3),
                "{label}: row {i} metadata"
            );
            match (&cr.4, &rr.4) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    assert_bytes_eq(&format!("{label}: row {i} ({} , pass {})", cr.0, cr.1), a, b)
                }
                _ => panic!("{label}: row {i} NULL-ness differs"),
            }
        }
        assert_eq!(c.display.len(), r.display.len(), "{label}: display rows");
        for (i, (a, b)) in c.display.iter().zip(r.display.iter()).enumerate() {
            assert_bytes_eq(&format!("{label}: display row {i}"), a, b);
        }
        c
    }
}

const SIZES: [(u32, u32); 5] = [(1, 1), (1, 9), (9, 1), (13, 7), (32, 17)];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The whole progressive pipeline, all legal IHDRs, both interlace types, five
/// sizes, five feed granularities, with the ancillary chunks (gAMA/tEXt/tRNS/
/// bKGD) present so the chunk handlers run.
#[test]
fn progressive_all_formats() {
    let mut rng = Rng::new(0x51a2_b3c4_d5e6_f701);
    let grans = [
        Gran::N(1),
        Gran::N(3),
        Gran::N(7),
        Gran::N(64),
        Gran::All,
    ];
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in SIZES.iter() {
                let src = Src::gen(&mut rng, ct, bd, w, h, il, true);
                let bytes = unsafe { encode(c_api(), &src) };
                let mut first: Option<Res> = None;
                for &g in grans.iter() {
                    let label = format!("ct={ct} bd={bd} il={il} {w}x{h} gran={g:?}");
                    let res = diff(&label, &bytes, Cfg::plain(), g);
                    assert!(res.ok, "{label}: progressive read failed {:?}", res.diag);
                    assert_eq!(res.info_calls, 1, "{label}: info callback once");
                    assert_eq!(res.ends, 1, "{label}: end callback once");
                    // The granularity must not change what the app sees.
                    match &first {
                        None => first = Some(res),
                        Some(f) => {
                            assert_eq!(f.rows, res.rows, "{label}: rows differ from gran=1");
                            assert_eq!(f.info_after, res.info_after, "{label}: info differs");
                        }
                    }
                }
                // Sanity: the decoded rows reproduce the source image.
                let f = first.unwrap();
                if il == PNG_INTERLACE_NONE {
                    let got: Vec<Vec<u8>> = f
                        .rows
                        .iter()
                        .filter_map(|r| r.4.clone())
                        .collect();
                    assert_eq!(got.len(), src.rows.len());
                    for (i, (a, b)) in got.iter().zip(src.rows.iter()).enumerate() {
                        assert_bytes_eq(&format!("roundtrip row {i}"), b, a);
                    }
                }
            }
        }
    }
}

/// Without ancillary chunks (bare IHDR/IDAT/IEND) and using
/// `png_read_update_info` instead of `png_start_read_image`.
#[test]
fn progressive_bare_and_update_info() {
    let mut rng = Rng::new(0x62b3_c4d5_e6f7_0812);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (13, 7), (32, 17)] {
                let src = Src::gen(&mut rng, ct, bd, w, h, il, false);
                let bytes = unsafe { encode(c_api(), &src) };
                for g in [Gran::N(1), Gran::N(7), Gran::All] {
                    let mut cfg = Cfg::plain();
                    cfg.update_info = true;
                    diff(
                        &format!("bare ct={ct} bd={bd} il={il} {w}x{h} gran={g:?}"),
                        &bytes,
                        cfg,
                        g,
                    );
                }
            }
        }
    }
}

/// No `png_set_interlace_handling`: the row callback gets the *unexpanded*
/// rows of each interlace pass.
#[test]
fn progressive_without_interlace_handling() {
    let mut rng = Rng::new(0x73c4_d5e6_f708_1923);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (1, 9), (9, 1), (13, 7), (32, 17)] {
                let src = Src::gen(&mut rng, ct, bd, w, h, il, true);
                let bytes = unsafe { encode(c_api(), &src) };
                for g in [Gran::N(1), Gran::N(64), Gran::All] {
                    let mut cfg = Cfg::plain();
                    cfg.interlace_handling = false;
                    diff(
                        &format!("noil ct={ct} bd={bd} il={il} {w}x{h} gran={g:?}"),
                        &bytes,
                        cfg,
                        g,
                    );
                }
            }
        }
    }
}

/// `png_progressive_combine_row` with a per-row display buffer.
///
/// Only exercised together with `png_set_interlace_handling`: without it
/// `png_combine_row` degenerates to a `memcpy` of a *full* row out of a buffer
/// that only holds the pass' `iwidth` pixels, i.e. it would read stale bytes
/// (C undefined behaviour, not a defined API path).
#[test]
fn progressive_combine_row() {
    let mut rng = Rng::new(0x84d5_e6f7_0819_2a34);
    for (ct, bd) in legal_ihdr() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(1u32, 1u32), (1, 9), (9, 1), (13, 7), (32, 17)] {
                let src = Src::gen(&mut rng, ct, bd, w, h, il, true);
                let bytes = unsafe { encode(c_api(), &src) };
                for g in [Gran::N(3), Gran::All] {
                    let mut cfg = Cfg::plain();
                    cfg.combine = true;
                    let res = diff(
                        &format!("combine ct={ct} bd={bd} il={il} {w}x{h} gran={g:?}"),
                        &bytes,
                        cfg,
                        g,
                    );
                    assert_eq!(res.display.len(), h as usize);
                    // After the last pass the blocky display holds the image.
                    // Only the meaningful bits are compared: png_combine_row
                    // deliberately preserves the destination's padding bits of
                    // the final partial byte of a row.
                    let bits = src.pixel_depth() as usize * w as usize;
                    let full = bits / 8;
                    let rem = bits % 8;
                    for (i, d) in res.display.iter().enumerate() {
                        assert_bytes_eq(
                            &format!("combine ct={ct} bd={bd} il={il} {w}x{h} final row {i}"),
                            &src.rows[i][..full],
                            &d[..full],
                        );
                        if rem != 0 {
                            let mask = 0xffu8 << (8 - rem);
                            assert_eq!(
                                src.rows[i][full] & mask,
                                d[full] & mask,
                                "combine ct={ct} bd={bd} il={il} {w}x{h} final row {i} tail"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `png_process_data_pause` with `save != 0` and with `save == 0`.
///
/// The pause is taken from the info and the end callback.  Those are the two
/// places libpng reaches from `png_push_read_chunk`, where setting
/// `buffer_size = 0` cleanly terminates the `png_process_data` loop.  Pausing
/// from the *row* callback is deliberately not tested: the row callback runs
/// inside `png_process_IDAT_data`, and when it returns `png_push_read_IDAT`
/// unconditionally executes `png_ptr->buffer_size -= save_size` on the
/// already-zeroed `buffer_size`, wrapping the `size_t` and (with
/// `idat_size == 0`) making `png_crc_finish` read four bytes out of empty push
/// buffers, i.e. uninitialised stack.  That is C undefined behaviour.
#[test]
fn process_data_pause() {
    let mut rng = Rng::new(0x95e6_f708_192a_3b45);
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let src = Src::gen(&mut rng, ct, bd, 13, 7, il, true);
            let bytes = unsafe { encode(c_api(), &src) };
            let reference = diff(
                &format!("pause-ref ct={ct} bd={bd} il={il}"),
                &bytes,
                Cfg::plain(),
                Gran::All,
            );
            for at in [PauseAt::Info, PauseAt::End] {
                for save in [0i32, 1] {
                    for g in [Gran::N(7), Gran::N(64), Gran::All] {
                        let mut cfg = Cfg::plain();
                        cfg.pause_at = at;
                        cfg.pause_save = save;
                        let label =
                            format!("pause ct={ct} bd={bd} il={il} at={at:?} save={save} g={g:?}");
                        let res = diff(&label, &bytes, cfg, g);
                        assert!(res.ok, "{label}: failed {:?}", res.diag);
                        assert_eq!(res.pause_rets.len(), 1, "{label}: paused once");
                        if save != 0 {
                            assert_eq!(res.pause_rets[0], 0, "{label}: save != 0 returns 0");
                        }
                        // Pausing must not change the decoded result.
                        assert_eq!(res.ends, 1, "{label}: end reached");
                        assert_eq!(res.rows, reference.rows, "{label}: rows after pause");
                        assert_eq!(
                            res.info_after, reference.info_after,
                            "{label}: info after pause"
                        );
                    }
                }
            }
        }
    }
}

/// `png_process_data_skip` is a stub that only reports an application warning.
#[test]
fn process_data_skip() {
    unsafe {
        for api in both() {
            set_current_api(api);

            // Default: application errors are fatal.
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let r = guard(|| (api.png_process_data_skip)(s.png));
            let d = diag_take();
            let strict = (r, d);

            // With benign errors enabled the same call only warns.
            diag_reset();
            let s2 = ReadSess::new(api, &[]);
            (api.png_set_benign_errors)(s2.png, 1);
            let r2 = guard(|| (api.png_process_data_skip)(s2.png));
            let d2 = diag_take();
            let benign = (r2, d2);

            if api.name == "C" {
                SKIP_C.with(|c| *c.borrow_mut() = Some((strict, benign)));
            } else {
                SKIP_C.with(|c| {
                    let cv = c.borrow_mut().take().expect("C ran first");
                    assert_eq!(cv.0 .0, strict.0, "skip strict return");
                    assert_eq!(cv.0 .1, strict.1, "skip strict diag");
                    assert_eq!(cv.1 .0, benign.0, "skip benign return");
                    assert_eq!(cv.1 .1, benign.1, "skip benign diag");
                });
                assert_eq!(benign.0, Some(0), "skip returns 0 when benign");
                assert_eq!(
                    benign.1.warnings.len(),
                    1,
                    "skip warns once: {:?}",
                    benign.1
                );
            }
        }
        // NULL png_ptr: png_app_warning dereferences png_ptr->flags, so this is
        // NOT a defined path and is not tested.
    }
}

type SkipRes = (Option<png_uint_32>, Diag);
thread_local! {
    static SKIP_C: std::cell::RefCell<Option<(SkipRes, SkipRes)>> =
        const { std::cell::RefCell::new(None) };
}

/// `png_push_fill_buffer` / `png_push_save_buffer` / `png_push_restore_buffer`
/// driven directly.
#[test]
fn push_buffer_helpers() {
    let mut rng = Rng::new(0xa6f7_0819_2a3b_4c56);
    let src = Src::gen(&mut rng, PNG_COLOR_TYPE_RGB, 8, 17, 5, PNG_INTERLACE_NONE, true);
    let bytes = unsafe { encode(c_api(), &src) };

    let mut out: Vec<(Vec<Vec<u8>>, Diag, bool)> = Vec::new();
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let v = ver();
            let png = (api.png_create_read_struct)(
                v.as_ptr(),
                null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            let info = (api.png_create_info_struct)(png);
            (api.png_set_progressive_read_fn)(png, null_mut(), None, None, None);

            let mut data = bytes.clone();
            let mut grabs: Vec<Vec<u8>> = Vec::new();
            let ok = guard(|| {
                // Restore the first 24 bytes as the "current" buffer and read
                // them back out through png_push_fill_buffer.
                (api.png_push_restore_buffer)(png, data.as_mut_ptr(), 24);
                let mut sig = vec![0u8; 8];
                (api.png_push_fill_buffer)(png, sig.as_mut_ptr(), 8);
                grabs.push(sig);
                let mut hdr = vec![0u8; 8];
                (api.png_push_fill_buffer)(png, hdr.as_mut_ptr(), 8);
                grabs.push(hdr);

                // Park the remaining 8 bytes in the save buffer, hand over the
                // next slice, then drain across the save/current boundary.
                (api.png_push_save_buffer)(png);
                (api.png_push_restore_buffer)(png, data.as_mut_ptr().add(24), 16);
                let mut across = vec![0u8; 20];
                (api.png_push_fill_buffer)(png, across.as_mut_ptr(), 20);
                grabs.push(across);

                // Over-long request: only the available bytes are copied, the
                // rest of the destination keeps its initial value.
                let mut over = vec![0xEEu8; 16];
                (api.png_push_fill_buffer)(png, over.as_mut_ptr(), 16);
                grabs.push(over);

                // NULL png_ptr is an explicit early return.
                let mut z = vec![0x11u8; 4];
                (api.png_push_fill_buffer)(null_mut(), z.as_mut_ptr(), 4);
                grabs.push(z);
            })
            .is_some();
            let d = diag_take();
            let mut pp = png;
            let mut ii = info;
            (api.png_destroy_read_struct)(&mut pp, &mut ii, null_mut());
            out.push((grabs, d, ok));
        }
    }
    assert_eq!(out[0].2, out[1].2, "push helpers: error parity");
    assert_eq!(out[0].1, out[1].1, "push helpers: diagnostics");
    assert_eq!(out[0].0.len(), out[1].0.len());
    for (i, (a, b)) in out[0].0.iter().zip(out[1].0.iter()).enumerate() {
        assert_bytes_eq(&format!("push fill_buffer grab {i}"), a, b);
    }
    // Independently: the first grab is the PNG signature, the second the IHDR
    // chunk header.
    assert_eq!(&out[0].0[0][..], &PNG_SIG[..]);
    assert_eq!(&out[0].0[1][..], &[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    assert_eq!(&out[0].0[2][..], &bytes[16..36]);
    assert_eq!(&out[0].0[4][..], &[0x11u8; 4]);
}

/// `png_process_some_data`, `png_push_read_sig`, `png_push_read_chunk` and
/// `png_push_read_IDAT` called directly, driving the state machine by hand.
#[test]
fn push_state_machine_direct() {
    let mut rng = Rng::new(0xb708_192a_3b4c_5d67);
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 2),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let src = Src::gen(&mut rng, ct, bd, 11, 6, il, true);
            let bytes = unsafe { encode(c_api(), &src) };
            let mut res: Vec<Res> = Vec::new();
            for api in both() {
                unsafe {
                    set_current_api(api);
                    diag_reset();
                    let v = ver();
                    let png = (api.png_create_read_struct)(
                        v.as_ptr(),
                        null_mut(),
                        Some(cb_error),
                        Some(cb_warning),
                    );
                    let info = (api.png_create_info_struct)(png);
                    let mut cap = Box::new(Cap::new(Cfg::plain()));
                    let capp: *mut Cap = &mut *cap;
                    cap.me = capp;
                    (api.png_set_progressive_read_fn)(
                        png,
                        capp as png_voidp,
                        Some(on_info),
                        Some(on_row),
                        Some(on_end),
                    );
                    let mut data = bytes.clone();
                    let ok = guard(|| {
                        (api.png_push_restore_buffer)(png, data.as_mut_ptr(), data.len());
                        // Signature handler, directly.
                        (api.png_push_read_sig)(png, info);
                        // IHDR (and whatever else) via the chunk handler,
                        // directly, until the info callback has fired (which
                        // means the IDAT header was seen).
                        let mut n = 0;
                        while (*capp).res.info_calls == 0 && n < 64 {
                            (api.png_push_read_chunk)(png, info);
                            n += 1;
                        }
                        // Now in IDAT mode: pump the IDAT handler directly.
                        let mut n = 0;
                        while (*capp).res.rows.is_empty() && n < 4096 {
                            (api.png_push_read_IDAT)(png);
                            n += 1;
                        }
                        // ... and let the generic dispatcher finish the job.
                        let mut n = 0;
                        while (*capp).res.ends == 0 && n < 200_000 {
                            (api.png_process_some_data)(png, info);
                            n += 1;
                        }
                    })
                    .is_some();
                    let diag = diag_take();
                    let mut r = std::mem::take(&mut cap.res);
                    r.ok = ok;
                    r.diag = diag;
                    let mut pp = png;
                    let mut ii = info;
                    (api.png_destroy_read_struct)(&mut pp, &mut ii, null_mut());
                    drop(cap);
                    res.push(r);
                }
            }
            let label = format!("direct ct={ct} bd={bd} il={il}");
            assert_eq!(res[0].ok, res[1].ok, "{label}: error parity");
            assert_eq!(res[0].diag, res[1].diag, "{label}: diagnostics");
            assert_eq!(res[0].info_before, res[1].info_before, "{label}: info before");
            assert_eq!(res[0].info_after, res[1].info_after, "{label}: info after");
            assert_eq!(res[0].rows, res[1].rows, "{label}: rows");
            assert_eq!(res[0].ends, res[1].ends, "{label}: end");
            assert_eq!(res[0].end_snapshot, res[1].end_snapshot, "{label}: end snapshot");
            assert_eq!(res[0].ends, 1, "{label}: reached IEND");
            // Same rows as the ordinary png_process_data driver.
            let ordinary = diff(&format!("{label} (ordinary)"), &bytes, Cfg::plain(), Gran::All);
            assert_eq!(ordinary.rows, res[0].rows, "{label}: direct vs png_process_data");
        }
    }
}

/// `png_process_data` and friends with NULL arguments (all documented early
/// returns).
#[test]
fn null_arguments() {
    unsafe {
        for api in both() {
            set_current_api(api);
            diag_reset();
            let s = ReadSess::new(api, &[]);
            let r = guard(|| {
                let mut b = [0u8; 4];
                (api.png_process_data)(null_mut(), null_mut(), b.as_mut_ptr(), 4);
                (api.png_process_data)(s.png, null_mut(), b.as_mut_ptr(), 4);
                (api.png_process_some_data)(null_mut(), null_mut());
                assert_eq!((api.png_process_data_pause)(null_mut(), 0), 0);
                assert_eq!((api.png_process_data_pause)(null_mut(), 1), 0);
                assert!((api.png_get_progressive_ptr)(null()).is_null());
                (api.png_set_progressive_read_fn)(null_mut(), null_mut(), None, None, None);
                (api.png_progressive_combine_row)(null(), b.as_mut_ptr(), b.as_ptr());
            });
            let d = diag_take();
            assert!(r.is_some(), "{}: null args errored: {:?}", api.name, d);
            assert_eq!(d, Diag::default(), "{}: null args diag", api.name);
        }
    }
}
