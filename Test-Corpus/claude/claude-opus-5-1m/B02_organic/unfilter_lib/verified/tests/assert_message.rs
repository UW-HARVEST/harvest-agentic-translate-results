//! One-off check that glibc's `__assert_fail` prefix (`<progname>: `) is
//! reproduced byte-for-byte by `src/cassert.rs`, i.e. that the *whole* assertion
//! message matches and not merely the path-normalised tail that the differential
//! harness compares.
mod common;

use common::{libs, GuardedBuf};
use std::ffi::c_void;
use std::os::unix::io::IntoRawFd;

/// Fork, run `cp_inflate(in, 1, out, 4096)` on the 1-byte stream `[0x03]`
/// (BFINAL=1, BTYPE=01, truncated ⇒ abort at `lib.c:115`) and return the child's
/// raw stderr.
fn raw_assert_message(f: common::InflateFn, tag: &str) -> String {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(dir).join(format!("cdiff-raw-{}-{tag}", std::process::id()));
    let fd = std::fs::File::create(&path).unwrap().into_raw_fd();

    let inb = GuardedBuf::new(4096);
    let outb = GuardedBuf::new(4096);
    inb.fill(0);
    inb.write_at(0, &[0x03]);

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0);
        if pid == 0 {
            libc::dup2(fd, 2);
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
            let nc = libc::rlimit {
                rlim_cur: 1,
                rlim_max: 1,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &nc);
            f(
                inb.ptr() as *mut c_void,
                1,
                outb.ptr() as *mut c_void,
                4096,
            );
            libc::_exit(0);
        }
        let mut st = 0;
        libc::waitpid(pid, &mut st, 0);
        assert!(libc::WIFSIGNALED(st) && libc::WTERMSIG(st) == libc::SIGABRT);
        libc::close(fd);
    }
    let s = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    s
}

#[test]
fn assert_message_prefix_matches() {
    let (c, r) = libs();
    let a = raw_assert_message(c.cp_inflate, "c");
    let b = raw_assert_message(r.cp_inflate, "rust");
    eprintln!("C    raw = {a:?}");
    eprintln!("Rust raw = {b:?}");

    // 1. the `<progname>: ` prefix must be identical (glibc uses __progname,
    //    src/cassert.rs uses basename(argv[0]))
    let pa = a.split(": ").next().unwrap();
    let pb = b.split(": ").next().unwrap();
    assert_eq!(pa, pb, "program-name prefix differs");
    assert!(!pa.is_empty(), "empty program name");

    // 2. everything from `lib.c` onwards must be identical; only the __FILE__
    //    path differs (CMake compiles with an absolute path).
    let ta = &a[a.rfind("lib.c:").unwrap()..];
    let tb = &b[b.rfind("lib.c:").unwrap()..];
    assert_eq!(ta, tb);
    assert_eq!(
        ta,
        "lib.c:115: cp_consume_bits: Assertion `s->count >= num_bits_to_read' failed.\n"
    );

    // 3. the only difference is the __FILE__ path
    let fa = &a[pa.len() + 2..a.rfind("lib.c:").unwrap()];
    let fb = &b[pb.len() + 2..b.rfind("lib.c:").unwrap()];
    eprintln!("C    __FILE__ prefix = {fa:?}");
    eprintln!("Rust __FILE__ prefix = {fb:?}");
    assert!(
        fa.ends_with("c_src/src/") || fa.is_empty(),
        "unexpected C __FILE__ prefix {fa:?}"
    );
    assert_eq!(fb, "c_src/src/");
}
