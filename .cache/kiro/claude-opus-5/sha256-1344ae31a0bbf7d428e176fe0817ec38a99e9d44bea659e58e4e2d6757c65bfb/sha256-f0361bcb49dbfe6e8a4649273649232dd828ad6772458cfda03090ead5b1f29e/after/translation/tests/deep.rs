//! Deep sweeps.
//!
//! Phase B compares stdout call-by-call, which caps how many inputs it can
//! afford. These tests trade granularity for volume: fd 1 is redirected once
//! per pass, hundreds of thousands of calls are made, and the two passes are
//! then compared as a whole (return-value vector + the complete stdout byte
//! stream). This is how the `(int)(float_val * 100)` conversion — the riskiest
//! part of the translation, since gcc's `cvttss2si` is not Rust's saturating
//! `as` cast — gets covered across every IEEE-754 binary32 class.

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Remove libtest's own progress chatter from a capture.
///
/// For tests that run longer than 60s libtest writes
/// `"\ntest <name> has been running for over 60 seconds\n"` to fd 1, which this
/// harness has redirected — so the message lands *inside* the captured stream,
/// possibly splitting a `printf` line in half. Excising exactly that substring
/// (including the leading newline it inserts) restores the original stream
/// byte-for-byte.
fn strip_libtest_noise(buf: Vec<u8>) -> Vec<u8> {
    const START: &[u8] = b"\ntest ";
    const END: &[u8] = b" seconds\n";
    let mut out = buf;
    loop {
        let Some(s) = out
            .windows(START.len())
            .position(|w| w == START)
            .and_then(|s| {
                out[s..]
                    .windows(END.len())
                    .position(|w| w == END)
                    .filter(|&rel| rel < 200)
                    .map(|rel| (s, s + rel + END.len()))
            })
        else {
            return out;
        };
        let (from, to) = s;
        out.drain(from..to);
    }
}

/// Run `f` with fd 1 redirected into a fresh temp file; return `f`'s value and
/// the whole captured byte stream.
fn pass<R>(tag: &str, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let path = std::env::temp_dir().join(format!("cdeep-{}-{tag}.out", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp file");
    let out = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(file.as_raw_fd(), 1) >= 0);
        let r = f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
        r
    };
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    drop(file);
    let _ = std::fs::remove_file(&path);
    (out, strip_libtest_noise(buf))
}

/// A caller-owned `ProcessState` whose `data` word can be set directly, so the
/// sweep does not pay for a `create_state`/`destroy_state` round trip per input.
struct Scratch {
    _buf: Vec<u8>,
    st: Box<ProcessState>,
}

impl Scratch {
    fn new() -> Scratch {
        let mut buf: Vec<u8> = b"State:0:Mode:3\0".to_vec();
        let st = Box::new(ProcessState {
            flags: 0x7B05,
            data: 0,
            buffer: buf.as_mut_ptr() as *mut c_char,
            capacity: buf.len() as c_int,
        });
        Scratch { _buf: buf, st }
    }
    fn ptr(&mut self) -> *mut ProcessState {
        &mut *self.st as *mut ProcessState
    }
}

fn sweep_confuse(im: &Impl, patterns: &[u32], op: c_int, tag: &str) -> (Vec<i32>, Vec<u8>) {
    let mut sc = Scratch::new();
    let ptr = sc.ptr();
    let cf = im.confuse_types;
    pass(tag, || {
        let mut rets = Vec::with_capacity(patterns.len());
        for &bits in patterns {
            unsafe {
                (*ptr).data = bits;
                rets.push(cf(ptr, op));
            }
        }
        rets
    })
}

fn compare_confuse(label: &str, patterns: &[u32], op: c_int) {
    let p = pair();
    let (rc, oc) = sweep_confuse(&p.c, patterns, op, &format!("{label}-c"));
    let (rr, or) = sweep_confuse(&p.r, patterns, op, &format!("{label}-r"));

    assert_eq!(rc.len(), patterns.len());
    if rc != rr {
        let i = rc.iter().zip(&rr).position(|(a, b)| a != b).unwrap();
        panic!(
            "[{label}] op={op} return mismatch at pattern #{i} = {:#010x} ({} as f32): C={} Rust={}",
            patterns[i],
            f32::from_bits(patterns[i]),
            rc[i],
            rr[i]
        );
    }
    if oc != or {
        let i = oc.iter().zip(&or).position(|(a, b)| a != b).unwrap_or(oc.len().min(or.len()));
        let lo = i.saturating_sub(120);
        panic!(
            "[{label}] op={op} stdout mismatch at byte {i} (C {} bytes, Rust {} bytes)\n  C   …{}\n  Rust…{}",
            oc.len(),
            or.len(),
            show(&oc[lo..(i + 120).min(oc.len())]),
            show(&or[lo..(i + 120).min(or.len())])
        );
    }
    // Guard against a vacuous pass: arms 0..3 print exactly one line per call,
    // out-of-range operations print nothing.
    if (0..=3).contains(&op) {
        assert!(
            oc.len() > patterns.len() * 10,
            "[{label}] op={op} produced only {} stdout bytes for {} calls — \
             the sweep is not exercising the library",
            oc.len(),
            patterns.len()
        );
        assert_eq!(
            oc.iter().filter(|&&b| b == b'\n').count(),
            patterns.len(),
            "[{label}] op={op} line count != call count"
        );
    } else {
        assert!(oc.is_empty(), "[{label}] op={op} printed {} bytes", oc.len());
    }
    eprintln!(
        "[{label}] op={op}: {} calls, {} stdout bytes matched",
        patterns.len(),
        oc.len()
    );
}

/// Structured coverage of the binary32 space: both signs x all 256 exponents
/// (zero, denormal, every normal, inf/NaN) x a spread of mantissas including
/// the extremes.
fn structured_patterns(mantissa_bits: u32, extra_random: usize, seed: u64) -> Vec<u32> {
    let step = 1u32 << (23 - mantissa_bits);
    let mut v = Vec::new();
    for sign in [0u32, 1] {
        for exp in 0u32..=255 {
            let mut m = 0u32;
            loop {
                v.push((sign << 31) | (exp << 23) | m);
                if m > 0x007f_ffff - step {
                    break;
                }
                m += step;
            }
            v.push((sign << 31) | (exp << 23) | 0x007f_ffff);
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..extra_random {
        v.push(rng.next_u32());
    }
    v
}

#[test]
fn deep_op1_float_to_int_structured() {
    // 2 * 256 * 33 + 20000 ≈ 37k patterns covering every exponent.
    let patterns = structured_patterns(5, 20_000, 0xD0001);
    compare_confuse("deep-op1-structured", &patterns, 1);
}

#[test]
fn deep_op1_float_to_int_overflow_band() {
    // The exponent band where `float_val * 100` straddles INT_MAX / INT_MIN is
    // exactly where a saturating cast diverges from `cvttss2si`. Sweep it
    // densely: exponents 140..170, 2 signs, 1024 mantissa steps => ~63k.
    let mut v = Vec::new();
    for sign in [0u32, 1] {
        for exp in 140u32..=170 {
            for k in 0u32..1024 {
                v.push((sign << 31) | (exp << 23) | (k << 13));
            }
        }
    }
    // And the exact integer boundaries, scaled down by 100 so that *100 lands
    // on / next to 2^31.
    for base in [
        2147483648.0f32,
        2147483520.0f32,
        21474836.0f32,
        21474837.0f32,
        21474838.0f32,
        -21474836.0f32,
        -21474837.0f32,
        -21474838.0f32,
        -2147483648.0f32,
    ] {
        for d in -8i32..=8 {
            let x = base + d as f32;
            v.push(x.to_bits());
            v.push((x / 100.0).to_bits());
            v.push((-x / 100.0).to_bits());
        }
    }
    compare_confuse("deep-op1-overflow-band", &v, 1);
}

#[test]
fn deep_op1_float_to_int_low_exponents() {
    // Denormals, zeros and sub-1.0 values, where truncation must yield 0 with
    // the correct sign behaviour, plus every representable small integer.
    let mut v = Vec::new();
    for sign in [0u32, 1] {
        for exp in 0u32..=140 {
            for k in 0u32..64 {
                v.push((sign << 31) | (exp << 23) | (k << 17));
            }
        }
    }
    for i in -2000i32..=2000 {
        v.push((i as f32).to_bits());
        v.push((i as f32 / 100.0).to_bits());
        v.push((i as f32 / 3.0).to_bits());
    }
    compare_confuse("deep-op1-low-exp", &v, 1);
}

#[test]
fn deep_op2_and_op3_full_byte_coverage() {
    // op 2 masks the low byte; op 3 sums bytes 0 and 1 as *signed* chars.
    // Sweep every combination of the two low bytes exhaustively (65536) plus
    // randomized high bytes.
    let mut v: Vec<u32> = Vec::with_capacity(70_000);
    for b1 in 0u32..256 {
        for b0 in 0u32..256 {
            v.push((b1 << 8) | b0);
        }
    }
    let mut rng = Rng::new(0xD0004);
    for _ in 0..20_000 {
        v.push(rng.next_u32());
    }
    compare_confuse("deep-op2", &v, 2);
    compare_confuse("deep-op3", &v, 3);
    compare_confuse("deep-op0", &v, 0);
}

#[test]
fn deep_op_out_of_range_wide() {
    let mut rng = Rng::new(0xD0005);
    let data: Vec<u32> = (0..2000).map(|_| rng.next_u32()).collect();
    for op in [4i32, 5, -1, -2, 1000, i32::MIN, i32::MAX, 0x7fff_fffe, -0x7fff_ffff] {
        compare_confuse(&format!("deep-op-oob{op}"), &data, op);
    }
}

// ---------------------------------------------------------------------------
// update_flags / process_buffer wide sweeps.
// ---------------------------------------------------------------------------

fn sweep_update(im: &Impl, params: &[i32], tag: &str) -> (Vec<u32>, Vec<u8>) {
    let mut sc = Scratch::new();
    let ptr = sc.ptr();
    let uf = im.update_flags;
    pass(tag, || {
        let mut rets = Vec::with_capacity(params.len());
        for &param in params {
            unsafe {
                (*ptr).flags = 0x7B05;
                uf(ptr, param);
                rets.push((*ptr).flags);
            }
        }
        rets
    })
}

#[test]
fn deep_update_flags_wide() {
    let mut params: Vec<i32> = (-70_000i32..=70_000).collect();
    let mut rng = Rng::new(0xD0006);
    for _ in 0..100_000 {
        params.push(rng.next_i32());
    }
    let p = pair();
    let (rc, oc) = sweep_update(&p.c, &params, "upd-c");
    let (rr, or) = sweep_update(&p.r, &params, "upd-r");
    if rc != rr {
        let i = rc.iter().zip(&rr).position(|(a, b)| a != b).unwrap();
        panic!(
            "update_flags(param={}) flags mismatch: C={:#010x} {:?} Rust={:#010x} {:?}",
            params[i],
            rc[i],
            decode_flags(rc[i]),
            rr[i],
            decode_flags(rr[i])
        );
    }
    assert_eq!(oc.len(), or.len(), "update_flags stdout length differs");
    assert!(oc == or, "update_flags stdout differs");
    // update_flags prints exactly 2 lines per call.
    assert_eq!(
        oc.iter().filter(|&&b| b == b'\n').count(),
        params.len() * 2,
        "update_flags sweep produced the wrong number of lines - not exercising the library"
    );
    eprintln!("update_flags sweep: {} calls, {} stdout bytes matched", params.len(), oc.len());
}

fn sweep_process(im: &Impl, cases: &[(Vec<u8>, c_char)], tag: &str) -> (Vec<i32>, Vec<u8>) {
    let pb = im.process_buffer;
    pass(tag, || {
        let mut rets = Vec::with_capacity(cases.len());
        for (content, target) in cases {
            let mut buf = content.clone();
            buf.push(0);
            let mut st = ProcessState {
                flags: 0x7B05,
                data: 0,
                buffer: buf.as_mut_ptr() as *mut c_char,
                capacity: buf.len() as c_int,
            };
            rets.push(unsafe { pb(&mut st as *mut ProcessState, *target) });
        }
        rets
    })
}

#[test]
fn deep_process_buffer_wide() {
    let mut rng = Rng::new(0xD0007);
    let mut cases: Vec<(Vec<u8>, c_char)> = Vec::new();
    // Random byte soup of many lengths, every target value.
    for _ in 0..20_000 {
        let len = (rng.next_u32() % 64) as usize;
        let content: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u32() % 255) as u8).collect();
        cases.push((content, rng.range_i32(-128, 127) as c_char));
    }
    // Two-symbol alphabets: maximal density of matches and adjacent matches.
    for _ in 0..20_000 {
        let len = (rng.next_u32() % 32) as usize;
        let content: Vec<u8> = (0..len)
            .map(|_| if rng.next_u32() & 1 == 0 { b'a' } else { b'b' })
            .collect();
        cases.push((content, if rng.next_u32() & 1 == 0 { b'a' as c_char } else { b'b' as c_char }));
    }
    // Long uniform buffers: match at every position, count up to 512.
    for len in [0usize, 1, 2, 3, 255, 256, 257, 511, 512, 1000] {
        cases.push((vec![b'q'; len], b'q' as c_char));
        cases.push((vec![0xffu8; len], -1));
        cases.push((vec![0x80u8; len], -128));
    }
    // A buffer holding every non-NUL byte, probed with all 256 targets.
    let all: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    for t in -128..=127i32 {
        cases.push((all.clone(), t as c_char));
    }

    let p = pair();
    let (rc, oc) = sweep_process(&p.c, &cases, "pb-c");
    let (rr, or) = sweep_process(&p.r, &cases, "pb-r");
    if rc != rr {
        let i = rc.iter().zip(&rr).position(|(a, b)| a != b).unwrap();
        panic!(
            "process_buffer mismatch at case #{i} (target {}, len {}): C={} Rust={}",
            cases[i].1,
            cases[i].0.len(),
            rc[i],
            rr[i]
        );
    }
    assert_eq!(oc.len(), or.len(), "process_buffer stdout length differs");
    assert!(oc == or, "process_buffer stdout differs");
    // One "Operation: memchr_found" line per match found.
    let matches: i64 = rc.iter().map(|&v| v.max(0) as i64).sum();
    assert_eq!(
        oc.iter().filter(|&&b| b == b'\n').count() as i64,
        matches,
        "process_buffer sweep line count != total matches"
    );
    assert!(matches > 100_000, "process_buffer sweep found only {matches} matches");
    eprintln!("process_buffer sweep: {} cases, {matches} matches, {} stdout bytes matched", cases.len(), oc.len());
}

// ---------------------------------------------------------------------------
// `confusion` wide sweep (all four arguments, whole pipeline).
// ---------------------------------------------------------------------------
fn sweep_confusion(im: &Impl, args: &[(i32, i32, i32, i32)], tag: &str) -> (Vec<i32>, Vec<u8>) {
    let f = im.confusion;
    pass(tag, || {
        args.iter()
            .map(|&(a, b, c, d)| unsafe { f(a, b, c, d) })
            .collect::<Vec<i32>>()
    })
}

#[test]
fn deep_confusion_wide() {
    let mut rng = Rng::new(0xD0008);
    let mut args: Vec<(i32, i32, i32, i32)> = Vec::new();
    for _ in 0..40_000 {
        args.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    // param1 chosen so the union is inf/NaN/huge and arm 1 overflows `int`,
    // which then overflows `result` on the `result += confusion_result` line.
    for sign in [0u32, 1] {
        for exp in [0u32, 1, 127, 148, 150, 152, 155, 200, 254, 255] {
            for m in [0u32, 1, 0x40_0000, 0x7f_ffff] {
                let p1 = ((sign << 31) | (exp << 23) | m) as i32;
                for p2 in [0i32, 0x2A, -1, i32::MIN] {
                    for p3 in [-3i32, 0, 5, 9] {
                        for p4 in [1i32, 5, -3, i32::MAX] {
                            args.push((p1, p2, p3, p4));
                        }
                    }
                }
            }
        }
    }
    let p = pair();
    let (rc, oc) = sweep_confusion(&p.c, &args, "cf-c");
    let (rr, or) = sweep_confusion(&p.r, &args, "cf-r");
    if rc != rr {
        let i = rc.iter().zip(&rr).position(|(a, b)| a != b).unwrap();
        panic!(
            "confusion{:?} mismatch: C={} Rust={}",
            args[i], rc[i], rr[i]
        );
    }
    assert_eq!(oc.len(), or.len(), "confusion stdout length differs");
    assert!(oc == or, "confusion stdout differs");
    // confusion prints at least 7 lines per call (4 debug + counter + bitfields + final).
    assert!(
        oc.iter().filter(|&&b| b == b'\n').count() >= args.len() * 7,
        "confusion sweep produced too few lines - not exercising the library"
    );
    eprintln!("confusion sweep: {} calls, {} stdout bytes matched", args.len(), oc.len());
}

// ---------------------------------------------------------------------------
// Volume self-check: makes the other sweeps' coverage auditable.
// ---------------------------------------------------------------------------
#[test]
fn deep_volume_selfcheck() {
    let p = pair();
    let pats = structured_patterns(5, 20_000, 1);
    let (rets, out) = sweep_confuse(&p.c, &pats, 1, "vol");
    eprintln!("structured_patterns(5,20000) = {} patterns", pats.len());
    assert_eq!(rets.len(), pats.len());
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), pats.len());
    assert!(pats.len() > 30_000, "pattern set unexpectedly small: {}", pats.len());
}

/// Strided sweep of the 2^32 binary32 space through `confuse_types(state, 1)`
/// — the only hand-written numeric emulation in the translation (gcc's
/// `cvttss2si` vs Rust's saturating `as` cast).
///
/// Window-parameterised so each invocation finishes well inside libtest's
/// 60-second watchdog (whose message would otherwise be written into the
/// redirected fd 1 and corrupt the capture). `sweep_full.sh` walks the windows
/// so that, together, they cover every 8th pattern across the whole space.
///
///   SWEEP_STRIDE  step between patterns              (default 256)
///   SWEEP_BEGIN   first pattern of this window       (default 0)
///   SWEEP_CHUNKS  number of 2^19-call chunks to run  (default: to the end)
#[test]
#[ignore = "long running; run explicitly or via sweep_full.sh"]
fn deep_op1_strided_full_float_space() {
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<u64>().ok());
    let stride = env("SWEEP_STRIDE").unwrap_or(256);
    let begin = env("SWEEP_BEGIN").unwrap_or(0);
    let max_chunks = env("SWEEP_CHUNKS").unwrap_or(u64::MAX);
    const CHUNK: usize = 1 << 19;
    assert!(stride >= 1);

    let p = pair();
    let mut next: u64 = begin;
    let mut total: u64 = 0;
    let mut chunk_no = 0u64;
    let mut buf: Vec<u32> = Vec::with_capacity(CHUNK);

    while next <= u32::MAX as u64 && chunk_no < max_chunks {
        buf.clear();
        while buf.len() < CHUNK && next <= u32::MAX as u64 {
            buf.push(next as u32);
            next += stride;
        }
        let (rc, oc) = sweep_confuse(&p.c, &buf, 1, "full-c");
        let (rr, or) = sweep_confuse(&p.r, &buf, 1, "full-r");
        if rc != rr {
            let i = rc.iter().zip(&rr).position(|(a, b)| a != b).unwrap();
            panic!(
                "chunk {chunk_no}: pattern {:#010x} ({} as f32, *100 = {}): C={} Rust={}",
                buf[i],
                f32::from_bits(buf[i]),
                f32::from_bits(buf[i]) * 100.0,
                rc[i],
                rr[i]
            );
        }
        assert_eq!(
            oc.iter().filter(|&&b| b == b'\n').count(),
            buf.len(),
            "chunk {chunk_no}: C line count != call count (capture contaminated)"
        );
        assert_eq!(
            or.iter().filter(|&&b| b == b'\n').count(),
            buf.len(),
            "chunk {chunk_no}: Rust line count != call count (capture contaminated)"
        );
        if oc != or {
            let i = oc.iter().zip(&or).position(|(a, b)| a != b).unwrap_or(0);
            let call = oc[..i].iter().filter(|&&b| b == b'\n').count();
            let ls = oc[..i].iter().rposition(|&b| b == b'\n').map_or(0, |q| q + 1);
            let ce = oc[ls..].iter().position(|&b| b == b'\n').map_or(oc.len(), |q| ls + q);
            let re = or[ls..].iter().position(|&b| b == b'\n').map_or(or.len(), |q| ls + q);
            let bits = buf[call.min(buf.len() - 1)];
            panic!(
                "chunk {chunk_no}: stdout diverges at call #{call} (pattern {:#010x}, f32 = {:?})\n  C   = {:?}\n  Rust= {:?}",
                bits,
                f32::from_bits(bits),
                String::from_utf8_lossy(&oc[ls..ce]),
                String::from_utf8_lossy(&or[ls..re])
            );
        }
        total += buf.len() as u64;
        chunk_no += 1;
    }
    eprintln!(
        "SWEEP OK: stride={stride} begin={begin} chunks={chunk_no} patterns={total} \
         (last pattern {})",
        next.saturating_sub(stride)
    );
    assert!(total > 0, "empty sweep window");
}
