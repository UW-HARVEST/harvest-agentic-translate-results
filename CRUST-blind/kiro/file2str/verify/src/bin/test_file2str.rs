use file2str::file2str::{file2str, file2strl};

#[test]
fn test_file2strl_valid_file() {
    let mut len: u32 = 0;
    let result = file2strl("tests/test.txt", &mut len);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Text file\n");
    assert_eq!(len, 11);
}

#[test]
fn test_file2str_valid_file() {
    let result = file2str("tests/test.txt");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Text file\n");
}

#[test]
fn test_file2strl_nonexistent_file() {
    let mut len: u32 = 999;
    let result = file2strl("nonexistent.txt", &mut len);
    assert!(result.is_none());
    assert_eq!(len, 999); // len unchanged on error
}

#[test]
fn test_file2str_nonexistent_file() {
    let result = file2str("nonexistent.txt");
    assert!(result.is_none());
}

#[test]
fn test_file2strl_empty_file() {
    let path = "tests/empty_test.txt";
    // Create empty file
    std::fs::File::create(path).unwrap();
    let mut len: u32 = 999;
    let result = file2strl(path, &mut len);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "");
    assert_eq!(len, 1);
    std::fs::remove_file(path).ok();
}

fn main() {}
