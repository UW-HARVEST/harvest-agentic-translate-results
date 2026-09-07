//! Phase B — CONFIGS.md rows 1..5: the lowest-level exported helpers
//! (`os_calloc`, `os_realloc`, `os_strdup`, `merror`, `FreeAlertData`).
//!
//! Every call goes through `dlsym` on both `.so`s.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::{c_char, c_int, c_void};

/// CONFIGS.md row 1 — `os_calloc` across randomized `num` x `size`.
#[test]
fn row01_os_calloc() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE01);

    let mut cases: Vec<(usize, usize)> = vec![
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (1, 96),
        (96, 1),
        (2, 48),
        (1024, 1),
        (1, 1024),
        (7, 13),
    ];
    for _ in 0..200 {
        cases.push((1 + rng.below(64), 1 + rng.below(512)));
    }

    for (num, size) in cases {
        unsafe {
            let a = (p.c.os_calloc)(num, size);
            let b = (p.rs.os_calloc)(num, size);
            // calloc(0, n) may legally return NULL or a unique pointer; glibc
            // returns non-NULL, and both libs call the same glibc.
            assert_same!(
                format!("os_calloc({num},{size}) null-ness"),
                a.is_null(),
                b.is_null()
            );
            if !a.is_null() {
                let n = num * size;
                let av = std::slice::from_raw_parts(a as *const u8, n).to_vec();
                let bv = std::slice::from_raw_parts(b as *const u8, n).to_vec();
                assert_same!(
                    format!("os_calloc({num},{size}) zero-fill"),
                    av.clone(),
                    bv
                );
                assert!(av.iter().all(|&x| x == 0), "os_calloc must zero-fill");
            }
            free(a);
            free(b);
        }
    }
}

/// CONFIGS.md row 2 — `os_realloc`: NULL-grow then randomized shrink/grow
/// chains, verifying the retained prefix.
#[test]
fn row02_os_realloc() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE02);

    for _ in 0..150 {
        unsafe {
            let mut sizes: Vec<usize> = Vec::new();
            for _ in 0..1 + rng.below(6) {
                sizes.push(1 + rng.below(1024));
            }

            let mut a: *mut c_void = std::ptr::null_mut();
            let mut b: *mut c_void = std::ptr::null_mut();
            let mut written = 0usize;

            for &sz in &sizes {
                a = (p.c.os_realloc)(a, sz);
                b = (p.rs.os_realloc)(b, sz);
                assert_same!(
                    format!("os_realloc(_,{sz}) null-ness"),
                    a.is_null(),
                    b.is_null()
                );
                assert!(!a.is_null());

                let keep = written.min(sz);
                let av = std::slice::from_raw_parts(a as *const u8, keep).to_vec();
                let bv = std::slice::from_raw_parts(b as *const u8, keep).to_vec();
                assert_same!(format!("os_realloc(_,{sz}) retained prefix"), av, bv);

                // Refill with a deterministic pattern so the next round's
                // prefix check is meaningful.
                for i in 0..sz {
                    *(a as *mut u8).add(i) = (i % 251) as u8;
                    *(b as *mut u8).add(i) = (i % 251) as u8;
                }
                written = sz;
            }
            free(a);
            free(b);
        }
    }
}

/// CONFIGS.md row 3 — `os_strdup` over randomized byte strings.
#[test]
fn row03_os_strdup() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE03);

    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b" ".to_vec(),
        b"alerts.log".to_vec(),
        b"** Alert 1.2: mail - syscheck".to_vec(),
        vec![0xFFu8; 1024],
        vec![b'x'; 4096],
    ];
    for _ in 0..200 {
        cases.push(rng.raw_token_upto(300));
    }

    for bytes in cases {
        unsafe {
            let s = cbytes(&bytes);
            let a = (p.c.os_strdup)(s.as_ptr());
            let b = (p.rs.os_strdup)(s.as_ptr());
            assert!(!a.is_null() && !b.is_null());
            let av = std::ffi::CStr::from_ptr(a).to_bytes().to_vec();
            let bv = std::ffi::CStr::from_ptr(b).to_bytes().to_vec();
            assert_same!("os_strdup content", av.clone(), bv);
            assert_eq!(av, bytes, "os_strdup must copy verbatim");
            // Independent allocations, not aliases of the input.
            assert_ne!(a as *const c_char, s.as_ptr());
            assert_ne!(b as *const c_char, s.as_ptr());
            free(a as *mut c_void);
            free(b as *mut c_void);
        }
    }
}

/// CONFIGS.md row 4 — `merror` stderr bytes, including 256-byte truncation.
#[test]
fn row04_merror() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE04);

    const FSTAT_ERROR: &[u8] =
        b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
    const FSEEK_ERROR: &[u8] =
        b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

    let mut cases: Vec<(&[u8], Vec<u8>, c_int, Vec<u8>)> = vec![
        (FSTAT_ERROR, b"alerts.log".to_vec(), 2, b"No such file or directory".to_vec()),
        (FSEEK_ERROR, b"alerts.log".to_vec(), 29, b"Illegal seek".to_vec()),
        (FSEEK_ERROR, b"".to_vec(), 0, b"".to_vec()),
        (FSTAT_ERROR, b"<stdin>".to_vec(), -1, b"Success".to_vec()),
        // Long file name -> snprintf truncation at 256 bytes.
        (FSTAT_ERROR, vec![b'A'; 400], 9, b"Bad file descriptor".to_vec()),
        (FSEEK_ERROR, vec![b'z'; 250], i32::MAX, vec![b'q'; 250]),
        (FSEEK_ERROR, vec![b'z'; 190], i32::MIN, b"x".to_vec()),
    ];
    for _ in 0..60 {
        let tmpl: &[u8] = if rng.bool() { FSTAT_ERROR } else { FSEEK_ERROR };
        cases.push((
            tmpl,
            rng.token_upto(400),
            rng.i32(),
            rng.token_upto(60),
        ));
    }

    for (tmpl, name, err, msg) in cases {
        unsafe {
            let t = cbytes(tmpl.strip_suffix(b"\0").unwrap()).into_raw();
            let n = cbytes(&name);
            let m = cbytes(&msg);

            let (_, c_out) = capture_stderr(|| (p.c.merror)(t, n.as_ptr(), err, m.as_ptr()));
            let (_, r_out) = capture_stderr(|| (p.rs.merror)(t, n.as_ptr(), err, m.as_ptr()));
            let _ = std::ffi::CString::from_raw(t);

            assert_same!(
                format!("merror(name.len={},err={err})", name.len()),
                String::from_utf8_lossy(&c_out).into_owned(),
                String::from_utf8_lossy(&r_out).into_owned()
            );
            assert!(!c_out.is_empty(), "merror must write something");
        }
    }
}

/// CONFIGS.md row 5 — `FreeAlertData` on all-NULL, all-set, and mixed structs.
///
/// The struct is hand-built on the heap with `malloc`/`strdup` (so the freeing
/// library's `free` matches the allocating allocator) and each library gets its
/// own identical copy. A successful run means both walked the same nine
/// pointers; a mismatch would trip glibc's heap checks.
#[test]
fn row05_free_alert_data() {
    let p = libs();
    let mut rng = Rng::new(0xC0FFEE05);

    unsafe fn build(mask: u32) -> *mut alert_data {
        unsafe {
            let a = malloc(std::mem::size_of::<alert_data>()) as *mut alert_data;
            std::ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<alert_data>());
            (*a).rule = 1234;
            (*a).level = 7;
            (*a).srcport = -1;
            (*a).dstport = 65535;
            let f = |i: u32, s: &str| -> *mut c_char {
                if mask & (1 << i) != 0 {
                    let cst = cs(s);
                    strdup(cst.as_ptr())
                } else {
                    std::ptr::null_mut()
                }
            };
            (*a).alertid = f(0, "1500000000.1234");
            (*a).date = f(1, "2006 Apr 13 16:15:17");
            (*a).location = f(2, "/var/log/auth.log");
            (*a).comment = f(3, "Unknown problem");
            (*a).group = f(4, "syscheck,pci_dss");
            (*a).srcip = f(5, "10.0.0.1");
            (*a).dstip = f(6, "10.0.0.2");
            (*a).user = f(7, "root");
            (*a).filename = f(8, "/etc/passwd");
            a
        }
    }

    let mut masks: Vec<u32> = vec![0, 0x1FF, 0b1010_1010_1, 0b0101_0101_0, 1, 0x100];
    for _ in 0..120 {
        masks.push((rng.next_u64() & 0x1FF) as u32);
    }

    for mask in masks {
        unsafe {
            (p.c.FreeAlertData)(build(mask));
            (p.rs.FreeAlertData)(build(mask));
        }
    }

    // Also free real `GetAlertData` results with the *other* library's
    // `FreeAlertData` symbol, proving the layouts agree.
    let sc = Scratch::new("free");
    let text = AlertSpec::minimal().render();
    let path = sc.write("alerts.log", text.as_bytes());
    unsafe {
        let fc = open_ro(&path);
        let a = (p.c.GetAlertData)(0, fc);
        assert!(!a.is_null());
        (p.rs.FreeAlertData)(a); // Rust frees a C-produced struct
        fclose(fc);

        let fr = open_ro(&path);
        let b = (p.rs.GetAlertData)(0, fr);
        assert!(!b.is_null());
        (p.c.FreeAlertData)(b); // C frees a Rust-produced struct
        fclose(fr);
    }
}
