//! Phase D — symbol parity and profile parity.
//!
//! * every symbol exported by the C `.so` must be exported by the Rust `.so`
//!   under the exact same name, and must be reachable with `dlsym`;
//! * the Rust `.so` must not be a stub: each exported symbol is actually called
//!   and differentially compared against the C one;
//! * the dev-profile and release-profile (`panic = "abort"`) Rust `.so`s must
//!   both export and behave identically (row C23 of `CONFIGS.md`).

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::ffi::c_int;
use std::path::Path;
use std::process::Command;

/// Globally *defined* dynamic symbols, i.e. what the library exports.
fn exported_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("running nm on {so:?} failed: {e}"));
    assert!(
        out.status.success(),
        "nm -D {so:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep only strong, globally visible definitions (T/D/B/R/...);
            // skip weak (V/W/w/v) toolchain-injected ones, which are not part
            // of the library's API and are emitted by the toolchain, not the
            // translated source.
            if matches!(kind, "T" | "D" | "B" | "R" | "G" | "S" | "i") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined symbols the loader must satisfy.
fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(|s| s.to_string()))
        .collect()
}

fn d1_exported_symbol_parity() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    eprintln!("      C exports  ({}): {:?}", c.len(), c);
    eprintln!("      Rust exports({}): {:?}", r.len(), r);
    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so — bad parse?"
    );
    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    // Not required to be equal, but for this library it is: report extras.
    let extra: Vec<_> = r.difference(&c).cloned().collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports symbols the C .so does not: {extra:?}"
    );
    assert!(c.contains("helloworld"), "helloworld not found by nm");
}

fn d2_every_c_symbol_is_dlsym_reachable_in_rust() {
    let c = exported_symbols(&c_so_path());
    let rl = open_lib(&rust_so_path());
    let cl = open_lib(&c_so_path());
    for name in &c {
        let mut key = name.clone().into_bytes();
        key.push(0);
        assert!(
            unsafe { rl.get::<*const ()>(&key) }.is_ok(),
            "dlsym({name}) failed on the Rust .so"
        );
        assert!(
            unsafe { cl.get::<*const ()>(&key) }.is_ok(),
            "dlsym({name}) failed on the C .so"
        );
    }
}

fn d3_no_unresolved_non_libc_symbols() {
    // If any needed symbol were unresolvable, RTLD_NOW would fail outright.
    let cl = open_lib_flags(&c_so_path(), libc::RTLD_NOW | libc::RTLD_LOCAL);
    let rl = open_lib_flags(&rust_so_path(), libc::RTLD_NOW | libc::RTLD_LOCAL);
    assert_ne!(hello_addr_os(&cl), 0);
    assert_ne!(hello_addr_os(&rl), 0);
    let u = undefined_symbols(&rust_so_path());
    let uc = undefined_symbols(&c_so_path());
    eprintln!("      Rust undefined ({}): all resolved by RTLD_NOW", u.len());
    // Both libraries must reach libc stdio for the message. Which entry point
    // is used is a compiler detail with identical observable behaviour: GCC
    // folds `printf("Hello World!\n")` into `puts("Hello World!")`, and so does
    // LLVM at opt-level > 0, while the unoptimized (dev) Rust build keeps the
    // `printf` call. Row D5 proves all three emit the same bytes.
    let stdio = |set: &BTreeSet<String>| {
        set.iter()
            .any(|s| s.starts_with("puts") || s.starts_with("printf") || s.starts_with("fwrite"))
    };
    assert!(stdio(&uc), "the C .so imports no stdio writer: {uc:?}");
    assert!(stdio(&u), "the Rust .so imports no stdio writer: {u:?}");
}

/// Every exported symbol is actually invoked (no stub / `unimplemented!()`),
/// and its result compared against the C library's.
fn d4_every_exported_symbol_is_callable_and_matches() {
    let _g = serial();
    let (c, r) = addrs();
    let names = exported_symbols(&c_so_path());
    assert_eq!(
        names.len(),
        1,
        "this table must be extended if the C library grows symbols: {names:?}"
    );
    // `helloworld`: int helloworld();
    let mut rng = Rng::new(SEED ^ 0xD4);
    for _ in 0..8 {
        let n = rng.range(1, 8) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || {
            (0..n).map(|_| unsafe { call0(c) }).collect::<Vec<c_int>>()
        });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || {
            (0..n).map(|_| unsafe { call0(r) }).collect::<Vec<c_int>>()
        });
        assert_same_bytes("D4 helloworld", &cb, &rb);
        assert_same_rets("D4 helloworld", &cr, &rr);
        assert_eq!(cb, expected(n));
        assert_eq!(cr, vec![0; n]);
    }
}

/// CONFIGS.md row C23: dev-profile and release-profile cdylibs are equivalent.
fn d5_dev_and_release_profiles_agree() {
    let _g = serial();
    let rel = rust_so_release_path();
    assert!(
        rel.exists(),
        "release cdylib missing at {rel:?}; build it with `cargo build --release --offline`"
    );
    // Symbol parity for the release artifact too.
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rel);
    assert_eq!(c, r, "release .so symbol set differs from the C .so");

    let cl = open_lib(&c_so_path());
    let dev = open_lib(&rust_so_path());
    let relib = open_lib(&rel);
    let (ca, da, ra) = (
        hello_addr(&cl),
        hello_addr(&dev),
        hello_addr(&relib),
    );
    let mut rng = Rng::new(SEED ^ 0xD5);
    for _ in 0..8 {
        let n = rng.range(1, 16) as usize;
        let mut streams = Vec::new();
        let mut rets = Vec::new();
        for addr in [ca, da, ra] {
            let (b, v) = capture_file(BufCfg::NoBuf, || {
                (0..n).map(|_| unsafe { call0(addr) }).collect::<Vec<c_int>>()
            });
            streams.push(b);
            rets.push(v);
        }
        assert_same_bytes("D5 dev vs C", &streams[0], &streams[1]);
        assert_same_bytes("D5 release vs C", &streams[0], &streams[2]);
        assert_same_rets("D5 dev vs C", &rets[0], &rets[1]);
        assert_same_rets("D5 release vs C", &rets[0], &rets[2]);
        assert_eq!(streams[0], expected(n));
    }
}

#[test]
fn phase_d_symbol_and_profile_parity() {
    let mut rows = Rows::new("Phase D — symbol / profile parity");
    rows.row("D1 nm -D exported-symbol diff is empty", d1_exported_symbol_parity);
    rows.row("D2 every C symbol is dlsym-reachable in Rust", d2_every_c_symbol_is_dlsym_reachable_in_rust);
    rows.row("D3 RTLD_NOW resolves every undefined symbol", d3_no_unresolved_non_libc_symbols);
    rows.row("D4 every exported symbol is called and matches", d4_every_exported_symbol_is_callable_and_matches);
    rows.row("D5 dev-profile and release-profile cdylibs agree (C23)", d5_dev_and_release_profiles_agree);
    rows.finish();
}
