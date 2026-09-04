use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_NOW};
use std::ffi::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const RTLD_DEEPBIND: c_int = 0x00008;
const N: usize = if cfg!(any(feature = "128f", feature = "128s")) {
    16
} else if cfg!(any(feature = "192f", feature = "192s")) {
    24
} else {
    32
};
const FULL_HEIGHT: usize = if cfg!(feature = "128f") || cfg!(feature = "192f") {
    66
} else if cfg!(feature = "256f") {
    68
} else if cfg!(feature = "256s") {
    64
} else {
    63
};
const D: usize = if cfg!(any(feature = "128f", feature = "192f")) {
    22
} else if cfg!(feature = "256f") {
    17
} else if cfg!(feature = "256s") {
    8
} else {
    7
};
const FORS_HEIGHT: usize = if cfg!(feature = "128f") {
    6
} else if cfg!(feature = "128s") {
    12
} else if cfg!(feature = "192f") {
    8
} else if cfg!(feature = "256f") {
    9
} else {
    14
};
const FORS_TREES: usize = if cfg!(feature = "128f") || cfg!(feature = "192f") {
    33
} else if cfg!(feature = "128s") {
    14
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
const GMR_BYTES: usize = if cfg!(feature = "blake") {
    if N >= 24 { 64 } else { 32 }
} else {
    N
};
const MESSAGE_BLOCK: usize = if cfg!(feature = "shake") {
    136
} else if cfg!(feature = "haraka") {
    32
} else if N >= 24 {
    128
} else {
    64
};

struct Libs {
    _crypto: Library,
    core: Library,
    backend: Library,
    rust: Library,
}

unsafe extern "C" fn deterministic_leaf(
    leaf: *mut u8,
    _ctx: *const c_void,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let tree = unsafe { std::slice::from_raw_parts(tree_addr, 8) };
    let out = unsafe { std::slice::from_raw_parts_mut(leaf, N) };
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = (addr_idx as u8)
            .wrapping_add(i as u8)
            .wrapping_add(tree[i % 8] as u8);
    }
}

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

impl Libs {
    unsafe fn load() -> &'static Self {
        let root = std::env::var_os("SPX_C_BUILD_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("../c_src/build"));
        let backend_name = if cfg!(feature = "blake") {
            "blake"
        } else if cfg!(feature = "sha2") {
            "sha2"
        } else if cfg!(feature = "shake") {
            "shake"
        } else {
            "haraka"
        };
        let crypto = unsafe {
            Library::open(Some(Path::new("/lib64/libcrypto.so.3")), RTLD_NOW | RTLD_GLOBAL)
        }
        .unwrap();
        let core_path = root.join("app/libsphincs_core_det.so");
        let core = unsafe { Library::open(Some(&core_path), RTLD_LAZY | RTLD_GLOBAL) }.unwrap();
        let backend_path = root
            .join("lib")
            .join(backend_name)
            .join(format!("lib{backend_name}.so"));
        let backend =
            unsafe { Library::open(Some(&backend_path), RTLD_NOW | RTLD_GLOBAL) }.unwrap();
        let rust_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/release/libsphincs_plus_translation.so");
        // C and Rust intentionally export identical global names. Prefer the
        // Rust DSO's own definitions for its internal relocations so loading
        // both implementations in one process does not merge their state.
        let rust =
            unsafe { Library::open(Some(&rust_path), RTLD_NOW | RTLD_DEEPBIND) }.unwrap();
        Box::leak(Box::new(Self {
            _crypto: crypto,
            core,
            backend,
            rust,
        }))
    }

    unsafe fn pair<T: Copy>(&self, name: &[u8]) -> (T, T) {
        let c = unsafe { self.core.get::<T>(name) }
            .or_else(|_| unsafe { self.backend.get::<T>(name) })
            .unwrap_or_else(|_| panic!("missing C symbol {}", String::from_utf8_lossy(name)));
        let r = unsafe { self.rust.get::<T>(name) }
            .unwrap_or_else(|_| panic!("missing Rust symbol {}", String::from_utf8_lossy(name)));
        (*c, *r)
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d595df4d0f33173)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn fill(&mut self, out: &mut [u8]) {
        for b in out {
            *b = self.next() as u8;
        }
    }
}

fn context_len() -> usize {
    if cfg!(feature = "haraka") {
        2 * N + 10 * 8 * 8 + 10 * 8 * 4
    } else if cfg!(feature = "sha2") {
        2 * N + 40 + if N > 16 { 72 } else { 0 }
    } else {
        2 * N
    }
}

fn contexts(rng: &mut Rng) -> (Vec<u64>, Vec<u64>) {
    let words = context_len().div_ceil(8);
    let mut c = vec![0u64; words];
    let mut r = vec![0u64; words];
    let c_bytes =
        unsafe { std::slice::from_raw_parts_mut(c.as_mut_ptr().cast::<u8>(), words * 8) };
    rng.fill(&mut c_bytes[..2 * N]);
    let r_bytes =
        unsafe { std::slice::from_raw_parts_mut(r.as_mut_ptr().cast::<u8>(), words * 8) };
    r_bytes[..2 * N].copy_from_slice(&c_bytes[..2 * N]);
    (c, r)
}

#[test]
fn sizes_utils_and_addresses_match() {
    let _guard = test_lock();
    unsafe {
        let libs = Libs::load();
        for name in [
            b"crypto_sign_secretkeybytes\0".as_slice(),
            b"crypto_sign_publickeybytes\0",
            b"crypto_sign_bytes\0",
            b"crypto_sign_seedbytes\0",
        ] {
            let (c, r): (unsafe extern "C" fn() -> u64, _) = libs.pair(name);
            assert_eq!(c(), r());
        }
        let (c_ull, r_ull): (unsafe extern "C" fn(*mut u8, u32, u64), _) =
            libs.pair(b"SPX_ull_to_bytes\0");
        let (c_btu, r_btu): (unsafe extern "C" fn(*const u8, u32) -> u64, _) =
            libs.pair(b"SPX_bytes_to_ull\0");
        for len in [0u32, 1, 4, 8] {
            for value in [0, 1, u32::MAX as u64, u64::MAX] {
                let mut co = [0xa5; 8];
                let mut ro = [0xa5; 8];
                c_ull(co.as_mut_ptr(), len, value);
                r_ull(ro.as_mut_ptr(), len, value);
                assert_eq!(co, ro);
                assert_eq!(c_btu(co.as_ptr(), len), r_btu(ro.as_ptr(), len));
            }
        }
        let setters: [(&[u8], bool); 8] = [
            (b"SPX_set_layer_addr\0", false),
            (b"SPX_set_tree_addr\0", true),
            (b"SPX_set_type\0", false),
            (b"SPX_set_keypair_addr\0", false),
            (b"SPX_set_chain_addr\0", false),
            (b"SPX_set_hash_addr\0", false),
            (b"SPX_set_tree_height\0", false),
            (b"SPX_set_tree_index\0", false),
        ];
        for (name, wide) in setters {
            for value in [0u64, 1, 6, 7, 255, 256, u32::MAX as u64, u64::MAX] {
                let mut ca = [0xa5a5a5a5u32; 8];
                let mut ra = ca;
                if wide {
                    let (c, r): (unsafe extern "C" fn(*mut u32, u64), _) = libs.pair(name);
                    c(ca.as_mut_ptr(), value);
                    r(ra.as_mut_ptr(), value);
                } else {
                    let (c, r): (unsafe extern "C" fn(*mut u32, u32), _) = libs.pair(name);
                    c(ca.as_mut_ptr(), value as u32);
                    r(ra.as_mut_ptr(), value as u32);
                }
                assert_eq!(ca, ra, "{}", String::from_utf8_lossy(name));
            }
        }
    }
}

#[test]
fn hash_wots_and_fors_surfaces_match() {
    let _guard = test_lock();
    unsafe {
        let libs = Libs::load();
        let (init_c, init_r): (unsafe extern "C" fn(*mut c_void), _) =
            libs.pair(b"SPX_initialize_hash_function\0");
        let (prf_c, prf_r): (
            unsafe extern "C" fn(*mut u8, *const c_void, *const u32),
            _,
        ) = libs.pair(b"SPX_prf_addr\0");
        let (gmr_c, gmr_r): (
            unsafe extern "C" fn(
                *mut u8,
                *const u8,
                *const u8,
                *const u8,
                u64,
                *const c_void,
            ),
            _,
        ) = libs.pair(b"SPX_gen_message_random\0");
        let (hm_c, hm_r): (
            unsafe extern "C" fn(
                *mut u8,
                *mut u64,
                *mut u32,
                *const u8,
                *const u8,
                *const u8,
                u64,
                *const c_void,
            ),
            _,
        ) = libs.pair(b"SPX_hash_message\0");
        let (th_c, th_r): (
            unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32),
            _,
        ) = libs.pair(b"SPX_thash\0");
        let (cl_c, cl_r): (unsafe extern "C" fn(*mut u32, *const u8), _) =
            libs.pair(b"SPX_chain_lengths\0");
        let (wpk_c, wpk_r): (
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *mut u32),
            _,
        ) = libs.pair(b"SPX_wots_pk_from_sig\0");
        let (root_c, root_r): (
            unsafe extern "C" fn(
                *mut u8,
                *const u8,
                u32,
                u32,
                *const u8,
                u32,
                *const c_void,
                *mut u32,
            ),
            _,
        ) = libs.pair(b"SPX_compute_root\0");
        let (fg_c, fg_r): (
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut c_void),
            _,
        ) = libs.pair(b"SPX_fors_gen_leafx1\0");
        let (tree_c, tree_r): (
            unsafe extern "C" fn(
                *mut u8,
                *mut u8,
                *const c_void,
                u32,
                u32,
                u32,
                Option<unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32)>,
                *mut u32,
            ),
            _,
        ) = libs.pair(b"SPX_treehash\0");
        let (merkle_c, merkle_r): (
            unsafe extern "C" fn(*mut u8, *mut u8, *const c_void, *mut u32, *mut u32, u32),
            _,
        ) = libs.pair(b"SPX_merkle_sign\0");
        let (merkle_root_c, merkle_root_r): (
            unsafe extern "C" fn(*mut u8, *const c_void),
            _,
        ) = libs.pair(b"SPX_merkle_gen_root\0");
        let (fs_c, fs_r): (
            unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const c_void, *const u32),
            _,
        ) = libs.pair(b"SPX_fors_sign\0");
        let (fp_c, fp_r): (
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const c_void, *const u32),
            _,
        ) = libs.pair(b"SPX_fors_pk_from_sig\0");
        let mut rng = Rng::new();
        for round in 0..8 {
            let (mut cc, mut rc) = contexts(&mut rng);
            init_c(cc.as_mut_ptr().cast());
            init_r(rc.as_mut_ptr().cast());
            assert_eq!(cc, rc);
            let mut addr = [0u32; 8];
            for x in &mut addr {
                *x = rng.next() as u32;
            }
            let mut co = vec![0u8; GMR_BYTES];
            let mut ro = vec![0u8; GMR_BYTES];
            prf_c(co.as_mut_ptr(), cc.as_ptr().cast(), addr.as_ptr());
            prf_r(ro.as_mut_ptr(), rc.as_ptr().cast(), addr.as_ptr());
            assert_eq!(&co[..N], &ro[..N]);
            let lengths = [
                0,
                1,
                MESSAGE_BLOCK - 1,
                MESSAGE_BLOCK,
                MESSAGE_BLOCK + 1,
                2 * MESSAGE_BLOCK + 1,
            ];
            let len = lengths[round % lengths.len()];
            let mut msg = vec![0u8; len];
            let mut key = vec![0u8; N];
            let mut optrand = vec![0u8; N];
            let mut pk = vec![0u8; PK_BYTES];
            rng.fill(&mut msg);
            rng.fill(&mut key);
            rng.fill(&mut optrand);
            rng.fill(&mut pk);
            gmr_c(
                co.as_mut_ptr(),
                key.as_ptr(),
                optrand.as_ptr(),
                msg.as_ptr(),
                len as u64,
                cc.as_ptr().cast(),
            );
            gmr_r(
                ro.as_mut_ptr(),
                key.as_ptr(),
                optrand.as_ptr(),
                msg.as_ptr(),
                len as u64,
                rc.as_ptr().cast(),
            );
            assert_eq!(co, ro);
            let mut cd = vec![0u8; FORS_MSG_BYTES];
            let mut rd = vec![0u8; FORS_MSG_BYTES];
            let (mut ct, mut rt, mut ci, mut ri) = (0, 0, 0, 0);
            hm_c(
                cd.as_mut_ptr(),
                &mut ct,
                &mut ci,
                co.as_ptr(),
                pk.as_ptr(),
                msg.as_ptr(),
                len as u64,
                cc.as_ptr().cast(),
            );
            hm_r(
                rd.as_mut_ptr(),
                &mut rt,
                &mut ri,
                ro.as_ptr(),
                pk.as_ptr(),
                msg.as_ptr(),
                len as u64,
                rc.as_ptr().cast(),
            );
            assert_eq!((cd, ct, ci), (rd, rt, ri));
            for blocks in [1usize, 2, WOTS_LEN, FORS_TREES] {
                let mut input = vec![0u8; blocks * N];
                rng.fill(&mut input);
                let (mut ca, mut ra) = (addr, addr);
                th_c(
                    co.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    cc.as_ptr().cast(),
                    ca.as_mut_ptr(),
                );
                th_r(
                    ro.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    rc.as_ptr().cast(),
                    ra.as_mut_ptr(),
                );
                assert_eq!((co.clone(), ca), (ro.clone(), ra));
            }
            let mut wmsg = vec![0u8; N];
            rng.fill(&mut wmsg);
            let mut cl = vec![0u32; WOTS_LEN];
            let mut rl = vec![0u32; WOTS_LEN];
            cl_c(cl.as_mut_ptr(), wmsg.as_ptr());
            cl_r(rl.as_mut_ptr(), wmsg.as_ptr());
            assert_eq!(cl, rl);
            let mut wsig = vec![0u8; WOTS_BYTES];
            rng.fill(&mut wsig);
            let mut cwpk = vec![0u8; WOTS_BYTES];
            let mut rwpk = vec![0u8; WOTS_BYTES];
            let (mut ca, mut ra) = (addr, addr);
            wpk_c(
                cwpk.as_mut_ptr(),
                wsig.as_ptr(),
                wmsg.as_ptr(),
                cc.as_ptr().cast(),
                ca.as_mut_ptr(),
            );
            wpk_r(
                rwpk.as_mut_ptr(),
                wsig.as_ptr(),
                wmsg.as_ptr(),
                rc.as_ptr().cast(),
                ra.as_mut_ptr(),
            );
            assert_eq!((cwpk, ca), (rwpk, ra));
            for height in [1usize, TREE_HEIGHT] {
                let mut leaf = vec![0u8; N];
                let mut auth = vec![0u8; height * N];
                rng.fill(&mut leaf);
                rng.fill(&mut auth);
                for leaf_idx in [0u32, 1, (1u32 << height.min(31)) - 1] {
                    let mut cr = vec![0u8; N];
                    let mut rr = vec![0u8; N];
                    let (mut ca, mut ra) = (addr, addr);
                    root_c(
                        cr.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        17,
                        auth.as_ptr(),
                        height as u32,
                        cc.as_ptr().cast(),
                        ca.as_mut_ptr(),
                    );
                    root_r(
                        rr.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        17,
                        auth.as_ptr(),
                        height as u32,
                        rc.as_ptr().cast(),
                        ra.as_mut_ptr(),
                    );
                    assert_eq!((cr, ca), (rr, ra));
                }
            }
            for idx in [0u32, 1, (1u32 << FORS_HEIGHT) - 1] {
                let mut cl = vec![0u8; N];
                let mut rl = vec![0u8; N];
                let (mut ci, mut ri) = (addr, addr);
                fg_c(cl.as_mut_ptr(), cc.as_ptr().cast(), idx, ci.as_mut_ptr().cast());
                fg_r(rl.as_mut_ptr(), rc.as_ptr().cast(), idx, ri.as_mut_ptr().cast());
                assert_eq!((cl, ci), (rl, ri));
            }
            if round == 0 {
                for height in [1usize, 2, TREE_HEIGHT] {
                    let leaf_idx = ((1usize << height) - 1).min(3) as u32;
                    let mut cr = vec![0u8; N];
                    let mut rr = vec![0u8; N];
                    let mut cauth = vec![0u8; height * N];
                    let mut rauth = vec![0u8; height * N];
                    let (mut ca, mut ra) = (addr, addr);
                    tree_c(
                        cr.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        cc.as_ptr().cast(),
                        leaf_idx,
                        5,
                        height as u32,
                        Some(deterministic_leaf),
                        ca.as_mut_ptr(),
                    );
                    tree_r(
                        rr.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        rc.as_ptr().cast(),
                        leaf_idx,
                        5,
                        height as u32,
                        Some(deterministic_leaf),
                        ra.as_mut_ptr(),
                    );
                    assert_eq!((cr, cauth, ca), (rr, rauth, ra));
                }
                let mut cmr = vec![0u8; N];
                let mut rmr = vec![0u8; N];
                merkle_root_c(cmr.as_mut_ptr(), cc.as_ptr().cast());
                merkle_root_r(rmr.as_mut_ptr(), rc.as_ptr().cast());
                assert_eq!(cmr, rmr);
                for idx in [0u32, 1, (1u32 << TREE_HEIGHT) - 1] {
                    let mut cs = vec![0u8; WOTS_BYTES + TREE_HEIGHT * N];
                    let mut rs = vec![0u8; WOTS_BYTES + TREE_HEIGHT * N];
                    let mut cr = vec![0u8; N];
                    let mut rr = vec![0u8; N];
                    let (mut cwa, mut rwa) = (addr, addr);
                    let (mut cta, mut rta) = (addr, addr);
                    merkle_c(
                        cs.as_mut_ptr(),
                        cr.as_mut_ptr(),
                        cc.as_ptr().cast(),
                        cwa.as_mut_ptr(),
                        cta.as_mut_ptr(),
                        idx,
                    );
                    merkle_r(
                        rs.as_mut_ptr(),
                        rr.as_mut_ptr(),
                        rc.as_ptr().cast(),
                        rwa.as_mut_ptr(),
                        rta.as_mut_ptr(),
                        idx,
                    );
                    assert_eq!((cs, cr, cwa, cta), (rs, rr, rwa, rta));
                }
            }
            if round < 2 {
                eprintln!("fors round {round}: start");
                let mut fmsg = vec![0u8; FORS_MSG_BYTES];
                rng.fill(&mut fmsg);
                let mut cs_buf = vec![0xa5u8; FORS_BYTES + 128];
                let mut rs_buf = vec![0xa5u8; FORS_BYTES + 128];
                let mut cpk_buf = vec![0xa5u8; N + 128];
                let mut rpk_buf = vec![0xa5u8; N + 128];
                let cs = &mut cs_buf[64..64 + FORS_BYTES];
                let rs = &mut rs_buf[64..64 + FORS_BYTES];
                let cpk = &mut cpk_buf[64..64 + N];
                let rpk = &mut rpk_buf[64..64 + N];
                fs_c(
                    cs.as_mut_ptr(),
                    cpk.as_mut_ptr(),
                    fmsg.as_ptr(),
                    cc.as_ptr().cast(),
                    addr.as_ptr(),
                );
                eprintln!("fors round {round}: C sign done");
                fs_r(
                    rs.as_mut_ptr(),
                    rpk.as_mut_ptr(),
                    fmsg.as_ptr(),
                    rc.as_ptr().cast(),
                    addr.as_ptr(),
                );
                eprintln!("fors round {round}: Rust sign done");
                assert!(cs_buf[..64].iter().all(|&b| b == 0xa5));
                assert!(cs_buf[64 + FORS_BYTES..].iter().all(|&b| b == 0xa5));
                assert!(rs_buf[..64].iter().all(|&b| b == 0xa5));
                assert!(rs_buf[64 + FORS_BYTES..].iter().all(|&b| b == 0xa5));
                assert_eq!((&cs_buf[64..64 + FORS_BYTES], &cpk_buf[64..64 + N]),
                    (&rs_buf[64..64 + FORS_BYTES], &rpk_buf[64..64 + N]));
                let mut cderived = vec![0u8; N];
                let mut rderived = vec![0u8; N];
                fp_c(
                    cderived.as_mut_ptr(),
                    cs_buf[64..].as_ptr(),
                    fmsg.as_ptr(),
                    cc.as_ptr().cast(),
                    addr.as_ptr(),
                );
                eprintln!("fors round {round}: C derive done");
                fp_r(
                    rderived.as_mut_ptr(),
                    rs_buf[64..].as_ptr(),
                    fmsg.as_ptr(),
                    rc.as_ptr().cast(),
                    addr.as_ptr(),
                );
                eprintln!("fors round {round}: Rust derive done");
                assert_eq!(
                    (cderived, &cpk_buf[64..64 + N]),
                    (rderived, &rpk_buf[64..64 + N])
                );
            }
        }
    }
}

#[test]
fn deterministic_rng_signing_and_errors_match() {
    let _guard = test_lock();
    unsafe {
        let libs = Libs::load();
        let (ri_c, ri_r): (unsafe extern "C" fn(*mut u8, *mut u8), _) =
            libs.pair(b"randombytes_init\0");
        let (rb_c, rb_r): (unsafe extern "C" fn(*mut u8, u64) -> c_int, _) =
            libs.pair(b"randombytes\0");
        let mut entropy = [0u8; 48];
        let mut personal = [0u8; 48];
        for i in 0..48 {
            entropy[i] = i as u8;
            personal[i] = (255 - i) as u8;
        }
        for p in [std::ptr::null_mut(), personal.as_mut_ptr()] {
            ri_c(entropy.as_mut_ptr(), p);
            ri_r(entropy.as_mut_ptr(), p);
            let mut co = [0u8; 65];
            let mut ro = [0u8; 65];
            assert_eq!(rb_c(co.as_mut_ptr(), 65), rb_r(ro.as_mut_ptr(), 65));
            assert_eq!(co, ro);
        }
        let (seed_kp_c, seed_kp_r): (
            unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign_seed_keypair\0");
        let (verify_c, verify_r): (
            unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign_verify\0");
        let (open_c, open_r): (
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign_open\0");
        let mut seed = vec![0u8; SEED_BYTES];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = (i * 17) as u8;
        }
        let mut cpk = vec![0u8; PK_BYTES];
        let mut rpk = vec![0u8; PK_BYTES];
        let mut csk = vec![0u8; SK_BYTES];
        let mut rsk = vec![0u8; SK_BYTES];
        assert_eq!(
            seed_kp_c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
            seed_kp_r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr())
        );
        assert_eq!(cpk, rpk);
        assert_eq!(csk, rsk);

        let (sign_c, sign_r): (
            unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign_signature\0");
        let (attached_c, attached_r): (
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign\0");
        let (keypair_c, keypair_r): (
            unsafe extern "C" fn(*mut u8, *mut u8) -> c_int,
            _,
        ) = libs.pair(b"crypto_sign_keypair\0");

        for len in [0usize, MESSAGE_BLOCK + 1] {
            let mut message = vec![0u8; len];
            for (i, byte) in message.iter_mut().enumerate() {
                *byte = (i as u8).wrapping_mul(29).wrapping_add(7);
            }
            ri_c(entropy.as_mut_ptr(), personal.as_mut_ptr());
            ri_r(entropy.as_mut_ptr(), personal.as_mut_ptr());
            let mut csig = vec![0u8; SIG_BYTES];
            let mut rsig = vec![0u8; SIG_BYTES];
            let (mut csl, mut rsl) = (0usize, 0usize);
            assert_eq!(
                sign_c(
                    csig.as_mut_ptr(),
                    &mut csl,
                    message.as_ptr(),
                    message.len(),
                    csk.as_ptr(),
                ),
                sign_r(
                    rsig.as_mut_ptr(),
                    &mut rsl,
                    message.as_ptr(),
                    message.len(),
                    rsk.as_ptr(),
                )
            );
            assert_eq!((csl, &csig), (rsl, &rsig));
            assert_eq!(
                verify_c(csig.as_ptr(), csl, message.as_ptr(), message.len(), cpk.as_ptr()),
                0
            );
            assert_eq!(
                verify_r(rsig.as_ptr(), rsl, message.as_ptr(), message.len(), rpk.as_ptr()),
                0
            );

            csig[N] ^= 1;
            rsig[N] ^= 1;
            assert_eq!(
                verify_c(csig.as_ptr(), csl, message.as_ptr(), message.len(), cpk.as_ptr()),
                verify_r(rsig.as_ptr(), rsl, message.as_ptr(), message.len(), rpk.as_ptr())
            );
        }

        let attached_message = [0x42u8; 7];
        ri_c(entropy.as_mut_ptr(), std::ptr::null_mut());
        ri_r(entropy.as_mut_ptr(), std::ptr::null_mut());
        let mut csm = vec![0u8; SIG_BYTES + attached_message.len()];
        let mut rsm = vec![0u8; SIG_BYTES + attached_message.len()];
        let (mut csml, mut rsml) = (0u64, 0u64);
        assert_eq!(
            attached_c(
                csm.as_mut_ptr(),
                &mut csml,
                attached_message.as_ptr(),
                attached_message.len() as u64,
                csk.as_ptr(),
            ),
            attached_r(
                rsm.as_mut_ptr(),
                &mut rsml,
                attached_message.as_ptr(),
                attached_message.len() as u64,
                rsk.as_ptr(),
            )
        );
        assert_eq!((csml, &csm), (rsml, &rsm));
        let mut copened = vec![0u8; csm.len()];
        let mut ropened = vec![0u8; rsm.len()];
        let (mut col, mut rol) = (0u64, 0u64);
        assert_eq!(
            open_c(copened.as_mut_ptr(), &mut col, csm.as_ptr(), csml, cpk.as_ptr()),
            open_r(ropened.as_mut_ptr(), &mut rol, rsm.as_ptr(), rsml, rpk.as_ptr())
        );
        assert_eq!((col, &copened[..col as usize]), (rol, &ropened[..rol as usize]));

        csm[0] ^= 1;
        rsm[0] ^= 1;
        copened.fill(0xa5);
        ropened.fill(0xa5);
        assert_eq!(
            open_c(copened.as_mut_ptr(), &mut col, csm.as_ptr(), csml, cpk.as_ptr()),
            open_r(ropened.as_mut_ptr(), &mut rol, rsm.as_ptr(), rsml, rpk.as_ptr())
        );
        assert_eq!((col, copened), (rol, ropened));

        ri_c(entropy.as_mut_ptr(), personal.as_mut_ptr());
        ri_r(entropy.as_mut_ptr(), personal.as_mut_ptr());
        let (mut cpk2, mut rpk2) = (vec![0u8; PK_BYTES], vec![0u8; PK_BYTES]);
        let (mut csk2, mut rsk2) = (vec![0u8; SK_BYTES], vec![0u8; SK_BYTES]);
        assert_eq!(
            keypair_c(cpk2.as_mut_ptr(), csk2.as_mut_ptr()),
            keypair_r(rpk2.as_mut_ptr(), rsk2.as_mut_ptr())
        );
        assert_eq!((cpk2, csk2), (rpk2, rsk2));
        let bogus = vec![0x5a; SIG_BYTES + 1];
        let msg = [0x33u8; 7];
        for len in [0, SIG_BYTES - 1, SIG_BYTES + 1] {
            assert_eq!(
                verify_c(bogus.as_ptr(), len, msg.as_ptr(), msg.len(), cpk.as_ptr()),
                verify_r(bogus.as_ptr(), len, msg.as_ptr(), msg.len(), rpk.as_ptr())
            );
        }
        for len in [0usize, 1, SIG_BYTES - 1] {
            let mut cm = vec![0xa5; len.max(1)];
            let mut rm = cm.clone();
            let (mut cml, mut rml) = (99, 99);
            let cr = open_c(
                cm.as_mut_ptr(),
                &mut cml,
                bogus.as_ptr(),
                len as u64,
                cpk.as_ptr(),
            );
            let rr = open_r(
                rm.as_mut_ptr(),
                &mut rml,
                bogus.as_ptr(),
                len as u64,
                rpk.as_ptr(),
            );
            assert_eq!((cr, cml, cm), (rr, rml, rm));
        }
        let (sei_c, sei_r): (
            unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, c_ulong) -> c_int,
            _,
        ) = libs.pair(b"seedexpander_init\0");
        let (se_c, se_r): (
            unsafe extern "C" fn(*mut AesXof, *mut u8, c_ulong) -> c_int,
            _,
        ) = libs.pair(b"seedexpander\0");
        let mut cx = AesXof::default();
        let mut rx = AesXof::default();
        let mut key = [7u8; 32];
        let mut div = [9u8; 8];
        assert_eq!(
            sei_c(&mut cx, key.as_mut_ptr(), div.as_mut_ptr(), 64),
            sei_r(&mut rx, key.as_mut_ptr(), div.as_mut_ptr(), 64)
        );
        if c_ulong::BITS > 32 {
            let before_c = cx.clone();
            let before_r = rx.clone();
            assert_eq!(
                sei_c(&mut cx, key.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000),
                -1
            );
            assert_eq!(
                sei_r(&mut rx, key.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000),
                -1
            );
            assert_eq!(cx, before_c);
            assert_eq!(rx, before_r);
        }
        assert_eq!(se_c(&mut cx, std::ptr::null_mut(), 1), -2);
        assert_eq!(se_r(&mut rx, std::ptr::null_mut(), 1), -2);
        assert_eq!(se_c(&mut cx, key.as_mut_ptr(), 64), -3);
        assert_eq!(se_r(&mut rx, key.as_mut_ptr(), 64), -3);

        let mut cx = AesXof::default();
        let mut rx = AesXof::default();
        assert_eq!(
            sei_c(&mut cx, key.as_mut_ptr(), div.as_mut_ptr(), 65),
            sei_r(&mut rx, key.as_mut_ptr(), div.as_mut_ptr(), 65)
        );
        for len in [0usize, 1, 15, 16, 17, 8] {
            let mut co = vec![0u8; len];
            let mut ro = vec![0u8; len];
            assert_eq!(
                se_c(&mut cx, co.as_mut_ptr(), len as c_ulong),
                se_r(&mut rx, ro.as_mut_ptr(), len as c_ulong)
            );
            assert_eq!((co, &cx), (ro, &rx));
        }
    }
}

#[test]
fn backend_public_primitives_match() {
    let _guard = test_lock();
    unsafe {
        let libs = Libs::load();
        let mut rng = Rng::new();

        #[cfg(feature = "blake")]
        {
            for (name, output_len) in [(b"blake256\0".as_slice(), 32usize), (b"blake512\0", 64)] {
                let (c, r): (
                    unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int,
                    _,
                ) = libs.pair(name);
                for len in [0usize, 1, 55, 56, 63, 64, 111, 112, 127, 128, 257] {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let mut co = vec![0u8; output_len];
                    let mut ro = vec![0u8; output_len];
                    assert_eq!(
                        c(co.as_mut_ptr(), input.as_ptr(), len as u64),
                        r(ro.as_mut_ptr(), input.as_ptr(), len as u64)
                    );
                    assert_eq!(co, ro);
                }
            }
        }

        #[cfg(feature = "sha2")]
        {
            for (name, output_len) in [(b"sha256\0".as_slice(), 32usize), (b"sha512\0", 64)] {
                let (c, r): (unsafe extern "C" fn(*mut u8, *const u8, usize), _) =
                    libs.pair(name);
                for len in [0usize, 1, 55, 56, 63, 64, 111, 112, 127, 128, 257] {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let mut co = vec![0u8; output_len];
                    let mut ro = vec![0u8; output_len];
                    c(co.as_mut_ptr(), input.as_ptr(), len);
                    r(ro.as_mut_ptr(), input.as_ptr(), len);
                    assert_eq!(co, ro);
                }
            }
        }

        #[cfg(feature = "shake")]
        {
            let (c, r): (
                unsafe extern "C" fn(*mut u8, usize, *const u8, usize),
                _,
            ) = libs.pair(b"shake256\0");
            for inlen in [0usize, 1, 135, 136, 137, 273] {
                let mut input = vec![0u8; inlen];
                rng.fill(&mut input);
                for outlen in [0usize, 1, 31, 32, 135, 136, 137, 271] {
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    c(co.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    r(ro.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    assert_eq!(co, ro);
                }
            }
        }

        #[cfg(feature = "haraka")]
        {
            let (mut cc, mut rc) = contexts(&mut rng);
            let (init_c, init_r): (unsafe extern "C" fn(*mut c_void), _) =
                libs.pair(b"SPX_initialize_hash_function\0");
            init_c(cc.as_mut_ptr().cast());
            init_r(rc.as_mut_ptr().cast());
            for (name, input_len, output_len) in [
                (b"SPX_haraka256\0".as_slice(), 32usize, 32usize),
                (b"SPX_haraka512\0".as_slice(), 64usize, 32usize),
                (b"SPX_haraka512_perm\0".as_slice(), 64usize, 64usize),
            ] {
                let (c, r): (
                    unsafe extern "C" fn(*mut u8, *const u8, *const c_void),
                    _,
                ) = libs.pair(name);
                let mut input = vec![0u8; input_len];
                rng.fill(&mut input);
                let mut co = vec![0u8; output_len];
                let mut ro = vec![0u8; output_len];
                c(co.as_mut_ptr(), input.as_ptr(), cc.as_ptr().cast());
                r(ro.as_mut_ptr(), input.as_ptr(), rc.as_ptr().cast());
                assert_eq!(co, ro);
            }
            let (c, r): (
                unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void),
                _,
            ) = libs.pair(b"SPX_haraka_S\0");
            for inlen in [0usize, 1, 31, 32, 33, 65] {
                let mut input = vec![0u8; inlen];
                rng.fill(&mut input);
                for outlen in [0usize, 1, 31, 32, 33, 65] {
                    let mut co = vec![0u8; outlen];
                    let mut ro = vec![0u8; outlen];
                    c(co.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64, cc.as_ptr().cast());
                    r(ro.as_mut_ptr(), outlen as u64, input.as_ptr(), inlen as u64, rc.as_ptr().cast());
                    assert_eq!(co, ro);
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AesXof {
    buffer: [u8; 16],
    buffer_pos: c_ulong,
    length_remaining: c_ulong,
    key: [u8; 32],
    ctr: [u8; 16],
}
