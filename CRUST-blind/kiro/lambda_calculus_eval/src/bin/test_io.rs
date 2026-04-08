use lambda_calculus_eval::io;
use std::io::Read;

#[test]
fn test_create_file() {
    let path = "/tmp/test_io_create.txt";
    let _ = io::create_file(path);
    assert!(std::path::Path::new(path).exists());
    let _ = io::delete_file(path);
}

#[test]
fn test_write_and_read_file() {
    let path = "/tmp/test_io_write.txt";
    // Use get_file with "w+" to get read+write access
    let mut f = io::get_file(path, "w+").unwrap();
    io::write_to_file(&mut f, "Hello").unwrap();
    // write_to_file rewinds, so we can read from start
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Hello");
    let _ = io::delete_file(path);
}

#[test]
fn test_get_file() {
    let path = "/tmp/test_io_get.txt";
    let _ = io::create_file(path).unwrap();
    let f = io::get_file(path, "r");
    assert!(f.is_ok());
    let _ = io::delete_file(path);
}

#[test]
fn test_delete_file() {
    let path = "/tmp/test_io_delete.txt";
    let _ = io::create_file(path).unwrap();
    assert!(std::path::Path::new(path).exists());
    io::delete_file(path).unwrap();
    assert!(!std::path::Path::new(path).exists());
}

#[test]
fn test_next_char() {
    let path = "/tmp/test_io_next.txt";
    // Use get_file with "w+" for read+write
    let mut f = io::get_file(path, "w+").unwrap();
    io::write_to_file(&mut f, "AB").unwrap();
    let c1 = io::next(&mut f).unwrap();
    let c2 = io::next(&mut f).unwrap();
    assert_eq!(c1, 'A');
    assert_eq!(c2, 'B');
    let _ = io::delete_file(path);
}

#[test]
fn test_close_file() {
    let path = "/tmp/test_io_close.txt";
    let f = io::create_file(path).unwrap();
    assert!(io::close_file(f).is_ok());
    let _ = io::delete_file(path);
}

fn main() {}
