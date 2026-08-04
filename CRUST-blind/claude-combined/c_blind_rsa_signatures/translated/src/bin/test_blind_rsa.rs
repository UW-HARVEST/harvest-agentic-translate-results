use c_blind_rsa_signatures::blind_rsa;
use blind_rsa::*;

#[test]
fn test_context_init_default() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();
    // C: brsa_context_init_default uses BRSA_SHA384 + BRSA_DEFAULT_SALT_LENGTH,
    //    which becomes salt_len = EVP_MD_size(SHA384) = 48
    assert_eq!(ctx.salt_len, 48);
}

#[test]
fn test_context_init_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();
    // C: deterministic init explicitly sets salt_len = 0
    assert_eq!(ctx.salt_len, 0);
}

#[test]
fn test_context_init_custom_sha256_default_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 32); // SHA-256 digest size
}

#[test]
fn test_context_init_custom_sha384_default_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 48); // SHA-384 digest size
}

#[test]
fn test_context_init_custom_sha512_default_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 64); // SHA-512 digest size
}

#[test]
fn test_context_init_custom_sha256_zero_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 0);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 0);
}

#[test]
fn test_context_init_custom_sha384_explicit_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 13);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 13);
}

#[test]
fn test_context_init_custom_sha512_explicit_salt() {
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, 99);
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 99);
}

#[test]
fn test_context_new_starts_clean() {
    let ctx = BRSAContext::new();
    assert_eq!(ctx.salt_len, 0);
    assert!(ctx.evp_md.is_none());
}

#[test]
fn test_blind_message_init_and_deinit() {
    let mut bm = BRSABlindMessage::new();
    assert_eq!(bm.blind_message_len, 0);
    assert!(bm.blind_message.is_empty());

    bm.brsa_blind_message_init(256);
    assert_eq!(bm.blind_message_len, 256);
    assert_eq!(bm.blind_message.len(), 256);
    // Buffer should be zero-initialised (matches OPENSSL_malloc + later overwrite).
    for &b in bm.blind_message {
        assert_eq!(b, 0);
    }

    bm.brsa_blind_message_deinit();
    assert_eq!(bm.blind_message_len, 0);
    assert_eq!(bm.blind_message.len(), 0);
}

#[test]
fn test_blinding_secret_init_and_deinit() {
    let mut bs = BRSABlindingSecret::new();
    assert_eq!(bs.secret_len, 0);
    assert!(bs.secret.is_empty());

    bs.brsa_blinding_secrete_init(256);
    assert_eq!(bs.secret_len, 256);
    assert_eq!(bs.secret.len(), 256);

    bs.brsa_blinding_secret_deinit();
    assert_eq!(bs.secret_len, 0);
    assert_eq!(bs.secret.len(), 0);
}

#[test]
fn test_blind_signature_init_and_deinit() {
    let mut bsig = BRSABlindSignature::new();
    assert_eq!(bsig.blind_sig_len, 0);
    assert!(bsig.blind_sig.is_empty());

    bsig.brsa_blind_signature_init(256);
    assert_eq!(bsig.blind_sig_len, 256);
    assert_eq!(bsig.blind_sig.len(), 256);

    bsig.brsa_blind_signature_deinit();
    assert_eq!(bsig.blind_sig_len, 0);
    assert_eq!(bsig.blind_sig.len(), 0);
}

#[test]
fn test_signature_init_and_deinit() {
    let mut sig = BRSASignature::new();
    assert_eq!(sig.sig_len, 0);
    assert!(sig.sig.is_empty());

    sig.brsa_signature_init(512);
    assert_eq!(sig.sig_len, 512);
    assert_eq!(sig.sig.len(), 512);

    sig.brsa_signature_deinit();
    assert_eq!(sig.sig_len, 0);
    assert_eq!(sig.sig.len(), 0);
}

#[test]
fn test_publickey_new_and_deinit() {
    let mut pk = BRSAPublicKey::new();
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());

    pk.brsa_publickey_deinit();
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_publickey_import_too_large_returns_minus_one() {
    let mut pk = BRSAPublicKey::new();
    // C: > MAX_SERIALIZED_PK_LEN (1000) returns -1 immediately.
    let rc = pk.brsa_publickey_import(&[0u8; 8], 1001);
    assert_eq!(rc, -1);
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_publickey_import_bad_data_returns_minus_one() {
    let mut pk = BRSAPublicKey::new();
    // C: garbage bytes -> d2i_PublicKey fails -> -1.
    let rc = pk.brsa_publickey_import(&[0u8; 4], 4);
    assert_eq!(rc, -1);
}

#[test]
fn test_publickey_recover_no_secret_key() {
    let mut pk = BRSAPublicKey::new();
    let sk = BRSASecretKey::new();
    // C: an empty SK has evp_pkey=NULL, so i2d_PublicKey fails -> -1.
    let rc = pk.brsa_publickey_recover(&sk);
    assert_eq!(rc, -1);
}

#[test]
fn test_secretkey_new_and_deinit() {
    let mut sk = BRSASecretKey::new();
    assert!(sk.evp_pkey.is_none());

    sk.brsa_secretkey_deinit();
    assert!(sk.evp_pkey.is_none());
}

#[test]
fn test_secretkey_import_bad_data() {
    let mut sk = BRSASecretKey::new();
    let rc = sk.brsa_secretkey_import(&[0u8; 4], 4);
    assert_eq!(rc, -1);
    assert!(sk.evp_pkey.is_none());
}

#[test]
fn test_keypair_generate_invalid_size() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    // 1024 bits is below MIN_MODULUS_BITS (2048) - both C and Rust fail.
    let rc = sk.brsa_keypair_generate(&mut pk, 1024);
    assert_eq!(rc, -1);
}

#[test]
fn test_serialized_key_new_and_deinit() {
    let mut sk_ser = BRSASerializedKey::new();
    assert_eq!(sk_ser.bytes_len, 0);
    assert!(sk_ser.bytes.is_empty());

    sk_ser.brsa_serializedkey_deinit();
    assert_eq!(sk_ser.bytes_len, 0);
    assert!(sk_ser.bytes.is_empty());
}

#[test]
fn test_serialized_key_export_with_no_keys() {
    let sk = BRSASecretKey::new();
    let pk = BRSAPublicKey::new();
    let mut sk_ser = BRSASerializedKey::new();

    let rc1 = sk_ser.brsa_secretkey_export(&sk);
    assert_eq!(rc1, -1);
    assert_eq!(sk_ser.bytes_len, 0);

    let mut pk_ser = BRSASerializedKey::new();
    let rc2 = pk_ser.brsa_publickey_export(&pk);
    assert_eq!(rc2, -1);
    assert_eq!(pk_ser.bytes_len, 0);
}

#[test]
fn test_message_randomizer_new() {
    let r = BRSAMessageRandomizer::new();
    for &b in &r.noise {
        assert_eq!(b, 0);
    }
    assert_eq!(r.noise.len(), 32);
}

#[test]
fn test_constants() {
    assert_eq!(MIN_MODULUS_BITS, 2048);
    assert_eq!(MAX_MODULUS_BITS, 4096);
    assert_eq!(MAX_SERIALIZED_PK_LEN, 1000);
    assert_eq!(MAX_HASH_DIGEST_LENGTH, 64); // EVP_MAX_MD_SIZE
    assert_eq!(BRSA_DEFAULT_SALT_LENGTH, usize::MAX);
}

#[test]
fn test_blind_message_generate_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut bm = BRSABlindMessage::new();
    let mut secret = BRSABlindingSecret::new();
    let mut pk = BRSAPublicKey::new();
    let mut msg = [0u8; 32];

    // Without a key, this fails (matches C: _rsa_parameters_check fails on NULL key).
    let rc = ctx.brsa_blind_message_generate(&mut bm, &mut msg, 32, &mut secret, &mut pk);
    assert_eq!(rc, -1);
}

#[test]
fn test_blind_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut bm = BRSABlindMessage::new();
    let mut secret = BRSABlindingSecret::new();
    let mut rand = BRSAMessageRandomizer::new();
    let mut pk = BRSAPublicKey::new();
    let msg = [0u8; 32];

    let rc = ctx.brsa_blind(&mut bm, &mut secret, &mut rand, &mut pk, &msg, 32);
    assert_eq!(rc, -1);
}

#[test]
fn test_blind_sign_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut bsig = BRSABlindSignature::new();
    let mut sk = BRSASecretKey::new();
    let bm = BRSABlindMessage::new();

    let rc = ctx.brsa_blind_sign(&mut bsig, &mut sk, &bm);
    assert_eq!(rc, -1);
}

#[test]
fn test_finalize_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sig = BRSASignature::new();
    let bsig = BRSABlindSignature::new();
    let secret = BRSABlindingSecret::new();
    let randomizer: Option<BRSAMessageRandomizer> = None;
    let mut pk = BRSAPublicKey::new();
    let msg = [0u8; 32];

    let rc = ctx.brsa_finalize(&mut sig, &bsig, &secret, &randomizer, &mut pk, &msg, 32);
    assert_eq!(rc, -1);
}

#[test]
fn test_verify_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let sig = BRSASignature::new();
    let mut pk = BRSAPublicKey::new();
    let randomizer: Option<BRSAMessageRandomizer> = None;
    let msg = [0u8; 32];

    let rc = ctx.brsa_verify(&sig, &mut pk, &randomizer, &msg, 32);
    assert_eq!(rc, -1);
}

#[test]
fn test_publickey_export_spki_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut spki = BRSASerializedKey::new();
    let pk = BRSAPublicKey::new();

    let rc = ctx.brsa_publickey_export_spki(&mut spki, &pk);
    assert_eq!(rc, -1);
}

#[test]
fn test_publickey_import_spki_too_large() {
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let buf = [0u8; 1001];
    // C: > MAX_SERIALIZED_PK_LEN -> -1
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 1001);
    assert_eq!(rc, -1);
}

#[test]
fn test_publickey_import_spki_too_small() {
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let buf = [0u8; 10];
    // C: <= template_len (73) -> -1
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 10);
    assert_eq!(rc, -1);
}

#[test]
fn test_publickey_id_no_key() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let id = [0u8; 4];
    let pk = BRSAPublicKey::new();

    // C: _publickey_id calls export_spki, which fails on a NULL key -> -1.
    let rc = ctx.brsa_publickey_id(&id, 4, &pk);
    assert_eq!(rc, -1);
}

#[test]
fn test_helper_rsa_bits_no_key() {
    // _rsa_bits with NULL EVP_PKEY in C would crash; in Rust we represent
    // the absence of a key explicitly with None.
    assert_eq!(_rsa_bits(None), 0);
}

#[test]
fn test_helper_rsa_size_no_key() {
    assert_eq!(_rsa_size(None), 0);
}

#[test]
fn test_helper_rsa_n_no_key() {
    assert!(_rsa_n(None).is_none());
}

#[test]
fn test_helper_rsa_e_no_key() {
    assert!(_rsa_e(None).is_none());
}

#[test]
fn test_helper_new_mont_domain_no_n() {
    assert!(new_mont_domain(None).is_none());
}

#[test]
fn test_helper_rsa_parameters_check_none() {
    // C aborts on NULL; in Rust we model that as -1 (no key, can't validate).
    assert_eq!(_rsa_parameters_check(None), -1);
}

#[test]
fn test_helper_hash_no_md() {
    let r = BRSAMessageRandomizer::new();
    let buf = [0u8; 64];
    let msg = [0u8; 4];
    // No EVP_MD -> error.
    assert_eq!(_hash(None, &r, &buf, &msg), -1);
}

#[test]
fn test_helper_blind_no_key() {
    let bm = BRSABlindMessage::new();
    let bs = BRSABlindingSecret::new();
    let pk = BRSAPublicKey::new();
    let padded = [0u8; 4];
    assert_eq!(_blind(&bm, &bs, &pk, None, &padded), -1);
}

#[test]
fn test_helper_check_canonical_no_key() {
    let sk = BRSASecretKey::new();
    let bm = BRSABlindMessage::new();
    assert_eq!(_check_cannonical(&sk, &bm), -1);
}

#[test]
fn test_helper_finalize_no_key() {
    let ctx = BRSAContext::new();
    let sig = BRSASignature::new();
    let bsig = BRSABlindSignature::new();
    let secret = BRSABlindingSecret::new();
    let r = BRSAMessageRandomizer::new();
    let pk = BRSAPublicKey::new();
    let msg = [0u8; 4];
    assert_eq!(_finalize(&ctx, &sig, &bsig, &secret, &r, &pk, None, &msg), -1);
}

#[test]
fn test_helper_bn_bn2bin_padded_none() {
    let mut buf = [0u8; 8];
    // No BIGNUM -> no work to do; C macro would crash.
    assert!(!BN_bn2bin_padded(&mut buf, 8, None));
}

fn main() {}
