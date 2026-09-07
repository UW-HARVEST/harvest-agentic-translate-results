use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_LAZY, RTLD_NOW};
use std::{
    collections::BTreeSet,
    ffi::c_void,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(any(feature = "128s", feature = "128f"))]
const N: usize = 16;
#[cfg(any(feature = "192s", feature = "192f"))]
const N: usize = 24;
#[cfg(any(feature = "256s", feature = "256f"))]
const N: usize = 32;

#[cfg(feature = "128s")]
const PARAMS: (usize, usize, usize, usize) = (63, 7, 12, 14);
#[cfg(feature = "128f")]
const PARAMS: (usize, usize, usize, usize) = (66, 22, 6, 33);
#[cfg(feature = "192s")]
const PARAMS: (usize, usize, usize, usize) = (63, 7, 14, 17);
#[cfg(feature = "192f")]
const PARAMS: (usize, usize, usize, usize) = (66, 22, 8, 33);
#[cfg(feature = "256s")]
const PARAMS: (usize, usize, usize, usize) = (64, 8, 14, 22);
#[cfg(feature = "256f")]
const PARAMS: (usize, usize, usize, usize) = (68, 17, 9, 35);

const FULL_HEIGHT: usize = PARAMS.0;
const D: usize = PARAMS.1;
const FORS_HEIGHT: usize = PARAMS.2;
const FORS_TREES: usize = PARAMS.3;
const TREE_HEIGHT: usize = FULL_HEIGHT / D;
const WOTS_LEN: usize = 2 * N + 3;
const WOTS_BYTES: usize = WOTS_LEN * N;
const FORS_MSG_BYTES: usize = (FORS_HEIGHT * FORS_TREES + 7) / 8;
const FORS_BYTES: usize = (FORS_HEIGHT + 1) * FORS_TREES * N;
const SIG_BYTES: usize = N + FORS_BYTES + D * WOTS_BYTES + FULL_HEIGHT * N;
const PK_BYTES: usize = 2 * N;
const SK_BYTES: usize = 4 * N;
const SEED_BYTES: usize = 3 * N;

#[cfg(feature = "blake")]
const BACKEND: &str = "blake";
#[cfg(feature = "sha2")]
const BACKEND: &str = "sha2";
#[cfg(feature = "shake")]
const BACKEND: &str = "shake";
#[cfg(feature = "haraka")]
const BACKEND: &str = "haraka";

struct Libraries {
    backend: &'static Library,
    core: &'static Library,
    rust: &'static Library,
    backend_path: PathBuf,
    core_path: PathBuf,
    rust_path: PathBuf,
}

impl Libraries {
    unsafe fn load() -> Self {
        let dir = PathBuf::from(std::env::var_os("SPHINCS_C_DIR").expect("SPHINCS_C_DIR"));
        let rust_path =
            PathBuf::from(std::env::var_os("SPHINCS_RUST_SO").expect("SPHINCS_RUST_SO"));
        let backend_path = dir.join("lib").join(BACKEND).join(format!("lib{BACKEND}.so"));
        let core_path = dir.join("app").join("libsphincs_core.so");
        let backend = Box::leak(Box::new(
            unsafe { Library::open(Some(&backend_path), RTLD_LAZY | RTLD_GLOBAL) }.unwrap(),
        ));
        let core = Box::leak(Box::new(
            unsafe { Library::open(Some(&core_path), RTLD_NOW | RTLD_GLOBAL) }.unwrap(),
        ));
        let rust = Box::leak(Box::new(unsafe { Library::new(&rust_path) }.unwrap()));
        Self { backend, core, rust, backend_path, core_path, rust_path }
    }

    unsafe fn c<T: Copy>(&self, name: &[u8]) -> T {
        unsafe {
            self.core
                .get::<T>(name)
                .or_else(|_| self.backend.get::<T>(name))
                .map(|symbol| *symbol)
                .unwrap_or_else(|error| panic!("C symbol {:?}: {error}", name))
        }
    }

    unsafe fn r<T: Copy>(&self, name: &[u8]) -> T {
        unsafe { *self.rust.get::<T>(name).unwrap() }
    }
}

#[repr(C, align(8))]
struct AlignedContext([u8; 2048]);

impl AlignedContext {
    fn new(pub_seed: &[u8], sk_seed: &[u8]) -> Self {
        let mut result = Self([0; 2048]);
        result.0[..N].copy_from_slice(pub_seed);
        result.0[N..2 * N].copy_from_slice(sk_seed);
        result
    }

    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr().cast()
    }
}

fn context_size() -> usize {
    if cfg!(feature = "sha2") {
        2 * N + 40 + if N >= 24 { 72 } else { 0 }
    } else if cfg!(feature = "haraka") {
        2 * N + 10 * 8 * 8 + 10 * 8 * 4
    } else {
        2 * N
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next() as u8;
        }
    }
}

unsafe extern "C" fn test_gen_leaf(
    leaf: *mut u8,
    _ctx: *const c_void,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let addr = unsafe { std::slice::from_raw_parts(tree_addr, 8) };
    for i in 0..N {
        unsafe {
            *leaf.add(i) = (addr_idx as u8)
                .wrapping_mul(37)
                .wrapping_add(addr[i % 8] as u8)
                .wrapping_add(i as u8);
        }
    }
}

fn nm_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(2))
        .map(str::to_owned)
        .collect()
}

#[test]
fn symbol_parity() {
    let libs = unsafe { Libraries::load() };
    let mut c = nm_symbols(&libs.core_path);
    c.extend(nm_symbols(&libs.backend_path));
    let rust = nm_symbols(&libs.rust_path);
    let missing: Vec<_> = c.difference(&rust).cloned().collect();
    assert!(missing.is_empty(), "missing Rust exports: {missing:?}");
}

#[test]
fn sizes_utilities_addresses_and_chain_lengths() {
    let libs = unsafe { Libraries::load() };
    unsafe {
        for name_expected in [
            (b"crypto_sign_secretkeybytes\0".as_slice(), SK_BYTES as u64),
            (b"crypto_sign_publickeybytes\0".as_slice(), PK_BYTES as u64),
            (b"crypto_sign_bytes\0".as_slice(), SIG_BYTES as u64),
            (b"crypto_sign_seedbytes\0".as_slice(), SEED_BYTES as u64),
        ] {
            let c: unsafe extern "C" fn() -> u64 = libs.c(name_expected.0);
            let r: unsafe extern "C" fn() -> u64 = libs.r(name_expected.0);
            assert_eq!(c(), name_expected.1);
            assert_eq!(r(), name_expected.1);
        }

        let c_ull: unsafe extern "C" fn(*mut u8, u32, u64) = libs.c(b"SPX_ull_to_bytes\0");
        let r_ull: unsafe extern "C" fn(*mut u8, u32, u64) = libs.r(b"SPX_ull_to_bytes\0");
        let c_back: unsafe extern "C" fn(*const u8, u32) -> u64 = libs.c(b"SPX_bytes_to_ull\0");
        let r_back: unsafe extern "C" fn(*const u8, u32) -> u64 = libs.r(b"SPX_bytes_to_ull\0");
        let c_u32: unsafe extern "C" fn(*mut u8, u32) = libs.c(b"SPX_u32_to_bytes\0");
        let r_u32: unsafe extern "C" fn(*mut u8, u32) = libs.r(b"SPX_u32_to_bytes\0");
        for len in [0u32, 1, 4, 8] {
            for value in [0, 1, 0x80, u32::MAX as u64, u64::MAX] {
                let mut c_out = [0xa5; 8];
                let mut r_out = c_out;
                c_ull(c_out.as_mut_ptr(), len, value);
                r_ull(r_out.as_mut_ptr(), len, value);
                assert_eq!(c_out, r_out);
                assert_eq!(c_back(c_out.as_ptr(), len), r_back(r_out.as_ptr(), len));
            }
        }
        for value in [0u32, 1, 0x80, 0x8000_0000, u32::MAX] {
            let (mut c_out, mut r_out) = ([0u8; 4], [0u8; 4]);
            c_u32(c_out.as_mut_ptr(), value);
            r_u32(r_out.as_mut_ptr(), value);
            assert_eq!(c_out, r_out);
        }

        type Setter = unsafe extern "C" fn(*mut u32, u32);
        type TreeSetter = unsafe extern "C" fn(*mut u32, u64);
        type Copier = unsafe extern "C" fn(*mut u32, *const u32);
        let setters = [
            b"SPX_set_layer_addr\0".as_slice(),
            b"SPX_set_type\0".as_slice(),
            b"SPX_set_keypair_addr\0".as_slice(),
            b"SPX_set_chain_addr\0".as_slice(),
            b"SPX_set_hash_addr\0".as_slice(),
            b"SPX_set_tree_height\0".as_slice(),
            b"SPX_set_tree_index\0".as_slice(),
        ];
        let mut rng = Rng(0x4d595df4d0f33173);
        for name in setters {
            let c: Setter = libs.c(name);
            let r: Setter = libs.r(name);
            for value in [0, 1, 6, 7, 255, 256, u32::MAX] {
                let mut a = [0u32; 8];
                for word in &mut a {
                    *word = rng.next() as u32;
                }
                let mut b = a;
                c(a.as_mut_ptr(), value);
                r(b.as_mut_ptr(), value);
                assert_eq!(a, b, "{name:?} value={value}");
            }
        }
        let c_tree: TreeSetter = libs.c(b"SPX_set_tree_addr\0");
        let r_tree: TreeSetter = libs.r(b"SPX_set_tree_addr\0");
        for value in [0, 1, u32::MAX as u64, u64::MAX] {
            let mut a = [0xdeadbeefu32; 8];
            let mut b = a;
            c_tree(a.as_mut_ptr(), value);
            r_tree(b.as_mut_ptr(), value);
            assert_eq!(a, b);
        }
        for name in [b"SPX_copy_subtree_addr\0".as_slice(), b"SPX_copy_keypair_addr\0"] {
            let c: Copier = libs.c(name);
            let r: Copier = libs.r(name);
            let source: [u32; 8] = std::array::from_fn(|_| rng.next() as u32);
            let mut a = [0xaaaaaaaa; 8];
            let mut b = a;
            c(a.as_mut_ptr(), source.as_ptr());
            r(b.as_mut_ptr(), source.as_ptr());
            assert_eq!(a, b);
        }

        let c_chain: unsafe extern "C" fn(*mut u32, *const u8) =
            libs.c(b"SPX_chain_lengths\0");
        let r_chain: unsafe extern "C" fn(*mut u32, *const u8) =
            libs.r(b"SPX_chain_lengths\0");
        for shape in 0..67 {
            let mut msg = vec![0u8; N];
            match shape {
                0 => msg.fill(0),
                1 => msg.fill(0xff),
                2 => {
                    for (i, byte) in msg.iter_mut().enumerate() {
                        *byte = if i % 2 == 0 { 0xaa } else { 0x55 };
                    }
                }
                _ => rng.fill(&mut msg),
            }
            let mut a = vec![0u32; WOTS_LEN];
            let mut b = a.clone();
            c_chain(a.as_mut_ptr(), msg.as_ptr());
            r_chain(b.as_mut_ptr(), msg.as_ptr());
            assert_eq!(a, b);
        }
    }
}

#[test]
fn hash_and_thash_surface() {
    let libs = unsafe { Libraries::load() };
    unsafe {
        type Init = unsafe extern "C" fn(*mut c_void);
        type Prf = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
        type Gen = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            *const u8,
            *const u8,
            u64,
            *const c_void,
        );
        type Hmsg = unsafe extern "C" fn(
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
        let c_init: Init = libs.c(b"SPX_initialize_hash_function\0");
        let r_init: Init = libs.r(b"SPX_initialize_hash_function\0");
        let c_prf: Prf = libs.c(b"SPX_prf_addr\0");
        let r_prf: Prf = libs.r(b"SPX_prf_addr\0");
        let c_gen: Gen = libs.c(b"SPX_gen_message_random\0");
        let r_gen: Gen = libs.r(b"SPX_gen_message_random\0");
        let c_hmsg: Hmsg = libs.c(b"SPX_hash_message\0");
        let r_hmsg: Hmsg = libs.r(b"SPX_hash_message\0");
        let c_thash: Thash = libs.c(b"SPX_thash\0");
        let r_thash: Thash = libs.r(b"SPX_thash\0");
        let mut rng = Rng(0x9e3779b97f4a7c15);
        for mlen in [0usize, 1, 31, 32, 63, 64, 65, 127, 136, 137, 257] {
            let mut pub_seed = vec![0; N];
            let mut sk_seed = vec![0; N];
            let mut sk_prf = vec![0; N];
            let mut optrand = vec![0; N];
            let mut pk = vec![0; PK_BYTES];
            let mut msg = vec![0; mlen];
            rng.fill(&mut pub_seed);
            rng.fill(&mut sk_seed);
            rng.fill(&mut sk_prf);
            rng.fill(&mut optrand);
            rng.fill(&mut pk);
            rng.fill(&mut msg);
            let mut c_ctx = AlignedContext::new(&pub_seed, &sk_seed);
            let mut r_ctx = AlignedContext::new(&pub_seed, &sk_seed);
            c_init(c_ctx.ptr());
            r_init(r_ctx.ptr());
            assert_eq!(&c_ctx.0[..context_size()], &r_ctx.0[..context_size()]);
            let mut addr: [u32; 8] = std::array::from_fn(|_| rng.next() as u32);
            let gen_output = if cfg!(feature = "blake") {
                if N >= 24 { 64 } else { 32 }
            } else {
                N
            };
            let mut a = vec![0; gen_output];
            let mut b = vec![0; N];
            b.resize(gen_output, 0);
            c_prf(a.as_mut_ptr(), c_ctx.ptr(), addr.as_ptr());
            r_prf(b.as_mut_ptr(), r_ctx.ptr(), addr.as_ptr());
            assert_eq!(a, b);
            c_gen(
                a.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                msg.as_ptr(),
                mlen as u64,
                c_ctx.ptr(),
            );
            r_gen(
                b.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                msg.as_ptr(),
                mlen as u64,
                r_ctx.ptr(),
            );
            assert_eq!(a, b);
            let r_value = a[..N].to_vec();
            let mut da = vec![0; FORS_MSG_BYTES];
            let mut db = da.clone();
            let (mut ta, mut tb, mut la, mut lb) = (0, 0, 0, 0);
            c_hmsg(
                da.as_mut_ptr(),
                &mut ta,
                &mut la,
                r_value.as_ptr(),
                pk.as_ptr(),
                msg.as_ptr(),
                mlen as u64,
                c_ctx.ptr(),
            );
            r_hmsg(
                db.as_mut_ptr(),
                &mut tb,
                &mut lb,
                r_value.as_ptr(),
                pk.as_ptr(),
                msg.as_ptr(),
                mlen as u64,
                r_ctx.ptr(),
            );
            assert_eq!((da, ta, la), (db, tb, lb));
            for blocks in [1usize, 2, WOTS_LEN, FORS_TREES] {
                let mut input = vec![0; blocks * N];
                rng.fill(&mut input);
                a.fill(0);
                b.fill(0);
                c_thash(
                    a.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    c_ctx.ptr(),
                    addr.as_mut_ptr(),
                );
                r_thash(
                    b.as_mut_ptr(),
                    input.as_ptr(),
                    blocks as u32,
                    r_ctx.ptr(),
                    addr.as_mut_ptr(),
                );
                assert_eq!(a, b, "thash blocks={blocks}");
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
fn direct_low_level_leaf_tree_and_recovery_exports() {
    let libs = unsafe { Libraries::load() };
    unsafe {
        type Init = unsafe extern "C" fn(*mut c_void);
        type WotsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut LeafInfo);
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
        type ForsLeaf =
            unsafe extern "C" fn(*mut u8, *const c_void, u32, *mut [u32; 8]);
        type ForsTree = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            u32,
            u32,
            u32,
            *mut u32,
            *mut [u32; 8],
        );
        type Compute = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            u32,
            u32,
            *const u8,
            u32,
            *const c_void,
            *mut u32,
        );
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
        type WotsPk = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            *const u8,
            *const c_void,
            *mut u32,
        );
        type ForsSign = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const u8,
            *const c_void,
            *const u32,
        );
        type ForsPk = unsafe extern "C" fn(
            *mut u8,
            *const u8,
            *const u8,
            *const c_void,
            *const u32,
        );
        type MerkleSign = unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *const c_void,
            *mut u32,
            *const u32,
            u32,
        );
        type MerkleRoot = unsafe extern "C" fn(*mut u8, *const c_void);
        let c_init: Init = libs.c(b"SPX_initialize_hash_function\0");
        let r_init: Init = libs.r(b"SPX_initialize_hash_function\0");
        let c_wleaf: WotsLeaf = libs.c(b"SPX_wots_gen_leafx1\0");
        let r_wleaf: WotsLeaf = libs.r(b"SPX_wots_gen_leafx1\0");
        let c_wtree: WotsTree = libs.c(b"SPX_wots_treehashx1\0");
        let r_wtree: WotsTree = libs.r(b"SPX_wots_treehashx1\0");
        let c_fleaf: ForsLeaf = libs.c(b"SPX_fors_gen_leafx1\0");
        let r_fleaf: ForsLeaf = libs.r(b"SPX_fors_gen_leafx1\0");
        let c_ftree: ForsTree = libs.c(b"SPX_fors_treehashx1\0");
        let r_ftree: ForsTree = libs.r(b"SPX_fors_treehashx1\0");
        let c_compute: Compute = libs.c(b"SPX_compute_root\0");
        let r_compute: Compute = libs.r(b"SPX_compute_root\0");
        let c_treehash: Treehash = libs.c(b"SPX_treehash\0");
        let r_treehash: Treehash = libs.r(b"SPX_treehash\0");
        let c_wpk: WotsPk = libs.c(b"SPX_wots_pk_from_sig\0");
        let r_wpk: WotsPk = libs.r(b"SPX_wots_pk_from_sig\0");
        let c_fsign: ForsSign = libs.c(b"SPX_fors_sign\0");
        let r_fsign: ForsSign = libs.r(b"SPX_fors_sign\0");
        let c_fpk: ForsPk = libs.c(b"SPX_fors_pk_from_sig\0");
        let r_fpk: ForsPk = libs.r(b"SPX_fors_pk_from_sig\0");
        let c_merkle: MerkleSign = libs.c(b"SPX_merkle_sign\0");
        let r_merkle: MerkleSign = libs.r(b"SPX_merkle_sign\0");
        let c_merkle_root: MerkleRoot = libs.c(b"SPX_merkle_gen_root\0");
        let r_merkle_root: MerkleRoot = libs.r(b"SPX_merkle_gen_root\0");
        let mut rng = Rng(0x243f6a8885a308d3);
        let mut pub_seed = vec![0u8; N];
        let mut sk_seed = vec![0u8; N];
        rng.fill(&mut pub_seed);
        rng.fill(&mut sk_seed);
        let mut cc = AlignedContext::new(&pub_seed, &sk_seed);
        let mut rc = AlignedContext::new(&pub_seed, &sk_seed);
        c_init(cc.ptr());
        r_init(rc.ptr());

        let mut c_steps: Vec<u32> = (0..WOTS_LEN).map(|i| (i % 16) as u32).collect();
        let mut r_steps = c_steps.clone();
        let mut c_sig = vec![0xa5; WOTS_BYTES];
        let mut r_sig = c_sig.clone();
        let base_addr: [u32; 8] = std::array::from_fn(|_| rng.next() as u32);
        let mut ci = LeafInfo {
            wots_sig: c_sig.as_mut_ptr(),
            wots_sign_leaf: 3,
            wots_steps: c_steps.as_mut_ptr(),
            leaf_addr: base_addr,
            pk_addr: base_addr,
        };
        let mut ri = LeafInfo {
            wots_sig: r_sig.as_mut_ptr(),
            wots_sign_leaf: 3,
            wots_steps: r_steps.as_mut_ptr(),
            leaf_addr: base_addr,
            pk_addr: base_addr,
        };
        let (mut ca, mut ra) = (vec![0u8; N], vec![0u8; N]);
        c_wleaf(ca.as_mut_ptr(), cc.ptr(), 3, &mut ci);
        r_wleaf(ra.as_mut_ptr(), rc.ptr(), 3, &mut ri);
        assert_eq!(
            (ca, c_sig.as_slice(), ci.leaf_addr, ci.pk_addr),
            (ra, r_sig.as_slice(), ri.leaf_addr, ri.pk_addr),
        );
        let (mut c_no_sig, mut r_no_sig) = (vec![0xa5; WOTS_BYTES], vec![0xa5; WOTS_BYTES]);
        let mut ci_no_sig = LeafInfo {
            wots_sig: c_no_sig.as_mut_ptr(),
            wots_sign_leaf: u32::MAX,
            wots_steps: c_steps.as_mut_ptr(),
            leaf_addr: base_addr,
            pk_addr: base_addr,
        };
        let mut ri_no_sig = LeafInfo {
            wots_sig: r_no_sig.as_mut_ptr(),
            wots_sign_leaf: u32::MAX,
            wots_steps: r_steps.as_mut_ptr(),
            leaf_addr: base_addr,
            pk_addr: base_addr,
        };
        let (mut ca, mut ra) = (vec![0u8; N], vec![0u8; N]);
        c_wleaf(ca.as_mut_ptr(), cc.ptr(), 4, &mut ci_no_sig);
        r_wleaf(ra.as_mut_ptr(), rc.ptr(), 4, &mut ri_no_sig);
        assert_eq!(
            (ca, c_no_sig, ci_no_sig.leaf_addr, ci_no_sig.pk_addr),
            (ra, r_no_sig, ri_no_sig.leaf_addr, ri_no_sig.pk_addr),
        );

        let mut c_finfo = base_addr;
        let mut r_finfo = base_addr;
        let (mut ca, mut ra) = (vec![0u8; N], vec![0u8; N]);
        c_fleaf(ca.as_mut_ptr(), cc.ptr(), 9, &mut c_finfo);
        r_fleaf(ra.as_mut_ptr(), rc.ptr(), 9, &mut r_finfo);
        assert_eq!((ca, c_finfo), (ra, r_finfo));

        let height = 2u32;
        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        let (mut cap, mut rap) = (vec![0u8; height as usize * N], vec![0u8; height as usize * N]);
        let mut ctree_addr = base_addr;
        let mut rtree_addr = base_addr;
        c_wtree(
            croot.as_mut_ptr(),
            cap.as_mut_ptr(),
            cc.ptr(),
            1,
            0,
            height,
            ctree_addr.as_mut_ptr(),
            &mut ci,
        );
        r_wtree(
            rroot.as_mut_ptr(),
            rap.as_mut_ptr(),
            rc.ptr(),
            1,
            0,
            height,
            rtree_addr.as_mut_ptr(),
            &mut ri,
        );
        assert_eq!((croot, cap, ctree_addr), (rroot, rap, rtree_addr));

        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        let (mut cap, mut rap) = (vec![0u8; height as usize * N], vec![0u8; height as usize * N]);
        let mut ctree_addr = base_addr;
        let mut rtree_addr = base_addr;
        c_ftree(
            croot.as_mut_ptr(),
            cap.as_mut_ptr(),
            cc.ptr(),
            2,
            4,
            height,
            ctree_addr.as_mut_ptr(),
            &mut c_finfo,
        );
        r_ftree(
            rroot.as_mut_ptr(),
            rap.as_mut_ptr(),
            rc.ptr(),
            2,
            4,
            height,
            rtree_addr.as_mut_ptr(),
            &mut r_finfo,
        );
        assert_eq!((croot, cap, ctree_addr), (rroot, rap, rtree_addr));

        for (leaf_idx, idx_offset, height) in [(0u32, 0u32, 1u32), (1, 8, TREE_HEIGHT as u32)] {
            let mut leaf = vec![0u8; N];
            let mut auth = vec![0u8; height as usize * N];
            rng.fill(&mut leaf);
            rng.fill(&mut auth);
            let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
            let mut ca_addr = base_addr;
            let mut ra_addr = base_addr;
            c_compute(
                croot.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                idx_offset,
                auth.as_ptr(),
                height,
                cc.ptr(),
                ca_addr.as_mut_ptr(),
            );
            r_compute(
                rroot.as_mut_ptr(),
                leaf.as_ptr(),
                leaf_idx,
                idx_offset,
                auth.as_ptr(),
                height,
                rc.ptr(),
                ra_addr.as_mut_ptr(),
            );
            assert_eq!((croot, ca_addr), (rroot, ra_addr));
        }

        for leaf_idx in [2u32, 3] {
            let height = 3u32;
            let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
            let (mut cap, mut rap) =
                (vec![0u8; height as usize * N], vec![0u8; height as usize * N]);
            let mut ca_addr = base_addr;
            let mut ra_addr = base_addr;
            c_treehash(
                croot.as_mut_ptr(),
                cap.as_mut_ptr(),
                cc.ptr(),
                leaf_idx,
                8,
                height,
                Some(test_gen_leaf),
                ca_addr.as_mut_ptr(),
            );
            r_treehash(
                rroot.as_mut_ptr(),
                rap.as_mut_ptr(),
                rc.ptr(),
                leaf_idx,
                8,
                height,
                Some(test_gen_leaf),
                ra_addr.as_mut_ptr(),
            );
            assert_eq!((croot, cap, ca_addr), (rroot, rap, ra_addr));
        }

        for shape in 0..3 {
            let mut raw_sig = vec![0u8; WOTS_BYTES];
            let mut msg = vec![0u8; N];
            rng.fill(&mut raw_sig);
            match shape {
                0 => msg.fill(0),
                1 => msg.fill(0xff),
                _ => rng.fill(&mut msg),
            }
            let (mut cpk, mut rpk) = (vec![0u8; WOTS_BYTES], vec![0u8; WOTS_BYTES]);
            let mut ca_addr = base_addr;
            let mut ra_addr = base_addr;
            c_wpk(cpk.as_mut_ptr(), raw_sig.as_ptr(), msg.as_ptr(), cc.ptr(), ca_addr.as_mut_ptr());
            r_wpk(rpk.as_mut_ptr(), raw_sig.as_ptr(), msg.as_ptr(), rc.ptr(), ra_addr.as_mut_ptr());
            assert_eq!((cpk, ca_addr), (rpk, ra_addr));
        }

        for shape in 0..4 {
            let mut fmsg = vec![0u8; FORS_MSG_BYTES];
            match shape {
                0 => fmsg.fill(0),
                1 => fmsg.fill(0xff),
                2 => {
                    for (i, byte) in fmsg.iter_mut().enumerate() {
                        *byte = if i % 2 == 0 { 0xaa } else { 0x55 };
                    }
                }
                _ => rng.fill(&mut fmsg),
            }
            let (mut csig, mut rsig) = (vec![0u8; FORS_BYTES], vec![0u8; FORS_BYTES]);
            let (mut cpk, mut rpk) = (vec![0u8; N], vec![0u8; N]);
            c_fsign(csig.as_mut_ptr(), cpk.as_mut_ptr(), fmsg.as_ptr(), cc.ptr(), base_addr.as_ptr());
            r_fsign(rsig.as_mut_ptr(), rpk.as_mut_ptr(), fmsg.as_ptr(), rc.ptr(), base_addr.as_ptr());
            assert_eq!((csig.as_slice(), cpk.as_slice()), (rsig.as_slice(), rpk.as_slice()));
            let (mut cr, mut rr) = (vec![0u8; N], vec![0u8; N]);
            c_fpk(cr.as_mut_ptr(), csig.as_ptr(), fmsg.as_ptr(), cc.ptr(), base_addr.as_ptr());
            r_fpk(rr.as_mut_ptr(), rsig.as_ptr(), fmsg.as_ptr(), rc.ptr(), base_addr.as_ptr());
            assert_eq!(cr, rr);
            assert_eq!(cr, cpk);
        }

        let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
        c_merkle_root(croot.as_mut_ptr(), cc.ptr());
        r_merkle_root(rroot.as_mut_ptr(), rc.ptr());
        assert_eq!(croot, rroot);
        for leaf_idx in [0u32, (1u32 << TREE_HEIGHT) - 1] {
            let (mut csig, mut rsig) = (
                vec![0u8; WOTS_BYTES + TREE_HEIGHT * N],
                vec![0u8; WOTS_BYTES + TREE_HEIGHT * N],
            );
            let (mut croot, mut rroot) = (vec![0u8; N], vec![0u8; N]);
            let mut c_wots_addr = base_addr;
            let mut r_wots_addr = base_addr;
            let c_tree_addr = base_addr;
            let r_tree_addr = base_addr;
            c_merkle(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                cc.ptr(),
                c_wots_addr.as_mut_ptr(),
                c_tree_addr.as_ptr(),
                leaf_idx,
            );
            r_merkle(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                rc.ptr(),
                r_wots_addr.as_mut_ptr(),
                r_tree_addr.as_ptr(),
                leaf_idx,
            );
            assert_eq!(
                (csig, croot, c_wots_addr),
                (rsig, rroot, r_wots_addr),
            );
        }
    }
}

#[test]
fn backend_primitive_exports() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Rng(0x13198a2e03707344);

        #[cfg(feature = "blake")]
        {
            type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
            type Init = unsafe extern "C" fn(*mut c_void);
            type Update = unsafe extern "C" fn(*mut c_void, *const u8, u64);
            type Final = unsafe extern "C" fn(*mut c_void, *mut u8);
            type Compress = unsafe extern "C" fn(*mut c_void, *const u8);
            type Mgf = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
            for (bits, state_words, block, output, one_name, init_name, update_name, final_name, compress_name, mgf_name) in [
                (256usize, 16usize, 64usize, 32usize, b"blake256\0".as_slice(), b"blake256_init\0".as_slice(), b"blake256_update\0".as_slice(), b"blake256_final\0".as_slice(), b"blake256_compress\0".as_slice(), b"SPX_blake256_mgf1\0".as_slice()),
                (512, 31, 128, 64, b"blake512\0", b"blake512_init\0", b"blake512_update\0", b"blake512_final\0", b"blake512_compress\0", b"SPX_blake512_mgf1\0"),
            ] {
                let c_one: OneShot = libs.c(one_name);
                let r_one: OneShot = libs.r(one_name);
                let c_init: Init = libs.c(init_name);
                let r_init: Init = libs.r(init_name);
                let c_update: Update = libs.c(update_name);
                let r_update: Update = libs.r(update_name);
                let c_final: Final = libs.c(final_name);
                let r_final: Final = libs.r(final_name);
                let c_compress: Compress = libs.c(compress_name);
                let r_compress: Compress = libs.r(compress_name);
                let c_mgf: Mgf = libs.c(mgf_name);
                let r_mgf: Mgf = libs.r(mgf_name);
                for len in [0usize, 1, block - 9, block - 8, block - 1, block, block + 1, 2 * block + 17] {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let (mut co, mut ro) = (vec![0u8; output], vec![0u8; output]);
                    assert_eq!(c_one(co.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                    assert_eq!(r_one(ro.as_mut_ptr(), input.as_ptr(), len as u64), 0);
                    assert_eq!(co, ro, "BLAKE-{bits} one-shot len={len}");

                    let (mut cs, mut rs) = (vec![0u64; state_words], vec![0u64; state_words]);
                    c_init(cs.as_mut_ptr().cast());
                    r_init(rs.as_mut_ptr().cast());
                    let split = len.min(block / 3);
                    c_update(cs.as_mut_ptr().cast(), input.as_ptr(), (split * 8) as u64);
                    r_update(rs.as_mut_ptr().cast(), input.as_ptr(), (split * 8) as u64);
                    c_update(cs.as_mut_ptr().cast(), input.as_ptr().add(split), ((len - split) * 8) as u64);
                    r_update(rs.as_mut_ptr().cast(), input.as_ptr().add(split), ((len - split) * 8) as u64);
                    assert_eq!(cs, rs, "BLAKE-{bits} incremental state len={len}");
                    co.fill(0);
                    ro.fill(0);
                    c_final(cs.as_mut_ptr().cast(), co.as_mut_ptr());
                    r_final(rs.as_mut_ptr().cast(), ro.as_mut_ptr());
                    assert_eq!(co, ro, "BLAKE-{bits} incremental final len={len}");
                }
                let mut block_input = vec![0u8; block];
                rng.fill(&mut block_input);
                let (mut cs, mut rs) = (vec![0u64; state_words], vec![0u64; state_words]);
                c_init(cs.as_mut_ptr().cast());
                r_init(rs.as_mut_ptr().cast());
                c_compress(cs.as_mut_ptr().cast(), block_input.as_ptr());
                r_compress(rs.as_mut_ptr().cast(), block_input.as_ptr());
                assert_eq!(cs, rs, "BLAKE-{bits} compress");
                for outlen in [0usize, 1, output - 1, output, output + 1, 2 * output + 3] {
                    let mut seed = vec![0u8; 19];
                    rng.fill(&mut seed);
                    let (mut co, mut ro) = (vec![0u8; outlen], vec![0u8; outlen]);
                    c_mgf(co.as_mut_ptr(), outlen, seed.as_ptr(), seed.len());
                    r_mgf(ro.as_mut_ptr(), outlen, seed.as_ptr(), seed.len());
                    assert_eq!(co, ro, "BLAKE-{bits} MGF outlen={outlen}");
                }
            }
            let c_cst: *const [u64; 16] = *libs.backend.get(b"cst\0").unwrap();
            let r_cst: *const [u64; 16] = *libs.rust.get(b"cst\0").unwrap();
            assert_eq!(*c_cst, *r_cst);
        }

        #[cfg(feature = "sha2")]
        {
            type OneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
            type Init = unsafe extern "C" fn(*mut u8);
            type Blocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
            type Final = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
            type Mgf = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
            for (width, state_len, block, output, one_name, init_name, blocks_name, final_name, mgf_name) in [
                (256usize, 40usize, 64usize, 32usize, b"sha256\0".as_slice(), b"sha256_inc_init\0".as_slice(), b"sha256_inc_blocks\0".as_slice(), b"sha256_inc_finalize\0".as_slice(), b"SPX_mgf1_256\0".as_slice()),
                (512, 72, 128, 64, b"sha512\0", b"sha512_inc_init\0", b"sha512_inc_blocks\0", b"sha512_inc_finalize\0", b"SPX_mgf1_512\0"),
            ] {
                let c_one: OneShot = libs.c(one_name);
                let r_one: OneShot = libs.r(one_name);
                let c_init: Init = libs.c(init_name);
                let r_init: Init = libs.r(init_name);
                let c_blocks: Blocks = libs.c(blocks_name);
                let r_blocks: Blocks = libs.r(blocks_name);
                let c_final: Final = libs.c(final_name);
                let r_final: Final = libs.r(final_name);
                let c_mgf: Mgf = libs.c(mgf_name);
                let r_mgf: Mgf = libs.r(mgf_name);
                for len in [0usize, 1, block - 9, block - 8, block - 1, block, block + 1, 2 * block + 17] {
                    let mut input = vec![0u8; len];
                    rng.fill(&mut input);
                    let (mut co, mut ro) = (vec![0u8; output], vec![0u8; output]);
                    c_one(co.as_mut_ptr(), input.as_ptr(), len);
                    r_one(ro.as_mut_ptr(), input.as_ptr(), len);
                    assert_eq!(co, ro, "SHA-{width} one-shot len={len}");
                    let full = len / block;
                    let consumed = full * block;
                    let (mut cs, mut rs) = (vec![0u8; state_len], vec![0u8; state_len]);
                    c_init(cs.as_mut_ptr());
                    r_init(rs.as_mut_ptr());
                    c_blocks(cs.as_mut_ptr(), input.as_ptr(), full);
                    r_blocks(rs.as_mut_ptr(), input.as_ptr(), full);
                    assert_eq!(cs, rs, "SHA-{width} blocks len={len}");
                    c_final(co.as_mut_ptr(), cs.as_mut_ptr(), input.as_ptr().add(consumed), len - consumed);
                    r_final(ro.as_mut_ptr(), rs.as_mut_ptr(), input.as_ptr().add(consumed), len - consumed);
                    assert_eq!(co, ro, "SHA-{width} incremental len={len}");
                }
                for outlen in [0usize, 1, output - 1, output, output + 1, 2 * output + 3] {
                    let mut seed = vec![0u8; 19];
                    rng.fill(&mut seed);
                    let (mut co, mut ro) = (vec![0u8; outlen], vec![0u8; outlen]);
                    c_mgf(co.as_mut_ptr(), outlen, seed.as_ptr(), seed.len());
                    r_mgf(ro.as_mut_ptr(), outlen, seed.as_ptr(), seed.len());
                    assert_eq!(co, ro, "SHA-{width} MGF outlen={outlen}");
                }
            }
        }

        #[cfg(feature = "shake")]
        {
            type OneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
            type Init = unsafe extern "C" fn(*mut u64);
            type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
            type Final = unsafe extern "C" fn(*mut u64);
            type Squeeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
            let c_one: OneShot = libs.c(b"shake256\0");
            let r_one: OneShot = libs.r(b"shake256\0");
            let c_init: Init = libs.c(b"shake256_inc_init\0");
            let r_init: Init = libs.r(b"shake256_inc_init\0");
            let c_absorb: Absorb = libs.c(b"shake256_inc_absorb\0");
            let r_absorb: Absorb = libs.r(b"shake256_inc_absorb\0");
            let c_final: Final = libs.c(b"shake256_inc_finalize\0");
            let r_final: Final = libs.r(b"shake256_inc_finalize\0");
            let c_squeeze: Squeeze = libs.c(b"shake256_inc_squeeze\0");
            let r_squeeze: Squeeze = libs.r(b"shake256_inc_squeeze\0");
            for len in [0usize, 1, 135, 136, 137, 273] {
                let mut input = vec![0u8; len];
                rng.fill(&mut input);
                for outlen in [0usize, 1, 135, 136, 137, 277] {
                    let (mut co, mut ro) = (vec![0u8; outlen], vec![0u8; outlen]);
                    c_one(co.as_mut_ptr(), outlen, input.as_ptr(), len);
                    r_one(ro.as_mut_ptr(), outlen, input.as_ptr(), len);
                    assert_eq!(co, ro, "SHAKE256 one-shot in={len} out={outlen}");
                    let (mut cs, mut rs) = ([0u64; 26], [0u64; 26]);
                    c_init(cs.as_mut_ptr());
                    r_init(rs.as_mut_ptr());
                    let split = len.min(37);
                    c_absorb(cs.as_mut_ptr(), input.as_ptr(), split);
                    r_absorb(rs.as_mut_ptr(), input.as_ptr(), split);
                    c_absorb(cs.as_mut_ptr(), input.as_ptr().add(split), len - split);
                    r_absorb(rs.as_mut_ptr(), input.as_ptr().add(split), len - split);
                    c_final(cs.as_mut_ptr());
                    r_final(rs.as_mut_ptr());
                    c_squeeze(co.as_mut_ptr(), outlen, cs.as_mut_ptr());
                    r_squeeze(ro.as_mut_ptr(), outlen, rs.as_mut_ptr());
                    assert_eq!(co, ro, "SHAKE256 incremental in={len} out={outlen}");
                }
            }
        }

        #[cfg(feature = "haraka")]
        {
            type InitHash = unsafe extern "C" fn(*mut c_void);
            type Hash = unsafe extern "C" fn(*mut u8, *const u8, *const c_void);
            type Sponge = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const c_void);
            type IncInit = unsafe extern "C" fn(*mut u8);
            type IncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const c_void);
            type IncFinal = unsafe extern "C" fn(*mut u8);
            type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const c_void);
            let mut seed = vec![0u8; N];
            rng.fill(&mut seed);
            let mut cc = AlignedContext::new(&seed, &seed);
            let mut rc = AlignedContext::new(&seed, &seed);
            let c_init_hash: InitHash = libs.c(b"SPX_initialize_hash_function\0");
            let r_init_hash: InitHash = libs.r(b"SPX_initialize_hash_function\0");
            c_init_hash(cc.ptr());
            r_init_hash(rc.ptr());
            assert_eq!(&cc.0[..context_size()], &rc.0[..context_size()]);
            for (name, input_len, output_len) in [
                (b"SPX_haraka256\0".as_slice(), 32usize, 32usize),
                (b"SPX_haraka512\0".as_slice(), 64, 32),
                (b"SPX_haraka512_perm\0".as_slice(), 64, 64),
            ] {
                let c_hash: Hash = libs.c(name);
                let r_hash: Hash = libs.r(name);
                let mut input = vec![0u8; input_len];
                rng.fill(&mut input);
                let (mut co, mut ro) = (vec![0u8; output_len], vec![0u8; output_len]);
                c_hash(co.as_mut_ptr(), input.as_ptr(), cc.ptr());
                r_hash(ro.as_mut_ptr(), input.as_ptr(), rc.ptr());
                assert_eq!(co, ro);
            }
            let c_sponge: Sponge = libs.c(b"SPX_haraka_S\0");
            let r_sponge: Sponge = libs.r(b"SPX_haraka_S\0");
            let c_inc_init: IncInit = libs.c(b"SPX_haraka_S_inc_init\0");
            let r_inc_init: IncInit = libs.r(b"SPX_haraka_S_inc_init\0");
            let c_inc_absorb: IncAbsorb = libs.c(b"SPX_haraka_S_inc_absorb\0");
            let r_inc_absorb: IncAbsorb = libs.r(b"SPX_haraka_S_inc_absorb\0");
            let c_inc_final: IncFinal = libs.c(b"SPX_haraka_S_inc_finalize\0");
            let r_inc_final: IncFinal = libs.r(b"SPX_haraka_S_inc_finalize\0");
            let c_inc_squeeze: IncSqueeze = libs.c(b"SPX_haraka_S_inc_squeeze\0");
            let r_inc_squeeze: IncSqueeze = libs.r(b"SPX_haraka_S_inc_squeeze\0");
            for len in [0usize, 1, 31, 32, 33, 97] {
                let mut input = vec![0u8; len];
                rng.fill(&mut input);
                for outlen in [0usize, 1, 31, 32, 33, 79] {
                    let (mut co, mut ro) = (vec![0u8; outlen], vec![0u8; outlen]);
                    c_sponge(co.as_mut_ptr(), outlen as u64, input.as_ptr(), len as u64, cc.ptr());
                    r_sponge(ro.as_mut_ptr(), outlen as u64, input.as_ptr(), len as u64, rc.ptr());
                    assert_eq!(co, ro, "Haraka sponge in={len} out={outlen}");
                    let (mut cs, mut rs) = ([0u8; 65], [0u8; 65]);
                    c_inc_init(cs.as_mut_ptr());
                    r_inc_init(rs.as_mut_ptr());
                    let split = len.min(13);
                    c_inc_absorb(cs.as_mut_ptr(), input.as_ptr(), split, cc.ptr());
                    r_inc_absorb(rs.as_mut_ptr(), input.as_ptr(), split, rc.ptr());
                    c_inc_absorb(cs.as_mut_ptr(), input.as_ptr().add(split), len - split, cc.ptr());
                    r_inc_absorb(rs.as_mut_ptr(), input.as_ptr().add(split), len - split, rc.ptr());
                    c_inc_final(cs.as_mut_ptr());
                    r_inc_final(rs.as_mut_ptr());
                    c_inc_squeeze(co.as_mut_ptr(), outlen, cs.as_mut_ptr(), cc.ptr());
                    r_inc_squeeze(ro.as_mut_ptr(), outlen, rs.as_mut_ptr(), rc.ptr());
                    assert_eq!(co, ro, "Haraka incremental in={len} out={outlen}");
                }
            }
        }
    }
}

#[test]
fn deterministic_rng_attached_and_overlap_api() {
    const RTLD_DEEPBIND: i32 = 0x00008;
    let det_core =
        PathBuf::from(std::env::var_os("SPHINCS_C_DET_CORE").expect("SPHINCS_C_DET_CORE"));
    let det_backend =
        PathBuf::from(std::env::var_os("SPHINCS_C_DET_BACKEND").expect("SPHINCS_C_DET_BACKEND"));
    let rust_path =
        PathBuf::from(std::env::var_os("SPHINCS_RUST_SO").expect("SPHINCS_RUST_SO"));
    unsafe {
        let _crypto = Box::leak(Box::new(
            Library::open(Some("libcrypto.so.3"), RTLD_NOW | RTLD_GLOBAL).unwrap(),
        ));
        let _backend = Box::leak(Box::new(
            Library::open(Some(det_backend), RTLD_LAZY | RTLD_GLOBAL).unwrap(),
        ));
        let c = Box::leak(Box::new(
            Library::open(
                Some(det_core),
                RTLD_NOW | RTLD_GLOBAL | RTLD_DEEPBIND,
            )
            .unwrap(),
        ));
        let r = Box::leak(Box::new(Library::new(rust_path).unwrap()));
        type RngInit = unsafe extern "C" fn(*mut u8, *mut u8);
        type Random = unsafe extern "C" fn(*mut u8, u64) -> i32;
        type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
        type Sign = unsafe extern "C" fn(
            *mut u8,
            *mut u64,
            *const u8,
            u64,
            *const u8,
        ) -> i32;
        type Open =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
        let c_init: RngInit = *c.get(b"randombytes_init\0").unwrap();
        let r_init: RngInit = *r.get(b"randombytes_init\0").unwrap();
        let c_random: Random = *c.get(b"randombytes\0").unwrap();
        let r_random: Random = *r.get(b"randombytes\0").unwrap();
        let c_key: SeedKeypair = *c.get(b"crypto_sign_seed_keypair\0").unwrap();
        let r_key: SeedKeypair = *r.get(b"crypto_sign_seed_keypair\0").unwrap();
        let c_sign: Sign = *c.get(b"crypto_sign\0").unwrap();
        let r_sign: Sign = *r.get(b"crypto_sign\0").unwrap();
        let c_open: Open = *c.get(b"crypto_sign_open\0").unwrap();
        let r_open: Open = *r.get(b"crypto_sign_open\0").unwrap();
        let mut entropy: [u8; 48] =
            std::array::from_fn(|i| (i as u8).wrapping_mul(29).wrapping_add(7));
        let mut personalization: [u8; 48] =
            std::array::from_fn(|i| (i as u8).wrapping_mul(13).wrapping_add(3));
        for len in [0usize, 1, 15, 16, 17, 79] {
            c_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
            r_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
            let (mut co, mut ro) = (vec![0u8; len], vec![0u8; len]);
            assert_eq!(c_random(co.as_mut_ptr(), len as u64), 0);
            assert_eq!(r_random(ro.as_mut_ptr(), len as u64), 0);
            assert_eq!(co, ro, "randombytes len={len}");
        }

        let seed: Vec<u8> = (0..SEED_BYTES)
            .map(|i| (i as u8).wrapping_mul(17).wrapping_add(5))
            .collect();
        let (mut cpk, mut rpk) = (vec![0u8; PK_BYTES], vec![0u8; PK_BYTES]);
        let (mut csk, mut rsk) = (vec![0u8; SK_BYTES], vec![0u8; SK_BYTES]);
        assert_eq!(c_key(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!(r_key(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!((cpk.as_slice(), csk.as_slice()), (rpk.as_slice(), rsk.as_slice()));

        let message: Vec<u8> = (0..41)
            .map(|i| (i as u8).wrapping_mul(11).wrapping_add(9))
            .collect();
        c_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        r_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let (mut csm, mut rsm) =
            (vec![0u8; SIG_BYTES + message.len()], vec![0u8; SIG_BYTES + message.len()]);
        let (mut c_len, mut r_len) = (0u64, 0u64);
        assert_eq!(
            c_sign(csm.as_mut_ptr(), &mut c_len, message.as_ptr(), message.len() as u64, csk.as_ptr()),
            0,
        );
        assert_eq!(
            r_sign(rsm.as_mut_ptr(), &mut r_len, message.as_ptr(), message.len() as u64, rsk.as_ptr()),
            0,
        );
        assert_eq!((c_len, csm.as_slice()), (r_len, rsm.as_slice()));
        let (mut cm, mut rm) = (vec![0xa5; csm.len()], vec![0xa5; rsm.len()]);
        let (mut cm_len, mut rm_len) = (u64::MAX, u64::MAX);
        assert_eq!(c_open(cm.as_mut_ptr(), &mut cm_len, csm.as_ptr(), c_len, cpk.as_ptr()), 0);
        assert_eq!(r_open(rm.as_mut_ptr(), &mut rm_len, rsm.as_ptr(), r_len, rpk.as_ptr()), 0);
        assert_eq!((cm_len, &cm[..cm_len as usize]), (rm_len, &rm[..rm_len as usize]));
        assert_eq!(&cm[..cm_len as usize], message);

        c_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        r_init(entropy.as_mut_ptr(), personalization.as_mut_ptr());
        let (mut cbuf, mut rbuf) =
            (vec![0u8; SIG_BYTES + message.len()], vec![0u8; SIG_BYTES + message.len()]);
        cbuf[SIG_BYTES..].copy_from_slice(&message);
        rbuf[SIG_BYTES..].copy_from_slice(&message);
        let (mut c_len, mut r_len) = (0u64, 0u64);
        assert_eq!(
            c_sign(
                cbuf.as_mut_ptr(),
                &mut c_len,
                cbuf.as_ptr().add(SIG_BYTES),
                message.len() as u64,
                csk.as_ptr(),
            ),
            0,
        );
        assert_eq!(
            r_sign(
                rbuf.as_mut_ptr(),
                &mut r_len,
                rbuf.as_ptr().add(SIG_BYTES),
                message.len() as u64,
                rsk.as_ptr(),
            ),
            0,
        );
        assert_eq!((c_len, cbuf.as_slice()), (r_len, rbuf.as_slice()));
        let (mut c_open_len, mut r_open_len) = (u64::MAX, u64::MAX);
        assert_eq!(
            c_open(cbuf.as_mut_ptr(), &mut c_open_len, cbuf.as_ptr(), c_len, cpk.as_ptr()),
            0,
        );
        assert_eq!(
            r_open(rbuf.as_mut_ptr(), &mut r_open_len, rbuf.as_ptr(), r_len, rpk.as_ptr()),
            0,
        );
        assert_eq!(
            (c_open_len, &cbuf[..c_open_len as usize]),
            (r_open_len, &rbuf[..r_open_len as usize]),
        );
        assert_eq!(&cbuf[..c_open_len as usize], message);
    }
}

unsafe fn deterministic_signature(
    libs: &Libraries,
    rust: bool,
    sk: &[u8],
    message: &[u8],
) -> Vec<u8> {
    type Init = unsafe extern "C" fn(*mut c_void);
    type Set = unsafe extern "C" fn(*mut u32, u32);
    type SetTree = unsafe extern "C" fn(*mut u32, u64);
    type CopyAddr = unsafe extern "C" fn(*mut u32, *const u32);
    type Gen = unsafe extern "C" fn(
        *mut u8,
        *const u8,
        *const u8,
        *const u8,
        u64,
        *const c_void,
    );
    type Hmsg = unsafe extern "C" fn(
        *mut u8,
        *mut u64,
        *mut u32,
        *const u8,
        *const u8,
        *const u8,
        u64,
        *const c_void,
    );
    type Fors = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const u8,
        *const c_void,
        *const u32,
    );
    type Merkle = unsafe extern "C" fn(
        *mut u8,
        *mut u8,
        *const c_void,
        *mut u32,
        *mut u32,
        u32,
    );
    macro_rules! get {
        ($ty:ty, $name:literal) => {
            if rust {
                unsafe { libs.r::<$ty>(concat!($name, "\0").as_bytes()) }
            } else {
                unsafe { libs.c::<$ty>(concat!($name, "\0").as_bytes()) }
            }
        };
    }
    let init: Init = get!(Init, "SPX_initialize_hash_function");
    let set_type: Set = get!(Set, "SPX_set_type");
    let set_layer: Set = get!(Set, "SPX_set_layer_addr");
    let set_keypair: Set = get!(Set, "SPX_set_keypair_addr");
    let set_tree: SetTree = get!(SetTree, "SPX_set_tree_addr");
    let copy_subtree: CopyAddr = get!(CopyAddr, "SPX_copy_subtree_addr");
    let generate: Gen = get!(Gen, "SPX_gen_message_random");
    let hmsg: Hmsg = get!(Hmsg, "SPX_hash_message");
    let fors: Fors = get!(Fors, "SPX_fors_sign");
    let merkle: Merkle = get!(Merkle, "SPX_merkle_sign");
    let mut ctx = AlignedContext::new(&sk[2 * N..3 * N], &sk[..N]);
    init(ctx.ptr());
    let mut signature = vec![0u8; SIG_BYTES];
    let optrand: Vec<_> = (0..N).map(|i| (i as u8).wrapping_mul(29).wrapping_add(7)).collect();
    generate(
        signature.as_mut_ptr(),
        sk[N..2 * N].as_ptr(),
        optrand.as_ptr(),
        message.as_ptr(),
        message.len() as u64,
        ctx.ptr(),
    );
    let mut digest = vec![0u8; FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut leaf = 0u32;
    hmsg(
        digest.as_mut_ptr(),
        &mut tree,
        &mut leaf,
        signature.as_ptr(),
        sk[2 * N..].as_ptr(),
        message.as_ptr(),
        message.len() as u64,
        ctx.ptr(),
    );
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type(wots_addr.as_mut_ptr(), 0);
    set_type(tree_addr.as_mut_ptr(), 2);
    set_tree(wots_addr.as_mut_ptr(), tree);
    set_keypair(wots_addr.as_mut_ptr(), leaf);
    let mut root = vec![0u8; N];
    let mut offset = N;
    fors(
        signature[offset..].as_mut_ptr(),
        root.as_mut_ptr(),
        digest.as_ptr(),
        ctx.ptr(),
        wots_addr.as_ptr(),
    );
    offset += FORS_BYTES;
    for layer in 0..D {
        set_layer(tree_addr.as_mut_ptr(), layer as u32);
        set_tree(tree_addr.as_mut_ptr(), tree);
        copy_subtree(wots_addr.as_mut_ptr(), tree_addr.as_ptr());
        set_keypair(wots_addr.as_mut_ptr(), leaf);
        merkle(
            signature[offset..].as_mut_ptr(),
            root.as_mut_ptr(),
            ctx.ptr(),
            wots_addr.as_mut_ptr(),
            tree_addr.as_mut_ptr(),
            leaf,
        );
        offset += WOTS_BYTES + TREE_HEIGHT * N;
        leaf = (tree & ((1u64 << TREE_HEIGHT) - 1)) as u32;
        tree >>= TREE_HEIGHT;
    }
    assert_eq!(offset, SIG_BYTES);
    signature
}

#[test]
fn seeded_keypair_full_pipeline_and_errors() {
    let libs = unsafe { Libraries::load() };
    unsafe {
        type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
        type Verify =
            unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
        type Open =
            unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
        let c_key: SeedKeypair = libs.c(b"crypto_sign_seed_keypair\0");
        let r_key: SeedKeypair = libs.r(b"crypto_sign_seed_keypair\0");
        let c_verify: Verify = libs.c(b"crypto_sign_verify\0");
        let r_verify: Verify = libs.r(b"crypto_sign_verify\0");
        let c_open: Open = libs.c(b"crypto_sign_open\0");
        let r_open: Open = libs.r(b"crypto_sign_open\0");
        let mut c_pk = vec![0; PK_BYTES];
        let mut r_pk = c_pk.clone();
        let mut c_sk = vec![0; SK_BYTES];
        let mut r_sk = c_sk.clone();
        let mut random_seed = vec![0u8; SEED_BYTES];
        Rng(0xa4093822299f31d0).fill(&mut random_seed);
        let seeds = [
            vec![0u8; SEED_BYTES],
            (0..SEED_BYTES).map(|i| i as u8).collect(),
            vec![0xff; SEED_BYTES],
            random_seed,
        ];
        for seed in seeds {
            assert_eq!(c_key(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr()), 0);
            assert_eq!(r_key(r_pk.as_mut_ptr(), r_sk.as_mut_ptr(), seed.as_ptr()), 0);
            assert_eq!(c_pk, r_pk);
            assert_eq!(c_sk, r_sk);
        }
        let message: Vec<_> = (0..73).map(|i| (i as u8).wrapping_mul(11)).collect();
        let c_sig = deterministic_signature(&libs, false, &c_sk, &message);
        let r_sig = deterministic_signature(&libs, true, &r_sk, &message);
        assert_eq!(c_sig, r_sig);
        assert_eq!(c_verify(c_sig.as_ptr(), SIG_BYTES, message.as_ptr(), message.len(), c_pk.as_ptr()), 0);
        assert_eq!(r_verify(r_sig.as_ptr(), SIG_BYTES, message.as_ptr(), message.len(), r_pk.as_ptr()), 0);
        for bad_len in [0, SIG_BYTES - 1, SIG_BYTES + 1] {
            assert_eq!(
                c_verify(std::ptr::null(), bad_len, std::ptr::null(), 0, std::ptr::null()),
                r_verify(std::ptr::null(), bad_len, std::ptr::null(), 0, std::ptr::null())
            );
        }
        let mut bad = c_sig.clone();
        bad[SIG_BYTES / 2] ^= 1;
        assert_eq!(
            c_verify(bad.as_ptr(), SIG_BYTES, message.as_ptr(), message.len(), c_pk.as_ptr()),
            -1
        );
        assert_eq!(
            r_verify(bad.as_ptr(), SIG_BYTES, message.as_ptr(), message.len(), r_pk.as_ptr()),
            -1
        );
        for short in [0usize, 1, SIG_BYTES - 1] {
            let signed = vec![0x5a; short];
            let mut ca = vec![0xa5; short.max(1)];
            let mut ra = ca.clone();
            let (mut cl, mut rl) = (u64::MAX, u64::MAX);
            let cr = c_open(ca.as_mut_ptr(), &mut cl, signed.as_ptr(), short as u64, c_pk.as_ptr());
            let rr = r_open(ra.as_mut_ptr(), &mut rl, signed.as_ptr(), short as u64, r_pk.as_ptr());
            assert_eq!((cr, cl, ca), (rr, rl, ra));
        }
        let mut signed = bad;
        signed.extend_from_slice(&message);
        let mut ca = vec![0xa5; signed.len()];
        let mut ra = ca.clone();
        let (mut cl, mut rl) = (99, 99);
        assert_eq!(
            c_open(ca.as_mut_ptr(), &mut cl, signed.as_ptr(), signed.len() as u64, c_pk.as_ptr()),
            -1
        );
        assert_eq!(
            r_open(ra.as_mut_ptr(), &mut rl, signed.as_ptr(), signed.len() as u64, r_pk.as_ptr()),
            -1
        );
        assert_eq!((cl, ca), (rl, ra));
    }
}

#[repr(C)]
#[derive(Clone)]
struct AesXof {
    buffer: [u8; 16],
    buffer_pos: usize,
    length_remaining: usize,
    key: [u8; 32],
    ctr: [u8; 16],
}

#[test]
fn seedexpander_error_surface() {
    let libs = unsafe { Libraries::load() };
    let det_core = PathBuf::from(
        std::env::var_os("SPHINCS_C_DET_CORE").expect("SPHINCS_C_DET_CORE"),
    );
    let det_backend = PathBuf::from(
        std::env::var_os("SPHINCS_C_DET_BACKEND").expect("SPHINCS_C_DET_BACKEND"),
    );
    unsafe {
        let _crypto = Box::leak(Box::new(
            Library::open(Some("libcrypto.so.3"), RTLD_NOW | RTLD_GLOBAL).unwrap(),
        ));
        let _backend = Box::leak(Box::new(
            Library::open(Some(det_backend), RTLD_LAZY | RTLD_GLOBAL).unwrap(),
        ));
        let det = Box::leak(Box::new(
            Library::open(Some(det_core), RTLD_NOW | RTLD_GLOBAL).unwrap(),
        ));
        type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, usize) -> i32;
        type Expand = unsafe extern "C" fn(*mut AesXof, *mut u8, usize) -> i32;
        let c_init: Init = *det.get(b"seedexpander_init\0").unwrap();
        let c_expand: Expand = *det.get(b"seedexpander\0").unwrap();
        let r_init: Init = libs.r(b"seedexpander_init\0");
        let r_expand: Expand = libs.r(b"seedexpander\0");
        let mut seed = [3u8; 32];
        let mut diversifier = [7u8; 8];
        let blank = AesXof {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        };
        let (mut ca, mut ra) = (blank.clone(), blank.clone());
        if usize::BITS > 32 {
            assert_eq!(
                c_init(&mut ca, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 1usize << 32),
                -1
            );
            assert_eq!(
                r_init(&mut ra, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 1usize << 32),
                -1
            );
        }
        assert_eq!(c_init(&mut ca, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 32), 0);
        assert_eq!(r_init(&mut ra, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 32), 0);
        assert_eq!(c_expand(&mut ca, std::ptr::null_mut(), 1), -2);
        assert_eq!(r_expand(&mut ra, std::ptr::null_mut(), 1), -2);
        let mut co = [0u8; 32];
        let mut ro = [0u8; 32];
        assert_eq!(c_expand(&mut ca, co.as_mut_ptr(), 32), -3);
        assert_eq!(r_expand(&mut ra, ro.as_mut_ptr(), 32), -3);
        assert_eq!(ca.length_remaining, ra.length_remaining);

        for chunks in [
            [1usize, 15, 16, 17, 31].as_slice(),
            [16usize, 16, 16, 16].as_slice(),
        ] {
            let (mut ca, mut ra) = (blank.clone(), blank.clone());
            assert_eq!(c_init(&mut ca, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 128), 0);
            assert_eq!(r_init(&mut ra, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 128), 0);
            for &len in chunks {
                let (mut co, mut ro) = (vec![0u8; len], vec![0u8; len]);
                assert_eq!(c_expand(&mut ca, co.as_mut_ptr(), len), 0);
                assert_eq!(r_expand(&mut ra, ro.as_mut_ptr(), len), 0);
                assert_eq!(co, ro);
                assert_eq!(
                    (ca.buffer, ca.buffer_pos, ca.length_remaining, ca.ctr),
                    (ra.buffer, ra.buffer_pos, ra.length_remaining, ra.ctr),
                );
            }
        }
        let (mut ca, mut ra) = (blank.clone(), blank);
        assert_eq!(c_init(&mut ca, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 64), 0);
        assert_eq!(r_init(&mut ra, seed.as_mut_ptr(), diversifier.as_mut_ptr(), 64), 0);
        ca.ctr[12..].fill(0xff);
        ra.ctr[12..].fill(0xff);
        let (mut co, mut ro) = ([0u8; 17], [0u8; 17]);
        assert_eq!(c_expand(&mut ca, co.as_mut_ptr(), co.len()), 0);
        assert_eq!(r_expand(&mut ra, ro.as_mut_ptr(), ro.len()), 0);
        assert_eq!((co, ca.ctr), (ro, ra.ctr));
    }
}
