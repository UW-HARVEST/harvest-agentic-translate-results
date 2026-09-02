//! Harness-integrity tests.
//!
//! Everything else in this suite rests on one assumption: that `call_both`
//! really drives TWO distinct implementations. If `dlopen` had deduplicated the
//! two `libdriver.so` files (they share a filename), or if the Rust `.so`'s
//! `driver` were resolving to the C `.so`'s `run` through symbol interposition,
//! then every differential assertion would pass vacuously by comparing one
//! library against itself.
//!
//! These tests prove that is not happening, by driving each library on its own
//! and observing that their persistent `static house_t the_house` states move
//! independently. That deliberately desynchronises the two states, which is why
//! this lives in its own test binary — no other test shares the process.

mod harness;

use harness::{harness, Entry, Harness, Which};

#[test]
fn integrity_01_libraries_have_independent_state() {
    let mut h = harness();

    // The harness constructor already performed exactly one `run(0)` on each
    // library, so both are at floors=3, bedrooms=5, bathrooms=3.5 and their
    // pristine captures were identical.
    let (c0, r0) = h.pristine_run0();
    assert_eq!(c0, r0, "pristine captures differ");
    let (f0, b0, ba0) = Harness::parse_last_state(c0);
    assert_eq!((f0, b0, ba0.as_str()), (3, 5, "3.5"));

    // Advance ONLY the C library, three times.
    let mut c_last = Vec::new();
    for _ in 0..3 {
        c_last = h.call_one(Which::C, Entry::Run, 0);
    }
    let (c_floors, _, c_bath) = Harness::parse_last_state(&c_last);
    assert_eq!(c_floors, 6, "C floors should have advanced 3 -> 6");
    assert_eq!(c_bath, "6.5");

    // The Rust library must be untouched by that: still at floors=3.
    let r_next = h.call_one(Which::Rust, Entry::Run, 0);
    let (r_floors, _, r_bath) = Harness::parse_last_state(&r_next);
    assert_eq!(
        r_floors, 4,
        "Rust state was advanced by calls made to the C library — the two .so files \
         are not independent, so every differential comparison would be vacuous"
    );
    assert_eq!(r_bath, "4.5");
    assert_ne!(
        c_last, r_next,
        "the two libraries produced identical output from deliberately different \
         states; dlopen probably returned the same handle twice"
    );

    // Re-synchronise so the invariant `call_both` relies on holds again, in case
    // more tests are ever added to this binary.
    for _ in 0..2 {
        let _ = h.call_one(Which::Rust, Entry::Run, 0);
    }
    let c_sync = h.call_one(Which::C, Entry::Run, 0);
    let r_sync = h.call_one(Which::Rust, Entry::Run, 0);
    assert_eq!(
        c_sync,
        r_sync,
        "failed to re-synchronise the two states\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_sync),
        String::from_utf8_lossy(&r_sync)
    );
}

/// The two loaded shared objects must be different files on disk, and both must
/// exist. Guards against `RUST_DRIVER_SO`/`C_DRIVER_SO` being pointed at the
/// same artifact.
#[test]
fn integrity_02_two_distinct_shared_objects_on_disk() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let c = std::env::var("C_DRIVER_SO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| root.join("c_src/build/libdriver.so"));
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let r = std::env::var("RUST_DRIVER_SO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let rel = manifest.join("target/release/libdriver.so");
            if rel.exists() {
                rel
            } else {
                manifest.join("target/debug/libdriver.so")
            }
        });

    assert!(c.exists(), "{} missing", c.display());
    assert!(r.exists(), "{} missing", r.display());
    let c_real = c.canonicalize().unwrap();
    let r_real = r.canonicalize().unwrap();
    assert_ne!(
        c_real,
        r_real,
        "C and Rust .so resolve to the same file: {}",
        c_real.display()
    );

    let c_bytes = std::fs::read(&c_real).unwrap();
    let r_bytes = std::fs::read(&r_real).unwrap();
    assert_ne!(c_bytes, r_bytes, "the two .so files have identical contents");
}
