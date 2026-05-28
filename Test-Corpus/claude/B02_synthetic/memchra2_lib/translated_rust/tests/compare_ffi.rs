use libloading::{Library, Symbol};
use std::os::raw::c_int;

type Memchra2Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const C_SO_PATH: &str = "c_src/build/libtranslated_rust.so";

fn rust_so_path() -> String {
    // Tests run from package root; the .so is in target/<profile>/libmemchra2_lib.so
    // cargo test builds debug; cargo test --release builds release
    let candidates = [
        "target/debug/libmemchra2_lib.so",
        "target/release/libmemchra2_lib.so",
    ];
    for p in &candidates {
        if std::path::Path::new(p).exists() {
            return (*p).to_string();
        }
    }
    candidates[0].to_string()
}

fn load_libs() -> (Library, Library) {
    let c_lib = unsafe { Library::new(C_SO_PATH) }.expect("load C .so");
    let r_lib = unsafe { Library::new(rust_so_path()) }.expect("load Rust .so");
    (c_lib, r_lib)
}

fn call_both(a: c_int, b: c_int, c: c_int, d: c_int) -> (c_int, c_int) {
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<Memchra2Fn> = c_lib.get(b"memchra2").expect("C memchra2");
        let r_fn: Symbol<Memchra2Fn> = r_lib.get(b"memchra2").expect("Rust memchra2");
        (c_fn(a, b, c, d), r_fn(a, b, c, d))
    }
}

#[test]
fn memchra2_zeros() {
    let (c, r) = call_both(0, 0, 0, 0);
    assert_eq!(c, r, "memchra2(0,0,0,0): C={} Rust={}", c, r);
}

#[test]
fn memchra2_ones() {
    let (c, r) = call_both(1, 2, 3, 4);
    assert_eq!(c, r, "memchra2(1,2,3,4): C={} Rust={}", c, r);
}

#[test]
fn memchra2_negative() {
    let (c, r) = call_both(-1, -2, -3, -4);
    assert_eq!(c, r);
}

#[test]
fn memchra2_large_values() {
    let (c, r) = call_both(1000000, 2000000, 3000000, 4000000);
    assert_eq!(c, r);
}

#[test]
fn memchra2_min_max() {
    let (c, r) = call_both(c_int::MIN, c_int::MAX, c_int::MIN, c_int::MAX);
    assert_eq!(c, r);
}

#[test]
fn memchra2_float_range() {
    // pick values where int_to_float_bits(a) yields f in (0,1000)
    // 1.0f32 == 0x3F800000 == 1065353216
    // 999.0f32 == 0x447A0000 == 1148846080
    for &a in &[1065353216, 1148846000, 1117782016] {
        let (c, r) = call_both(a, 5, 6, 7);
        assert_eq!(c, r, "a={}", a);
    }
}

#[test]
fn memchra2_random_battery() {
    // Deterministic pseudo-random samples
    let mut x: u64 = 0x123456789abcdef0;
    for _ in 0..1000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let a = (x >> 32) as u32 as i32;
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let b = (x >> 32) as u32 as i32;
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let c2 = (x >> 32) as u32 as i32;
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let d = (x >> 32) as u32 as i32;
        let (cv, rv) = call_both(a, b, c2, d);
        assert_eq!(cv, rv, "memchra2({},{},{},{}): C={} Rust={}", a, b, c2, d, cv, rv);
    }
}

#[test]
fn exported_symbols_match() {
    // Ensure both .so files export memchra2
    let (c_lib, r_lib) = load_libs();
    unsafe {
        let _: Symbol<Memchra2Fn> = c_lib.get(b"memchra2").expect("C memchra2");
        let _: Symbol<Memchra2Fn> = r_lib.get(b"memchra2").expect("Rust memchra2");
    }
}
