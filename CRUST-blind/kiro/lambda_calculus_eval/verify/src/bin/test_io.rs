use lambda_calculus_eval::io::*;
use std::io::{Read, Seek, SeekFrom};

#[test]
fn test_create_and_delete_file() {
    let path = "/tmp/test_io_create.txt";
    let _f = create_file(path).unwrap();
    assert!(std::path::Path::new(path).exists());
    delete_file(path).unwrap();
    assert!(!std::path::Path::new(path).exists());
}

#[test]
fn test_get_file_read() {
    let path = "/tmp/test_io_get_r.txt";
    std::fs::write(path, "hello").unwrap();
    let mut f = get_file(path, "r").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "hello");
    delete_file(path).unwrap();
}

#[test]
fn test_write_to_file() {
    let path = "/tmp/test_io_write.txt";
    let mut f = get_file(path, "w").unwrap();
    write_to_file(&mut f, "test content").unwrap();
    drop(f);
    // verify by reading back
    let mut f2 = get_file(path, "r").unwrap();
    let mut buf = String::new();
    f2.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "test content");
    delete_file(path).unwrap();
}

#[test]
fn test_next_char() {
    let path = "/tmp/test_io_next.txt";
    std::fs::write(path, "AB").unwrap();
    let mut f = get_file(path, "r").unwrap();
    let c1 = next(&mut f).unwrap();
    let c2 = next(&mut f).unwrap();
    assert_eq!(c1, 'A');
    assert_eq!(c2, 'B');
    delete_file(path).unwrap();
}

#[test]
fn test_close_file() {
    let path = "/tmp/test_io_close.txt";
    let f = create_file(path).unwrap();
    assert!(close_file(f).is_ok());
    delete_file(path).unwrap();
}

fn main() {}
