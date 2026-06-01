use impcheck::writer::Writer;
use std::fs::File;
use std::io::Read;

fn read_file_bytes(path: &std::path::Path) -> Vec<u8> {
    let mut f = File::open(path).unwrap();
    let mut data = Vec::new();
    f.read_to_end(&mut data).unwrap();
    data
}

#[test]
fn test_writer_init_creates_file() {
    let path = std::env::temp_dir().join("test_w_init.bin");
    let _ = std::fs::remove_file(&path);
    let _w = Writer::writer_init(path.to_str().unwrap());
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_int() {
    let path = std::env::temp_dir().join("test_w_int.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    w.write_int(0x12345678);
    drop(w);
    let bytes = read_file_bytes(&path);
    // Native endian: little-endian on x86
    let expected: i32 = 0x12345678;
    assert_eq!(bytes.as_slice(), &expected.to_ne_bytes());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_ints() {
    let path = std::env::temp_dir().join("test_w_ints.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    let data = [1i32, -2, 3, -4];
    w.write_ints(&data, 4);
    drop(w);
    let bytes = read_file_bytes(&path);
    let mut expected = Vec::new();
    for i in &data {
        expected.extend_from_slice(&i.to_ne_bytes());
    }
    assert_eq!(bytes, expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_ul() {
    let path = std::env::temp_dir().join("test_w_ul.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    w.write_ul(0xDEADBEEF12345678);
    drop(w);
    let bytes = read_file_bytes(&path);
    assert_eq!(bytes.as_slice(), &0xDEADBEEF12345678u64.to_ne_bytes());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_uls() {
    let path = std::env::temp_dir().join("test_w_uls.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    let data: [u64; 3] = [1, 0xFFFF, 0xFFFF_FFFF_FFFF_FFFF];
    w.write_uls(&data, 3);
    drop(w);
    let bytes = read_file_bytes(&path);
    let mut expected = Vec::new();
    for u in &data {
        expected.extend_from_slice(&u.to_ne_bytes());
    }
    assert_eq!(bytes, expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_bool() {
    let path = std::env::temp_dir().join("test_w_bool.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    w.write_bool(true);
    w.write_bool(false);
    drop(w);
    let bytes = read_file_bytes(&path);
    assert_eq!(bytes.as_slice(), &[1u8, 0u8]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_char() {
    let path = std::env::temp_dir().join("test_w_char.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    w.write_char('A' as i32);
    w.write_char('Z' as i32);
    drop(w);
    let bytes = read_file_bytes(&path);
    assert_eq!(bytes.as_slice(), &[b'A', b'Z']);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_writer_write_sig() {
    let path = std::env::temp_dir().join("test_w_sig.bin");
    let _ = std::fs::remove_file(&path);
    let mut w = Writer::writer_init(path.to_str().unwrap());
    let mut sig = [0u8; 16];
    for i in 0..16 {
        sig[i] = i as u8 + 1;
    }
    w.write_sig(&sig);
    drop(w);
    let bytes = read_file_bytes(&path);
    assert_eq!(bytes.as_slice(), &sig);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
