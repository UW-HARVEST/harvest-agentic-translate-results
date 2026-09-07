// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md (C1..C16). Both implementations are reached
// only through `dlopen` + exported C symbols.

mod common;

use common::*;

fn c_print() -> FnPtrArg {
    c_fn(b"printIntPtrLine\0")
}
fn rs_print() -> FnPtrArg {
    rs_fn(b"printIntPtrLine\0")
}
fn c_good() -> FnVoid {
    c_fn(b"good\0")
}
fn rs_good() -> FnVoid {
    rs_fn(b"good\0")
}
fn c_driver() -> FnIntArg {
    c_fn(b"driver\0")
}
fn rs_driver() -> FnIntArg {
    rs_fn(b"driver\0")
}

/// Compare stdout of the C and Rust implementations for one pointer input.
fn cmp_print(p: *const libc::c_int, ctx: &str) {
    let (cf, rf) = (c_print(), rs_print());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe { cf(p) });
    let r_out = capture(|| unsafe { rf(p) });
    assert_eq!(
        c_out,
        r_out,
        "printIntPtrLine mismatch ({ctx}): C={:?} RUST={:?}",
        show(&c_out),
        show(&r_out)
    );
}

// ---------------------------------------------------------------- C1
#[test]
fn c1_print_stack_int_randomized() {
    let mut rng = Rng::new(0xC1_5EED);
    for _ in 0..4096 {
        let v: libc::c_int = rng.next_i32();
        let slot = v;
        cmp_print(&slot as *const libc::c_int, &format!("stack v={v}"));
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_print_boundary_values() {
    for &v in INT_BOUNDARIES {
        let slot: libc::c_int = v;
        cmp_print(&slot as *const libc::c_int, &format!("boundary v={v}"));
        // Non-empty, and exactly the decimal rendering + '\n'.
        let cf = c_print();
        let _g = stdout_lock();
        let out = capture(|| unsafe { cf(&slot as *const libc::c_int) });
        assert_eq!(out, format!("{v}\n").into_bytes(), "C reference shape");
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_print_heap_int_randomized() {
    let mut rng = Rng::new(0xC3_5EED);
    for _ in 0..512 {
        let v = rng.next_i32();
        let b = Box::new(v as libc::c_int);
        cmp_print(&*b as *const libc::c_int, &format!("heap v={v}"));
    }
    // Also a raw malloc'ed slot, i.e. libc-allocated storage.
    unsafe {
        let p = libc::malloc(std::mem::size_of::<libc::c_int>()) as *mut libc::c_int;
        assert!(!p.is_null());
        for _ in 0..256 {
            let v = rng.next_i32();
            *p = v;
            cmp_print(p as *const libc::c_int, &format!("malloc v={v}"));
        }
        libc::free(p as *mut libc::c_void);
    }
}

// ---------------------------------------------------------------- C4
static STATIC_INTS: [libc::c_int; 6] = [0, 1, -1, 42, i32::MIN, i32::MAX];

#[test]
fn c4_print_static_storage() {
    for i in 0..STATIC_INTS.len() {
        cmp_print(
            &STATIC_INTS[i] as *const libc::c_int,
            &format!("static idx={i}"),
        );
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn c5_print_array_interior_elements() {
    let mut rng = Rng::new(0xC5_5EED);
    for _ in 0..128 {
        let n = 1 + rng.below(64) as usize;
        let arr: Vec<libc::c_int> = (0..n).map(|_| rng.next_i32()).collect();
        for &idx in &[0usize, n / 2, n - 1] {
            cmp_print(
                unsafe { arr.as_ptr().add(idx) },
                &format!("array n={n} idx={idx}"),
            );
        }
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn c6_print_mmapped_page() {
    unsafe {
        let len = 4096;
        let base = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let ints = base as *mut libc::c_int;
        let mut rng = Rng::new(0xC6_5EED);
        for i in 0..64usize {
            let v = rng.next_i32();
            *ints.add(i) = v;
            cmp_print(ints.add(i) as *const libc::c_int, &format!("mmap i={i}"));
        }
        libc::munmap(base, len);
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_print_misaligned_pointers() {
    unsafe {
        let len = 4096;
        let base = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let bytes = base as *mut u8;
        let mut rng = Rng::new(0xC7_5EED);
        for i in 0..len {
            *bytes.add(i) = rng.next_u8();
        }
        // byte offsets 1,2,3 (mod 4) are all unaligned for a 4-byte int
        for off in [1usize, 2, 3, 5, 6, 7, 1023, 2049] {
            cmp_print(
                bytes.add(off) as *const libc::c_int,
                &format!("misaligned off={off}"),
            );
        }
        libc::munmap(base, len);
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_print_many_calls_single_stream() {
    let mut rng = Rng::new(0xC8_5EED);
    let vals: Vec<libc::c_int> = (0..1000).map(|_| rng.next_i32()).collect();
    let (cf, rf) = (c_print(), rs_print());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe {
        for v in &vals {
            cf(v as *const libc::c_int);
        }
    });
    let r_out = capture(|| unsafe {
        for v in &vals {
            rf(v as *const libc::c_int);
        }
    });
    assert_eq!(c_out, r_out, "1000-call accumulated stdout differs");
    let expected: Vec<u8> = vals
        .iter()
        .flat_map(|v| format!("{v}\n").into_bytes())
        .collect();
    assert_eq!(c_out, expected, "C reference stream shape");
}

// ---------------------------------------------------------------- C9
#[test]
fn c9_good_single_call() {
    let (cf, rf) = (c_good(), rs_good());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe { cf() });
    let r_out = capture(|| unsafe { rf() });
    assert_eq!(c_out, r_out, "good() stdout differs");
    assert_eq!(c_out, b"5\n".to_vec(), "good() must print 5");
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_good_many_calls() {
    let (cf, rf) = (c_good(), rs_good());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe {
        for _ in 0..100 {
            cf()
        }
    });
    let r_out = capture(|| unsafe {
        for _ in 0..100 {
            rf()
        }
    });
    assert_eq!(c_out, r_out, "good() x100 differs");
    assert_eq!(c_out, b"5\n".repeat(100), "no state drift across calls");
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_driver_true() {
    let (cf, rf) = (c_driver(), rs_driver());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe { cf(1) });
    let r_out = capture(|| unsafe { rf(1) });
    assert_eq!(c_out, r_out, "driver(1) differs");
    assert_eq!(c_out, b"5\n".to_vec());
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_driver_randomized_nonzero() {
    let (cf, rf) = (c_driver(), rs_driver());
    let mut rng = Rng::new(0xC12_5EED);
    let mut vals = Vec::new();
    while vals.len() < 2048 {
        let v = rng.next_i32();
        if v != 0 {
            vals.push(v);
        }
    }
    for v in vals {
        let _g = stdout_lock();
        let c_out = capture(|| unsafe { cf(v) });
        let r_out = capture(|| unsafe { rf(v) });
        assert_eq!(c_out, r_out, "driver({v}) differs");
        assert_eq!(c_out, b"5\n".to_vec(), "driver({v}) must take good()");
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_driver_extreme_nonzero() {
    let (cf, rf) = (c_driver(), rs_driver());
    for v in [
        1,
        2,
        -1,
        0x100,
        0x0001_0000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        let _g = stdout_lock();
        let c_out = capture(|| unsafe { cf(v) });
        let r_out = capture(|| unsafe { rf(v) });
        assert_eq!(c_out, r_out, "driver({v}) differs");
        assert_eq!(c_out, b"5\n".to_vec());
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_driver_many_calls() {
    let (cf, rf) = (c_driver(), rs_driver());
    let _g = stdout_lock();
    let c_out = capture(|| unsafe {
        for i in 1..=100 {
            cf(i)
        }
    });
    let r_out = capture(|| unsafe {
        for i in 1..=100 {
            rf(i)
        }
    });
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, b"5\n".repeat(100));
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_mixed_pipeline_interleaved() {
    let (cp, rp) = (c_print(), rs_print());
    let (cg, rg) = (c_good(), rs_good());
    let (cd, rd) = (c_driver(), rs_driver());
    let mut rng = Rng::new(0xC15_5EED);

    for _ in 0..64 {
        let a: libc::c_int = rng.next_i32();
        let b: libc::c_int = rng.next_i32();
        let d: libc::c_int = loop {
            let v = rng.next_i32();
            if v != 0 {
                break v;
            }
        };
        let _g = stdout_lock();
        let c_out = capture(|| unsafe {
            cd(d);
            cp(&a as *const libc::c_int);
            cg();
            cp(&b as *const libc::c_int);
            cd(1);
        });
        let r_out = capture(|| unsafe {
            rd(d);
            rp(&a as *const libc::c_int);
            rg();
            rp(&b as *const libc::c_int);
            rd(1);
        });
        assert_eq!(
            c_out,
            r_out,
            "interleaved pipeline differs (a={a} b={b} d={d}): C={:?} RUST={:?}",
            show(&c_out),
            show(&r_out)
        );
        assert_eq!(c_out, format!("5\n{a}\n5\n{b}\n5\n").into_bytes());
    }
}

// ---------------------------------------------------------------- C16
// `bad()` / `driver(0)` is CWE-457 undefined behaviour. It is still a call the
// C accepts, so parity of the OBSERVABLE OUTCOME CLASS is checked here (in a
// forked child, since the expected outcome may be a fatal signal). The garbage
// pointer value itself is indeterminate and deliberately not pinned.
#[test]
fn c16_bad_path_outcome_class_parity() {
    let (cb, rb) = (c_fn::<FnVoid>(b"bad\0"), rs_fn::<FnVoid>(b"bad\0"));
    let (cd, rd) = (c_driver(), rs_driver());

    let c_bad = run_in_child(|| unsafe { cb() });
    let r_bad = run_in_child(|| unsafe { rb() });
    let c_d0 = run_in_child(|| unsafe { cd(0) });
    let r_d0 = run_in_child(|| unsafe { rd(0) });

    // driver(0) must route to bad() in both implementations: same class as bad().
    assert_eq!(
        c_bad.crashed(),
        c_d0.crashed(),
        "C: driver(0) and bad() must behave alike"
    );
    assert_eq!(
        r_bad.crashed(),
        r_d0.crashed(),
        "RUST: driver(0) and bad() must behave alike: {r_bad:?} vs {r_d0:?}"
    );
    assert_eq!(
        c_bad.crashed(),
        r_bad.crashed(),
        "bad() outcome class differs: C={c_bad:?} RUST={r_bad:?}"
    );
    assert_eq!(
        c_d0.crashed(),
        r_d0.crashed(),
        "driver(0) outcome class differs: C={c_d0:?} RUST={r_d0:?}"
    );
    if c_bad.crashed() {
        // Same fatal signal, and nothing was flushed before dying.
        assert_eq!(
            c_bad.signal, r_bad.signal,
            "bad() fatal signal differs: C={c_bad:?} RUST={r_bad:?}"
        );
        assert_eq!(
            c_d0.signal, r_d0.signal,
            "driver(0) fatal signal differs: C={c_d0:?} RUST={r_d0:?}"
        );
    }
    // Non-vacuity: if the garbage pointer was dereferenceable, both must have
    // produced exactly one `%d\n` line (value indeterminate, shape is not).
    for (tag, o) in [
        ("C bad", &c_bad),
        ("RUST bad", &r_bad),
        ("C driver(0)", &c_d0),
        ("RUST driver(0)", &r_d0),
    ] {
        if !o.crashed() {
            assert_single_decimal_line(&o.stdout, &format!("C16 {tag}"));
        }
    }
    eprintln!("C16: bad() C={c_bad:?} RUST={r_bad:?}");
}

// ---------------------------------------------------------------- extra
// Composition check: driver(nonzero) must be observationally identical to
// good(), which must be identical to printIntPtrLine(&5). This pins the call
// hierarchy, not just each wrapper in isolation.
#[test]
fn composition_driver_eq_good_eq_print5() {
    let five: libc::c_int = 5;
    let (cp, cg, cd) = (c_print(), c_good(), c_driver());
    let (rp, rg, rd) = (rs_print(), rs_good(), rs_driver());
    let _g = stdout_lock();
    let outs: Vec<Vec<u8>> = vec![
        capture(|| unsafe { cp(&five) }),
        capture(|| unsafe { cg() }),
        capture(|| unsafe { cd(7) }),
        capture(|| unsafe { rp(&five) }),
        capture(|| unsafe { rg() }),
        capture(|| unsafe { rd(7) }),
    ];
    for (i, o) in outs.iter().enumerate() {
        assert_eq!(o, &b"5\n".to_vec(), "variant {i} produced {:?}", show(o));
    }
}
