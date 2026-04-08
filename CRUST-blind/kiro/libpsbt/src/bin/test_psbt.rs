use libpsbt::psbt::*;
use libpsbt::tx::*;
use std::any::Any;

// --- Type-to-string tests ---

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
fn test_psbt_input_type_tostr() {
    assert_eq!(psbt_input_type_tostr(PsbtInputType::NonWitnessUtxo), "IN_NON_WITNESS_UTXO");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::WitnessUtxo), "IN_WITNESS_UTXO");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::PartialSig), "IN_PARTIAL_SIG");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::SighashType), "IN_SIGHASH_TYPE");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::RedeemScript), "IN_REDEEM_SCRIPT");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::WitnessScript), "IN_WITNESS_SCRIPT");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::Bip32Derivation), "IN_BIP32_DERIVATION");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::FinalScriptSig), "IN_FINAL_SCRIPTSIG");
    assert_eq!(psbt_input_type_tostr(PsbtInputType::FinalScriptWitness), "IN_FINAL_SCRIPTWITNESS");
}

#[test]
fn test_psbt_output_type_tostr() {
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::RedeemScript), "OUT_REDEEM_SCRIPT");
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::WitnessScript), "OUT_WITNESS_SCRIPT");
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::Bip32Derivation), "OUT_BIP32_DERIVATION");
}

#[test]
fn test_psbt_global_type_tostr() {
    assert_eq!(psbt_global_type_tostr(PsbtGlobalType::UnsignedTx), "GLOBAL_UNSIGNED_TX");
}

#[test]
fn test_psbt_txelem_type_tostr() {
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::Tx), "TX");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxIn), "TXIN");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxOut), "TXOUT");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::WitnessItem), "WITNESS_ITEM");
}

#[test]
fn test_psbt_type_tostr() {
    assert_eq!(psbt_type_tostr(0, PsbtScope::Global), "GLOBAL_UNSIGNED_TX");
    assert_eq!(psbt_type_tostr(4, PsbtScope::Inputs), "IN_REDEEM_SCRIPT");
    assert_eq!(psbt_type_tostr(1, PsbtScope::Outputs), "OUT_WITNESS_SCRIPT");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Inputs), "UNKNOWN_INPUT_TYPE");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Global), "UNKNOWN_GLOBAL_TYPE");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Outputs), "UNKNOWN_OUTPUT_TYPE");
}

// --- PSBT init/finalize tests ---

#[test]
fn test_psbt_init() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    let res = psbt_init(&mut psbt, &mut dest, 1024);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Init);
    assert_eq!(psbt_size(&psbt), 0);
}

#[test]
fn test_psbt_write_input_before_global_fails() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);

    let rec = PsbtRecord {
        record_type: 4, // IN_REDEEM_SCRIPT
        key: vec![],
        val: vec![0u8; 10],
        scope: PsbtScope::Inputs,
    };
    let res = psbt_write_input_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_psbt_finalize_without_outputs_fails() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);
    let res = psbt_finalize(&mut psbt);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_psbt_print_before_finalize_fails() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);
    let mut output = Vec::new();
    let res = psbt_print(&psbt, &mut output);
    assert_eq!(res, PsbtResult::InvalidState);
}

// --- PSBT write test vector (from C test.c) ---

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

const EXPECTED_WRITE_HEX: &str = "70736274ff01007c02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d870000000000010447522103c8727ce35f1c93eb0be21406ee9a923c89219fe9c9e8504c8314a6a22d1295c02103c74dc710c407d7db6e041ee212d985cd2826d93f806ed44912b9a1da691c977352ae0104220020a8f44467bf171d51499153e01c0bd6291109fc38bd21b3c3224c9dc6b57590df0000";

#[test]
fn test_psbt_write_test_vector() {
    let transaction: Vec<u8> = hex_to_bytes("02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000");
    let redeem_script_a: Vec<u8> = hex_to_bytes("522103c8727ce35f1c93eb0be21406ee9a923c89219fe9c9e8504c8314a6a22d1295c02103c74dc710c407d7db6e041ee212d985cd2826d93f806ed44912b9a1da691c977352ae");
    let redeem_script_b: Vec<u8> = hex_to_bytes("0020a8f44467bf171d51499153e01c0bd6291109fc38bd21b3c3224c9dc6b57590df");

    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);

    // Write global record (unsigned tx)
    let rec = PsbtRecord {
        record_type: 0, // PSBT_GLOBAL_UNSIGNED_TX
        key: vec![],
        val: transaction,
        scope: PsbtScope::Global,
    };
    assert_eq!(psbt_write_global_record(&mut psbt, &rec), PsbtResult::Ok);

    // Write first input record (redeem script)
    let rec = PsbtRecord {
        record_type: 4, // PSBT_IN_REDEEM_SCRIPT
        key: vec![],
        val: redeem_script_a,
        scope: PsbtScope::Inputs,
    };
    assert_eq!(psbt_write_input_record(&mut psbt, &rec), PsbtResult::Ok);

    // Write second input record (redeem script)
    let rec = PsbtRecord {
        record_type: 4, // PSBT_IN_REDEEM_SCRIPT
        key: vec![],
        val: redeem_script_b,
        scope: PsbtScope::Inputs,
    };
    assert_eq!(psbt_write_input_record(&mut psbt, &rec), PsbtResult::Ok);

    // New output record set
    assert_eq!(psbt_new_output_record_set(&mut psbt), PsbtResult::Ok);

    // Print should fail before finalize
    let mut output = Vec::new();
    assert_eq!(psbt_print(&psbt, &mut output), PsbtResult::InvalidState);

    // Finalize
    assert_eq!(psbt_finalize(&mut psbt), PsbtResult::Ok);

    // Verify size and hex output
    assert_eq!(psbt_size(&psbt), 246);
    assert_eq!(bytes_to_hex(&psbt.data), EXPECTED_WRITE_HEX);
}

// --- PSBT encode/decode tests ---

const PSBT_HEX: &str = "70736274ff01009a020000000258e87a21b56daf0c23be8e7070456c336f7cbaa5c8757924f545887bb2abdd750000000000ffffffff838d0427d0ec650a68aa46bb0b098aea4422c071b2ca78352a077959d07cea1d0100000000ffffffff0270aaf00800000000160014d85c2b71d0060b09c9886aeb815e50991dda124d00e1f5050000000016001400aea9a2e5f0f876a588df5546e8742d1d87008f00000000000100bb0200000001aad73931018bd25f84ae400b68848be09db706eac2ac18298babee71ab656f8b0000000048473044022058f6fc7c6a33e1b31548d481c826c015bd30135aad42cd67790dab66d2ad243b02204a1ced2604c6735b6393e5b41691dd78b00f0c5942fb9f751856faa938157dba01feffffff0280f0fa020000000017a9140fb9463421696b82c833af241c78c17ddbde493487d0f20a270100000017a91429ca74f8a08f81999428185c97b5d852e4063f6187650000000104475221029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f2102dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d752ae2206029583bf39ae0a609747ad199addd634fa6108559d6c5cd39b4c2183f1ab96e07f10d90c6a4f000000800000008000000080220602dab61ff49a14db6a7d02b0cd1fbb78fc4b18312b5b4e54dae4dba2fbfef536d710d90c6a4f0000008000000080010000800001012000c2eb0b0000000017a914b7f5faf40e3d40a5a459b1db3535f2b72fa921e88701042200208c2353173743b595dfb4a07b72ba8e42e3797da74e87fe7d9d7497e3b2028903010547522103089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc21023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7352ae2206023add904f3d6dcf59ddb906b0dee23529b7ffb9ed50e5e86151926860221f0e7310d90c6a4f000000800000008003000080220603089dc10c7ac6db54f91329af617333db388cead0c231f723379d1b99030b02dc10d90c6a4f00000080000000800200008000220203a9a4c37f5996d3aa25dbac6b570af0650394492942460b354753ed9eeca5877110d90c6a4f000000800000008004000080002202027f6399757d2eff55a136ad02c684b1838b6556e5f1b6b34282a94b6b5005109610d90c6a4f00000080000000800500008000";

#[test]
fn test_psbt_hex_decode() {
    let mut buf = [0u8; 2048];
    let mut psbt_len = 0usize;
    let hexlen = PSBT_HEX.len();

    let res = psbt_decode(PSBT_HEX, hexlen, &mut buf, 2048, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, hexlen / 2);
}

#[test]
fn test_psbt_read_and_encode_roundtrip() {
    let mut buf = [0u8; 2048];
    let mut intbuf = [0u8; 2048];
    let mut psbt_len = 0usize;
    let hexlen = PSBT_HEX.len();

    let res = psbt_decode(PSBT_HEX, hexlen, &mut intbuf, 2048, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt_len, hexlen / 2);

    let mut psbt = Psbt::new(2048);
    psbt_init(&mut psbt, &mut intbuf, psbt_len);

    let mut dummy = ();
    let res = psbt_read(&intbuf[..psbt_len], psbt_len, &mut psbt, None, &mut dummy);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Finalized);

    // Encode back to hex
    let mut out_len = 0usize;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut buf, 2048, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, hexlen + 1); // +1 for nul terminator
    assert_eq!(&buf[..hexlen], PSBT_HEX.as_bytes());
}

#[test]
fn test_psbt_read_with_handler() {
    let mut buf = [0u8; 2048];
    let mut psbt_len = 0usize;
    let hexlen = PSBT_HEX.len();

    let res = psbt_decode(PSBT_HEX, hexlen, &mut buf, 2048, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);

    let mut psbt = Psbt::new(2048);
    let mut intbuf = [0u8; 2048];
    psbt_init(&mut psbt, &mut intbuf, 2048);

    fn checker(elem: &mut PsbtElem, user_data: &mut dyn Any) {
        let step = user_data.downcast_mut::<i32>().unwrap();
        match elem {
            PsbtElem::Record { record, .. } => {
                if *step == 0 {
                    assert_eq!(record.record_type, 0); // GLOBAL_UNSIGNED_TX
                }
            }
            _ => {}
        }
        *step += 1;
    }

    let mut step: i32 = 0;
    let res = psbt_read(&buf[..psbt_len], psbt_len, &mut psbt, Some(checker), &mut step);
    assert_eq!(res, PsbtResult::Ok);
    assert!(step > 0);
}

#[test]
fn test_psbt_base64_decode() {
    let empty_inputs = "cHNidP8BACoCAAAAAAGA8PoCAAAAABepFCufG2xKKzFR7+3XGjiAZPO/VDBkhwAAAAAAAA==";
    let mut buf = [0u8; 2048];
    let mut psbt_len = 0usize;

    let res = psbt_decode(empty_inputs, empty_inputs.len(), &mut buf, 2048, &mut psbt_len);
    assert_eq!(res, PsbtResult::Ok);

    let mut psbt = Psbt::new(2048);
    let mut intbuf = [0u8; 2048];
    psbt_init(&mut psbt, &mut intbuf, psbt_len);

    let mut dummy = ();
    let res = psbt_read(&buf[..psbt_len], psbt_len, &mut psbt, None, &mut dummy);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Finalized);
}

#[test]
fn test_psbt_encode_before_finalize_fails() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);

    let mut buf = [0u8; 2048];
    let mut out_len = 0usize;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut buf, 2048, &mut out_len);
    assert_eq!(res, PsbtResult::WriteError);
}

#[test]
fn test_psbt_decode_too_small() {
    let mut buf = [0u8; 64];
    let mut psbt_len = 0usize;
    let res = psbt_decode("ab", 2, &mut buf, 64, &mut psbt_len);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_read_not_initialized() {
    let mut psbt = Psbt::new(1024);
    psbt.state = PsbtState::Global; // not Init
    let mut dummy = ();
    let res = psbt_read(&[0u8; 10], 10, &mut psbt, None, &mut dummy);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_psbt_encode_base64() {
    let mut buf = [0u8; 2048];
    let mut intbuf = [0u8; 2048];
    let mut psbt_len = 0usize;
    let hexlen = PSBT_HEX.len();

    psbt_decode(PSBT_HEX, hexlen, &mut intbuf, 2048, &mut psbt_len);

    let mut psbt = Psbt::new(2048);
    psbt_init(&mut psbt, &mut intbuf, psbt_len);

    let mut dummy = ();
    psbt_read(&intbuf[..psbt_len], psbt_len, &mut psbt, None, &mut dummy);

    let mut out_len = 0usize;
    let res = psbt_encode(&psbt, PsbtEncoding::Base64, &mut buf, 2048, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert!(out_len > 0);
    // The base64 output should start with "cHNid" (base64 of "psbt")
    assert!(buf[..5].starts_with(b"cHNid"));
}

#[test]
fn test_psbt_encode_base62() {
    // Use small known data that doesn't produce 6-bit indices >= 62
    let data = b"foo";
    let mut buf = [0u8; 64];
    let mut out_len = 0usize;
    let res = psbt_encode_raw(data, data.len(), PsbtEncoding::Base62, &mut buf, 64, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 4);
    assert_eq!(&buf[..4], b"Pczl");
}

#[test]
fn test_psbt_encode_protobuf_not_implemented() {
    let mut buf = [0u8; 64];
    let mut out_len = 0usize;
    let res = psbt_encode_raw(&[0u8; 4], 4, PsbtEncoding::Protobuf, &mut buf, 64, &mut out_len);
    assert_eq!(res, PsbtResult::NotImplemented);
}

#[test]
fn test_psbt_hex_encode_raw() {
    let data = [0x70u8, 0x73, 0x62, 0x74];
    let mut buf = [0u8; 64];
    let mut out_len = 0usize;
    let res = psbt_encode_raw(&data, 4, PsbtEncoding::Hex, &mut buf, 64, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 9); // 4*2 + 1 nul
    assert_eq!(&buf[..8], b"70736274");
}

#[test]
fn test_psbt_geterr() {
    let err = psbt_geterr();
    assert!(!err.is_empty());
}

#[test]
fn test_psbt_magic() {
    assert_eq!(PSBT_MAGIC, [0x70, 0x73, 0x62, 0x74]);
}

#[test]
fn test_psbt_new_input_record_set() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);

    let transaction = hex_to_bytes("02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000");

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: transaction,
        scope: PsbtScope::Global,
    };
    assert_eq!(psbt_write_global_record(&mut psbt, &rec), PsbtResult::Ok);

    // new_input_record_set from Global state
    assert_eq!(psbt_new_input_record_set(&mut psbt), PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::InputsNew);
}

#[test]
fn test_psbt_new_output_record_set_from_inputs() {
    let mut psbt = Psbt::new(1024);
    let mut dest = [0u8; 1024];
    psbt_init(&mut psbt, &mut dest, 1024);

    let transaction = hex_to_bytes("02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000");

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: transaction,
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);

    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: vec![0u8; 10],
        scope: PsbtScope::Inputs,
    };
    psbt_write_input_record(&mut psbt, &rec);

    // new_output_record_set from Inputs state
    assert_eq!(psbt_new_output_record_set(&mut psbt), PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::OutputsNew);
}

fn main() {}
