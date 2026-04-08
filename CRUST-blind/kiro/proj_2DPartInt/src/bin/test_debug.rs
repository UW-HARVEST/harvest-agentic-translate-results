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

fn main() {}
