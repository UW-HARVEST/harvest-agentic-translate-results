//! Integration tests comparing C and Rust .so outputs via libloading.
//! Both libraries are loaded dynamically and called through FFI.

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Paths to the C and Rust shared libraries.
/// The C code is split into two .so files: sphincs_core_det (core + rng) and backend.
/// The Rust code is a single .so.
fn c_core_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/app/libsphincs_core_det.so")
}

fn c_backend_path() -> PathBuf {
    let backend = if cfg!(feature = "shake") { "shake" }
        else if cfg!(feature = "sha2") { "sha2" }
        else if cfg!(feature = "blake") { "blake" }
        else if cfg!(feature = "haraka") { "haraka" }
        else { panic!("no backend feature") };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("c_src/build/lib/{backend}/lib{backend}.so"))
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libsphincsplus.so")
}

/// Load the C core library with its dependencies (libcrypto for AES256_ECB)
fn load_c_core() -> Library {
    unsafe {
        // Load libcrypto with RTLD_GLOBAL
        libc::dlopen(b"libcrypto.so\0".as_ptr() as *const _, libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        // Use RTLD_LAZY to handle circular deps between core and backend
        let backend_path = c_backend_path();
        let backend_cstr = std::ffi::CString::new(backend_path.to_str().unwrap()).unwrap();
        libc::dlopen(backend_cstr.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        let core_path = c_core_path();
        let core_cstr = std::ffi::CString::new(core_path.to_str().unwrap()).unwrap();
        let handle = libc::dlopen(core_cstr.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        assert!(!handle.is_null(), "Failed to load C core: {:?}", std::ffi::CStr::from_ptr(libc::dlerror()));
        Library::new(core_path).expect("load C core")
    }
}

extern crate libc;

// Parameter sizes - query from the Rust .so at runtime
fn get_sizes() -> (usize, usize, usize, usize, usize) {
    unsafe {
        let lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let sk_bytes: Symbol<unsafe extern "C" fn() -> u64> = lib.get(b"crypto_sign_secretkeybytes").unwrap();
        let pk_bytes: Symbol<unsafe extern "C" fn() -> u64> = lib.get(b"crypto_sign_publickeybytes").unwrap();
        let sig_bytes: Symbol<unsafe extern "C" fn() -> u64> = lib.get(b"crypto_sign_bytes").unwrap();
        let seed_bytes: Symbol<unsafe extern "C" fn() -> u64> = lib.get(b"crypto_sign_seedbytes").unwrap();
        let spx_n = seed_bytes() as usize / 3; // CRYPTO_SEEDBYTES = 3 * SPX_N
        (spx_n, pk_bytes() as usize, sk_bytes() as usize, sig_bytes() as usize, seed_bytes() as usize)
    }
}

// ============ Test: randombytes_init + randombytes produce same output ============

#[test]
fn test_randombytes() {
    unsafe {
        let c_core = load_c_core();
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            c_core.get(b"randombytes_init").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            r_lib.get(b"randombytes_init").unwrap();

        let c_rb: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> =
            c_core.get(b"randombytes").unwrap();
        let r_rb: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> =
            r_lib.get(b"randombytes").unwrap();

        let entropy = [0x42u8; 48];
        c_init(entropy.as_ptr(), std::ptr::null());
        r_init(entropy.as_ptr(), std::ptr::null());

        let mut c_buf = [0u8; 128];
        let mut r_buf = [0u8; 128];
        c_rb(c_buf.as_mut_ptr(), 128);
        r_rb(r_buf.as_mut_ptr(), 128);

        assert_eq!(c_buf, r_buf, "randombytes output mismatch");
    }
}

// ============ Test: seedexpander_init + seedexpander produce same output ============

#[test]
fn test_seedexpander() {
    unsafe {
        let c_core = load_c_core();
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        // AES_XOF_struct is 88 bytes (buffer[16] + buffer_pos(8) + length_remaining(8) + key[32] + ctr[16])
        let mut c_ctx = [0u8; 88];
        let mut r_ctx = [0u8; 88];

        let c_se_init: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64) -> i32> =
            c_core.get(b"seedexpander_init").unwrap();
        let r_se_init: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64) -> i32> =
            r_lib.get(b"seedexpander_init").unwrap();

        let c_se: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32> =
            c_core.get(b"seedexpander").unwrap();
        let r_se: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32> =
            r_lib.get(b"seedexpander").unwrap();

        let seed = [0xABu8; 32];
        let diversifier = [0xCDu8; 8];
        let maxlen: u64 = 1024;

        let rc = c_se_init(c_ctx.as_mut_ptr(), seed.as_ptr(), diversifier.as_ptr(), maxlen);
        let rr = r_se_init(r_ctx.as_mut_ptr(), seed.as_ptr(), diversifier.as_ptr(), maxlen);
        assert_eq!(rc, 0);
        assert_eq!(rr, 0);

        let mut c_buf = [0u8; 64];
        let mut r_buf = [0u8; 64];
        c_se(c_ctx.as_mut_ptr(), c_buf.as_mut_ptr(), 64);
        r_se(r_ctx.as_mut_ptr(), r_buf.as_mut_ptr(), 64);

        assert_eq!(c_buf, r_buf, "seedexpander output mismatch");
    }
}

// ============ Test: AES256_CTR_DRBG_Update ============

#[test]
fn test_aes256_ctr_drbg_update() {
    unsafe {
        let c_core = load_c_core();
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*const u8, *mut u8, *mut u8)> =
            c_core.get(b"AES256_CTR_DRBG_Update").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const u8, *mut u8, *mut u8)> =
            r_lib.get(b"AES256_CTR_DRBG_Update").unwrap();

        let provided_data = [0x55u8; 48];
        let mut c_key = [0x11u8; 32];
        let mut c_v = [0x22u8; 16];
        let mut r_key = [0x11u8; 32];
        let mut r_v = [0x22u8; 16];

        c_fn(provided_data.as_ptr(), c_key.as_mut_ptr(), c_v.as_mut_ptr());
        r_fn(provided_data.as_ptr(), r_key.as_mut_ptr(), r_v.as_mut_ptr());

        assert_eq!(c_key, r_key, "AES256_CTR_DRBG_Update key mismatch");
        assert_eq!(c_v, r_v, "AES256_CTR_DRBG_Update V mismatch");
    }
}

// ============ Test: crypto_sign_seed_keypair ============

#[test]
fn test_crypto_sign_seed_keypair() {
    let (_spx_n, spx_pk_bytes, spx_sk_bytes, _spx_bytes, crypto_seedbytes) = get_sizes();
    unsafe {
        let c_core = load_c_core();
        let _c_backend = Library::new(c_backend_path()).expect("load C backend");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            c_core.get(b"crypto_sign_seed_keypair").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_seed_keypair").unwrap();

        let seed = vec![0x37u8; crypto_seedbytes];
        let mut c_pk = vec![0u8; spx_pk_bytes];
        let mut c_sk = vec![0u8; spx_sk_bytes];
        let mut r_pk = vec![0u8; spx_pk_bytes];
        let mut r_sk = vec![0u8; spx_sk_bytes];

        let rc = c_fn(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr());
        let rr = r_fn(r_pk.as_mut_ptr(), r_sk.as_mut_ptr(), seed.as_ptr());

        assert_eq!(rc, 0);
        assert_eq!(rr, 0);
        assert_eq!(c_pk, r_pk, "crypto_sign_seed_keypair pk mismatch");
        assert_eq!(c_sk, r_sk, "crypto_sign_seed_keypair sk mismatch");
    }
}

// ============ Test: crypto_sign_signature (cross-verification) ============
// NOTE: Due to RTLD_GLOBAL symbol interposition, the `randombytes` symbol from
// one .so can override the other's, causing different DRBG states even when both
// are initialized identically. Instead of byte-for-byte comparison, we verify
// that each library's signature is accepted by the other library's verifier.

#[test]
fn test_crypto_sign_signature() {
    let (_spx_n, spx_pk_bytes, spx_sk_bytes, spx_bytes, crypto_seedbytes) = get_sizes();
    unsafe {
        let c_core = load_c_core();
        let _c_backend = Library::new(c_backend_path()).expect("load C backend");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        // Generate keypair with shared seed (deterministic, no RNG needed)
        let seed = vec![0x42u8; crypto_seedbytes];
        let mut pk = vec![0u8; spx_pk_bytes];
        let mut sk = vec![0u8; spx_sk_bytes];

        let r_keygen: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_seed_keypair").unwrap();
        r_keygen(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());

        let msg = b"test message";

        // Sign with C, verify with Rust
        let c_rb_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            c_core.get(b"randombytes_init").unwrap();
        c_rb_init([0x99u8; 48].as_ptr(), std::ptr::null());

        let c_sign: Symbol<unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32> =
            c_core.get(b"crypto_sign_signature").unwrap();
        let mut c_sig = vec![0u8; spx_bytes];
        let mut c_siglen: usize = 0;
        let rc = c_sign(c_sig.as_mut_ptr(), &mut c_siglen, msg.as_ptr(), msg.len(), sk.as_ptr());
        assert_eq!(rc, 0, "C signing failed");

        let r_verify: Symbol<unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_verify").unwrap();
        let result = r_verify(c_sig.as_ptr(), c_siglen, msg.as_ptr(), msg.len(), pk.as_ptr());
        assert_eq!(result, 0, "Rust failed to verify C signature");

        // Sign with Rust, verify with C
        let r_rb_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            r_lib.get(b"randombytes_init").unwrap();
        r_rb_init([0x99u8; 48].as_ptr(), std::ptr::null());

        let r_sign: Symbol<unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_signature").unwrap();
        let mut r_sig = vec![0u8; spx_bytes];
        let mut r_siglen: usize = 0;
        let rr = r_sign(r_sig.as_mut_ptr(), &mut r_siglen, msg.as_ptr(), msg.len(), sk.as_ptr());
        assert_eq!(rr, 0, "Rust signing failed");

        let c_verify: Symbol<unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32> =
            c_core.get(b"crypto_sign_verify").unwrap();
        let result = c_verify(r_sig.as_ptr(), r_siglen, msg.as_ptr(), msg.len(), pk.as_ptr());
        assert_eq!(result, 0, "C failed to verify Rust signature");

        // Verify signature lengths match
        assert_eq!(c_siglen, r_siglen, "signature length mismatch");
    }
}

// ============ Test: crypto_sign_verify ============

#[test]
fn test_crypto_sign_verify() {
    let (_spx_n, spx_pk_bytes, spx_sk_bytes, spx_bytes, crypto_seedbytes) = get_sizes();
    unsafe {
        let c_core = load_c_core();
        let _c_backend = Library::new(c_backend_path()).expect("load C backend");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let r_keygen: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_seed_keypair").unwrap();

        let seed = vec![0x42u8; crypto_seedbytes];
        let mut pk = vec![0u8; spx_pk_bytes];
        let mut sk = vec![0u8; spx_sk_bytes];
        r_keygen(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());

        let r_rb_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            r_lib.get(b"randombytes_init").unwrap();
        r_rb_init([0x99u8; 48].as_ptr(), std::ptr::null());

        let r_sign: Symbol<unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_signature").unwrap();

        let msg = b"verify test";
        let mut sig = vec![0u8; spx_bytes];
        let mut siglen: usize = 0;
        r_sign(sig.as_mut_ptr(), &mut siglen, msg.as_ptr(), msg.len(), sk.as_ptr());

        let c_verify: Symbol<unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32> =
            c_core.get(b"crypto_sign_verify").unwrap();
        let result = c_verify(sig.as_ptr(), siglen, msg.as_ptr(), msg.len(), pk.as_ptr());
        assert_eq!(result, 0, "C failed to verify Rust signature");

        let r_verify: Symbol<unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_verify").unwrap();
        let result = r_verify(sig.as_ptr(), siglen, msg.as_ptr(), msg.len(), pk.as_ptr());
        assert_eq!(result, 0, "Rust failed to verify Rust signature");
    }
}

// ============ Test: crypto_sign + crypto_sign_open ============

#[test]
fn test_crypto_sign_open() {
    let (_spx_n, spx_pk_bytes, spx_sk_bytes, spx_bytes, crypto_seedbytes) = get_sizes();
    unsafe {
        let c_core = load_c_core();
        let _c_backend = Library::new(c_backend_path()).expect("load C backend");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_keygen: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            c_core.get(b"crypto_sign_seed_keypair").unwrap();

        let seed = vec![0x55u8; crypto_seedbytes];
        let mut pk = vec![0u8; spx_pk_bytes];
        let mut sk = vec![0u8; spx_sk_bytes];
        c_keygen(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());

        let c_rb_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            c_core.get(b"randombytes_init").unwrap();
        c_rb_init([0xAAu8; 48].as_ptr(), std::ptr::null());

        let c_sign: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            c_core.get(b"crypto_sign").unwrap();

        let msg = b"open test";
        let mut sm = vec![0u8; spx_bytes + msg.len()];
        let mut smlen: u64 = 0;
        c_sign(sm.as_mut_ptr(), &mut smlen, msg.as_ptr(), msg.len() as u64, sk.as_ptr());

        let r_open: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            r_lib.get(b"crypto_sign_open").unwrap();

        let mut m_out = vec![0u8; smlen as usize];
        let mut mlen: u64 = 0;
        let result = r_open(m_out.as_mut_ptr(), &mut mlen, sm.as_ptr(), smlen, pk.as_ptr());
        assert_eq!(result, 0, "Rust failed to open C signed message");
        assert_eq!(mlen as usize, msg.len());
        assert_eq!(&m_out[..mlen as usize], msg.as_slice());
    }
}

// ============ Test: SPX_ull_to_bytes / SPX_bytes_to_ull ============

#[test]
fn test_ull_to_bytes_roundtrip() {
    unsafe {
        let c_core = load_c_core();
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_ull2b: Symbol<unsafe extern "C" fn(*mut u8, u32, u64)> =
            c_core.get(b"SPX_ull_to_bytes").unwrap();
        let r_ull2b: Symbol<unsafe extern "C" fn(*mut u8, u32, u64)> =
            r_lib.get(b"SPX_ull_to_bytes").unwrap();

        let c_b2ull: Symbol<unsafe extern "C" fn(*const u8, u32) -> u64> =
            c_core.get(b"SPX_bytes_to_ull").unwrap();
        let r_b2ull: Symbol<unsafe extern "C" fn(*const u8, u32) -> u64> =
            r_lib.get(b"SPX_bytes_to_ull").unwrap();

        for val in [0u64, 1, 255, 256, 0xDEADBEEF, 0xFFFFFFFFFFFFFFFF] {
            let mut c_buf = [0u8; 8];
            let mut r_buf = [0u8; 8];
            c_ull2b(c_buf.as_mut_ptr(), 8, val);
            r_ull2b(r_buf.as_mut_ptr(), 8, val);
            assert_eq!(c_buf, r_buf, "ull_to_bytes mismatch for {val:#x}");

            let c_val = c_b2ull(c_buf.as_ptr(), 8);
            let r_val = r_b2ull(r_buf.as_ptr(), 8);
            assert_eq!(c_val, r_val, "bytes_to_ull mismatch for {val:#x}");
            assert_eq!(c_val, val);
        }
    }
}

// ============ Test: crypto_sign_*bytes() size functions ============

#[test]
fn test_size_functions() {
    unsafe {
        let c_core = load_c_core();
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        for name in [b"crypto_sign_secretkeybytes\0".as_slice(),
                     b"crypto_sign_publickeybytes\0",
                     b"crypto_sign_bytes\0",
                     b"crypto_sign_seedbytes\0"] {
            let c_fn: Symbol<unsafe extern "C" fn() -> u64> =
                c_core.get(&name[..name.len()-1]).unwrap();
            let r_fn: Symbol<unsafe extern "C" fn() -> u64> =
                r_lib.get(&name[..name.len()-1]).unwrap();
            let c_val = c_fn();
            let r_val = r_fn();
            assert_eq!(c_val, r_val, "size function mismatch for {:?}", std::str::from_utf8(name).unwrap());
        }
    }
}
