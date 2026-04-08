use impcheck::writer::Writer;
use impcheck::trusted_utils;

#[test]
fn test_writer_init_and_write_int() {
    let path = "/tmp/test_writer_int.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_int(42);
        w.write_int(-1);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let v1 = trusted_utils::trusted_utils_read_int(&mut f);
    let v2 = trusted_utils::trusted_utils_read_int(&mut f);
    assert_eq!(v1, 42);
    assert_eq!(v2, -1);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_char() {
    let path = "/tmp/test_writer_char.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_char(b'A' as i32);
        w.write_char(b'Z' as i32);
    }
    let data = std::fs::read(path).unwrap();
    assert_eq!(data, vec![b'A', b'Z']);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_bool() {
    let path = "/tmp/test_writer_bool.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_bool(true);
        w.write_bool(false);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let v1 = trusted_utils::trusted_utils_read_bool(&mut f);
    let v2 = trusted_utils::trusted_utils_read_bool(&mut f);
    assert!(v1);
    assert!(!v2);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_ul() {
    let path = "/tmp/test_writer_ul.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_ul(123456789u64);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let v = trusted_utils::trusted_utils_read_ul(&mut f);
    assert_eq!(v, 123456789u64);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_sig() {
    let path = "/tmp/test_writer_sig.bin";
    let sig: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    {
        let mut w = Writer::writer_init(path);
        w.write_sig(&sig);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut out = [0u8; 16];
    trusted_utils::trusted_utils_read_sig(&mut out, &mut f);
    assert_eq!(out, sig);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_ints() {
    let path = "/tmp/test_writer_ints.bin";
    let data = [10i32, -20, 30];
    {
        let mut w = Writer::writer_init(path);
        w.write_ints(&data, 3);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut out = [0i32; 3];
    trusted_utils::trusted_utils_read_ints(&mut out, 3, &mut f);
    assert_eq!(out, data);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_uls() {
    let path = "/tmp/test_writer_uls.bin";
    let data = [100u64, 200, 300];
    {
        let mut w = Writer::writer_init(path);
        w.write_uls(&data, 3);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut out = [0u64; 3];
    trusted_utils::trusted_utils_read_uls(&mut out, 3, &mut f);
    assert_eq!(out, data);
    std::fs::remove_file(path).ok();
}

fn main() {}
