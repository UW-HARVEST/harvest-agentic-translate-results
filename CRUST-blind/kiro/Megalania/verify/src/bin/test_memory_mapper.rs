use Megalania::memory_mapper::{map_file, map_anonymous, unmap};

#[test]
fn test_map_anonymous() {
    let data = map_anonymous(1024).unwrap();
    assert_eq!(data.len(), 1024);
    // All bytes should be zero
    assert!(data.iter().all(|&b| b == 0));
}

#[test]
fn test_unmap() {
    let data = map_anonymous(64).unwrap();
    assert!(unmap(data).is_ok());
}

#[test]
fn test_map_file() {
    let tmp = std::env::temp_dir().join("megalania_test_map_file.bin");
    std::fs::write(&tmp, &[1, 2, 3, 4, 5]).unwrap();
    let data = map_file(&tmp).unwrap();
    assert_eq!(data, vec![1, 2, 3, 4, 5]);
    std::fs::remove_file(&tmp).ok();
}

fn main() {}
