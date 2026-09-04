//! Differential tests for AREA `blake2`:
//!
//! * `crypto_generichash/**` (crypto_generichash.c, blake2b/generichash_blake2.c,
//!   blake2b/ref/{blake2b-ref.c, blake2b-compress-ref.c, generichash_blake2b.c})
//! * `crypto_kdf/**` (crypto_kdf.c, blake2b/kdf_blake2b.c, hkdf/kdf_hkdf_sha256.c,
//!   hkdf/kdf_hkdf_sha512.c)
//! * `crypto_shorthash/**` (crypto_shorthash.c, siphash24/**)
//! * `crypto_verify/verify.c`
//! * plus `_sodium_blake2b_long` (crypto_pwhash/argon2/blake2b-long.c), which the
//!   Rust translation places in `src/blake2b.rs`.
//!
//! Every call goes through `dlopen`/`dlsym` on both shared objects.

#[macro_use]
mod common;

use core::ffi::{c_char, c_int, c_void};

// ============================================================ helpers =======

/// 64-byte-aligned scratch buffer, large enough for every opaque state used
/// here (`crypto_generichash_blake2b_state` = 384 bytes,
/// `crypto_kdf_hkdf_sha512_state` = 416 bytes).
#[repr(align(64))]
struct AlignedState([u8; 512]);

impl AlignedState {
    fn new() -> Self {
        AlignedState([0u8; 512])
    }
    fn p(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
    fn s(&self, n: usize) -> &[u8] {
        &self.0[..n]
    }
}

/// Fetch a `size_t (*)(void)` getter from both libraries, assert equality,
/// return the value.
macro_rules! chk_getter {
    ($name:expr) => {{
        let (c, r) = both!($name, unsafe extern "C" fn() -> usize);
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq!(cv, rv, "{}: getter mismatch", $name);
        cv
    }};
}

/// Fetch a `const char *(*)(void)` getter from both libraries and compare.
macro_rules! chk_str {
    ($name:literal) => {{
        let (c, r) = both!($name, unsafe extern "C" fn() -> *const c_char);
        unsafe {
            let cs = std::ffi::CStr::from_ptr(c());
            let rs = std::ffi::CStr::from_ptr(r());
            assert_eq!(cs, rs, concat!($name, ": string mismatch"));
        }
    }};
}

const CANARY: u8 = 0xA5;

/// Canary-filled output buffer: `n` payload bytes followed by 16 guard bytes.
fn canary(n: usize) -> Vec<u8> {
    vec![CANARY; n + 16]
}

// ---- FFI signatures --------------------------------------------------------

type FnGh = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type FnGhSp = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;
type FnGhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int;
type FnGhInitSp =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> c_int;
type FnGhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type FnGhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;
type FnKeygen = unsafe extern "C" fn(*mut u8);
type FnVoidInt = unsafe extern "C" fn() -> c_int;

// low-level blake2b
type FnB2Init = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type FnB2InitSp = unsafe extern "C" fn(*mut u8, u8, *const c_void, *const c_void) -> c_int;
type FnB2InitKey = unsafe extern "C" fn(*mut u8, u8, *const c_void, u8) -> c_int;
type FnB2InitKeySp =
    unsafe extern "C" fn(*mut u8, u8, *const c_void, u8, *const c_void, *const c_void) -> c_int;
type FnB2InitParam = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type FnB2Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type FnB2Final = unsafe extern "C" fn(*mut u8, *mut u8, u8) -> c_int;
type FnB2 = unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8) -> c_int;
type FnB2Sp = unsafe extern "C" fn(
    *mut u8,
    *const c_void,
    *const c_void,
    u8,
    u64,
    u8,
    *const c_void,
    *const c_void,
) -> c_int;
type FnB2Compress = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type FnB2Long = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;

// kdf
type FnKdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
type FnHkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type FnHkdfExtractInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type FnHkdfExtractUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type FnHkdfExtractFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type FnHkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;

// shorthash / verify
type FnShort = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type FnVerify = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

extern "C" {
    fn __errno_location() -> *mut c_int;
}
fn errno() -> c_int {
    unsafe { *__errno_location() }
}
fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}
const EINVAL: c_int = 22;

/// Random split of `n` bytes into chunks, deliberately including 0-length ones.
fn splits(rng: &mut common::Rng, n: usize) -> Vec<usize> {
    let mut v = Vec::new();
    let mut left = n;
    v.push(0); // always start with a zero-length update
    while left > 0 {
        let k = if rng.next_u64() & 7 == 0 {
            0
        } else {
            1 + rng.below(left)
        };
        v.push(k);
        left -= k;
        if rng.next_u64() & 3 == 0 {
            v.push(0);
        }
    }
    v.push(0);
    v
}

// ================================================== blake2-1: constants =====

#[test]
fn constants_and_primitives() {
    // crypto_generichash/crypto_generichash.c
    assert_eq!(chk_getter!("crypto_generichash_bytes_min"), 16);
    assert_eq!(chk_getter!("crypto_generichash_bytes_max"), 64);
    assert_eq!(chk_getter!("crypto_generichash_bytes"), 32);
    assert_eq!(chk_getter!("crypto_generichash_keybytes_min"), 16);
    assert_eq!(chk_getter!("crypto_generichash_keybytes_max"), 64);
    assert_eq!(chk_getter!("crypto_generichash_keybytes"), 32);
    assert_eq!(chk_getter!("crypto_generichash_statebytes"), 384);
    chk_str!("crypto_generichash_primitive");

    // crypto_generichash/blake2b/generichash_blake2.c
    assert_eq!(chk_getter!("crypto_generichash_blake2b_bytes_min"), 16);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_bytes_max"), 64);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_bytes"), 32);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_keybytes_min"), 16);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_keybytes_max"), 64);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_keybytes"), 32);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_saltbytes"), 16);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_personalbytes"), 16);
    assert_eq!(chk_getter!("crypto_generichash_blake2b_statebytes"), 384);

    // crypto_kdf
    assert_eq!(chk_getter!("crypto_kdf_bytes_min"), 16);
    assert_eq!(chk_getter!("crypto_kdf_bytes_max"), 64);
    assert_eq!(chk_getter!("crypto_kdf_contextbytes"), 8);
    assert_eq!(chk_getter!("crypto_kdf_keybytes"), 32);
    chk_str!("crypto_kdf_primitive");
    assert_eq!(chk_getter!("crypto_kdf_blake2b_bytes_min"), 16);
    assert_eq!(chk_getter!("crypto_kdf_blake2b_bytes_max"), 64);
    assert_eq!(chk_getter!("crypto_kdf_blake2b_contextbytes"), 8);
    assert_eq!(chk_getter!("crypto_kdf_blake2b_keybytes"), 32);

    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha256_keybytes"), 32);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha256_bytes_min"), 0);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha256_bytes_max"), 0xff * 32);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha256_statebytes"), 208);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha512_keybytes"), 64);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha512_bytes_min"), 0);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha512_bytes_max"), 0xff * 64);
    assert_eq!(chk_getter!("crypto_kdf_hkdf_sha512_statebytes"), 416);

    // crypto_shorthash
    assert_eq!(chk_getter!("crypto_shorthash_bytes"), 8);
    assert_eq!(chk_getter!("crypto_shorthash_keybytes"), 16);
    chk_str!("crypto_shorthash_primitive");
    assert_eq!(chk_getter!("crypto_shorthash_siphash24_bytes"), 8);
    assert_eq!(chk_getter!("crypto_shorthash_siphash24_keybytes"), 16);
    assert_eq!(chk_getter!("crypto_shorthash_siphashx24_bytes"), 16);
    assert_eq!(chk_getter!("crypto_shorthash_siphashx24_keybytes"), 16);

    // crypto_verify
    assert_eq!(chk_getter!("crypto_verify_16_bytes"), 16);
    assert_eq!(chk_getter!("crypto_verify_32_bytes"), 32);
    assert_eq!(chk_getter!("crypto_verify_64_bytes"), 64);
}

// =========================================== blake2-2: generichash one-shot ==

#[test]
fn generichash_oneshot() {
    let (cg, rg) = both!("crypto_generichash", FnGh);
    let (cb, rb) = both!("crypto_generichash_blake2b", FnGh);
    let mut rng = common::Rng::new(0x1111_2222_3333_4444);

    // input lengths straddling the 128-byte block and the 256-byte internal buffer
    let inlens = [
        0usize, 1, 2, 7, 8, 63, 64, 127, 128, 129, 191, 192, 255, 256, 257, 383, 384, 385, 1000,
        4096,
    ];
    for &keylen in &[0usize, 1, 15, 16, 31, 32, 33, 63, 64] {
        for &outlen in &[1usize, 15, 16, 17, 31, 32, 33, 63, 64] {
            for &n in &inlens {
                let msg = rng.bytes(n);
                let key = rng.bytes(if keylen == 0 { 1 } else { keylen });
                let kp = if keylen == 0 {
                    core::ptr::null()
                } else {
                    key.as_ptr()
                };
                let ctx = format!("o={outlen} k={keylen} n={n}");
                let (mut co, mut ro) = (canary(outlen), canary(outlen));
                let rc = unsafe { cg(co.as_mut_ptr(), outlen, msg.as_ptr(), n as u64, kp, keylen) };
                let rr = unsafe { rg(ro.as_mut_ptr(), outlen, msg.as_ptr(), n as u64, kp, keylen) };
                common::eqi(&format!("crypto_generichash {ctx}"), rc, rr);
                common::eqb(&format!("crypto_generichash {ctx}"), &co, &ro);
                assert!(
                    co[outlen..].iter().all(|&b| b == CANARY),
                    "write past outlen ({ctx})"
                );

                let (mut co, mut ro) = (canary(outlen), canary(outlen));
                let rc = unsafe { cb(co.as_mut_ptr(), outlen, msg.as_ptr(), n as u64, kp, keylen) };
                let rr = unsafe { rb(ro.as_mut_ptr(), outlen, msg.as_ptr(), n as u64, kp, keylen) };
                common::eqi(&format!("blake2b {ctx}"), rc, rr);
                common::eqb(&format!("blake2b {ctx}"), &co, &ro);
            }
        }
    }
}

/// `key != NULL` but `keylen == 0` must behave like the unkeyed path.
#[test]
fn generichash_nonnull_key_zero_len() {
    let (cb, rb) = both!("crypto_generichash_blake2b", FnGh);
    let mut rng = common::Rng::new(0xDEAD_BEEF);
    for n in [0usize, 1, 128, 300] {
        let msg = rng.bytes(n);
        let key = rng.bytes(32);
        let (mut co, mut ro) = (canary(32), canary(32));
        let rc = unsafe { cb(co.as_mut_ptr(), 32, msg.as_ptr(), n as u64, key.as_ptr(), 0) };
        let rr = unsafe { rb(ro.as_mut_ptr(), 32, msg.as_ptr(), n as u64, key.as_ptr(), 0) };
        common::eqi("nonnull key keylen=0", rc, rr);
        common::eqb("nonnull key keylen=0", &co, &ro);
        // identical to key = NULL
        let mut oo = canary(32);
        unsafe { cb(oo.as_mut_ptr(), 32, msg.as_ptr(), n as u64, core::ptr::null(), 0) };
        common::eqb("keylen=0 ignores key pointer", &oo, &co);
    }
}

#[test]
fn generichash_oneshot_errors() {
    let (cg, rg) = both!("crypto_generichash", FnGh);
    let (cb, rb) = both!("crypto_generichash_blake2b", FnGh);
    let (cs, rs) = both!("crypto_generichash_blake2b_salt_personal", FnGhSp);
    let mut rng = common::Rng::new(7);
    let msg = rng.bytes(64);
    let key = rng.bytes(64);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    // outlen out of range: 0, 65, 255, 256, huge
    for &outlen in &[0usize, 65, 100, 255, 256, 1000, usize::MAX] {
        let (mut co, mut ro) = (canary(8), canary(8));
        let rc = unsafe { cg(co.as_mut_ptr(), outlen, msg.as_ptr(), 64, core::ptr::null(), 0) };
        let rr = unsafe { rg(ro.as_mut_ptr(), outlen, msg.as_ptr(), 64, core::ptr::null(), 0) };
        common::eqi(&format!("generichash bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1, "outlen={outlen} should be rejected");
        common::eqb("bad outlen buffer untouched", &co, &ro);

        let rc = unsafe { cb(co.as_mut_ptr(), outlen, msg.as_ptr(), 64, core::ptr::null(), 0) };
        let rr = unsafe { rb(ro.as_mut_ptr(), outlen, msg.as_ptr(), 64, core::ptr::null(), 0) };
        common::eqi(&format!("blake2b bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1);

        let rc = unsafe {
            cs(
                co.as_mut_ptr(),
                outlen,
                msg.as_ptr(),
                64,
                core::ptr::null(),
                0,
                salt.as_ptr(),
                pers.as_ptr(),
            )
        };
        let rr = unsafe {
            rs(
                ro.as_mut_ptr(),
                outlen,
                msg.as_ptr(),
                64,
                core::ptr::null(),
                0,
                salt.as_ptr(),
                pers.as_ptr(),
            )
        };
        common::eqi(&format!("salt_personal bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1);
    }

    // keylen out of range: 65 .. huge (checked before the key==NULL misuse test)
    for &keylen in &[65usize, 100, 255, 256, 1000, usize::MAX] {
        let (mut co, mut ro) = (canary(32), canary(32));
        let rc = unsafe { cg(co.as_mut_ptr(), 32, msg.as_ptr(), 64, key.as_ptr(), keylen) };
        let rr = unsafe { rg(ro.as_mut_ptr(), 32, msg.as_ptr(), 64, key.as_ptr(), keylen) };
        common::eqi(&format!("generichash bad keylen={keylen}"), rc, rr);
        assert_eq!(rc, -1);
        common::eqb("bad keylen buffer untouched", &co, &ro);

        let rc = unsafe {
            cs(
                co.as_mut_ptr(),
                32,
                msg.as_ptr(),
                64,
                key.as_ptr(),
                keylen,
                salt.as_ptr(),
                pers.as_ptr(),
            )
        };
        let rr = unsafe {
            rs(
                ro.as_mut_ptr(),
                32,
                msg.as_ptr(),
                64,
                key.as_ptr(),
                keylen,
                salt.as_ptr(),
                pers.as_ptr(),
            )
        };
        common::eqi(&format!("salt_personal bad keylen={keylen}"), rc, rr);
        assert_eq!(rc, -1);
    }

    // in == NULL with inlen == 0 is fine (the `in == NULL && inlen > 0` misuse
    // check does not fire); in == NULL with inlen > 0 calls sodium_misuse() and
    // is therefore not testable in-process.
    let (mut co, mut ro) = (canary(32), canary(32));
    let rc = unsafe { cb(co.as_mut_ptr(), 32, core::ptr::null(), 0, core::ptr::null(), 0) };
    let rr = unsafe { rb(ro.as_mut_ptr(), 32, core::ptr::null(), 0, core::ptr::null(), 0) };
    common::eqi("in=NULL inlen=0", rc, rr);
    assert_eq!(rc, 0);
    common::eqb("in=NULL inlen=0", &co, &ro);
}

// ========================== blake2-3: salt/personal cross-product ============

#[test]
fn generichash_salt_personal_matrix() {
    let (c, r) = both!("crypto_generichash_blake2b_salt_personal", FnGhSp);
    let mut rng = common::Rng::new(0x5A5A_0F0F_1234_5678);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);
    let zeros = [0u8; 16];

    for &(sp, pp, tag) in &[
        (0usize, 0usize, "salt=NULL personal=NULL"),
        (1, 0, "salt=set personal=NULL"),
        (0, 1, "salt=NULL personal=set"),
        (1, 1, "salt=set personal=set"),
        (2, 2, "salt=zeros personal=zeros"),
    ] {
        let sptr = match sp {
            0 => core::ptr::null(),
            1 => salt.as_ptr(),
            _ => zeros.as_ptr(),
        };
        let pptr = match pp {
            0 => core::ptr::null(),
            1 => pers.as_ptr(),
            _ => zeros.as_ptr(),
        };
        for &keylen in &[0usize, 1, 16, 32, 64] {
            for &outlen in &[1usize, 16, 32, 64] {
                for &n in &[0usize, 1, 64, 128, 129, 256, 257, 1000] {
                    let msg = rng.bytes(n);
                    let key = rng.bytes(if keylen == 0 { 1 } else { keylen });
                    let kp = if keylen == 0 {
                        core::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let (mut co, mut ro) = (canary(outlen), canary(outlen));
                    let rc = unsafe {
                        c(
                            co.as_mut_ptr(),
                            outlen,
                            msg.as_ptr(),
                            n as u64,
                            kp,
                            keylen,
                            sptr,
                            pptr,
                        )
                    };
                    let rr = unsafe {
                        r(
                            ro.as_mut_ptr(),
                            outlen,
                            msg.as_ptr(),
                            n as u64,
                            kp,
                            keylen,
                            sptr,
                            pptr,
                        )
                    };
                    let ctx = format!("sp[{tag}] o={outlen} k={keylen} n={n}");
                    common::eqi(&ctx, rc, rr);
                    common::eqb(&ctx, &co, &ro);
                }
            }
        }
    }
    // NULL salt/personal must be equivalent to all-zero salt/personal
    let mut a = canary(32);
    let mut b = canary(32);
    let msg = rng.bytes(100);
    unsafe {
        c(
            a.as_mut_ptr(),
            32,
            msg.as_ptr(),
            100,
            core::ptr::null(),
            0,
            core::ptr::null(),
            core::ptr::null(),
        )
    };
    unsafe {
        c(
            b.as_mut_ptr(),
            32,
            msg.as_ptr(),
            100,
            core::ptr::null(),
            0,
            zeros.as_ptr(),
            zeros.as_ptr(),
        )
    };
    common::eqb("NULL salt/personal == zeros", &a, &b);
}

// ============================= blake2-4: streaming generichash ===============

#[test]
fn generichash_streaming() {
    let sb = chk_getter!("crypto_generichash_statebytes");
    let (ci, ri) = both!("crypto_generichash_init", FnGhInit);
    let (cu, ru) = both!("crypto_generichash_update", FnGhUpdate);
    let (cf, rf) = both!("crypto_generichash_final", FnGhFinal);
    let (cb, _rb) = both!("crypto_generichash_blake2b", FnGh);
    let mut rng = common::Rng::new(0xABCD_EF01_2345_6789);

    for &keylen in &[0usize, 1, 16, 32, 64] {
        for &outlen in &[1usize, 16, 32, 64] {
            for &n in &[
                0usize, 1, 63, 64, 127, 128, 129, 191, 192, 255, 256, 257, 300, 512, 1000, 4096,
            ] {
                let msg = rng.bytes(n);
                let key = rng.bytes(if keylen == 0 { 1 } else { keylen });
                let kp = if keylen == 0 {
                    core::ptr::null()
                } else {
                    key.as_ptr()
                };
                let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
                let rc = unsafe { ci(cst.p(), kp, keylen, outlen) };
                let rr = unsafe { ri(rst.p(), kp, keylen, outlen) };
                let ctx = format!("stream o={outlen} k={keylen} n={n}");
                common::eqi(&ctx, rc, rr);
                // The whole opaque state must be byte-identical: the C
                // `blake2b_state` is `#pragma pack(1)`-ed so it has no padding
                // holes, and the tail of the 384-byte opaque buffer is never
                // written by either side (both start from a zeroed buffer).
                common::eqb(&format!("{ctx}: state after init"), cst.s(sb), rst.s(sb));

                let mut off = 0usize;
                for k in splits(&mut rng, n) {
                    let p = if n == 0 {
                        core::ptr::null()
                    } else {
                        unsafe { msg.as_ptr().add(off) }
                    };
                    let rc = unsafe { cu(cst.p(), p, k as u64) };
                    let rr = unsafe { ru(rst.p(), p, k as u64) };
                    common::eqi(&format!("{ctx}: update {k}"), rc, rr);
                    common::eqb(
                        &format!("{ctx}: state after update {k} at {off}"),
                        cst.s(sb),
                        rst.s(sb),
                    );
                    off += k;
                }
                assert_eq!(off, n);

                let (mut co, mut ro) = (canary(outlen), canary(outlen));
                let rc = unsafe { cf(cst.p(), co.as_mut_ptr(), outlen) };
                let rr = unsafe { rf(rst.p(), ro.as_mut_ptr(), outlen) };
                common::eqi(&format!("{ctx}: final"), rc, rr);
                common::eqb(&format!("{ctx}: digest"), &co, &ro);
                common::eqb(&format!("{ctx}: state after final"), cst.s(sb), rst.s(sb));

                // cross-check against the one-shot API
                let mut oo = canary(outlen);
                unsafe { cb(oo.as_mut_ptr(), outlen, msg.as_ptr(), n as u64, kp, keylen) };
                common::eqb(&format!("{ctx}: stream == oneshot"), &oo, &co);
            }
        }
    }
}

#[test]
fn generichash_blake2b_streaming_and_init_salt_personal() {
    let sb = chk_getter!("crypto_generichash_blake2b_statebytes");
    let (ci, ri) = both!("crypto_generichash_blake2b_init", FnGhInit);
    let (cisp, risp) = both!("crypto_generichash_blake2b_init_salt_personal", FnGhInitSp);
    let (cu, ru) = both!("crypto_generichash_blake2b_update", FnGhUpdate);
    let (cf, rf) = both!("crypto_generichash_blake2b_final", FnGhFinal);
    let (csp, _rsp) = both!("crypto_generichash_blake2b_salt_personal", FnGhSp);
    let mut rng = common::Rng::new(0x0F1E_2D3C_4B5A_6978);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    // plain init/update/final with randomly-split updates
    for &keylen in &[0usize, 16, 32, 64] {
        for &outlen in &[1usize, 16, 32, 64] {
            for &n in &[0usize, 1, 128, 256, 257, 999] {
                let msg = rng.bytes(n);
                let key = rng.bytes(if keylen == 0 { 1 } else { keylen });
                let kp = if keylen == 0 {
                    core::ptr::null()
                } else {
                    key.as_ptr()
                };
                let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
                common::eqi("b2b_init", unsafe { ci(cst.p(), kp, keylen, outlen) }, unsafe {
                    ri(rst.p(), kp, keylen, outlen)
                });
                common::eqb("b2b_init state", cst.s(sb), rst.s(sb));
                let mut off = 0usize;
                for k in splits(&mut rng, n) {
                    let p = if n == 0 {
                        core::ptr::null()
                    } else {
                        unsafe { msg.as_ptr().add(off) }
                    };
                    common::eqi("b2b_update", unsafe { cu(cst.p(), p, k as u64) }, unsafe {
                        ru(rst.p(), p, k as u64)
                    });
                    common::eqb("b2b_update state", cst.s(sb), rst.s(sb));
                    off += k;
                }
                let (mut co, mut ro) = (canary(outlen), canary(outlen));
                common::eqi(
                    "b2b_final",
                    unsafe { cf(cst.p(), co.as_mut_ptr(), outlen) },
                    unsafe { rf(rst.p(), ro.as_mut_ptr(), outlen) },
                );
                common::eqb("b2b digest", &co, &ro);
                common::eqb("b2b state after final", cst.s(sb), rst.s(sb));
            }
        }
    }

    // init_salt_personal, all four NULL/non-NULL combinations, keyed & unkeyed
    for &(sp, pp) in &[(false, false), (true, false), (false, true), (true, true)] {
        let sptr = if sp { salt.as_ptr() } else { core::ptr::null() };
        let pptr = if pp { pers.as_ptr() } else { core::ptr::null() };
        for &keylen in &[0usize, 1, 16, 32, 64] {
            for &outlen in &[1usize, 16, 32, 64] {
                for &n in &[0usize, 1, 128, 256, 257, 700] {
                    let msg = rng.bytes(n);
                    let key = rng.bytes(if keylen == 0 { 1 } else { keylen });
                    let kp = if keylen == 0 {
                        core::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
                    let ctx = format!("init_sp s={sp} p={pp} k={keylen} o={outlen} n={n}");
                    common::eqi(
                        &ctx,
                        unsafe { cisp(cst.p(), kp, keylen, outlen, sptr, pptr) },
                        unsafe { risp(rst.p(), kp, keylen, outlen, sptr, pptr) },
                    );
                    common::eqb(&format!("{ctx}: state"), cst.s(sb), rst.s(sb));
                    let mut off = 0usize;
                    for k in splits(&mut rng, n) {
                        let p = if n == 0 {
                            core::ptr::null()
                        } else {
                            unsafe { msg.as_ptr().add(off) }
                        };
                        unsafe { cu(cst.p(), p, k as u64) };
                        unsafe { ru(rst.p(), p, k as u64) };
                        common::eqb(&format!("{ctx}: state upd {k}@{off}"), cst.s(sb), rst.s(sb));
                        off += k;
                    }
                    let (mut co, mut ro) = (canary(outlen), canary(outlen));
                    common::eqi(
                        &format!("{ctx}: final"),
                        unsafe { cf(cst.p(), co.as_mut_ptr(), outlen) },
                        unsafe { rf(rst.p(), ro.as_mut_ptr(), outlen) },
                    );
                    common::eqb(&format!("{ctx}: digest"), &co, &ro);

                    // must equal the one-shot salt/personal API
                    let mut oo = canary(outlen);
                    unsafe {
                        csp(
                            oo.as_mut_ptr(),
                            outlen,
                            msg.as_ptr(),
                            n as u64,
                            kp,
                            keylen,
                            sptr,
                            pptr,
                        )
                    };
                    common::eqb(&format!("{ctx}: stream == oneshot"), &oo, &co);
                }
            }
        }
    }
}

#[test]
fn generichash_init_errors_and_double_final() {
    let sb = 384usize;
    let (ci, ri) = both!("crypto_generichash_blake2b_init", FnGhInit);
    let (cisp, risp) = both!("crypto_generichash_blake2b_init_salt_personal", FnGhInitSp);
    let (cgi, rgi) = both!("crypto_generichash_init", FnGhInit);
    let (cu, ru) = both!("crypto_generichash_blake2b_update", FnGhUpdate);
    let (cf, rf) = both!("crypto_generichash_blake2b_final", FnGhFinal);
    let mut rng = common::Rng::new(31337);
    let key = rng.bytes(64);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    // outlen == 0 / > 64 => -1, state untouched
    for &outlen in &[0usize, 65, 100, 255, 256, usize::MAX] {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        let rc = unsafe { ci(cst.p(), core::ptr::null(), 0, outlen) };
        let rr = unsafe { ri(rst.p(), core::ptr::null(), 0, outlen) };
        common::eqi(&format!("init bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1);
        common::eqb("init bad outlen: state untouched", cst.s(sb), rst.s(sb));
        assert!(cst.0.iter().all(|&b| b == 0), "state must stay zeroed");

        let rc = unsafe { cgi(cst.p(), core::ptr::null(), 0, outlen) };
        let rr = unsafe { rgi(rst.p(), core::ptr::null(), 0, outlen) };
        common::eqi(&format!("generichash_init bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1);

        let rc =
            unsafe { cisp(cst.p(), core::ptr::null(), 0, outlen, salt.as_ptr(), pers.as_ptr()) };
        let rr =
            unsafe { risp(rst.p(), core::ptr::null(), 0, outlen, salt.as_ptr(), pers.as_ptr()) };
        common::eqi(&format!("init_sp bad outlen={outlen}"), rc, rr);
        assert_eq!(rc, -1);
    }

    // keylen > 64 => -1
    for &keylen in &[65usize, 100, 255, 256, usize::MAX] {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        let rc = unsafe { ci(cst.p(), key.as_ptr(), keylen, 32) };
        let rr = unsafe { ri(rst.p(), key.as_ptr(), keylen, 32) };
        common::eqi(&format!("init bad keylen={keylen}"), rc, rr);
        assert_eq!(rc, -1);
        common::eqb("init bad keylen: state untouched", cst.s(sb), rst.s(sb));

        let rc = unsafe { cisp(cst.p(), key.as_ptr(), keylen, 32, salt.as_ptr(), pers.as_ptr()) };
        let rr = unsafe { risp(rst.p(), key.as_ptr(), keylen, 32, salt.as_ptr(), pers.as_ptr()) };
        common::eqi(&format!("init_sp bad keylen={keylen}"), rc, rr);
        assert_eq!(rc, -1);
    }

    // key == NULL with keylen > 0: `key == NULL || keylen <= 0` selects the
    // *unkeyed* path (no misuse). Result must equal keylen == 0.
    for &keylen in &[1usize, 16, 32, 64] {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        let rc = unsafe { ci(cst.p(), core::ptr::null(), keylen, 32) };
        let rr = unsafe { ri(rst.p(), core::ptr::null(), keylen, 32) };
        common::eqi("init key=NULL keylen>0", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("init key=NULL keylen>0 state", cst.s(sb), rst.s(sb));

        let mut ref_st = AlignedState::new();
        unsafe { ci(ref_st.p(), core::ptr::null(), 0, 32) };
        common::eqb("key=NULL keylen>0 == unkeyed", ref_st.s(sb), cst.s(sb));

        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        let rc =
            unsafe { cisp(cst.p(), core::ptr::null(), keylen, 32, salt.as_ptr(), pers.as_ptr()) };
        let rr =
            unsafe { risp(rst.p(), core::ptr::null(), keylen, 32, salt.as_ptr(), pers.as_ptr()) };
        common::eqi("init_sp key=NULL keylen>0", rc, rr);
        assert_eq!(rc, 0);
        common::eqb("init_sp key=NULL keylen>0 state", cst.s(sb), rst.s(sb));
    }

    // final twice: the second call sees f[0] != 0 (last block) and returns -1
    let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
    unsafe { ci(cst.p(), core::ptr::null(), 0, 32) };
    unsafe { ri(rst.p(), core::ptr::null(), 0, 32) };
    let msg = rng.bytes(200);
    unsafe { cu(cst.p(), msg.as_ptr(), 200) };
    unsafe { ru(rst.p(), msg.as_ptr(), 200) };
    let (mut co, mut ro) = (canary(32), canary(32));
    assert_eq!(unsafe { cf(cst.p(), co.as_mut_ptr(), 32) }, 0);
    assert_eq!(unsafe { rf(rst.p(), ro.as_mut_ptr(), 32) }, 0);
    common::eqb("first final", &co, &ro);
    let (mut co2, mut ro2) = (canary(32), canary(32));
    let rc = unsafe { cf(cst.p(), co2.as_mut_ptr(), 32) };
    let rr = unsafe { rf(rst.p(), ro2.as_mut_ptr(), 32) };
    common::eqi("second final", rc, rr);
    assert_eq!(rc, -1, "second final must return -1");
    common::eqb("second final leaves buffer untouched", &co2, &ro2);
    common::eqb("state after second final", cst.s(sb), rst.s(sb));

    // update after final: state has f[0] != 0; update still buffers and
    // returns 0, and the resulting states must match.
    let rc = unsafe { cu(cst.p(), msg.as_ptr(), 10) };
    let rr = unsafe { ru(rst.p(), msg.as_ptr(), 10) };
    common::eqi("update after final", rc, rr);
    common::eqb("state after update-after-final", cst.s(sb), rst.s(sb));

    // update with in == NULL, inlen == 0 on a fresh state
    let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
    unsafe { ci(cst.p(), core::ptr::null(), 0, 32) };
    unsafe { ri(rst.p(), core::ptr::null(), 0, 32) };
    let rc = unsafe { cu(cst.p(), core::ptr::null(), 0) };
    let rr = unsafe { ru(rst.p(), core::ptr::null(), 0) };
    common::eqi("update NULL/0", rc, rr);
    common::eqb("state after update NULL/0", cst.s(sb), rst.s(sb));
}

// ======================== blake2-5: low-level `_sodium_blake2b*` =============

#[test]
fn low_level_blake2b_init_variants() {
    let sb = 384usize;
    let (ci, ri) = both!("_sodium_blake2b_init", FnB2Init);
    let (cisp, risp) = both!("_sodium_blake2b_init_salt_personal", FnB2InitSp);
    let (cik, rik) = both!("_sodium_blake2b_init_key", FnB2InitKey);
    let (ciksp, riksp) = both!("_sodium_blake2b_init_key_salt_personal", FnB2InitKeySp);
    let (cip, rip) = both!("_sodium_blake2b_init_param", FnB2InitParam);
    let (cu, ru) = both!("_sodium_blake2b_update", FnB2Update);
    let (cf, rf) = both!("_sodium_blake2b_final", FnB2Final);
    let mut rng = common::Rng::new(0x2468_ACE0_1357_9BDF);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    // blake2b_init: every valid outlen 1..=64
    for outlen in 1u8..=64 {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        common::eqi(
            &format!("_sodium_blake2b_init o={outlen}"),
            unsafe { ci(cst.p(), outlen) },
            unsafe { ri(rst.p(), outlen) },
        );
        common::eqb(&format!("init o={outlen} state"), cst.s(sb), rst.s(sb));
    }

    // blake2b_init_salt_personal: 4 NULL combinations
    for &(sp, pp) in &[(false, false), (true, false), (false, true), (true, true)] {
        let sptr = if sp {
            salt.as_ptr() as *const c_void
        } else {
            core::ptr::null()
        };
        let pptr = if pp {
            pers.as_ptr() as *const c_void
        } else {
            core::ptr::null()
        };
        for &outlen in &[1u8, 16, 32, 64] {
            let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
            common::eqi(
                "init_salt_personal",
                unsafe { cisp(cst.p(), outlen, sptr, pptr) },
                unsafe { risp(rst.p(), outlen, sptr, pptr) },
            );
            common::eqb(
                &format!("init_sp s={sp} p={pp} o={outlen} state"),
                cst.s(sb),
                rst.s(sb),
            );
        }
    }

    // blake2b_init_key / init_key_salt_personal: keylen 1..=64
    for keylen in 1u8..=64 {
        let key = rng.bytes(keylen as usize);
        for &outlen in &[1u8, 32, 64] {
            let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
            common::eqi(
                "init_key",
                unsafe { cik(cst.p(), outlen, key.as_ptr() as *const c_void, keylen) },
                unsafe { rik(rst.p(), outlen, key.as_ptr() as *const c_void, keylen) },
            );
            common::eqb(
                &format!("init_key k={keylen} o={outlen} state"),
                cst.s(sb),
                rst.s(sb),
            );

            let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
            common::eqi(
                "init_key_salt_personal",
                unsafe {
                    ciksp(
                        cst.p(),
                        outlen,
                        key.as_ptr() as *const c_void,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    )
                },
                unsafe {
                    riksp(
                        rst.p(),
                        outlen,
                        key.as_ptr() as *const c_void,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    )
                },
            );
            common::eqb(
                &format!("init_key_sp k={keylen} o={outlen} state"),
                cst.s(sb),
                rst.s(sb),
            );

            // NULL salt/personal variant
            let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
            unsafe {
                ciksp(
                    cst.p(),
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    core::ptr::null(),
                    core::ptr::null(),
                )
            };
            unsafe {
                riksp(
                    rst.p(),
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    core::ptr::null(),
                    core::ptr::null(),
                )
            };
            common::eqb("init_key_sp NULL salt/personal", cst.s(sb), rst.s(sb));
        }
    }

    // blake2b_init_param with fully random 64-byte parameter blocks: the only
    // way to exercise fanout/depth/leaf_length/node_offset/node_depth/
    // inner_length/reserved values other than the ones the wrappers hard-code.
    for i in 0..40 {
        let p = rng.bytes(64);
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        common::eqi("init_param", unsafe { cip(cst.p(), p.as_ptr()) }, unsafe {
            rip(rst.p(), p.as_ptr())
        });
        common::eqb(&format!("init_param #{i} state"), cst.s(sb), rst.s(sb));
        let n = rng.below(600);
        let msg = rng.bytes(n);
        unsafe { cu(cst.p(), msg.as_ptr(), n as u64) };
        unsafe { ru(rst.p(), msg.as_ptr(), n as u64) };
        common::eqb(&format!("init_param #{i} after update"), cst.s(sb), rst.s(sb));
        let outlen = 1 + (p[0] % 64);
        let (mut co, mut ro) = (canary(outlen as usize), canary(outlen as usize));
        common::eqi(
            "init_param final",
            unsafe { cf(cst.p(), co.as_mut_ptr(), outlen) },
            unsafe { rf(rst.p(), ro.as_mut_ptr(), outlen) },
        );
        common::eqb(&format!("init_param #{i} digest"), &co, &ro);
    }
}

/// `blake2b_set_lastnode()` is only reached when `S->last_node != 0`, which no
/// public entry point sets. Poke the byte directly (offset 360 of the packed
/// `blake2b_state`) to cover it.
#[test]
fn low_level_blake2b_last_node() {
    let sb = 384usize;
    let (ci, ri) = both!("_sodium_blake2b_init", FnB2Init);
    let (cu, ru) = both!("_sodium_blake2b_update", FnB2Update);
    let (cf, rf) = both!("_sodium_blake2b_final", FnB2Final);
    let mut rng = common::Rng::new(0x777);
    for &n in &[0usize, 1, 128, 257, 600] {
        let msg = rng.bytes(n);
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        unsafe { ci(cst.p(), 32) };
        unsafe { ri(rst.p(), 32) };
        cst.0[360] = 1;
        rst.0[360] = 1;
        unsafe { cu(cst.p(), msg.as_ptr(), n as u64) };
        unsafe { ru(rst.p(), msg.as_ptr(), n as u64) };
        common::eqb("last_node state after update", cst.s(sb), rst.s(sb));
        let (mut co, mut ro) = (canary(32), canary(32));
        common::eqi(
            "last_node final",
            unsafe { cf(cst.p(), co.as_mut_ptr(), 32) },
            unsafe { rf(rst.p(), ro.as_mut_ptr(), 32) },
        );
        common::eqb(&format!("last_node n={n} digest"), &co, &ro);
        common::eqb("last_node state after final", cst.s(sb), rst.s(sb));
    }
}

#[test]
fn low_level_blake2b_oneshot() {
    let (cb, rb) = both!("_sodium_blake2b", FnB2);
    let (cs, rs) = both!("_sodium_blake2b_salt_personal", FnB2Sp);
    let (cgh, _) = both!("crypto_generichash_blake2b", FnGh);
    let mut rng = common::Rng::new(0x1BAD_B002);
    let salt = rng.bytes(16);
    let pers = rng.bytes(16);

    for &keylen in &[0u8, 1, 16, 32, 64] {
        for &outlen in &[1u8, 16, 32, 64] {
            for &n in &[0usize, 1, 127, 128, 129, 256, 257, 1024] {
                let msg = rng.bytes(n);
                let key = rng.bytes(if keylen == 0 { 1 } else { keylen as usize });
                let kp = if keylen == 0 {
                    core::ptr::null()
                } else {
                    key.as_ptr() as *const c_void
                };
                let ip = if n == 0 {
                    core::ptr::null()
                } else {
                    msg.as_ptr() as *const c_void
                };
                let (mut co, mut ro) = (canary(outlen as usize), canary(outlen as usize));
                let ctx = format!("_sodium_blake2b k={keylen} o={outlen} n={n}");
                common::eqi(
                    &ctx,
                    unsafe { cb(co.as_mut_ptr(), ip, kp, outlen, n as u64, keylen) },
                    unsafe { rb(ro.as_mut_ptr(), ip, kp, outlen, n as u64, keylen) },
                );
                common::eqb(&ctx, &co, &ro);
                // must equal the public wrapper
                let mut oo = canary(outlen as usize);
                unsafe {
                    cgh(
                        oo.as_mut_ptr(),
                        outlen as usize,
                        if n == 0 { core::ptr::null() } else { msg.as_ptr() },
                        n as u64,
                        if keylen == 0 {
                            core::ptr::null()
                        } else {
                            key.as_ptr()
                        },
                        keylen as usize,
                    )
                };
                common::eqb(&format!("{ctx} == crypto_generichash_blake2b"), &oo, &co);

                for &(sp, pp) in &[(false, false), (true, false), (false, true), (true, true)] {
                    let sptr = if sp {
                        salt.as_ptr() as *const c_void
                    } else {
                        core::ptr::null()
                    };
                    let pptr = if pp {
                        pers.as_ptr() as *const c_void
                    } else {
                        core::ptr::null()
                    };
                    let (mut co, mut ro) = (canary(outlen as usize), canary(outlen as usize));
                    let ctx =
                        format!("_sodium_blake2b_sp s={sp} p={pp} k={keylen} o={outlen} n={n}");
                    common::eqi(
                        &ctx,
                        unsafe {
                            cs(co.as_mut_ptr(), ip, kp, outlen, n as u64, keylen, sptr, pptr)
                        },
                        unsafe {
                            rs(ro.as_mut_ptr(), ip, kp, outlen, n as u64, keylen, sptr, pptr)
                        },
                    );
                    common::eqb(&ctx, &co, &ro);
                }
            }
        }
    }
}

/// `blake2b_compress_ref` reads `S->h`, `S->t`, `S->f` and writes `S->h`, so a
/// fully random state buffer (identical on both sides) exercises it completely,
/// including carry/flag words that the streaming API never produces.
#[test]
fn low_level_blake2b_compress_ref() {
    let sb = 384usize;
    let (cc, rc_) = both!("_sodium_blake2b_compress_ref", FnB2Compress);
    let mut rng = common::Rng::new(0xC0DE_C0DE);

    for i in 0..64 {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        let seed = rng.bytes(sb);
        cst.0[..sb].copy_from_slice(&seed);
        rst.0[..sb].copy_from_slice(&seed);
        let block = rng.bytes(128);
        common::eqi(
            "compress_ref",
            unsafe { cc(cst.p(), block.as_ptr()) },
            unsafe { rc_(rst.p(), block.as_ptr()) },
        );
        common::eqb(&format!("compress_ref #{i} state"), cst.s(sb), rst.s(sb));
    }

    // and the degenerate all-zero / all-ones states
    for fill in [0u8, 0xff] {
        let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
        for j in 0..sb {
            cst.0[j] = fill;
            rst.0[j] = fill;
        }
        let block = rng.bytes(128);
        common::eqi(
            "compress_ref fill",
            unsafe { cc(cst.p(), block.as_ptr()) },
            unsafe { rc_(rst.p(), block.as_ptr()) },
        );
        common::eqb(&format!("compress_ref fill={fill:#x}"), cst.s(sb), rst.s(sb));
    }
}

#[test]
fn pick_best_implementation() {
    let (c1, r1) = both!("_sodium_blake2b_pick_best_implementation", FnVoidInt);
    let (c2, r2) = both!(
        "_crypto_generichash_blake2b_pick_best_implementation",
        FnVoidInt
    );
    common::eqi("_sodium_blake2b_pick_best_implementation", unsafe { c1() }, unsafe {
        r1()
    });
    assert_eq!(unsafe { c1() }, 0);
    common::eqi(
        "_crypto_generichash_blake2b_pick_best_implementation",
        unsafe { c2() },
        unsafe { r2() },
    );
    assert_eq!(unsafe { c2() }, 0);

    // hashing must still work (and agree) after re-selecting the implementation
    let (cb, rb) = both!("crypto_generichash_blake2b", FnGh);
    let mut rng = common::Rng::new(9);
    for n in [0usize, 1, 130, 900] {
        let msg = rng.bytes(n);
        let (mut co, mut ro) = (canary(32), canary(32));
        unsafe { cb(co.as_mut_ptr(), 32, msg.as_ptr(), n as u64, core::ptr::null(), 0) };
        unsafe { rb(ro.as_mut_ptr(), 32, msg.as_ptr(), n as u64, core::ptr::null(), 0) };
        common::eqb("after pick_best", &co, &ro);
    }
}

// ============================== blake2-6: blake2b_long =======================

#[test]
fn blake2b_long() {
    let (c, r) = both!("_sodium_blake2b_long", FnB2Long);
    let mut rng = common::Rng::new(0xB10B_10B1);
    // 0     -> rejected (crypto_generichash_blake2b_init with outlen 0)
    // <= 64 -> single-pass branch
    // > 64  -> chained branch (with and without the `toproduce > 64` loop)
    for &outlen in &[
        0usize, 1, 16, 32, 63, 64, 65, 66, 95, 96, 97, 127, 128, 129, 160, 192, 200, 1000,
    ] {
        for &n in &[0usize, 1, 64, 128, 257, 1000] {
            let msg = rng.bytes(n);
            let (mut co, mut ro) = (canary(outlen), canary(outlen));
            let ip = if n == 0 {
                core::ptr::null()
            } else {
                msg.as_ptr() as *const c_void
            };
            let rc = unsafe { c(co.as_mut_ptr() as *mut c_void, outlen, ip, n) };
            let rr = unsafe { r(ro.as_mut_ptr() as *mut c_void, outlen, ip, n) };
            common::eqi(&format!("blake2b_long o={outlen} n={n}"), rc, rr);
            common::eqb(&format!("blake2b_long o={outlen} n={n}"), &co, &ro);
            if outlen == 0 {
                assert_eq!(rc, -1, "blake2b_long outlen=0 must fail");
            } else {
                assert_eq!(rc, 0);
            }
        }
    }
}

// ================================ blake2-7: keygen ==========================

/// `*_keygen` calls `randombytes_buf`, so the bytes cannot be compared between
/// the two libraries. Verify instead that exactly KEYBYTES bytes are written.
#[test]
fn keygen_functions() {
    fn check(name: &str, f: FnKeygen, n: usize) {
        let mut buf = vec![CANARY; n + 16];
        unsafe { f(buf.as_mut_ptr()) };
        assert!(
            buf[n..].iter().all(|&b| b == CANARY),
            "{name}: wrote past {n} bytes"
        );
        assert!(buf[..n].iter().any(|&b| b != CANARY), "{name}: wrote nothing");
    }
    macro_rules! kg {
        ($name:literal, $n:expr) => {{
            let (c, r) = both!($name, FnKeygen);
            check(concat!("C ", $name), c, $n);
            check(concat!("Rust ", $name), r, $n);
        }};
    }
    kg!("crypto_generichash_keygen", 32);
    kg!("crypto_generichash_blake2b_keygen", 32);
    kg!("crypto_kdf_keygen", 32);
    kg!("crypto_shorthash_keygen", 16);
    kg!("crypto_kdf_hkdf_sha256_keygen", 32);
    kg!("crypto_kdf_hkdf_sha512_keygen", 64);
}

// ============================== blake2-8: kdf blake2b =======================

#[test]
fn kdf_blake2b_derive_from_key() {
    let (cd, rd) = both!("crypto_kdf_blake2b_derive_from_key", FnKdfDerive);
    let (cg, rg) = both!("crypto_kdf_derive_from_key", FnKdfDerive);
    let (csp, _) = both!("crypto_generichash_blake2b_salt_personal", FnGhSp);
    let mut rng = common::Rng::new(0x9E37_79B9_1234_ABCD);

    let ids = [
        0u64,
        1,
        2,
        0xff,
        0x100,
        0xffff_ffff,
        0x1_0000_0000,
        u64::MAX,
        0x0123_4567_89ab_cdef,
    ];
    for &subkey_len in &[16usize, 17, 31, 32, 33, 63, 64] {
        for &id in &ids {
            let key = rng.bytes(32);
            let ctx = rng.bytes(8);
            let (mut co, mut ro) = (canary(subkey_len), canary(subkey_len));
            let rc = unsafe {
                cd(
                    co.as_mut_ptr(),
                    subkey_len,
                    id,
                    ctx.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            };
            let rr = unsafe {
                rd(
                    ro.as_mut_ptr(),
                    subkey_len,
                    id,
                    ctx.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            };
            let tag = format!("kdf_blake2b len={subkey_len} id={id:#x}");
            common::eqi(&tag, rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&tag, &co, &ro);

            // crypto_kdf_derive_from_key must be the same thing
            let (mut co2, mut ro2) = (canary(subkey_len), canary(subkey_len));
            let rc2 = unsafe {
                cg(
                    co2.as_mut_ptr(),
                    subkey_len,
                    id,
                    ctx.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            };
            let rr2 = unsafe {
                rg(
                    ro2.as_mut_ptr(),
                    subkey_len,
                    id,
                    ctx.as_ptr() as *const c_char,
                    key.as_ptr(),
                )
            };
            common::eqi(&format!("crypto_kdf {tag}"), rc2, rr2);
            common::eqb(&format!("crypto_kdf {tag}"), &co2, &co);
            common::eqb(&format!("crypto_kdf {tag} rust"), &ro2, &ro);

            // independently reproduce it via the salt/personal API
            let mut salt = [0u8; 16];
            salt[..8].copy_from_slice(&id.to_le_bytes());
            let mut personal = [0u8; 16];
            personal[..8].copy_from_slice(&ctx);
            let mut expect = canary(subkey_len);
            unsafe {
                csp(
                    expect.as_mut_ptr(),
                    subkey_len,
                    core::ptr::null(),
                    0,
                    key.as_ptr(),
                    32,
                    salt.as_ptr(),
                    personal.as_ptr(),
                )
            };
            common::eqb(&format!("{tag} == salt_personal"), &expect, &co);
        }
    }

    // ctx bytes containing embedded NULs (it is a byte array, not a C string)
    let key = rng.bytes(32);
    let ctx = [0u8, 0, 0, 0, 0, 0, 0, 0];
    let (mut co, mut ro) = (canary(32), canary(32));
    let rc = unsafe { cd(co.as_mut_ptr(), 32, 5, ctx.as_ptr() as *const c_char, key.as_ptr()) };
    let rr = unsafe { rd(ro.as_mut_ptr(), 32, 5, ctx.as_ptr() as *const c_char, key.as_ptr()) };
    common::eqi("kdf ctx all-zero", rc, rr);
    common::eqb("kdf ctx all-zero", &co, &ro);
}

#[test]
fn kdf_blake2b_errors() {
    let (cd, rd) = both!("crypto_kdf_blake2b_derive_from_key", FnKdfDerive);
    let (cg, rg) = both!("crypto_kdf_derive_from_key", FnKdfDerive);
    let mut rng = common::Rng::new(4242);
    let key = rng.bytes(32);
    let ctx = rng.bytes(8);

    for &subkey_len in &[0usize, 1, 8, 15, 65, 100, 1000, usize::MAX] {
        let (mut co, mut ro) = (canary(8), canary(8));
        set_errno(0);
        let rc = unsafe {
            cd(
                co.as_mut_ptr(),
                subkey_len,
                1,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let ce = errno();
        set_errno(0);
        let rr = unsafe {
            rd(
                ro.as_mut_ptr(),
                subkey_len,
                1,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let re = errno();
        common::eqi(&format!("kdf bad len={subkey_len}"), rc, rr);
        assert_eq!(rc, -1, "subkey_len={subkey_len} must be rejected");
        assert_eq!(ce, EINVAL, "C errno for subkey_len={subkey_len}");
        assert_eq!(re, ce, "errno mismatch for subkey_len={subkey_len}");
        common::eqb("kdf bad len: output untouched", &co, &ro);

        set_errno(0);
        let rc = unsafe {
            cg(
                co.as_mut_ptr(),
                subkey_len,
                1,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let ce = errno();
        set_errno(0);
        let rr = unsafe {
            rg(
                ro.as_mut_ptr(),
                subkey_len,
                1,
                ctx.as_ptr() as *const c_char,
                key.as_ptr(),
            )
        };
        let re = errno();
        common::eqi(&format!("crypto_kdf bad len={subkey_len}"), rc, rr);
        assert_eq!(rc, -1);
        assert_eq!(ce, EINVAL);
        assert_eq!(re, ce);
    }
}

// ============================== blake2-9: hkdf ==============================

macro_rules! hkdf_suite {
    ($fname:ident, $prefix:literal, $prk:expr, $statebytes:expr, $max:expr, $seed:expr) => {
        #[test]
        fn $fname() {
            const PRKLEN: usize = $prk;
            const SB: usize = $statebytes;
            const MAX: usize = $max;
            let (cx, rx) = both!(concat!($prefix, "_extract"), FnHkdfExtract);
            let (cxi, rxi) = both!(concat!($prefix, "_extract_init"), FnHkdfExtractInit);
            let (cxu, rxu) = both!(concat!($prefix, "_extract_update"), FnHkdfExtractUpdate);
            let (cxf, rxf) = both!(concat!($prefix, "_extract_final"), FnHkdfExtractFinal);
            let (ce, re) = both!(concat!($prefix, "_expand"), FnHkdfExpand);
            assert_eq!(chk_getter!(concat!($prefix, "_statebytes")), SB);
            let mut rng = common::Rng::new($seed);

            // ---- extract: one-shot vs streaming, many salt/ikm lengths ----
            for &salt_len in &[0usize, 1, 16, 32, 55, 63, 64, 65, 100, 128, 200] {
                for &ikm_len in &[0usize, 1, 16, 32, 64, 100, 127, 128, 129, 500] {
                    let salt = rng.bytes(if salt_len == 0 { 1 } else { salt_len });
                    let ikm = rng.bytes(if ikm_len == 0 { 1 } else { ikm_len });
                    let sp = salt.as_ptr();
                    let ip = ikm.as_ptr();
                    let tag = format!("{} extract salt={salt_len} ikm={ikm_len}", $prefix);

                    let (mut co, mut ro) = (canary(PRKLEN), canary(PRKLEN));
                    let rc = unsafe { cx(co.as_mut_ptr(), sp, salt_len, ip, ikm_len) };
                    let rr = unsafe { rx(ro.as_mut_ptr(), sp, salt_len, ip, ikm_len) };
                    common::eqi(&tag, rc, rr);
                    assert_eq!(rc, 0);
                    common::eqb(&tag, &co, &ro);

                    // streaming form with random chunk splits (incl. 0-length)
                    let (mut cst, mut rst) = (AlignedState::new(), AlignedState::new());
                    let rc = unsafe { cxi(cst.p(), sp, salt_len) };
                    let rr = unsafe { rxi(rst.p(), sp, salt_len) };
                    common::eqi(&format!("{tag} init"), rc, rr);
                    common::eqb(&format!("{tag} state after init"), cst.s(SB), rst.s(SB));
                    let mut off = 0usize;
                    for k in splits(&mut rng, ikm_len) {
                        let p = unsafe { ip.add(off) };
                        let rc = unsafe { cxu(cst.p(), p, k) };
                        let rr = unsafe { rxu(rst.p(), p, k) };
                        common::eqi(&format!("{tag} update {k}"), rc, rr);
                        common::eqb(
                            &format!("{tag} state after update {k}@{off}"),
                            cst.s(SB),
                            rst.s(SB),
                        );
                        off += k;
                    }
                    let (mut cs2, mut rs2) = (canary(PRKLEN), canary(PRKLEN));
                    let rc = unsafe { cxf(cst.p(), cs2.as_mut_ptr()) };
                    let rr = unsafe { rxf(rst.p(), rs2.as_mut_ptr()) };
                    common::eqi(&format!("{tag} final"), rc, rr);
                    assert_eq!(rc, 0);
                    common::eqb(&format!("{tag} streamed prk"), &cs2, &rs2);
                    common::eqb(&format!("{tag} streamed == one-shot"), &cs2, &co);
                    // extract_final zeroes the whole state
                    assert!(
                        cst.s(SB).iter().all(|&b| b == 0),
                        "{tag}: C extract_final must zero the state"
                    );
                    common::eqb(&format!("{tag} state after final"), cst.s(SB), rst.s(SB));
                }
            }

            // salt == NULL / salt_len == 0 (unsalted HKDF)
            let ikm = rng.bytes(64);
            let (mut co, mut ro) = (canary(PRKLEN), canary(PRKLEN));
            let rc = unsafe { cx(co.as_mut_ptr(), core::ptr::null(), 0, ikm.as_ptr(), 64) };
            let rr = unsafe { rx(ro.as_mut_ptr(), core::ptr::null(), 0, ikm.as_ptr(), 64) };
            common::eqi("extract salt=NULL", rc, rr);
            common::eqb("extract salt=NULL", &co, &ro);

            // ikm == NULL / ikm_len == 0
            let salt = rng.bytes(32);
            let (mut co, mut ro) = (canary(PRKLEN), canary(PRKLEN));
            let rc = unsafe { cx(co.as_mut_ptr(), salt.as_ptr(), 32, core::ptr::null(), 0) };
            let rr = unsafe { rx(ro.as_mut_ptr(), salt.as_ptr(), 32, core::ptr::null(), 0) };
            common::eqi("extract ikm=NULL", rc, rr);
            common::eqb("extract ikm=NULL", &co, &ro);

            // ---- expand ----
            let prk = rng.bytes(PRKLEN);
            let mut out_lens: Vec<usize> = vec![
                0,
                1,
                PRKLEN - 1,
                PRKLEN,
                PRKLEN + 1,
                2 * PRKLEN - 1,
                2 * PRKLEN,
                2 * PRKLEN + 1,
                3 * PRKLEN,
                100,
                255,
                1000,
                MAX - 1,
                MAX,
            ];
            out_lens.dedup();
            for &out_len in &out_lens {
                for &ctx_len in &[0usize, 1, 8, 32, 64, 200] {
                    let cctx = rng.bytes(if ctx_len == 0 { 1 } else { ctx_len });
                    let (mut co, mut ro) = (canary(out_len), canary(out_len));
                    let tag = format!("{} expand out={out_len} ctx={ctx_len}", $prefix);
                    let rc = unsafe {
                        ce(
                            co.as_mut_ptr(),
                            out_len,
                            cctx.as_ptr() as *const c_char,
                            ctx_len,
                            prk.as_ptr(),
                        )
                    };
                    let rr = unsafe {
                        re(
                            ro.as_mut_ptr(),
                            out_len,
                            cctx.as_ptr() as *const c_char,
                            ctx_len,
                            prk.as_ptr(),
                        )
                    };
                    common::eqi(&tag, rc, rr);
                    assert_eq!(rc, 0, "{tag} should succeed");
                    common::eqb(&tag, &co, &ro);
                    assert!(
                        co[out_len..].iter().all(|&b| b == CANARY),
                        "{tag}: wrote past out_len"
                    );
                }
            }

            // ctx == NULL with ctx_len == 0
            let (mut co, mut ro) = (canary(64), canary(64));
            let rc = unsafe { ce(co.as_mut_ptr(), 64, core::ptr::null(), 0, prk.as_ptr()) };
            let rr = unsafe { re(ro.as_mut_ptr(), 64, core::ptr::null(), 0, prk.as_ptr()) };
            common::eqi("expand ctx=NULL", rc, rr);
            common::eqb("expand ctx=NULL", &co, &ro);

            // ---- expand error: out_len > BYTES_MAX ----
            for &out_len in &[MAX + 1, MAX + 2, MAX * 2, usize::MAX] {
                let (mut co, mut ro) = (canary(8), canary(8));
                let cctx = rng.bytes(8);
                set_errno(0);
                let rc = unsafe {
                    ce(
                        co.as_mut_ptr(),
                        out_len,
                        cctx.as_ptr() as *const c_char,
                        8,
                        prk.as_ptr(),
                    )
                };
                let cerr = errno();
                set_errno(0);
                let rr = unsafe {
                    re(
                        ro.as_mut_ptr(),
                        out_len,
                        cctx.as_ptr() as *const c_char,
                        8,
                        prk.as_ptr(),
                    )
                };
                let rerr = errno();
                common::eqi(&format!("{} expand out_len={out_len}", $prefix), rc, rr);
                assert_eq!(rc, -1, "out_len={out_len} must be rejected");
                assert_eq!(cerr, EINVAL);
                assert_eq!(rerr, cerr);
                common::eqb("expand too long: output untouched", &co, &ro);
            }
        }
    };
}

hkdf_suite!(
    hkdf_sha256,
    "crypto_kdf_hkdf_sha256",
    32,
    208,
    0xff * 32,
    0x1234_5678_9ABC_DEF0
);
hkdf_suite!(
    hkdf_sha512,
    "crypto_kdf_hkdf_sha512",
    64,
    416,
    0xff * 64,
    0x0FED_CBA9_8765_4321
);

// ============================ blake2-10: shorthash ==========================

#[test]
fn shorthash_siphash24() {
    let (cs, rs) = both!("crypto_shorthash_siphash24", FnShort);
    let (cg, rg) = both!("crypto_shorthash", FnShort);
    let mut rng = common::Rng::new(0x5150_5150_5150_5150);

    let mut lens: Vec<usize> = (0..=80).collect();
    lens.extend_from_slice(&[100, 127, 128, 129, 255, 256, 257, 1000, 4096]);
    for &n in &lens {
        for _ in 0..3 {
            let msg = rng.bytes(n);
            let key = rng.bytes(16);
            let mp = if n == 0 {
                core::ptr::null()
            } else {
                msg.as_ptr()
            };
            let (mut co, mut ro) = (canary(8), canary(8));
            let rc = unsafe { cs(co.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            let rr = unsafe { rs(ro.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            common::eqi(&format!("siphash24 n={n}"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&format!("siphash24 n={n}"), &co, &ro);

            let (mut co2, mut ro2) = (canary(8), canary(8));
            let rc = unsafe { cg(co2.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            let rr = unsafe { rg(ro2.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            common::eqi(&format!("crypto_shorthash n={n}"), rc, rr);
            common::eqb(&format!("crypto_shorthash n={n}"), &co2, &ro2);
            common::eqb("crypto_shorthash == siphash24", &co2, &co);
        }
    }
    // all-zero and all-ones keys
    for k in [[0u8; 16], [0xffu8; 16]] {
        for &n in &[0usize, 7, 8, 9, 64] {
            let msg = rng.bytes(n);
            let (mut co, mut ro) = (canary(8), canary(8));
            unsafe { cs(co.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            unsafe { rs(ro.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            common::eqb(&format!("siphash24 extreme key n={n}"), &co, &ro);
        }
    }
}

#[test]
fn shorthash_siphashx24() {
    let (cs, rs) = both!("crypto_shorthash_siphashx24", FnShort);
    let mut rng = common::Rng::new(0x2718_2818_2845_9045);

    let mut lens: Vec<usize> = (0..=80).collect();
    lens.extend_from_slice(&[100, 127, 128, 129, 255, 256, 257, 1000, 4096]);
    for &n in &lens {
        for _ in 0..3 {
            let msg = rng.bytes(n);
            let key = rng.bytes(16);
            let mp = if n == 0 {
                core::ptr::null()
            } else {
                msg.as_ptr()
            };
            let (mut co, mut ro) = (canary(16), canary(16));
            let rc = unsafe { cs(co.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            let rr = unsafe { rs(ro.as_mut_ptr(), mp, n as u64, key.as_ptr()) };
            common::eqi(&format!("siphashx24 n={n}"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&format!("siphashx24 n={n}"), &co, &ro);
        }
    }
    for k in [[0u8; 16], [0xffu8; 16]] {
        for &n in &[0usize, 7, 8, 9, 64] {
            let msg = rng.bytes(n);
            let (mut co, mut ro) = (canary(16), canary(16));
            unsafe { cs(co.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            unsafe { rs(ro.as_mut_ptr(), msg.as_ptr(), n as u64, k.as_ptr()) };
            common::eqb(&format!("siphashx24 extreme key n={n}"), &co, &ro);
        }
    }
}

// ============================= blake2-11: verify ============================

#[test]
fn verify_all() {
    let mut rng = common::Rng::new(0x1357_9BDF_2468_ACE0);

    macro_rules! vcase {
        ($name:literal, $n:expr) => {{
            const N: usize = $n;
            let (c, r) = both!($name, FnVerify);
            // equal buffers => 0
            for _ in 0..20 {
                let x = rng.bytes(N);
                let y = x.clone();
                let rc = unsafe { c(x.as_ptr(), y.as_ptr()) };
                let rr = unsafe { r(x.as_ptr(), y.as_ptr()) };
                common::eqi(concat!($name, " equal"), rc, rr);
                assert_eq!(rc, 0, concat!($name, ": equal must return 0"));
                // same pointer twice
                let rc = unsafe { c(x.as_ptr(), x.as_ptr()) };
                let rr = unsafe { r(x.as_ptr(), x.as_ptr()) };
                common::eqi(concat!($name, " aliased"), rc, rr);
                assert_eq!(rc, 0);
            }
            // differ in each single byte position, for each of 8 bit flips
            let x = rng.bytes(N);
            for i in 0..N {
                for bit in 0..8 {
                    let mut y = x.clone();
                    y[i] ^= 1u8 << bit;
                    let rc = unsafe { c(x.as_ptr(), y.as_ptr()) };
                    let rr = unsafe { r(x.as_ptr(), y.as_ptr()) };
                    assert_eq!(
                        rc, rr,
                        concat!($name, ": mismatch at byte {} bit {}"),
                        i, bit
                    );
                    assert_eq!(rc, -1, concat!($name, ": differing must return -1"));
                    // and the reverse argument order
                    let rc = unsafe { c(y.as_ptr(), x.as_ptr()) };
                    let rr = unsafe { r(y.as_ptr(), x.as_ptr()) };
                    assert_eq!(rc, rr);
                    assert_eq!(rc, -1);
                }
            }
            // fully random pairs
            for _ in 0..50 {
                let a = rng.bytes(N);
                let b = rng.bytes(N);
                let rc = unsafe { c(a.as_ptr(), b.as_ptr()) };
                let rr = unsafe { r(a.as_ptr(), b.as_ptr()) };
                common::eqi(concat!($name, " random"), rc, rr);
            }
            // degenerate all-zero / all-ones
            let z = vec![0u8; N];
            let o = vec![0xffu8; N];
            for (a, b, want) in [(&z, &z, 0), (&o, &o, 0), (&z, &o, -1), (&o, &z, -1)] {
                let rc = unsafe { c(a.as_ptr(), b.as_ptr()) };
                let rr = unsafe { r(a.as_ptr(), b.as_ptr()) };
                common::eqi(concat!($name, " degenerate"), rc, rr);
                assert_eq!(rc, want);
            }
        }};
    }

    vcase!("crypto_verify_16", 16);
    vcase!("crypto_verify_32", 32);
    vcase!("crypto_verify_64", 64);
}

// ===================== blake2-12: sodium_misuse() / assert() ================
//
// Every `sodium_misuse()` site (and the one live `assert()`) terminates the
// process, so it cannot be exercised in-process. Instead this test re-executes
// the test binary as a child process, once per (library, case) pair, and
// asserts that BOTH the C and the Rust `.so` die with SIGABRT.

const ABORT_ENV: &str = "BLAKE2_ABORT_CASE";

/// All abort sites in the AREA's C sources, keyed by name.
const ABORT_CASES: &[&str] = &[
    // crypto_generichash_blake2b_final: assert(outlen <= UINT8_MAX)
    "gh_final_outlen_256",
    "gh_final_outlen_300",
    // blake2b_final: !outlen || outlen > BLAKE2B_OUTBYTES
    "gh_final_outlen_0",
    "gh_final_outlen_65",
    "generichash_final_outlen_0",
    "b2_final_outlen_0",
    "b2_final_outlen_65",
    // blake2b_init: !outlen || outlen > BLAKE2B_OUTBYTES
    "b2_init_outlen_0",
    "b2_init_outlen_65",
    // blake2b_init_salt_personal
    "b2_init_sp_outlen_0",
    "b2_init_sp_outlen_65",
    // blake2b_init_key
    "b2_init_key_outlen_0",
    "b2_init_key_outlen_65",
    "b2_init_key_null",
    "b2_init_key_keylen_0",
    "b2_init_key_keylen_65",
    // blake2b_init_key_salt_personal
    "b2_init_key_sp_outlen_0",
    "b2_init_key_sp_outlen_65",
    "b2_init_key_sp_null",
    "b2_init_key_sp_keylen_0",
    "b2_init_key_sp_keylen_65",
    // blake2b()
    "b2_in_null",
    "b2_out_null",
    "b2_outlen_0",
    "b2_outlen_65",
    "b2_key_null_keylen_1",
    "b2_keylen_65",
    // blake2b_salt_personal()
    "b2sp_in_null",
    "b2sp_out_null",
    "b2sp_outlen_0",
    "b2sp_key_null_keylen_1",
    "b2sp_keylen_65",
    // the public wrappers reach the same sites through blake2b()
    "gh_in_null",
    "gh_key_null_keylen_1",
    "ghsp_in_null",
    "ghsp_key_null_keylen_1",
    "generichash_in_null",
];

#[test]
#[ignore = "driven as a subprocess by abort_paths"]
fn abort_child() {
    let spec = std::env::var(ABORT_ENV).unwrap_or_else(|_| std::process::exit(3));
    let (which, case) = spec.split_once(':').unwrap_or_else(|| std::process::exit(3));
    let l = common::libs();
    let lib = match which {
        "c" => &l.c,
        "r" => &l.r,
        _ => std::process::exit(3),
    };

    let key = [7u8; 64];
    let salt = [1u8; 16];
    let pers = [2u8; 16];
    let msg = [3u8; 64];
    let mut out = [0u8; 64];
    let mut st = AlignedState::new();

    macro_rules! s {
        ($n:literal, $t:ty) => {
            getsym!(lib, $n, $t)
        };
    }
    let init = s!("crypto_generichash_blake2b_init", FnGhInit);
    let ghfinal = s!("crypto_generichash_blake2b_final", FnGhFinal);
    let ghinit_c = s!("crypto_generichash_init", FnGhInit);
    let ghfinal_c = s!("crypto_generichash_final", FnGhFinal);
    let b2init = s!("_sodium_blake2b_init", FnB2Init);
    let b2initsp = s!("_sodium_blake2b_init_salt_personal", FnB2InitSp);
    let b2initkey = s!("_sodium_blake2b_init_key", FnB2InitKey);
    let b2initkeysp = s!("_sodium_blake2b_init_key_salt_personal", FnB2InitKeySp);
    let b2final = s!("_sodium_blake2b_final", FnB2Final);
    let b2 = s!("_sodium_blake2b", FnB2);
    let b2sp = s!("_sodium_blake2b_salt_personal", FnB2Sp);
    let gh = s!("crypto_generichash_blake2b", FnGh);
    let ghsp = s!("crypto_generichash_blake2b_salt_personal", FnGhSp);
    let ghc = s!("crypto_generichash", FnGh);

    let kv = key.as_ptr() as *const c_void;
    let sv = salt.as_ptr() as *const c_void;
    let pv = pers.as_ptr() as *const c_void;

    unsafe {
        match case {
            "gh_final_outlen_256" => {
                init(st.p(), core::ptr::null(), 0, 32);
                ghfinal(st.p(), out.as_mut_ptr(), 256);
            }
            "gh_final_outlen_300" => {
                init(st.p(), core::ptr::null(), 0, 32);
                ghfinal(st.p(), out.as_mut_ptr(), 300);
            }
            "gh_final_outlen_0" => {
                init(st.p(), core::ptr::null(), 0, 32);
                ghfinal(st.p(), out.as_mut_ptr(), 0);
            }
            "gh_final_outlen_65" => {
                init(st.p(), core::ptr::null(), 0, 32);
                ghfinal(st.p(), out.as_mut_ptr(), 65);
            }
            "generichash_final_outlen_0" => {
                ghinit_c(st.p(), core::ptr::null(), 0, 32);
                ghfinal_c(st.p(), out.as_mut_ptr(), 0);
            }
            "b2_final_outlen_0" => {
                b2init(st.p(), 32);
                b2final(st.p(), out.as_mut_ptr(), 0);
            }
            "b2_final_outlen_65" => {
                b2init(st.p(), 32);
                b2final(st.p(), out.as_mut_ptr(), 65);
            }
            "b2_init_outlen_0" => {
                b2init(st.p(), 0);
            }
            "b2_init_outlen_65" => {
                b2init(st.p(), 65);
            }
            "b2_init_sp_outlen_0" => {
                b2initsp(st.p(), 0, sv, pv);
            }
            "b2_init_sp_outlen_65" => {
                b2initsp(st.p(), 65, sv, pv);
            }
            "b2_init_key_outlen_0" => {
                b2initkey(st.p(), 0, kv, 32);
            }
            "b2_init_key_outlen_65" => {
                b2initkey(st.p(), 65, kv, 32);
            }
            "b2_init_key_null" => {
                b2initkey(st.p(), 32, core::ptr::null(), 32);
            }
            "b2_init_key_keylen_0" => {
                b2initkey(st.p(), 32, kv, 0);
            }
            "b2_init_key_keylen_65" => {
                b2initkey(st.p(), 32, kv, 65);
            }
            "b2_init_key_sp_outlen_0" => {
                b2initkeysp(st.p(), 0, kv, 32, sv, pv);
            }
            "b2_init_key_sp_outlen_65" => {
                b2initkeysp(st.p(), 65, kv, 32, sv, pv);
            }
            "b2_init_key_sp_null" => {
                b2initkeysp(st.p(), 32, core::ptr::null(), 32, sv, pv);
            }
            "b2_init_key_sp_keylen_0" => {
                b2initkeysp(st.p(), 32, kv, 0, sv, pv);
            }
            "b2_init_key_sp_keylen_65" => {
                b2initkeysp(st.p(), 32, kv, 65, sv, pv);
            }
            "b2_in_null" => {
                b2(out.as_mut_ptr(), core::ptr::null(), core::ptr::null(), 32, 1, 0);
            }
            "b2_out_null" => {
                b2(
                    core::ptr::null_mut(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    32,
                    64,
                    0,
                );
            }
            "b2_outlen_0" => {
                b2(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    0,
                    64,
                    0,
                );
            }
            "b2_outlen_65" => {
                b2(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    65,
                    64,
                    0,
                );
            }
            "b2_key_null_keylen_1" => {
                b2(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    32,
                    64,
                    1,
                );
            }
            "b2_keylen_65" => {
                b2(out.as_mut_ptr(), msg.as_ptr() as *const c_void, kv, 32, 64, 65);
            }
            "b2sp_in_null" => {
                b2sp(
                    out.as_mut_ptr(),
                    core::ptr::null(),
                    core::ptr::null(),
                    32,
                    1,
                    0,
                    sv,
                    pv,
                );
            }
            "b2sp_out_null" => {
                b2sp(
                    core::ptr::null_mut(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    32,
                    64,
                    0,
                    sv,
                    pv,
                );
            }
            "b2sp_outlen_0" => {
                b2sp(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    0,
                    64,
                    0,
                    sv,
                    pv,
                );
            }
            "b2sp_key_null_keylen_1" => {
                b2sp(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    core::ptr::null(),
                    32,
                    64,
                    1,
                    sv,
                    pv,
                );
            }
            "b2sp_keylen_65" => {
                b2sp(
                    out.as_mut_ptr(),
                    msg.as_ptr() as *const c_void,
                    kv,
                    32,
                    64,
                    65,
                    sv,
                    pv,
                );
            }
            "gh_in_null" => {
                gh(out.as_mut_ptr(), 32, core::ptr::null(), 1, core::ptr::null(), 0);
            }
            "gh_key_null_keylen_1" => {
                gh(out.as_mut_ptr(), 32, msg.as_ptr(), 64, core::ptr::null(), 1);
            }
            "ghsp_in_null" => {
                ghsp(
                    out.as_mut_ptr(),
                    32,
                    core::ptr::null(),
                    1,
                    core::ptr::null(),
                    0,
                    salt.as_ptr(),
                    pers.as_ptr(),
                );
            }
            "ghsp_key_null_keylen_1" => {
                ghsp(
                    out.as_mut_ptr(),
                    32,
                    msg.as_ptr(),
                    64,
                    core::ptr::null(),
                    1,
                    salt.as_ptr(),
                    pers.as_ptr(),
                );
            }
            "generichash_in_null" => {
                ghc(out.as_mut_ptr(), 32, core::ptr::null(), 1, core::ptr::null(), 0);
            }
            _ => std::process::exit(3),
        }
    }
    // Reaching this point means the library did NOT abort.
    std::process::exit(7);
}

#[test]
fn abort_paths() {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    for &case in ABORT_CASES {
        for which in ["c", "r"] {
            let st = std::process::Command::new(&exe)
                .args([
                    "--ignored",
                    "--exact",
                    "abort_child",
                    "--test-threads=1",
                    "--nocapture",
                ])
                .env(ABORT_ENV, format!("{which}:{case}"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("spawn child");
            assert_eq!(
                st.signal(),
                Some(libc_sigabrt()),
                "[{which}] {case}: expected SIGABRT, got status {st:?}"
            );
        }
    }
}

fn libc_sigabrt() -> i32 {
    6
}
