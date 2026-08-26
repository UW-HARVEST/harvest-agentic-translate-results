//! Phase B — CONFIGS.md rows 30–49: `Init_FileQueue`, `Read_FileMon` and the
//! `driver` one-shot wrapper, driven through the `.so` exports.
//!
//! These rows depend on the process CWD (`Handle_Queue` does
//! `fopen(fileq->file_name, "r")` with a *relative* name), so every test takes
//! a [`Scratch`] which serialises on a global lock and gives both
//! implementations the exact same directory contents.

mod common;

use common::*;
use std::ffi::{c_int, c_uint};

const MAIL: c_int = 0x001;
const EXEC: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;

const ALERTS: &str = "alerts.log";
const STDIN_NAME: &str = "<stdin>";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
struct InitResult {
    rc: c_int,
    q: QueueSnap,
    st: Option<StreamState>,
}

/// Run `Init_FileQueue` in `api`, snapshot everything observable, and close the
/// stream it leaves behind.
unsafe fn run_init(
    api: &Api,
    mut q: FileQueue,
    tm: &Tm,
    flags: c_int,
    preset: Option<*mut FILE>,
    ignore_mon: bool,
) -> InitResult {
    if let Some(f) = preset {
        q.fp = f;
    }
    let rc = (api.Init_FileQueue)(&mut q, tm, flags);
    let st = stream_state(q.fp);
    let snap = if ignore_mon {
        snap_queue_ignoring_mon(&q)
    } else {
        snap_queue(&q)
    };
    if !q.fp.is_null() {
        fclose(q.fp);
    }
    InitResult { rc, q: snap, st }
}

/// Differential `Init_FileQueue`. `open_input` is called once per
/// implementation when a caller-supplied `fp` is needed; it must open the *same
/// path* both times so `fstat` yields identical metadata.
fn diff_init(
    tag: &str,
    start: FileQueue,
    tm: &Tm,
    flags: c_int,
    preset_path: Option<&str>,
    ignore_mon: bool,
) {
    let (c, r) = apis();
    unsafe {
        let mk = || -> Option<*mut FILE> {
            preset_path.map(|p| {
                let f = fopen_str(p, "r");
                assert!(!f.is_null(), "fopen {p} failed");
                f
            })
        };
        let rc = run_init(c, start, tm, flags, mk(), ignore_mon);
        let rr = run_init(r, start, tm, flags, mk(), ignore_mon);
        assert_eq!(
            rc.rc, rr.rc,
            "[{tag}] flags={flags:#x} Init_FileQueue return code differs"
        );
        assert_eq!(rc.q, rr.q, "[{tag}] flags={flags:#x} file_queue differs");
        assert_eq!(rc.st, rr.st, "[{tag}] flags={flags:#x} stream state differs");
    }
}

#[derive(PartialEq, Eq, Debug)]
struct ReadResult {
    alert: Option<AlertSnap>,
    q: QueueSnap,
    st: Option<StreamState>,
}

/// `Init_FileQueue` followed by `n` `Read_FileMon` calls, all observable state
/// captured after each one.
unsafe fn run_read(
    api: &Api,
    mut q: FileQueue,
    init_tm: &Tm,
    read_tm: &Tm,
    flags: c_int,
    preset: Option<*mut FILE>,
    timeout: c_uint,
    n: usize,
) -> (c_int, Vec<ReadResult>) {
    if let Some(f) = preset {
        q.fp = f;
    }
    let rc = (api.Init_FileQueue)(&mut q, init_tm, flags);
    let mut out = Vec::new();
    for _ in 0..n {
        let p = (api.Read_FileMon)(&mut q, read_tm, timeout);
        let alert = take_alert(api, p);
        out.push(ReadResult {
            alert,
            q: snap_queue(&q),
            st: stream_state(q.fp),
        });
    }
    if !q.fp.is_null() {
        fclose(q.fp);
    }
    (rc, out)
}

#[allow(clippy::too_many_arguments)]
fn diff_read(
    tag: &str,
    start: FileQueue,
    init_tm: &Tm,
    read_tm: &Tm,
    flags: c_int,
    preset_path: Option<&str>,
    timeout: c_uint,
    n: usize,
) {
    let (c, r) = apis();
    unsafe {
        let mk = || -> Option<*mut FILE> {
            preset_path.map(|p| {
                let f = fopen_str(p, "r");
                assert!(!f.is_null(), "fopen {p} failed");
                f
            })
        };
        let (rcc, rc) = run_read(c, start, init_tm, read_tm, flags, mk(), timeout, n);
        let (rcr, rr) = run_read(r, start, init_tm, read_tm, flags, mk(), timeout, n);
        assert_eq!(rcc, rcr, "[{tag}] flags={flags:#x} Init rc differs");
        assert_eq!(
            rc, rr,
            "[{tag}] flags={flags:#x} timeout={timeout} Read_FileMon results differ"
        );
    }
}

fn diff_driver(tag: &str, day: c_int, month: c_int, year: c_int, timeout: c_uint, flags: c_int) {
    let (c, r) = apis();
    unsafe {
        let pc = (c.driver)(day, month, year, timeout, flags);
        let sc = take_alert(c, pc);
        let pr = (r.driver)(day, month, year, timeout, flags);
        let sr = take_alert(r, pr);
        assert_eq!(
            sc, sr,
            "[{tag}] driver({day},{month},{year},{timeout},{flags:#x}) differs"
        );
    }
}

fn m_below(rng: &mut Rng, n: u64) -> usize {
    rng.below(n) as usize
}

fn sample_alerts(rng: &mut Rng, n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        rand_alert(rng, i % 2 == 0).render(&mut v);
    }
    v
}

fn tm_of(mday: c_int, mon: c_int, year: c_int) -> Tm {
    let mut t = Tm::default();
    t.tm_mday = mday;
    t.tm_mon = mon;
    t.tm_year = year;
    t
}

// ---------------------------------------------------------------------------
// Row 30 — Init_FileQueue, flags = 0, alerts.log present (seek to EOF)
// ---------------------------------------------------------------------------

#[test]
fn cfg_30_init_flags0_file_present() {
    let s = Scratch::new("cfg30");
    let mut rng = Rng::new(0x3030_2024);
    for i in 0..40 {
        let n = rng.below(4) as usize;
        s.write(ALERTS, &sample_alerts(&mut rng, n));
        let tm = tm_of(
            1 + rng.below(28) as c_int,
            rng.below(12) as c_int,
            rng.below(200) as c_int,
        );
        diff_init(
            &format!("init0#{i}"),
            FileQueue::zeroed(),
            &tm,
            0,
            None,
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 31 — flags = CRALERT_READ_ALL: no seek, offset stays 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_31_init_read_all() {
    let s = Scratch::new("cfg31");
    let mut rng = Rng::new(0x3131_2024);
    for i in 0..40 {
        let _n = 1 + m_below(&mut rng, 3);
        s.write(ALERTS, &sample_alerts(&mut rng, _n));
        let tm = tm_of(5, (i % 12) as c_int, 124);
        diff_init(
            &format!("initRA#{i}"),
            FileQueue::zeroed(),
            &tm,
            READ_ALL,
            None,
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 32 & 33 — CRALERT_FP_SET with a caller-supplied stream
// ---------------------------------------------------------------------------

#[test]
fn cfg_32_33_init_fp_set_with_caller_stream() {
    let s = Scratch::new("cfg3233");
    let mut rng = Rng::new(0x3233_2024);
    for i in 0..40 {
        let _n = 1 + m_below(&mut rng, 3);
        s.write("input.log", &sample_alerts(&mut rng, _n));
        s.remove(ALERTS);
        let tm = tm_of(11, (i % 12) as c_int, 100);
        // Row 32: FP_SET only -> fseek to EOF happens on the caller's stream.
        diff_init(
            &format!("fpset#{i}"),
            FileQueue::zeroed(),
            &tm,
            FP_SET,
            Some("input.log"),
            false,
        );
        // Row 33: FP_SET | READ_ALL -> no fseek, offset unchanged.
        diff_init(
            &format!("fpsetRA#{i}"),
            FileQueue::zeroed(),
            &tm,
            FP_SET | READ_ALL,
            Some("input.log"),
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 34 — a file literally named `<stdin>` exists but flags = 0
// ---------------------------------------------------------------------------

#[test]
fn cfg_34_init_stdin_named_file_ignored_without_fp_set() {
    let s = Scratch::new("cfg34");
    let mut rng = Rng::new(0x3434_2024);
    for i in 0..25 {
        s.write(ALERTS, &sample_alerts(&mut rng, 2));
        s.write(STDIN_NAME, b"this is the <stdin> file, different size\n");
        let tm = tm_of(1, (i % 12) as c_int, 70);
        diff_init(
            &format!("stdinfile#{i}"),
            FileQueue::zeroed(),
            &tm,
            0,
            None,
            false,
        );
        // and with FP_SET but no caller fp: `<stdin>` must still NOT be opened
        diff_init(
            &format!("stdinfile-fpset#{i}"),
            FileQueue::zeroed(),
            &tm,
            FP_SET,
            None,
            false,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 35 — FP_SET with a NULL fp (accepted-side twin of ERRORS.md #20)
// ---------------------------------------------------------------------------

#[test]
fn cfg_35_init_fp_set_null_fp() {
    let s = Scratch::new("cfg35");
    s.write(STDIN_NAME, b"content\n");
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    let tm = tm_of(9, 3, 125);
    for flags in [FP_SET, FP_SET | READ_ALL, FP_SET | MAIL, FP_SET | READ_ALL | MAIL] {
        diff_init("fpset-nullfp", FileQueue::zeroed(), &tm, flags, None, false);
    }
}

// ---------------------------------------------------------------------------
// Row 36 — all 32 flag subsets x {alerts.log present, absent}
//          x {caller fp real, NULL}
// ---------------------------------------------------------------------------

#[test]
fn cfg_36_init_full_flag_matrix() {
    let s = Scratch::new("cfg36");
    let mut rng = Rng::new(0x3636_2024);
    let bits = [MAIL, EXEC, READ_ALL, READ_FAILED, FP_SET];
    let content = sample_alerts(&mut rng, 2);
    for present in [true, false] {
        for preset in [false, true] {
            if present {
                s.write(ALERTS, &content);
            } else {
                s.remove(ALERTS);
            }
            s.write("input.log", &content);
            for mask in 0u32..32 {
                let mut flags = 0;
                for (b, bit) in bits.iter().enumerate() {
                    if mask & (1 << b) != 0 {
                        flags |= *bit;
                    }
                }
                let tm = tm_of(17, (mask % 12) as c_int, 123);
                diff_init(
                    &format!("matrix present={present} preset={preset}"),
                    FileQueue::zeroed(),
                    &tm,
                    flags,
                    if preset { Some("input.log") } else { None },
                    false,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 37 — randomized valid tm scalars
// ---------------------------------------------------------------------------

#[test]
fn cfg_37_init_tm_scalars() {
    let s = Scratch::new("cfg37");
    s.write(ALERTS, b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\n");
    let mut rng = Rng::new(0x3737_2024);
    // every valid month at least once
    for mon in 0..12 {
        let tm = tm_of(1 + rng.below(31) as c_int, mon, rng.range_i64(-1900, 8099) as c_int);
        diff_init("tm-mon", FileQueue::zeroed(), &tm, READ_ALL, None, false);
    }
    for _ in 0..200 {
        let tm = tm_of(
            rng.range_i64(1, 31) as c_int,
            rng.range_i64(0, 11) as c_int,
            rng.range_i64(-1900, 8099) as c_int,
        );
        diff_init("tm-rand", FileQueue::zeroed(), &tm, READ_ALL, None, false);
    }
    // extreme but in-range-for-s_month values
    for &y in &[i32::MIN, i32::MIN + 1, -1900, -1, 0, 1, 8099, i32::MAX - 1, i32::MAX] {
        for &d in &[i32::MIN, -1, 0, 1, 31, i32::MAX] {
            let tm = tm_of(d, 6, y);
            diff_init("tm-extreme", FileQueue::zeroed(), &tm, READ_ALL, None, false);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 38 — a pre-dirtied file_queue proves the re-initialisation order
// ---------------------------------------------------------------------------

#[test]
fn cfg_38_init_pre_dirtied_struct() {
    let s = Scratch::new("cfg38");
    let mut rng = Rng::new(0x3838_2024);
    for present in [true, false] {
        if present {
            s.write(ALERTS, &sample_alerts(&mut rng, 2));
        } else {
            s.remove(ALERTS);
        }
        s.write("input.log", &sample_alerts(&mut rng, 1));
        for fill in [0x00u8, 0x01, 0x41, 0xAB, 0xFF, 0x7F, 0x80] {
            for flags in [0, READ_ALL, MAIL, FP_SET, FP_SET | READ_ALL] {
                let preset = if flags & FP_SET != 0 {
                    Some("input.log")
                } else {
                    None
                };
                diff_init(
                    &format!("dirty{fill:#02x} present={present}"),
                    FileQueue::dirty(fill),
                    &tm_of(3, 7, 111),
                    flags,
                    preset,
                    false,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 39 — Read_FileMon walking a multi-alert alerts.log (READ_ALL, timeout 0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_39_read_filemon_walks_file() {
    let s = Scratch::new("cfg39");
    let mut rng = Rng::new(0x3939_2024);
    for i in 0..60 {
        let n = 1 + rng.below(4) as usize;
        s.write(ALERTS, &sample_alerts(&mut rng, n));
        let tm = tm_of(2, (i % 12) as c_int, 120);
        diff_read(
            &format!("walk#{i}"),
            FileQueue::zeroed(),
            &tm,
            &tm,
            READ_ALL,
            None,
            0,
            n + 2,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 40 — flags = 0 (Init seeks to EOF) so Read_FileMon sees EOF
// ---------------------------------------------------------------------------

#[test]
fn cfg_40_read_filemon_after_seek_to_eof() {
    let s = Scratch::new("cfg40");
    let mut rng = Rng::new(0x4040_2024);
    for i in 0..40 {
        let _n = 1 + m_below(&mut rng, 3);
        s.write(ALERTS, &sample_alerts(&mut rng, _n));
        let tm = tm_of(4, (i % 12) as c_int, 121);
        diff_read(
            &format!("eof#{i}"),
            FileQueue::zeroed(),
            &tm,
            &tm,
            0,
            None,
            0,
            2,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 41 — FP_SET | READ_ALL: parse from the caller's stream
// ---------------------------------------------------------------------------

#[test]
fn cfg_41_read_filemon_from_caller_stream() {
    let s = Scratch::new("cfg41");
    let mut rng = Rng::new(0x4141_2024);
    for i in 0..40 {
        let n = 1 + rng.below(3) as usize;
        s.write("input.log", &sample_alerts(&mut rng, n));
        // `<stdin>` must exist, otherwise the NULL-path re-open sleeps 5 s.
        s.write(STDIN_NAME, b"");
        let tm = tm_of(6, (i % 12) as c_int, 122);
        diff_read(
            &format!("caller#{i}"),
            FileQueue::zeroed(),
            &tm,
            &tm,
            FP_SET | READ_ALL,
            Some("input.log"),
            0,
            n + 1,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 42 — the flag forwarded into GetAlertData comes from fileq->flags
// ---------------------------------------------------------------------------

#[test]
fn cfg_42_read_filemon_forwards_mail_flag() {
    let s = Scratch::new("cfg42");
    let mut rng = Rng::new(0x4242_2024);
    for i in 0..40 {
        // deliberately mixed mail / non-mail alerts
        let mut v = Vec::new();
        let n = 2 + rng.below(3) as usize;
        for k in 0..n {
            rand_alert(&mut rng, k % 2 == 0).render(&mut v);
        }
        s.write(ALERTS, &v);
        let tm = tm_of(8, (i % 12) as c_int, 123);
        for flags in [READ_ALL, MAIL | READ_ALL, EXEC | READ_ALL, READ_FAILED | READ_ALL] {
            diff_read(
                &format!("mailfwd#{i}"),
                FileQueue::zeroed(),
                &tm,
                &tm,
                flags,
                None,
                0,
                n + 2,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 43 — Read_FileMon re-assigns day/year/mon from its own `p`
// ---------------------------------------------------------------------------

#[test]
fn cfg_43_read_filemon_reassigns_tm() {
    let s = Scratch::new("cfg43");
    let mut rng = Rng::new(0x4343_2024);
    // an empty alerts.log guarantees the first GetAlertData returns NULL, which
    // is the only path that re-assigns day/year/mon.
    s.write(ALERTS, b"");
    for _ in 0..60 {
        let a = tm_of(
            rng.range_i64(1, 31) as c_int,
            rng.range_i64(0, 11) as c_int,
            rng.range_i64(0, 200) as c_int,
        );
        let b = tm_of(
            rng.range_i64(1, 31) as c_int,
            rng.range_i64(0, 11) as c_int,
            rng.range_i64(0, 200) as c_int,
        );
        diff_read(
            "reassign",
            FileQueue::zeroed(),
            &a,
            &b,
            READ_ALL,
            None,
            0,
            2,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 44 — the retry loop runs once (timeout = 1). Costs ~5 s per library.
// ---------------------------------------------------------------------------

#[test]
fn cfg_44_read_filemon_timeout_one() {
    let s = Scratch::new("cfg44");
    s.write(ALERTS, b"");
    let tm = tm_of(10, 5, 124);
    diff_read(
        "timeout1",
        FileQueue::zeroed(),
        &tm,
        &tm,
        READ_ALL,
        None,
        1,
        1,
    );
}

// ---------------------------------------------------------------------------
// Rows 45–49 — the `driver` one-shot wrapper
// ---------------------------------------------------------------------------

#[test]
fn cfg_45_driver_read_all_randomized() {
    let s = Scratch::new("cfg45");
    let mut rng = Rng::new(0x4545_2024);
    for i in 0..120 {
        let _n = 1 + m_below(&mut rng, 4);
        s.write(ALERTS, &sample_alerts(&mut rng, _n));
        diff_driver(
            &format!("drv#{i}"),
            rng.range_i64(1, 31) as c_int,
            rng.range_i64(0, 11) as c_int,
            rng.range_i64(0, 200) as c_int,
            0,
            READ_ALL,
        );
    }
}

#[test]
fn cfg_46_driver_flags_zero() {
    let s = Scratch::new("cfg46");
    let mut rng = Rng::new(0x4646_2024);
    for i in 0..40 {
        let _n = 1 + m_below(&mut rng, 3);
        s.write(ALERTS, &sample_alerts(&mut rng, _n));
        diff_driver(&format!("drv0#{i}"), 1, (i % 12) as c_int, 120, 0, 0);
    }
}

#[test]
fn cfg_47_driver_mail_variants() {
    let s = Scratch::new("cfg47");
    let mut rng = Rng::new(0x4747_2024);
    for i in 0..60 {
        let mut v = Vec::new();
        let n = 1 + rng.below(4) as usize;
        for k in 0..n {
            rand_alert(&mut rng, (k + i) % 2 == 0).render(&mut v);
        }
        s.write(ALERTS, &v);
        for flags in [READ_ALL, MAIL | READ_ALL] {
            diff_driver(&format!("drvmail#{i}"), 3, 3, 123, 0, flags);
        }
    }
}

#[test]
fn cfg_48_driver_full_flag_matrix() {
    let s = Scratch::new("cfg48");
    let mut rng = Rng::new(0x4848_2024);
    let bits = [MAIL, EXEC, READ_ALL, READ_FAILED, FP_SET];
    let content = sample_alerts(&mut rng, 2);
    // Both candidate file names exist, so no code path has to `file_sleep`.
    s.write(ALERTS, &content);
    s.write(STDIN_NAME, &content);
    for mask in 0u32..32 {
        let mut flags = 0;
        for (b, bit) in bits.iter().enumerate() {
            if mask & (1 << b) != 0 {
                flags |= *bit;
            }
        }
        diff_driver(
            &format!("drvmatrix{flags:#x}"),
            (mask as c_int) % 28 + 1,
            (mask as c_int) % 12,
            100 + mask as c_int,
            0,
            flags,
        );
    }
}

#[test]
fn cfg_49_driver_syscheck_alert() {
    let s = Scratch::new("cfg49");
    let mut rng = Rng::new(0x4949_2024);
    for i in 0..60 {
        let path = rng.token_len(1, 50);
        let mut v = Vec::new();
        v.extend_from_slice(b"** Alert 1234.5: mail - ossec,syscheck,\n");
        v.extend_from_slice(b"2006 Apr 13 16:15:17 (agent) 10.0.0.1->syscheck\n");
        v.extend_from_slice(b"Rule: 550 (level) 7 -> 'Integrity checksum changed.'\n");
        v.extend_from_slice(b"Integrity checksum changed for: '");
        v.extend_from_slice(&path);
        v.extend_from_slice(b"'\n");
        v.extend_from_slice(b"Old md5sum was: aaaa\n");
        s.write(ALERTS, &v);
        for flags in [READ_ALL, MAIL | READ_ALL] {
            diff_driver(&format!("drvsys#{i}"), 13, 3, 106, 0, flags);
        }
    }
}
