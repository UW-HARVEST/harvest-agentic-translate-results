// Phase D — symbol parity + negative controls.
//
// The negative controls matter as much as the positive tests: they prove the
// differential harness can actually SEE a divergence, so that the 60 passing tests
// in phases B and C are evidence rather than vacuous.

mod common;

use common::*;
use std::os::raw::c_int;
use std::process::Command;

fn so_paths() -> (String, String) {
    let md = env!("CARGO_MANIFEST_DIR");
    let c = std::env::var("C_SO").unwrap_or_else(|_| {
        let dir = format!("{md}/../c_src/build");
        std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {dir}: {e}"))
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
            .expect("no C .so; build c_src first")
            .to_string_lossy()
            .into_owned()
    });
    let r = std::env::var("RUST_SO").unwrap_or_else(|_| {
        let order: [&str; 2] = if cfg!(debug_assertions) {
            ["debug", "release"]
        } else {
            ["release", "debug"]
        };
        for prof in order {
            let p = format!("{md}/target/{prof}/libarrayfunc_lib.so");
            if std::path::Path::new(&p).exists() {
                return p;
            }
        }
        panic!("no Rust cdylib; run `cargo build --release`");
    });
    (c, r)
}

fn nm_defined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path])
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {path}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Exported code/data only; skip the toolchain's own bookkeeping.
            if kind == "T" || kind == "t" {
                Some(name.to_string())
            } else {
                let _ = kind;
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn nm_undefined(path: &str) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", path])
        .output()
        .expect("failed to run nm");
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The Phase D completion gate: the symbol diff must be EMPTY.
#[test]
fn d01_every_c_symbol_is_exported_by_rust() {
    let (c_so, r_so) = so_paths();
    let c_syms = nm_defined(&c_so);
    let r_syms = nm_defined(&r_so);

    // Sanity: we really did parse the C library.
    assert_eq!(
        c_syms.len(),
        11,
        "expected the 11 non-static functions of c_src/src/lib.c, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {missing:?}\n  C:    {c_syms:?}\n  Rust: {r_syms:?}",
        missing.len()
    );

    // And they must be reachable through dlsym with the exact same names, which
    // `pair()` already proves by resolving all 11 in both libraries.
    let _ = pair();
}

#[test]
fn d02_expected_symbol_list_is_exactly_the_c_surface() {
    let (c_so, _) = so_paths();
    let mut expected = vec![
        "add_operation",
        "arrayfunc",
        "compare_results_in_array",
        "compute_scaled_value",
        "compute_weighted_sum",
        "init_result_array",
        "modulo_operation",
        "multiply_operation",
        "process_with_foreach",
        "safe_double_to_int",
        "subtract_operation",
    ];
    expected.sort();
    assert_eq!(
        nm_defined(&c_so),
        expected,
        "the C symbol surface changed; SYMBOLS.md must be regenerated"
    );
}

#[test]
fn d03_rust_so_has_no_unresolved_non_libc_symbols() {
    let (_, r_so) = so_paths();
    let und = nm_undefined(&r_so);
    // Anything the Rust .so imports must come from libc / the runtime, never from
    // an untranslated C module of this project.
    let c_surface = nm_defined(&so_paths().0);
    for s in &und {
        assert!(
            !c_surface.contains(s),
            "Rust .so imports `{s}` from the C library instead of implementing it \
             — that module was not translated"
        );
    }
    // Every remaining undefined symbol must be satisfiable by the platform. Rather
    // than maintaining a hand-written libc allowlist (which cannot be complete),
    // assert the property that actually matters: the library loads with RTLD_NOW,
    // which forces the loader to bind EVERY undefined symbol immediately. If any
    // reference were dangling — e.g. a call into an untranslated C module — this
    // dlopen would fail.
    let flags = libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL;
    let lib = unsafe { libloading::os::unix::Library::open(Some(&r_so), flags) };
    let lib = lib.unwrap_or_else(|e| {
        panic!("Rust .so has unresolvable symbols (RTLD_NOW dlopen failed): {e}\n  undefined: {und:?}")
    });

    // While it is open, resolve all 11 exports by name to prove the parity list is
    // dlsym-reachable, not merely present in the symbol table.
    for name in nm_defined(&so_paths().0) {
        let cname = std::ffi::CString::new(name.clone()).unwrap();
        let r: Result<libloading::os::unix::Symbol<*const ()>, _> =
            unsafe { lib.get(cname.as_bytes_with_nul()) };
        assert!(r.is_ok(), "dlsym(`{name}`) failed on the Rust .so");
    }
    drop(lib);

    // Guard against a silently empty undefined set (which would make the above
    // vacuous): a real Rust cdylib always imports at least memcpy/malloc-class
    // symbols from glibc.
    assert!(
        und.len() > 5,
        "suspiciously small undefined set ({und:?}) — nm may have failed"
    );
}

// ===========================================================================
// Negative controls — prove the harness detects divergence
// ===========================================================================

#[test]
fn d04_negative_control_byte_compare_detects_a_single_bit() {
    let mut rng = Rng::seeded();
    let a = ArrBuf::poisoned(&mut rng);
    let mut b = a.clone();
    // flip one bit in a padding hole — the strictest place a real bug could hide
    b.bytes[4] ^= 0x01;
    let r = std::panic::catch_unwind(|| assert_bufs_eq("neg-control", &a, &b));
    assert!(
        r.is_err(),
        "assert_bufs_eq FAILED TO DETECT a one-bit difference — the whole Phase B \
         memory comparison would be vacuous"
    );

    // ...and it must accept identical buffers
    let c = a.clone();
    assert_bufs_eq("neg-control identical", &a, &c);
}

#[test]
fn d05_negative_control_wrong_function_is_caught() {
    let p = pair();
    // Compare C's add against C's subtract through the same code path the real
    // tests use. It MUST be reported as a divergence.
    let mut found = false;
    for a in 1..50i32 {
        for b in 1..50i32 {
            let x = unsafe { (p.c.add_operation)(a, b, 0, 0) };
            let y = unsafe { (p.rs.subtract_operation)(a, b, 0, 0) };
            if x != y {
                found = true;
            }
        }
    }
    assert!(found, "harness cannot distinguish add from subtract");
}

#[test]
fn d06_negative_control_process_detects_wrong_op() {
    let p = pair();
    let mut rng = Rng::seeded();
    // Driving C with `add` and Rust with `multiply` must produce a visible diff in
    // both the return value and the mutated memory.
    let vals: Vec<c_int> = (0..16).map(|_| (rng.next_u32() % 1000) as c_int + 7).collect();
    let start = ArrBuf::zeroed();
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut cv = vals.clone();
    let mut rv = vals.clone();
    let (ct, rt) = unsafe {
        (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), 10);
        (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), 10);
        (
            (p.c.process_with_foreach)(cb.as_ptr(), Some(p.c.add_operation)),
            (p.rs.process_with_foreach)(rb.as_ptr(), Some(p.rs.multiply_operation)),
        )
    };
    assert_ne!(ct, rt, "return-value comparison is vacuous");
    let r = std::panic::catch_unwind(move || assert_bufs_eq("neg-control ops", &cb, &rb));
    assert!(r.is_err(), "memory comparison is vacuous");
}

#[test]
fn d07_negative_control_arrayfunc_is_input_sensitive() {
    let p = pair();
    // If `arrayfunc` returned a constant, every test above would pass trivially.
    // Confirm it actually depends on all four parameters.
    let base = unsafe { (p.c.arrayfunc)(3, 5, 7, 11) };
    let mut distinct = std::collections::BTreeSet::new();
    for i in 0..4 {
        let mut args = [3, 5, 7, 11];
        args[i] = 1000 + i as c_int;
        let v = unsafe { (p.c.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_ne!(
            v, base,
            "arrayfunc ignores param{}; the differential tests would be weak",
            i + 1
        );
        let rv = unsafe { (p.rs.arrayfunc)(args[0], args[1], args[2], args[3]) };
        assert_eq!(v, rv);
        distinct.insert(v);
    }
    assert_eq!(distinct.len(), 4, "arrayfunc outputs should all differ");

    // A broad sample must produce many distinct outputs, not a saturated constant.
    let mut rng = Rng::new(0xABCD_1234_5678_9EF0);
    let mut outs = std::collections::BTreeSet::new();
    for _ in 0..5_000 {
        let (a, b, c, d) = (
            (rng.next_u32() % 2000) as c_int - 1000,
            (rng.next_u32() % 2000) as c_int - 1000,
            (rng.next_u32() % 2000) as c_int - 1000,
            (rng.next_u32() % 2000) as c_int - 1000,
        );
        let v = unsafe { (p.c.arrayfunc)(a, b, c, d) };
        assert_eq!(v, unsafe { (p.rs.arrayfunc)(a, b, c, d) });
        outs.insert(v);
    }
    assert!(
        outs.len() > 1000,
        "arrayfunc produced only {} distinct outputs over 5000 varied inputs — \
         the tests may be saturating instead of exercising real arithmetic",
        outs.len()
    );
}

#[test]
fn d08_negative_control_safe_double_to_int_is_not_constant() {
    let p = pair();
    let mut outs = std::collections::BTreeSet::new();
    let mut rng = Rng::seeded();
    for _ in 0..10_000 {
        let d = (rng.next_u32() as f64) - 2147483648.0 / 2.0;
        let v = unsafe { (p.c.safe_double_to_int)(d) };
        assert_eq!(v, unsafe { (p.rs.safe_double_to_int)(d) });
        outs.insert(v);
    }
    assert!(outs.len() > 5_000, "safe_double_to_int looks saturated: {} distinct", outs.len());
}
