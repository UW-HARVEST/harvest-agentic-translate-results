//! Phase D — symbol parity and cross-configuration parity, as executable tests.

mod harness;
use harness::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn defined_dynamic_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .collect()
}

/// Every symbol exported by the C `.so` must be exported by the Rust `.so`
/// under the exact same name.
#[test]
fn symbols_c_subset_of_rust() {
    let c = defined_dynamic_symbols(c_so());
    let r = defined_dynamic_symbols(rust_so());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n  C   : {c:?}\n  Rust: {r:?}"
    );
    // the C library really does export something (guards against an empty diff
    // caused by a broken nm invocation)
    assert!(c.contains("driver"), "C .so does not export `driver`: {c:?}");
    assert!(c.contains("main"), "C .so does not export `main`: {c:?}");
}

/// The Rust `.so` must not have unresolved non-libc symbols.
#[test]
fn rust_so_has_no_undefined_symbols() {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let undef: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol"))
        .collect();
    assert!(undef.is_empty(), "undefined symbols in the Rust .so: {undef:?}");
}

/// Both `driver` and `main` must be callable through `dlsym` (this is what the
/// rest of the suite relies on, asserted explicitly here).
#[test]
fn both_symbols_are_callable_via_dlsym() {
    let cases = vec![(b"abcdef".to_vec(), b"cd".to_vec())];
    let c = run_driver_batch(c_so(), "d_call.c", &cases);
    let r = run_driver_batch(rust_so(), "d_call.rs", &cases);
    assert_eq!(c.stdout, b"2\n", "unexpected C result: {c:?}");
    assert_eq!(r.stdout, b"2\n", "unexpected Rust result: {r:?}");

    let c = run_main(c_so(), "d_main.c", StdinKind::File(b"abcdef\ncd\n"));
    let r = run_main(rust_so(), "d_main.rs", StdinKind::File(b"abcdef\ncd\n"));
    assert_eq!(c.stdout, b"2\n", "unexpected C result: {c:?}");
    assert_eq!(r.stdout, b"2\n", "unexpected Rust result: {r:?}");
}

/// The C reference compiled with `-O2` must agree with the Rust translation too
/// — in particular for the inputs that trigger the program's out-of-bounds
/// write (`s[strlen(s)-1]` with an empty string), where a different code layout
/// could in principle become observable.
#[test]
fn optimized_c_reference_parity() {
    // the UB-triggering inputs first
    let ub_inputs: Vec<&[u8]> = vec![
        b"",
        b"a",
        b"\0",
        b"abc\n",
        b"\0abc\nxyz\n",
        b"abc\n\0xyz\n",
        b"\0\n\0\n",
        b"abcdef\ncd\n",
    ];
    for (i, input) in ub_inputs.iter().enumerate() {
        let c = run_main(c_so_opt(), &format!("d_o2_{i}.c"), StdinKind::File(input));
        let r = run_main(rust_so(), &format!("d_o2_{i}.rs"), StdinKind::File(input));
        assert_eq!(
            c.observable(),
            r.observable(),
            "-O2 C reference diverges for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(input)
        );
    }

    // and a randomized corpus through both entry points
    let mut rng = Rng::new(0xD1);
    for i in 0..120 {
        let len = rng.below(261);
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            let b = match rng.below(16) {
                0 | 1 => b'\n',
                2 => 0u8,
                3 => 0x80 | (rng.byte() & 0x7f),
                4 => b'\r',
                _ => ASCII[rng.below(ASCII.len())],
            };
            input.push(b);
        }
        let c = run_main(c_so_opt(), &format!("d_o2f_{i}.c"), StdinKind::File(&input));
        let r = run_main(rust_so(), &format!("d_o2f_{i}.rs"), StdinKind::File(&input));
        assert_eq!(
            c.observable(),
            r.observable(),
            "-O2 C reference diverges for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(&input)
        );
    }

    let mut cases = Vec::new();
    for _ in 0..300 {
        let l1 = rng.below(120);
        let l2 = rng.below(8);
        cases.push((rng.bytes_nonzero(l1), rng.bytes_nonzero(l2)));
    }
    let c = run_driver_batch(c_so_opt(), "d_o2_drv.c", &cases);
    let r = run_driver_batch(rust_so(), "d_o2_drv.rs", &cases);
    assert_eq!(
        c.observable(),
        r.observable(),
        "-O2 C `driver` diverges from the Rust translation"
    );
}
