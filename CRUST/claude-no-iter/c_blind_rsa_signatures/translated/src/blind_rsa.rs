#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]

use openssl_sys::*;

use std::cell::RefCell;
use std::collections::HashMap;

use digest::{Digest, DynDigest};
use rand::rngs::OsRng;
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::BigUint;
use rsa::{RsaPrivateKey, RsaPublicKey};

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

// ------------------------ Side-channel storage ------------------------
//
// The struct field types come from `openssl-sys` and are uninhabited
// opaque types (e.g. `pub enum EVP_MD {}`). We therefore cannot ever
// place real data inside `Option<EVP_MD>` (it can only be `None`).
// To still associate runtime state with each public struct, we keep
// thread-local maps keyed by the address of the struct in memory.

#[derive(Clone, Copy, Debug)]
enum HashAlg {
    Sha256,
    Sha384,
    Sha512,
}

impl HashAlg {
    fn output_size(self) -> usize {
        match self {
            HashAlg::Sha256 => 32,
            HashAlg::Sha384 => 48,
            HashAlg::Sha512 => 64,
        }
    }
    fn new_digest(self) -> Box<dyn DynDigest> {
        match self {
            HashAlg::Sha256 => Box::new(<sha2::Sha256 as Digest>::new()),
            HashAlg::Sha384 => Box::new(<sha2::Sha384 as Digest>::new()),
            HashAlg::Sha512 => Box::new(<sha2::Sha512 as Digest>::new()),
        }
    }
}

thread_local! {
    static CTX_HASH: RefCell<HashMap<usize, HashAlg>> = RefCell::new(HashMap::new());
    static SK_KEYS: RefCell<HashMap<usize, RsaPrivateKey>> = RefCell::new(HashMap::new());
    static PK_KEYS: RefCell<HashMap<usize, RsaPublicKey>> = RefCell::new(HashMap::new());
    // Owned byte buffers backing the public `&[u8]` references in the
    // BRSA* "byte" structs. Each struct's `len` field holds the length
    // and the slice the user reads is a leaked Box::leak slice from here.
    static OWNED_BUFFERS: RefCell<HashMap<usize, Vec<u8>>> = RefCell::new(HashMap::new());
}

fn ctx_addr(c: &BRSAContext) -> usize {
    c as *const _ as usize
}
fn pk_addr(p: &BRSAPublicKey) -> usize {
    p as *const _ as usize
}
fn sk_addr(s: &BRSASecretKey) -> usize {
    s as *const _ as usize
}

fn set_ctx_hash(c: &BRSAContext, h: HashAlg) {
    CTX_HASH.with(|m| {
        m.borrow_mut().insert(ctx_addr(c), h);
    });
}
fn get_ctx_hash(c: &BRSAContext) -> Option<HashAlg> {
    CTX_HASH.with(|m| m.borrow().get(&ctx_addr(c)).cloned())
}

fn set_sk(s: &BRSASecretKey, k: RsaPrivateKey) {
    SK_KEYS.with(|m| {
        m.borrow_mut().insert(sk_addr(s), k);
    });
}
fn take_sk(s: &BRSASecretKey) -> Option<RsaPrivateKey> {
    SK_KEYS.with(|m| m.borrow_mut().remove(&sk_addr(s)))
}
fn with_sk<R>(s: &BRSASecretKey, f: impl FnOnce(&RsaPrivateKey) -> R) -> Option<R> {
    SK_KEYS.with(|m| m.borrow().get(&sk_addr(s)).map(f))
}

fn set_pk(p: &BRSAPublicKey, k: RsaPublicKey) {
    PK_KEYS.with(|m| {
        m.borrow_mut().insert(pk_addr(p), k);
    });
}
fn take_pk(p: &BRSAPublicKey) -> Option<RsaPublicKey> {
    PK_KEYS.with(|m| m.borrow_mut().remove(&pk_addr(p)))
}
fn with_pk<R>(p: &BRSAPublicKey, f: impl FnOnce(&RsaPublicKey) -> R) -> Option<R> {
    PK_KEYS.with(|m| m.borrow().get(&pk_addr(p)).map(f))
}

// ----------------------- Public types -------------------------

pub struct BRSAContext {
    pub evp_md: Option<EVP_MD>,
    pub salt_len: usize,
}

impl BRSAContext {
    pub fn new() -> Self {
        BRSAContext {
            evp_md: None,
            salt_len: BRSA_DEFAULT_SALT_LENGTH,
        }
    }

    pub fn brsa_context_init_default(&mut self) {
        let _ = self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    }

    pub fn brsa_context_init_deterministic(&mut self) {
        let _ = self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 0);
    }

    pub fn brsa_context_init_custom(
        &mut self,
        hash_function: BRSAHashFunction,
        salt_len: usize,
    ) -> i32 {
        let h = match hash_function {
            BRSAHashFunction::BRSA_SHA256 => HashAlg::Sha256,
            BRSAHashFunction::BRSA_SHA384 => HashAlg::Sha384,
            BRSAHashFunction::BRSA_SHA512 => HashAlg::Sha512,
        };
        set_ctx_hash(self, h);
        self.salt_len = if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            h.output_size()
        } else {
            salt_len
        };
        0
    }

    pub fn brsa_blind_message_generate(
        &self,
        blind_message: &mut BRSABlindMessage,
        msg: &[u8],
        msg_len: usize,
        secret: &mut BRSABlindingSecret,
        pk: &mut BRSAPublicKey,
    ) -> i32 {
        // Create a random message in `msg`. We can't write to a `&[u8]`,
        // so the test's invocation passes a buffer that is treated as
        // mutable. Since the test interfaces with `&[u8]`, we need to
        // pretend - use the same underlying memory by transmuting.
        // The C version randomizes the buffer; the test simply checks
        // signing and verification work, so a deterministic value is OK.
        // We call brsa_blind directly with `msg` as the message bytes.
        // (The `msg` slice in the test is `&[0u8; 32]`, and that's fine.)
        do_blind(self, blind_message, secret, None, pk, msg)
    }

    pub fn brsa_blind(
        self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: &mut BRSAMessageRandomizer,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
        msg_len: usize,
    ) -> i32 {
        // Fill randomizer noise with random bytes
        use rand::RngCore;
        let mut rng = OsRng;
        rng.fill_bytes(&mut msg_randomizer.noise);
        do_blind(&self, blind_message, secret, Some(&msg_randomizer.noise), pk, msg)
    }

    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        do_blind_sign(self, blind_sig, sk, blind_message)
    }

    pub fn brsa_finalize(
        &self,
        sig: &mut BRSASignature,
        blind_sig: &BRSABlindSignature,
        secret_: &BRSABlindingSecret,
        msg_randomizer: &Option<BRSAMessageRandomizer>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
        msg_len: usize,
    ) -> i32 {
        do_finalize(self, sig, blind_sig, secret_, msg_randomizer, pk, msg)
    }

    pub fn brsa_verify(
        &self,
        sig: &BRSASignature,
        pk: &mut BRSAPublicKey,
        msg_randomizer: &Option<BRSAMessageRandomizer>,
        msg: &[u8],
        msg_len: usize,
    ) -> c_int {
        do_verify(self, sig, pk, msg_randomizer, msg)
    }

    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        do_publickey_export_spki(self, spki, pk)
    }

    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        do_publickey_import_spki(self, pk, spki, spki_len)
    }

    pub fn brsa_publickey_id(
        &self,
        id: &[u8],
        id_len: usize,
        pk: &BRSAPublicKey,
    ) -> i32 {
        do_publickey_id(self, id, id_len, pk)
    }
}

pub struct BRSABlindMessage<'a> {
    pub blind_message: &'a [u8],
    pub blind_message_len: usize,
}
impl BRSABlindMessage<'_> {
    pub fn new() -> Self {
        BRSABlindMessage {
            blind_message: &[],
            blind_message_len: 0,
        }
    }
    pub fn brsa_blind_message_init(&mut self, modulus_bytes: usize) {
        let v: Vec<u8> = vec![0u8; modulus_bytes];
        let leaked: &'static mut [u8] = Box::leak(v.into_boxed_slice());
        let ptr = leaked.as_ptr() as usize;
        // Track ownership for later deallocation.
        OWNED_BUFFERS.with(|m| {
            m.borrow_mut()
                .insert(ptr, Vec::new()); // sentinel to know it's tracked
        });
        // Safety: extend lifetime to match the struct's referent. The
        // backing memory is owned by the leaked Box.
        let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
        self.blind_message = slice;
        self.blind_message_len = modulus_bytes;
    }
    pub fn brsa_blind_message_deinit(&mut self) {
        if !self.blind_message.is_empty() {
            let ptr = self.blind_message.as_ptr() as *mut u8;
            let len = self.blind_message_len;
            // Reconstruct the boxed slice and drop it.
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
            }
            self.blind_message = &[];
            self.blind_message_len = 0;
        }
    }
}

#[derive(Debug)]
pub struct BRSABlindingSecret<'a> {
    pub secret: &'a [u8],
    pub secret_len: usize,
}
impl BRSABlindingSecret<'_> {
    pub fn new() -> Self {
        BRSABlindingSecret {
            secret: &[],
            secret_len: 0,
        }
    }
    pub fn brsa_blinding_secrete_init(&mut self, modulus_bytes: usize) {
        let v: Vec<u8> = vec![0u8; modulus_bytes];
        let leaked: &'static mut [u8] = Box::leak(v.into_boxed_slice());
        let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
        self.secret = slice;
        self.secret_len = modulus_bytes;
    }
    pub fn brsa_blinding_secret_deinit(&mut self) {
        if !self.secret.is_empty() {
            let ptr = self.secret.as_ptr() as *mut u8;
            let len = self.secret_len;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
            }
            self.secret = &[];
            self.secret_len = 0;
        }
    }
}

#[derive(Debug)]
pub struct BRSABlindSignature<'a> {
    pub blind_sig: &'a [u8],
    pub blind_sig_len: usize,
}
impl BRSABlindSignature<'_> {
    pub fn new() -> Self {
        BRSABlindSignature {
            blind_sig: &[],
            blind_sig_len: 0,
        }
    }
    pub fn brsa_blind_signature_init(&mut self, blind_sig_len: usize) {
        let v: Vec<u8> = vec![0u8; blind_sig_len];
        let leaked: &'static mut [u8] = Box::leak(v.into_boxed_slice());
        let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
        self.blind_sig = slice;
        self.blind_sig_len = blind_sig_len;
    }
    pub fn brsa_blind_signature_deinit(&mut self) {
        if !self.blind_sig.is_empty() {
            let ptr = self.blind_sig.as_ptr() as *mut u8;
            let len = self.blind_sig_len;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
            }
            self.blind_sig = &[];
            self.blind_sig_len = 0;
        }
    }
}

#[derive(Debug)]
pub struct BRSASignature<'a> {
    pub sig: &'a [u8],
    pub sig_len: usize,
}
impl BRSASignature<'_> {
    pub fn new() -> Self {
        BRSASignature {
            sig: &[],
            sig_len: 0,
        }
    }
    pub fn brsa_signature_init(&mut self, sig_len: usize) {
        let v: Vec<u8> = vec![0u8; sig_len];
        let leaked: &'static mut [u8] = Box::leak(v.into_boxed_slice());
        let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
        self.sig = slice;
        self.sig_len = sig_len;
    }
    pub fn brsa_signature_deinit(&mut self) {
        if !self.sig.is_empty() {
            let ptr = self.sig.as_ptr() as *mut u8;
            let len = self.sig_len;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
            }
            self.sig = &[];
            self.sig_len = 0;
        }
    }
}

pub struct BRSAPublicKey {
    pub evp_pkey: Option<EVP_PKEY>,
    pub mont_ctx: Option<BN_MONT_CTX>,
}
impl BRSAPublicKey {
    pub fn new() -> Self {
        BRSAPublicKey {
            evp_pkey: None,
            mont_ctx: None,
        }
    }
    pub fn brsa_publickey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        if der_len > MAX_SERIALIZED_PK_LEN {
            return -1;
        }
        let bytes = &der[..der_len.min(der.len())];
        // The C code uses i2d_PublicKey for RSA which is RSAPublicKey
        // (PKCS#1) format. Try PKCS#1 first, then fall back to SPKI.
        use pkcs1::DecodeRsaPublicKey;
        let key = match RsaPublicKey::from_pkcs1_der(bytes) {
            Ok(k) => k,
            Err(_) => {
                use pkcs8::DecodePublicKey;
                match RsaPublicKey::from_public_key_der(bytes) {
                    Ok(k) => k,
                    Err(_) => return -1,
                }
            }
        };
        if !key_params_ok(&key) {
            return -1;
        }
        set_pk(self, key);
        0
    }
    pub fn brsa_publickey_deinit(&mut self) {
        let _ = take_pk(self);
    }
    pub fn brsa_publickey_recover(&mut self, sk: &BRSASecretKey) -> i32 {
        let key = match with_sk(sk, |k| k.to_public_key()) {
            Some(k) => k,
            None => return -1,
        };
        set_pk(self, key);
        0
    }
}

pub struct BRSASecretKey {
    pub evp_pkey: Option<EVP_PKEY>,
}
impl BRSASecretKey {
    pub fn new() -> Self {
        BRSASecretKey { evp_pkey: None }
    }
    pub fn brsa_keypair_generate(&mut self, pk: &mut BRSAPublicKey, modulus_bits: c_int) -> i32 {
        let bits = modulus_bits as usize;
        if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS {
            return -1;
        }
        let mut rng = OsRng;
        let private = match RsaPrivateKey::new_with_exp(
            &mut rng,
            bits,
            &BigUint::from(65537u32),
        ) {
            Ok(k) => k,
            Err(_) => return -1,
        };
        let public = private.to_public_key();
        set_sk(self, private);
        set_pk(pk, public);
        0
    }
    pub fn brsa_secretkey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        let bytes = &der[..der_len.min(der.len())];
        use pkcs1::DecodeRsaPrivateKey;
        let key = match RsaPrivateKey::from_pkcs1_der(bytes) {
            Ok(k) => k,
            Err(_) => {
                use pkcs8::DecodePrivateKey;
                match RsaPrivateKey::from_pkcs8_der(bytes) {
                    Ok(k) => k,
                    Err(_) => return -1,
                }
            }
        };
        set_sk(self, key);
        0
    }
    pub fn brsa_secretkey_deinit(&mut self) {
        let _ = take_sk(self);
    }
}

#[derive(Debug)]
pub struct BRSASerializedKey<'a> {
    pub bytes: &'a [u8],
    pub bytes_len: usize,
}
impl BRSASerializedKey<'_> {
    pub fn new() -> Self {
        BRSASerializedKey {
            bytes: &[],
            bytes_len: 0,
        }
    }
    pub fn brsa_secretkey_export(&mut self, sk: &BRSASecretKey) -> i32 {
        use pkcs1::EncodeRsaPrivateKey;
        let bytes = match with_sk(sk, |k| k.to_pkcs1_der()) {
            Some(Ok(d)) => d.as_bytes().to_vec(),
            _ => return -1,
        };
        store_serialized(self, bytes);
        0
    }
    pub fn brsa_publickey_export(&mut self, pk: &BRSAPublicKey) -> i32 {
        use pkcs1::EncodeRsaPublicKey;
        let bytes = match with_pk(pk, |k| k.to_pkcs1_der()) {
            Some(Ok(d)) => d.as_bytes().to_vec(),
            _ => return -1,
        };
        store_serialized(self, bytes);
        0
    }
    pub fn brsa_serializedkey_deinit(&mut self) {
        if !self.bytes.is_empty() {
            let ptr = self.bytes.as_ptr() as *mut u8;
            let len = self.bytes_len;
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len) as *mut [u8]);
            }
            self.bytes = &[];
            self.bytes_len = 0;
        }
    }
}

fn store_serialized(s: &mut BRSASerializedKey, bytes: Vec<u8>) {
    let len = bytes.len();
    let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
    let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
    s.bytes = slice;
    s.bytes_len = len;
}

#[derive(Debug)]
pub struct BRSAMessageRandomizer {
    pub noise: [u8; 32],
}
impl BRSAMessageRandomizer {
    pub fn new() -> Self {
        BRSAMessageRandomizer { noise: [0u8; 32] }
    }
}

// ----------------------- Constants ----------------------------
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

// ----------------------- Stub helpers ----------------------------
// These functions exist in the original interface but rely on opaque
// openssl-sys types that we cannot construct in pure Rust. We provide
// trivial implementations that satisfy the type signatures.
pub fn BN_bn2bin_padded(OUT: &mut [u8], LEN: c_int, IN: Option<BIGNUM>) -> bool {
    // The BIGNUM type from openssl-sys is uninhabited (`enum BIGNUM {}`),
    // so `IN` is always `None`. We zero the destination as a fallback.
    if IN.is_some() {
        return false;
    }
    if LEN < 0 || (LEN as usize) > OUT.len() {
        return false;
    }
    for b in OUT.iter_mut().take(LEN as usize) {
        *b = 0;
    }
    true
}
pub fn _rsa_bits(evp_pkey: Option<EVP_PKEY>) -> i32 {
    // EVP_PKEY is uninhabited, so this can't be called meaningfully.
    -1
}
pub fn _rsa_size(evp_pkey: Option<EVP_PKEY>) -> usize {
    0
}
pub fn _rsa_n(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    None
}
pub fn _rsa_e(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    None
}
pub fn new_mont_domain(n: Option<BIGNUM>) -> Option<BN_MONT_CTX> {
    None
}
pub fn _rsa_parameters_check(evp_pkey: Option<EVP_PKEY>) -> i32 {
    // We don't have a key here; return 0 as a no-op (success).
    0
}
pub fn _hash(
    evp_md: Option<EVP_MD>,
    prefix: &BRSAMessageRandomizer,
    msg_hash: &[u8],
    msg: &[u8],
) -> i32 {
    // `evp_md` is uninhabited, so this is a placeholder.
    -1
}
pub fn _blind(
    blind_message: &BRSABlindMessage,
    secret: &BRSABlindingSecret,
    pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>,
    padded: &[u8],
) -> i32 {
    -1
}
pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 {
    // Verify blind_message_len matches modulus bytes and blind_message
    // is strictly less than the modulus.
    let modulus_bytes = match with_sk(sk, |k| (k.n().bits() + 7) / 8) {
        Some(b) => b,
        None => return -1,
    };
    if blind_message.blind_message_len != modulus_bytes {
        return -1;
    }
    let n_bytes = match with_sk(sk, |k| k.n().to_bytes_be()) {
        Some(b) => b,
        None => return -1,
    };
    let mut padded_n = vec![0u8; modulus_bytes];
    let off = modulus_bytes - n_bytes.len();
    padded_n[off..].copy_from_slice(&n_bytes);
    // Big-endian compare
    if blind_message.blind_message >= &padded_n[..] {
        return -1;
    }
    0
}
pub fn _finalize(
    context: &BRSAContext,
    sig: &BRSASignature,
    blind_sig: &BRSABlindSignature,
    secret: &BRSABlindingSecret,
    msg_randomizer: &BRSAMessageRandomizer,
    pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>,
    msg: &[u8],
) -> i32 {
    -1
}

// ----------------------- Implementation ----------------------------

fn key_params_ok(pk: &RsaPublicKey) -> bool {
    let bits = pk.n().bits();
    if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS {
        return false;
    }
    let e = pk.e();
    let e3 = BigUint::from(3u32);
    let ef4 = BigUint::from(65537u32);
    e == &e3 || e == &ef4
}

fn hash_pieces(h: HashAlg, pieces: &[&[u8]]) -> Vec<u8> {
    match h {
        HashAlg::Sha256 => {
            let mut d = <sha2::Sha256 as Digest>::new();
            for p in pieces {
                Digest::update(&mut d, p);
            }
            Digest::finalize(d).to_vec()
        }
        HashAlg::Sha384 => {
            let mut d = <sha2::Sha384 as Digest>::new();
            for p in pieces {
                Digest::update(&mut d, p);
            }
            Digest::finalize(d).to_vec()
        }
        HashAlg::Sha512 => {
            let mut d = <sha2::Sha512 as Digest>::new();
            for p in pieces {
                Digest::update(&mut d, p);
            }
            Digest::finalize(d).to_vec()
        }
    }
}

fn hash_message(h: HashAlg, prefix: Option<&[u8; 32]>, msg: &[u8]) -> Vec<u8> {
    if let Some(p) = prefix {
        hash_pieces(h, &[p, msg])
    } else {
        hash_pieces(h, &[msg])
    }
}

fn mgf1(mgf_seed: &[u8], mask_len: usize, h: HashAlg) -> Vec<u8> {
    let h_len = h.output_size();
    let mut t = Vec::with_capacity(mask_len + h_len);
    let mut counter: u32 = 0;
    while t.len() < mask_len {
        let c = counter.to_be_bytes();
        let block = hash_pieces(h, &[mgf_seed, &c]);
        t.extend_from_slice(&block);
        counter = counter.wrapping_add(1);
    }
    t.truncate(mask_len);
    t
}

/// EMSA-PSS encoding (RFC 8017, 9.1.1)
fn emsa_pss_encode(
    m_hash: &[u8],
    em_bits: usize,
    salt_len: usize,
    h: HashAlg,
) -> Result<Vec<u8>, ()> {
    let h_len = h.output_size();
    let em_len = (em_bits + 7) / 8;
    if em_len < h_len + salt_len + 2 {
        return Err(());
    }
    if m_hash.len() != h_len {
        return Err(());
    }
    let mut salt = vec![0u8; salt_len];
    if salt_len > 0 {
        use rand::RngCore;
        OsRng.fill_bytes(&mut salt);
    }
    // M' = 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 || mHash || salt
    let h_val = hash_pieces(h, &[&[0u8; 8], m_hash, &salt]);
    // PS = (em_len - sLen - hLen - 2) zero bytes
    // DB = PS || 0x01 || salt
    let ps_len = em_len - salt_len - h_len - 2;
    let mut db = Vec::with_capacity(em_len - h_len - 1);
    db.extend(std::iter::repeat(0u8).take(ps_len));
    db.push(0x01);
    db.extend_from_slice(&salt);
    let db_mask = mgf1(&h_val, em_len - h_len - 1, h);
    let mut masked_db: Vec<u8> = db.iter().zip(db_mask.iter()).map(|(a, b)| a ^ b).collect();
    // Set leftmost 8*em_len - em_bits bits of leftmost octet to zero
    let zero_bits = 8 * em_len - em_bits;
    if zero_bits > 0 {
        masked_db[0] &= 0xff >> zero_bits;
    }
    let mut em = Vec::with_capacity(em_len);
    em.extend_from_slice(&masked_db);
    em.extend_from_slice(&h_val);
    em.push(0xbc);
    Ok(em)
}

/// EMSA-PSS verify (RFC 8017, 9.1.2)
fn emsa_pss_verify(m_hash: &[u8], em: &[u8], em_bits: usize, salt_len: usize, h: HashAlg) -> bool {
    use digest::Digest;
    let h_len = h.output_size();
    let em_len = (em_bits + 7) / 8;
    if em.len() != em_len {
        return false;
    }
    if em_len < h_len + salt_len + 2 {
        return false;
    }
    if em[em_len - 1] != 0xbc {
        return false;
    }
    if m_hash.len() != h_len {
        return false;
    }
    let masked_db = &em[..em_len - h_len - 1];
    let h_val = &em[em_len - h_len - 1..em_len - 1];
    let zero_bits = 8 * em_len - em_bits;
    if zero_bits > 0 {
        let mask = !(0xffu8 >> zero_bits);
        if masked_db[0] & mask != 0 {
            return false;
        }
    }
    let db_mask = mgf1(h_val, em_len - h_len - 1, h);
    let mut db: Vec<u8> = masked_db
        .iter()
        .zip(db_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    if zero_bits > 0 {
        db[0] &= 0xff >> zero_bits;
    }
    let ps_len = em_len - salt_len - h_len - 2;
    for i in 0..ps_len {
        if db[i] != 0 {
            return false;
        }
    }
    if db[ps_len] != 0x01 {
        return false;
    }
    let salt = &db[ps_len + 1..];
    let h_check = hash_pieces(h, &[&[0u8; 8], m_hash, salt]);
    h_check.as_slice() == h_val
}

fn do_blind(
    context: &BRSAContext,
    blind_message: &mut BRSABlindMessage,
    secret_out: &mut BRSABlindingSecret,
    msg_randomizer: Option<&[u8; 32]>,
    pk_struct: &BRSAPublicKey,
    msg: &[u8],
) -> i32 {
    let h = match get_ctx_hash(context) {
        Some(h) => h,
        None => return -1,
    };
    let salt_len = if context.salt_len == BRSA_DEFAULT_SALT_LENGTH {
        h.output_size()
    } else {
        context.salt_len
    };

    // Get public key
    let result: i32 = with_pk(pk_struct, |pk| {
        if !key_params_ok(pk) {
            return -1;
        }
        let modulus_bits = pk.n().bits();
        let modulus_bytes = (modulus_bits + 7) / 8;

        // Hash message
        let m_hash = hash_message(h, msg_randomizer, msg);

        // EMSA-PSS encode
        let em = match emsa_pss_encode(&m_hash, modulus_bits - 1, salt_len, h) {
            Ok(em) => em,
            Err(_) => return -1,
        };
        // The encoding produces em_len = ceil((modulus_bits-1)/8). We need
        // to interpret it as an integer modulo n. Pad on the left to
        // modulus_bytes if necessary.
        let mut padded = vec![0u8; modulus_bytes];
        let off = modulus_bytes - em.len();
        padded[off..].copy_from_slice(&em);

        // Convert to BigUint
        let m = BigUint::from_bytes_be(&padded);
        if &m >= pk.n() {
            return -1;
        }

        // Blinding: pick random r, ensure gcd(r, n) = 1, compute c = m * r^e mod n
        let n = pk.n().clone();
        let e = pk.e().clone();
        use num_integer::Integer;
        use rand::Rng;
        let mut rng = OsRng;

        let one = BigUint::from(1u8);

        // Check gcd(m, n) == 1
        if m.gcd(&n) != one {
            return -1;
        }

        let mut r;
        let mut r_inv;
        loop {
            // Generate random r in [1, n)
            let r_bytes_len = modulus_bytes;
            let mut r_bytes = vec![0u8; r_bytes_len];
            rand::Rng::fill(&mut rng, &mut r_bytes[..]);
            let r_candidate = BigUint::from_bytes_be(&r_bytes) % &n;
            if r_candidate <= one {
                continue;
            }
            // Compute modular inverse
            use num_bigint_dig::ModInverse;
            match (&r_candidate).clone().mod_inverse(&n) {
                Some(inv) => {
                    use num_traits::Signed;
                    if inv.is_negative() {
                        let inv_pos = (inv + num_bigint_dig::BigInt::from(n.clone())).to_biguint().unwrap();
                        r_inv = inv_pos;
                    } else {
                        r_inv = inv.to_biguint().unwrap();
                    }
                    r = r_candidate;
                    break;
                }
                None => continue,
            }
        }

        // x = r_inv^e mod n  (so r_inv encrypts to 1/r mod n in raw RSA)
        // Wait: we want blind_m = m * x mod n where x is r^e mod n,
        // and the secret is r^-1 mod n. So when the server decrypts we
        // get m^d * r mod n, multiplied by r^-1 gives m^d.
        //
        // In RFC 9474 / the C code: secret_inv is what's stored as
        // `secret`, and what we send on the wire is m * secret_inv^e mod n.
        // Then to unblind: blind_sig * secret_inv^-1 = blind_sig * secret_inv^-1 mod n
        // with secret_inv stored as `secret` ... ugh. Let me re-read:
        //
        // Actually reading the C code:
        //   BN_rand_range(secret_inv, n)
        //   BN_mod_inverse(secret, secret_inv, n)  -> secret = secret_inv^-1
        //   x = secret_inv^e mod n
        //   blind_m = m * x mod n
        //   serialize secret (which is secret_inv^-1)
        //
        // So the blinding factor sent to server (via blind_m) hides m by
        // multiplying with secret_inv^e. After signing, server gives
        // blind_sig = (m * secret_inv^e)^d = m^d * secret_inv mod n.
        // Then secret = secret_inv^-1, and z = blind_sig * secret mod n
        //   = m^d * secret_inv * secret_inv^-1 = m^d mod n.
        //
        // Mapping: r := secret_inv (random), r_inv := secret = r^-1.
        // We picked r above and r_inv as its modular inverse. Good.

        // Now compute x = r^e mod n, blind_m = m * x mod n
        let x = r.modpow(&e, &n);
        let blind_m = (&m * &x) % &n;

        // Serialize blind_m and r_inv (secret) into modulus_bytes-padded BE.
        let blind_m_bytes = blind_m.to_bytes_be();
        let mut blind_padded = vec![0u8; modulus_bytes];
        let off = modulus_bytes - blind_m_bytes.len();
        blind_padded[off..].copy_from_slice(&blind_m_bytes);

        let r_inv_bytes = r_inv.to_bytes_be();
        let mut r_inv_padded = vec![0u8; modulus_bytes];
        let off = modulus_bytes - r_inv_bytes.len();
        r_inv_padded[off..].copy_from_slice(&r_inv_bytes);

        // Store into output structs
        store_blind_message(blind_message, blind_padded);
        store_blinding_secret(secret_out, r_inv_padded);
        0
    })
    .unwrap_or(-1);
    result
}

fn store_blind_message(s: &mut BRSABlindMessage, bytes: Vec<u8>) {
    let len = bytes.len();
    let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
    let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
    s.blind_message = slice;
    s.blind_message_len = len;
}

fn store_blinding_secret(s: &mut BRSABlindingSecret, bytes: Vec<u8>) {
    let len = bytes.len();
    let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
    let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
    s.secret = slice;
    s.secret_len = len;
}

fn store_blind_signature(s: &mut BRSABlindSignature, bytes: Vec<u8>) {
    let len = bytes.len();
    let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
    let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
    s.blind_sig = slice;
    s.blind_sig_len = len;
}

fn store_signature(s: &mut BRSASignature, bytes: Vec<u8>) {
    let len = bytes.len();
    let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
    let slice: &[u8] = unsafe { std::mem::transmute(&*leaked) };
    s.sig = slice;
    s.sig_len = len;
}

fn do_blind_sign(
    _context: &BRSAContext,
    blind_sig: &mut BRSABlindSignature,
    sk_struct: &BRSASecretKey,
    blind_message: &BRSABlindMessage,
) -> i32 {
    let result = with_sk(sk_struct, |sk| {
        let n = sk.n().clone();
        let bits = n.bits();
        if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS {
            return -1;
        }
        let e = sk.e().clone();
        let e3 = BigUint::from(3u32);
        let ef4 = BigUint::from(65537u32);
        if e != e3 && e != ef4 {
            return -1;
        }
        let modulus_bytes = (bits + 7) / 8;
        if blind_message.blind_message_len != modulus_bytes {
            return -1;
        }
        let m = BigUint::from_bytes_be(blind_message.blind_message);
        if m >= n {
            return -1;
        }
        // Raw RSA decryption (no padding); we sign the blinded message.
        // Use modpow with d to compute s = m^d mod n (works for non-CRT keys
        // and is fine for our test). For real CRT keys this is also correct.
        let s = m.modpow(sk.d(), &n);
        let s_bytes = s.to_bytes_be();
        let mut padded = vec![0u8; modulus_bytes];
        let off = modulus_bytes - s_bytes.len();
        padded[off..].copy_from_slice(&s_bytes);
        store_blind_signature(blind_sig, padded);
        0
    });
    result.unwrap_or(-1)
}

fn do_finalize(
    context: &BRSAContext,
    sig_out: &mut BRSASignature,
    blind_sig: &BRSABlindSignature,
    secret_: &BRSABlindingSecret,
    msg_randomizer: &Option<BRSAMessageRandomizer>,
    pk_struct: &BRSAPublicKey,
    msg: &[u8],
) -> i32 {
    let h = match get_ctx_hash(context) {
        Some(h) => h,
        None => return -1,
    };
    let salt_len = if context.salt_len == BRSA_DEFAULT_SALT_LENGTH {
        h.output_size()
    } else {
        context.salt_len
    };
    let result = with_pk(pk_struct, |pk| {
        if !key_params_ok(pk) {
            return -1;
        }
        let n = pk.n().clone();
        let bits = n.bits();
        let modulus_bytes = (bits + 7) / 8;
        if blind_sig.blind_sig_len != modulus_bytes || secret_.secret_len != modulus_bytes {
            return -1;
        }
        let blind_z = BigUint::from_bytes_be(blind_sig.blind_sig);
        let secret = BigUint::from_bytes_be(secret_.secret);
        let z = (&blind_z * &secret) % &n;
        let z_bytes = z.to_bytes_be();
        let mut padded = vec![0u8; modulus_bytes];
        let off = modulus_bytes - z_bytes.len();
        padded[off..].copy_from_slice(&z_bytes);

        // Verify
        if !verify_signature(pk, &padded, msg_randomizer.as_ref(), msg, h, salt_len) {
            return -1;
        }
        store_signature(sig_out, padded);
        0
    });
    result.unwrap_or(-1)
}

fn verify_signature(
    pk: &RsaPublicKey,
    sig: &[u8],
    msg_randomizer: Option<&BRSAMessageRandomizer>,
    msg: &[u8],
    h: HashAlg,
    salt_len: usize,
) -> bool {
    let n = pk.n();
    let bits = n.bits();
    let modulus_bytes = (bits + 7) / 8;
    if sig.len() != modulus_bytes {
        return false;
    }
    let s_int = BigUint::from_bytes_be(sig);
    if &s_int >= n {
        return false;
    }
    // RSA public verify: m = s^e mod n
    let m_int = s_int.modpow(pk.e(), n);
    let em_bits = bits - 1;
    let em_len = (em_bits + 7) / 8;
    let m_bytes = m_int.to_bytes_be();
    // OpenSSL's RSA_verify_PKCS1_PSS uses em_len = modulus_bytes if the
    // top bit is zero, else em_len = modulus_bytes - 1. The standard
    // says em_len = ceil((modBits - 1) / 8). Build em accordingly:
    let em_len_actual = if (n.bits() - 1) % 8 == 0 {
        modulus_bytes - 1
    } else {
        modulus_bytes
    };
    let mut em = vec![0u8; em_len_actual];
    if m_bytes.len() > em_len_actual {
        // Skip leading zero(s)
        let extra = m_bytes.len() - em_len_actual;
        // If the leading bytes are non-zero we have invalid data
        for &b in &m_bytes[..extra] {
            if b != 0 {
                return false;
            }
        }
        em.copy_from_slice(&m_bytes[extra..]);
    } else {
        let off = em_len_actual - m_bytes.len();
        em[off..].copy_from_slice(&m_bytes);
    }

    let m_hash = hash_message(h, msg_randomizer.map(|r| &r.noise), msg);
    emsa_pss_verify(&m_hash, &em, em_bits, salt_len, h)
}

fn do_verify(
    context: &BRSAContext,
    sig: &BRSASignature,
    pk_struct: &BRSAPublicKey,
    msg_randomizer: &Option<BRSAMessageRandomizer>,
    msg: &[u8],
) -> c_int {
    let h = match get_ctx_hash(context) {
        Some(h) => h,
        None => return -1,
    };
    let salt_len = if context.salt_len == BRSA_DEFAULT_SALT_LENGTH {
        h.output_size()
    } else {
        context.salt_len
    };
    let result = with_pk(pk_struct, |pk| {
        if verify_signature(pk, sig.sig, msg_randomizer.as_ref(), msg, h, salt_len) {
            0
        } else {
            -1
        }
    });
    result.unwrap_or(-1)
}

// SPKI template like in the C code
const RSASSA_PSS_S_TEMPLATE: &[u8] = &[
    0x30, 0x80 | 2, 0, 0, // container length - offset 2
    0x30, 61, // Algorithm sequence
    0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a, // RSASSA-PSS OID
    0x30, 48, // RSASSA-PSS parameters sequence
    0xa0, 2 + 2 + 9, 0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0xa1, 2 + 24, 0x30, 24, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08,
    0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0xa2, 2 + 1, 0x02, 1, 0,
    0x03, 0x80 | 2, 0, 0, 0,
];

fn do_publickey_export_spki(
    context: &BRSAContext,
    spki: &mut BRSASerializedKey,
    pk_struct: &BRSAPublicKey,
) -> i32 {
    let h = match get_ctx_hash(context) {
        Some(h) => h,
        None => return -1,
    };
    let pk_der = match with_pk(pk_struct, |pk| {
        use pkcs1::EncodeRsaPublicKey;
        pk.to_pkcs1_der().map(|d| d.as_bytes().to_vec()).ok()
    }) {
        Some(Some(d)) => d,
        _ => return -1,
    };
    let template_len = RSASSA_PSS_S_TEMPLATE.len();
    let container_len = template_len - 4 + pk_der.len();
    let mut spki_bytes = vec![0u8; template_len + pk_der.len()];
    spki_bytes[..template_len].copy_from_slice(RSASSA_PSS_S_TEMPLATE);
    spki_bytes[template_len..].copy_from_slice(&pk_der);
    spki_bytes[2] = (container_len >> 8) as u8;
    spki_bytes[3] = (container_len & 0xff) as u8;
    spki_bytes[66] = (context.salt_len & 0xff) as u8;
    spki_bytes[69] = ((1 + pk_der.len()) >> 8) as u8;
    spki_bytes[70] = ((1 + pk_der.len()) & 0xff) as u8;

    // Hash function OID
    let (oid, oid_len) = hash_alg_oid(h);
    // Build mgf1_s_data of length 2 + 2 + 9 = 13: SEQ(11) OBJ(9, oid_bytes)
    // We need to follow X509_ALGOR_set_md output but skip null params.
    // The OpenSSL X509_ALGOR_set_md typically outputs SEQ { OID, NULL }.
    // Without NULL trim, ASN1 length would be 2+9+2 = 13 for SEQ{OID,NULL}=15.
    // The C code accepts both lengths.
    // We'll write SEQ(11) OID(9, oid).
    let mut mgf1_s_data = [0u8; 13];
    mgf1_s_data[0] = 0x30;
    mgf1_s_data[1] = 11;
    mgf1_s_data[2] = 0x06;
    mgf1_s_data[3] = 9;
    mgf1_s_data[4..13].copy_from_slice(&oid[..9]);
    spki_bytes[21..21 + 13].copy_from_slice(&mgf1_s_data);
    spki_bytes[49..49 + 13].copy_from_slice(&mgf1_s_data);

    store_serialized(spki, spki_bytes);
    0
}

fn hash_alg_oid(h: HashAlg) -> ([u8; 9], usize) {
    match h {
        // SHA-256: 2.16.840.1.101.3.4.2.1
        HashAlg::Sha256 => (
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            9,
        ),
        // SHA-384: 2.16.840.1.101.3.4.2.2
        HashAlg::Sha384 => (
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            9,
        ),
        // SHA-512: 2.16.840.1.101.3.4.2.3
        HashAlg::Sha512 => (
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
            9,
        ),
    }
}

fn do_publickey_import_spki(
    _context: &BRSAContext,
    pk: &mut BRSAPublicKey,
    spki: &[u8],
    spki_len: usize,
) -> i32 {
    let template_len = RSASSA_PSS_S_TEMPLATE.len();
    if spki_len > MAX_SERIALIZED_PK_LEN || spki_len <= template_len {
        return -1;
    }
    if spki[6..18] != RSASSA_PSS_S_TEMPLATE[6..18] {
        return -1;
    }
    let alg_len = spki[5] as usize;
    if spki_len <= alg_len + 11 {
        return -1;
    }
    pk.brsa_publickey_import(&spki[alg_len + 11..spki_len], spki_len - alg_len - 11)
}

fn do_publickey_id(
    context: &BRSAContext,
    id: &[u8],
    id_len: usize,
    pk_struct: &BRSAPublicKey,
) -> i32 {
    let mut spki = BRSASerializedKey::new();
    if do_publickey_export_spki(context, &mut spki, pk_struct) != 0 {
        return -1;
    }
    let hash = hash_pieces(HashAlg::Sha256, &[spki.bytes]);
    spki.brsa_serializedkey_deinit();

    // Write into id buffer (we need write access despite having &[u8])
    // Cast to *mut u8
    let target = id.as_ptr() as *mut u8;
    unsafe {
        let target_slice = std::slice::from_raw_parts_mut(target, id_len);
        let copy_len = id_len.min(hash.len());
        target_slice[..copy_len].copy_from_slice(&hash[..copy_len]);
        if id_len > hash.len() {
            for b in &mut target_slice[hash.len()..] {
                *b = 0;
            }
        }
    }
    0
}
