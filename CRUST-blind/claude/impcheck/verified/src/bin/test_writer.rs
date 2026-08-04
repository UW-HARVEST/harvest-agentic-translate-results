use impcheck::writer::Writer;

fn tmp_path(name: &str) -> String {
    format!(
        "{}/impcheck_writer_test_{}_{}",
        std::env::temp_dir().display(),
        std::process::id(),
        name
    )
}

#[test]
fn test_writer_init_creates_file() {
    let path = tmp_path("init");
    let _w = Writer::writer_init(&path);
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_int_le() {
    let path = tmp_path("int");
    {
        let mut w = Writer::writer_init(&path);
        w.write_int(0x12345678);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![0x78, 0x56, 0x34, 0x12]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_int_negative() {
    let path = tmp_path("int_neg");
    {
        let mut w = Writer::writer_init(&path);
        w.write_int(-1);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![0xff, 0xff, 0xff, 0xff]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_ul_le() {
    let path = tmp_path("ul");
    {
        let mut w = Writer::writer_init(&path);
        w.write_ul(0x0123456789abcdefu64);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        bytes,
        vec![0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_bool_one_byte() {
    let path = tmp_path("bool");
    {
        let mut w = Writer::writer_init(&path);
        w.write_bool(true);
        w.write_bool(false);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![1, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_char_byte_value() {
    let path = tmp_path("char");
    {
        let mut w = Writer::writer_init(&path);
        // C: trusted_utils_write_char(c, file) writes c as a single byte (low 8 bits).
        w.write_char(b'X' as i32);
        w.write_char(b'Y' as i32);
        w.write_char(0x100 + b'Z' as i32); // truncated to byte 'Z'
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, vec![b'X', b'Y', b'Z']);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_ints_le_concatenated() {
    let path = tmp_path("ints");
    let data = [1i32, -1, 0x01020304];
    {
        let mut w = Writer::writer_init(&path);
        w.write_ints(&data, 3);
    }
    let bytes = std::fs::read(&path).unwrap();
    let expected = vec![
        0x01, 0x00, 0x00, 0x00, // 1
        0xff, 0xff, 0xff, 0xff, // -1
        0x04, 0x03, 0x02, 0x01, // 0x01020304
    ];
    assert_eq!(bytes, expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_ints_zero_count() {
    let path = tmp_path("ints_zero");
    let data: [i32; 0] = [];
    {
        let mut w = Writer::writer_init(&path);
        w.write_ints(&data, 0);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_uls_le_concatenated() {
    let path = tmp_path("uls");
    let data = [1u64, 0xdeadbeefu64];
    {
        let mut w = Writer::writer_init(&path);
        w.write_uls(&data, 2);
    }
    let bytes = std::fs::read(&path).unwrap();
    let expected = vec![
        // 1
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // 0xdeadbeef
        0xef, 0xbe, 0xad, 0xde, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(bytes, expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_sig_writes_16_bytes() {
    let path = tmp_path("sig");
    let mut sig = [0u8; 16];
    for i in 0..16 {
        sig[i] = (i + 1) as u8;
    }
    {
        let mut w = Writer::writer_init(&path);
        w.write_sig(&sig);
    }
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[..], &sig[..]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_combined_sequence() {
    let path = tmp_path("combined");
    {
        let mut w = Writer::writer_init(&path);
        w.write_int(0x01020304);
        w.write_bool(true);
        w.write_char(b'A' as i32);
        w.write_ul(0x0102030405060708u64);
    }
    let bytes = std::fs::read(&path).unwrap();
    let expected = vec![
        0x04, 0x03, 0x02, 0x01, // int LE
        0x01,                    // bool true
        b'A',                    // char
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // u64 LE
    ];
    assert_eq!(bytes, expected);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
