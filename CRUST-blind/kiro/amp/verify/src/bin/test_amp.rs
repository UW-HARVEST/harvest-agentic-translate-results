use amp::amp::{amp_encode, Amp, AMP_VERSION};

#[test]
fn test_amp_version_constant() {
    assert_eq!(AMP_VERSION, 1);
}

#[test]
fn test_encode_three_args() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(
        bytes,
        vec![19, 0, 0, 0, 4, 115, 111, 109, 101, 0, 0, 0, 5, 115, 116, 117, 102, 102, 0, 0, 0, 4, 104, 101, 114, 101]
    );
}

#[test]
fn test_encode_single_arg() {
    let encoded = amp_encode(&["hello"]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes, vec![17, 0, 0, 0, 5, 104, 101, 108, 108, 111]);
}

#[test]
fn test_encode_empty_string_arg() {
    let encoded = amp_encode(&[""]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes, vec![17, 0, 0, 0, 0]);
}

#[test]
fn test_encode_zero_args() {
    let encoded = amp_encode(&[]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes, vec![16]);
}

#[test]
fn test_encode_two_args() {
    let encoded = amp_encode(&["ab", "cd"]);
    let bytes: Vec<u8> = encoded.chars().map(|c| c as u8).collect();
    assert_eq!(bytes, vec![18, 0, 0, 0, 2, 97, 98, 0, 0, 0, 2, 99, 100]);
}

#[test]
fn test_decode_header_three_args() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
}

#[test]
fn test_decode_header_single_arg() {
    let encoded = amp_encode(&["hello"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
}

#[test]
fn test_decode_header_zero_args() {
    let encoded = amp_encode(&[]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 0);
}

#[test]
fn test_decode_arg_three_args() {
    let encoded = amp_encode(&["some", "stuff", "here"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.decode_arg(), "some");
    assert_eq!(msg.decode_arg(), "stuff");
    assert_eq!(msg.decode_arg(), "here");
}

#[test]
fn test_decode_arg_two_args() {
    let encoded = amp_encode(&["ab", "cd"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.decode_arg(), "ab");
    assert_eq!(msg.decode_arg(), "cd");
}

#[test]
fn test_decode_arg_single() {
    let encoded = amp_encode(&["hello"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&encoded);
    assert_eq!(msg.decode_arg(), "hello");
}

#[test]
fn test_encode_header_byte_format() {
    // Header byte should be (version << 4) | argc
    for argc in 0..5u8 {
        let args: Vec<&str> = (0..argc).map(|_| "x").collect();
        let encoded = amp_encode(&args);
        let header = encoded.chars().next().unwrap() as u8;
        assert_eq!(header >> 4, 1); // version
        assert_eq!(header & 0xf, argc); // argc
    }
}

fn main() {}
