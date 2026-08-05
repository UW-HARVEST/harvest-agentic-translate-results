//! Phase B — pack/unpack/sprintf. CONFIGS.md rows 32, 33.
//! Rust can call C variadic functions via extern "C" fn(..., ...) types, so we
//! call json_pack / json_unpack / json_sprintf directly through the .so.
mod common;

use common::*;
use std::os::raw::{c_char, c_double, c_int, c_void};

const JSON_SORT_KEYS: usize = 0x80;

// Variadic function pointer types.
type FnPack = unsafe extern "C" fn(*const c_char, ...) -> *mut c_void;
type FnUnpack = unsafe extern "C" fn(*mut c_void, *const c_char, ...) -> c_int;
type FnSprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut c_void;

unsafe fn dump(lib: &libloading::Library, v: *const c_void, flags: usize) -> Option<Vec<u8>> {
    if v.is_null() {
        return None;
    }
    let dumps: libloading::Symbol<FnDumps> = sym(lib, b"json_dumps");
    let s = dumps(v, flags);
    let out = cstr_to_vec(s);
    if !s.is_null() {
        libc_free(s as *mut c_void);
    }
    out
}

/// Compare pack results (dumped) between C and Rust for a closure that calls pack.
fn cmp_pack<F>(label: &str, f: F)
where
    F: Fn(&libloading::Library, &FnPack) -> Option<Vec<u8>>,
{
    let l = libs();
    let c = unsafe {
        let pack: libloading::Symbol<FnPack> = sym(&l.c, b"json_pack");
        f(&l.c, &*pack)
    };
    let r = unsafe {
        let pack: libloading::Symbol<FnPack> = sym(&l.r, b"json_pack");
        f(&l.r, &*pack)
    };
    assert_eq!(c, r, "pack mismatch [{label}]\nC={c:?}\nR={r:?}");
}

#[test]
fn row_32_pack_scalars() {
    unsafe {
        cmp_pack("i", |lib, pack| {
            let v = pack(b"i\0".as_ptr() as *const c_char, 42i32);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("I", |lib, pack| {
            let v = pack(b"I\0".as_ptr() as *const c_char, 9223372036854775807i64);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("f", |lib, pack| {
            let v = pack(b"f\0".as_ptr() as *const c_char, 3.14159f64 as c_double);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("s", |lib, pack| {
            let v = pack(b"s\0".as_ptr() as *const c_char, b"hello\0".as_ptr() as *const c_char);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("b_true", |lib, pack| {
            let v = pack(b"b\0".as_ptr() as *const c_char, 1i32);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("b_false", |lib, pack| {
            let v = pack(b"b\0".as_ptr() as *const c_char, 0i32);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("n", |lib, pack| {
            let v = pack(b"n\0".as_ptr() as *const c_char);
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
    }
}

#[test]
fn row_32_pack_containers() {
    unsafe {
        cmp_pack("array", |lib, pack| {
            let v = pack(
                b"[iisf]\0".as_ptr() as *const c_char,
                1i32,
                2i32,
                b"three\0".as_ptr() as *const c_char,
                4.5f64 as c_double,
            );
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, 0);
            del(v);
            out
        });
        cmp_pack("object", |lib, pack| {
            let v = pack(
                b"{s:i, s:s, s:f}\0".as_ptr() as *const c_char,
                b"a\0".as_ptr() as *const c_char,
                10i32,
                b"b\0".as_ptr() as *const c_char,
                b"str\0".as_ptr() as *const c_char,
                b"c\0".as_ptr() as *const c_char,
                2.5f64 as c_double,
            );
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, JSON_SORT_KEYS);
            del(v);
            out
        });
        cmp_pack("nested", |lib, pack| {
            let v = pack(
                b"{s:[ii], s:{s:i}}\0".as_ptr() as *const c_char,
                b"arr\0".as_ptr() as *const c_char,
                1i32,
                2i32,
                b"obj\0".as_ptr() as *const c_char,
                b"x\0".as_ptr() as *const c_char,
                99i32,
            );
            let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
            let out = dump(lib, v, JSON_SORT_KEYS);
            del(v);
            out
        });
    }
}

#[test]
fn row_32_pack_error_bad_format() {
    // Invalid format → NULL from both.
    cmp_pack("bad", |lib, pack| unsafe {
        let v = pack(b"{i}\0".as_ptr() as *const c_char, 1i32);
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let out = dump(lib, v, 0);
        if !v.is_null() {
            del(v);
        }
        out
    });
}

#[test]
fn row_32_unpack() {
    // Build root via json_pack in each lib, unpack, compare extracted values + ret.
    let l = libs();
    let run = |lib: &libloading::Library| unsafe {
        let pack: libloading::Symbol<FnPack> = sym(lib, b"json_pack");
        let unpack: libloading::Symbol<FnUnpack> = sym(lib, b"json_unpack");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

        let root = pack(
            b"{s:i, s:s, s:[iii]}\0".as_ptr() as *const c_char,
            b"num\0".as_ptr() as *const c_char,
            7i32,
            b"str\0".as_ptr() as *const c_char,
            b"val\0".as_ptr() as *const c_char,
            b"arr\0".as_ptr() as *const c_char,
            10i32,
            20i32,
            30i32,
        );
        let mut num: c_int = 0;
        let mut sptr: *const c_char = std::ptr::null();
        let mut a0: c_int = 0;
        let mut a1: c_int = 0;
        let mut a2: c_int = 0;
        let ret = unpack(
            root,
            b"{s:i, s:s, s:[iii]}\0".as_ptr() as *const c_char,
            b"num\0".as_ptr() as *const c_char,
            &mut num as *mut c_int,
            b"str\0".as_ptr() as *const c_char,
            &mut sptr as *mut *const c_char,
            b"arr\0".as_ptr() as *const c_char,
            &mut a0 as *mut c_int,
            &mut a1 as *mut c_int,
            &mut a2 as *mut c_int,
        );
        let s = cstr_to_vec(sptr);
        del(root);
        (ret, num, s, a0, a1, a2)
    };
    let c = unsafe { run(&l.c) };
    let r = unsafe { run(&l.r) };
    assert_eq!(c, r, "unpack mismatch");
}

#[test]
fn row_32_unpack_type_error() {
    // Unpack with wrong type → -1 from both.
    let l = libs();
    let run = |lib: &libloading::Library| unsafe {
        let pack: libloading::Symbol<FnPack> = sym(lib, b"json_pack");
        let unpack: libloading::Symbol<FnUnpack> = sym(lib, b"json_unpack");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let root = pack(b"[i]\0".as_ptr() as *const c_char, 5i32);
        // Try to unpack integer as string → error
        let mut sptr: *const c_char = std::ptr::null();
        let ret = unpack(
            root,
            b"[s]\0".as_ptr() as *const c_char,
            &mut sptr as *mut *const c_char,
        );
        del(root);
        ret
    };
    let c = unsafe { run(&l.c) };
    let r = unsafe { run(&l.r) };
    assert_eq!(c, r, "unpack type-error mismatch: C={c} R={r}");
}

#[test]
fn row_33_sprintf() {
    let l = libs();
    let run = |lib: &libloading::Library| unsafe {
        let sprintf: libloading::Symbol<FnSprintf> = sym(lib, b"json_sprintf");
        let del: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");
        let v = sprintf(
            b"value=%d str=%s hex=%x\0".as_ptr() as *const c_char,
            255i32,
            b"abc\0".as_ptr() as *const c_char,
            255i32,
        );
        let out = dump(lib, v, 0);
        if !v.is_null() {
            del(v);
        }
        out
    };
    let c = unsafe { run(&l.c) };
    let r = unsafe { run(&l.r) };
    assert_eq!(c, r, "sprintf mismatch");
}
