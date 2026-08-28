//! Phase C — ERRORS.md rows whose C behaviour is a *fault* (SIGSEGV) or an
//! `assert()` abort (SIGABRT).  Those cannot be observed in-process, so each
//! case is executed in a child process (once against the C `.so`, once against
//! the Rust `.so`) and the two termination statuses are compared.
//!
//! The C `.so` is built without `NDEBUG` (`C_FLAGS = -fPIC`), so `assert()` is
//! live in it; the Rust translation carries the same `assert!`s.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

/// ERRORS row -> case name.  Every one of these must terminate abnormally, and
/// C and Rust must terminate the *same* way.
const CASES: &[(&str, &str)] = &[
    ("row 4  arrgrowf: realloc failure -> write through NULL", "arrgrowf_oom"),
    ("row 5  arrfreef(NULL) -> free(NULL-32)", "arrfreef_null"),
    ("row 15 hmget_key_ts with temp == NULL", "hmget_ts_null_temp"),
    ("row 28 hash_string(NULL)", "hash_string_null"),
    ("row 31 hash_bytes(NULL, len>0)", "hash_bytes_null"),
    ("row 32 is_key_equal: stored key pointer is NULL", "strcmp_null_stored_key"),
    ("row 33 stralloc: remaining >= len but storage == NULL", "stralloc_no_storage"),
    ("row 37 stralloc(arena, NULL)", "stralloc_null_str"),
    ("row 39 strreset(NULL)", "strreset_null"),
    ("row 22 hmdel_key: assert(slot >= 0) after relocation", "hmdel_assert_slot"),
    ("row 41/22 hmdel_key with mode==2 + relocation -> assert", "hmdel_mode2_relocate"),
];

fn run_child(case: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", "crash_child", "--nocapture", "--test-threads=1"])
        .env("XCASE", case)
        .env("XWHICH", which)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    (st.code(), st.signal())
}

#[test]
fn crash_parity() {
    // Guard: the child must be runnable at all.
    let (code, sig) = run_child("nop", "c");
    assert_eq!(
        (code, sig),
        (Some(0), None),
        "the `nop` control case must exit cleanly"
    );
    let (code, sig) = run_child("nop", "rust");
    assert_eq!((code, sig), (Some(0), None));

    for (row, case) in CASES {
        let c = run_child(case, "c");
        let r = run_child(case, "rust");
        assert_ne!(
            c,
            (Some(0), None),
            "{row}: the C library did NOT fail for case `{case}` — the case no \
             longer reproduces the documented behaviour"
        );
        assert_eq!(
            c, r,
            "{row}: C and Rust terminated differently for case `{case}` \
             (C = {c:?}, Rust = {r:?}); signal 11 = SIGSEGV, 6 = SIGABRT"
        );
    }
}

// ---------------------------------------------------------------------------
// The child.  Does nothing unless XCASE is set, so a plain `cargo test` run
// treats it as a trivially-passing test.
// ---------------------------------------------------------------------------

#[test]
fn crash_child() {
    let case = match std::env::var("XCASE") {
        Ok(c) => c,
        Err(_) => return,
    };
    let which = std::env::var("XWHICH").unwrap_or_else(|_| "c".into());
    let p = common::libs();
    let l: &Lib = if which == "c" { &p.c } else { &p.r };
    unsafe { run_case(&case, l) };
}

unsafe fn run_case(case: &str, l: &Lib) {
    unsafe {
        match case {
            "nop" => {}

            // row 4 — realloc returns NULL, then the header store faults
            "arrgrowf_oom" => {
                let a = (l.arrgrowf)(std::ptr::null_mut(), 1, 0, usize::MAX / 2);
                std::hint::black_box(a);
            }

            // row 5 — free((char*)NULL - sizeof(header))
            "arrfreef_null" => {
                (l.arrfreef)(std::ptr::null_mut());
            }

            // row 15 — *temp = ... through a NULL out-param
            "hmget_ts_null_temp" => {
                let mut key = [1u8; 8];
                let a = (l.hmget_key_ts)(
                    std::ptr::null_mut(),
                    16,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    std::ptr::null_mut(),
                    0,
                );
                std::hint::black_box(a);
            }

            // row 28 — while (*str) on a NULL pointer
            "hash_string_null" => {
                let h = (l.hash_string)(std::ptr::null_mut(), 0);
                std::hint::black_box(h);
            }

            // row 31 — reading bytes from a NULL buffer
            "hash_bytes_null" => {
                let h = (l.hash_bytes)(std::ptr::null_mut(), 16, 0);
                std::hint::black_box(h);
            }

            // row 32 — strcmp(key, NULL) inside stbds_is_key_equal
            "strcmp_null_stored_key" => {
                (l.rand_seed)(0x4242);
                let elemsize = 16usize;
                let mut key: Vec<u8> = b"abcdefghij\0".to_vec();
                let kp = key.as_mut_ptr() as *mut c_void;
                let mut t = (l.shmode_func)(elemsize, 1 /* STBDS_SH_DEFAULT */);
                t = (l.hmput_key)(t, elemsize, kp, 8, 1);
                // clobber the stored key pointer of element 1 with NULL
                let idx = map_temp(t, elemsize);
                let e = (t as *mut u8).add(elemsize * idx as usize) as *mut *mut c_char;
                *e = std::ptr::null_mut();
                // the hash still matches, so is_key_equal is reached
                let t2 = (l.hmget_key)(t, elemsize, kp, 8, 1);
                std::hint::black_box(t2);
            }

            // row 33 — a->storage->storage with a NULL storage
            "stralloc_no_storage" => {
                let mut a = StringArena {
                    storage: std::ptr::null_mut(),
                    remaining: 100,
                    block: 0,
                    mode: 0,
                };
                let mut s: Vec<u8> = b"hi\0".to_vec();
                let r = (l.stralloc)(&raw mut a, s.as_mut_ptr() as *mut c_char);
                std::hint::black_box(r);
            }

            // row 37 — strlen(NULL)
            "stralloc_null_str" => {
                let mut a = StringArena::new();
                let r = (l.stralloc)(&raw mut a, std::ptr::null_mut());
                std::hint::black_box(r);
            }

            // row 39 — a->storage through a NULL arena
            "strreset_null" => {
                (l.strreset)(std::ptr::null_mut());
            }

            // row 22 — assert(slot >= 0): the relocated element's key no longer
            // hashes to a live slot
            "hmdel_assert_slot" => {
                (l.rand_seed)(0x5151);
                let elemsize = 16usize;
                let keysize = 8usize;
                let mut k1 = 0x1111_1111_1111_1111u64.to_le_bytes();
                let mut k2 = 0x2222_2222_2222_2222u64.to_le_bytes();
                let mut t = (l.hmput_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k1.as_mut_ptr() as *mut c_void,
                    keysize,
                    0,
                );
                let i1 = map_temp(t, elemsize);
                t = (l.hmput_key)(t, elemsize, k2.as_mut_ptr() as *mut c_void, keysize, 0);
                let i2 = map_temp(t, elemsize);
                assert_eq!(i1, 0);
                assert_eq!(i2, 1);
                // corrupt the FINAL element's key so the post-memmove re-find
                // cannot succeed
                let e2 = (t as *mut u8).add(elemsize * i2 as usize);
                std::ptr::write_bytes(e2, 0xEE, keysize);
                // deleting element 0 relocates element 1 into its place
                let t2 = (l.hmdel_key)(
                    t,
                    elemsize,
                    k1.as_mut_ptr() as *mut c_void,
                    keysize,
                    0,
                    0,
                );
                std::hint::black_box(t2);
            }

            // rows 41 + 22 — mode == 2 makes hmdel_key take the `else` re-find
            // branch (address of the element) while hashing it as a string, so
            // the relocation re-find misses and assert(slot >= 0) fires
            "hmdel_mode2_relocate" => {
                (l.rand_seed)(0x6161);
                let elemsize = 16usize;
                let keysize = 8usize;
                let mut a: Vec<u8> = b"alpha_key_0\0".to_vec();
                let mut b: Vec<u8> = b"bravo_key_1\0".to_vec();
                let mut t = (l.shmode_func)(elemsize, 1 /* STBDS_SH_DEFAULT */);
                t = (l.hmput_key)(t, elemsize, a.as_mut_ptr() as *mut c_void, keysize, 1);
                t = (l.hmput_key)(t, elemsize, b.as_mut_ptr() as *mut c_void, keysize, 1);
                // delete the FIRST key with mode == 2 -> relocation + bad re-find
                let t2 = (l.hmdel_key)(
                    t,
                    elemsize,
                    a.as_mut_ptr() as *mut c_void,
                    keysize,
                    0,
                    2 as c_int,
                );
                std::hint::black_box(t2);
            }

            other => panic!("unknown crash case `{other}`"),
        }
    }
}
