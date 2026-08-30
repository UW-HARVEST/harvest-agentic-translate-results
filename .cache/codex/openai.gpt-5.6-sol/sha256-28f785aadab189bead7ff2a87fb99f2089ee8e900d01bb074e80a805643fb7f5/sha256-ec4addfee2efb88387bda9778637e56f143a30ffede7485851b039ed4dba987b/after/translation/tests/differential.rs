use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL, RTLD_NOW};
use std::ffi::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

const RTLD_DEEPBIND: c_int = 0x00008;
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const N: usize = if cfg!(any(feature = "128f", feature = "128s")) {
    16
} else if cfg!(any(feature = "192f", feature = "192s")) {
    24
} else {
    32
};
const FULL_HEIGHT: usize = if cfg!(feature = "128f") {
    66
} else if cfg!(feature = "128s") {
    63
} else if cfg!(feature = "192f") {
    66
} else if cfg!(feature = "192s") {
    63
} else if cfg!(feature = "256f") {
    68
} else {
    64
};
const D: usize = if cfg!(any(feature = "128f", feature = "192f")) {
    22
} else if cfg!(feature = "256f") {
    17
} else if cfg!(any(feature = "128s", feature = "192s")) {
    7
} else {
    8
};
const FORS_HEIGHT: usize = if cfg!(feature = "128f") {
    6
} else if cfg!(feature = "128s") {
    12
} else if cfg!(feature = "192f") {
    8
} else if cfg!(feature = "192s") {
    14
} else if cfg!(feature = "256f") {
    9
} else {
    14
};
const FORS_TREES: usize = if cfg!(feature = "128f") {
    33
} else if cfg!(feature = "128s") {
    14
} else if cfg!(feature = "192f") {
    33
} else if cfg!(feature = "192s") {
    17
} else if cfg!(feature = "256f") {
    35
} else {
    22
};
const TREE_HEIGHT: usize = FULL_HEIGHT / D;
const WOTS_LEN: usize = 2 * N + 3;
const WOTS_BYTES: usize = WOTS_LEN * N;
const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
const SIG_BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
const PK_BYTES: usize = 2 * N;
const SK_BYTES: usize = 4 * N;
const SEED_BYTES: usize = 3 * N;

fn backend() -> &'static str {
    if cfg!(feature = "haraka") {
        "haraka"
    } else if cfg!(feature = "sha2") {
        "sha2"
    } else if cfg!(feature = "shake") {
        "shake"
    } else {
        "blake"
    }
}

fn thash_mode() -> &'static str {
    if cfg!(feature = "robust") { "robust" } else { "simple" }
}

fn secpar() -> &'static str {
    if cfg!(feature = "128f") {
        "128f"
    } else if cfg!(feature = "128s") {
        "128s"
    } else if cfg!(feature = "192f") {
        "192f"
    } else if cfg!(feature = "192s") {
        "192s"
    } else if cfg!(feature = "256f") {
        "256f"
    } else {
        "256s"
    }
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Libraries {
    _crypto: Library,
    backend: Library,
    core: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = workspace();
        let build = root.join("c_src/build-matrix").join(format!(
            "{}-{}-{}",
            backend(),
            thash_mode(),
            secpar()
        ));
        let crypto_path = [
            PathBuf::from("/usr/lib64/libcrypto.so.3.5.7"),
            PathBuf::from(
                "/nix/store/1mf3lj0mldr8732yvzjc12fig2407b3d-openssl-3.6.3/lib/libcrypto.so",
            ),
        ]
        .into_iter()
        .find(|path| path.exists())
        .expect("libcrypto is required for the C deterministic RNG");
        let backend_path = build
            .join("lib")
            .join(backend())
            .join(format!("lib{}.so", backend()));
        let core_path = build.join("app/libsphincs_core_det.so");
        let rust_path = root.join(
            "translation/target/release/lib005_sphincs_PQCgenKAT_sign_blake_128f_simple.so",
        );

        for path in [&backend_path, &core_path, &rust_path] {
            assert!(path.exists(), "missing shared library {}", path.display());
        }

        let crypto =
            unsafe { Library::open(Some(&crypto_path), RTLD_NOW | RTLD_GLOBAL) }.unwrap();
        // The C core and backend have cyclic undefined symbols.
        let backend =
            unsafe { Library::open(Some(&backend_path), RTLD_LAZY | RTLD_GLOBAL) }.unwrap();
        let core = unsafe { Library::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL) }.unwrap();
        // Keep Rust's own calls bound to Rust even though C symbols are global.
        let rust =
            unsafe { Library::open(Some(&rust_path), RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND) }
                .unwrap();
        Self { _crypto: crypto, backend, core, rust }
    }

    unsafe fn c<T: Copy>(&self, name: &[u8]) -> T {
        if let Ok(symbol) = unsafe { self.core.get::<T>(name) } {
            *symbol
        } else {
            *unsafe { self.backend.get::<T>(name) }.unwrap()
        }
    }

    unsafe fn rust<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.rust.get::<T>(name) }.unwrap()
    }

    unsafe fn c_data<T: Copy>(&self, name: &[u8]) -> T {
        let symbol = if let Ok(symbol) = unsafe { self.core.get::<*const T>(name) } {
            symbol
        } else {
            unsafe { self.backend.get::<*const T>(name) }.unwrap()
        };
        unsafe { **symbol }
    }

    unsafe fn rust_data<T: Copy>(&self, name: &[u8]) -> T {
        let symbol = unsafe { self.rust.get::<*const T>(name) }.unwrap();
        unsafe { **symbol }
    }

    fn has_c(&self, name: &[u8]) -> bool {
        unsafe {
            self.core.get::<*const c_void>(name).is_ok()
                || self.backend.get::<*const c_void>(name).is_ok()
        }
    }

    fn has_rust(&self, name: &[u8]) -> bool {
        unsafe { self.rust.get::<*const c_void>(name).is_ok() }
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(tag: u64) -> Self {
        Self(0x6a09_e667_f3bc_c909 ^ tag)
    }

    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    fn fill(&mut self, output: &mut [u8]) {
        for chunk in output.chunks_mut(8) {
            let word = self.u64().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut output = vec![0; len];
        self.fill(&mut output);
        output
    }
}

fn ctx_size() -> usize {
    let mut size = 2 * N;
    if cfg!(feature = "sha2") {
        size += 40;
        if N >= 24 {
            size += 72;
        }
    }
    if cfg!(feature = "haraka") {
        size += 10 * 8 * 8 + 10 * 8 * 4;
    }
    size
}

fn new_ctx(rng: &mut Rng) -> Vec<u64> {
    let mut words = vec![0u64; ctx_size().div_ceil(8)];
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), ctx_size()) };
    rng.fill(&mut bytes[..2 * N]);
    words
}

fn ctx_bytes(ctx: &[u64]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(ctx.as_ptr().cast::<u8>(), ctx_size()) }
}

unsafe fn initialized_contexts(libs: &Libraries, rng: &mut Rng) -> (Vec<u64>, Vec<u64>) {
    type Initialize = unsafe extern "C" fn(*mut c_void);
    let mut c = new_ctx(rng);
    let mut r = c.clone();
    unsafe {
        libs.c::<Initialize>(b"SPX_initialize_hash_function\0")(c.as_mut_ptr().cast());
        libs.rust::<Initialize>(b"SPX_initialize_hash_function\0")(r.as_mut_ptr().cast());
    }
    assert_eq!(ctx_bytes(&c), ctx_bytes(&r));
    (c, r)
}

fn symbol_names() -> Vec<&'static [u8]> {
    let mut names: Vec<&[u8]> = vec![
        b"AES256_CTR_DRBG_Update\0",
        b"AES256_ECB\0",
        b"DRBG_ctx\0",
        b"SPX_bytes_to_ull\0",
        b"SPX_chain_lengths\0",
        b"SPX_compute_root\0",
        b"SPX_copy_keypair_addr\0",
        b"SPX_copy_subtree_addr\0",
        b"SPX_fors_gen_leafx1\0",
        b"SPX_fors_pk_from_sig\0",
        b"SPX_fors_sign\0",
        b"SPX_fors_treehashx1\0",
        b"SPX_gen_message_random\0",
        b"SPX_hash_message\0",
        b"SPX_initialize_hash_function\0",
        b"SPX_merkle_gen_root\0",
        b"SPX_merkle_sign\0",
        b"SPX_prf_addr\0",
        b"SPX_set_chain_addr\0",
        b"SPX_set_hash_addr\0",
        b"SPX_set_keypair_addr\0",
        b"SPX_set_layer_addr\0",
        b"SPX_set_tree_addr\0",
        b"SPX_set_tree_height\0",
        b"SPX_set_tree_index\0",
        b"SPX_set_type\0",
        b"SPX_thash\0",
        b"SPX_treehash\0",
        b"SPX_u32_to_bytes\0",
        b"SPX_ull_to_bytes\0",
        b"SPX_wots_gen_leafx1\0",
        b"SPX_wots_pk_from_sig\0",
        b"SPX_wots_treehashx1\0",
        b"crypto_sign\0",
        b"crypto_sign_bytes\0",
        b"crypto_sign_keypair\0",
        b"crypto_sign_open\0",
        b"crypto_sign_publickeybytes\0",
        b"crypto_sign_secretkeybytes\0",
        b"crypto_sign_seed_keypair\0",
        b"crypto_sign_seedbytes\0",
        b"crypto_sign_signature\0",
        b"crypto_sign_verify\0",
        b"randombytes\0",
        b"randombytes_init\0",
        b"seedexpander\0",
        b"seedexpander_init\0",
    ];
    if cfg!(feature = "blake") {
        names.extend([
            b"SPX_blake256_mgf1\0".as_slice(),
            b"SPX_blake512_mgf1\0".as_slice(),
            b"blake256\0".as_slice(),
            b"blake256_compress\0".as_slice(),
            b"blake256_final\0".as_slice(),
            b"blake256_init\0".as_slice(),
            b"blake256_update\0".as_slice(),
            b"blake512\0".as_slice(),
            b"blake512_compress\0".as_slice(),
            b"blake512_final\0".as_slice(),
            b"blake512_init\0".as_slice(),
            b"blake512_update\0".as_slice(),
            b"cst\0".as_slice(),
        ]);
    } else if cfg!(feature = "sha2") {
        names.extend([
            b"SPX_mgf1_256\0".as_slice(),
            b"SPX_mgf1_512\0".as_slice(),
            b"SPX_seed_state\0".as_slice(),
            b"sha256\0".as_slice(),
            b"sha256_inc_blocks\0".as_slice(),
            b"sha256_inc_finalize\0".as_slice(),
            b"sha256_inc_init\0".as_slice(),
            b"sha512\0".as_slice(),
            b"sha512_inc_blocks\0".as_slice(),
            b"sha512_inc_finalize\0".as_slice(),
            b"sha512_inc_init\0".as_slice(),
        ]);
    } else if cfg!(feature = "shake") {
        names.extend([
            b"shake256\0".as_slice(),
            b"shake256_absorb\0".as_slice(),
            b"shake256_inc_absorb\0".as_slice(),
            b"shake256_inc_finalize\0".as_slice(),
            b"shake256_inc_init\0".as_slice(),
            b"shake256_inc_squeeze\0".as_slice(),
            b"shake256_squeezeblocks\0".as_slice(),
        ]);
    } else {
        names.extend([
            b"SPX_haraka256\0".as_slice(),
            b"SPX_haraka512\0".as_slice(),
            b"SPX_haraka512_perm\0".as_slice(),
            b"SPX_haraka_S\0".as_slice(),
            b"SPX_haraka_S_inc_absorb\0".as_slice(),
            b"SPX_haraka_S_inc_finalize\0".as_slice(),
            b"SPX_haraka_S_inc_init\0".as_slice(),
            b"SPX_haraka_S_inc_squeeze\0".as_slice(),
            b"SPX_tweak_constants\0".as_slice(),
        ]);
    }
    names
}

#[test]
fn exported_symbols_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    for name in symbol_names() {
        let display = std::str::from_utf8(&name[..name.len() - 1]).unwrap();
        assert!(libs.has_c(name), "C is missing expected symbol {display}");
        assert!(libs.has_rust(name), "Rust is missing C symbol {display}");
    }
}

#[test]
fn sizes_utilities_and_addresses_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    unsafe {
        type SizeFn = unsafe extern "C" fn() -> u64;
        for (name, expected) in [
            (b"crypto_sign_secretkeybytes\0".as_slice(), SK_BYTES),
            (b"crypto_sign_publickeybytes\0".as_slice(), PK_BYTES),
            (b"crypto_sign_bytes\0".as_slice(), SIG_BYTES),
            (b"crypto_sign_seedbytes\0".as_slice(), SEED_BYTES),
        ] {
            assert_eq!(libs.c::<SizeFn>(name)(), expected as u64);
            assert_eq!(libs.rust::<SizeFn>(name)(), expected as u64);
        }

        type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
        type U32ToBytes = unsafe extern "C" fn(*mut u8, u32);
        type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
        let c_ull = libs.c::<UllToBytes>(b"SPX_ull_to_bytes\0");
        let r_ull = libs.rust::<UllToBytes>(b"SPX_ull_to_bytes\0");
        let c_u32 = libs.c::<U32ToBytes>(b"SPX_u32_to_bytes\0");
        let r_u32 = libs.rust::<U32ToBytes>(b"SPX_u32_to_bytes\0");
        let c_bytes = libs.c::<BytesToUll>(b"SPX_bytes_to_ull\0");
        let r_bytes = libs.rust::<BytesToUll>(b"SPX_bytes_to_ull\0");
        let mut rng = Rng::new(1);
        for _ in 0..32 {
            let value = rng.u64();
            for len in [0usize, 1, 4, 8, 12] {
                let mut c = vec![0xa5; len];
                let mut r = c.clone();
                c_ull(c.as_mut_ptr(), len as u32, value);
                r_ull(r.as_mut_ptr(), len as u32, value);
                assert_eq!(c, r);
            }
            let value = rng.u32();
            let mut c = [0u8; 4];
            let mut r = [0u8; 4];
            c_u32(c.as_mut_ptr(), value);
            r_u32(r.as_mut_ptr(), value);
            assert_eq!(c, r);
            for len in 0..=8 {
                let input = rng.bytes(len);
                assert_eq!(
                    c_bytes(input.as_ptr(), len as u32),
                    r_bytes(input.as_ptr(), len as u32)
                );
            }
        }

        type SetU32 = unsafe extern "C" fn(*mut u32, u32);
        type SetU64 = unsafe extern "C" fn(*mut u32, u64);
        type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
        let setters = [
            b"SPX_set_layer_addr\0".as_slice(),
            b"SPX_set_type\0".as_slice(),
            b"SPX_set_keypair_addr\0".as_slice(),
            b"SPX_set_chain_addr\0".as_slice(),
            b"SPX_set_hash_addr\0".as_slice(),
            b"SPX_set_tree_height\0".as_slice(),
            b"SPX_set_tree_index\0".as_slice(),
        ];
        for name in setters {
            let c_fn = libs.c::<SetU32>(name);
            let r_fn = libs.rust::<SetU32>(name);
            for value in [0, 1, 6, 7, 0xff, 0x100, u32::MAX, rng.u32()] {
                let mut c = [0u32; 8];
                rng.fill(std::slice::from_raw_parts_mut(c.as_mut_ptr().cast(), 32));
                let mut r = c;
                c_fn(c.as_mut_ptr(), value);
                r_fn(r.as_mut_ptr(), value);
                assert_eq!(c, r, "{} value={value}", String::from_utf8_lossy(name));
            }
        }
        let c_tree = libs.c::<SetU64>(b"SPX_set_tree_addr\0");
        let r_tree = libs.rust::<SetU64>(b"SPX_set_tree_addr\0");
        for value in [0, 1, u64::MAX, rng.u64()] {
            let mut c = [rng.u32(); 8];
            let mut r = c;
            c_tree(c.as_mut_ptr(), value);
            r_tree(r.as_mut_ptr(), value);
            assert_eq!(c, r);
        }
        for name in [
            b"SPX_copy_subtree_addr\0".as_slice(),
            b"SPX_copy_keypair_addr\0".as_slice(),
        ] {
            let c_fn = libs.c::<CopyAddr>(name);
            let r_fn = libs.rust::<CopyAddr>(name);
            for _ in 0..32 {
                let source: [u32; 8] = std::array::from_fn(|_| rng.u32());
                let mut c: [u32; 8] = std::array::from_fn(|_| rng.u32());
                let mut r = c;
                c_fn(c.as_mut_ptr(), source.as_ptr());
                r_fn(r.as_mut_ptr(), source.as_ptr());
                assert_eq!(c, r);
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AesXof {
    buffer: [u8; 16],
    buffer_pos: c_ulong,
    length_remaining: c_ulong,
    key: [u8; 32],
    ctr: [u8; 16],
}

impl Default for AesXof {
    fn default() -> Self {
        Self {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: c_int,
}

#[test]
fn rng_and_error_paths_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    unsafe {
        type Aes = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        type Update = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        type Init = unsafe extern "C" fn(*mut u8, *mut u8);
        type Random = unsafe extern "C" fn(*mut u8, u64) -> c_int;
        type ExpInit =
            unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, c_ulong) -> c_int;
        type Expand = unsafe extern "C" fn(*mut AesXof, *mut u8, c_ulong) -> c_int;

        let c_aes = libs.c::<Aes>(b"AES256_ECB\0");
        let r_aes = libs.rust::<Aes>(b"AES256_ECB\0");
        let c_update = libs.c::<Update>(b"AES256_CTR_DRBG_Update\0");
        let r_update = libs.rust::<Update>(b"AES256_CTR_DRBG_Update\0");
        let c_init = libs.c::<Init>(b"randombytes_init\0");
        let r_init = libs.rust::<Init>(b"randombytes_init\0");
        let c_random = libs.c::<Random>(b"randombytes\0");
        let r_random = libs.rust::<Random>(b"randombytes\0");
        let c_exp_init = libs.c::<ExpInit>(b"seedexpander_init\0");
        let r_exp_init = libs.rust::<ExpInit>(b"seedexpander_init\0");
        let c_expand = libs.c::<Expand>(b"seedexpander\0");
        let r_expand = libs.rust::<Expand>(b"seedexpander\0");
        let mut rng = Rng::new(2);

        for _ in 0..32 {
            let mut key = rng.bytes(32);
            let mut ctr = rng.bytes(16);
            let mut c = [0u8; 16];
            let mut r = [0u8; 16];
            c_aes(key.as_mut_ptr(), ctr.as_mut_ptr(), c.as_mut_ptr());
            r_aes(key.as_mut_ptr(), ctr.as_mut_ptr(), r.as_mut_ptr());
            assert_eq!(c, r);

            for provided in [false, true] {
                let mut c_key: [u8; 32] = rng.bytes(32).try_into().unwrap();
                let mut r_key = c_key;
                let mut c_v: [u8; 16] = rng.bytes(16).try_into().unwrap();
                let mut r_v = c_v;
                let mut data: [u8; 48] = rng.bytes(48).try_into().unwrap();
                let ptr = if provided { data.as_mut_ptr() } else { std::ptr::null_mut() };
                c_update(ptr, c_key.as_mut_ptr(), c_v.as_mut_ptr());
                r_update(ptr, r_key.as_mut_ptr(), r_v.as_mut_ptr());
                assert_eq!(c_key, r_key);
                assert_eq!(c_v, r_v);
            }
        }

        for personalized in [false, true] {
            for iteration in 0..16 {
                let mut entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
                let mut personal: [u8; 48] = rng.bytes(48).try_into().unwrap();
                let personal_ptr = if personalized {
                    personal.as_mut_ptr()
                } else {
                    std::ptr::null_mut()
                };
                c_init(entropy.as_mut_ptr(), personal_ptr);
                r_init(entropy.as_mut_ptr(), personal_ptr);
                assert_eq!(
                    libs.c_data::<Drbg>(b"DRBG_ctx\0"),
                    libs.rust_data::<Drbg>(b"DRBG_ctx\0")
                );
                for len in [0usize, 1, 15, 16, 17, 48, 257] {
                    let mut c = vec![0u8; len];
                    let mut r = vec![0u8; len];
                    assert_eq!(c_random(c.as_mut_ptr(), len as u64), 0);
                    assert_eq!(r_random(r.as_mut_ptr(), len as u64), 0);
                    assert_eq!(c, r, "personalized={personalized} iteration={iteration}");
                    assert_eq!(
                        libs.c_data::<Drbg>(b"DRBG_ctx\0"),
                        libs.rust_data::<Drbg>(b"DRBG_ctx\0")
                    );
                }
            }
        }

        let mut seed = rng.bytes(32);
        let mut diversifier = rng.bytes(8);
        for maxlen in [0, 1, 16, 17, 4096, u32::MAX as c_ulong] {
            let mut c = AesXof::default();
            let mut r = AesXof::default();
            assert_eq!(
                c_exp_init(
                    &mut c,
                    seed.as_mut_ptr(),
                    diversifier.as_mut_ptr(),
                    maxlen
                ),
                0
            );
            assert_eq!(
                r_exp_init(
                    &mut r,
                    seed.as_mut_ptr(),
                    diversifier.as_mut_ptr(),
                    maxlen
                ),
                0
            );
            assert_eq!(c, r);
        }
        if c_ulong::BITS > 32 {
            let invalid = 0x1_0000_0000u64 as c_ulong;
            let mut c = AesXof::default();
            let mut r = AesXof::default();
            assert_eq!(
                c_exp_init(
                    &mut c,
                    seed.as_mut_ptr(),
                    diversifier.as_mut_ptr(),
                    invalid
                ),
                -1
            );
            assert_eq!(
                r_exp_init(
                    &mut r,
                    seed.as_mut_ptr(),
                    diversifier.as_mut_ptr(),
                    invalid
                ),
                -1
            );
        }

        let mut c = AesXof::default();
        let mut r = AesXof::default();
        c_exp_init(&mut c, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 512);
        r_exp_init(&mut r, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 512);
        for len in [0usize, 1, 15, 16, 17, 31, 32, 63] {
            let mut c_out = vec![0; len];
            let mut r_out = vec![0; len];
            assert_eq!(c_expand(&mut c, c_out.as_mut_ptr(), len as c_ulong), 0);
            assert_eq!(r_expand(&mut r, r_out.as_mut_ptr(), len as c_ulong), 0);
            assert_eq!(c_out, r_out);
            assert_eq!(c, r);
        }
        assert_eq!(c_expand(&mut c, std::ptr::null_mut(), 1), -2);
        assert_eq!(r_expand(&mut r, std::ptr::null_mut(), 1), -2);
        let remaining = c.length_remaining;
        let mut c_out = vec![0; remaining as usize];
        let mut r_out = vec![0; remaining as usize];
        assert_eq!(c_expand(&mut c, c_out.as_mut_ptr(), remaining), -3);
        assert_eq!(r_expand(&mut r, r_out.as_mut_ptr(), remaining), -3);
        assert_eq!(c, r);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Blake256State {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 64],
}

impl Default for Blake256State {
    fn default() -> Self {
        Self {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Blake512State {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 128],
}

impl Default for Blake512State {
    fn default() -> Self {
        Self {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }
}

#[test]
fn backend_primitives_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(3);
    unsafe {
        if cfg!(feature = "blake") {
            type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
            type Init256 = unsafe extern "C" fn(*mut Blake256State);
            type Update256 = unsafe extern "C" fn(*mut Blake256State, *const u8, u64);
            type Final256 = unsafe extern "C" fn(*mut Blake256State, *mut u8);
            type Compress256 = unsafe extern "C" fn(*mut Blake256State, *const u8);
            type Init512 = unsafe extern "C" fn(*mut Blake512State);
            type Update512 = unsafe extern "C" fn(*mut Blake512State, *const u8, u64);
            type Final512 = unsafe extern "C" fn(*mut Blake512State, *mut u8);
            type Compress512 = unsafe extern "C" fn(*mut Blake512State, *const u8);
            type Mgf = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);

            for (name, outlen, lengths) in [
                (
                    b"blake256\0".as_slice(),
                    32usize,
                    vec![0, 1, 54, 55, 56, 63, 64, 65, 119, 120, 121, 128, 129],
                ),
                (
                    b"blake512\0".as_slice(),
                    64usize,
                    vec![0, 1, 110, 111, 112, 127, 128, 129, 239, 240, 241, 256],
                ),
            ] {
                let c_hash = libs.c::<Hash>(name);
                let r_hash = libs.rust::<Hash>(name);
                for len in lengths {
                    for _ in 0..8 {
                        let input = rng.bytes(len);
                        let mut c = vec![0; outlen];
                        let mut r = vec![0; outlen];
                        assert_eq!(c_hash(c.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                        assert_eq!(r_hash(r.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                        assert_eq!(c, r, "{} len={len}", String::from_utf8_lossy(name));
                    }
                }
            }

            let c_init = libs.c::<Init256>(b"blake256_init\0");
            let r_init = libs.rust::<Init256>(b"blake256_init\0");
            let c_update = libs.c::<Update256>(b"blake256_update\0");
            let r_update = libs.rust::<Update256>(b"blake256_update\0");
            let c_final = libs.c::<Final256>(b"blake256_final\0");
            let r_final = libs.rust::<Final256>(b"blake256_final\0");
            let c_compress = libs.c::<Compress256>(b"blake256_compress\0");
            let r_compress = libs.rust::<Compress256>(b"blake256_compress\0");
            for splits in [[0usize, 0, 0], [1, 54, 1], [31, 33, 65], [64, 64, 7]] {
                let mut c = Blake256State::default();
                let mut r = Blake256State::default();
                c_init(&mut c);
                r_init(&mut r);
                assert_eq!(c, r);
                for len in splits {
                    let input = rng.bytes(len);
                    c_update(&mut c, input.as_ptr(), (len * 8) as u64);
                    r_update(&mut r, input.as_ptr(), (len * 8) as u64);
                    assert_eq!(c, r);
                }
                let mut c_out = [0u8; 32];
                let mut r_out = [0u8; 32];
                c_final(&mut c, c_out.as_mut_ptr());
                r_final(&mut r, r_out.as_mut_ptr());
                assert_eq!(c_out, r_out);
                assert_eq!(c, r);
            }
            for _ in 0..16 {
                let block = rng.bytes(64);
                let mut c = Blake256State::default();
                let mut r = Blake256State::default();
                c_init(&mut c);
                r_init(&mut r);
                c_compress(&mut c, block.as_ptr());
                r_compress(&mut r, block.as_ptr());
                assert_eq!(c, r);
            }

            let c_init = libs.c::<Init512>(b"blake512_init\0");
            let r_init = libs.rust::<Init512>(b"blake512_init\0");
            let c_update = libs.c::<Update512>(b"blake512_update\0");
            let r_update = libs.rust::<Update512>(b"blake512_update\0");
            let c_final = libs.c::<Final512>(b"blake512_final\0");
            let r_final = libs.rust::<Final512>(b"blake512_final\0");
            let c_compress = libs.c::<Compress512>(b"blake512_compress\0");
            let r_compress = libs.rust::<Compress512>(b"blake512_compress\0");
            for splits in [[0usize, 0, 0], [1, 110, 1], [63, 65, 129], [128, 128, 7]] {
                let mut c = Blake512State::default();
                let mut r = Blake512State::default();
                c_init(&mut c);
                r_init(&mut r);
                assert_eq!(c, r);
                for len in splits {
                    let input = rng.bytes(len);
                    c_update(&mut c, input.as_ptr(), (len * 8) as u64);
                    r_update(&mut r, input.as_ptr(), (len * 8) as u64);
                    assert_eq!(c, r);
                }
                let mut c_out = [0u8; 64];
                let mut r_out = [0u8; 64];
                c_final(&mut c, c_out.as_mut_ptr());
                r_final(&mut r, r_out.as_mut_ptr());
                assert_eq!(c_out, r_out);
                assert_eq!(c, r);
            }
            for _ in 0..16 {
                let block = rng.bytes(128);
                let mut c = Blake512State::default();
                let mut r = Blake512State::default();
                c_init(&mut c);
                r_init(&mut r);
                c_compress(&mut c, block.as_ptr());
                r_compress(&mut r, block.as_ptr());
                assert_eq!(c, r);
            }

            for (name, digest_len) in [
                (b"SPX_blake256_mgf1\0".as_slice(), 32usize),
                (b"SPX_blake512_mgf1\0".as_slice(), 64usize),
            ] {
                let c_mgf = libs.c::<Mgf>(name);
                let r_mgf = libs.rust::<Mgf>(name);
                for outlen in [0, 1, digest_len - 1, digest_len, digest_len + 1, 3 * digest_len] {
                    let input = rng.bytes(47);
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_mgf(c.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    r_mgf(r.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    assert_eq!(c, r);
                }
            }
            assert_eq!(
                libs.c_data::<[u64; 16]>(b"cst\0"),
                libs.rust_data::<[u64; 16]>(b"cst\0")
            );
        } else if cfg!(feature = "sha2") {
            type Hash = unsafe extern "C" fn(*mut u8, *const u8, usize);
            type Init = unsafe extern "C" fn(*mut u8);
            type Blocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
            type Final = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
            type Mgf = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
            for (prefix, output, block, pad) in [
                ("sha256", 32usize, 64usize, 56usize),
                ("sha512", 64usize, 128usize, 112usize),
            ] {
                let hash_name = format!("{prefix}\0");
                let init_name = format!("{prefix}_inc_init\0");
                let blocks_name = format!("{prefix}_inc_blocks\0");
                let final_name = format!("{prefix}_inc_finalize\0");
                let c_hash = libs.c::<Hash>(hash_name.as_bytes());
                let r_hash = libs.rust::<Hash>(hash_name.as_bytes());
                let c_init = libs.c::<Init>(init_name.as_bytes());
                let r_init = libs.rust::<Init>(init_name.as_bytes());
                let c_blocks = libs.c::<Blocks>(blocks_name.as_bytes());
                let r_blocks = libs.rust::<Blocks>(blocks_name.as_bytes());
                let c_final = libs.c::<Final>(final_name.as_bytes());
                let r_final = libs.rust::<Final>(final_name.as_bytes());
                for len in [0, 1, pad - 1, pad, pad + 1, block - 1, block, block + 1, 2 * block] {
                    let input = rng.bytes(len);
                    let mut c = vec![0; output];
                    let mut r = vec![0; output];
                    c_hash(c.as_mut_ptr(), input.as_ptr(), input.len());
                    r_hash(r.as_mut_ptr(), input.as_ptr(), input.len());
                    assert_eq!(c, r, "{prefix} len={len}");

                    let full_blocks = len / block;
                    let tail = &input[full_blocks * block..];
                    let state_len = output + 8;
                    let mut c_state = vec![0u8; state_len];
                    let mut r_state = vec![0u8; state_len];
                    c_init(c_state.as_mut_ptr());
                    r_init(r_state.as_mut_ptr());
                    c_blocks(c_state.as_mut_ptr(), input.as_ptr(), full_blocks);
                    r_blocks(r_state.as_mut_ptr(), input.as_ptr(), full_blocks);
                    assert_eq!(c_state, r_state);
                    let mut c_inc = vec![0; output];
                    let mut r_inc = vec![0; output];
                    c_final(c_inc.as_mut_ptr(), c_state.as_mut_ptr(), tail.as_ptr(), tail.len());
                    r_final(r_inc.as_mut_ptr(), r_state.as_mut_ptr(), tail.as_ptr(), tail.len());
                    assert_eq!(c_inc, r_inc);
                    assert_eq!(c_inc, c);
                    assert_eq!(c_state, r_state);
                }
            }
            for (name, digest_len) in [
                (b"SPX_mgf1_256\0".as_slice(), 32usize),
                (b"SPX_mgf1_512\0".as_slice(), 64usize),
            ] {
                let c_mgf = libs.c::<Mgf>(name);
                let r_mgf = libs.rust::<Mgf>(name);
                for outlen in [0, 1, digest_len - 1, digest_len, digest_len + 1, 3 * digest_len] {
                    let input = rng.bytes(47);
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_mgf(c.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    r_mgf(r.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    assert_eq!(c, r);
                }
            }
        } else if cfg!(feature = "shake") {
            type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
            type IncInit = unsafe extern "C" fn(*mut u64);
            type IncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
            type IncFinalize = unsafe extern "C" fn(*mut u64);
            type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
            type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
            type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
            let c_shake = libs.c::<Shake>(b"shake256\0");
            let r_shake = libs.rust::<Shake>(b"shake256\0");
            for inlen in [0usize, 1, 135, 136, 137, 272, 319] {
                for outlen in [0usize, 1, 135, 136, 137, 272, 319] {
                    let input = rng.bytes(inlen);
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_shake(c.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    r_shake(r.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    assert_eq!(c, r, "shake in={inlen} out={outlen}");
                }
            }
            let c_absorb = libs.c::<Absorb>(b"shake256_absorb\0");
            let r_absorb = libs.rust::<Absorb>(b"shake256_absorb\0");
            let c_squeeze = libs.c::<Squeeze>(b"shake256_squeezeblocks\0");
            let r_squeeze = libs.rust::<Squeeze>(b"shake256_squeezeblocks\0");
            for inlen in [0usize, 1, 135, 136, 137, 319] {
                let input = rng.bytes(inlen);
                let mut c_state = [0u64; 25];
                let mut r_state = [0u64; 25];
                c_absorb(c_state.as_mut_ptr(), input.as_ptr(), inlen);
                r_absorb(r_state.as_mut_ptr(), input.as_ptr(), inlen);
                assert_eq!(c_state, r_state);
                let mut c = vec![0; 3 * 136];
                let mut r = vec![0; 3 * 136];
                c_squeeze(c.as_mut_ptr(), 3, c_state.as_mut_ptr());
                r_squeeze(r.as_mut_ptr(), 3, r_state.as_mut_ptr());
                assert_eq!(c, r);
                assert_eq!(c_state, r_state);
            }
            let c_init = libs.c::<IncInit>(b"shake256_inc_init\0");
            let r_init = libs.rust::<IncInit>(b"shake256_inc_init\0");
            let c_absorb = libs.c::<IncAbsorb>(b"shake256_inc_absorb\0");
            let r_absorb = libs.rust::<IncAbsorb>(b"shake256_inc_absorb\0");
            let c_final = libs.c::<IncFinalize>(b"shake256_inc_finalize\0");
            let r_final = libs.rust::<IncFinalize>(b"shake256_inc_finalize\0");
            let c_squeeze = libs.c::<IncSqueeze>(b"shake256_inc_squeeze\0");
            let r_squeeze = libs.rust::<IncSqueeze>(b"shake256_inc_squeeze\0");
            for splits in [[0usize, 0, 0], [1, 134, 1], [31, 105, 137], [136, 136, 7]] {
                let mut c_state = [0u64; 26];
                let mut r_state = [0u64; 26];
                c_init(c_state.as_mut_ptr());
                r_init(r_state.as_mut_ptr());
                for len in splits {
                    let input = rng.bytes(len);
                    c_absorb(c_state.as_mut_ptr(), input.as_ptr(), len);
                    r_absorb(r_state.as_mut_ptr(), input.as_ptr(), len);
                    assert_eq!(c_state, r_state);
                }
                c_final(c_state.as_mut_ptr());
                r_final(r_state.as_mut_ptr());
                assert_eq!(c_state, r_state);
                for outlen in [0usize, 1, 135, 136, 137, 319] {
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_squeeze(c.as_mut_ptr(), outlen, c_state.as_mut_ptr());
                    r_squeeze(r.as_mut_ptr(), outlen, r_state.as_mut_ptr());
                    assert_eq!(c, r);
                    assert_eq!(c_state, r_state);
                }
            }
        } else {
            type Init = unsafe extern "C" fn(*mut c_void);
            type Sponge =
                unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
            type IncInit = unsafe extern "C" fn(*mut u8);
            type IncAbsorb =
                unsafe extern "C" fn(*mut u8, *const u8, usize, *const c_void);
            type IncFinalize = unsafe extern "C" fn(*mut u8);
            type IncSqueeze =
                unsafe extern "C" fn(*mut u8, usize, *mut u8, *const c_void);
            type Perm = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
            let c_init = libs.c::<Init>(b"SPX_tweak_constants\0");
            let r_init = libs.rust::<Init>(b"SPX_tweak_constants\0");
            let mut c_ctx = new_ctx(&mut rng);
            let mut r_ctx = c_ctx.clone();
            c_init(c_ctx.as_mut_ptr().cast());
            r_init(r_ctx.as_mut_ptr().cast());
            assert_eq!(ctx_bytes(&c_ctx), ctx_bytes(&r_ctx));

            let c_sponge = libs.c::<Sponge>(b"SPX_haraka_S\0");
            let r_sponge = libs.rust::<Sponge>(b"SPX_haraka_S\0");
            for inlen in [0usize, 1, 31, 32, 33, 96] {
                for outlen in [0usize, 1, 31, 32, 33, 95] {
                    let input = rng.bytes(inlen);
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_sponge(
                        c.as_mut_ptr(),
                        outlen as u64,
                        input.as_ptr(),
                        inlen as u64,
                        c_ctx.as_ptr().cast(),
                    );
                    r_sponge(
                        r.as_mut_ptr(),
                        outlen as u64,
                        input.as_ptr(),
                        inlen as u64,
                        r_ctx.as_ptr().cast(),
                    );
                    assert_eq!(c, r);
                }
            }

            let c_inc_init = libs.c::<IncInit>(b"SPX_haraka_S_inc_init\0");
            let r_inc_init = libs.rust::<IncInit>(b"SPX_haraka_S_inc_init\0");
            let c_absorb = libs.c::<IncAbsorb>(b"SPX_haraka_S_inc_absorb\0");
            let r_absorb = libs.rust::<IncAbsorb>(b"SPX_haraka_S_inc_absorb\0");
            let c_final = libs.c::<IncFinalize>(b"SPX_haraka_S_inc_finalize\0");
            let r_final = libs.rust::<IncFinalize>(b"SPX_haraka_S_inc_finalize\0");
            let c_squeeze = libs.c::<IncSqueeze>(b"SPX_haraka_S_inc_squeeze\0");
            let r_squeeze = libs.rust::<IncSqueeze>(b"SPX_haraka_S_inc_squeeze\0");
            for splits in [[0usize, 0, 0], [1, 30, 1], [17, 15, 33], [32, 32, 7]] {
                let mut c_state = [0u8; 65];
                let mut r_state = [0u8; 65];
                c_inc_init(c_state.as_mut_ptr());
                r_inc_init(r_state.as_mut_ptr());
                for len in splits {
                    let input = rng.bytes(len);
                    c_absorb(
                        c_state.as_mut_ptr(),
                        input.as_ptr(),
                        len,
                        c_ctx.as_ptr().cast(),
                    );
                    r_absorb(
                        r_state.as_mut_ptr(),
                        input.as_ptr(),
                        len,
                        r_ctx.as_ptr().cast(),
                    );
                    assert_eq!(c_state, r_state);
                }
                c_final(c_state.as_mut_ptr());
                r_final(r_state.as_mut_ptr());
                assert_eq!(c_state, r_state);
                for outlen in [0usize, 1, 31, 32, 33, 95] {
                    let mut c = vec![0; outlen];
                    let mut r = vec![0; outlen];
                    c_squeeze(
                        c.as_mut_ptr(),
                        outlen,
                        c_state.as_mut_ptr(),
                        c_ctx.as_ptr().cast(),
                    );
                    r_squeeze(
                        r.as_mut_ptr(),
                        outlen,
                        r_state.as_mut_ptr(),
                        r_ctx.as_ptr().cast(),
                    );
                    assert_eq!(c, r);
                    assert_eq!(c_state, r_state);
                }
            }

            for (name, input_len, output_len) in [
                (b"SPX_haraka512_perm\0".as_slice(), 64usize, 64usize),
                (b"SPX_haraka512\0".as_slice(), 64usize, 32usize),
                (b"SPX_haraka256\0".as_slice(), 32usize, 32usize),
            ] {
                let c_fn = libs.c::<Perm>(name);
                let r_fn = libs.rust::<Perm>(name);
                for _ in 0..32 {
                    let input = rng.bytes(input_len);
                    let mut c = vec![0; output_len];
                    let mut r = vec![0; output_len];
                    c_fn(c.as_mut_ptr(), input.as_ptr(), c_ctx.as_ptr().cast());
                    r_fn(r.as_mut_ptr(), input.as_ptr(), r_ctx.as_ptr().cast());
                    assert_eq!(c, r, "{}", String::from_utf8_lossy(name));
                }
            }
        }
    }
}

#[test]
fn hash_thash_and_wots_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(4);
    unsafe {
        type Prf = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
        type GenRandom = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            *const u8,
            *const u8,
            u64,
            *const c_void,
        );
        type HashMessage = unsafe extern "C" fn(
            *mut u8,
            *mut u64,
            *mut u32,
            *const u8,
            *const u8,
            *const u8,
            u64,
            *const c_void,
        );
        type Thash =
            unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
        type ChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
        type WotsPk =
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *mut u32);

        let c_prf = libs.c::<Prf>(b"SPX_prf_addr\0");
        let r_prf = libs.rust::<Prf>(b"SPX_prf_addr\0");
        let c_random = libs.c::<GenRandom>(b"SPX_gen_message_random\0");
        let r_random = libs.rust::<GenRandom>(b"SPX_gen_message_random\0");
        let c_hash = libs.c::<HashMessage>(b"SPX_hash_message\0");
        let r_hash = libs.rust::<HashMessage>(b"SPX_hash_message\0");
        let c_thash = libs.c::<Thash>(b"SPX_thash\0");
        let r_thash = libs.rust::<Thash>(b"SPX_thash\0");
        let c_lengths = libs.c::<ChainLengths>(b"SPX_chain_lengths\0");
        let r_lengths = libs.rust::<ChainLengths>(b"SPX_chain_lengths\0");
        let c_wots = libs.c::<WotsPk>(b"SPX_wots_pk_from_sig\0");
        let r_wots = libs.rust::<WotsPk>(b"SPX_wots_pk_from_sig\0");

        for sample in 0..16 {
            let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
            let addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c = vec![0u8; N];
            let mut r = vec![0u8; N];
            c_prf(c.as_mut_ptr(), c_ctx.as_ptr().cast(), addr.as_ptr());
            r_prf(r.as_mut_ptr(), r_ctx.as_ptr().cast(), addr.as_ptr());
            assert_eq!(c, r, "prf sample={sample}");

            let sk_prf = rng.bytes(N);
            let optrand = rng.bytes(N);
            for len in [0usize, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 257] {
                let message = rng.bytes(len);
                let random_output_len = if cfg!(feature = "blake") {
                    if N >= 24 { 64 } else { 32 }
                } else {
                    N
                };
                let mut c = vec![0; random_output_len];
                let mut r = vec![0; random_output_len];
                c_random(
                    c.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    c_ctx.as_ptr().cast(),
                );
                r_random(
                    r.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    r_ctx.as_ptr().cast(),
                );
                assert_eq!(c, r, "gen_message_random len={len}");

                let pk = rng.bytes(PK_BYTES);
                let randomizer = rng.bytes(N);
                let mut c_digest = vec![0; FORS_MSG_BYTES];
                let mut r_digest = vec![0; FORS_MSG_BYTES];
                let mut c_tree = u64::MAX;
                let mut r_tree = u64::MAX;
                let mut c_leaf = u32::MAX;
                let mut r_leaf = u32::MAX;
                c_hash(
                    c_digest.as_mut_ptr(),
                    &mut c_tree,
                    &mut c_leaf,
                    randomizer.as_ptr(),
                    pk.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    c_ctx.as_ptr().cast(),
                );
                r_hash(
                    r_digest.as_mut_ptr(),
                    &mut r_tree,
                    &mut r_leaf,
                    randomizer.as_ptr(),
                    pk.as_ptr(),
                    message.as_ptr(),
                    len as u64,
                    r_ctx.as_ptr().cast(),
                );
                assert_eq!(c_digest, r_digest, "hash_message digest len={len}");
                assert_eq!(c_tree, r_tree, "hash_message tree len={len}");
                assert_eq!(c_leaf, r_leaf, "hash_message leaf len={len}");
            }

            for blocks in [1usize, 2, WOTS_LEN, FORS_TREES] {
                let input = rng.bytes(blocks * N);
                let mut c_addr = addr;
                let mut r_addr = addr;
                let mut c = vec![0; N];
                let mut r = vec![0; N];
                c_thash(
                    c.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    c_ctx.as_ptr().cast(),
                    c_addr.as_mut_ptr(),
                );
                r_thash(
                    r.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    r_ctx.as_ptr().cast(),
                    r_addr.as_mut_ptr(),
                );
                assert_eq!(c, r, "thash blocks={blocks}");
                assert_eq!(c_addr, r_addr, "thash address blocks={blocks}");
            }

            for message in [vec![0; N], vec![0xff; N], rng.bytes(N)] {
                let mut c = vec![0u32; WOTS_LEN];
                let mut r = vec![0u32; WOTS_LEN];
                c_lengths(c.as_mut_ptr(), message.as_ptr());
                r_lengths(r.as_mut_ptr(), message.as_ptr());
                assert_eq!(c, r);
            }

            let signature = rng.bytes(WOTS_BYTES);
            let message = rng.bytes(N);
            let mut c_addr = addr;
            let mut r_addr = addr;
            let mut c = vec![0; WOTS_BYTES];
            let mut r = vec![0; WOTS_BYTES];
            c_wots(
                c.as_mut_ptr(),
                signature.as_ptr(),
                message.as_ptr(),
                c_ctx.as_ptr().cast(),
                c_addr.as_mut_ptr(),
            );
            r_wots(
                r.as_mut_ptr(),
                signature.as_ptr(),
                message.as_ptr(),
                r_ctx.as_ptr().cast(),
                r_addr.as_mut_ptr(),
            );
            assert_eq!(c, r);
            assert_eq!(c_addr, r_addr);
        }
    }
}

unsafe extern "C" fn deterministic_leaf(
    leaf: *mut u8,
    _ctx: *const c_void,
    index: u32,
    tree_addr: *const u32,
) {
    let address = unsafe { std::slice::from_raw_parts(tree_addr, 8) };
    let output = unsafe { std::slice::from_raw_parts_mut(leaf, N) };
    for (i, byte) in output.iter_mut().enumerate() {
        *byte = (index as u8)
            .wrapping_mul(29)
            .wrapping_add(i as u8)
            .wrapping_add(address[i % 8] as u8);
    }
}

#[test]
fn roots_and_generic_treehash_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(5);
    unsafe {
        type ComputeRoot = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            u32,
            u32,
            *const u8,
            u32,
            *const c_void,
            *mut u32,
        );
        type LeafFn = unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32);
        type Treehash = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            u32,
            u32,
            u32,
            Option<LeafFn>,
            *mut u32,
        );
        let c_root = libs.c::<ComputeRoot>(b"SPX_compute_root\0");
        let r_root = libs.rust::<ComputeRoot>(b"SPX_compute_root\0");
        let c_treehash = libs.c::<Treehash>(b"SPX_treehash\0");
        let r_treehash = libs.rust::<Treehash>(b"SPX_treehash\0");
        for _ in 0..16 {
            let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
            for height in [1usize, TREE_HEIGHT, FORS_HEIGHT] {
                for parity in [0u32, 1] {
                    let leaf = rng.bytes(N);
                    let auth = rng.bytes(height * N);
                    let leaf_index =
                        (rng.u32() & ((1u32 << height.min(31)) - 1)) & !1 | parity;
                    let offset = rng.u32() & 0xff00;
                    let addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
                    let mut c_addr = addr;
                    let mut r_addr = addr;
                    let mut c = vec![0; N];
                    let mut r = vec![0; N];
                    c_root(
                        c.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_index,
                        offset,
                        auth.as_ptr(),
                        height as u32,
                        c_ctx.as_ptr().cast(),
                        c_addr.as_mut_ptr(),
                    );
                    r_root(
                        r.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_index,
                        offset,
                        auth.as_ptr(),
                        height as u32,
                        r_ctx.as_ptr().cast(),
                        r_addr.as_mut_ptr(),
                    );
                    assert_eq!(c, r, "compute_root height={height}");
                    assert_eq!(c_addr, r_addr);
                }
            }

            for height in [1usize, 2, TREE_HEIGHT] {
                for leaf_index in [0u32, (1u32 << height) / 2, (1u32 << height) - 1] {
                    let offset = if leaf_index == 0 { 0 } else { 17 };
                    let addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
                    let mut c_addr = addr;
                    let mut r_addr = addr;
                    let mut c_root_out = vec![0; N];
                    let mut r_root_out = vec![0; N];
                    let mut c_auth = vec![0; height * N];
                    let mut r_auth = vec![0; height * N];
                    c_treehash(
                        c_root_out.as_mut_ptr(),
                        c_auth.as_mut_ptr(),
                        c_ctx.as_ptr().cast(),
                        leaf_index,
                        offset,
                        height as u32,
                        Some(deterministic_leaf),
                        c_addr.as_mut_ptr(),
                    );
                    r_treehash(
                        r_root_out.as_mut_ptr(),
                        r_auth.as_mut_ptr(),
                        r_ctx.as_ptr().cast(),
                        leaf_index,
                        offset,
                        height as u32,
                        Some(deterministic_leaf),
                        r_addr.as_mut_ptr(),
                    );
                    assert_eq!(c_root_out, r_root_out, "treehash height={height}");
                    assert_eq!(c_auth, r_auth, "treehash auth height={height}");
                    assert_eq!(c_addr, r_addr);
                }
            }
        }
    }
}

#[repr(C)]
struct LeafInfo {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *mut u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

#[test]
fn x1_fors_and_merkle_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(6);
    unsafe {
        type WotsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfo);
        type ForsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut c_void);
        type WotsTree = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            u32,
            u32,
            u32,
            *mut u32,
            *mut LeafInfo,
        );
        type ForsTree = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            u32,
            u32,
            u32,
            *mut u32,
            *mut c_void,
        );
        type ForsSign =
            unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32);
        type ForsPk =
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32);
        type MerkleSign = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            *mut u32,
            *mut u32,
            u32,
        );
        type MerkleRoot = unsafe extern "C" fn(*mut u8, *const c_void);

        let c_wots_leaf = libs.c::<WotsLeaf>(b"SPX_wots_gen_leafx1\0");
        let r_wots_leaf = libs.rust::<WotsLeaf>(b"SPX_wots_gen_leafx1\0");
        let c_fors_leaf = libs.c::<ForsLeaf>(b"SPX_fors_gen_leafx1\0");
        let r_fors_leaf = libs.rust::<ForsLeaf>(b"SPX_fors_gen_leafx1\0");
        let c_wots_tree = libs.c::<WotsTree>(b"SPX_wots_treehashx1\0");
        let r_wots_tree = libs.rust::<WotsTree>(b"SPX_wots_treehashx1\0");
        let c_fors_tree = libs.c::<ForsTree>(b"SPX_fors_treehashx1\0");
        let r_fors_tree = libs.rust::<ForsTree>(b"SPX_fors_treehashx1\0");
        let c_fors_sign = libs.c::<ForsSign>(b"SPX_fors_sign\0");
        let r_fors_sign = libs.rust::<ForsSign>(b"SPX_fors_sign\0");
        let c_fors_pk = libs.c::<ForsPk>(b"SPX_fors_pk_from_sig\0");
        let r_fors_pk = libs.rust::<ForsPk>(b"SPX_fors_pk_from_sig\0");
        let c_merkle = libs.c::<MerkleSign>(b"SPX_merkle_sign\0");
        let r_merkle = libs.rust::<MerkleSign>(b"SPX_merkle_sign\0");
        let c_merkle_root = libs.c::<MerkleRoot>(b"SPX_merkle_gen_root\0");
        let r_merkle_root = libs.rust::<MerkleRoot>(b"SPX_merkle_gen_root\0");

        for sample in 0..8 {
            let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
            let base_leaf: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let base_pk: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let steps: Vec<u32> = (0..WOTS_LEN).map(|_| rng.u32() % 16).collect();
            for matching in [false, true] {
                let leaf_index = rng.u32() & ((1u32 << TREE_HEIGHT) - 1);
                let sign_leaf = if matching { leaf_index } else { leaf_index ^ 1 };
                let mut c_steps = steps.clone();
                let mut r_steps = steps.clone();
                let mut c_sig = vec![0u8; WOTS_BYTES];
                let mut r_sig = vec![0u8; WOTS_BYTES];
                let mut c_info = LeafInfo {
                    wots_sig: c_sig.as_mut_ptr(),
                    wots_sign_leaf: sign_leaf,
                    wots_steps: c_steps.as_mut_ptr(),
                    leaf_addr: base_leaf,
                    pk_addr: base_pk,
                };
                let mut r_info = LeafInfo {
                    wots_sig: r_sig.as_mut_ptr(),
                    wots_sign_leaf: sign_leaf,
                    wots_steps: r_steps.as_mut_ptr(),
                    leaf_addr: base_leaf,
                    pk_addr: base_pk,
                };
                let mut c_leaf = vec![0; N];
                let mut r_leaf = vec![0; N];
                c_wots_leaf(
                    c_leaf.as_mut_ptr(),
                    c_ctx.as_ptr().cast(),
                    leaf_index,
                    &mut c_info,
                );
                r_wots_leaf(
                    r_leaf.as_mut_ptr(),
                    r_ctx.as_ptr().cast(),
                    leaf_index,
                    &mut r_info,
                );
                assert_eq!(c_leaf, r_leaf, "wots leaf sample={sample}");
                assert_eq!(c_sig, r_sig);
                assert_eq!(c_info.leaf_addr, r_info.leaf_addr);
                assert_eq!(c_info.pk_addr, r_info.pk_addr);
            }

            for index in [0u32, 1, (1u32 << FORS_HEIGHT.min(31)) - 1] {
                let mut c_info = base_leaf;
                let mut r_info = base_leaf;
                let mut c_leaf = vec![0; N];
                let mut r_leaf = vec![0; N];
                c_fors_leaf(
                    c_leaf.as_mut_ptr(),
                    c_ctx.as_ptr().cast(),
                    index,
                    c_info.as_mut_ptr().cast(),
                );
                r_fors_leaf(
                    r_leaf.as_mut_ptr(),
                    r_ctx.as_ptr().cast(),
                    index,
                    r_info.as_mut_ptr().cast(),
                );
                assert_eq!(c_leaf, r_leaf);
                assert_eq!(c_info, r_info);
            }
        }

        let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
        for leaf_index in [0u32, (1u32 << TREE_HEIGHT) / 2, (1u32 << TREE_HEIGHT) - 1] {
            let mut c_steps: Vec<u32> = (0..WOTS_LEN).map(|_| rng.u32() % 16).collect();
            let mut r_steps = c_steps.clone();
            let mut c_sig = vec![0u8; WOTS_BYTES];
            let mut r_sig = vec![0u8; WOTS_BYTES];
            let leaf_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let pk_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c_info = LeafInfo {
                wots_sig: c_sig.as_mut_ptr(),
                wots_sign_leaf: leaf_index,
                wots_steps: c_steps.as_mut_ptr(),
                leaf_addr,
                pk_addr,
            };
            let mut r_info = LeafInfo {
                wots_sig: r_sig.as_mut_ptr(),
                wots_sign_leaf: leaf_index,
                wots_steps: r_steps.as_mut_ptr(),
                leaf_addr,
                pk_addr,
            };
            let tree_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c_addr = tree_addr;
            let mut r_addr = tree_addr;
            let mut c_root = vec![0; N];
            let mut r_root = vec![0; N];
            let mut c_auth = vec![0; TREE_HEIGHT * N];
            let mut r_auth = vec![0; TREE_HEIGHT * N];
            c_wots_tree(
                c_root.as_mut_ptr(),
                c_auth.as_mut_ptr(),
                c_ctx.as_ptr().cast(),
                leaf_index,
                0,
                TREE_HEIGHT as u32,
                c_addr.as_mut_ptr(),
                &mut c_info,
            );
            r_wots_tree(
                r_root.as_mut_ptr(),
                r_auth.as_mut_ptr(),
                r_ctx.as_ptr().cast(),
                leaf_index,
                0,
                TREE_HEIGHT as u32,
                r_addr.as_mut_ptr(),
                &mut r_info,
            );
            assert_eq!(c_root, r_root);
            assert_eq!(c_auth, r_auth);
            assert_eq!(c_sig, r_sig);
            assert_eq!(c_addr, r_addr);
            assert_eq!(c_info.leaf_addr, r_info.leaf_addr);
            assert_eq!(c_info.pk_addr, r_info.pk_addr);
        }

        let direct_fors_height = FORS_HEIGHT.min(4);
        for leaf_index in [
            0u32,
            (1u32 << direct_fors_height) / 2,
            (1u32 << direct_fors_height) - 1,
        ] {
            let offset = 1u32 << direct_fors_height;
            let info: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c_info = info;
            let mut r_info = info;
            let tree_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c_addr = tree_addr;
            let mut r_addr = tree_addr;
            let mut c_root = vec![0; N];
            let mut r_root = vec![0; N];
            let mut c_auth = vec![0; direct_fors_height * N];
            let mut r_auth = vec![0; direct_fors_height * N];
            c_fors_tree(
                c_root.as_mut_ptr(),
                c_auth.as_mut_ptr(),
                c_ctx.as_ptr().cast(),
                leaf_index,
                offset,
                direct_fors_height as u32,
                c_addr.as_mut_ptr(),
                c_info.as_mut_ptr().cast(),
            );
            r_fors_tree(
                r_root.as_mut_ptr(),
                r_auth.as_mut_ptr(),
                r_ctx.as_ptr().cast(),
                leaf_index,
                offset,
                direct_fors_height as u32,
                r_addr.as_mut_ptr(),
                r_info.as_mut_ptr().cast(),
            );
            assert_eq!(c_root, r_root);
            assert_eq!(c_auth, r_auth);
            assert_eq!(c_addr, r_addr);
            assert_eq!(c_info, r_info);
        }

        for _ in 0..2 {
            let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
            let message = rng.bytes(FORS_MSG_BYTES);
            let address: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut c_sig = vec![0; FORS_BYTES];
            let mut r_sig = vec![0; FORS_BYTES];
            let mut c_pk = vec![0; N];
            let mut r_pk = vec![0; N];
            c_fors_sign(
                c_sig.as_mut_ptr(),
                c_pk.as_mut_ptr(),
                message.as_ptr(),
                c_ctx.as_ptr().cast(),
                address.as_ptr(),
            );
            r_fors_sign(
                r_sig.as_mut_ptr(),
                r_pk.as_mut_ptr(),
                message.as_ptr(),
                r_ctx.as_ptr().cast(),
                address.as_ptr(),
            );
            assert_eq!(c_sig, r_sig);
            assert_eq!(c_pk, r_pk);
            let mut c_rebuilt = vec![0; N];
            let mut r_rebuilt = vec![0; N];
            c_fors_pk(
                c_rebuilt.as_mut_ptr(),
                c_sig.as_ptr(),
                message.as_ptr(),
                c_ctx.as_ptr().cast(),
                address.as_ptr(),
            );
            r_fors_pk(
                r_rebuilt.as_mut_ptr(),
                r_sig.as_ptr(),
                message.as_ptr(),
                r_ctx.as_ptr().cast(),
                address.as_ptr(),
            );
            assert_eq!(c_rebuilt, r_rebuilt);
            assert_eq!(c_rebuilt, c_pk);
        }

        let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
        for leaf_index in [0u32, (1u32 << TREE_HEIGHT) / 2, (1u32 << TREE_HEIGHT) - 1] {
            let mut c_wots_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut r_wots_addr = c_wots_addr;
            let mut c_tree_addr: [u32; 8] = std::array::from_fn(|_| rng.u32());
            let mut r_tree_addr = c_tree_addr;
            let mut c_sig = vec![0; WOTS_BYTES + TREE_HEIGHT * N];
            let mut r_sig = vec![0; WOTS_BYTES + TREE_HEIGHT * N];
            let root = rng.bytes(N);
            let mut c_root = root.clone();
            let mut r_root = root;
            c_merkle(
                c_sig.as_mut_ptr(),
                c_root.as_mut_ptr(),
                c_ctx.as_ptr().cast(),
                c_wots_addr.as_mut_ptr(),
                c_tree_addr.as_mut_ptr(),
                leaf_index,
            );
            r_merkle(
                r_sig.as_mut_ptr(),
                r_root.as_mut_ptr(),
                r_ctx.as_ptr().cast(),
                r_wots_addr.as_mut_ptr(),
                r_tree_addr.as_mut_ptr(),
                leaf_index,
            );
            assert_eq!(c_sig, r_sig);
            assert_eq!(c_root, r_root);
            assert_eq!(c_wots_addr, r_wots_addr);
            assert_eq!(c_tree_addr, r_tree_addr);
        }
        for _ in 0..2 {
            let (c_ctx, r_ctx) = initialized_contexts(&libs, &mut rng);
            let mut c_root = vec![0; N];
            let mut r_root = vec![0; N];
            c_merkle_root(c_root.as_mut_ptr(), c_ctx.as_ptr().cast());
            r_merkle_root(r_root.as_mut_ptr(), r_ctx.as_ptr().cast());
            assert_eq!(c_root, r_root);
        }
    }
}

unsafe fn reset_drbg(
    libs: &Libraries,
    entropy: &mut [u8; 48],
    personalization: Option<&mut [u8; 48]>,
) {
    type Init = unsafe extern "C" fn(*mut u8, *mut u8);
    let personal = personalization
        .map(|value| value.as_mut_ptr())
        .unwrap_or(std::ptr::null_mut());
    unsafe {
        libs.c::<Init>(b"randombytes_init\0")(entropy.as_mut_ptr(), personal);
        libs.rust::<Init>(b"randombytes_init\0")(entropy.as_mut_ptr(), personal);
    }
}

#[test]
fn signing_pipeline_and_rejections_match() {
    let _guard = test_lock();
    let libs = unsafe { Libraries::load() };
    let mut rng = Rng::new(7);
    unsafe {
        type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
        type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
        type Signature =
            unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
        type Verify =
            unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
        type Sign =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
        type Open =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
        let c_seed_keypair = libs.c::<SeedKeypair>(b"crypto_sign_seed_keypair\0");
        let r_seed_keypair = libs.rust::<SeedKeypair>(b"crypto_sign_seed_keypair\0");
        let c_keypair = libs.c::<Keypair>(b"crypto_sign_keypair\0");
        let r_keypair = libs.rust::<Keypair>(b"crypto_sign_keypair\0");
        let c_signature = libs.c::<Signature>(b"crypto_sign_signature\0");
        let r_signature = libs.rust::<Signature>(b"crypto_sign_signature\0");
        let c_verify = libs.c::<Verify>(b"crypto_sign_verify\0");
        let r_verify = libs.rust::<Verify>(b"crypto_sign_verify\0");
        let c_sign = libs.c::<Sign>(b"crypto_sign\0");
        let r_sign = libs.rust::<Sign>(b"crypto_sign\0");
        let c_open = libs.c::<Open>(b"crypto_sign_open\0");
        let r_open = libs.rust::<Open>(b"crypto_sign_open\0");

        let seed = rng.bytes(SEED_BYTES);
        let mut c_pk = vec![0; PK_BYTES];
        let mut r_pk = vec![0; PK_BYTES];
        let mut c_sk = vec![0; SK_BYTES];
        let mut r_sk = vec![0; SK_BYTES];
        assert_eq!(
            c_seed_keypair(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr()),
            0
        );
        assert_eq!(
            r_seed_keypair(r_pk.as_mut_ptr(), r_sk.as_mut_ptr(), seed.as_ptr()),
            0
        );
        assert_eq!(c_pk, r_pk);
        assert_eq!(c_sk, r_sk);

        let mut entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
        reset_drbg(&libs, &mut entropy, None);
        let mut c_random_pk = vec![0; PK_BYTES];
        let mut c_random_sk = vec![0; SK_BYTES];
        assert_eq!(c_keypair(c_random_pk.as_mut_ptr(), c_random_sk.as_mut_ptr()), 0);
        let mut entropy_copy = entropy;
        type Init = unsafe extern "C" fn(*mut u8, *mut u8);
        libs.rust::<Init>(b"randombytes_init\0")(
            entropy_copy.as_mut_ptr(),
            std::ptr::null_mut(),
        );
        let mut r_random_pk = vec![0; PK_BYTES];
        let mut r_random_sk = vec![0; SK_BYTES];
        assert_eq!(r_keypair(r_random_pk.as_mut_ptr(), r_random_sk.as_mut_ptr()), 0);
        assert_eq!(c_random_pk, r_random_pk);
        assert_eq!(c_random_sk, r_random_sk);

        let mut valid_signature = Vec::new();
        let mut valid_message = Vec::new();
        for (case, len) in [0usize, 1, 64, 257].into_iter().enumerate() {
            let message = rng.bytes(len);
            let mut entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
            let mut c_entropy = entropy;
            let mut r_entropy = entropy;
            libs.c::<Init>(b"randombytes_init\0")(
                c_entropy.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            libs.rust::<Init>(b"randombytes_init\0")(
                r_entropy.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            let mut c_sig = vec![0; SIG_BYTES];
            let mut r_sig = vec![0; SIG_BYTES];
            let mut c_len = usize::MAX;
            let mut r_len = usize::MAX;
            assert_eq!(
                c_signature(
                    c_sig.as_mut_ptr(),
                    &mut c_len,
                    message.as_ptr(),
                    message.len(),
                    c_sk.as_ptr(),
                ),
                0
            );
            assert_eq!(
                r_signature(
                    r_sig.as_mut_ptr(),
                    &mut r_len,
                    message.as_ptr(),
                    message.len(),
                    r_sk.as_ptr(),
                ),
                0
            );
            assert_eq!(c_len, SIG_BYTES);
            assert_eq!(r_len, SIG_BYTES);
            assert_eq!(c_sig, r_sig, "detached signature case={case}");
            assert_eq!(
                c_verify(
                    c_sig.as_ptr(),
                    c_sig.len(),
                    message.as_ptr(),
                    message.len(),
                    c_pk.as_ptr(),
                ),
                0
            );
            assert_eq!(
                r_verify(
                    r_sig.as_ptr(),
                    r_sig.len(),
                    message.as_ptr(),
                    message.len(),
                    r_pk.as_ptr(),
                ),
                0
            );
            assert_eq!(
                c_verify(
                    r_sig.as_ptr(),
                    r_sig.len(),
                    message.as_ptr(),
                    message.len(),
                    c_pk.as_ptr(),
                ),
                0
            );
            assert_eq!(
                r_verify(
                    c_sig.as_ptr(),
                    c_sig.len(),
                    message.as_ptr(),
                    message.len(),
                    r_pk.as_ptr(),
                ),
                0
            );
            if case == 3 {
                valid_signature = c_sig;
                valid_message = message;
            }
        }

        for bad_len in [0usize, SIG_BYTES - 1, SIG_BYTES + 1] {
            let mut signature = valid_signature.clone();
            signature.resize(bad_len, 0);
            assert_eq!(
                c_verify(
                    signature.as_ptr(),
                    bad_len,
                    valid_message.as_ptr(),
                    valid_message.len(),
                    c_pk.as_ptr(),
                ),
                -1
            );
            assert_eq!(
                r_verify(
                    signature.as_ptr(),
                    bad_len,
                    valid_message.as_ptr(),
                    valid_message.len(),
                    r_pk.as_ptr(),
                ),
                -1
            );
        }
        let mut tampered = valid_signature.clone();
        tampered[SIG_BYTES / 2] ^= 0x80;
        assert_eq!(
            c_verify(
                tampered.as_ptr(),
                tampered.len(),
                valid_message.as_ptr(),
                valid_message.len(),
                c_pk.as_ptr(),
            ),
            -1
        );
        assert_eq!(
            r_verify(
                tampered.as_ptr(),
                tampered.len(),
                valid_message.as_ptr(),
                valid_message.len(),
                r_pk.as_ptr(),
            ),
            -1
        );

        for overlapping in [false, true] {
            let message = rng.bytes(73);
            let mut c_buffer = vec![0u8; SIG_BYTES + message.len()];
            let mut r_buffer = vec![0u8; SIG_BYTES + message.len()];
            let c_message = if overlapping {
                c_buffer[..message.len()].copy_from_slice(&message);
                c_buffer.as_ptr()
            } else {
                message.as_ptr()
            };
            let r_message = if overlapping {
                r_buffer[..message.len()].copy_from_slice(&message);
                r_buffer.as_ptr()
            } else {
                message.as_ptr()
            };
            let mut c_entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
            let mut r_entropy = c_entropy;
            libs.c::<Init>(b"randombytes_init\0")(
                c_entropy.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            libs.rust::<Init>(b"randombytes_init\0")(
                r_entropy.as_mut_ptr(),
                std::ptr::null_mut(),
            );
            let mut c_len = u64::MAX;
            let mut r_len = u64::MAX;
            assert_eq!(
                c_sign(
                    c_buffer.as_mut_ptr(),
                    &mut c_len,
                    c_message,
                    message.len() as u64,
                    c_sk.as_ptr(),
                ),
                0
            );
            assert_eq!(
                r_sign(
                    r_buffer.as_mut_ptr(),
                    &mut r_len,
                    r_message,
                    message.len() as u64,
                    r_sk.as_ptr(),
                ),
                0
            );
            assert_eq!(c_len, r_len);
            assert_eq!(c_buffer, r_buffer, "attached overlap={overlapping}");

            for alias_output in [false, true] {
                let mut c_signed = c_buffer.clone();
                let mut r_signed = r_buffer.clone();
                let mut c_output = vec![0xa5; c_signed.len()];
                let mut r_output = vec![0xa5; r_signed.len()];
                let c_out = if alias_output {
                    c_signed.as_mut_ptr()
                } else {
                    c_output.as_mut_ptr()
                };
                let r_out = if alias_output {
                    r_signed.as_mut_ptr()
                } else {
                    r_output.as_mut_ptr()
                };
                let mut c_message_len = u64::MAX;
                let mut r_message_len = u64::MAX;
                let c_result = c_open(
                    c_out,
                    &mut c_message_len,
                    c_signed.as_ptr(),
                    c_signed.len() as u64,
                    c_pk.as_ptr(),
                );
                let r_result = r_open(
                    r_out,
                    &mut r_message_len,
                    r_signed.as_ptr(),
                    r_signed.len() as u64,
                    r_pk.as_ptr(),
                );
                assert_eq!(
                    c_result, r_result,
                    "open overlap={overlapping} alias_output={alias_output}"
                );
                if !overlapping {
                    assert_eq!(c_result, 0);
                }
                assert_eq!(c_message_len, r_message_len);
                if alias_output {
                    assert_eq!(
                        &c_signed[..c_message_len as usize],
                        &r_signed[..r_message_len as usize]
                    );
                } else {
                    assert_eq!(c_output, r_output);
                }
            }
        }

        for short_len in [0usize, 1, SIG_BYTES - 1] {
            let signed = rng.bytes(short_len);
            let mut c_output = vec![0xa5; short_len];
            let mut r_output = vec![0xa5; short_len];
            let mut c_len = u64::MAX;
            let mut r_len = u64::MAX;
            assert_eq!(
                c_open(
                    c_output.as_mut_ptr(),
                    &mut c_len,
                    signed.as_ptr(),
                    short_len as u64,
                    c_pk.as_ptr(),
                ),
                -1
            );
            assert_eq!(
                r_open(
                    r_output.as_mut_ptr(),
                    &mut r_len,
                    signed.as_ptr(),
                    short_len as u64,
                    r_pk.as_ptr(),
                ),
                -1
            );
            assert_eq!(c_len, 0);
            assert_eq!(r_len, 0);
            assert_eq!(c_output, vec![0; short_len]);
            assert_eq!(r_output, vec![0; short_len]);
        }

        let mut invalid_signed = vec![0u8; SIG_BYTES + 31];
        rng.fill(&mut invalid_signed);
        let mut c_output = vec![0xa5; invalid_signed.len()];
        let mut r_output = vec![0xa5; invalid_signed.len()];
        let mut c_len = u64::MAX;
        let mut r_len = u64::MAX;
        assert_eq!(
            c_open(
                c_output.as_mut_ptr(),
                &mut c_len,
                invalid_signed.as_ptr(),
                invalid_signed.len() as u64,
                c_pk.as_ptr(),
            ),
            -1
        );
        assert_eq!(
            r_open(
                r_output.as_mut_ptr(),
                &mut r_len,
                invalid_signed.as_ptr(),
                invalid_signed.len() as u64,
                r_pk.as_ptr(),
            ),
            -1
        );
        assert_eq!(c_len, 0);
        assert_eq!(r_len, 0);
        assert_eq!(c_output, vec![0; invalid_signed.len()]);
        assert_eq!(r_output, vec![0; invalid_signed.len()]);
    }
}
