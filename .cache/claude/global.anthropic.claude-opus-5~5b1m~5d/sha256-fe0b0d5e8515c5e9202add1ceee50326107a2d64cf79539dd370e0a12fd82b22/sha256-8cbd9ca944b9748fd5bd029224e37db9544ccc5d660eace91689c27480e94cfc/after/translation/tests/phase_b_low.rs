//! Phase B, rows 1-5: the lowest-level exported entry points
//! (`os_calloc`, `os_realloc`, `os_strdup`, `merror`, `FreeAlertData`).

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

/* ---------------- row 1: os_calloc ---------------- */

#[test]
fn cfg_01_os_calloc() {
    let b = both();
    let mut rng = Rng::new(0x0101_0101_0101_0101);
    unsafe {
        // deterministic edge shapes + randomized ones
        let mut cases: Vec<(usize, usize)> = vec![
            (0, 0),
            (0, 16),
            (16, 0),
            (1, 1),
            (1, 96),
            (96, 1),
            (7, 13),
            (1, 4096),
            (1024, 8),
        ];
        for _ in 0..200 {
            cases.push((rng.below(64) + 1, rng.below(256) + 1));
        }

        for (num, size) in cases {
            let pc = (b.c.os_calloc)(num, size);
            let pr = (b.rs.os_calloc)(num, size);
            assert!(!pc.is_null(), "C os_calloc({num},{size}) returned NULL");
            assert!(!pr.is_null(), "Rust os_calloc({num},{size}) returned NULL");

            // both must hand back num*size zero bytes
            let n = num * size;
            let sc = std::slice::from_raw_parts(pc as *const u8, n);
            let sr = std::slice::from_raw_parts(pr as *const u8, n);
            assert!(sc.iter().all(|&x| x == 0), "C block not zeroed");
            assert_eq!(sc, sr, "os_calloc({num},{size}) contents differ");

            free(pc);
            free(pr);
        }
    }
}

/* ---------------- row 2: os_realloc ---------------- */

#[test]
fn cfg_02_os_realloc() {
    let b = both();
    let mut rng = Rng::new(0x0202_0202_0202_0202);
    unsafe {
        for _ in 0..300 {
            let n0 = rng.below(512) + 1;
            let n1 = rng.below(512) + 1;
            let pat: Vec<u8> = (0..n0).map(|i| (i as u8).wrapping_mul(31).wrapping_add(7)).collect();

            // ptr == NULL (malloc semantics)
            let mut pc = (b.c.os_realloc)(std::ptr::null_mut(), n0);
            let mut pr = (b.rs.os_realloc)(std::ptr::null_mut(), n0);
            assert!(!pc.is_null() && !pr.is_null(), "os_realloc(NULL,{n0}) NULL");
            std::ptr::copy_nonoverlapping(pat.as_ptr(), pc as *mut u8, n0);
            std::ptr::copy_nonoverlapping(pat.as_ptr(), pr as *mut u8, n0);

            // grow / shrink an existing block, content up to min(n0,n1) preserved
            pc = (b.c.os_realloc)(pc, n1);
            pr = (b.rs.os_realloc)(pr, n1);
            assert!(!pc.is_null() && !pr.is_null(), "os_realloc(p,{n1}) NULL");
            let keep = n0.min(n1);
            let sc = std::slice::from_raw_parts(pc as *const u8, keep);
            let sr = std::slice::from_raw_parts(pr as *const u8, keep);
            assert_eq!(sc, &pat[..keep], "C os_realloc lost data");
            assert_eq!(sc, sr, "os_realloc({n0}->{n1}) contents differ");

            free(pc);
            free(pr);
        }
    }
}

/* ---------------- row 3: os_strdup ---------------- */

#[test]
fn cfg_03_os_strdup() {
    let b = both();
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"x".to_vec(),
            b" ".to_vec(),
            b"'quoted'".to_vec(),
            b"a b c d".to_vec(),
            vec![0xffu8; 300],
            vec![0x80u8; 1],
            (1..=255u8).collect(),
            vec![b'z'; 4096],
        ];
        for _ in 0..300 {
            let n = rng.below(200) + 1;
            cases.push((0..n).map(|_| (rng.below(255) + 1) as u8).collect());
        }

        for case in cases {
            let cs = CString::new(case.clone()).unwrap();
            let pc = (b.c.os_strdup)(cs.as_ptr());
            let pr = (b.rs.os_strdup)(cs.as_ptr());
            assert!(!pc.is_null() && !pr.is_null());
            let sc = std::slice::from_raw_parts(pc as *const u8, strlen(pc));
            let sr = std::slice::from_raw_parts(pr as *const u8, strlen(pr));
            assert_eq!(sc, &case[..], "C os_strdup mangled input");
            assert_eq!(sc, sr, "os_strdup differs for {:?}", &case[..case.len().min(32)]);
            free(pc as *mut c_void);
            free(pr as *mut c_void);
        }
    }
}

/* ---------------- row 4: merror ---------------- */

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

#[test]
fn cfg_04_merror_both_templates() {
    let b = both();
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    let dir = tmp_root();
    unsafe {
        let templates: [&[u8]; 4] = [
            FSTAT_ERROR,
            FSEEK_ERROR,
            b"no conversions at all\0",
            b"%s|%d|%s\0",
        ];
        for i in 0..200 {
            let tpl = templates[i % templates.len()];
            // occasionally long enough to overflow merror's 256-byte buffer
            let name: Vec<u8> = if i % 7 == 0 {
                vec![b'L'; 200 + rng.below(200)]
            } else {
                rng.token(60)
            };
            let msg: Vec<u8> = if i % 11 == 0 {
                vec![b'M'; 150 + rng.below(200)]
            } else {
                rng.token(40)
            };
            let err: c_int = rng.i32();

            let cname = CString::new(name.clone()).unwrap();
            let cmsg = CString::new(msg.clone()).unwrap();

            let out_c = capture_stderr(&dir.join("merror_c.txt"), || {
                (b.c.merror)(
                    tpl.as_ptr() as *const c_char,
                    cname.as_ptr(),
                    err,
                    cmsg.as_ptr(),
                )
            });
            let out_r = capture_stderr(&dir.join("merror_r.txt"), || {
                (b.rs.merror)(
                    tpl.as_ptr() as *const c_char,
                    cname.as_ptr(),
                    err,
                    cmsg.as_ptr(),
                )
            });
            assert_eq!(
                out_c, out_r,
                "merror stderr differs (tpl={:?}, err={err})",
                String::from_utf8_lossy(tpl)
            );
            assert!(out_c.ends_with(b"\n"), "C merror did not end with a newline");
            assert!(out_c.len() <= 256, "C merror wrote more than 255+NL bytes");
        }
    }
}

/* ---------------- row 5: FreeAlertData ---------------- */

/// Build an `alert_data` with `imp`'s own allocator and free it with `freer`'s
/// `FreeAlertData` -- both directions, so the C struct is released by the Rust
/// object and vice versa.
unsafe fn free_roundtrip(alloc: &Impl, freer: &Impl, populate: u32) {
    let p = (alloc.os_calloc)(1, std::mem::size_of::<alert_data>()) as *mut alert_data;
    assert!(!p.is_null());
    let a = &mut *p;
    a.rule = 1234;
    a.level = 7;
    a.srcport = -1;
    a.dstport = 65536;
    let mut set = |bit: u32, slot: &mut *mut c_char, text: &[u8]| {
        if populate & bit != 0 {
            let cs = CString::new(text).unwrap();
            *slot = (alloc.os_strdup)(cs.as_ptr());
        }
    };
    set(1 << 0, &mut a.alertid, b"1500000000.1");
    set(1 << 1, &mut a.date, b"2006 Apr 13 16:15:17");
    set(1 << 2, &mut a.location, b"host->/var/log/x");
    set(1 << 3, &mut a.comment, b"some comment");
    set(1 << 4, &mut a.group, b"syscheck,");
    set(1 << 5, &mut a.srcip, b"1.2.3.4");
    set(1 << 6, &mut a.dstip, b"5.6.7.8");
    set(1 << 7, &mut a.user, b"root");
    set(1 << 8, &mut a.filename, b"/etc/passwd");

    (freer.FreeAlertData)(p);
}

#[test]
fn cfg_05_free_alert_data_all_field_subsets_cross_freed() {
    let b = both();
    unsafe {
        // all-NULL, all-set, and every single-field case, plus a sampling of mixes
        let mut masks: Vec<u32> = vec![0, 0x1ff];
        for i in 0..9 {
            masks.push(1 << i);
        }
        let mut rng = Rng::new(0x0505_0505_0505_0505);
        for _ in 0..64 {
            masks.push((rng.next_u64() & 0x1ff) as u32);
        }
        for m in masks {
            free_roundtrip(&b.c, &b.c, m);
            free_roundtrip(&b.rs, &b.rs, m);
            free_roundtrip(&b.c, &b.rs, m); // C-allocated, Rust-freed
            free_roundtrip(&b.rs, &b.c, m); // Rust-allocated, C-freed
        }
    }
}

/// `FreeAlertData` on a struct produced by the *other* library's parser.
#[test]
fn cfg_05b_free_parser_output_cross() {
    let b = both();
    let dir = tmp_root();
    let src = simple_alert();
    unsafe {
        for (parser, freer) in [(&b.c, &b.rs), (&b.rs, &b.c), (&b.c, &b.c), (&b.rs, &b.rs)] {
            let fp = open_bytes(&dir, "cross_free.log", &src);
            let a = (parser.GetAlertData)(0, fp);
            assert!(!a.is_null(), "{} failed to parse", parser.name);
            let s = snap(a);
            (freer.FreeAlertData)(a);
            assert!(s.is_some());
            fclose(fp);
        }
    }
}
