//! Phase C — the `STBDS_ASSERT` rows of `ERRORS.md` (31-33, 44, 54, 61).
//!
//! `c_src` is compiled without `-DNDEBUG` (`nm -D -u` on the C `.so` lists
//! `U __assert_fail`), so a violated invariant writes a diagnostic to stderr and
//! raises `SIGABRT`.  Two of those asserts are reachable from ordinary API
//! inputs; because an abort cannot be observed in-process, each case is run in a
//! re-exec'd child and the wait status of the C child is compared with the wait
//! status of the Rust child.
//!
//! The remaining assert rows are unreachable through the public API and are
//! documented as such in `ERRORS.md`:
//!
//! * row 31 `slot < table->slot_count` — `stbds_hm_find_slot` masks `pos` with
//!   `slot_count-1` before returning, so the result is always in range.
//! * row 44 `(size_t) i+1 <= stbds_arrcap(a)` — the preceding `stbds_arrgrowf`
//!   guarantees it.
//! * row 54 `len <= a->remaining` — the `len > blocksize` arm returns early and
//!   the other arm sets `remaining = blocksize >= len`.
//! * row 61 `used_count_threshold + tombstone_count_threshold < slot_count` —
//!   fails only for `slot_count <= 2`, and the three call sites only ever pass
//!   `8`, `slot_count*2`, or `slot_count>>1` with `slot_count > 8`.
//!
//! All six checks are nevertheless present in the Rust translation, so the two
//! libraries stay in lock-step if a future change makes one reachable.

mod common;

use common::*;
use std::ffi::{c_char, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const ENV_CASE: &str = "HARVEST_ABORT_CASE";

/// ERRORS.md row 32 — `STBDS_ASSERT(slot >= 0)` at `lib.c:846`.
///
/// `mode == 2` is `>= STBDS_HM_STRING`, so hashing and comparison are
/// string-based, but it is `!= STBDS_HM_STRING`, so the swap-with-last re-find
/// takes the raw-bytes branch and hands `stbds_hm_find_slot` a pointer to the
/// *element* (whose first eight bytes are a `char *` value) as if it were a
/// string.  That lookup fails and the assert rejects it.
unsafe fn case_row32(api: &Api) {
    (api.rand_seed)(0x31415926);
    let t0 = (api.shmode_func)(16, STBDS_SH_DEFAULT);
    let mut ka = b"aaa\0".to_vec();
    let mut kb = b"bbb\0".to_vec();
    let mut kc = b"ccc\0".to_vec();
    let mut t = t0;
    for k in [&mut ka, &mut kb, &mut kc] {
        t = (api.hmput_key)(t, 16, k.as_mut_ptr() as *mut c_void, 8, 2);
    }
    // old_index == 0, final_index == 2  ->  the swap-with-last path
    let _ = (api.hmdel_key)(t, 16, ka.as_mut_ptr() as *mut c_void, 8, 0, 2);
}

/// ERRORS.md row 33 — `STBDS_ASSERT(b->index[i] == final_index)` at `lib.c:849`.
///
/// `STBDS_SH_DEFAULT` stores the caller's key pointer verbatim, so the caller can
/// mutate a key after insertion.  Making the *last* element's key compare equal
/// to an earlier element's key means the post-memmove re-find locates the earlier
/// element's slot, whose stored index is not `final_index`.
unsafe fn case_row33(api: &Api) {
    (api.rand_seed)(0x31415926);
    let t0 = (api.shmode_func)(16, STBDS_SH_DEFAULT);
    let mut ka = b"xxx\0".to_vec();
    let mut kb = b"yyy\0".to_vec();
    let mut kc = b"zzz\0".to_vec();
    let mut t = t0;
    for k in [&mut ka, &mut kb, &mut kc] {
        t = (api.hmput_key)(t, 16, k.as_mut_ptr() as *mut c_void, 8, STBDS_HM_STRING);
    }
    // the table keeps `kc`'s pointer; mutate its contents to alias `ka`
    kc[0] = b'x';
    kc[1] = b'x';
    kc[2] = b'x';
    // deleting the middle key moves the aliased element into its place
    let _ = (api.hmdel_key)(
        t,
        16,
        kb.as_mut_ptr() as *mut c_void,
        8,
        0,
        STBDS_HM_STRING,
    );
}

fn run_case(spec: &str) {
    let (case, which) = spec.split_once(':').expect("spec is CASE:LIB");
    let path = if which == "c" {
        c_so_path()
    } else {
        rust_so_path()
    };
    let api = unsafe { Api::load("child", &path) };
    unsafe {
        match case {
            "row32" => case_row32(&api),
            "row33" => case_row33(&api),
            _ => panic!("unknown case {}", case),
        }
    }
}

/// (exit code, terminating signal) of a child running `spec`.
fn child_status(spec: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .arg("--exact")
        .arg("phase_c_assert_abort_parity")
        .arg("--test-threads=1")
        .env(ENV_CASE, spec)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    (out.code(), out.signal())
}

const SIGABRT: i32 = 6;

#[test]
fn phase_c_assert_abort_parity() {
    if let Ok(spec) = std::env::var(ENV_CASE) {
        run_case(&spec);
        // reached only if the assert did NOT fire
        std::process::exit(0);
    }

    for case in ["row32", "row33"] {
        let c = child_status(&format!("{}:c", case));
        let r = child_status(&format!("{}:r", case));
        assert_eq!(
            c.1,
            Some(SIGABRT),
            "{}: the C library must abort (got code={:?} signal={:?})",
            case,
            c.0,
            c.1
        );
        assert_eq!(
            c, r,
            "{}: C exited with (code={:?}, signal={:?}) but Rust exited with (code={:?}, signal={:?})",
            case, c.0, c.1, r.0, r.1
        );
    }
}

// keep imports used by the case bodies
const _: Option<*const c_char> = None;
