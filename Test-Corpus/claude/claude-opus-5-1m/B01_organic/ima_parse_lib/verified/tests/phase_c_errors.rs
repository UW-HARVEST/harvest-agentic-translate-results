//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows that make the C dereference a
//! null/wild pointer or spin forever cannot be tested in-process, so they are
//! driven by re-exec'ing this test binary as a child (see `crash_child`) and
//! comparing how the C child and the Rust child *terminate* (same signal, or
//! both timing out).

mod common;

use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::*;

// ===========================================================================
// Rows 1-3 — the three explicit rejections
// ===========================================================================

#[test]
fn row01_bad_magic_returns_minus_1() {
    let rng = Rng::new(0xC001);
    // Every 4-byte type other than "caff" must give exactly -1.
    let mut cases: Vec<[u8; 4]> = vec![
        *b"CAFF", *b"ffac", *b"caf\0", *b"\0aff", *b"caFf", *b"cafg", *b"baff",
        [0, 0, 0, 0], [0xff; 4], *b"desc", *b"ima4", *b"RIFF", *b"FORM", *b"OggS",
    ];
    for _ in 0..3000 {
        cases.push(rng.arr4());
    }

    for t in cases {
        let mut caf = Caf::new(t, 1, rng.next_u16());
        caf.desc(rng.arr4(), &Desc::ima4());
        caf.pakt(rng.arr4(), &Pakt::new(1));
        caf.data(rng.arr4(), 0, 0, &[]);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "magic {t:?}: C and Rust must agree");
        let expected = if t == *b"caff" { 0 } else { -1 };
        assert_eq!(c.ret, expected, "magic {t:?} must return {expected}");
    }
}

#[test]
fn row02_bad_version_returns_minus_2() {
    let rng = Rng::new(0xC002);
    // Exhaustive: only version 1 escapes -2 (also covered in Phase B row 30,
    // here asserted as an *error* property with a valid remainder).
    let mut caf = Caf::new(*b"caff", 1, 0);
    caf.desc([0; 4], &Desc::ima4());
    caf.pakt([0; 4], &Pakt::new(1));
    caf.data([0; 4], 0, 0, &[]);
    let mut buf = caf.buf.clone();

    for v in 0..=u16::MAX {
        buf[4..6].copy_from_slice(&v.to_be_bytes());
        let (c, r) = run_both(&buf);
        assert_eq!(c, r, "version {v:#06x}: C and Rust must agree");
        let expected = if v == 1 { 0 } else { -2 };
        assert_eq!(c.ret, expected, "version {v:#06x} must return {expected}");
    }
    let _ = rng.next_u64();
}

#[test]
fn row03_bad_format_id_returns_minus_3() {
    let rng = Rng::new(0xC003);
    let mut cases: Vec<[u8; 4]> = vec![
        *b"IMA4", *b"4ami", *b"lpcm", *b"aac ", *b"alac", *b"ima\0", *b"\0ma4",
        [0, 0, 0, 0], [0xff; 4], *b"caff", *b"desc",
    ];
    for _ in 0..3000 {
        cases.push(rng.arr4());
    }

    for fid in cases {
        let mut desc = Desc::random_ima4(&rng);
        desc.format_id = fid;
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &Pakt::random(&rng));
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[]);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "format_id {fid:?}: C and Rust must agree");
        let expected = if fid == *b"ima4" { 0 } else { -3 };
        assert_eq!(c.ret, expected, "format_id {fid:?} must return {expected}");
    }
}

// ===========================================================================
// Row 9 — no pakt chunk AND a bad format_id: -3 wins over the null deref
// ===========================================================================

#[test]
fn row09_bad_format_id_beats_null_pakt() {
    let rng = Rng::new(0xC009);
    for i in 0..2000 {
        let mut desc = Desc::random_ima4(&rng);
        // Anything but "ima4".
        desc.format_id = loop {
            let t = rng.arr4();
            if t != *b"ima4" {
                break t;
            }
        };
        // Deliberately NO pakt chunk: `pakt` stays NULL, but the -3 check
        // happens before `pakt->frame_count`, so this must return cleanly.
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[]);

        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "row09/{i}: C and Rust must agree");
        assert_eq!(
            c.ret, -3,
            "row09/{i}: -3 must be returned before `pakt` is dereferenced"
        );
        assert_eq!(c.info, InfoBuf::poisoned(), "row09/{i}: info must be untouched");
    }
}

// ===========================================================================
// Rows 14, 18, 19 — unvalidated values pass straight through
// ===========================================================================

#[test]
fn row14_data_size_unvalidated() {
    let rng = Rng::new(0xC014);
    for size in [
        0i64,
        1,
        -1,
        2,
        1 << 32,
        1 << 47,
        -(1i64 << 47),
        i64::MIN,
        i64::MAX,
        u64::MAX as i64,
    ] {
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &Desc::random_ima4(&rng));
        caf.pakt(rng.arr4(), &Pakt::random(&rng));
        caf.data(rng.arr4(), size, rng.next_u32(), &[]);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "size {size}");
        assert_eq!(c.ret, 0, "size {size} must not be rejected");
        assert_eq!(c.info.size(), size as u64, "size {size} must pass through");
    }
}

#[test]
fn row18_channel_count_unvalidated() {
    let rng = Rng::new(0xC018);
    for ch in [0u32, 1, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        let mut desc = Desc::random_ima4(&rng);
        desc.channels_per_frame = ch;
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &Pakt::random(&rng));
        caf.data(rng.arr4(), 0, 0, &[]);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "channels {ch:#010x}");
        assert_eq!(c.ret, 0, "channels {ch:#010x} must not be rejected");
        assert_eq!(c.info.channel_count(), ch);
    }
}

#[test]
fn row19_frame_count_unvalidated() {
    let rng = Rng::new(0xC019);
    for fc in [0i64, 1, -1, i64::MIN, i64::MAX, -(1i64 << 62)] {
        let mut pakt = Pakt::random(&rng);
        pakt.frame_count = fc;
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &Desc::random_ima4(&rng));
        caf.pakt(rng.arr4(), &pakt);
        caf.data(rng.arr4(), 0, 0, &[]);
        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "frame_count {fc}");
        assert_eq!(c.ret, 0, "frame_count {fc} must not be rejected");
        assert_eq!(c.info.frame_count(), fc as u64);
    }
}

// ===========================================================================
// Row 20 — truncated stream: there is no length parameter, so both
// implementations must read past the logical end *identically*.
//
// Made deterministic by placing the truncated stream at the start of a large
// zero-filled allocation: the bytes "past the end" are real, known memory, so
// the reads are in-bounds for the process even though they are past the stream.
// ===========================================================================

/// Writes a chunk directly into a pre-existing region: type, zero padding,
/// big-endian `size`, then `payload`. The walk's next stop is `off + 16 + size`.
fn plant(region: &mut [u8], off: usize, type4: [u8; 4], size: i64, payload: &[u8]) {
    region[off..off + 4].copy_from_slice(&type4);
    region[off + 4..off + 8].fill(0);
    region[off + 8..off + 16].copy_from_slice(&size.to_be_bytes());
    region[off + 16..off + 16 + payload.len()].copy_from_slice(payload);
}

#[test]
fn row20_truncated_stream_reads_past_logical_end() {
    let rng = Rng::new(0xC020);

    // The first chunk sits at offset 8 and every stride through the zero filler
    // is exactly sizeof(caf_chunk) = 16, so the walk only ever visits offsets
    // congruent to 8 mod 16. The recovery chain below is planted on that lattice
    // so the walk is guaranteed to land on it.
    const DESC_AT: usize = 1032; // 1032 % 16 == 8
    const PAKT_AT: usize = DESC_AT + 16 + DESC_PAYLOAD_LEN; // 1080
    const DATA_AT: usize = PAKT_AT + 16 + PAKT_PAYLOAD_LEN; // 1120

    for trunc_len in [8usize, 9, 12, 16, 20, 23, 24, 32, 40] {
        // 4 KiB of zeros: a zero chunk type is unknown and a zero size gives a
        // 16-byte stride, so the walk marches forward deterministically and
        // every byte it touches is real, initialised memory.
        let mut region = vec![0u8; 4096];

        // A full, valid stream, truncated to `trunc_len` bytes. `trunc_len` is
        // always < 96, so the stream's own `data` chunk is never included and
        // the walk necessarily runs past the logical end into the filler.
        let mut caf = Caf::valid_header(&rng);
        caf.desc(rng.arr4(), &Desc::ima4());
        caf.pakt(rng.arr4(), &Pakt::new(64));
        caf.data(rng.arr4(), 128, 0, &[]);
        assert!(trunc_len < 96);
        region[..trunc_len].copy_from_slice(&caf.buf[..trunc_len]);

        // Recovery chain, so the walk terminates instead of running off the
        // allocation (that variant is the `no_data_chunk` child scenario).
        plant(&mut region, DESC_AT, *b"desc", DESC_PAYLOAD_LEN as i64, &Desc::ima4().encode());
        plant(&mut region, PAKT_AT, *b"pakt", PAKT_PAYLOAD_LEN as i64, &Pakt::new(64).encode());
        plant(&mut region, DATA_AT, *b"data", 128, &[0u8; 4]);

        let (c, r) = run_both(&region);
        assert_eq!(
            c, r,
            "truncated to {trunc_len} bytes: C and Rust must read past the end identically"
        );
        assert_eq!(
            c.ret, 0,
            "truncated to {trunc_len} bytes: walk should recover on the planted chain"
        );
        assert_eq!(c.info.size(), 128);
        assert_eq!(
            c.info.blocks(),
            region.as_ptr() as u64 + (DATA_AT + BLOCKS_OFF_FROM_CHUNK) as u64
        );
    }
}

// ===========================================================================
// Rows 4-8, 10-13 — faults and hangs, verified in child processes
// ===========================================================================

const SCENARIO_ENV: &str = "IMA_DIFF_SCENARIO";
const IMPL_ENV: &str = "IMA_DIFF_IMPL";
const CHILD_TEST: &str = "crash_child";
const SIGSEGV: i32 = 11;
const SIGBUS: i32 = 7;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum ChildResult {
    Exited(i32),
    Signalled(i32),
    TimedOut,
}

/// Builds the stream for a scenario. `Some(buf)` streams are passed as `data`.
/// Scenarios needing a null pointer return `None`.
fn scenario_buffer(name: &str) -> Option<Vec<u8>> {
    let rng = Rng::new(0xDEAD_BEEF);
    match name {
        // Row 4/5/6: null pointers.
        "null_data" | "null_both" => None,
        "null_info" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.desc([0; 4], &Desc::ima4());
            caf.pakt([0; 4], &Pakt::new(1));
            caf.data([0; 4], 16, 0, &[]);
            Some(caf.buf)
        }

        // Row 7: a data chunk with no preceding desc -> desc == NULL.
        "desc_null" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.pakt([0; 4], &Pakt::new(1));
            caf.data([0; 4], 16, 0, &[]);
            Some(caf.buf)
        }

        // Row 8: valid ima4 desc but no pakt -> pakt == NULL.
        "pakt_null" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.desc([0; 4], &Desc::ima4());
            caf.data([0; 4], 16, 0, &[]);
            Some(caf.buf)
        }

        // Row 10: no data chunk anywhere, so `for(;;)` never breaks.
        //
        // 4 KiB of zeros gives an unknown chunk type (0) and a zero size, i.e. a
        // 16-byte stride, so the walk marches deterministically along the
        // offsets congruent to 8 mod 16. The final in-region slot then jumps
        // 1 TiB forward, which is reliably unmapped.
        //
        // Letting the walk simply run off the end of the allocation instead
        // would NOT be a fair comparison: the first unmapped page after a heap
        // block differs per process, and if the wild address happens to land in
        // a thread's stack guard page, Rust's runtime SIGSEGV handler reports a
        // stack overflow and calls abort() (SIGABRT) rather than re-raising
        // SIGSEGV. That is a property of the process's memory map, not of the
        // library, so the jump target is pinned instead.
        "no_data_chunk" => {
            let mut buf = vec![0u8; 4096];
            buf[0..4].copy_from_slice(b"caff");
            buf[4..6].copy_from_slice(&1u16.to_be_bytes());
            let last = 4072; // 4072 % 16 == 8, and 4072 + 16 <= 4096
            buf[last..last + 4].copy_from_slice(b"free");
            buf[last + 8..last + 16].copy_from_slice(&(1i64 << 40).to_be_bytes());
            Some(buf)
        }

        // Row 11: size == -16 makes chunk += 16 - 16, i.e. no progress at all.
        "self_loop" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.chunk_raw(*b"free", [0; 4], -16, &[]);
            Some(caf.buf)
        }

        // Row 12: a hugely negative size walks far below the buffer.
        "backward_oob" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.chunk_raw(*b"free", [0; 4], -(1i64 << 40), &[]);
            Some(caf.buf)
        }

        // Row 13: size at the extremes of i64 overflows the pointer.
        "size_i64_max" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.chunk_raw(*b"free", [0; 4], i64::MAX, &[]);
            Some(caf.buf)
        }
        "size_i64_min" => {
            let mut caf = Caf::new(*b"caff", 1, 0);
            caf.chunk_raw(*b"free", [0; 4], i64::MIN, &[]);
            Some(caf.buf)
        }

        other => panic!("unknown scenario {other} (rng={})", rng.next_u64()),
    }
}

/// The child half: runs one scenario against one implementation. Expected to
/// fault or hang for most scenarios.
#[test]
#[ignore = "child-process helper, driven by the fault/hang differential tests"]
fn crash_child() {
    let name = std::env::var(SCENARIO_ENV).expect("scenario env var");
    let which = std::env::var(IMPL_ENV).expect("impl env var");
    let f = match which.as_str() {
        "c" => c_ima_parse(),
        "rust" => rust_ima_parse(),
        other => panic!("bad impl {other}"),
    };

    let buf = scenario_buffer(&name);
    let data: *const c_void = match &buf {
        Some(b) => b.as_ptr().cast(),
        None => std::ptr::null(),
    };

    let ret = unsafe {
        if name == "null_info" || name == "null_both" {
            // Pass a NULL `info` (and, for null_both, a NULL `data` too).
            f(std::ptr::null_mut(), data)
        } else {
            let mut info = InfoBuf::poisoned();
            let p: *mut c_void = info.0.as_mut_ptr().cast();
            f(p, data)
        }
    };

    // Reached only if the C did *not* fault; the exit code carries the result
    // so the parent can compare it.
    println!("NO-FAULT ret={ret}");
    std::process::exit(if ret == 0 { 100 } else { 100 - (-ret) });
}

fn run_child(scenario: &str, which: &str, timeout: Duration) -> ChildResult {
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args(["--exact", CHILD_TEST, "--ignored", "--nocapture", "--test-threads=1"])
        .env(SCENARIO_ENV, scenario)
        .env(IMPL_ENV, which)
        // Keep the child from dumping core for every fault scenario.
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");

    let start = Instant::now();
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                return match status.signal() {
                    Some(sig) => ChildResult::Signalled(sig),
                    None => ChildResult::Exited(status.code().unwrap_or(-1)),
                };
            }
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return ChildResult::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

/// Asserts the C child and the Rust child terminate the same way.
#[track_caller]
fn assert_same_termination(scenario: &str, timeout: Duration) -> ChildResult {
    let c = run_child(scenario, "c", timeout);
    let r = run_child(scenario, "rust", timeout);
    assert_eq!(
        c, r,
        "scenario `{scenario}`: C terminated as {c:?} but Rust terminated as {r:?}"
    );
    c
}

/// For scenarios whose faulting address is *exact and canonical* (a NULL
/// pointer, or a small offset from NULL): both must die on the very same signal.
#[track_caller]
fn assert_both_fault(scenario: &str) {
    let res = assert_same_termination(scenario, Duration::from_secs(20));
    match res {
        ChildResult::Signalled(SIGSEGV) | ChildResult::Signalled(SIGBUS) => {}
        other => panic!("scenario `{scenario}`: expected SIGSEGV/SIGBUS in both, got {other:?}"),
    }
}

/// For scenarios that fault on a *wild* address produced by unchecked pointer
/// arithmetic.
///
/// Both implementations must fault, but the exact signal is not a property of
/// the library: on x86-64 an address whose high bits are not a sign extension of
/// bit 47 is *non-canonical*, and touching it raises a general-protection fault
/// that Linux reports as SIGBUS, whereas a canonical-but-unmapped address raises
/// a page fault reported as SIGSEGV. Which side of that line
/// `chunk + 16 + size` lands on depends on how the compiler happens to fold the
/// arithmetic (observed: the default `-O0` build and `-O2`/`-O3`/`-Os` give
/// SIGSEGV, `-O1` gives SIGBUS for `size == i64::MAX`). So both are required to
/// die by one of those two signals, rather than by the identical one.
#[track_caller]
fn assert_both_fault_wild_address(scenario: &str) {
    let timeout = Duration::from_secs(20);
    let c = run_child(scenario, "c", timeout);
    let r = run_child(scenario, "rust", timeout);
    for (label, res) in [("C", c), ("Rust", r)] {
        match res {
            ChildResult::Signalled(SIGSEGV) | ChildResult::Signalled(SIGBUS) => {}
            other => panic!(
                "scenario `{scenario}`: {label} should have faulted on a wild address, \
                 got {other:?} (C={c:?}, Rust={r:?})"
            ),
        }
    }
}

#[track_caller]
fn assert_both_hang(scenario: &str) {
    let res = assert_same_termination(scenario, Duration::from_secs(3));
    assert_eq!(
        res,
        ChildResult::TimedOut,
        "scenario `{scenario}`: expected both to loop forever"
    );
}

#[test]
fn row04_null_data_faults_in_both() {
    assert_both_fault("null_data");
}

#[test]
fn row05_null_info_faults_in_both() {
    assert_both_fault("null_info");
}

#[test]
fn row06_null_data_and_info_faults_in_both() {
    assert_both_fault("null_both");
}

#[test]
fn row07_null_desc_faults_in_both() {
    assert_both_fault("desc_null");
}

#[test]
fn row08_null_pakt_faults_in_both() {
    assert_both_fault("pakt_null");
}

#[test]
fn row10_no_data_chunk_walks_off_the_end_in_both() {
    assert_both_fault_wild_address("no_data_chunk");
}

/// Companion to row 10, in-process and fully deterministic: the loop has no
/// termination condition other than a `data` chunk, so it must walk an
/// arbitrarily long run of unknown chunks. Both implementations must traverse
/// the identical path and stop at the identical place.
#[test]
fn row10_long_unknown_chunk_walk_is_identical() {
    for slots in [1usize, 2, 3, 16, 255, 4096, 65535] {
        // `slots` unknown chunks of stride 16, then the recovery chain.
        let region_len = 8 + slots * 16 + 16 + DESC_PAYLOAD_LEN + 16 + PAKT_PAYLOAD_LEN + 32;
        let mut region = vec![0u8; region_len];
        region[0..4].copy_from_slice(b"caff");
        region[4..6].copy_from_slice(&1u16.to_be_bytes());

        let desc_at = 8 + slots * 16;
        let pakt_at = desc_at + 16 + DESC_PAYLOAD_LEN;
        let data_at = pakt_at + 16 + PAKT_PAYLOAD_LEN;
        plant(&mut region, desc_at, *b"desc", DESC_PAYLOAD_LEN as i64, &Desc::ima4().encode());
        plant(&mut region, pakt_at, *b"pakt", PAKT_PAYLOAD_LEN as i64, &Pakt::new(9).encode());
        plant(&mut region, data_at, *b"data", 77, &[0u8; 4]);

        let (c, r) = run_both(&region);
        assert_eq!(c, r, "walk over {slots} unknown chunks diverged");
        assert_eq!(c.ret, 0, "walk over {slots} unknown chunks should succeed");
        assert_eq!(c.info.size(), 77);
        assert_eq!(c.info.frame_count(), 9);
        assert_eq!(
            c.info.blocks(),
            region.as_ptr() as u64 + (data_at + BLOCKS_OFF_FROM_CHUNK) as u64
        );
    }
}

#[test]
fn row11_chunk_size_minus_16_loops_forever_in_both() {
    assert_both_hang("self_loop");
}

#[test]
fn row12_hugely_negative_chunk_size_faults_in_both() {
    assert_both_fault_wild_address("backward_oob");
}

#[test]
fn row13_chunk_size_i64_extremes_fault_in_both() {
    assert_both_fault_wild_address("size_i64_max");
    assert_both_fault_wild_address("size_i64_min");
}

// ===========================================================================
// Row 16 / generic FFI boundary — there is no `enum` in this ABI, so the
// closest analogue of an out-of-range discriminant is an unrecognised 32-bit
// chunk type. Exhaustively checked for near-misses of every known code.
// ===========================================================================

#[test]
fn row16_unrecognised_chunk_types_are_skipped_identically() {
    let rng = Rng::new(0xC016);

    // Every single-byte mutation of every known fourcc, plus random codes.
    let mut types: Vec<[u8; 4]> = Vec::new();
    for known in [*b"desc", *b"pakt", *b"data", *b"caff", *b"ima4"] {
        for pos in 0..4 {
            for b in 0..=255u8 {
                let mut t = known;
                t[pos] = b;
                types.push(t);
            }
        }
    }
    for _ in 0..2000 {
        types.push(rng.arr4());
    }

    for t in types {
        // Filler chunk of the type under test, between a valid desc/pakt pair
        // and the terminating data chunk. Its payload is a valid ima4 desc so
        // that even a `desc` mutation stays in bounds and well defined.
        let mut caf = Caf::new(*b"caff", 1, 0);
        caf.desc([0; 4], &Desc::ima4());
        caf.pakt([0; 4], &Pakt::new(7));
        caf.chunk(t, [0; 4], &Desc::ima4().encode());
        caf.data([0; 4], 21, 0, &[]);

        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "chunk type {t:?} diverged");
        if !is_known_type(t) {
            assert_eq!(c.ret, 0, "unknown chunk type {t:?} must simply be skipped");
        }
    }
}

// ===========================================================================
// Return-code exhaustiveness: the only values ima_parse may ever return
// ===========================================================================

#[test]
fn return_codes_are_only_0_minus1_minus2_minus3() {
    let rng = Rng::new(0xC0DE);
    let mut seen = std::collections::BTreeSet::new();

    for _ in 0..20_000 {
        // Randomly valid or invalid in each independent dimension.
        let magic = if rng.below(3) == 0 { rng.arr4() } else { *b"caff" };
        let version = if rng.below(3) == 0 { rng.next_u16() } else { 1 };
        let fid = if rng.below(3) == 0 { rng.arr4() } else { *b"ima4" };

        let mut desc = Desc::random_ima4(&rng);
        desc.format_id = fid;

        let mut caf = Caf::new(magic, version, rng.next_u16());
        caf.desc(rng.arr4(), &desc);
        caf.pakt(rng.arr4(), &Pakt::random(&rng));
        caf.data(rng.arr4(), rng.next_u64() as i64, rng.next_u32(), &[]);

        let (c, r) = run_both(&caf.buf);
        assert_eq!(c, r, "diverged: magic={magic:?} version={version} fid={fid:?}");
        seen.insert(c.ret);

        // Check the documented precedence order.
        let expected = if magic != *b"caff" {
            -1
        } else if version != 1 {
            -2
        } else if fid != *b"ima4" {
            -3
        } else {
            0
        };
        assert_eq!(
            c.ret, expected,
            "precedence wrong for magic={magic:?} version={version} fid={fid:?}"
        );
    }

    assert_eq!(
        seen,
        [0, -1, -2, -3].into_iter().collect::<std::collections::BTreeSet<i32>>(),
        "all four exit codes must be exercised"
    );
}
