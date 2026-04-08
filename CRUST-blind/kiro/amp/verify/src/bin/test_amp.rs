use amp::amp::{amp_encode, Amp, AMP_VERSION};

#[test]
fn test_amp_version() {
    assert_eq!(AMP_VERSION, 1);
}

#[test]
fn test_encode_three_args() {
    let buf = amp_encode(&["some", "stuff", "here"]);
    let bytes: Vec<u8> = buf.chars().map(|c| c as u8).collect();
    assert_eq!(
        bytes,
        vec![
            19, 0, 0, 0, 4, 115, 111, 109, 101, 0, 0, 0, 5, 115, 116, 117, 102, 102, 0, 0, 0,
            4, 104, 101, 114, 101
        ]
    );
}

#[test]
fn test_encode_header_byte() {
    for argc in 0u8..=5 {
        let args: Vec<&str> = (0..argc).map(|_| "x").collect();
        let buf = amp_encode(&args);
        let header = buf.chars().next().unwrap() as u8;
        assert_eq!(header, (AMP_VERSION as u8) << 4 | argc);
    }
}

#[test]
fn test_decode_three_args() {
    let buf = amp_encode(&["some", "stuff", "here"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
    assert_eq!(msg.decode_arg(), "some");
    assert_eq!(msg.decode_arg(), "stuff");
    assert_eq!(msg.decode_arg(), "here");
}

#[test]
fn test_decode_one_arg() {
    let buf = amp_encode(&["hello"]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
    assert_eq!(msg.decode_arg(), "hello");
}

#[test]
fn test_decode_zero_args() {
    let buf = amp_encode(&[]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 0);
}

#[test]
fn test_decode_empty_string_arg() {
    let buf = amp_encode(&[""]);
    let mut msg = Amp { version: 0, argc: 0, buf: String::new() };
    msg.decode(&buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 1);
    let arg = msg.decode_arg();
    assert_eq!(arg.len(), 0);
    assert_eq!(arg, "");
}

fn main() {}
