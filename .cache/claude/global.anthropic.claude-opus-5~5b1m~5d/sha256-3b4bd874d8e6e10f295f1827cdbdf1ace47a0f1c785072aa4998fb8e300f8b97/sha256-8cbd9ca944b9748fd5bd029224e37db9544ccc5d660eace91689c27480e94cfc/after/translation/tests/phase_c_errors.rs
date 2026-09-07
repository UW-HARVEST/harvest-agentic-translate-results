//! Phase C — error-path differential tests, one test per row of ERRORS.md.
//!
//! The C library performs no validation at all (0 `if`s, 0 asserts, 0 error
//! returns), so the "errors" are the implicit boundaries of a raw C pointer API
//! plus the value boundaries of the arithmetic. Each row asserts C and Rust
//! reject/behave *identically* — same sentinel value, or same fatal signal.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ===========================================================================
// Row 1 — NULL pointer: unchecked deref in C. Must die the same way in Rust.
//
// Run in a CHILD process (the fault is not catchable), once per library, and
// compare the raw wait status. The child is this same test binary re-invoked
// with `NULL_DEREF_LIB` set.
// ===========================================================================

const NULL_DEREF_ENV: &str = "NULL_DEREF_LIB";

#[test]
fn err_01_null_pointer_child() {
    // In the parent (env unset) this is a no-op so the test suite still passes.
    let Ok(which) = std::env::var(NULL_DEREF_ENV) else {
        return;
    };
    let path = match which.as_str() {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("bad {NULL_DEREF_ENV}={other}"),
    };
    let lib = unsafe { libloading::Library::new(&path) }.unwrap();
    let f: NextDoubleFn = unsafe {
        let s: libloading::Symbol<NextDoubleFn> = lib.get(b"next_double\0").unwrap();
        *s
    };
    eprintln!("child: calling next_double(NULL) in {which}");
    let v = unsafe { f(std::ptr::null_mut()) };
    // Must not be reached; if it somehow is, print so the parent sees it.
    println!("UNEXPECTEDLY_RETURNED {}", v);
    std::process::exit(0);
}

fn run_null_deref_child(which: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["err_01_null_pointer_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(NULL_DEREF_ENV, which)
        .env("C_SO", c_so_path())
        .env("RUST_SO", rust_so_path())
        .output()
        .expect("spawn child");
    (
        out.status.code(),
        out.status.signal(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

#[test]
fn err_01_null_pointer_same_fatal_signal() {
    let (c_code, c_sig, c_out) = run_null_deref_child("c");
    let (r_code, r_sig, r_out) = run_null_deref_child("rust");

    eprintln!("C   : code={c_code:?} signal={c_sig:?}");
    eprintln!("Rust: code={r_code:?} signal={r_sig:?}");

    assert!(
        !c_out.contains("UNEXPECTEDLY_RETURNED"),
        "C returned from next_double(NULL): {c_out}"
    );
    assert!(
        !r_out.contains("UNEXPECTEDLY_RETURNED"),
        "Rust returned from next_double(NULL): {r_out}"
    );
    assert_eq!(
        c_sig, r_sig,
        "null deref must kill the process with the SAME signal (C {c_sig:?} vs Rust {r_sig:?})"
    );
    assert_eq!(
        c_code, r_code,
        "null deref must give the SAME exit code (C {c_code:?} vs Rust {r_code:?})"
    );
    assert_eq!(
        c_sig,
        Some(libc_sigsegv()),
        "expected SIGSEGV from the C reference, got {c_sig:?}"
    );
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

// ===========================================================================
// Row 2 — exactly-sized 16-byte object: no over-read, no over-write.
// ===========================================================================

#[test]
fn err_02_exact_size_allocation_no_overread() {
    assert_eq!(std::mem::size_of::<CnRnd>(), 16, "cn_rnd_t must be 16 bytes");
    assert_eq!(std::mem::align_of::<CnRnd>(), 8);

    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 102);
    for k in 0..2_000 {
        let seed = r.seed_pair();
        // Two separate 16-byte heap allocations; guard by comparing the
        // complete 16 bytes before/after and checking nothing else changed.
        let mut cb: Box<CnRnd> = Box::new(seed);
        let mut rb: Box<CnRnd> = Box::new(seed);
        let sentinel_c: Box<[u64; 2]> = Box::new([0xDEAD_BEEF_DEAD_BEEF; 2]);
        let sentinel_r: Box<[u64; 2]> = Box::new([0xDEAD_BEEF_DEAD_BEEF; 2]);

        let cv = unsafe { p.c.step_raw(&mut *cb as *mut CnRnd) };
        let rv = unsafe { p.rs.step_raw(&mut *rb as *mut CnRnd) };

        assert_eq!(cv, rv, "row2 iter {k}: bits differ");
        assert_eq!(*cb, *rb, "row2 iter {k}: 16-byte object differs");
        assert_eq!(*sentinel_c, [0xDEAD_BEEF_DEAD_BEEFu64; 2], "row2: C clobbered neighbour");
        assert_eq!(*sentinel_r, [0xDEAD_BEEF_DEAD_BEEFu64; 2], "row2: Rust clobbered neighbour");
    }
}

// ===========================================================================
// Row 3 — misaligned pointer (also covered as CONFIGS row 25).
// ===========================================================================

#[test]
fn err_03_misaligned_pointer() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 103);
    for off in 1..8usize {
        for k in 0..200 {
            let seed = r.seed_pair();
            let mut cbuf = [0u8; 32];
            cbuf[off..off + 8].copy_from_slice(&seed.state[0].to_ne_bytes());
            cbuf[off + 8..off + 16].copy_from_slice(&seed.state[1].to_ne_bytes());
            let mut rbuf = cbuf;

            let cv = unsafe { p.c.step_raw(cbuf.as_mut_ptr().add(off) as *mut CnRnd) };
            let rv = unsafe { p.rs.step_raw(rbuf.as_mut_ptr().add(off) as *mut CnRnd) };

            assert_eq!(cv, rv, "row3 off {off} iter {k}: bits differ");
            assert_eq!(cbuf, rbuf, "row3 off {off} iter {k}: bytes differ");
        }
    }
}

// ===========================================================================
// Row 4 — the all-zero "invalid seed" is NOT rejected; it is a fixed point.
// ===========================================================================

#[test]
fn err_04_all_zero_state_is_accepted_fixed_point() {
    let p = pair();
    let mut cs = CnRnd::new(0, 0);
    let mut rs = CnRnd::new(0, 0);
    for i in 0..10_000 {
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        assert_eq!(cv, rv, "row4 step {i}: bits differ");
        assert_eq!(cs, rs, "row4 step {i}: state differs");
        // exact documented behaviour, not merely "both did something"
        assert_eq!(cv, 0x0000_0000_0000_0000, "row4 step {i}: C must return +0.0 bits");
        assert_eq!(cs, CnRnd::new(0, 0), "row4 step {i}: state must stay all-zero");
    }
    // and it must be +0.0, not -0.0
    let mut s = CnRnd::new(0, 0);
    let v = p.rs.step_f(&mut s);
    assert!(v == 0.0 && v.is_sign_positive(), "must be +0.0, got {v}");
}

// ===========================================================================
// Row 5 — mantissa == all ones: the upper boundary, must stay < 1.0.
// ===========================================================================

#[test]
fn err_05_mantissa_all_ones_boundary() {
    let p = pair();
    // 0x3FFFFFFFFFFFFFFF == 2 - 2^-52; minus 1.0 == 1 - 2^-52 == 0x3FEFFFFFFFFFFFFE
    let expect_bits = (1.0f64 - 2f64.powi(-52)).to_bits();
    let mut r = SplitMix64::new(FIXED_SEED ^ 105);
    for k in 0..1_000 {
        let y = r.next_u64();
        let seed = seed_for_value(u64::MAX, y);
        assert_eq!(model_step(seed).0, u64::MAX);

        let mut cs = seed;
        let mut rs = seed;
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        assert_eq!(cv, rv, "row5 iter {k}: bits differ");
        assert_eq!(cs, rs, "row5 iter {k}: state differs");
        assert_eq!(cv, expect_bits, "row5 iter {k}: expected 1.0-2^-52");
        assert_eq!(cv, 0x3FEF_FFFF_FFFF_FFFE, "row5 iter {k}: raw bit pattern");
        let f = f64::from_bits(cv);
        assert!(f < 1.0 && f.is_finite() && !f.is_nan());
    }

    // one step past: the exponent field can never be disturbed, because
    // `value >> 12` is always <= 0xF_FFFF_FFFF_FFFF. Prove it over the whole
    // random domain.
    let mut r2 = SplitMix64::new(FIXED_SEED ^ 205);
    for k in 0..20_000 {
        let seed = r2.seed_pair();
        let mut cs = seed;
        let mut rs = seed;
        let cv = p.c.step(&mut cs);
        let rv = p.rs.step(&mut rs);
        assert_eq!(cv, rv, "row5b iter {k}");
        assert_eq!(cs, rs, "row5b iter {k}");
        let f = f64::from_bits(cv);
        assert!((0.0..1.0).contains(&f), "row5b iter {k}: {f} escaped [0,1)");
    }
}

// ===========================================================================
// Row 6 — `x + y` unsigned wraparound (must be wrapping_add, not `+`).
// ===========================================================================

#[test]
fn err_06_return_sum_wraps_modulo_2_64() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 106);
    let mut wrapped = 0usize;
    let mut tries = 0usize;
    while wrapped < 3_000 {
        tries += 1;
        assert!(tries < 10_000_000);
        let seed = r.seed_pair();
        let x_final = model_step(seed).1.state[1];
        if x_final.checked_add(seed.state[1]).is_none() {
            let mut cs = seed;
            let mut rs = seed;
            let cv = p.c.step(&mut cs);
            let rv = p.rs.step(&mut rs);
            assert_eq!(cv, rv, "row6 hit {wrapped}: wrapped sum bits differ");
            assert_eq!(cs, rs, "row6 hit {wrapped}: state differs");
            wrapped += 1;
        }
    }
    // A maximally wrapping case: value == 0 via wraparound (not via x==y==0).
    let seed = seed_for_value(0, 0x8000_0000_0000_0001);
    assert_ne!(seed, CnRnd::new(0, 0));
    let mut cs = seed;
    let mut rs = seed;
    assert_eq!(p.c.step(&mut cs), p.rs.step(&mut rs));
    assert_eq!(cs, rs);
}

// ===========================================================================
// Row 7 — `x << 23` discards the high 23 bits.
// ===========================================================================

#[test]
fn err_07_left_shift_23_discards_high_bits() {
    let p = pair();
    // every x whose set bits all live in the top 23 positions => x<<23 == 0
    for i in 41..64u32 {
        let x = 1u64 << i;
        assert_eq!(x << 23, 0, "precondition");
        for y in [0u64, 1, u64::MAX, 0x5555_5555_5555_5555] {
            p.assert_same(CnRnd::new(x, y), 2, &format!("row7 x=1<<{i} y={y:#x}"));
        }
    }
    let mut r = SplitMix64::new(FIXED_SEED ^ 107);
    for k in 0..3_000 {
        let x = r.next_u64() | 0xFFFF_FE00_0000_0000; // all high 23 bits set
        let y = r.next_u64();
        p.assert_same(CnRnd::new(x, y), 2, &format!("row7 iter {k}"));
    }
}

// ===========================================================================
// Row 8 — right shifts collapsing to zero (`x>>17 == 0`, `y>>26 == 0`).
// ===========================================================================

#[test]
fn err_08_right_shifts_to_zero() {
    let p = pair();
    let mut r = SplitMix64::new(FIXED_SEED ^ 108);
    for k in 0..3_000 {
        let x = r.next_u64() & 0x1_FFFF; // x >> 17 == 0
        let y = r.next_u64() & 0x3FF_FFFF; // y >> 26 == 0
        assert_eq!(x >> 17, 0);
        assert_eq!(y >> 26, 0);
        p.assert_same(CnRnd::new(x, y), 2, &format!("row8 iter {k}"));
    }
    // exact boundaries of each shift
    for x in [0u64, 1, (1 << 17) - 1, 1 << 17, (1 << 17) + 1] {
        for y in [0u64, 1, (1 << 26) - 1, 1 << 26, (1 << 26) + 1] {
            p.assert_same(CnRnd::new(x, y), 3, &format!("row8 boundary x={x} y={y}"));
        }
    }
}

// ===========================================================================
// Rows 9 / 10 — no enum and no length/size parameter exists in the API.
// Asserted mechanically against the header so the row cannot silently rot.
// ===========================================================================

#[test]
fn err_09_no_enum_or_scalar_parameters_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let h = std::fs::read_to_string(root.join("c_src/include/lib.h")).unwrap();
    let c = std::fs::read_to_string(root.join("c_src/src/lib.c")).unwrap();

    assert!(!h.contains("enum"), "header now declares an enum; add ERRORS.md rows for out-of-range variants:\n{h}");
    assert!(!c.contains("enum"), "lib.c now uses an enum; add ERRORS.md rows");

    // The only public prototype takes exactly one pointer parameter.
    let proto: Vec<&str> = h.lines().filter(|l| l.contains("next_double")).collect();
    assert_eq!(proto.len(), 1, "unexpected number of next_double declarations: {proto:?}");
    let args = proto[0]
        .split_once('(')
        .and_then(|(_, r)| r.split_once(')'))
        .map(|(a, _)| a.trim().to_string())
        .expect("prototype shape");
    assert_eq!(args, "cn_rnd_t *rnd", "the signature changed; re-derive ERRORS.md");
    assert!(!args.contains(','), "more than one parameter now exists");

    // No validation exists anywhere in the C, which is why the error surface
    // is limited to the implicit boundaries.
    for pat in ["if (", "if(", "assert", "return -", "return NULL", "errno", "goto"] {
        assert!(
            !c.contains(pat),
            "lib.c now contains `{pat}`; ERRORS.md must be re-derived"
        );
    }
}

// ===========================================================================
// Generic boundary sweep: every extreme bit pattern of the only input.
// ===========================================================================

#[test]
fn err_generic_extreme_bit_patterns() {
    let p = pair();
    let extremes: [u64; 18] = [
        0,
        1,
        2,
        u64::MAX,
        u64::MAX - 1,
        1 << 63,
        (1u64 << 63) | 1,
        i64::MAX as u64,
        0x5555_5555_5555_5555,
        0xAAAA_AAAA_AAAA_AAAA,
        0xFFFF_FFFF_0000_0000,
        0x0000_0000_FFFF_FFFF,
        1 << 17,
        (1 << 17) - 1,
        1 << 23,
        (1 << 26) - 1,
        1 << 26,
        0xFFFF_FE00_0000_0000,
    ];
    for &x in &extremes {
        for &y in &extremes {
            p.assert_same(CnRnd::new(x, y), 5, &format!("extremes x={x:#018x} y={y:#018x}"));
        }
    }
}
