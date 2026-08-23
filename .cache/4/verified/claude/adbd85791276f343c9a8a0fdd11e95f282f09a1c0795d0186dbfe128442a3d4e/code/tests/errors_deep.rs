//! Phase C+ — the *deep* error surface: the diagnostics that no ordinary use of
//! the public API can reach.
//!
//! `tests/errors.rs` drives libpng's error paths through the documented API.  A
//! residue of `png_error` / `png_warning` sites is left over that only fires
//!
//!   * from inside a libpng callback (`pngerror.c:593`),
//!   * from an internal entry point the public setters would have rejected first
//!     (`png_write_IHDR`, `png_write_iCCP`, `png_write_iTXt`, `png_write_sPLT`,
//!     `png_write_tIME`, `png_write_sCAL_s`, `png_read_start_row`), or
//!   * from a datastream shape that has to be assembled byte by byte (a zlib
//!     stream that ends before its chunk does, an IDAT with the zlib FDICT bit
//!     set, an sCAL with a good width and a broken height, ...).
//!
//! Every case below names the exact `file:line` in `c_src/src/` it is aiming at
//! and asserts that the C library and the Rust translation produce the same
//! thing.  `reached_the_intended_sites` at the bottom then proves, from
//! `observed_all()`, that the messages really were produced, so that a future
//! refactor of the translation cannot silently stop reaching them.
//!
//! ## Sites that are provably unreachable on this build
//!
//! Four of the targeted sites are dead code on a 64-bit `size_t`; they are
//! documented here (with the comparison and the constants) instead of tested,
//! because reaching them needs either a >2 GiB object or a 32-bit `size_t`:
//!
//! * `pngwutil.c:1589` "tEXt: text too long" —
//!   `if (text_len > PNG_UINT_31_MAX - (key_len+1)) png_error(...)`.
//!   `text_len = strlen(text)` and `key_len <= 79`, so the trigger is a NUL
//!   terminated string of more than `0x7fffffff - 80 == 2147483567` bytes.
//!
//! * `pngwutil.c:1736` "iTXt: uncompressed text too long" —
//!   `if (comp.input_len > PNG_UINT_31_MAX-prefix_len) png_error(...)`, where
//!   `comp.input_len = strlen(text)` and `prefix_len <= 81 + lang + lang_key`.
//!   Again a >2 GiB C string.
//!
//! * `pngrutil.c:4649` "Row has too many bytes to allocate in memory" —
//!   `if (png_ptr->rowbytes > (PNG_SIZE_MAX - 1)) png_error(...)`.
//!   `PNG_SIZE_MAX` is `((size_t)(-1))` = 2^64-1 here, while
//!   `rowbytes = PNG_ROWBYTES(pixel_depth, width)` is at most
//!   `(2^31-1) * (64/8) == 17179869176` — 30 bits short of the limit.
//!
//! * `png.c:2007` "Image width is too large for this architecture" — the test at
//!   png.c:1989 is
//!
//!   ```c
//!       if (((width + 7) & ~(png_alloc_size_t)7) >
//!           (((PNG_SIZE_MAX - 48 /* big_row_buf hack */
//!                           - 1) /* filter byte */
//!                           / 8) /* 8-byte RGBA pixels */
//!                           - 1))/* extra max_pixel_depth pad */
//!   ```
//!
//!   i.e. `> ((2^64-1 - 49) / 8) - 1 == 2305843009213693902`, while `width` is a
//!   `png_uint_32`, so the left-hand side is at most `4294967302 & ~7`.  Ten
//!   orders of magnitude short.  (The `width > PNG_UINT_31_MAX` warning at
//!   png.c:1977 does *not* short-circuit this one — it only sets `error = 1` —
//!   so the branch really is evaluated on every call and is simply never true.)
//!
//! Four more of the targeted sites are dead for a different reason: an earlier
//! test in the *same* function already rejects every input that could reach
//! them.  Each is documented on the test that pins down what happens instead:
//!
//! * `pngrutil.c:211` "bad header (invalid length)" — shadowed by the identical
//!   condition inside `png_get_uint_31`, 14 lines earlier; see
//!   `chunk_header_invalid_length`.
//! * `pngwutil.c:1148` "Profile length does not match profile" — shadowed by the
//!   literally identical `if` at pngwutil.c:1137; see `iccp_write_direct`.
//! * `pngrutil.c:1635` "sPLT chunk too long" — a 64-bit `PNG_SIZE_MAX`
//!   comparison against a value bounded by the chunk length; see
//!   `splt_chunk_cache`.
//! * `pngset.c:1832` "Compression buffer size limited to system maximum" — see
//!   `compression_buffer_size_system_maximum`.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};

/* ================================================================== */
/* small local helpers                                                 */
/* ================================================================== */

/// Run `f` against the C library with a fresh `Tls`; used to manufacture the
/// *valid* datastreams that the read-side cases then mutate.
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

fn base_png(ct: c_int, bd: c_int, il: c_int, w: u32, h: u32, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut img = Img::random(&mut rng, w, h, ct, bd);
    img.interlace = il;
    let v = with_c(|api| unsafe { write_plain(api, &img, &WriteOpts::default()).bytes });
    assert!(!v.is_empty(), "reference write produced nothing");
    v
}

/// Read `data` with both libraries and require the same outcome.
fn diff_read(case: &str, data: &[u8], setup: impl Fn(&Api, *mut PngStruct, *mut PngInfo) + Copy) {
    assert_same(case, |api| unsafe {
        let mut o = Outcome::default();
        let rr = read_image(api, data, &ReadOpts::default(), &mut |a, p, i| setup(a, p, i));
        o.push(format!("guard={:?}", rr.guard));
        for r in &rr.rows {
            o.output.extend_from_slice(r);
        }
        o
    });
}

fn noop(_: &Api, _: *mut PngStruct, _: *mut PngInfo) {}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A zlib stream carrying `data` in stored (uncompressed) deflate blocks, so
/// that every byte of it is under the test's control.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x78u8, 0x01];
    if data.is_empty() {
        v.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut i = 0;
        while i < data.len() {
            let n = (data.len() - i).min(65535);
            let last = i + n == data.len();
            v.push(if last { 1 } else { 0 });
            v.extend_from_slice(&(n as u16).to_le_bytes());
            v.extend_from_slice(&(!(n as u16)).to_le_bytes());
            v.extend_from_slice(&data[i..i + n]);
            i += n;
        }
    }
    v.extend_from_slice(&adler32(data).to_be_bytes());
    v
}

/// A minimal ICC profile that `png_icc_check_length` / `_header` / `_tag_table`
/// all accept for an RGB image (same shape as `tests/chunks.rs::Icc::new`).
fn icc_profile(total: usize) -> Vec<u8> {
    let ntags = 1u32;
    let mut v = vec![0u8; total.max(132 + 12 * ntags as usize + 8)];
    let n = v.len() as u32;
    v[0..4].copy_from_slice(&n.to_be_bytes());
    v[8] = 2; /* major version 2 */
    v[12..16].copy_from_slice(b"mntr");
    v[16..20].copy_from_slice(b"RGB ");
    v[20..24].copy_from_slice(b"XYZ ");
    v[36..40].copy_from_slice(b"acsp");
    v[64..68].copy_from_slice(&0u32.to_be_bytes()); /* intent */
    /* the D50 illuminant */
    v[68..80].copy_from_slice(&[
        0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
    ]);
    v[128..132].copy_from_slice(&ntags.to_be_bytes());
    v[132..136].copy_from_slice(b"rXYZ");
    v[136..140].copy_from_slice(&144u32.to_be_bytes());
    v[140..144].copy_from_slice(&8u32.to_be_bytes());
    v
}

/// A hand-built `w`x`h` 8-bit grey PNG whose IDAT payload is exactly `idat`.
fn handmade_gray(w: u32, h: u32, idat: &[u8]) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr));
    v.extend_from_slice(&chunk(b"IDAT", idat));
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

/* There is no `libc` crate available offline, so declare what we need. */
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    #[link_name = "malloc"]
    fn raw_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn raw_free(p: *mut c_void);
}

/* ================================================================== */
/* pngerror.c:593 / pngerror.c:600 -- png_set_longjmp_fn               */
/* ================================================================== */

/// `pngerror.c:600` "Application jmp_buf size changed".
///
/// The first call with `size <= sizeof png_ptr->jmp_buf_local` (200 here) takes
/// the *stack* allocation branch and records `jmp_buf_size = 0`; the second call
/// therefore lands in the `else` at pngerror.c:580 with `size == 0`, recovers
/// `size = 200`, finds `jmp_buf_ptr == &jmp_buf_local` (no error), and then
/// `size != jmp_buf_size` at pngerror.c:598 -> the warning and a NULL return.
///
/// The `size > 200` first call instead allocates, so `jmp_buf_size` is the real
/// size and a *matching* second call succeeds; that is recorded too.
#[test]
fn jmp_buf_size_changed() {
    let sizes: [usize; 8] = [0, 1, 8, 199, 200, 201, 300, 4096];
    for &first in &sizes {
        for &second in &sizes {
            for write in [false, true] {
                assert_same(
                    &format!("set_longjmp_fn {} then {} write={}", first, second, write),
                    |api| unsafe {
                        let mut o = Outcome::default();
                        let (png, info) = if write { new_write(api) } else { new_read(api) };
                        // NB: no `guarded` here -- png_set_longjmp_fn is what
                        // arms the trap, so arming one first would hide the
                        // state under test.  Neither call can be fatal for
                        // these inputs (see the doc comment).
                        let a = (api.png_set_longjmp_fn)(png, None, first);
                        let b = (api.png_set_longjmp_fn)(png, None, second);
                        o.push(format!(
                            "first_null={} second_null={} same={}",
                            a.is_null(),
                            b.is_null(),
                            !a.is_null() && a == b
                        ));
                        // and a third call with the original size
                        let c = (api.png_set_longjmp_fn)(png, None, first);
                        o.push(format!("third_null={} same_as_first={}", c.is_null(), c == a));
                        if write {
                            destroy_write(api, png, info)
                        } else {
                            destroy_read(api, png, info)
                        }
                        o
                    },
                );
            }
        }
    }
}

/// The warning callback used by `libpng_jmp_buf_still_allocated`: it calls
/// `png_set_longjmp_fn` from *inside* a libpng warning, which is the only place
/// an application can observe a `png_struct` whose `jmp_buf_ptr` is set and
/// whose `jmp_buf_size` is 0 (png.c:323-324 does exactly that while
/// `png_create_png_struct` runs).
unsafe extern "C" fn warn_then_set_longjmp(png: *mut PngStruct, msg: *const c_char) {
    let m = if msg.is_null() {
        "<null>".to_string()
    } else {
        std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned()
    };
    observe(&m);
    log(format!("warning: {}", m));
    let api = cur_api();
    let jb = (api.png_set_longjmp_fn)(png, None, 200);
    // Only reached if the library did *not* png_error out.
    log(format!("set_longjmp_fn from warn_fn -> null={}", jb.is_null()));
}

/// `pngerror.c:593` "Libpng jmp_buf still allocated".
///
/// Requires `jmp_buf_ptr != NULL && jmp_buf_size == 0 &&
/// jmp_buf_ptr != &png_ptr->jmp_buf_local`.  `png_free_jmpbuf` always clears
/// both fields together, so the only window is inside `png_create_png_struct`,
/// which sets
///
/// ```c
///     create_struct.jmp_buf_ptr = &create_jmp_buf;   /* png.c:323 */
///     create_struct.jmp_buf_size = 0; /*stack allocation*/
/// ```
///
/// and then calls `png_user_version_check`, which `png_warning`s on a version
/// mismatch (png.c:245).  Calling `png_set_longjmp_fn` from that warning handler
/// hits pngerror.c:586 with `&create_jmp_buf != &create_struct.jmp_buf_local`.
///
/// The trap is armed with a NULL `png_ptr` so that the shim's `setjmp` frame
/// exists without libpng having any part in it (see `harness_run`), which is
/// what lets the resulting `png_error` be caught: `png_create_read_struct`
/// itself never returns.
#[test]
fn libpng_jmp_buf_still_allocated() {
    for (tag, ver) in [
        ("mismatch major", "9.9.9\0"),
        ("mismatch minor", "1.9.59.git\0"),
        ("empty", "\0"),
        ("matching", "1.6.59.git\0"),
    ] {
        for write in [false, true] {
            let vp = ver.as_ptr() as *const c_char;
            assert_same(
                &format!("create with version {} write={}", tag, write),
                |api| unsafe {
                    let mut o = Outcome::default();
                    let sh = &libs().shim;
                    let mut png: *mut PngStruct = core::ptr::null_mut();
                    let g = guarded(api, core::ptr::null_mut(), &mut || {
                        png = if write {
                            (api.png_create_write_struct)(
                                vp,
                                core::ptr::null_mut(),
                                Some(sh.error_fn),
                                Some(warn_then_set_longjmp),
                            )
                        } else {
                            (api.png_create_read_struct)(
                                vp,
                                core::ptr::null_mut(),
                                Some(sh.error_fn),
                                Some(warn_then_set_longjmp),
                            )
                        };
                        log(format!("create -> null={}", png.is_null()));
                    });
                    o.push(format!("guard={:?}", g));
                    if !png.is_null() {
                        let info = (api.png_create_info_struct)(png);
                        if write {
                            destroy_write(api, png, info)
                        } else {
                            destroy_read(api, png, info)
                        }
                    }
                    o
                },
            );
        }
    }
}

/* ================================================================== */
/* pngread.c:444 -- "Invalid attempt to read row data"                 */
/* ================================================================== */

/// `pngread.c:443`:
///
/// ```c
///     if ((png_ptr->mode & PNG_HAVE_IDAT) == 0)
///        png_error(png_ptr, "Invalid attempt to read row data");
/// ```
///
/// `PNG_HAVE_IDAT` is only set by `png_read_info` (pngread.c:127) when it walks
/// onto the first IDAT, and `png_read_info` cannot return before then (it loops
/// until it sees one).  So the way in is to skip `png_read_info` altogether and
/// call `png_read_row` on a virgin read struct: `png_read_row` first runs
/// `png_read_start_row` (which is happy with width == height == 0: `row_bytes`
/// comes out as `PNG_ROWBYTES(0,0) + 1 + 0 == 1`) and then hits the test above.
///
/// The variants also cover the case where `png_set_IHDR` has been used to fill
/// in `info_ptr` (which does *not* touch `png_ptr->mode`) and the case where the
/// row pointers are NULL.
#[test]
fn read_row_without_idat() {
    for which in 0..6usize {
        assert_same_forked(&format!("read_row before read_info #{}", which), |api| {
            unsafe {
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let mut row = vec![0u8; 64];
                let mut disp = vec![0u8; 64];
                let g = guarded(api, png, &mut || {
                    if which >= 2 {
                        // fill in info_ptr without ever reading a header
                        (api.png_set_IHDR)(
                            png,
                            info,
                            4,
                            2,
                            8,
                            PNG_COLOR_TYPE_GRAY,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                    }
                    if which >= 4 {
                        // and initialise the row machinery explicitly first
                        (api.png_read_start_row)(png);
                    }
                    match which % 2 {
                        0 => (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut()),
                        _ => (api.png_read_row)(png, row.as_mut_ptr(), disp.as_mut_ptr()),
                    }
                });
                format!("{:?}", g)
            }
        });
    }
    // The same through png_read_rows / png_read_image, which funnel into
    // png_read_row.
    for n in [1u32, 2, 5] {
        assert_same_forked(&format!("read_rows before read_info n={}", n), |api| unsafe {
            let (png, _info) = new_read(api);
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            let rows: Vec<Vec<u8>> = (0..n).map(|_| vec![0u8; 64]).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter().map(|r| r.as_ptr() as *mut u8).collect();
            let g = guarded(api, png, &mut || {
                (api.png_read_rows)(png, ptrs.as_mut_ptr(), core::ptr::null_mut(), n);
            });
            format!("{:?}", g)
        });
    }
}

/* ================================================================== */
/* pngread.c:729 / 747 -- ".Too many IDATs found" / "..Too many ..."   */
/* ================================================================== */

/// `pngread.c:729` and `pngread.c:747`.  The leading dots are *not*
/// `PNG_STRING_NEWLINE` artefacts: they are literally in the C source, put there
/// to tell the two `png_read_end` sites apart:
///
/// ```c
///     png_benign_error(png_ptr, ".Too many IDATs found");    /* :729, the
///                                    "handle IDAT as unknown" branch */
///     png_benign_error(png_ptr, "..Too many IDATs found");   /* :747, the
///                                    ordinary branch */
/// ```
///
/// Both need `png_read_end` to meet an IDAT with either a non-zero length while
/// `PNG_FLAG_ZSTREAM_ENDED` is clear, or `PNG_HAVE_CHUNK_AFTER_IDAT` set.  The
/// recipe is therefore: a normal file with a *second* run of IDATs placed after
/// another chunk, read to the end without consuming all the rows (so the zstream
/// has not ended).  `:729` additionally needs
/// `png_chunk_unknown_handling(png_IDAT) != 0`, i.e.
/// `png_set_keep_unknown_chunks(PNG_HANDLE_CHUNK_ALWAYS)`.
#[test]
fn too_many_idats_in_read_end() {
    let good = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 8, 6, 0x1da7_0001);
    let chunks = split_chunks(&good);
    let idat = chunks.iter().find(|(n, _)| n == "IDAT").expect("IDAT").1.clone();
    let payload = good[idat.start + 8..idat.end - 4].to_vec();

    // (a) file, then a tEXt (sets PNG_HAVE_CHUNK_AFTER_IDAT), then more IDAT.
    let mut with_extra = good[..idat.end].to_vec();
    with_extra.extend_from_slice(&chunk(b"tEXt", b"Key\0after"));
    with_extra.extend_from_slice(&chunk(b"IDAT", &payload));
    with_extra.extend_from_slice(&chunk(b"IDAT", &[]));
    with_extra.extend_from_slice(&good[idat.end..]);

    // (b) file, then a non-empty IDAT straight away.
    let mut doubled = good[..idat.end].to_vec();
    doubled.extend_from_slice(&chunk(b"IDAT", &payload));
    doubled.extend_from_slice(&good[idat.end..]);

    // (c) file, then an empty IDAT after another chunk.
    let mut empty_after = good[..idat.end].to_vec();
    empty_after.extend_from_slice(&chunk(b"pHYs", &[0, 0, 0, 1, 0, 0, 0, 1, 1]));
    empty_after.extend_from_slice(&chunk(b"IDAT", &[]));
    empty_after.extend_from_slice(&good[idat.end..]);

    for (tag, data) in [
        ("tEXt then IDAT", &with_extra),
        ("IDAT twice", &doubled),
        ("pHYs then empty IDAT", &empty_after),
    ] {
        // keep == None: the ordinary branch (pngread.c:747)
        // keep == ALWAYS: the unknown-handling branch (pngread.c:729)
        for keep in [None, Some(PNG_HANDLE_CHUNK_ALWAYS), Some(PNG_HANDLE_CHUNK_NEVER)] {
            for rows in [RowMode::None, RowMode::Row] {
                for benign in [false, true] {
                    let case = format!(
                        "read_end {} keep={:?} rows={:?} benign={}",
                        tag, keep, rows, benign
                    );
                    assert_same(&case, |api| unsafe {
                        let mut o = Outcome::default();
                        let opts = ReadOpts {
                            rows,
                            update_info: rows != RowMode::None,
                            ..ReadOpts::default()
                        };
                        let rr = read_image(api, data, &opts, &mut |a, p, _i| {
                            if benign {
                                (a.png_set_benign_errors)(p, 1);
                            }
                            if let Some(k) = keep {
                                (a.png_set_keep_unknown_chunks)(p, k, core::ptr::null(), 0);
                            }
                        });
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
/* pngrtran.c:120 and pngtrans.c:845 -- setters after the row init     */
/* ================================================================== */

/// `pngrtran.c:119`:
///
/// ```c
///     if ((png_ptr->flags & PNG_FLAG_ROW_INIT) != 0)
///        png_app_error(png_ptr,
///            "invalid after png_start_read_image or png_read_update_info");
/// ```
///
/// (`png_rtran_ok`, the gate in front of every read transform) and the twin
/// check in `png_set_user_transform_info` at `pngtrans.c:845`:
///
/// ```c
///     if ((png_ptr->mode & PNG_IS_READ_STRUCT) != 0 &&
///        (png_ptr->flags & PNG_FLAG_ROW_INIT) != 0)
///        png_app_error(png_ptr,
///            "info change after png_start_read_image or png_read_update_info");
/// ```
///
/// `PNG_FLAG_ROW_INIT` is set at the very end of `png_read_start_row`
/// (pngrutil.c:4682), so any `png_set_*` transform issued after
/// `png_read_update_info` trips it.  `png_app_error` is fatal by default and a
/// warning after `png_set_benign_errors(png, 1)`; both are exercised.
#[test]
fn transform_after_update_info() {
    let good = base_png(PNG_COLOR_TYPE_RGB_ALPHA, 8, PNG_INTERLACE_NONE, 6, 4, 0x2711);
    let bkgd = png_color_16 { index: 0, red: 1, green: 2, blue: 3, gray: 4 };
    for which in 0..18usize {
        for benign in [false, true] {
            let case = format!("set after update_info #{} benign={}", which, benign);
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                tls().input = good.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let g = guarded(api, png, &mut || {
                    if benign {
                        (api.png_set_benign_errors)(png, 1);
                    }
                    (api.png_read_info)(png, info);
                    (api.png_read_update_info)(png, info);
                    log("--- now installing a transform ---".to_string());
                    match which {
                        0 => (api.png_set_expand)(png),
                        1 => (api.png_set_gray_to_rgb)(png),
                        2 => (api.png_set_packing)(png),
                        3 => (api.png_set_bgr)(png),
                        4 => (api.png_set_swap)(png),
                        5 => (api.png_set_invert_alpha)(png),
                        6 => (api.png_set_strip_alpha)(png),
                        7 => (api.png_set_strip_16)(png),
                        8 => (api.png_set_scale_16)(png),
                        9 => (api.png_set_packswap)(png),
                        10 => (api.png_set_invert_mono)(png),
                        11 => (api.png_set_expand_16)(png),
                        12 => (api.png_set_gamma_fixed)(png, 220000, 45455),
                        13 => (api.png_set_background_fixed)(
                            png,
                            &bkgd,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0,
                            0,
                        ),
                        14 => (api.png_set_rgb_to_gray_fixed)(png, PNG_ERROR_ACTION_NONE, -1, -1),
                        15 => (api.png_set_alpha_mode_fixed)(png, PNG_ALPHA_STANDARD, 100000),
                        16 => (api.png_set_add_alpha)(png, 255, PNG_FILLER_AFTER),
                        // pngtrans.c:845
                        _ => (api.png_set_user_transform_info)(png, core::ptr::null_mut(), 8, 4),
                    }
                    log("--- transform installed ---".to_string());
                    log(format!("rowbytes={}", (api.png_get_rowbytes)(png, info)));
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            });
        }
    }
    // png_set_<chunk> after png_read_update_info: pngtrans.c:845 is the only
    // "info change" gate, so also prove the chunk setters are *not* gated (their
    // outcome is recorded either way).
    for benign in [false, true] {
        assert_same(
            &format!("set_user_transform_info twice benign={}", benign),
            |api| unsafe {
                let mut o = Outcome::default();
                tls().input = good.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let g = guarded(api, png, &mut || {
                    if benign {
                        (api.png_set_benign_errors)(png, 1);
                    }
                    (api.png_read_info)(png, info);
                    // before the row init: accepted
                    (api.png_set_user_transform_info)(png, core::ptr::null_mut(), 8, 4);
                    (api.png_read_update_info)(png, info);
                    // after: rejected
                    (api.png_set_user_transform_info)(png, core::ptr::null_mut(), 16, 2);
                    log(format!("rowbytes={}", (api.png_get_rowbytes)(png, info)));
                });
                o.push(format!("guard={:?}", g));
                destroy_read(api, png, info);
                o
            },
        );
    }
    // ... and on a *write* struct, where pngtrans.c:842's PNG_IS_READ_STRUCT
    // test means the same call is always allowed.
    assert_same("set_user_transform_info on write struct", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let row = [0u8; 8];
        let g = guarded(api, png, &mut || {
            (api.png_set_IHDR)(
                png,
                info,
                2,
                2,
                8,
                PNG_COLOR_TYPE_GRAY,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_write_info)(png, info);
            (api.png_write_row)(png, row.as_ptr() as *mut u8);
            (api.png_set_user_transform_info)(png, core::ptr::null_mut(), 8, 1);
            (api.png_write_row)(png, row.as_ptr() as *mut u8);
            (api.png_write_end)(png, info);
        });
        o.push(format!("guard={:?}", g));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o
    });
}

/* ================================================================== */
/* pngrtran.c:1886 -- "invalid background gamma type"                  */
/* ================================================================== */

/// `pngrtran.c:1866-1887`, inside `png_init_read_transformations`:
///
/// ```c
///     switch (png_ptr->background_gamma_type)
///     {
///        case PNG_BACKGROUND_GAMMA_SCREEN: ...
///        case PNG_BACKGROUND_GAMMA_FILE:   ...
///        case PNG_BACKGROUND_GAMMA_UNIQUE: ...
///        default:
///           png_error(png_ptr, "invalid background gamma type");
///     }
/// ```
///
/// `png_set_background_fixed` stores `background_gamma_code` in a `png_byte`
/// without range-checking it (pngrtran.c:163) — it only rejects
/// `PNG_BACKGROUND_GAMMA_UNKNOWN` (== 0) with the warning "Application must
/// supply a known background gamma" at pngrtran.c:153.  So any code in 4..255
/// gets through.
///
/// Three further conditions have to hold to get to the `switch`:
///  * `PNG_COMPOSE` set — `png_set_background_fixed` does that,
///  * the colour type is not `PNG_COLOR_TYPE_PALETTE` (pngrtran.c:1860),
///  * the gamma tables get built (pngrtran.c:1671-1684), which for the COMPOSE
///    case needs a significant file *or* screen gamma — hence the
///    `png_set_gamma_fixed` call.
#[test]
fn invalid_background_gamma_type() {
    let bkgd = png_color_16 { index: 1, red: 0x20, green: 0x30, blue: 0x40, gray: 0x28 };
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ] {
        let good = base_png(ct, bd, PNG_INTERLACE_NONE, 5, 3, 0x1886 ^ ct as u64);
        for code in [-1i32, 0, 1, 2, 3, 4, 5, 17, 99, 255, 256] {
            for expand in [false, true] {
                for gamma in [false, true] {
                    let case = format!(
                        "background gamma code={} ct={} bd={} expand={} gamma={}",
                        code, ct, bd, expand, gamma
                    );
                    diff_read(&case, &good, move |api, png, _i| unsafe {
                        if gamma {
                            (api.png_set_gamma_fixed)(png, 220000, 45455);
                        }
                        if expand {
                            (api.png_set_expand)(png);
                        }
                        (api.png_set_background_fixed)(png, &bkgd, code, 0, 100000);
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* pngrtran.c:2104 -- "Palette is NULL in indexed image"               */
/* ================================================================== */

/// `pngrtran.c:2103`, in `png_read_transform_info`:
///
/// ```c
///     if (info_ptr->color_type == PNG_COLOR_TYPE_PALETTE) {
///        ...
///        if (png_ptr->palette == NULL)
///           png_error (png_ptr, "Palette is NULL in indexed image");
///     }
/// ```
///
/// A real indexed datastream can never get here: `png_read_info` fails with
/// "Missing PLTE before IDAT" (pngread.c:122) when the PLTE is absent, and a
/// present-but-empty PLTE is rejected by `png_set_PLTE` ("Invalid palette",
/// pngset.c:784) unless `PNG_FLAG_MNG_EMPTY_PLTE` is permitted — and even then
/// `png_set_PLTE` `png_calloc`s a full 256-entry `png_ptr->palette`, so the
/// pointer is never NULL.
///
/// The condition is on `info_ptr->color_type` but the palette is read from
/// `png_ptr`, and `png_set_IHDR` writes only `info_ptr`.  So: a *read* struct
/// whose `info_ptr` is told it is a palette image by hand, `png_set_expand` (to
/// turn on `PNG_EXPAND`), then `png_read_update_info`, whose
/// `png_read_start_row` is content with the untouched `png_ptr` (width 0,
/// `pixel_depth` 0) and whose `png_read_transform_info` then sees
/// `info_ptr->color_type == PALETTE` with `png_ptr->palette == NULL`.
#[test]
fn palette_is_null_in_indexed_image() {
    for ct in [
        PNG_COLOR_TYPE_PALETTE,
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for bd in [1i32, 4, 8] {
            for expand in [false, true] {
                for set_plte in [false, true] {
                    let case = format!(
                        "info-only IHDR ct={} bd={} expand={} plte={}",
                        ct, bd, expand, set_plte
                    );
                    assert_same_forked(&case, move |api| unsafe {
                        let (png, info) = new_read(api);
                        let pal = vec![png_color { red: 9, green: 8, blue: 7 }; 4];
                        let g = guarded(api, png, &mut || {
                            (api.png_set_IHDR)(
                                png,
                                info,
                                4,
                                2,
                                bd,
                                ct,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            if set_plte {
                                (api.png_set_PLTE)(png, info, pal.as_ptr(), 4);
                            }
                            if expand {
                                (api.png_set_expand)(png);
                            }
                            (api.png_read_update_info)(png, info);
                            log(format!(
                                "rowbytes={} channels={}",
                                (api.png_get_rowbytes)(png, info),
                                (api.png_get_channels)(png, info)
                            ));
                        });
                        format!("{:?}", g)
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* pngrutil.c:211 -- "bad header (invalid length)" is DEAD CODE        */
/* ================================================================== */

/// `pngrutil.c:210` cannot be reached on any build of this source:
///
/// ```c
///     png_read_data(png_ptr, buf, 8);
///     length = png_get_uint_31(png_ptr, buf);         /* :197 */
///     ...
///     if (buf[0] >= 0x80U)
///        png_chunk_error(png_ptr, "bad header (invalid length)");   /* :211 */
/// ```
///
/// and `png_get_uint_31` (pngrutil.c:40-49) is
///
/// ```c
///     png_uint_32 uval = png_get_uint_32(buf);
///     if (uval > PNG_UINT_31_MAX) png_error(png_ptr, "PNG unsigned integer out of range");
/// ```
///
/// `buf[0] >= 0x80` is *exactly* `png_get_uint_32(buf) >= 0x80000000`, which is
/// `> PNG_UINT_31_MAX == 0x7fffffff`; the `png_error` at :46 therefore always
/// fires first, 14 lines earlier.  (`png_get_uint_31` is a real call here, not a
/// macro — this build defines neither `PNG_USE_READ_MACROS` nor any override,
/// see `grep -rn png_get_uint_31 c_src/`.)
///
/// This test pins the *observable* behaviour down instead: every chunk-header
/// length with the top bit set must produce "PNG unsigned integer out of range"
/// from both libraries, never "bad header (invalid length)".
#[test]
fn chunk_header_invalid_length() {
    let good = base_png(PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, 4, 3, 0x211);
    let chunks = split_chunks(&good);
    for (name, range) in &chunks {
        for len in [0x8000_0000u32, 0x8000_0001, 0xc000_0000, 0xffff_ffff, 0x7fff_ffff] {
            let mut v = good.clone();
            v[range.start..range.start + 4].copy_from_slice(&len.to_be_bytes());
            diff_read(&format!("{} header length {:#010x}", name, len), &v, noop);
        }
    }
    // and png_read_chunk_header called directly, with the input under our
    // control byte for byte
    for hdr in [
        [0x80u8, 0, 0, 0, b'g', b'A', b'M', b'A'],
        [0xff, 0xff, 0xff, 0xff, b'I', b'D', b'A', b'T'],
        [0x7f, 0xff, 0xff, 0xff, b'I', b'E', b'N', b'D'],
        [0x00, 0x00, 0x00, 0x04, b'1', b'2', b'3', b'4'],
        [0x00, 0x00, 0x00, 0x04, b'g', b'A', b'M', b'A'],
    ] {
        assert_same(&format!("read_chunk_header {:02x?}", hdr), |api| unsafe {
            let mut o = Outcome::default();
            tls().input = hdr.to_vec();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
            let g = guarded(api, png, &mut || {
                let n = (api.png_read_chunk_header)(png);
                log(format!("length={:#x}", n));
            });
            o.push(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
}

/* ================================================================== */
/* pngrutil.c:789 / pngrutil.c:1455 -- "extra compressed data"         */
/* ================================================================== */

/// `pngrutil.c:787-789` (`png_decompress_chunk`, used by zTXt and iTXt):
///
/// ```c
///     if (ret == Z_STREAM_END && chunklength - prefix_size != lzsize)
///        png_chunk_benign_error(png_ptr, "extra compressed data");
/// ```
///
/// `lzsize` comes back from `png_inflate` as the number of bytes the zlib layer
/// actually consumed, so any trailing byte after the end of the deflate stream
/// (including its ADLER32) trips it.
///
/// `pngrutil.c:1443-1456` (`png_handle_iCCP`) is the same condition expressed
/// against the *unread* chunk remainder:
///
/// ```c
///     if (length > 0 && !(png_ptr->flags & PNG_FLAG_BENIGN_ERRORS_WARN))
///        errmsg = "extra compressed data";
///     else if (size == 0) {
///        if (length > 0)
///           png_chunk_warning(png_ptr, "extra compressed data");   /* :1455 */
/// ```
///
/// The iCCP reader pulls the chunk in `PNG_INFLATE_BUF_SIZE` (== 1024, see
/// `c_src/include/pnglibconf.h:210`) blocks on demand, so the padding after the
/// profile has to be big enough that a whole block is still unread when the
/// profile is complete — 2000 bytes here.  Line :1455 additionally needs
/// `PNG_FLAG_BENIGN_ERRORS_WARN`, i.e. `png_set_benign_errors(png, 1)`.
#[test]
fn extra_compressed_data() {
    let gray = base_png(PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, 4, 3, 0x789);
    let rgb = base_png(PNG_COLOR_TYPE_RGB, 8, PNG_INTERLACE_NONE, 4, 3, 0x790);

    /* ---- zTXt / iTXt: pngrutil.c:789 ---- */
    for trail in [1usize, 2, 5, 64, 1200] {
        for benign in [false, true] {
            // zTXt = keyword \0 compression_method zlib_stream
            let mut d = b"Comment\0\0".to_vec();
            d.extend_from_slice(&zlib_stored(b"some decompressed text"));
            d.extend_from_slice(&vec![0x5au8; trail]);
            let c = chunk(b"zTXt", &d);
            let v = insert_before(&gray, "IDAT", &c);
            diff_read(
                &format!("zTXt trailing {} benign={}", trail, benign),
                &v,
                move |api, png, _| unsafe {
                    if benign {
                        (api.png_set_benign_errors)(png, 1);
                    }
                },
            );
            // iTXt = keyword \0 compression_flag compression_method lang \0
            //        translated \0 zlib_stream
            let mut d = b"Comment\0\x01\x00en\0Kommentar\0".to_vec();
            d.extend_from_slice(&zlib_stored(b"some decompressed text"));
            d.extend_from_slice(&vec![0x5au8; trail]);
            let c = chunk(b"iTXt", &d);
            let v = insert_before(&gray, "IDAT", &c);
            diff_read(
                &format!("iTXt trailing {} benign={}", trail, benign),
                &v,
                move |api, png, _| unsafe {
                    if benign {
                        (api.png_set_benign_errors)(png, 1);
                    }
                },
            );
        }
    }
    // and with no trailing data at all, as the control
    let mut d = b"Comment\0\0".to_vec();
    d.extend_from_slice(&zlib_stored(b"some decompressed text"));
    let v = insert_before(&gray, "IDAT", &chunk(b"zTXt", &d));
    diff_read("zTXt exact length", &v, noop);

    /* ---- iCCP: pngrutil.c:1455 ---- */
    let prof = icc_profile(260);
    for trail in [0usize, 1, 8, 300, 1000, 1024, 1025, 2000, 3000] {
        for benign in [false, true] {
            let mut d = b"k\0\0".to_vec();
            d.extend_from_slice(&zlib_stored(&prof));
            d.extend_from_slice(&vec![0xa5u8; trail]);
            let c = chunk(b"iCCP", &d);
            let v = insert_before(&rgb, "IDAT", &c);
            diff_read(
                &format!("iCCP trailing {} benign={}", trail, benign),
                &v,
                move |api, png, _| unsafe {
                    if benign {
                        (api.png_set_benign_errors)(png, 1);
                    }
                },
            );
        }
    }
}

/* ================================================================== */
/* pngrutil.c:1577 -- "No space in chunk cache for sPLT"               */
/* pngrutil.c:1635 -- "sPLT chunk too long" is DEAD CODE               */
/* ================================================================== */

/// `pngrutil.c:1566-1581`:
///
/// ```c
///     if (png_ptr->user_chunk_cache_max != 0) {
///        if (png_ptr->user_chunk_cache_max == 1) { png_crc_finish(...); return handled_error; }
///        if (--png_ptr->user_chunk_cache_max == 1) {
///           png_warning(png_ptr, "No space in chunk cache for sPLT");
/// ```
///
/// so the warning needs `user_chunk_cache_max == 2` exactly when the sPLT is
/// met.  Every cached chunk decrements the counter, so the sweep below varies
/// both the limit and how many sPLTs precede the one under test.
///
/// `pngrutil.c:1633-1636` in the same function is unreachable here:
///
/// ```c
///     dl     = data_length / entry_size;                       /* entry_size >= 6 */
///     max_dl = PNG_SIZE_MAX / (sizeof (png_sPLT_entry));       /* == (2^64-1)/10 */
///     if (dl > max_dl) png_warning(png_ptr, "sPLT chunk too long");
/// ```
///
/// `data_length` is bounded by the chunk length, i.e. by `PNG_UINT_31_MAX`, so
/// `dl <= 2147483647/6 == 357913941`, while `max_dl == 1844674407370955161`.
#[test]
fn splt_chunk_cache() {
    let gray = base_png(PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, 4, 3, 0x1577);
    // sPLT payload: name \0 depth then nentries * 6 bytes (depth 8)
    let splt = |n: usize, depth: u8| -> Vec<u8> {
        let mut d = format!("p{}", n).into_bytes();
        d.push(0);
        d.push(depth);
        let per = if depth == 8 { 6 } else { 10 };
        for i in 0..3usize {
            for k in 0..per {
                d.push((i * per + k) as u8);
            }
        }
        chunk(b"sPLT", &d)
    };
    for nsplt in 1..=4usize {
        let mut v = gray.clone();
        for i in 0..nsplt {
            v = insert_before(&v, "IDAT", &splt(i, 8));
        }
        for cache in [0u32, 1, 2, 3, 4, 5, 6, 1000] {
            // NB: the limit has to be installed *before* png_read_info, because
            // the sPLT chunks sit before IDAT and are therefore handled by
            // png_read_info itself.  `read_image`'s `setup` hook runs after it,
            // which is why this case cannot use `diff_read`.
            assert_same(
                &format!("sPLT x{} chunk_cache_max={}", nsplt, cache),
                |api| unsafe {
                    let mut o = Outcome::default();
                    tls().input = v.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                    (api.png_set_chunk_cache_max)(png, cache);
                    let g = guarded(api, png, &mut || {
                        log(format!("cache={}", (api.png_get_chunk_cache_max)(png)));
                        (api.png_read_info)(png, info);
                        let mut sp: *mut png_sPLT_t = core::ptr::null_mut();
                        log(format!(
                            "sPLT n={} cache_left={}",
                            (api.png_get_sPLT)(png, info, &mut sp),
                            (api.png_get_chunk_cache_max)(png)
                        ));
                        (api.png_read_update_info)(png, info);
                        let rb = (api.png_get_rowbytes)(png, info);
                        let h = (api.png_get_image_height)(png, info) as usize;
                        let mut row = vec![0u8; rb];
                        for _ in 0..h {
                            (api.png_read_row)(png, row.as_mut_ptr(), core::ptr::null_mut());
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
    // A very long sPLT (the "too long" arithmetic above), plus the neighbouring
    // "sPLT chunk has bad length" / "malformed sPLT chunk" rejections.
    for (tag, data) in [
        ("huge but valid", {
            let mut d = b"big\0\x08".to_vec();
            d.extend_from_slice(&vec![0x11u8; 6 * 2000]);
            d
        }),
        ("bad length", {
            let mut d = b"big\0\x08".to_vec();
            d.extend_from_slice(&vec![0x11u8; 6 * 2000 + 1]);
            d
        }),
        ("depth 16 huge", {
            let mut d = b"big\0\x10".to_vec();
            d.extend_from_slice(&vec![0x11u8; 10 * 1200]);
            d
        }),
    ] {
        let v = insert_before(&gray, "IDAT", &chunk(b"sPLT", &data));
        diff_read(&format!("sPLT {}", tag), &v, noop);
    }
}

/* ================================================================== */
/* pngrutil.c:2316 / 2319 -- sCAL height                               */
/* ================================================================== */

/// `pngrutil.c:2311-2319`:
///
/// ```c
///     size_t heighti = i;
///     state = 0;
///     if (png_check_fp_number((png_const_charp)buffer, length, &state, &i) == 0 ||
///         i != length)
///        png_chunk_benign_error(png_ptr, "bad height format");        /* :2316 */
///     else if (PNG_FP_IS_POSITIVE(state) == 0)
///        png_chunk_benign_error(png_ptr, "non-positive height");      /* :2319 */
/// ```
///
/// which is only reached once the *width* has parsed cleanly and is positive
/// (pngrutil.c:2302-2307).  `tests/errors.rs::ancillary_chunk_semantics` only
/// ever varies both numbers together, so it always stops at the width; the cases
/// below pair a good width ("1.5") with every kind of broken height.
#[test]
fn scal_height_rejections() {
    let gray = base_png(PNG_COLOR_TYPE_GRAY, 8, PNG_INTERLACE_NONE, 4, 3, 0x2316);
    let widths = ["1.5", "1e2", "+3"];
    let heights: [&str; 16] = [
        "2.5",     // ok
        "",        // bad height format (nothing there)
        "x",       // bad height format
        "e",       // bad height format
        "1e",      // bad height format (exponent with no digits)
        ".",       // bad height format
        "+",       // bad height format
        "-",       // bad height format
        "1.5x",    // bad height format (trailing junk)
        "1.5\0",   // bad height format (i != length)
        "1 5",     // bad height format
        "0",       // non-positive height
        "0.0",     // non-positive height
        "-1",      // non-positive height
        "-0.5e1",  // non-positive height
        "0e10",    // non-positive height
    ];
    for unit in [1u8, 2] {
        for w in widths {
            for h in heights {
                let mut d = vec![unit];
                d.extend_from_slice(w.as_bytes());
                d.push(0);
                d.extend_from_slice(h.as_bytes());
                let c = chunk(b"sCAL", &d);
                for benign in [false, true] {
                    let v = insert_before(&gray, "IDAT", &c);
                    diff_read(
                        &format!("sCAL u={} w={:?} h={:?} benign={}", unit, w, h, benign),
                        &v,
                        move |api, png, _| unsafe {
                            if benign {
                                (api.png_set_benign_errors)(png, 1);
                            }
                        },
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/* pngrutil.c:3478 -- "invalid user transform pixel depth"             */
/* ================================================================== */

/// `pngrutil.c:3476-3478`, in `png_combine_row`:
///
/// ```c
///     else /* pixel_depth >= 8 */
///     {
///        /* Validate the depth - it must be a multiple of 8 */
///        if (pixel_depth & 7)
///           png_error(png_ptr, "invalid user transform pixel depth");
/// ```
///
/// `pixel_depth` is `png_ptr->transformed_pixel_depth`, which pngread.c:487 sets
/// from `row_info.pixel_depth` after the transforms have run, and
/// `png_do_read_transformations` computes that as
/// `user_transform_depth * user_transform_channels` (pngrtran.c:5165-5172).  So
/// `png_set_user_transform_info(png, NULL, 3, 3)` makes it 9 — at least 8 but
/// not a multiple of 8.
///
/// `png_combine_row` is only entered on the interlaced path (pngread.c:497-508),
/// so the image has to be Adam7 *and* `png_set_interlace_handling` must have
/// been called; `pass < 6` holds for the first six passes.
///
/// Two neighbouring guards have to be satisfied on the way:
///  * pngread.c:488 "sequential row overflow" — `png_read_start_row` folds the
///    user pixel depth into `maximum_pixel_depth` (pngrutil.c:4572-4578), so 9
///    is not an overflow;
///  * pngrutil.c:3249 "internal row size calculation error" —
///    `png_read_transform_info` stores the same 9-bit `info_rowbytes`
///    (pngrtran.c:2256-2277), so the two agree.
#[test]
fn invalid_user_transform_pixel_depth() {
    for il in [PNG_INTERLACE_ADAM7, PNG_INTERLACE_NONE] {
        for (ct, bd) in [(PNG_COLOR_TYPE_GRAY, 8), (PNG_COLOR_TYPE_RGB, 8)] {
            let good = base_png(ct, bd, il, 9, 7, 0x3478 ^ ct as u64);
            for (depth, channels) in [
                (3, 3),   // 9  -> invalid
                (5, 5),   // 25 -> invalid
                (1, 9),   // 9  -> invalid
                (8, 1),   // 8  -> fine
                (8, 3),   // 24 -> fine
                (16, 2),  // 32 -> fine
                (4, 1),   // 4  -> below 8, takes the bit-mask path
                (2, 3),   // 6  -> below 8
            ] {
                for interlace_handling in [false, true] {
                    let case = format!(
                        "user pixel depth {}x{} il={} ct={} handling={}",
                        depth, channels, il, ct, interlace_handling
                    );
                    assert_same_forked(&case, |api| unsafe {
                        tls().input = good.clone();
                        tls().in_pos = 0;
                        let (png, info) = new_read(api);
                        (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                        let g = guarded(api, png, &mut || {
                            (api.png_read_info)(png, info);
                            (api.png_set_read_user_transform_fn)(png, Some(user_transform_cb));
                            (api.png_set_user_transform_info)(
                                png,
                                core::ptr::null_mut(),
                                depth,
                                channels,
                            );
                            let passes = if interlace_handling {
                                (api.png_set_interlace_handling)(png)
                            } else {
                                1
                            };
                            (api.png_read_update_info)(png, info);
                            let rb = (api.png_get_rowbytes)(png, info);
                            let h = (api.png_get_image_height)(png, info) as usize;
                            log(format!("passes={} rowbytes={}", passes, rb));
                            // Generous rows: the transformed depth may be larger
                            // than what png_get_rowbytes reports for a lying
                            // user transform, and libpng writes what it computes.
                            let mut rows: Vec<Vec<u8>> =
                                (0..h).map(|_| vec![0u8; rb * 4 + 64]).collect();
                            for _ in 0..passes {
                                for r in rows.iter_mut() {
                                    (api.png_read_row)(
                                        png,
                                        r.as_mut_ptr(),
                                        core::ptr::null_mut(),
                                    );
                                }
                            }
                            (api.png_read_end)(png, info);
                        });
                        format!("{:?}", g)
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* pngset.c:1407 / pngset.c:1540 -- unknown chunk locations            */
/* ================================================================== */

/// `pngset.c:1385-1407` (`check_location`):
///
/// ```c
///     location &= (PNG_HAVE_IHDR|PNG_HAVE_PLTE|PNG_AFTER_IDAT);   /* 0x01|0x02|0x08 */
///     if (location == 0 && (png_ptr->mode & PNG_IS_READ_STRUCT) == 0) {
///        png_app_warning(png_ptr, "png_set_unknown_chunks now expects a valid location");
///        location = (png_byte)(png_ptr->mode & (PNG_HAVE_IHDR|PNG_HAVE_PLTE|PNG_AFTER_IDAT));
///     }
///     if (location == 0)
///        png_error(png_ptr, "invalid location in png_set_unknown_chunks");   /* :1407 */
/// ```
///
/// On a **read** struct the recovery branch is skipped outright, so any location
/// whose 0x0b bits are all clear is fatal at once.  On a write struct it is only
/// fatal while `png_ptr->mode` is still 0, i.e. before `png_write_info`.
///
/// `pngset.c:1538-1540` (`png_set_unknown_chunk_location`) is the milder twin:
///
/// ```c
///     if ((location & (PNG_HAVE_IHDR|PNG_HAVE_PLTE|PNG_AFTER_IDAT)) == 0) {
///        png_app_error(png_ptr, "invalid unknown chunk location");
/// ```
///
/// and needs `0 <= chunk < info_ptr->unknown_chunks_num`, i.e. a successfully
/// stored unknown chunk to re-locate.
#[test]
fn unknown_chunk_locations() {
    let data = vec![1u8, 2, 3, 4];
    for loc in [0u8, 4, 16, 32, 64, 128, 0x14, 1, 2, 8, 3, 0x0b, 0xff] {
        for read in [true, false] {
            for after_ihdr in [false, true] {
                let case = format!(
                    "set_unknown_chunks loc={} read={} after_ihdr={}",
                    loc, read, after_ihdr
                );
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = if read { new_read(api) } else { new_write(api) };
                    if !read {
                        (api.png_set_write_fn)(
                            png,
                            core::ptr::null_mut(),
                            Some(write_cb),
                            Some(flush_cb),
                        );
                    }
                    let mut uc = png_unknown_chunk {
                        name: *b"unKn\0",
                        data: data.as_ptr() as *mut u8,
                        size: data.len(),
                        location: loc,
                    };
                    let g = guarded(api, png, &mut || {
                        (api.png_set_IHDR)(
                            png,
                            info,
                            2,
                            2,
                            8,
                            PNG_COLOR_TYPE_GRAY,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        if after_ihdr && !read {
                            // png_write_info sets PNG_HAVE_IHDR in png_ptr->mode,
                            // which is what check_location falls back on.
                            (api.png_write_info)(png, info);
                        }
                        (api.png_set_unknown_chunks)(png, info, &mut uc, 1);
                        log(format!(
                            "stored={}",
                            (api.png_get_unknown_chunks)(png, info, core::ptr::null_mut())
                        ));
                    });
                    o.push(format!("guard={:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    if read {
                        destroy_read(api, png, info)
                    } else {
                        destroy_write(api, png, info)
                    }
                    o
                });
            }
        }
    }
    // png_set_unknown_chunk_location: pngset.c:1540
    for idx in [-1i32, 0, 1, 2, 99] {
        for loc in [0i32, 4, 16, 1, 2, 8, 0x0b, 0xff, -1, 0x10000] {
            for benign in [false, true] {
                let case = format!("set_unknown_chunk_location idx={} loc={} benign={}", idx, loc, benign);
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = new_write(api);
                    (api.png_set_write_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(write_cb),
                        Some(flush_cb),
                    );
                    let mut uc = png_unknown_chunk {
                        name: *b"unKn\0",
                        data: data.as_ptr() as *mut u8,
                        size: data.len(),
                        location: PNG_INFO_CHUNK_LOCATION_BEFORE_PLTE,
                    };
                    let g = guarded(api, png, &mut || {
                        if benign {
                            (api.png_set_benign_errors)(png, 1);
                        }
                        (api.png_set_IHDR)(
                            png,
                            info,
                            2,
                            2,
                            8,
                            PNG_COLOR_TYPE_GRAY,
                            PNG_INTERLACE_NONE,
                            PNG_COMPRESSION_TYPE_BASE,
                            PNG_FILTER_TYPE_BASE,
                        );
                        (api.png_set_unknown_chunks)(png, info, &mut uc, 1);
                        (api.png_set_unknown_chunk_location)(png, info, idx, loc);
                        let mut got: *mut png_unknown_chunk = core::ptr::null_mut();
                        let n = (api.png_get_unknown_chunks)(png, info, &mut got);
                        log(format!("n={}", n));
                        for i in 0..n as usize {
                            let u = &*got.add(i);
                            log(format!("unknown[{}] loc={} size={}", i, u.location, u.size));
                        }
                    });
                    o.push(format!("guard={:?}", g));
                    destroy_write(api, png, info);
                    o
                });
            }
        }
    }
}

/* ================================================================== */
/* pngset.c:1681 -- "png_set_keep_unknown_chunks: too many chunks"     */
/* ================================================================== */

/// `pngset.c:1679-1683`:
///
/// ```c
///     if (num_chunks + old_num_chunks > UINT_MAX/5)
///        png_app_error(png_ptr, "png_set_keep_unknown_chunks: too many chunks");
/// ```
///
/// `num_chunks` is `(unsigned)num_chunks_in`, so any `num_chunks_in` above
/// `UINT_MAX/5 == 858993459` (and any negative value other than the documented
/// `-1`, which is remapped to the built-in ignore list) trips it.  The list
/// pointer is never dereferenced before the check, so a 5-byte list is enough.
#[test]
fn keep_unknown_too_many() {
    let list: Vec<u8> = b"unKn\0abCd\0".to_vec();
    for n in [
        -1i32,
        0,
        1,
        2,
        858_993_459,
        858_993_460,
        1_000_000_000,
        i32::MAX,
    ] {
        for keep in [
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            PNG_HANDLE_CHUNK_NEVER,
            PNG_HANDLE_CHUNK_IF_SAFE,
            PNG_HANDLE_CHUNK_ALWAYS,
            -1,
            PNG_HANDLE_CHUNK_LAST,
        ] {
            for benign in [false, true] {
                let case = format!("keep_unknown n={} keep={} benign={}", n, keep, benign);
                assert_same_forked(&case, |api| unsafe {
                    let (png, _info) = new_read(api);
                    let g = guarded(api, png, &mut || {
                        if benign {
                            (api.png_set_benign_errors)(png, 1);
                        }
                        // A first, small call so that `old_num_chunks` is
                        // non-zero for the big one.
                        (api.png_set_keep_unknown_chunks)(
                            png,
                            PNG_HANDLE_CHUNK_ALWAYS,
                            list.as_ptr(),
                            2,
                        );
                        (api.png_set_keep_unknown_chunks)(png, keep, list.as_ptr(), n);
                    });
                    format!("{:?}", g)
                });
            }
        }
    }
    // NULL list with a positive count: "png_set_keep_unknown_chunks: no chunk
    // list" (pngset.c:1665), the check immediately before.
    for n in [1i32, 5, i32::MAX] {
        assert_same(&format!("keep_unknown null list n={}", n), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_read(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_keep_unknown_chunks)(
                    png,
                    PNG_HANDLE_CHUNK_ALWAYS,
                    core::ptr::null(),
                    n,
                );
            });
            o.push(format!("{:?}", g));
            destroy_read(api, png, info);
            o
        });
    }
}

/* ================================================================== */
/* pngset.c:950 -- "Insufficient memory to store text"                 */
/* ================================================================== */

/// `pngset.c:947-950`:
///
/// ```c
///     ret = png_set_text_2(png_ptr, info_ptr, text_ptr, num_text);
///     if (ret != 0)
///        png_error(png_ptr, "Insufficient memory to store text");
/// ```
///
/// `png_set_text_2` returns 1 from exactly two places, both of which report
/// through `png_chunk_report(..., PNG_CHUNK_WRITE_ERROR)` first:
/// "too many text chunks" (pngset.c:1000, the `png_realloc_array` failure) and
/// "text chunk: out of memory" (pngset.c:1092, the `png_malloc_base` failure).
///
/// On a **write** struct `png_chunk_report` with `PNG_CHUNK_WRITE_ERROR` becomes
/// `png_app_error` (pngerror.c:510), which is fatal and never returns — so
/// pngset.c:950 is dead there.  On a **read** struct the same call becomes
/// `png_chunk_warning` (pngerror.c:493), returns, and the `png_error` at :950
/// fires.  Hence: a read struct plus an allocator that fails at exactly the
/// right allocation.
///
/// Allocation order on a read struct is #1 `png_struct`, #2 `png_info`,
/// #3 the `info_ptr->text` array, #4 the per-entry key/text block.
static FAIL_AFTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn failing_malloc(_png: *mut PngStruct, size: usize) -> *mut c_void {
    let t = tls();
    t.alloc_serial += 1;
    let n = FAIL_AFTER.load(std::sync::atomic::Ordering::Relaxed);
    if t.alloc_serial as usize > n {
        log(format!("malloc #{} size={} -> FAIL", t.alloc_serial, size));
        return core::ptr::null_mut();
    }
    log(format!("malloc #{} size={} -> ok", t.alloc_serial, size));
    raw_malloc(size.max(1))
}

unsafe extern "C" fn failing_free(_png: *mut PngStruct, p: *mut c_void) {
    raw_free(p);
}

#[test]
fn text_storage_out_of_memory() {
    let key = cs("Comment");
    let txt = cs("some text to store");
    let lang = cs("en");
    let lk = cs("Kommentar");
    for n in 0..8usize {
        for read in [true, false] {
            for comp in [
                PNG_TEXT_COMPRESSION_NONE,
                PNG_TEXT_COMPRESSION_zTXt,
                PNG_ITXT_COMPRESSION_NONE,
            ] {
                for ntext in [1i32, 2, 9] {
                    let case = format!(
                        "set_text oom after={} read={} comp={} n={}",
                        n, read, comp, ntext
                    );
                    assert_same_forked(&case, |api| unsafe {
                        FAIL_AFTER.store(n, std::sync::atomic::Ordering::Relaxed);
                        let sh = &libs().shim;
                        let png = if read {
                            (api.png_create_read_struct_2)(
                                VER,
                                core::ptr::null_mut(),
                                Some(sh.error_fn),
                                Some(warn_cb),
                                core::ptr::null_mut(),
                                Some(failing_malloc),
                                Some(failing_free),
                            )
                        } else {
                            (api.png_create_write_struct_2)(
                                VER,
                                core::ptr::null_mut(),
                                Some(sh.error_fn),
                                Some(warn_cb),
                                core::ptr::null_mut(),
                                Some(failing_malloc),
                                Some(failing_free),
                            )
                        };
                        if png.is_null() {
                            return "create -> NULL".to_string();
                        }
                        let info = (api.png_create_info_struct)(png);
                        if info.is_null() {
                            return "info -> NULL".to_string();
                        }
                        let mut ts: Vec<png_text> = (0..ntext)
                            .map(|_| png_text {
                                compression: comp,
                                key: key.as_ptr() as *mut c_char,
                                text: txt.as_ptr() as *mut c_char,
                                text_length: 0,
                                itxt_length: 0,
                                lang: lang.as_ptr() as *mut c_char,
                                lang_key: lk.as_ptr() as *mut c_char,
                            })
                            .collect();
                        let g = guarded(api, png, &mut || {
                            (api.png_set_text)(png, info, ts.as_mut_ptr(), ntext);
                            let mut got: *mut png_text = core::ptr::null_mut();
                            log(format!(
                                "get_text={}",
                                (api.png_get_text)(
                                    png,
                                    info,
                                    &mut got,
                                    core::ptr::null_mut()
                                )
                            ));
                        });
                        format!("{:?}", g)
                    });
                }
            }
        }
    }
    FAIL_AFTER.store(usize::MAX, std::sync::atomic::Ordering::Relaxed);
}

/* ================================================================== */
/* pngwio.c:60 -- "Write Error"                                       */
/* ================================================================== */

/// `pngwio.c:57-60`, in `png_default_write_data`:
///
/// ```c
///     check = fwrite(data, 1, length, (FILE *)png_ptr->io_ptr);
///     if (check != length)
///        png_error(png_ptr, "Write Error");
/// ```
///
/// `/dev/full` does *not* work for this: glibc buffers, so `fwrite` of the whole
/// of a small PNG succeeds and only the (unchecked) `fflush` in
/// `png_default_flush` would fail.  A stream opened `"rb"` fails in `fwrite`
/// itself and returns 0 immediately, which is what this test uses.  `/dev/full`
/// is exercised too, with a payload large enough to force a flush inside
/// `fwrite`, so that both behaviours are pinned down.
#[test]
fn write_error_from_stdio() {
    for (tag, path, mode, w, h) in [
        ("/dev/null rb", "/dev/null", "rb", 4u32, 3u32),
        ("/dev/zero rb", "/dev/zero", "rb", 4, 3),
        ("/dev/full wb small", "/dev/full", "wb", 4, 3),
        ("/dev/full wb large", "/dev/full", "wb", 200, 200),
    ] {
        let mut rng = Rng::new(0x60 ^ w as u64);
        let img = Img::random(&mut rng, w, h, PNG_COLOR_TYPE_RGB, 8);
        assert_same_forked(&format!("png_init_io {}", tag), |api| unsafe {
            let p = cs(path);
            let m = cs(mode);
            let f = fopen(p.as_ptr(), m.as_ptr());
            if f.is_null() {
                return format!("fopen {} failed", path);
            }
            let (png, info) = new_write(api);
            (api.png_init_io)(png, f);
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
                    (api.png_write_row)(png, r.as_ptr() as *mut u8);
                }
                (api.png_write_end)(png, info);
                (api.png_write_flush)(png);
            });
            let _ = fclose(f);
            format!("{:?}", g)
        });
    }
}

/* ================================================================== */
/* pngwutil.c:200 -- "length exceeds PNG maximum"                      */
/* ================================================================== */

/// `pngwutil.c:198-200`, in `png_write_complete_chunk`:
///
/// ```c
///     /* On 64-bit architectures 'length' may not fit in a png_uint_32. */
///     if (length > PNG_UINT_31_MAX)
///        png_error(png_ptr, "length exceeds PNG maximum");
/// ```
///
/// Note this is *not* reachable through `png_write_chunk_start`: that forwards to
/// `png_write_chunk_header`, whose `length` is already a `png_uint_32` and which
/// has no such check (pngwutil.c:131-136).  `png_write_chunk` is the public
/// entry point to `png_write_complete_chunk` and takes a `size_t`.
///
/// The error fires before any byte of `data` is touched, so a 4-byte buffer is
/// enough for a claimed length of 2 GiB.
///
/// The *accepted* side of the boundary is deliberately not walked all the way up
/// to `PNG_UINT_31_MAX`: a length of `0x7fffffff` is legal, so libpng would go
/// on to `png_write_chunk_data(data, 0x7fffffff)` and read 2 GiB out of a
/// 4-byte buffer.  Only lengths that the buffer really covers are passed on the
/// legal side.
#[test]
fn write_chunk_length_exceeds_maximum() {
    let data = [1u8, 2, 3, 4];
    for len in [
        0usize,
        1,
        4,
        0x8000_0000,
        0x8000_0001,
        0xffff_ffff,
        0x1_0000_0000,
        usize::MAX,
    ] {
        for name in [*b"unKn", *b"IDAT", *b"1234"] {
            let case = format!("write_chunk len={:#x} name={:?}", len, name);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    // Only lengths <= 4 may actually write; anything larger must
                    // be rejected by pngwutil.c:200 before `data` is read.
                    (api.png_write_chunk)(png, name.as_ptr(), data.as_ptr(), len);
                });
                format!("{:?} out={}", g, tls().output.len())
            });
        }
    }
    // png_write_chunk_start has no such limit; record what it does instead.
    for len in [0u32, 4, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        assert_same(&format!("write_chunk_start len={:#x}", len), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_write_sig)(png);
                (api.png_write_chunk_start)(png, b"unKn".as_ptr(), len);
                (api.png_write_chunk_data)(png, data.as_ptr(), 4);
                (api.png_write_chunk_end)(png);
            });
            o.push(format!("{:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
    }
}

/* ================================================================== */
/* pngset.c:1832 -- "Compression buffer size ..." is DEAD CODE         */
/* ================================================================== */

/// `pngset.c:1826-1836` cannot fire on any build of this source:
///
/// ```c
///     if (size == 0 || size > PNG_UINT_31_MAX)                  /* :1804 */
///        png_error(png_ptr, "invalid compression buffer size");
///     ...
///     if (size > ZLIB_IO_MAX) {                                 /* :1830 */
///        png_warning(png_ptr, "Compression buffer size limited to system maximum");
///        size = ZLIB_IO_MAX; /* must fit */
///     }
/// ```
///
/// `PNG_UINT_31_MAX` is `0x7fffffff` (png.h) and `ZLIB_IO_MAX` is `((uInt)-1)`,
/// i.e. `0xffffffff` (pngstruct.h:53).  Every `size` big enough for the
/// `> ZLIB_IO_MAX` test has already been rejected, fatally, by the
/// `> PNG_UINT_31_MAX` test 26 lines earlier — the comment at :1827 ("this is
/// always false ... it can be true when integer overflow happens") refers to a
/// 32-bit `size_t`, where `PNG_UINT_31_MAX < SIZE_MAX < ZLIB_IO_MAX` does not
/// hold in the same way.
///
/// This test walks both boundaries and asserts what is observable instead:
/// everything above `PNG_UINT_31_MAX` produces "invalid compression buffer
/// size", on a read struct and on a write struct alike.
#[test]
fn compression_buffer_size_system_maximum() {
    for size in [
        0usize,
        1,
        5,
        6,
        7,
        0x7fff_fffe,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
        0x1_0000_0000,
        0x1_0000_0001,
        usize::MAX,
    ] {
        for write in [false, true] {
            for in_use in [false, true] {
                let case = format!(
                    "compression_buffer_size {:#x} write={} in_use={}",
                    size, write, in_use
                );
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let (png, info) = if write { new_write(api) } else { new_read(api) };
                    if write {
                        (api.png_set_write_fn)(
                            png,
                            core::ptr::null_mut(),
                            Some(write_cb),
                            Some(flush_cb),
                        );
                    }
                    let row = [0u8; 4];
                    let g = guarded(api, png, &mut || {
                        if write && in_use {
                            // claim the deflate stream, so that the
                            // "cannot be changed because it is in use" branch at
                            // pngset.c:1818 is exercised too
                            (api.png_set_IHDR)(
                                png,
                                info,
                                2,
                                2,
                                8,
                                PNG_COLOR_TYPE_GRAY,
                                PNG_INTERLACE_NONE,
                                PNG_COMPRESSION_TYPE_BASE,
                                PNG_FILTER_TYPE_BASE,
                            );
                            (api.png_write_info)(png, info);
                            (api.png_write_row)(png, row.as_ptr() as *mut u8);
                        }
                        (api.png_set_compression_buffer_size)(png, size);
                        log(format!(
                            "get={}",
                            (api.png_get_compression_buffer_size)(png)
                        ));
                    });
                    o.push(format!("{:?}", g));
                    o.output = std::mem::take(&mut tls().output);
                    if write {
                        destroy_write(api, png, info)
                    } else {
                        destroy_read(api, png, info)
                    }
                    o
                });
            }
        }
    }
}

/* ================================================================== */
/* pngwutil.c:773 / 804 -- png_write_IHDR called directly              */
/* ================================================================== */

/// `pngwutil.c:771-806`:
///
/// ```c
///     if (compression_type != PNG_COMPRESSION_TYPE_BASE) {
///        png_warning(png_ptr, "Invalid compression type specified");   /* :773 */
///        compression_type = PNG_COMPRESSION_TYPE_BASE;
///     }
///     ...
///     if (interlace_type != PNG_INTERLACE_NONE && interlace_type != PNG_INTERLACE_ADAM7) {
///        png_warning(png_ptr, "Invalid interlace type specified");     /* :804 */
///        interlace_type = PNG_INTERLACE_ADAM7;
///     }
/// ```
///
/// Unreachable through `png_set_IHDR`, which runs `png_check_IHDR` first: that
/// warns ("Unknown compression method in IHDR", png.c:2075 / "Unknown interlace
/// method in IHDR", png.c:2069) and then makes the whole call fatal with
/// "Invalid IHDR data" (png.c:2121).  `png_write_IHDR` is exported, so it can be
/// called directly with the values `png_set_IHDR` would have refused.
///
/// The argument order is `(png_ptr, width, height, bit_depth, color_type,
/// compression_method, filter_method, interlace_method)` — pngpriv.h:1280.
#[test]
fn write_IHDR_direct() {
    for comp in [-1i32, 0, 1, 2, 64, 99, 255, 256] {
        for il in [-1i32, 0, 1, 2, 3, 99, 255] {
            for filt in [0i32, 64, 1] {
                for (ct, bd) in [
                    (PNG_COLOR_TYPE_GRAY, 8),
                    (PNG_COLOR_TYPE_RGB, 8),
                    (PNG_COLOR_TYPE_PALETTE, 4),
                    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
                ] {
                    let case = format!(
                        "write_IHDR comp={} il={} filt={} ct={} bd={}",
                        comp, il, filt, ct, bd
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
                            (api.png_write_sig)(png);
                            (api.png_write_IHDR)(png, 4, 3, bd, ct, comp, filt, il);
                        });
                        o.push(format!("{:?}", g));
                        o.output = std::mem::take(&mut tls().output);
                        destroy_write(api, png, info);
                        o
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* pngwutil.c:1148 / 1154 -- png_write_iCCP called directly            */
/* ================================================================== */

/// `pngwutil.c:1151-1154`:
///
/// ```c
///     name_len = png_check_keyword(png_ptr, name, new_name);
///     if (name_len == 0)
///        png_error(png_ptr, "iCCP: invalid keyword");
/// ```
///
/// `png_check_keyword` returns 0 for an empty keyword and for one that consists
/// only of characters it strips (leading/trailing spaces), so `""` or `"   "`
/// reaches it.  `png_set_iCCP` accepts such a name silently (pngset.c:900 only
/// rejects a NULL one) and it is `png_write_iCCP`, at `png_write_info` time, that
/// rejects it — but only *after* the profile checks at pngwutil.c:1131-1148,
/// which need a well-formed profile.  Both routes are exercised: through
/// `png_set_iCCP` + `png_write_info`, and by calling `png_write_iCCP` directly.
///
/// `pngwutil.c:1147-1148` "Profile length does not match profile" is dead code:
///
/// ```c
///     if (png_get_uint_32(profile) != profile_len)
///        png_error(png_ptr, "Incorrect data in iCCP");            /* :1137-1138 */
///     ...
///     png_uint_32 embedded_profile_len = png_get_uint_32(profile);
///     if (profile_len != embedded_profile_len)
///        png_error(png_ptr, "Profile length does not match profile");  /* :1148 */
/// ```
///
/// The two `if`s test the identical expression, so the earlier "Incorrect data
/// in iCCP" always wins.  This test asserts that: a profile whose embedded
/// length disagrees with `proflen` must produce "Incorrect data in iCCP".
#[test]
fn iccp_write_direct() {
    let good = icc_profile(260);
    let mut short_len = good.clone();
    short_len[0..4].copy_from_slice(&100u32.to_be_bytes());
    let mut long_len = good.clone();
    long_len[0..4].copy_from_slice(&99_999u32.to_be_bytes());
    let mut v4_unaligned = icc_profile(262);
    v4_unaligned[8] = 4;
    let profiles: [(&str, &Vec<u8>); 4] = [
        ("good", &good),
        ("embedded length too short", &short_len),
        ("embedded length too long", &long_len),
        ("v4 unaligned length", &v4_unaligned),
    ];
    for (ptag, prof) in profiles {
        for kw in ["", " ", "   ", "\t", "k", "  spaced  "] {
            let name = cs(kw);
            // (a) png_write_iCCP directly
            let case = format!("write_iCCP direct kw={:?} prof={}", kw, ptag);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    (api.png_write_iCCP)(png, name.as_ptr(), prof.as_ptr(), prof.len() as u32);
                });
                format!("{:?} out={}", g, tls().output.len())
            });
            // (b) through png_set_iCCP + png_write_info
            let case = format!("set_iCCP then write_info kw={:?} prof={}", kw, ptag);
            assert_same_forked(&case, |api| unsafe {
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
                        2,
                        2,
                        8,
                        PNG_COLOR_TYPE_RGB,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_iCCP)(
                        png,
                        info,
                        name.as_ptr(),
                        0,
                        prof.as_ptr(),
                        prof.len() as u32,
                    );
                    (api.png_write_info)(png, info);
                });
                format!("{:?} out={}", g, tls().output.len())
            });
        }
        // a proflen that disagrees with the buffer's own header, both ways
        for plen in [0u32, 4, 131, 132, 200, 260, 261, 999] {
            let case = format!("write_iCCP proflen={} prof={}", plen, ptag);
            let mut buf = prof.clone();
            buf.resize(buf.len().max(plen as usize), 0);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let nm = cs("icc");
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    (api.png_write_iCCP)(png, nm.as_ptr(), buf.as_ptr(), plen);
                });
                format!("{:?} out={}", g, tls().output.len())
            });
        }
    }
}

/* ================================================================== */
/* pngwutil.c:1194 -- png_write_sPLT called directly                   */
/* ================================================================== */

/// `pngwutil.c:1191-1194`:
///
/// ```c
///     name_len = png_check_keyword(png_ptr, spalette->name, new_name);
///     if (name_len == 0)
///        png_error(png_ptr, "sPLT: invalid keyword");
/// ```
///
/// `png_set_sPLT` stores whatever name it is given (pngset.c only rejects a NULL
/// `entries`/name), so this only fires later, from `png_write_sPLT`.  Calling it
/// directly keeps the case independent of the write sequencing.
#[test]
fn splt_write_direct() {
    let ent = vec![
        png_sPLT_entry { red: 1, green: 2, blue: 3, alpha: 4, frequency: 5 };
        8
    ];
    for kw in ["", " ", "    ", "\t\t", "ok", "  padded  "] {
        for depth in [8u8, 16, 1] {
            for nent in [0i32, 1, 8] {
                let nm = cs(kw);
                let case = format!("write_sPLT kw={:?} depth={} n={}", kw, depth, nent);
                assert_same_forked(&case, |api| unsafe {
                    let (png, _info) = new_write(api);
                    (api.png_set_write_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(write_cb),
                        Some(flush_cb),
                    );
                    let sp = png_sPLT_t {
                        name: nm.as_ptr() as *mut c_char,
                        depth,
                        entries: ent.as_ptr() as *mut png_sPLT_entry,
                        nentries: nent,
                    };
                    let g = guarded(api, png, &mut || {
                        (api.png_write_sig)(png);
                        (api.png_write_sPLT)(png, &sp);
                    });
                    format!("{:?} out={:02x?}", g, tls().output)
                });
            }
        }
    }
}

/* ================================================================== */
/* pngwutil.c:1692 -- png_write_iTXt called directly                   */
/* ================================================================== */

/// `pngwutil.c:1679-1693`:
///
/// ```c
///     switch (compression)
///     {
///        case PNG_ITXT_COMPRESSION_NONE:            /*  1 */
///        case PNG_TEXT_COMPRESSION_NONE:            /* -1 */
///           compression = new_key[++key_len] = 0; break;
///        case PNG_TEXT_COMPRESSION_zTXt:            /*  0 */
///        case PNG_ITXT_COMPRESSION_zTXt:            /*  2 */
///           compression = new_key[++key_len] = 1; break;
///        default:
///           png_error(png_ptr, "iTXt: invalid compression");
///     }
/// ```
///
/// So the accepted set is `{-1, 0, 1, 2}` and anything else is fatal.
/// `png_set_text` cannot deliver such a value: `png_set_text_2` rejects
/// `compression < PNG_TEXT_COMPRESSION_NONE || >= PNG_TEXT_COMPRESSION_LAST`
/// (pngset.c:1028) with "text compression mode is out of range" — the same
/// `{-1,0,1,2}` window.  `png_write_iTXt` is exported, so it can be called with
/// 3, -2, ... directly.  Note the keyword check at pngwutil.c:1675 runs first,
/// so the keyword must be valid to get to the `switch`.
#[test]
fn itxt_write_direct() {
    let lang = cs("en");
    let lk = cs("Kommentar");
    let text = cs("hello iTXt");
    for comp in [-4i32, -3, -2, -1, 0, 1, 2, 3, 4, 99, i32::MIN, i32::MAX] {
        for kw in ["Comment", "", "  "] {
            let key = cs(kw);
            let case = format!("write_iTXt comp={} kw={:?}", comp, kw);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    (api.png_write_iTXt)(
                        png,
                        comp,
                        key.as_ptr(),
                        lang.as_ptr(),
                        lk.as_ptr(),
                        text.as_ptr(),
                    );
                });
                format!("{:?} out={}", g, tls().output.len())
            });
        }
    }
    // The neighbouring direct writers, for the same "invalid keyword" family:
    // png_write_tEXt ("tEXt: invalid keyword", pngwutil.c:1580) and
    // png_write_zTXt ("zTXt: invalid compression type", pngwutil.c:1628, then
    // "zTXt: invalid keyword" at pngwutil.c:1633).
    for kw in ["", " ", "Comment"] {
        let key = cs(kw);
        for comp in [-1i32, 0, 1, 2, 3] {
            let case = format!("write_tEXt/zTXt kw={:?} comp={}", kw, comp);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    (api.png_write_tEXt)(png, key.as_ptr(), text.as_ptr(), 0);
                });
                let out1 = tls().output.len();
                let g2 = guarded(api, png, &mut || {
                    (api.png_write_zTXt)(png, key.as_ptr(), text.as_ptr(), comp);
                });
                format!("{:?} {:?} out={} {}", g, g2, out1, tls().output.len())
            });
        }
    }
}

/* ================================================================== */
/* pngwutil.c:1862 -- "Can't write sCAL (buffer too small)"            */
/* ================================================================== */

/// `pngwutil.c:1856-1864`:
///
/// ```c
///     wlen = strlen(width);  hlen = strlen(height);
///     total_len = wlen + hlen + 2;
///     if (total_len > 64) {
///        png_warning(png_ptr, "Can't write sCAL (buffer too small)");
///        return;
///     }
/// ```
///
/// `png_set_sCAL_s` accepts any string `png_check_fp_string` calls a number, and
/// a decimal fraction can be arbitrarily long, so two 32-digit numbers already
/// exceed the 64-byte `buf`.  The boundary (63/64/65) is walked exactly.
#[test]
fn scal_write_buffer_too_small() {
    // "1." followed by n digits is always a valid fp number of length n+2.
    let num = |n: usize| -> String {
        let mut s = String::from("1.");
        for i in 0..n {
            s.push((b'0' + (i % 10) as u8) as char);
        }
        s
    };
    for (wl, hl) in [
        (1usize, 1usize),
        (28, 30),
        (29, 31),
        (30, 31),
        (30, 32),
        (31, 31),
        (40, 40),
        (100, 3),
        (3, 100),
    ] {
        let w = num(wl);
        let h = num(hl);
        let total = w.len() + h.len() + 2;
        for unit in [1i32, 2] {
            // (a) png_write_sCAL_s directly
            let case = format!("write_sCAL_s total={} unit={}", total, unit);
            assert_same_forked(&case, |api| unsafe {
                let (png, _info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let a = cs(&w);
                let b = cs(&h);
                let g = guarded(api, png, &mut || {
                    (api.png_write_sig)(png);
                    (api.png_write_sCAL_s)(png, unit, a.as_ptr(), b.as_ptr());
                });
                format!("{:?} out={}", g, tls().output.len())
            });
            // (b) through png_set_sCAL_s + png_write_info
            let case = format!("set_sCAL_s then write_info total={} unit={}", total, unit);
            assert_same_forked(&case, |api| unsafe {
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(
                    png,
                    core::ptr::null_mut(),
                    Some(write_cb),
                    Some(flush_cb),
                );
                let a = cs(&w);
                let b = cs(&h);
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        2,
                        2,
                        8,
                        PNG_COLOR_TYPE_GRAY,
                        PNG_INTERLACE_NONE,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    (api.png_set_sCAL_s)(png, info, unit, a.as_ptr(), b.as_ptr());
                    (api.png_write_info)(png, info);
                });
                format!("{:?} out={}", g, tls().output.len())
            });
        }
    }
}

/* ================================================================== */
/* pngwutil.c:1912 -- "Invalid time specified for tIME chunk"          */
/* ================================================================== */

/// `pngwutil.c:1908-1914`:
///
/// ```c
///     if (mod_time->month  > 12 || mod_time->month  < 1 ||
///         mod_time->day    > 31 || mod_time->day    < 1 ||
///         mod_time->hour   > 23 || mod_time->second > 60) {
///        png_warning(png_ptr, "Invalid time specified for tIME chunk");
///        return;
///     }
/// ```
///
/// `png_set_tIME` filters the same values out first, with the *different*
/// message "Ignoring invalid time value" (png.c:802 / pngset.c), so nothing that
/// goes through the public setter can arrive here.  `png_write_tIME` is
/// exported, so it can be handed the out-of-range struct directly.  Note
/// `minute` is deliberately *not* validated by the C code — that is recorded
/// too.
#[test]
fn write_tIME_direct() {
    let times: [png_time; 16] = [
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
        png_time { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 0, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 13, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 255, day: 1, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 0, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 32, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 255, hour: 0, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 1, hour: 24, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 1, hour: 255, minute: 0, second: 0 },
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 60, second: 0 },
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 255, second: 0 },
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 0, second: 61 },
        png_time { year: 2024, month: 1, day: 1, hour: 0, minute: 0, second: 255 },
        png_time { year: 65535, month: 6, day: 15, hour: 12, minute: 30, second: 30 },
    ];
    for (i, t) in times.iter().enumerate() {
        // (a) png_write_tIME directly
        assert_same(&format!("write_tIME direct #{} {:?}", i, t), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, core::ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let g = guarded(api, png, &mut || {
                (api.png_write_sig)(png);
                (api.png_write_tIME)(png, t);
            });
            o.push(format!("{:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            o
        });
        // (b) through png_set_tIME, which rejects with a different message
        assert_same(&format!("set_tIME #{} {:?}", i, t), |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            let g = guarded(api, png, &mut || {
                (api.png_set_tIME)(png, info, t);
                let mut got: *mut png_time = core::ptr::null_mut();
                log(format!(
                    "get_tIME={} null={}",
                    (api.png_get_tIME)(png, info, &mut got),
                    got.is_null()
                ));
            });
            o.push(format!("{:?}", g));
            destroy_write(api, png, info);
            o
        });
    }
}

/* ================================================================== */
/* png.c:3634 -- "gamma table being rebuilt"                           */
/* ================================================================== */

/// `png.c:3632-3636`:
///
/// ```c
///     if (png_ptr->gamma_table != NULL || png_ptr->gamma_16_table != NULL)
///     {
///        png_warning(png_ptr, "gamma table being rebuilt");
///        png_destroy_gamma_table(png_ptr);
///     }
/// ```
///
/// `png_build_gamma_table` is called from `png_init_read_transformations`
/// (pngrtran.c:1685), which runs from `png_read_start_row`.  Calling
/// `png_read_update_info` twice does *not* get there: the second call is caught
/// by pngread.c:191 ("png_read_update_info/png_start_read_image: duplicate
/// call") because `PNG_FLAG_ROW_INIT` is already set.  `png_read_start_row`
/// itself has no such guard and is exported, so calling it a second time
/// re-enters `png_build_gamma_table` with the tables still allocated.
///
/// A significant gamma is needed for the tables to be built at all
/// (pngrtran.c:1671), hence `png_set_gamma_fixed`.  The second
/// `png_read_start_row` then also re-claims the zstream, which
/// `png_inflate_claim` reports (pngrutil.c:416-431) — that outcome is part of
/// what is compared.
#[test]
fn gamma_table_being_rebuilt() {
    for (ct, bd) in [
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ] {
        let good = base_png(ct, bd, PNG_INTERLACE_NONE, 6, 4, 0x3634 ^ ct as u64);
        for nstart in [1usize, 2, 3] {
            for update_first in [false, true] {
                let case = format!(
                    "gamma rebuild ct={} bd={} starts={} update_first={}",
                    ct, bd, nstart, update_first
                );
                assert_same_forked(&case, |api| unsafe {
                    tls().input = good.clone();
                    tls().in_pos = 0;
                    let (png, info) = new_read(api);
                    (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                    let g = guarded(api, png, &mut || {
                        (api.png_read_info)(png, info);
                        // screen 1.0 / file 0.45455: the *product* has to be far
                        // from 1.0 for png_init_gamma_values (pngrtran.c:1405) to
                        // report a correction, without which
                        // png_init_read_transformations never calls
                        // png_build_gamma_table at all.  (2.2 / 0.45455 would
                        // multiply out to 1.00001 and build nothing.)
                        (api.png_set_gamma_fixed)(png, 100000, 45455);
                        if update_first {
                            (api.png_read_update_info)(png, info);
                        }
                        for i in 0..nstart {
                            log(format!("--- png_read_start_row #{} ---", i + 1));
                            (api.png_read_start_row)(png);
                        }
                        log(format!("rowbytes={}", (api.png_get_rowbytes)(png, info)));
                    });
                    format!("{:?}", g)
                });
            }
        }
        // and png_read_update_info twice, which is the *guarded* path
        assert_same(
            &format!("update_info twice ct={} bd={}", ct, bd),
            |api| unsafe {
                let mut o = Outcome::default();
                tls().input = good.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_read_fn)(png, core::ptr::null_mut(), Some(read_cb));
                let g = guarded(api, png, &mut || {
                    (api.png_set_benign_errors)(png, 1);
                    (api.png_read_info)(png, info);
                    (api.png_set_gamma_fixed)(png, 100000, 45455);
                    (api.png_read_update_info)(png, info);
                    (api.png_read_update_info)(png, info);
                    (api.png_read_update_info)(png, info);
                });
                o.push(format!("{:?}", g));
                destroy_read(api, png, info);
                o
            },
        );
    }
}

/* ================================================================== */
/* pngpread.c:562 -- "Decompression error in IDAT"                     */
/* ================================================================== */

thread_local! {
    static PROG_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

unsafe extern "C" fn prog_info_cb(png: *mut PngStruct, info: *mut PngInfo) {
    let api = cur_api();
    log(format!(
        "info_cb {}x{} depth={} ct={}",
        (api.png_get_image_width)(png, info),
        (api.png_get_image_height)(png, info),
        (api.png_get_bit_depth)(png, info),
        (api.png_get_color_type)(png, info)
    ));
    // MUST be called from the info callback: it is png_read_start_row that sets
    // png_ptr->num_rows and allocates png_ptr->row_buf.  Without it num_rows
    // stays 0, so pngpread.c:553 `row_number >= num_rows` is true and every zlib
    // failure is reported as "Truncated compressed data in IDAT" instead of
    // reaching the pngpread.c:559-562 pair.
    (api.png_read_update_info)(png, info);
    log(format!("rowbytes={}", (api.png_get_rowbytes)(png, info)));
}

unsafe extern "C" fn prog_row_cb(_png: *mut PngStruct, row: *mut u8, n: u32, pass: c_int) {
    PROG_ROWS.with(|c| c.set(c.get() + 1));
    log(format!("row_cb n={} pass={} null={}", n, pass, row.is_null()));
}

unsafe extern "C" fn prog_end_cb(_png: *mut PngStruct, _info: *mut PngInfo) {
    log("end_cb".to_string());
}

/// `pngpread.c:544-563`, in `png_push_read_IDAT`:
///
/// ```c
///     ret = PNG_INFLATE(png_ptr, Z_SYNC_FLUSH);
///     if (ret != Z_OK && ret != Z_STREAM_END) {
///        ...
///        if (png_ptr->row_number >= png_ptr->num_rows || png_ptr->pass > 6)
///           png_warning(png_ptr, "Truncated compressed data in IDAT");
///        else {
///           if (ret == Z_DATA_ERROR)
///              png_benign_error(png_ptr, "IDAT: ADLER32 checksum mismatch");
///           else
///              png_error(png_ptr, "Decompression error in IDAT");      /* :562 */
///        }
/// ```
///
/// Almost every corruption of a deflate stream makes `inflate` return
/// `Z_DATA_ERROR`, which takes the *other* branch — which is why
/// `tests/progressive.rs`'s IDAT fuzzing only ever reaches the ADLER32 message.
/// The remaining code that reaches :562 is `Z_NEED_DICT` (2): a zlib header with
/// the FDICT flag set.  `0x78 0x20` is such a header — `(0x78*256 + 0x20) % 31
/// == 0`, so the header checksum is valid, `0x78 >> 4 == 7` so the window-size
/// guard at pngrutil.c:529 passes, and FDICT (bit 5 of FLG) makes `inflate`
/// stop with `Z_NEED_DICT` after reading the 4-byte DICTID.
///
/// The chunk CRC is computed over the corrupt payload so that the zlib layer,
/// not the CRC layer, is what rejects it.
#[test]
fn progressive_decompression_error() {
    // 4x2 8-bit grey: num_rows == 2, so row_number(0) < num_rows and pass == 0.
    let mut fdict = vec![0x78u8, 0x20];
    fdict.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); /* DICTID */
    fdict.extend_from_slice(&[0x01, 0x05, 0x00, 0xfa, 0xff, 0, 1, 2, 3, 4]);
    let with_fdict = handmade_gray(4, 2, &fdict);

    // The same header without the trailing data, and a couple of other headers
    // whose first inflate() call cannot return Z_OK/Z_STREAM_END/Z_DATA_ERROR.
    let bare_fdict = handmade_gray(4, 2, &[0x78, 0x20, 0x12, 0x34, 0x56, 0x78]);
    let fdict_only = handmade_gray(4, 2, &[0x78, 0x20]);
    // FLG values that keep (CMF*256+FLG) % 31 == 0 with FDICT set: 32, 63(no), ...
    let fdict_fl3 = handmade_gray(4, 2, &[0x78, 0xbe, 0x12, 0x34, 0x56, 0x78, 0x03, 0x00]);
    // A plain Z_DATA_ERROR for contrast (invalid deflate block type 3).
    let data_err = handmade_gray(4, 2, &[0x78, 0x01, 0x07, 0x00, 0x00, 0x00, 0x00, 0x01]);
    let good = handmade_gray(1, 1, &zlib_stored(&[0u8, 0x80]));

    for (tag, data) in [
        ("FDICT + data", &with_fdict),
        ("FDICT header only", &bare_fdict),
        ("FDICT no dictid", &fdict_only),
        ("FDICT FLEVEL3", &fdict_fl3),
        ("Z_DATA_ERROR", &data_err),
        ("valid 1x1", &good),
    ] {
        for feed in [1usize, 3, 8, 4096] {
            for benign in [false, true] {
                let case = format!("progressive {} feed={} benign={}", tag, feed, benign);
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    PROG_ROWS.with(|c| c.set(0));
                    let (png, info) = new_read(api);
                    (api.png_set_progressive_read_fn)(
                        png,
                        core::ptr::null_mut(),
                        Some(prog_info_cb),
                        Some(prog_row_cb),
                        Some(prog_end_cb),
                    );
                    let mut buf = data.to_vec();
                    let g = guarded(api, png, &mut || {
                        if benign {
                            (api.png_set_benign_errors)(png, 1);
                        }
                        let mut pos = 0usize;
                        while pos < buf.len() {
                            let n = feed.min(buf.len() - pos);
                            (api.png_process_data)(png, info, buf.as_mut_ptr().add(pos), n);
                            pos += n;
                        }
                    });
                    o.push(format!("guard={:?} rows={}", g, PROG_ROWS.with(|c| c.get())));
                    destroy_read(api, png, info);
                    o
                });
                // ... and the same stream through the *sequential* reader, whose
                // png_read_IDAT_data reports the identical zlib failure with a
                // different message.
                let case = format!("sequential {} benign={}", tag, benign);
                if feed == 1 {
                    diff_read(&case, data, move |api, png, _| unsafe {
                        if benign {
                            (api.png_set_benign_errors)(png, 1);
                        }
                    });
                }
            }
        }
    }
}

/* ================================================================== */
/* pngread.c:1643 / 3960 / 3986 / 3994 -- the simplified API           */
/* ================================================================== */

/// The four remaining `pngread.c` sites are internal-consistency assertions in
/// the simplified read API.  All four are unreachable from outside; the cases
/// below drive the combinations that come closest, and record everything, so the
/// two libraries have to agree on the *near misses* as well.
///
/// * `pngread.c:1642` "color-map index out of range" —
///   `if (ip > 255) png_error(...)` in `png_create_colormap_entry`.  Every one of
///   the 24 call sites passes either a loop counter bounded by
///   `PNG_GRAY_COLORMAP_ENTRIES` (256), `PNG_GA_COLORMAP_ENTRIES` (256),
///   `PNG_RGB_COLORMAP_ENTRIES + 1 + 27` (== 244), `png_ptr->num_palette`
///   (clamped to 256 at pngread.c:2691) or the literal 254; or the value
///   `gray = PNG_sRGB_FROM_LINEAR(...)`, and `PNG_sRGB_FROM_LINEAR` is
///   `((png_byte)(0xff & ...))` — a byte by construction.  So `ip <= 255`
///   always.
///
/// * `pngread.c:3986` "unexpected alpha swap transformation" — needs
///   `do_local_background == 2` together with `PNG_SWAP_ALPHA` or
///   (`PNG_ADD_ALPHA` and not `PNG_FLAG_FILLER_AFTER`).  The only
///   `png_set_swap_alpha` call in `png_image_read_direct` is at pngread.c:3902,
///   guarded by `if (do_local_background != 2)`; and `png_set_add_alpha` at
///   pngread.c:3859 sits in the `else /* output needs an alpha channel */`
///   branch, which requires `(base_format & PNG_FORMAT_FLAG_ALPHA) == 0`,
///   whereas `do_local_background` is only ever set (pngread.c:3669) when that
///   same bit *is* set.  The two conditions are mutually exclusive.
///
/// * `pngread.c:3959` "png_image_read: alpha channel lost" and
///   `pngread.c:3993` "png_read_image: invalid transformations" — commented
///   "internal error" / "This is actually an internal error." in the C source:
///   they compare the format `png_image_read_direct` asked libpng for against
///   the format `png_read_update_info` produced, and only fire if libpng
///   disagrees with itself.
#[test]
fn simplified_internal_error_neighbourhood() {
    // Inputs chosen to drive do_local_background / do_local_compose:
    // an alpha channel or a tRNS, optionally a gAMA far from sRGB, optionally
    // interlaced (which turns on do_local_scale).
    let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
    for (ct, bd) in [
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 8),
    ] {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for gama in [None, Some(20_000i32)] {
                for trns in [false, true] {
                    let mut rng = Rng::new(0x3960 ^ ct as u64 ^ (bd as u64) << 8);
                    let mut img = Img::random(&mut rng, 7, 5, ct, bd);
                    img.interlace = il;
                    let bytes = with_c(|api| unsafe {
                        write_image(api, &img, &WriteOpts::default(), &mut |a, p, i| {
                            if let Some(g) = gama {
                                (a.png_set_gAMA_fixed)(p, i, g);
                            }
                            if trns {
                                match ct {
                                    PNG_COLOR_TYPE_PALETTE => {
                                        let t = vec![0x40u8; img.palette.len()];
                                        (a.png_set_tRNS)(
                                            p,
                                            i,
                                            t.as_ptr(),
                                            t.len() as c_int,
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
                                        (a.png_set_tRNS)(p, i, core::ptr::null(), 0, &c);
                                    }
                                    PNG_COLOR_TYPE_RGB => {
                                        let c = png_color_16 {
                                            index: 0,
                                            red: 1,
                                            green: 2,
                                            blue: 3,
                                            gray: 0,
                                        };
                                        (a.png_set_tRNS)(p, i, core::ptr::null(), 0, &c);
                                    }
                                    _ => {}
                                }
                            }
                        })
                        .bytes
                    });
                    if bytes.is_empty() {
                        continue;
                    }
                    sources.push((
                        format!(
                            "ct={} bd={} il={} gama={:?} trns={}",
                            ct, bd, il, gama, trns
                        ),
                        bytes,
                    ));
                }
            }
        }
    }

    // The output formats that involve rgb->gray (no COLOR), alpha removal
    // (no ALPHA), AFIRST and LINEAR -- the four ingredients of
    // do_local_background == 2.
    let formats: [(&str, u32); 14] = [
        ("GRAY", PNG_FORMAT_GRAY),
        ("GA", PNG_FORMAT_GA),
        ("AG", PNG_FORMAT_AG),
        ("RGB", PNG_FORMAT_RGB),
        ("RGBA", PNG_FORMAT_RGBA),
        ("ARGB", PNG_FORMAT_ARGB),
        ("BGRA", PNG_FORMAT_BGRA),
        ("ABGR", PNG_FORMAT_ABGR),
        ("LINEAR_Y", PNG_FORMAT_LINEAR_Y),
        ("LINEAR_Y_ALPHA", PNG_FORMAT_LINEAR_Y_ALPHA),
        ("LINEAR_Y_ALPHA|AFIRST", PNG_FORMAT_LINEAR_Y_ALPHA | PNG_FORMAT_FLAG_AFIRST),
        ("LINEAR_RGB", PNG_FORMAT_LINEAR_RGB),
        ("LINEAR_RGB_ALPHA|AFIRST", PNG_FORMAT_LINEAR_RGB_ALPHA | PNG_FORMAT_FLAG_AFIRST),
        ("RGB_COLORMAP", PNG_FORMAT_RGB_COLORMAP),
    ];

    let back = png_color { red: 0x30, green: 0x60, blue: 0x90 };
    for (stag, src) in &sources {
        for (ftag, fmt) in formats {
            for with_back in [false, true] {
                let case = format!("png_image_read {} -> {} back={}", stag, ftag, with_back);
                // The simplified API never longjmps out to the caller (it traps
                // png_error internally with png_safe_execute), so there is
                // nothing here that needs a forked child.
                assert_same(&case, |api| unsafe {
                    let mut o = Outcome::default();
                    let mut im = png_image { version: PNG_IMAGE_VERSION, ..Default::default() };
                    let r = (api.png_image_begin_read_from_memory)(
                        &mut im,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    if r == 0 {
                        o.push(format!("begin=0 woe={}", im.warning_or_error));
                        return o;
                    }
                    im.format = fmt;
                    let channels = (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1;
                    let csize = ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1;
                    let pixch = if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
                        1
                    } else {
                        channels
                    };
                    let stride = (im.width * pixch) as i32;
                    let mut buf = vec![0u8; (stride as usize) * (im.height as usize) * 2 + 64];
                    let mut cmap = vec![0u8; channels as usize * 256 * csize as usize + 64];
                    im.colormap_entries = 256;
                    let rr = (api.png_image_finish_read)(
                        &mut im,
                        if with_back {
                            &back as *const png_color
                        } else {
                            core::ptr::null()
                        },
                        buf.as_mut_ptr() as *mut c_void,
                        stride,
                        cmap.as_mut_ptr() as *mut c_void,
                    );
                    let msg: Vec<u8> = im.message.iter().map(|&c| c as u8).collect();
                    let end = msg.iter().position(|&c| c == 0).unwrap_or(64);
                    o.push(format!(
                        "r={} woe={} fmt=0x{:x} entries={} msg={:?}",
                        rr,
                        im.warning_or_error,
                        im.format,
                        im.colormap_entries,
                        String::from_utf8_lossy(&msg[..end]),
                    ));
                    o.output = buf;
                    o.output.extend_from_slice(&cmap);
                    o
                });
            }
        }
    }
}

/* ================================================================== */
/* the proof that the intended sites really were reached               */
/* ================================================================== */

/// Assert that every message this file set out to produce really was produced by
/// **both** libraries (`assert_same` / `assert_same_forked` compare *before*
/// `observe()` records, so a recorded message is by construction one they
/// agreed on).
///
/// `observed_all()` rather than `observed()` because several of the cases run
/// each side in a `fork()`ed child, whose observations reach this process
/// through `target/observed/<pid>.txt` rather than the in-process set.
///
/// The libtest harness gives no ordering guarantee between `#[test]` functions,
/// so every scenario is called directly from here.
#[test]
fn reached_the_intended_sites() {
    jmp_buf_size_changed();
    libpng_jmp_buf_still_allocated();
    read_row_without_idat();
    too_many_idats_in_read_end();
    transform_after_update_info();
    invalid_background_gamma_type();
    palette_is_null_in_indexed_image();
    chunk_header_invalid_length();
    extra_compressed_data();
    splt_chunk_cache();
    scal_height_rejections();
    invalid_user_transform_pixel_depth();
    unknown_chunk_locations();
    keep_unknown_too_many();
    text_storage_out_of_memory();
    write_error_from_stdio();
    write_chunk_length_exceeds_maximum();
    compression_buffer_size_system_maximum();
    write_IHDR_direct();
    iccp_write_direct();
    splt_write_direct();
    itxt_write_direct();
    scal_write_buffer_too_small();
    write_tIME_direct();
    gamma_table_being_rebuilt();
    progressive_decompression_error();

    let seen = observed_all();
    let want: [(&str, &str); 24] = [
        ("pngerror.c:593", "Libpng jmp_buf still allocated"),
        ("pngerror.c:600", "Application jmp_buf size changed"),
        ("pngread.c:444", "Invalid attempt to read row data"),
        ("pngread.c:729", ".Too many IDATs found"),
        ("pngread.c:747", "..Too many IDATs found"),
        (
            "pngrtran.c:120",
            "invalid after png_start_read_image or png_read_update_info",
        ),
        (
            "pngtrans.c:845",
            "info change after png_start_read_image or png_read_update_info",
        ),
        ("pngrtran.c:1886", "invalid background gamma type"),
        ("pngrtran.c:2104", "Palette is NULL in indexed image"),
        ("pngrutil.c:46 (not :211)", "PNG unsigned integer out of range"),
        ("pngrutil.c:789 / 1455", "extra compressed data"),
        ("pngrutil.c:1577", "No space in chunk cache for sPLT"),
        ("pngrutil.c:2316", "bad height format"),
        ("pngrutil.c:2319", "non-positive height"),
        ("pngrutil.c:3478", "invalid user transform pixel depth"),
        ("pngset.c:1407", "invalid location in png_set_unknown_chunks"),
        ("pngset.c:1540", "invalid unknown chunk location"),
        (
            "pngset.c:1681",
            "png_set_keep_unknown_chunks: too many chunks",
        ),
        ("pngset.c:950", "Insufficient memory to store text"),
        ("pngwio.c:60", "Write Error"),
        ("pngwutil.c:200", "length exceeds PNG maximum"),
        ("pngwutil.c:773", "Invalid compression type specified"),
        ("pngwutil.c:804", "Invalid interlace type specified"),
        ("png.c:3634", "gamma table being rebuilt"),
    ];
    // The write-side keyword / time / sCAL / iCCP sites, kept separate only for
    // a clearer failure message.
    let want2: [(&str, &str); 8] = [
        ("pngwutil.c:1154", "iCCP: invalid keyword"),
        ("pngwutil.c:1137 (not :1148)", "Incorrect data in iCCP"),
        ("pngwutil.c:1194", "sPLT: invalid keyword"),
        ("pngwutil.c:1692", "iTXt: invalid compression"),
        ("pngwutil.c:1862", "Can't write sCAL (buffer too small)"),
        ("pngwutil.c:1912", "Invalid time specified for tIME chunk"),
        ("pngpread.c:562", "Decompression error in IDAT"),
        (
            "pngset.c:1804 (not :1832)",
            "invalid compression buffer size",
        ),
    ];
    let mut missing: Vec<String> = Vec::new();
    for (site, msg) in want.iter().chain(want2.iter()) {
        // png_chunk_* prefix the chunk name, hence the substring relation.
        if !seen.iter().any(|s| s.contains(msg)) {
            missing.push(format!("{} {:?}", site, msg));
        }
    }
    assert!(
        missing.is_empty(),
        "errors_deep did not reach {} of its target sites:\n  {}\n\
         (observed {} distinct diagnostics)",
        missing.len(),
        missing.join("\n  "),
        seen.len()
    );
    eprintln!(
        "errors_deep: reached all {} target diagnostics ({} distinct messages \
         observed by the whole run so far, {} in-process comparisons + {} forked)",
        want.len() + want2.len(),
        seen.len(),
        CASES.load(std::sync::atomic::Ordering::Relaxed),
        FORKED_CASES.load(std::sync::atomic::Ordering::Relaxed)
    );
}
