//! Phase B rows 32-38 and Phase C rows 26 / 31 / 32: the `driver()` one-shot
//! entry point, driven through both `.so`s.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

extern "C" {
    fn mkfifo(path: *const c_char, mode: u32) -> c_int;
}

/* =================== CONFIGS row 32 =================== */

#[test]
fn cfg32_driver_read_all_random() {
    let _g = guard();
    let mut rng = Rng::new(0x3232);
    unsafe {
        for case in 0..300 {
            let content = random_stream(&mut rng);
            write_file(ALERTS_DAILY, &content);
            let day = rng.range_i32(-40, 40);
            let mon = rng.range_i32(0, 11);
            let year = rng.range_i32(-200, 300);
            let flags = CRALERT_READ_ALL
                | if rng.bool() { CRALERT_MAIL_SET } else { 0 }
                | if rng.bool() { CRALERT_EXEC_SET } else { 0 }
                | if rng.bool() { CRALERT_READ_FAILED } else { 0 };
            diff_driver(day, mon, year, 0, flags, &format!("driver fuzz {case}"));
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 33 =================== */

#[test]
fn cfg33_driver_flags0() {
    let _g = guard();
    unsafe {
        for body in [
            String::new(),
            "no alert\n".to_string(),
            full_alert("50.1", true, "syscheck,"),
        ] {
            write_file(ALERTS_DAILY, body.as_bytes());
            for (d, m, y) in [(1, 0, 100), (31, 11, 0), (15, 5, 199)] {
                // flags==0 seeks to EOF, so nothing is ever read
                diff_driver(d, m, y, 0, 0, "driver flags0");
            }
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 34 =================== */

#[test]
fn cfg34_driver_mail_filter() {
    let _g = guard();
    unsafe {
        let corpora = [
            ("mail-only", full_alert("51.1", true, "ossec,")),
            ("nomail-only", full_alert("51.2", false, "ossec,")),
            (
                "mail-then-nomail",
                format!(
                    "{}{}",
                    full_alert("51.3", true, "syscheck,"),
                    full_alert("51.4", false, "ossec,")
                ),
            ),
            (
                "nomail-then-mail",
                format!(
                    "{}{}",
                    full_alert("51.5", false, "ossec,"),
                    full_alert("51.6", true, "syscheck,")
                ),
            ),
        ];
        for (label, body) in corpora {
            write_file(ALERTS_DAILY, body.as_bytes());
            for flags in [
                CRALERT_READ_ALL,
                CRALERT_READ_ALL | CRALERT_MAIL_SET,
                CRALERT_READ_ALL | CRALERT_MAIL_SET | CRALERT_EXEC_SET,
            ] {
                diff_driver(4, 3, 105, 0, flags, label);
            }
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 35 =================== */

#[test]
fn cfg35_driver_fp_set() {
    let _g = guard();
    unsafe {
        let body = full_alert("52.1", true, "syscheck,");
        write_file(STDIN_NAME, body.as_bytes());
        write_file(ALERTS_DAILY, body.as_bytes());
        for flags in [
            CRALERT_FP_SET,
            CRALERT_FP_SET | CRALERT_READ_ALL,
            CRALERT_FP_SET | CRALERT_MAIL_SET,
            CRALERT_FP_SET | CRALERT_READ_ALL | CRALERT_MAIL_SET,
        ] {
            diff_driver(6, 6, 106, 0, flags, "driver fp_set");
        }
        remove_file(STDIN_NAME);
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 36 =================== */

#[test]
fn cfg36_driver_all_flag_combos() {
    let _g = guard();
    unsafe {
        let body = format!(
            "{}{}",
            full_alert("53.1", true, "syscheck,"),
            full_alert("53.2", false, "ossec,")
        );
        // Both possible queue names exist so that no code path can hit
        // file_sleep() (which would cost FQ_TIMEOUT = 5 s per call).
        write_file(ALERTS_DAILY, body.as_bytes());
        write_file(STDIN_NAME, body.as_bytes());
        for flags in 0..32 {
            diff_driver(9, (flags % 12) as c_int, 111, 0, flags, "flag combo");
        }
        // and with only the "<stdin>" name present
        remove_file(ALERTS_DAILY);
        for flags in [CRALERT_FP_SET, CRALERT_FP_SET | CRALERT_READ_ALL] {
            diff_driver(9, 1, 111, 0, flags, "flag combo, no alerts.log");
        }
        remove_file(STDIN_NAME);
    }
}

/* =================== CONFIGS row 37 =================== */

#[test]
fn cfg37_driver_degenerate_queue_files() {
    let _g = guard();
    use std::os::unix::fs::PermissionsExt;
    unsafe {
        // empty file: no sleeping, just no alert
        write_file(ALERTS_DAILY, b"");
        diff_driver(1, 0, 100, 0, CRALERT_READ_ALL, "empty queue file");
        diff_driver(1, 0, 100, 0, 0, "empty queue file, flags0");

        // a file containing only a NUL byte / binary junk
        write_file(ALERTS_DAILY, &[0u8, 1, 2, 3, b'\n', 0xff, b'\n']);
        diff_driver(1, 0, 100, 0, CRALERT_READ_ALL, "binary queue file");

        // absent (costs 2 x FQ_TIMEOUT)
        remove_file(ALERTS_DAILY);
        let start = std::time::Instant::now();
        diff_driver(1, 0, 100, 0, CRALERT_READ_ALL, "absent queue file");
        assert!(start.elapsed().as_secs() >= 5, "expected file_sleep()");

        // present but unreadable (costs 2 x FQ_TIMEOUT); skipped as root
        if libc_geteuid() != 0 {
            write_file(ALERTS_DAILY, b"whatever\n");
            std::fs::set_permissions(ALERTS_DAILY, std::fs::Permissions::from_mode(0o000)).unwrap();
            diff_driver(1, 0, 100, 0, CRALERT_READ_ALL, "unreadable queue file");
            std::fs::set_permissions(ALERTS_DAILY, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        remove_file(ALERTS_DAILY);

        // a directory named alerts.log: fopen succeeds on Linux, so this checks
        // the fseek/fstat/fgets behaviour on a directory stream
        let _ = std::fs::remove_dir_all(ALERTS_DAILY);
        std::fs::create_dir(ALERTS_DAILY).unwrap();
        diff_driver(1, 0, 100, 0, CRALERT_READ_ALL, "directory queue file");
        diff_driver(1, 0, 100, 0, 0, "directory queue file, flags0");
        std::fs::remove_dir_all(ALERTS_DAILY).unwrap();
    }
}

/* =================== CONFIGS row 38 =================== */

#[test]
fn cfg38_driver_extreme_day_year() {
    let _g = guard();
    let mut rng = Rng::new(0x3838);
    unsafe {
        write_file(ALERTS_DAILY, full_alert("54.1", true, "syscheck,").as_bytes());
        let extremes = [
            i32::MIN,
            i32::MIN + 1,
            -1900,
            -1,
            0,
            1,
            i32::MAX - 1900,
            i32::MAX,
        ];
        for &d in &extremes {
            for &y in &extremes {
                diff_driver(d, 0, y, 0, CRALERT_READ_ALL, "extreme day/year");
            }
        }
        for _ in 0..100 {
            diff_driver(
                rng.i32(),
                rng.range_i32(0, 11),
                rng.i32(),
                0,
                CRALERT_READ_ALL,
                "random day/year",
            );
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== ERRORS row 26 =================== */

#[test]
fn err26_driver_init_failure() {
    let _g = guard();
    unsafe {
        remove_file(ALERTS_DAILY);
        // A FIFO named alerts.log: fopen("r") succeeds (we hold it open O_RDWR
        // so there is a writer), but fseek(fp, 0, SEEK_END) fails with ESPIPE,
        // so Handle_Queue -> -1, Init_FileQueue -> -1 and driver must print
        // "File queue initialization failed" and return NULL.
        let p = cpath(ALERTS_DAILY);
        assert_eq!(mkfifo(p.as_ptr(), 0o600), 0, "mkfifo failed");
        let holder = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(ALERTS_DAILY)
            .unwrap();

        // flags==0 => Handle_Queue does the fseek and fails.
        diff_driver(1, 0, 100, 0, 0, "driver init failure (fifo)");

        let (cres, cerr) = capture_stderr(|| {
            let ptr = (cc().driver)(1, 0, 100, 0, 0);
            let s = snap_alert(ptr);
            if !ptr.is_null() {
                (cc().FreeAlertData)(ptr);
            }
            s
        });
        assert_eq!(cres, None, "driver must return NULL when Init fails");
        let cerr = String::from_utf8_lossy(&cerr).to_string();
        assert!(
            cerr.contains("(1116): Could not set position in file 'alerts.log'"),
            "expected FSEEK_ERROR, got {cerr:?}"
        );
        assert!(
            cerr.contains("File queue initialization failed"),
            "expected the driver message, got {cerr:?}"
        );

        drop(holder);
        remove_file(ALERTS_DAILY);
    }
}

/* =================== ERRORS row 31 =================== */

#[test]
fn err31_undefined_flag_bits() {
    let _g = guard();
    let mut rng = Rng::new(0x3131_3131);
    unsafe {
        let body = format!(
            "{}{}",
            full_alert("55.1", true, "syscheck,"),
            full_alert("55.2", false, "ossec,")
        );
        write_file(ALERTS_DAILY, body.as_bytes());
        write_file(STDIN_NAME, body.as_bytes());
        // Any int is a legal `flags` value across the FFI boundary; only the
        // five defined bits may change behaviour.
        for flags in [
            i32::MIN,
            i32::MAX,
            -1,
            0x20,
            0x40,
            0x7fff_ffe0,
            !0x1f,
            i32::MIN | CRALERT_READ_ALL,
            i32::MAX & !CRALERT_FP_SET,
        ] {
            diff_driver(2, 2, 102, 0, flags, &format!("undefined bits {flags:#x}"));
        }
        for _ in 0..120 {
            let flags = rng.i32();
            diff_driver(2, 2, 102, 0, flags, &format!("random flags {flags:#x}"));
        }
        remove_file(ALERTS_DAILY);
        remove_file(STDIN_NAME);
    }
}

/* =================== ERRORS row 32 =================== */

#[test]
fn err32_driver_timeout_extremes() {
    let _g = guard();
    unsafe {
        // An alert IS available, so the timeout loop is never entered and even
        // u32::MAX costs nothing.
        write_file(ALERTS_DAILY, full_alert("56.1", true, "syscheck,").as_bytes());
        for timeout in [0u32, 1, 2, 1000, u32::MAX] {
            diff_driver(1, 0, 100, timeout, CRALERT_READ_ALL, "timeout with hit");
        }
        // A miss with timeout == 1 must go round the loop exactly once, i.e.
        // sleep FQ_TIMEOUT seconds and then return NULL.
        let start = std::time::Instant::now();
        diff_driver(1, 0, 100, 1, 0, "timeout 1 with miss");
        let el = start.elapsed().as_secs();
        assert!((10..30).contains(&el), "expected ~2x5s of file_sleep, got {el}s");
        remove_file(ALERTS_DAILY);
    }
}

/* =================== worker (unused) =================== */

#[test]
fn zz_subprocess_worker() {
    let Some(action) = worker_action() else {
        return;
    };
    panic!("unknown worker action {action:?}");
}

extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}
