//! Level 6: the top-level entry point `hm_geti`.
//!
//! `hm_geti` returns nothing and its internal `assert`s abort the process on
//! failure, so it is compared two ways:
//!   * each side is run in a *child process* and the exit status (normal exit vs
//!     SIGABRT from a failed assertion) must match;
//!   * in-process, the state the run leaves behind in the library-global
//!     `stbds_hash_seed` is compared by building a probe table afterwards --
//!     that only matches if both runs performed the same sequence of table
//!     allocations.

mod common;

use common::*;
use std::ffi::c_void;
use std::process::Command;

const ENV_SIDE: &str = "HMGETI_SIDE";
const ENV_NUM: &str = "HMGETI_NUM";

/// Doubles as the child-process worker: when `HMGETI_SIDE` is set this "test"
/// simply performs one `hm_geti` call and exits.
#[test]
fn hm_geti_child_worker() {
    let Ok(side) = std::env::var(ENV_SIDE) else {
        return; // ordinary parent run: nothing to do
    };
    let num: i32 = std::env::var(ENV_NUM).unwrap().parse().unwrap();
    let p = load_pair();
    let im = match side.as_str() {
        "c" => &p.c,
        _ => &p.r,
    };
    unsafe { (im.hm_geti)(num) };
    std::process::exit(0);
}

fn child_status(side: &str, num: i32) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(["--exact", "hm_geti_child_worker", "--test-threads=1"])
        .env(ENV_SIDE, side)
        .env(ENV_NUM, num.to_string())
        .output()
        .expect("failed to spawn child");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    (out.status.code(), signal)
}

#[test]
fn hm_geti_exit_status_matches() {
    if std::env::var(ENV_SIDE).is_ok() {
        return;
    }
    let mut nums: Vec<i32> = (0..40).collect();
    nums.extend_from_slice(&[50, 63, 64, 65, 100, 127, 128, 200, 500, 1000, 2000]);
    for num in nums {
        let c = child_status("c", num);
        let r = child_status("rust", num);
        assert_eq!(
            c, r,
            "hm_geti({num}): C exited with {c:?}, Rust exited with {r:?}"
        );
        assert_eq!(
            c,
            (Some(0), None),
            "hm_geti({num}): expected a clean exit, got {c:?}"
        );
    }
}

#[test]
fn hm_geti_leaves_matching_global_state() {
    if std::env::var(ENV_SIDE).is_ok() {
        return;
    }
    let p = load_pair();
    let mut nums: Vec<i32> = (0..24).collect();
    nums.extend_from_slice(&[40, 100, 256, 300, 1000]);
    for num in nums {
        p.reset_seed(DEFAULT_SEED);
        unsafe {
            (p.c.hm_geti)(num);
            (p.r.hm_geti)(num);
        }
        // probe: a fresh table picks up the current global seed
        let es = 8usize;
        let mut key = 1i32;
        let ct = unsafe {
            (p.c.hmput_key)(
                std::ptr::null_mut(),
                es,
                &mut key as *mut i32 as *mut c_void,
                4,
                STBDS_HM_BINARY,
            )
        };
        let rt = unsafe {
            (p.r.hmput_key)(
                std::ptr::null_mut(),
                es,
                &mut key as *mut i32 as *mut c_void,
                4,
                STBDS_HM_BINARY,
            )
        };
        assert_bytes_eq(
            &format!("global seed state after hm_geti({num})"),
            &unsafe { snapshot_map(ct, es, true) },
            &unsafe { snapshot_map(rt, es, true) },
        );
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (p.r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn hm_geti_negative_num() {
    if std::env::var(ENV_SIDE).is_ok() {
        return;
    }
    // num <= 0 skips every loop; only the leading asserts run
    for num in [-1i32, -100, i32::MIN] {
        let c = child_status("c", num);
        let r = child_status("rust", num);
        assert_eq!(c, r, "hm_geti({num})");
    }
}
