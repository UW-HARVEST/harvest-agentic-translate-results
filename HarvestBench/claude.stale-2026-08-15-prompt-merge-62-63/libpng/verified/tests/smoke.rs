//! Harness smoke test: both libraries load, basic queries agree, and one
//! complete write cycle produces identical bytes.
mod support;

use std::ffi::{c_char, c_int, c_void};
use support::*;

type Png = *mut c_void;

#[test]
fn version_queries_match() {
    let p = pair();
    for lib in [&p.c, &p.rust] {
        let n: unsafe extern "C" fn() -> u32 = lib.f("png_access_version_number");
        assert_eq!(unsafe { n() }, 10659, "{}", lib.tag);
        let ver: unsafe extern "C" fn(*mut c_void) -> *const c_char = lib.f("png_get_libpng_ver");
        assert_eq!(cstr(unsafe { ver(std::ptr::null_mut()) }), "1.6.59.git");
    }
    let a: unsafe extern "C" fn(*mut c_void) -> *const c_char = p.c.f("png_get_copyright");
    let b: unsafe extern "C" fn(*mut c_void) -> *const c_char = p.rust.f("png_get_copyright");
    assert_eq!(
        cstr(unsafe { a(std::ptr::null_mut()) }),
        cstr(unsafe { b(std::ptr::null_mut()) })
    );
    let a: unsafe extern "C" fn(*mut c_void) -> *const c_char = p.c.f("png_get_header_version");
    let b: unsafe extern "C" fn(*mut c_void) -> *const c_char = p.rust.f("png_get_header_version");
    assert_eq!(
        cstr(unsafe { a(std::ptr::null_mut()) }),
        cstr(unsafe { b(std::ptr::null_mut()) })
    );
}

#[test]
fn simple_write_matches() {
    diff("smoke-write", |lib| {
        session_reset(Vec::new());
        let create: unsafe extern "C" fn(*const c_char, *mut c_void, *mut c_void, *mut c_void) -> Png =
            lib.f("png_create_write_struct");
        let set_longjmp: unsafe extern "C" fn(Png, *const c_void, usize) -> *mut c_void =
            lib.f("png_set_longjmp_fn");
        let set_write_fn: unsafe extern "C" fn(Png, *mut c_void, *mut c_void, *mut c_void) =
            lib.f("png_set_write_fn");
        let create_info: unsafe extern "C" fn(Png) -> *mut c_void = lib.f("png_create_info_struct");
        let set_ihdr: unsafe extern "C" fn(Png, *mut c_void, u32, u32, c_int, c_int, c_int, c_int, c_int) =
            lib.f("png_set_IHDR");
        let write_info: unsafe extern "C" fn(Png, *mut c_void) = lib.f("png_write_info");
        let write_row: unsafe extern "C" fn(Png, *const u8) = lib.f("png_write_row");
        let write_end: unsafe extern "C" fn(Png, *mut c_void) = lib.f("png_write_end");
        let destroy: unsafe extern "C" fn(*mut Png, *mut *mut c_void) = lib.f("png_destroy_write_struct");

        let mut row = [0u8; 3 * 8];
        for (i, b) in row.iter_mut().enumerate() {
            *b = (i * 7) as u8;
        }
        let rc = protected(|| unsafe {
            let png = create(VER_STRING.as_ptr() as *const c_char, std::ptr::null_mut(),
                cb_error as *mut c_void, cb_warning as *mut c_void);
            log(format!("create={}", !png.is_null()));
            set_longjmp(png, shim().longjmp_ptr, shim().jmp_buf_size);
            set_write_fn(png, std::ptr::null_mut(), cb_write as *mut c_void, cb_flush as *mut c_void);
            let info = create_info(png);
            set_ihdr(png, info, 8, 4, 8, 2, 0, 0, 0);
            write_info(png, info);
            for _ in 0..4 {
                write_row(png, row.as_ptr());
            }
            write_end(png, info);
            let mut pp = png;
            let mut ip = info;
            destroy(&mut pp, &mut ip);
            log("destroyed".to_string());
        });
        Trace { lines: take_log(), out: take_out(), rc }
    });
}

#[test]
fn simple_read_matches() {
    use support::pngbuild::Builder;
    let png = Builder::new(9, 5, 8, 2).build_valid(12345);
    diff("smoke-read", |lib| {
        let mut rows: Vec<u8> = vec![0; 64];
        with_read(lib, &png, &mut |c, p, i| unsafe {
            (c.read_info)(p, i);
            log_all_info(c, p, i);
            let rb = (c.get_rowbytes)(p, i);
            for r in 0..5 {
                (c.read_row)(p, rows.as_mut_ptr(), std::ptr::null_mut());
                log(format!("row{r}={}", hex(&rows[..rb])));
            }
            (c.read_end)(p, std::ptr::null_mut());
        })
    });
}

#[test]
fn interlaced_read_matches() {
    use support::pngbuild::Builder;
    let png = Builder::new(11, 7, 4, 0).interlace(1).build_valid(999);
    diff("smoke-read-interlace", |lib| {
        let mut buf: Vec<u8> = vec![0; 64];
        with_read(lib, &png, &mut |c, p, i| unsafe {
            (c.read_info)(p, i);
            log_all_info(c, p, i);
            let passes = (c.set_interlace_handling)(p);
            log(format!("passes={passes}"));
            (c.read_update_info)(p, i);
            let rb = (c.get_rowbytes)(p, i);
            for pass in 0..passes {
                for r in 0..7 {
                    (c.read_row)(p, buf.as_mut_ptr(), std::ptr::null_mut());
                    log(format!("p{pass}r{r}={}", hex(&buf[..rb])));
                }
            }
            (c.read_end)(p, std::ptr::null_mut());
        })
    });
}
