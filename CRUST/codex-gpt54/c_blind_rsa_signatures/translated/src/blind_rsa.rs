use openssl_sys::*;
use num_bigint_dig::{BigInt, BigUint, ToBigInt};
use num_traits::{One, Signed, Zero};
use rand::rngs::OsRng;
use rand::RngCore;
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::collections::HashMap;
use std::os::raw::c_int;
use std::sync::{LazyLock, Mutex};

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

#[derive(Clone, Copy)]
struct ContextState {
    hash_function: BRSAHashFunction,
}

static CONTEXTS: LazyLock<Mutex<HashMap<usize, ContextState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SECRET_KEYS: LazyLock<Mutex<HashMap<usize, RsaPrivateKey>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static PUBLIC_KEYS: LazyLock<Mutex<HashMap<usize, RsaPublicKey>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct BRSAContext {
    pub evp_md: Option<EVP_MD>,
    pub salt_len: usize,
}

impl BRSAContext {
    pub fn new() -> Self {
        Self {
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
        self.evp_md = None;
        self.salt_len = if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            hash_output_len(hash_function)
        } else {
            salt_len
        };
        set_context_state(self, ContextState { hash_function });
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
        if msg_len > msg.len() {
            return -1;
        }

        let mut generated = vec![0u8; msg_len];
        OsRng.fill_bytes(&mut generated);

        // The provided Rust signature lost mutability compared to the C API.
        // Mirror the C behavior for the expected mutable callers.
        unsafe {
            std::ptr::copy_nonoverlapping(generated.as_ptr(), msg.as_ptr() as *mut u8, msg_len);
        }

        blind_message.brsa_blind_message_deinit();
        secret.brsa_blinding_secret_deinit();

        blind_message_from_parts(
            self,
            blind_message,
            secret,
            None,
            pk,
            &generated,
        )
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
        if msg_len > msg.len() {
            return -1;
        }

        blind_message.brsa_blind_message_deinit();
        secret.brsa_blinding_secret_deinit();

        blind_message_from_parts(
            &self,
            blind_message,
            secret,
            Some(msg_randomizer),
            pk,
            &msg[..msg_len],
        )
    }

    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let skey = match get_secret_key(sk) {
            Some(key) => key,
            None => return -1,
        };
        if check_private_key(&skey) != 0 || check_canonical(&skey, blind_message) != 0 {
            return -1;
        }

        let modulus_len = modulus_bytes_from_public(&skey.to_public_key());
        let blind_m = BigUint::from_bytes_be(blind_message.blind_message);
        let blind_sig_value = blind_m.modpow(skey.d(), skey.n());
        let encoded = match biguint_to_padded_bytes(&blind_sig_value, modulus_len) {
            Some(bytes) => bytes,
            None => return -1,
        };

        blind_sig.brsa_blind_signature_deinit();
        blind_sig.blind_sig = leak_bytes(encoded.clone());
        blind_sig.blind_sig_len = encoded.len();
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
        if msg_len > msg.len() {
            return -1;
        }

        let pkey = match get_public_key(pk) {
            Some(key) => key,
            None => return -1,
        };
        if check_public_key(&pkey) != 0 {
            return -1;
        }

        let modulus_len = modulus_bytes_from_public(&pkey);
        if blind_sig.blind_sig_len != modulus_len || secret_.secret_len != modulus_len {
            return -1;
        }

        let blind_z = BigUint::from_bytes_be(blind_sig.blind_sig);
        let secret = BigUint::from_bytes_be(secret_.secret);
        let z = (blind_z * secret) % pkey.n();
        let encoded = match biguint_to_padded_bytes(&z, modulus_len) {
            Some(bytes) => bytes,
            None => return -1,
        };

        sig.brsa_signature_deinit();
        sig.sig = leak_bytes(encoded.clone());
        sig.sig_len = encoded.len();

        if rsassa_pss_verify(
            self,
            sig.sig,
            msg_randomizer.as_ref(),
            &pkey,
            &msg[..msg_len],
        ) != 0
        {
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
        if msg_len > msg.len() {
            return -1;
        }
        let pkey = match get_public_key(pk) {
            Some(key) => key,
            None => return -1,
        };
        rsassa_pss_verify(self, sig.sig, msg_randomizer.as_ref(), &pkey, &msg[..msg_len])
    }

    pub fn brsa_publickey_export_spki(
        &self,
        spki: &mut BRSASerializedKey,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pkey = match get_public_key(pk) {
            Some(key) => key,
            None => return -1,
        };

        let raw = match pkey.to_pkcs1_der() {
            Ok(doc) => doc.as_bytes().to_vec(),
            Err(_) => return -1,
        };

        let mut spki_bytes = RSASSA_PSS_TEMPLATE.to_vec();
        spki_bytes.extend_from_slice(&raw);

        let container_len = RSASSA_PSS_TEMPLATE.len() - 4 + raw.len();
        if container_len > u16::MAX as usize || raw.len() + 1 > u16::MAX as usize {
            return -1;
        }

        let digest_alg = digest_algorithm_identifier(context_hash_function(self));
        spki_bytes[2] = ((container_len >> 8) & 0xff) as u8;
        spki_bytes[3] = (container_len & 0xff) as u8;
        spki_bytes[21..34].copy_from_slice(&digest_alg);
        spki_bytes[49..62].copy_from_slice(&digest_alg);
        spki_bytes[66] = (self.salt_len & 0xff) as u8;
        spki_bytes[69] = (((raw.len() + 1) >> 8) & 0xff) as u8;
        spki_bytes[70] = ((raw.len() + 1) & 0xff) as u8;

        spki.brsa_serializedkey_deinit();
        spki.bytes = leak_bytes(spki_bytes.clone());
        spki.bytes_len = spki_bytes.len();
        0
    }

    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        let _ = self;

        if spki_len > spki.len()
            || spki_len > MAX_SERIALIZED_PK_LEN
            || spki_len <= RSASSA_PSS_TEMPLATE.len()
        {
            return -1;
        }
        let spki = &spki[..spki_len];

        if RSASSA_PSS_TEMPLATE[6..18] != spki[6..18] {
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
        let mut spki = BRSASerializedKey::new();
        if self.brsa_publickey_export_spki(&mut spki, pk) != 0 {
            return -1;
        }

        let digest = Sha256::digest(spki.bytes);
        let out_len = id_len.min(digest.len()).min(id.len());

        // The provided Rust signature lost mutability compared to the C API.
        unsafe {
            std::ptr::copy_nonoverlapping(digest.as_ptr(), id.as_ptr() as *mut u8, out_len);
            if id_len > digest.len() && id.len() >= id_len {
                std::ptr::write_bytes(id.as_ptr().add(digest.len()) as *mut u8, 0, id_len - digest.len());
            }
        }

        spki.brsa_serializedkey_deinit();
        0
    }
}

pub struct BRSABlindMessage<'a> {
    pub blind_message: &'a [u8],
    pub blind_message_len: usize,
}

impl BRSABlindMessage<'_> {
    pub fn new() -> Self {
        Self {
            blind_message: &[],
            blind_message_len: 0,
        }
    }

    pub fn brsa_blind_message_init(&mut self, modulus_bytes: usize) {
        self.blind_message = leak_bytes(vec![0u8; modulus_bytes]);
        self.blind_message_len = modulus_bytes;
    }

    pub fn brsa_blind_message_deinit(&mut self) {
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
        Self {
            secret: &[],
            secret_len: 0,
        }
    }

    pub fn brsa_blinding_secrete_init(&mut self, modulus_bytes: usize) {
        self.secret = leak_bytes(vec![0u8; modulus_bytes]);
        self.secret_len = modulus_bytes;
    }

    pub fn brsa_blinding_secret_deinit(&mut self) {
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
        Self {
            blind_sig: &[],
            blind_sig_len: 0,
        }
    }

    pub fn brsa_blind_signature_init(&mut self, blind_sig_len: usize) {
        self.blind_sig = leak_bytes(vec![0u8; blind_sig_len]);
        self.blind_sig_len = blind_sig_len;
    }

    pub fn brsa_blind_signature_deinit(&mut self) {
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
        Self { sig: &[], sig_len: 0 }
    }

    pub fn brsa_signature_init(&mut self, sig_len: usize) {
        self.sig = leak_bytes(vec![0u8; sig_len]);
        self.sig_len = sig_len;
    }

    pub fn brsa_signature_deinit(&mut self) {
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
        Self {
            evp_pkey: None,
            mont_ctx: None,
        }
    }

    pub fn brsa_publickey_import(&mut self, der: &[u8], der_len: usize) -> i32 {
        self.brsa_publickey_deinit();
        if der_len > der.len() || der_len > MAX_SERIALIZED_PK_LEN {
            return -1;
        }

        let key = match RsaPublicKey::from_pkcs1_der(&der[..der_len]) {
            Ok(key) => key,
            Err(_) => return -1,
        };
        if check_public_key(&key) != 0 {
            return -1;
        }

        set_public_key(self, key);
        self.evp_pkey = None;
        self.mont_ctx = None;
        0
    }

    pub fn brsa_publickey_deinit(&mut self) {
        PUBLIC_KEYS.lock().unwrap().remove(&object_id(self));
        self.evp_pkey = None;
        self.mont_ctx = None;
    }

    pub fn brsa_publickey_recover(&mut self, sk: &BRSASecretKey) -> i32 {
        let skey = match get_secret_key(sk) {
            Some(key) => key,
            None => return -1,
        };
        let pkey = skey.to_public_key();
        if check_public_key(&pkey) != 0 {
            return -1;
        }

        self.brsa_publickey_deinit();
        set_public_key(self, pkey);
        0
    }
}

pub struct BRSASecretKey {
    pub evp_pkey: Option<EVP_PKEY>,
}

impl BRSASecretKey {
    pub fn new() -> Self {
        Self { evp_pkey: None }
    }

    pub fn brsa_keypair_generate(
        &mut self,
        pk: &mut BRSAPublicKey,
        modulus_bits: c_int,
    ) -> i32 {
        self.brsa_secretkey_deinit();
        pk.brsa_publickey_deinit();

        if modulus_bits <= 0 {
            return -1;
        }

        let mut rng = OsRng;
        let public_exponent = BigUint::from(65537u32);
        let key = match RsaPrivateKey::new_with_exp(
            &mut rng,
            modulus_bits as usize,
            &public_exponent,
        ) {
            Ok(key) => key,
            Err(_) => return -1,
        };

        set_secret_key(self, key.clone());
        set_public_key(pk, key.to_public_key());
        0
    }

    pub fn brsa_secretkey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        self.brsa_secretkey_deinit();
        if der_len > der.len() {
            return -1;
        }

        let key = match RsaPrivateKey::from_pkcs1_der(&der[..der_len]) {
            Ok(key) => key,
            Err(_) => return -1,
        };
        set_secret_key(self, key);
        0
    }

    pub fn brsa_secretkey_deinit(&mut self) {
        SECRET_KEYS.lock().unwrap().remove(&object_id(self));
        self.evp_pkey = None;
    }
}

#[derive(Debug)]
pub struct BRSASerializedKey<'a> {
    pub bytes: &'a [u8],
    pub bytes_len: usize,
}

impl BRSASerializedKey<'_> {
    pub fn new() -> Self {
        Self { bytes: &[], bytes_len: 0 }
    }

    pub fn brsa_secretkey_export(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let skey = match get_secret_key(sk) {
            Some(key) => key,
            None => return -1,
        };
        let der = match skey.to_pkcs1_der() {
            Ok(doc) => doc.as_bytes().to_vec(),
            Err(_) => return -1,
        };

        self.brsa_serializedkey_deinit();
        self.bytes = leak_bytes(der.clone());
        self.bytes_len = der.len();
        0
    }

    pub fn brsa_publickey_export(
        &mut self,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let pkey = match get_public_key(pk) {
            Some(key) => key,
            None => return -1,
        };
        let der = match pkey.to_pkcs1_der() {
            Ok(doc) => doc.as_bytes().to_vec(),
            Err(_) => return -1,
        };

        self.brsa_serializedkey_deinit();
        self.bytes = leak_bytes(der.clone());
        self.bytes_len = der.len();
        0
    }

    pub fn brsa_serializedkey_deinit(&mut self) {
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
        Self { noise: [0u8; 32] }
    }
}

pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

#[allow(non_snake_case)]
pub fn BN_bn2bin_padded(OUT: &mut [u8], LEN: c_int, IN: Option<BIGNUM>) -> bool {
    let _ = (OUT, LEN, IN);
    false
}

pub fn _rsa_bits(evp_pkey: Option<EVP_PKEY>) -> i32 {
    let _ = evp_pkey;
    0
}

pub fn _rsa_size(evp_pkey: Option<EVP_PKEY>) -> usize {
    let _ = evp_pkey;
    0
}

pub fn _rsa_n(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    let _ = evp_pkey;
    None
}

pub fn _rsa_e(evp_pkey: Option<EVP_PKEY>) -> Option<BIGNUM> {
    let _ = evp_pkey;
    None
}

pub fn new_mont_domain(n: Option<BIGNUM>) -> Option<BN_MONT_CTX> {
    let _ = n;
    None
}

pub fn _rsa_parameters_check(evp_pkey: Option<EVP_PKEY>) -> i32 {
    let _ = evp_pkey;
    0
}

pub fn _hash(evp_md: Option<EVP_MD>, prefix: &BRSAMessageRandomizer, msg_hash: &[u8], msg: &[u8]) -> i32 {
    let _ = (evp_md, prefix, msg_hash, msg);
    0
}

pub fn _blind(
    blind_message: &BRSABlindMessage,
    secret: &BRSABlindingSecret,
    pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>,
    padded: &[u8],
) -> i32 {
    let _ = (blind_message, secret, pk, bn_ctx, padded);
    0
}

pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 {
    let skey = match get_secret_key(sk) {
        Some(key) => key,
        None => return -1,
    };
    check_canonical(&skey, blind_message)
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
    let _ = (sig, bn_ctx);
    let mut sig_out = BRSASignature::new();
    let mut pk_clone = BRSAPublicKey::new();
    if let Some(key) = get_public_key(pk) {
        set_public_key(&mut pk_clone, key);
    } else {
        return -1;
    }
    context.brsa_finalize(
        &mut sig_out,
        blind_sig,
        secret,
        &Some(BRSAMessageRandomizer { noise: msg_randomizer.noise }),
        &mut pk_clone,
        msg,
        msg.len(),
    )
}

const RSASSA_PSS_TEMPLATE: [u8; 72] = [
    0x30, 0x82, 0x00, 0x00,
    0x30, 61,
    0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0a,
    0x30, 48,
    0xa0, 11,
    0x30, 11, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0xa1, 26,
    0x30, 24, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08,
    0x30, 11, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0xa2, 3, 0x02, 1, 0,
    0x03, 0x82, 0x00, 0x00,
    0,
];

fn object_id<T>(value: &T) -> usize {
    value as *const T as usize
}

fn leak_bytes(bytes: Vec<u8>) -> &'static [u8] {
    Box::leak(bytes.into_boxed_slice())
}

fn set_context_state(context: &BRSAContext, state: ContextState) {
    CONTEXTS.lock().unwrap().insert(object_id(context), state);
}

fn context_hash_function(context: &BRSAContext) -> BRSAHashFunction {
    CONTEXTS
        .lock()
        .unwrap()
        .get(&object_id(context))
        .copied()
        .map(|state| state.hash_function)
        .unwrap_or_else(|| infer_hash_function_from_salt_len(context.salt_len))
}

fn infer_hash_function_from_salt_len(salt_len: usize) -> BRSAHashFunction {
    match salt_len {
        32 => BRSAHashFunction::BRSA_SHA256,
        64 => BRSAHashFunction::BRSA_SHA512,
        _ => BRSAHashFunction::BRSA_SHA384,
    }
}

fn hash_output_len(hash_function: BRSAHashFunction) -> usize {
    match hash_function {
        BRSAHashFunction::BRSA_SHA256 => 32,
        BRSAHashFunction::BRSA_SHA384 => 48,
        BRSAHashFunction::BRSA_SHA512 => 64,
    }
}

fn digest_algorithm_identifier(hash_function: BRSAHashFunction) -> [u8; 13] {
    let oid = match hash_function {
        BRSAHashFunction::BRSA_SHA256 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
        BRSAHashFunction::BRSA_SHA384 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
        BRSAHashFunction::BRSA_SHA512 => [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
    };

    let mut out = [0u8; 13];
    out[0] = 0x30;
    out[1] = 11;
    out[2] = 0x06;
    out[3] = 9;
    out[4..13].copy_from_slice(&oid);
    out
}

fn set_secret_key(sk: &BRSASecretKey, key: RsaPrivateKey) {
    SECRET_KEYS.lock().unwrap().insert(object_id(sk), key);
}

fn get_secret_key(sk: &BRSASecretKey) -> Option<RsaPrivateKey> {
    SECRET_KEYS.lock().unwrap().get(&object_id(sk)).cloned()
}

fn set_public_key(pk: &BRSAPublicKey, key: RsaPublicKey) {
    PUBLIC_KEYS.lock().unwrap().insert(object_id(pk), key);
}

fn get_public_key(pk: &BRSAPublicKey) -> Option<RsaPublicKey> {
    PUBLIC_KEYS.lock().unwrap().get(&object_id(pk)).cloned()
}

fn check_private_key(key: &RsaPrivateKey) -> i32 {
    check_public_key(&key.to_public_key())
}

fn check_public_key(key: &RsaPublicKey) -> i32 {
    let modulus_bits = key.n().bits();
    if modulus_bits < MIN_MODULUS_BITS || modulus_bits > MAX_MODULUS_BITS {
        return -1;
    }

    let e = key.e();
    if *e != BigUint::from(3u8) && *e != BigUint::from(65537u32) {
        return -1;
    }

    0
}

fn modulus_bytes_from_public(key: &RsaPublicKey) -> usize {
    key.n().to_bytes_be().len()
}

fn compute_message_hash(
    context: &BRSAContext,
    randomizer: Option<&BRSAMessageRandomizer>,
    msg: &[u8],
) -> Vec<u8> {
    match context_hash_function(context) {
        BRSAHashFunction::BRSA_SHA256 => {
            let mut hasher = Sha256::new();
            if randomizer.is_some() {
                hasher.update(msg);
            }
            hasher.update(msg);
            hasher.finalize().to_vec()
        }
        BRSAHashFunction::BRSA_SHA384 => {
            let mut hasher = Sha384::new();
            if randomizer.is_some() {
                hasher.update(msg);
            }
            hasher.update(msg);
            hasher.finalize().to_vec()
        }
        BRSAHashFunction::BRSA_SHA512 => {
            let mut hasher = Sha512::new();
            if randomizer.is_some() {
                hasher.update(msg);
            }
            hasher.update(msg);
            hasher.finalize().to_vec()
        }
    }
}

fn mgf1(hash_function: BRSAHashFunction, seed: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut counter = 0u32;

    while out.len() < len {
        let counter_bytes = counter.to_be_bytes();
        let block = match hash_function {
            BRSAHashFunction::BRSA_SHA256 => {
                let mut hasher = Sha256::new();
                hasher.update(seed);
                hasher.update(counter_bytes);
                hasher.finalize().to_vec()
            }
            BRSAHashFunction::BRSA_SHA384 => {
                let mut hasher = Sha384::new();
                hasher.update(seed);
                hasher.update(counter_bytes);
                hasher.finalize().to_vec()
            }
            BRSAHashFunction::BRSA_SHA512 => {
                let mut hasher = Sha512::new();
                hasher.update(seed);
                hasher.update(counter_bytes);
                hasher.finalize().to_vec()
            }
        };
        let take = (len - out.len()).min(block.len());
        out.extend_from_slice(&block[..take]);
        counter = counter.wrapping_add(1);
    }

    out
}

fn emsa_pss_encode(context: &BRSAContext, msg_hash: &[u8], em_bits: usize) -> Option<Vec<u8>> {
    let hash_function = context_hash_function(context);
    let h_len = hash_output_len(hash_function);
    let s_len = context.salt_len;
    let em_len = (em_bits + 7) / 8;

    if msg_hash.len() != h_len || em_len < h_len + s_len + 2 {
        return None;
    }

    let mut salt = vec![0u8; s_len];
    if s_len > 0 {
        OsRng.fill_bytes(&mut salt);
    }

    let mut m_prime = vec![0u8; 8];
    m_prime.extend_from_slice(msg_hash);
    m_prime.extend_from_slice(&salt);

    let h = match hash_function {
        BRSAHashFunction::BRSA_SHA256 => Sha256::digest(&m_prime).to_vec(),
        BRSAHashFunction::BRSA_SHA384 => Sha384::digest(&m_prime).to_vec(),
        BRSAHashFunction::BRSA_SHA512 => Sha512::digest(&m_prime).to_vec(),
    };

    let db_len = em_len - h_len - 1;
    let ps_len = em_len - s_len - h_len - 2;
    let mut db = vec![0u8; db_len];
    db[ps_len] = 0x01;
    db[ps_len + 1..].copy_from_slice(&salt);

    let db_mask = mgf1(hash_function, &h, db_len);
    let mut masked_db: Vec<u8> = db
        .iter()
        .zip(db_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();

    let unused_bits = 8 * em_len - em_bits;
    if unused_bits > 0 {
        masked_db[0] &= 0xff >> unused_bits;
    }

    let mut em = masked_db;
    em.extend_from_slice(&h);
    em.push(0xbc);
    Some(em)
}

fn emsa_pss_verify(context: &BRSAContext, msg_hash: &[u8], em: &[u8], em_bits: usize) -> i32 {
    let hash_function = context_hash_function(context);
    let h_len = hash_output_len(hash_function);
    let s_len = context.salt_len;
    let em_len = (em_bits + 7) / 8;

    if msg_hash.len() != h_len || em.len() != em_len || em_len < h_len + s_len + 2 {
        return -1;
    }
    if em[em_len - 1] != 0xbc {
        return -1;
    }

    let db_len = em_len - h_len - 1;
    let masked_db = &em[..db_len];
    let h = &em[db_len..db_len + h_len];
    let unused_bits = 8 * em_len - em_bits;
    if unused_bits > 0 && (masked_db[0] & !((0xffu8) >> unused_bits)) != 0 {
        return -1;
    }

    let db_mask = mgf1(hash_function, h, db_len);
    let mut db: Vec<u8> = masked_db
        .iter()
        .zip(db_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    if unused_bits > 0 {
        db[0] &= 0xff >> unused_bits;
    }

    let ps_len = em_len - s_len - h_len - 2;
    if db[..ps_len].iter().any(|byte| *byte != 0) || db[ps_len] != 0x01 {
        return -1;
    }
    let salt = &db[ps_len + 1..];

    let mut m_prime = vec![0u8; 8];
    m_prime.extend_from_slice(msg_hash);
    m_prime.extend_from_slice(salt);

    let expected = match hash_function {
        BRSAHashFunction::BRSA_SHA256 => Sha256::digest(&m_prime).to_vec(),
        BRSAHashFunction::BRSA_SHA384 => Sha384::digest(&m_prime).to_vec(),
        BRSAHashFunction::BRSA_SHA512 => Sha512::digest(&m_prime).to_vec(),
    };

    if expected == h { 0 } else { -1 }
}

fn blind_message_from_parts(
    context: &BRSAContext,
    blind_message: &mut BRSABlindMessage,
    secret_out: &mut BRSABlindingSecret,
    msg_randomizer: Option<&mut BRSAMessageRandomizer>,
    pk: &mut BRSAPublicKey,
    msg: &[u8],
) -> i32 {
    let pkey = match get_public_key(pk) {
        Some(key) => key,
        None => return -1,
    };
    if check_public_key(&pkey) != 0 {
        return -1;
    }

    let randomizer_ref = msg_randomizer.map(|randomizer| {
        OsRng.fill_bytes(&mut randomizer.noise);
        &*randomizer
    });

    let msg_hash = compute_message_hash(context, randomizer_ref, msg);
    let modulus_bits = pkey.n().bits();
    let padded = match emsa_pss_encode(context, &msg_hash, modulus_bits - 1) {
        Some(bytes) => bytes,
        None => return -1,
    };

    let m = BigUint::from_bytes_be(&padded);
    if gcd_biguint(m.clone(), pkey.n().clone()) != BigUint::one() {
        return -1;
    }

    let secret_inv = loop {
        let candidate = random_biguint_below(pkey.n());
        if candidate <= BigUint::one() {
            continue;
        }
        if let Some(inverse) = mod_inverse(&candidate, pkey.n()) {
            break (candidate, inverse);
        }
    };
    let (secret_inv, secret) = secret_inv;

    let x = secret_inv.modpow(pkey.e(), pkey.n());
    let blind_m = (m * x) % pkey.n();
    let modulus_len = modulus_bytes_from_public(&pkey);

    let blind_bytes = match biguint_to_padded_bytes(&blind_m, modulus_len) {
        Some(bytes) => bytes,
        None => return -1,
    };
    let secret_bytes = match biguint_to_padded_bytes(&secret, modulus_len) {
        Some(bytes) => bytes,
        None => return -1,
    };

    blind_message.blind_message = leak_bytes(blind_bytes.clone());
    blind_message.blind_message_len = blind_bytes.len();
    secret_out.secret = leak_bytes(secret_bytes.clone());
    secret_out.secret_len = secret_bytes.len();
    0
}

fn rsassa_pss_verify(
    context: &BRSAContext,
    sig: &[u8],
    msg_randomizer: Option<&BRSAMessageRandomizer>,
    pk: &RsaPublicKey,
    msg: &[u8],
) -> i32 {
    let modulus_len = modulus_bytes_from_public(pk);
    if sig.len() != modulus_len {
        return -1;
    }

    let sig_value = BigUint::from_bytes_be(sig);
    if &sig_value >= pk.n() {
        return -1;
    }

    let em_value = sig_value.modpow(pk.e(), pk.n());
    let em = match biguint_to_padded_bytes(&em_value, modulus_len) {
        Some(bytes) => bytes,
        None => return -1,
    };
    let msg_hash = compute_message_hash(context, msg_randomizer, msg);

    emsa_pss_verify(context, &msg_hash, &em, pk.n().bits() - 1)
}

fn check_canonical(sk: &RsaPrivateKey, blind_message: &BRSABlindMessage) -> i32 {
    let pkey = sk.to_public_key();
    let modulus_len = modulus_bytes_from_public(&pkey);
    if blind_message.blind_message_len != modulus_len {
        return -1;
    }

    let modulus = match biguint_to_padded_bytes(pkey.n(), modulus_len) {
        Some(bytes) => bytes,
        None => return -1,
    };
    if blind_message.blind_message >= modulus.as_slice() {
        -1
    } else {
        0
    }
}

fn random_biguint_below(n: &BigUint) -> BigUint {
    let len = n.to_bytes_be().len().max(1);
    loop {
        let mut bytes = vec![0u8; len];
        OsRng.fill_bytes(&mut bytes);
        let candidate = BigUint::from_bytes_be(&bytes);
        if &candidate < n {
            return candidate;
        }
    }
}

fn gcd_biguint(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a
}

fn mod_inverse(a: &BigUint, n: &BigUint) -> Option<BigUint> {
    let mut t = BigInt::zero();
    let mut new_t = BigInt::one();
    let mut r = n.to_bigint()?;
    let mut new_r = a.to_bigint()?;

    while !new_r.is_zero() {
        let quotient = &r / &new_r;

        let tmp_t = &t - &quotient * &new_t;
        t = new_t;
        new_t = tmp_t;

        let tmp_r = &r - &quotient * &new_r;
        r = new_r;
        new_r = tmp_r;
    }

    if r != BigInt::one() {
        return None;
    }
    if t.is_negative() {
        t += n.to_bigint()?;
    }
    t.to_biguint()
}

fn biguint_to_padded_bytes(value: &BigUint, len: usize) -> Option<Vec<u8>> {
    let bytes = value.to_bytes_be();
    if bytes.len() > len {
        return None;
    }
    let mut out = vec![0u8; len];
    out[len - bytes.len()..].copy_from_slice(&bytes);
    Some(out)
}
