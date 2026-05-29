use libpsbt::psbt::{
    psbt_decode, psbt_encode, psbt_encode_raw, psbt_finalize, psbt_geterr, psbt_global_type_tostr,
    psbt_init, psbt_input_type_tostr, psbt_new_input_record_set, psbt_new_output_record_set,
    psbt_output_type_tostr, psbt_print, psbt_read, psbt_size, psbt_state_tostr,
    psbt_txelem_type_tostr, psbt_type_tostr, psbt_write_global_record, psbt_write_input_record,
    psbt_write_output_record, Psbt, PsbtElem, PsbtEncoding, PsbtGlobalType, PsbtInputType,
    PsbtOutputType, PsbtRecord, PsbtResult, PsbtScope, PsbtState, PsbtTxElemType, PSBT_MAGIC,
    MAX_SERIALIZE_SIZE,
};

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[test]
fn test_max_serialize_size_constant() {
    assert_eq!(MAX_SERIALIZE_SIZE, 0x02000000);
}

#[test]
fn test_psbt_magic_bytes() {
    assert_eq!(PSBT_MAGIC, [0x70, 0x73, 0x62, 0x74]);
}

#[test]
fn test_psbt_size_initial_zero() {
    let psbt = Psbt::new(1024);
    assert_eq!(psbt_size(&psbt), 0);
}

#[test]
fn test_psbt_init_resets_state() {
    let mut psbt = Psbt::new(1024);
    let mut buf = vec![0u8; 1024];
    let res = psbt_init(&mut psbt, &mut buf, 1024);
    assert_eq!(res, PsbtResult::Ok);
    assert!(matches!(psbt.state, PsbtState::Init));
    assert_eq!(psbt.write_pos, 0);
    assert_eq!(psbt.data_capacity, 1024);
}

#[test]
fn test_psbt_state_tostr() {
    assert_eq!(psbt_state_tostr(PsbtState::Init), "INIT");
    assert_eq!(psbt_state_tostr(PsbtState::Global), "GLOBAL");
    assert_eq!(psbt_state_tostr(PsbtState::Inputs), "INPUTS");
    assert_eq!(psbt_state_tostr(PsbtState::InputsNew), "INPUTS_NEW");
    assert_eq!(psbt_state_tostr(PsbtState::Outputs), "OUTPUTS");
    assert_eq!(psbt_state_tostr(PsbtState::OutputsNew), "OUTPUTS_NEW");
    assert_eq!(psbt_state_tostr(PsbtState::Finalized), "FINALIZED");
}

#[test]
fn test_psbt_global_type_tostr() {
    assert_eq!(
        psbt_global_type_tostr(PsbtGlobalType::UnsignedTx),
        "GLOBAL_UNSIGNED_TX"
    );
}

#[test]
fn test_psbt_input_type_tostr() {
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::NonWitnessUtxo),
        "IN_NON_WITNESS_UTXO"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::WitnessUtxo),
        "IN_WITNESS_UTXO"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::PartialSig),
        "IN_PARTIAL_SIG"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::SighashType),
        "IN_SIGHASH_TYPE"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::RedeemScript),
        "IN_REDEEM_SCRIPT"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::WitnessScript),
        "IN_WITNESS_SCRIPT"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::Bip32Derivation),
        "IN_BIP32_DERIVATION"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::FinalScriptSig),
        "IN_FINAL_SCRIPTSIG"
    );
    assert_eq!(
        psbt_input_type_tostr(PsbtInputType::FinalScriptWitness),
        "IN_FINAL_SCRIPTWITNESS"
    );
}

#[test]
fn test_psbt_output_type_tostr() {
    assert_eq!(
        psbt_output_type_tostr(PsbtOutputType::RedeemScript),
        "OUT_REDEEM_SCRIPT"
    );
    assert_eq!(
        psbt_output_type_tostr(PsbtOutputType::WitnessScript),
        "OUT_WITNESS_SCRIPT"
    );
    assert_eq!(
        psbt_output_type_tostr(PsbtOutputType::Bip32Derivation),
        "OUT_BIP32_DERIVATION"
    );
}

#[test]
fn test_psbt_txelem_type_tostr() {
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::Tx), "TX");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxIn), "TXIN");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxOut), "TXOUT");
    assert_eq!(
        psbt_txelem_type_tostr(PsbtTxElemType::WitnessItem),
        "WITNESS_ITEM"
    );
}

#[test]
fn test_psbt_type_tostr_global() {
    assert_eq!(psbt_type_tostr(0, PsbtScope::Global), "GLOBAL_UNSIGNED_TX");
    assert_eq!(
        psbt_type_tostr(99, PsbtScope::Global),
        "UNKNOWN_GLOBAL_TYPE"
    );
}

#[test]
fn test_psbt_type_tostr_inputs() {
    assert_eq!(psbt_type_tostr(0, PsbtScope::Inputs), "IN_NON_WITNESS_UTXO");
    assert_eq!(psbt_type_tostr(8, PsbtScope::Inputs), "IN_FINAL_SCRIPTWITNESS");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Inputs), "UNKNOWN_INPUT_TYPE");
}

#[test]
fn test_psbt_type_tostr_outputs() {
    assert_eq!(psbt_type_tostr(0, PsbtScope::Outputs), "OUT_REDEEM_SCRIPT");
    assert_eq!(
        psbt_type_tostr(2, PsbtScope::Outputs),
        "OUT_BIP32_DERIVATION"
    );
    assert_eq!(
        psbt_type_tostr(99, PsbtScope::Outputs),
        "UNKNOWN_OUTPUT_TYPE"
    );
}

#[test]
fn test_psbt_geterr_returns_static() {
    // Just verify it returns a string
    let s = psbt_geterr();
    assert!(!s.is_empty());
}

#[test]
fn test_psbt_decode_hex() {
    // From C harness: psbt_decode_hex "70736274ff0100" -> result=0 len=7 bytes=70736274ff0100
    let src = "70736274ff0100";
    let mut dest = vec![0u8; 1024];
    let dest_cap = dest.len();
    let mut psbt_len = 0;
    let res = psbt_decode(src, src.len(), &mut dest, dest_cap, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, 7);
    assert_eq!(&dest[..psbt_len], &hex_to_bytes("70736274ff0100"));
}

#[test]
fn test_psbt_decode_b64() {
    // From C harness:
    let src =
        "cHNidP8BACoCAAAAAAGA8PoCAAAAABepFCufG2xKKzFR7+3XGjiAZPO/VDBkhwAAAAAAAA==";
    let expected = "70736274ff01002a02000000000180f0fa020000000017a9142b9f1b6c4a2b3151efedd71a388064f3bf54306487000000000000";
    let mut dest = vec![0u8; 1024];
    let dest_cap = dest.len();
    let mut psbt_len = 0;
    let res = psbt_decode(src, src.len(), &mut dest, dest_cap, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, 52);
    assert_eq!(&dest[..psbt_len], &hex_to_bytes(expected));
}

#[test]
fn test_psbt_decode_too_small_returns_read_error() {
    let src = "70";
    let mut dest = vec![0u8; 64];
    let dest_cap = dest.len();
    let mut psbt_len = 0;
    let res = psbt_decode(src, src.len(), &mut dest, dest_cap, &mut psbt_len);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_encode_raw_hex() {
    // C: encode_hex "010203" -> result=0 len=7 (3*2+1) string="010203"
    let mut out = vec![0u8; 64];
    let cap = out.len();
    let mut out_len = 0;
    let data = hex_to_bytes("010203");
    let res = psbt_encode_raw(&data, data.len(), PsbtEncoding::Hex, &mut out, cap, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 7);
    assert_eq!(&out[..6], b"010203");
    assert_eq!(out[6], 0); // nul terminator
}

#[test]
fn test_psbt_encode_raw_base64() {
    // C: encode_b64 "010203" -> result=0 len=4 string="AQID"
    let mut out = vec![0u8; 64];
    let cap = out.len();
    let mut out_len = 0;
    let data = hex_to_bytes("010203");
    let res = psbt_encode_raw(&data, data.len(), PsbtEncoding::Base64, &mut out, cap, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 4);
    assert_eq!(&out[..out_len], b"AQID");
}

#[test]
fn test_psbt_encode_raw_protobuf_not_implemented() {
    let mut out = vec![0u8; 64];
    let cap = out.len();
    let mut out_len = 0;
    let res = psbt_encode_raw(b"\x01", 1, PsbtEncoding::Protobuf, &mut out, cap, &mut out_len);
    assert_eq!(res, PsbtResult::NotImplemented);
}

#[test]
fn test_write_input_record_before_global_returns_invalid_state() {
    let mut psbt = Psbt::new(1024);
    let rec = PsbtRecord {
        record_type: PsbtInputType::RedeemScript as u8,
        key: vec![],
        val: vec![1, 2, 3],
        scope: PsbtScope::Inputs,
    };
    let res = psbt_write_input_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_write_output_record_before_global_returns_invalid_state() {
    let mut psbt = Psbt::new(1024);
    let rec = PsbtRecord {
        record_type: PsbtOutputType::RedeemScript as u8,
        key: vec![],
        val: vec![1, 2, 3],
        scope: PsbtScope::Outputs,
    };
    let res = psbt_write_output_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_finalize_before_outputs_returns_invalid_state() {
    let mut psbt = Psbt::new(1024);
    let res = psbt_finalize(&mut psbt);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_print_before_finalized_returns_invalid_state() {
    let psbt = Psbt::new(1024);
    let mut buf: Vec<u8> = Vec::new();
    let res = psbt_print(&psbt, &mut buf);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_build_minimal_psbt() {
    // Build minimal psbt: header + global record (unsigned tx 010000000000ffffffff) +
    // new_input_record_set + new_output_record_set + finalize
    // Expected bytes from C harness: 70736274ff01000a010000000000ffffffff000000
    let mut psbt = Psbt::new(1024);
    let tx_bytes = hex_to_bytes("010000000000ffffffff");
    let rec = PsbtRecord {
        record_type: PsbtGlobalType::UnsignedTx as u8,
        key: vec![],
        val: tx_bytes,
        scope: PsbtScope::Global,
    };
    assert_eq!(psbt_write_global_record(&mut psbt, &rec), PsbtResult::Ok);
    assert_eq!(psbt_new_input_record_set(&mut psbt), PsbtResult::Ok);
    assert_eq!(psbt_new_output_record_set(&mut psbt), PsbtResult::Ok);
    assert_eq!(psbt_finalize(&mut psbt), PsbtResult::Ok);
    assert!(matches!(psbt.state, PsbtState::Finalized));

    let expected = hex_to_bytes("70736274ff01000a010000000000ffffffff000000");
    assert_eq!(psbt.data, expected);
    assert_eq!(psbt_size(&psbt), expected.len());
}

#[test]
fn test_build_test_vector_psbt() {
    // Replicate test.c test_vector(): two redeem-script inputs, no outputs.
    // Expected from C harness.
    let expected_hex = "70736274ff01007c02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d870000000000010447522103c8727ce35f1c93eb0be21406ee9a923c89219fe9c9e8504c8314a6a22d1295c02103c74dc710c407d7db6e041ee212d985cd2826d93f806ed44912b9a1da691c977352ae0104220020a8f44467bf171d51499153e01c0bd6291109fc38bd21b3c3224c9dc6b57590df0000";

    let tx_bytes = hex_to_bytes("02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000");
    let redeem_a = hex_to_bytes("522103c8727ce35f1c93eb0be21406ee9a923c89219fe9c9e8504c8314a6a22d1295c02103c74dc710c407d7db6e041ee212d985cd2826d93f806ed44912b9a1da691c977352ae");
    let redeem_b = hex_to_bytes("0020a8f44467bf171d51499153e01c0bd6291109fc38bd21b3c3224c9dc6b57590df");

    let mut psbt = Psbt::new(1024);

    let mut rec = PsbtRecord {
        record_type: PsbtGlobalType::UnsignedTx as u8,
        key: vec![],
        val: tx_bytes,
        scope: PsbtScope::Global,
    };
    assert_eq!(psbt_write_global_record(&mut psbt, &rec), PsbtResult::Ok);

    rec = PsbtRecord {
        record_type: PsbtInputType::RedeemScript as u8,
        key: vec![],
        val: redeem_a,
        scope: PsbtScope::Inputs,
    };
    assert_eq!(psbt_write_input_record(&mut psbt, &rec), PsbtResult::Ok);

    rec = PsbtRecord {
        record_type: PsbtInputType::RedeemScript as u8,
        key: vec![],
        val: redeem_b,
        scope: PsbtScope::Inputs,
    };
    assert_eq!(psbt_write_input_record(&mut psbt, &rec), PsbtResult::Ok);

    assert_eq!(psbt_new_output_record_set(&mut psbt), PsbtResult::Ok);
    assert_eq!(psbt_finalize(&mut psbt), PsbtResult::Ok);

    assert_eq!(bytes_to_hex(&psbt.data), expected_hex);
}

#[test]
fn test_psbt_read_and_record_count() {
    // Use a known hex psbt from test.c (psbt_hex). Decode, then read it.
    let psbt_hex = "70736274ff01009a020000000258e87a21b56daf0c23be8e7070456c336f7cbaa5c8757924f545887bb2abdd750000000000ffffffff838d0427d0ec650a68aa46bb0b098aea4422c071b2ca78352a077959d07cea1d0100000000ffffffff0270aaf00800000000160014d85c2b71d0060b09c9886aeb815e50991dda124d00e1f5050000000016001400aea9a2e5f0f876a588df5546e8742d1d87008f00000000000100bb0200000001aad73931018bd25f84ae400b68848be09db706eac2ac18298babee71ab656f8b0000000048473044022058f6fc7c6a33e1b31548d481c826c015bd30135aad42cd67790dab66d2ad243b02204a1ced2604c6735b6393e5b41691dd78b00f0c5942fb9f751856faa938157dba01feffffff0280f0fa020000000017a9140fb9463421696b82c833af241c78c17ddbde493487d0f20a270100000017a91429ca74f8a08f81999428185c97b5d852e4063f6187650000000104475221029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f2102dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d752ae2206029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f10d90c6a4f000000800000008000000080220602dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d710d90c6a4f0000008000000080010000800001012000c2eb0b0000000017a914b7f5faf40e3d40a5a459b1db3535f2b72fa921e88701042200208c2353173743b595dfb4a07b72ba8e42e3797da74e87fe7d9d7497e3b2028903010547522103089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc21023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7352ae2206023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7310d90c6a4f000000800000008003000080220603089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc10d90c6a4f00000080000000800200008000220203a9a4c37f5996d3aa25dbac6b570af0650394492942460b354753ed9eeca5877110d90c6a4f000000800000008004000080002202027f6399757d2eff55a136ad02c684b1838b6556e5f1b6b34282a94b6b5005109610d90c6a4f00000080000000800500008000";

    // Decode hex.
    let mut buf = vec![0u8; 2048];
    let cap = buf.len();
    let mut psbt_len = 0;
    let res = psbt_decode(psbt_hex, psbt_hex.len(), &mut buf, cap, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, psbt_hex.len() / 2);

    // Read into a fresh PSBT.
    let mut psbt = Psbt::new(2048);
    let mut counter = RecCounter::default();
    let res = psbt_read(
        &buf[..psbt_len],
        psbt_len,
        &mut psbt,
        Some(handler),
        &mut counter,
    );
    assert_eq!(res, PsbtResult::Ok);
    assert!(matches!(psbt.state, PsbtState::Finalized));

    // The first record encountered must be GLOBAL_UNSIGNED_TX (type 0).
    assert!(counter.first_record_type.is_some());
    assert_eq!(counter.first_record_type, Some(0));

    // The first record after global is the input's NON_WITNESS_UTXO (type 0).
    // (matches test.c's read_test_vector_checker)
    assert_eq!(counter.records.len() >= 2, true);
    assert_eq!(counter.records[0], 0); // global unsigned tx
    assert_eq!(counter.records[1], 0); // PSBT_IN_NON_WITNESS_UTXO
}

#[test]
fn test_psbt_read_then_encode_hex_roundtrip() {
    // Mirrors encode_decode_test() in test.c.
    let psbt_hex = "70736274ff01009a020000000258e87a21b56daf0c23be8e7070456c336f7cbaa5c8757924f545887bb2abdd750000000000ffffffff838d0427d0ec650a68aa46bb0b098aea4422c071b2ca78352a077959d07cea1d0100000000ffffffff0270aaf00800000000160014d85c2b71d0060b09c9886aeb815e50991dda124d00e1f5050000000016001400aea9a2e5f0f876a588df5546e8742d1d87008f00000000000100bb0200000001aad73931018bd25f84ae400b68848be09db706eac2ac18298babee71ab656f8b0000000048473044022058f6fc7c6a33e1b31548d481c826c015bd30135aad42cd67790dab66d2ad243b02204a1ced2604c6735b6393e5b41691dd78b00f0c5942fb9f751856faa938157dba01feffffff0280f0fa020000000017a9140fb9463421696b82c833af241c78c17ddbde493487d0f20a270100000017a91429ca74f8a08f81999428185c97b5d852e4063f6187650000000104475221029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f2102dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d752ae2206029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f10d90c6a4f000000800000008000000080220602dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d710d90c6a4f0000008000000080010000800001012000c2eb0b0000000017a914b7f5faf40e3d40a5a459b1db3535f2b72fa921e88701042200208c2353173743b595dfb4a07b72ba8e42e3797da74e87fe7d9d7497e3b2028903010547522103089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc21023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7352ae2206023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7310d90c6a4f000000800000008003000080220603089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc10d90c6a4f00000080000000800200008000220203a9a4c37f5996d3aa25dbac6b570af0650394492942460b354753ed9eeca5877110d90c6a4f000000800000008004000080002202027f6399757d2eff55a136ad02c684b1838b6556e5f1b6b34282a94b6b5005109610d90c6a4f00000080000000800500008000";

    let mut intbuf = vec![0u8; 2048];
    let intcap = intbuf.len();
    let mut psbt_len = 0;
    let res = psbt_decode(psbt_hex, psbt_hex.len(), &mut intbuf, intcap, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, psbt_hex.len() / 2);

    let mut psbt = Psbt::new(2048);
    let res = psbt_read(&intbuf[..psbt_len], psbt_len, &mut psbt, None, &mut ());
    assert_eq!(res, PsbtResult::Ok);

    let mut buf = vec![0u8; 4096];
    let bufcap = buf.len();
    let mut out_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut buf, bufcap, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, psbt_hex.len() + 1);
    assert_eq!(&buf[..psbt_hex.len()], psbt_hex.as_bytes());
    assert_eq!(buf[psbt_hex.len()], 0);
}

#[test]
fn test_psbt_encode_before_finalized_returns_write_error() {
    let psbt = Psbt::new(64);
    let mut out = vec![0u8; 256];
    let cap = out.len();
    let mut out_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut out, cap, &mut out_len);
    assert_eq!(res, PsbtResult::WriteError);
}

#[test]
fn test_psbt_read_invalid_state_when_already_initialized_after_global() {
    let mut psbt = Psbt::new(1024);
    psbt.state = PsbtState::Global;
    let res = psbt_read(b"abc", 3, &mut psbt, None, &mut ());
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_empty_input_psbt_read() {
    // empty_inputs base64 from test.c.
    let src = "cHNidP8BACoCAAAAAAGA8PoCAAAAABepFCufG2xKKzFR7+3XGjiAZPO/VDBkhwAAAAAAAA==";
    let mut buf = vec![0u8; 2048];
    let cap = buf.len();
    let mut psbt_len = 0;
    let res = psbt_decode(src, src.len(), &mut buf, cap, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, 52);

    let mut psbt = Psbt::new(2048);
    let res = psbt_read(&buf[..psbt_len], psbt_len, &mut psbt, None, &mut ());
    assert_eq!(res, PsbtResult::Ok);
    assert!(matches!(psbt.state, PsbtState::Finalized));
}

// ---- Helpers (must be declared before use is ok in Rust top-level) ----

#[derive(Default)]
struct RecCounter {
    first_record_type: Option<u8>,
    records: Vec<u8>,
}

fn handler(elem: &mut PsbtElem, ud: &mut dyn std::any::Any) {
    let counter = ud.downcast_mut::<RecCounter>().unwrap();
    if let PsbtElem::Record { record, .. } = elem {
        if counter.first_record_type.is_none() {
            counter.first_record_type = Some(record.record_type);
        }
        counter.records.push(record.record_type);
    }
}

fn main() {}
