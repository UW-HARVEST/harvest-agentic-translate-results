//! Level 3/4: `Init_FileQueue` and `Read_FileMon` from `src/file-queue.c`, and
//! the `driver` entry point from `src/driver.c`.
//!
//! These functions open `alerts.log` relative to the process working directory,
//! so each test takes the global lock and chdir's into its own scratch dir.

mod common;

use common::*;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_int, c_long, c_uint};
use std::path::{Path, PathBuf};

/// Everything observable after an `Init_FileQueue` / `Read_FileMon` call.
#[derive(Debug, PartialEq, Eq, Clone)]
struct QueueResult {
    ret: c_int,
    queue: QueueSnap,
    pos: Option<c_long>,
    eof: Option<bool>,
    err: Option<bool>,
}

fn capture_stderr<F: FnOnce() -> R, R>(f: F) -> (R, Vec<u8>) {
    unsafe {
        let path = std::env::temp_dir().join(format!(
            "c2rust-stderr3-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut tmp = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open capture file");
        use std::os::fd::AsRawFd;
        let cap_fd = tmp.as_raw_fd();

        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(2);
        assert!(saved >= 0);
        assert!(libc::dup2(cap_fd, 2) >= 0);

        let r = f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 2);
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut out = Vec::new();
        tmp.read_to_end(&mut out).expect("read");
        let _ = std::fs::remove_file(&path);
        (r, out)
    }
}

/// RAII working-directory switch.
struct Cwd(PathBuf);

impl Cwd {
    fn enter(dir: &Path) -> Cwd {
        let old = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir).expect("chdir");
        Cwd(old)
    }
}

impl Drop for Cwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn tm(day: c_int, mon: c_int, year: c_int) -> libc::tm {
    let mut t: libc::tm = unsafe { std::mem::zeroed() };
    t.tm_mday = day;
    t.tm_mon = mon;
    t.tm_year = year;
    t
}

unsafe fn zeroed_queue() -> file_queue {
    std::mem::zeroed()
}

unsafe fn run_init(imp: &Impl, t: &libc::tm, flags: c_int, preset: Option<*mut libc::FILE>) -> QueueResult {
    let mut fq = zeroed_queue();
    if let Some(fp) = preset {
        fq.fp = fp;
    }
    *libc::__errno_location() = 0;
    let ret = (imp.Init_FileQueue)(&mut fq, t, flags);
    let mut res = QueueResult {
        ret,
        queue: snap_queue(&fq),
        pos: None,
        eof: None,
        err: None,
    };
    if !fq.fp.is_null() {
        let s = snap_stream(fq.fp);
        res.pos = Some(s.pos);
        res.eof = Some(s.eof);
        res.err = Some(s.err);
        libc::fclose(fq.fp);
    }
    res
}

// ---------------------------------------------------------------------------
// Init_FileQueue
// ---------------------------------------------------------------------------

fn init_case(tag: &str, contents: Option<&[u8]>, t: &libc::tm, flags: c_int) {
    let p = pair();
    let dir = TempDir::new("initfq");
    if let Some(c) = contents {
        dir.file("alerts.log", c);
    }

    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    let (rc, err_c) = capture_stderr(|| unsafe { run_init(&p.c, t, flags, None) });
    let (rr, err_rs) = capture_stderr(|| unsafe { run_init(&p.rs, t, flags, None) });

    assert_eq!(
        rc, rr,
        "[{tag} flags={flags:#x}] Init_FileQueue result differs\nC:    {rc:#?}\nRust: {rr:#?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_rs),
        "[{tag} flags={flags:#x}] stderr differs"
    );
}

const ALERT: &[u8] = b"** Alert 1755787624.1234: mail - syscheck,pci_dss_11.5,\n\
2025 Aug 21 13:27:04 (agent-01) 10.0.0.1->syscheck\n\
Rule: 550 (level 7) -> 'Integrity checksum changed.'\n\
Src IP: 192.168.1.10\n\
Src Port: 4242\n\
Dst IP: 10.1.2.3\n\
Dst Port: 80\n\
User: root\n\
Integrity checksum changed for: '/etc/passwd'\n";

const ALL_FLAGS: &[c_int] = &[
    0,
    CRALERT_MAIL_SET,
    CRALERT_EXEC_SET,
    CRALERT_READ_ALL,
    CRALERT_READ_FAILED,
    CRALERT_FP_SET,
    CRALERT_FP_SET | CRALERT_READ_ALL,
    CRALERT_MAIL_SET | CRALERT_READ_ALL,
    CRALERT_FP_SET | CRALERT_MAIL_SET,
    0x1f,
];

#[test]
fn init_file_queue_all_flags_with_file() {
    let t = tm(21, 7, 125);
    for &f in ALL_FLAGS {
        init_case("with-file", Some(ALERT), &t, f);
    }
}

#[test]
fn init_file_queue_all_flags_missing_file() {
    let t = tm(1, 0, 100);
    for &f in ALL_FLAGS {
        init_case("missing-file", None, &t, f);
    }
}

#[test]
fn init_file_queue_empty_file() {
    let t = tm(31, 11, 200);
    for f in [0, CRALERT_READ_ALL, CRALERT_FP_SET] {
        init_case("empty-file", Some(b""), &t, f);
    }
}

#[test]
fn init_file_queue_every_month_and_odd_dates() {
    // `strncpy(fileq->mon, s_month[p->tm_mon], 3)` for each valid month, plus
    // the `tm_year + 1900` arithmetic including negative and overflowing years.
    for mon in 0..12 {
        init_case("months", Some(ALERT), &tm(15, mon, 125), 0);
    }
    for (day, year) in [
        (0, 0),
        (-1, -1900),
        (31, -2000),
        (999, 99999),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX - 1900),
    ] {
        init_case("odd-dates", Some(ALERT), &tm(day, 0, year), 0);
    }
}

#[test]
fn init_file_queue_with_preset_fp() {
    // CRALERT_FP_SET keeps a caller-supplied stream: it must be seeked to the
    // end and fstat'ed rather than reopened.
    let p = pair();
    let dir = TempDir::new("presetfp");
    let path = dir.file("input.log", ALERT);
    let t = tm(21, 7, 125);

    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    for &flags in &[CRALERT_FP_SET, CRALERT_FP_SET | CRALERT_READ_ALL] {
        let (rc, err_c) = capture_stderr(|| unsafe {
            let fp = fopen(&path, b"r");
            run_init(&p.c, &t, flags, Some(fp))
        });
        let (rr, err_rs) = capture_stderr(|| unsafe {
            let fp = fopen(&path, b"r");
            run_init(&p.rs, &t, flags, Some(fp))
        });
        assert_eq!(rc, rr, "preset fp, flags={flags:#x}\nC: {rc:#?}\nRust: {rr:#?}");
        assert_eq!(
            String::from_utf8_lossy(&err_c),
            String::from_utf8_lossy(&err_rs)
        );
    }
}

#[test]
fn init_file_queue_unseekable_stream() {
    // fseek on a FIFO fails with ESPIPE, which drives the merror(FSEEK_ERROR)
    // branch and makes Init_FileQueue return -1.
    let p = pair();
    let dir = TempDir::new("fifo");
    let fifo = dir.0.join("alerts.log");
    let cpath = cstring(fifo.to_str().unwrap().as_bytes());
    unsafe {
        assert_eq!(libc::mkfifo(cpath.as_ptr(), 0o600), 0, "mkfifo failed");
    }
    // Hold the FIFO open read-write so the library's fopen("r") cannot block.
    let holder = unsafe { libc::open(cpath.as_ptr(), libc::O_RDWR) };
    assert!(holder >= 0);

    let t = tm(21, 7, 125);
    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    for &flags in &[0, CRALERT_MAIL_SET] {
        let (rc, err_c) = capture_stderr(|| unsafe { run_init(&p.c, &t, flags, None) });
        let (rr, err_rs) = capture_stderr(|| unsafe { run_init(&p.rs, &t, flags, None) });
        assert_eq!(rc, rr, "fifo init flags={flags:#x}\nC: {rc:#?}\nRust: {rr:#?}");
        assert_eq!(
            String::from_utf8_lossy(&err_c),
            String::from_utf8_lossy(&err_rs),
            "fifo init stderr differs (flags={flags:#x})"
        );
        assert!(!err_c.is_empty(), "expected an FSEEK error message");
        assert_eq!(rc.ret, -1, "expected Init_FileQueue to fail on a FIFO");
    }
    unsafe { libc::close(holder) };
}

#[test]
fn init_file_queue_directory_target() {
    // `alerts.log` is a directory: glibc's fopen succeeds but reads fail.
    let p = pair();
    let dir = TempDir::new("isdir");
    std::fs::create_dir(dir.0.join("alerts.log")).expect("mkdir");

    let t = tm(21, 7, 125);
    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    for &flags in &[0, CRALERT_READ_ALL] {
        let (rc, err_c) = capture_stderr(|| unsafe { run_init(&p.c, &t, flags, None) });
        let (rr, err_rs) = capture_stderr(|| unsafe { run_init(&p.rs, &t, flags, None) });
        assert_eq!(rc, rr, "dir init flags={flags:#x}\nC: {rc:#?}\nRust: {rr:#?}");
        assert_eq!(
            String::from_utf8_lossy(&err_c),
            String::from_utf8_lossy(&err_rs)
        );
    }
}

// ---------------------------------------------------------------------------
// Read_FileMon
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
struct ReadResult {
    alert: Option<AlertSnap>,
    queue: QueueSnap,
    pos: Option<c_long>,
}

unsafe fn run_read(
    imp: &Impl,
    t: &libc::tm,
    flags: c_int,
    timeout: c_uint,
    calls: usize,
) -> Vec<ReadResult> {
    let mut fq = zeroed_queue();
    *libc::__errno_location() = 0;
    (imp.Init_FileQueue)(&mut fq, t, flags);

    let mut out = Vec::new();
    for _ in 0..calls {
        *libc::__errno_location() = 0;
        let al = (imp.Read_FileMon)(&mut fq, t, timeout);
        let snap = snap_alert(al);
        let pos = if fq.fp.is_null() {
            None
        } else {
            Some(libc::ftell(fq.fp))
        };
        out.push(ReadResult {
            alert: snap,
            queue: snap_queue(&fq),
            pos,
        });
        if !al.is_null() {
            (imp.FreeAlertData)(al);
        }
    }
    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }
    out
}

fn read_case(
    tag: &str,
    contents: Option<&[u8]>,
    t: &libc::tm,
    flags: c_int,
    timeout: c_uint,
    calls: usize,
) {
    let p = pair();
    let dir = TempDir::new("readmon");
    if let Some(c) = contents {
        dir.file("alerts.log", c);
    }

    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    let (rc, err_c) = capture_stderr(|| unsafe { run_read(&p.c, t, flags, timeout, calls) });
    let (rr, err_rs) = capture_stderr(|| unsafe { run_read(&p.rs, t, flags, timeout, calls) });

    assert_eq!(
        rc, rr,
        "[{tag} flags={flags:#x} timeout={timeout}] Read_FileMon results differ\nC:    {rc:#?}\nRust: {rr:#?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_rs),
        "[{tag} flags={flags:#x}] stderr differs"
    );
}

#[test]
fn read_file_mon_read_all_returns_alerts() {
    // CRALERT_READ_ALL keeps the stream at offset 0, so the first GetAlertData
    // succeeds immediately and no sleeping happens.
    let t = tm(21, 7, 125);
    let mut two = ALERT.to_vec();
    two.extend_from_slice(
        b"** Alert 1755787625.9: mail - authentication_failed,\n\
2025 Aug 21 13:27:05 (agent-02) 10.0.0.2->/var/log/secure\n\
Rule: 5710 (level 5) -> 'Attempt to login'\n\
Src IP: 203.0.113.7\n",
    );
    read_case("read-all-one", Some(ALERT), &t, CRALERT_READ_ALL, 0, 1);
    read_case("read-all-two", Some(&two), &t, CRALERT_READ_ALL, 0, 2);
    read_case(
        "read-all-mail",
        Some(&two),
        &t,
        CRALERT_READ_ALL | CRALERT_MAIL_SET,
        0,
        2,
    );
}

#[test]
fn read_file_mon_seek_to_end_finds_nothing() {
    // Default flags seek to EOF, so nothing is found and (with timeout 0) the
    // retry loop is skipped entirely.
    let t = tm(21, 7, 125);
    read_case("seek-end", Some(ALERT), &t, 0, 0, 2);
    read_case("seek-end-empty", Some(b""), &t, 0, 0, 1);
    read_case("seek-end-junk", Some(b"no alerts here\n"), &t, 0, 0, 1);
}

#[test]
fn read_file_mon_every_month() {
    for mon in 0..12 {
        read_case("months", Some(ALERT), &tm(9, mon, 130), CRALERT_READ_ALL, 0, 1);
    }
}

#[test]
fn read_file_mon_fp_set_flags() {
    // CRALERT_FP_SET makes the queue name `<stdin>`, which cannot be opened, so
    // Read_FileMon's re-open attempt fails. Each failure costs a 5 s
    // file_sleep() per implementation, so only one call is made.
    let t = tm(21, 7, 125);
    read_case("fp-set", Some(ALERT), &t, CRALERT_FP_SET, 0, 1);
}

#[test]
fn read_file_mon_missing_file_sleeps() {
    // No alerts.log at all: Handle_Queue returns 0 and file_sleep() runs once
    // for each implementation (5 s each).
    let t = tm(21, 7, 125);
    read_case("missing", None, &t, 0, 0, 1);
}

#[test]
fn read_file_mon_timeout_retry_loop() {
    // timeout = 1 exercises one iteration of the retry loop (one 5 s sleep per
    // implementation).
    let t = tm(21, 7, 125);
    read_case("timeout-1", Some(ALERT), &t, 0, 1, 1);
}

// ---------------------------------------------------------------------------
// driver
// ---------------------------------------------------------------------------

fn driver_case(
    tag: &str,
    contents: Option<&[u8]>,
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) {
    let p = pair();
    let dir = TempDir::new("driver");
    if let Some(c) = contents {
        dir.file("alerts.log", c);
    }

    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    let (ac, err_c) = capture_stderr(|| unsafe {
        *libc::__errno_location() = 0;
        let a = (p.c.driver)(day, month, year, timeout, flags);
        let s = snap_alert(a);
        if !a.is_null() {
            (p.c.FreeAlertData)(a);
        }
        s
    });
    let (ar, err_rs) = capture_stderr(|| unsafe {
        *libc::__errno_location() = 0;
        let a = (p.rs.driver)(day, month, year, timeout, flags);
        let s = snap_alert(a);
        if !a.is_null() {
            (p.rs.FreeAlertData)(a);
        }
        s
    });

    assert_eq!(
        ac, ar,
        "[{tag} d={day} m={month} y={year} t={timeout} flags={flags:#x}] driver result differs\nC:    {ac:#?}\nRust: {ar:#?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_rs),
        "[{tag}] stderr differs"
    );
}

#[test]
fn driver_read_all_returns_alert() {
    driver_case("read-all", Some(ALERT), 21, 7, 125, 0, CRALERT_READ_ALL);
    driver_case(
        "read-all-mail",
        Some(ALERT),
        21,
        7,
        125,
        0,
        CRALERT_READ_ALL | CRALERT_MAIL_SET,
    );
    // Alert without the `mail` keyword combined with CRALERT_MAIL_SET.
    let nomail: &[u8] = b"** Alert 9.9: - grp,\n2025 Aug 21 13:27:04 loc\nRule: 1 (level 2) -> 'x'\n";
    driver_case(
        "read-all-nomail",
        Some(nomail),
        21,
        7,
        125,
        0,
        CRALERT_READ_ALL | CRALERT_MAIL_SET,
    );
    driver_case(
        "read-all-nomail-plain",
        Some(nomail),
        21,
        7,
        125,
        0,
        CRALERT_READ_ALL,
    );
}

#[test]
fn driver_default_flags_returns_null() {
    driver_case("default", Some(ALERT), 21, 7, 125, 0, 0);
    driver_case("default-empty", Some(b""), 1, 0, 0, 0, 0);
    driver_case("default-junk", Some(b"nothing to see\n"), 1, 0, 0, 0, 0);
}

#[test]
fn driver_every_month() {
    for mon in 0..12 {
        driver_case("months", Some(ALERT), 15, mon, 125, 0, CRALERT_READ_ALL);
    }
}

#[test]
fn driver_extreme_date_values() {
    for (day, year) in [
        (0, 0),
        (-1, -1900),
        (999, 99999),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX - 1900),
    ] {
        driver_case("extreme", Some(ALERT), day, 0, year, 0, CRALERT_READ_ALL);
    }
}

#[test]
fn driver_multi_alert_log() {
    let mut v: Vec<u8> = Vec::new();
    for i in 1..=4 {
        v.extend_from_slice(
            format!(
                "** Alert 175578762{i}.{i}: mail - group{i},syscheck,\n\
                 2025 Aug 2{i} 13:27:0{i} (agent-{i}) 10.0.0.{i}->/var/log/x{i}\n\
                 Rule: {i}00 (level {i}) -> 'message {i}'\n\
                 Src IP: 10.{i}.{i}.{i}\n\
                 Src Port: {}\n\
                 Dst IP: 172.16.0.{i}\n\
                 Dst Port: {}\n\
                 User: user{i}\n\
                 Integrity checksum changed for: '/etc/file{i}'\n",
                1000 + i,
                2000 + i
            )
            .as_bytes(),
        );
    }
    // Only the first alert is returned; driver closes the stream afterwards.
    driver_case("multi", Some(&v), 21, 7, 125, 0, CRALERT_READ_ALL);
}

#[test]
fn driver_missing_file() {
    driver_case("missing", None, 21, 7, 125, 0, 0);
}

#[test]
fn driver_fp_set_flag() {
    driver_case("fp-set", Some(ALERT), 21, 7, 125, 0, CRALERT_FP_SET);
}

#[test]
fn driver_init_failure_path() {
    // A FIFO makes Init_FileQueue fail, so driver prints its own message and
    // returns NULL without touching Read_FileMon.
    let p = pair();
    let dir = TempDir::new("driverfifo");
    let fifo = dir.0.join("alerts.log");
    let cpath = cstring(fifo.to_str().unwrap().as_bytes());
    unsafe {
        assert_eq!(libc::mkfifo(cpath.as_ptr(), 0o600), 0);
    }
    let holder = unsafe { libc::open(cpath.as_ptr(), libc::O_RDWR) };
    assert!(holder >= 0);

    let _g = lock();
    let _cwd = Cwd::enter(&dir.0);

    let (ac, err_c) = capture_stderr(|| unsafe {
        *libc::__errno_location() = 0;
        let a = (p.c.driver)(21, 7, 125, 0, 0);
        let s = snap_alert(a);
        if !a.is_null() {
            (p.c.FreeAlertData)(a);
        }
        s
    });
    let (ar, err_rs) = capture_stderr(|| unsafe {
        *libc::__errno_location() = 0;
        let a = (p.rs.driver)(21, 7, 125, 0, 0);
        let s = snap_alert(a);
        if !a.is_null() {
            (p.rs.FreeAlertData)(a);
        }
        s
    });

    unsafe { libc::close(holder) };

    assert_eq!(ac, None, "driver should return NULL when init fails");
    assert_eq!(ac, ar);
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        String::from_utf8_lossy(&err_rs),
        "driver init-failure stderr differs"
    );
    assert!(
        err_c.contains(&b'1'),
        "expected merror + driver diagnostics, got {:?}",
        String::from_utf8_lossy(&err_c)
    );
}
