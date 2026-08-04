use libpsbt::psbt::{
    psbt_decode, psbt_encode, psbt_encode_raw, psbt_finalize, psbt_geterr,
    psbt_global_type_tostr, psbt_init, psbt_input_type_tostr, psbt_new_input_record_set,
    psbt_new_output_record_set, psbt_output_type_tostr, psbt_print, psbt_read, psbt_size,
    psbt_state_tostr, psbt_txelem_type_tostr, psbt_type_tostr, psbt_write_global_record,
    psbt_write_input_record, psbt_write_output_record, Psbt, PsbtElem, PsbtEncoding,
    PsbtGlobalType, PsbtInputType, PsbtOutputType, PsbtRecord, PsbtResult, PsbtScope, PsbtState,
    PsbtTxElemType, PSBT_MAGIC,
};

const TRANSACTION: &[u8] = &[
    0x02, 0x00, 0x00, 0x00, 0x02, 0x2e, 0x8c, 0x7d, 0x8d, 0x37, 0xc4, 0x27, 0xe0, 0x60, 0xec, 0x00,
    0x2e, 0xc1, 0xc2, 0xbc, 0x30, 0x19, 0x6f, 0xc2, 0xf7, 0x5d, 0x6a, 0x88, 0x44, 0xcb, 0xc0, 0x36,
    0x51, 0xc0, 0x81, 0x43, 0x0a, 0x01, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x96, 0xa0,
    0x4e, 0x0c, 0xc6, 0x36, 0xf3, 0x77, 0x93, 0x3e, 0x3d, 0x93, 0xac, 0xcc, 0x62, 0x7f, 0xaa, 0xcd,
    0xbc, 0xdb, 0x5a, 0x96, 0x24, 0xdf, 0x1b, 0x49, 0x0b, 0xd0, 0x45, 0xf2, 0x4d, 0x2c, 0x00, 0x00,
    0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x01, 0xe0, 0x2b, 0xe5, 0x0e, 0x00, 0x00, 0x00, 0x00,
    0x17, 0xa9, 0x14, 0xb5, 0x3b, 0xb0, 0xdc, 0x1d, 0xb8, 0xc8, 0xd8, 0x03, 0xe3, 0xe3, 0x9f, 0x78,
    0x4d, 0x42, 0xe4, 0x73, 0x7f, 0xfa, 0x0d, 0x87, 0x00, 0x00, 0x00, 0x00,
];

const REDEEM_SCRIPT_A: &[u8] = &[
    0x52, 0x21, 0x03, 0xc8, 0x72, 0x7c, 0xe3, 0x5f, 0x1c, 0x93, 0xeb, 0x0b, 0xe2, 0x14, 0x06, 0xee,
    0x9a, 0x92, 0x3c, 0x89, 0x21, 0x9f, 0xe9, 0xc9, 0xe8, 0x50, 0x4c, 0x83, 0x14, 0xa6, 0xa2, 0x2d,
    0x12, 0x95, 0xc0, 0x21, 0x03, 0xc7, 0x4d, 0xc7, 0x10, 0xc4, 0x07, 0xd7, 0xdb, 0x6e, 0x04, 0x1e,
    0xe2, 0x12, 0xd9, 0x85, 0xcd, 0x28, 0x26, 0xd9, 0x3f, 0x80, 0x6e, 0xd4, 0x49, 0x12, 0xb9, 0xa1,
    0xda, 0x69, 0x1c, 0x97, 0x73, 0x52, 0xae,
];

#[test]
fn test_psbt_magic_const() {
    assert_eq!(PSBT_MAGIC, [0x70, 0x73, 0x62, 0x74]);
}

#[test]
fn test_state_tostr() {
    assert_eq!(psbt_state_tostr(PsbtState::Init), "INIT");
    assert_eq!(psbt_state_tostr(PsbtState::Global), "GLOBAL");
    assert_eq!(psbt_state_tostr(PsbtState::Inputs), "INPUTS");
    assert_eq!(psbt_state_tostr(PsbtState::InputsNew), "INPUTS_NEW");
    assert_eq!(psbt_state_tostr(PsbtState::Outputs), "OUTPUTS");
    assert_eq!(psbt_state_tostr(PsbtState::OutputsNew), "OUTPUTS_NEW");
    assert_eq!(psbt_state_tostr(PsbtState::Finalized), "FINALIZED");
}

#[test]
fn test_type_tostr() {
    assert_eq!(psbt_type_tostr(0, PsbtScope::Global), "GLOBAL_UNSIGNED_TX");
    assert_eq!(psbt_type_tostr(0, PsbtScope::Inputs), "IN_NON_WITNESS_UTXO");
    assert_eq!(psbt_type_tostr(4, PsbtScope::Inputs), "IN_REDEEM_SCRIPT");
    assert_eq!(psbt_type_tostr(8, PsbtScope::Inputs), "IN_FINAL_SCRIPTWITNESS");
    assert_eq!(psbt_type_tostr(2, PsbtScope::Outputs), "OUT_BIP32_DERIVATION");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Inputs), "UNKNOWN_INPUT_TYPE");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Global), "UNKNOWN_GLOBAL_TYPE");
    assert_eq!(psbt_type_tostr(99, PsbtScope::Outputs), "UNKNOWN_OUTPUT_TYPE");
}

#[test]
fn test_global_type_tostr() {
    assert_eq!(psbt_global_type_tostr(PsbtGlobalType::UnsignedTx), "GLOBAL_UNSIGNED_TX");
}

#[test]
fn test_output_type_tostr() {
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::RedeemScript), "OUT_REDEEM_SCRIPT");
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::WitnessScript), "OUT_WITNESS_SCRIPT");
    assert_eq!(psbt_output_type_tostr(PsbtOutputType::Bip32Derivation), "OUT_BIP32_DERIVATION");
}

#[test]
fn test_input_type_tostr() {
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
fn test_txelem_type_tostr() {
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::Tx), "TX");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxIn), "TXIN");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::TxOut), "TXOUT");
    assert_eq!(psbt_txelem_type_tostr(PsbtTxElemType::WitnessItem), "WITNESS_ITEM");
}

#[test]
fn test_psbt_init() {
    let mut psbt = Psbt::new(0);
    let mut buf = [0u8; 1024];
    let res = psbt_init(&mut psbt, &mut buf, 1024);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Init);
    assert_eq!(psbt_size(&psbt), 0);
    assert_eq!(psbt.data_capacity, 1024);
}

#[test]
fn test_write_input_before_global_returns_invalid_state() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: vec![1, 2, 3],
        scope: PsbtScope::Inputs,
    };
    let res = psbt_write_input_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_full_write_flow_matches_c() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    // Write global record
    let rec = PsbtRecord {
        record_type: PsbtGlobalType::UnsignedTx as u8,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    let res = psbt_write_global_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Global);
    assert_eq!(psbt_size(&psbt), 132);

    // Write input record
    let rec = PsbtRecord {
        record_type: PsbtInputType::RedeemScript as u8,
        key: vec![],
        val: REDEEM_SCRIPT_A.to_vec(),
        scope: PsbtScope::Inputs,
    };
    let res = psbt_write_input_record(&mut psbt, &rec);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Inputs);
    assert_eq!(psbt_size(&psbt), 207);

    // New output set
    let res = psbt_new_output_record_set(&mut psbt);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::OutputsNew);
    assert_eq!(psbt_size(&psbt), 208);

    // print before finalize -> InvalidState
    let mut sink = Vec::new();
    let res = psbt_print(&psbt, &mut sink);
    assert_eq!(res, PsbtResult::InvalidState);

    // Finalize
    let res = psbt_finalize(&mut psbt);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Finalized);
    assert_eq!(psbt_size(&psbt), 209);

    // Print after finalize works
    let mut sink = Vec::new();
    let res = psbt_print(&psbt, &mut sink);
    assert_eq!(res, PsbtResult::Ok);
    // 209 bytes * 2 hex chars + newline = 419 bytes
    assert_eq!(sink.len(), 419);
}

#[test]
fn test_encode_hex_after_finalize() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);

    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: REDEEM_SCRIPT_A.to_vec(),
        scope: PsbtScope::Inputs,
    };
    psbt_write_input_record(&mut psbt, &rec);
    psbt_new_output_record_set(&mut psbt);
    psbt_finalize(&mut psbt);

    let mut out = vec![0u8; 4096];
    let mut out_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut out, 4096, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    // psbt_size = 209, hex: 418 + 1 nul = 419
    assert_eq!(out_len, 419);
    let expected_prefix = b"70736274ff01007c";
    assert_eq!(&out[..expected_prefix.len()], expected_prefix);
    // Last 4 hex chars before nul should be "0000"
    assert_eq!(&out[414..418], b"0000");
    assert_eq!(out[418], 0); // nul terminator
}

#[test]
fn test_encode_base64_after_finalize() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);
    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: REDEEM_SCRIPT_A.to_vec(),
        scope: PsbtScope::Inputs,
    };
    psbt_write_input_record(&mut psbt, &rec);
    psbt_new_output_record_set(&mut psbt);
    psbt_finalize(&mut psbt);

    let mut out = vec![0u8; 4096];
    let mut out_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Base64, &mut out, 4096, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 280);
    let expected_prefix = b"cHNidP8BAHwC";
    assert_eq!(&out[..expected_prefix.len()], expected_prefix);
}

#[test]
fn test_encode_before_finalize_fails() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let mut out = vec![0u8; 4096];
    let mut out_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut out, 4096, &mut out_len);
    assert_eq!(res, PsbtResult::WriteError);
}

#[test]
fn test_psbt_decode_hex() {
    let hex = "deadbeef";
    let mut dest = [0u8; 16];
    let mut plen = 0;
    let res = psbt_decode(hex, hex.len(), &mut dest, 16, &mut plen);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(plen, 4);
    assert_eq!(&dest[..4], &[0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn test_psbt_decode_invalid_hex() {
    let hex = "deadbeeg"; // 'g' is not a valid hex digit
    let mut dest = [0u8; 16];
    let mut plen = 0;
    let res = psbt_decode(hex, hex.len(), &mut dest, 16, &mut plen);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_decode_odd_length_hex() {
    let hex = "abc";
    let mut dest = [0u8; 16];
    let mut plen = 0;
    let res = psbt_decode(hex, hex.len(), &mut dest, 16, &mut plen);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_decode_too_small_dest() {
    let hex = "deadbeef";
    let mut dest = [0u8; 2];
    let mut plen = 0;
    let res = psbt_decode(hex, hex.len(), &mut dest, 2, &mut plen);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_decode_too_small_src() {
    let s = "ab";
    let mut dest = [0u8; 16];
    let mut plen = 0;
    let res = psbt_decode(s, s.len(), &mut dest, 16, &mut plen);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_psbt_decode_base64() {
    // base64 prefix "cHNid" must match
    // encode known small bytes: "psbt" maps to 0x70 0x73 0x62 0x74; "psbtfoo"
    // We use the base64 of "psbtttt" = "cHNidHR0dA==" which has cHNid prefix
    let s = "cHNidHR0dA==";
    let mut dest = [0u8; 32];
    let mut plen = 0;
    let res = psbt_decode(s, s.len(), &mut dest, 32, &mut plen);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(plen, 7);
    assert_eq!(&dest[..7], b"psbtttt");
}

#[test]
fn test_psbt_geterr_returns_psbt_error() {
    assert_eq!(psbt_geterr(), "psbt error");
}

#[test]
fn test_round_trip_read_after_decode() {
    // Build a PSBT, encode to hex, decode, then read back
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);
    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: REDEEM_SCRIPT_A.to_vec(),
        scope: PsbtScope::Inputs,
    };
    psbt_write_input_record(&mut psbt, &rec);
    psbt_new_output_record_set(&mut psbt);
    psbt_finalize(&mut psbt);

    let mut hex_out = vec![0u8; 4096];
    let mut hex_len = 0;
    let res = psbt_encode(&psbt, PsbtEncoding::Hex, &mut hex_out, 4096, &mut hex_len);
    assert_eq!(res, PsbtResult::Ok);
    // remove nul
    let hex_str = std::str::from_utf8(&hex_out[..hex_len - 1]).unwrap();

    let mut decoded = vec![0u8; 2048];
    let mut decoded_len = 0;
    let res = psbt_decode(hex_str, hex_str.len(), &mut decoded, 2048, &mut decoded_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(decoded_len, 209);

    let mut psbt2 = Psbt::new(2048);
    let mut intbuf = vec![0u8; 2048];
    psbt_init(&mut psbt2, &mut intbuf, 2048);

    let mut nothing: i32 = 0;
    let res = psbt_read(&decoded, decoded_len, &mut psbt2, None, &mut nothing);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt2.state, PsbtState::Finalized);
}

#[test]
fn test_read_invalid_state_if_not_init() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);

    // psbt is no longer in Init
    let mut nothing: i32 = 0;
    let res = psbt_read(&[], 0, &mut psbt, None, &mut nothing);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_read_invalid_magic() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let bad = [0x00u8, 0x01, 0x02, 0x03, 0xff, 0x00, 0x00];
    let mut nothing: i32 = 0;
    let res = psbt_read(&bad, bad.len(), &mut psbt, None, &mut nothing);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_finalize_invalid_state() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);
    let res = psbt_finalize(&mut psbt);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_print_before_finalize() {
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let mut sink = Vec::new();
    let res = psbt_print(&psbt, &mut sink);
    assert_eq!(res, PsbtResult::InvalidState);
}

#[test]
fn test_encode_raw_hex() {
    let data = [0xdeu8, 0xad, 0xbe, 0xef];
    let mut out = [0u8; 32];
    let mut out_len = 0;
    let res = psbt_encode_raw(&data, 4, PsbtEncoding::Hex, &mut out, 32, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 9); // 4*2 + nul = 9
    assert_eq!(&out[..8], b"deadbeef");
    assert_eq!(out[8], 0);
}

#[test]
fn test_encode_raw_base64() {
    let data = b"hello";
    let mut out = [0u8; 32];
    let mut out_len = 0;
    let res = psbt_encode_raw(data, 5, PsbtEncoding::Base64, &mut out, 32, &mut out_len);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(out_len, 8);
    assert_eq!(&out[..8], b"aGVsbG8=");
}

#[test]
fn test_encode_raw_protobuf_returns_not_implemented() {
    let data = [0u8; 4];
    let mut out = [0u8; 32];
    let mut out_len = 0;
    let res = psbt_encode_raw(&data, 4, PsbtEncoding::Protobuf, &mut out, 32, &mut out_len);
    assert_eq!(res, PsbtResult::NotImplemented);
}

#[test]
fn test_read_with_callback() {
    // Build a finalized PSBT, then read it with a record-counter handler.
    let mut psbt = Psbt::new(1024);
    let mut buf = [0u8; 1024];
    psbt_init(&mut psbt, &mut buf, 1024);

    let rec = PsbtRecord {
        record_type: 0,
        key: vec![],
        val: TRANSACTION.to_vec(),
        scope: PsbtScope::Global,
    };
    psbt_write_global_record(&mut psbt, &rec);
    let rec = PsbtRecord {
        record_type: 4,
        key: vec![],
        val: REDEEM_SCRIPT_A.to_vec(),
        scope: PsbtScope::Inputs,
    };
    psbt_write_input_record(&mut psbt, &rec);
    psbt_new_output_record_set(&mut psbt);
    psbt_finalize(&mut psbt);

    let raw_data: Vec<u8> = psbt.data.clone();
    let raw_len = raw_data.len();

    let mut psbt2 = Psbt::new(2048);
    let mut intbuf = vec![0u8; 2048];
    psbt_init(&mut psbt2, &mut intbuf, 2048);

    fn counter_handler(elem: &mut PsbtElem, ud: &mut dyn std::any::Any) {
        let count = ud.downcast_mut::<i32>().unwrap();
        if let PsbtElem::Record { .. } = elem {
            *count += 1;
        }
    }

    let mut count: i32 = 0;
    let res = psbt_read(&raw_data, raw_len, &mut psbt2, Some(counter_handler), &mut count);
    assert_eq!(res, PsbtResult::Ok);
    // Two records: 1 global, 1 input
    assert_eq!(count, 2);
}

fn main() {}
