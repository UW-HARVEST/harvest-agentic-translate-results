//! t07_aead.rs — C-vs-Rust differential verification of the AEAD, secretbox and
//! secretstream surfaces.
//!
//! Specification: `CONFIGS.md` rows 164–201 and `ERRORS.md` rows 84–130.
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly.
//!
//! Every output buffer is prefilled with 0xAA and carries a trailing 0xAA guard
//! region; the FULL buffer (payload + guard) is compared between the two
//! libraries and the guard is asserted intact, so an over-write is caught even
//! when both libraries agree on the payload.
//!
//! Row → test mapping is at the bottom of this comment block.
//!
//! CONFIGS
//! -------
//! * 164 chacha20poly1305 encrypt/decrypt ........... `r164_cp_encrypt_decrypt`
//! * 165 chacha20poly1305 detached .................. `r165_cp_detached`
//! * 166 chacha20poly1305 chunk boundary ............ `r166_cp_chunk_boundary`
//! * 167 chacha20poly1305_ietf encrypt/decrypt ...... `r167_cpi_encrypt_decrypt`
//! * 168 chacha20poly1305_ietf detached ............. `r168_cpi_detached`
//! * 169 chacha20poly1305_ietf chunk boundary ....... `r169_cpi_chunk_boundary`
//! * 170 xchacha20poly1305_ietf encrypt/decrypt ..... `r170_xcpi_encrypt_decrypt`
//! * 171 xchacha20poly1305_ietf detached ............ `r171_xcpi_detached`
//! * 172 ad==NULL / nsec!=NULL / in-place / NULL outs `r172_null_nsec_inplace_outparams`
//! * 173 `*_decrypt_detached` with `m == NULL` ....... `r173_decrypt_detached_verify_only`
//! * 174 chacha keygen + constants ................... `r174_chacha_keygen_and_constants`
//! * 175 aegis128l encrypt/decrypt ................... `r175_aegis128l_encrypt_decrypt`
//! * 176 aegis128l detached + m==NULL ................ `r176_aegis128l_detached`
//! * 177 aegis256 encrypt/decrypt .................... `r177_aegis256_encrypt_decrypt`
//! * 178 aegis256 detached + m==NULL ................. `r178_aegis256_detached`
//! * 179 `_pick_best_implementation` .................. `r179_aegis_pick_best_implementation`
//! * 180 aegis keygen + constants ..................... `r180_aegis_keygen_and_constants`
//! * 181 aes256gcm stubs + statebytes + constants ..... `r181_aes256gcm_stubs_and_constants`
//! * 182 secretbox_easy / _open_easy .................. `r182_secretbox_easy`
//! * 183 secretbox_detached disjoint .................. `r183_secretbox_detached_disjoint`
//! * 184 secretbox_detached overlap axis (4 shapes) ... `r184_secretbox_detached_overlap`
//! * 185 secretbox_detached chunk boundary ............ `r185_secretbox_detached_chunk_boundary`
//! * 186 `crypto_secretbox` / `_open` (NaCl padded) ... `r186_secretbox_nacl_padded`
//! * 187 `crypto_secretbox_xsalsa20poly1305[_open]` ... `r187_secretbox_xsalsa20poly1305`
//! * 188 `crypto_secretbox_open_detached(m=NULL)` ..... `r188_secretbox_open_detached_verify_only`
//! * 189 xchacha20poly1305 easy/open_easy ............. `r189_secretbox_xchacha_easy`
//! * 190 xchacha20poly1305 detached overlap + m==NULL   `r190_secretbox_xchacha_detached_overlap`
//! * 191 secretbox keygen + constants + primitive ..... `r191_secretbox_keygen_and_constants`
//! * 192 secretstream TAG_MESSAGE ..................... `r192_ss_tag_message`
//! * 193 secretstream TAG_PUSH ........................ `r193_ss_tag_push`
//! * 194 secretstream TAG_REKEY ....................... `r194_ss_tag_rekey`
//! * 195 secretstream TAG_FINAL ....................... `r195_ss_tag_final`
//! * 196 secretstream arbitrary tags .................. `r196_ss_arbitrary_tags`
//! * 197 secretstream explicit `_rekey` ............... `r197_ss_explicit_rekey`
//! * 198 secretstream multi-message sequence .......... `r198_ss_multi_message_sequence`
//! * 199 secretstream poly1305 pad quirk ............. `r199_ss_pad_quirk`
//! * 200 `_pull` NULL out-params ...................... `r200_ss_pull_null_outparams`
//! * 201 secretstream keygen + constants .............. `r201_ss_keygen_and_constants`
//!
//! ERRORS
//! ------
//! * 84  ........ `e84_cp_encrypt_mlen_misuse`
//! * 85  ........ `e85_cp_decrypt_clen_too_short`
//! * 86/87 ...... `e86_e87_cp_decrypt_detached_forged`
//! * 88  ........ `e88_cpi_encrypt_mlen_misuse`
//! * 89  ........ `e89_cpi_decrypt_clen_too_short`
//! * 90/91 ...... `e90_e91_cpi_decrypt_detached_forged`
//! * 92  ........ `e92_xcpi_encrypt_mlen_misuse`
//! * 93  ........ `e93_xcpi_decrypt_clen_too_short`
//! * 94/95 ...... `e94_e95_xcpi_decrypt_detached_forged`
//! * 96  ........ `e96_aegis128l_encrypt_mlen_misuse`
//! * 97  ........ `e97_aegis128l_encrypt_detached_misuse_after_maclen`
//! * 98  ........ `e98_aegis128l_decrypt_clen_too_short`
//! * 99  ........ `e99_aegis128l_decrypt_detached_len_reject`
//! * 100 ........ `e100_aegis128l_decrypt_detached_forged`
//! * 101 ........ `e101_aegis256_encrypt_mlen_misuse`
//! * 102 ........ `e102_aegis256_encrypt_detached_misuse_after_maclen`
//! * 103 ........ `e103_aegis256_decrypt_clen_too_short`
//! * 104 ........ `e104_aegis256_decrypt_detached_len_reject`
//! * 105 ........ `e105_aegis256_decrypt_detached_forged`
//! * 106–115 .... `e106_e115_aes256gcm_enosys`
//! * 116 ........ `e116_secretbox_easy_mlen_misuse`
//! * 117 ........ `e117_secretbox_open_easy_clen_too_short`
//! * 118 ........ `e118_secretbox_open_detached_forged`
//! * 119 ........ `e119_xsalsa20poly1305_mlen_below_zerobytes`
//! * 120 ........ `e120_xsalsa20poly1305_open_clen_below_zerobytes`
//! * 121 ........ `e121_xsalsa20poly1305_open_forged`
//! * 122 ........ `e122_secretbox_xchacha_easy_mlen_misuse`
//! * 123 ........ `e123_secretbox_xchacha_open_easy_clen_too_short`
//! * 124 ........ `e124_secretbox_xchacha_open_detached_forged`
//! * 125 ........ `e125_ss_push_mlen_misuse`
//! * 126 ........ `e126_ss_pull_inlen_too_short`
//! * 127 ........ `e127_ss_pull_mlen_misuse`
//! * 128 ........ `e128_ss_pull_forged_state_not_advanced`
//! * 129 ........ `e129_ss_init_pull_any_header`
//! * 130 ........ `e130_ss_push_any_tag`

mod common;
use common::*;
use libc::{c_char, c_int};
use libloading::Library;
use std::ffi::CStr;
use std::ptr;
use std::sync::OnceLock;

// =============================================================== fn types

type SizeFn = unsafe extern "C" fn() -> usize;
type IntFn = unsafe extern "C" fn() -> c_int;
type U8Fn = unsafe extern "C" fn() -> u8;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type CharFn = unsafe extern "C" fn() -> *const c_char;
type BufFn = unsafe extern "C" fn(*mut libc::c_void, usize);

type AeadEnc = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type AeadEncDet = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
    *const u8,
) -> c_int;
type AeadDec = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
type AeadDecDet = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
    u64,
    *const u8,
    *const u8,
) -> c_int;
type BeforenmFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

type SbEasy = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbDet =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8) -> c_int;
type SbOpenDet =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8) -> c_int;

type SsInitPush = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SsInitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SsPush = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *const u8,
    u64,
    *const u8,
    u64,
    u8,
) -> c_int;
type SsPull = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut u64,
    *mut u8,
    *const u8,
    u64,
    *const u8,
    u64,
) -> c_int;
type SsRekey = unsafe extern "C" fn(*mut u8);

type StreamFn = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int;
type XorIc32Fn =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int;
type P1Init = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type P1Upd = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type P1Fin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

// =============================================================== constants

/// Prefill byte for every output buffer and every out-param sentinel.
const FILL: u8 = 0xAA;
/// Trailing guard region; must never be touched by either library.
const PAD: usize = 32;
/// 0xAA-filled `unsigned long long` sentinel for `*clen_p` / `*mlen_p` / ...
const U64SENT: u64 = 0xAAAA_AAAA_AAAA_AAAA;
/// Linux `ENOSYS`.
const ENOSYS: c_int = 38;
/// `STREAM_POLY1305_CHUNK` from the C sources.
const CHUNK: usize = 131072;

/// Serialises every test that mutates the process-global `randombytes`
/// implementation pointer (cargo runs the tests of one target as parallel
/// threads inside ONE process, sharing both `.so`s).
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// =============================================================== small helpers

fn guard_intact(what: &str, who: &str, b: &[u8], len: usize) {
    assert!(
        b[len..].iter().all(|&x| x == FILL),
        "{what}: {who} wrote OUTSIDE the requested {len} bytes \
         (0xAA trailing guard clobbered: {})",
        hexs(&b[len..])
    );
}

fn lib_of(which: usize) -> &'static Library {
    if which == 0 { &libs().c } else { &libs().r }
}

fn ad_ptr(ad: Option<&[u8]>) -> (*const u8, u64) {
    match ad {
        None => (ptr::null(), 0),
        Some(s) => (s.as_ptr(), s.len() as u64),
    }
}

/// `rng.bytes(rng.below(n))` in one call (two `&mut rng` borrows otherwise).
fn rnd(rng: &mut Rng, upper: usize) -> Vec<u8> {
    let n = rng.below(upper);
    rng.bytes(n)
}

/// `rng.bytes(lo + rng.below(n))`.
fn rnd_min(rng: &mut Rng, lo: usize, upper: usize) -> Vec<u8> {
    let n = lo + rng.below(upper);
    rng.bytes(n)
}

/// 16-byte-aligned scratch region (the C `crypto_onetimeauth_poly1305_state`
/// and `crypto_aead_aes256gcm_state` are both `CRYPTO_ALIGN(16)`).
struct Aligned16 {
    v: Vec<u128>,
    n: usize,
}
impl Aligned16 {
    fn new(n: usize, fill: u8) -> Self {
        let mut v = vec![0u128; n.div_ceil(16).max(1)];
        unsafe { ptr::write_bytes(v.as_mut_ptr() as *mut u8, fill, v.len() * 16) };
        Aligned16 { v, n }
    }
    fn p(&mut self) -> *mut u8 {
        self.v.as_mut_ptr() as *mut u8
    }
    fn all(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.v.as_ptr() as *const u8, self.v.len() * 16) }
    }
    fn len(&self) -> usize {
        self.n
    }
}

/// Restore both libraries to their own `randombytes_sysrandom_implementation`
/// after a test that installed the deterministic RNG.
fn restore_default_rng() {
    type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> c_int;
    let l = libs();
    let (c, r) = unsafe { pair::<SetImplFn>("randombytes_set_implementation") };
    // For a DATA symbol, `Symbol<*const T>` derefs to the symbol's own address.
    let pc = *unsafe { sym::<*const RandombytesImpl>(&l.c, "randombytes_sysrandom_implementation") };
    let pr = *unsafe { sym::<*const RandombytesImpl>(&l.r, "randombytes_sysrandom_implementation") };
    unsafe {
        c(pc);
        r(pr);
    }
}

/// Advance both deterministic counters by `n` bytes so a `*_keygen` row can be
/// driven with many DIFFERENT (but still lockstep) RNG states.
fn advance_det_rng(n: usize) {
    let (cb, rb) = unsafe { pair::<BufFn>("randombytes_buf") };
    let mut sink = vec![0u8; n.max(1)];
    unsafe {
        cb(sink.as_mut_ptr() as *mut libc::c_void, n);
        rb(sink.as_mut_ptr() as *mut libc::c_void, n);
    }
}

fn errno_get() -> c_int {
    unsafe { *libc::__errno_location() }
}
fn errno_set(v: c_int) {
    unsafe { *libc::__errno_location() = v }
}

// =========================================== forked fatal-path plumbing
//
// Every `sodium_misuse()` row is exercised in a forked child so the abort()
// can be observed. A SIGSEGV/SIGBUS handler marks "the guard did NOT fire and
// the call ran on into unmapped memory", which distinguishes a real
// `sodium_misuse()` (SIGABRT) from a guard that never triggered.

const FAULT: i64 = 42;

extern "C" fn fault_handler(_sig: c_int) {
    unsafe { libc::_exit(FAULT as c_int) }
}

/// Async-signal-safe and allocation-free. Also disables core dumps: without
/// this every intentional abort()/fault child would dump its address space.
unsafe fn arm_fault_marker() {
    let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    libc::setrlimit(libc::RLIMIT_CORE, &rl);
    let mut sa: libc::sigaction = std::mem::zeroed();
    sa.sa_sigaction = fault_handler as extern "C" fn(c_int) as libc::sighandler_t;
    libc::sigemptyset(&mut sa.sa_mask);
    sa.sa_flags = 0;
    libc::sigaction(libc::SIGSEGV, &sa, ptr::null_mut());
    libc::sigaction(libc::SIGBUS, &sa, ptr::null_mut());
}

/// Resolve `name` in both libraries in the PARENT (so the child needs neither
/// dlsym nor malloc), then run `body` once per library in a forked child.
fn both_forked<T: Copy + 'static, B: Fn(T) -> i64 + Copy>(
    name: &str,
    body: B,
) -> (Outcome, Outcome) {
    let l = libs();
    let fc: T = *unsafe { sym::<T>(&l.c, name) };
    let fr: T = *unsafe { sym::<T>(&l.r, name) };
    let oc = forked(move || {
        unsafe { arm_fault_marker() };
        body(fc)
    });
    let or = forked(move || {
        unsafe { arm_fault_marker() };
        body(fr)
    });
    (oc, or)
}

fn expect_outcome<T: Copy + 'static, B: Fn(T) -> i64 + Copy>(
    what: &str,
    name: &str,
    body: B,
    want: Outcome,
) {
    let (oc, or) = both_forked::<T, B>(name, body);
    assert_same_fatal(what, oc, or);
    assert_eq!(oc, want, "{what}: C outcome was {oc:?}, expected {want:?}");
}

const MISUSE: Outcome = Outcome::Signaled(SIGABRT);
const NO_MISUSE: Outcome = Outcome::Returned(FAULT);

/// A page shared with forked children, so an out-param written just BEFORE an
/// `abort()` (ERRORS 97/102/125/127) is still observable in the parent.
struct SharedPage {
    p: *mut u8,
    n: usize,
}
impl SharedPage {
    fn new(n: usize) -> Self {
        unsafe {
            let p = libc::mmap(
                ptr::null_mut(),
                n,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap MAP_SHARED failed");
            let sp = SharedPage { p: p as *mut u8, n };
            sp.reset();
            sp
        }
    }
    fn reset(&self) {
        unsafe { ptr::write_bytes(self.p, FILL, self.n) };
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p, self.n) }
    }
    fn u64_at(&self, off: usize) -> u64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.bytes()[off..off + 8]);
        u64::from_ne_bytes(b)
    }
}

/// A plain scratch buffer allocated in the PARENT; children only touch it.
struct Scratch {
    _v: Vec<u8>,
    p: *mut u8,
}
fn scratch(n: usize) -> Scratch {
    let mut v = vec![0u8; n];
    let p = v.as_mut_ptr();
    Scratch { _v: v, p }
}

// ============================================================ AEAD families

struct Aead {
    /// symbol prefix
    p: &'static str,
    kb: usize,
    nb: usize,
    ab: usize,
}

const CP: Aead = Aead { p: "crypto_aead_chacha20poly1305", kb: 32, nb: 8, ab: 16 };
const CPI: Aead = Aead { p: "crypto_aead_chacha20poly1305_ietf", kb: 32, nb: 12, ab: 16 };
const XCPI: Aead = Aead { p: "crypto_aead_xchacha20poly1305_ietf", kb: 32, nb: 24, ab: 16 };
const A128L: Aead = Aead { p: "crypto_aead_aegis128l", kb: 16, nb: 16, ab: 32 };
const A256: Aead = Aead { p: "crypto_aead_aegis256", kb: 32, nb: 32, ab: 32 };

impl Aead {
    fn n(&self, suffix: &str) -> String {
        format!("{}{}", self.p, suffix)
    }
}

/// All five real AEADs of CONFIGS 164–180.
const AEADS: &[&Aead] = &[&CP, &CPI, &XCPI, &A128L, &A256];

/// `*_encrypt`. Returns the (proven identical) `mlen + ABYTES` ciphertext.
#[allow(clippy::too_many_arguments)]
fn enc(
    a: &Aead,
    m: &[u8],
    ad: Option<&[u8]>,
    npub: &[u8],
    k: &[u8],
    nsec: bool,
    clen_p: bool,
    tag: &str,
) -> Vec<u8> {
    let name = a.n("_encrypt");
    let (fc, fr) = unsafe { pair::<AeadEnc>(&name) };
    let mlen = m.len();
    let out = mlen + a.ab;
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let (adp, adl) = ad_ptr(ad);
    let mut nsc = [0x5Au8; 16];
    let mut nsr = [0x5Au8; 16];
    let (nspc, nspr) = if nsec {
        (nsc.as_mut_ptr() as *const u8, nsr.as_mut_ptr() as *const u8)
    } else {
        (ptr::null(), ptr::null())
    };
    let mut cc = U64SENT;
    let mut cr = U64SENT;
    let (ccp, crp) = if clen_p {
        (&mut cc as *mut u64, &mut cr as *mut u64)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(bc.as_mut_ptr(), ccp, m.as_ptr(), mlen as u64, adp, adl, nspc, npub.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(br.as_mut_ptr(), crp, m.as_ptr(), mlen as u64, adp, adl, nspr, npub.as_ptr(), k.as_ptr())
    };
    let what = format!(
        "{name} [{tag}] mlen={mlen} adlen={adl} ad_null={} nsec={nsec} clen_p={clen_p} k={} npub={}",
        ad.is_none(),
        hexs(k),
        hexs(npub)
    );
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}, expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, out);
    guard_intact(&what, "rust", &br, out);
    assert_eq!(cc, cr, "{what}: *clen_p differs (C={cc} rust={cr})");
    if clen_p {
        assert_eq!(cc, out as u64, "{what}: *clen_p should be mlen+ABYTES");
    } else {
        assert_eq!(cc, U64SENT, "{what}: clen_p was NULL yet the sentinel moved");
    }
    if nsec {
        assert_eq!(nsc, [0x5Au8; 16], "{what}: C wrote through nsec (NSECBYTES==0)");
        assert_eq!(nsr, [0x5Au8; 16], "{what}: rust wrote through nsec");
    }
    bc.truncate(out);
    bc
}

/// `*_encrypt` with `c == m` (in place). The buffer initially holds `m`.
fn enc_ip(a: &Aead, m: &[u8], ad: Option<&[u8]>, npub: &[u8], k: &[u8], tag: &str) -> Vec<u8> {
    let name = a.n("_encrypt");
    let (fc, fr) = unsafe { pair::<AeadEnc>(&name) };
    let mlen = m.len();
    let out = mlen + a.ab;
    let mut bc = vec![FILL; out + PAD];
    bc[..mlen].copy_from_slice(m);
    let mut br = bc.clone();
    let (adp, adl) = ad_ptr(ad);
    let pc = bc.as_mut_ptr();
    let pr = br.as_mut_ptr();
    let mut cc = U64SENT;
    let mut cr = U64SENT;
    let rc = unsafe {
        fc(pc, &mut cc, pc, mlen as u64, adp, adl, ptr::null(), npub.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(pr, &mut cr, pr, mlen as u64, adp, adl, ptr::null(), npub.as_ptr(), k.as_ptr())
    };
    let what = format!("{name} [{tag}/in-place c==m] mlen={mlen} adlen={adl} k={}", hexs(k));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, out);
    guard_intact(&what, "rust", &br, out);
    assert_eq!(cc, cr, "{what}: *clen_p differs");
    bc.truncate(out);
    bc
}

/// `*_encrypt_detached`. Returns `(c, mac)`.
#[allow(clippy::too_many_arguments)]
fn enc_det(
    a: &Aead,
    m: &[u8],
    ad: Option<&[u8]>,
    npub: &[u8],
    k: &[u8],
    nsec: bool,
    maclen_p: bool,
    tag: &str,
) -> (Vec<u8>, Vec<u8>) {
    let name = a.n("_encrypt_detached");
    let (fc, fr) = unsafe { pair::<AeadEncDet>(&name) };
    let mlen = m.len();
    let mut bc = vec![FILL; mlen + PAD];
    let mut br = vec![FILL; mlen + PAD];
    let mut mc = vec![FILL; a.ab + PAD];
    let mut mr = vec![FILL; a.ab + PAD];
    let (adp, adl) = ad_ptr(ad);
    let mut nsc = [0x5Au8; 16];
    let mut nsr = [0x5Au8; 16];
    let (nspc, nspr) = if nsec {
        (nsc.as_mut_ptr() as *const u8, nsr.as_mut_ptr() as *const u8)
    } else {
        (ptr::null(), ptr::null())
    };
    let mut lc = U64SENT;
    let mut lr = U64SENT;
    let (lcp, lrp) = if maclen_p {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(
            bc.as_mut_ptr(), mc.as_mut_ptr(), lcp, m.as_ptr(), mlen as u64, adp, adl, nspc,
            npub.as_ptr(), k.as_ptr(),
        )
    };
    let rr = unsafe {
        fr(
            br.as_mut_ptr(), mr.as_mut_ptr(), lrp, m.as_ptr(), mlen as u64, adp, adl, nspr,
            npub.as_ptr(), k.as_ptr(),
        )
    };
    let what = format!(
        "{name} [{tag}] mlen={mlen} adlen={adl} ad_null={} nsec={nsec} maclen_p={maclen_p} k={} npub={}",
        ad.is_none(), hexs(k), hexs(npub)
    );
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: C returned {rc}");
    assert_eq_bytes(&format!("{what} [c]"), &bc, &br);
    assert_eq_bytes(&format!("{what} [mac]"), &mc, &mr);
    guard_intact(&what, "C c", &bc, mlen);
    guard_intact(&what, "rust c", &br, mlen);
    guard_intact(&what, "C mac", &mc, a.ab);
    guard_intact(&what, "rust mac", &mr, a.ab);
    assert_eq!(lc, lr, "{what}: *maclen_p differs (C={lc} rust={lr})");
    if maclen_p {
        assert_eq!(lc, a.ab as u64, "{what}: *maclen_p should be ABYTES");
    } else {
        assert_eq!(lc, U64SENT, "{what}: maclen_p was NULL yet the sentinel moved");
    }
    if nsec {
        assert_eq!(nsc, [0x5Au8; 16], "{what}: C wrote through nsec");
        assert_eq!(nsr, [0x5Au8; 16], "{what}: rust wrote through nsec");
    }
    bc.truncate(mlen);
    mc.truncate(a.ab);
    (bc, mc)
}

/// `*_decrypt`. Returns `(rc, plaintext_buffer_without_guard)`.
#[allow(clippy::too_many_arguments)]
fn dec(
    a: &Aead,
    c: &[u8],
    ad: Option<&[u8]>,
    npub: &[u8],
    k: &[u8],
    nsec: bool,
    mlen_p: bool,
    tag: &str,
) -> (c_int, Vec<u8>) {
    let name = a.n("_decrypt");
    let (fc, fr) = unsafe { pair::<AeadDec>(&name) };
    let clen = c.len();
    let out = clen.saturating_sub(a.ab);
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let (adp, adl) = ad_ptr(ad);
    let mut nsc = [0x5Au8; 16];
    let mut nsr = [0x5Au8; 16];
    let (nspc, nspr) = if nsec {
        (nsc.as_mut_ptr(), nsr.as_mut_ptr())
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let mut lc = U64SENT;
    let mut lr = U64SENT;
    let (lcp, lrp) = if mlen_p {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(bc.as_mut_ptr(), lcp, nspc, c.as_ptr(), clen as u64, adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(br.as_mut_ptr(), lrp, nspr, c.as_ptr(), clen as u64, adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let what = format!(
        "{name} [{tag}] clen={clen} adlen={adl} ad_null={} nsec={nsec} mlen_p={mlen_p} k={} npub={}",
        ad.is_none(), hexs(k), hexs(npub)
    );
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, out);
    guard_intact(&what, "rust", &br, out);
    assert_eq!(lc, lr, "{what}: *mlen_p differs (C={lc} rust={lr})");
    if mlen_p {
        let want = if rc == 0 { out as u64 } else { 0 };
        assert_eq!(lc, want, "{what}: *mlen_p should be {want}");
    } else {
        assert_eq!(lc, U64SENT, "{what}: mlen_p was NULL yet the sentinel moved");
    }
    if nsec {
        assert_eq!(nsc, [0x5Au8; 16], "{what}: C wrote through nsec");
        assert_eq!(nsr, [0x5Au8; 16], "{what}: rust wrote through nsec");
    }
    bc.truncate(out);
    (rc, bc)
}

/// `*_decrypt` with `m == c` (in place).
fn dec_ip(a: &Aead, c: &[u8], ad: Option<&[u8]>, npub: &[u8], k: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let name = a.n("_decrypt");
    let (fc, fr) = unsafe { pair::<AeadDec>(&name) };
    let clen = c.len();
    let mut bc = vec![FILL; clen + PAD];
    bc[..clen].copy_from_slice(c);
    let mut br = bc.clone();
    let (adp, adl) = ad_ptr(ad);
    let pc = bc.as_mut_ptr();
    let pr = br.as_mut_ptr();
    let mut lc = U64SENT;
    let mut lr = U64SENT;
    let rc = unsafe {
        fc(pc, &mut lc, ptr::null_mut(), pc, clen as u64, adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(pr, &mut lr, ptr::null_mut(), pr, clen as u64, adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let what = format!("{name} [{tag}/in-place m==c] clen={clen} adlen={adl} k={}", hexs(k));
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, clen);
    guard_intact(&what, "rust", &br, clen);
    assert_eq!(lc, lr, "{what}: *mlen_p differs");
    bc.truncate(clen);
    (rc, bc)
}

/// `*_decrypt_detached`. `m_null` selects the verify-only code path.
/// The returned buffer is the FULL prefilled plaintext buffer + guard, so the
/// caller can assert `memset(m, 0, clen)` on a MAC failure (or its absence).
#[allow(clippy::too_many_arguments)]
fn dec_det(
    a: &Aead,
    c: &[u8],
    mac: &[u8],
    ad: Option<&[u8]>,
    npub: &[u8],
    k: &[u8],
    nsec: bool,
    m_null: bool,
    tag: &str,
) -> (c_int, Vec<u8>) {
    let name = a.n("_decrypt_detached");
    let (fc, fr) = unsafe { pair::<AeadDecDet>(&name) };
    let clen = c.len();
    let mut bc = vec![FILL; clen + PAD];
    let mut br = vec![FILL; clen + PAD];
    let (pc, pr) = if m_null {
        (ptr::null_mut(), ptr::null_mut())
    } else {
        (bc.as_mut_ptr(), br.as_mut_ptr())
    };
    let (adp, adl) = ad_ptr(ad);
    let mut nsc = [0x5Au8; 16];
    let mut nsr = [0x5Au8; 16];
    let (nspc, nspr) = if nsec {
        (nsc.as_mut_ptr(), nsr.as_mut_ptr())
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(pc, nspc, c.as_ptr(), clen as u64, mac.as_ptr(), adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(pr, nspr, c.as_ptr(), clen as u64, mac.as_ptr(), adp, adl, npub.as_ptr(), k.as_ptr())
    };
    let what = format!(
        "{name} [{tag}] clen={clen} adlen={adl} ad_null={} nsec={nsec} m_null={m_null} k={} npub={} mac={}",
        ad.is_none(), hexs(k), hexs(npub), hexs(mac)
    );
    assert_eq!(rc, rr, "{what}: return value differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    if !m_null {
        guard_intact(&what, "C", &bc, clen);
        guard_intact(&what, "rust", &br, clen);
    } else {
        assert!(
            bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
            "{what}: verify-only mode must not touch any plaintext buffer"
        );
    }
    if nsec {
        assert_eq!(nsc, [0x5Au8; 16], "{what}: C wrote through nsec");
        assert_eq!(nsr, [0x5Au8; 16], "{what}: rust wrote through nsec");
    }
    (rc, bc)
}

/// One full valid round trip through all four entry points of one AEAD.
/// Returns the number of distinct inputs driven (always 1) so callers can count.
fn round_trip(a: &Aead, m: &[u8], ad: Option<&[u8]>, npub: &[u8], k: &[u8], tag: &str) {
    // one-shot
    let c = enc(a, m, ad, npub, k, false, true, tag);
    let (rc, p) = dec(a, &c, ad, npub, k, false, true, tag);
    assert_eq!(rc, 0, "{} [{tag}]: decrypt of a fresh ciphertext failed", a.p);
    assert_eq_bytes(&format!("{} [{tag}] round-trip plaintext", a.p), m, &p);
    // detached, and the detached MAC must equal the tail of the one-shot output
    let (c2, mac) = enc_det(a, m, ad, npub, k, false, true, tag);
    assert_eq_bytes(&format!("{} [{tag}] detached c == one-shot head", a.p), &c[..m.len()], &c2);
    assert_eq_bytes(&format!("{} [{tag}] detached mac == one-shot tail", a.p), &c[m.len()..], &mac);
    let (rc2, p2) = dec_det(a, &c2, &mac, ad, npub, k, false, false, tag);
    assert_eq!(rc2, 0, "{} [{tag}]: decrypt_detached of a fresh ciphertext failed", a.p);
    assert_eq_bytes(&format!("{} [{tag}] detached round-trip plaintext", a.p), m, &p2[..m.len()]);
}

// ==================================================== CONFIGS 164–174: chacha

/// The `mlen`/`adlen` sweeps named verbatim by the CONFIGS rows.
const CP_MLEN: &[usize] = &[0, 1, 63, 64, 65, 1000];
const CP_ADLEN: &[usize] = &[0, 1, 16, 17, 64];
const IETF_MLEN: &[usize] = &[0, 1, 15, 16, 17, 63, 64, 65, 1000];
const IETF_ADLEN: &[usize] = &[0, 1, 15, 16, 17];
const A128L_MLEN: &[usize] = &[0, 1, 31, 32, 33, 64, 65, 1000];
const A128L_ADLEN: &[usize] = &[0, 1, 31, 32, 33, 63, 64, 65, 127, 128];
const A256_MLEN: &[usize] = &[0, 1, 15, 16, 17, 32, 33, 1000];
const A256_ADLEN: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33];

fn sweep(a: &Aead, mlens: &[usize], adlens: &[usize], seed: u64, reps: usize, tag: &str) -> usize {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for &ml in mlens {
        for &al in adlens {
            for _ in 0..reps {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let k = rng.bytes(a.kb);
                let npub = rng.bytes(a.nb);
                round_trip(a, &m, Some(&ad), &npub, &k, tag);
                iters += 1;
            }
        }
    }
    iters
}

/// Row 164: pre-IETF chacha20poly1305 (8-byte nonce, NO 16-byte padding, the
/// two lengths appended as interleaved `STORE64_LE`s).
#[test]
fn r164_cp_encrypt_decrypt() {
    init_both();
    let mut iters = 0usize;
    let mut rng = Rng::new(SEED ^ 164);
    for &ml in CP_MLEN {
        for &al in CP_ADLEN {
            for _ in 0..3 {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let k = rng.bytes(CP.kb);
                let npub = rng.bytes(CP.nb);
                let c = enc(&CP, &m, Some(&ad), &npub, &k, false, true, "row164");
                let (rc, p) = dec(&CP, &c, Some(&ad), &npub, &k, false, true, "row164");
                assert_eq!(rc, 0, "row164 decrypt failed");
                assert_eq_bytes("row164 round trip", &m, &p);
                // a one-bit flip anywhere in c (payload or MAC) must be rejected
                if !c.is_empty() {
                    let mut bad = c.clone();
                    let i = rng.below(bad.len());
                    bad[i] ^= 1 << rng.below(8);
                    let (rb, pb) = dec(&CP, &bad, Some(&ad), &npub, &k, false, true, "row164/flip");
                    assert_eq!(rb, -1, "row164: a flipped bit at {i} was ACCEPTED");
                    assert!(pb.iter().all(|&x| x == 0), "row164: m not zeroed on MAC failure");
                }
                iters += 1;
            }
        }
    }
    // also the interesting key/nonce patterns
    let keys = patterns(CP.kb, &mut rng);
    let nonces = patterns(CP.nb, &mut rng);
    for (i, k) in keys.iter().enumerate() {
        for (j, n) in nonces.iter().enumerate() {
            let m = rng.bytes(1 + ((i + j) * 37) % 200);
            let ad = rng.bytes((i * 5 + j) % 20);
            round_trip(&CP, &m, Some(&ad), n, k, "row164/patterns");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row164 only drove {iters} inputs");
}

/// Row 165: `_encrypt_detached` / `_decrypt_detached` with a separate MAC.
#[test]
fn r165_cp_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 165);
    let mut iters = 0usize;
    for &ml in CP_MLEN {
        for &al in CP_ADLEN {
            for _ in 0..3 {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let k = rng.bytes(CP.kb);
                let npub = rng.bytes(CP.nb);
                let (c, mac) = enc_det(&CP, &m, Some(&ad), &npub, &k, false, true, "row165");
                assert_eq!(mac.len(), CP.ab);
                let (rc, p) = dec_det(&CP, &c, &mac, Some(&ad), &npub, &k, false, false, "row165");
                assert_eq!(rc, 0, "row165 decrypt_detached failed");
                assert_eq_bytes("row165 round trip", &m, &p[..ml]);
                // forged MAC: -1 and the whole plaintext buffer zeroed
                let mut bad = mac.clone();
                bad[rng.below(CP.ab)] ^= 0x80;
                let (rb, pb) =
                    dec_det(&CP, &c, &bad, Some(&ad), &npub, &k, false, false, "row165/forged");
                assert_eq!(rb, -1, "row165: forged MAC accepted");
                assert!(pb[..ml].iter().all(|&x| x == 0), "row165: m not zeroed");
                guard_intact("row165/forged", "C", &pb, ml);
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row165 only drove {iters} inputs");
}

/// Big message buffers for the `STREAM_POLY1305_CHUNK` rows, allocated once.
fn big_msgs() -> &'static (Vec<u8>, Vec<u8>) {
    static B: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    B.get_or_init(|| {
        let mut rng = Rng::new(0xC0FFEE_1234_5678);
        (rng.bytes(CHUNK), rng.bytes(CHUNK + 1))
    })
}

/// Row 166: `mlen == STREAM_POLY1305_CHUNK` and `chunk + 1`. Encryption
/// restarts `crypto_stream_chacha20_xor_ic` at `ic += cl/64` across the chunk
/// boundary while decryption does a SINGLE `xor_ic(1)` over the whole message,
/// so the two paths must agree bit-for-bit.
#[test]
fn r166_cp_chunk_boundary() {
    init_both();
    let (m0, m1) = big_msgs();
    let mut rng = Rng::new(SEED ^ 166);
    let mut iters = 0usize;
    for m in [m0, m1] {
        for _ in 0..32 {
            let k = rng.bytes(CP.kb);
            let npub = rng.bytes(CP.nb);
            let ad = rnd(&mut rng, 20);
            let c = enc(&CP, m, Some(&ad), &npub, &k, false, true, "row166");
            let (rc, p) = dec(&CP, &c, Some(&ad), &npub, &k, false, true, "row166");
            assert_eq!(rc, 0, "row166 decrypt failed for mlen={}", m.len());
            assert_eq_bytes("row166 round trip", m, &p);
            let (c2, mac) = enc_det(&CP, m, Some(&ad), &npub, &k, false, true, "row166/det");
            assert_eq_bytes("row166 detached == one-shot", &c[..m.len()], &c2);
            assert_eq_bytes("row166 detached mac", &c[m.len()..], &mac);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row166 only drove {iters} inputs");
}

/// Row 167: IETF layout — 12-byte nonce, `_pad0` of `((0x10-len)&0xf)` after
/// both `ad` and `c`, then the two `STORE64_LE` lengths at the end. The pad is
/// zero exactly when the length is a multiple of 16.
#[test]
fn r167_cpi_encrypt_decrypt() {
    init_both();
    let iters = sweep(&CPI, IETF_MLEN, IETF_ADLEN, SEED ^ 167, 2, "row167");
    // pattern matrix on top
    let mut rng = Rng::new(SEED ^ 0x167);
    let mut extra = 0usize;
    for k in patterns(CPI.kb, &mut rng) {
        for n in patterns(CPI.nb, &mut rng) {
            let m = rnd(&mut rng, 300);
            let ad = rnd(&mut rng, 40);
            round_trip(&CPI, &m, Some(&ad), &n, &k, "row167/patterns");
            extra += 1;
        }
    }
    assert!(iters + extra >= 64, "row167 only drove {} inputs", iters + extra);
}

/// Row 168: IETF `_encrypt_detached` / `_decrypt_detached`.
#[test]
fn r168_cpi_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 168);
    let mut iters = 0usize;
    for &ml in IETF_MLEN {
        for &al in IETF_ADLEN {
            for _ in 0..2 {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let k = rng.bytes(CPI.kb);
                let npub = rng.bytes(CPI.nb);
                let (c, mac) = enc_det(&CPI, &m, Some(&ad), &npub, &k, false, true, "row168");
                let (rc, p) = dec_det(&CPI, &c, &mac, Some(&ad), &npub, &k, false, false, "row168");
                assert_eq!(rc, 0, "row168 decrypt_detached failed");
                assert_eq_bytes("row168 round trip", &m, &p[..ml]);
                // a different adlen must change the MAC (the pad+length encoding
                // is what makes the IETF layout unambiguous)
                if al > 0 {
                    let (_, mac2) =
                        enc_det(&CPI, &m, Some(&ad[..al - 1]), &npub, &k, false, true, "row168/ad-1");
                    assert_ne!(mac, mac2, "row168: MAC ignores adlen");
                }
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row168 only drove {iters} inputs");
}

/// Row 169: IETF chunk boundary (the counter is `uint32_t` here).
#[test]
fn r169_cpi_chunk_boundary() {
    init_both();
    let (m0, m1) = big_msgs();
    let mut rng = Rng::new(SEED ^ 169);
    let mut iters = 0usize;
    for m in [m0, m1] {
        for _ in 0..32 {
            let k = rng.bytes(CPI.kb);
            let npub = rng.bytes(CPI.nb);
            let ad = rnd(&mut rng, 20);
            let c = enc(&CPI, m, Some(&ad), &npub, &k, false, true, "row169");
            let (rc, p) = dec(&CPI, &c, Some(&ad), &npub, &k, false, true, "row169");
            assert_eq!(rc, 0, "row169 decrypt failed for mlen={}", m.len());
            assert_eq_bytes("row169 round trip", m, &p);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row169 only drove {iters} inputs");
}

/// Row 170: XChaCha20-Poly1305-IETF — 24-byte nonce, HChaCha20 sub-key,
/// `npub2 = 4 zero bytes || npub[16..24)`.
#[test]
fn r170_xcpi_encrypt_decrypt() {
    init_both();
    let iters = sweep(&XCPI, IETF_MLEN, IETF_ADLEN, SEED ^ 170, 2, "row170");
    // The construction must equal chacha20poly1305_ietf under the derived
    // sub-key and nonce; that identity is asserted through dlsym'd primitives
    // only, and it holds for BOTH libraries because every value below has
    // already been proven identical.
    let mut rng = Rng::new(SEED ^ 0x170);
    let (hc, hr) = unsafe {
        pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int>(
            "crypto_core_hchacha20",
        )
    };
    let mut extra = 0usize;
    for _ in 0..16 {
        let k = rng.bytes(XCPI.kb);
        let npub = rng.bytes(24);
        let m = rnd(&mut rng, 200);
        let ad = rnd(&mut rng, 30);
        let mut k2c = [0u8; 32];
        let mut k2r = [0u8; 32];
        unsafe {
            hc(k2c.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), ptr::null());
            hr(k2r.as_mut_ptr(), npub.as_ptr(), k.as_ptr(), ptr::null());
        }
        assert_eq_bytes("row170 hchacha20 subkey", &k2c, &k2r);
        let mut npub2 = [0u8; 12];
        npub2[4..].copy_from_slice(&npub[16..24]);
        let a = enc(&XCPI, &m, Some(&ad), &npub, &k, false, true, "row170/id");
        let b = enc(&CPI, &m, Some(&ad), &npub2, &k2c, false, true, "row170/id-ietf");
        assert_eq_bytes("row170 xchacha == ietf(subkey, npub2)", &a, &b);
        extra += 1;
    }
    assert!(iters + extra >= 64, "row170 only drove {} inputs", iters + extra);
}

/// Row 171: XChaCha20-Poly1305-IETF detached.
#[test]
fn r171_xcpi_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 171);
    let mut iters = 0usize;
    for &ml in IETF_MLEN {
        for &al in IETF_ADLEN {
            for _ in 0..2 {
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let k = rng.bytes(XCPI.kb);
                let npub = rng.bytes(XCPI.nb);
                let (c, mac) = enc_det(&XCPI, &m, Some(&ad), &npub, &k, false, true, "row171");
                let (rc, p) = dec_det(&XCPI, &c, &mac, Some(&ad), &npub, &k, false, false, "row171");
                assert_eq!(rc, 0, "row171 decrypt_detached failed");
                assert_eq_bytes("row171 round trip", &m, &p[..ml]);
                // flipping any of the last 8 nonce bytes (which land in npub2)
                // or any of the first 16 (which feed HChaCha20) must reject
                let mut bn = npub.clone();
                bn[rng.below(24)] ^= 0x40;
                let (rb, _) = dec_det(&XCPI, &c, &mac, Some(&ad), &bn, &k, false, false, "row171/n");
                assert_eq!(rb, -1, "row171: wrong nonce accepted");
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row171 only drove {iters} inputs");
}

/// Row 172: `ad == NULL` with `adlen == 0`; `nsec != NULL` (every
/// `*_NSECBYTES` is 0 so the pointer is only ever `(void)`-cast); in-place
/// `c == m`; and `*clen_p` / `*maclen_p` / `*mlen_p` all NULL.
#[test]
fn r172_null_nsec_inplace_outparams() {
    init_both();
    let mut rng = Rng::new(SEED ^ 172);
    let mut iters = 0usize;
    for a in AEADS {
        for &ml in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 128, 200, 257] {
            let m = rng.bytes(ml);
            let k = rng.bytes(a.kb);
            let npub = rng.bytes(a.nb);
            let ad = rnd_min(&mut rng, 1, 20);

            // ad == NULL with adlen == 0 must equal ad = <empty non-null slice>
            let cn = enc(a, &m, None, &npub, &k, false, true, "row172/ad-null");
            let ce = enc(a, &m, Some(&[]), &npub, &k, false, true, "row172/ad-empty");
            assert_eq_bytes("row172 ad==NULL,adlen==0 == empty ad", &cn, &ce);
            let (rc, p) = dec(a, &cn, None, &npub, &k, false, true, "row172/ad-null");
            assert_eq!(rc, 0);
            assert_eq_bytes("row172 ad-null round trip", &m, &p);

            // nsec != NULL on every entry point
            let c2 = enc(a, &m, Some(&ad), &npub, &k, true, true, "row172/nsec");
            let c3 = enc(a, &m, Some(&ad), &npub, &k, false, true, "row172/no-nsec");
            assert_eq_bytes("row172 nsec must be ignored", &c2, &c3);
            let (rc2, _) = dec(a, &c2, Some(&ad), &npub, &k, true, true, "row172/nsec");
            assert_eq!(rc2, 0);
            let (cd, mac) = enc_det(a, &m, Some(&ad), &npub, &k, true, true, "row172/nsec");
            let (rc3, _) = dec_det(a, &cd, &mac, Some(&ad), &npub, &k, true, false, "row172/nsec");
            assert_eq!(rc3, 0);

            // NULL out-params
            let c4 = enc(a, &m, Some(&ad), &npub, &k, false, false, "row172/clen-null");
            assert_eq_bytes("row172 clen_p==NULL output", &c2, &c4);
            let (rc4, p4) = dec(a, &c4, Some(&ad), &npub, &k, false, false, "row172/mlen-null");
            assert_eq!(rc4, 0);
            assert_eq_bytes("row172 mlen_p==NULL plaintext", &m, &p4);
            let (cd2, mac2) = enc_det(a, &m, Some(&ad), &npub, &k, false, false, "row172/maclen-null");
            assert_eq_bytes("row172 maclen_p==NULL c", &cd, &cd2);
            assert_eq_bytes("row172 maclen_p==NULL mac", &mac, &mac2);

            // in-place c == m / m == c
            let cip = enc_ip(a, &m, Some(&ad), &npub, &k, "row172");
            assert_eq_bytes("row172 in-place encrypt == disjoint", &c2, &cip);
            let (rc5, pip) = dec_ip(a, &cip, Some(&ad), &npub, &k, "row172");
            assert_eq!(rc5, 0, "row172 in-place decrypt failed");
            assert_eq_bytes("row172 in-place decrypt plaintext", &m, &pip[..ml]);

            iters += 1;
        }
    }
    assert!(iters >= 64, "row172 only drove {iters} inputs");
}

/// Row 173 / ERRORS 87, 91, 95: `*_decrypt_detached` with `m == NULL` is a
/// DISTINCT code path — the C returns `crypto_verify_*`'s result directly and
/// never runs the stream cipher, so no plaintext is produced and (on failure)
/// no buffer is zeroed.
#[test]
fn r173_decrypt_detached_verify_only() {
    init_both();
    let mut rng = Rng::new(SEED ^ 173);
    let mut iters = 0usize;
    for a in AEADS {
        for &ml in &[0usize, 1, 15, 16, 31, 32, 33, 64, 65, 200, 1000] {
            for _ in 0..2 {
                let m = rng.bytes(ml);
                let ad = rnd(&mut rng, 40);
                let k = rng.bytes(a.kb);
                let npub = rng.bytes(a.nb);
                let (c, mac) = enc_det(a, &m, Some(&ad), &npub, &k, false, true, "row173");
                // valid MAC, verify-only
                let (rc, _) = dec_det(a, &c, &mac, Some(&ad), &npub, &k, false, true, "row173/ok");
                assert_eq!(rc, 0, "row173: verify-only rejected a valid MAC");
                // forged MAC, verify-only: -1 and NOTHING written
                let mut bad = mac.clone();
                bad[rng.below(a.ab)] ^= 1 << rng.below(8);
                let (rb, _) = dec_det(a, &c, &bad, Some(&ad), &npub, &k, false, true, "row173/bad");
                assert_eq!(rb, -1, "row173: verify-only accepted a forged MAC");
                // and the same forged input WITH m != NULL zeroes the plaintext
                let (rb2, pb) =
                    dec_det(a, &c, &bad, Some(&ad), &npub, &k, false, false, "row173/bad-m");
                assert_eq!(rb2, -1);
                assert!(
                    pb[..ml].iter().all(|&x| x == 0),
                    "row173: m != NULL must be memset(0) on MAC failure, got {}",
                    hexs(&pb[..ml])
                );
                guard_intact("row173/bad-m", "C", &pb, ml);
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row173 only drove {iters} inputs");
}

/// Row 174: the chacha-family `*_keygen` entry points (driven by the injected
/// deterministic RNG so the keys are byte-comparable) and every constant
/// getter. The uppercase `crypto_aead_chacha20poly1305_IETF_*` /
/// `crypto_aead_xchacha20poly1305_IETF_*` aliases are preprocessor `#define`s
/// in the public headers, NOT exported symbols: that is asserted below by
/// requiring `dlsym` to fail on them in BOTH libraries.
#[test]
fn r174_chacha_keygen_and_constants() {
    let _g = rng_lock();
    init_both();

    let want: &[(&str, usize)] = &[
        ("crypto_aead_chacha20poly1305_keybytes", 32),
        ("crypto_aead_chacha20poly1305_nsecbytes", 0),
        ("crypto_aead_chacha20poly1305_npubbytes", 8),
        ("crypto_aead_chacha20poly1305_abytes", 16),
        ("crypto_aead_chacha20poly1305_ietf_keybytes", 32),
        ("crypto_aead_chacha20poly1305_ietf_nsecbytes", 0),
        ("crypto_aead_chacha20poly1305_ietf_npubbytes", 12),
        ("crypto_aead_chacha20poly1305_ietf_abytes", 16),
        ("crypto_aead_xchacha20poly1305_ietf_keybytes", 32),
        ("crypto_aead_xchacha20poly1305_ietf_nsecbytes", 0),
        ("crypto_aead_xchacha20poly1305_ietf_npubbytes", 24),
        ("crypto_aead_xchacha20poly1305_ietf_abytes", 16),
    ];
    for (n, v) in want {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, *v, "{n}: expected {v}");
    }
    // MESSAGEBYTES_MAX
    for (n, v) in [
        ("crypto_aead_chacha20poly1305_messagebytes_max", u64::MAX - 16),
        ("crypto_aead_chacha20poly1305_ietf_messagebytes_max", 64 * ((1u64 << 32) - 1)),
        ("crypto_aead_xchacha20poly1305_ietf_messagebytes_max", u64::MAX - 16),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c() as u64, r() as u64) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
    // the uppercase aliases are macros, not symbols, in BOTH libraries
    for n in [
        "crypto_aead_chacha20poly1305_IETF_KEYBYTES",
        "crypto_aead_chacha20poly1305_IETF_NSECBYTES",
        "crypto_aead_chacha20poly1305_IETF_NPUBBYTES",
        "crypto_aead_chacha20poly1305_IETF_ABYTES",
        "crypto_aead_chacha20poly1305_IETF_MESSAGEBYTES_MAX",
        "crypto_aead_xchacha20poly1305_IETF_KEYBYTES",
        "crypto_aead_xchacha20poly1305_IETF_MESSAGEBYTES_MAX",
    ] {
        let l = libs();
        let mut nm: Vec<u8> = n.as_bytes().to_vec();
        nm.push(0);
        let gc = unsafe { l.c.get::<SizeFn>(&nm) }.is_ok();
        let gr = unsafe { l.r.get::<SizeFn>(&nm) }.is_ok();
        assert_eq!(gc, gr, "{n}: symbol presence differs (C={gc} rust={gr})");
        assert!(!gc, "{n} is a header macro and must NOT be an exported symbol");
    }

    install_det_rng(false);
    let mut iters = 0usize;
    for name in [
        "crypto_aead_chacha20poly1305_keygen",
        "crypto_aead_chacha20poly1305_ietf_keygen",
        "crypto_aead_xchacha20poly1305_ietf_keygen",
    ] {
        let (c, r) = unsafe { pair::<KeygenFn>(name) };
        for i in 0..24 {
            reset_det_rng();
            advance_det_rng(i);
            let mut kc = vec![FILL; 32 + PAD];
            let mut kr = vec![FILL; 32 + PAD];
            unsafe {
                c(kc.as_mut_ptr());
                r(kr.as_mut_ptr());
            }
            assert_eq_bytes(&format!("{name} #{i}"), &kc, &kr);
            guard_intact(name, "C", &kc, 32);
            guard_intact(name, "rust", &kr, 32);
            assert!(kc[..32] != [FILL; 32], "{name} wrote nothing");
            iters += 1;
        }
    }
    restore_default_rng();
    assert!(iters >= 64, "row174 only drove {iters} keygen inputs");
}

// ============================================== CONFIGS 175–181: AEGIS + GCM

/// Row 175: AEGIS-128L, RATE 32. The `adlen` sweep straddles both the
/// `2*RATE` `absorb2` fast path and the single-`RATE` `absorb` path, plus the
/// `adlen % RATE` zero-padded tail.
#[test]
fn r175_aegis128l_encrypt_decrypt() {
    init_both();
    let iters = sweep(&A128L, A128L_MLEN, A128L_ADLEN, SEED ^ 175, 1, "row175");
    assert!(iters >= 64, "row175 only drove {iters} inputs");
    // constants the row pins
    let (c, r) = unsafe { pair::<SizeFn>("crypto_aead_aegis128l_abytes") };
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!((a, b), (32, 32), "aegis128l ABYTES must be 32");
}

/// Row 176: AEGIS-128L detached, including the `m == NULL` scratch-decrypt path
/// (the C runs the whole `aegis128l_dec` / `aegis128l_declast` chain into a
/// stack `dst` so the state still advances identically).
#[test]
fn r176_aegis128l_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 176);
    let mut iters = 0usize;
    for &ml in A128L_MLEN {
        for &al in A128L_ADLEN {
            let m = rng.bytes(ml);
            let ad = rng.bytes(al);
            let k = rng.bytes(A128L.kb);
            let npub = rng.bytes(A128L.nb);
            let (c, mac) = enc_det(&A128L, &m, Some(&ad), &npub, &k, false, true, "row176");
            let (rc, p) = dec_det(&A128L, &c, &mac, Some(&ad), &npub, &k, false, false, "row176");
            assert_eq!(rc, 0, "row176 decrypt_detached failed");
            assert_eq_bytes("row176 round trip", &m, &p[..ml]);
            // m == NULL scratch path, valid and forged
            let (rv, _) = dec_det(&A128L, &c, &mac, Some(&ad), &npub, &k, false, true, "row176/vo");
            assert_eq!(rv, 0, "row176: scratch-decrypt rejected a valid MAC");
            let mut bad = mac.clone();
            bad[rng.below(A128L.ab)] ^= 0x11;
            let (rf, _) = dec_det(&A128L, &c, &bad, Some(&ad), &npub, &k, false, true, "row176/vo-bad");
            assert_eq!(rf, -1, "row176: scratch-decrypt accepted a forged MAC");
            let (rf2, pf) =
                dec_det(&A128L, &c, &bad, Some(&ad), &npub, &k, false, false, "row176/bad");
            assert_eq!(rf2, -1);
            assert!(pf[..ml].iter().all(|&x| x == 0), "row176: m not zeroed on failure");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row176 only drove {iters} inputs");
}

/// Row 177: AEGIS-256, RATE 16.
#[test]
fn r177_aegis256_encrypt_decrypt() {
    init_both();
    let iters = sweep(&A256, A256_MLEN, A256_ADLEN, SEED ^ 177, 1, "row177");
    assert!(iters >= 64, "row177 only drove {iters} inputs");
    for (n, v) in [
        ("crypto_aead_aegis256_abytes", 32usize),
        ("crypto_aead_aegis256_keybytes", 32),
        ("crypto_aead_aegis256_npubbytes", 32),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
}

/// Row 178: AEGIS-256 detached + `m == NULL`.
#[test]
fn r178_aegis256_detached() {
    init_both();
    let mut rng = Rng::new(SEED ^ 178);
    let mut iters = 0usize;
    for &ml in A256_MLEN {
        for &al in A256_ADLEN {
            let m = rng.bytes(ml);
            let ad = rng.bytes(al);
            let k = rng.bytes(A256.kb);
            let npub = rng.bytes(A256.nb);
            let (c, mac) = enc_det(&A256, &m, Some(&ad), &npub, &k, false, true, "row178");
            let (rc, p) = dec_det(&A256, &c, &mac, Some(&ad), &npub, &k, false, false, "row178");
            assert_eq!(rc, 0, "row178 decrypt_detached failed");
            assert_eq_bytes("row178 round trip", &m, &p[..ml]);
            let (rv, _) = dec_det(&A256, &c, &mac, Some(&ad), &npub, &k, false, true, "row178/vo");
            assert_eq!(rv, 0);
            let mut bad = mac.clone();
            bad[rng.below(A256.ab)] ^= 0x22;
            let (rf, _) = dec_det(&A256, &c, &bad, Some(&ad), &npub, &k, false, true, "row178/vo-bad");
            assert_eq!(rf, -1);
            let (rf2, pf) = dec_det(&A256, &c, &bad, Some(&ad), &npub, &k, false, false, "row178/bad");
            assert_eq!(rf2, -1);
            assert!(pf[..ml].iter().all(|&x| x == 0), "row178: m not zeroed");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row178 only drove {iters} inputs");
}

/// Row 179: `_crypto_aead_aegis{128l,256}_pick_best_implementation`. In this
/// build (no `HAVE_ARMCRYPTO`, no `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H`) the
/// selection can only ever land on the `*_soft` implementation, so the function
/// returns 0 and the observable behaviour before/after is bit-identical.
#[test]
fn r179_aegis_pick_best_implementation() {
    init_both();
    let mut rng = Rng::new(SEED ^ 179);
    let mut iters = 0usize;
    for (fam, name) in [
        (&A128L, "_crypto_aead_aegis128l_pick_best_implementation"),
        (&A256, "_crypto_aead_aegis256_pick_best_implementation"),
    ] {
        // baseline outputs BEFORE the explicit pick
        let mut cases = vec![];
        for _ in 0..32 {
            let m = rnd(&mut rng, 300);
            let ad = rnd(&mut rng, 130);
            let k = rng.bytes(fam.kb);
            let npub = rng.bytes(fam.nb);
            let c = enc(fam, &m, Some(&ad), &npub, &k, false, true, "row179/before");
            cases.push((m, ad, k, npub, c));
        }
        let (pc, pr) = unsafe { pair::<IntFn>(name) };
        let (a, b) = unsafe { (pc(), pr()) };
        assert_eq!(a, b, "{name}: return differs (C={a} rust={b})");
        assert_eq!(a, 0, "{name}: must return 0");
        // idempotent: calling it again changes nothing
        let (a2, b2) = unsafe { (pc(), pr()) };
        assert_eq!((a2, b2), (0, 0), "{name}: second call differs");
        for (m, ad, k, npub, want) in &cases {
            let got = enc(fam, m, Some(ad), npub, k, false, true, "row179/after");
            assert_eq_bytes(
                &format!("{name}: output changed after picking the implementation"),
                want,
                &got,
            );
            let (rc, p) = dec(fam, &got, Some(ad), npub, k, false, true, "row179/after");
            assert_eq!(rc, 0);
            assert_eq_bytes("row179 round trip after pick", m, &p);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row179 only drove {iters} inputs");
}

/// Row 180: AEGIS `*_keygen` + every constant getter.
#[test]
fn r180_aegis_keygen_and_constants() {
    let _g = rng_lock();
    init_both();
    for (n, v) in [
        ("crypto_aead_aegis128l_keybytes", 16usize),
        ("crypto_aead_aegis128l_nsecbytes", 0),
        ("crypto_aead_aegis128l_npubbytes", 16),
        ("crypto_aead_aegis128l_abytes", 32),
        ("crypto_aead_aegis256_keybytes", 32),
        ("crypto_aead_aegis256_nsecbytes", 0),
        ("crypto_aead_aegis256_npubbytes", 32),
        ("crypto_aead_aegis256_abytes", 32),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
    for n in ["crypto_aead_aegis128l_messagebytes_max", "crypto_aead_aegis256_messagebytes_max"] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c() as u64, r() as u64) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, (1u64 << 61) - 1, "{n}: expected min(SIZE_MAX-32, 2^61-1)");
    }

    install_det_rng(false);
    let mut iters = 0usize;
    for (name, kb) in [
        ("crypto_aead_aegis128l_keygen", 16usize),
        ("crypto_aead_aegis256_keygen", 32),
    ] {
        let (c, r) = unsafe { pair::<KeygenFn>(name) };
        for i in 0..40 {
            reset_det_rng();
            advance_det_rng(i);
            let mut kc = vec![FILL; kb + PAD];
            let mut kr = vec![FILL; kb + PAD];
            unsafe {
                c(kc.as_mut_ptr());
                r(kr.as_mut_ptr());
            }
            assert_eq_bytes(&format!("{name} #{i}"), &kc, &kr);
            guard_intact(name, "C", &kc, kb);
            guard_intact(name, "rust", &kr, kb);
            iters += 1;
        }
    }
    restore_default_rng();
    assert!(iters >= 64, "row180 only drove {iters} keygen inputs");
}

/// The 9 AES-256-GCM entry points that are ENOSYS stubs in this build, driven
/// with random-but-valid-looking arguments. Every buffer is prefilled 0xAA and
/// asserted untouched afterwards: the stub must return before writing anything.
#[allow(clippy::type_complexity)]
fn gcm_stub_case(rng: &mut Rng, tag: &str) {
    let mlen = rng.below(200);
    let adlen = rng.below(50);
    let m = rng.bytes(mlen);
    let ad = rng.bytes(adlen);
    let k = rng.bytes(32);
    let npub = rng.bytes(12);
    let sb = {
        let (c, r) = unsafe { pair::<SizeFn>("crypto_aead_aes256gcm_statebytes") };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "crypto_aead_aes256gcm_statebytes differs (C={a} rust={b})");
        a
    };

    // one buffer set per library so an accidental write is visible
    let mk = |n: usize| (vec![FILL; n + PAD], vec![FILL; n + PAD]);

    macro_rules! chk {
        ($name:literal, $bufs:expr, $outs:expr, $callc:expr, $callr:expr) => {{
            let (mut bc, mut br) = $bufs;
            let (mut oc, mut or) = $outs;
            errno_set(0);
            let rc = $callc(&mut bc, &mut oc);
            let ec = errno_get();
            errno_set(0);
            let rr = $callr(&mut br, &mut or);
            let er = errno_get();
            let what = format!("{} [{tag}] mlen={mlen} adlen={adlen}", $name);
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: expected -1");
            assert_eq!(ec, er, "{what}: errno differs (C={ec} rust={er})");
            assert_eq!(ec, ENOSYS, "{what}: errno must be ENOSYS ({ENOSYS}), got {ec}");
            assert_eq_bytes(&what, &bc, &br);
            assert!(
                bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                "{what}: an ENOSYS stub wrote into the output buffer"
            );
            assert_eq!(oc, or, "{what}: out-param differs");
            assert_eq!(oc, U64SENT, "{what}: an ENOSYS stub wrote the out-param");
        }};
    }

    let ce = unsafe { sym::<AeadEnc>(&libs().c, "crypto_aead_aes256gcm_encrypt") };
    let re = unsafe { sym::<AeadEnc>(&libs().r, "crypto_aead_aes256gcm_encrypt") };
    chk!(
        "crypto_aead_aes256gcm_encrypt",
        mk(mlen + 16),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            ce(b.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(), adlen as u64,
               ptr::null(), npub.as_ptr(), k.as_ptr())
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            re(b.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(), adlen as u64,
               ptr::null(), npub.as_ptr(), k.as_ptr())
        }
    );

    let cd = unsafe { sym::<AeadDec>(&libs().c, "crypto_aead_aes256gcm_decrypt") };
    let rd = unsafe { sym::<AeadDec>(&libs().r, "crypto_aead_aes256gcm_decrypt") };
    chk!(
        "crypto_aead_aes256gcm_decrypt",
        mk(mlen + 16),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            cd(b.as_mut_ptr(), o, ptr::null_mut(), m.as_ptr(), mlen as u64, ad.as_ptr(),
               adlen as u64, npub.as_ptr(), k.as_ptr())
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            rd(b.as_mut_ptr(), o, ptr::null_mut(), m.as_ptr(), mlen as u64, ad.as_ptr(),
               adlen as u64, npub.as_ptr(), k.as_ptr())
        }
    );

    let ced = unsafe { sym::<AeadEncDet>(&libs().c, "crypto_aead_aes256gcm_encrypt_detached") };
    let red = unsafe { sym::<AeadEncDet>(&libs().r, "crypto_aead_aes256gcm_encrypt_detached") };
    let mut macc = vec![FILL; 16 + PAD];
    let mut macr = vec![FILL; 16 + PAD];
    chk!(
        "crypto_aead_aes256gcm_encrypt_detached",
        mk(mlen),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            ced(b.as_mut_ptr(), macc.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(),
                adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr())
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            red(b.as_mut_ptr(), macr.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(),
                adlen as u64, ptr::null(), npub.as_ptr(), k.as_ptr())
        }
    );
    assert_eq_bytes("aes256gcm_encrypt_detached mac", &macc, &macr);
    assert!(macc.iter().all(|&x| x == FILL), "encrypt_detached stub wrote the MAC");

    let cdd = unsafe { sym::<AeadDecDet>(&libs().c, "crypto_aead_aes256gcm_decrypt_detached") };
    let rdd = unsafe { sym::<AeadDecDet>(&libs().r, "crypto_aead_aes256gcm_decrypt_detached") };
    let fake_mac = [0x5Au8; 16];
    chk!(
        "crypto_aead_aes256gcm_decrypt_detached",
        mk(mlen),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, _o: &mut u64| unsafe {
            cdd(b.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, fake_mac.as_ptr(),
                ad.as_ptr(), adlen as u64, npub.as_ptr(), k.as_ptr())
        },
        |b: &mut Vec<u8>, _o: &mut u64| unsafe {
            rdd(b.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, fake_mac.as_ptr(),
                ad.as_ptr(), adlen as u64, npub.as_ptr(), k.as_ptr())
        }
    );

    // _beforenm: the state buffer must stay untouched
    let cb = unsafe { sym::<BeforenmFn>(&libs().c, "crypto_aead_aes256gcm_beforenm") };
    let rb = unsafe { sym::<BeforenmFn>(&libs().r, "crypto_aead_aes256gcm_beforenm") };
    let mut stc = Aligned16::new(sb, FILL);
    let mut str_ = Aligned16::new(sb, FILL);
    errno_set(0);
    let r1 = unsafe { cb(stc.p(), k.as_ptr()) };
    let e1 = errno_get();
    errno_set(0);
    let r2 = unsafe { rb(str_.p(), k.as_ptr()) };
    let e2 = errno_get();
    assert_eq!(r1, r2, "crypto_aead_aes256gcm_beforenm [{tag}]: return differs");
    assert_eq!(r1, -1, "crypto_aead_aes256gcm_beforenm must return -1");
    assert_eq!(e1, e2, "crypto_aead_aes256gcm_beforenm: errno differs");
    assert_eq!(e1, ENOSYS, "crypto_aead_aes256gcm_beforenm: errno must be ENOSYS");
    assert_eq_bytes("aes256gcm_beforenm state", stc.all(), str_.all());
    assert!(
        stc.all().iter().all(|&x| x == FILL),
        "crypto_aead_aes256gcm_beforenm wrote into the state ({} bytes)",
        stc.len()
    );

    // the four *_afternm entry points take the state in place of the key
    let stp_c = stc.p();
    let stp_r = str_.p();
    let cea = unsafe { sym::<AeadEnc>(&libs().c, "crypto_aead_aes256gcm_encrypt_afternm") };
    let rea = unsafe { sym::<AeadEnc>(&libs().r, "crypto_aead_aes256gcm_encrypt_afternm") };
    chk!(
        "crypto_aead_aes256gcm_encrypt_afternm",
        mk(mlen + 16),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            cea(b.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(), adlen as u64,
                ptr::null(), npub.as_ptr(), stp_c)
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            rea(b.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(), adlen as u64,
                ptr::null(), npub.as_ptr(), stp_r)
        }
    );

    let ceda =
        unsafe { sym::<AeadEncDet>(&libs().c, "crypto_aead_aes256gcm_encrypt_detached_afternm") };
    let reda =
        unsafe { sym::<AeadEncDet>(&libs().r, "crypto_aead_aes256gcm_encrypt_detached_afternm") };
    let mut mac2c = vec![FILL; 16 + PAD];
    let mut mac2r = vec![FILL; 16 + PAD];
    chk!(
        "crypto_aead_aes256gcm_encrypt_detached_afternm",
        mk(mlen),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            ceda(b.as_mut_ptr(), mac2c.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(),
                 adlen as u64, ptr::null(), npub.as_ptr(), stp_c)
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            reda(b.as_mut_ptr(), mac2r.as_mut_ptr(), o, m.as_ptr(), mlen as u64, ad.as_ptr(),
                 adlen as u64, ptr::null(), npub.as_ptr(), stp_r)
        }
    );
    assert_eq_bytes("aes256gcm_encrypt_detached_afternm mac", &mac2c, &mac2r);
    assert!(mac2c.iter().all(|&x| x == FILL));

    let cda = unsafe { sym::<AeadDec>(&libs().c, "crypto_aead_aes256gcm_decrypt_afternm") };
    let rda = unsafe { sym::<AeadDec>(&libs().r, "crypto_aead_aes256gcm_decrypt_afternm") };
    chk!(
        "crypto_aead_aes256gcm_decrypt_afternm",
        mk(mlen + 16),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            cda(b.as_mut_ptr(), o, ptr::null_mut(), m.as_ptr(), mlen as u64, ad.as_ptr(),
                adlen as u64, npub.as_ptr(), stp_c)
        },
        |b: &mut Vec<u8>, o: &mut u64| unsafe {
            rda(b.as_mut_ptr(), o, ptr::null_mut(), m.as_ptr(), mlen as u64, ad.as_ptr(),
                adlen as u64, npub.as_ptr(), stp_r)
        }
    );

    let cdda =
        unsafe { sym::<AeadDecDet>(&libs().c, "crypto_aead_aes256gcm_decrypt_detached_afternm") };
    let rdda =
        unsafe { sym::<AeadDecDet>(&libs().r, "crypto_aead_aes256gcm_decrypt_detached_afternm") };
    chk!(
        "crypto_aead_aes256gcm_decrypt_detached_afternm",
        mk(mlen),
        (U64SENT, U64SENT),
        |b: &mut Vec<u8>, _o: &mut u64| unsafe {
            cdda(b.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, fake_mac.as_ptr(),
                 ad.as_ptr(), adlen as u64, npub.as_ptr(), stp_c)
        },
        |b: &mut Vec<u8>, _o: &mut u64| unsafe {
            rdda(b.as_mut_ptr(), ptr::null_mut(), m.as_ptr(), mlen as u64, fake_mac.as_ptr(),
                 ad.as_ptr(), adlen as u64, npub.as_ptr(), stp_r)
        }
    );
}

/// Row 181 + ERRORS 106: `crypto_aead_aes256gcm_is_available()` returns 0 and
/// all 9 operational entry points are ENOSYS stubs; `_statebytes` is still a
/// 16-aligned `sizeof(state)` and every constant getter still works.
#[test]
fn r181_aes256gcm_stubs_and_constants() {
    init_both();
    // ERRORS 106
    let (c, r) = unsafe { pair::<IntFn>("crypto_aead_aes256gcm_is_available") };
    for _ in 0..8 {
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "crypto_aead_aes256gcm_is_available: C={a} rust={b}");
        assert_eq!(a, 0, "this build has no HW AES; is_available must be 0");
    }
    for (n, v) in [
        ("crypto_aead_aes256gcm_keybytes", 32usize),
        ("crypto_aead_aes256gcm_nsecbytes", 0),
        ("crypto_aead_aes256gcm_npubbytes", 12),
        ("crypto_aead_aes256gcm_abytes", 16),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
    let (c, r) = unsafe { pair::<SizeFn>("crypto_aead_aes256gcm_messagebytes_max") };
    let (a, b) = unsafe { (c() as u64, r() as u64) };
    assert_eq!(a, b, "crypto_aead_aes256gcm_messagebytes_max: C={a} rust={b}");
    assert_eq!(a, 16 * ((1u64 << 32) - 2), "aes256gcm MESSAGEBYTES_MAX");
    let (c, r) = unsafe { pair::<SizeFn>("crypto_aead_aes256gcm_statebytes") };
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "crypto_aead_aes256gcm_statebytes: C={a} rust={b}");
    assert_eq!(a % 16, 0, "statebytes must be 16-aligned, got {a}");
    assert_eq!(a, 512, "sizeof(crypto_aead_aes256gcm_state) is 512");

    let mut rng = Rng::new(SEED ^ 181);
    for i in 0..64 {
        gcm_stub_case(&mut rng, &format!("row181#{i}"));
    }
}

/// ERRORS 107–115: every one of the 9 operational entry points returns -1 with
/// `errno == ENOSYS`, for 64 randomized argument sets, with the full output
/// buffer and every out-param asserted untouched.
#[test]
fn e106_e115_aes256gcm_enosys() {
    init_both();
    let (c, r) = unsafe { pair::<IntFn>("crypto_aead_aes256gcm_is_available") };
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!((a, b), (0, 0), "ERRORS 106: is_available must be 0 in both");
    let mut rng = Rng::new(SEED ^ 0x106);
    for i in 0..64 {
        gcm_stub_case(&mut rng, &format!("e107-115#{i}"));
    }
    // errno is not clobbered when the caller had a different value and the
    // stub still fails: the C assigns unconditionally.
    errno_set(libc::EINVAL);
    let ce = unsafe { sym::<AeadEnc>(&libs().c, "crypto_aead_aes256gcm_encrypt") };
    let mut buf = vec![FILL; 64];
    let k = [1u8; 32];
    let np = [2u8; 12];
    let rc = unsafe {
        ce(buf.as_mut_ptr(), ptr::null_mut(), buf.as_ptr(), 0, ptr::null(), 0, ptr::null(),
           np.as_ptr(), k.as_ptr())
    };
    assert_eq!(rc, -1);
    assert_eq!(errno_get(), ENOSYS, "errno must be overwritten with ENOSYS");
}

// ================================================ CONFIGS 182–191: secretbox

struct Sb {
    p: &'static str,
    kb: usize,
    nb: usize,
    mb: usize,
}
const SBS: Sb = Sb { p: "crypto_secretbox", kb: 32, nb: 24, mb: 16 };
const SBX: Sb = Sb { p: "crypto_secretbox_xchacha20poly1305", kb: 32, nb: 24, mb: 16 };

impl Sb {
    fn n(&self, s: &str) -> String {
        format!("{}{}", self.p, s)
    }
}

/// The row-182/189 `mlen` sweep: `mlen0 = min(mlen, 32)` is the first-block
/// axis, so 31/32/33 straddle it exactly.
const SB_MLEN: &[usize] = &[0, 1, 31, 32, 33, 64, 1000];

/// `*_easy`. Returns the `mlen + MACBYTES` output.
fn sb_easy(s: &Sb, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> Vec<u8> {
    let name = s.n("_easy");
    let (fc, fr) = unsafe { pair::<SbEasy>(&name) };
    let out = m.len() + s.mb;
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), m.as_ptr(), m.len() as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!("{name} [{tag}] mlen={} k={} n={}", m.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: expected 0");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, out);
    guard_intact(&what, "rust", &br, out);
    bc.truncate(out);
    bc
}

/// `*_open_easy`. Returns `(rc, full prefilled plaintext buffer + guard)`.
fn sb_open_easy(s: &Sb, c: &[u8], n: &[u8], k: &[u8], tag: &str) -> (c_int, Vec<u8>) {
    let name = s.n("_open_easy");
    let (fc, fr) = unsafe { pair::<SbEasy>(&name) };
    let out = c.len().saturating_sub(s.mb);
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let rc = unsafe { fc(bc.as_mut_ptr(), c.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(br.as_mut_ptr(), c.as_ptr(), c.len() as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!("{name} [{tag}] clen={} k={} n={}", c.len(), hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    guard_intact(&what, "C", &bc, out);
    guard_intact(&what, "rust", &br, out);
    (rc, bc)
}

/// `*_detached` with fully DISJOINT buffers. Returns `(c, mac)`.
fn sb_det(s: &Sb, m: &[u8], n: &[u8], k: &[u8], tag: &str) -> (Vec<u8>, Vec<u8>) {
    let name = s.n("_detached");
    let (fc, fr) = unsafe { pair::<SbDet>(&name) };
    let mlen = m.len();
    let mut bc = vec![FILL; mlen + PAD];
    let mut br = vec![FILL; mlen + PAD];
    let mut mc = vec![FILL; s.mb + PAD];
    let mut mr = vec![FILL; s.mb + PAD];
    let rc = unsafe {
        fc(bc.as_mut_ptr(), mc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
    };
    let rr = unsafe {
        fr(br.as_mut_ptr(), mr.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr())
    };
    let what = format!("{name} [{tag}] mlen={mlen} k={} n={}", hexs(k), hexs(n));
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: expected 0");
    assert_eq_bytes(&format!("{what} [c]"), &bc, &br);
    assert_eq_bytes(&format!("{what} [mac]"), &mc, &mr);
    guard_intact(&what, "C c", &bc, mlen);
    guard_intact(&what, "rust c", &br, mlen);
    guard_intact(&what, "C mac", &mc, s.mb);
    guard_intact(&what, "rust mac", &mr, s.mb);
    bc.truncate(mlen);
    mc.truncate(s.mb);
    (bc, mc)
}

/// `*_open_detached`, disjoint buffers. `m_null` selects verify-only mode.
fn sb_open_det(
    s: &Sb,
    c: &[u8],
    mac: &[u8],
    n: &[u8],
    k: &[u8],
    m_null: bool,
    tag: &str,
) -> (c_int, Vec<u8>) {
    let name = s.n("_open_detached");
    let (fc, fr) = unsafe { pair::<SbOpenDet>(&name) };
    let clen = c.len();
    let mut bc = vec![FILL; clen + PAD];
    let mut br = vec![FILL; clen + PAD];
    let (pc, pr) = if m_null {
        (ptr::null_mut(), ptr::null_mut())
    } else {
        (bc.as_mut_ptr(), br.as_mut_ptr())
    };
    let rc = unsafe { fc(pc, c.as_ptr(), mac.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(pr, c.as_ptr(), mac.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
    let what = format!(
        "{name} [{tag}] clen={clen} m_null={m_null} k={} n={} mac={}",
        hexs(k), hexs(n), hexs(mac)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&what, &bc, &br);
    if !m_null {
        guard_intact(&what, "C", &bc, clen);
        guard_intact(&what, "rust", &br, clen);
    } else {
        assert!(
            bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
            "{what}: verify-only mode must not touch a plaintext buffer"
        );
    }
    (rc, bc)
}

/// The four overlap shapes of CONFIGS 184 / 190.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Shape {
    Disjoint,
    InPlace,
    /// `c = m + shift` (0 < shift < mlen): the C memmove()s FORWARD.
    Forward,
    /// `m = c + shift` (0 < shift < mlen): the C memmove()s BACKWARD.
    Backward,
}

/// Returns `(buflen, m_off, c_off)` for one overlap shape.
fn shape_offsets(sh: Shape, mlen: usize, shift: usize) -> (usize, usize, usize) {
    match sh {
        Shape::Disjoint => (2 * mlen + 2 * PAD, 0, mlen + PAD),
        Shape::InPlace => (mlen + PAD, 0, 0),
        Shape::Forward => (mlen + shift + PAD, 0, shift),
        Shape::Backward => (mlen + shift + PAD, shift, 0),
    }
}

/// `*_detached` driven with `c` and `m` at controlled offsets inside ONE buffer,
/// so the C's explicit `uintptr_t` partial-overlap detection + `memmove` runs.
/// The FULL buffer (both libraries) is compared, so the memmove's side effects
/// on the untouched parts of the buffer are part of the assertion.
#[allow(clippy::too_many_arguments)]
fn sb_det_shaped(
    s: &Sb,
    m: &[u8],
    n: &[u8],
    k: &[u8],
    sh: Shape,
    shift: usize,
    tag: &str,
) -> (Vec<u8>, Vec<u8>, usize) {
    let name = s.n("_detached");
    let (fc, fr) = unsafe { pair::<SbDet>(&name) };
    let mlen = m.len();
    let (bl, mo, co) = shape_offsets(sh, mlen, shift);
    let mut bc = vec![FILL; bl];
    bc[mo..mo + mlen].copy_from_slice(m);
    let mut br = bc.clone();
    let mut mc = vec![FILL; s.mb + PAD];
    let mut mr = vec![FILL; s.mb + PAD];
    let rc = unsafe {
        fc(
            bc.as_mut_ptr().add(co), mc.as_mut_ptr(), bc.as_ptr().add(mo), mlen as u64,
            n.as_ptr(), k.as_ptr(),
        )
    };
    let rr = unsafe {
        fr(
            br.as_mut_ptr().add(co), mr.as_mut_ptr(), br.as_ptr().add(mo), mlen as u64,
            n.as_ptr(), k.as_ptr(),
        )
    };
    let what = format!(
        "{name} [{tag}] shape={sh:?} shift={shift} mlen={mlen} m_off={mo} c_off={co} k={}",
        hexs(k)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what}: expected 0");
    assert_eq_bytes(&format!("{what} [buffer]"), &bc, &br);
    assert_eq_bytes(&format!("{what} [mac]"), &mc, &mr);
    guard_intact(&what, "C mac", &mc, s.mb);
    guard_intact(&what, "rust mac", &mr, s.mb);
    mc.truncate(s.mb);
    (bc, mc, co)
}

/// `*_open_detached` with the same offset control.
#[allow(clippy::too_many_arguments)]
fn sb_open_det_shaped(
    s: &Sb,
    c: &[u8],
    mac: &[u8],
    n: &[u8],
    k: &[u8],
    sh: Shape,
    shift: usize,
    tag: &str,
) -> (c_int, Vec<u8>, usize) {
    let name = s.n("_open_detached");
    let (fc, fr) = unsafe { pair::<SbOpenDet>(&name) };
    let clen = c.len();
    // for open, `c` is the input and `m` the output: swap the roles of the two
    // offsets so `Forward` still means "the destination is above the source".
    let (bl, co, mo) = shape_offsets(sh, clen, shift);
    let mut bc = vec![FILL; bl];
    bc[co..co + clen].copy_from_slice(c);
    let mut br = bc.clone();
    let rc = unsafe {
        fc(
            bc.as_mut_ptr().add(mo), bc.as_ptr().add(co), mac.as_ptr(), clen as u64,
            n.as_ptr(), k.as_ptr(),
        )
    };
    let rr = unsafe {
        fr(
            br.as_mut_ptr().add(mo), br.as_ptr().add(co), mac.as_ptr(), clen as u64,
            n.as_ptr(), k.as_ptr(),
        )
    };
    let what = format!(
        "{name} [{tag}] shape={sh:?} shift={shift} clen={clen} c_off={co} m_off={mo} k={}",
        hexs(k)
    );
    assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&format!("{what} [buffer]"), &bc, &br);
    (rc, bc, mo)
}

/// Row 182: `crypto_secretbox_easy` / `_open_easy`; `block0` carries
/// `min(mlen,32)` message bytes after 32 zeros.
#[test]
fn r182_secretbox_easy() {
    init_both();
    let mut rng = Rng::new(SEED ^ 182);
    let mut iters = 0usize;
    for &ml in SB_MLEN {
        for _ in 0..10 {
            let m = rng.bytes(ml);
            let k = rng.bytes(SBS.kb);
            let n = rng.bytes(SBS.nb);
            let c = sb_easy(&SBS, &m, &n, &k, "row182");
            let (rc, p) = sb_open_easy(&SBS, &c, &n, &k, "row182");
            assert_eq!(rc, 0, "row182 open_easy failed");
            assert_eq_bytes("row182 round trip", &m, &p[..ml]);
            // easy == MAC || detached-ciphertext
            let (cd, mac) = sb_det(&SBS, &m, &n, &k, "row182/det");
            assert_eq_bytes("row182 easy head is the MAC", &c[..SBS.mb], &mac);
            assert_eq_bytes("row182 easy tail is the ciphertext", &c[SBS.mb..], &cd);
            // any single-bit flip is rejected and leaves m untouched
            let mut bad = c.clone();
            let i = rng.below(bad.len());
            bad[i] ^= 1 << rng.below(8);
            let (rb, pb) = sb_open_easy(&SBS, &bad, &n, &k, "row182/flip");
            assert_eq!(rb, -1, "row182: flipped bit at {i} accepted");
            assert!(
                pb.iter().all(|&x| x == FILL),
                "row182: open_easy must NOT write m on a MAC failure, got {}",
                hexs(&pb)
            );
            iters += 1;
        }
    }
    // key / nonce pattern matrix
    for k in patterns(SBS.kb, &mut rng) {
        for n in patterns(SBS.nb, &mut rng) {
            let m = rnd(&mut rng, 200);
            let c = sb_easy(&SBS, &m, &n, &k, "row182/patterns");
            let (rc, p) = sb_open_easy(&SBS, &c, &n, &k, "row182/patterns");
            assert_eq!(rc, 0);
            assert_eq_bytes("row182 pattern round trip", &m, &p[..m.len()]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row182 only drove {iters} inputs");
}

/// Row 183: `crypto_secretbox_detached` / `_open_detached` with DISJOINT
/// buffers over the same `mlen` sweep.
#[test]
fn r183_secretbox_detached_disjoint() {
    init_both();
    let mut rng = Rng::new(SEED ^ 183);
    let mut iters = 0usize;
    for &ml in SB_MLEN {
        for _ in 0..10 {
            let m = rng.bytes(ml);
            let k = rng.bytes(SBS.kb);
            let n = rng.bytes(SBS.nb);
            let (c, mac) = sb_det(&SBS, &m, &n, &k, "row183");
            let (rc, p) = sb_open_det(&SBS, &c, &mac, &n, &k, false, "row183");
            assert_eq!(rc, 0, "row183 open_detached failed");
            assert_eq_bytes("row183 round trip", &m, &p[..ml]);
            // wrong nonce / wrong key must be rejected
            let mut bn = n.clone();
            bn[rng.below(SBS.nb)] ^= 0x08;
            let (rb, pb) = sb_open_det(&SBS, &c, &mac, &bn, &k, false, "row183/nonce");
            assert_eq!(rb, -1, "row183: wrong nonce accepted");
            assert!(pb.iter().all(|&x| x == FILL), "row183: m written on failure");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row183 only drove {iters} inputs");
}

/// Row 184: THE OVERLAP AXIS. `crypto_secretbox_detached` / `_open_detached`
/// do explicit `uintptr_t` partial-overlap detection followed by `memmove`;
/// all four distinct shapes must produce byte-identical whole buffers.
#[test]
fn r184_secretbox_detached_overlap() {
    init_both();
    let mut rng = Rng::new(SEED ^ 184);
    let mut iters = 0usize;
    for &ml in &[2usize, 16, 31, 32, 33, 48, 64, 65, 200, 1000] {
        let shifts = [1usize, ml / 2, ml - 1];
        for &shift in &shifts {
            if shift == 0 || shift >= ml {
                continue;
            }
            for _ in 0..2 {
                let m = rng.bytes(ml);
                let k = rng.bytes(SBS.kb);
                let n = rng.bytes(SBS.nb);
                // the reference (disjoint) answer
                let (cref, macref) = sb_det(&SBS, &m, &n, &k, "row184/ref");
                for sh in [Shape::Disjoint, Shape::InPlace, Shape::Forward, Shape::Backward] {
                    let (buf, mac, co) = sb_det_shaped(&SBS, &m, &n, &k, sh, shift, "row184");
                    assert_eq_bytes(
                        &format!("row184 {sh:?} shift={shift} mlen={ml}: MAC != disjoint MAC"),
                        &macref,
                        &mac,
                    );
                    assert_eq_bytes(
                        &format!("row184 {sh:?} shift={shift} mlen={ml}: c != disjoint c"),
                        &cref,
                        &buf[co..co + ml],
                    );
                    // and the ciphertext opens back to the plaintext
                    let (rc, p, mo) = sb_open_det_shaped(
                        &SBS, &cref, &macref, &n, &k, sh, shift, "row184/open",
                    );
                    assert_eq!(rc, 0, "row184 {sh:?}: open_detached failed");
                    assert_eq_bytes(
                        &format!("row184 {sh:?} shift={shift} open plaintext"),
                        &m,
                        &p[mo..mo + ml],
                    );
                    iters += 1;
                }
            }
        }
    }
    assert!(iters >= 64, "row184 only drove {iters} inputs");
}

/// Row 185: `mlen == STREAM_POLY1305_CHUNK` / `chunk + 1` for
/// `crypto_secretbox_detached`. The 64-bit `ic` restarts at `mlen0` and steps
/// by `cl / 64` per chunk, while `_open_detached` decrypts the tail with a
/// SINGLE `xor_ic(1)`.
#[test]
fn r185_secretbox_detached_chunk_boundary() {
    init_both();
    let (m0, m1) = big_msgs();
    let mut rng = Rng::new(SEED ^ 185);
    let mut iters = 0usize;
    for m in [m0, m1] {
        for _ in 0..32 {
            let k = rng.bytes(SBS.kb);
            let n = rng.bytes(SBS.nb);
            let (c, mac) = sb_det(&SBS, m, &n, &k, "row185");
            let (rc, p) = sb_open_det(&SBS, &c, &mac, &n, &k, false, "row185");
            assert_eq!(rc, 0, "row185 open_detached failed for mlen={}", m.len());
            assert_eq_bytes("row185 round trip", m, &p[..m.len()]);
            // easy() over the same message must agree
            let ce = sb_easy(&SBS, m, &n, &k, "row185/easy");
            assert_eq_bytes("row185 easy MAC", &ce[..SBS.mb], &mac);
            assert_eq_bytes("row185 easy ciphertext", &ce[SBS.mb..], &c);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row185 only drove {iters} inputs");
}

/// Row 186: the deprecated NaCl-padded `crypto_secretbox` / `crypto_secretbox_open`.
/// `m` needs 32 leading zero bytes, the output gets 16 leading zero bytes, and
/// `mlen` must be at least `ZEROBYTES` (32).
#[test]
fn r186_secretbox_nacl_padded() {
    init_both();
    let (ec, er) = unsafe { pair::<SbEasy>("crypto_secretbox") };
    let (oc, or) = unsafe { pair::<SbEasy>("crypto_secretbox_open") };
    let mut rng = Rng::new(SEED ^ 186);
    let mut iters = 0usize;
    for &ml in &[32usize, 33, 64, 96, 1000] {
        for _ in 0..14 {
            let mut m = vec![0u8; 32];
            m.extend_from_slice(&rng.bytes(ml - 32));
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut cc = vec![FILL; ml + PAD];
            let mut cr = vec![FILL; ml + PAD];
            let rc = unsafe { ec(cc.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let rr = unsafe { er(cr.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let what = format!("crypto_secretbox [row186] mlen={ml} k={}", hexs(&k));
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, 0, "{what}: expected 0");
            assert_eq_bytes(&what, &cc, &cr);
            guard_intact(&what, "C", &cc, ml);
            guard_intact(&what, "rust", &cr, ml);
            assert!(
                cc[..16].iter().all(|&x| x == 0),
                "{what}: the first 16 output bytes must be zero, got {}",
                hexs(&cc[..16])
            );
            // open
            let mut pc = vec![FILL; ml + PAD];
            let mut pr = vec![FILL; ml + PAD];
            let rc2 =
                unsafe { oc(pc.as_mut_ptr(), cc.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let rr2 =
                unsafe { or(pr.as_mut_ptr(), cc.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let what2 = format!("crypto_secretbox_open [row186] clen={ml}");
            assert_eq!(rc2, rr2, "{what2}: return differs (C={rc2} rust={rr2})");
            assert_eq!(rc2, 0, "{what2}: expected 0");
            assert_eq_bytes(&what2, &pc, &pr);
            guard_intact(&what2, "C", &pc, ml);
            guard_intact(&what2, "rust", &pr, ml);
            assert_eq_bytes("row186 round trip (incl. the 32 zero bytes)", &m, &pc[..ml]);
            // the padded form must agree with _easy on the payload
            let ce = sb_easy(&SBS, &m[32..], &n, &k, "row186/easy");
            assert_eq_bytes("row186 NaCl MAC == easy MAC", &cc[16..32], &ce[..16]);
            assert_eq_bytes("row186 NaCl ct == easy ct", &cc[32..ml], &ce[16..]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row186 only drove {iters} inputs");
}

/// Row 187: the primitive-level `crypto_secretbox_xsalsa20poly1305[_open]`,
/// which `crypto_secretbox[_open]` merely forwards to.
#[test]
fn r187_secretbox_xsalsa20poly1305() {
    init_both();
    let (ec, er) = unsafe { pair::<SbEasy>("crypto_secretbox_xsalsa20poly1305") };
    let (oc, or) = unsafe { pair::<SbEasy>("crypto_secretbox_xsalsa20poly1305_open") };
    let (dc, dr) = unsafe { pair::<SbEasy>("crypto_secretbox") };
    let (doc, dor) = unsafe { pair::<SbEasy>("crypto_secretbox_open") };
    let mut rng = Rng::new(SEED ^ 187);
    let mut iters = 0usize;
    for &ml in &[32usize, 33, 64, 96, 1000] {
        for _ in 0..14 {
            let mut m = vec![0u8; 32];
            m.extend_from_slice(&rng.bytes(ml - 32));
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut a = vec![FILL; ml + PAD];
            let mut b = vec![FILL; ml + PAD];
            let mut a2 = vec![FILL; ml + PAD];
            let mut b2 = vec![FILL; ml + PAD];
            let (r1, r2, r3, r4) = unsafe {
                (
                    ec(a.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    er(b.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    dc(a2.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    dr(b2.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            let what = format!("crypto_secretbox_xsalsa20poly1305 [row187] mlen={ml}");
            assert_eq!((r1, r2, r3, r4), (0, 0, 0, 0), "{what}: some return != 0");
            assert_eq_bytes(&what, &a, &b);
            assert_eq_bytes(&format!("{what}: dispatch != primitive"), &a, &a2);
            assert_eq_bytes(&format!("{what}: dispatch != primitive (rust)"), &b, &b2);
            guard_intact(&what, "C", &a, ml);
            guard_intact(&what, "rust", &b, ml);
            let mut p = vec![FILL; ml + PAD];
            let mut q = vec![FILL; ml + PAD];
            let mut p2 = vec![FILL; ml + PAD];
            let mut q2 = vec![FILL; ml + PAD];
            let (s1, s2, s3, s4) = unsafe {
                (
                    oc(p.as_mut_ptr(), a.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    or(q.as_mut_ptr(), a.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    doc(p2.as_mut_ptr(), a.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                    dor(q2.as_mut_ptr(), a.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()),
                )
            };
            assert_eq!((s1, s2, s3, s4), (0, 0, 0, 0), "{what}: open returned != 0");
            assert_eq_bytes("row187 open", &p, &q);
            assert_eq_bytes("row187 open dispatch", &p, &p2);
            assert_eq_bytes("row187 open dispatch (rust)", &q, &q2);
            assert_eq_bytes("row187 round trip", &m, &p[..ml]);
            assert!(p[..32].iter().all(|&x| x == 0), "row187: m[0..32) must be zeroed");
            guard_intact("row187 open", "C", &p, ml);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row187 only drove {iters} inputs");
}

/// Row 188: `crypto_secretbox_open_detached` with `m == NULL` — verify-only.
#[test]
fn r188_secretbox_open_detached_verify_only() {
    init_both();
    let mut rng = Rng::new(SEED ^ 188);
    let mut iters = 0usize;
    for &ml in &[0usize, 1, 15, 16, 31, 32, 33, 64, 65, 200, 1000] {
        for _ in 0..7 {
            let m = rng.bytes(ml);
            let k = rng.bytes(SBS.kb);
            let n = rng.bytes(SBS.nb);
            let (c, mac) = sb_det(&SBS, &m, &n, &k, "row188");
            let (rc, _) = sb_open_det(&SBS, &c, &mac, &n, &k, true, "row188/ok");
            assert_eq!(rc, 0, "row188: verify-only rejected a valid MAC");
            let mut bad = mac.clone();
            bad[rng.below(SBS.mb)] ^= 1 << rng.below(8);
            let (rb, _) = sb_open_det(&SBS, &c, &bad, &n, &k, true, "row188/bad");
            assert_eq!(rb, -1, "row188: verify-only accepted a forged MAC");
            iters += 1;
        }
    }
    assert!(iters >= 64, "row188 only drove {iters} inputs");
}

/// Row 189: `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy`. The
/// detached form calls `chacha20_xor` over exactly `mlen0 + 32` bytes, then ONE
/// `xor_ic(1)` for the rest, and poly1305 over the whole ciphertext with no
/// chunking at all.
#[test]
fn r189_secretbox_xchacha_easy() {
    init_both();
    let mut rng = Rng::new(SEED ^ 189);
    let mut iters = 0usize;
    for &ml in SB_MLEN {
        for _ in 0..10 {
            let m = rng.bytes(ml);
            let k = rng.bytes(SBX.kb);
            let n = rng.bytes(SBX.nb);
            let c = sb_easy(&SBX, &m, &n, &k, "row189");
            let (rc, p) = sb_open_easy(&SBX, &c, &n, &k, "row189");
            assert_eq!(rc, 0, "row189 open_easy failed");
            assert_eq_bytes("row189 round trip", &m, &p[..ml]);
            let (cd, mac) = sb_det(&SBX, &m, &n, &k, "row189/det");
            assert_eq_bytes("row189 easy head is the MAC", &c[..SBX.mb], &mac);
            assert_eq_bytes("row189 easy tail is the ciphertext", &c[SBX.mb..], &cd);
            // the xchacha variant must NOT equal the xsalsa one
            let cs = sb_easy(&SBS, &m, &n, &k, "row189/xsalsa");
            if ml > 0 {
                assert_ne!(c, cs, "row189: xchacha and xsalsa secretbox agree?!");
            }
            let mut bad = c.clone();
            let i = rng.below(bad.len());
            bad[i] ^= 1 << rng.below(8);
            let (rb, pb) = sb_open_easy(&SBX, &bad, &n, &k, "row189/flip");
            assert_eq!(rb, -1, "row189: flipped bit at {i} accepted");
            assert!(pb.iter().all(|&x| x == FILL), "row189: m written on failure");
            iters += 1;
        }
    }
    // long messages: the xchacha variant does NOT chunk, so 131073 exercises a
    // single 131073-byte xor_ic(1) call
    for m in [&big_msgs().0, &big_msgs().1] {
        for _ in 0..4 {
            let k = rng.bytes(SBX.kb);
            let n = rng.bytes(SBX.nb);
            let (c, mac) = sb_det(&SBX, m, &n, &k, "row189/big");
            let (rc, p) = sb_open_det(&SBX, &c, &mac, &n, &k, false, "row189/big");
            assert_eq!(rc, 0);
            assert_eq_bytes("row189 big round trip", m, &p[..m.len()]);
            iters += 1;
        }
    }
    assert!(iters >= 64, "row189 only drove {iters} inputs");
}

/// Row 190: the xchacha20poly1305 overlap axis (same 4 shapes) plus `m == NULL`.
#[test]
fn r190_secretbox_xchacha_detached_overlap() {
    init_both();
    let mut rng = Rng::new(SEED ^ 190);
    let mut iters = 0usize;
    for &ml in &[2usize, 16, 31, 32, 33, 48, 64, 65, 200, 1000] {
        for &shift in &[1usize, ml / 2, ml - 1] {
            if shift == 0 || shift >= ml {
                continue;
            }
            let m = rng.bytes(ml);
            let k = rng.bytes(SBX.kb);
            let n = rng.bytes(SBX.nb);
            let (cref, macref) = sb_det(&SBX, &m, &n, &k, "row190/ref");
            for sh in [Shape::Disjoint, Shape::InPlace, Shape::Forward, Shape::Backward] {
                let (buf, mac, co) = sb_det_shaped(&SBX, &m, &n, &k, sh, shift, "row190");
                assert_eq_bytes(&format!("row190 {sh:?} MAC"), &macref, &mac);
                assert_eq_bytes(&format!("row190 {sh:?} c"), &cref, &buf[co..co + ml]);
                let (rc, p, mo) =
                    sb_open_det_shaped(&SBX, &cref, &macref, &n, &k, sh, shift, "row190/open");
                assert_eq!(rc, 0, "row190 {sh:?}: open_detached failed");
                assert_eq_bytes(&format!("row190 {sh:?} plaintext"), &m, &p[mo..mo + ml]);
                iters += 1;
            }
            // verify-only
            let (rv, _) = sb_open_det(&SBX, &cref, &macref, &n, &k, true, "row190/vo");
            assert_eq!(rv, 0, "row190: verify-only rejected a valid MAC");
            let mut bad = macref.clone();
            bad[0] ^= 0x80;
            let (rf, _) = sb_open_det(&SBX, &cref, &bad, &n, &k, true, "row190/vo-bad");
            assert_eq!(rf, -1, "row190: verify-only accepted a forged MAC");
        }
    }
    assert!(iters >= 64, "row190 only drove {iters} inputs");
}

/// Row 191: `crypto_secretbox*_keygen` + every constant getter + `_primitive`.
#[test]
fn r191_secretbox_keygen_and_constants() {
    let _g = rng_lock();
    init_both();
    for (n, v) in [
        ("crypto_secretbox_keybytes", 32usize),
        ("crypto_secretbox_noncebytes", 24),
        ("crypto_secretbox_macbytes", 16),
        ("crypto_secretbox_zerobytes", 32),
        ("crypto_secretbox_boxzerobytes", 16),
        ("crypto_secretbox_xsalsa20poly1305_keybytes", 32),
        ("crypto_secretbox_xsalsa20poly1305_noncebytes", 24),
        ("crypto_secretbox_xsalsa20poly1305_macbytes", 16),
        ("crypto_secretbox_xsalsa20poly1305_zerobytes", 32),
        ("crypto_secretbox_xsalsa20poly1305_boxzerobytes", 16),
        ("crypto_secretbox_xchacha20poly1305_keybytes", 32),
        ("crypto_secretbox_xchacha20poly1305_noncebytes", 24),
        ("crypto_secretbox_xchacha20poly1305_macbytes", 16),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
    // every MESSAGEBYTES_MAX here is SODIUM_SIZE_MAX - MACBYTES
    for n in [
        "crypto_secretbox_messagebytes_max",
        "crypto_secretbox_xsalsa20poly1305_messagebytes_max",
        "crypto_secretbox_xchacha20poly1305_messagebytes_max",
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c() as u64, r() as u64) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, u64::MAX - 16, "{n}: expected SODIUM_SIZE_MAX - MACBYTES");
    }
    let (c, r) = unsafe { pair::<CharFn>("crypto_secretbox_primitive") };
    let (a, b) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(a, b, "crypto_secretbox_primitive differs");
    assert_eq!(a.to_str().unwrap(), "xsalsa20poly1305");

    install_det_rng(false);
    let mut iters = 0usize;
    for name in ["crypto_secretbox_keygen", "crypto_secretbox_xsalsa20poly1305_keygen"] {
        let (c, r) = unsafe { pair::<KeygenFn>(name) };
        for i in 0..40 {
            reset_det_rng();
            advance_det_rng(i);
            let mut kc = vec![FILL; 32 + PAD];
            let mut kr = vec![FILL; 32 + PAD];
            unsafe {
                c(kc.as_mut_ptr());
                r(kr.as_mut_ptr());
            }
            assert_eq_bytes(&format!("{name} #{i}"), &kc, &kr);
            guard_intact(name, "C", &kc, 32);
            guard_intact(name, "rust", &kr, 32);
            iters += 1;
        }
    }
    // the xchacha20poly1305 secretbox has NO keygen in the C, so neither
    // library may export one
    for n in ["crypto_secretbox_xchacha20poly1305_keygen"] {
        let l = libs();
        let mut nm: Vec<u8> = n.as_bytes().to_vec();
        nm.push(0);
        let gc = unsafe { l.c.get::<KeygenFn>(&nm) }.is_ok();
        let gr = unsafe { l.r.get::<KeygenFn>(&nm) }.is_ok();
        assert_eq!(gc, gr, "{n}: symbol presence differs (C={gc} rust={gr})");
        assert!(!gc, "{n} does not exist in libsodium 1.0.23");
    }
    restore_default_rng();
    assert!(iters >= 64, "row191 only drove {iters} keygen inputs");
}

// ============================================ CONFIGS 192–201: secretstream

const SS: &str = "crypto_secretstream_xchacha20poly1305";
const SS_ABYTES: usize = 17;
const SS_HEADERBYTES: usize = 24;
const SS_KEYBYTES: usize = 32;
const TAG_MESSAGE: u8 = 0x00;
const TAG_PUSH: u8 = 0x01;
const TAG_REKEY: u8 = 0x02;
const TAG_FINAL: u8 = 0x03;

fn ss_statebytes() -> usize {
    static S: OnceLock<usize> = OnceLock::new();
    *S.get_or_init(|| {
        let (c, r) = unsafe { pair::<SizeFn>("crypto_secretstream_xchacha20poly1305_statebytes") };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "crypto_secretstream_xchacha20poly1305_statebytes: C={a} rust={b}");
        assert_eq!(a, 52, "sizeof(state) == k[32] + nonce[12] + _pad[8]");
        a
    })
}

/// The OPAQUE secretstream state, oversized and 0xAA-prefilled, held once per
/// library. The WHOLE buffer (state + trailing guard) is compared after every
/// single operation.
struct SsState {
    c: Vec<u8>,
    r: Vec<u8>,
}
impl SsState {
    fn new() -> Self {
        let sb = ss_statebytes();
        SsState { c: vec![FILL; sb + PAD], r: vec![FILL; sb + PAD] }
    }
    fn check(&self, what: &str) {
        assert_eq_bytes(&format!("{what} [STATE]"), &self.c, &self.r);
        guard_intact(what, "C state", &self.c, ss_statebytes());
        guard_intact(what, "rust state", &self.r, ss_statebytes());
    }
    fn body(&self) -> Vec<u8> {
        self.c[..ss_statebytes()].to_vec()
    }
    /// `state->k` (bytes 0..32).
    fn k(&self) -> Vec<u8> {
        self.c[..32].to_vec()
    }
    /// `state->nonce` (bytes 32..44) = COUNTER[4] || INONCE[8].
    fn nonce(&self) -> Vec<u8> {
        self.c[32..44].to_vec()
    }
}

fn ss_init_push(st: &mut SsState, k: &[u8], what: &str) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<SsInitPush>(&format!("{SS}_init_push")) };
    let mut hc = vec![FILL; SS_HEADERBYTES + PAD];
    let mut hr = vec![FILL; SS_HEADERBYTES + PAD];
    let rc = unsafe { fc(st.c.as_mut_ptr(), hc.as_mut_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(st.r.as_mut_ptr(), hr.as_mut_ptr(), k.as_ptr()) };
    assert_eq!(rc, rr, "{what} init_push: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{what} init_push must return 0");
    assert_eq_bytes(&format!("{what} init_push header"), &hc, &hr);
    guard_intact(what, "C header", &hc, SS_HEADERBYTES);
    guard_intact(what, "rust header", &hr, SS_HEADERBYTES);
    st.check(&format!("{what} init_push"));
    hc.truncate(SS_HEADERBYTES);
    hc
}

fn ss_init_pull(st: &mut SsState, header: &[u8], k: &[u8], what: &str) -> c_int {
    let (fc, fr) = unsafe { pair::<SsInitPull>(&format!("{SS}_init_pull")) };
    let rc = unsafe { fc(st.c.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    let rr = unsafe { fr(st.r.as_mut_ptr(), header.as_ptr(), k.as_ptr()) };
    assert_eq!(rc, rr, "{what} init_pull: return differs (C={rc} rust={rr})");
    st.check(&format!("{what} init_pull"));
    rc
}

fn ss_rekey(st: &mut SsState, what: &str) {
    let (fc, fr) = unsafe { pair::<SsRekey>(&format!("{SS}_rekey")) };
    unsafe {
        fc(st.c.as_mut_ptr());
        fr(st.r.as_mut_ptr());
    }
    st.check(&format!("{what} rekey"));
}

/// `_push`. Returns the `mlen + 17` output.
fn ss_push(
    st: &mut SsState,
    m: &[u8],
    ad: Option<&[u8]>,
    tag: u8,
    outlen_p: bool,
    what: &str,
) -> Vec<u8> {
    let (fc, fr) = unsafe { pair::<SsPush>(&format!("{SS}_push")) };
    let mlen = m.len();
    let out = mlen + SS_ABYTES;
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let (adp, adl) = ad_ptr(ad);
    let mut lc = U64SENT;
    let mut lr = U64SENT;
    let (lcp, lrp) = if outlen_p {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(st.c.as_mut_ptr(), bc.as_mut_ptr(), lcp, m.as_ptr(), mlen as u64, adp, adl, tag)
    };
    let rr = unsafe {
        fr(st.r.as_mut_ptr(), br.as_mut_ptr(), lrp, m.as_ptr(), mlen as u64, adp, adl, tag)
    };
    let w = format!(
        "{SS}_push [{what}] mlen={mlen} adlen={adl} ad_null={} tag={tag:#04x} outlen_p={outlen_p}",
        ad.is_none()
    );
    assert_eq!(rc, rr, "{w}: return differs (C={rc} rust={rr})");
    assert_eq!(rc, 0, "{w}: _push must return 0");
    assert_eq_bytes(&w, &bc, &br);
    guard_intact(&w, "C", &bc, out);
    guard_intact(&w, "rust", &br, out);
    assert_eq!(lc, lr, "{w}: *outlen_p differs (C={lc} rust={lr})");
    if outlen_p {
        assert_eq!(lc, out as u64, "{w}: *outlen_p must be ABYTES + mlen");
    } else {
        assert_eq!(lc, U64SENT, "{w}: outlen_p was NULL yet the sentinel moved");
    }
    st.check(&w);
    bc.truncate(out);
    bc
}

/// `_pull`. Returns `(rc, full prefilled plaintext buffer + guard, *mlen_p, *tag_p)`.
#[allow(clippy::too_many_arguments)]
fn ss_pull(
    st: &mut SsState,
    input: &[u8],
    ad: Option<&[u8]>,
    mlen_p: bool,
    tag_p: bool,
    what: &str,
) -> (c_int, Vec<u8>, u64, u8) {
    let (fc, fr) = unsafe { pair::<SsPull>(&format!("{SS}_pull")) };
    let inlen = input.len();
    let out = inlen.saturating_sub(SS_ABYTES);
    let mut bc = vec![FILL; out + PAD];
    let mut br = vec![FILL; out + PAD];
    let (adp, adl) = ad_ptr(ad);
    let mut lc = U64SENT;
    let mut lr = U64SENT;
    let (lcp, lrp) = if mlen_p {
        (&mut lc as *mut u64, &mut lr as *mut u64)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let mut tc: u8 = FILL;
    let mut tr: u8 = FILL;
    let (tcp, trp) = if tag_p {
        (&mut tc as *mut u8, &mut tr as *mut u8)
    } else {
        (ptr::null_mut(), ptr::null_mut())
    };
    let rc = unsafe {
        fc(st.c.as_mut_ptr(), bc.as_mut_ptr(), lcp, tcp, input.as_ptr(), inlen as u64, adp, adl)
    };
    let rr = unsafe {
        fr(st.r.as_mut_ptr(), br.as_mut_ptr(), lrp, trp, input.as_ptr(), inlen as u64, adp, adl)
    };
    let w = format!(
        "{SS}_pull [{what}] inlen={inlen} adlen={adl} ad_null={} mlen_p={mlen_p} tag_p={tag_p}",
        ad.is_none()
    );
    assert_eq!(rc, rr, "{w}: return differs (C={rc} rust={rr})");
    assert_eq_bytes(&w, &bc, &br);
    guard_intact(&w, "C", &bc, out);
    guard_intact(&w, "rust", &br, out);
    assert_eq!(lc, lr, "{w}: *mlen_p differs (C={lc} rust={lr})");
    assert_eq!(tc, tr, "{w}: *tag_p differs (C={tc:#04x} rust={tr:#04x})");
    if !mlen_p {
        assert_eq!(lc, U64SENT, "{w}: mlen_p was NULL yet the sentinel moved");
    }
    if !tag_p {
        assert_eq!(tc, FILL, "{w}: tag_p was NULL yet the sentinel moved");
    }
    st.check(&w);
    (rc, bc, lc, tc)
}

/// One `_push` + `_pull` step with the full set of cross-checks: the pull side
/// must recover the plaintext, report the tag, and end up in EXACTLY the same
/// state as the push side (secretstream is symmetric).
fn ss_step(
    push: &mut SsState,
    pull: &mut SsState,
    m: &[u8],
    ad: Option<&[u8]>,
    tag: u8,
    what: &str,
) -> Vec<u8> {
    let ct = ss_push(push, m, ad, tag, true, what);
    let (rc, p, ml, tg) = ss_pull(pull, &ct, ad, true, true, what);
    assert_eq!(rc, 0, "{what}: _pull rejected a freshly pushed message");
    assert_eq!(ml, m.len() as u64, "{what}: *mlen_p is wrong");
    assert_eq!(tg, tag, "{what}: *tag_p is {tg:#04x}, pushed {tag:#04x}");
    assert_eq_bytes(&format!("{what}: plaintext"), m, &p[..m.len()]);
    assert_eq_bytes(
        &format!("{what}: push and pull states diverged"),
        &push.body(),
        &pull.body(),
    );
    ct
}

/// Drive one whole tag over the CONFIGS-192 `mlen`/`adlen` cross-product.
/// `mlen == 48` is the value that makes the quirky pad `(0x10-64+mlen)&0xf` zero.
const SS_MLEN: &[usize] = &[0, 1, 15, 16, 48, 63, 64, 65];
const SS_ADLEN: &[usize] = &[0, 1, 15, 16, 17];

fn ss_tag_sweep(tag: u8, seed: u64, label: &str) -> usize {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for &ml in SS_MLEN {
        for &al in SS_ADLEN {
            for _ in 0..2 {
                let k = rng.bytes(SS_KEYBYTES);
                let header = rng.bytes(SS_HEADERBYTES);
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let mut push = SsState::new();
                let mut pull = SsState::new();
                assert_eq!(ss_init_pull(&mut push, &header, &k, label), 0);
                assert_eq!(ss_init_pull(&mut pull, &header, &k, label), 0);
                let before = push.body();
                ss_step(&mut push, &mut pull, &m, Some(&ad), tag, label);
                let after = push.body();
                assert_ne!(before, after, "{label}: _push did not advance the state");
                // whether the state was rekeyed is fully determined by bit 0x02
                let rekeyed = after[..32] != before[..32];
                let want = (tag & TAG_REKEY) != 0;
                assert_eq!(
                    rekeyed, want,
                    "{label}: tag={tag:#04x} rekey expected {want}, observed {rekeyed} \
                     (k before={} after={})",
                    hexs(&before[..32]),
                    hexs(&after[..32])
                );
                // after a rekey the counter is reset to 1; otherwise it is 2
                let ctr = u32::from_le_bytes([after[32], after[33], after[34], after[35]]);
                assert_eq!(
                    ctr,
                    if want { 1 } else { 2 },
                    "{label}: counter after one push with tag={tag:#04x}"
                );
                iters += 1;
            }
        }
    }
    iters
}

/// Row 192: TAG_MESSAGE (0x00). `mlen == 0` produces a 17-byte frame.
#[test]
fn r192_ss_tag_message() {
    init_both();
    let iters = ss_tag_sweep(TAG_MESSAGE, SEED ^ 192, "row192");
    assert!(iters >= 64, "row192 only drove {iters} inputs");
    // mlen == 0 => exactly ABYTES out
    let mut rng = Rng::new(SEED ^ 0x192);
    let k = rng.bytes(SS_KEYBYTES);
    let header = rng.bytes(SS_HEADERBYTES);
    let mut push = SsState::new();
    ss_init_pull(&mut push, &header, &k, "row192/zero");
    let ct = ss_push(&mut push, &[], None, TAG_MESSAGE, true, "row192/zero");
    assert_eq!(ct.len(), 17, "row192: mlen==0 must give a 17-byte frame");
    // and `_init_push` produces the very same state as `_init_pull` on the
    // header it emitted
    let _g = rng_lock();
    install_det_rng(false);
    reset_det_rng();
    let mut a = SsState::new();
    let h = ss_init_push(&mut a, &k, "row192/init_push");
    let mut b = SsState::new();
    ss_init_pull(&mut b, &h, &k, "row192/init_pull");
    assert_eq_bytes("row192 init_push state == init_pull(header) state", &a.body(), &b.body());
    restore_default_rng();
}

/// Row 193: TAG_PUSH (0x01) — bit 0x02 is clear, so NO automatic rekey.
#[test]
fn r193_ss_tag_push() {
    init_both();
    let iters = ss_tag_sweep(TAG_PUSH, SEED ^ 193, "row193");
    assert!(iters >= 64, "row193 only drove {iters} inputs");
}

/// Row 194: TAG_REKEY (0x02) — triggers the automatic rekey.
#[test]
fn r194_ss_tag_rekey() {
    init_both();
    let iters = ss_tag_sweep(TAG_REKEY, SEED ^ 194, "row194");
    assert!(iters >= 64, "row194 only drove {iters} inputs");
}

/// Row 195: TAG_FINAL (0x03 == PUSH|REKEY) — also rekeys.
#[test]
fn r195_ss_tag_final() {
    init_both();
    let iters = ss_tag_sweep(TAG_FINAL, SEED ^ 195, "row195");
    assert!(iters >= 64, "row195 only drove {iters} inputs");
}

/// Row 196 + ERRORS 130: ARBITRARY tag bytes are ACCEPTED with no validation
/// whatsoever; only bit 0x02 makes the state rekey.
#[test]
fn r196_ss_arbitrary_tags() {
    init_both();
    let mut rng = Rng::new(SEED ^ 196);
    let mut iters = 0usize;
    for &tag in &[0x04u8, 0x42, 0x7f, 0xff] {
        for &ml in SS_MLEN {
            for _ in 0..2 {
                let k = rng.bytes(SS_KEYBYTES);
                let header = rng.bytes(SS_HEADERBYTES);
                let m = rng.bytes(ml);
                let ad = rnd(&mut rng, 20);
                let mut push = SsState::new();
                let mut pull = SsState::new();
                ss_init_pull(&mut push, &header, &k, "row196");
                ss_init_pull(&mut pull, &header, &k, "row196");
                let before = push.body();
                ss_step(&mut push, &mut pull, &m, Some(&ad), tag, "row196");
                let after = push.body();
                let rekeyed = after[..32] != before[..32];
                assert_eq!(
                    rekeyed,
                    (tag & TAG_REKEY) != 0,
                    "row196: tag={tag:#04x} — only bit 0x02 may trigger a rekey"
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row196 only drove {iters} inputs");
}

/// Row 197: the explicit symmetric `_rekey` mid-stream. Applying it on both
/// sides keeps the two states in lockstep; applying it on ONE side breaks the
/// stream (the next `_pull` fails), which is what makes it observable.
#[test]
fn r197_ss_explicit_rekey() {
    init_both();
    let mut rng = Rng::new(SEED ^ 197);
    let mut iters = 0usize;
    for &ml in SS_MLEN {
        for _ in 0..9 {
            let k = rng.bytes(SS_KEYBYTES);
            let header = rng.bytes(SS_HEADERBYTES);
            let mut push = SsState::new();
            let mut pull = SsState::new();
            ss_init_pull(&mut push, &header, &k, "row197");
            ss_init_pull(&mut pull, &header, &k, "row197");
            let m0 = rng.bytes(ml);
            ss_step(&mut push, &mut pull, &m0, None, TAG_MESSAGE, "row197/pre");
            let pre = push.body();
            ss_rekey(&mut push, "row197");
            ss_rekey(&mut pull, "row197");
            let post = push.body();
            assert_ne!(pre, post, "row197: _rekey did not change the state");
            assert_eq_bytes("row197: _rekey must be symmetric", &push.body(), &pull.body());
            // the counter is reset to 1
            assert_eq!(
                u32::from_le_bytes([post[32], post[33], post[34], post[35]]),
                1,
                "row197: _rekey must reset the counter to 1"
            );
            let m1 = rng.bytes(ml);
            ss_step(&mut push, &mut pull, &m1, None, TAG_MESSAGE, "row197/post");
            // rekeying only ONE side desynchronises the stream
            ss_rekey(&mut push, "row197/one-sided");
            let ct = ss_push(&mut push, &m1, None, TAG_MESSAGE, true, "row197/one-sided");
            let (rc, p, ml2, tg) = ss_pull(&mut pull, &ct, None, true, true, "row197/one-sided");
            assert_eq!(rc, -1, "row197: a one-sided _rekey must break the stream");
            assert_eq!(ml2, 0, "row197: *mlen_p must stay 0 on failure");
            assert_eq!(tg, 0xff, "row197: *tag_p must stay 0xff on failure");
            assert!(
                p.iter().all(|&x| x == FILL),
                "row197: _pull must not write m on a MAC failure"
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "row197 only drove {iters} inputs");
}

/// Row 198: an 8+ message sequence mixing all four tags. Every ciphertext and
/// the FULL state after every step are compared, and because `_push` folds the
/// MAC into `STATE_INONCE` (`STATE_INONCE ^= mac[0..8)`) the ORDER of the
/// messages is part of the state — replaying a frame out of order must fail.
#[test]
fn r198_ss_multi_message_sequence() {
    init_both();
    let mut rng = Rng::new(SEED ^ 198);
    let tags = [
        TAG_MESSAGE, TAG_PUSH, TAG_MESSAGE, TAG_REKEY, TAG_MESSAGE, TAG_PUSH, TAG_FINAL,
        TAG_MESSAGE, TAG_REKEY, TAG_FINAL,
    ];
    let mut iters = 0usize;
    for round in 0..8 {
        let k = rng.bytes(SS_KEYBYTES);
        let header = rng.bytes(SS_HEADERBYTES);
        let mut push = SsState::new();
        let mut pull = SsState::new();
        ss_init_pull(&mut push, &header, &k, "row198");
        ss_init_pull(&mut pull, &header, &k, "row198");
        let mut frames = vec![];
        let mut states = vec![push.body()];
        for (i, &tag) in tags.iter().enumerate() {
            let m = rnd(&mut rng, 200);
            let ad = rnd(&mut rng, 30);
            let what = format!("row198/round{round}/msg{i}/tag{tag:#04x}");
            let ct = ss_step(&mut push, &mut pull, &m, Some(&ad), tag, &what);
            // no state may repeat: the MAC chaining makes every step unique
            let now = push.body();
            assert!(
                !states.contains(&now),
                "{what}: the state repeated — MAC chaining is broken"
            );
            states.push(now);
            frames.push((m, ad, ct, tag));
            iters += 1;
        }
        assert!(frames.len() >= 8, "row198: fewer than 8 messages");
        // replaying an earlier frame into the (now advanced) pull state fails
        let (_, ad0, ct0, _) = &frames[0];
        let (rc, p, ml, tg) = ss_pull(&mut pull, ct0, Some(ad0), true, true, "row198/replay");
        assert_eq!(rc, -1, "row198: a replayed frame was ACCEPTED");
        assert_eq!(ml, 0, "row198: *mlen_p must stay 0");
        assert_eq!(tg, 0xff, "row198: *tag_p must stay 0xff");
        assert!(p.iter().all(|&x| x == FILL), "row198: m written on failure");
        // ... and a fresh, correctly ordered message still works afterwards
        let m = rnd(&mut rng, 100);
        ss_step(&mut push, &mut pull, &m, None, TAG_MESSAGE, "row198/after-replay");
        iters += 1;
    }
    assert!(iters >= 64, "row198 only drove {iters} inputs");
}

/// Reference re-implementation of the `_push` poly1305 input, built ONLY from
/// dlsym'd primitives of the requested library. `padlen` is passed explicitly so
/// the CONFIGS-199 quirk can be pinned against the "intended" alternative.
fn ss_ref_push(
    which: usize,
    sk: &[u8],
    nonce: &[u8],
    tag: u8,
    m: &[u8],
    ad: &[u8],
    padlen: u64,
) -> ([u8; 16], Vec<u8>) {
    let l = lib_of(which);
    let ietf = unsafe { sym::<StreamFn>(l, "crypto_stream_chacha20_ietf") };
    let xic = unsafe { sym::<XorIc32Fn>(l, "crypto_stream_chacha20_ietf_xor_ic") };
    let pi = unsafe { sym::<P1Init>(l, "crypto_onetimeauth_poly1305_init") };
    let pu = unsafe { sym::<P1Upd>(l, "crypto_onetimeauth_poly1305_update") };
    let pf = unsafe { sym::<P1Fin>(l, "crypto_onetimeauth_poly1305_final") };
    let mut st = Aligned16::new(512, 0);
    let mut block = [0u8; 64];
    let pad0 = [0u8; 16];
    let mut mac = [0u8; 16];
    let mut ct = vec![0u8; m.len()];
    unsafe {
        ietf(block.as_mut_ptr(), 64, nonce.as_ptr(), sk.as_ptr());
        pi(st.p(), block.as_ptr());
        pu(st.p(), ad.as_ptr(), ad.len() as u64);
        pu(st.p(), pad0.as_ptr(), (0x10u64.wrapping_sub(ad.len() as u64)) & 0xf);
        block = [0u8; 64];
        block[0] = tag;
        xic(block.as_mut_ptr(), block.as_ptr(), 64, nonce.as_ptr(), 1, sk.as_ptr());
        pu(st.p(), block.as_ptr(), 64);
        xic(ct.as_mut_ptr(), m.as_ptr(), m.len() as u64, nonce.as_ptr(), 2, sk.as_ptr());
        pu(st.p(), ct.as_ptr(), m.len() as u64);
        pu(st.p(), pad0.as_ptr(), padlen);
        let mut slen = (ad.len() as u64).to_le_bytes();
        pu(st.p(), slen.as_ptr(), 8);
        slen = (64u64 + m.len() as u64).to_le_bytes();
        pu(st.p(), slen.as_ptr(), 8);
        pf(st.p(), mac.as_mut_ptr());
    }
    (mac, ct)
}

/// Row 199: the poly1305 pad after the ciphertext is `(0x10 - 64 + mlen) & 0xf`
/// (i.e. `mlen & 0xf`), NOT the intended `(0x10 - (64 + mlen)) & 0xf`
/// (i.e. `(0x10 - mlen) & 0xf`). The two differ whenever `mlen % 16` is neither
/// 0 nor 8, and the difference is WIRE-FORMAT VISIBLE in the MAC. `mlen == 48`
/// makes the quirky pad 0.
#[test]
fn r199_ss_pad_quirk() {
    init_both();
    let mut rng = Rng::new(SEED ^ 199);
    let mut iters = 0usize;
    let mut differed = 0usize;
    for &ml in SS_MLEN {
        let quirk = (0x10u64.wrapping_sub(64).wrapping_add(ml as u64)) & 0xf;
        let intended = (0x10u64.wrapping_sub(64 + ml as u64)) & 0xf;
        assert_eq!(quirk, (ml as u64) & 0xf, "row199: the quirky pad is mlen & 0xf");
        assert_eq!(
            intended,
            (0x10u64.wrapping_sub(ml as u64)) & 0xf,
            "row199: the intended pad is (0x10 - mlen) & 0xf"
        );
        if ml == 48 {
            assert_eq!(quirk, 0, "row199: mlen==48 must make the quirky pad 0");
        }
        for &tag in &[TAG_MESSAGE, TAG_PUSH, TAG_REKEY, TAG_FINAL] {
            for &al in SS_ADLEN {
                let k = rng.bytes(SS_KEYBYTES);
                let header = rng.bytes(SS_HEADERBYTES);
                let m = rng.bytes(ml);
                let ad = rng.bytes(al);
                let mut push = SsState::new();
                ss_init_pull(&mut push, &header, &k, "row199");
                let sk = push.k();
                let nonce = push.nonce();
                let ct = ss_push(&mut push, &m, Some(&ad), tag, true, "row199");

                for which in 0..2 {
                    let (mac_q, c_q) = ss_ref_push(which, &sk, &nonce, tag, &m, &ad, quirk);
                    let lib = if which == 0 { "C" } else { "rust" };
                    assert_eq_bytes(
                        &format!("row199 [{lib}] mlen={ml} tag={tag:#04x}: ciphertext"),
                        &c_q,
                        &ct[1..1 + ml],
                    );
                    assert_eq_bytes(
                        &format!(
                            "row199 [{lib}] mlen={ml} adlen={al} tag={tag:#04x}: the MAC does NOT \
                             match the QUIRKY pad {quirk}"
                        ),
                        &mac_q,
                        &ct[1 + ml..],
                    );
                    if quirk != intended {
                        let (mac_i, _) = ss_ref_push(which, &sk, &nonce, tag, &m, &ad, intended);
                        assert_ne!(
                            mac_i.as_slice(),
                            &ct[1 + ml..],
                            "row199 [{lib}] mlen={ml}: the MAC matches the INTENDED pad \
                             {intended} — the quirk is gone"
                        );
                    }
                }
                if quirk != intended {
                    differed += 1;
                }
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row199 only drove {iters} inputs");
    assert!(
        differed >= 16,
        "row199: only {differed} inputs actually distinguished the two pad forms"
    );
}

/// Row 200: `_pull` with `mlen_p == NULL` and/or `tag_p == NULL`.
#[test]
fn r200_ss_pull_null_outparams() {
    init_both();
    let mut rng = Rng::new(SEED ^ 200);
    let mut iters = 0usize;
    for &ml in SS_MLEN {
        for &tag in &[TAG_MESSAGE, TAG_REKEY] {
            for &(mp, tp) in &[(true, true), (true, false), (false, true), (false, false)] {
                let k = rng.bytes(SS_KEYBYTES);
                let header = rng.bytes(SS_HEADERBYTES);
                let m = rng.bytes(ml);
                let ad = rnd(&mut rng, 20);
                let mut push = SsState::new();
                let mut pull = SsState::new();
                ss_init_pull(&mut push, &header, &k, "row200");
                ss_init_pull(&mut pull, &header, &k, "row200");
                // outlen_p == NULL on the push side too
                let ct = ss_push(&mut push, &m, Some(&ad), tag, mp, "row200");
                let (rc, p, mlp, tgp) = ss_pull(&mut pull, &ct, Some(&ad), mp, tp, "row200");
                assert_eq!(rc, 0, "row200: _pull failed with mlen_p={mp} tag_p={tp}");
                assert_eq_bytes("row200 plaintext", &m, &p[..ml]);
                if mp {
                    assert_eq!(mlp, ml as u64, "row200: *mlen_p wrong");
                }
                if tp {
                    assert_eq!(tgp, tag, "row200: *tag_p wrong");
                }
                assert_eq_bytes("row200 states", &push.body(), &pull.body());
                // a short frame with NULL out-params must still just return -1
                let (rs, _, _, _) = ss_pull(&mut pull, &ct[..SS_ABYTES - 1], None, mp, tp, "row200/short");
                assert_eq!(rs, -1, "row200: a short frame must be rejected");
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "row200 only drove {iters} inputs");
}

/// Row 201: `_keygen` + `_statebytes` / `_abytes` / `_headerbytes` /
/// `_keybytes` / `_messagebytes_max` and the four `_tag_*` getters.
#[test]
fn r201_ss_keygen_and_constants() {
    let _g = rng_lock();
    init_both();
    for (n, v) in [
        ("crypto_secretstream_xchacha20poly1305_abytes", 17usize),
        ("crypto_secretstream_xchacha20poly1305_headerbytes", 24),
        ("crypto_secretstream_xchacha20poly1305_keybytes", 32),
        ("crypto_secretstream_xchacha20poly1305_statebytes", 52),
    ] {
        let (c, r) = unsafe { pair::<SizeFn>(n) };
        let (a, b) = unsafe { (c(), r()) };
        assert_eq!(a, b, "{n}: C={a} rust={b}");
        assert_eq!(a, v, "{n}: expected {v}");
    }
    let (c, r) = unsafe { pair::<SizeFn>("crypto_secretstream_xchacha20poly1305_messagebytes_max") };
    let (a, b) = unsafe { (c() as u64, r() as u64) };
    assert_eq!(a, b, "messagebytes_max: C={a} rust={b}");
    assert_eq!(a, 64 * ((1u64 << 32) - 2), "min(SIZE_MAX-17, 64*(2^32-2))");
    for (n, v) in [
        ("crypto_secretstream_xchacha20poly1305_tag_message", 0x00u8),
        ("crypto_secretstream_xchacha20poly1305_tag_push", 0x01),
        ("crypto_secretstream_xchacha20poly1305_tag_rekey", 0x02),
        ("crypto_secretstream_xchacha20poly1305_tag_final", 0x03),
    ] {
        let (c, r) = unsafe { pair::<U8Fn>(n) };
        let (x, y) = unsafe { (c(), r()) };
        assert_eq!(x, y, "{n}: C={x:#04x} rust={y:#04x}");
        assert_eq!(x, v, "{n}: expected {v:#04x}");
    }

    install_det_rng(false);
    let (kc, kr) = unsafe { pair::<KeygenFn>("crypto_secretstream_xchacha20poly1305_keygen") };
    let mut iters = 0usize;
    for i in 0..48 {
        reset_det_rng();
        advance_det_rng(i);
        let mut a = vec![FILL; SS_KEYBYTES + PAD];
        let mut b = vec![FILL; SS_KEYBYTES + PAD];
        unsafe {
            kc(a.as_mut_ptr());
            kr(b.as_mut_ptr());
        }
        assert_eq_bytes(&format!("secretstream keygen #{i}"), &a, &b);
        guard_intact("secretstream keygen", "C", &a, SS_KEYBYTES);
        guard_intact("secretstream keygen", "rust", &b, SS_KEYBYTES);
        iters += 1;
    }
    // `_init_push` is the other RNG consumer: its 24-byte header and the whole
    // resulting state must match byte for byte.
    for i in 0..24 {
        reset_det_rng();
        advance_det_rng(i);
        let mut st = SsState::new();
        let k = vec![(i as u8).wrapping_mul(37); SS_KEYBYTES];
        let h = ss_init_push(&mut st, &k, "row201/init_push");
        assert_eq!(h.len(), SS_HEADERBYTES);
        iters += 1;
    }
    restore_default_rng();
    assert!(iters >= 64, "row201 only drove {iters} inputs");
}

// ==================================================== ERRORS 84–130

const CP_MAX: u64 = u64::MAX - 16;
const CPI_MAX: u64 = 64 * ((1u64 << 32) - 1); // 274877906880
const AEGIS_MAX: u64 = (1u64 << 61) - 1;
const SB_MAX: u64 = u64::MAX - 16;
const SS_MAX: u64 = 64 * ((1u64 << 32) - 2); // 274877906816

/// A writable region followed by an inaccessible guard page, so a call that
/// runs PAST its bound faults immediately and deterministically instead of
/// wandering through the heap.
fn guarded_scratch(n: usize) -> *mut u8 {
    unsafe {
        let total = n + 4096;
        let p = libc::mmap(
            ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert!(p != libc::MAP_FAILED, "mmap failed");
        let r = libc::mprotect((p as *mut u8).add(n) as *mut libc::c_void, 4096, libc::PROT_NONE);
        assert_eq!(r, 0, "mprotect PROT_NONE failed");
        p as *mut u8
    }
}

/// Assert that `messagebytes_max()` really is the value the misuse rows assume.
fn assert_max(name: &str, want: u64) {
    let (c, r) = unsafe { pair::<SizeFn>(name) };
    let (a, b) = unsafe { (c() as u64, r() as u64) };
    assert_eq!(a, b, "{name}: C={a} rust={b}");
    assert_eq!(a, want, "{name}: expected {want}");
}

/// Row 84: `crypto_aead_chacha20poly1305_encrypt` with
/// `mlen > SODIUM_SIZE_MAX - 16` calls `sodium_misuse()`.
///
/// The `mlen == MESSAGEBYTES_MAX` (no-misuse) side of the boundary is NOT
/// driven: it would require the callee to compute `c + (2^64 - 17)`, whose byte
/// offset exceeds `isize::MAX`, which a debug-profile Rust build rejects with
/// its own `ptr::add` precondition abort. That is undefined behaviour in the C
/// too and only reachable with a 16-exabyte buffer, so the row is pinned by the
/// misuse side plus the exact `messagebytes_max()` value.
#[test]
fn e84_cp_encrypt_mlen_misuse() {
    init_both();
    assert_max("crypto_aead_chacha20poly1305_messagebytes_max", CP_MAX);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x42u8; 32];
    let n = [0x24u8; 8];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    for mlen in [CP_MAX + 1, CP_MAX + 7, u64::MAX] {
        expect_outcome::<AeadEnc, _>(
            &format!("ERRORS 84 crypto_aead_chacha20poly1305_encrypt(mlen={mlen}) must misuse"),
            "crypto_aead_chacha20poly1305_encrypt",
            move |f| unsafe { f(p, ptr::null_mut(), p, mlen, ptr::null(), 0, ptr::null(), np, kp) as i64 },
            MISUSE,
        );
    }
    // an in-range call on the same arguments still returns 0 in both libraries
    let mut rng = Rng::new(SEED ^ 84);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let npub = rng.bytes(8);
        enc(&CP, &m, None, &npub, &k, false, true, "e84/legal");
    }
}

/// Row 85: `crypto_aead_chacha20poly1305_decrypt` with `clen < ABYTES` returns
/// -1 and sets `*mlen_p = 0` without touching `m`.
#[test]
fn e85_cp_decrypt_clen_too_short() {
    init_both();
    let mut rng = Rng::new(SEED ^ 85);
    let mut iters = 0usize;
    for clen in 0..CP.ab {
        for _ in 0..5 {
            let c = rng.bytes(clen);
            let k = rng.bytes(CP.kb);
            let npub = rng.bytes(CP.nb);
            let ad = rnd(&mut rng, 20);
            let (rc, buf) = dec(&CP, &c, Some(&ad), &npub, &k, false, true, "e85");
            assert_eq!(rc, -1, "ERRORS 85: clen={clen} < 16 must return -1");
            assert!(buf.is_empty(), "e85: no plaintext may be produced");
            // mlen_p == NULL variant
            let (rc2, _) = dec(&CP, &c, Some(&ad), &npub, &k, false, false, "e85/null");
            assert_eq!(rc2, -1);
            iters += 1;
        }
    }
    // exactly ABYTES is legal (it decrypts a zero-length message)
    for _ in 0..8 {
        let k = rng.bytes(CP.kb);
        let npub = rng.bytes(CP.nb);
        let c = enc(&CP, &[], None, &npub, &k, false, true, "e85/boundary");
        assert_eq!(c.len(), CP.ab);
        let (rc, _) = dec(&CP, &c, None, &npub, &k, false, true, "e85/boundary");
        assert_eq!(rc, 0, "e85: clen == ABYTES must be accepted");
        iters += 1;
    }
    assert!(iters >= 64, "e85 only drove {iters} inputs");
}

/// Rows 86 + 87: `_decrypt_detached` on a `crypto_verify_16` mismatch —
/// `memset(m, 0, clen)` then -1 when `m != NULL`, and NO zeroing at all when
/// `m == NULL`.
#[test]
fn e86_e87_cp_decrypt_detached_forged() {
    init_both();
    forged_detached_rows(&CP, SEED ^ 86, "e86/e87");
}

/// Shared body for ERRORS 86/87, 90/91, 94/95, 100, 105.
fn forged_detached_rows(a: &Aead, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for &ml in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 200, 1000] {
        for _ in 0..5 {
            let m = rng.bytes(ml);
            let ad = rnd(&mut rng, 40);
            let k = rng.bytes(a.kb);
            let npub = rng.bytes(a.nb);
            let (c, mac) = enc_det(a, &m, Some(&ad), &npub, &k, false, true, label);
            let mut bad = mac.clone();
            bad[rng.below(a.ab)] ^= 1 << rng.below(8);
            // m != NULL: the FULL clen bytes are zeroed and the guard survives
            let (rc, buf) = dec_det(a, &c, &bad, Some(&ad), &npub, &k, false, false, label);
            assert_eq!(rc, -1, "{label}: a forged MAC was accepted");
            assert_eq!(
                &buf[..ml],
                vec![0u8; ml].as_slice(),
                "{label}: m must be memset(0, clen) on a MAC mismatch, got {}",
                hexs(&buf[..ml])
            );
            guard_intact(label, "C", &buf, ml);
            // m == NULL: no zeroing (nothing is written anywhere)
            let (rc2, buf2) = dec_det(a, &c, &bad, Some(&ad), &npub, &k, false, true, label);
            assert_eq!(rc2, -1, "{label}: verify-only accepted a forged MAC");
            assert!(
                buf2.iter().all(|&x| x == FILL),
                "{label}: m == NULL must not zero anything"
            );
            // a wrong AD is also a MAC mismatch
            let mut bad_ad = ad.clone();
            if !bad_ad.is_empty() {
                bad_ad[0] ^= 1;
                let (rc3, buf3) =
                    dec_det(a, &c, &mac, Some(&bad_ad), &npub, &k, false, false, label);
                assert_eq!(rc3, -1, "{label}: a wrong AD was accepted");
                assert!(buf3[..ml].iter().all(|&x| x == 0), "{label}: m not zeroed (wrong AD)");
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "{label} only drove {iters} inputs");
}

/// Row 88: the IETF bound is `64 * (2^32 - 1)`; one byte over misuses and the
/// bound itself does NOT (the call then runs on and faults on the guard page).
#[test]
fn e88_cpi_encrypt_mlen_misuse() {
    init_both();
    assert_max("crypto_aead_chacha20poly1305_ietf_messagebytes_max", CPI_MAX);
    let p = guarded_scratch(1 << 16);
    let k = [0x11u8; 32];
    let n = [0x22u8; 12];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<AeadEnc, _>(
        &format!("ERRORS 88 ietf_encrypt(mlen={}) must misuse", CPI_MAX + 1),
        "crypto_aead_chacha20poly1305_ietf_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, CPI_MAX + 1, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        MISUSE,
    );
    expect_outcome::<AeadEnc, _>(
        &format!("ERRORS 88 ietf_encrypt(mlen={CPI_MAX}) is exactly AT the bound: no misuse"),
        "crypto_aead_chacha20poly1305_ietf_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, CPI_MAX, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        NO_MISUSE,
    );
    let mut rng = Rng::new(SEED ^ 88);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let npub = rng.bytes(12);
        enc(&CPI, &m, None, &npub, &k, false, true, "e88/legal");
    }
}

/// Row 89: IETF `_decrypt` with `clen < 16`.
#[test]
fn e89_cpi_decrypt_clen_too_short() {
    init_both();
    let mut rng = Rng::new(SEED ^ 89);
    let mut iters = 0usize;
    for clen in 0..CPI.ab {
        for _ in 0..5 {
            let c = rng.bytes(clen);
            let k = rng.bytes(CPI.kb);
            let npub = rng.bytes(CPI.nb);
            let (rc, buf) = dec(&CPI, &c, None, &npub, &k, false, true, "e89");
            assert_eq!(rc, -1, "ERRORS 89: clen={clen} must return -1");
            assert!(buf.is_empty());
            iters += 1;
        }
    }
    for _ in 0..8 {
        let k = rng.bytes(CPI.kb);
        let npub = rng.bytes(CPI.nb);
        let c = enc(&CPI, &[], None, &npub, &k, false, true, "e89/boundary");
        let (rc, _) = dec(&CPI, &c, None, &npub, &k, false, true, "e89/boundary");
        assert_eq!(rc, 0);
        iters += 1;
    }
    assert!(iters >= 64, "e89 only drove {iters} inputs");
}

/// Rows 90 + 91.
#[test]
fn e90_e91_cpi_decrypt_detached_forged() {
    init_both();
    forged_detached_rows(&CPI, SEED ^ 90, "e90/e91");
}

/// Row 92: the XChaCha bound is `SODIUM_SIZE_MAX - 16`; see `e84` for why only
/// the misuse side of the boundary is driven.
#[test]
fn e92_xcpi_encrypt_mlen_misuse() {
    init_both();
    assert_max("crypto_aead_xchacha20poly1305_ietf_messagebytes_max", CP_MAX);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x33u8; 32];
    let n = [0x44u8; 24];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    for mlen in [CP_MAX + 1, u64::MAX] {
        expect_outcome::<AeadEnc, _>(
            &format!("ERRORS 92 xchacha20poly1305_ietf_encrypt(mlen={mlen}) must misuse"),
            "crypto_aead_xchacha20poly1305_ietf_encrypt",
            move |f| unsafe {
                f(p, ptr::null_mut(), p, mlen, ptr::null(), 0, ptr::null(), np, kp) as i64
            },
            MISUSE,
        );
    }
    let mut rng = Rng::new(SEED ^ 92);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let npub = rng.bytes(24);
        enc(&XCPI, &m, None, &npub, &k, false, true, "e92/legal");
    }
}

/// Row 93.
#[test]
fn e93_xcpi_decrypt_clen_too_short() {
    init_both();
    let mut rng = Rng::new(SEED ^ 93);
    let mut iters = 0usize;
    for clen in 0..XCPI.ab {
        for _ in 0..5 {
            let c = rng.bytes(clen);
            let k = rng.bytes(XCPI.kb);
            let npub = rng.bytes(XCPI.nb);
            let (rc, buf) = dec(&XCPI, &c, None, &npub, &k, false, true, "e93");
            assert_eq!(rc, -1, "ERRORS 93: clen={clen} must return -1");
            assert!(buf.is_empty());
            iters += 1;
        }
    }
    for _ in 0..8 {
        let k = rng.bytes(XCPI.kb);
        let npub = rng.bytes(XCPI.nb);
        let c = enc(&XCPI, &[], None, &npub, &k, false, true, "e93/boundary");
        let (rc, _) = dec(&XCPI, &c, None, &npub, &k, false, true, "e93/boundary");
        assert_eq!(rc, 0);
        iters += 1;
    }
    assert!(iters >= 64, "e93 only drove {iters} inputs");
}

/// Rows 94 + 95.
#[test]
fn e94_e95_xcpi_decrypt_detached_forged() {
    init_both();
    forged_detached_rows(&XCPI, SEED ^ 94, "e94/e95");
}

/// Row 96: `crypto_aead_aegis128l_encrypt` with `mlen > min(SIZE_MAX-32, 2^61-1)`.
/// Both sides of the boundary are reachable here because `2^61` fits in `isize`.
#[test]
fn e96_aegis128l_encrypt_mlen_misuse() {
    init_both();
    assert_max("crypto_aead_aegis128l_messagebytes_max", AEGIS_MAX);
    let p = guarded_scratch(1 << 16);
    let k = [0x55u8; 16];
    let n = [0x66u8; 16];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<AeadEnc, _>(
        "ERRORS 96 aegis128l_encrypt(mlen=2^61) must misuse",
        "crypto_aead_aegis128l_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, AEGIS_MAX + 1, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        MISUSE,
    );
    expect_outcome::<AeadEnc, _>(
        "ERRORS 96 aegis128l_encrypt(mlen=2^61-1) is AT the bound: no misuse",
        "crypto_aead_aegis128l_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, AEGIS_MAX, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        NO_MISUSE,
    );
    let mut rng = Rng::new(SEED ^ 96);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(16);
        let npub = rng.bytes(16);
        enc(&A128L, &m, None, &npub, &k, false, true, "e96/legal");
    }
}

/// Rows 97 + 102: `*_encrypt_detached` writes `*maclen_p = ABYTES` BEFORE it
/// checks the bounds, so on the misuse path the out-param has ALREADY been
/// mutated. The check runs in a forked child writing into a MAP_SHARED page, so
/// the parent can observe the mutation that happened just before `abort()`.
fn aegis_encrypt_detached_misuse(fam: &Aead, kb: usize, nb: usize, label: &str) {
    let sh = SharedPage::new(4096);
    let p = guarded_scratch(1 << 16);
    let mp = sh.p as *mut u64;
    let k = vec![0x77u8; kb];
    let n = vec![0x88u8; nb];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    let name = fam.n("_encrypt_detached");
    let macp = unsafe { p.add(1 << 15) };

    for (which, mlen, adlen) in [
        ("mlen", AEGIS_MAX + 1, 0u64),
        ("adlen", 0u64, AEGIS_MAX + 1),
        ("both", AEGIS_MAX + 1, AEGIS_MAX + 1),
    ] {
        sh.reset();
        assert_eq!(sh.u64_at(0), U64SENT, "the shared sentinel was not reset");
        let l = libs();
        let fc: AeadEncDet = *unsafe { sym::<AeadEncDet>(&l.c, &name) };
        let fr: AeadEncDet = *unsafe { sym::<AeadEncDet>(&l.r, &name) };
        let body = move |f: AeadEncDet| unsafe {
            f(p, macp, mp, p, mlen, p, adlen, ptr::null(), np, kp) as i64
        };
        let oc = forked(move || {
            unsafe { arm_fault_marker() };
            body(fc)
        });
        let after_c = sh.u64_at(0);
        sh.reset();
        let or = forked(move || {
            unsafe { arm_fault_marker() };
            body(fr)
        });
        let after_r = sh.u64_at(0);
        let what = format!("{label} {name} ({which} > MESSAGEBYTES_MAX)");
        assert_same_fatal(&what, oc, or);
        assert_eq!(oc, MISUSE, "{what}: expected SIGABRT, got {oc:?}");
        assert_eq!(
            after_c, after_r,
            "{what}: *maclen_p as written before abort() differs (C={after_c} rust={after_r})"
        );
        assert_eq!(
            after_c, fam.ab as u64,
            "{what}: *maclen_p must ALREADY be ABYTES ({}) when the misuse fires, got {after_c}",
            fam.ab
        );
    }
    // maclen_p == NULL on the same path must still abort
    expect_outcome::<AeadEncDet, _>(
        &format!("{label} {name} with maclen_p == NULL must still misuse"),
        &name,
        move |f| unsafe {
            f(p, macp, ptr::null_mut(), p, AEGIS_MAX + 1, p, 0, ptr::null(), np, kp) as i64
        },
        MISUSE,
    );
    // and at the bound it does not misuse
    expect_outcome::<AeadEncDet, _>(
        &format!("{label} {name}(mlen=2^61-1) is AT the bound: no misuse"),
        &name,
        move |f| unsafe {
            f(p, macp, ptr::null_mut(), p, AEGIS_MAX, p, 0, ptr::null(), np, kp) as i64
        },
        NO_MISUSE,
    );
}

#[test]
fn e97_aegis128l_encrypt_detached_misuse_after_maclen() {
    init_both();
    aegis_encrypt_detached_misuse(&A128L, 16, 16, "ERRORS 97");
    let mut rng = Rng::new(SEED ^ 97);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let ad = rnd(&mut rng, 130);
        let k = rng.bytes(16);
        let npub = rng.bytes(16);
        enc_det(&A128L, &m, Some(&ad), &npub, &k, false, true, "e97/legal");
    }
}

#[test]
fn e102_aegis256_encrypt_detached_misuse_after_maclen() {
    init_both();
    aegis_encrypt_detached_misuse(&A256, 32, 32, "ERRORS 102");
    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let ad = rnd(&mut rng, 130);
        let k = rng.bytes(32);
        let npub = rng.bytes(32);
        enc_det(&A256, &m, Some(&ad), &npub, &k, false, true, "e102/legal");
    }
}

/// Rows 98 + 103: `*_decrypt` with `clen < ABYTES` (32 for both AEGIS variants).
fn aegis_decrypt_short(fam: &Aead, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for clen in 0..fam.ab {
        for _ in 0..3 {
            let c = rng.bytes(clen);
            let k = rng.bytes(fam.kb);
            let npub = rng.bytes(fam.nb);
            let (rc, buf) = dec(fam, &c, None, &npub, &k, false, true, label);
            assert_eq!(rc, -1, "{label}: clen={clen} < {} must return -1", fam.ab);
            assert!(buf.is_empty());
            let (rc2, _) = dec(fam, &c, None, &npub, &k, false, false, label);
            assert_eq!(rc2, -1);
            iters += 1;
        }
    }
    for _ in 0..8 {
        let k = rng.bytes(fam.kb);
        let npub = rng.bytes(fam.nb);
        let c = enc(fam, &[], None, &npub, &k, false, true, label);
        assert_eq!(c.len(), fam.ab);
        let (rc, _) = dec(fam, &c, None, &npub, &k, false, true, label);
        assert_eq!(rc, 0, "{label}: clen == ABYTES must be accepted");
        iters += 1;
    }
    assert!(iters >= 64, "{label} only drove {iters} inputs");
}

#[test]
fn e98_aegis128l_decrypt_clen_too_short() {
    init_both();
    aegis_decrypt_short(&A128L, SEED ^ 98, "ERRORS 98");
}

#[test]
fn e103_aegis256_decrypt_clen_too_short() {
    init_both();
    aegis_decrypt_short(&A256, SEED ^ 103, "ERRORS 103");
}

/// Rows 99 + 104: `*_decrypt_detached` returns -1 (NOT a misuse) when `clen` or
/// `adlen` exceeds `MESSAGEBYTES_MAX`. The guard fires before any memory is
/// touched, so the call is safe to make directly.
fn aegis_decrypt_detached_len_reject(fam: &Aead, kb: usize, nb: usize, label: &str) {
    let name = fam.n("_decrypt_detached");
    let (fc, fr) = unsafe { pair::<AeadDecDet>(&name) };
    let mut rng = Rng::new(SEED ^ (fam.ab as u64) ^ 0x99);
    let k = vec![0xABu8; kb];
    let n = vec![0xCDu8; nb];
    let mut iters = 0usize;
    for &(clen, adlen) in &[
        (AEGIS_MAX + 1, 0u64),
        (u64::MAX, 0),
        (AEGIS_MAX + 1, AEGIS_MAX + 1),
        (0, AEGIS_MAX + 1),
        (16, u64::MAX),
    ] {
        for _ in 0..14 {
            let mut bc = vec![FILL; 256];
            let mut br = vec![FILL; 256];
            let mac = rng.bytes(fam.ab);
            let rc = unsafe {
                fc(
                    bc.as_mut_ptr(), ptr::null_mut(), bc.as_ptr(), clen, mac.as_ptr(),
                    br.as_ptr(), adlen, n.as_ptr(), k.as_ptr(),
                )
            };
            let rr = unsafe {
                fr(
                    br.as_mut_ptr(), ptr::null_mut(), br.as_ptr(), clen, mac.as_ptr(),
                    bc.as_ptr(), adlen, n.as_ptr(), k.as_ptr(),
                )
            };
            let what = format!("{label} {name}(clen={clen}, adlen={adlen})");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: must return -1 (not misuse)");
            assert!(
                bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                "{what}: nothing may be written when the length guard fires"
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "{label} only drove {iters} inputs");
}

#[test]
fn e99_aegis128l_decrypt_detached_len_reject() {
    init_both();
    aegis_decrypt_detached_len_reject(&A128L, 16, 16, "ERRORS 99");
}

#[test]
fn e104_aegis256_decrypt_detached_len_reject() {
    init_both();
    aegis_decrypt_detached_len_reject(&A256, 32, 32, "ERRORS 104");
}

/// Row 100: AEGIS-128L `crypto_verify_32` mismatch (maclen 32).
#[test]
fn e100_aegis128l_decrypt_detached_forged() {
    init_both();
    forged_detached_rows(&A128L, SEED ^ 100, "ERRORS 100");
}

/// Row 105: AEGIS-256 `crypto_verify_32` mismatch.
#[test]
fn e105_aegis256_decrypt_detached_forged() {
    init_both();
    forged_detached_rows(&A256, SEED ^ 105, "ERRORS 105");
}

/// Row 101: `crypto_aead_aegis256_encrypt` bound.
#[test]
fn e101_aegis256_encrypt_mlen_misuse() {
    init_both();
    assert_max("crypto_aead_aegis256_messagebytes_max", AEGIS_MAX);
    let p = guarded_scratch(1 << 16);
    let k = [0x99u8; 32];
    let n = [0xAAu8; 32];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    expect_outcome::<AeadEnc, _>(
        "ERRORS 101 aegis256_encrypt(mlen=2^61) must misuse",
        "crypto_aead_aegis256_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, AEGIS_MAX + 1, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        MISUSE,
    );
    expect_outcome::<AeadEnc, _>(
        "ERRORS 101 aegis256_encrypt(mlen=2^61-1) is AT the bound: no misuse",
        "crypto_aead_aegis256_encrypt",
        move |f| unsafe {
            f(p, ptr::null_mut(), p, AEGIS_MAX, ptr::null(), 0, ptr::null(), np, kp) as i64
        },
        NO_MISUSE,
    );
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let npub = rng.bytes(32);
        enc(&A256, &m, None, &npub, &k, false, true, "e101/legal");
    }
}

// ------------------------------------------------- ERRORS 116–124: secretbox

/// Row 116: `crypto_secretbox_easy` with `mlen > SODIUM_SIZE_MAX - 16`.
/// (Only the misuse side is driven — see `e84` for why.)
#[test]
fn e116_secretbox_easy_mlen_misuse() {
    init_both();
    assert_max("crypto_secretbox_messagebytes_max", SB_MAX);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x12u8; 32];
    let n = [0x34u8; 24];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    for mlen in [SB_MAX + 1, SB_MAX + 9, u64::MAX] {
        expect_outcome::<SbEasy, _>(
            &format!("ERRORS 116 crypto_secretbox_easy(mlen={mlen}) must misuse"),
            "crypto_secretbox_easy",
            move |f| unsafe { f(p, p, mlen, np, kp) as i64 },
            MISUSE,
        );
    }
    let mut rng = Rng::new(SEED ^ 116);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        sb_easy(&SBS, &m, &n, &k, "e116/legal");
    }
}

/// Rows 117 + 123: `*_open_easy` with `clen < MACBYTES` returns -1 and never
/// touches `m` (the guard is BEFORE the delegation to `_open_detached`).
fn open_easy_short(s: &Sb, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for clen in 0..s.mb {
        for _ in 0..5 {
            let c = rng.bytes(clen);
            let k = rng.bytes(s.kb);
            let n = rng.bytes(s.nb);
            let (rc, buf) = sb_open_easy(s, &c, &n, &k, label);
            assert_eq!(rc, -1, "{label}: clen={clen} < {} must return -1", s.mb);
            assert!(
                buf.iter().all(|&x| x == FILL),
                "{label}: nothing may be written for clen={clen}"
            );
            iters += 1;
        }
    }
    // clen == MACBYTES is legal: it opens a zero-length message
    for _ in 0..8 {
        let k = rng.bytes(s.kb);
        let n = rng.bytes(s.nb);
        let c = sb_easy(s, &[], &n, &k, label);
        assert_eq!(c.len(), s.mb);
        let (rc, _) = sb_open_easy(s, &c, &n, &k, label);
        assert_eq!(rc, 0, "{label}: clen == MACBYTES must be accepted");
        iters += 1;
    }
    assert!(iters >= 64, "{label} only drove {iters} inputs");
}

#[test]
fn e117_secretbox_open_easy_clen_too_short() {
    init_both();
    open_easy_short(&SBS, SEED ^ 117, "ERRORS 117");
}

#[test]
fn e123_secretbox_xchacha_open_easy_clen_too_short() {
    init_both();
    open_easy_short(&SBX, SEED ^ 123, "ERRORS 123");
}

/// Rows 118 + 124: `*_open_detached` on a poly1305 mismatch zeroes its internal
/// sub-key and returns -1 WITHOUT writing anything to `m` (unlike the AEAD
/// `*_decrypt_detached` functions, which memset the plaintext).
fn open_detached_forged(s: &Sb, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    let mut iters = 0usize;
    for &ml in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 200, 1000] {
        for _ in 0..5 {
            let m = rng.bytes(ml);
            let k = rng.bytes(s.kb);
            let n = rng.bytes(s.nb);
            let (c, mac) = sb_det(s, &m, &n, &k, label);
            let mut bad = mac.clone();
            bad[rng.below(s.mb)] ^= 1 << rng.below(8);
            for m_null in [false, true] {
                let (rc, buf) = sb_open_det(s, &c, &bad, &n, &k, m_null, label);
                assert_eq!(rc, -1, "{label}: a forged MAC was accepted (m_null={m_null})");
                assert!(
                    buf.iter().all(|&x| x == FILL),
                    "{label}: open_detached must NOT write m on a MAC mismatch \
                     (m_null={m_null}), got {}",
                    hexs(&buf)
                );
            }
            // a flipped ciphertext byte is equally rejected
            if ml > 0 {
                let mut bc = c.clone();
                bc[rng.below(ml)] ^= 1;
                let (rc, buf) = sb_open_det(s, &bc, &mac, &n, &k, false, label);
                assert_eq!(rc, -1, "{label}: a flipped ciphertext byte was accepted");
                assert!(buf.iter().all(|&x| x == FILL), "{label}: m written on failure");
            }
            iters += 1;
        }
    }
    assert!(iters >= 64, "{label} only drove {iters} inputs");
}

#[test]
fn e118_secretbox_open_detached_forged() {
    init_both();
    open_detached_forged(&SBS, SEED ^ 118, "ERRORS 118");
}

#[test]
fn e124_secretbox_xchacha_open_detached_forged() {
    init_both();
    open_detached_forged(&SBX, SEED ^ 124, "ERRORS 124");
}

/// Row 119: `crypto_secretbox_xsalsa20poly1305` (== `crypto_secretbox`) rejects
/// `mlen < ZEROBYTES` (32) with -1 and writes nothing.
#[test]
fn e119_xsalsa20poly1305_mlen_below_zerobytes() {
    init_both();
    let mut rng = Rng::new(SEED ^ 119);
    let mut iters = 0usize;
    for name in ["crypto_secretbox_xsalsa20poly1305", "crypto_secretbox"] {
        let (fc, fr) = unsafe { pair::<SbEasy>(name) };
        for mlen in 0..32usize {
            let m = rng.bytes(mlen.max(1));
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut bc = vec![FILL; 64];
            let mut br = vec![FILL; 64];
            let rc =
                unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
            let rr =
                unsafe { fr(br.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), k.as_ptr()) };
            let what = format!("ERRORS 119 {name}(mlen={mlen})");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: mlen < 32 must return -1");
            assert_eq_bytes(&what, &bc, &br);
            assert!(
                bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                "{what}: nothing may be written"
            );
            iters += 1;
        }
        // mlen == 32 is the first accepted length
        for _ in 0..8 {
            let m = vec![0u8; 32];
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut bc = vec![FILL; 32 + PAD];
            let mut br = vec![FILL; 32 + PAD];
            let rc = unsafe { fc(bc.as_mut_ptr(), m.as_ptr(), 32, n.as_ptr(), k.as_ptr()) };
            let rr = unsafe { fr(br.as_mut_ptr(), m.as_ptr(), 32, n.as_ptr(), k.as_ptr()) };
            assert_eq!((rc, rr), (0, 0), "ERRORS 119: mlen == 32 must be accepted");
            assert_eq_bytes("ERRORS 119 boundary", &bc, &br);
            guard_intact("ERRORS 119 boundary", "C", &bc, 32);
            iters += 1;
        }
    }
    assert!(iters >= 64, "e119 only drove {iters} inputs");
}

/// Row 120: `crypto_secretbox_xsalsa20poly1305_open` (== `crypto_secretbox_open`)
/// rejects `clen < 32`.
#[test]
fn e120_xsalsa20poly1305_open_clen_below_zerobytes() {
    init_both();
    let mut rng = Rng::new(SEED ^ 120);
    let mut iters = 0usize;
    for name in ["crypto_secretbox_xsalsa20poly1305_open", "crypto_secretbox_open"] {
        let (fc, fr) = unsafe { pair::<SbEasy>(name) };
        for clen in 0..32usize {
            let c = rng.bytes(clen.max(1));
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut bc = vec![FILL; 64];
            let mut br = vec![FILL; 64];
            let rc =
                unsafe { fc(bc.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
            let rr =
                unsafe { fr(br.as_mut_ptr(), c.as_ptr(), clen as u64, n.as_ptr(), k.as_ptr()) };
            let what = format!("ERRORS 120 {name}(clen={clen})");
            assert_eq!(rc, rr, "{what}: return differs (C={rc} rust={rr})");
            assert_eq!(rc, -1, "{what}: clen < 32 must return -1");
            assert_eq_bytes(&what, &bc, &br);
            assert!(
                bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                "{what}: nothing may be written"
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "e120 only drove {iters} inputs");
}

/// Row 121: `crypto_secretbox_xsalsa20poly1305_open` on a poly1305 mismatch
/// returns -1 and leaves `m` untouched (the verify happens before the stream).
#[test]
fn e121_xsalsa20poly1305_open_forged() {
    init_both();
    let (ec, _er) = unsafe { pair::<SbEasy>("crypto_secretbox_xsalsa20poly1305") };
    let (oc, or) = unsafe { pair::<SbEasy>("crypto_secretbox_xsalsa20poly1305_open") };
    let mut rng = Rng::new(SEED ^ 121);
    let mut iters = 0usize;
    for &ml in &[32usize, 33, 48, 64, 96, 1000] {
        for _ in 0..12 {
            let mut m = vec![0u8; 32];
            m.extend_from_slice(&rng.bytes(ml - 32));
            let k = rng.bytes(32);
            let n = rng.bytes(24);
            let mut c = vec![0u8; ml];
            let rc = unsafe { ec(c.as_mut_ptr(), m.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            assert_eq!(rc, 0);
            // flip a bit in the MAC (c[16..32]) or in the ciphertext (c[32..])
            let i = 16 + rng.below(ml - 16);
            c[i] ^= 1 << rng.below(8);
            let mut bc = vec![FILL; ml + PAD];
            let mut br = vec![FILL; ml + PAD];
            let r1 = unsafe { oc(bc.as_mut_ptr(), c.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let r2 = unsafe { or(br.as_mut_ptr(), c.as_ptr(), ml as u64, n.as_ptr(), k.as_ptr()) };
            let what = format!("ERRORS 121 open(forged at byte {i}) clen={ml}");
            assert_eq!(r1, r2, "{what}: return differs (C={r1} rust={r2})");
            assert_eq!(r1, -1, "{what}: a forged MAC was accepted");
            assert_eq_bytes(&what, &bc, &br);
            assert!(
                bc.iter().all(|&x| x == FILL) && br.iter().all(|&x| x == FILL),
                "{what}: m must be untouched on a MAC mismatch, got {}",
                hexs(&bc)
            );
            iters += 1;
        }
    }
    assert!(iters >= 64, "e121 only drove {iters} inputs");
}

/// Row 122: `crypto_secretbox_xchacha20poly1305_easy` bound.
#[test]
fn e122_secretbox_xchacha_easy_mlen_misuse() {
    init_both();
    assert_max("crypto_secretbox_xchacha20poly1305_messagebytes_max", SB_MAX);
    let s = scratch(4096);
    let p = s.p;
    let k = [0x56u8; 32];
    let n = [0x78u8; 24];
    let kp = k.as_ptr();
    let np = n.as_ptr();
    for mlen in [SB_MAX + 1, u64::MAX] {
        expect_outcome::<SbEasy, _>(
            &format!("ERRORS 122 crypto_secretbox_xchacha20poly1305_easy(mlen={mlen}) must misuse"),
            "crypto_secretbox_xchacha20poly1305_easy",
            move |f| unsafe { f(p, p, mlen, np, kp) as i64 },
            MISUSE,
        );
    }
    let mut rng = Rng::new(SEED ^ 122);
    for _ in 0..64 {
        let m = rnd(&mut rng, 200);
        let k = rng.bytes(32);
        let n = rng.bytes(24);
        sb_easy(&SBX, &m, &n, &k, "e122/legal");
    }
}

// --------------------------------------------- ERRORS 125–130: secretstream

/// A live secretstream state usable from a forked child: allocated (and
/// `_init_pull`-initialised) in the PARENT so the child needs no allocator.
fn forkable_state(which: usize, header: &[u8], k: &[u8]) -> *mut u8 {
    let p = guarded_scratch(4096);
    let f: SsInitPull = *unsafe { sym::<SsInitPull>(lib_of(which), &format!("{SS}_init_pull")) };
    let rc = unsafe { f(p, header.as_ptr(), k.as_ptr()) };
    assert_eq!(rc, 0, "init_pull for the forkable state failed");
    p
}

/// Row 125: `_push` with `mlen > min(SIZE_MAX-17, 64*(2^32-2))` misuses, and
/// `*outlen_p` has ALREADY been set to 0 by the time the abort happens.
#[test]
fn e125_ss_push_mlen_misuse() {
    init_both();
    assert_max("crypto_secretstream_xchacha20poly1305_messagebytes_max", SS_MAX);
    let header = [0x11u8; SS_HEADERBYTES];
    let k = [0x22u8; SS_KEYBYTES];
    let stc = forkable_state(0, &header, &k);
    let str_ = forkable_state(1, &header, &k);
    let out = guarded_scratch(1 << 16);
    let sh = SharedPage::new(4096);
    let op = sh.p as *mut u64;
    let name = format!("{SS}_push");
    let l = libs();
    let fc: SsPush = *unsafe { sym::<SsPush>(&l.c, &name) };
    let fr: SsPush = *unsafe { sym::<SsPush>(&l.r, &name) };

    for mlen in [SS_MAX + 1, SS_MAX + 64, u64::MAX] {
        sh.reset();
        let oc = forked(move || {
            unsafe { arm_fault_marker() };
            unsafe { fc(stc, out, op, out, mlen, ptr::null(), 0, TAG_MESSAGE) as i64 }
        });
        let after_c = sh.u64_at(0);
        sh.reset();
        let or = forked(move || {
            unsafe { arm_fault_marker() };
            unsafe { fr(str_, out, op, out, mlen, ptr::null(), 0, TAG_MESSAGE) as i64 }
        });
        let after_r = sh.u64_at(0);
        let what = format!("ERRORS 125 {name}(mlen={mlen})");
        assert_same_fatal(&what, oc, or);
        assert_eq!(oc, MISUSE, "{what}: expected SIGABRT, got {oc:?}");
        assert_eq!(
            after_c, after_r,
            "{what}: *outlen_p as written before abort() differs (C={after_c} rust={after_r})"
        );
        assert_eq!(
            after_c, 0,
            "{what}: *outlen_p must ALREADY be 0 when the misuse fires, got {after_c}"
        );
    }
    // at the bound the guard does NOT fire (the call runs on and faults)
    expect_outcome::<SsPush, _>(
        &format!("ERRORS 125 {name}(mlen={SS_MAX}) is AT the bound: no misuse"),
        &name,
        move |f: SsPush| unsafe {
            f(stc, out, ptr::null_mut(), out, SS_MAX, ptr::null(), 0, TAG_MESSAGE) as i64
        },
        NO_MISUSE,
    );
    // outlen_p == NULL on the misuse path must still abort
    expect_outcome::<SsPush, _>(
        &format!("ERRORS 125 {name} with outlen_p == NULL must still misuse"),
        &name,
        move |f: SsPush| unsafe {
            f(stc, out, ptr::null_mut(), out, SS_MAX + 1, ptr::null(), 0, TAG_MESSAGE) as i64
        },
        MISUSE,
    );
}

/// Row 126: `_pull` with `inlen < ABYTES` (17) returns -1 with `*mlen_p == 0`
/// and `*tag_p == 0xff`, and never reads `in`.
#[test]
fn e126_ss_pull_inlen_too_short() {
    init_both();
    let mut rng = Rng::new(SEED ^ 126);
    let mut iters = 0usize;
    for inlen in 0..SS_ABYTES {
        for _ in 0..4 {
            let k = rng.bytes(SS_KEYBYTES);
            let header = rng.bytes(SS_HEADERBYTES);
            let input = rng.bytes(inlen);
            let mut st = SsState::new();
            ss_init_pull(&mut st, &header, &k, "e126");
            let before = st.body();
            let (rc, buf, ml, tg) = ss_pull(&mut st, &input, None, true, true, "e126");
            assert_eq!(rc, -1, "ERRORS 126: inlen={inlen} < 17 must return -1");
            assert_eq!(ml, 0, "ERRORS 126: *mlen_p must be 0");
            assert_eq!(tg, 0xff, "ERRORS 126: *tag_p must be 0xff");
            assert!(buf.iter().all(|&x| x == FILL), "ERRORS 126: nothing may be written");
            assert_eq_bytes("ERRORS 126: the state must not advance", &before, &st.body());
            // NULL out-params on the same path
            let (rc2, _, _, _) = ss_pull(&mut st, &input, None, false, false, "e126/null");
            assert_eq!(rc2, -1);
            assert_eq_bytes("ERRORS 126: state still unchanged", &before, &st.body());
            iters += 1;
        }
    }
    // inlen == 17 is the first accepted length (a zero-length message)
    for _ in 0..8 {
        let k = rng.bytes(SS_KEYBYTES);
        let header = rng.bytes(SS_HEADERBYTES);
        let mut push = SsState::new();
        let mut pull = SsState::new();
        ss_init_pull(&mut push, &header, &k, "e126/boundary");
        ss_init_pull(&mut pull, &header, &k, "e126/boundary");
        let ct = ss_push(&mut push, &[], None, TAG_MESSAGE, true, "e126/boundary");
        assert_eq!(ct.len(), SS_ABYTES);
        let (rc, _, ml, tg) = ss_pull(&mut pull, &ct, None, true, true, "e126/boundary");
        assert_eq!(rc, 0, "ERRORS 126: inlen == 17 must be accepted");
        assert_eq!((ml, tg), (0, TAG_MESSAGE));
        iters += 1;
    }
    assert!(iters >= 64, "e126 only drove {iters} inputs");
}

/// Row 127: `_pull` with `inlen - 17 > MESSAGEBYTES_MAX` misuses, after having
/// already written `*mlen_p = 0` and `*tag_p = 0xff`.
#[test]
fn e127_ss_pull_mlen_misuse() {
    init_both();
    let header = [0x33u8; SS_HEADERBYTES];
    let k = [0x44u8; SS_KEYBYTES];
    let stc = forkable_state(0, &header, &k);
    let str_ = forkable_state(1, &header, &k);
    let buf = guarded_scratch(1 << 16);
    let sh = SharedPage::new(4096);
    let mlp = sh.p as *mut u64;
    let tgp = unsafe { sh.p.add(16) };
    let name = format!("{SS}_pull");
    let l = libs();
    let fc: SsPull = *unsafe { sym::<SsPull>(&l.c, &name) };
    let fr: SsPull = *unsafe { sym::<SsPull>(&l.r, &name) };

    for inlen in [SS_MAX + 18, u64::MAX] {
        sh.reset();
        let oc = forked(move || {
            unsafe { arm_fault_marker() };
            unsafe { fc(stc, buf, mlp, tgp, buf, inlen, ptr::null(), 0) as i64 }
        });
        let (mc, tc) = (sh.u64_at(0), sh.bytes()[16]);
        sh.reset();
        let or = forked(move || {
            unsafe { arm_fault_marker() };
            unsafe { fr(str_, buf, mlp, tgp, buf, inlen, ptr::null(), 0) as i64 }
        });
        let (mr, tr) = (sh.u64_at(0), sh.bytes()[16]);
        let what = format!("ERRORS 127 {name}(inlen={inlen})");
        assert_same_fatal(&what, oc, or);
        assert_eq!(oc, MISUSE, "{what}: expected SIGABRT, got {oc:?}");
        assert_eq!(mc, mr, "{what}: *mlen_p before abort differs (C={mc} rust={mr})");
        assert_eq!(tc, tr, "{what}: *tag_p before abort differs (C={tc:#04x} rust={tr:#04x})");
        assert_eq!(mc, 0, "{what}: *mlen_p must already be 0");
        assert_eq!(tc, 0xff, "{what}: *tag_p must already be 0xff");
    }
    // inlen == MESSAGEBYTES_MAX + 17 is exactly AT the bound: no misuse
    expect_outcome::<SsPull, _>(
        &format!("ERRORS 127 {name}(inlen=MESSAGEBYTES_MAX+17) is AT the bound: no misuse"),
        &name,
        move |f: SsPull| unsafe {
            f(stc, buf, ptr::null_mut(), ptr::null_mut(), buf, SS_MAX + 17, ptr::null(), 0) as i64
        },
        NO_MISUSE,
    );
}

/// Row 128: on a forged MAC `_pull` returns -1, leaves `*mlen_p == 0` and
/// `*tag_p == 0xff`, and — crucially — does NOT advance the state, so a
/// subsequent CORRECT `_pull` of the very same frame still succeeds.
#[test]
fn e128_ss_pull_forged_state_not_advanced() {
    init_both();
    let mut rng = Rng::new(SEED ^ 128);
    let mut iters = 0usize;
    for &ml in SS_MLEN {
        for &tag in &[TAG_MESSAGE, TAG_PUSH, TAG_REKEY, TAG_FINAL] {
            for _ in 0..3 {
                let k = rng.bytes(SS_KEYBYTES);
                let header = rng.bytes(SS_HEADERBYTES);
                let m = rng.bytes(ml);
                let ad = rnd(&mut rng, 20);
                let mut push = SsState::new();
                let mut pull = SsState::new();
                ss_init_pull(&mut push, &header, &k, "e128");
                ss_init_pull(&mut pull, &header, &k, "e128");
                let ct = ss_push(&mut push, &m, Some(&ad), tag, true, "e128");
                let before = pull.body();

                // flip a bit anywhere in the frame (tag byte, ciphertext or MAC)
                let mut bad = ct.clone();
                let i = rng.below(bad.len());
                bad[i] ^= 1 << rng.below(8);
                let (rc, buf, mlp, tgp) = ss_pull(&mut pull, &bad, Some(&ad), true, true, "e128");
                if rc == 0 {
                    // flipping the tag byte alone is authenticated too, so a
                    // successful pull here would be a real forgery
                    panic!("ERRORS 128: a forged frame (bit {i}) was ACCEPTED");
                }
                assert_eq!(rc, -1, "ERRORS 128: expected -1");
                assert_eq!(mlp, 0, "ERRORS 128: *mlen_p must stay 0");
                assert_eq!(tgp, 0xff, "ERRORS 128: *tag_p must stay 0xff");
                assert!(
                    buf.iter().all(|&x| x == FILL),
                    "ERRORS 128: _pull must not write m on a MAC failure, got {}",
                    hexs(&buf)
                );
                assert_eq_bytes(
                    "ERRORS 128: the state MUST NOT advance on a MAC failure",
                    &before,
                    &pull.body(),
                );
                // a wrong AD is also a MAC failure with the same guarantees
                let mut bad_ad = ad.clone();
                bad_ad.push(0x5A);
                let (rc2, _, m2, t2) = ss_pull(&mut pull, &ct, Some(&bad_ad), true, true, "e128/ad");
                assert_eq!(rc2, -1, "ERRORS 128: a wrong AD was accepted");
                assert_eq!((m2, t2), (0, 0xff));
                assert_eq_bytes(
                    "ERRORS 128: the state MUST NOT advance for a wrong AD",
                    &before,
                    &pull.body(),
                );
                // and now the CORRECT frame still opens
                let (rc3, p3, m3, t3) = ss_pull(&mut pull, &ct, Some(&ad), true, true, "e128/retry");
                assert_eq!(rc3, 0, "ERRORS 128: the correct frame failed after a forgery");
                assert_eq!(m3, ml as u64);
                assert_eq!(t3, tag);
                assert_eq_bytes("ERRORS 128: recovered plaintext", &m, &p3[..ml]);
                assert_eq_bytes(
                    "ERRORS 128: push/pull states diverged after the retry",
                    &push.body(),
                    &pull.body(),
                );
                iters += 1;
            }
        }
    }
    assert!(iters >= 64, "e128 only drove {iters} inputs");
}

/// Row 129: `_init_pull` accepts ANY 24-byte header with no validation at all —
/// it always returns 0. A wrong header only shows up as a MAC failure on the
/// first `_pull`.
#[test]
fn e129_ss_init_pull_any_header() {
    init_both();
    let mut rng = Rng::new(SEED ^ 129);
    let mut headers: Vec<Vec<u8>> = vec![
        vec![0u8; SS_HEADERBYTES],
        vec![0xffu8; SS_HEADERBYTES],
        (0..SS_HEADERBYTES).map(|i| i as u8).collect(),
        (0..SS_HEADERBYTES).map(|i| 0xff - i as u8).collect(),
    ];
    for _ in 0..60 {
        headers.push(rng.bytes(SS_HEADERBYTES));
    }
    let mut iters = 0usize;
    for (i, h) in headers.iter().enumerate() {
        let k = rng.bytes(SS_KEYBYTES);
        let mut st = SsState::new();
        let rc = ss_init_pull(&mut st, h, &k, &format!("e129/#{i}"));
        assert_eq!(rc, 0, "ERRORS 129: _init_pull must always return 0");
        // the state is fully determined by the header: nonce == [1,0,0,0] || h[16..24)
        let nonce = st.nonce();
        assert_eq!(&nonce[..4], &[1, 0, 0, 0], "ERRORS 129: counter must be reset to 1");
        assert_eq!(&nonce[4..], &h[16..24], "ERRORS 129: INONCE must be header[16..24)");
        assert!(st.k() != vec![FILL; 32], "ERRORS 129: state->k was not written");
        // a WRONG header shows up only as a MAC failure on the first pull
        let mut push = SsState::new();
        ss_init_pull(&mut push, h, &k, "e129/push");
        let m = rnd(&mut rng, 100);
        let ct = ss_push(&mut push, &m, None, TAG_MESSAGE, true, "e129/push");
        let mut wrong = h.clone();
        wrong[rng.below(SS_HEADERBYTES)] ^= 1 << rng.below(8);
        let mut bad = SsState::new();
        let rcb = ss_init_pull(&mut bad, &wrong, &k, "e129/wrong");
        assert_eq!(rcb, 0, "ERRORS 129: a wrong header is still ACCEPTED by _init_pull");
        let (rp, _, mlp, tgp) = ss_pull(&mut bad, &ct, None, true, true, "e129/wrong-pull");
        assert_eq!(rp, -1, "ERRORS 129: a wrong header must fail at the first _pull");
        assert_eq!((mlp, tgp), (0, 0xff));
        iters += 1;
    }
    assert!(iters >= 64, "e129 only drove {iters} inputs");
}

/// Row 130: `_push` performs NO validation of `tag` — every one of the 256
/// possible byte values is accepted and returns 0; only bit 0x02 rekeys.
#[test]
fn e130_ss_push_any_tag() {
    init_both();
    let mut rng = Rng::new(SEED ^ 130);
    let k = rng.bytes(SS_KEYBYTES);
    let header = rng.bytes(SS_HEADERBYTES);
    let mut iters = 0usize;
    for tag in 0u16..=255 {
        let tag = tag as u8;
        let m = rnd(&mut rng, 80);
        let ad = rnd(&mut rng, 20);
        let mut push = SsState::new();
        let mut pull = SsState::new();
        ss_init_pull(&mut push, &header, &k, "e130");
        ss_init_pull(&mut pull, &header, &k, "e130");
        let before = push.body();
        let ct = ss_push(&mut push, &m, Some(&ad), tag, true, "e130");
        let (rc, p, mlp, tgp) = ss_pull(&mut pull, &ct, Some(&ad), true, true, "e130");
        assert_eq!(rc, 0, "ERRORS 130: tag={tag:#04x} was rejected by _pull");
        assert_eq!(mlp, m.len() as u64);
        assert_eq!(tgp, tag, "ERRORS 130: _pull must report the exact tag byte");
        assert_eq_bytes("ERRORS 130 plaintext", &m, &p[..m.len()]);
        let after = push.body();
        let rekeyed = after[..32] != before[..32];
        assert_eq!(
            rekeyed,
            (tag & TAG_REKEY) != 0,
            "ERRORS 130: tag={tag:#04x} — only bit 0x02 may trigger a rekey"
        );
        assert_eq_bytes("ERRORS 130 push/pull states", &push.body(), &pull.body());
        iters += 1;
    }
    assert_eq!(iters, 256, "e130 must cover all 256 tag bytes, covered {iters}");
}
