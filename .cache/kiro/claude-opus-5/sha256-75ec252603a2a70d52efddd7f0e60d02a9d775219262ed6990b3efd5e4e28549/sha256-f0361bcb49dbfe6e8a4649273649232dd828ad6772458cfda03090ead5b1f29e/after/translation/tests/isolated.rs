//! Isolated (subprocess) differential harness.
//!
//! Several rows in ERRORS.md end in `abort()` (a live `assert()`, since the
//! CMake build defines no `NDEBUG`) or in a genuine SIGSEGV. Those cannot be
//! observed from inside a test process, so each case is run in a fresh child
//! process — once against the C `.so`, once against the Rust `.so` — and the
//! parent compares the full observable outcome:
//!
//!   * process exit code and terminating signal (SIGABRT / SIGSEGV / clean)
//!   * everything the case printed on stdout (return value, `cp_error_reason`,
//!     and the produced bytes)
//!
//! For the C child, stderr is also captured so the parent can confirm *which*
//! `assert()` fired and therefore that the intended ERRORS.md row really was
//! reached (not merely "it crashed somehow").
//!
//! Run with `cargo test --release --test isolated`.

mod common;

use common::deflate::*;
use common::*;
use std::os::raw::{c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Case definitions — identical list in parent and child.
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Case {
    Inflate {
        stream: Vec<u8>,
        in_bytes: c_int,
        align: usize,
        out_bytes: c_int,
        out_cap: usize,
        null_in: bool,
    },
    Png {
        data: Vec<u8>,
        length: c_int,
    },
}

fn inf(stream: Vec<u8>, out_cap: usize) -> Case {
    let n = stream.len() as c_int;
    Case::Inflate {
        stream,
        in_bytes: n,
        align: 0,
        out_bytes: out_cap as c_int,
        out_cap,
        null_in: false,
    }
}

fn inf_n(stream: Vec<u8>, in_bytes: c_int, out_cap: usize) -> Case {
    Case::Inflate {
        stream,
        in_bytes,
        align: 0,
        out_bytes: out_cap as c_int,
        out_cap,
        null_in: false,
    }
}

/// (row label, expected C assertion function or "" / "SEGV" / "CLEAN", case)
fn cases() -> Vec<(String, &'static str, Case)> {
    let mut v: Vec<(String, &'static str, Case)> = Vec::new();

    // --- A7: in_bytes == 0 -> bits_left == 0 -> assert(s->bits_left > 0)
    v.push(("A7 in_bytes=0".into(), "cp_read_bits", inf_n(vec![0x01, 0, 0, 0], 0, 16)));

    // --- A16: in_bytes < 0
    for n in [-1i32, -4, -1000, i32::MIN] {
        v.push((
            format!("A16 in_bytes={n}"),
            "cp_read_bits",
            inf_n(vec![0x01, 0, 0, 0, 0, 0, 0, 0], n, 16),
        ));
    }

    // --- A8: stream truncated mid-block -> a read once bits_left <= 0
    {
        let data: Vec<u8> = (0..64u8).collect();
        let full = deflate_fixed_lz(&data);
        for cut in [1usize, 2, 3, 5, 9] {
            if full.len() > cut {
                let n = (full.len() - cut) as c_int;
                v.push((
                    format!("A8 truncated by {cut}"),
                    "",
                    inf_n(full.clone(), n, data.len()),
                ));
            }
        }
        // and over-declared input length (E9)
        for extra in [1i32, 2, 3, 4, 16] {
            v.push((
                format!("E9 in_bytes+{extra}"),
                "",
                inf_n(full.clone(), full.len() as c_int + extra, data.len()),
            ));
        }
    }

    // --- A9/A10: a stream that ends exactly on a block boundary but claims
    //     another block, so the header read overflows the remaining bits.
    for tail in 0..8usize {
        let mut s = vec![0x00u8]; // bfinal=0, btype=0 -> stored, then runs out
        s.extend(std::iter::repeat(0u8).take(tail));
        v.push((format!("A9/A10 stored header tail={tail}"), "", inf(s, 16)));
    }

    // --- A12: cp_ptr with bits_left not byte aligned
    //     (a non-final block followed by a stored block at an odd bit offset)
    {
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &[Tok::Lit(1), Tok::Lit(2)], false);
        // start a stored block but give it a bogus length so cp_ptr is reached
        bw.bit(1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(0xFFFF, 16);
        bw.bits(0x0000, 16);
        let s = bw.finish();
        v.push(("A12 stored after fixed".into(), "", inf(s, 64)));
    }

    // --- A13: code length >= 16 in cp_build, reached by a dynamic header whose
    //     run-length codes overflow `lens`.
    // --- A14: cp_decode prefix assertion, reached by an incomplete Huffman
    //     tree in a dynamic header.
    // Both are produced by fuzzing dynamic-block headers below.
    {
        let mut seed = 0x1234_5678u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        for i in 0..600 {
            let n = 8 + (next() % 40) as usize;
            let mut s = Vec::with_capacity(n);
            // bfinal=1, btype=2 (dynamic) in the low 3 bits of byte 0
            s.push(0x05 | ((next() as u8) & 0xF8));
            for _ in 1..n {
                s.push(next() as u8);
            }
            v.push((format!("A13/A14 fuzz dynamic header {i}"), "", inf(s, 512)));
        }
    }

    // --- A14 via fixed blocks with random bodies
    {
        let mut seed = 0x9E37_79B9u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        for i in 0..400 {
            let n = 6 + (next() % 30) as usize;
            let mut s = Vec::with_capacity(n);
            s.push(0x03 | ((next() as u8) & 0xF8)); // bfinal=1, btype=1
            for _ in 1..n {
                s.push(next() as u8);
            }
            v.push((format!("A14 fuzz fixed body {i}"), "", inf(s, 512)));
        }
    }

    // --- A17: in == NULL
    for n in [4i32, 8, 64] {
        v.push((
            format!("A17 in=NULL in_bytes={n}"),
            "SEGV",
            Case::Inflate {
                stream: Vec::new(),
                in_bytes: n,
                align: 0,
                out_bytes: 16,
                out_cap: 16,
                null_in: true,
            },
        ));
    }

    // --- cp_dynamic stack-frame overrun.
    //
    // `cp_dynamic` declares `uint8_t lens[288 + 32]` but a run-length code
    // (16/17/18) decoded near the end of the table keeps writing past it, into
    // the C's own stack frame (`lenlens`, then `sym`, `nlen`, `ndst`, `nlit`,
    // the loop counters and `n`). It also reads `lens[-1]` when symbol 16 is the
    // first code-length symbol. This is the single most delicate part of the
    // translation, so it gets dedicated hostile vectors on top of the fuzzing.
    {
        // (a) overflow by up to 137 bytes: 319 literal lengths then symbol 18
        //     (zero run of 11..138) at n == 319.
        for extra in [0u32, 1, 5, 60, 127] {
            let mut seq: Vec<(usize, u32, u32)> = (0..319).map(|_| (8usize, 0u32, 0u32)).collect();
            seq.push((18, extra, 7));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
            // some literal bits so the block body has something to chew on
            bw.bits(0, 32);
            let s = bw.finish();
            v.push((format!("cp_dynamic overflow sym18 extra={extra}"), "", inf(s, 512)));
        }
        // (b) overflow by 1..7 bytes: symbol 17 (zero run 3..10) at n == 319
        for extra in 0..8u32 {
            let mut seq: Vec<(usize, u32, u32)> = (0..319).map(|_| (8usize, 0u32, 0u32)).collect();
            seq.push((17, extra, 3));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
            bw.bits(0, 32);
            let s = bw.finish();
            v.push((format!("cp_dynamic overflow sym17 extra={extra}"), "", inf(s, 512)));
        }
        // (c) overflow via symbol 16 (repeat previous, 3..6) at n == 319
        for extra in 0..4u32 {
            let mut seq: Vec<(usize, u32, u32)> = (0..319).map(|_| (8usize, 0u32, 0u32)).collect();
            seq.push((16, extra, 2));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
            bw.bits(0, 32);
            let s = bw.finish();
            v.push((format!("cp_dynamic overflow sym16 extra={extra}"), "", inf(s, 512)));
        }
        // (d) symbol 16 as the FIRST code-length symbol: the C reads lens[-1],
        //     i.e. uninitialised stack.
        for extra in 0..4u32 {
            let mut seq: Vec<(usize, u32, u32)> = vec![(16, extra, 2)];
            while seq.len() < 40 {
                seq.push((8, 0, 0));
            }
            seq.push((18, 127, 7));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
            bw.bits(0, 32);
            let s = bw.finish();
            v.push((format!("cp_dynamic lens[-1] via sym16 first extra={extra}"), "", inf(s, 512)));
        }
        // (e) overflow starting from many different values of n, so the run
        //     lands on each interesting frame slot (lenlens, sym, nlen, ndst,
        //     nlit, the three loop counters, n itself, and beyond).
        for start in [
            250usize, 300, 310, 315, 316, 317, 318, 319, 200, 260, 280, 290, 305, 312, 314,
        ] {
            for (sym, extra, nb) in [(18usize, 127u32, 7u32), (17, 7, 3), (16, 3, 2)] {
                let mut seq: Vec<(usize, u32, u32)> =
                    (0..start).map(|_| (8usize, 0u32, 0u32)).collect();
                seq.push((sym, extra, nb));
                // keep going until the table would be full, so the loop must
                // re-read the (possibly clobbered) nlit/ndst/n
                let mut bw = BitWriter::new();
                write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
                bw.bits(0, 64);
                let s = bw.finish();
                v.push((
                    format!("cp_dynamic overflow n={start} sym={sym}"),
                    "",
                    inf(s, 512),
                ));
            }
        }
        // (f) smaller nlit/ndst so the loop terminates before the table is full
        //     but a run still overshoots (writes above nlit+ndst but inside lens)
        for (nlit, ndst) in [(257usize, 1usize), (257, 32), (270, 10), (288, 1)] {
            let target = nlit + ndst;
            let mut seq: Vec<(usize, u32, u32)> =
                (0..target.saturating_sub(1)).map(|_| (8usize, 0u32, 0u32)).collect();
            seq.push((18, 127, 7));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, nlit, ndst, &seq, true);
            bw.bits(0, 64);
            let s = bw.finish();
            v.push((
                format!("cp_dynamic overshoot nlit={nlit} ndst={ndst}"),
                "",
                inf(s, 512),
            ));
        }
        // (g) code lengths >= 16 are impossible from a 3-bit read, but a run of
        //     symbol 16 can duplicate a large value; and a length of 15 is the
        //     largest legal one. Check the boundary explicitly.
        for l in [7usize, 15] {
            if l > 7 {
                continue; // 3-bit field caps CL symbols at 7
            }
            let mut seq: Vec<(usize, u32, u32)> = (0..319).map(|_| (l, 0u32, 0u32)).collect();
            seq.push((16, 3, 2));
            let mut bw = BitWriter::new();
            write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
            bw.bits(0, 32);
            let s = bw.finish();
            v.push((format!("cp_dynamic len={l} then sym16"), "", inf(s, 512)));
        }
        // (h) POSITIVE overrun cases: a run that ends between `lens[320]` and
        //     the `ndst` slot (frame offset 356) clobbers only slots the C no
        //     longer reads (`lenlens`, its padding, `sym`, `nlen`), so the C
        //     survives the overrun and still decodes the block. These are the
        //     cases that check the emulated frame reproduces the C's *data*,
        //     not merely its crash. `extra` is swept across the boundary so the
        //     later values start clobbering `ndst`, `nlit`, the loop counters
        //     and `n`.
        {
            let lit_used: Vec<usize> = (0..288).collect();
            let lit_lens = balanced_lens(&lit_used, 288);
            let mut dst_lens = vec![0u8; 32];
            dst_lens[0] = 1;
            let mut all: Vec<u8> = lit_lens.clone();
            all.extend_from_slice(&dst_lens);
            assert_eq!(all.len(), 320);
            assert_eq!(all[319], 0, "the trailing run must write the value 0");

            let payload: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(37)).collect();
            let lc = canonical(&lit_lens);
            for extra in 0..64u32 {
                let mut seq: Vec<(usize, u32, u32)> =
                    (0..319).map(|i| (all[i] as usize, 0u32, 0u32)).collect();
                seq.push((18, extra, 7)); // zero run of 11..138 starting at n=319
                let mut bw = BitWriter::new();
                write_dynamic_header_raw(&mut bw, 288, 32, &seq, true);
                for &b in &payload {
                    bw.code(lc[b as usize], lit_lens[b as usize] as u32);
                }
                bw.code(lc[256], lit_lens[256] as u32);
                let s = bw.finish();
                // n ends at 330 + extra; 356 is the `ndst` slot
                let expect: &'static str = if 330 + extra <= 355 { "CLEAN" } else { "" };
                v.push((
                    format!("cp_dynamic survivable overrun extra={extra} (n_end={})", 330 + extra),
                    expect,
                    inf(s, 256),
                ));
            }
        }
    }

    // --- A12 sweep: try hard to reach `cp_ptr`'s `assert(!(s->bits_left & 7))`.
    //     `bits_left ≡ count (mod 8)` is invariant except across the final-word
    //     load in `cp_peak_bits` (which adds `bits_left`, not `last_bytes * 8`),
    //     so the only way in is to make that load happen in an earlier block and
    //     then enter a stored block. Sweep block-1 length, input length and
    //     input alignment systematically.
    {
        let ll = fixed_lit_lens();
        let lc = canonical(&ll);
        for nlit in 0..28usize {
            for pad in 0..4usize {
                for tail in 0..14usize {
                    let mut bw = BitWriter::new();
                    // non-final fixed block with `nlit` literals
                    bw.bit(0);
                    bw.bits(1, 2);
                    for k in 0..nlit {
                        let b = (k as u8).wrapping_mul(29);
                        bw.code(lc[b as usize], ll[b as usize] as u32);
                    }
                    bw.code(lc[256], ll[256] as u32);
                    // final stored block
                    bw.bit(1);
                    bw.bits(0, 2);
                    bw.align();
                    let len = tail as u16;
                    bw.bits(len as u32, 16);
                    bw.bits((!len) as u32, 16);
                    bw.bytes(&vec![0x77u8; tail]);
                    let s = bw.finish();
                    v.push((
                        format!("A12 sweep nlit={nlit} pad={pad} tail={tail}"),
                        "",
                        Case::Inflate {
                            stream: s.clone(),
                            in_bytes: s.len() as c_int,
                            align: pad,
                            out_bytes: 256,
                            out_cap: 256,
                            null_in: false,
                        },
                    ));
                }
            }
        }
    }

    // --- D21b: every single-byte corruption of a valid PNG
    {
        let data: Vec<u8> = (0..3 * 2 * 4).map(|i| (i * 13 + 5) as u8).collect();
        let mut s = PngSpec::new(3, 2, 6);
        s.raw = encode_scanlines(3, 2, 4, &data, &vec![0u8; 2]);
        s.deflate = DeflateMode::FixedLz;
        let png = s.build();
        for i in 0..png.len() {
            for xor in [0xFFu8, 0x01, 0x80, 0x20] {
                let mut p = png.clone();
                p[i] ^= xor;
                let len = p.len() as c_int;
                v.push((
                    format!("D21b byte {i} ^ {xor:#02x}"),
                    "",
                    Case::Png { data: p, length: len },
                ));
            }
        }
        // and truncation / over-declared length at every offset
        for cut in 1..png.len() {
            v.push((
                format!("D21b truncate -{cut}"),
                "",
                Case::Png {
                    data: png.clone(),
                    length: (png.len() - cut) as c_int,
                },
            ));
        }
        for extra in [1i32, 4, 32, 512, 4096] {
            v.push((
                format!("D21b oversized +{extra}"),
                "",
                Case::Png {
                    data: png.clone(),
                    length: png.len() as c_int + extra,
                },
            ));
        }
    }

    // --- corrupted deflate payloads inside an otherwise valid PNG
    {
        let data: Vec<u8> = (0..4 * 4 * 3).map(|i| (i * 7) as u8).collect();
        let mut base = PngSpec::new(4, 4, 2);
        base.raw = encode_scanlines(4, 4, 3, &data, &vec![0u8; 4]);
        base.deflate = DeflateMode::DynamicLz;
        let mut seed = 0xABCD_EF01u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        for i in 0..500 {
            let mut z = zlib(&deflate_dynamic_lz(&base.raw));
            let k = 2 + (next() as usize % (z.len() - 6).max(1));
            if k < z.len() {
                z[k] ^= (next() as u8) | 1;
            }
            let mut s = PngSpec::new(4, 4, 2);
            s.raw = base.raw.clone();
            s.raw_zlib = Some(z);
            let p = s.build();
            let len = p.len() as c_int;
            v.push((
                format!("deflate payload fuzz {i}"),
                "",
                Case::Png { data: p, length: len },
            ));
        }
    }

    v
}

// ---------------------------------------------------------------------------
// Child
// ---------------------------------------------------------------------------

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn child(which: &str, idx: usize) -> ! {
    let all = cases();
    let (_label, _expect, case) = all[idx].clone();
    // A CPU-time limit rather than a wall-clock one: some malformed streams make
    // `cp_inflate` loop forever, and the limit must trip at the same point for
    // both libraries regardless of how loaded the machine is. Exceeding it
    // raises SIGXCPU, whose default action terminates the process.
    unsafe {
        let lim = libc::rlimit {
            rlim_cur: 5,
            rlim_max: 5,
        };
        libc::setrlimit(libc::RLIMIT_CPU, &lim);
        // Some malformed inputs make the C walk a multi-gigabyte range; cap the
        // address space (identically for both libraries) so those cases fail
        // fast and deterministically instead of thrashing.
        let aslim = libc::rlimit {
            rlim_cur: 1 << 30,
            rlim_max: 1 << 30,
        };
        libc::setrlimit(libc::RLIMIT_AS, &aslim);
        // Hundreds of cases end in SIGABRT/SIGSEGV; writing a core dump for each
        // dominates the runtime and tells us nothing.
        let nocore = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &nocore);
    }
    let path = if which == "c" { c_so_path() } else { rust_so_path() };
    let lib = unsafe { Lib::open("child", &path) };
    unsafe {
        lib.set_error_sentinel();
        match case {
            Case::Inflate {
                stream,
                in_bytes,
                align,
                out_bytes,
                out_cap,
                null_in,
            } => {
                let inbuf = AlignedBuf::new(&stream, align);
                let outbuf = AlignedBuf::new(&vec![0u8; out_cap.max(1)], 0);
                let inp: *mut c_void = if null_in {
                    std::ptr::null_mut()
                } else {
                    inbuf.ptr as *mut c_void
                };
                let ret = (lib.cp_inflate)(inp, in_bytes, outbuf.ptr as *mut c_void, out_bytes);
                let out = std::slice::from_raw_parts(outbuf.ptr, out_cap.max(1));
                println!(
                    "ret={} err={:?} out={}",
                    ret,
                    lib.error_reason(),
                    hex(out)
                );
            }
            Case::Png { data, length } => {
                let buf = AlignedBuf::new(&data, 0);
                let img = (lib.load_png_mem)(buf.ptr, length);
                let mut pix = String::new();
                if !img.pix.is_null() {
                    let n = (img.w as i64) * (img.h as i64);
                    if n > 0 && n < 1 << 24 {
                        let s = std::slice::from_raw_parts(img.pix as *const u8, (n * 4) as usize);
                        pix = hex(s);
                    }
                }
                println!(
                    "w={} h={} null={} err={:?} pix={}",
                    img.w,
                    img.h,
                    img.pix.is_null(),
                    lib.error_reason(),
                    pix
                );
            }
        }
    }
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Parent
// ---------------------------------------------------------------------------

struct Outcome {
    status: String,
    stdout: String,
    stderr: String,
}

/// Children run with `MALLOC_PERTURB_` set. The C reads uninitialised `malloc`
/// memory whenever a malformed input makes the DEFLATE stream shorter than
/// `(w+1)*h*bpp` (`cp_unfilter`/`cp_convert` still walk the whole buffer), so
/// without it the comparison would be against unspecified heap contents rather
/// than against the C's behaviour. `MALLOC_PERTURB_` makes glibc fill freshly
/// allocated blocks with a fixed byte, which makes those reads deterministic
/// and identical for both libraries. glibc's tcache fast path skips the
/// perturbation, so `glibc.malloc.tcache_count=0` is set as well.
///
/// Each child runs under `timeout -s KILL`, because some malformed streams make
/// `cp_inflate` loop forever (a `bfinal` bit that never gets set); both
/// implementations must hang identically. `timeout` reports a signal-killed
/// child as exit code `128 + signal`, so the status string below distinguishes
/// clean exits, SIGABRT, SIGSEGV and timeouts.
/// Wall-clock backstop only; the real bound is the child's RLIMIT_CPU.
const CHILD_TIMEOUT_SECS: &str = "20";

fn norm(code: Option<i32>, signal: Option<i32>) -> String {
    if let Some(s) = signal {
        return format!("sig{s}");
    }
    match code {
        Some(0) => "clean".into(),
        Some(124) => "TIMEOUT".into(),
        Some(134) => "sig6".into(),  // SIGABRT
        Some(139) => "sig11".into(), // SIGSEGV
        Some(137) => "sig9".into(),  // SIGKILL (wall-clock backstop)
        Some(152) => "sig24".into(), // SIGXCPU (CPU limit)
        Some(c) if c > 128 => format!("sig{}", c - 128),
        Some(c) => format!("exit{c}"),
        None => "unknown".into(),
    }
}

fn run_child(exe: &str, which: &str, idx: usize) -> Outcome {
    let out = Command::new("timeout")
        .args(["-s", "KILL", CHILD_TIMEOUT_SECS, exe])
        .env("MALLOC_PERTURB_", "42")
        .env("GLIBC_TUNABLES", "glibc.malloc.tcache_count=0")
        .env("CP_ISOLATED_WHICH", which)
        .env("CP_ISOLATED_IDX", idx.to_string())
        .stdin(Stdio::null())
        .output()
        .expect("spawn child");
    Outcome {
        status: norm(out.status.code(), out.status.signal()),
        stdout: String::from_utf8_lossy(&out.stdout).trim_end().to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).trim_end().to_string(),
    }
}

fn main() {
    if let (Ok(which), Ok(idx)) = (
        std::env::var("CP_ISOLATED_WHICH"),
        std::env::var("CP_ISOLATED_IDX"),
    ) {
        child(&which, idx.parse().unwrap());
    }

    let exe = std::env::current_exe().unwrap().to_string_lossy().to_string();
    let all = cases();
    println!("running {} isolated differential cases", all.len());

    let mut failures: Vec<String> = Vec::new();
    let mut stats = std::collections::BTreeMap::<String, usize>::new();
    let mut asserts_seen = std::collections::BTreeSet::<String>::new();
    let mut clean_overruns = 0usize;

    if let Ok(only) = std::env::var("CP_ONLY") {
        let i: usize = only.parse().unwrap();
        println!("case {i}: {}", all[i].0);
        let c = run_child(&exe, "c", i);
        let r = run_child(&exe, "rust", i);
        println!("C   : {} | {}\n{}", c.status, c.stdout, c.stderr);
        println!("Rust: {} | {}\n{}", r.status, r.stdout, r.stderr);
        return;
    }

    // Every case is an independent pair of subprocesses, so run them in
    // parallel; a handful of cases deliberately run until the wall-clock
    // backstop, and serialising those would dominate the wall time.
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(4, 32))
        .unwrap_or(8);
    let results: Vec<(Outcome, Outcome)> = {
        let exe = &exe;
        let n = all.len();
        let mut slots: Vec<Option<(Outcome, Outcome)>> = (0..n).map(|_| None).collect();
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for t in 0..nthreads {
                let idxs: Vec<usize> = (0..n).filter(|i| i % nthreads == t).collect();
                handles.push(scope.spawn(move || {
                    idxs.into_iter()
                        .map(|i| (i, (run_child(exe, "c", i), run_child(exe, "rust", i))))
                        .collect::<Vec<_>>()
                }));
            }
            for h in handles {
                for (i, pair) in h.join().unwrap() {
                    slots[i] = Some(pair);
                }
            }
        });
        slots.into_iter().map(|x| x.unwrap()).collect()
    };
    println!("(ran {} case pairs on {nthreads} threads)", all.len());
    for (i, (label, expect, _)) in all.iter().enumerate() {
        let (c, r) = &results[i];

        *stats.entry(c.status.clone()).or_default() += 1;
        if std::env::var("CP_VERBOSE").is_ok() && c.status != "clean" {
            println!("#{i} {label}: C={} Rust={}", c.status, r.status);
        }

        // which assert() fired in the C, if any
        if let Some(pos) = c.stderr.find(": Assertion") {
            let head = &c.stderr[..pos];
            let f = head.rsplit(": ").next().unwrap_or("?").to_string();
            asserts_seen.insert(f.clone());
            if !expect.is_empty() && *expect != "SEGV" && f != *expect {
                failures.push(format!(
                    "[{label}] expected assert in {expect}, C asserted in {f}"
                ));
            }
        } else if *expect == "SEGV" && c.status != "sig11" {
            failures.push(format!("[{label}] expected SIGSEGV, C gave {}", c.status));
        } else if *expect == "CLEAN" {
            if c.status != "clean" {
                failures.push(format!(
                    "[#{i} {label}] expected the C to survive this overrun, got {} ({})",
                    c.status, c.stderr
                ));
            } else if !c.stdout.contains("ret=1") {
                failures.push(format!(
                    "[#{i} {label}] expected the C to decode the block, got {}",
                    c.stdout
                ));
            } else {
                clean_overruns += 1;
            }
        } else if !expect.is_empty() && *expect != "SEGV" && *expect != "CLEAN" && c.status != "sig6" {
            failures.push(format!(
                "[{label}] expected an assert in {expect}, C gave {} ({})",
                c.status, c.stderr
            ));
        }

        if c.status != r.status {
            failures.push(format!(
                "[#{i} {label}] termination differs: C={} Rust={}\n  C stderr: {}\n  Rust stderr: {}",
                c.status, r.status, c.stderr, r.stderr
            ));
        } else if c.stdout != r.stdout {
            failures.push(format!(
                "[#{i} {label}] stdout differs:\n  C   : {}\n  Rust: {}",
                c.stdout, r.stdout
            ));
        }
    }

    println!("outcome distribution: {stats:?}");
    println!("C assertions actually reached: {asserts_seen:?}");
    println!("cp_dynamic frame overruns that still decoded: {clean_overruns}");

    // Evidence that the abort rows are genuinely exercised, not just "nothing
    // crashed and everything trivially matched".
    let want_asserts = ["cp_read_bits", "cp_consume_bits", "cp_decode"];
    for w in want_asserts {
        if !asserts_seen.contains(w) {
            failures.push(format!(
                "no case reached the assert in {w}; the abort rows are untested"
            ));
        }
    }
    if stats.get("sig6").copied().unwrap_or(0) == 0 {
        failures.push("no case terminated with SIGABRT".into());
    }
    if stats.get("sig11").copied().unwrap_or(0) == 0 {
        failures.push("no case terminated with SIGSEGV".into());
    }
    if clean_overruns < 10 {
        failures.push(format!(
            "only {clean_overruns} cp_dynamic frame-overrun cases produced real output; \
             the stack-frame emulation is not positively tested"
        ));
    }
    if stats.get("clean").copied().unwrap_or(0) < all.len() / 4 {
        failures.push("suspiciously few cases completed cleanly".into());
    }

    if failures.is_empty() {
        println!("\nisolated: ok. {} cases matched", all.len());
    } else {
        eprintln!("\nisolated: {} FAILURES", failures.len());
        for f in failures.iter().take(40) {
            eprintln!("  {f}");
        }
        eprintln!("  ... {} total", failures.len());
        std::process::exit(1);
    }
}
