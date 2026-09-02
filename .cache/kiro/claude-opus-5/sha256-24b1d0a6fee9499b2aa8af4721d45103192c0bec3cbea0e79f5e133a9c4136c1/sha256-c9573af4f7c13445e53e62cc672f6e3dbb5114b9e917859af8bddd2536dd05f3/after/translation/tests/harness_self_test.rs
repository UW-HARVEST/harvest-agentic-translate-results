//! Self-tests for the differential harness itself. If any of these fail, the
//! results of `configs.rs` / `errors.rs` are not trustworthy.

mod common;

use common::{drivers, measured_call};
use std::ffi::{c_char, c_void};

extern "C" {
    fn free(p: *mut c_void);
}

/// The whole allocation comparison depends on both `.so`s binding their
/// `calloc` relocation to the interposer in the test executable.
#[test]
fn interposition_is_active() {
    let p = drivers();
    let src = b"abc".as_ptr() as *const c_char;

    unsafe {
        let (a, ia) = measured_call(&p.c, 3, src);
        assert!(!a.is_null());
        free(a as *mut c_void);
        assert_eq!(
            ia.calls, 1,
            "the C .so did NOT bind calloc to the test executable's interposer \
             (recorded {} calls); allocation comparisons would be vacuous",
            ia.calls
        );
        assert_eq!(ia.request, Some((1, 8)), "C calloc(1, 3*4/3+4) == (1, 8)");

        let (b, ib) = measured_call(&p.rs, 3, src);
        assert!(!b.is_null());
        free(b as *mut c_void);
        assert_eq!(
            ib.calls, 1,
            "the Rust .so did NOT bind calloc to the test executable's interposer"
        );
        assert_eq!(ib.request, Some((1, 8)));
    }
}

/// The recording must be immune to other threads' allocator traffic.
#[test]
fn interposition_is_thread_filtered() {
    let p = drivers();
    let noise = std::thread::spawn(|| {
        // Hammer calloc from another thread while the measurement runs.
        extern "C" {
            fn calloc(n: usize, m: usize) -> *mut c_void;
            fn free(p: *mut c_void);
        }
        for i in 0..200_000usize {
            unsafe {
                let q = calloc(1, 1 + (i % 97));
                free(q);
            }
        }
    });
    let src = b"abcdefgh".as_ptr() as *const c_char;
    for _ in 0..2_000 {
        unsafe {
            let (ptr, info) = measured_call(&p.c, 8, src);
            assert!(!ptr.is_null());
            free(ptr as *mut c_void);
            assert_eq!(info.calls, 1, "noise from another thread leaked in");
            assert_eq!(info.request, Some((1, 14)));
        }
    }
    noise.join().unwrap();
}

/// Both `.so`s must be *different* objects — otherwise every test would be
/// comparing one library against itself.
#[test]
fn drivers_are_distinct_objects() {
    let p = drivers();
    let a = p.c.encode_base64 as usize;
    let b = p.rs.encode_base64 as usize;
    assert_ne!(
        a, b,
        "C and Rust encode_base64 resolved to the same address — the same .so \
         was loaded twice"
    );
}

/// Sanity: the harness must actually be able to *detect* a divergence.
/// Encoding a known vector through both must agree with the RFC 4648 answer,
/// so a silently broken comparison would show up here.
#[test]
fn known_answer_vectors() {
    let p = drivers();
    let cases: &[(&[u8], &str)] = &[
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
        (b"\x00", "AA=="),
        (b"\xff\xff\xff", "////"),
        (b"\xfb\xf0", "+/A="),
    ];
    for (input, expect) in cases {
        for d in [&p.c, &p.rs] {
            unsafe {
                let ptr = (d.encode_base64)(input.len() as i32, input.as_ptr() as *const c_char);
                assert!(!ptr.is_null());
                let got = std::ffi::CStr::from_ptr(ptr).to_bytes().to_vec();
                free(ptr as *mut c_void);
                assert_eq!(
                    String::from_utf8_lossy(&got),
                    *expect,
                    "{} mis-encoded {:?}",
                    d.name,
                    String::from_utf8_lossy(input)
                );
            }
        }
    }
}
