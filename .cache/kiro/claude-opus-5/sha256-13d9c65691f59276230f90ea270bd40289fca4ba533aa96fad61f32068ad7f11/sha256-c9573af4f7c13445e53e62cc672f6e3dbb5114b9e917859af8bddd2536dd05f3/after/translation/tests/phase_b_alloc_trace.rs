//! Phase B/C supplement — differential comparison of **allocator behaviour**.
//!
//! Return values, stdout, stderr and the log file do not reveal whether the
//! translation allocates and frees the same things as the C.  A missing
//! `free(manager)` on `create_task_manager`'s failure path, an extra
//! allocation, or a `Vec` smuggled into a hot path are all invisible there but
//! are real divergences.
//!
//! `tests/common/mod.rs` interposes `malloc`/`free` for the whole process (the
//! test binary is linked with `-rdynamic`, so both dlopen'ed `.so`s resolve to
//! it), which makes the allocation sequence observable.  Each scenario is run
//! once untraced to warm up any lazy initialisation, then traced, and the C's
//! totals are compared with the Rust's.

mod common;

use common::{
    arm_malloc_failure, cstring, disarm_malloc_failure, serial, trace_start, trace_stop, AllocStats,
    Api, Rng,
};
use std::ffi::{c_char, c_int, c_void, CString};
use std::os::unix::io::AsRawFd;

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
}

/// Sends the library's stdout/stderr to `/dev/null` and points `$LOG_FILE`
/// there too, so a traced scenario produces no output and no files.
struct Quiet {
    saved_out: c_int,
    saved_err: c_int,
    _devnull: std::fs::File,
}

impl Quiet {
    fn new() -> Quiet {
        let devnull = std::fs::File::create("/dev/null").unwrap();
        let v = CString::new("/dev/null").unwrap();
        unsafe {
            setenv(c"LOG_FILE".as_ptr(), v.as_ptr(), 1);
            fflush(std::ptr::null_mut());
            let saved_out = dup(1);
            let saved_err = dup(2);
            dup2(devnull.as_raw_fd(), 1);
            dup2(devnull.as_raw_fd(), 2);
            Quiet {
                saved_out,
                saved_err,
                _devnull: devnull,
            }
        }
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved_out, 1);
            dup2(self.saved_err, 2);
            close(self.saved_out);
            close(self.saved_err);
        }
    }
}

/// Run `f` twice: once to warm up, once with the allocator traced.
fn measure(api: &Api, f: &dyn Fn(&Api)) -> AllocStats {
    f(api);
    unsafe { fflush(std::ptr::null_mut()) };
    trace_start();
    f(api);
    trace_stop()
}

fn assert_same_allocs(tag: &str, f: &dyn Fn(&Api)) {
    let _g = serial();
    let p = common::pair();
    let _q = Quiet::new();
    let c = measure(&p.c, f);
    let r = measure(&p.rust, f);
    drop(_q);
    assert_eq!(
        c, r,
        "allocator behaviour differs in `{tag}`: C={c:?} Rust={r:?}"
    );
    assert!(
        c.mallocs > 0,
        "`{tag}` did not allocate at all - the trace is vacuous"
    );
}

// ---------------------------------------------------------------------------

#[test]
fn alloc01_create_destroy_is_balanced() {
    // create_task_manager: malloc(16) + malloc(max_tasks*260)
    // destroy_task_manager: free(tasks) + free(manager)
    assert_same_allocs("alloc01", &|api| unsafe {
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        (api.destroy_task_manager)(m);
    });
}

#[test]
fn alloc02_full_low_level_pipeline() {
    let mut rng = Rng::new(0xA110C_01);
    let items: Vec<(Vec<u8>, i32)> = (0..9)
        .map(|_| {
            let n = rng.below(300);
            let body = rng.cstr_body(n);
            (cstring(&body), rng.priority())
        })
        .collect();
    assert_same_allocs("alloc02", &|api| unsafe {
        (api.initialize_logger)();
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        for (d, p) in &items {
            (api.add_task)(m, d.as_ptr() as *const c_char, *p as c_int);
        }
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

#[test]
fn alloc03_driver_end_to_end() {
    let mut rng = Rng::new(0xA110C_02);
    for i in 0..8 {
        let nlines = rng.range(1, 14);
        let blob = cstring(&rng.blob(nlines, 300));
        assert_same_allocs(&format!("alloc03-{i}"), &|api| unsafe {
            (api.driver)(blob.as_ptr() as *const c_char);
        });
    }
}

#[test]
fn alloc04_driver_over_capacity() {
    // Rejected lines are still malloc'ed and freed by driver.
    let mut rng = Rng::new(0xA110C_03);
    let blob = cstring(&rng.blob(40, 120));
    let v = CString::new("5").unwrap();
    unsafe { setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1) };
    assert_same_allocs("alloc04", &|api| unsafe {
        (api.driver)(blob.as_ptr() as *const c_char);
    });
}

#[test]
fn alloc05_tasks_alloc_failure_frees_the_manager() {
    // ERRORS.md row 12/14: the C does `free(manager)` before returning NULL.
    // Dropping that `free` is invisible in the log, so it is checked here.
    unsafe {
        let v = CString::new("10").unwrap();
        setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1);
    }
    assert_same_allocs("alloc05-interposed", &|api| unsafe {
        (api.initialize_logger)();
        let before = arm_malloc_failure(2600);
        let m = (api.create_task_manager)();
        let fired = disarm_malloc_failure(before);
        assert!(fired && m.is_null());
        (api.finalize_logger)();
    });

    // Same branch reached naturally, with a capacity whose size wraps.
    unsafe {
        let v = CString::new("-1").unwrap();
        setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1);
    }
    assert_same_allocs("alloc05-wrap", &|api| unsafe {
        (api.initialize_logger)();
        let m = (api.create_task_manager)();
        assert!(m.is_null());
        (api.finalize_logger)();
    });
    unsafe {
        let v = CString::new("10").unwrap();
        setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1);
    }
}

#[test]
fn alloc06_manager_alloc_failure_frees_nothing() {
    // ERRORS.md row 11: the C returns NULL *without* freeing (there is nothing
    // to free).  A Rust version that freed something here would diverge.
    assert_same_allocs("alloc06", &|api| unsafe {
        (api.initialize_logger)();
        let before = arm_malloc_failure(16);
        let m = (api.create_task_manager)();
        let fired = disarm_malloc_failure(before);
        assert!(fired && m.is_null());
        (api.finalize_logger)();
    });
}

#[test]
fn alloc07_driver_task_alloc_failure_cleans_up() {
    // ERRORS.md row 25: destroy_task_manager + finalize_logger still run.
    let line = vec![b'q'; 60];
    let blob = cstring(&line);
    assert_same_allocs("alloc07", &|api| unsafe {
        let before = arm_malloc_failure(61);
        let r = (api.driver)(blob.as_ptr() as *const c_char);
        let fired = disarm_malloc_failure(before);
        assert!(fired && r == 1);
    });
}

#[test]
fn alloc08_capacity_sweep() {
    for cap in ["0", "1", "2", "10", "64", "500"] {
        let v = CString::new(cap).unwrap();
        unsafe { setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1) };
        let mut rng = Rng::new(0xA110C_04 + cap.len() as u64);
        let blob = cstring(&rng.blob(12, 100));
        assert_same_allocs(&format!("alloc08-{cap}"), &|api| unsafe {
            (api.driver)(blob.as_ptr() as *const c_char);
        });
    }
    let v = CString::new("10").unwrap();
    unsafe { setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1) };
}
