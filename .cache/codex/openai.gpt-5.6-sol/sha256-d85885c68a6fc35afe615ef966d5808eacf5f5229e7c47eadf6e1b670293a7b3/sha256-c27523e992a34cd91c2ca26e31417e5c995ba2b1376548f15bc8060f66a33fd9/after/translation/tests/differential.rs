use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY};
use std::ffi::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};

const RTLD_DEEPBIND: c_int = 0x00008;

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
const D: usize = if cfg!(feature = "128f") {
    22
} else if cfg!(feature = "128s") {
    7
} else if cfg!(feature = "192f") {
    22
} else if cfg!(feature = "192s") {
    7
} else if cfg!(feature = "256f") {
    17
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
const WOTS_LEN1: usize = 2 * N;
const WOTS_LEN2: usize = 3;
const WOTS_LEN: usize = WOTS_LEN1 + WOTS_LEN2;
const WOTS_BYTES: usize = WOTS_LEN * N;
const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
const BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
const PK_BYTES: usize = 2 * N;
const SK_BYTES: usize = 4 * N;
const SEED_BYTES: usize = 3 * N;

struct Libs {
    _crypto: Library,
    _backend: Library,
    core: Library,
    rust: Library,
}

impl Libs {
    unsafe fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_backend = std::env::var_os("SPHINCS_C_BACKEND")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("../c_src/build/lib/blake/libblake.so"));
        let c_core = std::env::var_os("SPHINCS_C_CORE")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("../c_src/build/app/libsphincs_core_det.so"));
        let rust_so = std::env::var_os("SPHINCS_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release/libsphincs_plus_translation.so"));
        let crypto = std::env::var_os("SPHINCS_LIBCRYPTO")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/patching-agent/lib/libcrypto.so.1.1"));
        assert!(c_backend.exists(), "missing C backend {}", c_backend.display());
        assert!(c_core.exists(), "missing C core {}", c_core.display());
        assert!(rust_so.exists(), "missing Rust cdylib {}", rust_so.display());
        let crypto = unsafe { Library::open(Some(&crypto), RTLD_LAZY | RTLD_GLOBAL).unwrap() };
        let core = unsafe { Library::open(Some(&c_core), RTLD_LAZY | RTLD_GLOBAL).unwrap() };
        let backend = unsafe { Library::open(Some(&c_backend), RTLD_LAZY | RTLD_GLOBAL).unwrap() };
        let rust = unsafe { Library::open(Some(&rust_so), RTLD_LAZY | RTLD_DEEPBIND).unwrap() };
        Self { _crypto: crypto, _backend: backend, core, rust }
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe {
            self.core.get::<T>(name)
                .or_else(|_| self._backend.get::<T>(name))
                .unwrap_or_else(|e| panic!("C symbol {}: {e}", String::from_utf8_lossy(name)))
        };
        let r = unsafe {
            self.rust.get::<T>(name)
                .unwrap_or_else(|e| panic!("Rust symbol {}: {e}", String::from_utf8_lossy(name)))
        };
        (*c, *r)
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self { Self(0x5eed_cafe_f00d_baad) }
    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn fill(&mut self, out: &mut [u8]) {
        for chunk in out.chunks_mut(8) {
            let bytes = self.u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }
}

fn ctx_words() -> usize {
    let bytes = if cfg!(feature = "haraka") {
        2 * N + 10 * 8 * 8 + 10 * 8 * 4
    } else if cfg!(feature = "sha2") {
        2 * N + 40 + if N >= 24 { 72 } else { 0 }
    } else {
        2 * N
    };
    bytes.div_ceil(8)
}

fn make_ctx(rng: &mut Rng) -> Vec<u64> {
    let mut ctx = vec![0u64; ctx_words()];
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(ctx.as_mut_ptr().cast::<u8>(), ctx.len() * 8)
    };
    rng.fill(&mut bytes[..2 * N]);
    ctx
}

unsafe fn initialize_pair(libs: &Libs, base: &[u64]) -> (Vec<u64>, Vec<u64>) {
    type F = unsafe extern "C" fn(*mut c_void);
    let (c, r) = unsafe { libs.pair::<F>(b"SPX_initialize_hash_function\0") };
    let mut cc = base.to_vec();
    let mut rr = base.to_vec();
    unsafe {
        c(cc.as_mut_ptr().cast());
        r(rr.as_mut_ptr().cast());
    }
    assert_eq!(cc, rr);
    (cc, rr)
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LeafInfo {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *mut u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

unsafe extern "C" fn deterministic_leaf(
    out: *mut u8,
    _ctx: *const c_void,
    idx: u32,
    tree_addr: *const u32,
) {
    let addr = unsafe { std::slice::from_raw_parts(tree_addr, 8) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, N) };
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = (idx as u8).wrapping_add(i as u8).wrapping_add(addr[i % 8] as u8);
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

#[test]
fn exported_symbol_and_data_parity() {
    unsafe {
        let libs = Libs::new();
        let common = [
            "SPX_bytes_to_ull", "SPX_chain_lengths", "SPX_compute_root",
            "SPX_copy_keypair_addr", "SPX_copy_subtree_addr", "SPX_fors_gen_leafx1",
            "SPX_fors_pk_from_sig", "SPX_fors_sign", "SPX_fors_treehashx1",
            "SPX_gen_message_random", "SPX_hash_message", "SPX_initialize_hash_function",
            "SPX_merkle_gen_root", "SPX_merkle_sign", "SPX_prf_addr",
            "SPX_set_chain_addr", "SPX_set_hash_addr", "SPX_set_keypair_addr",
            "SPX_set_layer_addr", "SPX_set_tree_addr", "SPX_set_tree_height",
            "SPX_set_tree_index", "SPX_set_type", "SPX_thash", "SPX_treehash",
            "SPX_u32_to_bytes", "SPX_ull_to_bytes", "SPX_wots_gen_leafx1",
            "SPX_wots_pk_from_sig", "SPX_wots_treehashx1", "crypto_sign",
            "crypto_sign_bytes", "crypto_sign_keypair", "crypto_sign_open",
            "crypto_sign_publickeybytes", "crypto_sign_secretkeybytes",
            "crypto_sign_seed_keypair", "crypto_sign_seedbytes", "crypto_sign_signature",
            "crypto_sign_verify", "randombytes",
        ];
        for name in common {
            let mut nul = name.as_bytes().to_vec();
            nul.push(0);
            let _: (unsafe extern "C" fn(), unsafe extern "C" fn()) = libs.pair(&nul);
        }
        let rng_symbols = [
            "AES256_CTR_DRBG_Update", "AES256_ECB", "randombytes_init",
            "seedexpander", "seedexpander_init",
        ];
        for name in rng_symbols {
            let mut nul = name.as_bytes().to_vec();
            nul.push(0);
            let _: (unsafe extern "C" fn(), unsafe extern "C" fn()) = libs.pair(&nul);
        }
        if cfg!(feature = "blake") {
            for name in [
                "SPX_blake256_mgf1", "SPX_blake512_mgf1", "blake256",
                "blake256_compress", "blake256_final", "blake256_init",
                "blake256_update", "blake512", "blake512_compress", "blake512_final",
                "blake512_init", "blake512_update",
            ] {
                let mut nul = name.as_bytes().to_vec();
                nul.push(0);
                let _: (unsafe extern "C" fn(), unsafe extern "C" fn()) = libs.pair(&nul);
            }
            let c = libs._backend.get::<*const [u64; 16]>(b"cst\0").unwrap();
            let r = libs.rust.get::<*const [u64; 16]>(b"cst\0").unwrap();
            assert_eq!(**c, **r);
        }
        let c = libs.core.get::<*mut Drbg>(b"DRBG_ctx\0").unwrap();
        let r = libs.rust.get::<*mut Drbg>(b"DRBG_ctx\0").unwrap();
        assert_eq!(std::mem::size_of::<Drbg>(), std::mem::size_of::<Drbg>());
        assert!(!(*c).is_null());
        assert!(!(*r).is_null());
    }
}

#[test]
fn sizes_utils_and_addresses_match() {
    unsafe {
        let libs = Libs::new();
        type Size = unsafe extern "C" fn() -> u64;
        for (name, expected) in [
            (b"crypto_sign_secretkeybytes\0".as_slice(), SK_BYTES),
            (b"crypto_sign_publickeybytes\0".as_slice(), PK_BYTES),
            (b"crypto_sign_bytes\0".as_slice(), BYTES),
            (b"crypto_sign_seedbytes\0".as_slice(), SEED_BYTES),
        ] {
            let (c, r) = libs.pair::<Size>(name);
            assert_eq!(c(), expected as u64);
            assert_eq!(c(), r());
        }

        type UllTo = unsafe extern "C" fn(*mut u8, u32, u64);
        type From = unsafe extern "C" fn(*const u8, u32) -> u64;
        let (cu, ru) = libs.pair::<UllTo>(b"SPX_ull_to_bytes\0");
        let (cf, rf) = libs.pair::<From>(b"SPX_bytes_to_ull\0");
        let mut rng = Rng::new();
        for len in [0usize, 1, 2, 4, 8] {
            for _ in 0..32 {
                let value = rng.u64();
                let mut a = [0xa5; 8];
                let mut b = [0xa5; 8];
                cu(a.as_mut_ptr(), len as u32, value);
                ru(b.as_mut_ptr(), len as u32, value);
                assert_eq!(a, b);
                assert_eq!(cf(a.as_ptr(), len as u32), rf(b.as_ptr(), len as u32));
            }
        }

        type Set32 = unsafe extern "C" fn(*mut u32, u32);
        type Set64 = unsafe extern "C" fn(*mut u32, u64);
        for name in [
            b"SPX_set_layer_addr\0".as_slice(), b"SPX_set_type\0",
            b"SPX_set_keypair_addr\0", b"SPX_set_chain_addr\0",
            b"SPX_set_hash_addr\0", b"SPX_set_tree_height\0", b"SPX_set_tree_index\0",
        ] {
            let (c, r) = libs.pair::<Set32>(name);
            for value in [0, 1, 6, 7, u32::MAX] {
                let mut a = [0xfeed_beef; 8];
                let mut b = a;
                c(a.as_mut_ptr(), value);
                r(b.as_mut_ptr(), value);
                assert_eq!(a, b, "{}", String::from_utf8_lossy(name));
            }
        }
        let (c, r) = libs.pair::<Set64>(b"SPX_set_tree_addr\0");
        for value in [0, 1, 0x0102_0304_0506_0708, u64::MAX] {
            let mut a = [0xfeed_beef; 8];
            let mut b = a;
            c(a.as_mut_ptr(), value);
            r(b.as_mut_ptr(), value);
            assert_eq!(a, b);
        }
        type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
        for name in [b"SPX_copy_subtree_addr\0".as_slice(), b"SPX_copy_keypair_addr\0"] {
            let (c, r) = libs.pair::<CopyAddr>(name);
            for _ in 0..32 {
                let src: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
                let mut a: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
                let mut b = a;
                c(a.as_mut_ptr(), src.as_ptr());
                r(b.as_mut_ptr(), src.as_ptr());
                assert_eq!(a, b);
            }
        }
    }
}

#[test]
fn deterministic_rng_and_errors_match() {
    unsafe {
        let libs = Libs::new();
        type Init = unsafe extern "C" fn(*mut u8, *mut u8);
        type Random = unsafe extern "C" fn(*mut u8, u64) -> c_int;
        type Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        type Update = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        let (ci, ri) = libs.pair::<Init>(b"randombytes_init\0");
        let (cr, rr) = libs.pair::<Random>(b"randombytes\0");
        let (ce, re) = libs.pair::<Ecb>(b"AES256_ECB\0");
        let (cu, ru) = libs.pair::<Update>(b"AES256_CTR_DRBG_Update\0");
        let mut rng = Rng::new();
        for personalized in [false, true] {
            let mut entropy = [0u8; 48];
            let mut personal = [0u8; 48];
            rng.fill(&mut entropy);
            rng.fill(&mut personal);
            ci(entropy.as_mut_ptr(), personalized.then_some(personal.as_mut_ptr()).unwrap_or(std::ptr::null_mut()));
            ri(entropy.as_mut_ptr(), personalized.then_some(personal.as_mut_ptr()).unwrap_or(std::ptr::null_mut()));
            for len in [0usize, 1, 15, 16, 17, 48, 65] {
                let mut a = vec![0u8; len];
                let mut b = vec![0u8; len];
                assert_eq!(cr(a.as_mut_ptr(), len as u64), 0);
                assert_eq!(rr(b.as_mut_ptr(), len as u64), 0);
                assert_eq!(a, b);
            }
        }
        for _ in 0..32 {
            let mut key = [0u8; 32];
            let mut ctr = [0u8; 16];
            rng.fill(&mut key);
            rng.fill(&mut ctr);
            let mut a = [0u8; 16];
            let mut b = [0u8; 16];
            ce(key.as_mut_ptr(), ctr.as_mut_ptr(), a.as_mut_ptr());
            re(key.as_mut_ptr(), ctr.as_mut_ptr(), b.as_mut_ptr());
            assert_eq!(a, b);
            let mut ka = key;
            let mut kb = key;
            let mut va = ctr;
            let mut vb = ctr;
            let mut provided = [0u8; 48];
            rng.fill(&mut provided);
            cu(provided.as_mut_ptr(), ka.as_mut_ptr(), va.as_mut_ptr());
            ru(provided.as_mut_ptr(), kb.as_mut_ptr(), vb.as_mut_ptr());
            assert_eq!((ka, va), (kb, vb));
            cu(std::ptr::null_mut(), ka.as_mut_ptr(), va.as_mut_ptr());
            ru(std::ptr::null_mut(), kb.as_mut_ptr(), vb.as_mut_ptr());
            assert_eq!((ka, va), (kb, vb));
        }

        type XInit = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, c_ulong) -> c_int;
        type Xof = unsafe extern "C" fn(*mut AesXof, *mut u8, c_ulong) -> c_int;
        let (cxi, rxi) = libs.pair::<XInit>(b"seedexpander_init\0");
        let (cx, rx) = libs.pair::<Xof>(b"seedexpander\0");
        let mut seed = [0u8; 32];
        let mut div = [0u8; 8];
        rng.fill(&mut seed);
        rng.fill(&mut div);
        let zero = AesXof { buffer: [0; 16], buffer_pos: 0, length_remaining: 0, key: [0; 32], ctr: [0; 16] };
        let mut ca = zero;
        let mut ra = zero;
        assert_eq!(cxi(&mut ca, seed.as_mut_ptr(), div.as_mut_ptr(), 100), 0);
        assert_eq!(rxi(&mut ra, seed.as_mut_ptr(), div.as_mut_ptr(), 100), 0);
        assert_eq!(ca, ra);
        for len in [1usize, 15, 16, 17, 31] {
            let mut a = vec![0u8; len];
            let mut b = vec![0u8; len];
            assert_eq!(cx(&mut ca, a.as_mut_ptr(), len as c_ulong), 0);
            assert_eq!(rx(&mut ra, b.as_mut_ptr(), len as c_ulong), 0);
            assert_eq!((a, ca), (b, ra));
        }
        assert_eq!(cx(&mut ca, std::ptr::null_mut(), 1), -2);
        assert_eq!(rx(&mut ra, std::ptr::null_mut(), 1), -2);
        let remaining = ca.length_remaining;
        assert_eq!(cx(&mut ca, seed.as_mut_ptr(), remaining), -3);
        assert_eq!(rx(&mut ra, seed.as_mut_ptr(), remaining), -3);
        if c_ulong::BITS > 32 {
            assert_eq!(cxi(&mut ca, seed.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000), -1);
            assert_eq!(rxi(&mut ra, seed.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000), -1);
        }
    }
}

#[cfg(feature = "blake")]
#[test]
fn blake_primitives_match_randomized_boundaries() {
    unsafe {
        let libs = Libs::new();
        type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
        let (c256, r256) = libs.pair::<Hash>(b"blake256\0");
        let (c512, r512) = libs.pair::<Hash>(b"blake512\0");
        let mut rng = Rng::new();
        for len in [0usize, 1, 54, 55, 56, 63, 64, 65, 110, 111, 112, 127, 128, 129, 255, 256, 257] {
            for _ in 0..8 {
                let mut input = vec![0u8; len];
                rng.fill(&mut input);
                let mut a = [0u8; 64];
                let mut b = [0u8; 64];
                assert_eq!(c256(a.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                assert_eq!(r256(b.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                assert_eq!(&a[..32], &b[..32]);
                assert_eq!(c512(a.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                assert_eq!(r512(b.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                assert_eq!(a, b);
            }
        }

        type I256 = unsafe extern "C" fn(*mut Blake256State);
        type U256 = unsafe extern "C" fn(*mut Blake256State, *const u8, u64);
        type F256 = unsafe extern "C" fn(*mut Blake256State, *mut u8);
        let (ci, ri) = libs.pair::<I256>(b"blake256_init\0");
        let (cu, ru) = libs.pair::<U256>(b"blake256_update\0");
        let (cf, rf) = libs.pair::<F256>(b"blake256_final\0");
        for len in [0usize, 1, 55, 56, 64, 65, 129] {
            let mut input = vec![0u8; len];
            rng.fill(&mut input);
            let mut cs: Blake256State = std::mem::zeroed();
            let mut rs: Blake256State = std::mem::zeroed();
            ci(&mut cs);
            ri(&mut rs);
            let split = len / 2;
            cu(&mut cs, input.as_ptr(), (split * 8) as u64);
            ru(&mut rs, input.as_ptr(), (split * 8) as u64);
            cu(&mut cs, input.as_ptr().add(split), ((len - split) * 8) as u64);
            ru(&mut rs, input.as_ptr().add(split), ((len - split) * 8) as u64);
            let mut a = [0u8; 32];
            let mut b = [0u8; 32];
            cf(&mut cs, a.as_mut_ptr());
            rf(&mut rs, b.as_mut_ptr());
            assert_eq!(a, b);
        }
        type C256 = unsafe extern "C" fn(*mut Blake256State, *const u8);
        let (cc, rc) = libs.pair::<C256>(b"blake256_compress\0");
        let mut block = [0u8; 64]; rng.fill(&mut block);
        let (mut cs, mut rs): (Blake256State, Blake256State) =
            (std::mem::zeroed(), std::mem::zeroed());
        ci(&mut cs); ri(&mut rs);
        cc(&mut cs, block.as_ptr()); rc(&mut rs, block.as_ptr());
        assert_eq!(cs, rs);

        type I512 = unsafe extern "C" fn(*mut Blake512State);
        type U512 = unsafe extern "C" fn(*mut Blake512State, *const u8, u64);
        type F512 = unsafe extern "C" fn(*mut Blake512State, *mut u8);
        type C512 = unsafe extern "C" fn(*mut Blake512State, *const u8);
        let (ci5, ri5) = libs.pair::<I512>(b"blake512_init\0");
        let (cu5, ru5) = libs.pair::<U512>(b"blake512_update\0");
        let (cf5, rf5) = libs.pair::<F512>(b"blake512_final\0");
        let (cc5, rc5) = libs.pair::<C512>(b"blake512_compress\0");
        let mut input = vec![0u8; 145]; rng.fill(&mut input);
        let (mut cs5, mut rs5): (Blake512State, Blake512State) =
            (std::mem::zeroed(), std::mem::zeroed());
        ci5(&mut cs5); ri5(&mut rs5);
        cu5(&mut cs5, input.as_ptr(), 128 * 8);
        ru5(&mut rs5, input.as_ptr(), 128 * 8);
        let (mut a5, mut b5) = ([0u8; 64], [0u8; 64]);
        cf5(&mut cs5, a5.as_mut_ptr()); rf5(&mut rs5, b5.as_mut_ptr());
        assert_eq!(a5, b5);
        let mut block5 = [0u8; 128]; rng.fill(&mut block5);
        ci5(&mut cs5); ri5(&mut rs5);
        cc5(&mut cs5, block5.as_ptr()); rc5(&mut rs5, block5.as_ptr());
        assert_eq!(cs5, rs5);

        type Mgf = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
        for name in [b"SPX_blake256_mgf1\0".as_slice(), b"SPX_blake512_mgf1\0"] {
            let (c, r) = libs.pair::<Mgf>(name);
            for outlen in [0usize, 1, 31, 32, 33, 63, 64, 65, 129] {
                let mut input = [0u8; 47];
                rng.fill(&mut input);
                let mut a = vec![0u8; outlen];
                let mut b = vec![0u8; outlen];
                c(a.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                r(b.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), input.len() as c_ulong);
                assert_eq!(a, b);
            }
        }
    }
}

#[test]
fn backend_public_primitives_match() {
    unsafe {
        let libs = Libs::new();
        let mut rng = Rng::new();

        #[cfg(feature = "sha2")]
        {
            type Hash = unsafe extern "C" fn(*mut u8, *const u8, usize);
            for (name, outlen, lengths) in [
                (b"sha256\0".as_slice(), 32usize, vec![0, 1, 55, 56, 63, 64, 65]),
                (b"sha512\0".as_slice(), 64usize, vec![0, 1, 111, 112, 127, 128, 129]),
            ] {
                let (c, r) = libs.pair::<Hash>(name);
                for len in lengths {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let (mut a, mut b) = (vec![0u8; outlen], vec![0u8; outlen]);
                    c(a.as_mut_ptr(), input.as_ptr(), len);
                    r(b.as_mut_ptr(), input.as_ptr(), len);
                    assert_eq!(a, b);
                }
            }
            type I = unsafe extern "C" fn(*mut u8);
            type B = unsafe extern "C" fn(*mut u8, *const u8, usize);
            type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
            for (prefix, state_len, block, outlen) in [
                ("sha256", 40usize, 64usize, 32usize),
                ("sha512", 72usize, 128usize, 64usize),
            ] {
                let mut n = format!("{prefix}_inc_init").into_bytes(); n.push(0);
                let (ci, ri) = libs.pair::<I>(&n);
                let mut n = format!("{prefix}_inc_blocks").into_bytes(); n.push(0);
                let (cb, rb) = libs.pair::<B>(&n);
                let mut n = format!("{prefix}_inc_finalize").into_bytes(); n.push(0);
                let (cf, rf) = libs.pair::<F>(&n);
                let mut input = vec![0u8; block + 17];
                rng.fill(&mut input);
                let (mut cs, mut rs) = (vec![0u8; state_len], vec![0u8; state_len]);
                ci(cs.as_mut_ptr()); ri(rs.as_mut_ptr());
                cb(cs.as_mut_ptr(), input.as_ptr(), 1);
                rb(rs.as_mut_ptr(), input.as_ptr(), 1);
                let (mut a, mut b) = (vec![0u8; outlen], vec![0u8; outlen]);
                cf(a.as_mut_ptr(), cs.as_mut_ptr(), input.as_ptr().add(block), 17);
                rf(b.as_mut_ptr(), rs.as_mut_ptr(), input.as_ptr().add(block), 17);
                assert_eq!(a, b);
            }
            type Mgf = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
            for name in [b"SPX_mgf1_256\0".as_slice(), b"SPX_mgf1_512\0"] {
                let (c, r) = libs.pair::<Mgf>(name);
                let mut input = [0u8; 37]; rng.fill(&mut input);
                for len in [0usize, 1, 31, 32, 33, 64, 65, 129] {
                    let (mut a, mut b) = (vec![0u8; len], vec![0u8; len]);
                    c(a.as_mut_ptr(), len as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    r(b.as_mut_ptr(), len as c_ulong, input.as_ptr(), input.len() as c_ulong);
                    assert_eq!(a, b);
                }
            }
        }

        #[cfg(feature = "shake")]
        {
            type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
            let (c, r) = libs.pair::<Shake>(b"shake256\0");
            for ilen in [0usize, 1, 135, 136, 137, 273] {
                for olen in [0usize, 1, 31, 136, 137, 272] {
                    let mut input = vec![0u8; ilen]; rng.fill(&mut input);
                    let (mut a, mut b) = (vec![0u8; olen], vec![0u8; olen]);
                    c(a.as_mut_ptr(), olen, input.as_ptr(), ilen);
                    r(b.as_mut_ptr(), olen, input.as_ptr(), ilen);
                    assert_eq!(a, b);
                }
            }
            type Init = unsafe extern "C" fn(*mut u64);
            type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
            type Final = unsafe extern "C" fn(*mut u64);
            type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
            let (ci, ri) = libs.pair::<Init>(b"shake256_inc_init\0");
            let (ca, ra) = libs.pair::<Absorb>(b"shake256_inc_absorb\0");
            let (cf, rf) = libs.pair::<Final>(b"shake256_inc_finalize\0");
            let (csq, rsq) = libs.pair::<Squeeze>(b"shake256_inc_squeeze\0");
            let mut input = vec![0u8; 211]; rng.fill(&mut input);
            let (mut cst, mut rst) = ([0u64; 26], [0u64; 26]);
            ci(cst.as_mut_ptr()); ri(rst.as_mut_ptr());
            ca(cst.as_mut_ptr(), input.as_ptr(), 73); ra(rst.as_mut_ptr(), input.as_ptr(), 73);
            ca(cst.as_mut_ptr(), input.as_ptr().add(73), 138);
            ra(rst.as_mut_ptr(), input.as_ptr().add(73), 138);
            cf(cst.as_mut_ptr()); rf(rst.as_mut_ptr());
            let (mut a, mut b) = (vec![0u8; 277], vec![0u8; 277]);
            csq(a.as_mut_ptr(), a.len(), cst.as_mut_ptr());
            rsq(b.as_mut_ptr(), b.len(), rst.as_mut_ptr());
            assert_eq!(a, b);
            type AbsorbFull = unsafe extern "C" fn(*mut u64, *const u8, usize);
            type Blocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
            let (caf, raf) = libs.pair::<AbsorbFull>(b"shake256_absorb\0");
            let (cb, rb) = libs.pair::<Blocks>(b"shake256_squeezeblocks\0");
            let (mut cfull, mut rfull) = ([0u64; 25], [0u64; 25]);
            caf(cfull.as_mut_ptr(), input.as_ptr(), input.len());
            raf(rfull.as_mut_ptr(), input.as_ptr(), input.len());
            let (mut ao, mut bo) = (vec![0u8; 272], vec![0u8; 272]);
            cb(ao.as_mut_ptr(), 2, cfull.as_mut_ptr());
            rb(bo.as_mut_ptr(), 2, rfull.as_mut_ptr());
            assert_eq!(ao, bo);
        }

        #[cfg(feature = "haraka")]
        {
            let base = make_ctx(&mut rng);
            let (cc, rr) = initialize_pair(&libs, &base);
            type Fixed = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
            for (name, ilen, olen) in [
                (b"SPX_haraka256\0".as_slice(), 32usize, 32usize),
                (b"SPX_haraka512\0".as_slice(), 64usize, 32usize),
                (b"SPX_haraka512_perm\0".as_slice(), 64usize, 64usize),
            ] {
                let (c, r) = libs.pair::<Fixed>(name);
                let mut input = vec![0u8; ilen]; rng.fill(&mut input);
                let (mut a, mut b) = (vec![0u8; olen], vec![0u8; olen]);
                c(a.as_mut_ptr(), input.as_ptr(), cc.as_ptr().cast());
                r(b.as_mut_ptr(), input.as_ptr(), rr.as_ptr().cast());
                assert_eq!(a, b);
            }
            type Sponge = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
            let (c, r) = libs.pair::<Sponge>(b"SPX_haraka_S\0");
            let mut input = vec![0u8; 99]; rng.fill(&mut input);
            let (mut a, mut b) = (vec![0u8; 97], vec![0u8; 97]);
            c(a.as_mut_ptr(), a.len() as u64, input.as_ptr(), input.len() as u64, cc.as_ptr().cast());
            r(b.as_mut_ptr(), b.len() as u64, input.as_ptr(), input.len() as u64, rr.as_ptr().cast());
            assert_eq!(a, b);
            type HInit = unsafe extern "C" fn(*mut u8);
            type HAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const c_void);
            type HFinal = unsafe extern "C" fn(*mut u8);
            type HSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const c_void);
            let (ci, ri) = libs.pair::<HInit>(b"SPX_haraka_S_inc_init\0");
            let (ca, ra) = libs.pair::<HAbsorb>(b"SPX_haraka_S_inc_absorb\0");
            let (cf, rf) = libs.pair::<HFinal>(b"SPX_haraka_S_inc_finalize\0");
            let (cs, rs) = libs.pair::<HSqueeze>(b"SPX_haraka_S_inc_squeeze\0");
            let (mut cst, mut rst) = ([0u8; 65], [0u8; 65]);
            ci(cst.as_mut_ptr()); ri(rst.as_mut_ptr());
            ca(cst.as_mut_ptr(), input.as_ptr(), input.len(), cc.as_ptr().cast());
            ra(rst.as_mut_ptr(), input.as_ptr(), input.len(), rr.as_ptr().cast());
            cf(cst.as_mut_ptr()); rf(rst.as_mut_ptr());
            let (mut ao, mut bo) = (vec![0u8; 71], vec![0u8; 71]);
            cs(ao.as_mut_ptr(), ao.len(), cst.as_mut_ptr(), cc.as_ptr().cast());
            rs(bo.as_mut_ptr(), bo.len(), rst.as_mut_ptr(), rr.as_ptr().cast());
            assert_eq!(ao, bo);
        }
    }
}

#[test]
fn hash_thash_wots_and_root_match() {
    unsafe {
        let libs = Libs::new();
        let mut rng = Rng::new();
        let base = make_ctx(&mut rng);
        let (cc, rr) = initialize_pair(&libs, &base);
        type Prf = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
        let (cp, rp) = libs.pair::<Prf>(b"SPX_prf_addr\0");
        for typ in 0..=7u32 {
            for _ in 0..8 {
                let mut addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
                addr[4] = typ;
                let mut a = vec![0u8; N];
                let mut b = vec![0u8; N];
                cp(a.as_mut_ptr(), cc.as_ptr().cast(), addr.as_ptr());
                rp(b.as_mut_ptr(), rr.as_ptr().cast(), addr.as_ptr());
                assert_eq!(a, b);
            }
        }

        type Gen = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const c_void);
        let (cg, rg) = libs.pair::<Gen>(b"SPX_gen_message_random\0");
        type Hm = unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const c_void);
        let (ch, rh) = libs.pair::<Hm>(b"SPX_hash_message\0");
        for len in [0usize, 1, 31, 32, 33, 63, 64, 65, 129] {
            let mut message = vec![0u8; len];
            let mut sk_prf = vec![0u8; N];
            let mut optrand = vec![0u8; N];
            let mut pk = vec![0u8; PK_BYTES];
            rng.fill(&mut message);
            rng.fill(&mut sk_prf);
            rng.fill(&mut optrand);
            rng.fill(&mut pk);
            let mut ca = vec![0u8; 64];
            let mut ra = vec![0u8; 64];
            cg(ca.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), message.as_ptr(), len as u64, cc.as_ptr().cast());
            rg(ra.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), message.as_ptr(), len as u64, rr.as_ptr().cast());
            assert_eq!(&ca[..N], &ra[..N]);
            let mut cd = vec![0u8; FORS_MSG_BYTES];
            let mut rd = vec![0u8; FORS_MSG_BYTES];
            let (mut ct, mut rt, mut cl, mut rl) = (0u64, 0u64, 0u32, 0u32);
            ch(cd.as_mut_ptr(), &mut ct, &mut cl, ca.as_ptr(), pk.as_ptr(), message.as_ptr(), len as u64, cc.as_ptr().cast());
            rh(rd.as_mut_ptr(), &mut rt, &mut rl, ra.as_ptr(), pk.as_ptr(), message.as_ptr(), len as u64, rr.as_ptr().cast());
            assert_eq!((cd, ct, cl), (rd, rt, rl));
        }

        type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
        let (ct, rt) = libs.pair::<Thash>(b"SPX_thash\0");
        for blocks in [1usize, 2, WOTS_LEN, FORS_TREES] {
            for _ in 0..8 {
                let mut input = vec![0u8; blocks * N];
                rng.fill(&mut input);
                let addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
                let mut ac = addr;
                let mut ar = addr;
                let mut a = vec![0u8; N];
                let mut b = vec![0u8; N];
                ct(a.as_mut_ptr(), input.as_ptr(), blocks as u32, cc.as_ptr().cast(), ac.as_mut_ptr());
                rt(b.as_mut_ptr(), input.as_ptr(), blocks as u32, rr.as_ptr().cast(), ar.as_mut_ptr());
                assert_eq!((a, ac), (b, ar));
            }
        }

        type Chains = unsafe extern "C" fn(*mut u32, *const u8);
        let (cl, rl) = libs.pair::<Chains>(b"SPX_chain_lengths\0");
        for pattern in 0..34 {
            let mut msg = vec![0u8; N];
            if pattern == 1 { msg.fill(0xff); } else if pattern > 1 { rng.fill(&mut msg); }
            let mut a = vec![0u32; WOTS_LEN];
            let mut b = vec![0u32; WOTS_LEN];
            cl(a.as_mut_ptr(), msg.as_ptr());
            rl(b.as_mut_ptr(), msg.as_ptr());
            assert_eq!(a, b);
        }

        type Wots = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *mut u32);
        let (cw, rw) = libs.pair::<Wots>(b"SPX_wots_pk_from_sig\0");
        for _ in 0..4 {
            let mut sig = vec![0u8; WOTS_BYTES];
            let mut msg = vec![0u8; N];
            rng.fill(&mut sig);
            rng.fill(&mut msg);
            let addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
            let mut ac = addr;
            let mut ar = addr;
            let mut a = vec![0u8; WOTS_BYTES];
            let mut b = vec![0u8; WOTS_BYTES];
            cw(a.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), cc.as_ptr().cast(), ac.as_mut_ptr());
            rw(b.as_mut_ptr(), sig.as_ptr(), msg.as_ptr(), rr.as_ptr().cast(), ar.as_mut_ptr());
            assert_eq!((a, ac), (b, ar));
        }

        type Root = unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const c_void, *mut u32);
        let (croot, rroot) = libs.pair::<Root>(b"SPX_compute_root\0");
        for height in [1usize, TREE_HEIGHT] {
            for leaf_idx in [0u32, 1, (1u32 << height.min(31)).saturating_sub(1)] {
                let mut leaf = vec![0u8; N];
                let mut auth = vec![0u8; height * N + 1];
                rng.fill(&mut leaf);
                rng.fill(&mut auth);
                let addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
                let mut ac = addr;
                let mut ar = addr;
                let mut a = vec![0u8; N];
                let mut b = vec![0u8; N];
                croot(a.as_mut_ptr(), leaf.as_ptr(), leaf_idx, 3, auth.as_ptr(), height as u32, cc.as_ptr().cast(), ac.as_mut_ptr());
                rroot(b.as_mut_ptr(), leaf.as_ptr(), leaf_idx, 3, auth.as_ptr(), height as u32, rr.as_ptr().cast(), ar.as_mut_ptr());
                assert_eq!((a, ac), (b, ar));
            }
        }
    }
}

#[test]
fn direct_treehash_wots_leaf_merkle_and_keypair_match() {
    unsafe {
        let libs = Libs::new();
        let mut rng = Rng::new();
        let base = make_ctx(&mut rng);
        let (cc, rr) = initialize_pair(&libs, &base);

        type Treehash = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, u32, u32, u32,
            Option<unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32)>,
            *mut u32,
        );
        let (ct, rt) = libs.pair::<Treehash>(b"SPX_treehash\0");
        for height in [1usize, TREE_HEIGHT] {
            let addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
            let (mut ca, mut ra) = (addr, addr);
            let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
            let (mut cpath, mut rpath) =
                (vec![0u8; height * N], vec![0u8; height * N]);
            ct(croot.as_mut_ptr(), cpath.as_mut_ptr(), cc.as_ptr().cast(), 0, 5,
                height as u32, Some(deterministic_leaf), ca.as_mut_ptr());
            rt(rroot.as_mut_ptr(), rpath.as_mut_ptr(), rr.as_ptr().cast(), 0, 5,
                height as u32, Some(deterministic_leaf), ra.as_mut_ptr());
            assert_eq!((croot, cpath, ca), (rroot, rpath, ra));
        }

        type WotsLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfo);
        let (cwl, rwl) = libs.pair::<WotsLeaf>(b"SPX_wots_gen_leafx1\0");
        for signing in [false, true] {
            let leaf_idx = 2u32;
            let mut csteps: Vec<u32> = (0..WOTS_LEN).map(|_| (rng.u64() % 16) as u32).collect();
            let mut rsteps = csteps.clone();
            let (mut csig, mut rsig) = (vec![0u8; WOTS_BYTES], vec![0u8; WOTS_BYTES]);
            let leaf_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
            let pk_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
            let sign_leaf = if signing { leaf_idx } else { u32::MAX };
            let mut ci = LeafInfo {
                wots_sig: csig.as_mut_ptr(), wots_sign_leaf: sign_leaf,
                wots_steps: csteps.as_mut_ptr(), leaf_addr, pk_addr,
            };
            let mut ri = LeafInfo {
                wots_sig: rsig.as_mut_ptr(), wots_sign_leaf: sign_leaf,
                wots_steps: rsteps.as_mut_ptr(), leaf_addr, pk_addr,
            };
            let (mut ca, mut ra) = (vec![0u8; N], vec![0u8; N]);
            cwl(ca.as_mut_ptr(), cc.as_ptr().cast(), leaf_idx, &mut ci);
            rwl(ra.as_mut_ptr(), rr.as_ptr().cast(), leaf_idx, &mut ri);
            assert_eq!((ca, csig, ci.leaf_addr, ci.pk_addr),
                       (ra, rsig, ri.leaf_addr, ri.pk_addr));
        }

        type WotsTree = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, u32, u32, u32, *mut u32, *mut LeafInfo,
        );
        let (cwt, rwt) = libs.pair::<WotsTree>(b"SPX_wots_treehashx1\0");
        let mut csteps = vec![0u32; WOTS_LEN];
        let mut rsteps = csteps.clone();
        let (mut csig, mut rsig) = (vec![0u8; WOTS_BYTES], vec![0u8; WOTS_BYTES]);
        let leaf_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let pk_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let mut ci = LeafInfo {
            wots_sig: csig.as_mut_ptr(), wots_sign_leaf: 1,
            wots_steps: csteps.as_mut_ptr(), leaf_addr, pk_addr,
        };
        let mut ri = LeafInfo {
            wots_sig: rsig.as_mut_ptr(), wots_sign_leaf: 1,
            wots_steps: rsteps.as_mut_ptr(), leaf_addr, pk_addr,
        };
        let tree_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let (mut cta, mut rta) = (tree_addr, tree_addr);
        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        let (mut cpath, mut rpath) =
            (vec![0u8; TREE_HEIGHT * N], vec![0u8; TREE_HEIGHT * N]);
        cwt(croot.as_mut_ptr(), cpath.as_mut_ptr(), cc.as_ptr().cast(), 1, 0,
            TREE_HEIGHT as u32, cta.as_mut_ptr(), &mut ci);
        rwt(rroot.as_mut_ptr(), rpath.as_mut_ptr(), rr.as_ptr().cast(), 1, 0,
            TREE_HEIGHT as u32, rta.as_mut_ptr(), &mut ri);
        assert_eq!((croot, cpath, csig, cta, ci.leaf_addr, ci.pk_addr),
                   (rroot, rpath, rsig, rta, ri.leaf_addr, ri.pk_addr));

        type Merkle = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, *mut u32, *mut u32, u32,
        );
        let (cm, rm) = libs.pair::<Merkle>(b"SPX_merkle_sign\0");
        let (mut csig, mut rsig) =
            (vec![0u8; WOTS_BYTES + TREE_HEIGHT * N], vec![0u8; WOTS_BYTES + TREE_HEIGHT * N]);
        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        let wa: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let ta: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let (mut cwa, mut rwa, mut cta, mut rta) = (wa, wa, ta, ta);
        cm(csig.as_mut_ptr(), croot.as_mut_ptr(), cc.as_ptr().cast(),
            cwa.as_mut_ptr(), cta.as_mut_ptr(), 1);
        rm(rsig.as_mut_ptr(), rroot.as_mut_ptr(), rr.as_ptr().cast(),
            rwa.as_mut_ptr(), rta.as_mut_ptr(), 1);
        assert_eq!((csig, croot, cwa, cta), (rsig, rroot, rwa, rta));

        type Init = unsafe extern "C" fn(*mut u8, *mut u8);
        type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
        let (ci, ri) = libs.pair::<Init>(b"randombytes_init\0");
        let (ck, rk) = libs.pair::<Keypair>(b"crypto_sign_keypair\0");
        let mut entropy = [0u8; 48];
        rng.fill(&mut entropy);
        ci(entropy.as_mut_ptr(), std::ptr::null_mut());
        ri(entropy.as_mut_ptr(), std::ptr::null_mut());
        let (mut cpk, mut rpk) = (vec![0u8; PK_BYTES], vec![0u8; PK_BYTES]);
        let (mut csk, mut rsk) = (vec![0u8; SK_BYTES], vec![0u8; SK_BYTES]);
        assert_eq!(ck(cpk.as_mut_ptr(), csk.as_mut_ptr()), 0);
        assert_eq!(rk(rpk.as_mut_ptr(), rsk.as_mut_ptr()), 0);
        assert_eq!((cpk, csk), (rpk, rsk));
    }
}

#[test]
fn fors_merkle_and_signing_match() {
    unsafe {
        let libs = Libs::new();
        let mut rng = Rng::new();
        let base = make_ctx(&mut rng);
        let (cc, rr) = initialize_pair(&libs, &base);
        type ForsLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut c_void);
        let (cfl, rfl) = libs.pair::<ForsLeaf>(b"SPX_fors_gen_leafx1\0");
        for idx in [0u32, 1, 17, 63] {
            let info: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
            let mut ci = info;
            let mut ri = info;
            let mut a = vec![0u8; N];
            let mut b = vec![0u8; N];
            cfl(a.as_mut_ptr(), cc.as_ptr().cast(), idx, ci.as_mut_ptr().cast());
            rfl(b.as_mut_ptr(), rr.as_ptr().cast(), idx, ri.as_mut_ptr().cast());
            assert_eq!((a, ci), (b, ri), "FORS leaf idx={idx}");
        }
        type ForsTree = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, u32, u32, u32, *mut u32, *mut c_void,
        );
        let (cft, rft) = libs.pair::<ForsTree>(b"SPX_fors_treehashx1\0");
        let info: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let tree_addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let (mut ci, mut ri) = (info, info);
        let (mut ca, mut ra) = (tree_addr, tree_addr);
        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        let (mut cauth, mut rauth) =
            (vec![0u8; FORS_HEIGHT * N], vec![0u8; FORS_HEIGHT * N]);
        cft(croot.as_mut_ptr(), cauth.as_mut_ptr(), cc.as_ptr().cast(), 3, 0,
            FORS_HEIGHT as u32, ca.as_mut_ptr(), ci.as_mut_ptr().cast());
        rft(rroot.as_mut_ptr(), rauth.as_mut_ptr(), rr.as_ptr().cast(), 3, 0,
            FORS_HEIGHT as u32, ra.as_mut_ptr(), ri.as_mut_ptr().cast());
        assert_eq!((croot, cauth, ca, ci), (rroot, rauth, ra, ri), "FORS treehash");

        type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32);
        type ForsPk = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32);
        let (cfs, rfs) = libs.pair::<ForsSign>(b"SPX_fors_sign\0");
        let (cfp, rfp) = libs.pair::<ForsPk>(b"SPX_fors_pk_from_sig\0");
        let mut msg = vec![0u8; FORS_MSG_BYTES];
        rng.fill(&mut msg);
        let addr: [u32; 8] = std::array::from_fn(|_| rng.u64() as u32);
        let mut cs = vec![0u8; FORS_BYTES];
        let mut rs = vec![0u8; FORS_BYTES];
        let mut cpk = vec![0u8; N];
        let mut rpk = vec![0u8; N];
        cfs(cs.as_mut_ptr(), cpk.as_mut_ptr(), msg.as_ptr(), cc.as_ptr().cast(), addr.as_ptr());
        rfs(rs.as_mut_ptr(), rpk.as_mut_ptr(), msg.as_ptr(), rr.as_ptr().cast(), addr.as_ptr());
        assert_eq!((cs.clone(), cpk.clone()), (rs.clone(), rpk.clone()));
        let mut cpk2 = vec![0u8; N];
        let mut rpk2 = vec![0u8; N];
        cfp(cpk2.as_mut_ptr(), cs.as_ptr(), msg.as_ptr(), cc.as_ptr().cast(), addr.as_ptr());
        rfp(rpk2.as_mut_ptr(), rs.as_ptr(), msg.as_ptr(), rr.as_ptr().cast(), addr.as_ptr());
        assert_eq!((cpk2, cpk), (rpk2, rpk));

        type MRoot = unsafe extern "C" fn(*mut u8, *const c_void);
        let (cmr, rmr) = libs.pair::<MRoot>(b"SPX_merkle_gen_root\0");
        let mut a = vec![0u8; N];
        let mut b = vec![0u8; N];
        cmr(a.as_mut_ptr(), cc.as_ptr().cast());
        rmr(b.as_mut_ptr(), rr.as_ptr().cast());
        assert_eq!(a, b);

        type SeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
        type Sig = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
        type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
        type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
        type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
        let (ckp, rkp) = libs.pair::<SeedKp>(b"crypto_sign_seed_keypair\0");
        let (csig, rsig) = libs.pair::<Sig>(b"crypto_sign_signature\0");
        let (cv, rv) = libs.pair::<Verify>(b"crypto_sign_verify\0");
        let (csgn, rsgn) = libs.pair::<Sign>(b"crypto_sign\0");
        let (cop, rop) = libs.pair::<Open>(b"crypto_sign_open\0");
        for len in [0usize, 1, 33] {
            let mut seed = vec![0u8; SEED_BYTES];
            let mut message = vec![0u8; len];
            rng.fill(&mut seed);
            rng.fill(&mut message);
            let (mut cpk, mut rpk) = (vec![0u8; PK_BYTES], vec![0u8; PK_BYTES]);
            let (mut csk, mut rsk) = (vec![0u8; SK_BYTES], vec![0u8; SK_BYTES]);
            assert_eq!(ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()), 0);
            assert_eq!(rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()), 0);
            assert_eq!((cpk.clone(), csk.clone()), (rpk.clone(), rsk.clone()));

            let entropy = std::array::from_fn::<_, 48, _>(|i| i as u8);
            type Init = unsafe extern "C" fn(*mut u8, *mut u8);
            let (ci, ri) = libs.pair::<Init>(b"randombytes_init\0");
            ci(entropy.as_ptr() as *mut u8, std::ptr::null_mut());
            ri(entropy.as_ptr() as *mut u8, std::ptr::null_mut());
            let (mut cdet, mut rdet) = (vec![0u8; BYTES], vec![0u8; BYTES]);
            let (mut clen, mut rlen) = (0usize, 0usize);
            assert_eq!(csig(cdet.as_mut_ptr(), &mut clen, message.as_ptr(), len, csk.as_ptr()), 0);
            assert_eq!(rsig(rdet.as_mut_ptr(), &mut rlen, message.as_ptr(), len, rsk.as_ptr()), 0);
            assert_eq!((cdet.clone(), clen), (rdet.clone(), rlen));
            assert_eq!(cv(cdet.as_ptr(), clen, message.as_ptr(), len, cpk.as_ptr()), 0);
            assert_eq!(rv(rdet.as_ptr(), rlen, message.as_ptr(), len, rpk.as_ptr()), 0);
            assert_eq!(cv(cdet.as_ptr(), clen - 1, message.as_ptr(), len, cpk.as_ptr()), -1);
            assert_eq!(rv(rdet.as_ptr(), rlen - 1, message.as_ptr(), len, rpk.as_ptr()), -1);
            assert_eq!(cv(cdet.as_ptr(), 0, message.as_ptr(), len, cpk.as_ptr()), -1);
            assert_eq!(rv(rdet.as_ptr(), 0, message.as_ptr(), len, rpk.as_ptr()), -1);
            let mut extended = cdet.clone();
            extended.push(0);
            assert_eq!(cv(extended.as_ptr(), BYTES + 1, message.as_ptr(), len, cpk.as_ptr()), -1);
            assert_eq!(rv(extended.as_ptr(), BYTES + 1, message.as_ptr(), len, rpk.as_ptr()), -1);
            let mut bad = cdet.clone();
            bad[BYTES / 2] ^= 1;
            assert_eq!(cv(bad.as_ptr(), BYTES, message.as_ptr(), len, cpk.as_ptr()), -1);
            assert_eq!(rv(bad.as_ptr(), BYTES, message.as_ptr(), len, rpk.as_ptr()), -1);

            ci(entropy.as_ptr() as *mut u8, std::ptr::null_mut());
            ri(entropy.as_ptr() as *mut u8, std::ptr::null_mut());
            let (mut csm, mut rsm) = (vec![0u8; BYTES + len], vec![0u8; BYTES + len]);
            let (mut csmlen, mut rsmlen) = (0u64, 0u64);
            assert_eq!(csgn(csm.as_mut_ptr(), &mut csmlen, message.as_ptr(), len as u64, csk.as_ptr()), 0);
            assert_eq!(rsgn(rsm.as_mut_ptr(), &mut rsmlen, message.as_ptr(), len as u64, rsk.as_ptr()), 0);
            assert_eq!((csm.clone(), csmlen), (rsm.clone(), rsmlen));
            let (mut cout, mut rout) = (vec![0xa5; BYTES + len], vec![0xa5; BYTES + len]);
            let (mut colen, mut rolen) = (u64::MAX, u64::MAX);
            assert_eq!(cop(cout.as_mut_ptr(), &mut colen, csm.as_ptr(), csmlen, cpk.as_ptr()), 0);
            assert_eq!(rop(rout.as_mut_ptr(), &mut rolen, rsm.as_ptr(), rsmlen, rpk.as_ptr()), 0);
            assert_eq!((&cout[..colen as usize], colen), (&rout[..rolen as usize], rolen));
            assert_eq!(&cout[..len], message);
            let mut corrupt = csm.clone();
            corrupt[BYTES / 2] ^= 1;
            let (mut cbad, mut rbad) =
                (vec![0xa5; BYTES + len], vec![0xa5; BYTES + len]);
            let (mut cblen, mut rblen) = (u64::MAX, u64::MAX);
            assert_eq!(cop(cbad.as_mut_ptr(), &mut cblen, corrupt.as_ptr(), csmlen, cpk.as_ptr()), -1);
            assert_eq!(rop(rbad.as_mut_ptr(), &mut rblen, corrupt.as_ptr(), rsmlen, rpk.as_ptr()), -1);
            assert_eq!((cbad, cblen), (rbad, rblen));
        }

        let (cop, rop) = libs.pair::<Open>(b"crypto_sign_open\0");
        let short = vec![0x5a; BYTES - 1];
        let pk = vec![0u8; PK_BYTES];
        let (mut ca, mut ra) = (vec![0xa5; BYTES], vec![0xa5; BYTES]);
        let (mut cl, mut rl) = (99u64, 99u64);
        assert_eq!(cop(ca.as_mut_ptr(), &mut cl, short.as_ptr(), short.len() as u64, pk.as_ptr()), -1);
        assert_eq!(rop(ra.as_mut_ptr(), &mut rl, short.as_ptr(), short.len() as u64, pk.as_ptr()), -1);
        assert_eq!((ca, cl), (ra, rl));
    }
}

#[test]
fn drivers_match_stdout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_driver = std::env::var_os("SPHINCS_C_DRIVER")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("../c_src/build/app/driver"));
    let rust_driver = std::env::var_os("SPHINCS_RUST_DRIVER")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/release/driver"));
    if !c_driver.exists() {
        eprintln!("skipping absent C driver {}", c_driver.display());
        return;
    }
    let c = std::process::Command::new(c_driver).output().unwrap();
    let r = std::process::Command::new(rust_driver).output().unwrap();
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
}
