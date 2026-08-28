//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Rows 1-3 and 11-16 are in-process differential calls (both `.so`s, same
//! input, same expected sentinel/error code).
//!
//! Rows 4-10 are *fault* paths: the C library has no NULL checks and no bound
//! on its chunk scan, so the "expected C result" for those inputs is a specific
//! signal (or a non-terminating loop).  Those are verified by re-exec'ing this
//! very test binary as a child process (`crash_worker`), once against the C
//! `.so` and once against the Rust `.so`, and comparing how the two children
//! terminated.  Anything else would take the test harness down with them.

mod support;

use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::*;

// ===========================================================================
// Row 1 — invalid magic -> -1
// ===========================================================================
#[test]
fn err01_bad_magic() {
    let curated: &[[u8; 4]] = &[
        *b"caf\0",
        *b"cafe",
        *b"cafg",
        *b"baff",
        *b"CAFF",
        *b"ffac", // the byte-swapped fourcc
        *b"\0\0\0\0",
        [0xFF, 0xFF, 0xFF, 0xFF],
        *b"caFf",
        *b"cAff",
        *b"Caff",
        *b"cafF",
    ];
    let mut rng = Rng::new(0x2222_0001);
    for (i, &magic) in curated.iter().enumerate() {
        // A file that would otherwise parse perfectly: only the magic is wrong.
        let mut b = FileBuilder::new(Rng::new(rng.u64()), magic, 1);
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(0, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("err01 i={i} magic={magic:?}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, -1, "err01 i={i} magic={magic:?}");
        assert_eq!(o.c_info, InfoBytes::sentinel(), "err01: *info untouched");
        assert_eq!(o.r_info, InfoBytes::sentinel(), "err01: *info untouched");
    }
}

// ===========================================================================
// Row 2 — valid magic, version != 1 -> -2
// ===========================================================================
#[test]
fn err02_bad_version() {
    let curated: &[u16] = &[
        0,
        2,
        3,
        0x0100, // 1 with the wrong byte order
        0xFFFF,
        0xFFFE,
        0x8000,
        0x7FFF,
        0x0001u16.swap_bytes(),
    ];
    let mut rng = Rng::new(0x2222_0002);
    for (i, &ver) in curated.iter().enumerate() {
        assert_ne!(ver, 1);
        let mut b = FileBuilder::new(Rng::new(rng.u64()), MAGIC_CAFF, ver);
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(0, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("err02 i={i} ver=0x{ver:04x}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, -2, "err02 i={i} ver=0x{ver:04x}");
        assert_eq!(o.c_info, InfoBytes::sentinel());
        assert_eq!(o.r_info, InfoBytes::sentinel());
    }
}

// ===========================================================================
// Row 3 — desc->format_id != "ima4" -> -3
// ===========================================================================
#[test]
fn err03_bad_format_id() {
    let curated: &[[u8; 4]] = &[
        *b"ima3",
        *b"ima5",
        *b"IMA4",
        *b"4ami", // the byte-swapped fourcc
        *b"Ima4",
        *b"imA4",
        *b"\0\0\0\0",
        [0xFF, 0xFF, 0xFF, 0xFF],
        *b"alac",
        *b"lpcm",
    ];
    let mut rng = Rng::new(0x2222_0003);
    for (i, &fmt) in curated.iter().enumerate() {
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, fmt, ch);
        b.pakt(fc);
        b.data(0, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("err03 i={i} fmt={fmt:?}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, -3, "err03 i={i} fmt={fmt:?}");
        assert_eq!(o.c_info, InfoBytes::sentinel());
        assert_eq!(o.r_info, InfoBytes::sentinel());
    }
}

/// Row 3 again, but with **no `pakt` chunk at all**: the `-3` return happens
/// before `pakt->frame_count` is read, so a NULL `pakt` must not fault here.
#[test]
fn err03_bad_format_id_no_pakt() {
    let mut rng = Rng::new(0x2222_0013);
    for i in 0..2_000usize {
        let fmt = loop {
            let f = rng.fourcc();
            if f != FMT_IMA4 {
                break f;
            }
        };
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        let (sr, ch) = (rng.u64(), rng.u32());
        b.desc(sr, fmt, ch);
        b.data(rng.u64() as i64, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("err03-nopakt i={i}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, -3);
        assert_eq!(o.c_info, InfoBytes::sentinel());
    }
}

// ===========================================================================
// Row 11 — version, one step past the valid range (and exhaustively elsewhere)
// ===========================================================================
#[test]
fn err11_version_boundaries() {
    // `cfg03_version_exhaustive` covers all 65 536 values; this pins the
    // immediate neighbours of the only accepted value.
    let mut rng = Rng::new(0x2222_0011);
    for ver in [0u16, 1, 2] {
        let mut b = FileBuilder::new(Rng::new(rng.u64()), MAGIC_CAFF, ver);
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(0, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::aligned(&bytes);
        let o = assert_same(&format!("err11 ver={ver}"), &bytes, buf.ptr());
        assert_eq!(o.c_ret, if ver == 1 { 0 } else { -2 });
    }
}

// ===========================================================================
// Row 12 — fully random 32-bit magic (including, rarely, the valid one)
// ===========================================================================
#[test]
fn err12_magic_randomized() {
    let mut rng = Rng::new(0x2222_0012);
    for i in 0..20_000usize {
        let magic = rng.fourcc();
        let mut b = FileBuilder::new(Rng::new(rng.u64()), magic, 1);
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        b.data(rng.u64() as i64, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::new(&bytes, i % 8);
        let o = assert_same(&format!("err12 i={i} magic={magic:?}"), &bytes, buf.ptr());
        let expect = if magic == MAGIC_CAFF { 0 } else { -1 };
        assert_eq!(o.c_ret, expect, "err12 i={i} magic={magic:?}");
    }
}

// ===========================================================================
// Row 13 — fully random 32-bit format_id
// ===========================================================================
#[test]
fn err13_format_id_randomized() {
    let mut rng = Rng::new(0x2222_0014);
    for i in 0..20_000usize {
        let fmt = rng.fourcc();
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        b.desc(sr, fmt, ch);
        b.pakt(fc);
        b.data(rng.u64() as i64, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::new(&bytes, i % 8);
        let o = assert_same(&format!("err13 i={i} fmt={fmt:?}"), &bytes, buf.ptr());
        let expect = if fmt == FMT_IMA4 { 0 } else { -3 };
        assert_eq!(o.c_ret, expect, "err13 i={i} fmt={fmt:?}");
    }
}

// ===========================================================================
// Row 14 — out-of-range "enum" values for chunk->type across the FFI boundary
//
// `chunk->type` is an unconstrained `ima_u32_t` fed into an `if / else if`
// chain with exactly three recognised values.  Any of the other 4 294 967 293
// values must fall through to the skip branch.  `desc` and `pakt` are emitted
// *before* the fuzzed chunk, so even the 3-in-2^32 chance of hitting a
// recognised fourcc stays a well-defined (non-faulting) input.
// ===========================================================================
#[test]
fn err14_unknown_chunk_type_enum_fuzz() {
    let mut rng = Rng::new(0x2222_0015);
    for i in 0..20_000usize {
        let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
        let ds = rng.u64() as i64;
        let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
        b.desc(sr, FMT_IMA4, ch);
        b.pakt(fc);
        // 1..4 chunks with completely arbitrary 32-bit types.
        let k = rng.range_usize(1, 4);
        let mut types = Vec::new();
        for _ in 0..k {
            let t = rng.fourcc();
            types.push(t);
            let n = rng.range_usize(0, 48);
            b.unknown_sized(t, n as i64, n);
        }
        b.data(ds, 8);
        let bytes = b.finish();
        let buf = AlignedBuf::new(&bytes, i % 8);
        let o = assert_same(&format!("err14 i={i} types={types:?}"), &bytes, buf.ptr());
        // Unless a fuzzed type collided with a recognised fourcc, the file
        // parses cleanly and info->size is the data chunk's declared size.
        if !types
            .iter()
            .any(|t| *t == T_DESC || *t == T_PAKT || *t == T_DATA)
        {
            assert_eq!(o.c_ret, 0, "err14 i={i}: unknown types must be skipped");
            assert_eq!(o.c_info.size(), ds as u64, "err14 i={i}");
            assert_eq!(o.c_info.frame_count(), fc, "err14 i={i}");
            assert_eq!(o.c_info.channel_count(), ch, "err14 i={i}");
        }
    }
}

// ===========================================================================
// Row 15 — oversized / negative chunk lengths reaching info->size
// ===========================================================================
#[test]
fn err15_chunk_size_extremes() {
    const SIZES: &[i64] = &[
        0,
        1,
        -1,
        i64::MIN,
        i64::MAX,
        -16,
        -32,
        u64::MAX as i64,
        (u64::MAX >> 1) as i64,
        1 << 62,
        -(1i64 << 62),
    ];
    let mut rng = Rng::new(0x2222_0016);
    for (i, &ds) in SIZES.iter().enumerate() {
        for off in 0..8usize {
            let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
            let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
            b.desc(sr, FMT_IMA4, ch);
            b.pakt(fc);
            b.data(ds, 8);
            let bytes = b.finish();
            let buf = AlignedBuf::new(&bytes, off);
            let o = assert_same(&format!("err15 i={i} ds={ds} off={off}"), &bytes, buf.ptr());
            assert_eq!(o.c_ret, 0);
            assert_eq!(o.c_info.size(), ds as u64, "err15 i={i} ds={ds}");
        }
    }
}

// ===========================================================================
// Row 16 — misaligned `data` pointer (C casts without alignment guarantees)
// ===========================================================================
#[test]
fn err16_misaligned_pointer() {
    let mut rng = Rng::new(0x2222_0017);
    for off in 0..8usize {
        for i in 0..500usize {
            let mut b = FileBuilder::valid_header(Rng::new(rng.u64()));
            let (sr, ch, fc) = (rng.u64(), rng.u32(), rng.u64());
            b.desc(sr, FMT_IMA4, ch);
            b.pakt(fc);
            b.data(rng.u64() as i64, 8);
            let bytes = b.finish();
            let buf = AlignedBuf::new(&bytes, off);
            assert_eq!(buf.ptr() as usize % 8, off % 8);
            let o = assert_same(&format!("err16 off={off} i={i}"), &bytes, buf.ptr());
            assert_eq!(o.c_ret, 0);
        }
    }
}

// ===========================================================================
// Rows 4-10 — fault / non-termination paths, run in child processes
// ===========================================================================

/// Guard-page-backed buffer: `pages` writable pages followed by one `PROT_NONE`
/// page, so any read past the end faults at an exactly known address.
struct GuardedBuf {
    base: *mut u8,
    total: usize,
    usable: usize,
}

impl GuardedBuf {
    fn new(pages: usize) -> Self {
        let page = 4096usize;
        let usable = pages * page;
        let total = usable + page;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let base = base as *mut u8;
        let rc = unsafe { libc::mprotect(base.add(usable) as *mut c_void, page, libc::PROT_NONE) };
        assert_eq!(rc, 0, "mprotect failed");
        unsafe { std::ptr::write_bytes(base, 0, usable) };
        GuardedBuf {
            base,
            total,
            usable,
        }
    }
    /// First address inside the guard page.
    fn guard(&self) -> *mut u8 {
        unsafe { self.base.add(self.usable) }
    }
    fn write_at(&self, off: usize, bytes: &[u8]) {
        assert!(off + bytes.len() <= self.usable);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.base.add(off), bytes.len()) };
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.base as *mut c_void, self.total) };
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ChildOutcome {
    Signal(i32),
    Exit(i32),
    TimedOut,
}

/// Re-executes this test binary so that it runs only `crash_worker`, with
/// `IMA_CRASH_CASE` / `IMA_CRASH_LIB` selecting the scenario and the library.
fn run_child(case: &str, lib: &str, timeout: Duration) -> ChildOutcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args([
            "crash_worker",
            "--exact",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("IMA_CRASH_CASE", case)
        .env("IMA_CRASH_LIB", lib)
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn crash_worker child");

    let start = Instant::now();
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                return match status.signal() {
                    Some(s) => ChildOutcome::Signal(s),
                    None => ChildOutcome::Exit(status.code().unwrap_or(-1)),
                };
            }
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return ChildOutcome::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

fn fault_row(case: &str) {
    let c = run_child(case, "c", Duration::from_secs(20));
    let r = run_child(case, "rust", Duration::from_secs(20));
    assert_eq!(
        c, r,
        "[{case}] C child terminated as {c:?} but the Rust child terminated as {r:?}"
    );
    assert!(
        matches!(c, ChildOutcome::Signal(libc::SIGSEGV) | ChildOutcome::Signal(libc::SIGBUS)),
        "[{case}] expected a memory fault, got {c:?}"
    );
}

// Row 4 — the `data` chunk is reached with no preceding `desc` chunk.
#[test]
fn err04_null_desc_segv() {
    fault_row("null_desc");
}

// Row 5 — valid `desc`/`format_id` but no `pakt` chunk before `data`.
#[test]
fn err05_null_pakt_segv() {
    fault_row("null_pakt");
}

// Row 6 — `data == NULL`.
#[test]
fn err06_null_data_segv() {
    fault_row("null_data");
}

// Row 7 — `info == NULL` with a fully valid buffer (fault on the *write*).
#[test]
fn err07_null_info_segv() {
    fault_row("null_info");
}

// Row 8 — no `data` chunk anywhere: the unbounded scan walks into the guard page.
#[test]
fn err08_no_data_chunk_segv() {
    fault_row("no_data_chunk");
}

// Row 9 — a chunk whose size is -16 makes the scan stand still: infinite loop.
#[test]
fn err09_self_referential_chunk_hangs() {
    let c = run_child("self_loop", "c", Duration::from_secs(3));
    let r = run_child("self_loop", "rust", Duration::from_secs(3));
    assert_eq!(
        c, r,
        "[self_loop] C child terminated as {c:?} but the Rust child terminated as {r:?}"
    );
    assert_eq!(
        c,
        ChildOutcome::TimedOut,
        "[self_loop] both implementations must loop forever, got {c:?}"
    );
}

// Row 10 — truncated buffer: the header / first chunk header straddles the end
// of the mapping.
#[test]
fn err10_truncated_header_segv() {
    fault_row("trunc_type"); // fault while reading header->type
    fault_row("trunc_version"); // magic ok, fault while reading header->version
    fault_row("trunc_chunk"); // header ok, fault while reading chunk->type
    fault_row("unmapped"); // non-NULL but unmapped `data` pointer
}

// ---------------------------------------------------------------------------
// The child-side worker.  `#[ignore]` keeps it out of normal runs; it is only
// ever selected explicitly by `run_child`.
// ---------------------------------------------------------------------------
#[test]
#[ignore]
fn crash_worker() {
    let Ok(case) = std::env::var("IMA_CRASH_CASE") else {
        // Selected by `--ignored` without a case: nothing to do.
        return;
    };
    let which = std::env::var("IMA_CRASH_LIB").unwrap_or_else(|_| "c".to_string());
    let l = libs();
    let f: ImaParseFn = match which.as_str() {
        "c" => l.c,
        "rust" => l.rust,
        other => panic!("unknown IMA_CRASH_LIB {other}"),
    };
    let mut info = InfoBytes::sentinel();
    let info_ptr = &mut info as *mut InfoBytes as *mut c_void;
    let mut rng = Rng::new(0xC0FFEE);

    match case.as_str() {
        // ---- Row 4: header, then a `data` chunk with no `desc` before it ----
        "null_desc" => {
            let mut b = FileBuilder::valid_header(Rng::new(1));
            b.data(0, 34);
            let bytes = b.finish();
            let buf = AlignedBuf::aligned(&bytes);
            let r = unsafe { f(info_ptr, buf.ptr()) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 5: desc with a VALID format_id, then data, no pakt ----
        "null_pakt" => {
            let mut b = FileBuilder::valid_header(Rng::new(2));
            b.desc(44100f64.to_bits(), FMT_IMA4, 2);
            b.data(0, 34);
            let bytes = b.finish();
            let buf = AlignedBuf::aligned(&bytes);
            let r = unsafe { f(info_ptr, buf.ptr()) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 6: data == NULL ----
        "null_data" => {
            let r = unsafe { f(info_ptr, std::ptr::null()) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 7: info == NULL, buffer fully valid ----
        "null_info" => {
            let mut b = FileBuilder::valid_header(Rng::new(3));
            b.desc(44100f64.to_bits(), FMT_IMA4, 2);
            b.pakt(1234);
            b.data(64, 34);
            let bytes = b.finish();
            let buf = AlignedBuf::aligned(&bytes);
            let r = unsafe { f(std::ptr::null_mut(), buf.ptr()) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 8: no `data` chunk: the scan runs into the guard page ----
        // The page is all zeros, so every chunk has type 0 (unknown) and size 0
        // and the scan advances by exactly sizeof(struct caf_chunk) == 16 until
        // `chunk->size` lands in the guard page.
        "no_data_chunk" => {
            let g = GuardedBuf::new(1);
            let mut b = FileBuilder::valid_header(Rng::new(4));
            b.desc(44100f64.to_bits(), FMT_IMA4, 2);
            b.pakt(99);
            let bytes = b.finish();
            g.write_at(0, &bytes);
            let r = unsafe { f(info_ptr, g.base as *const c_void) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 9: chunk size -16 => `chunk` never advances ----
        "self_loop" => {
            let mut b = FileBuilder::valid_header(Rng::new(5));
            b.unknown_sized(*b"junk", -16, 64);
            let bytes = b.finish();
            let buf = AlignedBuf::aligned(&bytes);
            let r = unsafe { f(info_ptr, buf.ptr()) };
            println!("unexpectedly returned {r}");
        }
        // ---- Row 10: truncation variants against a guard page ----
        "trunc_type" => {
            // `data` points straight at the guard page.
            let g = GuardedBuf::new(1);
            let r = unsafe { f(info_ptr, g.guard() as *const c_void) };
            println!("unexpectedly returned {r}");
        }
        "trunc_version" => {
            // Only the 4 magic bytes are mapped; `header->version` is in the
            // guard page.
            let g = GuardedBuf::new(1);
            g.write_at(g.usable - 4, &MAGIC_CAFF);
            let p = unsafe { g.base.add(g.usable - 4) };
            let r = unsafe { f(info_ptr, p as *const c_void) };
            println!("unexpectedly returned {r}");
        }
        "trunc_chunk" => {
            // The whole 8-byte header is mapped and valid; the first chunk
            // header starts in the guard page.
            let g = GuardedBuf::new(1);
            let mut hdr = Vec::new();
            hdr.extend_from_slice(&MAGIC_CAFF);
            hdr.extend_from_slice(&1u16.to_be_bytes());
            hdr.extend_from_slice(&rng.u16().to_be_bytes());
            g.write_at(g.usable - 8, &hdr);
            let p = unsafe { g.base.add(g.usable - 8) };
            let r = unsafe { f(info_ptr, p as *const c_void) };
            println!("unexpectedly returned {r}");
        }
        "unmapped" => {
            // Non-NULL but certainly unmapped.
            let r = unsafe { f(info_ptr, 1usize as *const c_void) };
            println!("unexpectedly returned {r}");
        }
        other => panic!("unknown IMA_CRASH_CASE {other}"),
    }
    // Reaching here means the scenario did NOT fault, which itself is a
    // divergence signal the parent will see as Exit(0) for one library only.
    println!("case {case} lib {which} completed without faulting: {}", info.describe());
}
