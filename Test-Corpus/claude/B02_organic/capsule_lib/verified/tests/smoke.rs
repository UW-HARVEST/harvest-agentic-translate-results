//! Harness smoke test + Phase D symbol-parity check performed at runtime.

mod common;
use common::*;

#[test]
fn harness_loads_both_libraries() {
    let (c, r) = load_pair();
    eprintln!("C   .so: {}", c.path.display());
    eprintln!("RUST.so: {}", r.path.display());
    assert_eq!(BOUND_SYMBOLS.len(), 38, "harness must bind all 38 symbols");
}

/// Phase D: every symbol `nm -D` reports for the C `.so` must also resolve in
/// the Rust `.so`. Done here through `dlsym` so it is enforced by the test run
/// itself, not only by an offline `nm` diff.
#[test]
fn symbol_parity_via_dlsym() {
    let cpath = c_so_path();
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", cpath.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed");
    let text = String::from_utf8_lossy(&out.stdout);
    let c_syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            if kind == "T" { Some(name.to_string()) } else { None }
        })
        .collect();
    assert!(
        c_syms.len() >= 38,
        "expected >= 38 exported C symbols, got {}",
        c_syms.len()
    );

    let rlib = unsafe {
        libloading::os::unix::Library::open(
            Some(rust_so_path()),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
        )
    }
    .expect("dlopen rust .so with RTLD_NOW");

    let mut missing = Vec::new();
    for s in &c_syms {
        let mut name = s.clone().into_bytes();
        name.push(0);
        let got = unsafe { rlib.get::<*const std::ffi::c_void>(&name) };
        if got.is_err() {
            missing.push(s.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // And the harness binds every one of them.
    let mut unbound: Vec<&String> = c_syms
        .iter()
        .filter(|s| !BOUND_SYMBOLS.contains(&s.as_str()))
        .collect();
    unbound.sort();
    assert!(
        unbound.is_empty(),
        "C symbols not exercised by the differential harness: {unbound:?}"
    );
    eprintln!("symbol parity: {} / {} symbols OK", c_syms.len(), c_syms.len());
}

#[test]
fn capsule_entry_point_smoke() {
    let (c, r) = load_pair();
    let mut d = Diff::new("smoke/capsule");
    for &(a, b, cc, dd, e) in &[
        (-40.0f32, -40.0f32, -20.0f32, 100.0f32, 10.0f32),
        (0.0, 0.0, 0.0, 0.0, 0.0),
        (-70.0, 0.0, -70.0, 0.0, 5.0),
        (-100.0, -100.0, 100.0, 100.0, 1.0),
    ] {
        d.int(
            &format!("capsule({a},{b},{cc},{dd},{e})"),
            (c.capsule)(a, b, cc, dd, e),
            (r.capsule)(a, b, cc, dd, e),
        );
    }
    d.finish();
}
