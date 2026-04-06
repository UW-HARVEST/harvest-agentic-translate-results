use openssl_sys::*;
use std::cell::RefCell;
use std::collections::HashMap;

use num_bigint_dig::BigUint;
use num_bigint_dig::RandBigInt;
use num_bigint_dig::RandPrime;
use num_bigint_dig::ModInverse;
use num_traits::{One, Zero};
use num_integer::Integer;
use sha2::{Sha256, Sha384, Sha512, Digest};
use rand::rngs::OsRng;
use rand::RngCore;

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

// Internal key storage
struct PrivKeyData {
    n: BigUint,
    e: BigUint,
    d: BigUint,
    primes: Vec<BigUint>,
}

struct PubKeyData {
    n: BigUint,
    e: BigUint,
}

thread_local! {
    static SK_STORE: RefCell<HashMap<usize, Box<PrivKeyData>>> = RefCell::new(HashMap::new());
    static PK_STORE: RefCell<HashMap<usize, Box<PubKeyData>>> = RefCell::new(HashMap::new());
}

fn sk_id(sk: &BRSASecretKey) -> usize {
    sk as *const BRSASecretKey as usize
}

fn pk_id(pk: &BRSAPublicKey) -> usize {
    pk as *const BRSAPublicKey as usize
}

fn leak_vec(v: Vec<u8>) -> &'static [u8] {
    let len = v.len();
    let ptr = Box::into_raw(v.into_boxed_slice()) as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn unleak_slice(s: &[u8]) -> Vec<u8> {
    if s.is_empty() {
        return Vec::new();
    }
    let len = s.len();
    let ptr = s.as_ptr() as *mut u8;
    unsafe { Vec::from_raw_parts(ptr, len, len) }
}

// Hash function helpers
fn hash_id(ctx: &BRSAContext) -> u8 {
    ((ctx.salt_len >> 56) & 0xFF) as u8
}

fn real_salt_len(ctx: &BRSAContext) -> usize {
    ctx.salt_len & 0x00FFFFFFFFFFFFFF
}

fn encode_ctx(hash_id: u8, salt_len: usize) -> usize {
    ((hash_id as usize) << 56) | (salt_len & 0x00FFFFFFFFFFFFFF)
}

fn hash_output_size(hid: u8) -> usize {
    match hid {
        0 => 32,  // SHA256
        1 => 48,  // SHA384
        2 => 64,  // SHA512
        _ => 48,
    }
}

fn compute_hash(hid: u8, prefix: Option<&[u8; 32]>, msg: &[u8]) -> Vec<u8> {
    match hid {
        0 => {
            let mut h = Sha256::new();
            if prefix.is_some() { h.update(msg); }
            h.update(msg);
            h.finalize().to_vec()
        }
        1 => {
            let mut h = Sha384::new();
            if prefix.is_some() { h.update(msg); }
            h.update(msg);
            h.finalize().to_vec()
        }
        2 => {
            let mut h = Sha512::new();
            if prefix.is_some() { h.update(msg); }
            h.update(msg);
            h.finalize().to_vec()
        }
        _ => {
            let mut h = Sha384::new();
            if prefix.is_some() { h.update(msg); }
            h.update(msg);
            h.finalize().to_vec()
        }
    }
}

// MGF1
fn mgf1(seed: &[u8], len: usize, hid: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let mut counter: u32 = 0;
    while out.len() < len {
        let c_bytes = counter.to_be_bytes();
        let hash = match hid {
            0 => { let mut h = Sha256::new(); h.update(seed); h.update(&c_bytes); h.finalize().to_vec() }
            1 => { let mut h = Sha384::new(); h.update(seed); h.update(&c_bytes); h.finalize().to_vec() }
            2 => { let mut h = Sha512::new(); h.update(seed); h.update(&c_bytes); h.finalize().to_vec() }
            _ => { let mut h = Sha384::new(); h.update(seed); h.update(&c_bytes); h.finalize().to_vec() }
        };
        out.extend_from_slice(&hash);
        counter += 1;
    }
    out.truncate(len);
    out
}

// PSS encode
fn emsa_pss_encode(m_hash: &[u8], em_bits: usize, salt: &[u8], hid: u8) -> Option<Vec<u8>> {
    let h_len = hash_output_size(hid);
    let s_len = salt.len();
    let em_len = (em_bits + 7) / 8;

    if m_hash.len() != h_len { return None; }
    if em_len < h_len + s_len + 2 { return None; }

    let mut em = vec![0u8; em_len];

    // H = Hash(0x00..00 || m_hash || salt)
    let prefix = [0u8; 8];
    let h = match hid {
        0 => { let mut hasher = Sha256::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        1 => { let mut hasher = Sha384::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        2 => { let mut hasher = Sha512::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        _ => { let mut hasher = Sha384::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
    };

    // DB = PS || 0x01 || salt
    let db_len = em_len - h_len - 1;
    let mut db = vec![0u8; db_len];
    db[db_len - s_len - 1] = 0x01;
    db[db_len - s_len..].copy_from_slice(salt);

    // dbMask = MGF(H, db_len)
    let mask = mgf1(&h, db_len, hid);
    for i in 0..db_len {
        db[i] ^= mask[i];
    }

    // Clear top bits
    db[0] &= 0xFF >> (8 * em_len - em_bits);

    // EM = maskedDB || H || 0xbc
    em[..db_len].copy_from_slice(&db);
    em[db_len..db_len + h_len].copy_from_slice(&h);
    em[em_len - 1] = 0xBC;

    Some(em)
}

// PSS verify
fn emsa_pss_verify(m_hash: &[u8], em: &[u8], key_bits: usize, s_len: usize, hid: u8) -> bool {
    let h_len = hash_output_size(hid);
    let em_bits = key_bits - 1;
    let em_len = (em_bits + 7) / 8;
    let key_len = em.len();

    if m_hash.len() != h_len { return false; }
    if em_len < h_len + s_len + 2 { return false; }

    let em = &em[key_len - em_len..];

    if em[em_len - 1] != 0xBC { return false; }

    let db_len = em_len - h_len - 1;
    let mut db = em[..db_len].to_vec();
    let h = &em[db_len..db_len + h_len];

    // Check top bits
    if db[0] & (0xFF_u8.checked_shl(8 - (8 * em_len - em_bits) as u32).unwrap_or(0)) != 0 {
        return false;
    }

    // dbMask = MGF(H, db_len)
    let mask = mgf1(h, db_len, hid);
    for i in 0..db_len {
        db[i] ^= mask[i];
    }
    db[0] &= 0xFF >> (8 * em_len - em_bits);

    // Check PS
    for i in 0..em_len - h_len - s_len - 2 {
        if db[i] != 0 { return false; }
    }
    if db[em_len - h_len - s_len - 2] != 0x01 { return false; }

    let salt = &db[db_len - s_len..];

    // H' = Hash(0x00..00 || m_hash || salt)
    let prefix = [0u8; 8];
    let h_prime = match hid {
        0 => { let mut hasher = Sha256::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        1 => { let mut hasher = Sha384::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        2 => { let mut hasher = Sha512::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
        _ => { let mut hasher = Sha384::new(); hasher.update(&prefix); hasher.update(m_hash); hasher.update(salt); hasher.finalize().to_vec() }
    };

    h == h_prime.as_slice()
}

// RSA key generation using num-bigint-dig
fn generate_rsa_key(bits: usize) -> Option<PrivKeyData> {
    let mut rng = OsRng;
    let e = BigUint::from(65537u64);
    
    for _ in 0..100 {
        let p = rng.gen_prime(bits / 2);
        let q = rng.gen_prime(bits / 2);
        if p == q { continue; }
        
        let n = &p * &q;
        if n.bits() != bits { continue; }
        
        let p1 = &p - BigUint::one();
        let q1 = &q - BigUint::one();
        let totient = p1.lcm(&q1);
        
        if let Some(d) = e.clone().mod_inverse(&totient) {
            if let Some(d) = d.to_biguint() {
                return Some(PrivKeyData {
                    n,
                    e: e.clone(),
                    d,
                    primes: vec![p, q],
                });
            }
        }
    }
    None
}

fn bn_to_padded(bn: &BigUint, len: usize) -> Vec<u8> {
    let bytes = bn.to_bytes_be();
    if bytes.len() >= len {
        bytes[bytes.len() - len..].to_vec()
    } else {
        let mut padded = vec![0u8; len - bytes.len()];
        padded.extend_from_slice(&bytes);
        padded
    }
}

fn modulus_bytes(n: &BigUint) -> usize {
    (n.bits() + 7) / 8
}

fn modulus_bits(n: &BigUint) -> usize {
    n.bits()
}

// DER encoding helpers for PKCS#1
fn encode_der_uint(val: &BigUint) -> Vec<u8> {
    let bytes = val.to_bytes_be();
    let mut content = bytes;
    // Add leading zero if high bit set
    if !content.is_empty() && content[0] & 0x80 != 0 {
        let mut padded = vec![0u8];
        padded.extend_from_slice(&content);
        content = padded;
    }
    let mut out = vec![0x02]; // INTEGER tag
    out.extend_from_slice(&encode_der_length(content.len()));
    out.extend_from_slice(&content);
    out
}

fn encode_der_length(len: usize) -> Vec<u8> {
    if len < 128 {
        vec![len as u8]
    } else if len < 256 {
        vec![0x81, len as u8]
    } else if len < 65536 {
        vec![0x82, (len >> 8) as u8, (len & 0xFF) as u8]
    } else {
        vec![0x83, (len >> 16) as u8, ((len >> 8) & 0xFF) as u8, (len & 0xFF) as u8]
    }
}

fn encode_der_sequence(contents: &[u8]) -> Vec<u8> {
    let mut out = vec![0x30]; // SEQUENCE tag
    out.extend_from_slice(&encode_der_length(contents.len()));
    out.extend_from_slice(contents);
    out
}

// PKCS#1 RSAPrivateKey DER encoding
fn encode_private_key_der(key: &PrivKeyData) -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(&encode_der_uint(&BigUint::zero())); // version
    content.extend_from_slice(&encode_der_uint(&key.n));
    content.extend_from_slice(&encode_der_uint(&key.e));
    content.extend_from_slice(&encode_der_uint(&key.d));
    content.extend_from_slice(&encode_der_uint(&key.primes[0]));
    content.extend_from_slice(&encode_der_uint(&key.primes[1]));
    // dp = d mod (p-1)
    let dp = &key.d % (&key.primes[0] - BigUint::one());
    content.extend_from_slice(&encode_der_uint(&dp));
    // dq = d mod (q-1)
    let dq = &key.d % (&key.primes[1] - BigUint::one());
    content.extend_from_slice(&encode_der_uint(&dq));
    // qinv = q^-1 mod p
    if let Some(qinv) = key.primes[1].clone().mod_inverse(&key.primes[0]) {
        if let Some(qinv) = qinv.to_biguint() {
            content.extend_from_slice(&encode_der_uint(&qinv));
        }
    }
    encode_der_sequence(&content)
}

// PKCS#1 RSAPublicKey DER encoding
fn encode_public_key_der(n: &BigUint, e: &BigUint) -> Vec<u8> {
    let mut content = Vec::new();
    content.extend_from_slice(&encode_der_uint(n));
    content.extend_from_slice(&encode_der_uint(e));
    encode_der_sequence(&content)
}

// DER decoding helpers
fn decode_der_length(data: &[u8], pos: &mut usize) -> Option<usize> {
    if *pos >= data.len() { return None; }
    let b = data[*pos];
    *pos += 1;
    if b < 128 {
        Some(b as usize)
    } else {
        let num_bytes = (b & 0x7F) as usize;
        if *pos + num_bytes > data.len() { return None; }
        let mut len = 0usize;
        for i in 0..num_bytes {
            len = (len << 8) | data[*pos + i] as usize;
        }
        *pos += num_bytes;
        Some(len)
    }
}

fn decode_der_uint(data: &[u8], pos: &mut usize) -> Option<BigUint> {
    if *pos >= data.len() || data[*pos] != 0x02 { return None; }
    *pos += 1;
    let len = decode_der_length(data, pos)?;
    if *pos + len > data.len() { return None; }
    let bytes = &data[*pos..*pos + len];
    *pos += len;
    // Skip leading zeros
    let bytes = if !bytes.is_empty() && bytes[0] == 0 { &bytes[1..] } else { bytes };
    if bytes.is_empty() {
        Some(BigUint::zero())
    } else {
        Some(BigUint::from_bytes_be(bytes))
    }
}

fn decode_private_key_der(der: &[u8]) -> Option<PrivKeyData> {
    let mut pos = 0;
    if pos >= der.len() || der[pos] != 0x30 { return None; }
    pos += 1;
    let _seq_len = decode_der_length(der, &mut pos)?;
    let _version = decode_der_uint(der, &mut pos)?;
    let n = decode_der_uint(der, &mut pos)?;
    let e = decode_der_uint(der, &mut pos)?;
    let d = decode_der_uint(der, &mut pos)?;
    let p = decode_der_uint(der, &mut pos)?;
    let q = decode_der_uint(der, &mut pos)?;
    // dp, dq, qinv are optional for our purposes
    Some(PrivKeyData { n, e, d, primes: vec![p, q] })
}

fn decode_public_key_der(der: &[u8]) -> Option<PubKeyData> {
    let mut pos = 0;
    if pos >= der.len() || der[pos] != 0x30 { return None; }
    pos += 1;
    let _seq_len = decode_der_length(der, &mut pos)?;
    let n = decode_der_uint(der, &mut pos)?;
    let e = decode_der_uint(der, &mut pos)?;
    Some(PubKeyData { n, e })
}

fn rsa_parameters_check(n: &BigUint, e: &BigUint) -> bool {
    let bits = n.bits();
    if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS { return false; }
    *e == BigUint::from(3u64) || *e == BigUint::from(65537u64)
}


pub struct BRSAContext {
    pub evp_md: Option<EVP_MD>,
    pub salt_len: usize,
}
impl BRSAContext {
    pub fn new() -> Self {
        BRSAContext { evp_md: None, salt_len: 0 }
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
        let hid: u8 = match hash_function {
            BRSAHashFunction::BRSA_SHA256 => 0,
            BRSAHashFunction::BRSA_SHA384 => 1,
            BRSAHashFunction::BRSA_SHA512 => 2,
        };
        let actual_salt = if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            hash_output_size(hid)
        } else {
            salt_len
        };
        self.salt_len = encode_ctx(hid, actual_salt);
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
        // Fill msg with random bytes
        let msg_slice = unsafe { std::slice::from_raw_parts_mut(msg.as_ptr() as *mut u8, msg_len) };
        OsRng.fill_bytes(msg_slice);
        // Use a temporary context that consumes self by value workaround
        self.brsa_blind_internal(blind_message, secret, &mut None, pk, msg, msg_len)
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
        OsRng.fill_bytes(&mut msg_randomizer.noise);
        self.brsa_blind_internal(blind_message, secret, &mut Some(msg_randomizer.noise), pk, msg, msg_len)
    }
    fn brsa_blind_internal(
        &self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: &mut Option<[u8; 32]>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
        msg_len: usize,
    ) -> i32 {
        let hid = hash_id(self);
        let salt_len = real_salt_len(self);
        
        let pk_data = PK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&pk_id(pk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        let pk_data = match pk_data {
            Some(d) => d,
            None => return -1,
        };
        
        if !rsa_parameters_check(&pk_data.n, &pk_data.e) { return -1; }
        
        let mod_bytes = modulus_bytes(&pk_data.n);
        
        // Hash message
        let prefix = msg_randomizer.as_ref().map(|n| n);
        let msg_hash = compute_hash(hid, prefix, &msg[..msg_len]);
        
        // PSS encode
        let mut salt_bytes = vec![0u8; salt_len];
        OsRng.fill_bytes(&mut salt_bytes);
        
        let em_bits = pk_data.n.bits() - 1;
        let padded = match emsa_pss_encode(&msg_hash, em_bits, &salt_bytes, hid) {
            Some(p) => p,
            None => return -1,
        };
        
        // Convert to BigUint
        let m = BigUint::from_bytes_be(&padded);
        
        // Check gcd(m, n) == 1
        if m.gcd(&pk_data.n) != BigUint::one() { return -1; }
        
        // Generate blinding factor
        let mut rng = OsRng;
        let (secret_val, blind_m) = loop {
            let r_inv = rng.gen_biguint_below(&pk_data.n);
            if r_inv.is_zero() || r_inv.is_one() { continue; }
            if let Some(r) = r_inv.clone().mod_inverse(&pk_data.n) {
                if let Some(r) = r.to_biguint() {
                    // x = r_inv^e mod n
                    let x = r_inv.modpow(&pk_data.e, &pk_data.n);
                    // blind_m = m * x mod n
                    let blind_m = (&m * &x) % &pk_data.n;
                    break (r, blind_m);
                }
            }
        };
        
        // Serialize
        let bm_bytes = bn_to_padded(&blind_m, mod_bytes);
        let sec_bytes = bn_to_padded(&secret_val, mod_bytes);
        
        blind_message.blind_message = leak_vec(bm_bytes);
        blind_message.blind_message_len = mod_bytes;
        secret.secret = leak_vec(sec_bytes);
        secret.secret_len = mod_bytes;
        
        0
    }
    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let sk_data = SK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&sk_id(sk)).map(|d| PrivKeyData {
                n: d.n.clone(), e: d.e.clone(), d: d.d.clone(), primes: d.primes.clone(),
            })
        });
        let sk_data = match sk_data {
            Some(d) => d,
            None => return -1,
        };
        
        if !rsa_parameters_check(&sk_data.n, &sk_data.e) { return -1; }
        
        let mod_bytes = modulus_bytes(&sk_data.n);
        
        // Check canonical
        if blind_message.blind_message_len != mod_bytes { return -1; }
        let bm = BigUint::from_bytes_be(blind_message.blind_message);
        if bm >= sk_data.n { return -1; }
        
        // Raw RSA: blind_sig = bm^d mod n
        let result = bm.modpow(&sk_data.d, &sk_data.n);
        
        let sig_bytes = bn_to_padded(&result, mod_bytes);
        blind_sig.blind_sig = leak_vec(sig_bytes);
        blind_sig.blind_sig_len = mod_bytes;
        
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
        let pk_data = PK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&pk_id(pk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        let pk_data = match pk_data {
            Some(d) => d,
            None => return -1,
        };
        
        if !rsa_parameters_check(&pk_data.n, &pk_data.e) { return -1; }
        
        let mod_bytes = modulus_bytes(&pk_data.n);
        if blind_sig.blind_sig_len != mod_bytes || secret_.secret_len != mod_bytes { return -1; }
        
        let blind_z = BigUint::from_bytes_be(blind_sig.blind_sig);
        let secret_val = BigUint::from_bytes_be(secret_.secret);
        
        // z = blind_z * secret mod n
        let z = (&blind_z * &secret_val) % &pk_data.n;
        
        let sig_bytes = bn_to_padded(&z, mod_bytes);
        
        // Verify before returning
        let hid = hash_id(self);
        let salt_len = real_salt_len(self);
        
        let prefix = msg_randomizer.as_ref().map(|r| &r.noise);
        let msg_hash = compute_hash(hid, prefix, &msg[..msg_len]);
        
        // em = z^e mod n
        let em_val = z.modpow(&pk_data.e, &pk_data.n);
        let em = bn_to_padded(&em_val, mod_bytes);
        
        if !emsa_pss_verify(&msg_hash, &em, pk_data.n.bits(), salt_len, hid) {
            return -1;
        }
        
        sig.sig = leak_vec(sig_bytes);
        sig.sig_len = mod_bytes;
        
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
        let pk_data = PK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&pk_id(pk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        let pk_data = match pk_data {
            Some(d) => d,
            None => return -1,
        };
        
        let mod_bytes = modulus_bytes(&pk_data.n);
        if sig.sig_len != mod_bytes { return -1; }
        
        let hid = hash_id(self);
        let salt_len = real_salt_len(self);
        
        let prefix = msg_randomizer.as_ref().map(|r| &r.noise);
        let msg_hash = compute_hash(hid, prefix, &msg[..msg_len]);
        
        let s = BigUint::from_bytes_be(sig.sig);
        let em_val = s.modpow(&pk_data.e, &pk_data.n);
        let em = bn_to_padded(&em_val, mod_bytes);
        
        if emsa_pss_verify(&msg_hash, &em, pk_data.n.bits(), salt_len, hid) {
            0
        } else {
            -1
        }
    }
    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_data = PK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&pk_id(pk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        let pk_data = match pk_data {
            Some(d) => d,
            None => return -1,
        };
        
        let pk_der = encode_public_key_der(&pk_data.n, &pk_data.e);
        let hid = hash_id(self);
        let salt_len = real_salt_len(self);
        
        // Build SPKI with RSASSA-PSS algorithm
        let hash_oid: &[u8] = match hid {
            0 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01], // SHA-256
            1 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02], // SHA-384
            2 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03], // SHA-512
            _ => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02], // SHA-384
        };
        
        // Use the template from C code
        let mut template: Vec<u8> = vec![
            0x30, 0x80 | 2, 0, 0, // container - offset 2,3
            0x30, 61, // Algorithm sequence
                0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a, // RSASSA-PSS OID
                0x30, 48, // params
                    0xa0, 2 + 2 + 9,
                    0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // hash - offset 21
                    
                    0xa1, 2 + 24,
                    0x30, 24, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08, // MGF1
                        0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // MGF1 hash - offset 49
                    
                    0xa2, 2 + 1, 0x02, 1, 0, // salt length - offset 66
            0x03, 0x80 | 2, 0, 0, // bit string - offset 69
                0 // no partial bytes
        ];
        
        // Fill in hash OIDs
        template[23..23+9].copy_from_slice(hash_oid);
        template[51..51+9].copy_from_slice(hash_oid);
        
        // Salt length
        template[66] = salt_len as u8;
        
        // Container length
        let container_len = template.len() - 4 + pk_der.len();
        template[2] = (container_len >> 8) as u8;
        template[3] = (container_len & 0xff) as u8;
        
        // Bit string length
        let bit_string_content_len = 1 + pk_der.len();
        template[69] = (bit_string_content_len >> 8) as u8;
        template[70] = (bit_string_content_len & 0xff) as u8;
        
        let mut result = template;
        result.extend_from_slice(&pk_der);
        
        let len = result.len();
        spki.bytes = leak_vec(result);
        spki.bytes_len = len;
        
        0
    }
    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        let template_len = 72; // size of template
        if spki_len > MAX_SERIALIZED_PK_LEN || spki_len <= template_len { return -1; }
        // Check RSASSA-PSS OID
        let expected = &[0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a];
        if spki.len() < 18 || &spki[6..17] != &expected[..11] { return -1; }
        let alg_len = spki[5] as usize;
        if spki_len <= alg_len + 11 { return -1; }
        pk.brsa_publickey_import(&spki[alg_len + 11..spki_len], spki_len - alg_len - 11)
    }
    pub fn brsa_publickey_id(
        &self,
        id: &[u8],
        id_len: usize,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let mut spki = BRSASerializedKey { bytes: &[], bytes_len: 0 };
        if self.brsa_publickey_export_spki(&mut spki, pk) != 0 { return -1; }
        
        let hash = Sha256::digest(spki.bytes);
        spki.brsa_serializedkey_deinit();
        
        let id_slice = unsafe { std::slice::from_raw_parts_mut(id.as_ptr() as *mut u8, id_len) };
        let copy_len = id_len.min(32);
        id_slice[..copy_len].copy_from_slice(&hash[..copy_len]);
        if id_len > 32 {
            id_slice[32..].fill(0);
        }
        
        0
    }
}

pub struct BRSABlindMessage<'a> {
    pub blind_message: &'a [u8],
    pub blind_message_len: usize,
}
impl BRSABlindMessage<'_> {
    pub fn new() -> Self {
        BRSABlindMessage { blind_message: &[], blind_message_len: 0 }
    }
    pub fn brsa_blind_message_init(&mut self, modulus_bytes: usize) {
        let v = vec![0u8; modulus_bytes];
        self.blind_message = leak_vec(v);
        self.blind_message_len = modulus_bytes;
    }
    pub fn brsa_blind_message_deinit(&mut self) {
        if !self.blind_message.is_empty() {
            let _ = unleak_slice(self.blind_message);
        }
        self.blind_message = &[];
        self.blind_message_len = 0;
    }
}

#[derive(Debug)]
pub struct BRSABlindingSecret<'a> {
    pub secret: &'a [u8],
    pub secret_len: usize,
}
impl BRSABlindingSecret<'_> {
    pub fn new() -> Self {
        BRSABlindingSecret { secret: &[], secret_len: 0 }
    }
    pub fn brsa_blinding_secrete_init(&mut self, modulus_bytes: usize) {
        let v = vec![0u8; modulus_bytes];
        self.secret = leak_vec(v);
        self.secret_len = modulus_bytes;
    }
    pub fn brsa_blinding_secret_deinit(&mut self) {
        if !self.secret.is_empty() {
            let _ = unleak_slice(self.secret);
        }
        self.secret = &[];
        self.secret_len = 0;
    }
}

#[derive(Debug)]
pub struct BRSABlindSignature<'a> {
    pub blind_sig: &'a [u8],
    pub blind_sig_len: usize,
}
impl BRSABlindSignature<'_> {
    pub fn new() -> Self {
        BRSABlindSignature { blind_sig: &[], blind_sig_len: 0 }
    }
    pub fn brsa_blind_signature_init(&mut self, blind_sig_len: usize) {
        let v = vec![0u8; blind_sig_len];
        self.blind_sig = leak_vec(v);
        self.blind_sig_len = blind_sig_len;
    }
    pub fn brsa_blind_signature_deinit(&mut self) {
        if !self.blind_sig.is_empty() {
            let _ = unleak_slice(self.blind_sig);
        }
        self.blind_sig = &[];
        self.blind_sig_len = 0;
    }
}

#[derive(Debug)]
pub struct BRSASignature<'a> {
    pub sig: &'a [u8],
    pub sig_len: usize,
}
impl BRSASignature<'_> {
    pub fn new() -> Self {
        BRSASignature { sig: &[], sig_len: 0 }
    }
    pub fn brsa_signature_init(&mut self, sig_len: usize) {
        let v = vec![0u8; sig_len];
        self.sig = leak_vec(v);
        self.sig_len = sig_len;
    }
    pub fn brsa_signature_deinit(&mut self) {
        if !self.sig.is_empty() {
            let _ = unleak_slice(self.sig);
        }
        self.sig = &[];
        self.sig_len = 0;
    }
}

pub struct BRSAPublicKey {
    pub evp_pkey: Option<EVP_PKEY>,
    pub mont_ctx: Option<BN_MONT_CTX>,
}
impl BRSAPublicKey {
    pub fn new() -> Self {
        BRSAPublicKey { evp_pkey: None, mont_ctx: None }
    }
    pub fn brsa_publickey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        if der_len > MAX_SERIALIZED_PK_LEN { return -1; }
        let pk_data = match decode_public_key_der(&der[..der_len]) {
            Some(d) => d,
            None => return -1,
        };
        if !rsa_parameters_check(&pk_data.n, &pk_data.e) { return -1; }
        PK_STORE.with(|store| {
            store.borrow_mut().insert(pk_id(self), Box::new(pk_data));
        });
        0
    }
    pub fn brsa_publickey_deinit(&mut self) {
        PK_STORE.with(|store| {
            store.borrow_mut().remove(&pk_id(self));
        });
    }
    pub fn brsa_publickey_recover(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let sk_data = SK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&sk_id(sk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        match sk_data {
            Some(pk_data) => {
                PK_STORE.with(|store| {
                    store.borrow_mut().insert(pk_id(self), Box::new(pk_data));
                });
                0
            }
            None => -1,
        }
    }
}

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
        let key_data = match generate_rsa_key(modulus_bits as usize) {
            Some(d) => d,
            None => return -1,
        };
        let pub_data = PubKeyData { n: key_data.n.clone(), e: key_data.e.clone() };
        SK_STORE.with(|store| {
            store.borrow_mut().insert(sk_id(self), Box::new(key_data));
        });
        PK_STORE.with(|store| {
            store.borrow_mut().insert(pk_id(pk), Box::new(pub_data));
        });
        0
    }
    pub fn brsa_secretkey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        let key_data = match decode_private_key_der(&der[..der_len]) {
            Some(d) => d,
            None => return -1,
        };
        SK_STORE.with(|store| {
            store.borrow_mut().insert(sk_id(self), Box::new(key_data));
        });
        0
    }
    pub fn brsa_secretkey_deinit(&mut self) {
        SK_STORE.with(|store| {
            store.borrow_mut().remove(&sk_id(self));
        });
    }
}

#[derive(Debug)]
pub struct BRSASerializedKey<'a> {
    pub bytes: &'a [u8],
    pub bytes_len: usize,
}
impl BRSASerializedKey<'_> {
    pub fn new() -> Self {
        BRSASerializedKey { bytes: &[], bytes_len: 0 }
    }
    pub fn brsa_secretkey_export(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let sk_data = SK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&sk_id(sk)).map(|d| PrivKeyData {
                n: d.n.clone(), e: d.e.clone(), d: d.d.clone(), primes: d.primes.clone(),
            })
        });
        match sk_data {
            Some(key) => {
                let der = encode_private_key_der(&key);
                let len = der.len();
                self.bytes = leak_vec(der);
                self.bytes_len = len;
                0
            }
            None => -1,
        }
    }
    pub fn brsa_publickey_export(
        &mut self,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_data = PK_STORE.with(|store| {
            let store = store.borrow();
            store.get(&pk_id(pk)).map(|d| PubKeyData { n: d.n.clone(), e: d.e.clone() })
        });
        match pk_data {
            Some(key) => {
                let der = encode_public_key_der(&key.n, &key.e);
                let len = der.len();
                self.bytes = leak_vec(der);
                self.bytes_len = len;
                0
            }
            None => -1,
        }
    }
    pub fn brsa_serializedkey_deinit(&mut self) {
        if !self.bytes.is_empty() {
            let _ = unleak_slice(self.bytes);
        }
        self.bytes = &[];
        self.bytes_len = 0;
    }
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

// Constants
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

// Standalone functions (kept for interface compatibility)
pub fn BN_bn2bin_padded(OUT: &mut [u8], LEN: c_int, IN: Option<BIGNUM>) -> bool {
    false // Not used in pure Rust implementation
}
pub fn _rsa_bits(evp_pkey: Option<EVP_PKEY>) -> i32 {
    0
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
    -1
}
pub fn _hash(evp_md: Option<EVP_MD>, prefix: &BRSAMessageRandomizer, msg_hash: &[u8], msg: &[u8]) -> i32 {
    0
}
pub fn _blind(blind_message: &BRSABlindMessage, secret: &BRSABlindingSecret, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, padded: &[u8]) -> i32 {
    0
}
pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 {
    0
}
pub fn _finalize(context: &BRSAContext, sig: &BRSASignature, blind_sig: &BRSABlindSignature,
    secret: &BRSABlindingSecret, msg_randomizer: &BRSAMessageRandomizer, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, msg: &[u8]) -> i32 {
    0
}
