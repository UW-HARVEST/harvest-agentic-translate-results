use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // CARGO_TARGET_DIR or default target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libhdr_compare_lib.so");
    p
}

struct Libs {
    _c: Library,
    _r: Library,
    c_fn: HdrCompareFn,
    r_fn: HdrCompareFn,
}

fn load_libs() -> Libs {
    unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let r = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        let c_sym: Symbol<HdrCompareFn> =
            c.get(b"hdr_compare\0").expect("missing hdr_compare in C .so");
        let r_sym: Symbol<HdrCompareFn> = r
            .get(b"hdr_compare\0")
            .expect("missing hdr_compare in Rust .so");
        let c_fn: HdrCompareFn = *c_sym;
        let r_fn: HdrCompareFn = *r_sym;
        Libs {
            _c: c,
            _r: r,
            c_fn,
            r_fn,
        }
    }
}

fn check(libs: &Libs, h1: &[u8; 3], h2: &[u8; 3]) {
    unsafe {
        let cv = (libs.c_fn)(h1.as_ptr(), h2.as_ptr());
        let rv = (libs.r_fn)(h1.as_ptr(), h2.as_ptr());
        assert_eq!(
            cv, rv,
            "mismatch for h1={:02x?} h2={:02x?}: C={} R={}",
            h1, h2, cv, rv
        );
    }
}

#[test]
fn exhaustive_h2_first_byte() {
    let libs = load_libs();
    // For each value of h2[0], test some representative h1/h2 values
    for h2_0 in 0u16..=255 {
        let h2_0 = h2_0 as u8;
        for h2_1 in [0x00u8, 0xe2, 0xe3, 0xf0, 0xff, 0xfe, 0x10] {
            for h2_2 in [0x00u8, 0xff, 0xf0, 0x0c, 0x10, 0x40, 0x80, 0xcc] {
                for h1_1 in [0x00u8, 0xe2, 0xfe, 0xff] {
                    for h1_2 in [0x00u8, 0xff, 0x0c, 0x40, 0xf0] {
                        let h1 = [0u8, h1_1, h1_2];
                        let h2 = [h2_0, h2_1, h2_2];
                        check(&libs, &h1, &h2);
                    }
                }
            }
        }
    }
}

#[test]
fn full_h1_h2_when_valid_header() {
    // Cover every (h1[1], h1[2], h2[1], h2[2]) combination but limited; use
    // strategic enumeration over the bits that actually affect the result.
    let libs = load_libs();

    // h2[0] must be 0xff for any chance of hdr_valid. Vary it across a few values.
    for &h2_0 in &[0x00u8, 0xff, 0x80, 0xfe] {
        // Cover every value of h2[1] (256) and h2[2] (256)
        for h2_1 in 0u16..=255 {
            let h2_1 = h2_1 as u8;
            for h2_2 in 0u16..=255 {
                let h2_2 = h2_2 as u8;
                // h1[1], h1[2]: pick values that touch the bits the function inspects
                for &h1_1 in &[0x00u8, 0xff, 0x55, 0xaa, h2_1] {
                    for &h1_2 in &[0x00u8, 0xff, 0x55, 0xaa, 0x0c, 0xf0, h2_2] {
                        let h1 = [0u8, h1_1, h1_2];
                        let h2 = [h2_0, h2_1, h2_2];
                        check(&libs, &h1, &h2);
                    }
                }
            }
        }
    }
}

#[test]
fn fuzz_random() {
    let libs = load_libs();
    // Simple LCG so we don't need an external rand crate
    let mut state: u64 = 0xdeadbeefcafebabe;
    for _ in 0..200_000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bytes = state.to_le_bytes();
        let h1 = [bytes[0], bytes[1], bytes[2]];
        let h2 = [bytes[3], bytes[4], bytes[5]];
        check(&libs, &h1, &h2);
    }
}

#[test]
fn known_valid_header_cases() {
    let libs = load_libs();
    // h2 = [0xff, 0xfb, 0x90]: h2[0]=0xff, (h2[1]&0xF0)==0xf0, version bits ok,
    // bitrate (h2[2]>>4)=9 != 15, sampling (h2[2]>>2 & 3)=0 != 3 -> valid.
    let h2 = [0xffu8, 0xfb, 0x90];
    let h1_eq = [0x00u8, 0xfb, 0x90];
    check(&libs, &h1_eq, &h2);

    // Different h1 values for the same h2
    for h1_1 in 0u16..=255 {
        for h1_2 in 0u16..=255 {
            let h1 = [0u8, h1_1 as u8, h1_2 as u8];
            check(&libs, &h1, &h2);
        }
    }
}
