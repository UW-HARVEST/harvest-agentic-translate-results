//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md` (rows 1..=10; row 11 lives in `error_paths_siphash.rs` because it
//! needs an exclusive fd 1).
//!
//! The library has **no** error returns, so most rows assert the *implicit*
//! rejection behaviour (bytes silently not taken, sign-extension masking) and the
//! faulting rows assert that both objects die from the **same signal**.

mod common;

use common::{diff_hash, diff_hash_raw, impls, seed_corpus, Rng};
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Runs `f` in a forked child and reports how the child terminated.
/// Used for the two rows whose C behaviour is a fault.
fn child_outcome<F: FnOnce()>(f: F) -> Outcome {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // Child: do the (expected to fault) call, then exit cleanly if it
        // somehow survived.
        f();
        unsafe { libc::_exit(0) };
    }
    let mut status: libc::c_int = 0;
    let r = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else {
        Outcome::Exited(libc::WEXITSTATUS(status))
    }
}

/// mmap `pages` readable/writable pages followed by a guaranteed unmapped hole,
/// so reads past the end fault deterministically. Returns `(base, len)`.
fn mapped_with_guard(pages: usize) -> (*mut u8, usize) {
    let pg = 4096usize;
    let total = (pages + 1) * pg;
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(base, libc::MAP_FAILED, "mmap failed");
    let base = base as *mut u8;
    // Punch out the trailing page so [base + pages*pg, ...) is a hole.
    let rc = unsafe { libc::munmap(base.add(pages * pg) as *mut c_void, pg) };
    assert_eq!(rc, 0, "munmap of guard page failed");
    (base, pages * pg)
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — switch (len - i) -> case 0 takes no tail byte
// ---------------------------------------------------------------------------

#[test]
fn row01_tail_case_zero_takes_no_bytes() {
    let (c, _) = impls();
    let mut rng = Rng::new(0xE001);
    let mut buf = vec![0u8; 128];

    for len in [0usize, 8, 16, 24, 32, 64, 96, 120] {
        for t in 0..32 {
            rng.fill(&mut buf);
            let seed = if t % 2 == 0 { 0 } else { rng.next_usize() };
            let base = diff_hash(&mut buf, len, seed, &format!("row01 len={len} t={t}"));

            // The documented consequence of `case 0`: no byte at index >= len is
            // consulted. Scribble over the tail region and the hash must not move.
            if len < buf.len() {
                let saved = buf[len..].to_vec();
                for b in buf[len..].iter_mut() {
                    *b = !*b;
                }
                let after = unsafe {
                    (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed)
                };
                assert_eq!(
                    base, after,
                    "row01: C consulted bytes beyond len={len} (case 0 should take none)"
                );
                let again =
                    diff_hash(&mut buf, len, seed, &format!("row01b len={len} t={t}"));
                assert_eq!(base, again, "row01: differential after scribble, len={len}");
                buf[len..].copy_from_slice(&saved);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — the switch's implicit default is unreachable
// ---------------------------------------------------------------------------

#[test]
fn row02_switch_default_unreachable() {
    // Structural invariant of the C loop at line 18: on exit, len - i in 0..=7.
    for len in 0..=4096usize {
        let i = (len / 8) * 8; // what the C loop leaves `i` at
        let rem = len - i;
        assert!(
            rem <= 7,
            "len={len}: len - i = {rem} would reach the switch's implicit default"
        );
    }
    // And the differential across every length in that sweep (sampled densely
    // enough to hit every (block-count, remainder) pair).
    let mut rng = Rng::new(0xE002);
    let mut buf = vec![0u8; 300];
    for len in 0..=280usize {
        rng.fill(&mut buf);
        let seed = rng.next_usize();
        diff_hash(&mut buf, len, seed, &format!("row02 len={len}"));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — NULL pointer with len == 0 must NOT be rejected or fault
// ---------------------------------------------------------------------------

#[test]
fn row03_null_ptr_len_zero() {
    let mut rng = Rng::new(0xE003);
    for seed in seed_corpus(&mut rng, 128) {
        let via_null = diff_hash_raw(
            std::ptr::null_mut(),
            0,
            seed,
            &format!("row03 null len=0 seed={seed:#x}"),
        );
        // Must equal the len==0 hash computed from a real buffer: no dereference
        // happens, so the pointer value is irrelevant.
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        let via_buf = diff_hash(&mut buf, 0, seed, &format!("row03 buf len=0 seed={seed:#x}"));
        assert_eq!(
            via_null, via_buf,
            "row03: NULL/len=0 must hash the same as any pointer with len=0 (seed={seed:#x})"
        );
    }
    // Also with wildly bogus non-null pointers — still never dereferenced.
    for p in [1usize, 0xdead_beef, usize::MAX, usize::MAX - 7] {
        diff_hash_raw(p as *mut c_void, 0, 0, &format!("row03 bogus p={p:#x} len=0"));
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — len == 0 ignores the buffer entirely
// ---------------------------------------------------------------------------

#[test]
fn row04_len_zero_ignores_buffer() {
    let mut rng = Rng::new(0xE004);
    for seed in seed_corpus(&mut rng, 32) {
        let mut reference: Option<usize> = None;
        for t in 0..64 {
            let mut buf = vec![0u8; 64];
            rng.fill(&mut buf);
            let h = diff_hash(&mut buf, 0, seed, &format!("row04 seed={seed:#x} t={t}"));
            match reference {
                None => reference = Some(h),
                Some(r) => assert_eq!(
                    r, h,
                    "row04: len=0 hash varied with buffer contents (seed={seed:#x})"
                ),
            }
        }
        for fill in [0x00u8, 0xff] {
            let mut b = vec![fill; 64];
            let h = diff_hash(&mut b, 0, seed, &format!("row04 fill={fill:#04x}"));
            assert_eq!(reference.unwrap(), h, "row04: fill={fill:#04x}");
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — `len << 56` keeps only len & 0xFF (len >= 256)
// ---------------------------------------------------------------------------

#[test]
fn row05_len_shift_truncates_to_low_byte() {
    let mut rng = Rng::new(0xE005);
    let mut buf = vec![0u8; 1100];
    // Cross the 0xFF boundary densely: 255/256/257 and a full 256-wide window,
    // so every `len & 0xFF` value and every remainder is exercised at len >= 256.
    for len in 250..=520usize {
        for t in 0..2 {
            rng.fill(&mut buf);
            let seed = if t == 0 { 0 } else { rng.next_usize() };
            diff_hash(&mut buf, len, seed, &format!("row05 len={len} t={t}"));
        }
    }
    for len in [255usize, 256, 257, 511, 512, 513, 767, 768, 1023, 1024] {
        for fill in [0x00u8, 0x80, 0xff] {
            let mut b = vec![fill; 1100];
            for s in [0usize, usize::MAX] {
                diff_hash(
                    &mut b,
                    len,
                    s,
                    &format!("row05 len={len} fill={fill:#04x} seed={s:#x}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — tail `case 4` sign-extension makes len 4,5,6,7 collide
// ---------------------------------------------------------------------------

#[test]
fn row06_tail_sign_extension_collision() {
    let (c, _) = impls();
    let mut rng = Rng::new(0xE006);

    for t in 0..512 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        buf[3] |= 0x80; // force the signed overflow in `data |= (d[3] << 24)`
        let seed = if t % 3 == 0 { 0 } else { rng.next_usize() };

        // Differential for every one of the four lengths...
        let h: Vec<usize> = (4..=7)
            .map(|len| diff_hash(&mut buf, len, seed, &format!("row06 t={t} len={len}")))
            .collect();
        // ...and the documented C consequence: they are all the same value,
        // because bits 31..63 of `data` are forced to 1, masking `len << 56`
        // and the d[4]/d[5]/d[6] contributions.
        for (k, v) in h.iter().enumerate() {
            assert_eq!(
                h[0], *v,
                "row06: C/Rust lengths 4..7 must collide when d[3]>=0x80 \
                 (t={t}, len={}, buf={buf:02x?})",
                4 + k
            );
        }

        // Bytes 4,5,6 are entirely ignored for these lengths.
        let baseline = h[0];
        for pos in 4..7usize {
            let mut b2 = buf;
            b2[pos] = !b2[pos];
            let got = unsafe { (c.hash_bytes)(b2.as_mut_ptr() as *mut c_void, 7, seed) };
            assert_eq!(
                baseline, got,
                "row06: C consulted d[{pos}] even though sign-extension masks it"
            );
            diff_hash(&mut b2, 7, seed, &format!("row06 mask t={t} pos={pos}"));
        }
    }

    // And the mirror image: with d[3] < 0x80 there is NO masking, so the four
    // lengths must (essentially always) differ — this catches a Rust that
    // sign-extends unconditionally.
    let mut distinct_seen = 0;
    for t in 0..256 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        buf[3] &= 0x7f;
        let seed = rng.next_usize();
        let h: Vec<usize> = (4..=7)
            .map(|len| diff_hash(&mut buf, len, seed, &format!("row06n t={t} len={len}")))
            .collect();
        if h.iter().any(|v| *v != h[0]) {
            distinct_seen += 1;
        }
    }
    assert!(
        distinct_seen > 200,
        "row06: with d[3] < 0x80 lengths 4..7 should almost always differ, \
         only {distinct_seen}/256 did"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — main-loop sign-extension swallows the high word
// ---------------------------------------------------------------------------

#[test]
fn row07_block_sign_extension_swallows_high_word() {
    let (c, _) = impls();
    let mut rng = Rng::new(0xE007);

    for t in 0..256 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        buf[3] |= 0x80; // low word sign-extends -> bits 32..63 all ones
        let seed = if t % 3 == 0 { 0 } else { rng.next_usize() };

        let baseline = diff_hash(&mut buf, 8, seed, &format!("row07 t={t}"));

        // d[4..8] cannot influence the result any more.
        for _ in 0..8 {
            let mut b2 = buf;
            for k in 4..8usize {
                b2[k] = rng.next_u8();
            }
            let got = unsafe { (c.hash_bytes)(b2.as_mut_ptr() as *mut c_void, 8, seed) };
            assert_eq!(
                baseline, got,
                "row07: C result changed with d[4..8] although the low word's \
                 sign-extension masks them (t={t})"
            );
            diff_hash(&mut b2, 8, seed, &format!("row07 vary-high t={t}"));
        }
    }

    // Mirror: with d[3] < 0x80 the high word DOES matter.
    let mut changed = 0;
    for t in 0..256 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        buf[3] &= 0x7f;
        let seed = rng.next_usize();
        let baseline = diff_hash(&mut buf, 8, seed, &format!("row07n t={t}"));
        let mut b2 = buf;
        b2[5] ^= 0xff;
        let got = diff_hash(&mut b2, 8, seed, &format!("row07n2 t={t}"));
        if got != baseline {
            changed += 1;
        }
    }
    assert!(
        changed > 200,
        "row07: with d[3] < 0x80 the high word must matter, only {changed}/256 changed"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — NULL pointer with len > 0 faults identically
// ---------------------------------------------------------------------------

#[test]
fn row08_null_ptr_nonzero_len_faults() {
    let (c, r) = impls();
    for len in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 4096, usize::MAX] {
        let co = child_outcome(|| unsafe {
            let v = (c.hash_bytes)(std::ptr::null_mut(), len, 0);
            std::hint::black_box(v);
        });
        let ro = child_outcome(|| unsafe {
            let v = (r.hash_bytes)(std::ptr::null_mut(), len, 0);
            std::hint::black_box(v);
        });
        assert_eq!(
            co,
            Outcome::Signaled(libc::SIGSEGV),
            "row08: C should die with SIGSEGV for NULL/len={len}, got {co:?}"
        );
        assert_eq!(
            co, ro,
            "row08: NULL pointer with len={len} — C and Rust terminated differently\n  \
             C   = {co:?}\n  RUST= {ro:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — oversized len faults identically
// ---------------------------------------------------------------------------

#[test]
fn row09_oversized_len_faults() {
    let (c, r) = impls();
    let (base, valid) = mapped_with_guard(1); // 4096 valid bytes, then a hole
    unsafe {
        for i in 0..valid {
            *base.add(i) = (i & 0xff) as u8;
        }
    }
    for len in [valid + 1, valid + 8, 1usize << 20, 1usize << 30] {
        let p = base as *mut c_void;
        let co = child_outcome(|| unsafe {
            let v = (c.hash_bytes)(p, len, 0);
            std::hint::black_box(v);
        });
        let ro = child_outcome(|| unsafe {
            let v = (r.hash_bytes)(p, len, 0);
            std::hint::black_box(v);
        });
        assert_eq!(
            co,
            Outcome::Signaled(libc::SIGSEGV),
            "row09: C should die with SIGSEGV for oversized len={len}, got {co:?}"
        );
        assert_eq!(
            co, ro,
            "row09: oversized len={len} — C and Rust terminated differently\n  \
             C   = {co:?}\n  RUST= {ro:?}"
        );
    }
    // The in-bounds length on the very same mapping must still work and agree.
    diff_hash_raw(base as *mut c_void, valid, 0, "row09 in-bounds");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — seed extremes are all valid, never rejected
// ---------------------------------------------------------------------------

#[test]
fn row10_seed_extremes() {
    let mut rng = Rng::new(0xE010);
    let mut buf = vec![0u8; 96];
    rng.fill(&mut buf);
    let seeds = [
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        1usize << 63,
        (1usize << 63) - 1,
        1usize << 62,
        0x8000_0000,
        0x7fff_ffff,
        0xffff_ffff,
        0xffff_ffff_0000_0000,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
    ];
    for &s in &seeds {
        for len in 0..=72usize {
            diff_hash(&mut buf, len, s, &format!("row10 seed={s:#x} len={len}"));
        }
        // seed and !seed both feed the state; check the complement too.
        for len in [0usize, 7, 8, 9, 16, 33, 64] {
            diff_hash(&mut buf, len, !s, &format!("row10c seed=!{s:#x} len={len}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: the Rust must not over-read past `len`
// ---------------------------------------------------------------------------

/// Places the input so that its last byte is the last byte of a mapped page,
/// with an unmapped hole immediately after. If the Rust translation ever read
/// even one byte past `len` (e.g. via an 8-byte load or a slice of the wrong
/// length), this faults where the C does not.
#[test]
fn boundary_no_overread_past_len() {
    let (base, valid) = mapped_with_guard(1);
    let mut rng = Rng::new(0xE0FF);
    unsafe {
        for i in 0..valid {
            *base.add(i) = rng.next_u8();
        }
    }
    for len in 0..=64usize {
        let p = unsafe { base.add(valid - len) } as *mut c_void;
        for seed in [0usize, 1, usize::MAX, 1usize << 63] {
            diff_hash_raw(
                p,
                len,
                seed,
                &format!("boundary len={len} at end-of-page seed={seed:#x}"),
            );
        }
    }
    // Same, but with the buffer *starting* right after a hole is impossible;
    // instead re-verify every 8-byte-aligned start offset near the end.
    for off in 0..16usize {
        let len = 16 - off;
        let p = unsafe { base.add(valid - len) } as *mut c_void;
        diff_hash_raw(p, len, 0, &format!("boundary2 off={off} len={len}"));
    }
}

// ---------------------------------------------------------------------------
// Extra generic boundary: there are no enums, but exercise the full int domain
// of the only non-pointer/non-size parameter shape reachable here.
// ---------------------------------------------------------------------------

#[test]
fn boundary_len_values_around_word_multiples() {
    let mut rng = Rng::new(0xE0AA);
    let mut buf = vec![0u8; 200];
    for base in [0usize, 8, 16, 24, 32, 64, 128, 176] {
        for delta in 0..=8usize {
            let len = base + delta;
            if len > 192 {
                continue;
            }
            for t in 0..8 {
                rng.fill(&mut buf);
                let seed = if t == 0 { 0 } else { rng.next_usize() };
                diff_hash(
                    &mut buf,
                    len,
                    seed,
                    &format!("boundary3 len={len} (base={base}+{delta}) t={t}"),
                );
            }
        }
    }
}
