use openssl_sys::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use digest::DynDigest;
use num_bigint_dig::traits::ModInverse;
use num_bigint_dig::{BigUint, RandBigInt};
use num_integer::Integer;
use num_traits::{One, Zero};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use sha2::{Sha256, Sha384, Sha512};
use digest::Digest;

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

// ========================================================================
// Side-table state (because the openssl-sys placeholder types are empty
// enums and cannot actually carry state).
// ========================================================================

#[derive(Clone)]
struct ContextState {
    hash: BRSAHashFunction,
}

#[derive(Clone)]
struct PublicKeyState {
    n: BigUint,
    e: BigUint,
}

#[derive(Clone)]
struct SecretKeyState {
    inner: RsaPrivateKey,
}

fn context_registry() -> &'static Mutex<HashMap<usize, ContextState>> {
    static R: OnceLock<Mutex<HashMap<usize, ContextState>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pk_registry() -> &'static Mutex<HashMap<usize, PublicKeyState>> {
    static R: OnceLock<Mutex<HashMap<usize, PublicKeyState>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sk_registry() -> &'static Mutex<HashMap<usize, SecretKeyState>> {
    static R: OnceLock<Mutex<HashMap<usize, SecretKeyState>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key_of<T>(p: *const T) -> usize {
    p as usize
}

fn ctx_get(ctx: *const BRSAContext) -> Option<ContextState> {
    context_registry().lock().unwrap().get(&key_of(ctx)).cloned()
}

fn ctx_set(ctx: *const BRSAContext, st: ContextState) {
    context_registry().lock().unwrap().insert(key_of(ctx), st);
}

fn pk_get(pk: *const BRSAPublicKey) -> Option<PublicKeyState> {
    pk_registry().lock().unwrap().get(&key_of(pk)).cloned()
}

fn pk_set(pk: *const BRSAPublicKey, st: PublicKeyState) {
    pk_registry().lock().unwrap().insert(key_of(pk), st);
}

fn pk_remove(pk: *const BRSAPublicKey) {
    pk_registry().lock().unwrap().remove(&key_of(pk));
}

fn sk_get(sk: *const BRSASecretKey) -> Option<SecretKeyState> {
    sk_registry().lock().unwrap().get(&key_of(sk)).cloned()
}

fn sk_set(sk: *const BRSASecretKey, st: SecretKeyState) {
    sk_registry().lock().unwrap().insert(key_of(sk), st);
}

fn sk_remove(sk: *const BRSASecretKey) {
    sk_registry().lock().unwrap().remove(&key_of(sk));
}

fn hash_output_size(h: BRSAHashFunction) -> usize {
    match h {
        BRSAHashFunction::BRSA_SHA256 => 32,
        BRSAHashFunction::BRSA_SHA384 => 48,
        BRSAHashFunction::BRSA_SHA512 => 64,
    }
}

fn make_digest(h: BRSAHashFunction) -> Box<dyn DynDigest> {
    match h {
        BRSAHashFunction::BRSA_SHA256 => Box::new(Sha256::new()),
        BRSAHashFunction::BRSA_SHA384 => Box::new(Sha384::new()),
        BRSAHashFunction::BRSA_SHA512 => Box::new(Sha512::new()),
    }
}

fn hash_msg(h: BRSAHashFunction, prefix: Option<&[u8; 32]>, msg: &[u8]) -> Vec<u8> {
    match h {
        BRSAHashFunction::BRSA_SHA256 => {
            let mut hasher = Sha256::new();
            if let Some(p) = prefix {
                Digest::update(&mut hasher, p);
            }
            Digest::update(&mut hasher, msg);
            hasher.finalize().to_vec()
        }
        BRSAHashFunction::BRSA_SHA384 => {
            let mut hasher = Sha384::new();
            if let Some(p) = prefix {
                Digest::update(&mut hasher, p);
            }
            Digest::update(&mut hasher, msg);
            hasher.finalize().to_vec()
        }
        BRSAHashFunction::BRSA_SHA512 => {
            let mut hasher = Sha512::new();
            if let Some(p) = prefix {
                Digest::update(&mut hasher, p);
            }
            Digest::update(&mut hasher, msg);
            hasher.finalize().to_vec()
        }
    }
}

// ============================================================================
// MGF1 mask generation
// ============================================================================
fn mgf1_xor(out: &mut [u8], digest: &mut dyn DynDigest, seed: &[u8]) {
    let mut counter: u32 = 0;
    let mut i = 0;
    while i < out.len() {
        digest.update(seed);
        let c_be = counter.to_be_bytes();
        digest.update(&c_be);
        let h = digest.finalize_reset();
        let mut j = 0;
        while j < h.len() && i < out.len() {
            out[i] ^= h[j];
            j += 1;
            i += 1;
        }
        counter = counter.wrapping_add(1);
    }
}

// ============================================================================
// EMSA-PSS encode (deterministic given salt)
// ============================================================================
fn emsa_pss_encode(
    m_hash: &[u8],
    em_bits: usize,
    salt: &[u8],
    hash: &mut dyn DynDigest,
) -> Result<Vec<u8>, ()> {
    let h_len = hash.output_size();
    let s_len = salt.len();
    let em_len = (em_bits + 7) / 8;

    if m_hash.len() != h_len {
        return Err(());
    }
    if em_len < h_len + s_len + 2 {
        return Err(());
    }
    let mut em = vec![0u8; em_len];
    let (db, h_part) = em.split_at_mut(em_len - h_len - 1);
    let h_part = &mut h_part[..(em_len - 1) - db.len()];

    // M' = 8 zero bytes || m_hash || salt
    let prefix = [0u8; 8];
    hash.update(&prefix);
    hash.update(m_hash);
    hash.update(salt);
    let hashed = hash.finalize_reset();
    h_part.copy_from_slice(&hashed);

    db[em_len - s_len - h_len - 2] = 0x01;
    db[em_len - s_len - h_len - 1..].copy_from_slice(salt);

    mgf1_xor(db, hash, h_part);

    let extra_bits = 8 * em_len - em_bits;
    if extra_bits > 0 {
        db[0] &= 0xFF >> extra_bits;
    }

    em[em_len - 1] = 0xBC;
    Ok(em)
}

// ============================================================================
// EMSA-PSS verify
// ============================================================================
fn emsa_pss_verify(
    m_hash: &[u8],
    em: &mut [u8],
    s_len: usize,
    hash: &mut dyn DynDigest,
    key_bits: usize,
) -> Result<(), ()> {
    let em_bits = key_bits - 1;
    let em_len = (em_bits + 7) / 8;
    let key_len = (key_bits + 7) / 8;
    let h_len = hash.output_size();

    if em.len() < key_len {
        return Err(());
    }
    let em = &mut em[key_len - em_len..];

    if m_hash.len() != h_len {
        return Err(());
    }
    if em_len < h_len + s_len + 2 {
        return Err(());
    }
    if em[em.len() - 1] != 0xBC {
        return Err(());
    }
    let (db, h_part) = em.split_at_mut(em_len - h_len - 1);
    let h_part = &mut h_part[..h_len];

    let extra_bits = 8 * em_len - em_bits;
    if extra_bits > 0 {
        if db[0] & (0xFFu8.checked_shl(8 - extra_bits as u32).unwrap_or(0)) != 0 {
            return Err(());
        }
    }

    mgf1_xor(db, hash, h_part);

    if extra_bits > 0 {
        db[0] &= 0xFF >> extra_bits;
    }

    // 10. Check: leading bytes are zero, followed by 0x01
    if db.len() < em_len - h_len - s_len - 1 {
        return Err(());
    }
    let zero_len = em_len - h_len - s_len - 2;
    for &b in &db[..zero_len] {
        if b != 0 {
            return Err(());
        }
    }
    if db[zero_len] != 0x01 {
        return Err(());
    }
    let salt = &db[db.len() - s_len..];

    let prefix = [0u8; 8];
    hash.update(&prefix);
    hash.update(m_hash);
    hash.update(salt);
    let h0 = hash.finalize_reset();

    if h0.as_ref() != h_part {
        return Err(());
    }
    Ok(())
}

// ============================================================================
// BIGNUM serialization (big-endian, padded)
// ============================================================================
fn bn_to_padded(b: &BigUint, len: usize) -> Option<Vec<u8>> {
    let bytes = b.to_bytes_be();
    if bytes.len() > len {
        return None;
    }
    let mut out = vec![0u8; len];
    let off = len - bytes.len();
    out[off..].copy_from_slice(&bytes);
    Some(out)
}

// ============================================================================
// PKCS#1 RSAPublicKey/RSAPrivateKey DER encoding (matches OpenSSL i2d_PublicKey
// /i2d_PrivateKey for EVP_PKEY_RSA)
// ============================================================================

// DER encoding helpers
fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len < 0x100 {
        vec![0x81, len as u8]
    } else if len < 0x10000 {
        vec![0x82, (len >> 8) as u8, (len & 0xff) as u8]
    } else if len < 0x1000000 {
        vec![0x83, (len >> 16) as u8, ((len >> 8) & 0xff) as u8, (len & 0xff) as u8]
    } else {
        vec![0x84, (len >> 24) as u8, ((len >> 16) & 0xff) as u8, ((len >> 8) & 0xff) as u8, (len & 0xff) as u8]
    }
}

fn der_integer(value: &BigUint) -> Vec<u8> {
    let mut bytes = value.to_bytes_be();
    if bytes.is_empty() {
        bytes = vec![0u8];
    }
    // If the high bit of the first byte is set, prepend a 0x00 byte
    if bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0x00);
    }
    let mut out = vec![0x02];
    out.extend(der_len(bytes.len()));
    out.extend(bytes);
    out
}

fn der_sequence(content: &[u8]) -> Vec<u8> {
    let mut out = vec![0x30];
    out.extend(der_len(content.len()));
    out.extend(content);
    out
}

fn encode_rsa_public_key_pkcs1(n: &BigUint, e: &BigUint) -> Vec<u8> {
    let mut content = Vec::new();
    content.extend(der_integer(n));
    content.extend(der_integer(e));
    der_sequence(&content)
}

fn encode_rsa_private_key_pkcs1(sk: &RsaPrivateKey) -> Vec<u8> {
    let n = sk.n();
    let e = sk.e();
    let d = sk.d();
    let primes = sk.primes();
    let p = &primes[0];
    let q = &primes[1];
    let one = BigUint::one();
    let dp = d % (p - &one);
    let dq = d % (q - &one);
    let qinv = q
        .clone()
        .mod_inverse(p.clone())
        .unwrap_or_else(|| 0.into());
    let qinv_bu = if qinv.sign() == num_bigint_dig::Sign::Minus {
        // shouldn't happen for valid key
        BigUint::zero()
    } else {
        qinv.to_biguint().unwrap_or(BigUint::zero())
    };

    let mut content = Vec::new();
    content.extend(der_integer(&BigUint::zero())); // version
    content.extend(der_integer(n));
    content.extend(der_integer(e));
    content.extend(der_integer(d));
    content.extend(der_integer(p));
    content.extend(der_integer(q));
    content.extend(der_integer(&dp));
    content.extend(der_integer(&dq));
    content.extend(der_integer(&qinv_bu));
    der_sequence(&content)
}

// Minimal DER decoding for RSA PKCS#1 keys
struct DerReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> DerReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    fn read_len(&mut self) -> Option<usize> {
        let first = self.read_byte()?;
        if first < 0x80 {
            Some(first as usize)
        } else {
            let n = (first & 0x7fu8) as usize;
            if n == 0 || n > 4 {
                return None;
            }
            let mut len = 0usize;
            for _ in 0..n {
                let b = self.read_byte()?;
                len = (len << 8) | b as usize;
            }
            Some(len)
        }
    }

    fn read_tag_len(&mut self, tag: u8) -> Option<usize> {
        let t = self.read_byte()?;
        if t != tag {
            return None;
        }
        self.read_len()
    }

    fn read_bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.pos + n > self.buf.len() {
            return None;
        }
        let r = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Some(r)
    }

    fn read_integer(&mut self) -> Option<BigUint> {
        let len = self.read_tag_len(0x02)?;
        let bytes = self.read_bytes(len)?;
        // Skip leading zeros (allowed for sign)
        let bytes = if bytes.is_empty() {
            bytes
        } else if bytes[0] == 0 && bytes.len() > 1 {
            &bytes[1..]
        } else {
            bytes
        };
        Some(BigUint::from_bytes_be(bytes))
    }
}

fn decode_rsa_public_key_pkcs1(der: &[u8]) -> Option<(BigUint, BigUint)> {
    let mut r = DerReader::new(der);
    let seq_len = r.read_tag_len(0x30)?;
    let inner_buf = r.read_bytes(seq_len)?;
    let mut inner = DerReader::new(inner_buf);
    let n = inner.read_integer()?;
    let e = inner.read_integer()?;
    Some((n, e))
}

fn decode_rsa_private_key_pkcs1(der: &[u8]) -> Option<RsaPrivateKey> {
    let mut r = DerReader::new(der);
    let seq_len = r.read_tag_len(0x30)?;
    let inner_buf = r.read_bytes(seq_len)?;
    let mut inner = DerReader::new(inner_buf);
    let _version = inner.read_integer()?;
    let n = inner.read_integer()?;
    let e = inner.read_integer()?;
    let d = inner.read_integer()?;
    let p = inner.read_integer()?;
    let q = inner.read_integer()?;
    let _dp = inner.read_integer()?;
    let _dq = inner.read_integer()?;
    let _qinv = inner.read_integer()?;
    RsaPrivateKey::from_components(n, e, d, vec![p, q]).ok()
}

// ============================================================================
// Helper: check RSA params
// ============================================================================
fn rsa_params_check(n_bits: usize, e: &BigUint) -> bool {
    if n_bits < MIN_MODULUS_BITS || n_bits > MAX_MODULUS_BITS {
        return false;
    }
    let e3 = BigUint::from(3u32);
    let e_f4 = BigUint::from(65537u32);
    e == &e3 || e == &e_f4
}

// ============================================================================
// SPKI encoding for RSASSA-PSS (matches the C template)
// ============================================================================
const RSASSA_PSS_S_TEMPLATE: [u8; 72] = [
    0x30, 0x80 | 2, 0, 0, // SEQ, length-2bytes (filled in)  — bytes 0..=3
        0x30, 61, // Algorithm sequence  — bytes 4..=5
            0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a, // RSASSA-PSS OID  — 6..=16
            0x30, 48, // RSASSA-PSS parameters  — 17..=18
                0xa0, 2 + 2 + 9, // ctx[0]  — 19..=20
                0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // hash AlgorithmIdentifier  — 21..=33
                0xa1, 2 + 24, // ctx[1]  — 34..=35
                0x30, 24, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08, // MGF1 alg  — 36..=48
                    0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // MGF1 inner hash  — 49..=61
                0xa2, 2 + 1, 0x02, 1, 0, // salt length  — 62..=66
        0x03, 0x80 | 2, 0, 0, // BIT STRING length-2bytes  — 67..=70
            0, // 71
];

// MGF1 inner sequence data (2+9 bytes for hash AlgorithmIdentifier embedded
// in BIT STRING in MGF1 parameters). The format produced by openssl X509_ALGOR_set_md.
// For SHA256: 30 0d 06 09 60 86 48 01 65 03 04 02 01 - 13 bytes (with NULL)
// For SHA384: 30 0d 06 09 60 86 48 01 65 03 04 02 02
// For SHA512: 30 0d 06 09 60 86 48 01 65 03 04 02 03
// The C code expects 13-byte form (with NULL) and trims to 11-byte form (without NULL).
// Actually re-reading C code:
//   sizeof mgf1_s_data = 2 + 2 + 9 = 13
// The expected layout is 2+2+9 = 13 bytes:
//   30 LL 06 09 OID(9)
// Wait that's only 11 bytes (1+1+1+1+9=13). Actually 2+2+9 means tag+len + tag+len + content?
// Actually `2 + 2 + 9 = 13`, and the actual encoding is:
//   30 0b 06 09 OID(9)  -- 2+2+9=13 bytes total, no NULL parameters
// X509_ALGOR_set_md by default uses NULL parameters: 30 0d 06 09 OID 05 00 = 15 bytes
// So the 15-byte form gets trimmed to 13-byte form.

fn hash_oid(h: BRSAHashFunction) -> [u8; 9] {
    match h {
        BRSAHashFunction::BRSA_SHA256 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
        BRSAHashFunction::BRSA_SHA384 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
        BRSAHashFunction::BRSA_SHA512 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
    }
}

// Build the 13-byte hash AlgorithmIdentifier: 30 0b 06 09 OID(9), no parameters
fn hash_alg_id_no_null(h: BRSAHashFunction) -> [u8; 13] {
    let oid = hash_oid(h);
    let mut out = [0u8; 13];
    out[0] = 0x30;
    out[1] = 11; // 2 + 9
    out[2] = 0x06;
    out[3] = 9;
    out[4..13].copy_from_slice(&oid);
    out
}

// ============================================================================
// BRSAContext methods
// ============================================================================
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
        self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    }
    pub fn brsa_context_init_deterministic(&mut self) {
        self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 0);
    }
    pub fn brsa_context_init_custom(
        &mut self,
        hash_function: BRSAHashFunction,
        salt_len: usize,
    ) -> i32 {
        self.evp_md = None;
        if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            self.salt_len = hash_output_size(hash_function);
        } else {
            self.salt_len = salt_len;
        }
        ctx_set(self as *const _, ContextState { hash: hash_function });
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
        // Fill msg with random bytes (the C function takes a writable uint8_t *msg)
        let msg_ptr = msg.as_ptr() as *mut u8;
        unsafe {
            let slice = std::slice::from_raw_parts_mut(msg_ptr, msg_len);
            use rand::RngCore;
            rand::thread_rng().fill_bytes(slice);
            self.do_blind(blind_message, secret, None, pk, slice, msg_len)
        }
    }
    fn do_blind(
        &self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: Option<&mut BRSAMessageRandomizer>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
        msg_len: usize,
    ) -> i32 {
        let pk_state = match pk_get(pk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let ctx_state = match ctx_get(self as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let modulus_bits = pk_state.n.bits();
        if !rsa_params_check(modulus_bits, &pk_state.e) {
            return -1;
        }
        let modulus_bytes = (modulus_bits + 7) / 8;

        // Fill randomizer if requested
        let prefix_arr = match msg_randomizer {
            Some(m) => {
                use rand::RngCore;
                rand::thread_rng().fill_bytes(&mut m.noise);
                Some(m.noise)
            }
            None => None,
        };

        // Hash the message
        let m_hash = hash_msg(ctx_state.hash, prefix_arr.as_ref(), &msg[..msg_len]);

        // EMSA-PSS encode
        let salt_len = self.salt_len;
        let mut salt = vec![0u8; salt_len];
        if salt_len > 0 {
            use rand::RngCore;
            rand::thread_rng().fill_bytes(&mut salt);
        }
        let em_bits = modulus_bits - 1;
        let mut digest = make_digest(ctx_state.hash);
        let em = match emsa_pss_encode(&m_hash, em_bits, &salt, digest.as_mut()) {
            Ok(e) => e,
            Err(_) => return -1,
        };
        // Pad em to modulus_bytes (left-pad with zeros if shorter)
        let padded = if em.len() == modulus_bytes {
            em
        } else if em.len() < modulus_bytes {
            let mut p = vec![0u8; modulus_bytes];
            p[modulus_bytes - em.len()..].copy_from_slice(&em);
            p
        } else {
            return -1;
        };

        _blind_internal(blind_message, secret, &pk_state, &padded, modulus_bytes)
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
        self.do_blind(blind_message, secret, Some(msg_randomizer), pk, msg, msg_len)
    }
    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let sk_state = match sk_get(sk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let n = sk_state.inner.n();
        let modulus_bits = n.bits();
        if !rsa_params_check(modulus_bits, sk_state.inner.e()) {
            return -1;
        }
        let modulus_bytes = (modulus_bits + 7) / 8;

        if blind_message.blind_message.len() != modulus_bytes {
            return -1;
        }

        // Check canonical: blind_message < n
        let m = BigUint::from_bytes_be(blind_message.blind_message);
        if &m >= n {
            return -1;
        }

        // Sign: c = m^d mod n, using rsa::hazmat
        let s = match rsa::hazmat::rsa_decrypt::<rand::rngs::OsRng>(None, &sk_state.inner, &m) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let s_bytes = match bn_to_padded(&s, modulus_bytes) {
            Some(b) => b,
            None => return -1,
        };
        let leaked: &'static [u8] = Box::leak(s_bytes.into_boxed_slice());
        blind_sig.blind_sig = leaked;
        blind_sig.blind_sig_len = modulus_bytes;
        0
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
        let pk_state = match pk_get(pk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        if !rsa_params_check(pk_state.n.bits(), &pk_state.e) {
            return -1;
        }
        let modulus_bits = pk_state.n.bits();
        let modulus_bytes = (modulus_bits + 7) / 8;
        if blind_sig.blind_sig_len != modulus_bytes || secret_.secret_len != modulus_bytes {
            return -1;
        }
        let secret_bn = BigUint::from_bytes_be(secret_.secret);
        let blind_z = BigUint::from_bytes_be(blind_sig.blind_sig);
        let z = (blind_z * secret_bn) % &pk_state.n;
        let z_bytes = match bn_to_padded(&z, modulus_bytes) {
            Some(b) => b,
            None => return -1,
        };
        let leaked: &'static [u8] = Box::leak(z_bytes.clone().into_boxed_slice());
        sig.sig = leaked;
        sig.sig_len = modulus_bytes;

        // Verify
        if self.brsa_verify(sig, pk, msg_randomizer, msg, msg_len) != 0 {
            return -1;
        }
        0
    }
    pub fn brsa_verify(
        &self,
        sig: &BRSASignature,
        pk: &mut BRSAPublicKey,
        msg_randomizer: &Option<BRSAMessageRandomizer>,
        msg: &[u8],
        msg_len: usize,
    ) -> c_int {
        let pk_state = match pk_get(pk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let ctx_state = match ctx_get(self as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let modulus_bits = pk_state.n.bits();
        let modulus_bytes = (modulus_bits + 7) / 8;
        if sig.sig_len != modulus_bytes {
            return -1;
        }
        // Hash message
        let prefix_arr = msg_randomizer.as_ref().map(|m| m.noise);
        let m_hash = hash_msg(
            ctx_state.hash,
            prefix_arr.as_ref(),
            &msg[..msg_len],
        );

        // RSA public verify: m = sig^e mod n
        let s_bn = BigUint::from_bytes_be(sig.sig);
        if &s_bn >= &pk_state.n {
            return -1;
        }
        let m = match rsa::hazmat::rsa_encrypt(
            &RsaPublicKey::new_unchecked(pk_state.n.clone(), pk_state.e.clone()),
            &s_bn,
        ) {
            Ok(v) => v,
            Err(_) => return -1,
        };
        let mut em = match bn_to_padded(&m, modulus_bytes) {
            Some(b) => b,
            None => return -1,
        };

        let mut digest = make_digest(ctx_state.hash);
        match emsa_pss_verify(
            &m_hash,
            &mut em,
            self.salt_len,
            digest.as_mut(),
            modulus_bits,
        ) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_state = match pk_get(pk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let ctx_state = match ctx_get(self as *const _) {
            Some(s) => s,
            None => return -1,
        };

        let raw = encode_rsa_public_key_pkcs1(&pk_state.n, &pk_state.e);
        let raw_len = raw.len();

        let template_len = RSASSA_PSS_S_TEMPLATE.len();
        let container_len = template_len - 4 + raw_len;
        let mut spki_bytes = vec![0u8; template_len + raw_len];
        spki_bytes[..template_len].copy_from_slice(&RSASSA_PSS_S_TEMPLATE);
        spki_bytes[template_len..].copy_from_slice(&raw);

        spki_bytes[2] = (container_len >> 8) as u8;
        spki_bytes[3] = (container_len & 0xff) as u8;
        spki_bytes[66] = (self.salt_len & 0xff) as u8;
        spki_bytes[69] = ((1 + raw_len) >> 8) as u8;
        spki_bytes[70] = ((1 + raw_len) & 0xff) as u8;

        let alg_id = hash_alg_id_no_null(ctx_state.hash);
        spki_bytes[21..21 + 13].copy_from_slice(&alg_id);
        spki_bytes[49..49 + 13].copy_from_slice(&alg_id);

        let leaked: &'static [u8] = Box::leak(spki_bytes.clone().into_boxed_slice());
        spki.bytes = leaked;
        spki.bytes_len = template_len + raw_len;
        0
    }
    pub fn brsa_publickey_import_spki(
        &self,
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
        pk.brsa_publickey_import(&spki[alg_len + 11..], spki_len - alg_len - 11)
    }
    pub fn brsa_publickey_id(
        &self,
        id: &[u8],
        id_len: usize,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let mut spki = BRSASerializedKey {
            bytes: &[],
            bytes_len: 0,
        };
        if self.brsa_publickey_export_spki(&mut spki, pk) != 0 {
            return -1;
        }
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, spki.bytes);
        let h = hasher.finalize();
        let id_ptr = id.as_ptr() as *mut u8;
        unsafe {
            let id_slice = std::slice::from_raw_parts_mut(id_ptr, id_len);
            let mut out_len = id_len;
            if out_len > h.len() {
                out_len = h.len();
                for i in id_slice[out_len..].iter_mut() {
                    *i = 0;
                }
            }
            id_slice[..out_len].copy_from_slice(&h[..out_len]);
        }
        spki.brsa_serializedkey_deinit();
        0
    }
}

// ============================================================================
// _blind helper (works on a private state pair instead of bn_ctx)
// ============================================================================
fn _blind_internal(
    blind_message: &mut BRSABlindMessage,
    secret: &mut BRSABlindingSecret,
    pk_state: &PublicKeyState,
    padded: &[u8],
    modulus_bytes: usize,
) -> i32 {
    let m = BigUint::from_bytes_be(padded);
    let n = &pk_state.n;
    let e = &pk_state.e;

    // gcd(m, n) == 1
    if m.gcd(n) != BigUint::one() {
        return -1;
    }

    // pick random secret_inv coprime with n, compute its inverse `secret`
    let mut rng = rand::thread_rng();
    let one = BigUint::one();
    let secret_inv;
    let secret_bn;
    loop {
        let r = rng.gen_biguint_below(n);
        if r <= one {
            continue;
        }
        if let Some(inv) = r.clone().mod_inverse(n.clone()) {
            // inv may be a BigInt, convert to BigUint mod n
            let inv_bu = match inv.to_biguint() {
                Some(v) => v,
                None => continue,
            };
            secret_inv = r;
            secret_bn = inv_bu;
            break;
        }
    }

    // x = secret_inv^e mod n
    let x = secret_inv.modpow(e, n);
    // blind_m = m * x mod n
    let blind_m = (m * x) % n;

    let blind_m_bytes = match bn_to_padded(&blind_m, modulus_bytes) {
        Some(b) => b,
        None => return -1,
    };
    let secret_bytes = match bn_to_padded(&secret_bn, modulus_bytes) {
        Some(b) => b,
        None => return -1,
    };

    let leaked_bm: &'static [u8] = Box::leak(blind_m_bytes.into_boxed_slice());
    blind_message.blind_message = leaked_bm;
    blind_message.blind_message_len = modulus_bytes;

    let leaked_sb: &'static [u8] = Box::leak(secret_bytes.into_boxed_slice());
    secret.secret = leaked_sb;
    secret.secret_len = modulus_bytes;

    0
}

// ============================================================================
// BRSABlindMessage
// ============================================================================
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
        self.blind_message_len = modulus_bytes;
    }
    pub fn brsa_blind_message_deinit(&mut self) {
        self.blind_message = &[];
        self.blind_message_len = 0;
    }
}

// ============================================================================
// BRSABlindingSecret
// ============================================================================
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
        self.secret_len = modulus_bytes;
    }
    pub fn brsa_blinding_secret_deinit(&mut self) {
        self.secret = &[];
        self.secret_len = 0;
    }
}

// ============================================================================
// BRSABlindSignature
// ============================================================================
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
        self.blind_sig_len = blind_sig_len;
    }
    pub fn brsa_blind_signature_deinit(&mut self) {
        self.blind_sig = &[];
        self.blind_sig_len = 0;
    }
}

// ============================================================================
// BRSASignature
// ============================================================================
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
        self.sig_len = sig_len;
    }
    pub fn brsa_signature_deinit(&mut self) {
        self.sig = &[];
        self.sig_len = 0;
    }
}

// ============================================================================
// BRSAPublicKey
// ============================================================================
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
    pub fn brsa_publickey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        if der_len > MAX_SERIALIZED_PK_LEN {
            return -1;
        }
        let (n, e) = match decode_rsa_public_key_pkcs1(&der[..der_len]) {
            Some(v) => v,
            None => return -1,
        };
        if !rsa_params_check(n.bits(), &e) {
            return -1;
        }
        pk_set(self as *const _, PublicKeyState { n, e });
        0
    }
    pub fn brsa_publickey_deinit(&mut self) {
        pk_remove(self as *const _);
        self.evp_pkey = None;
        self.mont_ctx = None;
    }
    pub fn brsa_publickey_recover(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let sk_state = match sk_get(sk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        pk_set(
            self as *const _,
            PublicKeyState {
                n: sk_state.inner.n().clone(),
                e: sk_state.inner.e().clone(),
            },
        );
        0
    }
}

// ============================================================================
// BRSASecretKey
// ============================================================================
pub struct BRSASecretKey {
    pub evp_pkey: Option<EVP_PKEY>,
}
impl BRSASecretKey {
    pub fn new() -> Self {
        BRSASecretKey { evp_pkey: None }
    }
    pub fn brsa_keypair_generate(
        &mut self,
        pk: &mut BRSAPublicKey,
        modulus_bits: c_int,
    ) -> i32 {
        if modulus_bits <= 0 {
            return -1;
        }
        let mut rng = rand::thread_rng();
        let priv_key = match RsaPrivateKey::new_with_exp(
            &mut rng,
            modulus_bits as usize,
            &BigUint::from(65537u32),
        ) {
            Ok(k) => k,
            Err(_) => return -1,
        };
        let n = priv_key.n().clone();
        let e = priv_key.e().clone();
        sk_set(
            self as *const _,
            SecretKeyState { inner: priv_key },
        );
        pk_set(pk as *const _, PublicKeyState { n, e });
        0
    }
    pub fn brsa_secretkey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        let priv_key = match decode_rsa_private_key_pkcs1(&der[..der_len]) {
            Some(k) => k,
            None => return -1,
        };
        sk_set(self as *const _, SecretKeyState { inner: priv_key });
        0
    }
    pub fn brsa_secretkey_deinit(&mut self) {
        sk_remove(self as *const _);
        self.evp_pkey = None;
    }
}

// ============================================================================
// BRSASerializedKey
// ============================================================================
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
    pub fn brsa_secretkey_export(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let sk_state = match sk_get(sk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let der = encode_rsa_private_key_pkcs1(&sk_state.inner);
        let len = der.len();
        let leaked: &'static [u8] = Box::leak(der.into_boxed_slice());
        self.bytes = leaked;
        self.bytes_len = len;
        0
    }
    pub fn brsa_publickey_export(
        &mut self,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_state = match pk_get(pk as *const _) {
            Some(s) => s,
            None => return -1,
        };
        let der = encode_rsa_public_key_pkcs1(&pk_state.n, &pk_state.e);
        let len = der.len();
        let leaked: &'static [u8] = Box::leak(der.into_boxed_slice());
        self.bytes = leaked;
        self.bytes_len = len;
        0
    }
    pub fn brsa_serializedkey_deinit(&mut self) {
        self.bytes = &[];
        self.bytes_len = 0;
    }
}

// ============================================================================
// BRSAMessageRandomizer
// ============================================================================
#[derive(Debug)]
pub struct BRSAMessageRandomizer {
    pub noise: [u8; 32],
}
impl BRSAMessageRandomizer {
    pub fn new() -> Self {
        BRSAMessageRandomizer { noise: [0u8; 32] }
    }
}

// ============================================================================
// Constants
// ============================================================================
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

// ============================================================================
// Header helpers (kept as required by interface; mostly unimplemented stubs)
// ============================================================================
pub fn BN_bn2bin_padded(_OUT: &mut [u8], _LEN: c_int, _IN: Option<BIGNUM>) -> bool {
    // Cannot construct BIGNUM (empty enum) — stub.
    false
}
pub fn _rsa_bits(_evp_pkey: Option<EVP_PKEY>) -> i32 {
    0
}
pub fn _rsa_size(_evp_pkey: Option<EVP_PKEY>) -> usize {
    0
}
pub fn _rsa_n(_evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    None
}
pub fn _rsa_e(_evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    None
}
pub fn new_mont_domain(_n: Option<BIGNUM>) -> Option<BN_MONT_CTX> {
    None
}
pub fn _rsa_parameters_check(_evp_pkey: Option<EVP_PKEY>) -> i32 {
    -1
}
pub fn _hash(_evp_md: Option<EVP_MD>, _prefix: &BRSAMessageRandomizer, _msg_hash: &[u8], _msg: &[u8]) -> i32 {
    -1
}
pub fn _blind(
    _blind_message: &BRSABlindMessage,
    _secret: &BRSABlindingSecret,
    _pk: &BRSAPublicKey,
    _bn_ctx: Option<BN_CTX>,
    _padded: &[u8],
) -> i32 {
    -1
}
pub fn _check_cannonical(_sk: &BRSASecretKey, _blind_message: &BRSABlindMessage) -> i32 {
    -1
}
pub fn _finalize(
    _context: &BRSAContext,
    _sig: &BRSASignature,
    _blind_sig: &BRSABlindSignature,
    _secret: &BRSABlindingSecret,
    _msg_randomizer: &BRSAMessageRandomizer,
    _pk: &BRSAPublicKey,
    _bn_ctx: Option<BN_CTX>,
    _msg: &[u8],
) -> i32 {
    -1
}
