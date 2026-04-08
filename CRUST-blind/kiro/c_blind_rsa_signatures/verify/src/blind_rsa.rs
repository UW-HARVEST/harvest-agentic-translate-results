use openssl_sys::*;

use openssl::bn::{BigNum, BigNumContext};
use openssl::hash::{Hasher, MessageDigest};
use openssl::pkey::{Private, Public};
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

static REGISTRY: std::sync::LazyLock<Mutex<HashMap<usize, Box<dyn Any + Send>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn reg_store(addr: usize, val: Box<dyn Any + Send>) {
    REGISTRY.lock().unwrap().insert(addr, val);
}
fn reg_get<T: 'static + Clone>(addr: usize) -> Option<T> {
    REGISTRY.lock().unwrap().get(&addr).and_then(|v| v.downcast_ref::<T>()).cloned()
}
fn reg_remove(addr: usize) {
    REGISTRY.lock().unwrap().remove(&addr);
}

fn leak_vec(v: Vec<u8>) -> &'static [u8] {
    Box::leak(v.into_boxed_slice())
}
fn reclaim_slice(s: &[u8]) {
    if !s.is_empty() {
        unsafe {
            let ptr = s.as_ptr() as *mut u8;
            let len = s.len();
            drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)));
        }
    }
}

// Encode hash function in upper bits of salt_len
const HASH_SHIFT: u32 = 48;
const SALT_MASK: usize = (1usize << HASH_SHIFT) - 1;

fn encode_salt(hash_fn: BRSAHashFunction, salt_len: usize) -> usize {
    let h = match hash_fn {
        BRSAHashFunction::BRSA_SHA256 => 0usize,
        BRSAHashFunction::BRSA_SHA384 => 1usize,
        BRSAHashFunction::BRSA_SHA512 => 2usize,
    };
    (h << HASH_SHIFT) | (salt_len & SALT_MASK)
}
fn decode_hash(encoded: usize) -> MessageDigest {
    match encoded >> HASH_SHIFT {
        0 => MessageDigest::sha256(),
        1 => MessageDigest::sha384(),
        2 => MessageDigest::sha512(),
        _ => MessageDigest::sha384(),
    }
}
fn decode_salt(encoded: usize) -> usize {
    encoded & SALT_MASK
}

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;
#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
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
        let md = match hash_function {
            BRSAHashFunction::BRSA_SHA256 => MessageDigest::sha256(),
            BRSAHashFunction::BRSA_SHA384 => MessageDigest::sha384(),
            BRSAHashFunction::BRSA_SHA512 => MessageDigest::sha512(),
        };
        let actual_salt = if salt_len == BRSA_DEFAULT_SALT_LENGTH { md.size() } else { salt_len };
        self.salt_len = encode_salt(hash_function, actual_salt);
        0
    }

    pub fn md(&self) -> MessageDigest { decode_hash(self.salt_len) }
    pub fn salt(&self) -> usize { decode_salt(self.salt_len) }

    pub fn brsa_blind_message_generate(
        &self,
        blind_message: &mut BRSABlindMessage,
        msg: &mut [u8],
        msg_len: usize,
        secret: &mut BRSABlindingSecret,
        pk: &mut BRSAPublicKey,
    ) -> i32 {
        if rand_bytes(&mut msg[..msg_len]).is_err() { return -1; }
        self.blind_internal(blind_message, secret, &None, pk, &msg[..msg_len])
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
        let mut mr = Some(BRSAMessageRandomizer { noise: [0u8; 32] });
        if rand_bytes(&mut mr.as_mut().unwrap().noise).is_err() { return -1; }
        msg_randomizer.noise = mr.as_ref().unwrap().noise;
        self.blind_internal(blind_message, secret, &mr, pk, &msg[..msg_len])
    }

    fn blind_internal(
        &self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: &Option<BRSAMessageRandomizer>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
    ) -> i32 {
        let pk_addr = pk as *const _ as usize;
        let rsa_pub: Rsa<Public> = match reg_get(pk_addr) {
            Some(r) => r,
            None => return -1,
        };
        if rsa_parameters_check_rsa_pub(&rsa_pub) != 0 { return -1; }
        let modulus_bytes = rsa_pub.size() as usize;
        let md = self.md();

        // Hash message
        let msg_hash = match hash_msg(md, msg_randomizer, msg) {
            Some(h) => h,
            None => return -1,
        };

        // PSS padding
        let padded = match pss_encode(md, &msg_hash, modulus_bytes, self.salt()) {
            Some(p) => p,
            None => return -1,
        };

        // Blind
        blind_internal_bn(blind_message, secret, &rsa_pub, &padded)
    }

    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let sk_addr = sk as *const _ as usize;
        let rsa_priv: Rsa<Private> = match reg_get(sk_addr) {
            Some(r) => r,
            None => return -1,
        };
        if rsa_parameters_check_rsa_priv(&rsa_priv) != 0 { return -1; }
        let modulus_bytes = rsa_priv.size() as usize;
        if blind_message.blind_message_len != modulus_bytes { return -1; }

        // Check canonical
        let n = match rsa_priv.n().to_vec_padded(modulus_bytes as i32) {
            Ok(v) => v, Err(_) => return -1,
        };
        let bm = blind_message.blind_message;
        if bm.len() != modulus_bytes { return -1; }
        // Check bm < n
        let mut found_less = false;
        for i in 0..modulus_bytes {
            if bm[i] < n[i] { found_less = true; break; }
            if bm[i] > n[i] || i + 1 == modulus_bytes { return -1; }
        }
        if !found_less { return -1; }

        // Raw RSA private encrypt (no padding)
        let mut out = vec![0u8; modulus_bytes];
        match rsa_priv.private_encrypt(bm, &mut out, Padding::NONE) {
            Ok(_) => {},
            Err(_) => return -1,
        }
        blind_sig.blind_sig_len = modulus_bytes;
        blind_sig.blind_sig = leak_vec(out);
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
        let pk_addr = pk as *const _ as usize;
        let rsa_pub: Rsa<Public> = match reg_get(pk_addr) {
            Some(r) => r,
            None => return -1,
        };
        if rsa_parameters_check_rsa_pub(&rsa_pub) != 0 { return -1; }
        let modulus_bytes = rsa_pub.size() as usize;
        if blind_sig.blind_sig_len != modulus_bytes || secret_.secret_len != modulus_bytes {
            return -1;
        }

        let mut ctx = match BigNumContext::new() { Ok(c) => c, Err(_) => return -1 };
        let secret_bn = match BigNum::from_slice(secret_.secret) { Ok(b) => b, Err(_) => return -1 };
        let blind_z = match BigNum::from_slice(blind_sig.blind_sig) { Ok(b) => b, Err(_) => return -1 };
        let n = match BigNum::from_slice(&rsa_pub.n().to_vec()) { Ok(b) => b, Err(_) => return -1 };
        let mut z = BigNum::new().unwrap();
        if z.mod_mul(&blind_z, &secret_bn, &n, &mut ctx).is_err() { return -1; }

        let sig_bytes = match z.to_vec_padded(modulus_bytes as i32) {
            Ok(v) => v,
            Err(_) => return -1,
        };
        sig.sig_len = modulus_bytes;
        sig.sig = leak_vec(sig_bytes);

        // Verify
        if self.verify_internal(sig, &rsa_pub, msg_randomizer, &msg[..msg_len]) != 0 {
            sig.brsa_signature_deinit();
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
        let pk_addr = pk as *const _ as usize;
        let rsa_pub: Rsa<Public> = match reg_get(pk_addr) {
            Some(r) => r,
            None => return -1,
        };
        self.verify_internal(sig, &rsa_pub, msg_randomizer, &msg[..msg_len])
    }

    fn verify_internal(
        &self,
        sig: &BRSASignature,
        rsa_pub: &Rsa<Public>,
        msg_randomizer: &Option<BRSAMessageRandomizer>,
        msg: &[u8],
    ) -> i32 {
        let modulus_bytes = rsa_pub.size() as usize;
        if sig.sig_len != modulus_bytes { return -1; }
        let md = self.md();

        let msg_hash = match hash_msg(md, msg_randomizer, msg) {
            Some(h) => h,
            None => return -1,
        };

        // RSA public decrypt (no padding) to get EM
        let mut em = vec![0u8; modulus_bytes];
        if rsa_pub.public_decrypt(sig.sig, &mut em, Padding::NONE).is_err() { return -1; }

        // PSS verify
        if pss_verify(md, &msg_hash, &em, modulus_bytes, self.salt()) { 0 } else { -1 }
    }

    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_addr = pk as *const _ as usize;
        let rsa_pub: Rsa<Public> = match reg_get(pk_addr) {
            Some(r) => r,
            None => return -1,
        };
        let pk_der = match rsa_pub.public_key_to_der_pkcs1() {
            Ok(d) => d,
            Err(_) => return -1,
        };

        let template = rsassa_pss_spki_template();
        let template_len = template.len();
        let container_len = template_len - 4 + pk_der.len();

        let mut spki_bytes = template;
        spki_bytes.extend_from_slice(&pk_der);
        spki_bytes[2] = (container_len >> 8) as u8;
        spki_bytes[3] = (container_len & 0xff) as u8;
        spki_bytes[66] = (self.salt() & 0xff) as u8;
        spki_bytes[69] = ((1 + pk_der.len()) >> 8) as u8;
        spki_bytes[70] = ((1 + pk_der.len()) & 0xff) as u8;

        // Fill in hash algorithm OID
        let hash_oid = hash_algorithm_oid(self.md());
        // offset 21: SEQ, len, OBJ, 9, <9 bytes OID>
        spki_bytes[21] = 0x30; // SEQ
        spki_bytes[22] = 2 + 9; // len
        spki_bytes[23] = 0x06; // OBJ
        spki_bytes[24] = 9;
        spki_bytes[25..34].copy_from_slice(&hash_oid);
        // offset 49: same
        spki_bytes[49] = 0x30;
        spki_bytes[50] = 2 + 9;
        spki_bytes[51] = 0x06;
        spki_bytes[52] = 9;
        spki_bytes[53..62].copy_from_slice(&hash_oid);

        spki.bytes_len = spki_bytes.len();
        spki.bytes = leak_vec(spki_bytes);
        0
    }

    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        let template_len = rsassa_pss_spki_template().len();
        if spki_len > MAX_SERIALIZED_PK_LEN || spki_len <= template_len { return -1; }
        // Check algorithm OID at offset 6..18
        let template = rsassa_pss_spki_template();
        if spki[6..18] != template[6..18] { return -1; }
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
        let mut spki = BRSASerializedKey::new();
        if self.brsa_publickey_export_spki(&mut spki, pk) != 0 { return -1; }

        let hash_result = match openssl::hash::hash(MessageDigest::sha256(), spki.bytes) {
            Ok(h) => h,
            Err(_) => { spki.brsa_serializedkey_deinit(); return -1; }
        };
        spki.brsa_serializedkey_deinit();

        let h = hash_result.to_vec();
        let out_len = id_len.min(h.len());
        // Write to id (it's &[u8] but we need to write — use unsafe)
        unsafe {
            let id_ptr = id.as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(h.as_ptr(), id_ptr, out_len);
            if out_len < id_len {
                std::ptr::write_bytes(id_ptr.add(out_len), 0, id_len - out_len);
            }
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
        self.blind_message_len = modulus_bytes;
        self.blind_message = leak_vec(v);
    }
    pub fn brsa_blind_message_deinit(&mut self) {
        if !self.blind_message.is_empty() { reclaim_slice(self.blind_message); }
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
        self.secret_len = modulus_bytes;
        self.secret = leak_vec(v);
    }
    pub fn brsa_blinding_secret_deinit(&mut self) {
        if !self.secret.is_empty() { reclaim_slice(self.secret); }
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
        self.blind_sig_len = blind_sig_len;
        self.blind_sig = leak_vec(v);
    }
    pub fn brsa_blind_signature_deinit(&mut self) {
        if !self.blind_sig.is_empty() { reclaim_slice(self.blind_sig); }
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
        self.sig_len = sig_len;
        self.sig = leak_vec(v);
    }
    pub fn brsa_signature_deinit(&mut self) {
        if !self.sig.is_empty() { reclaim_slice(self.sig); }
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
    pub fn brsa_publickey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        if der_len > MAX_SERIALIZED_PK_LEN { return -1; }
        let rsa_pub = match Rsa::public_key_from_der_pkcs1(&der[..der_len]) {
            Ok(r) => r,
            Err(_) => return -1,
        };
        if rsa_parameters_check_rsa_pub(&rsa_pub) != 0 { return -1; }
        let addr = self as *const _ as usize;
        reg_store(addr, Box::new(rsa_pub));
        0
    }
    pub fn brsa_publickey_deinit(&mut self) {
        let addr = self as *const _ as usize;
        reg_remove(addr);
    }
    pub fn brsa_publickey_recover(&mut self, sk: &BRSASecretKey) -> i32 {
        let sk_addr = sk as *const _ as usize;
        let rsa_priv: Rsa<Private> = match reg_get(sk_addr) {
            Some(r) => r,
            None => return -1,
        };
        let der = match rsa_priv.public_key_to_der_pkcs1() {
            Ok(d) => d,
            Err(_) => return -1,
        };
        self.brsa_publickey_import(&der, der.len())
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
        let rsa = match Rsa::generate(modulus_bits as u32) {
            Ok(r) => r,
            Err(_) => return -1,
        };
        let addr = self as *const _ as usize;
        reg_store(addr, Box::new(rsa.clone()));
        pk.brsa_publickey_recover(self)
    }
    pub fn brsa_secretkey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        let rsa_priv = match Rsa::private_key_from_der(&der[..der_len]) {
            Ok(r) => r,
            Err(_) => return -1,
        };
        let addr = self as *const _ as usize;
        reg_store(addr, Box::new(rsa_priv));
        0
    }
    pub fn brsa_secretkey_deinit(&mut self) {
        let addr = self as *const _ as usize;
        reg_remove(addr);
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
    pub fn brsa_secretkey_export(&mut self, sk: &BRSASecretKey) -> i32 {
        let sk_addr = sk as *const _ as usize;
        let rsa_priv: Rsa<Private> = match reg_get(sk_addr) {
            Some(r) => r,
            None => return -1,
        };
        let der = match rsa_priv.private_key_to_der() {
            Ok(d) => d,
            Err(_) => return -1,
        };
        self.bytes_len = der.len();
        self.bytes = leak_vec(der);
        0
    }
    pub fn brsa_publickey_export(&mut self, pk: &BRSAPublicKey) -> i32 {
        let pk_addr = pk as *const _ as usize;
        let rsa_pub: Rsa<Public> = match reg_get(pk_addr) {
            Some(r) => r,
            None => return -1,
        };
        let der = match rsa_pub.public_key_to_der_pkcs1() {
            Ok(d) => d,
            Err(_) => return -1,
        };
        self.bytes_len = der.len();
        self.bytes = leak_vec(der);
        0
    }
    pub fn brsa_serializedkey_deinit(&mut self) {
        if !self.bytes.is_empty() { reclaim_slice(self.bytes); }
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

// Constants and standalone functions
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

pub fn BN_bn2bin_padded(OUT: &mut [u8], LEN: c_int, IN: Option<BIGNUM>) -> bool {
    // Standalone stub — actual logic uses openssl::bn::BigNum::to_vec_padded
    let _ = (OUT, LEN, IN);
    false
}
pub fn _rsa_bits(evp_pkey: Option<EVP_PKEY>) -> i32 {
    let _ = evp_pkey; 0
}
pub fn _rsa_size(evp_pkey: Option<EVP_PKEY>) -> usize {
    let _ = evp_pkey; 0
}
pub fn _rsa_n(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    let _ = evp_pkey; None
}
pub fn _rsa_e(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    let _ = evp_pkey; None
}
pub fn new_mont_domain(n: Option<BIGNUM>) -> Option<BN_MONT_CTX> {
    let _ = n; None
}
pub fn _rsa_parameters_check(evp_pkey: Option<EVP_PKEY>) -> i32 {
    let _ = evp_pkey; -1
}
pub fn _hash(evp_md: Option<EVP_MD>, prefix: &BRSAMessageRandomizer, msg_hash: &[u8], msg: &[u8]) -> i32 {
    let _ = (evp_md, prefix, msg_hash, msg); -1
}
pub fn _blind(blind_message: &BRSABlindMessage, secret: &BRSABlindingSecret, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, padded: &[u8]) -> i32 {
    let _ = (blind_message, secret, pk, bn_ctx, padded); -1
}
pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 {
    let _ = (sk, blind_message); -1
}
pub fn _finalize(context: &BRSAContext, sig: &BRSASignature, blind_sig: &BRSABlindSignature,
    secret: &BRSABlindingSecret, msg_randomizer: &BRSAMessageRandomizer, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, msg: &[u8]) -> i32 {
    let _ = (context, sig, blind_sig, secret, msg_randomizer, pk, bn_ctx, msg); -1
}

// Helper functions

fn rsa_parameters_check_rsa_pub(rsa: &Rsa<Public>) -> i32 {
    let bits = rsa.size() as usize * 8;
    if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS { return -1; }
    let e = rsa.e();
    let e3 = BigNum::from_u32(3).unwrap();
    let ef4 = BigNum::from_u32(65537).unwrap();
    if e == e3.as_ref() || e == ef4.as_ref() { 0 } else { -1 }
}

fn rsa_parameters_check_rsa_priv(rsa: &Rsa<Private>) -> i32 {
    let bits = rsa.size() as usize * 8;
    if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS { return -1; }
    let e = rsa.e();
    let e3 = BigNum::from_u32(3).unwrap();
    let ef4 = BigNum::from_u32(65537).unwrap();
    if e == e3.as_ref() || e == ef4.as_ref() { 0 } else { -1 }
}

fn hash_msg(md: MessageDigest, prefix: &Option<BRSAMessageRandomizer>, msg: &[u8]) -> Option<Vec<u8>> {
    let mut hasher = Hasher::new(md).ok()?;
    if prefix.is_some() {
        // C code hashes msg twice when prefix is non-NULL (hashes msg instead of noise)
        hasher.update(msg).ok()?;
    }
    hasher.update(msg).ok()?;
    Some(hasher.finish().ok()?.to_vec())
}

// MGF1 as defined in RFC 8017
fn mgf1(md: MessageDigest, seed: &[u8], len: usize) -> Option<Vec<u8>> {
    let h_len = md.size();
    let mut out = Vec::with_capacity(len);
    let mut counter = 0u32;
    while out.len() < len {
        let mut hasher = Hasher::new(md).ok()?;
        hasher.update(seed).ok()?;
        hasher.update(&counter.to_be_bytes()).ok()?;
        let h = hasher.finish().ok()?;
        out.extend_from_slice(&h[..h_len.min(len - out.len())]);
        counter += 1;
    }
    out.truncate(len);
    Some(out)
}

// EMSA-PSS-ENCODE from RFC 8017 Section 9.1.1
fn pss_encode(md: MessageDigest, m_hash: &[u8], em_bits_bytes: usize, s_len: usize) -> Option<Vec<u8>> {
    let h_len = md.size();
    let em_len = em_bits_bytes;
    let em_bits = em_len * 8 - 1; // For RSA, emBits = modBits - 1

    if em_len < h_len + s_len + 2 { return None; }

    let mut salt = vec![0u8; s_len];
    if s_len > 0 { rand_bytes(&mut salt).ok()?; }

    // M' = (0x)00 00 00 00 00 00 00 00 || mHash || salt
    let mut m_prime = vec![0u8; 8];
    m_prime.extend_from_slice(m_hash);
    m_prime.extend_from_slice(&salt);

    let mut hasher = Hasher::new(md).ok()?;
    hasher.update(&m_prime).ok()?;
    let h = hasher.finish().ok()?.to_vec();

    let ps_len = em_len - s_len - h_len - 2;
    let mut db = vec![0u8; ps_len];
    db.push(0x01);
    db.extend_from_slice(&salt);

    let db_mask = mgf1(md, &h, db.len())?;
    for i in 0..db.len() { db[i] ^= db_mask[i]; }

    // Set leftmost bits to zero
    let top_bits = 8 * em_len - em_bits;
    if top_bits > 0 { db[0] &= 0xff >> top_bits; }

    let mut em = db;
    em.extend_from_slice(&h);
    em.push(0xbc);
    Some(em)
}

// EMSA-PSS-VERIFY from RFC 8017 Section 9.1.2
fn pss_verify(md: MessageDigest, m_hash: &[u8], em: &[u8], em_bytes: usize, s_len: usize) -> bool {
    let h_len = md.size();
    let em_len = em_bytes;
    let em_bits = em_len * 8 - 1;

    if em_len < h_len + s_len + 2 { return false; }
    if em[em_len - 1] != 0xbc { return false; }

    let db_len = em_len - h_len - 1;
    let db = &em[..db_len];
    let h = &em[db_len..db_len + h_len];

    let top_bits = 8 * em_len - em_bits;
    if top_bits > 0 && (db[0] >> (8 - top_bits)) != 0 { return false; }

    let db_mask = match mgf1(md, h, db_len) { Some(m) => m, None => return false };
    let mut db_unmasked = db.to_vec();
    for i in 0..db_len { db_unmasked[i] ^= db_mask[i]; }
    if top_bits > 0 { db_unmasked[0] &= 0xff >> top_bits; }

    let ps_len = em_len - h_len - s_len - 2;
    for i in 0..ps_len {
        if db_unmasked[i] != 0 { return false; }
    }
    if db_unmasked[ps_len] != 0x01 { return false; }

    let salt = &db_unmasked[ps_len + 1..];

    let mut m_prime = vec![0u8; 8];
    m_prime.extend_from_slice(m_hash);
    m_prime.extend_from_slice(salt);

    let mut hasher = match Hasher::new(md) { Ok(h) => h, Err(_) => return false };
    if hasher.update(&m_prime).is_err() { return false; }
    let h_prime = match hasher.finish() { Ok(h) => h, Err(_) => return false };

    h == &h_prime[..h_len]
}

fn blind_internal_bn(
    blind_message: &mut BRSABlindMessage,
    secret_out: &mut BRSABlindingSecret,
    rsa_pub: &Rsa<Public>,
    padded: &[u8],
) -> i32 {
    let mut ctx = match BigNumContext::new() { Ok(c) => c, Err(_) => return -1 };
    let m = match BigNum::from_slice(padded) { Ok(b) => b, Err(_) => return -1 };
    let n = match BigNum::from_slice(&rsa_pub.n().to_vec()) { Ok(b) => b, Err(_) => return -1 };
    let e = match BigNum::from_slice(&rsa_pub.e().to_vec()) { Ok(b) => b, Err(_) => return -1 };

    // Check gcd(m, n) == 1
    let one = BigNum::from_u32(1).unwrap();
    let mut gcd = BigNum::new().unwrap();
    if gcd.gcd(&m, &n, &mut ctx).is_err() { return -1; }
    if gcd != *one { return -1; }

    // Generate random secret_inv, compute secret = secret_inv^(-1) mod n
    let mut secret_inv = BigNum::new().unwrap();
    let mut secret = BigNum::new().unwrap();
    loop {
        if n.rand_range(&mut secret_inv).is_err() { return -1; }
        if secret_inv == *one { continue; }
        match secret.mod_inverse(&secret_inv, &n, &mut ctx) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }

    // x = secret_inv^e mod n
    let mut x = BigNum::new().unwrap();
    if x.mod_exp(&secret_inv, &e, &n, &mut ctx).is_err() { return -1; }

    // blind_m = m * x mod n
    let mut blind_m = BigNum::new().unwrap();
    if blind_m.mod_mul(&m, &x, &n, &mut ctx).is_err() { return -1; }

    let modulus_bytes = rsa_pub.size() as usize;
    let bm_bytes = match blind_m.to_vec_padded(modulus_bytes as i32) {
        Ok(v) => v, Err(_) => return -1,
    };
    let s_bytes = match secret.to_vec_padded(modulus_bytes as i32) {
        Ok(v) => v, Err(_) => return -1,
    };

    blind_message.blind_message_len = modulus_bytes;
    blind_message.blind_message = leak_vec(bm_bytes);
    secret_out.secret_len = modulus_bytes;
    secret_out.secret = leak_vec(s_bytes);
    0
}

fn hash_algorithm_oid(md: MessageDigest) -> [u8; 9] {
    let size = md.size();
    match size {
        32 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01], // SHA-256
        48 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02], // SHA-384
        64 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03], // SHA-512
        _ =>  [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02], // default SHA-384
    }
}

fn rsassa_pss_spki_template() -> Vec<u8> {
    vec![
        0x30, 0x80 | 2, 0, 0, // SEQ, container length (offset 2,3)
        0x30, 61, // Algorithm sequence
            0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a, // RSASSA-PSS OID
            0x30, 48, // RSASSA-PSS parameters
                0xa0, 2 + 2 + 9,
                0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Hash function (offset 21)

                0xa1, 2 + 24,
                0x30, 24, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08, // MGF1
                    0x30, 2 + 9, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, // MGF1 hash (offset 49)

                0xa2, 2 + 1, 0x02, 1, 0, // Salt length (offset 66)
        0x03, 0x80 | 2, 0, 0, // BIT STRING (offset 69,70)
            0, // No partial bytes
    ]
}
