#![allow(non_snake_case, non_camel_case_types, unused_variables)]

use openssl_sys::*;
use openssl::bn::{BigNum, BigNumContext};
use openssl::hash::{Hasher, MessageDigest};
use openssl::pkey::{Private, Public};
use openssl::rsa::{Padding, Rsa};
use std::collections::HashMap;
use std::sync::Mutex;

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

// Global stores for key data (keyed by struct address)
static PK_STORE: Mutex<Option<HashMap<usize, Vec<u8>>>> = Mutex::new(None);
static SK_STORE: Mutex<Option<HashMap<usize, Vec<u8>>>> = Mutex::new(None);
static CTX_HASH_STORE: Mutex<Option<HashMap<usize, BRSAHashFunction>>> = Mutex::new(None);

fn store_set(store: &Mutex<Option<HashMap<usize, Vec<u8>>>>, addr: usize, data: Vec<u8>) {
    store.lock().unwrap().get_or_insert_with(HashMap::new).insert(addr, data);
}
fn store_get(store: &Mutex<Option<HashMap<usize, Vec<u8>>>>, addr: usize) -> Option<Vec<u8>> {
    store.lock().unwrap().as_ref().and_then(|m| m.get(&addr).cloned())
}
fn store_remove(store: &Mutex<Option<HashMap<usize, Vec<u8>>>>, addr: usize) {
    if let Some(m) = store.lock().unwrap().as_mut() { m.remove(&addr); }
}

fn leak_vec(v: Vec<u8>) -> &'static [u8] {
    Box::leak(v.into_boxed_slice())
}

fn get_md(hf: BRSAHashFunction) -> MessageDigest {
    match hf {
        BRSAHashFunction::BRSA_SHA256 => MessageDigest::sha256(),
        BRSAHashFunction::BRSA_SHA384 => MessageDigest::sha384(),
        BRSAHashFunction::BRSA_SHA512 => MessageDigest::sha512(),
    }
}

fn recover_pk_rsa(pk: &BRSAPublicKey) -> Option<Rsa<Public>> {
    let der = store_get(&PK_STORE, pk as *const _ as usize)?;
    Rsa::public_key_from_der_pkcs1(&der).ok()
}

fn recover_sk_rsa(sk: &BRSASecretKey) -> Option<Rsa<Private>> {
    let der = store_get(&SK_STORE, sk as *const _ as usize)?;
    Rsa::private_key_from_der(&der).ok()
}

fn rsa_parameters_check_rsa<T: openssl::pkey::HasPublic>(rsa: &Rsa<T>) -> bool {
    let bits = rsa.size() as usize * 8;
    if bits < MIN_MODULUS_BITS || bits > MAX_MODULUS_BITS { return false; }
    let e = rsa.e();
    let e3 = BigNum::from_u32(3).unwrap();
    let ef4 = BigNum::from_u32(65537).unwrap();
    e == &*e3 || e == &*ef4
}

fn hash_bytes(md: MessageDigest, parts: &[&[u8]]) -> Option<Vec<u8>> {
    let mut h = Hasher::new(md).ok()?;
    for p in parts { h.update(p).ok()?; }
    Some(h.finish().ok()?.to_vec())
}

// MGF1 mask generation function (RFC 8017 B.2.1)
fn mgf1(md: MessageDigest, seed: &[u8], len: usize) -> Option<Vec<u8>> {
    let h_len = md.size();
    let mut out = Vec::with_capacity(len);
    let mut counter = 0u32;
    while out.len() < len {
        let c = counter.to_be_bytes();
        let h = hash_bytes(md, &[seed, &c])?;
        out.extend_from_slice(&h);
        counter += 1;
    }
    out.truncate(len);
    Some(out)
}

fn xor_bytes(a: &mut [u8], b: &[u8]) {
    for (x, y) in a.iter_mut().zip(b.iter()) { *x ^= y; }
}

// EMSA-PSS-ENCODE (RFC 8017 Section 9.1.1)
fn pss_encode(md: MessageDigest, m_hash: &[u8], em_len: usize, salt_len: usize) -> Option<Vec<u8>> {
    let h_len = md.size();
    if em_len < h_len + salt_len + 2 { return None; }

    let mut salt = vec![0u8; salt_len];
    if salt_len > 0 {
        openssl::rand::rand_bytes(&mut salt).ok()?;
    }

    // M' = (0x)00 00 00 00 00 00 00 00 || mHash || salt
    let padding = [0u8; 8];
    let h = hash_bytes(md, &[&padding, m_hash, &salt])?;

    let ps_len = em_len - salt_len - h_len - 2;
    let mut db = vec![0u8; ps_len + 1 + salt_len];
    db[ps_len] = 0x01;
    db[ps_len + 1..].copy_from_slice(&salt);

    let db_mask = mgf1(md, &h, db.len())?;
    xor_bytes(&mut db, &db_mask);

    // Set leftmost bits to zero
    let top_bits = (8 * em_len - (em_len * 8 - 1)).min(8);
    // Actually: emBits = 8*emLen - 1 for RSA
    // The number of zero bits at the top = 8*emLen - emBits = 8*emLen - (modBits-1)
    // For simplicity, since emLen = ceil((modBits-1)/8), the top bit should be 0
    db[0] &= 0x7f;

    let mut em = Vec::with_capacity(em_len);
    em.extend_from_slice(&db);
    em.extend_from_slice(&h);
    em.push(0xbc);
    Some(em)
}

// EMSA-PSS-VERIFY (RFC 8017 Section 9.1.2)
fn pss_verify(md: MessageDigest, m_hash: &[u8], em: &[u8], em_bits: usize, salt_len: usize) -> bool {
    let h_len = md.size();
    let em_len = em.len();
    if em_len < h_len + salt_len + 2 { return false; }
    if em[em_len - 1] != 0xbc { return false; }

    let db_len = em_len - h_len - 1;
    let db = &em[..db_len];
    let h = &em[db_len..db_len + h_len];

    // Check top bits are zero
    let top_mask: u8 = 0xff << (8 - (8 * em_len - em_bits));
    if em_len * 8 > em_bits && (db[0] & top_mask) != 0 { return false; }

    let db_mask = match mgf1(md, h, db_len) {
        Some(m) => m,
        None => return false,
    };
    let mut db_unmasked = db.to_vec();
    xor_bytes(&mut db_unmasked, &db_mask);

    // Zero top bits
    if em_len * 8 > em_bits {
        db_unmasked[0] &= !top_mask;
    }

    let ps_len = em_len - h_len - salt_len - 2;
    for i in 0..ps_len {
        if db_unmasked[i] != 0 { return false; }
    }
    if db_unmasked[ps_len] != 0x01 { return false; }

    let salt = &db_unmasked[ps_len + 1..];
    let padding = [0u8; 8];
    let h_check = match hash_bytes(md, &[&padding, m_hash, salt]) {
        Some(h) => h,
        None => return false,
    };
    h == h_check
}

fn bn_to_padded(bn: &openssl::bn::BigNumRef, len: usize) -> Vec<u8> {
    let b = bn.to_vec();
    let mut out = vec![0u8; len];
    if b.len() <= len {
        out[len - b.len()..].copy_from_slice(&b);
    }
    out
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
        let md = get_md(hash_function);
        self.salt_len = if salt_len == BRSA_DEFAULT_SALT_LENGTH { md.size() } else { salt_len };
        CTX_HASH_STORE.lock().unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(self as *const _ as usize, hash_function);
        0
    }

    fn md(&self) -> MessageDigest {
        let hf = CTX_HASH_STORE.lock().unwrap()
            .as_ref()
            .and_then(|m| m.get(&(self as *const _ as usize)).copied())
            .unwrap_or(BRSAHashFunction::BRSA_SHA384);
        get_md(hf)
    }

    fn hash_msg(&self, prefix: Option<&BRSAMessageRandomizer>, msg: &[u8]) -> Option<Vec<u8>> {
        let md = self.md();
        let mut h = Hasher::new(md).ok()?;
        // C code: when prefix != NULL, hashes msg twice (msg || msg)
        if prefix.is_some() { h.update(msg).ok()?; }
        h.update(msg).ok()?;
        Some(h.finish().ok()?.to_vec())
    }

    pub fn brsa_blind_message_generate(
        &self,
        blind_message: &mut BRSABlindMessage,
        msg: &[u8],
        msg_len: usize,
        secret: &mut BRSABlindingSecret,
        pk: &mut BRSAPublicKey,
    ) -> i32 {
        // Fill msg with random bytes (msg is &[u8] but C API treats it as mutable)
        unsafe {
            let p = msg.as_ptr() as *mut u8;
            let s = std::slice::from_raw_parts_mut(p, msg_len);
            if openssl::rand::rand_bytes(s).is_err() { return -1; }
        }
        self.blind_inner(blind_message, secret, None, pk, &msg[..msg_len])
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
        if openssl::rand::rand_bytes(&mut msg_randomizer.noise).is_err() { return -1; }
        self.blind_inner(blind_message, secret, Some(msg_randomizer), pk, &msg[..msg_len])
    }

    fn blind_inner(
        &self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: Option<&BRSAMessageRandomizer>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
    ) -> i32 {
        let pk_rsa = match recover_pk_rsa(pk) { Some(r) => r, None => return -1 };
        if !rsa_parameters_check_rsa(&pk_rsa) { return -1; }
        let modulus_bytes = pk_rsa.size() as usize;
        let md = self.md();

        let msg_hash = match self.hash_msg(msg_randomizer, msg) { Some(h) => h, None => return -1 };

        // PSS encode
        // emBits = modBits - 1
        let em_bits = modulus_bytes * 8 - 1;
        let padded = match pss_encode(md, &msg_hash, modulus_bytes, self.salt_len) {
            Some(p) => p, None => return -1
        };

        let mut ctx = match BigNumContext::new() { Ok(c) => c, Err(_) => return -1 };
        let m = match BigNum::from_slice(&padded) { Ok(b) => b, Err(_) => return -1 };
        let n = match pk_rsa.n().to_owned() { Ok(b) => b, Err(_) => return -1 };
        let e = match pk_rsa.e().to_owned() { Ok(b) => b, Err(_) => return -1 };

        // Check gcd(m, n) == 1
        let mut gcd = BigNum::new().unwrap();
        if gcd.gcd(&m, &n, &mut ctx).is_err() { return -1; }
        let one = BigNum::from_u32(1).unwrap();
        if gcd != one { return -1; }

        // Random blinding factor
        let mut secret_inv;
        let mut secret_bn = BigNum::new().unwrap();
        loop {
            let mut tmp = BigNum::new().unwrap();
            if n.rand_range(&mut tmp).is_err() { return -1; }
            secret_inv = tmp;
            if secret_inv == one { continue; }
            match secret_bn.mod_inverse(&secret_inv, &n, &mut ctx) {
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

        blind_message.blind_message = leak_vec(bn_to_padded(&blind_m, modulus_bytes));
        blind_message.blind_message_len = modulus_bytes;
        secret.secret = leak_vec(bn_to_padded(&secret_bn, modulus_bytes));
        secret.secret_len = modulus_bytes;
        0
    }

    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let sk_rsa = match recover_sk_rsa(sk) { Some(r) => r, None => return -1 };
        if !rsa_parameters_check_rsa(&sk_rsa) { return -1; }
        let modulus_bytes = sk_rsa.size() as usize;
        if blind_message.blind_message_len != modulus_bytes { return -1; }

        // Check canonical: blind_message < n
        let n_padded = bn_to_padded(sk_rsa.n(), modulus_bytes);
        for i in 0..modulus_bytes {
            let a = blind_message.blind_message[i];
            let b = n_padded[i];
            if a < b { break; }
            if a > b || i + 1 == modulus_bytes { return -1; }
        }

        let mut result = vec![0u8; modulus_bytes];
        match sk_rsa.private_encrypt(blind_message.blind_message, &mut result, Padding::NONE) {
            Ok(_) => {
                blind_sig.blind_sig = leak_vec(result);
                blind_sig.blind_sig_len = modulus_bytes;
                0
            }
            Err(_) => -1,
        }
    }

    fn rsassa_pss_verify_inner(
        &self,
        sig: &BRSASignature,
        pk: &BRSAPublicKey,
        msg_randomizer: Option<&BRSAMessageRandomizer>,
        msg: &[u8],
    ) -> i32 {
        let pk_rsa = match recover_pk_rsa(pk) { Some(r) => r, None => return -1 };
        let modulus_bytes = pk_rsa.size() as usize;
        if sig.sig_len != modulus_bytes { return -1; }

        let msg_hash = match self.hash_msg(msg_randomizer, msg) { Some(h) => h, None => return -1 };

        let mut em = vec![0u8; modulus_bytes];
        if pk_rsa.public_decrypt(sig.sig, &mut em, Padding::NONE).is_err() { return -1; }

        let em_bits = modulus_bytes * 8 - 1;
        if !pss_verify(self.md(), &msg_hash, &em, em_bits, self.salt_len) { return -1; }
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
        let pk_rsa = match recover_pk_rsa(pk) { Some(r) => r, None => return -1 };
        if !rsa_parameters_check_rsa(&pk_rsa) { return -1; }
        let modulus_bytes = pk_rsa.size() as usize;
        if blind_sig.blind_sig_len != modulus_bytes || secret_.secret_len != modulus_bytes {
            return -1;
        }

        let mut ctx = match BigNumContext::new() { Ok(c) => c, Err(_) => return -1 };
        let secret_bn = match BigNum::from_slice(secret_.secret) { Ok(b) => b, Err(_) => return -1 };
        let blind_z = match BigNum::from_slice(blind_sig.blind_sig) { Ok(b) => b, Err(_) => return -1 };
        let n = match pk_rsa.n().to_owned() { Ok(b) => b, Err(_) => return -1 };

        let mut z = BigNum::new().unwrap();
        if z.mod_mul(&blind_z, &secret_bn, &n, &mut ctx).is_err() { return -1; }

        sig.sig = leak_vec(bn_to_padded(&z, modulus_bytes));
        sig.sig_len = modulus_bytes;

        if self.rsassa_pss_verify_inner(sig, pk, msg_randomizer.as_ref(), &msg[..msg_len]) != 0 {
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
        self.rsassa_pss_verify_inner(sig, pk, msg_randomizer.as_ref(), &msg[..msg_len])
    }

    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pk_rsa = match recover_pk_rsa(pk) { Some(r) => r, None => return -1 };
        let pk_der = match pk_rsa.public_key_to_der_pkcs1() { Ok(d) => d, Err(_) => return -1 };

        let alg_oid: &[u8] = match self.md().type_().as_raw() {
            nid if nid == openssl::nid::Nid::SHA256.as_raw() =>
                &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            nid if nid == openssl::nid::Nid::SHA384.as_raw() =>
                &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            nid if nid == openssl::nid::Nid::SHA512.as_raw() =>
                &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
            _ => return -1,
        };

        // Template matching C code's rsassa_pss_s_template
        let mut t: Vec<u8> = vec![
            0x30, 0x82, 0, 0,
            0x30, 61,
                0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a,
                0x30, 48,
                    0xa0, 13, 0x30, 11, 0x06, 9, 0,0,0,0,0,0,0,0,0,
                    0xa1, 26, 0x30, 24,
                        0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08,
                        0x30, 11, 0x06, 9, 0,0,0,0,0,0,0,0,0,
                    0xa2, 3, 0x02, 1, 0,
            0x03, 0x82, 0, 0,
                0,
        ];
        let template_len = t.len();
        t.extend_from_slice(&pk_der);

        let container_len = template_len - 4 + pk_der.len();
        t[2] = (container_len >> 8) as u8;
        t[3] = (container_len & 0xff) as u8;
        t[66] = (self.salt_len & 0xff) as u8;
        t[69] = ((1 + pk_der.len()) >> 8) as u8;
        t[70] = ((1 + pk_der.len()) & 0xff) as u8;

        // Hash OID at offsets 25..34 and 53..62
        t[25..34].copy_from_slice(alg_oid);
        t[53..62].copy_from_slice(alg_oid);

        let len = t.len();
        spki.bytes = leak_vec(t);
        spki.bytes_len = len;
        0
    }

    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        let template_len = 72usize;
        if spki_len > MAX_SERIALIZED_PK_LEN || spki_len <= template_len { return -1; }
        // Check RSASSA-PSS OID
        if spki.len() < 17 || spki[6..17] != [0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a] {
            return -1;
        }
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
        let mut spki_key = BRSASerializedKey::new();
        if self.brsa_publickey_export_spki(&mut spki_key, pk) != 0 { return -1; }
        let hash = openssl::sha::sha256(spki_key.bytes);
        spki_key.brsa_serializedkey_deinit();

        let id_mut = unsafe { std::slice::from_raw_parts_mut(id.as_ptr() as *mut u8, id_len) };
        let out_len = id_len.min(hash.len());
        id_mut[..out_len].copy_from_slice(&hash[..out_len]);
        if out_len < id_len {
            for b in &mut id_mut[out_len..] { *b = 0; }
        }
        0
    }
}

pub struct BRSABlindMessage<'a> {
    pub blind_message: &'a [u8],
    pub blind_message_len: usize,
}
impl BRSABlindMessage<'_> {
    pub fn new() -> Self { BRSABlindMessage { blind_message: &[], blind_message_len: 0 } }
    pub fn brsa_blind_message_init(&mut self, modulus_bytes: usize) {
        self.blind_message = leak_vec(vec![0u8; modulus_bytes]);
        self.blind_message_len = modulus_bytes;
    }
    pub fn brsa_blind_message_deinit(&mut self) {
        free_leaked_slice(self.blind_message, self.blind_message_len);
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
    pub fn new() -> Self { BRSABlindingSecret { secret: &[], secret_len: 0 } }
    pub fn brsa_blinding_secrete_init(&mut self, modulus_bytes: usize) {
        self.secret = leak_vec(vec![0u8; modulus_bytes]);
        self.secret_len = modulus_bytes;
    }
    pub fn brsa_blinding_secret_deinit(&mut self) {
        free_leaked_slice(self.secret, self.secret_len);
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
    pub fn new() -> Self { BRSABlindSignature { blind_sig: &[], blind_sig_len: 0 } }
    pub fn brsa_blind_signature_init(&mut self, blind_sig_len: usize) {
        self.blind_sig = leak_vec(vec![0u8; blind_sig_len]);
        self.blind_sig_len = blind_sig_len;
    }
    pub fn brsa_blind_signature_deinit(&mut self) {
        free_leaked_slice(self.blind_sig, self.blind_sig_len);
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
    pub fn new() -> Self { BRSASignature { sig: &[], sig_len: 0 } }
    pub fn brsa_signature_init(&mut self, sig_len: usize) {
        self.sig = leak_vec(vec![0u8; sig_len]);
        self.sig_len = sig_len;
    }
    pub fn brsa_signature_deinit(&mut self) {
        free_leaked_slice(self.sig, self.sig_len);
        self.sig = &[];
        self.sig_len = 0;
    }
}

fn free_leaked_slice(s: &[u8], len: usize) {
    if !s.is_empty() && len > 0 {
        unsafe {
            let ptr = s.as_ptr() as *mut u8;
            std::ptr::write_bytes(ptr, 0, len);
            drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)));
        }
    }
}

pub struct BRSAPublicKey {
    pub evp_pkey: Option<EVP_PKEY>,
    pub mont_ctx: Option<BN_MONT_CTX>,
}
impl BRSAPublicKey {
    pub fn new() -> Self { BRSAPublicKey { evp_pkey: None, mont_ctx: None } }
    pub fn brsa_publickey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        if der_len > MAX_SERIALIZED_PK_LEN { return -1; }
        let rsa = match Rsa::public_key_from_der_pkcs1(&der[..der_len]) { Ok(r) => r, Err(_) => return -1 };
        if !rsa_parameters_check_rsa(&rsa) { return -1; }
        store_set(&PK_STORE, self as *const _ as usize, der[..der_len].to_vec());
        0
    }
    pub fn brsa_publickey_deinit(&mut self) {
        store_remove(&PK_STORE, self as *const _ as usize);
    }
    pub fn brsa_publickey_recover(&mut self, sk: &BRSASecretKey) -> i32 {
        let sk_rsa = match recover_sk_rsa(sk) { Some(r) => r, None => return -1 };
        let pk_der = match sk_rsa.public_key_to_der_pkcs1() { Ok(d) => d, Err(_) => return -1 };
        self.brsa_publickey_import(&pk_der, pk_der.len())
    }
}

pub struct BRSASecretKey {
    pub evp_pkey: Option<EVP_PKEY>,
}
impl BRSASecretKey {
    pub fn new() -> Self { BRSASecretKey { evp_pkey: None } }
    pub fn brsa_keypair_generate(&mut self, pk: &mut BRSAPublicKey, modulus_bits: c_int) -> i32 {
        let rsa = match Rsa::generate(modulus_bits as u32) { Ok(r) => r, Err(_) => return -1 };
        let sk_der = match rsa.private_key_to_der() { Ok(d) => d, Err(_) => return -1 };
        store_set(&SK_STORE, self as *const _ as usize, sk_der);
        pk.brsa_publickey_recover(self)
    }
    pub fn brsa_secretkey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        if Rsa::<Private>::private_key_from_der(&der[..der_len]).is_err() { return -1; }
        store_set(&SK_STORE, self as *const _ as usize, der[..der_len].to_vec());
        0
    }
    pub fn brsa_secretkey_deinit(&mut self) {
        store_remove(&SK_STORE, self as *const _ as usize);
    }
}

#[derive(Debug)]
pub struct BRSASerializedKey<'a> {
    pub bytes: &'a [u8],
    pub bytes_len: usize,
}
impl BRSASerializedKey<'_> {
    pub fn new() -> Self { BRSASerializedKey { bytes: &[], bytes_len: 0 } }
    pub fn brsa_secretkey_export(&mut self, sk: &BRSASecretKey) -> i32 {
        let sk_rsa = match recover_sk_rsa(sk) { Some(r) => r, None => return -1 };
        let der = match sk_rsa.private_key_to_der() { Ok(d) => d, Err(_) => return -1 };
        self.bytes_len = der.len();
        self.bytes = leak_vec(der);
        0
    }
    pub fn brsa_publickey_export(&mut self, pk: &BRSAPublicKey) -> i32 {
        let pk_rsa = match recover_pk_rsa(pk) { Some(r) => r, None => return -1 };
        let der = match pk_rsa.public_key_to_der_pkcs1() { Ok(d) => d, Err(_) => return -1 };
        self.bytes_len = der.len();
        self.bytes = leak_vec(der);
        0
    }
    pub fn brsa_serializedkey_deinit(&mut self) {
        free_leaked_slice(self.bytes, self.bytes_len);
        self.bytes = &[];
        self.bytes_len = 0;
    }
}

#[derive(Debug)]
pub struct BRSAMessageRandomizer {
    pub noise: [u8; 32],
}
impl BRSAMessageRandomizer {
    pub fn new() -> Self { BRSAMessageRandomizer { noise: [0u8; 32] } }
}

// Constants
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

// Standalone functions - these take opaque zero-sized types and serve as API stubs
pub fn BN_bn2bin_padded(OUT: &mut [u8], LEN: c_int, IN: Option<BIGNUM>) -> bool { false }
pub fn _rsa_bits(evp_pkey: Option<EVP_PKEY>) -> i32 { 0 }
pub fn _rsa_size(evp_pkey: Option<EVP_PKEY>) -> usize { 0 }
pub fn _rsa_n(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> { None }
pub fn _rsa_e(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> { None }
pub fn new_mont_domain(n: Option<BIGNUM>) -> Option<BN_MONT_CTX> { None }
pub fn _rsa_parameters_check(evp_pkey: Option<EVP_PKEY>) -> i32 { -1 }
pub fn _hash(evp_md: Option<EVP_MD>, prefix: &BRSAMessageRandomizer, msg_hash: &[u8], msg: &[u8]) -> i32 { -1 }
pub fn _blind(blind_message: &BRSABlindMessage, secret: &BRSABlindingSecret, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, padded: &[u8]) -> i32 { -1 }
pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 { -1 }
pub fn _finalize(context: &BRSAContext, sig: &BRSASignature, blind_sig: &BRSABlindSignature,
    secret: &BRSABlindingSecret, msg_randomizer: &BRSAMessageRandomizer, pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>, msg: &[u8]) -> i32 { -1 }
