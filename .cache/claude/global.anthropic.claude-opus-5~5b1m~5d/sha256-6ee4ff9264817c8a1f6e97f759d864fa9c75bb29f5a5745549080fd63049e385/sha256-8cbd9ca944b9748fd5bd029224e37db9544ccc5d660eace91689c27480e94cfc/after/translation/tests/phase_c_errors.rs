//! Phase C — error-path differential tests.
//! One test per row of `ERRORS.md` (E1..E14), plus the generic FFI boundary
//! cases (null pointers, zero/oversized lengths, out-of-range "enum"/int
//! values one step past the valid range).

mod harness;
use harness::*;

use std::ffi::c_char;

// ---------------------------------------------------------------------------
// E1 — strrchr -> NULL: extractFilename must return `path`, never NULL.
// ---------------------------------------------------------------------------
#[test]
fn e1_separator_absent_returns_path() {
    let p = pair();
    let mut rng = Rng::seeded(0xE1);
    for i in 0..300 {
        let len = rng.range(1, 40);
        let v: Vec<u8> = (0..len).map(|_| rng.ascii_except(SEP)).collect();
        let b = CBuf::new(&v);
        let rc = unsafe { (p.c.extract)(b.ptr(), SEP as i8) };
        let rr = unsafe { (p.rs.extract)(b.ptr(), SEP as i8) };
        assert_eq!(rc, b.ptr(), "E1[{i}]: C did not return path");
        assert_eq!(rr, b.ptr(), "E1[{i}]: Rust did not return path");
        assert!(!rr.is_null(), "E1[{i}]: Rust returned NULL");
    }
}

// ---------------------------------------------------------------------------
// E2 — empty path, separator != 0.
// ---------------------------------------------------------------------------
#[test]
fn e2_empty_path_returns_path() {
    let p = pair();
    let b = CBuf::new(b"");
    for sep in 1u16..=255 {
        let s = sep as u8 as i8 as c_char;
        let rc = unsafe { (p.c.extract)(b.ptr(), s) };
        let rr = unsafe { (p.rs.extract)(b.ptr(), s) };
        assert_eq!(rc, b.ptr(), "E2: C, sep={sep:#04x}");
        assert_eq!(rr, b.ptr(), "E2: Rust, sep={sep:#04x}");
    }
}

// ---------------------------------------------------------------------------
// E3 — separator == '\0': strrchr matches the terminator, so the result is
// path + strlen(path) + 1 (one past the NUL).
// ---------------------------------------------------------------------------
#[test]
fn e3_nul_separator_returns_one_past_end() {
    let p = pair();
    let mut rng = Rng::seeded(0xE3);
    for i in 0..300 {
        let len = rng.range(0, 40);
        let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let b = CBuf::new(&v);
        let rc = unsafe { (p.c.extract)(b.ptr(), 0) } as usize - b.ptr() as usize;
        let rr = unsafe { (p.rs.extract)(b.ptr(), 0) } as usize - b.ptr() as usize;
        assert_eq!(rc, rr, "E3[{i}]: mismatch");
        assert_eq!(rc, len + 1, "E3[{i}]: C did not return path+len+1");
    }
}

// ---------------------------------------------------------------------------
// E4 — extractFilename(NULL, sep) must crash identically in both.
// ---------------------------------------------------------------------------
#[test]
fn e4_null_path_crashes() {
    let o = diff_fork("E4", |im| {
        let r = unsafe { (im.extract)(std::ptr::null(), SEP as i8) };
        // If it somehow returns, encode that distinctly.
        unsafe { _exit(if r.is_null() { 41 } else { 42 }) };
    });
    assert_eq!(
        o,
        Outcome::Signalled(11),
        "E4: expected SIGSEGV from both implementations, got {o:?}"
    );
}

// ---------------------------------------------------------------------------
// E5 — calloc failure -> fprintf(stderr, ...) + exit(30).
// ---------------------------------------------------------------------------
#[test]
fn e5_alloc_failure_exits_30() {
    let o = diff_fork("E5", |im| {
        let path = CBuf::new(b"/dir/file.txt");
        let dir = CBuf::new(b"out");
        let r = unsafe { (im.create)(path.ptr(), dir.ptr(), usize::MAX / 2) };
        // must not get here
        unsafe { _exit(if r.is_null() { 51 } else { 52 }) };
    });
    assert_eq!(o, Outcome::Exited(30), "E5: expected exit(30), got {o:?}");
}

// ---------------------------------------------------------------------------
// E11 — a different oversized suffixLen (one step below the wrap point) must
// also produce exit(30) identically.
// ---------------------------------------------------------------------------
#[test]
fn e11_near_max_suffixlen_exits_30() {
    // path="/a/b/c.bin" -> filenameLen = 5 ("c.bin"), outDirLen = 6 ("outdir")
    // base = 6 + 1 + 5 + 1 = 13, so size = 13 + suffixLen (mod 2^64).
    let base = 6usize + 1 + 5 + 1;
    let run = |n: usize| -> Outcome {
        diff_fork(&format!("E11 n={n:#x}"), move |im| {
            let path = CBuf::new(b"/a/b/c.bin");
            let dir = CBuf::new(b"outdir");
            let r = unsafe { (im.create)(path.ptr(), dir.ptr(), n) };
            unsafe { _exit(if r.is_null() { 51 } else { 52 }) };
        })
    };

    // (a) values whose wrapped size is still astronomically large -> exit(30)
    for &n in &[
        usize::MAX - 100,
        usize::MAX / 2 + 1,
        1usize << 62,
        (1usize << 63) + 12345,
        usize::MAX - base, // size == SIZE_MAX exactly
    ] {
        assert!(
            base.wrapping_add(n) > (1usize << 47),
            "E11(a): case {n:#x} does not actually request a huge size"
        );
        let o = run(n);
        assert_eq!(
            o,
            Outcome::Exited(30),
            "E11(a): suffixLen={n:#x} expected exit(30), got {o:?}"
        );
    }

    // (b) values one step past the wrap point: the size computation wraps to a
    // tiny number, `calloc` SUCCEEDS, and the C then writes past the end of the
    // allocation. This is the C's behaviour and the Rust must match it exactly,
    // including the fact that it does NOT exit(30). `diff_fork` asserts the two
    // implementations terminate identically.
    for &n in &[
        usize::MAX - 8,               // size = 4
        usize::MAX - base + 1,        // size = 0
        usize::MAX,                   // size = base - 1 = 12
        usize::MAX - 1,
        usize::MAX - base + 2,        // size = 1
    ] {
        let o = run(n);
        assert_eq!(
            o,
            Outcome::Exited(52),
            "E11(b): suffixLen={n:#x} (wrapped size {}) expected the allocation to \
             succeed and a non-NULL return from BOTH, got {o:?}",
            base.wrapping_add(n)
        );
    }
}

// ---------------------------------------------------------------------------
// E6 — suffixLen == SIZE_MAX makes the size computation wrap.
//
//   size = outDirLen + 1 + filenameLen + SIZE_MAX + 1
//        = outDirLen + filenameLen + 1                (mod 2^64)
//
// which is exactly the number of bytes the false arm writes, so the call is
// well-defined w.r.t. the allocation and can be compared in-process.
// ---------------------------------------------------------------------------
#[test]
fn e6_suffixlen_size_overflow_wraps() {
    let p = pair();
    let cases: &[(&[u8], &[u8])] = &[
        (b"/x/y", b"dir"),   // false arm, size = 3 + 1 + 1 = 5
        (b"/x/y", b"dir/"),  // true  arm, size = 4 + 1 + 1 = 6
        (b"file", b"o"),
        (b"", b"o/"),
        (b"a/b/c/dddd", b"some/where"),
    ];
    for (i, (path, dir)) in cases.iter().enumerate() {
        let bp = CBuf::new(path);
        let bd = CBuf::new(dir);
        let flen = ref_filename_len(path, SEP);
        let n = ref_alloc_size(dir.len(), flen, usize::MAX);
        assert_eq!(n, dir.len() + flen + 1, "E6[{i}]: wrap math");
        let oc = unsafe { (p.c.create)(bp.ptr(), bd.ptr(), usize::MAX) };
        let or = unsafe { (p.rs.create)(bp.ptr(), bd.ptr(), usize::MAX) };
        assert!(!oc.is_null(), "E6[{i}]: C returned NULL");
        assert!(!or.is_null(), "E6[{i}]: Rust returned NULL");
        let sc = unsafe { std::slice::from_raw_parts(oc as *const u8, n) }.to_vec();
        let sr = unsafe { std::slice::from_raw_parts(or as *const u8, n) }.to_vec();
        unsafe {
            free(oc as *mut _);
            free(or as *mut _);
        }
        assert_eq!(sc, sr, "E6[{i}]: wrapped-size buffer mismatch");
    }

    // A few more wrap points, checked for identical termination behaviour.
    for &n in &[usize::MAX, usize::MAX - 1, usize::MAX - 2] {
        let o = diff_fork(&format!("E6-fork n={n:#x}"), move |im| {
            let bp = CBuf::new(b"/x/y");
            let bd = CBuf::new(b"dir");
            let r = unsafe { (im.create)(bp.ptr(), bd.ptr(), n) };
            unsafe { _exit(if r.is_null() { 61 } else { 60 }) };
        });
        assert_eq!(o, Outcome::Exited(60), "E6-fork n={n:#x}: got {o:?}");
    }
}

// ---------------------------------------------------------------------------
// E7 — empty outDirName: outDirName[strlen(outDirName)-1] == outDirName[-1].
// Both branches, with the preceding byte under our control.
// ---------------------------------------------------------------------------
#[test]
fn e7_empty_outdir_reads_byte_before_buffer() {
    let mut rng = Rng::seeded(0xE7);
    for i in 0..200 {
        let plen = rng.range(0, 24);
        let mut path: Vec<u8> = (0..plen).map(|_| rng.ascii_except(SEP)).collect();
        for _ in 0..rng.below(3) {
            if !path.is_empty() {
                let j = rng.below(path.len());
                path[j] = SEP;
            }
        }
        let n = rng.range(0, 8);
        // preceding byte == separator -> true arm (no separator inserted)
        let out_t = diff_create_raw_outdir(&path, &[SEP, 0], 1, n, &format!("E7-true[{i}]"));
        // preceding byte != separator -> false arm (separator inserted at [0])
        let out_f = diff_create_raw_outdir(&path, &[b'Q', 0], 1, n, &format!("E7-false[{i}]"));

        let fname = &path[ref_filename_offset(&path, SEP)..];
        assert_eq!(&out_t[..fname.len()], fname, "E7[{i}]: true arm layout");
        assert_eq!(out_f[0], SEP, "E7[{i}]: false arm must insert '/'");
        assert_eq!(&out_f[1..1 + fname.len()], fname, "E7[{i}]: false arm layout");
    }
}

// ---------------------------------------------------------------------------
// E8/E9/E10 — null pointer arguments to FIO_createFilename_fromOutDir.
// ---------------------------------------------------------------------------
#[test]
fn e8_null_path_crashes() {
    let o = diff_fork("E8", |im| {
        let dir = CBuf::new(b"out");
        let r = unsafe { (im.create)(std::ptr::null(), dir.ptr(), 0) };
        unsafe { _exit(if r.is_null() { 81 } else { 82 }) };
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "E8: expected a fatal signal from both, got {o:?}"
    );
}

#[test]
fn e9_null_outdir_crashes() {
    let o = diff_fork("E9", |im| {
        let path = CBuf::new(b"/a/b.txt");
        let r = unsafe { (im.create)(path.ptr(), std::ptr::null(), 0) };
        unsafe { _exit(if r.is_null() { 91 } else { 92 }) };
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "E9: expected a fatal signal from both, got {o:?}"
    );
}

#[test]
fn e10_both_null_crash() {
    let o = diff_fork("E10", |im| {
        let r = unsafe { (im.create)(std::ptr::null(), std::ptr::null(), 0) };
        unsafe { _exit(if r.is_null() { 101 } else { 102 }) };
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "E10: expected a fatal signal from both, got {o:?}"
    );
}

// ---------------------------------------------------------------------------
// E12 — out-of-range "enum"/int values across the FFI boundary.
//
// `char separator` is passed in a full register; a C caller may hand over any
// `int`. Every value 0..=255, the high-bit (negative signed char) values, and
// values *past* the valid char range (0x100, 0x1FF, -1, INT_MIN, INT_MAX) must
// behave identically in both implementations.
// ---------------------------------------------------------------------------
#[test]
fn e12_out_of_range_separator_values() {
    // (a) every in-range byte value, on a path containing all byte values
    let all: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    for sep in 0u16..=255 {
        diff_extract(&all, sep as u8, &format!("E12a sep={sep:#04x}"));
    }

    // (b) values one step past the char range, passed as a full c_int
    let oversized: &[i32] = &[
        0x100, 0x101, 0x17f, 0x1ff, 0xdead, -1, -2, -128, -129, -256, -257,
        i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1,
    ];
    for &sep in oversized {
        diff_extract_int(&all, sep, &format!("E12b sep={sep:#x}"));
        diff_extract_int(b"", sep, &format!("E12b-empty sep={sep:#x}"));
        diff_extract_int(b"/usr/local/bin/zstd", sep, &format!("E12b-path sep={sep:#x}"));
    }

    // (c) the same over randomized paths
    let mut rng = Rng::seeded(0xE12);
    for i in 0..400 {
        let len = rng.range(0, 32);
        let v: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let sep = rng.next_u64() as i32;
        diff_extract_int(&v, sep, &format!("E12c[{i}] sep={sep:#x}"));
    }
}

// ---------------------------------------------------------------------------
// E13 — path consisting solely of separators -> empty filename.
// ---------------------------------------------------------------------------
#[test]
fn e13_path_all_separators() {
    for k in 1..=8usize {
        let p = vec![SEP; k];
        for &n in &[0usize, 1, 7] {
            let a = diff_create(&p, b"out", n, &format!("E13a k={k} n={n}"));
            let b = diff_create(&p, b"out/", n, &format!("E13b k={k} n={n}"));
            assert_eq!(&a[..4], b"out/", "E13: false arm prefix");
            assert!(a[4..].iter().all(|&x| x == 0), "E13: padding not zero");
            assert_eq!(&b[..4], b"out/", "E13: true arm prefix");
            assert!(b[4..].iter().all(|&x| x == 0), "E13: padding not zero");
        }
    }
}

// ---------------------------------------------------------------------------
// E14 — empty path.
// ---------------------------------------------------------------------------
#[test]
fn e14_empty_path() {
    for dir in [
        &b"out"[..],
        &b"out/"[..],
        &b"/"[..],
        &b"a/b/c"[..],
        &b"a/b/c/"[..],
    ] {
        for &n in &[0usize, 1, 32] {
            let out = diff_create(b"", dir, n, &format!("E14 dir={dir:?} n={n}"));
            let ins = if dir.last() == Some(&SEP) { 0 } else { 1 };
            assert_eq!(&out[..dir.len()], dir);
            if ins == 1 {
                assert_eq!(out[dir.len()], SEP);
            }
            assert!(out[dir.len() + ins..].iter().all(|&b| b == 0));
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases: zero lengths, empty everything.
// ---------------------------------------------------------------------------
#[test]
fn generic_zero_and_empty_boundaries() {
    // empty path AND non-empty dir, suffixLen 0
    diff_create(b"", b"d", 0, "gen1");
    // dir of length 1 that IS the separator
    diff_create(b"", b"/", 0, "gen2");
    diff_create(b"x", b"/", 0, "gen3");
    // single-character everything
    diff_create(b"a", b"b", 0, "gen4");
    diff_create(b"/", b"/", 0, "gen5");
    // extractFilename on a 1-byte path for every separator value
    for sep in 0u16..=255 {
        diff_extract(b"a", sep as u8, &format!("gen6 sep={sep:#04x}"));
        diff_extract(b"", sep as u8, &format!("gen7 sep={sep:#04x}"));
        diff_extract(b"/", sep as u8, &format!("gen8 sep={sep:#04x}"));
    }
}

// ---------------------------------------------------------------------------
// E5b — the diagnostic written to stderr on the calloc-failure path must be
// byte-for-byte identical. The child's stderr is redirected into a pipe.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn dup2(old: i32, new: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, n: usize) -> isize;
}

fn stderr_of_alloc_failure(im: &'static harness::Impl, suffix_len: usize) -> (Outcome, Vec<u8>) {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (r, w) = (fds[0], fds[1]);
    let status = fork_status(|| {
        unsafe {
            close(r);
            dup2(w, 2); // stderr -> pipe
            close(w);
        }
        let path = CBuf::new(b"/dir/file.txt");
        let dir = CBuf::new(b"out");
        let res = unsafe { (im.create)(path.ptr(), dir.ptr(), suffix_len) };
        unsafe { _exit(if res.is_null() { 51 } else { 52 }) };
    });
    unsafe { close(w) };
    let mut out = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let n = unsafe { read(r, buf.as_mut_ptr(), buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { close(r) };
    (classify(status), out)
}

#[test]
fn e5b_alloc_failure_stderr_is_byte_identical() {
    let p = pair();
    for &n in &[usize::MAX / 2, 1usize << 62, usize::MAX - 100] {
        let (oc, sc) = stderr_of_alloc_failure(&p.c, n);
        let (or, sr) = stderr_of_alloc_failure(&p.rs, n);
        assert_eq!(oc, Outcome::Exited(30), "E5b: C exit status, n={n:#x}");
        assert_eq!(or, Outcome::Exited(30), "E5b: Rust exit status, n={n:#x}");
        assert_eq!(
            sc,
            sr,
            "E5b: stderr differs for n={n:#x}\n C    = {:?}\n Rust = {:?}",
            String::from_utf8_lossy(&sc),
            String::from_utf8_lossy(&sr)
        );
        assert!(
            sc.starts_with(b"zstd: FIO_createFilename_fromOutDir: "),
            "E5b: unexpected C message {:?}",
            String::from_utf8_lossy(&sc)
        );
    }
}
