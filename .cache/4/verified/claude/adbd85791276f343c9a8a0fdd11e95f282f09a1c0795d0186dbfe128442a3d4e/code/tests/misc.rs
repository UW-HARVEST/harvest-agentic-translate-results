//! Phase D — the odds and ends.
//!
//! Covers CONFIGS.md rows
//!   C-23  `misc::memory`             (png_malloc / calloc / free / arrays)
//!   C-64  `misc::io_state`           (png_get_io_state / io_chunk_type / init_io)
//!   C-142 `misc::crc_action`         (png_set_crc_action, 36 combinations)
//!   C-143 `misc::user_limits`        (png_set_user_limits & the chunk limits)
//!   C-144 `misc::custom_alloc`       (png_create_*_struct_2, png_set_mem_fn)
//!   C-146 `misc::status_callbacks`   (png_set_read/write_status_fn)
//!   C-148 `misc::options`            (png_set_option, png_permit_mng_features)
//!   C-149 `misc::row_number`         (png_get_current_row/pass_number)
//!   C-150 `misc::longjmp_fn`         (png_set_longjmp_fn / png_longjmp)
//!   C-151 `misc::struct_lifecycle`   (create / init / destroy, every order)
//!   C-152 `misc::grayscale_palette`  (png_build_grayscale_palette)
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null, null_mut};
use std::sync::atomic::{AtomicUsize, Ordering};

/* ------------------------------------------------------------------ */
/* bookkeeping: how many differential comparisons each row performs    */
/* ------------------------------------------------------------------ */

static COMPARISONS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static NCMP: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn bump() {
    NCMP.with(|c| c.set(c.get() + 1));
    COMPARISONS.fetch_add(1, Ordering::Relaxed);
}

/// Compare C against Rust for one scenario (and count it).
#[track_caller]
fn same<F>(case: &str, f: F)
where
    F: FnMut(&Api) -> Outcome,
{
    bump();
    assert_same(case, f);
}

/// Same, but each side runs in its own `fork()`ed child so that a crash or an
/// `abort()` becomes a compared observation instead of the end of the test run.
#[track_caller]
fn samef<F>(case: &str, f: F)
where
    F: Fn(&Api) -> String,
{
    bump();
    assert_same_forked(case, f);
}

fn report(tag: &str) {
    eprintln!(
        "[misc::{}] differential comparisons: {}",
        tag,
        NCMP.with(|c| c.get())
    );
}

/* ------------------------------------------------------------------ */
/* small utilities                                                     */
/* ------------------------------------------------------------------ */

extern "C" {
    #[link_name = "malloc"]
    fn xmalloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn xfree(p: *mut c_void);
    /* The stdio bits png_init_io needs.  There is no `libc` crate offline, so
     * declare exactly what is used here. */
    fn tmpfile() -> *mut c_void;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn rewind(f: *mut c_void);
    fn fread(p: *mut u8, sz: usize, n: usize, f: *mut c_void) -> usize;
}

/// A read/write `FILE*` for `png_init_io` to use.  `tmpfile()` is preferred but
/// is not always permitted to create files in `/tmp`, so fall back to a named
/// file in the test scratch directory.
fn temp_file(tag: &str) -> *mut c_void {
    unsafe {
        let f = tmpfile();
        if !f.is_null() {
            return f;
        }
        let p = scratch_dir().join(format!("misc-{}.tmp", tag));
        let cp = cs(p.to_str().unwrap());
        let mode = cs("w+b");
        fopen(cp.as_ptr(), mode.as_ptr())
    }
}

/// Run `f` once, against the C library only, with a fresh `Tls` — used to build
/// the input datastreams the differential scenarios then feed to both libraries.
fn with_c<T>(f: impl FnOnce(&Api) -> T) -> T {
    let l = libs();
    let mut state = Box::new(Tls::default());
    let prev = set_tls(&mut *state as *mut Tls);
    let prev_api = set_cur_api(&l.c as *const Api);
    let r = f(&l.c);
    set_cur_api(prev_api);
    set_tls(prev);
    r
}

/// `png_get_io_chunk_type()` rendered readably.
fn chunk_str(v: u32) -> String {
    let b = v.to_be_bytes();
    if b.iter().all(|c| c.is_ascii_alphanumeric()) {
        String::from_utf8_lossy(&b).into_owned()
    } else {
        format!("{:#010x}", v)
    }
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A valid zlib stream built out of "stored" deflate blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78u8, 0x01];
    if data.is_empty() {
        v.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut i = 0;
        while i < data.len() {
            let n = (data.len() - i).min(65535);
            let last = i + n == data.len();
            v.push(u8::from(last));
            v.extend_from_slice(&(n as u16).to_le_bytes());
            v.extend_from_slice(&(!(n as u16)).to_le_bytes());
            v.extend_from_slice(&data[i..i + n]);
            i += n;
        }
    }
    v.extend_from_slice(&adler32(data).to_be_bytes());
    v
}

/// A 260-byte ICC profile libpng accepts.
fn icc_profile() -> Vec<u8> {
    let mut v = vec![0u8; 260];
    let n = v.len() as u32;
    v[0..4].copy_from_slice(&n.to_be_bytes());
    v[8] = 2;
    v[12..16].copy_from_slice(b"mntr");
    v[16..20].copy_from_slice(b"RGB ");
    v[20..24].copy_from_slice(b"XYZ ");
    v[36..40].copy_from_slice(b"acsp");
    v[64..68].copy_from_slice(&0u32.to_be_bytes());
    v[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    v[128..132].copy_from_slice(&1u32.to_be_bytes());
    v[132..136].copy_from_slice(b"rXYZ");
    v[136..140].copy_from_slice(&144u32.to_be_bytes());
    v[140..144].copy_from_slice(&8u32.to_be_bytes());
    v
}

/// Replace the first chunk called `name` with one that has a broken CRC.
fn break_crc(png: &[u8], name: &str) -> Vec<u8> {
    for (n, r) in split_chunks(png) {
        if n == name {
            let mut nm = [0u8; 4];
            nm.copy_from_slice(&png[r.start + 4..r.start + 8]);
            let data = png[r.start + 8..r.end - 4].to_vec();
            let mut v = png[..r.start].to_vec();
            v.extend_from_slice(&chunk_bad_crc(&nm, &data));
            v.extend_from_slice(&png[r.end..]);
            return v;
        }
    }
    panic!("no {} chunk to corrupt", name);
}

/// Overwrite the IHDR `filter_method` byte, keeping the CRC correct.
fn set_ihdr_filter(png: &[u8], f: u8) -> Vec<u8> {
    for (n, r) in split_chunks(png) {
        if n == "IHDR" {
            let mut d = png[r.start + 8..r.end - 4].to_vec();
            d[11] = f;
            let mut v = png[..r.start].to_vec();
            v.extend_from_slice(&chunk(b"IHDR", &d));
            v.extend_from_slice(&png[r.end..]);
            return v;
        }
    }
    panic!("no IHDR chunk");
}

/// Rewrite the (single) IDAT chunk's payload, keeping the CRC correct.
fn patch_idat(png: &[u8], f: impl Fn(&mut Vec<u8>)) -> Vec<u8> {
    let idats: Vec<_> = split_chunks(png)
        .into_iter()
        .filter(|(n, _)| n == "IDAT")
        .collect();
    assert_eq!(idats.len(), 1, "expected exactly one IDAT");
    let r = idats[0].1.clone();
    let mut d = png[r.start + 8..r.end - 4].to_vec();
    f(&mut d);
    let mut v = png[..r.start].to_vec();
    v.extend_from_slice(&chunk(b"IDAT", &d));
    v.extend_from_slice(&png[r.end..]);
    v
}

/* ------------------------------------------------------------------ */
/* callbacks used by more than one row                                 */
/* ------------------------------------------------------------------ */

/// `png_set_read_fn` callback that samples the IO state at every single call.
unsafe extern "C" fn io_read_cb(png: *mut PngStruct, data: *mut u8, len: usize) {
    let api = cur_api();
    log(format!(
        "R state={:#06x} chunk={} len={}",
        (api.png_get_io_state)(png),
        chunk_str((api.png_get_io_chunk_type)(png)),
        len
    ));
    read_cb(png, data, len);
}

/// `png_set_write_fn` callback that samples the IO state at every single call.
unsafe extern "C" fn io_write_cb(png: *mut PngStruct, data: *mut u8, len: usize) {
    let api = cur_api();
    log(format!(
        "W state={:#06x} chunk={} len={}",
        (api.png_get_io_state)(png),
        chunk_str((api.png_get_io_chunk_type)(png)),
        len
    ));
    write_cb(png, data, len);
}

/// A `png_longjmp_ptr` that is only ever *stored*, never called (except by the
/// deliberate `png_longjmp` abort cases, where returning is exactly the point).
unsafe extern "C" fn dummy_longjmp(_jb: *mut JmpBuf, val: c_int) {
    log(format!("dummy_longjmp({})", val));
}

/* --- an allocator with a budget: the (N+1)'th allocation fails --- */

thread_local! {
    static BUDGET: std::cell::Cell<i64> = const { std::cell::Cell::new(-1) };
}

unsafe extern "C" fn budget_malloc(_png: *mut PngStruct, size: usize) -> *mut c_void {
    let t = tls();
    t.alloc_serial += 1;
    let serial = t.alloc_serial as i64;
    let budget = BUDGET.with(|c| c.get());
    if budget >= 0 && serial > budget {
        log(format!("alloc #{} size={} -> refused", serial, size));
        return null_mut();
    }
    let p = xmalloc(size.max(1));
    log(format!("alloc #{} size={} ok={}", serial, size, !p.is_null()));
    p
}

unsafe extern "C" fn budget_free(_png: *mut PngStruct, p: *mut c_void) {
    if !p.is_null() {
        tls().counter += 1;
        xfree(p);
    }
}

/// A stable, non-heap `mem_ptr` so that `png_get_mem_ptr` can be compared for
/// equality (never compare raw heap addresses between the two libraries).
static MEM_TAG: u8 = 0xa5;
fn mem_tag() -> *mut c_void {
    &MEM_TAG as *const u8 as *mut c_void
}

/* ------------------------------------------------------------------ */
/* test images / datastreams                                           */
/* ------------------------------------------------------------------ */

fn rgb_file(w: u32, h: u32, seed: u64) -> Vec<u8> {
    with_c(|api| unsafe {
        let mut rng = Rng::new(seed);
        let img = Img::random(&mut rng, w, h, PNG_COLOR_TYPE_RGB, 8);
        write_plain(api, &img, &WriteOpts::default()).bytes
    })
}

/// A file with several ancillary chunks around the image data, so that the IO
/// state trace and the CRC-action matrix have something to chew on.
fn multi_chunk_file(palette: bool) -> Vec<u8> {
    with_c(|api| unsafe {
        let mut rng = Rng::new(0x5c0f);
        let mut img = if palette {
            let mut i = Img::random(&mut rng, 9, 4, PNG_COLOR_TYPE_PALETTE, 8);
            i.palette.truncate(32);
            for r in i.rows.iter_mut() {
                for b in r.iter_mut() {
                    *b %= 32;
                }
            }
            i
        } else {
            Img::random(&mut rng, 9, 4, PNG_COLOR_TYPE_RGB, 8)
        };
        img.interlace = PNG_INTERLACE_NONE;
        let wr = write_image(api, &img, &WriteOpts::default(), &mut |api, png, info| {
            (api.png_set_gAMA_fixed)(png, info, 45455);
            (api.png_set_pHYs)(png, info, 300, 300, PNG_RESOLUTION_METER);
            let sb = png_color_8 {
                red: 8,
                green: 8,
                blue: 8,
                gray: 8,
                alpha: 8,
            };
            (api.png_set_sBIT)(png, info, &sb);
            let t = png_time {
                year: 2024,
                month: 3,
                day: 4,
                hour: 5,
                minute: 6,
                second: 7,
            };
            (api.png_set_tIME)(png, info, &t);
            let key = cs("Comment");
            let txt = cs("a moderately long comment so the chunk is not tiny");
            let te = png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: key.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: null_mut(),
                lang_key: null_mut(),
            };
            (api.png_set_text)(png, info, &te, 1);
            if palette {
                let trans: Vec<u8> = (0..16u8).map(|i| i * 16).collect();
                (api.png_set_tRNS)(png, info, trans.as_ptr(), 16, null());
                let hist: Vec<u16> = (0..32u16).map(|i| i * 3).collect();
                (api.png_set_hIST)(png, info, hist.as_ptr());
            }
            (api.png_set_oFFs)(png, info, -5, 7, PNG_OFFSET_PIXEL);
        });
        wr.bytes
    })
}

/// The same, plus the chunks whose storage `png_free_data` has to release
/// (iCCP, sPLT, sCAL, pCAL, eXIf and an unknown chunk).
fn rich_file() -> Vec<u8> {
    let base = with_c(|api| unsafe {
        let mut rng = Rng::new(0x11c);
        let mut img = Img::random(&mut rng, 8, 4, PNG_COLOR_TYPE_PALETTE, 8);
        img.palette.truncate(64);
        for r in img.rows.iter_mut() {
            for b in r.iter_mut() {
                *b %= 64;
            }
        }
        let wr = write_image(api, &img, &WriteOpts::default(), &mut |api, png, info| {
            (api.png_set_gAMA_fixed)(png, info, 45455);
            let trans: Vec<u8> = (0..8u8).map(|i| i * 32).collect();
            (api.png_set_tRNS)(png, info, trans.as_ptr(), 8, null());
            let hist: Vec<u16> = (0..64u16).map(|i| i + 1).collect();
            (api.png_set_hIST)(png, info, hist.as_ptr());
            let k1 = cs("Title");
            let v1 = cs("first text item");
            let k2 = cs("Author");
            let v2 = cs("second text item");
            let k3 = cs("Comment");
            let v3 = cs("third text item");
            let mk = |k: &std::ffi::CString, v: &std::ffi::CString| png_text {
                compression: PNG_TEXT_COMPRESSION_NONE,
                key: k.as_ptr() as *mut c_char,
                text: v.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: 0,
                lang: null_mut(),
                lang_key: null_mut(),
            };
            let ts = [mk(&k1, &v1), mk(&k2, &v2), mk(&k3, &v3)];
            (api.png_set_text)(png, info, ts.as_ptr(), 3);
            let iccname = cs("an icc profile");
            let prof = icc_profile();
            (api.png_set_iCCP)(
                png,
                info,
                iccname.as_ptr(),
                0,
                prof.as_ptr(),
                prof.len() as u32,
            );
            let sname = cs("suggested palette one");
            let sname2 = cs("suggested palette two");
            let mut ents = [png_sPLT_entry {
                red: 1,
                green: 2,
                blue: 3,
                alpha: 4,
                frequency: 5,
            }; 3];
            let mut ents2 = [png_sPLT_entry {
                red: 9,
                green: 8,
                blue: 7,
                alpha: 6,
                frequency: 5,
            }; 2];
            let sps = [
                png_sPLT_t {
                    name: sname.as_ptr() as *mut c_char,
                    depth: 8,
                    entries: ents.as_mut_ptr(),
                    nentries: 3,
                },
                png_sPLT_t {
                    name: sname2.as_ptr() as *mut c_char,
                    depth: 8,
                    entries: ents2.as_mut_ptr(),
                    nentries: 2,
                },
            ];
            (api.png_set_sPLT)(png, info, sps.as_ptr(), 2);
            let sw = cs("1.5");
            let sh = cs("2.5");
            (api.png_set_sCAL_s)(png, info, PNG_SCALE_METER, sw.as_ptr(), sh.as_ptr());
            let purpose = cs("a purpose");
            let units = cs("a unit");
            let p0 = cs("100");
            let p1 = cs("200");
            let mut params = [p0.as_ptr() as *mut c_char, p1.as_ptr() as *mut c_char];
            (api.png_set_pCAL)(
                png,
                info,
                purpose.as_ptr(),
                0,
                255,
                PNG_EQUATION_LINEAR,
                2,
                units.as_ptr(),
                params.as_mut_ptr(),
            );
            let mut exif: Vec<u8> = b"II*\0\x08\0\0\0\0\0".to_vec();
            (api.png_set_eXIf_1)(png, info, exif.len() as u32, exif.as_mut_ptr());
        });
        wr.bytes
    });
    // Two unknown (private, safe-to-copy) chunks, added by hand, so that
    // `png_free_data(.., PNG_FREE_UNKN, 1)` addresses a real entry.
    let v = insert_before(&base, "IDAT", &chunk(b"prVt", b"private chunk payload"));
    insert_before(&v, "IDAT", &chunk(b"qrVt", b"second private chunk"))
}

/* ================================================================== */
/* C-23 — the memory allocator API                                     */
/* ================================================================== */

/// The sizes every allocator entry point is probed with: 0, tiny, a page, and
/// several that cannot possibly succeed.
const MEM_SIZES: [usize; 9] = [
    0,
    1,
    8,
    4096,
    65536,
    1 << 20,
    1usize << 62,
    usize::MAX - 1,
    usize::MAX,
];

fn touchable(size: usize) -> bool {
    size > 0 && size <= (1 << 20)
}

/// Create a read struct either with the default allocator or with the logging
/// `png_set_mem_fn` one.
unsafe fn mk(api: &Api, custom: bool) -> (*mut PngStruct, *mut PngInfo) {
    if !custom {
        return new_read(api);
    }
    let sh = &libs().shim;
    let png = (api.png_create_read_struct_2)(
        VER,
        null_mut(),
        Some(sh.error_fn),
        Some(warn_cb),
        mem_tag(),
        Some(malloc_cb),
        Some(free_cb),
    );
    assert!(!png.is_null(), "{}: create_read_struct_2", api.which);
    let info = (api.png_create_info_struct)(png);
    assert!(!info.is_null());
    (png, info)
}

/// C-23: `png_malloc`, `png_calloc`, `png_malloc_warn`, `png_malloc_base`,
/// `png_malloc_array`, `png_realloc_array`, `png_free`, `png_free_data`.
#[test]
fn memory() {
    /* ---- png_malloc / png_malloc_warn / png_malloc_base / png_calloc ---- */
    for custom in [false, true] {
        for &size in &MEM_SIZES {
            for which in ["malloc", "malloc_warn", "malloc_base", "calloc"] {
                same(
                    &format!("{} size={} custom={}", which, size, custom),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = mk(api, custom);
                        let mut p: *mut c_void = null_mut();
                        let g = guarded(api, png, &mut || {
                            p = match which {
                                "malloc" => (api.png_malloc)(png, size),
                                "malloc_warn" => (api.png_malloc_warn)(png, size),
                                "malloc_base" => (api.png_malloc_base)(png, size),
                                _ => (api.png_calloc)(png, size),
                            };
                            log(format!("{}({}) null={}", which, size, p.is_null()));
                            if !p.is_null() && touchable(size) {
                                let b = core::slice::from_raw_parts(p as *const u8, size);
                                if which == "calloc" {
                                    log(format!(
                                        "calloc zeroed={} sum={}",
                                        b.iter().all(|&x| x == 0),
                                        b.iter().map(|&x| x as u64).sum::<u64>()
                                    ));
                                }
                                let q = p as *mut u8;
                                *q = 0x5a;
                                *q.add(size - 1) = 0xa5;
                                log(format!("touch {:#04x} {:#04x}", *q, *q.add(size - 1)));
                            }
                            (api.png_free)(png, p);
                            p = null_mut();
                        });
                        o.push(format!("guard={:?}", g));
                        if !p.is_null() {
                            (api.png_free)(png, p);
                        }
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }

    /* ---- png_calloc really returns zeroed memory even after reuse ---- */
    for custom in [false, true] {
        for &size in &[1usize, 8, 64, 999, 4096, 65536] {
            same(
                &format!("calloc after dirty size={} custom={}", size, custom),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = mk(api, custom);
                    let g = guarded(api, png, &mut || {
                        // dirty a block of exactly this size, then release it
                        let a = (api.png_malloc)(png, size);
                        assert!(!a.is_null());
                        core::ptr::write_bytes(a as *mut u8, 0xcc, size);
                        (api.png_free)(png, a);
                        // ... and see whether png_calloc still hands back zeros
                        let b = (api.png_calloc)(png, size);
                        assert!(!b.is_null());
                        let s = core::slice::from_raw_parts(b as *const u8, size);
                        log(format!(
                            "calloc({}) zeroed={} nonzero={}",
                            size,
                            s.iter().all(|&x| x == 0),
                            s.iter().filter(|&&x| x != 0).count()
                        ));
                        (api.png_free)(png, b);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_malloc_base / png_free with a NULL png_ptr ---- */
    same("malloc_base(NULL) and free(NULL,..)", |api| unsafe {
        let mut o = Outcome::default();
        for &size in &[0usize, 1, 32, 4096, usize::MAX] {
            let p = (api.png_malloc_base)(null_mut(), size);
            o.push(format!("malloc_base(NULL,{}) null={}", size, p.is_null()));
            // freeing through a NULL png_ptr is a documented no-op, so release
            // the block with the real allocator instead.
            if !p.is_null() {
                (api.png_free)(null_mut(), p);
                xfree(p);
            }
        }
        let (png, info) = new_read(api);
        (api.png_free)(png, null_mut());
        (api.png_free)(null_mut(), null_mut());
        (api.png_free_default)(png, null_mut());
        (api.png_free_default)(null_mut(), null_mut());
        o.push("free(NULL) survived".to_string());
        let mut p = null_mut();
        let g = guarded(api, png, &mut || {
            p = (api.png_malloc)(null_mut(), 16);
            log(format!("png_malloc(NULL,16) null={}", p.is_null()));
            let q = (api.png_malloc_warn)(null_mut(), 16);
            log(format!("png_malloc_warn(NULL,16) null={}", q.is_null()));
            let r = (api.png_malloc_default)(null_mut(), 16);
            log(format!("png_malloc_default(NULL,16) null={}", r.is_null()));
        });
        o.push(format!("guard={:?}", g));
        let _ = p;
        destroy_read(api, png, info);
        o
    });

    /* ---- png_malloc_default bypasses the user allocator ---- */
    for custom in [false, true] {
        for &size in &[0usize, 1, 4096, usize::MAX] {
            same(
                &format!("malloc_default size={} custom={}", size, custom),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = mk(api, custom);
                    let g = guarded(api, png, &mut || {
                        let p = (api.png_malloc_default)(png, size);
                        log(format!("malloc_default({}) null={}", size, p.is_null()));
                        if !p.is_null() {
                            // allocated by plain malloc: release the same way
                            (api.png_free_default)(png, p);
                        }
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_malloc_array ---- */
    let array_cases: [(c_int, usize); 16] = [
        (0, 8),
        (-1, 8),
        (i32::MIN, 8),
        (1, 0),
        (0, 0),
        (-3, 0),
        (1, 1),
        (1, 8),
        (2, 8),
        (17, 3),
        (1000, 16),
        (i32::MAX, 1),
        (i32::MAX, 2),
        (i32::MAX, 1usize << 40),
        (1024, usize::MAX),
        (3, usize::MAX / 2),
    ];
    for custom in [false, true] {
        for &(n, es) in &array_cases {
            same(
                &format!("malloc_array n={} es={} custom={}", n, es, custom),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = mk(api, custom);
                    let g = guarded(api, png, &mut || {
                        let p = (api.png_malloc_array)(png, n, es);
                        log(format!("malloc_array({},{}) null={}", n, es, p.is_null()));
                        if !p.is_null() {
                            let total = (n as usize).saturating_mul(es);
                            if total <= (1 << 20) {
                                core::ptr::write_bytes(p as *mut u8, 0x77, total);
                                log(format!("wrote {} bytes", total));
                            }
                            (api.png_free)(png, p);
                        }
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_realloc_array: the error paths ---- */
    let bad_realloc: [(bool, c_int, c_int, usize); 10] = [
        // (pass a real old array, old_elements, add_elements, element_size)
        (false, 0, 0, 8),
        (false, 0, -1, 8),
        (false, 0, i32::MIN, 8),
        (false, 0, 4, 0),
        (false, -1, 4, 8),
        (false, 3, 4, 8),   // old_array NULL but old_elements > 0
        (true, i32::MAX - 1, 5, 4),
        (true, 1000, 1000, 1usize << 50),
        (true, 2, i32::MAX, 4),
        (true, 0, i32::MAX, 1usize << 40),
    ];
    for custom in [false, true] {
        for &(real, oldn, addn, es) in &bad_realloc {
            same(
                &format!(
                    "realloc_array real={} old={} add={} es={} custom={}",
                    real, oldn, addn, es, custom
                ),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = mk(api, custom);
                    // A non-NULL pointer that is never dereferenced because
                    // every one of these cases bails out first.
                    let fake = 0x1000usize as *const c_void;
                    let g = guarded(api, png, &mut || {
                        let old = if real { fake } else { null() };
                        let p = (api.png_realloc_array)(png, old, oldn, addn, es);
                        log(format!("realloc_array null={}", p.is_null()));
                        if !p.is_null() {
                            (api.png_free)(png, p);
                        }
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_realloc_array: the successful path, contents compared ---- */
    for custom in [false, true] {
        for &(oldn, addn, es) in &[
            (0i32, 1i32, 1usize),
            (0, 1, 8),
            (0, 5, 4),
            (1, 1, 4),
            (3, 5, 4),
            (3, 5, 1),
            (7, 1, 16),
            (10, 10, 3),
            (100, 28, 2),
        ] {
            same(
                &format!(
                    "realloc_array grow old={} add={} es={} custom={}",
                    oldn, addn, es, custom
                ),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = mk(api, custom);
                    let g = guarded(api, png, &mut || {
                        let mut old: *mut c_void = null_mut();
                        if oldn > 0 {
                            old = (api.png_malloc_array)(png, oldn, es);
                            assert!(!old.is_null());
                            let s = core::slice::from_raw_parts_mut(
                                old as *mut u8,
                                oldn as usize * es,
                            );
                            for (i, b) in s.iter_mut().enumerate() {
                                *b = (i as u8).wrapping_mul(7).wrapping_add(1);
                            }
                            log(format!("old array: {:02x?}", s));
                        }
                        let new = (api.png_realloc_array)(png, old, oldn, addn, es);
                        log(format!("new array null={}", new.is_null()));
                        if !new.is_null() {
                            let total = (oldn + addn) as usize * es;
                            let s = core::slice::from_raw_parts(new as *const u8, total);
                            log(format!("new array: {:02x?}", s));
                            (api.png_free)(png, new);
                        }
                        (api.png_free)(png, old);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_free_data over every mask, on a fully populated info ---- */
    let rich = rich_file();
    let masks: [(&str, u32); 13] = [
        ("TEXT", PNG_FREE_TEXT),
        ("TRNS", PNG_FREE_TRNS),
        ("PLTE", PNG_FREE_PLTE),
        ("HIST", PNG_FREE_HIST),
        ("ICCP", PNG_FREE_ICCP),
        ("SPLT", PNG_FREE_SPLT),
        ("PCAL", PNG_FREE_PCAL),
        ("SCAL", PNG_FREE_SCAL),
        ("EXIF", PNG_FREE_EXIF),
        ("UNKN", PNG_FREE_UNKN),
        ("ROWS", PNG_FREE_ROWS),
        ("ALL", PNG_FREE_ALL),
        ("NONE", 0),
    ];
    for (tag, mask) in masks {
        for num in [-1i32, 0, 1] {
            same(
                &format!("png_free_data {} num={}", tag, num),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = rich.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    (api.png_set_keep_unknown_chunks)(
                        png,
                        PNG_HANDLE_CHUNK_ALWAYS,
                        null(),
                        0,
                    );
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        // png_free_data() trusts `num`, so make sure the entry
                        // the test asks it to free really exists.
                        let mut tp: *mut png_text = null_mut();
                        let mut nt = 0;
                        let ntext = (api.png_get_text)(png, info, &mut tp, &mut nt);
                        let mut sp: *mut png_sPLT_t = null_mut();
                        let nsplt = (api.png_get_sPLT)(png, info, &mut sp);
                        let mut up: *mut png_unknown_chunk = null_mut();
                        let nunk = (api.png_get_unknown_chunks)(png, info, &mut up);
                        assert!(
                            ntext > num && nsplt > num && nunk > num,
                            "{}: rich_file has only {} tEXt / {} sPLT / {} unknown \
                             chunks, so png_free_data(.., {}) would be out of bounds",
                            api.which,
                            ntext,
                            nsplt,
                            nunk,
                            num
                        );
                        log(format!(
                            "counts: text={} splt={} unknown={}",
                            ntext, nsplt, nunk
                        ));
                        log(format!(
                            "before: valid={:#x} texts={}",
                            (api.png_get_valid)(png, info, 0xffff_ffff),
                            {
                                let mut tp: *mut png_text = null_mut();
                                let mut n = 0;
                                (api.png_get_text)(png, info, &mut tp, &mut n)
                            }
                        ));
                        (api.png_free_data)(png, info, mask, num);
                        log(format!(
                            "after: valid={:#x} texts={}",
                            (api.png_get_valid)(png, info, 0xffff_ffff),
                            {
                                let mut tp: *mut png_text = null_mut();
                                let mut n = 0;
                                (api.png_get_text)(png, info, &mut tp, &mut n)
                            }
                        ));
                        // a second identical call must be a harmless no-op
                        (api.png_free_data)(png, info, mask, num);
                        log(format!(
                            "again: valid={:#x}",
                            (api.png_get_valid)(png, info, 0xffff_ffff)
                        ));
                        // and NULL arguments must be ignored
                        (api.png_free_data)(png, null_mut(), mask, num);
                        (api.png_free_data)(null_mut(), info, mask, num);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    report("memory");
}

/* ================================================================== */
/* C-64 — png_get_io_state / png_get_io_chunk_type / png_init_io        */
/* ================================================================== */

/// C-64: the IO state machine, sampled from inside every read and write call.
#[test]
fn io_state() {
    /* ---- the io_ptr accessors ---- */
    same("png_get_io_ptr", |api| unsafe {
        let mut o = Outcome::default();
        let (rp, ri) = new_read(api);
        o.push(format!(
            "fresh read io_ptr null={}",
            (api.png_get_io_ptr)(rp).is_null()
        ));
        (api.png_set_read_fn)(rp, mem_tag(), Some(read_cb));
        o.push(format!(
            "after set_read_fn io_ptr==tag {}",
            (api.png_get_io_ptr)(rp) == mem_tag()
        ));
        (api.png_set_read_fn)(rp, null_mut(), None);
        o.push(format!(
            "read_fn=NULL io_ptr null={} state={:#06x}",
            (api.png_get_io_ptr)(rp).is_null(),
            (api.png_get_io_state)(rp)
        ));
        o.push(format!(
            "fresh read io_state={:#06x} chunk={}",
            (api.png_get_io_state)(rp),
            chunk_str((api.png_get_io_chunk_type)(rp))
        ));
        destroy_read(api, rp, ri);

        let (wp, wi) = new_write(api);
        o.push(format!(
            "fresh write io_ptr null={} state={:#06x} chunk={}",
            (api.png_get_io_ptr)(wp).is_null(),
            (api.png_get_io_state)(wp),
            chunk_str((api.png_get_io_chunk_type)(wp))
        ));
        (api.png_set_write_fn)(wp, mem_tag(), Some(write_cb), Some(flush_cb));
        o.push(format!(
            "after set_write_fn io_ptr==tag {}",
            (api.png_get_io_ptr)(wp) == mem_tag()
        ));
        // setting a read fn on a write struct warns and clears write_data_fn
        (api.png_set_read_fn)(wp, null_mut(), Some(read_cb));
        o.push("crossed read/write fn".to_string());
        destroy_write(api, wp, wi);
        o.push(format!(
            "get_io_ptr(NULL) null={}",
            (api.png_get_io_ptr)(null_mut()).is_null()
        ));
        o
    });

    /* ---- the full write trace ---- */
    for palette in [false, true] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(0x105 ^ (palette as u64) ^ ((il as u64) << 8));
            let mut img = if palette {
                let mut i = Img::random(&mut rng, 9, 4, PNG_COLOR_TYPE_PALETTE, 4);
                i.palette.truncate(16);
                i
            } else {
                Img::random(&mut rng, 9, 4, PNG_COLOR_TYPE_RGB, 8)
            };
            img.interlace = il;
            same(
                &format!("write io trace palette={} il={}", palette, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(png, null_mut(), Some(io_write_cb), Some(flush_cb));
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
                        if img.color_type == PNG_COLOR_TYPE_PALETTE {
                            (api.png_set_PLTE)(
                                png,
                                info,
                                img.palette.as_ptr(),
                                img.palette.len() as c_int,
                            );
                        }
                        (api.png_set_gAMA_fixed)(png, info, 45455);
                        (api.png_set_pHYs)(png, info, 72, 72, PNG_RESOLUTION_UNKNOWN);
                        let key = cs("Comment");
                        let txt = cs("io state trace");
                        let te = png_text {
                            compression: PNG_TEXT_COMPRESSION_NONE,
                            key: key.as_ptr() as *mut c_char,
                            text: txt.as_ptr() as *mut c_char,
                            text_length: 0,
                            itxt_length: 0,
                            lang: null_mut(),
                            lang_key: null_mut(),
                        };
                        (api.png_set_text)(png, info, &te, 1);
                        (api.png_write_info)(png, info);
                        let passes = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        for _ in 0..passes {
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                        }
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!(
                        "guard={:?} final state={:#06x} chunk={}",
                        g,
                        (api.png_get_io_state)(png),
                        chunk_str((api.png_get_io_chunk_type)(png))
                    ));
                    o.output = std::mem::take(&mut tls().output);
                    destroy_write(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- io_state while the raw chunk writer is used directly ---- */
    for &(pieces, len) in &[(1usize, 0usize), (1, 7), (2, 100), (5, 9000), (3, 8192)] {
        same(
            &format!("raw chunk io trace pieces={} len={}", pieces, len),
            |api| unsafe {
                let mut o = Outcome::default();
                let mut rng = Rng::new(0x2a2a ^ len as u64 ^ ((pieces as u64) << 32));
                let data = rng.bytes(len);
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(png, null_mut(), Some(io_write_cb), Some(flush_cb));
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    if pieces == 1 {
                        (api.png_write_chunk)(png, b"prVt".as_ptr(), data.as_ptr(), data.len());
                    } else {
                        (api.png_write_chunk_start)(png, b"prVt".as_ptr(), data.len() as u32);
                        let step = data.len().div_ceil(pieces).max(1);
                        let mut i = 0;
                        while i < data.len() {
                            let k = step.min(data.len() - i);
                            (api.png_write_chunk_data)(png, data.as_ptr().add(i), k);
                            i += k;
                        }
                        (api.png_write_chunk_end)(png);
                    }
                });
                o.push(format!(
                    "guard={:?} state={:#06x} chunk={}",
                    g,
                    (api.png_get_io_state)(png),
                    chunk_str((api.png_get_io_chunk_type)(png))
                ));
                o.output = std::mem::take(&mut tls().output);
                destroy_write(api, png, core::ptr::null_mut());
                o
            },
        );
    }

    /* ---- io_state over a big image, i.e. many IDAT writes, plus flushes ---- */
    for &(w, h, flush) in &[(64u32, 40u32, 0i32), (64, 40, 3), (100, 60, 1)] {
        let mut rng = Rng::new(0x2b2b ^ w as u64 ^ ((h as u64) << 20) ^ ((flush as u64) << 40));
        let img = Img::random(&mut rng, w, h, PNG_COLOR_TYPE_RGB_ALPHA, 8);
        let mut f = Vec::new();
        same(
            &format!("io trace big {}x{} flush={}", w, h, flush),
            |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(png, null_mut(), Some(io_write_cb), Some(flush_cb));
                (api.png_set_compression_buffer_size)(png, 512);
                if flush > 0 {
                    (api.png_set_flush)(png, flush);
                }
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        img.w,
                        img.h,
                        8,
                        PNG_COLOR_TYPE_RGB_ALPHA,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_write_info)(png, info);
                    for r in &img.rows {
                        (api.png_write_row)(png, r.as_ptr());
                    }
                    (api.png_write_end)(png, info);
                });
                o.push(format!("guard={:?} flushes={}", g, tls().flushes));
                o.output = std::mem::take(&mut tls().output);
                if api.which == "C" {
                    f = o.output.clone();
                }
                destroy_write(api, png, info);
                o
            },
        );
        same(&format!("io trace big read {}x{}", w, h), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = f.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(io_read_cb));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let hh = (api.png_get_image_height)(png, info) as usize;
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..hh {
                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- io_state at the point a truncated datastream fails ---- */
    let whole = multi_chunk_file(false);
    for cut in [0usize, 1, 4, 8, 9, 20, 33, 60, 90, 120] {
        let cut = cut.min(whole.len());
        same(&format!("io trace truncated at {}", cut), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = whole[..cut].to_vec();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(io_read_cb));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let h = (api.png_get_image_height)(png, info) as usize;
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..h {
                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!(
                "guard={:?} state={:#06x} chunk={}",
                g,
                (api.png_get_io_state)(png),
                chunk_str((api.png_get_io_chunk_type)(png))
            ));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- the full read trace ---- */
    let files: [(&str, Vec<u8>); 5] = [
        ("rgb", multi_chunk_file(false)),
        ("palette", multi_chunk_file(true)),
        ("handmade", handmade_gray1x1()),
        ("rich", rich_file()),
        (
            "interlaced",
            with_c(|api| unsafe {
                let mut rng = Rng::new(0x2c2c);
                let mut img = Img::random(&mut rng, 11, 7, PNG_COLOR_TYPE_GRAY_ALPHA, 16);
                img.interlace = PNG_INTERLACE_ADAM7;
                write_plain(api, &img, &WriteOpts::default()).bytes
            }),
        ),
    ];
    for (tag, file) in &files {
        for pre in [0usize, 4, 8] {
            same(&format!("read io trace {} sig={}", tag, pre), |api| unsafe {
                let mut o = Outcome::default();
                tls().input = file[pre..].to_vec();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, null_mut(), Some(io_read_cb));
                let g = guarded(api, png, &mut || {
                    (api.png_set_sig_bytes)(png, pre as c_int);
                    (api.png_read_info)(png, info);
                    log(format!(
                        "after read_info state={:#06x} chunk={}",
                        (api.png_get_io_state)(png),
                        chunk_str((api.png_get_io_chunk_type)(png))
                    ));
                    let h = (api.png_get_image_height)(png, info) as usize;
                    let rb = (api.png_get_rowbytes)(png, info);
                    let passes =
                        if (api.png_get_interlace_type)(png, info) as c_int == PNG_INTERLACE_ADAM7 {
                            7
                        } else {
                            1
                        };
                    let mut row = vec![0u8; rb];
                    for _ in 0..passes {
                        for _ in 0..h {
                            (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                        }
                    }
                    (api.png_read_end)(png, info);
                    log(format!(
                        "after read_end state={:#06x} chunk={}",
                        (api.png_get_io_state)(png),
                        chunk_str((api.png_get_io_chunk_type)(png))
                    ));
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            });
        }
    }

    /* ---- png_init_io with a real FILE* from tmpfile() ---- */
    let mut rng = Rng::new(0xf11e);
    let img = Img::random(&mut rng, 11, 5, PNG_COLOR_TYPE_RGB_ALPHA, 8);
    same("png_init_io write to tmpfile", |api| unsafe {
        let mut o = Outcome::default();
        let f = temp_file(api.which);
        assert!(!f.is_null(), "no writable temporary FILE*");
        let (png, info) = new_write(api);
        (api.png_init_io)(png, f);
        o.push(format!("io_ptr==FILE* {}", (api.png_get_io_ptr)(png) == f));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB_ALPHA,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for r in &img.rows {
                (api.png_write_row)(png, r.as_ptr());
            }
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        fflush(f);
        rewind(f);
        let mut buf = vec![0u8; 1 << 16];
        let n = fread(buf.as_mut_ptr(), 1, buf.len(), f);
        buf.truncate(n);
        o.push(format!("wrote {} bytes to the FILE*", n));
        o.output = buf;
        fclose(f);
        destroy_write(api, png, info);
        o
    });

    let file = rgb_file(11, 5, 0xf11f);
    let path = scratch_dir().join("misc-io-state.png");
    std::fs::write(&path, &file).expect("write temp png");
    let cpath = cs(path.to_str().unwrap());
    same("png_init_io read from a real file", |api| unsafe {
        let mut o = Outcome::default();
        let mode = cs("rb");
        let f = fopen(cpath.as_ptr(), mode.as_ptr());
        assert!(!f.is_null(), "fopen failed");
        let (png, info) = new_read(api);
        (api.png_init_io)(png, f);
        o.push(format!("io_ptr==FILE* {}", (api.png_get_io_ptr)(png) == f));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            log_info(api, png, info, "file read");
            let h = (api.png_get_image_height)(png, info) as usize;
            let rb = (api.png_get_rowbytes)(png, info);
            for _ in 0..h {
                let mut row = vec![0u8; rb];
                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                log(format!("row {:02x?}", row));
            }
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        fclose(f);
        destroy_read(api, png, info);
        o
    });

    same("png_init_io(png, NULL) then get_io_ptr", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        (api.png_init_io)(png, null_mut());
        o.push(format!(
            "io_ptr null={}",
            (api.png_get_io_ptr)(png).is_null()
        ));
        (api.png_init_io)(null_mut(), mem_tag());
        o.push("init_io(NULL, tag) survived".to_string());
        destroy_read(api, png, info);
        o
    });

    // Reading through the default (stdio) read function with a NULL FILE* is
    // fatal in both libraries; compare *how* it dies.
    samef("png_init_io(NULL FILE*) then read_info", |api| {
        guarded_in_child(api, false, &mut |api, png, info| unsafe {
            (api.png_init_io)(png, null_mut());
            (api.png_read_info)(png, info);
        })
    });

    report("io_state");
}

/* ================================================================== */
/* C-142 — png_set_crc_action                                          */
/* ================================================================== */

/// Drive a read with a given (crit, ancil) CRC policy installed *before*
/// `png_read_info`, so that a broken IHDR CRC is covered too.
unsafe fn read_crc(api: &Api, data: &[u8], crit: c_int, ancil: c_int) -> Outcome {
    let mut o = Outcome::default();
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
    (api.png_set_crc_action)(png, crit, ancil);
    let g = guarded(api, png, &mut || {
        (api.png_read_info)(png, info);
        log_info(api, png, info, "crc");
        let mut gam = 0.0f64;
        log(format!(
            "gAMA r={} v={}",
            (api.png_get_gAMA)(png, info, &mut gam),
            gam
        ));
        let h = (api.png_get_image_height)(png, info) as usize;
        let rb = (api.png_get_rowbytes)(png, info);
        let mut rows = vec![vec![0u8; rb]; h];
        for r in rows.iter_mut() {
            (api.png_read_row)(png, r.as_mut_ptr(), null_mut());
        }
        for (y, r) in rows.iter().enumerate() {
            log(format!("row {}: {:02x?}", y, r));
        }
        (api.png_read_end)(png, info);
    });
    o.push(format!("guard={:?}", g));
    destroy_read(api, png, info);
    o
}

/// C-142: all 36 (crit, ancil) combinations against correct, broken-ancillary
/// and broken-critical CRCs.
#[test]
fn crc_action() {
    let rgb = multi_chunk_file(false);
    let pal = multi_chunk_file(true);
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("rgb ok".into(), rgb.clone()),
        ("bad gAMA crc".into(), break_crc(&rgb, "gAMA")),
        ("bad tEXt crc".into(), break_crc(&rgb, "tEXt")),
        ("bad IHDR crc".into(), break_crc(&rgb, "IHDR")),
        ("bad IDAT crc".into(), break_crc(&rgb, "IDAT")),
        ("bad IEND crc".into(), break_crc(&rgb, "IEND")),
        ("pal ok".into(), pal.clone()),
        ("bad PLTE crc".into(), break_crc(&pal, "PLTE")),
        ("bad tRNS crc".into(), break_crc(&pal, "tRNS")),
        ("bad hIST crc".into(), break_crc(&pal, "hIST")),
    ];
    let actions = [
        PNG_CRC_DEFAULT,
        PNG_CRC_ERROR_QUIT,
        PNG_CRC_WARN_DISCARD,
        PNG_CRC_WARN_USE,
        PNG_CRC_QUIET_USE,
        PNG_CRC_NO_CHANGE,
    ];
    for (tag, data) in &cases {
        for &crit in &actions {
            for &ancil in &actions {
                same(
                    &format!("{} crit={} ancil={}", tag, crit, ancil),
                    |api| unsafe { read_crc(api, data, crit, ancil) },
                );
            }
        }
    }

    /* out-of-range actions fall into the `default:` arms */
    for &(crit, ancil) in &[
        (6, 6),
        (-1, -1),
        (100, 100),
        (i32::MIN, i32::MAX),
        (6, PNG_CRC_QUIET_USE),
        (PNG_CRC_QUIET_USE, 6),
        (PNG_CRC_NO_CHANGE, -5),
    ] {
        for (tag, data) in &cases {
            same(
                &format!("{} odd crit={} ancil={}", tag, crit, ancil),
                |api| unsafe { read_crc(api, data, crit, ancil) },
            );
        }
    }

    /* repeated / accumulating calls, and the NULL png_ptr no-op */
    same("crc_action repeated + NULL", |api| unsafe {
        let mut o = Outcome::default();
        (api.png_set_crc_action)(null_mut(), PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE);
        o.push("NULL survived".to_string());
        tls().input = break_crc(&rgb, "gAMA");
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        (api.png_set_crc_action)(png, PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE);
        (api.png_set_crc_action)(png, PNG_CRC_NO_CHANGE, PNG_CRC_NO_CHANGE);
        (api.png_set_crc_action)(png, PNG_CRC_WARN_USE, PNG_CRC_NO_CHANGE);
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            log_info(api, png, info, "repeated");
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });

    report("crc_action");
}

/* ================================================================== */
/* C-143 — the user limits                                             */
/* ================================================================== */

/// C-143: `png_set_user_limits` and friends, below / at / above the real sizes.
#[test]
fn user_limits() {
    /* ---- the plain getter/setter round trip ---- */
    same("user limit getters", |api| unsafe {
        let mut o = Outcome::default();
        for write in [false, true] {
            let (png, info) = if write { new_write(api) } else { new_read(api) };
            o.push(format!(
                "write={} defaults w={} h={} cache={} malloc={}",
                write,
                (api.png_get_user_width_max)(png),
                (api.png_get_user_height_max)(png),
                (api.png_get_chunk_cache_max)(png),
                (api.png_get_chunk_malloc_max)(png)
            ));
            for &(w, h) in &[
                (0u32, 0u32),
                (1, 1),
                (9, 4),
                (10, 5),
                (0x7fff_ffff, 0x7fff_ffff),
                (0xffff_ffff, 0xffff_ffff),
                (1_000_000, 1_000_000),
            ] {
                (api.png_set_user_limits)(png, w, h);
                o.push(format!(
                    "set({},{}) -> w={} h={}",
                    w,
                    h,
                    (api.png_get_user_width_max)(png),
                    (api.png_get_user_height_max)(png)
                ));
            }
            for &c in &[0u32, 1, 2, 3, 100, 0xffff_ffff] {
                (api.png_set_chunk_cache_max)(png, c);
                o.push(format!(
                    "cache({}) -> {}",
                    c,
                    (api.png_get_chunk_cache_max)(png)
                ));
            }
            for &m in &[0usize, 1, 2, 100, 8192, usize::MAX, usize::MAX - 1] {
                (api.png_set_chunk_malloc_max)(png, m);
                o.push(format!(
                    "malloc_max({}) -> {}",
                    m,
                    (api.png_get_chunk_malloc_max)(png)
                ));
            }
            if write {
                destroy_write(api, png, info);
            } else {
                destroy_read(api, png, info);
            }
        }
        // NULL png_ptr
        (api.png_set_user_limits)(null_mut(), 1, 1);
        (api.png_set_chunk_cache_max)(null_mut(), 1);
        (api.png_set_chunk_malloc_max)(null_mut(), 1);
        o.push(format!(
            "NULL getters w={} h={} cache={} malloc={}",
            (api.png_get_user_width_max)(null_mut()),
            (api.png_get_user_height_max)(null_mut()),
            (api.png_get_chunk_cache_max)(null_mut()),
            (api.png_get_chunk_malloc_max)(null_mut())
        ));
        o
    });

    /* ---- the read side: limits below / equal to / above 9x4 ---- */
    let file = multi_chunk_file(false); // 9 x 4
    for &(w, h) in &[
        (0u32, 0u32),
        (1, 1),
        (8, 4),
        (9, 3),
        (9, 4),
        (10, 5),
        (0x7fff_ffff, 0x7fff_ffff),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        same(&format!("read limits w={} h={}", w, h), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = file.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            (api.png_set_user_limits)(png, w, h);
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                log_info(api, png, info, "limits");
                let rows = (api.png_get_image_height)(png, info) as usize;
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..rows {
                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- the write side: png_set_IHDR against the limits ---- */
    for &(w, h) in &[(0u32, 0u32), (1, 1), (5, 5), (6, 6), (7, 7), (0x7fff_ffff, 7)] {
        same(&format!("write limits w={} h={}", w, h), |api| unsafe {
            let mut o = Outcome::default();
            let mut rng = Rng::new(0x717 ^ w as u64 ^ ((h as u64) << 20));
            let img = Img::random(&mut rng, 6, 6, PNG_COLOR_TYPE_GRAY, 8);
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
            (api.png_set_user_limits)(png, w, h);
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    img.w,
                    img.h,
                    8,
                    PNG_COLOR_TYPE_GRAY,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                (api.png_write_info)(png, info);
                for r in &img.rows {
                    (api.png_write_row)(png, r.as_ptr());
                }
                (api.png_write_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
    }

    /* ---- chunk_cache_max against a file with many ancillary chunks ---- */
    let mut many = rgb_file(4, 2, 0x7a7a);
    for i in 0..8u8 {
        let mut d = Vec::new();
        d.extend_from_slice(format!("Key{}", i).as_bytes());
        d.push(0);
        d.extend_from_slice(b"some cached text payload");
        many = insert_before(&many, "IDAT", &chunk(b"tEXt", &d));
    }
    for &cache in &[0u32, 1, 2, 3, 4, 8, 9, 100] {
        same(&format!("chunk_cache_max={}", cache), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = many.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            (api.png_set_chunk_cache_max)(png, cache);
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let mut tp: *mut png_text = null_mut();
                let mut n = 0;
                let cnt = (api.png_get_text)(png, info, &mut tp, &mut n);
                log(format!("texts={} n={}", cnt, n));
                for i in 0..cnt {
                    let t = *tp.add(i as usize);
                    log(format!(
                        "text[{}] key={:?} len={}",
                        i,
                        if t.key.is_null() {
                            "<null>".to_string()
                        } else {
                            std::ffi::CStr::from_ptr(t.key).to_string_lossy().into_owned()
                        },
                        t.text_length
                    ));
                }
                log(format!(
                    "cache left={}",
                    (api.png_get_chunk_cache_max)(png)
                ));
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- chunk_malloc_max against a big tEXt and a big unknown chunk ---- */
    let big = {
        let mut d = b"BigKey\0".to_vec();
        d.extend(std::iter::repeat(b'x').take(5000));
        let mut f = rgb_file(4, 2, 0x7b7b);
        f = insert_before(&f, "IDAT", &chunk(b"tEXt", &d));
        insert_before(&f, "IDAT", &chunk(b"prVt", &vec![0x42u8; 5000]))
    };
    for &m in &[0usize, 1, 8, 100, 4999, 5000, 5001, 8192, usize::MAX] {
        same(&format!("chunk_malloc_max={}", m), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = big.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            (api.png_set_chunk_malloc_max)(png, m);
            (api.png_set_keep_unknown_chunks)(png, PNG_HANDLE_CHUNK_ALWAYS, null(), 0);
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let mut tp: *mut png_text = null_mut();
                let mut n = 0;
                log(format!(
                    "texts={} unknown={}",
                    (api.png_get_text)(png, info, &mut tp, &mut n),
                    (api.png_get_unknown_chunks)(png, info, &mut null_mut())
                ));
                (api.png_read_end)(png, info);
            });
            o.push(format!(
                "guard={:?} malloc_max={}",
                g,
                (api.png_get_chunk_malloc_max)(png)
            ));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- limits against hand-built IHDRs with extreme dimensions ---- */
    let plain = rgb_file(4, 2, 0x7c7c);
    let ihdr_range = split_chunks(&plain)
        .into_iter()
        .find(|(n, _)| n == "IHDR")
        .map(|(_, r)| r)
        .unwrap();
    for &(w, h) in &[
        (1u32, 1u32),
        (0, 1),
        (1, 0),
        (1000, 1000),
        (1_000_000, 1_000_000),
        (1_000_001, 1),
        (1, 1_000_001),
        (0x7fff_ffff, 1),
        (1, 0x7fff_ffff),
        (0x8000_0000, 1),
        (0xffff_ffff, 0xffff_ffff),
    ] {
        let mut d = plain[ihdr_range.start + 8..ihdr_range.end - 4].to_vec();
        d[0..4].copy_from_slice(&w.to_be_bytes());
        d[4..8].copy_from_slice(&h.to_be_bytes());
        let mut file = plain[..ihdr_range.start].to_vec();
        file.extend_from_slice(&chunk(b"IHDR", &d));
        file.extend_from_slice(&plain[ihdr_range.end..]);
        for &(lw, lh) in &[
            (0u32, 0u32),
            (1, 1),
            (1000, 1000),
            (1_000_000, 1_000_000),
            (0x7fff_ffff, 0x7fff_ffff),
            (0xffff_ffff, 0xffff_ffff),
        ] {
            same(
                &format!("IHDR {}x{} limits {}x{}", w, h, lw, lh),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = file.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    (api.png_set_user_limits)(png, lw, lh);
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        log_info(api, png, info, "extreme");
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- chunk_malloc_max against the compressed-text limit ---- */
    let ztxt = {
        let mut d = b"ZKey\0".to_vec();
        d.push(0); // zTXt compression method
        d.extend_from_slice(&zlib_stored(&vec![b'z'; 4000]));
        insert_before(&rgb_file(4, 2, 0x7d7d), "IDAT", &chunk(b"zTXt", &d))
    };
    for &m in &[0usize, 1, 100, 3999, 4000, 4001, 65536, usize::MAX] {
        same(&format!("zTXt malloc_max={}", m), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = ztxt.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            (api.png_set_chunk_malloc_max)(png, m);
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                let mut tp: *mut png_text = null_mut();
                let mut n = 0;
                let cnt = (api.png_get_text)(png, info, &mut tp, &mut n);
                log(format!("texts={}", cnt));
                for i in 0..cnt {
                    let t = *tp.add(i as usize);
                    log(format!("text[{}] comp={} len={}", i, t.compression, t.text_length));
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- both limits at once, on a large real image ---- */
    let big_img = rgb_file(40, 30, 0x7e7e);
    for &(lw, lh, cache, malloc_max) in &[
        (0u32, 0u32, 0u32, 0usize),
        (40, 30, 1, 1),
        (39, 30, 2, 100),
        (40, 29, 3, 65536),
        (0x7fff_ffff, 0x7fff_ffff, 0, usize::MAX),
    ] {
        same(
            &format!(
                "combined limits {}x{} cache={} malloc={}",
                lw, lh, cache, malloc_max
            ),
            |api| unsafe {
                let mut o = Outcome::default();
                tls().input = big_img.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                (api.png_set_user_limits)(png, lw, lh);
                (api.png_set_chunk_cache_max)(png, cache);
                (api.png_set_chunk_malloc_max)(png, malloc_max);
                let g = guarded(api, png, &mut || {
                    (api.png_read_info)(png, info);
                    log_info(api, png, info, "combined");
                    let h = (api.png_get_image_height)(png, info) as usize;
                    let rb = (api.png_get_rowbytes)(png, info);
                    let mut row = vec![0u8; rb];
                    for _ in 0..h {
                        (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                        o.output.extend_from_slice(&row);
                    }
                    (api.png_read_end)(png, info);
                });
                o.push(format!(
                    "guard={:?} w={} h={} cache={} malloc={}",
                    g,
                    (api.png_get_user_width_max)(png),
                    (api.png_get_user_height_max)(png),
                    (api.png_get_chunk_cache_max)(png),
                    (api.png_get_chunk_malloc_max)(png)
                ));
                destroy_read(api, png, info);
                o
            },
        );
    }

    report("user_limits");
}

/* ================================================================== */
/* C-144 — the custom allocator                                        */
/* ================================================================== */

/// C-144: `png_create_read_struct_2` / `png_create_write_struct_2` /
/// `png_set_mem_fn` / `png_get_mem_ptr`, including an allocator that starts
/// failing after N allocations.
#[test]
fn custom_alloc() {
    /* ---- mem_ptr plumbing ---- */
    same("mem_ptr plumbing", |api| unsafe {
        let mut o = Outcome::default();
        let sh = &libs().shim;
        let (png, info) = new_read(api);
        o.push(format!(
            "default mem_ptr null={}",
            (api.png_get_mem_ptr)(png).is_null()
        ));
        (api.png_set_mem_fn)(png, mem_tag(), Some(malloc_cb), Some(free_cb));
        o.push(format!(
            "after set_mem_fn ==tag {}",
            (api.png_get_mem_ptr)(png) == mem_tag()
        ));
        (api.png_set_mem_fn)(png, null_mut(), None, None);
        o.push(format!(
            "cleared mem_ptr null={}",
            (api.png_get_mem_ptr)(png).is_null()
        ));
        (api.png_set_mem_fn)(null_mut(), mem_tag(), Some(malloc_cb), Some(free_cb));
        o.push(format!(
            "get_mem_ptr(NULL) null={}",
            (api.png_get_mem_ptr)(null_mut()).is_null()
        ));
        destroy_read(api, png, info);

        for write in [false, true] {
            let p = if write {
                (api.png_create_write_struct_2)(
                    VER,
                    null_mut(),
                    Some(sh.error_fn),
                    Some(warn_cb),
                    mem_tag(),
                    Some(malloc_cb),
                    Some(free_cb),
                )
            } else {
                (api.png_create_read_struct_2)(
                    VER,
                    null_mut(),
                    Some(sh.error_fn),
                    Some(warn_cb),
                    mem_tag(),
                    Some(malloc_cb),
                    Some(free_cb),
                )
            };
            o.push(format!("create_2 write={} null={}", write, p.is_null()));
            o.push(format!("mem_ptr==tag {}", (api.png_get_mem_ptr)(p) == mem_tag()));
            let i = (api.png_create_info_struct)(p);
            o.push(format!("info null={}", i.is_null()));
            if write {
                destroy_write(api, p, i);
            } else {
                destroy_read(api, p, i);
            }
        }
        o.push(format!("frees={}", tls().counter));
        o
    });

    /* ---- the allocation-size sequence of a whole write and read ---- */
    let mut rng = Rng::new(0xa11c);
    let img = Img::random(&mut rng, 13, 7, PNG_COLOR_TYPE_RGB, 8);
    let mut file = Vec::new();
    same("alloc sequence: write", |api| unsafe {
        let mut o = Outcome::default();
        let sh = &libs().shim;
        let png = (api.png_create_write_struct_2)(
            VER,
            null_mut(),
            Some(sh.error_fn),
            Some(warn_cb),
            mem_tag(),
            Some(malloc_cb),
            Some(free_cb),
        );
        assert!(!png.is_null());
        let info = (api.png_create_info_struct)(png);
        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for r in &img.rows {
                (api.png_write_row)(png, r.as_ptr());
            }
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        if api.which == "C" {
            file = o.output.clone();
        }
        destroy_write(api, png, info);
        o.push(format!(
            "allocs={} frees={}",
            tls().allocs.len(),
            tls().counter
        ));
        o.push(format!(
            "sizes={:?}",
            tls().allocs.iter().map(|&(_, s)| s).collect::<Vec<_>>()
        ));
        o
    });
    assert!(!file.is_empty());

    same("alloc sequence: read", |api| unsafe {
        let mut o = Outcome::default();
        let sh = &libs().shim;
        tls().input = file.clone();
        tls().in_pos = 0;
        let png = (api.png_create_read_struct_2)(
            VER,
            null_mut(),
            Some(sh.error_fn),
            Some(warn_cb),
            mem_tag(),
            Some(malloc_cb),
            Some(free_cb),
        );
        assert!(!png.is_null());
        let info = (api.png_create_info_struct)(png);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            let h = (api.png_get_image_height)(png, info) as usize;
            let rb = (api.png_get_rowbytes)(png, info);
            let mut row = vec![0u8; rb];
            for _ in 0..h {
                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                o.output.extend_from_slice(&row);
            }
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o.push(format!(
            "allocs={} frees={}",
            tls().allocs.len(),
            tls().counter
        ));
        o.push(format!(
            "sizes={:?}",
            tls().allocs.iter().map(|&(_, s)| s).collect::<Vec<_>>()
        ));
        o
    });

    /* ---- the allocation-size sequence over every shape and both interlace
     *      modes: a difference in any internal buffer size shows up here ---- */
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng =
                Rng::new(0xa11d ^ ((ct as u64) << 16) ^ ((bd as u64) << 8) ^ (il as u64));
            let mut img = Img::random(&mut rng, 9, 5, ct, bd);
            img.interlace = il;
            let mut f = Vec::new();
            same(
                &format!("alloc sizes write ct={} bd={} il={}", ct, bd, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let sh = &libs().shim;
                    let png = (api.png_create_write_struct_2)(
                        VER,
                        null_mut(),
                        Some(sh.error_fn),
                        Some(warn_cb),
                        mem_tag(),
                        Some(malloc_cb),
                        Some(free_cb),
                    );
                    assert!(!png.is_null());
                    let info = (api.png_create_info_struct)(png);
                    (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            img.w,
                            img.h,
                            bd,
                            ct,
                            il,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if ct == PNG_COLOR_TYPE_PALETTE {
                            (api.png_set_PLTE)(
                                png,
                                info,
                                img.palette.as_ptr(),
                                img.palette.len() as c_int,
                            );
                        }
                        (api.png_write_info)(png, info);
                        let passes = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        for _ in 0..passes {
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                        }
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    if api.which == "C" {
                        f = o.output.clone();
                    }
                    destroy_write(api, png, info);
                    o.push(format!(
                        "sizes={:?} frees={}",
                        tls().allocs.iter().map(|&(_, s)| s).collect::<Vec<_>>(),
                        tls().counter
                    ));
                    o
                },
            );
            same(
                &format!("alloc sizes read ct={} bd={} il={}", ct, bd, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let sh = &libs().shim;
                    tls().input = f.clone();
                    tls().in_pos = 0;
                    let png = (api.png_create_read_struct_2)(
                        VER,
                        null_mut(),
                        Some(sh.error_fn),
                        Some(warn_cb),
                        mem_tag(),
                        Some(malloc_cb),
                        Some(free_cb),
                    );
                    assert!(!png.is_null());
                    let info = (api.png_create_info_struct)(png);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        (api.png_read_update_info)(png, info);
                        let h = (api.png_get_image_height)(png, info) as usize;
                        let rb = (api.png_get_rowbytes)(png, info);
                        let passes = if (api.png_get_interlace_type)(png, info) as c_int
                            == PNG_INTERLACE_ADAM7
                        {
                            7
                        } else {
                            1
                        };
                        let mut rows = vec![vec![0u8; rb]; h];
                        for _ in 0..passes {
                            for r in rows.iter_mut() {
                                (api.png_read_row)(png, r.as_mut_ptr(), null_mut());
                            }
                        }
                        for r in &rows {
                            o.output.extend_from_slice(r);
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o.push(format!(
                        "sizes={:?} frees={}",
                        tls().allocs.iter().map(|&(_, s)| s).collect::<Vec<_>>(),
                        tls().counter
                    ));
                    o
                },
            );
        }
    }

    /* ---- the chunk-heavy file: exercises png_malloc_array /
     *      png_realloc_array for the text, sPLT and unknown-chunk arrays ---- */
    let rich = rich_file();
    for keep in [
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        PNG_HANDLE_CHUNK_NEVER,
        PNG_HANDLE_CHUNK_IF_SAFE,
        PNG_HANDLE_CHUNK_ALWAYS,
    ] {
        same(&format!("alloc sizes chunk-heavy keep={}", keep), |api| unsafe {
            let mut o = Outcome::default();
            let sh = &libs().shim;
            tls().input = rich.clone();
            tls().in_pos = 0;
            let png = (api.png_create_read_struct_2)(
                VER,
                null_mut(),
                Some(sh.error_fn),
                Some(warn_cb),
                mem_tag(),
                Some(budget_malloc),
                Some(budget_free),
            );
            BUDGET.with(|c| c.set(-1));
            assert!(!png.is_null());
            let info = (api.png_create_info_struct)(png);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            (api.png_set_keep_unknown_chunks)(png, keep, null(), 0);
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                log_info(api, png, info, "rich");
                let h = (api.png_get_image_height)(png, info) as usize;
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..h {
                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o.push(format!("frees={}", tls().counter));
            o
        });
    }

    /* ---- an allocator that refuses everything after the N'th call ---- */
    let budget_read_file = rgb_file(7, 3, 0xbad1);
    for n in 0..14i64 {
        for write in [false, true] {
            samef(
                &format!("alloc budget {} write={}", n, write),
                |api| unsafe {
                    let sh = &libs().shim;
                    BUDGET.with(|c| c.set(n));
                    let png = if write {
                        (api.png_create_write_struct_2)(
                            VER,
                            null_mut(),
                            Some(sh.error_fn),
                            Some(warn_cb),
                            mem_tag(),
                            Some(budget_malloc),
                            Some(budget_free),
                        )
                    } else {
                        (api.png_create_read_struct_2)(
                            VER,
                            null_mut(),
                            Some(sh.error_fn),
                            Some(warn_cb),
                            mem_tag(),
                            Some(budget_malloc),
                            Some(budget_free),
                        )
                    };
                    log(format!("create null={}", png.is_null()));
                    if png.is_null() {
                        return "create failed".to_string();
                    }
                    let info = (api.png_create_info_struct)(png);
                    log(format!("info null={}", info.is_null()));
                    let mut rng = Rng::new(0xbad0);
                    let img = Img::random(&mut rng, 7, 3, PNG_COLOR_TYPE_RGB, 8);
                    let g = if write {
                        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                        guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                img.w,
                                img.h,
                                8,
                                PNG_COLOR_TYPE_RGB,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (api.png_write_info)(png, info);
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        })
                    } else {
                        tls().input = budget_read_file.clone();
                        tls().in_pos = 0;
                        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                        guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            let h = (api.png_get_image_height)(png, info) as usize;
                            let rb = (api.png_get_rowbytes)(png, info);
                            let mut row = vec![0u8; rb];
                            for _ in 0..h {
                                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                            }
                            (api.png_read_end)(png, info);
                        })
                    };
                    format!("{:?} out={} bytes", g, tls().output.len())
                },
            );
        }
    }

    /* ---- png_set_mem_fn installed *after* creation ---- */
    let midlife = rgb_file(6, 3, 0xabc);
    same("set_mem_fn mid-life", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        tls().input = midlife.clone();
        tls().in_pos = 0;
        // the png_struct itself came from plain malloc; from here on the user
        // allocator handles everything else
        (api.png_set_mem_fn)(png, mem_tag(), Some(malloc_cb), Some(free_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            let h = (api.png_get_image_height)(png, info) as usize;
            let rb = (api.png_get_rowbytes)(png, info);
            let mut row = vec![0u8; rb];
            for _ in 0..h {
                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
            }
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o.push(format!(
            "sizes={:?} frees={}",
            tls().allocs.iter().map(|&(_, s)| s).collect::<Vec<_>>(),
            tls().counter
        ));
        o
    });

    report("custom_alloc");
}

/* ================================================================== */
/* C-146 — the row status callbacks                                    */
/* ================================================================== */

/// C-146: `png_set_read_status_fn` / `png_set_write_status_fn` over every legal
/// shape, interlaced and not.
#[test]
fn status_callbacks() {
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(0x57a7 ^ ((ct as u64) << 16) ^ ((bd as u64) << 8) ^ il as u64);
            let mut img = Img::random(&mut rng, 9, 5, ct, bd);
            img.interlace = il;
            let opts = WriteOpts {
                status_fn: true,
                ..Default::default()
            };
            let mut file = Vec::new();
            same(
                &format!("status write ct={} bd={} il={}", ct, bd, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let wr = write_plain(api, &img, &opts);
                    o.push(format!("guard={:?}", wr.guard));
                    o.output = wr.bytes.clone();
                    if api.which == "C" {
                        file = wr.bytes.clone();
                    }
                    o
                },
            );
            let ropts = ReadOpts {
                status_fn: true,
                ..Default::default()
            };
            same(
                &format!("status read ct={} bd={} il={}", ct, bd, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let rr = read_plain(api, &file, &ropts);
                    o.push(format!("guard={:?}", rr.guard));
                    for r in &rr.rows {
                        o.output.extend_from_slice(r);
                    }
                    o
                },
            );
        }
    }

    /* different image shapes (1 row, 1 column, tall, wide) */
    for &(w, h) in &[(1u32, 1u32), (1, 13), (13, 1), (17, 2), (2, 17), (33, 33)] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(0x57a8 ^ (w as u64) ^ ((h as u64) << 16) ^ ((il as u64) << 32));
            let mut img = Img::random(&mut rng, w, h, PNG_COLOR_TYPE_GRAY_ALPHA, 8);
            img.interlace = il;
            let mut file = Vec::new();
            same(&format!("status write {}x{} il={}", w, h, il), |api| unsafe {
                let mut o = Outcome::default();
                let wr = write_plain(
                    api,
                    &img,
                    &WriteOpts {
                        status_fn: true,
                        ..Default::default()
                    },
                );
                o.push(format!("guard={:?}", wr.guard));
                o.output = wr.bytes.clone();
                if api.which == "C" {
                    file = wr.bytes.clone();
                }
                o
            });
            same(&format!("status read {}x{} il={}", w, h, il), |api| unsafe {
                let mut o = Outcome::default();
                let rr = read_plain(
                    api,
                    &file,
                    &ReadOpts {
                        status_fn: true,
                        ..Default::default()
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

    /* installing / removing the callback part way through, and NULL png_ptr */
    let file = rgb_file(8, 6, 0x57a9);
    same("status fn toggled mid-stream", |api| unsafe {
        let mut o = Outcome::default();
        tls().input = file.clone();
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        (api.png_set_read_status_fn)(null_mut(), Some(read_status_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            let h = (api.png_get_image_height)(png, info) as usize;
            let rb = (api.png_get_rowbytes)(png, info);
            let mut row = vec![0u8; rb];
            for y in 0..h {
                if y == 2 {
                    (api.png_set_read_status_fn)(png, Some(read_status_cb));
                }
                if y == 4 {
                    (api.png_set_read_status_fn)(png, None);
                }
                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
            }
            (api.png_read_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });

    let mut rng = Rng::new(0x57aa);
    let img = Img::random(&mut rng, 8, 6, PNG_COLOR_TYPE_RGB, 8);
    same("write status fn toggled mid-stream", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
        (api.png_set_write_status_fn)(null_mut(), Some(write_status_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                img.w,
                img.h,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            for (y, r) in img.rows.iter().enumerate() {
                if y == 1 {
                    (api.png_set_write_status_fn)(png, Some(write_status_cb));
                }
                if y == 4 {
                    (api.png_set_write_status_fn)(png, None);
                }
                (api.png_write_row)(png, r.as_ptr());
            }
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o
    });

    report("status_callbacks");
}

/* ================================================================== */
/* C-149 — png_get_current_row_number / png_get_current_pass_number     */
/* ================================================================== */

unsafe extern "C" fn rn_read_status(png: *mut PngStruct, row: u32, pass: c_int) {
    let api = cur_api();
    log(format!(
        "read status row={} pass={} cur_row={} cur_pass={}",
        row,
        pass,
        (api.png_get_current_row_number)(png),
        (api.png_get_current_pass_number)(png)
    ));
}

unsafe extern "C" fn rn_write_status(png: *mut PngStruct, row: u32, pass: c_int) {
    let api = cur_api();
    log(format!(
        "write status row={} pass={} cur_row={} cur_pass={}",
        row,
        pass,
        (api.png_get_current_row_number)(png),
        (api.png_get_current_pass_number)(png)
    ));
}

unsafe extern "C" fn rn_transform(
    png: *mut PngStruct,
    row_info: *mut png_row_info,
    _row: *mut u8,
) {
    let api = cur_api();
    let ri = *row_info;
    log(format!(
        "transform w={} rb={} cur_row={} cur_pass={} uptr_null={}",
        ri.width,
        ri.rowbytes,
        (api.png_get_current_row_number)(png),
        (api.png_get_current_pass_number)(png),
        (api.png_get_user_transform_ptr)(png).is_null()
    ));
}

/// C-149: the current row / pass number, sampled from the status callback and
/// from a user transform, for reads and writes, interlaced and not.
#[test]
fn row_number() {
    same("row/pass number of a NULL png_ptr", |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!(
            "row={} pass={}",
            (api.png_get_current_row_number)(null_mut()),
            (api.png_get_current_pass_number)(null_mut())
        ));
        let (png, info) = new_read(api);
        o.push(format!(
            "fresh read row={} pass={}",
            (api.png_get_current_row_number)(png),
            (api.png_get_current_pass_number)(png)
        ));
        destroy_read(api, png, info);
        let (png, info) = new_write(api);
        o.push(format!(
            "fresh write row={} pass={}",
            (api.png_get_current_row_number)(png),
            (api.png_get_current_pass_number)(png)
        ));
        destroy_write(api, png, info);
        o
    });

    for &(w, h) in &[(1u32, 1u32), (5, 3), (9, 9), (17, 2), (2, 17)] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng = Rng::new(0x2ec ^ w as u64 ^ ((h as u64) << 20) ^ ((il as u64) << 40));
            let mut img = Img::random(&mut rng, w, h, PNG_COLOR_TYPE_RGB, 8);
            img.interlace = il;
            let mut file = Vec::new();

            same(
                &format!("row number write {}x{} il={}", w, h, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                    (api.png_set_write_status_fn)(png, Some(rn_write_status));
                    (api.png_set_write_user_transform_fn)(png, Some(rn_transform));
                    (api.png_set_user_transform_info)(png, mem_tag(), 8, 3);
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            img.w,
                            img.h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            img.interlace,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        (api.png_write_info)(png, info);
                        let passes = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        for _ in 0..passes {
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                        }
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!(
                        "guard={:?} final row={} pass={}",
                        g,
                        (api.png_get_current_row_number)(png),
                        (api.png_get_current_pass_number)(png)
                    ));
                    o.output = std::mem::take(&mut tls().output);
                    if api.which == "C" {
                        file = o.output.clone();
                    }
                    destroy_write(api, png, info);
                    o
                },
            );

            same(
                &format!("row number read {}x{} il={}", w, h, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = file.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    (api.png_set_read_status_fn)(png, Some(rn_read_status));
                    (api.png_set_read_user_transform_fn)(png, Some(rn_transform));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        (api.png_read_update_info)(png, info);
                        let hh = (api.png_get_image_height)(png, info) as usize;
                        let rb = (api.png_get_rowbytes)(png, info);
                        let passes = if (api.png_get_interlace_type)(png, info) as c_int
                            == PNG_INTERLACE_ADAM7
                        {
                            7
                        } else {
                            1
                        };
                        let mut rows = vec![vec![0u8; rb]; hh];
                        for _ in 0..passes {
                            for r in rows.iter_mut() {
                                (api.png_read_row)(png, r.as_mut_ptr(), null_mut());
                            }
                        }
                        for r in &rows {
                            log(format!("row {:02x?}", r));
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!(
                        "guard={:?} final row={} pass={}",
                        g,
                        (api.png_get_current_row_number)(png),
                        (api.png_get_current_pass_number)(png)
                    ));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* the row/pass number sampled from the caller's own loop, for every legal
     * shape and both interlace modes, and through every row entry point */
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let mut rng =
                Rng::new(0x2ed ^ ((ct as u64) << 16) ^ ((bd as u64) << 8) ^ (il as u64));
            let mut img = Img::random(&mut rng, 11, 6, ct, bd);
            img.interlace = il;
            let mut file = Vec::new();
            same(
                &format!("row number sampled write ct={} bd={} il={}", ct, bd, il),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                    (api.png_set_write_status_fn)(png, Some(rn_write_status));
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            img.w,
                            img.h,
                            bd,
                            ct,
                            il,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if ct == PNG_COLOR_TYPE_PALETTE {
                            (api.png_set_PLTE)(
                                png,
                                info,
                                img.palette.as_ptr(),
                                img.palette.len() as c_int,
                            );
                        }
                        (api.png_write_info)(png, info);
                        let passes = if il == PNG_INTERLACE_ADAM7 {
                            (api.png_set_interlace_handling)(png)
                        } else {
                            1
                        };
                        for p in 0..passes {
                            for (y, r) in img.rows.iter().enumerate() {
                                (api.png_write_row)(png, r.as_ptr());
                                log(format!(
                                    "after write p={} y={} row={} pass={}",
                                    p,
                                    y,
                                    (api.png_get_current_row_number)(png),
                                    (api.png_get_current_pass_number)(png)
                                ));
                            }
                        }
                        (api.png_write_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    if api.which == "C" {
                        file = o.output.clone();
                    }
                    destroy_write(api, png, info);
                    o
                },
            );
            for mode in [RowMode::Row, RowMode::Rows(2), RowMode::Image] {
                same(
                    &format!(
                        "row number sampled read ct={} bd={} il={} {:?}",
                        ct, bd, il, mode
                    ),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        tls().input = file.clone();
                        tls().in_pos = 0;
                        let (png, info) = new_read(api);
                        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                        (api.png_set_read_status_fn)(png, Some(rn_read_status));
                        let g = guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            (api.png_read_update_info)(png, info);
                            let hh = (api.png_get_image_height)(png, info) as usize;
                            let rb = (api.png_get_rowbytes)(png, info);
                            let passes = if (api.png_get_interlace_type)(png, info) as c_int
                                == PNG_INTERLACE_ADAM7
                            {
                                7
                            } else {
                                1
                            };
                            let mut rows = vec![vec![0u8; rb]; hh];
                            match mode {
                                RowMode::Image => {
                                    let mut ptrs: Vec<*mut u8> =
                                        rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
                                    (api.png_read_image)(png, ptrs.as_mut_ptr());
                                }
                                RowMode::Rows(n) => {
                                    for _ in 0..passes {
                                        let mut y = 0;
                                        while y < hh {
                                            let k = n.min(hh - y);
                                            let mut ptrs: Vec<*mut u8> = rows[y..y + k]
                                                .iter()
                                                .map(|r| r.as_ptr() as *mut u8)
                                                .collect();
                                            (api.png_read_rows)(
                                                png,
                                                ptrs.as_mut_ptr(),
                                                null_mut(),
                                                k as u32,
                                            );
                                            y += k;
                                            log(format!(
                                                "after read_rows row={} pass={}",
                                                (api.png_get_current_row_number)(png),
                                                (api.png_get_current_pass_number)(png)
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    for _ in 0..passes {
                                        for r in rows.iter_mut() {
                                            (api.png_read_row)(png, r.as_mut_ptr(), null_mut());
                                            log(format!(
                                                "after read_row row={} pass={}",
                                                (api.png_get_current_row_number)(png),
                                                (api.png_get_current_pass_number)(png)
                                            ));
                                        }
                                    }
                                }
                            }
                            for r in &rows {
                                o.output.extend_from_slice(r);
                            }
                            (api.png_read_end)(png, info);
                        });
                        o.push(format!(
                            "guard={:?} final row={} pass={}",
                            g,
                            (api.png_get_current_row_number)(png),
                            (api.png_get_current_pass_number)(png)
                        ));
                        destroy_read(api, png, info);
                        o
                    },
                );
            }
        }
    }

    report("row_number");
}

/* ================================================================== */
/* C-148 — png_set_option / png_permit_mng_features / png_reset_zstream */
/* ================================================================== */

/// C-148: every option number × every `onoff` value, the returned previous
/// state, and the three options that actually change behaviour.
#[test]
fn options() {
    /* ---- the whole option matrix, on read and on write structs ---- */
    let onoffs: [c_int; 8] = [
        PNG_OPTION_ON,
        PNG_OPTION_OFF,
        0,
        1,
        -1,
        5,
        255,
        i32::MIN,
    ];
    for write in [false, true] {
        for opt in 0..18i32 {
            same(&format!("set_option {} write={}", opt, write), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = if write { new_write(api) } else { new_read(api) };
                for &v in &onoffs {
                    // twice, so the *previous state* return value is exercised
                    let a = (api.png_set_option)(png, opt, v);
                    let b = (api.png_set_option)(png, opt, v);
                    o.push(format!("opt={} onoff={} -> {} then {}", opt, v, a, b));
                }
                // and now flip between on and off repeatedly
                for &v in &[0, 1, 0, 0, 1, 1] {
                    o.push(format!(
                        "flip {} -> {}",
                        v,
                        (api.png_set_option)(png, opt, v)
                    ));
                }
                if write {
                    destroy_write(api, png, info);
                } else {
                    destroy_read(api, png, info);
                }
                o.push(format!(
                    "NULL png -> {}",
                    (api.png_set_option)(null_mut(), opt, PNG_OPTION_ON)
                ));
                o
            });
        }
    }

    /* every option set at once, in both orders */
    same("all options at once", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_read(api);
        for opt in 0..18i32 {
            o.push(format!("on {} -> {}", opt, (api.png_set_option)(png, opt, 1)));
        }
        for opt in (0..18i32).rev() {
            o.push(format!("off {} -> {}", opt, (api.png_set_option)(png, opt, 0)));
        }
        for opt in 0..18i32 {
            o.push(format!(
                "read back {} -> {}",
                opt,
                (api.png_set_option)(png, opt, 0)
            ));
        }
        destroy_read(api, png, info);
        o
    });

    /* ---- PNG_MAXIMUM_INFLATE_WINDOW ---- */
    let base = rgb_file(6, 4, 0x0071);
    // a zlib header claiming a 64 KiB window (CINFO = 8): libpng rejects this
    // itself unless the maximum inflate window has been forced on.
    let big_window = patch_idat(&base, |d| {
        d[0] = 0x88;
        d[1] = 0x1c;
    });
    // a stream written with a genuinely small window
    let small_window = with_c(|api| unsafe {
        let mut rng = Rng::new(0x5a11);
        let img = Img::random(&mut rng, 20, 10, PNG_COLOR_TYPE_RGB, 8);
        write_plain(
            api,
            &img,
            &WriteOpts {
                window_bits: Some(9),
                level: Some(9),
                ..Default::default()
            },
        )
        .bytes
    });
    for (tag, data) in [
        ("valid", base.clone()),
        ("cinfo=8", big_window.clone()),
        ("window_bits=9", small_window.clone()),
    ] {
        for on in [0, 1] {
            same(
                &format!("MAXIMUM_INFLATE_WINDOW {} on={}", tag, on),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = data.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    o.push(format!(
                        "prev={}",
                        (api.png_set_option)(png, PNG_MAXIMUM_INFLATE_WINDOW, on)
                    ));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        let h = (api.png_get_image_height)(png, info) as usize;
                        let rb = (api.png_get_rowbytes)(png, info);
                        let mut row = vec![0u8; rb];
                        for _ in 0..h {
                            (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                            log(format!("row {:02x?}", row));
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- PNG_IGNORE_ADLER32 ---- */
    let bad_adler = patch_idat(&base, |d| {
        let n = d.len();
        d[n - 1] ^= 0xff;
    });
    let bad_adler2 = patch_idat(&base, |d| {
        let n = d.len();
        d[n - 4] ^= 0x80;
    });
    for (tag, data) in [
        ("valid", base.clone()),
        ("adler last byte", bad_adler),
        ("adler first byte", bad_adler2),
    ] {
        for on in [0, 1] {
            same(&format!("IGNORE_ADLER32 {} on={}", tag, on), |api| unsafe {
                let mut o = Outcome::default();
                tls().input = data.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                o.push(format!(
                    "prev={}",
                    (api.png_set_option)(png, PNG_IGNORE_ADLER32, on)
                ));
                let g = guarded(api, png, &mut || {
                    (api.png_read_info)(png, info);
                    let h = (api.png_get_image_height)(png, info) as usize;
                    let rb = (api.png_get_rowbytes)(png, info);
                    let mut row = vec![0u8; rb];
                    for _ in 0..h {
                        (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                        log(format!("row {:02x?}", row));
                    }
                    (api.png_read_end)(png, info);
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            });
        }
    }

    /* ---- PNG_SKIP_sRGB_CHECK_PROFILE on an iCCP chunk ---- */
    let good = icc_profile();
    let mut bad = icc_profile();
    bad[16..20].copy_from_slice(b"GRAY"); // wrong colour space for an RGB PNG
    let mut nod50 = icc_profile();
    for b in nod50[68..80].iter_mut() {
        *b = 0;
    }
    for (tag, prof) in [("good", good), ("gray space", bad), ("not D50", nod50)] {
        let mut d = b"an icc profile\0\0".to_vec();
        d.extend_from_slice(&zlib_stored(&prof));
        let file = insert_before(&base, "IDAT", &chunk(b"iCCP", &d));
        for on in [0, 1] {
            same(
                &format!("SKIP_sRGB_CHECK_PROFILE {} on={}", tag, on),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = file.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    o.push(format!(
                        "prev={}",
                        (api.png_set_option)(png, PNG_SKIP_sRGB_CHECK_PROFILE, on)
                    ));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        log_info(api, png, info, "iccp");
                        let mut name: *mut c_char = null_mut();
                        let mut comp = 0;
                        let mut pp: *mut u8 = null_mut();
                        let mut plen = 0u32;
                        let r = (api.png_get_iCCP)(
                            png, info, &mut name, &mut comp, &mut pp, &mut plen,
                        );
                        log(format!("get_iCCP r={} comp={} len={}", r, comp, plen));
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- png_permit_mng_features ---- */
    same("png_permit_mng_features", |api| unsafe {
        let mut o = Outcome::default();
        for write in [false, true] {
            let (png, info) = if write { new_write(api) } else { new_read(api) };
            for v in 0..9u32 {
                o.push(format!(
                    "write={} permit({}) -> {}",
                    write,
                    v,
                    (api.png_permit_mng_features)(png, v)
                ));
            }
            for &v in &[0xffff_ffffu32, 5, 4, 1, 0] {
                o.push(format!(
                    "write={} permit({}) -> {}",
                    write,
                    v,
                    (api.png_permit_mng_features)(png, v)
                ));
            }
            if write {
                destroy_write(api, png, info);
            } else {
                destroy_read(api, png, info);
            }
        }
        o.push(format!(
            "permit(NULL,5) -> {}",
            (api.png_permit_mng_features)(null_mut(), 5)
        ));
        o
    });

    // MNG features permitted while reading a real PNG datastream warns.
    for v in 0..6u32 {
        same(&format!("mng features {} on a PNG", v), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = base.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            o.push(format!("permitted={}", (api.png_permit_mng_features)(png, v)));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                log_info(api, png, info, "mng");
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- what png_permit_mng_features actually unlocks:
     *      PNG_INTRAPIXEL_DIFFERENCING (filter method 64), which is only legal
     *      when libpng has *not* seen a PNG signature, i.e. when the datastream
     *      is embedded in an MNG one.  `png_set_sig_bytes(png, >= 3)` is what
     *      keeps PNG_HAVE_PNG_SIGNATURE clear. ---- */
    let mng_shapes: [(c_int, c_int); 5] = [
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ];

    // write side
    for &(ct, bd) in &mng_shapes {
        for &mng in &[0u32, PNG_FLAG_MNG_FILTER_64 as u32, PNG_ALL_MNG_FEATURES as u32] {
            for &sig in &[0i32, 4, 8] {
                let mut rng =
                    Rng::new(0x11a7 ^ (ct as u64) ^ ((bd as u64) << 8) ^ ((mng as u64) << 16));
                let img = Img::random(&mut rng, 7, 4, ct, bd);
                same(
                    &format!(
                        "mng write filter64 ct={} bd={} mng={} sig={}",
                        ct, bd, mng, sig
                    ),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = new_write(api);
                        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                        o.push(format!("permitted={}", (api.png_permit_mng_features)(png, mng)));
                        (api.png_set_sig_bytes)(png, sig);
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                img.w,
                                img.h,
                                bd,
                                ct,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_INTRAPIXEL_DIFFERENCING,
                            );
                            if ct == PNG_COLOR_TYPE_PALETTE {
                                (api.png_set_PLTE)(
                                    png,
                                    info,
                                    img.palette.as_ptr(),
                                    img.palette.len() as c_int,
                                );
                            }
                            (api.png_write_info)(png, info);
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        });
                        o.push(format!("guard={:?}", g));
                        o.output = std::mem::take(&mut tls().output);
                        destroy_write(api, png, info);
                        o
                    },
                );
            }
        }
    }

    // read side: a normal datastream whose IHDR claims filter method 64
    for &(ct, bd) in &mng_shapes {
        let mut rng = Rng::new(0x11a8 ^ (ct as u64) ^ ((bd as u64) << 8));
        let img = Img::random(&mut rng, 7, 4, ct, bd);
        let plain = with_c(|api| unsafe { write_plain(api, &img, &WriteOpts::default()).bytes });
        for &filt in &[64u8, 1, 2, 65, 255] {
            let patched = set_ihdr_filter(&plain, filt);
            for &sig in &[0usize, 4, 8] {
                for &mng in &[0u32, PNG_FLAG_MNG_FILTER_64 as u32, PNG_ALL_MNG_FEATURES as u32] {
                    same(
                        &format!(
                            "mng read filter={} ct={} bd={} sig={} mng={}",
                            filt, ct, bd, sig, mng
                        ),
                        |api| unsafe {
                            let mut o = Outcome::default();
                            tls().input = patched[sig..].to_vec();
                            tls().in_pos = 0;
                            let (png, info) = new_read(api);
                            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                            o.push(format!(
                                "permitted={}",
                                (api.png_permit_mng_features)(png, mng)
                            ));
                            (api.png_set_sig_bytes)(png, sig as c_int);
                            let g = guarded(api, png, &mut || {
                                (api.png_read_info)(png, info);
                                log_info(api, png, info, "mng");
                                let h = (api.png_get_image_height)(png, info) as usize;
                                let rb = (api.png_get_rowbytes)(png, info);
                                let mut row = vec![0u8; rb];
                                for y in 0..h {
                                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                                    log(format!("row {} {:02x?}", y, row));
                                }
                                (api.png_read_end)(png, info);
                            });
                            o.push(format!("guard={:?}", g));
                            destroy_read(api, png, info);
                            o
                        },
                    );
                }
            }
        }
    }

    /* a genuine MNG-embedded round trip: write with intrapixel differencing,
     * then read it back and check the pixels survive */
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        let mut rng = Rng::new(0x11a9 ^ (ct as u64) ^ ((bd as u64) << 8));
        let img = Img::random(&mut rng, 9, 5, ct, bd);
        let embedded = with_c(|api| unsafe {
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
            (api.png_permit_mng_features)(png, PNG_FLAG_MNG_FILTER_64 as u32);
            (api.png_set_sig_bytes)(png, 8);
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    img.w,
                    img.h,
                    bd,
                    ct,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_INTRAPIXEL_DIFFERENCING,
                );
                (api.png_write_info)(png, info);
                for r in &img.rows {
                    (api.png_write_row)(png, r.as_ptr());
                }
                (api.png_write_end)(png, info);
            });
            assert_eq!(g, Guard::Ok, "MNG-embedded write failed");
            destroy_write(api, png, info);
            std::mem::take(&mut tls().output)
        });
        assert!(!embedded.is_empty());
        for &mng in &[0u32, PNG_FLAG_MNG_FILTER_64 as u32] {
            same(
                &format!("mng round trip ct={} bd={} mng={}", ct, bd, mng),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = embedded.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
                    (api.png_permit_mng_features)(png, mng);
                    (api.png_set_sig_bytes)(png, 8);
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        let h = (api.png_get_image_height)(png, info) as usize;
                        let rb = (api.png_get_rowbytes)(png, info);
                        let mut row = vec![0u8; rb];
                        for y in 0..h {
                            (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                            log(format!("row {} matches={}", y, row == img.rows[y]));
                            o.output.extend_from_slice(&row);
                        }
                        (api.png_read_end)(png, info);
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_read(api, png, info);
                    o
                },
            );
        }
    }

    /* ---- PNG_FLAG_MNG_EMPTY_PLTE ---- */
    for &mng in &[0u32, PNG_FLAG_MNG_EMPTY_PLTE as u32, PNG_ALL_MNG_FEATURES as u32] {
        for &(ct, npal) in &[
            (PNG_COLOR_TYPE_PALETTE, 0i32),
            (PNG_COLOR_TYPE_PALETTE, 1),
            (PNG_COLOR_TYPE_RGB, 0),
            (PNG_COLOR_TYPE_RGB, 4),
        ] {
            for null_palette in [false, true] {
                same(
                    &format!(
                        "empty PLTE mng={} ct={} npal={} null={}",
                        mng, ct, npal, null_palette
                    ),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let mut rng = Rng::new(0x3117 ^ (mng as u64) ^ ((npal as u64) << 16));
                        let img = Img::random(&mut rng, 4, 2, ct, 8);
                        let pal: Vec<png_color> = (0..4)
                            .map(|_| png_color {
                                red: rng.u8(),
                                green: rng.u8(),
                                blue: rng.u8(),
                            })
                            .collect();
                        let (png, info) = new_write(api);
                        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
                        (api.png_permit_mng_features)(png, mng);
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                img.w,
                                img.h,
                                8,
                                ct,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (api.png_set_PLTE)(
                                png,
                                info,
                                if null_palette { null() } else { pal.as_ptr() },
                                npal,
                            );
                            log(format!(
                                "valid after set_PLTE = {:#x}",
                                (api.png_get_valid)(png, info, 0xffff_ffff)
                            ));
                            let bg = png_color_16 {
                                index: 3,
                                red: 1,
                                green: 2,
                                blue: 3,
                                gray: 4,
                            };
                            (api.png_set_bKGD)(png, info, &bg);
                            (api.png_write_info)(png, info);
                            for r in &img.rows {
                                (api.png_write_row)(png, r.as_ptr());
                            }
                            (api.png_write_end)(png, info);
                        });
                        o.push(format!("guard={:?}", g));
                        o.output = std::mem::take(&mut tls().output);
                        destroy_write(api, png, info);
                        o
                    },
                );
            }
        }
    }

    /* ---- png_reset_zstream ---- */
    same("png_reset_zstream", |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!(
            "reset(NULL) -> {}",
            (api.png_reset_zstream)(null_mut())
        ));
        tls().input = base.clone();
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        o.push(format!("fresh -> {}", (api.png_reset_zstream)(png)));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
            log(format!("after read_info -> {}", (api.png_reset_zstream)(png)));
            let h = (api.png_get_image_height)(png, info) as usize;
            let rb = (api.png_get_rowbytes)(png, info);
            let mut row = vec![0u8; rb];
            (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
            log(format!("after one row -> {}", (api.png_reset_zstream)(png)));
            for _ in 1..h {
                (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
            }
        });
        o.push(format!("guard={:?}", g));
        o.push(format!("after -> {}", (api.png_reset_zstream)(png)));
        destroy_read(api, png, info);
        o
    });

    report("options");
}

/* ================================================================== */
/* C-150 — png_set_longjmp_fn / png_longjmp                            */
/* ================================================================== */

/// glibc's `sizeof(jmp_buf)` on x86-64; the harness asserts this at start-up.
const JB: usize = 200;

/// C-150: `png_set_longjmp_fn` with every interesting `jmp_buf_size`, called
/// once and twice, with a NULL `longjmp_fn`, plus `png_longjmp` itself.
#[test]
fn longjmp_fn() {
    let sizes: [usize; 12] = [0, 1, 8, 100, JB - 1, JB, JB + 1, 256, 400, 1000, 4096, 65536];

    /* ---- one call on a fresh struct ---- */
    for &n in &sizes {
        for write in [false, true] {
            same(
                &format!("set_longjmp_fn once {} write={}", n, write),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = if write { new_write(api) } else { new_read(api) };
                    let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), n);
                    o.push(format!("size={} null={}", n, p.is_null()));
                    // whether the harness can still arm its own trap afterwards
                    // is itself an observation
                    let g = guarded(api, png, &mut || {
                        log("body ran".to_string());
                    });
                    o.push(format!("guard={:?}", g));
                    if write {
                        destroy_write(api, png, info);
                    } else {
                        destroy_read(api, png, info);
                    }
                    o
                },
            );
        }
    }

    /* ---- called twice: same size, then every other size ---- */
    for &a in &sizes {
        for &b in &sizes {
            same(&format!("set_longjmp_fn {} then {}", a, b), |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_read(api);
                let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), a);
                o.push(format!("first {} null={}", a, p.is_null()));
                let q = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), b);
                o.push(format!("second {} null={}", b, q.is_null()));
                o.push(format!("same pointer={}", p == q));
                let r = (api.png_set_longjmp_fn)(png, None, b);
                o.push(format!("third (NULL fn) null={}", r.is_null()));
                destroy_read(api, png, info);
                o
            });
        }
    }

    /* ---- NULL png_ptr, and png_free_jmpbuf ---- */
    same("set_longjmp_fn NULL png / free_jmpbuf", |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!(
            "NULL png null={}",
            (api.png_set_longjmp_fn)(null_mut(), Some(dummy_longjmp), JB).is_null()
        ));
        (api.png_free_jmpbuf)(null_mut());
        o.push("free_jmpbuf(NULL) survived".to_string());
        for &n in &[0usize, 100, JB, 1000] {
            let (png, info) = new_read(api);
            let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), n);
            o.push(format!("armed {} null={}", n, p.is_null()));
            (api.png_free_jmpbuf)(png);
            o.push("freed".to_string());
            // after png_free_jmpbuf the struct is back to its virgin state
            let q = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), n);
            o.push(format!("re-armed {} null={}", n, q.is_null()));
            (api.png_free_jmpbuf)(png);
            (api.png_free_jmpbuf)(png);
            o.push("double free_jmpbuf survived".to_string());
            let g = guarded(api, png, &mut || log("body".to_string()));
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
        }
        o
    });

    /* ---- png_longjmp through a properly armed trap ---- */
    for &val in &[1i32, 2, 42, -1, 0, i32::MAX] {
        same(&format!("png_longjmp({})", val), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || {
                log("before longjmp".to_string());
                (api.png_longjmp)(png, val);
                log("after longjmp (must not appear)".to_string());
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* ---- png_longjmp with no trap, or with a longjmp_fn that returns:
     *      both reach PNG_ABORT(), which is only survivable in a child. ---- */
    samef("png_longjmp with no jmp_buf", |api| unsafe {
        let (png, _info) = new_read(api);
        log("about to longjmp".to_string());
        (api.png_longjmp)(png, 1);
        "returned from png_longjmp".to_string()
    });

    samef("png_longjmp with a returning longjmp_fn", |api| unsafe {
        let (png, _info) = new_read(api);
        let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), JB);
        log(format!("armed null={}", p.is_null()));
        (api.png_longjmp)(png, 7);
        "returned from png_longjmp".to_string()
    });

    samef("png_longjmp with a NULL longjmp_fn", |api| unsafe {
        let (png, _info) = new_read(api);
        let p = (api.png_set_longjmp_fn)(png, None, JB);
        log(format!("armed null={}", p.is_null()));
        (api.png_longjmp)(png, 3);
        "returned from png_longjmp".to_string()
    });

    samef("png_longjmp(NULL, 1)", |api| unsafe {
        log("about to longjmp".to_string());
        (api.png_longjmp)(null_mut(), 1);
        "returned from png_longjmp".to_string()
    });

    /* ---- a real error still finds its way out after re-arming ---- */
    same("error after re-arming the trap", |api| unsafe {
        let mut o = Outcome::default();
        tls().input = b"definitely not a png".to_vec();
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
        let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), JB);
        o.push(format!("pre-armed null={}", p.is_null()));
        let g = guarded(api, png, &mut || {
            (api.png_read_info)(png, info);
        });
        o.push(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o
    });

    report("longjmp_fn");
}

/* ================================================================== */
/* C-151 — the struct lifecycle                                        */
/* ================================================================== */

/// C-151: `png_create_png_struct` … `png_destroy_info_struct`, in every
/// legal (and several illegal) orders.
#[test]
fn struct_lifecycle() {
    /* ---- png_create_png_struct / png_destroy_png_struct ---- */
    same("create/destroy png_struct", |api| unsafe {
        let mut o = Outcome::default();
        let sh = &libs().shim;
        for custom in [false, true] {
            let p = (api.png_create_png_struct)(
                VER,
                mem_tag(),
                Some(sh.error_fn),
                Some(warn_cb),
                if custom { mem_tag() } else { null_mut() },
                if custom { Some(malloc_cb) } else { None },
                if custom { Some(free_cb) } else { None },
            );
            o.push(format!("custom={} null={}", custom, p.is_null()));
            if !p.is_null() {
                o.push(format!(
                    "error_ptr==tag {} mem_ptr null={}",
                    (api.png_get_error_ptr)(p) == mem_tag(),
                    (api.png_get_mem_ptr)(p).is_null()
                ));
                let i = (api.png_create_info_struct)(p);
                o.push(format!("info null={}", i.is_null()));
                let mut ip = i;
                (api.png_destroy_info_struct)(p, &mut ip);
                o.push(format!("info cleared={}", ip.is_null()));
                (api.png_destroy_png_struct)(p);
                o.push("destroyed".to_string());
            }
        }
        (api.png_destroy_png_struct)(null_mut());
        o.push("destroy(NULL) survived".to_string());
        o.push(format!("frees={}", tls().counter));
        o
    });

    /* ---- the version check ---- */
    let long_ver: String = std::iter::repeat('9').take(100).collect();
    let versions: [(&str, Option<&str>); 10] = [
        ("current", Some("1.6.59.git")),
        ("1.6.0", Some("1.6.0")),
        ("1.6.", Some("1.6.")),
        ("1.6", Some("1.6")),
        ("1.0.0", Some("1.0.0")),
        ("1.5.30", Some("1.5.30")),
        ("2.6.0", Some("2.6.0")),
        ("empty", Some("")),
        ("100 chars", Some("")), // replaced below
        ("NULL", None),
    ];
    for (i, (tag, v)) in versions.iter().enumerate() {
        let owned: Option<String> = if i == 8 {
            Some(long_ver.clone())
        } else {
            v.map(|s| s.to_string())
        };
        for write in [false, true] {
            same(
                &format!("user_png_ver {} write={}", tag, write),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let sh = &libs().shim;
                    let held = owned.as_ref().map(|s| cs(s));
                    let vp: *const c_char = match &held {
                        Some(c) => c.as_ptr(),
                        None => null(),
                    };
                    let p = if write {
                        (api.png_create_write_struct)(
                            vp,
                            null_mut(),
                            Some(sh.error_fn),
                            Some(warn_cb),
                        )
                    } else {
                        (api.png_create_read_struct)(
                            vp,
                            null_mut(),
                            Some(sh.error_fn),
                            Some(warn_cb),
                        )
                    };
                    o.push(format!("create null={}", p.is_null()));
                    if !p.is_null() {
                        // and the standalone checker on a live struct
                        o.push(format!(
                            "user_version_check(same)={}",
                            (api.png_user_version_check)(p, vp)
                        ));
                        let i = (api.png_create_info_struct)(p);
                        if write {
                            destroy_write(api, p, i);
                        } else {
                            destroy_read(api, p, i);
                        }
                    }
                    o
                },
            );
        }
    }

    /* png_user_version_check on a struct we own, for every string */
    same("png_user_version_check matrix", |api| unsafe {
        let mut o = Outcome::default();
        for v in [
            Some("1.6.59.git"),
            Some("1.6.0"),
            Some("1.6"),
            Some("1."),
            Some("1"),
            Some(""),
            Some("1.0.0"),
            Some("0.6.59.git"),
            Some("1.7.0"),
            Some("1.6.59.gitextra"),
            None,
        ] {
            let (png, info) = new_read(api);
            let held = v.map(cs);
            let vp: *const c_char = match &held {
                Some(c) => c.as_ptr(),
                None => null(),
            };
            o.push(format!("check({:?}) = {}", v, (api.png_user_version_check)(png, vp)));
            destroy_read(api, png, info);
        }
        o
    });

    /* ---- png_create_info_struct / png_destroy_info_struct ---- */
    same("info struct lifecycle", |api| unsafe {
        let mut o = Outcome::default();
        o.push(format!(
            "create_info(NULL) null={}",
            (api.png_create_info_struct)(null_mut()).is_null()
        ));
        let (png, _) = new_read(api);
        let mut infos = Vec::new();
        for _ in 0..5 {
            let i = (api.png_create_info_struct)(png);
            o.push(format!("info null={}", i.is_null()));
            infos.push(i);
        }
        // destroy them all, then destroy again through the (now NULL) slots
        for i in infos.iter_mut() {
            (api.png_destroy_info_struct)(png, i);
            o.push(format!("cleared={}", i.is_null()));
            (api.png_destroy_info_struct)(png, i);
            o.push("second destroy survived".to_string());
        }
        (api.png_destroy_info_struct)(png, null_mut());
        let mut i = (api.png_create_info_struct)(png);
        (api.png_destroy_info_struct)(null_mut(), &mut i);
        o.push(format!("destroy with NULL png: still set={}", !i.is_null()));
        (api.png_destroy_info_struct)(png, &mut i);
        let mut nul: *mut PngInfo = null_mut();
        (api.png_destroy_info_struct)(png, &mut nul);
        o.push("destroy of a NULL info survived".to_string());
        let mut p = png;
        (api.png_destroy_read_struct)(&mut p, null_mut(), null_mut());
        o.push(format!("png cleared={}", p.is_null()));
        o
    });

    /* ---- png_info_init_3: the size sweep reveals sizeof(png_info) ---- */
    // `sizeof(png_info)` is 352 bytes in this build (the `png_create_info_struct`
    // entry in the custom-allocator size log above proves that both libraries
    // agree on it), so sweep densely around that boundary as well.
    let probe: Vec<usize> = {
        let mut v: Vec<usize> = vec![0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];
        v.extend(330..=376);
        v.extend((0..20).map(|i| 1000 + i * 32));
        v.push(usize::MAX);
        v.push(usize::MAX / 2);
        v
    };
    same("png_info_init_3 size sweep", |api| unsafe {
        let mut o = Outcome::default();
        let (png, _) = new_read(api);
        for &n in &probe {
            let mut i = (api.png_create_info_struct)(png);
            assert!(!i.is_null());
            let before = i;
            (api.png_info_init_3)(&mut i, n);
            o.push(format!(
                "size={} null={} reallocated={}",
                n,
                i.is_null(),
                !i.is_null() && i != before
            ));
            if !i.is_null() {
                (api.png_destroy_info_struct)(png, &mut i);
            }
        }
        // a NULL *ptr_ptr is an immediate no-op
        let mut nul: *mut PngInfo = null_mut();
        (api.png_info_init_3)(&mut nul, 0);
        o.push(format!("NULL info still null={}", nul.is_null()));
        (api.png_info_init_3)(&mut nul, 1 << 20);
        o.push("NULL info big size survived".to_string());
        let mut p = png;
        (api.png_destroy_read_struct)(&mut p, null_mut(), null_mut());
        o
    });

    /* an info struct that has been through png_info_init_3 must still work for
     * a real read, whichever branch was taken */
    let lifecycle_file = multi_chunk_file(false);
    for &n in &[0usize, 8, 128, 351, 352, 353, 512, 4096, usize::MAX] {
        same(&format!("info_init_3({}) then read", n), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = lifecycle_file.clone();
            tls().in_pos = 0;
            let (png, first) = new_read(api);
            let mut info = first;
            (api.png_info_init_3)(&mut info, n);
            o.push(format!("null={} same={}", info.is_null(), info == first));
            if info.is_null() {
                let mut p = png;
                (api.png_destroy_read_struct)(&mut p, null_mut(), null_mut());
                return o;
            }
            (api.png_set_read_fn)(png, null_mut(), Some(read_cb));
            let g = guarded(api, png, &mut || {
                (api.png_read_info)(png, info);
                log_info(api, png, info, "after init_3");
                let h = (api.png_get_image_height)(png, info) as usize;
                let rb = (api.png_get_rowbytes)(png, info);
                let mut row = vec![0u8; rb];
                for _ in 0..h {
                    (api.png_read_row)(png, row.as_mut_ptr(), null_mut());
                    o.output.extend_from_slice(&row);
                }
                (api.png_read_end)(png, info);
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }

    /* png_create_png_struct when the allocator refuses the png_struct */
    for n in 0..3i64 {
        samef(&format!("create_png_struct budget {}", n), |api| unsafe {
            let sh = &libs().shim;
            BUDGET.with(|c| c.set(n));
            let p = (api.png_create_png_struct)(
                VER,
                null_mut(),
                Some(sh.error_fn),
                Some(warn_cb),
                mem_tag(),
                Some(budget_malloc),
                Some(budget_free),
            );
            log(format!("create_png_struct null={}", p.is_null()));
            if !p.is_null() {
                let i = (api.png_create_info_struct)(p);
                log(format!("info null={}", i.is_null()));
                let mut ip = i;
                (api.png_destroy_info_struct)(p, &mut ip);
                (api.png_destroy_png_struct)(p);
            }
            "done".to_string()
        });
    }

    /* a struct that owns a malloc'd jmp_buf must survive destruction */
    for &n in &[0usize, 100, 200, 201, 1000, 65536] {
        same(&format!("destroy with jmp_buf size {}", n), |api| unsafe {
            let mut o = Outcome::default();
            for write in [false, true] {
                let (png, info) = if write { new_write(api) } else { new_read(api) };
                let p = (api.png_set_longjmp_fn)(png, Some(dummy_longjmp), n);
                o.push(format!("write={} armed null={}", write, p.is_null()));
                if write {
                    destroy_write(api, png, info);
                } else {
                    destroy_read(api, png, info);
                }
                o.push("destroyed".to_string());
            }
            o
        });
    }

    // png_info_init_3(NULL, ..) dereferences its argument.
    for n in [0usize, 8, 1 << 20] {
        samef(&format!("png_info_init_3(NULL, {})", n), move |api| unsafe {
            log("calling".to_string());
            (api.png_info_init_3)(null_mut(), n);
            "returned".to_string()
        });
    }

    /* ---- png_destroy_read_struct / png_destroy_write_struct ---- */
    same("destroy_* with NULL out-params", |api| unsafe {
        let mut o = Outcome::default();
        (api.png_destroy_read_struct)(null_mut(), null_mut(), null_mut());
        (api.png_destroy_write_struct)(null_mut(), null_mut());
        o.push("all-NULL survived".to_string());

        let mut nul: *mut PngStruct = null_mut();
        let mut nuli: *mut PngInfo = null_mut();
        (api.png_destroy_read_struct)(&mut nul, &mut nuli, &mut nuli);
        (api.png_destroy_write_struct)(&mut nul, &mut nuli);
        o.push("NULL contents survived".to_string());

        // destroy the info structs but not the png_struct, then vice versa
        let (png, info) = new_read(api);
        let end = (api.png_create_info_struct)(png);
        let mut i = info;
        let mut e = end;
        (api.png_destroy_read_struct)(null_mut(), &mut i, &mut e);
        o.push(format!("info cleared={} end cleared={}", i.is_null(), e.is_null()));
        let mut p = png;
        (api.png_destroy_read_struct)(&mut p, null_mut(), null_mut());
        o.push(format!("png cleared={}", p.is_null()));
        // and again through the (now NULL) locals: a documented no-op
        (api.png_destroy_read_struct)(&mut p, &mut i, &mut e);
        o.push("second destroy survived".to_string());

        let (png, info) = new_write(api);
        let mut p = png;
        let mut i = info;
        (api.png_destroy_write_struct)(&mut p, &mut i);
        o.push(format!(
            "write cleared png={} info={}",
            p.is_null(),
            i.is_null()
        ));
        (api.png_destroy_write_struct)(&mut p, &mut i);
        o.push("second write destroy survived".to_string());

        // destroying a read struct with the write API and vice versa
        let (png, info) = new_read(api);
        let mut p = png;
        let mut i = info;
        (api.png_destroy_write_struct)(&mut p, &mut i);
        o.push("read struct destroyed by write API".to_string());
        let (png, info) = new_write(api);
        let mut p = png;
        let mut i = info;
        (api.png_destroy_read_struct)(&mut p, &mut i, null_mut());
        o.push("write struct destroyed by read API".to_string());
        o
    });

    // Destroying twice with the *saved* (already freed) pointers is a double
    // free: compare how each library dies.
    for write in [false, true] {
        samef(&format!("double destroy write={}", write), move |api| unsafe {
            let (png, info) = if write { new_write(api) } else { new_read(api) };
            let mut p = png;
            let mut i = info;
            if write {
                (api.png_destroy_write_struct)(&mut p, &mut i);
            } else {
                (api.png_destroy_read_struct)(&mut p, &mut i, null_mut());
            }
            log("first destroy done".to_string());
            let mut p2 = png;
            let mut i2 = info;
            if write {
                (api.png_destroy_write_struct)(&mut p2, &mut i2);
            } else {
                (api.png_destroy_read_struct)(&mut p2, &mut i2, null_mut());
            }
            "second destroy returned".to_string()
        });
    }

    /* ---- a full round trip through png_create_png_struct only ---- */
    same("create_png_struct then use it as a write struct", |api| unsafe {
        let mut o = Outcome::default();
        let sh = &libs().shim;
        let png = (api.png_create_png_struct)(
            VER,
            null_mut(),
            Some(sh.error_fn),
            Some(warn_cb),
            null_mut(),
            None,
            None,
        );
        o.push(format!("null={}", png.is_null()));
        let info = (api.png_create_info_struct)(png);
        (api.png_set_write_fn)(png, null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                3,
                2,
                8,
                PNG_COLOR_TYPE_GRAY,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        let mut i = info;
        (api.png_destroy_info_struct)(png, &mut i);
        (api.png_destroy_png_struct)(png);
        o
    });

    report("struct_lifecycle");
}

/* ================================================================== */
/* C-152 — png_build_grayscale_palette                                 */
/* ================================================================== */

/// C-152: every bit depth, valid and invalid, into a palette pre-filled with a
/// known pattern; all 768 bytes are compared.
#[test]
fn grayscale_palette() {
    let depths: [c_int; 18] = [
        1,
        2,
        4,
        8,
        0,
        3,
        5,
        6,
        7,
        9,
        10,
        15,
        16,
        17,
        32,
        255,
        -1,
        i32::MIN,
    ];
    for &bd in &depths {
        for seed in 0..3u64 {
            same(
                &format!("build_grayscale_palette bd={} seed={}", bd, seed),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let mut rng = Rng::new(0x9a11 ^ seed);
                    // 256 entries pre-filled with a known pattern
                    let mut pal = vec![png_color::default(); 256];
                    for (i, e) in pal.iter_mut().enumerate() {
                        e.red = rng.u8();
                        e.green = (i as u8) ^ 0x5a;
                        e.blue = rng.u8();
                    }
                    let before = pal.clone();
                    (api.png_build_grayscale_palette)(bd, pal.as_mut_ptr());
                    // all 768 bytes go into the compared output
                    for e in &pal {
                        o.output.push(e.red);
                        o.output.push(e.green);
                        o.output.push(e.blue);
                    }
                    let changed = pal
                        .iter()
                        .zip(before.iter())
                        .filter(|(a, b)| a != b)
                        .count();
                    o.push(format!("bd={} entries changed={}", bd, changed));
                    let first = pal
                        .iter()
                        .take(20)
                        .map(|e| format!("{:02x}{:02x}{:02x}", e.red, e.green, e.blue))
                        .collect::<Vec<_>>()
                        .join(" ");
                    o.push(format!("first entries: {}", first));
                    o
                },
            );
        }
    }

    same("build_grayscale_palette(NULL)", |api| unsafe {
        let mut o = Outcome::default();
        for &bd in &[1, 2, 4, 8, 0, 16, -1] {
            (api.png_build_grayscale_palette)(bd, null_mut());
            o.push(format!("bd={} NULL palette survived", bd));
        }
        o
    });

    report("grayscale_palette");
}
