use twoDPartInt::debug;
use std::fs;

fn unique_dir(name: &str) -> String {
    format!("/tmp/twoDPartInt_debug_test_{}_{}", name, std::process::id())
}

#[test]
fn test_write_debug_information() {
    let dir = unique_dir("dbg");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    debug::write_debug_information(42, 3, 7, &dir);
    let path = format!("{}/debug_step_42.csv", dir);
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "step,particle_index,contacts_size");
    assert_eq!(lines[1], "3,7");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_header() {
    let dir = unique_dir("hdr");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = format!("{}/header.csv", dir);
    let mut file = fs::File::create(&path).unwrap();
    debug::write_header(0, 0, &mut file);
    drop(file);
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content.trim(), "step,particle_index,contacts_size");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_values() {
    let dir = unique_dir("vals");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = format!("{}/values.csv", dir);
    let mut file = fs::File::create(&path).unwrap();
    debug::write_values(11, 22, &mut file);
    drop(file);
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content.trim(), "11,22");
    let _ = fs::remove_dir_all(&dir);
}

fn main() {}
