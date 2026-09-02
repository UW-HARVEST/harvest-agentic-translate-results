//! Phase B rows 3–5 / Phase C rows 34–35 — exhaustive byte sweeps.
//!
//! These enumerate *every* 1-, 2- and 3-byte NUL-terminated input, which covers
//! every accept/reject branch of `valid_1..valid_4` and every boundary constant
//! by construction rather than by guesswork.

mod common;

use common::*;
use std::ffi::c_char;

/// Fast path: reuse one stack buffer and compare `w_utf8_drop` offsets directly
/// (16.6M iterations makes per-call `Vec` allocation the dominant cost).
fn drop_off(f: DropFn, buf: &[u8]) -> usize {
    let base = buf.as_ptr() as *const c_char;
    let ret = unsafe { f(base) };
    ret as usize - base as usize
}

#[test]
fn row03_and_err34_exhaustive_single_byte() {
    let p = pair();
    for b in 1u16..=255 {
        let s = [b as u8];
        assert_drop_eq(&p, &s, "row03 single byte");
        for m in MODES {
            assert_filter_eq(&p, &s, m, "row03 single byte filter");
        }
    }
}

#[test]
fn row04_exhaustive_two_bytes_drop() {
    let p = pair();
    let mut buf = [0u8; 3];
    for b0 in 1u16..=255 {
        for b1 in 1u16..=255 {
            buf[0] = b0 as u8;
            buf[1] = b1 as u8;
            let c = drop_off(p.c.drop_fn, &buf);
            let r = drop_off(p.rs.drop_fn, &buf);
            assert_eq!(c, r, "drop mismatch on [{b0:02X} {b1:02X}]: C={c} Rust={r}");
        }
    }
}

#[test]
fn row04_exhaustive_two_bytes_filter() {
    let p = pair();
    for b0 in 1u16..=255 {
        for b1 in 1u16..=255 {
            let s = [b0 as u8, b1 as u8];
            for m in CANONICAL_MODES {
                assert_filter_eq(&p, &s, m, "row04 2-byte filter");
            }
        }
    }
    // non-canonical bool over a representative slice of the same space
    for b0 in [0x7Fu8, 0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xEF, 0xF0, 0xF4, 0xF5, 0xFF] {
        for b1 in 1u16..=255 {
            let s = [b0, b1 as u8];
            for m in [2u8, 0xFF] {
                assert_filter_eq(&p, &s, m, "row04 2-byte filter noncanonical");
            }
        }
    }
}

#[test]
fn row05_exhaustive_three_bytes_drop() {
    let p = pair();
    let mut buf = [0u8; 4];
    for b0 in 1u16..=255 {
        buf[0] = b0 as u8;
        for b1 in 1u16..=255 {
            buf[1] = b1 as u8;
            for b2 in 1u16..=255 {
                buf[2] = b2 as u8;
                let c = drop_off(p.c.drop_fn, &buf);
                let r = drop_off(p.rs.drop_fn, &buf);
                if c != r {
                    panic!("drop mismatch on [{b0:02X} {b1:02X} {b2:02X}]: C={c} Rust={r}");
                }
            }
        }
    }
}

/// Exhaustive 3-byte sweep through `w_utf8_filter` in both canonical modes.
/// Every triple is covered; only the leading byte loop is split so the test
/// stays well inside the time budget.
#[test]
fn row05_exhaustive_three_bytes_filter() {
    let p = pair();
    let mut cbuf = [0u8; 4];
    let mut rbuf = [0u8; 4];
    for b0 in 1u16..=255 {
        cbuf[0] = b0 as u8;
        rbuf[0] = b0 as u8;
        for b1 in 1u16..=255 {
            cbuf[1] = b1 as u8;
            rbuf[1] = b1 as u8;
            for b2 in 1u16..=255 {
                cbuf[2] = b2 as u8;
                rbuf[2] = b2 as u8;
                for m in CANONICAL_MODES {
                    let c = call_filter(p.c.filter_fn, &cbuf, m);
                    let r = call_filter(p.rs.filter_fn, &rbuf, m);
                    if c != r {
                        panic!(
                            "filter mismatch on [{b0:02X} {b1:02X} {b2:02X}] mode {m}: \
                             C={c:02X?} Rust={r:02X?}"
                        );
                    }
                }
            }
        }
    }
}

/// Exhaustive 4-byte sweep restricted to the lead bytes that can begin a 4-byte
/// form plus the boundary leads, so every `valid_4` sub-condition is hit with
/// every possible continuation triple.
#[test]
fn row05b_exhaustive_four_byte_leads() {
    let p = pair();
    let mut buf = [0u8; 5];
    for b0 in [
        0x7Fu8, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF3, 0xF4,
        0xF5, 0xF7, 0xF8, 0xFF,
    ] {
        buf[0] = b0;
        for b1 in 1u16..=255 {
            buf[1] = b1 as u8;
            for b2 in 1u16..=255 {
                buf[2] = b2 as u8;
                for b3 in 1u16..=255 {
                    buf[3] = b3 as u8;
                    let c = drop_off(p.c.drop_fn, &buf);
                    let r = drop_off(p.rs.drop_fn, &buf);
                    if c != r {
                        panic!(
                            "drop mismatch on [{b0:02X} {b1:02X} {b2:02X} {b3:02X}]: \
                             C={c} Rust={r}"
                        );
                    }
                }
            }
        }
    }
}
