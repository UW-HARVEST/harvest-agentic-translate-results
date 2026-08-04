#![allow(non_snake_case)]
#![allow(unused_imports)]
use libbase122::base122::{decode, encode, BitReader, BitWriter};

#[test]
fn test_bitreader_one_byte_all_ones() {
    // Input: 0xFF = "11111111"
    let input = [0xFFu8];
    let mut reader = BitReader::new(&input);
    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b0111_1111); // "01111111"

    let (n, v) = reader.read(7);
    assert_eq!(n, 1);
    assert_eq!(v, 0b0000_0001); // "00000001"

    let (n, v) = reader.read(7);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

#[test]
fn test_bitreader_two_bytes() {
    // "10101010 11111111"
    let input = [0xAAu8, 0xFFu8];
    let mut reader = BitReader::new(&input);

    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b0101_0101); // "01010101"

    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b0011_1111); // "00111111"

    let (n, v) = reader.read(7);
    assert_eq!(n, 2);
    assert_eq!(v, 0b0000_0011); // "00000011"
}

#[test]
fn test_bitreader_initial_state() {
    let input = [0u8];
    let reader = BitReader::new(&input);
    assert_eq!(reader.byte_pos, 0);
    assert_eq!(reader.bit_pos, 0);
    assert_eq!(reader.input.len(), 1);
}

#[test]
fn test_bitwriter_one_byte() {
    let mut buf = [0u8; 1];
    let buf_len = buf.len();
    let mut writer = BitWriter::new(Some(&mut buf[..]), buf_len);
    let inp = 0b0000_1111u8; // last bit = 1

    let res = writer.write(1, inp);
    assert!(res.is_ok());
    let res = writer.write(1, inp);
    assert!(res.is_ok());
    let res = writer.write(5, inp);
    assert!(res.is_ok());

    // Now the buffer should be "11011110"
    assert_eq!(buf[0], 0b1101_1110);

    // Trying to write 5 more bits exceeds capacity (only 1 bit left)
    let mut writer2 = BitWriter::new(Some(&mut buf[..]), buf_len);
    writer2.write(1, inp).unwrap();
    writer2.write(1, inp).unwrap();
    writer2.write(5, inp).unwrap();
    let res = writer2.write(5, inp);
    assert!(res.is_err());
}

#[test]
fn test_bitwriter_two_bytes() {
    let mut buf = [0u8; 2];
    let buf_len = buf.len();
    {
        let mut writer = BitWriter::new(Some(&mut buf[..]), buf_len);
        writer.write(1, 0xFF).unwrap();
    }
    assert_eq!(buf, [0b1000_0000, 0b0000_0000]);

    let mut buf = [0u8; 2];
    let buf_len = buf.len();
    let res_buf;
    {
        let mut writer = BitWriter::new(Some(&mut buf[..]), buf_len);
        writer.write(1, 0xFF).unwrap();
        writer.write(8, 0x0F).unwrap();
        // Expected: "10000111 10000000"
        writer.write(1, 0x0).unwrap();
        writer.write(1, 0xFF).unwrap();
        writer.write(5, 0xFF).unwrap();
        let res = writer.write(1, 0xFF);
        res_buf = res;
    }
    assert!(res_buf.is_err());
    assert_eq!(buf, [0b1000_0111, 0b1011_1111]);

    // Now check intermediate states with separate writers
    let mut buf2 = [0u8; 2];
    let buf2_len = buf2.len();
    {
        let mut writer = BitWriter::new(Some(&mut buf2[..]), buf2_len);
        writer.write(1, 0xFF).unwrap();
        writer.write(8, 0x0F).unwrap();
    }
    assert_eq!(buf2, [0b1000_0111, 0b1000_0000]);

    let mut buf3 = [0u8; 2];
    let buf3_len = buf3.len();
    {
        let mut writer = BitWriter::new(Some(&mut buf3[..]), buf3_len);
        writer.write(1, 0xFF).unwrap();
        writer.write(8, 0x0F).unwrap();
        writer.write(1, 0x0).unwrap();
        writer.write(1, 0xFF).unwrap();
    }
    assert_eq!(buf3, [0b1000_0111, 0b1010_0000]);
}

#[test]
fn test_bitwriter_count_only() {
    let mut writer = BitWriter::new(None, 0);
    assert!(writer.count_only);
    writer.write(7, 0xFF).unwrap();
    writer.write(7, 0xFF).unwrap();
    assert_eq!(writer.cur_bit, 14);
}

#[test]
fn test_encode_empty() {
    let result = encode(&[]).unwrap();
    assert_eq!(result.len(), 0);
    assert_eq!(result, Vec::<u8>::new());
}

#[test]
fn test_encode_one_byte_ff() {
    // C: 1byte_0xFF: encoded[2]=7F40
    let result = encode(&[0xFF]).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result, vec![0x7F, 0x40]);
}

#[test]
fn test_encode_alternating_aa() {
    // C: AAAAAAAA: encoded[5]=552A552A50
    let result = encode(&[0xAA, 0xAA, 0xAA, 0xAA]).unwrap();
    assert_eq!(result, vec![0x55, 0x2A, 0x55, 0x2A, 0x50]);
}

#[test]
fn test_encode_with_illegal_zero() {
    // C: 0xFF_with_0x00: encoded[3]=C2BF60
    let result = encode(&[0x00, 0xFF]).unwrap();
    assert_eq!(result, vec![0xC2, 0xBF, 0x60]);
}

#[test]
fn test_encode_hello() {
    // C: hello: encoded[6]=34192D46633C
    let result = encode(b"hello").unwrap();
    assert_eq!(result, vec![0x34, 0x19, 0x2D, 0x46, 0x63, 0x3C]);
}

#[test]
fn test_encode_hello_world() {
    // C: HelloWorld: encoded[15]=24192D46633C58202B5B6ED3A31042
    let result = encode(b"Hello, World!").unwrap();
    assert_eq!(
        result,
        vec![
            0x24, 0x19, 0x2D, 0x46, 0x63, 0x3C, 0x58, 0x20, 0x2B, 0x5B, 0x6E, 0xD3, 0xA3, 0x10,
            0x42
        ]
    );
}

#[test]
fn test_encode_all_ones_various_lengths() {
    // From C output:
    // all1s_1: 7F40
    // all1s_2: 7F7F60
    // all1s_3: 7F7F7F70
    // all1s_4: 7F7F7F7F78
    // all1s_5: 7F7F7F7F7F7C
    // all1s_6: 7F7F7F7F7F7F7E
    // all1s_7: 7F7F7F7F7F7F7F7F
    // all1s_8: 7F7F7F7F7F7F7F7F7F40
    let cases: Vec<(usize, Vec<u8>)> = vec![
        (0, vec![]),
        (1, vec![0x7F, 0x40]),
        (2, vec![0x7F, 0x7F, 0x60]),
        (3, vec![0x7F, 0x7F, 0x7F, 0x70]),
        (4, vec![0x7F, 0x7F, 0x7F, 0x7F, 0x78]),
        (5, vec![0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7C]),
        (6, vec![0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7E]),
        (7, vec![0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F]),
        (
            8,
            vec![0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x40],
        ),
    ];
    for (n, expected) in cases {
        let input = vec![0xFFu8; n];
        let result = encode(&input).unwrap();
        assert_eq!(result, expected, "all1s_{}", n);
    }
}

#[test]
fn test_decode_round_trip_one_ff() {
    // C: decode_7F40 -> FF
    let result = decode(&[0x7F, 0x40]).unwrap();
    assert_eq!(result, vec![0xFF]);
}

#[test]
fn test_decode_with_illegal_zero() {
    // C: decode_C2BF60 -> 00FF
    let result = decode(&[0xC2, 0xBF, 0x60]).unwrap();
    assert_eq!(result, vec![0x00, 0xFF]);
}

#[test]
fn test_decode_C780() {
    // C: decode_C780 -> 15
    let result = decode(&[0xC7, 0x80]).unwrap();
    assert_eq!(result, vec![0x15]);
}

#[test]
fn test_decode_7F7FDE80() {
    // C: decode_7F7FDE80 -> FFFC
    let result = decode(&[0x7F, 0x7F, 0xDE, 0x80]).unwrap();
    assert_eq!(result, vec![0xFF, 0xFC]);
}

#[test]
fn test_decode_specified_examples() {
    // From C test.c:
    // {.encoded = "01111111 11011110 10000000", .expect = "11111110", .decodedLen = 1}
    // 0x7F 0xDE 0x80 -> 0xFE
    let result = decode(&[0x7F, 0xDE, 0x80]).unwrap();
    assert_eq!(result, vec![0xFE]);

    // {.encoded = "00000000 11011110 10000000", .expect = "00000000", .decodedLen = 1}
    // 0x00 0xDE 0x80 -> 0x00
    let result = decode(&[0x00, 0xDE, 0x80]).unwrap();
    assert_eq!(result, vec![0x00]);

    // {.encoded = "00000000 11000010 10000000", .expect = "00000000 00000000", .decodedLen = 2}
    // 0x00 0xC2 0x80 -> 0x00 0x00
    let result = decode(&[0x00, 0xC2, 0x80]).unwrap();
    assert_eq!(result, vec![0x00, 0x00]);

    // {.encoded = "11001111 10000001 01100000", .expect = "01000101 00000111", .decodedLen = 2}
    // 0xCF 0x81 0x60 -> 0x45 0x07
    let result = decode(&[0xCF, 0x81, 0x60]).unwrap();
    assert_eq!(result, vec![0x45, 0x07]);
}

#[test]
fn test_decode_error_first_byte_malformed() {
    // 11111111 -> "First byte of two byte sequence malformed"
    let res = decode(&[0xFF]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("First byte of two byte sequence malformed"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_missing_second_byte() {
    // 11011110 -> "Two byte sequence is missing second byte"
    let res = decode(&[0xDE]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("Two byte sequence is missing second byte"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_second_byte_malformed() {
    // 11011110 11111111 -> "Second byte of two byte sequence malformed"
    let res = decode(&[0xDE, 0xFF]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("Second byte of two byte sequence malformed"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_unexpected_extra() {
    // 11011110 10111111 01111111 -> "Got unexpected extra data after shortened two byte sequence"
    let res = decode(&[0xDE, 0xBF, 0x7F]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message
            .contains("Got unexpected extra data after shortened two byte sequence"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_unrecognized_illegal() {
    // 11011010 10111111 -> "Got unrecognized illegal index"
    let res = decode(&[0xDA, 0xBF]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("Got unrecognized illegal index"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_not_byte_multiple() {
    // 01111111 -> "Decoded data is not a byte multiple"
    let res = decode(&[0x7F]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("Decoded data is not a byte multiple"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_extra_data_in_last_byte() {
    // 01111111 01111111 -> "Encoded data is malformed. Last byte has extra data."
    let res = decode(&[0x7F, 0x7F]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message
            .contains("Encoded data is malformed. Last byte has extra data."),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_extra_in_two_byte_seq() {
    // 11011110 10000000 -> "Decoded data is not a byte multiple"
    let res = decode(&[0xDE, 0x80]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message.contains("Decoded data is not a byte multiple"),
        "got: {}",
        err.message
    );
}

#[test]
fn test_decode_error_extra_after_two_byte() {
    // 01111111 11011111 10100000 -> "Encoded data is malformed. Last byte has extra data."
    let res = decode(&[0x7F, 0xDF, 0xA0]);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.message
            .contains("Encoded data is malformed. Last byte has extra data."),
        "got: {}",
        err.message
    );
}

#[test]
fn test_round_trip_all_ones_lengths_0_to_65() {
    for n in 0..=65 {
        let input = vec![0xFFu8; n];
        let encoded = encode(&input).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, input, "round trip failed for length {}", n);
    }
}

#[test]
fn test_round_trip_specific_cases() {
    // From C test.c roundtrip_test_t:
    // "one byte": data = 0xFF, encoded = 0x7F 0x40
    let data = vec![0xFFu8];
    let encoded = encode(&data).unwrap();
    assert_eq!(encoded, vec![0x7F, 0x40]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);

    // "several bytes":
    // data = "10101010 10101010 10101010 10101010" -> 0xAA 0xAA 0xAA 0xAA
    // encoded = "01010101 00101010 01010101 00101010 01010000"
    //         = 0x55 0x2A 0x55 0x2A 0x50
    let data = vec![0xAAu8; 4];
    let encoded = encode(&data).unwrap();
    assert_eq!(encoded, vec![0x55, 0x2A, 0x55, 0x2A, 0x50]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);

    // "illegal one byte":
    // data = "00000000 11111111" -> 0x00 0xFF
    // encoded = "11000010 10111111 01100000" -> 0xC2 0xBF 0x60
    let data = vec![0x00u8, 0xFF];
    let encoded = encode(&data).unwrap();
    assert_eq!(encoded, vec![0xC2, 0xBF, 0x60]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);

    // "fuzz crash 1": data = "00010101" -> 0x15, encoded = 0xC7 0x80
    let data = vec![0x15u8];
    let encoded = encode(&data).unwrap();
    assert_eq!(encoded, vec![0xC7, 0x80]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_encode_byte_sequence_then_decode() {
    // C: rt_5bytes encoded[6]=09CA8A674468
    let data = [0x12u8, 0x34, 0x56, 0x78, 0x9A];
    let encoded = encode(&data).unwrap();
    assert_eq!(encoded, vec![0x09, 0xCA, 0x8A, 0x67, 0x44, 0x68]);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data.to_vec());
}

fn main() {}
