//! Phase C — assert/abort rows from `ERRORS.md`.
//!
//! `c_src` is built with no `-DNDEBUG` (see `SYMBOLS.md`), so `assert()` is live
//! and an assert-tripping input makes the C library `abort()`. An abort cannot be
//! observed in-process, so each case is run in a **child process** (a re-exec of
//! this same test binary, selected by `CP_PROBE`) and the two sides are compared
//! by wait status.

mod common;

use common::deflate::*;
use common::*;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Probe cases
// ---------------------------------------------------------------------------

/// (case name, ERRORS.md row) — each runs one assert-tripping call.
const CASES: &[(&str, &str)] = &[
    ("g1_in_bytes_zero", "G1 / #16 assert(s->bits_left > 0)"),
    ("g4_in_bytes_negative", "G4 / #16 assert(s->bits_left > 0)"),
    ("r13_truncated_mid_symbol", "#13 assert(s->count >= n)"),
    ("r16_no_final_block", "#16 assert(s->bits_left > 0)"),
    ("r18_would_overflow", "#18 assert(!cp_would_overflow(...))"),
    ("r19_fixed_table_len_ge_16", "#19 assert(len < 16)"),
    ("r20_corrupt_huffman", "#20 assert((search>>len)==(key>>len))"),
    ("r11_stored_unaligned", "#11 assert(!(s->bits_left & 7))"),
];

fn run_case(imp: &Impl, case: &str) {
    match case {
        // ---- G1: in_bytes == 0 -> bits_left == 0 at the very first read -----
        "g1_in_bytes_zero" => {
            let mut inbuf = AlignedBuf::new(&[], 0, 8);
            let mut out = vec![0u8; 64];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), 0, out.as_mut_ptr() as *mut _, 64)
            };
            println!("ret={r}");
        }
        // ---- G4: negative in_bytes -> bits_left < 0 -------------------------
        "g4_in_bytes_negative" => {
            let mut inbuf = AlignedBuf::new(&[0u8; 16], 0, 8);
            let mut out = vec![0u8; 64];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), -4, out.as_mut_ptr() as *mut _, 64)
            };
            println!("ret={r}");
        }
        // ---- #13: stream truncated in the middle of a Huffman symbol -------
        "r13_truncated_mid_symbol" => {
            let toks: Vec<Tok> = (0..40).map(|i| Tok::Lit((i * 7) as u8)).collect();
            let mut bw = BitWriter::new();
            write_fixed_block(&mut bw, true, &toks);
            let mut s = bw.finish();
            s.truncate(3); // cut mid-stream, no end-of-block
            let mut inbuf = AlignedBuf::new(&s, 0, 8);
            let n = inbuf.len();
            let mut out = vec![0u8; 4096];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), n, out.as_mut_ptr() as *mut _, 4096)
            };
            println!("ret={r}");
        }
        // ---- #16: every block has bfinal == 0, so the reader runs off the end
        "r16_no_final_block" => {
            let mut bw = BitWriter::new();
            for _ in 0..3 {
                write_fixed_block(&mut bw, false, &[Tok::Lit(1)]);
            }
            let s = bw.finish();
            let mut inbuf = AlignedBuf::new(&s, 0, 8);
            let n = inbuf.len();
            let mut out = vec![0u8; 4096];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), n, out.as_mut_ptr() as *mut _, 4096)
            };
            println!("ret={r}");
        }
        // ---- #18: 1-byte input, so a 16-bit read cannot be satisfied -------
        "r18_would_overflow" => {
            // bfinal=1, btype=0 (stored) -> immediately reads 16 bits of LEN
            let mut inbuf = AlignedBuf::new(&[0x01u8], 0, 8);
            let mut out = vec![0u8; 64];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), 1, out.as_mut_ptr() as *mut _, 64)
            };
            println!("ret={r}");
        }
        // ---- #19: caller puts a code length >= 16 into cp_fixed_table -------
        "r19_fixed_table_len_ge_16" => {
            let mut lit = fixed_lit_lens();
            lit[0] = 20; // >= 16
            let dist = fixed_dist_lens();
            write_fixed_table(imp, &lit, &dist);
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(1, 2); // btype 1 -> cp_fixed -> cp_build over the bad table
            let s = {
                let mut v = bw.finish();
                v.extend([0u8; 8]);
                v
            };
            let mut inbuf = AlignedBuf::new(&s, 0, 8);
            let n = inbuf.len();
            let mut out = vec![0u8; 4096];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), n, out.as_mut_ptr() as *mut _, 4096)
            };
            println!("ret={r}");
        }
        // ---- #20: a fixed block whose bits decode to no valid code ----------
        "r20_corrupt_huffman" => {
            // A dynamic header declaring a single 1-bit literal code, then bits
            // that walk off the tree.
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(2, 2); // btype 2
            bw.bits(0, 5); // nlit = 257
            bw.bits(0, 5); // ndst = 1
            bw.bits(0, 4); // nlen = 4
            for _ in 0..4 {
                bw.bits(0, 3); // all code lengths zero -> empty CL tree
            }
            let s = {
                let mut v = bw.finish();
                v.extend([0xFFu8; 16]);
                v
            };
            let mut inbuf = AlignedBuf::new(&s, 0, 8);
            let n = inbuf.len();
            let mut out = vec![0u8; 4096];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), n, out.as_mut_ptr() as *mut _, 4096)
            };
            println!("ret={r}");
        }
        // ---- #11: stored block reached at a non-byte-aligned bit position ---
        "r11_stored_unaligned" => {
            // A fixed block first (leaves `count` not a multiple of 8), then a
            // stored block: `cp_stored`'s `count & 7` alignment is then wrong and
            // `cp_ptr` asserts on `bits_left & 7`.
            let mut bw = BitWriter::new();
            write_fixed_block(&mut bw, false, &[Tok::Lit(0xAB), Tok::Lit(0xCD)]);
            write_stored_block(&mut bw, true, &[1, 2, 3, 4, 5]);
            let s = bw.finish();
            let mut inbuf = AlignedBuf::new(&s, 0, 8);
            let n = inbuf.len();
            let mut out = vec![0u8; 4096];
            let r = unsafe {
                (imp.inflate)(inbuf.ptr(), n, out.as_mut_ptr() as *mut _, 4096)
            };
            println!("ret={r}");
        }
        other => panic!("unknown probe case {other}"),
    }
}

// ---------------------------------------------------------------------------
// Targeted search for ERRORS.md #11: `cp_ptr`'s assert(!(s->bits_left & 7)).
//
// Reaching it needs a stored block that is NOT the first block, entered after the
// bit reader's `bits_left ≡ count (mod 8)` invariant has been broken. The
// invariant breaks only when `cp_peak_bits` takes its `final_word` branch while
// `count` is not a multiple of 8 (that branch does `count += bits_left`, which
// double counts the already-buffered bits). So: emit a fixed-Huffman block with
// enough 8- and 9-bit literals to drain `count` into 1..15 exactly as the last
// whole word runs out, then a stored block.
//
// The search sweeps literal counts and stream lengths and reports which assert
// each candidate reaches. Candidates are run in a resumable child because a hit
// aborts the process.
// ---------------------------------------------------------------------------

const R11_MAX: usize = 320;

/// Structured candidate `k`.
///
/// Reaching `cp_ptr` (and thus its alignment assert) needs three things at once:
///   1. a stored block that is NOT the first block, so `count` is arbitrary;
///   2. `cp_peak_bits` to have taken its `final_word` branch while `count` was not
///      a multiple of 8. That branch does `count += bits_left`, double-counting the
///      already-buffered bits, which breaks the `bits_left ≡ count (mod 8)`
///      invariant and lets later reads drive `bits_left` negative
///      (`-13 & 7 == 3`, so the assert fires);
///   3. `LEN == (uint16_t)~NLEN` to still hold on bits that are partly past the
///      end of the input.
///
/// The sweep varies the leading block's literal count and code lengths, the raw
/// LEN/NLEN pair (including complementary pairs), and how many bytes are cut from
/// the end, which is what forces condition 2.
fn r11_candidate(k: usize) -> (Vec<u8>, usize) {
    let mut rng = Rng::new(0xC0FF_EE00_0000_0001 ^ (k as u64).wrapping_mul(0x9E37_79B9));
    let n_lit = ((k / 15) % 34) as usize;
    let variant = (k / 510) % 5;
    // `cut` is the fastest-varying axis: it is what forces the over-read, and
    // cutting exactly past the LEN field (leaving NLEN to read back as 0) is the
    // only way LEN == (uint16_t)~NLEN can hold on partially-past-the-end bits.
    let cut = k % 15;
    let align = (k / 2550) % 4;

    let mut bw = BitWriter::new();
    let toks: Vec<Tok> = (0..n_lit)
        .map(|i| {
            Tok::Lit(match variant {
                0 => i as u8,
                1 => 200u8.wrapping_add(i as u8),
                2 => 0xFF,
                3 => 0x00,
                _ => rng.byte(),
            })
        })
        .collect();
    write_fixed_block(&mut bw, false, &toks);

    // Second block: stored, with a LEN/NLEN pair written by hand so the sweep can
    // try complementary and non-complementary values plus all-ones patterns.
    bw.bits(1, 1);
    bw.bits(0, 2);
    bw.align_to_byte();
    let len: u16 = match (k / 5) % 4 {
        0 => 0xFFFF,
        1 => rng.next_u32() as u16,
        2 => (k % 64) as u16,
        _ => 0xFF00,
    };
    let nlen: u16 = if k % 2 == 0 { !len } else { rng.next_u32() as u16 };
    bw.raw_bytes(&len.to_le_bytes());
    bw.raw_bytes(&nlen.to_le_bytes());
    bw.raw_bytes(&[0xFF; 8]);

    let mut s = bw.finish();
    let keep = s.len().saturating_sub(cut).max(1);
    s.truncate(keep);
    (s, align)
}

#[test]
fn r11_search_child() {
    let start: usize = match std::env::var("CP_R11_START") {
        Ok(v) => v.parse().unwrap(),
        Err(_) => return,
    };
    use std::io::Write;
    let side = std::env::var("CP_SIDE").expect("CP_SIDE");
    let p = load_pair();
    let imp = if side == "c" { &p.c } else { &p.rs };
    let mut out = std::io::stdout();
    for k in start..R11_MAX {
        let (s, align) = r11_candidate(k);
        let r = run_inflate_ex(imp, &s, align, 4096, 70_000, 70_000);
        writeln!(out, "CASE {k} ret={} reason={:?}", r.ret, r.reason).unwrap();
        out.flush().unwrap();
    }
    writeln!(out, "DONE").unwrap();
}

/// Same resumable driver as the fuzz, over the structured r11 candidates.
fn r11_side(side: &str) -> (Vec<String>, Vec<(usize, String)>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut lines = Vec::new();
    let mut aborts: Vec<(usize, String)> = Vec::new();
    let mut start = 0usize;
    while start < R11_MAX {
        let out = Command::new("timeout")
            .arg("60")
            .arg(&exe)
            .args(["--exact", "r11_search_child", "--nocapture", "--test-threads=1"])
            .env("CP_R11_START", start.to_string())
            .env("CP_SIDE", side)
            .env("RUST_BACKTRACE", "0")
            .stdin(Stdio::null())
            .output()
            .expect("spawn r11 child");
        let text = String::from_utf8_lossy(&out.stdout);
        let mut last = None;
        let mut done = false;
        for l in text.lines() {
            if let Some(at) = l.find("CASE ") {
                let idx: usize = l[at + 5..].split_whitespace().next().unwrap().parse().unwrap();
                last = Some(idx);
                lines.push(l[at..].to_string());
            } else if l.contains("DONE") {
                done = true;
            }
        }
        if done {
            break;
        }
        let failed = last.map(|i| i + 1).unwrap_or(start);
        assert!(
            out.status.signal().is_some() || out.status.code() == Some(124),
            "[{side}] r11 child died without a signal: {:?}",
            out.status
        );
        aborts.push((
            failed,
            assertion_expr(&String::from_utf8_lossy(&out.stderr)),
        ));
        start = failed + 1;
    }
    (lines, aborts)
}

#[test]
fn err11_cp_ptr_alignment_assert() {
    if std::env::var("CP_R11_START").is_ok()
        || std::env::var("CP_PROBE").is_ok()
        || std::env::var("CP_FUZZ_START").is_ok()
    {
        return;
    }
    let hc = std::thread::spawn(|| r11_side("c"));
    let hr = std::thread::spawn(|| r11_side("rs"));
    let (cl, ca) = hc.join().unwrap();
    let (rl, ra) = hr.join().unwrap();

    let mut hist: std::collections::BTreeMap<&str, usize> = Default::default();
    for (_, e) in &ca {
        *hist.entry(e.as_str()).or_default() += 1;
    }
    println!(
        "\nERRORS.md #11 search: {} candidates returned, {} aborted",
        cl.len(),
        ca.len()
    );
    for (e, n) in &hist {
        println!("  {n:4}x  assert({e})");
    }

    // The differential requirement: identical results and identical abort sites.
    assert_eq!(ca, ra, "C and Rust diverge on the r11 candidate sweep");
    assert_eq!(cl.len(), rl.len(), "different survivor counts");
    for (a, b) in cl.iter().zip(rl.iter()) {
        assert_eq!(a, b, "r11 candidate divergence");
    }
    assert_eq!(cl.len() + ca.len(), R11_MAX, "candidates unaccounted for");

    let hit = ca
        .iter()
        .find(|(_, e)| e.contains("s->bits_left & 7"))
        .map(|(i, _)| *i);
    match hit {
        Some(i) => println!(
            "  -> ERRORS.md #11 REACHED at candidate {i}; C and Rust abort identically"
        ),
        None => panic!(
            "ERRORS.md #11 (cp_ptr alignment assert) was not reached by any of the \
             {R11_MAX} candidates, so the row is unverified. Asserts reached: {hist:?}"
        ),
    }
}

/// The child entry point. Re-exec'd as
/// `<test-bin> --exact probe_runner --nocapture` with `CP_PROBE` / `CP_SIDE` set.
#[test]
fn probe_runner() {
    let case = match std::env::var("CP_PROBE") {
        Ok(v) => v,
        Err(_) => return, // parent-side run: no-op
    };
    let side = std::env::var("CP_SIDE").expect("CP_SIDE");
    let p = load_pair();
    let imp = if side == "c" { &p.c } else { &p.rs };
    run_case(imp, &case);
    println!("survived");
}

#[derive(Debug, PartialEq, Eq)]
struct ProbeResult {
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    signal: Option<i32>,
    stdout: String,
    /// The assertion expression that fired, when the process died.
    assertion: Option<String>,
}

fn probe(case: &str, side: &str) -> ProbeResult {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "probe_runner", "--nocapture", "--test-threads=1"])
        .env("CP_PROBE", case)
        .env("CP_SIDE", side)
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .output()
        .expect("spawn probe child");
    ProbeResult {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                l.find("ret=")
                    .map(|a| l[a..].to_string())
                    .or_else(|| if l.contains("survived") { Some("survived".into()) } else { None })
            })
            .collect::<Vec<_>>()
            .join("|"),
        assertion: if out.status.signal().is_some() {
            Some(assertion_expr(&String::from_utf8_lossy(&out.stderr)))
        } else {
            None
        },
    }
}

#[test]
fn assert_rows_abort_identically() {
    if std::env::var("CP_PROBE").is_ok() {
        return; // child process: skip
    }
    let mut report = Vec::new();
    let mut bad = Vec::new();
    for (case, row) in CASES {
        let c = probe(case, "c");
        let r = probe(case, "rs");
        let same = c.signal == r.signal
            && c.code == r.code
            && c.stdout == r.stdout
            && c.assertion == r.assertion;
        report.push(format!(
            "{:28} {:44} C: sig={:?} out={:?} assert={:?}   Rust: sig={:?} out={:?} assert={:?}  {}",
            case,
            row,
            c.signal,
            c.stdout,
            c.assertion,
            r.signal,
            r.stdout,
            r.assertion,
            if same { "MATCH" } else { "DIVERGE" }
        ));
        if !same {
            bad.push(*case);
        }
    }
    println!("\n=== ERRORS.md assert rows (child-process differential) ===");
    for l in &report {
        println!("{l}");
    }
    assert!(
        bad.is_empty(),
        "assert-path divergence between C and Rust in: {bad:?}\n{}",
        report.join("\n")
    );
}

// ===========================================================================
// Abort-resilient fuzz over the FULL random input space (all block types).
//
// A live C `assert()` kills the process, so the fuzz body runs in a child that
// streams one line per case. When the child dies we record the aborting index
// and resume at the next one. Both sides must produce the same line sequence
// AND abort at exactly the same indices.
// ===========================================================================

/// Kept modest on purpose: every assert-tripping case kills a child process, and
/// the OS core-dump handler costs ~0.5 s per abort. Bulk volume comes from
/// `phase_c_errors::g_random_garbage_streams` (4000 in-process cases); this sweep
/// exists to cover the *abort* half of the input space, which cannot be observed
/// in-process at all.
const FUZZ_N: usize = 300;
const FUZZ_SEED: u64 = 0xF0FA_1234_9876_ABCD;
/// `cp_stored` will `memcpy` up to `LEN` (65535) bytes. Both buffers get that
/// much zeroed slack so the C's real overflow stays inside allocated, identical
/// memory instead of hashing unrelated heap contents (or segfaulting).
const FUZZ_SLACK: usize = 70_000;

/// One fuzz case: a random byte string interpreted as a deflate stream.
fn fuzz_case(rng: &mut Rng) -> (Vec<u8>, usize, i32) {
    let n = rng.range(1, 64) as usize;
    let mut s = rng.bytes(n);
    if !s.is_empty() && rng.bool_pct(50) {
        // bias towards each block type so all four `switch (btype)` arms are hit
        s[0] = (s[0] & !0b111) | (rng.below(8) as u8);
    }
    let align = rng.below(4);
    let out_bytes = rng.range(-16, 4096) as i32;
    (s, align, out_bytes)
}

#[test]
fn fuzz_child() {
    let start: usize = match std::env::var("CP_FUZZ_START") {
        Ok(v) => v.parse().unwrap(),
        Err(_) => return,
    };
    use std::io::Write;
    let side = std::env::var("CP_SIDE").expect("CP_SIDE");
    let p = load_pair();
    let imp = if side == "c" { &p.c } else { &p.rs };
    let mut rng = Rng::new(FUZZ_SEED);
    let mut out = std::io::stdout();
    for i in 0..FUZZ_N {
        let (s, align, out_bytes) = fuzz_case(&mut rng);
        if i < start {
            continue; // keep the RNG in lockstep, skip the call
        }
        let r = run_inflate_ex(imp, &s, align, out_bytes, FUZZ_SLACK, FUZZ_SLACK);
        let digest = r
            .out
            .iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
                (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
            });
        writeln!(
            out,
            "CASE {i} ret={} reason={:?} digest={:016x}",
            r.ret, r.reason, digest
        )
        .unwrap();
        out.flush().unwrap();
    }
    writeln!(out, "DONE").unwrap();
}

struct FuzzRun {
    lines: Vec<String>,
    /// (case index, the `Assertion \`...\`` text that fired)
    aborts: Vec<(usize, String)>,
}

/// Pull the assertion expression out of a glibc/`cp_assert!` diagnostic. glibc
/// writes "<prog>: <file>:<line>: <fn>: Assertion `EXPR' failed."; the Rust side
/// writes "lib.c: Assertion `EXPR' failed.". Comparing EXPR makes the two
/// libraries comparable and shows *which* assert each input reaches.
fn assertion_expr(stderr: &str) -> String {
    for l in stderr.lines().rev() {
        if let Some(at) = l.find("Assertion `") {
            let rest = &l[at + 11..];
            if let Some(end) = rest.find("' failed") {
                return rest[..end].to_string();
            }
        }
    }
    "<no assertion diagnostic>".to_string()
}

fn fuzz_side(side: &str) -> FuzzRun {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut lines: Vec<String> = Vec::new();
    let mut aborts: Vec<(usize, String)> = Vec::new();
    let mut start = 0usize;
    let mut spawns = 0usize;
    while start < FUZZ_N {
        spawns += 1;
        assert!(spawns < FUZZ_N + 16, "runaway respawn loop");
        let out = Command::new("timeout")
            .arg("60")
            .arg(&exe)
            .args(["--exact", "fuzz_child", "--nocapture", "--test-threads=1"])
            .env("CP_FUZZ_START", start.to_string())
            .env("CP_SIDE", side)
            .env("RUST_BACKTRACE", "0")
            .stdin(Stdio::null())
            .output()
            .expect("spawn fuzz child");
        let text = String::from_utf8_lossy(&out.stdout);
        let mut last = None;
        let mut done = false;
        for l in text.lines() {
            // The libtest harness writes "test fuzz_child ... " with no trailing
            // newline, so the child's first marker shares that line.
            if let Some(at) = l.find("CASE ") {
                let rest = &l[at + 5..];
                let idx: usize = rest.split_whitespace().next().unwrap().parse().unwrap();
                last = Some(idx);
                lines.push(l[at..].to_string());
            } else if l.contains("DONE") {
                done = true;
            }
        }
        if done {
            break;
        }
        // The child died. The aborting case is the one after the last emitted
        // line (a case is printed only after the call returns).
        let failed = match last {
            Some(i) => i + 1,
            None => start,
        };
        assert!(
            out.status.signal().is_some() || out.status.code() == Some(124),
            "[{side}] fuzz child exited without DONE and without a signal: {:?} (start={start})",
            out.status
        );
        let expr = assertion_expr(&String::from_utf8_lossy(&out.stderr));
        aborts.push((failed, expr));
        start = failed + 1;
    }
    FuzzRun { lines, aborts }
}

#[test]
fn fuzz_full_input_space_matches() {
    if std::env::var("CP_FUZZ_START").is_ok() || std::env::var("CP_PROBE").is_ok() {
        return;
    }
    // Run both sides concurrently; each spawns its own children.
    let hc = std::thread::spawn(|| fuzz_side("c"));
    let hr = std::thread::spawn(|| fuzz_side("rs"));
    let c = hc.join().expect("c fuzz thread");
    let r = hr.join().expect("rs fuzz thread");
    println!(
        "fuzz: {} cases returned normally, {} aborted (C); {} / {} (Rust)",
        c.lines.len(),
        c.aborts.len(),
        r.lines.len(),
        r.aborts.len()
    );
    // Which assert each side reaches, and how often.
    let mut hist: std::collections::BTreeMap<&str, usize> = Default::default();
    for (_, e) in &c.aborts {
        *hist.entry(e.as_str()).or_default() += 1;
    }
    println!("assert coverage reached by the fuzz sweep:");
    for (e, n) in &hist {
        println!("  {n:4}x  assert({e})");
    }
    assert_eq!(
        c.aborts, r.aborts,
        "C and Rust abort on different fuzz cases, or on different assertions\n  C:    {:?}\n  Rust: {:?}",
        c.aborts, r.aborts
    );
    assert_eq!(
        c.lines.len(),
        r.lines.len(),
        "different number of surviving fuzz cases"
    );
    for (a, b) in c.lines.iter().zip(r.lines.iter()) {
        assert_eq!(a, b, "fuzz case divergence");
    }
    assert_eq!(
        c.lines.len() + c.aborts.len(),
        FUZZ_N,
        "fuzz did not account for all {FUZZ_N} cases ({} returned + {} aborted)",
        c.lines.len(),
        c.aborts.len()
    );
    assert!(
        !c.aborts.is_empty(),
        "fuzz never reached an abort path — the sweep is not exercising the live C asserts"
    );
    assert!(!c.lines.is_empty(), "fuzz never reached a returning path");
}
