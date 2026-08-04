#![allow(unused_imports, dead_code)]
use libbase122::base122;

// ---------- helper functions ----------

fn hex_to_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut half: u8 = 0;
    let mut got = 0u32;
    for c in s.chars() {
        let v = match c {
            '0'..='9' => (c as u8) - b'0',
            'a'..='f' => (c as u8) - b'a' + 10,
            'A'..='F' => (c as u8) - b'A' + 10,
            ' ' | '\t' | '\n' => continue,
            _ => panic!("bad hex char {}", c),
        };
        half = half * 16 + v;
        got += 1;
        if got == 2 {
            out.push(half);
            half = 0;
            got = 0;
        }
    }
    assert_eq!(got, 0, "expected even hex");
    out
}

fn bits_to_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut cur: u8 = 0;
    let mut got = 0u32;
    for c in s.chars() {
        let v = match c {
            '0' => 0,
            '1' => 1,
            ' ' | '\t' | '\n' => continue,
            _ => panic!("bad bit char"),
        };
        cur = cur * 2 + v;
        got += 1;
        if got == 8 {
            out.push(cur);
            cur = 0;
            got = 0;
        }
    }
    if got != 0 {
        // pad remaining left, or just put what we have
        out.push(cur << (8 - got));
    }
    out
}

// ---------- BitReader tests ----------

#[test]
fn test_bitreader_read_one_byte() {
    // input 11111111
    let input = bits_to_bytes("11111111");
    let mut reader = base122::BitReader::new(&input);

    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b01111111); // top 7 bits of 11111111

    let (n, v) = reader.read(7);
    assert_eq!(n, 1);
    assert_eq!(v, 0b00000001);

    let (n, v) = reader.read(7);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

#[test]
fn test_bitreader_read_two_bytes() {
    // input 10101010 11111111
    let input = bits_to_bytes("10101010 11111111");
    let mut reader = base122::BitReader::new(&input);

    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b01010101);

    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0b00111111);

    let (n, v) = reader.read(7);
    assert_eq!(n, 2);
    assert_eq!(v, 0b00000011);
}

#[test]
fn test_bitreader_position_tracking() {
    let input = vec![0b11001010u8, 0b10101010];
    let mut reader = base122::BitReader::new(&input);
    assert_eq!(reader.byte_pos, 0);
    assert_eq!(reader.bit_pos, 0);

    let (n, _) = reader.read(3);
    assert_eq!(n, 3);
    assert_eq!(reader.byte_pos, 0);
    assert_eq!(reader.bit_pos, 3);

    let (n, _) = reader.read(8);
    assert_eq!(n, 8);
    assert_eq!(reader.byte_pos, 1);
    assert_eq!(reader.bit_pos, 3);

    let (n, _) = reader.read(5);
    assert_eq!(n, 5);
    assert_eq!(reader.byte_pos, 2);
    assert_eq!(reader.bit_pos, 0);
}

#[test]
fn test_bitreader_read_1_bit() {
    let input = vec![0b10100110u8];
    let mut reader = base122::BitReader::new(&input);
    let bits = [1u8, 0, 1, 0, 0, 1, 1, 0];
    for &b in &bits {
        let (n, v) = reader.read(1);
        assert_eq!(n, 1);
        assert_eq!(v, b);
    }
    let (n, v) = reader.read(1);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

#[test]
fn test_bitreader_read_8_bits_aligned() {
    let input = vec![0xABu8, 0xCD];
    let mut reader = base122::BitReader::new(&input);
    let (n, v) = reader.read(8);
    assert_eq!(n, 8);
    assert_eq!(v, 0xAB);
    let (n, v) = reader.read(8);
    assert_eq!(n, 8);
    assert_eq!(v, 0xCD);
    let (n, v) = reader.read(8);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

// ---------- BitWriter tests ----------

#[test]
fn test_bitwriter_one_byte() {
    let mut buf = [0u8; 1];
    let len = buf.len();
    let mut writer = base122::BitWriter::new(Some(&mut buf), len);

    let r = writer.write(1, 0b00001111u8);
    assert!(r.is_ok());
    assert_eq!(writer.cur_bit, 1);

    let r = writer.write(1, 0b00001111u8);
    assert!(r.is_ok());
    assert_eq!(writer.cur_bit, 2);

    let r = writer.write(5, 0b00001111u8);
    assert!(r.is_ok());
    assert_eq!(writer.cur_bit, 7);

    let r = writer.write(5, 0b00001111u8);
    assert!(r.is_err());

    drop(writer);
    // After writing 1, then 1, then 5 bits of 0b1111 (= 01111),
    // final byte should be: 1 1 0 1 1 1 1 0 = 0b11011110
    assert_eq!(buf[0], 0b11011110u8);
}

#[test]
fn test_bitwriter_two_bytes() {
    let mut buf = [0u8; 2];
    let len = buf.len();
    let mut writer = base122::BitWriter::new(Some(&mut buf), len);

    // write 1 bit of 0xFF (= 1)
    writer.write(1, 0xFF).unwrap();
    // write 8 bits of 0x0F
    writer.write(8, 0x0F).unwrap();
    // write 1 bit of 0x0
    writer.write(1, 0x0).unwrap();
    // write 1 bit of 0xFF (= 1)
    writer.write(1, 0xFF).unwrap();
    // write 5 bits of 0xFF (= 11111)
    writer.write(5, 0xFF).unwrap();

    // Should now be at 16 bits
    assert_eq!(writer.cur_bit, 16);

    // Next write of 1 bit should fail (no capacity).
    let r = writer.write(1, 0xFF);
    assert!(r.is_err());

    drop(writer);
    // Expected:
    // start: 00000000 00000000
    // write 1 bit of 1: 10000000 00000000
    // write 8 bits 0x0F (00001111): 10000111 10000000
    // write 1 bit 0: 10000111 10000000
    // write 1 bit 1: 10000111 10100000
    // write 5 bits of 11111: 10000111 10111111
    assert_eq!(buf[0], 0b10000111);
    assert_eq!(buf[1], 0b10111111);
}

#[test]
fn test_bitwriter_count_only() {
    // count_only=true is when output is None
    let mut writer = base122::BitWriter::new(None, 0);
    assert!(writer.count_only);
    writer.write(7, 0).unwrap();
    assert_eq!(writer.cur_bit, 7);
    writer.write(8, 0).unwrap();
    assert_eq!(writer.cur_bit, 15);
    writer.write(1, 0).unwrap();
    assert_eq!(writer.cur_bit, 16);
}

#[test]
fn test_bitwriter_capacity_exceeded() {
    let mut buf = [0u8; 1];
    let len = buf.len();
    let mut writer = base122::BitWriter::new(Some(&mut buf), len);
    writer.write(8, 0xFF).unwrap();
    let r = writer.write(1, 0);
    assert!(r.is_err());
}

// ---------- Encode tests ----------

#[test]
fn test_encode_empty() {
    let out = base122::encode(&[]).unwrap();
    assert_eq!(out, Vec::<u8>::new());
}

#[test]
fn test_encode_single_ff() {
    // C oracle: encode "FF" -> 7F40
    let out = base122::encode(&hex_to_bytes("FF")).unwrap();
    assert_eq!(out, hex_to_bytes("7F40"));
}

#[test]
fn test_encode_single_00() {
    // C oracle: encode "00" -> C280
    let out = base122::encode(&hex_to_bytes("00")).unwrap();
    assert_eq!(out, hex_to_bytes("C280"));
}

#[test]
fn test_encode_single_aa() {
    // C oracle: encode "AA" -> 55DE80
    let out = base122::encode(&hex_to_bytes("AA")).unwrap();
    assert_eq!(out, hex_to_bytes("55DE80"));
}

#[test]
fn test_encode_two_byte_00ff() {
    // C oracle: encode "00FF" -> C2BF60
    let out = base122::encode(&hex_to_bytes("00FF")).unwrap();
    assert_eq!(out, hex_to_bytes("C2BF60"));
}

#[test]
fn test_encode_two_byte_ffff() {
    // C oracle: encode "FFFF" -> 7F7F60
    let out = base122::encode(&hex_to_bytes("FFFF")).unwrap();
    assert_eq!(out, hex_to_bytes("7F7F60"));
}

#[test]
fn test_encode_eight_bytes_seq() {
    // C oracle: encode "0102030405060708" -> C380403020140C0704DE80
    let out = base122::encode(&hex_to_bytes("0102030405060708")).unwrap();
    assert_eq!(out, hex_to_bytes("C380403020140C0704DE80"));
}

#[test]
fn test_encode_aabbccddeeff() {
    // C oracle: encode "AABBCCDDEEFF" -> 552E794D6F3B7E
    let out = base122::encode(&hex_to_bytes("AABBCCDDEEFF")).unwrap();
    assert_eq!(out, hex_to_bytes("552E794D6F3B7E"));
}

#[test]
fn test_encode_illegal_byte_0a() {
    // 0x0A is illegal (newline). C oracle: encode "0A" -> 05DE80
    let out = base122::encode(&hex_to_bytes("0A")).unwrap();
    assert_eq!(out, hex_to_bytes("05DE80"));
}

#[test]
fn test_encode_illegal_byte_22() {
    // 0x22 is illegal (double quote). C oracle: encode "22" -> 11DE80
    let out = base122::encode(&hex_to_bytes("22")).unwrap();
    assert_eq!(out, hex_to_bytes("11DE80"));
}

#[test]
fn test_encode_round_trip_test_cases() {
    // Round-trip cases from C test main()
    // one byte: data 11111111 -> encoded 01111111 01000000
    let data = bits_to_bytes("11111111");
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, bits_to_bytes("01111111 01000000"));

    // several bytes
    let data = bits_to_bytes("10101010 10101010 10101010 10101010");
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(
        encoded,
        bits_to_bytes("01010101 00101010 01010101 00101010 01010000")
    );

    // illegal one byte
    let data = bits_to_bytes("00000000 11111111");
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, bits_to_bytes("11000010 10111111 01100000"));

    // fuzz crash 1: data 00010101 -> 11000111 10000000
    let data = bits_to_bytes("00010101");
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, bits_to_bytes("11000111 10000000"));
}

#[test]
fn test_encode_all_ones_lengths() {
    // C oracle expected encodings for FF*i for i in 0..=10
    // i=0 -> empty
    let cases: &[(usize, &str)] = &[
        (0, ""),
        (1, "7F40"),
        (2, "7F7F60"),
        (3, "7F7F7F70"),
        (4, "7F7F7F7F78"),
        (5, "7F7F7F7F7F7C"),
        (6, "7F7F7F7F7F7F7E"),
        (7, "7F7F7F7F7F7F7F7F"),
        (8, "7F7F7F7F7F7F7F7F7F40"),
        (9, "7F7F7F7F7F7F7F7F7F7F60"),
        (10, "7F7F7F7F7F7F7F7F7F7F7F70"),
    ];
    for &(i, expect_hex) in cases {
        let in_data = vec![0xFFu8; i];
        let got = base122::encode(&in_data).unwrap();
        let expect = hex_to_bytes(expect_hex);
        assert_eq!(got, expect, "all-ones len={} mismatch", i);
    }
}

// ---------- Decode tests ----------

#[test]
fn test_decode_empty() {
    let out = base122::decode(&[]).unwrap();
    assert_eq!(out, Vec::<u8>::new());
}

#[test]
fn test_decode_single() {
    // C oracle: decode 7F40 -> FF
    let out = base122::decode(&hex_to_bytes("7F40")).unwrap();
    assert_eq!(out, hex_to_bytes("FF"));
}

#[test]
fn test_decode_two_bytes() {
    let out = base122::decode(&hex_to_bytes("7F7F60")).unwrap();
    assert_eq!(out, hex_to_bytes("FFFF"));
}

#[test]
fn test_decode_with_illegal() {
    let out = base122::decode(&hex_to_bytes("C2BF60")).unwrap();
    assert_eq!(out, hex_to_bytes("00FF"));
}

#[test]
fn test_decode_zero() {
    let out = base122::decode(&hex_to_bytes("C280")).unwrap();
    assert_eq!(out, hex_to_bytes("00"));
}

#[test]
fn test_decode_aa() {
    let out = base122::decode(&hex_to_bytes("55DE80")).unwrap();
    assert_eq!(out, hex_to_bytes("AA"));
}

#[test]
fn test_decode_test_cases_from_c() {
    // From test.c test_decode tests with valid expected output.
    // 01111111 11011110 10000000 -> 11111110 (1 byte)
    let enc = bits_to_bytes("01111111 11011110 10000000");
    let dec = base122::decode(&enc).unwrap();
    assert_eq!(dec, bits_to_bytes("11111110"));

    // 00000000 11011110 10000000 -> 00000000 (1 byte)
    let enc = bits_to_bytes("00000000 11011110 10000000");
    let dec = base122::decode(&enc).unwrap();
    assert_eq!(dec, bits_to_bytes("00000000"));

    // 00000000 11000010 10000000 -> 00000000 00000000 (2 bytes)
    let enc = bits_to_bytes("00000000 11000010 10000000");
    let dec = base122::decode(&enc).unwrap();
    assert_eq!(dec, bits_to_bytes("00000000 00000000"));

    // 11001111 10000001 01100000 -> 01000101 00000111 (2 bytes)
    let enc = bits_to_bytes("11001111 10000001 01100000");
    let dec = base122::decode(&enc).unwrap();
    assert_eq!(dec, bits_to_bytes("01000101 00000111"));
}

// ---------- Decode error tests ----------

#[test]
fn test_decode_err_last_byte_extra() {
    // 01111111 01111111 01111111
    let enc = bits_to_bytes("01111111 01111111 01111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Last byte has extra data"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_second_byte_malformed() {
    // 11011110 11111111 -> "Second byte of two byte sequence malformed"
    let enc = bits_to_bytes("11011110 11111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Second byte of two byte sequence malformed"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_first_byte_malformed() {
    // 11111111 -> "First byte of two byte sequence malformed"
    let enc = bits_to_bytes("11111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("First byte of two byte sequence malformed"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_missing_second_byte() {
    let enc = bits_to_bytes("11011110");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Two byte sequence is missing second byte"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_unexpected_extra_data() {
    // 11011110 10111111 01111111 -> "Got unexpected extra data after shortened two byte sequence"
    let enc = bits_to_bytes("11011110 10111111 01111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Got unexpected extra data after shortened two byte sequence"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_unrecognized_illegal_index() {
    // 11011010 10111111 -> "Got unrecognized illegal index"
    let enc = bits_to_bytes("11011010 10111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Got unrecognized illegal index"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_not_byte_multiple() {
    // 01111111 -> "Decoded data is not a byte multiple"
    let enc = bits_to_bytes("01111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Decoded data is not a byte multiple"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_last_byte_extra2() {
    let enc = bits_to_bytes("01111111 01111111");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Last byte has extra data"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_last_byte_extra_via_two_byte() {
    // 01111111 11011111 10100000 -> "Encoded data is malformed. Last byte has extra data."
    let enc = bits_to_bytes("01111111 11011111 10100000");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Last byte has extra data"),
        "got: {}",
        msg
    );
}

#[test]
fn test_decode_err_two_byte_not_byte_multiple() {
    // 11011110 10000000 -> "Decoded data is not a byte multiple"
    let enc = bits_to_bytes("11011110 10000000");
    let r = base122::decode(&enc);
    assert!(r.is_err());
    let msg = r.unwrap_err().message;
    assert!(
        msg.contains("Decoded data is not a byte multiple"),
        "got: {}",
        msg
    );
}

// ---------- Round-trip tests ----------

#[test]
fn test_round_trip_all_ones() {
    // Test round-tripping all-1s data of byte length 0..=65
    for i in 0..=65usize {
        let data = vec![0xFFu8; i];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "round-trip failed at len {}", i);
    }
}

#[test]
fn test_round_trip_zeros() {
    for i in 0..=20usize {
        let data = vec![0x00u8; i];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "zero round-trip failed at len {}", i);
    }
}

#[test]
fn test_round_trip_all_byte_values() {
    // Round-trip each single byte value 0..256
    for b in 0u16..256 {
        let data = vec![b as u8];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "byte value {} round-trip failed", b);
    }
}

#[test]
fn test_round_trip_pseudo_random() {
    // Deterministic pseudo-random sequence
    let mut data = Vec::new();
    let mut x: u32 = 0xDEADBEEF;
    for _ in 0..200 {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        data.push((x >> 16) as u8);
    }
    let encoded = base122::encode(&data).unwrap();
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_round_trip_illegal_chars() {
    // Each illegal byte alone
    for &v in &[0u8, 10, 13, 34, 38, 92] {
        let data = vec![v];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "illegal byte {} round-trip failed", v);
    }
    // All illegal bytes together
    let data: Vec<u8> = vec![0, 10, 13, 34, 38, 92];
    let encoded = base122::encode(&data).unwrap();
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

// ---------- Base122Error tests ----------

#[test]
fn test_base122error_display() {
    // Trigger an error and verify Display works.
    let err = base122::decode(&hex_to_bytes("FF")).unwrap_err();
    let s = format!("{}", err);
    assert!(s.contains("First byte of two byte sequence malformed"), "got: {}", s);
}

#[test]
fn test_base122error_clone_debug() {
    let err = base122::decode(&hex_to_bytes("FF")).unwrap_err();
    let cloned = err.clone();
    assert_eq!(cloned.message, err.message);
    let _ = format!("{:?}", err);
}

fn main() {}
