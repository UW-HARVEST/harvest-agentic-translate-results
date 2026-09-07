//! Isolation self-check.
//!
//! The harness has to open the C `.so`s `RTLD_GLOBAL` (they are mutually
//! recursive), which puts a definition of every exported symbol into the global
//! search scope *ahead* of the dlopened Rust library. Without
//! `RTLD_DEEPBIND` the Rust library's internal references get interposed by the
//! C ones and the whole differential suite silently degenerates into "C vs C".
//!
//! This file proves that is not happening: it loads a *second, completely
//! isolated* copy of the Rust `.so` with `dlmopen(LM_ID_NEWLM, …)` — a fresh
//! link-map namespace that cannot see the C libraries at all — and checks that
//! the normally-loaded Rust library produces identical results. If interposition
//! were occurring, the normal handle would run C code and these would differ.

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_long, c_void, CString};

extern "C" {
    fn dlmopen(lmid: c_long, file: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

const LM_ID_NEWLM: c_long = -1;
const RTLD_NOW_: c_int = 0x2;
const RTLD_LOCAL_: c_int = 0;

struct Isolated(*mut c_void);

impl Isolated {
    fn open(path: &str) -> Self {
        let cs = CString::new(path).unwrap();
        let h = unsafe { dlmopen(LM_ID_NEWLM, cs.as_ptr(), RTLD_NOW_ | RTLD_LOCAL_) };
        if h.is_null() {
            let e = unsafe { std::ffi::CStr::from_ptr(dlerror()) };
            panic!("dlmopen({path}) failed: {}", e.to_string_lossy());
        }
        Isolated(h)
    }
    fn raw(&self, name: &str) -> *mut c_void {
        let cs = CString::new(name).unwrap();
        let p = unsafe { dlsym(self.0, cs.as_ptr()) };
        assert!(!p.is_null(), "isolated lib: symbol {name} not found");
        p
    }
    fn f<T: Copy>(&self, name: &str) -> T {
        let p = self.raw(name);
        unsafe { *(&p as *const _ as *const T) }
    }
    unsafe fn data(&self, name: &str, len: usize) -> &[u8] {
        core::slice::from_raw_parts(self.raw(name) as *const u8, len)
    }
}

fn rust_so_path() -> String {
    if let Ok(p) = std::env::var("SPX_RUST_SO") {
        return p;
    }
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for prof in ["release", "debug"] {
        let q = base.join(prof).join("libsphincs_plus.so");
        if q.exists() {
            return q.to_string_lossy().into_owned();
        }
    }
    panic!("no libsphincs_plus.so found");
}

/// Force the main harness (and therefore the RTLD_GLOBAL C libraries) to be
/// loaded first, so the isolated copy is genuinely competing with them.
fn setup() -> (&'static Pair, Isolated) {
    let p = libs();
    let iso = Isolated::open(&rust_so_path());
    (p, iso)
}

#[test]
fn iso_01_exported_addresses_are_distinct() {
    let (p, iso) = setup();
    for name in ["SPX_thash", "SPX_prf_addr", "crypto_sign_verify", "AES256_ECB"] {
        let a = p.c.raw(name);
        let b = p.r.raw(name);
        let c = iso.raw(name);
        assert_ne!(a, b, "{name}: C and Rust resolved to the SAME address");
        assert_ne!(b, c, "{name}: isolated copy resolved to the same address");
    }
}

#[test]
fn iso_02_thash_matches_isolated_rust() {
    let (p, iso) = setup();
    let init_n: FnInitHash = p.r.f("SPX_initialize_hash_function");
    let init_i: FnInitHash = iso.f("SPX_initialize_hash_function");
    let th_n: FnThash = p.r.f("SPX_thash");
    let th_i: FnThash = iso.f("SPX_thash");

    let mut rng = Rng::for_row(9001);
    for i in 0..8 {
        let pub_seed = rng.bytes(SPX_N);
        let sk_seed = rng.bytes(SPX_N);
        let mut cn = Ctx::with_seeds(&pub_seed, &sk_seed);
        let mut ci = Ctx::with_seeds(&pub_seed, &sk_seed);
        unsafe {
            init_n(cn.as_mut_ptr());
            init_i(ci.as_mut_ptr());
        }
        eq_bytes("iso/initialize_hash_function", i, &cn.0, &ci.0);

        for &blocks in &[1usize, 2, SPX_WOTS_LEN] {
            let input = rng.bytes(blocks * SPX_N);
            let addr = rng.addr();
            let mut an = addr;
            let mut ai = addr;
            let mut on = vec![0xA5u8; SPX_N + 8];
            let mut oi = on.clone();
            unsafe {
                th_n(
                    on.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as core::ffi::c_uint,
                    cn.as_ptr(),
                    an.as_mut_ptr(),
                );
                th_i(
                    oi.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as core::ffi::c_uint,
                    ci.as_ptr(),
                    ai.as_mut_ptr(),
                );
            }
            eq_bytes(&format!("iso/thash(blocks={blocks})"), i, &on, &oi);
            eq_u32s("iso/thash addr", i, &an, &ai);
        }
    }
}

#[test]
fn iso_03_drbg_matches_isolated_rust() {
    let (p, iso) = setup();
    let in_n: FnRandombytesInit = p.r.f("randombytes_init");
    let in_i: FnRandombytesInit = iso.f("randombytes_init");
    let rb_n: FnRandombytes = p.r.f("randombytes");
    let rb_i: FnRandombytes = iso.f("randombytes");

    let mut rng = Rng::for_row(9002);
    for i in 0..8 {
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        unsafe {
            in_n(e1.as_mut_ptr(), core::ptr::null_mut());
            in_i(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        let sn = unsafe { p.r.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
        let si = unsafe { iso.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
        eq_bytes("iso/randombytes_init DRBG_ctx", i, &sn, &si);

        let xlen = 100usize;
        let mut on = vec![0u8; xlen];
        let mut oi = vec![0u8; xlen];
        unsafe {
            rb_n(on.as_mut_ptr(), xlen as u64);
            rb_i(oi.as_mut_ptr(), xlen as u64);
        }
        eq_bytes("iso/randombytes out", i, &on, &oi);
    }
}

#[test]
fn iso_04_keypair_matches_isolated_rust() {
    let (p, iso) = setup();
    let kn: FnSeedKeypair = p.r.f("crypto_sign_seed_keypair");
    let ki: FnSeedKeypair = iso.f("crypto_sign_seed_keypair");
    let mut rng = Rng::for_row(9003);
    for i in 0..2 {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pkn = vec![0xA5u8; SPX_PK_BYTES];
        let mut skn = vec![0xA5u8; SPX_SK_BYTES];
        let mut pki = pkn.clone();
        let mut ski = skn.clone();
        unsafe {
            kn(pkn.as_mut_ptr(), skn.as_mut_ptr(), seed.as_ptr());
            ki(pki.as_mut_ptr(), ski.as_mut_ptr(), seed.as_ptr());
        }
        eq_bytes("iso/seed_keypair pk", i, &pkn, &pki);
        eq_bytes("iso/seed_keypair sk", i, &skn, &ski);
    }
}
