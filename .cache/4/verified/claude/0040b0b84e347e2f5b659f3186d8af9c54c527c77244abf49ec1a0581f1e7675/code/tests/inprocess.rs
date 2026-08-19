//! In-process differential test (`harness = false`, so this binary owns fd 1 and
//! nothing else writes to it).
//!
//! Both shared libraries — the C build of `c_src/src/main.c` and the Rust
//! `cdylib` — are loaded into THIS process with `libloading` and their exported
//! `driver` symbol is called through the FFI boundary. stdout is captured by
//! `dup2`'ing a temp file over fd 1, so the comparison sees the exact bytes each
//! library writes (C `printf` buffering included).
//!
//! Progress/diagnostics go to stderr; the process exits non-zero on any
//! divergence.

mod common;

use common::*;
use std::io::{Read, Seek, Write};
use std::os::raw::{c_int, c_void};
use std::os::unix::io::AsRawFd;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// Same libc instance the dlopen'ed C library writes through.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with fd 1 redirected into a temp file and returns everything that
/// was written to it.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = tmp_dir().join(format!("inproc-{}.out", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    // flush every C stream (the C library's `printf` output) — the Rust library
    // flushes its own `std::io::stdout` inside `driver`.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };

    file.rewind().expect("rewind");
    let mut v = Vec::new();
    file.read_to_end(&mut v).expect("read capture");
    let _ = std::fs::remove_file(&path);
    v
}

type DriverFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

struct Lib {
    name: &'static str,
    _lib: libloading::Library,
    driver: DriverFn,
}

fn load(name: &'static str, path: &std::path::Path) -> Lib {
    // SAFETY: both libraries are plain C-ABI shared objects.
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
    let driver: DriverFn = unsafe {
        let s: libloading::Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {path:?}: {e}"));
        *s
    };
    // `main` must resolve as well (symbol parity through dlsym); it is exercised
    // per-process by tests/differential.rs.
    unsafe {
        let _: libloading::Symbol<MainFn> = lib
            .get(b"main\0")
            .unwrap_or_else(|e| panic!("dlsym main in {path:?}: {e}"));
    }
    Lib {
        name,
        _lib: lib,
        driver,
    }
}

fn main() {
    let c = load("C", &c_lib_path());
    let r = load("Rust", &rust_lib_path());
    eprintln!("loaded both shared libraries via libloading (dlsym driver+main OK)");

    let mut failures = 0usize;
    let mut checks = 0usize;

    let check = |vals: &[i32], label: &str, failures: &mut usize, checks: &mut usize| {
        let cout = capture(|| {
            for v in vals {
                unsafe { (c.driver)(*v) };
            }
        });
        let rout = capture(|| {
            for v in vals {
                unsafe { (r.driver)(*v) };
            }
        });
        *checks += vals.len();
        if cout != rout {
            *failures += 1;
            let cl: Vec<&[u8]> = cout.split(|&b| b == b'\n').collect();
            let rl: Vec<&[u8]> = rout.split(|&b| b == b'\n').collect();
            let mut shown = 0;
            for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
                if a != b && shown < 5 {
                    shown += 1;
                    eprintln!(
                        "FAIL [{label}] driver({}) {} != {} ",
                        vals.get(i).copied().unwrap_or_default(),
                        String::from_utf8_lossy(a),
                        String::from_utf8_lossy(b)
                    );
                }
            }
            if shown == 0 {
                eprintln!(
                    "FAIL [{label}] output lengths differ: {}={} {}={}",
                    c.name,
                    cout.len(),
                    r.name,
                    rout.len()
                );
            }
        }
        // independent oracle cross-check on the C output
        let mut want = Vec::new();
        for v in vals {
            want.extend_from_slice(&expected_image(*v));
        }
        if cout != want {
            *failures += 1;
            eprintln!("FAIL [{label}] C output disagrees with the struct-image oracle");
        }
    };

    // CONFIGS C1/C2/C3: zero, small values, extremes
    check(
        &[0, 1, -1, 2, -2, 3, -3, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1],
        "C1-C3 fixed",
        &mut failures,
        &mut checks,
    );

    // CONFIGS C4: byte / nibble patterns
    let pats: Vec<i32> = [
        0x0000_00ffu32,
        0x0000_ff00,
        0x00ff_0000,
        0xff00_0000,
        0x7f7f_7f7f,
        0x8080_8080,
        0xdead_beef,
        0xffff_ffff,
        0x0123_4567,
        0x89ab_cdef,
        0xfedc_ba98,
        0x7654_3210,
        0x0f0f_0f0f,
        0xf0f0_f0f0,
    ]
    .iter()
    .map(|&u| u as i32)
    .collect();
    check(&pats, "C4 byte patterns", &mut failures, &mut checks);

    // CONFIGS C5: powers of two and neighbours
    let mut pow = Vec::new();
    for s in 0..32u32 {
        let v = 1u32.wrapping_shl(s) as i32;
        pow.extend_from_slice(&[v, v.wrapping_neg(), v.wrapping_sub(1), v.wrapping_add(1)]);
    }
    check(&pow, "C5 powers of two", &mut failures, &mut checks);

    // CONFIGS C6/C7: many randomized values, all in ONE process
    let mut rng = Rng::new(0x0000_1111_2222_3333);
    let rand_vals: Vec<i32> = (0..20000).map(|_| rng.next_i32()).collect();
    check(&rand_vals, "C6/C7 random", &mut failures, &mut checks);

    // interleaved calls: C, Rust, C, Rust, ... in one process, so that neither
    // library's stream state can hide a divergence
    let mut rng = Rng::new(0x9999_8888_7777_6666);
    for i in 0..64 {
        let v = rng.next_i32();
        let cout = capture(|| unsafe { (c.driver)(v) });
        let rout = capture(|| unsafe { (r.driver)(v) });
        checks += 1;
        if cout != rout || cout != expected_image(v) {
            failures += 1;
            eprintln!(
                "FAIL [interleaved #{i}] driver({v}) C={:?} Rust={:?}",
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
        }
    }

    eprintln!("in-process differential checks: {checks} driver calls compared");
    if failures > 0 {
        eprintln!("{failures} FAILING group(s)");
        std::process::exit(1);
    }
    eprintln!("in-process differential test: OK");
}
