//! Gap-closing differential tests written during the final audit for rows that
//! the per-area suites had left unchecked.
#[macro_use]
mod common;

use core::ffi::{c_int, c_void};

/// ERRORS.md row `blake2-E64`: `_sodium_blake2b_long` rejects `outlen > UINT32_MAX`.
///
/// The C's `if (outlen > UINT32_MAX) goto fail;` is the FIRST statement of the
/// function (blake2b-long.c:20), before any write to `pout`, so the row IS
/// testable with a small output buffer: nothing may be written and both
/// libraries must return -1.
#[test]
fn blake2b_long_outlen_over_u32max() {
    type F = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;
    let (c, r) = both!("_sodium_blake2b_long", F);
    let mut rng = common::Rng::new(0x5EED_B10B);

    for &outlen in &[
        1usize << 32,          // UINT32_MAX + 1
        (1usize << 32) + 1,
        (1usize << 32) + 63,
        1usize << 40,
        usize::MAX,
    ] {
        for inlen in [0usize, 1, 64, 200] {
            let input = rng.bytes(inlen);
            // 256-byte canary buffers: assert the C and Rust leave them alone.
            let mut cbuf = [0xA5u8; 256];
            let mut rbuf = [0xA5u8; 256];
            let inp = if inlen == 0 {
                core::ptr::null()
            } else {
                input.as_ptr()
            };
            let rc = unsafe { c(cbuf.as_mut_ptr() as *mut c_void, outlen, inp as *const c_void, inlen) };
            let rr = unsafe { r(rbuf.as_mut_ptr() as *mut c_void, outlen, inp as *const c_void, inlen) };
            assert_eq!(
                rc, -1,
                "C must reject outlen={} (inlen={})", outlen, inlen
            );
            assert_eq!(
                rc, rr,
                "blake2b_long outlen={} inlen={}: return mismatch", outlen, inlen
            );
            common::eqb(
                &format!("blake2b_long outlen={} inlen={} output buffer", outlen, inlen),
                &cbuf,
                &rbuf,
            );
            assert!(
                cbuf.iter().all(|&b| b == 0xA5),
                "C wrote to the output buffer on the rejection path"
            );
            assert!(
                rbuf.iter().all(|&b| b == 0xA5),
                "Rust wrote to the output buffer on the rejection path"
            );
        }
    }

    // Control: the largest accepted outlen still behaves identically (and the
    // check really is `>` and not `>=`) — u32::MAX itself would need a 4 GiB
    // buffer, so use the largest cheap accepted value plus a boundary sweep of
    // small accepted lengths through the same entry point.
    for outlen in [1usize, 63, 64, 65, 127, 128, 129, 255, 256] {
        let input = rng.bytes(37);
        let mut cbuf = vec![0xA5u8; outlen + 16];
        let mut rbuf = vec![0xA5u8; outlen + 16];
        let rc = unsafe {
            c(
                cbuf.as_mut_ptr() as *mut c_void,
                outlen,
                input.as_ptr() as *const c_void,
                input.len(),
            )
        };
        let rr = unsafe {
            r(
                rbuf.as_mut_ptr() as *mut c_void,
                outlen,
                input.as_ptr() as *const c_void,
                input.len(),
            )
        };
        assert_eq!(rc, 0, "C should accept outlen={}", outlen);
        assert_eq!(rc, rr, "blake2b_long outlen={}: return mismatch", outlen);
        common::eqb(&format!("blake2b_long outlen={}", outlen), &cbuf, &rbuf);
    }
}

// ---------------------------------------------------------------------------
// Abort-parity tests for the ERRORS.md rows whose C path ends in
// `sodium_misuse()` / `assert()`. These cannot run in-process (they kill the
// test binary), so the test re-executes itself as a child process, performs the
// call against ONE library, and the parent asserts that both the C and the Rust
// child died with SIGABRT (signal 6) — i.e. the rejection is identical, not
// merely "both failed somehow".
// ---------------------------------------------------------------------------

const ABORT_CASES: &[&str] = &[
    // mac-E10 / E13 / E15: hmac init with key == NULL && keylen > 0.
    "hmacsha256_init_null_key",
    "hmacsha512_init_null_key",
    "hmacsha512256_init_null_key",
    // pwhash-E2: crypto_pwhash_str_alg with an out-of-range alg enum value.
    "pwhash_str_alg_bad_alg_0",
    "pwhash_str_alg_bad_alg_3",
    "pwhash_str_alg_bad_alg_neg1",
    "pwhash_str_alg_bad_alg_99",
    // h2c-E2 / E3 (and ed25519low-E36): assert(h_len <= 0xff) in core_h2c.c.
    // This is the regression test for the Rust fix in src/core_ed25519.rs.
    "h2c_hlen_256_sha256",
    "h2c_hlen_256_sha512",
    "h2c_hlen_1000_sha512",
    // blake2b: assert(outlen <= UINT8_MAX) in crypto_generichash_blake2b_final.
    // Regression test for the Rust fix in src/blake2b.rs.
    "generichash_final_outlen_300",
    // pwhash-E99: escrypt_PBKDF2_SHA256 with dkLen > 0x1fffffffe0. The
    // `sodium_misuse()` is the first statement (pbkdf2-sha256.c:63), before any
    // write to `buf`, so a small output buffer is enough.
    "pbkdf2_dklen_too_large",
    "pbkdf2_dklen_size_max",
    // ---- MESSAGEBYTES_MAX / NULL-output misuse guards -------------------
    // Every one of these `sodium_misuse()` calls is the FIRST statement of the
    // function (verified in the C source), so a tiny buffer plus mlen/adlen =
    // u64::MAX reaches it without any memory being touched. Rows:
    // aead1-E1/E2/E3/E22/E28/E35/E37, aead2-E12/E13/E14/E15,
    // box-E12/E13/E22/E23/E27/E33/E38/E40.
    "misuse_cp_encrypt",
    "misuse_cp_ietf_encrypt",
    "misuse_xcp_ietf_encrypt",
    "misuse_aegis128l_encrypt",
    "misuse_aegis128l_detached_mlen",
    "misuse_aegis128l_detached_adlen",
    "misuse_aegis256_encrypt",
    "misuse_aegis256_detached_mlen",
    "misuse_aegis256_detached_adlen",
    "misuse_secretbox_easy",
    "misuse_secretbox_x_easy",
    "misuse_box_easy",
    "misuse_box_easy_afternm",
    "misuse_box_x_easy",
    "misuse_box_x_easy_afternm",
    "misuse_box_seal",
    "misuse_box_x_seal",
    "misuse_secretstream_push",
    "misuse_secretstream_pull",
    "misuse_kx_client_both_null",
    "misuse_kx_server_both_null",
];

fn run_case(case: &str, which: &str) -> std::process::Output {
    let exe = std::env::current_exe().unwrap();
    std::process::Command::new(exe)
        // `--nocapture` matters: libtest buffers a test's stdout and prints it
        // only when the test ends, so without it the marker would be lost when
        // the library aborts the process.
        .args([
            "--exact",
            "gap_abort_child",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("GAP_ABORT_CASE", case)
        .env("GAP_ABORT_LIB", which)
        .output()
        .expect("spawn child")
}

#[test]
fn abort_parity() {
    use std::os::unix::process::ExitStatusExt;
    for case in ABORT_CASES {
        let mut sigs = Vec::new();
        for which in ["c", "rust"] {
            let out = run_case(case, which);
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            assert!(
                !err.contains("panicked at"),
                "{case}/{which}: child panicked instead of aborting in the library:\n{err}"
            );
            let sig = out.status.signal();
            let code = out.status.code();
            assert!(
                String::from_utf8_lossy(&out.stdout).contains("CASE-STARTED"),
                "{case}/{which}: child never reached the call (stdout={:?} stderr={})",
                String::from_utf8_lossy(&out.stdout),
                err
            );
            assert_eq!(
                sig,
                Some(6),
                "{case}/{which}: expected SIGABRT, got signal={sig:?} code={code:?}\n{err}"
            );
            sigs.push(sig);
        }
        assert_eq!(sigs[0], sigs[1], "{case}: C and Rust died differently");
    }
}

/// Child-process worker for `abort_parity`. Never run directly.
#[test]
#[ignore]
fn gap_abort_child() {
    let case = match std::env::var("GAP_ABORT_CASE") {
        Ok(c) => c,
        Err(_) => return,
    };
    let which = std::env::var("GAP_ABORT_LIB").unwrap();
    let l = common::libs();
    let lib = if which == "c" { &l.c } else { &l.r };

    // Flush the marker BEFORE the aborting call so the parent can tell
    // "aborted in the library" from "died on the way there".
    use std::io::Write;
    println!("CASE-STARTED {case} {which}");
    std::io::stdout().flush().unwrap();

    unsafe {
        match case.as_str() {
            "hmacsha256_init_null_key" => {
                let f = getsym!(
                    lib,
                    "crypto_auth_hmacsha256_init",
                    unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int
                );
                let mut st = vec![0u8; 512];
                f(st.as_mut_ptr(), core::ptr::null(), 1);
            }
            "hmacsha512_init_null_key" => {
                let f = getsym!(
                    lib,
                    "crypto_auth_hmacsha512_init",
                    unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int
                );
                let mut st = vec![0u8; 1024];
                f(st.as_mut_ptr(), core::ptr::null(), 1);
            }
            "hmacsha512256_init_null_key" => {
                let f = getsym!(
                    lib,
                    "crypto_auth_hmacsha512256_init",
                    unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int
                );
                let mut st = vec![0u8; 1024];
                f(st.as_mut_ptr(), core::ptr::null(), 1);
            }
            c if c.starts_with("pwhash_str_alg_bad_alg_") => {
                let alg: c_int = match c.rsplit('_').next().unwrap() {
                    "0" => 0,
                    "3" => 3,
                    "neg1" => -1,
                    _ => 99,
                };
                let f = getsym!(
                    lib,
                    "crypto_pwhash_str_alg",
                    unsafe extern "C" fn(*mut u8, *const u8, u64, u64, usize, c_int) -> c_int
                );
                let ops = getsym!(
                    lib,
                    "crypto_pwhash_opslimit_min",
                    unsafe extern "C" fn() -> usize
                )();
                let mem = getsym!(
                    lib,
                    "crypto_pwhash_memlimit_min",
                    unsafe extern "C" fn() -> usize
                )();
                let mut out = vec![0u8; 256];
                let pw = b"password\0";
                f(out.as_mut_ptr(), pw.as_ptr(), 8, ops as u64, mem, alg);
            }
            c if c.starts_with("h2c_hlen_") => {
                let (h_len, alg): (usize, c_int) = match c {
                    "h2c_hlen_256_sha256" => (256, 1),
                    "h2c_hlen_256_sha512" => (256, 2),
                    _ => (1000, 2),
                };
                let f = getsym!(
                    lib,
                    "_sodium_core_h2c_string_to_hash",
                    unsafe extern "C" fn(
                        *mut u8,
                        usize,
                        *const u8,
                        usize,
                        *const u8,
                        usize,
                        c_int,
                    ) -> c_int
                );
                let mut h = vec![0u8; h_len];
                let ctx = b"CTX";
                let msg = b"msg";
                f(
                    h.as_mut_ptr(),
                    h_len,
                    ctx.as_ptr(),
                    ctx.len(),
                    msg.as_ptr(),
                    msg.len(),
                    alg,
                );
            }
            "generichash_final_outlen_300" => {
                let init = getsym!(
                    lib,
                    "crypto_generichash_blake2b_init",
                    unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int
                );
                let fin = getsym!(
                    lib,
                    "crypto_generichash_blake2b_final",
                    unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int
                );
                let mut st = vec![0u8; 512];
                init(st.as_mut_ptr(), core::ptr::null(), 0, 32);
                let mut out = vec![0u8; 512];
                fin(st.as_mut_ptr(), out.as_mut_ptr(), 300);
            }
            c if c.starts_with("pbkdf2_dklen_") => {
                let dklen: usize = if c.ends_with("size_max") {
                    usize::MAX
                } else {
                    0x1fff_ffff_e0usize + 1
                };
                let f = getsym!(
                    lib,
                    "_sodium_escrypt_PBKDF2_SHA256",
                    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize)
                );
                let pw = b"password";
                let salt = b"NaCl";
                let mut out = [0u8; 64];
                f(
                    pw.as_ptr(),
                    pw.len(),
                    salt.as_ptr(),
                    salt.len(),
                    1,
                    out.as_mut_ptr(),
                    dklen,
                );
            }
            c if c.starts_with("misuse_") => {
                type Aead = unsafe extern "C" fn(
                    *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8,
                    *const u8,
                ) -> c_int;
                type AeadDet = unsafe extern "C" fn(
                    *mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8,
                    *const u8, *const u8,
                ) -> c_int;
                type Easy =
                    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
                type BoxEasy = unsafe extern "C" fn(
                    *mut u8, *const u8, u64, *const u8, *const u8, *const u8,
                ) -> c_int;
                type Seal = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
                type Push = unsafe extern "C" fn(
                    *mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8,
                ) -> c_int;
                type Pull = unsafe extern "C" fn(
                    *mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64,
                ) -> c_int;
                type Kx = unsafe extern "C" fn(
                    *mut u8, *mut u8, *const u8, *const u8, *const u8,
                ) -> c_int;

                // Scratch buffers: nothing may be read/written before the guard.
                let mut buf = [0u8; 256];
                let mut buf2 = [0u8; 256];
                let key = [7u8; 64];
                let nonce = [3u8; 32];
                let mut outlen: u64 = 0;
                let big = u64::MAX;
                let b = buf.as_mut_ptr();
                let b2 = buf2.as_mut_ptr();
                let k = key.as_ptr();
                let n = nonce.as_ptr();

                match case.as_str() {
                    "misuse_cp_encrypt" => {
                        getsym!(lib, "crypto_aead_chacha20poly1305_encrypt", Aead)(
                            b, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_cp_ietf_encrypt" => {
                        getsym!(lib, "crypto_aead_chacha20poly1305_ietf_encrypt", Aead)(
                            b, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_xcp_ietf_encrypt" => {
                        getsym!(lib, "crypto_aead_xchacha20poly1305_ietf_encrypt", Aead)(
                            b, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis128l_encrypt" => {
                        getsym!(lib, "crypto_aead_aegis128l_encrypt", Aead)(
                            b, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis128l_detached_mlen" => {
                        getsym!(lib, "crypto_aead_aegis128l_encrypt_detached", AeadDet)(
                            b, b2, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis128l_detached_adlen" => {
                        getsym!(lib, "crypto_aead_aegis128l_encrypt_detached", AeadDet)(
                            b, b2, &mut outlen, k, 0, k, big, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis256_encrypt" => {
                        getsym!(lib, "crypto_aead_aegis256_encrypt", Aead)(
                            b, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis256_detached_mlen" => {
                        getsym!(lib, "crypto_aead_aegis256_encrypt_detached", AeadDet)(
                            b, b2, &mut outlen, k, big, k, 0, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_aegis256_detached_adlen" => {
                        getsym!(lib, "crypto_aead_aegis256_encrypt_detached", AeadDet)(
                            b, b2, &mut outlen, k, 0, k, big, core::ptr::null(), n, k,
                        );
                    }
                    "misuse_secretbox_easy" => {
                        getsym!(lib, "crypto_secretbox_easy", Easy)(b, k, big, n, k);
                    }
                    "misuse_secretbox_x_easy" => {
                        getsym!(lib, "crypto_secretbox_xchacha20poly1305_easy", Easy)(
                            b, k, big, n, k,
                        );
                    }
                    "misuse_box_easy" => {
                        getsym!(lib, "crypto_box_easy", BoxEasy)(b, k, big, n, k, k);
                    }
                    "misuse_box_easy_afternm" => {
                        getsym!(lib, "crypto_box_easy_afternm", Easy)(b, k, big, n, k);
                    }
                    "misuse_box_x_easy" => {
                        getsym!(
                            lib,
                            "crypto_box_curve25519xchacha20poly1305_easy",
                            BoxEasy
                        )(b, k, big, n, k, k);
                    }
                    "misuse_box_x_easy_afternm" => {
                        getsym!(
                            lib,
                            "crypto_box_curve25519xchacha20poly1305_easy_afternm",
                            Easy
                        )(b, k, big, n, k);
                    }
                    "misuse_box_seal" => {
                        getsym!(lib, "crypto_box_seal", Seal)(b, k, big, k);
                    }
                    "misuse_box_x_seal" => {
                        getsym!(lib, "crypto_box_curve25519xchacha20poly1305_seal", Seal)(
                            b, k, big, k,
                        );
                    }
                    "misuse_secretstream_push" => {
                        let mut st = [0u8; 64];
                        getsym!(lib, "crypto_secretstream_xchacha20poly1305_push", Push)(
                            st.as_mut_ptr(),
                            b,
                            &mut outlen,
                            k,
                            big,
                            k,
                            0,
                            0,
                        );
                    }
                    "misuse_secretstream_pull" => {
                        let mut st = [0u8; 64];
                        let mut tag = 0u8;
                        getsym!(lib, "crypto_secretstream_xchacha20poly1305_pull", Pull)(
                            st.as_mut_ptr(),
                            b,
                            &mut outlen,
                            &mut tag,
                            k,
                            big,
                            k,
                            0,
                        );
                    }
                    "misuse_kx_client_both_null" => {
                        getsym!(lib, "crypto_kx_client_session_keys", Kx)(
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            k,
                            k,
                            k,
                        );
                    }
                    "misuse_kx_server_both_null" => {
                        getsym!(lib, "crypto_kx_server_session_keys", Kx)(
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            k,
                            k,
                            k,
                        );
                    }
                    other => panic!("unknown misuse case {other}"),
                }
            }
            other => panic!("unknown abort case {other}"),
        }
    }
    // If we get here the library did NOT abort: signal a distinct exit code.
    println!("SURVIVED {case} {which}");
    std::io::stdout().flush().unwrap();
    std::process::exit(7);
}
