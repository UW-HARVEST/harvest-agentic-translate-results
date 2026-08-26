//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library performs no validation at all (see `ERRORS.md` for the grep
//! evidence), so its "rejection" behaviour for a contract violation is a fault.
//! Rows E1/E2 therefore run the call in a forked child and compare the **exact**
//! termination status (signal number) of C vs Rust — not merely "both failed".

mod common;

use common::*;
use std::ffi::c_char;
use std::ptr;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Term {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child and report how the child terminated.
fn in_child<F: FnOnce()>(f: F) -> Term {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            f();
            libc::_exit(0);
        }
        let mut status: i32 = 0;
        let r = libc::waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Term::Signaled(libc::WTERMSIG(status))
        } else {
            Term::Exited(libc::WEXITSTATUS(status))
        }
    }
}

// ---------------------------------------------------------------- E1
#[test]
fn e1_null_pointer() {
    // Load both libraries *before* forking.
    let c = c_impl();
    let r = rust_impl();

    let tc = in_child(|| unsafe {
        let out = (c.tool_basename)(ptr::null_mut());
        // If it somehow returns, encode "returned NULL" / "returned non-NULL".
        libc::_exit(if out.is_null() { 10 } else { 11 });
    });
    let tr = in_child(|| unsafe {
        let out = (r.tool_basename)(ptr::null_mut());
        libc::_exit(if out.is_null() { 10 } else { 11 });
    });

    assert_eq!(tc, tr, "NULL path: C terminated {tc:?} but Rust terminated {tr:?}");
    assert_eq!(
        tc,
        Term::Signaled(libc::SIGSEGV),
        "expected the unchecked NULL dereference to raise SIGSEGV in C"
    );
}

// ---------------------------------------------------------------- E2
#[test]
fn e2_unterminated_buffer_guard_page() {
    let c = c_impl();
    let r = rust_impl();

    unsafe {
        let page = libc::sysconf(libc::_SC_PAGESIZE) as usize;
        let total = page * 2;
        let map = libc::mmap(
            ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(map, libc::MAP_FAILED, "mmap failed");
        let base = map as *mut u8;
        // First page: no NUL anywhere, so the scan must run off the end.
        ptr::write_bytes(base, b'a', page);
        // Second page: unmapped-equivalent guard.
        assert_eq!(
            libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE),
            0,
            "mprotect failed"
        );

        let tc = in_child(|| {
            let _ = (c.tool_basename)(base as *mut c_char);
            libc::_exit(12);
        });
        let tr = in_child(|| {
            let _ = (r.tool_basename)(base as *mut c_char);
            libc::_exit(12);
        });

        assert_eq!(
            tc, tr,
            "unterminated buffer: C terminated {tc:?} but Rust terminated {tr:?}"
        );
        assert_eq!(
            tc,
            Term::Signaled(libc::SIGSEGV),
            "expected the unbounded scan to fault on the guard page in C"
        );

        libc::munmap(map, total);
    }
}

// ---------------------------------------------------------------- E3
#[test]
fn e3_zero_length() {
    // "" -> both strrchr() calls return NULL -> `path` returned unchanged.
    let mut b_c = vec![0u8];
    let mut b_r = vec![0u8];
    let out_c = run_one(c_impl(), &mut b_c, 0);
    let out_r = run_one(rust_impl(), &mut b_r, 0);
    assert_eq!(out_c, out_r);
    assert_eq!(out_c.offset, 0, "empty string must return the same pointer");
    assert!(out_c.string.is_empty());
    diff(b"");
}

// ---------------------------------------------------------------- E4
#[test]
fn e4_separator_is_last_byte() {
    // The returned pointer is exactly one past the last character: it points at
    // the NUL terminator, i.e. the empty basename.
    let cases: &[&[u8]] = &[
        b"/", b"\\", b"a/", b"a\\", b"ab/", b"ab\\", b"/a/", b"\\a\\", b"dir/sub/", b"dir\\sub\\",
        b"mixed/dir\\", b"mixed\\dir/",
    ];
    for case in cases {
        diff(case);
        let mut buf = case.to_vec();
        buf.push(0);
        let out = run_one(c_impl(), &mut buf.clone(), 0);
        assert_eq!(
            out.offset,
            case.len() as isize,
            "expected pointer to the NUL terminator for {}",
            Esc(case)
        );
        assert!(out.string.is_empty(), "expected empty basename for {}", Esc(case));
        let out_r = run_one(rust_impl(), &mut buf, 0);
        assert_eq!(out, out_r);
    }
    // Longer: separator is the last byte for every length up to 128.
    let mut rng = Rng::new(SEED ^ 0xE4);
    for len in 0..=128usize {
        for sep in [b'/', b'\\'] {
            let mut s: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
            s.push(sep);
            diff(&s);
        }
    }
}

// ---------------------------------------------------------------- E5
#[test]
fn e5_only_separators() {
    let cases: &[&[u8]] = &[
        b"/", b"//", b"///", b"////", b"\\", b"\\\\", b"\\\\\\", b"/\\", b"\\/", b"/\\/\\", b"\\/\\/",
        b"//\\\\//",
    ];
    for case in cases {
        diff(case);
        let mut buf = case.to_vec();
        buf.push(0);
        let out = run_one(c_impl(), &mut buf, 0);
        assert_eq!(out.offset, case.len() as isize);
        assert!(out.string.is_empty());
    }
    let mut rng = Rng::new(SEED ^ 0xE5);
    for len in 1..=200usize {
        let s: Vec<u8> = (0..len).map(|_| rng.pick(&[b'/', b'\\'])).collect();
        diff(&s);
    }
}

// ---------------------------------------------------------------- E6
#[test]
fn e6_oversized_input() {
    let mut rng = Rng::new(SEED ^ 0xE6);
    for size in [1usize << 20, 4usize << 20] {
        // no separator
        let plain: Vec<u8> = (0..size)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != b'/' && b != b'\\' {
                    return b;
                }
            })
            .collect();
        diff(&plain);
        // separator at the very last position (worst case for "one past the end")
        let mut s = plain.clone();
        *s.last_mut().unwrap() = b'/';
        diff(&s);
        let mut s = plain.clone();
        *s.last_mut().unwrap() = b'\\';
        diff(&s);
        // separator at the very first position
        let mut s = plain.clone();
        s[0] = b'/';
        diff(&s);
    }
}

// ---------------------------------------------------------------- E7
#[test]
fn e7_high_bit_bytes() {
    // 0x80..=0xff are negative as `char` on x86-64; none of them is a separator.
    let mut s: Vec<u8> = (0x80u8..=0xff).collect();
    diff(&s);
    s.reverse();
    diff(&s);
    for b in 0x80u8..=0xff {
        diff(&[b]);
        diff(&[b, b]);
        diff(&[b'/', b]);
        diff(&[b, b'/']);
        diff(&[b'\\', b]);
        diff(&[b, b'\\']);
        diff(&[b, b'/', b, b'\\', b]);
    }
    // exhaustive over all non-NUL bytes, alone and around separators
    for b in 1u8..=0xff {
        diff(&[b]);
        diff(&[b, b'/', b'\\', b]);
    }
}

// ---------------------------------------------------------------- E8
#[test]
fn e8_separator_after_nul() {
    // Data beyond the terminator must never be examined.
    let buf: &[&[u8]] = &[
        b"ab\0/x\0",
        b"ab\0\\x\0",
        b"\0/\\/\\\0",
        b"a/b\0/deeper\0",
        b"a\\b\0\\deeper\0",
    ];
    for b in buf {
        diff_at(b, 0);
    }
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..2000 {
        let len = rng.below(40);
        let mut v: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'/',
                1 => b'\\',
                _ => rng.nonzero_byte(),
            })
            .collect();
        v.push(0);
        let tail = 1 + rng.below(24);
        for _ in 0..tail {
            v.push(rng.pick(&[b'/', b'\\', b'q']));
        }
        v.push(0);
        diff_at(&v, 0);
    }
}

// ---------------------------------------------------------------- E9
#[test]
fn e9_idempotent_on_result() {
    let cases: &[&[u8]] = &[
        b"", b"/", b"\\", b"a", b"a/b", b"a\\b", b"a/b\\c", b"a\\b/c", b"dir/", b"dir\\", b"//",
        b"\\\\", b"/\\", b"\\/",
    ];
    for case in cases {
        let mut v = case.to_vec();
        v.push(0);
        diff_twice(&v, 0);
    }
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..4000 {
        let len = rng.below(48);
        let mut v: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => b'/',
                1 => b'\\',
                _ => rng.nonzero_byte(),
            })
            .collect();
        v.push(0);
        diff_twice(&v, 0);
    }
}

// ---------------------------------------------------------------- E10
#[test]
fn e10_no_enum_in_abi() {
    // Mechanical confirmation that the "out-of-range enum value" class of input
    // does not exist for this ABI: the whole public header is one function whose
    // only parameter is a `char *`.
    let header = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/include/lib.h"))
        .expect("header must exist");
    assert!(!header.contains("enum"), "header gained an enum: {header}");
    assert!(!header.contains("int"), "header gained an int parameter: {header}");
    let decls: Vec<&str> = header.lines().filter(|l| l.contains(';')).collect();
    assert_eq!(decls, vec!["char *tool_basename(char *path);"], "public ABI changed");
}

// ------------------------------------------------- generic boundary sweep
#[test]
fn generic_boundaries() {
    // one step past every interesting length boundary, with and without
    // separators in the boundary position
    let mut rng = Rng::new(SEED ^ 0xB0);
    for n in [0usize, 1, 2, 3, 4, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257] {
        let plain: Vec<u8> = (0..n)
            .map(|_| loop {
                let b = rng.nonzero_byte();
                if b != b'/' && b != b'\\' {
                    return b;
                }
            })
            .collect();
        diff(&plain);
        for sep in [b'/', b'\\'] {
            for pos in [0usize, n / 2, n.saturating_sub(1)] {
                if n == 0 {
                    continue;
                }
                let mut s = plain.clone();
                s[pos] = sep;
                diff(&s);
            }
        }
    }
}
