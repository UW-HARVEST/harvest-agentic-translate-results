use c_blind_rsa_signatures::blind_rsa;

use blind_rsa::{
    BRSABlindMessage, BRSABlindSignature, BRSABlindingSecret, BRSAContext, BRSAHashFunction,
    BRSAMessageRandomizer, BRSAPublicKey, BRSASecretKey, BRSASerializedKey, BRSASignature,
    BRSA_DEFAULT_SALT_LENGTH, MAX_HASH_DIGEST_LENGTH, MAX_MODULUS_BITS, MAX_SERIALIZED_PK_LEN,
    MIN_MODULUS_BITS,
};

// ---------- Constants ----------

#[test]
fn test_constants() {
    // C #define BRSA_DEFAULT_SALT_LENGTH ((size_t) -1)
    assert_eq!(BRSA_DEFAULT_SALT_LENGTH, usize::MAX);
    // C #define MIN_MODULUS_BITS 2048
    assert_eq!(MIN_MODULUS_BITS, 2048);
    // C #define MAX_MODULUS_BITS 4096
    assert_eq!(MAX_MODULUS_BITS, 4096);
    // C #define MAX_SERIALIZED_PK_LEN 1000
    assert_eq!(MAX_SERIALIZED_PK_LEN, 1000);
    // C #define MAX_HASH_DIGEST_LENGTH EVP_MAX_MD_SIZE -- which is 64
    assert_eq!(MAX_HASH_DIGEST_LENGTH, 64);
}

// ---------- BRSAContext ----------

#[test]
fn test_context_init_default_sets_sha384_salt_len() {
    // C ground truth: brsa_context_init_default produces salt_len = 48 (SHA384 digest size).
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();
    assert_eq!(ctx.salt_len, 48);
}

#[test]
fn test_context_init_deterministic_sets_zero_salt() {
    // C ground truth: brsa_context_init_deterministic produces salt_len = 0.
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();
    assert_eq!(ctx.salt_len, 0);
}

#[test]
fn test_context_init_custom_sha256_default_salt() {
    // C ground truth: SHA256 + BRSA_DEFAULT_SALT_LENGTH -> salt_len = 32, returns 0.
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(
        BRSAHashFunction::BRSA_SHA256,
        BRSA_DEFAULT_SALT_LENGTH,
    );
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 32);
}

#[test]
fn test_context_init_custom_sha384_default_salt() {
    // C ground truth: SHA384 + BRSA_DEFAULT_SALT_LENGTH -> salt_len = 48, returns 0.
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(
        BRSAHashFunction::BRSA_SHA384,
        BRSA_DEFAULT_SALT_LENGTH,
    );
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 48);
}

#[test]
fn test_context_init_custom_sha512_default_salt() {
    // C ground truth: SHA512 + BRSA_DEFAULT_SALT_LENGTH -> salt_len = 64, returns 0.
    let mut ctx = BRSAContext::new();
    let rc = ctx.brsa_context_init_custom(
        BRSAHashFunction::BRSA_SHA512,
        BRSA_DEFAULT_SALT_LENGTH,
    );
    assert_eq!(rc, 0);
    assert_eq!(ctx.salt_len, 64);
}

#[test]
fn test_context_init_custom_explicit_salt_lengths() {
    // C ground truth: explicit salt_len values are stored verbatim, return 0.
    for &n in &[0usize, 17usize, 48usize, 999usize] {
        let mut ctx = BRSAContext::new();
        let rc = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, n);
        assert_eq!(rc, 0);
        assert_eq!(ctx.salt_len, n);
    }
}

#[test]
fn test_context_init_custom_each_hash_explicit_salt_42() {
    let mut ctx = BRSAContext::new();
    assert_eq!(
        ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 42),
        0
    );
    assert_eq!(ctx.salt_len, 42);

    let mut ctx2 = BRSAContext::new();
    assert_eq!(
        ctx2.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, 42),
        0
    );
    assert_eq!(ctx2.salt_len, 42);

    let mut ctx3 = BRSAContext::new();
    assert_eq!(
        ctx3.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, 42),
        0
    );
    assert_eq!(ctx3.salt_len, 42);
}

#[test]
fn test_context_new_defaults() {
    let ctx = BRSAContext::new();
    assert_eq!(ctx.salt_len, 0);
    assert!(ctx.evp_md.is_none());
}

// ---------- BRSAPublicKey ----------

#[test]
fn test_public_key_new() {
    let pk = BRSAPublicKey::new();
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_public_key_import_too_large_returns_error() {
    // C ground truth: brsa_publickey_import with der_len > 1000 returns -1.
    let mut pk = BRSAPublicKey::new();
    let big = vec![0u8; 1100];
    let rc = pk.brsa_publickey_import(&big, 1001);
    assert_eq!(rc, -1);
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_public_key_import_invalid_data_returns_error() {
    // The C function tries to parse DER and fails on garbage; returns -1.
    let mut pk = BRSAPublicKey::new();
    let garbage = vec![0u8; 200];
    let rc = pk.brsa_publickey_import(&garbage, 200);
    assert_eq!(rc, -1);
}

#[test]
fn test_public_key_deinit_clears_fields() {
    let mut pk = BRSAPublicKey::new();
    pk.brsa_publickey_deinit();
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_public_key_recover_without_real_key_fails() {
    // The Rust placeholder cannot recover a real key; returns -1.
    let sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = pk.brsa_publickey_recover(&sk);
    assert_eq!(rc, -1);
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

// ---------- BRSASecretKey ----------

#[test]
fn test_secret_key_new() {
    let sk = BRSASecretKey::new();
    assert!(sk.evp_pkey.is_none());
}

#[test]
fn test_secret_key_keypair_generate_too_small_returns_error() {
    // Rust's brsa_keypair_generate validates modulus_bits < MIN_MODULUS_BITS -> -1.
    // C ground truth: kp 1024 -> -1.
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = sk.brsa_keypair_generate(&mut pk, 1024);
    assert_eq!(rc, -1);
    assert!(sk.evp_pkey.is_none());
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_secret_key_keypair_generate_zero_returns_error() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = sk.brsa_keypair_generate(&mut pk, 0);
    assert_eq!(rc, -1);
}

#[test]
fn test_secret_key_keypair_generate_negative_returns_error() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = sk.brsa_keypair_generate(&mut pk, -1);
    assert_eq!(rc, -1);
}

#[test]
fn test_secret_key_keypair_generate_too_large_returns_error() {
    // Rust's added range check: modulus_bits > MAX_MODULUS_BITS -> -1.
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = sk.brsa_keypair_generate(&mut pk, 8192);
    assert_eq!(rc, -1);
}

#[test]
fn test_secret_key_keypair_generate_in_range_still_fails_due_to_placeholder() {
    // Even with a valid range, the Rust placeholder cannot produce a real key.
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let rc = sk.brsa_keypair_generate(&mut pk, 2048);
    assert_eq!(rc, -1);
    assert!(sk.evp_pkey.is_none());
    assert!(pk.evp_pkey.is_none());
    assert!(pk.mont_ctx.is_none());
}

#[test]
fn test_secret_key_import_garbage_returns_error() {
    // C ground truth: importing garbage bytes -> -1.
    let mut sk = BRSASecretKey::new();
    let der = vec![0u8; 4];
    let rc = sk.brsa_secretkey_import(&der, 4);
    assert_eq!(rc, -1);
    assert!(sk.evp_pkey.is_none());
}

#[test]
fn test_secret_key_import_overflow_length_returns_error() {
    // The Rust validation rejects der_len > LONG_MAX. Use usize::MAX as input length.
    let mut sk = BRSASecretKey::new();
    let der: Vec<u8> = vec![];
    let rc = sk.brsa_secretkey_import(&der, usize::MAX);
    assert_eq!(rc, -1);
}

#[test]
fn test_secret_key_deinit_clears_field() {
    let mut sk = BRSASecretKey::new();
    sk.brsa_secretkey_deinit();
    assert!(sk.evp_pkey.is_none());
}

// ---------- BRSASerializedKey ----------

#[test]
fn test_serialized_key_new() {
    let sk = BRSASerializedKey::new();
    assert_eq!(sk.bytes_len, 0);
    assert!(sk.bytes.is_empty());
}

#[test]
fn test_serialized_key_secretkey_export_without_real_key_fails() {
    let sk = BRSASecretKey::new();
    let mut serialized = BRSASerializedKey::new();
    let rc = serialized.brsa_secretkey_export(&sk);
    assert_eq!(rc, -1);
    assert_eq!(serialized.bytes_len, 0);
    assert!(serialized.bytes.is_empty());
}

#[test]
fn test_serialized_key_publickey_export_without_real_key_fails() {
    let pk = BRSAPublicKey::new();
    let mut serialized = BRSASerializedKey::new();
    let rc = serialized.brsa_publickey_export(&pk);
    assert_eq!(rc, -1);
    assert_eq!(serialized.bytes_len, 0);
    assert!(serialized.bytes.is_empty());
}

#[test]
fn test_serialized_key_deinit_resets_fields() {
    let mut sk = BRSASerializedKey::new();
    sk.brsa_serializedkey_deinit();
    assert_eq!(sk.bytes_len, 0);
    assert!(sk.bytes.is_empty());
}

// ---------- BRSABlindMessage ----------

#[test]
fn test_blind_message_new() {
    let bm = BRSABlindMessage::new();
    assert_eq!(bm.blind_message_len, 0);
    assert!(bm.blind_message.is_empty());
}

#[test]
fn test_blind_message_init_records_length() {
    let mut bm = BRSABlindMessage::new();
    bm.brsa_blind_message_init(256);
    assert_eq!(bm.blind_message_len, 256);
    // The Rust translation keeps the slice empty even when an init length is set.
    assert!(bm.blind_message.is_empty());
}

#[test]
fn test_blind_message_deinit_resets_length() {
    let mut bm = BRSABlindMessage::new();
    bm.brsa_blind_message_init(256);
    bm.brsa_blind_message_deinit();
    assert_eq!(bm.blind_message_len, 0);
    assert!(bm.blind_message.is_empty());
}

// ---------- BRSABlindingSecret ----------

#[test]
fn test_blinding_secret_new() {
    let bs = BRSABlindingSecret::new();
    assert_eq!(bs.secret_len, 0);
    assert!(bs.secret.is_empty());
}

#[test]
fn test_blinding_secret_init_records_length() {
    let mut bs = BRSABlindingSecret::new();
    bs.brsa_blinding_secrete_init(256);
    assert_eq!(bs.secret_len, 256);
    assert!(bs.secret.is_empty());
}

#[test]
fn test_blinding_secret_deinit_resets_length() {
    let mut bs = BRSABlindingSecret::new();
    bs.brsa_blinding_secrete_init(256);
    bs.brsa_blinding_secret_deinit();
    assert_eq!(bs.secret_len, 0);
    assert!(bs.secret.is_empty());
}

// ---------- BRSABlindSignature ----------

#[test]
fn test_blind_signature_new() {
    let bs = BRSABlindSignature::new();
    assert_eq!(bs.blind_sig_len, 0);
    assert!(bs.blind_sig.is_empty());
}

#[test]
fn test_blind_signature_init_records_length() {
    let mut bs = BRSABlindSignature::new();
    bs.brsa_blind_signature_init(256);
    assert_eq!(bs.blind_sig_len, 256);
    assert!(bs.blind_sig.is_empty());
}

#[test]
fn test_blind_signature_deinit_resets_length() {
    let mut bs = BRSABlindSignature::new();
    bs.brsa_blind_signature_init(256);
    bs.brsa_blind_signature_deinit();
    assert_eq!(bs.blind_sig_len, 0);
    assert!(bs.blind_sig.is_empty());
}

// ---------- BRSASignature ----------

#[test]
fn test_signature_new() {
    let s = BRSASignature::new();
    assert_eq!(s.sig_len, 0);
    assert!(s.sig.is_empty());
}

#[test]
fn test_signature_init_records_length() {
    let mut s = BRSASignature::new();
    s.brsa_signature_init(256);
    assert_eq!(s.sig_len, 256);
    assert!(s.sig.is_empty());
}

#[test]
fn test_signature_deinit_resets_length() {
    let mut s = BRSASignature::new();
    s.brsa_signature_init(256);
    s.brsa_signature_deinit();
    assert_eq!(s.sig_len, 0);
    assert!(s.sig.is_empty());
}

// ---------- BRSAMessageRandomizer ----------

#[test]
fn test_message_randomizer_new_zero_initialized() {
    let r = BRSAMessageRandomizer::new();
    assert_eq!(r.noise.len(), 32);
    for &b in &r.noise {
        assert_eq!(b, 0u8);
    }
}

// ---------- BRSAContext crypto-API methods (all return -1 because of placeholder types) ----------

#[test]
fn test_brsa_blind_message_generate_returns_error() {
    let ctx = BRSAContext::new();
    let mut bm = BRSABlindMessage::new();
    let mut secret = BRSABlindingSecret::new();
    let mut pk = BRSAPublicKey::new();
    let mut msg = vec![0u8; 32];
    let rc = ctx.brsa_blind_message_generate(&mut bm, &mut msg, 32, &mut secret, &mut pk);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_blind_returns_error() {
    let ctx = BRSAContext::new();
    let mut bm = BRSABlindMessage::new();
    let mut secret = BRSABlindingSecret::new();
    let mut randomizer = BRSAMessageRandomizer::new();
    let mut pk = BRSAPublicKey::new();
    let msg = b"hello";
    let rc = ctx.brsa_blind(&mut bm, &mut secret, &mut randomizer, &mut pk, msg, msg.len());
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_blind_sign_returns_error() {
    let ctx = BRSAContext::new();
    let mut blind_sig = BRSABlindSignature::new();
    let mut sk = BRSASecretKey::new();
    let bm = BRSABlindMessage::new();
    let rc = ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &bm);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_finalize_returns_error() {
    let ctx = BRSAContext::new();
    let mut sig = BRSASignature::new();
    let blind_sig = BRSABlindSignature::new();
    let secret = BRSABlindingSecret::new();
    let randomizer: Option<BRSAMessageRandomizer> = None;
    let mut pk = BRSAPublicKey::new();
    let msg = b"hello";
    let rc = ctx.brsa_finalize(&mut sig, &blind_sig, &secret, &randomizer, &mut pk, msg, msg.len());
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_verify_returns_error() {
    let ctx = BRSAContext::new();
    let sig = BRSASignature::new();
    let mut pk = BRSAPublicKey::new();
    let randomizer: Option<BRSAMessageRandomizer> = None;
    let msg = b"hello";
    let rc = ctx.brsa_verify(&sig, &mut pk, &randomizer, msg, msg.len());
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_export_spki_returns_error() {
    let ctx = BRSAContext::new();
    let mut spki = BRSASerializedKey::new();
    let pk = BRSAPublicKey::new();
    let rc = ctx.brsa_publickey_export_spki(&mut spki, &pk);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_import_spki_too_short_returns_error() {
    // C ground truth: spki_len <= template_len (75) -> -1.
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let buf = vec![0u8; 75];
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 75);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_import_spki_too_large_returns_error() {
    // C ground truth: spki_len > 1000 -> -1.
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let buf = vec![0u8; 1100];
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 1001);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_import_spki_alg_len_too_large_returns_error() {
    // C ground truth: spki_len <= alg_len + 11 -> -1.
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let mut buf = vec![0u8; 100];
    buf[5] = 200; // alg_len = 200; 100 <= 200 + 11 -> -1
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 100);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_import_spki_just_above_template_returns_error() {
    // C ground truth: spki_len = 76 (just above template_len = 75) returns -1
    // because the algorithm template / DER content does not match.
    let ctx = BRSAContext::new();
    let mut pk = BRSAPublicKey::new();
    let buf = vec![0u8; 76];
    let rc = ctx.brsa_publickey_import_spki(&mut pk, &buf, 76);
    assert_eq!(rc, -1);
}

#[test]
fn test_brsa_publickey_id_returns_error() {
    // The Rust translation cannot compute SPKI/SHA-256 without a real key, so it returns -1.
    let ctx = BRSAContext::new();
    let pk = BRSAPublicKey::new();
    let id = vec![0u8; 4];
    let rc = ctx.brsa_publickey_id(&id, 4, &pk);
    assert_eq!(rc, -1);
}

// ---------- Free functions (module-level helpers) ----------

#[test]
fn test_bn_bn2bin_padded_with_none_returns_false_and_zeros_buffer() {
    use blind_rsa::BN_bn2bin_padded;
    let mut out = vec![0xFFu8; 8];
    let ok = BN_bn2bin_padded(&mut out, 4, None);
    assert!(!ok);
    // First 4 bytes were zeroed; remaining bytes left untouched.
    assert_eq!(&out[..4], &[0, 0, 0, 0]);
    assert_eq!(&out[4..], &[0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_bn_bn2bin_padded_zero_length_returns_false_no_zeroing() {
    use blind_rsa::BN_bn2bin_padded;
    let mut out = vec![0xAAu8; 4];
    let ok = BN_bn2bin_padded(&mut out, 0, None);
    assert!(!ok);
    assert_eq!(out, vec![0xAA, 0xAA, 0xAA, 0xAA]);
}

#[test]
fn test_bn_bn2bin_padded_len_greater_than_buffer_zeros_full_buffer() {
    use blind_rsa::BN_bn2bin_padded;
    let mut out = vec![0xFFu8; 4];
    let ok = BN_bn2bin_padded(&mut out, 100, None);
    assert!(!ok);
    assert_eq!(out, vec![0, 0, 0, 0]);
}

#[test]
fn test_rsa_bits_with_none_returns_zero() {
    use blind_rsa::_rsa_bits;
    assert_eq!(_rsa_bits(None), 0);
}

#[test]
fn test_rsa_size_with_none_returns_zero() {
    use blind_rsa::_rsa_size;
    assert_eq!(_rsa_size(None), 0);
}

#[test]
fn test_rsa_n_with_none_returns_none() {
    use blind_rsa::_rsa_n;
    assert!(_rsa_n(None).is_none());
}

#[test]
fn test_rsa_e_with_none_returns_none() {
    use blind_rsa::_rsa_e;
    assert!(_rsa_e(None).is_none());
}

#[test]
fn test_new_mont_domain_with_none_returns_none() {
    use blind_rsa::new_mont_domain;
    assert!(new_mont_domain(None).is_none());
}

#[test]
fn test_rsa_parameters_check_with_none_returns_error() {
    use blind_rsa::_rsa_parameters_check;
    // _rsa_bits(None) is 0, which is < MIN_MODULUS_BITS, so the function returns -1.
    assert_eq!(_rsa_parameters_check(None), -1);
}

#[test]
fn test_hash_with_empty_msg_hash_returns_error() {
    use blind_rsa::_hash;
    let randomizer = BRSAMessageRandomizer::new();
    let msg = b"some message";
    let rc = _hash(None, &randomizer, &[], msg);
    assert_eq!(rc, -1);
}

#[test]
fn test_hash_normal_inputs_returns_error() {
    use blind_rsa::_hash;
    let randomizer = BRSAMessageRandomizer::new();
    let buf = vec![0u8; 64];
    let msg = b"some message";
    let rc = _hash(None, &randomizer, &buf, msg);
    assert_eq!(rc, -1);
}

#[test]
fn test_blind_with_empty_padded_returns_error() {
    use blind_rsa::_blind;
    let bm = BRSABlindMessage::new();
    let secret = BRSABlindingSecret::new();
    let pk = BRSAPublicKey::new();
    let padded: &[u8] = &[];
    let rc = _blind(&bm, &secret, &pk, None, padded);
    assert_eq!(rc, -1);
}

#[test]
fn test_blind_with_mismatched_lengths_returns_error() {
    use blind_rsa::_blind;
    let mut bm = BRSABlindMessage::new();
    bm.brsa_blind_message_init(256);
    let secret = BRSABlindingSecret::new(); // secret_len = 0
    let pk = BRSAPublicKey::new();
    let padded = vec![0u8; 128]; // mismatched length compared to bm
    let rc = _blind(&bm, &secret, &pk, None, &padded);
    assert_eq!(rc, -1);
}

#[test]
fn test_blind_with_consistent_lengths_still_returns_error() {
    use blind_rsa::_blind;
    let mut bm = BRSABlindMessage::new();
    bm.brsa_blind_message_init(128);
    let mut secret = BRSABlindingSecret::new();
    secret.brsa_blinding_secrete_init(128);
    let pk = BRSAPublicKey::new();
    let padded = vec![0u8; 128];
    // Without a real key, the Rust translation cannot perform the blind operation.
    let rc = _blind(&bm, &secret, &pk, None, &padded);
    assert_eq!(rc, -1);
}

#[test]
fn test_check_cannonical_returns_error() {
    use blind_rsa::_check_cannonical;
    let sk = BRSASecretKey::new();
    let bm = BRSABlindMessage::new();
    let rc = _check_cannonical(&sk, &bm);
    assert_eq!(rc, -1);
}

#[test]
fn test_finalize_with_inconsistent_lengths_returns_error() {
    use blind_rsa::_finalize;
    let ctx = BRSAContext::new();
    let mut sig = BRSASignature::new();
    sig.brsa_signature_init(64);
    let mut blind_sig = BRSABlindSignature::new();
    blind_sig.brsa_blind_signature_init(128);
    let mut secret = BRSABlindingSecret::new();
    secret.brsa_blinding_secrete_init(64); // mismatch with blind_sig
    let randomizer = BRSAMessageRandomizer::new();
    let pk = BRSAPublicKey::new();
    let msg = b"some message";
    let rc = _finalize(&ctx, &sig, &blind_sig, &secret, &randomizer, &pk, None, msg);
    assert_eq!(rc, -1);
}

#[test]
fn test_finalize_with_consistent_lengths_still_returns_error() {
    use blind_rsa::_finalize;
    let ctx = BRSAContext::new();
    let mut sig = BRSASignature::new();
    sig.brsa_signature_init(128);
    let mut blind_sig = BRSABlindSignature::new();
    blind_sig.brsa_blind_signature_init(128);
    let mut secret = BRSABlindingSecret::new();
    secret.brsa_blinding_secrete_init(128);
    let randomizer = BRSAMessageRandomizer::new();
    let pk = BRSAPublicKey::new();
    let msg = b"some message";
    let rc = _finalize(&ctx, &sig, &blind_sig, &secret, &randomizer, &pk, None, msg);
    assert_eq!(rc, -1);
}

#[test]
fn test_finalize_with_zero_sig_length_and_consistent_lens_returns_error() {
    use blind_rsa::_finalize;
    // sig.sig_len == 0 path -> permitted, but ultimately still -1 due to placeholder.
    let ctx = BRSAContext::new();
    let sig = BRSASignature::new(); // sig_len = 0
    let mut blind_sig = BRSABlindSignature::new();
    blind_sig.brsa_blind_signature_init(128);
    let mut secret = BRSABlindingSecret::new();
    secret.brsa_blinding_secrete_init(128);
    let randomizer = BRSAMessageRandomizer::new();
    let pk = BRSAPublicKey::new();
    let msg = b"hi";
    let rc = _finalize(&ctx, &sig, &blind_sig, &secret, &randomizer, &pk, None, msg);
    assert_eq!(rc, -1);
}

fn main() {}
