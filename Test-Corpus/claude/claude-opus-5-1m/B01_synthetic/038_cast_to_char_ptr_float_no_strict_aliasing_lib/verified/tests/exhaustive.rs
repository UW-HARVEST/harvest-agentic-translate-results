//! EXHAUSTIVE differential verification of the whole input domain.
//!
//! `driver` takes a single 32-bit argument and has no state, no options and no
//! other inputs, so the entire input domain is the 2^32 `float` bit patterns.
//! That is small enough to enumerate completely, which turns Phase B from
//! "sampled" into "proved for every possible input".
//!
//! This test is `#[ignore]`d because a full sweep takes roughly an hour; the
//! sampled rows in `tests/differential.rs` are what the normal suite runs.
//!
//! Run it with:
//!
//! ```sh
//! cargo build --offline                       # cdylib is NOT rebuilt by cargo test
//! cargo test --offline --test exhaustive -- --ignored --nocapture
//! ```
//!
//! Environment knobs:
//! * `EXHAUSTIVE_START` / `EXHAUSTIVE_END` -- inclusive/exclusive `u64` bounds of
//!   the bit-pattern range to sweep (default `0` .. `1<<32`), so the sweep can be
//!   split across several invocations.
//! * `EXHAUSTIVE_CHUNK` -- values compared per capture (default `1<<20`).

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr::null_mut;

use libloading::Library;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

type DriverFn = unsafe extern "C" fn(f32);
const REC: usize = 9;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    std::env::var("HARVEST_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir().join("c_src/build/libdriver.so"))
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|d| d.parent()) {
            let c = dir.join("libdriver.so");
            if c.exists() {
                return c;
            }
        }
    }
    manifest_dir().join("target/debug/libdriver.so")
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Redirect fd 1 to `file`, run `f`, restore, and return the bytes written.
fn capture_into<F: FnOnce()>(path: &PathBuf, buf: &mut Vec<u8>, f: F) {
    let file = File::create(path).expect("create capture file");
    unsafe { fflush(null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0);
    f();
    unsafe {
        fflush(null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
    }
    drop(file);
    buf.clear();
    let mut fh = File::open(path).expect("reopen capture file");
    std::io::Read::read_to_end(&mut fh, buf).expect("read capture file");
}

#[test]
#[ignore = "exhaustive sweep of all 2^32 inputs; takes ~1h, run explicitly"]
fn exhaustive_full_domain() {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    assert!(c_path.exists(), "missing {}", c_path.display());
    assert!(r_path.exists(), "missing {} (run `cargo build`)", r_path.display());

    let (c_lib, r_lib);
    let (cf, rf): (DriverFn, DriverFn) = unsafe {
        c_lib = Library::new(&c_path).expect("dlopen C");
        r_lib = Library::new(&r_path).expect("dlopen Rust");
        (
            *c_lib.get::<DriverFn>(b"driver\0").expect("C driver"),
            *r_lib.get::<DriverFn>(b"driver\0").expect("Rust driver"),
        )
    };

    let start = env_u64("EXHAUSTIVE_START", 0);
    let end = env_u64("EXHAUSTIVE_END", 1u64 << 32);
    let chunk = env_u64("EXHAUSTIVE_CHUNK", 1 << 20).max(1);
    assert!(start < end && end <= (1u64 << 32), "bad range {start}..{end}");

    let c_tmp = std::env::temp_dir().join(format!("exh-c-{}.out", std::process::id()));
    let r_tmp = std::env::temp_dir().join(format!("exh-r-{}.out", std::process::id()));
    let mut c_buf = Vec::with_capacity((chunk as usize) * REC);
    let mut r_buf = Vec::with_capacity((chunk as usize) * REC);

    let t0 = std::time::Instant::now();
    let mut done: u64 = 0;
    let mut base = start;
    while base < end {
        let n = chunk.min(end - base);

        capture_into(&c_tmp, &mut c_buf, || {
            for k in 0..n {
                unsafe { cf(f32::from_bits((base + k) as u32)) };
            }
        });
        capture_into(&r_tmp, &mut r_buf, || {
            for k in 0..n {
                unsafe { rf(f32::from_bits((base + k) as u32)) };
            }
        });

        assert_eq!(
            c_buf.len(),
            n as usize * REC,
            "C reference stream for chunk at {base:#x} is {} bytes, expected {} \
             -- capture polluted (run single-threaded)",
            c_buf.len(),
            n as usize * REC
        );

        if c_buf != r_buf {
            let off = c_buf
                .iter()
                .zip(r_buf.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(c_buf.len().min(r_buf.len()));
            let idx = off / REC;
            let bits = (base + idx as u64) as u32;
            let lo = idx * REC;
            panic!(
                "EXHAUSTIVE divergence at bits {bits:#010x} (f32 {:?}), byte offset {off}\n  \
                 C   : {:?}\n  Rust: {:?}",
                f32::from_bits(bits),
                String::from_utf8_lossy(&c_buf[lo..(lo + REC).min(c_buf.len())]),
                String::from_utf8_lossy(&r_buf[lo.min(r_buf.len())..(lo + REC).min(r_buf.len())]),
            );
        }

        done += n;
        base += n;
        let pct = 100.0 * done as f64 / (end - start) as f64;
        let el = t0.elapsed().as_secs_f64();
        let eta = if done > 0 { el / done as f64 * ((end - start) - done) as f64 } else { 0.0 };
        eprint!(
            "\rexhaustive: {done}/{} ({pct:.2}%) elapsed {el:.0}s eta {eta:.0}s   ",
            end - start
        );
        let _ = std::io::stderr().flush();
    }
    let _ = std::fs::remove_file(&c_tmp);
    let _ = std::fs::remove_file(&r_tmp);
    eprintln!(
        "\nexhaustive: VERIFIED {done} bit patterns in {:.0}s -- C and Rust agree byte-for-byte \
         over the swept domain [{start:#x}, {end:#x})",
        t0.elapsed().as_secs_f64()
    );
}
