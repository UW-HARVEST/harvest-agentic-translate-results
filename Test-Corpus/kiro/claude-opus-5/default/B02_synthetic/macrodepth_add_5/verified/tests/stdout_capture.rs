//! Phase B — rows 25..27 of `CONFIGS.md`: the bytes the three `mdcore.c`
//! helpers write to stdout, compared directly at the `.so` boundary.
//!
//! `main`'s stdout is already compared in `driver_cli.rs`, but that only ever
//! calls `use_generated(REPEAT)`. Capturing fd 1 around individual `dlsym`ed
//! calls lets every `n` and every `(a, b)` shape be checked against its
//! `printf` format string.
//!
//! fd 1 is process-global, so everything here runs inside one `#[test]`.

mod common;

use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::AsRawFd;

use common::*;

/// fd 1 is process-global; serialise every capture.
static FD1_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Redirect fd 1 to a temporary file for the duration of `f` and return the
/// bytes written. Flushes C stdio (`fflush(NULL)`) before and after; the Rust
/// `.so` uses a `LineWriter`, so each `println!` is already flushed.
fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let mut file = tempfile::anonymous();
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let value = f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        let mut buf = Vec::new();
        file.rewind_and_read(&mut buf);
        (value, buf)
    }
}

/// Minimal anonymous temp file (avoids pulling in a tempfile dependency).
mod tempfile {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::{AsRawFd, RawFd};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct Anon {
        file: std::fs::File,
        path: std::path::PathBuf,
    }

    pub fn anonymous() -> Anon {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "md-stdout-{}-{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("create temp capture file");
        Anon { file, path }
    }

    impl Anon {
        pub fn rewind_and_read(&mut self, out: &mut Vec<u8>) {
            self.file.seek(SeekFrom::Start(0)).expect("seek");
            self.file.read_to_end(out).expect("read");
        }
    }

    impl AsRawFd for Anon {
        fn as_raw_fd(&self) -> RawFd {
            self.file.as_raw_fd()
        }
    }

    impl Drop for Anon {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[test]
fn r25_r26_r27_stdout_bytes_match() {
    let (c, r) = (c_impl(), rust_impl());

    let mut pairs: Vec<(c_int, c_int)> = SMALL_PAIRS.to_vec();
    pairs.extend(BOUNDARY_PAIRS.iter().copied());
    pairs.extend(random_pairs(SEED ^ 0x99, 48));

    /* ---- row 25: helper_call -> "helper.call=%d helper.acc=%d\n" ---- */
    {
        let (cf, rf) = (c.binop("helper_call"), r.binop("helper_call"));
        for (a, b) in pairs.iter().copied() {
            let (cv, cout) = capture(|| unsafe { cf(a, b) });
            let (rv, rout) = capture(|| unsafe { rf(a, b) });
            assert_eq!(cv, rv, "[{}] helper_call({a},{b}) return", config_label());
            assert_eq!(
                cout,
                rout,
                "[{}] helper_call({a},{b}) stdout\n  C   : {:?}\n  Rust: {:?}",
                config_label(),
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
            assert!(
                cout.starts_with(b"helper.call=") && cout.ends_with(b"\n"),
                "[{}] unexpected C helper_call stdout {:?}",
                config_label(),
                String::from_utf8_lossy(&cout)
            );
        }
    }

    /* ---- row 26: helper_ptr -> "helper.ptr=%d\n" ---- */
    {
        let (cf, rf) = (c.binop("helper_ptr"), r.binop("helper_ptr"));
        for (a, b) in pairs.iter().copied() {
            let (cv, cout) = capture(|| unsafe { cf(a, b) });
            let (rv, rout) = capture(|| unsafe { rf(a, b) });
            assert_eq!(cv, rv, "[{}] helper_ptr({a},{b}) return", config_label());
            assert_eq!(
                cout,
                rout,
                "[{}] helper_ptr({a},{b}) stdout\n  C   : {:?}\n  Rust: {:?}",
                config_label(),
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
        }
    }

    /* ---- row 27: use_generated -> "gen.acc=%d\n" ---- */
    {
        let (cf, rf) = (c.unop("use_generated"), r.unop("use_generated"));
        let mut ns: Vec<c_int> = IN_RANGE_N.to_vec();
        ns.extend_from_slice(OUT_OF_RANGE_N);
        ns.push(REPEAT);
        ns.extend(random_ints(SEED ^ 0xAA, 48));
        for n in ns {
            let (cv, cout) = capture(|| unsafe { cf(n) });
            let (rv, rout) = capture(|| unsafe { rf(n) });
            assert_eq!(cv, rv, "[{}] use_generated({n}) return", config_label());
            assert_eq!(
                cout,
                rout,
                "[{}] use_generated({n}) stdout\n  C   : {:?}\n  Rust: {:?}",
                config_label(),
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
            assert_eq!(
                cout,
                format!("gen.acc={cv}\n").into_bytes(),
                "[{}] use_generated({n}) C stdout format",
                config_label()
            );
        }
    }
}

/// Sanity check that the capture machinery actually captures (otherwise the test
/// above could pass vacuously by comparing two empty buffers).
#[test]
fn capture_machinery_is_not_vacuous() {
    let c = c_impl();
    let f = c.unop("use_generated");
    let (_, out) = capture(|| unsafe { f(1) });
    assert!(!out.is_empty(), "fd-1 capture produced nothing");
    let mut s = String::new();
    (&out[..]).read_to_string(&mut s).unwrap();
    assert!(s.starts_with("gen.acc="), "captured {s:?}");
}
