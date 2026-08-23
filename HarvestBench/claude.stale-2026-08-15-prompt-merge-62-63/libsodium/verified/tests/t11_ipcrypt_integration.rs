//! t11 — `crypto_ipcrypt` (CONFIGS 265–273, ERRORS 392–394) plus the
//! cross-module integration pipelines (CONFIGS 274–278).
//!
//! Every call goes through `dlsym` on BOTH shared libraries (see
//! `tests/common/mod.rs`); no Rust function is ever called directly.
//!
//! Output-buffer discipline used everywhere in this file: each output buffer is
//! `n + GUARD` bytes prefilled with `0xAA`; after the call the trailing GUARD
//! bytes are asserted to still be `0xAA` (no overrun) and the FULL buffer
//! (payload + guard) is compared between the two libraries.

mod common;
use common::*;

use libc::{c_char, c_int, c_uchar, c_void};
use libloading::Library;
use std::ffi::CStr;
use std::sync::{Mutex, MutexGuard, OnceLock};

// =============================================================== signatures

type SizeFn = unsafe extern "C" fn() -> usize;
type IntFn = unsafe extern "C" fn() -> c_int;
type U8Fn = unsafe extern "C" fn() -> c_uchar;
type KeygenFn = unsafe extern "C" fn(*mut u8);
/// `void f(out, in, k)` — ipcrypt encrypt/decrypt/nd_decrypt/ndx_decrypt/pfx_*
type Ipc3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
/// `void f(out, in, t, k)` — ipcrypt nd_encrypt / ndx_encrypt
type Ipc4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);

type Kp2 = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Conv = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Fn3i = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type BoxEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type Shash = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type HkExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type HkExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;
type AeadEnc =
    unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8, u64, *const u8, *const u8, *const u8) -> c_int;
type AeadDec =
    unsafe extern "C" fn(*mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64, *const u8, *const u8) -> c_int;
type Ghash = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type SmBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type StreamXor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type XwEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type XwEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type PwHash = unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, c_int) -> c_int;
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
type SsInit = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SsPush = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, c_uchar) -> c_int;
type SsPull = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64) -> c_int;
type KxSess = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;

// ============================================================= buffer guards

const GUARD: usize = 16;
const FILL: u8 = 0xAA;

/// `n` payload bytes + `GUARD` sentinel bytes, all prefilled with `0xAA`.
fn ob(n: usize) -> Vec<u8> {
    vec![FILL; n + GUARD]
}

fn guard(what: &str, b: &[u8], n: usize) {
    assert_eq!(b.len(), n + GUARD, "{what}: buffer must be payload+GUARD");
    for (i, &x) in b[n..].iter().enumerate() {
        assert_eq!(
            x, FILL,
            "{what}: OUTPUT OVERRUN — guard byte {i} (absolute offset {}) was overwritten \
             (0x{x:02x} != 0xAA); full buffer = {}",
            n + i,
            hexs(b)
        );
    }
}

/// Check both guards, then compare the FULL buffers (payload + guard).
fn cmp_out(what: &str, n: usize, c: &[u8], r: &[u8]) {
    guard(&format!("{what} [C]"), c, n);
    guard(&format!("{what} [rust]"), r, n);
    assert_eq_bytes(what, c, r);
}

// ============================================================== IP helpers

/// `::ffff:a.b.c.d`
fn ipv4_mapped(o: [u8; 4]) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[10] = 0xff;
    b[11] = 0xff;
    b[12..16].copy_from_slice(&o);
    b
}

/// `2001:db8::1` — a native IPv6 address (NOT IPv4-mapped).
const IP6_2001DB8_1: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

fn is_v4_mapped(b: &[u8]) -> bool {
    b[..10].iter().all(|&x| x == 0) && b[10] == 0xff && b[11] == 0xff
}

/// Prefix-bit `p` counted MSB-first from byte 0 — exactly the C's
/// `prefix_len_bits` axis (`bit_pos = 127 - prefix_len_bits`).
fn get_pbit(ip: &[u8], p: usize) -> u8 {
    (ip[p / 8] >> (7 - p % 8)) & 1
}

fn set_pbit(ip: &mut [u8], p: usize, v: u8) {
    let m = 0x80u8 >> (p % 8);
    if v & 1 != 0 {
        ip[p / 8] |= m;
    } else {
        ip[p / 8] &= !m;
    }
}

fn shared_pbits(a: &[u8], b: &[u8]) -> usize {
    for p in 0..128 {
        if get_pbit(a, p) != get_pbit(b, p) {
            return p;
        }
    }
    128
}

/// Two 16-byte IPs sharing EXACTLY `n` leading prefix bits, neither IPv4-mapped.
/// `n` must not fall inside byte 10 (prefix bits 80..88), which is forced.
fn pfx_pair(rng: &mut Rng, n: usize) -> ([u8; 16], [u8; 16]) {
    assert!(!(80..88).contains(&n), "pfx_pair: n={n} collides with the forced byte 10");
    let mut a = [0u8; 16];
    rng.fill(&mut a);
    let mut b = a;
    if n < 128 {
        set_pbit(&mut b, n, 1 ^ get_pbit(&a, n));
        for p in (n + 1)..128 {
            set_pbit(&mut b, p, rng.byte() & 1);
        }
    }
    // byte 10 != 0xff  =>  never IPv4-mapped  =>  pfx starts at prefix bit 0.
    a[10] = 0x01;
    b[10] = 0x01;
    assert!(!is_v4_mapped(&a) && !is_v4_mapped(&b));
    if n < 128 {
        assert_eq!(shared_pbits(&a, &b), n, "pfx_pair construction broken for n={n}");
    }
    (a, b)
}

// ==================================================== RNG session management
//
// cargo runs the tests of one binary as parallel threads inside ONE process and
// both `.so`s are shared by all of them, so any test that swaps the global
// `randombytes` implementation must be serialised and must restore it.

static RNG_LOCK: Mutex<()> = Mutex::new(());

fn rng_lock() -> MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn data_ptr<T: 'static>(lib: &'static Library, name: &str) -> *const T {
    let s = unsafe { sym::<*const T>(lib, name) };
    *s
}

fn restore_sysrandom() {
    let l = libs();
    let (c, r) = unsafe { pair::<SetImplFn>("randombytes_set_implementation") };
    let pc = data_ptr::<RandombytesImpl>(&l.c, "randombytes_sysrandom_implementation");
    let pr = data_ptr::<RandombytesImpl>(&l.r, "randombytes_sysrandom_implementation");
    unsafe {
        c(pc);
        r(pr);
    }
}

/// Holds the serialisation lock and puts both libraries back on `sysrandom`
/// even if the test body panics.
struct RngSession {
    _g: MutexGuard<'static, ()>,
}

impl RngSession {
    fn new(with_uniform: bool) -> Self {
        let g = rng_lock();
        install_det_rng(with_uniform);
        RngSession { _g: g }
    }
}

impl Drop for RngSession {
    fn drop(&mut self) {
        restore_sysrandom();
    }
}

// ================================================ pre-`sodium_init()` capture
//
// CONFIGS 278 needs results produced BEFORE `sodium_init()` has ever run in the
// process. `sodium_init()` is global and irreversible, so the pre-init snapshot
// is taken in a `fork()`ed child. To guarantee the fork happens before any
// `sodium_init()`, EVERY test in this file calls `init_all()` (never
// `init_both()` directly), and `init_all()` drives the snapshot through a
// `OnceLock` first — `OnceLock::get_or_init` blocks the other test threads, so
// no thread can slip a `sodium_init()` in ahead of the fork.

const SUBSET_REGION: usize = 16384;

struct PreinitCapture {
    c: Vec<u8>,
    r: Vec<u8>,
}

static PREINIT: OnceLock<PreinitCapture> = OnceLock::new();

fn preinit() -> &'static PreinitCapture {
    PREINIT.get_or_init(capture_preinit)
}

/// The one entry point every `#[test]` in this file uses.
fn init_all() {
    let _ = preinit();
    init_both();
}

/// `mmap(MAP_SHARED|MAP_ANONYMOUS)` scratch page shared with `fork()`ed children.
struct Shm {
    base: *mut u8,
    len: usize,
}

impl Shm {
    fn new(len: usize) -> Shm {
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(p != libc::MAP_FAILED, "mmap(MAP_SHARED) failed");
        unsafe { std::ptr::write_bytes(p as *mut u8, 0, len) };
        Shm { base: p as *mut u8, len }
    }
    fn slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.len) }
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.base as *mut c_void, self.len) };
    }
}

/// Append-only writer over a raw region: `[0..4)` = payload length (u32 LE),
/// `[4..)` = payload, last payload byte = "all output guards intact" flag.
/// Performs NO heap allocation, so it is safe inside a `fork()`ed child.
struct W {
    p: *mut u8,
    n: usize,
    ok: bool,
}

impl W {
    fn new(p: *mut u8) -> W {
        W { p, n: 0, ok: true }
    }
    unsafe fn push(&mut self, b: &[u8]) {
        if 4 + self.n + b.len() + 1 > SUBSET_REGION {
            self.ok = false;
            return;
        }
        std::ptr::copy_nonoverlapping(b.as_ptr(), self.p.add(4 + self.n), b.len());
        self.n += b.len();
    }
    /// Push a full `payload+GUARD` buffer, flagging a clobbered guard.
    unsafe fn out(&mut self, buf: &[u8], n: usize) {
        if !buf[n..].iter().all(|&x| x == FILL) {
            self.ok = false;
        }
        self.push(buf);
    }
    unsafe fn rc(&mut self, rc: c_int) {
        self.push(&(rc as i32).to_le_bytes());
    }
    unsafe fn finish(mut self) {
        let f = [self.ok as u8];
        // Reserve was accounted for by `push`'s `+ 1`.
        std::ptr::copy_nonoverlapping(f.as_ptr(), self.p.add(4 + self.n), 1);
        self.n += 1;
        std::ptr::write_unaligned(self.p as *mut u32, self.n as u32);
    }
}

// ------------------------------------------------------------- symbol tables

struct IpcSyms {
    enc: Ipc3,
    dec: Ipc3,
    nd_enc: Ipc4,
    nd_dec: Ipc3,
    ndx_enc: Ipc4,
    ndx_dec: Ipc3,
    pfx_enc: Ipc3,
    pfx_dec: Ipc3,
    keygen: [KeygenFn; 4],
    getters: [SizeFn; 12],
    pick: IntFn,
}

/// The 12 `*bytes` getters, in header order, with their required values.
const IPC_GETTERS: [(&str, usize); 12] = [
    ("crypto_ipcrypt_bytes", 16),
    ("crypto_ipcrypt_keybytes", 16),
    ("crypto_ipcrypt_nd_keybytes", 16),
    ("crypto_ipcrypt_nd_tweakbytes", 8),
    ("crypto_ipcrypt_nd_inputbytes", 16),
    ("crypto_ipcrypt_nd_outputbytes", 24),
    ("crypto_ipcrypt_ndx_keybytes", 32),
    ("crypto_ipcrypt_ndx_tweakbytes", 16),
    ("crypto_ipcrypt_ndx_inputbytes", 16),
    ("crypto_ipcrypt_ndx_outputbytes", 32),
    ("crypto_ipcrypt_pfx_keybytes", 32),
    ("crypto_ipcrypt_pfx_bytes", 16),
];

const IPC_KEYGENS: [(&str, usize); 4] = [
    ("crypto_ipcrypt_keygen", 16),
    ("crypto_ipcrypt_nd_keygen", 16),
    ("crypto_ipcrypt_ndx_keygen", 32),
    ("crypto_ipcrypt_pfx_keygen", 32),
];

fn f<T: Copy + 'static>(lib: &'static Library, name: &str) -> T {
    *unsafe { sym::<T>(lib, name) }
}

fn resolve_ipc(lib: &'static Library) -> IpcSyms {
    IpcSyms {
        enc: f(lib, "crypto_ipcrypt_encrypt"),
        dec: f(lib, "crypto_ipcrypt_decrypt"),
        nd_enc: f(lib, "crypto_ipcrypt_nd_encrypt"),
        nd_dec: f(lib, "crypto_ipcrypt_nd_decrypt"),
        ndx_enc: f(lib, "crypto_ipcrypt_ndx_encrypt"),
        ndx_dec: f(lib, "crypto_ipcrypt_ndx_decrypt"),
        pfx_enc: f(lib, "crypto_ipcrypt_pfx_encrypt"),
        pfx_dec: f(lib, "crypto_ipcrypt_pfx_decrypt"),
        keygen: std::array::from_fn(|i| f(lib, IPC_KEYGENS[i].0)),
        getters: std::array::from_fn(|i| f(lib, IPC_GETTERS[i].0)),
        pick: f(lib, "_crypto_ipcrypt_pick_best_implementation"),
    }
}

struct SubsetSyms {
    ipc: IpcSyms,
    sip24: Shash,
    sipx24: Shash,
    sign_seed_kp: SeedKp,
    sk2c: Conv,
    pk2c: Conv,
    beforenm: Fn3i,
    box_easy: BoxEasy,
    box_open: BoxEasy,
    sb_easy: SbEasy,
    sb_open: SbEasy,
    hk_extract: HkExtract,
    hk_expand: HkExpand,
    aead_enc: AeadEnc,
    aead_dec: AeadDec,
    ghash: Ghash,
    otauth: Shash,
    smbase: SmBase,
    chacha_xor: StreamXor,
    xw_seed_kp: SeedKp,
    xw_enc_det: XwEncDet,
    xw_dec: Fn3i,
}

fn resolve_subset(lib: &'static Library) -> SubsetSyms {
    SubsetSyms {
        ipc: resolve_ipc(lib),
        sip24: f(lib, "crypto_shorthash_siphash24"),
        sipx24: f(lib, "crypto_shorthash_siphashx24"),
        sign_seed_kp: f(lib, "crypto_sign_ed25519_seed_keypair"),
        sk2c: f(lib, "crypto_sign_ed25519_sk_to_curve25519"),
        pk2c: f(lib, "crypto_sign_ed25519_pk_to_curve25519"),
        beforenm: f(lib, "crypto_box_beforenm"),
        box_easy: f(lib, "crypto_box_easy"),
        box_open: f(lib, "crypto_box_open_easy"),
        sb_easy: f(lib, "crypto_secretbox_easy"),
        sb_open: f(lib, "crypto_secretbox_open_easy"),
        hk_extract: f(lib, "crypto_kdf_hkdf_sha256_extract"),
        hk_expand: f(lib, "crypto_kdf_hkdf_sha256_expand"),
        aead_enc: f(lib, "crypto_aead_xchacha20poly1305_ietf_encrypt"),
        aead_dec: f(lib, "crypto_aead_xchacha20poly1305_ietf_decrypt"),
        ghash: f(lib, "crypto_generichash"),
        otauth: f(lib, "crypto_onetimeauth_poly1305"),
        smbase: f(lib, "crypto_scalarmult_curve25519_base"),
        chacha_xor: f(lib, "crypto_stream_chacha20_ietf_xor"),
        xw_seed_kp: f(lib, "crypto_kem_xwing_seed_keypair"),
        xw_enc_det: f(lib, "crypto_kem_xwing_enc_deterministic"),
        xw_dec: f(lib, "crypto_kem_xwing_dec"),
    }
}

// --------------------------------------------------- the CONFIGS-278 subset
//
// Entirely deterministic (NO `randombytes` consumer anywhere) and allocation
// free, so it produces identical bytes when run in a `fork()`ed child before
// `sodium_init()` and in the parent after it.

unsafe fn run_subset(s: &SubsetSyms, base: *mut u8) {
    let mut w = W::new(base);

    let k16: [u8; 16] = std::array::from_fn(|i| (i as u8).wrapping_mul(17) ^ 0x5c);
    let k32: [u8; 32] = std::array::from_fn(|i| (i as u8).wrapping_mul(29).wrapping_add(7));
    let kdeg: [u8; 32] = std::array::from_fn(|i| k16[i % 16]); // identical halves
    let t8: [u8; 8] = std::array::from_fn(|i| 0xa0 ^ i as u8);
    let t16: [u8; 16] = std::array::from_fn(|i| 0x5e ^ (i as u8).wrapping_mul(3));
    let ip4 = ipv4_mapped([192, 0, 2, 1]);
    let ip6 = IP6_2001DB8_1;

    // ---- ipcrypt, all four modes, both directions, incl. degenerate key ----
    let mut e16 = [FILL; 16 + GUARD];
    (s.ipc.enc)(e16.as_mut_ptr(), ip4.as_ptr(), k16.as_ptr());
    w.out(&e16, 16);
    let mut d16 = [FILL; 16 + GUARD];
    (s.ipc.dec)(d16.as_mut_ptr(), e16.as_ptr(), k16.as_ptr());
    w.out(&d16, 16);

    let mut nd = [FILL; 24 + GUARD];
    (s.ipc.nd_enc)(nd.as_mut_ptr(), ip6.as_ptr(), t8.as_ptr(), k16.as_ptr());
    w.out(&nd, 24);
    let mut ndp = [FILL; 16 + GUARD];
    (s.ipc.nd_dec)(ndp.as_mut_ptr(), nd.as_ptr(), k16.as_ptr());
    w.out(&ndp, 16);

    let mut ndx = [FILL; 32 + GUARD];
    (s.ipc.ndx_enc)(ndx.as_mut_ptr(), ip6.as_ptr(), t16.as_ptr(), k32.as_ptr());
    w.out(&ndx, 32);
    let mut ndxp = [FILL; 16 + GUARD];
    (s.ipc.ndx_dec)(ndxp.as_mut_ptr(), ndx.as_ptr(), k32.as_ptr());
    w.out(&ndxp, 16);

    let mut ndxd = [FILL; 32 + GUARD];
    (s.ipc.ndx_enc)(ndxd.as_mut_ptr(), ip6.as_ptr(), t16.as_ptr(), kdeg.as_ptr());
    w.out(&ndxd, 32);

    let mut pfx = [FILL; 16 + GUARD];
    (s.ipc.pfx_enc)(pfx.as_mut_ptr(), ip4.as_ptr(), k32.as_ptr());
    w.out(&pfx, 16);
    let mut pfxp = [FILL; 16 + GUARD];
    (s.ipc.pfx_dec)(pfxp.as_mut_ptr(), pfx.as_ptr(), k32.as_ptr());
    w.out(&pfxp, 16);
    let mut pfxd = [FILL; 16 + GUARD];
    (s.ipc.pfx_enc)(pfxd.as_mut_ptr(), ip6.as_ptr(), kdeg.as_ptr());
    w.out(&pfxd, 16);

    // ---- the 12 getters and the implementation picker ----
    let mut g = [0u8; 12];
    for i in 0..12 {
        g[i] = (s.ipc.getters[i])() as u8;
    }
    w.push(&g);
    let picked = (s.ipc.pick)();
    w.rc(picked);
    let mut e16b = [FILL; 16 + GUARD];
    (s.ipc.enc)(e16b.as_mut_ptr(), ip4.as_ptr(), k16.as_ptr());
    w.out(&e16b, 16);

    // ---- siphash (ERRORS 394 also pre-init) ----
    let msg: [u8; 32] = std::array::from_fn(|i| (i as u8).wrapping_mul(11));
    let mut h8 = [FILL; 8 + GUARD];
    w.rc((s.sip24)(h8.as_mut_ptr(), msg.as_ptr(), 32, k16.as_ptr()));
    w.out(&h8, 8);
    let mut h8z = [FILL; 8 + GUARD];
    w.rc((s.sip24)(h8z.as_mut_ptr(), msg.as_ptr(), 0, k16.as_ptr()));
    w.out(&h8z, 8);
    let mut h16 = [FILL; 16 + GUARD];
    w.rc((s.sipx24)(h16.as_mut_ptr(), msg.as_ptr(), 32, k16.as_ptr()));
    w.out(&h16, 16);
    let mut h16z = [FILL; 16 + GUARD];
    w.rc((s.sipx24)(h16z.as_mut_ptr(), msg.as_ptr(), 0, k16.as_ptr()));
    w.out(&h16z, 16);

    // ---- a few primitives whose `_pick_best_implementation` runs in init ----
    let mut gh = [FILL; 32 + GUARD];
    w.rc((s.ghash)(gh.as_mut_ptr(), 32, msg.as_ptr(), 32, k32.as_ptr(), 32));
    w.out(&gh, 32);
    let mut ot = [FILL; 16 + GUARD];
    w.rc((s.otauth)(ot.as_mut_ptr(), msg.as_ptr(), 32, k32.as_ptr()));
    w.out(&ot, 16);
    let mut sm = [FILL; 32 + GUARD];
    w.rc((s.smbase)(sm.as_mut_ptr(), k32.as_ptr()));
    w.out(&sm, 32);
    let n24: [u8; 24] = std::array::from_fn(|i| 0x31 ^ (i as u8));
    let mut cx = [FILL; 64 + GUARD];
    let long: [u8; 64] = std::array::from_fn(|i| (i as u8).wrapping_mul(13).wrapping_add(1));
    w.rc((s.chacha_xor)(cx.as_mut_ptr(), long.as_ptr(), 64, n24.as_ptr(), k32.as_ptr()));
    w.out(&cx, 64);

    // ---- sign -> curve25519 -> box -> secretbox (CONFIGS 274, pre-init) ----
    let seed: [u8; 32] = std::array::from_fn(|i| 0x77 ^ (i as u8));
    let mut spk = [FILL; 32 + GUARD];
    let mut ssk = [FILL; 64 + GUARD];
    w.rc((s.sign_seed_kp)(spk.as_mut_ptr(), ssk.as_mut_ptr(), seed.as_ptr()));
    w.out(&spk, 32);
    w.out(&ssk, 64);
    let mut csk = [FILL; 32 + GUARD];
    w.rc((s.sk2c)(csk.as_mut_ptr(), ssk.as_ptr()));
    w.out(&csk, 32);
    let mut cpk = [FILL; 32 + GUARD];
    w.rc((s.pk2c)(cpk.as_mut_ptr(), spk.as_ptr()));
    w.out(&cpk, 32);
    let mut bk = [FILL; 32 + GUARD];
    w.rc((s.beforenm)(bk.as_mut_ptr(), cpk.as_ptr(), csk.as_ptr()));
    w.out(&bk, 32);
    let mut bc = [FILL; 48 + GUARD];
    w.rc((s.box_easy)(bc.as_mut_ptr(), msg.as_ptr(), 32, n24.as_ptr(), cpk.as_ptr(), csk.as_ptr()));
    w.out(&bc, 48);
    let mut bp = [FILL; 32 + GUARD];
    w.rc((s.box_open)(bp.as_mut_ptr(), bc.as_ptr(), 48, n24.as_ptr(), cpk.as_ptr(), csk.as_ptr()));
    w.out(&bp, 32);
    let mut sc = [FILL; 48 + GUARD];
    w.rc((s.sb_easy)(sc.as_mut_ptr(), msg.as_ptr(), 32, n24.as_ptr(), bk.as_ptr()));
    w.out(&sc, 48);
    let mut sp = [FILL; 32 + GUARD];
    w.rc((s.sb_open)(sp.as_mut_ptr(), sc.as_ptr(), 48, n24.as_ptr(), bk.as_ptr()));
    w.out(&sp, 32);

    // ---- kem -> kdf -> aead (CONFIGS 276, pre-init) ----
    let mut xpk = [FILL; 1216 + GUARD];
    let mut xsk = [FILL; 32 + GUARD];
    w.rc((s.xw_seed_kp)(xpk.as_mut_ptr(), xsk.as_mut_ptr(), seed.as_ptr()));
    w.out(&xpk, 1216);
    w.out(&xsk, 32);
    let seed64: [u8; 64] = std::array::from_fn(|i| 0x42 ^ (i as u8));
    let mut xct = [FILL; 1120 + GUARD];
    let mut xss = [FILL; 32 + GUARD];
    w.rc((s.xw_enc_det)(xct.as_mut_ptr(), xss.as_mut_ptr(), xpk.as_ptr(), seed64.as_ptr()));
    w.out(&xct, 1120);
    w.out(&xss, 32);
    let mut xss2 = [FILL; 32 + GUARD];
    w.rc((s.xw_dec)(xss2.as_mut_ptr(), xct.as_ptr(), xsk.as_ptr()));
    w.out(&xss2, 32);
    let mut prk = [FILL; 32 + GUARD];
    w.rc((s.hk_extract)(prk.as_mut_ptr(), k16.as_ptr(), 16, xss.as_ptr(), 32));
    w.out(&prk, 32);
    let mut okm = [FILL; 56 + GUARD];
    let ctx = b"t11-subset";
    w.rc((s.hk_expand)(okm.as_mut_ptr(), 56, ctx.as_ptr() as *const c_char, ctx.len(), prk.as_ptr()));
    w.out(&okm, 56);
    let ad: [u8; 8] = std::array::from_fn(|i| 0xd0 ^ (i as u8));
    let mut ac = [FILL; 48 + GUARD];
    let mut aclen: u64 = 0;
    w.rc((s.aead_enc)(
        ac.as_mut_ptr(),
        &mut aclen,
        msg.as_ptr(),
        32,
        ad.as_ptr(),
        8,
        std::ptr::null(),
        okm[32..].as_ptr(),
        okm.as_ptr(),
    ));
    w.out(&ac, 48);
    w.push(&aclen.to_le_bytes());
    let mut ap = [FILL; 32 + GUARD];
    let mut aplen: u64 = 0;
    w.rc((s.aead_dec)(
        ap.as_mut_ptr(),
        &mut aplen,
        std::ptr::null_mut(),
        ac.as_ptr(),
        48,
        ad.as_ptr(),
        8,
        okm[32..].as_ptr(),
        okm.as_ptr(),
    ));
    w.out(&ap, 32);
    w.push(&aplen.to_le_bytes());

    w.finish();
}

/// Run the subset in the current (post-`sodium_init`) process.
fn subset_now(lib: &'static Library) -> Vec<u8> {
    let s = resolve_subset(lib);
    let mut buf = vec![0u8; SUBSET_REGION];
    unsafe { run_subset(&s, buf.as_mut_ptr()) };
    extract_region(&buf)
}

fn extract_region(buf: &[u8]) -> Vec<u8> {
    let n = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
    assert!(n > 0 && 4 + n <= buf.len(), "subset region length {n} is bogus");
    buf[4..4 + n].to_vec()
}

/// Fork BEFORE `sodium_init()` and record the subset for both libraries.
fn capture_preinit() -> PreinitCapture {
    let l = libs(); // dlopen only — does NOT call sodium_init()
    let sc = resolve_subset(&l.c);
    let sr = resolve_subset(&l.r);
    let shm = Shm::new(2 * SUBSET_REGION);
    let base = shm.base;
    let out = forked(move || {
        unsafe {
            run_subset(&sc, base);
            run_subset(&sr, base.add(SUBSET_REGION));
        }
        0
    });
    assert_eq!(
        out,
        Outcome::Returned(0),
        "CONFIGS 278: the pre-sodium_init() child died ({out:?}) — every call in the \
         subset must be valid before initialisation"
    );
    let all = shm.slice();
    PreinitCapture {
        c: extract_region(&all[..SUBSET_REGION]),
        r: extract_region(&all[SUBSET_REGION..]),
    }
}

// ============================================================ trace plumbing

type Trace = Vec<(String, Vec<u8>)>;

fn tp(tr: &mut Trace, label: &str, b: &[u8]) {
    tr.push((label.to_string(), b.to_vec()));
}

fn trc(tr: &mut Trace, label: &str, rc: c_int) {
    tr.push((label.to_string(), (rc as i32).to_le_bytes().to_vec()));
}

fn cmp_trace(what: &str, c: &Trace, r: &Trace) {
    assert_eq!(
        c.len(),
        r.len(),
        "{what}: pipeline trace length differs (C {} steps, rust {} steps)",
        c.len(),
        r.len()
    );
    for (i, ((cl, cv), (rl, rv))) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(cl, rl, "{what}: trace step {i} label differs ({cl} vs {rl})");
        assert_eq_bytes(&format!("{what} step {i} `{cl}`"), cv, rv);
    }
}

fn both_libs() -> [&'static Library; 2] {
    let l = libs();
    [&l.c, &l.r]
}

// ##########################################################################
// #  PART 1 — crypto_ipcrypt (CONFIGS 265–273, ERRORS 392–394)
// ##########################################################################

/// Keys for the 16-byte-key modes: fixed patterns + randoms.
fn keys16(rng: &mut Rng, extra: usize) -> Vec<Vec<u8>> {
    let mut v = vec![vec![0u8; 16], vec![0xffu8; 16], (0..16).map(|i| i as u8).collect()];
    for _ in 0..extra {
        v.push(rng.bytes(16));
    }
    v
}

/// Keys for the 32-byte-key modes with GUARANTEED DISTINCT halves.
fn keys32_distinct(rng: &mut Rng, extra: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        {
            let mut k = vec![0u8; 32];
            k[16] = 0x01; // halves must differ
            k
        },
        {
            let mut k = vec![0xffu8; 32];
            k[0] = 0xfe;
            k
        },
        (0..32).map(|i| i as u8).collect(),
    ];
    for _ in 0..extra {
        let mut k = rng.bytes(32);
        if k[..16] == k[16..] {
            k[0] ^= 0x01;
        }
        v.push(k);
    }
    v
}

fn ips(rng: &mut Rng, extra: usize) -> Vec<Vec<u8>> {
    let mut v = vec![
        vec![0u8; 16],
        vec![0xffu8; 16],
        ipv4_mapped([192, 0, 2, 1]).to_vec(),
        ipv4_mapped([0, 0, 0, 0]).to_vec(),
        ipv4_mapped([255, 255, 255, 255]).to_vec(),
        IP6_2001DB8_1.to_vec(),
    ];
    for _ in 0..extra {
        v.push(rng.bytes(16));
    }
    v
}

/// CONFIGS 265 — `crypto_ipcrypt_encrypt` / `_decrypt`, deterministic mode.
#[test]
fn cfg265_ipcrypt_encrypt_decrypt() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_encrypt", Ipc3);
    let (cd, rd) = fnpair!("crypto_ipcrypt_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 265);
    let ks = keys16(&mut rng, 8);
    let is = ips(&mut rng, 8);

    let mut n = 0usize;
    for k in &ks {
        for ip in &is {
            let tag = format!("CONFIGS265 encrypt key={} ip={}", hexs(k), hexs(ip));
            let mut oc = ob(16);
            let mut or = ob(16);
            unsafe {
                ce(oc.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
                re(or.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            }
            cmp_out(&tag, 16, &oc, &or);

            // round-trip, asserted independently in BOTH libraries
            let mut pc = ob(16);
            let mut pr = ob(16);
            unsafe {
                cd(pc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                rd(pr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
            }
            cmp_out(&format!("CONFIGS265 decrypt(ct) key={}", hexs(k)), 16, &pc, &pr);
            assert_eq_bytes(&format!("{tag}: C round-trip"), ip, &pc[..16]);
            assert_eq_bytes(&format!("{tag}: rust round-trip"), ip, &pr[..16]);

            // raw decrypt of an arbitrary 16-byte block (decrypt is total)
            let ct = rng.bytes(16);
            let mut xc = ob(16);
            let mut xr = ob(16);
            unsafe {
                cd(xc.as_mut_ptr(), ct.as_ptr(), k.as_ptr());
                rd(xr.as_mut_ptr(), ct.as_ptr(), k.as_ptr());
            }
            cmp_out(&format!("CONFIGS265 raw decrypt key={} ct={}", hexs(k), hexs(&ct)), 16, &xc, &xr);
            n += 1;
        }
    }
    assert!(n >= 64, "CONFIGS 265 drove only {n} key x IP combinations (need >= 64)");
    eprintln!("CONFIGS 265: {n} key x IP combinations");
}

/// CONFIGS 266 — `crypto_ipcrypt_nd_encrypt` / `_nd_decrypt` (tweak 8, out 24).
#[test]
fn cfg266_ipcrypt_nd() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_nd_encrypt", Ipc4);
    let (cd, rd) = fnpair!("crypto_ipcrypt_nd_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 266);
    let ks = keys16(&mut rng, 3);
    let ts: Vec<Vec<u8>> = {
        let mut v = vec![vec![0u8; 8], vec![0xffu8; 8], (0..8).map(|i| i as u8).collect()];
        for _ in 0..3 {
            v.push(rng.bytes(8));
        }
        v
    };
    let is = ips(&mut rng, 2);

    let mut n = 0usize;
    for k in &ks {
        for t in &ts {
            for ip in &is {
                let tag = format!("CONFIGS266 nd_encrypt key={} t={} ip={}", hexs(k), hexs(t), hexs(ip));
                let mut oc = ob(24);
                let mut or = ob(24);
                unsafe {
                    ce(oc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    re(or.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                cmp_out(&tag, 24, &oc, &or);
                // out = tweak || ct
                assert_eq_bytes(&format!("{tag}: C out[0..8) must be the tweak"), t, &oc[..8]);
                assert_eq_bytes(&format!("{tag}: rust out[0..8) must be the tweak"), t, &or[..8]);

                // `_nd_decrypt` recovers the tweak from in[0..8)
                let mut pc = ob(16);
                let mut pr = ob(16);
                unsafe {
                    cd(pc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                    rd(pr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
                }
                cmp_out(&format!("CONFIGS266 nd_decrypt {tag}"), 16, &pc, &pr);
                assert_eq_bytes(&format!("{tag}: C nd round-trip"), ip, &pc[..16]);
                assert_eq_bytes(&format!("{tag}: rust nd round-trip"), ip, &pr[..16]);
                n += 1;
            }
        }
    }

    // `_nd_decrypt` on an arbitrary 24-byte blob: the tweak comes from in[0..8).
    for _ in 0..64 {
        let k = rng.bytes(16);
        let blob = rng.bytes(24);
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cd(pc.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            rd(pr.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("CONFIGS266 nd_decrypt(raw) k={} in={}", hexs(&k), hexs(&blob)), 16, &pc, &pr);
        n += 1;
    }
    assert!(n >= 64, "CONFIGS 266 drove only {n} combinations (need >= 64)");
    eprintln!("CONFIGS 266: {n} key x tweak x IP combinations");
}

/// CONFIGS 267 — `crypto_ipcrypt_ndx_*` with DISTINCT key halves.
#[test]
fn cfg267_ipcrypt_ndx_distinct_halves() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_ndx_encrypt", Ipc4);
    let (cd, rd) = fnpair!("crypto_ipcrypt_ndx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 267);
    let ks = keys32_distinct(&mut rng, 3);
    let ts: Vec<Vec<u8>> = {
        let mut v = vec![vec![0u8; 16], vec![0xffu8; 16], (0..16).map(|i| i as u8).collect()];
        for _ in 0..3 {
            v.push(rng.bytes(16));
        }
        v
    };
    let is = ips(&mut rng, 2);

    let mut n = 0usize;
    for k in &ks {
        assert_ne!(&k[..16], &k[16..], "CONFIGS 267 needs distinct halves");
        for t in &ts {
            for ip in &is {
                let tag = format!("CONFIGS267 ndx_encrypt key={} t={} ip={}", hexs(k), hexs(t), hexs(ip));
                let mut oc = ob(32);
                let mut or = ob(32);
                unsafe {
                    ce(oc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                    re(or.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                }
                cmp_out(&tag, 32, &oc, &or);
                assert_eq_bytes(&format!("{tag}: C out[0..16) must be the tweak"), t, &oc[..16]);
                assert_eq_bytes(&format!("{tag}: rust out[0..16) must be the tweak"), t, &or[..16]);

                let mut pc = ob(16);
                let mut pr = ob(16);
                unsafe {
                    cd(pc.as_mut_ptr(), oc.as_ptr(), k.as_ptr());
                    rd(pr.as_mut_ptr(), or.as_ptr(), k.as_ptr());
                }
                cmp_out(&format!("CONFIGS267 ndx_decrypt {tag}"), 16, &pc, &pr);
                assert_eq_bytes(&format!("{tag}: C ndx round-trip"), ip, &pc[..16]);
                assert_eq_bytes(&format!("{tag}: rust ndx round-trip"), ip, &pr[..16]);
                n += 1;
            }
        }
    }
    // `_ndx_decrypt` reads the tweak from in[0..16) — arbitrary 32-byte blobs.
    for _ in 0..64 {
        let mut k = rng.bytes(32);
        if k[..16] == k[16..] {
            k[0] ^= 1;
        }
        let blob = rng.bytes(32);
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cd(pc.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
            rd(pr.as_mut_ptr(), blob.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("CONFIGS267 ndx_decrypt(raw) k={} in={}", hexs(&k), hexs(&blob)), 16, &pc, &pr);
        n += 1;
    }
    assert!(n >= 64, "CONFIGS 267 drove only {n} combinations (need >= 64)");
    eprintln!("CONFIGS 267: {n} key x tweak x IP combinations");
}

/// CONFIGS 268 / ERRORS 393 — `crypto_ipcrypt_ndx_*` with IDENTICAL key halves.
///
/// The C re-derives the second schedule from `k[i] ^ 0x5a`, so `A||A` must
/// behave EXACTLY like the non-degenerate key `(A^0x5a) || A` (`tkeys` come from
/// `k+16`, `rkeys` from `k`). That equivalence is asserted inside each library
/// separately, which proves the branch is actually taken, and the raw bytes are
/// compared across libraries.
#[test]
fn cfg268_ipcrypt_ndx_identical_halves() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_ndx_encrypt", Ipc4);
    let (cd, rd) = fnpair!("crypto_ipcrypt_ndx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 268);

    let mut halves: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16], (0..16).map(|i| i as u8).collect()];
    for _ in 0..64 {
        halves.push(rng.bytes(16));
    }

    let mut n = 0usize;
    for a in &halves {
        let mut kdeg = a.clone();
        kdeg.extend_from_slice(a); // A || A  -> degenerate
        let mut kequiv: Vec<u8> = a.iter().map(|&x| x ^ 0x5a).collect();
        kequiv.extend_from_slice(a); // (A^0x5a) || A -> same schedules, non-degenerate
        assert_ne!(&kequiv[..16], &kequiv[16..]);

        let t = rng.bytes(16);
        let ip = if n % 3 == 0 { ipv4_mapped([10, 1, 2, 3]).to_vec() } else { rng.bytes(16) };
        let tag = format!("CONFIGS268/ERRORS393 ndx A={} t={} ip={}", hexs(a), hexs(&t), hexs(&ip));

        let mut oc = ob(32);
        let mut or = ob(32);
        unsafe {
            ce(oc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kdeg.as_ptr());
            re(or.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kdeg.as_ptr());
        }
        cmp_out(&tag, 32, &oc, &or);

        let mut qc = ob(32);
        let mut qr = ob(32);
        unsafe {
            ce(qc.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kequiv.as_ptr());
            re(qr.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kequiv.as_ptr());
        }
        cmp_out(&format!("{tag} (equivalent key)"), 32, &qc, &qr);
        assert_eq_bytes(
            &format!("{tag}: C did NOT re-derive the 2nd schedule from k[i]^0x5a"),
            &oc,
            &qc,
        );
        assert_eq_bytes(
            &format!("{tag}: rust did NOT re-derive the 2nd schedule from k[i]^0x5a"),
            &or,
            &qr,
        );

        // round-trip must still work with the degenerate key
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cd(pc.as_mut_ptr(), oc.as_ptr(), kdeg.as_ptr());
            rd(pr.as_mut_ptr(), or.as_ptr(), kdeg.as_ptr());
        }
        cmp_out(&format!("{tag} ndx_decrypt"), 16, &pc, &pr);
        assert_eq_bytes(&format!("{tag}: C degenerate-key round-trip"), &ip, &pc[..16]);
        assert_eq_bytes(&format!("{tag}: rust degenerate-key round-trip"), &ip, &pr[..16]);
        n += 1;
    }
    assert!(n >= 64, "CONFIGS 268 drove only {n} degenerate keys (need >= 64)");
    eprintln!("CONFIGS 268 / ERRORS 393 (ndx): {n} identical-halves keys");
}

/// CONFIGS 269 — `crypto_ipcrypt_pfx_*`: IPv4-mapped input starts at prefix bit
/// 96 and forces `out[10..12) = 0xffff`; native IPv6 starts at prefix bit 0.
#[test]
fn cfg269_ipcrypt_pfx_ipv4_mapped_vs_native() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_pfx_encrypt", Ipc3);
    let (cd, rd) = fnpair!("crypto_ipcrypt_pfx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 269);

    let mut n = 0usize;
    let mut v4_seen = 0usize;
    let mut v6_seen = 0usize;
    for it in 0..64 {
        let mut k = rng.bytes(32);
        if k[..16] == k[16..] {
            k[0] ^= 1;
        }
        // --- IPv4-mapped: prefix_start == 96 ---
        let mut o4 = [0u8; 4];
        rng.fill(&mut o4);
        let v4 = if it == 0 { ipv4_mapped([192, 0, 2, 1]) } else { ipv4_mapped(o4) };
        let tag4 = format!("CONFIGS269 pfx v4-mapped key={} ip={}", hexs(&k), hexs(&v4));
        let mut c4 = ob(16);
        let mut r4 = ob(16);
        unsafe {
            ce(c4.as_mut_ptr(), v4.as_ptr(), k.as_ptr());
            re(r4.as_mut_ptr(), v4.as_ptr(), k.as_ptr());
        }
        cmp_out(&tag4, 16, &c4, &r4);
        for (who, o) in [("C", &c4), ("rust", &r4)] {
            assert!(
                o[..10].iter().all(|&x| x == 0) && o[10] == 0xff && o[11] == 0xff,
                "{tag4}: {who} must keep the IPv4-mapped prefix (out[0..10)=0, out[10..12)=0xffff); got {}",
                hexs(&o[..16])
            );
        }
        // round-trip (the ciphertext is itself IPv4-mapped, so decrypt also starts at 96)
        let mut p4c = ob(16);
        let mut p4r = ob(16);
        unsafe {
            cd(p4c.as_mut_ptr(), c4.as_ptr(), k.as_ptr());
            rd(p4r.as_mut_ptr(), r4.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("{tag4} decrypt"), 16, &p4c, &p4r);
        assert_eq_bytes(&format!("{tag4}: C round-trip"), &v4, &p4c[..16]);
        assert_eq_bytes(&format!("{tag4}: rust round-trip"), &v4, &p4r[..16]);
        v4_seen += 1;

        // --- the SAME 16 bytes but NOT IPv4-mapped: prefix_start == 0 ---
        let mut v6 = v4;
        v6[10] = 0xfe; // breaks the ::ffff: prefix
        assert!(!is_v4_mapped(&v6));
        let tag6 = format!("CONFIGS269 pfx native-v6 key={} ip={}", hexs(&k), hexs(&v6));
        let mut c6 = ob(16);
        let mut r6 = ob(16);
        unsafe {
            ce(c6.as_mut_ptr(), v6.as_ptr(), k.as_ptr());
            re(r6.as_mut_ptr(), v6.as_ptr(), k.as_ptr());
        }
        cmp_out(&tag6, 16, &c6, &r6);
        for (who, o) in [("C", &c6), ("rust", &r6)] {
            assert!(
                !(o[..10].iter().all(|&x| x == 0) && o[10] == 0xff && o[11] == 0xff),
                "{tag6}: {who} treated a NON-IPv4-mapped address as IPv4-mapped \
                 (prefix_start must be 0, not 96); got {}",
                hexs(&o[..16])
            );
        }
        let mut p6c = ob(16);
        let mut p6r = ob(16);
        unsafe {
            cd(p6c.as_mut_ptr(), c6.as_ptr(), k.as_ptr());
            rd(p6r.as_mut_ptr(), r6.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("{tag6} decrypt"), 16, &p6c, &p6r);
        assert_eq_bytes(&format!("{tag6}: C round-trip"), &v6, &p6c[..16]);
        assert_eq_bytes(&format!("{tag6}: rust round-trip"), &v6, &p6r[..16]);
        v6_seen += 1;
        n += 2;
    }

    // fixed shapes: all-zero and all-0xff are NOT IPv4-mapped
    for ip in [[0u8; 16], [0xffu8; 16], IP6_2001DB8_1] {
        assert!(!is_v4_mapped(&ip));
        let k: Vec<u8> = (0..32).map(|i| i as u8).collect();
        let mut c = ob(16);
        let mut r = ob(16);
        unsafe {
            ce(c.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            re(r.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("CONFIGS269 pfx fixed ip={}", hexs(&ip)), 16, &c, &r);
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cd(pc.as_mut_ptr(), c.as_ptr(), k.as_ptr());
            rd(pr.as_mut_ptr(), r.as_ptr(), k.as_ptr());
        }
        cmp_out(&format!("CONFIGS269 pfx fixed decrypt ip={}", hexs(&ip)), 16, &pc, &pr);
        assert_eq_bytes("CONFIGS269 C fixed round-trip", &ip, &pc[..16]);
        assert_eq_bytes("CONFIGS269 rust fixed round-trip", &ip, &pr[..16]);
        n += 1;
    }
    assert_eq!(v4_seen, 64);
    assert_eq!(v6_seen, 64);
    assert!(n >= 64, "CONFIGS 269 drove only {n} inputs (need >= 64)");
    eprintln!("CONFIGS 269: {n} pfx inputs ({v4_seen} IPv4-mapped, {v6_seen} native IPv6)");
}

/// CONFIGS 270 — `pfx` prefix preservation and the identical-key-halves path.
#[test]
fn cfg270_ipcrypt_pfx_prefix_preservation() {
    init_all();
    let (ce, re) = fnpair!("crypto_ipcrypt_pfx_encrypt", Ipc3);
    let (cd, rd) = fnpair!("crypto_ipcrypt_pfx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 270);

    // The prefix lengths named in the row (plus 128 = identical addresses).
    const NS: [usize; 9] = [0, 1, 8, 32, 64, 96, 120, 127, 128];

    let mut n = 0usize;
    for &want in NS.iter() {
        for it in 0..8 {
            let (a, b) = pfx_pair(&mut rng, want);
            // alternate between a normal key and a degenerate identical-halves key
            let key: Vec<u8> = if it % 2 == 0 {
                let mut k = rng.bytes(32);
                if k[..16] == k[16..] {
                    k[0] ^= 1;
                }
                k
            } else {
                let h = rng.bytes(16);
                let mut k = h.clone();
                k.extend_from_slice(&h);
                k
            };
            let tag = format!("CONFIGS270 pfx n={want} key={} a={} b={}", hexs(&key), hexs(&a), hexs(&b));

            let mut ca = ob(16);
            let mut ra = ob(16);
            let mut cb = ob(16);
            let mut rb = ob(16);
            unsafe {
                ce(ca.as_mut_ptr(), a.as_ptr(), key.as_ptr());
                re(ra.as_mut_ptr(), a.as_ptr(), key.as_ptr());
                ce(cb.as_mut_ptr(), b.as_ptr(), key.as_ptr());
                re(rb.as_mut_ptr(), b.as_ptr(), key.as_ptr());
            }
            cmp_out(&format!("{tag} E(a)"), 16, &ca, &ra);
            cmp_out(&format!("{tag} E(b)"), 16, &cb, &rb);

            for (who, x, y) in [("C", &ca, &cb), ("rust", &ra, &rb)] {
                let got = shared_pbits(&x[..16], &y[..16]);
                assert_eq!(
                    got, want,
                    "{tag}: {who} broke prefix preservation — inputs share exactly {want} \
                     leading bits but ciphertexts share {got}\n  E(a)={}\n  E(b)={}",
                    hexs(&x[..16]),
                    hexs(&y[..16])
                );
            }

            // and decryption inverts it
            let mut pa = ob(16);
            let mut qa = ob(16);
            unsafe {
                cd(pa.as_mut_ptr(), ca.as_ptr(), key.as_ptr());
                rd(qa.as_mut_ptr(), ra.as_ptr(), key.as_ptr());
            }
            cmp_out(&format!("{tag} D(E(a))"), 16, &pa, &qa);
            assert_eq_bytes(&format!("{tag}: C round-trip"), &a, &pa[..16]);
            assert_eq_bytes(&format!("{tag}: rust round-trip"), &a, &qa[..16]);
            n += 1;
        }
    }

    // IPv4-mapped pairs: both start at prefix bit 96, so they share >= 96 bits
    // and the ciphertexts must share exactly as many bits as the inputs.
    for _ in 0..16 {
        let mut k = rng.bytes(32);
        if k[..16] == k[16..] {
            k[0] ^= 1;
        }
        let mut o1 = [0u8; 4];
        rng.fill(&mut o1);
        let mut o2 = o1;
        let bit = rng.below(32);
        // flip one bit of the 32-bit v4 field and randomise everything below it
        let a = ipv4_mapped(o1);
        {
            let p = 96 + bit;
            let mut tmp = a;
            set_pbit(&mut tmp, p, 1 ^ get_pbit(&a, p));
            for q in (p + 1)..128 {
                set_pbit(&mut tmp, q, rng.byte() & 1);
            }
            o2.copy_from_slice(&tmp[12..16]);
        }
        let b = ipv4_mapped(o2);
        assert!(is_v4_mapped(&a) && is_v4_mapped(&b));
        let want = shared_pbits(&a, &b);
        assert!(want >= 96, "IPv4-mapped pair must share >= 96 bits, got {want}");
        let mut ca = ob(16);
        let mut ra = ob(16);
        let mut cb = ob(16);
        let mut rb = ob(16);
        unsafe {
            ce(ca.as_mut_ptr(), a.as_ptr(), k.as_ptr());
            re(ra.as_mut_ptr(), a.as_ptr(), k.as_ptr());
            ce(cb.as_mut_ptr(), b.as_ptr(), k.as_ptr());
            re(rb.as_mut_ptr(), b.as_ptr(), k.as_ptr());
        }
        cmp_out("CONFIGS270 v4 E(a)", 16, &ca, &ra);
        cmp_out("CONFIGS270 v4 E(b)", 16, &cb, &rb);
        for (who, x, y) in [("C", &ca, &cb), ("rust", &ra, &rb)] {
            let got = shared_pbits(&x[..16], &y[..16]);
            assert_eq!(
                got, want,
                "CONFIGS270: {who} broke IPv4-mapped prefix preservation (want {want}, got {got})"
            );
        }
        n += 1;
    }

    assert!(n >= 64, "CONFIGS 270 drove only {n} prefix pairs (need >= 64)");
    eprintln!("CONFIGS 270: {n} prefix-sharing pairs over lengths {NS:?}");
}

/// CONFIGS 271 — all four `*_keygen` functions under the deterministic RNG.
#[test]
fn cfg271_ipcrypt_all_keygens() {
    init_all();
    let _sess = RngSession::new(false);
    let (cbuf, rbuf) = fnpair!("randombytes_buf", unsafe extern "C" fn(*mut c_void, usize));

    let mut n = 0usize;
    // (a) each keygen must consume EXACTLY keybytes from randombytes_buf
    for (name, len) in IPC_KEYGENS {
        let (ck, rk) = unsafe { pair::<KeygenFn>(name) };
        for who in 0..2 {
            reset_det_rng();
            let mut expect = vec![0u8; len];
            unsafe {
                if who == 0 {
                    cbuf(expect.as_mut_ptr() as *mut c_void, len);
                } else {
                    rbuf(expect.as_mut_ptr() as *mut c_void, len);
                }
            }
            reset_det_rng();
            let mut got = ob(len);
            unsafe {
                if who == 0 {
                    ck(got.as_mut_ptr());
                } else {
                    rk(got.as_mut_ptr());
                }
            }
            let lbl = if who == 0 { "C" } else { "rust" };
            guard(&format!("CONFIGS271 {name} [{lbl}]"), &got, len);
            assert_eq_bytes(
                &format!("CONFIGS271 {name} [{lbl}] must be exactly randombytes_buf({len})"),
                &expect,
                &got[..len],
            );
        }
    }

    // (b) long lockstep run: both libraries have independent counters that
    // advance identically, so C and rust output must match every time.
    reset_det_rng();
    for _ in 0..64 {
        for (name, len) in IPC_KEYGENS {
            let (ck, rk) = unsafe { pair::<KeygenFn>(name) };
            let mut kc = ob(len);
            let mut kr = ob(len);
            unsafe {
                ck(kc.as_mut_ptr());
                rk(kr.as_mut_ptr());
            }
            cmp_out(&format!("CONFIGS271 {name}"), len, &kc, &kr);
            assert!(
                kc[..len].iter().any(|&x| x != FILL),
                "CONFIGS271 {name} wrote nothing into the output buffer"
            );
            n += 1;
        }
    }
    assert_eq!(n, 256, "CONFIGS 271 must drive 64 iterations x 4 keygens");
    eprintln!("CONFIGS 271: {n} keygen calls (4 keygens x 64 iterations)");
}

/// CONFIGS 272 — `_crypto_ipcrypt_pick_best_implementation` + all 12 getters.
#[test]
fn cfg272_ipcrypt_getters_and_pick_best_implementation() {
    init_all();
    assert_eq!(IPC_GETTERS.len(), 12);
    let expected: Vec<usize> = IPC_GETTERS.iter().map(|g| g.1).collect();
    assert_eq!(expected, vec![16, 16, 16, 8, 16, 24, 32, 16, 16, 32, 32, 16]);

    for (name, want) in IPC_GETTERS {
        let (c, r) = unsafe { pair::<SizeFn>(name) };
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, want, "CONFIGS272: C {name}() = {cv}, header says {want}");
        assert_eq!(rv, want, "CONFIGS272: rust {name}() = {rv}, header says {want}");
    }

    // The picker selects the soft implementation in this build (no HAVE_* macros),
    // returns 0, and is idempotent.
    let (cp, rp) = fnpair!("_crypto_ipcrypt_pick_best_implementation", IntFn);
    let (ce, re) = fnpair!("crypto_ipcrypt_encrypt", Ipc3);
    let (cd, rd) = fnpair!("crypto_ipcrypt_decrypt", Ipc3);
    let key: Vec<u8> = (0..16).map(|i| i as u8).collect();
    let ip = ipv4_mapped([192, 0, 2, 1]);
    let mut base_c = ob(16);
    let mut base_r = ob(16);
    unsafe {
        ce(base_c.as_mut_ptr(), ip.as_ptr(), key.as_ptr());
        re(base_r.as_mut_ptr(), ip.as_ptr(), key.as_ptr());
    }
    cmp_out("CONFIGS272 baseline encrypt", 16, &base_c, &base_r);

    for round in 0..4 {
        let (rc, rr) = unsafe { (cp(), rp()) };
        assert_eq!(rc, 0, "CONFIGS272: C picker returned {rc} on round {round}");
        assert_eq!(rr, 0, "CONFIGS272: rust picker returned {rr} on round {round}");
        let mut c = ob(16);
        let mut r = ob(16);
        unsafe {
            ce(c.as_mut_ptr(), ip.as_ptr(), key.as_ptr());
            re(r.as_mut_ptr(), ip.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS272 encrypt after pick round {round}"), 16, &c, &r);
        assert_eq_bytes(
            &format!("CONFIGS272: C output changed after re-picking (round {round})"),
            &base_c,
            &c,
        );
        assert_eq_bytes(
            &format!("CONFIGS272: rust output changed after re-picking (round {round})"),
            &base_r,
            &r,
        );
        // decrypt still inverts
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cd(pc.as_mut_ptr(), c.as_ptr(), key.as_ptr());
            rd(pr.as_mut_ptr(), r.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS272 decrypt after pick round {round}"), 16, &pc, &pr);
        assert_eq_bytes("CONFIGS272 C round-trip", &ip, &pc[..16]);
        assert_eq_bytes("CONFIGS272 rust round-trip", &ip, &pr[..16]);
    }
    eprintln!("CONFIGS 272: 12 getters verified, picker idempotent over 4 rounds");
}

/// CONFIGS 273 — `sodium_ip2bin` -> ipcrypt -> `sodium_bin2ip` round-trip.
#[test]
fn cfg273_ip2bin_ipcrypt_bin2ip_integration() {
    init_all();
    let (ci2b, ri2b) = fnpair!("sodium_ip2bin", Ip2Bin);
    let (cb2i, rb2i) = fnpair!("sodium_bin2ip", Bin2Ip);
    let (cenc, renc) = fnpair!("crypto_ipcrypt_encrypt", Ipc3);
    let (cdec, rdec) = fnpair!("crypto_ipcrypt_decrypt", Ipc3);
    let (cpfx, rpfx) = fnpair!("crypto_ipcrypt_pfx_encrypt", Ipc3);
    let (cpfxd, rpfxd) = fnpair!("crypto_ipcrypt_pfx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 273);

    const MAXLEN: usize = 46;
    // ip2bin in both libraries; asserts rc + bin + guard.
    let i2b = |s: &str| -> [u8; 16] {
        let cs = std::ffi::CString::new(s).unwrap();
        let mut bc = ob(16);
        let mut br = ob(16);
        let (rc, rr) = unsafe {
            (
                ci2b(bc.as_mut_ptr(), cs.as_ptr(), s.len()),
                ri2b(br.as_mut_ptr(), cs.as_ptr(), s.len()),
            )
        };
        assert_eq!(rc, rr, "CONFIGS273 sodium_ip2bin({s:?}) rc differs (C {rc}, rust {rr})");
        assert_eq!(rc, 0, "CONFIGS273 sodium_ip2bin({s:?}) failed with {rc}");
        cmp_out(&format!("CONFIGS273 ip2bin({s:?})"), 16, &bc, &br);
        bc[..16].try_into().unwrap()
    };
    // bin2ip in both libraries; asserts pointer identity semantics + string.
    let b2i = |bin: &[u8]| -> String {
        let mut sc = vec![0u8; MAXLEN + GUARD];
        let mut sr = vec![0u8; MAXLEN + GUARD];
        sc.iter_mut().for_each(|x| *x = FILL);
        sr.iter_mut().for_each(|x| *x = FILL);
        let (pc, pr) = unsafe {
            (
                cb2i(sc.as_mut_ptr() as *mut c_char, MAXLEN, bin.as_ptr()),
                rb2i(sr.as_mut_ptr() as *mut c_char, MAXLEN, bin.as_ptr()),
            )
        };
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "CONFIGS273 sodium_bin2ip null-ness differs for bin={}",
            hexs(bin)
        );
        assert!(!pc.is_null(), "CONFIGS273 sodium_bin2ip failed for bin={}", hexs(bin));
        assert_eq!(pc as usize, sc.as_ptr() as usize, "C bin2ip must return its `ip` argument");
        assert_eq!(pr as usize, sr.as_ptr() as usize, "rust bin2ip must return its `ip` argument");
        guard("CONFIGS273 bin2ip [C]", &sc, MAXLEN);
        guard("CONFIGS273 bin2ip [rust]", &sr, MAXLEN);
        let a = unsafe { CStr::from_ptr(pc) }.to_str().unwrap().to_string();
        let b = unsafe { CStr::from_ptr(pr) }.to_str().unwrap().to_string();
        assert_eq!(a, b, "CONFIGS273 sodium_bin2ip string differs for bin={}", hexs(bin));
        // Also compare the FULL buffers (tail bytes past the NUL included).
        assert_eq_bytes(&format!("CONFIGS273 bin2ip buffer bin={}", hexs(bin)), &sc, &sr);
        a
    };

    let key: Vec<u8> = (0..16).map(|i| i as u8).collect();
    let pkey: Vec<u8> = (0..32).map(|i| (i as u8) ^ 0x33).collect();

    let mut n = 0usize;
    // (a) the two addresses named in the row, through the deterministic mode
    //     AND through the prefix-preserving mode.
    for s in ["192.0.2.1", "2001:db8::1"] {
        let bin = i2b(s);
        assert_eq!(b2i(&bin), s, "CONFIGS273 {s:?} is not canonical");

        let mut ec = ob(16);
        let mut er = ob(16);
        unsafe {
            cenc(ec.as_mut_ptr(), bin.as_ptr(), key.as_ptr());
            renc(er.as_mut_ptr(), bin.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 encrypt({s:?})"), 16, &ec, &er);
        let ctstr = b2i(&ec[..16]);
        let back = i2b(&ctstr);
        assert_eq_bytes(&format!("CONFIGS273 bin2ip/ip2bin({s:?}) not an involution"), &ec[..16], &back);
        let mut dc = ob(16);
        let mut dr = ob(16);
        unsafe {
            cdec(dc.as_mut_ptr(), back.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), back.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 decrypt({s:?})"), 16, &dc, &dr);
        assert_eq_bytes(&format!("CONFIGS273 {s:?} full round-trip"), &bin, &dc[..16]);
        assert_eq!(b2i(&dc[..16]), s, "CONFIGS273 {s:?} did not come back as text");

        // pfx keeps the address family, so the text form keeps its shape.
        let mut pc = ob(16);
        let mut pr = ob(16);
        unsafe {
            cpfx(pc.as_mut_ptr(), bin.as_ptr(), pkey.as_ptr());
            rpfx(pr.as_mut_ptr(), bin.as_ptr(), pkey.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 pfx_encrypt({s:?})"), 16, &pc, &pr);
        let ptxt = b2i(&pc[..16]);
        assert_eq!(
            ptxt.contains('.'),
            s.contains('.'),
            "CONFIGS273 pfx changed the address family: {s:?} -> {ptxt:?}"
        );
        let pback = i2b(&ptxt);
        assert_eq_bytes("CONFIGS273 pfx text round-trip", &pc[..16], &pback);
        let mut qc = ob(16);
        let mut qr = ob(16);
        unsafe {
            cpfxd(qc.as_mut_ptr(), pback.as_ptr(), pkey.as_ptr());
            rpfxd(qr.as_mut_ptr(), pback.as_ptr(), pkey.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 pfx_decrypt({s:?})"), 16, &qc, &qr);
        assert_eq_bytes("CONFIGS273 pfx full round-trip", &bin, &qc[..16]);
        n += 1;
    }

    // (b) randomised addresses: random 16-byte bin -> text -> bin -> ipcrypt -> text -> bin
    for i in 0..64 {
        let mut bin = rng.bytes(16);
        if i % 4 == 0 {
            // force an IPv4-mapped shape so the dotted-quad text path is hit too
            let o: [u8; 4] = [bin[12], bin[13], bin[14], bin[15]];
            bin = ipv4_mapped(o).to_vec();
        }
        let s = b2i(&bin);
        let back = i2b(&s);
        assert_eq_bytes(&format!("CONFIGS273 bin2ip/ip2bin({s:?})"), &bin, &back);

        let mut ec = ob(16);
        let mut er = ob(16);
        unsafe {
            cenc(ec.as_mut_ptr(), bin.as_ptr(), key.as_ptr());
            renc(er.as_mut_ptr(), bin.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 encrypt({s:?})"), 16, &ec, &er);
        let cts = b2i(&ec[..16]);
        let ctb = i2b(&cts);
        assert_eq_bytes("CONFIGS273 ciphertext text round-trip", &ec[..16], &ctb);
        let mut dc = ob(16);
        let mut dr = ob(16);
        unsafe {
            cdec(dc.as_mut_ptr(), ctb.as_ptr(), key.as_ptr());
            rdec(dr.as_mut_ptr(), ctb.as_ptr(), key.as_ptr());
        }
        cmp_out(&format!("CONFIGS273 decrypt({cts:?})"), 16, &dc, &dr);
        assert_eq_bytes("CONFIGS273 randomised full round-trip", &bin, &dc[..16]);
        assert_eq!(b2i(&dc[..16]), s);
        n += 1;
    }
    assert!(n >= 64, "CONFIGS 273 drove only {n} addresses (need >= 64)");
    eprintln!("CONFIGS 273: {n} ip2bin -> ipcrypt -> bin2ip round-trips");
}

// ------------------------------------------------------------- ERRORS 392–394

/// Exercise every ipcrypt entry point with adversarial-but-valid inputs.
/// Stack-only (fork safe) and returns 0 unless a call misbehaves.
unsafe fn exercise_ipc_all(s: &IpcSyms) -> i64 {
    let zero16 = [0u8; 16];
    let ff16 = [0xffu8; 16];
    let zero32 = [0u8; 32];
    let ff32 = [0xffu8; 32];
    let ip4 = ipv4_mapped([0, 0, 0, 0]);
    let ip6 = IP6_2001DB8_1;
    let t8z = [0u8; 8];
    let t8f = [0xffu8; 8];

    let mut o16 = [0u8; 16];
    let mut o24 = [0u8; 24];
    let mut o32 = [0u8; 32];

    for k in [&zero16, &ff16] {
        for ip in [&zero16, &ff16, &ip4, &ip6] {
            (s.enc)(o16.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            (s.dec)(o16.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            for t in [&t8z, &t8f] {
                (s.nd_enc)(o24.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                (s.nd_dec)(o16.as_mut_ptr(), o24.as_ptr(), k.as_ptr());
            }
        }
    }
    // 32-byte-key modes, including the degenerate identical-halves keys
    for k in [&zero32, &ff32] {
        for ip in [&zero16, &ff16, &ip4, &ip6] {
            for t in [&zero16, &ff16] {
                (s.ndx_enc)(o32.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), k.as_ptr());
                (s.ndx_dec)(o16.as_mut_ptr(), o32.as_ptr(), k.as_ptr());
            }
            (s.pfx_enc)(o16.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
            (s.pfx_dec)(o16.as_mut_ptr(), ip.as_ptr(), k.as_ptr());
        }
    }
    for i in 0..4 {
        (s.keygen[i])(o32.as_mut_ptr());
    }
    let mut acc = 0usize;
    for i in 0..12 {
        acc += (s.getters[i])();
    }
    if acc != 16 + 16 + 16 + 8 + 16 + 24 + 32 + 16 + 16 + 32 + 32 + 16 {
        return 7;
    }
    if (s.pick)() != 0 {
        return 8;
    }
    0
}

/// ERRORS 392 — `crypto_ipcrypt_*` has NO error paths: every entry point
/// returns `void`, takes no length argument and can never signal failure or
/// reach `sodium_misuse()`. Asserted by driving every entry point (including
/// the degenerate all-zero / all-0xff keys) in a forked child for each library
/// and requiring a clean exit from both.
#[test]
fn err392_ipcrypt_has_no_error_paths() {
    init_all();
    let l = libs();
    let sc = resolve_ipc(&l.c);
    let sr = resolve_ipc(&l.r);

    let oc = forked(|| unsafe { exercise_ipc_all(&sc) });
    let or = forked(|| unsafe { exercise_ipc_all(&sr) });
    assert_same_fatal("ERRORS 392 crypto_ipcrypt_* (no error paths)", oc, or);
    assert_eq!(
        oc,
        Outcome::Returned(0),
        "ERRORS 392: the C library did not run every ipcrypt entry point cleanly ({oc:?})"
    );
    assert_eq!(
        or,
        Outcome::Returned(0),
        "ERRORS 392: the rust library did not run every ipcrypt entry point cleanly ({or:?})"
    );

    // Same thing in-process (so a hang/abort would also be visible here), with
    // every output buffer guarded and compared.
    let key: Vec<u8> = vec![0u8; 32];
    let ip = vec![0u8; 16];
    let t = vec![0u8; 16];
    let (cne, rne) = fnpair!("crypto_ipcrypt_ndx_encrypt", Ipc4);
    let mut a = ob(32);
    let mut b = ob(32);
    unsafe {
        cne(a.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), key.as_ptr());
        rne(b.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), key.as_ptr());
    }
    cmp_out("ERRORS392 ndx_encrypt(all-zero key)", 32, &a, &b);
    eprintln!("ERRORS 392: all ipcrypt entry points return void, no error path reachable");
}

/// ERRORS 393 — degenerate key with equal 16-byte halves: no error, the second
/// schedule is silently re-derived from `k[i] ^ 0x5a`, for BOTH `ndx_*` and
/// `pfx_*`. Proven by the `A||A` == `(A^0x5a)||A` (ndx) and
/// `A||A` == `A||(A^0x5a)` (pfx) equivalences inside each library.
#[test]
fn err393_degenerate_identical_key_halves() {
    init_all();
    let (cnx, rnx) = fnpair!("crypto_ipcrypt_ndx_encrypt", Ipc4);
    let (cnxd, rnxd) = fnpair!("crypto_ipcrypt_ndx_decrypt", Ipc3);
    let (cpf, rpf) = fnpair!("crypto_ipcrypt_pfx_encrypt", Ipc3);
    let (cpfd, rpfd) = fnpair!("crypto_ipcrypt_pfx_decrypt", Ipc3);
    let mut rng = Rng::new(SEED ^ 393);

    let mut halves: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16], (0..16).map(|i| i as u8).collect()];
    for _ in 0..64 {
        halves.push(rng.bytes(16));
    }

    let mut n_ndx = 0usize;
    let mut n_pfx = 0usize;
    for (idx, a) in halves.iter().enumerate() {
        let flipped: Vec<u8> = a.iter().map(|&x| x ^ 0x5a).collect();
        let mut kdeg = a.clone();
        kdeg.extend_from_slice(a);

        // --- ndx: tkeys <- k+16, rkeys <- k, so the equivalent key is (A^0x5a)||A
        let mut kx = flipped.clone();
        kx.extend_from_slice(a);
        let t = rng.bytes(16);
        let ip = rng.bytes(16);
        let tag = format!("ERRORS393 ndx A={} t={} ip={}", hexs(a), hexs(&t), hexs(&ip));
        let mut c1 = ob(32);
        let mut r1 = ob(32);
        let mut c2 = ob(32);
        let mut r2 = ob(32);
        unsafe {
            cnx(c1.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kdeg.as_ptr());
            rnx(r1.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kdeg.as_ptr());
            cnx(c2.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kx.as_ptr());
            rnx(r2.as_mut_ptr(), ip.as_ptr(), t.as_ptr(), kx.as_ptr());
        }
        cmp_out(&format!("{tag} degenerate"), 32, &c1, &r1);
        cmp_out(&format!("{tag} equivalent"), 32, &c2, &r2);
        assert_eq_bytes(&format!("{tag}: C ^0x5a re-derivation missing"), &c1, &c2);
        assert_eq_bytes(&format!("{tag}: rust ^0x5a re-derivation missing"), &r1, &r2);
        let mut p1 = ob(16);
        let mut q1 = ob(16);
        unsafe {
            cnxd(p1.as_mut_ptr(), c1.as_ptr(), kdeg.as_ptr());
            rnxd(q1.as_mut_ptr(), r1.as_ptr(), kdeg.as_ptr());
        }
        cmp_out(&format!("{tag} ndx_decrypt"), 16, &p1, &q1);
        assert_eq_bytes(&format!("{tag}: C round-trip"), &ip, &p1[..16]);
        assert_eq_bytes(&format!("{tag}: rust round-trip"), &ip, &q1[..16]);
        n_ndx += 1;

        // --- pfx: k1keys <- k, k2keys <- k+16, so the equivalent key is A||(A^0x5a)
        if idx % 2 == 0 {
            let mut kp = a.clone();
            kp.extend_from_slice(&flipped);
            let ipp = if idx % 4 == 0 { ipv4_mapped([172, 16, 5, 9]).to_vec() } else { rng.bytes(16) };
            let ptag = format!("ERRORS393 pfx A={} ip={}", hexs(a), hexs(&ipp));
            let mut d1 = ob(16);
            let mut e1 = ob(16);
            let mut d2 = ob(16);
            let mut e2 = ob(16);
            unsafe {
                cpf(d1.as_mut_ptr(), ipp.as_ptr(), kdeg.as_ptr());
                rpf(e1.as_mut_ptr(), ipp.as_ptr(), kdeg.as_ptr());
                cpf(d2.as_mut_ptr(), ipp.as_ptr(), kp.as_ptr());
                rpf(e2.as_mut_ptr(), ipp.as_ptr(), kp.as_ptr());
            }
            cmp_out(&format!("{ptag} degenerate"), 16, &d1, &e1);
            cmp_out(&format!("{ptag} equivalent"), 16, &d2, &e2);
            assert_eq_bytes(&format!("{ptag}: C ^0x5a re-derivation missing"), &d1, &d2);
            assert_eq_bytes(&format!("{ptag}: rust ^0x5a re-derivation missing"), &e1, &e2);
            let mut f1 = ob(16);
            let mut g1 = ob(16);
            unsafe {
                cpfd(f1.as_mut_ptr(), d1.as_ptr(), kdeg.as_ptr());
                rpfd(g1.as_mut_ptr(), e1.as_ptr(), kdeg.as_ptr());
            }
            cmp_out(&format!("{ptag} pfx_decrypt"), 16, &f1, &g1);
            assert_eq_bytes(&format!("{ptag}: C round-trip"), &ipp, &f1[..16]);
            assert_eq_bytes(&format!("{ptag}: rust round-trip"), &ipp, &g1[..16]);
            n_pfx += 1;
        }
    }
    assert!(n_ndx >= 64, "ERRORS 393 (ndx) drove only {n_ndx} degenerate keys");
    assert!(n_pfx >= 32, "ERRORS 393 (pfx) drove only {n_pfx} degenerate keys");
    eprintln!("ERRORS 393: {n_ndx} ndx + {n_pfx} pfx degenerate identical-halves keys");
}

/// ERRORS 394 — `crypto_shorthash_siphash24` / `siphashx24` accept ANY `inlen`
/// including 0 and always return 0.
#[test]
fn err394_siphash_any_inlen_including_zero() {
    init_all();
    let (c24, r24) = fnpair!("crypto_shorthash_siphash24", Shash);
    let (cx, rx) = fnpair!("crypto_shorthash_siphashx24", Shash);
    let c24f: Shash = *c24;
    let r24f: Shash = *r24;
    let cxf: Shash = *cx;
    let rxf: Shash = *rx;
    let mut rng = Rng::new(SEED ^ 394);
    let keys = patterns(16, &mut rng);
    let variants: [(&str, Shash, Shash, usize); 2] =
        [("siphash24", c24f, r24f, 8), ("siphashx24", cxf, rxf, 16)];

    let mut n = 0usize;
    let mut zero_cases = 0usize;
    for k in &keys {
        // the special row: inlen == 0
        let dummy = [0x5au8; 1];
        for (name, cf, rf, outlen) in variants {
            let mut oc = ob(outlen);
            let mut or = ob(outlen);
            let (rc, rr) = unsafe {
                (
                    cf(oc.as_mut_ptr(), dummy.as_ptr(), 0, k.as_ptr()),
                    rf(or.as_mut_ptr(), dummy.as_ptr(), 0, k.as_ptr()),
                )
            };
            assert_eq!(rc, 0, "ERRORS394: C {name}(inlen=0) returned {rc}, must be 0");
            assert_eq!(rr, 0, "ERRORS394: rust {name}(inlen=0) returned {rr}, must be 0");
            cmp_out(&format!("ERRORS394 {name} inlen=0 key={}", hexs(k)), outlen, &oc, &or);
            zero_cases += 1;
            n += 1;
        }
        // the whole length sweep must also be accepted
        for &l in LENS {
            let msg = rng.bytes(l);
            for (name, cf, rf, outlen) in variants {
                let mut oc = ob(outlen);
                let mut or = ob(outlen);
                let (rc, rr) = unsafe {
                    (
                        cf(oc.as_mut_ptr(), msg.as_ptr(), l as u64, k.as_ptr()),
                        rf(or.as_mut_ptr(), msg.as_ptr(), l as u64, k.as_ptr()),
                    )
                };
                assert_eq!(rc, 0, "ERRORS394: C {name}(inlen={l}) returned {rc}");
                assert_eq!(rr, 0, "ERRORS394: rust {name}(inlen={l}) returned {rr}");
                cmp_out(&format!("ERRORS394 {name} inlen={l} key={}", hexs(k)), outlen, &oc, &or);
                n += 1;
            }
        }
    }

    // `in` is nonnull(1,4) only for `out`/`k`, so a NULL message with inlen 0 is
    // legal: both libraries must survive it identically.
    let key = [0x42u8; 16];
    for (name, cf, rf, _outlen) in variants {
        let run = |g: Shash| -> Outcome {
            forked(move || {
                let mut o = [0u8; 16];
                unsafe { g(o.as_mut_ptr(), std::ptr::null(), 0, key.as_ptr()) as i64 }
            })
        };
        let (oc, or) = (run(cf), run(rf));
        assert_same_fatal(&format!("ERRORS394 {name}(in=NULL, inlen=0)"), oc, or);
        assert_eq!(oc, Outcome::Returned(0), "ERRORS394: C {name}(NULL,0) -> {oc:?}");
        n += 1;
    }

    assert_eq!(zero_cases, 10, "ERRORS394 must cover inlen=0 for both variants x 5 keys");
    assert!(n >= 64, "ERRORS 394 drove only {n} cases (need >= 64)");
    eprintln!("ERRORS 394: {n} siphash cases ({zero_cases} with inlen=0)");
}

// ##########################################################################
// #  PART 2 — cross-module integration pipelines (CONFIGS 274–278)
// ##########################################################################

/// CONFIGS 274: `crypto_sign_ed25519_keypair` (det-RNG) ->
/// `_sk_to_curve25519` / `_pk_to_curve25519` -> `crypto_box_easy` /
/// `_open_easy` -> `crypto_secretbox_easy` / `_open_easy` with the derived key.
fn pipe274(lib: &'static Library, iter: usize, tr: &mut Trace) {
    let kp: Kp2 = f(lib, "crypto_sign_ed25519_keypair");
    let sk2c: Conv = f(lib, "crypto_sign_ed25519_sk_to_curve25519");
    let pk2c: Conv = f(lib, "crypto_sign_ed25519_pk_to_curve25519");
    let bxe: BoxEasy = f(lib, "crypto_box_easy");
    let bxo: BoxEasy = f(lib, "crypto_box_open_easy");
    let bnm: Fn3i = f(lib, "crypto_box_beforenm");
    let sbe: SbEasy = f(lib, "crypto_secretbox_easy");
    let sbo: SbEasy = f(lib, "crypto_secretbox_open_easy");

    let mlen = 1 + iter % 48;
    let msg: Vec<u8> = (0..mlen).map(|i| ((i * 31 + iter * 7) & 0xff) as u8).collect();
    let nonce: [u8; 24] = std::array::from_fn(|i| (i as u8) ^ (iter as u8));

    // two ed25519 identities straight from the (deterministic) RNG
    let side = |name: &str, tr: &mut Trace| -> (Vec<u8>, Vec<u8>) {
        let mut spk = ob(32);
        let mut ssk = ob(64);
        let rc = unsafe { kp(spk.as_mut_ptr(), ssk.as_mut_ptr()) };
        trc(tr, &format!("{name}.sign_keypair rc"), rc);
        assert_eq!(rc, 0, "274 {name}: crypto_sign_ed25519_keypair -> {rc}");
        guard(&format!("274 {name}.sign_pk"), &spk, 32);
        guard(&format!("274 {name}.sign_sk"), &ssk, 64);
        tp(tr, &format!("{name}.sign_pk"), &spk);
        tp(tr, &format!("{name}.sign_sk"), &ssk);

        let mut csk = ob(32);
        let rc = unsafe { sk2c(csk.as_mut_ptr(), ssk.as_ptr()) };
        trc(tr, &format!("{name}.sk_to_curve25519 rc"), rc);
        assert_eq!(rc, 0, "274 {name}: sk_to_curve25519 -> {rc}");
        guard(&format!("274 {name}.curve_sk"), &csk, 32);
        tp(tr, &format!("{name}.curve_sk"), &csk);

        let mut cpk = ob(32);
        let rc = unsafe { pk2c(cpk.as_mut_ptr(), spk.as_ptr()) };
        trc(tr, &format!("{name}.pk_to_curve25519 rc"), rc);
        assert_eq!(rc, 0, "274 {name}: pk_to_curve25519 -> {rc}");
        guard(&format!("274 {name}.curve_pk"), &cpk, 32);
        tp(tr, &format!("{name}.curve_pk"), &cpk);
        (cpk[..32].to_vec(), csk[..32].to_vec())
    };
    let (a_pk, a_sk) = side("A", tr);
    let (b_pk, b_sk) = side("B", tr);

    // crypto_box_easy A -> B
    let mut ct = ob(mlen + 16);
    let rc = unsafe { bxe(ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), b_pk.as_ptr(), a_sk.as_ptr()) };
    trc(tr, "box_easy rc", rc);
    assert_eq!(rc, 0, "274: crypto_box_easy -> {rc}");
    guard("274 box ct", &ct, mlen + 16);
    tp(tr, "box_ct", &ct);

    let mut pt = ob(mlen);
    let rc = unsafe {
        bxo(pt.as_mut_ptr(), ct.as_ptr(), (mlen + 16) as u64, nonce.as_ptr(), a_pk.as_ptr(), b_sk.as_ptr())
    };
    trc(tr, "box_open_easy rc", rc);
    assert_eq!(rc, 0, "274: crypto_box_open_easy -> {rc}");
    guard("274 box pt", &pt, mlen);
    tp(tr, "box_pt", &pt);
    assert_eq_bytes("274 crypto_box round-trip", &msg, &pt[..mlen]);

    // the derived (beforenm) key feeds secretbox
    let mut dk = ob(32);
    let rc = unsafe { bnm(dk.as_mut_ptr(), b_pk.as_ptr(), a_sk.as_ptr()) };
    trc(tr, "box_beforenm rc", rc);
    assert_eq!(rc, 0, "274: crypto_box_beforenm -> {rc}");
    guard("274 derived key", &dk, 32);
    tp(tr, "derived_key", &dk);

    let mut sct = ob(mlen + 16);
    let rc = unsafe { sbe(sct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), dk.as_ptr()) };
    trc(tr, "secretbox_easy rc", rc);
    assert_eq!(rc, 0, "274: crypto_secretbox_easy -> {rc}");
    guard("274 secretbox ct", &sct, mlen + 16);
    tp(tr, "secretbox_ct", &sct);

    let mut spt = ob(mlen);
    let rc = unsafe { sbo(spt.as_mut_ptr(), sct.as_ptr(), (mlen + 16) as u64, nonce.as_ptr(), dk.as_ptr()) };
    trc(tr, "secretbox_open_easy rc", rc);
    assert_eq!(rc, 0, "274: crypto_secretbox_open_easy -> {rc}");
    guard("274 secretbox pt", &spt, mlen);
    tp(tr, "secretbox_pt", &spt);
    assert_eq_bytes("274 crypto_secretbox round-trip", &msg, &spt[..mlen]);
}

#[test]
fn cfg274_sign_to_box_to_secretbox_pipeline() {
    init_all();
    let _sess = RngSession::new(false);
    reset_det_rng();
    let [lc, lr] = both_libs();
    let mut n = 0usize;
    for it in 0..64 {
        // Both libraries have independent deterministic counters that advance by
        // the same amount per iteration, so they stay in lockstep.
        let mut tc: Trace = Vec::new();
        let mut tred: Trace = Vec::new();
        pipe274(lc, it, &mut tc);
        pipe274(lr, it, &mut tred);
        cmp_trace(&format!("CONFIGS274 iteration {it}"), &tc, &tred);
        n += 1;
    }
    assert_eq!(n, 64, "CONFIGS 274 must run 64 end-to-end pipelines");
    eprintln!("CONFIGS 274: {n} sign -> curve25519 -> box -> secretbox pipelines");
}

/// CONFIGS 275: `crypto_kx_keypair` x2 -> `_client_session_keys` /
/// `_server_session_keys` -> secretstream `init_push` -> 8 pushes mixing
/// MESSAGE/PUSH/REKEY/FINAL -> `init_pull` + 8 pulls. Every header, ciphertext,
/// tag and the FULL state buffer is traced at each step.
fn pipe275(lib: &'static Library, iter: usize, tr: &mut Trace) {
    let kxkp: Kp2 = f(lib, "crypto_kx_keypair");
    let cli: KxSess = f(lib, "crypto_kx_client_session_keys");
    let srv: KxSess = f(lib, "crypto_kx_server_session_keys");
    let statebytes: SizeFn = f(lib, "crypto_secretstream_xchacha20poly1305_statebytes");
    let init_push: SsInit = f(lib, "crypto_secretstream_xchacha20poly1305_init_push");
    let init_pull: SsInitPull = f(lib, "crypto_secretstream_xchacha20poly1305_init_pull");
    let push: SsPush = f(lib, "crypto_secretstream_xchacha20poly1305_push");
    let pull: SsPull = f(lib, "crypto_secretstream_xchacha20poly1305_pull");
    let tag_msg: U8Fn = f(lib, "crypto_secretstream_xchacha20poly1305_tag_message");
    let tag_push: U8Fn = f(lib, "crypto_secretstream_xchacha20poly1305_tag_push");
    let tag_rekey: U8Fn = f(lib, "crypto_secretstream_xchacha20poly1305_tag_rekey");
    let tag_final: U8Fn = f(lib, "crypto_secretstream_xchacha20poly1305_tag_final");

    let sb = unsafe { statebytes() };
    tp(tr, "statebytes", &(sb as u64).to_le_bytes());
    assert_eq!(sb, 52, "275: unexpected secretstream statebytes {sb}");

    let (tm, tpu, trk, tfi) = unsafe { (tag_msg(), tag_push(), tag_rekey(), tag_final()) };
    tp(tr, "tags", &[tm, tpu, trk, tfi]);
    assert_eq!((tm, tpu, trk, tfi), (0x00, 0x01, 0x02, 0x03), "275: tag constants wrong");

    // --- kx: client and server keypairs from the deterministic RNG ---
    let mut cpk = ob(32);
    let mut csk = ob(32);
    let rc = unsafe { kxkp(cpk.as_mut_ptr(), csk.as_mut_ptr()) };
    trc(tr, "client.kx_keypair rc", rc);
    assert_eq!(rc, 0);
    guard("275 client pk", &cpk, 32);
    guard("275 client sk", &csk, 32);
    tp(tr, "client.pk", &cpk);
    tp(tr, "client.sk", &csk);

    let mut spk = ob(32);
    let mut ssk = ob(32);
    let rc = unsafe { kxkp(spk.as_mut_ptr(), ssk.as_mut_ptr()) };
    trc(tr, "server.kx_keypair rc", rc);
    assert_eq!(rc, 0);
    guard("275 server pk", &spk, 32);
    guard("275 server sk", &ssk, 32);
    tp(tr, "server.pk", &spk);
    tp(tr, "server.sk", &ssk);

    let mut crx = ob(32);
    let mut ctx_ = ob(32);
    let rc = unsafe { cli(crx.as_mut_ptr(), ctx_.as_mut_ptr(), cpk.as_ptr(), csk.as_ptr(), spk.as_ptr()) };
    trc(tr, "client_session_keys rc", rc);
    assert_eq!(rc, 0, "275: crypto_kx_client_session_keys -> {rc}");
    guard("275 client rx", &crx, 32);
    guard("275 client tx", &ctx_, 32);
    tp(tr, "client.rx", &crx);
    tp(tr, "client.tx", &ctx_);

    let mut srx = ob(32);
    let mut stx = ob(32);
    let rc = unsafe { srv(srx.as_mut_ptr(), stx.as_mut_ptr(), spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr()) };
    trc(tr, "server_session_keys rc", rc);
    assert_eq!(rc, 0, "275: crypto_kx_server_session_keys -> {rc}");
    guard("275 server rx", &srx, 32);
    guard("275 server tx", &stx, 32);
    tp(tr, "server.rx", &srx);
    tp(tr, "server.tx", &stx);
    assert_eq_bytes("275 client.tx must equal server.rx", &ctx_[..32], &srx[..32]);
    assert_eq_bytes("275 client.rx must equal server.tx", &crx[..32], &stx[..32]);

    // --- secretstream over client.tx (== server.rx) ---
    let key = ctx_[..32].to_vec();
    let mut st = ob(sb);
    let mut header = ob(24);
    let rc = unsafe { init_push(st.as_mut_ptr(), header.as_mut_ptr(), key.as_ptr()) };
    trc(tr, "init_push rc", rc);
    assert_eq!(rc, 0, "275: init_push -> {rc}");
    guard("275 push state", &st, sb);
    guard("275 header", &header, 24);
    tp(tr, "header", &header);
    tp(tr, "state.after_init_push", &st);

    let tags = [tm, tm, tpu, tm, trk, tm, tpu, tfi];
    let mut msgs: Vec<Vec<u8>> = Vec::new();
    let mut ads: Vec<Vec<u8>> = Vec::new();
    let mut cts: Vec<Vec<u8>> = Vec::new();
    for (i, &tag) in tags.iter().enumerate() {
        let mlen = (iter * 3 + i * 5) % 40;
        let m: Vec<u8> = (0..mlen).map(|j| ((j * 17 + i * 3 + iter) & 0xff) as u8).collect();
        let adlen = (iter + i) % 17;
        let ad: Vec<u8> = (0..adlen).map(|j| ((j * 5 + i) & 0xff) as u8).collect();
        let mut ct = ob(mlen + 17);
        let mut clen: u64 = 0xdead_beef;
        let rc = unsafe {
            push(
                st.as_mut_ptr(),
                ct.as_mut_ptr(),
                &mut clen,
                m.as_ptr(),
                mlen as u64,
                ad.as_ptr(),
                adlen as u64,
                tag,
            )
        };
        trc(tr, &format!("push[{i}] rc"), rc);
        assert_eq!(rc, 0, "275: push[{i}] -> {rc}");
        assert_eq!(clen, (mlen + 17) as u64, "275: push[{i}] clen {clen}");
        guard(&format!("275 push[{i}] ct"), &ct, mlen + 17);
        guard(&format!("275 push[{i}] state"), &st, sb);
        tp(tr, &format!("push[{i}].clen"), &clen.to_le_bytes());
        tp(tr, &format!("push[{i}].ct"), &ct);
        tp(tr, &format!("state.after_push[{i}]"), &st);
        msgs.push(m);
        ads.push(ad);
        cts.push(ct[..mlen + 17].to_vec());
    }

    // --- pull side, keyed with server.rx ---
    let mut st2 = ob(sb);
    let rc = unsafe { init_pull(st2.as_mut_ptr(), header.as_ptr(), srx.as_ptr()) };
    trc(tr, "init_pull rc", rc);
    assert_eq!(rc, 0, "275: init_pull -> {rc}");
    guard("275 pull state", &st2, sb);
    tp(tr, "state.after_init_pull", &st2);

    for i in 0..8 {
        let mlen = msgs[i].len();
        let mut m = ob(mlen);
        let mut got: u64 = 0xdead_beef;
        let mut tag: u8 = 0xff;
        let rc = unsafe {
            pull(
                st2.as_mut_ptr(),
                m.as_mut_ptr(),
                &mut got,
                &mut tag,
                cts[i].as_ptr(),
                cts[i].len() as u64,
                ads[i].as_ptr(),
                ads[i].len() as u64,
            )
        };
        trc(tr, &format!("pull[{i}] rc"), rc);
        assert_eq!(rc, 0, "275: pull[{i}] -> {rc}");
        assert_eq!(got, mlen as u64, "275: pull[{i}] mlen {got} != {mlen}");
        assert_eq!(tag, tags[i], "275: pull[{i}] tag 0x{tag:02x} != 0x{:02x}", tags[i]);
        guard(&format!("275 pull[{i}] pt"), &m, mlen);
        guard(&format!("275 pull[{i}] state"), &st2, sb);
        assert_eq_bytes(&format!("275 pull[{i}] plaintext"), &msgs[i], &m[..mlen]);
        tp(tr, &format!("pull[{i}].mlen"), &got.to_le_bytes());
        tp(tr, &format!("pull[{i}].tag"), &[tag]);
        tp(tr, &format!("pull[{i}].pt"), &m);
        tp(tr, &format!("state.after_pull[{i}]"), &st2);
    }
    // After the FINAL tag both states must agree.
    tp(tr, "final.push_state", &st);
    tp(tr, "final.pull_state", &st2);
}

#[test]
fn cfg275_kx_to_secretstream_pipeline() {
    init_all();
    let _sess = RngSession::new(false);
    reset_det_rng();
    let [lc, lr] = both_libs();
    let mut n = 0usize;
    for it in 0..64 {
        let mut tc: Trace = Vec::new();
        let mut trr: Trace = Vec::new();
        pipe275(lc, it, &mut tc);
        pipe275(lr, it, &mut trr);
        cmp_trace(&format!("CONFIGS275 iteration {it}"), &tc, &trr);
        n += 1;
    }
    assert_eq!(n, 64, "CONFIGS 275 must run 64 end-to-end pipelines");
    eprintln!("CONFIGS 275: {n} kx -> secretstream pipelines (8 pushes/pulls each)");
}

/// CONFIGS 276: xwing KEM -> HKDF-SHA256 extract/expand -> xchacha20poly1305-ietf.
/// `use_rng == false` uses `_seed_keypair` / `_enc_deterministic` so the whole
/// pipeline is deterministic without the RNG shim.
fn pipe276(lib: &'static Library, iter: usize, use_rng: bool, tr: &mut Trace) {
    let seed_kp: SeedKp = f(lib, "crypto_kem_xwing_seed_keypair");
    let kp: Kp2 = f(lib, "crypto_kem_xwing_keypair");
    let enc_det: XwEncDet = f(lib, "crypto_kem_xwing_enc_deterministic");
    let enc: XwEnc = f(lib, "crypto_kem_xwing_enc");
    let dec: Fn3i = f(lib, "crypto_kem_xwing_dec");
    let extract: HkExtract = f(lib, "crypto_kdf_hkdf_sha256_extract");
    let expand: HkExpand = f(lib, "crypto_kdf_hkdf_sha256_expand");
    let aenc: AeadEnc = f(lib, "crypto_aead_xchacha20poly1305_ietf_encrypt");
    let adec: AeadDec = f(lib, "crypto_aead_xchacha20poly1305_ietf_decrypt");

    const PK: usize = 1216;
    const SK: usize = 32;
    const CT: usize = 1120;
    const SS: usize = 32;

    let mut pk = ob(PK);
    let mut sk = ob(SK);
    if use_rng {
        let rc = unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        trc(tr, "xwing_keypair rc", rc);
        assert_eq!(rc, 0, "276: crypto_kem_xwing_keypair -> {rc}");
    } else {
        let seed: [u8; 32] = std::array::from_fn(|i| ((i * 7 + iter * 13) & 0xff) as u8);
        let rc = unsafe { seed_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        trc(tr, "xwing_seed_keypair rc", rc);
        assert_eq!(rc, 0, "276: crypto_kem_xwing_seed_keypair -> {rc}");
        tp(tr, "kem.seed", &seed);
    }
    guard("276 kem pk", &pk, PK);
    guard("276 kem sk", &sk, SK);
    tp(tr, "kem.pk", &pk);
    tp(tr, "kem.sk", &sk);

    let mut ct = ob(CT);
    let mut ss = ob(SS);
    if use_rng {
        let rc = unsafe { enc(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr()) };
        trc(tr, "xwing_enc rc", rc);
        assert_eq!(rc, 0, "276: crypto_kem_xwing_enc -> {rc}");
    } else {
        let eseed: [u8; 64] = std::array::from_fn(|i| ((i * 11 + iter * 5 + 1) & 0xff) as u8);
        let rc = unsafe { enc_det(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr()) };
        trc(tr, "xwing_enc_deterministic rc", rc);
        assert_eq!(rc, 0, "276: crypto_kem_xwing_enc_deterministic -> {rc}");
        tp(tr, "kem.enc_seed", &eseed);
    }
    guard("276 kem ct", &ct, CT);
    guard("276 kem ss", &ss, SS);
    tp(tr, "kem.ct", &ct);
    tp(tr, "kem.ss", &ss);

    let mut ss2 = ob(SS);
    let rc = unsafe { dec(ss2.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
    trc(tr, "xwing_dec rc", rc);
    assert_eq!(rc, 0, "276: crypto_kem_xwing_dec -> {rc}");
    guard("276 kem ss2", &ss2, SS);
    tp(tr, "kem.ss_dec", &ss2);
    assert_eq_bytes("276 KEM shared secret mismatch", &ss[..SS], &ss2[..SS]);

    // HKDF-SHA256 extract(salt, ikm=ss) -> prk -> expand -> key || nonce
    let salt: Vec<u8> = (0..(iter % 33)).map(|i| ((i * 3 + iter) & 0xff) as u8).collect();
    let mut prk = ob(32);
    let rc = unsafe { extract(prk.as_mut_ptr(), salt.as_ptr(), salt.len(), ss.as_ptr(), SS) };
    trc(tr, "hkdf_extract rc", rc);
    assert_eq!(rc, 0, "276: hkdf extract -> {rc}");
    guard("276 hkdf prk", &prk, 32);
    tp(tr, "hkdf.salt", &salt);
    tp(tr, "hkdf.prk", &prk);

    let ctxs = format!("t11-276-{}", iter % 7);
    let mut okm = ob(56); // 32-byte key + 24-byte nonce
    let rc = unsafe {
        expand(
            okm.as_mut_ptr(),
            56,
            ctxs.as_ptr() as *const c_char,
            ctxs.len(),
            prk.as_ptr(),
        )
    };
    trc(tr, "hkdf_expand rc", rc);
    assert_eq!(rc, 0, "276: hkdf expand -> {rc}");
    guard("276 hkdf okm", &okm, 56);
    tp(tr, "hkdf.okm", &okm);

    let aead_key = okm[..32].to_vec();
    let npub = okm[32..56].to_vec();
    let mlen = 1 + iter % 64;
    let msg: Vec<u8> = (0..mlen).map(|i| ((i * 23 + iter * 3) & 0xff) as u8).collect();
    let adlen = iter % 19;
    let ad: Vec<u8> = (0..adlen).map(|i| ((i * 9 + 5) & 0xff) as u8).collect();

    let mut ac = ob(mlen + 16);
    let mut aclen: u64 = 0xdead_beef;
    let rc = unsafe {
        aenc(
            ac.as_mut_ptr(),
            &mut aclen,
            msg.as_ptr(),
            mlen as u64,
            ad.as_ptr(),
            adlen as u64,
            std::ptr::null(),
            npub.as_ptr(),
            aead_key.as_ptr(),
        )
    };
    trc(tr, "aead_encrypt rc", rc);
    assert_eq!(rc, 0, "276: aead encrypt -> {rc}");
    assert_eq!(aclen, (mlen + 16) as u64);
    guard("276 aead ct", &ac, mlen + 16);
    tp(tr, "aead.clen", &aclen.to_le_bytes());
    tp(tr, "aead.ct", &ac);

    let mut ap = ob(mlen);
    let mut aplen: u64 = 0xdead_beef;
    let rc = unsafe {
        adec(
            ap.as_mut_ptr(),
            &mut aplen,
            std::ptr::null_mut(),
            ac.as_ptr(),
            (mlen + 16) as u64,
            ad.as_ptr(),
            adlen as u64,
            npub.as_ptr(),
            aead_key.as_ptr(),
        )
    };
    trc(tr, "aead_decrypt rc", rc);
    assert_eq!(rc, 0, "276: aead decrypt -> {rc}");
    assert_eq!(aplen, mlen as u64);
    guard("276 aead pt", &ap, mlen);
    tp(tr, "aead.mlen", &aplen.to_le_bytes());
    tp(tr, "aead.pt", &ap);
    assert_eq_bytes("276 aead round-trip", &msg, &ap[..mlen]);
}

#[test]
fn cfg276_kem_to_kdf_to_aead_pipeline() {
    init_all();
    let [lc, lr] = both_libs();
    let mut n = 0usize;
    // deterministic half — no RNG shim needed
    for it in 0..64 {
        let mut tc: Trace = Vec::new();
        let mut trr: Trace = Vec::new();
        pipe276(lc, it, false, &mut tc);
        pipe276(lr, it, false, &mut trr);
        cmp_trace(&format!("CONFIGS276 deterministic iteration {it}"), &tc, &trr);
        n += 1;
    }
    // RNG half — `crypto_kem_xwing_keypair` / `_enc` under the det RNG
    {
        let _sess = RngSession::new(false);
        reset_det_rng();
        for it in 0..8 {
            let mut tc: Trace = Vec::new();
            let mut trr: Trace = Vec::new();
            pipe276(lc, it, true, &mut tc);
            pipe276(lr, it, true, &mut trr);
            cmp_trace(&format!("CONFIGS276 RNG iteration {it}"), &tc, &trr);
            n += 1;
        }
    }
    assert_eq!(n, 72, "CONFIGS 276 must run 64 deterministic + 8 RNG pipelines");
    eprintln!("CONFIGS 276: {n} kem -> kdf -> aead pipelines");
}

/// CONFIGS 277: `crypto_pwhash` (argon2id13, opslimit=1, memlimit=8192,
/// SALTBYTES=16) -> `crypto_secretbox_easy` / `_open_easy`.
fn pipe277(lib: &'static Library, iter: usize, tr: &mut Trace) {
    const ALG_ARGON2ID13: c_int = 2;
    const SALTBYTES: usize = 16;
    const OPSLIMIT: u64 = 1; // crypto_pwhash_argon2id_OPSLIMIT_MIN
    const MEMLIMIT: usize = 8192; // crypto_pwhash_argon2id_MEMLIMIT_MIN

    let pw: PwHash = f(lib, "crypto_pwhash");
    let saltb: SizeFn = f(lib, "crypto_pwhash_saltbytes");
    let opsmin: SizeFn = f(lib, "crypto_pwhash_opslimit_min");
    let memmin: SizeFn = f(lib, "crypto_pwhash_memlimit_min");
    let algd: IntFn = f(lib, "crypto_pwhash_alg_argon2id13");
    let sbe: SbEasy = f(lib, "crypto_secretbox_easy");
    let sbo: SbEasy = f(lib, "crypto_secretbox_open_easy");

    let (sb, om, mm, alg) = unsafe { (saltb(), opsmin(), memmin(), algd()) };
    tp(tr, "params", &[sb as u8, om as u8, alg as u8]);
    tp(tr, "memlimit_min", &(mm as u64).to_le_bytes());
    assert_eq!(sb, SALTBYTES, "277: crypto_pwhash_saltbytes {sb}");
    assert_eq!(om as u64, OPSLIMIT, "277: opslimit_min {om}");
    assert_eq!(mm, MEMLIMIT, "277: memlimit_min {mm}");
    assert_eq!(alg, ALG_ARGON2ID13, "277: alg_argon2id13 {alg}");

    let plen = iter % 32;
    let passwd: Vec<u8> = (0..plen).map(|i| 0x30 + ((i + iter) % 60) as u8).collect();
    let salt: Vec<u8> = (0..SALTBYTES).map(|i| ((i * 19 + iter * 7) & 0xff) as u8).collect();

    let mut key = ob(32);
    let rc = unsafe {
        pw(
            key.as_mut_ptr(),
            32,
            passwd.as_ptr() as *const c_char,
            plen as u64,
            salt.as_ptr(),
            OPSLIMIT,
            MEMLIMIT,
            ALG_ARGON2ID13,
        )
    };
    trc(tr, "crypto_pwhash rc", rc);
    assert_eq!(rc, 0, "277: crypto_pwhash(opslimit=1, memlimit=8192) -> {rc}");
    guard("277 pwhash key", &key, 32);
    tp(tr, "pwhash.passwd", &passwd);
    tp(tr, "pwhash.salt", &salt);
    tp(tr, "pwhash.key", &key);

    let mlen = 1 + iter % 40;
    let msg: Vec<u8> = (0..mlen).map(|i| ((i * 37 + iter) & 0xff) as u8).collect();
    let nonce: [u8; 24] = std::array::from_fn(|i| ((i * 5 + iter * 3) & 0xff) as u8);

    let mut ct = ob(mlen + 16);
    let rc = unsafe { sbe(ct.as_mut_ptr(), msg.as_ptr(), mlen as u64, nonce.as_ptr(), key.as_ptr()) };
    trc(tr, "secretbox_easy rc", rc);
    assert_eq!(rc, 0, "277: crypto_secretbox_easy -> {rc}");
    guard("277 secretbox ct", &ct, mlen + 16);
    tp(tr, "secretbox.ct", &ct);

    let mut pt = ob(mlen);
    let rc = unsafe { sbo(pt.as_mut_ptr(), ct.as_ptr(), (mlen + 16) as u64, nonce.as_ptr(), key.as_ptr()) };
    trc(tr, "secretbox_open_easy rc", rc);
    assert_eq!(rc, 0, "277: crypto_secretbox_open_easy -> {rc}");
    guard("277 secretbox pt", &pt, mlen);
    tp(tr, "secretbox.pt", &pt);
    assert_eq_bytes("277 secretbox round-trip", &msg, &pt[..mlen]);
}

#[test]
fn cfg277_pwhash_to_secretbox_pipeline() {
    init_all();
    // `argon2_hash()` starts with `randombytes_buf(hash, hashlen)`, so every
    // `crypto_pwhash` call CONSUMES from the process-global randombytes stream
    // even though its result is deterministic. Hold the same lock the
    // deterministic-RNG rows use so this row cannot steal their bytes.
    let _g = rng_lock();
    let [lc, lr] = both_libs();
    let mut n = 0usize;
    for it in 0..64 {
        let mut tc: Trace = Vec::new();
        let mut trr: Trace = Vec::new();
        pipe277(lc, it, &mut tc);
        pipe277(lr, it, &mut trr);
        cmp_trace(&format!("CONFIGS277 iteration {it}"), &tc, &trr);
        n += 1;
    }
    assert_eq!(n, 64, "CONFIGS 277 must run 64 pwhash -> secretbox pipelines");
    eprintln!("CONFIGS 277: {n} pwhash(argon2id, ops=1, mem=8192) -> secretbox pipelines");
}

/// CONFIGS 278 — a representative subset of everything above, run BOTH before
/// and after `sodium_init()`.
///
/// The pre-init half is taken in a `fork()`ed child (see `capture_preinit`)
/// which writes into an `mmap(MAP_SHARED)` page; the parent then compares C's
/// pre-init bytes against Rust's pre-init bytes, and both against the post-init
/// results. The `*_pick_best_implementation` static initialisers are already
/// valid pre-init, so all four blobs must be identical.
#[test]
fn cfg278_pre_and_post_sodium_init() {
    init_all();
    let pre = preinit();
    let [lc, lr] = both_libs();
    let post_c = subset_now(lc);
    let post_r = subset_now(lr);

    // the trailing byte of each blob is the "no output-buffer overrun" flag
    for (what, blob) in [
        ("pre-init C", &pre.c),
        ("pre-init rust", &pre.r),
        ("post-init C", &post_c),
        ("post-init rust", &post_r),
    ] {
        assert!(!blob.is_empty(), "CONFIGS278 {what}: empty result blob");
        assert_eq!(
            *blob.last().unwrap(),
            1u8,
            "CONFIGS278 {what}: an output buffer's 0xAA trailing guard was clobbered"
        );
        assert!(blob.len() > 3000, "CONFIGS278 {what}: blob only {} bytes", blob.len());
    }

    assert_eq_bytes("CONFIGS278 PRE-sodium_init(): C vs rust", &pre.c, &pre.r);
    assert_eq_bytes("CONFIGS278 POST-sodium_init(): C vs rust", &post_c, &post_r);
    assert_eq_bytes("CONFIGS278 C: pre-init vs post-init", &pre.c, &post_c);
    assert_eq_bytes("CONFIGS278 rust: pre-init vs post-init", &pre.r, &post_r);
    eprintln!(
        "CONFIGS 278: {} bytes of results identical across {{C, rust}} x {{pre-init, post-init}}",
        pre.c.len()
    );
}
