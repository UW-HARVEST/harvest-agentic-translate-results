//! CONFIGS.md rows C01, C29, C30 — the level-0 entry points:
//! `os_calloc` / `os_realloc` / `os_strdup`, `FreeAlertData`, `merror`.

mod common;

use common::*;
use std::ffi::{CString, c_char, c_void};

/// C01 — the three `shared.h` helpers over randomized sizes and strings.
#[test]
fn c01_alloc_helpers() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC01);

    // os_calloc: the returned block must be readable and fully zeroed.
    let mut sizes: Vec<(usize, usize)> = vec![
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (1, 4096),
        (4096, 1),
        (7, 13),
        (1, 96),
    ];
    for _ in 0..120 {
        sizes.push((1 + rng.below(64), 1 + rng.below(512)));
    }
    for (num, size) in sizes {
        for lib in [c, r] {
            let p = unsafe { (lib.os_calloc)(num, size) };
            assert!(
                !p.is_null(),
                "{}: os_calloc({num},{size}) returned NULL",
                lib.name
            );
            let n = num * size;
            let bytes = unsafe { std::slice::from_raw_parts(p as *const u8, n) };
            assert!(
                bytes.iter().all(|&b| b == 0),
                "{}: os_calloc({num},{size}) not zeroed",
                lib.name
            );
            unsafe { free(p) };
        }
    }

    // os_realloc: grow and shrink from a live block, contents preserved.
    for _ in 0..120 {
        let n0 = 1 + rng.below(256);
        let pattern: Vec<u8> = (0..n0).map(|_| (rng.next_u64() & 0xFF) as u8).collect();
        let n1 = 1 + rng.below(512);
        let keep = n0.min(n1);
        for lib in [c, r] {
            let p = unsafe { (lib.os_calloc)(n0, 1) };
            unsafe { std::ptr::copy_nonoverlapping(pattern.as_ptr(), p as *mut u8, n0) };
            let q = unsafe { (lib.os_realloc)(p, n1) };
            assert!(!q.is_null(), "{}: os_realloc(_, {n1}) NULL", lib.name);
            let got = unsafe { std::slice::from_raw_parts(q as *const u8, keep) };
            assert_eq!(
                got,
                &pattern[..keep],
                "{}: os_realloc lost data ({n0} -> {n1})",
                lib.name
            );
            unsafe { free(q) };
        }
    }

    // os_realloc(NULL, n) behaves as malloc in both.
    for n in [1usize, 8, 96, 1024] {
        for lib in [c, r] {
            let p = unsafe { (lib.os_realloc)(std::ptr::null_mut(), n) };
            assert!(!p.is_null(), "{}: os_realloc(NULL,{n})", lib.name);
            unsafe { free(p) };
        }
    }

    // os_strdup: byte-identical copies, including empty and long strings.
    let mut strings: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"alerts.log".to_vec(),
        vec![b'x'; 4096],
        (1..=255u8).collect(),
    ];
    for _ in 0..150 {
        strings.push(rng.token(200).into_iter().filter(|&b| b != 0).collect());
    }
    for s in strings {
        let cs = CString::new(s.clone()).unwrap();
        let mut outs = Vec::new();
        for lib in [c, r] {
            let p = unsafe { (lib.os_strdup)(cs.as_ptr()) };
            assert!(!p.is_null());
            let got =
                unsafe { std::slice::from_raw_parts(p as *const u8, strlen(p as *const c_char)) }
                    .to_vec();
            unsafe { free(p as *mut c_void) };
            outs.push(got);
        }
        assert_eq!(outs[0], outs[1], "os_strdup differs for {s:?}");
        assert_eq!(outs[0], s, "os_strdup is not a faithful copy");
    }
}

/// C29 — `FreeAlertData` over a fully populated struct, a fully NULL struct and
/// randomized partial mixes. Each library allocates and frees with its OWN
/// exported helpers, exactly as the C does internally.
#[test]
fn c29_free_alert_data() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xC29);

    // 9 owned pointers: alertid, date, location, comment, group, srcip, dstip,
    // user, filename.
    let mut masks: Vec<u16> = vec![0, 0x1FF];
    for _ in 0..120 {
        masks.push((rng.next_u64() & 0x1FF) as u16);
    }

    for mask in masks {
        let g = world();
        let run = |lib: &Lib| {
            capture_stderr(|| unsafe {
                let a = (lib.os_calloc)(1, 96) as *mut alert_data;
                assert!(!a.is_null());
                let set = |bit: u16, slot: *mut *mut c_char, text: &str| {
                    if mask & bit != 0 {
                        let cs = CString::new(text).unwrap();
                        *slot = (lib.os_strdup)(cs.as_ptr());
                    }
                };
                set(0x001, &raw mut (*a).alertid, "1461102540.1234");
                set(0x002, &raw mut (*a).date, "2016 Apr 19 20:29:00");
                set(0x004, &raw mut (*a).location, "host->/var/log/messages");
                set(0x008, &raw mut (*a).comment, "some comment");
                set(0x010, &raw mut (*a).group, "syscheck,");
                set(0x020, &raw mut (*a).srcip, "10.0.0.1");
                set(0x040, &raw mut (*a).dstip, "10.0.0.2");
                set(0x080, &raw mut (*a).user, "root");
                set(0x100, &raw mut (*a).filename, "/etc/passwd");
                (*a).rule = 1002;
                (*a).level = 7;
                (*a).srcport = 22;
                (*a).dstport = 443;
                (lib.free_alert_data)(a);
            })
            .1
        };
        let ce = run(c);
        let re = run(r);
        drop(g);
        assert_eq!(
            String::from_utf8_lossy(&ce),
            String::from_utf8_lossy(&re),
            "FreeAlertData(mask={mask:#x}) stderr differs"
        );
        assert!(ce.is_empty(), "FreeAlertData must be silent");
    }
}

/// C30 — `merror` over both templates, randomized arguments, including a
/// `file_name` long enough to truncate the 256-byte `snprintf` buffer.
#[test]
fn c30_merror() {
    let g = world();
    let (c, r) = libs();
    let templates = [
        "(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].",
        "(1116): Could not set position in file '%s' due to [(%d)-(%s)].",
        "%s|%d|%s",
        "no format specifiers at all",
        "%s",
    ];
    let mut rng = Rng::new(0xC30);
    let mut names: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"alerts.log".to_vec(),
        b"<stdin>".to_vec(),
        vec![b'L'; 255],   // exactly fills the buffer
        vec![b'L'; 256],   // one past
        vec![b'L'; 1000],  // heavy truncation
        vec![b'%'; 40],    // percent signs in the *argument*, not the format
    ];
    for _ in 0..80 {
        names.push(rng.token(400).into_iter().filter(|&b| b != 0).collect());
    }

    for tmpl in templates {
        let ct = CString::new(tmpl).unwrap();
        for name in &names {
            let cn = CString::new(name.clone()).unwrap();
            let cm = CString::new(
                rng.token(60)
                    .into_iter()
                    .filter(|&b| b != 0)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let err = rng.i32();
            let call = |lib: &Lib| {
                capture_stderr(|| unsafe {
                    (lib.merror)(ct.as_ptr(), cn.as_ptr(), err, cm.as_ptr());
                })
                .1
            };
            let a = call(c);
            let b = call(r);
            assert_eq!(
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b),
                "merror differs: tmpl={tmpl:?} name_len={} err={err}",
                name.len()
            );
            // The C always terminates with the "%s\n" newline it adds itself.
            assert!(a.ends_with(b"\n"));
            // snprintf truncates to 255 chars + NUL, then merror appends "\n".
            assert!(
                a.len() <= 256,
                "merror output must fit the 256-byte buffer (+\\n): {}",
                a.len()
            );
        }
    }
    drop(g);
}
