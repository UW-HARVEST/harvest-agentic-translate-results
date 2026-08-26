//! Scratch: brute-force search for inputs that trip each live `assert()` of
//! `c_src/src/lib.c`, so `tests/errors.rs` can pin them down precisely.
//! Run with `cargo test --test discover -- --ignored --nocapture`.

mod common;

use common::*;
use std::collections::BTreeMap;

#[test]
#[ignore]
fn discover_asserts() {
    let p = pair();
    let mut found: BTreeMap<String, (Vec<u8>, usize, i32, i32)> = BTreeMap::new();
    let mut rng = Rng::new(0xF00D);

    // small, curated case set (one fork per case, so keep it in the thousands)
    let mut cases: Vec<(Vec<u8>, usize, i32, i32)> = Vec::new();
    // every block-header nibble, lengths 0..6, all alignments
    for len in 0..=6usize {
        for b0 in (0..=255u8).step_by(7) {
            let mut bytes = vec![b0];
            for i in 1..len {
                bytes.push((i as u8).wrapping_mul(37).wrapping_add(b0));
            }
            bytes.truncate(len);
            for align in [0usize, 1, 2, 3] {
                cases.push((bytes.clone(), align, len as i32, 64));
            }
        }
    }
    // random small streams
    for _ in 0..600 {
        let n = rng.range(1, 14) as usize;
        let bytes = rng.bytes(n);
        cases.push((
            bytes,
            rng.below(4) as usize,
            n as i32,
            [0i32, 1, 4, 64, 512][rng.below(5) as usize],
        ));
    }
    // fixed-Huffman prefix followed by a stored block (drives cp_ptr's
    // `assert(!(s->bits_left & 7))`)
    for nlit in 0..12usize {
        for slen in 0..6usize {
            let mut bw = BitWriter::new();
            let toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
            write_fixed_block(&mut bw, &toks, false);
            let data: Vec<u8> = (0..slen).map(|i| 0xB0 + i as u8).collect();
            write_stored_block(&mut bw, &data, true, None);
            let d = bw.finish();
            for align in 0..4usize {
                cases.push((d.clone(), align, d.len() as i32, 512));
            }
        }
    }
    // negative / oversized lengths
    for n in [-1i32, -8, i32::MIN, 5, 100] {
        for align in 0..4usize {
            cases.push((vec![1, 2, 3, 4], align, n, 16));
        }
    }
    // truncated valid streams
    for nlit in 1..8usize {
        let toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8 + 65)).collect();
        for d in [deflate_fixed(&toks), deflate_dynamic(&toks, true)] {
            for cut in 0..d.len() {
                for align in 0..4usize {
                    cases.push((d[..cut].to_vec(), align, cut as i32, 64));
                }
            }
        }
    }
    println!("cases: {}", cases.len());

    let mut mismatches = 0;
    for (bytes, align, in_bytes, out_bytes) in cases {
        let (mut buf, off) = aligned_input(&bytes, align);
        let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
        let alloc = (out_bytes.max(0) as usize) + 64;
        let (a, b) = run_forked_pair(
            || {
                let r = call_inflate(&p.c, ptr, in_bytes, out_bytes, alloc);
                let mut v = r.rc.to_le_bytes().to_vec();
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v
            },
            || {
                let r = call_inflate(&p.rust, ptr, in_bytes, out_bytes, alloc);
                let mut v = r.rc.to_le_bytes().to_vec();
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v
            },
        );
        if a.outcome != b.outcome {
            mismatches += 1;
            if mismatches < 20 {
                println!(
                    "MISMATCH bytes={bytes:02X?} align={align} in={in_bytes} out={out_bytes}\n  c={:?} stderr={:?}\n  rust={:?} stderr={:?}",
                    a.outcome, a.stderr, b.outcome, b.stderr
                );
            }
        }
        if let Some(x) = a.assertion() {
            found
                .entry(x)
                .or_insert((bytes.clone(), align, in_bytes, out_bytes));
        }
    }

    println!("\n==== assertions reachable through cp_inflate ====");
    for (k, v) in &found {
        println!("{k}\n    bytes={:02X?} align={} in={} out={}", v.0, v.1, v.2, v.3);
    }
    println!("==== total mismatches: {mismatches} ====");
    assert_eq!(mismatches, 0, "C and Rust disagreed on {mismatches} inputs");
}

#[test]
#[ignore]
fn discover_png_asserts() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut rng = Rng::new(0xBEEF);
    let mut mismatches = 0;

    // valid container, corrupted DEFLATE payload
    for iter in 0..900 {
        let ct = [0u8, 2, 3, 4, 6][rng.below(5) as usize];
        let (w, h) = (rng.range(1, 8), rng.range(1, 8));
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(6) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 3);
        let raw = s.raw();
        let mut d = s.deflate.run(&raw);
        // corrupt / truncate
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
                let n = rng.range(0, 40) as usize;
                d = rng.bytes(n);
            }
            _ => {
                let i = rng.below(d.len().max(1) as u32) as usize;
                d.truncate(i);
                d.extend(rng.bytes(4));
            }
        }
        s.raw_zlib = Some(zlib_wrap(&d, 0x78, 0x9C, 0));
        let png = s.build();
        let p = pair();
        let buf = padded(&png);
        let len = png.len() as i32;
        let (a, b) = run_forked_pair(
            || {
                let r = call_load_png(&p.c, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v
            },
            || {
                let r = call_load_png(&p.rust, &buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v
            },
        );
        if a.outcome != b.outcome {
            mismatches += 1;
            if mismatches < 15 {
                println!(
                    "PNG MISMATCH iter={iter}\n  c={:?} stderr={:?}\n  rust={:?} stderr={:?}",
                    a.outcome, a.stderr, b.outcome, b.stderr
                );
            }
        }
        if let Some(x) = a.assertion() {
            found.entry(x).or_insert(format!("iter={iter}"));
        }
    }
    println!("\n==== assertions reachable through load_png_mem ====");
    for (k, v) in &found {
        println!("{k}   ({v})");
    }
    println!("==== total png mismatches: {mismatches} ====");
    assert_eq!(mismatches, 0);
}

// ---------------------------------------------------------------------------
// batched searcher: run many cases inside ONE fork, reporting the index of
// each case before running it, so an abort pinpoints the offending case.
// ---------------------------------------------------------------------------

type Case = (Vec<u8>, usize, i32, i32);

fn run_batch(im: &Impl, cases: &[Case], start: usize) -> (usize, bool, String) {
    // returns (index_of_last_started_case, aborted, stderr)
    let mut errp = [0i32; 2];
    let mut datap = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(errp.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { libc::pipe(datap.as_mut_ptr()) }, 0);
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        unsafe {
            libc::close(errp[0]);
            libc::close(datap[0]);
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            libc::dup2(errp[1], 2);
            libc::alarm(60);
        }
        for i in start..cases.len() {
            let idx = (i as u32).to_le_bytes();
            unsafe {
                libc::write(datap[1], idx.as_ptr() as *const std::ffi::c_void, 4);
            }
            let (bytes, align, in_bytes, out_bytes) = &cases[i];
            let (mut buf, off) = aligned_input(bytes, *align);
            let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
            let alloc = (out_bytes.max(&0).max(&0)).unsigned_abs() as usize + 4096;
            let _ = call_inflate(im, ptr, *in_bytes, *out_bytes, alloc);
        }
        unsafe {
            let done = u32::MAX.to_le_bytes();
            libc::write(datap[1], done.as_ptr() as *const std::ffi::c_void, 4);
            libc::close(datap[1]);
            libc::_exit(0)
        };
    }
    unsafe {
        libc::close(errp[1]);
        libc::close(datap[1]);
    }
    let mut idx = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = unsafe { libc::read(datap[0], buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        idx.extend_from_slice(&buf[..n as usize]);
    }
    let mut err = Vec::new();
    loop {
        let n = unsafe { libc::read(errp[0], buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        err.extend_from_slice(&buf[..n as usize]);
    }
    unsafe {
        libc::close(datap[0]);
        libc::close(errp[0]);
    }
    let mut status = 0;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    let last = if idx.len() >= 4 {
        u32::from_le_bytes(idx[idx.len() - 4..].try_into().unwrap())
    } else {
        u32::MAX
    };
    let died = libc::WIFSIGNALED(status);
    let last_idx = if last == u32::MAX { cases.len() } else { last as usize };
    (last_idx, died, String::from_utf8_lossy(&err).into_owned())
}

/// `[fixed block, BFINAL=0][stored block]` combinations, looking for
/// `cp_ptr`'s `assert(!(s->bits_left & 7))`.
#[test]
#[ignore]
fn search_cp_ptr_assert() {
    let p = pair();
    let mut cases: Vec<Case> = Vec::new();
    let mut meta: Vec<String> = Vec::new();
    for pattern in 0u32..4 {
        for nlit in 0..40usize {
            for l in 0..20u16 {
                for ndata in 0..4usize {
                    for align in 0..4usize {
                        let mut bw = BitWriter::new();
                        let toks: Vec<Tok> = (0..nlit)
                            .map(|i| {
                                Tok::Lit(match pattern {
                                    0 => 0,
                                    1 => 200,
                                    2 => if i % 2 == 0 { 0 } else { 200 },
                                    _ => (i * 29) as u8,
                                })
                            })
                            .collect();
                        write_fixed_block(&mut bw, &toks, false);
                        // stored header by hand: BFINAL=1, BTYPE=00
                        bw.bits(1, 1);
                        bw.bits(0, 2);
                        bw.align();
                        bw.raw(&l.to_le_bytes());
                        bw.raw(&(!l).to_le_bytes());
                        let d: Vec<u8> = (0..ndata).map(|i| 0xE0 + i as u8).collect();
                        bw.raw(&d);
                        let s = bw.finish();
                        meta.push(format!(
                            "pattern={pattern} nlit={nlit} LEN={l} ndata={ndata} align={align}"
                        ));
                        cases.push((s.clone(), align, s.len() as i32, 4096));
                    }
                }
            }
        }
    }
    println!("cases: {}", cases.len());
    let mut start = 0usize;
    let mut hits = 0;
    while start < cases.len() {
        let (last, died, err) = run_batch(&p.c, &cases, start);
        if !died {
            break;
        }
        let a = err
            .find("Assertion `")
            .map(|i| {
                let i = i + 11;
                let j = err[i..].find("' failed").unwrap_or(0);
                err[i..i + j].to_string()
            })
            .unwrap_or_else(|| format!("signal, stderr={err:?}"));
        if a.contains("bits_left & 7") {
            println!("FOUND cp_ptr assert at case {last}: {}", meta[last]);
            println!("   bytes={:02X?} align={} in={}", cases[last].0, cases[last].1, cases[last].2);
            hits += 1;
            if hits >= 3 {
                return;
            }
        }
        start = last + 1;
    }
    println!("cp_ptr assert hits: {hits}");
}

/// Broad randomised search over `[fixed|dynamic block][stored header][tail]`
/// streams and raw byte strings, reporting every distinct assertion found.
#[test]
#[ignore]
fn search_all_asserts() {
    let p = pair();
    let mut rng = Rng::new(0x5EAF00D);
    let mut cases: Vec<Case> = Vec::new();
    let mut meta: Vec<String> = Vec::new();

    // A) [fixed block, BFINAL=0][stored header][tail]
    for _ in 0..20_000 {
        let nlit = rng.below(80) as usize;
        let pat = rng.below(4);
        let mut bw = BitWriter::new();
        let toks: Vec<Tok> = (0..nlit)
            .map(|i| {
                Tok::Lit(match pat {
                    0 => 0,
                    1 => 200,
                    2 => if i % 2 == 0 { 0 } else { 200 },
                    _ => rng.u8(),
                })
            })
            .collect();
        if rng.below(4) == 0 {
            write_dynamic_block(&mut bw, &toks, false, rng.below(2) == 0);
        } else {
            write_fixed_block(&mut bw, &toks, false);
        }
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        let len = [0u16, 1, 2, 3, 8, 0x0FFF, 0x7FFF, 0xFFFF][rng.below(8) as usize];
        let nlen = if rng.below(2) == 0 { !len } else { rng.u32() as u16 };
        bw.raw(&len.to_le_bytes());
        bw.raw(&nlen.to_le_bytes());
        let ntail = rng.below(9) as usize;
        let tail = rng.bytes(ntail);
        bw.raw(&tail);
        let s = bw.finish();
        let align = rng.below(4) as usize;
        meta.push(format!("A nlit={nlit} pat={pat} LEN={len:#06X} NLEN={nlen:#06X} ntail={ntail} align={align}"));
        cases.push((s.clone(), align, s.len() as i32, 4096));
    }
    // B) raw random byte strings
    for _ in 0..20_000 {
        let n = rng.range(0, 18) as usize;
        let s = rng.bytes(n);
        let align = rng.below(4) as usize;
        meta.push(format!("B n={n} align={align}"));
        cases.push((s, align, n as i32, [0i32, 1, 16, 4096][rng.below(4) as usize]));
    }
    println!("cases: {}", cases.len());

    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    let mut start = 0usize;
    let mut deaths = 0;
    while start < cases.len() {
        let (last, died, err) = run_batch(&p.c, &cases, start);
        if !died {
            break;
        }
        deaths += 1;
        if deaths > 200 {
            println!("stopping after 200 deaths");
            break;
        }
        let a = err
            .find("Assertion `")
            .map(|i| {
                let i = i + 11;
                let j = err[i..].find("' failed").unwrap_or(0);
                err[i..i + j].to_string()
            })
            .unwrap_or_else(|| "NON-ASSERT SIGNAL".to_string());
        found.entry(a).or_insert(last);
        start = last + 1;
    }
    println!("deaths: {deaths}");
    println!("==== distinct assertions ====");
    for (k, &i) in &found {
        println!(
            "{k}\n    {}\n    bytes={:02X?} align={} in={} out={}",
            meta[i], cases[i].0, cases[i].1, cases[i].2, cases[i].3
        );
    }
}

/// Analytically constructed input for `cp_ptr`'s `assert(!(s->bits_left & 7))`.
///
/// Derivation: the only way `bits_left` becomes non-byte-aligned is a refill
/// from `s->final_word`, which adds `bits_left` (not 32) to `count`.  Writing
/// `c0` for `count` at that refill, `lb` for `last_bytes` and `f=0`, one gets
/// `R = bits_left = 8*lb + c0`, `count = 2*c0 + 8*lb`, and after the stored
/// header `bits_left ≡ -c0 (mod 8)` — independent of everything else.  So the
/// assert fires exactly when `c0 % 8 != 0`.  Feasibility (`bits_left > 0` and
/// `count >= 16` at the NLEN read) forces `lb = 3`, `c0 = 9`, and the refill
/// must happen at the `cp_decode` of the end-of-block symbol.
///
/// `c0 = 9` needs `consumed = 32*wc - 9` at that decode, i.e. 3 header bits +
/// `32*wc - 12` token bits, with every earlier decode at `count >= 16`.
/// `wc = 2`: `8a + 9b = 52` -> `a = 2` eight-bit literals + `b = 4` nine-bit
/// literals.  `in_bytes = 4*wc + lb = 11`.
pub fn cp_ptr_assert_stream() -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.bits(0, 1); // BFINAL = 0
    bw.bits(1, 2); // BTYPE  = 01 (fixed)
    bw.code(0x30 + 0, 8); // literal 0     (8 bits)
    bw.code(0x30 + 1, 8); // literal 1     (8 bits)
    for _ in 0..4 {
        bw.code(0x190 + (200 - 144), 9); // literal 200 (9 bits)
    }
    bw.code(0x00, 7); // end of block  (7 bits)  <- refill happens here, c0 = 9
    bw.bits(1, 1); // BFINAL = 1
    bw.bits(0, 2); // BTYPE  = 00 (stored)
    // count == 32 here, so `cp_read_bits(s, s->count & 7)` reads 0 bits
    bw.bits(0xFFFF, 16); // LEN  = 0xFFFF
    bw.bits(0, 7); // the 7 remaining real bits of NLEN; the top 9 are phantom
                   // zeros, so NLEN reads back as 0 == (uint16_t)~0xFFFF
    let v = bw.finish();
    assert_eq!(v.len(), 11, "the derivation assumes in_bytes == 11");
    v
}

#[test]
#[ignore]
fn verify_cp_ptr_assert() {
    let p = pair();
    let s = cp_ptr_assert_stream();
    println!("stream = {:02X?} ({} bytes)", s, s.len());
    for align in 0..4usize {
        let (mut buf, off) = aligned_input(&s, align);
        let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
        let a = run_forked_capture(|| {
            let r = call_inflate(&p.c, ptr, s.len() as i32, 4096, 4096 + 64);
            r.rc.to_le_bytes().to_vec()
        });
        let b = run_forked_capture(|| {
            let r = call_inflate(&p.rust, ptr, s.len() as i32, 4096, 4096 + 64);
            r.rc.to_le_bytes().to_vec()
        });
        println!(
            "align={align}: C {:?} assert={:?} | RUST {:?}",
            a.outcome,
            a.assertion(),
            b.outcome
        );
    }
}

/// Row 41: `cp_build`'s `assert(len < 16)`.  A code length >= 16 can only come
/// from the *public writable* `cp_fixed_table`, so poke it (inside the fork, so
/// the parent's copy stays pristine).
#[test]
#[ignore]
fn verify_cp_build_assert() {
    let p = pair();
    let toks = vec![Tok::Lit(b'x')];
    let d = deflate_fixed(&toks);
    for poke in [16u8, 17, 31, 40, 47] {
        let (mut buf, off) = aligned_input(&d, 0);
        let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
        let a = run_forked_capture(|| {
            unsafe { *p.c.fixed_table = poke };
            let r = call_inflate(&p.c, ptr, d.len() as i32, 64, 128);
            r.rc.to_le_bytes().to_vec()
        });
        let b = run_forked_capture(|| {
            unsafe { *p.rust.fixed_table = poke };
            let r = call_inflate(&p.rust, ptr, d.len() as i32, 64, 128);
            r.rc.to_le_bytes().to_vec()
        });
        println!(
            "poke={poke}: C {:?} assert={:?} | RUST {:?}",
            a.outcome,
            a.assertion(),
            b.outcome
        );
    }
}

/// Row 38: `cp_read_bits`'s `assert(num_bits_to_read <= 32)`.  Every argument
/// is either a literal <= 16 or an entry of the *public writable*
/// `cp_len_extra_bits` / `cp_dist_extra_bits` tables, so poke those.
#[test]
#[ignore]
fn verify_read_bits_range_assert() {
    let p = pair();
    // a match of length 3 uses cp_len_extra_bits[0]; distance 1 uses
    // cp_dist_extra_bits[0]
    let toks = vec![Tok::Lit(b'q'), Tok::Match { len: 3, dist: 1 }];
    let d = deflate_fixed(&toks);
    for (which, poke) in [("len", 33u8), ("len", 64), ("len", 255), ("dist", 33), ("dist", 200)] {
        let (mut buf, off) = aligned_input(&d, 0);
        let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
        let go = |im: &Impl| {
            let t = if which == "len" {
                im.len_extra_bits
            } else {
                im.dist_extra_bits
            };
            unsafe { *t = poke };
            let r = call_inflate(im, ptr, d.len() as i32, 4096, 4096 + 64);
            r.rc.to_le_bytes().to_vec()
        };
        let a = run_forked_capture(|| go(&p.c));
        let b = run_forked_capture(|| go(&p.rust));
        println!(
            "{which}_extra_bits[0]={poke}: C {:?} assert={:?} | RUST {:?}",
            a.outcome,
            a.assertion(),
            b.outcome
        );
    }
}

/// Reproduce the row-34 sweep and print every case where C and Rust differ.
#[test]
#[ignore]
fn repro_row34() {
    let p = pair();
    let mut rng = Rng::new(0x3401);
    let mut shown = 0;
    for iter in 0..3 {
        let n = rng.range(1, 40) as usize;
        let d = rng.bytes(n);
        for align in 0..1usize {
            let (mut buf, off) = aligned_input(&d, align);
            let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
            let a = run_forked_capture(|| {
                let r = call_inflate(&p.c, ptr, n as i32, 256, 256 + 64);
                r.rc.to_le_bytes().to_vec()
            });
            let b = run_forked_capture(|| {
                let r = call_inflate(&p.rust, ptr, n as i32, 256, 256 + 64);
                r.rc.to_le_bytes().to_vec()
            });
            if a.outcome != b.outcome {
                shown += 1;
                println!(
                    "iter={iter} align={align} n={n} bytes={:02X?}\n  C   ={:?} stderr={:?}\n  RUST={:?} stderr={:?}",
                    d, a.outcome, a.stderr, b.outcome, b.stderr
                );
                if shown >= 6 {
                    return;
                }
            }
        }
    }
    println!("divergences: {shown}");
}

/// Rebuild the iter=226 case of `discover_png_asserts` and dump it.
#[test]
#[ignore]
fn repro_png226() {
    let mut rng = Rng::new(0xBEEF);
    for iter in 0..900 {
        let ct = [0u8, 2, 3, 4, 6][rng.below(5) as usize];
        let (w, h) = (rng.range(1, 8), rng.range(1, 8));
        let mut s = Spec::new(w, h, ct);
        s.filters = (0..h).map(|_| rng.below(6) as u8).collect();
        s.payload = rng.bytes((w * h) as usize * bpp_of(ct) + 3);
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
                let n = rng.range(0, 40) as usize;
                d = rng.bytes(n);
            }
            _ => {
                let i = rng.below(d.len().max(1) as u32) as usize;
                d.truncate(i);
                d.extend(rng.bytes(4));
            }
        }
        s.raw_zlib = Some(zlib_wrap(&d, 0x78, 0x9C, 0));
        if iter != 226 {
            continue;
        }
        let png = s.build();
        println!("ct={ct} {w}x{h} filters={:?}", s.filters);
        println!("deflate({} bytes) = {:02X?}", d.len(), d);
        println!("png({} bytes) = {:02X?}", png.len(), png);
        // what does cp_inflate alone do?  (out buffer owned by the test)
        let pix_bytes = ((w + 1) * h * 4) as i32;
        let out_size_bpp = ((w + 1) * h * bpp_of(ct) as u32) as i32;
        println!("pix_bytes={pix_bytes} out_size_bpp={out_size_bpp} in_bytes={}", d.len());
        let r = diff_inflate_abort(&d, d.len() as i32, 2, pix_bytes, "png226 inflate only");
        println!("cp_inflate alone: {:?} stderr={:?}", r.outcome, r.stderr);

        // Is the divergence caused by the library, or by the *order* in which
        // the two children are forked?  Run C twice, then Rust twice, then C
        // and Rust in the opposite order.
        let p2 = pair();
        let buf = padded(&png);
        let len = png.len() as i32;
        fn run(im: &'static Impl, buf: &'static [u8], len: i32) -> ForkResult {
            run_forked_capture(move || {
                let r = call_load_png(im, buf, len);
                let mut v = vec![r.ok as u8];
                v.extend_from_slice(r.err.unwrap_or_default().as_bytes());
                v
            })
        }
        let buf: &'static [u8] = Box::leak(buf.into_boxed_slice());
        for (label, im) in [
            ("C   #1", &p2.c),
            ("C   #2", &p2.c),
            ("C   #3", &p2.c),
            ("RUST#1", &p2.rust),
            ("RUST#2", &p2.rust),
            ("RUST#3", &p2.rust),
        ] {
            let r = run(im, buf, len);
            println!("{label}: {:?} stderr={:?}", r.outcome, r.stderr.trim());
        }
        return;
    }
}
