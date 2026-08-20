//! Phase C — error-path differential tests.
//!
//! One test per differentiable row of `ERRORS.md`, plus the generic FFI
//! boundary cases (NULL, zero/oversized lengths, one-past-range "enum" values).
//!
//! Rows E1, E7, E8, E14, E15, E16 and E17 are process-fatal in C (SIGSEGV /
//! SIGFPE / unreachable allocator failure) — see the notes at the bottom of
//! `ERRORS.md`; `e_nonfaulting_rows_are_documented` asserts the documentation
//! is present rather than crashing the harness.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
}

// ===========================================================================
// E2 — create_buffer with a negative capacity: malloc sees a huge size_t
// ===========================================================================

fn e2_create_buffer_negative_capacity_returns_null() {
    let mut caps: Vec<c_int> = vec![-1, -2, -3, -4, -8, -16, -32, -1024, i32::MIN, i32::MIN + 1];
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..500 {
        caps.push(rng.range_i32(i32::MIN, -1));
    }

    diff("E2 negative initial_capacity", |imp, t| {
        for &cap in &caps {
            let p = unsafe { (imp.create_buffer)(cap) };
            t.push(Obs::IsNull(p.is_null()));
            if !p.is_null() {
                // Should be unreachable; record for the diff if it ever happens.
                t.buf(p);
                unsafe { (imp.destroy_buffer)(p) };
            }
        }
    });

    // Independently pin the expected sentinel (the C ground truth), so this is
    // not merely "both did something".
    for &cap in &caps {
        let p = unsafe { (c().create_buffer)(cap) };
        assert!(p.is_null(), "C create_buffer({cap}) should return NULL");
        let q = unsafe { (rs().create_buffer)(cap) };
        assert!(q.is_null(), "RUST create_buffer({cap}) should return NULL");
    }
}

// ===========================================================================
// E3 — create_buffer with a huge positive capacity
// ===========================================================================

fn e3_create_buffer_huge_positive_capacity_agrees() {
    let caps: Vec<c_int> = vec![
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        2_000_000_000,
        1_500_000_000,
        1 << 30,
    ];
    diff("E3 huge positive initial_capacity", |imp, t| {
        for &cap in &caps {
            let p = unsafe { (imp.create_buffer)(cap) };
            t.push(Obs::IsNull(p.is_null()));
            if !p.is_null() {
                t.buf(p);
                t.content(p); // data[0] == '\0'
                unsafe { (imp.destroy_buffer)(p) };
            }
        }
    });
}

// ===========================================================================
// E4 — create_buffer(0): malloc(0) succeeds, then a 1-byte OOB '\0' store
// ===========================================================================

fn e4_create_buffer_zero_capacity() {
    diff("E4 initial_capacity == 0", |imp, t| {
        for _ in 0..64 {
            let p = unsafe { (imp.create_buffer)(0) };
            t.push(Obs::IsNull(p.is_null()));
            t.buf(p);
            t.content(p);
            unsafe { (imp.destroy_buffer)(p) };
        }
    });

    // Pin the C ground truth explicitly.
    for imp in [c(), rs()] {
        let p = unsafe { (imp.create_buffer)(0) };
        assert!(!p.is_null(), "{} create_buffer(0) must succeed", imp.name);
        let b = unsafe { *p };
        assert_eq!(b.capacity, 0, "{} capacity", imp.name);
        assert_eq!(b.length, 0, "{} length", imp.name);
        assert_eq!(unsafe { *b.data }, 0, "{} data[0]", imp.name);
        unsafe { (imp.destroy_buffer)(p) };
    }
}

// ===========================================================================
// E5 — append_to_buffer: realloc fails because new_capacity overflows to
//      a negative int, which sign-extends to a ~1.8e19 size_t
// ===========================================================================

fn e5_append_realloc_failure_returns_minus_1() {
    // required = length + strlen + 1 must be positive, but required*2 must wrap.
    // length = 1_500_000_000, strlen = 1  ->  required = 1_500_000_002
    //                                        required*2 wraps to -1_294_967_292
    let cases: Vec<(c_int, usize)> = vec![
        (1_500_000_000, 1),
        (1_073_741_824, 1),  // required = 1073741826, *2 -> -2147483644
        (1_073_741_823, 1),  // required = 1073741825, *2 -> -2147483646
        (2_000_000_000, 0),  // required = 2000000001, *2 -> -294967294
        (i32::MAX - 1, 0),   // required = 2147483647, *2 -> -2
        (1_500_000_000, 32),
    ];

    diff("E5 realloc failure (new_capacity wraps negative)", |imp, t| {
        for &(len, slen) in &cases {
            let p = unsafe { (imp.create_buffer)(16) };
            t.buf(p);
            let before = unsafe { *p };
            unsafe { (*p).length = len };
            let s = cstring(&vec![b'x'; slen]);
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
            let after = unsafe { *p };
            // buffer must be untouched: same data pointer, capacity, length
            t.push(Obs::IsNull(after.data != before.data));
            t.push(Obs::Fields(after.capacity, after.length));
            // restore a sane length before freeing
            unsafe { (*p).length = 0 };
            unsafe { (imp.destroy_buffer)(p) };
        }
    });

    // Pin the sentinel: it must be exactly -1 on both sides.
    for imp in [c(), rs()] {
        for &(len, slen) in &cases {
            let p = unsafe { (imp.create_buffer)(16) };
            unsafe { (*p).length = len };
            let s = cstring(&vec![b'x'; slen]);
            let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
            assert_eq!(
                r, -1,
                "{} append_to_buffer with length={len} strlen={slen} must return -1",
                imp.name
            );
            assert_eq!(unsafe { (*p).capacity }, 16, "{} capacity untouched", imp.name);
            assert_eq!(unsafe { (*p).length }, len, "{} length untouched", imp.name);
            unsafe { (*p).length = 0 };
            unsafe { (imp.destroy_buffer)(p) };
        }
    }
}

// ===========================================================================
// E6 — append_to_buffer: realloc with a huge-but-positive new_capacity.
//      Host dependent (2 GiB may or may not be allocatable) — the requirement
//      is that C and Rust agree exactly.
// ===========================================================================

fn e6_append_realloc_huge_positive_agrees() {
    // length = 1_073_741_820, strlen = 2 -> required = 1_073_741_823
    //                                       required*2 = 2_147_483_646 (positive)
    const LEN: c_int = 1_073_741_820;
    diff("E6 realloc huge positive new_capacity", |imp, t| {
        let p = unsafe { (imp.create_buffer)(8) };
        t.buf(p);
        unsafe { (*p).length = LEN };
        let s = cstring(b"ab");
        let r = unsafe { (imp.append_to_buffer)(p, s.as_ptr() as *const c_char) };
        t.push(Obs::Ret(r));
        let b = unsafe { *p };
        t.push(Obs::Fields(b.capacity, b.length));
        if r == 0 {
            // The three bytes strcpy wrote are the only deterministic content.
            t.content_tail(p, LEN as usize);
        }
        unsafe { (*p).length = 0 };
        unsafe { (imp.destroy_buffer)(p) };
    });
}

// ===========================================================================
// E9 — destroy_buffer(NULL) is a no-op
// ===========================================================================

fn e9_destroy_buffer_null_is_noop() {
    diff("E9 destroy_buffer(NULL)", |imp, t| {
        for _ in 0..1000 {
            unsafe { (imp.destroy_buffer)(std::ptr::null_mut()) };
        }
        t.mark("survived destroy_buffer(NULL) x1000");
        // Heap must still be usable afterwards.
        let p = unsafe { (imp.create_buffer)(8) };
        t.buf(p);
        t.content(p);
        unsafe { (imp.destroy_buffer)(p) };
    });
}

// ===========================================================================
// E10 — destroy_buffer on a struct whose `data` is NULL: skip free(data),
//       still free the struct
// ===========================================================================

fn e10_destroy_buffer_null_data() {
    diff("E10 destroy_buffer with data == NULL", |imp, t| {
        for i in 0..256 {
            // The struct itself must come from libc malloc (destroy_buffer
            // free()s it), exactly like create_buffer's allocation.
            let p = unsafe { malloc(size_of::<StringBuffer>()) } as *mut StringBuffer;
            assert!(!p.is_null());
            unsafe {
                (*p).data = std::ptr::null_mut();
                (*p).capacity = i as c_int;
                (*p).length = -(i as c_int);
                (imp.destroy_buffer)(p);
            }
        }
        t.mark("survived destroy_buffer(data==NULL) x256");
        // Heap sanity afterwards.
        let q = unsafe { (imp.create_buffer)(64) };
        t.buf(q);
        let s = cstring(b"still-alive");
        t.push(Obs::Ret(unsafe {
            (imp.append_to_buffer)(q, s.as_ptr() as *const c_char)
        }));
        t.content(q);
        unsafe { (imp.destroy_buffer)(q) };
    });
}

// ===========================================================================
// E11 — get_operation_name default branch: any int outside 0..=3.
//       This is the "invalid enum value across FFI" case.
// ===========================================================================

fn e11_get_operation_name_out_of_range() {
    let mut codes: Vec<c_int> = vec![
        -1,
        -2,
        -3,
        -4,
        -5,
        4,
        5,
        6,
        7,
        8,
        100,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..4000 {
        let v = rng.next_i32();
        if !(0..=3).contains(&v) {
            codes.push(v);
        }
    }

    diff("E11 get_operation_name default branch", |imp, t| {
        for &code in &codes {
            let p = unsafe { (imp.get_operation_name)(code) };
            t.push(Obs::CStr(read_cstr(p)));
        }
    });

    // Pin the exact sentinel string.
    for imp in [c(), rs()] {
        for &code in &codes {
            let p = unsafe { (imp.get_operation_name)(code) };
            assert_eq!(
                read_cstr(p),
                b"unknown".to_vec(),
                "{} get_operation_name({code})",
                imp.name
            );
        }
        // one step past each end of the valid range
        assert_eq!(read_cstr(unsafe { (imp.get_operation_name)(-1) }), b"unknown");
        assert_eq!(read_cstr(unsafe { (imp.get_operation_name)(4) }), b"unknown");
        // and the in-range values, for contrast
        assert_eq!(read_cstr(unsafe { (imp.get_operation_name)(0) }), b"add");
        assert_eq!(read_cstr(unsafe { (imp.get_operation_name)(3) }), b"divide");
    }
}

// ===========================================================================
// E12 — perform_operation falls through to `return 0` for any non-matching
//       operation string
// ===========================================================================

fn unknown_ops() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"unknown".to_vec(),
        b" ".to_vec(),
        b"Add".to_vec(),
        b"ADD".to_vec(),
        b"aDd".to_vec(),
        b"add ".to_vec(),
        b" add".to_vec(),
        b"add\n".to_vec(),
        b"ad".to_vec(),
        b"a".to_vec(),
        b"adds".to_vec(),
        b"addadd".to_vec(),
        b"subtrac".to_vec(),
        b"subtracts".to_vec(),
        b"Subtract".to_vec(),
        b"multipl".to_vec(),
        b"multiplyy".to_vec(),
        b"Multiply".to_vec(),
        b"divid".to_vec(),
        b"divides".to_vec(),
        b"Divide".to_vec(),
        b"DIVIDE".to_vec(),
        b"div".to_vec(),
        b"mul".to_vec(),
        b"sub".to_vec(),
        b"+".to_vec(),
        b"-".to_vec(),
        b"*".to_vec(),
        b"/".to_vec(),
        b"\x01".to_vec(),
        b"\x7f".to_vec(),
        b"\x80".to_vec(),
        b"\xff".to_vec(),
        b"\xff\xfe\xfd".to_vec(),
        b"add\x80".to_vec(),
        b"\x80add".to_vec(),
        vec![b'z'; 1024],
        (1u8..=255u8).collect(),
    ];
    let mut rng = Rng::new(SEED ^ 0xE12);
    for _ in 0..500 {
        let n = rng.below(24) as usize;
        let s = rng.bytes(n, 0x01, 0xff);
        if !OPS.iter().any(|o| *o == &s[..]) {
            v.push(s);
        }
    }
    v
}

fn e12_perform_operation_unknown_op() {
    let ops = unknown_ops();
    let mut rng = Rng::new(SEED ^ 0xE13);
    let pairs: Vec<(c_int, c_int)> = (0..40)
        .map(|_| (rng.interesting_i32(), rng.interesting_i32()))
        .chain([
            (0, 0),
            (1, 0),
            (0, 1),
            (i32::MIN, -1), // must NOT divide (op is unknown) -> plain 0
            (i32::MAX, i32::MIN),
        ])
        .collect();

    diff("E12 perform_operation unknown operation", |imp, t| {
        for op in &ops {
            let z = cstring(op);
            for &(a, b) in &pairs {
                let r = unsafe { (imp.perform_operation)(a, b, z.as_ptr() as *const c_char) };
                t.push(Obs::Ret(r));
            }
        }
    });

    // Pin the sentinel: it must be exactly 0.
    for imp in [c(), rs()] {
        for op in &ops {
            let z = cstring(op);
            for &(a, b) in &pairs {
                let r = unsafe { (imp.perform_operation)(a, b, z.as_ptr() as *const c_char) };
                assert_eq!(
                    r,
                    0,
                    "{} perform_operation({a}, {b}, {:?}) must be 0",
                    imp.name,
                    String::from_utf8_lossy(op)
                );
            }
        }
    }
}

/// A NUL byte truncates the comparison: `"add\0extra"` IS "add" to strcmp.
fn e12b_perform_operation_embedded_nul_truncates() {
    let cases: Vec<(&[u8], c_int)> = vec![
        (b"add\0extra", 0),
        (b"subtract\0junk", 0),
        (b"multiply\0\xff", 0),
        (b"divide\0x", 0),
        (b"\0add", 0),
        (b"\0", 0),
    ];
    diff("E12b embedded NUL in operation", |imp, t| {
        for (raw, _) in &cases {
            // Already contains a NUL; append one more so it is definitely
            // terminated even if the first byte is not NUL.
            let mut z = raw.to_vec();
            z.push(0);
            for &(a, b) in &[(10, 3), (-10, 3), (7, -2), (0, 0), (i32::MAX, 2)] {
                let r = unsafe { (imp.perform_operation)(a, b, z.as_ptr() as *const c_char) };
                t.push(Obs::Ret(r));
            }
        }
    });
}

// ===========================================================================
// E13 — perform_operation("divide") with b == 0 returns 0 (no SIGFPE)
// ===========================================================================

fn e13_perform_operation_divide_by_zero() {
    let mut avals: Vec<c_int> = vec![0, 1, -1, 2, -2, 7, -7, i32::MAX, i32::MIN, i32::MIN + 1];
    let mut rng = Rng::new(SEED ^ 0xE14);
    for _ in 0..2000 {
        avals.push(rng.interesting_i32());
    }

    diff("E13 divide by zero", |imp, t| {
        let z = cstring(b"divide");
        for &a in &avals {
            let r = unsafe { (imp.perform_operation)(a, 0, z.as_ptr() as *const c_char) };
            t.push(Obs::Ret(r));
        }
    });

    for imp in [c(), rs()] {
        let z = cstring(b"divide");
        for &a in &avals {
            let r = unsafe { (imp.perform_operation)(a, 0, z.as_ptr() as *const c_char) };
            assert_eq!(r, 0, "{} perform_operation({a}, 0, \"divide\")", imp.name);
        }
    }
}

// ===========================================================================
// E18 — buffapp fallback when intermediate3 == 0
// ===========================================================================

fn e18_buffapp_intermediate3_zero_fallback() {
    // (p1 % 4, forcing i1 == 0) x (p3 % 4, forcing i2 == 0) plus the cases
    // where i1 * i2 wraps to zero without either factor being zero.
    let cases: Vec<(c_int, c_int, c_int, c_int)> = vec![
        (0, 0, 0, 0),          // add/add, both zero
        (4, -4, 8, -8),        // add/add, i1 = 0, i2 = 0
        (5, 5, 9, 9),          // subtract/subtract -> 0, 0
        (2, 0, 6, 0),          // multiply/multiply by 0 -> 0, 0
        (3, 0, 7, 0),          // divide by zero -> 0, 0
        (-1, 5, -2, 6),        // unknown/unknown -> 0, 0
        (4, -4, 3, 5),         // i1 = 0, i2 != 0  -> i3 = 0
        (4, 7, 8, -8),         // i1 != 0, i2 = 0  -> i3 = 0
        (65536, 65536, 65536, 65536), // multiply wrap: i1*i2 == 2^32 -> 0
        (1 << 16, 1 << 16, 1 << 16, 1 << 16),
        (i32::MIN, 0, i32::MIN, 0), // add/add -> i1 = i2 = INT_MIN, i3 = 0 (wrap)
        (2, 1 << 16, 2, 1 << 16),   // multiply/multiply -> 2^17 each, product wraps
        (1, 1, 1, 1),               // subtract/subtract -> 0,0
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX), // divide/divide -> 1,1 (i3 = 1)
    ];

    // Verify each case really takes the fallback (where intended) and that both
    // implementations agree on return value AND stdout.
    for &(a, b, cc, d) in &cases {
        diff_buffapp(a, b, cc, d);
    }

    // Explicitly assert the fallback value for the unambiguous ones.
    let fallback: Vec<(c_int, c_int, c_int, c_int)> = vec![
        (0, 0, 0, 0),
        (4, -4, 8, -8),
        (5, 5, 9, 9),
        (2, 0, 6, 0),
        (3, 0, 7, 0),
        (-1, 5, -2, 6),
        (4, -4, 3, 5),
        (4, 7, 8, -8),
    ];
    let _d = discard_stdout();
    for &(a, b, cc, d) in &fallback {
        let want = a
            .wrapping_add(b)
            .wrapping_add(cc)
            .wrapping_add(d);
        for imp in [c(), rs()] {
            let got = unsafe { (imp.buffapp)(a, b, cc, d) };
            assert_eq!(
                got, want,
                "{} buffapp({a},{b},{cc},{d}) should take the i3==0 fallback",
                imp.name
            );
        }
    }
}

// ===========================================================================
// E19 — buffapp with negative `% 4`: C's truncating modulo hits `default`
// ===========================================================================

fn e19_buffapp_negative_modulo_unknown_op() {
    // Any negative param whose residue is -1/-2/-3 must log "unknown(...)" and
    // contribute 0 to `result`.
    let mut rng = Rng::new(SEED ^ 0xE19);
    let mut cases: Vec<(c_int, c_int, c_int, c_int)> = Vec::new();
    for r1 in [-1i32, -2, -3] {
        for r3 in [-1i32, -2, -3] {
            for _ in 0..4 {
                let k1 = rng.range_i32(0, 1000);
                let k3 = rng.range_i32(0, 1000);
                cases.push((
                    -(k1 * 4 + (-r1)),
                    rng.interesting_i32(),
                    -(k3 * 4 + (-r3)),
                    rng.interesting_i32(),
                ));
            }
        }
    }
    cases.push((-1, 12345, -2, -54321));
    cases.push((i32::MIN + 1, 1, i32::MIN + 2, 2)); // residues -3 and -2

    for &(a, b, cc, d) in &cases {
        diff_buffapp(a, b, cc, d);
    }

    // The log must literally say "unknown", and the result must be the
    // 4-way-sum fallback (both intermediates are 0 => i3 == 0).
    for &(a, b, cc, d) in &cases {
        let (rc, oc) = capture_stdout(|| unsafe { (c().buffapp)(a, b, cc, d) });
        let (rr, or) = capture_stdout(|| unsafe { (rs().buffapp)(a, b, cc, d) });
        assert_eq!(rc, rr);
        assert_eq!(oc, or);
        assert_eq!(
            rc,
            a.wrapping_add(b).wrapping_add(cc).wrapping_add(d),
            "buffapp({a},{b},{cc},{d}) fallback"
        );
        let s = String::from_utf8_lossy(&oc).to_string();
        assert!(
            s.contains("Operation 1: unknown(") && s.contains("Operation 2: unknown("),
            "expected 'unknown' ops in log, got:\n{s}"
        );
    }
}

// ===========================================================================
// Generic FFI boundary cases (beyond the ERRORS.md table)
// ===========================================================================

/// Zero and one-past-boundary lengths, and every `initial_capacity` in a dense
/// band around the interesting values.
fn e_generic_capacity_band() {
    let mut caps: Vec<c_int> = (-8..=64).collect();
    caps.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MAX,
        i32::MAX - 1,
        -(1 << 30),
        1 << 30,
    ]);
    diff("generic initial_capacity band", |imp, t| {
        for &cap in &caps {
            let p = unsafe { (imp.create_buffer)(cap) };
            t.push(Obs::IsNull(p.is_null()));
            if !p.is_null() {
                t.buf(p);
                t.content(p);
                // A zero-length append is valid for every capacity.
                let s = cstring(b"");
                t.push(Obs::Ret(unsafe {
                    (imp.append_to_buffer)(p, s.as_ptr() as *const c_char)
                }));
                t.buf(p);
                t.content(p);
                unsafe { (imp.destroy_buffer)(p) };
            }
        }
    });
}

/// `get_operation_name` over a dense band that straddles both ends of the
/// switch's valid range, plus every power-of-two boundary.
fn e_generic_op_code_band() {
    let mut codes: Vec<c_int> = (-32..=32).collect();
    for s in 0..31 {
        codes.push(1i32 << s);
        codes.push(-(1i32 << s));
        codes.push((1i32 << s) - 1);
    }
    codes.extend([i32::MIN, i32::MAX]);
    diff("generic op_code band", |imp, t| {
        for &code in &codes {
            t.push(Obs::CStr(read_cstr(unsafe {
                (imp.get_operation_name)(code)
            })));
        }
    });
}

/// Every valid operation name against every extreme operand pair, including
/// the `divide` pairs adjacent to the excluded `INT_MIN / -1` trap.
fn e_generic_operand_extremes() {
    const V: [i32; 12] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
    ];
    for op in OPS.iter() {
        let z = cstring(op);
        let is_div = *op == &b"divide"[..];
        let label = format!("generic extremes {}", String::from_utf8_lossy(op));
        diff(&label, |imp, t| {
            for &a in &V {
                for &b in &V {
                    // E14: C traps on INT_MIN / -1 (UB). Excluded by design.
                    if is_div && a == i32::MIN && b == -1 {
                        continue;
                    }
                    t.push(Obs::Ret(unsafe {
                        (imp.perform_operation)(a, b, z.as_ptr() as *const c_char)
                    }));
                }
            }
        });
    }
}

/// The `buffapp` residue boundaries: every param exactly at a `% 4` transition.
fn e_generic_buffapp_residue_boundaries() {
    let mut vals: Vec<c_int> = (-12..=12).collect();
    vals.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 2,
        i32::MAX - 3,
    ]);
    let _d = discard_stdout();
    for &a in &vals {
        for &b in &vals {
            diff_buffapp_ret(a, b, b, a);
            diff_buffapp_ret(a, a, b, b);
        }
    }
}

/// The rows whose C behaviour is a fatal fault or an unreachable allocator
/// failure are recorded as such in ERRORS.md; assert the documentation exists
/// so the table cannot silently lose them.
fn e_nonfaulting_rows_are_documented() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/ERRORS.md"))
        .expect("ERRORS.md must exist");
    for row in [
        "| E1 |", "| E2 |", "| E3 |", "| E4 |", "| E5 |", "| E6 |", "| E7 |", "| E8 |", "| E9 |",
        "| E10 |", "| E11 |", "| E12 |", "| E13 |", "| E14 |", "| E15 |", "| E16 |", "| E17 |",
        "| E18 |", "| E19 |",
    ] {
        assert!(doc.contains(row), "ERRORS.md is missing row {row}");
    }
    for note in ["E1", "E7", "E8", "E14", "E15", "E16", "E17"] {
        assert!(
            doc.contains(&format!("{note}")),
            "ERRORS.md must document the non-differentiable row {note}"
        );
    }
}

fn zz_libraries_loaded() {
    assert_loaded();
}

// ===========================================================================
// Sequential runner entry point (harness = false)
// ===========================================================================

fn main() {
    let mut r = Runner::new();
    r.case("e2_create_buffer_negative_capacity_returns_null", e2_create_buffer_negative_capacity_returns_null);
    r.case("e3_create_buffer_huge_positive_capacity_agrees", e3_create_buffer_huge_positive_capacity_agrees);
    r.case("e4_create_buffer_zero_capacity", e4_create_buffer_zero_capacity);
    r.case("e5_append_realloc_failure_returns_minus_1", e5_append_realloc_failure_returns_minus_1);
    r.case("e6_append_realloc_huge_positive_agrees", e6_append_realloc_huge_positive_agrees);
    r.case("e9_destroy_buffer_null_is_noop", e9_destroy_buffer_null_is_noop);
    r.case("e10_destroy_buffer_null_data", e10_destroy_buffer_null_data);
    r.case("e11_get_operation_name_out_of_range", e11_get_operation_name_out_of_range);
    r.case("e12_perform_operation_unknown_op", e12_perform_operation_unknown_op);
    r.case("e12b_perform_operation_embedded_nul_truncates", e12b_perform_operation_embedded_nul_truncates);
    r.case("e13_perform_operation_divide_by_zero", e13_perform_operation_divide_by_zero);
    r.case("e18_buffapp_intermediate3_zero_fallback", e18_buffapp_intermediate3_zero_fallback);
    r.case("e19_buffapp_negative_modulo_unknown_op", e19_buffapp_negative_modulo_unknown_op);
    r.case("e_generic_capacity_band", e_generic_capacity_band);
    r.case("e_generic_op_code_band", e_generic_op_code_band);
    r.case("e_generic_operand_extremes", e_generic_operand_extremes);
    r.case("e_generic_buffapp_residue_boundaries", e_generic_buffapp_residue_boundaries);
    r.case("e_nonfaulting_rows_are_documented", e_nonfaulting_rows_are_documented);
    r.case("zz_libraries_loaded", zz_libraries_loaded);
    r.finish();
}
