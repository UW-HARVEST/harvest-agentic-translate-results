//! Phase B — CONFIGS.md rows 25..38 and 45: `Init_FileQueue` and
//! `Read_FileMon` driven directly through both `.so`s (not via `driver`), with
//! the full 440-byte `file_queue` compared after every step.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::{c_int, c_uint};

const ALL_FLAGS: c_int =
    CRALERT_MAIL_SET | CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_FP_SET;

const MONTHS: [&[u8; 3]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

fn sample_alerts(n: usize) -> String {
    let specs: Vec<AlertSpec> = (0..n)
        .map(|i| {
            let mut a = AlertSpec::minimal();
            a.id = format!("15000000{:02}.{}", i, i * 7);
            a.group = format!("g{i},syslog");
            a.location = format!("/var/log/f{i}.log");
            a.rule = Some(format!(
                "Rule: {} (level {}) -> 'comment {i}'",
                1000 + i,
                i % 16
            ));
            a.srcip = Some(format!("Src IP: 10.0.0.{}", i % 256));
            a.srcport = Some(format!("Src Port: {}", 1000 + i));
            a.user = Some(format!("User: u{i}"));
            a
        })
        .collect();
    render_all(&specs)
}

/// Differential `Init_FileQueue`: identical `tm`, identical flags, identical
/// `alerts.log`; the whole `file_queue` plus the return code is compared.
///
/// `supply_fp` is used for the `CRALERT_FP_SET` rows: each library gets its own
/// freshly-opened `FILE*` on the same file.
fn diff_init(
    tag: &str,
    dir: &std::path::Path,
    t: &tm,
    flags: c_int,
    supply_fp: bool,
    dirty: bool,
) -> (c_int, FqSnap) {
    let p = libs();
    let _cwd = enter_dir(dir);
    let file = dir.join("alerts.log");

    unsafe {
        let run = |lib: &Lib| -> (c_int, FqSnap) {
            let mut fq: file_queue = if dirty {
                let mut raw = std::mem::MaybeUninit::<file_queue>::uninit();
                std::ptr::write_bytes(raw.as_mut_ptr() as *mut u8, 0xAA, size_of::<file_queue>());
                let mut v = raw.assume_init();
                // A garbage FILE* would be dereferenced by the FP_SET path;
                // keep that one field sane.
                v.fp = std::ptr::null_mut();
                v
            } else {
                file_queue::default()
            };
            if supply_fp {
                assert!(file.exists(), "FP_SET rows need an existing file");
                fq.fp = open_ro(&file);
            }
            let rc = (lib.Init_FileQueue)(&mut fq, t, flags);
            let snap = snap_fq(&fq);
            if !fq.fp.is_null() {
                fclose(fq.fp);
            }
            (rc, snap)
        };

        let c = run(&p.c);
        let r = run(&p.rs);
        assert!(
            c == r,
            "DIVERGENCE in Init_FileQueue [{tag}] flags={flags:#x} \
             tm_mday={} tm_mon={} tm_year={}\n  C   : {:#?}\n  Rust: {:#?}",
            t.tm_mday,
            t.tm_mon,
            t.tm_year,
            c,
            r
        );
        c
    }
}

/// Row 25 — `flags = 0`, `alerts.log` present: open + seek-to-END + `fstat`.
#[test]
fn row25_init_default_flags() {
    let sc = Scratch::new("init25");
    let text = sample_alerts(3);
    sc.write("alerts.log", text.as_bytes());
    let t = tm {
        tm_mday: 13,
        tm_mon: 3,
        tm_year: 106,
        ..Default::default()
    };
    let (rc, snap) = diff_init("default", &sc.dir, &t, 0, false, false);
    assert_eq!(rc, 0);
    assert_eq!(snap.file_name, b"alerts.log");
    assert_eq!(&snap.mon[..3], b"Apr");
    assert_eq!(snap.year, 2006);
    assert_eq!(snap.day, 13);
    // seek-to-END happened.
    assert_eq!(snap.offset, Some(text.len() as i64));
}

/// Row 26 — `CRALERT_READ_ALL`: no seek-to-end, offset stays 0.
#[test]
fn row26_init_read_all() {
    let sc = Scratch::new("init26");
    sc.write("alerts.log", sample_alerts(3).as_bytes());
    let t = tm {
        tm_mday: 1,
        tm_mon: 0,
        tm_year: 70,
        ..Default::default()
    };
    let (rc, snap) = diff_init("read-all", &sc.dir, &t, CRALERT_READ_ALL, false, false);
    assert_eq!(rc, 0);
    assert_eq!(snap.offset, Some(0));
    assert_eq!(snap.file_name, b"alerts.log");
}

/// Row 27 — `CRALERT_FP_SET` with a caller-supplied `fp`: `fp` preserved, name
/// becomes `<stdin>`, seek-to-END still applied.
#[test]
fn row27_init_fp_set() {
    let sc = Scratch::new("init27");
    let text = sample_alerts(2);
    sc.write("alerts.log", text.as_bytes());
    let t = tm {
        tm_mday: 29,
        tm_mon: 11,
        tm_year: 200,
        ..Default::default()
    };
    let (rc, snap) = diff_init("fp-set", &sc.dir, &t, CRALERT_FP_SET, true, false);
    assert_eq!(rc, 0);
    assert!(!snap.fp_is_null);
    assert_eq!(snap.file_name, b"<stdin>");
    assert_eq!(&snap.mon[..3], b"Dec");
    assert_eq!(snap.offset, Some(text.len() as i64));
}

/// Row 28 — `CRALERT_FP_SET | CRALERT_READ_ALL`: `fp` preserved, no seek.
#[test]
fn row28_init_fp_set_read_all() {
    let sc = Scratch::new("init28");
    sc.write("alerts.log", sample_alerts(2).as_bytes());
    let t = tm {
        tm_mday: 5,
        tm_mon: 6,
        tm_year: 123,
        ..Default::default()
    };
    let (rc, snap) = diff_init(
        "fp-set+read-all",
        &sc.dir,
        &t,
        CRALERT_FP_SET | CRALERT_READ_ALL,
        true,
        false,
    );
    assert_eq!(rc, 0);
    assert_eq!(snap.offset, Some(0));
    assert_eq!(snap.file_name, b"<stdin>");
}

/// Row 29 — every flag bit alone, the full mask, and randomized undefined bits.
#[test]
fn row29_init_flag_matrix() {
    let sc = Scratch::new("init29");
    sc.write("alerts.log", sample_alerts(2).as_bytes());
    let t = tm {
        tm_mday: 2,
        tm_mon: 1,
        tm_year: 99,
        ..Default::default()
    };

    let mut flags: Vec<c_int> = vec![
        0,
        CRALERT_MAIL_SET,
        CRALERT_EXEC_SET,
        CRALERT_READ_ALL,
        CRALERT_READ_FAILED,
        CRALERT_FP_SET,
        CRALERT_FP_SET | CRALERT_READ_ALL,
        CRALERT_MAIL_SET | CRALERT_READ_ALL,
        ALL_FLAGS,
        0x20,
        0x7FFF_FFFF,
    ];
    let mut rng = Rng::new(0xF00D_0029);
    for _ in 0..60 {
        flags.push(rng.i32());
    }

    for flags in flags {
        // FP_SET rows need a caller-supplied fp; give one whenever the bit is
        // set so the C code follows its intended path instead of NULL-deref.
        let supply = flags & CRALERT_FP_SET != 0;
        diff_init("flag-matrix", &sc.dir, &t, flags, supply, false);
    }
}

/// Row 30 — all twelve `tm_mon` values map to `Jan`..`Dec` in `mon[4]`.
#[test]
fn row30_init_all_months() {
    let sc = Scratch::new("init30");
    sc.write("alerts.log", b"");
    for mon in 0..12 {
        let t = tm {
            tm_mday: 1 + mon,
            tm_mon: mon,
            tm_year: 100 + mon,
            ..Default::default()
        };
        let (rc, snap) = diff_init("months", &sc.dir, &t, CRALERT_READ_ALL, false, false);
        assert_eq!(rc, 0);
        assert_eq!(
            &snap.mon[..3],
            &MONTHS[mon as usize][..],
            "mon for tm_mon={mon}"
        );
        // `strncpy(dst, src, 3)` copies exactly 3 bytes; mon[3] comes from the
        // caller's zeroed struct.
        assert_eq!(snap.mon[3], 0);
    }
}

/// Row 31 — randomized `tm_mday` / `tm_year` extremes (`year = tm_year + 1900`
/// overflows without any check in C).
#[test]
fn row31_init_tm_extremes() {
    let sc = Scratch::new("init31");
    sc.write("alerts.log", sample_alerts(1).as_bytes());
    let mut rng = Rng::new(0xF00D_0031);

    let mut cases: Vec<(c_int, c_int)> = vec![
        (0, 0),
        (-1, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (1, i32::MAX - 1899),
        (31, -1900),
        (i32::MAX, i32::MIN),
    ];
    for _ in 0..120 {
        cases.push((rng.i32(), rng.i32()));
    }

    for (mday, year) in cases {
        let t = tm {
            tm_mday: mday,
            tm_mon: (rng.below(12)) as c_int,
            tm_year: year,
            ..Default::default()
        };
        let (rc, snap) = diff_init("tm-extremes", &sc.dir, &t, CRALERT_READ_ALL, false, false);
        assert_eq!(rc, 0);
        assert_eq!(snap.day, mday);
        assert_eq!(snap.year, year.wrapping_add(1900));
    }
}

/// Row 32 — pre-dirtied `file_queue` (0xAA fill): checks the field resets and
/// the `memset(file_name, 0, MAX_FQUEUE+1)`.
#[test]
fn row32_init_dirty_struct() {
    let sc = Scratch::new("init32");
    sc.write("alerts.log", sample_alerts(2).as_bytes());
    let t = tm {
        tm_mday: 7,
        tm_mon: 8,
        tm_year: 101,
        ..Default::default()
    };
    for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET | CRALERT_READ_ALL] {
        let (rc, snap) = diff_init("dirty", &sc.dir, &t, flags, false, true);
        assert_eq!(rc, 0);
        // `mon` gets 3 bytes from strncpy; mon[3] is still the 0xAA garbage,
        // exactly as in C.
        assert_eq!(&snap.mon[..3], b"Sep");
        assert_eq!(snap.mon[3], 0xAA);
        assert_eq!(snap.file_name, b"alerts.log");
        // Everything past the NUL must be zero from the memset.
        assert!(snap.file_name_raw[11..].iter().all(|&b| b == 0));
    }
}

// ---------------------------------------------------------------------------
// Read_FileMon
// ---------------------------------------------------------------------------

/// Drive `Init_FileQueue` then `Read_FileMon` `rounds` times, snapshotting the
/// `file_queue` after every single call.
unsafe fn init_then_read(
    lib: &Lib,
    dir: &std::path::Path,
    t_init: &tm,
    t_read: &tm,
    flags: c_int,
    supply_fp: bool,
    timeout: c_uint,
    rounds: usize,
) -> (c_int, Vec<FqSnap>, Vec<AlertSnap>) {
    unsafe {
        let mut fq = file_queue::default();
        if supply_fp {
            fq.fp = open_ro(&dir.join("alerts.log"));
        }
        let rc = (lib.Init_FileQueue)(&mut fq, t_init, flags);
        let mut fqs = vec![snap_fq(&fq)];
        let mut alerts = Vec::new();
        for _ in 0..rounds {
            let a = (lib.Read_FileMon)(&mut fq, t_read, timeout);
            alerts.push(snap_and_free(lib, a));
            fqs.push(snap_fq(&fq));
        }
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
        (rc, fqs, alerts)
    }
}

fn diff_pipeline(
    tag: &str,
    dir: &std::path::Path,
    t_init: &tm,
    t_read: &tm,
    flags: c_int,
    supply_fp: bool,
    timeout: c_uint,
    rounds: usize,
) -> Vec<AlertSnap> {
    let p = libs();
    let _cwd = enter_dir(dir);
    unsafe {
        let c = init_then_read(&p.c, dir, t_init, t_read, flags, supply_fp, timeout, rounds);
        let r = init_then_read(&p.rs, dir, t_init, t_read, flags, supply_fp, timeout, rounds);
        assert!(
            c == r,
            "DIVERGENCE in Init_FileQueue+Read_FileMon [{tag}] flags={flags:#x} timeout={timeout}\n  C   : {c:#?}\n  Rust: {r:#?}"
        );
        c.2
    }
}

/// Row 33 — one alert, `CRALERT_READ_ALL`, `timeout = 0`.
#[test]
fn row33_read_filemon_single() {
    let sc = Scratch::new("read33");
    sc.write("alerts.log", sample_alerts(1).as_bytes());
    let t = tm {
        tm_mday: 13,
        tm_mon: 3,
        tm_year: 106,
        ..Default::default()
    };
    let alerts = diff_pipeline(
        "single",
        &sc.dir,
        &t,
        &t,
        CRALERT_READ_ALL,
        false,
        0,
        1,
    );
    assert!(matches!(alerts[0], AlertSnap::Some { .. }));
}

/// Row 34 — many alerts, read repeatedly until exhaustion.
#[test]
fn row34_read_filemon_sequence() {
    for n in [2usize, 5, 9] {
        let sc = Scratch::new("read34");
        sc.write("alerts.log", sample_alerts(n).as_bytes());
        let t = tm {
            tm_mday: 1,
            tm_mon: 5,
            tm_year: 110,
            ..Default::default()
        };
        let alerts = diff_pipeline(
            &format!("seq{n}"),
            &sc.dir,
            &t,
            &t,
            CRALERT_READ_ALL,
            false,
            0,
            n + 1,
        );
        for i in 0..n {
            assert!(
                matches!(alerts[i], AlertSnap::Some { .. }),
                "alert {i} of {n} should parse"
            );
        }
        assert_eq!(alerts[n], AlertSnap::Null);
    }
}

/// Row 35 — `CRALERT_READ_ALL | CRALERT_MAIL_SET` over mixed mail/non-mail.
#[test]
fn row35_read_filemon_mail_filter() {
    let mut mail = AlertSpec::minimal();
    mail.kind = "mail".into();
    let mut exec = AlertSpec::minimal();
    exec.kind = "exec".into();
    exec.id = "1500000001.9".into();

    let sc = Scratch::new("read35");
    sc.write(
        "alerts.log",
        render_all(&[exec.clone(), mail.clone(), exec, mail]).as_bytes(),
    );
    let t = tm {
        tm_mday: 3,
        tm_mon: 2,
        tm_year: 120,
        ..Default::default()
    };
    for flags in [
        CRALERT_READ_ALL,
        CRALERT_READ_ALL | CRALERT_MAIL_SET,
        CRALERT_READ_ALL | ALL_FLAGS & !CRALERT_FP_SET,
    ] {
        diff_pipeline("mail-filter", &sc.dir, &t, &t, flags, false, 0, 5);
    }
}

/// Row 36 — `CRALERT_FP_SET | CRALERT_READ_ALL` with a caller-supplied `fp`.
#[test]
fn row36_read_filemon_fp_set() {
    let sc = Scratch::new("read36");
    sc.write("alerts.log", sample_alerts(3).as_bytes());
    let t = tm {
        tm_mday: 8,
        tm_mon: 7,
        tm_year: 105,
        ..Default::default()
    };
    let alerts = diff_pipeline(
        "fp-set",
        &sc.dir,
        &t,
        &t,
        CRALERT_FP_SET | CRALERT_READ_ALL,
        true,
        0,
        4,
    );
    assert!(matches!(alerts[0], AlertSnap::Some { .. }));
}

/// Row 37 — the `tm` handed to `Read_FileMon` differs from the `Init` one, so
/// the re-stamping of `day` / `year` / `mon` on the NULL path is observable.
#[test]
fn row37_read_filemon_different_tm() {
    let sc = Scratch::new("read37");
    // Empty queue -> `GetAlertData` returns NULL -> the re-stamp runs.
    sc.write("alerts.log", b"");
    let t_init = tm {
        tm_mday: 1,
        tm_mon: 0,
        tm_year: 70,
        ..Default::default()
    };
    for mon in 0..12 {
        let t_read = tm {
            tm_mday: 28,
            tm_mon: mon,
            tm_year: 122,
            ..Default::default()
        };
        diff_pipeline(
            "diff-tm",
            &sc.dir,
            &t_init,
            &t_read,
            CRALERT_READ_ALL,
            false,
            0,
            1,
        );
    }
}

/// Row 38 — `timeout = 1` on an exhausted queue: one retry and one 5 s
/// `file_sleep` per library.
#[test]
fn row38_read_filemon_timeout_one() {
    let sc = Scratch::new("read38");
    sc.write("alerts.log", b"no alerts here\n");
    let t = tm {
        tm_mday: 4,
        tm_mon: 4,
        tm_year: 111,
        ..Default::default()
    };
    let alerts = diff_pipeline(
        "timeout1",
        &sc.dir,
        &t,
        &t,
        CRALERT_READ_ALL,
        false,
        1,
        1,
    );
    assert_eq!(alerts[0], AlertSnap::Null);
}

/// Row 45 — the whole `Init_FileQueue` → `Read_FileMon` → `FreeAlertData`
/// pipeline across randomized flag / content / `tm` combinations, with the
/// `file_queue` compared after every step.
#[test]
fn row45_pipeline_matrix() {
    let mut rng = Rng::new(0xF00D_0045);

    let contents: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"garbage\n".to_vec(),
        sample_alerts(1).into_bytes(),
        sample_alerts(4).into_bytes(),
        {
            let mut a = AlertSpec::minimal();
            a.group = "syscheck".into();
            a.body = vec!["Integrity checksum changed for: '/etc/hosts'".into()];
            a.render().into_bytes()
        },
        {
            // Truncated final alert.
            let s = sample_alerts(3);
            s.as_bytes()[..s.len() - 20].to_vec()
        },
    ];

    for (ci, content) in contents.iter().enumerate() {
        let sc = Scratch::new("pipe45");
        sc.write("alerts.log", content);
        // Every flag combination, deterministically (not sampled).
        for base in [
            CRALERT_READ_ALL,
            CRALERT_READ_ALL | CRALERT_MAIL_SET,
            CRALERT_READ_ALL | CRALERT_EXEC_SET | CRALERT_READ_FAILED,
            CRALERT_FP_SET | CRALERT_READ_ALL,
            CRALERT_FP_SET | CRALERT_READ_ALL | CRALERT_MAIL_SET,
            0,
            CRALERT_MAIL_SET,
        ] {
            let supply = base & CRALERT_FP_SET != 0;
            // Under CRALERT_FP_SET the re-`Handle_Queue(fileq, 0)` tries to
            // `fopen("<stdin>")`, which always fails and costs a 5 s
            // `file_sleep` per library. Keep those rows to a single round.
            let rounds = if supply { 1 } else { 1 + rng.below(4) };
            // `timeout` is kept at 0: any larger value costs 5 s per retry per
            // library and exercises the same loop (row 38 covers timeout = 1).
            let t_init = tm {
                tm_mday: rng.i32(),
                tm_mon: rng.below(12) as c_int,
                tm_year: rng.i32(),
                ..Default::default()
            };
            let t_read = tm {
                tm_mday: rng.i32(),
                tm_mon: rng.below(12) as c_int,
                tm_year: rng.i32(),
                ..Default::default()
            };
            diff_pipeline(
                &format!("matrix-c{ci}-f{base:#x}"),
                &sc.dir,
                &t_init,
                &t_read,
                base,
                supply,
                0,
                rounds,
            );
        }
    }
}

/// CONFIGS.md row 46 — the `while (i < timeout)` retry loop actually SUCCEEDING.
///
/// This is the one `Read_FileMon` path no other row reaches: the first
/// `GetAlertData` must fail, the re-`Handle_Queue` must succeed (re-opening and
/// seeking to EOF), the loop's first attempt must also fail, and only then does
/// data appear. An appender thread writes a complete alert while the library is
/// inside its 5 s `file_sleep`, so the second loop iteration returns it.
#[test]
fn row46_read_filemon_retry_loop_succeeds() {
    let p = libs();
    let t = tm {
        tm_mday: 11,
        tm_mon: 10,
        tm_year: 124,
        ..Default::default()
    };

    let sc = Scratch::new("read46");
    let file = sc.path("alerts.log");
    let alert = sample_alerts(1);

    unsafe {
        let run = |lib: &Lib| {
            std::fs::write(&file, b"junk line, not an alert\n").unwrap();
            let _cwd = enter_dir(&sc.dir);

            let mut fq = file_queue::default();
            let rc = (lib.Init_FileQueue)(&mut fq, &t, CRALERT_READ_ALL);

            // Append while the library sits in file_sleep (FQ_TIMEOUT = 5 s),
            // i.e. after the loop's first attempt has already failed.
            let f2 = file.clone();
            let payload = alert.clone();
            let appender = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                use std::io::Write;
                let mut fh = std::fs::OpenOptions::new().append(true).open(&f2).unwrap();
                fh.write_all(payload.as_bytes()).unwrap();
                fh.flush().unwrap();
            });

            let a = (lib.Read_FileMon)(&mut fq, &t, 3);
            appender.join().unwrap();
            let snap = snap_and_free(lib, a);
            let fqs = snap_fq(&fq);
            if !fq.fp.is_null() {
                fclose(fq.fp);
            }
            (rc, snap, fqs)
        };

        let c = run(&p.c);
        let r = run(&p.rs);
        // The file is rewritten between the two runs, so inode/mtime differ for
        // reasons unrelated to the code under test.
        assert!(
            (c.0, c.1.clone(), c.2.clone().masked()) == (r.0, r.1.clone(), r.2.clone().masked()),
            "DIVERGENCE in Read_FileMon retry loop\n  C   : {:#?}\n  Rust: {:#?}",
            (c.0, &c.1, c.2.clone().masked()),
            (r.0, &r.1, r.2.masked())
        );
        assert!(
            matches!(c.1, AlertSnap::Some { .. }),
            "the retry loop was expected to pick up the appended alert, got {:?}",
            c.1
        );
    }
}
