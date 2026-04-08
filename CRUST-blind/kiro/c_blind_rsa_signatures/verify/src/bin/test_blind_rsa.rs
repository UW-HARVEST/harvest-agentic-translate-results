use c_blind_rsa_signatures::blind_rsa::*;

// --- Context initialization tests ---

#[test]
fn test_context_init_default() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();
    assert_eq!(ctx.salt_len, 48); // SHA384 digest size
}

#[test]
fn test_context_init_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();
    assert_eq!(ctx.salt_len, 0);
}

#[test]
fn test_context_init_custom_sha256_salt48() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 48);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt_len, 48);
}

#[test]
fn test_context_init_custom_default_salt_sha256() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt_len, 32);
}

#[test]
fn test_context_init_custom_default_salt_sha384() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA384, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt_len, 48);
}

#[test]
fn test_context_init_custom_default_salt_sha512() {
    let mut ctx = BRSAContext::new();
    let ret = ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA512, BRSA_DEFAULT_SALT_LENGTH);
    assert_eq!(ret, 0);
    assert_eq!(ctx.salt_len, 64);
}

// --- Key generation tests ---

#[test]
fn test_keypair_generate_2048() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let ret = sk.brsa_keypair_generate(&mut pk, 2048);
    assert_eq!(ret, 0);
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

#[test]
fn test_keypair_generate_1024_fails() {
    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    let ret = sk.brsa_keypair_generate(&mut pk, 1024);
    assert_eq!(ret, -1);
}

// --- Key serialization round-trip tests ---

#[test]
fn test_secretkey_export_import_roundtrip() {
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
fn test_publickey_export_import_roundtrip() {
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

    // Both public keys should export to the same DER
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

// --- SPKI export/import tests ---

#[test]
fn test_spki_export_import_roundtrip() {
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

    // Verify the imported key exports to the same DER as the original
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

// --- Key ID tests ---

#[test]
fn test_publickey_id() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let key_id = [0u8; 4];
    assert_eq!(ctx.brsa_publickey_id(&key_id, 4, &pk), 0);
    // Key ID should be non-zero (it's a SHA256 hash prefix)
    assert!(key_id.iter().any(|&b| b != 0));

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

#[test]
fn test_publickey_id_large_buffer_zero_pads() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let key_id = [0xffu8; 64];
    assert_eq!(ctx.brsa_publickey_id(&key_id, 64, &pk), 0);
    // Bytes 32..64 should be zero-padded (SHA256 is 32 bytes)
    for i in 32..64 {
        assert_eq!(key_id[i], 0, "byte {} should be zero-padded", i);
    }

    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// --- Full blind RSA flow: default context ---

#[test]
fn test_blind_rsa_flow_default() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(ctx.brsa_blind_message_generate(&mut blind_msg, &msg, 32, &mut client_secret, &mut pk), 0);
    assert_eq!(blind_msg.blind_message_len, 256); // 2048 bits = 256 bytes
    assert_eq!(client_secret.secret_len, 256);

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);
    assert_eq!(blind_sig.blind_sig_len, 256);
    blind_msg.brsa_blind_message_deinit();

    let mut sig = BRSASignature::new();
    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    assert_eq!(ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32), 0);
    assert_eq!(sig.sig_len, 256);
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);

    sig.brsa_signature_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// --- Full blind RSA flow: deterministic context ---

#[test]
fn test_blind_rsa_flow_deterministic() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_deterministic();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(ctx.brsa_blind_message_generate(&mut blind_msg, &msg, 32, &mut client_secret, &mut pk), 0);

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);

    // Sign the same blind message again - deterministic should produce identical result
    let mut blind_sig2 = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig2, &mut sk, &blind_msg), 0);
    assert_eq!(blind_sig.blind_sig_len, blind_sig2.blind_sig_len);
    assert_eq!(blind_sig.blind_sig, blind_sig2.blind_sig);

    // Finalize and verify
    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32), 0);
    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);

    sig.brsa_signature_deinit();
    blind_sig.brsa_blind_signature_deinit();
    blind_sig2.brsa_blind_signature_deinit();
    blind_msg.brsa_blind_message_deinit();
    client_secret.brsa_blinding_secret_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// --- Full blind RSA flow: custom parameters (SHA256, salt_len=48) ---

#[test]
fn test_blind_rsa_flow_custom_sha256() {
    let mut ctx = BRSAContext::new();
    assert_eq!(ctx.brsa_context_init_custom(BRSAHashFunction::BRSA_SHA256, 48), 0);

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    let msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(ctx.brsa_blind_message_generate(&mut blind_msg, &msg, 32, &mut client_secret, &mut pk), 0);

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk, &msg, 32), 0);
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk, &msg_randomizer, &msg, 32), 0);

    sig.brsa_signature_deinit();
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();
}

// --- Key export after import round-trip preserves signing ability ---

#[test]
fn test_key_reimport_then_sign() {
    let mut ctx = BRSAContext::new();
    ctx.brsa_context_init_default();

    let mut sk = BRSASecretKey::new();
    let mut pk = BRSAPublicKey::new();
    assert_eq!(sk.brsa_keypair_generate(&mut pk, 2048), 0);

    // Export keys
    let mut sk_der = BRSASerializedKey::new();
    let mut pk_der = BRSASerializedKey::new();
    assert_eq!(sk_der.brsa_secretkey_export(&sk), 0);
    assert_eq!(pk_der.brsa_publickey_export(&pk), 0);

    // Deinit originals
    sk.brsa_secretkey_deinit();
    pk.brsa_publickey_deinit();

    // Re-import
    let mut sk2 = BRSASecretKey::new();
    let mut pk2 = BRSAPublicKey::new();
    assert_eq!(sk2.brsa_secretkey_import(sk_der.bytes, sk_der.bytes_len), 0);
    assert_eq!(pk2.brsa_publickey_import(pk_der.bytes, pk_der.bytes_len), 0);
    sk_der.brsa_serializedkey_deinit();
    pk_der.brsa_serializedkey_deinit();

    // Use re-imported keys for full flow
    let msg = [0u8; 32];
    let mut blind_msg = BRSABlindMessage::new();
    let mut client_secret = BRSABlindingSecret::new();
    assert_eq!(ctx.brsa_blind_message_generate(&mut blind_msg, &msg, 32, &mut client_secret, &mut pk2), 0);

    let mut blind_sig = BRSABlindSignature::new();
    assert_eq!(ctx.brsa_blind_sign(&mut blind_sig, &mut sk2, &blind_msg), 0);
    blind_msg.brsa_blind_message_deinit();

    let msg_randomizer: Option<BRSAMessageRandomizer> = None;
    let mut sig = BRSASignature::new();
    assert_eq!(ctx.brsa_finalize(&mut sig, &blind_sig, &client_secret, &msg_randomizer, &mut pk2, &msg, 32), 0);
    blind_sig.brsa_blind_signature_deinit();
    client_secret.brsa_blinding_secret_deinit();

    assert_eq!(ctx.brsa_verify(&sig, &mut pk2, &msg_randomizer, &msg, 32), 0);

    sig.brsa_signature_deinit();
    sk2.brsa_secretkey_deinit();
    pk2.brsa_publickey_deinit();
}

fn main() {}
