//! Integration tests comparing C and Rust SPHINCS+ implementations.
//! Loads C shared libraries via libloading and compares outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::path::PathBuf;

// We need to load libcrypto with RTLD_GLOBAL so the C .so can find OpenSSL symbols
fn load_global(path: &str) -> *mut std::ffi::c_void {
    let cpath = std::ffi::CString::new(path).unwrap();
    unsafe { libc::dlopen(cpath.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) }
}

const SPX_N: usize = 16;
const SPX_ADDR_BYTES: usize = 32;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
const SPX_WOTS_LEN: usize = 35; // 32 + 3
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_FORS_HEIGHT: usize = 6;
const SPX_FORS_TREES: usize = 33;
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_D: usize = 22;
const SPX_FULL_HEIGHT: usize = 66;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;

fn lib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build")
}

fn load_libs() -> (Library, Library) {
    let _crypto = load_global("libcrypto.so");
    assert!(!_crypto.is_null(), "failed to load libcrypto.so");
    // Circular dependency: core_det needs blake symbols, blake needs core_det symbols
    // Use RTLD_LAZY | RTLD_GLOBAL to resolve lazily
    let core_path = lib_dir().join("app/libsphincs_core_det.so");
    let cpath = std::ffi::CString::new(core_path.to_str().unwrap()).unwrap();
    let _core_global = unsafe { libc::dlopen(cpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
    assert!(!_core_global.is_null(), "failed to load libsphincs_core_det.so globally");
    let blake_path = lib_dir().join("lib/blake/libblake.so");
    let bpath = std::ffi::CString::new(blake_path.to_str().unwrap()).unwrap();
    let _blake_global = unsafe { libc::dlopen(bpath.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
    assert!(!_blake_global.is_null(), "failed to load libblake.so globally");
    unsafe {
        let blake = Library::new(lib_dir().join("lib/blake/libblake.so")).expect("load blake");
        let core = Library::new(lib_dir().join("app/libsphincs_core_det.so")).expect("load core");
        (blake, core)
    }
}

// ============================================================
// Level 0: utils (ull_to_bytes, u32_to_bytes, bytes_to_ull)
// ============================================================
#[test]
fn test_ull_to_bytes() {
    let (_blake, core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, u32, u64);
    let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_ull_to_bytes").unwrap() };

    for &(outlen, val) in &[(8u32, 0x0102030405060708u64), (4, 0xDEADBEEF), (1, 0xFF), (8, 0)] {
        let mut c_out = vec![0u8; outlen as usize];
        let mut r_out = vec![0u8; outlen as usize];
        unsafe {
            c_fn(c_out.as_mut_ptr(), outlen, val);
            sphincsplus::utils::ull_to_bytes(r_out.as_mut_ptr(), outlen, val);
        }
        assert_eq!(c_out, r_out, "ull_to_bytes mismatch for outlen={outlen} val={val:#x}");
    }
}

#[test]
fn test_u32_to_bytes() {
    let (_blake, core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, u32);
    let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_u32_to_bytes").unwrap() };

    for &val in &[0xDEADBEEFu32, 0, 0xFF, 0x01020304] {
        let mut c_out = [0u8; 4];
        let mut r_out = [0u8; 4];
        unsafe {
            c_fn(c_out.as_mut_ptr(), val);
            sphincsplus::utils::u32_to_bytes(r_out.as_mut_ptr(), val);
        }
        assert_eq!(c_out, r_out, "u32_to_bytes mismatch for val={val:#x}");
    }
}

#[test]
fn test_bytes_to_ull() {
    let (_blake, core) = load_libs();
    type Fn = unsafe extern "C" fn(*const u8, u32) -> u64;
    let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_bytes_to_ull").unwrap() };

    let data = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    for inlen in 1..=8u32 {
        let c_val = unsafe { c_fn(data.as_ptr(), inlen) };
        let r_val = unsafe { sphincsplus::utils::bytes_to_ull(data.as_ptr(), inlen) };
        assert_eq!(c_val, r_val, "bytes_to_ull mismatch for inlen={inlen}");
    }
}

// ============================================================
// Level 1: address functions
// ============================================================
#[test]
fn test_address_functions() {
    let (_blake, core) = load_libs();

    // Test set_layer_addr
    {
        type Fn = unsafe extern "C" fn(*mut u32, u32);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_set_layer_addr").unwrap() };
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        unsafe { c_fn(c_addr.as_mut_ptr(), 5) };
        sphincsplus::address::set_layer_addr(&mut r_addr, 5);
        assert_eq!(c_addr, r_addr, "set_layer_addr mismatch");
    }

    // Test set_tree_addr
    {
        type Fn = unsafe extern "C" fn(*mut u32, u64);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_set_tree_addr").unwrap() };
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        unsafe { c_fn(c_addr.as_mut_ptr(), 0x123456789ABCDEF0) };
        sphincsplus::address::set_tree_addr(&mut r_addr, 0x123456789ABCDEF0);
        assert_eq!(c_addr, r_addr, "set_tree_addr mismatch");
    }

    // Test set_type
    {
        type Fn = unsafe extern "C" fn(*mut u32, u32);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_set_type").unwrap() };
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        unsafe { c_fn(c_addr.as_mut_ptr(), 3) };
        sphincsplus::address::set_type(&mut r_addr, 3);
        assert_eq!(c_addr, r_addr, "set_type mismatch");
    }

    // Test set_keypair_addr
    {
        type Fn = unsafe extern "C" fn(*mut u32, u32);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_set_keypair_addr").unwrap() };
        let mut c_addr = [0u32; 8];
        let mut r_addr = [0u32; 8];
        unsafe { c_fn(c_addr.as_mut_ptr(), 42) };
        sphincsplus::address::set_keypair_addr(&mut r_addr, 42);
        assert_eq!(c_addr, r_addr, "set_keypair_addr mismatch");
    }

    // Test copy_subtree_addr
    {
        type Fn = unsafe extern "C" fn(*mut u32, *const u32);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_copy_subtree_addr").unwrap() };
        let src = [1u32, 2, 3, 4, 5, 6, 7, 8];
        let mut c_out = [0u32; 8];
        let mut r_out = [0u32; 8];
        unsafe { c_fn(c_out.as_mut_ptr(), src.as_ptr()) };
        sphincsplus::address::copy_subtree_addr(&mut r_out, &src);
        assert_eq!(c_out, r_out, "copy_subtree_addr mismatch");
    }

    // Test copy_keypair_addr
    {
        type Fn = unsafe extern "C" fn(*mut u32, *const u32);
        let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_copy_keypair_addr").unwrap() };
        let src = [0xAAu32, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22];
        let mut c_out = [0u32; 8];
        let mut r_out = [0u32; 8];
        unsafe { c_fn(c_out.as_mut_ptr(), src.as_ptr()) };
        sphincsplus::address::copy_keypair_addr(&mut r_out, &src);
        assert_eq!(c_out, r_out, "copy_keypair_addr mismatch");
    }

    // Test set_chain_addr, set_hash_addr, set_tree_height, set_tree_index
    {
        type Fn = unsafe extern "C" fn(*mut u32, u32);
        for (name, val) in [
            ("SPX_set_chain_addr", 7u32),
            ("SPX_set_hash_addr", 15),
            ("SPX_set_tree_height", 3),
            ("SPX_set_tree_index", 0x12345),
        ] {
            let c_fn: Symbol<Fn> = unsafe { core.get(name.as_bytes()).unwrap() };
            let mut c_addr = [0u32; 8];
            let mut r_addr = [0u32; 8];
            unsafe { c_fn(c_addr.as_mut_ptr(), val) };
            match name {
                "SPX_set_chain_addr" => sphincsplus::address::set_chain_addr(&mut r_addr, val),
                "SPX_set_hash_addr" => sphincsplus::address::set_hash_addr(&mut r_addr, val),
                "SPX_set_tree_height" => sphincsplus::address::set_tree_height(&mut r_addr, val),
                "SPX_set_tree_index" => sphincsplus::address::set_tree_index(&mut r_addr, val),
                _ => unreachable!(),
            }
            assert_eq!(c_addr, r_addr, "{name} mismatch");
        }
    }
}

// ============================================================
// Level 2: hash functions (blake backend)
// ============================================================
#[test]
fn test_initialize_hash_function() {
    let (blake, _core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut sphincsplus::context::SpxCtx);
    let c_fn: Symbol<Fn> = unsafe { blake.get(b"SPX_initialize_hash_function").unwrap() };

    let mut c_ctx = sphincsplus::context::SpxCtx::new();
    let mut r_ctx = sphincsplus::context::SpxCtx::new();
    c_ctx.pub_seed = [0x42; SPX_N];
    r_ctx.pub_seed = [0x42; SPX_N];
    unsafe { c_fn(&mut c_ctx) };
    unsafe { sphincsplus::blake::hash_blake::SPX_initialize_hash_function(&mut r_ctx) };
    // For blake, this is a no-op, but verify no corruption
    assert_eq!(c_ctx.pub_seed, r_ctx.pub_seed);
}

#[test]
fn test_prf_addr() {
    let (blake, _core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, *const sphincsplus::context::SpxCtx, *const u32);
    let c_fn: Symbol<Fn> = unsafe { blake.get(b"SPX_prf_addr").unwrap() };

    let mut ctx = sphincsplus::context::SpxCtx::new();
    for i in 0..SPX_N { ctx.pub_seed[i] = (i * 3 + 1) as u8; ctx.sk_seed[i] = (i * 7 + 2) as u8; }
    let addr = [1u32, 2, 3, 4, 5, 6, 7, 8];

    let mut c_out = [0u8; SPX_N];
    let mut r_out = [0u8; SPX_N];
    unsafe {
        c_fn(c_out.as_mut_ptr(), &ctx, addr.as_ptr());
        sphincsplus::blake::hash_blake::SPX_prf_addr(r_out.as_mut_ptr(), &ctx, addr.as_ptr());
    }
    assert_eq!(c_out, r_out, "prf_addr mismatch");
}

#[test]
fn test_thash() {
    let (blake, _core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, *const u8, u32, *const sphincsplus::context::SpxCtx, *mut u32);
    let c_fn: Symbol<Fn> = unsafe { blake.get(b"SPX_thash").unwrap() };

    let mut ctx = sphincsplus::context::SpxCtx::new();
    for i in 0..SPX_N { ctx.pub_seed[i] = (i + 10) as u8; }
    let input = [0xABu8; SPX_N * 2];
    let mut addr = [0u32; 8];
    let mut addr2 = [0u32; 8];

    for inblocks in [1u32, 2] {
        let mut c_out = [0u8; SPX_N];
        let mut r_out = [0u8; SPX_N];
        addr.fill(0);
        addr2.fill(0);
        unsafe {
            c_fn(c_out.as_mut_ptr(), input.as_ptr(), inblocks, &ctx, addr.as_mut_ptr());
            sphincsplus::blake::thash_blake_simple::SPX_thash(
                r_out.as_mut_ptr(), input.as_ptr(), inblocks, &ctx, addr2.as_mut_ptr(),
            );
        }
        assert_eq!(c_out, r_out, "thash mismatch for inblocks={inblocks}");
    }
}

#[test]
fn test_gen_message_random() {
    let (blake, _core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const sphincsplus::context::SpxCtx);
    let c_fn: Symbol<Fn> = unsafe { blake.get(b"SPX_gen_message_random").unwrap() };

    let ctx = sphincsplus::context::SpxCtx::new();
    let sk_prf = [0x11u8; SPX_N];
    let optrand = [0x22u8; SPX_N];
    let msg = [0x33u8; 100];

    let mut c_out = [0u8; SPX_N];
    let mut r_out = [0u8; SPX_N];
    unsafe {
        c_fn(c_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(), 100, &ctx);
        sphincsplus::blake::hash_blake::SPX_gen_message_random(
            r_out.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(), 100, &ctx,
        );
    }
    assert_eq!(c_out, r_out, "gen_message_random mismatch");
}

#[test]
fn test_hash_message() {
    let (blake, _core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const sphincsplus::context::SpxCtx);
    let c_fn: Symbol<Fn> = unsafe { blake.get(b"SPX_hash_message").unwrap() };

    let ctx = sphincsplus::context::SpxCtx::new();
    let r = [0x44u8; SPX_N];
    let pk = [0x55u8; SPX_PK_BYTES];
    let msg = [0x66u8; 200];

    let (mut c_digest, mut r_digest) = ([0u8; SPX_FORS_MSG_BYTES], [0u8; SPX_FORS_MSG_BYTES]);
    let (mut c_tree, mut r_tree) = (0u64, 0u64);
    let (mut c_leaf, mut r_leaf) = (0u32, 0u32);

    unsafe {
        c_fn(c_digest.as_mut_ptr(), &mut c_tree, &mut c_leaf, r.as_ptr(), pk.as_ptr(), msg.as_ptr(), 200, &ctx);
        sphincsplus::blake::hash_blake::SPX_hash_message(
            r_digest.as_mut_ptr(), &mut r_tree, &mut r_leaf, r.as_ptr(), pk.as_ptr(), msg.as_ptr(), 200, &ctx,
        );
    }
    assert_eq!(c_digest, r_digest, "hash_message digest mismatch");
    assert_eq!(c_tree, r_tree, "hash_message tree mismatch");
    assert_eq!(c_leaf, r_leaf, "hash_message leaf mismatch");
}

// ============================================================
// Level 3: wots (chain_lengths)
// ============================================================
#[test]
fn test_chain_lengths() {
    let (_blake, core) = load_libs();
    type Fn = unsafe extern "C" fn(*mut u32, *const u8);
    let c_fn: Symbol<Fn> = unsafe { core.get(b"SPX_chain_lengths").unwrap() };

    let msg = [0xABu8; SPX_N];
    let mut c_lengths = [0u32; SPX_WOTS_LEN];
    let mut r_lengths = [0u32; SPX_WOTS_LEN];
    unsafe { c_fn(c_lengths.as_mut_ptr(), msg.as_ptr()) };
    sphincsplus::wots::chain_lengths(&mut r_lengths, &msg);
    assert_eq!(c_lengths, r_lengths, "chain_lengths mismatch");
}

// ============================================================
// Level 4: RNG (AES256_CTR_DRBG_Update, randombytes_init, randombytes)
// ============================================================
#[test]
fn test_aes256_ctr_drbg_update() {
    let (_blake, core) = load_libs();
    type Fn = unsafe extern "C" fn(*const u8, *mut u8, *mut u8);
    let c_fn: Symbol<Fn> = unsafe { core.get(b"AES256_CTR_DRBG_Update").unwrap() };

    let provided = [0x42u8; 48];
    let (mut c_key, mut r_key) = ([0u8; 32], [0u8; 32]);
    let (mut c_v, mut r_v) = ([0u8; 16], [0u8; 16]);

    unsafe {
        c_fn(provided.as_ptr(), c_key.as_mut_ptr(), c_v.as_mut_ptr());
        sphincsplus::rng::AES256_CTR_DRBG_Update(provided.as_ptr(), r_key.as_mut_ptr(), r_v.as_mut_ptr());
    }
    assert_eq!(c_key, r_key, "AES256_CTR_DRBG_Update key mismatch");
    assert_eq!(c_v, r_v, "AES256_CTR_DRBG_Update V mismatch");
}

#[test]
fn test_rng_deterministic() {
    let (_blake, core) = load_libs();
    type InitFn = unsafe extern "C" fn(*const u8, *const u8);
    type RandFn = unsafe extern "C" fn(*mut u8, u64) -> i32;
    let c_init: Symbol<InitFn> = unsafe { core.get(b"randombytes_init").unwrap() };
    let c_rand: Symbol<RandFn> = unsafe { core.get(b"randombytes").unwrap() };

    let entropy = {
        let mut e = [0u8; 48];
        for i in 0..48 { e[i] = i as u8; }
        e
    };

    // C side
    let mut c_out = [0u8; 48];
    unsafe {
        c_init(entropy.as_ptr(), std::ptr::null());
        c_rand(c_out.as_mut_ptr(), 48);
    }

    // Rust side
    let mut r_out = [0u8; 48];
    unsafe {
        sphincsplus::rng::randombytes_init(entropy.as_ptr(), std::ptr::null());
        sphincsplus::rng::rng_randombytes(r_out.as_mut_ptr(), 48);
    }

    assert_eq!(c_out, r_out, "randombytes deterministic output mismatch");
}

// ============================================================
// Level 5: crypto_sign_seed_keypair (end-to-end with deterministic seed)
// ============================================================
#[test]
fn test_crypto_sign_seed_keypair() {
    let (blake, core) = load_libs();

    // We need both libs loaded for the C side since core depends on blake
    type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    let c_fn: Symbol<KeypairFn> = unsafe { core.get(b"crypto_sign_seed_keypair").unwrap() };

    // Need to also load blake's init function for the C side
    type InitHashFn = unsafe extern "C" fn(*mut sphincsplus::context::SpxCtx);
    let _c_init_hash: Symbol<InitHashFn> = unsafe { blake.get(b"SPX_initialize_hash_function").unwrap() };

    let seed = [0x42u8; CRYPTO_SEEDBYTES];
    let (mut c_pk, mut r_pk) = ([0u8; SPX_PK_BYTES], [0u8; SPX_PK_BYTES]);
    let (mut c_sk, mut r_sk) = ([0u8; SPX_SK_BYTES], [0u8; SPX_SK_BYTES]);

    unsafe {
        c_fn(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr());
        sphincsplus::sign::crypto_sign_seed_keypair(r_pk.as_mut_ptr(), r_sk.as_mut_ptr(), seed.as_ptr());
    }
    assert_eq!(c_pk, r_pk, "crypto_sign_seed_keypair pk mismatch");
    assert_eq!(c_sk, r_sk, "crypto_sign_seed_keypair sk mismatch");
}

// ============================================================
// Level 6: Full sign/verify cycle with deterministic RNG
// ============================================================
#[test]
fn test_crypto_sign_verify_deterministic() {
    let (_blake, core) = load_libs();

    type InitFn = unsafe extern "C" fn(*const u8, *const u8);
    type RandFn = unsafe extern "C" fn(*mut u8, u64) -> i32;
    type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    type OpenFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;

    let c_init: Symbol<InitFn> = unsafe { core.get(b"randombytes_init").unwrap() };
    let c_rand: Symbol<RandFn> = unsafe { core.get(b"randombytes").unwrap() };
    let c_keypair: Symbol<KeypairFn> = unsafe { core.get(b"crypto_sign_keypair").unwrap() };
    let c_sign: Symbol<SignFn> = unsafe { core.get(b"crypto_sign").unwrap() };
    let c_open: Symbol<OpenFn> = unsafe { core.get(b"crypto_sign_open").unwrap() };

    let entropy = {
        let mut e = [0u8; 48];
        for i in 0..48 { e[i] = i as u8; }
        e
    };

    // C side
    unsafe { c_init(entropy.as_ptr(), std::ptr::null()) };
    let mut c_pk = [0u8; SPX_PK_BYTES];
    let mut c_sk = [0u8; SPX_SK_BYTES];
    unsafe { c_keypair(c_pk.as_mut_ptr(), c_sk.as_mut_ptr()) };

    let msg = b"test message for sphincs+";
    let mlen = msg.len() as u64;
    let mut c_sm = vec![0u8; SPX_BYTES + msg.len()];
    let mut c_smlen: u64 = 0;
    unsafe { c_sign(c_sm.as_mut_ptr(), &mut c_smlen, msg.as_ptr(), mlen, c_sk.as_ptr()) };

    // Rust side - reset RNG to same state
    unsafe { sphincsplus::rng::randombytes_init(entropy.as_ptr(), std::ptr::null()) };
    let mut r_pk = [0u8; SPX_PK_BYTES];
    let mut r_sk = [0u8; SPX_SK_BYTES];
    unsafe { sphincsplus::sign::crypto_sign_keypair(r_pk.as_mut_ptr(), r_sk.as_mut_ptr()) };

    assert_eq!(c_pk, r_pk, "keypair pk mismatch");
    assert_eq!(c_sk, r_sk, "keypair sk mismatch");

    let mut r_sm = vec![0u8; SPX_BYTES + msg.len()];
    let mut r_smlen: u64 = 0;
    unsafe { sphincsplus::sign::crypto_sign(r_sm.as_mut_ptr(), &mut r_smlen, msg.as_ptr(), mlen, r_sk.as_ptr()) };

    assert_eq!(c_smlen, r_smlen, "smlen mismatch");
    assert_eq!(&c_sm[..c_smlen as usize], &r_sm[..r_smlen as usize], "signed message mismatch");

    // Verify C signature with Rust and vice versa
    let mut m_out = vec![0u8; SPX_BYTES + msg.len()];
    let mut mlen_out: u64 = 0;
    let ret = unsafe { sphincsplus::sign::crypto_sign_open(m_out.as_mut_ptr(), &mut mlen_out, c_sm.as_ptr(), c_smlen, c_pk.as_ptr()) };
    assert_eq!(ret, 0, "Rust failed to verify C signature");
    assert_eq!(mlen_out, mlen);
}
