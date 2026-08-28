//! Phase C — ERRORS.md rows 1–6, 24, 27–29: every *returning* rejection path.
//!
//! Abort/`assert()` rows live in `tests/aborts.rs` (they need a child process).

mod common;

use common::*;

const E_STORED_COMPLEMENT: &str =
    "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
const E_STORED_BEYOND: &str = "Stored block extends beyond end of input stream.";
const E_OUT_SYMBOL: &str = "Attempted to overwrite out buffer while outputting a symbol.";
const E_BAD_DISTANCE: &str = "Attempted to write before out buffer (invalid backwards distance).";
const E_OUT_STRING: &str = "Attempted to overwrite out buffer while outputting a string.";
const E_UNKNOWN_BLOCK: &str = "Detected unknown block type within input stream.";

fn reason(p: &Pair) -> (Option<String>, Option<String>) {
    (
        p.c.error_reason().map(|v| String::from_utf8_lossy(&v).into_owned()),
        p.rs.error_reason().map(|v| String::from_utf8_lossy(&v).into_owned()),
    )
}

/// Run on both libraries, require failure, and require the *same* message.
fn expect_error(p: &Pair, stream: &[u8], in_off: usize, out_len: usize, msg: &str, label: &str) {
    let (rc, _) = diff_inflate(p, stream, in_off, out_len, 0, label);
    assert_eq!(rc, 0, "[{label}] expected cp_inflate to fail");
    let (c, rs) = reason(p);
    assert_eq!(c.as_deref(), Some(msg), "[{label}] C message");
    assert_eq!(rs.as_deref(), Some(msg), "[{label}] Rust message");
}

// --- ERRORS row 1 ----------------------------------------------------------
#[test]
fn err01_stored_len_nlen_mismatch() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for len in [0usize, 1, 4, 8, 32, 200] {
        let payload = rng.bytes(len);
        for corrupt in 0..4u32 {
            let mut stream = stored_stream(&payload, true);
            // flip a bit in NLEN (bytes 3..5) so it is no longer ~LEN
            let idx = 3 + (corrupt as usize % 2);
            stream[idx] ^= 1 << (corrupt / 2);
            for off in 0..4usize {
                expect_error(
                    &p,
                    &stream,
                    off,
                    len.max(1),
                    E_STORED_COMPLEMENT,
                    &format!("err01/len{len}/c{corrupt}/off{off}"),
                );
            }
        }
        // and a wholesale wrong NLEN
        let mut stream = stored_stream(&payload, true);
        stream[3] = 0x00;
        stream[4] = 0x00;
        if len != 0xFFFF {
            expect_error(&p, &stream, 0, len.max(1), E_STORED_COMPLEMENT, "err01/zeroNLEN");
        }
    }
}

// --- ERRORS row 2 ----------------------------------------------------------
/// The C requires `bits_left / 8 <= LEN`; appending bytes after a stored block
/// makes more input remain than `LEN` claims, so it rejects.
#[test]
fn err02_stored_extends_beyond() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE2);
    for len in [3usize, 4, 8, 16, 64] {
        for extra in [1usize, 2, 3, 4, 9, 40] {
            let payload = rng.bytes(len);
            let mut stream = stored_stream(&payload, true);
            stream.extend_from_slice(&rng.bytes(extra));
            for off in 0..4usize {
                expect_error(
                    &p,
                    &stream,
                    off,
                    len + extra,
                    E_STORED_BEYOND,
                    &format!("err02/len{len}/extra{extra}/off{off}"),
                );
            }
        }
    }
}

// --- ERRORS row 3 ----------------------------------------------------------
#[test]
fn err03_out_full_on_literal() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE3);
    for case in 0..64 {
        let n = rng.range(1, 20) as usize;
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &items);
        let mut stream = w.bytes;
        stream.extend_from_slice(&[0u8; 4]);
        // out_bytes smaller than the payload: the (out_len)-th literal trips it
        for out_len in [0usize, 1, n / 2] {
            if out_len >= n {
                continue;
            }
            for off in 0..4usize {
                expect_error(
                    &p,
                    &stream,
                    off,
                    out_len,
                    E_OUT_SYMBOL,
                    &format!("err03/{case}/out{out_len}/off{off}"),
                );
            }
        }
    }
}

// --- ERRORS row 4 ----------------------------------------------------------
/// A match as the very first item: `out - backwards_distance < begin`.
#[test]
fn err04_bad_backwards_distance() {
    let p = pair();
    let lit = Huff::new(fixed_lit_lens());
    let dst = Huff::new(fixed_dist_lens());
    let mut rng = Rng::new(SEED ^ 0xE4);
    for case in 0..64 {
        // `prefix` literals then a match whose distance exceeds what was output
        let prefix = rng.below(6) as usize;
        let dist_idx = rng.below(20) as usize;
        let dist_extra = if DIST_EXTRA[dist_idx] == 0 {
            0
        } else {
            rng.below(1u32 << DIST_EXTRA[dist_idx])
        };
        let dist = DIST_BASE[dist_idx] + dist_extra;
        if (dist as usize) <= prefix {
            continue;
        }
        let len_idx = rng.below(8) as usize; // 0 extra bits

        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        for _ in 0..prefix {
            lit.put(&mut w, rng.u8() as usize);
        }
        lit.put(&mut w, 257 + len_idx);
        dst.put(&mut w, dist_idx);
        w.bits_lsb(dist_extra, DIST_EXTRA[dist_idx] as u32);
        lit.put(&mut w, 256);
        let mut stream = w.bytes;
        stream.extend_from_slice(&[0u8; 8]);

        // out big enough that the *length* check would pass, so the distance
        // check is the one that fires
        let out_len = prefix + 512;
        for off in 0..4usize {
            expect_error(
                &p,
                &stream,
                off,
                out_len,
                E_BAD_DISTANCE,
                &format!("err04/{case}/off{off}"),
            );
        }
    }
}

// --- ERRORS row 5 ----------------------------------------------------------
/// Distance is fine, but the copy would run past `out_end`.
#[test]
fn err05_out_full_on_string() {
    let p = pair();
    let lit = Huff::new(fixed_lit_lens());
    let dst = Huff::new(fixed_dist_lens());
    let mut rng = Rng::new(SEED ^ 0xE5);
    for case in 0..64 {
        let prefix = rng.range(1, 8) as usize;
        let len_idx = rng.range(2, 8) as usize; // length 5..10, 0 extra bits
        let length = LEN_BASE[len_idx] as usize;
        let mut w = BitWriter::new();
        w.bit(1);
        w.bits_lsb(1, 2);
        for _ in 0..prefix {
            lit.put(&mut w, rng.u8() as usize);
        }
        lit.put(&mut w, 257 + len_idx);
        dst.put(&mut w, 0); // distance 1
        lit.put(&mut w, 256);
        let mut stream = w.bytes;
        stream.extend_from_slice(&[0u8; 8]);

        // room for the prefix but not for the whole copied string
        let out_len = prefix + length - 1;
        for off in 0..4usize {
            expect_error(
                &p,
                &stream,
                off,
                out_len,
                E_OUT_STRING,
                &format!("err05/{case}/off{off}"),
            );
        }
    }
}

// --- ERRORS row 6 / 27 -----------------------------------------------------
#[test]
fn err06_btype_3_unknown_block() {
    let p = pair();
    for bfinal in [0u8, 1] {
        let b0 = bfinal | (3 << 1);
        for extra in [3usize, 4, 5, 7, 8, 16] {
            let mut stream = vec![b0];
            stream.extend_from_slice(&vec![0u8; extra]);
            for off in 0..4usize {
                expect_error(
                    &p,
                    &stream,
                    off,
                    16,
                    E_UNKNOWN_BLOCK,
                    &format!("err06/bf{bfinal}/x{extra}/off{off}"),
                );
            }
        }
    }
}

// --- ERRORS row 24 ---------------------------------------------------------
#[test]
fn err24_out_bytes_zero_and_negative() {
    let p = pair();
    let items: Vec<Item> = (0..8).map(|i| Item::Lit(i as u8 + 1)).collect();
    let mut w = BitWriter::new();
    write_fixed_block(&mut w, true, &items);
    let mut stream = w.bytes;
    stream.extend_from_slice(&[0u8; 4]);

    let f_c = p.c.cp_inflate();
    let f_rs = p.rs.cp_inflate();

    for out_bytes in [0i32, -1, -8, -1000, i32::MIN] {
        p.c.set_error_reason_null();
        p.rs.set_error_reason_null();
        let mut in_c = AlignedBuf::new(&stream, 0);
        let mut in_rs = AlignedBuf::new(&stream, 0);
        let mut out_c = AlignedBuf::zeroed(64, 0);
        let mut out_rs = AlignedBuf::zeroed(64, 0);
        let n = stream.len() as std::ffi::c_int;
        let rc_c = unsafe {
            f_c(
                in_c.ptr() as *mut std::ffi::c_void,
                n,
                out_c.ptr() as *mut std::ffi::c_void,
                out_bytes,
            )
        };
        let rc_rs = unsafe {
            f_rs(
                in_rs.ptr() as *mut std::ffi::c_void,
                n,
                out_rs.ptr() as *mut std::ffi::c_void,
                out_bytes,
            )
        };
        assert_eq!(rc_c, rc_rs, "out_bytes={out_bytes}: rc differs");
        assert_eq!(rc_c, 0, "out_bytes={out_bytes}: expected failure");
        assert_eq!(out_c.all_bytes(), out_rs.all_bytes(), "out_bytes={out_bytes}");
        let (c, rs) = reason(&p);
        assert_eq!(c.as_deref(), Some(E_OUT_SYMBOL), "out_bytes={out_bytes}");
        assert_eq!(rs.as_deref(), Some(E_OUT_SYMBOL), "out_bytes={out_bytes}");
    }
}

// --- ERRORS row 28 ---------------------------------------------------------
/// `cp_stored` performs *no* output bound check, so a stored block with
/// `LEN > out_bytes` overruns the output buffer.  Both implementations must
/// overrun identically.
#[test]
fn err28_stored_overruns_out() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE8);
    for len in [4usize, 8, 17, 64, 200] {
        for out_len in [0usize, 1, 3] {
            let payload = rng.bytes(len);
            let stream = stored_stream(&payload, true);
            // AlignedBuf::zeroed over-allocates, so the overrun stays inside our
            // own allocation and is byte-comparable.
            let f_c = p.c.cp_inflate();
            let f_rs = p.rs.cp_inflate();
            let mut in_c = AlignedBuf::new(&stream, 0);
            let mut in_rs = AlignedBuf::new(&stream, 0);
            let mut out_c = AlignedBuf::zeroed(len + 64, 0);
            let mut out_rs = AlignedBuf::zeroed(len + 64, 0);
            let n = stream.len() as std::ffi::c_int;
            let rc_c = unsafe {
                f_c(
                    in_c.ptr() as *mut std::ffi::c_void,
                    n,
                    out_c.ptr() as *mut std::ffi::c_void,
                    out_len as std::ffi::c_int,
                )
            };
            let rc_rs = unsafe {
                f_rs(
                    in_rs.ptr() as *mut std::ffi::c_void,
                    n,
                    out_rs.ptr() as *mut std::ffi::c_void,
                    out_len as std::ffi::c_int,
                )
            };
            assert_eq!(rc_c, rc_rs, "len={len} out={out_len}");
            assert_eq!(rc_c, 1, "len={len} out={out_len}: stored has no bound check");
            assert_eq!(
                out_c.all_bytes(),
                out_rs.all_bytes(),
                "len={len} out={out_len}: overrun bytes differ"
            );
            assert_eq!(&out_c.payload()[..len], &payload[..], "payload not copied");
        }
    }
}

// --- ERRORS row 29 ---------------------------------------------------------
#[test]
fn err29_stored_zero_len() {
    let p = pair();
    let stream = stored_stream(&[], true);
    assert_eq!(stream, vec![0x01u8, 0x00, 0x00, 0xFF, 0xFF]);
    for off in 0..4usize {
        for out_len in [0usize, 1, 8] {
            let (rc, got) =
                diff_inflate(&p, &stream, off, out_len, 0, &format!("err29/off{off}/o{out_len}"));
            assert_eq!(rc, 1);
            assert!(got.iter().all(|&b| b == 0));
        }
    }
}

/// Extra boundary: a *stored* block whose declared LEN is one past what the
/// remaining input can supply, and one below.
#[test]
fn err30_stored_len_off_by_one() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xE9);
    for real in [4usize, 8, 32] {
        let payload = rng.bytes(real);
        for delta in [-1i32, 1] {
            let mut stream = stored_stream(&payload, true);
            let claimed = (real as i32 + delta) as u16;
            stream[1] = (claimed & 0xFF) as u8;
            stream[2] = (claimed >> 8) as u8;
            let nlen = !claimed;
            stream[3] = (nlen & 0xFF) as u8;
            stream[4] = (nlen >> 8) as u8;
            // LEN one *below* the remaining bytes -> row 2; one above -> success
            // with a short/over copy.  Only require both libraries to agree.
            for off in 0..4usize {
                let _ = diff_inflate(
                    &p,
                    &stream,
                    off,
                    real + 8,
                    0,
                    &format!("err30/real{real}/d{delta}/off{off}"),
                );
            }
        }
    }
}

/// Extra boundary: literal exactly filling `out_bytes`, then one more literal.
#[test]
fn err31_literal_at_exact_boundary() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xEA);
    for n in 1..40usize {
        let items: Vec<Item> = (0..n).map(|_| Item::Lit(rng.u8())).collect();
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &items);
        let mut stream = w.bytes;
        stream.extend_from_slice(&[0u8; 4]);
        // exactly enough -> success
        let (rc, _) = diff_inflate(&p, &stream, 0, n, 0, &format!("err31/{n}/fit"));
        assert_eq!(rc, 1);
        // one byte short -> row 3
        expect_error(&p, &stream, 0, n - 1, E_OUT_SYMBOL, &format!("err31/{n}/short"));
    }
}
