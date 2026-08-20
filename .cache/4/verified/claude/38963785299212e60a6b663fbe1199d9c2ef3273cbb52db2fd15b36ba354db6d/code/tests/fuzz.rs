//! Property-style differential fuzzing with fixed seeds.
//!
//! These tests do not target a specific `CONFIGS.md`/`ERRORS.md` row; they
//! exist to catch anything the enumerated rows miss.  Malformed inputs are
//! compared through the *paired* fork driver (`run_forked_pair`), because
//! `load_png_mem` lets `cp_inflate` write past the end of `img.pix` whenever
//! `bpp < 4`, so the two calls must not share a process (nor observe different
//! parent heap state).
//!
//! The exhaustive versions of these sweeps live in `tests/discover.rs` behind
//! `#[ignore]` (`cargo test --test discover -- --ignored`).

mod common;

use common::*;

const COLOUR_TYPES: [u8; 5] = [0, 2, 3, 4, 6];

/// Valid PNGs over the whole configuration space, checked in process against
/// both libraries *and* against the independent reference decoder.
#[test]
fn fuzz_valid_pngs() {
    let mut rng = Rng::new(0xC0FFEE);
    for iter in 0..600 {
        let ct = COLOUR_TYPES[rng.below(5) as usize];
        let w = rng.range(1, 48);
        let h = rng.range(1, 48);
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 23);
        s.deflate = match rng.below(5) {
            0 => Deflate::Fixed,
            1 => Deflate::Dynamic { rle: false },
            2 => Deflate::Dynamic { rle: true },
            3 => Deflate::Stored,
            _ => Deflate::Flate2(rng.range(1, 9)),
        };
        s.n_idat = rng.range(1, 6) as usize;
        s.empty_idats = rng.below(4) == 0;
        s.cmf = ((rng.below(8) as u8) << 4) | 0x08;
        s.flg = rng.u8() & !0x20;
        s.bad_crc = rng.below(3) == 0;
        s.iend = rng.below(4) != 0;
        s.ihdr_extra = rng.below(4) as usize;
        if rng.below(3) == 0 {
            s.pre_chunks = vec![Chunk::new(b"gAMA", rng.bytes(4))];
        }
        if ct == 3 {
            s.plte = Some(rng.bytes(256 * 3));
            if rng.below(2) == 0 {
                let tl = rng.range(0, 300) as usize;
                s.trns = Some(rng.bytes(tl));
            }
        }
        let tn = rng.below(64) as usize;
        s.trailing = rng.bytes(tn);

        let png = s.build();
        let label = format!("fuzz-valid iter={iter} ct={ct} {w}x{h}");
        let r = diff_png(&png, &label);
        if r.ok {
            if let Some(expect) = reference_rgba(&s) {
                assert_eq!(r.pixels, expect, "[{label}] vs the reference decoder");
            }
        } else {
            // the only legitimate rejection of a well formed file here is a
            // multi-stored-block DEFLATE stream (ERRORS.md row 26)
            assert_eq!(
                r.err.as_deref(),
                Some("Stored block extends beyond end of input stream."),
                "[{label}] unexpected rejection"
            );
        }
    }
}

/// Corrupted / truncated DEFLATE payloads inside otherwise valid PNGs.
#[test]
fn fuzz_corrupt_deflate_in_png() {
    let p = pair();
    let mut rng = Rng::new(0xBAD5EED);
    for iter in 0..200 {
        let ct = COLOUR_TYPES[rng.below(5) as usize];
        let (w, h) = (rng.range(1, 10), rng.range(1, 10));
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(6) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 5);
        let raw = s.raw();
        let mut d = s.deflate.run(&raw);
        match rng.below(4) {
            0 => {
                let n = rng.below(d.len().max(1) as u32) as usize;
                d.truncate(n);
            }
            1 => {
                if !d.is_empty() {
                    let i = rng.below(d.len() as u32) as usize;
                    d[i] ^= 1 << rng.below(8);
                }
            }
            2 => {
                let n = rng.range(0, 48) as usize;
                d = rng.bytes(n);
            }
            _ => {
                let i = rng.below(d.len().max(1) as u32) as usize;
                d.truncate(i);
                d.extend(rng.bytes(4));
            }
        }
        s.raw_zlib = Some(zlib_wrap(&d, 0x78, 0x9C, rng.u32()));
        let png = s.build();
        let buf = padded(&png);
        let len = png.len() as i32;
        let (a, b) = run_forked_pair(
            || {
                let r = call_load_png(&p.c, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(&r.w.to_le_bytes());
                v.extend_from_slice(&r.h.to_le_bytes());
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.pixels);
                v
            },
            || {
                let r = call_load_png(&p.rust, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(&r.w.to_le_bytes());
                v.extend_from_slice(&r.h.to_le_bytes());
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.pixels);
                v
            },
        );
        assert_eq!(
            a.outcome, b.outcome,
            "[fuzz-corrupt iter={iter}] mismatch\n  c stderr={:?}\n  rust stderr={:?}\n  png={:02X?}",
            a.stderr, b.stderr, png
        );
    }
}

/// Random raw byte strings handed straight to `cp_inflate`, over every input
/// alignment and a range of `out_bytes`.
#[test]
fn fuzz_raw_inflate() {
    let p = pair();
    let mut rng = Rng::new(0x1CEB00DA);
    for iter in 0..200 {
        let n = rng.range(0, 24) as usize;
        let d = rng.bytes(n);
        let align = rng.below(4) as usize;
        let out_bytes = [0i32, 1, 4, 64, 1024][rng.below(5) as usize];
        let (mut buf, off) = aligned_input(&d, align);
        let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
        let alloc = out_bytes as usize + 64;
        let (a, b) = run_forked_pair(
            || {
                let r = call_inflate(&p.c, ptr, n as i32, out_bytes, alloc);
                let mut v = r.rc.to_le_bytes().to_vec();
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.out);
                v
            },
            || {
                let r = call_inflate(&p.rust, ptr, n as i32, out_bytes, alloc);
                let mut v = r.rc.to_le_bytes().to_vec();
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.out);
                v
            },
        );
        assert_eq!(
            a.outcome, b.outcome,
            "[fuzz-raw iter={iter}] bytes={d:02X?} align={align} out_bytes={out_bytes}\n  c stderr={:?}\n  rust stderr={:?}",
            a.stderr, b.stderr
        );
    }
}

/// Corrupted PNG *containers* (mangled chunk headers, lengths and names) with a
/// valid DEFLATE payload.
#[test]
fn fuzz_corrupt_container() {
    let p = pair();
    let mut rng = Rng::new(0x5AFE_7E5Fu64);
    for iter in 0..200 {
        let ct = COLOUR_TYPES[rng.below(5) as usize];
        let (w, h) = (rng.range(1, 10), rng.range(1, 10));
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(5) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 5);
        let mut png = s.build();
        // flip a few bytes anywhere in the container
        for _ in 0..rng.range(1, 4) {
            let i = rng.below(png.len() as u32) as usize;
            png[i] ^= 1 << rng.below(8);
        }
        let len = match rng.below(4) {
            0 => png.len() as i32,
            1 => rng.below(png.len() as u32 + 1) as i32,
            2 => png.len() as i32 + rng.below(64) as i32,
            _ => -(rng.below(64) as i32),
        };
        let buf = padded(&png);
        let (a, b) = run_forked_pair(
            || {
                let r = call_load_png(&p.c, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(&r.w.to_le_bytes());
                v.extend_from_slice(&r.h.to_le_bytes());
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.pixels);
                v
            },
            || {
                let r = call_load_png(&p.rust, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(&r.w.to_le_bytes());
                v.extend_from_slice(&r.h.to_le_bytes());
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v.extend_from_slice(&r.pixels);
                v
            },
        );
        assert_eq!(
            a.outcome, b.outcome,
            "[fuzz-container iter={iter}] png_length={len}\n  c stderr={:?}\n  rust stderr={:?}\n  png={:02X?}",
            a.stderr, b.stderr, png
        );
    }
}
