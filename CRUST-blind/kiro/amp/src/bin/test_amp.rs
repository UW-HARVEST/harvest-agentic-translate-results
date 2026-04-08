use amp::amp::{amp_encode, Amp, AMP_VERSION};

#[test]
fn test_amp_version() {
    assert_eq!(AMP_VERSION, 1);
}

#[test]
fn test_encode_three_args() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    // Header: (1 << 4) | 3 = 0x13
    assert_eq!(bytes[0], 0x13);
    // "some" len=4: 00 00 00 04
    assert_eq!(&bytes[1..5], &[0, 0, 0, 4]);
    assert_eq!(&bytes[5..9], b"some");
    // "stuff" len=5: 00 00 00 05
    assert_eq!(&bytes[9..13], &[0, 0, 0, 5]);
    assert_eq!(&bytes[13..18], b"stuff");
    // "here" len=4: 00 00 00 04
    assert_eq!(&bytes[18..22], &[0, 0, 0, 4]);
    assert_eq!(&bytes[22..26], b"here");
    assert_eq!(bytes.len(), 26);
}

#[test]
fn test_encode_single_arg() {
    let encoded = amp_encode(&["hello"]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes[0], 0x11);
    assert_eq!(&bytes[1..5], &[0, 0, 0, 5]);
    assert_eq!(&bytes[5..10], b"hello");
    assert_eq!(bytes.len(), 10);
}

#[test]
fn test_encode_empty_string_arg() {
    let encoded = amp_encode(&[""]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes[0], 0x11);
    assert_eq!(&bytes[1..5], &[0, 0, 0, 0]);
    assert_eq!(bytes.len(), 5);
}

#[test]
fn test_decode_header() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
}

#[test]
fn test_decode_single_arg() {
    let encoded = amp_encode(&["hello"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
    let arg = msg.decode_arg();
    assert_eq!(arg, "hello");
}

#[test]
fn test_decode_three_args() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.decode_arg(), "some");
    assert_eq!(msg.decode_arg(), "stuff");
    assert_eq!(msg.decode_arg(), "here");
}

#[test]
fn test_decode_empty_string_arg() {
    let encoded = amp_encode(&[""]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.argc, 1);
    let arg = msg.decode_arg();
    assert_eq!(arg, "");
}

#[test]
fn test_encode_header_version_and_argc() {
    for n in 0..=15u8 {
        let args: Vec<&str> = (0..n).map(|_| "x").collect();
        let encoded = amp_encode(&args);
        let header = encoded.as_bytes()[0];
        assert_eq!(header >> 4, AMP_VERSION as u8);
        assert_eq!(header & 0xf, n);
    }
}

#[test]
fn test_roundtrip_various() {
    let cases: &[&[&str]] = &[
        &["a"],
        &["abc", "def"],
        &["", "nonempty"],
        &["hello world", "foo bar baz"],
    ];
    for args in cases {
        let encoded = amp_encode(args);
        let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
        msg.decode(&encoded);
        assert_eq!(msg.version, 1);
        assert_eq!(msg.argc, args.len() as i16);
        for expected in *args {
            assert_eq!(msg.decode_arg(), *expected);
        }
    }
}

fn main() {}
