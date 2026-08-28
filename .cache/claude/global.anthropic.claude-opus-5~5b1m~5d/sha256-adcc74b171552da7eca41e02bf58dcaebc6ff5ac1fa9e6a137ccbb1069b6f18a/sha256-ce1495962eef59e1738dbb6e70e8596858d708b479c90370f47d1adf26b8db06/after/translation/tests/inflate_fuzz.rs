//! Broad corrupt-input fuzzing of `cp_inflate`.
//!
//! `cp_inflate` validates almost nothing, so a corrupt stream can drive the C
//! code into genuinely undefined behaviour.  These tests
//!
//!  * check C/Rust parity for every input they generate,
//!  * *classify* every outcome, so that the classes are visible, and
//!  * run every divergence past the harness's **undefined-behaviour oracle**
//!    (`common::is_layout_dependent`): the same unmodified `c_src/src/lib.c`
//!    compiled a second time with `-fstack-protector-all
//!    --param=ssp-buffer-size=1`, which only moves the function-local variables
//!    around.  If the two C builds already disagree with each other, the input
//!    has reached one of the C source's out-of-bounds *stack* accesses - see
//!    `inflate_errors::err35_lens_overshoot_hangs_the_c_library` - and no
//!    translation can match a particular compiler's frame.  Every other
//!    divergence fails the test.
//!
//! Run with `--nocapture` to see the histograms.

mod common;

use common::deflate::*;
use common::*;
use std::collections::BTreeMap;

fn run_stream(stream: &[u8], out_bytes: i32, skew: usize, seed: u64) -> (Case, Outcome, Outcome) {
    let mut rng = Rng::new(seed);
    let in_off = 64 + skew;
    let out_off = (in_off + stream.len() + 128) & !15;
    let total = out_off + out_bytes.max(0) as usize + 4096;
    let mut scratch: Vec<u8> = (0..total).map(|_| rng.u8()).collect();
    scratch[in_off..in_off + stream.len()].copy_from_slice(stream);
    // 1s is a ~1000x margin for these input sizes; a corrupt stream that makes
    // the C library spin forever is an expected outcome (see the module docs).
    let case = Case::inflate(scratch, in_off as isize, stream.len() as i32, out_off as isize, out_bytes)
        .with_timeout(1);
    let a = run(c_ref(), &case);
    let b = run(rust_lib(), &case);
    (case, a, b)
}

fn classify(o: &Outcome) -> String {
    match (&o.status, &o.assert_msg) {
        (Status::Signaled(s), Some(m)) => {
            let short = m.splitn(2, ": ").nth(1).unwrap_or(m);
            format!("sig{s} {short}")
        }
        (Status::Signaled(libc::SIGALRM), None) => "hung (SIGALRM)".to_string(),
        (Status::Signaled(s), None) => format!("sig{s}"),
        (Status::Exited(_), _) => match (&o.ret, &o.err) {
            (1, _) => "ok".to_string(),
            (_, Some(e)) => format!("err: {}", &String::from_utf8_lossy(e)[..20.min(e.len())]),
            _ => "ret0".to_string(),
        },
    }
}

struct Report {
    hist: BTreeMap<String, usize>,
    known_ub: usize,
    bad: Vec<String>,
    total: usize,
}

impl Report {
    fn new() -> Report {
        Report { hist: BTreeMap::new(), known_ub: 0, bad: vec![], total: 0 }
    }
    /// Any divergence is checked against the UB oracle (`is_layout_dependent`):
    /// if the two *C* builds - identical source, identical `NDEBUG`, different
    /// stack frame layout - already disagree, the input has reached one of
    /// `c_src`'s out-of-bounds stack accesses and no translation can match it.
    /// Anything else is a real bug.
    fn record(&mut self, case: &Case, c: &Outcome, r: &Outcome, desc: impl FnOnce() -> String) {
        self.total += 1;
        *self.hist.entry(classify(c)).or_default() += 1;
        if c == r {
            return;
        }
        if is_layout_dependent_given(case, c) {
            self.known_ub += 1;
            return;
        }
        if self.bad.len() < 8 {
            self.bad.push(format!("{}\n    C={c:?}\n    R={r:?}", desc()));
        } else {
            self.bad.push(String::new());
        }
    }
    fn finish(&self, name: &str) {
        println!(
            "{name}: {} cases, {} provably layout-dependent (C's own OOB stack \
             accesses), {} unexplained\nhistogram: {:#?}",
            self.total,
            self.known_ub,
            self.bad.len(),
            self.hist
        );
        for b in self.bad.iter().filter(|b| !b.is_empty()) {
            println!("  {b}");
        }
        assert!(self.bad.is_empty(), "{name}: {} unexplained divergences", self.bad.len());
        // sanity: the fuzzer must actually be reaching interesting states
        assert!(self.hist.len() >= 2, "{name}: suspiciously uniform outcomes");
    }
}

/// Purely random bytes.
#[test]
fn fuzz01_random_bytes() {
    let mut rng = Rng::new(0xF01);
    let mut rep = Report::new();
    for i in 0..1200 {
        let n = rng.below(40) as usize + 1;
        let stream = rng.bytes(n);
        let out_bytes = rng.pick(&[0i32, 1, 16, 256, 4096]);
        let skew = rng.below(4) as usize;
        let (case, a, b) = run_stream(&stream, out_bytes, skew, rng.next_u64());
        let s2 = stream.clone();
        rep.record(&case, &a, &b, || {
            format!("#{i} n={n} out={out_bytes} skew={skew} stream={}", hex(&s2))
        });
    }
    rep.finish("fuzz01_random_bytes");
}

/// Longer random inputs, so that full words are loaded and multi-block paths
/// are reached.
#[test]
fn fuzz02_longer_random_bytes() {
    let mut rng = Rng::new(0xF02);
    let mut rep = Report::new();
    for i in 0..500 {
        let n = rng.below(400) as usize + 40;
        let stream = rng.bytes(n);
        let out_bytes = rng.pick(&[0i32, 7, 64, 1024, 4096]);
        let skew = rng.below(4) as usize;
        let (case, a, b) = run_stream(&stream, out_bytes, skew, rng.next_u64());
        rep.record(&case, &a, &b, || format!("#{i} n={n} out={out_bytes} skew={skew}"));
    }
    rep.finish("fuzz02_longer_random_bytes");
}

/// Truncations of valid streams: the classic "ran out of input" family.
#[test]
fn fuzz03_truncated_valid_streams() {
    let mut rng = Rng::new(0xF03);
    let mut rep = Report::new();
    let t = Tables::default();
    for _ in 0..90 {
        let n = rng.below(30) as usize + 1;
        let syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        for kind in 0..3 {
            let mut bw = BitWriter::new();
            match kind {
                0 => emit_fixed(&mut bw, &syms, true),
                1 => {
                    let spec = dyn_spec_for(&syms, 288, 32, &t);
                    emit_dynamic(&mut bw, &spec, &syms, true, &t)
                }
                _ => {
                    let d = rng.bytes(n);
                    emit_stored(&mut bw, &d, true)
                }
            }
            let full = bw.finish();
            for cut in 1..=full.len().min(10) {
                let s = &full[..full.len() - cut];
                if s.is_empty() {
                    continue;
                }
                let (case, a, b) = run_stream(s, 4096, 0, rng.next_u64());
                let sv = s.to_vec();
                rep.record(&case, &a, &b, || format!("kind={kind} cut={cut} stream={}", hex(&sv)));
            }
        }
    }
    rep.finish("fuzz03_truncated_valid_streams");
}

/// Single-bit mutations of valid streams.
#[test]
fn fuzz04_mutated_valid_streams() {
    let mut rng = Rng::new(0xF04);
    let mut rep = Report::new();
    let t = Tables::default();
    for _ in 0..300 {
        let n = rng.below(30) as usize + 1;
        let mut syms: Vec<Sym> = (0..n).map(|_| Sym::Lit(rng.u8())).collect();
        if n > 4 {
            syms.push(Sym::Match(rng.range(3, 20) as u32, rng.range(1, n as i32) as u32));
        }
        let mut bw = BitWriter::new();
        match rng.below(3) {
            0 => emit_fixed(&mut bw, &syms, true),
            1 => {
                let spec = dyn_spec_for(&syms, 288, 32, &t);
                emit_dynamic(&mut bw, &spec, &syms, true, &t)
            }
            _ => {
                let d = rng.bytes(n);
                emit_stored(&mut bw, &d, true)
            }
        }
        let full = bw.finish();
        for _ in 0..4 {
            let mut s = full.clone();
            let pos = rng.below(s.len() as u32) as usize;
            let bit = rng.below(8);
            s[pos] ^= 1 << bit;
            let (case, a, b) = run_stream(&s, 4096, rng.below(4) as usize, rng.next_u64());
            let sv = s.clone();
            rep.record(&case, &a, &b, || format!("pos={pos} bit={bit} stream={}", hex(&sv)));
        }
    }
    rep.finish("fuzz04_mutated_valid_streams");
}

/// Randomized *structurally valid* dynamic-block headers with randomized code
/// length vectors - the richest source of `cp_build` / `cp_decode` states.
#[test]
fn fuzz05_random_dynamic_headers() {
    let mut rng = Rng::new(0xF05);
    let mut rep = Report::new();
    let t = Tables::default();
    for i in 0..400 {
        let nlit = rng.range(257, 288) as usize;
        let ndst = rng.range(1, 32) as usize;
        // a random (possibly Kraft-incomplete) length vector
        let maxlen = rng.range(1, 15) as u8;
        let lit_lens: Vec<u8> =
            (0..nlit).map(|_| if rng.below(3) == 0 { rng.range(1, maxlen as i32) as u8 } else { 0 }).collect();
        let dist_lens: Vec<u8> =
            (0..ndst).map(|_| if rng.below(2) == 0 { rng.range(1, maxlen as i32) as u8 } else { 0 }).collect();
        let mut spec = DynSpec::new(lit_lens, dist_lens);
        spec.cl_mode = rng.pick(&[ClMode::LITERAL, ClMode::R16, ClMode::R17, ClMode::R18, ClMode::ALL]);
        let mut bw = BitWriter::new();
        // no symbols: just the header, then whatever the padding decodes to
        emit_dynamic_header_only(&mut bw, &spec);
        let mut stream = bw.finish();
        let npad = rng.below(40) as usize + 8;
        stream.extend(rng.bytes(npad));
        let (case, a, b) = run_stream(&stream, rng.pick(&[0i32, 64, 4096]), 0, rng.next_u64());
        rep.record(&case, &a, &b, || format!("#{i} nlit={nlit} ndst={ndst} maxlen={maxlen}"));
        let _ = &t;
    }
    rep.finish("fuzz05_random_dynamic_headers");
}

/// Random `unfilter` arguments, for symmetry (its whole input space is scalars
/// plus one buffer, so this is a genuine fuzz of the public header's API).
#[test]
fn fuzz06_unfilter_random_arguments() {
    let mut rng = Rng::new(0xF06);
    let mut divergences = 0usize;
    for _ in 0..4000 {
        let w = rng.range(-6, 40);
        let bpp = rng.range(-6, 10);
        let h = rng.range(-2, 20);
        let len = w.wrapping_mul(bpp);
        let rows = if h > 0 { h as i64 } else { 0 } + 2;
        let span = (rows * ((len as i64).abs() + 2) + 64) as usize;
        let pad = span as isize;
        let total = 2 * span + span;
        let mut scratch: Vec<u8> = (0..total).map(|_| rng.u8()).collect();
        // random filter bytes, valid *and* invalid
        for r in 0..h.max(0) {
            let off = pad + (r as isize) * (len as isize + 1);
            if off >= 0 && (off as usize) < total {
                scratch[off as usize] =
                    if rng.below(4) == 0 { rng.u8() } else { rng.below(5) as u8 };
            }
        }
        let case = Case::unfilter(scratch, w, h, bpp, pad);
        let a = run(c_ref(), &case);
        let b = run(rust_lib(), &case);
        if a != b {
            divergences += 1;
            if divergences < 4 {
                println!("unfilter divergence: {:?}\n  C={a:?}\n  R={b:?}", case.call);
            }
        }
    }
    assert_eq!(divergences, 0, "{divergences} unfilter divergences");
}
