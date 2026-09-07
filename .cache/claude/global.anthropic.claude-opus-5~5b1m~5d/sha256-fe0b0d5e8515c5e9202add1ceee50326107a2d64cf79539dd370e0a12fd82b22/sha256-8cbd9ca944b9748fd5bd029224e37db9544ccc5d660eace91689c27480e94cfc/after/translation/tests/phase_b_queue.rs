//! Phase B, rows 24-44: `Init_FileQueue`, `Read_FileMon` (the low-level queue
//! API, driven directly) and the `driver` one-shot wrapper.

mod common;
use common::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};

const MONTHS: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

fn multi_alert(n: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(b"** Alert 15000000");
        v.extend_from_slice(format!("{:02}", i).as_bytes());
        v.extend_from_slice(b".1: ");
        v.extend_from_slice(if i % 2 == 0 { b"mail" } else { b"noop" });
        v.extend_from_slice(b" - ossec,syscheck,\n");
        v.extend_from_slice(b"2006 Apr 13 16:15:1");
        v.extend_from_slice(format!("{}", i % 10).as_bytes());
        v.extend_from_slice(b" host->/var/log/x\n");
        v.extend_from_slice(b"Rule: ");
        v.extend_from_slice(format!("{}", 500 + i).as_bytes());
        v.extend_from_slice(b" (level ");
        v.extend_from_slice(format!("{}", i % 16).as_bytes());
        v.extend_from_slice(b") -> 'comment ");
        v.extend_from_slice(&rng.token(10));
        v.extend_from_slice(b"'\n");
        v.extend_from_slice(b"Src IP: 10.0.0.");
        v.extend_from_slice(format!("{}", i).as_bytes());
        v.extend_from_slice(b"\nSrc Port: ");
        v.extend_from_slice(format!("{}", 1000 + i).as_bytes());
        v.extend_from_slice(b"\nDst IP: 10.1.0.1\nDst Port: 22\nUser: u");
        v.extend_from_slice(format!("{}", i).as_bytes());
        v.extend_from_slice(b"\nIntegrity checksum changed for: '/etc/f");
        v.extend_from_slice(format!("{}", i).as_bytes());
        v.extend_from_slice(b"'\nsome trailing log\n\n");
    }
    v
}

/* ================= rows 24, 25, 28, 29, 30: Init_FileQueue ================= */

/// Run `Init_FileQueue` on both libraries with a fresh zeroed `file_queue`
/// and compare the return value plus every observable field.
#[track_caller]
unsafe fn cmp_init(t: &tm, flags: c_int, include_mon: bool) -> (c_int, QueueSnap) {
    let b = both();
    let mut qc = file_queue::zeroed();
    let mut qr = file_queue::zeroed();
    let rc = (b.c.Init_FileQueue)(&mut qc, t, flags);
    let rr = (b.rs.Init_FileQueue)(&mut qr, t, flags);
    let sc = qsnap(&qc, include_mon);
    let sr = qsnap(&qr, include_mon);
    assert_eq!(rc, rr, "Init_FileQueue return differs (flags={flags:#x})");
    assert_eq!(sc, sr, "file_queue differs (flags={flags:#x})");
    if !qc.fp.is_null() {
        fclose(qc.fp);
    }
    if !qr.fp.is_null() {
        fclose(qr.fp);
    }
    (rc, sc)
}

#[test]
fn cfg_24_25_28_30_init_file_queue_present_file() {
    let w = workdir("r24");
    let mut rng = Rng::new(0x2424_2424_2424_2424);
    unsafe {
        for &(name, body) in &[
            ("empty", &b""[..]),
            ("small", &b"x\n"[..]),
            ("alerts", &b"** Alert 1500000000.1: mail - syslog,\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n"[..]),
        ] {
            w.write("alerts.log", body);
            for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET, CRALERT_MAIL_SET | CRALERT_READ_ALL, CRALERT_EXEC_SET | CRALERT_READ_FAILED] {
                for mon in 0..12 {
                    let day = if rng.bool() { rng.i32() } else { rng.below(32) as c_int };
                    let year = if rng.bool() { rng.i32() } else { rng.below(200) as c_int };
                    let t = tm::new(day, mon, year);
                    let (rc, s) = cmp_init(&t, flags, true);

                    // row 24/25/28/30 expectations, verified against C's own result
                    assert_eq!(rc, 0, "{name}/{flags:#x}: expected success");
                    assert_eq!(s.day, day);
                    assert_eq!(s.year, year.wrapping_add(1900));
                    assert_eq!(&s.mon[..3], MONTHS[mon as usize], "mon for {mon}");
                    assert_eq!(s.mon[3], 0, "mon[3] must stay NUL");
                    assert_eq!(s.file_name, b"alerts.log".to_vec());
                    assert_eq!(s.flags, flags);
                    assert!(!s.fp_null);
                    assert_eq!(s.st_size, body.len() as i64);
                    assert_eq!(s.last_change, s.st_mtime);
                    // READ_ALL -> offset 0, otherwise seeked to EOF
                    let want = if flags & CRALERT_READ_ALL != 0 { 0 } else { body.len() as i64 };
                    assert_eq!(s.offset, want, "{name}/{flags:#x}: wrong start offset");
                }
            }
        }
    }
}

#[test]
fn cfg_29_init_file_queue_absent_file() {
    let w = workdir("r29");
    w.remove("alerts.log");
    unsafe {
        for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET, 0x20, -1 & !CRALERT_FP_SET] {
            for mon in [0, 5, 11] {
                let t = tm::new(3, mon, 106);
                let (rc, s) = cmp_init(&t, flags, true);
                assert_eq!(rc, 0, "Handle_Queue returns 0, not <0");
                assert!(s.fp_null);
                assert_eq!(s.last_change, 0);
                assert_eq!(s.file_name, b"alerts.log".to_vec());
            }
        }
    }
}

/* ================= rows 26, 27: CRALERT_FP_SET ================= */

/// `Init_FileQueue` with a caller-supplied `fp` (`CRALERT_FP_SET`).
#[track_caller]
unsafe fn cmp_init_fp_set(body: &[u8], flags: c_int, start: i64, t: &tm) -> QueueSnap {
    let b = both();
    let d = tmp_root();
    let mut qc = file_queue::zeroed();
    let mut qr = file_queue::zeroed();
    qc.fp = open_bytes(&d, "fpset_c.log", body);
    qr.fp = open_bytes(&d, "fpset_r.log", body);
    fseek(qc.fp, start as _, SEEK_SET);
    fseek(qr.fp, start as _, SEEK_SET);

    let rc = (b.c.Init_FileQueue)(&mut qc, t, flags);
    let rr = (b.rs.Init_FileQueue)(&mut qr, t, flags);
    // st_ino/st_size differ because the two streams are over distinct files
    let mut sc = qsnap(&qc, true);
    let mut sr = qsnap(&qr, true);
    assert_eq!(sc.st_size, body.len() as i64);
    assert_eq!(sr.st_size, body.len() as i64);
    sc.st_ino = 0;
    sr.st_ino = 0;
    sc.st_mtime = 0;
    sr.st_mtime = 0;
    sc.last_change = 0;
    sr.last_change = 0;
    assert_eq!(rc, rr, "Init_FileQueue(FP_SET) return differs");
    assert_eq!(sc, sr, "file_queue(FP_SET) differs (flags={flags:#x})");
    if !qc.fp.is_null() {
        fclose(qc.fp);
    }
    if !qr.fp.is_null() {
        fclose(qr.fp);
    }
    sc
}

#[test]
fn cfg_26_27_fp_set_uses_callers_stream() {
    let body = multi_alert(3, 1);
    unsafe {
        for mon in [0, 3, 11] {
            let t = tm::new(9, mon, 123);
            // row 26: FP_SET only -> seek to EOF
            let s = cmp_init_fp_set(&body, CRALERT_FP_SET, 0, &t);
            assert_eq!(s.file_name, b"<stdin>".to_vec());
            assert!(!s.fp_null, "caller fp must be kept");
            assert_eq!(s.offset, body.len() as i64);

            // row 27: FP_SET|READ_ALL -> stream position untouched
            for start in [0i64, 5, 40, body.len() as i64] {
                let s = cmp_init_fp_set(&body, CRALERT_FP_SET | CRALERT_READ_ALL, start, &t);
                assert_eq!(s.file_name, b"<stdin>".to_vec());
                assert_eq!(s.offset, start, "READ_ALL must not seek");
            }
        }
    }
}

/* ================= rows 31-33, 35, 37: Read_FileMon ================= */

/// Full end-to-end low-level drive: `Init_FileQueue` then repeated
/// `Read_FileMon` on the *same* queue, comparing every returned alert.
#[track_caller]
unsafe fn cmp_read_filemon(
    t: &tm,
    flags: c_int,
    timeout: c_uint,
    calls: usize,
) -> Vec<Option<AlertSnap>> {
    let b = both();
    let mut qc = file_queue::zeroed();
    let mut qr = file_queue::zeroed();
    let rc = (b.c.Init_FileQueue)(&mut qc, t, flags);
    let rr = (b.rs.Init_FileQueue)(&mut qr, t, flags);
    assert_eq!(rc, rr);

    let mut outc = Vec::new();
    let mut outr = Vec::new();
    for _ in 0..calls {
        let ac = (b.c.Read_FileMon)(&mut qc, t, timeout);
        let sc = snap(ac);
        if !ac.is_null() {
            (b.c.FreeAlertData)(ac);
        }
        let ar = (b.rs.Read_FileMon)(&mut qr, t, timeout);
        let sr = snap(ar);
        if !ar.is_null() {
            (b.rs.FreeAlertData)(ar);
        }
        assert_eq!(sc, sr, "Read_FileMon differs (flags={flags:#x})");
        let done = sc.is_none();
        outc.push(sc);
        outr.push(sr);
        if done {
            break;
        }
    }
    assert_eq!(
        qsnap(&qc, true),
        qsnap(&qr, true),
        "file_queue after Read_FileMon differs (flags={flags:#x})"
    );
    if !qc.fp.is_null() {
        fclose(qc.fp);
    }
    if !qr.fp.is_null() {
        fclose(qr.fp);
    }
    outc
}

#[test]
fn cfg_31_32_33_35_37_read_filemon() {
    let w = workdir("r31");
    let mut rng = Rng::new(0x3131_3131_3131_3131);
    unsafe {
        for n in [1usize, 2, 5] {
            w.write("alerts.log", &multi_alert(n, n as u64 * 7 + 1));
            for flags in [
                CRALERT_READ_ALL,                        // row 31/32
                CRALERT_READ_ALL | CRALERT_MAIL_SET,     // row 33
                0,                                       // row 35
                CRALERT_MAIL_SET,
                CRALERT_READ_ALL | CRALERT_EXEC_SET | CRALERT_READ_FAILED,
            ] {
                for _ in 0..4 {
                    // row 37: randomized tm on the second-pass GetFile_Queue path
                    let t = tm::new(rng.i32(), rng.below(12) as c_int, rng.i32());
                    let got = cmp_read_filemon(&t, flags, 0, n + 2);
                    if flags & CRALERT_READ_ALL != 0 {
                        assert!(
                            got[0].is_some(),
                            "READ_ALL should yield an alert (flags={flags:#x})"
                        );
                    } else {
                        // row 35: seeked to EOF, timeout 0 -> nothing, no sleep
                        assert!(got[0].is_none(), "non-READ_ALL must yield NULL");
                    }
                }
            }
        }
    }
}

/* ================= row 34: Read_FileMon on a caller-supplied fp ================= */

#[test]
fn cfg_34_read_filemon_fp_set_read_all() {
    let b = both();
    let d = tmp_root();
    let body = multi_alert(4, 34);
    // NOTE: once the caller's stream is drained, `Read_FileMon` calls
    // `Handle_Queue(fileq, 0)` -- flags literal 0, so it fcloses the caller's fp
    // and re-`fopen`s "<stdin>", which fails and costs one 5 s `file_sleep()`
    // per library. Kept to exactly one month value to bound the runtime.
    unsafe {
        for mon in [6] {
            let t = tm::new(1, mon, 100);
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            qc.fp = open_bytes(&d, "r34_c.log", &body);
            qr.fp = open_bytes(&d, "r34_r.log", &body);
            let flags = CRALERT_FP_SET | CRALERT_READ_ALL;
            assert_eq!(
                (b.c.Init_FileQueue)(&mut qc, &t, flags),
                (b.rs.Init_FileQueue)(&mut qr, &t, flags)
            );
            let mut any = false;
            for _ in 0..6 {
                let ac = (b.c.Read_FileMon)(&mut qc, &t, 0);
                let sc = snap(ac);
                if !ac.is_null() {
                    (b.c.FreeAlertData)(ac);
                }
                let ar = (b.rs.Read_FileMon)(&mut qr, &t, 0);
                let sr = snap(ar);
                if !ar.is_null() {
                    (b.rs.FreeAlertData)(ar);
                }
                assert_eq!(sc, sr, "Read_FileMon(FP_SET|READ_ALL) differs");
                if sc.is_some() {
                    any = true;
                } else {
                    break;
                }
            }
            assert!(any, "expected at least one alert from the caller's stream");
            if !qc.fp.is_null() {
                fclose(qc.fp);
            }
            if !qr.fp.is_null() {
                fclose(qr.fp);
            }
        }
    }
}

/* ================= rows 38-43: driver ================= */

#[track_caller]
unsafe fn cmp_driver(day: c_int, month: c_int, year: c_int, timeout: c_uint, flags: c_int) {
    let b = both();
    let ac = (b.c.driver)(day, month, year, timeout, flags);
    let sc = snap(ac);
    if !ac.is_null() {
        (b.c.FreeAlertData)(ac);
    }
    let ar = (b.rs.driver)(day, month, year, timeout, flags);
    let sr = snap(ar);
    if !ar.is_null() {
        (b.rs.FreeAlertData)(ar);
    }
    assert_eq!(
        sc, sr,
        "driver({day},{month},{year},{timeout},{flags:#x}) differs"
    );
}

#[test]
fn cfg_38_39_driver_read_all() {
    let w = workdir("r38");
    let mut rng = Rng::new(0x3839_3839_3839_3839);
    unsafe {
        for n in [1usize, 3, 6] {
            w.write("alerts.log", &multi_alert(n, 100 + n as u64));
            for flags in [
                CRALERT_READ_ALL,
                CRALERT_READ_ALL | CRALERT_MAIL_SET,
                CRALERT_READ_ALL | CRALERT_EXEC_SET,
                CRALERT_READ_ALL | CRALERT_READ_FAILED,
            ] {
                for _ in 0..25 {
                    cmp_driver(rng.i32(), rng.below(12) as c_int, rng.i32(), 0, flags);
                }
            }
        }
    }
}

#[test]
fn cfg_40_driver_default_flags_seeks_to_eof() {
    let w = workdir("r40");
    let b = both();
    unsafe {
        w.write("alerts.log", &multi_alert(3, 40));
        for flags in [0, CRALERT_MAIL_SET, CRALERT_EXEC_SET | CRALERT_READ_FAILED] {
            let ac = (b.c.driver)(5, 3, 106, 0, flags);
            let ar = (b.rs.driver)(5, 3, 106, 0, flags);
            assert_eq!(snap(ac), snap(ar));
            assert!(ac.is_null(), "EOF-seek path must return NULL");
            assert!(ar.is_null(), "EOF-seek path must return NULL");
        }
    }
}

#[test]
fn cfg_41_driver_fp_set_uses_stdin_name() {
    let w = workdir("r41");
    unsafe {
        // A real file named `<stdin>` makes Read_FileMon's Handle_Queue(0) succeed
        // (and keeps the 5 s file_sleep out of the picture).
        w.write("<stdin>", &multi_alert(2, 41));
        w.write("alerts.log", &multi_alert(2, 99));
        for flags in [
            CRALERT_FP_SET | CRALERT_READ_ALL,
            CRALERT_FP_SET | CRALERT_READ_ALL | CRALERT_MAIL_SET,
        ] {
            cmp_driver(5, 3, 106, 0, flags);
        }
    }
}

#[test]
fn cfg_42_driver_arbitrary_flag_ints() {
    let w = workdir("r42");
    let mut rng = Rng::new(0x4242_4242_4242_4242);
    unsafe {
        w.write("alerts.log", &multi_alert(3, 42));
        w.write("<stdin>", &multi_alert(2, 43));
        let mut flags: Vec<c_int> = Vec::new();
        // full cross-product of the five documented bits
        for m in 0..32 {
            flags.push(m as c_int);
        }
        // plus unknown-bit noise, including out-of-range "enum" values
        flags.extend_from_slice(&[0x20, 0x40, 0x1000, -1, i32::MIN, i32::MAX, -2, 0x7fff_fff0]);
        for _ in 0..40 {
            flags.push(rng.i32());
        }
        for f in flags {
            cmp_driver(5, 3, 106, 0, f);
        }
    }
}

#[test]
fn cfg_43_44_driver_and_getalertdata_file_fuzz() {
    let w = workdir("r43");
    let d = tmp_root();
    let b = both();
    let mut rng = Rng::new(0x4344_4344_4344_4344);

    // A pool of line shapes drawn from every branch of the parser.
    let shapes: [&[u8]; 22] = [
        b"** Alert 1500000000.1: mail - ossec,syscheck,",
        b"** Alert 1500000000.2: noop - syslog,errors,",
        b"** Alert 1500000000.3: mail",
        b"** Alert bogus",
        b"** Alert",
        b"2006 Apr 13 16:15:17 host->/var/log/auth.log",
        b"2006 Apr 13 161517 nocolonspace",
        b"nocolon at all here",
        b"Rule: 5715 (level 5) -> 'ok comment'",
        b"Rule: 5715 (level 5) -> 'unterminated",
        b"Rule: 5715 nolevel noquote",
        b"Rule: 5715",
        b"Src IP: 1.2.3.4",
        b"Src Port: 65536",
        b"Src Port: notanumber",
        b"Dst IP: 5.6.7.8",
        b"Dst Port: -7",
        b"User: root",
        b"Integrity checksum changed for: '/etc/passwd'",
        b"Integrity checksum changed for: '",
        b"Old md5sum was: deadbeef",
        b"",
    ];

    unsafe {
        for i in 0..200 {
            let mut file = Vec::new();
            for _ in 0..(1 + rng.below(14)) {
                file.extend_from_slice(rng.pick(&shapes));
                file.push(b'\n');
            }
            if rng.below(4) == 0 {
                while file.last() == Some(&b'\n') {
                    file.pop();
                }
            }
            let flag = *rng.pick(&[
                0,
                CRALERT_MAIL_SET,
                CRALERT_READ_ALL,
                CRALERT_READ_ALL | CRALERT_MAIL_SET,
            ]);

            // row 44: lowest level, drained to exhaustion
            let dc = drain(&b.c, &d, &format!("r44_{i}"), &file, flag, 20);
            let dr = drain(&b.rs, &d, &format!("r44_{i}"), &file, flag, 20);
            if dc != dr {
                panic!(
                    "GetAlertData fuzz divergence #{i} flag={flag:#x}\n--- input ---\n{}\n--- C ---\n{dc:#?}\n--- Rust ---\n{dr:#?}",
                    String::from_utf8_lossy(&file)
                );
            }

            // row 43: same bytes through the full driver pipeline
            w.write("alerts.log", &file);
            cmp_driver(5, 3, 106, 0, CRALERT_READ_ALL | (flag & CRALERT_MAIL_SET));
        }
    }
}

/* ================= file_name construction sanity ================= */

#[test]
fn cfg_26b_file_name_is_stdin_iff_fp_set() {
    let w = workdir("r26b");
    w.write("alerts.log", b"");
    unsafe {
        for f in 0..32 {
            let t = tm::new(1, 0, 100);
            let mut qc = file_queue::zeroed();
            let mut qr = file_queue::zeroed();
            (b"".as_ptr(), 0);
            let rc = (both().c.Init_FileQueue)(&mut qc, &t, f);
            let rr = (both().rs.Init_FileQueue)(&mut qr, &t, f);
            assert_eq!(rc, rr);
            let sc = qsnap(&qc, true);
            let sr = qsnap(&qr, true);
            assert_eq!(sc.file_name, sr.file_name);
            let want: &[u8] = if f & CRALERT_FP_SET != 0 {
                b"<stdin>"
            } else {
                b"alerts.log"
            };
            assert_eq!(sc.file_name, want.to_vec(), "flags={f:#x}");
            if !qc.fp.is_null() {
                fclose(qc.fp);
            }
            if !qr.fp.is_null() {
                fclose(qr.fp);
            }
        }
    }
}

/// The `file_name` buffer must be NUL-terminated at both index 0 (cleared) and
/// `MAX_FQUEUE`, and `snprintf` is bounded to `MAX_FQUEUE` bytes.
#[test]
fn cfg_26c_file_name_buffer_bounds() {
    let w = workdir("r26c");
    w.write("alerts.log", b"");
    unsafe {
        let t = tm::new(1, 0, 100);
        let mut qc = file_queue::zeroed();
        let mut qr = file_queue::zeroed();
        // pre-poison the whole buffer so we can see exactly what gets written
        for i in 0..=MAX_FQUEUE {
            qc.file_name[i] = 0x41;
            qr.file_name[i] = 0x41;
        }
        (both().c.Init_FileQueue)(&mut qc, &t, 0);
        (both().rs.Init_FileQueue)(&mut qr, &t, 0);
        let bc: Vec<u8> = qc.file_name.iter().map(|&x| x as u8).collect();
        let br: Vec<u8> = qr.file_name.iter().map(|&x| x as u8).collect();
        assert_eq!(bc, br, "whole file_name buffer must match byte for byte");
        assert_eq!(&bc[..11], b"alerts.log\0");
        assert!(bc[11..].iter().all(|&x| x == 0));
        if !qc.fp.is_null() {
            fclose(qc.fp);
        }
        if !qr.fp.is_null() {
            fclose(qr.fp);
        }
        let _ = CString::new("x").unwrap();
        let _: *const c_char = std::ptr::null();
    }
}
