use twoDPartInt::debug;

#[test]
fn test_write_debug_information() {
    let dir = std::env::temp_dir().join("test_debug_info");
    std::fs::create_dir_all(&dir).unwrap();
    debug::write_debug_information(5, 2, 10, dir.to_str().unwrap());
    let content = std::fs::read_to_string(dir.join("debug_step_5.csv")).unwrap();
    assert!(content.contains("step,particle_index"));
    assert!(content.contains("5,2"));
    assert!(content.contains("particle_index,contacts_size"));
    assert!(content.contains("2,10"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_header() {
    let dir = std::env::temp_dir().join("test_debug_header");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("header_test.csv");
    let mut file = std::fs::File::create(&path).unwrap();
    debug::write_header(3, 7, &mut file);
    drop(file);
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("step,particle_index"));
    assert!(content.contains("3,7"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_values() {
    let dir = std::env::temp_dir().join("test_debug_values");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("values_test.csv");
    let mut file = std::fs::File::create(&path).unwrap();
    debug::write_values(4, 12, &mut file);
    drop(file);
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("particle_index,contacts_size"));
    assert!(content.contains("4,12"));
    std::fs::remove_dir_all(&dir).ok();
}

fn main() {}
