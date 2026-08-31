//! `gethex()` — the hex-float parser from dtoa.c.
//!
//! Signature: `void gethex(const char **sp, U *rvp, int rounding, int sign)`
//! where `U` is a `union { double d; ULong L[2]; }`. `*sp` must point at the
//! `0x`/`0X` prefix (gethex skips two characters unconditionally).

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void};

type FnGethexReal =
    unsafe extern "C" fn(*mut *const c_char, *mut c_void, c_int, c_int);

fn hex_inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    let lits: &[&str] = &[
        "0x0",
        "0x1",
        "0x2",
        "0xf",
        "0xF",
        "0x10",
        "0xff",
        "0xFFFF",
        "0x1p0",
        "0x1p1",
        "0x1p-1",
        "0x1p+1",
        "0x1P10",
        "0x1p1000",
        "0x1p-1000",
        "0x1p1023",
        "0x1p1024",
        "0x1p-1022",
        "0x1p-1074",
        "0x1p-1075",
        "0x1p99999999999999999999",
        "0x1p-99999999999999999999",
        "0x1.8p1",
        "0x1.8p0",
        "0x.8p0",
        "0x.8",
        "0x8.",
        "0x.",
        "0x",
        "0x.0",
        "0x0.0",
        "0x0p0",
        "0x0000",
        "0x0000.0000",
        "0x0.0000000000000000001",
        "0x1.fffffffffffffp1023",
        "0x1.fffffffffffff8p1023",
        "0x1.0000000000001p0",
        "0x1.00000000000008p0",
        "0x1.00000000000018p0",
        "0x1.0000000000000fp0",
        "0x1.8000000000000p0",
        "0x1.7ffffffffffffp0",
        "0x10000000000000000",
        "0xffffffffffffffffffffffff",
        "0x123456789abcdef0123456789abcdef",
        "0x1.921fb54442d18p1",
        "0x1p-1023",
        "0x1p-1024",
        "0x1.8p-1074",
        "0x1.8p-1075",
        "0x3p-1075",
        "0x1p-1076",
        "0x0.0000000000001p-1022",
        "0xdeadbeef",
        "0xDEADBEEFp-8",
        "0x1abcp",
        "0x1abcp+",
        "0x1abcp-",
        "0x1abcpz",
        "0x1abcq",
        "0xzz",
        "0x1.2.3",
        "0x..1",
        "0x1p1p1",
        "0x1.p1",
        "0x1..p1",
        "0x000000000000000000001p0",
        "0x0.00000000000000000001p80",
    ];
    for s in lits {
        v.push(s.as_bytes().to_vec());
    }
    // long digit strings across several Bigint words
    for n in [1usize, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100] {
        v.push(format!("0x{}", "f".repeat(n)).into_bytes());
        v.push(format!("0x{}p-4", "1".repeat(n)).into_bytes());
        v.push(format!("0x0.{}1", "0".repeat(n)).into_bytes());
        v.push(format!("0x1.{}p0", "0".repeat(n)).into_bytes());
    }
    // pseudo-random hex literals
    let mut s: u64 = 0x0123_4567_89ab_cdef;
    for _ in 0..1500 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mant = s >> 8;
        let e = ((s >> 3) & 0x7ff) as i64 - 1100;
        v.push(format!("0x{mant:x}p{e}").into_bytes());
        v.push(format!("0x{:x}.{:x}p{}", s & 0xffff, mant, e).into_bytes());
    }
    v
}

#[test]
fn gethex_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnGethexReal> = c.sym("gethex");
    let fr: Symbol<FnGethexReal> = r.sym("gethex");
    extern "C" {
        fn __errno_location() -> *mut c_int;
    }

    for input in hex_inputs() {
        let z = std::ffi::CString::new(input.clone()).unwrap();
        for rounding in [0i32, 1, 2, 3] {
            for sign in [0i32, 1] {
                unsafe {
                    // rvp is a union { double; ULong[2] }: 8 bytes, but gethex
                    // may leave one half untouched, so pre-fill identically.
                    let mut ua: u64 = 0xdead_beef_feed_face;
                    let mut ub: u64 = 0xdead_beef_feed_face;
                    let mut pa: *const c_char = z.as_ptr();
                    let mut pb: *const c_char = z.as_ptr();
                    *__errno_location() = 0;
                    fc(&mut pa, &mut ua as *mut u64 as *mut c_void, rounding, sign);
                    let ea = *__errno_location();
                    *__errno_location() = 0;
                    fr(&mut pb, &mut ub as *mut u64 as *mut c_void, rounding, sign);
                    let eb = *__errno_location();

                    let oa = pa.offset_from(z.as_ptr());
                    let ob = pb.offset_from(z.as_ptr());
                    assert_eq!(
                        (ua, oa, ea),
                        (ub, ob, eb),
                        "gethex({:?}, rounding {rounding}, sign {sign})\n  C   bits {:#018x} d={:e} end +{} errno {}\n  Rust bits {:#018x} d={:e} end +{} errno {}",
                        String::from_utf8_lossy(&input),
                        ua,
                        f64::from_bits(ua),
                        oa,
                        ea,
                        ub,
                        f64::from_bits(ub),
                        ob,
                        eb
                    );
                }
            }
        }
    }
}
