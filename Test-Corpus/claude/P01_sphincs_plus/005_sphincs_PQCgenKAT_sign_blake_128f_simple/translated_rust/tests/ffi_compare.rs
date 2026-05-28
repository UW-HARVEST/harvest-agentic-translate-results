// FFI comparison tests: load both the C .so and Rust .so via libloading,
// invoke matching exports, and compare outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::path::PathBuf;

// ---------------------------------------------------------------------
// Helpers: locate the .so files.
// ---------------------------------------------------------------------

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
    } else if name == "core" {
        q.push("app");
        q.push("libsphincs_core.so");
    }
    q
}

struct CLibs {
    backend: Library,
    core: Library,
}

impl CLibs {
    fn load() -> Self {
        let backend_name = if cfg!(feature = "blake") {
            "blake"
        } else if cfg!(feature = "haraka") {
            "haraka"
        } else if cfg!(feature = "sha2") {
            "sha2"
        } else if cfg!(feature = "shake") {
            "shake"
        } else {
            panic!("no backend feature enabled");
        };
        unsafe {
            // Pre-load libcrypto so RTLD_NOW can resolve EVP_* symbols used
            // by rng.c.
            let lazy_global = libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL;
            let _ = libloading::os::unix::Library::open(
                Some("libcrypto.so"),
                lazy_global,
            ).or_else(|_| {
                libloading::os::unix::Library::open(
                    Some("libcrypto.so.1.0.0"),
                    lazy_global,
                )
            }).or_else(|_| {
                libloading::os::unix::Library::open(
                    Some("libcrypto.so.3"),
                    lazy_global,
                )
            }).or_else(|_| {
                libloading::os::unix::Library::open(
                    Some("libcrypto.so.10"),
                    lazy_global,
                )
            });
            // Use RTLD_LAZY|RTLD_GLOBAL: backend and core circularly reference
            // each other.
            let flags = libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL;
            let backend_os = libloading::os::unix::Library::open(
                Some(find_c_so(backend_name)),
                flags,
            ).expect("c backend lib");
            let backend = Library::from(backend_os);
            let core_os = libloading::os::unix::Library::open(
                Some(find_c_so("core_det")),
                flags,
            ).expect("c core lib");
            let core = Library::from(core_os);
            CLibs { backend, core }
        }
    }
}

fn load_rust() -> Library {
    unsafe { Library::new(find_rust_so()).expect("rust lib") }
}

// Resolve a symbol from either the core or backend library.
unsafe fn get_either<'a, T>(libs: &'a CLibs, name: &[u8]) -> Symbol<'a, T> {
    unsafe {
        if let Ok(s) = libs.core.get::<T>(name) {
            return s;
        }
        libs.backend.get::<T>(name).unwrap_or_else(|e| panic!("symbol {} not found: {}", String::from_utf8_lossy(name), e))
    }
}

// ---------------------------------------------------------------------
// SPX_N and other params from the cargo features (must match C).
// ---------------------------------------------------------------------
#[cfg(feature = "128s")]
const SPX_N: usize = 16;
#[cfg(feature = "128f")]
const SPX_N: usize = 16;
#[cfg(feature = "192s")]
const SPX_N: usize = 24;
#[cfg(feature = "192f")]
const SPX_N: usize = 24;
#[cfg(feature = "256s")]
const SPX_N: usize = 32;
#[cfg(feature = "256f")]
const SPX_N: usize = 32;

// SpxCtx size depends on backend.
fn ctx_buf() -> Vec<u8> {
    // Generous size: 2*SPX_N + 40 + 72 + 10*8*8 + 10*8*4 = 2*SPX_N + 1192
    vec![0u8; 2 * SPX_N + 2048]
}

// ---------------------------------------------------------------------
// Test: SPX_u32_to_bytes
// ---------------------------------------------------------------------
#[test]
fn test_spx_u32_to_bytes() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, u32)> =
            get_either(&cl, b"SPX_u32_to_bytes");
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, u32)> =
            rl.get(b"SPX_u32_to_bytes").unwrap();
        for v in [0u32, 1, 0xdeadbeef, 0xffffffff, 0x12345678] {
            let mut a = [0u8; 4];
            let mut b = [0u8; 4];
            c_fn(a.as_mut_ptr(), v);
            r_fn(b.as_mut_ptr(), v);
            assert_eq!(a, b, "u32_to_bytes mismatch for {:08x}", v);
        }
    }
}

// ---------------------------------------------------------------------
// Test: SPX_bytes_to_ull and SPX_ull_to_bytes
// ---------------------------------------------------------------------
#[test]
fn test_spx_bytes_ull_roundtrip() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn_to: Symbol<unsafe extern "C" fn(*mut u8, u32, u64)> =
            get_either(&cl, b"SPX_ull_to_bytes");
        let r_fn_to: Symbol<unsafe extern "C" fn(*mut u8, u32, u64)> =
            rl.get(b"SPX_ull_to_bytes").unwrap();
        let c_fn_from: Symbol<unsafe extern "C" fn(*const u8, u32) -> u64> =
            get_either(&cl, b"SPX_bytes_to_ull");
        let r_fn_from: Symbol<unsafe extern "C" fn(*const u8, u32) -> u64> =
            rl.get(b"SPX_bytes_to_ull").unwrap();
        for v in [0u64, 1, 0x123456789abcdef0, 0xffffffffffffffff] {
            for outlen in [1u32, 2, 4, 8] {
                let mut a = vec![0u8; outlen as usize];
                let mut b = vec![0u8; outlen as usize];
                c_fn_to(a.as_mut_ptr(), outlen, v);
                r_fn_to(b.as_mut_ptr(), outlen, v);
                assert_eq!(a, b, "ull_to_bytes mismatch v={:x} len={}", v, outlen);
                let cv = c_fn_from(a.as_ptr(), outlen);
                let rv = r_fn_from(b.as_ptr(), outlen);
                assert_eq!(cv, rv);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Test: SPX_set_*_addr family
// ---------------------------------------------------------------------
#[test]
fn test_spx_address_setters() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        macro_rules! cmp_setter {
            ($name:literal, $val:expr, $val_type:ty) => {{
                let c_fn: Symbol<unsafe extern "C" fn(*mut u32, $val_type)> =
                    cl.core.get($name.as_bytes()).unwrap();
                let r_fn: Symbol<unsafe extern "C" fn(*mut u32, $val_type)> =
                    rl.get($name.as_bytes()).unwrap();
                let mut a = [0u32; 8];
                let mut b = [0u32; 8];
                for i in 0..8 {
                    a[i] = (i as u32) * 0x10101010 + 0x07060504;
                    b[i] = a[i];
                }
                c_fn(a.as_mut_ptr(), $val);
                r_fn(b.as_mut_ptr(), $val);
                assert_eq!(a, b, "{} mismatch", $name);
            }};
        }
        cmp_setter!("SPX_set_layer_addr", 0x42u32, u32);
        cmp_setter!("SPX_set_type", 0x03u32, u32);
        cmp_setter!("SPX_set_keypair_addr", 0xdeadbeefu32, u32);
        cmp_setter!("SPX_set_chain_addr", 0x55u32, u32);
        cmp_setter!("SPX_set_hash_addr", 0xaau32, u32);
        cmp_setter!("SPX_set_tree_height", 0x42u32, u32);
        cmp_setter!("SPX_set_tree_index", 0xdeadbeefu32, u32);
        cmp_setter!("SPX_set_tree_addr", 0x123456789abcdef0u64, u64);
    }
}

// ---------------------------------------------------------------------
// Test: SPX_copy_subtree_addr / SPX_copy_keypair_addr
// ---------------------------------------------------------------------
#[test]
fn test_spx_address_copies() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
            let c_fn: Symbol<unsafe extern "C" fn(*mut u32, *const u32)> =
                cl.core.get(name.as_bytes()).unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(*mut u32, *const u32)> =
                rl.get(name.as_bytes()).unwrap();
            let src: [u32; 8] = [0x11111111, 0x22222222, 0x33333333, 0x44444444,
                                 0x55555555, 0x66666666, 0x77777777, 0x88888888];
            let mut a = [0xAAAAAAAAu32; 8];
            let mut b = [0xAAAAAAAAu32; 8];
            c_fn(a.as_mut_ptr(), src.as_ptr());
            r_fn(b.as_mut_ptr(), src.as_ptr());
            assert_eq!(a, b, "{} mismatch", name);
        }
    }
}

// ---------------------------------------------------------------------
// Test: blake256, blake512 (only blake feature)
// ---------------------------------------------------------------------
#[cfg(feature = "blake")]
#[test]
fn test_blake256() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            cl.backend.get(b"blake256").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            rl.get(b"blake256").unwrap();
        for inlen in [0usize, 1, 32, 64, 100, 256, 1000] {
            let input: Vec<u8> = (0..inlen).map(|i| (i * 7 + 3) as u8).collect();
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            c_fn(a.as_mut_ptr(), input.as_ptr(), inlen as u64);
            r_fn(b.as_mut_ptr(), input.as_ptr(), inlen as u64);
            assert_eq!(a, b, "blake256 mismatch inlen={}", inlen);
        }
    }
}

#[cfg(feature = "blake")]
#[test]
fn test_blake512() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            cl.backend.get(b"blake512").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            rl.get(b"blake512").unwrap();
        for inlen in [0usize, 1, 32, 64, 100, 128, 256, 1000] {
            let input: Vec<u8> = (0..inlen).map(|i| (i * 11 + 7) as u8).collect();
            let mut a = [0u8; 64];
            let mut b = [0u8; 64];
            c_fn(a.as_mut_ptr(), input.as_ptr(), inlen as u64);
            r_fn(b.as_mut_ptr(), input.as_ptr(), inlen as u64);
            assert_eq!(a, b, "blake512 mismatch inlen={}", inlen);
        }
    }
}

#[cfg(feature = "blake")]
#[test]
fn test_blake256_mgf1() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong)> =
            cl.backend.get(b"SPX_blake256_mgf1").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong)> =
            rl.get(b"SPX_blake256_mgf1").unwrap();
        for &(inlen, outlen) in &[(8usize, 16usize), (32, 64), (50, 100), (16, 32)] {
            let input: Vec<u8> = (0..inlen).map(|i| (i * 5 + 1) as u8).collect();
            let mut a = vec![0u8; outlen];
            let mut b = vec![0u8; outlen];
            c_fn(a.as_mut_ptr(), outlen as core::ffi::c_ulong, input.as_ptr(), inlen as core::ffi::c_ulong);
            r_fn(b.as_mut_ptr(), outlen as core::ffi::c_ulong, input.as_ptr(), inlen as core::ffi::c_ulong);
            assert_eq!(a, b, "blake256_mgf1 mismatch in={} out={}", inlen, outlen);
        }
    }
}

// ---------------------------------------------------------------------
// Test: SPX_initialize_hash_function (verify by writing same pub_seed/sk_seed
// then comparing the resulting context buffer byte-for-byte).
// We need a context layout that matches. For blake/shake backends, the C ctx
// is just pub_seed[N] || sk_seed[N]; for sha2 it's that plus state arrays;
// for haraka it's plus tweaked tables.
// ---------------------------------------------------------------------
#[test]
fn test_initialize_hash_function() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8)> =
            get_either(&cl, b"SPX_initialize_hash_function");
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8)> =
            rl.get(b"SPX_initialize_hash_function").unwrap();
        let mut a = ctx_buf();
        let mut b = ctx_buf();
        // initialize seeds
        for i in 0..SPX_N {
            a[i] = (i as u8).wrapping_mul(7);
            a[SPX_N + i] = (i as u8).wrapping_mul(11);
            b[i] = a[i];
            b[SPX_N + i] = a[SPX_N + i];
        }
        c_fn(a.as_mut_ptr());
        r_fn(b.as_mut_ptr());
        // For most backends, only the first 2*SPX_N bytes (seeds) are touched
        // for blake/shake. For sha2 and haraka, more is touched. Compare only
        // those bytes that the function definitely sets.
        // Compare the entire buffer up to the SpxCtx size assumption.
        let cmp_len = if cfg!(feature = "blake") || cfg!(feature = "shake") {
            2 * SPX_N
        } else if cfg!(feature = "sha2") {
            #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
            { 2 * SPX_N + 40 + 72 }
            #[cfg(any(feature = "128s", feature = "128f"))]
            { 2 * SPX_N + 40 }
        } else {
            // haraka: 2*N + 10*8*8 + 10*8*4
            2 * SPX_N + 10 * 8 * 8 + 10 * 8 * 4
        };
        assert_eq!(&a[..cmp_len], &b[..cmp_len], "ctx mismatch after init");
    }
}

// ---------------------------------------------------------------------
// Test: SPX_prf_addr
// ---------------------------------------------------------------------
#[test]
fn test_prf_addr() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = get_either(&cl, b"SPX_initialize_hash_function");
        let init_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_initialize_hash_function").unwrap();
        let prf_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u32)> =
            get_either(&cl, b"SPX_prf_addr");
        let prf_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u32)> =
            rl.get(b"SPX_prf_addr").unwrap();
        let mut a = ctx_buf();
        let mut b = ctx_buf();
        for i in 0..SPX_N {
            a[i] = (i as u8).wrapping_mul(13).wrapping_add(2);
            a[SPX_N + i] = (i as u8).wrapping_mul(17).wrapping_add(5);
            b[i] = a[i];
            b[SPX_N + i] = a[SPX_N + i];
        }
        init_c(a.as_mut_ptr());
        init_r(b.as_mut_ptr());
        let addr: [u32; 8] = [0x01020304, 0x05060708, 0x090a0b0c, 0x0d0e0f10,
                              0x11121314, 0x15161718, 0x191a1b1c, 0x1d1e1f20];
        let mut oa = vec![0u8; SPX_N];
        let mut ob = vec![0u8; SPX_N];
        prf_c(oa.as_mut_ptr(), a.as_ptr(), addr.as_ptr());
        prf_r(ob.as_mut_ptr(), b.as_ptr(), addr.as_ptr());
        assert_eq!(oa, ob, "prf_addr mismatch");
    }
}

// ---------------------------------------------------------------------
// Test: SPX_thash with various inblocks
// ---------------------------------------------------------------------
#[test]
fn test_thash() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = get_either(&cl, b"SPX_initialize_hash_function");
        let init_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_initialize_hash_function").unwrap();
        let thash_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32)> =
            get_either(&cl, b"SPX_thash");
        let thash_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32)> =
            rl.get(b"SPX_thash").unwrap();
        let mut a = ctx_buf();
        let mut b = ctx_buf();
        for i in 0..SPX_N {
            a[i] = (i as u8).wrapping_mul(3).wrapping_add(1);
            a[SPX_N + i] = (i as u8).wrapping_mul(5).wrapping_add(2);
            b[i] = a[i];
            b[SPX_N + i] = a[SPX_N + i];
        }
        init_c(a.as_mut_ptr());
        init_r(b.as_mut_ptr());
        for &inblocks in &[1u32, 2u32, 8u32] {
            let inlen = inblocks as usize * SPX_N;
            let input: Vec<u8> = (0..inlen).map(|i| (i * 7 + 11) as u8).collect();
            let mut addr_a: [u32; 8] = [0x10203040, 0, 0x50607080, 0,
                                        0, 0, 0, 0];
            let mut addr_b = addr_a;
            let mut oa = vec![0u8; SPX_N];
            let mut ob = vec![0u8; SPX_N];
            thash_c(oa.as_mut_ptr(), input.as_ptr(), inblocks, a.as_ptr(), addr_a.as_mut_ptr());
            thash_r(ob.as_mut_ptr(), input.as_ptr(), inblocks, b.as_ptr(), addr_b.as_mut_ptr());
            assert_eq!(oa, ob, "thash inblocks={} mismatch", inblocks);
        }
    }
}

// ---------------------------------------------------------------------
// Test: SPX_chain_lengths
// ---------------------------------------------------------------------
#[test]
fn test_chain_lengths() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, *const u8)> =
            get_either(&cl, b"SPX_chain_lengths");
        let r_fn: Symbol<unsafe extern "C" fn(*mut u32, *const u8)> =
            rl.get(b"SPX_chain_lengths").unwrap();

        // SPX_WOTS_LEN1 = 8*N/4, SPX_WOTS_LEN2 = N<=8?2: N<=136?3:4 (so 3 for N>=16)
        let len1 = 8 * SPX_N / 4;
        let len2 = if SPX_N <= 8 { 2 } else if SPX_N <= 136 { 3 } else { 4 };
        let total = len1 + len2;
        let msg: Vec<u8> = (0..SPX_N).map(|i| (i * 13 + 9) as u8).collect();
        let mut a = vec![0u32; total];
        let mut b = vec![0u32; total];
        c_fn(a.as_mut_ptr(), msg.as_ptr());
        r_fn(b.as_mut_ptr(), msg.as_ptr());
        assert_eq!(a, b, "chain_lengths mismatch");
    }
}

// ---------------------------------------------------------------------
// Test: SPX_compute_root
// ---------------------------------------------------------------------
#[test]
fn test_compute_root() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        let init_c: Symbol<unsafe extern "C" fn(*mut u8)> = get_either(&cl, b"SPX_initialize_hash_function");
        let init_r: Symbol<unsafe extern "C" fn(*mut u8)> = rl.get(b"SPX_initialize_hash_function").unwrap();
        let cr_c: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32)> =
            get_either(&cl, b"SPX_compute_root");
        let cr_r: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32)> =
            rl.get(b"SPX_compute_root").unwrap();
        let mut a = ctx_buf();
        let mut b = ctx_buf();
        for i in 0..SPX_N {
            a[i] = (i as u8).wrapping_mul(3);
            a[SPX_N + i] = (i as u8).wrapping_mul(5);
            b[i] = a[i];
            b[SPX_N + i] = a[SPX_N + i];
        }
        init_c(a.as_mut_ptr());
        init_r(b.as_mut_ptr());
        let height = 5u32;
        let leaf: Vec<u8> = (0..SPX_N).map(|i| (i * 7) as u8).collect();
        let auth: Vec<u8> = (0..SPX_N * height as usize).map(|i| (i * 9 + 1) as u8).collect();
        let mut root_a = vec![0u8; SPX_N];
        let mut root_b = vec![0u8; SPX_N];
        let mut addr_a = [0u32; 8];
        let mut addr_b = [0u32; 8];
        cr_c(root_a.as_mut_ptr(), leaf.as_ptr(), 7, 1, auth.as_ptr(), height, a.as_ptr(), addr_a.as_mut_ptr());
        cr_r(root_b.as_mut_ptr(), leaf.as_ptr(), 7, 1, auth.as_ptr(), height, b.as_ptr(), addr_b.as_mut_ptr());
        assert_eq!(root_a, root_b, "compute_root mismatch");
        assert_eq!(addr_a, addr_b, "compute_root addr mismatch");
    }
}

// ---------------------------------------------------------------------
// Test: end-to-end signing - crypto_sign_keypair, crypto_sign, crypto_sign_open
// ---------------------------------------------------------------------
#[test]
fn test_e2e_keypair_sign_verify() {
    let cl = CLibs::load();
    let rl = load_rust();
    unsafe {
        // Both libs use the deterministic randombytes_init/randombytes (rng.c).
        let init_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> = cl.core.get(b"randombytes_init").unwrap();
        let init_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> = rl.get(b"randombytes_init").unwrap();

        let kp_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> = cl.core.get(b"crypto_sign_keypair").unwrap();
        let kp_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> = rl.get(b"crypto_sign_keypair").unwrap();

        let sign_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            cl.core.get(b"crypto_sign").unwrap();
        let sign_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            rl.get(b"crypto_sign").unwrap();

        let open_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            cl.core.get(b"crypto_sign_open").unwrap();
        let open_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            rl.get(b"crypto_sign_open").unwrap();

        let pkbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_publickeybytes").unwrap();
        let skbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_secretkeybytes").unwrap();
        let sigbytes_c: Symbol<unsafe extern "C" fn() -> u64> = cl.core.get(b"crypto_sign_bytes").unwrap();

        let pk_len = pkbytes_c() as usize;
        let sk_len = skbytes_c() as usize;
        let sig_len = sigbytes_c() as usize;

        // Verify Rust returns same sizes
        let pkbytes_r: Symbol<unsafe extern "C" fn() -> u64> = rl.get(b"crypto_sign_publickeybytes").unwrap();
        let skbytes_r: Symbol<unsafe extern "C" fn() -> u64> = rl.get(b"crypto_sign_secretkeybytes").unwrap();
        let sigbytes_r: Symbol<unsafe extern "C" fn() -> u64> = rl.get(b"crypto_sign_bytes").unwrap();
        assert_eq!(pkbytes_r() as usize, pk_len);
        assert_eq!(skbytes_r() as usize, sk_len);
        assert_eq!(sigbytes_r() as usize, sig_len);

        // Init both PRGs with the same seed
        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }
        let mut entropy_c = entropy;
        init_c(entropy_c.as_mut_ptr(), std::ptr::null_mut());
        let mut entropy_r = entropy;
        init_r(entropy_r.as_mut_ptr(), std::ptr::null_mut());

        // Generate keys
        let mut pk_c = vec![0u8; pk_len];
        let mut sk_c = vec![0u8; sk_len];
        let mut pk_r = vec![0u8; pk_len];
        let mut sk_r = vec![0u8; sk_len];
        kp_c(pk_c.as_mut_ptr(), sk_c.as_mut_ptr());
        kp_r(pk_r.as_mut_ptr(), sk_r.as_mut_ptr());
        assert_eq!(pk_c, pk_r, "pk mismatch");
        assert_eq!(sk_c, sk_r, "sk mismatch");

        // Sign a message
        let msg: Vec<u8> = (0..33).map(|i| (i * 7 + 5) as u8).collect();
        let mut sm_c = vec![0u8; sig_len + msg.len()];
        let mut sm_r = vec![0u8; sig_len + msg.len()];
        let mut smlen_c: u64 = 0;
        let mut smlen_r: u64 = 0;
        sign_c(sm_c.as_mut_ptr(), &mut smlen_c, msg.as_ptr(), msg.len() as u64, sk_c.as_ptr());
        sign_r(sm_r.as_mut_ptr(), &mut smlen_r, msg.as_ptr(), msg.len() as u64, sk_r.as_ptr());
        assert_eq!(smlen_c, smlen_r, "smlen mismatch");
        assert_eq!(&sm_c[..smlen_c as usize], &sm_r[..smlen_r as usize], "signature mismatch");

        // Verify
        let mut m_out_c = vec![0u8; sm_c.len()];
        let mut m_out_r = vec![0u8; sm_r.len()];
        let mut mlen_c: u64 = 0;
        let mut mlen_r: u64 = 0;
        let r1 = open_c(m_out_c.as_mut_ptr(), &mut mlen_c, sm_c.as_ptr(), smlen_c, pk_c.as_ptr());
        let r2 = open_r(m_out_r.as_mut_ptr(), &mut mlen_r, sm_r.as_ptr(), smlen_r, pk_r.as_ptr());
        assert_eq!(r1, 0); assert_eq!(r2, 0);
        assert_eq!(mlen_c, msg.len() as u64);
        assert_eq!(mlen_r, msg.len() as u64);
        assert_eq!(&m_out_c[..mlen_c as usize], &msg[..]);
        assert_eq!(&m_out_r[..mlen_r as usize], &msg[..]);
    }
}
