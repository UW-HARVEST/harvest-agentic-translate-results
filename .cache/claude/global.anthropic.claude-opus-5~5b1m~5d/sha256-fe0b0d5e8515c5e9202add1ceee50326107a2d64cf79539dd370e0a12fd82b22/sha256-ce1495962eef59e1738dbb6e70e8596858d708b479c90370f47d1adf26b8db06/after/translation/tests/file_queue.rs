//! Phase B rows 20-31 and Phase C rows 18-25 / 27: `Init_FileQueue` and
//! `Read_FileMon`, driven through both `.so`s.

mod common;

use common::*;
use std::ffi::{c_int, c_uint};

/* ------------------------------------------------------------------ */
/* helpers                                                            */
/* ------------------------------------------------------------------ */

#[derive(Clone, Debug)]
enum Fp {
    /// leave `fileq->fp` NULL
    Null,
    /// a normal read-only stream on `name`
    File(String),
    /// a non-seekable stream (pipe) pre-loaded with `data`
    Pipe(Vec<u8>),
    /// a stream on `name` whose underlying fd has been closed
    ClosedFd(String),
}

unsafe fn make_fp(kind: &Fp) -> *mut FILE {
    match kind {
        Fp::Null => std::ptr::null_mut(),
        Fp::File(n) => open_ro(n),
        Fp::Pipe(data) => {
            let mut fds = [0i32; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0);
            if !data.is_empty() {
                write(fds[1], data.as_ptr() as *const _, data.len());
            }
            close(fds[1]);
            let m = cpath("r");
            let f = fdopen(fds[0], m.as_ptr());
            assert!(!f.is_null());
            f
        }
        Fp::ClosedFd(n) => {
            // Move the descriptor to a deliberately HIGH number before closing
            // it, so that the next `open` (glibc always picks the lowest free
            // fd) cannot silently make `fileno(fp)` valid again.
            use std::sync::atomic::{AtomicI32, Ordering};
            static NEXT: AtomicI32 = AtomicI32::new(900);
            let target = NEXT.fetch_add(1, Ordering::Relaxed);
            let f0 = open_ro(n);
            assert_eq!(dup2(fileno(f0), target), target);
            fclose(f0);
            let m = cpath("r");
            let f = fdopen(target, m.as_ptr());
            assert!(!f.is_null());
            close(target);
            f
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct InitOut {
    rc: c_int,
    q: QueueSnap,
    stream: Option<StreamSnap>,
    err: String,
}

/// Runs `Init_FileQueue` on a fresh `file_queue`.
///
/// `prefill` (when `Some`) is copied over the whole struct first, so that the
/// fields the C leaves untouched are also compared.
unsafe fn run_init(
    api: &Api,
    flags: c_int,
    t: &tm,
    fp: &Fp,
    prefill: Option<&[u8]>,
) -> (InitOut, *mut file_queue) {
    let q = Box::leak(Box::new(file_queue::zeroed())) as *mut file_queue;
    if let Some(bytes) = prefill {
        assert_eq!(bytes.len(), std::mem::size_of::<file_queue>());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), q as *mut u8, bytes.len());
    }
    (*q).fp = make_fp(fp);
    set_errno(0);
    let (rc, err) = capture_stderr(|| (api.Init_FileQueue)(q, t, flags));
    let out = InitOut {
        rc,
        q: snap_queue(q),
        stream: snap_stream((*q).fp),
        err: String::from_utf8_lossy(&err).to_string(),
    };
    (out, q)
}

unsafe fn close_queue(q: *mut file_queue) {
    if !(*q).fp.is_null() {
        fclose((*q).fp);
        (*q).fp = std::ptr::null_mut();
    }
    drop(Box::from_raw(q));
}

/// Differential `Init_FileQueue`. Caller holds the exclusive [`guard`].
unsafe fn diff_init(flags: c_int, t: &tm, fp: &Fp, prefill: Option<&[u8]>, label: &str) {
    let (co, cq) = run_init(cc(), flags, t, fp, prefill);
    let (ro, rq) = run_init(rs(), flags, t, fp, prefill);
    assert_eq!(
        co, ro,
        "{label}: Init_FileQueue(flags={flags:#x}, mday={}, mon={}, year={}) differs",
        t.tm_mday, t.tm_mon, t.tm_year
    );
    close_queue(cq);
    close_queue(rq);
}

#[derive(Debug, PartialEq, Eq)]
struct MonOut {
    init: InitOut,
    alerts: Vec<Option<AlertSnap>>,
    q_after: QueueSnap,
    stream_after: Option<StreamSnap>,
    err: String,
}

/// `Init_FileQueue` followed by `n` `Read_FileMon` calls.
unsafe fn run_mon(
    api: &Api,
    flags: c_int,
    t: &tm,
    fp: &Fp,
    timeout: c_uint,
    n: usize,
) -> MonOut {
    let (init, q) = run_init(api, flags, t, fp, None);
    let mut alerts = Vec::new();
    let (_, err) = capture_stderr(|| {
        for _ in 0..n {
            set_errno(0);
            let a = (api.Read_FileMon)(q, t, timeout);
            let s = snap_alert(a);
            if !a.is_null() {
                (api.FreeAlertData)(a);
            }
            let done = s.is_none();
            alerts.push(s);
            if done {
                break;
            }
        }
    });
    let out = MonOut {
        init,
        alerts,
        q_after: snap_queue(q),
        stream_after: snap_stream((*q).fp),
        err: String::from_utf8_lossy(&err).to_string(),
    };
    close_queue(q);
    out
}

unsafe fn diff_mon(flags: c_int, t: &tm, fp: &Fp, timeout: c_uint, n: usize, label: &str) {
    let c = run_mon(cc(), flags, t, fp, timeout, n);
    let r = run_mon(rs(), flags, t, fp, timeout, n);
    assert_eq!(
        c, r,
        "{label}: Read_FileMon(flags={flags:#x}, timeout={timeout}) differs"
    );
}

const MONTHS: [&[u8; 3]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

/* =================== CONFIGS row 20 =================== */

#[test]
fn cfg20_init_flags0_present() {
    let _g = guard();
    let mut rng = Rng::new(0x2020);
    unsafe {
        write_file(ALERTS_DAILY, full_alert("30.1", true, "ossec,").as_bytes());
        for _ in 0..200 {
            let t = tm::new(rng.range_i32(-5, 40), rng.range_i32(0, 11), rng.range_i32(-200, 300));
            diff_init(0, &t, &Fp::Null, None, "init flags0");
        }
        // explicit month sweep: the `mon` field must carry the right 3 bytes
        for m in 0..12 {
            let t = tm::new(15, m, 106);
            let (co, cq) = run_init(cc(), 0, &t, &Fp::Null, None);
            let (ro, rq) = run_init(rs(), 0, &t, &Fp::Null, None);
            assert_eq!(co, ro, "month {m}");
            assert_eq!(&co.q.mon[..3], &MONTHS[m as usize][..], "month {m} name");
            assert_eq!(co.q.year, 2006);
            assert_eq!(co.q.day, 15);
            assert_eq!(co.q.file_name, b"alerts.log".to_vec());
            assert_eq!(co.rc, 0);
            // flags==0 => seek to EOF
            assert_eq!(
                co.stream.as_ref().unwrap().pos,
                std::fs::metadata(ALERTS_DAILY).unwrap().len() as i64
            );
            close_queue(cq);
            close_queue(rq);
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 21 =================== */

#[test]
fn cfg21_init_read_all() {
    let _g = guard();
    let mut rng = Rng::new(0x2121);
    unsafe {
        for content in [
            &b""[..],
            b"short\n",
            full_alert("31.1", false, "syscheck,").as_bytes(),
        ] {
            write_file(ALERTS_DAILY, content);
            for _ in 0..40 {
                let t = tm::new(rng.range_i32(1, 31), rng.range_i32(0, 11), rng.range_i32(0, 200));
                diff_init(CRALERT_READ_ALL, &t, &Fp::Null, None, "init read_all");
            }
            let t = tm::new(1, 0, 100);
            let (co, cq) = run_init(cc(), CRALERT_READ_ALL, &t, &Fp::Null, None);
            let (ro, rq) = run_init(rs(), CRALERT_READ_ALL, &t, &Fp::Null, None);
            assert_eq!(co, ro);
            assert_eq!(co.stream.as_ref().unwrap().pos, 0, "READ_ALL must not seek");
            close_queue(cq);
            close_queue(rq);
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS rows 22 / 23 =================== */

#[test]
fn cfg22_init_fp_set() {
    let _g = guard();
    unsafe {
        let name = scratch("fpset", full_alert("32.1", true, "ossec,").as_bytes());
        let t = tm::new(9, 5, 123);
        let (co, cq) = run_init(cc(), CRALERT_FP_SET, &t, &Fp::File(name.clone()), None);
        let (ro, rq) = run_init(rs(), CRALERT_FP_SET, &t, &Fp::File(name.clone()), None);
        assert_eq!(co, ro);
        assert_eq!(co.rc, 0);
        assert!(!co.q.fp_null, "FP_SET must keep the caller's fp");
        assert_eq!(co.q.file_name, b"<stdin>".to_vec());
        assert_eq!(
            co.stream.as_ref().unwrap().pos,
            std::fs::metadata(&name).unwrap().len() as i64,
            "no READ_ALL => seek to EOF"
        );
        close_queue(cq);
        close_queue(rq);
        let _ = std::fs::remove_file(&name);
    }
}

#[test]
fn cfg23_init_fp_set_read_all() {
    let _g = guard();
    unsafe {
        let name = scratch("fpsetra", full_alert("33.1", true, "syscheck,").as_bytes());
        let t = tm::new(9, 11, 123);
        let flags = CRALERT_FP_SET | CRALERT_READ_ALL;
        let (co, cq) = run_init(cc(), flags, &t, &Fp::File(name.clone()), None);
        let (ro, rq) = run_init(rs(), flags, &t, &Fp::File(name.clone()), None);
        assert_eq!(co, ro);
        assert_eq!(co.rc, 0);
        assert_eq!(co.q.file_name, b"<stdin>".to_vec());
        assert_eq!(co.stream.as_ref().unwrap().pos, 0);
        close_queue(cq);
        close_queue(rq);
        let _ = std::fs::remove_file(&name);
    }
}

/* =================== CONFIGS row 24 =================== */

#[test]
fn cfg24_init_all_flag_combos() {
    let _g = guard();
    unsafe {
        let payload = full_alert("34.1", true, "syscheck,");
        let name = scratch("combo", payload.as_bytes());
        for present in [true, false] {
            if present {
                write_file(ALERTS_DAILY, payload.as_bytes());
            } else {
                remove_file(ALERTS_DAILY);
            }
            for flags in 0..32 {
                for fp in [Fp::Null, Fp::File(name.clone())] {
                    let t = tm::new(3, (flags % 12) as c_int, 120);
                    diff_init(
                        flags,
                        &t,
                        &fp,
                        None,
                        &format!("combo present={present} flags={flags:#x} fp={fp:?}"),
                    );
                }
            }
        }
        remove_file(ALERTS_DAILY);
        let _ = std::fs::remove_file(&name);
    }
}

/* =================== CONFIGS row 25 =================== */

#[test]
fn cfg25_init_prefilled_struct() {
    let _g = guard();
    let mut rng = Rng::new(0x2525);
    unsafe {
        write_file(ALERTS_DAILY, b"junk\n");
        let sz = std::mem::size_of::<file_queue>();
        for case in 0..120 {
            let mut pre: Vec<u8> = (0..sz).map(|_| rng.below(256) as u8).collect();
            // `fp` must not be a wild pointer: only flag combinations WITHOUT
            // CRALERT_FP_SET are safe with garbage there, and those reset it to
            // NULL first; zero it anyway so the FP_SET rows stay well defined.
            let fp_off = 288usize;
            pre[fp_off..fp_off + 8].fill(0);
            let flags = if case % 2 == 0 {
                (case as c_int) & 0x0f
            } else {
                rng.i32() & !CRALERT_FP_SET
            };
            let t = tm::new(rng.range_i32(1, 31), rng.range_i32(0, 11), rng.range_i32(0, 200));
            diff_init(flags, &t, &Fp::Null, Some(&pre), &format!("prefill {case}"));
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 26 =================== */

#[test]
fn cfg26_readfilemon_read_all_hit() {
    let _g = guard();
    unsafe {
        write_file(ALERTS_DAILY, full_alert("35.1", true, "syscheck,").as_bytes());
        let t = tm::new(7, 3, 111);
        diff_mon(CRALERT_READ_ALL, &t, &Fp::Null, 0, 1, "read_all hit");
        diff_mon(
            CRALERT_READ_ALL | CRALERT_MAIL_SET,
            &t,
            &Fp::Null,
            0,
            1,
            "read_all + mail hit",
        );
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 27 =================== */

#[test]
fn cfg27_readfilemon_flags0_miss() {
    let _g = guard();
    unsafe {
        write_file(ALERTS_DAILY, full_alert("36.1", true, "ossec,").as_bytes());
        for (mday, mon, year) in [(1, 0, 100), (31, 11, 199), (28, 6, 0)] {
            let t = tm::new(mday, mon, year);
            // flags==0 -> fp is at EOF, so the read misses; the queue is then
            // re-opened (again seeked to EOF) and, with timeout 0, NULL comes
            // back without any file_sleep().
            diff_mon(0, &t, &Fp::Null, 0, 1, "flags0 miss");
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 28 =================== */

#[test]
fn cfg28_readfilemon_mail_filter() {
    let _g = guard();
    unsafe {
        for (label, body) in [
            ("mail only", full_alert("37.1", true, "ossec,")),
            ("nomail only", full_alert("37.2", false, "ossec,")),
            (
                "mixed",
                format!(
                    "{}{}",
                    full_alert("37.3", false, "ossec,"),
                    full_alert("37.4", true, "syscheck,")
                ),
            ),
        ] {
            write_file(ALERTS_DAILY, body.as_bytes());
            let t = tm::new(2, 1, 101);
            diff_mon(
                CRALERT_READ_ALL | CRALERT_MAIL_SET,
                &t,
                &Fp::Null,
                0,
                4,
                label,
            );
            diff_mon(CRALERT_READ_ALL, &t, &Fp::Null, 0, 4, label);
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 29 =================== */

#[test]
fn cfg29_readfilemon_repeated() {
    let _g = guard();
    let mut rng = Rng::new(0x2929);
    unsafe {
        for case in 0..40 {
            let n = 1 + rng.below(4);
            let mut body = String::new();
            for i in 0..n {
                body.push_str(&full_alert(
                    &format!("38.{case}.{i}"),
                    rng.bool(),
                    if rng.bool() { "syscheck," } else { "ossec," },
                ));
            }
            write_file(ALERTS_DAILY, body.as_bytes());
            let t = tm::new(rng.range_i32(1, 31), rng.range_i32(0, 11), rng.range_i32(0, 200));
            diff_mon(
                CRALERT_READ_ALL,
                &t,
                &Fp::Null,
                0,
                6,
                &format!("repeated {case}"),
            );
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 30 =================== */

#[test]
fn cfg30_readfilemon_fp_set_stdin_file() {
    let _g = guard();
    unsafe {
        // `Read_FileMon` hard-codes Handle_Queue(fileq, 0), so it re-opens the
        // *name* produced by GetFile_Queue, which is "<stdin>" when FP_SET is
        // in fileq->flags. Create that file so the re-open succeeds and no
        // file_sleep() happens.
        let payload = full_alert("39.1", true, "syscheck,");
        write_file(STDIN_NAME, payload.as_bytes());
        write_file(ALERTS_DAILY, payload.as_bytes());
        let name = scratch("fpmon", payload.as_bytes());
        let t = tm::new(4, 8, 130);
        for flags in [
            CRALERT_FP_SET,
            CRALERT_FP_SET | CRALERT_READ_ALL,
            CRALERT_FP_SET | CRALERT_MAIL_SET,
            CRALERT_FP_SET | CRALERT_READ_ALL | CRALERT_MAIL_SET,
        ] {
            diff_mon(flags, &t, &Fp::File(name.clone()), 0, 3, "fp_set mon");
            diff_mon(flags, &t, &Fp::Null, 0, 3, "fp_set mon, null fp");
        }
        remove_file(STDIN_NAME);
        remove_file(ALERTS_DAILY);
        let _ = std::fs::remove_file(&name);
    }
}

/* =================== CONFIGS row 31 =================== */

#[test]
fn cfg31_readfilemon_tm_fields() {
    let _g = guard();
    let mut rng = Rng::new(0x3131);
    unsafe {
        write_file(ALERTS_DAILY, b"nothing parseable\n");
        for _ in 0..120 {
            let t = tm::new(rng.range_i32(-100, 100), rng.range_i32(0, 11), rng.i32() >> 8);
            diff_mon(0, &t, &Fp::Null, 0, 1, "tm fields");
        }
        // the miss path writes day/year/mon into the queue
        for m in 0..12 {
            let t = tm::new(21, m, 77);
            let c = run_mon(cc(), 0, &t, &Fp::Null, 0, 1);
            let r = run_mon(rs(), 0, &t, &Fp::Null, 0, 1);
            assert_eq!(c, r, "tm month {m}");
            assert_eq!(&c.q_after.mon[..3], &MONTHS[m as usize][..]);
            assert_eq!(c.q_after.day, 21);
            assert_eq!(c.q_after.year, 1977);
        }
        remove_file(ALERTS_DAILY);
    }
}

/* =================== CONFIGS row 39 =================== */

/// `Read_FileMon` used as a genuinely low-level entry point: the `file_queue`
/// is hand-built by the caller instead of coming from `Init_FileQueue`.
#[test]
fn cfg39_readfilemon_handcrafted_queue() {
    let _g = guard();
    let mut rng = Rng::new(0x3939);
    unsafe {
        let body = format!(
            "{}{}",
            full_alert("60.1", true, "syscheck,"),
            full_alert("60.2", false, "ossec,")
        );
        write_file(ALERTS_DAILY, body.as_bytes());
        write_file(STDIN_NAME, body.as_bytes());
        let src = scratch("hand", body.as_bytes());

        // (a) fp == NULL and a caller-chosen file_name: the first Handle_Queue
        //     opens *that* name, then GetFile_Queue overwrites it.
        // (b) fp already open at position 0: the first GetAlertData hits.
        for variant in 0..4 {
            for flags in [
                0,
                CRALERT_READ_ALL,
                CRALERT_MAIL_SET,
                CRALERT_FP_SET,
                CRALERT_FP_SET | CRALERT_READ_ALL,
                rng.i32(),
            ] {
                let mut outs = Vec::new();
                for api in [cc(), rs()] {
                    let q = Box::leak(Box::new(file_queue::zeroed())) as *mut file_queue;
                    (*q).flags = flags;
                    (*q).day = 17;
                    (*q).year = 1999;
                    (*q).mon = [b'X' as _, b'Y' as _, b'Z' as _, 0];
                    (*q).last_change = 12345;
                    let name = cpath(&src);
                    std::ptr::copy_nonoverlapping(
                        name.as_ptr(),
                        (*q).file_name.as_mut_ptr(),
                        name.len(),
                    );
                    (*q).fp = match variant {
                        0 => std::ptr::null_mut(),
                        1 => open_ro(&src),
                        2 => {
                            let f = open_ro(&src);
                            fseek(f, 0, SEEK_END);
                            f
                        }
                        _ => make_fp(&Fp::Pipe(body.as_bytes().to_vec())),
                    };
                    let t = tm::new(5, 2, 103);
                    let mut alerts = Vec::new();
                    let (_, err) = capture_stderr(|| {
                        for _ in 0..3 {
                            set_errno(0);
                            let a = (api.Read_FileMon)(q, &t, 0);
                            let s = snap_alert(a);
                            if !a.is_null() {
                                (api.FreeAlertData)(a);
                            }
                            let done = s.is_none();
                            alerts.push(s);
                            if done {
                                break;
                            }
                        }
                    });
                    outs.push((
                        alerts,
                        snap_queue(q),
                        snap_stream((*q).fp),
                        String::from_utf8_lossy(&err).to_string(),
                    ));
                    close_queue(q);
                }
                assert_eq!(
                    outs[0], outs[1],
                    "handcrafted queue variant={variant} flags={flags:#x} differs"
                );
            }
        }
        remove_file(ALERTS_DAILY);
        remove_file(STDIN_NAME);
        let _ = std::fs::remove_file(&src);
    }
}

/* =================== CONFIGS row 40 =================== */

/// Re-initialising an already initialised queue, and interleaving
/// `Init_FileQueue` / `Read_FileMon` calls on the same struct.
#[test]
fn cfg40_init_then_reinit_and_interleave() {
    let _g = guard();
    unsafe {
        let body = full_alert("61.1", true, "syscheck,");
        write_file(ALERTS_DAILY, body.as_bytes());
        write_file(STDIN_NAME, body.as_bytes());
        let src = scratch("reinit", body.as_bytes());
        let seqs: [&[(u8, c_int)]; 6] = [
            &[(0, 0), (0, 0)],
            &[(0, CRALERT_READ_ALL), (0, CRALERT_READ_ALL)],
            &[(0, CRALERT_READ_ALL), (1, 0), (0, 0)],
            &[(0, 0), (1, 0), (0, CRALERT_READ_ALL), (1, 0)],
            &[(0, CRALERT_FP_SET), (1, 0), (0, CRALERT_FP_SET | CRALERT_READ_ALL)],
            &[(1, 0), (0, CRALERT_READ_ALL), (1, 0), (1, 0)],
        ];
        for (i, seq) in seqs.iter().enumerate() {
            let mut outs = Vec::new();
            for api in [cc(), rs()] {
                let q = Box::leak(Box::new(file_queue::zeroed())) as *mut file_queue;
                (*q).fp = open_ro(&src);
                let t = tm::new(11, 4, 108);
                let mut log: Vec<String> = Vec::new();
                let (_, err) = capture_stderr(|| {
                    for &(op, flags) in seq.iter() {
                        set_errno(0);
                        if op == 0 {
                            let rc = (api.Init_FileQueue)(q, &t, flags);
                            log.push(format!("init({flags:#x})={rc} {:?}", snap_queue(q)));
                        } else {
                            let a = (api.Read_FileMon)(q, &t, 0);
                            log.push(format!("mon={:?} {:?}", snap_alert(a), snap_queue(q)));
                            if !a.is_null() {
                                (api.FreeAlertData)(a);
                            }
                        }
                    }
                });
                outs.push((log, snap_stream((*q).fp), String::from_utf8_lossy(&err).to_string()));
                close_queue(q);
            }
            assert_eq!(outs[0], outs[1], "interleave sequence {i} differs");
        }
        remove_file(ALERTS_DAILY);
        remove_file(STDIN_NAME);
        let _ = std::fs::remove_file(&src);
    }
}

/* =================== ERRORS row 18 =================== */

#[test]
fn err18_init_no_alerts_log() {
    let _g = guard();
    unsafe {
        remove_file(ALERTS_DAILY);
        for flags in [0, CRALERT_READ_ALL, CRALERT_MAIL_SET, CRALERT_EXEC_SET, CRALERT_READ_FAILED] {
            let t = tm::new(1, 0, 100);
            let (co, cq) = run_init(cc(), flags, &t, &Fp::Null, None);
            let (ro, rq) = run_init(rs(), flags, &t, &Fp::Null, None);
            assert_eq!(co, ro, "missing alerts.log, flags={flags:#x}");
            assert_eq!(co.rc, 0, "fopen failure is NOT an Init error");
            assert!(co.q.fp_null);
            assert_eq!(co.q.last_change, 0);
            assert_eq!(co.err, "");
            close_queue(cq);
            close_queue(rq);
        }
    }
}

/* =================== ERRORS row 19 =================== */

#[test]
fn err19_fpset_null_fp() {
    let _g = guard();
    unsafe {
        remove_file(ALERTS_DAILY);
        for flags in [CRALERT_FP_SET, CRALERT_FP_SET | CRALERT_MAIL_SET] {
            let t = tm::new(1, 0, 100);
            let (co, cq) = run_init(cc(), flags, &t, &Fp::Null, None);
            let (ro, rq) = run_init(rs(), flags, &t, &Fp::Null, None);
            assert_eq!(co, ro, "FP_SET with NULL fp, flags={flags:#x}");
            assert_eq!(co.rc, 0);
            assert!(co.q.fp_null);
            assert_eq!(co.q.file_name, b"<stdin>".to_vec());
            close_queue(cq);
            close_queue(rq);
        }
        // FP_SET | READ_ALL with a NULL fp skips both the fseek and the fstat.
        let t = tm::new(1, 0, 100);
        let flags = CRALERT_FP_SET | CRALERT_READ_ALL;
        let (co, cq) = run_init(cc(), flags, &t, &Fp::Null, None);
        let (ro, rq) = run_init(rs(), flags, &t, &Fp::Null, None);
        assert_eq!(co, ro);
        assert_eq!(co.rc, 0);
        close_queue(cq);
        close_queue(rq);
    }
}

/* =================== ERRORS row 20 =================== */

#[test]
fn err20_init_fseek_fails_returns_minus1() {
    let _g = guard();
    unsafe {
        let data = full_alert("40.1", true, "ossec,").into_bytes();
        let flags = CRALERT_FP_SET; // no READ_ALL => fseek(fp,0,SEEK_END)
        let t = tm::new(1, 0, 100);
        let (co, cq) = run_init(cc(), flags, &t, &Fp::Pipe(data.clone()), None);
        let (ro, rq) = run_init(rs(), flags, &t, &Fp::Pipe(data.clone()), None);
        assert_eq!(co, ro, "fseek failure path differs");
        assert_eq!(co.rc, -1, "fseek failure must yield -1");
        assert!(co.q.fp_null, "fp must be closed and NULLed");
        assert!(
            co.err.contains("(1116): Could not set position in file '<stdin>'"),
            "unexpected stderr {:?}",
            co.err
        );
        assert!(co.err.contains("Illegal seek"), "expected ESPIPE: {:?}", co.err);
        close_queue(cq);
        close_queue(rq);
    }
}

/* =================== ERRORS row 21 =================== */

#[test]
fn err21_init_fstat_fails_returns_minus1() {
    let _g = guard();
    unsafe {
        let name = scratch("badfd", b"whatever\n");
        // READ_ALL skips the fseek so that the fstat is the first syscall to
        // hit the closed descriptor.
        let flags = CRALERT_FP_SET | CRALERT_READ_ALL;
        let t = tm::new(1, 0, 100);
        let (co, cq) = run_init(cc(), flags, &t, &Fp::ClosedFd(name.clone()), None);
        let (ro, rq) = run_init(rs(), flags, &t, &Fp::ClosedFd(name.clone()), None);
        assert_eq!(co, ro, "fstat failure path differs");
        assert_eq!(co.rc, -1, "fstat failure must yield -1");
        assert!(co.q.fp_null);
        assert!(
            co.err
                .contains("(1118): Could not retrieve information of file '<stdin>'"),
            "unexpected stderr {:?}",
            co.err
        );
        assert!(
            co.err.contains("Bad file descriptor"),
            "expected EBADF: {:?}",
            co.err
        );
        close_queue(cq);
        close_queue(rq);
        let _ = std::fs::remove_file(&name);
    }
}

/* =================== ERRORS row 22 (costs 2 x FQ_TIMEOUT) =================== */

#[test]
fn err22_readfilemon_queue_unavailable() {
    let _g = guard();
    unsafe {
        remove_file(ALERTS_DAILY);
        let t = tm::new(1, 0, 100);
        let start = std::time::Instant::now();
        diff_mon(0, &t, &Fp::Null, 0, 1, "queue unavailable");
        let c = run_mon(cc(), 0, &t, &Fp::Null, 0, 1);
        assert_eq!(c.alerts, vec![None]);
        assert!(
            start.elapsed().as_secs() >= 5,
            "file_sleep() should have been hit"
        );
    }
}

/* =================== ERRORS row 23 =================== */

#[test]
fn err23_readfilemon_null_fp_unreachable() {
    // `file-queue.c:156-158` can never fire: it is only reached when
    // `Handle_Queue(fileq, 0)` returned 1, which requires a successful `fopen`,
    // which guarantees a non-NULL `fp`. Both translations keep the branch, so
    // there is nothing observable to compare. Documented in ERRORS.md row 23.
    // The reachable neighbours are covered by err18/err19/err22.
    let _g = guard();
    unsafe {
        remove_file(ALERTS_DAILY);
        write_file(ALERTS_DAILY, b"");
        let t = tm::new(1, 0, 100);
        diff_mon(0, &t, &Fp::Null, 0, 1, "null fp neighbour");
        remove_file(ALERTS_DAILY);
    }
}

/* =================== ERRORS row 24 (costs 2 x FQ_TIMEOUT) =================== */

#[test]
fn err24_readfilemon_file_vanishes() {
    let _g = guard();
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc_geteuid() } == 0 {
        eprintln!("skipping err24: running as root, chmod 000 would not deny access");
        return;
    }
    unsafe {
        // `Init` opens the queue file; it then becomes unopenable, so the
        // *second* `Handle_Queue` inside `Read_FileMon` fails and the sleep +
        // NULL path is taken. Permission changes (rather than deletion) keep the
        // inode, size and mtime identical across the C and the Rust run.
        write_file(ALERTS_DAILY, b"no alert here\n");
        let t = tm::new(1, 0, 100);
        let mut outs = Vec::new();
        for api in [cc(), rs()] {
            std::fs::set_permissions(ALERTS_DAILY, std::fs::Permissions::from_mode(0o644)).unwrap();
            let (init, q) = run_init(api, CRALERT_READ_ALL, &t, &Fp::Null, None);
            std::fs::set_permissions(ALERTS_DAILY, std::fs::Permissions::from_mode(0o000)).unwrap();
            let (a, err) = capture_stderr(|| {
                let a = (api.Read_FileMon)(q, &t, 0);
                let s = snap_alert(a);
                if !a.is_null() {
                    (api.FreeAlertData)(a);
                }
                s
            });
            outs.push((
                init,
                a,
                snap_queue(q),
                snap_stream((*q).fp),
                String::from_utf8_lossy(&err).to_string(),
            ));
            close_queue(q);
        }
        std::fs::set_permissions(ALERTS_DAILY, std::fs::Permissions::from_mode(0o644)).unwrap();
        remove_file(ALERTS_DAILY);
        assert_eq!(outs[0], outs[1], "vanishing queue file path differs");
        assert_eq!(outs[0].1, None);
        assert!(outs[0].2.fp_null, "fp must have been closed");
    }
}

extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

/* =================== ERRORS row 25 =================== */

#[test]
fn err25_readfilemon_timeout_zero() {
    let _g = guard();
    unsafe {
        write_file(ALERTS_DAILY, b"nothing parseable\n");
        let t = tm::new(1, 0, 100);
        let start = std::time::Instant::now();
        diff_mon(0, &t, &Fp::Null, 0, 1, "timeout 0");
        diff_mon(CRALERT_READ_ALL, &t, &Fp::Null, 0, 1, "timeout 0 read_all");
        assert!(
            start.elapsed().as_secs() < 5,
            "timeout==0 must not sleep at all"
        );
        remove_file(ALERTS_DAILY);
    }
}

/* =================== ERRORS row 27 =================== */

#[test]
fn err27_out_of_range_month_is_ub_in_c() {
    let _g = guard();
    write_file(ALERTS_DAILY, b"");
    // `s_month[p->tm_mon]` is read without any bounds check, so a month outside
    // 0..=11 is undefined behaviour in the C. On this build `s_month[12]` falls
    // into .bss and is NULL, which makes the C `strncpy` crash; the Rust
    // translation range-guards the copy instead. Prove the C really is UB here
    // (so there is no defined behaviour to match) and that the Rust survives.
    let c = run_worker("c:oob_month");
    assert!(
        c.signal.is_some(),
        "expected the C to die on the out-of-range s_month read, got {c:#?}"
    );
    let r = run_worker("rust:oob_month");
    assert!(
        r.signal.is_none() && r.status == Some(0),
        "the Rust guard should keep it alive, got {r:#?}"
    );
    // Every in-range month, on the other hand, must match exactly.
    unsafe {
        for m in 0..12 {
            let t = tm::new(1, m, 100);
            diff_init(0, &t, &Fp::Null, None, &format!("in-range month {m}"));
        }
    }
    remove_file(ALERTS_DAILY);
}

/* =================== sub-process worker =================== */

#[test]
fn zz_subprocess_worker() {
    let Some(action) = worker_action() else {
        return;
    };
    let (api, rest) = worker_api(&action);
    unsafe {
        match rest {
            "oob_month" => {
                let mut q = file_queue::zeroed();
                let t = tm::new(1, 12, 100);
                let rc = (api.Init_FileQueue)(&mut q, &t, 0);
                if !q.fp.is_null() {
                    fclose(q.fp);
                }
                emit(&format!("rc={rc}"));
            }
            other => panic!("unknown worker action {other:?}"),
        }
    }
    std::process::exit(0);
}
