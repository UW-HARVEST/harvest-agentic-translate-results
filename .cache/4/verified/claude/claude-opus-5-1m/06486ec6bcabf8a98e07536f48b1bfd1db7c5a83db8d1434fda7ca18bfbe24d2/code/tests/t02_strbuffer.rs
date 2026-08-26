//! Differential tests for `strbuffer.c` (CONFIGS rows 6-11, ERRORS rows 207-211).
//!
//! Every operation is driven through the two `.so`s' function pointers; after
//! each call the fully-observable state — `(length, size, bytes at value)` — is
//! snapshotted from both libraries and compared.
mod common;
use common::*;

use std::ffi::{c_char, c_void};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The complete observable state of a `strbuffer_t`: `length`, `size` and the
/// `length + 1` bytes of the (always NUL-terminated) buffer. `value == NULL`
/// yields an empty byte vector — distinguishable from a live empty buffer,
/// which always yields exactly one byte (`[0]`).
fn snap(l: &Lib, sb: &strbuffer_t) -> (usize, usize, Vec<u8>) {
    let p = unsafe { (l.strbuffer_value)(sb as *const strbuffer_t) };
    let bytes = if p.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(p as *const u8, sb.length + 1) }.to_vec()
    };
    (sb.length, sb.size, bytes)
}

type Snap = (usize, usize, Vec<u8>);

#[track_caller]
fn eq_snap(what: &str, c: &Snap, r: &Snap) {
    if c.0 != r.0 || c.1 != r.1 || c.2 != r.2 {
        let first_diff = c
            .2
            .iter()
            .zip(r.2.iter())
            .position(|(a, b)| a != b)
            .map(|i| format!("{} (C={:#04x} RUST={:#04x})", i, c.2[i], r.2[i]))
            .unwrap_or_else(|| String::from("<none, differing lengths only>"));
        panic!(
            "C vs RUST strbuffer divergence in {}\n  \
             C   : length={} size={} nbytes={}\n  \
             RUST: length={} size={} nbytes={}\n  \
             first differing byte: {}\n  \
             C    bytes[..64]: {:?}\n  \
             RUST bytes[..64]: {:?}",
            what,
            c.0,
            c.1,
            c.2.len(),
            r.0,
            r.1,
            r.2.len(),
            first_diff,
            &c.2[..c.2.len().min(64)],
            &r.2[..r.2.len().min(64)],
        );
    }
}

/// `strbuffer_init` on a zeroed struct in both libraries (return values must
/// agree and must be 0, otherwise the machine is out of memory).
fn init_both(d: &Duo) -> (strbuffer_t, strbuffer_t) {
    let mut c = strbuffer_t::zeroed();
    let mut r = strbuffer_t::zeroed();
    unsafe {
        let rc = (d.c.strbuffer_init)(&mut c);
        let rr = (d.rs.strbuffer_init)(&mut r);
        eq("strbuffer_init return", rc, rr);
        assert_eq!(rc, 0, "strbuffer_init failed (out of memory?)");
    }
    (c, r)
}

/// Each buffer must be closed by the library that initialised it — the two
/// `.so`s have independent allocator hooks.
fn close_both(d: &Duo, c: &mut strbuffer_t, r: &mut strbuffer_t) {
    unsafe {
        (d.c.strbuffer_close)(c);
        (d.rs.strbuffer_close)(r);
    }
}

/// One `strbuffer_append_bytes` on both libraries: compares the return value
/// and the resulting snapshot.
#[track_caller]
fn append_bytes_both(
    d: &Duo,
    c: &mut strbuffer_t,
    r: &mut strbuffer_t,
    data: &[u8],
    what: &str,
) -> i32 {
    let p = data.as_ptr() as *const c_char;
    let (rc, rr) = unsafe {
        (
            (d.c.strbuffer_append_bytes)(c, p, data.len()),
            (d.rs.strbuffer_append_bytes)(r, p, data.len()),
        )
    };
    eq(&format!("{} return", what), rc, rr);
    eq_snap(what, &snap(&d.c, c), &snap(&d.rs, r));
    rc
}

/// Deterministic payload of `n` bytes containing embedded NULs and the full
/// 0x00..0xFF range (so negative `c_char` values are exercised too).
fn payload(rng: &mut Rng, n: usize) -> Vec<u8> {
    let mut v = rng.random_bytes(n);
    for i in (0..n).step_by(7) {
        v[i] = 0; // guarantee embedded NULs
    }
    if n > 3 {
        v[1] = 0x7F;
        v[2] = 0x80;
        v[3] = 0xFF;
    }
    v
}

// ---------------------------------------------------------------------------
// 1. CONFIGS 6 — strbuffer_init / strbuffer_close
// ---------------------------------------------------------------------------

#[test]
fn init_and_close_state() {
    let d = duo();
    let mut c = strbuffer_t::zeroed();
    let mut r = strbuffer_t::zeroed();
    unsafe {
        let rc = (d.c.strbuffer_init)(&mut c);
        let rr = (d.rs.strbuffer_init)(&mut r);
        eq("strbuffer_init return", rc, rr);
        eq("strbuffer_init return == 0", rc, 0);

        eq("strbuffer_init length", c.length, r.length);
        eq("strbuffer_init size", c.size, r.size);
        eq("strbuffer_init size == STRBUFFER_MIN_SIZE", c.size, 16usize);
        eq("strbuffer_init length == 0", c.length, 0usize);
        assert!(!c.value.is_null(), "C: strbuffer_init left value NULL");
        assert!(!r.value.is_null(), "RUST: strbuffer_init left value NULL");
        eq("strbuffer_init value[0]", *c.value, *r.value);
        eq("strbuffer_init value[0] == 0", *c.value, 0i8);

        // strbuffer_value must hand back exactly the stored pointer.
        eq(
            "strbuffer_value == value",
            (d.c.strbuffer_value)(&c) as usize == c.value as usize,
            (d.rs.strbuffer_value)(&r) as usize == r.value as usize,
        );
        eq_snap("after init", &snap(&d.c, &c), &snap(&d.rs, &r));

        (d.c.strbuffer_close)(&mut c);
        (d.rs.strbuffer_close)(&mut r);

        eq(
            "strbuffer_close value NULL",
            c.value.is_null(),
            r.value.is_null(),
        );
        assert!(c.value.is_null(), "C: strbuffer_close left value non-NULL");
        eq("strbuffer_close length", c.length, r.length);
        eq("strbuffer_close size", c.size, r.size);
        eq("strbuffer_close length == 0", c.length, 0usize);
        eq("strbuffer_close size == 0", c.size, 0usize);
        eq_snap("after close", &snap(&d.c, &c), &snap(&d.rs, &r));

        // Closing again is a no-op on a NULL value.
        (d.c.strbuffer_close)(&mut c);
        (d.rs.strbuffer_close)(&mut r);
        eq_snap("after second close", &snap(&d.c, &c), &snap(&d.rs, &r));
    }
}

// ---------------------------------------------------------------------------
// 2. CONFIGS 7 — strbuffer_append_byte across the 16 -> 32 -> 64 boundaries
// ---------------------------------------------------------------------------

#[test]
fn append_byte_growth_boundary() {
    let d = duo();
    let mut rng = Rng::new(0x00B0_FFEE_0000_0007);
    let mut bytes: Vec<u8> = vec![0x00, 0x7F, 0x80, 0xFF, b'a', b'Z', 0x01, 0xFE];
    while bytes.len() < 40 {
        bytes.push((rng.next_u32() & 0xFF) as u8);
    }
    // Straddle the growth boundaries with the interesting byte values.
    bytes[14] = 0xFF;
    bytes[15] = 0x00; // the append that grows 16 -> 32
    bytes[16] = 0x80;
    bytes[30] = 0x7F;
    bytes[31] = 0xFF; // the append that grows 32 -> 64
    bytes[32] = 0x00;

    let (mut c, mut r) = init_both(d);
    for (i, &b) in bytes.iter().enumerate() {
        let (rc, rr) = unsafe {
            (
                (d.c.strbuffer_append_byte)(&mut c, b as c_char),
                (d.rs.strbuffer_append_byte)(&mut r, b as c_char),
            )
        };
        eq(&format!("append_byte #{} ({:#04x}) return", i, b), rc, rr);
        eq(&format!("append_byte #{} return == 0", i), rc, 0);
        eq_snap(
            &format!("append_byte #{} ({:#04x})", i, b),
            &snap(&d.c, &c),
            &snap(&d.rs, &r),
        );
    }
    // 40 single-byte appends: 16 -> 32 (at length 15) -> 64 (at length 31).
    eq("40 append_byte length", c.length, 40usize);
    eq("40 append_byte size (C)", c.size, 64usize);
    eq("40 append_byte size", c.size, r.size);
    close_both(d, &mut c, &mut r);
}

// ---------------------------------------------------------------------------
// 3. CONFIGS 8 — strbuffer_append_bytes at every interesting size
// ---------------------------------------------------------------------------

#[test]
fn append_bytes_sizes() {
    let d = duo();
    let mut rng = Rng::new(0x5152_5354_5556_5758);
    for &n in &[0usize, 1, 2, 15, 16, 17, 31, 32, 100, 1000, 10000] {
        let data1 = payload(&mut rng, n);
        let data2 = payload(&mut rng, n);
        let (mut c, mut r) = init_both(d);
        let rc = append_bytes_both(d, &mut c, &mut r, &data1, &format!("append_bytes({}) #1", n));
        eq(&format!("append_bytes({}) #1 return == 0", n), rc, 0);
        eq(&format!("append_bytes({}) #1 length", n), c.length, n);
        let rc = append_bytes_both(d, &mut c, &mut r, &data2, &format!("append_bytes({}) #2", n));
        eq(&format!("append_bytes({}) #2 return == 0", n), rc, 0);
        eq(&format!("append_bytes({}) #2 length", n), c.length, 2 * n);
        // The C keeps the exact bytes plus a terminating NUL.
        let s = snap(&d.c, &c);
        let mut expect = data1.clone();
        expect.extend_from_slice(&data2);
        expect.push(0);
        assert_eq!(s.2, expect, "C content wrong for append_bytes({})", n);
        close_both(d, &mut c, &mut r);
    }
}

// ---------------------------------------------------------------------------
// 4. CONFIGS 8 — randomized append_bytes sequences
// ---------------------------------------------------------------------------

#[test]
fn append_bytes_randomized_sequences() {
    let d = duo();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for seq in 0..400 {
        let (mut c, mut r) = init_both(d);
        let nappends = 1 + rng.below(29); // 1..=29
        for step in 0..nappends {
            let n = rng.below(201); // 0..=200
            let data = if rng.below(4) == 0 {
                vec![0u8; n] // all-NUL chunk
            } else {
                payload(&mut rng, n)
            };
            let rc = append_bytes_both(
                d,
                &mut c,
                &mut r,
                &data,
                &format!("seq {} step {} append_bytes({})", seq, step, n),
            );
            eq(
                &format!("seq {} step {} return == 0", seq, step),
                rc,
                0,
            );
        }
        close_both(d, &mut c, &mut r);
    }
}

// ---------------------------------------------------------------------------
// 5. CONFIGS 9 / ERRORS 210 — strbuffer_pop, including the empty-buffer case
// ---------------------------------------------------------------------------

#[test]
fn pop_behaviour() {
    let d = duo();
    let mut rng = Rng::new(0x0B0B_0B0B_0000_F00D);
    for &n in &[0usize, 1, 2, 16, 17, 100] {
        let data = payload(&mut rng, n);
        let (mut c, mut r) = init_both(d);
        if n > 0 {
            append_bytes_both(d, &mut c, &mut r, &data, &format!("pop setup({})", n));
        }
        for i in 0..(n + 3) {
            let (pc, pr) = unsafe {
                (
                    (d.c.strbuffer_pop)(&mut c),
                    (d.rs.strbuffer_pop)(&mut r),
                )
            };
            eq(&format!("pop #{} of {} return", i, n), pc, pr);
            let expect = if i < n { data[n - 1 - i] as c_char } else { 0 };
            eq(&format!("pop #{} of {} return (vs C source)", i, n), pc, expect);
            eq_snap(
                &format!("pop #{} of {}", i, n),
                &snap(&d.c, &c),
                &snap(&d.rs, &r),
            );
        }
        // ERRORS 210: popping an empty buffer yields '\0' and leaves length 0.
        eq("pop-exhausted length == 0", c.length, 0usize);
        eq("pop-exhausted length", c.length, r.length);
        // A buffer emptied by pop is still usable.
        append_bytes_both(d, &mut c, &mut r, b"reuse", &format!("append after pop({})", n));
        close_both(d, &mut c, &mut r);
    }
}

// ---------------------------------------------------------------------------
// 6. CONFIGS 10 — strbuffer_clear then reuse
// ---------------------------------------------------------------------------

#[test]
fn clear_then_reuse() {
    let d = duo();
    let mut rng = Rng::new(0xC1EA_2C1E_A2C1_EA2C);
    for &n in &[0usize, 1, 100, 10000] {
        let first = payload(&mut rng, n);
        let second = payload(&mut rng, n / 2 + 3);
        let (mut c, mut r) = init_both(d);
        if n > 0 {
            append_bytes_both(d, &mut c, &mut r, &first, &format!("clear setup({})", n));
        }
        unsafe {
            (d.c.strbuffer_clear)(&mut c);
            (d.rs.strbuffer_clear)(&mut r);
        }
        eq_snap(
            &format!("after clear({})", n),
            &snap(&d.c, &c),
            &snap(&d.rs, &r),
        );
        eq(&format!("clear({}) length == 0", n), c.length, 0usize);
        // `size` must be retained (clear does not release the allocation).
        assert_eq!(
            snap(&d.c, &c).2,
            vec![0u8],
            "C: clear must leave a single NUL byte"
        );

        append_bytes_both(d, &mut c, &mut r, &second, &format!("reuse after clear({})", n));
        eq(
            &format!("reuse after clear({}) length", n),
            c.length,
            second.len(),
        );
        // Clear again, then append twice more.
        unsafe {
            (d.c.strbuffer_clear)(&mut c);
            (d.rs.strbuffer_clear)(&mut r);
        }
        eq_snap(
            &format!("after second clear({})", n),
            &snap(&d.c, &c),
            &snap(&d.rs, &r),
        );
        append_bytes_both(d, &mut c, &mut r, &first, &format!("reuse2a({})", n));
        append_bytes_both(d, &mut c, &mut r, &second, &format!("reuse2b({})", n));
        close_both(d, &mut c, &mut r);
    }
}

// ---------------------------------------------------------------------------
// 7. CONFIGS 11 / ERRORS 211 — strbuffer_value + strbuffer_steal_value
// ---------------------------------------------------------------------------

#[test]
fn value_and_steal_value() {
    let d = duo();
    let mut rng = Rng::new(0x57EA_1_57EA_1u64);
    for &n in &[0usize, 1, 17, 500] {
        let data = payload(&mut rng, n);
        let (mut c, mut r) = init_both(d);
        if n > 0 {
            append_bytes_both(d, &mut c, &mut r, &data, &format!("steal setup({})", n));
        }

        // strbuffer_value: same contents in both libraries.
        unsafe {
            let cp = (d.c.strbuffer_value)(&c);
            let rp = (d.rs.strbuffer_value)(&r);
            assert!(!cp.is_null() && !rp.is_null(), "value must be non-NULL");
            eq_bytes(
                &format!("strbuffer_value({})", n),
                std::slice::from_raw_parts(cp as *const u8, c.length + 1),
                std::slice::from_raw_parts(rp as *const u8, r.length + 1),
            );
        }

        let before_c = snap(&d.c, &c);
        let before_r = snap(&d.rs, &r);
        eq_snap(&format!("before steal({})", n), &before_c, &before_r);

        unsafe {
            let cp = (d.c.strbuffer_steal_value)(&mut c);
            let rp = (d.rs.strbuffer_steal_value)(&mut r);
            assert!(!cp.is_null(), "C: first steal returned NULL");
            eq(
                &format!("steal({}) NULLness", n),
                cp.is_null(),
                rp.is_null(),
            );
            // The stolen block holds the same bytes (+ NUL) in both libraries.
            let cb = std::slice::from_raw_parts(cp as *const u8, before_c.0 + 1);
            let rb = std::slice::from_raw_parts(rp as *const u8, before_r.0 + 1);
            eq_bytes(&format!("stolen bytes({})", n), cb, rb);
            assert_eq!(
                cb.to_vec(),
                before_c.2,
                "C: stolen bytes differ from the buffer contents"
            );

            // The C only clears `value`; `length` and `size` are LEFT AS-IS.
            eq(
                &format!("steal({}) value NULL", n),
                c.value.is_null(),
                r.value.is_null(),
            );
            assert!(c.value.is_null(), "C: steal must NULL out value");
            eq(&format!("steal({}) length", n), c.length, r.length);
            eq(&format!("steal({}) size", n), c.size, r.size);
            eq(
                &format!("steal({}) length unchanged (C keeps it)", n),
                c.length,
                before_c.0,
            );
            eq(
                &format!("steal({}) size unchanged (C keeps it)", n),
                c.size,
                before_c.1,
            );
            // snap() now sees value == NULL -> empty byte vector in both.
            eq_snap(
                &format!("after steal({})", n),
                &snap(&d.c, &c),
                &snap(&d.rs, &r),
            );

            // strbuffer_value on a stolen-from buffer returns NULL.
            eq(
                &format!("strbuffer_value after steal({}) NULL", n),
                (d.c.strbuffer_value)(&c).is_null(),
                (d.rs.strbuffer_value)(&r).is_null(),
            );

            // ERRORS 211: the second steal returns NULL.
            let cp2 = (d.c.strbuffer_steal_value)(&mut c);
            let rp2 = (d.rs.strbuffer_steal_value)(&mut r);
            eq(
                &format!("second steal({}) NULLness", n),
                cp2.is_null(),
                rp2.is_null(),
            );
            assert!(cp2.is_null(), "C: second steal must return NULL");
            eq(&format!("second steal({}) length", n), c.length, r.length);
            eq(&format!("second steal({}) size", n), c.size, r.size);

            // Each stolen block belongs to the library that produced it.
            (d.c.jsonp_free)(cp as *mut c_void);
            (d.rs.jsonp_free)(rp as *mut c_void);
        }
        // value is already NULL; close() just zeroes length/size.
        close_both(d, &mut c, &mut r);
        eq_snap(
            &format!("close after steal({})", n),
            &snap(&d.c, &c),
            &snap(&d.rs, &r),
        );
        eq("close after steal length", c.length, 0usize);
        eq("close after steal size", c.size, 0usize);
    }
}

// ---------------------------------------------------------------------------
// 8. ERRORS 207/208/209 — the three overflow guards in append_bytes
// ---------------------------------------------------------------------------

/// A hand-built `strbuffer_t` with a bogus `length`/`size` whose append must be
/// rejected *before* any memory is touched. Only the return value (and the fact
/// that the struct is untouched) is compared: the state is not reachable
/// through the public API, so `snap()` cannot be taken.
#[track_caller]
fn guard_case(d: &Duo, what: &str, length: usize, size: usize, append: usize) {
    let src = vec![0x41u8; 512];
    let mut rets = [0i32; 2];
    for (i, l) in d.both().into_iter().enumerate() {
        unsafe {
            let buf = (l.jsonp_malloc)(64) as *mut c_char;
            assert!(!buf.is_null(), "{}: jsonp_malloc failed", l.which);
            std::ptr::write_bytes(buf as *mut u8, 0, 64);
            let mut sb = strbuffer_t {
                value: buf,
                length,
                size,
            };
            rets[i] = (l.strbuffer_append_bytes)(&mut sb, src.as_ptr() as *const c_char, append);
            assert_eq!(
                sb.value, buf,
                "{}: {} must not modify value",
                l.which, what
            );
            assert_eq!(
                sb.length, length,
                "{}: {} must not modify length",
                l.which, what
            );
            assert_eq!(sb.size, size, "{}: {} must not modify size", l.which, what);
            // The 64-byte block must be untouched (all zeroes).
            let seen = std::slice::from_raw_parts(buf as *const u8, 64);
            assert!(
                seen.iter().all(|&b| b == 0),
                "{}: {} wrote into the buffer: {:?}",
                l.which,
                what,
                seen
            );
            (l.jsonp_free)(buf as *mut c_void);
        }
    }
    eq(&format!("{} return", what), rets[0], rets[1]);
    assert_eq!(rets[0], -1, "C: {} must return -1", what);
}

#[test]
fn append_bytes_overflow_rejections() {
    let d = duo();
    let src = vec![0x5Au8; 32];

    // --- ERRORS 207: `size > SIZE_MAX - 1`, on a real freshly-init'd buffer.
    // --- and the SIZE_MAX-1 case, which passes all three guards and is then
    //     rejected by the failing `jsonp_realloc(.., SIZE_MAX)`.
    for &n in &[usize::MAX, usize::MAX - 1] {
        let (mut c, mut r) = init_both(d);
        let before_c = snap(&d.c, &c);
        let before_r = snap(&d.rs, &r);
        eq_snap(&format!("before append_bytes({})", n), &before_c, &before_r);
        let cptr_c = c.value;
        let cptr_r = r.value;

        let (rc, rr) = unsafe {
            (
                (d.c.strbuffer_append_bytes)(&mut c, src.as_ptr() as *const c_char, n),
                (d.rs.strbuffer_append_bytes)(&mut r, src.as_ptr() as *const c_char, n),
            )
        };
        eq(&format!("append_bytes(size={}) return", n), rc, rr);
        assert_eq!(rc, -1, "C: append_bytes(size={}) must return -1", n);

        let after_c = snap(&d.c, &c);
        let after_r = snap(&d.rs, &r);
        eq_snap(&format!("after append_bytes({})", n), &after_c, &after_r);
        assert_eq!(
            before_c, after_c,
            "C: append_bytes(size={}) changed observable state",
            n
        );
        assert_eq!(
            before_r, after_r,
            "RUST: append_bytes(size={}) changed observable state",
            n
        );
        assert_eq!(c.value, cptr_c, "C: value pointer changed");
        assert_eq!(r.value, cptr_r, "RUST: value pointer changed");
        close_both(d, &mut c, &mut r);
    }

    // --- ERRORS 209: `strbuff->size > SIZE_MAX / 2`.
    // size = 2^63 == SIZE_MAX/2 + 1, append 2^63 so `size >= sb.size - length`.
    guard_case(
        d,
        "guard `strbuff->size > SIZE_MAX/2` (size=2^63, append=2^63)",
        0,
        usize::MAX / 2 + 1,
        usize::MAX / 2 + 1,
    );
    // The same guard with a smaller append that still reaches the grow branch.
    guard_case(
        d,
        "guard `strbuff->size > SIZE_MAX/2` (size=SIZE_MAX-4, length=SIZE_MAX-8, append=8)",
        usize::MAX - 8,
        usize::MAX - 4,
        8,
    );

    // --- ERRORS 208: `strbuff->length > SIZE_MAX - 1 - size`.
    // `size` must stay <= SIZE_MAX/2 so guard 209 does not fire first.
    guard_case(
        d,
        "guard `length > SIZE_MAX-1-size` (length=SIZE_MAX-100, size=16, append=200)",
        usize::MAX - 100,
        16,
        200,
    );
    // size == SIZE_MAX/2 exactly (guard 209 uses `>`, so it does not fire) and
    // length == 2^62 > SIZE_MAX-1-append == 2^62-1.
    guard_case(
        d,
        "guard `length > SIZE_MAX-1-size` (length=2^62, size=SIZE_MAX/2, append=SIZE_MAX-2^62)",
        1usize << 62,
        usize::MAX / 2,
        usize::MAX - (1usize << 62),
    );

    // --- ERRORS 207 again, on a hand-built buffer: `size > SIZE_MAX - 1`.
    guard_case(
        d,
        "guard `size > SIZE_MAX-1` (length=0, size=16, append=SIZE_MAX)",
        0,
        16,
        usize::MAX,
    );
}
