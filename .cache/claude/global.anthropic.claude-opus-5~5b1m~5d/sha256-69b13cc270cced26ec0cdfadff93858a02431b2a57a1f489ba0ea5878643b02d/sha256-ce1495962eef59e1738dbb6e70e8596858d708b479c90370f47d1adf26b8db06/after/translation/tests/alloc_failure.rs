// Phase C, rows E7 / E15 / E16 — the allocation-failure paths.
//
//   E7  create_buffer: `malloc` fails  -> NULL, no strcpy
//   E15 charinbuf mode 2: buffer NULL  -> "Failed to allocate buffer", -1
//   E16 charinbuf mode 4: buffer NULL  -> no further output, result 0
//
// These branches are unreachable while the heap is healthy, so each test
// re-executes this very test binary as a child process, clamps RLIMIT_AS (and,
// for the two `charinbuf` rows, additionally drains the malloc arena) and then
// drives BOTH shared objects through the failing path. All comparisons happen
// inside the child; the verdict is reported through its exit code.

mod support;

use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::process::Command;

const RLIMIT_AS: c_int = 9; // Linux
const EXIT_AGREE_OOM: i32 = 0; // both libraries took the failure path, identically
const EXIT_AGREE_NO_OOM: i32 = 10; // allocation unexpectedly succeeded in both
const EXIT_DIVERGE: i32 = 20; // C and Rust disagreed

#[repr(C)]
#[derive(Clone, Copy)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

extern "C" {
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn dup(fd: c_int) -> c_int;
    fn dup2(a: c_int, b: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

fn vm_size_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("read /proc/self/status");
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmSize:") {
            let kb: u64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .expect("parse VmSize");
            return kb * 1024;
        }
    }
    panic!("VmSize not found");
}

/// Chain of malloc'd blocks; the `next` pointer lives inside each block so that
/// draining the heap needs no Rust allocation.
struct Chain(*mut c_void);

impl Chain {
    /// Allocates until every request size in `ladder` is refused. Performs no
    /// Rust-side allocation, so it is safe to call under a clamped RLIMIT_AS.
    fn drain(ladder: &[usize]) -> Chain {
        let mut head: *mut c_void = std::ptr::null_mut();
        unsafe {
            for &size in ladder {
                loop {
                    let p = malloc(size.max(8));
                    if p.is_null() {
                        break;
                    }
                    *(p as *mut *mut c_void) = head;
                    head = p;
                }
            }
        }
        Chain(head)
    }

    fn release(self) {
        let mut p = self.0;
        unsafe {
            while !p.is_null() {
                let next = *(p as *mut *mut c_void);
                free(p);
                p = next;
            }
        }
    }
}

fn set_as_limit(bytes: u64) -> RLimit {
    let mut old = RLimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        assert_eq!(getrlimit(RLIMIT_AS, &mut old), 0, "getrlimit failed");
        let new = RLimit {
            rlim_cur: bytes.min(old.rlim_max),
            rlim_max: old.rlim_max,
        };
        assert_eq!(setrlimit(RLIMIT_AS, &new), 0, "setrlimit failed");
    }
    old
}

fn restore_as_limit(old: RLimit) {
    unsafe {
        assert_eq!(setrlimit(RLIMIT_AS, &old), 0, "restoring setrlimit failed");
    }
}

// ---------------------------------------------------------------------------
// Child side
// ---------------------------------------------------------------------------

fn child_create_buffer() -> ! {
    let (c, r) = support::both();
    // Warm up stdio/lazy state while the heap is still healthy.
    let _ = support::capture(|| unsafe { (c.charinbuf)(1, 0, 0, 0) });
    let _ = support::capture(|| unsafe { (r.charinbuf)(1, 0, 0, 0) });

    // A 64 MiB NUL-terminated source string, allocated *before* the clamp.
    const BIG: usize = 64 * 1024 * 1024;
    let mut big = vec![b'q'; BIG];
    big.push(0);

    let old = set_as_limit(vm_size_bytes() + 2 * 1024 * 1024);
    let (pc, pr) = unsafe {
        let pc = (c.create_buffer)(big.as_ptr() as *const c_char);
        let pr = (r.create_buffer)(big.as_ptr() as *const c_char);
        (pc, pr)
    };
    restore_as_limit(old);

    let verdict = match (pc.is_null(), pr.is_null()) {
        (true, true) => EXIT_AGREE_OOM,
        (false, false) => EXIT_AGREE_NO_OOM,
        _ => EXIT_DIVERGE,
    };
    if !pc.is_null() {
        // If it did succeed the copy must still be correct.
        unsafe {
            let n = std::ffi::CStr::from_ptr(pc).to_bytes().len();
            eprintln!("child: C create_buffer succeeded, len {n}");
            free(pc as *mut c_void);
        }
    }
    if !pr.is_null() {
        unsafe {
            let n = std::ffi::CStr::from_ptr(pr).to_bytes().len();
            eprintln!("child: Rust create_buffer succeeded, len {n}");
            free(pr as *mut c_void);
        }
    }
    eprintln!(
        "child(create_buffer): C null={} Rust null={}",
        pc.is_null(),
        pr.is_null()
    );
    std::process::exit(verdict);
}

fn child_charinbuf_oom(mode: c_int) -> ! {
    let (c, r) = support::both();
    // Warm up: allocate stdout's buffer, resolve everything, so that the
    // failing region below performs no allocation of its own.
    let _ = support::capture(|| unsafe { (c.charinbuf)(mode, 1, 2, 3) });
    let _ = support::capture(|| unsafe { (r.charinbuf)(mode, 1, 2, 3) });

    let dir = std::env::temp_dir();
    let path_c = dir.join(format!("charinbuf_oom_c_{}.txt", std::process::id()));
    let path_r = dir.join(format!("charinbuf_oom_r_{}.txt", std::process::id()));
    let file_c = std::fs::File::create(&path_c).expect("create c capture file");
    let file_r = std::fs::File::create(&path_r).expect("create r capture file");
    let fd_c = file_c.as_raw_fd();
    let fd_r = file_r.as_raw_fd();

    // Request ladder, built *before* the clamp so no Rust allocation happens
    // while the heap is exhausted. 24 = strlen("Testing malloc and free")+1,
    // 38 = strlen("Search for character X in this buffer")+1.
    let ladder: Vec<usize> = {
        let base = [1usize << 20, 1 << 16, 4096, 512, 64, 40, 38, 32, 24, 16, 8];
        // Two sweeps: glibc can leave a small remainder chunk behind after a
        // larger request has failed.
        base.iter().chain(base.iter()).copied().collect()
    };

    let old = set_as_limit(vm_size_bytes() + 512 * 1024);
    let chain = Chain::drain(&ladder);
    // Confirm the arena really is empty for the size the library will request.
    let probe = unsafe { malloc(if mode == 2 { 24 } else { 38 }) };
    let arena_empty = probe.is_null();
    if !probe.is_null() {
        unsafe { free(probe) };
    }

    let (rc, rr) = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        dup2(fd_c, 1);
        let rc = (c.charinbuf)(mode, 1, 2, 3);
        fflush(std::ptr::null_mut());
        dup2(fd_r, 1);
        let rr = (r.charinbuf)(mode, 1, 2, 3);
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        (rc, rr)
    };

    chain.release();
    restore_as_limit(old);

    let out_c = std::fs::read(&path_c).unwrap_or_default();
    let out_r = std::fs::read(&path_r).unwrap_or_default();
    let _ = std::fs::remove_file(&path_c);
    let _ = std::fs::remove_file(&path_r);

    eprintln!(
        "child(charinbuf mode {mode}): arena_empty={arena_empty} C rc={rc} Rust rc={rr}\n  C   out: {:?}\n  Rust out: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r)
    );

    if rc != rr || out_c != out_r {
        std::process::exit(EXIT_DIVERGE);
    }
    // The failure path is identified by the C result: -1 for mode 2, 0 for
    // mode 4 (`result` keeps its initialiser because the `if (buffer)` body is
    // skipped entirely).
    let took_failure_path = match mode {
        2 => rc == -1,
        4 => out_c == b"Mode 4: Using memchr to find character\n" && rc == 0,
        _ => unreachable!(),
    };
    std::process::exit(if took_failure_path {
        EXIT_AGREE_OOM
    } else {
        EXIT_AGREE_NO_OOM
    });
}

// ---------------------------------------------------------------------------
// Parent side
// ---------------------------------------------------------------------------

fn run_child(scenario: &str, test_name: &str) -> i32 {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
        .env("CHARINBUF_OOM_CHILD", scenario)
        .output()
        .expect("spawn child");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let code = out.status.code().unwrap_or(-1);
    assert_ne!(
        code, EXIT_DIVERGE,
        "C and Rust diverged on the allocation-failure path ({scenario}):\n{stderr}"
    );
    assert!(
        code == EXIT_AGREE_OOM || code == EXIT_AGREE_NO_OOM,
        "child for {scenario} exited with {code} (expected {EXIT_AGREE_OOM} or {EXIT_AGREE_NO_OOM}):\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&out.stdout)
    );
    eprintln!("[{scenario}] child verdict {code}\n{stderr}");
    code
}

fn scenario_env() -> Option<String> {
    std::env::var("CHARINBUF_OOM_CHILD").ok()
}

#[test]
fn create_buffer_oom() {
    if scenario_env().as_deref() == Some("create_buffer") {
        child_create_buffer();
    }
    let code = run_child("create_buffer", "create_buffer_oom");
    assert_eq!(
        code, EXIT_AGREE_OOM,
        "the 64 MiB allocation was expected to fail under the clamped RLIMIT_AS"
    );
}

#[test]
fn charinbuf_mode2_oom() {
    if scenario_env().as_deref() == Some("mode2") {
        child_charinbuf_oom(2);
    }
    let code = run_child("mode2", "charinbuf_mode2_oom");
    assert_eq!(
        code, EXIT_AGREE_OOM,
        "expected mode 2 to report the allocation failure (C returns -1)"
    );
}

#[test]
fn charinbuf_mode4_oom() {
    if scenario_env().as_deref() == Some("mode4") {
        child_charinbuf_oom(4);
    }
    let code = run_child("mode4", "charinbuf_mode4_oom");
    assert_eq!(
        code, EXIT_AGREE_OOM,
        "expected mode 4 to skip the body and return 0 (C's `result` initialiser)"
    );
}
