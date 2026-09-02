//! ERRORS.md rows E5–E14: the rejection paths in `file-queue.c`
//! (`Handle_Queue`, `Init_FileQueue`, `Read_FileMon`) and in `driver.c`.
//!
//! These drive the LOW-LEVEL entry points directly, because `Handle_Queue` is
//! `static` and several of its branches are unreachable through `driver`.
//!
//! Several rows go through `file_sleep()`, a 5 s `select` — those tests are
//! marked in their doc comment with their cost.

mod common;

use common::*;
use std::ffi::{CString, c_char, c_int};

const READ_ALL: c_int = 0x004;
const FP_SET: c_int = 0x010;

struct InitOutcome {
    rc: c_int,
    queue: QueueSnap,
    stderr: Vec<u8>,
}

/// Run `Init_FileQueue` on `lib` with a freshly zeroed queue (what `driver`
/// does), capturing the return code, the resulting queue state and stderr.
fn init_on(lib: &Lib, flags: c_int, preset_fp: Option<*mut FILE>, t: &tm) -> InitOutcome {
    let mut fq = file_queue::zeroed();
    if let Some(fp) = preset_fp {
        fq.fp = fp;
    }
    let (rc, err) = capture_stderr(|| {
        set_errno(0);
        unsafe { (lib.init_file_queue)(&mut fq, t, flags) }
    });
    let queue = unsafe { snap_queue(&fq) };
    unsafe {
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
    }
    InitOutcome {
        rc,
        queue,
        stderr: err,
    }
}

fn assert_init_eq(flags: c_int, mk_fp: Option<&dyn Fn() -> *mut FILE>, what: &str) -> InitOutcome {
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);
    let a = init_on(c, flags, mk_fp.map(|f| f()), &t);
    let b = init_on(r, flags, mk_fp.map(|f| f()), &t);
    assert_eq!(a.rc, b.rc, "[{what}] Init_FileQueue rc differs");
    assert_eq!(a.queue, b.queue, "[{what}] queue state differs");
    assert_eq!(
        String::from_utf8_lossy(&a.stderr),
        String::from_utf8_lossy(&b.stderr),
        "[{what}] stderr differs"
    );
    a
}

/// E5 — `alerts.log` absent: `fopen` fails, `Handle_Queue` returns 0, so
/// `Init_FileQueue` still returns **0** and leaves `fp == NULL`.
#[test]
fn e5_missing_alerts_log() {
    let g = world();
    remove_alerts_log();
    let out = assert_init_eq(0, None, "E5 missing alerts.log, flags=0");
    assert_eq!(out.rc, 0, "E5: fopen failure is NOT propagated as an error");
    assert!(out.fp_null(), "E5: fp must stay NULL");
    assert_eq!(out.queue.file_name, b"alerts.log".to_vec());
    assert_eq!(out.queue.last_change, 0);
    assert!(out.stderr.is_empty(), "E5 is silent");

    // Same with READ_ALL, and with an unreadable file (fopen fails on EACCES).
    assert_init_eq(READ_ALL, None, "E5 missing alerts.log, flags=READ_ALL");

    write_alerts_log(MINIMAL.as_bytes());
    let p = scratch_dir().join("alerts.log");
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    std::fs::set_permissions(&p, perms).unwrap();
    let unreadable = std::fs::File::open(&p).is_err();
    if unreadable {
        let out = assert_init_eq(0, None, "E5 unreadable alerts.log");
        assert_eq!(out.rc, 0);
        assert!(out.fp_null());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&p, perms).unwrap();
    }
    drop(g);
}

/// E6 — `CRALERT_FP_SET` set, `CRALERT_READ_ALL` clear, `fp == NULL`:
/// `Handle_Queue` returns 0 at `file-queue.c:86`, before any fseek.
#[test]
fn e6_fp_set_null_fp() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let out = assert_init_eq(FP_SET, None, "E6 FP_SET with NULL fp");
    assert_eq!(out.rc, 0, "E6: returns 0, not -1");
    assert!(out.fp_null());
    assert_eq!(
        out.queue.file_name,
        b"<stdin>".to_vec(),
        "E6: FP_SET renames the queue to <stdin>"
    );
    assert_eq!(out.queue.last_change, 0);
    assert!(out.stderr.is_empty());
    drop(g);
}

/// C08 / E6 variant — `FP_SET | READ_ALL` with `fp == NULL` skips the fseek AND
/// the fstat, so `Handle_Queue` reaches `return (1)` with a NULL `fp`.
#[test]
fn e6b_fp_set_read_all_null_fp() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let out = assert_init_eq(FP_SET | READ_ALL, None, "E6b FP_SET|READ_ALL, NULL fp");
    assert_eq!(out.rc, 0);
    assert!(out.fp_null());
    assert_eq!(out.queue.file_name, b"<stdin>".to_vec());
    assert!(out.stderr.is_empty());
    drop(g);
}

/// E7 / E9 — `fseek(fp, 0, SEEK_END)` fails on a non-seekable stream:
/// `merror(FSEEK_ERROR, ...)`, `fclose`, `fp = NULL`, `Handle_Queue` → -1, so
/// `Init_FileQueue` returns **-1**.
#[test]
fn e7_fseek_fails_on_pipe() {
    let g = world();
    let mk = || unseekable_stream(MINIMAL.as_bytes());
    let out = assert_init_eq(FP_SET, Some(&mk), "E7 FP_SET over a pipe");
    assert_eq!(out.rc, -1, "E7/E9: Init_FileQueue must return -1");
    assert!(out.fp_null(), "E7: fp is closed and nulled");
    let msg = String::from_utf8_lossy(&out.stderr);
    assert!(
        msg.contains("(1116): Could not set position in file '<stdin>'")
            && msg.contains("Illegal seek"),
        "E7 unexpected merror text: {msg:?}"
    );
    assert!(msg.ends_with('\n'), "merror appends a newline");
    drop(g);
}

/// E10 — `driver` reports and returns NULL when `Init_FileQueue` fails.
///
/// Reached by making `alerts.log` a FIFO: `fopen` succeeds (a writer is held
/// open) but `fseek(SEEK_END)` fails with ESPIPE.
#[test]
fn e10_driver_init_fail() {
    let g = world();
    let (c, r) = libs();
    let (ca, cerr, ra, rerr) = with_fifo_alerts_log(|| {
        let (ca, cerr) = capture_stderr(|| unsafe {
            set_errno(0);
            let a = (c.driver)(19, 3, 116, 0, 0);
            let s = snap_alert(a);
            if !a.is_null() {
                (c.free_alert_data)(a);
            }
            s
        });
        let (ra, rerr) = capture_stderr(|| unsafe {
            set_errno(0);
            let a = (r.driver)(19, 3, 116, 0, 0);
            let s = snap_alert(a);
            if !a.is_null() {
                (r.free_alert_data)(a);
            }
            s
        });
        (ca, cerr, ra, rerr)
    });
    assert_eq!(ca, ra, "E10 driver return differs");
    assert_eq!(
        String::from_utf8_lossy(&cerr),
        String::from_utf8_lossy(&rerr),
        "E10 stderr differs"
    );
    assert!(ca.is_none(), "E10: driver must return NULL");
    let msg = String::from_utf8_lossy(&cerr);
    assert!(
        msg.contains("(1116): Could not set position in file 'alerts.log'"),
        "E10 expected the merror from Handle_Queue: {msg:?}"
    );
    assert!(
        msg.contains("File queue initialization failed"),
        "E10 expected driver's own message: {msg:?}"
    );
    // driver's message has no trailing newline, so it must be the LAST bytes.
    assert!(
        cerr.ends_with(b"File queue initialization failed"),
        "E10 message order/termination differs: {msg:?}"
    );
    drop(g);
}

// ---------------------------------------------------------------------------
// Read_FileMon rejection paths (these pay for file_sleep())
// ---------------------------------------------------------------------------

struct MonOutcome {
    alert: Option<AlertSnap>,
    queue: QueueSnap,
    stderr: Vec<u8>,
}

/// `Init_FileQueue` then `Read_FileMon`, with an optional mutation in between.
fn mon_on(
    lib: &Lib,
    flags: c_int,
    timeout: u32,
    t: &tm,
    between: &dyn Fn(),
    preset_fp: Option<*mut FILE>,
) -> MonOutcome {
    let mut fq = file_queue::zeroed();
    if let Some(fp) = preset_fp {
        fq.fp = fp;
    }
    let (out, err) = capture_stderr(|| {
        set_errno(0);
        unsafe {
            let rc = (lib.init_file_queue)(&mut fq, t, flags);
            assert_eq!(rc, 0, "precondition: Init_FileQueue must succeed");
            between();
            let a = (lib.read_file_mon)(&mut fq, t, timeout);
            let s = snap_alert(a);
            if !a.is_null() {
                (lib.free_alert_data)(a);
            }
            s
        }
    });
    let queue = unsafe { snap_queue(&fq) };
    unsafe {
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
    }
    MonOutcome {
        alert: out,
        queue,
        stderr: err,
    }
}

fn assert_mon_eq(
    flags: c_int,
    timeout: u32,
    setup: &dyn Fn(),
    between: &dyn Fn(),
    what: &str,
) -> MonOutcome {
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);
    setup();
    let a = mon_on(c, flags, timeout, &t, between, None);
    setup();
    let b = mon_on(r, flags, timeout, &t, between, None);
    assert_eq!(a.alert, b.alert, "[{what}] Read_FileMon result differs");
    assert_eq!(a.queue, b.queue, "[{what}] queue state differs");
    assert_eq!(
        String::from_utf8_lossy(&a.stderr),
        String::from_utf8_lossy(&b.stderr),
        "[{what}] stderr differs"
    );
    a
}

/// E11 — `fp == NULL` on entry and `Handle_Queue(fileq, 0)` cannot open the
/// file: one `file_sleep()` then NULL. Cost: ~10 s (5 s per implementation).
///
/// Also the only observable check on `FQ_TIMEOUT`: `file_sleep` is a `static`
/// helper whose 5 s `select` shows up nowhere but the wall clock.
#[test]
fn e11_read_filemon_null_fp_no_file() {
    let g = world();
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);

    remove_alerts_log();
    let t0 = std::time::Instant::now();
    let a = mon_on(c, 0, 0, &t, &|| {}, None);
    let c_elapsed = t0.elapsed();

    remove_alerts_log();
    let t1 = std::time::Instant::now();
    let b = mon_on(r, 0, 0, &t, &|| {}, None);
    let r_elapsed = t1.elapsed();

    assert_eq!(a.alert, b.alert, "E11 Read_FileMon result differs");
    assert_eq!(a.queue, b.queue, "E11 queue state differs");
    assert_eq!(
        String::from_utf8_lossy(&a.stderr),
        String::from_utf8_lossy(&b.stderr),
        "E11 stderr differs"
    );
    assert!(a.alert.is_none());
    assert!(a.queue.fp_is_null);

    // FQ_TIMEOUT == 5: exactly one file_sleep() on this path.
    for (name, e) in [("C", c_elapsed), ("RUST", r_elapsed)] {
        assert!(
            e.as_millis() >= 4500 && e.as_millis() < 9000,
            "{name}: expected one ~5 s file_sleep(), took {e:?}"
        );
    }
    let delta = c_elapsed.as_millis().abs_diff(r_elapsed.as_millis());
    assert!(
        delta < 1500,
        "FQ_TIMEOUT mismatch: C slept {c_elapsed:?}, RUST slept {r_elapsed:?}"
    );
    drop(g);
}

/// E13 — the first `GetAlertData` yields NULL and the file has since been
/// deleted, so the re-`Handle_Queue(fileq, 0)` fails: sleep then NULL.
/// Cost: ~10 s.
#[test]
fn e13_file_deleted_midway() {
    let g = world();
    let out = assert_mon_eq(
        READ_ALL,
        0,
        &|| write_alerts_log(b"nothing parseable here\n"),
        &|| remove_alerts_log(),
        "E13 alerts.log deleted between the two GetAlertData calls",
    );
    assert!(out.alert.is_none());
    assert!(out.queue.fp_is_null, "E13: Handle_Queue nulls fp on failure");
    drop(g);
}

/// E14 — the `while (i < timeout)` loop runs out. `timeout = 0` costs nothing;
/// `timeout = 1` costs one `file_sleep()` per implementation (~10 s).
#[test]
fn e14_timeout_expires() {
    let g = world();
    // timeout = 0: no loop iteration at all.
    let out = assert_mon_eq(
        0,
        0,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        &|| {},
        "E14 timeout=0 (flags=0 seeks to EOF so nothing is ever found)",
    );
    assert!(out.alert.is_none());
    assert!(!out.queue.fp_is_null, "E14: the reopened fp stays valid");

    // timeout = 1: exactly one iteration + one sleep.
    let out = assert_mon_eq(
        0,
        1,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        &|| {},
        "E14 timeout=1",
    );
    assert!(out.alert.is_none());
    drop(g);
}

/// E14 boundary — `timeout` is `unsigned int`; check that a huge value is not
/// reached when the first `GetAlertData` already succeeds (so no sleeping).
#[test]
fn e14_huge_timeout_not_reached_on_success() {
    let g = world();
    let out = assert_mon_eq(
        READ_ALL,
        u32::MAX,
        &|| write_alerts_log(MINIMAL.as_bytes()),
        &|| {},
        "E14 timeout=UINT_MAX with a parseable alert",
    );
    let a = out.alert.expect("must return the alert immediately");
    assert_eq!(a.rule, 1002);
    assert_eq!(a.level, 7);
    drop(g);
}

/// E7 control — `merror` itself, both templates, compared byte for byte.
#[test]
fn e7_merror_byte_identical() {
    let g = world();
    let (c, r) = libs();
    let fstat_t =
        CString::new("(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].")
            .unwrap();
    let fseek_t =
        CString::new("(1116): Could not set position in file '%s' due to [(%d)-(%s)].").unwrap();
    let mut rng = Rng::new(0xE7);
    for i in 0..80 {
        let tmpl = if i % 2 == 0 { &fstat_t } else { &fseek_t };
        let name = CString::new(rng.token(300).into_iter().filter(|&b| b != 0).collect::<Vec<_>>())
            .unwrap();
        let msg = CString::new(rng.token(80).into_iter().filter(|&b| b != 0).collect::<Vec<_>>())
            .unwrap();
        let err = rng.i32();
        let call = |lib: &Lib| {
            capture_stderr(|| unsafe {
                (lib.merror)(
                    tmpl.as_ptr() as *const c_char,
                    name.as_ptr() as *const c_char,
                    err,
                    msg.as_ptr() as *const c_char,
                )
            })
            .1
        };
        let a = call(c);
        let b = call(r);
        assert_eq!(
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b),
            "merror #{i} differs"
        );
    }
    drop(g);
}

impl InitOutcome {
    fn fp_null(&self) -> bool {
        self.queue.fp_is_null
    }
}
