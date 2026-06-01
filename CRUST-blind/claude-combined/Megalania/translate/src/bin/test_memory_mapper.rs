use Megalania::memory_mapper::{map_anonymous, map_file, unmap};
use std::io::Write;

#[test]
fn test_map_anonymous_returns_zeroed() {
    let buf = map_anonymous(64).expect("map_anonymous failed");
    assert_eq!(buf.len(), 64);
    for &b in buf.iter() {
        assert_eq!(b, 0);
    }
}

#[test]
fn test_map_anonymous_zero_size() {
    let buf = map_anonymous(0).expect("map_anonymous failed");
    assert_eq!(buf.len(), 0);
}

#[test]
fn test_map_file_reads_contents() {
    // Create a temp file
    let mut path = std::env::temp_dir();
    path.push(format!("megalania_test_{}", std::process::id()));
    {
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(b"hello world").expect("write");
    }
    let buf = map_file(&path).expect("map_file failed");
    assert_eq!(buf, b"hello world".to_vec());
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_map_file_missing_returns_err() {
    let path = "/nonexistent/path/that/does/not/exist/megalania_test";
    let result = map_file(path);
    assert!(result.is_err());
}

#[test]
fn test_unmap_succeeds() {
    let buf = map_anonymous(16).expect("map_anonymous failed");
    let r = unmap(buf);
    assert!(r.is_ok());
}

fn main() {}
