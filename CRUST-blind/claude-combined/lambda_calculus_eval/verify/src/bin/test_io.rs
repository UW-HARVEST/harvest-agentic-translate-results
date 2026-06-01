use lambda_calculus_eval::io::{
    close_file, create_file, delete_file, get_file, next, write_to_file,
};
use std::io::{Read, Seek, SeekFrom, Write};

fn unique_path(name: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/lc_io_test_{}_{}", name, n)
}

#[test]
fn test_create_file_creates_file() {
    let path = unique_path("create");
    let f = create_file(&path).expect("create_file failed");
    drop(f);
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_get_file_opens_file() {
    let path = unique_path("get");
    let mut f = create_file(&path).expect("create_file failed");
    f.write_all(b"hello").unwrap();
    drop(f);
    let mut g = get_file(&path, "r").expect("get_file failed");
    let mut buf = String::new();
    g.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "hello");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_write_to_file_writes_and_rewinds() {
    let path = unique_path("write");
    let mut f = create_file(&path).expect("create");
    write_to_file(&mut f, "hello world").expect("write");
    // After rewind, position is 0
    let pos = f.stream_position().unwrap();
    assert_eq!(pos, 0);
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "hello world");
    drop(f);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_delete_file_removes_file() {
    let path = unique_path("del");
    let f = create_file(&path).expect("create");
    drop(f);
    assert!(std::path::Path::new(&path).exists());
    delete_file(&path).expect("delete");
    assert!(!std::path::Path::new(&path).exists());
}

#[test]
fn test_close_file_does_not_error() {
    let path = unique_path("close");
    let f = create_file(&path).expect("create");
    close_file(f).expect("close");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_next_reads_one_byte() {
    let path = unique_path("next");
    let mut f = create_file(&path).expect("create");
    f.write_all(b"abc").unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let c = next(&mut f).expect("next");
    assert_eq!(c, 'a');
    let c2 = next(&mut f).expect("next");
    assert_eq!(c2, 'b');
    let c3 = next(&mut f).expect("next");
    assert_eq!(c3, 'c');
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_delete_file_returns_error_on_missing() {
    let path = unique_path("missing");
    let r = delete_file(&path);
    assert!(r.is_err());
}

fn main() {}
