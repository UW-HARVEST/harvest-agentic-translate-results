//! Phase C — error-path differential tests, one test per `ERRORS.md` row.

mod common;
use common::*;

use std::ffi::c_void;
use std::process::Command;

// ===========================================================================
// Row 1 — header->type != 'caff'  =>  -1
// ===========================================================================
#[test]
fn err_row1_bad_magic() {
    let l = Libs::load();
    let rng = Rng::new(0x0001_0001);

    // Hand-picked near-misses (all written big-endian into bytes 0..4).
    let near: Vec<u32> = vec![
        0x0000_0000,             // all zero
        0xffff_ffff,             // all ones
        0x6361_6667,             // "cafg"
        0x6361_4666,             // "caFf"
        0x4361_6666,             // "Caff"
        0x6361_6665,             // "cafe"
        0x6666_6163,             // "ffac" (byte-reversed)
        0x6361_6600,             // "caf\0"
        0x0063_6166,             // "\0caf"
        0x6361_6676,             // "cafv"
        FOURCC_CAFF ^ 0x0000_0001,
        FOURCC_CAFF ^ 0x0100_0000,
        FOURCC_CAFF ^ 0x8000_0000,
    ];
    for (i, &ty) in near.iter().enumerate() {
        let mut d = Doc::with_header(&rng, ty, 1);
        d.pad(&rng, 128);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x7E);
        assert_eq!(crc, rrc, "row1 near i={i} ty={ty:#010x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row1 near i={i}: info mismatch");
        assert_eq!(crc, -1, "row1 near i={i} ty={ty:#010x}: expected -1");
    }

    // Randomized sweep: also feeds *valid* magic occasionally so both sides
    // agree on the boundary, not just on rejection.
    for i in 0..20_000 {
        let ty = if i % 97 == 0 { FOURCC_CAFF } else { rng.next_u32() };
        let mut d = Doc::with_header(&rng, ty, 1);
        // Enough trailing bytes for the version check + a data chunk so the
        // valid-magic iterations do not run off the buffer.
        d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, 34, 1);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x7E);
        assert_eq!(crc, rrc, "row1 rand i={i} ty={ty:#010x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row1 rand i={i}: info mismatch");
        if ty != FOURCC_CAFF {
            assert_eq!(crc, -1, "row1 rand i={i} ty={ty:#010x}");
        }
    }
}

// ===========================================================================
// Row 2 — header->version != 1  =>  -2   (exhaustive over all 65536 values)
// ===========================================================================
#[test]
fn err_row2_bad_version() {
    let l = Libs::load();
    let rng = Rng::new(0x0002_0002);
    for &v in &[0u16, 2, 3, 0x0100, 0x00ff, 0x7fff, 0x8000, 0xfffe, 0xffff] {
        let mut d = Doc::with_header(&rng, FOURCC_CAFF, v);
        d.pad(&rng, 128);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x1D);
        assert_eq!(crc, rrc, "row2 v={v:#06x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row2 v={v:#06x}: info mismatch");
        assert_eq!(crc, -2, "row2 v={v:#06x}: expected -2");
    }
}

#[test]
fn err_row2_bad_version_exhaustive() {
    let l = Libs::load();
    let rng = Rng::new(0x0002_BEEF);
    // One shared buffer; only bytes 4..6 change, so this is fast.
    let mut d = Doc::new(&rng);
    d.desc(&rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
    d.pakt(&rng, rng.next_u64());
    d.data(&rng, 34, 1);
    let mut buf = d.bytes;
    for v in 0u32..=0xffff {
        let v = v as u16;
        buf[4..6].copy_from_slice(&v.to_be_bytes());
        let ((crc, cb), (rrc, rb)) = l.call_both(&buf, 0x2E);
        assert_eq!(crc, rrc, "row2ex v={v:#06x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row2ex v={v:#06x}: info mismatch");
        let want = if v == 1 { 0 } else { -2 };
        assert_eq!(crc, want, "row2ex v={v:#06x}");
    }
}

// ===========================================================================
// Row 3 — desc->format_id != 'ima4'  =>  -3
// ===========================================================================
#[test]
fn err_row3_bad_format_id() {
    let l = Libs::load();
    let rng = Rng::new(0x0003_0003);

    let near: Vec<u32> = vec![
        0x0000_0000,
        0xffff_ffff,
        0x696d_6135,             // "ima5"
        0x494d_4134,             // "IMA4"
        0x696d_6100,             // "ima\0"
        0x3461_6d69,             // "4ami" (reversed)
        0x696d_6234,             // "imb4"
        FOURCC_IMA4 ^ 0x0000_0001,
        FOURCC_IMA4 ^ 0x0100_0000,
        FOURCC_IMA4 ^ 0x0000_0100,
        0x616c_6163,             // "alac"
        0x6c70_636d,             // "lpcm"
    ];
    for (i, &fmt) in near.iter().enumerate() {
        let mut d = Doc::new(&rng);
        d.desc(&rng, rng.next_u64(), fmt, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, 34, 1);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x3F);
        assert_eq!(crc, rrc, "row3 near i={i} fmt={fmt:#010x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row3 near i={i}: info mismatch");
        assert_eq!(crc, -3, "row3 near i={i} fmt={fmt:#010x}: expected -3");
    }

    for i in 0..20_000 {
        let fmt = if i % 89 == 0 { FOURCC_IMA4 } else { rng.next_u32() };
        let mut d = Doc::new(&rng);
        d.desc(&rng, rng.next_u64(), fmt, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, rng.next_u64() as i64, 1);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x3F);
        assert_eq!(crc, rrc, "row3 rand i={i} fmt={fmt:#010x}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row3 rand i={i}: info mismatch");
        if fmt != FOURCC_IMA4 {
            assert_eq!(crc, -3, "row3 rand i={i} fmt={fmt:#010x}");
        }
    }
}

// `-3` must win over a missing `pakt`: the format check happens BEFORE
// `pakt->frame_count` is dereferenced, so a bad format id with pakt == NULL
// must still return -3 (and must NOT crash) in both libraries.
#[test]
fn err_row3_bad_format_id_precedes_null_pakt() {
    let l = Libs::load();
    let rng = Rng::new(0x0003_FEED);
    for i in 0..200 {
        let mut bad = rng.next_u32();
        if bad == FOURCC_IMA4 {
            bad ^= 0x55;
        }
        let mut d = Doc::new(&rng);
        d.desc(&rng, rng.next_u64(), bad, rng.next_u32());
        d.data(&rng, 34, 1); // no pakt at all
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x4B);
        assert_eq!(crc, rrc, "row3/pakt i={i}");
        assert_eq!(cb, rb, "row3/pakt i={i}: info mismatch");
        assert_eq!(crc, -3, "row3/pakt i={i}");
    }
}

// ===========================================================================
// Row 10 — truncated / zero-length buffers.
// ===========================================================================
#[test]
fn err_row10_truncated_buffer() {
    let l = Libs::load();
    let rng = Rng::new(0x000A_000A);

    // A 4 KiB zeroed backing store; we present shorter and shorter prefixes.
    // Everything the C reads past the "logical" end is still inside the
    // allocation and identical for both calls, so the comparison is valid.
    for logical_len in 0..40usize {
        let backing = vec![0u8; 4096];
        let slice = &backing[..logical_len];
        let ((crc, cb), (rrc, rb)) = l.call_both(slice, 0x6C);
        assert_eq!(crc, rrc, "row10 zeros len={logical_len}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row10 zeros len={logical_len}: info mismatch");
        // all-zero magic -> -1
        assert_eq!(crc, -1, "row10 zeros len={logical_len}");
    }

    // "caff" magic followed by nothing but zeros -> version 0 -> -2.
    for logical_len in 0..40usize {
        let mut backing = vec![0u8; 4096];
        backing[0..4].copy_from_slice(&FOURCC_CAFF.to_be_bytes());
        let slice = &backing[..logical_len];
        let ((crc, cb), (rrc, rb)) = l.call_both(slice, 0x6C);
        assert_eq!(crc, rrc, "row10 caff len={logical_len}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row10 caff len={logical_len}: info mismatch");
        assert_eq!(crc, -2, "row10 caff len={logical_len}");
    }

    // Random 8-byte headers presented as a 0-length slice of a big allocation:
    // proves the header read itself (and only it) decides the outcome even when
    // the caller claims the buffer is empty.
    for i in 0..2_000 {
        let mut backing = vec![0u8; 4096];
        rng.fill(&mut backing[..8]);
        // Guard: a random header that happened to be a *valid* one would send
        // the unbounded chunk scan off the end of the mapping (ERRORS row 6,
        // covered out-of-process).  Keep this row on the rejection paths.
        if backing[0..4] == FOURCC_CAFF.to_be_bytes() {
            backing[4] = 0xff;
            backing[5] = 0xff;
        }
        let empty = &backing[..0];
        let ((crc, cb), (rrc, rb)) = l.call_both(empty, 0x6C);
        assert_eq!(crc, rrc, "row10 empty i={i}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row10 empty i={i}: info mismatch");
        assert!(crc == -1 || crc == -2, "row10 empty i={i}: rc={crc}");
    }

    // A *complete* but minimally sized valid document (8 + 16 + 32 + 16 + 24 +
    // 16 + 4 = 116 bytes, zero blocks) and every truncation of it that still
    // errors out in the header.
    let doc = valid_doc(&rng, 0, 0, 0, 0, 0);
    assert_eq!(doc.len(), 8 + 16 + 32 + 16 + 24 + 16 + 4);
    // Only prefixes that stop inside the header are safe in-process: a prefix
    // of >= 6 bytes carries the valid `version` too and would start the
    // unbounded chunk scan (ERRORS row 6 -- tested out-of-process instead).
    for cut in [0usize, 1, 2, 3, 4, 5] {
        let mut backing = vec![0u8; 4096];
        backing[..cut].copy_from_slice(&doc[..cut]);
        let s = &backing[..cut];
        let ((crc, cb), (rrc, rb)) = l.call_both(s, 0x6C);
        assert_eq!(crc, rrc, "row10 cut={cut}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "row10 cut={cut}: info mismatch");
    }
    // ...and the untruncated one, which must succeed identically.
    l.assert_same("row10 full-minimal", &doc);
}

// ===========================================================================
// Row 11 — `double -> unsigned long long` UB matrix (bit-exact).
//
// This is the same conversion Phase B rows 11-14 cover, re-asserted here as an
// explicit error-surface row because the C behaviour is undefined and therefore
// the single most likely place for the Rust to diverge.
// ===========================================================================
#[test]
fn err_row11_sample_rate_ub_matrix() {
    let l = Libs::load();
    let rng = Rng::new(0x000B_000B);
    let two63 = 9223372036854775808.0f64;

    let mut values: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        two63,
        two63 - 1024.0,
        two63 + 2048.0,
        two63 * 2.0,
        two63 * 2.0 - 2048.0,
        -two63,
        -two63 - 2048.0,
        -two63 * 2.0,
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0000),
    ];
    // Random exponents biased towards the out-of-range regions.
    for _ in 0..2_000 {
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let exp = 60 + rng.below(20) as i32; // 2^60 .. 2^79
        let mant = 1.0 + (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        values.push(sign * mant * (exp as f64).exp2());
    }

    for (i, &v) in values.iter().enumerate() {
        let raw = v.to_bits();
        let mut d = Doc::new(&rng);
        d.desc(&rng, raw, FOURCC_IMA4, rng.next_u32());
        d.pakt(&rng, rng.next_u64());
        d.data(&rng, 34, 1);
        let ((crc, cb), (rrc, rb)) = l.call_both(&d.bytes, 0x11);
        assert_eq!(crc, rrc, "row11 i={i} bits={raw:#018x}");
        assert_eq!(
            cb, rb,
            "row11 i={i} v={v:e} bits={raw:#018x}\n C ={}\n R ={}",
            hex(&cb),
            hex(&rb)
        );
    }
}

// ===========================================================================
// Rows 4, 5, 6, 8, 9 — out-of-process crash-parity tests.
//
// These conditions make the C dereference a NULL pointer (or loop off the end
// of the buffer).  Both libraries must fail the *same* way, which can only be
// observed from a separate process.  The parent re-executes this very test
// binary with `IMA_CRASH_CASE=<lib>:<case>` and compares the exit statuses.
// ===========================================================================

fn valid_data_chunk_only(rng: &Rng) -> Vec<u8> {
    // valid header, then a `data` chunk immediately -> desc stays NULL
    let mut d = Doc::new(rng);
    d.data(rng, 34, 2);
    d.bytes
}

fn desc_then_data(rng: &Rng) -> Vec<u8> {
    // valid header, valid desc (ima4), then `data` -> pakt stays NULL
    let mut d = Doc::new(rng);
    d.desc(rng, rng.next_u64(), FOURCC_IMA4, rng.next_u32());
    d.data(rng, 34, 2);
    d.bytes
}

fn full_valid(rng: &Rng) -> Vec<u8> {
    valid_doc(rng, rng.next_u64(), 2, 1234, 34, 2)
}

extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
}

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: isize = -1;

/// A region of zero-size unknown chunks followed by a `PROT_NONE` guard page.
///
/// This makes ERRORS row 6 *deterministic*: the scan advances exactly 16 bytes
/// per iteration (`sizeof(struct caf_chunk)`), and the very first read that
/// leaves the data region lands in the guard page, which is nowhere near the
/// thread stack.  Both libraries therefore fault at the same address with plain
/// SIGSEGV, independently of ASLR.
struct GuardedWalk {
    base: *mut u8,
    len: usize,
}

impl GuardedWalk {
    fn new(data_pages: usize) -> GuardedWalk {
        const PAGE: usize = 4096;
        let total = (data_pages + 1) * PAGE;
        let p = unsafe {
            mmap(
                core::ptr::null_mut(),
                total,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(p as isize, MAP_FAILED, "mmap failed");
        let base = p as *mut u8;
        let len = data_pages * PAGE;
        unsafe {
            core::ptr::write_bytes(base, 0, len);
            let s = core::slice::from_raw_parts_mut(base, len);
            s[0..4].copy_from_slice(&FOURCC_CAFF.to_be_bytes());
            s[4..6].copy_from_slice(&1u16.to_be_bytes());
            let mut off = 8;
            while off + 16 <= len {
                s[off..off + 4].copy_from_slice(&0x5858_5858u32.to_be_bytes()); // "XXXX"
                s[off + 8..off + 16].copy_from_slice(&0u64.to_be_bytes()); // size 0
                off += 16;
            }
            // Guard page: the scan must die the moment it steps past `len`.
            assert_eq!(
                mprotect(base.add(len) as *mut c_void, PAGE, PROT_NONE),
                0,
                "mprotect failed"
            );
        }
        GuardedWalk { base, len }
    }
}

/// The worker half of the crash tests.  Does nothing unless `IMA_CRASH_CASE`
/// is set, so it is a no-op during a normal `cargo test` run.
#[test]
fn crash_worker() {
    let spec = match std::env::var("IMA_CRASH_CASE") {
        Ok(s) => s,
        Err(_) => return,
    };
    let (which, case) = spec.split_once(':').expect("IMA_CRASH_CASE=<lib>:<case>");
    let l = Libs::load();
    let f = if which == "c" { l.c_parse } else { l.r_parse };
    let rng = Rng::new(0xDEAD_BEEF);
    let mut info = ImaInfo::poisoned(0x00);

    let rc = match case {
        "null_desc" => {
            let b = valid_data_chunk_only(&rng);
            unsafe { f(&mut info, b.as_ptr() as *const c_void) }
        }
        "null_pakt" => {
            let b = desc_then_data(&rng);
            unsafe { f(&mut info, b.as_ptr() as *const c_void) }
        }
        "null_info" => {
            let b = full_valid(&rng);
            unsafe { f(core::ptr::null_mut(), b.as_ptr() as *const c_void) }
        }
        "null_data" => unsafe { f(&mut info, core::ptr::null()) },
        "no_data" => {
            let g = GuardedWalk::new(64); // 256 KiB of chunks + a PROT_NONE page
            let _ = g.len;
            unsafe { f(&mut info, g.base as *const c_void) }
        }
        other => panic!("unknown crash case {other}"),
    };
    // If we get here the call did NOT crash — report the return code so the
    // parent can compare that instead.
    println!("SURVIVED rc={rc}");
    std::process::exit(70);
}

/// Run `crash_worker` in a child process for `which` = "c" | "r".
/// Returns `(exit_code, signal, stdout)`.
fn run_crash_child(which: &str, case: &str, timeout_secs: u64) -> (Option<i32>, Option<i32>, String) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args(["crash_worker", "--exact", "--nocapture", "--test-threads=1"])
        .env("IMA_CRASH_CASE", format!("{which}:{case}"))
        .env("C_SO", c_so_path())
        .env("RUST_SO", rust_so_path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn crash worker");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                let mut out = String::new();
                if let Some(mut s) = child.stdout.take() {
                    use std::io::Read;
                    let _ = s.read_to_string(&mut out);
                }
                return (status.code(), status.signal(), out);
            }
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (None, Some(-1), "TIMEOUT".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    }
}

fn assert_crash_parity(case: &str, timeout_secs: u64) {
    assert_crash_parity_ex(case, timeout_secs, true)
}

/// `exact_signal == false` means: only require that both children died the same
/// *kind* of death (abnormal termination), not with the identical signal number.
/// This is needed for the runaway-scan case, where the scan walks off the end of
/// the mapping and whether the faulting address lands in a plain unmapped page
/// (SIGSEGV) or in the thread's stack guard page (which the Rust test harness'
/// own SIGSEGV handler turns into `abort()` / SIGABRT) depends purely on ASLR --
/// it varies run-to-run for the *C* library too, so it is not a property of the
/// translation.
fn assert_crash_parity_ex(case: &str, timeout_secs: u64, exact_signal: bool) {
    let (cc, cs, co) = run_crash_child("c", case, timeout_secs);
    let (rc, rs, ro) = run_crash_child("r", case, timeout_secs);
    let survived = co.contains("SURVIVED") || ro.contains("SURVIVED");
    let ctx = format!(
        "crash parity mismatch for `{case}`:\n  C   code={cc:?} signal={cs:?} out={co:?}\n  Rust code={rc:?} signal={rs:?} out={ro:?}"
    );
    assert_eq!(
        co.contains("SURVIVED"),
        ro.contains("SURVIVED"),
        "{ctx}"
    );
    if exact_signal {
        assert_eq!(cs, rs, "{ctx}");
    } else {
        // Both must have died abnormally (never a clean exit, never a timeout).
        for (who, sig) in [("C", cs), ("Rust", rs)] {
            let sig = sig.unwrap_or_else(|| panic!("{who} child did not die abnormally: {ctx}"));
            assert!(
                sig == 11 || sig == 6 || sig == 4 || sig == 7,
                "{who} child died with unexpected signal {sig}: {ctx}"
            );
        }
    }
    if survived {
        // Both survived: the printed return codes must match exactly.
        assert_eq!(
            co.lines().find(|l| l.starts_with("SURVIVED")),
            ro.lines().find(|l| l.starts_with("SURVIVED")),
            "both survived `{case}` but with different return codes"
        );
    } else {
        assert_eq!(
            cs,
            Some(libc_sigsegv()),
            "expected SIGSEGV from the C library for `{case}`, got code={cc:?} signal={cs:?}"
        );
    }
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

#[test]
fn err_row4_null_desc_both_crash() {
    assert_crash_parity("null_desc", 30);
}

#[test]
fn err_row5_null_pakt_both_crash() {
    assert_crash_parity("null_pakt", 30);
}

#[test]
fn err_row8_null_info_both_crash() {
    assert_crash_parity("null_info", 30);
}

#[test]
fn err_row9_null_data_both_crash() {
    assert_crash_parity("null_data", 30);
}

#[test]
fn err_row6_no_data_chunk_hangs() {
    // The scan has no termination condition other than finding a `data` chunk,
    // so it walks off the end of the buffer in BOTH libraries.  See the comment
    // on `assert_crash_parity_ex` for why the signal number itself is not a
    // stable property here.
    assert_crash_parity("no_data", 60);
}

/// The *deterministic* half of ERRORS row 6 / CONFIGS row 6: a 4 MiB buffer
/// filled with zero-size unknown chunks, with the `data` chunk placed at the
/// very last 16-byte slot.  Both libraries must walk all 262 000-odd iterations
/// and agree exactly on where they stopped -- this pins the runaway-scan stride
/// without relying on a fault.
#[test]
fn err_row6_long_walk_terminates_identically() {
    let l = Libs::load();
    let rng = Rng::new(0x0006_0006);
    for &total in &[64 * 1024usize, 1024 * 1024, 4 * 1024 * 1024] {
        let mut v = vec![0u8; total];
        rng.fill(&mut v);
        v[0..4].copy_from_slice(&FOURCC_CAFF.to_be_bytes());
        v[4..6].copy_from_slice(&1u16.to_be_bytes());
        // Every slot is a size-0 unknown chunk ("XXXX") so the cursor advances
        // by exactly sizeof(struct caf_chunk) == 16.
        let mut off = 8;
        let mut slots = Vec::new();
        while off + 16 <= total {
            v[off..off + 4].copy_from_slice(&0x5858_5858u32.to_be_bytes());
            v[off + 8..off + 16].copy_from_slice(&0u64.to_be_bytes());
            slots.push(off);
            off += 16;
        }
        // desc + pakt need real bodies, so give them dedicated (non-size-0)
        // slots at the front: desc at slots[0] (size 32 -> skips 2 slots),
        // pakt right after it (size 24).
        let desc_off = slots[0];
        v[desc_off..desc_off + 4].copy_from_slice(&FOURCC_DESC.to_be_bytes());
        v[desc_off + 8..desc_off + 16].copy_from_slice(&32i64.to_be_bytes());
        let sr = rng.next_u64();
        v[desc_off + 16..desc_off + 24].copy_from_slice(&sr.to_le_bytes());
        v[desc_off + 24..desc_off + 28].copy_from_slice(&FOURCC_IMA4.to_be_bytes());
        let ch = rng.next_u32();
        v[desc_off + 40..desc_off + 44].copy_from_slice(&ch.to_be_bytes());
        let pakt_off = desc_off + 16 + 32;
        v[pakt_off..pakt_off + 4].copy_from_slice(&FOURCC_PAKT.to_be_bytes());
        v[pakt_off + 8..pakt_off + 16].copy_from_slice(&24i64.to_be_bytes());
        let fc = rng.next_u64();
        v[pakt_off + 24..pakt_off + 32].copy_from_slice(&fc.to_be_bytes());
        // ...and re-stamp the slots those two bodies overlap so the walk is
        // undisturbed after them.
        let resume = pakt_off + 16 + 24;
        let mut off = resume;
        while off + 16 <= total {
            v[off..off + 4].copy_from_slice(&0x5858_5858u32.to_be_bytes());
            v[off + 8..off + 16].copy_from_slice(&0u64.to_be_bytes());
            off += 16;
        }
        // The last full slot reachable from `resume` becomes the `data` chunk.
        let mut last = resume;
        while last + 32 <= total {
            last += 16;
        }
        let ds = rng.next_u64() as i64;
        v[last..last + 4].copy_from_slice(&FOURCC_DATA.to_be_bytes());
        v[last + 8..last + 16].copy_from_slice(&ds.to_be_bytes());

        let ((crc, cb), (rrc, rb)) = l.call_both(&v, 0x66);
        assert_eq!(crc, rrc, "long walk total={total}: rc {crc} vs {rrc}");
        assert_eq!(cb, rb, "long walk total={total}: info mismatch");
        assert_eq!(crc, 0, "long walk total={total}: expected success");
        assert_eq!(
            u64::from_le_bytes(cb[8..16].try_into().unwrap()),
            ds as u64,
            "long walk total={total}: info->size"
        );
        assert_eq!(
            u64::from_le_bytes(cb[0..8].try_into().unwrap()) as usize,
            v.as_ptr() as usize + last + CHUNK_HDR + 4,
            "long walk total={total}: blocks pointer"
        );
        assert_eq!(
            u32::from_le_bytes(cb[32..36].try_into().unwrap()),
            ch,
            "long walk total={total}: channel_count"
        );
        assert_eq!(
            u64::from_le_bytes(cb[24..32].try_into().unwrap()),
            fc,
            "long walk total={total}: frame_count"
        );
    }
}
