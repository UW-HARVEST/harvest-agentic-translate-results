use amp::amp::{amp_encode, Amp, AMP_VERSION};

#[test]
fn test_amp_version_constant() {
    // The protocol version constant must be 1.
    assert_eq!(AMP_VERSION, 1);
}

#[test]
fn test_encode_three_args() {
    // Ground truth from running C:
    // 3args len=26 bytes=[19,0,0,0,4,115,111,109,101,0,0,0,5,115,116,117,102,102,0,0,0,4,104,101,114,101]
    let args = ["some", "stuff", "here"];
    let buf = amp_encode(&args);
    let bytes = buf.as_bytes();
    let expected: &[u8] = &[
        19, 0, 0, 0, 4, b's', b'o', b'm', b'e', 0, 0, 0, 5, b's', b't', b'u', b'f', b'f', 0, 0, 0,
        4, b'h', b'e', b'r', b'e',
    ];
    assert_eq!(bytes.len(), 26);
    assert_eq!(bytes, expected);
}

#[test]
fn test_encode_one_arg() {
    // Ground truth: 1arg len=6 bytes=[17,0,0,0,1,97]
    let args = ["a"];
    let buf = amp_encode(&args);
    let bytes = buf.as_bytes();
    let expected: &[u8] = &[17, 0, 0, 0, 1, b'a'];
    assert_eq!(bytes.len(), 6);
    assert_eq!(bytes, expected);
}

#[test]
fn test_encode_empty_arg() {
    // Ground truth: empty len=5 bytes=[17,0,0,0,0]
    let args = [""];
    let buf = amp_encode(&args);
    let bytes = buf.as_bytes();
    let expected: &[u8] = &[17, 0, 0, 0, 0];
    assert_eq!(bytes.len(), 5);
    assert_eq!(bytes, expected);
}

#[test]
fn test_encode_zero_args() {
    // Ground truth: 0args len=1 bytes=[16]
    let args: [&str; 0] = [];
    let buf = amp_encode(&args);
    let bytes = buf.as_bytes();
    let expected: &[u8] = &[16];
    assert_eq!(bytes.len(), 1);
    assert_eq!(bytes, expected);
}

#[test]
fn test_encode_two_args_hello_world() {
    // Ground truth: hello-world len=19 bytes=[18,0,0,0,5,104,101,108,108,111,0,0,0,5,119,111,114,108,100]
    let args = ["hello", "world"];
    let buf = amp_encode(&args);
    let bytes = buf.as_bytes();
    let expected: &[u8] = &[
        18, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o', 0, 0, 0, 5, b'w', b'o', b'r', b'l', b'd',
    ];
    assert_eq!(bytes.len(), 19);
    assert_eq!(bytes, expected);
}

#[test]
fn test_decode_header_three_args() {
    // Encode three args, decode header, and verify version + argc.
    let args = ["some", "stuff", "here"];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);

    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
    // After decoding the header, msg.buf should be the original buf without the first byte.
    assert_eq!(msg.buf.as_bytes(), &buf.as_bytes()[1..]);
}

#[test]
fn test_decode_header_two_args() {
    let args = ["hello", "world"];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);

    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 2);
    assert_eq!(msg.buf.as_bytes(), &buf.as_bytes()[1..]);
}

#[test]
fn test_decode_header_zero_args() {
    let args: [&str; 0] = [];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);

    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 0);
    // No payload after header.
    assert_eq!(msg.buf.len(), 0);
}

#[test]
fn test_decode_args_three() {
    // Mirror the C test in tests/test.c:
    //   args = { "some", "stuff", "here" };
    //   amp_decode -> version=1, argc=3
    //   amp_decode_arg returns each arg in order.
    let args = ["some", "stuff", "here"];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    let arg0 = msg.decode_arg().to_string();
    assert_eq!(arg0, "some");

    let arg1 = msg.decode_arg().to_string();
    assert_eq!(arg1, "stuff");

    let arg2 = msg.decode_arg().to_string();
    assert_eq!(arg2, "here");
}

#[test]
fn test_decode_args_two() {
    let args = ["hello", "world"];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 2);

    let arg0 = msg.decode_arg().to_string();
    assert_eq!(arg0, "hello");

    let arg1 = msg.decode_arg().to_string();
    assert_eq!(arg1, "world");
}

#[test]
fn test_decode_arg_single_byte() {
    let args = ["a"];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);

    let arg0 = msg.decode_arg().to_string();
    assert_eq!(arg0, "a");
}

#[test]
fn test_decode_arg_empty_string() {
    // Encoding a single empty arg: header byte 17, 4 zero length bytes.
    let args = [""];
    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);

    let arg0 = msg.decode_arg().to_string();
    assert_eq!(arg0, "");
}

#[test]
fn test_encode_decode_round_trip_many_args() {
    // The header low nibble holds argc in 4 bits, so up to 15 args are valid.
    let args = [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron",
    ];
    assert_eq!(args.len(), 15);

    let buf = amp_encode(&args);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 15);

    for expected in args.iter() {
        let got = msg.decode_arg().to_string();
        assert_eq!(&got, expected);
    }
}

#[test]
fn test_encode_decode_round_trip_long_arg() {
    // Use a longer string to make sure the 4-byte length is encoded correctly.
    let long_arg: String = "x".repeat(300);
    let args = [long_arg.as_str(), "tail"];

    let buf = amp_encode(&args);
    // Sanity-check the header and the length prefix of the first arg.
    let bytes = buf.as_bytes();
    assert_eq!(bytes[0], (1u8 << 4) | 2);
    // 300 = 0x12C, so big-endian 4 bytes are [0, 0, 1, 44].
    assert_eq!(&bytes[1..5], &[0, 0, 1, 44]);

    let mut msg = Amp {
        version: 0,
        argc: 0,
        buf: String::new(),
    };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 2);

    let arg0 = msg.decode_arg().to_string();
    assert_eq!(arg0.len(), 300);
    assert_eq!(arg0, long_arg);

    let arg1 = msg.decode_arg().to_string();
    assert_eq!(arg1, "tail");
}

#[test]
fn test_encode_total_length_is_correct() {
    // From C: encoded length = 1 (header) + sum_i (4 + len(arg_i)).
    let args = ["abc", "defgh", "ij"];
    let buf = amp_encode(&args);
    let expected_len = 1 + (4 + 3) + (4 + 5) + (4 + 2);
    assert_eq!(buf.as_bytes().len(), expected_len);
}

#[test]
fn test_header_byte_layout() {
    // Header byte: (AMP_VERSION << 4) | argc.
    // For argc=3 with version=1, header byte = 0x13 = 19.
    let args = ["x", "y", "z"];
    let buf = amp_encode(&args);
    assert_eq!(buf.as_bytes()[0], 0x13);

    // For argc=0, header byte = 0x10 = 16.
    let empty: [&str; 0] = [];
    let buf2 = amp_encode(&empty);
    assert_eq!(buf2.as_bytes()[0], 0x10);
}

fn main() {}
