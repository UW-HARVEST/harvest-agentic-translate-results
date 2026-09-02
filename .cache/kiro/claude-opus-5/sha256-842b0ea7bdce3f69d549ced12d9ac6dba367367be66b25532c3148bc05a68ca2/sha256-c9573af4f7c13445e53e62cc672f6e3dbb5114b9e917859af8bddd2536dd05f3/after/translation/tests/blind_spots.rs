//! Extra blind-spot probes: input shapes a real caller can construct that the
//! C header does not forbid in any machine-checkable way.
//!
//! * aliased / overlapping `a` and `b`
//! * misaligned `a` / `b` (the C reads with plain `mov`, which tolerates it)
//!
//! These are not in `CONFIGS.md`/`ERRORS.md` as C *branches* — the C has no code
//! for them — but they are distinct inputs whose behaviour must still match.

mod common;

use common::*;

/// `a == b`: the top-level `memcpy` has `src == dst`, and every `b[k] = a[i]`
/// inside the merge targets the same array it reads from.
#[test]
fn aliased_buffers_identical_pointers() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xA1);

    for &n in &[0usize, 1, 2, 3, 4, 5, 8, 16, 17, 33, 64] {
        for _ in 0..8 {
            let base = gen_small_range(&mut rng, n);

            let mut c_buf = base.clone();
            let mut r_buf = base.clone();
            let (cp, rp) = if n == 0 {
                (std::ptr::null_mut(), std::ptr::null_mut())
            } else {
                (c_buf.as_mut_ptr(), r_buf.as_mut_ptr())
            };

            unsafe { (pair.c)(cp, cp, n as i32) };
            unsafe { (pair.rust)(rp, rp, n as i32) };

            assert_eq!(
                c_buf, r_buf,
                "DIVERGENCE [aliased a==b n={n}]\ninput={base:?}\nC   ={c_buf:?}\nRust={r_buf:?}"
            );
        }
    }
}

/// Overlapping-but-not-equal `a` and `b`, at whole-element offsets (the only
/// overlap reachable for a `spritebatch_sprite_t*`, since elements are 16-byte
/// slots).
#[test]
fn overlapping_buffers_element_offsets() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xA2);

    for &n in &[1usize, 2, 3, 4, 8, 16] {
        for shift in 1..=n {
            for _ in 0..4 {
                let total = n + shift;
                let base = gen_small_range(&mut rng, total);

                let mut c_buf = base.clone();
                let mut r_buf = base.clone();

                unsafe {
                    let ca = c_buf.as_mut_ptr();
                    let cb = ca.add(shift);
                    (pair.c)(ca, cb, n as i32);

                    let ra = r_buf.as_mut_ptr();
                    let rb = ra.add(shift);
                    (pair.rust)(ra, rb, n as i32);
                }

                assert_eq!(
                    c_buf, r_buf,
                    "DIVERGENCE [overlap n={n} shift={shift}]\n\
                     input={base:?}\nC   ={c_buf:?}\nRust={r_buf:?}"
                );
            }
        }
    }
}

/// Misaligned `a` and `b`. The C compiles the struct accesses to plain `mov`
/// instructions with no alignment requirement on x86-64, so a caller handing it
/// a pointer into an unaligned byte buffer gets well-defined behaviour from the
/// C. The Rust must produce the same bytes.
#[test]
fn misaligned_buffers() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xA3);

    for &n in &[1usize, 2, 3, 4, 5, 8, 16, 33] {
        for off in [1usize, 2, 3, 4, 5, 7] {
            for _ in 0..4 {
                let bytes_len = n * 16 + 16;

                // Byte-granular backing store, so the pointers really are
                // misaligned rather than merely claimed to be.
                let mut a_c: Vec<u8> = (0..bytes_len).map(|_| rng.next_u64() as u8).collect();
                let mut b_c: Vec<u8> = (0..bytes_len).map(|_| rng.next_u64() as u8).collect();
                let a_r = a_c.clone();
                let b_r = b_c.clone();
                let mut a_r = a_r;
                let mut b_r = b_r;

                unsafe {
                    (pair.c)(
                        a_c.as_mut_ptr().add(off).cast(),
                        b_c.as_mut_ptr().add(off).cast(),
                        n as i32,
                    );
                    (pair.rust)(
                        a_r.as_mut_ptr().add(off).cast(),
                        b_r.as_mut_ptr().add(off).cast(),
                        n as i32,
                    );
                }

                assert_eq!(
                    a_c, a_r,
                    "DIVERGENCE [misaligned `a` n={n} off={off}]\nC   ={a_c:02x?}\nRust={a_r:02x?}"
                );
                assert_eq!(
                    b_c, b_r,
                    "DIVERGENCE [misaligned `b` n={n} off={off}]\nC   ={b_c:02x?}\nRust={b_r:02x?}"
                );
            }
        }
    }
}

/// Only one of the two buffers misaligned.
#[test]
fn one_buffer_misaligned() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xA4);

    for &n in &[1usize, 2, 3, 5, 8, 17] {
        for off in [1usize, 3, 4] {
            let bytes_len = n * 16 + 16;
            for misalign_a in [true, false] {
                let a0: Vec<u8> = (0..bytes_len).map(|_| rng.next_u64() as u8).collect();
                let b0: Vec<u8> = (0..bytes_len).map(|_| rng.next_u64() as u8).collect();

                let mut a_c = a0.clone();
                let mut b_c = b0.clone();
                let mut a_r = a0.clone();
                let mut b_r = b0.clone();

                let (ao, bo) = if misalign_a { (off, 0) } else { (0, off) };

                unsafe {
                    (pair.c)(
                        a_c.as_mut_ptr().add(ao).cast(),
                        b_c.as_mut_ptr().add(bo).cast(),
                        n as i32,
                    );
                    (pair.rust)(
                        a_r.as_mut_ptr().add(ao).cast(),
                        b_r.as_mut_ptr().add(bo).cast(),
                        n as i32,
                    );
                }

                assert_eq!(a_c, a_r, "DIVERGENCE [one-misaligned a n={n} off={off} a?{misalign_a}]");
                assert_eq!(b_c, b_r, "DIVERGENCE [one-misaligned b n={n} off={off} a?{misalign_a}]");
            }
        }
    }
}
