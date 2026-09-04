//! Phase B — CONFIGS.md rows 141, 154, 155, 156, 159.
//!
//! Every zero-argument accessor exported by the C `.so` must return the exact
//! same value from the Rust `.so`. This is 371 symbols across five C return
//! types; the name lists in `common/accessors.rs` are generated mechanically
//! from the public headers.

mod common;
use common::accessors;
use common::*;

#[test]
fn size_t_accessors_match() {
    let d = duo();
    assert_eq!(accessors::SIZE.len(), 303);
    for n in accessors::SIZE {
        let (cf, rf) = d.pair::<unsafe extern "C" fn() -> usize>(n);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{n}(): C={c} Rust={r}");
    }
}

#[test]
fn unsigned_long_long_accessors_match() {
    let d = duo();
    assert_eq!(accessors::ULL.len(), 19);
    for n in accessors::ULL {
        let (cf, rf) = d.pair::<unsafe extern "C" fn() -> u64>(n);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{n}(): C={c} Rust={r}");
    }
}

#[test]
fn int_accessors_match() {
    let d = duo();
    assert_eq!(accessors::INT.len(), 21);
    for n in accessors::INT {
        let (cf, rf) = d.pair::<unsafe extern "C" fn() -> i32>(n);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{n}(): C={c} Rust={r}");
    }
}

#[test]
fn unsigned_char_accessors_match() {
    let d = duo();
    assert_eq!(accessors::UCHAR.len(), 8);
    for n in accessors::UCHAR {
        let (cf, rf) = d.pair::<unsafe extern "C" fn() -> u8>(n);
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{n}(): C={c} Rust={r}");
    }
}

#[test]
fn string_accessors_match() {
    let d = duo();
    assert_eq!(accessors::STR.len(), 20);
    for n in accessors::STR {
        let (cf, rf) = d.pair::<unsafe extern "C" fn() -> *const libc::c_char>(n);
        let (c, r) = unsafe {
            (
                std::ffi::CStr::from_ptr(cf()).to_owned(),
                std::ffi::CStr::from_ptr(rf()).to_owned(),
            )
        };
        assert_eq!(c, r, "{n}()");
    }
}

/// CONFIGS.md row 153 — `sodium_init` is idempotent and returns the same value.
#[test]
fn sodium_init_idempotent() {
    let d = duo();
    let (cf, rf) = d.pair::<unsafe extern "C" fn() -> i32>("sodium_init");
    for _ in 0..3 {
        unsafe { eq_i32("sodium_init", cf(), rf()) };
    }
}

/// Symbol parity, asserted from inside the test suite as well as from the shell:
/// every symbol the C `.so` exports must resolve in the Rust `.so`.
#[test]
fn symbol_parity() {
    let d = duo();
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../c_src/build/libsodium.so"
        ))
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .collect();
    assert!(
        names.len() >= 890,
        "expected >=890 C symbols, got {}",
        names.len()
    );
    let missing: Vec<&&str> = names.iter().filter(|n| !d.has(n)).collect();
    assert!(
        missing.is_empty(),
        "{} symbols missing from Rust .so: {:?}",
        missing.len(),
        missing
    );
}
