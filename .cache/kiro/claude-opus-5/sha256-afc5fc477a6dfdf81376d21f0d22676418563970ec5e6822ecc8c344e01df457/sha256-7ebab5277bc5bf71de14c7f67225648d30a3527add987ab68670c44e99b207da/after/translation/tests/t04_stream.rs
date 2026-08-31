//! Stream ciphers (salsa20 / salsa2012 / salsa208 / xsalsa20 / chacha20 /
//! chacha20-ietf / xchacha20) and short hashes (siphash24 / siphashx24).
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong};

type FnStream = unsafe extern "C" fn(*mut c_uchar, c_ulonglong, *const c_uchar, *const c_uchar) -> c_int;
type FnStreamXor = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnStreamXorIc64 = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    u64,
    *const c_uchar,
) -> c_int;
type FnStreamXorIc32 = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    u32,
    *const c_uchar,
) -> c_int;
type FnKeygen = unsafe extern "C" fn(*mut c_uchar);

/// `ic_width`: 0 = no `_xor_ic`, 64 = uint64_t counter, 32 = uint32_t counter.
fn stream_suite(prefix: &str, ic_width: u32) {
    for s in ["keybytes", "noncebytes", "messagebytes_max"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let (cnb, _): (FnSize, FnSize) = pair(&format!("{prefix}_noncebytes"));
        let kb = ckb();
        let nb = cnb();

        let (cs, rs): (FnStream, FnStream) = pair(prefix);
        let (cx, rx): (FnStreamXor, FnStreamXor) = pair(&format!("{prefix}_xor"));

        let mut rng = Rng::new(0x2000 + prefix.len() as u64);
        let maxlen = 5000usize;
        let msg = rng.vec(maxlen + 1);

        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        let mut nonces: Vec<Vec<u8>> = vec![vec![0u8; nb], vec![0xffu8; nb]];
        nonces.push(rng.vec(nb));

        let lens = msg_lens();
        for key in &keys {
            for nonce in &nonces {
                for &len in &lens {
                    // keystream generation
                    let mut co = vec![0xAAu8; len + 8];
                    let mut ro = vec![0xAAu8; len + 8];
                    let a = cs(co.as_mut_ptr(), len as c_ulonglong, nonce.as_ptr(), key.as_ptr());
                    let b = rs(ro.as_mut_ptr(), len as c_ulonglong, nonce.as_ptr(), key.as_ptr());
                    assert_eq!(a, b, "{prefix} return len {len}");
                    assert_bytes_eq(&format!("{prefix} keystream len {len}"), &co, &ro);

                    // xor
                    let mut co = vec![0xAAu8; len + 8];
                    let mut ro = vec![0xAAu8; len + 8];
                    let a = cx(
                        co.as_mut_ptr(),
                        msg.as_ptr(),
                        len as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    let b = rx(
                        ro.as_mut_ptr(),
                        msg.as_ptr(),
                        len as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_xor return len {len}");
                    assert_bytes_eq(&format!("{prefix}_xor len {len}"), &co, &ro);
                }
                // in-place xor
                for &len in &[0usize, 1, 63, 64, 65, 1000] {
                    let mut ca = msg[..len + 8].to_vec();
                    let mut ra = msg[..len + 8].to_vec();
                    cx(
                        ca.as_mut_ptr(),
                        ca.as_ptr(),
                        len as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    rx(
                        ra.as_mut_ptr(),
                        ra.as_ptr(),
                        len as c_ulonglong,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    );
                    assert_bytes_eq(&format!("{prefix}_xor in-place len {len}"), &ca, &ra);
                }
            }
        }

        // _xor_ic with interesting initial counters, including wrap points
        if ic_width == 64 {
            let (c, r): (FnStreamXorIc64, FnStreamXorIc64) = pair(&format!("{prefix}_xor_ic"));
            let ics: [u64; 10] = [
                0,
                1,
                2,
                0xff,
                0xffff_ffff,
                0x1_0000_0000,
                0x1_0000_0001,
                0xffff_ffff_ffff_fffe,
                0xffff_ffff_ffff_ffff,
                0x0123_4567_89ab_cdef,
            ];
            for key in &keys {
                for nonce in &nonces {
                    for &ic in &ics {
                        for &len in &[0usize, 1, 63, 64, 65, 127, 128, 129, 200, 1000, 2048] {
                            let mut co = vec![0xAAu8; len + 8];
                            let mut ro = vec![0xAAu8; len + 8];
                            let a = c(
                                co.as_mut_ptr(),
                                msg.as_ptr(),
                                len as c_ulonglong,
                                nonce.as_ptr(),
                                ic,
                                key.as_ptr(),
                            );
                            let b = r(
                                ro.as_mut_ptr(),
                                msg.as_ptr(),
                                len as c_ulonglong,
                                nonce.as_ptr(),
                                ic,
                                key.as_ptr(),
                            );
                            assert_eq!(a, b, "{prefix}_xor_ic return ic={ic} len {len}");
                            assert_bytes_eq(
                                &format!("{prefix}_xor_ic ic={ic:#x} len {len}"),
                                &co,
                                &ro,
                            );
                        }
                    }
                }
            }
        } else if ic_width == 32 {
            let (c, r): (FnStreamXorIc32, FnStreamXorIc32) = pair(&format!("{prefix}_xor_ic"));
            // `crypto_stream_chacha20_ietf_xor_ic` calls sodium_misuse() (which
            // aborts) when ic > 2^32 - ceil(mlen/64); stay below that here and
            // cover the abort path in the dedicated misuse-parity test.
            let ics: [u32; 8] = [
                0,
                1,
                2,
                0xff,
                0xffff,
                0x1234_5678,
                0xffff_ff00,
                0xffff_ffff - 64,
            ];
            for key in &keys {
                for nonce in &nonces {
                    for &ic in &ics {
                        for &len in &[0usize, 1, 63, 64, 65, 127, 128, 129, 200, 1000, 2048] {
                            let mut co = vec![0xAAu8; len + 8];
                            let mut ro = vec![0xAAu8; len + 8];
                            let a = c(
                                co.as_mut_ptr(),
                                msg.as_ptr(),
                                len as c_ulonglong,
                                nonce.as_ptr(),
                                ic,
                                key.as_ptr(),
                            );
                            let b = r(
                                ro.as_mut_ptr(),
                                msg.as_ptr(),
                                len as c_ulonglong,
                                nonce.as_ptr(),
                                ic,
                                key.as_ptr(),
                            );
                            assert_eq!(a, b, "{prefix}_xor_ic return ic={ic} len {len}");
                            assert_bytes_eq(
                                &format!("{prefix}_xor_ic ic={ic:#x} len {len}"),
                                &co,
                                &ro,
                            );
                        }
                    }
                }
            }
        }

        // keygen
        let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
        }
    }
}

#[test]
fn crypto_stream_salsa20_matches() {
    stream_suite("crypto_stream_salsa20", 64);
}

#[test]
fn crypto_stream_salsa2012_matches() {
    stream_suite("crypto_stream_salsa2012", 0);
}

#[test]
fn crypto_stream_salsa208_matches() {
    stream_suite("crypto_stream_salsa208", 0);
}

#[test]
fn crypto_stream_xsalsa20_matches() {
    stream_suite("crypto_stream_xsalsa20", 64);
}

#[test]
fn crypto_stream_chacha20_matches() {
    stream_suite("crypto_stream_chacha20", 64);
}

#[test]
fn crypto_stream_chacha20_ietf_matches() {
    stream_suite("crypto_stream_chacha20_ietf", 32);
}

#[test]
fn crypto_stream_xchacha20_matches() {
    stream_suite("crypto_stream_xchacha20", 64);
}

#[test]
fn crypto_stream_generic_matches() {
    cmp_cstr("crypto_stream_primitive");
    stream_suite("crypto_stream", 0);
}

// ---------------------------------------------------------------------------
// shorthash
// ---------------------------------------------------------------------------

type FnMac = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, c_ulonglong, *const c_uchar) -> c_int;

fn shorthash_suite(prefix: &str) {
    for s in ["bytes", "keybytes"] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let (cb, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let (ckb, _): (FnSize, FnSize) = pair(&format!("{prefix}_keybytes"));
        let ob = cb();
        let kb = ckb();
        let (c, r): (FnMac, FnMac) = pair(prefix);
        let mut rng = Rng::new(0x2100 + prefix.len() as u64);
        let msg = rng.vec(3001);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));
        keys.push((0..kb as u8).collect());
        for key in &keys {
            for len in msg_lens() {
                if len > 3000 {
                    continue;
                }
                let mut co = vec![0xAAu8; ob + 8];
                let mut ro = vec![0xAAu8; ob + 8];
                let a = c(co.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                let b = r(ro.as_mut_ptr(), msg.as_ptr(), len as c_ulonglong, key.as_ptr());
                assert_eq!(a, b, "{prefix} return len {len}");
                assert_bytes_eq(&format!("{prefix} len {len} key {}", hex(key)), &co, &ro);
            }
        }
        if has(&format!("{prefix}_keygen")) {
            let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{prefix}_keygen"));
            for _ in 0..4 {
                let mut a = vec![0xAAu8; kb + 8];
                let mut b = vec![0xAAu8; kb + 8];
                det_reset();
                ck(a.as_mut_ptr());
                det_reset();
                rk(b.as_mut_ptr());
                assert_bytes_eq(&format!("{prefix}_keygen"), &a, &b);
            }
        }
    }
}

#[test]
fn crypto_shorthash_siphash24_matches() {
    shorthash_suite("crypto_shorthash_siphash24");
}

#[test]
fn crypto_shorthash_siphashx24_matches() {
    shorthash_suite("crypto_shorthash_siphashx24");
}

#[test]
fn crypto_shorthash_generic_matches() {
    cmp_cstr("crypto_shorthash_primitive");
    shorthash_suite("crypto_shorthash");
}
