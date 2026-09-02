//! Harness self-checks: prove the tests really load two DIFFERENT `.so` files
//! (the C one and the profile-matching Rust one) and reach them only through
//! their exported symbols.

mod common;

#[test]
fn loads_c_and_matching_profile_rust_so() {
    let (c, r) = common::loaded_paths();
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());

    assert!(c.exists() && r.exists());
    assert_ne!(
        c.canonicalize().unwrap(),
        r.canonicalize().unwrap(),
        "C and Rust must be two distinct shared objects"
    );
    assert!(
        c.to_string_lossy().contains("c_src/build"),
        "C .so must come from c_src/build, got {}",
        c.display()
    );

    // The Rust .so must belong to the SAME profile as this test binary.
    let exe = std::env::current_exe().unwrap();
    let pdir = exe.parent().unwrap().parent().unwrap();
    assert_eq!(
        r.parent().unwrap().canonicalize().unwrap(),
        pdir.canonicalize().unwrap(),
        "loaded Rust .so is from the wrong profile directory"
    );

    // Freshness: the loaded snapshot must be at least as new as the Rust source,
    // otherwise every differential result could be against a stale library.
    let so_m = r.metadata().unwrap().modified().unwrap();
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    assert!(
        src.metadata().unwrap().modified().unwrap() <= so_m,
        "loaded .so is older than src/lib.rs — tests would compare a stale library"
    );

    // It must be a private snapshot, so nothing can replace it mid-test.
    assert!(
        r.file_name().unwrap().to_string_lossy().starts_with("tb-snapshot-"),
        "expected a private snapshot, got {}",
        r.display()
    );
    // Both must resolve `tool_basename` and both pointers must be non-null and distinct.
    let l = common::libs();
    let c_fn = l.c_basename as usize;
    let r_fn = l.rust_basename as usize;
    assert_ne!(c_fn, 0);
    assert_ne!(r_fn, 0);
    assert_ne!(c_fn, r_fn, "both symbols resolved to the same address");
}

/// Sanity: the two libraries agree on a trivial input. If this fails, every
/// other differential test result is meaningless.
#[test]
fn smoke_both_libraries_respond() {
    common::assert_same(b"/usr/bin/x", "smoke");
}
