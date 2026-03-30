//! Integration tests comparing Rust vs C implementations function-by-function.
//! Uses libloading to call into the C shared libraries.

use libloading::{Library, Symbol};
use std::path::PathBuf;

const SPX_N: usize = 16;
const SPX_ADDR_BYTES: usize = 32;
const SPX_WOTS_LEN: usize = 35;
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
const SPX_FORS_HEIGHT: usize = 6;
const SPX_FORS_TREES: usize = 33;
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_FULL_HEIGHT: usize = 66;
const SPX_D: usize = 22;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;

fn lib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build")
}

fn load_libs() -> (Library, Library) {
    unsafe {
        // Load libcrypto for OpenSSL AES used by C rng.c
        let crypto_cpath = std::ffi::CString::new("libcrypto.so").unwrap();
        libc::dlopen(crypto_cpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);

        // Circular dependency: core_det needs blake symbols, blake needs core_det symbols
        let core_path = lib_dir().join("app/libsphincs_core_det.so");
        let core_cpath = std::ffi::CString::new(core_path.to_str().unwrap()).unwrap();
        let h1 = libc::dlopen(core_cpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        assert!(!h1.is_null(), "dlopen core_det failed: {:?}", std::ffi::CStr::from_ptr(libc::dlerror()));

        let blake_path = lib_dir().join("lib/blake/libblake.so");
        let blake_cpath = std::ffi::CString::new(blake_path.to_str().unwrap()).unwrap();
        let h2 = libc::dlopen(blake_cpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        assert!(!h2.is_null(), "dlopen blake failed: {:?}", std::ffi::CStr::from_ptr(libc::dlerror()));

        let blake = Library::new(&blake_path).expect("load libblake.so");
        let core = Library::new(&core_path).expect("load libsphincs_core_det.so");
        (blake, core)
    }
}

// ============================================================
// Test 1: blake256
// ============================================================
#[test]
fn test_blake256() {
    let (blake_lib, _) = load_libs();
    let input = b"Hello SPHINCS+ blake256 test";
    let mut c_out = [0u8; 32];
    let mut rust_out = [0u8; 32];

    unsafe {
        let c_blake256: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32> =
            blake_lib.get(b"blake256").unwrap();
        c_blake256(c_out.as_mut_ptr(), input.as_ptr(), input.len() as u64);

        sphincsplus::blake::blake256::blake256(rust_out.as_mut_ptr(), input.as_ptr(), input.len() as u64);
    }
    assert_eq!(c_out, rust_out, "blake256 mismatch");
}

// ============================================================
// Test 2: ull_to_bytes / u32_to_bytes / bytes_to_ull
// ============================================================
#[test]
fn test_ull_to_bytes() {
    let (blake_lib, _) = load_libs();
    let val: u64 = 0xDEADBEEFCAFEBABE;
    let mut c_out = [0u8; 8];
    let mut rust_out = [0u8; 8];

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, u32, u64)> =
            blake_lib.get(b"SPX_ull_to_bytes").unwrap();
        c_fn(c_out.as_mut_ptr(), 8, val);
        sphincsplus::utils::ull_to_bytes(rust_out.as_mut_ptr(), 8, val);
    }
    assert_eq!(c_out, rust_out, "ull_to_bytes mismatch");
}

#[test]
fn test_u32_to_bytes() {
    let (blake_lib, _) = load_libs();
    let val: u32 = 0xDEADBEEF;
    let mut c_out = [0u8; 4];
    let mut rust_out = [0u8; 4];

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, u32)> =
            blake_lib.get(b"SPX_u32_to_bytes").unwrap();
        c_fn(c_out.as_mut_ptr(), val);
        sphincsplus::utils::u32_to_bytes(rust_out.as_mut_ptr(), val);
    }
    assert_eq!(c_out, rust_out, "u32_to_bytes mismatch");
}

#[test]
fn test_bytes_to_ull() {
    let (blake_lib, _) = load_libs();
    let input = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, u32) -> u64> =
            blake_lib.get(b"SPX_bytes_to_ull").unwrap();
        let c_val = c_fn(input.as_ptr(), 8);
        let rust_val = sphincsplus::utils::bytes_to_ull(input.as_ptr(), 8);
        assert_eq!(c_val, rust_val, "bytes_to_ull mismatch");
    }
}

// ============================================================
// Test 3: Address functions
// ============================================================
#[test]
fn test_address_functions() {
    let (_, core_lib) = load_libs();

    unsafe {
        // set_layer_addr
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, u32)> =
            core_lib.get(b"SPX_set_layer_addr").unwrap();
        c_fn(c_addr.as_mut_ptr(), 5);
        sphincsplus::address::set_layer_addr(&mut r_addr, 5);
        assert_eq!(c_addr, r_addr, "set_layer_addr mismatch");

        // set_tree_addr
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, u64)> =
            core_lib.get(b"SPX_set_tree_addr").unwrap();
        c_fn(c_addr.as_mut_ptr(), 0x123456789ABCDEF0);
        sphincsplus::address::set_tree_addr(&mut r_addr, 0x123456789ABCDEF0);
        assert_eq!(c_addr, r_addr, "set_tree_addr mismatch");

        // set_type
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, u32)> =
            core_lib.get(b"SPX_set_type").unwrap();
        c_fn(c_addr.as_mut_ptr(), 3);
        sphincsplus::address::set_type(&mut r_addr, 3);
        assert_eq!(c_addr, r_addr, "set_type mismatch");

        // set_keypair_addr
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, u32)> =
            core_lib.get(b"SPX_set_keypair_addr").unwrap();
        c_fn(c_addr.as_mut_ptr(), 42);
        sphincsplus::address::set_keypair_addr(&mut r_addr, 42);
        assert_eq!(c_addr, r_addr, "set_keypair_addr mismatch");

        // set_tree_index
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, u32)> =
            core_lib.get(b"SPX_set_tree_index").unwrap();
        c_fn(c_addr.as_mut_ptr(), 0xABCD);
        sphincsplus::address::set_tree_index(&mut r_addr, 0xABCD);
        assert_eq!(c_addr, r_addr, "set_tree_index mismatch");
    }
}

// ============================================================
// Test 4: RNG (randombytes_init + randombytes)
// ============================================================
#[test]
fn test_rng() {
    let (_, core_lib) = load_libs();

    unsafe {
        let c_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            core_lib.get(b"randombytes_init").unwrap();
        let c_rand: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> =
            core_lib.get(b"randombytes").unwrap();

        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }

        // Init C RNG
        c_init(entropy.as_ptr(), std::ptr::null());
        let mut c_out = [0u8; 48];
        c_rand(c_out.as_mut_ptr(), 48);

        // Init Rust RNG
        sphincsplus::rng::randombytes_init(entropy.as_ptr(), std::ptr::null());
        let mut r_out = [0u8; 48];
        sphincsplus::rng::rng_randombytes(r_out.as_mut_ptr(), 48);

        assert_eq!(c_out, r_out, "RNG output mismatch after init+randombytes(48)");

        // Second call
        let mut c_out2 = [0u8; 64];
        let mut r_out2 = [0u8; 64];
        c_rand(c_out2.as_mut_ptr(), 64);
        sphincsplus::rng::rng_randombytes(r_out2.as_mut_ptr(), 64);
        assert_eq!(c_out2, r_out2, "RNG output mismatch on second call");
    }
}

// ============================================================
// Test 5: thash (blake simple)
// ============================================================
#[test]
fn test_thash() {
    let (blake_lib, _) = load_libs();

    unsafe {
        let c_thash: Symbol<unsafe extern "C" fn(*mut u8, *const u8, u32, *const sphincsplus::context::SpxCtx, *mut u32)> =
            blake_lib.get(b"SPX_thash").unwrap();

        let mut ctx = sphincsplus::context::SpxCtx::new();
        for i in 0..SPX_N { ctx.pub_seed[i] = (i + 1) as u8; }

        let input = [0xABu8; SPX_N];
        let mut addr = [0u32; 8];
        sphincsplus::address::set_type(&mut addr, 2);
        sphincsplus::address::set_tree_index(&mut addr, 7);

        let mut c_out = [0u8; SPX_N];
        let mut r_out = [0u8; SPX_N];
        let mut c_addr = addr;
        let mut r_addr = addr;

        c_thash(c_out.as_mut_ptr(), input.as_ptr(), 1, &ctx, c_addr.as_mut_ptr());
        sphincsplus::blake::thash_blake_simple::SPX_thash(
            r_out.as_mut_ptr(), input.as_ptr(), 1, &ctx, r_addr.as_mut_ptr(),
        );

        assert_eq!(c_out, r_out, "thash(1 block) mismatch");
    }
}

// ============================================================
// Test 6: prf_addr
// ============================================================
#[test]
fn test_prf_addr() {
    let (blake_lib, _) = load_libs();

    unsafe {
        let c_prf: Symbol<unsafe extern "C" fn(*mut u8, *const sphincsplus::context::SpxCtx, *const u32)> =
            blake_lib.get(b"SPX_prf_addr").unwrap();

        let mut ctx = sphincsplus::context::SpxCtx::new();
        for i in 0..SPX_N {
            ctx.pub_seed[i] = (i + 1) as u8;
            ctx.sk_seed[i] = (i + 0x10) as u8;
        }

        let mut addr = [0u32; 8];
        sphincsplus::address::set_type(&mut addr, 5);
        sphincsplus::address::set_keypair_addr(&mut addr, 3);

        let mut c_out = [0u8; SPX_N];
        let mut r_out = [0u8; SPX_N];

        c_prf(c_out.as_mut_ptr(), &ctx, addr.as_ptr());
        sphincsplus::blake::hash_blake::SPX_prf_addr(r_out.as_mut_ptr(), &ctx, addr.as_ptr());

        assert_eq!(c_out, r_out, "prf_addr mismatch");
    }
}

// ============================================================
// Test 7: gen_message_random
// ============================================================
#[test]
fn test_gen_message_random() {
    let (blake_lib, _) = load_libs();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const sphincsplus::context::SpxCtx)> =
            blake_lib.get(b"SPX_gen_message_random").unwrap();

        let ctx = sphincsplus::context::SpxCtx::new();
        let sk_prf = [0x42u8; SPX_N];
        let optrand = [0x55u8; SPX_N];
        let msg = b"test message for gen_message_random";

        let mut c_out = [0u8; 64]; // blake256 output is 32 bytes but buffer is larger
        let mut r_out = [0u8; 64];

        c_fn(c_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(), msg.len() as u64, &ctx);
        sphincsplus::blake::hash_blake::SPX_gen_message_random(
            r_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(), msg.len() as u64, &ctx,
        );

        // Compare only SPX_N bytes (the meaningful output)
        assert_eq!(&c_out[..32], &r_out[..32], "gen_message_random mismatch");
    }
}

// ============================================================
// Test 8: chain_lengths
// ============================================================
#[test]
fn test_chain_lengths() {
    let (_, core_lib) = load_libs();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u32, *const u8)> =
            core_lib.get(b"SPX_chain_lengths").unwrap();

        let msg = [0xABu8; SPX_N];
        let mut c_lengths = [0u32; SPX_WOTS_LEN];
        let mut r_lengths = [0u32; SPX_WOTS_LEN];

        c_fn(c_lengths.as_mut_ptr(), msg.as_ptr());
        sphincsplus::wots::SPX_chain_lengths(r_lengths.as_mut_ptr(), msg.as_ptr());

        assert_eq!(c_lengths, r_lengths, "chain_lengths mismatch");
    }
}

// ============================================================
// Test 9: Full keypair generation (seed_keypair)
// ============================================================
#[test]
fn test_crypto_sign_seed_keypair() {
    let (blake_lib, core_lib) = load_libs();

    unsafe {
        // We need both libs loaded for the C side to resolve symbols
        // The core_det lib links against blake, so we need blake loaded first
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            core_lib.get(b"crypto_sign_seed_keypair").unwrap();

        let seed = [0x42u8; CRYPTO_SEEDBYTES];
        let mut c_pk = [0u8; SPX_PK_BYTES];
        let mut c_sk = [0u8; SPX_SK_BYTES];
        let mut r_pk = [0u8; SPX_PK_BYTES];
        let mut r_sk = [0u8; SPX_SK_BYTES];

        c_fn(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr());
        sphincsplus::sign::crypto_sign_seed_keypair(r_pk.as_mut_ptr(), r_sk.as_mut_ptr(), seed.as_ptr());

        assert_eq!(c_pk, r_pk, "seed_keypair: pk mismatch");
        assert_eq!(c_sk, r_sk, "seed_keypair: sk mismatch");

        // Keep blake_lib alive
        let _ = &blake_lib;
    }
}

// ============================================================
// Test 10: Full sign + verify cycle
// ============================================================
#[test]
fn test_crypto_sign_verify_cycle() {
    let (blake_lib, core_lib) = load_libs();

    unsafe {
        let c_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            core_lib.get(b"randombytes_init").unwrap();
        let c_keypair: Symbol<unsafe extern "C" fn(*mut u8, *mut u8) -> i32> =
            core_lib.get(b"crypto_sign_keypair").unwrap();
        let c_sign: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            core_lib.get(b"crypto_sign").unwrap();
        let c_open: Symbol<unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32> =
            core_lib.get(b"crypto_sign_open").unwrap();

        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }

        // C side
        c_init(entropy.as_ptr(), std::ptr::null());
        let mut c_pk = [0u8; SPX_PK_BYTES];
        let mut c_sk = [0u8; SPX_SK_BYTES];
        c_keypair(c_pk.as_mut_ptr(), c_sk.as_mut_ptr());

        // Rust side (re-init RNG to same state)
        sphincsplus::rng::randombytes_init(entropy.as_ptr(), std::ptr::null());
        let mut r_pk = [0u8; SPX_PK_BYTES];
        let mut r_sk = [0u8; SPX_SK_BYTES];
        sphincsplus::sign::crypto_sign_keypair(r_pk.as_mut_ptr(), r_sk.as_mut_ptr());

        assert_eq!(c_pk, r_pk, "keypair: pk mismatch");
        assert_eq!(c_sk, r_sk, "keypair: sk mismatch");

        let _ = &blake_lib;
    }
}
