use amp::amp::{amp_encode, Amp, AMP_VERSION};

fn make_msg() -> Amp {
    Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    }
}

#[test]
fn test_constant_amp_version() {
    assert_eq!(AMP_VERSION, 1);
}

#[test]
fn test_encode_three_args() {
    // Expected from running the C reference:
    //   bytes: 13 00 00 00 04 73 6f 6d 65 00 00 00 05 73 74 75 66 66 00 00 00 04 68 65 72 65
    let buf = amp_encode(&["some", "stuff", "here"]);
    let expected: Vec<u8> = vec![
        0x13, 0x00, 0x00, 0x00, 0x04, b's', b'o', b'm', b'e', 0x00, 0x00, 0x00, 0x05, b's', b't',
        b'u', b'f', b'f', 0x00, 0x00, 0x00, 0x04, b'h', b'e', b'r', b'e',
    ];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 26);
}

#[test]
fn test_encode_single_arg() {
    // C reference output: 11 00 00 00 01 61
    let buf = amp_encode(&["a"]);
    let expected: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x01, b'a'];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 6);
}

#[test]
fn test_encode_with_empty_arg() {
    // C reference output: 12 00 00 00 00 00 00 00 01 78
    let buf = amp_encode(&["", "x"]);
    let expected: Vec<u8> = vec![
        0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, b'x',
    ];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 10);
}

#[test]
fn test_encode_two_args_hello_world() {
    // C reference output:
    // 12 00 00 00 05 68 65 6c 6c 6f 00 00 00 05 77 6f 72 6c 64
    let buf = amp_encode(&["hello", "world"]);
    let expected: Vec<u8> = vec![
        0x12, 0x00, 0x00, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o', 0x00, 0x00, 0x00, 0x05, b'w',
        b'o', b'r', b'l', b'd',
    ];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 19);
}

#[test]
fn test_encode_long_arg() {
    // length 26 (0x1a)
    let buf = amp_encode(&["Lorem ipsum dolor sit amet"]);
    let mut expected: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x1a];
    expected.extend_from_slice(b"Lorem ipsum dolor sit amet");
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 31);
}

#[test]
fn test_encode_four_args() {
    // C reference output:
    // 14 00 00 00 03 61 62 63 00 00 00 05 64 65 66 67 68
    //    00 00 00 02 69 6a 00 00 00 06 6b 6c 6d 6e 6f 70
    let buf = amp_encode(&["abc", "defgh", "ij", "klmnop"]);
    let expected: Vec<u8> = vec![
        0x14, 0x00, 0x00, 0x00, 0x03, b'a', b'b', b'c', 0x00, 0x00, 0x00, 0x05, b'd', b'e', b'f',
        b'g', b'h', 0x00, 0x00, 0x00, 0x02, b'i', b'j', 0x00, 0x00, 0x00, 0x06, b'k', b'l', b'm',
        b'n', b'o', b'p',
    ];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 33);
}

#[test]
fn test_encode_zero_args() {
    // C reference output: 10  (just the version/argc header)
    let argv: [&str; 0] = [];
    let buf = amp_encode(&argv);
    let expected: Vec<u8> = vec![0x10];
    assert_eq!(buf.as_bytes(), expected.as_slice());
    assert_eq!(buf.len(), 1);
}

#[test]
fn test_decode_header_three_args() {
    // Build encoded buffer the same way C does, then decode header.
    let buf = amp_encode(&["some", "stuff", "here"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
    // After decoding the header, msg.buf is the body (everything past byte 0).
    // It must therefore be 25 bytes (26 - 1).
    assert_eq!(msg.buf.len(), 25);
}

#[test]
fn test_decode_header_single_arg() {
    let buf = amp_encode(&["a"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
    assert_eq!(msg.buf.len(), 5);
}

#[test]
fn test_decode_header_zero_args() {
    let argv: [&str; 0] = [];
    let buf = amp_encode(&argv);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 0);
    assert_eq!(msg.buf.len(), 0);
}

#[test]
fn test_decode_arg_three_args_some_stuff_here() {
    let buf = amp_encode(&["some", "stuff", "here"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    let a0 = msg.decode_arg().to_string();
    assert_eq!(a0, "some");
    let a1 = msg.decode_arg().to_string();
    assert_eq!(a1, "stuff");
    let a2 = msg.decode_arg().to_string();
    assert_eq!(a2, "here");
}

#[test]
fn test_decode_arg_single() {
    let buf = amp_encode(&["a"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.argc, 1);
    let a = msg.decode_arg().to_string();
    assert_eq!(a, "a");
}

#[test]
fn test_decode_arg_empty_first() {
    let buf = amp_encode(&["", "x"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 2);
    let a0 = msg.decode_arg().to_string();
    assert_eq!(a0, "");
    let a1 = msg.decode_arg().to_string();
    assert_eq!(a1, "x");
}

#[test]
fn test_decode_arg_long() {
    let buf = amp_encode(&["Lorem ipsum dolor sit amet"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
    let a = msg.decode_arg().to_string();
    assert_eq!(a, "Lorem ipsum dolor sit amet");
}

#[test]
fn test_decode_arg_four_args() {
    let buf = amp_encode(&["abc", "defgh", "ij", "klmnop"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 4);
    let a0 = msg.decode_arg().to_string();
    assert_eq!(a0, "abc");
    let a1 = msg.decode_arg().to_string();
    assert_eq!(a1, "defgh");
    let a2 = msg.decode_arg().to_string();
    assert_eq!(a2, "ij");
    let a3 = msg.decode_arg().to_string();
    assert_eq!(a3, "klmnop");
}

#[test]
fn test_round_trip_hello_world() {
    let buf = amp_encode(&["hello", "world"]);
    let mut msg = make_msg();
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 2);
    assert_eq!(msg.decode_arg().to_string(), "hello");
    assert_eq!(msg.decode_arg().to_string(), "world");
}

fn main() {}
