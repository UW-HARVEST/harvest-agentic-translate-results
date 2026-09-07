//! Anti-loophole check: the differential tests use ONE self-contained C shared
//! object (`cbuild_flat/libspx_<combo>.so`) rather than the three CMake outputs,
//! because the CMake trio cross-references each other's symbols and can only be
//! loaded `RTLD_GLOBAL`, where it would interpose the identically named Rust
//! exports.
//!
//! This test verifies the substitution is sound: it loads the REAL CMake
//! artifacts (`app/libsphincs_core_det.so` + `lib/<be>/lib<be>.so`) and checks
//! that they agree byte-for-byte with the flat object on a spread of entry
//! points from every level of the library.
//!
//! (`kat_all.sh` independently checks the CMake-linked `driver` executable
//! against the Rust `driver` for all 48 combinations.)

mod common;
use common::*;

use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_LAZY};
use std::path::PathBuf;

type SizeFn = unsafe extern "C" fn() -> u64;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type InitHash = unsafe extern "C" fn(*mut u8);
type PrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type MerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);
type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type Aes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);

struct CmakeLibs {
    _crypto: Option<Library>,
    backend: Library,
    core: Library,
}

impl CmakeLibs {
    fn load() -> CmakeLibs {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let cdir = root.join("cbuild").join(combo());
        let be = backend_name();
        let backend_path = cdir.join("lib").join(be).join(format!("lib{be}.so"));
        let core_path = cdir.join("app").join("libsphincs_core_det.so");
        for p in [&backend_path, &core_path] {
            assert!(p.exists(), "missing {} (run ../build_c_all.sh)", p.display());
        }
        // RTLD_LAZY: libsphincs_core_det.so and lib<be>.so have mutually
        // undefined function symbols, so eager binding of either alone fails.
        let _crypto =
            unsafe { Library::open(Some("/usr/lib64/libcrypto.so.3"), RTLD_LAZY | RTLD_GLOBAL).ok() };
        let backend =
            unsafe { Library::open(Some(&backend_path), RTLD_LAZY | RTLD_GLOBAL).unwrap() };
        let core = unsafe { Library::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL).unwrap() };
        CmakeLibs {
            _crypto,
            backend,
            core,
        }
    }
    fn get<T>(&self, name: &str) -> Symbol<T> {
        unsafe {
            if let Ok(s) = self.core.get::<T>(name.as_bytes()) {
                return s;
            }
            self.backend
                .get::<T>(name.as_bytes())
                .unwrap_or_else(|e| panic!("CMake symbol `{name}`: {e}"))
        }
    }
}

#[test]
fn z01_cmake_libs_match_flat_lib() {
    let flat = libs();
    let cm = CmakeLibs::load();
    let mut rng = Rng::new(SEED + 700);

    // Non-vacuity: the CMake artifacts and the flat object must be distinct
    // implementations at distinct addresses (the CMake pair is RTLD_GLOBAL, the
    // flat object RTLD_LOCAL, and the test binary itself exports no project
    // symbol, so neither can bind to the other).
    for name in ["SPX_thash", "crypto_sign_verify", "SPX_prf_addr", "AES256_ECB"] {
        let a = flat.c::<unsafe extern "C" fn()>(name).into_raw() as usize;
        let b = cm.get::<unsafe extern "C" fn()>(name).into_raw() as usize;
        assert_ne!(a, b, "`{name}`: flat and CMake objects resolved to one address");
    }

    // sizes
    for name in [
        "crypto_sign_secretkeybytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
    ] {
        let a = unsafe { (flat.c::<SizeFn>(name))() };
        let b = unsafe { (cm.get::<SizeFn>(name))() };
        assert_eq!(a, b, "{name}");
    }

    // level 1: initialize_hash_function / prf_addr / thash
    let fi = flat.c::<InitHash>("SPX_initialize_hash_function");
    let ci = cm.get::<InitHash>("SPX_initialize_hash_function");
    let fp = flat.c::<PrfAddr>("SPX_prf_addr");
    let cp = cm.get::<PrfAddr>("SPX_prf_addr");
    let ft = flat.c::<Thash>("SPX_thash");
    let ct = cm.get::<Thash>("SPX_thash");
    for _ in 0..8 {
        let pub_seed = rng.bytes(SPX_N);
        let sk_seed = rng.bytes(SPX_N);
        let mut fc = Ctx::new();
        let mut cc = Ctx::new();
        fc.set_seeds(&pub_seed, &sk_seed);
        cc.set_seeds(&pub_seed, &sk_seed);
        unsafe {
            fi(fc.as_mut_ptr());
            ci(cc.as_mut_ptr());
        }
        eq_bytes("initialize_hash_function (flat vs cmake)", fc.bytes(), cc.bytes());
        let addr = rng.addr();
        let mut fo = vec![0u8; SPX_N];
        let mut co = vec![0u8; SPX_N];
        unsafe {
            fp(fo.as_mut_ptr(), fc.as_ptr(), addr.as_ptr());
            cp(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
        }
        eq_bytes("prf_addr (flat vs cmake)", &fo, &co);
        for inblocks in [1u32, 2, SPX_FORS_TREES] {
            let inp = rng.bytes(inblocks as usize * SPX_N);
            let mut fa = addr;
            let mut ca = addr;
            let mut fo = vec![0u8; SPX_N];
            let mut co = vec![0u8; SPX_N];
            unsafe {
                ft(fo.as_mut_ptr(), inp.as_ptr(), inblocks, fc.as_ptr(), fa.as_mut_ptr());
                ct(co.as_mut_ptr(), inp.as_ptr(), inblocks, cc.as_ptr(), ca.as_mut_ptr());
            }
            eq_bytes(
                &format!("thash(inblocks={inblocks}) (flat vs cmake)"),
                &fo,
                &co,
            );
        }

        // level 3
        let fg = flat.c::<MerkleGenRoot>("SPX_merkle_gen_root");
        let cg = cm.get::<MerkleGenRoot>("SPX_merkle_gen_root");
        let mut fr = vec![0u8; SPX_N];
        let mut cr = vec![0u8; SPX_N];
        unsafe {
            fg(fr.as_mut_ptr(), fc.as_ptr());
            cg(cr.as_mut_ptr(), cc.as_ptr());
        }
        eq_bytes("merkle_gen_root (flat vs cmake)", &fr, &cr);

        let ffs = flat.c::<ForsSign>("SPX_fors_sign");
        let cfs = cm.get::<ForsSign>("SPX_fors_sign");
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let fa = rng.addr();
        let mut fsig = vec![0u8; SPX_FORS_BYTES];
        let mut csig = vec![0u8; SPX_FORS_BYTES];
        let mut fpk = vec![0u8; SPX_N];
        let mut cpk2 = vec![0u8; SPX_N];
        unsafe {
            ffs(fsig.as_mut_ptr(), fpk.as_mut_ptr(), m.as_ptr(), fc.as_ptr(), fa.as_ptr());
            cfs(csig.as_mut_ptr(), cpk2.as_mut_ptr(), m.as_ptr(), cc.as_ptr(), fa.as_ptr());
        }
        eq_bytes("fors_sign sig (flat vs cmake)", &fsig, &csig);
        eq_bytes("fors_sign pk (flat vs cmake)", &fpk, &cpk2);
    }

    // level 4: seed_keypair + verify
    let fk = flat.c::<SeedKeypair>("crypto_sign_seed_keypair");
    let ck = cm.get::<SeedKeypair>("crypto_sign_seed_keypair");
    let fv = flat.c::<Verify>("crypto_sign_verify");
    let cv = cm.get::<Verify>("crypto_sign_verify");
    for _ in 0..4 {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut fpk = vec![0u8; SPX_PK_BYTES];
        let mut cpk = vec![0u8; SPX_PK_BYTES];
        let mut fsk = vec![0u8; SPX_SK_BYTES];
        let mut csk = vec![0u8; SPX_SK_BYTES];
        unsafe {
            fk(fpk.as_mut_ptr(), fsk.as_mut_ptr(), seed.as_ptr());
            ck(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        }
        eq_bytes("seed_keypair pk (flat vs cmake)", &fpk, &cpk);
        eq_bytes("seed_keypair sk (flat vs cmake)", &fsk, &csk);
        let sig = rng.bytes(SPX_BYTES);
        let m = rng.bytes(33);
        let a = unsafe { fv(sig.as_ptr(), SPX_BYTES, m.as_ptr(), 33, fpk.as_ptr()) };
        let b = unsafe { cv(sig.as_ptr(), SPX_BYTES, m.as_ptr(), 33, cpk.as_ptr()) };
        assert_eq!(a, b, "crypto_sign_verify (flat vs cmake)");
        // wrong siglen boundary
        let a = unsafe { fv(sig.as_ptr(), SPX_BYTES - 1, m.as_ptr(), 33, fpk.as_ptr()) };
        let b = unsafe { cv(sig.as_ptr(), SPX_BYTES - 1, m.as_ptr(), 33, cpk.as_ptr()) };
        assert_eq!(a, b);
        assert_eq!(a, -1);
    }

    // rng.c
    let fe = flat.c::<Aes256Ecb>("AES256_ECB");
    let ce = cm.get::<Aes256Ecb>("AES256_ECB");
    for _ in 0..8 {
        let key = rng.bytes(32);
        let ctr = rng.bytes(16);
        let mut fk2 = key.clone();
        let mut ck2 = key.clone();
        let mut fc2 = ctr.clone();
        let mut cc2 = ctr.clone();
        let mut fo = vec![0u8; 16];
        let mut co = vec![0u8; 16];
        unsafe {
            fe(fk2.as_mut_ptr(), fc2.as_mut_ptr(), fo.as_mut_ptr());
            ce(ck2.as_mut_ptr(), cc2.as_mut_ptr(), co.as_mut_ptr());
        }
        eq_bytes("AES256_ECB (flat vs cmake)", &fo, &co);
    }
}
