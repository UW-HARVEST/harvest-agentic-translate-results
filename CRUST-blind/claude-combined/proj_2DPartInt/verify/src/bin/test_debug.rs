use std::fs;
use std::path::PathBuf;
use twoDPartInt::debug;

fn temp_dir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(name);
    if p.exists() {
        let _ = fs::remove_dir_all(&p);
    }
    p
}

#[test]
fn test_write_debug_information_creates_file() {
    let dir = temp_dir("test_debug_info");
    debug::write_debug_information(3, 2, 5, dir.to_str().unwrap());
    let mut file_path = dir.clone();
    file_path.push("debug_step_3.txt");
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("step,particle_index"));
    assert!(content.contains("3,2"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_header_writes_data() {
    let dir = temp_dir("test_debug_header");
    fs::create_dir_all(&dir).unwrap();
    let mut file_path = dir.clone();
    file_path.push("h.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    debug::write_header(11, 22, &mut file);
    drop(file);
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("11,22"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_values_writes_data() {
    let dir = temp_dir("test_debug_values");
    fs::create_dir_all(&dir).unwrap();
    let mut file_path = dir.clone();
    file_path.push("v.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    debug::write_values(2, 7, &mut file);
    drop(file);
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("2,7"));
    let _ = fs::remove_dir_all(&dir);
}

fn main() {}
