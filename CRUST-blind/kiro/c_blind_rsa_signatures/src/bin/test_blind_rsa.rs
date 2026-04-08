use c_blind_rsa_signatures::blind_rsa::*;

// ---- Context initialization tests ----

#[test]
fn test_context_init_default() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();
    // Default uses SHA384 with salt_len = hash output size (48)
    assert_eq!(ctx.salt(), 48);
}

#[test]
fn test_context_init_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();
    // Deterministic uses SHA384 with salt_len = 0
    assert_eq!(ctx.salt(), 0);
}

#[test]
fn test_context_init_custom_sha256() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 48);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt(), 48);
}

#[test]
fn test_context_init_custom_sha512() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, 32);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt(), 32);
}

#[test]
fn test_context_init_custom_default_salt() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt(), 32);
}

#[test]
fn test_context_init_custom_sha384_default_salt() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt(), 48);
}

#[test]
fn test_context_init_custom_sha512_default_salt() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt(), 64);
}

// ---- Key generation tests ----

#[test]
fn test_keypair_generate_2048() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

#[test]
fn test_keypair_generate_4096() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 4096), 0);
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// ---- Key export/import round-trip tests ----

#[test]
fn test_secretkey_export_import() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut sk_der = BRSASerializedKey::new();
    assert_eq!(sk_der.brsa_secretkey_export(&sk), 0);
    assert!(sk_der.bytes_len > 0);

    let mut sk2 = BRSASecretKey::new();
    assert_eq!(sk2.brsa_secretkey_import(sk_der.bytes, sk_der.bytes_len), 0);

    sk_der.brsa_serializedkey_deinit();
    sk.brsa_secretkey_deinit();
    sk2.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

#[test]
fn test_publickey_export_import() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut pk_der = BRSASerializedKey::new();
    assert_eq!(pk_der.brsa_publickey_export(&pk), 0);
    assert!(pk_der.bytes_len > 0);

    let mut pk2 = BRSAPublicKey::new();
    assert_eq!(pk2.brsa_publickey_import(pk_der.bytes, pk_der.bytes_len), 0);

    pk_der.brsa_serializedkey_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
    pk2.brsa_publickey_deinit();
}

#[test]
fn test_publickey_recover() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut pk2 = BRSAPublicKey::new();
    assert_eq!(pk2.brsa_publickey_recover(&sk), 0);

    let mut der1 = BRSASerializedKey::new();
    let mut der2 = BRSASerializedKey::new();
    assert_eq!(der1.brsa_publickey_export(&pk), 0);
    assert_eq!(der2.brsa_publickey_export(&pk2), 0);
    assert_eq!(der1.bytes_len, der2.bytes_len);
    assert_eq!(der1.bytes, der2.bytes);

    der1.brsa_serializedkey_deinit();
    der2.brsa_serializedkey_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
    pk2.brsa_publickey_deinit();
}

// ---- Full blind signature flow (mirrors C test_default) ----

#[test]
fn test_blind_sign_flow_default() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(
        ctx.brsa_blind_message_generate(&mut blind_msg, &mut msg, 32, &mut client_secret, &mut pk),
        0
    );
    assert!(blind_msg.blind_message_len > 0);
    assert!(client_secret.secret_len > 0);

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    let mut sig = BRSASignature::new();
    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    assert_eq!(
        ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32),
        0
    );
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);
    sig.brsa_signature_deinit();

    // Key ID
    let key_id = [0u8; 4];
    assert_eq!(ctx.brsa_publickey_id(&key_id, 4, &pk), 0);

    // Key serialization round-trip
    let mut sk_der = BRSASerializedKey::new();
    let mut pk_der = BRSASerializedKey::new();
    assert_eq!(sk_der.brsa_secretkey_export(&sk), 0);
    assert_eq!(pk_der.brsa_publickey_export(&pk), 0);

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();

    let mut sk2 = BRSASecretKey::new();
    let mut pk2 = BRSAPublicKey::new();
    assert_eq!(sk2.brsa_secretkey_import(sk_der.bytes, sk_der.bytes_len), 0);
    assert_eq!(pk2.brsa_publickey_import(pk_der.bytes, pk_der.bytes_len), 0);
    sk_der.brsa_serializedkey_deinit();
    pk_der.brsa_serializedkey_deinit();

    sk2.brsa_secretkey_deinit();
    pk2.brsa_publickey_deinit();
}

// ---- Deterministic blind signature flow (mirrors C test_deterministic) ----

#[test]
fn test_blind_sign_flow_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(
        ctx.brsa_blind_message_generate(&mut blind_msg, &mut msg, 32, &mut client_secret, &mut pk),
        0
    );

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);

    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(
        ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32),
        0
    );
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);
    sig.brsa_signature_deinit();

    // Sign the same blind message again — deterministic should produce same blind_sig
    let mut blind_sig2 = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig2, &mut sk, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    assert_eq!(blind_sig.blind_sig_len, blind_sig2.blind_sig_len);
    assert_eq!(blind_sig.blind_sig, blind_sig2.blind_sig);

    blind_sig.brsa_blind_signature_deinit();
    blind_sig2.brsa_blind_signature_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// ---- Custom parameters flow (mirrors C test_custom_parameters) ----

#[test]
fn test_blind_sign_flow_custom_params() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 48);

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(
        ctx.brsa_blind_message_generate(&mut blind_msg, &mut msg, 32, &mut client_secret, &mut pk),
        0
    );

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(
        ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32),
        0
    );
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);
    sig.brsa_signature_deinit();

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// ---- SPKI export/import round-trip ----

#[test]
fn test_spki_export_import() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut spki = BRSASerializedKey::new();
    assert_eq!(ctx.brsa_publickey_export_spki(&mut spki, &pk), 0);
    assert!(spki.bytes_len > 0);

    let mut pk2 = BRSAPublicKey::new();
    assert_eq!(ctx.brsa_publickey_import_spki(&mut pk2, spki.bytes, spki.bytes_len), 0);

    let mut der1 = BRSASerializedKey::new();
    let mut der2 = BRSASerializedKey::new();
    assert_eq!(der1.brsa_publickey_export(&pk), 0);
    assert_eq!(der2.brsa_publickey_export(&pk2), 0);
    assert_eq!(der1.bytes_len, der2.bytes_len);
    assert_eq!(der1.bytes, der2.bytes);

    der1.brsa_serializedkey_deinit();
    der2.brsa_serializedkey_deinit();
    spki.brsa_serializedkey_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
    pk2.brsa_publickey_deinit();
}

// ---- Public key ID ----

#[test]
fn test_publickey_id_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let id1 = [0u8; 4];
    let id2 = [0u8; 4];
    assert_eq!(ctx.brsa_publickey_id(&id1, 4, &pk), 0);
    assert_eq!(ctx.brsa_publickey_id(&id2, 4, &pk), 0);
    assert_eq!(id1, id2);

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

#[test]
fn test_publickey_id_larger_buffer() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let id = [0xffu8; 64];
    assert_eq!(ctx.brsa_publickey_id(&id, 64, &pk), 0);
    // Bytes 32..64 should be zero (C code zeroes them)
    for &b in &id[32..64] {
        assert_eq!(b, 0);
    }

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// ---- Verify fails with wrong message ----

#[test]
fn test_verify_wrong_message_fails() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let mut msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(
        ctx.brsa_blind_message_generate(&mut blind_msg, &mut msg, 32, &mut client_secret, &mut pk),
        0
    );

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(
        ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32),
        0
    );
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    // Verify with wrong message should fail
    let wrong_msg = [1u8; 32];
    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &wrong_msg, 32), -1);

    sig.brsa_signature_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// ---- Edge: import invalid DER ----

#[test]
fn test_publickey_import_invalid_der() {
    let mut pk = BRSAPublicKey::new();
    let garbage = [0u8; 10];
    assert_eq!(pk.brsa_publickey_import(&garbage, 10), -1);
}

#[test]
fn test_secretkey_import_invalid_der() {
    let mut sk = BRSASecretKey::new();
    let garbage = [0u8; 10];
    assert_eq!(sk.brsa_secretkey_import(&garbage, 10), -1);
}

// ---- Edge: publickey import with oversized DER ----

#[test]
fn test_publickey_import_oversized_der() {
    let mut pk = BRSAPublicKey::new();
    let big = vec![0u8; MAX_SERIALIZED_PK_LEN + 1];
    assert_eq!(pk.brsa_publickey_import(&big, big.len()), -1);
}

// ---- MessageRandomizer struct ----

#[test]
fn test_message_randomizer_new() {
    let mr = BRSAMessageRandomizer::new();
    assert_eq!(mr.noise, [0u8; 32]);
}

// ---- Deinit on empty structs (should not panic) ----

#[test]
fn test_deinit_empty_structs() {
    let mut bm = BRSABlindMessage::new();
    bm.brsa_blind_message_deinit();

    let mut bs = BRSABlindingSecret::new();
    bs.brsa_blinding_secret_deinit();

    let mut bsig = BRSABlindSignature::new();
    bsig.brsa_blind_signature_deinit();

    let mut sig = BRSASignature::new();
    sig.brsa_signature_deinit();

    let mut sk = BRSASerializedKey::new();
    sk.brsa_serializedkey_deinit();
}

// ---- Constants match C ----

#[test]
fn test_constants() {
    assert_eq!(MIN_MODULUS_BITS, 2048);
    assert_eq!(MAX_MODULUS_BITS, 4096);
    assert_eq!(MAX_SERIALIZED_PK_LEN, 1000);
    assert_eq!(BRSA_DEFAULT_SALT_LENGTH, usize::MAX);
}

fn main() {}
