use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_NOW};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

const RTLD_DEEPBIND: i32 = 0x00008;

struct Libraries {
    backend: Library,
    core: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tuple = format!("{}-{}-{}", backend(), thash_variant(), parameter_set());
        let build = manifest.join("target/c-ref").join(tuple);
        let backend_path = build
            .join("lib")
            .join(backend())
            .join(format!("lib{}.so", backend()));
        let core_path = build.join("app/libsphincs_core.so");
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let rust_path = manifest
            .join("target")
            .join(profile)
            .join("libsphincs_plus.so");
        assert_exists(&backend_path);
        assert_exists(&core_path);
        assert_exists(&rust_path);
        Self {
            backend: unsafe {
                Library::open(Some(&backend_path), RTLD_LAZY | RTLD_GLOBAL).unwrap()
            },
            core: unsafe {
                Library::open(Some(&core_path), RTLD_NOW | RTLD_GLOBAL).unwrap()
            },
            rust: unsafe {
                Library::open(Some(&rust_path), RTLD_NOW | RTLD_GLOBAL).unwrap()
            },
        }
    }
}

fn assert_exists(path: &Path) {
    assert!(path.is_file(), "missing shared library: {}", path.display());
}

fn backend() -> &'static str {
    if cfg!(feature = "blake") {
        "blake"
    } else if cfg!(feature = "sha2") {
        "sha2"
    } else if cfg!(feature = "shake") {
        "shake"
    } else {
        "haraka"
    }
}

fn thash_variant() -> &'static str {
    if cfg!(feature = "robust") { "robust" } else { "simple" }
}

fn parameter_set() -> &'static str {
    if cfg!(feature = "128s") {
        "128s"
    } else if cfg!(feature = "128f") {
        "128f"
    } else if cfg!(feature = "192s") {
        "192s"
    } else if cfg!(feature = "192f") {
        "192f"
    } else if cfg!(feature = "256s") {
        "256s"
    } else {
        "256f"
    }
}

#[derive(Clone, Copy)]
struct Params {
    n: usize,
    d: usize,
    tree_height: usize,
    fors_height: usize,
    fors_trees: usize,
}

impl Params {
    fn current() -> Self {
        match parameter_set() {
            "128s" => Self { n: 16, d: 7, tree_height: 9, fors_height: 12, fors_trees: 14 },
            "128f" => Self { n: 16, d: 22, tree_height: 3, fors_height: 6, fors_trees: 33 },
            "192s" => Self { n: 24, d: 7, tree_height: 9, fors_height: 14, fors_trees: 17 },
            "192f" => Self { n: 24, d: 22, tree_height: 3, fors_height: 8, fors_trees: 33 },
            "256s" => Self { n: 32, d: 8, tree_height: 8, fors_height: 14, fors_trees: 22 },
            "256f" => Self { n: 32, d: 17, tree_height: 4, fors_height: 9, fors_trees: 35 },
            _ => unreachable!(),
        }
    }

    fn wots_len(self) -> usize { 2 * self.n + 3 }
    fn wots_bytes(self) -> usize { self.wots_len() * self.n }
    fn fors_msg_bytes(self) -> usize { (self.fors_height * self.fors_trees + 7) / 8 }
    fn fors_bytes(self) -> usize { (self.fors_height + 1) * self.fors_trees * self.n }
    fn sig_bytes(self) -> usize {
        self.n + self.fors_bytes() + self.d * self.wots_bytes()
            + self.d * self.tree_height * self.n
    }
    fn pk_bytes(self) -> usize { 2 * self.n }
    fn sk_bytes(self) -> usize { 4 * self.n }
    fn seed_bytes(self) -> usize { 3 * self.n }
    fn ctx_bytes(self) -> usize {
        let base = 2 * self.n;
        if backend() == "sha2" {
            base + 40 + if self.n >= 24 { 72 } else { 0 }
        } else if backend() == "haraka" {
            base + 10 * 8 * 8 + 10 * 8 * 4
        } else {
            base
        }
    }
}

struct Prng(u64);

impl Prng {
    fn new() -> Self { Self(0x6a09_e667_f3bc_c909) }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let word = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
    }
}

fn aligned_bytes(len: usize) -> Vec<u64> {
    vec![0; len.div_ceil(8)]
}

unsafe fn load_deterministic_core(libs: &Libraries) -> (Library, Library) {
    let _ = &libs.backend;
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tuple = format!("{}-{}-{}", backend(), thash_variant(), parameter_set());
    let det_path = manifest
        .join("target/c-ref")
        .join(tuple)
        .join("app/libsphincs_core_det.so");
    assert_exists(&det_path);
    let crypto = unsafe {
        Library::open(
            Some(Path::new("/lib64/libcrypto.so.3")),
            RTLD_NOW | RTLD_GLOBAL,
        )
        .unwrap()
    };
    let det = unsafe {
        Library::open(
            Some(&det_path),
            RTLD_NOW | RTLD_GLOBAL | RTLD_DEEPBIND,
        )
        .unwrap()
    };
    (crypto, det)
}

fn bytes(words: &[u64], len: usize) -> &[u8] {
    unsafe { std::slice::from_raw_parts(words.as_ptr().cast(), len) }
}

fn bytes_mut(words: &mut [u64], len: usize) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast(), len) }
}

#[test]
fn size_queries_match() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type F = unsafe extern "C" fn() -> u64;
        for (name, expected) in [
            (b"crypto_sign_secretkeybytes\0".as_slice(), p.sk_bytes()),
            (b"crypto_sign_publickeybytes\0".as_slice(), p.pk_bytes()),
            (b"crypto_sign_bytes\0".as_slice(), p.sig_bytes()),
            (b"crypto_sign_seedbytes\0".as_slice(), p.seed_bytes()),
        ] {
            let c: libloading::os::unix::Symbol<F> = libs.core.get(name).unwrap();
            let r: libloading::os::unix::Symbol<F> = libs.rust.get(name).unwrap();
            assert_eq!(c(), expected as u64, "{name:?} C size");
            assert_eq!(r(), c(), "{name:?} Rust/C size");
        }
    }
}

#[test]
fn integer_and_address_helpers_match_randomized() {
    unsafe {
        let libs = Libraries::load();
        type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
        type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
        type U32ToBytes = unsafe extern "C" fn(*mut u8, u32);
        let c_ull: libloading::os::unix::Symbol<UllToBytes> =
            libs.core.get(b"SPX_ull_to_bytes\0").unwrap();
        let r_ull: libloading::os::unix::Symbol<UllToBytes> =
            libs.rust.get(b"SPX_ull_to_bytes\0").unwrap();
        let c_btu: libloading::os::unix::Symbol<BytesToUll> =
            libs.core.get(b"SPX_bytes_to_ull\0").unwrap();
        let r_btu: libloading::os::unix::Symbol<BytesToUll> =
            libs.rust.get(b"SPX_bytes_to_ull\0").unwrap();
        let c_u32: libloading::os::unix::Symbol<U32ToBytes> =
            libs.core.get(b"SPX_u32_to_bytes\0").unwrap();
        let r_u32: libloading::os::unix::Symbol<U32ToBytes> =
            libs.rust.get(b"SPX_u32_to_bytes\0").unwrap();
        let mut rng = Prng::new();
        for width in [0u32, 1, 2, 4, 8] {
            for _ in 0..64 {
                let value = rng.next_u64();
                let mut c = [0xa5; 8];
                let mut r = [0xa5; 8];
                c_ull(c.as_mut_ptr(), width, value);
                r_ull(r.as_mut_ptr(), width, value);
                assert_eq!(r, c);
                assert_eq!(r_btu(r.as_ptr(), width), c_btu(c.as_ptr(), width));
            }
        }
        for _ in 0..128 {
            let value = rng.next_u64() as u32;
            let mut c = [0u8; 4];
            let mut r = [0u8; 4];
            c_u32(c.as_mut_ptr(), value);
            r_u32(r.as_mut_ptr(), value);
            assert_eq!(r, c);
        }

        type Setter32 = unsafe extern "C" fn(*mut u32, u32);
        type Setter64 = unsafe extern "C" fn(*mut u32, u64);
        for name in [
            b"SPX_set_layer_addr\0".as_slice(),
            b"SPX_set_type\0",
            b"SPX_set_keypair_addr\0",
            b"SPX_set_chain_addr\0",
            b"SPX_set_hash_addr\0",
            b"SPX_set_tree_height\0",
            b"SPX_set_tree_index\0",
        ] {
            let c: libloading::os::unix::Symbol<Setter32> = libs.core.get(name).unwrap();
            let r: libloading::os::unix::Symbol<Setter32> = libs.rust.get(name).unwrap();
            for _ in 0..64 {
                let mut ca: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
                let mut ra = ca;
                let value = rng.next_u64() as u32;
                c(ca.as_mut_ptr(), value);
                r(ra.as_mut_ptr(), value);
                assert_eq!(ra, ca, "{name:?}");
            }
        }
        let c_tree: libloading::os::unix::Symbol<Setter64> =
            libs.core.get(b"SPX_set_tree_addr\0").unwrap();
        let r_tree: libloading::os::unix::Symbol<Setter64> =
            libs.rust.get(b"SPX_set_tree_addr\0").unwrap();
        for _ in 0..64 {
            let mut ca: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
            let mut ra = ca;
            let value = rng.next_u64();
            c_tree(ca.as_mut_ptr(), value);
            r_tree(ra.as_mut_ptr(), value);
            assert_eq!(ra, ca);
        }

        type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
        for name in [
            b"SPX_copy_subtree_addr\0".as_slice(),
            b"SPX_copy_keypair_addr\0",
        ] {
            let c: libloading::os::unix::Symbol<CopyAddr> = libs.core.get(name).unwrap();
            let r: libloading::os::unix::Symbol<CopyAddr> = libs.rust.get(name).unwrap();
            for _ in 0..64 {
                let source: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
                let mut ca: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
                let mut ra = ca;
                c(ca.as_mut_ptr(), source.as_ptr());
                r(ra.as_mut_ptr(), source.as_ptr());
                assert_eq!(ra, ca, "{name:?}");
            }
        }
    }
}

#[test]
fn backend_hash_surface_matches_randomized() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type Init = unsafe extern "C" fn(*mut c_void);
        type Prf = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
        type GenRandom = unsafe extern "C" fn(
            *mut u8, *const u8, *const u8, *const u8, u64, *const c_void,
        );
        type HashMessage = unsafe extern "C" fn(
            *mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64,
            *const c_void,
        );
        type Thash =
            unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
        let ci: libloading::os::unix::Symbol<Init> =
            libs.backend.get(b"SPX_initialize_hash_function\0").unwrap();
        let ri: libloading::os::unix::Symbol<Init> =
            libs.rust.get(b"SPX_initialize_hash_function\0").unwrap();
        let cp: libloading::os::unix::Symbol<Prf> =
            libs.backend.get(b"SPX_prf_addr\0").unwrap();
        let rp: libloading::os::unix::Symbol<Prf> =
            libs.rust.get(b"SPX_prf_addr\0").unwrap();
        let cg: libloading::os::unix::Symbol<GenRandom> =
            libs.backend.get(b"SPX_gen_message_random\0").unwrap();
        let rg: libloading::os::unix::Symbol<GenRandom> =
            libs.rust.get(b"SPX_gen_message_random\0").unwrap();
        let ch: libloading::os::unix::Symbol<HashMessage> =
            libs.backend.get(b"SPX_hash_message\0").unwrap();
        let rh: libloading::os::unix::Symbol<HashMessage> =
            libs.rust.get(b"SPX_hash_message\0").unwrap();
        let ct: libloading::os::unix::Symbol<Thash> =
            libs.backend.get(b"SPX_thash\0").unwrap();
        let rt: libloading::os::unix::Symbol<Thash> =
            libs.rust.get(b"SPX_thash\0").unwrap();
        let mut rng = Prng::new();

        for case in 0..24 {
            let mut cc = aligned_bytes(p.ctx_bytes());
            let mut rc = aligned_bytes(p.ctx_bytes());
            rng.fill(&mut bytes_mut(&mut cc, p.ctx_bytes())[..2 * p.n]);
            bytes_mut(&mut rc, p.ctx_bytes())[..2 * p.n]
                .copy_from_slice(&bytes(&cc, p.ctx_bytes())[..2 * p.n]);
            ci(cc.as_mut_ptr().cast());
            ri(rc.as_mut_ptr().cast());
            assert_eq!(bytes(&rc, p.ctx_bytes()), bytes(&cc, p.ctx_bytes()));

            let mut addr: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
            let output_len = if backend() == "blake" {
                if p.n >= 24 { 64 } else { 32 }
            } else {
                p.n
            };
            let mut co = vec![0u8; output_len];
            let mut ro = vec![0u8; output_len];
            cp(co.as_mut_ptr(), cc.as_ptr().cast(), addr.as_ptr());
            rp(ro.as_mut_ptr(), rc.as_ptr().cast(), addr.as_ptr());
            assert_eq!(ro, co, "PRF case {case}");

            let mlen = [0, 1, 31, 32, 63, 64, 65, 137][case % 8];
            let mut msg = vec![0u8; mlen];
            let mut sk_prf = vec![0u8; p.n];
            let mut optrand = vec![0u8; p.n];
            rng.fill(&mut msg);
            rng.fill(&mut sk_prf);
            rng.fill(&mut optrand);
            cg(
                co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(),
                mlen as u64, cc.as_ptr().cast(),
            );
            rg(
                ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), msg.as_ptr(),
                mlen as u64, rc.as_ptr().cast(),
            );
            assert_eq!(ro, co, "gen_message_random case {case}");

            let mut pk = vec![0u8; p.pk_bytes()];
            rng.fill(&mut pk);
            let mut cd = vec![0u8; p.fors_msg_bytes()];
            let mut rd = vec![0u8; p.fors_msg_bytes()];
            let (mut ctree, mut rtree) = (0u64, 0u64);
            let (mut cleaf, mut rleaf) = (0u32, 0u32);
            ch(
                cd.as_mut_ptr(), &mut ctree, &mut cleaf, co.as_ptr(), pk.as_ptr(),
                msg.as_ptr(), mlen as u64, cc.as_ptr().cast(),
            );
            rh(
                rd.as_mut_ptr(), &mut rtree, &mut rleaf, ro.as_ptr(), pk.as_ptr(),
                msg.as_ptr(), mlen as u64, rc.as_ptr().cast(),
            );
            assert_eq!((rd, rtree, rleaf), (cd, ctree, cleaf), "hash case {case}");

            for blocks in [1usize, 2, p.wots_len(), p.fors_trees] {
                let mut input = vec![0u8; blocks * p.n];
                rng.fill(&mut input);
                let mut ca = addr;
                let mut ra = addr;
                ct(
                    co.as_mut_ptr(), input.as_ptr(), blocks as u32,
                    cc.as_ptr().cast(), ca.as_mut_ptr(),
                );
                rt(
                    ro.as_mut_ptr(), input.as_ptr(), blocks as u32,
                    rc.as_ptr().cast(), ra.as_mut_ptr(),
                );
                assert_eq!(ro, co, "thash blocks={blocks} case={case}");
                assert_eq!(ra, ca);
            }
            addr[0] ^= case as u32;
        }
    }
}

#[test]
fn wots_chain_lengths_match_randomized() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type F = unsafe extern "C" fn(*mut u32, *const u8);
        let c: libloading::os::unix::Symbol<F> =
            libs.core.get(b"SPX_chain_lengths\0").unwrap();
        let r: libloading::os::unix::Symbol<F> =
            libs.rust.get(b"SPX_chain_lengths\0").unwrap();
        let mut rng = Prng::new();
        for case in 0..128 {
            let mut message = vec![0u8; p.n];
            if case == 1 {
                message.fill(0xff);
            } else if case > 1 {
                rng.fill(&mut message);
            }
            let mut co = vec![0u32; p.wots_len()];
            let mut ro = vec![0u32; p.wots_len()];
            c(co.as_mut_ptr(), message.as_ptr());
            r(ro.as_mut_ptr(), message.as_ptr());
            assert_eq!(ro, co, "case {case}");
        }
    }
}

#[cfg(feature = "blake")]
#[test]
fn blake_one_shot_and_mgf1_match_boundaries() {
    unsafe {
        let libs = Libraries::load();
        type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
        type Mgf = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
        let mut rng = Prng::new();
        for (hash_name, mgf_name, digest, lengths) in [
            (
                b"blake256\0".as_slice(),
                b"SPX_blake256_mgf1\0".as_slice(),
                32usize,
                vec![0, 1, 55, 56, 63, 64, 65, 129],
            ),
            (
                b"blake512\0".as_slice(),
                b"SPX_blake512_mgf1\0".as_slice(),
                64usize,
                vec![0, 1, 111, 112, 127, 128, 129, 257],
            ),
        ] {
            let ch: libloading::os::unix::Symbol<Hash> = libs.backend.get(hash_name).unwrap();
            let rh: libloading::os::unix::Symbol<Hash> = libs.rust.get(hash_name).unwrap();
            let cm: libloading::os::unix::Symbol<Mgf> = libs.backend.get(mgf_name).unwrap();
            let rm: libloading::os::unix::Symbol<Mgf> = libs.rust.get(mgf_name).unwrap();
            for len in lengths {
                let mut input = vec![0u8; len];
                rng.fill(&mut input);
                let mut co = vec![0u8; digest];
                let mut ro = vec![0u8; digest];
                assert_eq!(rh(ro.as_mut_ptr(), input.as_ptr(), len as u64),
                           ch(co.as_mut_ptr(), input.as_ptr(), len as u64));
                assert_eq!(ro, co, "{hash_name:?}, len={len}");
            }
            for outlen in [0, 1, digest - 1, digest, digest + 1, 2 * digest + 7] {
                let mut input = vec![0u8; 37];
                rng.fill(&mut input);
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                cm(co.as_mut_ptr(), outlen, input.as_ptr(), input.len());
                rm(ro.as_mut_ptr(), outlen, input.as_ptr(), input.len());
                assert_eq!(ro, co, "{mgf_name:?}, outlen={outlen}");
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Blake256State {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Blake512State {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

#[cfg(feature = "blake")]
#[test]
fn blake_incremental_and_compress_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Prng::new();
        macro_rules! check_blake {
            ($state:ty, $prefix:literal, $block:expr, $digest:expr) => {{
                type Init = unsafe extern "C" fn(*mut $state);
                type Update = unsafe extern "C" fn(*mut $state, *const u8, u64);
                type Compress = unsafe extern "C" fn(*mut $state, *const u8);
                type Final = unsafe extern "C" fn(*mut $state, *mut u8);
                let ci: libloading::os::unix::Symbol<Init> =
                    libs.backend.get(concat!($prefix, "_init\0").as_bytes()).unwrap();
                let ri: libloading::os::unix::Symbol<Init> =
                    libs.rust.get(concat!($prefix, "_init\0").as_bytes()).unwrap();
                let cu: libloading::os::unix::Symbol<Update> =
                    libs.backend.get(concat!($prefix, "_update\0").as_bytes()).unwrap();
                let ru: libloading::os::unix::Symbol<Update> =
                    libs.rust.get(concat!($prefix, "_update\0").as_bytes()).unwrap();
                let cc: libloading::os::unix::Symbol<Compress> =
                    libs.backend.get(concat!($prefix, "_compress\0").as_bytes()).unwrap();
                let rc: libloading::os::unix::Symbol<Compress> =
                    libs.rust.get(concat!($prefix, "_compress\0").as_bytes()).unwrap();
                let cf: libloading::os::unix::Symbol<Final> =
                    libs.backend.get(concat!($prefix, "_final\0").as_bytes()).unwrap();
                let rf: libloading::os::unix::Symbol<Final> =
                    libs.rust.get(concat!($prefix, "_final\0").as_bytes()).unwrap();
                let mut cs: $state = std::mem::zeroed();
                let mut rs: $state = std::mem::zeroed();
                ci(&mut cs);
                ri(&mut rs);
                assert_eq!(rs, cs);
                let mut input = vec![0u8; $block + 17];
                rng.fill(&mut input);
                cu(&mut cs, input.as_ptr(), 13);
                ru(&mut rs, input.as_ptr(), 13);
                assert_eq!(rs, cs);
                cu(&mut cs, input.as_ptr().add(2), (($block + 15) * 8) as u64);
                ru(&mut rs, input.as_ptr().add(2), (($block + 15) * 8) as u64);
                assert_eq!(rs, cs);
                let mut co = vec![0u8; $digest];
                let mut ro = vec![0u8; $digest];
                cf(&mut cs, co.as_mut_ptr());
                rf(&mut rs, ro.as_mut_ptr());
                assert_eq!((ro, rs), (co, cs));

                let mut block = vec![0u8; $block];
                rng.fill(&mut block);
                let mut cs: $state = std::mem::zeroed();
                let mut rs: $state = std::mem::zeroed();
                ci(&mut cs);
                ri(&mut rs);
                cc(&mut cs, block.as_ptr());
                rc(&mut rs, block.as_ptr());
                assert_eq!(rs, cs);
            }};
        }
        check_blake!(Blake256State, "blake256", 64usize, 32usize);
        check_blake!(Blake512State, "blake512", 128usize, 64usize);
    }
}

#[cfg(feature = "sha2")]
#[test]
fn sha2_primitive_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Prng::new();
        macro_rules! check_sha {
            ($prefix:literal, $state_len:expr, $block:expr, $digest:expr) => {{
                type One = unsafe extern "C" fn(*mut u8, *const u8, usize);
                type Init = unsafe extern "C" fn(*mut u8);
                type Blocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
                type Final = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
                let ch: libloading::os::unix::Symbol<One> =
                    libs.backend.get(concat!($prefix, "\0").as_bytes()).unwrap();
                let rh: libloading::os::unix::Symbol<One> =
                    libs.rust.get(concat!($prefix, "\0").as_bytes()).unwrap();
                let ci: libloading::os::unix::Symbol<Init> =
                    libs.backend.get(concat!($prefix, "_inc_init\0").as_bytes()).unwrap();
                let ri: libloading::os::unix::Symbol<Init> =
                    libs.rust.get(concat!($prefix, "_inc_init\0").as_bytes()).unwrap();
                let cb: libloading::os::unix::Symbol<Blocks> =
                    libs.backend.get(concat!($prefix, "_inc_blocks\0").as_bytes()).unwrap();
                let rb: libloading::os::unix::Symbol<Blocks> =
                    libs.rust.get(concat!($prefix, "_inc_blocks\0").as_bytes()).unwrap();
                let cf: libloading::os::unix::Symbol<Final> =
                    libs.backend.get(concat!($prefix, "_inc_finalize\0").as_bytes()).unwrap();
                let rf: libloading::os::unix::Symbol<Final> =
                    libs.rust.get(concat!($prefix, "_inc_finalize\0").as_bytes()).unwrap();
                for len in [0usize, 1, $block - 1, $block, $block + 1, 2 * $block + 7] {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let mut co = vec![0u8; $digest];
                    let mut ro = vec![0u8; $digest];
                    ch(co.as_mut_ptr(), input.as_ptr(), len);
                    rh(ro.as_mut_ptr(), input.as_ptr(), len);
                    assert_eq!(ro, co, "{} len={len}", $prefix);
                }
                let mut input = vec![0u8; $block + 19];
                rng.fill(&mut input);
                let mut cs = vec![0u8; $state_len];
                let mut rs = vec![0u8; $state_len];
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                cb(cs.as_mut_ptr(), input.as_ptr(), 1);
                rb(rs.as_mut_ptr(), input.as_ptr(), 1);
                assert_eq!(rs, cs);
                let mut co = vec![0u8; $digest];
                let mut ro = vec![0u8; $digest];
                cf(co.as_mut_ptr(), cs.as_mut_ptr(), input.as_ptr().add($block), 19);
                rf(ro.as_mut_ptr(), rs.as_mut_ptr(), input.as_ptr().add($block), 19);
                assert_eq!(ro, co);
            }};
        }
        check_sha!("sha256", 40usize, 64usize, 32usize);
        check_sha!("sha512", 72usize, 128usize, 64usize);
        type Mgf = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
        for (name, digest) in [
            (b"SPX_mgf1_256\0".as_slice(), 32usize),
            (b"SPX_mgf1_512\0".as_slice(), 64usize),
        ] {
            let c: libloading::os::unix::Symbol<Mgf> = libs.backend.get(name).unwrap();
            let r: libloading::os::unix::Symbol<Mgf> = libs.rust.get(name).unwrap();
            let mut input = [0u8; 41];
            rng.fill(&mut input);
            for len in [0, 1, digest - 1, digest, digest + 1, 2 * digest + 3] {
                let mut co = vec![0u8; len];
                let mut ro = vec![0u8; len];
                c(co.as_mut_ptr(), len, input.as_ptr(), input.len());
                r(ro.as_mut_ptr(), len, input.as_ptr(), input.len());
                assert_eq!(ro, co, "{name:?} len={len}");
            }
        }
    }
}

#[cfg(feature = "shake")]
#[test]
fn shake_primitive_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Prng::new();
        type One = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
        let ch: libloading::os::unix::Symbol<One> =
            libs.backend.get(b"shake256\0").unwrap();
        let rh: libloading::os::unix::Symbol<One> =
            libs.rust.get(b"shake256\0").unwrap();
        for len in [0usize, 1, 135, 136, 137, 281] {
            let mut input = vec![0u8; len];
            rng.fill(&mut input);
            for outlen in [0usize, 1, 135, 136, 137, 289] {
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                ch(co.as_mut_ptr(), outlen, input.as_ptr(), len);
                rh(ro.as_mut_ptr(), outlen, input.as_ptr(), len);
                assert_eq!(ro, co, "in={len} out={outlen}");
            }
        }
        type Init = unsafe extern "C" fn(*mut u64);
        type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
        type Finalize = unsafe extern "C" fn(*mut u64);
        type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
        let ci: libloading::os::unix::Symbol<Init> =
            libs.backend.get(b"shake256_inc_init\0").unwrap();
        let ri: libloading::os::unix::Symbol<Init> =
            libs.rust.get(b"shake256_inc_init\0").unwrap();
        let ca: libloading::os::unix::Symbol<Absorb> =
            libs.backend.get(b"shake256_inc_absorb\0").unwrap();
        let ra: libloading::os::unix::Symbol<Absorb> =
            libs.rust.get(b"shake256_inc_absorb\0").unwrap();
        let cf: libloading::os::unix::Symbol<Finalize> =
            libs.backend.get(b"shake256_inc_finalize\0").unwrap();
        let rf: libloading::os::unix::Symbol<Finalize> =
            libs.rust.get(b"shake256_inc_finalize\0").unwrap();
        let cs: libloading::os::unix::Symbol<Squeeze> =
            libs.backend.get(b"shake256_inc_squeeze\0").unwrap();
        let rs: libloading::os::unix::Symbol<Squeeze> =
            libs.rust.get(b"shake256_inc_squeeze\0").unwrap();
        let mut input = vec![0u8; 291];
        rng.fill(&mut input);
        let mut cstate = [0u64; 26];
        let mut rstate = [0u64; 26];
        ci(cstate.as_mut_ptr());
        ri(rstate.as_mut_ptr());
        for (offset, len) in [(0, 1), (1, 135), (136, 155)] {
            ca(cstate.as_mut_ptr(), input.as_ptr().add(offset), len);
            ra(rstate.as_mut_ptr(), input.as_ptr().add(offset), len);
            assert_eq!(rstate, cstate);
        }
        cf(cstate.as_mut_ptr());
        rf(rstate.as_mut_ptr());
        let mut co = [0u8; 277];
        let mut ro = [0u8; 277];
        cs(co.as_mut_ptr(), 17, cstate.as_mut_ptr());
        rs(ro.as_mut_ptr(), 17, rstate.as_mut_ptr());
        cs(co.as_mut_ptr().add(17), 260, cstate.as_mut_ptr());
        rs(ro.as_mut_ptr().add(17), 260, rstate.as_mut_ptr());
        assert_eq!((ro, rstate), (co, cstate));

        type SqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
        let cab: libloading::os::unix::Symbol<Absorb> =
            libs.backend.get(b"shake256_absorb\0").unwrap();
        let rab: libloading::os::unix::Symbol<Absorb> =
            libs.rust.get(b"shake256_absorb\0").unwrap();
        let csb: libloading::os::unix::Symbol<SqueezeBlocks> =
            libs.backend.get(b"shake256_squeezeblocks\0").unwrap();
        let rsb: libloading::os::unix::Symbol<SqueezeBlocks> =
            libs.rust.get(b"shake256_squeezeblocks\0").unwrap();
        let mut cstate = [0u64; 25];
        let mut rstate = [0u64; 25];
        cab(cstate.as_mut_ptr(), input.as_ptr(), input.len());
        rab(rstate.as_mut_ptr(), input.as_ptr(), input.len());
        let mut co = [0u8; 272];
        let mut ro = [0u8; 272];
        csb(co.as_mut_ptr(), 2, cstate.as_mut_ptr());
        rsb(ro.as_mut_ptr(), 2, rstate.as_mut_ptr());
        assert_eq!((ro, rstate), (co, cstate));
    }
}

#[cfg(feature = "haraka")]
#[test]
fn haraka_primitive_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        let mut rng = Prng::new();
        let mut cc = aligned_bytes(p.ctx_bytes());
        let mut rc = aligned_bytes(p.ctx_bytes());
        rng.fill(&mut bytes_mut(&mut cc, p.ctx_bytes())[..2 * p.n]);
        bytes_mut(&mut rc, p.ctx_bytes())[..2 * p.n]
            .copy_from_slice(&bytes(&cc, p.ctx_bytes())[..2 * p.n]);
        type Tweak = unsafe extern "C" fn(*mut c_void);
        let ct: libloading::os::unix::Symbol<Tweak> =
            libs.backend.get(b"SPX_tweak_constants\0").unwrap();
        let rt: libloading::os::unix::Symbol<Tweak> =
            libs.rust.get(b"SPX_tweak_constants\0").unwrap();
        ct(cc.as_mut_ptr().cast());
        rt(rc.as_mut_ptr().cast());
        assert_eq!(bytes(&rc, p.ctx_bytes()), bytes(&cc, p.ctx_bytes()));

        type Fixed = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
        for (name, input_len, output_len) in [
            (b"SPX_haraka256\0".as_slice(), 32usize, 32usize),
            (b"SPX_haraka512\0".as_slice(), 64usize, 32usize),
            (b"SPX_haraka512_perm\0".as_slice(), 64usize, 64usize),
        ] {
            let c: libloading::os::unix::Symbol<Fixed> = libs.backend.get(name).unwrap();
            let r: libloading::os::unix::Symbol<Fixed> = libs.rust.get(name).unwrap();
            for _ in 0..32 {
                let mut input = vec![0u8; input_len];
                rng.fill(&mut input);
                let mut co = vec![0u8; output_len];
                let mut ro = vec![0u8; output_len];
                c(co.as_mut_ptr(), input.as_ptr(), cc.as_ptr().cast());
                r(ro.as_mut_ptr(), input.as_ptr(), rc.as_ptr().cast());
                assert_eq!(ro, co, "{name:?}");
            }
        }

        type Sponge =
            unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
        let cs: libloading::os::unix::Symbol<Sponge> =
            libs.backend.get(b"SPX_haraka_S\0").unwrap();
        let rs: libloading::os::unix::Symbol<Sponge> =
            libs.rust.get(b"SPX_haraka_S\0").unwrap();
        for inlen in [0usize, 1, 31, 32, 33, 97] {
            let mut input = vec![0u8; inlen];
            rng.fill(&mut input);
            for outlen in [0usize, 1, 31, 32, 33, 99] {
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                cs(co.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64,
                   cc.as_ptr().cast());
                rs(ro.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64,
                   rc.as_ptr().cast());
                assert_eq!(ro, co, "in={inlen} out={outlen}");
            }
        }

        type Init = unsafe extern "C" fn(*mut u8);
        type Absorb =
            unsafe extern "C" fn(*mut u8, *const u8, usize, *const c_void);
        type Finalize = unsafe extern "C" fn(*mut u8);
        type Squeeze =
            unsafe extern "C" fn(*mut u8, usize, *mut u8, *const c_void);
        let ci: libloading::os::unix::Symbol<Init> =
            libs.backend.get(b"SPX_haraka_S_inc_init\0").unwrap();
        let ri: libloading::os::unix::Symbol<Init> =
            libs.rust.get(b"SPX_haraka_S_inc_init\0").unwrap();
        let ca: libloading::os::unix::Symbol<Absorb> =
            libs.backend.get(b"SPX_haraka_S_inc_absorb\0").unwrap();
        let ra: libloading::os::unix::Symbol<Absorb> =
            libs.rust.get(b"SPX_haraka_S_inc_absorb\0").unwrap();
        let cf: libloading::os::unix::Symbol<Finalize> =
            libs.backend.get(b"SPX_haraka_S_inc_finalize\0").unwrap();
        let rf: libloading::os::unix::Symbol<Finalize> =
            libs.rust.get(b"SPX_haraka_S_inc_finalize\0").unwrap();
        let csq: libloading::os::unix::Symbol<Squeeze> =
            libs.backend.get(b"SPX_haraka_S_inc_squeeze\0").unwrap();
        let rsq: libloading::os::unix::Symbol<Squeeze> =
            libs.rust.get(b"SPX_haraka_S_inc_squeeze\0").unwrap();
        let mut input = [0u8; 101];
        rng.fill(&mut input);
        let mut cstate = [0u8; 65];
        let mut rstate = [0u8; 65];
        ci(cstate.as_mut_ptr());
        ri(rstate.as_mut_ptr());
        for (offset, len) in [(0, 1), (1, 31), (32, 69)] {
            ca(cstate.as_mut_ptr(), input.as_ptr().add(offset), len, cc.as_ptr().cast());
            ra(rstate.as_mut_ptr(), input.as_ptr().add(offset), len, rc.as_ptr().cast());
            assert_eq!(rstate, cstate);
        }
        cf(cstate.as_mut_ptr());
        rf(rstate.as_mut_ptr());
        let mut co = [0u8; 103];
        let mut ro = [0u8; 103];
        csq(co.as_mut_ptr(), 17, cstate.as_mut_ptr(), cc.as_ptr().cast());
        rsq(ro.as_mut_ptr(), 17, rstate.as_mut_ptr(), rc.as_ptr().cast());
        csq(co.as_mut_ptr().add(17), 86, cstate.as_mut_ptr(), cc.as_ptr().cast());
        rsq(ro.as_mut_ptr().add(17), 86, rstate.as_mut_ptr(), rc.as_ptr().cast());
        assert_eq!((ro, rstate), (co, cstate));
    }
}

#[test]
fn seeded_keypair_matches() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
        let c: libloading::os::unix::Symbol<F> =
            libs.core.get(b"crypto_sign_seed_keypair\0").unwrap();
        let r: libloading::os::unix::Symbol<F> =
            libs.rust.get(b"crypto_sign_seed_keypair\0").unwrap();
        let mut rng = Prng::new();
        for case in 0..2 {
            let mut seed = vec![0u8; p.seed_bytes()];
            rng.fill(&mut seed);
            seed[0] ^= case;
            let mut cpk = vec![0u8; p.pk_bytes()];
            let mut rpk = vec![0u8; p.pk_bytes()];
            let mut csk = vec![0u8; p.sk_bytes()];
            let mut rsk = vec![0u8; p.sk_bytes()];
            assert_eq!(r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
                       c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()));
            assert_eq!(rpk, cpk, "public key case {case}");
            assert_eq!(rsk, csk, "secret key case {case}");
        }
    }
}

#[test]
fn explicit_signature_error_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type Verify =
            unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
        type Open =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
        let cv: libloading::os::unix::Symbol<Verify> =
            libs.core.get(b"crypto_sign_verify\0").unwrap();
        let rv: libloading::os::unix::Symbol<Verify> =
            libs.rust.get(b"crypto_sign_verify\0").unwrap();
        let co: libloading::os::unix::Symbol<Open> =
            libs.core.get(b"crypto_sign_open\0").unwrap();
        let ro: libloading::os::unix::Symbol<Open> =
            libs.rust.get(b"crypto_sign_open\0").unwrap();
        let sig = vec![0u8; p.sig_bytes() + 1];
        let pk = vec![0u8; p.pk_bytes()];
        let message = [0x5au8; 3];
        for len in [0, p.sig_bytes() - 1, p.sig_bytes() + 1] {
            assert_eq!(
                rv(sig.as_ptr(), len, message.as_ptr(), message.len(), pk.as_ptr()),
                cv(sig.as_ptr(), len, message.as_ptr(), message.len(), pk.as_ptr()),
                "siglen={len}",
            );
        }
        assert_eq!(
            rv(sig.as_ptr(), p.sig_bytes(), message.as_ptr(), message.len(), pk.as_ptr()),
            cv(sig.as_ptr(), p.sig_bytes(), message.as_ptr(), message.len(), pk.as_ptr()),
            "full-length signature with mismatching reconstructed root",
        );
        for smlen in [0usize, 1, p.sig_bytes() - 1] {
            let signed = vec![0x33u8; smlen.max(1)];
            let mut cm = vec![0xa5u8; smlen + 8];
            let mut rm = cm.clone();
            let (mut cmlen, mut rmlen) = (u64::MAX, u64::MAX);
            let cr = co(
                cm.as_mut_ptr(), &mut cmlen, signed.as_ptr(), smlen as u64, pk.as_ptr(),
            );
            let rr = ro(
                rm.as_mut_ptr(), &mut rmlen, signed.as_ptr(), smlen as u64, pk.as_ptr(),
            );
            assert_eq!((rr, rmlen), (cr, cmlen), "smlen={smlen}");
            assert_eq!(rm, cm, "zeroing smlen={smlen}");
        }
        let smlen = p.sig_bytes() + message.len();
        let signed = vec![0x33u8; smlen];
        let mut cm = vec![0xa5u8; smlen + 8];
        let mut rm = cm.clone();
        let (mut cmlen, mut rmlen) = (u64::MAX, u64::MAX);
        let cr = co(cm.as_mut_ptr(), &mut cmlen, signed.as_ptr(), smlen as u64, pk.as_ptr());
        let rr = ro(rm.as_mut_ptr(), &mut rmlen, signed.as_ptr(), smlen as u64, pk.as_ptr());
        assert_eq!((rr, rmlen, rm), (cr, cmlen, cm), "embedded verification failure");
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct AesXof {
    buffer: [u8; 16],
    buffer_pos: usize,
    length_remaining: usize,
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

#[test]
fn deterministic_rng_signing_and_rng_errors_match() {
    unsafe {
        let libs = Libraries::load();
        let (_crypto, det) = load_deterministic_core(&libs);
        let p = Params::current();
        type Init = unsafe extern "C" fn(*mut u8, *mut u8);
        type Random = unsafe extern "C" fn(*mut u8, u64) -> i32;
        type Aes = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        type Update = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
        type ExpInit =
            unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, usize) -> i32;
        type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, usize) -> i32;
        let ci: libloading::os::unix::Symbol<Init> =
            det.get(b"randombytes_init\0").unwrap();
        let ri: libloading::os::unix::Symbol<Init> =
            libs.rust.get(b"randombytes_init\0").unwrap();
        let cr: libloading::os::unix::Symbol<Random> =
            det.get(b"randombytes\0").unwrap();
        let rr: libloading::os::unix::Symbol<Random> =
            libs.rust.get(b"randombytes\0").unwrap();
        let ca: libloading::os::unix::Symbol<Aes> = det.get(b"AES256_ECB\0").unwrap();
        let ra: libloading::os::unix::Symbol<Aes> =
            libs.rust.get(b"AES256_ECB\0").unwrap();
        let cu: libloading::os::unix::Symbol<Update> =
            det.get(b"AES256_CTR_DRBG_Update\0").unwrap();
        let ru: libloading::os::unix::Symbol<Update> =
            libs.rust.get(b"AES256_CTR_DRBG_Update\0").unwrap();
        let cei: libloading::os::unix::Symbol<ExpInit> =
            det.get(b"seedexpander_init\0").unwrap();
        let rei: libloading::os::unix::Symbol<ExpInit> =
            libs.rust.get(b"seedexpander_init\0").unwrap();
        let ce: libloading::os::unix::Symbol<Exp> =
            det.get(b"seedexpander\0").unwrap();
        let re: libloading::os::unix::Symbol<Exp> =
            libs.rust.get(b"seedexpander\0").unwrap();
        let mut rng = Prng::new();
        let mut entropy = [0u8; 48];
        let mut personalization = [0u8; 48];
        rng.fill(&mut entropy);
        rng.fill(&mut personalization);

        for personal in [false, true] {
            let ptr = if personal {
                personalization.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            };
            ci(entropy.as_mut_ptr(), ptr);
            let mut co = vec![0u8; 81];
            assert_eq!(cr(co.as_mut_ptr(), co.len() as u64), 0);
            ri(entropy.as_mut_ptr(), ptr);
            let mut ro = vec![0u8; 81];
            assert_eq!(rr(ro.as_mut_ptr(), ro.len() as u64), 0);
            assert_eq!(ro, co, "personalization={personal}");
        }

        for _ in 0..32 {
            let mut key = [0u8; 32];
            let mut ctr = [0u8; 16];
            rng.fill(&mut key);
            rng.fill(&mut ctr);
            let mut co = [0u8; 16];
            let mut ro = [0u8; 16];
            ca(key.as_mut_ptr(), ctr.as_mut_ptr(), co.as_mut_ptr());
            ra(key.as_mut_ptr(), ctr.as_mut_ptr(), ro.as_mut_ptr());
            assert_eq!(ro, co);

            let mut ck = key;
            let mut rk = key;
            let mut cv = ctr;
            let mut rv = ctr;
            let mut provided = [0u8; 48];
            rng.fill(&mut provided);
            cu(provided.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
            ru(provided.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
            assert_eq!((rk, rv), (ck, cv));
        }

        let mut seed = [0u8; 32];
        let mut diversifier = [0u8; 8];
        rng.fill(&mut seed);
        rng.fill(&mut diversifier);
        let mut cx = AesXof::default();
        let mut rx = AesXof::default();
        assert_eq!(
            rei(&mut rx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 100),
            cei(&mut cx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 100),
        );
        assert_eq!(rx, cx);
        let mut co = [0u8; 17];
        let mut ro = [0u8; 17];
        assert_eq!(re(&mut rx, ro.as_mut_ptr(), ro.len()),
                   ce(&mut cx, co.as_mut_ptr(), co.len()));
        assert_eq!((ro, &rx), (co, &cx));
        assert_eq!(rei(&mut rx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 1usize << 32),
                   cei(&mut cx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 1usize << 32));
        assert_eq!(re(&mut rx, std::ptr::null_mut(), 1),
                   ce(&mut cx, std::ptr::null_mut(), 1));
        assert_eq!(rei(&mut rx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 9),
                   cei(&mut cx, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 9));
        assert_eq!(re(&mut rx, ro.as_mut_ptr(), 9), ce(&mut cx, co.as_mut_ptr(), 9));

        type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
        type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
        type Signature = unsafe extern "C" fn(
            *mut u8, *mut usize, *const u8, usize, *const u8,
        ) -> i32;
        type Verify =
            unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
        type Sign =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
        type Open =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
        let ckp: libloading::os::unix::Symbol<SeedKeypair> =
            det.get(b"crypto_sign_seed_keypair\0").unwrap();
        let rkp: libloading::os::unix::Symbol<SeedKeypair> =
            libs.rust.get(b"crypto_sign_seed_keypair\0").unwrap();
        let ckg: libloading::os::unix::Symbol<Keypair> =
            det.get(b"crypto_sign_keypair\0").unwrap();
        let rkg: libloading::os::unix::Symbol<Keypair> =
            libs.rust.get(b"crypto_sign_keypair\0").unwrap();
        let cs: libloading::os::unix::Symbol<Signature> =
            det.get(b"crypto_sign_signature\0").unwrap();
        let rs: libloading::os::unix::Symbol<Signature> =
            libs.rust.get(b"crypto_sign_signature\0").unwrap();
        let cv: libloading::os::unix::Symbol<Verify> =
            det.get(b"crypto_sign_verify\0").unwrap();
        let rv: libloading::os::unix::Symbol<Verify> =
            libs.rust.get(b"crypto_sign_verify\0").unwrap();
        let csign: libloading::os::unix::Symbol<Sign> =
            det.get(b"crypto_sign\0").unwrap();
        let rsign: libloading::os::unix::Symbol<Sign> =
            libs.rust.get(b"crypto_sign\0").unwrap();
        let copen: libloading::os::unix::Symbol<Open> =
            det.get(b"crypto_sign_open\0").unwrap();
        let ropen: libloading::os::unix::Symbol<Open> =
            libs.rust.get(b"crypto_sign_open\0").unwrap();
        ci(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut cgenerated_pk = vec![0u8; p.pk_bytes()];
        let mut cgenerated_sk = vec![0u8; p.sk_bytes()];
        assert_eq!(ckg(cgenerated_pk.as_mut_ptr(), cgenerated_sk.as_mut_ptr()), 0);
        ri(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut rgenerated_pk = vec![0u8; p.pk_bytes()];
        let mut rgenerated_sk = vec![0u8; p.sk_bytes()];
        assert_eq!(rkg(rgenerated_pk.as_mut_ptr(), rgenerated_sk.as_mut_ptr()), 0);
        assert_eq!((rgenerated_pk, rgenerated_sk), (cgenerated_pk, cgenerated_sk));
        let mut key_seed = vec![0u8; p.seed_bytes()];
        rng.fill(&mut key_seed);
        let mut cpk = vec![0u8; p.pk_bytes()];
        let mut rpk = vec![0u8; p.pk_bytes()];
        let mut csk = vec![0u8; p.sk_bytes()];
        let mut rsk = vec![0u8; p.sk_bytes()];
        assert_eq!(rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), key_seed.as_ptr()),
                   ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), key_seed.as_ptr()));
        assert_eq!((&rpk, &rsk), (&cpk, &csk));
        let mut message = vec![0u8; 65];
        rng.fill(&mut message);
        ci(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut csig = vec![0u8; p.sig_bytes()];
        let mut csiglen = 0usize;
        assert_eq!(cs(csig.as_mut_ptr(), &mut csiglen, message.as_ptr(), message.len(), csk.as_ptr()), 0);
        ri(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut rsig = vec![0u8; p.sig_bytes()];
        let mut rsiglen = 0usize;
        assert_eq!(rs(rsig.as_mut_ptr(), &mut rsiglen, message.as_ptr(), message.len(), rsk.as_ptr()), 0);
        assert_eq!((rsiglen, &rsig), (csiglen, &csig));
        assert_eq!(rv(rsig.as_ptr(), rsiglen, message.as_ptr(), message.len(), rpk.as_ptr()),
                   cv(csig.as_ptr(), csiglen, message.as_ptr(), message.len(), cpk.as_ptr()));

        ci(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut csm = vec![0u8; p.sig_bytes() + message.len()];
        csm[p.sig_bytes()..].copy_from_slice(&message);
        let mut csmlen = 0u64;
        assert_eq!(csign(csm.as_mut_ptr(), &mut csmlen, csm.as_ptr().add(p.sig_bytes()),
                         message.len() as u64, csk.as_ptr()), 0);
        ri(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let mut rsm = vec![0u8; p.sig_bytes() + message.len()];
        rsm[p.sig_bytes()..].copy_from_slice(&message);
        let mut rsmlen = 0u64;
        assert_eq!(rsign(rsm.as_mut_ptr(), &mut rsmlen, rsm.as_ptr().add(p.sig_bytes()),
                         message.len() as u64, rsk.as_ptr()), 0);
        assert_eq!((rsmlen, &rsm), (csmlen, &csm));
        let mut cm = vec![0xa5u8; csm.len()];
        let mut rm = vec![0xa5u8; rsm.len()];
        let (mut cmlen, mut rmlen) = (0u64, 0u64);
        assert_eq!(ropen(rm.as_mut_ptr(), &mut rmlen, rsm.as_ptr(), rsmlen, rpk.as_ptr()),
                   copen(cm.as_mut_ptr(), &mut cmlen, csm.as_ptr(), csmlen, cpk.as_ptr()));
        assert_eq!((rmlen, &rm[..rmlen as usize]), (cmlen, &cm[..cmlen as usize]));
        assert_eq!(&rm[..rmlen as usize], message);
    }
}

#[repr(C)]
#[derive(Clone)]
struct LeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *mut u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

#[test]
fn direct_wots_fors_and_merkle_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let p = Params::current();
        type Init = unsafe extern "C" fn(*mut c_void);
        let ci: libloading::os::unix::Symbol<Init> =
            libs.backend.get(b"SPX_initialize_hash_function\0").unwrap();
        let ri: libloading::os::unix::Symbol<Init> =
            libs.rust.get(b"SPX_initialize_hash_function\0").unwrap();
        let mut rng = Prng::new();
        let mut cc = aligned_bytes(p.ctx_bytes());
        let mut rc = aligned_bytes(p.ctx_bytes());
        rng.fill(&mut bytes_mut(&mut cc, p.ctx_bytes())[..2 * p.n]);
        bytes_mut(&mut rc, p.ctx_bytes())[..2 * p.n]
            .copy_from_slice(&bytes(&cc, p.ctx_bytes())[..2 * p.n]);
        ci(cc.as_mut_ptr().cast());
        ri(rc.as_mut_ptr().cast());

        type Wots = unsafe extern "C" fn(
            *mut u8, *const u8, *const u8, *const c_void, *mut u32,
        );
        let cw: libloading::os::unix::Symbol<Wots> =
            libs.core.get(b"SPX_wots_pk_from_sig\0").unwrap();
        let rw: libloading::os::unix::Symbol<Wots> =
            libs.rust.get(b"SPX_wots_pk_from_sig\0").unwrap();
        let mut sig = vec![0u8; p.wots_bytes()];
        let mut message = vec![0u8; p.n];
        rng.fill(&mut sig);
        rng.fill(&mut message);
        let addr: [u32; 8] = std::array::from_fn(|_| rng.next_u64() as u32);
        let mut ca = addr;
        let mut ra = addr;
        let mut cpk = vec![0u8; p.wots_bytes()];
        let mut rpk = vec![0u8; p.wots_bytes()];
        cw(cpk.as_mut_ptr(), sig.as_ptr(), message.as_ptr(), cc.as_ptr().cast(), ca.as_mut_ptr());
        rw(rpk.as_mut_ptr(), sig.as_ptr(), message.as_ptr(), rc.as_ptr().cast(), ra.as_mut_ptr());
        assert_eq!((rpk, ra), (cpk, ca));

        type Root = unsafe extern "C" fn(
            *mut u8, *const u8, u32, u32, *const u8, u32, *const c_void, *mut u32,
        );
        let croot: libloading::os::unix::Symbol<Root> =
            libs.core.get(b"SPX_compute_root\0").unwrap();
        let rroot: libloading::os::unix::Symbol<Root> =
            libs.rust.get(b"SPX_compute_root\0").unwrap();
        for leaf_idx in [0u32, 1, (1u32 << p.tree_height) - 1] {
            let mut leaf = vec![0u8; p.n];
            let mut auth = vec![0u8; p.tree_height * p.n];
            rng.fill(&mut leaf);
            rng.fill(&mut auth);
            let mut co = vec![0u8; p.n];
            let mut ro = vec![0u8; p.n];
            let mut ca = addr;
            let mut ra = addr;
            croot(co.as_mut_ptr(), leaf.as_ptr(), leaf_idx, 7, auth.as_ptr(),
                  p.tree_height as u32, cc.as_ptr().cast(), ca.as_mut_ptr());
            rroot(ro.as_mut_ptr(), leaf.as_ptr(), leaf_idx, 7, auth.as_ptr(),
                  p.tree_height as u32, rc.as_ptr().cast(), ra.as_mut_ptr());
            assert_eq!((ro, ra), (co, ca), "leaf_idx={leaf_idx}");
        }

        type ForsSign =
            unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32);
        type ForsPk =
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32);
        let cfs: libloading::os::unix::Symbol<ForsSign> =
            libs.core.get(b"SPX_fors_sign\0").unwrap();
        let rfs: libloading::os::unix::Symbol<ForsSign> =
            libs.rust.get(b"SPX_fors_sign\0").unwrap();
        let cfp: libloading::os::unix::Symbol<ForsPk> =
            libs.core.get(b"SPX_fors_pk_from_sig\0").unwrap();
        let rfp: libloading::os::unix::Symbol<ForsPk> =
            libs.rust.get(b"SPX_fors_pk_from_sig\0").unwrap();
        let mut digest = vec![0u8; p.fors_msg_bytes()];
        rng.fill(&mut digest);
        let mut cfsig = vec![0u8; p.fors_bytes()];
        let mut rfsig = vec![0u8; p.fors_bytes()];
        let mut cfpk = vec![0u8; p.n];
        let mut rfpk = vec![0u8; p.n];
        cfs(cfsig.as_mut_ptr(), cfpk.as_mut_ptr(), digest.as_ptr(), cc.as_ptr().cast(), addr.as_ptr());
        rfs(rfsig.as_mut_ptr(), rfpk.as_mut_ptr(), digest.as_ptr(), rc.as_ptr().cast(), addr.as_ptr());
        assert_eq!((&rfsig, &rfpk), (&cfsig, &cfpk));
        let mut cderived = vec![0u8; p.n];
        let mut rderived = vec![0u8; p.n];
        cfp(cderived.as_mut_ptr(), cfsig.as_ptr(), digest.as_ptr(), cc.as_ptr().cast(), addr.as_ptr());
        rfp(rderived.as_mut_ptr(), rfsig.as_ptr(), digest.as_ptr(), rc.as_ptr().cast(), addr.as_ptr());
        assert_eq!((rderived, rfpk), (cderived, cfpk));

        type MerkleSign = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, *mut u32, *mut u32, u32,
        );
        type MerkleRoot = unsafe extern "C" fn(*mut u8, *const c_void);
        let cms: libloading::os::unix::Symbol<MerkleSign> =
            libs.core.get(b"SPX_merkle_sign\0").unwrap();
        let rms: libloading::os::unix::Symbol<MerkleSign> =
            libs.rust.get(b"SPX_merkle_sign\0").unwrap();
        let cmr: libloading::os::unix::Symbol<MerkleRoot> =
            libs.core.get(b"SPX_merkle_gen_root\0").unwrap();
        let rmr: libloading::os::unix::Symbol<MerkleRoot> =
            libs.rust.get(b"SPX_merkle_gen_root\0").unwrap();
        let merkle_len = p.wots_bytes() + p.tree_height * p.n;
        let mut cmsig = vec![0u8; merkle_len];
        let mut rmsig = vec![0u8; merkle_len];
        let mut cmroot = vec![0u8; p.n];
        let mut rmroot = vec![0u8; p.n];
        let mut cwa = addr;
        let mut rwa = addr;
        let mut cta = addr;
        let mut rta = addr;
        let leaf_idx = (1u32 << p.tree_height) / 2;
        cms(cmsig.as_mut_ptr(), cmroot.as_mut_ptr(), cc.as_ptr().cast(),
            cwa.as_mut_ptr(), cta.as_mut_ptr(), leaf_idx);
        rms(rmsig.as_mut_ptr(), rmroot.as_mut_ptr(), rc.as_ptr().cast(),
            rwa.as_mut_ptr(), rta.as_mut_ptr(), leaf_idx);
        assert_eq!((rmsig, rmroot, rwa, rta), (cmsig, cmroot, cwa, cta));
        let mut ctop = vec![0u8; p.n];
        let mut rtop = vec![0u8; p.n];
        cmr(ctop.as_mut_ptr(), cc.as_ptr().cast());
        rmr(rtop.as_mut_ptr(), rc.as_ptr().cast());
        assert_eq!(rtop, ctop);

        type WotsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfoX1);
        let cwl: libloading::os::unix::Symbol<WotsLeaf> =
            libs.core.get(b"SPX_wots_gen_leafx1\0").unwrap();
        let rwl: libloading::os::unix::Symbol<WotsLeaf> =
            libs.rust.get(b"SPX_wots_gen_leafx1\0").unwrap();
        let mut steps = vec![0u32; p.wots_len()];
        for step in &mut steps { *step = (rng.next_u64() % 16) as u32; }
        let mut cwsig = vec![0u8; p.wots_bytes()];
        let mut rwsig = vec![0u8; p.wots_bytes()];
        let mut ci = LeafInfoX1 {
            wots_sig: cwsig.as_mut_ptr(), wots_sign_leaf: 3,
            wots_steps: steps.as_mut_ptr(), leaf_addr: addr, pk_addr: addr,
        };
        let mut ri = LeafInfoX1 {
            wots_sig: rwsig.as_mut_ptr(), wots_sign_leaf: 3,
            wots_steps: steps.as_mut_ptr(), leaf_addr: addr, pk_addr: addr,
        };
        let mut cleaf = vec![0u8; p.n];
        let mut rleaf = vec![0u8; p.n];
        cwl(cleaf.as_mut_ptr(), cc.as_ptr().cast(), 3, &mut ci);
        rwl(rleaf.as_mut_ptr(), rc.as_ptr().cast(), 3, &mut ri);
        assert_eq!((rleaf, rwsig, ri.leaf_addr, ri.pk_addr),
                   (cleaf, cwsig, ci.leaf_addr, ci.pk_addr));

        type ForsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut u32);
        let cfl: libloading::os::unix::Symbol<ForsLeaf> =
            libs.core.get(b"SPX_fors_gen_leafx1\0").unwrap();
        let rfl: libloading::os::unix::Symbol<ForsLeaf> =
            libs.rust.get(b"SPX_fors_gen_leafx1\0").unwrap();
        let mut cfi = addr;
        let mut rfi = addr;
        let mut cleaf = vec![0u8; p.n];
        let mut rleaf = vec![0u8; p.n];
        cfl(cleaf.as_mut_ptr(), cc.as_ptr().cast(), 11, cfi.as_mut_ptr());
        rfl(rleaf.as_mut_ptr(), rc.as_ptr().cast(), 11, rfi.as_mut_ptr());
        assert_eq!((rleaf, rfi), (cleaf, cfi));

        type GenLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32);
        type TreeHash = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, u32, u32, u32,
            Option<GenLeaf>, *mut u32,
        );
        let ctree: libloading::os::unix::Symbol<TreeHash> =
            libs.core.get(b"SPX_treehash\0").unwrap();
        let rtree: libloading::os::unix::Symbol<TreeHash> =
            libs.rust.get(b"SPX_treehash\0").unwrap();
        let ccallback: GenLeaf = std::mem::transmute::<ForsLeaf, GenLeaf>(*cfl);
        let rcallback: GenLeaf = std::mem::transmute::<ForsLeaf, GenLeaf>(*rfl);
        for leaf_idx in [0u32, 3, 7] {
            let mut croot = vec![0u8; p.n];
            let mut rroot = vec![0u8; p.n];
            let mut cauth = vec![0u8; 3 * p.n];
            let mut rauth = vec![0u8; 3 * p.n];
            let mut ca = addr;
            let mut ra = addr;
            ctree(croot.as_mut_ptr(), cauth.as_mut_ptr(), cc.as_ptr().cast(),
                  leaf_idx, 8, 3, Some(ccallback), ca.as_mut_ptr());
            rtree(rroot.as_mut_ptr(), rauth.as_mut_ptr(), rc.as_ptr().cast(),
                  leaf_idx, 8, 3, Some(rcallback), ra.as_mut_ptr());
            assert_eq!((rroot, rauth, ra), (croot, cauth, ca));
        }

        type TreeHashX1 = unsafe extern "C" fn(
            *mut u8, *mut u8, *const c_void, u32, u32, u32, *mut u32, *mut c_void,
        );
        let cwt: libloading::os::unix::Symbol<TreeHashX1> =
            libs.core.get(b"SPX_wots_treehashx1\0").unwrap();
        let rwt: libloading::os::unix::Symbol<TreeHashX1> =
            libs.rust.get(b"SPX_wots_treehashx1\0").unwrap();
        let cft: libloading::os::unix::Symbol<TreeHashX1> =
            libs.core.get(b"SPX_fors_treehashx1\0").unwrap();
        let rft: libloading::os::unix::Symbol<TreeHashX1> =
            libs.rust.get(b"SPX_fors_treehashx1\0").unwrap();
        let mut croot = vec![0u8; p.n];
        let mut rroot = vec![0u8; p.n];
        let mut cauth = vec![0u8; 3 * p.n];
        let mut rauth = vec![0u8; 3 * p.n];
        let mut cwsig = vec![0u8; p.wots_bytes()];
        let mut rwsig = vec![0u8; p.wots_bytes()];
        let mut csteps = vec![0u32; p.wots_len()];
        for step in &mut csteps { *step = (rng.next_u64() % 16) as u32; }
        let mut rsteps = csteps.clone();
        let mut cinfo = LeafInfoX1 {
            wots_sig: cwsig.as_mut_ptr(), wots_sign_leaf: 3,
            wots_steps: csteps.as_mut_ptr(), leaf_addr: addr, pk_addr: addr,
        };
        let mut rinfo = LeafInfoX1 {
            wots_sig: rwsig.as_mut_ptr(), wots_sign_leaf: 3,
            wots_steps: rsteps.as_mut_ptr(), leaf_addr: addr, pk_addr: addr,
        };
        let mut ca = addr;
        let mut ra = addr;
        cwt(croot.as_mut_ptr(), cauth.as_mut_ptr(), cc.as_ptr().cast(),
            3, 8, 3, ca.as_mut_ptr(), (&mut cinfo as *mut LeafInfoX1).cast());
        rwt(rroot.as_mut_ptr(), rauth.as_mut_ptr(), rc.as_ptr().cast(),
            3, 8, 3, ra.as_mut_ptr(), (&mut rinfo as *mut LeafInfoX1).cast());
        assert_eq!(
            (rroot, rauth, rwsig, ra, rinfo.leaf_addr, rinfo.pk_addr),
            (croot, cauth, cwsig, ca, cinfo.leaf_addr, cinfo.pk_addr),
        );

        for leaf_idx in [0u32, 3, 7] {
            let mut croot = vec![0u8; p.n];
            let mut rroot = vec![0u8; p.n];
            let mut cauth = vec![0u8; 3 * p.n];
            let mut rauth = vec![0u8; 3 * p.n];
            let mut cinfo = addr;
            let mut rinfo = addr;
            let mut ca = addr;
            let mut ra = addr;
            cft(croot.as_mut_ptr(), cauth.as_mut_ptr(), cc.as_ptr().cast(),
                leaf_idx, 8, 3, ca.as_mut_ptr(), cinfo.as_mut_ptr().cast());
            rft(rroot.as_mut_ptr(), rauth.as_mut_ptr(), rc.as_ptr().cast(),
                leaf_idx, 8, 3, ra.as_mut_ptr(), rinfo.as_mut_ptr().cast());
            assert_eq!((rroot, rauth, ra, rinfo), (croot, cauth, ca, cinfo));
        }
    }
}
