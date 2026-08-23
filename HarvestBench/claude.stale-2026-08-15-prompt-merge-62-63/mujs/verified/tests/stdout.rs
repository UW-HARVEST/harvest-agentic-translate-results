//! Phase B — the exported entry points whose entire observable output goes to
//! **stdout/stderr** via `printf`/`fputs` rather than through a return value:
//!
//!   `js_gc(J, report)`   — jsgc.c, prints GC statistics for any non-zero report
//!   `js_trap(J, pc)`     — jsrun.c, dumps the value stack + environment
//!   `jsS_dumpstrings(J)` — jsintern.c, dumps the string-intern tree
//!   the default report handler — jsstate.c, `fputs(message, stderr)`
//!   `debugger;`          — the OP_DEBUGGER opcode, which calls `js_trap`
//!
//! Nothing else in the suite compares these, so the file redirects the real
//! file descriptors 1 and 2 to a temp file around each call and diffs the bytes.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_int;

/// Redirect fd 1 and fd 2 into a temp file for the duration of `f`, and return
/// everything that was written.
fn capture_fds<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        // Flush Rust's own buffers first so they do not land in the capture.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        let mut path = std::env::temp_dir();
        path.push(format!(
            "mujs_capture_{}_{:?}.txt",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("temp file");
        let fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        let saved_out = libc::dup(1);
        let saved_err = libc::dup(2);
        assert!(saved_out >= 0 && saved_err >= 0);
        libc::dup2(fd, 1);
        libc::dup2(fd, 2);

        let r = f();

        // The C library writes through stdio; make it flush before we look.
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_out, 1);
        libc::dup2(saved_err, 2);
        libc::close(saved_out);
        libc::close(saved_err);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_to_end(&mut buf).unwrap();
        let _ = std::fs::remove_file(&path);
        (r, buf)
    }
}

/// `js_trap` prints raw heap pointers (`[Object 0x7f...]`, `[Function 0x7f...,
/// name, file:line]`) via `%p`. Those can never agree between two independently
/// allocated libraries, so mask every `0x<hex>` run to `0xADDR` — the LENGTH of
/// the masked token is normalised too, so a divergence in anything else
/// (including the presence/absence of an address) is still caught.
fn mask_addresses(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'0' && i + 1 < b.len() && (b[i + 1] == b'x' || b[i + 1] == b'X') {
            let mut j = i + 2;
            while j < b.len() && (b[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                out.extend_from_slice(b"0xADDR");
                i = j;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Run `f` against the C library and then the Rust library, capturing fd 1+2
/// each time, and assert the captured bytes are identical modulo heap addresses.
#[track_caller]
fn diff_capture(label: &str, mut f: impl FnMut(&Api)) {
    let (c, r) = both(|api, _| capture_fds(|| f(api)).1);
    let (cm, rm) = (mask_addresses(&c), mask_addresses(&r));
    assert_eq!(
        String::from_utf8_lossy(&cm),
        String::from_utf8_lossy(&rm),
        "DIVERGENCE in stdout/stderr for {} (addresses masked)",
        label
    );
    // Sanity: the number of masked addresses must also agree.
    let count = |s: &[u8]| s.windows(6).filter(|w| *w == b"0xADDR").count();
    assert_eq!(
        count(&cm),
        count(&rm),
        "DIVERGENCE in the NUMBER of printed addresses for {}",
        label
    );
}

// ------------------------------------------------------------------ js_gc
#[test]
fn js_gc_report_output_matches() {
    for report in [0, 1, 2, -1, i32::MAX, i32::MIN] {
        diff_capture(&format!("js_gc(report={})", report), |api| unsafe {
            let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            // create some garbage first so the numbers are non-trivial
            (api.js_dostring)(
                J,
                cs("var a=[]; for (var i=0;i<200;++i) a.push({x:i, s:'v'+i}); a=null;").as_ptr(),
            );
            (api.js_gc)(J, report);
            (api.js_gc)(J, report);
            (api.js_freestate)(J);
        });
    }
}

// ------------------------------------------------------------------ js_trap / debugger
#[test]
fn js_trap_and_debugger_output_matches() {
    let scripts = [
        "debugger;",
        "var x=1; debugger; print(x);",
        "function f(a,b){ debugger; return a+b } print(f(1,2));",
        "(function(){ var y='s'; debugger })();",
        "with({q:1}) { debugger }",
        "try { debugger } catch(e) { print('c') }",
        "for (var i=0;i<2;++i) { debugger }",
    ];
    for src in scripts {
        diff_capture(&format!("debugger/js_trap {:?}", src), |api| {
            let _ = run_script(api, 0, src);
        });
    }
}

#[test]
fn js_trap_direct_pc_values() {
    // js_trap(J, pc): pc < 0 means "no bytecode context"; any other value
    // indexes the current function's code. Called from a cfunction there is no
    // js_Function, so only the pc<0 shape is defined — but the C reads
    // `J->trace[J->tracetop]` unconditionally, so exercise several values.
    for pc in [-1, 0] {
        diff_capture(&format!("js_trap(pc={})", pc), |api| unsafe {
            let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
            (api.js_pushnumber)(J, 42.0);
            (api.js_pushstring)(J, cs("hello").as_ptr());
            (api.js_newobject)(J);
            (api.js_trap)(J, pc);
            (api.js_freestate)(J);
        });
    }
}

// ------------------------------------------------------------------ jsS_dumpstrings
#[test]
fn dumpstrings_output_matches() {
    let programs: &[&str] = &[
        "",
        "var a=1;",
        "var alpha=1, beta=2, gamma=3; print(alpha+beta+gamma);",
        "var o={}; for (var i=0;i<50;++i) o['key'+i]=i; print(Object.keys(o).length);",
        "print('a','bb','ccc','dddd');",
    ];
    for src in programs {
        diff_capture(&format!("jsS_dumpstrings {:?}", src), |api| unsafe {
            let J = new_state(api, 0);
            (api.js_dostring)(J, cs(src).as_ptr());
            // intern a fixed set so the tree shape is deterministic
            for w in ["zeta", "eta", "theta", "iota", "kappa", "a", "zz"] {
                (api.js_intern)(J, cs(w).as_ptr());
            }
            (api.jsS_dumpstrings)(J);
            (api.js_freestate)(J);
        });
    }
}

// ------------------------------------------------------------------ default report handler
#[test]
fn default_report_handler_writes_to_stderr() {
    // jsstate.c js_defaultreport: fputs(message, stderr); fputc('\n', stderr).
    // A state that has NOT had js_setreport called uses it.
    let scripts = ["null.x", "(", "throw 'plain string'", "throw 42", "1+1"];
    for src in scripts {
        let (c, r) = both(|api, _| {
            let (rc, bytes) = capture_fds(|| unsafe {
                let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
                let rc = (api.js_dostring)(J, cs(src).as_ptr());
                (api.js_freestate)(J);
                rc
            });
            (rc, bytes)
        });
        assert_eq!(
            c.0, r.0,
            "DIVERGENCE default-report rc for {:?}: C={} Rust={}",
            src, c.0, r.0
        );
        assert_eq!(
            String::from_utf8_lossy(&c.1),
            String::from_utf8_lossy(&r.1),
            "DIVERGENCE default report stderr for {:?}",
            src
        );
    }
}

// ------------------------------------------------------------------ js_buffer helpers
#[test]
fn js_buffer_putc_puts_putm_grow() {
    // jsintern.c js_putc/js_puts/js_putm grow a js_Buffer past its initial
    // capacity. The buffer is opaque, so drive it and then read the bytes back
    // through the header layout: `struct js_Buffer { int n, m; char s[]; }`.
    #[repr(C)]
    struct Buffer {
        n: c_int,
        m: c_int,
        // s[] follows
    }
    let (c, r) = both(|api, _| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let mut sb: *mut std::os::raw::c_void = std::ptr::null_mut();
        for i in 0..300 {
            (api.js_putc)(J, &mut sb, (b'a' + (i % 26) as u8) as c_int);
        }
        (api.js_puts)(J, &mut sb, cs("--a-longer-chunk-of-text--").as_ptr());
        let m = cs("0123456789abcdef");
        (api.js_putm)(J, &mut sb, m.as_ptr(), m.as_ptr().add(10));
        (api.js_putc)(J, &mut sb, 0);
        let hdr = sb as *const Buffer;
        let n = (*hdr).n;
        let cap = (*hdr).m;
        let data = (sb as *const u8).add(std::mem::size_of::<Buffer>());
        let bytes = std::slice::from_raw_parts(data, n as usize).to_vec();
        (api.js_free)(J, sb);
        (api.js_freestate)(J);
        (n, cap, bytes)
    });
    assert_eq!(
        c.0, r.0,
        "js_Buffer length diverged: C={} Rust={}",
        c.0, r.0
    );
    assert_eq!(
        c.1, r.1,
        "js_Buffer capacity diverged: C={} Rust={}",
        c.1, r.1
    );
    assert_eq!(
        String::from_utf8_lossy(&c.2),
        String::from_utf8_lossy(&r.2),
        "js_Buffer contents diverged"
    );
}
