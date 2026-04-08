use impcheck::writer::Writer;
use std::io::Read;

#[test]
fn test_writer_write_int() {
    let path = "/tmp/test_writer_int.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_int(42);
        w.write_int(-1);
        w.write_int(0);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf.len(), 12); // 3 ints * 4 bytes
    let v0 = i32::from_ne_bytes(buf[0..4].try_into().unwrap());
    let v1 = i32::from_ne_bytes(buf[4..8].try_into().unwrap());
    let v2 = i32::from_ne_bytes(buf[8..12].try_into().unwrap());
    assert_eq!(v0, 42);
    assert_eq!(v1, -1);
    assert_eq!(v2, 0);
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
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, vec![1, 0]);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_char() {
    let path = "/tmp/test_writer_char.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_char(b'A' as i32);
        w.write_char(b'B' as i32);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, vec![b'A', b'B']);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_ul() {
    let path = "/tmp/test_writer_ul.bin";
    {
        let mut w = Writer::writer_init(path);
        w.write_ul(12345678901234u64);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf.len(), 8);
    let v = u64::from_ne_bytes(buf[0..8].try_into().unwrap());
    assert_eq!(v, 12345678901234u64);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_ints() {
    let path = "/tmp/test_writer_ints.bin";
    let data = [10i32, 20, 30];
    {
        let mut w = Writer::writer_init(path);
        w.write_ints(&data, 3);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf.len(), 12);
    for i in 0..3 {
        let v = i32::from_ne_bytes(buf[i*4..(i+1)*4].try_into().unwrap());
        assert_eq!(v, data[i]);
    }
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
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf.len(), 24);
    for i in 0..3 {
        let v = u64::from_ne_bytes(buf[i*8..(i+1)*8].try_into().unwrap());
        assert_eq!(v, data[i]);
    }
    std::fs::remove_file(path).ok();
}

#[test]
fn test_writer_write_sig() {
    let path = "/tmp/test_writer_sig.bin";
    let sig: [u8; 16] = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
    {
        let mut w = Writer::writer_init(path);
        w.write_sig(&sig);
    }
    let mut f = std::fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, sig.to_vec());
    std::fs::remove_file(path).ok();
}

fn main() {}
