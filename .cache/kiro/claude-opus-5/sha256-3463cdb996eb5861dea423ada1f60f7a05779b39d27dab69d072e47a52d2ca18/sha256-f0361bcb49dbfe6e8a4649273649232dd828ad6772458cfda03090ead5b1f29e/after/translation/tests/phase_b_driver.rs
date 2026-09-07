//! Phase B — CONFIGS.md rows 39..44: the `driver` one-shot wrapper, end to end
//! through both `.so`s.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::{c_int, c_uint};

const ALL_FLAGS: c_int =
    CRALERT_MAIL_SET | CRALERT_EXEC_SET | CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_FP_SET;

fn sample_alerts(n: usize) -> String {
    let specs: Vec<AlertSpec> = (0..n)
        .map(|i| {
            let mut a = AlertSpec::minimal();
            a.id = format!("15000000{:02}.{}", i, i * 3);
            a.group = format!("g{i},syscheck");
            a.location = format!("/var/log/d{i}.log");
            a.rule = Some(format!("Rule: {} (level {}) -> 'c{i}'", 500 + i, i % 16));
            a.srcip = Some(format!("Src IP: 172.16.0.{}", i % 256));
            a.dstport = Some(format!("Dst Port: {}", 20 + i));
            a.body = vec![format!("Integrity checksum changed for: '/etc/x{i}'")];
            a
        })
        .collect();
    render_all(&specs)
}

/// Call `driver` on both libraries in the same directory with the same
/// arguments and compare the returned alert plus everything written to stderr.
fn diff_driver(
    tag: &str,
    dir: &std::path::Path,
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> AlertSnap {
    let p = libs();
    let _cwd = enter_dir(dir);
    unsafe {
        let (c, c_err) = capture_stderr(|| {
            snap_and_free(&p.c, (p.c.driver)(day, month, year, timeout, flags))
        });
        let (r, r_err) = capture_stderr(|| {
            snap_and_free(&p.rs, (p.rs.driver)(day, month, year, timeout, flags))
        });
        assert!(
            c == r,
            "DIVERGENCE in driver [{tag}] day={day} month={month} year={year} \
             timeout={timeout} flags={flags:#x}\n  C   : {c:#?}\n  Rust: {r:#?}"
        );
        assert!(
            c_err == r_err,
            "DIVERGENCE in driver stderr [{tag}] flags={flags:#x}\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_err),
            String::from_utf8_lossy(&r_err)
        );
        c
    }
}

/// Row 39 — `CRALERT_READ_ALL`, `timeout = 0`, one full alert.
#[test]
fn row39_driver_single_alert() {
    let sc = Scratch::new("drv39");
    sc.write("alerts.log", sample_alerts(1).as_bytes());
    let got = diff_driver("single", &sc.dir, 13, 3, 106, 0, CRALERT_READ_ALL);
    match got {
        AlertSnap::Some {
            rule,
            level,
            ref group,
            ref filename,
            ..
        } => {
            assert_eq!(rule, 500);
            assert_eq!(level, 0);
            assert_eq!(group.as_deref(), Some(&b"g0,syscheck"[..]));
            assert_eq!(filename.as_deref(), Some(&b"/etc/x0"[..]));
        }
        AlertSnap::Null => panic!("expected an alert"),
    }
}

/// Row 40 — `CRALERT_READ_ALL | CRALERT_MAIL_SET` over mail and non-mail.
#[test]
fn row40_driver_mail_filter() {
    let mut mail = AlertSpec::minimal();
    mail.kind = "mail".into();
    let mut exec = AlertSpec::minimal();
    exec.kind = "exec".into();

    for (tag, specs) in [
        ("mail-first", vec![mail.clone(), exec.clone()]),
        ("exec-first", vec![exec.clone(), mail.clone()]),
        ("exec-only", vec![exec.clone(), exec.clone()]),
        ("mail-only", vec![mail.clone(), mail.clone()]),
    ] {
        let sc = Scratch::new("drv40");
        sc.write("alerts.log", render_all(&specs).as_bytes());
        for flags in [CRALERT_READ_ALL, CRALERT_READ_ALL | CRALERT_MAIL_SET] {
            diff_driver(tag, &sc.dir, 1, 0, 100, 0, flags);
        }
    }
}

/// Row 41 — `flags = 0`: `Init_FileQueue` seeks to EOF, so nothing is readable.
#[test]
fn row41_driver_seek_to_end() {
    let sc = Scratch::new("drv41");
    sc.write("alerts.log", sample_alerts(3).as_bytes());
    let got = diff_driver("seek-end", &sc.dir, 13, 3, 106, 0, 0);
    assert_eq!(got, AlertSnap::Null);
}

/// Row 42 — randomized `day` / `month` (0..11) / `year`.
#[test]
fn row42_driver_random_dates() {
    let sc = Scratch::new("drv42");
    sc.write("alerts.log", sample_alerts(2).as_bytes());
    let mut rng = Rng::new(0xDEAD_0042);

    let mut cases: Vec<(c_int, c_int, c_int)> = vec![
        (0, 0, 0),
        (-1, 11, -1),
        (i32::MAX, 0, i32::MAX),
        (i32::MIN, 11, i32::MIN),
        (31, 6, 1900),
        (1, 0, -1900),
    ];
    for _ in 0..150 {
        cases.push((rng.i32(), rng.below(12) as c_int, rng.i32()));
    }
    for (d, m, y) in cases {
        diff_driver("rand-date", &sc.dir, d, m, y, 0, CRALERT_READ_ALL);
    }
}

/// Row 43 — randomized full flag bitmasks, including undefined bits.
///
/// `CRALERT_FP_SET` is included: `driver` memsets its `file_queue` to zero, so
/// `fp` is NULL and the FP_SET path takes the "queue not available" route. That
/// costs a 5 s `file_sleep` per library, so those masks are enumerated once
/// rather than sampled repeatedly.
#[test]
fn row43_driver_flag_masks() {
    let sc = Scratch::new("drv43");
    sc.write("alerts.log", sample_alerts(2).as_bytes());

    // Cheap masks (no CRALERT_FP_SET) — sampled broadly.
    let mut rng = Rng::new(0xDEAD_0043);
    let mut cheap: Vec<c_int> = vec![
        0,
        CRALERT_MAIL_SET,
        CRALERT_EXEC_SET,
        CRALERT_READ_ALL,
        CRALERT_READ_FAILED,
        CRALERT_READ_ALL | CRALERT_MAIL_SET,
        CRALERT_READ_ALL | CRALERT_EXEC_SET | CRALERT_READ_FAILED,
        ALL_FLAGS & !CRALERT_FP_SET,
        0x20,
        0x7FFF_FFFF & !CRALERT_FP_SET,
    ];
    for _ in 0..60 {
        cheap.push(rng.i32() & !CRALERT_FP_SET);
    }
    for flags in cheap {
        diff_driver("mask", &sc.dir, 13, 3, 106, 0, flags);
    }

    // Masks that include CRALERT_FP_SET (one 5 s sleep per library each).
    for flags in [
        CRALERT_FP_SET,
        CRALERT_FP_SET | CRALERT_READ_ALL,
        ALL_FLAGS,
        -1,
        i32::MIN | CRALERT_FP_SET,
    ] {
        diff_driver("mask-fp", &sc.dir, 13, 3, 106, 0, flags);
    }
}

/// Row 44 — randomized whole alert files through the full `driver` pipeline.
#[test]
fn row44_driver_random_files() {
    let mut rng = Rng::new(0xDEAD_0044);
    let kinds = ["mail", "exec", "mailx", "-"];
    let groups = ["syslog", "syscheck", "ossec,syscheck,", "", "pci_dss"];

    for iter in 0..200 {
        let n = rng.below(4);
        let mut text = String::new();
        for i in 0..n {
            let mut a = AlertSpec::minimal();
            a.id = format!("{}.{}", 1500000000u64 + rng.below(9999) as u64, i);
            a.kind = (*rng.choice(&kinds)).into();
            a.group = (*rng.choice(&groups)).into();
            a.location = format!(
                "/var/log/{}",
                String::from_utf8_lossy(&rng.token_upto(10)).replace([' ', '\t'], "_")
            );
            if rng.bool() {
                a.rule = Some(format!(
                    "Rule: {} (level {}) -> '{}'",
                    rng.i32(),
                    rng.below(17),
                    String::from_utf8_lossy(&rng.token_upto(16)).replace('\'', "q")
                ));
            }
            if rng.bool() {
                a.srcip = Some(format!("Src IP: {}", String::from_utf8_lossy(&rng.token_upto(16))));
            }
            if rng.bool() {
                a.srcport = Some(format!("Src Port: {}", rng.i32()));
            }
            if rng.bool() {
                a.dstip = Some(format!("Dst IP: {}", String::from_utf8_lossy(&rng.token_upto(16))));
            }
            if rng.bool() {
                a.dstport = Some(format!("Dst Port: {}", rng.i32()));
            }
            if rng.bool() {
                a.user = Some(format!("User: {}", String::from_utf8_lossy(&rng.token_upto(10))));
            }
            for _ in 0..rng.below(3) {
                let mut line = String::from_utf8_lossy(&rng.token_upto(50)).into_owned();
                if rng.below(5) == 0 {
                    line = format!("Integrity checksum changed for: '{line}'");
                }
                a.body.push(line);
            }
            text.push_str(&a.render());
        }
        if rng.below(4) == 0 {
            while text.ends_with('\n') {
                text.pop();
            }
        }

        let sc = Scratch::new("drv44");
        sc.write("alerts.log", text.as_bytes());
        let flags = *rng.choice(&[
            CRALERT_READ_ALL,
            CRALERT_READ_ALL | CRALERT_MAIL_SET,
            CRALERT_READ_ALL | CRALERT_EXEC_SET,
            CRALERT_READ_ALL | CRALERT_READ_FAILED | CRALERT_MAIL_SET,
        ]);
        diff_driver(
            &format!("rand-file{iter}"),
            &sc.dir,
            rng.i32(),
            rng.below(12) as c_int,
            rng.i32(),
            0,
            flags,
        );
    }
}
