//! Progressive ("push") read differential tests, CONFIGS.md rows P1..P8.
//!
//! Every test builds its input PNG with `support::pngbuild` (independent of
//! both libraries), then drives the C `.so` and the Rust `.so` through the
//! identical `png_process_data` call sequence on those identical bytes and
//! compares the complete trace byte for byte.
//!
//! The progressive reader hands the application `png_struct::row_buf` directly,
//! so parts of a row that libpng does not write (the columns before
//! `PNG_PASS_START_COL(pass)` of an expanded interlaced row, for instance) are
//! visible to the row callback.  To keep that deterministic every read struct
//! here is created with `png_create_read_struct_2` and the harness memory
//! callbacks, whose `malloc` is a `calloc` — so both libraries start from
//! zeroed buffers and any dependence on "uninitialised" memory is identical.
mod support;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};
use support::core::*;
use support::pngbuild::{self, Builder, Chunk};
use support::*;

// ---------------------------------------------------------------------------
// misc helpers
// ---------------------------------------------------------------------------

/// The 15 legal (colour_type, bit_depth) combinations.
const COMBOS: &[(u8, u8)] = &[
    (0, 1),
    (0, 2),
    (0, 4),
    (0, 8),
    (0, 16),
    (2, 8),
    (2, 16),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 8),
    (4, 8),
    (4, 16),
    (6, 8),
    (6, 16),
];

/// Bytes logged past the end of a row, so an overrun becomes visible.
const SLACK: usize = 8;

/// The reference `libpng.so` in `target/cbuild` is linked without `-lm`, so its
/// undefined `floor`/`pow` references have to be satisfied from the global
/// symbol scope (`png_set_gamma` reaches `png_fixed` -> `floor`).  Loading
/// `libm` with `RTLD_GLOBAL` does that for both libraries identically.
fn ensure_libm() {
    use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_NOW};
    use std::sync::OnceLock;
    static LIBM: OnceLock<UnixLib> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        UnixLib::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL).expect("dlopen libm.so.6")
    });
}

unsafe fn sl(p: *const u8, n: usize) -> &'static [u8] {
    if n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

// ---------------------------------------------------------------------------
// input construction
// ---------------------------------------------------------------------------

fn palette_for(bd: u8, seed: u64) -> Vec<u8> {
    let n = 1usize << bd; // every index of a `bd`-bit image is in range
    let mut r = Rng::new(seed);
    (0..3 * n).map(|_| r.byte()).collect()
}

fn sbit_for(ct: u8, bd: u8) -> Vec<u8> {
    let v = if bd > 1 { bd - 1 } else { 1 };
    match ct {
        0 => vec![v],
        2 => vec![v, v, v],
        3 => vec![8, 7, 6],
        4 => vec![v, v],
        _ => vec![v, v, v, v],
    }
}

fn bkgd_for(ct: u8, bd: u8) -> Vec<u8> {
    match ct {
        3 => vec![1],
        0 | 4 => {
            let m: u32 = if bd >= 16 { 0x1234 } else { (1u32 << bd) - 1 };
            (m as u16).to_be_bytes().to_vec()
        }
        _ => {
            let m: u16 = if bd >= 16 { 0x1234 } else { (1u16 << bd) - 1 };
            let mut v = Vec::new();
            for k in [m, m / 2, m / 3] {
                v.extend_from_slice(&k.to_be_bytes());
            }
            v
        }
    }
}

fn trns_for(ct: u8, bd: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    match ct {
        0 => {
            let m: u32 = if bd >= 16 { 0xffff } else { (1u32 << bd) - 1 };
            ((r.next_u32() % (m + 1)) as u16).to_be_bytes().to_vec()
        }
        2 => {
            let m: u32 = if bd >= 16 { 0xffff } else { 0xff };
            let mut v = Vec::new();
            for _ in 0..3 {
                v.extend_from_slice(&((r.next_u32() % (m + 1)) as u16).to_be_bytes());
            }
            v
        }
        3 => {
            let n = 1usize << bd;
            r.bytes(n)
        }
        _ => Vec::new(),
    }
}

/// Plain valid PNG (PLTE added for palette images), all row filters 0.
fn mk(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    b.build_valid(seed)
}

/// Valid PNG carrying gAMA + sBIT + (tRNS): enough for the transform tests.
fn mk_rich(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    b = b.add(b"gAMA", 45455u32.to_be_bytes().to_vec());
    b = b.add(b"sBIT", sbit_for(ct, bd));
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    let t = trns_for(ct, bd, seed ^ 0x7a17_9999);
    if !t.is_empty() {
        b = b.add(b"tRNS", t);
    }
    b.build_valid(seed)
}

/// Raw pre-compression stream whose row filter bytes cycle through 0..4.
fn raw_filters(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed);
    let mut out = Vec::new();
    let mut fi = 0u8;
    if il == 0 {
        let rb = pngbuild::rowbytes(ct, bd, w);
        for _ in 0..h {
            out.push(fi % 5);
            fi = fi.wrapping_add(1);
            for _ in 0..rb {
                out.push(r.byte());
            }
        }
    } else {
        for p in 0..7 {
            let pw = pngbuild::pass_width(w, p);
            let ph = pngbuild::pass_height(h, p);
            if pw == 0 || ph == 0 {
                continue;
            }
            let rb = pngbuild::rowbytes(ct, bd, pw);
            for _ in 0..ph {
                out.push(fi % 5);
                fi = fi.wrapping_add(1);
                for _ in 0..rb {
                    out.push(r.byte());
                }
            }
        }
    }
    out
}

/// Valid PNG exercising all five row filter types.
fn mk_filters(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut b = Builder::new(w, h, bd, ct).interlace(il);
    if ct == 3 {
        b = b.add(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234));
    }
    let raw = raw_filters(w, h, ct, bd, il, seed ^ 0xf117_e000);
    b.build(&raw, 0)
}

fn ztxt(key: &[u8], text: &[u8]) -> Vec<u8> {
    let mut d = key.to_vec();
    d.push(0);
    d.push(0); // compression method: zlib
    d.extend_from_slice(&pngbuild::zlib_stored(text));
    d
}

fn text_chunk(key: &[u8], text: &[u8]) -> Vec<u8> {
    let mut d = key.to_vec();
    d.push(0);
    d.extend_from_slice(text);
    d
}

const CHRM: [u32; 8] = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];

fn chrm_bytes() -> Vec<u8> {
    let mut v = Vec::new();
    for x in CHRM {
        v.extend_from_slice(&x.to_be_bytes());
    }
    v
}

fn time_bytes() -> Vec<u8> {
    let mut v = 2024u16.to_be_bytes().to_vec();
    v.extend_from_slice(&[7, 14, 12, 34, 56]);
    v
}

/// An image carrying ancillary chunks before *and* after IDAT, in canonical
/// order, plus private (unknown) chunks in both places.
fn mk_ancillary(w: u32, h: u32, ct: u8, bd: u8, il: u8, seed: u64) -> Vec<u8> {
    let mut r = Rng::new(seed ^ 0xa11c);
    let mut cs: Vec<Chunk> = Vec::new();
    let b = Builder::new(w, h, bd, ct).interlace(il);
    cs.push(Chunk::new(b"IHDR", b.ihdr_bytes()));
    cs.push(Chunk::new(b"gAMA", 45455u32.to_be_bytes().to_vec()));
    cs.push(Chunk::new(b"cHRM", chrm_bytes()));
    cs.push(Chunk::new(b"sBIT", sbit_for(ct, bd)));
    cs.push(Chunk::new(b"tIME", time_bytes()));
    cs.push(Chunk::new(b"prVt", r.bytes(11)));
    if ct == 3 {
        cs.push(Chunk::new(b"PLTE", palette_for(bd, seed ^ 0x5eed_1234)));
    }
    cs.push(Chunk::new(b"bKGD", bkgd_for(ct, bd)));
    cs.push(Chunk::new(b"tEXt", text_chunk(b"Title", b"before idat")));
    cs.push(Chunk::new(b"zTXt", ztxt(b"Comment", b"compressed before idat")));
    cs.push(Chunk::new(b"prIv", r.bytes(5)));
    cs.push(Chunk::new(b"IDAT", pngbuild::zlib_stored(&b.raw_rows(seed))));
    cs.push(Chunk::new(b"tEXt", text_chunk(b"After", b"after idat")));
    cs.push(Chunk::new(b"zTXt", ztxt(b"zAfter", b"compressed after idat")));
    cs.push(Chunk::new(b"poSt", r.bytes(7)));
    cs.push(Chunk::new(b"IEND", Vec::new()));
    pngbuild::join(&cs)
}

// ---------------------------------------------------------------------------
// the context the C callbacks work through
// ---------------------------------------------------------------------------

const T_EXPAND: u32 = 1 << 0;
const T_GRAY_TO_RGB: u32 = 1 << 1;
const T_STRIP16: u32 = 1 << 2;
const T_PACKING: u32 = 1 << 3;
const T_GAMMA: u32 = 1 << 4;
const T_BGR: u32 = 1 << 5;
const T_EXPAND16: u32 = 1 << 6;
const T_SWAP: u32 = 1 << 7;

const TNAMES: &[(u32, &str)] = &[
    (T_EXPAND, "expand"),
    (T_GRAY_TO_RGB, "gray2rgb"),
    (T_STRIP16, "strip16"),
    (T_PACKING, "packing"),
    (T_GAMMA, "gamma"),
    (T_BGR, "bgr"),
    (T_EXPAND16, "expand16"),
    (T_SWAP, "swap"),
];

fn tname(m: u32) -> String {
    let mut s = String::new();
    for &(b, n) in TNAMES {
        if m & b != 0 {
            if !s.is_empty() {
                s.push('+');
            }
            s.push_str(n);
        }
    }
    if s.is_empty() {
        s.push_str("none");
    }
    s
}

#[repr(C)]
struct Ctx {
    /// Function table of the library currently being driven.
    core: *const Core,
    user_ptr: *mut c_void,
    /// Full-image destination buffer for `png_progressive_combine_row`.
    dst: *mut u8,
    stride: usize,
    dst_len: usize,
    width: u32,
    height: u32,
    interlaced: bool,

    // --- configuration -----------------------------------------------------
    tflags: u32,
    interlace_handling: bool,
    /// use `png_start_read_image` instead of `png_read_update_info`
    start_read_image: bool,
    combine: bool,
    dump_dst: bool,
    check_ptr: bool,
    log_rows: bool,
    /// `save` argument for a `png_process_data_pause` inside the info callback
    /// (-1: do not pause)
    pause_info: c_int,
    /// same, inside the row callback
    pause_row: c_int,
    /// which row-callback invocation pauses (1-based)
    pause_row_at: u32,
    /// `save` argument for a pause inside the user chunk callback, i.e. exactly
    /// at a chunk boundary (-1: do not pause)
    pause_chunk: c_int,
    /// `png_set_keep_unknown_chunks(png, keep_default, NULL, 0)` (-1: skip)
    keep_default: c_int,
    /// `png_set_keep_unknown_chunks(png, keep_list, KEEP_LIST, n)` (-1: skip)
    keep_list: c_int,
    keep_list_n: c_int,
    /// value the user chunk callback returns (-2: do not install it)
    user_chunk_ret: c_int,
    /// `png_set_benign_errors` argument (-1: do not call)
    benign: c_int,
    /// 0: never call `png_process_data_skip`; 1: after every `png_process_data`;
    /// 2: once before the first `png_process_data`; 3: only after a pause
    skip_mode: u8,

    // --- state -------------------------------------------------------------
    rowbytes: usize,
    pixel_depth: usize,
    row_calls: u32,
    info_calls: u32,
    end_calls: u32,
    chunk_calls: u32,
    pause_ret: usize,
    pause_seen: bool,
}

fn ctx_of(w: u32, h: u32, il: u8) -> Ctx {
    Ctx {
        core: std::ptr::null(),
        user_ptr: std::ptr::null_mut(),
        dst: std::ptr::null_mut(),
        stride: 0,
        dst_len: 0,
        width: w,
        height: h,
        interlaced: il != 0,
        tflags: 0,
        interlace_handling: il != 0,
        start_read_image: false,
        combine: false,
        dump_dst: false,
        check_ptr: false,
        log_rows: true,
        pause_info: -1,
        pause_row: -1,
        pause_row_at: 1,
        pause_chunk: -1,
        keep_default: -1,
        keep_list: -1,
        keep_list_n: 0,
        user_chunk_ret: -2,
        benign: -1,
        skip_mode: 0,
        rowbytes: 0,
        pixel_depth: 0,
        row_calls: 0,
        info_calls: 0,
        end_calls: 0,
        chunk_calls: 0,
        pause_ret: 0,
        pause_seen: false,
    }
}

thread_local! {
    static CTX: Cell<*mut Ctx> = Cell::new(std::ptr::null_mut());
    static LABEL: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

fn set_ctx(p: *mut Ctx) {
    CTX.with(|c| c.set(p));
}

fn ctx_ptr() -> *mut Ctx {
    CTX.with(|c| c.get())
}

/// Chunk list for `png_set_keep_unknown_chunks`: two *known* chunk names, so
/// that libpng is forced to treat them as unknown.
static KEEP_LIST: [u8; 15] = [
    b'g', b'A', b'M', b'A', 0, b't', b'E', b'X', b't', 0, b'p', b'r', b'V', b't', 0,
];

// ---------------------------------------------------------------------------
// callbacks
// ---------------------------------------------------------------------------

unsafe fn apply_transforms(c: &Core, png: Png, m: u32) {
    if m & T_EXPAND != 0 {
        (c.set_expand)(png);
    }
    if m & T_GRAY_TO_RGB != 0 {
        (c.set_gray_to_rgb)(png);
    }
    if m & T_STRIP16 != 0 {
        (c.set_strip_16)(png);
    }
    if m & T_PACKING != 0 {
        (c.set_packing)(png);
    }
    if m & T_GAMMA != 0 {
        (c.set_gamma)(png, 2.2, 0.45455);
    }
    if m & T_BGR != 0 {
        (c.set_bgr)(png);
    }
    if m & T_EXPAND16 != 0 {
        (c.set_expand_16)(png);
    }
    if m & T_SWAP != 0 {
        (c.set_swap)(png);
    }
}

unsafe fn do_pause(k: &mut Ctx, c: &Core, png: Png, save: c_int, site: &str) {
    let r = (c.process_data_pause)(png, save);
    log(format!("  PAUSE({site},save={save})={r}"));
    if !k.pause_seen {
        k.pause_seen = true;
        k.pause_ret = r;
    }
}

/// Number of bytes of `new_row` that libpng has filled in.
fn row_valid(k: &Ctx, pass: c_int) -> usize {
    if !k.interlaced || k.interlace_handling {
        k.rowbytes
    } else {
        let p = if pass < 0 {
            0
        } else if pass > 6 {
            6
        } else {
            pass as usize
        };
        let pw = pngbuild::pass_width(k.width, p) as usize;
        (k.pixel_depth * pw + 7) / 8
    }
}

unsafe extern "C" fn info_cb(png: Png, info: Info) {
    let kp = ctx_ptr();
    if kp.is_null() {
        return;
    }
    let k = &mut *kp;
    let c = &*k.core;
    k.info_calls += 1;
    log(format!("INFO_CB#{}", k.info_calls));
    // A stream with an IDAT chunk after a non-IDAT chunk gets the info callback
    // a second time; the row initialisation must happen exactly once (a second
    // png_read_update_info is a "duplicate call" application error).
    if k.info_calls == 1 {
        apply_transforms(c, png, k.tflags);
        if k.interlace_handling {
            log(format!("  passes={}", (c.set_interlace_handling)(png)));
        }
        if k.start_read_image {
            (c.start_read_image)(png);
        } else {
            (c.read_update_info)(png, info);
        }
    }
    let rb = (c.get_rowbytes)(png, info);
    let ch = (c.get_channels)(png, info);
    let bd = (c.get_bit_depth)(png, info);
    k.rowbytes = rb;
    k.pixel_depth = ch as usize * bd as usize;
    log(format!(
        "  rowbytes={rb} channels={ch} depth={bd} color={}",
        (c.get_color_type)(png, info)
    ));
    log_all_info(c, png, info);
    if k.check_ptr {
        log(format!(
            "  prog_ptr_eq={} io_ptr_eq={}",
            ((c.get_progressive_ptr)(png) == k.user_ptr) as u8,
            ((c.get_io_ptr)(png) == k.user_ptr) as u8
        ));
    }
    if k.pause_info >= 0 {
        let save = k.pause_info;
        do_pause(k, c, png, save, "info");
    }
}

unsafe extern "C" fn row_cb(png: Png, new_row: *mut u8, row_num: u32, pass: c_int) {
    let kp = ctx_ptr();
    if kp.is_null() {
        return;
    }
    let k = &mut *kp;
    let c = &*k.core;
    k.row_calls += 1;
    let n = row_valid(k, pass);
    log(format!(
        "ROW#{} num={row_num} pass={pass} null={} n={n}",
        k.row_calls,
        new_row.is_null() as u8
    ));
    if k.log_rows && !new_row.is_null() {
        log(format!(
            "  src={} slack={}",
            hex(sl(new_row, n)),
            hex(sl(new_row.add(n), SLACK))
        ));
    }
    if k.combine && !k.dst.is_null() && row_num < k.height {
        let dp = k.dst.add(row_num as usize * k.stride);
        (c.progressive_combine_row)(png, dp, new_row);
        log(format!(
            "  dst[{row_num}]={} slack={}",
            hex(sl(dp, k.rowbytes)),
            hex(sl(dp.add(k.rowbytes), SLACK))
        ));
    }
    if k.pause_row >= 0 && k.row_calls == k.pause_row_at {
        let save = k.pause_row;
        do_pause(k, c, png, save, "row");
    }
}

unsafe extern "C" fn end_cb(png: Png, info: Info) {
    let kp = ctx_ptr();
    if kp.is_null() {
        return;
    }
    let k = &mut *kp;
    let c = &*k.core;
    k.end_calls += 1;
    log(format!("END_CB#{}", k.end_calls));
    log_all_info(c, png, info);
}

unsafe extern "C" fn user_chunk_cb(png: Png, chunk: *mut c_void) -> c_int {
    let kp = ctx_ptr();
    if kp.is_null() {
        return 0;
    }
    let k = &mut *kp;
    let c = &*k.core;
    k.chunk_calls += 1;
    if chunk.is_null() {
        log("USER_CHUNK <null>".to_string());
        return k.user_chunk_ret;
    }
    let u = &*(chunk as *const PngUnknownChunk);
    log(format!(
        "USER_CHUNK#{} name={} nul={} size={} loc={} data={} ret={}",
        k.chunk_calls,
        String::from_utf8_lossy(&u.name[..4]),
        u.name[4],
        u.size,
        u.location,
        if u.data.is_null() {
            "<null>".to_string()
        } else {
            hex(sl(u.data, u.size))
        },
        k.user_chunk_ret
    ));
    log(format!(
        "  user_chunk_ptr_eq={}",
        ((c.get_user_chunk_ptr)(png) == k.user_ptr) as u8
    ));
    if k.pause_chunk >= 0 {
        let save = k.pause_chunk;
        do_pause(k, c, png, save, "chunk");
    }
    k.user_chunk_ret
}

// ---------------------------------------------------------------------------
// driver
// ---------------------------------------------------------------------------

/// Feed `input` to `lib` in chunks of `feed` bytes (`feed == 0`: all at once).
fn run_one(lib: &Lib, input: &[u8], feed: usize, k: *mut Ctx) -> Trace {
    session_reset(Vec::new());
    let core = Core::new(lib);
    let mut buf = input.to_vec();
    let bp = buf.as_mut_ptr();
    let blen = buf.len();
    unsafe {
        (*k).core = &core;
        (*k).rowbytes = 0;
        (*k).pixel_depth = 0;
        (*k).row_calls = 0;
        (*k).info_calls = 0;
        (*k).end_calls = 0;
        (*k).chunk_calls = 0;
        (*k).pause_ret = 0;
        (*k).pause_seen = false;
    }
    set_ctx(k);
    let step = if feed == 0 { blen.max(1) } else { feed };
    let rc = protected(|| unsafe {
        let png = (core.create_read_2)(
            VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            cb_error as Cb,
            cb_warning as Cb,
            std::ptr::null_mut(),
            cb_malloc as Cb,
            cb_free as Cb,
        );
        log(format!("create_read2={}", if png.is_null() { 0 } else { 1 }));
        if png.is_null() {
            return;
        }
        (core.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
        if (*k).benign >= 0 {
            (core.set_benign_errors)(png, (*k).benign);
        }
        (core.set_progressive_read_fn)(
            png,
            (*k).user_ptr,
            info_cb as Cb,
            row_cb as Cb,
            end_cb as Cb,
        );
        if (*k).check_ptr {
            log(format!(
                "prog_ptr_eq={} io_ptr_eq={} prog_ptr_null={}",
                ((core.get_progressive_ptr)(png) == (*k).user_ptr) as u8,
                ((core.get_io_ptr)(png) == (*k).user_ptr) as u8,
                (core.get_progressive_ptr)(png).is_null() as u8
            ));
        }
        if (*k).user_chunk_ret > -2 {
            (core.set_read_user_chunk_fn)(png, (*k).user_ptr, user_chunk_cb as Cb);
            log(format!(
                "user_chunk_ptr_eq={}",
                ((core.get_user_chunk_ptr)(png) == (*k).user_ptr) as u8
            ));
        }
        if (*k).keep_default >= 0 {
            (core.set_keep_unknown_chunks)(png, (*k).keep_default, std::ptr::null(), 0);
        }
        if (*k).keep_list >= 0 {
            (core.set_keep_unknown_chunks)(
                png,
                (*k).keep_list,
                KEEP_LIST.as_ptr(),
                (*k).keep_list_n,
            );
        }
        let info = (core.create_info)(png);
        log(format!("create_info={}", if info.is_null() { 0 } else { 1 }));
        if (*k).skip_mode == 2 {
            log(format!("skip_pre={}", (core.process_data_skip)(png)));
        }
        let mut off = 0usize;
        let mut calls = 0u32;
        while off < blen {
            let n = std::cmp::min(step, blen - off);
            (*k).pause_seen = false;
            (*k).pause_ret = 0;
            (core.process_data)(png, info, bp.add(off), n);
            calls += 1;
            let r = (*k).pause_ret;
            let adv = if r >= n {
                if r != 0 {
                    log(format!("NO_PROGRESS r={r} n={n}"));
                }
                n
            } else {
                n - r
            };
            off += adv;
            if (*k).skip_mode == 1 || ((*k).skip_mode == 3 && (*k).pause_seen) {
                let s = (core.process_data_skip)(png);
                log(format!("skip={s}"));
                off = std::cmp::min(off + s as usize, blen);
            }
            if calls > 200_000 {
                log("ITER_LIMIT".to_string());
                break;
            }
        }
        // Data that a `png_process_data_pause(png, 1)` stored inside libpng is
        // only looked at by the *next* call, so hand it a few zero-length
        // buffers (which is also the only way to exercise buffer_size == 0 with
        // a non-empty save buffer).
        for f in 0..4 {
            log(format!("flush{f}"));
            (core.process_data)(png, info, bp, 0);
        }
        log(format!(
            "calls={calls} rows={} info_cb={} end_cb={} chunk_cb={}",
            (*k).row_calls,
            (*k).info_calls,
            (*k).end_calls,
            (*k).chunk_calls
        ));
        if (*k).dump_dst && !(*k).dst.is_null() {
            for y in 0..(*k).height as usize {
                let dp = (*k).dst.add(y * (*k).stride);
                log(format!("IMG[{y}]={}", hex(sl(dp, (*k).stride))));
            }
        }
        let mut p = png;
        let mut i = info;
        (core.destroy_read)(&mut p, &mut i, std::ptr::null_mut());
        log("destroyed".to_string());
    });
    set_ctx(std::ptr::null_mut());
    let live = with_session(|s| s.live_allocs);
    log(format!("live_allocs={live}"));
    let lines = take_log();
    // Debugging aid (never part of the compared trace): PDUMP=1 dumps every
    // trace, with the configuration label and the exact input bytes.
    if std::env::var_os("PDUMP").is_some() {
        LABEL.with(|l| {
            eprintln!(
                "=== [{}] {} rc={rc} lines={} in={}",
                l.borrow(),
                lib.tag,
                lines.len(),
                hex(input)
            )
        });
        for l in &lines {
            eprintln!("{l}");
        }
    }
    Trace {
        lines,
        out: take_out(),
        rc,
    }
}

/// Run one configuration against both libraries and require identical traces.
fn case(label: &str, input: &[u8], feed: usize, k: &mut Ctx) {
    let need = k.combine || k.dump_dst;
    let stride = k.width as usize * 8 + 40;
    let mut dstv: Vec<u8> = if need {
        vec![0u8; k.height as usize * stride]
    } else {
        Vec::new()
    };
    k.stride = stride;
    k.dst_len = dstv.len();
    k.dst = if need {
        dstv.as_mut_ptr()
    } else {
        std::ptr::null_mut()
    };
    LABEL.with(|l| *l.borrow_mut() = label.to_string());
    let kp = k as *mut Ctx;
    diff(label, |lib| {
        unsafe {
            if !(*kp).dst.is_null() {
                std::ptr::write_bytes((*kp).dst, 0, (*kp).dst_len);
            }
        }
        run_one(lib, input, feed, kp)
    });
    k.dst = std::ptr::null_mut();
    k.dst_len = 0;
    drop(dstv);
}

// ---------------------------------------------------------------------------
// P1 — png_process_data: feed sizes × all colour/depth combos
// ---------------------------------------------------------------------------

#[test]
fn p1_feed_sizes_all_combos() {
    ensure_libm();
    // 0 == "the whole file in one call"
    let feeds: &[usize] = &[1, 2, 3, 7, 13, 64, 0];
    for &(ct, bd) in COMBOS {
        for &w in &[1u32, 7, 17] {
            for &h in &[1u32, 5] {
                for &feed in feeds {
                    for &sx in &[0u64, 0x9e37_79b9] {
                        let seed = 0x1_0000
                            + ct as u64 * 97
                            + bd as u64 * 13
                            + w as u64 * 3
                            + h as u64
                            + sx;
                        let png = mk(w, h, ct, bd, 0, seed);
                        let mut k = ctx_of(w, h, 0);
                        case(
                            &format!("P1 ct={ct} bd={bd} w={w} h={h} s={sx} feed={feed}"),
                            &png,
                            feed,
                            &mut k,
                        );
                    }
                }
            }
        }
    }
    // all five row filter types, and png_start_read_image instead of
    // png_read_update_info in the info callback
    for &(ct, bd) in COMBOS {
        for &feed in &[1usize, 13, 0] {
            let png = mk_filters(7, 5, ct, bd, 0, 0x1_5000 + ct as u64 * 31 + bd as u64);
            let mut k = ctx_of(7, 5, 0);
            case(
                &format!("P1f ct={ct} bd={bd} feed={feed}"),
                &png,
                feed,
                &mut k,
            );
            let mut k = ctx_of(7, 5, 0);
            k.start_read_image = true;
            case(
                &format!("P1s ct={ct} bd={bd} feed={feed}"),
                &png,
                feed,
                &mut k,
            );
        }
    }
    // non-interlaced png_progressive_combine_row: a plain row copy
    for &(ct, bd) in COMBOS {
        let png = mk(7, 4, ct, bd, 0, 0x1_9000 + ct as u64 * 17 + bd as u64);
        let mut k = ctx_of(7, 4, 0);
        k.combine = true;
        k.dump_dst = true;
        case(&format!("P1c ct={ct} bd={bd}"), &png, 5, &mut k);
    }
    // png_push_read_sig: every single-byte corruption of the signature, and a
    // stream that simply stops (the push reader keeps the partial data and
    // never reports the end)
    let good = mk(4, 2, 2, 8, 0, 0x1_a000);
    for i in 0..8usize {
        for &feed in &[1usize, 3, 0] {
            let mut bad = good.clone();
            bad[i] ^= 0x20;
            let mut k = ctx_of(4, 2, 0);
            case(&format!("P1sig i={i} feed={feed}"), &bad, feed, &mut k);
        }
    }
    for cut in [1usize, 7, 8, 9, 20, 26, 33, 40] {
        if cut >= good.len() {
            continue;
        }
        for &feed in &[1usize, 5, 0] {
            let mut k = ctx_of(4, 2, 0);
            case(
                &format!("P1cut cut={cut} feed={feed}"),
                &good[..cut],
                feed,
                &mut k,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// P2 — interlaced input + png_progressive_combine_row
// ---------------------------------------------------------------------------

#[test]
fn p2_interlaced_combine() {
    ensure_libm();
    let feeds: &[usize] = &[1, 5, 64, 0];
    // Shapes chosen to hit every special case of `png_push_process_row`: the
    // `height <= 4` branches, and the widths that make pass 1/3/5 empty
    // (width < 5 / < 3 / < 2).
    let shapes: &[(u32, u32)] = &[
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 5),
        (8, 8),
        (9, 7),
        (2, 7),
        (7, 2),
        (1, 8),
        (8, 1),
    ];
    for &(ct, bd) in COMBOS {
        for &(w, h) in shapes {
            for &feed in feeds {
                let seed = 0x2_0000 + ct as u64 * 89 + bd as u64 * 7 + w as u64 + h as u64;
                let png = mk(w, h, ct, bd, 1, seed);
                // (a) the documented mode: interlace handling on, every
                //     intermediate destination row logged
                let mut k = ctx_of(w, h, 1);
                k.combine = true;
                k.dump_dst = true;
                case(
                    &format!("P2 ct={ct} bd={bd} w={w} h={h} feed={feed}"),
                    &png,
                    feed,
                    &mut k,
                );
                // (b) interlace handling off: the raw sub-image rows
                let mut k = ctx_of(w, h, 1);
                k.interlace_handling = false;
                case(
                    &format!("P2raw ct={ct} bd={bd} w={w} h={h} feed={feed}"),
                    &png,
                    feed,
                    &mut k,
                );
            }
        }
    }
    // all five row filter types over the interlaced passes
    for &(ct, bd) in COMBOS {
        for &feed in &[3usize, 0] {
            let png = mk_filters(9, 7, ct, bd, 1, 0x2_8000 + ct as u64 * 13 + bd as u64);
            let mut k = ctx_of(9, 7, 1);
            k.combine = true;
            k.dump_dst = true;
            case(
                &format!("P2f ct={ct} bd={bd} feed={feed}"),
                &png,
                feed,
                &mut k,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// P3 — png_process_data_pause
// ---------------------------------------------------------------------------

#[test]
fn p3_process_data_pause() {
    ensure_libm();
    // (a) pause inside the info callback (the documented place to do it: the
    //     application has just learned the image geometry and may need to
    //     allocate).  save = 0 makes the caller re-supply the unprocessed
    //     bytes, save = 1 leaves them inside libpng.
    for &(ct, bd) in COMBOS {
        for il in [0u8, 1] {
            for save in [0i32, 1] {
                for &feed in &[1usize, 3, 13, 64, 0] {
                    let seed = 0x3_0000 + ct as u64 * 41 + bd as u64 * 5 + il as u64;
                    let png = mk(5, 5, ct, bd, il, seed);
                    let mut k = ctx_of(5, 5, il);
                    k.pause_info = save;
                    k.combine = il != 0;
                    case(
                        &format!("P3i ct={ct} bd={bd} il={il} save={save} feed={feed}"),
                        &png,
                        feed,
                        &mut k,
                    );
                }
            }
        }
    }
    // (b) an image with a big chunk before IDAT, so that at the moment of the
    //     pause libpng holds a large save_buffer
    for save in [0i32, 1] {
        for &feed in &[1usize, 7, 100, 0] {
            let mut r = Rng::new(0x3_7777);
            let png = Builder::new(8, 4, 8, 2)
                .add(b"tEXt", text_chunk(b"Key", &r.bytes(600)))
                .add(b"biGx", r.bytes(400))
                .build_valid(0x3_7777);
            let mut k = ctx_of(8, 4, 0);
            k.pause_info = save;
            case(
                &format!("P3big save={save} feed={feed}"),
                &png,
                feed,
                &mut k,
            );
        }
    }
    // (c) pause inside the user chunk callback: a pause exactly at a chunk
    //     boundary (png_push_read_chunk touches no buffer accounting after
    //     png_handle_unknown returns, so this one is well defined for both
    //     save values).
    for &(ct, bd) in &[(0u8, 8u8), (3, 4), (6, 16)] {
        let png = mk_ancillary(5, 3, ct, bd, 0, 0x3_e000 + ct as u64);
        for save in [0i32, 1] {
            for &feed in &[1usize, 7, 0] {
                for &keep in &[PNG_HANDLE_CHUNK_NEVER, PNG_HANDLE_CHUNK_ALWAYS] {
                    let mut tag = [0u8; 4];
                    let mut k = ctx_of(5, 3, 0);
                    k.keep_default = keep;
                    k.user_chunk_ret = 1;
                    k.user_ptr = tag.as_mut_ptr() as *mut c_void;
                    k.pause_chunk = save;
                    case(
                        &format!("P3c ct={ct} bd={bd} keep={keep} save={save} feed={feed}"),
                        &png,
                        feed,
                        &mut k,
                    );
                }
            }
        }
    }
    // (d) pause inside the row callback, save = 0.
    //
    //     Two restrictions, both dictated by what the C actually does:
    //
    //     * The whole file is fed in a single call.  `png_push_read_IDAT`
    //       subtracts the amount it just handed to zlib from
    //       `png_struct::buffer_size` *after* `png_process_IDAT_data` (and
    //       therefore after the row callback) returned, so the pause -- which
    //       sets buffer_size to 0 -- makes that subtraction wrap around.  With
    //       the whole file in the current buffer every subsequent read still
    //       comes out of the caller's buffer and the reader runs to IEND, i.e.
    //       the C behaviour is well defined; with a partial feed libpng would
    //       instead read from an exhausted buffer.
    //     * save = 1 is not used: png_push_save_buffer() also zeroes
    //       current_buffer_size, so the same subtraction wraps *that* around
    //       too and the C reader walks off the end of the caller's buffer -
    //       there is no defined behaviour to compare against.
    //
    //     NOTE (divergence, do not "fix" by removing the test): the C reader
    //     completes the image here, because `png_ptr->buffer_size -= save_size`
    //     is a well defined unsigned wraparound in C.  `src/pngpread.rs` uses a
    //     plain `-=` for it (lines 454 and 483), which panics under the debug
    //     profile's overflow checks and aborts the process, so this
    //     configuration currently kills the test binary at the first case.
    for &(ct, bd) in &[(0u8, 8u8), (0, 1), (2, 8), (3, 4), (4, 16), (6, 16)] {
        for il in [0u8, 1] {
            for at in [1u32, 3] {
                let png = mk(5, 5, ct, bd, il, 0x3_9000 + ct as u64 * 7 + bd as u64);
                let mut k = ctx_of(5, 5, il);
                k.pause_row = 0;
                k.pause_row_at = at;
                case(
                    &format!("P3r ct={ct} bd={bd} il={il} save=0 at={at}"),
                    &png,
                    0,
                    &mut k,
                );
            }
        }
    }
    // (e) pause in both the info and the row callback
    for at in [1u32, 2] {
        let png = mk(7, 5, 6, 8, 0, 0x3_c000);
        let mut k = ctx_of(7, 5, 0);
        k.pause_info = 0;
        k.pause_row = 0;
        k.pause_row_at = at;
        case(&format!("P3ir at={at}"), &png, 0, &mut k);
    }
}

// ---------------------------------------------------------------------------
// P4 — png_process_data_skip
// ---------------------------------------------------------------------------

#[test]
fn p4_process_data_skip() {
    ensure_libm();
    let mut r = Rng::new(0x4_0000);
    // a large unknown private chunk and a large tEXt chunk
    let big_unknown = Builder::new(8, 4, 8, 6)
        .add(b"biGx", r.bytes(700))
        .build_valid(0x4_0001);
    let big_text = Builder::new(8, 4, 4, 0)
        .add(b"tEXt", text_chunk(b"Comment", &r.bytes(900)))
        .build_valid(0x4_0002);
    let both = Builder::new(6, 3, 8, 3)
        .add(b"PLTE", palette_for(8, 0x4_0003))
        .add(b"biGx", r.bytes(300))
        .add(b"tEXt", text_chunk(b"K", &r.bytes(300)))
        .build_valid(0x4_0003);
    // the same, but with the large chunks *after* IDAT
    let after = insert_after_idat(
        &insert_after_idat(
            &Builder::new(8, 4, 8, 6).build_valid(0x4_0004),
            Chunk::new(b"biGx", r.bytes(500)),
        ),
        Chunk::new(b"tEXt", text_chunk(b"Post", &r.bytes(500))),
    );
    let inputs: &[(&str, &Vec<u8>)] = &[
        ("unknown", &big_unknown),
        ("text", &big_text),
        ("both", &both),
        ("after", &after),
    ];
    for &(name, png) in inputs {
        for &benign in &[-1i32, 0, 1] {
            for &feed in &[1usize, 13, 0] {
                for &(smode, pause) in &[(1u8, -1i32), (3, 0), (3, 1), (2, -1)] {
                    let mut k = ctx_of(if name == "both" { 6 } else { 8 }, if name == "both" { 3 } else { 4 }, 0);
                    k.benign = benign;
                    k.skip_mode = smode;
                    k.pause_info = pause;
                    case(
                        &format!(
                            "P4 in={name} benign={benign} feed={feed} skip={smode} pause={pause}"
                        ),
                        png,
                        feed,
                        &mut k,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// P5 — transforms set in the info callback
// ---------------------------------------------------------------------------

#[test]
fn p5_transforms_in_info_callback() {
    ensure_libm();
    let sets: &[u32] = &[
        T_EXPAND,
        T_GRAY_TO_RGB,
        T_STRIP16,
        T_PACKING,
        T_GAMMA,
        T_BGR,
        T_EXPAND | T_GRAY_TO_RGB,
        T_EXPAND | T_STRIP16 | T_BGR,
        T_PACKING | T_GRAY_TO_RGB,
        T_EXPAND | T_EXPAND16 | T_SWAP,
        T_GAMMA | T_EXPAND | T_GRAY_TO_RGB | T_BGR | T_STRIP16,
    ];
    for &m in sets {
        for &(ct, bd) in COMBOS {
            for &feed in &[7usize, 0] {
                let seed = 0x5_0000 + ct as u64 * 53 + bd as u64 * 3 + m as u64;
                let png = mk_rich(7, 3, ct, bd, 0, seed);
                let mut k = ctx_of(7, 3, 0);
                k.tflags = m;
                case(
                    &format!("P5 t={} ct={ct} bd={bd} feed={feed}", tname(m)),
                    &png,
                    feed,
                    &mut k,
                );
            }
        }
    }
    // the same transforms over an interlaced image, combining into the
    // destination buffer
    for &m in &[
        T_EXPAND,
        T_GRAY_TO_RGB,
        T_PACKING,
        T_EXPAND | T_GRAY_TO_RGB | T_BGR,
        T_GAMMA | T_STRIP16,
    ] {
        for &(ct, bd) in COMBOS {
            let seed = 0x5_8000 + ct as u64 * 59 + bd as u64 * 3 + m as u64;
            let png = mk_rich(9, 7, ct, bd, 1, seed);
            let mut k = ctx_of(9, 7, 1);
            k.tflags = m;
            k.combine = true;
            k.dump_dst = true;
            case(
                &format!("P5i t={} ct={ct} bd={bd}", tname(m)),
                &png,
                11,
                &mut k,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// P6 — IDAT split into many chunks, zero-length IDATs
// ---------------------------------------------------------------------------

/// Insert `c` directly after the (first) IDAT chunk of `png`.
fn insert_after_idat(png: &[u8], c: Chunk) -> Vec<u8> {
    let cs = pngbuild::split(png);
    let idx = pngbuild::find(&cs, b"IDAT").expect("no IDAT");
    let mut out: Vec<Chunk> = cs[..=idx].to_vec();
    out.push(c);
    out.extend(cs[idx + 1..].iter().cloned());
    pngbuild::join(&out)
}

/// Rebuild `png` with its (single) IDAT chunk cut into `part`-byte IDAT chunks;
/// `zero` inserts zero-length IDAT chunks: 1 = before the run, 2 = in the
/// middle, 3 = after the run, 4 = all three.
fn resplit_idat(png: &[u8], part: usize, zero: u8) -> Vec<u8> {
    let cs = pngbuild::split(png);
    let idx = pngbuild::find(&cs, b"IDAT").expect("no IDAT");
    let data = cs[idx].data.clone();
    let mut out: Vec<Chunk> = cs[..idx].to_vec();
    let empty = Chunk::new(b"IDAT", Vec::new());
    if zero == 1 || zero == 4 {
        out.push(empty.clone());
    }
    let parts: Vec<Vec<u8>> = if part == 0 {
        vec![data.clone()]
    } else {
        data.chunks(part).map(|c| c.to_vec()).collect()
    };
    let mid = parts.len() / 2;
    for (i, p) in parts.iter().enumerate() {
        if i == mid && (zero == 2 || zero == 4) {
            out.push(empty.clone());
        }
        out.push(Chunk::new(b"IDAT", p.clone()));
    }
    if zero == 3 || zero == 4 {
        out.push(empty.clone());
    }
    out.extend(cs[idx + 1..].iter().cloned());
    pngbuild::join(&out)
}

#[test]
fn p6_multi_idat() {
    ensure_libm();
    let combos: &[(u8, u8)] = &[(0, 1), (0, 8), (2, 16), (3, 4), (6, 8)];
    for &(ct, bd) in combos {
        for il in [0u8, 1] {
            for &part in &[1usize, 2, 3] {
                for zero in [0u8, 1, 2, 3, 4] {
                    for &feed in &[7usize, 0] {
                        let base = mk(6, 4, ct, bd, il, 0x6_0000 + ct as u64 * 11 + bd as u64);
                        let png = resplit_idat(&base, part, zero);
                        let mut k = ctx_of(6, 4, il);
                        k.combine = il != 0;
                        case(
                            &format!(
                                "P6 ct={ct} bd={bd} il={il} part={part} zero={zero} feed={feed}"
                            ),
                            &png,
                            feed,
                            &mut k,
                        );
                    }
                }
            }
        }
    }
    // only zero-length IDATs around one intact IDAT chunk
    for &(ct, bd) in combos {
        for zero in [1u8, 2, 3, 4] {
            for &feed in &[1usize, 0] {
                let base = mk(4, 2, ct, bd, 0, 0x6_9000 + ct as u64);
                let png = resplit_idat(&base, 0, zero);
                let mut k = ctx_of(4, 2, 0);
                case(
                    &format!("P6z ct={ct} bd={bd} zero={zero} feed={feed}"),
                    &png,
                    feed,
                    &mut k,
                );
            }
        }
    }
    // The IDAT structure checks that only the progressive reader performs
    // ("Missing IHDR/PLTE before IDAT", "Not enough compressed data",
    // "Too many IDATs found", the two "Extra ... data in IDAT" messages).
    let seed = 0x6_f000u64;
    let plain = mk(4, 2, 0, 8, 0, seed);
    let cs = pngbuild::split(&plain);
    let ii = pngbuild::find(&cs, b"IDAT").unwrap();
    // (1) a palette image with no PLTE chunk
    let no_plte = Builder::new(4, 2, 4, 3).build_valid(seed);
    // (2) no IHDR at all: the first chunk is IDAT
    let no_ihdr = pngbuild::join(&cs[1..]);
    // (3) a complete zlib stream, then tEXt, then one more IDAT
    let mut extra: Vec<Chunk> = cs[..=ii].to_vec();
    extra.push(Chunk::new(b"tEXt", text_chunk(b"Mid", b"between idats")));
    extra.push(Chunk::new(b"IDAT", vec![0x11, 0x22, 0x33]));
    extra.extend(cs[ii + 1..].iter().cloned());
    let extra_idat = pngbuild::join(&extra);
    // (4) the zlib stream cut short
    let mut trunc: Vec<Chunk> = cs.clone();
    let n = trunc[ii].data.len();
    trunc[ii].data.truncate(n - 5);
    let trunc_zlib = pngbuild::join(&trunc);
    // (5) bytes appended after the zlib end code
    let mut tail: Vec<Chunk> = cs.clone();
    tail[ii].data.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
    let tail_bytes = pngbuild::join(&tail);
    // (6) one row too many in the decompressed stream
    let raw3 = Builder::new(4, 3, 8, 0).raw_rows(seed);
    let extra_row = Builder::new(4, 2, 8, 0).build(&raw3, 0);
    let structural: &[(&str, &Vec<u8>, u32, u32)] = &[
        ("no_plte", &no_plte, 4, 2),
        ("no_ihdr", &no_ihdr, 4, 2),
        ("extra_idat", &extra_idat, 4, 2),
        ("trunc_zlib", &trunc_zlib, 4, 2),
        ("tail_bytes", &tail_bytes, 4, 2),
        ("extra_row", &extra_row, 4, 2),
    ];
    for &(name, png, w, h) in structural {
        for &feed in &[1usize, 7, 0] {
            let mut k = ctx_of(w, h, 0);
            case(&format!("P6x {name} feed={feed}"), png, feed, &mut k);
        }
    }
}

// ---------------------------------------------------------------------------
// P7 — ancillary + unknown chunks, keep values, user chunk callback
// ---------------------------------------------------------------------------

#[test]
fn p7_ancillary_and_unknown_chunks() {
    ensure_libm();
    let keeps: &[c_int] = &[
        -1,
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ];
    for &(ct, bd) in &[(0u8, 8u8), (3, 4), (6, 16), (2, 8), (4, 16)] {
        let png = mk_ancillary(5, 3, ct, bd, 0, 0x7_0000 + ct as u64 * 7 + bd as u64);
        for &keep in keeps {
            for &ucb in &[-2i32, -1, 0, 1] {
                for &feed in &[1usize, 13, 0] {
                    let mut tag = [0u8; 4];
                    let mut k = ctx_of(5, 3, 0);
                    k.keep_default = keep;
                    k.user_chunk_ret = ucb;
                    k.user_ptr = tag.as_mut_ptr() as *mut c_void;
                    case(
                        &format!("P7 ct={ct} bd={bd} keep={keep} ucb={ucb} feed={feed}"),
                        &png,
                        feed,
                        &mut k,
                    );
                }
            }
        }
        // a keep *list* naming known chunks (gAMA, tEXt) plus a private one
        for &keep in &[
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
        ] {
            for &ucb in &[-2i32, 1] {
                let mut tag = [0u8; 4];
                let mut k = ctx_of(5, 3, 0);
                k.keep_list = keep;
                k.keep_list_n = 3;
                k.user_chunk_ret = ucb;
                k.user_ptr = tag.as_mut_ptr() as *mut c_void;
                case(
                    &format!("P7list ct={ct} bd={bd} keep={keep} ucb={ucb}"),
                    &png,
                    9,
                    &mut k,
                );
                // ... and a global default *plus* a per-chunk list
                for &dflt in &[PNG_HANDLE_CHUNK_NEVER, PNG_HANDLE_CHUNK_ALWAYS] {
                    let mut k = ctx_of(5, 3, 0);
                    k.keep_default = dflt;
                    k.keep_list = keep;
                    k.keep_list_n = 3;
                    k.user_chunk_ret = ucb;
                    k.user_ptr = tag.as_mut_ptr() as *mut c_void;
                    case(
                        &format!("P7both ct={ct} bd={bd} dflt={dflt} keep={keep} ucb={ucb}"),
                        &png,
                        0,
                        &mut k,
                    );
                }
            }
        }
    }
    // interlaced, with the chunk cache and malloc limits left at default
    let png = mk_ancillary(9, 7, 2, 8, 1, 0x7_9000);
    for &keep in &[PNG_HANDLE_CHUNK_NEVER, PNG_HANDLE_CHUNK_ALWAYS] {
        for &feed in &[1usize, 0] {
            let mut k = ctx_of(9, 7, 1);
            k.keep_default = keep;
            k.combine = true;
            case(&format!("P7il keep={keep} feed={feed}"), &png, feed, &mut k);
        }
    }
}

// ---------------------------------------------------------------------------
// P8 — png_get_progressive_ptr
// ---------------------------------------------------------------------------

#[test]
fn p8_progressive_ptr() {
    ensure_libm();
    // The pointer round trip is checked inside every callback and right after
    // png_set_progressive_read_fn; only the equality boolean is logged.
    for &(ct, bd) in &[(0u8, 8u8), (3, 2), (6, 16)] {
        for il in [0u8, 1] {
            for &feed in &[3usize, 0] {
                let png = mk(5, 5, ct, bd, il, 0x8_0000 + ct as u64 * 3 + bd as u64);
                let mut tag = [0x5au8; 16];
                let mut k = ctx_of(5, 5, il);
                k.check_ptr = true;
                k.user_ptr = tag.as_mut_ptr() as *mut c_void;
                case(
                    &format!("P8 ct={ct} bd={bd} il={il} feed={feed}"),
                    &png,
                    feed,
                    &mut k,
                );
                // and with a NULL user pointer
                let mut k = ctx_of(5, 5, il);
                k.check_ptr = true;
                k.user_ptr = std::ptr::null_mut();
                case(
                    &format!("P8null ct={ct} bd={bd} il={il} feed={feed}"),
                    &png,
                    feed,
                    &mut k,
                );
            }
        }
    }
    // png_get_progressive_ptr(NULL) must be NULL, and the pointer must survive
    // a second png_set_progressive_read_fn with different callbacks.
    diff("P8 direct", |lib| {
        session_reset(Vec::new());
        let c = Core::new(lib);
        let mut tag = [7u8; 8];
        let tp = tag.as_mut_ptr() as *mut c_void;
        let rc = protected(|| unsafe {
            log(format!(
                "null_png={}",
                (c.get_progressive_ptr)(std::ptr::null_mut()).is_null() as u8
            ));
            let png = (c.create_read)(
                VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                cb_error as Cb,
                cb_warning as Cb,
            );
            log(format!("create={}", if png.is_null() { 0 } else { 1 }));
            if png.is_null() {
                return;
            }
            (c.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
            log(format!(
                "fresh_null={}",
                (c.get_progressive_ptr)(png).is_null() as u8
            ));
            (c.set_progressive_read_fn)(
                png,
                tp,
                info_cb as Cb,
                row_cb as Cb,
                end_cb as Cb,
            );
            log(format!("eq1={}", ((c.get_progressive_ptr)(png) == tp) as u8));
            (c.set_progressive_read_fn)(
                png,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            log(format!(
                "after_null_eq={} is_null={}",
                ((c.get_progressive_ptr)(png) == tp) as u8,
                (c.get_progressive_ptr)(png).is_null() as u8
            ));
            (c.set_progressive_read_fn)(png, tp, info_cb as Cb, row_cb as Cb, end_cb as Cb);
            log(format!("eq2={}", ((c.get_progressive_ptr)(png) == tp) as u8));
            let mut p = png;
            (c.destroy_read)(&mut p, std::ptr::null_mut(), std::ptr::null_mut());
            log("destroyed".to_string());
        });
        Trace {
            lines: take_log(),
            out: take_out(),
            rc,
        }
    });
}
