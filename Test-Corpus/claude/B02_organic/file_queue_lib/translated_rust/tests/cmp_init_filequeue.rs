//! Compare Init_FileQueue / Read_FileMon / driver outputs between C and Rust.

mod common;

use common::*;

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::path::PathBuf;

const MAX_FQUEUE: usize = 256;

#[repr(C)]
struct FileQueue {
    last_change: libc::time_t,
    year: c_int,
    day: c_int,
    flags: c_int,
    mon: [c_char; 4],
    file_name: [c_char; MAX_FQUEUE + 1],
    fp: *mut libc::FILE,
    f_status: libc::stat,
}

unsafe fn make_zero_fq() -> Box<FileQueue> {
    unsafe {
        let layout = std::alloc::Layout::new::<FileQueue>();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut FileQueue;
        Box::from_raw(ptr)
    }
}

fn make_tm(day: c_int, month: c_int, year: c_int) -> libc::tm {
    libc::tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: day,
        tm_mon: month,
        tm_year: year,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    }
}

fn run_in_dir<F: FnOnce(&PathBuf)>(dir: &PathBuf, f: F) {
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let _g = scopeguard_chdir(prev);
    f(dir);
}

struct ChDirGuard(PathBuf);
impl Drop for ChDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}
fn scopeguard_chdir(p: PathBuf) -> ChDirGuard {
    ChDirGuard(p)
}

fn unique_tmp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "harvest_init_fq_{}_{}_{:?}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn init_filequeue_no_file_present() {
    // Without alerts.log present, Init_FileQueue calls Handle_Queue which
    // returns 0 (file not openable, fseek/fstat blocks skipped because fp is NULL).
    // last_change should remain 0 (since f_status is zeroed and we never call fstat).
    let dir = unique_tmp_dir("missing");

    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnInitFileQueue> = sym(&c, b"Init_FileQueue");
        let rf: libloading::Symbol<FnInitFileQueue> = sym(&r, b"Init_FileQueue");

        let mut tm = make_tm(15, 6, 124); // 2024-07-15
        let mut cq = make_zero_fq();
        let mut rq = make_zero_fq();

        run_in_dir(&dir, |_| {
            let cret = cf(&mut *cq as *mut FileQueue as *mut c_void, &mut tm, 0);
            let rret = rf(&mut *rq as *mut FileQueue as *mut c_void, &mut tm, 0);
            assert_eq!(cret, rret, "Init_FileQueue return value mismatch");
        });

        assert_eq!(cq.day, rq.day);
        assert_eq!(cq.year, rq.year);
        assert_eq!(cq.flags, rq.flags);
        assert_eq!(cq.last_change, rq.last_change);
        let cmon = std::slice::from_raw_parts(cq.mon.as_ptr() as *const u8, 3);
        let rmon = std::slice::from_raw_parts(rq.mon.as_ptr() as *const u8, 3);
        assert_eq!(cmon, rmon);
        let cname = libc::strlen(cq.file_name.as_ptr());
        let rname = libc::strlen(rq.file_name.as_ptr());
        assert_eq!(cname, rname);
        let cn = std::slice::from_raw_parts(cq.file_name.as_ptr() as *const u8, cname);
        let rn = std::slice::from_raw_parts(rq.file_name.as_ptr() as *const u8, rname);
        assert_eq!(cn, rn);

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn init_filequeue_with_file_present_read_all() {
    let dir = unique_tmp_dir("read_all");
    std::fs::write(dir.join("alerts.log"), b"** Alert 1.1: mail - g\n").unwrap();

    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnInitFileQueue> = sym(&c, b"Init_FileQueue");
        let rf: libloading::Symbol<FnInitFileQueue> = sym(&r, b"Init_FileQueue");

        let mut tm = make_tm(15, 6, 124);
        let mut cq = make_zero_fq();
        let mut rq = make_zero_fq();

        run_in_dir(&dir, |_| {
            // CRALERT_READ_ALL = 0x004
            let cret = cf(&mut *cq as *mut FileQueue as *mut c_void, &mut tm, 0x004);
            let rret = rf(&mut *rq as *mut FileQueue as *mut c_void, &mut tm, 0x004);
            assert_eq!(cret, rret, "Init_FileQueue return value mismatch");
        });

        assert_eq!(cq.day, rq.day);
        assert_eq!(cq.year, rq.year);
        assert_eq!(cq.flags, rq.flags);
        // last_change should now be set to the mtime of the file (same file, same mtime
        // for both calls, modulo time resolution — since we call them right after each
        // other, they should observe the same mtime).
        assert_eq!(cq.last_change, rq.last_change);

        if !cq.fp.is_null() {
            libc::fclose(cq.fp);
            cq.fp = std::ptr::null_mut();
        }
        if !rq.fp.is_null() {
            libc::fclose(rq.fp);
            rq.fp = std::ptr::null_mut();
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn driver_returns_null_when_no_log() {
    // No alerts.log file, no FP_SET, so initial Handle_Queue fails (returns 0),
    // Init_FileQueue returns 0 (only -1 on fseek/fstat error). Then Read_FileMon
    // tries to handle the queue again — also fails — and after timeout iterations
    // returns NULL.
    let dir = unique_tmp_dir("driver_null");

    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnDriver> = sym(&c, b"driver");
        let rf: libloading::Symbol<FnDriver> = sym(&r, b"driver");

        run_in_dir(&dir, |_| {
            // Use timeout=0 to avoid the 5-second sleep inside Read_FileMon.
            let cret = cf(15, 6, 124, 0, 0);
            let rret = rf(15, 6, 124, 0, 0);
            // When timeout=0 and fp can't be opened, Handle_Queue returns 0,
            // file_sleep is called once, then NULL is returned from Read_FileMon.
            assert!(cret.is_null());
            assert!(rret.is_null());
        });

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn driver_with_alert_in_log_read_all_no_mail() {
    let dir = unique_tmp_dir("driver_alert");
    let log = b"** Alert 1500000000.123: syslog - testgroup\n\
                2017 Jul 14 10:00:00 (host) src->/var/log/auth.log\n\
                Rule: 100 (level 3) -> 'msg'\n\
                Src IP: 1.2.3.4\n\
                Src Port: 22\n\
                User: bob\n\
                ** Alert 1500000001.500: syslog - other\n";
    std::fs::write(dir.join("alerts.log"), log).unwrap();

    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnDriver> = sym(&c, b"driver");
        let rf: libloading::Symbol<FnDriver> = sym(&r, b"driver");
        let c_free: libloading::Symbol<FnFreeAlertData> = sym(&c, b"FreeAlertData");
        let r_free: libloading::Symbol<FnFreeAlertData> = sym(&r, b"FreeAlertData");

        let (c_snap, r_snap) = run_two_drivers(&dir, &cf, &rf);

        // Make sure we got a non-NULL result and it matches.
        assert!(c_snap.is_some(), "C driver returned NULL");
        assert_eq!(c_snap, r_snap);
        // We got the snapshot already, but we also need to free the actual returned alerts.
        // Re-run to obtain pointers we can free.
        let (cret, rret) = run_two_drivers_raw(&dir, &cf, &rf);
        if !cret.is_null() {
            c_free(cret);
        }
        if !rret.is_null() {
            r_free(rret);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

unsafe fn run_two_drivers(
    dir: &PathBuf,
    cf: &libloading::Symbol<FnDriver>,
    rf: &libloading::Symbol<FnDriver>,
) -> (Option<AlertDataSnapshot>, Option<AlertDataSnapshot>) {
    unsafe {
        let mut c_snap = None;
        let mut r_snap = None;
        run_in_dir(dir, |_| {
            // CRALERT_READ_ALL = 0x004
            let cret = cf(15, 6, 124, 1, 0x004);
            let rret = rf(15, 6, 124, 1, 0x004);
            c_snap = snapshot_alert(cret);
            r_snap = snapshot_alert(rret);
            // free here so we don't leak
            if !cret.is_null() {
                let lib = load_c();
                let f: libloading::Symbol<FnFreeAlertData> = sym(&lib, b"FreeAlertData");
                f(cret);
            }
            if !rret.is_null() {
                let lib = load_rust();
                let f: libloading::Symbol<FnFreeAlertData> = sym(&lib, b"FreeAlertData");
                f(rret);
            }
        });
        (c_snap, r_snap)
    }
}

unsafe fn run_two_drivers_raw(
    dir: &PathBuf,
    cf: &libloading::Symbol<FnDriver>,
    rf: &libloading::Symbol<FnDriver>,
) -> (*mut AlertData, *mut AlertData) {
    unsafe {
        let mut cret: *mut AlertData = std::ptr::null_mut();
        let mut rret: *mut AlertData = std::ptr::null_mut();
        run_in_dir(dir, |_| {
            cret = cf(15, 6, 124, 1, 0x004);
            rret = rf(15, 6, 124, 1, 0x004);
        });
        (cret, rret)
    }
}
