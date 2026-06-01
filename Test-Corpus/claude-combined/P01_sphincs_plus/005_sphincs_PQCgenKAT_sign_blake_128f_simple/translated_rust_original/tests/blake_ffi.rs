// Compare FFI calls between C and Rust libs for blake/robust/128s

#![cfg(all(feature = "blake", feature = "robust", feature = "128s"))]

use libloading::{Library, Symbol};
use std::path::PathBuf;

fn libdir() -> PathBuf {
    PathBuf::from("c_src/build/lib/blake")
}

fn libname() -> PathBuf {
    libdir().join("libblake.so")
}

fn rust_libname() -> PathBuf {
    PathBuf::from("target/debug/libsphincs_plus.so")
}

#[test]
fn blake256_oneshot_via_ffi() {
    unsafe {
        let lib_c = Library::new(libname()).unwrap();
        let blake256_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            lib_c.get(b"blake256").unwrap();

        let lib_r = Library::new(rust_libname()).unwrap();
        let blake256_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            lib_r.get(b"blake256").unwrap();

        let long_input: Vec<u8> = (0..200u8).collect();
        for inputs in &[
            &b""[..],
            &b"abc"[..],
            &b"hello world"[..],
            &b"a longer string that is going to span multiple blocks abcdefghijklmnopqrstuvwxyz0123456789"[..],
            &long_input[..],
            &long_input[..63],
            &long_input[..64],
            &long_input[..65],
            &long_input[..127],
            &long_input[..128],
            &long_input[..129],
        ] {
            let mut c_out = [0u8; 32];
            let mut r_out = [0u8; 32];
            blake256_c(c_out.as_mut_ptr(), inputs.as_ptr(), inputs.len() as u64);
            blake256_r(r_out.as_mut_ptr(), inputs.as_ptr(), inputs.len() as u64);
            assert_eq!(c_out, r_out, "blake256 FFI mismatch on input len={}", inputs.len());
        }
    }
}

#[test]
fn randombytes_via_ffi() {
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();

        let lib_c = Library::new("c_src/build/app/libsphincs_core_det.so").unwrap();
        let init_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> =
            lib_c.get(b"randombytes_init").unwrap();
        let rb_c: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> =
            lib_c.get(b"randombytes").unwrap();

        let lib_r = Library::new(rust_libname()).unwrap();
        let init_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> =
            lib_r.get(b"randombytes_init").unwrap();
        let rb_r: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> =
            lib_r.get(b"randombytes").unwrap();

        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }

        let mut c_out = [0u8; 100];
        let mut r_out = [0u8; 100];
        init_c(entropy.as_mut_ptr(), core::ptr::null_mut());
        rb_c(c_out.as_mut_ptr(), 100);
        init_r(entropy.as_mut_ptr(), core::ptr::null_mut());
        rb_r(r_out.as_mut_ptr(), 100);

        assert_eq!(c_out, r_out, "randombytes mismatch");
    }
}

#[test]
fn blake_prf_addr_via_ffi() {
    unsafe {
        // C lib needs sphincs_core_det loaded for SPX_set_* helpers, but
        // SPX_prf_addr itself is in libblake.so.
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _blake_g: UnixLib = UnixLib::open(Some(libname().to_str().unwrap()), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _core_g: UnixLib = UnixLib::open(Some("c_src/build/app/libsphincs_core_det.so"), RTLD_LAZY | RTLD_GLOBAL).unwrap();

        let lib_c = Library::new(libname()).unwrap();
        let prf_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u32)> =
            lib_c.get(b"SPX_prf_addr").unwrap();
        let lib_r = Library::new(rust_libname()).unwrap();
        let prf_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u32)> =
            lib_r.get(b"SPX_prf_addr").unwrap();

        // ctx is just pub_seed||sk_seed (BLAKE backend has no extra state)
        let mut ctx_buf = [0u8; 32];
        for i in 0..32 { ctx_buf[i] = (100 + i) as u8; }
        let mut addr = [0u32; 8];
        for i in 0..8 { addr[i] = (i as u32) * 0x01020304; }

        let mut c_out = [0u8; 16];
        let mut r_out = [0u8; 16];
        prf_c(c_out.as_mut_ptr(), ctx_buf.as_ptr(), addr.as_ptr());
        prf_r(r_out.as_mut_ptr(), ctx_buf.as_ptr(), addr.as_ptr());
        assert_eq!(c_out, r_out, "prf_addr mismatch");
    }
}

#[test]
fn blake_thash_via_ffi() {
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _blake_g: UnixLib = UnixLib::open(Some(libname().to_str().unwrap()), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _core_g: UnixLib = UnixLib::open(Some("c_src/build/app/libsphincs_core_det.so"), RTLD_LAZY | RTLD_GLOBAL).unwrap();

        let lib_c = Library::new(libname()).unwrap();
        let f_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32)> =
            lib_c.get(b"SPX_thash").unwrap();
        let lib_r = Library::new(rust_libname()).unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32)> =
            lib_r.get(b"SPX_thash").unwrap();

        let mut ctx_buf = [0u8; 32];
        for i in 0..32 { ctx_buf[i] = (100 + i) as u8; }
        let mut addr = [0u32; 8];
        for i in 0..8 { addr[i] = (i as u32) * 0x01020304; }

        for inblocks in [1u32, 2, 3, 14] {
            let in_len = (inblocks as usize) * 16;
            let mut input = vec![0u8; in_len];
            for i in 0..in_len { input[i] = (i as u8).wrapping_mul(7); }
            let mut c_out = [0u8; 16];
            let mut r_out = [0u8; 16];
            let mut addr_c = addr;
            let mut addr_r = addr;
            f_c(c_out.as_mut_ptr(), input.as_ptr(), inblocks, ctx_buf.as_ptr(), addr_c.as_mut_ptr());
            f_r(r_out.as_mut_ptr(), input.as_ptr(), inblocks, ctx_buf.as_ptr(), addr_r.as_mut_ptr());
            assert_eq!(c_out, r_out, "thash mismatch inblocks={}", inblocks);
        }
    }
}

#[test]
fn blake_three_update_tiny() {
    // Mirror exactly what gen_message_random does — 3 updates passing
    // datalen as raw byte counts (the C bug). Compare directly.
    use sphincs_plus::hash::blake::blake256::{
        blake256_init_inner, blake256_update_inner, blake256_final_inner, BlakeState256,
    };
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let lib_c = Library::new(libname()).unwrap();
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = lib_c.get(b"blake256_init").unwrap();
        let upd_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64)> = lib_c.get(b"blake256_update").unwrap();
        let fin_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> = lib_c.get(b"blake256_final").unwrap();

        let sk_prf = vec![1u8; 16];
        let optrand = vec![2u8; 16];
        let m = b"hello world";

        // C path
        let mut state_c = vec![0u8; 200];
        init_c(state_c.as_mut_ptr());
        upd_c(state_c.as_mut_ptr(), sk_prf.as_ptr(), 16);
        upd_c(state_c.as_mut_ptr(), optrand.as_ptr(), 16);
        upd_c(state_c.as_mut_ptr(), m.as_ptr(), 11);
        let mut c_out = [0u8; 32];
        fin_c(state_c.as_mut_ptr(), c_out.as_mut_ptr());

        // Rust path
        let mut s = BlakeState256 {
            h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
        };
        blake256_init_inner(&mut s);
        blake256_update_inner(&mut s, &sk_prf, 16);
        blake256_update_inner(&mut s, &optrand, 16);
        blake256_update_inner(&mut s, m, 11);
        let mut r_out = [0u8; 32];
        blake256_final_inner(&mut s, &mut r_out);
        eprintln!("C  : {:?}", c_out);
        eprintln!("Rs : {:?}", r_out);
        assert_eq!(c_out, r_out);
    }
}

#[test]
fn blake_gen_message_random_via_ffi() {
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _blake_g: UnixLib = UnixLib::open(Some(libname().to_str().unwrap()), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _core_g: UnixLib = UnixLib::open(Some("c_src/build/app/libsphincs_core_det.so"), RTLD_LAZY | RTLD_GLOBAL).unwrap();

        let lib_c = Library::new(libname()).unwrap();
        let f_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8)> =
            lib_c.get(b"SPX_gen_message_random").unwrap();
        let lib_r = Library::new(rust_libname()).unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8)> =
            lib_r.get(b"SPX_gen_message_random").unwrap();

        let mut ctx_buf = [0u8; 32];
        for i in 0..32 { ctx_buf[i] = (100 + i) as u8; }
        let sk_prf = vec![1u8; 16];
        let optrand = vec![2u8; 16];
        let m = b"hello world";

        let mut c_out = [0u8; 16];
        let mut r_out = [0u8; 16];
        f_c(c_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), m.len() as u64, ctx_buf.as_ptr());
        f_r(r_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), m.len() as u64, ctx_buf.as_ptr());
        assert_eq!(c_out, r_out, "gen_message_random");
    }
}

#[test]
fn blake_hash_message_via_ffi() {
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _blake_g: UnixLib = UnixLib::open(Some(libname().to_str().unwrap()), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _core_g: UnixLib = UnixLib::open(Some("c_src/build/app/libsphincs_core_det.so"), RTLD_LAZY | RTLD_GLOBAL).unwrap();

        let lib_c = Library::new(libname()).unwrap();
        let f_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8)> =
            lib_c.get(b"SPX_hash_message").unwrap();
        let lib_r = Library::new(rust_libname()).unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8)> =
            lib_r.get(b"SPX_hash_message").unwrap();

        let mut ctx_buf = [0u8; 32];
        for i in 0..32 { ctx_buf[i] = (100 + i) as u8; }
        let r = vec![3u8; 16];
        let pk = vec![4u8; 32];
        let m = b"signature message";

        let mut c_dig = vec![0u8; 21]; // SPX_FORS_MSG_BYTES for 128s
        let mut r_dig = vec![0u8; 21];
        let mut c_tree = 0u64;
        let mut r_tree = 0u64;
        let mut c_leaf = 0u32;
        let mut r_leaf = 0u32;
        f_c(c_dig.as_mut_ptr(), &mut c_tree, &mut c_leaf,
            r.as_ptr(), pk.as_ptr(), m.as_ptr(), m.len() as u64, ctx_buf.as_ptr());
        f_r(r_dig.as_mut_ptr(), &mut r_tree, &mut r_leaf,
            r.as_ptr(), pk.as_ptr(), m.as_ptr(), m.len() as u64, ctx_buf.as_ptr());
        assert_eq!(c_dig, r_dig, "digest mismatch");
        assert_eq!(c_tree, r_tree, "tree mismatch");
        assert_eq!(c_leaf, r_leaf, "leaf mismatch");
    }
}

// keypair_via_ffi: full keypair through both libs in one process is
// fragile due to symbol collisions between the C and Rust .so files
// for crypto_sign_*, randombytes, etc. The end-to-end equivalence is
// verified instead by run_combo.sh which compares driver stdout for
// every feature combination.
#[ignore]
#[test]
fn keypair_via_ffi() {
    unsafe {
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
        // Lazy load with RTLD_GLOBAL — both libs reference each other's symbols
        let _crypto: UnixLib = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _blake_g: UnixLib = UnixLib::open(Some(libname().to_str().unwrap()), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let _core_g: UnixLib = UnixLib::open(Some("c_src/build/app/libsphincs_core_det.so"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let lib_c_blake = Library::new(libname()).unwrap();
        let lib_c = Library::new("c_src/build/app/libsphincs_core_det.so").unwrap();
        let init_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> =
            lib_c.get(b"randombytes_init").unwrap();
        let kp_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> =
            lib_c.get(b"crypto_sign_keypair").unwrap();

        let lib_r = Library::new(rust_libname()).unwrap();
        let init_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> =
            lib_r.get(b"randombytes_init").unwrap();
        let kp_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> =
            lib_r.get(b"crypto_sign_keypair").unwrap();

        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }

        let mut pk_c = vec![0u8; 32];
        let mut sk_c = vec![0u8; 64];
        init_c(entropy.as_mut_ptr(), core::ptr::null_mut());
        kp_c(pk_c.as_mut_ptr(), sk_c.as_mut_ptr());

        let mut pk_r = vec![0u8; 32];
        let mut sk_r = vec![0u8; 64];
        init_r(entropy.as_mut_ptr(), core::ptr::null_mut());
        kp_r(pk_r.as_mut_ptr(), sk_r.as_mut_ptr());

        assert_eq!(sk_c[..48], sk_r[..48], "sk seed mismatch");
        assert_eq!(sk_c, sk_r, "sk mismatch");
        assert_eq!(pk_c, pk_r, "pk mismatch");
        let _ = lib_c_blake;
    }
}

#[test]
fn blake256_mgf1_via_ffi() {
    unsafe {
        let lib_c = Library::new(libname()).unwrap();
        let mgf1_c: Symbol<unsafe extern "C" fn(*mut u8, u64, *const u8, u64)> =
            lib_c.get(b"SPX_blake256_mgf1").unwrap();
        let lib_r = Library::new(rust_libname()).unwrap();
        let mgf1_r: Symbol<unsafe extern "C" fn(*mut u8, u64, *const u8, u64)> =
            lib_r.get(b"SPX_blake256_mgf1").unwrap();

        let input = b"hello";
        for outlen in &[16u64, 32, 48, 64, 100] {
            let mut c_out = vec![0u8; *outlen as usize];
            let mut r_out = vec![0u8; *outlen as usize];
            mgf1_c(c_out.as_mut_ptr(), *outlen, input.as_ptr(), input.len() as u64);
            mgf1_r(r_out.as_mut_ptr(), *outlen, input.as_ptr(), input.len() as u64);
            assert_eq!(c_out, r_out, "mgf1 mismatch outlen={}", outlen);
        }
    }
}
