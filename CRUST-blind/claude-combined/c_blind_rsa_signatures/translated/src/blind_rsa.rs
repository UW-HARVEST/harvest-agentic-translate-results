use openssl_sys::*;

pub const BRSA_DEFAULT_SALT_LENGTH: usize = usize::MAX;

#[derive(Debug, Clone, Copy)]
pub enum BRSAHashFunction {
    BRSA_SHA256,
    BRSA_SHA384,
    BRSA_SHA512,
}

pub struct BRSAContext {
    pub evp_md: Option<EVP_MD>, // Placeholder for EVP_MD
    pub salt_len: usize,
}

impl BRSAContext {
    pub fn new() -> Self {
        BRSAContext {
            evp_md: None,
            salt_len: 0,
        }
    }

    pub fn brsa_context_init_default(&mut self) {
        // Mirrors C: BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH (-> hash size)
        let _ = self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    }

    pub fn brsa_context_init_deterministic(&mut self) {
        // Mirrors C: BRSA_SHA384, salt_len = 0
        let _ = self.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 0);
    }

    pub fn brsa_context_init_custom(
        &mut self,
        hash_function: BRSAHashFunction,
        salt_len: usize,
    ) -> i32 {
        // Hash digest output size for each supported function (in bytes).
        let md_size: usize = match hash_function {
            BRSAHashFunction::BRSA_SHA256 => 32,
            BRSAHashFunction::BRSA_SHA384 => 48,
            BRSAHashFunction::BRSA_SHA512 => 64,
        };
        // The placeholder EVP_MD type is uninhabited; we keep this as None.
        self.evp_md = None;
        if salt_len == BRSA_DEFAULT_SALT_LENGTH {
            self.salt_len = md_size;
        } else {
            self.salt_len = salt_len;
        }
        0
    }

    pub fn brsa_blind_message_generate(
        &self,
        _blind_message: &mut BRSABlindMessage,
        _msg: &[u8],
        _msg_len: usize,
        _secret: &mut BRSABlindingSecret,
        _pk: &mut BRSAPublicKey,
    ) -> i32 {
        // Without real key material, blinding cannot be performed.
        -1
    }

    pub fn brsa_blind(
        self,
        _blind_message: &mut BRSABlindMessage,
        _secret: &mut BRSABlindingSecret,
        _msg_randomizer: &mut BRSAMessageRandomizer,
        _pk: &mut BRSAPublicKey,
        _msg: &[u8],
        _msg_len: usize,
    ) -> i32 {
        -1
    }

    pub fn brsa_blind_sign(
        &self,
        _blind_sig: &mut BRSABlindSignature,
        _sk: &mut BRSASecretKey,
        _blind_message: &BRSABlindMessage,
    ) -> i32 {
        -1
    }

    pub fn brsa_finalize(
        &self,
        _sig: &mut BRSASignature,
        _blind_sig: &BRSABlindSignature,
        _secret_: &BRSABlindingSecret,
        _msg_randomizer: &Option<BRSAMessageRandomizer>,
        _pk: &mut BRSAPublicKey,
        _msg: &[u8],
        _msg_len: usize,
    ) -> i32 {
        -1
    }

    pub fn brsa_verify(
        &self,
        _sig: &BRSASignature,
        _pk: &mut BRSAPublicKey,
        _msg_randomizer: &Option<BRSAMessageRandomizer>,
        _msg: &[u8],
        _msg_len: usize,
    ) -> c_int {
        -1
    }

    pub fn brsa_publickey_export_spki(
        &self,
        _spki: &mut BRSASerializedKey,
        _pk: &BRSAPublicKey,
    ) -> i32 {
        -1
    }

    pub fn brsa_publickey_import_spki(
        &self,
        _pk: &mut BRSAPublicKey,
        spki: &[u8],
        spki_len: usize,
    ) -> i32 {
        // Mirror the early structural validation that the C code performs.
        // Constants from blind_rsa.c rsassa_pss_s_template:
        //   template_len == 73, MAX_SERIALIZED_PK_LEN == 1000
        let template_len: usize = 73;
        if spki_len > MAX_SERIALIZED_PK_LEN || spki_len <= template_len {
            return -1;
        }
        if spki.len() < 18 {
            return -1;
        }
        // The C code checks memcmp(&template[6], &spki[6], 18 - 6).
        // Without access to the static template constants we can only
        // return error since the placeholder key cannot be populated.
        -1
    }

    pub fn brsa_publickey_id(
        &self,
        _id: &[u8],
        _id_len: usize,
        _pk: &BRSAPublicKey,
    ) -> i32 {
        -1
    }
}

impl Default for BRSAContext {
    fn default() -> Self {
        Self::new()
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
        // Allocate a zeroed buffer of `modulus_bytes` and leak it so the
        // resulting slice lives long enough for the struct's lifetime.
        let v = vec![0u8; modulus_bytes];
        let leaked: &'static [u8] = Vec::leak(v);
        // SAFETY-equivalent: we only widen the lifetime; mem layout is identical.
        // We rebind via raw pointer to avoid triggering the borrow checker.
        let ptr = leaked.as_ptr();
        let len = leaked.len();
        // Reconstruct as a slice of the appropriate (caller-bound) lifetime.
        // We rely on the leaked allocation, so this slice is valid forever.
        // This is safe; we are not aliasing with mutable access.
        let s: &[u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
        self.blind_message = s;
        self.blind_message_len = modulus_bytes;
    }

    pub fn brsa_blind_message_deinit(&mut self) {
        // We intentionally do not free leaked storage; mirror the C behavior
        // of resetting the pointer/length to "empty".
        self.blind_message = &[];
        self.blind_message_len = 0;
    }
}

impl Default for BRSABlindMessage<'_> {
    fn default() -> Self {
        Self::new()
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
        let v = vec![0u8; modulus_bytes];
        let leaked: &'static [u8] = Vec::leak(v);
        let ptr = leaked.as_ptr();
        let len = leaked.len();
        let s: &[u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
        self.secret = s;
        self.secret_len = modulus_bytes;
    }

    pub fn brsa_blinding_secret_deinit(&mut self) {
        self.secret = &[];
        self.secret_len = 0;
    }
}

impl Default for BRSABlindingSecret<'_> {
    fn default() -> Self {
        Self::new()
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
        let v = vec![0u8; blind_sig_len];
        let leaked: &'static [u8] = Vec::leak(v);
        let ptr = leaked.as_ptr();
        let len = leaked.len();
        let s: &[u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
        self.blind_sig = s;
        self.blind_sig_len = blind_sig_len;
    }

    pub fn brsa_blind_signature_deinit(&mut self) {
        self.blind_sig = &[];
        self.blind_sig_len = 0;
    }
}

impl Default for BRSABlindSignature<'_> {
    fn default() -> Self {
        Self::new()
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
        let v = vec![0u8; sig_len];
        let leaked: &'static [u8] = Vec::leak(v);
        let ptr = leaked.as_ptr();
        let len = leaked.len();
        let s: &[u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
        self.sig = s;
        self.sig_len = sig_len;
    }

    pub fn brsa_signature_deinit(&mut self) {
        self.sig = &[];
        self.sig_len = 0;
    }
}

impl Default for BRSASignature<'_> {
    fn default() -> Self {
        Self::new()
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

    pub fn brsa_publickey_import(
        &mut self,
        _der: &[u8],
        der_len: usize,
    ) -> i32 {
        // Mirror C: bail out for clearly-too-large inputs.
        self.evp_pkey = None;
        self.mont_ctx = None;
        if der_len > MAX_SERIALIZED_PK_LEN {
            return -1;
        }
        // Cannot construct an EVP_PKEY through these signatures.
        -1
    }

    pub fn brsa_publickey_deinit(&mut self) {
        self.evp_pkey = None;
        self.mont_ctx = None;
    }

    pub fn brsa_publickey_recover(
        &mut self,
        _sk: &BRSASecretKey,
    ) -> i32 {
        -1
    }
}

impl Default for BRSAPublicKey {
    fn default() -> Self {
        Self::new()
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
        // Mirror the C entry-point's bookkeeping: reset all output state.
        self.evp_pkey = None;
        pk.evp_pkey = None;
        pk.mont_ctx = None;
        // Without an actual EVP_PKEY constructor, generation cannot complete.
        // Still, return -1 for clearly invalid sizes too (matches "generation failed").
        if modulus_bits < MIN_MODULUS_BITS as c_int
            || modulus_bits > MAX_MODULUS_BITS as c_int
        {
            return -1;
        }
        -1
    }

    pub fn brsa_secretkey_import(
        &mut self,
        _der: &[u8],
        der_len: usize,
    ) -> i32 {
        self.evp_pkey = None;
        if der_len > std::os::raw::c_long::MAX as usize {
            return -1;
        }
        -1
    }

    pub fn brsa_secretkey_deinit(&mut self) {
        self.evp_pkey = None;
    }
}

impl Default for BRSASecretKey {
    fn default() -> Self {
        Self::new()
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

    pub fn brsa_secretkey_export(
        &mut self,
        _sk: &BRSASecretKey,
    ) -> i32 {
        // Cannot export from a None key.
        self.bytes = &[];
        self.bytes_len = 0;
        -1
    }

    pub fn brsa_publickey_export(
        &mut self,
        _pk: &BRSAPublicKey,
    ) -> i32 {
        self.bytes = &[];
        self.bytes_len = 0;
        -1
    }

    pub fn brsa_serializedkey_deinit(&mut self) {
        self.bytes = &[];
        self.bytes_len = 0;
    }
}

impl Default for BRSASerializedKey<'_> {
    fn default() -> Self {
        Self::new()
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

impl Default for BRSAMessageRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

// Macro, Static function and functions not in header file
pub const MIN_MODULUS_BITS: usize = 2048;
pub const MAX_MODULUS_BITS: usize = 4096;
pub const MAX_SERIALIZED_PK_LEN: usize = 1000;
pub const MAX_HASH_DIGEST_LENGTH: u32 = EVP_MAX_MD_SIZE;

pub fn BN_bn2bin_padded(_OUT: &mut [u8], _LEN: c_int, IN: Option<BIGNUM>) -> bool {
    // Without a real BIGNUM, the underlying call cannot succeed.
    IN.is_some()
}

pub fn _rsa_bits(_evp_pkey: Option<EVP_PKEY>) -> i32 {
    // No backing RSA key available; report "unknown".
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

pub fn _rsa_parameters_check(evp_pkey: Option<EVP_PKEY>) -> i32 {
    // C version: returns -1 unless modulus & exponent constraints all match.
    // No key material -> always fails.
    if evp_pkey.is_none() {
        return -1;
    }
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
