//! Differential runner.
//!
//! `diffrunner <lib.so> <corpus-file>` loads *one* shared library through
//! `dlopen`/`dlsym` (never linking against it) and replays every record of the
//! corpus against it, printing one deterministic line per record.
//!
//! Each record is executed in a forked child, so a crash / `assert()` abort is
//! reported as a signal instead of ending the run.  The runner is
//! single-threaded, which is what makes `fork()` safe here (a `cargo test`
//! harness thread pool is not).
//!
//! Running this program twice, once per library, and diffing stdout is the
//! byte-for-byte differential test.

use libloading::Library;
use std::ffi::CStr;
use std::io::Write;
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut u8,
}

type FnLoad = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
type FnInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

// ---------------------------------------------------------------------------
// record tags (kept in sync with tests/common/mod.rs)
// ---------------------------------------------------------------------------
const TAG_PNG: u8 = 0;
const TAG_PNG_LEN: u8 = 1;
const TAG_PNG_NULL: u8 = 2;
const TAG_INFLATE: u8 = 3;
const TAG_INFLATE_NULL_IN: u8 = 4;
const TAG_INFLATE_NULL_OUT: u8 = 5;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

fn rd_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

struct Lib {
    _lib: Library,
    load: FnLoad,
    inflate: FnInflate,
    err: *mut *const c_char,
}

impl Lib {
    fn open(path: &str) -> Lib {
        unsafe {
            let lib = Library::new(path).expect("dlopen failed");
            let load: libloading::Symbol<FnLoad> = lib.get(b"load_png_mem\0").unwrap();
            let inflate: libloading::Symbol<FnInflate> = lib.get(b"cp_inflate\0").unwrap();
            let err: libloading::Symbol<*mut *const c_char> =
                lib.get(b"cp_error_reason\0").unwrap();
            let load = *load;
            let inflate = *inflate;
            let err = *err;
            Lib {
                _lib: lib,
                load,
                inflate,
                err,
            }
        }
    }

    fn reason(&self) -> String {
        unsafe {
            let p = *self.err;
            if p.is_null() {
                "(null)".to_string()
            } else {
                match CStr::from_ptr(p).to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => "(non-utf8)".to_string(),
                }
            }
        }
    }
}

/// Runs one record and returns the report string.
fn run_record(lib: &Lib, rec: &[u8]) -> String {
    let tag = rec[0];
    let body = &rec[1..];
    unsafe {
        match tag {
            TAG_PNG => {
                let img = (lib.load)(body.as_ptr(), body.len() as c_int);
                report_img(lib, img)
            }
            TAG_PNG_LEN => {
                let n = rd_i32(body, 0);
                let data = &body[4..];
                // pass the *real* buffer with a possibly-bogus length
                let img = (lib.load)(data.as_ptr(), n);
                report_img(lib, img)
            }
            TAG_PNG_NULL => {
                let n = rd_i32(body, 0);
                let img = (lib.load)(std::ptr::null(), n);
                report_img(lib, img)
            }
            TAG_INFLATE => {
                let out_bytes = rd_i32(body, 0);
                let align = rd_i32(body, 4);
                let in_len = rd_i32(body, 8);
                let data = &body[12..];
                // malloc a 4-aligned block, then offset by `align` so that
                // `first_bytes` in cp_inflate takes every value 0..3.
                let raw = libc::malloc(data.len() + 8) as *mut u8;
                let inp = raw.add(align as usize % 4);
                std::ptr::copy_nonoverlapping(data.as_ptr(), inp, data.len());
                let out = if out_bytes > 0 {
                    libc::calloc(out_bytes as usize, 1) as *mut u8
                } else {
                    libc::calloc(1, 1) as *mut u8
                };
                let ret = (lib.inflate)(
                    inp as *mut c_void,
                    in_len,
                    out as *mut c_void,
                    out_bytes,
                );
                let n = if out_bytes > 0 { out_bytes as usize } else { 0 };
                let h = fnv1a(std::slice::from_raw_parts(out, n));
                format!(
                    "inflate ret={} outhash=0x{:016x} err={}",
                    ret,
                    h,
                    lib.reason()
                )
            }
            TAG_INFLATE_NULL_IN => {
                let out_bytes = rd_i32(body, 0);
                let in_len = rd_i32(body, 4);
                let out = libc::calloc(if out_bytes > 0 { out_bytes as usize } else { 1 }, 1)
                    as *mut u8;
                let ret = (lib.inflate)(
                    std::ptr::null_mut(),
                    in_len,
                    out as *mut c_void,
                    out_bytes,
                );
                let n = if out_bytes > 0 { out_bytes as usize } else { 0 };
                let h = fnv1a(std::slice::from_raw_parts(out, n));
                format!(
                    "inflate ret={} outhash=0x{:016x} err={}",
                    ret,
                    h,
                    lib.reason()
                )
            }
            TAG_INFLATE_NULL_OUT => {
                let out_bytes = rd_i32(body, 0);
                let in_len = rd_i32(body, 4);
                let data = &body[8..];
                let raw = libc::malloc(data.len() + 8) as *mut u8;
                std::ptr::copy_nonoverlapping(data.as_ptr(), raw, data.len());
                let ret = (lib.inflate)(
                    raw as *mut c_void,
                    in_len,
                    std::ptr::null_mut(),
                    out_bytes,
                );
                format!("inflate ret={} outhash=(null) err={}", ret, lib.reason())
            }
            other => format!("bad-tag {other}"),
        }
    }
}

fn report_img(lib: &Lib, img: CpImage) -> String {
    unsafe {
        if img.pix.is_null() {
            format!(
                "img w={} h={} pix=null hash=- err={}",
                img.w,
                img.h,
                lib.reason()
            )
        } else {
            // cp_convert / cp_depalette write exactly w*h pixels = w*h*4 bytes.
            // The remaining h*4 bytes of the (w+1)*h*4 allocation keep whatever
            // the inflate step / malloc left there, which is not deterministic
            // for short streams, so they are not hashed.
            let n = (img.w as i64) * (img.h as i64) * 4;
            let n = if n < 0 { 0 } else { n as usize };
            let h = fnv1a(std::slice::from_raw_parts(img.pix, n));
            let s = format!(
                "img w={} h={} pix=ok hash=0x{:016x} err={}",
                img.w,
                img.h,
                h,
                lib.reason()
            );
            libc::free(img.pix as *mut c_void);
            s
        }
    }
}

/// Fills the allocator's free lists with a known pattern.
///
/// `load_png_mem` hands back a `malloc`'d pixel buffer that a short/truncated
/// DEFLATE stream only partially fills, so part of the result is *uninitialised
/// heap*.  That content differs between the C-runner process and the
/// Rust-runner process simply because their allocation histories differ, which
/// would look like a translation bug.  Priming both processes' free lists with
/// the same byte makes those reads deterministic and comparable; running the
/// same library under two different patterns identifies the records whose result
/// depends on uninitialised memory at all (see `diff_corpus`).
fn prime_heap(pattern: u8) {
    if pattern == 0 {
        return;
    }
    const SIZES: &[usize] = &[
        8, 16, 24, 30, 32, 40, 48, 56, 64, 72, 80, 96, 112, 120, 128, 160, 192, 256, 320, 384, 512,
        640, 768, 1024, 1280, 1536, 2048, 3072, 4096, 6144, 8192, 12288, 16384, 32768, 65536,
        131072,
    ];
    let mut blocks = Vec::new();
    unsafe {
        for _ in 0..6 {
            for &s in SIZES {
                let p = libc::malloc(s);
                if !p.is_null() {
                    libc::memset(p, pattern as c_int, s);
                    blocks.push(p);
                }
            }
        }
        for p in blocks.into_iter().rev() {
            libc::free(p);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("usage: diffrunner <lib.so> <corpus-file> [heap-prime-byte]");
        std::process::exit(2);
    }
    let prime: u8 = if args.len() == 4 {
        args[3].parse().expect("prime byte must be 0..=255")
    } else {
        0
    };
    // Many records make the assert-enabled C reference abort or segfault.  With
    // a piped `kernel.core_pattern` each of those would hand a core dump to
    // systemd-coredump, which costs ~70 ms per case; RLIMIT_CORE = 0 suppresses
    // it (and children inherit the limit).
    unsafe {
        let rl = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
        // A piped core_pattern is invoked even with RLIMIT_CORE == 0; marking the
        // process non-dumpable makes the kernel skip it entirely.
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }

    let lib = Lib::open(&args[1]);
    let blob = std::fs::read(&args[2]).expect("cannot read corpus");

    // parse: [u32 len][len bytes] ...
    let mut records: Vec<&[u8]> = Vec::new();
    let mut i = 0usize;
    while i + 4 <= blob.len() {
        let n = u32::from_le_bytes([blob[i], blob[i + 1], blob[i + 2], blob[i + 3]]) as usize;
        i += 4;
        assert!(i + n <= blob.len(), "truncated corpus");
        records.push(&blob[i..i + n]);
        i += n;
    }

    prime_heap(prime);

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for (idx, rec) in records.iter().enumerate() {
        let line = run_forked(&lib, rec);
        writeln!(out, "{idx:05} {line}").unwrap();
    }
    out.flush().unwrap();
}

/// Runs `run_record` in a forked child; returns `status=... <report>`.
fn run_forked(lib: &Lib, rec: &[u8]) -> String {
    unsafe {
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let pid = libc::fork();
        if pid == 0 {
            // child -- silence the assert/abort chatter on stderr
            libc::close(fds[0]);
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
                libc::close(devnull);
            }
            // A malformed stream can make the library loop forever (e.g. a
            // corrupt Huffman tree that decodes a zero-bit, zero-length match:
            // `while (length--)` copies nothing and no bits are consumed).  The
            // assert-enabled reference build catches those, a NDEBUG build does
            // not, so cap every child at one second -> status=signal:14.
            libc::alarm(1);
            let report = run_record(lib, rec);
            libc::alarm(0);
            let b = report.as_bytes();
            let mut off = 0usize;
            while off < b.len() {
                let w = libc::write(fds[1], b[off..].as_ptr() as *const c_void, b.len() - off);
                if w <= 0 {
                    break;
                }
                off += w as usize;
            }
            libc::close(fds[1]);
            libc::_exit(0);
        }
        libc::close(fds[1]);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let r = libc::read(fds[0], chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if r <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..r as usize]);
        }
        libc::close(fds[0]);
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        let st = if libc::WIFEXITED(status) {
            format!("exited:{}", libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            format!("signal:{}", libc::WTERMSIG(status))
        } else {
            "unknown".to_string()
        };
        let body = String::from_utf8_lossy(&buf).to_string();
        format!("status={st} {body}")
    }
}
