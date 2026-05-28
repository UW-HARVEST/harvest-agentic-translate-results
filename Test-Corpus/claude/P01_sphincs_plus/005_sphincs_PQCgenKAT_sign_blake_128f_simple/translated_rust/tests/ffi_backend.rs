// Backend-specific FFI tests — exercise hash-backend specific exports.

use libloading::{Library, Symbol};
use std::path::PathBuf;

fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let mut dir = exe.parent().unwrap().to_path_buf();
    while dir.parent().is_some() {
        let candidate = dir.join("libsphincs_plus.so");
        if candidate.exists() {
            return candidate;
        }
        dir = dir.parent().unwrap().to_path_buf();
    }
    panic!("could not locate libsphincs_plus.so");
}

fn current_backend() -> &'static str {
    if cfg!(feature = "blake") { "blake" }
    else if cfg!(feature = "haraka") { "haraka" }
    else if cfg!(feature = "sha2") { "sha2" }
    else if cfg!(feature = "shake") { "shake" }
    else { panic!("no backend feature enabled") }
}

fn current_thash() -> &'static str {
    if cfg!(feature = "robust") { "robust" }
    else if cfg!(feature = "simple") { "simple" }
    else { panic!("no thash feature enabled") }
}

fn current_secpar() -> &'static str {
    if cfg!(feature = "128s") { "128s" }
    else if cfg!(feature = "128f") { "128f" }
    else if cfg!(feature = "192s") { "192s" }
    else if cfg!(feature = "192f") { "192f" }
    else if cfg!(feature = "256s") { "256s" }
    else if cfg!(feature = "256f") { "256f" }
    else { panic!("no secpar feature enabled") }
}

fn find_c_so(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    let build_name = format!("build_{}_{}_{}", current_backend(), current_thash(), current_secpar());
    let single_dir = p.join(&build_name);
    let primary_dir = if single_dir.exists() { single_dir } else { p.join("build") };
    let mut q = primary_dir.clone();
    if name == "blake" || name == "haraka" || name == "sha2" || name == "shake" {
        q.push("lib");
        q.push(name);
        q.push(format!("lib{}.so", name));
    } else if name == "core_det" {
        q.push("app");
        q.push("libsphincs_core_det.so");
    }
    q
}

struct CLibs {
    backend: Library,
    core: Library,
}

impl CLibs {
    fn load() -> Self {
        let backend_name = current_backend();
        unsafe {
            let lazy_global = libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL;
            // Pre-load libcrypto for OpenSSL deps
            let _ = libloading::os::unix::Library::open(
                Some("libcrypto.so"), lazy_global,
            ).or_else(|_| libloading::os::unix::Library::open(Some("libcrypto.so.1.0.0"), lazy_global))
             .or_else(|_| libloading::os::unix::Library::open(Some("libcrypto.so.3"), lazy_global))
             .or_else(|_| libloading::os::unix::Library::open(Some("libcrypto.so.10"), lazy_global));
            let backend_os = libloading::os::unix::Library::open(
                Some(find_c_so(backend_name)), lazy_global,
            ).expect("c backend lib");
            let backend = Library::from(backend_os);
            let core_os = libloading::os::unix::Library::open(
                Some(find_c_so("core_det")), lazy_global,
            ).expect("c core lib");
            let core = Library::from(core_os);
            CLibs { backend, core }
        }
    }
}

fn load_rust() -> Library {
    unsafe { Library::new(find_rust_so()).expect("rust lib") }
}

#[cfg(feature = "shake")]
mod shake_tests {
    use super::*;

    #[test]
    fn test_shake256_one_shot() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, *const u8, usize)> =
                cl.backend.get(b"shake256").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, *const u8, usize)> =
                rl.get(b"shake256").unwrap();
            for &(inlen, outlen) in &[(0usize, 32usize), (32, 64), (100, 100), (200, 300)] {
                let input: Vec<u8> = (0..inlen).map(|i| (i * 5 + 1) as u8).collect();
                let mut a = vec![0u8; outlen];
                let mut b = vec![0u8; outlen];
                c_fn(a.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                r_fn(b.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                assert_eq!(a, b, "shake256 mismatch in={} out={}", inlen, outlen);
            }
        }
    }

    #[test]
    fn test_shake256_inc() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let init_c: Symbol<unsafe extern "C" fn(*mut u64)> = cl.backend.get(b"shake256_inc_init").unwrap();
            let init_r: Symbol<unsafe extern "C" fn(*mut u64)> = rl.get(b"shake256_inc_init").unwrap();
            let abs_c: Symbol<unsafe extern "C" fn(*mut u64, *const u8, usize)> = cl.backend.get(b"shake256_inc_absorb").unwrap();
            let abs_r: Symbol<unsafe extern "C" fn(*mut u64, *const u8, usize)> = rl.get(b"shake256_inc_absorb").unwrap();
            let fin_c: Symbol<unsafe extern "C" fn(*mut u64)> = cl.backend.get(b"shake256_inc_finalize").unwrap();
            let fin_r: Symbol<unsafe extern "C" fn(*mut u64)> = rl.get(b"shake256_inc_finalize").unwrap();
            let sq_c: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u64)> = cl.backend.get(b"shake256_inc_squeeze").unwrap();
            let sq_r: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u64)> = rl.get(b"shake256_inc_squeeze").unwrap();

            let mut sa = [0u64; 26];
            let mut sb = [0u64; 26];
            init_c(sa.as_mut_ptr());
            init_r(sb.as_mut_ptr());
            let chunk1: Vec<u8> = (0..50).map(|i| (i * 3 + 1) as u8).collect();
            let chunk2: Vec<u8> = (0..200).map(|i| (i * 7 + 11) as u8).collect();
            abs_c(sa.as_mut_ptr(), chunk1.as_ptr(), chunk1.len());
            abs_r(sb.as_mut_ptr(), chunk1.as_ptr(), chunk1.len());
            abs_c(sa.as_mut_ptr(), chunk2.as_ptr(), chunk2.len());
            abs_r(sb.as_mut_ptr(), chunk2.as_ptr(), chunk2.len());
            fin_c(sa.as_mut_ptr());
            fin_r(sb.as_mut_ptr());
            let mut oa = vec![0u8; 200];
            let mut ob = vec![0u8; 200];
            sq_c(oa.as_mut_ptr(), 100, sa.as_mut_ptr());
            sq_r(ob.as_mut_ptr(), 100, sb.as_mut_ptr());
            sq_c(oa.as_mut_ptr().add(100), 100, sa.as_mut_ptr());
            sq_r(ob.as_mut_ptr().add(100), 100, sb.as_mut_ptr());
            assert_eq!(oa, ob, "shake256_inc squeezed bytes mismatch");
        }
    }
}

#[cfg(feature = "sha2")]
mod sha2_tests {
    use super::*;

    #[test]
    fn test_sha256_one_shot() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, usize)> =
                cl.backend.get(b"sha256").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, usize)> =
                rl.get(b"sha256").unwrap();
            for inlen in [0usize, 1, 32, 55, 56, 64, 65, 100, 1000] {
                let input: Vec<u8> = (0..inlen).map(|i| (i * 3 + 1) as u8).collect();
                let mut a = [0u8; 32];
                let mut b = [0u8; 32];
                c_fn(a.as_mut_ptr(), input.as_ptr(), inlen);
                r_fn(b.as_mut_ptr(), input.as_ptr(), inlen);
                assert_eq!(a, b, "sha256 mismatch inlen={}", inlen);
            }
        }
    }

    #[test]
    fn test_sha512_one_shot() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, usize)> =
                cl.backend.get(b"sha512").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, usize)> =
                rl.get(b"sha512").unwrap();
            for inlen in [0usize, 1, 64, 111, 112, 128, 200, 1000] {
                let input: Vec<u8> = (0..inlen).map(|i| (i * 7 + 5) as u8).collect();
                let mut a = [0u8; 64];
                let mut b = [0u8; 64];
                c_fn(a.as_mut_ptr(), input.as_ptr(), inlen);
                r_fn(b.as_mut_ptr(), input.as_ptr(), inlen);
                assert_eq!(a, b, "sha512 mismatch inlen={}", inlen);
            }
        }
    }

    #[test]
    fn test_mgf1_256() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong)> =
                cl.backend.get(b"SPX_mgf1_256").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong)> =
                rl.get(b"SPX_mgf1_256").unwrap();
            for &(inlen, outlen) in &[(8usize, 16usize), (32, 64), (50, 100)] {
                let input: Vec<u8> = (0..inlen).map(|i| (i * 5 + 1) as u8).collect();
                let mut a = vec![0u8; outlen];
                let mut b = vec![0u8; outlen];
                c_fn(a.as_mut_ptr(), outlen as core::ffi::c_ulong, input.as_ptr(), inlen as core::ffi::c_ulong);
                r_fn(b.as_mut_ptr(), outlen as core::ffi::c_ulong, input.as_ptr(), inlen as core::ffi::c_ulong);
                assert_eq!(a, b, "mgf1_256 mismatch in={} out={}", inlen, outlen);
            }
        }
    }
}

#[cfg(feature = "haraka")]
mod haraka_tests {
    use super::*;

    fn build_zero_ctx_then_tweak(libs: &CLibs, sel: &str) -> Vec<u8> {
        const CTX_SIZE: usize = 1024;
        let mut ctx = vec![0u8; CTX_SIZE];
        // Set pub_seed and sk_seed to test values
        let n = if cfg!(feature = "128s") || cfg!(feature = "128f") { 16 }
                else if cfg!(feature = "192s") || cfg!(feature = "192f") { 24 }
                else { 32 };
        for i in 0..n { ctx[i] = (i as u8).wrapping_mul(13); }
        for i in 0..n { ctx[n + i] = (i as u8).wrapping_mul(17); }
        unsafe {
            if sel == "c" {
                let f: Symbol<unsafe extern "C" fn(*mut u8)> = libs.backend.get(b"SPX_tweak_constants").unwrap();
                f(ctx.as_mut_ptr());
            } else {
                panic!("expected sel=c");
            }
        }
        ctx
    }

    #[test]
    fn test_haraka512_perm() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let twc_c: Symbol<unsafe extern "C" fn(*mut u8)> = cl.backend.get(b"SPX_tweak_constants").unwrap();
            let twc_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_tweak_constants").unwrap();
            const CTX_SIZE: usize = 1024;
            let mut ca = vec![0u8; CTX_SIZE];
            let mut cb = vec![0u8; CTX_SIZE];
            let n = if cfg!(feature = "128s") || cfg!(feature = "128f") { 16 }
                    else if cfg!(feature = "192s") || cfg!(feature = "192f") { 24 }
                    else { 32 };
            for i in 0..n { ca[i] = (i as u8).wrapping_mul(13); cb[i] = ca[i]; }
            for i in 0..n { ca[n + i] = (i as u8).wrapping_mul(17); cb[n + i] = ca[n + i]; }
            twc_c(ca.as_mut_ptr());
            twc_r(cb.as_mut_ptr());
            // The tweaked constants tables come right after pub_seed+sk_seed in
            // both C and Rust. They should match.
            // Compare 2*N + 10*8*8 (tweaked512_rc64) + 10*8*4 (tweaked256_rc32)
            let cmp_len = 2 * n + 10 * 8 * 8 + 10 * 8 * 4;
            assert_eq!(&ca[..cmp_len], &cb[..cmp_len], "tweak_constants ctx mismatch");

            // Now test haraka512_perm
            let perm_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                cl.backend.get(b"SPX_haraka512_perm").unwrap();
            let perm_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                rl.get(b"SPX_haraka512_perm").unwrap();
            let input: [u8; 64] = core::array::from_fn(|i| (i * 11 + 5) as u8);
            let mut oa = [0u8; 64];
            let mut ob = [0u8; 64];
            perm_c(oa.as_mut_ptr(), input.as_ptr(), ca.as_ptr());
            perm_r(ob.as_mut_ptr(), input.as_ptr(), cb.as_ptr());
            assert_eq!(oa, ob, "haraka512_perm mismatch");
        }
    }

    #[test]
    fn test_haraka256_512() {
        let cl = CLibs::load();
        let rl = load_rust();
        unsafe {
            let twc_c: Symbol<unsafe extern "C" fn(*mut u8)> = cl.backend.get(b"SPX_tweak_constants").unwrap();
            let twc_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_tweak_constants").unwrap();
            const CTX_SIZE: usize = 1024;
            let mut ca = vec![0u8; CTX_SIZE];
            let mut cb = vec![0u8; CTX_SIZE];
            let n = if cfg!(feature = "128s") || cfg!(feature = "128f") { 16 }
                    else if cfg!(feature = "192s") || cfg!(feature = "192f") { 24 }
                    else { 32 };
            for i in 0..n { ca[i] = 7u8.wrapping_add(i as u8); cb[i] = ca[i]; }
            for i in 0..n { ca[n + i] = 13u8.wrapping_add(i as u8); cb[n + i] = ca[n + i]; }
            twc_c(ca.as_mut_ptr());
            twc_r(cb.as_mut_ptr());

            // haraka256
            let h256_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                cl.backend.get(b"SPX_haraka256").unwrap();
            let h256_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                rl.get(b"SPX_haraka256").unwrap();
            let input32: [u8; 32] = core::array::from_fn(|i| (i * 9 + 1) as u8);
            let mut oa = [0u8; 32];
            let mut ob = [0u8; 32];
            h256_c(oa.as_mut_ptr(), input32.as_ptr(), ca.as_ptr());
            h256_r(ob.as_mut_ptr(), input32.as_ptr(), cb.as_ptr());
            assert_eq!(oa, ob, "haraka256 mismatch");

            // haraka512 (always writes 32 bytes)
            let h512_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                cl.backend.get(b"SPX_haraka512").unwrap();
            let h512_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8)> =
                rl.get(b"SPX_haraka512").unwrap();
            let input64: [u8; 64] = core::array::from_fn(|i| (i * 7 + 3) as u8);
            let mut oa2 = vec![0u8; 32];
            let mut ob2 = vec![0u8; 32];
            h512_c(oa2.as_mut_ptr(), input64.as_ptr(), ca.as_ptr());
            h512_r(ob2.as_mut_ptr(), input64.as_ptr(), cb.as_ptr());
            assert_eq!(oa2, ob2, "haraka512 mismatch");
        }
    }
}

// ---------------------------------------------------------------------
// Tests applicable to all backends — generic high-level functions
// ---------------------------------------------------------------------

#[test]
fn test_seed_keypair() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            cl.core.get(b"crypto_sign_seed_keypair").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            rl.get(b"crypto_sign_seed_keypair").unwrap();
        let pkbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_publickeybytes").unwrap();
        let skbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_secretkeybytes").unwrap();
        let seedbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_seedbytes").unwrap();
        let pk_len = pkbytes_c() as usize;
        let sk_len = skbytes_c() as usize;
        let seed_len = seedbytes_c() as usize;

        let seed: Vec<u8> = (0..seed_len).map(|i| (i * 13 + 7) as u8).collect();
        let mut pk_a = vec![0u8; pk_len];
        let mut sk_a = vec![0u8; sk_len];
        let mut pk_b = vec![0u8; pk_len];
        let mut sk_b = vec![0u8; sk_len];
        c_fn(pk_a.as_mut_ptr(), sk_a.as_mut_ptr(), seed.as_ptr());
        r_fn(pk_b.as_mut_ptr(), sk_b.as_mut_ptr(), seed.as_ptr());
        assert_eq!(pk_a, pk_b, "seed keypair pk mismatch");
        assert_eq!(sk_a, sk_b, "seed keypair sk mismatch");
    }
}

// ---------------------------------------------------------------------
// Tests for higher-level functions: fors_sign, fors_pk_from_sig,
// merkle_gen_root, merkle_sign.
// These rely on the deterministic randombytes.
// ---------------------------------------------------------------------

#[test]
fn test_merkle_gen_root_via_keypair() {
    // crypto_sign_seed_keypair calls merkle_gen_root internally; the keypair
    // test in ffi_compare already validates that whole chain. Add a
    // standalone test for merkle_gen_root for stronger isolation.
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = cl.backend.get(b"SPX_initialize_hash_function").unwrap_or_else(|_| {
            cl.core.get(b"SPX_initialize_hash_function").unwrap()
        });
        let init_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_initialize_hash_function").unwrap();
        let mgr_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8)> = cl.core.get(b"SPX_merkle_gen_root").unwrap();
        let mgr_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8)> = rl.get(b"SPX_merkle_gen_root").unwrap();

        // fast variants only — slow ones (s) take a long time
        if cfg!(any(feature = "128s", feature = "192s", feature = "256s")) {
            return;
        }

        const CTX_SIZE: usize = 2048;
        let n = if cfg!(feature = "128s") || cfg!(feature = "128f") { 16 }
                else if cfg!(feature = "192s") || cfg!(feature = "192f") { 24 }
                else { 32 };
        let mut ca = vec![0u8; CTX_SIZE];
        let mut cb = vec![0u8; CTX_SIZE];
        for i in 0..n { ca[i] = (i as u8).wrapping_mul(13); cb[i] = ca[i]; }
        for i in 0..n { ca[n + i] = (i as u8).wrapping_mul(17); cb[n + i] = ca[n + i]; }
        init_c(ca.as_mut_ptr());
        init_r(cb.as_mut_ptr());

        let mut ra = vec![0u8; n];
        let mut rb = vec![0u8; n];
        mgr_c(ra.as_mut_ptr(), ca.as_ptr());
        mgr_r(rb.as_mut_ptr(), cb.as_ptr());
        assert_eq!(ra, rb, "merkle_gen_root mismatch");
    }
}

#[test]
fn test_fors_sign_pk_from_sig() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        // Setup ctx
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = cl.backend.get(b"SPX_initialize_hash_function").unwrap_or_else(|_| {
            cl.core.get(b"SPX_initialize_hash_function").unwrap()
        });
        let init_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_initialize_hash_function").unwrap();
        let fors_sign_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32)> =
            cl.core.get(b"SPX_fors_sign").unwrap();
        let fors_sign_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32)> =
            rl.get(b"SPX_fors_sign").unwrap();
        let fors_pk_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32)> =
            cl.core.get(b"SPX_fors_pk_from_sig").unwrap();
        let fors_pk_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32)> =
            rl.get(b"SPX_fors_pk_from_sig").unwrap();

        // Compute params
        let n = if cfg!(feature = "128s") || cfg!(feature = "128f") { 16 }
                else if cfg!(feature = "192s") || cfg!(feature = "192f") { 24 }
                else { 32 };
        let (fors_height, fors_trees) = if cfg!(feature = "128s") { (12, 14) }
                else if cfg!(feature = "128f") { (6, 33) }
                else if cfg!(feature = "192s") { (14, 17) }
                else if cfg!(feature = "192f") { (8, 33) }
                else if cfg!(feature = "256s") { (14, 22) }
                else { (9, 35) };
        let fors_msg_bytes = (fors_height * fors_trees + 7) / 8;
        let fors_bytes = (fors_height + 1) * fors_trees * n;

        const CTX_SIZE: usize = 2048;
        let mut ca = vec![0u8; CTX_SIZE];
        let mut cb = vec![0u8; CTX_SIZE];
        for i in 0..n { ca[i] = (i as u8).wrapping_mul(13).wrapping_add(1); cb[i] = ca[i]; }
        for i in 0..n { ca[n + i] = (i as u8).wrapping_mul(17).wrapping_add(3); cb[n + i] = ca[n + i]; }
        init_c(ca.as_mut_ptr());
        init_r(cb.as_mut_ptr());

        let m: Vec<u8> = (0..fors_msg_bytes).map(|i| (i * 7 + 11) as u8).collect();
        let addr: [u32; 8] = [0, 0, 0, 0, 0x03000000, 0x12345678, 0, 0];

        let mut sig_a = vec![0u8; fors_bytes];
        let mut sig_b = vec![0u8; fors_bytes];
        let mut pk_a = vec![0u8; n];
        let mut pk_b = vec![0u8; n];

        fors_sign_c(sig_a.as_mut_ptr(), pk_a.as_mut_ptr(), m.as_ptr(), ca.as_ptr(), addr.as_ptr());
        fors_sign_r(sig_b.as_mut_ptr(), pk_b.as_mut_ptr(), m.as_ptr(), cb.as_ptr(), addr.as_ptr());
        assert_eq!(sig_a, sig_b, "fors_sign sig mismatch");
        assert_eq!(pk_a, pk_b, "fors_sign pk mismatch");

        // Now reverse: derive PK from sig
        let mut pk2_a = vec![0u8; n];
        let mut pk2_b = vec![0u8; n];
        fors_pk_c(pk2_a.as_mut_ptr(), sig_a.as_ptr(), m.as_ptr(), ca.as_ptr(), addr.as_ptr());
        fors_pk_r(pk2_b.as_mut_ptr(), sig_b.as_ptr(), m.as_ptr(), cb.as_ptr(), addr.as_ptr());
        assert_eq!(pk2_a, pk2_b, "fors_pk_from_sig mismatch");
        assert_eq!(pk_a, pk2_a, "fors round trip mismatch");
    }
}
