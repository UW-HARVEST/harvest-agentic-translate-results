//! CONFIGS.md rows C04–C12 — `Init_FileQueue` and `Read_FileMon` driven
//! directly (not through the `driver` wrapper), across the full flag matrix and
//! as a composed pipeline.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;
const EXEC: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;

/// `Init_FileQueue` on a zeroed queue with an optional caller-supplied `fp`.
fn init_snap(
    lib: &Lib,
    flags: c_int,
    t: &tm,
    preset_fp: Option<*mut FILE>,
) -> (c_int, QueueSnap, Vec<u8>) {
    let mut fq = file_queue::zeroed();
    if let Some(fp) = preset_fp {
        fq.fp = fp;
    }
    let (rc, err) = capture_stderr(|| {
        set_errno(0);
        unsafe { (lib.init_file_queue)(&mut fq, t, flags) }
    });
    let s = unsafe { snap_queue(&fq) };
    unsafe {
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
    }
    (rc, s, err)
}

fn assert_init_eq(
    flags: c_int,
    mk_fp: Option<&dyn Fn() -> *mut FILE>,
    what: &str,
) -> (c_int, QueueSnap) {
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);
    let a = init_snap(c, flags, &t, mk_fp.map(|f| f()));
    let b = init_snap(r, flags, &t, mk_fp.map(|f| f()));
    assert_eq!(a.0, b.0, "[{what}] rc differs");
    assert_eq!(a.1, b.1, "[{what}] queue differs");
    assert_eq!(
        String::from_utf8_lossy(&a.2),
        String::from_utf8_lossy(&b.2),
        "[{what}] stderr differs"
    );
    (a.0, a.1)
}

/// C04 — flags = 0: open `alerts.log`, seek to END, fstat.
#[test]
fn c04_init_default() {
    let g = world();
    for content in [
        b"".to_vec(),
        b"x".to_vec(),
        MINIMAL.as_bytes().to_vec(),
        vec![b'z'; 5000],
    ] {
        write_alerts_log(&content);
        let (rc, q) = assert_init_eq(0, None, &format!("C04 flags=0, {} bytes", content.len()));
        assert_eq!(rc, 0);
        assert!(!q.fp_is_null);
        assert_eq!(q.file_name, b"alerts.log".to_vec());
        assert_eq!(
            q.fp_pos,
            content.len() as i64,
            "C04: flags=0 must seek to the END of the file"
        );
        assert_eq!(q.st_size, content.len() as i64);
        assert_eq!(q.last_change, PINNED_MTIME);
        assert_eq!(q.st_mtime, PINNED_MTIME);
        assert_eq!(q.flags, 0);
    }
    drop(g);
}

/// C05 — flags = `READ_ALL`: no seek, so the stream stays at offset 0.
#[test]
fn c05_init_read_all() {
    let g = world();
    for content in [b"".to_vec(), MINIMAL.as_bytes().to_vec(), vec![b'z'; 5000]] {
        write_alerts_log(&content);
        let (rc, q) = assert_init_eq(
            READ_ALL,
            None,
            &format!("C05 READ_ALL, {} bytes", content.len()),
        );
        assert_eq!(rc, 0);
        assert!(!q.fp_is_null);
        assert_eq!(q.fp_pos, 0, "C05: READ_ALL must NOT seek to the end");
        assert_eq!(q.st_size, content.len() as i64);
        assert_eq!(q.flags, READ_ALL);
    }
    drop(g);
}

/// C06 — flags = `FP_SET` with a caller-supplied seekable `fp`: `file_name`
/// becomes `<stdin>`, the stream is adopted (not reopened) but still seeked to
/// the end, and the fstat describes the CALLER's file.
#[test]
fn c06_init_fp_set() {
    let g = world();
    write_alerts_log(b"this is the alerts.log, which must NOT be opened\n");
    let payload = MINIMAL.as_bytes().to_vec();
    let path = temp_file("c06", &payload);
    let mk = || open_r(&path);
    let (rc, q) = assert_init_eq(FP_SET, Some(&mk), "C06 FP_SET with a real fp");
    assert_eq!(rc, 0);
    assert!(!q.fp_is_null, "C06: the caller's fp is kept");
    assert_eq!(q.file_name, b"<stdin>".to_vec());
    assert_eq!(
        q.fp_pos,
        payload.len() as i64,
        "C06: FP_SET without READ_ALL still seeks to the end"
    );
    assert_eq!(
        q.st_size,
        payload.len() as i64,
        "C06: fstat must describe the adopted stream, not alerts.log"
    );
    assert_eq!(q.flags, FP_SET);
    drop(g);
}

/// C07 — flags = `FP_SET | READ_ALL`: neither reopen nor seek, so whatever
/// offset the caller left is preserved exactly.
#[test]
fn c07_init_fp_set_read_all() {
    let g = world();
    write_alerts_log(b"must not be opened\n");
    let payload = MINIMAL.as_bytes().to_vec();
    let path = temp_file("c07", &payload);
    let mut rng = Rng::new(0xC07);
    let mut offsets: Vec<i64> = vec![0, 1, payload.len() as i64 - 1, payload.len() as i64];
    for _ in 0..40 {
        offsets.push(rng.below(payload.len() + 32) as i64);
    }
    for off in offsets {
        let mk = || {
            let fp = open_r(&path);
            unsafe { fseek(fp, off, SEEK_SET) };
            fp
        };
        let (rc, q) = assert_init_eq(
            FP_SET | READ_ALL,
            Some(&mk),
            &format!("C07 FP_SET|READ_ALL at offset {off}"),
        );
        assert_eq!(rc, 0);
        assert_eq!(q.fp_pos, off, "C07: the caller's offset must be preserved");
        assert_eq!(q.file_name, b"<stdin>".to_vec());
        assert_eq!(q.st_size, payload.len() as i64);
    }
    drop(g);
}

/// C08 — `FP_SET | READ_ALL` with `fp == NULL`: fseek AND fstat are skipped, so
/// `last_change` comes from the still-zeroed `f_status`.
#[test]
fn c08_init_fp_set_null_fp_read_all() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let (rc, q) = assert_init_eq(FP_SET | READ_ALL, None, "C08 FP_SET|READ_ALL, NULL fp");
    assert_eq!(rc, 0);
    assert!(q.fp_is_null);
    assert_eq!(q.fp_pos, -1);
    assert_eq!(q.st_size, 0, "C08: fstat was never called");
    assert_eq!(q.last_change, 0);
    assert_eq!(q.file_name, b"<stdin>".to_vec());
    drop(g);
}

/// C09 — the undocumented bits (`EXEC_SET`, `READ_FAILED`) and every other bit
/// outside `MAIL|READ_ALL|FP_SET` must be inert for `Init_FileQueue`, except
/// that they are stored verbatim in `fileq->flags`.
#[test]
fn c09_inert_bits() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);
    let live = MAIL | READ_ALL | FP_SET;

    let mut extra: Vec<c_int> = vec![0, EXEC, READ_FAILED, EXEC | READ_FAILED, 0x20, 0x1000, !live];
    let mut rng = Rng::new(0xC09);
    for _ in 0..60 {
        extra.push(rng.i32() & !live);
    }

    // FP_SET is excluded from `base` so no caller fp is needed.
    for base in [0, MAIL, READ_ALL, MAIL | READ_ALL] {
        for &e in &extra {
            let plain = init_snap(c, base, &t, None);
            let with_extra = init_snap(c, base | e, &t, None);
            // Only `flags` itself may differ.
            let mut norm = with_extra.1.clone();
            norm.flags = plain.1.flags;
            assert_eq!(
                plain.1, norm,
                "C: inert bits {e:#x} changed behaviour for base={base:#x}"
            );
            assert_eq!(with_extra.1.flags, base | e, "flags stored verbatim");

            let rplain = init_snap(r, base, &t, None);
            let rwith = init_snap(r, base | e, &t, None);
            let mut rnorm = rwith.1.clone();
            rnorm.flags = rplain.1.flags;
            assert_eq!(
                rplain.1, rnorm,
                "RUST: inert bits {e:#x} changed behaviour for base={base:#x}"
            );
            assert_eq!(plain.1, rplain.1, "C/RUST differ for base={base:#x}");
            assert_eq!(with_extra.1, rwith.1, "C/RUST differ for {:#x}", base | e);
        }
    }
    drop(g);
}

// ---------------------------------------------------------------------------
// Composed pipeline: Init_FileQueue -> Read_FileMon (repeatedly)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Step {
    alert: Option<AlertSnap>,
    queue: QueueSnap,
}

/// Init once, then call `Read_FileMon` `n` times, snapshotting each step.
fn pipeline(lib: &Lib, flags: c_int, timeout: u32, t: &tm, n: usize) -> (c_int, Vec<Step>, Vec<u8>) {
    let mut fq = file_queue::zeroed();
    let ((rc, steps), err) = capture_stderr(|| unsafe {
        set_errno(0);
        let rc = (lib.init_file_queue)(&mut fq, t, flags);
        let mut steps = Vec::new();
        if rc == 0 {
            for _ in 0..n {
                let a = (lib.read_file_mon)(&mut fq, t, timeout);
                let snap = snap_alert(a);
                let done = a.is_null();
                if !a.is_null() {
                    (lib.free_alert_data)(a);
                }
                steps.push(Step {
                    alert: snap,
                    queue: snap_queue(&fq),
                });
                if done {
                    break;
                }
            }
        }
        (rc, steps)
    });
    unsafe {
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
    }
    (rc, steps, err)
}

fn assert_pipeline_eq(
    flags: c_int,
    timeout: u32,
    n: usize,
    setup: &dyn Fn(),
    what: &str,
) -> Vec<Step> {
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);
    setup();
    let a = pipeline(c, flags, timeout, &t, n);
    setup();
    let b = pipeline(r, flags, timeout, &t, n);
    assert_eq!(a.0, b.0, "[{what}] Init rc differs");
    assert_eq!(a.1.len(), b.1.len(), "[{what}] step count differs");
    for (i, (x, y)) in a.1.iter().zip(b.1.iter()).enumerate() {
        assert_eq!(x, y, "[{what}] step {i} differs");
    }
    assert_eq!(
        String::from_utf8_lossy(&a.2),
        String::from_utf8_lossy(&b.2),
        "[{what}] stderr differs"
    );
    a.1
}

/// C10 — full pipeline, `READ_ALL`, one complete alert: returned by the FIRST
/// `GetAlertData` inside `Read_FileMon`, so no `file_sleep` is paid.
#[test]
fn c10_pipeline_read_all_one_alert() {
    let g = world();
    let steps = assert_pipeline_eq(
        READ_ALL,
        0,
        1,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        "C10 READ_ALL one alert",
    );
    let a = steps[0].alert.as_ref().expect("must return the alert");
    assert_eq!(a.rule, 1002);
    assert_eq!(a.level, 7);
    assert_eq!(a.alertid.as_deref(), Some(&b"1461102540.1234"[..]));
    drop(g);
}

/// C11 — full pipeline over a MANY-alert file: repeated `Read_FileMon` walks the
/// alerts one at a time via the `fseek`-back, then terminates.
#[test]
fn c11_pipeline_many_alerts() {
    let g = world();
    let mut rng = Rng::new(0xC11);
    for n in 0..40 {
        let nalerts = 1 + rng.below(5);
        let mut content = Vec::new();
        for k in 0..nalerts {
            let group = *rng.pick(&["syslog,", "ossec,syscheck,", "errors,"]);
            content.extend_from_slice(&alert_block(
                &format!("146110254{k}.{n}"),
                group,
                &format!("2016 Apr 19 20:29:0{} h{k}->/var/log/m{k}", k % 10),
                &[
                    "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
                    "Integrity checksum changed for: '/etc/passwd'",
                    "Src IP: 10.0.0.1",
                    "Src Port: 4242",
                    "User: root",
                ],
            ));
        }
        let flag = READ_ALL | if rng.bool() { MAIL } else { 0 };
        let steps = assert_pipeline_eq(
            flag,
            0,
            nalerts + 2,
            &|| write_alerts_log(&content),
            &format!("C11 pipeline #{n} ({nalerts} alerts, flag={flag:#x})"),
        );
        let got = steps.iter().filter(|s| s.alert.is_some()).count();
        assert_eq!(
            got, nalerts,
            "C11 #{n}: expected {nalerts} alerts, drained {got}"
        );
    }
    drop(g);
}

/// C12 — flags = 0 (seek to end): `Read_FileMon` finds nothing, reopens and
/// re-seeks to the end, then honours `timeout`. `timeout = 0` costs no sleep;
/// `timeout = 1` costs one `file_sleep()` per implementation.
#[test]
fn c12_pipeline_seek_end() {
    let g = world();
    let steps = assert_pipeline_eq(
        0,
        0,
        1,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        "C12 flags=0 timeout=0",
    );
    assert!(steps[0].alert.is_none(), "C12: nothing is ever found");
    assert!(!steps[0].queue.fp_is_null);
    assert_eq!(
        steps[0].queue.fp_pos, MINIMAL.len() as i64,
        "C12: the reopened stream is seeked to the end again"
    );

    let steps = assert_pipeline_eq(
        0,
        1,
        1,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        "C12 flags=0 timeout=1",
    );
    assert!(steps[0].alert.is_none());
    drop(g);
}

/// C12 variant — an EMPTY `alerts.log` with flags = 0 and `READ_ALL`.
#[test]
fn c12_pipeline_empty_file() {
    let g = world();
    for flags in [0, READ_ALL] {
        let steps = assert_pipeline_eq(
            flags,
            0,
            2,
            &|| write_alerts_log(b""),
            &format!("C12 empty file flags={flags:#x}"),
        );
        assert!(steps.iter().all(|s| s.alert.is_none()));
    }
    drop(g);
}
