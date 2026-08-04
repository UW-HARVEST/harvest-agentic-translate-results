use Megalania::memory_mapper::{map_anonymous, map_file, unmap};

#[test]
fn test_map_anonymous_zeros() {
    let buf = map_anonymous(64).expect("could not map anonymous");
    assert_eq!(buf.len(), 64);
    for b in buf.iter() {
        assert_eq!(*b, 0);
    }
}

#[test]
fn test_map_anonymous_large() {
    let buf = map_anonymous(1024).expect("could not map anonymous");
    assert_eq!(buf.len(), 1024);
}

#[test]
fn test_map_anonymous_zero() {
    let buf = map_anonymous(0).expect("could not map anonymous");
    assert_eq!(buf.len(), 0);
}

#[test]
fn test_unmap() {
    let buf = map_anonymous(32).expect("could not map");
    let res = unmap(buf);
    assert!(res.is_ok());
}

#[test]
fn test_map_file_round_trip() {
    let path = std::env::temp_dir().join("test_map_file.bin");
    let payload: Vec<u8> = (0..32u8).collect();
    std::fs::write(&path, &payload).expect("could not write");
    let buf = map_file(&path).expect("could not map");
    assert_eq!(buf, payload);
    let _ = std::fs::remove_file(&path);
}

fn main() {}
