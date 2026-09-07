//! Phase C, abort parity.
//!
//! `STBDS_ASSERT` is plain `assert` and the C `.so` is built without `NDEBUG`
//! (it imports `__assert_fail`), so a failing assertion **aborts**. Most of
//! those assertions are unreachable, but ERRORS.md rows 25/44 are reachable
//! through the public ABI: deleting a NON-tail entry of a `STBDS_SH_STRDUP`
//! string map with `mode == 2` (a legal `int`, just not exactly
//! `STBDS_HM_STRING`) makes the relocation re-lookup hash the raw pointer bytes,
//! `stbds_hm_find_slot` returns -1, and `STBDS_ASSERT(slot >= 0)` fires.
//!
//! The Rust must abort in exactly the same place, so this test runs the case in
//! a child process for each library and compares the termination signal.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;

const ENV: &str = "DIFF_ABORT_CASE";

/// The reachable assertion case. Aborts on a faithful implementation.
fn victim(which: &str) {
    let l = libs();
    let imp = if which == "c" { &l.c } else { &l.r };
    let elemsize = 16usize;
    seed_both(0x31415926);
    unsafe {
        let mut m = (imp.shmode_func)(elemsize, SH_STRDUP);
        let keys: [&[u8]; 5] = [b"alpha\0", b"bravo\0", b"charlie\0", b"delta\0", b"echo\0"];
        for k in keys {
            m = (imp.hmput_key)(
                m,
                elemsize,
                k.as_ptr() as *mut c_void,
                8,
                HM_STRING,
            );
        }
        eprintln!("[{which}] populated");
        // entry 0 is not the tail (length-2 == 4), so the relocation path runs
        let victim_key = *(m as *mut *mut std::ffi::c_char);
        m = (imp.hmdel_key)(m, elemsize, victim_key as *mut c_void, 8, 0, 2);
        eprintln!("[{which}] NO ABORT (map={m:?})");
    }
}

#[test]
fn abort_parity_hmdel_mode2_non_tail() {
    if let Ok(which) = std::env::var(ENV) {
        victim(&which);
        // reached only if the implementation failed to assert
        std::process::exit(0);
    }

    let _s = session(0x31415926);
    let exe = std::env::current_exe().unwrap();
    let mut results = Vec::new();
    for which in ["c", "r"] {
        let out = std::process::Command::new(&exe)
            .args([
                "--exact",
                "abort_parity_hmdel_mode2_non_tail",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(ENV, which)
            .env_remove("RUST_BACKTRACE")
            .output()
            .expect("spawn child");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        results.push((
            which,
            out.status.code(),
            out.status.signal(),
            stderr.contains("NO ABORT"),
            stderr,
        ));
    }

    let (_, c_code, c_sig, c_noabort, ref c_err) = results[0];
    let (_, r_code, r_sig, r_noabort, ref r_err) = results[1];

    assert!(
        c_err.contains("[c] populated"),
        "C child did not get as far as the delete:\n{c_err}"
    );
    assert!(
        r_err.contains("[r] populated"),
        "Rust child did not get as far as the delete:\n{r_err}"
    );
    assert_eq!(
        c_noabort, r_noabort,
        "one implementation asserted and the other did not\n C stderr:\n{c_err}\n RUST stderr:\n{r_err}"
    );
    assert!(
        !c_noabort,
        "the C is expected to abort on STBDS_ASSERT(slot >= 0):\n{c_err}"
    );
    assert_eq!(
        (c_code, c_sig),
        (r_code, r_sig),
        "termination differs: C={:?}/{:?} RUST={:?}/{:?}\n C stderr:\n{c_err}\n RUST stderr:\n{r_err}",
        c_code,
        c_sig,
        r_code,
        r_sig
    );
    assert_eq!(c_sig, Some(6), "both must die with SIGABRT");
}
