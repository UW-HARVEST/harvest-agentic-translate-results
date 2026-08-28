//! Phase C — ERRORS.md rows 11, 13, 14, 16, 18, 19, 20, 25, 26: the `assert()`
//! paths.  A failing `assert()` raises `SIGABRT`, which would take the test
//! process down, so each scenario is run twice in a *child* process — once
//! against the C `.so`, once against the Rust `.so` — and the
//! `(exit code, signal, stdout, stderr)` triple must match.

mod common;

use common::fork::*;
use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// child side
// ---------------------------------------------------------------------------

const ENV_CASE: &str = "CP_ABORT_CASE";
const ENV_LIB: &str = "CP_ABORT_LIB";

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Load exactly one of the two libraries.
fn single_lib(which: &str) -> Lib {
    let p = if which == "c" { c_so_path() } else { rust_so_path() };
    // `Lib::open` is private to the module; re-implement with libloading.
    let lib = unsafe { libloading::Library::new(&p) }.expect("dlopen");
    Lib { lib, name: if which == "c" { "C" } else { "Rust" } }
}

fn write_bytes<T: Copy>(l: &Lib, sym: &[u8], v: &[T]) {
    let p: *mut T = l.data(sym);
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), p, v.len()) };
}

/// A single deterministic scenario.  Returns a printable summary; if the
/// library aborts, this never returns.
fn run_case(l: &Lib, case: &str) -> String {
    let f = l.cp_inflate();

    // helper closure
    let call = |stream: &[u8], in_off: usize, in_bytes: i32, out_len: usize, out_bytes: i32| {
        let mut i = AlignedBuf::new(stream, in_off);
        let mut o = AlignedBuf::zeroed(out_len, 0);
        let rc = unsafe {
            f(
                i.ptr() as *mut std::ffi::c_void,
                in_bytes,
                o.ptr() as *mut std::ffi::c_void,
                out_bytes,
            )
        };
        let reason = l
            .error_reason()
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_else(|| "<null>".into());
        format!("rc={rc} out={:016x} reason={reason}", fnv1a(o.all_bytes()))
    };

    match case {
        // --- ERRORS row 16 -------------------------------------------------
        "in_bytes_zero" => call(&[0u8; 8], 0, 0, 8, 8),
        // --- ERRORS row 26 -------------------------------------------------
        "null_in" => {
            let rc = unsafe { f(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0) };
            format!("rc={rc}")
        }
        // --- ERRORS row 25 -------------------------------------------------
        // `in_bytes < 0`: `last_bytes = in_bytes & 3` makes the final-word loop
        // read *before* the buffer, so hand it an offset of 8 to keep the read
        // inside our own allocation.
        "in_bytes_negative_1" => call(&[0xAAu8; 16], 8, -1, 16, 16),
        "in_bytes_negative_4" => call(&[0x55u8; 16], 8, -4, 16, 16),
        "in_bytes_negative_big" => call(&[0x37u8; 16], 8, -1000, 16, 16),
        // --- ERRORS row 13 (`cp_consume_bits`: count >= num_bits_to_read) ---
        "consume_more_than_buffered" => call(&[0x01u8, 0x00], 0, 2, 16, 16),
        // --- ERRORS row 20 (`cp_decode`: no matching code) -----------------
        // BTYPE=2 with HLIT=HDIST=HCLEN=0 and 12 zero code-length bits leaves
        // an *empty* code-length tree, so `cp_decode` reads `tree[-1]`.
        "decode_empty_tree" => call(&[0x05u8, 0, 0, 0, 0, 0, 0, 0], 0, 8, 16, 16),
        // truncated fixed stream: the last code cannot be completed
        "decode_truncated" => {
            let mut w = BitWriter::new();
            let items: Vec<Item> = (0..40).map(|i| Item::Lit((i * 7) as u8)).collect();
            write_fixed_block(&mut w, true, &items);
            let s = &w.bytes[..3];
            call(s, 0, s.len() as i32, 64, 64)
        }
        // --- ERRORS row 14 (`cp_read_bits`: num_bits_to_read <= 32) --------
        "read_bits_gt_32" => {
            write_bytes(l, b"cp_len_extra_bits\0", &[33u8; 31]);
            let mut w = BitWriter::new();
            let lit = Huff::new(fixed_lit_lens());
            w.bit(1);
            w.bits_lsb(1, 2);
            lit.put(&mut w, 0x41);
            lit.put(&mut w, 257);
            let mut s = w.bytes.clone();
            s.resize(4, 0);
            call(&s, 0, 4, 64, 64)
        }
        // --- ERRORS row 18 (`cp_read_bits`: !cp_would_overflow) ------------
        "would_overflow" => {
            write_bytes(l, b"cp_len_extra_bits\0", &[30u8; 31]);
            let mut w = BitWriter::new();
            let lit = Huff::new(fixed_lit_lens());
            w.bit(1);
            w.bits_lsb(1, 2);
            lit.put(&mut w, 0x41);
            lit.put(&mut w, 257);
            let mut s = w.bytes.clone();
            s.resize(4, 0);
            call(&s, 0, 4, 64, 64)
        }
        // --- ERRORS row 19 (`cp_build`: len < 16) --------------------------
        "code_length_ge_16" => {
            let mut t = fixed_lit_lens();
            t[0] = 16;
            let mut all = t;
            all.extend_from_slice(&fixed_dist_lens());
            write_bytes(l, b"cp_fixed_table\0", &all);
            let mut s = vec![0x03u8]; // BFINAL=1, BTYPE=1
            s.extend_from_slice(&[0u8; 7]);
            call(&s, 0, 8, 64, 64)
        }
        "code_length_255" => {
            let mut t = fixed_lit_lens();
            t[7] = 255;
            let mut all = t;
            all.extend_from_slice(&fixed_dist_lens());
            write_bytes(l, b"cp_fixed_table\0", &all);
            let mut s = vec![0x03u8];
            s.extend_from_slice(&[0u8; 7]);
            call(&s, 0, 8, 64, 64)
        }
        // --- ERRORS row 11 (`cp_ptr`: !(bits_left & 7)) --------------------
        // Fixed block that immediately ends, followed by a stored block, with
        // `first_bytes == 2` so `cp_peak_bits` takes the `final_word` branch at
        // a bit position that is not byte aligned.  `bits_left` then goes
        // negative (-5) and `bits_left & 7 == 3`.
        "cp_ptr_unaligned" => call(&[0x02u8, 0xE4, 0xFF, 0x1F, 0x00], 2, 5, 64, 64),
        other => {
            if let Some(rest) = other.strip_prefix("fuzz:") {
                let idx: u64 = rest.parse().expect("fuzz index");
                let (stream, in_off, out_len) = fuzz_input(idx);
                return call(&stream, in_off, stream.len() as i32, 128 * 1024, out_len as i32);
            }
            panic!("unknown case {other}");
        }
    }
}

/// Deterministic pseudo-random `cp_inflate` input #`idx`.
fn fuzz_input(idx: u64) -> (Vec<u8>, usize, usize) {
    let mut rng = Rng::new(SEED ^ 0xF0_0000 ^ idx.wrapping_mul(0x9E37_79B9));
    let n = rng.range(1, 24) as usize;
    let stream = rng.bytes(n);
    let in_off = rng.below(4) as usize;
    let out_len = rng.below(33) as usize;
    (stream, in_off, out_len)
}

/// The child entry point.  Does nothing unless the driver set `CP_ABORT_CASE`.
#[test]
fn abort_child_worker() {
    no_core_dumps();
    let case = match std::env::var(ENV_CASE) {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = std::env::var(ENV_LIB).unwrap_or_else(|_| "c".into());
    let l = single_lib(&which);
    arm_child_timeout();
    let summary = run_case(&l, &case);
    println!("SUMMARY {summary}");
    // Flush and leave without running the rest of libtest's bookkeeping.
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// driver side
// ---------------------------------------------------------------------------

struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    summary: Option<String>,
    assertion: Option<String>,
    stderr: String,
}

fn run_child(case: &str, which: &str) -> Outcome {
    no_core_dumps();
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "abort_child_worker", "--nocapture", "--test-threads", "1"])
        .env(ENV_CASE, case)
        .env(ENV_LIB, which)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let summary = stdout
        .lines()
        .find(|l| l.starts_with("SUMMARY "))
        .map(|l| l["SUMMARY ".len()..].to_string());
    // glibc: "<prog>: <file>:<line>: <func>: Assertion `<expr>' failed."
    let assertion = stderr
        .lines()
        .find(|l| l.contains("Assertion `"))
        .map(|l| {
            // strip the leading "<prog>: " so the two children (same binary)
            // cannot differ for irrelevant reasons
            match l.find(": /") {
                Some(i) => l[i + 2..].to_string(),
                None => l.to_string(),
            }
        });
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        summary,
        assertion,
        stderr,
    }
}

/// Run one scenario against both libraries and require identical observable
/// behaviour.  Returns the assertion text (if the process aborted).
fn diff_case(case: &str) -> Option<String> {
    let c = run_child(case, "c");
    let r = run_child(case, "rust");
    assert_eq!(
        (c.signal, c.code),
        (r.signal, r.code),
        "[{case}] exit status differs: C signal={:?} code={:?} / Rust signal={:?} code={:?}\n\
         C stderr:\n{}\nRust stderr:\n{}",
        c.signal,
        c.code,
        r.signal,
        r.code,
        c.stderr,
        r.stderr
    );
    assert_eq!(
        c.assertion, r.assertion,
        "[{case}] assertion diagnostic differs\nC:    {:?}\nRust: {:?}",
        c.assertion, r.assertion
    );
    assert_eq!(c.summary, r.summary, "[{case}] result summary differs");
    if c.signal.is_some() {
        assert_eq!(c.signal, Some(6), "[{case}] expected SIGABRT, got {:?}", c.signal);
        assert!(c.assertion.is_some(), "[{case}] aborted without an assertion message");
    }
    c.assertion
}

fn assert_assertion(case: &str, expect_expr: &str, expect_fn: &str, expect_line: u32) {
    let a = diff_case(case).unwrap_or_else(|| panic!("[{case}] did not abort"));
    assert!(
        a.contains(&format!("lib.c:{expect_line}: {expect_fn}: Assertion `{expect_expr}' failed.")),
        "[{case}] unexpected assertion: {a}"
    );
}

// --- ERRORS row 16 ---------------------------------------------------------
#[test]
fn abort16_in_bytes_zero() {
    assert_assertion("in_bytes_zero", "s->bits_left > 0", "cp_read_bits", 119);
}

// --- ERRORS row 26 ---------------------------------------------------------
#[test]
fn abort26_null_in() {
    assert_assertion("null_in", "s->bits_left > 0", "cp_read_bits", 119);
}

// --- ERRORS row 25 ---------------------------------------------------------
#[test]
fn abort25_in_bytes_negative() {
    for case in ["in_bytes_negative_1", "in_bytes_negative_4", "in_bytes_negative_big"] {
        assert_assertion(case, "s->bits_left > 0", "cp_read_bits", 119);
    }
}

// --- ERRORS row 13 ---------------------------------------------------------
#[test]
fn abort13_consume_more_than_buffered() {
    assert_assertion(
        "consume_more_than_buffered",
        "s->count >= num_bits_to_read",
        "cp_consume_bits",
        109,
    );
}

// --- ERRORS row 20 ---------------------------------------------------------
#[test]
fn abort20_decode_no_match() {
    assert_assertion(
        "decode_empty_tree",
        "(search >> len) == (key >> len)",
        "cp_decode",
        211,
    );
    // a truncated stream must at least behave identically
    diff_case("decode_truncated");
}

// --- ERRORS row 14 ---------------------------------------------------------
#[test]
fn abort14_read_bits_gt_32() {
    assert_assertion("read_bits_gt_32", "num_bits_to_read <= 32", "cp_read_bits", 117);
}

// --- ERRORS row 18 ---------------------------------------------------------
#[test]
fn abort18_would_overflow() {
    assert_assertion(
        "would_overflow",
        "!cp_would_overflow(s, num_bits_to_read)",
        "cp_read_bits",
        121,
    );
}

// --- ERRORS row 19 ---------------------------------------------------------
#[test]
fn abort19_code_length_ge_16() {
    assert_assertion("code_length_ge_16", "len < 16", "cp_build", 148);
    assert_assertion("code_length_255", "len < 16", "cp_build", 148);
}

// --- ERRORS row 11 ---------------------------------------------------------
#[test]
fn abort11_cp_ptr_unaligned() {
    assert_assertion("cp_ptr_unaligned", "!(s->bits_left & 7)", "cp_ptr", 89);
}

/// Random-input sweep (exec-based, so the *whole* stderr text is compared):
/// whatever the C does with garbage — return an error, succeed, or abort on any
/// of the asserts — the Rust must do the same.
#[test]
fn abort_fuzz_random_inputs() {
    let n: u64 = std::env::var("CP_FUZZ_N").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let mut survived = 0usize;
    for i in 0..n {
        if let Some(a) = diff_case(&format!("fuzz:{i}")) {
            seen.insert(assertion_expr(&a));
        } else {
            survived += 1;
        }
    }
    eprintln!("exec-fuzz: {survived}/{n} returned normally; assertions hit: {seen:#?}");
    assert!(survived > 0, "no random input returned normally — the corpus is degenerate");
}


/// Thousands of unstructured random inputs.
#[test]
fn fork_fuzz_unstructured() {
    let p = pair();
    let n: u64 = std::env::var("CP_FORK_FUZZ_N").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let mut survived = 0usize;
    for i in 0..n {
        let len = rng.range(1, 40) as usize;
        let stream = rng.bytes(len);
        let in_off = rng.below(4) as usize;
        let out_bytes = rng.below(64) as i32;
        match diff_fork(
            &p,
            &stream,
            in_off,
            stream.len() as i32,
            128 * 1024,
            out_bytes,
            &format!("unstructured/{i}"),
        ) {
            Some(a) => {
                seen.insert(a);
            }
            None => survived += 1,
        }
    }
    eprintln!("fork-fuzz unstructured: {survived}/{n} returned normally; assertions: {seen:#?}");
    assert!(survived > 0);
}

/// Mutations of *valid* streams: start from a well-formed fixed or dynamic
/// block and flip bits / truncate / extend, so the fuzzer reaches deep inside
/// `cp_dynamic` / `cp_block` instead of bouncing off the block header.
#[test]
fn fork_fuzz_mutated_valid() {
    let p = pair();
    let n: u64 = std::env::var("CP_FORK_FUZZ_N").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
    let mut rng = Rng::new(SEED ^ 0xCAFE);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let mut survived = 0usize;
    for i in 0..n {
        // build a valid stream
        let kind = rng.below(3);
        let mut items: Vec<Item> = Vec::new();
        let mut produced = 0usize;
        let count = rng.range(1, 24);
        for _ in 0..count {
            if produced >= 4 && rng.below(3) == 0 {
                let dist = rng.range(1, produced.min(64) as u32);
                let len = rng.range(3, 40);
                items.push(Item::Match(len, dist));
                produced += len as usize;
            } else {
                items.push(Item::Lit(rng.u8()));
                produced += 1;
            }
        }
        let mut stream = match kind {
            0 => {
                let plen = rng.range(0, 32) as usize;
                let payload = rng.bytes(plen);
                stored_stream(&payload, true)
            }
            1 => {
                let mut w = BitWriter::new();
                write_fixed_block(&mut w, true, &items);
                w.bytes
            }
            _ => {
                let mut lits: Vec<usize> = vec![256];
                for it in &items {
                    match *it {
                        Item::Lit(b) => lits.push(b as usize),
                        Item::Match(len, _) => lits.push(257 + length_code(len).0),
                        Item::RawMatch { len_idx, .. } => lits.push(257 + len_idx),
                    }
                }
                let mut dsts: Vec<usize> = Vec::new();
                for it in &items {
                    match *it {
                        Item::Match(_, d) => dsts.push(distance_code(d).0),
                        Item::RawMatch { dist_idx, .. } => dsts.push(dist_idx),
                        Item::Lit(_) => {}
                    }
                }
                if dsts.is_empty() {
                    dsts.push(0);
                }
                let lit_lens = balanced_lens(288, &lits);
                let dst_lens = balanced_lens(32, &dsts);
                let cl = cl_stream_literal(&lit_lens, &dst_lens);
                let (cl_lens, nlen) = cl_lens_for(&cl);
                let mut w = BitWriter::new();
                write_dynamic_block(
                    &mut w, true, &lit_lens, &dst_lens, &cl, &cl_lens, nlen,
                    &PERMUTATION_ORDER, &items,
                );
                w.bytes
            }
        };
        stream.extend_from_slice(&[0u8; 4]);

        // mutate
        let nmut = rng.below(4);
        for _ in 0..nmut {
            if stream.is_empty() {
                break;
            }
            let which = rng.below(3);
            match which {
                0 => {
                    let idx = rng.below(stream.len() as u32) as usize;
                    stream[idx] ^= 1 << rng.below(8);
                }
                1 => {
                    let idx = rng.below(stream.len() as u32) as usize;
                    stream[idx] = rng.u8();
                }
                _ => {
                    let keep = rng.below(stream.len() as u32 + 1) as usize;
                    stream.truncate(keep.max(1));
                }
            }
        }

        let in_off = rng.below(4) as usize;
        let out_bytes = rng.below(256) as i32;
        match diff_fork(
            &p,
            &stream,
            in_off,
            stream.len() as i32,
            256 * 1024,
            out_bytes,
            &format!("mutated/{i}"),
        ) {
            Some(a) => {
                seen.insert(a);
            }
            None => survived += 1,
        }
    }
    eprintln!("fork-fuzz mutated: {survived}/{n} returned normally; assertions: {seen:#?}");
    assert!(survived > 0);
}

/// Boundary sweep over `in_bytes` / `out_bytes` values that are *not* the
/// buffer size, plus every input alignment — the classic off-by-one surface.
#[test]
fn fork_fuzz_length_boundaries() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xB0DE);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let cases: u32 = std::env::var("CP_FORK_BOUND_N").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    for case in 0..cases {
        let items: Vec<Item> = (0..rng.range(1, 12)).map(|_| Item::Lit(rng.u8())).collect();
        let mut w = BitWriter::new();
        write_fixed_block(&mut w, true, &items);
        let mut full = w.bytes;
        full.extend_from_slice(&[0u8; 4]);
        for in_bytes in [
            0i32,
            1,
            2,
            3,
            full.len() as i32 - 1,
            full.len() as i32,
            full.len() as i32 + 1,
            -1,
        ] {
            for out_bytes in [-1i32, 0, 1, items.len() as i32 - 1, items.len() as i32] {
                for in_off in 0..4usize {
                    if let Some(a) = diff_fork(
                        &p,
                        &full,
                        in_off + 8, // keep negative-index reads inside our allocation
                        in_bytes,
                        64 * 1024,
                        out_bytes,
                        &format!("bounds/{case}/{in_bytes}/{out_bytes}/{in_off}"),
                    ) {
                        seen.insert(a);
                    }
                }
            }
        }
    }
    eprintln!("fork-fuzz boundaries: assertions: {seen:#?}");
}
