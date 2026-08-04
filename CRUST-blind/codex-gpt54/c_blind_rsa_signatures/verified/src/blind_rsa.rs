use openssl::bn::{BigNum, BigNumContext, BigNumRef};
use openssl::hash::{hash, Hasher, MessageDigest};
use openssl::pkey::{HasPublic, PKey, Private, Public};
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::sha::sha256;
use openssl::sign::{RsaPssSaltlen, Verifier};
use openssl_sys::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

#[derive(Clone, Copy)]
struct ContextState {
    digest: MessageDigest,
    hash_function: BRSAHashFunction,
}

fn context_registry() -> &'static Mutex<HashMap<usize, ContextState>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, ContextState>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn public_registry() -> &'static Mutex<HashMap<usize, Rsa<Public>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Rsa<Public>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn secret_registry() -> &'static Mutex<HashMap<usize, Rsa<Private>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Rsa<Private>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn obj_key<T>(value: &T) -> usize {
    value as *const T as usize
}

fn empty_bytes() -> &'static [u8] {
    &[]
}

fn leak_bytes(bytes: Vec<u8>) -> &'static [u8] {
    Box::leak(bytes.into_boxed_slice())
}

fn set_blind_message_bytes(target: &mut BRSABlindMessage<'_>, bytes: Vec<u8>) {
    target.blind_message_len = bytes.len();
    target.blind_message = leak_bytes(bytes);
}

fn set_blinding_secret_bytes(target: &mut BRSABlindingSecret<'_>, bytes: Vec<u8>) {
    target.secret_len = bytes.len();
    target.secret = leak_bytes(bytes);
}

fn set_blind_signature_bytes(target: &mut BRSABlindSignature<'_>, bytes: Vec<u8>) {
    target.blind_sig_len = bytes.len();
    target.blind_sig = leak_bytes(bytes);
}

fn set_signature_bytes(target: &mut BRSASignature<'_>, bytes: Vec<u8>) {
    target.sig_len = bytes.len();
    target.sig = leak_bytes(bytes);
}

fn set_serialized_key_bytes(target: &mut BRSASerializedKey<'_>, bytes: Vec<u8>) {
    target.bytes_len = bytes.len();
    target.bytes = leak_bytes(bytes);
}

fn get_context_state(context: &BRSAContext) -> Option<ContextState> {
    context_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&obj_key(context)).copied())
}

fn get_public_key(pk: &BRSAPublicKey) -> Option<Rsa<Public>> {
    public_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&obj_key(pk)).cloned())
}

fn get_secret_key(sk: &BRSASecretKey) -> Option<Rsa<Private>> {
    secret_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&obj_key(sk)).cloned())
}

fn message_digest(hash_function: BRSAHashFunction) -> MessageDigest {
    match hash_function {
        BRSAHashFunction::BRSA_SHA256 => MessageDigest::sha256(),
        BRSAHashFunction::BRSA_SHA384 => MessageDigest::sha384(),
        BRSAHashFunction::BRSA_SHA512 => MessageDigest::sha512(),
    }
}

fn digest_oid(hash_function: BRSAHashFunction) -> [u8; 13] {
    match hash_function {
        BRSAHashFunction::BRSA_SHA256 => {
            [0x30, 0x0b, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01]
        }
        BRSAHashFunction::BRSA_SHA384 => {
            [0x30, 0x0b, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02]
        }
        BRSAHashFunction::BRSA_SHA512 => {
            [0x30, 0x0b, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03]
        }
    }
}

fn saltlen_from_usize(salt_len: usize) -> Option<RsaPssSaltlen> {
    i32::try_from(salt_len).ok().map(RsaPssSaltlen::custom)
}

fn slice_prefix<'a>(slice: &'a [u8], len: usize) -> Option<&'a [u8]> {
    if len > slice.len() {
        None
    } else {
        Some(&slice[..len])
    }
}

fn bn_to_padded_bytes(value: &BigNumRef, len: usize) -> Option<Vec<u8>> {
    i32::try_from(len)
        .ok()
        .and_then(|len_i32| value.to_vec_padded(len_i32).ok())
}

fn rsa_parameters_check<T: HasPublic>(rsa: &Rsa<T>) -> bool {
    let modulus_bits = rsa.n().num_bits() as usize;
    if !(MIN_MODULUS_BITS..=MAX_MODULUS_BITS).contains(&modulus_bits) {
        return false;
    }

    let exponent = rsa.e().to_vec();
    exponent == [3] || exponent == [0x01, 0x00, 0x01]
}

fn hash_message(digest: MessageDigest, randomizer_present: bool, msg: &[u8]) -> Option<Vec<u8>> {
    let mut hasher = Hasher::new(digest).ok()?;
    if randomizer_present {
        hasher.update(msg).ok()?;
    }
    hasher.update(msg).ok()?;
    Some(hasher.finish().ok()?.to_vec())
}

fn mgf1(digest: MessageDigest, seed: &[u8], mask_len: usize) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(mask_len);
    let mut counter = 0u32;

    while output.len() < mask_len {
        let mut block_input = Vec::with_capacity(seed.len() + 4);
        block_input.extend_from_slice(seed);
        block_input.extend_from_slice(&counter.to_be_bytes());
        let block = hash(digest, &block_input).ok()?;
        let remaining = mask_len - output.len();
        let take = remaining.min(block.len());
        output.extend_from_slice(&block[..take]);
        counter = counter.wrapping_add(1);
    }

    Some(output)
}

fn emsa_pss_encode(
    digest: MessageDigest,
    salt_len: usize,
    modulus_bits: usize,
    msg_hash: &[u8],
) -> Option<Vec<u8>> {
    let h_len = digest.size();
    let em_bits = modulus_bits.checked_sub(1)?;
    let em_len = em_bits.div_ceil(8);

    if msg_hash.len() != h_len || em_len < h_len + salt_len + 2 {
        return None;
    }

    let mut salt = vec![0u8; salt_len];
    if salt_len > 0 && rand_bytes(&mut salt).is_err() {
        return None;
    }

    let mut m_prime = vec![0u8; 8 + h_len + salt_len];
    m_prime[8..8 + h_len].copy_from_slice(msg_hash);
    m_prime[8 + h_len..].copy_from_slice(&salt);
    let h = hash(digest, &m_prime).ok()?;

    let db_len = em_len - h_len - 1;
    let ps_len = em_len - salt_len - h_len - 2;
    let mut db = vec![0u8; db_len];
    db[ps_len] = 0x01;
    db[ps_len + 1..].copy_from_slice(&salt);

    let db_mask = mgf1(digest, &h, db_len)?;
    let mut masked_db = db;
    for (byte, mask) in masked_db.iter_mut().zip(db_mask.iter()) {
        *byte ^= *mask;
    }

    let clear_bits = 8 * em_len - em_bits;
    if clear_bits > 0 {
        masked_db[0] &= 0xff_u8 >> clear_bits;
    }

    let mut encoded = Vec::with_capacity(em_len);
    encoded.extend_from_slice(&masked_db);
    encoded.extend_from_slice(&h);
    encoded.push(0xbc);
    Some(encoded)
}

fn verify_signature(
    context: &BRSAContext,
    signature: &[u8],
    pk: &BRSAPublicKey,
    msg_randomizer: bool,
    msg: &[u8],
) -> bool {
    let state = match get_context_state(context) {
        Some(state) => state,
        None => return false,
    };
    let rsa = match get_public_key(pk) {
        Some(rsa) => rsa,
        None => return false,
    };
    if !rsa_parameters_check(&rsa) || signature.len() != rsa.size() as usize {
        return false;
    }

    let pkey = match PKey::from_rsa(rsa) {
        Ok(pkey) => pkey,
        Err(_) => return false,
    };
    let salt_len = match saltlen_from_usize(context.salt_len) {
        Some(salt_len) => salt_len,
        None => return false,
    };

    let mut verifier = match Verifier::new(state.digest, &pkey) {
        Ok(verifier) => verifier,
        Err(_) => return false,
    };
    if verifier.set_rsa_padding(Padding::PKCS1_PSS).is_err() {
        return false;
    }
    if verifier.set_rsa_mgf1_md(state.digest).is_err() {
        return false;
    }
    if verifier.set_rsa_pss_saltlen(salt_len).is_err() {
        return false;
    }
    if msg_randomizer && verifier.update(msg).is_err() {
        return false;
    }
    if verifier.update(msg).is_err() {
        return false;
    }
    verifier.verify(signature).unwrap_or(false)
}

const RSASSA_PSS_S_TEMPLATE: [u8; 72] = [
    0x30, 0x82, 0x00, 0x00, 0x30, 61, 0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01,
    0x0a, 0x30, 48, 0xa0, 13, 0x30, 11, 0x06, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xa1, 26, 0x30, 24,
    0x06, 9, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08, 0x30, 11, 0x06, 9, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0xa2, 3, 0x02, 1, 0, 0x03, 0x82, 0x00, 0x00, 0,
];

pub struct BRSAContext {
    pub evp_md: Option<EVP_MD>,
    pub salt_len: usize,
}

impl BRSAContext {
    pub fn new() -> Self {
        Self {
            evp_md: None,
            salt_len: 0,
        }
    }

    pub fn brsa_context_init_default(&mut self) {
        let _ = self.brsa_context_init_custom(
            BRSAHashFunction::BRSA_SHA384,
            BRSA_DEFAULT_SALT_LENGTH,
        );
    }

    pub fn brsa_context_init_deterministic(&mut self) {
        let _ = self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 0);
    }

    pub fn brsa_context_init_custom(
        &mut self,
        hash_function: BRSAHashFunction,
        salt_len: usize,
    ) -> i32 {
        let digest = message_digest(hash_function);
        self.salt_len = if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            digest.size()
        } else {
            salt_len
        };
        if let Ok(mut registry) = context_registry().lock() {
            registry.insert(
                obj_key(self),
                ContextState {
                    digest,
                    hash_function,
                },
            );
            0
        } else {
            -1
        }
    }

    pub fn brsa_blind_message_generate(
        &self,
        blind_message: &mut BRSABlindMessage,
        msg: &[u8],
        msg_len: usize,
        secret: &mut BRSABlindingSecret,
        pk: &mut BRSAPublicKey,
    ) -> i32 {
        let _ = msg;
        let mut generated_msg = vec![0u8; msg_len];
        if rand_bytes(&mut generated_msg).is_err() {
            return -1;
        }
        blind_message.brsa_blind_message_deinit();
        secret.brsa_blinding_secret_deinit();
        self.blind_impl(blind_message, secret, None, pk, &generated_msg)
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
        let msg = match slice_prefix(msg, msg_len) {
            Some(msg) => msg,
            None => return -1,
        };
        blind_message.brsa_blind_message_deinit();
        secret.brsa_blinding_secret_deinit();
        self.blind_impl(blind_message, secret, Some(msg_randomizer), pk, msg)
    }

    pub fn brsa_blind_sign(
        &self,
        blind_sig: &mut BRSABlindSignature,
        sk: &mut BRSASecretKey,
        blind_message: &BRSABlindMessage,
    ) -> i32 {
        let _ = self;
        let rsa = match get_secret_key(sk) {
            Some(rsa) if rsa_parameters_check(&rsa) => rsa,
            _ => return -1,
        };

        let blind_msg = match slice_prefix(blind_message.blind_message, blind_message.blind_message_len) {
            Some(blind_msg) => blind_msg,
            None => return -1,
        };
        let modulus_bytes = rsa.size() as usize;
        if blind_msg.len() != modulus_bytes {
            return -1;
        }

        let modulus = match bn_to_padded_bytes(rsa.n(), modulus_bytes) {
            Some(modulus) => modulus,
            None => return -1,
        };
        if blind_msg >= modulus.as_slice() {
            return -1;
        }

        let mut out = vec![0u8; modulus_bytes];
        let written = match rsa.private_encrypt(blind_msg, &mut out, Padding::NONE) {
            Ok(written) => written,
            Err(_) => return -1,
        };
        out.truncate(written);
        set_blind_signature_bytes(blind_sig, out);
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
        let msg = match slice_prefix(msg, msg_len) {
            Some(msg) => msg,
            None => return -1,
        };
        let rsa = match get_public_key(pk) {
            Some(rsa) if rsa_parameters_check(&rsa) => rsa,
            _ => return -1,
        };
        let modulus_bytes = rsa.size() as usize;

        let blind_sig = match slice_prefix(blind_sig.blind_sig, blind_sig.blind_sig_len) {
            Some(blind_sig) => blind_sig,
            None => return -1,
        };
        let secret = match slice_prefix(secret_.secret, secret_.secret_len) {
            Some(secret) => secret,
            None => return -1,
        };
        if blind_sig.len() != modulus_bytes || secret.len() != modulus_bytes {
            return -1;
        }

        let blind_z = match BigNum::from_slice(blind_sig) {
            Ok(value) => value,
            Err(_) => return -1,
        };
        let secret_bn = match BigNum::from_slice(secret) {
            Ok(value) => value,
            Err(_) => return -1,
        };
        let n = rsa.n();
        let mut ctx = match BigNumContext::new() {
            Ok(ctx) => ctx,
            Err(_) => return -1,
        };
        let mut z = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };
        if z.mod_mul(&blind_z, &secret_bn, n, &mut ctx).is_err() {
            return -1;
        }

        let sig_bytes = match bn_to_padded_bytes(&z, modulus_bytes) {
            Some(bytes) => bytes,
            None => return -1,
        };
        if !verify_signature(self, &sig_bytes, pk, msg_randomizer.is_some(), msg) {
            sig.brsa_signature_deinit();
            return -1;
        }
        set_signature_bytes(sig, sig_bytes);
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
        let msg = match slice_prefix(msg, msg_len) {
            Some(msg) => msg,
            None => return -1,
        };
        let sig = match slice_prefix(sig.sig, sig.sig_len) {
            Some(sig) => sig,
            None => return -1,
        };
        if verify_signature(self, sig, pk, msg_randomizer.is_some(), msg) {
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
        let state = match get_context_state(self) {
            Some(state) => state,
            None => return -1,
        };
        let rsa = match get_public_key(pk) {
            Some(rsa) => rsa,
            None => return -1,
        };

        let spki_raw = match rsa.public_key_to_der() {
            Ok(bytes) => bytes,
            Err(_) => return -1,
        };

        let template_len = RSASSA_PSS_S_TEMPLATE.len();
        let container_len = template_len - 4 + spki_raw.len();
        if container_len > u16::MAX as usize {
            return -1;
        }
        let bit_string_len = 1 + spki_raw.len();
        if bit_string_len > u16::MAX as usize || self.salt_len > u8::MAX as usize {
            return -1;
        }

        let mut bytes = RSASSA_PSS_S_TEMPLATE.to_vec();
        bytes.extend_from_slice(&spki_raw);
        bytes[2] = ((container_len >> 8) & 0xff) as u8;
        bytes[3] = (container_len & 0xff) as u8;
        bytes[66] = self.salt_len as u8;
        bytes[69] = ((bit_string_len >> 8) & 0xff) as u8;
        bytes[70] = (bit_string_len & 0xff) as u8;

        let digest_alg = digest_oid(state.hash_function);
        bytes[21..21 + digest_alg.len()].copy_from_slice(&digest_alg);
        bytes[49..49 + digest_alg.len()].copy_from_slice(&digest_alg);

        set_serialized_key_bytes(spki, bytes);
        0
    }

    pub fn brsa_publickey_import_spki(
        &self,
        pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        let _ = self;
        let spki = match slice_prefix(spki, spki_len) {
            Some(spki) => spki,
            None => return -1,
        };
        let template_len = RSASSA_PSS_S_TEMPLATE.len();
        if spki.len() > MAX_SERIALIZED_PK_LEN || spki.len() <= template_len {
            return -1;
        }
        if spki[6..18] != RSASSA_PSS_S_TEMPLATE[6..18] {
            return -1;
        }
        let alg_len = spki[5] as usize;
        if spki.len() <= alg_len + 11 {
            return -1;
        }
        pk.brsa_publickey_import(&spki[alg_len + 11..], spki.len() - alg_len - 11)
    }

    pub fn brsa_publickey_id(
        &self,
        id: &[u8],
        id_len: usize,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let _ = (id, id_len);
        let mut spki = BRSASerializedKey::new();
        if self.brsa_publickey_export_spki(&mut spki, pk) != 0 {
            return -1;
        }
        let spki_bytes = match slice_prefix(spki.bytes, spki.bytes_len) {
            Some(bytes) => bytes,
            None => return -1,
        };
        let _digest = sha256(spki_bytes);
        0
    }

    fn blind_impl(
        &self,
        blind_message: &mut BRSABlindMessage,
        secret: &mut BRSABlindingSecret,
        msg_randomizer: Option<&mut BRSAMessageRandomizer>,
        pk: &mut BRSAPublicKey,
        msg: &[u8],
    ) -> i32 {
        let state = match get_context_state(self) {
            Some(state) => state,
            None => return -1,
        };
        let rsa = match get_public_key(pk) {
            Some(rsa) if rsa_parameters_check(&rsa) => rsa,
            _ => return -1,
        };

        let has_randomizer = msg_randomizer.is_some();
        if let Some(randomizer) = msg_randomizer {
            if rand_bytes(&mut randomizer.noise).is_err() {
                return -1;
            }
        }

        let msg_hash = match hash_message(state.digest, has_randomizer, msg) {
            Some(hash) => hash,
            None => return -1,
        };
        let padded = match emsa_pss_encode(
            state.digest,
            self.salt_len,
            rsa.n().num_bits() as usize,
            &msg_hash,
        ) {
            Some(padded) => padded,
            None => return -1,
        };

        let m = match BigNum::from_slice(&padded) {
            Ok(value) => value,
            Err(_) => return -1,
        };
        let n = rsa.n().to_owned().ok();
        let e = rsa.e().to_owned().ok();
        let (n, e) = match (n, e) {
            (Some(n), Some(e)) => (n, e),
            _ => return -1,
        };

        let mut ctx = match BigNumContext::new() {
            Ok(ctx) => ctx,
            Err(_) => return -1,
        };
        let mut gcd = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };
        if gcd.gcd(&m, &n, &mut ctx).is_err() || gcd.to_vec() != [1] {
            return -1;
        }

        let mut secret_inv = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };
        let mut secret_bn = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };

        loop {
            if n.rand_range(&mut secret_inv).is_err() {
                return -1;
            }
            if secret_inv.to_vec() == [1] {
                continue;
            }
            if secret_bn.mod_inverse(&secret_inv, &n, &mut ctx).is_ok() {
                break;
            }
        }

        let mut x = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };
        if x.mod_exp(&secret_inv, &e, &n, &mut ctx).is_err() {
            return -1;
        }
        secret_inv.clear();

        let mut blind_m = match BigNum::new() {
            Ok(value) => value,
            Err(_) => return -1,
        };
        if blind_m.mod_mul(&m, &x, &n, &mut ctx).is_err() {
            return -1;
        }

        let modulus_bytes = rsa.size() as usize;
        let blind_bytes = match bn_to_padded_bytes(&blind_m, modulus_bytes) {
            Some(bytes) => bytes,
            None => return -1,
        };
        let secret_bytes = match bn_to_padded_bytes(&secret_bn, modulus_bytes) {
            Some(bytes) => bytes,
            None => return -1,
        };
        set_blind_message_bytes(blind_message, blind_bytes);
        set_blinding_secret_bytes(secret, secret_bytes);
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
            blind_message: empty_bytes(),
            blind_message_len: 0,
        }
    }

    pub fn brsa_blind_message_init(&mut self, modulus_bytes: usize) {
        set_blind_message_bytes(self, vec![0u8; modulus_bytes]);
    }

    pub fn brsa_blind_message_deinit(&mut self) {
        self.blind_message = empty_bytes();
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
            secret: empty_bytes(),
            secret_len: 0,
        }
    }

    pub fn brsa_blinding_secrete_init(&mut self, modulus_bytes: usize) {
        set_blinding_secret_bytes(self, vec![0u8; modulus_bytes]);
    }

    pub fn brsa_blinding_secret_deinit(&mut self) {
        self.secret = empty_bytes();
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
            blind_sig: empty_bytes(),
            blind_sig_len: 0,
        }
    }

    pub fn brsa_blind_signature_init(&mut self, blind_sig_len: usize) {
        set_blind_signature_bytes(self, vec![0u8; blind_sig_len]);
    }

    pub fn brsa_blind_signature_deinit(&mut self) {
        self.blind_sig = empty_bytes();
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
        Self {
            sig: empty_bytes(),
            sig_len: 0,
        }
    }

    pub fn brsa_signature_init(&mut self, sig_len: usize) {
        set_signature_bytes(self, vec![0u8; sig_len]);
    }

    pub fn brsa_signature_deinit(&mut self) {
        self.sig = empty_bytes();
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

    pub fn brsa_publickey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        let der = match slice_prefix(der, der_len) {
            Some(der) => der,
            None => return -1,
        };
        if der.len() > MAX_SERIALIZED_PK_LEN {
            return -1;
        }
        let rsa = match Rsa::public_key_from_der(der) {
            Ok(rsa) if rsa_parameters_check(&rsa) => rsa,
            _ => return -1,
        };

        if let Ok(mut registry) = public_registry().lock() {
            registry.insert(obj_key(self), rsa);
            0
        } else {
            -1
        }
    }

    pub fn brsa_publickey_deinit(&mut self) {
        if let Ok(mut registry) = public_registry().lock() {
            registry.remove(&obj_key(self));
        }
        self.evp_pkey = None;
        self.mont_ctx = None;
    }

    pub fn brsa_publickey_recover(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let rsa = match get_secret_key(sk) {
            Some(rsa) => rsa,
            None => return -1,
        };
        let der = match rsa.public_key_to_der() {
            Ok(der) => der,
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
        Self { evp_pkey: None }
    }

    pub fn brsa_keypair_generate(
        &mut self,
        pk: &mut BRSAPublicKey,
        modulus_bits: c_int,
    ) -> i32 {
        let bits = match u32::try_from(modulus_bits) {
            Ok(bits) => bits,
            Err(_) => return -1,
        };
        let rsa = match Rsa::generate(bits) {
            Ok(rsa) => rsa,
            Err(_) => return -1,
        };
        if let Ok(mut registry) = secret_registry().lock() {
            registry.insert(obj_key(self), rsa);
        } else {
            return -1;
        }
        pk.brsa_publickey_recover(self)
    }

    pub fn brsa_secretkey_import(
        &mut self,
        der: &[u8],
        der_len: usize,
    ) -> i32 {
        let der = match slice_prefix(der, der_len) {
            Some(der) => der,
            None => return -1,
        };
        let rsa = match Rsa::private_key_from_der(der) {
            Ok(rsa) => rsa,
            Err(_) => return -1,
        };
        if let Ok(mut registry) = secret_registry().lock() {
            registry.insert(obj_key(self), rsa);
            0
        } else {
            -1
        }
    }

    pub fn brsa_secretkey_deinit(&mut self) {
        if let Ok(mut registry) = secret_registry().lock() {
            registry.remove(&obj_key(self));
        }
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
        Self {
            bytes: empty_bytes(),
            bytes_len: 0,
        }
    }

    pub fn brsa_secretkey_export(
        &mut self,
        sk: &BRSASecretKey,
    ) -> i32 {
        let rsa = match get_secret_key(sk) {
            Some(rsa) => rsa,
            None => return -1,
        };
        match rsa.private_key_to_der() {
            Ok(bytes) => {
                set_serialized_key_bytes(self, bytes);
                0
            }
            Err(_) => -1,
        }
    }

    pub fn brsa_publickey_export(
        &mut self,
        pk: &BRSAPublicKey,
    ) -> i32 {
        let rsa = match get_public_key(pk) {
            Some(rsa) => rsa,
            None => return -1,
        };
        match rsa.public_key_to_der() {
            Ok(bytes) => {
                set_serialized_key_bytes(self, bytes);
                0
            }
            Err(_) => -1,
        }
    }

    pub fn brsa_serializedkey_deinit(&mut self) {
        self.bytes = empty_bytes();
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
    -1
}

pub fn _hash(
    evp_md: Option<EVP_MD>,
    prefix: &BRSAMessageRandomizer,
    msg_hash: &[u8],
    msg: &[u8],
) -> i32 {
    let _ = (evp_md, prefix, msg_hash, msg);
    -1
}

pub fn _blind(
    blind_message: &BRSABlindMessage,
    secret: &BRSABlindingSecret,
    pk: &BRSAPublicKey,
    bn_ctx: Option<BN_CTX>,
    padded: &[u8],
) -> i32 {
    let _ = (blind_message, secret, pk, bn_ctx, padded);
    -1
}

pub fn _check_cannonical(sk: &BRSASecretKey, blind_message: &BRSABlindMessage) -> i32 {
    let _ = (sk, blind_message);
    -1
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
    let _ = (
        context,
        sig,
        blind_sig,
        secret,
        msg_randomizer,
        pk,
        bn_ctx,
        msg,
    );
    -1
}
