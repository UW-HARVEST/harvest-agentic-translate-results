//! Phase C — abort-path differential tests.
//!
//! The reference C `.so` is built without `NDEBUG`, so a failed `assert()` calls
//! `abort()`. That is observable behaviour, not a value, so it cannot be
//! compared in-process: the first abort would take the test runner with it.
//!
//! Instead every case is executed in a *child* process (this same test binary
//! re-executed with `--exact child_worker`), once against the C `.so` and once
//! against the Rust `.so`. The parent compares the exact sequence of `RESULT`
//! lines the child printed *and* how the child died. Cases are batched, and when
//! both children abort at the same case index the parent resumes with the
//! remaining cases, so one abort does not hide the cases behind it.

mod common;

use common::deflate::*;
use common::rng::{Rng, SEED};
use common::*;

use std::ffi::c_void;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ===========================================================================
// Case representation
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub stream: Vec<u8>,
    pub align: usize,
    pub out_bytes: i32,
    /// `None` => use `stream.len()`
    pub in_bytes: Option<i32>,
    pub null_in: bool,
    pub null_out: bool,
    /// Optional replacement for the exported `cp_fixed_table` (320 bytes).
    pub fixed_table: Option<Vec<u8>>,
}

impl Case {
    fn new(stream: Vec<u8>, align: usize, out_bytes: i32) -> Case {
        Case {
            stream,
            align,
            out_bytes,
            in_bytes: None,
            null_in: false,
            null_out: false,
            fixed_table: None,
        }
    }

    fn encode(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            hex(&self.stream),
            self.align,
            self.out_bytes,
            self.in_bytes.map(|x| x.to_string()).unwrap_or_default(),
            self.null_in as u8,
            self.null_out as u8,
            self.fixed_table.as_deref().map(hex).unwrap_or_default(),
        )
    }

    fn decode(s: &str) -> Case {
        let f: Vec<&str> = s.split(',').collect();
        assert_eq!(f.len(), 7, "bad case encoding {s:?}");
        Case {
            stream: unhex(f[0]),
            align: f[1].parse().unwrap(),
            out_bytes: f[2].parse().unwrap(),
            in_bytes: if f[3].is_empty() {
                None
            } else {
                Some(f[3].parse().unwrap())
            },
            null_in: f[4] == "1",
            null_out: f[5] == "1",
            fixed_table: if f[6].is_empty() {
                None
            } else {
                Some(unhex(f[6]))
            },
        }
    }
}

// ===========================================================================
// Child worker
// ===========================================================================

#[test]
fn child_worker() {
    let cases = match std::env::var("CP_CHILD_CASES_FILE") {
        Ok(pathv) => std::fs::read_to_string(&pathv)
            .unwrap_or_else(|e| panic!("cannot read case file {pathv}: {e}")),
        Err(_) => return, // not a child; nothing to do
    };
    let which = std::env::var("CP_CHILD_IMPL").expect("CP_CHILD_IMPL");
    let im: Impl = load_one(&which);
    let im = &im;

    for enc in cases.split('\n') {
        if enc.is_empty() {
            continue;
        }
        let case = Case::decode(enc);
        if let Some(t) = &case.fixed_table {
            im.set_fixed_table(t);
        }
        let mut inbuf = AlignedBuf::new(&case.stream, case.align);
        let in_ptr = if case.null_in {
            std::ptr::null_mut()
        } else {
            inbuf.as_mut_ptr() as *mut c_void
        };
        let out_len = case.out_bytes.max(0) as usize;
        let mut out = vec![0u8; out_len];
        let out_ptr = if case.null_out {
            std::ptr::null_mut()
        } else {
            out.as_mut_ptr() as *mut c_void
        };
        let in_bytes = case.in_bytes.unwrap_or(case.stream.len() as i32);

        im.clear_error();
        let ret = unsafe { (im.cp_inflate)(in_ptr, in_bytes, out_ptr, case.out_bytes) };
        let err = im.error();
        println!(
            "RESULT ret={} err={} out={}",
            ret,
            err.unwrap_or_else(|| "<null>".to_string()),
            digest(&out)
        );
        std::io::stdout().flush().unwrap();
    }
    println!("DONE");
    std::io::stdout().flush().unwrap();
}

// ===========================================================================
// Parent driver
// ===========================================================================

struct ChildOutcome {
    results: Vec<String>,
    done: bool,
    status: String,
}

fn run_child(which: &str, case_file: &std::path::Path) -> ChildOutcome {
    // Wrap in `timeout` so a case that makes either implementation loop forever
    // is still comparable (both should time out with the same status).
    let out = Command::new("timeout")
        .arg("20")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "child_worker", "--nocapture"])
        .env("CP_CHILD_IMPL", which)
        .env("CP_CHILD_CASES_FILE", case_file)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let results: Vec<String> = stdout
        .lines()
        .filter(|l| l.starts_with("RESULT "))
        .map(|l| l.to_string())
        .collect();
    let done = stdout.lines().any(|l| l == "DONE");
    let status = match (out.status.code(), out.status.signal()) {
        (Some(c), _) => format!("exit={c}"),
        (None, Some(s)) => format!("signal={s}"),
        _ => "unknown".to_string(),
    };
    ChildOutcome {
        results,
        done,
        status,
    }
}

/// The case list is handed to children through a temp file: encoded case lists
/// quickly exceed the environment size limit.
fn write_case_file(ctx: &str, cases: &[Case]) -> std::path::PathBuf {
    let enc: Vec<String> = cases.iter().map(|c| c.encode()).collect();
    let name = format!(
        "cp_cases_{}_{}_{}.txt",
        std::process::id(),
        ctx.bytes().fold(0u64, |a, b| a.wrapping_mul(131).wrapping_add(b as u64)),
        cases.len()
    );
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, enc.join("\n")).expect("cannot write case file");
    path
}

/// Run every case in both implementations and require identical behaviour,
/// including *where* the process died.
fn diff_cases(ctx: &str, cases: &[Case]) {
    let mut start = 0usize;
    let mut guard = 0;
    while start < cases.len() {
        guard += 1;
        assert!(guard < 4096, "[{ctx}] too many abort restarts");
        let batch = &cases[start..];
        let file = write_case_file(ctx, batch);
        let c = run_child("c", &file);
        let r = run_child("rust", &file);
        let _ = std::fs::remove_file(&file);

        let n = c.results.len().min(r.results.len());
        for i in 0..n {
            assert_eq!(
                c.results[i], r.results[i],
                "[{ctx}] case #{} diverged\n  case: {:?}\n  C   : {}\n  Rust: {}",
                start + i,
                batch[i],
                c.results[i],
                r.results[i]
            );
        }
        assert_eq!(
            c.results.len(),
            r.results.len(),
            "[{ctx}] different number of completed cases starting at #{start} \
             (C {} / Rust {}, C status {} / Rust status {}); first uncompared case: {:?}",
            c.results.len(),
            r.results.len(),
            c.status,
            r.status,
            batch.get(n).map(|x| format!("{x:?}")).unwrap_or_default()
        );
        assert_eq!(
            c.status, r.status,
            "[{ctx}] exit status differs after {} cases from #{start}\n  case: {:?}",
            c.results.len(),
            batch.get(c.results.len()).map(|x| format!("{x:?}"))
        );

        if c.done {
            assert_eq!(c.results.len(), batch.len(), "[{ctx}] child lost cases");
            return;
        }
        // Both died on the case just past the last reported result: skip it and
        // continue with the rest.
        let died_on = c.results.len();
        start += died_on + 1;
    }
}

// ===========================================================================
// Helpers to build streams
// ===========================================================================

fn lits(bytes: &[u8]) -> Vec<Op> {
    bytes.iter().map(|&b| Op::Lit(b)).collect()
}

fn fixed_stream(rng: &mut Rng, n: usize) -> Vec<u8> {
    let data = rng.bytes(n);
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &lits(&data));
    w.finish()
}

fn dynamic_stream(rng: &mut Rng, n: usize) -> Vec<u8> {
    let data = rng.bytes(n);
    let ops = lits(&data);
    let d = dynamic_for(rng, &ops, Shape::Balanced, RepeatOpts::all(), 257, 1);
    let mut w = BitWriter::new();
    emit_dynamic(&mut w, true, &d, &ops);
    w.finish()
}

// ===========================================================================
// A5 / A7 — input exhausted or shorter than the next read
// ===========================================================================

#[test]
fn a5_zero_and_negative_in_bytes() {
    let mut cases = Vec::new();
    for &in_bytes in &[0i32, -1, -2, -3, -4, -8, -100, i32::MIN + 1] {
        for align in 0..4usize {
            for &out_bytes in &[0i32, 1, 64] {
                let mut c = Case::new(vec![0u8; 64], align, out_bytes);
                c.in_bytes = Some(in_bytes);
                cases.push(c);
            }
        }
    }
    diff_cases("A5 in_bytes<=0", &cases);
}

#[test]
fn a5_null_input_pointer() {
    let mut cases = Vec::new();
    for &in_bytes in &[0i32, -1] {
        for &out_bytes in &[0i32, 16] {
            let mut c = Case::new(Vec::new(), 0, out_bytes);
            c.in_bytes = Some(in_bytes);
            c.null_in = true;
            cases.push(c);
        }
    }
    diff_cases("A5 null in", &cases);
}

#[test]
fn null_output_pointer_without_writes() {
    // out_bytes == 0 means `out_end == out`, so a literal is rejected before any
    // dereference; an empty block writes nothing at all.
    let mut rng = Rng::new(SEED ^ 0x4E30);
    let mut cases = Vec::new();
    let mut w = BitWriter::new();
    emit_fixed(&mut w, true, &[]);
    let empty = w.finish();
    for stream in [empty, fixed_stream(&mut rng, 4), dynamic_stream(&mut rng, 4)] {
        for align in 0..4usize {
            let mut c = Case::new(stream.clone(), align, 0);
            c.null_out = true;
            cases.push(c);
        }
    }
    diff_cases("null out, out_bytes=0", &cases);
}

#[test]
fn truncated_streams() {
    let mut rng = Rng::new(SEED ^ 0x7000);
    let mut cases = Vec::new();
    for n in [1usize, 4, 17, 60] {
        for stream in [fixed_stream(&mut rng, n), dynamic_stream(&mut rng, n)] {
            // Every cut for short streams, a sampled subset for longer ones so
            // the abort-restart loop stays fast.
            let step = 1 + stream.len() / 12;
            let mut cut = 1;
            while cut < stream.len() {
                cases.push(Case::new(stream[..cut].to_vec(), cut % 4, (n + 64) as i32));
                cut += step;
            }
        }
    }
    // Also: full stream but a shrunken in_bytes (same buffer contents, the
    // decoder is told there is less input than there is).
    for n in [8usize, 40] {
        let stream = fixed_stream(&mut rng, n);
        let step = 1 + stream.len() / 12;
        let mut ib = 0i32;
        while ib < stream.len() as i32 {
            let mut c = Case::new(stream.clone(), 0, (n + 64) as i32);
            c.in_bytes = Some(ib);
            cases.push(c);
            ib += step as i32;
        }
    }
    diff_cases("truncated", &cases);
}

#[test]
fn stored_block_truncated_and_oversized_len() {
    let mut rng = Rng::new(SEED ^ 0x5700);
    let mut cases = Vec::new();
    // LEN larger than the payload actually present: the C copies LEN bytes with
    // no out-buffer check (U3), so out_bytes is always >= LEN here.
    for len in [1u16, 2, 4, 16, 64] {
        let payload = rng.bytes(len as usize);
        let mut w = BitWriter::new();
        emit_stored_lens(&mut w, true, &payload, len, !len);
        let full = w.finish();
        let step = 1 + full.len() / 8;
        let mut cut = 1;
        while cut < full.len() {
            cases.push(Case::new(full[..cut].to_vec(), cut % 4, 4096));
            cut += step;
        }
        // Declared LEN bigger than what follows. The C `memcpy`s LEN bytes with
        // no bound check (U3); `AlignedBuf`'s zero margins keep the over-read
        // deterministic, and out_bytes is always >= LEN.
        for bigger in [len + 1, len + 7, len.wrapping_mul(3) | 1, 0x0FFF] {
            let mut w = BitWriter::new();
            emit_stored_lens(&mut w, true, &payload, bigger, !bigger);
            cases.push(Case::new(w.finish(), 0, 0x2000));
        }
    }
    diff_cases("stored truncated / oversized LEN", &cases);
}

// ===========================================================================
// A8 — a code length >= 16 reaches cp_build
// ===========================================================================

/// Reaching `cp_build` with a code length >= 16 is `ERRORS.md` row U6: the C
/// indexes its `int counts[16]` / `codes[16]` / `first[16]` stack arrays out of
/// bounds. With `assert()` live the C aborts and so does the Rust, which is what
/// this test checks. Compiled with `-DNDEBUG` the C instead corrupts its own
/// frame and eventually segfaults, so there is no defined result to match.
#[test]
#[cfg_attr(
    not(feature = "c_asserts"),
    ignore = "ERRORS.md row U6 is undefined behaviour once NDEBUG removes assert(len < 16)"
)]
fn a8_fixed_table_with_oversized_code_length() {
    let mut rng = Rng::new(SEED ^ 0xA800);
    let stream = fixed_stream(&mut rng, 8);
    let mut cases = Vec::new();
    for &bad in &[16u8, 17, 31, 32, 100, 255] {
        for &idx in &[0usize, 1, 143, 144, 255, 256, 287, 288, 300, 319] {
            let mut t = fixed_table();
            t[idx] = bad;
            let mut c = Case::new(stream.clone(), 0, 4096);
            c.fixed_table = Some(t);
            cases.push(c);
        }
    }
    diff_cases("A8 cp_fixed_table len>=16", &cases);
}

// ===========================================================================
// A9 / degenerate trees — incomplete and over-subscribed Huffman codes
// ===========================================================================

/// Emit a dynamic block with arbitrary (possibly invalid) code lengths.
fn raw_dynamic(lit_lens: Vec<u8>, dst_lens: Vec<u8>, ops: &[Op]) -> Option<Vec<u8>> {
    if lit_lens[256] == 0 {
        // emit_dynamic requires a code for end-of-block; emit the ops only.
        return None;
    }
    let d = Dynamic {
        lit_lens,
        dst_lens,
        repeats: RepeatOpts::none(),
        force_nlen: None,
    };
    let mut w = BitWriter::new();
    emit_dynamic(&mut w, true, &d, ops);
    Some(w.finish())
}

#[test]
fn a9_incomplete_and_oversubscribed_literal_codes() {
    let mut cases = Vec::new();

    // Incomplete: Kraft sum < 1.
    for len256 in 1..=9u8 {
        for extra in [None, Some((65u8, 3u8)), Some((66, 9))] {
            let mut lit = vec![0u8; 257];
            lit[256] = len256;
            if let Some((sym, l)) = extra {
                lit[sym as usize] = l;
            }
            let ops: Vec<Op> = match extra {
                Some((sym, _)) => vec![Op::Lit(sym)],
                None => vec![],
            };
            if let Some(s) = raw_dynamic(lit, vec![0u8; 1], &ops) {
                for align in 0..2usize {
                    cases.push(Case::new(s.clone(), align, 4096));
                }
            }
        }
    }

    // Over-subscribed: Kraft sum > 1 (more symbols than the depth allows).
    for n in [3usize, 5, 9] {
        let mut lit = vec![0u8; 257];
        lit[256] = 1;
        for i in 0..n {
            lit[i] = 1; // n+1 symbols all at length 1
        }
        let ops: Vec<Op> = (0..n).map(|i| Op::Lit(i as u8)).collect();
        if let Some(s) = raw_dynamic(lit, vec![0u8; 1], &ops) {
            cases.push(Case::new(s, 0, 4096));
        }
    }

    diff_cases("A9 incomplete/oversubscribed lit code", &cases);
}

#[test]
fn a9_empty_distance_tree_with_match() {
    // `ndst` symbols all length 0 => cp_build returns 0 => cp_decode is called
    // with hi == 0 and reads tree[-1] (the tail of `s->lit`). Deterministic in
    // both implementations because the state struct layout and zero-init match.
    let mut cases = Vec::new();
    for ndst in [1usize, 2, 8, 32] {
        let mut lit = vec![0u8; 258];
        lit[256] = 1;
        lit[257] = 1; // length symbol 257 => len 3
        let ops = vec![Op::Raw {
            lsym: 257,
            lextra: 0,
            dsym: 0,
            dextra: 0,
        }];
        // `emit_ops` asserts on a zero-length distance code, so the length
        // symbol is emitted by hand after the header.
        let _ = &ops;
        let d = Dynamic {
            lit_lens: lit.clone(),
            dst_lens: vec![0u8; ndst],
            repeats: RepeatOpts::none(),
            force_nlen: None,
        };
        let mut w = BitWriter::new();
        let (lit_codes, _) = emit_dynamic_header_only(&mut w, true, &d);
        w.code(lit_codes[257], lit[257] as u32);
        let s = w.finish();
        for align in 0..2usize {
            cases.push(Case::new(s.clone(), align, 4096));
        }
    }
    diff_cases("A9 empty distance tree", &cases);
}

#[test]
fn a9_empty_literal_tree() {
    // Every lit/len length zero => the literal tree is empty and cp_decode reads
    // s->lit[-1], i.e. the tail of s->lookup.
    let mut cases = Vec::new();
    for nlit in [257usize, 260, 288] {
        for ndst in [1usize, 4, 32] {
            let d = Dynamic {
                lit_lens: vec![0u8; nlit],
                dst_lens: vec![0u8; ndst],
                repeats: RepeatOpts::none(),
                force_nlen: None,
            };
            let mut w = BitWriter::new();
            emit_dynamic_header_only(&mut w, true, &d);
            // a few arbitrary payload bits
            w.bits(0b1011_0101, 8);
            w.bits(0b0110_1001, 8);
            let s = w.finish();
            for align in 0..2usize {
                cases.push(Case::new(s.clone(), align, 4096));
            }
        }
    }
    diff_cases("A9 empty literal tree", &cases);
}

// ===========================================================================
// Broad randomized malformed-input fuzz
// ===========================================================================

/// Random bytes as a whole stream. `btype` is forced to 0 or 1 so the fuzz stays
/// clear of `cp_dynamic`'s undefined-behaviour rows U1/U2 (uninitialised
/// `lens[-1]` and writes past `lens[319]`), which have no defined C result.
#[test]
fn fuzz_random_streams_btype_0_and_1() {
    let mut rng = Rng::new(SEED ^ 0xF0F0);
    let mut cases = Vec::new();
    for _ in 0..600 {
        let n = 1 + rng.below(48);
        let mut s = rng.bytes(n);
        let btype = (rng.below(2) as u8) << 1;
        s[0] = (s[0] & !0b110) | btype;
        let align = rng.below(4);
        let out_bytes = [0i32, 1, 7, 64, 1024][rng.below(5)];
        cases.push(Case::new(s, align, out_bytes));
    }
    diff_cases("fuzz btype 0/1", &cases);
}

/// Corrupt single bits/bytes of otherwise valid fixed-Huffman streams.
///
/// Byte 0 (which carries `BFINAL=1` and `BTYPE=1`) is left intact so the stream
/// stays a single fixed block. That keeps the whole test inside fully defined C
/// behaviour: the fixed Huffman trees are always complete, so `cp_decode` can
/// only return symbols 0..287 / distances 0..31, every `cp_len_*` / `cp_dist_*`
/// lookup is in range, and `cp_build` never sees a code length >= 16 (row U6).
/// Letting the corruption reach a second, `BTYPE=2` block would put the decoder
/// into `cp_dynamic`, whose malformed-input paths are rows U1/U2/U6.
#[test]
fn fuzz_bitflipped_fixed_streams() {
    let mut rng = Rng::new(SEED ^ 0xB17F);
    let mut cases = Vec::new();
    for _ in 0..40 {
        let n = 1 + rng.below(40);
        let base = fixed_stream(&mut rng, n);
        assert_eq!(base[0] & 0b111, 0b011, "expected BFINAL=1, BTYPE=1");
        for _ in 0..12 {
            let mut s = base.clone();
            let flips = 1 + rng.below(3);
            for _ in 0..flips {
                let i = 1 + rng.below(s.len() - 1);
                s[i] ^= 1 << rng.below(8);
            }
            cases.push(Case::new(s, rng.below(4), (n + 64) as i32));
        }
    }
    diff_cases("fuzz bitflip fixed", &cases);
}

/// Corrupt otherwise valid *dynamic* streams, but only in the bytes *after* the
/// block header (block type, HLIT/HDIST/HCLEN, the code-length code and the
/// coded lit/dist lengths).
///
/// Corrupting the header itself can make `cp_dynamic`'s code-length loop
/// overshoot its 320-byte stack array `lens` or read `lens[-1]`
/// (`ERRORS.md` rows U2 and U1). Both are undefined behaviour in C: in practice
/// the C smashes its own frame and loops forever, so there is no defined result
/// for the Rust to match. Restricting the corruption to the payload leaves the
/// Huffman trees intact and exercises `cp_decode` / `cp_block` on malformed data
/// with a fully defined C behaviour.
#[test]
fn fuzz_bitflipped_dynamic_payloads() {
    let mut rng = Rng::new(SEED ^ 0xB17D);
    let mut cases = Vec::new();
    for _ in 0..24 {
        let n = 4 + rng.below(40);
        let data = rng.bytes(n);
        let ops = lits(&data);
        let d = dynamic_for(&mut rng, &ops, Shape::Balanced, RepeatOpts::all(), 257, 1);

        // Emit the header alone to learn where the payload starts.
        let mut probe = BitWriter::new();
        let _ = emit_dynamic_header_only(&mut probe, true, &d);
        let header_bits = probe.bit_len();
        let first_payload_byte = header_bits / 8 + 1;

        let mut w = BitWriter::new();
        emit_dynamic(&mut w, true, &d, &ops);
        let base = w.finish();
        if base.len() <= first_payload_byte {
            continue;
        }
        for _ in 0..8 {
            let mut s = base.clone();
            let flips = 1 + rng.below(3);
            for _ in 0..flips {
                let i = first_payload_byte + rng.below(s.len() - first_payload_byte);
                s[i] ^= 1 << rng.below(8);
            }
            cases.push(Case::new(s, rng.below(4), (n + 64) as i32));
        }
    }
    diff_cases("fuzz bitflip dynamic payload", &cases);
}

/// Exploratory only: corrupts the dynamic-block *header* as well, which can
/// reach `ERRORS.md` rows U1/U2 (uninitialised `lens[-1]`, writes past
/// `lens[319]`). The C's behaviour there is undefined - it overwrites its own
/// stack frame and typically spins forever - so this cannot be a pass/fail
/// differential test. Run it manually with
/// `cargo test --release --test phase_c_subproc -- --ignored fuzz_bitflipped_dynamic_headers`
/// to inspect the divergences.
#[test]
#[ignore = "reaches ERRORS.md rows U1/U2, which are undefined behaviour in the C"]
fn fuzz_bitflipped_dynamic_headers() {
    let mut rng = Rng::new(SEED ^ 0xB17E);
    let mut cases = Vec::new();
    for _ in 0..20 {
        let n = 1 + rng.below(40);
        let base = dynamic_stream(&mut rng, n);
        for _ in 0..6 {
            let mut s = base.clone();
            let i = rng.below(s.len());
            s[i] ^= 1 << rng.below(8);
            cases.push(Case::new(s, rng.below(4), (n + 64) as i32));
        }
    }
    diff_cases("fuzz bitflip dynamic header", &cases);
}

// ===========================================================================
// U1 — code-length symbol 16 as the very first symbol (reads `lens[-1]`)
// ===========================================================================

/// `cp_dynamic` starts with `uint8_t lens[288 + 32]` uninitialised, and code
/// length symbol 16 copies `lens[n - 1]`. When 16 is the first symbol decoded,
/// the C reads `lens[-1]` (row U1).
///
/// Empirically both implementations abort on these inputs (the resulting
/// all-zero literal code makes `cp_decode` run with `hi == 0`, whose
/// `assert((search >> len) == (key >> len))` fails in both), so the row *is*
/// differentially testable even though the read itself is undefined.
fn sym16_first_stream(rep_extra: u32, tail: u8, pad_bytes: usize) -> Vec<u8> {
    // Code-length alphabet: symbols 16 and 18 only, one bit each.
    let mut cl_lens = [0u8; 19];
    cl_lens[16] = 1;
    cl_lens[18] = 1;
    let cl_codes = canonical_codes(&cl_lens);

    let nlit = 257usize;
    let ndst = 1usize;
    let nlen = 4usize; // permutation indices of 16 and 18 are 0 and 2

    let mut w = BitWriter::new();
    w.bits(1, 1); // bfinal
    w.bits(2, 2); // btype = dynamic
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits((nlen - 4) as u32, 4);
    for i in 0..nlen {
        w.bits(cl_lens[PERMUTATION[i]] as u32, 3);
    }
    // symbol 16 first: repeat the *previous* length -> reads lens[-1]
    w.code(cl_codes[16], 1);
    w.bits(rep_extra, 2);
    // pad the rest of the length sequence with zeros (symbol 18)
    let mut n = 3 + rep_extra as usize;
    while n < nlit + ndst {
        let want = (nlit + ndst - n).min(138);
        let run = want.max(11);
        w.code(cl_codes[18], 1);
        w.bits((run - 11) as u32, 7);
        n += run;
    }
    for _ in 0..pad_bytes {
        w.bits(tail as u32, 8);
    }
    w.finish()
}

#[test]
fn u1_code_length_symbol_16_first() {
    let mut cases = Vec::new();
    for rep_extra in 0..4u32 {
        for &tail in &[0u8, 0x5A, 0xFF] {
            for &pad in &[2usize, 24] {
                cases.push(Case::new(
                    sym16_first_stream(rep_extra, tail, pad),
                    rep_extra as usize % 4,
                    512,
                ));
            }
        }
    }
    diff_cases("U1 symbol 16 first", &cases);
}
