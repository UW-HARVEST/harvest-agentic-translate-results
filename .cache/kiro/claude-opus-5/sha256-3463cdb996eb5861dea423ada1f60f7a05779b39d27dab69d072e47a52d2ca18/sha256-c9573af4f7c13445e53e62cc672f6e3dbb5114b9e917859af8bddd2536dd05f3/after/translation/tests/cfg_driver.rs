//! CONFIGS.md row C13 — the `driver` one-shot wrapper, randomized over its five
//! parameters and over randomized `alerts.log` contents.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;
const READ_ALL: c_int = 0x004;
const FP_SET: c_int = 0x010;

fn drive(lib: &Lib, day: c_int, month: c_int, year: c_int, timeout: u32, flags: c_int)
-> (Option<AlertSnap>, Vec<u8>) {
    capture_stderr(|| unsafe {
        set_errno(0);
        let a = (lib.driver)(day, month, year, timeout, flags);
        let s = snap_alert(a);
        if !a.is_null() {
            (lib.free_alert_data)(a);
        }
        s
    })
}

fn assert_driver_eq(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: u32,
    flags: c_int,
    what: &str,
) -> Option<AlertSnap> {
    let (c, r) = libs();
    let a = drive(c, day, month, year, timeout, flags);
    let b = drive(r, day, month, year, timeout, flags);
    assert_eq!(a.0, b.0, "[{what}] driver result differs");
    assert_eq!(
        String::from_utf8_lossy(&a.1),
        String::from_utf8_lossy(&b.1),
        "[{what}] stderr differs"
    );
    a.0
}

/// C13 — `driver` with `READ_ALL` (the only flag combination under which it can
/// ever return an alert), randomized over day / month / year and over the
/// contents of `alerts.log`.
#[test]
fn c13_driver_read_all() {
    let g = world();
    let mut rng = Rng::new(0xC13);

    // Canonical case first, with the expected values pinned.
    write_alerts_log(MINIMAL.as_bytes());
    let a = assert_driver_eq(19, 3, 116, 0, READ_ALL, "C13 canonical")
        .expect("driver must return the alert");
    assert_eq!(a.rule, 1002);
    assert_eq!(a.level, 7);
    assert_eq!(a.alertid.as_deref(), Some(&b"1461102540.1234"[..]));
    assert_eq!(a.group.as_deref(), Some(&b"syslog,errors,"[..]));

    for n in 0..120 {
        // Randomized alerts.log
        let nalerts = rng.below(4);
        let mut content = Vec::new();
        if rng.below(4) == 0 {
            content.extend_from_slice(b"leading noise\n");
        }
        for k in 0..nalerts {
            let group = *rng.pick(&["syslog,", "ossec,syscheck,", "errors,", ""]);
            let body: Vec<&str> = vec![
                *rng.pick(&[
                    "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
                    "Rule: 1002 (level 3) -> 'Unknown problem.'",
                    "Rule: 5716 (level 5) -> 'SSHD authentication failed.'",
                    "not a rule line at all",
                ]),
                *rng.pick(&[
                    "Src IP: 10.0.0.1",
                    "Src Port: 4242",
                    "Dst IP: 10.0.0.2",
                    "Dst Port: 22",
                    "User: root",
                    "Integrity checksum changed for: '/etc/passwd'",
                    "free-form",
                ]),
            ];
            content.extend_from_slice(&alert_block(
                &format!("146110254{k}.{n}"),
                group,
                &format!("2016 Apr 19 20:29:0{} h{k}->/var/log/m{k}", k % 10),
                &body,
            ));
        }
        write_alerts_log(&content);

        // Randomized parameters. tm_mon stays in 0..11 (see ERRORS.md G7) and
        // timeout stays 0 so the suite never pays a 5 s file_sleep here.
        let day = rng.i32();
        let month = rng.below(12) as c_int;
        let year = rng.i32();
        let inert = rng.i32() & !(MAIL | READ_ALL | FP_SET);
        let flags = READ_ALL | inert | if rng.bool() { MAIL } else { 0 };
        assert_driver_eq(
            day,
            month,
            year,
            0,
            flags,
            &format!("C13 random#{n} day={day} mon={month} year={year} flags={flags:#x}"),
        );
    }
    drop(g);
}

/// C13 — every documented flag combination through `driver`, with `timeout = 0`.
///
/// Combinations WITHOUT `READ_ALL` seek to the end of `alerts.log` and then
/// re-`Handle_Queue` with flags 0, so they never find anything; combinations
/// WITH `FP_SET` derive the file name `<stdin>`, which cannot be opened, so
/// `Read_FileMon` pays one `file_sleep()` (5 s per implementation). Those are
/// covered here at one representative each to stay inside the time budget.
#[test]
fn c13_driver_flag_matrix() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());

    // Cheap half: no FP_SET => no <stdin> => no sleep.
    for flags in [0, MAIL, READ_ALL, MAIL | READ_ALL, 0x002, 0x008, 0x00A] {
        assert_driver_eq(19, 3, 116, 0, flags, &format!("C13 matrix flags={flags:#x}"));
    }

    // FP_SET representatives (these sleep).
    for flags in [FP_SET, FP_SET | READ_ALL] {
        let a = assert_driver_eq(19, 3, 116, 0, flags, &format!("C13 matrix flags={flags:#x}"));
        assert!(
            a.is_none(),
            "C13: FP_SET makes driver look for '<stdin>', which cannot be opened"
        );
    }
    drop(g);
}

/// C13 — `driver` when `alerts.log` is missing (pays one `file_sleep()` each).
#[test]
fn c13_driver_missing_file() {
    let g = world();
    remove_alerts_log();
    let a = assert_driver_eq(19, 3, 116, 0, READ_ALL, "C13 missing alerts.log");
    assert!(a.is_none());
    drop(g);
}

/// C13 — `driver` month boundary values across all twelve valid `s_month`
/// indices, each with a parseable file so the alert comes back immediately.
#[test]
fn c13_driver_all_months() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    for month in 0..12 {
        let a = assert_driver_eq(1, month, 0, 0, READ_ALL, &format!("C13 month={month}"));
        assert!(a.is_some(), "C13 month={month} must still parse");
    }
    drop(g);
}
