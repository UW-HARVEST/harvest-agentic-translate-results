//! Tier 7: the remaining exported entry points - the ones that need a
//! `png_struct` but are not reached by a plain encode/decode: numeric and
//! string formatting, keyword checking, raw chunk writing, the option and
//! version queries, the memory hooks, the status and user-transform callbacks
//! and the file-based simplified API.

mod common;
use common::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};

/* ---------------------------------------------- a bare struct for helpers */

/// Run `body` with a fresh write struct (no I/O attached) in each library and
/// compare whatever `body` reports.
fn with_bare_write<T: PartialEq + std::fmt::Debug>(
    label: &str,
    body: impl Fn(&Ctx) -> T,
) {
    let l = libs();
    let run = |lib: &Lib| -> (Result<T, ()>, Diag) {
        diag_reset();
        let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
        let ver = cs(PNG_LIBPNG_VER_STRING);
        let png = unsafe {
            create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
        };
        assert!(!png.is_null());
        let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
        let info = unsafe { create_info(png) };
        let ctx = Ctx { lib, png, info };
        let r = guard(|| body(&ctx));
        let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
        let mut p = png;
        let mut i = info;
        let _ = guard(|| unsafe { destroy(&mut p, &mut i) });
        (r, diag_take())
    };
    let (ra, da) = run(&l.c);
    let (rb, db) = run(&l.r);
    assert_eq!(da, db, "{label}: diagnostics differ");
    assert_eq!(ra.is_ok(), rb.is_ok(), "{label}: error flag differs");
    if let (Ok(x), Ok(y)) = (ra, rb) {
        assert_eq!(x, y, "{label}: result differs");
    }
}

/* ------------------------------------------------------- png_ascii_from_* */

#[test]
fn ascii_from_fp() {
    let values: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 1.5, 3.14159265358979, 2.718281828459045, 1e-5, 1e-10, 1e10,
        1e20, 1e-20, 1e300, 1e-300, 9.9999999, 0.99999999, 1.0 / 3.0, 100000.0, 65535.0, 1e15,
        123456789.0, 0.000123456, -1234.5678, f64::MIN_POSITIVE, f64::MAX,
    ];
    for &v in &values {
        for size in [1usize, 2, 5, 8, 10, 16, 24, 32, 64] {
            for precision in [0u32, 1, 2, 3, 5, 7, 15, 17, 20] {
                let label = format!("ascii_from_fp({v}, size={size}, prec={precision})");
                with_bare_write(&label, |c| {
                    type F = unsafe extern "C-unwind" fn(
                        png_structp,
                        *mut c_char,
                        usize,
                        f64,
                        c_uint,
                    );
                    let f: libloading::Symbol<F> = c.sym("png_ascii_from_fp");
                    let mut buf = vec![0x5au8; 96];
                    unsafe { f(c.png, buf.as_mut_ptr() as *mut c_char, size, v, precision) };
                    buf
                });
            }
        }
    }
}

#[test]
fn ascii_from_fixed() {
    let values: Vec<i32> = vec![
        0, 1, -1, 5, -5, 100000, -100000, 12345, -12345, 99999, 100001, i32::MAX, i32::MIN,
        i32::MAX - 1, i32::MIN + 1, 10, -10, 100, 1000, 65536, 2147483, -2147483,
    ];
    for &v in &values {
        for size in [1usize, 2, 5, 8, 10, 16, 24, 32] {
            let label = format!("ascii_from_fixed({v}, size={size})");
            with_bare_write(&label, |c| {
                type F = unsafe extern "C-unwind" fn(png_structp, *mut c_char, usize, i32);
                let f: libloading::Symbol<F> = c.sym("png_ascii_from_fixed");
                let mut buf = vec![0x5au8; 64];
                unsafe { f(c.png, buf.as_mut_ptr() as *mut c_char, size, v) };
                buf
            });
        }
    }
}

#[test]
fn fixed_conversions() {
    let values: Vec<f64> = vec![
        0.0, 1.0, -1.0, 0.5, 21474.0, 21475.0, -21474.0, -21475.0, 21474.83647, 100000.0,
        1e-6, 214748.0, 214749.0, 1.0 / 3.0, 1e10, -1e10, 9.99999,
    ];
    for &v in &values {
        let label = format!("png_fixed({v})");
        with_bare_write(&label, |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, f64, *const c_char) -> i32;
            let f: libloading::Symbol<F> = c.sym("png_fixed");
            let name = cs("test");
            unsafe { f(c.png, v, name.as_ptr()) }
        });
        let label = format!("png_fixed_ITU({v})");
        with_bare_write(&label, |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, f64, *const c_char) -> u32;
            let f: libloading::Symbol<F> = c.sym("png_fixed_ITU");
            let name = cs("test");
            unsafe { f(c.png, v, name.as_ptr()) }
        });
    }
}

#[test]
fn get_uint_31() {
    let mut bufs: Vec<[u8; 4]> = vec![
        [0, 0, 0, 0],
        [0x7f, 0xff, 0xff, 0xff],
        [0x80, 0, 0, 0],
        [0xff, 0xff, 0xff, 0xff],
        [0, 0, 0, 1],
    ];
    let mut s: u32 = 4242;
    for _ in 0..64 {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        bufs.push(s.to_be_bytes());
    }
    for b in bufs {
        let label = format!("get_uint_31({b:?})");
        with_bare_write(&label, |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, *const u8) -> u32;
            let f: libloading::Symbol<F> = c.sym("png_get_uint_31");
            unsafe { f(c.png, b.as_ptr()) }
        });
    }
}

/* -------------------------------------------------------- png_check_keyword */

#[test]
fn check_keyword() {
    let keys = [
        "",
        "Title",
        " Title",
        "Title ",
        " Title ",
        "Ti  tle",
        "Ti\ttle",
        "Ti\x01tle",
        "Ti\x7ftle",
        "Ti\u{e9}tle",
        "a",
        &"k".repeat(79),
        &"k".repeat(80),
        &"k".repeat(200),
        "  ",
        "\u{a0}key",
        "key\u{a0}",
    ];
    for k in keys {
        let label = format!("check_keyword({k:?})");
        let key = CString::new(k).unwrap();
        with_bare_write(&label, move |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, *const c_char, *mut u8) -> u32;
            let f: libloading::Symbol<F> = c.sym("png_check_keyword");
            let mut newkey = vec![0x5au8; 128];
            let n = unsafe { f(c.png, key.as_ptr(), newkey.as_mut_ptr()) };
            (n, newkey)
        });
    }
}

/* ------------------------------------------------------ raw chunk writing */

#[test]
fn raw_chunk_writing() {
    let l = libs();
    let payloads: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0],
        (0u8..32).collect(),
        vec![0xff; 300],
        b"hello world".to_vec(),
    ];
    for name in [&b"prVt"[..], b"IEND", b"tEXt", b"zzZz"] {
        for p in &payloads {
            let run = |lib: &Lib| {
                write_with(lib, |c, _| {
                    c.call1("png_write_sig");
                    let f: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *const u8, *const u8, usize),
                    > = c.sym("png_write_chunk");
                    unsafe { f(c.png, name.as_ptr(), p.as_ptr(), p.len()) };
                    // and the split form
                    let s: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *const u8, u32),
                    > = c.sym("png_write_chunk_start");
                    unsafe { s(c.png, name.as_ptr(), p.len() as u32) };
                    let d: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *const u8, usize),
                    > = c.sym("png_write_chunk_data");
                    // in two halves to exercise incremental CRC accumulation
                    let half = p.len() / 2;
                    unsafe { d(c.png, p.as_ptr(), half) };
                    unsafe { d(c.png, p.as_ptr().add(half), p.len() - half) };
                    c.call1("png_write_chunk_end");
                    // raw data write and flush
                    let w: libloading::Symbol<
                        unsafe extern "C-unwind" fn(png_structp, *const u8, usize),
                    > = c.sym("png_write_data");
                    unsafe { w(c.png, b"RAW".as_ptr(), 3) };
                    c.call1("png_flush");
                })
            };
            let a = run(&l.c);
            let b = run(&l.r);
            let ctx = format!("{} len={}", String::from_utf8_lossy(name), p.len());
            assert_eq!(a.diag, b.diag, "chunk/{ctx}: diag differs");
            assert_eq!(a.errored, b.errored, "chunk/{ctx}: error differs");
            assert_eq!(a.flushes, b.flushes, "chunk/{ctx}: flushes differ");
            assert_eq!(
                a.bytes, b.bytes,
                "chunk/{ctx}: bytes differ\n C: {}\n R: {}",
                hex(&a.bytes),
                hex(&b.bytes)
            );
        }
    }
}

#[test]
fn crc_accumulation() {
    // png_reset_crc / png_calculate_crc are observable through the CRC that
    // png_write_chunk_end emits, but exercise them directly as well.
    let l = libs();
    let run = |lib: &Lib| {
        write_with(lib, |c, notes| {
            let reset: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp)> =
                c.sym("png_reset_crc");
            let calc: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *const u8, usize),
            > = c.sym("png_calculate_crc");
            // The running CRC is only visible through a written chunk, so wrap
            // the calls in a chunk and let the emitted CRC do the comparing.
            let s: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *const u8, u32)> =
                c.sym("png_write_chunk_start");
            unsafe { s(c.png, b"prVt".as_ptr(), 0) };
            unsafe { reset(c.png) };
            for chunk in [&b"abc"[..], b"", b"0123456789", b"\xff\x00\xff"] {
                unsafe { calc(c.png, chunk.as_ptr(), chunk.len()) };
            }
            c.call1("png_write_chunk_end");
            notes.push("done".to_string());
        })
    };
    let a = run(&l.c);
    let b = run(&l.r);
    assert_eq!(a.bytes, b.bytes, "crc bytes differ: {} vs {}", hex(&a.bytes), hex(&b.bytes));
    assert_eq!(a.diag, b.diag);
}

/* ------------------------------------------------------- option / version */

#[test]
fn options_and_versions() {
    for option in -2i32..12 {
        for onoff in -1i32..4 {
            let label = format!("png_set_option({option},{onoff})");
            with_bare_write(&label, move |c| {
                type F = unsafe extern "C-unwind" fn(png_structp, c_int, c_int) -> c_int;
                let f: libloading::Symbol<F> = c.sym("png_set_option");
                unsafe { f(c.png, option, onoff) }
            });
        }
    }
    for mask in [0u32, 1, 2, 3, 4, 7, 0xffff_ffff] {
        let label = format!("png_permit_mng_features({mask:#x})");
        with_bare_write(&label, move |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, u32) -> u32;
            let f: libloading::Symbol<F> = c.sym("png_permit_mng_features");
            unsafe { f(c.png, mask) }
        });
    }
    for name in [&b"prVt"[..], b"IHDR", b"tEXt", b"zzZz", b"IDAT"] {
        let label = format!("png_handle_as_unknown({})", String::from_utf8_lossy(name));
        with_bare_write(&label, move |c| {
            type F = unsafe extern "C-unwind" fn(png_structp, *const u8) -> c_int;
            let f: libloading::Symbol<F> = c.sym("png_handle_as_unknown");
            unsafe { f(c.png, name.as_ptr()) }
        });
    }
    // png_user_version_check accepts a range of version strings
    let l = libs();
    for v in [
        PNG_LIBPNG_VER_STRING,
        "1.6.59",
        "1.6.0",
        "1.5.0",
        "1.7.0",
        "2.0.0",
        "",
        "garbage",
        "1.6.59.git\0extra",
    ] {
        let run = |lib: &Lib| -> (c_int, Diag) {
            diag_reset();
            let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
            let ver = cs(PNG_LIBPNG_VER_STRING);
            let png = unsafe {
                create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
            };
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *const c_char) -> c_int,
            > = lib.sym("png_user_version_check");
            let s = CString::new(v.replace('\0', "")).unwrap();
            let r = guard(|| unsafe { f(png, s.as_ptr()) });
            let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
            let mut p = png;
            let _ = guard(|| unsafe { destroy(&mut p, std::ptr::null_mut()) });
            (r.unwrap_or(-999), diag_take())
        };
        let a = run(&l.c);
        let b = run(&l.r);
        assert_eq!(a, b, "png_user_version_check({v:?}) differs");
    }
    // creating a struct with a mismatched version string
    for v in ["1.5.0", "1.7.0", "2.0.0", "", "1.6.0"] {
        let run = |lib: &Lib| -> (bool, Diag) {
            diag_reset();
            let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_read_struct");
            let s = cs(v);
            let r = guard(|| unsafe {
                create(s.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
            });
            let ok = match r {
                Ok(p) => {
                    let non_null = !p.is_null();
                    if non_null {
                        let destroy: libloading::Symbol<FnDestroyRead> =
                            lib.sym("png_destroy_read_struct");
                        let mut q = p;
                        let _ = guard(|| unsafe {
                            destroy(&mut q, std::ptr::null_mut(), std::ptr::null_mut())
                        });
                    }
                    non_null
                }
                Err(()) => false,
            };
            (ok, diag_take())
        };
        let a = run(&l.c);
        let b = run(&l.r);
        assert_eq!(a, b, "png_create_read_struct(version {v:?}) differs");
    }
}

/* ------------------------------------------------------------ error paths */

#[test]
fn error_and_warning_emitters() {
    let l = libs();
    let messages = [
        "",
        "plain message",
        "a message with a #hash",
        "@1 parameter",
        "\u{e9} high byte",
        &"x".repeat(200),
    ];
    for m in messages {
        for which in [
            "png_error",
            "png_warning",
            "png_benign_error",
            "png_app_error",
            "png_app_warning",
            "png_chunk_error",
            "png_chunk_warning",
            "png_chunk_benign_error",
            "png_chunk_report",
        ] {
            let run = |lib: &Lib| -> (bool, Diag) {
                diag_reset();
                let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
                let ver = cs(PNG_LIBPNG_VER_STRING);
                let png = unsafe {
                    create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
                };
                let msg = cs(m);
                let r = guard(|| unsafe {
                    if which == "png_chunk_report" {
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, *const c_char, c_int),
                        > = lib.sym(which);
                        f(png, msg.as_ptr(), 1);
                    } else {
                        let f: libloading::Symbol<
                            unsafe extern "C-unwind" fn(png_structp, *const c_char),
                        > = lib.sym(which);
                        f(png, msg.as_ptr());
                    }
                });
                let destroy: libloading::Symbol<FnDestroyWrite> =
                    lib.sym("png_destroy_write_struct");
                let mut p = png;
                let _ = guard(|| unsafe { destroy(&mut p, std::ptr::null_mut()) });
                (r.is_err(), diag_take())
            };
            let a = run(&l.c);
            let b = run(&l.r);
            assert_eq!(a, b, "{which}({m:?}) differs");
        }
    }
}

#[test]
fn formatted_warnings() {
    let l = libs();
    // png_warning_parameters is char[8][32]
    #[repr(C)]
    struct Params([[c_char; 32]; 8]);
    for msg in [
        "no parameters",
        "one @1 parameter",
        "two @1 and @2",
        "all @1@2@3@4@5@6@7@8",
        "out of range @9",
        "trailing @",
    ] {
        let run = |lib: &Lib| -> Diag {
            diag_reset();
            let create: libloading::Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
            let ver = cs(PNG_LIBPNG_VER_STRING);
            let png = unsafe {
                create(ver.as_ptr(), std::ptr::null_mut(), Some(error_cb), Some(warning_cb))
            };
            let mut p = Params([[0; 32]; 8]);
            let set_s: libloading::Symbol<
                unsafe extern "C-unwind" fn(*mut Params, c_int, *const c_char),
            > = lib.sym("png_warning_parameter");
            let set_u: libloading::Symbol<
                unsafe extern "C-unwind" fn(*mut Params, c_int, c_int, usize),
            > = lib.sym("png_warning_parameter_unsigned");
            let set_i: libloading::Symbol<
                unsafe extern "C-unwind" fn(*mut Params, c_int, c_int, i32),
            > = lib.sym("png_warning_parameter_signed");
            let s1 = cs("string one");
            let long = cs(&"y".repeat(64));
            unsafe {
                set_s(&mut p, 1, s1.as_ptr());
                set_s(&mut p, 2, long.as_ptr());
                set_u(&mut p, 3, 1, 12345);
                set_u(&mut p, 4, 2, 7);
                set_u(&mut p, 5, 3, 0xdeadbeef);
                set_u(&mut p, 6, 4, 0xff);
                set_i(&mut p, 7, 1, -4242);
                set_i(&mut p, 8, 5, -100000);
                // out of range indices must be ignored
                set_s(&mut p, 0, s1.as_ptr());
                set_s(&mut p, 9, s1.as_ptr());
            }
            let f: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, *mut Params, *const c_char),
            > = lib.sym("png_formatted_warning");
            let m = cs(msg);
            let _ = guard(|| unsafe { f(png, &mut p, m.as_ptr()) });
            let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
            let mut q = png;
            let _ = guard(|| unsafe { destroy(&mut q, std::ptr::null_mut()) });
            diag_take()
        };
        assert_eq!(run(&l.c), run(&l.r), "png_formatted_warning({msg:?}) differs");
    }
}

/* ---------------------------------------------------------- memory hooks */

thread_local! {
    static MEMLOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static LIVE: RefCell<Vec<(usize, usize)>> = RefCell::new(Vec::new());
}

unsafe extern "C-unwind" fn my_malloc(_p: png_structp, size: usize) -> png_voidp {
    let mut v: Vec<u8> = Vec::with_capacity(size.max(1));
    let ptr = v.as_mut_ptr();
    let cap = v.capacity();
    std::mem::forget(v);
    LIVE.with(|l| l.borrow_mut().push((ptr as usize, cap)));
    MEMLOG.with(|l| l.borrow_mut().push(format!("alloc")));
    ptr as png_voidp
}

unsafe extern "C-unwind" fn my_free(_p: png_structp, ptr: png_voidp) {
    if ptr.is_null() {
        return;
    }
    LIVE.with(|l| {
        let mut l = l.borrow_mut();
        if let Some(i) = l.iter().position(|&(a, _)| a == ptr as usize) {
            let (a, cap) = l.remove(i);
            unsafe { drop(Vec::from_raw_parts(a as *mut u8, 0, cap)) };
        }
    });
    MEMLOG.with(|l| l.borrow_mut().push(format!("free")));
}

#[test]
fn custom_memory_hooks() {
    let l = libs();
    let run = |lib: &Lib| -> (Vec<u8>, usize, bool, Diag) {
        diag_reset();
        MEMLOG.with(|m| m.borrow_mut().clear());
        LIVE.with(|m| m.borrow_mut().clear());
        let mut sink = Box::new(MemWriter { buf: Vec::new(), flushes: 0 });
        type FnCreate2 = unsafe extern "C-unwind" fn(
            *const c_char,
            png_voidp,
            Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
            Option<unsafe extern "C-unwind" fn(png_structp, *const c_char)>,
            png_voidp,
            Option<unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp>,
            Option<unsafe extern "C-unwind" fn(png_structp, png_voidp)>,
        ) -> png_structp;
        let create: libloading::Symbol<FnCreate2> = lib.sym("png_create_write_struct_2");
        let ver = cs(PNG_LIBPNG_VER_STRING);
        let png = unsafe {
            create(
                ver.as_ptr(),
                std::ptr::null_mut(),
                Some(error_cb),
                Some(warning_cb),
                0x1234 as png_voidp,
                Some(my_malloc),
                Some(my_free),
            )
        };
        assert!(!png.is_null());
        let getmem: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp) -> png_voidp> =
            lib.sym("png_get_mem_ptr");
        let memptr = unsafe { getmem(png) } as usize;
        let create_info: libloading::Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
        let info = unsafe { create_info(png) };
        sink_register(png, (&mut *sink) as *mut MemWriter as *mut c_void);
        let set_write: libloading::Symbol<FnSetWriteFn> = lib.sym("png_set_write_fn");
        unsafe {
            set_write(
                png,
                (&mut *sink) as *mut MemWriter as *mut c_void,
                Some(mem_write),
                Some(mem_flush),
            )
        };
        let ctx = Ctx { lib, png, info };
        let res = guard(|| {
            type Fihdr = unsafe extern "C-unwind" fn(
                png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int,
            );
            let f: libloading::Symbol<Fihdr> = ctx.sym("png_set_IHDR");
            unsafe {
                f(png, info, 12, 4, 8, PNG_COLOR_TYPE_RGB as c_int, PNG_INTERLACE_NONE,
                  PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE)
            };
            ctx.call2("png_write_info");
            let rows: Vec<Vec<u8>> = (0..4u32).map(|r| vec![(r * 37) as u8; 36]).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter().map(|x| x.as_ptr() as *mut u8).collect();
            let g: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, *mut *mut u8, u32)> =
                ctx.sym("png_write_image");
            unsafe { g(png, ptrs.as_mut_ptr(), 4) };
            ctx.call2("png_write_end");
            // exercise the public allocator wrappers too
            let m: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp,
            > = ctx.sym("png_malloc");
            let c2: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp,
            > = ctx.sym("png_calloc");
            let mw: libloading::Symbol<
                unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp,
            > = ctx.sym("png_malloc_warn");
            let fr: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_voidp)> =
                ctx.sym("png_free");
            for size in [1usize, 16, 1024] {
                let p1 = unsafe { m(png, size) };
                let p2 = unsafe { c2(png, size) };
                let p3 = unsafe { mw(png, size) };
                // png_calloc must zero its result
                if !p2.is_null() {
                    let s = unsafe { std::slice::from_raw_parts(p2 as *const u8, size) };
                    assert!(s.iter().all(|&b| b == 0), "png_calloc did not zero");
                }
                unsafe { fr(png, p1) };
                unsafe { fr(png, p2) };
                unsafe { fr(png, p3) };
            }
            unsafe { fr(png, std::ptr::null_mut()) };
        });
        let destroy: libloading::Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
        let mut p = png;
        let mut i = info;
        let _ = guard(|| unsafe { destroy(&mut p, &mut i) });
        sink_clear();
        let leaks = LIVE.with(|l| l.borrow().len());
        let allocs = MEMLOG.with(|l| l.borrow().iter().filter(|s| *s == "alloc").count());
        assert!(allocs > 0, "custom allocator was never used");
        assert_eq!(leaks, 0, "custom allocator saw a leak");
        (std::mem::take(&mut sink.buf), memptr, res.is_err(), diag_take())
    };
    let a = run(&l.c);
    let b = run(&l.r);
    assert_eq!(a.1, 0x1234, "C: mem_ptr not returned");
    assert_eq!(b.1, 0x1234, "Rust: mem_ptr not returned");
    assert_eq!(a.2, b.2, "error flag differs");
    assert_eq!(a.3, b.3, "diag differs");
    assert_eq!(a.0, b.0, "bytes differ with custom allocator");
}

#[test]
fn default_allocator_entry_points() {
    // png_malloc_default / png_free_default / png_malloc_base / png_malloc_array
    // and png_realloc_array are reachable with a plain struct.
    with_bare_write("malloc_default", |c| {
        let m: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp> =
            c.sym("png_malloc_default");
        let f: libloading::Symbol<unsafe extern "C-unwind" fn(png_structp, png_voidp)> =
            c.sym("png_free_default");
        let base: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp,
        > = c.sym("png_malloc_base");
        let arr: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, c_int, usize) -> png_voidp,
        > = c.sym("png_malloc_array");
        let realloc: libloading::Symbol<
            unsafe extern "C-unwind" fn(png_structp, *const c_void, c_int, c_int, usize) -> png_voidp,
        > = c.sym("png_realloc_array");
        let mut nonnull = Vec::new();
        for size in [1usize, 32, 4096] {
            let p = unsafe { m(c.png, size) };
            nonnull.push(!p.is_null());
            unsafe { f(c.png, p) };
            let p = unsafe { base(c.png, size) };
            nonnull.push(!p.is_null());
            unsafe { f(c.png, p) };
        }
        let a = unsafe { arr(c.png, 4, 8) };
        nonnull.push(!a.is_null());
        let b = unsafe { realloc(c.png, a, 4, 4, 8) };
        nonnull.push(!b.is_null());
        unsafe { f(c.png, b) };
        // pathological sizes must fail the same way
        for size in [0usize, usize::MAX, usize::MAX / 2] {
            let p = unsafe { base(c.png, size) };
            nonnull.push(!p.is_null());
            if !p.is_null() {
                unsafe { f(c.png, p) };
            }
        }
        let a = unsafe { arr(c.png, -1, 8) };
        nonnull.push(!a.is_null());
        let a = unsafe { arr(c.png, 0, 8) };
        nonnull.push(!a.is_null());
        let a = unsafe { arr(c.png, c_int::MAX, 8) };
        nonnull.push(!a.is_null());
        nonnull
    });
}
