//! Phase D — the progressive ("push") reader, `c_src/src/pngpread.c`.
//!
//! Covers CONFIGS.md rows
//!   * C-134 → `chunk_sizes`      (`png_set_progressive_read_fn`, `png_process_data`,
//!                                 `png_progressive_combine_row`, `png_get_progressive_ptr`)
//!   * C-135 → `pause_skip`       (`png_process_data_pause`, `png_process_data_skip`)
//!   * C-136 → `split_boundaries` (`png_push_read_chunk` / `png_push_read_IDAT` /
//!                                 `png_push_save_buffer` / `png_push_restore_buffer` /
//!                                 `png_push_fill_buffer` buffer boundaries, plus every
//!                                 error path of `pngpread.c` that is reachable from
//!                                 the outside: ERRORS.md D-67 … D-84)
//!   * C-137 → `transforms`       (read transforms installed from the info callback)
//!
//! Every case builds the input file once with the **C** library (so both
//! libraries see byte-identical input), then feeds that file to *both* libraries
//! through `png_process_data` and compares
//!
//!   * every `info_fn` / `row_fn` / `end_fn` invocation, in order,
//!   * everything `png_get_*` reports before and after `png_read_update_info`,
//!   * every byte of every destination row produced by
//!     `png_progressive_combine_row`,
//!   * every `png_process_data_pause` / `png_process_data_skip` return value,
//!   * every warning and the fatal-error message, if any.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_int, c_void};
use std::cell::Cell;

/* ------------------------------------------------------------------ */
/* bookkeeping                                                         */
/* ------------------------------------------------------------------ */

thread_local! {
    /// Number of differential comparisons this `#[test]` has performed.
    static NCMP: Cell<usize> = const { Cell::new(0) };
    /// Largest number of bytes a `png_process_data_pause` ever asked to be
    /// re-supplied; used to prove the pause tests really did pause.
    static MAX_REWIND: Cell<usize> = const { Cell::new(0) };
    /// Largest number of row callbacks any run has produced; used to prove the
    /// scenarios really decode rows rather than agreeing on nothing.
    static MAX_ROWS: Cell<usize> = const { Cell::new(0) };
}

/// `PROG_TRACE=1` names every case before it runs, which is what you want when
/// a case does not merely fail but takes the whole test binary down with it.
fn trace_case(case: &str) {
    if std::env::var_os("PROG_TRACE").is_some() {
        eprintln!("CASE {}", case);
    }
}

fn same<F>(case: &str, f: F)
where
    F: FnMut(&Api) -> Outcome,
{
    NCMP.with(|c| c.set(c.get() + 1));
    trace_case(case);
    assert_same(case, f);
}

fn same_forked<F>(case: &str, f: F)
where
    F: Fn(&Api) -> String,
{
    NCMP.with(|c| c.set(c.get() + 1));
    trace_case(case);
    assert_same_forked(case, f);
}

fn report(name: &str) {
    let n = NCMP.with(|c| c.get());
    let r = MAX_ROWS.with(|c| c.get());
    eprintln!(
        "progressive::{}: {} differential comparisons (deepest run delivered {} rows)",
        name, n, r
    );
    assert!(n > 0);
    assert!(r > 0, "no run ever reached the row callback");
}

/// A stable address handed to `png_set_progressive_read_fn` so that
/// `png_get_progressive_ptr` / `png_get_io_ptr` can be checked.
static COOKIE: u8 = 0x77;

fn cookie() -> *mut c_void {
    &COOKIE as *const u8 as *mut c_void
}

/// Install a fresh `Tls` and make the C `Api` current for the duration of `f`;
/// used to build the reference input files outside of `assert_same`.
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

/// Write `img` with the C library and return the file.
fn build(img: &Img, opts: &WriteOpts) -> Vec<u8> {
    let v = with_c_tls(|api| unsafe { write_plain(api, img, opts).bytes });
    assert!(!v.is_empty(), "reference write produced nothing");
    v
}

fn build_plain(img: &Img) -> Vec<u8> {
    build(img, &WriteOpts::default())
}

/* ------------------------------------------------------------------ */
/* the read transforms this file installs from the info callback        */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tf {
    Expand,
    Strip16,
    GrayToRgb,
    PaletteToRgb,
    Packing,
    Bgr,
    Swap,
    Filler(u32, c_int),
    InvertMono,
    /// `(screen gamma, file gamma)` in libpng fixed point
    Gamma(i32, i32),
    Expand16,
    StripAlpha,
    Packswap,
    InvertAlpha,
    SwapAlpha,
    AddAlpha(u32, c_int),
    Scale16,
    ExpandGray,
    TrnsToAlpha,
}

/// The ten transforms C-137 names explicitly.
const CORE_TF: [Tf; 10] = [
    Tf::Expand,
    Tf::Strip16,
    Tf::GrayToRgb,
    Tf::PaletteToRgb,
    Tf::Packing,
    Tf::Bgr,
    Tf::Swap,
    Tf::Filler(0xa5, PNG_FILLER_AFTER),
    Tf::InvertMono,
    Tf::Gamma(220000, 45455),
];

/// The pool the random combinations are drawn from.
const POOL_TF: [Tf; 18] = [
    Tf::Expand,
    Tf::Strip16,
    Tf::GrayToRgb,
    Tf::PaletteToRgb,
    Tf::Packing,
    Tf::Bgr,
    Tf::Swap,
    Tf::Filler(0x3c, PNG_FILLER_BEFORE),
    Tf::InvertMono,
    Tf::Gamma(100000, 50000),
    Tf::Expand16,
    Tf::StripAlpha,
    Tf::Packswap,
    Tf::InvertAlpha,
    Tf::SwapAlpha,
    Tf::AddAlpha(0x1f, PNG_FILLER_AFTER),
    Tf::Scale16,
    Tf::ExpandGray,
];

unsafe fn apply(api: &Api, png: *mut PngStruct, t: Tf) {
    match t {
        Tf::Expand => (api.png_set_expand)(png),
        Tf::Strip16 => (api.png_set_strip_16)(png),
        Tf::GrayToRgb => (api.png_set_gray_to_rgb)(png),
        Tf::PaletteToRgb => (api.png_set_palette_to_rgb)(png),
        Tf::Packing => (api.png_set_packing)(png),
        Tf::Bgr => (api.png_set_bgr)(png),
        Tf::Swap => (api.png_set_swap)(png),
        Tf::Filler(v, loc) => (api.png_set_filler)(png, v, loc),
        Tf::InvertMono => (api.png_set_invert_mono)(png),
        Tf::Gamma(s, f) => {
            (api.png_set_gamma)(png, s as f64 / 100000.0, f as f64 / 100000.0)
        }
        Tf::Expand16 => (api.png_set_expand_16)(png),
        Tf::StripAlpha => (api.png_set_strip_alpha)(png),
        Tf::Packswap => (api.png_set_packswap)(png),
        Tf::InvertAlpha => (api.png_set_invert_alpha)(png),
        Tf::SwapAlpha => (api.png_set_swap_alpha)(png),
        Tf::AddAlpha(v, loc) => (api.png_set_add_alpha)(png, v, loc),
        Tf::Scale16 => (api.png_set_scale_16)(png),
        Tf::ExpandGray => (api.png_set_expand_gray_1_2_4_to_8)(png),
        Tf::TrnsToAlpha => (api.png_set_tRNS_to_alpha)(png),
    }
    log(format!("apply {:?}", t));
}

/* ------------------------------------------------------------------ */
/* per-run configuration + the progressive callbacks                   */
/* ------------------------------------------------------------------ */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowAct {
    /// `png_progressive_combine_row` into `tls().rows[row_number]`.
    Combine,
    /// Log the raw interlaced pass row the callback was handed.
    Raw,
    /// Only log that the callback happened (used where combining would be
    /// undefined, e.g. a deliberately corrupted `transformed_pixel_depth`).
    Nothing,
}

#[derive(Clone)]
struct Cfg {
    /* --- what the info callback does --- */
    tr: Vec<Tf>,
    /// call `png_set_interlace_handling` (so the app gets `height` rows/pass)
    ih: bool,
    /// `png_read_update_info` (false → `png_start_read_image`)
    update_info: bool,
    /// call neither → `row_buf` stays NULL, which libpng has to survive
    no_row_init: bool,
    benign: Option<c_int>,
    act: RowAct,

    /* --- pausing / skipping --- */
    pause_info: Option<c_int>,
    pause_end: Option<c_int>,
    pause_feed: Option<c_int>,
    skip_feed: bool,

    /* --- struct-wide options installed before the first feed --- */
    keep_unknown: Option<c_int>,
    crc_action: Option<(c_int, c_int)>,

    /* --- how the stream is fed --- */
    sig_bytes: Option<c_int>,
    /// `png_process_data(..., NULL, 0)` calls after the piece list is exhausted
    trailing_empty: usize,
    /// pass a NULL buffer for the trailing empty feeds
    trailing_null: bool,
    log_feeds: bool,
    log_rows: bool,

    /* --- runtime state, filled in by the callbacks --- */
    rowbytes: usize,
    width: u32,
    height: u32,
    pd: usize,
    interlaced: bool,
    info_calls: usize,
    row_calls: usize,
    end_calls: usize,
    pause_ret: usize,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            tr: Vec::new(),
            ih: true,
            update_info: true,
            no_row_init: false,
            benign: None,
            act: RowAct::Combine,
            keep_unknown: None,
            crc_action: None,
            pause_info: None,
            pause_end: None,
            pause_feed: None,
            skip_feed: false,
            sig_bytes: None,
            trailing_empty: 0,
            trailing_null: false,
            log_feeds: false,
            log_rows: true,
            rowbytes: 0,
            width: 0,
            height: 0,
            pd: 0,
            interlaced: false,
            info_calls: 0,
            row_calls: 0,
            end_calls: 0,
            pause_ret: 0,
        }
    }
}

thread_local! {
    static CFG: std::cell::UnsafeCell<Cfg> = std::cell::UnsafeCell::new(Cfg::default());
}

/// The active configuration.  A raw pointer, not a `RefCell`: libpng may
/// `longjmp` straight out of a callback, which would leave a `RefCell`
/// permanently borrowed.
fn cfg() -> &'static mut Cfg {
    let p = CFG.with(|c| c.get());
    unsafe { &mut *p }
}

/// Everything `log_info` records, but without `png_get_IHDR` -- which calls
/// `png_check_IHDR` and is therefore *fatal* both on an info struct that never
/// saw an IHDR chunk and on some perfectly legitimate post-transform states
/// (e.g. a 16-bit palette expansion).  Using the individual accessors keeps the
/// scenario running so that the rows can still be compared.
unsafe fn log_shape(api: &Api, png: *mut PngStruct, info: *mut PngInfo, tag: &str) {
    log(format!(
        "{}: {}x{} depth={} color={} il={} comp={} filter={}",
        tag,
        (api.png_get_image_width)(png, info),
        (api.png_get_image_height)(png, info),
        (api.png_get_bit_depth)(png, info),
        (api.png_get_color_type)(png, info),
        (api.png_get_interlace_type)(png, info),
        (api.png_get_compression_type)(png, info),
        (api.png_get_filter_type)(png, info),
    ));
    log(format!(
        "{}: rowbytes={} channels={} valid=0x{:x} palette_max={}",
        tag,
        (api.png_get_rowbytes)(png, info),
        (api.png_get_channels)(png, info),
        (api.png_get_valid)(png, info, 0xffffffff),
        (api.png_get_palette_max)(png, info)
    ));
}

const PASS_START: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];
const PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];

/// Width of the interlaced row of `pass` (`png_ptr->iwidth`).
fn iwidth(width: u32, interlaced: bool, pass: c_int) -> u32 {
    if !interlaced {
        return width;
    }
    let p = pass.clamp(0, 6) as usize;
    (width + PASS_INC[p] - 1 - PASS_START[p]) / PASS_INC[p]
}

unsafe extern "C" fn info_cb(png: *mut PngStruct, info: *mut PngInfo) {
    let api = cur_api();
    let c = cfg();
    c.info_calls += 1;
    log(format!("info_cb #{}", c.info_calls));
    log_shape(api, png, info, "info_cb");
    if let Some(b) = c.benign {
        (api.png_set_benign_errors)(png, b);
    }
    let trs = c.tr.clone();
    for t in trs {
        apply(api, png, t);
    }
    c.width = (api.png_get_image_width)(png, info);
    c.height = (api.png_get_image_height)(png, info);
    c.interlaced = (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7;
    if c.interlaced && c.ih {
        log(format!(
            "interlace_handling={}",
            (api.png_set_interlace_handling)(png)
        ));
    }
    if !c.no_row_init {
        if c.update_info {
            (api.png_read_update_info)(png, info);
        } else {
            (api.png_start_read_image)(png);
        }
    }
    log_shape(api, png, info, "after row init");
    c.rowbytes = (api.png_get_rowbytes)(png, info);
    c.pd = (api.png_get_channels)(png, info) as usize * (api.png_get_bit_depth)(png, info) as usize;
    // The destination rows png_progressive_combine_row writes into.  They must
    // be *zeroed*: png_combine_row only fills in the pixels of the current pass
    // and preserves the rest, so an uninitialised buffer would make the
    // comparison depend on uninitialised heap.
    tls().rows = vec![vec![0u8; c.rowbytes.max(1)]; (c.height.max(1)) as usize];
    log(format!(
        "info_cb: dest {} rows of {} bytes, pixel_depth={}",
        tls().rows.len(),
        c.rowbytes,
        c.pd
    ));
    if let Some(s) = c.pause_info {
        let r = (api.png_process_data_pause)(png, s);
        log(format!("pause(info, save={}) -> {}", s, r));
        c.pause_ret += r;
    }
}

unsafe extern "C" fn row_cb(png: *mut PngStruct, new_row: *mut u8, row_num: u32, pass: c_int) {
    let api = cur_api();
    let c = cfg();
    c.row_calls += 1;
    let n = c.row_calls;
    let have = !new_row.is_null();
    match c.act {
        RowAct::Nothing => {
            log(format!(
                "row_cb #{} row={} pass={} new_row={}",
                n, row_num, pass, have
            ));
        }
        RowAct::Raw => {
            if have {
                let iw = iwidth(c.width, c.interlaced, pass);
                let len = png_rowbytes(c.pd, iw as usize);
                let s = core::slice::from_raw_parts(new_row, len);
                log(format!(
                    "row_cb #{} row={} pass={} iw={} raw={:02x?}",
                    n, row_num, pass, iw, s
                ));
            } else {
                log(format!(
                    "row_cb #{} row={} pass={} new_row=NULL",
                    n, row_num, pass
                ));
            }
            // exercise the NULL / non-NULL branch of the public entry point too
            (api.png_progressive_combine_row)(png, core::ptr::null_mut(), core::ptr::null());
        }
        RowAct::Combine => {
            let idx = row_num as usize;
            let t = tls();
            if idx < t.rows.len() {
                let p = t.rows[idx].as_mut_ptr();
                (api.png_progressive_combine_row)(png, p, new_row);
                if c.log_rows {
                    log(format!(
                        "row_cb #{} row={} pass={} new_row={} -> {:02x?}",
                        n, row_num, pass, have, t.rows[idx]
                    ));
                } else {
                    log(format!(
                        "row_cb #{} row={} pass={} new_row={}",
                        n, row_num, pass, have
                    ));
                }
            } else {
                log(format!(
                    "row_cb #{} row={} pass={} new_row={} (row out of range, {} kept)",
                    n,
                    row_num,
                    pass,
                    have,
                    t.rows.len()
                ));
            }
        }
    }
}

unsafe extern "C" fn end_cb(png: *mut PngStruct, info: *mut PngInfo) {
    let api = cur_api();
    let c = cfg();
    c.end_calls += 1;
    log(format!("end_cb #{}", c.end_calls));
    log_shape(api, png, info, "end_cb");
    if let Some(s) = c.pause_end {
        let r = (api.png_process_data_pause)(png, s);
        log(format!("pause(end, save={}) -> {}", s, r));
        c.pause_ret += r;
    }
}

/* ------------------------------------------------------------------ */
/* the driver                                                          */
/* ------------------------------------------------------------------ */

/// Feed `data` to the progressive reader in pieces of the given lengths.
///
/// A piece length is clamped to whatever is left; a length of `0` produces a
/// genuine zero-length `png_process_data` call.  When a callback paused the
/// reader with `save == 0`, the returned byte count is *rewound*, i.e. those
/// bytes are supplied again by the following pieces — exactly what png.h
/// requires of the application.
unsafe fn run(api: &Api, data: &[u8], pieces: &[usize], c0: Cfg) -> Outcome {
    let mut o = Outcome::default();
    *cfg() = c0.clone();
    let mut buf = data.to_vec();
    let (png, info) = new_read(api);
    (api.png_set_progressive_read_fn)(
        png,
        cookie(),
        Some(info_cb),
        Some(row_cb),
        Some(end_cb),
    );
    o.push(format!(
        "progressive_ptr_ok={} io_ptr_ok={}",
        (api.png_get_progressive_ptr)(png) == cookie(),
        (api.png_get_io_ptr)(png) == cookie()
    ));
    let mut pos = 0usize;
    let mut feeds = 0usize;
    let mut fed = 0usize;
    let mut rewound = 0usize;
    let g = guarded(api, png, &mut || {
        // NB: inside the trap -- png_set_sig_bytes(> 8) is fatal.
        if let Some(k) = c0.sig_bytes {
            (api.png_set_sig_bytes)(png, k);
        }
        if let Some(keep) = c0.keep_unknown {
            (api.png_set_keep_unknown_chunks)(png, keep, core::ptr::null(), 0);
        }
        if let Some((crit, anc)) = c0.crc_action {
            (api.png_set_crc_action)(png, crit, anc);
        }
        for &want in pieces {
            if pos >= buf.len() && want != 0 {
                break;
            }
            let n = want.min(buf.len() - pos);
            let p = buf.as_mut_ptr().add(pos);
            cfg().pause_ret = 0;
            if c0.log_feeds {
                log(format!("feed #{} at {} len {}", feeds + 1, pos, n));
            }
            (api.png_process_data)(png, info, p, n);
            feeds += 1;
            fed += n;
            pos += n;
            if let Some(s) = c0.pause_feed {
                let r = (api.png_process_data_pause)(png, s);
                log(format!("pause(after feed #{}, save={}) -> {}", feeds, s, r));
                cfg().pause_ret += r;
            }
            if c0.skip_feed {
                let r = (api.png_process_data_skip)(png);
                log(format!("skip(after feed #{}) -> {}", feeds, r));
            }
            let back = cfg().pause_ret.min(pos);
            if back != 0 {
                pos -= back;
                rewound += back;
                log(format!("rewind {} to {}", back, pos));
            }
        }
        for i in 0..c0.trailing_empty {
            let p = if c0.trailing_null {
                core::ptr::null_mut()
            } else {
                buf.as_mut_ptr()
            };
            log(format!("trailing empty feed #{} null={}", i + 1, c0.trailing_null));
            (api.png_process_data)(png, info, p, 0);
            feeds += 1;
        }
    });
    MAX_REWIND.with(|c| c.set(c.get().max(rewound)));
    MAX_ROWS.with(|c| c.set(c.get().max(cfg().row_calls)));
    let c = cfg();
    o.push(format!("guard={:?}", g));
    o.push(format!(
        "feeds={} fed={} rewound={} pos={} of {} info={} row={} end={}",
        feeds,
        fed,
        rewound,
        pos,
        buf.len(),
        c.info_calls,
        c.row_calls,
        c.end_calls
    ));
    if g == Guard::Ok {
        // NB: this has to run under its own error trap.  png_get_IHDR calls
        // png_check_IHDR, which is fatal on a stream that never got as far as
        // the IHDR chunk -- and a png_error raised outside `guarded` would
        // longjmp onto a dead stack frame and take the test binary with it.
        let g2 = guarded(api, png, &mut || {
            log_shape(api, png, info, "final");
            log(format!(
                "final pause(0)={} pause(1)={}",
                (api.png_process_data_pause)(png, 0),
                (api.png_process_data_pause)(png, 1)
            ));
        });
        o.push(format!("post guard={:?}", g2));
    }
    for (y, r) in tls().rows.iter().enumerate() {
        o.push(format!("row {} = {:02x?}", y, r));
        o.output.extend_from_slice(r);
    }
    destroy_read(api, png, info);
    o
}

fn fixed_pieces(n: usize, k: usize) -> Vec<usize> {
    let k = k.max(1);
    vec![k; n.div_ceil(k).max(1)]
}

/// `fixed_pieces` with room for `extra` more feeds (needed when a `pause` with
/// `save == 0` makes the application re-supply bytes).
fn fixed_pieces_slack(n: usize, k: usize, extra: usize) -> Vec<usize> {
    let k = k.max(1);
    vec![k; n.div_ceil(k).max(1) + extra]
}

/* ------------------------------------------------------------------ */
/* datastream surgery                                                  */
/* ------------------------------------------------------------------ */

fn parts(file: &[u8]) -> Vec<(String, Vec<u8>)> {
    split_chunks(file)
        .into_iter()
        .map(|(n, r)| (n, file[r.start + 8..r.end - 4].to_vec()))
        .collect()
}

fn assemble(ps: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut v = SIG.to_vec();
    for (n, d) in ps {
        let b = n.as_bytes();
        assert_eq!(b.len(), 4);
        let nm = [b[0], b[1], b[2], b[3]];
        v.extend_from_slice(&chunk(&nm, d));
    }
    v
}

/// Byte range of the *data* of the first chunk called `name`.
fn chunk_data_range(file: &[u8], name: &str) -> std::ops::Range<usize> {
    let (_, r) = split_chunks(file)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no {} chunk", name));
    (r.start + 8)..(r.end - 4)
}

/* ================================================================== */
/* C-134 — feed the file in fixed-size pieces                          */
/* ================================================================== */

/// C-134: every one of the 15 legal (colour type, bit depth) pairs x interlace
/// NONE/ADAM7, fed in pieces of 1, 2, 3, 5, 13, 100, 1024, 8192 bytes and all at
/// once, with several randomised sizes and pixel contents per shape.
#[test]
fn chunk_sizes() {
    const SIZES: [(u32, u32); 6] = [(1, 1), (2, 3), (5, 5), (9, 5), (17, 3), (33, 4)];
    const FEEDS: [usize; 8] = [1, 2, 3, 5, 13, 100, 1024, 8192];

    for (si, &(ct, bd)) in VALID_SHAPES.iter().enumerate() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for k in 0..3usize {
                let (w, h) = SIZES[(si * 3 + k) % SIZES.len()];
                let mut rng = Rng::new(
                    0x9a5_0000
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((il as u64) << 24)
                        ^ ((k as u64) << 8)
                        ^ si as u64,
                );
                let mut img = Img::random(&mut rng, w, h, ct, bd);
                img.interlace = il;
                let opts = WriteOpts {
                    filter_mask: Some(rng.pick(&[
                        PNG_NO_FILTERS,
                        PNG_FILTER_NONE,
                        PNG_FILTER_SUB,
                        PNG_FILTER_UP,
                        PNG_FILTER_AVG,
                        PNG_FILTER_PAETH,
                        PNG_ALL_FILTERS,
                    ])),
                    level: Some(rng.range(0, 10) as c_int),
                    ..Default::default()
                };
                let file = build(&img, &opts);
                let tag = format!("ct={} bd={} il={} {}x{} ({} bytes)", ct, bd, il, w, h, file.len());

                // all at once, then every fixed granularity
                let mut plans: Vec<(String, Vec<usize>)> =
                    vec![("whole".to_string(), vec![file.len()])];
                for &f in &FEEDS {
                    plans.push((format!("feed{}", f), fixed_pieces(file.len(), f)));
                }
                // The completely combined image must be the image that was
                // written -- but only when the rows have no spare bits: for a
                // partial last byte png_combine_row deliberately *keeps* the
                // destination's bits, which here are the zeros we start from.
                let exact = (img.bit_depth as usize * channels_of(img.color_type) * w as usize) % 8 == 0;
                let expect: Vec<u8> = img.rows.concat();

                for (name, pieces) in &plans {
                    let mut decoded: Vec<u8> = Vec::new();
                    same(&format!("C-134 {} {}", tag, name), |api| unsafe {
                        let o = run(
                            api,
                            &file,
                            pieces,
                            Cfg {
                                ih: true,
                                ..Default::default()
                            },
                        );
                        if api.which == "C" {
                            decoded = o.output.clone();
                        }
                        o
                    });
                    if exact {
                        assert_eq!(
                            decoded, expect,
                            "C-134 {} {}: the progressively combined image is not the \
                             image that was written -- the test itself is wrong",
                            tag, name
                        );
                    } else {
                        assert_eq!(decoded.len(), expect.len(), "C-134 {} {}", tag, name);
                    }
                }

                // The other legal way to drive an interlaced image: do *not*
                // call png_set_interlace_handling, so the application receives
                // the un-expanded rows of each pass (num_rows is recomputed per
                // pass in png_read_push_finish_row).
                if il == PNG_INTERLACE_ADAM7 {
                    for &f in &[1usize, 7, 64, file.len()] {
                        same(
                            &format!("C-134 {} raw-pass feed{}", tag, f),
                            |api| unsafe {
                                run(
                                    api,
                                    &file,
                                    &fixed_pieces(file.len(), f),
                                    Cfg {
                                        ih: false,
                                        act: RowAct::Raw,
                                        ..Default::default()
                                    },
                                )
                            },
                        );
                    }

                    // ... and combining anyway, which for an interlaced image
                    // without PNG_INTERLACE makes png_combine_row memcpy a full
                    // row out of the (calloc'd) interlaced row buffer.  Ugly,
                    // but completely determined, so it is fair game.
                    for &f in &[3usize, file.len()] {
                        same(
                            &format!("C-134 {} no-ih combine feed{}", tag, f),
                            |api| unsafe {
                                run(
                                    api,
                                    &file,
                                    &fixed_pieces(file.len(), f),
                                    Cfg {
                                        ih: false,
                                        act: RowAct::Combine,
                                        ..Default::default()
                                    },
                                )
                            },
                        );
                    }
                }

                // png_start_read_image instead of png_read_update_info
                same(&format!("C-134 {} start_read_image", tag), |api| unsafe {
                    run(
                        api,
                        &file,
                        &fixed_pieces(file.len(), 11),
                        Cfg {
                            ih: true,
                            update_info: false,
                            ..Default::default()
                        },
                    )
                });
            }
        }
    }
    report("chunk_sizes");
}

/* ================================================================== */
/* C-135 — png_process_data_pause / png_process_data_skip               */
/* ================================================================== */

/// C-135: `png_process_data_pause` with `save` 0 and 1, called from the info
/// callback, from the end callback and after every feed (resuming until the
/// whole file has been consumed), and `png_process_data_skip`.
#[test]
fn pause_skip() {
    // three reference files: plain, interlaced, and one with several IDATs
    let mut rng = Rng::new(0x9a_05e);
    let f_plain = build_plain(&Img::random(&mut rng, 9, 6, PNG_COLOR_TYPE_RGB, 8));
    let mut il = Img::random(&mut rng, 7, 7, PNG_COLOR_TYPE_PALETTE, 4);
    il.interlace = PNG_INTERLACE_ADAM7;
    let f_il = build_plain(&il);
    let f_multi = build(
        &Img::random(&mut rng, 40, 8, PNG_COLOR_TYPE_RGB_ALPHA, 8),
        &WriteOpts {
            buffer_size: Some(64),
            level: Some(0),
            ..Default::default()
        },
    );
    let n_idat = split_chunks(&f_multi).iter().filter(|(n, _)| n == "IDAT").count();
    assert!(
        n_idat > 3,
        "the multi-IDAT reference file only has {} IDAT chunks, so \
         png_push_read_IDAT's chunk-header path is barely exercised",
        n_idat
    );
    let files: [(&str, &Vec<u8>); 3] = [
        ("plain", &f_plain),
        ("interlaced", &f_il),
        ("multi-idat", &f_multi),
    ];

    /* --- pause from the info callback, which really has pending data --- */
    for (fname, file) in files {
        for save in [0, 1] {
            for &k in &[1usize, 3, 13, 100, file.len()] {
                same(
                    &format!("C-135 pause(info,{}) {} feed{}", save, fname, k),
                    |api| unsafe {
                        run(
                            api,
                            file,
                            &fixed_pieces_slack(file.len(), k, 4),
                            Cfg {
                                pause_info: Some(save),
                                log_feeds: true,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    assert!(
        MAX_REWIND.with(|c| c.get()) > 0,
        "png_process_data_pause(save=0) never reported any unprocessed bytes -- \
         the pause test is not actually pausing anything"
    );

    /* --- pause from the end callback --- */
    for (fname, file) in files {
        for save in [0, 1] {
            for &k in &[5usize, 37, file.len()] {
                same(
                    &format!("C-135 pause(end,{}) {} feed{}", save, fname, k),
                    |api| unsafe {
                        run(
                            api,
                            file,
                            &fixed_pieces_slack(file.len(), k, 4),
                            Cfg {
                                pause_end: Some(save),
                                trailing_empty: 2,
                                log_feeds: true,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* --- pause from both callbacks at once --- */
    for (fname, file) in files {
        for save in [0, 1] {
            same(
                &format!("C-135 pause(info+end,{}) {}", save, fname),
                |api| unsafe {
                    run(
                        api,
                        file,
                        &fixed_pieces_slack(file.len(), 17, 6),
                        Cfg {
                            pause_info: Some(save),
                            pause_end: Some(save),
                            log_feeds: true,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    /* --- pause after every feed (png.h: "only within png_process_data", so
           after the call it must be a no-op that reports 0) --- */
    for (fname, file) in files {
        for save in [0, 1] {
            for &k in &[1usize, 9, 250, file.len()] {
                same(
                    &format!("C-135 pause(after-feed,{}) {} feed{}", save, fname, k),
                    |api| unsafe {
                        run(
                            api,
                            file,
                            &fixed_pieces_slack(file.len(), k, 4),
                            Cfg {
                                pause_feed: Some(save),
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* --- png_process_data_pause on a struct that has not been fed at all,
           and on a NULL png_ptr (R-91) --- */
    same("C-135 pause without any data", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        (api.png_set_progressive_read_fn)(png, cookie(), Some(info_cb), Some(row_cb), Some(end_cb));
        for save in [0, 1, 0, 1, 7, -1] {
            o.push(format!(
                "pause(save={}) -> {}",
                save,
                (api.png_process_data_pause)(png, save)
            ));
        }
        o.push(format!(
            "pause(NULL,0) -> {} pause(NULL,1) -> {}",
            (api.png_process_data_pause)(core::ptr::null_mut(), 0),
            (api.png_process_data_pause)(core::ptr::null_mut(), 1)
        ));
        o.push(format!(
            "progressive_ptr(NULL) is null = {}",
            (api.png_get_progressive_ptr)(core::ptr::null_mut()).is_null()
        ));
        // png_progressive_combine_row on a NULL png_ptr, and with a NULL
        // new_row (the "this was an empty interlace row" flag), must be inert.
        let mut dst = [0xa5u8; 8];
        (api.png_progressive_combine_row)(
            core::ptr::null_mut(),
            dst.as_mut_ptr(),
            dst.as_ptr(),
        );
        (api.png_progressive_combine_row)(png, dst.as_mut_ptr(), core::ptr::null());
        (api.png_progressive_combine_row)(
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null(),
        );
        o.push(format!("combine_row inert, dst={:02x?}", dst));
        destroy_read(api, png, info);
        o
    });

    /* --- png_process_data with a NULL png_ptr / NULL info_ptr --- */
    same("C-135 process_data with NULL arguments", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        (api.png_set_progressive_read_fn)(png, cookie(), Some(info_cb), Some(row_cb), Some(end_cb));
        *cfg() = Cfg::default();
        let mut d = f_plain.clone();
        (api.png_process_data)(core::ptr::null_mut(), info, d.as_mut_ptr(), d.len());
        o.push("process_data(NULL, info) returned".to_string());
        (api.png_process_data)(png, core::ptr::null_mut(), d.as_mut_ptr(), d.len());
        o.push("process_data(png, NULL) returned".to_string());
        o.push(format!(
            "info={} row={} end={}",
            cfg().info_calls,
            cfg().row_calls,
            cfg().end_calls
        ));
        // and now for real, so the struct is in a known state
        let g = guarded(api, png, &mut || {
            (api.png_process_data)(png, info, d.as_mut_ptr(), d.len());
        });
        o.push(format!("guard={:?} rows={}", g, cfg().row_calls));
        destroy_read(api, png, info);
        o
    });

    /* --- png_process_data_skip: an unimplemented API that reports itself
           through png_app_warning, i.e. it is *fatal* unless the application
           allowed benign errors (ERRORS.md D-67, R-92) --- */
    for benign in [None, Some(0), Some(1)] {
        same(
            &format!("C-135 skip benign={:?} standalone", benign),
            |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_read(api);
                (api.png_set_progressive_read_fn)(
                    png,
                    cookie(),
                    Some(info_cb),
                    Some(row_cb),
                    Some(end_cb),
                );
                if let Some(b) = benign {
                    (api.png_set_benign_errors)(png, b);
                }
                let mut ret = 0u32;
                let g = guarded(api, png, &mut || {
                    ret = (api.png_process_data_skip)(png);
                    log(format!("skip #1 -> {}", ret));
                    ret = (api.png_process_data_skip)(png);
                    log(format!("skip #2 -> {}", ret));
                });
                o.push(format!("guard={:?} ret={}", g, ret));
                destroy_read(api, png, info);
                o
            },
        );
    }
    for benign in [Some(0), Some(1)] {
        for &k in &[1usize, 40, f_plain.len()] {
            same(
                &format!("C-135 skip benign={:?} feed{}", benign, k),
                |api| unsafe {
                    run(
                        api,
                        &f_plain,
                        &fixed_pieces(f_plain.len(), k),
                        Cfg {
                            benign,
                            skip_feed: true,
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    // png_process_data_skip(NULL) dereferences NULL inside png_app_warning;
    // that is still an observation both libraries have to agree on.
    same_forked("C-135 skip(NULL)", |api| unsafe {
        let r = (api.png_process_data_skip)(core::ptr::null_mut());
        format!("skip(NULL) -> {}", r)
    });

    assert!(
        observed()
            .iter()
            .any(|m| m.contains("png_process_data_skip is not implemented")),
        "png_process_data_skip never reported itself"
    );

    report("pause_skip");
}

/* ================================================================== */
/* C-136 — adversarial feeding + every reachable pngpread.c error       */
/* ================================================================== */

/// C-136: buffer save/restore boundaries — zero-length feeds, chunk headers
/// split across feeds at every offset, IDAT split mid-row, the signature fed one
/// byte at a time, data after IEND, truncated input, `png_set_sig_bytes` with a
/// pre-consumed signature — and the error paths of `pngpread.c`.
#[test]
fn split_boundaries() {
    let mut rng = Rng::new(0x5b_0);
    let img = Img::random(&mut rng, 8, 8, PNG_COLOR_TYPE_RGB, 8);
    let file = build_plain(&img);
    let mut pal = Img::random(&mut rng, 6, 5, PNG_COLOR_TYPE_PALETTE, 4);
    pal.interlace = PNG_INTERLACE_ADAM7;
    let pfile = build_plain(&pal);
    let mut g16 = Img::random(&mut rng, 5, 4, PNG_COLOR_TYPE_GRAY_ALPHA, 16);
    g16.interlace = PNG_INTERLACE_ADAM7;
    let gfile = build_plain(&g16);
    let refs: [(&str, &Vec<u8>); 3] = [("rgb8", &file), ("pal4i", &pfile), ("ga16i", &gfile)];

    /* --- zero-length png_process_data calls --- */
    for (fname, f) in refs {
        let plans: [(&str, Vec<usize>); 5] = [
            ("all zeros then whole", {
                let mut v = vec![0usize; 6];
                v.push(f.len());
                v
            }),
            ("zero between every byte", {
                let mut v = Vec::new();
                for _ in 0..f.len() {
                    v.push(0);
                    v.push(1);
                }
                v
            }),
            ("zero between every 7 bytes", {
                let mut v = Vec::new();
                for _ in 0..f.len().div_ceil(7) {
                    v.push(0);
                    v.push(0);
                    v.push(7);
                }
                v
            }),
            ("growing pieces with zeros", {
                let mut v = Vec::new();
                let mut k = 0usize;
                let mut tot = 0usize;
                while tot < f.len() {
                    v.push(0);
                    v.push(k);
                    tot += k;
                    k += 1;
                }
                v.push(f.len());
                v
            }),
            ("whole then zeros", vec![f.len(), 0, 0, 0]),
        ];
        for (name, pieces) in plans {
            same(&format!("C-136 {} zero-feeds {}", fname, name), |api| unsafe {
                run(
                    api,
                    f,
                    &pieces,
                    Cfg {
                        trailing_empty: 3,
                        ..Default::default()
                    },
                )
            });
        }
        // trailing zero-length feeds with a NULL buffer
        same(&format!("C-136 {} NULL zero-feeds", fname), |api| unsafe {
            run(
                api,
                f,
                &[f.len()],
                Cfg {
                    trailing_empty: 4,
                    trailing_null: true,
                    ..Default::default()
                },
            )
        });
    }

    /* --- randomised piece sizes: the save-buffer path has to cope with any
           sequence of boundaries, not just the ones we thought of --- */
    for (fname, f) in refs {
        for seed in 0..12u64 {
            let mut r = Rng::new(0x9d_0000 ^ (seed << 8) ^ f.len() as u64);
            let mut pieces = Vec::new();
            let mut tot = 0usize;
            while tot < f.len() {
                let k = r.pick(&[0usize, 1, 1, 2, 3, 4, 7, 8, 9, 12, 16, 31, 64, 200]);
                pieces.push(k);
                tot += k;
            }
            pieces.push(f.len());
            same(
                &format!("C-136 {} random pieces seed {}", fname, seed),
                |api| unsafe {
                    run(
                        api,
                        f,
                        &pieces,
                        Cfg {
                            log_feeds: true,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    /* --- the 8-byte signature one byte at a time, and every prefix --- */
    for (fname, f) in refs {
        let mut v = vec![1usize; 8];
        v.push(f.len());
        same(&format!("C-136 {} signature byte by byte", fname), |api| unsafe {
            run(api, f, &v, Cfg { log_feeds: true, ..Default::default() })
        });
        for cut in 1..=8usize {
            same(
                &format!("C-136 {} signature split at {}", fname, cut),
                |api| unsafe {
                    run(
                        api,
                        f,
                        &[cut, f.len()],
                        Cfg {
                            log_feeds: true,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    /* --- chunk headers split across feeds at every possible offset --- */
    for (fname, f) in refs {
        let starts: Vec<usize> = split_chunks(f).into_iter().map(|(_, r)| r.start).collect();
        for &s in &starts {
            for k in 0..8usize {
                let at = s + k;
                same(
                    &format!("C-136 {} header split at {}+{}", fname, s, k),
                    |api| unsafe {
                        run(
                            api,
                            f,
                            &[at, f.len()],
                            Cfg {
                                log_feeds: true,
                                ..Default::default()
                            },
                        )
                    },
                );
                // three pieces: the header byte alone in the middle
                same(
                    &format!("C-136 {} header split at {}+{} (3 pieces)", fname, s, k),
                    |api| unsafe {
                        run(
                            api,
                            f,
                            &[at, 1, f.len()],
                            Cfg {
                                log_feeds: true,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* --- IDAT split mid-row / at every offset inside the compressed data --- */
    for (fname, f) in refs {
        let r = chunk_data_range(f, "IDAT");
        let n = r.len();
        let offs: Vec<usize> = if n <= 24 {
            (0..=n).collect()
        } else {
            let mut v: Vec<usize> = (0..8).collect();
            v.extend([n / 4, n / 3, n / 2, (2 * n) / 3, n - 3, n - 2, n - 1, n]);
            v
        };
        for off in offs {
            let at = r.start + off;
            same(
                &format!("C-136 {} IDAT split at data+{}", fname, off),
                |api| unsafe {
                    run(
                        api,
                        f,
                        &[at, 1, 2, f.len()],
                        Cfg {
                            log_feeds: true,
                            ..Default::default()
                        },
                    )
                },
            );
        }
        // and the whole IDAT payload one byte at a time
        let mut v = vec![r.start];
        v.extend(vec![1usize; r.len()]);
        v.push(f.len());
        same(&format!("C-136 {} IDAT byte by byte", fname), |api| unsafe {
            run(api, f, &v, Cfg::default())
        });
        let _ = n;
    }

    /* --- data after IEND --- */
    for (fname, f) in refs {
        let tails: [(&str, Vec<u8>); 4] = [
            ("garbage", vec![0xde, 0xad, 0xbe, 0xef]),
            ("another IEND", chunk(b"IEND", &[])),
            ("a whole second file", f.to_vec()),
            ("zero bytes of a chunk header", vec![0, 0, 0, 0]),
        ];
        for (name, tail) in tails {
            let mut d = f.to_vec();
            d.extend_from_slice(&tail);
            for &k in &[1usize, 13, d.len()] {
                same(
                    &format!("C-136 {} after IEND {} feed{}", fname, name, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                trailing_empty: 1,
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* --- truncated input: feed only the first N bytes and stop --- */
    for (fname, f) in refs {
        let n = f.len();
        let mut cuts: Vec<usize> = (0..=n.min(40)).collect();
        let mut i = 40;
        while i < n {
            cuts.push(i);
            i += 3;
        }
        cuts.push(n - 1);
        cuts.dedup();
        for cut in cuts {
            same(&format!("C-136 {} truncated to {}", fname, cut), |api| unsafe {
                run(
                    api,
                    &f[..cut],
                    &[cut.max(1)],
                    Cfg {
                        log_rows: false,
                        trailing_empty: 1,
                        ..Default::default()
                    },
                )
            });
        }
        // truncated *and* fed one byte at a time
        for cut in [1usize, 8, 9, 20, n / 2, n - 1] {
            same(
                &format!("C-136 {} truncated to {} byte by byte", fname, cut),
                |api| unsafe {
                    run(
                        api,
                        &f[..cut],
                        &fixed_pieces(cut, 1),
                        Cfg {
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    /* --- png_set_sig_bytes + a pre-consumed signature --- */
    for (fname, f) in refs {
        for pre in 0..=8usize {
            for &k in &[1usize, 3, f.len()] {
                let d = f[pre..].to_vec();
                same(
                    &format!("C-136 {} sig_bytes={} feed{}", fname, pre, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                sig_bytes: Some(pre as c_int),
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
        // out of range: png_set_sig_bytes rejects > 8 and clamps negatives to 0
        for pre in [9i32, 100, -1, -3] {
            same(
                &format!("C-136 {} sig_bytes={} (out of range)", fname, pre),
                |api| unsafe {
                    run(
                        api,
                        f,
                        &fixed_pieces(f.len(), 5),
                        Cfg {
                            sig_bytes: Some(pre),
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                },
            );
        }
        // a lie: claim bytes were consumed that were not
        for pre in [1usize, 4, 8] {
            same(
                &format!("C-136 {} sig_bytes={} but nothing consumed", fname, pre),
                |api| unsafe {
                    run(
                        api,
                        f,
                        &fixed_pieces(f.len(), 5),
                        Cfg {
                            sig_bytes: Some(pre as c_int),
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                },
            );
        }
    }

    /* ============================================================== */
    /* error paths                                                    */
    /* ============================================================== */

    /* D-68 / D-69: a bad signature, including the `num_to_check - 4`
       wrap-around when fewer than four bytes have been offered. */
    {
        let mut bad_first = file.clone();
        bad_first[0] ^= 0xff;
        let mut ascii = file.clone();
        ascii[4] = 10; // CR -> LF, the classic FTP-in-text-mode corruption
        let mut late = file.clone();
        late[7] ^= 0x01;
        let cases: [(&str, Vec<u8>); 3] = [
            ("bad first byte", bad_first),
            ("CRLF converted", ascii),
            ("bad last signature byte", late),
        ];
        for (name, d) in cases {
            for &k in &[1usize, 2, 3, 4, 5, 8, d.len()] {
                same(
                    &format!("D-68/69 signature {} feed{}", name, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                log_feeds: true,
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
        // a completely non-PNG stream, and a stream shorter than the signature
        for (name, d) in [
            ("text", b"not a png at all, sorry\n".to_vec()),
            ("one byte", vec![0x89u8]),
            ("two bytes", vec![0x89u8, 0x50]),
            ("three wrong bytes", vec![0x00u8, 0x00, 0x00]),
            ("seven good bytes", SIG[..7].to_vec()),
            ("empty", Vec::new()),
        ] {
            for &k in &[1usize, 2, 3, 8] {
                same(&format!("D-68 non-PNG {} feed{}", name, k), |api| unsafe {
                    run(
                        api,
                        &d,
                        &fixed_pieces(d.len(), k),
                        Cfg {
                            log_feeds: true,
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                });
            }
        }
    }

    /* D-70: an IDAT (or any chunk) before IHDR. */
    {
        let ps = parts(&file);
        let mut early_idat = vec![("IDAT".to_string(), Vec::new())];
        early_idat.extend(ps.clone());
        let mut early_idat2 = vec![(
            "IDAT".to_string(),
            ps.iter().find(|(n, _)| n == "IDAT").unwrap().1.clone(),
        )];
        early_idat2.extend(ps.clone());
        for (name, d) in [
            ("empty IDAT first", assemble(&early_idat)),
            ("full IDAT first", assemble(&early_idat2)),
        ] {
            for &k in &[1usize, 9, d.len()] {
                same(&format!("D-70 {} feed{}", name, k), |api| unsafe {
                    run(
                        api,
                        &d,
                        &fixed_pieces(d.len(), k),
                        Cfg {
                            log_rows: false,
                            ..Default::default()
                        },
                    )
                });
            }
        }
    }

    /* D-71: a palette image whose PLTE has been removed. */
    {
        let ps: Vec<(String, Vec<u8>)> =
            parts(&pfile).into_iter().filter(|(n, _)| n != "PLTE").collect();
        let d = assemble(&ps);
        for &k in &[1usize, 7, d.len()] {
            same(&format!("D-71 missing PLTE feed{}", k), |api| unsafe {
                run(
                    api,
                    &d,
                    &fixed_pieces(d.len(), k),
                    Cfg {
                        log_rows: false,
                        ..Default::default()
                    },
                )
            });
        }
    }

    /* D-72: an IDAT after a non-IDAT chunk that follows IDAT ("Too many IDATs
       found", a benign error, i.e. a warning on a read struct by default). */
    {
        let ps = parts(&file);
        let idat = ps.iter().find(|(n, _)| n == "IDAT").unwrap().1.clone();
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        for (n, d) in ps {
            if n == "IEND" {
                out.push(("tEXt".to_string(), b"Comment\0hi".to_vec()));
                out.push(("IDAT".to_string(), idat.clone()));
            }
            out.push((n, d));
        }
        let d = assemble(&out);
        for benign in [None, Some(0), Some(1)] {
            for &k in &[1usize, 11, d.len()] {
                same(
                    &format!("D-72 too many IDATs benign={:?} feed{}", benign, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                benign,
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* D-73: "Invalid IHDR length". */
    {
        let ps = parts(&file);
        let ihdr = ps.iter().find(|(n, _)| n == "IHDR").unwrap().1.clone();
        for extra in [-1i32, 1, 8] {
            let mut h = ihdr.clone();
            if extra < 0 {
                h.pop();
            } else {
                h.extend(vec![0u8; extra as usize]);
            }
            let mut out = ps.clone();
            out[0] = ("IHDR".to_string(), h);
            let d = assemble(&out);
            for &k in &[1usize, 9, d.len()] {
                same(
                    &format!("D-73 IHDR length {:+} feed{}", extra, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* D-77: "Not enough compressed data" — an IDAT that stops short. */
    for (fname, f) in refs {
        let ps = parts(f);
        let idat = ps.iter().find(|(n, _)| n == "IDAT").unwrap().1.clone();
        let lens: Vec<usize> = [0usize, 1, 2, 3, idat.len() / 2, idat.len() - 1]
            .into_iter()
            .filter(|&l| l < idat.len())
            .collect();
        for l in lens {
            let mut out: Vec<(String, Vec<u8>)> = Vec::new();
            for (n, d) in ps.clone() {
                if n == "IDAT" {
                    out.push(("IDAT".to_string(), idat[..l].to_vec()));
                } else {
                    out.push((n, d));
                }
            }
            let d = assemble(&out);
            for &k in &[1usize, 13, d.len()] {
                same(
                    &format!("D-77 {} short IDAT {} of {} feed{}", fname, l, idat.len(), k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* D-82 / D-83: too *much* compressed data.  Patching the IHDR height down
       makes inflate produce rows nobody asked for ("Extra compressed data in
       IDAT"); appending bytes to the IDAT payload leaves data after the zlib
       end code ("Extra compression data in IDAT"). */
    {
        let mut tall = Img::random(&mut Rng::new(0xe47a), 6, 6, PNG_COLOR_TYPE_GRAY, 8);
        tall.interlace = PNG_INTERLACE_NONE;
        let tf = build_plain(&tall);
        let ps = parts(&tf);
        for newh in [1u32, 2, 5] {
            let mut out = ps.clone();
            let mut h = out[0].1.clone();
            h[4..8].copy_from_slice(&newh.to_be_bytes());
            out[0] = ("IHDR".to_string(), h);
            let d = assemble(&out);
            for &k in &[1usize, 17, d.len()] {
                same(
                    &format!("D-82 height {} -> {} feed{}", tall.h, newh, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg {
                                log_rows: true,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
        for extra in [1usize, 4, 64] {
            let mut out = ps.clone();
            for p in out.iter_mut() {
                if p.0 == "IDAT" {
                    p.1.extend(vec![0x5au8; extra]);
                }
            }
            let d = assemble(&out);
            for &k in &[1usize, 17, d.len()] {
                same(
                    &format!("D-83 {} extra IDAT bytes feed{}", extra, k),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), k),
                            Cfg::default(),
                        )
                    },
                );
            }
        }
    }

    /* D-79 / D-80 / D-81: a damaged zlib stream.  The CRC is recomputed so that
       the *decompressor* is what rejects the stream, not png_crc_finish. */
    {
        let mut base = Img::random(&mut Rng::new(0x2b_ad), 12, 5, PNG_COLOR_TYPE_RGB, 8);
        base.interlace = PNG_INTERLACE_NONE;
        let bf = build_plain(&base);
        let ps = parts(&bf);
        let idat = ps.iter().find(|(n, _)| n == "IDAT").unwrap().1.clone();
        let mut rng = Rng::new(0x2b_ad_5eed);
        for case in 0..24usize {
            let mut z = idat.clone();
            let hits = 1 + rng.below(3);
            for _ in 0..hits {
                let at = rng.below(z.len());
                z[at] ^= 1 << rng.below(8);
            }
            let mut out: Vec<(String, Vec<u8>)> = Vec::new();
            for (n, d) in ps.clone() {
                if n == "IDAT" {
                    out.push(("IDAT".to_string(), z.clone()));
                } else {
                    out.push((n, d));
                }
            }
            let d = assemble(&out);
            for benign in [Some(0), Some(1)] {
                same(
                    &format!("D-79/80/81 corrupt zlib #{} benign={:?}", case, benign),
                    |api| unsafe {
                        run(
                            api,
                            &d,
                            &fixed_pieces(d.len(), 7),
                            Cfg {
                                benign,
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
    }

    /* D-84: "bad adaptive filter value" — a hand-built stored-deflate IDAT
       whose filter byte is out of range. */
    for f in 0..=6u8 {
        let d = handmade_1x2_gray(f);
        for &k in &[1usize, 9, d.len()] {
            same(
                &format!("D-84 filter byte {} feed{}", f, k),
                |api| unsafe {
                    run(
                        api,
                        &d,
                        &fixed_pieces(d.len(), k),
                        Cfg::default(),
                    )
                },
            );
        }
    }

    /* Neither png_read_update_info nor png_start_read_image: png_ptr->row_buf
       stays NULL, and libpng has to notice rather than write through it. */
    for (fname, f) in refs {
        same(&format!("C-136 {} no row init", fname), |api| unsafe {
            run(
                api,
                f,
                &fixed_pieces(f.len(), 23),
                Cfg {
                    no_row_init: true,
                    act: RowAct::Nothing,
                    ..Default::default()
                },
            )
        });
    }

    /* png_process_data called again after a fatal error — the struct is in an
       undefined state, so run each library in its own child. */
    {
        let mut broken = file.clone();
        broken[1] ^= 0xff;
        let after_end = {
            let mut d = file.clone();
            d.extend_from_slice(&chunk(b"tEXt", b"k\0v"));
            d
        };
        for (name, d) in [("after a bad signature", broken), ("after IEND", after_end)] {
            same_forked(&format!("C-136 process_data again {}", name), |api| unsafe {
                *cfg() = Cfg {
                    log_rows: false,
                    ..Default::default()
                };
                let mut buf = d.clone();
                let (png, info) = new_read(api);
                (api.png_set_progressive_read_fn)(
                    png,
                    cookie(),
                    Some(info_cb),
                    Some(row_cb),
                    Some(end_cb),
                );
                let g1 = guarded(api, png, &mut || {
                    (api.png_process_data)(png, info, buf.as_mut_ptr(), buf.len());
                });
                let g2 = guarded(api, png, &mut || {
                    (api.png_process_data)(png, info, buf.as_mut_ptr(), buf.len());
                });
                let g3 = guarded(api, png, &mut || {
                    (api.png_process_data)(png, info, buf.as_mut_ptr(), 0);
                    let r = (api.png_process_data_pause)(png, 0);
                    log(format!("pause -> {}", r));
                });
                format!(
                    "{:?} / {:?} / {:?} rows={}",
                    g1,
                    g2,
                    g3,
                    cfg().row_calls
                )
            });
        }
    }

    /* --- a stream big enough to make png_push_save_buffer grow, reallocate
           and compact its buffer many times over: a 4 kB tEXt before IDAT, a
           3 kB unknown chunk after IDAT and a ~14 kB uncompressed IDAT, fed one
           byte at a time --- */
    {
        let mut big = Img::random(&mut Rng::new(0xb1_9), 60, 30, PNG_COLOR_TYPE_RGB, 8);
        big.interlace = PNG_INTERLACE_NONE;
        let mut bf = build(
            &big,
            &WriteOpts {
                level: Some(0),
                buffer_size: Some(97),
                ..Default::default()
            },
        );
        let mut txt = b"Comment\0".to_vec();
        txt.extend(Rng::new(0x7e_47).bytes(4000).iter().map(|b| 0x20 + (b % 0x5e)));
        bf = insert_before(&bf, "IDAT", &chunk(b"tEXt", &txt));
        bf = insert_after_last(&bf, "IDAT", &chunk(b"prVt", &Rng::new(0x9f).bytes(3000)));
        assert!(bf.len() > 12000, "big stream is only {} bytes", bf.len());
        for keep in [None, Some(PNG_HANDLE_CHUNK_ALWAYS)] {
            for &k in &[1usize, 3, 97, 250, 8192, bf.len()] {
                same(
                    &format!("C-136 big stream keep={:?} feed{}", keep, k),
                    |api| unsafe {
                        run(
                            api,
                            &bf,
                            &fixed_pieces(bf.len(), k),
                            Cfg {
                                keep_unknown: keep,
                                log_rows: false,
                                ..Default::default()
                            },
                        )
                    },
                );
            }
        }
        // randomised piece sizes over the same big stream
        for seed in 0..6u64 {
            let mut r = Rng::new(0xb1_9_5eed ^ seed);
            let mut pieces = Vec::new();
            let mut tot = 0usize;
            while tot < bf.len() {
                let n = r.pick(&[0usize, 1, 2, 5, 17, 64, 255, 256, 257, 1000, 4096]);
                pieces.push(n);
                tot += n;
            }
            pieces.push(bf.len());
            same(&format!("C-136 big stream random pieces {}", seed), |api| unsafe {
                run(
                    api,
                    &bf,
                    &pieces,
                    Cfg {
                        keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS),
                        log_rows: false,
                        ..Default::default()
                    },
                )
            });
        }

        /* --- and the same stream with a broken CRC on each chunk in turn,
               under every png_set_crc_action pair --- */
        for target in ["IHDR", "tEXt", "IDAT", "prVt", "IEND"] {
            let ps = parts(&bf);
            let mut d = SIG.to_vec();
            for (n, data) in &ps {
                let b = n.as_bytes();
                let nm = [b[0], b[1], b[2], b[3]];
                if n == target {
                    d.extend_from_slice(&chunk_bad_crc(&nm, data));
                } else {
                    d.extend_from_slice(&chunk(&nm, data));
                }
            }
            for crit in [PNG_CRC_DEFAULT, PNG_CRC_WARN_USE, PNG_CRC_QUIET_USE] {
                for anc in [PNG_CRC_DEFAULT, PNG_CRC_ERROR_QUIT, PNG_CRC_WARN_DISCARD] {
                    same(
                        &format!("C-136 bad CRC on {} crc_action=({},{})", target, crit, anc),
                        |api| unsafe {
                            run(
                                api,
                                &d,
                                &fixed_pieces(d.len(), 61),
                                Cfg {
                                    crc_action: Some((crit, anc)),
                                    keep_unknown: Some(PNG_HANDLE_CHUNK_ALWAYS),
                                    log_rows: false,
                                    ..Default::default()
                                },
                            )
                        },
                    );
                }
            }
        }
    }

    /* --- a chunk length the *progressive* reader parses with
           png_get_uint_31 (png_push_read_IDAT) rather than with
           png_read_chunk_header --- */
    {
        let mut small = Img::random(&mut Rng::new(0x1e_31), 3, 2, PNG_COLOR_TYPE_GRAY, 8);
        small.interlace = PNG_INTERLACE_NONE;
        let sf = build_plain(&small);
        let ps = parts(&sf);
        let ihdr = ps.iter().find(|(n, _)| n == "IHDR").unwrap().1.clone();
        let idat = ps.iter().find(|(n, _)| n == "IDAT").unwrap().1.clone();
        for bad in [0x80000000u32, 0xffffffff, 0x7fffffff] {
            // the header png_push_read_IDAT reads *after* the IDAT it is in
            let mut d = SIG.to_vec();
            d.extend_from_slice(&chunk(b"IHDR", &ihdr));
            d.extend_from_slice(&chunk(b"IDAT", &idat));
            d.extend_from_slice(&bad.to_be_bytes());
            d.extend_from_slice(b"IEND");
            d.extend_from_slice(&crc32(b"IEND").to_be_bytes());
            for &k in &[1usize, 9, d.len()] {
                same(
                    &format!("C-136 post-IDAT chunk length 0x{:08x} feed{}", bad, k),
                    |api| unsafe {
                        run(api, &d, &fixed_pieces(d.len(), k), Cfg::default())
                    },
                );
            }
            // and the length of the IDAT itself, which chunk mode parses
            let mut e = SIG.to_vec();
            e.extend_from_slice(&chunk(b"IHDR", &ihdr));
            e.extend_from_slice(&bad.to_be_bytes());
            e.extend_from_slice(b"IDAT");
            e.extend_from_slice(&idat);
            e.extend_from_slice(&crc32(b"IDAT").to_be_bytes());
            e.extend_from_slice(&chunk(b"IEND", &[]));
            for &k in &[1usize, 9, e.len()] {
                same(
                    &format!("C-136 IDAT chunk length 0x{:08x} feed{}", bad, k),
                    |api| unsafe {
                        run(api, &e, &fixed_pieces(e.len(), k), Cfg::default())
                    },
                );
            }
        }
    }

    // Prove the error paths above were really reached rather than silently
    // agreeing on a happy path.
    let seen = observed();
    for want in [
        "Not a PNG file",
        "PNG file corrupted by ASCII conversion",
        "Missing IHDR before IDAT",
        "Missing PLTE before IDAT",
        "Too many IDATs found",
        "Invalid IHDR length",
        "Not enough compressed data",
        "bad adaptive filter value",
        "Extra compressed data in IDAT",
        "Extra compression data in IDAT",
        "PNG unsigned integer out of range",
        "CRC error",
        // png_chunk_warning prefixes the chunk name, hence the substring match
        "Too many IDATs found",
        "ADLER32 checksum mismatch",
        "Truncated compressed data in IDAT",
    ] {
        assert!(
            seen.iter().any(|m| m.contains(want)),
            "the progressive error-path cases never produced {:?}; observed: {:?}",
            want,
            seen
        );
    }

    report("split_boundaries");
}

/// A hand-built 1x2 8-bit grey PNG whose two rows carry filter byte `f`.
/// Level-0 (stored) deflate, so the bytes are entirely under our control.
fn handmade_1x2_gray(f: u8) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&2u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    let raw = [f, 0x40u8, f, 0x80u8];
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    z.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    z.extend_from_slice(&raw);
    let mut a: (u32, u32) = (1, 0);
    for &b in &raw {
        a.0 = (a.0 + b as u32) % 65521;
        a.1 = (a.1 + a.0) % 65521;
    }
    z.extend_from_slice(&((a.1 << 16) | a.0).to_be_bytes());
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr));
    v.extend_from_slice(&chunk(b"IDAT", &z));
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

/* ================================================================== */
/* C-137 — the progressive reader with read transforms                  */
/* ================================================================== */

/// C-137: read transforms installed from the info callback (which then calls
/// `png_read_update_info`, so the destination rows are sized from the
/// *transformed* `png_get_rowbytes`).
#[test]
fn transforms() {
    for (si, &(ct, bd)) in VALID_SHAPES.iter().enumerate() {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(
                0x7_4a45 ^ ((ct as u64) << 40) ^ ((bd as u64) << 32) ^ ((il as u64) << 24) ^ si as u64,
            );
            let (w, h) = [(1u32, 1u32), (5, 4), (9, 3), (13, 6)][si % 4];
            let mut img = Img::random(&mut rng, w, h, ct, bd);
            img.interlace = il;
            let file = build_plain(&img);
            let tag = format!("ct={} bd={} il={} {}x{}", ct, bd, il, w, h);

            /* one transform at a time */
            for t in CORE_TF {
                for &k in &[1usize, 13, file.len()] {
                    same(
                        &format!("C-137 {} {:?} feed{}", tag, t, k),
                        |api| unsafe {
                            run(
                                api,
                                &file,
                                &fixed_pieces(file.len(), k),
                                Cfg {
                                    tr: vec![t],
                                    ..Default::default()
                                },
                            )
                        },
                    );
                }
            }

            /* random combinations */
            for c in 0..8usize {
                let n = 2 + rng.below(3);
                let mut tr: Vec<Tf> = Vec::new();
                for _ in 0..n {
                    let t = rng.pick(&POOL_TF);
                    if !tr.contains(&t) {
                        tr.push(t);
                    }
                }
                let feed = rng.pick(&[1usize, 3, 29, 1024]);
                let k = if feed == 1024 { file.len() } else { feed };
                let tr2 = tr.clone();
                same(
                    &format!("C-137 {} combo#{} {:?} feed{}", tag, c, tr, k),
                    |api| unsafe {
                        run(
                            api,
                            &file,
                            &fixed_pieces(file.len(), k),
                            Cfg {
                                tr: tr2.clone(),
                                ..Default::default()
                            },
                        )
                    },
                );
            }

            /* the transforms that need tRNS to do anything, on a file that has it */
            if ct == PNG_COLOR_TYPE_PALETTE || ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB
            {
                let trns_file = with_c_tls(|api| unsafe {
                    write_image(api, &img, &WriteOpts::default(), &mut |api, png, info| {
                        match img.color_type {
                            PNG_COLOR_TYPE_PALETTE => {
                                let n = img.palette.len();
                                let t: Vec<u8> = (0..n).map(|i| (i as u8) ^ 0x55).collect();
                                (api.png_set_tRNS)(
                                    png,
                                    info,
                                    t.as_ptr(),
                                    n as c_int,
                                    core::ptr::null(),
                                );
                            }
                            PNG_COLOR_TYPE_GRAY => {
                                let c = png_color_16 {
                                    index: 0,
                                    red: 0,
                                    green: 0,
                                    blue: 0,
                                    gray: 1,
                                };
                                (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
                            }
                            _ => {
                                let c = png_color_16 {
                                    index: 0,
                                    red: 1,
                                    green: 2,
                                    blue: 3,
                                    gray: 0,
                                };
                                (api.png_set_tRNS)(png, info, core::ptr::null(), 0, &c);
                            }
                        }
                    })
                    .bytes
                });
                for tr in [
                    vec![Tf::TrnsToAlpha],
                    vec![Tf::Expand],
                    vec![Tf::TrnsToAlpha, Tf::Expand16],
                    vec![Tf::Expand, Tf::Strip16, Tf::Bgr],
                    vec![Tf::Expand, Tf::GrayToRgb, Tf::Filler(0x11, PNG_FILLER_AFTER)],
                ] {
                    let tr2 = tr.clone();
                    same(
                        &format!("C-137 {} tRNS {:?}", tag, tr),
                        |api| unsafe {
                            run(
                                api,
                                &trns_file,
                                &fixed_pieces(trns_file.len(), 19),
                                Cfg {
                                    tr: tr2.clone(),
                                    ..Default::default()
                                },
                            )
                        },
                    );
                }
            }
        }
    }

    /* A user transform that lies about the pixel depth is how the two
       "progressive row" consistency checks in png_push_process_row are reached
       (ERRORS.md D-85 and D-86). */
    for (name, mode) in [("grow", 0u64), ("shrink then grow", 1u64)] {
        let mut img = Img::random(&mut Rng::new(0xd_8586 ^ mode), 4, 3, PNG_COLOR_TYPE_GRAY, 16);
        img.interlace = PNG_INTERLACE_NONE;
        let f = build_plain(&img);
        same(&format!("D-85/86 lying user transform ({})", name), |api| unsafe {
            let mut o = Outcome::default();
            *cfg() = Cfg {
                act: RowAct::Nothing,
                ..Default::default()
            };
            LIE.with(|c| c.set(mode));
            LIE_N.with(|c| c.set(0));
            let mut buf = f.clone();
            let (png, info) = new_read(api);
            (api.png_set_progressive_read_fn)(
                png,
                cookie(),
                Some(lying_info_cb),
                Some(row_cb),
                Some(end_cb),
            );
            let g = guarded(api, png, &mut || {
                (api.png_process_data)(png, info, buf.as_mut_ptr(), buf.len());
            });
            o.push(format!(
                "guard={:?} rows={} lies={}",
                g,
                cfg().row_calls,
                LIE_N.with(|c| c.get())
            ));
            destroy_read(api, png, info);
            o
        });
    }

    let seen = observed();
    for want in [
        "progressive row overflow",
        "internal progressive row size calculation error",
    ] {
        assert!(
            seen.iter().any(|m| m.contains(want)),
            "the lying-user-transform cases never produced {:?}; observed: {:?}",
            want,
            seen
        );
    }

    report("transforms");
}

thread_local! {
    static LIE: Cell<u64> = const { Cell::new(0) };
    static LIE_N: Cell<u64> = const { Cell::new(0) };
}

/// Reports a bogus `pixel_depth` so that `png_push_process_row`'s
/// `transformed_pixel_depth` bookkeeping rejects the row.
unsafe extern "C" fn lying_transform_cb(
    _png: *mut PngStruct,
    row_info: *mut png_row_info,
    _row: *mut u8,
) {
    let n = LIE_N.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    });
    let mode = LIE.with(|c| c.get());
    // png_do_read_transformations recomputes pixel_depth from bit_depth *
    // channels straight after this callback returns, so lie about *those*.
    let old = (*row_info).pixel_depth;
    if mode == 0 {
        // 16 x 4 = 64 bits/pixel, far above maximum_pixel_depth
        (*row_info).bit_depth = 16;
        (*row_info).channels = 4;
    } else if n == 0 {
        (*row_info).bit_depth = 8;
        (*row_info).channels = 1;
    } else {
        (*row_info).bit_depth = 16;
        (*row_info).channels = 1;
    }
    log(format!(
        "lying_transform #{} pixel_depth {} -> bit_depth {} x channels {}",
        n,
        old,
        (*row_info).bit_depth,
        (*row_info).channels
    ));
}

unsafe extern "C" fn lying_info_cb(png: *mut PngStruct, info: *mut PngInfo) {
    let api = cur_api();
    let c = cfg();
    c.info_calls += 1;
    log("info_cb (lying)".to_string());
    log_shape(api, png, info, "info_cb");
    (api.png_set_read_user_transform_fn)(png, Some(lying_transform_cb));
    (api.png_read_update_info)(png, info);
    log_shape(api, png, info, "after row init");
    c.width = (api.png_get_image_width)(png, info);
    c.height = (api.png_get_image_height)(png, info);
    c.rowbytes = (api.png_get_rowbytes)(png, info);
    tls().rows = vec![vec![0u8; c.rowbytes.max(1)]; c.height.max(1) as usize];
}
