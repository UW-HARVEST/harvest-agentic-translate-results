//! Phase B: differential tests for pure serialization/inspection functions.
//! Every call goes through BOTH the C `.so` and the Rust `.so` via libloading.

mod common;
use common::{Libs, Rng};

type FnGetU32 = unsafe extern "C" fn(*const u8) -> u32;
type FnGetU16 = unsafe extern "C" fn(*const u8) -> u16;
type FnGetI32 = unsafe extern "C" fn(*const u8) -> i32;
type FnSaveU32 = unsafe extern "C" fn(*mut u8, u32);
type FnSaveI32 = unsafe extern "C" fn(*mut u8, i32);
type FnSaveU16 = unsafe extern "C" fn(*mut u8, std::os::raw::c_uint);
type FnSigCmp = unsafe extern "C" fn(*const u8, usize, usize) -> std::os::raw::c_int;
type FnAccessVer = unsafe extern "C" fn() -> u32;

// CONFIGS row 1
#[test]
fn get_uint_32_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnGetU32> = libs.c.get(b"png_get_uint_32").unwrap();
        let r: libloading::Symbol<FnGetU32> = libs.rust.get(b"png_get_uint_32").unwrap();
        let mut rng = Rng::new(1);
        // boundary values
        let mut cases: Vec<[u8; 4]> = vec![
            [0, 0, 0, 0],
            [0xff, 0xff, 0xff, 0xff],
            [0x7f, 0xff, 0xff, 0xff],
            [0x80, 0, 0, 0],
            [0, 0, 0, 1],
            [1, 0, 0, 0],
        ];
        for _ in 0..5000 {
            cases.push(rng.next_u32().to_be_bytes());
        }
        for b in &cases {
            assert_eq!(c(b.as_ptr()), r(b.as_ptr()), "get_uint_32 {:?}", b);
        }
    }
}

// CONFIGS row 2
#[test]
fn get_uint_16_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnGetU16> = libs.c.get(b"png_get_uint_16").unwrap();
        let r: libloading::Symbol<FnGetU16> = libs.rust.get(b"png_get_uint_16").unwrap();
        let mut rng = Rng::new(2);
        let mut cases: Vec<[u8; 2]> = vec![[0, 0], [0xff, 0xff], [0x80, 0], [0, 1]];
        for _ in 0..5000 {
            cases.push(rng.next_u16().to_be_bytes());
        }
        for b in &cases {
            assert_eq!(c(b.as_ptr()), r(b.as_ptr()), "get_uint_16 {:?}", b);
        }
    }
}

// CONFIGS row 3
#[test]
fn get_int_32_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnGetI32> = libs.c.get(b"png_get_int_32").unwrap();
        let r: libloading::Symbol<FnGetI32> = libs.rust.get(b"png_get_int_32").unwrap();
        let mut rng = Rng::new(3);
        let mut cases: Vec<[u8; 4]> = vec![
            [0, 0, 0, 0],
            [0xff, 0xff, 0xff, 0xff],
            [0x80, 0, 0, 0], // sign bit set
            [0x7f, 0xff, 0xff, 0xff],
            [0x80, 0, 0, 1],
        ];
        for _ in 0..5000 {
            cases.push(rng.next_u32().to_be_bytes());
        }
        for b in &cases {
            assert_eq!(c(b.as_ptr()), r(b.as_ptr()), "get_int_32 {:?}", b);
        }
    }
}

// CONFIGS row 5
#[test]
fn save_uint_32_roundtrip_matches() {
    let libs = Libs::load();
    unsafe {
        let cs: libloading::Symbol<FnSaveU32> = libs.c.get(b"png_save_uint_32").unwrap();
        let rs: libloading::Symbol<FnSaveU32> = libs.rust.get(b"png_save_uint_32").unwrap();
        let mut rng = Rng::new(5);
        let mut vals: Vec<u32> = vec![0, 1, 0x7fffffff, 0x80000000, 0xffffffff];
        for _ in 0..5000 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            cs(cb.as_mut_ptr(), v);
            rs(rb.as_mut_ptr(), v);
            assert_eq!(cb, rb, "save_uint_32 {v}");
        }
    }
}

// CONFIGS row 6
#[test]
fn save_int_32_matches() {
    let libs = Libs::load();
    unsafe {
        let cs: libloading::Symbol<FnSaveI32> = libs.c.get(b"png_save_int_32").unwrap();
        let rs: libloading::Symbol<FnSaveI32> = libs.rust.get(b"png_save_int_32").unwrap();
        let mut rng = Rng::new(6);
        let mut vals: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, -2147483647];
        for _ in 0..5000 {
            vals.push(rng.next_u32() as i32);
        }
        for v in vals {
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            cs(cb.as_mut_ptr(), v);
            rs(rb.as_mut_ptr(), v);
            assert_eq!(cb, rb, "save_int_32 {v}");
        }
    }
}

// CONFIGS row 7
#[test]
fn save_uint_16_matches() {
    let libs = Libs::load();
    unsafe {
        let cs: libloading::Symbol<FnSaveU16> = libs.c.get(b"png_save_uint_16").unwrap();
        let rs: libloading::Symbol<FnSaveU16> = libs.rust.get(b"png_save_uint_16").unwrap();
        let mut rng = Rng::new(7);
        let mut vals: Vec<u32> = vec![0, 1, 0x7fff, 0x8000, 0xffff];
        for _ in 0..5000 {
            vals.push(rng.next_u16() as u32);
        }
        for v in vals {
            let mut cb = [0u8; 2];
            let mut rb = [0u8; 2];
            cs(cb.as_mut_ptr(), v as std::os::raw::c_uint);
            rs(rb.as_mut_ptr(), v as std::os::raw::c_uint);
            assert_eq!(cb, rb, "save_uint_16 {v}");
        }
    }
}

// CONFIGS row 8 + ERRORS row 14
#[test]
fn sig_cmp_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnSigCmp> = libs.c.get(b"png_sig_cmp").unwrap();
        let r: libloading::Symbol<FnSigCmp> = libs.rust.get(b"png_sig_cmp").unwrap();
        let good: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
        let mut rng = Rng::new(8);
        // valid signature, all valid (start, num) sub-ranges
        for start in 0..8usize {
            for num in 0..=(8 - start) {
                assert_eq!(
                    c(good.as_ptr(), start, num),
                    r(good.as_ptr(), start, num),
                    "sig_cmp good start={start} num={num}"
                );
            }
        }
        // random buffers (mostly invalid)
        for _ in 0..3000 {
            let mut buf = [0u8; 8];
            for b in buf.iter_mut() {
                *b = rng.next_u8();
            }
            let start = (rng.next_u32() % 8) as usize;
            let num = (rng.next_u32() % (8 - start + 1) as u32) as usize;
            let cv = c(buf.as_ptr(), start, num);
            let rv = r(buf.as_ptr(), start, num);
            // libpng returns the difference; compare exact value.
            assert_eq!(cv, rv, "sig_cmp rand {:?} start={start} num={num}", buf);
        }
    }
}

// CONFIGS row 21: png_convert_to_rfc1123_buffer
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PngTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}
type FnRfc1123 = unsafe extern "C" fn(*mut u8, *const PngTime) -> std::os::raw::c_int;

#[test]
fn convert_to_rfc1123_buffer_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnRfc1123> =
            libs.c.get(b"png_convert_to_rfc1123_buffer").unwrap();
        let r: libloading::Symbol<FnRfc1123> =
            libs.rust.get(b"png_convert_to_rfc1123_buffer").unwrap();
        let mut rng = Rng::new(21);
        // Mix of valid and out-of-range values (C validates ranges).
        let mut times: Vec<PngTime> = vec![
            PngTime { year: 1995, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
            PngTime { year: 2026, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
            PngTime { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 },
            PngTime { year: 65535, month: 13, day: 32, hour: 24, minute: 60, second: 61 },
        ];
        for _ in 0..2000 {
            times.push(PngTime {
                year: rng.next_u16(),
                month: rng.next_u8(),
                day: rng.next_u8(),
                hour: rng.next_u8(),
                minute: rng.next_u8(),
                second: rng.next_u8(),
            });
        }
        for t in &times {
            let mut cb = [0u8; 29];
            let mut rb = [0u8; 29];
            let cr = c(cb.as_mut_ptr(), t as *const PngTime);
            let rr = r(rb.as_mut_ptr(), t as *const PngTime);
            assert_eq!(cr, rr, "rfc1123 return differs for {:?}", (t.year, t.month, t.day, t.hour, t.minute, t.second));
            // On success (ret==1) buffers must match; on failure C leaves buffer
            // in an implementation-defined state, so only compare when ok.
            if cr == 1 {
                assert_eq!(cb, rb, "rfc1123 buffer differs");
            }
        }
        // NOTE: the C code dereferences `ptime` without a NULL check (it only
        // guards `out == NULL`), so a NULL ptime is UB in C itself and is not a
        // valid differential input. We instead cover the `out == NULL` guard.
        let cr = c(std::ptr::null_mut(), &times[0] as *const PngTime);
        let rr = r(std::ptr::null_mut(), &times[0] as *const PngTime);
        assert_eq!(cr, rr, "rfc1123 null out return differs");
        assert_eq!(cr, 0, "rfc1123 null out should return 0");
    }
}

// CONFIGS row 9
#[test]
fn access_version_matches() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FnAccessVer> = libs.c.get(b"png_access_version_number").unwrap();
        let r: libloading::Symbol<FnAccessVer> = libs.rust.get(b"png_access_version_number").unwrap();
        assert_eq!(c(), r(), "access_version_number");
    }
}
