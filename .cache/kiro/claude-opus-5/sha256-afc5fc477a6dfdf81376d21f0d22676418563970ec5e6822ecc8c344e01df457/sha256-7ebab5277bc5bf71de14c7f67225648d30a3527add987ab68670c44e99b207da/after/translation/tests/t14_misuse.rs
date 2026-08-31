//! `sodium_misuse()` / `assert()` parity.
//!
//! Several libsodium entry points deliberately abort the process on invalid
//! arguments instead of returning an error. Those paths cannot be exercised
//! in-process, so each case is run in a child process (one for the C library,
//! one for the Rust library) and the two exit statuses are compared.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uchar, c_ulonglong, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// Every misuse case, by name. Each is performed by `run_case` below.
const CASES: &[&str] = &[
    "chacha20_ietf_xor_ic_overflow",
    "kx_client_session_keys_both_null",
    "kx_server_session_keys_both_null",
    "pwhash_str_alg_bad_alg",
    "bin2hex_maxlen_too_small",
    "bin2hex_maxlen_exact",
    "bin2base64_bad_variant",
    "bin2base64_maxlen_too_small",
    "base642bin_bad_variant",
    "blake2b_init_outlen_zero",
    "blake2b_init_outlen_too_big",
    "blake2b_init_key_null_key",
    "blake2b_init_key_keylen_zero",
    "blake2b_init_key_keylen_too_big",
    "blake2b_final_outlen_zero",
    "hmacsha256_init_null_key_nonzero_len",
    "hmacsha512_init_null_key_nonzero_len",
    "core_h2c_hlen_too_big",
    "argon2_encode_string_short_dst",
];

/// Perform one misuse case against `which` ("c" or "rs"). Returns normally if
/// the library did not abort.
fn run_case(case: &str, which: &str) {
    let l = libs();
    let lib = if which == "c" { &l.c } else { &l.rs };
    macro_rules! sym {
        ($t:ty, $n:expr) => {{
            let mut n = $n.as_bytes().to_vec();
            n.push(0u8);
            let s: libloading::Symbol<$t> =
                unsafe { lib.get(&n) }.unwrap_or_else(|e| panic!("missing {}: {e}", $n));
            *s
        }};
    }
    unsafe {
        match case {
            "chacha20_ietf_xor_ic_overflow" => {
                type F = unsafe extern "C" fn(
                    *mut c_uchar,
                    *const c_uchar,
                    c_ulonglong,
                    *const c_uchar,
                    u32,
                    *const c_uchar,
                ) -> c_int;
                let f = sym!(F, "crypto_stream_chacha20_ietf_xor_ic");
                let m = [0u8; 128];
                let mut c = [0u8; 128];
                let n = [0u8; 12];
                let k = [0u8; 32];
                let r = f(
                    c.as_mut_ptr(),
                    m.as_ptr(),
                    128,
                    n.as_ptr(),
                    0xffff_ffff,
                    k.as_ptr(),
                );
                println!("returned {r}");
            }
            "kx_client_session_keys_both_null" | "kx_server_session_keys_both_null" => {
                type F = unsafe extern "C" fn(
                    *mut c_uchar,
                    *mut c_uchar,
                    *const c_uchar,
                    *const c_uchar,
                    *const c_uchar,
                ) -> c_int;
                let name = if case.contains("client") {
                    "crypto_kx_client_session_keys"
                } else {
                    "crypto_kx_server_session_keys"
                };
                let f = sym!(F, name);
                let pk = [1u8; 32];
                let sk = [2u8; 32];
                let peer = [3u8; 32];
                let r = f(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    peer.as_ptr(),
                );
                println!("returned {r}");
            }
            "pwhash_str_alg_bad_alg" => {
                type F = unsafe extern "C" fn(
                    *mut c_char,
                    *const c_char,
                    c_ulonglong,
                    c_ulonglong,
                    usize,
                    c_int,
                ) -> c_int;
                let f = sym!(F, "crypto_pwhash_str_alg");
                let mut out = [0u8; 256];
                let pw = b"pw";
                let r = f(
                    out.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as c_ulonglong,
                    3,
                    1 << 16,
                    99,
                );
                println!("returned {r}");
            }
            "bin2hex_maxlen_too_small" | "bin2hex_maxlen_exact" => {
                type F =
                    unsafe extern "C" fn(*mut c_char, usize, *const c_uchar, usize) -> *mut c_char;
                let f = sym!(F, "sodium_bin2hex");
                let bin = [0xdeu8, 0xad, 0xbe, 0xef];
                let mut out = [0u8; 32];
                // ABORTS when hex_maxlen <= bin_len * 2 (so 8 aborts, 9 is fine)
                let maxlen = if case.ends_with("exact") { 8 } else { 4 };
                let p = f(out.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), 4);
                println!("returned {}", !p.is_null());
            }
            "bin2base64_bad_variant" | "bin2base64_maxlen_too_small" => {
                type F = unsafe extern "C" fn(
                    *mut c_char,
                    usize,
                    *const c_uchar,
                    usize,
                    c_int,
                ) -> *mut c_char;
                let f = sym!(F, "sodium_bin2base64");
                let bin = [1u8, 2, 3, 4, 5, 6];
                let mut out = [0u8; 64];
                let (maxlen, variant) = if case.ends_with("variant") {
                    (64usize, 0 as c_int)
                } else {
                    (4usize, 1 as c_int)
                };
                let p = f(
                    out.as_mut_ptr() as *mut c_char,
                    maxlen,
                    bin.as_ptr(),
                    bin.len(),
                    variant,
                );
                println!("returned {}", !p.is_null());
            }
            "base642bin_bad_variant" => {
                type F = unsafe extern "C" fn(
                    *mut c_uchar,
                    usize,
                    *const c_char,
                    usize,
                    *const c_char,
                    *mut usize,
                    *mut *const c_char,
                    c_int,
                ) -> c_int;
                let f = sym!(F, "sodium_base642bin");
                let b64 = b"QUJD";
                let mut out = [0u8; 32];
                let r = f(
                    out.as_mut_ptr(),
                    32,
                    b64.as_ptr() as *const c_char,
                    4,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                );
                println!("returned {r}");
            }
            "blake2b_init_outlen_zero"
            | "blake2b_init_outlen_too_big"
            | "blake2b_final_outlen_zero" => {
                type FInit = unsafe extern "C" fn(*mut c_void, u8) -> c_int;
                type FFinal = unsafe extern "C" fn(*mut c_void, *mut c_uchar, u8) -> c_int;
                let init = sym!(FInit, "_sodium_blake2b_init");
                let mut st = AlignedBuf::new(512, 0);
                if case == "blake2b_final_outlen_zero" {
                    assert_eq!(init(st.as_mut_ptr() as *mut c_void, 32), 0);
                    let fin = sym!(FFinal, "_sodium_blake2b_final");
                    let mut out = [0u8; 64];
                    let r = fin(st.as_mut_ptr() as *mut c_void, out.as_mut_ptr(), 0);
                    println!("returned {r}");
                } else {
                    let outlen: u8 = if case.ends_with("zero") { 0 } else { 65 };
                    let r = init(st.as_mut_ptr() as *mut c_void, outlen);
                    println!("returned {r}");
                }
            }
            "blake2b_init_key_null_key"
            | "blake2b_init_key_keylen_zero"
            | "blake2b_init_key_keylen_too_big" => {
                type F = unsafe extern "C" fn(*mut c_void, u8, *const c_void, u8) -> c_int;
                let f = sym!(F, "_sodium_blake2b_init_key");
                let mut st = AlignedBuf::new(512, 0);
                let key = [7u8; 128];
                let (kptr, keylen): (*const c_void, u8) = match case {
                    "blake2b_init_key_null_key" => (std::ptr::null(), 32),
                    "blake2b_init_key_keylen_zero" => (key.as_ptr() as *const c_void, 0),
                    _ => (key.as_ptr() as *const c_void, 65),
                };
                let r = f(st.as_mut_ptr() as *mut c_void, 32, kptr, keylen);
                println!("returned {r}");
            }
            "hmacsha256_init_null_key_nonzero_len"
            | "hmacsha512_init_null_key_nonzero_len" => {
                type F = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize) -> c_int;
                let name = if case.starts_with("hmacsha256") {
                    "crypto_auth_hmacsha256_init"
                } else {
                    "crypto_auth_hmacsha512_init"
                };
                let f = sym!(F, name);
                let mut st = AlignedBuf::new(512, 0);
                let r = f(st.as_mut_ptr() as *mut c_void, std::ptr::null(), 5);
                println!("returned {r}");
            }
            "core_h2c_hlen_too_big" => {
                type F = unsafe extern "C" fn(
                    *mut c_uchar,
                    usize,
                    *const c_uchar,
                    usize,
                    *const c_uchar,
                    usize,
                    c_int,
                ) -> c_int;
                let f = sym!(F, "_sodium_core_h2c_string_to_hash");
                let mut out = vec![0u8; 512];
                let ctx = b"ctx";
                let msg = b"msg";
                let r = f(
                    out.as_mut_ptr(),
                    256,
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    1,
                );
                println!("returned {r}");
            }
            "argon2_encode_string_short_dst" => {
                #[repr(C)]
                struct Ctx {
                    out: *mut u8,
                    outlen: u32,
                    pwd: *mut u8,
                    pwdlen: u32,
                    salt: *mut u8,
                    saltlen: u32,
                    secret: *mut u8,
                    secretlen: u32,
                    ad: *mut u8,
                    adlen: u32,
                    t_cost: u32,
                    m_cost: u32,
                    lanes: u32,
                    threads: u32,
                    flags: u32,
                }
                type F = unsafe extern "C" fn(*mut c_char, usize, *mut Ctx, c_int) -> c_int;
                let f = sym!(F, "_sodium_argon2_encode_string");
                let mut hash = [9u8; 32];
                let mut pwd = [1u8; 8];
                let mut salt = [2u8; 16];
                let mut ctx = Ctx {
                    out: hash.as_mut_ptr(),
                    outlen: 32,
                    pwd: pwd.as_mut_ptr(),
                    pwdlen: 8,
                    salt: salt.as_mut_ptr(),
                    saltlen: 16,
                    secret: std::ptr::null_mut(),
                    secretlen: 0,
                    ad: std::ptr::null_mut(),
                    adlen: 0,
                    t_cost: 1,
                    m_cost: 8,
                    lanes: 1,
                    threads: 1,
                    flags: 0,
                };
                let mut dst = [0u8; 128];
                // 64 bytes is enough for the textual prefix but not for the
                // base64 of the salt, so sodium_bin2base64 misuses.
                let r = f(dst.as_mut_ptr() as *mut c_char, 64, &mut ctx as *mut _, 1);
                println!("returned {r}");
            }
            other => panic!("unknown misuse case {other}"),
        }
    }
}

/// Child entry point. Not a real assertion test: it exists so the parent can
/// re-exec this binary and observe how the process terminates.
#[test]
fn misuse_child() {
    let case = match std::env::var("SODIUM_MISUSE_CASE") {
        Ok(c) => c,
        Err(_) => return, // running normally: nothing to do
    };
    let which = std::env::var("SODIUM_MISUSE_LIB").expect("SODIUM_MISUSE_LIB");
    run_case(&case, &which);
    // If we get here the library returned instead of aborting.
    println!("SURVIVED");
}

fn child_status(case: &str, which: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .arg("misuse_child")
        .arg("--exact")
        .arg("--nocapture")
        .env("SODIUM_MISUSE_CASE", case)
        .env("SODIUM_MISUSE_LIB", which)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn child");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), out.status.signal(), combined)
}

#[test]
fn misuse_behaviour_matches() {
    if std::env::var("SODIUM_MISUSE_CASE").is_ok() {
        return; // we are the child
    }
    let mut failures: Vec<String> = Vec::new();
    for case in CASES {
        let (cc, cs, cout) = child_status(case, "c");
        let (rc, rs, rout) = child_status(case, "rs");
        let c_aborted = cs.is_some();
        let r_aborted = rs.is_some();
        let c_survived = cout.contains("SURVIVED");
        let r_survived = rout.contains("SURVIVED");
        eprintln!(
            "{case:40} C: {} (code={cc:?} sig={cs:?})  Rust: {} (code={rc:?} sig={rs:?})",
            if c_aborted { "aborted" } else if c_survived { "returned" } else { "failed" },
            if r_aborted { "aborted" } else if r_survived { "returned" } else { "failed" },
        );
        if c_aborted != r_aborted {
            failures.push(format!(
                "{case}: C {} but Rust {}\n  C   output: {}\n  Rust output: {}",
                if c_aborted { "aborted" } else { "returned" },
                if r_aborted { "aborted" } else { "returned" },
                cout.trim().replace('\n', " | "),
                rout.trim().replace('\n', " | "),
            ));
            continue;
        }
        if !c_aborted {
            // both returned: the printed return value must match
            let cret = cout
                .lines()
                .find(|l| l.starts_with("returned "))
                .unwrap_or("")
                .to_string();
            let rret = rout
                .lines()
                .find(|l| l.starts_with("returned "))
                .unwrap_or("")
                .to_string();
            if cret != rret {
                failures.push(format!("{case}: return value differs: C {cret:?} Rust {rret:?}"));
            }
        } else if cs != rs {
            failures.push(format!("{case}: abort signal differs: C {cs:?} Rust {rs:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "misuse behaviour mismatches:\n{}",
        failures.join("\n")
    );
}
