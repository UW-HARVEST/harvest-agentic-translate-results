//! Shared differential-test harness.
//!
//! Both the C reference and the Rust translation are loaded as **shared
//! objects** through `libloading`; no Rust function is ever called directly, so
//! every test also exercises the `#[no_mangle]`/`extern "C"` export wrappers.
//!
//! ## Load order matters
//!
//! `libsphincs_core_det.so` has *undefined* references to the backend hooks
//! (`SPX_thash`, `SPX_prf_addr`, …) and to OpenSSL's `EVP_*`, so those have to
//! be in the global namespace before it is opened.  But the Rust `cdylib`
//! exports the *same* names, so if it were opened after the C libraries went
//! global, the dynamic linker could bind the Rust library's internal calls to
//! the C implementations and silently turn every test into "C vs C".
//!
//! To make that impossible the Rust library is opened **first**, with
//! `RTLD_NOW | RTLD_LOCAL`: `RTLD_NOW` forces all of its relocations to be
//! resolved immediately, at a point where nothing else is loaded that could
//! provide them, and `RTLD_LOCAL` keeps its exports out of the global scope.
//! `tests/configs.rs::cfg00_no_symbol_interposition` re-checks this
//! empirically.
//!
//! Paths and the ground-truth parameter dump come from `$SPX_DIF_DIR`, which
//! `run_tests.sh` points at `/tmp/dif/<backend>_<thash>_<secpar>/`.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol, RTLD_GLOBAL, RTLD_LAZY, RTLD_LOCAL, RTLD_NOW};
use std::collections::HashMap;
use std::ffi::{c_int, c_uint, c_ulong, c_ulonglong, c_void};
use std::sync::{Mutex, OnceLock};

/* ------------------------------------------------------------------ */
/* ground-truth parameters (dumped by harness/dump_params.c)           */
/* ------------------------------------------------------------------ */

pub struct Params {
    ints: HashMap<String, u64>,
    strs: HashMap<String, String>,
}

impl Params {
    fn parse(text: &str) -> Self {
        let mut ints = HashMap::new();
        let mut strs = HashMap::new();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match v.trim().parse::<u64>() {
                Ok(n) => {
                    ints.insert(k.to_string(), n);
                }
                Err(_) => {
                    strs.insert(k.to_string(), v.trim().to_string());
                }
            }
        }
        Params { ints, strs }
    }

    /// A numeric parameter.  Panics if absent: a missing parameter means the
    /// dump and the test disagree about the configuration, which must never be
    /// silently tolerated.
    pub fn n(&self, key: &str) -> usize {
        *self
            .ints
            .get(key)
            .unwrap_or_else(|| panic!("params.txt has no `{key}`")) as usize
    }
    pub fn opt(&self, key: &str) -> Option<usize> {
        self.ints.get(key).map(|v| *v as usize)
    }
    pub fn s(&self, key: &str) -> &str {
        self.strs
            .get(key)
            .unwrap_or_else(|| panic!("params.txt has no `{key}`"))
    }

    pub fn backend(&self) -> &str {
        self.s("BACKEND")
    }
    pub fn thash(&self) -> &str {
        self.s("THASH")
    }
    pub fn combo(&self) -> &str {
        self.s("COMBO")
    }
    /// `SPX_SHA512` / `SPX_BLAKE512`: selects the 512-bit variant for
    /// `inblocks > 1` in `thash` and for `blakeX`/`shaX` in `hash_<b>.c`.
    pub fn x512(&self) -> bool {
        self.n("X512") == 1
    }

    // frequently used shorthands
    pub fn n_(&self) -> usize {
        self.n("SPX_N")
    }
    pub fn addr_bytes(&self) -> usize {
        self.n("SPX_ADDR_BYTES")
    }
    pub fn wots_len(&self) -> usize {
        self.n("SPX_WOTS_LEN")
    }
    pub fn wots_bytes(&self) -> usize {
        self.n("SPX_WOTS_BYTES")
    }
    pub fn wots_w(&self) -> usize {
        self.n("SPX_WOTS_W")
    }
    pub fn fors_trees(&self) -> usize {
        self.n("SPX_FORS_TREES")
    }
    pub fn fors_height(&self) -> usize {
        self.n("SPX_FORS_HEIGHT")
    }
    pub fn fors_bytes(&self) -> usize {
        self.n("SPX_FORS_BYTES")
    }
    pub fn fors_msg_bytes(&self) -> usize {
        self.n("SPX_FORS_MSG_BYTES")
    }
    pub fn tree_height(&self) -> usize {
        self.n("SPX_TREE_HEIGHT")
    }
    pub fn d(&self) -> usize {
        self.n("SPX_D")
    }
    pub fn spx_bytes(&self) -> usize {
        self.n("SPX_BYTES")
    }
    pub fn pk_bytes(&self) -> usize {
        self.n("SPX_PK_BYTES")
    }
    pub fn sk_bytes(&self) -> usize {
        self.n("SPX_SK_BYTES")
    }
    pub fn seed_bytes(&self) -> usize {
        self.n("CRYPTO_SEEDBYTES")
    }
    pub fn ctx_size(&self) -> usize {
        self.n("sizeof_spx_ctx")
    }
    pub fn leaf_info_size(&self) -> usize {
        self.n("sizeof_leaf_info_x1")
    }
}

/* ------------------------------------------------------------------ */
/* the two libraries under comparison                                 */
/* ------------------------------------------------------------------ */

pub struct Libs {
    /// Rust `cdylib` — opened first, `RTLD_NOW | RTLD_LOCAL`.
    pub rs: Library,
    /// `libsphincs_core_det.so` (app/src/* + rng.c).
    pub c_core: Library,
    /// `lib<backend>.so`.
    pub c_backend: Library,
    _crypto: Option<Library>,
}

impl Libs {
    /// Look a symbol up in the Rust `.so`.
    pub unsafe fn r<T>(&self, name: &str) -> Symbol<T> {
        self.rs
            .get(name.as_bytes())
            .unwrap_or_else(|e| panic!("rust .so is missing `{name}`: {e}"))
    }
    /// Look a symbol up in the C `.so`s (core first, then the backend).
    pub unsafe fn c<T>(&self, name: &str) -> Symbol<T> {
        if let Ok(s) = self.c_core.get::<T>(name.as_bytes()) {
            return s;
        }
        self.c_backend
            .get(name.as_bytes())
            .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"))
    }
    /// Raw address of a **data** symbol (e.g. `DRBG_ctx`, `cst`).
    pub unsafe fn r_data(&self, name: &str) -> *mut u8 {
        let s: Symbol<*mut u8> = self.r(name);
        s.into_raw() as *mut u8
    }
    pub unsafe fn c_data(&self, name: &str) -> *mut u8 {
        let s: Symbol<*mut u8> = self.c(name);
        s.into_raw() as *mut u8
    }
}

static CTX: OnceLock<(Libs, Params)> = OnceLock::new();

pub fn env() -> &'static (Libs, Params) {
    CTX.get_or_init(|| {
        let dir = std::env::var("SPX_DIF_DIR").expect(
            "set SPX_DIF_DIR to /tmp/dif/<backend>_<thash>_<secpar> \
             (see run_tests.sh); build it with ./build_matrix.sh",
        );
        let params = Params::parse(
            &std::fs::read_to_string(format!("{dir}/params.txt"))
                .expect("params.txt missing -- run ./build_matrix.sh"),
        );

        // 1. Rust FIRST, fully bound, private namespace.  See module docs.
        let rs = unsafe { Library::open(Some(format!("{dir}/librs.so")), RTLD_NOW | RTLD_LOCAL) }
            .expect("cannot open librs.so");

        // 2. libcrypto (rng.c's AES256_ECB uses EVP_*; only the C side needs it).
        let _crypto = ["/tmp/osslib/libcrypto.so", "/usr/lib64/libcrypto.so.3"]
            .iter()
            .find_map(|p| unsafe { Library::open(Some(p), RTLD_NOW | RTLD_GLOBAL) }.ok());

        // 3./4. The two C libraries reference each other: `lib<backend>.so`
        // compiles `app/src/utils.c`, whose `compute_root` calls
        // `SPX_set_tree_index` from `libsphincs_core_det.so`, while the core
        // calls `SPX_thash` from the backend.  In the CMake build the `driver`
        // links both at once, so the cycle is fine; when dlopen'ing them one at
        // a time the resolution has to be deferred, hence RTLD_LAZY.  They go in
        // the GLOBAL scope so they find each other -- and only each other, since
        // the Rust library above was opened RTLD_LOCAL and fully bound already.
        let c_backend = unsafe {
            Library::open(Some(format!("{dir}/libc_backend.so")), RTLD_LAZY | RTLD_GLOBAL)
        }
        .expect("cannot open libc_backend.so");

        let c_core = unsafe {
            Library::open(Some(format!("{dir}/libc_core_det.so")), RTLD_LAZY | RTLD_GLOBAL)
        }
        .expect("cannot open libc_core_det.so");

        (
            Libs {
                rs,
                c_core,
                c_backend,
                _crypto,
            },
            params,
        )
    })
}

/// `DRBG_ctx` is a process-global in both libraries, so any test that calls
/// `randombytes`, `randombytes_init`, `crypto_sign_keypair` or
/// `crypto_sign_signature` (which draws `optrand`) must hold this.
pub static DRBG: Mutex<()> = Mutex::new(());

/// Opens **only** the Rust `.so`, with no C library anywhere in the process.
/// Used by `cfg00_no_symbol_interposition` from a child process.
pub fn rs_only() -> (Library, Params) {
    let dir = std::env::var("SPX_DIF_DIR").expect("SPX_DIF_DIR");
    let params = Params::parse(&std::fs::read_to_string(format!("{dir}/params.txt")).unwrap());
    let lib = unsafe { Library::open(Some(format!("{dir}/librs.so")), RTLD_NOW | RTLD_LOCAL) }
        .expect("cannot open librs.so");
    (lib, params)
}

/// A deterministic digest over a broad slice of the Rust library's behaviour,
/// computed using *only* that library.  Comparing the value obtained with the C
/// libraries loaded against the value obtained in a clean process proves that no
/// symbol interposition is happening.
pub unsafe fn rs_fingerprint(rs: &Library, p: &Params) -> String {
    let mut acc = 0xcbf2_9ce4_8422_2325u64;
    let mut absorb = |b: &[u8]| {
        for x in b {
            acc = (acc ^ *x as u64).wrapping_mul(0x100_0000_01b3);
        }
    };

    let n = p.n_();
    let mut rng = Rng::new(0xF1_9E_00);

    // initialize_hash_function + prf_addr + thash at several inblocks
    let init: Symbol<FnInitHash> = rs.get(b"SPX_initialize_hash_function").unwrap();
    let prf: Symbol<FnPrfAddr> = rs.get(b"SPX_prf_addr").unwrap();
    let thash: Symbol<FnThash> = rs.get(b"SPX_thash").unwrap();
    let mut ctx = vec![0u8; p.ctx_size()];
    let seeds = rng.bytes(2 * n);
    ctx[..2 * n].copy_from_slice(&seeds);
    (*init)(ctx.as_mut_ptr());
    absorb(&ctx);
    for _ in 0..8 {
        let addr = rng.addr();
        let mut out = vec![0u8; n];
        (*prf)(out.as_mut_ptr(), ctx.as_ptr(), addr.as_ptr());
        absorb(&out);
    }
    for nb in [1usize, 2, 3, p.wots_len(), p.fors_trees()] {
        let mut a = rng.addr();
        let input = rng.bytes(nb * n);
        let mut out = vec![0u8; n];
        (*thash)(
            out.as_mut_ptr(),
            input.as_ptr(),
            nb as c_uint,
            ctx.as_ptr(),
            a.as_mut_ptr(),
        );
        absorb(&out);
    }

    // full keygen / sign / verify pipeline
    let kp: Symbol<FnSeedKeypair> = rs.get(b"crypto_sign_seed_keypair").unwrap();
    let rbi: Symbol<FnRandombytesInit> = rs.get(b"randombytes_init").unwrap();
    let sig: Symbol<FnSignature> = rs.get(b"crypto_sign_signature").unwrap();
    let ver: Symbol<FnVerify> = rs.get(b"crypto_sign_verify").unwrap();
    let seed = rng.bytes(p.seed_bytes());
    let mut pk = vec![0u8; p.pk_bytes()];
    let mut sk = vec![0u8; p.sk_bytes()];
    (*kp)(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    absorb(&pk);
    absorb(&sk);
    let mut ent: Vec<u8> = (0..48u8).collect();
    (*rbi)(ent.as_mut_ptr(), std::ptr::null_mut());
    for mlen in [0usize, 1, 33, 200] {
        let m = rng.bytes(mlen);
        let mut s = vec![0u8; p.spx_bytes()];
        let mut sl = 0usize;
        (*sig)(s.as_mut_ptr(), &mut sl, m.as_ptr(), mlen, sk.as_ptr());
        absorb(&s);
        absorb(&sl.to_le_bytes());
        let v = (*ver)(s.as_ptr(), sl, m.as_ptr(), mlen, pk.as_ptr());
        absorb(&v.to_le_bytes());
    }
    format!("{acc:016x}")
}

/* ------------------------------------------------------------------ */
/* deterministic PRNG (fixed seed => reproducible property tests)      */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64, so even seed 0 produces a good stream
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        while v.len() < n {
            v.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        v.truncate(n);
        v
    }
    pub fn fill(&mut self, dst: &mut [u8]) {
        let n = dst.len();
        dst.copy_from_slice(&self.bytes(n));
    }
    /// A random 8-word SPHINCS+ address.
    pub fn addr(&mut self) -> [u32; 8] {
        let mut a = [0u32; 8];
        for w in a.iter_mut() {
            *w = self.next_u32();
        }
        a
    }
}

/* ------------------------------------------------------------------ */
/* comparison helpers                                                 */
/* ------------------------------------------------------------------ */

pub fn same(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()));
        panic!(
            "{what}: C != Rust (len {} vs {}, first difference at byte {at})\n  C   : {}\n  Rust: {}",
            c.len(),
            r.len(),
            hex_around(c, at),
            hex_around(r, at)
        );
    }
}

pub fn same_i(what: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{what}: C returned {c}, Rust returned {r}");
}

pub fn same_u(what: &str, c: u64, r: u64) {
    assert_eq!(c, r, "{what}: C returned {c}, Rust returned {r}");
}

fn hex_around(b: &[u8], at: usize) -> String {
    let lo = at.saturating_sub(8);
    let hi = (at + 8).min(b.len());
    let mut s = String::new();
    if lo > 0 {
        s.push_str("..");
    }
    for x in &b[lo..hi] {
        s.push_str(&format!("{x:02x}"));
    }
    if hi < b.len() {
        s.push_str("..");
    }
    s
}

/// A "sponge" buffer padded with a guard pattern, so that a translation that
/// writes too many bytes is caught rather than silently tolerated.
pub fn guarded(len: usize, guard: usize) -> Vec<u8> {
    let mut v = vec![0xA5u8; len + guard];
    v[..len].fill(0);
    v
}

/* ------------------------------------------------------------------ */
/* C function-pointer type aliases                                    */
/* ------------------------------------------------------------------ */

pub type FnAddrU32 = unsafe extern "C" fn(*mut u32, u32);
pub type FnAddrU64 = unsafe extern "C" fn(*mut u32, u64);
pub type FnAddrCopy = unsafe extern "C" fn(*mut u32, *const u32);

pub type FnUllToBytes = unsafe extern "C" fn(*mut u8, c_uint, c_ulonglong);
pub type FnU32ToBytes = unsafe extern "C" fn(*mut u8, u32);
pub type FnBytesToUll = unsafe extern "C" fn(*const u8, c_uint) -> c_ulonglong;

pub type FnThash = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
pub type FnPrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
pub type FnInitHash = unsafe extern "C" fn(*mut u8);
pub type FnGenMsgRandom =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, c_ulonglong, *const u8);
pub type FnHashMessage = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    c_ulonglong,
    *const u8,
);

pub type FnComputeRoot =
    unsafe extern "C" fn(*mut u8, *const u8, u32, u32, *const u8, u32, *const u8, *mut u32);
pub type GenLeaf = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u32);
pub type FnTreehash =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, GenLeaf, *mut u32);
pub type FnTreehashX1 =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u32, u32, u32, *mut u32, *mut u8);

pub type FnChainLengths = unsafe extern "C" fn(*mut c_uint, *const u8);
pub type FnWotsPkFromSig =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
pub type FnWotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut u8);
pub type FnForsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut u8);
pub type FnForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
pub type FnForsPkFromSig =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
pub type FnMerkleSign =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
pub type FnMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

pub type FnSizes = unsafe extern "C" fn() -> c_ulonglong;
pub type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
pub type FnKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
pub type FnSignature =
    unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
pub type FnVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
pub type FnSign =
    unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int;
pub type FnOpen =
    unsafe extern "C" fn(*mut u8, *mut c_ulonglong, *const u8, c_ulonglong, *const u8) -> c_int;

pub type FnRandombytes = unsafe extern "C" fn(*mut u8, c_ulonglong) -> c_int;
pub type FnRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
pub type FnAes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
pub type FnDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
pub type FnSeedexpanderInit =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, c_ulong) -> c_int;
pub type FnSeedexpander = unsafe extern "C" fn(*mut u8, *mut u8, c_ulong) -> c_int;

/* blake */
pub type FnBlakeOneShot = unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int;
pub type FnBlakeInit = unsafe extern "C" fn(*mut u8);
pub type FnBlakeUpdate = unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong);
pub type FnBlakeFinal = unsafe extern "C" fn(*mut u8, *mut u8);
pub type FnBlakeCompress = unsafe extern "C" fn(*mut u8, *const u8);
pub type FnMgf1 = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);

/* sha2 */
pub type FnShaOneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
pub type FnShaIncInit = unsafe extern "C" fn(*mut u8);
pub type FnShaIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
pub type FnShaIncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
pub type FnSeedState = unsafe extern "C" fn(*mut u8);

/* shake */
pub type FnShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
pub type FnShakeIncInit = unsafe extern "C" fn(*mut u64);
pub type FnShakeIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
pub type FnShakeIncFinalize = unsafe extern "C" fn(*mut u64);
pub type FnShakeIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
pub type FnShakeAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
pub type FnShakeSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);

/* haraka */
pub type FnTweakConstants = unsafe extern "C" fn(*mut u8);
pub type FnHaraka512 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
pub type FnHarakaS =
    unsafe extern "C" fn(*mut u8, c_ulonglong, *const u8, c_ulonglong, *const u8);
pub type FnHarakaSIncInit = unsafe extern "C" fn(*mut u8);
pub type FnHarakaSIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
pub type FnHarakaSIncFinalize = unsafe extern "C" fn(*mut u8);
pub type FnHarakaSIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

/// Suppress "unused import" for `c_void` on configurations that do not need it.
pub fn _keep(_: *mut c_void) {}

/// A poison-tolerant `DRBG` lock: one failing test must not cascade into
/// `PoisonError` failures that hide the real cause.
pub fn drbg_lock() -> std::sync::MutexGuard<'static, ()> {
    DRBG.lock().unwrap_or_else(|e| e.into_inner())
}
