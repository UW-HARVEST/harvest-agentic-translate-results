//! Harness self-check + first divergence probes.

mod harness;

use harness::{pair, run, Outcome};

fn assert_same(what: &str, a: &Outcome, b: &Outcome) {
    if a != b {
        panic!("DIVERGENCE [{what}]\n  C   : {a:?}\n  Rust: {b:?}");
    }
}

#[test]
fn t00_libraries_load_and_export_everything() {
    let p = pair();
    eprintln!("C   .so: {}", harness::c_so_path().display());
    eprintln!("Rust.so: {}", harness::rust_so_path().display());
    // Touching every symbol proves it resolved.
    for l in [&p.c, &p.rust] {
        unsafe {
            assert_eq!(*l.cp_fixed_table.add(0), 8, "{}", l.name);
            assert_eq!(*l.cp_fixed_table.add(287), 8, "{}", l.name);
            assert_eq!(*l.cp_fixed_table.add(288), 5, "{}", l.name);
            assert_eq!(*l.cp_permutation_order.add(0), 16, "{}", l.name);
            assert_eq!(*l.cp_len_extra_bits.add(8), 1, "{}", l.name);
            assert_eq!(*l.cp_len_base.add(28), 258, "{}", l.name);
            assert_eq!(*l.cp_dist_extra_bits.add(29), 13, "{}", l.name);
            assert_eq!(*l.cp_dist_base.add(29), 24577, "{}", l.name);
            assert!((*l.cp_error_reason).is_null(), "{}", l.name);
        }
    }
}

/// Exported global tables must be byte-identical between the two `.so`s.
#[test]
fn t01_global_tables_identical() {
    let p = pair();
    unsafe {
        let cmp_u8 = |a: *mut u8, b: *mut u8, n: usize, what: &str| {
            let va = std::slice::from_raw_parts(a, n);
            let vb = std::slice::from_raw_parts(b, n);
            assert_eq!(va, vb, "table {what} differs");
        };
        let cmp_u32 = |a: *mut u32, b: *mut u32, n: usize, what: &str| {
            let va = std::slice::from_raw_parts(a, n);
            let vb = std::slice::from_raw_parts(b, n);
            assert_eq!(va, vb, "table {what} differs");
        };
        cmp_u8(p.c.cp_fixed_table, p.rust.cp_fixed_table, 320, "cp_fixed_table");
        cmp_u8(
            p.c.cp_permutation_order,
            p.rust.cp_permutation_order,
            19,
            "cp_permutation_order",
        );
        cmp_u8(
            p.c.cp_len_extra_bits,
            p.rust.cp_len_extra_bits,
            31,
            "cp_len_extra_bits",
        );
        cmp_u8(
            p.c.cp_dist_extra_bits,
            p.rust.cp_dist_extra_bits,
            32,
            "cp_dist_extra_bits",
        );
        cmp_u32(p.c.cp_len_base, p.rust.cp_len_base, 31, "cp_len_base");
        cmp_u32(p.c.cp_dist_base, p.rust.cp_dist_base, 32, "cp_dist_base");
    }
}

/// The fork harness must correctly observe a normal return.
#[test]
fn t02_harness_observes_normal_return() {
    let p = pair();
    let sh = &p.shared;
    let raw_off = harness::OFF_OUT + 64;
    for lib in [&p.c, &p.rust] {
        sh.fill(0xAB);
        sh.write(raw_off, &[0u8, 1, 2, 3, 4, 5, 6, 7]);
        let o = run(sh, lib, (raw_off, 16), |l| unsafe {
            (l.unfilter)(2, 1, 3, sh.at(raw_off))
        });
        assert!(o.completed, "{} child died: {:?}", lib.name, o);
        assert_eq!(o.ret, 1);
    }
}

/// The fork harness must correctly observe SIGABRT from a live C `assert()`.
/// `in_bytes == 0` ⇒ `s->bits_left == 0` ⇒ `assert(s->bits_left > 0)` fires.
#[test]
fn t03_harness_observes_c_abort() {
    let p = pair();
    let sh = &p.shared;
    sh.clear();
    let o = run(sh, &p.c, (harness::OFF_OUT, 16), |l| unsafe {
        (l.cp_inflate)(sh.in_ptr(0) as *mut _, 0, sh.out_ptr(0) as *mut _, 16)
    });
    assert!(!o.completed, "expected the C to abort, got {o:?}");
    assert_eq!(o.signal, libc::SIGABRT, "expected SIGABRT, got {o:?}");
}

/// Probe: does the Rust match that abort? (Documents the E6 divergence.)
#[test]
fn t04_probe_zero_in_bytes() {
    let p = pair();
    let sh = &p.shared;
    let snap = (harness::OFF_OUT, 32);

    sh.clear();
    let c = run(sh, &p.c, snap, |l| unsafe {
        (l.cp_inflate)(sh.in_ptr(0) as *mut _, 0, sh.out_ptr(0) as *mut _, 16)
    });
    sh.clear();
    let r = run(sh, &p.rust, snap, |l| unsafe {
        (l.cp_inflate)(sh.in_ptr(0) as *mut _, 0, sh.out_ptr(0) as *mut _, 16)
    });
    assert_same("cp_inflate(in_bytes=0)", &c, &r);
}
