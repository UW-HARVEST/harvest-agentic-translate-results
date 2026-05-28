// Integration tests comparing C and Rust .so outputs through libloading.
// We never call Rust functions directly — we always go through the .so exports.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::os::raw::c_void;

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO: &str = "target/release/libcomplexmode_lib.so";

unsafe fn open_libs() -> (Library, Library) {
    let c = unsafe { Library::new(C_SO).expect("failed to load C .so") };
    let r = unsafe { Library::new(RUST_SO).expect("failed to load Rust .so") };
    (c, r)
}

fn get_libc_free() -> unsafe extern "C" fn(*mut c_void) {
    unsafe {
        let libc = libloading::Library::new("libc.so.6").expect("libc");
        let f: Symbol<unsafe extern "C" fn(*mut c_void)> =
            libc.get(b"free\0").expect("libc free");
        let raw = *f;
        std::mem::forget(libc);
        raw
    }
}

#[test]
fn test_check_permissions() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            c.get(b"check_permissions\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            r.get(b"check_permissions\0").unwrap();
        let cases = [
            (0o644, 0o400),
            (0o644, 0o200),
            (0o644, 0o100),
            (0o644, 0o600),
            (0o644, 0o644),
            (0o000, 0o100),
            (0o777, 0o100),
            (-1, 0o100),
            (0, 0),
            (0o100, 0),
        ];
        for (p, req) in cases {
            assert_eq!(cf(p, req), rf(p, req), "perms {:o} req {:o}", p, req);
        }
    }
}

#[test]
fn test_safe_add() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c.get(b"safe_add\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            r.get(b"safe_add\0").unwrap();
        // Has perms (0o600 covers READ|WRITE)
        let cases = [
            (1, 2, 0o644),
            (10, 20, 0o600),
            (-5, 5, 0o644),
            (i32::MAX, 1, 0o644),    // wraparound
            (i32::MIN, -1, 0o644),   // wraparound
            (1, 2, 0o400),           // missing WRITE_PERM => returns 0
            (1, 2, 0o200),           // missing READ_PERM => returns 0
            (1, 2, 0o000),           // no perms => returns 0
        ];
        for (a, b, p) in cases {
            assert_eq!(cf(a, b, p), rf(a, b, p), "safe_add({}, {}, {:o})", a, b, p);
        }
    }
}

#[test]
fn test_create_result_string() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char> =
            c.get(b"create_result_string\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char> =
            r.get(b"create_result_string\0").unwrap();
        let free = get_libc_free();

        let ops: &[&[u8]] = &[
            b"add\0",
            b"multiply\0",
            b"\0",
            b"a-very-long-operation-name-that-is-probably-too-long\0",
        ];
        let vals = [0, 1, -1, 12345, -54321, i32::MAX, i32::MIN];

        for op in ops {
            for &v in &vals {
                let cp = cf(op.as_ptr() as *const c_char, v);
                let rp = rf(op.as_ptr() as *const c_char, v);
                assert!(!cp.is_null());
                assert!(!rp.is_null());

                // Compare bytes up to NUL (within 64 bytes).
                let mut cs: Vec<u8> = Vec::new();
                let mut rs: Vec<u8> = Vec::new();
                for i in 0..64 {
                    let cb = *(cp.add(i) as *const u8);
                    let rb = *(rp.add(i) as *const u8);
                    if cb == 0 && rb == 0 {
                        break;
                    }
                    cs.push(cb);
                    rs.push(rb);
                }
                assert_eq!(
                    cs, rs,
                    "op={:?} val={} differ\nC={:?}\nR={:?}",
                    String::from_utf8_lossy(op),
                    v,
                    String::from_utf8_lossy(&cs),
                    String::from_utf8_lossy(&rs)
                );
                free(cp as *mut c_void);
                free(rp as *mut c_void);
            }
        }
    }
}

#[test]
fn test_multiply_with_log() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int> =
            c.get(b"multiply_with_log\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int> =
            r.get(b"multiply_with_log\0").unwrap();
        let free = get_libc_free();

        let cases = [(1, 2), (3, 4), (-5, 6), (0, 100), (1000, 1000), (i32::MAX, 2)];
        for (a, b) in cases {
            let mut cmsg: *mut c_char = std::ptr::null_mut();
            let mut rmsg: *mut c_char = std::ptr::null_mut();
            let cv = cf(a, b, &mut cmsg);
            let rv = rf(a, b, &mut rmsg);
            assert_eq!(cv, rv, "multiply_with_log result for ({},{})", a, b);
            assert!(!cmsg.is_null());
            assert!(!rmsg.is_null());

            // Compare strings.
            let mut cs: Vec<u8> = Vec::new();
            let mut rs: Vec<u8> = Vec::new();
            for i in 0..64 {
                let cb = *(cmsg.add(i) as *const u8);
                let rb = *(rmsg.add(i) as *const u8);
                if cb == 0 && rb == 0 {
                    break;
                }
                cs.push(cb);
                rs.push(rb);
            }
            assert_eq!(
                cs, rs,
                "multiply_with_log({},{}) log mismatch\nC={:?}\nR={:?}",
                a, b,
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
            free(cmsg as *mut c_void);
            free(rmsg as *mut c_void);
        }
    }
}

#[test]
fn test_copy_and_sum() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(*mut c_int, c_int) -> c_int> =
            c.get(b"copy_and_sum\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut c_int, c_int) -> c_int> =
            r.get(b"copy_and_sum\0").unwrap();

        let cases: Vec<Vec<c_int>> = vec![
            vec![1, 2, 3],
            vec![0, 0, 0],
            vec![-1, -2, -3],
            vec![i32::MAX, 1, 0],
            vec![100],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ];

        for case in &cases {
            let mut a = case.clone();
            let mut b = case.clone();
            let cv = cf(a.as_mut_ptr(), case.len() as c_int);
            let rv = rf(b.as_mut_ptr(), case.len() as c_int);
            assert_eq!(cv, rv, "copy_and_sum mismatch for {:?}", case);
        }

        // Null pointer case.
        let cv = cf(std::ptr::null_mut(), 3);
        let rv = rf(std::ptr::null_mut(), 3);
        assert_eq!(cv, rv);
    }
}

#[test]
fn test_compare_operations() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int> =
            c.get(b"compare_operations\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int> =
            r.get(b"compare_operations\0").unwrap();

        let pairs: Vec<(&[u8], &[u8])> = vec![
            (b"abc\0", b"abc\0"),
            (b"abc\0", b"abd\0"),
            (b"abd\0", b"abc\0"),
            (b"\0", b"\0"),
            (b"abc\0", b"\0"),
            (b"\0", b"abc\0"),
            (b"long_string\0", b"long_string_more\0"),
        ];
        for (a, b) in &pairs {
            let cv = cf(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
            let rv = rf(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char);
            // strcmp can return any nonzero, but Rust uses libc strcmp too — sign should match.
            assert_eq!(cv.signum(), rv.signum(), "compare {:?} vs {:?}", a, b);
        }

        // NULL cases.
        let some = b"abc\0";
        let null: *const c_char = std::ptr::null();
        let cv = cf(null, some.as_ptr() as *const c_char);
        let rv = rf(null, some.as_ptr() as *const c_char);
        assert_eq!(cv, rv);
        let cv = cf(some.as_ptr() as *const c_char, null);
        let rv = rf(some.as_ptr() as *const c_char, null);
        assert_eq!(cv, rv);
        let cv = cf(null, null);
        let rv = rf(null, null);
        assert_eq!(cv, rv);
    }
}

#[test]
fn test_complexmode() {
    unsafe {
        let (c, r) = open_libs();
        let cf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c.get(b"complexmode\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r.get(b"complexmode\0").unwrap();

        let cases = [
            (1, 1, 2, 3),
            (1, -10, 5, 0),
            (1, i32::MAX, 1, 0),
            (2, 2, 3, 0),
            (2, -2, 5, 0),
            (2, 0, 100, 0),
            (3, 1, 2, 3),
            (3, -1, -2, -3),
            (3, 100, 200, 300),
            (4, 2, 3, 4),
            (4, -1, 5, 10),
            (5, 1, 2, 3),
            (0, 1, 2, 3),
            (-1, 1, 2, 3),
            (99, 0, 0, 0),
        ];
        for (m, a, b, d) in cases {
            let cv = cf(m, a, b, d);
            let rv = rf(m, a, b, d);
            assert_eq!(cv, rv, "complexmode({}, {}, {}, {})", m, a, b, d);
        }
    }
}
