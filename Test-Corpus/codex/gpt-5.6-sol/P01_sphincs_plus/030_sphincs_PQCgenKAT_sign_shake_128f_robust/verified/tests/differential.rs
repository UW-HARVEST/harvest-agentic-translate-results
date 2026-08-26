use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_NOW};
use std::env;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Libraries {
    _crypto: Library,
    c_core: Library,
    c_backend: Library,
    c_system: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_build = env::var_os("SPHINCS_C_BUILD")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("c_src/build"));
        let rust_so = env::var_os("SPHINCS_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target/release/libsphincs_plus.so"));
        let core = env::var_os("SPHINCS_C_CORE")
            .map(PathBuf::from)
            .unwrap_or_else(|| c_build.join("app/libsphincs_core_det.so"));
        let system_core = c_build.join("app/libsphincs_core.so");
        let backend = c_build
            .join("lib")
            .join(backend_name())
            .join(format!("lib{}.so", backend_name()));

        assert_file(&core);
        assert_file(&system_core);
        assert_file(&backend);
        assert_file(&rust_so);

        // Load Rust first and locally so its SPX_* and RNG relocations bind to
        // its own definitions instead of the later C RTLD_GLOBAL group.
        let rust = Library::open(Some(&rust_so), RTLD_NOW)
            .unwrap_or_else(|e| panic!("load {}: {e}", rust_so.display()));

        // CMake links OpenSSL into the driver rather than the deterministic
        // core shared object, so reproduce the driver's global link context.
        let crypto = Library::open(Some(Path::new("libcrypto.so.3")), RTLD_NOW | RTLD_GLOBAL)
            .or_else(|_| Library::open(Some(Path::new("libcrypto.so")), RTLD_NOW | RTLD_GLOBAL))
            .unwrap_or_else(|e| panic!("load libcrypto: {e}"));

        // The two C libraries deliberately resolve symbols from one another.
        let c_core = Library::open(Some(&core), RTLD_LAZY | RTLD_GLOBAL)
            .unwrap_or_else(|e| panic!("load {}: {e}", core.display()));
        let c_backend = Library::open(Some(&backend), RTLD_NOW | RTLD_GLOBAL)
            .unwrap_or_else(|e| panic!("load {}: {e}", backend.display()));
        let c_system = Library::open(Some(&system_core), RTLD_NOW)
            .unwrap_or_else(|e| panic!("load {}: {e}", system_core.display()));
        Self {
            _crypto: crypto,
            c_core,
            c_backend,
            c_system,
            rust,
        }
    }

    unsafe fn c<T: Copy>(&self, name: &str) -> T {
        self.c_core
            .get::<T>(name.as_bytes())
            .or_else(|_| self.c_backend.get::<T>(name.as_bytes()))
            .map(|symbol| *symbol)
            .unwrap_or_else(|e| panic!("C symbol {name}: {e}"))
    }

    unsafe fn r<T: Copy>(&self, name: &str) -> T {
        *self
            .rust
            .get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("Rust symbol {name}: {e}"))
    }

    unsafe fn c_system<T: Copy>(&self, name: &str) -> T {
        *self
            .c_system
            .get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("C system-core symbol {name}: {e}"))
    }

    fn has_c(&self, name: &str) -> bool {
        unsafe {
            self.c_core.get::<*const c_void>(name.as_bytes()).is_ok()
                || self.c_backend.get::<*const c_void>(name.as_bytes()).is_ok()
        }
    }

    fn has_rust(&self, name: &str) -> bool {
        unsafe { self.rust.get::<*const c_void>(name.as_bytes()).is_ok() }
    }

    unsafe fn c_data<const N: usize>(&self, name: &str) -> [u8; N] {
        let address = *self
            .c_core
            .get::<*const u8>(name.as_bytes())
            .or_else(|_| self.c_backend.get::<*const u8>(name.as_bytes()))
            .unwrap_or_else(|e| panic!("C data symbol {name}: {e}"));
        let mut bytes = [0u8; N];
        ptr::copy_nonoverlapping(address, bytes.as_mut_ptr(), N);
        bytes
    }

    unsafe fn rust_data<const N: usize>(&self, name: &str) -> [u8; N] {
        let address = *self
            .rust
            .get::<*const u8>(name.as_bytes())
            .unwrap_or_else(|e| panic!("Rust data symbol {name}: {e}"));
        let mut bytes = [0u8; N];
        ptr::copy_nonoverlapping(address, bytes.as_mut_ptr(), N);
        bytes
    }
}

fn assert_file(path: &Path) {
    assert!(
        path.is_file(),
        "required shared object missing: {}",
        path.display()
    );
}

fn backend_name() -> &'static str {
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

#[derive(Clone, Copy)]
struct Params {
    n: usize,
    d: usize,
    tree_height: usize,
    fors_height: usize,
    fors_trees: usize,
    fors_msg_bytes: usize,
    fors_bytes: usize,
    wots_len: usize,
    wots_bytes: usize,
    bytes: usize,
    pk_bytes: usize,
    sk_bytes: usize,
    seed_bytes: usize,
}

impl Params {
    unsafe fn from_library(libs: &Libraries) -> Self {
        type SizeFn = unsafe extern "C" fn() -> u64;
        let sk_bytes = libs.c::<SizeFn>("crypto_sign_secretkeybytes")() as usize;
        let pk_bytes = libs.c::<SizeFn>("crypto_sign_publickeybytes")() as usize;
        let bytes = libs.c::<SizeFn>("crypto_sign_bytes")() as usize;
        let seed_bytes = libs.c::<SizeFn>("crypto_sign_seedbytes")() as usize;
        let n = pk_bytes / 2;
        let (d, full_height, fors_height, fors_trees) = secpar_dimensions();
        let tree_height = full_height / d;
        let wots_len = 2 * n + 3;
        let wots_bytes = wots_len * n;
        let fors_msg_bytes = (fors_height * fors_trees + 7) / 8;
        let fors_bytes = (fors_height + 1) * fors_trees * n;
        Self {
            n,
            d,
            tree_height,
            fors_height,
            fors_trees,
            fors_msg_bytes,
            fors_bytes,
            wots_len,
            wots_bytes,
            bytes,
            pk_bytes,
            sk_bytes,
            seed_bytes,
        }
    }
}

fn secpar_dimensions() -> (usize, usize, usize, usize) {
    if cfg!(feature = "128s") {
        (7, 63, 12, 14)
    } else if cfg!(feature = "128f") {
        (22, 66, 6, 33)
    } else if cfg!(feature = "192s") {
        (7, 63, 14, 17)
    } else if cfg!(feature = "192f") {
        (22, 66, 8, 33)
    } else if cfg!(feature = "256s") {
        (8, 64, 14, 22)
    } else {
        (17, 68, 9, 35)
    }
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x8f4d_3b2a_1907_65c1)
    }

    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let word = self.u64().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
    }
}

static CALLBACK_N: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn deterministic_leaf(
    leaf: *mut u8,
    _ctx: *const c_void,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let n = CALLBACK_N.load(Ordering::Relaxed);
    let addr = tree_addr.cast::<u8>();
    for i in 0..n {
        *leaf.add(i) = (addr_idx.rotate_left((i % 31) as u32) as u8)
            ^ *addr.add(i % 32)
            ^ (i as u8).wrapping_mul(29);
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

#[repr(C)]
struct ForsInfo {
    leaf_addr: [u32; 8],
}

fn assert_same(label: &str, c: &[u8], r: &[u8]) {
    assert_eq!(c, r, "{label} differs");
}

unsafe fn initialized_context(libs: &Libraries, p: Params, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    type Init = unsafe extern "C" fn(*mut c_void);
    let mut c = vec![0xa5; 2048];
    rng.fill(&mut c[..2 * p.n]);
    let mut r = c.clone();
    libs.c::<Init>("SPX_initialize_hash_function")(c.as_mut_ptr().cast());
    libs.r::<Init>("SPX_initialize_hash_function")(r.as_mut_ptr().cast());
    assert_same("SPX_initialize_hash_function context", &c, &r);
    (c, r)
}

fn expected_symbols() -> Vec<&'static str> {
    let mut names = vec![
        "AES256_CTR_DRBG_Update",
        "AES256_ECB",
        "DRBG_ctx",
        "SPX_bytes_to_ull",
        "SPX_chain_lengths",
        "SPX_compute_root",
        "SPX_copy_keypair_addr",
        "SPX_copy_subtree_addr",
        "SPX_fors_gen_leafx1",
        "SPX_fors_pk_from_sig",
        "SPX_fors_sign",
        "SPX_fors_treehashx1",
        "SPX_gen_message_random",
        "SPX_hash_message",
        "SPX_initialize_hash_function",
        "SPX_merkle_gen_root",
        "SPX_merkle_sign",
        "SPX_prf_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_keypair_addr",
        "SPX_set_layer_addr",
        "SPX_set_tree_addr",
        "SPX_set_tree_height",
        "SPX_set_tree_index",
        "SPX_set_type",
        "SPX_thash",
        "SPX_treehash",
        "SPX_u32_to_bytes",
        "SPX_ull_to_bytes",
        "SPX_wots_gen_leafx1",
        "SPX_wots_pk_from_sig",
        "SPX_wots_treehashx1",
        "crypto_sign",
        "crypto_sign_bytes",
        "crypto_sign_keypair",
        "crypto_sign_open",
        "crypto_sign_publickeybytes",
        "crypto_sign_secretkeybytes",
        "crypto_sign_seed_keypair",
        "crypto_sign_seedbytes",
        "crypto_sign_signature",
        "crypto_sign_verify",
        "randombytes",
        "randombytes_init",
        "seedexpander",
        "seedexpander_init",
    ];
    match backend_name() {
        "blake" => names.extend([
            "SPX_blake256_mgf1",
            "SPX_blake512_mgf1",
            "blake256",
            "blake256_compress",
            "blake256_final",
            "blake256_init",
            "blake256_update",
            "blake512",
            "blake512_compress",
            "blake512_final",
            "blake512_init",
            "blake512_update",
            "cst",
        ]),
        "sha2" => names.extend([
            "SPX_mgf1_256",
            "SPX_mgf1_512",
            "SPX_seed_state",
            "sha256",
            "sha256_inc_blocks",
            "sha256_inc_finalize",
            "sha256_inc_init",
            "sha512",
            "sha512_inc_blocks",
            "sha512_inc_finalize",
            "sha512_inc_init",
        ]),
        "shake" => names.extend([
            "shake256",
            "shake256_absorb",
            "shake256_inc_absorb",
            "shake256_inc_finalize",
            "shake256_inc_init",
            "shake256_inc_squeeze",
            "shake256_squeezeblocks",
        ]),
        "haraka" => names.extend([
            "SPX_haraka256",
            "SPX_haraka512",
            "SPX_haraka512_perm",
            "SPX_haraka_S",
            "SPX_haraka_S_inc_absorb",
            "SPX_haraka_S_inc_finalize",
            "SPX_haraka_S_inc_init",
            "SPX_haraka_S_inc_squeeze",
            "SPX_tweak_constants",
        ]),
        _ => unreachable!(),
    }
    names.sort_unstable();
    names.dedup();
    names
}

unsafe fn test_symbol_and_size_surface(libs: &Libraries, p: Params) {
    for name in expected_symbols() {
        assert!(libs.has_c(name), "C is missing expected symbol {name}");
        assert!(libs.has_rust(name), "Rust is missing C symbol {name}");
    }

    type SizeFn = unsafe extern "C" fn() -> u64;
    for name in [
        "crypto_sign_secretkeybytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
    ] {
        assert_eq!(libs.c::<SizeFn>(name)(), libs.r::<SizeFn>(name)(), "{name}");
    }
    assert_eq!(p.sk_bytes, 4 * p.n);
    assert_eq!(p.pk_bytes, 2 * p.n);
    assert_eq!(p.seed_bytes, 3 * p.n);
    assert_eq!(
        p.bytes,
        p.n + p.fors_bytes + p.d * p.wots_bytes + p.d * p.tree_height * p.n
    );
}

unsafe fn test_endian_and_address(libs: &Libraries, rng: &mut Rng) {
    type UllTo = unsafe extern "C" fn(*mut u8, u32, u64);
    type U32To = unsafe extern "C" fn(*mut u8, u32);
    type ToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
    let c_ull = libs.c::<UllTo>("SPX_ull_to_bytes");
    let r_ull = libs.r::<UllTo>("SPX_ull_to_bytes");
    let c_u32 = libs.c::<U32To>("SPX_u32_to_bytes");
    let r_u32 = libs.r::<U32To>("SPX_u32_to_bytes");
    let c_to = libs.c::<ToUll>("SPX_bytes_to_ull");
    let r_to = libs.r::<ToUll>("SPX_bytes_to_ull");

    for len in [0u32, 1, 4, 8] {
        for _ in 0..24 {
            let value = rng.u64();
            let mut c = [0x5a; 8];
            let mut r = c;
            c_ull(c.as_mut_ptr(), len, value);
            r_ull(r.as_mut_ptr(), len, value);
            assert_eq!(c, r, "ull_to_bytes len={len}");
            assert_eq!(c_to(c.as_ptr(), len), r_to(r.as_ptr(), len));
        }
    }
    for value in [0, 1, 0xff, 0x100, u32::MAX] {
        let mut c = [0u8; 4];
        let mut r = [0u8; 4];
        c_u32(c.as_mut_ptr(), value);
        r_u32(r.as_mut_ptr(), value);
        assert_eq!(c, r);
    }

    type Set32 = unsafe extern "C" fn(*mut u32, u32);
    type Set64 = unsafe extern "C" fn(*mut u32, u64);
    type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
    for name in [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_keypair_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
        "SPX_set_tree_index",
    ] {
        for value in [0, 1, 0xff, 0x100, u32::MAX] {
            let mut c = [0xa5a5_a5a5u32; 8];
            let mut r = c;
            libs.c::<Set32>(name)(c.as_mut_ptr(), value);
            libs.r::<Set32>(name)(r.as_mut_ptr(), value);
            assert_eq!(c, r, "{name} value={value:#x}");
        }
    }
    for value in [0, 1, 0xff, 0x100, u32::MAX as u64 + 1, u64::MAX] {
        let mut c = [0x5a5a_5a5au32; 8];
        let mut r = c;
        libs.c::<Set64>("SPX_set_tree_addr")(c.as_mut_ptr(), value);
        libs.r::<Set64>("SPX_set_tree_addr")(r.as_mut_ptr(), value);
        assert_eq!(c, r, "SPX_set_tree_addr value={value:#x}");
    }
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        for _ in 0..24 {
            let mut input = [0u32; 8];
            for value in &mut input {
                *value = rng.u64() as u32;
            }
            let mut c = [0xa5a5_a5a5u32; 8];
            let mut r = c;
            libs.c::<CopyAddr>(name)(c.as_mut_ptr(), input.as_ptr());
            libs.r::<CopyAddr>(name)(r.as_mut_ptr(), input.as_ptr());
            assert_eq!(c, r, "{name}");
        }
    }
}

unsafe fn test_blake(libs: &Libraries, rng: &mut Rng) {
    type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type Init = unsafe extern "C" fn(*mut c_void);
    type Update = unsafe extern "C" fn(*mut c_void, *const u8, u64);
    type Final = unsafe extern "C" fn(*mut c_void, *mut u8);
    type Compress = unsafe extern "C" fn(*mut c_void, *const u8);
    type Mgf = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    for (prefix, output, block, state_size, padding_points) in [
        ("blake256", 32usize, 64usize, 128usize, vec![54, 55, 56]),
        ("blake512", 64usize, 128usize, 248usize, vec![110, 111, 112]),
    ] {
        let mut lengths = vec![0, 1, block - 1, block, block + 1, 2 * block + 1];
        lengths.extend(padding_points);
        for len in lengths {
            let mut input = vec![0u8; len];
            rng.fill(&mut input);
            let mut c = vec![0xa5; output];
            let mut r = c.clone();
            let c_rc = libs.c::<Hash>(prefix)(c.as_mut_ptr(), input.as_ptr(), len as u64);
            let r_rc = libs.r::<Hash>(prefix)(r.as_mut_ptr(), input.as_ptr(), len as u64);
            assert_eq!(c_rc, r_rc, "{prefix} return");
            assert_same(prefix, &c, &r);

            let mut cs = vec![0xa5; state_size];
            let mut rs = cs.clone();
            libs.c::<Init>(&format!("{prefix}_init"))(cs.as_mut_ptr().cast());
            libs.r::<Init>(&format!("{prefix}_init"))(rs.as_mut_ptr().cast());
            assert_same(&format!("{prefix}_init"), &cs, &rs);
            let split = len.min(block.saturating_sub(1));
            for (offset, take) in [(0, split), (split, len - split)] {
                libs.c::<Update>(&format!("{prefix}_update"))(
                    cs.as_mut_ptr().cast(),
                    input.as_ptr().add(offset),
                    (take * 8) as u64,
                );
                libs.r::<Update>(&format!("{prefix}_update"))(
                    rs.as_mut_ptr().cast(),
                    input.as_ptr().add(offset),
                    (take * 8) as u64,
                );
                assert_same(&format!("{prefix}_update"), &cs, &rs);
            }
            let mut c_stream = vec![0u8; output];
            let mut r_stream = vec![0u8; output];
            libs.c::<Final>(&format!("{prefix}_final"))(
                cs.as_mut_ptr().cast(),
                c_stream.as_mut_ptr(),
            );
            libs.r::<Final>(&format!("{prefix}_final"))(
                rs.as_mut_ptr().cast(),
                r_stream.as_mut_ptr(),
            );
            assert_same(&format!("{prefix}_final"), &c_stream, &r_stream);
        }

        let mut cs = vec![0u8; state_size];
        let mut rs = cs.clone();
        let mut block_data = vec![0u8; block];
        rng.fill(&mut cs);
        rs.copy_from_slice(&cs);
        rng.fill(&mut block_data);
        libs.c::<Compress>(&format!("{prefix}_compress"))(
            cs.as_mut_ptr().cast(),
            block_data.as_ptr(),
        );
        libs.r::<Compress>(&format!("{prefix}_compress"))(
            rs.as_mut_ptr().cast(),
            block_data.as_ptr(),
        );
        assert_same(&format!("{prefix}_compress"), &cs, &rs);

        let mgf = format!("SPX_{prefix}_mgf1");
        for inlen in [0usize, 1, 16, 32] {
            for outlen in [0usize, 1, output - 1, output, output + 1, 2 * output + 1] {
                let mut input = vec![0u8; inlen];
                rng.fill(&mut input);
                let mut c = vec![0xa5; outlen];
                let mut r = c.clone();
                libs.c::<Mgf>(&mgf)(c.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64);
                libs.r::<Mgf>(&mgf)(r.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64);
                assert_same(&mgf, &c, &r);
            }
        }
    }
}

unsafe fn test_sha2(libs: &Libraries, rng: &mut Rng) {
    type Hash = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type Init = unsafe extern "C" fn(*mut u8);
    type Blocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type Final = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type Mgf = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    for (bits, output, block, state_len, pad) in [
        (256usize, 32usize, 64usize, 40usize, 56usize),
        (512, 64, 128, 72, 112),
    ] {
        for len in [
            0,
            1,
            pad - 1,
            pad,
            block - 1,
            block,
            block + 1,
            2 * block + 1,
        ] {
            let mut input = vec![0u8; len];
            rng.fill(&mut input);
            let mut c = vec![0u8; output];
            let mut r = vec![0u8; output];
            let hash = format!("sha{bits}");
            libs.c::<Hash>(&hash)(c.as_mut_ptr(), input.as_ptr(), len);
            libs.r::<Hash>(&hash)(r.as_mut_ptr(), input.as_ptr(), len);
            assert_same(&hash, &c, &r);

            let blocks = len / block;
            let tail = len % block;
            let mut cs = vec![0xa5; state_len];
            let mut rs = cs.clone();
            libs.c::<Init>(&format!("{hash}_inc_init"))(cs.as_mut_ptr());
            libs.r::<Init>(&format!("{hash}_inc_init"))(rs.as_mut_ptr());
            libs.c::<Blocks>(&format!("{hash}_inc_blocks"))(
                cs.as_mut_ptr(),
                input.as_ptr(),
                blocks,
            );
            libs.r::<Blocks>(&format!("{hash}_inc_blocks"))(
                rs.as_mut_ptr(),
                input.as_ptr(),
                blocks,
            );
            assert_same(&format!("{hash}_inc_blocks"), &cs, &rs);
            let mut ci = vec![0u8; output];
            let mut ri = vec![0u8; output];
            libs.c::<Final>(&format!("{hash}_inc_finalize"))(
                ci.as_mut_ptr(),
                cs.as_mut_ptr(),
                input.as_ptr().add(blocks * block),
                tail,
            );
            libs.r::<Final>(&format!("{hash}_inc_finalize"))(
                ri.as_mut_ptr(),
                rs.as_mut_ptr(),
                input.as_ptr().add(blocks * block),
                tail,
            );
            assert_same(&format!("{hash}_inc_finalize"), &ci, &ri);
        }
        for outlen in [0usize, 1, output - 1, output, output + 1, 2 * output + 1] {
            let mut input = vec![0u8; 33];
            rng.fill(&mut input);
            let mut c = vec![0u8; outlen];
            let mut r = vec![0u8; outlen];
            let name = format!("SPX_mgf1_{bits}");
            libs.c::<Mgf>(&name)(c.as_mut_ptr(), outlen as u64, input.as_ptr(), 33);
            libs.r::<Mgf>(&name)(r.as_mut_ptr(), outlen as u64, input.as_ptr(), 33);
            assert_same(&name, &c, &r);
        }
    }
}

unsafe fn test_shake(libs: &Libraries, rng: &mut Rng) {
    type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type Init = unsafe extern "C" fn(*mut u64);
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type Final = unsafe extern "C" fn(*mut u64);
    type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    let rate = 136usize;
    for inlen in [0, 1, rate - 1, rate, rate + 1, 2 * rate + 1] {
        for outlen in [0, 1, rate - 1, rate, rate + 1] {
            let mut input = vec![0u8; inlen];
            rng.fill(&mut input);
            let mut c = vec![0u8; outlen];
            let mut r = vec![0u8; outlen];
            libs.c::<Shake>("shake256")(c.as_mut_ptr(), outlen, input.as_ptr(), inlen);
            libs.r::<Shake>("shake256")(r.as_mut_ptr(), outlen, input.as_ptr(), inlen);
            assert_same("shake256", &c, &r);

            let mut cs = [0u64; 26];
            let mut rs = [0u64; 26];
            libs.c::<Init>("shake256_inc_init")(cs.as_mut_ptr());
            libs.r::<Init>("shake256_inc_init")(rs.as_mut_ptr());
            let split = inlen / 2;
            for (offset, take) in [(0, split), (split, inlen - split)] {
                libs.c::<Absorb>("shake256_inc_absorb")(
                    cs.as_mut_ptr(),
                    input.as_ptr().add(offset),
                    take,
                );
                libs.r::<Absorb>("shake256_inc_absorb")(
                    rs.as_mut_ptr(),
                    input.as_ptr().add(offset),
                    take,
                );
            }
            libs.c::<Final>("shake256_inc_finalize")(cs.as_mut_ptr());
            libs.r::<Final>("shake256_inc_finalize")(rs.as_mut_ptr());
            let mut ci = vec![0u8; outlen];
            let mut ri = vec![0u8; outlen];
            libs.c::<Squeeze>("shake256_inc_squeeze")(ci.as_mut_ptr(), outlen, cs.as_mut_ptr());
            libs.r::<Squeeze>("shake256_inc_squeeze")(ri.as_mut_ptr(), outlen, rs.as_mut_ptr());
            assert_same("shake256 incremental", &ci, &ri);
            assert_eq!(cs, rs, "shake256 incremental state");
        }
    }
}

unsafe fn test_haraka(libs: &Libraries, p: Params, rng: &mut Rng) {
    let (c_ctx, r_ctx) = initialized_context(libs, p, rng);
    type Fixed = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
    for (name, inlen, outlen) in [
        ("SPX_haraka256", 32usize, 32usize),
        ("SPX_haraka512", 64, 32),
        ("SPX_haraka512_perm", 64, 64),
    ] {
        for _ in 0..24 {
            let mut input = vec![0u8; inlen];
            rng.fill(&mut input);
            let mut c = vec![0u8; outlen];
            let mut r = vec![0u8; outlen];
            libs.c::<Fixed>(name)(c.as_mut_ptr(), input.as_ptr(), c_ctx.as_ptr().cast());
            libs.r::<Fixed>(name)(r.as_mut_ptr(), input.as_ptr(), r_ctx.as_ptr().cast());
            assert_same(name, &c, &r);
        }
    }

    type Sponge = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
    for inlen in [0usize, 1, 31, 32, 33, 64, 65] {
        for outlen in [0usize, 1, 31, 32, 33, 64, 65] {
            let mut input = vec![0u8; inlen];
            rng.fill(&mut input);
            let mut c = vec![0u8; outlen];
            let mut r = vec![0u8; outlen];
            libs.c::<Sponge>("SPX_haraka_S")(
                c.as_mut_ptr(),
                outlen as u64,
                input.as_ptr(),
                inlen as u64,
                c_ctx.as_ptr().cast(),
            );
            libs.r::<Sponge>("SPX_haraka_S")(
                r.as_mut_ptr(),
                outlen as u64,
                input.as_ptr(),
                inlen as u64,
                r_ctx.as_ptr().cast(),
            );
            assert_same("SPX_haraka_S", &c, &r);
        }
    }
}

unsafe fn test_hash_api(libs: &Libraries, p: Params, rng: &mut Rng) {
    let (c_ctx, r_ctx) = initialized_context(libs, p, rng);
    let mut addr = [0u32; 8];
    for value in &mut addr {
        *value = rng.u64() as u32;
    }

    type Prf = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
    for _ in 0..32 {
        let mut c = vec![0u8; p.n];
        let mut r = vec![0u8; p.n];
        libs.c::<Prf>("SPX_prf_addr")(c.as_mut_ptr(), c_ctx.as_ptr().cast(), addr.as_ptr());
        libs.r::<Prf>("SPX_prf_addr")(r.as_mut_ptr(), r_ctx.as_ptr().cast(), addr.as_ptr());
        assert_same("SPX_prf_addr", &c, &r);
        addr[7] = addr[7].wrapping_add(1);
    }

    type GenRandom =
        unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const c_void);
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
    for mlen in [0usize, 1, 31, 32, 63, 64, 65, 127, 128, 129] {
        let mut message = vec![0u8; mlen];
        let mut sk_prf = vec![0u8; p.n];
        let mut optrand = vec![0u8; p.n];
        let mut pk = vec![0u8; p.pk_bytes];
        rng.fill(&mut message);
        rng.fill(&mut sk_prf);
        rng.fill(&mut optrand);
        rng.fill(&mut pk);
        let mut cr = vec![0u8; p.n.max(64)];
        let mut rr = vec![0u8; p.n.max(64)];
        libs.c::<GenRandom>("SPX_gen_message_random")(
            cr.as_mut_ptr(),
            sk_prf.as_ptr(),
            optrand.as_ptr(),
            message.as_ptr(),
            mlen as u64,
            c_ctx.as_ptr().cast(),
        );
        libs.r::<GenRandom>("SPX_gen_message_random")(
            rr.as_mut_ptr(),
            sk_prf.as_ptr(),
            optrand.as_ptr(),
            message.as_ptr(),
            mlen as u64,
            r_ctx.as_ptr().cast(),
        );
        assert_same("SPX_gen_message_random", &cr[..p.n], &rr[..p.n]);

        let mut cd = vec![0u8; p.fors_msg_bytes];
        let mut rd = vec![0u8; p.fors_msg_bytes];
        let (mut ct, mut rt) = (0u64, 0u64);
        let (mut cl, mut rl) = (0u32, 0u32);
        libs.c::<HashMessage>("SPX_hash_message")(
            cd.as_mut_ptr(),
            &mut ct,
            &mut cl,
            cr.as_ptr(),
            pk.as_ptr(),
            message.as_ptr(),
            mlen as u64,
            c_ctx.as_ptr().cast(),
        );
        libs.r::<HashMessage>("SPX_hash_message")(
            rd.as_mut_ptr(),
            &mut rt,
            &mut rl,
            rr.as_ptr(),
            pk.as_ptr(),
            message.as_ptr(),
            mlen as u64,
            r_ctx.as_ptr().cast(),
        );
        assert_same("SPX_hash_message digest", &cd, &rd);
        assert_eq!((ct, cl), (rt, rl), "SPX_hash_message indices");
    }

    type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
    for blocks in [0usize, 1, 2, p.wots_len, p.fors_trees] {
        let mut input = vec![0u8; blocks * p.n];
        rng.fill(&mut input);
        let mut c = vec![0u8; p.n];
        let mut r = vec![0u8; p.n];
        let mut ca = addr;
        let mut ra = addr;
        libs.c::<Thash>("SPX_thash")(
            c.as_mut_ptr(),
            input.as_ptr(),
            blocks as u32,
            c_ctx.as_ptr().cast(),
            ca.as_mut_ptr(),
        );
        libs.r::<Thash>("SPX_thash")(
            r.as_mut_ptr(),
            input.as_ptr(),
            blocks as u32,
            r_ctx.as_ptr().cast(),
            ra.as_mut_ptr(),
        );
        assert_same("SPX_thash", &c, &r);
        assert_eq!(ca, ra, "SPX_thash address");
    }
}

unsafe fn test_wots_and_roots(libs: &Libraries, p: Params, rng: &mut Rng) {
    let (c_ctx, r_ctx) = initialized_context(libs, p, rng);
    type Chain = unsafe extern "C" fn(*mut u32, *const u8);
    for fill in [0u8, 0xff] {
        let msg = vec![fill; p.n];
        let mut c = vec![0u32; p.wots_len];
        let mut r = c.clone();
        libs.c::<Chain>("SPX_chain_lengths")(c.as_mut_ptr(), msg.as_ptr());
        libs.r::<Chain>("SPX_chain_lengths")(r.as_mut_ptr(), msg.as_ptr());
        assert_eq!(c, r, "SPX_chain_lengths");
    }

    type Wots = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *mut u32);
    let mut sig = vec![0u8; p.wots_bytes];
    let mut msg = vec![0u8; p.n];
    rng.fill(&mut sig);
    rng.fill(&mut msg);
    let mut ca = [0u32; 8];
    rng.fill(std::slice::from_raw_parts_mut(
        ca.as_mut_ptr().cast::<u8>(),
        32,
    ));
    let mut ra = ca;
    let mut c = vec![0u8; p.wots_bytes];
    let mut r = c.clone();
    libs.c::<Wots>("SPX_wots_pk_from_sig")(
        c.as_mut_ptr(),
        sig.as_ptr(),
        msg.as_ptr(),
        c_ctx.as_ptr().cast(),
        ca.as_mut_ptr(),
    );
    libs.r::<Wots>("SPX_wots_pk_from_sig")(
        r.as_mut_ptr(),
        sig.as_ptr(),
        msg.as_ptr(),
        r_ctx.as_ptr().cast(),
        ra.as_mut_ptr(),
    );
    assert_same("SPX_wots_pk_from_sig", &c, &r);
    assert_eq!(ca, ra);

    type ComputeRoot =
        unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const c_void, *mut u32);
    for height in [1usize, p.tree_height] {
        for leaf in [0u32, 1, (1u32 << height) - 1] {
            let mut node = vec![0u8; p.n];
            let mut auth = vec![0u8; height * p.n];
            rng.fill(&mut node);
            rng.fill(&mut auth);
            let mut co = vec![0u8; p.n];
            let mut ro = vec![0u8; p.n];
            let mut ca = [0u32; 8];
            let mut ra = ca;
            libs.c::<ComputeRoot>("SPX_compute_root")(
                co.as_mut_ptr(),
                node.as_ptr(),
                leaf,
                3,
                auth.as_ptr(),
                height as u32,
                c_ctx.as_ptr().cast(),
                ca.as_mut_ptr(),
            );
            libs.r::<ComputeRoot>("SPX_compute_root")(
                ro.as_mut_ptr(),
                node.as_ptr(),
                leaf,
                3,
                auth.as_ptr(),
                height as u32,
                r_ctx.as_ptr().cast(),
                ra.as_mut_ptr(),
            );
            assert_same("SPX_compute_root", &co, &ro);
            assert_eq!(ca, ra);
        }
    }
}

unsafe fn test_direct_tree_fors_merkle(libs: &Libraries, p: Params, rng: &mut Rng) {
    let (c_ctx, r_ctx) = initialized_context(libs, p, rng);

    type Treehash = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const c_void,
        u32,
        u32,
        u32,
        Option<unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32)>,
        *mut u32,
    );
    CALLBACK_N.store(p.n, Ordering::Relaxed);
    for height in [1usize, 2, p.tree_height] {
        for leaf in [0u32, (1u32 << height) / 2, (1u32 << height) - 1] {
            let mut croot = vec![0u8; p.n];
            let mut rroot = vec![0u8; p.n];
            let mut cauth = vec![0u8; height * p.n];
            let mut rauth = vec![0u8; height * p.n];
            let mut ca = [0u32; 8];
            rng.fill(std::slice::from_raw_parts_mut(
                ca.as_mut_ptr().cast::<u8>(),
                32,
            ));
            let mut ra = ca;
            libs.c::<Treehash>("SPX_treehash")(
                croot.as_mut_ptr(),
                cauth.as_mut_ptr(),
                c_ctx.as_ptr().cast(),
                leaf,
                7,
                height as u32,
                Some(deterministic_leaf),
                ca.as_mut_ptr(),
            );
            libs.r::<Treehash>("SPX_treehash")(
                rroot.as_mut_ptr(),
                rauth.as_mut_ptr(),
                r_ctx.as_ptr().cast(),
                leaf,
                7,
                height as u32,
                Some(deterministic_leaf),
                ra.as_mut_ptr(),
            );
            assert_same("SPX_treehash root", &croot, &rroot);
            assert_same("SPX_treehash auth", &cauth, &rauth);
            assert_eq!(ca, ra, "SPX_treehash address");
        }
    }

    type WotsLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfo);
    let mut steps = vec![0u32; p.wots_len];
    for step in &mut steps {
        *step = (rng.u64() % 16) as u32;
    }
    for signing in [false, true] {
        let leaf_idx = 3u32;
        let sign_leaf = if signing { leaf_idx } else { u32::MAX };
        let mut csig = vec![0xa5; p.wots_bytes];
        let mut rsig = csig.clone();
        let mut ci = LeafInfo {
            wots_sig: if signing {
                csig.as_mut_ptr()
            } else {
                ptr::null_mut()
            },
            wots_sign_leaf: sign_leaf,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        };
        rng.fill(std::slice::from_raw_parts_mut(
            ci.leaf_addr.as_mut_ptr().cast::<u8>(),
            64,
        ));
        let mut ri = LeafInfo {
            wots_sig: if signing {
                rsig.as_mut_ptr()
            } else {
                ptr::null_mut()
            },
            wots_sign_leaf: sign_leaf,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: ci.leaf_addr,
            pk_addr: ci.pk_addr,
        };
        let mut c = vec![0u8; p.n];
        let mut r = vec![0u8; p.n];
        libs.c::<WotsLeaf>("SPX_wots_gen_leafx1")(
            c.as_mut_ptr(),
            c_ctx.as_ptr().cast(),
            leaf_idx,
            &mut ci,
        );
        libs.r::<WotsLeaf>("SPX_wots_gen_leafx1")(
            r.as_mut_ptr(),
            r_ctx.as_ptr().cast(),
            leaf_idx,
            &mut ri,
        );
        assert_same("SPX_wots_gen_leafx1 leaf", &c, &r);
        assert_same("SPX_wots_gen_leafx1 sig", &csig, &rsig);
        assert_eq!(ci.leaf_addr, ri.leaf_addr);
        assert_eq!(ci.pk_addr, ri.pk_addr);
    }

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
    for height in [1usize, p.tree_height] {
        let leaf = (1u32 << height) / 2;
        let mut csig = vec![0u8; p.wots_bytes];
        let mut rsig = vec![0u8; p.wots_bytes];
        let mut ci = LeafInfo {
            wots_sig: csig.as_mut_ptr(),
            wots_sign_leaf: leaf,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        };
        let mut ri = LeafInfo {
            wots_sig: rsig.as_mut_ptr(),
            wots_sign_leaf: leaf,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: ci.leaf_addr,
            pk_addr: ci.pk_addr,
        };
        let mut ca = [0u32; 8];
        let mut ra = ca;
        let mut croot = vec![0u8; p.n];
        let mut rroot = vec![0u8; p.n];
        let mut cauth = vec![0u8; height * p.n];
        let mut rauth = vec![0u8; height * p.n];
        libs.c::<WotsTree>("SPX_wots_treehashx1")(
            croot.as_mut_ptr(),
            cauth.as_mut_ptr(),
            c_ctx.as_ptr().cast(),
            leaf,
            0,
            height as u32,
            ca.as_mut_ptr(),
            &mut ci,
        );
        libs.r::<WotsTree>("SPX_wots_treehashx1")(
            rroot.as_mut_ptr(),
            rauth.as_mut_ptr(),
            r_ctx.as_ptr().cast(),
            leaf,
            0,
            height as u32,
            ra.as_mut_ptr(),
            &mut ri,
        );
        assert_same("SPX_wots_treehashx1 root", &croot, &rroot);
        assert_same("SPX_wots_treehashx1 auth", &cauth, &rauth);
        assert_same("SPX_wots_treehashx1 sig", &csig, &rsig);
        assert_eq!(ca, ra);
    }

    type ForsLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut ForsInfo);
    let mut cfi = ForsInfo {
        leaf_addr: [0u32; 8],
    };
    rng.fill(std::slice::from_raw_parts_mut(
        cfi.leaf_addr.as_mut_ptr().cast::<u8>(),
        32,
    ));
    let mut rfi = ForsInfo {
        leaf_addr: cfi.leaf_addr,
    };
    let mut cleaf = vec![0u8; p.n];
    let mut rleaf = vec![0u8; p.n];
    libs.c::<ForsLeaf>("SPX_fors_gen_leafx1")(
        cleaf.as_mut_ptr(),
        c_ctx.as_ptr().cast(),
        13,
        &mut cfi,
    );
    libs.r::<ForsLeaf>("SPX_fors_gen_leafx1")(
        rleaf.as_mut_ptr(),
        r_ctx.as_ptr().cast(),
        13,
        &mut rfi,
    );
    assert_same("SPX_fors_gen_leafx1", &cleaf, &rleaf);
    assert_eq!(cfi.leaf_addr, rfi.leaf_addr);

    type ForsTree = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const c_void,
        u32,
        u32,
        u32,
        *mut u32,
        *mut ForsInfo,
    );
    for height in [1usize, p.fors_height] {
        let leaf = (1u32 << height) / 2;
        let mut croot = vec![0u8; p.n];
        let mut rroot = vec![0u8; p.n];
        let mut cauth = vec![0u8; height * p.n];
        let mut rauth = vec![0u8; height * p.n];
        let mut ca = [0u32; 8];
        let mut ra = ca;
        let mut ci = ForsInfo {
            leaf_addr: [0u32; 8],
        };
        let mut ri = ForsInfo {
            leaf_addr: ci.leaf_addr,
        };
        libs.c::<ForsTree>("SPX_fors_treehashx1")(
            croot.as_mut_ptr(),
            cauth.as_mut_ptr(),
            c_ctx.as_ptr().cast(),
            leaf,
            17,
            height as u32,
            ca.as_mut_ptr(),
            &mut ci,
        );
        libs.r::<ForsTree>("SPX_fors_treehashx1")(
            rroot.as_mut_ptr(),
            rauth.as_mut_ptr(),
            r_ctx.as_ptr().cast(),
            leaf,
            17,
            height as u32,
            ra.as_mut_ptr(),
            &mut ri,
        );
        assert_same("SPX_fors_treehashx1 root", &croot, &rroot);
        assert_same("SPX_fors_treehashx1 auth", &cauth, &rauth);
        assert_eq!(ca, ra);
        assert_eq!(ci.leaf_addr, ri.leaf_addr);
    }

    type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32);
    type ForsPk = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32);
    let mut message = vec![0u8; p.fors_msg_bytes];
    rng.fill(&mut message);
    let mut addr = [0u32; 8];
    rng.fill(std::slice::from_raw_parts_mut(
        addr.as_mut_ptr().cast::<u8>(),
        32,
    ));
    let mut csig = vec![0u8; p.fors_bytes];
    let mut rsig = vec![0u8; p.fors_bytes];
    let mut cpk = vec![0u8; p.n];
    let mut rpk = vec![0u8; p.n];
    libs.c::<ForsSign>("SPX_fors_sign")(
        csig.as_mut_ptr(),
        cpk.as_mut_ptr(),
        message.as_ptr(),
        c_ctx.as_ptr().cast(),
        addr.as_ptr(),
    );
    libs.r::<ForsSign>("SPX_fors_sign")(
        rsig.as_mut_ptr(),
        rpk.as_mut_ptr(),
        message.as_ptr(),
        r_ctx.as_ptr().cast(),
        addr.as_ptr(),
    );
    assert_same("SPX_fors_sign sig", &csig, &rsig);
    assert_same("SPX_fors_sign pk", &cpk, &rpk);
    let mut cderived = vec![0u8; p.n];
    let mut rderived = vec![0u8; p.n];
    libs.c::<ForsPk>("SPX_fors_pk_from_sig")(
        cderived.as_mut_ptr(),
        csig.as_ptr(),
        message.as_ptr(),
        c_ctx.as_ptr().cast(),
        addr.as_ptr(),
    );
    libs.r::<ForsPk>("SPX_fors_pk_from_sig")(
        rderived.as_mut_ptr(),
        rsig.as_ptr(),
        message.as_ptr(),
        r_ctx.as_ptr().cast(),
        addr.as_ptr(),
    );
    assert_same("SPX_fors_pk_from_sig", &cderived, &rderived);
    assert_eq!(cderived, cpk);

    type MerkleSign =
        unsafe extern "C" fn(*mut u8, *mut u8, *const c_void, *mut u32, *mut u32, u32);
    for leaf in [
        0u32,
        (1u32 << p.tree_height) / 2,
        (1u32 << p.tree_height) - 1,
    ] {
        let sig_len = p.wots_bytes + p.tree_height * p.n;
        let mut csig = vec![0u8; sig_len];
        let mut rsig = vec![0u8; sig_len];
        let mut croot = vec![0u8; p.n];
        rng.fill(&mut croot);
        let mut rroot = croot.clone();
        let mut cw = [0u32; 8];
        let mut rw = cw;
        let mut ct = [0u32; 8];
        let mut rt = ct;
        libs.c::<MerkleSign>("SPX_merkle_sign")(
            csig.as_mut_ptr(),
            croot.as_mut_ptr(),
            c_ctx.as_ptr().cast(),
            cw.as_mut_ptr(),
            ct.as_mut_ptr(),
            leaf,
        );
        libs.r::<MerkleSign>("SPX_merkle_sign")(
            rsig.as_mut_ptr(),
            rroot.as_mut_ptr(),
            r_ctx.as_ptr().cast(),
            rw.as_mut_ptr(),
            rt.as_mut_ptr(),
            leaf,
        );
        assert_same("SPX_merkle_sign sig", &csig, &rsig);
        assert_same("SPX_merkle_sign root", &croot, &rroot);
        assert_eq!((cw, ct), (rw, rt));
    }

    type MerkleRoot = unsafe extern "C" fn(*mut u8, *const c_void);
    let mut croot = vec![0u8; p.n];
    let mut rroot = vec![0u8; p.n];
    libs.c::<MerkleRoot>("SPX_merkle_gen_root")(croot.as_mut_ptr(), c_ctx.as_ptr().cast());
    libs.r::<MerkleRoot>("SPX_merkle_gen_root")(rroot.as_mut_ptr(), r_ctx.as_ptr().cast());
    assert_same("SPX_merkle_gen_root", &croot, &rroot);
}

unsafe fn test_rng_surface(libs: &Libraries, rng: &mut Rng) {
    type Aes = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    type Update = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    type InitDrbg = unsafe extern "C" fn(*mut u8, *mut u8);
    type Random = unsafe extern "C" fn(*mut u8, u64) -> i32;
    type SeedInit = unsafe extern "C" fn(*mut c_void, *mut u8, *mut u8, u64) -> i32;
    type Seed = unsafe extern "C" fn(*mut c_void, *mut u8, u64) -> i32;

    for _ in 0..32 {
        let mut key = [0u8; 32];
        let mut ctr = [0u8; 16];
        rng.fill(&mut key);
        rng.fill(&mut ctr);
        let mut c = [0u8; 16];
        let mut r = [0u8; 16];
        libs.c::<Aes>("AES256_ECB")(key.as_mut_ptr(), ctr.as_mut_ptr(), c.as_mut_ptr());
        libs.r::<Aes>("AES256_ECB")(key.as_mut_ptr(), ctr.as_mut_ptr(), r.as_mut_ptr());
        assert_eq!(c, r, "AES256_ECB");
    }

    for with_data in [false, true] {
        let mut ck = [0u8; 32];
        let mut cv = [0xffu8; 16];
        let mut rv = cv;
        let mut data = [0u8; 48];
        rng.fill(&mut ck);
        let mut rk = ck;
        rng.fill(&mut data);
        let data_ptr = if with_data {
            data.as_mut_ptr()
        } else {
            ptr::null_mut()
        };
        libs.c::<Update>("AES256_CTR_DRBG_Update")(data_ptr, ck.as_mut_ptr(), cv.as_mut_ptr());
        libs.r::<Update>("AES256_CTR_DRBG_Update")(data_ptr, rk.as_mut_ptr(), rv.as_mut_ptr());
        assert_eq!((ck, cv), (rk, rv), "AES256_CTR_DRBG_Update");
    }

    let mut seed = [0u8; 32];
    let mut diversifier = [0u8; 8];
    rng.fill(&mut seed);
    rng.fill(&mut diversifier);
    for maxlen in [0u64, 1, 15, 16, 17, u32::MAX as u64, 1u64 << 32] {
        let mut cctx = [0xa5u8; 80];
        let mut rctx = cctx;
        let cr = libs.c::<SeedInit>("seedexpander_init")(
            cctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            maxlen,
        );
        let rr = libs.r::<SeedInit>("seedexpander_init")(
            rctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            maxlen,
        );
        assert_eq!(cr, rr, "seedexpander_init maxlen={maxlen}");
        assert_eq!(cctx, rctx, "seedexpander_init state maxlen={maxlen}");
    }

    let mut cctx = [0u8; 80];
    let mut rctx = [0u8; 80];
    assert_eq!(
        libs.c::<SeedInit>("seedexpander_init")(
            cctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            257,
        ),
        0
    );
    assert_eq!(
        libs.r::<SeedInit>("seedexpander_init")(
            rctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            257,
        ),
        0
    );
    for len in [0u64, 1, 15, 16, 17, 31, 32, 33] {
        let mut c = vec![0u8; len as usize];
        let mut r = vec![0u8; len as usize];
        let cr = libs.c::<Seed>("seedexpander")(cctx.as_mut_ptr().cast(), c.as_mut_ptr(), len);
        let rr = libs.r::<Seed>("seedexpander")(rctx.as_mut_ptr().cast(), r.as_mut_ptr(), len);
        assert_eq!(cr, rr, "seedexpander return len={len}");
        assert_same("seedexpander", &c, &r);
        assert_eq!(cctx, rctx, "seedexpander state len={len}");
    }

    let mut entropy = [0u8; 48];
    let mut personalization = [0u8; 48];
    rng.fill(&mut entropy);
    rng.fill(&mut personalization);
    for personalized in [false, true] {
        let personalization_ptr = if personalized {
            personalization.as_mut_ptr()
        } else {
            ptr::null_mut()
        };
        libs.c::<InitDrbg>("randombytes_init")(entropy.as_mut_ptr(), personalization_ptr);
        libs.r::<InitDrbg>("randombytes_init")(entropy.as_mut_ptr(), personalization_ptr);
        assert_eq!(
            libs.c_data::<52>("DRBG_ctx"),
            libs.rust_data::<52>("DRBG_ctx"),
            "DRBG_ctx after randombytes_init personalized={personalized}"
        );
        for len in [0u64, 1, 15, 16, 17, 31, 32, 33] {
            let mut c = vec![0u8; len as usize];
            let mut r = vec![0u8; len as usize];
            let cr = libs.c::<Random>("randombytes")(c.as_mut_ptr(), len);
            let rr = libs.r::<Random>("randombytes")(r.as_mut_ptr(), len);
            assert_eq!(cr, rr, "randombytes return");
            assert_same("randombytes", &c, &r);
            assert_eq!(
                libs.c_data::<52>("DRBG_ctx"),
                libs.rust_data::<52>("DRBG_ctx"),
                "DRBG_ctx after randombytes len={len}"
            );
        }
    }
}

unsafe fn test_system_random_surface(libs: &Libraries) {
    // The non-deterministic core has the same symbol but reads /dev/urandom.
    // Compare only its API write extent; random bytes cannot be equal by
    // definition. Guard bytes detect short/oversized writes at chunk borders.
    type SystemRandom = unsafe extern "C" fn(*mut u8, u64);
    for len in [0usize, 1, 1_048_575, 1_048_576, 1_048_577] {
        let mut output = vec![0xa5; len + 32];
        libs.c_system::<SystemRandom>("randombytes")(output.as_mut_ptr().add(16), len as u64);
        assert_eq!(
            &output[..16],
            &[0xa5; 16],
            "system randombytes prefix guard"
        );
        assert_eq!(
            &output[16 + len..],
            &[0xa5; 16],
            "system randombytes suffix guard"
        );
    }
}

unsafe fn reset_drbg(libs: &Libraries, entropy: &mut [u8; 48]) {
    type Init = unsafe extern "C" fn(*mut u8, *mut u8);
    libs.c::<Init>("randombytes_init")(entropy.as_mut_ptr(), ptr::null_mut());
    libs.r::<Init>("randombytes_init")(entropy.as_mut_ptr(), ptr::null_mut());
}

unsafe fn test_signing_and_errors(libs: &Libraries, p: Params, rng: &mut Rng) {
    type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
    type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    type SignDetached =
        unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
    type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
    type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;

    let mut seed = vec![0u8; p.seed_bytes];
    rng.fill(&mut seed);
    let mut cpk = vec![0u8; p.pk_bytes];
    let mut rpk = vec![0u8; p.pk_bytes];
    let mut csk = vec![0u8; p.sk_bytes];
    let mut rsk = vec![0u8; p.sk_bytes];
    assert_eq!(
        libs.c::<SeedKeypair>("crypto_sign_seed_keypair")(
            cpk.as_mut_ptr(),
            csk.as_mut_ptr(),
            seed.as_ptr(),
        ),
        libs.r::<SeedKeypair>("crypto_sign_seed_keypair")(
            rpk.as_mut_ptr(),
            rsk.as_mut_ptr(),
            seed.as_ptr(),
        )
    );
    assert_same("crypto_sign_seed_keypair pk", &cpk, &rpk);
    assert_same("crypto_sign_seed_keypair sk", &csk, &rsk);

    let mut entropy = [0u8; 48];
    rng.fill(&mut entropy);
    reset_drbg(libs, &mut entropy);
    let mut cpk2 = vec![0u8; p.pk_bytes];
    let mut rpk2 = vec![0u8; p.pk_bytes];
    let mut csk2 = vec![0u8; p.sk_bytes];
    let mut rsk2 = vec![0u8; p.sk_bytes];
    let c_rc = libs.c::<Keypair>("crypto_sign_keypair")(cpk2.as_mut_ptr(), csk2.as_mut_ptr());
    let r_rc = libs.r::<Keypair>("crypto_sign_keypair")(rpk2.as_mut_ptr(), rsk2.as_mut_ptr());
    assert_eq!(c_rc, r_rc);
    assert_same("crypto_sign_keypair pk", &cpk2, &rpk2);
    assert_same("crypto_sign_keypair sk", &csk2, &rsk2);

    for mlen in [0usize, 1, 31, 32, 65] {
        let mut message = vec![0u8; mlen];
        rng.fill(&mut message);
        reset_drbg(libs, &mut entropy);
        let mut csig = vec![0u8; p.bytes];
        let mut rsig = vec![0u8; p.bytes];
        let (mut clen, mut rlen) = (0usize, 0usize);
        let c_rc = libs.c::<SignDetached>("crypto_sign_signature")(
            csig.as_mut_ptr(),
            &mut clen,
            message.as_ptr(),
            mlen,
            csk.as_ptr(),
        );
        let r_rc = libs.r::<SignDetached>("crypto_sign_signature")(
            rsig.as_mut_ptr(),
            &mut rlen,
            message.as_ptr(),
            mlen,
            rsk.as_ptr(),
        );
        assert_eq!((c_rc, clen), (r_rc, rlen));
        assert_same("crypto_sign_signature", &csig, &rsig);
        assert_eq!(
            libs.c::<Verify>("crypto_sign_verify")(
                csig.as_ptr(),
                clen,
                message.as_ptr(),
                mlen,
                cpk.as_ptr(),
            ),
            0
        );
        assert_eq!(
            libs.r::<Verify>("crypto_sign_verify")(
                rsig.as_ptr(),
                rlen,
                message.as_ptr(),
                mlen,
                rpk.as_ptr(),
            ),
            0
        );

        // ERRORS.md row 1: both short and oversized detached lengths.
        for bad_len in [p.bytes - 1, p.bytes + 1] {
            assert_eq!(
                libs.c::<Verify>("crypto_sign_verify")(
                    ptr::null(),
                    bad_len,
                    ptr::null(),
                    0,
                    ptr::null(),
                ),
                libs.r::<Verify>("crypto_sign_verify")(
                    ptr::null(),
                    bad_len,
                    ptr::null(),
                    0,
                    ptr::null(),
                )
            );
        }

        // ERRORS.md row 2: exact-length signature with a changed byte.
        csig[p.n] ^= 1;
        rsig[p.n] ^= 1;
        assert_eq!(
            libs.c::<Verify>("crypto_sign_verify")(
                csig.as_ptr(),
                csig.len(),
                message.as_ptr(),
                mlen,
                cpk.as_ptr(),
            ),
            libs.r::<Verify>("crypto_sign_verify")(
                rsig.as_ptr(),
                rsig.len(),
                message.as_ptr(),
                mlen,
                rpk.as_ptr(),
            )
        );
    }

    let mut message = vec![0u8; 33];
    rng.fill(&mut message);
    reset_drbg(libs, &mut entropy);
    let mut csm = vec![0u8; p.bytes + message.len()];
    let mut rsm = vec![0u8; p.bytes + message.len()];
    let (mut csmlen, mut rsmlen) = (0u64, 0u64);
    assert_eq!(
        libs.c::<Sign>("crypto_sign")(
            csm.as_mut_ptr(),
            &mut csmlen,
            message.as_ptr(),
            message.len() as u64,
            csk.as_ptr(),
        ),
        libs.r::<Sign>("crypto_sign")(
            rsm.as_mut_ptr(),
            &mut rsmlen,
            message.as_ptr(),
            message.len() as u64,
            rsk.as_ptr(),
        )
    );
    assert_eq!(csmlen, rsmlen);
    assert_same("crypto_sign", &csm, &rsm);
    let mut cm = vec![0xa5; csm.len()];
    let mut rm = vec![0xa5; rsm.len()];
    let (mut cmlen, mut rmlen) = (u64::MAX, u64::MAX);
    assert_eq!(
        libs.c::<Open>("crypto_sign_open")(
            cm.as_mut_ptr(),
            &mut cmlen,
            csm.as_ptr(),
            csmlen,
            cpk.as_ptr(),
        ),
        libs.r::<Open>("crypto_sign_open")(
            rm.as_mut_ptr(),
            &mut rmlen,
            rsm.as_ptr(),
            rsmlen,
            rpk.as_ptr(),
        )
    );
    assert_eq!(cmlen, rmlen);
    assert_same(
        "crypto_sign_open",
        &cm[..cmlen as usize],
        &rm[..rmlen as usize],
    );

    // ERRORS.md row 3: too-short signed message, including zero.
    for short in [0usize, 1, p.bytes - 1] {
        let mut co = vec![0xa5; short.max(1)];
        let mut ro = co.clone();
        let (mut cl, mut rl) = (u64::MAX, u64::MAX);
        let cr = libs.c::<Open>("crypto_sign_open")(
            co.as_mut_ptr(),
            &mut cl,
            csm.as_ptr(),
            short as u64,
            cpk.as_ptr(),
        );
        let rr = libs.r::<Open>("crypto_sign_open")(
            ro.as_mut_ptr(),
            &mut rl,
            rsm.as_ptr(),
            short as u64,
            rpk.as_ptr(),
        );
        assert_eq!((cr, cl), (rr, rl));
        assert_same("crypto_sign_open short clear", &co, &ro);
    }

    // ERRORS.md row 4: exact-size input whose nested verification fails.
    csm[0] ^= 1;
    rsm[0] ^= 1;
    let mut co = vec![0xa5; csm.len()];
    let mut ro = co.clone();
    let (mut cl, mut rl) = (u64::MAX, u64::MAX);
    let cr = libs.c::<Open>("crypto_sign_open")(
        co.as_mut_ptr(),
        &mut cl,
        csm.as_ptr(),
        csmlen,
        cpk.as_ptr(),
    );
    let rr = libs.r::<Open>("crypto_sign_open")(
        ro.as_mut_ptr(),
        &mut rl,
        rsm.as_ptr(),
        rsmlen,
        rpk.as_ptr(),
    );
    assert_eq!((cr, cl), (rr, rl));
    assert_same("crypto_sign_open invalid clear", &co, &ro);
}

unsafe fn test_rng_errors(libs: &Libraries) {
    type SeedInit = unsafe extern "C" fn(*mut c_void, *mut u8, *mut u8, u64) -> i32;
    type Seed = unsafe extern "C" fn(*mut c_void, *mut u8, u64) -> i32;
    let mut seed = [0u8; 32];
    let mut diversifier = [0u8; 8];
    let mut cctx = [0xa5u8; 80];
    let mut rctx = cctx;

    // ERRORS.md row 5.
    for maxlen in [1u64 << 32, (1u64 << 32) + 1, u64::MAX] {
        let cr = libs.c::<SeedInit>("seedexpander_init")(
            cctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            maxlen,
        );
        let rr = libs.r::<SeedInit>("seedexpander_init")(
            rctx.as_mut_ptr().cast(),
            seed.as_mut_ptr(),
            diversifier.as_mut_ptr(),
            maxlen,
        );
        assert_eq!((cr, cctx), (rr, rctx));
    }

    libs.c::<SeedInit>("seedexpander_init")(
        cctx.as_mut_ptr().cast(),
        seed.as_mut_ptr(),
        diversifier.as_mut_ptr(),
        17,
    );
    libs.r::<SeedInit>("seedexpander_init")(
        rctx.as_mut_ptr().cast(),
        seed.as_mut_ptr(),
        diversifier.as_mut_ptr(),
        17,
    );

    // ERRORS.md row 6. The null-output branch executes before the length check.
    for len in [0u64, 1, 17, u64::MAX] {
        assert_eq!(
            libs.c::<Seed>("seedexpander")(cctx.as_mut_ptr().cast(), ptr::null_mut(), len),
            libs.r::<Seed>("seedexpander")(rctx.as_mut_ptr().cast(), ptr::null_mut(), len)
        );
    }

    // ERRORS.md row 7: equality and greater-than are both rejected.
    for len in [17u64, 18, u64::MAX] {
        let mut co = [0xa5u8; 18];
        let mut ro = co;
        assert_eq!(
            libs.c::<Seed>("seedexpander")(cctx.as_mut_ptr().cast(), co.as_mut_ptr(), len,),
            libs.r::<Seed>("seedexpander")(rctx.as_mut_ptr().cast(), ro.as_mut_ptr(), len,)
        );
        assert_eq!(co, ro);
        assert_eq!(cctx, rctx);
    }
}

#[test]
fn ffi_differential_surface() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::from_library(&libs);
        let mut rng = Rng::new();

        test_symbol_and_size_surface(&libs, p);
        test_endian_and_address(&libs, &mut rng);
        match backend_name() {
            "blake" => test_blake(&libs, &mut rng),
            "sha2" => test_sha2(&libs, &mut rng),
            "shake" => test_shake(&libs, &mut rng),
            "haraka" => test_haraka(&libs, p, &mut rng),
            _ => unreachable!(),
        }
        test_hash_api(&libs, p, &mut rng);
        test_wots_and_roots(&libs, p, &mut rng);
        test_direct_tree_fors_merkle(&libs, p, &mut rng);
        test_rng_surface(&libs, &mut rng);
        test_rng_errors(&libs);
        test_signing_and_errors(&libs, p, &mut rng);
    }
}

#[test]
fn ffi_system_random_boundary_surface() {
    unsafe {
        let libs = Libraries::load();
        test_system_random_surface(&libs);
    }
}
