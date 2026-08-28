//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`: E1 (null input) and the generic FFI
//! boundary rows G1..G6. Row E2 (malloc failure) needs a process-wide resource
//! limit, so it lives in its own test binary: `tests/malloc_failure.rs`.
//!
//! Each test asserts the two implementations produce the *same* rejection —
//! the same sentinel (`NULL`), not merely "both failed somehow".

mod common;

use common::{Rng, bytes_with_nul, c_free, libs};
use std::ffi::c_char;

// ---------------------------------------------------------------------------
// E1 / G1 — `str == NULL` takes `if(!str)` and returns NULL.
// ---------------------------------------------------------------------------
#[test]
fn e1_null_pointer_input() {
    let l = libs();

    let c_res = unsafe { (l.c)(std::ptr::null()) };
    let r_res = unsafe { (l.rust)(std::ptr::null()) };

    assert!(
        c_res.is_null(),
        "E1: C must return NULL for a NULL argument, got {c_res:p}"
    );
    assert!(
        r_res.is_null(),
        "E1: Rust must return NULL for a NULL argument, got {r_res:p}"
    );
    assert_eq!(
        c_res.is_null(),
        r_res.is_null(),
        "E1: rejection sentinel diverged"
    );

    // The sentinel is exactly (char *)0 in both, not some other falsy value.
    assert_eq!(c_res as usize, 0, "E1: C sentinel is not 0");
    assert_eq!(r_res as usize, 0, "E1: Rust sentinel is not 0");

    // Repeat: the null path must be stable and must not allocate.
    for i in 0..1000 {
        let c = unsafe { (l.c)(std::ptr::null()) };
        let r = unsafe { (l.rust)(std::ptr::null()) };
        assert_eq!(c as usize, 0, "E1/iter{i}: C");
        assert_eq!(r as usize, 0, "E1/iter{i}: Rust");
    }
}

// ---------------------------------------------------------------------------
// E1 corollary — the C sets no errno on the null path, so neither may Rust.
// ---------------------------------------------------------------------------
#[test]
fn e1_null_path_leaves_errno_untouched() {
    let l = libs();

    for sentinel in [0i32, 1, 42, libc::EINVAL, libc::ENOMEM] {
        unsafe {
            *libc::__errno_location() = sentinel;
            let c = (l.c)(std::ptr::null());
            let c_errno = *libc::__errno_location();

            *libc::__errno_location() = sentinel;
            let r = (l.rust)(std::ptr::null());
            let r_errno = *libc::__errno_location();

            assert!(c.is_null() && r.is_null());
            assert_eq!(
                c_errno, sentinel,
                "C must not modify errno on the NULL path"
            );
            assert_eq!(
                r_errno, sentinel,
                "Rust must not modify errno on the NULL path"
            );
            assert_eq!(c_errno, r_errno, "errno behaviour diverged");
        }
    }
}

// ---------------------------------------------------------------------------
// G2 — zero length (`""`) is NOT an error: must return a non-NULL 1-byte copy.
//
// This is the boundary that a naive "reject empty input" translation would get
// wrong, so it is asserted as an explicit non-rejection.
// ---------------------------------------------------------------------------
#[test]
fn g2_empty_string_is_not_an_error() {
    let l = libs();
    let empty = b"\0";
    let src = empty.as_ptr() as *const c_char;

    let c = unsafe { (l.c)(src) };
    let r = unsafe { (l.rust)(src) };

    assert!(!c.is_null(), "G2: C must NOT reject the empty string");
    assert!(!r.is_null(), "G2: Rust must NOT reject the empty string");

    unsafe {
        assert_eq!(bytes_with_nul(c), vec![0u8], "G2: C result");
        assert_eq!(bytes_with_nul(r), vec![0u8], "G2: Rust result");
        c_free(c);
        c_free(r);
    }
}

// ---------------------------------------------------------------------------
// G3 — out-of-range enum values across the FFI boundary.
//
// `custom_strdup` has no enum/flag/mode parameter (proven against the header
// text itself), so there is no invalid variant to pass. The closest analogue is
// a payload byte outside the "expected" ASCII range: as a *signed* `char` those
// are negative. All 255 non-NUL values must be accepted, never rejected.
// ---------------------------------------------------------------------------
#[test]
fn g3_no_enum_parameters_all_byte_values_are_valid() {
    // Mechanically confirm the claim from ERRORS.md against the real header.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("c_src/include/lib.h must be readable");
    assert!(
        !header.contains("enum"),
        "header declares an enum; ERRORS.md row G3 must be revisited:\n{header}"
    );
    assert!(
        header.contains("custom_strdup"),
        "unexpected header contents:\n{header}"
    );

    let l = libs();

    // Every non-NUL byte value, alone, must be copied and never rejected.
    for b in 1u16..=255 {
        let buf = [b as u8, 0u8];
        let src = buf.as_ptr() as *const c_char;
        let c = unsafe { (l.c)(src) };
        let r = unsafe { (l.rust)(src) };
        assert!(!c.is_null(), "G3: C rejected byte 0x{b:02X}");
        assert!(!r.is_null(), "G3: Rust rejected byte 0x{b:02X}");
        unsafe {
            assert_eq!(bytes_with_nul(c), vec![b as u8, 0], "G3/C byte 0x{b:02X}");
            assert_eq!(bytes_with_nul(r), vec![b as u8, 0], "G3/Rust byte 0x{b:02X}");
            c_free(c);
            c_free(r);
        }
    }

    // 0x80 and 0xFF are the sign-boundary and all-ones cases; check them as a
    // long run too, since a signed-vs-unsigned `char` slip would show up in
    // length handling rather than in a single byte.
    for &fill in &[0x80u8, 0xFFu8, 0x7Fu8] {
        let mut buf = vec![fill; 4096];
        buf.push(0);
        let src = buf.as_ptr() as *const c_char;
        let c = unsafe { (l.c)(src) };
        let r = unsafe { (l.rust)(src) };
        assert!(!c.is_null() && !r.is_null(), "G3: run of 0x{fill:02X}");
        unsafe {
            let cb = bytes_with_nul(c);
            let rb = bytes_with_nul(r);
            assert_eq!(cb.len(), 4097, "G3: C length for run of 0x{fill:02X}");
            assert_eq!(cb, rb, "G3: run of 0x{fill:02X} diverged");
            c_free(c);
            c_free(r);
        }
    }
}

// ---------------------------------------------------------------------------
// G4 / G5 — there is no documented valid range and no max-length constant in
// the C, so "oversized" and "one past the range" cannot be constructed as
// *rejections*: the only length-related rejection is malloc failure (E2, tested
// in tests/malloc_failure.rs). What is asserted here is the flip side — large
// lengths that the allocator CAN satisfy must be accepted by both, not
// rejected. A Rust translation that invented a length cap would fail here.
// ---------------------------------------------------------------------------
#[test]
fn g4_g5_no_length_cap_large_inputs_are_accepted() {
    let l = libs();

    for &len in &[
        (1usize << 16) - 1,
        1 << 16,
        (1 << 16) + 1,
        (1 << 20) - 1,
        1 << 20,
        (1 << 20) + 1,
        (1 << 22) + 12345,
    ] {
        let mut buf = vec![0x5Au8; len];
        buf.push(0);
        let src = buf.as_ptr() as *const c_char;

        let c = unsafe { (l.c)(src) };
        let r = unsafe { (l.rust)(src) };
        assert!(!c.is_null(), "G4: C rejected a satisfiable len={len}");
        assert!(!r.is_null(), "G4: Rust rejected a satisfiable len={len}");
        unsafe {
            let cb = bytes_with_nul(c);
            let rb = bytes_with_nul(r);
            assert_eq!(cb.len(), len + 1, "G4: C length for len={len}");
            assert_eq!(rb.len(), len + 1, "G4: Rust length for len={len}");
            assert_eq!(cb, rb, "G4: len={len} diverged");
            c_free(c);
            c_free(r);
        }
    }
}

// ---------------------------------------------------------------------------
// G6 — a rejection must not poison later calls (`lib.c` has no global state).
// ---------------------------------------------------------------------------
#[test]
fn g6_failure_does_not_poison_later_calls() {
    let l = libs();
    let mut rng = Rng::new();

    for i in 0..1500 {
        // Alternate: rejection, then success, in both orders.
        let len = rng.in_range(0, 128);
        let mut buf = rng.payload(len);
        buf.push(0);
        let src = buf.as_ptr() as *const c_char;

        if i % 2 == 0 {
            assert_eq!(unsafe { (l.c)(std::ptr::null()) } as usize, 0);
            assert_eq!(unsafe { (l.rust)(std::ptr::null()) } as usize, 0);
        }

        let c = unsafe { (l.c)(src) };
        let r = unsafe { (l.rust)(src) };
        assert!(!c.is_null(), "G6/iter{i}: C poisoned after rejection");
        assert!(!r.is_null(), "G6/iter{i}: Rust poisoned after rejection");

        let mut expected = buf[..len].to_vec();
        expected.push(0);
        unsafe {
            assert_eq!(bytes_with_nul(c), expected, "G6/iter{i}: C");
            assert_eq!(bytes_with_nul(r), expected, "G6/iter{i}: Rust");
            c_free(c);
            c_free(r);
        }

        if i % 2 == 1 {
            assert_eq!(unsafe { (l.c)(std::ptr::null()) } as usize, 0);
            assert_eq!(unsafe { (l.rust)(std::ptr::null()) } as usize, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Extra: NULL interleaved from multiple threads — the function is pure, so both
// implementations must be equally reentrant.
// ---------------------------------------------------------------------------
#[test]
fn g6_reentrant_from_multiple_threads() {
    let l = libs();
    let c_fn = l.c;
    let r_fn = l.rust;

    let mut handles = Vec::new();
    for t in 0..8u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::with_seed(Rng::SEED ^ (t + 1));
            for i in 0..2000 {
                if i % 7 == 0 {
                    assert_eq!(unsafe { c_fn(std::ptr::null()) } as usize, 0);
                    assert_eq!(unsafe { r_fn(std::ptr::null()) } as usize, 0);
                    continue;
                }
                let len = rng.in_range(0, 200);
                let mut buf = rng.payload(len);
                buf.push(0);
                let src = buf.as_ptr() as *const c_char;
                let c = unsafe { c_fn(src) };
                let r = unsafe { r_fn(src) };
                assert!(!c.is_null() && !r.is_null(), "thread{t}/iter{i}");
                let mut expected = buf[..len].to_vec();
                expected.push(0);
                unsafe {
                    assert_eq!(bytes_with_nul(c), expected, "thread{t}/iter{i}: C");
                    assert_eq!(bytes_with_nul(r), expected, "thread{t}/iter{i}: Rust");
                    c_free(c);
                    c_free(r);
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
